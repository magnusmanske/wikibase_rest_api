use crate::{entity::EntityType, Language, RestApi, RestApiError};
use nutype::nutype;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[nutype(
    validate(greater_or_equal = 1, less_or_equal = 500),
    derive(Debug, Display, Clone, PartialEq)
)]
pub struct SearchLimit(u16);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultText {
    language: String,
    value: String,
}

impl SearchResultText {
    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResultMatch {
    #[serde(rename = "type")]
    match_type: String,
    language: String,
    text: String,
}

impl SearchResultMatch {
    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn match_type(&self) -> &str {
        &self.match_type
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    id: String,
    #[serde(rename = "display-label")]
    display_label: Option<SearchResultText>,
    description: Option<SearchResultText>,
    #[serde(rename = "match")]
    search_match: SearchResultMatch,
}

impl SearchResult {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub const fn display_label(&self) -> Option<&SearchResultText> {
        self.display_label.as_ref()
    }

    pub const fn description(&self) -> Option<&SearchResultText> {
        self.description.as_ref()
    }

    pub const fn search_match(&self) -> &SearchResultMatch {
        &self.search_match
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchKind {
    Search,
    Suggest,
}

impl SearchKind {
    const fn path_prefix(self) -> &'static str {
        match self {
            SearchKind::Search => "search",
            SearchKind::Suggest => "suggest",
        }
    }
}

#[derive(Debug)]
pub struct Search {
    entity_type: EntityType,
    kind: SearchKind,
    q: String,
    language: Language,
    limit: Option<SearchLimit>,
    offset: Option<usize>,
}

impl Search {
    pub fn items<S: Into<String>>(q: S, language: Language) -> Self {
        Self {
            entity_type: EntityType::Item,
            kind: SearchKind::Search,
            q: q.into(),
            language,
            limit: None,
            offset: None,
        }
    }

    pub fn properties<S: Into<String>>(q: S, language: Language) -> Self {
        Self {
            entity_type: EntityType::Property,
            kind: SearchKind::Search,
            q: q.into(),
            language,
            limit: None,
            offset: None,
        }
    }

    pub fn suggest_items<S: Into<String>>(q: S, language: Language) -> Self {
        Self {
            entity_type: EntityType::Item,
            kind: SearchKind::Suggest,
            q: q.into(),
            language,
            limit: None,
            offset: None,
        }
    }

    pub fn suggest_properties<S: Into<String>>(q: S, language: Language) -> Self {
        Self {
            entity_type: EntityType::Property,
            kind: SearchKind::Suggest,
            q: q.into(),
            language,
            limit: None,
            offset: None,
        }
    }

    pub const fn with_limit(mut self, limit: SearchLimit) -> Self {
        self.limit = Some(limit);
        self
    }

    pub const fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    async fn generate_json_request(&self, api: &RestApi) -> Result<reqwest::Request, RestApiError> {
        let path = self.get_my_rest_api_path();
        let mut params = HashMap::new();
        params.insert("q".to_string(), self.q.to_string());
        params.insert("language".to_string(), self.language.to_string());
        if let Some(limit) = &self.limit {
            params.insert("limit".to_string(), format!("{limit}"));
        }
        if let Some(offset) = &self.offset {
            params.insert("offset".to_string(), offset.to_string());
        }
        let mut request = api
            .wikibase_request_builder(&path, params, reqwest::Method::GET)
            .await?
            .build()?;
        request
            .headers_mut()
            .insert(reqwest::header::CONTENT_TYPE, "application/json".parse()?);
        Ok(request)
    }

    pub async fn get(&self, api: &RestApi) -> Result<Vec<SearchResult>, RestApiError> {
        let request = self.generate_json_request(api).await?;
        let response = api.execute(request).await?;
        let response = self.filter_response_error(response).await?;
        Self::response_to_results(&response)
    }

    fn response_to_results(response: &Value) -> Result<Vec<SearchResult>, RestApiError> {
        // A malformed entry is an error, not silently skipped: callers get the whole list or none.
        response["results"]
            .as_array()
            .ok_or(RestApiError::MissingResults)?
            .iter()
            .map(|result| SearchResult::deserialize(result).map_err(RestApiError::from))
            .collect()
    }

    fn get_my_rest_api_path(&self) -> String {
        format!(
            "/{prefix}/{group}",
            prefix = self.kind.path_prefix(),
            group = self.entity_type.group_name()
        )
    }

    async fn filter_response_error(
        &self,
        response: reqwest::Response,
    ) -> Result<Value, RestApiError> {
        if !response.status().is_success() {
            return Err(RestApiError::from_response(response).await);
        }
        let j: Value = response.json().await?;
        Ok(j)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_get_my_rest_api_path() {
        let en = Language::try_new("en").unwrap();
        assert_eq!(
            Search::items("foo", en.clone()).get_my_rest_api_path(),
            "/search/items"
        );
        assert_eq!(
            Search::properties("foo", en.clone()).get_my_rest_api_path(),
            "/search/properties"
        );
        assert_eq!(
            Search::suggest_items("foo", en.clone()).get_my_rest_api_path(),
            "/suggest/items"
        );
        assert_eq!(
            Search::suggest_properties("foo", en).get_my_rest_api_path(),
            "/suggest/properties"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn test_response_to_results() {
        let v = std::fs::read_to_string("test_data/test_search_response.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let results = Search::response_to_results(&v).unwrap();
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].id(), "Q123");
        assert_eq!(results[1].id(), "Q234");
        assert_eq!(results[2].id(), "Q345");
        assert_eq!(results[3].id(), "Q456");
        assert_eq!(results[0].display_label().unwrap().value(), "potato");
        assert_eq!(results[0].description().unwrap().value(), "staple food");
        assert_eq!(results[1].display_label().unwrap().value(), "potato");
        assert_eq!(
            results[1].description().unwrap().value(),
            "species of plant"
        );
        assert!(results[2].description().is_none());
        assert!(results[3].display_label().is_none());
        assert_eq!(results[0].search_match().match_type(), "label");
        assert_eq!(results[1].search_match().match_type(), "label");
        assert_eq!(results[2].search_match().match_type(), "label");
        assert_eq!(results[3].search_match().match_type(), "description");
    }

    #[test]
    fn test_response_to_results_malformed_is_error() {
        // A result whose `id` is the wrong type must fail, not be silently dropped.
        let v = json!({"results": [{"id": 123}]});
        assert!(Search::response_to_results(&v).is_err());
    }

    #[test]
    fn test_response_to_results_missing_field_is_error() {
        let v = json!({"not_results": []});
        assert!(Search::response_to_results(&v).is_err());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_search() {
        let v = std::fs::read_to_string("test_data/test_search_response.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/search/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let language = Language::try_new("en").unwrap();
        let results = Search::items("potato", language).get(&api).await.unwrap();
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].id(), "Q123");
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_search_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/rest.php/wikibase/v1/search/items"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(json!({"code": "bad", "message": "no"})),
            )
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let language = Language::try_new("en").unwrap();
        match Search::items("potato", language)
            .get(&api)
            .await
            .unwrap_err()
        {
            RestApiError::ApiError { payload, .. } => assert_eq!(payload.code(), "bad"),
            e => panic!("Wrong error type: {e:?}"),
        }
    }
}
