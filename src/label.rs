impl_language_string_value!(Label, "labels", "labels_with_language_fallback", "label", "Label");

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{bearer_token, body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_labels_get_match_with_fallback() {
        let id = "Q42";
        let mock_path = format!(
            "/w/rest.php/wikibase/v1/entities/items/{id}/labels_with_language_fallback/foo"
        );
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Douglas Adams")))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let id = EntityId::item(id);
        let label = Label::get_with_fallback(&id, "foo", &api).await.unwrap();
        assert_eq!(label.language(), "foo");
        assert_eq!(label.value(), "Douglas Adams");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_label_get() {
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/labels/en");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Douglas Adams")))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let id = EntityId::item(id);
        let label = Label::get(&id, "en", &api).await.unwrap();
        assert_eq!(label.language(), "en");
        assert_eq!(label.value(), "Douglas Adams");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_label_put() {
        let label = "Foo bar";
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/labels/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(json!({"label": label})))
            .and(method("PUT"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(label)))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build();

        let id = EntityId::item(id);
        let new_label = Label::new("en", label);
        let return_label = new_label.put(&id, &mut api).await.unwrap();
        assert_eq!(return_label.language(), "en");
        assert_eq!(return_label.value(), label);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_label_delete() {
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/labels/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Label deleted")))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build();

        let id = EntityId::item(id);
        let label = Label::new("en", "");
        let result = label.delete(&id, &mut api).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_into_language_string() {
        let label = Label::new("en", "Foo bar");
        let ls: LanguageString = label.into();
        assert_eq!(ls.language(), "en");
        assert_eq!(ls.value(), "Foo bar");
    }

    #[test]
    fn test_from_language_string() {
        let ls = LanguageString::new("en", "Foo bar");
        let label = Label::from(ls);
        assert_eq!(label.language(), "en");
        assert_eq!(label.value(), "Foo bar");
    }
}
