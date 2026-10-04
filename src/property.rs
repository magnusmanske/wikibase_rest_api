use crate::{
    aliases::Aliases,
    aliases_in_language::AliasesInLanguage,
    descriptions::Descriptions,
    entity::{Entity, EntityType},
    entity_patch::PropertyPatch,
    labels::Labels,
    statements::Statements,
    DataType, EntityId, FromJson, HeaderInfo, HttpMisc, RestApiError,
};
use derive_where::DeriveWhere;
use serde::Serialize;
use serde_json::Value;

#[derive(DeriveWhere, Debug, Clone, Default, Serialize)]
#[derive_where(PartialEq)]
pub struct Property {
    #[serde(skip_serializing_if = "EntityId::is_none")]
    id: EntityId,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_type: Option<DataType>,
    #[serde(skip_serializing_if = "Labels::is_empty")]
    labels: Labels,
    #[serde(skip_serializing_if = "Descriptions::is_empty")]
    descriptions: Descriptions,
    #[serde(skip_serializing_if = "Aliases::is_empty")]
    aliases: Aliases,
    #[serde(skip_serializing_if = "Statements::is_empty")]
    statements: Statements,
    #[serde(skip)]
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl HttpMisc for Property {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        id.entity_path()
    }
}

impl FromJson for Property {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let id = j["id"]
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "id".to_string(),
                j: j.clone(),
            })?;
        Ok(Self {
            id: EntityId::property(id),
            data_type: j["data_type"].as_str().map(DataType::new),
            labels: Labels::from_json_or_default(&j["labels"])?,
            descriptions: Descriptions::from_json_or_default(&j["descriptions"])?,
            aliases: Aliases::from_json_or_default(&j["aliases"])?,
            statements: Statements::from_json_or_default(&j["statements"])?,
            header_info,
        })
    }
}

impl Entity for Property {
    const ENTITY_TYPE: EntityType = EntityType::Property;

    fn id(&self) -> &EntityId {
        &self.id
    }

    fn set_id(&mut self, id: EntityId) {
        self.id = id;
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

    /// Returns the aliases of the property for a specific language, as an `AliasesInLanguage` object
    pub fn as_aliases<S: Into<String>>(&self, lang: S) -> AliasesInLanguage {
        self.aliases.in_language(lang)
    }

    /// Returns the data type of the property
    pub const fn data_type(&self) -> Option<&DataType> {
        self.data_type.as_ref()
    }

    /// Sets the data type of the property
    pub fn set_data_type(&mut self, data_type: Option<DataType>) {
        self.data_type = data_type;
    }

    /// Generates a patch to transform `other` into `self`.
    ///
    /// # Errors
    /// Returns an error if a statement in `other` has no ID (it can't be addressed).
    pub fn patch(&self, other: &Self) -> Result<PropertyPatch, RestApiError> {
        Ok(PropertyPatch::default()
            .with_part("/labels", self.labels.patch(&other.labels)?)
            .with_part(
                "/descriptions",
                self.descriptions.patch(&other.descriptions)?,
            )
            .with_part("/aliases", self.aliases.patch(&other.aliases)?)
            // Statement patch paths are already entity-level (`/statements/...`).
            .with_part("", self.statements.patch(&other.statements)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_strings::LanguageStrings;
    use crate::{LanguageString, Patch, RestApi, Statement};
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
            .build()
            .unwrap();

        let property = Property::get(&EntityId::property("P214"), &api)
            .await
            .unwrap();
        assert_eq!(property.data_type(), Some(&DataType::ExternalId));
        let j = serde_json::to_string(&property).unwrap(); // Convert property to JSON text
        let v: Value = serde_json::from_str(&j).unwrap(); // Convert to JSON value
        let property_from_json = Property::from_json(&v).unwrap(); // Convert back to property
        assert_eq!(property, property_from_json); // Check if the reconstituted property is identical to the original
    }

    #[test]
    fn test_id() {
        let id = EntityId::property("P214");
        let property = Property {
            id: id.to_owned(),
            ..Default::default()
        };
        assert_eq!(property.id(), &id);
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
    fn test_unknown_data_type_round_trips() {
        let v = json!({"id": "P1", "data_type": "edtf"});
        let property = Property::from_json(&v).unwrap();
        assert_eq!(property.data_type(), Some(&DataType::Other("edtf".into())));
        assert_eq!(serde_json::to_value(&property).unwrap(), v);
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
        let property = Property::from_json(&v).unwrap();
        assert_eq!(property.id(), &EntityId::property("P214"));
        assert_eq!(property.data_type(), Some(&DataType::ExternalId));
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
        assert_eq!(property.data_type(), Some(&DataType::WikibaseItem));
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
        let property = Property::from_json(&v).unwrap();
        let j = serde_json::to_value(&property).unwrap();
        assert_eq!(j["data_type"], "external-id");
    }

    #[test]
    fn test_get_rest_api_path() {
        let property = Property::default();
        let id = EntityId::property("P214");
        assert_eq!(
            property.get_my_rest_api_path(&id).unwrap(),
            "/entities/properties/P214"
        );
    }

    #[test]
    fn test_set_id() {
        let mut property = Property::default();
        assert!(property.id().is_none());
        property.set_id(EntityId::property("P214"));
        assert_eq!(property.id(), &EntityId::property("P214"));
    }

    #[test]
    fn test_patch() {
        let mut p1 = Property::default();
        let mut p2 = Property::default();
        p1.labels_mut().insert(LanguageString::new("en", "label"));
        p2.labels_mut().insert(LanguageString::new("en", "label2"));
        let patch = p1.patch(&p2).unwrap();
        assert_eq!(patch.patch().len(), 1);
        assert_eq!(patch.patch()[0].path(), "/labels/en");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_item_post() {
        let j214 = std::fs::read_to_string("test_data/P214.json").unwrap();
        let v214: Value = serde_json::from_str(&j214).unwrap();
        let mut property = Property::from_json(&v214).unwrap();
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
            .build()
            .unwrap();

        // Check that an error is returned when trying to post an item that already has an ID
        let r0 = property.post(&api).await;
        assert_eq!(r0.err().unwrap().to_string(), "ID already set");

        // Clear the ID and try again
        property.id = EntityId::None;
        let r1 = property.post(&api).await.unwrap();
        assert_eq!(r1.id(), v.id());
    }
}
