use std::collections::HashMap;

use reqwest::Request;
use serde_json::Value;

use crate::{RestApi, RestApiError};

/// The default time to wait until bearer token is renewed. API says 4h so setting it to 3h50min
const DEFAULT_RENEWAL_INTERVAL_SEC: u64 = (3 * 60 + 50) * 60;

#[derive(Debug, Clone, Default)]
pub struct BearerToken {
    client_id: Option<String>,
    client_secret: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    last_update: Option<std::time::Instant>,
    renewal_interval: std::time::Duration,
}

impl BearerToken {
    /// Returns the `OAuth2` bearer token
    pub const fn get(&self) -> &Option<String> {
        &self.access_token
    }

    /// For non-owner-only clients, returns a URL to send the user to login and authorize the client.
    /// Upon authorizing, the user will be redirected to the URL with a code, which can be exchanged for an access token, via `get_access_token`.
    pub fn authorization_code_url(&self, api: &RestApi) -> Result<String, RestApiError> {
        let client_id = self
            .client_id
            .as_ref()
            .ok_or_else(|| RestApiError::ClientIdRequired)?;
        let api_url = api.api_url();
        Ok(format!(
            "{api_url}/oauth2/authorize?client_id={client_id}&response_type=code"
        ))
    }

    /// Returns the renewal interval for the `OAuth2` bearer token.
    pub const fn access_token_renewal_interval(&self) -> std::time::Duration {
        self.renewal_interval
    }

    /// Internal use only.
    pub const fn client_id(&self) -> &Option<String> {
        &self.client_id
    }

    /// Internal use only.
    pub const fn client_secret(&self) -> &Option<String> {
        &self.client_secret
    }

    fn generate_get_access_token_parameters(
        &self,
        code: &str,
    ) -> Result<HashMap<String, String>, RestApiError> {
        let client_id = self
            .client_id
            .as_ref()
            .ok_or(RestApiError::ClientIdRequired)?;
        let client_secret = self
            .client_secret
            .as_ref()
            .ok_or(RestApiError::ClientSecretRequired)?;

        let params = [
            ("grant_type", "authorization_code"),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
        ];
        Ok(Self::array2hashmap(&params))
    }

    async fn generate_get_access_token_request(
        &self,
        api: &RestApi,
        code: &str,
    ) -> Result<Request, RestApiError> {
        let params = self.generate_get_access_token_parameters(code)?;
        let headers = api.headers_from_token(self).await?;
        let url = format!("{api_url}/oauth2/access_token", api_url = api.api_url());
        let mut request = api
            .client()
            .post(url)
            .headers(headers)
            .form(&params)
            .build()?;
        request.headers_mut().insert(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse()?,
        );
        Ok(request)
    }

    /// Exchanges a code for an access token
    pub async fn get_access_token(
        &mut self,
        api: &RestApi,
        code: &str,
    ) -> Result<(), RestApiError> {
        let request = self.generate_get_access_token_request(api, code).await?;
        let response = api.client().execute(request).await?;
        let j: Value = response.json().await?;
        self.set_tokens_from_json(j)
    }

    /// Sets the `OAuth2` bearer token and refresh token from a JSON response
    fn set_tokens_from_json(&mut self, j: Value) -> Result<(), RestApiError> {
        let access_token = j["access_token"]
            .as_str()
            .ok_or(RestApiError::AccessTokenRequired)?
            .to_string();
        let refresh_token = j["refresh_token"]
            .as_str()
            .ok_or(RestApiError::RefreshTokenRequired)?
            .to_string();
        let renewal_interval = j["expires_in"].as_u64().unwrap_or_default() / 10 * 9; // 90% of max duration
        self.set_tokens(Some(access_token), Some(refresh_token));
        self.set_renewal_interval(renewal_interval);
        self.touch_access_token();
        Ok(())
    }

    /// Updates the last bearer token update time to current time
    fn touch_access_token(&mut self) {
        self.last_update = Some(std::time::Instant::now());
    }

    pub const fn refresh_token(&self) -> &Option<String> {
        &self.refresh_token
    }

    /// Sets the renewal interval for the `OAuth2` bearer token
    pub fn set_renewal_interval(&mut self, renewal_interval: u64) {
        let renewal_interval = match renewal_interval {
            0 => DEFAULT_RENEWAL_INTERVAL_SEC,
            renewal_interval => renewal_interval,
        };
        self.renewal_interval = std::time::Duration::from_secs(renewal_interval);
    }

