use crate::{
    aliases_patch::AliasesPatch, prelude::LanguageStrings, EntityId, FromJson, HeaderInfo,
    LanguageString, RestApi, RestApiError, RevisionMatch,
};
use derive_where::DeriveWhere;
use reqwest::StatusCode;
use serde::ser::{Serialize, SerializeMap};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(DeriveWhere, Debug, Clone, Default)]
#[derive_where(PartialEq)]
pub struct Aliases {
    ls: HashMap<String, Vec<String>>,
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl Aliases {
    /// Creates a new `Aliases` struct for the given entity ID.
    /// If the API cannot find anything, the `Aliases` struct will be empty.
    ///
    /// # Errors
    /// Returns a `RestApiError` if the API request fails.
    pub async fn get_match(
        id: &EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let response = Self::get_match_response(id, api, rm).await?;

        let header_info = HeaderInfo::from_header(response.headers());
        let ls = Self::get_match_check_response(response).await?;
        Ok(Self { ls, header_info })
    }

    /// Creates a new `Aliases` struct for the given entity ID.
    /// If the API cannot find anything, the `Aliases` struct will be empty.
    ///
    /// # Errors
    /// Returns a `RestApiError` if the API request fails.
    pub async fn get(id: &EntityId, api: &RestApi) -> Result<Self, RestApiError> {
        Self::get_match(id, api, RevisionMatch::default()).await
    }

    /// Returns the list of values for a language
    pub fn get_lang<S: Into<String>>(&self, language: S) -> Vec<&str> {
        self.ls
            .get(&language.into())
            .map_or_else(Vec::new, |v| v.iter().map(|s| s.as_str()).collect())
    }

    /// Returns the list of values for a language, mutable
    pub fn get_lang_mut<S: Into<String>>(&mut self, language: S) -> &mut Vec<String> {
        self.ls.entry(language.into()).or_default()
    }

    /// Generates a patch to transform `other` into `self`
    ///
    /// # Errors
    /// Returns a `RestApiError` if the API request fails.
    pub fn patch(&self, other: &Self) -> Result<AliasesPatch, RestApiError> {
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = AliasesPatch::from_json(&json!(patch))?;
        Ok(patch)
    }

    /// Returns the number of languages
    pub fn len(&self) -> usize {
        self.ls.len()
    }

    /// Returns true if there are no language strings
    pub fn is_empty(&self) -> bool {
        self.ls.is_empty()
    }

    fn from_json_header_info_part(
        language: &str,
        values: &[Value],
    ) -> Result<(String, Vec<String>), RestApiError> {
        let values = values
            .iter()
            .map(|v| {
                Ok(v.as_str()
                    .ok_or_else(|| RestApiError::MissingOrInvalidField {
                        field: "LanguageStringsMultiple".into(),
                        j: v.to_owned(),
                    })?
                    .to_string())
            })
            .collect::<Result<Vec<String>, RestApiError>>()?;
        Ok((language.to_owned(), values))
    }

    async fn get_match_response(
        id: &EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<reqwest::Response, RestApiError> {
        let path = format!("/entities/{group}/{id}/aliases", group = id.group()?);
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        let response = api.execute(request).await?;
        Ok(response)
    }

    async fn get_match_check_response(
        response: reqwest::Response,
    ) -> Result<HashMap<String, Vec<String>>, RestApiError> {
        let ls: HashMap<String, Vec<String>> = match response.error_for_status() {
            Ok(response) => response.json().await?,
            Err(e) => {
                if e.status() == Some(StatusCode::NOT_FOUND) {
                    HashMap::new()
                } else {
                    return Err(e.into());
                }
            }
        };
        Ok(ls)
    }
}

impl FromJson for Aliases {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let ls = j
            .as_object()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "LanguageStringsMultiple".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|(language, value)| {
                value.as_array().map_or_else(
                    || {
                        Err(RestApiError::MissingOrInvalidField {
                            field: "LanguageStringsMultiple".into(),
                            j: value.to_owned(),
                        })
                    },
                    |v| Self::from_json_header_info_part(language, v),
                )
            })
            .collect::<Result<HashMap<String, Vec<String>>, RestApiError>>()?;
        let ret = Self { ls, header_info };
        Ok(ret)
    }
}

impl LanguageStrings for Aliases {
    fn has_language<S: Into<String>>(&self, language: S) -> bool {
        self.ls.contains_key(&language.into())
    }

    fn insert(&mut self, ls: LanguageString) {
        let entry = self.ls.entry(ls.language().to_string()).or_default();
        if !entry.contains(ls.value()) {
            entry.push(ls.value().to_owned());
        }
    }
}

impl Serialize for Aliases {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.ls.len()))?;
        for (language, values) in &self.ls {
            s.serialize_entry(language, &values)?;
        }
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_aliases_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();
        let v: Value = v["aliases"].clone();

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/aliases");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let sitelinks = Aliases::get(&EntityId::item("Q42"), &api).await.unwrap();
        assert_eq!(sitelinks.ls.len(), 64);
        assert_eq!(sitelinks.get_lang("tok")[0], "jan Takala Atan");
    }

    #[test]
    fn test_aliases() {
        let j = json!({
            "en": ["Hello", "Hi"],
            "de": ["Hallo", "Hi"],
        });
        let ls = Aliases::from_json(&j).unwrap();
        assert_eq!(ls.get_lang("en"), vec!["Hello", "Hi"]);
        assert_eq!(ls.get_lang("de"), vec!["Hallo", "Hi"]);
        assert!(ls.get_lang("fr").is_empty());
    }

    #[test]
    fn test_aliases_insert() {
        let mut ls = Aliases::default();
        ls.insert(LanguageString::new("en", "Hello"));
        ls.insert(LanguageString::new("de", "Hallo"));
        ls.insert(LanguageString::new("en", "Hi"));
        assert_eq!(ls.get_lang("en"), vec!["Hello", "Hi"]);
        assert_eq!(ls.get_lang("de"), vec!["Hallo"]);
    }

    #[test]
    fn test_patch_aliases() {
        let mut l1 = Aliases::default();
        l1.insert(LanguageString::new("en", "Foo"));
        l1.insert(LanguageString::new("en", "Bar"));
        l1.insert(LanguageString::new("en", "Baz"));
        l1.insert(LanguageString::new("de", "Foobar"));
        let mut l2 = l1.clone();
        l2.get_lang_mut("en")[2] = "Boo".to_string();
        l2.get_lang_mut("en").remove(1);
        l2.insert(LanguageString::new("de", "Foobaz"));

        let patch = l2.patch(&l1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"patch":[{"op":"add","path":"/de/1","value":"Foobaz"},{"op":"replace","path":"/en/1","value":"Boo"},{"op":"remove","path":"/en/2"}]})
        );
    }

    #[test]
    fn test_header_info_multiple() {
        let l = Aliases::default();
        assert_eq!(l.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_serialize2() {
        let mut l = Aliases::default();
        l.insert(LanguageString::new("en", "Foo"));
        l.insert(LanguageString::new("en", "Bar"));
        l.insert(LanguageString::new("de", "Baz"));
        let s = serde_json::to_string(&l).unwrap();
        assert!(s.contains(r#""en":["Foo","Bar"]"#));
        assert!(s.contains(r#""de":["Baz"]"#));
    }

    #[test]
    fn test_is_empty() {
        let mut l = Aliases::default();
        assert!(l.is_empty());
        l.insert(LanguageString::new("en", "Foo"));
        assert!(!l.is_empty());
    }
}
