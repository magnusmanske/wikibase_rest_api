use serde::Serialize;
use serde_json::Value;

use crate::{statement_value::StatementValue, DataType, RestApiError};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize)]
pub struct PropertyType {
    id: String,
    #[serde(rename = "data_type", skip_serializing_if = "Option::is_none")]
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
    ///
    /// `data_type` may be `null` (or absent), e.g. for a statement whose property has
    /// since been deleted; that yields a `PropertyType` without a data type.
    /// # Errors
    /// Returns an error if `id` is missing, or `data_type` is neither a string nor null.
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        let datatype = match &j["data_type"] {
            Value::Null => None,
            Value::String(s) => Some(DataType::new(s.as_str())),
            _ => {
                return Err(RestApiError::MissingOrInvalidField {
                    field: "data_type".into(),
                    j: j.to_owned(),
                })
            }
        };
        let id = j["id"]
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "id".into(),
                j: j.to_owned(),
            })?;
        Ok(Self::new(id, datatype))
    }

    /// Creates a new `PropertyType` object from an ID, without a `DataType`.
    pub fn property<S: Into<String>>(id: S) -> Self {
        Self::new(id, None)
    }

    /// Returns the ID of the `PropertyType`.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the `DataType` of the `PropertyType`.
    pub const fn datatype(&self) -> Option<&DataType> {
        self.datatype.as_ref()
    }
}

/// Implement the From trait for &str to `PropertyType`, for convenience assignments.
impl From<&str> for PropertyType {
    fn from(s: &str) -> Self {
        Self::property(s)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PropertyValue {
    property: PropertyType,
    value: StatementValue,
}

impl PropertyValue {
    pub const fn new(property: PropertyType, value: StatementValue) -> Self {
        Self { property, value }
    }

    /// Creates a new `PropertyValue` (a qualifier or reference part) from a JSON object.
    /// # Errors
    /// Returns an error if the property or value is missing or invalid.
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        Ok(Self::new(
            PropertyType::from_json(&j["property"])?,
            StatementValue::from_json(&j["value"])?,
        ))
    }

    pub const fn property(&self) -> &PropertyType {
        &self.property
    }

    pub const fn value(&self) -> &StatementValue {
        &self.value
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
        assert_eq!(p.datatype(), Some(&DataType::String));
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
        assert_eq!(pv.property().datatype(), Some(&DataType::String));
        assert_eq!(
            pv.value(),
            &StatementValue::Value(StatementValueContent::String("Hello".to_string()))
        );
    }

    #[test]
    fn test_property_value_serialize() {
        let pt = PropertyType::new("P123", Some(DataType::String));
        let value = StatementValueContent::String("Hello".to_string());
        let pv = PropertyValue::new(pt, value.into());
        let j = serde_json::to_value(&pv).unwrap();
        assert_eq!(j["property"]["id"], "P123");
        assert_eq!(j["property"]["data_type"], "string");
        assert!(j["value"].is_object());
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
    fn test_property_type_null_data_type() {
        // A statement on a since-deleted property has `"data_type": null`.
        let j = serde_json::json!({"id": "P123", "data_type": null});
        let pt = PropertyType::from_json(&j).unwrap();
        assert_eq!(pt.datatype(), None);
        assert_eq!(serde_json::to_string(&pt).unwrap(), r#"{"id":"P123"}"#);
    }

    #[test]
    fn test_property_type_unknown_data_type_kept() {
        let j = serde_json::json!({"id": "P123", "data_type": "edtf"});
        let pt = PropertyType::from_json(&j).unwrap();
        assert_eq!(pt.datatype(), Some(&DataType::Other("edtf".into())));
        assert_eq!(serde_json::to_value(&pt).unwrap(), j);
    }

    #[test]
    fn test_property_value_from_json() {
        let j = serde_json::json!({
            "property": {"id": "P1", "data_type": "string"},
            "value": {"type": "novalue"},
        });
        let pv = PropertyValue::from_json(&j).unwrap();
        assert_eq!(pv.property().id(), "P1");
        assert_eq!(pv.value(), &StatementValue::NoValue);
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
