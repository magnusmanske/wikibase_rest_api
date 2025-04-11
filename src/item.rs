use crate::{
    aliases::Aliases,
    aliases_in_language::AliasesInLanguage,
    descriptions::Descriptions,
    entity::{Entity, EntityType},
    entity_patch::EntityPatch,
    labels::Labels,
    sitelinks::Sitelinks,
    statements::Statements,
    EntityId, FromJson, HeaderInfo, HttpMisc, Patch, RestApi, RestApiError,
};
use async_trait::async_trait;
use derivative::Derivative;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;

#[derive(Derivative, Debug, Clone, Default)]
#[derivative(PartialEq)]
pub struct Item {
    id: EntityId,
    labels: Labels,
    descriptions: Descriptions,
    aliases: Aliases,
    sitelinks: Sitelinks,
    statements: Statements,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl HttpMisc for Item {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!("/entities/{}/{id}", id.group()?))
    }
}

#[async_trait]
impl Entity for Item {
    fn id(&self) -> EntityId {
        self.id.to_owned()
    }

    fn from_json_header_info(j: Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let id = j["id"]
            .as_str()
            .ok_or(RestApiError::MissingOrInvalidField {
                field: "id".into(),
                j: j.to_owned(),
            })?
            .to_string();
        Ok(Self {
            id: EntityId::Item(id),
            labels: Labels::from_json(&j["labels"])?,
            descriptions: Descriptions::from_json(&j["descriptions"])?,
            aliases: Aliases::from_json(&j["aliases"])?,
            sitelinks: Sitelinks::from_json(&j["sitelinks"])?,
            statements: Statements::from_json(&j["statements"])?,
            header_info,
        })
    }

    async fn post(&self, api: &RestApi) -> Result<Self, RestApiError> {
        self.post_with_type(EntityType::Item, api).await
    }
}

impl Serialize for Item {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // #lizard forgives the complexity
        let mut fields = 5;
        if self.id.is_some() {
            fields += 1;
        }
        if self.labels.is_empty() {
            fields -= 1;
        }
        if self.descriptions.is_empty() {
            fields -= 1;
        }
        if self.aliases.is_empty() {
            fields -= 1;
        }
        if self.sitelinks.is_empty() {
            fields -= 1;
        }
        if self.statements.is_empty() {
            fields -= 1;
        }
        let mut s = serializer.serialize_struct("Item", fields)?;
        if self.id.is_some() {
            let id: String = self.id.to_owned().into();
            s.serialize_field("id", &id)?;
        }
        if !self.labels.is_empty() {
            s.serialize_field("labels", &self.labels)?;
        }
        if !self.descriptions.is_empty() {
            s.serialize_field("descriptions", &self.descriptions)?;
        }
        if !self.aliases.is_empty() {
            s.serialize_field("aliases", &self.aliases)?;
        }
        if !self.sitelinks.is_empty() {
            s.serialize_field("sitelinks", &self.sitelinks)?;
        }
        if !self.statements.is_empty() {
            s.serialize_field("statements", &self.statements)?;
        }
        s.end()
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

    /// Returns the aliases of the item as an `Aliases` object.
    pub fn as_aliases<S: Into<String>>(&self, lang: S) -> AliasesInLanguage {
        let lang: String = lang.into();
        let v: Vec<String> = self
            .aliases
            .get_lang(&lang)
            .iter()
            .map(|x| x.to_string())
            .collect();
        AliasesInLanguage::new(lang, v)
    }

    /// Returns the sitelinks of the item.
    pub const fn sitelinks(&self) -> &Sitelinks {
        &self.sitelinks
    }

    /// Returns the sitelinks of the item (mutable).
    pub const fn sitelinks_mut(&mut self) -> &mut Sitelinks {
        &mut self.sitelinks
    }

    /// Returns the header information of the item.
    pub const fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    /// Generates a patch to transform `other` into `self`
    pub fn patch(&self, other: &Self) -> Result<EntityPatch, RestApiError> {
        let labels_patch = self.labels.patch(other.labels())?;
        let descriptions_patch = self.descriptions.patch(other.descriptions())?;
        let aliases_patch = self.aliases.patch(other.aliases())?;
        let sitelinks_patch = self.sitelinks.patch(other.sitelinks())?;
        let statements_patch = self.statements.patch(other.statements())?;

        let mut ret = EntityPatch::item();
        ret.patch_mut().extend(labels_patch.patch().to_owned());
        ret.patch_mut()
            .extend(descriptions_patch.patch().to_owned());
        ret.patch_mut().extend(aliases_patch.patch().to_owned());
        ret.patch_mut().extend(sitelinks_patch.patch().to_owned());
        ret.patch_mut().extend(statements_patch.patch().to_owned());

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_strings::LanguageStrings;
    use crate::{LanguageString, RestApi, Sitelink, Statement};
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
            .build();

        Item::get(EntityId::item(id), &api).await
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_item_get() {
        let item = get_test_item("Q42").await.unwrap();
        assert_eq!(item.id(), EntityId::item("Q42"));
        assert!(item.labels.has_language("en"));
        assert_eq!(item.labels().get_lang("en").unwrap(), "Douglas Adams");
        assert!(item
            .aliases()
            .get_lang("en")
            .contains(&"Douglas Noël Adams"));
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
            .build();

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
            .build();
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
        let item_from_json = Item::from_json(v).unwrap(); // Convert back to Item
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
        assert_eq!(patch.patch().len(), 1);
    }
}
