use reqwest::header::InvalidHeaderValue;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
};

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

#[derive(Debug)]
pub enum RestApiError {
    ApiError {
        status: reqwest::StatusCode,
        status_text: String,
        payload: RestApiErrorPayload,
    },
    ClientIdRequired,
    ClientSecretRequired,
    RefreshTokenRequired,
    AccessTokenRequired,
    Reqwest(reqwest::Error),
    InvalidHeaderValue(InvalidHeaderValue),
    NotImplementedInRestApi {
        method: reqwest::Method,
        path: String,
    },
    UnexpectedResponse(Value),
    MissingId,
    HasId,
    MissingOrInvalidField {
        field: String,
        j: Value,
    },
    WrongType {
        field: String,
        j: Value,
    },
    IsNone,
    UnknownEntityLetter(String),
    UnknownValue(String),
    UnknownDataType(String),
    SerdeJson(serde_json::Error),
    UnknownStatementRank(String),
    ApiNotSet,
    EmptyValue(String),
    UnsupportedMethod(reqwest::Method),
    RestApiUrlInvalid(String),
}

impl Display for RestApiError {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // #lizard forgives the complexity
        match self {
            RestApiError::ApiError {
                status,
                status_text,
                payload,
            } => {
                write!(f, "{} {} / {}", status, status_text, payload)
            }
            RestApiError::Reqwest(e) => {
                write!(f, "{}", e)
            }
            RestApiError::InvalidHeaderValue(e) => {
                write!(f, "{}", e)
            }
            RestApiError::NotImplementedInRestApi { method, path } => {
                write!(
                    f,
                    "Method {} not implemented for path {} in REST API",
                    method, path
                )
            }
            RestApiError::ClientIdRequired => {
                write!(f, "Client ID required")
            }
            RestApiError::ClientSecretRequired => {
                write!(f, "Client secret required")
            }
            RestApiError::RefreshTokenRequired => {
                write!(f, "Refresh token required")
            }
            RestApiError::AccessTokenRequired => {
                write!(f, "Access token required")
            }
            RestApiError::UnexpectedResponse(v) => {
                write!(f, "Unexpected response: {}", v)
            }
            RestApiError::MissingId => {
                write!(f, "Missing ID")
            }
            RestApiError::HasId => {
                write!(f, "ID already set")
            }
            RestApiError::MissingOrInvalidField { field, j } => {
                if j.get(field).is_none() {
                    write!(f, "Missing field {}: {}", field, j)
                } else {
                    write!(f, "Invalid field type for {}: {}", field, j)
                }
            }
            RestApiError::WrongType { field, j } => {
                write!(f, "Wrong type for {}: {}", field, j)
            }
            RestApiError::IsNone => {
                write!(f, "Entity ID is None")
            }
            RestApiError::UnknownEntityLetter(s) => {
                write!(f, "Unrecognized entity ID letter: {}", s)
            }
            RestApiError::UnknownValue(s) => {
                write!(f, "Unknown value: {}", s)
            }
            RestApiError::SerdeJson(e) => {
                write!(f, "{}", e)
            }
            RestApiError::ApiNotSet => {
                write!(f, "API not set")
            }
            RestApiError::UnknownDataType(s) => {
                write!(f, "Unknown data type: {}", s)
            }
            RestApiError::UnknownStatementRank(s) => {
                write!(f, "Unknown statement rank: {}", s)
            }
            RestApiError::EmptyValue(s) => {
                write!(f, "Empty value: {}", s)
            }
            RestApiError::UnsupportedMethod(m) => {
                write!(f, "Unsupported method: {}", m)
            }
            RestApiError::RestApiUrlInvalid(s) => {
                write!(f, "REST API URL does not contain '/rest.php': {}", s)
            }
        }
    }
}

impl Error for RestApiError {}

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
            "400 Bad Request Bad Request / code: message / {\"key\":\"value\"}"
        );
    }

    #[tokio::test]
    async fn test_error_reqwest() {
        let error = reqwest::get("not a url").await.unwrap_err();
        let rest_api_error: RestApiError = error.into();
        assert_eq!(rest_api_error.to_string(), "builder error");
    }

    #[test]
    fn test_invalid_header_value() {
        let error = HeaderValue::from_str("\u{0}").unwrap_err();
        let rest_api_error: RestApiError = error.into();
        assert_eq!(rest_api_error.to_string(), "failed to parse header value");
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
            "EOF while parsing an object at line 1 column 1"
        );
    }
}
