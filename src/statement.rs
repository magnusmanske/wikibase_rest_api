use crate::{
    property_value::{PropertyType, PropertyValue},
    statement_patch::StatementPatch,
    statement_value::StatementValue,
    EditMetadata, EntityId, FromJson, HeaderInfo, HttpMisc, Reference, RestApi, RestApiError,
    RevisionMatch, StatementRank,
};
use derivative::Derivative;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Derivative, Debug, Clone, Default)]
#[derivative(PartialEq)]
pub struct Statement {
    id: Option<String>,
    property: PropertyType,
    value: StatementValue,
    rank: StatementRank,
    references: Vec<Reference>,
    qualifiers: Vec<PropertyValue>,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl Serialize for Statement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // #lizard forgives the complexity
        let mut fields = 5;
        if self.id.is_some() {
            fields += 1;
        }
        let mut s = serializer.serialize_struct("Statement", fields)?;
        if let Some(id) = &self.id {
            s.serialize_field("id", &id)?
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
    fn get_rest_api_path(&self, _id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/statements/{id}",
            id = self.id().ok_or(RestApiError::MissingId)?
        ))
    }
}

// GET/PUT/POST/DELETE
impl Statement {
    pub fn new_string(property: &str, s: &str) -> Self {
        Self {
            id: None,
            property: PropertyType::property(property),
            value: StatementValue::new_string(s),
            rank: StatementRank::Normal,
            references: vec![],
            qualifiers: vec![],
            header_info: HeaderInfo::default(),
        }
    }

    /// Generates a new statement ID
    pub fn new_id_for_entity(entity_id: &EntityId) -> String {
        let uuid = Uuid::new_v4();
        let uuid = uuid.to_string().to_ascii_uppercase();
        format!("{entity_id}${uuid}")
    }

    /// Fetches a statement from the API
    ///
    /// Usage Example:
    /// ```text
    /// use wikibase::statement::Statement;
    /// use wikibase::RestApi;
    /// #[tokio::main]
    /// async fn main() {
    ///     let api = RestApi::builder().api("https://www.wikidata.org/w/rest.php").build().unwrap();
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
    /// Usage Example:
    /// ```text
    /// use wikibase::statement::Statement;
    /// use wikibase::RestApi;
    /// #[tokio::main]
    /// async fn main() {
    ///     let api = RestApi::builder().api("https://www.wikidata.org/w/rest.php").build().unwrap();
    ///     let mut statement = Statement::new_string("P31", "Q42");
    ///     statement = statement.put_match(&mut api, EditMetadata::default()).await.unwrap();
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
        let path = format!("/statements/{statement_id}");
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut());
        let response = api.execute(request).await?;
        let header_info = HeaderInfo::from_header(response.headers());
        let j: Value = response.error_for_status()?.json().await?;
        Self::from_json_header_info(&j, header_info)
    }

    /// Creates a new statement via the API with revision matching.
    pub async fn put_match(
        &self,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError> {
        let j = json!({"statement": self});
        let request = self
            .generate_json_request(&EntityId::None, reqwest::Method::PUT, j, api, &em)
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
        let j = json!({});
        let request = self
            .generate_json_request(&EntityId::None, reqwest::Method::DELETE, j, api, &em)
            .await?;
        let j: Value = api
            .execute(request)
            .await?
            .error_for_status()?
            .json()
            .await?;
        if j == "Statement deleted" {
            return Ok(());
        }
        Err(RestApiError::UnexpectedResponse(j))
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
    pub fn set_rank(&mut self, rank: StatementRank) {
        self.rank = rank;
    }

    /// Returns the references of the statement, mutable
    pub fn references_mut(&mut self) -> &mut Vec<Reference> {
        &mut self.references
    }

    /// Returns the qualifiers of the statement, mutable
    pub fn qualifiers_mut(&mut self) -> &mut Vec<PropertyValue> {
        &mut self.qualifiers
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
        let rank = StatementRank::from_str(rank_text)?;
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
            id: Some(id.to_string()),
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
        let statement_id = match self.id {
            Some(ref id) => id,
            None => return Err(RestApiError::MissingId),
        };
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = StatementPatch::from_json(statement_id, &json!(patch))?;
        return Ok(patch);
    }

    fn references_from_json(j: &Value) -> Result<Vec<Reference>, RestApiError> {
        let mut ret = vec![];
        let array = j.as_array().ok_or(RestApiError::WrongType {
            field: "references".into(),
            j: j.to_owned(),
        })?;
        for reference in array {
            ret.push(Reference::from_json(reference)?);
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
    pub fn id(&self) -> Option<&String> {
        self.id.as_ref()
    }

    /// Sets the statement ID
    pub fn set_id(&mut self, id: Option<String>) {
        self.id = id;
    }

    /// Returns the statement property
    pub fn property(&self) -> &PropertyType {
        &self.property
    }

    /// Returns the statement value
    pub fn value(&self) -> &StatementValue {
        &self.value
    }

    /// Returns the statement rank
    pub fn rank(&self) -> &StatementRank {
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
    use crate::statement_value::StatementValueContent;
    use wiremock::matchers::{body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_statement_get() {
        // #lizard forgives the complexity
        let v = std::fs::read_to_string("test_data/test_statement_get.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let statement_id = v["id"].as_str().unwrap().to_string();
        let mock_path = format!("/w/rest.php/wikibase/v0/statements/{statement_id}",);

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(v))
            .mount(&mock_server)
            .await;

        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
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
    async fn test_statement_put() {
        // #lizard forgives the complexity
        let v = std::fs::read_to_string("test_data/test_statement_put.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let statement_id = v["before"]["id"].as_str().unwrap();
        let mock_path = format!("/w/rest.php/wikibase/v0/statements/{statement_id}");
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
        let mut api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

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
    async fn test_statement_delete() {
        // #lizard forgives the complexity
        let statement_id = "Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9";
        let mock_path = format!("/w/rest.php/wikibase/v0/statements/{statement_id}");

        let statement_id2 = "no_such_statement";
        let mock_path2 = format!("/w/rest.php/wikibase/v0/statements/{statement_id2}");

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
        let mut api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        // Delete
        let mut statement = Statement::new_string("P31", "Q42");
        statement.set_id(Some(statement_id.to_string()));
        assert!(!statement.delete(&mut api).await.is_err());

        // Delete (error)
        let mut statement = Statement::new_string("P31", "Q42");
        statement.set_id(Some(statement_id2.to_string()));
        let result = statement.delete(&mut api).await.unwrap_err().to_string();
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
        let id = Statement::new_id_for_entity(&entity_id);
        assert_eq!(&id[0..4], "Q42$");
    }
}
