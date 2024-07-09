use async_trait::async_trait;
use serde_json::{json, Value};
use crate::{patch_entry::PatchEntry, EditMetadata, EntityId, HeaderInfo, HttpMisc, RestApi, RestApiError};

#[async_trait]
pub trait Patch<T: FromJson>: Sized+HttpMisc {
    /// Returns the patch entries
    fn patch(&self) -> &Vec<PatchEntry>;

    /// Returns the mutable patch entries
    fn patch_mut(&mut self) -> &mut Vec<PatchEntry>;

    /// `path` is a JSON patch path, eg "/enwiki/title"
    fn add<S: Into<String>>(&mut self, path: S, value: Value) {
        self.patch_mut().push(PatchEntry::new("add", &path.into(), value));
    }

    /// `path` is a JSON patch path, eg "/enwiki/title"
    fn replace<S: Into<String>>(&mut self, path: S, value: Value) {
        self.patch_mut().push(PatchEntry::new("replace", &path.into(), value));
    }

    /// `path` is a JSON patch path, eg "/enwiki/title"
    fn remove<S: Into<String>>(&mut self, path: S) {
        self.patch_mut().push(PatchEntry::new("remove", &path.into(), Value::Null));
    }

    /// checks if the patch list is empty
    fn is_empty(&self) -> bool {
        self.patch().is_empty()
    }

    /// Applies the entire patch against the API
    async fn apply(&self, id: &EntityId, api: &mut RestApi) -> Result<T, RestApiError> {
        self.apply_match(id, api, EditMetadata::default()).await
    }

    /// Applies the entire patch against the API, conditional on metadata
    async fn apply_match(&self, id: &EntityId, api: &mut RestApi, em: EditMetadata) -> Result<T, RestApiError> {
        let j = json!({"patch": self.patch()});
        let request = self.generate_json_request(&id, reqwest::Method::PATCH, j, api, &em).await?;
        let response = api.execute(request).await?;
        let (j, header_info) = self.filter_response_error(response).await?;
        Ok(T::from_json_header_info(&j, header_info)?)
    }
}

pub trait FromJson : Sized {
    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError>;
    fn header_info(&self) -> &HeaderInfo;

    fn from_json(j: &Value) -> Result<Self, RestApiError> {
        Self::from_json_header_info(j, HeaderInfo::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::aliases_patch::AliasesPatch;

    use super::*;

    #[test]
    fn test_add() {
        let mut p = AliasesPatch::default();
        p.add("en", json!("foo"));
        assert_eq!(p.patch(), &vec![
            PatchEntry::new("add", "en", json!("foo")),
        ]);
    }

    #[test]
    fn test_replace() {
        let mut p = AliasesPatch::default();
        p.replace("en", 0, "foo");
        assert_eq!(p.patch(), &vec![
            PatchEntry::new("replace", "/en/0", json!("foo")),
        ]);
    }

    #[test]
    fn test_remove() {
        let mut p = AliasesPatch::default();
        p.remove("en", 1);
        assert_eq!(p.patch(), &vec![
            PatchEntry::new("remove", "/en/1", Value::Null),
        ]);
    }

    #[test]
    fn test_is_empty() {
        let p = AliasesPatch::default();
        assert!(p.is_empty());
    }
}