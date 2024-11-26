use crate::{
    aliases_patch::AliasesPatch, language_strings_patch::LanguageStringsPatch, EntityId, FromJson,
    HeaderInfo, HttpGetEntity, HttpMisc, LanguageString, RestApi, RestApiError, RevisionMatch,
};
use async_trait::async_trait;
use derivative::Derivative;
use serde::ser::{Serialize, SerializeMap};
use serde_json::{json, Value};
use std::collections::HashMap;

pub trait LanguageStrings {
    fn insert(&mut self, ls: LanguageString);
    fn has_language<S: Into<String>>(&self, language: S) -> bool;
}

#[derive(Derivative, Debug, Clone, Default)]
#[derivative(PartialEq)]
pub struct LanguageStringsSingle {
    ls: HashMap<String, String>,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl HttpMisc for LanguageStringsSingle {
    fn get_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/labels",
            group = id.group()?
        ))
    }
}

#[async_trait]
impl HttpGetEntity for LanguageStringsSingle {
    async fn get_match(
        id: &EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let path = format!("/entities/{group}/{id}/labels", group = id.group()?);
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        let response = api.execute(request).await?;
        let header_info = HeaderInfo::from_header(response.headers());
        let j: Value = response.error_for_status()?.json().await?;
        Self::from_json_header_info(&j, header_info)
    }
}

impl LanguageStringsSingle {
    /// Returns the value for a language
    pub fn get_lang<S: Into<String>>(&self, language: S) -> Option<&str> {
        self.ls.get(&language.into()).map(|s| s.as_str())
    }

    /// Generates a patch to transform `other` into `self`
    pub fn patch_labels(&self, other: &Self) -> Result<LanguageStringsPatch, RestApiError> {
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = LanguageStringsPatch::labels_from_json(&json!(patch))?;
        Ok(patch)
    }

    /// Generates a patch to transform `other` into `self`
    pub fn patch_descriptions(&self, other: &Self) -> Result<LanguageStringsPatch, RestApiError> {
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = LanguageStringsPatch::descriptions_from_json(&json!(patch))?;
        Ok(patch)
    }

    /// Returns the number of labels/languages
    pub fn len(&self) -> usize {
        self.ls.len()
    }

    /// Returns true if there are no labels/languages
    pub fn is_empty(&self) -> bool {
        self.ls.is_empty()
    }
}

impl FromJson for LanguageStringsSingle {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let ls = j
            .as_object()
            .ok_or_else(|| RestApiError::WrongType {
                field: "LanguageStringSingle".to_string(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|(language, value)| {
                let value = value
                    .as_str()
                    .ok_or_else(|| RestApiError::MissingOrInvalidField {
                        field: "LanguageStringSingle".into(),
                        j: value.to_owned(),
                    })?;
                Ok((language.to_owned(), value.to_string()))
            })
            .collect::<Result<HashMap<String, String>, RestApiError>>()?;
        let ret = Self { ls, header_info };
        Ok(ret)
    }
}

impl LanguageStrings for LanguageStringsSingle {
    fn has_language<S: Into<String>>(&self, language: S) -> bool {
        self.ls.contains_key(&language.into())
    }

    fn insert(&mut self, ls: LanguageString) {
        self.ls
            .insert(ls.language().to_string(), ls.value().to_string());
    }
}

impl Serialize for LanguageStringsSingle {
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

// ______________________________________________________________________________________________________________

#[derive(Derivative, Debug, Clone, Default)]
#[derivative(PartialEq)]
pub struct LanguageStringsMultiple {
    ls: HashMap<String, Vec<String>>,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl LanguageStringsMultiple {
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
}

impl FromJson for LanguageStringsMultiple {
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

impl LanguageStrings for LanguageStringsMultiple {
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

impl Serialize for LanguageStringsMultiple {
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

    #[test]
    fn test_language_strings_single() {
        let j = json!({
            "en": "Hello",
            "de": "Hallo",
        });
        let ls = LanguageStringsSingle::from_json(&j).unwrap();
        assert_eq!(ls.get_lang("en"), Some("Hello"));
        assert_eq!(ls.get_lang("de"), Some("Hallo"));
        assert_eq!(ls.get_lang("fr"), None);
    }

