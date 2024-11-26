use crate::{
    language_strings::LanguageStringsSingle, patch_entry::PatchEntry, EditMetadata, EntityId,
    FromJson, HttpMisc, Patch, RestApi, RestApiError,
};
use async_trait::async_trait;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Serialize)]
enum Mode {
    Labels,
    Descriptions,
}

impl Mode {
    const fn as_str(&self) -> &str {
        match self {
            Mode::Labels => "labels",
            Mode::Descriptions => "descriptions",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LanguageStringsPatch {
    patch: Vec<PatchEntry>,
    mode: Mode,
}

impl LanguageStringsPatch {
    pub const fn labels() -> Self {
        Self {
            patch: vec![],
            mode: Mode::Labels,
        }
    }

    pub const fn descriptions() -> Self {
        Self {
            patch: vec![],
            mode: Mode::Descriptions,
        }
    }

    /// Generates a patch from JSON, presumably from `json_patch`
    pub fn labels_from_json(j: &Value) -> Result<Self, RestApiError> {
        Ok(Self {
            patch: Self::from_json(j)?,
            mode: Mode::Labels,
        })
    }

    /// Generates a patch from JSON, presumably from `json_patch`
    pub fn descriptions_from_json(j: &Value) -> Result<Self, RestApiError> {
        Ok(Self {
            patch: Self::from_json(j)?,
            mode: Mode::Descriptions,
        })
    }

    fn from_json(j: &Value) -> Result<Vec<PatchEntry>, RestApiError> {
        j.as_array()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "LanguageStringsPatch".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|x| serde_json::from_value(x.clone()).map_err(|e| e.into()))
            .collect::<Result<Vec<PatchEntry>, RestApiError>>()
    }

    // TODO add?

    /// Adds a command to replace the value of a language string.
    pub fn replace<S1: Into<String>, S2: Into<String>>(&mut self, language: S1, value: S2) {
        <Self as Patch<LanguageStringsSingle>>::replace(
            self,
            format!("/{}", language.into()),
            value.into().into(),
        );
    }

    /// Adds a command to remove the value for the language.
    pub fn remove<S: Into<String>>(&mut self, language: S) {
        <Self as Patch<LanguageStringsSingle>>::remove(self, format!("/{}", language.into()));
    }
}

#[async_trait]
impl Patch<LanguageStringsSingle> for LanguageStringsPatch {
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
    ) -> Result<LanguageStringsSingle, RestApiError> {
        let j0 = json!({"patch": self.patch});
        let request = self
            .generate_json_request(id, reqwest::Method::PATCH, j0, api, &em)
            .await?;
        let response = api.execute(request).await?;
        let (j, header_info) = self.filter_response_error(response).await?;
        Ok(LanguageStringsSingle::from_json_header_info(
            &j,
            header_info,
        )?)
    }
}

impl HttpMisc for LanguageStringsPatch {
    fn get_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/{mode}",
            group = id.group()?,
            mode = self.mode.as_str()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use wiremock::matchers::{bearer_token, body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_language_strings_single_patch() {
        let id = "Q42";
        let page_title = "Foo Bar";
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let mut new_label = v["labels"].clone();
        new_label["en"] = json!(page_title);

        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/labels");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(
            json!({"patch":[{"op": "replace","path": "/en","value": page_title}]}),
        ))
        .and(method("PATCH"))
        .and(path(&mock_path))
        .and(bearer_token(token))
        .and(header("content-type", "application/json-patch+json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "12345")
                .set_body_json(new_label),
        )
        .mount(&mock_server)
        .await;
        let mut api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .set_access_token(token)
            .build()
            .unwrap();

        // Apply patch and check API response
        let id = EntityId::new(id).unwrap();
        let mut patch = LanguageStringsPatch::labels();
        patch.replace("en", page_title);
        let ls = patch.apply(&id, &mut api).await.unwrap();
        assert_eq!(ls.get_lang("en").unwrap(), page_title);
    }

    #[test]
    fn test_remove() {
        let mut patch = LanguageStringsPatch::labels();
        patch.remove("en");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("remove", "/en", Value::Null)]
        );
    }

    #[test]
    fn test_patch() {
        let mut patch = LanguageStringsPatch::labels();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_descriptions() {
        let mut patch = LanguageStringsPatch::descriptions();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            patch.patch,
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_mode_as_str() {
        assert_eq!(Mode::Labels.as_str(), "labels");
        assert_eq!(Mode::Descriptions.as_str(), "descriptions");
    }

    #[test]
    fn test_patch_fn() {
        let mut patch = LanguageStringsPatch::labels();
        patch.replace("en", "Foo Bar");
        assert_eq!(
            *patch.patch(),
            vec![PatchEntry::new("replace", "/en", json!("Foo Bar"))]
        );
    }

    #[test]
    fn test_from_json() {
        let j = json!([
            {"op": "replace", "path": "/en", "value": "Foo Bar"},
            {"op": "remove", "path": "/de"}
        ]);
        let patch = LanguageStringsPatch::from_json(&j).unwrap();
        assert_eq!(
            patch,
            vec![
                PatchEntry::new("replace", "/en", json!("Foo Bar")),
                PatchEntry::new("remove", "/de", Value::Null)
            ]
        );
    }
}
