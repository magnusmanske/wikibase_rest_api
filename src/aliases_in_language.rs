use crate::{
    EditMetadata, EntityId, HeaderInfo, HttpGet, HttpMisc, RestApi, RestApiError, RevisionMatch,
};
use async_trait::async_trait;
use derivative::Derivative;
use reqwest::{Response, StatusCode};
use serde_json::{json, Value};
use std::collections::HashMap;

/// A group of aliases in a specific language.
#[derive(Derivative, Debug, Clone)]
#[derivative(PartialEq)]
pub struct AliasesInLanguage {
    language: String,
    values: Vec<String>,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl AliasesInLanguage {
    /// Constructs a new `Aliases` object from a language code and a list of aliases.
    pub fn new<S: Into<String>>(language: S, values: Vec<String>) -> Self {
        Self {
            language: language.into(),
            values,
            header_info: HeaderInfo::default(),
        }
    }

    /// Constructs a new `Aliases` object from a language code and a JSON array of (string) aliases.
    pub fn from_json<S: Into<String>>(language: S, j: &Value) -> Result<Self, RestApiError> {
        Self::from_json_header_info(language, j, HeaderInfo::default())
    }

    /// Constructs a new `Aliases` object from a language code and a JSON array of (string) aliases.
    pub fn from_json_header_info<S: Into<String>>(
        language: S,
        j: &Value,
        header_info: HeaderInfo,
    ) -> Result<Self, RestApiError> {
        let language = language.into();
        if language.trim().is_empty() {
            return Err(RestApiError::EmptyValue("Language".into()));
        }
        let aliases = j
            .as_array()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "Aliases".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|v| {
                Ok(v.as_str()
                    .ok_or_else(|| RestApiError::MissingOrInvalidField {
                        field: "Aliases".into(),
                        j: v.to_owned(),
                    })?
                    .to_string())
            })
            .collect::<Result<Vec<String>, RestApiError>>()?;
        Ok(Self {
            language,
            values: aliases,
            header_info,
        })
    }

    /// Adds an alias to the list of aliases (only if it is not already present).
    pub fn push(&mut self, alias: String) {
        if !self.values.contains(&alias) {
            self.values.push(alias);
        }
    }

    /// Returns the list of aliases.
    pub const fn values(&self) -> &Vec<String> {
        &self.values
    }

    /// Returns the number of aliases.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if the list of aliases is empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Returns the language code of the aliases.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Adds one or more aliases to the list of aliases.
    pub async fn post(&self, id: &EntityId, api: &mut RestApi) -> Result<Self, RestApiError> {
        self.post_meta(id, api, EditMetadata::default()).await
    }

    /// Adds one or more aliases to the list of aliases, using conditions and edit metadata.
    pub async fn post_meta(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError> {
        let j = json!({"aliases": self.values});
        let (j, header_info) = self
            .run_json_query(id, reqwest::Method::POST, j, api, &em)
            .await?;
        Self::from_json_header_info(&self.language, &j, header_info)
    }

    /// Returns the header information of the last HTTP response (revision ID, last modified).
    pub const fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    async fn check_get_match_response(
        language: &str,
        response: Response,
    ) -> Result<Self, RestApiError> {
        let header_info = HeaderInfo::from_header(response.headers());
        let j: Value = match response.error_for_status() {
            Ok(response) => response.json().await?,
            Err(e) => {
                if e.status() == Some(StatusCode::NOT_FOUND) {
                    json!([])
                } else {
                    return Err(e.into());
                }
            }
        };
        Self::from_json_header_info(language, &j, header_info)
    }
}

impl HttpMisc for AliasesInLanguage {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/aliases/{language}",
            group = id.group()?,
            language = self.language
        ))
    }
}

#[async_trait]
impl HttpGet for AliasesInLanguage {
    async fn get_match(
        id: &EntityId,
        language: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let path = format!(
            "/entities/{group}/{id}/aliases/{language}",
            group = id.group()?
        );
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        let response = api.execute(request).await?;
        Self::check_get_match_response(language, response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{bearer_token, body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_aliases_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id_q42 = v["id"].as_str().unwrap();

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id_q42}/aliases/en");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["aliases"]["en"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let id = EntityId::item("Q42");
        let aliases = AliasesInLanguage::get(&id, "en", &api).await.unwrap();
        assert!(aliases.values.contains(&"Douglas Noël Adams".to_string()));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_aliases_post() {
        // #lizard forgives the complexity
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();
        let new_alias = "Foo bar baz";
        let mut new_aliases = v["aliases"]["en"].to_owned();
        new_aliases.as_array_mut().unwrap().push(json!(new_alias));

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/aliases/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["aliases"]["en"]))
            .mount(&mock_server)
            .await;
        Mock::given(body_partial_json(json!({"aliases": [new_alias]})))
            .and(method("POST"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(&new_aliases))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build();

        let id2 = EntityId::item("Q42");
        let aliases = AliasesInLanguage::get(&id2, "en", &api).await.unwrap();
        let new_aliases2 = AliasesInLanguage::new("en", vec![new_alias.to_string()]);
        let new_aliases2 = new_aliases2.post(&id2, &mut api).await.unwrap();
        assert_eq!(new_aliases2.len(), aliases.len() + 1);
        assert!(new_aliases2.values.contains(&new_alias.to_string()));

        // Check non-existing item
        let id3 = EntityId::item("Q12345");
        assert_eq!(
            AliasesInLanguage::get(&id3, "en", &api)
                .await
                .unwrap()
                .len(),
            0
        );
    }

    #[test]
    fn test_aliases_new() {
        let aliases = AliasesInLanguage::new("en", vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(aliases.language(), "en");
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn test_aliases_from_json() {
        let j = json!(["Foo", "Bar"]);
        let aliases = AliasesInLanguage::from_json("en", &j).unwrap();
        assert_eq!(aliases.language(), "en");
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn test_aliases_push() {
        let mut aliases = AliasesInLanguage::new("en", vec!["Foo".to_string()]);
        aliases.push("Bar".to_string());
        aliases.push("Foo".to_string());
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn test_aliases_values() {
        let aliases = AliasesInLanguage::new("en", vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(
            aliases.values(),
            &vec!["Foo".to_string(), "Bar".to_string()]
        );
    }

    #[test]
    fn test_aliases_len() {
        let aliases = AliasesInLanguage::new("en", vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(aliases.len(), 2);
    }

    #[test]
    fn test_aliases_header_info() {
        let aliases = AliasesInLanguage::new("en", vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(aliases.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_from_json_header_info() {
        let j = json!(12345);

        let aliases = AliasesInLanguage::from_json("", &j).unwrap_err();
        assert_eq!(aliases.to_string(), "Empty value: Language");

        let aliases2 = AliasesInLanguage::from_json("en", &j).unwrap_err();
        assert_eq!(aliases2.to_string(), "Missing field Aliases: 12345");
    }

    #[test]
    fn test_is_empty() {
        let aliases = AliasesInLanguage::new("en", vec!["Foo".to_string(), "Bar".to_string()]);
        assert!(!aliases.is_empty());
        let aliases2 = AliasesInLanguage::new("en", vec![]);
        assert!(aliases2.is_empty());
    }
}
