use reqwest::header::InvalidHeaderValue;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};
use thiserror::Error;

/// Maximum number of characters of offending JSON shown in an error message.
const MAX_JSON_SNIPPET_CHARS: usize = 200;

/// Renders JSON for an error message, truncated so that a large document
/// (e.g. a whole entity) doesn't flood logs. The full value stays available
/// on the error variant itself.
fn json_snippet(j: &Value) -> String {
    let s = j.to_string();
    match s.char_indices().nth(MAX_JSON_SNIPPET_CHARS) {
        Some((cut, _)) => format!("{}…", &s[..cut]),
        None => s,
    }
}

/// The error body returned by the server. Wikibase uses `code`/`message`/`context`;
/// the OAuth endpoints use `error`/`message`, which is accepted as an alias for `code`.
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct RestApiErrorPayload {
    #[serde(default, alias = "error")]
    code: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    context: HashMap<String, Value>,
}

impl RestApiErrorPayload {
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn context(&self) -> &HashMap<String, Value> {
        &self.context
    }

    /// Returns the `resource_type` context of a `resource-not-found` error
    /// (e.g. `"item"` or `"aliases"`), which tells *what* was missing.
    pub fn resource_type(&self) -> Option<&str> {
        self.context.get("resource_type")?.as_str()
    }
}

impl Display for RestApiErrorPayload {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}: {} / {}",
            self.code,
            self.message,
            json!(self.context)
        )
    }
}

#[derive(Error, Debug)]
pub enum RestApiError {
    #[error("API error {status}: {payload}")]
    ApiError {
        status: reqwest::StatusCode,
        status_text: String,
        payload: RestApiErrorPayload,
    },
    #[error("Client ID required")]
    ClientIdRequired,
    #[error("Client secret required")]
    ClientSecretRequired,
    #[error("Refresh token required")]
    RefreshTokenRequired,
    #[error("Access token required")]
    AccessTokenRequired,
    #[error("Reqwest Error: {0}")]
    Reqwest(reqwest::Error),
    #[error("Invalid header value: {0}")]
    InvalidHeaderValue(InvalidHeaderValue),
    #[error("Method {method} not implemented for path {path} in REST API")]
    NotImplementedInRestApi {
        method: reqwest::Method,
        path: String,
    },
    #[error("Missing ID")]
    MissingId,
    #[error("ID already set")]
    HasId,
    #[error("Missing field {field}: {}", json_snippet(.j))]
    MissingOrInvalidField { field: String, j: Value },
    #[error("Wrong type for {field}: {}", json_snippet(.j))]
    WrongType { field: String, j: Value },
    #[error("Entity ID is None")]
    IsNone,
    #[error("Unrecognized entity ID letter: {0}")]
    UnknownEntityLetter(String),
    #[error("Invalid entity ID: {0}")]
    InvalidEntityId(String),
    #[error("Unknown value: {0}")]
    UnknownValue(String),
    #[error("Serde JSON error: {0}")]
    SerdeJson(serde_json::Error),
    #[error("Unknown statement rank: {0}")]
    UnknownStatementRank(String),
    #[error("API not set")]
    ApiNotSet,
    #[error("Empty value: {0}")]
    EmptyValue(String),
    #[error("Invalid language code: {0}")]
    InvalidLanguageCode(String),
    #[error("Invalid site ID: {0}")]
    InvalidSiteId(String),
    #[error("Unsupported method: {0}")]
    UnsupportedMethod(reqwest::Method),
    #[error("REST API URL is invalid: {0}")]
    RestApiUrlInvalid(String),
    #[error("Invalid precision")]
    InvalidPrecision,
    #[error("Missing results field in response")]
    MissingResults,
    #[error("REST API path not implemented: {0}")]
    PathNotImplemented(String),
}

impl From<reqwest::Error> for RestApiError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl From<InvalidHeaderValue> for RestApiError {
    fn from(e: InvalidHeaderValue) -> Self {
        Self::InvalidHeaderValue(e)
    }
}

impl From<serde_json::Error> for RestApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson(e)
    }
}

impl RestApiError {
    /// Returns `true` if this is an API error with HTTP status 429 (Too Many Requests).
    pub const fn is_rate_limited(&self) -> bool {
        matches!(
            self,
            RestApiError::ApiError { status, .. }
                if status.as_u16() == reqwest::StatusCode::TOO_MANY_REQUESTS.as_u16()
        )
    }

    /// Returns `true` if this is an API error with HTTP status 404 (Not Found).
    pub const fn is_not_found(&self) -> bool {
        matches!(
            self,
            RestApiError::ApiError { status, .. }
                if status.as_u16() == reqwest::StatusCode::NOT_FOUND.as_u16()
        )
    }

    /// Returns `true` if this is a 404 `resource-not-found` for the given resource type
    /// (e.g. `"aliases"`), as opposed to e.g. the entity itself not existing.
    pub fn is_missing_resource(&self, resource_type: &str) -> bool {
        matches!(
            self,
            RestApiError::ApiError { status, payload, .. }
                if *status == reqwest::StatusCode::NOT_FOUND
                    && payload.resource_type() == Some(resource_type)
        )
    }

