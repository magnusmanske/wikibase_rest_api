impl_language_string_patch!(LabelsPatch, crate::labels::Labels, "labels", "LabelsPatch");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{labels::Labels, language_strings::LanguageStrings, LanguageString, RestApi};
    use serde_json::{json, Value};
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_add() {
        let mut patch = LabelsPatch::default();
        patch.add("de", "Foo");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("add", "/de", json!("Foo"))]
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_apply_returns_labels() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q42/labels"))
            .and(body_partial_json(json!({
                "patch": [{"op": "add", "path": "/es", "value": "Douglas Adams"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"es": "Douglas Adams"})))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();
        let before = Labels::default();
        let mut after = before.clone();
        after.insert(LanguageString::new("es", "Douglas Adams"));
        // No type annotation needed: a LabelsPatch can only produce Labels.
        let updated = after
            .patch(&before)
            .unwrap()
            .apply(&EntityId::item("Q42"), &api)
            .await
            .unwrap();
        assert_eq!(updated.get_lang("es"), Some("Douglas Adams"));
    }

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
            patch.patch,
            vec![
                PatchEntry::new("replace", "/en", json!("Foo Bar")),
                PatchEntry::new("remove", "/de", Value::Null)
            ]
        );
    }

    #[test]
    fn test_from_json_not_array() {
        // A non-array patch source must surface as WrongType.
        let err = LabelsPatch::from_json(&json!(123)).unwrap_err();
        match err {
            RestApiError::WrongType { field, .. } => assert_eq!(field, "LabelsPatch"),
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
