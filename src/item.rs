use crate::{
    aliases::Aliases,
    aliases_in_language::AliasesInLanguage,
    descriptions::Descriptions,
    entity::{Entity, EntityType},
    entity_patch::ItemPatch,
    labels::Labels,
    sitelinks::Sitelinks,
    statements::Statements,
    EntityId, FromJson, HeaderInfo, HttpMisc, RestApiError,
};
use derive_where::DeriveWhere;
use serde::Serialize;
use serde_json::Value;

#[derive(DeriveWhere, Debug, Clone, Default, Serialize)]
#[derive_where(PartialEq)]
pub struct Item {
    #[serde(skip_serializing_if = "EntityId::is_none")]
    id: EntityId,
    #[serde(skip_serializing_if = "Labels::is_empty")]
    labels: Labels,
    #[serde(skip_serializing_if = "Descriptions::is_empty")]
    descriptions: Descriptions,
    #[serde(skip_serializing_if = "Aliases::is_empty")]
    aliases: Aliases,
    #[serde(skip_serializing_if = "Sitelinks::is_empty")]
    sitelinks: Sitelinks,
    #[serde(skip_serializing_if = "Statements::is_empty")]
    statements: Statements,
    #[serde(skip)]
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl HttpMisc for Item {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        id.entity_path()
    }
}

impl FromJson for Item {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let id = j["id"]
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "id".into(),
                j: j.to_owned(),
            })?;
        Ok(Self {
            id: EntityId::item(id),
            labels: Labels::from_json_or_default(&j["labels"])?,
            descriptions: Descriptions::from_json_or_default(&j["descriptions"])?,
            aliases: Aliases::from_json_or_default(&j["aliases"])?,
            sitelinks: Sitelinks::from_json_or_default(&j["sitelinks"])?,
            statements: Statements::from_json_or_default(&j["statements"])?,
            header_info,
        })
    }
}

impl Entity for Item {
    const ENTITY_TYPE: EntityType = EntityType::Item;

    fn id(&self) -> &EntityId {
        &self.id
    }

    fn set_id(&mut self, id: EntityId) {
        self.id = id;
    }
}

impl Item {
    /// Returns the statements of the item.
    pub const fn statements(&self) -> &Statements {
        &self.statements
    }

    /// Returns the statements of the item (mutable).
    pub const fn statements_mut(&mut self) -> &mut Statements {
        &mut self.statements
    }

    /// Returns the labels of the item.
    pub const fn labels(&self) -> &Labels {
        &self.labels
    }

    /// Returns the labels of the item (mutable).
    pub const fn labels_mut(&mut self) -> &mut Labels {
        &mut self.labels
    }

    /// Returns the descriptions of the item.
    pub const fn descriptions(&self) -> &Descriptions {
        &self.descriptions
    }

    /// Returns the descriptions of the item (mutable).
    pub const fn descriptions_mut(&mut self) -> &mut Descriptions {
        &mut self.descriptions
    }

    /// Returns the aliases of the item.
    pub const fn aliases(&self) -> &Aliases {
        &self.aliases
    }

    /// Returns the aliases of the item (mutable).
    pub const fn aliases_mut(&mut self) -> &mut Aliases {
        &mut self.aliases
    }

    /// Returns the aliases of the item in one language, as an `AliasesInLanguage` object.
    pub fn as_aliases<S: Into<String>>(&self, lang: S) -> AliasesInLanguage {
        self.aliases.in_language(lang)
    }

    /// Returns the sitelinks of the item.
    pub const fn sitelinks(&self) -> &Sitelinks {
        &self.sitelinks
    }

    /// Returns the sitelinks of the item (mutable).
    pub const fn sitelinks_mut(&mut self) -> &mut Sitelinks {
        &mut self.sitelinks
    }

