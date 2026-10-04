use crate::{
    patch_entry::PatchEntry, statement::Statement, EditMetadata, EntityId, HttpMisc, Patch,
    PatchApply, RestApi, RestApiError,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct StatementPatch {
    statement_id: String,
    patch: Vec<PatchEntry>,
}

impl HttpMisc for StatementPatch {
    /// `id` is the entity the statement belongs to; `EntityId::None` uses the
    /// entity-independent `/statements/{statement_id}` endpoint.
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Statement::rest_api_path(id, &self.statement_id)
    }
}

impl StatementPatch {
    /// Generates a new `StatementPatch` for a given statement ID
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self {
            statement_id: id.into(),
            patch: vec![],
        }
    }

    /// Generates a patch from JSON, presumably from `json_patch`
    pub fn from_json<S: Into<String>>(
        statement_id: S,
        j: &Value,
    ) -> Result<StatementPatch, RestApiError> {
        Ok(StatementPatch {
            patch: PatchEntry::list_from_json(j, "StatementPatch")?,
            statement_id: statement_id.into(),
        })
    }

    /// Adds a command to replace the content of a statement
    pub fn replace_content(&mut self, value: Value) {
        self.replace("/value/content".to_string(), value);
    }

    /// Applies the patch via `/statements/{statement_id}`; no entity ID needed.
    pub async fn apply(&self, api: &RestApi) -> Result<Statement, RestApiError> {
        self.apply_match(api, EditMetadata::default()).await
    }

    /// Applies the patch via `/statements/{statement_id}`, with edit metadata.
    pub async fn apply_match(
        &self,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Statement, RestApiError> {
        self.apply_match_for_entity(&EntityId::None, api, em).await
    }

    /// Applies the patch via `/entities/{group}/{entity_id}/statements/{statement_id}`.
    pub async fn apply_for_entity(
        &self,
        entity_id: &EntityId,
        api: &RestApi,
    ) -> Result<Statement, RestApiError> {
        self.apply_match_for_entity(entity_id, api, EditMetadata::default())
            .await
    }

    /// Applies the patch via `/entities/{group}/{entity_id}/statements/{statement_id}`,
    /// with edit metadata.
    pub async fn apply_match_for_entity(
        &self,
        entity_id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Statement, RestApiError> {
        <Self as PatchApply<Statement>>::apply_match(self, entity_id, api, em).await
    }
}

impl Patch for StatementPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }
}

impl PatchApply<Statement> for StatementPatch {}

#[cfg(test)]
mod tests {
    use crate::{statement_value::StatementValue, FromJson};
    use serde_json::json;
    use wiremock::matchers::{bearer_token, body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_statement_patch() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let mut new_statement = v["statements"]["P31"][0].clone();
        new_statement["value"]["content"] = json!("Q6");

        let statement_id = "Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9";
        let mock_path = format!("/w/rest.php/wikibase/v1/statements/{statement_id}");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(
            json!({"patch":[{"op": "replace","path": "/value/content","value": "Q6"}]}),
        ))
        .and(method("PATCH"))
        .and(path(&mock_path))
        .and(bearer_token(token))
        .and(header("content-type", "application/json-patch+json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "12345")
                .set_body_json(new_statement),
        )
        .mount(&mock_server)
        .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build()
            .unwrap();

        // Patch statement
        let mut patch = StatementPatch::new(statement_id);
        patch.replace_content(json!("Q6"));
        let statement = patch.apply(&api).await.unwrap();
        assert_eq!(statement.header_info().revision_id(), Some(12345));
        assert_eq!(statement.value(), &StatementValue::new_string("Q6"));
    }

    #[test]
    fn test_replace_content() {
        let mut patch = StatementPatch::new("Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9");
        patch.replace_content(json!("Q6"));
        assert_eq!(
            patch.patch(),
            &[PatchEntry::new("replace", "/value/content", json!("Q6"))]
        );
    }

    #[test]
    fn test_from_json() {
        let statement_id = "Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9";
        let j = json!([{"op": "replace", "path": "/value/content", "value": "Q6"}]);
        let patch = StatementPatch::from_json(statement_id, &j).unwrap();
        assert_eq!(
            patch.patch(),
            &[PatchEntry::new("replace", "/value/content", json!("Q6"))]
        );
        assert_eq!(
            patch.get_my_rest_api_path(&EntityId::None).unwrap(),
            format!("/statements/{statement_id}")
        );
    }

    #[test]
    fn test_from_json_not_an_array() {
        let j = json!({"op": "replace"});
        let err = StatementPatch::from_json("Q42$X", &j).unwrap_err();
        assert!(matches!(
            err,
            RestApiError::WrongType { field, .. } if field == "StatementPatch"
        ));
    }

    #[test]
    fn test_get_rest_api_path() {
        let patch = StatementPatch::new("Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9");
        assert_eq!(
            patch
                .get_my_rest_api_path(&EntityId::new("Q42").unwrap())
                .unwrap(),
            "/entities/items/Q42/statements/Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9"
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_apply_for_entity() {
        let v = std::fs::read_to_string("test_data/test_statement_get.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let statement_id = v["id"].as_str().unwrap().to_string();
        let entity = statement_id.split('$').next().unwrap().to_string();
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path(format!(
                "/w/rest.php/wikibase/v1/entities/items/{entity}/statements/{statement_id}"
            )))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();
        let mut patch = StatementPatch::new(&statement_id);
        patch.replace_content(json!("Q5"));
        let statement = patch
            .apply_for_entity(&EntityId::item(entity), &api)
            .await
            .unwrap();
        assert_eq!(statement.id(), Some(statement_id.as_str()));
    }
}
