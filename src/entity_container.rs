use crate::{entity::Entity, EntityId, Item, Property, RestApi, RestApiError};
use futures::prelude::*;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{RwLock, RwLockReadGuard};

/// One entity's fetch outcome.
type FetchOutcome<E> = (EntityId, Result<E, RestApiError>);

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
        // Load items and properties concurrently, each with its own retry sweeps.
        let (mut report, properties_report) = futures::future::join(
            self.load_with_retries(&self.items, entity_ids),
            self.load_with_retries(&self.properties, entity_ids),
        )
        .await;
        report.merge(properties_report);
        report
    }

    /// Returns the IDs of kind `E` from `entity_ids` that are not yet in `store`.
    fn ids_to_load<E: Entity>(
        store: &HashMap<String, E>,
        entity_ids: &[EntityId],
    ) -> Vec<EntityId> {
        entity_ids
            .iter()
            .filter(|id| id.kind() == Some(E::ENTITY_TYPE))
            .filter(|id| id.id().is_ok_and(|key| !store.contains_key(key)))
            .cloned()
            .collect()
    }

    /// Fetches entities concurrently, returning each ID's outcome (not flattened, so failures
    /// are preserved rather than silently dropped).
    async fn fetch<E: Entity>(&self, ids: &[EntityId], concurrency: usize) -> Vec<FetchOutcome<E>> {
        let api = &self.api;
        // Each future owns its ID; borrowing it from `ids` would make the future non-`Send`.
        let futures = ids.iter().cloned().map(|id| async move {
            let result = E::get(&id, api).await;
            (id, result)
        });
        futures::stream::iter(futures)
            .buffer_unordered(concurrency)
            .collect()
            .await
    }

    /// Loads all not-yet-loaded entities of kind `E` from `entity_ids` into `store`,
    /// re-sweeping rate-limited IDs up to `container_retries` times.
    async fn load_with_retries<E: Entity>(
        &self,
        store: &RwLock<HashMap<String, E>>,
        entity_ids: &[EntityId],
    ) -> LoadReport {
        let mut ids = Self::ids_to_load(&*store.read().await, entity_ids);
        let mut report = LoadReport::default();
        let mut concurrency = self.max_concurrent_load;
        for round in 0..=self.container_retries {
            if ids.is_empty() {
                break;
            }
            let outcomes = self.fetch::<E>(&ids, concurrency).await;
            let (loaded, rate_limited) = self.classify_round(outcomes, round, &mut report);
            if !loaded.is_empty() {
                let mut store = store.write().await;
                for (id, entity) in loaded {
                    if let Ok(key) = id.id() {
                        store.insert(key.clone(), entity);
                        report.loaded.push(id);
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
    /// missing (404) and terminal failures into `report`.
    fn classify_round<E>(
        &self,
        outcomes: Vec<FetchOutcome<E>>,
        round: usize,
        report: &mut LoadReport,
    ) -> (Vec<(EntityId, E)>, Vec<EntityId>) {
        let mut loaded = Vec::new();
        let mut rate_limited = Vec::new();
        for (id, result) in outcomes {
            match result {
                Ok(entity) => loaded.push((id, entity)),
                Err(e) if e.is_not_found() => report.missing.push(id),
                Err(e) if e.is_rate_limited() && round < self.container_retries => {
                    rate_limited.push(id);
                }
                Err(e) => report.failed.push((id, e)),
            }
        }
        (loaded, rate_limited)
    }

    /// Waits out the rate limit and halves concurrency before re-sweeping the failed IDs.
    async fn prepare_resweep(&self, ids: Vec<EntityId>, concurrency: &mut usize) -> Vec<EntityId> {
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

    /// Returns read access to all items in the container, keyed by ID.
    ///
    /// Loads wait while the returned guard is held, so drop it promptly.
    pub async fn items(&self) -> RwLockReadGuard<'_, HashMap<String, Item>> {
        self.items.read().await
    }

    /// Returns read access to all properties in the container, keyed by ID.
    ///
    /// Loads wait while the returned guard is held, so drop it promptly.
    pub async fn properties(&self) -> RwLockReadGuard<'_, HashMap<String, Property>> {
        self.properties.read().await
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

    #[test]
    fn test_load_report_future_is_send() {
        // The generic loader must still yield a `Send` future, so callers can spawn it.
        fn assert_send<T: Send>(_: &T) {}
        let api = Arc::new(RestApi::wikidata().unwrap());
        let container = EntityContainer::builder().api(api).build().unwrap();
        let ids = [EntityId::item("Q1")];
        let future = container.load_report(&ids);
        assert_send(&future);
    }

    #[test]
    fn test_ids_to_load() {
        let mut store = HashMap::new();
        store.insert("Q1".to_string(), Item::default());
        let ids = [
            EntityId::item("Q1"),
            EntityId::item("Q2"),
            EntityId::property("P1"),
            EntityId::None,
        ];
        assert_eq!(
            EntityContainer::ids_to_load(&store, &ids),
            vec![EntityId::item("Q2")]
        );
        let properties: HashMap<String, Property> = HashMap::new();
        assert_eq!(
            EntityContainer::ids_to_load(&properties, &ids),
            vec![EntityId::property("P1")]
        );
    }

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
        assert!(ec.items().await.contains_key("Q42"));
        assert!(ec.items().await.contains_key("Q255"));
        assert!(ec.properties().await.contains_key("P214"));
        assert!(!ec.properties().await.contains_key("Q42"));
        assert!(!ec.items().await.contains_key("P214"));

        // Convenience accessors.
        assert_eq!(
            ec.get_item("Q42").await.unwrap().id(),
            &EntityId::item("Q42")
        );
        assert!(ec.get_item("Q999").await.is_none());
        assert_eq!(
            ec.get_property("P214").await.unwrap().id(),
            &EntityId::property("P214")
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
            .build()
            .unwrap();
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
            .build()
            .unwrap();
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
            .build()
            .unwrap();
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
            .build()
            .unwrap();
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

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_load_single_kind_skips_empty_sweep() {
        // Loading only a property leaves the item sweep with an empty id list (immediate
        // break), and vice versa for loading only an item.
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(entity_json("Q1")))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/properties/P1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(entity_json("P1")))
            .mount(&mock_server)
            .await;
        let api = Arc::new(
            RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
                .unwrap()
                .build()
                .unwrap(),
        );

        // Only a property: item sweep starts empty.
        let ec = EntityContainer::builder().api(api.clone()).build().unwrap();
        ec.load(&[EntityId::property("P1")]).await.unwrap();
        assert!(ec.get_property("P1").await.is_some());

        // Only an item: property sweep starts empty.
        let ec2 = EntityContainer::builder().api(api).build().unwrap();
        ec2.load(&[EntityId::item("Q1")]).await.unwrap();
        assert!(ec2.get_item("Q1").await.is_some());
    }

    #[test]
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_max_concurrent() {
        let api = Arc::new(
            RestApi::builder("https://test.wikidata.org/w/rest.php")
                .unwrap()
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
    #[cfg_attr(miri, ignore)] // TODO this should work in miri
    fn test_max_concurrent_default() {
        let api = Arc::new(
            RestApi::builder("https://test.wikidata.org/w/rest.php")
                .unwrap()
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
