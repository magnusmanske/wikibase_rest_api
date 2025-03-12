use crate::{
    patch_entry::PatchEntry, EditMetadata, EntityId, FromJson, HttpMisc, Patch, PatchApply,
    RestApi, RestApiError, Statement,
};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct StatementPatch {
    statement_id: String,
    patch: Vec<PatchEntry>,
}

impl HttpMisc for StatementPatch {
    fn get_rest_api_path(&self, _id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!("/statements/{id}", id = self.statement_id))
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
        let pe = j
            .as_array()
            .ok_or(RestApiError::WrongType {
                field: "StatementPatch".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|x| serde_json::from_value(x.clone()).map_err(|e| e.into()))
            .collect::<Result<Vec<PatchEntry>, RestApiError>>()?;
        Ok(StatementPatch {
            patch: pe,
            statement_id: statement_id.into(),
        })
    }

    /// Adds a command to replace the content of a statement
    pub fn replace_content(&mut self, value: Value) {
        self.replace("/value/content".to_string(), value);
    }

    // Overrides the Patch<Statement> implementation becaue we don't need the EntityId
    pub async fn apply(&self, api: &mut RestApi) -> Result<Statement, RestApiError> {
        self.apply_match(api, EditMetadata::default()).await
    }

    // Overrides the Patch<Statement> implementation becaue we don't need the EntityId
    pub async fn apply_match(
        &self,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Statement, RestApiError> {
        <Self as PatchApply<Statement>>::apply_match(self, &EntityId::None, api, em).await
    }
}

#[async_trait]
impl Patch for StatementPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }
}

#[async_trait]
impl PatchApply<Statement> for StatementPatch {
    async fn apply_match(
        &self,
        _id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Statement, RestApiError> {
        let j0 = json!({"patch":self.patch});
        let request = self
            .generate_json_request(&EntityId::None, reqwest::Method::PATCH, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let (j, header_info) = self.filter_response_error(response).await?;
        Statement::from_json_header_info(&j, header_info)
    }
}

#[cfg(test)]
mod tests {
    use crate::statement_value::StatementValue;
    use wiremock::matchers::{bearer_token, body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
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
        let mut api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build();

        // Patch statement
        let mut patch = StatementPatch::new(statement_id);
        patch.replace_content(json!("Q6"));
        let statement = patch.apply(&mut api).await.unwrap();
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
    fn test_get_rest_api_path() {
        let patch = StatementPatch::new("Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9");
        assert_eq!(
            patch
                .get_rest_api_path(&EntityId::new("Q42").unwrap())
                .unwrap(),
            "/statements/Q42$F078E5B3-F9A8-480E-B7AC-D97778CBBEF9"
        );
    }
}
