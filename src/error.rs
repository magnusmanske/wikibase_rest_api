use reqwest::header::InvalidHeaderValue;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct RestApiErrorPayload {
    code: String,
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
    #[error("ApiError: {status} {status_text} / {payload:?}")]
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
    #[error("Unexpected response: {0}")]
    UnexpectedResponse(Value),
    #[error("Missing ID")]
    MissingId,
    #[error("ID already set")]
    HasId,
    #[error("Missing field {field}: {j}")]
    MissingOrInvalidField { field: String, j: Value },
    #[error("Wrong type for {field}: {j}")]
    WrongType { field: String, j: Value },
    #[error("Entity ID is None")]
    IsNone,
    #[error("Unrecognized entity ID letter: {0}")]
    UnknownEntityLetter(String),
    #[error("Unknown value: {0}")]
    UnknownValue(String),
    #[error("Unknown data type: {0}")]
    UnknownDataType(String),
    #[error("Serde JSON error: {0}")]
    SerdeJson(serde_json::Error),
    #[error("Unknown statement rank: {0}")]
    UnknownStatementRank(String),
    #[error("API not set")]
    ApiNotSet,
    #[error("Empty value: {0}")]
    EmptyValue(String),
    #[error("Unsupported method: {0}")]
    UnsupportedMethod(reqwest::Method),
    #[error("REST API URL is invalid: {0}")]
    RestApiUrlInvalid(String),
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
    pub async fn from_response(response: reqwest::Response) -> Self {
        let status = response.status();
        let status_text = status.canonical_reason().unwrap_or_default().to_owned();
        let payload = response
            .json()
            .await
            .unwrap_or(RestApiErrorPayload::default());
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
            code: "code".to_owned(),
            message: "message".to_owned(),
            context: [("key".to_owned(), json!("value"))]
                .iter()
                .cloned()
                .collect(),
        };
        assert_eq!(payload.code(), "code");
        assert_eq!(payload.message(), "message");
        assert_eq!(payload.context().get("key").unwrap(), &json!("value"));
    }

    #[test]
    fn test_rest_api_error_display() {
        let payload = RestApiErrorPayload {
            code: "code".to_owned(),
            message: "message".to_owned(),
            context: [("key".to_owned(), json!("value"))]
                .iter()
                .cloned()
                .collect(),
        };
        let error = RestApiError::ApiError {
            status: reqwest::StatusCode::BAD_REQUEST,
            status_text: "Bad Request".to_owned(),
            payload,
        };
        assert_eq!(
            error.to_string(),
            "ApiError: 400 Bad Request Bad Request / RestApiErrorPayload { code: \"code\", message: \"message\", context: {\"key\": String(\"value\")} }"
        );
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
            code: "code".to_owned(),
            message: "message".to_owned(),
            context: HashMap::new(),
        };
        assert_eq!(payload.code(), "code");
    }

    #[test]
    fn test_payload_message() {
        let payload = RestApiErrorPayload {
            code: "code".to_owned(),
            message: "message".to_owned(),
            context: HashMap::new(),
        };
        assert_eq!(payload.message(), "message");
    }

    #[test]
    fn test_payload_context() {
        let payload = RestApiErrorPayload {
            code: "code".to_owned(),
            message: "message".to_owned(),
            context: [("key".to_owned(), json!("value"))]
                .iter()
                .cloned()
                .collect(),
        };
        assert_eq!(payload.context().get("key").unwrap(), &json!("value"));
    }

    #[test]
    fn test_payload_fmt() {
        let payload = RestApiErrorPayload {
            code: "code".to_owned(),
            message: "message".to_owned(),
            context: [("key".to_owned(), json!("value"))]
                .iter()
                .cloned()
                .collect(),
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
