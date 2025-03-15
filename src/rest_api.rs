use crate::{bearer_token::BearerToken, rest_api_builder::RestApiBuilder, RestApiError};
use reqwest::header::HeaderMap;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RestApi {
    client: reqwest::Client,
    user_agent: String,
    api_url: String,
    api_version: u8,
    pub token: Arc<RwLock<BearerToken>>,
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
        let wikibase_path = format!("{}{}", self.wikibase_root(), path.into());
        self.request_builder(&wikibase_path, headers, params, method)
    }

    /// Returns a `RestApi` instance for Wikidata
    pub fn wikidata() -> Result<RestApi, RestApiError> {
        Ok(RestApi::builder("https://www.wikidata.org/w/rest.php")?.build())
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
    pub(crate) const fn new(
        client: reqwest::Client,
        user_agent: String,
        api_url: String,
        api_version: u8,
        token: Arc<RwLock<BearerToken>>,
    ) -> Self {
        Self {
            client,
            user_agent,
            api_url,
            api_version,
            token,
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
                format!("Bearer {}", access_token).parse()?,
            );
        }
        Ok(headers)
    }

    pub fn token(&self) -> Arc<RwLock<BearerToken>> {
        self.token.clone()
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
    fn test_client() {
        let client = reqwest::Client::new();
        let api = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_client(client.clone())
            .build();
        assert_eq!(format!("{:?}", api.client), format!("{:?}", client));
    }
}