    /// Sets the `OAuth2` bearer token and refresh token
    pub fn set_tokens(&mut self, access_token: Option<String>, refresh_token: Option<String>) {
        self.access_token = access_token;
        self.refresh_token = refresh_token;
    }

    /// Checks if the bearer token needs to be updated, and updates it if necessary
    pub async fn check(&mut self, api: &RestApi, request: &Request) -> Result<(), RestApiError> {
        let method = request.method();
        if method == reqwest::Method::GET {
            return Ok(());
        }
        if self.can_update_access_token() {
            self.renew_access_token(api).await?;
        }
        Ok(())
    }

    /// Sets the `OAuth2` bearer token (owner-only clients are supported)
    pub fn set_access_token<S: Into<String>>(&mut self, access_token: S) {
        self.access_token = Some(access_token.into());
    }

    //// Sets the OAuth2 client ID and client secret
    pub fn set_oauth2_info<S1: Into<String>, S2: Into<String>>(
        &mut self,
        client_id: S1,
        client_secret: S2,
    ) {
        self.client_id = Some(client_id.into());
        self.client_secret = Some(client_secret.into());
    }

    /// Returns `true` if an `OAuth2` bearer token is present
    pub const fn has_access_token(&self) -> bool {
        self.access_token.is_some()
    }

    /// Returns `true` if the client ID and client secret are present
    const fn can_update_access_token(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }

    /// Check if last bearer token update is within the renewal interval
    fn does_access_token_need_updating(&self) -> bool {
        if let Some(last_update) = self.last_update {
            let elapsed = last_update.elapsed();
            if elapsed < self.renewal_interval {
                return false;
            }
        }
        true
    }

