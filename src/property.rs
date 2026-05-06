use crate::{
    aliases::Aliases,
    aliases_in_language::AliasesInLanguage,
    descriptions::Descriptions,
    entity::{Entity, EntityType},
    entity_patch::EntityPatch,
    labels::Labels,
    patch::Patch,
    statements::Statements,
    DataType, EntityId, FromJson, HeaderInfo, HttpMisc, RestApi, RestApiError,
};
use derive_where::DeriveWhere;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::Value;

#[derive(DeriveWhere, Debug, Clone, Default)]
#[derive_where(PartialEq)]
pub struct Property {
    id: EntityId,
    data_type: Option<DataType>,
    labels: Labels,
    descriptions: Descriptions,
    aliases: Aliases,
    statements: Statements,
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl HttpMisc for Property {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!("/entities/{}/{id}", id.group()?))
    }
}

impl Entity for Property {
    fn id(&self) -> EntityId {
        self.id.to_owned()
    }

    fn set_id(&mut self, id: EntityId) {
        self.id = id;
    }

    fn from_json_header_info(j: Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let id = j["id"]
            .as_str()
            .ok_or(RestApiError::MissingOrInvalidField {
                field: "id".to_string(),
                j: j.clone(),
            })?;
        let data_type = j["data_type"].as_str().and_then(|s| DataType::new(s).ok());
        Ok(Self {
            id: EntityId::property(id),
            data_type,
            labels: Labels::from_json_or_default(&j["labels"])?,
            descriptions: Descriptions::from_json_or_default(&j["descriptions"])?,
            aliases: Aliases::from_json_or_default(&j["aliases"])?,
            statements: Statements::from_json_or_default(&j["statements"])?,
            header_info,
        })
    }

    async fn post(&self, api: &RestApi) -> Result<Self, RestApiError> {
        self.post_with_type(EntityType::Property, api).await
    }
}

impl Serialize for Property {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // #lizard forgives the complexity
        let mut fields = 5;
        if self.id.is_some() {
            fields += 1;
        }
        if self.data_type.is_some() {
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
        if self.statements.is_empty() {
            fields -= 1;
        }
        let mut s = serializer.serialize_struct("Property", fields)?;
        if self.id.is_some() {
            let id: String = self.id.to_owned().into();
            s.serialize_field("id", &id)?;
        }
        if let Some(dt) = self.data_type {
            s.serialize_field("data_type", dt.as_str())?;
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
        if !self.statements.is_empty() {
            s.serialize_field("statements", &self.statements)?;
        }
        s.end()
    }
}

impl Property {
    /// Returns the statements of the property
    pub const fn statements(&self) -> &Statements {
        &self.statements
    }

    /// Returns the statements of the property, mutable
    pub const fn statements_mut(&mut self) -> &mut Statements {
        &mut self.statements
    }

    /// Returns the labels of the property
    pub const fn labels(&self) -> &Labels {
        &self.labels
    }

    /// Returns the labels of the property, mutable
    pub const fn labels_mut(&mut self) -> &mut Labels {
        &mut self.labels
    }

    /// Returns the descriptions of the property
    pub const fn descriptions(&self) -> &Descriptions {
        &self.descriptions
    }

    /// Returns the descriptions of the property, mutable
    pub const fn descriptions_mut(&mut self) -> &mut Descriptions {
        &mut self.descriptions
    }

    /// Returns the aliases of the property
    pub const fn aliases(&self) -> &Aliases {
        &self.aliases
    }

    /// Returns the aliases of the property, mutable
    pub const fn aliases_mut(&mut self) -> &mut Aliases {
        &mut self.aliases
    }

    /// Returns the aliases of the property for a specific language, as an `Aliases` object
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

    /// Returns the header info of the property
    pub const fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    /// Returns the data type of the property
    pub const fn data_type(&self) -> Option<DataType> {
        self.data_type
    }

    /// Sets the data type of the property
    pub const fn set_data_type(&mut self, data_type: Option<DataType>) {
        self.data_type = data_type;
    }

    /// Generates a patch to transform `other` into `self`
    pub fn patch(&self, other: &Self) -> Result<EntityPatch, RestApiError> {
        let labels_patch = self.labels.patch(other.labels())?;
        let descriptions_patch = self.descriptions.patch(other.descriptions())?;
        let aliases_patch = self.aliases.patch(other.aliases())?;
        let statements_patch = self.statements.patch(other.statements())?;

        let mut ret = EntityPatch::property();
        ret.patch_mut().extend(labels_patch.patch().to_owned());
        ret.patch_mut()
            .extend(descriptions_patch.patch().to_owned());
        ret.patch_mut().extend(aliases_patch.patch().to_owned());
        ret.patch_mut().extend(statements_patch.patch().to_owned());

        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_strings::LanguageStrings;
    use crate::{LanguageString, RestApi, Statement};
    use serde_json::json;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_property_get_and_json_serialize() {
        let p214 = std::fs::read_to_string("test_data/P214.json").unwrap();
        let v214: Value = serde_json::from_str(&p214).unwrap();

        let mock_path = "/w/rest.php/wikibase/v1/entities/properties/P214";
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v214))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let property = Property::get(EntityId::property("P214"), &api)
            .await
            .unwrap();
        assert_eq!(property.data_type(), Some(DataType::ExternalId));
        let j = serde_json::to_string(&property).unwrap(); // Convert property to JSON text
        let v: Value = serde_json::from_str(&j).unwrap(); // Convert to JSON value
        let property_from_json = Property::from_json(v).unwrap(); // Convert back to property
        assert_eq!(property, property_from_json); // Check if the reconstituted property is identical to the original
    }

