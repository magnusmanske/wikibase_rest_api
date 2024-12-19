use reqwest::header::HeaderMap;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::{bearer_token::BearerToken, RestApiError};

/// The default user agent
const DEFAULT_USER_AGENT: &str = "Rust Wikibase REST API";

/// The latest supported version of the Wikibase REST API
const WIKIBASE_REST_API_VERSION: u8 = 0;

#[derive(Debug, Clone)]
pub struct RestApi {
    client: reqwest::Client,
    user_agent: String,
    api_url: String,
    api_version: u8,
    pub token: Arc<RwLock<BearerToken>>,
}

impl RestApi {
    /// Returns an empty `RestApiBuilder`
    pub fn builder() -> RestApiBuilder {
        RestApiBuilder::default()
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
        let wikibase_path = format!("{}{}", self.wikibase_root(), path.into());
        self.request_builder(&wikibase_path, headers, params, method)
    }

    fn wikibase_root(&self) -> String {
        format!("/wikibase/v{}", self.api_version)
    }

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

    /// Returns a `HeaderMap` with the user agent and `OAuth2` bearer token (if present)
    pub(crate) async fn headers_from_token(
        &self,
        token: &BearerToken,
    ) -> Result<HeaderMap, RestApiError> {
        let mut headers = HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, self.user_agent.parse()?);
        if let Some(access_token) = &token.get() {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", access_token).parse()?,
            );
        }
        Ok(headers)
    }

    pub fn array2hashmap(&self, array: &[(&str, &str)]) -> HashMap<String, String> {
        array
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// Executes a `reqwest::Request`, and returns a `reqwest::Response`.
    /// # Errors
    /// Returns an error if the request cannot be executed
    pub async fn execute(
        &self,
        request: reqwest::Request,
    ) -> Result<reqwest::Response, RestApiError> {
        self.token.write().await.check(self, &request).await?;
        let response = self.client.execute(request).await?;
        Ok(response)
    }

    pub async fn get_openapi_json(&self) -> Result<serde_json::Value, RestApiError> {
        let request = self
            .wikibase_request_builder("/openapi.json", HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        let response = self.execute(request).await?;
        let json = response.json().await?;
        Ok(json)
    }

    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    pub const fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

#[derive(Debug, Default)]
pub struct RestApiBuilder {
    client: reqwest::Client,
    token: BearerToken,
    user_agent: Option<String>,
    api_url: Option<String>,
    api_version: Option<u8>,
    renewal_interval: Option<std::time::Duration>,
}

impl RestApiBuilder {
    /// Sets the REST API URL, specifically the URL ending in "rest.php". This in mandatory.
    pub fn api<S: Into<String>>(mut self, api_url: S) -> Self {
        self.api_url = Some(api_url.into());
        self
    }

    pub const fn api_version(mut self, api_version: u8) -> Self {
        self.api_version = Some(api_version);
        self
    }

    /// Sets the `OAuth2` bearer token.
    pub fn set_access_token<S: Into<String>>(mut self, access_token: S) -> Self {
        self.token.set_access_token(access_token);
        self
    }

    /// Sets the user agent. By default, the user agent is "Rust Wikibase REST API; {`package_name`}/{`package_version`}"
    pub fn user_agent<S: Into<String>>(mut self, user_agent: S) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Sets the `reqwest::Client`. By default, a new `reqwest::Client` is created.
    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// Sets the `OAuth2` client ID and client secret
    #[cfg(not(tarpaulin_include))]
    pub fn oauth2_info<S1: Into<String>, S2: Into<String>>(
        mut self,
        client_id: S1,
        client_secret: S2,
    ) -> Self {
        self.token.set_oauth2_info(client_id, client_secret);
        self
    }

    fn validate_api_url(&self) -> Result<String, RestApiError> {
        let api_url = match &self.api_url {
            Some(api_url) => api_url.to_owned(),
            None => return Err(RestApiError::ApiNotSet),
        };
        let (base, _rest) = api_url
            .split_once("/rest.php")
            .ok_or_else(|| RestApiError::RestApiUrlInvalid(api_url.to_owned()))?;
        let ret = format!("{base}/rest.php");
        Ok(ret)
    }

    /// Builds the `RestApi`. Returns an error if no REST API URL is set.
    /// # Errors
    /// Returns an error if no REST API URL is set.
    pub fn build(&self) -> Result<RestApi, RestApiError> {
        let api_url = self.validate_api_url()?;
        let mut token = self.token.to_owned();
        token.set_renewal_interval(0); // Will use default value instead of 0
        Ok(RestApi {
            client: self.client.clone(),
            user_agent: self
                .user_agent
                .clone()
                .unwrap_or(Self::default_user_agent()),
            api_url,
            api_version: self.api_version.unwrap_or(WIKIBASE_REST_API_VERSION),
            token: Arc::new(RwLock::new(token)),
        })
    }

    /// Sets the interval for bearer token renewal. By default, the interval is `DEFAULT_RENEWAL_INTERVAL_SEC`.
    #[cfg(not(tarpaulin_include))]
    pub const fn access_token_renewal(mut self, renewal_interval: std::time::Duration) -> Self {
        self.renewal_interval = Some(renewal_interval);
        self
    }

    /// Returns the default user agent, a versioned string based on `DEFAULT_USER_AGENT`.
    fn default_user_agent() -> String {
        format!(
            "{DEFAULT_USER_AGENT}; {}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_get_openapi_json() {
        let expected_json = std::fs::read_to_string("test_data/openapi.json").unwrap();
        let expected_json: serde_json::Value = serde_json::from_str(&expected_json).unwrap();
        let mock_path = "/w/rest.php/wikibase/v0/openapi.json";
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(expected_json.clone()))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        let json = api.get_openapi_json().await.unwrap();
        assert_eq!(json, expected_json);
    }

    #[test]
    fn test_array2hashmap() {
        let api = RestApi::builder()
            .api("https://test.wikidata.org/w/rest.php")
            .build()
            .unwrap();
        let array = [("a", "1"), ("b", "2")];
        let hashmap = api.array2hashmap(&array);
        assert_eq!(hashmap.get("a"), Some(&"1".to_string()));
        assert_eq!(hashmap.get("b"), Some(&"2".to_string()));
    }

    #[test]
    fn test_default_user_agent() {
        let user_agent = RestApiBuilder::default_user_agent();
        assert!(user_agent.starts_with(DEFAULT_USER_AGENT));
        assert!(user_agent.contains(env!("CARGO_PKG_NAME")));
        assert!(user_agent.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_validate_api_url_default() {
        let builder = RestApiBuilder::default();
        let api_url = builder.validate_api_url();
        assert!(api_url.is_err());
    }

    #[test]
    fn test_validate_api_url_api() {
        let builder = RestApiBuilder::default().api("https://www.wikidata.org/w/api.php");
        let api_url = builder.validate_api_url();
        assert!(api_url.is_err());
    }

    #[test]
    fn test_validate_api_url_rest_api() {
        let builder = RestApiBuilder::default().api("https://www.wikidata.org/w/rest.php");
        let api_url = builder.validate_api_url();
        assert!(api_url.is_ok());
    }

    #[test]
    fn test_user_agent() {
        let api = RestApi::builder()
            .api("https://test.wikidata.org/w/rest.php")
            .build()
            .unwrap();
        assert_eq!(api.user_agent, RestApiBuilder::default_user_agent());

        let builder = RestApi::builder()
            .user_agent("Test User Agent")
            .api("https://test.wikidata.org/w/rest.php")
            .build()
            .unwrap();
        assert_eq!(builder.user_agent, "Test User Agent");
    }

    #[test]
    fn test_client() {
        let client = reqwest::Client::new();
        let api = RestApi::builder()
            .client(client.clone())
            .api("https://test.wikidata.org/w/rest.php")
            .build()
            .unwrap();
        assert_eq!(format!("{:?}", api.client), format!("{:?}", client));
    }
}
