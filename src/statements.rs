use crate::{
    statements_patch::StatementsPatch, EditMetadata, EntityId, FromJson, HeaderInfo, HttpGetEntity,
    HttpMisc, Patch, RestApi, RestApiError, RevisionMatch, Statement,
};
use async_trait::async_trait;
use derive_where::DeriveWhere;
use serde::ser::{Serialize, SerializeMap};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(DeriveWhere, Debug, Clone, Default)]
#[derive_where(PartialEq)]
pub struct Statements {
    statements: HashMap<String, Vec<Statement>>, // property => Statements
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl Statements {
    /// Creates a new `Statements` object from a JSON structure
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        Self::from_json_header_info(j, HeaderInfo::default())
    }

    /// Creates a new `Statements` object from a JSON structure with header info
    pub fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let mut ret = Self::default();
        let statements_j = j
            .as_object()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "Statements".into(),
                j: j.to_owned(),
            })?;
        for (property, statements) in statements_j {
            let statements =
                statements
                    .as_array()
                    .ok_or_else(|| RestApiError::MissingOrInvalidField {
                        field: property.into(),
                        j: json!(statements),
                    })?;
            let statements = statements
                .iter()
                .map(Statement::from_json)
                .collect::<Result<Vec<Statement>, RestApiError>>()?;
            ret.statements.insert(property.to_owned(), statements);
        }
        ret.header_info = header_info;
        Ok(ret)
    }

    /// Returns the number of statements
    pub fn len(&self) -> usize {
        self.statements.iter().flat_map(|(_, v)| v).count()
    }

    /// Returns true if there are no statements
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }

    /// Returns the Statements for a specific property
    pub fn property<S: Into<String>>(&self, property: S) -> Vec<&Statement> {
        self.statements
            .get(&property.into())
            .map_or_else(Vec::new, |v| v.iter().collect())
    }

    pub fn insert(&mut self, statement: Statement) {
        let property = statement.property().to_owned();
        self.statements
            .entry(property.id().to_owned())
            .or_default()
            .push(statement);
    }

    pub const fn statements(&self) -> &HashMap<String, Vec<Statement>> {
        &self.statements
    }

    pub const fn statements_mut(&mut self) -> &mut HashMap<String, Vec<Statement>> {
        &mut self.statements
    }

    pub const fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    // Returns a list of all statements with an ID, as HashMap ID => &Statement
    fn get_id_statement_map(&self) -> HashMap<&str, &Statement> {
        self.statements
            .values()
            .flat_map(|v| v.iter())
            .filter_map(|statement| Some((statement.id()?.as_str(), statement)))
            .collect()
    }

    // Returns a list of all statements without IDs
    fn get_statements_without_id(&self) -> Vec<&Statement> {
        self.statements
            .values()
            .flat_map(|v| v.iter())
            .filter(|statement| statement.id().is_none())
            .collect()
    }

    pub fn patch(&self, other: &Self) -> Result<StatementsPatch, RestApiError> {
        // Statements without ID in other => fail
        if !other.get_statements_without_id().is_empty() {
            return Err(RestApiError::MissingId);
        }

        let mut patch = StatementsPatch::default();
        let from_statements_with_id = self.get_id_statement_map();
        let to_statements_with_id = other.get_id_statement_map();

        Self::patch_modify_remove(&mut patch, &from_statements_with_id, &to_statements_with_id)?;
        Self::patch_add_new(&mut patch, from_statements_with_id, to_statements_with_id);

        Ok(patch)
    }

    fn patch_modify_remove(
        patch: &mut StatementsPatch,
        from_statements_with_id: &HashMap<&str, &Statement>,
        to_statements_with_id: &HashMap<&str, &Statement>,
    ) -> Result<(), RestApiError> {
        for (statement_id, from_statement) in from_statements_with_id {
            match to_statements_with_id.get(statement_id) {
                Some(to_statement) => {
                    // Modify statement
                    let statement_patch = from_statement.patch(to_statement)?;
                    patch.patch_mut().extend(statement_patch.patch().to_owned());
                }
                None => {
                    // Remove statement
                    let statement_path = format!("/statements/{statement_id}"); // TODO check
                    patch.remove(statement_path);
                }
            }
        }
        Ok(())
    }

    fn patch_add_new(
        patch: &mut StatementsPatch,
        from_statements_with_id: HashMap<&str, &Statement>,
        to_statements_with_id: HashMap<&str, &Statement>,
    ) {
        // Add new statements
        for (statement_id, to_statement) in &to_statements_with_id {
            if !from_statements_with_id.contains_key(statement_id) {
                // Add new statement
                let add_path = format!("/statements/{statement_id}"); // TODO check
                let value = json!(to_statement);
                patch.add(add_path, value);
            }
        }
    }
}

