use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;

use crate::{statement_value::StatementValue, DataType, RestApiError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PropertyType {
    id: String,
    datatype: Option<DataType>,
}

impl PropertyType {
    /// Creates a new `PropertyType` object from an ID and a `DataType`.
    pub fn new<S: Into<String>>(id: S, datatype: Option<DataType>) -> Self {
        Self {
            id: id.into(),
            datatype,
        }
    }

    /// Creates a new `PropertyType` object from a JSON object.
    /// # Errors
    /// Returns an error if the JSON object does not contain the required fields.
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        let datatype_text =
            j["data_type"]
                .as_str()
                .ok_or_else(|| RestApiError::MissingOrInvalidField {
                    field: "data_type".into(),
                    j: j.to_owned(),
                })?;
        let datatype = DataType::new(datatype_text).ok();
        Ok(Self {
            id: j["id"]
                .as_str()
                .ok_or_else(|| RestApiError::MissingOrInvalidField {
                    field: "id".into(),
                    j: j.to_owned(),
                })?
                .to_string(),
            datatype,
        })
    }

    /// Creates a new `PropertyType` object from an ID, with a default `DataType::WikibaseItem`.
    pub fn property<S: Into<String>>(id: S) -> Self {
        Self {
            id: id.into(),
            datatype: None,
        }
    }

    /// Returns the ID of the `PropertyType`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the `DataType` of the `PropertyType`.
    pub const fn datatype(&self) -> &Option<DataType> {
        &self.datatype
    }
}

impl Serialize for PropertyType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let num = 1 + if self.datatype.is_some() { 1 } else { 0 };
        let mut s = serializer.serialize_struct("PropertyType", num)?;
        s.serialize_field("id", &self.id)?;
        if let Some(datatype) = &self.datatype {
            s.serialize_field("data_type", datatype.as_str())?;
        }
        s.end()
    }
}

/// Implement the From trait for &str to `PropertyType`, for convenience assignments.
impl From<&str> for PropertyType {
    fn from(s: &str) -> Self {
        Self::property(s)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyValue {
    property: PropertyType,
    value: StatementValue,
}

impl PropertyValue {
    pub const fn new(property: PropertyType, value: StatementValue) -> Self {
        Self { property, value }
    }

    pub const fn property(&self) -> &PropertyType {
        &self.property
    }

    pub const fn value(&self) -> &StatementValue {
        &self.value
    }
}

impl Serialize for PropertyValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("PropertyValue", 2)?;
        s.serialize_field("property", &self.property)?;
        s.serialize_field("value", &self.value)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::statement_value_content::StatementValueContent;

    use super::*;

    #[test]
    fn test_property_type() {
        let j = serde_json::json!({
            "id": "P123",
            "data_type": "string",
        });
        let p = PropertyType::from_json(&j).unwrap();
        assert_eq!(p.id(), "P123");
        assert_eq!(p.datatype(), &Some(DataType::String));
    }

    #[test]
    fn test_property_value() {
        let j = serde_json::json!({
            "id": "P123",
            "data_type": "string",
        });
        let p = PropertyType::from_json(&j).unwrap();
        let v = StatementValueContent::String("Hello".to_string());
        let pv = PropertyValue::new(p, v.into());
        assert_eq!(pv.property().id(), "P123");
        assert_eq!(pv.property().datatype(), &Some(DataType::String));
        assert_eq!(
            pv.value(),
            &StatementValue::Value(StatementValueContent::String("Hello".to_string()))
        );
    }

    #[test]
    fn test_property_type_serialize() {
        let j = serde_json::json!({
            "id": "P123",
            "data_type": "string",
        });
        let p = PropertyType::from_json(&j).unwrap();
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, r#"{"id":"P123","data_type":"string"}"#);
    }

    #[test]
    fn test_property_type_serialize_faulty_data_type() {
        let j = serde_json::json!({
            "id": "P123",
            "data_type": 567,
        });
        let pt = PropertyType::from_json(&j);
        assert!(pt.is_err());
    }

    #[test]
    fn test_property_type_serialize_faulty_id() {
        let j = serde_json::json!({
            "id": 123,
            "data_type": "string",
        });
        let pt = PropertyType::from_json(&j);
        assert!(pt.is_err());
    }

    #[test]
    fn test_property_type_eq() {
        let j = serde_json::json!({
            "id": "P123",
            "data_type": "string",
        });
        let pt1 = PropertyType::from_json(&j).unwrap();
        let pt2 = PropertyType::new("P123", Some(DataType::String));
        assert_eq!(pt1, pt2);
    }

    #[test]
    fn test_property_type_ord() {
        let pt1 = PropertyType::new("P122", Some(DataType::String));
        let pt2 = PropertyType::new("P123", Some(DataType::String));
        let pt3 = PropertyType::new("P123", Some(DataType::ExternalId));
        assert!(pt1 < pt2); // Same data type, value differs
        assert!(pt2 < pt3); // Same value, data type differs
    }
}
