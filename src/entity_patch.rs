/// NOTE: THIS IS INCOMPLETE AND UNTESTED!
use crate::{
    entity::{Entity, EntityType},
    patch_entry::PatchEntry,
    EditMetadata, EntityId, HttpMisc, Item, Property, RestApi, RestApiError,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EntityPatch {
    patch: Vec<PatchEntry>,
    mode: EntityType,
}

impl EntityPatch {
    pub const fn item() -> Self {
        Self {
            patch: vec![],
            mode: EntityType::Item,
        }
    }

    pub const fn property() -> Self {
        Self {
            patch: vec![],
            mode: EntityType::Property,
        }
    }

    /// Returns the patch entries
    pub const fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    /// Returns the mutable patch entries
    pub const fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }

    /// checks if the patch list is empty
    pub const fn is_empty(&self) -> bool {
        self.patch().is_empty()
    }

    /// Applies the entire patch against the API
    pub async fn apply_item(&self, id: &EntityId, api: &RestApi) -> Result<Item, RestApiError> {
        self.apply_match_item(id, api, EditMetadata::default())
            .await
    }

    pub async fn apply_property(
        &self,
        id: &EntityId,
        api: &RestApi,
    ) -> Result<Property, RestApiError> {
        self.apply_match_property(id, api, EditMetadata::default())
            .await
    }

    /// Applies the entire patch against the API
    pub async fn apply_match_item(
        &self,
        id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Item, RestApiError> {
        let j0 = json!({"patch": self.patch()});
        let request = self
            .generate_json_request(id, reqwest::Method::PATCH, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let (j1, header_info) = self.filter_response_error(response).await?;
        Item::from_json_header_info(j1, header_info)
    }

    /// Applies the entire patch against the API, conditional on metadata
    pub async fn apply_match_property(
        // TODO
        &self,
        id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Property, RestApiError> {
        let j0 = json!({"patch": self.patch()});
        let request = self
            .generate_json_request(id, reqwest::Method::PATCH, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let (j1, header_info) = self.filter_response_error(response).await?;
        Property::from_json_header_info(j1, header_info)
    }
}

impl HttpMisc for EntityPatch {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/{mode}",
            group = id.group()?,
            mode = self.mode.as_str()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mode() {
        assert_eq!(EntityType::Item.as_str(), "item");
        assert_eq!(EntityType::Property.as_str(), "property");
    }

    #[test]
    fn test_get_rest_api_path() {
        let patch = EntityPatch::item();
        let id = EntityId::new("Q123").unwrap();
        assert_eq!(
            patch.get_my_rest_api_path(&id).unwrap(),
            "/entities/items/Q123/item"
        );
    }

    #[test]
    fn test_item() {
        let patch = EntityPatch::item();
        assert!(patch.is_empty());
        assert_eq!(patch.mode, EntityType::Item);
    }

    #[test]
    fn test_property() {
        let patch = EntityPatch::property();
        assert!(patch.is_empty());
        assert_eq!(patch.mode, EntityType::Property);
    }

    #[test]
    fn test_patch() {
        let mut patch = EntityPatch::item();
        assert!(patch.is_empty());
        patch
            .patch_mut()
            .push(PatchEntry::new("add", "/enwiki/title", json!("foo")));
        assert_eq!(patch.patch().len(), 1);
    }

    #[test]
    fn test_is_empty() {
        let mut patch = EntityPatch::item();
        assert!(patch.is_empty());
        patch
            .patch_mut()
            .push(PatchEntry::new("add", "/enwiki/title", json!("foo")));
        assert!(!patch.is_empty());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_apply_item() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: serde_json::Value = serde_json::from_str(&v).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q42/item"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let mut patch = EntityPatch::item();
        patch
            .patch_mut()
            .push(PatchEntry::new("add", "/labels/de", json!("Test")));

        // apply_item -> apply_match_item (default EditMetadata).
        let item = patch
            .apply_item(&EntityId::item("Q42"), &api)
            .await
            .unwrap();
        assert_eq!(item.id(), &EntityId::item("Q42"));

        // apply_match_item directly, with explicit metadata.
        let item2 = patch
            .apply_match_item(&EntityId::item("Q42"), &api, EditMetadata::default())
            .await
            .unwrap();
        assert_eq!(item2.id(), &EntityId::item("Q42"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_apply_property() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let v = std::fs::read_to_string("test_data/P214.json").unwrap();
        let v: serde_json::Value = serde_json::from_str(&v).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(
                "/w/rest.php/wikibase/v1/entities/properties/P214/property",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let mut patch = EntityPatch::property();
        patch
            .patch_mut()
            .push(PatchEntry::new("add", "/labels/de", json!("Test")));

        // apply_property -> apply_match_property (default EditMetadata).
        let property = patch
            .apply_property(&EntityId::property("P214"), &api)
            .await
            .unwrap();
        assert_eq!(property.id(), &EntityId::property("P214"));

        // apply_match_property directly, with explicit metadata.
        let property2 = patch
            .apply_match_property(&EntityId::property("P214"), &api, EditMetadata::default())
            .await
            .unwrap();
        assert_eq!(property2.id(), &EntityId::property("P214"));
    }
}
