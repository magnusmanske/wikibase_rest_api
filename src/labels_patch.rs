use crate::{
    descriptions::Descriptions, labels::Labels, patch_entry::PatchEntry, EditMetadata, EntityId,
    FromJson, HttpMisc, Patch, RestApi, RestApiError,
};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct LabelsPatch {
    patch: Vec<PatchEntry>,
}

impl LabelsPatch {
    pub fn from_json(j: &Value) -> Result<Vec<PatchEntry>, RestApiError> {
        j.as_array()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "LabelsPatch".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|x| serde_json::from_value(x.clone()).map_err(|e| e.into()))
            .collect::<Result<Vec<PatchEntry>, RestApiError>>()
    }

    // TODO add?

    /// Adds a command to replace the value of a language string.
    /// TODO Labels?
    pub fn replace<S1: Into<String>, S2: Into<String>>(&mut self, language: S1, value: S2) {
        <Self as Patch<Labels>>::replace(
            self,
            format!("/{}", language.into()),
            value.into().into(),
        );
    }

    /// Adds a command to remove the value for the language.
    /// TODO Labels?
    pub fn remove<S: Into<String>>(&mut self, language: S) {
        <Self as Patch<Labels>>::remove(self, format!("/{}", language.into()));
    }
}

#[async_trait]
impl Patch<Labels> for LabelsPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }

    async fn apply_match(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Labels, RestApiError> {
        let j0 = json!({"patch": self.patch});
        let request = self
            .generate_json_request(id, reqwest::Method::PATCH, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let (j, header_info) = self.filter_response_error(response).await?;
        Ok(Labels::from_json_header_info(&j, header_info)?)
    }
}

#[async_trait]
impl Patch<Descriptions> for LabelsPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }

    async fn apply_match(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Descriptions, RestApiError> {
        let j0 = json!({"patch": self.patch});
        let request = self
            .generate_json_request(id, reqwest::Method::PATCH, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let (j, header_info) = self.filter_response_error(response).await?;
        Ok(Descriptions::from_json_header_info(&j, header_info)?)
    }
}

impl HttpMisc for LabelsPatch {
    fn get_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/labels",
            group = id.group()?
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn test_remove() {
        let mut patch = LabelsPatch::default();
        patch.remove("en");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("remove", "/en", Value::Null)]
        );
    }

    #[test]
    fn test_patch() {
        let mut patch = LabelsPatch::default();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_patch_fn() {
        let mut patch = LabelsPatch::default();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            *<LabelsPatch as Patch<Labels>>::patch(&patch),
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_from_json() {
        let j = json!([
            {"op": "replace", "path": "/en", "value": "Foo Bar"},
            {"op": "remove", "path": "/de"}
        ]);
        let patch = LabelsPatch::from_json(&j).unwrap();
        assert_eq!(
            patch,
            vec![
                PatchEntry::new("replace", "/en", json!("Foo Bar")),
                PatchEntry::new("remove", "/de", Value::Null)
            ]
        );
    }
}
