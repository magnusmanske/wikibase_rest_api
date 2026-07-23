impl_language_string_collection!(
    Descriptions,
    "descriptions",
    "Descriptions",
    descriptions_from_json
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_language_strings_single() {
        let j = json!({
            "en": "Hello",
            "de": "Hallo",
        });
        let ls = Descriptions::from_json(&j).unwrap();
        assert_eq!(ls.get_lang("en"), Some("Hello"));
        assert_eq!(ls.get_lang("de"), Some("Hallo"));
        assert_eq!(ls.get_lang("fr"), None);
    }

    #[test]
    fn test_language_strings_insert() {
        let mut ls = Descriptions::default();
        ls.insert(LanguageString::new("en", "Hello"));
        ls.insert(LanguageString::new("de", "Hallo"));
        ls.insert(LanguageString::new("en", "Hi"));
        assert_eq!(ls.get_lang("en"), Some("Hi"));
        assert_eq!(ls.get_lang("de"), Some("Hallo"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_language_strings_single_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();

        let mock_path = "/w/rest.php/wikibase/v1/entities/items/Q42/descriptions";
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["descriptions"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let id = EntityId::new("Q42").unwrap();
        let ls = Descriptions::get(&id, &api).await.unwrap();
        assert_eq!(ls.get_lang("en-gb"), Some("English writer and humourist"));
    }

    #[test]
    fn test_patch_descriptions() {
        let mut l1 = Descriptions::default();
        l1.insert(LanguageString::new("en", "Foo"));
        l1.insert(LanguageString::new("de", "Bar"));
        let mut l2 = l1.clone();
        l2.insert(LanguageString::new("en", "Baz"));

        let patch = l2.patch(&l1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"mode":"Descriptions","patch":[{"op":"replace","path":"/en","value":"Baz"}]})
        );
    }

    #[test]
    fn test_get_rest_api_path() {
        let l = Descriptions::default();
        let id = EntityId::new("Q42").unwrap();
        assert_eq!(
            l.get_my_rest_api_path(&id).unwrap(),
            "/entities/items/Q42/descriptions"
        );
    }

    #[test]
    fn test_header_info_single() {
        let l = Descriptions::default();
        assert_eq!(l.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_serialize() {
        let mut l = Descriptions::default();
        l.insert(LanguageString::new("en", "Foo"));
        l.insert(LanguageString::new("de", "Bar"));
        let s = serde_json::to_string(&l).unwrap();
        assert!(s.contains(r#""en":"Foo""#));
        assert!(s.contains(r#""de":"Bar""#));
    }

    #[test]
    fn test_is_empty() {
        let mut l = Descriptions::default();
        assert!(l.is_empty());
        l.insert(LanguageString::new("en", "Foo"));
        assert!(!l.is_empty());
    }
}
