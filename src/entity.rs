use crate::{EditMetadata, EntityId, HeaderInfo, HttpMisc, RestApi, RestApiError, RevisionMatch};
use reqwest::{Request, Response};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
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

pub trait Entity: Default + Sized + Serialize + HttpMisc {
    fn id(&self) -> EntityId;
    fn set_id(&mut self, id: EntityId);
    fn from_json_header_info(j: Value, header_info: HeaderInfo) -> Result<Self, RestApiError>;

    fn from_json(j: Value) -> Result<Self, RestApiError> {
        Self::from_json_header_info(j, HeaderInfo::default())
    }

    async fn get(id: EntityId, api: &RestApi) -> Result<Self, RestApiError> {
        Self::get_match(id, api, RevisionMatch::default()).await
    }

    async fn generate_get_match_request(
        id: EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Request, RestApiError> {
        Self::generate_get_match_request_fields(id, api, rm, &[]).await
    }

    async fn generate_get_match_request_fields(
        id: EntityId,
        api: &RestApi,
        rm: RevisionMatch,
        fields: &[&str],
    ) -> Result<Request, RestApiError> {
        let path = format!("/entities/{group}/{id}", group = id.group()?);
        let mut params = HashMap::new();
        if !fields.is_empty() {
            params.insert("_fields".to_string(), fields.join(","));
        }
        let mut request = api
            .wikibase_request_builder(&path, params, reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        Ok(request)
    }

    async fn get_match(
        id: EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let request = Self::generate_get_match_request(id, api, rm).await?;
        let response = api.execute(request).await?;
        if !response.status().is_success() {
            return Err(RestApiError::from_response(response).await);
        }
        let hi = HeaderInfo::from_header(response.headers());
        let j: Value = response.error_for_status()?.json().await?;
        let ret = Self::from_json_header_info(j, hi)?;
        Ok(ret)
    }

    async fn get_fields(
        id: EntityId,
        fields: &[&str],
        api: &RestApi,
    ) -> Result<Self, RestApiError> {
        Self::get_match_fields(id, api, RevisionMatch::default(), fields).await
    }

    async fn get_match_fields(
        id: EntityId,
        api: &RestApi,
        rm: RevisionMatch,
        fields: &[&str],
    ) -> Result<Self, RestApiError> {
        let request = Self::generate_get_match_request_fields(id, api, rm, fields).await?;
        let response = api.execute(request).await?;
        if !response.status().is_success() {
            return Err(RestApiError::from_response(response).await);
        }
        let hi = HeaderInfo::from_header(response.headers());
        let j: Value = response.error_for_status()?.json().await?;
        Self::from_json_header_info(j, hi)
    }

    async fn post(&self, api: &RestApi) -> Result<Self, RestApiError>;

    async fn post_with_type(
        &self,
        entity_type: EntityType,
        api: &RestApi,
    ) -> Result<Self, RestApiError> {
        self.post_with_type_and_metadata(entity_type, api, EditMetadata::default())
            .await
    }

    async fn build_post_with_type_and_metadata_request(
        &self,
        entity_type: EntityType,
        path: &str,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<reqwest::Request, RestApiError> {
        let mut request = api
            .wikibase_request_builder(path, HashMap::new(), reqwest::Method::POST)
            .await?
            .build()?;
        let mut j: Value = json!({entity_type.type_name(): self});
        Self::add_metadata_to_json(&mut j, &em);
        *request.body_mut() = Some(format!("{j}").into());
        Ok(request)
    }

    async fn check_post_with_type_and_metadata_response(
        path: &str,
        response: Response,
    ) -> Result<Response, RestApiError> {
        if response.status().is_success() {
            return Ok(response);
        }
        let status_code = response.status();
        if status_code == 404 {
            return Err(RestApiError::NotImplementedInRestApi {
                method: reqwest::Method::POST,
                path: path.to_string(),
            });
        }
        Err(RestApiError::from_response(response).await)
    }

    async fn post_with_type_and_metadata(
        &self,
        entity_type: EntityType,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError> {
        if self.id().is_some() {
            return Err(RestApiError::HasId);
        }
        let path = format!("/entities/{group}", group = entity_type.group_name());
        let request = self
            .build_post_with_type_and_metadata_request(entity_type, &path, api, em)
            .await?;
        let response = api.execute(request).await?;
        let response = Self::check_post_with_type_and_metadata_response(&path, response).await?;

        let j: Value = response.json().await?;
        // TODO return entire entity? Check if it's the same as this one?
        let ret = Self::from_json(j)?;
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{item::Item, RestApi};
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
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
            .build();

        let item = Item::get_fields(EntityId::item("Q42"), &["labels"], &api)
            .await
            .unwrap();
        assert_eq!(item.labels().get_lang("en"), Some("Douglas Adams"));
        assert!(item.statements().is_empty());
    }
}
