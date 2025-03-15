use crate::{bearer_token::BearerToken, RestApi, RestApiError};
use std::sync::Arc;
use tokio::sync::RwLock;

/// The default user agent
const DEFAULT_USER_AGENT: &str = "Rust Wikibase REST API";

/// The latest supported version of the Wikibase REST API
const WIKIBASE_REST_API_VERSION: u8 = 1;

#[derive(Debug)]
pub struct RestApiBuilder {
    client: Option<reqwest::Client>,
    token: BearerToken,
    user_agent: Option<String>,
    api_url: String,
    api_version: Option<u8>,
    renewal_interval: Option<std::time::Duration>,
}

impl RestApiBuilder {
    /// Sets the REST API URL, specifically the URL ending in "rest.php". This in mandatory.
    /// # Errors
    /// Returns an error if REST API URL is invalid.
    pub fn new<S: Into<String>>(api_url: S) -> Result<Self, RestApiError> {
        let api_url = Self::validate_api_url(&api_url.into())?;
        Ok(Self {
            client: None,
            token: BearerToken::default(),
            user_agent: None,
            api_url,
            api_version: None,
            renewal_interval: None,
        })
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
        self.client = Some(client);
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

    /// Builds the `RestApi`. Returns an error if no REST API URL is set.
    /// The builder gets consumed by this operation.
    /// # Returns
    /// Returns a `RestApi` instance.
    pub fn build(self) -> RestApi {
        let api_url = self.api_url;
        let mut token = self.token;
        if let Some(interval) = self.renewal_interval {
            token.set_renewal_interval(interval.as_secs());
        }
        let token = Arc::new(RwLock::new(token));
        let user_agent = self.user_agent.unwrap_or(Self::default_user_agent());
        let api_version = self.api_version.unwrap_or(WIKIBASE_REST_API_VERSION);
        let client = self.client.unwrap_or_default(); // TODO check why miri fails here
        RestApi::new(client, user_agent, api_url, api_version, token)
    }

    /// Checks if the REST API URL is valid. The URL must end in "rest.php".
    /// Removes anything beyone that.
    fn validate_api_url(api_url: &str) -> Result<String, RestApiError> {
        let (base, _rest) = api_url
            .split_once("/rest.php")
            .ok_or_else(|| RestApiError::RestApiUrlInvalid(api_url.to_owned()))?;
        let ret = format!("{base}/rest.php");
        Ok(ret)
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
    use std::time::Duration;

    #[test]
    fn test_default_user_agent() {
        let user_agent = RestApiBuilder::default_user_agent();
        assert!(user_agent.starts_with(DEFAULT_USER_AGENT));
        assert!(user_agent.contains(env!("CARGO_PKG_NAME")));
        assert!(user_agent.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_validate_api_url_default() {
        let builder = RestApiBuilder::new("foobar");
        assert!(builder.is_err());
    }

    #[test]
    fn test_validate_api_url_api() {
        let builder = RestApiBuilder::new("https://www.wikidata.org/w/api.php");
        assert!(builder.is_err());
    }

    #[test]
    fn test_validate_api_url_rest_api() {
        let builder = RestApiBuilder::new("https://www.wikidata.org/w/rest.php");
        assert!(builder.is_ok());
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_user_agent() {
        let api1 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .build();
        assert_eq!(api1.user_agent(), RestApiBuilder::default_user_agent());

        let api2 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_user_agent("Test User Agent")
            .build();
        assert_eq!(api2.user_agent(), "Test User Agent");
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_with_api_version() {
        let api1 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .build();
        assert_eq!(api1.api_version(), WIKIBASE_REST_API_VERSION);

        let api2 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_api_version(2)
            .build();
        assert_eq!(api2.api_version(), 2);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_with_access_token_renewal() {
        let api1 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .build();
        assert_eq!(
            api1.token().read().await.access_token_renewal_interval(),
            Duration::from_secs(0)
        );

        let api2 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_access_token_renewal(std::time::Duration::from_secs(60))
            .build();
        assert_eq!(
            api2.token().read().await.access_token_renewal_interval(),
            Duration::from_secs(60)
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_with_oauth2_info() {
        let api1 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .build();
        assert_eq!(*api1.token().read().await.client_id(), None);
        assert_eq!(*api1.token().read().await.client_secret(), None);

        let api2 = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .with_oauth2_info("client_id", "client_secret")
            .build();
        assert_eq!(
            *api2.token().read().await.client_id(),
            Some("client_id".to_string())
        );
        assert_eq!(
            *api2.token().read().await.client_secret(),
            Some("client_secret".to_string())
        );
    }
}
