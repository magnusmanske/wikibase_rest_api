use crate::{property_value::PropertyValue, RestApiError};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Reference {
    parts: Vec<PropertyValue>,
    hash: String,
}

impl Reference {
    /// Creates a new Reference object from a JSON structure
    /// # Errors
    /// Returns an error if the JSON structure is missing a required field or if a field is invalid
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        let hash = j["hash"]
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "hash".into(),
                j: j.to_owned(),
            })?
            .to_string();
        let parts = j["parts"]
            .as_array()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "parts".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(PropertyValue::from_json)
            .collect::<Result<Vec<PropertyValue>, RestApiError>>()?;
        Ok(Reference { parts, hash })
    }

    /// Returns the parts of the reference
    pub fn parts(&self) -> &[PropertyValue] {
        &self.parts
    }

    /// Returns the hash of the reference
    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub const fn parts_mut(&mut self) -> &mut Vec<PropertyValue> {
        &mut self.parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{property_value::PropertyType, statement_value::StatementValue};

    #[test]
    fn test_parts() {
        let reference = Reference {
            parts: vec![PropertyValue::new(
                PropertyType::new("P123", None),
                StatementValue::new_string("test"),
            )],
            hash: "hash".to_string(),
        };
        assert_eq!(
            reference.parts(),
            &[PropertyValue::new(
                PropertyType::new("P123", None),
                StatementValue::new_string("test")
            )]
        );
    }

    #[test]
    fn test_from_json_err() {
        let json = r#"{"hash":"hash","parts":12345}"#;
        assert!(Reference::from_json(&serde_json::from_str(json).unwrap()).is_err());
    }

    #[test]
    fn test_parts_mut() {
        let mut reference = Reference {
            parts: vec![PropertyValue::new(
                PropertyType::new("P123", None),
                StatementValue::new_string("test"),
            )],
            hash: "hash".to_string(),
        };
        reference.parts_mut().push(PropertyValue::new(
            PropertyType::new("P456", None),
            StatementValue::new_string("test"),
        ));
        assert_eq!(
            reference.parts(),
            &[
                PropertyValue::new(
                    PropertyType::new("P123", None),
                    StatementValue::new_string("test")
                ),
                PropertyValue::new(
                    PropertyType::new("P456", None),
                    StatementValue::new_string("test")
                )
            ]
        );
    }

    #[test]
    fn test_serialize() {
        // Covers the custom Serialize impl (hash + parts fields).
        let reference = Reference {
            parts: vec![PropertyValue::new(
                PropertyType::new("P123", None),
                StatementValue::new_string("test"),
            )],
            hash: "the-hash".to_string(),
        };
        let j = serde_json::to_value(&reference).unwrap();
        assert_eq!(j["hash"], "the-hash");
        assert!(j["parts"].is_array());
        assert_eq!(j["parts"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_hash() {
        let reference = Reference {
            parts: vec![PropertyValue::new(
                PropertyType::new("P123", None),
                StatementValue::new_string("test"),
            )],
            hash: "hash".to_string(),
        };
        assert_eq!(reference.hash(), "hash");
    }
}
