use crate::{
    entity::Entity, patch_entry::PatchEntry, EntityId, HttpMisc, Item, Patch, PatchApply, Property,
    RestApiError,
};
use derive_where::derive_where;
use std::marker::PhantomData;

/// A JSON Patch against a whole entity (`PATCH /entities/{group}/{id}`).
///
/// Paths are entity-level, e.g. `/labels/en` or `/statements/P31/0`. Usually created via
/// [`Item::patch`] / [`Property::patch`]; apply it with [`PatchApply::apply`], which
/// returns the patched entity.
#[derive_where(Debug, Clone, PartialEq, Default)]
pub struct EntityPatch<E> {
    patch: Vec<PatchEntry>,
    entity: PhantomData<E>,
}

/// A patch against a whole [`Item`].
pub type ItemPatch = EntityPatch<Item>;

/// A patch against a whole [`Property`].
pub type PropertyPatch = EntityPatch<Property>;

impl<E> EntityPatch<E> {
    /// Appends the entries of a sub-patch (e.g. a `LabelsPatch`), prefixing each path with
    /// `prefix` (e.g. `/labels`) so it addresses the right part of the entity document.
    pub(crate) fn with_part<P: Patch>(mut self, prefix: &str, mut part: P) -> Self {
        self.patch.extend(
            part.patch_mut()
                .drain(..)
                .map(|entry| entry.prefixed(prefix)),
        );
        self
    }
}

impl<E> Patch for EntityPatch<E> {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }
}

impl<E: Entity> HttpMisc for EntityPatch<E> {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        // Catch e.g. an `ItemPatch` applied to a property ID before it reaches the server.
        if id.kind() != Some(E::ENTITY_TYPE) {
            return Err(RestApiError::InvalidEntityId(id.to_string()));
        }
        id.entity_path()
    }
}

impl<E: Entity> PatchApply<E> for EntityPatch<E> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{labels_patch::LabelsPatch, RestApi};
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_get_rest_api_path() {
        let item_id = EntityId::new("Q123").unwrap();
        assert_eq!(
            ItemPatch::default().get_my_rest_api_path(&item_id).unwrap(),
            "/entities/items/Q123"
        );
        let property_id = EntityId::new("P123").unwrap();
        assert_eq!(
            PropertyPatch::default()
                .get_my_rest_api_path(&property_id)
                .unwrap(),
            "/entities/properties/P123"
        );
    }

    #[test]
    fn test_get_rest_api_path_wrong_kind() {
        let id = EntityId::property("P1");
        assert!(matches!(
            ItemPatch::default().get_my_rest_api_path(&id),
            Err(RestApiError::InvalidEntityId(_))
        ));
        assert!(PropertyPatch::default()
            .get_my_rest_api_path(&EntityId::None)
            .is_err());
    }

    #[test]
    fn test_with_part_prefixes_paths() {
        let mut labels = LabelsPatch::default();
        labels.replace("en", "Foo");
        let patch = ItemPatch::default().with_part("/labels", labels);
        assert_eq!(
            patch.patch(),
            &vec![PatchEntry::new("replace", "/labels/en", json!("Foo"))]
        );
    }

    #[test]
    fn test_is_empty() {
        let mut patch = ItemPatch::default();
        assert!(patch.is_empty());
        patch.add("/sitelinks/enwiki/title", json!("foo"));
        assert!(!patch.is_empty());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_apply_item() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: serde_json::Value = serde_json::from_str(&v).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q42"))
            .and(body_partial_json(json!({
                "patch": [{"op": "add", "path": "/labels/de", "value": "Test"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let mut patch = ItemPatch::default();
        patch.add("/labels/de", json!("Test"));
        let item = patch.apply(&EntityId::item("Q42"), &api).await.unwrap();
        assert_eq!(item.id(), &EntityId::item("Q42"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_apply_property() {
        let v = std::fs::read_to_string("test_data/P214.json").unwrap();
        let v: serde_json::Value = serde_json::from_str(&v).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/w/rest.php/wikibase/v1/entities/properties/P214"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let mut patch = PropertyPatch::default();
        patch.add("/labels/de", json!("Test"));
        let property = patch
            .apply(&EntityId::property("P214"), &api)
            .await
            .unwrap();
        assert_eq!(property.id(), &EntityId::property("P214"));
    }
}
