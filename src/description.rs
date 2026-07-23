impl_language_string_value!(
    Description,
    "descriptions",
    "descriptions_with_language_fallback",
    "description",
    "Description"
);

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{bearer_token, body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_descriptions_get_match_with_fallback() {
        let id = "Q42";
        let mock_path = format!(
            "/w/rest.php/wikibase/v1/entities/items/{id}/descriptions_with_language_fallback/foo"
        );
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Douglas Adams")))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let description = Description::get_with_fallback(&id, "foo", &api)
            .await
            .unwrap();
        assert_eq!(description.language(), "foo");
        assert_eq!(description.value(), "Douglas Adams");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_description_get() {
        let id = "Q42";
        let mock_description = "Foo bar baz";
        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/descriptions/en");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_description))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let description = Description::get(&id, "en", &api).await.unwrap();
        assert_eq!(description.language(), "en");
        assert_eq!(description.value(), mock_description);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_description_put() {
        let description = "Foo bar baz";
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/descriptions/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(json!({"description": description})))
            .and(method("PUT"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(description)))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let new_description = Description::new("en", description);
        let return_description = new_description.put(&id, &api).await.unwrap();
        assert_eq!(return_description.language(), "en");
        assert_eq!(return_description.value(), description);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_description_delete() {
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/descriptions/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Description deleted")))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let description = Description::new("en", "");
        let result = description.delete(&id, &api).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_from() {
        let ls = LanguageString::new("en", "Foo bar baz");
        let description = Description::from(ls);
        assert_eq!(description.language(), "en");
        assert_eq!(description.value(), "Foo bar baz");
    }

    #[test]
    fn test_into() {
        let description = Description::new("en", "Foo bar baz");
        let ls: LanguageString = description.into();
        assert_eq!(ls.language(), "en");
        assert_eq!(ls.value(), "Foo bar baz");
    }
}
