use crate::{EditMetadata, EntityId, HeaderInfo, HttpMisc, RestApi, RestApiError, RevisionMatch};
use async_trait::async_trait;
use reqwest::{Request, Response};
use serde::ser::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
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

    pub const fn group_name(&self) -> &str {
        match self {
            EntityType::Item => "items",
            EntityType::Property => "properties",
        }
    }
}

// impl Serialize for EntityType {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         let mut s = serializer.serialize_map(Some(self.ls.len()))?;
//         for (language, ls) in &self.ls {
//             s.serialize_entry(language, ls)?;
//         }
//         s.end()
//     }
// }

#[async_trait]
pub trait Entity: Default + Sized + Serialize + HttpMisc {
    fn id(&self) -> EntityId;
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
        let path = format!("/entities/{group}/{id}", group = id.group()?);
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::GET)
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

    #[test]
    fn test_entity_type() {
        assert_eq!(EntityType::Item.type_name(), "item");
        assert_eq!(EntityType::Property.type_name(), "property");
        assert_eq!(EntityType::Item.group_name(), "items");
        assert_eq!(EntityType::Property.group_name(), "properties");
    }
}
