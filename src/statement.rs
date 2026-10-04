use crate::{
    patch_entry::PatchEntry,
    property_value::{PropertyType, PropertyValue},
    statement_patch::StatementPatch,
    statement_value::StatementValue,
    statement_value_content::{StatementValueContent, TimePrecision},
    DataType, EditMetadata, EntityId, FromJson, HeaderInfo, HttpMisc, Patch, Reference, RestApi,
    RestApiError, RevisionMatch, StatementRank,
};
use derive_where::DeriveWhere;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(DeriveWhere, Debug, Clone, Default, Serialize)]
#[derive_where(PartialEq)]
pub struct Statement {
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    statement_id: Option<String>,
    property: PropertyType,
    value: StatementValue,
    rank: StatementRank,
    references: Vec<Reference>,
    qualifiers: Vec<PropertyValue>,
    #[serde(skip)]
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl HttpMisc for Statement {
    /// `id` is the entity the statement belongs to; `EntityId::None` uses the
    /// entity-independent `/statements/{statement_id}` endpoint.
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        let statement_id = self.id().ok_or(RestApiError::MissingId)?;
        Self::rest_api_path(id, statement_id)
    }
}

// GET/PUT/POST/DELETE
//
// Every operation is available in two flavours that hit equivalent endpoints:
// - `get`/`put`/`delete`/… use `/statements/{statement_id}`;
// - `*_for_entity` use `/entities/{group}/{entity_id}/statements/{statement_id}`, where the
//   server additionally checks that the statement belongs to that entity.
impl Statement {
    /// Convenience function to create a new string statement
    pub fn new_string(property: &str, s: &str) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::String)),
            value: StatementValue::new_string(s),
            ..Default::default()
        }
    }

    /// Convenience function to create a new external ID statement
    pub fn new_external_id<S1: Into<String>, S2: Into<String>>(property: S1, s: S2) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::ExternalId)),
            value: StatementValue::new_string(s),
            ..Default::default()
        }
    }

    /// Convenience function to create a new URL statement
    pub fn new_url<S1: Into<String>, S2: Into<String>>(property: S1, s: S2) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::Url)),
            value: StatementValue::new_string(s),
            ..Default::default()
        }
    }

    /// Convenience function to create a new URL statement
    pub fn new_monolingual_text<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        property: S1,
        language: S2,
        text: S3,
    ) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::MonolingualText)),
            value: StatementValue::Value(StatementValueContent::new_monolingual_text(
                language, text,
            )),
            ..Default::default()
        }
    }

    /// Convenience function to create a new item statement
    /// (note that this does not check if the item ID is valid)
    pub fn new_item<S1: Into<String>, S2: Into<String>>(property: S1, item_id: S2) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::WikibaseItem)),
            value: StatementValue::new_string(item_id.into()),
            ..Default::default()
        }
    }

    /// Convenience function to create a new time statement
    pub fn new_time<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        property: S1,
        time: S2,
        precision: TimePrecision,
        calendarmodel: S3,
    ) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::Time)),
            value: StatementValue::Value(StatementValueContent::Time {
                time: time.into(),
                precision,
                calendarmodel: calendarmodel.into(),
            }),
            ..Default::default()
        }
    }

    /// Convenience function to create a new file statement
    pub fn new_file<S1: Into<String>, S2: Into<String>>(property: S1, filename: S2) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::CommonsMedia)),
            value: StatementValue::new_string(filename.into()),
            ..Default::default()
        }
    }

    // TODO more convenience functions

    /// Adds a single reference to the statement, returning the statement.
    /// Useful for constructing statements.
    pub fn with_reference(mut self, reference: Reference) -> Self {
        self.references.push(reference);
        self
    }

    /// Adds multiple references to the statement, returning the statement.
    /// Useful for constructing statements.
    pub fn with_references(mut self, references: Vec<Reference>) -> Self {
        self.references.extend(references);
        self
    }

    /// Adds a single qualifier to the statement, returning the statement.
    /// Useful for constructing statements.
    pub fn with_qualifier(mut self, qualifier: PropertyValue) -> Self {
        self.qualifiers.push(qualifier);
        self
    }

    /// Adds a single qualifier to the statement, returning the statement.
    /// Useful for constructing statements.
    pub fn with_qualifiers(mut self, qualifiers: Vec<PropertyValue>) -> Self {
        self.qualifiers.extend(qualifiers);
        self
    }

    /// Converts the statement into a `PropertyValue`.
    /// Destroys the `Statement`.
    /// Useful for creating `PropertyValue` using the new_* functions.
    pub fn as_property_value(self) -> PropertyValue {
        PropertyValue::new(self.property, self.value)
    }

    /// Generates a new statement ID
    pub fn new_id_for_entity(&mut self, entity_id: &EntityId) {
        let uuid = Uuid::new_v4().to_string().to_ascii_uppercase();
        self.set_id(Some(format!("{entity_id}${uuid}")));
    }

    /// Fetches a statement from the API
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wikibase_rest_api::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let api = RestApi::wikidata().unwrap();
    ///     let statement = Statement::get("Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9", &api).await.unwrap();
    ///     println!("{:?}", statement);
    /// }
    /// ```
    pub async fn get(statement_id: &str, api: &RestApi) -> Result<Self, RestApiError> {
        Self::get_match(statement_id, api, RevisionMatch::default()).await
    }

    /// Fetches a statement from the API with revision matching
    pub async fn get_match(
        statement_id: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        Self::get_match_for_entity(&EntityId::None, statement_id, api, rm).await
    }

    /// Fetches a statement of a specific entity from the API
    pub async fn get_for_entity(
        entity_id: &EntityId,
        statement_id: &str,
        api: &RestApi,
    ) -> Result<Self, RestApiError> {
        Self::get_match_for_entity(entity_id, statement_id, api, RevisionMatch::default()).await
    }

    /// Fetches a statement of a specific entity from the API with revision matching
    pub async fn get_match_for_entity(
        entity_id: &EntityId,
        statement_id: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let path = Self::rest_api_path(entity_id, statement_id)?;
        let (j, header_info) = Self::get_match_internal(api, &path, rm).await?;
        Self::from_json_header_info(&j, header_info)
    }

    /// Replaces an existing statement (identified by its `id`) via the API.
    ///
    /// Returns the statement as stored by the server. To *add* a new statement to an
    /// entity, use [`Statements::post`](crate::statements::Statements::post) instead.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wikibase_rest_api::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let api = RestApi::wikidata().unwrap(); // Use Wikidata API
    ///     let mut statement = Statement::get("Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9", &api).await.unwrap();
    ///     statement.set_rank(StatementRank::Preferred);
    ///     statement = statement.put(&api).await.unwrap(); // Replace the statement
    /// }
    /// ```
    pub async fn put(&self, api: &RestApi) -> Result<Self, RestApiError> {
        self.put_match(api, EditMetadata::default()).await
    }

    /// Replaces an existing statement via the API, with edit metadata.
    /// See [`put`](Self::put).
    pub async fn put_match(&self, api: &RestApi, em: EditMetadata) -> Result<Self, RestApiError> {
        self.put_match_for_entity(&EntityId::None, api, em).await
    }

    /// Replaces an existing statement of a specific entity via the API.
    /// See [`put`](Self::put).
    pub async fn put_for_entity(
        &self,
        entity_id: &EntityId,
        api: &RestApi,
    ) -> Result<Self, RestApiError> {
        self.put_match_for_entity(entity_id, api, EditMetadata::default())
            .await
    }

    /// Replaces an existing statement of a specific entity via the API, with edit metadata.
    /// See [`put`](Self::put).
    pub async fn put_match_for_entity(
        &self,
        entity_id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError> {
        let j = json!({"statement": self});
        let (j, header_info) = self
            .run_json_query(entity_id, reqwest::Method::PUT, j, api, &em)
            .await?;
        Self::from_json_header_info(&j, header_info)
    }

    /// Deletes a statement via the API
    pub async fn delete(&self, api: &RestApi) -> Result<(), RestApiError> {
        self.delete_match(api, EditMetadata::default()).await
    }

    /// Deletes a statement via the API with revision matching
    pub async fn delete_match(&self, api: &RestApi, em: EditMetadata) -> Result<(), RestApiError> {
        self.delete_match_for_entity(&EntityId::None, api, em).await
    }

    /// Deletes a statement of a specific entity via the API
    pub async fn delete_for_entity(
        &self,
        entity_id: &EntityId,
        api: &RestApi,
    ) -> Result<(), RestApiError> {
        self.delete_match_for_entity(entity_id, api, EditMetadata::default())
            .await
    }

    /// Deletes a statement of a specific entity via the API with revision matching
    pub async fn delete_match_for_entity(
        &self,
        entity_id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<(), RestApiError> {
        self.run_json_query(entity_id, reqwest::Method::DELETE, json!({}), api, &em)
            .await?;
        Ok(())
    }

    /// Sets the statement property
    pub fn set_property(&mut self, property: PropertyType) {
        self.property = property;
    }

    /// Sets the statement value
    pub fn set_value(&mut self, value: StatementValue) {
        self.value = value;
    }

    /// Sets the statement rank
    pub const fn set_rank(&mut self, rank: StatementRank) {
        self.rank = rank;
    }

    /// Returns the references of the statement, mutable
    pub const fn references_mut(&mut self) -> &mut Vec<Reference> {
        &mut self.references
    }

    /// Returns the qualifiers of the statement, mutable
    pub const fn qualifiers_mut(&mut self) -> &mut Vec<PropertyValue> {
        &mut self.qualifiers
    }

    /// The REST path of a statement: entity-scoped if `entity_id` is set, global otherwise.
    pub(crate) fn rest_api_path(
        entity_id: &EntityId,
        statement_id: &str,
    ) -> Result<String, RestApiError> {
        if entity_id.is_none() {
            Ok(format!("/statements/{statement_id}"))
        } else {
            Ok(format!(
                "{}/statements/{statement_id}",
                entity_id.entity_path()?
            ))
        }
    }
}

// FromJson helper function
impl Statement {
    fn generate_id_rank_from_json_header_info(
        j: &Value,
    ) -> Result<(String, StatementRank), RestApiError> {
        let id = j["id"]
            .as_str()
            .ok_or(RestApiError::MissingOrInvalidField {
                field: "id".into(),
                j: j.to_owned(),
            })?
            .to_string();
        let rank_text = j["rank"]
            .as_str()
            .ok_or(RestApiError::MissingOrInvalidField {
                field: "rank".into(),
                j: j.to_owned(),
            })?;
        let rank = StatementRank::new(rank_text)?;
        Ok((id, rank))
    }
}

impl FromJson for Statement {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let (id, rank) = Self::generate_id_rank_from_json_header_info(j)?;
        let property = PropertyType::from_json(&j["property"])?;
        let value = StatementValue::from_json(&j["value"])?;
        Ok(Statement {
            statement_id: Some(id.to_string()),
            property,
            rank,
            value,
            references: Self::references_from_json(&j["references"])?,
            qualifiers: Self::qualifiers_from_json(&j["qualifiers"])?,
            header_info,
        })
    }
}

