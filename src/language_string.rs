use crate::RestApiError;
use nutype::nutype;
use serde::{Deserialize, Serialize};

#[nutype(
    sanitize(trim, lowercase),
    validate(regex = "^[a-z]{2}[a-z0-9-]*$"),
    derive(Debug, Display, Clone, PartialEq)
)]
pub struct Language(String);

impl Language {
    /// Validates a raw language code for use in a REST API path, returning the
    /// sanitized (trimmed, lower-cased) form. Rejects anything that is not a
    /// well-formed language code — including strings that could inject extra
    /// URL path segments — as [`RestApiError::InvalidLanguageCode`].
    pub(crate) fn validated(input: &str) -> Result<String, RestApiError> {
        Self::try_new(input)
            .map(Self::into_inner)
            .map_err(|_| RestApiError::InvalidLanguageCode(input.to_string()))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LanguageString {
    language: String,
    value: String,
}

impl LanguageString {
    /// Constructs a new `LanguageString` object from a language code and a string.
    pub fn new<S1: Into<String>, S2: Into<String>>(language: S1, value: S2) -> LanguageString {
        LanguageString {
            language: language.into(),
            value: value.into(),
        }
    }

    /// Returns the language code of the language string.
    pub const fn language(&self) -> &String {
        &self.language
    }

    /// Returns the value (text) of the language string.
    pub const fn value(&self) -> &String {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_string() {
        let s = LanguageString::new("en", "Hello");
        assert_eq!(s.language(), "en");
        assert_eq!(s.value(), "Hello");
    }

    #[test]
    fn test_language_string_serialize() {
        let s = LanguageString::new("en", "Hello");
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, r#"{"language":"en","value":"Hello"}"#);
    }

    #[test]
    fn test_language_validated_sanitizes() {
        // A well-formed code is trimmed and lower-cased.
        assert_eq!(Language::validated(" EN ").unwrap(), "en");
        assert_eq!(Language::validated("de-CH").unwrap(), "de-ch");
    }

    #[test]
    fn test_language_validated_rejects() {
        // Empty, path-injecting, and otherwise malformed codes are rejected.
        for bad in ["", "e", "en/labels", "../en", "en wiki"] {
            match Language::validated(bad).unwrap_err() {
                RestApiError::InvalidLanguageCode(input) => assert_eq!(input, bad),
                e => panic!("Wrong error type for {bad:?}: {e:?}"),
            }
        }
    }
}
