use crate::{bearer_token::BearerToken, rest_api_builder::RestApiBuilder, RestApiError};
use reqwest::header::HeaderMap;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;

const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
/// Upper bound on how long a single retry will wait, even if the server asks for more.
/// Prevents a hostile or misconfigured `Retry-After` from blocking the client indefinitely.
const DEFAULT_MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct RestApi {
    client: reqwest::Client,
    user_agent: String,
    api_url: String,
    api_version: u8,
    pub token: Arc<RwLock<BearerToken>>,
    max_retries: u32,
    retry_base_delay: Duration,
    max_retry_after: Duration,
}

impl RestApi {
    /// Returns a `RestApiBuilder`. Wrapper around `RestApiBuilder::new()`.
    pub fn builder<S: Into<String>>(api_url: S) -> Result<RestApiBuilder, RestApiError> {
        RestApiBuilder::new(api_url)
    }

    /// Returns the user agent
    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    /// Returns the API version
    pub const fn api_version(&self) -> u8 {
        self.api_version
    }

    /// Returns a `RequestBuilder` for a Wikibase REST API request
    /// # Errors
    /// Returns an error if the headers cannot be created
    pub async fn wikibase_request_builder<S: Into<String>>(
        &self,
        path: S,
        params: HashMap<String, String>,
        method: reqwest::Method,
    ) -> Result<reqwest::RequestBuilder, RestApiError> {
        let mut headers = self.headers().await?;
        headers.insert(reqwest::header::ACCEPT, "application/json".parse()?);
        match method {
            reqwest::Method::GET => {}
            reqwest::Method::PATCH => {
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json-patch+json"),
                );
            }
            _ => {
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    reqwest::header::HeaderValue::from_static("application/json"),
                );
            }
        }
        let wikibase_path = format!("{}{}", self.wikibase_root(), path.into());
        self.request_builder(&wikibase_path, headers, params, method)
    }

    /// Returns a `RestApi` instance for Wikidata
    pub fn wikidata() -> Result<RestApi, RestApiError> {
        Ok(RestApi::builder("https://www.wikidata.org/w/rest.php")?.build())
    }

    /// Executes a `reqwest::Request` with automatic retry on 429 and 5xx errors.
    /// Respects `Retry-After` headers when present.
    /// # Errors
    /// Returns an error if all retry attempts fail
    pub async fn execute(
        &self,
        request: reqwest::Request,
    ) -> Result<reqwest::Response, RestApiError> {
        self.token.write().await.check(self, &request).await?;

        for attempt in 0..=self.max_retries {
            // Clone for a possible retry. If the body isn't cloneable (e.g. a stream),
            // we can only send it once — execute the original and return its result.
            let req = match request.try_clone() {
                Some(req) => req,
                None => return Ok(self.client.execute(request).await?),
            };

            let response = self.client.execute(req).await?;
            let status = response.status();
            let retryable =
                status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error();

            if retryable && attempt < self.max_retries {
                let delay = self.retry_delay(&response, attempt);
                tokio::time::sleep(delay).await;
                continue;
            }
            if retryable {
                return Err(RestApiError::from_response(response).await);
            }
            return Ok(response);
        }

        // The loop always returns; this satisfies the type checker only.
        Err(RestApiError::EmptyValue(
            "all retry attempts exhausted".into(),
        ))
    }

    /// Calculates the delay before retrying. Honors `Retry-After` (both delta-seconds and
    /// HTTP-date forms), capped at `max_retry_after`; otherwise uses jittered exponential backoff.
    fn retry_delay(&self, response: &reqwest::Response, attempt: u32) -> Duration {
        if let Some(retry_after) = Self::retry_after(response) {
            return retry_after.min(self.max_retry_after);
        }
        self.backoff_with_jitter(attempt)
    }

    /// Parses a `Retry-After` header, supporting both delta-seconds and HTTP-date forms.
    fn retry_after(response: &reqwest::Response) -> Option<Duration> {
        let value = response.headers().get("Retry-After")?.to_str().ok()?;
        if let Ok(seconds) = value.parse::<u64>() {
            return Some(Duration::from_secs(seconds));
        }
        let when = httpdate::parse_http_date(value).ok()?;
        when.duration_since(SystemTime::now()).ok()
    }

    /// Exponential backoff with ±25% jitter, capped at `max_retry_after`.
    /// Jitter avoids synchronized retry storms when many clients back off together.
    fn backoff_with_jitter(&self, attempt: u32) -> Duration {
        let multiplier = 2_u32.saturating_pow(attempt.min(16));
        let base = self
            .retry_base_delay
            .saturating_mul(multiplier)
            .min(self.max_retry_after);
        let factor = 0.75 + Self::jitter_fraction() * 0.5; // [0.75, 1.25)
        base.mul_f64(factor)
    }

    /// A pseudo-random fraction in [0, 1) derived from the current time. Good enough for
    /// jitter; deliberately dependency-free (no `rand`).
    fn jitter_fraction() -> f64 {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        f64::from(nanos % 1000) / 1000.0
    }

    /// Executes a request and returns the parsed JSON body. Any non-success status
    /// is converted into a `RestApiError::ApiError` carrying the server error payload.
    async fn execute_json(
        &self,
        request: reqwest::Request,
    ) -> Result<serde_json::Value, RestApiError> {
        let response = self.execute(request).await?;
        if !response.status().is_success() {
            return Err(RestApiError::from_response(response).await);
        }
        Ok(response.json().await?)
    }

    /// Returns the `OpenAPI` JSON for the Wikibase REST API
    pub async fn get_openapi_json(&self) -> Result<serde_json::Value, RestApiError> {
        let request = self
            .wikibase_request_builder("/openapi.json", HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        self.execute_json(request).await
    }

    /// Returns the map of property data types to value types.
    /// Keys are data-type strings (e.g. `"wikibase-item"`); values are value-type strings
    /// (e.g. `"wikibase-entityid"`).
    /// # Errors
    /// Returns an error if the request fails or the response cannot be parsed.
    pub async fn get_property_data_types(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, RestApiError> {
        let request = self
            .wikibase_request_builder("/property-data-types", HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        let map = serde_json::from_value(self.execute_json(request).await?)?;
        Ok(map)
    }

    /// Returns the API URL
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    /// Returns the `reqwest::Client`
    pub const fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Creates a new `RestApi` instance.
    /// Only available internally, use `RestApi::builder()` instead.
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn new(
        client: reqwest::Client,
        user_agent: String,
        api_url: String,
        api_version: u8,
        token: Arc<RwLock<BearerToken>>,
        max_retries: u32,
        retry_base_delay: Duration,
        max_retry_after: Duration,
    ) -> Self {
        Self {
            client,
            user_agent,
            api_url,
            api_version,
            token,
            max_retries,
            retry_base_delay,
            max_retry_after,
        }
    }

    /// Returns a `HeaderMap` with the user agent and `OAuth2` bearer token (if present).
    /// Only available internally.
    pub(crate) async fn headers_from_token(
        &self,
        token: &BearerToken,
    ) -> Result<HeaderMap, RestApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, self.user_agent.parse()?);
        if let Some(access_token) = &token.get() {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {access_token}").parse()?,
            );
        }
        Ok(headers)
    }

    pub fn token(&self) -> Arc<RwLock<BearerToken>> {
        self.token.clone()
    }

    /// Returns the maximum number of retries on 429/5xx errors.
    pub const fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Returns the base delay for exponential backoff retries.
    pub const fn retry_base_delay(&self) -> Duration {
        self.retry_base_delay
    }

    /// Returns the maximum delay honored for a single retry (the `Retry-After` cap).
    pub const fn max_retry_after(&self) -> Duration {
        self.max_retry_after
    }

    /// Returns the root path for the Wikibase REST API, based on the version number
    fn wikibase_root(&self) -> String {
        format!("/wikibase/v{}", self.api_version)
    }

    /// Builds a `reqwest::RequestBuilder` from the method, client, path, and parameters
    fn request_builder<S: Into<String>>(
        &self,
        path: S,
        headers: HeaderMap,
        params: HashMap<String, String>,
        method: reqwest::Method,
    ) -> Result<reqwest::RequestBuilder, RestApiError> {
        let url = format!("{}{}", self.api_url, path.into());
        Ok(match method {
            reqwest::Method::GET => self.client.get(url).headers(headers).query(&params),
            reqwest::Method::POST => self.client.post(url).headers(headers).form(&params),
            reqwest::Method::PATCH => self.client.patch(url).headers(headers).form(&params),
            reqwest::Method::PUT => self.client.put(url).headers(headers).form(&params),
            reqwest::Method::DELETE => self.client.delete(url).headers(headers).form(&params),
            _ => return Err(RestApiError::UnsupportedMethod(method)),
        })
    }

    /// Returns a `HeaderMap` with the user agent and `OAuth2` bearer token (if present)
    async fn headers(&self) -> Result<HeaderMap, RestApiError> {
        let token = self.token.read().await;
        self.headers_from_token(&token).await
    }

    pub(crate) const fn default_max_retries() -> u32 {
        DEFAULT_MAX_RETRIES
    }

    pub(crate) const fn default_retry_base_delay() -> Duration {
        DEFAULT_RETRY_BASE_DELAY
    }

    pub(crate) const fn default_max_retry_after() -> Duration {
        DEFAULT_MAX_RETRY_AFTER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_get_openapi_json() {
        let expected_json = std::fs::read_to_string("test_data/openapi.json").unwrap();
        let expected_json: serde_json::Value = serde_json::from_str(&expected_json).unwrap();
        let mock_path = "/w/rest.php/wikibase/v1/openapi.json";
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(expected_json.clone()))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let json = api.get_openapi_json().await.unwrap();
        assert_eq!(json, expected_json);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_get_property_data_types() {
        use std::collections::HashMap;
        let expected: HashMap<String, String> = [
            ("wikibase-item".to_string(), "wikibase-entityid".to_string()),
            ("external-id".to_string(), "string".to_string()),
        ]
        .into();
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/property-data-types"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&expected))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();
        let result = api.get_property_data_types().await.unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_client() {
        let client = reqwest::Client::new();
        let api = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_client(client.clone())
            .build();
        assert_eq!(format!("{:?}", api.client), format!("{:?}", client));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_execute_json_error_carries_payload() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/property-data-types"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({"code": "bad", "message": "nope"})),
            )
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        // A 4xx must surface as ApiError with the server payload, not a bare Reqwest error.
        match api.get_property_data_types().await.unwrap_err() {
            RestApiError::ApiError {
                status, payload, ..
            } => {
                assert_eq!(status, 400);
                assert_eq!(payload.code(), "bad");
            }
            e => panic!("Wrong error type: {e:?}"),
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_retry_on_429() {
        let mock_server = MockServer::start().await;
        let mock_path = "/w/rest.php/wikibase/v1/openapi.json";

        // First two requests return 429, third succeeds
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "0"))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&mock_server)
            .await;

        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_max_retries(3)
            .with_retry_base_delay(Duration::from_millis(10))
            .build();

        let result = api.get_openapi_json().await;
        assert!(result.is_ok());
    }

    fn response_with_retry_after(value: &str) -> reqwest::Response {
        reqwest::Response::from(
            http::Response::builder()
                .status(429)
                .header("Retry-After", value)
                .body("")
                .unwrap(),
        )
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_retry_after_seconds() {
        let response = response_with_retry_after("5");
        assert_eq!(
            RestApi::retry_after(&response),
            Some(Duration::from_secs(5))
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_retry_after_http_date_future() {
        // A date ~1h in the future should yield a positive, roughly-1h delay.
        let future = SystemTime::now() + Duration::from_secs(3600);
        let response = response_with_retry_after(&httpdate::fmt_http_date(future));
        let delay = RestApi::retry_after(&response).unwrap();
        assert!(delay > Duration::from_secs(3000) && delay <= Duration::from_secs(3600));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_retry_after_garbage_is_none() {
        let response = response_with_retry_after("not-a-date");
        assert_eq!(RestApi::retry_after(&response), None);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_retry_delay_caps_retry_after() {
        let api = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_max_retry_after(Duration::from_secs(10))
            .build();
        // Server asks for a day; we clamp to the configured maximum.
        let response = response_with_retry_after("86400");
        assert_eq!(api.retry_delay(&response, 0), Duration::from_secs(10));
    }

    #[test]
    fn test_backoff_with_jitter_bounds() {
        let api = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_retry_base_delay(Duration::from_secs(1))
            .build();
        // Attempt 2 => base 4s, jittered into [3s, 5s).
        let delay = api.backoff_with_jitter(2);
        assert!(delay >= Duration::from_secs(3) && delay < Duration::from_secs(5));
    }

    #[test]
    fn test_jitter_fraction_range() {
        let f = RestApi::jitter_fraction();
        assert!((0.0..1.0).contains(&f));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_retry_exhausted() {
        let mock_server = MockServer::start().await;
        let mock_path = "/w/rest.php/wikibase/v1/openapi.json";

        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_max_retries(1)
            .with_retry_base_delay(Duration::from_millis(10))
            .build();

        let result = api.get_openapi_json().await;
        assert!(result.is_err());
    }
}
