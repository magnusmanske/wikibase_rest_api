use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchEntry {
    op: String,
    path: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Value::is_null")]
    value: Value,
}

impl PatchEntry {
    /// Constructs a new `PatchEntry` object from an operation, a path, and a value.
    pub fn new<S1: Into<String>,S2: Into<String>>(op: S1, path: S2, value: Value) -> Self {
        Self {
            op: op.into(),
            path: path.into(),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use super::*;

    #[test]
    fn test_patch_entry() {
        let pe = PatchEntry::new("replace", "/enwiki/title", json!("Foo Bar"));
        assert_eq!(pe.op, "replace");
        assert_eq!(pe.path, "/enwiki/title");
        assert_eq!(pe.value, json!("Foo Bar"));
    }
}