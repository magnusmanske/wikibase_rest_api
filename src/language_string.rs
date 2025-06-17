use nutype::nutype;
use serde::{Deserialize, Serialize};

#[nutype(
    sanitize(trim, lowercase),
    validate(regex = "^[a-z]{2}[a-z0-9-]*$"),
    derive(Debug, Display, Clone, PartialEq)
)]
pub struct Language(String);

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

    /// Returns the value (test) of the language string.
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
}