    fn get_renew_access_token_parameters(&self) -> Result<HashMap<String, String>, RestApiError> {
        let client_id = self
            .client_id
            .as_ref()
            .ok_or(RestApiError::ClientIdRequired)?;
        let client_secret = self
            .client_secret
            .as_ref()
            .ok_or(RestApiError::ClientSecretRequired)?;
        let refresh_token = self
            .refresh_token
            .as_ref()
            .ok_or_else(|| RestApiError::RefreshTokenRequired)?;
        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
        ];
        Ok(Self::array2hashmap(&params))
    }

    async fn get_renew_access_token_request(&self, api: &RestApi) -> Result<Request, RestApiError> {
        let params = self.get_renew_access_token_parameters()?;
        let headers = api.headers_from_token(self).await?;
        let url = format!("{}{}", api.api_url(), "/oauth2/access_token");
        let mut request = api
            .client()
            .post(url)
            .headers(headers)
            .form(&params)
            .build()?;

        request.headers_mut().insert(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse()?,
        );
        Ok(request)
    }

    /// Refresh the `OAuth2` bearer token for Non-owner-only clients
    pub async fn renew_access_token(&mut self, api: &RestApi) -> Result<(), RestApiError> {
        if !self.does_access_token_need_updating() {
            return Ok(());
        }
        let request = self.get_renew_access_token_request(api).await?;
        let response = api.client().execute(request).await?;
        let j: Value = response.json().await?;
        self.set_tokens_from_json(j)
    }

    fn array2hashmap(array: &[(&str, &str)]) -> HashMap<String, String> {
        array
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_has_access_token() {
        let mut token = BearerToken::default();
        assert!(!token.has_access_token());
        token.set_access_token("test");
        assert!(token.has_access_token());
    }

    #[test]
    fn test_can_update_access_token() {
        let mut token = BearerToken::default();
        assert!(!token.can_update_access_token());
        token.set_oauth2_info("client_id", "client_secret");
        assert!(token.can_update_access_token());
    }

    #[test]
    fn test_does_access_token_need_updating() {
        let mut token = BearerToken::default();
        assert!(token.does_access_token_need_updating());
        token.touch_access_token();
        assert!(token.does_access_token_need_updating());
        token.set_renewal_interval(0);
        assert!(!token.does_access_token_need_updating());
    }

    #[test]
    fn test_get() {
        let mut token = BearerToken::default();
        assert_eq!(token.get(), &None);
        token.set_access_token("test");
        assert_eq!(token.get(), &Some("test".to_string()));
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_authorization_code_url() {
        let mut token = BearerToken::default();
        let api = RestApi::builder("https://www.wikidata.org/w/rest.php")
            .unwrap()
            .build();
        token.set_oauth2_info("client_id", "client_secret");
        assert_eq!(token.authorization_code_url(&api).unwrap(), "https://www.wikidata.org/w/rest.php/oauth2/authorize?client_id=client_id&response_type=code");
    }

    #[test]
    fn test_set_tokens_from_json() {
        let mut token = BearerToken::default();
        let j = serde_json::json!({
            "access_token": "foo",
            "refresh_token": "bar",
            "expires_in": 3600,
        });
        token.set_tokens_from_json(j).unwrap();
        assert_eq!(token.get(), &Some("foo".to_string()));
        assert_eq!(token.refresh_token(), &Some("bar".to_string()));
        assert_eq!(
            token.renewal_interval,
            std::time::Duration::from_secs(3600 / 10 * 9)
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_get_access_token() {
        // #lizard forgives the complexity
        let client_id = "client_id_foobar";
        let client_secret = "client_secret_foobar";
        let code = "code_foobar";
        let mock_path = "/w/rest.php/oauth2/access_token";

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("grant_type=authorization_code"))
            .and(body_string_contains(format!("client_id={client_id}")))
            .and(body_string_contains(format!(
                "client_secret={client_secret}"
            )))
            .and(body_string_contains(format!("code={code}")))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access_token_foobar",
                "refresh_token": "refresh_token_foobar",
                "expires_in": 3600,
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        // Test error cases
        assert!(api
            .token
            .write()
            .await
            .get_access_token(&api, code)
            .await
            .is_err());

        // Test success case
        api.token
            .write()
            .await
            .set_oauth2_info(client_id, client_secret);
        api.token
            .write()
            .await
            .get_access_token(&api, code)
            .await
            .unwrap();
        assert_eq!(
            api.token.read().await.get().to_owned().unwrap(),
            "access_token_foobar"
        );
        assert_eq!(
            api.token.read().await.refresh_token().to_owned().unwrap(),
            "refresh_token_foobar"
        );
        assert_eq!(
            api.token.read().await.renewal_interval,
            std::time::Duration::from_secs(3600 / 10 * 9)
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_renew_access_token() {
        // #lizard forgives the complexity
        let client_id = "client_id_foobar";
        let client_secret = "client_secret_foobar";
        let refresh_token = "refresh_token_foobar";
        let mock_path = "/w/rest.php/oauth2/access_token";

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains(format!("client_id={client_id}")))
            .and(body_string_contains(format!(
                "client_secret={client_secret}"
            )))
            .and(body_string_contains(format!(
                "refresh_token={refresh_token}"
            )))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "access_token_foobar2",
                "refresh_token": "refresh_token_foobar2",
                "expires_in": 3600,
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        // Test error cases
        assert!(api
            .token
            .write()
            .await
            .renew_access_token(&api)
            .await
            .is_err());

        // Test success case
        api.token
            .write()
            .await
            .set_oauth2_info(client_id, client_secret);
        api.token
            .write()
            .await
            .set_tokens(None, Some("refresh_token_foobar".to_string()));
        api.token
            .write()
            .await
            .renew_access_token(&api)
            .await
            .unwrap();
        assert_eq!(
            api.token.read().await.get().to_owned().unwrap(),
            "access_token_foobar2"
        );
        assert_eq!(
            api.token.read().await.refresh_token().to_owned().unwrap(),
            "refresh_token_foobar2"
        );
        assert_eq!(
            api.token.read().await.renewal_interval,
            std::time::Duration::from_secs(3600 / 10 * 9)
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_renew_access_token_no_need() {
        let api = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .build();
        let mut bt = BearerToken::default();
        bt.touch_access_token();
        bt.renewal_interval = std::time::Duration::from_secs(3600);
        // This will fail if not for "no update needed", since client ID and secret are not set
        assert!(bt.renew_access_token(&api).await.is_ok());
    }

    #[test]
    fn test_array2hashmap() {
        let array = [("a", "1"), ("b", "2")];
        let hashmap = BearerToken::array2hashmap(&array);
        assert_eq!(hashmap.get("a"), Some(&"1".to_string()));
        assert_eq!(hashmap.get("b"), Some(&"2".to_string()));
    }
}
