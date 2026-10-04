use crate::RestApiError;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PatchEntry {
    op: String,
    path: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Value::is_null")]
    value: Value,
}

impl PatchEntry {
    /// Constructs a new `PatchEntry` object from an operation, a path, and a value.
    pub fn new<S1: Into<String>, S2: Into<String>>(op: S1, path: S2, value: Value) -> Self {
        Self {
            op: op.into(),
            path: path.into(),
            value,
        }
    }

    /// Returns the operation.
    pub fn op(&self) -> &str {
        &self.op
    }

    /// Returns the path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the value.
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Returns this entry with `prefix` prepended to its path, e.g. to lift a
    /// `/en` labels-patch entry to the entity-level path `/labels/en`.
    pub(crate) fn prefixed(mut self, prefix: &str) -> Self {
        self.path = format!("{prefix}{}", self.path);
        self
    }

    /// Parses a JSON Patch array (e.g. from `json_patch::diff`) into entries.
    /// `field` names the patch type for the error if `j` is not an array.
    pub(crate) fn list_from_json(j: &Value, field: &str) -> Result<Vec<Self>, RestApiError> {
        j.as_array()
            .ok_or_else(|| RestApiError::WrongType {
                field: field.into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|x| Self::deserialize(x).map_err(RestApiError::from))
            .collect()
    }

    /// Computes the JSON Patch entries that transform `from` into `to`.
    pub(crate) fn diff<T: Serialize>(
        from: &T,
        to: &T,
        field: &str,
    ) -> Result<Vec<Self>, RestApiError> {
        let patch = json_patch::diff(&json!(from), &json!(to));
        Self::list_from_json(&json!(patch), field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_patch_entry() {
        let pe = PatchEntry::new("replace", "/enwiki/title", json!("Foo Bar"));
        assert_eq!(pe.op, "replace");
        assert_eq!(pe.path, "/enwiki/title");
        assert_eq!(pe.value, json!("Foo Bar"));
    }

    #[test]
    fn test_prefixed() {
        let pe = PatchEntry::new("add", "/de", json!("x")).prefixed("/labels");
        assert_eq!(pe.path(), "/labels/de");
    }

    #[test]
    fn test_list_from_json() {
        let j = json!([{"op": "remove", "path": "/en"}]);
        let entries = PatchEntry::list_from_json(&j, "Test").unwrap();
        assert_eq!(entries, vec![PatchEntry::new("remove", "/en", Value::Null)]);
        assert!(matches!(
            PatchEntry::list_from_json(&json!({}), "Test"),
            Err(RestApiError::WrongType { .. })
        ));
        assert!(PatchEntry::list_from_json(&json!([{"op": 1}]), "Test").is_err());
    }

    #[test]
    fn test_diff() {
        let entries = PatchEntry::diff(&json!({"a": 1}), &json!({"a": 2}), "Test").unwrap();
        assert_eq!(entries, vec![PatchEntry::new("replace", "/a", json!(2))]);
    }

    #[test]
    fn test_patch_entry_default() {
        let pe = PatchEntry::default();
        assert_eq!(pe.op, "");
        assert_eq!(pe.path, "");
        assert_eq!(pe.value, Value::Null);
    }

    #[test]
    fn test_patch_entry_methods() {
        let pe = PatchEntry::new("replace", "/enwiki/title", json!("Foo Bar"));
        assert_eq!(pe.op(), "replace");
        assert_eq!(pe.path(), "/enwiki/title");
        assert_eq!(pe.value(), &json!("Foo Bar"));
    }
}
