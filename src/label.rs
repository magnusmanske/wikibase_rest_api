use crate::{
    EditMetadata, EntityId, HeaderInfo, HttpDelete, HttpGet, HttpMisc, HttpPut, LanguageString,
    RestApi, RestApiError, RevisionMatch,
};
use async_trait::async_trait;
use derivative::Derivative;
use reqwest::Request;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Derivative, Debug, Clone)]
#[derivative(PartialEq)]
pub struct Label {
    ls: LanguageString,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl Label {
    /// Constructs a new `Label` object from a language code and a label.
    pub fn new<S1: Into<String>, S2: Into<String>>(language: S1, value: S2) -> Label {
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
    ) -> Result<Request, RestApiError> {
        let path = format!(
            "/entities/{group}/{id}/labels/{language}",
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

impl Deref for Label {
    type Target = LanguageString;

    fn deref(&self) -> &Self::Target {
        &self.ls
    }
}

impl From<LanguageString> for Label {
    fn from(ls: LanguageString) -> Self {
        Self {
            ls,
            header_info: HeaderInfo::default(),
        }
    }
}

impl From<Label> for LanguageString {
    fn from(val: Label) -> Self {
        val.ls
    }
}

impl HttpMisc for Label {
    fn get_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/labels/{language}",
            group = id.group()?,
            language = self.ls.language()
        ))
    }
}

#[async_trait]
impl HttpGet for Label {
    async fn get_match(
        id: &EntityId,
        language: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let request = Self::generate_get_match_request(id, language, api, rm).await?;
        let j: Value = api
            .execute(request)
            .await?
            .error_for_status()?
            .json()
            .await?;
        let s = j
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "Label".into(),
                j: j.to_owned(),
            })?;
        Ok(Self {
            ls: LanguageString::new(language, s),
            header_info: HeaderInfo::default(),
        })
    }
}

#[async_trait]
impl HttpDelete for Label {
    async fn delete_meta(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<(), RestApiError> {
        let j = json!({});
        let (j, _header_info) = self
            .run_json_query(id, reqwest::Method::DELETE, j, api, &em)
            .await?;
        match j.as_str() {
            Some("Label deleted") => Ok(()),
            Some("Description deleted") => Ok(()),
            _ => Err(RestApiError::UnexpectedResponse(j)),
        }
    }
}

#[async_trait]
impl HttpPut for Label {
    async fn put_meta(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError> {
        let j = json!({"label": self.ls.value()});
        let (j, header_info) = self
            .run_json_query(id, reqwest::Method::PUT, j, api, &em)
            .await?;
        let value = j
            .as_str()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "Label".into(),
                j: j.to_owned(),
            })?;
        let mut ret = Self::new(self.language(), value);
        ret.header_info = header_info;
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{bearer_token, body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn test_label_get() {
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/labels/en");
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
        let label = Label::get(&id, "en", &api).await.unwrap();
        assert_eq!(label.language(), "en");
        assert_eq!(label.value(), "Douglas Adams");
    }

    #[tokio::test]
    async fn test_label_put() {
        let label = "Foo bar";
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/labels/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(json!({"label": label})))
            .and(method("PUT"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!(label)))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .set_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let new_label = Label::new("en", label);
        let return_label = new_label.put(&id, &mut api).await.unwrap();
        assert_eq!(return_label.language(), "en");
        assert_eq!(return_label.value(), label);
    }

    #[tokio::test]
    async fn test_label_delete() {
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v0/entities/items/{id}/labels/en");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Label deleted")))
            .mount(&mock_server)
            .await;
        let mut api = RestApi::builder()
            .api(&(mock_server.uri() + "/w/rest.php"))
            .set_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let label = Label::new("en", "");
        let result = label.delete(&id, &mut api).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_into_language_string() {
        let label = Label::new("en", "Foo bar");
        let ls: LanguageString = label.into();
        assert_eq!(ls.language(), "en");
        assert_eq!(ls.value(), "Foo bar");
    }

    #[test]
    fn test_from_language_string() {
        let ls = LanguageString::new("en", "Foo bar");
        let label = Label::from(ls);
        assert_eq!(label.language(), "en");
        assert_eq!(label.value(), "Foo bar");
    }
}