    pub async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let status_text = status.canonical_reason().unwrap_or_default().to_string();
        let payload = response.json().await.unwrap_or_default();
        RestApiError::ApiError {
            status,
            status_text,
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use serde_json::json;

    #[test]
    fn test_rest_api_error_payload() {
        let payload = RestApiErrorPayload {
            code: "code".to_string(),
            message: "message".to_string(),
            context: HashMap::from([("key".to_string(), json!("value"))]),
        };
        assert_eq!(payload.code(), "code");
        assert_eq!(payload.message(), "message");
        assert_eq!(payload.context().get("key").unwrap(), &json!("value"));
    }

    #[test]
    fn test_rest_api_error_display() {
        let payload = RestApiErrorPayload {
            code: "code".to_string(),
            message: "message".to_string(),
            context: HashMap::from([("key".to_string(), json!("value"))]),
        };
        let error = RestApiError::ApiError {
            status: reqwest::StatusCode::BAD_REQUEST,
            status_text: "Bad Request".to_string(),
            payload,
        };
        assert_eq!(
            error.to_string(),
            "API error 400 Bad Request: code: message / {\"key\":\"value\"}"
        );
    }

    #[test]
    fn test_payload_resource_type() {
        let payload: RestApiErrorPayload = serde_json::from_value(json!({
            "code": "resource-not-found",
            "message": "The requested resource does not exist",
            "context": {"resource_type": "aliases"}
        }))
        .unwrap();
        assert_eq!(payload.resource_type(), Some("aliases"));
        assert_eq!(RestApiErrorPayload::default().resource_type(), None);
    }

    #[test]
    fn test_is_missing_resource() {
        let error = |status, resource_type: &str| RestApiError::ApiError {
            status,
            status_text: String::new(),
            payload: serde_json::from_value(json!({
                "code": "resource-not-found",
                "message": "m",
                "context": {"resource_type": resource_type}
            }))
            .unwrap(),
        };
        assert!(error(reqwest::StatusCode::NOT_FOUND, "aliases").is_missing_resource("aliases"));
        assert!(!error(reqwest::StatusCode::NOT_FOUND, "item").is_missing_resource("aliases"));
        assert!(!error(reqwest::StatusCode::BAD_REQUEST, "aliases").is_missing_resource("aliases"));
        assert!(!RestApiError::MissingId.is_missing_resource("aliases"));
    }

    #[test]
    fn test_payload_oauth_error_alias() {
        // OAuth endpoints report `error` rather than `code`.
        let payload: RestApiErrorPayload =
            serde_json::from_value(json!({"error": "invalid_grant", "message": "bad token"}))
                .unwrap();
        assert_eq!(payload.code(), "invalid_grant");
        assert_eq!(payload.message(), "bad token");
    }

    #[test]
    fn test_json_snippet_truncates() {
        let long = json!("x".repeat(1000));
        let error = RestApiError::WrongType {
            field: "f".into(),
            j: long.clone(),
        };
        let msg = error.to_string();
        assert!(msg.ends_with('…'));
        assert!(msg.chars().count() < 250);
        // Short values are shown in full.
        assert_eq!(json_snippet(&json!({"a": 1})), "{\"a\":1}");
        // The full value is still available on the variant.
        match error {
            RestApiError::WrongType { j, .. } => assert_eq!(j, long),
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_error_reqwest() {
        let error = reqwest::get("not a url").await.unwrap_err();
        let rest_api_error: RestApiError = error.into();
        assert_eq!(rest_api_error.to_string(), "Reqwest Error: builder error");
    }

    #[test]
    fn test_invalid_header_value() {
        let error = HeaderValue::from_str("\u{0}").unwrap_err();
        let rest_api_error: RestApiError = error.into();
        assert_eq!(
            rest_api_error.to_string(),
            "Invalid header value: failed to parse header value"
        );
    }

    #[test]
    fn test_payload_code() {
        let payload = RestApiErrorPayload {
            code: "code".to_string(),
            message: "message".to_string(),
            context: HashMap::new(),
        };
        assert_eq!(payload.code(), "code");
    }

    #[test]
    fn test_payload_message() {
        let payload = RestApiErrorPayload {
            code: "code".to_string(),
            message: "message".to_string(),
            context: HashMap::new(),
        };
        assert_eq!(payload.message(), "message");
    }

    #[test]
    fn test_payload_context() {
        let payload = RestApiErrorPayload {
            code: "code".to_string(),
            message: "message".to_string(),
            context: HashMap::from([("key".to_string(), json!("value"))]),
        };
        assert_eq!(payload.context().get("key").unwrap(), &json!("value"));
    }

    #[test]
    fn test_payload_fmt() {
        let payload = RestApiErrorPayload {
            code: "code".to_string(),
            message: "message".to_string(),
            context: HashMap::from([("key".to_string(), json!("value"))]),
        };
        let s = format!("{payload}");
        assert_eq!(s, "code: message / {\"key\":\"value\"}");
    }

    #[test]
    fn test_from_serde_json_error() {
        let error = serde_json::from_str::<Value>("{").unwrap_err();
        let rest_api_error: RestApiError = error.into();
        assert_eq!(
            rest_api_error.to_string(),
            "Serde JSON error: EOF while parsing an object at line 1 column 1"
        );
    }
}
