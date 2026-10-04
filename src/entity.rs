use crate::{EditMetadata, EntityId, FromJson, HttpMisc, RestApi, RestApiError, RevisionMatch};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum EntityType {
    Item,
    Property,
}

impl EntityType {
    pub const fn type_name(&self) -> &str {
        match self {
            EntityType::Item => "item",
            EntityType::Property => "property",
        }
    }

    pub const fn as_str(&self) -> &str {
        self.type_name()
    }

    pub const fn group_name(&self) -> &str {
        match self {
            EntityType::Item => "items",
            EntityType::Property => "properties",
        }
    }
}

/// A top-level Wikibase entity (`Item` or `Property`), with GET and POST (create) support.
pub trait Entity: Default + Serialize + HttpMisc + FromJson {
    /// The kind of entity this type represents.
    const ENTITY_TYPE: EntityType;

    fn id(&self) -> &EntityId;
    fn set_id(&mut self, id: EntityId);

    /// Fetches the entity.
    async fn get(id: &EntityId, api: &RestApi) -> Result<Self, RestApiError> {
        Self::get_match(id, api, RevisionMatch::default()).await
    }

    /// Fetches the entity, conditional on `rm`.
    async fn get_match(
        id: &EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        Self::get_match_fields(id, &[], api, rm).await
    }

    /// Fetches only the given top-level `fields` (e.g. `["labels"]`) of the entity.
    async fn get_fields(
        id: &EntityId,
        fields: &[&str],
        api: &RestApi,
    ) -> Result<Self, RestApiError> {
        Self::get_match_fields(id, fields, api, RevisionMatch::default()).await
    }

    /// Fetches only the given top-level `fields` of the entity, conditional on `rm`.
    /// An empty `fields` slice fetches the whole entity.
    async fn get_match_fields(
        id: &EntityId,
        fields: &[&str],
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let mut params = HashMap::new();
        if !fields.is_empty() {
            params.insert("_fields".to_string(), fields.join(","));
        }
        let mut request = api
            .wikibase_request_builder(&id.entity_path()?, params, reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        let (j, header_info) = Self::api_execute(api, request).await?;
        Self::from_json_header_info(&j, header_info)
    }

    /// Creates the entity via the API. The entity must not have an ID yet.
    /// Returns the newly created entity, as reported by the server.
    async fn post(&self, api: &RestApi) -> Result<Self, RestApiError> {
        self.post_meta(api, EditMetadata::default()).await
    }

    /// Creates the entity via the API, with edit metadata.
    /// The entity must not have an ID yet.
    async fn post_meta(&self, api: &RestApi, em: EditMetadata) -> Result<Self, RestApiError> {
        if self.id().is_some() {
            return Err(RestApiError::HasId);
        }
        let path = format!("/entities/{}", Self::ENTITY_TYPE.group_name());
        let mut body = json!({Self::ENTITY_TYPE.type_name(): self});
        Self::add_metadata_to_json(&mut body, &em);
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::POST)
            .await?
            .build()?;
        *request.body_mut() = Some(body.to_string().into());
        let response = api.execute(request).await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            // Older Wikibase versions don't offer entity creation via REST.
            return Err(RestApiError::NotImplementedInRestApi {
                method: reqwest::Method::POST,
                path,
            });
        }
        let (j, header_info) = Self::parse_response(response).await?;
        Self::from_json_header_info(&j, header_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{item::Item, RestApi};
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_entity_type() {
        assert_eq!(EntityType::Item.type_name(), "item");
        assert_eq!(EntityType::Property.type_name(), "property");
        assert_eq!(EntityType::Item.group_name(), "items");
        assert_eq!(EntityType::Property.group_name(), "properties");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_get_fields() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q42"))
            .and(query_param("_fields", "labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "Q42",
                "labels": {"en": "Douglas Adams"},
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let item = Item::get_fields(&EntityId::item("Q42"), &["labels"], &api)
            .await
            .unwrap();
        assert_eq!(item.labels().get_lang("en"), Some("Douglas Adams"));
        assert!(item.statements().is_empty());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_get_match() {
        // Exercises the no-`_fields` request path and `get` -> `get_match`.
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "Q42",
                "labels": {"en": "Douglas Adams"},
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let item = Item::get(&EntityId::item("Q42"), &api).await.unwrap();
        assert_eq!(item.id(), &EntityId::item("Q42"));
        assert_eq!(item.labels().get_lang("en"), Some("Douglas Adams"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_post_success() {
        // Exercises building the POST request body and parsing the created entity.
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/w/rest.php/wikibase/v1/entities/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "Q123",
                "labels": {"en": "test"},
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let item = Item::default();
        let created = item.post(&api).await.unwrap();
        assert_eq!(created.id(), &EntityId::item("Q123"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_post_meta_sends_metadata() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/w/rest.php/wikibase/v1/entities/items"))
            .and(body_partial_json(
                json!({"comment": "new item", "bot": true}),
            ))
            .respond_with(
                ResponseTemplate::new(201)
                    .insert_header("ETag", r#"W/"7""#)
                    .set_body_json(json!({"id": "Q7"})),
            )
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();
        let mut em = EditMetadata::default();
        em.set_comment(Some("new item".to_string()));
        em.set_bot(true);
        let created = Item::default().post_meta(&api, em).await.unwrap();
        assert_eq!(created.id(), &EntityId::item("Q7"));
        // The created entity carries the response's revision.
        assert_eq!(created.header_info().revision_id(), Some(7));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_post_non_404_error() {
        // A non-404 error response is surfaced as an `ApiError`, not `NotImplementedInRestApi`.
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/w/rest.php/wikibase/v1/entities/items"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "code": "bad-request",
                "message": "nope",
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let item = Item::default();
        let err = item.post(&api).await.err().unwrap();
        match err {
            RestApiError::ApiError { status, .. } => assert_eq!(status, 400),
            other => panic!("Wrong error type: {other:?}"),
        }
    }
}
