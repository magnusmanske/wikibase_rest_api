use crate::HttpGetEntityWithFallback;
use crate::{
    get_put_delete::HttpMisc, EditMetadata, EntityId, HeaderInfo, HttpDelete, HttpGet, HttpPut,
    LanguageString, RestApi, RestApiError, RevisionMatch,
};
use async_trait::async_trait;
use derivative::Derivative;
use reqwest::Request;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Derivative, Debug, Clone)]
#[derivative(PartialEq)]
pub struct Description {
    ls: LanguageString,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl Description {
    /// Constructs a new `Description` object from a language code and a description.
    pub fn new<S1: Into<String>, S2: Into<String>>(language: S1, value: S2) -> Description {
        Self {
            ls: LanguageString::new(language, value),
            header_info: HeaderInfo::default(),
        }
    }

    async fn generate_get_match_request(
        id: &EntityId,
        language: &str,
        api: &RestApi,
        rm: RevisionMatch,
        mode: &str,
    ) -> Result<Request, RestApiError> {
        let path = format!(
            "/entities/{group}/{id}/{mode}/{language}",
            group = id.group()?
        );
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        Ok(request)
    }
}

impl Deref for Description {
    type Target = LanguageString;

    fn deref(&self) -> &Self::Target {
        &self.ls
    }
}

impl From<LanguageString> for Description {
    fn from(ls: LanguageString) -> Self {
        Self {
            ls,
            header_info: HeaderInfo::default(),
        }
    }
}

impl From<Description> for LanguageString {
    fn from(val: Description) -> Self {
        val.ls
    }
}

impl HttpMisc for Description {
    fn get_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/descriptions/{language}",
            group = id.group()?,
            language = self.ls.language()
        ))
    }
}

#[async_trait]
impl HttpGetEntityWithFallback for Description {
    async fn get_match_with_fallback(
        id: &EntityId,
        language: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let request = Self::generate_get_match_request(
            id,
            language,
            api,
            rm,
            "descriptions_with_language_fallback",
        )
        .await?;
        let j: Value = api
            .execute(request)
            .await?
            .error_for_status()?
            .json()
            .await?;
        let s = j
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "Descriptions".into(),
                j: j.to_owned(),
            })?;
        Ok(Self {
            ls: LanguageString::new(language, s),
            header_info: HeaderInfo::default(),
        })
    }
}

#[async_trait]
impl HttpGet for Description {
    async fn get_match(
        id: &EntityId,
        language: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let request =
            Self::generate_get_match_request(id, language, api, rm, "descriptions").await?;
        let j: Value = api
            .execute(request)
            .await?
            .error_for_status()?
            .json()
            .await?;
        let s = j
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "Description".into(),
                j: j.to_owned(),
            })?;
        Ok(Self {
            ls: LanguageString::new(language, s),
            header_info: HeaderInfo::default(),
        })
    }
}

#[async_trait]
impl HttpDelete for Description {
    async fn delete_meta(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<(), RestApiError> {
        let j = json!({});
        self.run_json_query(id, reqwest::Method::DELETE, j, api, &em)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl HttpPut for Description {
    async fn put_meta(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError> {
        let j = json!({"description": self.ls.value()});
        let (j, header_info) = self
            .run_json_query(id, reqwest::Method::PUT, j, api, &em)
            .await?;
        let value = j
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "Description".into(),
                j: j.to_owned(),
            })?;
        let mut ret = Self::new(self.language(), value);
        ret.header_info = header_info;
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{bearer_token, body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_descriptions_get_match_with_fallback() {
        let id = "Q42";
        let mock_path = format!(
            "/w/rest.php/wikibase/v0/entities/items/{id}/descriptions_with_language_fallback/foo"
        );
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Douglas Adams")))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let description = Description::get_with_fallback(&id, "foo", &api)
            .await
            .unwrap();
        assert_eq!(description.language(), "foo");
        assert_eq!(description.value(), "Douglas Adams");
    }

    #[tokio::test]
    async fn test_description_get() {
        let id = "Q42";
        let mock_description = "Foo bar baz";
        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/descriptions/en");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_description))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let description = Description::get(&id, "en", &api).await.unwrap();
        assert_eq!(description.language(), "en");
        assert_eq!(description.value(), mock_description);
    }

    #[tokio::test]
    async fn test_description_put() {
        let description = "Foo bar baz";
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/descriptions/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(json!({"description": description})))
            .and(method("PUT"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(description)))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .set_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let new_description = Description::new("en", description);
        let return_description = new_description.put(&id, &mut api).await.unwrap();
        assert_eq!(return_description.language(), "en");
        assert_eq!(return_description.value(), description);
    }

    #[tokio::test]
    async fn test_description_delete() {
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/descriptions/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Description deleted")))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .set_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let description = Description::new("en", "");
        let result = description.delete(&id, &mut api).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_from() {
        let ls = LanguageString::new("en", "Foo bar baz");
        let description = Description::from(ls);
        assert_eq!(description.language(), "en");
        assert_eq!(description.value(), "Foo bar baz");
    }

    #[test]
    fn test_into() {
        let description = Description::new("en", "Foo bar baz");
        let ls: LanguageString = description.into();
        assert_eq!(ls.language(), "en");
        assert_eq!(ls.value(), "Foo bar baz");
    }
}
