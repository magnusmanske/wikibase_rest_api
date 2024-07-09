use serde::Serialize;
use serde_json::Value;
use crate::{patch_entry::PatchEntry, EntityId, HttpMisc, Patch, RestApiError, Sitelinks};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct SitelinksPatch {
    patch: Vec<PatchEntry>,
}

impl SitelinksPatch {
    /// Adds a command to replace the title of a sitelink
    pub fn replace_title<S1: Into<String>, S2: Into<String>>(&mut self, wiki: S1, value: S2) {
        self.replace(&format!("/{}/title", wiki.into()), value.into().into());
    }
    
    /// Generates a patch from JSON, presumably from json_patch
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        let pe = j.as_array()
            .ok_or(RestApiError::WrongType{field: "SitelinksPatch".into(), j: j.to_owned()})?
            .iter()
            .map(|x| serde_json::from_value(x.clone()).map_err(|e| e.into()))
            .collect::<Result<Vec<PatchEntry>,RestApiError>>()?;
        Ok(Self {
            patch: pe,
        })
    }
}

impl Patch<Sitelinks> for SitelinksPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }
}

impl HttpMisc for SitelinksPatch {
    fn get_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!("/entities/{}/{id}/sitelinks",id.group()?))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{bearer_token, body_partial_json, header, method, path};
    use crate::RestApi;

    use super::*;

    #[tokio::test]
    async fn test_sitelinks_patch() {
        let id = "Q42";
        let page_title = "Foo Bar";
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let mut new_sitelinks = v["sitelinks"].clone();
        new_sitelinks["enwiki"]["title"] = json!(page_title);

        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/sitelinks");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(json!({"patch":[{"op": "replace","path": "/enwiki/title","value": page_title}]})))
            .and(method("PATCH"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .and(header("content-type", "application/json-patch+json"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "12345").set_body_json(new_sitelinks))
            .mount(&mock_server).await;
        let mut api = RestApi::builder().api(&(mock_server.uri()+"/w/rest.php")).set_access_token(token).build().unwrap();

        // Apply patch and check API response
        let id = EntityId::new(id).unwrap();
        let mut patch = SitelinksPatch::default();
        patch.replace_title("enwiki", page_title);
        let sl = patch.apply(&id, &mut api).await.unwrap();
        assert_eq!(sl.get_wiki("enwiki").unwrap().title(), page_title);
    }

    #[test]
    fn test_replace_title() {
        let mut patch = SitelinksPatch::default();
        patch.replace_title("enwiki", "Foo Bar");
        assert_eq!(patch.patch(), &[PatchEntry::new("replace", "/enwiki/title", json!("Foo Bar"))]);
    }
}