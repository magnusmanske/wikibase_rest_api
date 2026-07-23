use crate::{
    patch_entry::PatchEntry, statements_patch::StatementsPatch, EditMetadata, EntityId, FromJson,
    HeaderInfo, HttpGetEntity, HttpMisc, Patch, RestApi, RestApiError, RevisionMatch, Statement,
};
use derive_where::DeriveWhere;
use serde::ser::{Serialize, SerializeMap};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

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

    /// Creates a new `Statements` object from a JSON structure, returning a default if null
    pub fn from_json_or_default(j: &Value) -> Result<Self, RestApiError> {
        if j.is_null() {
            Ok(Self::default())
        } else {
            Self::from_json(j)
        }
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
        self.statements.values().flatten().count()
    }

    /// Returns true if there are no statements.
    /// Consistent with `len()`: a property key holding an empty list still counts as empty.
    pub fn is_empty(&self) -> bool {
        self.statements.values().all(Vec::is_empty)
    }

    /// Returns the Statements for a specific property
    pub fn property<S: AsRef<str>>(&self, property: S) -> Vec<&Statement> {
        self.statements
            .get(property.as_ref())
            .map_or_else(Vec::new, |v| v.iter().collect())
    }

    /// Returns the mutable Statements for a specific property
    pub fn property_mut<S: AsRef<str>>(&mut self, property: S) -> Vec<&mut Statement> {
        self.statements
            .get_mut(property.as_ref())
            .map_or_else(Vec::new, |v| v.iter_mut().collect())
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

    /// Locates every statement (by ID) in this collection as `(id, property, index, statement)`.
    /// The index is the position within its property's array — i.e. the JSON Patch path component.
    fn id_locations(&self) -> Vec<(&str, &str, usize, &Statement)> {
        let mut locations = Vec::new();
        for (property, statements) in &self.statements {
            for (index, statement) in statements.iter().enumerate() {
                if let Some(id) = statement.id() {
                    locations.push((id.as_str(), property.as_str(), index, statement));
                }
            }
        }
        locations
    }

    /// Generates a JSON Patch that transforms `other` into `self`, suitable for the entity
    /// PATCH endpoint. Paths address statements as `/statements/{property}/{index}` using
    /// `other`'s indices (that is the document the patch is applied to).
    ///
    /// Operations are ordered so sequential application stays correct: modifications first
    /// (they don't resize arrays), then removals highest-index-first (so earlier removals
    /// don't shift later ones), then additions appended with `-`.
    pub fn patch(&self, other: &Self) -> Result<StatementsPatch, RestApiError> {
        // #lizard forgives the complexity
        // Every base statement must have an ID; without one it cannot be matched or located.
        if !other.get_statements_without_id().is_empty() {
            return Err(RestApiError::MissingId);
        }

        let mut base = other.id_locations();
        base.sort_by(|a, b| a.1.cmp(b.1).then(a.2.cmp(&b.2))); // by property, then index
        let target_by_id = self.get_id_statement_map();
        let base_ids: HashSet<&str> = base.iter().map(|&(id, ..)| id).collect();

        let mut patch = StatementsPatch::default();

        // 1. Modify statements present in both (matched by ID).
        for &(id, property, index, base_stmt) in &base {
            if let Some(target_stmt) = target_by_id.get(id).copied() {
                let prefix = format!("/statements/{property}/{index}");
                Self::push_modify(&mut patch, &prefix, target_stmt, base_stmt)?;
            }
        }

        // 2. Remove statements in `other` but not in `self`, highest index first per property.
        let mut removals: Vec<(&str, usize)> = base
            .iter()
            .filter(|(id, ..)| !target_by_id.contains_key(id))
            .map(|&(_, property, index, _)| (property, index))
            .collect();
        removals.sort_by(|a, b| a.0.cmp(b.0).then(b.1.cmp(&a.1)));
        for (property, index) in removals {
            patch.remove(format!("/statements/{property}/{index}"));
        }

        // 3. Add statements new to `self` (no ID, or an ID absent from `other`), appended.
        let mut properties: Vec<&String> = self.statements.keys().collect();
        properties.sort();
        for property in properties {
            for statement in self.statements.get(property).into_iter().flatten() {
                let is_new = statement
                    .id()
                    .is_none_or(|id| !base_ids.contains(id.as_str()));
                if is_new {
                    patch.add(format!("/statements/{property}/-"), json!(statement));
                }
            }
        }

        Ok(patch)
    }

    /// Appends a single statement's diff to `patch`, prefixing each op path with the statement's
    /// entity-level location so the ops apply within the entity document.
    fn push_modify(
        patch: &mut StatementsPatch,
        prefix: &str,
        target: &Statement,
        base: &Statement,
    ) -> Result<(), RestApiError> {
        let diff = target.patch(base)?;
        for entry in diff.patch() {
            let entity_path = format!("{prefix}{}", entry.path());
            patch.patch_mut().push(PatchEntry::new(
                entry.op(),
                entity_path,
                entry.value().clone(),
            ));
        }
        Ok(())
    }
}

