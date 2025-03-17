use crate::{patch_entry::PatchEntry, EntityId, HttpMisc, Patch, RestApiError};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct LabelsPatch {
    patch: Vec<PatchEntry>,
}

impl LabelsPatch {
    pub fn from_json(j: &Value) -> Result<Vec<PatchEntry>, RestApiError> {
        j.as_array()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "LabelsPatch".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|x| serde_json::from_value(x.clone()).map_err(|e| e.into()))
            .collect::<Result<Vec<PatchEntry>, RestApiError>>()
    }

    // TODO add?

    /// Adds a command to replace the value of a language string.
    /// TODO Labels?
    pub fn replace<S1: Into<String>, S2: Into<String>>(&mut self, language: S1, value: S2) {
        <Self as Patch>::replace(self, format!("/{}", language.into()), value.into().into());
    }

    /// Adds a command to remove the value for the language.
    /// TODO Labels?
    pub fn remove<S: Into<String>>(&mut self, language: S) {
        <Self as Patch>::remove(self, format!("/{}", language.into()));
    }
}

impl Patch for LabelsPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }
}

impl HttpMisc for LabelsPatch {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/labels",
            group = id.group()?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn test_remove() {
        let mut patch = LabelsPatch::default();
        patch.remove("en");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("remove", "/en", Value::Null)]
        );
    }

    #[test]
    fn test_patch() {
        let mut patch = LabelsPatch::default();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_patch_fn() {
        let mut patch = LabelsPatch::default();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            *<LabelsPatch as Patch>::patch(&patch),
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_from_json() {
        let j = json!([
            {"op": "replace", "path": "/en", "value": "Foo Bar"},
            {"op": "remove", "path": "/de"}
        ]);
        let patch = LabelsPatch::from_json(&j).unwrap();
        assert_eq!(
            patch,
            vec![
                PatchEntry::new("replace", "/en", json!("Foo Bar")),
                PatchEntry::new("remove", "/de", Value::Null)
            ]
        );
    }

    #[test]
    fn test_get_rest_api_path_items() {
        let patch = LabelsPatch::default();
        let id = EntityId::new("Q12345").unwrap();
        assert_eq!(
            patch.get_my_rest_api_path(&id).unwrap(),
            "/entities/items/Q12345/labels"
        );
    }

    #[test]
    fn test_get_rest_api_path_properties() {
        let patch = LabelsPatch::default();
        let id = EntityId::new("P123").unwrap();
        assert_eq!(
            patch.get_my_rest_api_path(&id).unwrap(),
            "/entities/properties/P123/labels"
        );
    }
}
