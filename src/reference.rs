use rayon::prelude::*;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;
use crate::{property_value::{PropertyType, PropertyValue}, statement_value::StatementValue, RestApiError};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reference {
    parts: Vec<PropertyValue>,
    hash: String,
}

impl Reference {
    /// Creates a new Reference object from a JSON structure
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        let hash = j["hash"]
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField { field: "hash".into(), j: j.to_owned() })?
            .to_string();
        let parts = j["parts"]
            .as_array()
            .ok_or_else(|| RestApiError::MissingOrInvalidField { field: "parts".into(), j: j.to_owned() })?
            .par_iter()
            .map(|part| {
                let property = PropertyType::from_json(&part["property"])?;
                let value = StatementValue::from_json(&part["value"])?;
                Ok(PropertyValue::new(property, value))
            })
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
    
    pub fn parts_mut(&mut self) -> &mut Vec<PropertyValue> {
        &mut self.parts
    }
}

impl Serialize for Reference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Reference", 2)?;
        s.serialize_field("hash", &self.hash)?;
        s.serialize_field("parts", &self.parts)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parts() {
        let reference = Reference {
            parts: vec![PropertyValue::new(PropertyType::new("P123", None), StatementValue::new_string("test"))],
            hash: "hash".to_string(),
        };
        assert_eq!(reference.parts(), &[PropertyValue::new(PropertyType::new("P123", None), StatementValue::new_string("test"))]);
    }

    #[test]
    fn test_from_json_err() {
        let json = r#"{"hash":"hash","parts":12345}"#;
        assert!(Reference::from_json(&serde_json::from_str(json).unwrap()).is_err());
    }

    #[test]
    fn test_parts_mut() {
        let mut reference = Reference {
            parts: vec![PropertyValue::new(PropertyType::new("P123", None), StatementValue::new_string("test"))],
            hash: "hash".to_string(),
        };
        reference.parts_mut().push(PropertyValue::new(PropertyType::new("P456", None), StatementValue::new_string("test")));
        assert_eq!(reference.parts(), &[PropertyValue::new(PropertyType::new("P123", None), StatementValue::new_string("test")), PropertyValue::new(PropertyType::new("P456", None), StatementValue::new_string("test"))]);
    }

    #[test]
    fn test_hash() {
        let reference = Reference {
            parts: vec![PropertyValue::new(PropertyType::new("P123", None), StatementValue::new_string("test"))],
            hash: "hash".to_string(),
        };
        assert_eq!(reference.hash(), "hash");
    }


}
