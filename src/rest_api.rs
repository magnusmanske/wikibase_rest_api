use crate::{bearer_token::BearerToken, rest_api_builder::RestApiBuilder, RestApiError};
use reqwest::header::HeaderMap;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct RestApi {
    client: reqwest::Client,
    user_agent: String,
    api_url: String,
    api_version: u8,
    pub token: Arc<RwLock<BearerToken>>,
    max_retries: u32,
    retry_base_delay: Duration,
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
                    reqwest::header::HeaderValue::from_static("json-patch+json"),
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

        let mut last_error = None;
        for attempt in 0..=self.max_retries {
            // Clone the request for retries (first attempt uses original)
            let req = if attempt == 0 {
                request
                    .try_clone()
                    .ok_or_else(|| RestApiError::EmptyValue("request not cloneable".into()))?
            } else {
                match request.try_clone() {
                    Some(r) => r,
                    None => break, // Can't retry streaming requests
                }
            };

            let response = self.client.execute(req).await?;
            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error()
            {
                if attempt < self.max_retries {
                    let delay = self.retry_delay(&response, attempt);
                    tokio::time::sleep(delay).await;
                    last_error = Some(RestApiError::from_response(response).await);
                    continue;
                }
                // Last attempt failed — return the error
                return Err(RestApiError::from_response(response).await);
            }

            return Ok(response);
        }

        // Unreachable in normal flow, but handle edge case
        Err(last_error.unwrap_or_else(|| {
            RestApiError::EmptyValue("all retry attempts exhausted".into())
        }))
    }

    /// Calculates the delay before retrying, respecting `Retry-After` header if present.
    fn retry_delay(&self, response: &reqwest::Response, attempt: u32) -> Duration {
        // Check for Retry-After header (seconds)
        if let Some(retry_after) = response
            .headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            return Duration::from_secs(retry_after);
        }
        // Exponential backoff: base_delay * 2^attempt
        self.retry_base_delay * 2u32.pow(attempt)
    }

    /// Returns the `OpenAPI` JSON for the Wikibase REST API
    pub async fn get_openapi_json(&self) -> Result<serde_json::Value, RestApiError> {
        let request = self
            .wikibase_request_builder("/openapi.json", HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        let response = self.execute(request).await?;
        let json = response.json().await?;
        Ok(json)
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
    ) -> Self {
        Self {
            client,
            user_agent,
            api_url,
            api_version,
            token,
            max_retries,
            retry_base_delay,
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
