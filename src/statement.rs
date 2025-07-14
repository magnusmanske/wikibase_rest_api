use crate::{
    property_value::{PropertyType, PropertyValue},
    statement_patch::StatementPatch,
    statement_value::StatementValue,
    statement_value_content::{StatementValueContent, TimePrecision},
    DataType, EditMetadata, EntityId, FromJson, HeaderInfo, HttpMisc, Reference, RestApi,
    RestApiError, RevisionMatch, StatementRank,
};
use derive_where::DeriveWhere;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(DeriveWhere, Debug, Clone, Default)]
#[derive_where(PartialEq)]
pub struct Statement {
    statement_id: Option<String>,
    property: PropertyType,
    value: StatementValue,
    rank: StatementRank,
    references: Vec<Reference>,
    qualifiers: Vec<PropertyValue>,
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl Serialize for Statement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // #lizard forgives the complexity
        let mut fields = 5;
        if self.statement_id.is_some() {
            fields += 1;
        }
        let mut s = serializer.serialize_struct("Statement", fields)?;
        if let Some(id) = &self.statement_id {
            s.serialize_field("id", &id)?;
        }
        s.serialize_field("property", &self.property)?;
        s.serialize_field("value", &self.value)?;
        s.serialize_field("rank", &self.rank.as_str())?;
        s.serialize_field("references", &self.references)?;
        s.serialize_field("qualifiers", &self.qualifiers)?;
        s.end()
    }
}

impl HttpMisc for Statement {
    fn get_my_rest_api_path(&self, _id: &EntityId) -> Result<String, RestApiError> {
        let id = self.id().ok_or(RestApiError::MissingId)?;
        Self::get_rest_api_path_from_id(id)
    }
}

