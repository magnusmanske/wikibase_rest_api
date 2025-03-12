use crate::{bearer_token::BearerToken, RestApi, RestApiError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The default user agent
const DEFAULT_USER_AGENT: &str = "Rust Wikibase REST API";

/// The latest supported version of the Wikibase REST API
const WIKIBASE_REST_API_VERSION: u8 = 1;

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
    pub fn with_api<S: Into<String>>(mut self, api_url: S) -> Self {
        self.api_url = Some(api_url.into());
        self
    }

    /// Sets the API version (u8). Default is 1.
    pub const fn with_api_version(mut self, api_version: u8) -> Self {
        self.api_version = Some(api_version);
        self
    }

    /// Sets the `OAuth2` bearer token.
    pub fn with_access_token<S: Into<String>>(mut self, access_token: S) -> Self {
        self.token.set_access_token(access_token);
        self
    }

    /// Sets the user agent. By default, the user agent is "Rust Wikibase REST API; {`package_name`}/{`package_version`}"
    pub fn with_user_agent<S: Into<String>>(mut self, user_agent: S) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Sets the `reqwest::Client`. By default, a new `reqwest::Client` is created.
    pub fn with_client(mut self, client: reqwest::Client) -> Self {
        self.client = client;
        self
    }

    /// Sets the interval for bearer token renewal. By default, the interval is `DEFAULT_RENEWAL_INTERVAL_SEC`.
    #[cfg(not(tarpaulin_include))]
    pub const fn with_access_token_renewal(
        mut self,
        renewal_interval: std::time::Duration,
    ) -> Self {
        self.renewal_interval = Some(renewal_interval);
        self
    }

    /// Sets the `OAuth2` client ID and client secret
    #[cfg(not(tarpaulin_include))]
    pub fn with_oauth2_info<S1: Into<String>, S2: Into<String>>(
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
        let user_agent = self
            .user_agent
            .clone()
            .unwrap_or(Self::default_user_agent());
        Ok(RestApi::new(
            self.client.clone(),
            user_agent,
            api_url,
            self.api_version.unwrap_or(WIKIBASE_REST_API_VERSION),
            Arc::new(RwLock::new(token)),
        ))
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
        let builder = RestApiBuilder::default().with_api("https://www.wikidata.org/w/api.php");
        let api_url = builder.validate_api_url();
        assert!(api_url.is_err());
    }

    #[test]
    fn test_validate_api_url_rest_api() {
        let builder = RestApiBuilder::default().with_api("https://www.wikidata.org/w/rest.php");
        let api_url = builder.validate_api_url();
        assert!(api_url.is_ok());
    }

    #[test]
    fn test_user_agent() {
        let api1 = RestApi::builder()
            .with_api("https://test.wikidata.org/w/rest.php")
            .build()
            .unwrap();
        assert_eq!(api1.user_agent(), RestApiBuilder::default_user_agent());

        let api2 = RestApi::builder()
            .with_user_agent("Test User Agent")
            .with_api("https://test.wikidata.org/w/rest.php")
            .build()
            .unwrap();
        assert_eq!(api2.user_agent(), "Test User Agent");
    }
}