    #[test]
    fn test_id() {
        let id = EntityId::property("P214");
        let property = Property {
            id: id.to_owned(),
            ..Default::default()
        };
        assert_eq!(property.id(), id);
    }

    #[test]
    fn test_statements() {
        let mut property = Property::default();
        assert_eq!(property.statements().len(), 0);
        property
            .statements_mut()
            .insert(Statement::new_string("P31", "Q42"));
        assert_eq!(property.statements().len(), 1);
    }

    #[test]
    fn test_labels() {
        let mut property = Property::default();
        assert_eq!(property.labels().len(), 0);
        property
            .labels_mut()
            .insert(LanguageString::new("en", "label"));
        assert_eq!(property.labels().len(), 1);
    }

    #[test]
    fn test_descriptions() {
        let mut property = Property::default();
        assert_eq!(property.descriptions().len(), 0);
        property
            .descriptions_mut()
            .insert(LanguageString::new("en", "description"));
        assert_eq!(property.descriptions().len(), 1);
    }

    #[test]
    fn test_aliases() {
        let mut property = Property::default();
        assert_eq!(property.aliases().len(), 0);
        property
            .aliases_mut()
            .insert(LanguageString::new("en", "alias"));
        assert_eq!(property.aliases().len(), 1);
    }

    #[test]
    fn test_as_aliases() {
        let mut property = Property::default();
        property
            .aliases_mut()
            .insert(LanguageString::new("en", "alias"));
        let aliases = property.as_aliases("en");
        assert_eq!(aliases.len(), 1);
    }

    #[test]
    fn test_header_info() {
        let header_info = HeaderInfo::default();
        let property = Property {
            header_info: header_info.to_owned(),
            ..Default::default()
        };
        assert_eq!(property.header_info(), &header_info);
    }

    #[test]
    fn test_serialize() {
        let mut property = Property {
            id: EntityId::property("P214"),
            data_type: Some(DataType::ExternalId),
            ..Default::default()
        };
        property
            .labels_mut()
            .insert(LanguageString::new("en", "label"));
        property
            .descriptions_mut()
            .insert(LanguageString::new("en", "description"));
        property
            .aliases_mut()
            .insert(LanguageString::new("en", "alias"));
        let j = serde_json::to_string(&property).unwrap();
        let v: Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["id"], "P214");
        assert_eq!(v["data_type"], "external-id");
        assert_eq!(v["labels"]["en"], "label");
        assert_eq!(v["descriptions"]["en"], "description");
        assert_eq!(v["aliases"]["en"][0], "alias");
    }

    #[test]
    fn test_from_json() {
        let v = json!({
            "id": "P214",
            "data_type": "external-id",
            "labels": {"en": "label"},
            "descriptions": {"en": "description"},
            "aliases": {"en": ["alias"]},
            "statements": {},
        });
        let property = Property::from_json(v).unwrap();
        assert_eq!(property.id(), EntityId::property("P214"));
        assert_eq!(property.data_type(), Some(DataType::ExternalId));
        assert_eq!(property.labels().get_lang("en").unwrap(), "label");
        assert_eq!(
            property.descriptions().get_lang("en").unwrap(),
            "description"
        );
        assert_eq!(property.aliases().get_lang("en"), &["alias"]);
    }

    #[test]
    fn test_data_type() {
        let mut property = Property::default();
        assert_eq!(property.data_type(), None);
        property.set_data_type(Some(DataType::WikibaseItem));
        assert_eq!(property.data_type(), Some(DataType::WikibaseItem));
        property.set_data_type(None);
        assert_eq!(property.data_type(), None);
    }

    #[test]
    fn test_serialize_data_type() {
        let mut property = Property {
            id: EntityId::property("P214"),
            data_type: Some(DataType::ExternalId),
            ..Default::default()
        };
        property
            .labels_mut()
            .insert(LanguageString::new("en", "label"));
        let j = serde_json::to_string(&property).unwrap();
        let v: Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["data_type"], "external-id");
    }

    #[test]
    fn test_data_type_roundtrip() {
        let v = json!({
            "id": "P214",
            "data_type": "external-id",
            "labels": {},
            "descriptions": {},
            "aliases": {},
            "statements": {},
        });
        let property = Property::from_json(v).unwrap();
        let j = serde_json::to_value(&property).unwrap();
        assert_eq!(j["data_type"], "external-id");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_item_post() {
        let j214 = std::fs::read_to_string("test_data/P214.json").unwrap();
        let v214: Value = serde_json::from_str(&j214).unwrap();
        let mut property = Property::from_json(v214).unwrap();
        let v = property.to_owned();

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/w/rest.php/wikibase/v1/entities/properties"))
            .and(body_partial_json(
                json!({"property": {"labels": {"en": property.labels().get_lang("en")}}}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        // Check that an error is returned when trying to post an item that already has an ID
        let r0 = property.post(&api).await;
        assert_eq!(r0.err().unwrap().to_string(), "ID already set");

        // Clear the ID and try again
        property.id = EntityId::None;
        let r1 = property.post(&api).await.unwrap();
        assert_eq!(r1.id(), v.id());
    }
}