    #[test]
    fn test_language_strings_multiple() {
        let j = json!({
            "en": ["Hello", "Hi"],
            "de": ["Hallo", "Hi"],
        });
        let ls = LanguageStringsMultiple::from_json(&j).unwrap();
        assert_eq!(ls.get_lang("en"), vec!["Hello", "Hi"]);
        assert_eq!(ls.get_lang("de"), vec!["Hallo", "Hi"]);
        assert!(ls.get_lang("fr").is_empty());
    }

    #[test]
    fn test_language_strings_insert() {
        let mut ls = LanguageStringsSingle::default();
        ls.insert(LanguageString::new("en", "Hello"));
        ls.insert(LanguageString::new("de", "Hallo"));
        ls.insert(LanguageString::new("en", "Hi"));
        assert_eq!(ls.get_lang("en"), Some("Hi"));
        assert_eq!(ls.get_lang("de"), Some("Hallo"));
    }

    #[test]
    fn test_language_strings_multiple_insert() {
        let mut ls = LanguageStringsMultiple::default();
        ls.insert(LanguageString::new("en", "Hello"));
        ls.insert(LanguageString::new("de", "Hallo"));
        ls.insert(LanguageString::new("en", "Hi"));
        assert_eq!(ls.get_lang("en"), vec!["Hello", "Hi"]);
        assert_eq!(ls.get_lang("de"), vec!["Hallo"]);
    }

    #[tokio::test]
    async fn test_language_strings_single_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();

        let mock_path = "/w/rest.php/wikibase/v0/entities/items/Q42/labels";
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["labels"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        let id = EntityId::new("Q42").unwrap();
        let ls = LanguageStringsSingle::get(&id, &api).await.unwrap();
        assert_eq!(ls.get_lang("en"), Some("Douglas Adams"));
    }

    #[test]
    fn test_patch_labels() {
        let mut l1 = LanguageStringsSingle::default();
        l1.insert(LanguageString::new("en", "Foo"));
        l1.insert(LanguageString::new("de", "Bar"));
        let mut l2 = l1.clone();
        l2.insert(LanguageString::new("en", "Baz"));

        let patch = l2.patch_labels(&l1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"mode":"Labels","patch":[{"op":"replace","path":"/en","value":"Baz"}]})
        );
    }

    #[test]
    fn test_patch_descriptions() {
        let mut l1 = LanguageStringsSingle::default();
        l1.insert(LanguageString::new("en", "Foo"));
        l1.insert(LanguageString::new("de", "Bar"));
        let mut l2 = l1.clone();
        l2.insert(LanguageString::new("en", "Baz"));

        let patch = l2.patch_descriptions(&l1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"mode":"Descriptions","patch":[{"op":"replace","path":"/en","value":"Baz"}]})
        );
    }

    #[test]
    fn test_patch_aliases() {
        let mut l1 = LanguageStringsMultiple::default();
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
    fn test_get_rest_api_path() {
        let l = LanguageStringsSingle::default();
        let id = EntityId::new("Q42").unwrap();
        assert_eq!(
            l.get_rest_api_path(&id).unwrap(),
            "/entities/items/Q42/labels"
        );
    }

    #[test]
    fn test_header_info_single() {
        let l = LanguageStringsSingle::default();
        assert_eq!(l.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_header_info_multiple() {
        let l = LanguageStringsMultiple::default();
        assert_eq!(l.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_serialize() {
        let mut l = LanguageStringsSingle::default();
        l.insert(LanguageString::new("en", "Foo"));
        l.insert(LanguageString::new("de", "Bar"));
        let s = serde_json::to_string(&l).unwrap();
        assert!(s.contains(r#""en":"Foo""#));
        assert!(s.contains(r#""de":"Bar""#));
    }

    #[test]
    fn test_serialize2() {
        let mut l = LanguageStringsMultiple::default();
        l.insert(LanguageString::new("en", "Foo"));
        l.insert(LanguageString::new("en", "Bar"));
        l.insert(LanguageString::new("de", "Baz"));
        let s = serde_json::to_string(&l).unwrap();
        assert!(s.contains(r#""en":["Foo","Bar"]"#));
        assert!(s.contains(r#""de":["Baz"]"#));
    }
}