// GET
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

impl Statements {
    /// Returns statements for a specific property, filtering server-side.
    pub async fn get_for_property(
        id: &EntityId,
        property_id: &str,
        api: &RestApi,
    ) -> Result<Self, RestApiError> {
        Self::get_for_property_match(id, property_id, api, RevisionMatch::default()).await
    }

    /// Returns statements for a specific property, with revision matching.
    pub async fn get_for_property_match(
        id: &EntityId,
        property_id: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let path = Self::get_rest_api_path(id)?;
        let mut params = HashMap::new();
        params.insert("property".to_string(), property_id.to_string());
        let mut request = api
            .wikibase_request_builder(&path, params, reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        let (j, header_info) = Self::api_execute(api, request).await?;
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
        api: &RestApi,
    ) -> Result<Statement, RestApiError> {
        self.post_meta(id, statement, api, EditMetadata::default())
            .await
    }

    /// Posts a new statement to an entity with metadata
    pub async fn post_meta(
        &self,
        id: &EntityId,
        mut statement: Statement,
        api: &RestApi,
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
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
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
        let statement = statements.post(&id, statement, &api).await.unwrap();
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

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statements_get_for_property() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let p31_statements = json!({"P31": v["statements"]["P31"]});

        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(
                "/w/rest.php/wikibase/v1/entities/items/Q42/statements",
            ))
            .and(wiremock::matchers::query_param("property", "P31"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&p31_statements))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let id = EntityId::item("Q42");
        let stmts = Statements::get_for_property(&id, "P31", &api)
            .await
            .unwrap();
        assert!(!stmts.property("P31").is_empty());
        assert!(stmts.property("P21").is_empty());
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

    #[test]
    fn test_patch_paths() {
        fn stmt(property: &str, value: &str, id: Option<&str>) -> Statement {
            let mut s = Statement::new_string(property, value);
            s.set_id(id.map(ToString::to_string));
            s
        }

        // base (other): P31=[Q42$A→Q1, Q42$B→Q2], P279=[Q42$C→Q3]
        let mut base = Statements::default();
        base.insert(stmt("P31", "Q1", Some("Q42$A")));
        base.insert(stmt("P31", "Q2", Some("Q42$B")));
        base.insert(stmt("P279", "Q3", Some("Q42$C")));

        // target (self): modify Q42$A to Q9, add a new statement, drop Q42$B and Q42$C
        let mut target = Statements::default();
        target.insert(stmt("P31", "Q9", Some("Q42$A")));
        target.insert(stmt("P31", "Q5", None));

        let patch = target.patch(&base).unwrap();
        let ops: Vec<(&str, &str)> = patch.patch().iter().map(|e| (e.op(), e.path())).collect();

        // Paths are entity-relative (/statements/{property}/{index}); modify, then removals
        // highest-index-first, then append.
        assert_eq!(
            ops,
            vec![
                ("replace", "/statements/P31/0/value/content"),
                ("remove", "/statements/P279/0"),
                ("remove", "/statements/P31/1"),
                ("add", "/statements/P31/-"),
            ]
        );
    }

    #[test]
    fn test_patch_base_without_id_fails() {
        let mut base = Statements::default();
        base.insert(Statement::new_string("P31", "Q1")); // no ID
        let target = Statements::default();
        assert!(target.patch(&base).is_err());
    }
}
