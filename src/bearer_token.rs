use crate::{RestApi, RestApiError};
use reqwest::Request;
use serde_json::Value;
use std::{collections::HashMap, time::Duration};

/// The default time to wait until bearer token is renewed. API says 4h so setting it to 3h50min
const DEFAULT_RENEWAL_INTERVAL: Duration = Duration::from_secs((3 * 60 + 50) * 60);

#[derive(Debug, Clone, Default)]
pub struct BearerToken {
    client_id: Option<String>,
    client_secret: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    last_update: Option<std::time::Instant>,
    /// `None` means "use `DEFAULT_RENEWAL_INTERVAL`".
    renewal_interval: Option<Duration>,
}

impl BearerToken {
    /// Returns the `OAuth2` bearer token
    pub fn get(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    /// For non-owner-only clients, returns a URL to send the user to login and authorize the client.
    /// Upon authorizing, the user will be redirected to the URL with a code, which can be exchanged for an access token, via `get_access_token`.
    pub fn authorization_code_url(&self, api: &RestApi) -> Result<String, RestApiError> {
        let client_id = self
            .client_id
            .as_ref()
            .ok_or(RestApiError::ClientIdRequired)?;
        Ok(format!(
            "{}/oauth2/authorize?client_id={}&response_type=code",
            api.api_url(),
            client_id
        ))
    }

    /// Returns the renewal interval for the `OAuth2` bearer token.
    pub fn access_token_renewal_interval(&self) -> Duration {
        self.renewal_interval.unwrap_or(DEFAULT_RENEWAL_INTERVAL)
    }

    /// Internal use only.
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    /// Internal use only.
    pub fn client_secret(&self) -> Option<&str> {
        self.client_secret.as_deref()
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

        Ok(HashMap::from([
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), client_id.clone()),
            ("client_secret".to_string(), client_secret.clone()),
            ("code".to_string(), code.to_string()),
        ]))
    }

    async fn generate_get_access_token_request(
        &self,
        api: &RestApi,
        code: &str,
    ) -> Result<Request, RestApiError> {
        let params = self.generate_get_access_token_parameters(code)?;
        let headers = api.headers_from_token(self).await?;
        let url = format!("{}/oauth2/access_token", api.api_url());
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
    ///
    /// # Errors
    /// Returns an `RestApiError` if the request fails.
    pub async fn get_access_token(
        &mut self,
        api: &RestApi,
        code: &str,
    ) -> Result<(), RestApiError> {
        let request = self.generate_get_access_token_request(api, code).await?;
        self.execute_token_request(api, request).await
    }

    /// Sends a request to the `OAuth2` token endpoint and stores the returned tokens.
    /// A non-success response surfaces as `RestApiError::ApiError` carrying the server's
    /// OAuth error (e.g. `invalid_grant`), rather than a misleading "token required".
    async fn execute_token_request(
        &mut self,
        api: &RestApi,
        request: Request,
    ) -> Result<(), RestApiError> {
        let response = api.client().execute(request).await?;
        if !response.status().is_success() {
            return Err(RestApiError::from_response(response).await);
        }
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
        // Renew at 90% of the token lifetime; fall back to the default if none is given.
        self.renewal_interval = j["expires_in"]
            .as_u64()
            .filter(|&secs| secs > 0)
            .map(|secs| Duration::from_secs(secs / 10 * 9));
        self.set_tokens(Some(access_token), Some(refresh_token));
        self.touch_access_token();
        Ok(())
    }

    /// Updates the last bearer token update time to current time
    fn touch_access_token(&mut self) {
        self.last_update = Some(std::time::Instant::now());
    }

    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    /// Sets the renewal interval for the `OAuth2` bearer token
    pub const fn set_renewal_interval(&mut self, renewal_interval: Duration) {
        self.renewal_interval = Some(renewal_interval);
    }

    /// Sets the `OAuth2` bearer token and refresh token
    pub fn set_tokens(&mut self, access_token: Option<String>, refresh_token: Option<String>) {
        self.access_token = access_token;
        self.refresh_token = refresh_token;
    }

    /// Returns `true` if a request with this method requires an up-front token renewal.
    /// Read-only, so the hot path (GETs and fresh tokens) needs only a read lock.
    pub(crate) fn needs_renewal(&self, method: &reqwest::Method) -> bool {
        *method != reqwest::Method::GET
            && self.can_update_access_token()
            && self.does_access_token_need_updating()
    }

    /// Sets the `OAuth2` bearer token (owner-only clients are supported)
    pub fn set_access_token<S: Into<String>>(&mut self, access_token: S) {
        self.access_token = Some(access_token.into());
    }

    /// Sets the `OAuth2` client ID and client secret
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
            if elapsed < self.access_token_renewal_interval() {
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
            .ok_or(RestApiError::RefreshTokenRequired)?;
        Ok(HashMap::from([
            ("client_id".to_string(), client_id.clone()),
            ("client_secret".to_string(), client_secret.clone()),
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("refresh_token".to_string(), refresh_token.clone()),
        ]))
    }

    async fn get_renew_access_token_request(&self, api: &RestApi) -> Result<Request, RestApiError> {
        let params = self.get_renew_access_token_parameters()?;
        let headers = api.headers_from_token(self).await?;
        let url = format!("{}/oauth2/access_token", api.api_url());
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
        self.execute_token_request(api, request).await
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
        // Never updated: needs updating.
        assert!(token.does_access_token_need_updating());
        // Just updated, default interval: fresh.
        token.touch_access_token();
        assert!(!token.does_access_token_need_updating());
        // A zero interval means it is immediately stale again.
        token.set_renewal_interval(Duration::ZERO);
        assert!(token.does_access_token_need_updating());
    }

    #[test]
    fn test_get() {
        let mut token = BearerToken::default();
        assert_eq!(token.get(), None);
        token.set_access_token("test");
        assert_eq!(token.get(), Some("test"));
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_authorization_code_url() {
        let mut token = BearerToken::default();
        let api = RestApi::builder("https://www.wikidata.org/w/rest.php")
            .unwrap()
            .build()
            .unwrap();
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
        assert_eq!(token.get(), Some("foo"));
        assert_eq!(token.refresh_token(), Some("bar"));
        assert_eq!(
            token.renewal_interval,
            Some(Duration::from_secs(3600 / 10 * 9))
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
            .build()
            .unwrap();

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
        assert_eq!(api.token.read().await.get().unwrap(), "access_token_foobar");
        assert_eq!(
            api.token.read().await.refresh_token().unwrap(),
            "refresh_token_foobar"
        );
        assert_eq!(
            api.token.read().await.renewal_interval,
            Some(Duration::from_secs(3600 / 10 * 9))
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
            .build()
            .unwrap();

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
            api.token.read().await.get().unwrap(),
            "access_token_foobar2"
        );
        assert_eq!(
            api.token.read().await.refresh_token().unwrap(),
            "refresh_token_foobar2"
        );
        assert_eq!(
            api.token.read().await.renewal_interval,
            Some(Duration::from_secs(3600 / 10 * 9))
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_renew_access_token_no_need() {
        let api = RestApi::builder("https://test.wikidata.org/w/rest.php")
            .unwrap()
            .build()
            .unwrap();
        let mut bt = BearerToken::default();
        bt.touch_access_token();
        bt.set_renewal_interval(Duration::from_secs(3600));
        // This will fail if not for "no update needed", since client ID and secret are not set
        assert!(bt.renew_access_token(&api).await.is_ok());
    }

    #[test]
    fn test_generate_get_access_token_parameters() {
        let mut token = BearerToken::default();
        token.set_oauth2_info("test_client", "test_secret");
        let params = token
            .generate_get_access_token_parameters("test_code")
            .unwrap();
        assert_eq!(params.get("grant_type").unwrap(), "authorization_code");
        assert_eq!(params.get("client_id").unwrap(), "test_client");
        assert_eq!(params.get("client_secret").unwrap(), "test_secret");
        assert_eq!(params.get("code").unwrap(), "test_code");
    }

    #[test]
    fn test_get_renew_access_token_parameters() {
        let mut token = BearerToken::default();
        token.set_oauth2_info("test_client", "test_secret");
        token.set_tokens(None, Some("test_refresh".to_string()));
        let params = token.get_renew_access_token_parameters().unwrap();
        assert_eq!(params.get("grant_type").unwrap(), "refresh_token");
        assert_eq!(params.get("client_id").unwrap(), "test_client");
        assert_eq!(params.get("client_secret").unwrap(), "test_secret");
        assert_eq!(params.get("refresh_token").unwrap(), "test_refresh");
    }

    #[test]
    fn test_generate_get_access_token_parameters_missing_secret() {
        // client_id present but client_secret missing -> ClientSecretRequired.
        let token = BearerToken {
            client_id: Some("id".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            token.generate_get_access_token_parameters("code"),
            Err(RestApiError::ClientSecretRequired)
        ));
    }

    #[test]
    fn test_get_renew_access_token_parameters_missing_secret() {
        let token = BearerToken {
            client_id: Some("id".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            token.get_renew_access_token_parameters(),
            Err(RestApiError::ClientSecretRequired)
        ));
    }

    #[test]
    fn test_get_renew_access_token_parameters_missing_refresh() {
        // client_id + client_secret present, but no refresh token -> RefreshTokenRequired.
        let mut token = BearerToken::default();
        token.set_oauth2_info("id", "secret");
        assert!(matches!(
            token.get_renew_access_token_parameters(),
            Err(RestApiError::RefreshTokenRequired)
        ));
    }

    #[test]
    fn test_set_tokens_from_json_missing_refresh() {
        // access_token present but refresh_token missing -> RefreshTokenRequired.
        let mut token = BearerToken::default();
        let j = serde_json::json!({"access_token": "foo"});
        assert!(matches!(
            token.set_tokens_from_json(j),
            Err(RestApiError::RefreshTokenRequired)
        ));
    }

    #[test]
    fn test_needs_renewal() {
        let mut token = BearerToken::default();
        // No OAuth2 info: never needs renewal.
        assert!(!token.needs_renewal(&reqwest::Method::POST));
        token.set_oauth2_info("id", "secret");
        // GET is always read-only, so no renewal.
        assert!(!token.needs_renewal(&reqwest::Method::GET));
        // A non-GET with OAuth2 info and no prior update needs a renewal.
        assert!(token.needs_renewal(&reqwest::Method::POST));
    }

    #[test]
    fn test_set_tokens_from_json_without_expiry_uses_default() {
        let mut token = BearerToken::default();
        token.set_renewal_interval(Duration::from_secs(1));
        let j = json!({"access_token": "a", "refresh_token": "r"});
        token.set_tokens_from_json(j).unwrap();
        assert_eq!(
            token.access_token_renewal_interval(),
            DEFAULT_RENEWAL_INTERVAL
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_renew_access_token_error_status() {
        // An OAuth error response surfaces as ApiError with the OAuth error code,
        // not as a misleading AccessTokenRequired.
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/w/rest.php/oauth2/access_token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "invalid_grant",
                "message": "The refresh token is invalid.",
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();
        let mut token = BearerToken::default();
        token.set_oauth2_info("id", "secret");
        token.set_tokens(None, Some("refresh".to_string()));
        match token.renew_access_token(&api).await.unwrap_err() {
            RestApiError::ApiError {
                status, payload, ..
            } => {
                assert_eq!(status, 400);
                assert_eq!(payload.code(), "invalid_grant");
            }
            e => panic!("Wrong error type: {e:?}"),
        }
        assert_eq!(token.get(), None);
    }
}
