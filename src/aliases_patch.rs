use crate::{
    aliases::Aliases, patch_entry::PatchEntry, EntityId, HttpMisc, Patch, PatchApply, RestApiError,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct AliasesPatch {
    patch: Vec<PatchEntry>,
}

impl AliasesPatch {
    /// Adds a command to replace an alias in a specific language, at a specific position
    pub fn replace<S1: Into<String>, S2: Into<String>>(
        &mut self,
        language: S1,
        num: usize,
        value: S2,
    ) {
        <Self as Patch>::replace(
            self,
            format!("/{}/{num}", language.into()),
            value.into().into(),
        );
    }

    /// Adds a command to remove an alias in a specific language, at a specific position
    pub fn remove<S: Into<String>>(&mut self, language: S, num: usize) {
        <Self as Patch>::remove(self, format!("/{}/{num}", language.into()));
    }

    /// Generates a patch from JSON, presumably from `json_patch`
    pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
        Ok(Self {
            patch: PatchEntry::list_from_json(j, "AliasesPatch")?,
        })
    }
}

impl Patch for AliasesPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }
}

impl PatchApply<Aliases> for AliasesPatch {}

impl HttpMisc for AliasesPatch {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!("/entities/{}/{id}/aliases", id.group()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RestApi;
    use serde_json::json;
    use wiremock::matchers::{bearer_token, body_partial_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_aliases_patch() {
        let id = "Q42";
        let new_alias = "Foo bar baz";
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let mut new_aliases = v["aliases"].to_owned();
        new_aliases["en"][1] = json!(new_alias);

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/aliases");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(
            json!({"patch":[{"op": "replace","path": "/en/1","value": new_alias}]}),
        ))
        .and(method("PATCH"))
        .and(path(&mock_path))
        .and(bearer_token(token))
        .and(header("content-type", "application/json-patch+json"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "12345")
                .set_body_json(&new_aliases),
        )
        .mount(&mock_server)
        .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build()
            .unwrap();

        // Apply patch
        let id = EntityId::new(id).unwrap();
        let mut patch = AliasesPatch::default();
        patch.replace("en", 1, new_alias);
        let new_aliases2 = patch.apply(&id, &api).await.unwrap();
        assert_eq!(new_aliases2.get_lang("en")[1], new_alias);
    }
}
