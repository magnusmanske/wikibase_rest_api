impl_language_string_patch!(DescriptionsPatch, "descriptions", "DescriptionsPatch");

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn test_remove() {
        let mut patch = DescriptionsPatch::default();
        patch.remove("en");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("remove", "/en", Value::Null)]
        );
    }

    #[test]
    fn test_patch() {
        let mut patch = DescriptionsPatch::default();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_descriptions() {
        let mut patch = DescriptionsPatch::default();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_patch_fn() {
        let mut patch = DescriptionsPatch::default();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            *<DescriptionsPatch as Patch>::patch(&patch),
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_from_json() {
        let j = json!([
            {"op": "replace", "path": "/en", "value": "Foo Bar"},
            {"op": "remove", "path": "/de"}
        ]);
        let patch = DescriptionsPatch::from_json(&j).unwrap();
        assert_eq!(
            patch,
            vec![
                PatchEntry::new("replace", "/en", json!("Foo Bar")),
                PatchEntry::new("remove", "/de", Value::Null)
            ]
        );
    }

    #[test]
    fn test_get_rest_api_path() {
        let patch = DescriptionsPatch::default();
        let id = EntityId::new("Q123").unwrap();
        assert_eq!(
            patch.get_my_rest_api_path(&id).unwrap(),
            "/entities/items/Q123/descriptions"
        );
    }
}