    /// Generates a patch to transform `other` into `self`.
    ///
    /// # Errors
    /// Returns an error if a statement in `other` has no ID (it can't be addressed).
    pub fn patch(&self, other: &Self) -> Result<ItemPatch, RestApiError> {
        Ok(ItemPatch::default()
            .with_part("/labels", self.labels.patch(&other.labels)?)
            .with_part(
                "/descriptions",
                self.descriptions.patch(&other.descriptions)?,
            )
            .with_part("/aliases", self.aliases.patch(&other.aliases)?)
            .with_part("/sitelinks", self.sitelinks.patch(&other.sitelinks)?)
            // Statement patch paths are already entity-level (`/statements/...`).
            .with_part("", self.statements.patch(&other.statements)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_strings::LanguageStrings;
    use crate::{LanguageString, Patch, PatchApply, PatchEntry, RestApi, Sitelink, Statement};
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn get_test_item(id: &str) -> Result<Item, RestApiError> {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q0"))
            .respond_with(ResponseTemplate::new(400).set_body_json(
                json!({"code": "invalid-item-id","message": "Not a valid item ID: Q0"}),
            ))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q6"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({"code": "item-not-found","message": "Could not find an item with the ID: Q6"})))
            .mount(&mock_server).await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        Item::get(&EntityId::item(id), &api).await
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_item_get() {
        let item = get_test_item("Q42").await.unwrap();
        assert_eq!(item.id(), &EntityId::item("Q42"));
        assert!(item.labels.has_language("en"));
        assert_eq!(item.labels().get_lang("en").unwrap(), "Douglas Adams");
        assert!(item
            .aliases()
            .get_lang("en")
            .contains(&"Douglas Noël Adams".to_string()));
        assert!(item.descriptions.has_language("en"));
        assert!(item.aliases.has_language("en"));
        assert!(item.sitelinks.get_wiki("enwiki").is_some());
        assert!(!item.statements.is_empty());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_item_post() {
        let mut item = get_test_item("Q42").await.unwrap();
        let v = item.to_owned();

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/w/rest.php/wikibase/v1/entities/items"))
            .and(body_partial_json(
                json!({"item": {"labels": {"en": item.labels().get_lang("en")}}}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        // Check that an error is returned when trying to post an item that already has an ID
        let r0 = item.post(&api).await;
        assert_eq!(r0.err().unwrap().to_string(), "ID already set");

        // Clear the ID and try again
        item.id = EntityId::None;
        let r1 = item.post(&api).await.unwrap();
        assert_eq!(r1.id(), v.id());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_item_post_404() {
        let item = Item::default();
        let mock_server = MockServer::start().await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();
        let r = item.post(&api).await;
        assert_eq!(
            r.err().unwrap().to_string(),
            "Method POST not implemented for path /entities/items in REST API"
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_invalid_item() {
        let item = get_test_item("Q0").await;
        // assert_eq!(item.err().unwrap().to_string(), "invalid-item-id");
        let err = item.err().unwrap();
        match err {
            RestApiError::ApiError {
                status,
                status_text,
                payload,
            } => {
                assert_eq!(status, 400);
                assert_eq!(status_text, "Bad Request");
                assert_eq!(payload.code(), "invalid-item-id");
                assert_eq!(payload.message(), "Not a valid item ID: Q0");
                assert_eq!(payload.context().len(), 0);
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_deleted_item() {
        let item = get_test_item("Q6").await;
        let err = item.err().unwrap();
        match err {
            RestApiError::ApiError {
                status,
                status_text,
                payload,
            } => {
                assert_eq!(status, 404);
                assert_eq!(status_text, "Not Found");
                assert_eq!(payload.code(), "item-not-found");
                assert_eq!(payload.message(), "Could not find an item with the ID: Q6");
                assert_eq!(payload.context().len(), 0);
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_json_serialize() {
        let item = get_test_item("Q42").await.unwrap();
        let j = serde_json::to_string(&item).unwrap(); // Convert item to JSON text
        let v: Value = serde_json::from_str(&j).unwrap(); // Convert to JSON value
        let item_from_json = Item::from_json(&v).unwrap(); // Convert back to Item
        assert_eq!(item, item_from_json); // Check if the reconstituted item is identical to the original
    }

    #[test]
    fn test_labels() {
        let mut item = Item::default();
        assert_eq!(item.labels().len(), 0);
        item.labels_mut().insert(LanguageString::new("en", "label"));
        assert_eq!(item.labels().len(), 1);
    }

    #[test]
    fn test_descriptions() {
        let mut item = Item::default();
        assert_eq!(item.descriptions().len(), 0);
        item.descriptions_mut()
            .insert(LanguageString::new("en", "description"));
        assert_eq!(item.descriptions().len(), 1);
    }

    #[test]
    fn test_aliases() {
        let mut item = Item::default();
        assert_eq!(item.aliases().len(), 0);
        item.aliases_mut()
            .insert(LanguageString::new("en", "alias"));
        assert_eq!(item.aliases().len(), 1);
    }

    #[test]
    fn test_as_aliases() {
        let mut item = Item::default();
        item.aliases_mut()
            .insert(LanguageString::new("en", "alias"));
        let aliases = item.as_aliases("en");
        assert_eq!(aliases.len(), 1);
    }

    #[test]
    fn test_statements() {
        let mut item = Item::default();
        assert_eq!(item.statements().len(), 0);
        item.statements_mut()
            .insert(Statement::new_string("P31", "Q42"));
        assert_eq!(item.statements().len(), 1);
    }

    #[test]
    fn test_sitelinks() {
        let mut item = Item::default();
        assert_eq!(item.sitelinks().len(), 0);
        item.sitelinks_mut()
            .set_wiki(Sitelink::new("enwiki", "Q42"));
        assert_eq!(item.sitelinks().len(), 1);
    }

    #[test]
    fn test_header_info() {
        let hi = HeaderInfo::default();
        let item = Item::default();
        assert_eq!(item.header_info(), &hi);
    }

    #[test]
    fn test_get_rest_api_path() {
        let item = Item::default();
        let id = EntityId::item("Q42");
        let path = item.get_my_rest_api_path(&id).unwrap();
        assert_eq!(path, "/entities/items/Q42");
    }

    #[test]
    fn test_set_id() {
        let mut item = Item::default();
        assert!(item.id().is_none());
        item.set_id(EntityId::item("Q42"));
        assert_eq!(item.id(), &EntityId::item("Q42"));
    }

    #[test]
    fn test_from_json_with_id_and_serialize() {
        let v = json!({"id": "Q42", "labels": {"en": "Douglas Adams"}});
        let item = Item::from_json(&v).unwrap();
        assert_eq!(item.id(), &EntityId::item("Q42"));
        assert_eq!(item.labels().get_lang("en"), Some("Douglas Adams"));
        // Serializing an item that has an ID emits the `id` field.
        let j = serde_json::to_value(&item).unwrap();
        assert_eq!(j["id"], "Q42");
        assert_eq!(j["labels"]["en"], "Douglas Adams");
    }

    #[test]
    fn test_patch() {
        let mut item1 = Item::default();
        let mut item2 = Item::default();
        item1
            .labels_mut()
            .insert(LanguageString::new("en", "label"));
        item2
            .labels_mut()
            .insert(LanguageString::new("en", "label2"));
        let patch = item1.patch(&item2).unwrap();
        // Sub-patch paths are lifted to entity-level paths.
        assert_eq!(
            patch.patch(),
            &vec![PatchEntry::new("replace", "/labels/en", json!("label"))]
        );
    }

    #[test]
    fn test_patch_all_parts() {
        let before = Item::default();
        let mut after = Item::default();
        after.labels_mut().insert(LanguageString::new("en", "L"));
        after
            .descriptions_mut()
            .insert(LanguageString::new("en", "D"));
        after.aliases_mut().insert(LanguageString::new("en", "A"));
        after.sitelinks_mut().set_wiki(Sitelink::new("enwiki", "T"));
        after
            .statements_mut()
            .insert(Statement::new_string("P1", "S"));
        let patch = after.patch(&before).unwrap();
        let paths: Vec<&str> = patch.patch().iter().map(|e| e.path()).collect();
        assert_eq!(
            paths,
            vec![
                "/labels/en",
                "/descriptions/en",
                "/aliases/en",
                "/sitelinks/enwiki",
                "/statements/P1/-"
            ]
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_patch_apply_round_trip() {
        // Item::patch + apply hits the entity endpoint with entity-level paths.
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/w/rest.php/wikibase/v1/entities/items/Q1"))
            .and(body_partial_json(json!({
                "patch": [{"op": "add", "path": "/labels/de", "value": "Eins"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "Q1", "labels": {"de": "Eins"}
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();
        let before = Item::from_json(&json!({"id": "Q1"})).unwrap();
        let mut after = before.clone();
        after.labels_mut().insert(LanguageString::new("de", "Eins"));
        let patched = after
            .patch(&before)
            .unwrap()
            .apply(before.id(), &api)
            .await
            .unwrap();
        assert_eq!(patched.labels().get_lang("de"), Some("Eins"));
    }
}