// GET/PUT/POST/DELETE
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
    pub fn new_external_id(property: &str, s: &str) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::ExternalId)),
            value: StatementValue::new_string(s),
            ..Default::default()
        }
    }

    /// Convenience function to create a new URL statement
    pub fn new_url(property: &str, s: &str) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::Url)),
            value: StatementValue::new_string(s),
            ..Default::default()
        }
    }

    /// Convenience function to create a new URL statement
    pub fn new_monolingual_text(property: &str, language: &str, s: &str) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::MonolingualText)),
            value: StatementValue::Value(StatementValueContent::MonolingualText {
                language: language.to_string(),
                text: s.to_string(),
            }),

            ..Default::default()
        }
    }

    /// Convenience function to create a new item statement
    /// (note that this does not check if the item ID is valid)
    pub fn new_item(property: &str, item_id: &str) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::Item)),
            value: StatementValue::new_string(item_id),
            ..Default::default()
        }
    }

    /// Convenience function to create a new time statement
    pub fn new_time(
        property: &str,
        time: &str,
        precision: TimePrecision,
        calendarmodel: &str,
    ) -> Self {
        Self {
            property: PropertyType::new(property, Some(DataType::Time)),
            value: StatementValue::Value(StatementValueContent::Time {
                time: time.to_string(),
                precision,
                calendarmodel: calendarmodel.to_string(),
            }),
            ..Default::default()
        }
    }

    // TODO more convenience functions

    pub fn with_reference(mut self, reference: Reference) -> Self {
        self.references.push(reference);
        self
    }

    pub fn with_references(mut self, references: Vec<Reference>) -> Self {
        self.references.extend(references);
        self
    }

    /// Converts the statement into a `PropertyValue`.
    /// Destroys the `Statement`.
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

    /// Creates a new statement via the API. And `id` needs to be set.
    ///
    /// Returns a `Statement`, which is **not** the same as the input `Statement`, but should be identical.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wikibase_rest_api::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut api = RestApi::wikidata().unwrap(); // Use Wikidata API
    ///     let mut statement = Statement::new_string("P31", "Q42"); // New statement
    ///     statement.new_id_for_entity(&EntityId::new("Q13406268").unwrap()); // New statement ID for entity
    ///     statement = statement.put(&mut api).await.unwrap(); // Add statement to entity
    /// }
    pub async fn put(&self, api: &mut RestApi) -> Result<Self, RestApiError> {
        self.put_match(api, EditMetadata::default()).await
    }

    /// Deletes a statement via the API
    pub async fn delete(&self, api: &mut RestApi) -> Result<(), RestApiError> {
        self.delete_match(api, EditMetadata::default()).await
    }

    /// Fetches a statement from the API with revision matching
    pub async fn get_match(
        statement_id: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let path = Self::get_rest_api_path_from_id(statement_id)?;
        let (j, header_info) = Self::get_match_internal(api, &path, rm).await?;
        Self::from_json_header_info(&j, header_info)
    }

    /// Creates a new statement via the API. And `id` needs to be set.
    ///
    /// Returns a `Statement`, which is **not** the same as the input `Statement`, but should be identical.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use wikibase_rest_api::prelude::*;
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut api = RestApi::wikidata().unwrap(); // Use Wikidata API
    ///     let mut statement = Statement::new_string("P31", "Q42"); // New statement
    ///     statement.new_id_for_entity(&EntityId::new("Q13406268").unwrap()); // New statement ID for entity
    ///     statement = statement.put_match(&mut api, EditMetadata::default()).await.unwrap();
    /// }
    pub async fn put_match(
        &self,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError> {
        let j0 = json!({"statement": self});
        let request = self
            .generate_json_request(&EntityId::None, reqwest::Method::PUT, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let header_info = HeaderInfo::from_header(response.headers());
        let j: Value = response.error_for_status()?.json().await?;
        Self::from_json_header_info(&j, header_info)
    }

    /// Deletes a statement via the API with revision matching
    pub async fn delete_match(
        &self,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<(), RestApiError> {
        let j0 = json!({});
        let request = self
            .generate_json_request(&EntityId::None, reqwest::Method::DELETE, j0, api, &em)
            .await?;
        let message: Value = api
            .execute(request)
            .await?
            .error_for_status()?
            .json()
            .await?;
        if message == "Statement deleted" {
            return Ok(());
        }
        Err(RestApiError::UnexpectedResponse(message))
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

    fn get_rest_api_path_from_id(id: &str) -> Result<String, RestApiError> {
        Ok(format!("/statements/{id}"))
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
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = StatementPatch::from_json(statement_id, &json!(patch))?;
        Ok(patch)
    }

    fn references_from_json(j: &Value) -> Result<Vec<Reference>, RestApiError> {
        let mut ret = vec![];
        let array = j.as_array().ok_or(RestApiError::WrongType {
            field: "references".into(),
            j: j.to_owned(),
        })?;
        for reference in array {
            let ref_from_json = Reference::from_json(reference)?;
            ret.push(ref_from_json);
        }
        Ok(ret)
    }

    fn qualifiers_from_json(j: &Value) -> Result<Vec<PropertyValue>, RestApiError> {
        let array = j.as_array().ok_or(RestApiError::WrongType {
            field: "qualifiers".into(),
            j: j.to_owned(),
        })?;
        let mut ret = vec![];
        for pv in array.iter() {
            let property = PropertyType::from_json(&pv["property"])?;
            let value = StatementValue::from_json(&pv["value"])?;
            ret.push(PropertyValue::new(property, value));
        }
        Ok(ret)
    }

    /// Returns the statement ID
    pub const fn id(&self) -> Option<&String> {
        self.statement_id.as_ref()
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
            .build();
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
        let mut api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        // Get and check statement
        let mut statement = Statement::get(statement_id, &api).await.unwrap();
        assert_eq!(*statement.value(), mock_value_before);

        // Change statement
        statement.value = mock_value_after.to_owned();

        // PUT, and check return value
        let statement = statement.put(&mut api).await.unwrap();
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
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
              "code": "invalid-statement-id",
              "message": format!("Not a valid statement ID: {statement_id2}")
            })))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        // Delete
        let mut statement0 = Statement::new_string("P31", "Q42");
        statement0.set_id(Some(statement_id.to_string()));
        assert!(statement0.delete(&mut api).await.is_ok());

        // Delete (error)
        let mut statement1 = Statement::new_string("P31", "Q42");
        statement1.set_id(Some(statement_id2.to_string()));
        let result = statement1.delete(&mut api).await.unwrap_err().to_string();
        assert_eq!(
            result,
            r#"Unexpected response: {"code":"invalid-statement-id","message":"Not a valid statement ID: no_such_statement"}"#
        );
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
}
