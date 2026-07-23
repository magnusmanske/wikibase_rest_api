use crate::{entity::Entity, EntityId, Item, Property, RestApi, RestApiError};
use futures::prelude::*;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

const MAX_CONCURRENT_LOAD_DEFAULT: usize = 10;
const CONTAINER_RETRIES_DEFAULT: usize = 2;
const CONTAINER_BACKOFF_DEFAULT: Duration = Duration::from_secs(5);

/// Outcome of loading a batch of entities. Nothing is ever dropped silently:
/// every requested ID ends up in exactly one of the three buckets.
#[derive(Debug, Default)]
pub struct LoadReport {
    loaded: Vec<EntityId>,
    missing: Vec<EntityId>,
    failed: Vec<(EntityId, RestApiError)>,
}

impl LoadReport {
    /// IDs that were successfully loaded into the container.
    pub fn loaded(&self) -> &[EntityId] {
        &self.loaded
    }

    /// IDs the server reported as not found (HTTP 404). Routine for bulk loads, not an error.
    pub fn missing(&self) -> &[EntityId] {
        &self.missing
    }

    /// IDs that failed to load, with the error, after all retries were exhausted.
    pub fn failed(&self) -> &[(EntityId, RestApiError)] {
        &self.failed
    }

    /// Returns `true` if any entity failed to load.
    pub const fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    fn merge(&mut self, other: Self) {
        self.loaded.extend(other.loaded);
        self.missing.extend(other.missing);
        self.failed.extend(other.failed);
    }
}

#[derive(Debug, Clone)]
pub struct EntityContainer {
    api: Arc<RestApi>,
    items: Arc<RwLock<HashMap<String, Item>>>,
    properties: Arc<RwLock<HashMap<String, Property>>>,
    max_concurrent_load: usize,
    container_retries: usize,
    container_backoff: Duration,
}

impl EntityContainer {
    /// Returns a new `EntityContainerBuilder` to configure a new `EntityContainer`.
    pub fn builder() -> EntityContainerBuilder {
        EntityContainerBuilder::default()
    }

    /// Loads the entities with the given `EntityId`s into the container.
    ///
    /// This is the simple path: entities the server reports as missing (HTTP 404) are
    /// simply absent afterwards, and the first genuine failure is returned as an error.
    /// For bulk loads where partial success must be inspected, use [`load_report`](Self::load_report).
    ///
    /// # Errors
    /// Returns the first `RestApiError` encountered (after retries), if any.
    pub async fn load(&self, entity_ids: &[EntityId]) -> Result<(), RestApiError> {
        let report = self.load_report(entity_ids).await;
        match report.failed.into_iter().next() {
            Some((_id, error)) => Err(error),
            None => Ok(()),
        }
    }

    /// Loads the given `EntityId`s and returns a full [`LoadReport`], never aborting early.
    ///
    /// Successfully loaded entities are inserted into the container. A 429 (rate limit)
    /// triggers a re-sweep of only the affected IDs, at halved concurrency, up to
    /// `container_retries` times; other failures are reported as-is (the request layer
    /// has already retried them).
    pub async fn load_report(&self, entity_ids: &[EntityId]) -> LoadReport {
        let item_ids = {
            let items = self.items.read().await;
            Self::get_items_to_load(&items, entity_ids)
        };
        let property_ids = {
            let properties = self.properties.read().await;
            Self::get_properties_to_load(&properties, entity_ids)
        };

        // Load items and properties concurrently, each with its own retry sweeps.
        let (mut report, properties_report) = futures::future::join(
            self.load_items_with_retries(item_ids),
            self.load_properties_with_retries(property_ids),
        )
        .await;
        report.merge(properties_report);
        report
    }

    fn get_items_to_load(items: &HashMap<String, Item>, entity_ids: &[EntityId]) -> Vec<String> {
        entity_ids
            .iter()
            .filter_map(|id| match id {
                EntityId::Item(id) => Some(id.as_str()),
                _ => None,
            })
            .filter(|id| !items.contains_key(*id))
            .map(ToOwned::to_owned)
            .collect()
    }

    fn get_properties_to_load(
        properties: &HashMap<String, Property>,
        entity_ids: &[EntityId],
    ) -> Vec<String> {
        entity_ids
            .iter()
            .filter_map(|id| match id {
                EntityId::Property(id) => Some(id.as_str()),
                _ => None,
            })
            .filter(|id| !properties.contains_key(*id))
            .map(ToOwned::to_owned)
            .collect()
    }

    /// Fetches items concurrently, returning each ID's outcome (not flattened, so failures
    /// are preserved rather than silently dropped).
    async fn fetch_items(
        &self,
        item_ids: &[String],
        concurrency: usize,
    ) -> Vec<(String, Result<Item, RestApiError>)> {
        let futures = item_ids.iter().map(|id| {
            let id = id.clone();
            async move {
                let result = Item::get(EntityId::item(id.as_str()), &self.api).await;
                (id, result)
            }
        });
        futures::stream::iter(futures)
            .buffer_unordered(concurrency)
            .collect()
            .await
    }

