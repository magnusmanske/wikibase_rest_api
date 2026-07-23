impl_language_string_patch!(LabelsPatch, "labels", "LabelsPatch");

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
    fn test_from_json_not_array() {
        // A non-array patch source must surface as MissingOrInvalidField.
        let err = LabelsPatch::from_json(&json!(123)).unwrap_err();
        match err {
            RestApiError::MissingOrInvalidField { field, .. } => assert_eq!(field, "LabelsPatch"),
            e => panic!("Wrong error type: {e:?}"),
        }
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
