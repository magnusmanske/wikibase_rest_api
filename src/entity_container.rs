use crate::{entity::Entity, EntityId, Item, Property, RestApi, RestApiError};
use futures::prelude::*;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

const MAX_CONCURRENT_LOAD_DEFAULT: usize = 10;

#[derive(Debug, Clone)]
pub struct EntityContainer {
    api: Arc<RestApi>,
    items: Arc<RwLock<HashMap<String, Item>>>,
    properties: Arc<RwLock<HashMap<String, Property>>>,
    max_concurrent_load: usize,
}

impl EntityContainer {
    /// Returns a new `EntityContainerBuilder` to configure a new `EntityContainer`.
    pub fn builder() -> EntityContainerBuilder {
        EntityContainerBuilder::default()
    }

    /// Loads the entities with the given `EntityId`s into the container.
    pub async fn load(&self, entity_ids: &[EntityId]) -> Result<(), RestApiError> {
        let mut items = self.items.write().await;
        let item_ids = Self::get_items_to_load(&items, entity_ids);
        self.load_items(&mut items, &item_ids).await?;
        drop(items);

        let mut properties = self.properties.write().await;
        let property_ids = Self::get_properties_to_load(&properties, entity_ids);
        self.load_properties(&mut properties, &property_ids).await?;
        drop(properties);

        Ok(())
    }

    fn get_items_to_load(items: &HashMap<String, Item>, entity_ids: &[EntityId]) -> Vec<String> {
        entity_ids
            .iter()
            .filter_map(|id| match id {
                EntityId::Item(id) => Some(id.to_owned()),
                _ => None,
            })
            .filter(|id| !items.contains_key(id))
            .collect()
    }

    async fn load_items(
        &self,
        items: &mut HashMap<String, Item>,
        item_ids: &[String],
    ) -> Result<(), RestApiError> {
        if item_ids.is_empty() {
            return Ok(());
        }
        let futures = item_ids
            .iter()
            .map(|id| Item::get(EntityId::item(id), &self.api))
            .collect::<Vec<_>>();
        let stream = futures::stream::iter(futures).buffer_unordered(self.max_concurrent_load);
        let results = stream.collect::<Vec<_>>().await;
        let results = results
            .into_iter()
            .collect::<Vec<Result<Item, RestApiError>>>();
        for item in results.into_iter().flatten() {
            let id = item.id().id()?.to_owned();
            items.insert(id, item);
        }
        Ok(())
    }

    fn get_properties_to_load(
        properties: &HashMap<String, Property>,
        entity_ids: &[EntityId],
    ) -> Vec<String> {
        entity_ids
            .iter()
            .filter_map(|id| match id {
                EntityId::Property(id) => Some(id.to_owned()),
                _ => None,
            })
            .filter(|id| !properties.contains_key(id))
            .collect()
    }

    async fn load_properties(
        &self,
        properties: &mut HashMap<String, Property>,
        property_ids: &[String],
    ) -> Result<(), RestApiError> {
        if property_ids.is_empty() {
            return Ok(());
        }
        let futures = property_ids
            .iter()
            .map(|id| Property::get(EntityId::property(id), &self.api))
            .collect::<Vec<_>>();
        let stream = futures::stream::iter(futures).buffer_unordered(self.max_concurrent_load);
        let results = stream.collect::<Vec<_>>().await;
        let results = results
            .into_iter()
            .collect::<Vec<Result<Property, RestApiError>>>();
        for property in results.into_iter().flatten() {
            let id = property.id().id()?.to_owned();
            properties.insert(id, property);
        }
        Ok(())
    }

    /// Returns a reference to the items in the container.
    pub fn items(&self) -> Arc<RwLock<HashMap<String, Item>>> {
        self.items.clone()
    }

    /// Returns a reference to the properties in the container.
    pub fn properties(&self) -> Arc<RwLock<HashMap<String, Property>>> {
        self.properties.clone()
    }
}

#[derive(Debug, Default)]
pub struct EntityContainerBuilder {
    api: Option<Arc<RestApi>>,
    max_concurrent_load: usize,
}

impl EntityContainerBuilder {
    /// Sets the `RestApi` to use for loading entities. **Mandatory**
    pub fn api(mut self, api: Arc<RestApi>) -> Self {
        self.api = Some(api);
        self
    }

    /// Sets the maximum number of concurrent loads to perform. Default is 10.
    pub const fn max_concurrent(mut self, max_concurrent_load: usize) -> Self {
        self.max_concurrent_load = max_concurrent_load;
        self
    }

    /// Builds a new `EntityContainer` with the configured options.
    pub fn build(self) -> Result<EntityContainer, RestApiError> {
        let api = self.api.ok_or_else(|| RestApiError::ApiNotSet)?;
        let mut max_concurrent_load = self.max_concurrent_load;
        if max_concurrent_load == 0 {
            max_concurrent_load = MAX_CONCURRENT_LOAD_DEFAULT;
        }
        Ok(EntityContainer {
            api,
            items: Arc::new(RwLock::new(HashMap::new())),
            properties: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_load,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RestApi;
    use serde_json::Value;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_entity_container() {
        // #lizard forgives the complexity
        let q42_str = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let q42: Value = serde_json::from_str(&q42_str).unwrap();
        let q255_str = std::fs::read_to_string("test_data/Q255.json").unwrap();
        let q255: Value = serde_json::from_str(&q255_str).unwrap();
        let p214_str = std::fs::read_to_string("test_data/P214.json").unwrap();
        let p214: Value = serde_json::from_str(&p214_str).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v0/entities/items/Q42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&q42))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v0/entities/items/Q255"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&q255))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v0/entities/properties/P214"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&p214))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        let ec = EntityContainer::builder()
            .api(Arc::new(api))
            .build()
            .unwrap();
        ec.load(&[
            EntityId::item("Q42"),
            EntityId::property("P214"),
            EntityId::item("Q255"),
        ])
        .await
        .unwrap();
        assert!(ec.items().read().await.contains_key("Q42"));
        assert!(ec.items().read().await.contains_key("Q255"));
        assert!(ec.properties().read().await.contains_key("P214"));
        assert!(!ec.properties().read().await.contains_key("Q42"));
        assert!(!ec.items().read().await.contains_key("P214"));
    }

    #[test]
    fn test_max_concurrent() {
        let api = Arc::new(
            RestApi::builder()
                .api("https://test.wikidata.org/w/rest.php")
                .build()
                .unwrap(),
        );
        let ec = EntityContainer::builder()
            .api(api.clone())
            .max_concurrent(5)
            .build()
            .unwrap();
        assert_eq!(ec.max_concurrent_load, 5);
    }

    #[test]
    fn test_max_concurrent_default() {
        let api = Arc::new(
            RestApi::builder()
                .api("https://test.wikidata.org/w/rest.php")
                .build()
                .unwrap(),
        );
        let ec = EntityContainer::builder()
            .api(api.clone())
            .max_concurrent(0)
            .build()
            .unwrap();
        assert_eq!(ec.max_concurrent_load, MAX_CONCURRENT_LOAD_DEFAULT);
    }
}
