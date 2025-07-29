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
    /* DO WE NEED THIS?
       /// Generates a patch from JSON, presumably from `json_patch`
       pub fn item_from_json(j: &Value) -> Result<Self, RestApiError> {
           Ok(Self {
               patch: Self::patch_from_json(j)?,
               mode: Mode::Item,
           })
       }

       /// Generates a patch from JSON, presumably from `json_patch`
       pub fn property_from_json(j: &Value) -> Result<Self, RestApiError> {
           Ok(Self {
               patch: Self::patch_from_json(j)?,
               mode: Mode::Property,
           })
       }

       fn patch_from_json(j: &Value) -> Result<Vec<PatchEntry>, RestApiError> {
           j.as_array()
               .ok_or_else(|| RestApiError::MissingOrInvalidField {
                   field: "EntityPatch".into(),
                   j: j.to_owned(),
               })?
               .iter()
               .map(|x| serde_json::from_value(x.clone()).map_err(|e| e.into()))
               .collect::<Result<Vec<PatchEntry>, RestApiError>>()
       }
    */
    /// Returns the patch entries
    pub const fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    /// Returns the mutable patch entries
    pub const fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }

    // /// `path` is a JSON patch path, eg "/enwiki/title"
    // pub fn add<S: Into<String>>(&mut self, path: S, value: Value) {
    //     self.patch_mut()
    //         .push(PatchEntry::new("add", path.into(), value));
    // }

    // /// `path` is a JSON patch path, eg "/enwiki/title"
    // pub fn replace<S: Into<String>>(&mut self, path: S, value: Value) {
    //     self.patch_mut()
    //         .push(PatchEntry::new("replace", path.into(), value));
    // }

    // /// `path` is a JSON patch path, eg "/enwiki/title"
    // pub fn remove<S: Into<String>>(&mut self, path: S) {
    //     self.patch_mut()
    //         .push(PatchEntry::new("remove", path.into(), Value::Null));
    // }

    /// checks if the patch list is empty
    pub const fn is_empty(&self) -> bool {
        self.patch().is_empty()
    }

    /// Applies the entire patch against the API
    pub async fn apply_item(&self, id: &EntityId, api: &mut RestApi) -> Result<Item, RestApiError> {
        self.apply_match_item(id, api, EditMetadata::default())
            .await
    }

    pub async fn apply_property(
        &self,
        id: &EntityId,
        api: &mut RestApi,
    ) -> Result<Property, RestApiError> {
        self.apply_match_property(id, api, EditMetadata::default())
            .await
    }

    /// Applies the entire patch against the API
    pub async fn apply_match_item(
        &self,
        id: &EntityId,
        api: &mut RestApi,
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
        api: &mut RestApi,
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
}