// The rest
impl Statement {
    /// Generates a patch to transform `other` into `self`
    pub fn patch(&self, other: &Self) -> Result<StatementPatch, RestApiError> {
        let statement_id = match self.statement_id {
            Some(ref id) => id,
            None => return Err(RestApiError::MissingId),
        };
        let mut patch = StatementPatch::new(statement_id);
        *patch.patch_mut() = PatchEntry::diff(other, self, "StatementPatch")?;
        Ok(patch)
    }

    fn references_from_json(j: &Value) -> Result<Vec<Reference>, RestApiError> {
        Self::array_from_json(j, "references", Reference::from_json)
    }

    fn qualifiers_from_json(j: &Value) -> Result<Vec<PropertyValue>, RestApiError> {
        Self::array_from_json(j, "qualifiers", PropertyValue::from_json)
    }

    fn array_from_json<T>(
        j: &Value,
        field: &str,
        parse: fn(&Value) -> Result<T, RestApiError>,
    ) -> Result<Vec<T>, RestApiError> {
        j.as_array()
            .ok_or_else(|| RestApiError::WrongType {
                field: field.into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(parse)
            .collect()
    }

    /// Returns the statement ID
    pub fn id(&self) -> Option<&str> {
        self.statement_id.as_deref()
    }

    /// Sets the statement ID
    pub fn set_id(&mut self, id: Option<String>) {
        self.statement_id = id;
    }

    /// Returns the statement property
    pub const fn property(&self) -> &PropertyType {
        &self.property
    }

    /// Returns the statement value
    pub const fn value(&self) -> &StatementValue {
        &self.value
    }

    /// Returns the statement rank
    pub const fn rank(&self) -> &StatementRank {
        &self.rank
    }

    /// Returns the references of the statement
    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    /// Returns the qualifiers of the statement
    pub fn qualifiers(&self) -> &[PropertyValue] {
        &self.qualifiers
    }

    /// Checks if this statement has the same qualifiers as another statement,
    /// regardless of their order.
    pub fn same_qualifiers_as(&self, other: &Statement) -> bool {
        let contains_all =
            |a: &[PropertyValue], b: &[PropertyValue]| a.iter().all(|q| b.contains(q));
        self.qualifiers.len() == other.qualifiers.len()
            && contains_all(&self.qualifiers, &other.qualifiers)
            && contains_all(&other.qualifiers, &self.qualifiers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::statement_value_content::StatementValueContent;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statement_get() {
        // #lizard forgives the complexity
        let v = std::fs::read_to_string("test_data/test_statement_get.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let statement_id = v["id"].as_str().unwrap().to_string();
        let mock_path = format!("/w/rest.php/wikibase/v1/statements/{statement_id}",);

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(v))
            .mount(&mock_server)
            .await;

        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();
        let statement = Statement::get(&statement_id, &api).await.unwrap();
        assert_eq!(statement.id().unwrap(), &statement_id);
        assert_eq!(
            *statement.value(),
            StatementValue::Value(StatementValueContent::String("Q42".to_string()))
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statement_put() {
        // #lizard forgives the complexity
        let v = std::fs::read_to_string("test_data/test_statement_put.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let statement_id = v["before"]["id"].as_str().unwrap();
        let mock_path = format!("/w/rest.php/wikibase/v1/statements/{statement_id}");
        let mock_value_before = StatementValue::Value(StatementValueContent::String(
            v["before"]["value"]["content"]
                .as_str()
                .unwrap()
                .to_string(),
        ));
        let mock_value_after = StatementValue::Value(StatementValueContent::String(
            v["after"]["value"]["content"].as_str().unwrap().to_string(),
        ));

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["before"]))
            .mount(&mock_server)
            .await;
        Mock::given(body_partial_json(json!({"statement": &v["after"]})))
            .and(method("PUT"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["after"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        // Get and check statement
        let mut statement = Statement::get(statement_id, &api).await.unwrap();
        assert_eq!(*statement.value(), mock_value_before);

        // Change statement
        statement.value = mock_value_after.to_owned();

        // PUT, and check return value
        let statement = statement.put(&api).await.unwrap();
        assert_eq!(*statement.value(), mock_value_after);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statement_delete() {
        // #lizard forgives the complexity
        let statement_id = "Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9";
        let mock_path = format!("/w/rest.php/wikibase/v1/statements/{statement_id}");

        let statement_id2 = "no_such_statement";
        let mock_path2 = format!("/w/rest.php/wikibase/v1/statements/{statement_id2}");

        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json("Statement deleted"))
            .mount(&mock_server)
            .await;
        Mock::given(method("DELETE"))
            .and(path(&mock_path2))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
              "code": "statement-not-found",
              "message": format!("Could not find a statement with the ID: {statement_id2}")
            })))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        // Delete
        let mut statement0 = Statement::new_string("P31", "Q42");
        statement0.set_id(Some(statement_id.to_string()));
        assert!(statement0.delete(&api).await.is_ok());

        // Delete (error)
        let mut statement1 = Statement::new_string("P31", "Q42");
        statement1.set_id(Some(statement_id2.to_string()));
        assert!(statement1.delete(&api).await.is_err());
    }

    #[test]
    fn test_patch() {
        let mut s1 = Statement::default();
        s1.set_id(Some("Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9".to_string()));
        s1.set_property(PropertyType::property("P31"));
        s1.set_value(StatementValue::new_string("Q42"));
        let mut s2 = s1.clone();
        s2.set_property(PropertyType::property("P32"));
        s2.set_value(StatementValue::new_string("Q43"));
        let patch = s2.patch(&s1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"patch":[{"op":"replace","path":"/property/id","value":"P32"},{"op":"replace","path":"/value/content","value":"Q43"}],"statement_id":"Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9"})
        );
    }

    #[test]
    fn test_set_rank() {
        let mut s = Statement::default();
        assert_eq!(s.rank(), &StatementRank::Normal);
        s.set_rank(StatementRank::Preferred);
        assert_eq!(s.rank(), &StatementRank::Preferred);
    }

    #[test]
    fn test_references_mut() {
        let mut s = Statement::default();
        assert_eq!(s.references_mut().len(), 0);
        s.references_mut().push(Reference::default());
        assert_eq!(s.references_mut().len(), 1);
    }

    #[test]
    fn test_qualifiers_mut() {
        let mut s = Statement::default();
        assert_eq!(s.qualifiers_mut().len(), 0);
        s.qualifiers_mut().push(PropertyValue::new(
            PropertyType::property("P31"),
            StatementValue::new_string("Q42"),
        ));
        assert_eq!(s.qualifiers_mut().len(), 1);
    }

    #[test]
    fn test_rank() {
        let s = Statement::default();
        assert_eq!(s.rank(), &StatementRank::Normal);
    }

    #[test]
    fn test_references() {
        let s = Statement::default();
        assert_eq!(s.references().len(), 0);
    }

    #[test]
    fn test_patch_no_id() {
        let s1 = Statement::default();
        let s2 = Statement::default();
        let patch = s2.patch(&s1);
        assert!(patch.is_err());
    }

    #[test]
    fn test_references_from_json_not_array() {
        let j = json!(123);
        assert!(Statement::references_from_json(&j).is_err());
    }

    #[test]
    fn test_references_from_json_not_a_reference() {
        let j = json!([123]);
        assert!(Statement::references_from_json(&j).is_err());
    }

    #[test]
    fn test_references_from_json() {
        let j = json!([
            Reference::default(),
            Reference::default(),
            Reference::default()
        ]);
        let references = Statement::references_from_json(&j).unwrap();
        assert_eq!(references.len(), 3);
    }

    #[test]
    fn test_new_id_for_entity() {
        let entity_id = EntityId::new("Q42").unwrap();
        let mut statement = Statement::default();
        statement.new_id_for_entity(&entity_id);
        assert_eq!(&statement.id().unwrap()[0..4], "Q42$");
    }

    #[test]
    fn test_new_external_id() {
        let s = Statement::new_external_id("P214", "12345");
        assert_eq!(s.property().datatype(), Some(&DataType::ExternalId));
        assert_eq!(s.value(), &StatementValue::new_string("12345"));
    }

    #[test]
    fn test_new_url() {
        let s = Statement::new_url("P856", "https://example.org");
        assert_eq!(s.property().datatype(), Some(&DataType::Url));
        assert_eq!(
            s.value(),
            &StatementValue::new_string("https://example.org")
        );
    }

    #[test]
    fn test_new_monolingual_text() {
        let s = Statement::new_monolingual_text("P1476", "en", "Hello");
        assert_eq!(s.property().datatype(), Some(&DataType::MonolingualText));
        assert_eq!(
            s.value(),
            &StatementValue::Value(StatementValueContent::new_monolingual_text("en", "Hello"))
        );
    }

    #[test]
    fn test_new_item() {
        let s = Statement::new_item("P31", "Q42");
        assert_eq!(s.property().datatype(), Some(&DataType::WikibaseItem));
        assert_eq!(json!(s)["property"]["data_type"], "wikibase-item");
        assert_eq!(s.value(), &StatementValue::new_string("Q42"));
    }

    #[test]
    fn test_new_time() {
        let s = Statement::new_time(
            "P569",
            "+2001-12-31T00:00:00Z",
            TimePrecision::Day,
            "http://www.wikidata.org/entity/Q1985727",
        );
        assert_eq!(s.property().datatype(), Some(&DataType::Time));
        assert_eq!(
            s.value(),
            &StatementValue::Value(StatementValueContent::Time {
                time: "+2001-12-31T00:00:00Z".to_string(),
                precision: TimePrecision::Day,
                calendarmodel: "http://www.wikidata.org/entity/Q1985727".to_string(),
            })
        );
    }

    #[test]
    fn test_new_file() {
        let s = Statement::new_file("P18", "Example.jpg");
        assert_eq!(s.property().datatype(), Some(&DataType::CommonsMedia));
        assert_eq!(s.value(), &StatementValue::new_string("Example.jpg"));
    }

    #[test]
    fn test_with_reference_and_references() {
        let s = Statement::new_string("P31", "Q42")
            .with_reference(Reference::default())
            .with_references(vec![Reference::default(), Reference::default()]);
        assert_eq!(s.references().len(), 3);
    }

    #[test]
    fn test_with_qualifier_and_qualifiers() {
        let qualifier = PropertyValue::new(
            PropertyType::property("P580"),
            StatementValue::new_string("Q42"),
        );
        let s = Statement::new_string("P31", "Q42")
            .with_qualifier(qualifier.clone())
            .with_qualifiers(vec![qualifier.clone(), qualifier]);
        assert_eq!(s.qualifiers().len(), 3);
    }

    #[test]
    fn test_as_property_value() {
        let s = Statement::new_string("P31", "Q42");
        let pv = s.as_property_value();
        assert_eq!(
            pv.property(),
            &PropertyType::new("P31", Some(DataType::String))
        );
        assert_eq!(pv.value(), &StatementValue::new_string("Q42"));
    }

    #[test]
    fn test_same_qualifiers_as() {
        let qualifier = PropertyValue::new(
            PropertyType::property("P580"),
            StatementValue::new_string("Q42"),
        );
        let s1 = Statement::new_string("P31", "Q42").with_qualifier(qualifier.clone());
        let s2 = Statement::new_string("P31", "Q42").with_qualifier(qualifier);
        let s3 = Statement::new_string("P31", "Q42");
        assert!(s1.same_qualifiers_as(&s2));
        assert!(!s1.same_qualifiers_as(&s3));
        // Symmetric: a statement without qualifiers is not "the same" as one with them.
        assert!(!s3.same_qualifiers_as(&s1));
    }

    #[test]
    fn test_same_qualifiers_as_ignores_order() {
        let q1 = PropertyValue::new(
            PropertyType::property("P1"),
            StatementValue::new_string("a"),
        );
        let q2 = PropertyValue::new(
            PropertyType::property("P2"),
            StatementValue::new_string("b"),
        );
        let s1 = Statement::default().with_qualifiers(vec![q1.clone(), q2.clone()]);
        let s2 = Statement::default().with_qualifiers(vec![q2, q1]);
        assert!(s1.same_qualifiers_as(&s2));
    }

    #[test]
    fn test_rest_api_path() {
        let sid = "Q42$ABC";
        assert_eq!(
            Statement::rest_api_path(&EntityId::None, sid).unwrap(),
            "/statements/Q42$ABC"
        );
        assert_eq!(
            Statement::rest_api_path(&EntityId::item("Q42"), sid).unwrap(),
            "/entities/items/Q42/statements/Q42$ABC"
        );
        assert_eq!(
            Statement::rest_api_path(&EntityId::property("P1"), "P1$X").unwrap(),
            "/entities/properties/P1/statements/P1$X"
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statement_for_entity_endpoints() {
        // get/put/delete via /entities/items/{id}/statements/{statement_id}
        let v = std::fs::read_to_string("test_data/test_statement_get.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let statement_id = v["id"].as_str().unwrap().to_string();
        let entity_id = EntityId::item(statement_id.split('$').next().unwrap());
        let mock_path =
            format!("/w/rest.php/wikibase/v1/entities/items/{entity_id}/statements/{statement_id}");
        let mock_server = MockServer::start().await;
        for verb in ["GET", "PUT"] {
            Mock::given(method(verb))
                .and(path(&mock_path))
                .respond_with(ResponseTemplate::new(200).set_body_json(&v))
                .expect(1)
                .mount(&mock_server)
                .await;
        }
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json("Statement deleted"))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let statement = Statement::get_for_entity(&entity_id, &statement_id, &api)
            .await
            .unwrap();
        assert_eq!(statement.id(), Some(statement_id.as_str()));
        let statement = statement.put_for_entity(&entity_id, &api).await.unwrap();
        statement.delete_for_entity(&entity_id, &api).await.unwrap();
    }

    #[test]
    fn test_serialize_with_id() {
        let mut s = Statement::new_string("P31", "Q42");
        s.set_id(Some("Q42$abc".to_string()));
        let v = json!(s);
        assert_eq!(v["id"], "Q42$abc");
        assert_eq!(v["property"]["id"], "P31");
    }
}