// GET
#[async_trait]
impl HttpGetEntity for Statements {
    async fn get_match(
        id: &EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let path = Self::get_rest_api_path(id)?;
        let (j, header_info) = Self::get_match_internal(api, &path, rm).await?;
        Self::from_json_header_info(&j, header_info)
    }
}

// POST
impl Statements {
    /// Posts a new statement to an entity
    pub async fn post(
        &self,
        id: &EntityId,
        statement: Statement,
        api: &mut RestApi,
    ) -> Result<Statement, RestApiError> {
        self.post_meta(id, statement, api, EditMetadata::default())
            .await
    }

    /// Posts a new statement to an entity with metadata
    pub async fn post_meta(
        &self,
        id: &EntityId,
        mut statement: Statement,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Statement, RestApiError> {
        statement.set_id(None);
        let j0 = json!({"statement": statement});
        let request = self
            .generate_json_request(id, reqwest::Method::POST, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let (j, _statement_id) = self.filter_response_error(response).await?;
        // TODO add to self.statements?
        Statement::from_json(&j)
    }
}

impl HttpMisc for Statements {
    fn get_rest_api_path(id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/statements",
            group = id.group()?
        ))
    }
}

impl Serialize for Statements {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.statements.len()))?;
        for (property, statements) in &self.statements {
            s.serialize_entry(property, statements)?;
        }
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::statement_value::StatementValue;
    use http::{HeaderMap, HeaderValue};
    use wiremock::matchers::{bearer_token, body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statements_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();

        let mock_path = "/w/rest.php/wikibase/v1/entities/items/Q42/statements";
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["statements"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let statements = Statements::get(&EntityId::item("Q42"), &api).await.unwrap();
        assert!(!statements.property("P31").is_empty());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statements_post() {
        // #lizard forgives the complexity
        let id = EntityId::item("Q42");
        let v = std::fs::read_to_string("test_data/test_statements_post.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let mock_path = "/w/rest.php/wikibase/v1/entities/items/Q42/statements";
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({}))
                    .insert_header("ETag", "123"),
            )
            .mount(&mock_server)
            .await;
        Mock::given(body_partial_json(
            json!({"statement": {"value":{"content":"Q5"}}}),
        ))
        .and(method("POST"))
        .and(path(mock_path))
        .and(bearer_token(token))
        .respond_with(ResponseTemplate::new(200).set_body_json(&v))
        .mount(&mock_server)
        .await;
        let mut api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build();

        // Get and check existing statements
        let statements = Statements::get(&id, &api).await.unwrap();
        assert!(statements.property("P31").is_empty());

        // Create new statement
        let mut statement = Statement::default();
        statement.set_property("P31".into());
        statement.set_value(StatementValue::new_string("Q5"));

        // POST new statement
        let statement = statements.post(&id, statement, &mut api).await.unwrap();
        assert_eq!(statement.value(), &StatementValue::new_string("Q5"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_eq() {
        // To ensure that statement lists with and without header info are equal
        let id = EntityId::item("Q42");
        let mock_path = "/w/rest.php/wikibase/v1/entities/items/Q42/statements";
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({}))
                    .insert_header("ETag", "123"),
            )
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build();

        // Get empty statements but with revision ID
        let statements1 = Statements::get(&id, &api).await.unwrap();
        let statements2 = Statements::default();
        assert_eq!(statements1, statements2);
    }

    #[test]
    fn test_insert_and_len() {
        let mut statements = Statements::default();
        assert_eq!(statements.len(), 0);
        let mut statement = Statement::default();
        statement.set_property("P31".into());
        statements.insert(statement.clone());
        statements.insert(statement.clone());
        statement.set_property("P1".into());
        statements.insert(statement.clone());
        assert_eq!(statements.len(), 3);
    }

    #[test]
    fn test_statements_statements() {
        let mut statements = Statements::default();
        let mut statement = Statement::default();
        statement.set_property("P31".into());
        statements.insert(statement.clone());
        statement.set_property("P1".into());
        statements.insert(statement.clone());
        assert_eq!(statements.statements().len(), 2);
        statements.statements_mut().remove("P31");
        assert_eq!(statements.statements().len(), 1);
    }

    #[test]
    fn test_header_info() {
        let mut headers = HeaderMap::new();
        headers.insert("ETag", HeaderValue::from_str("1234567890").unwrap());
        headers.insert(
            "Last-Modified",
            HeaderValue::from_str("Wed, 21 Oct 2015 07:28:00 GMT").unwrap(),
        );
        let hi = HeaderInfo::from_header(&headers);
        let mut statements = Statements::default();
        assert_eq!(statements.header_info(), &HeaderInfo::default());
        statements.header_info = hi.to_owned();
        assert_eq!(statements.header_info(), &hi);
    }

    #[test]
    fn test_get_id_statement_map() {
        let mut statements = Statements::default();
        let mut statement = Statement::default();
        statement.set_id(Some("Q1".into()));
        statement.set_property("P31".into());
        statements.insert(statement.clone());
        statement.set_id(Some("Q2".into()));
        statement.set_property("P1".into());
        statements.insert(statement.clone());
        let id_statement_map = statements.get_id_statement_map();
        assert_eq!(id_statement_map.len(), 2);
        assert_eq!(id_statement_map.get("Q1").unwrap().property().id(), "P31");
        assert_eq!(id_statement_map.get("Q2").unwrap().property().id(), "P1");
    }

    #[test]
    fn test_get_statements_without_id() {
        let mut statements = Statements::default();
        let mut statement = Statement::default();
        statement.set_id(Some("Q1".into()));
        statement.set_property("P31".into());
        statements.insert(statement.clone());
        statement.set_id(None);
        statement.set_property("P1".into());
        statements.insert(statement.clone());
        let statements_without_id = statements.get_statements_without_id();
        assert_eq!(statements_without_id.len(), 1);
        assert_eq!(statements_without_id[0].property().id(), "P1");
    }

    #[test]
    fn test_patch() {
        let mut statements1 = Statements::default();
        let mut statement = Statement::default();
        statement.set_id(Some("Q1".into()));
        statement.set_property("P31".into());
        statements1.insert(statement.clone());
        statement.set_id(Some("Q2".into()));
        statement.set_property("P1".into());
        statements1.insert(statement.clone());

        let mut statements2 = Statements::default();
        statement.set_id(Some("Q1".into()));
        statement.set_property("P31".into());
        statements2.insert(statement.clone());
        statement.set_id(Some("Q3".into()));
        statement.set_property("P1".into());
        statements2.insert(statement.clone());

        let patch = statements1.patch(&statements2).unwrap();
        assert_eq!(patch.patch().len(), 2);
        assert_eq!(patch.patch()[0].op(), "remove");
        assert_eq!(patch.patch()[1].op(), "add");
    }
}