    async fn fetch_properties(
        &self,
        property_ids: &[String],
        concurrency: usize,
    ) -> Vec<(String, Result<Property, RestApiError>)> {
        let futures = property_ids.iter().map(|id| {
            let id = id.clone();
            async move {
                let result = Property::get(EntityId::property(id.as_str()), &self.api).await;
                (id, result)
            }
        });
        futures::stream::iter(futures)
            .buffer_unordered(concurrency)
            .collect()
            .await
    }

    async fn load_items_with_retries(&self, mut ids: Vec<String>) -> LoadReport {
        let mut report = LoadReport::default();
        let mut concurrency = self.max_concurrent_load;
        for round in 0..=self.container_retries {
            if ids.is_empty() {
                break;
            }
            let outcomes = self.fetch_items(&ids, concurrency).await;
            let (loaded, rate_limited) =
                self.classify_round(outcomes, round, EntityId::Item, &mut report);
            if !loaded.is_empty() {
                let mut items = self.items.write().await;
                for item in loaded {
                    if let Ok(id) = item.id().id() {
                        report.loaded.push(EntityId::Item(id.clone()));
                        items.insert(id.clone(), item);
                    }
                }
            }
            if rate_limited.is_empty() {
                break;
            }
            ids = self.prepare_resweep(rate_limited, &mut concurrency).await;
        }
        report
    }

    async fn load_properties_with_retries(&self, mut ids: Vec<String>) -> LoadReport {
        let mut report = LoadReport::default();
        let mut concurrency = self.max_concurrent_load;
        for round in 0..=self.container_retries {
            if ids.is_empty() {
                break;
            }
            let outcomes = self.fetch_properties(&ids, concurrency).await;
            let (loaded, rate_limited) =
                self.classify_round(outcomes, round, EntityId::Property, &mut report);
            if !loaded.is_empty() {
                let mut properties = self.properties.write().await;
                for property in loaded {
                    if let Ok(id) = property.id().id() {
                        report.loaded.push(EntityId::Property(id.clone()));
                        properties.insert(id.clone(), property);
                    }
                }
            }
            if rate_limited.is_empty() {
                break;
            }
            ids = self.prepare_resweep(rate_limited, &mut concurrency).await;
        }
        report
    }

    /// Sorts one round's outcomes into loaded entities and the IDs to re-sweep, recording
    /// missing (404) and terminal failures into `report`. `make_id` tags IDs by entity kind.
    fn classify_round<E>(
        &self,
        outcomes: Vec<(String, Result<E, RestApiError>)>,
        round: usize,
        make_id: fn(String) -> EntityId,
        report: &mut LoadReport,
    ) -> (Vec<E>, Vec<String>) {
        let mut loaded = Vec::new();
        let mut rate_limited = Vec::new();
        for (id, result) in outcomes {
            match result {
                Ok(entity) => loaded.push(entity),
                Err(e) if e.is_not_found() => report.missing.push(make_id(id)),
                Err(e) if e.is_rate_limited() && round < self.container_retries => {
                    rate_limited.push(id);
                }
                Err(e) => report.failed.push((make_id(id), e)),
            }
        }
        (loaded, rate_limited)
    }

    /// Waits out the rate limit and halves concurrency before re-sweeping the failed IDs.
    async fn prepare_resweep(&self, ids: Vec<String>, concurrency: &mut usize) -> Vec<String> {
        tokio::time::sleep(self.container_backoff).await;
        *concurrency = (*concurrency / 2).max(1);
        ids
    }

    /// Returns a clone of a loaded item, if present.
    pub async fn get_item<S: AsRef<str>>(&self, id: S) -> Option<Item> {
        self.items.read().await.get(id.as_ref()).cloned()
    }

    /// Returns a clone of a loaded property, if present.
    pub async fn get_property<S: AsRef<str>>(&self, id: S) -> Option<Property> {
        self.properties.read().await.get(id.as_ref()).cloned()
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
    container_retries: Option<usize>,
    container_backoff: Option<Duration>,
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

    /// Sets how many times a rate-limited (429) subset is re-swept. Default is 2.
    pub const fn container_retries(mut self, retries: usize) -> Self {
        self.container_retries = Some(retries);
        self
    }

    /// Sets the delay before re-sweeping a rate-limited subset. Default is 5 seconds.
    pub const fn container_backoff(mut self, backoff: Duration) -> Self {
        self.container_backoff = Some(backoff);
        self
    }

    /// Builds a new `EntityContainer` with the configured options.
    ///
    /// # Errors
    /// Returns an `RestApiError` if the API could not be built.
    pub fn build(self) -> Result<EntityContainer, RestApiError> {
        let api = self.api.ok_or(RestApiError::ApiNotSet)?;
        let mut max_concurrent_load = self.max_concurrent_load;
        if max_concurrent_load == 0 {
            max_concurrent_load = MAX_CONCURRENT_LOAD_DEFAULT;
        }
        Ok(EntityContainer {
            api,
            container_retries: self.container_retries.unwrap_or(CONTAINER_RETRIES_DEFAULT),
            container_backoff: self.container_backoff.unwrap_or(CONTAINER_BACKOFF_DEFAULT),
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
    #[cfg_attr(miri, ignore)]
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
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&q42))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q255"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&q255))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/properties/P214"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&p214))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

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

