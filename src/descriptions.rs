use crate::{
    language_strings_patch::LanguageStringsPatch, prelude::LanguageStrings, EntityId, FromJson,
    HeaderInfo, HttpGetEntity, HttpMisc, LanguageString, RestApi, RestApiError, RevisionMatch,
};
use async_trait::async_trait;
use derive_where::DeriveWhere;
use serde::ser::{Serialize, SerializeMap};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(DeriveWhere, Debug, Clone, Default)]
#[derive_where(PartialEq)]
pub struct Descriptions {
    ls: HashMap<String, String>,
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl Descriptions {
    /// Returns the value for a language
    pub fn get_lang<S: Into<String>>(&self, language: S) -> Option<&str> {
        self.ls.get(&language.into()).map(|s| s.as_str())
    }

    /// Returns the number of labels/languages
    pub fn len(&self) -> usize {
        self.ls.len()
    }

    /// Returns true if there are no labels/languages
    pub fn is_empty(&self) -> bool {
        self.ls.is_empty()
    }

    /// Returns a reference to the descriptions/languages
    pub const fn list(&self) -> &HashMap<String, String> {
        &self.ls
    }

    /// Returns a mutable reference to the descriptions/languages
    pub const fn list_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.ls
    }

    /// Generates a patch to transform `other` into `self`
    ///
    /// # Errors
    /// Returns an `RestApiError` if the request fails.
    pub fn patch(&self, other: &Self) -> Result<LanguageStringsPatch, RestApiError> {
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = LanguageStringsPatch::descriptions_from_json(&json!(patch))?;
        Ok(patch)
    }
}

impl HttpMisc for Descriptions {
    fn get_rest_api_path(id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/descriptions",
            group = id.group()?
        ))
    }
}

#[async_trait]
impl HttpGetEntity for Descriptions {
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

impl FromJson for Descriptions {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let ls = j
            .as_object()
            .ok_or_else(|| RestApiError::WrongType {
                field: "Descriptions".to_string(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|(language, value)| {
                let value = value
                    .as_str()
                    .ok_or_else(|| RestApiError::MissingOrInvalidField {
                        field: "Descriptions".into(),
                        j: value.to_owned(),
                    })?;
                Ok((language.to_owned(), value.to_string()))
            })
            .collect::<Result<HashMap<String, String>, RestApiError>>()?;
        let ret = Self { ls, header_info };
        Ok(ret)
    }
}

impl LanguageStrings for Descriptions {
    fn has_language<S: Into<String>>(&self, language: S) -> bool {
        self.ls.contains_key(&language.into())
    }

    fn insert(&mut self, ls: LanguageString) {
        self.ls
            .insert(ls.language().to_string(), ls.value().to_string());
    }
}

impl Serialize for Descriptions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.ls.len()))?;
        for (language, ls) in &self.ls {
            s.serialize_entry(language, ls)?;
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

    #[test]
    fn test_language_strings_single() {
        let j = json!({
            "en": "Hello",
            "de": "Hallo",
        });
        let ls = Descriptions::from_json(&j).unwrap();
        assert_eq!(ls.get_lang("en"), Some("Hello"));
        assert_eq!(ls.get_lang("de"), Some("Hallo"));
        assert_eq!(ls.get_lang("fr"), None);
    }

    #[test]
    fn test_language_strings_insert() {
        let mut ls = Descriptions::default();
        ls.insert(LanguageString::new("en", "Hello"));
        ls.insert(LanguageString::new("de", "Hallo"));
        ls.insert(LanguageString::new("en", "Hi"));
        assert_eq!(ls.get_lang("en"), Some("Hi"));
        assert_eq!(ls.get_lang("de"), Some("Hallo"));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_language_strings_single_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();

        let mock_path = "/w/rest.php/wikibase/v1/entities/items/Q42/descriptions";
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["descriptions"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let id = EntityId::new("Q42").unwrap();
        let ls = Descriptions::get(&id, &api).await.unwrap();
        assert_eq!(ls.get_lang("en-gb"), Some("English writer and humourist"));
    }

    #[test]
    fn test_patch_descriptions() {
        let mut l1 = Descriptions::default();
        l1.insert(LanguageString::new("en", "Foo"));
        l1.insert(LanguageString::new("de", "Bar"));
        let mut l2 = l1.clone();
        l2.insert(LanguageString::new("en", "Baz"));

        let patch = l2.patch(&l1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"mode":"Descriptions","patch":[{"op":"replace","path":"/en","value":"Baz"}]})
        );
    }

    #[test]
    fn test_get_rest_api_path() {
        let l = Descriptions::default();
        let id = EntityId::new("Q42").unwrap();
        assert_eq!(
            l.get_my_rest_api_path(&id).unwrap(),
            "/entities/items/Q42/descriptions"
        );
    }

    #[test]
    fn test_header_info_single() {
        let l = Descriptions::default();
        assert_eq!(l.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_serialize() {
        let mut l = Descriptions::default();
        l.insert(LanguageString::new("en", "Foo"));
        l.insert(LanguageString::new("de", "Bar"));
        let s = serde_json::to_string(&l).unwrap();
        assert!(s.contains(r#""en":"Foo""#));
        assert!(s.contains(r#""de":"Bar""#));
    }

    #[test]
    fn test_is_empty() {
        let mut l = Descriptions::default();
        assert!(l.is_empty());
        l.insert(LanguageString::new("en", "Foo"));
        assert!(!l.is_empty());
    }
}