        // Convenience accessors.
        assert_eq!(
            ec.get_item("Q42").await.unwrap().id(),
            EntityId::item("Q42")
        );
        assert!(ec.get_item("Q999").await.is_none());
        assert_eq!(
            ec.get_property("P214").await.unwrap().id(),
            EntityId::property("P214")
        );
        assert!(ec.get_property("P999").await.is_none());
    }

    // A minimal valid entity body: only the `id` field is required, the rest default.
    fn entity_json(id: &str) -> Value {
        serde_json::json!({ "id": id })
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_report_classifies_outcomes() {
        // #lizard forgives the complexity
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(entity_json("Q1")))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q6"))
            .respond_with(
                ResponseTemplate::new(404).set_body_json(
                    serde_json::json!({"code": "item-not-found", "message": "gone"}),
                ),
            )
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q7"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;
        // max_retries=0 so the 500 surfaces immediately rather than being retried.
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_max_retries(0)
            .build();
        let ec = EntityContainer::builder()
            .api(Arc::new(api))
            .build()
            .unwrap();

        let report = ec
            .load_report(&[
                EntityId::item("Q1"),
                EntityId::item("Q6"),
                EntityId::item("Q7"),
            ])
            .await;

        assert_eq!(report.loaded(), &[EntityId::item("Q1")]);
        assert_eq!(report.missing(), &[EntityId::item("Q6")]);
        assert_eq!(report.failed().len(), 1);
        assert_eq!(report.failed()[0].0, EntityId::item("Q7"));
        assert!(report.has_failures());
        // The loaded entity is actually in the container.
        assert!(ec.get_item("Q1").await.is_some());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_aborts_on_failure_but_not_on_missing() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q6"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q7"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_max_retries(0)
            .build();
        let ec = EntityContainer::builder()
            .api(Arc::new(api))
            .build()
            .unwrap();

        // A missing (404) entity is not an error for load().
        assert!(ec.load(&[EntityId::item("Q6")]).await.is_ok());
        // A genuine failure aborts load().
        assert!(ec.load(&[EntityId::item("Q7")]).await.is_err());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_report_429_resweep() {
        // Q1 is rate-limited once, then succeeds; Q2 succeeds on the first try.
        // With max_retries=0 the 429 surfaces to the container, which re-sweeps only Q1.
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q1"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(1)
            .expect(1)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(entity_json("Q1")))
            .expect(1)
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(entity_json("Q2")))
            .expect(1) // proves Q2 is not re-swept
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_max_retries(0)
            .build();
        let ec = EntityContainer::builder()
            .api(Arc::new(api))
            .container_backoff(std::time::Duration::from_millis(1))
            .build()
            .unwrap();

        let report = ec
            .load_report(&[EntityId::item("Q1"), EntityId::item("Q2")])
            .await;

        assert!(!report.has_failures());
        assert_eq!(report.loaded().len(), 2);
        assert!(ec.get_item("Q1").await.is_some());
        // Expectations verified on drop confirm Q1 was fetched twice, Q2 once.
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_report_429_exhausted_is_failure() {
        // Persistent 429 with no successful re-sweep ends up as a failure, not a silent drop.
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q1"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_max_retries(0)
            .build();
        let ec = EntityContainer::builder()
            .api(Arc::new(api))
            .container_retries(1)
            .container_backoff(std::time::Duration::from_millis(1))
            .build()
            .unwrap();

        let report = ec.load_report(&[EntityId::item("Q1")]).await;
        assert_eq!(report.failed().len(), 1);
        assert!(report.failed()[0].1.is_rate_limited());
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_max_concurrent() {
        let api = Arc::new(
            RestApi::builder("https://test.wikidata.org/w/rest.php")
                .unwrap()
                .build(),
        );
        let ec = EntityContainer::builder()
            .api(api.clone())
            .max_concurrent(5)
            .build()
            .unwrap();
        assert_eq!(ec.max_concurrent_load, 5);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_max_concurrent_default() {
        let api = Arc::new(
            RestApi::builder("https://test.wikidata.org/w/rest.php")
                .unwrap()
                .build(),
        );
        let ec = EntityContainer::builder()
            .api(api.clone())
            .max_concurrent(0)
            .build()
            .unwrap();
        assert_eq!(ec.max_concurrent_load, MAX_CONCURRENT_LOAD_DEFAULT);
    }
}
