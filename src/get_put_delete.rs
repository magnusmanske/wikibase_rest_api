use crate::{prelude::RestApiError, EditMetadata, EntityId, HeaderInfo, RestApi, RevisionMatch};
use reqwest::Request;
use serde_json::{json, Value};
use std::collections::HashMap;

pub trait HttpMisc {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Self::get_rest_api_path(id)
    }

    fn get_rest_api_path(id: &EntityId) -> Result<String, RestApiError> {
        // A type that relies on the default `get_my_rest_api_path` must implement this;
        // returning an error (rather than panicking) keeps the misuse recoverable.
        Err(RestApiError::PathNotImplemented(format!(
            "{}::get_rest_api_path for {id}",
            std::any::type_name::<Self>()
        )))
    }

    fn add_metadata_to_json(j: &mut Value, em: &EditMetadata) {
        if j.get("tags").is_none() {
            j["tags"] = json!(em.tags());
        }
        if j.get("bot").is_none() {
            j["bot"] = json!(em.bot());
        }
        if j.get("comment").is_none() {
            if let Some(comment) = em.comment() {
                j["comment"] = json!(comment);
            }
        }
    }

    async fn get_match_internal(
        api: &RestApi,
        path: &str,
        rm: RevisionMatch,
    ) -> Result<(Value, HeaderInfo), RestApiError> {
        let mut request = api
            .wikibase_request_builder(path, HashMap::new(), reqwest::Method::GET)
            .await?
            .build()?;
        rm.modify_headers(request.headers_mut())?;
        Self::api_execute(api, request).await
    }

    async fn api_execute(
        api: &RestApi,
        request: Request,
    ) -> Result<(Value, HeaderInfo), RestApiError> {
        let response = api.execute(request).await?;
        Self::parse_response(response).await
    }

    /// Parses a response into its JSON body and header info. Any non-success status
    /// is converted into a `RestApiError::ApiError` carrying the server error payload,
    /// so all operations report failures through the same variant.
    async fn parse_response(
        response: reqwest::Response,
    ) -> Result<(Value, HeaderInfo), RestApiError> {
        if !response.status().is_success() {
            return Err(RestApiError::from_response(response).await);
        }
        let header_info = HeaderInfo::from_header(response.headers());
        let j: Value = response.json().await?;
        Ok((j, header_info))
    }

    async fn run_json_query(
        &self,
        id: &EntityId,
        method: reqwest::Method,
        j: Value,
        api: &RestApi,
        em: &EditMetadata,
    ) -> Result<(Value, HeaderInfo), RestApiError> {
        let request = self.generate_json_request(id, method, j, api, em).await?;
        let response = api.execute(request).await?;
        self.filter_response_error(response).await
    }

    async fn generate_json_request(
        &self,
        id: &EntityId,
        method: reqwest::Method,
        mut j: Value,
        api: &RestApi,
        em: &EditMetadata,
    ) -> Result<reqwest::Request, RestApiError> {
        Self::add_metadata_to_json(&mut j, em);
        let path = self.get_my_rest_api_path(id)?;
        let content_type = match method {
            reqwest::Method::PATCH => "application/json-patch+json",
            _ => "application/json",
        }
        .parse()?;
        let mut request = api
            .wikibase_request_builder(&path, HashMap::new(), method)
            .await?
            .build()?;
        request
            .headers_mut()
            .insert(reqwest::header::CONTENT_TYPE, content_type);
        em.revision_match().modify_headers(request.headers_mut())?;
        *request.body_mut() = Some(format!("{j}").into());
        Ok(request)
    }

    async fn filter_response_error(
        &self,
        response: reqwest::Response,
    ) -> Result<(Value, HeaderInfo), RestApiError> {
        Self::parse_response(response).await
    }
}

/// A trait implementing a HTTP GET operation.
pub trait HttpGet: Sized + HttpMisc {
    async fn get_match(
        id: &EntityId,
        part_id: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError>;

    async fn get(id: &EntityId, part_id: &str, api: &RestApi) -> Result<Self, RestApiError> {
        Self::get_match(id, part_id, api, RevisionMatch::default()).await
    }
}

/// A trait implementing a HTTP PUT operation.
pub trait HttpPut: Sized + HttpMisc {
    async fn put_meta(
        &self,
        id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError>;

    async fn put(&self, id: &EntityId, api: &RestApi) -> Result<Self, RestApiError> {
        self.put_meta(id, api, EditMetadata::default()).await
    }
}

/// A trait implementing a HTTP DELETE operation.
pub trait HttpDelete: Sized + HttpMisc {
    async fn delete_meta(
        &self,
        id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<(), RestApiError>;

    async fn delete(&self, id: &EntityId, api: &RestApi) -> Result<(), RestApiError> {
        self.delete_meta(id, api, EditMetadata::default()).await
    }
}

pub trait HttpGetEntity: Sized + HttpMisc {
    async fn get_match(
        id: &EntityId,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError>
    where
        Self: Sized;

    async fn get(id: &EntityId, api: &RestApi) -> Result<Self, RestApiError>
    where
        Self: Sized,
    {
        Self::get_match(id, api, RevisionMatch::default()).await
    }
}

pub trait HttpGetEntityWithFallback: Sized + HttpMisc {
    async fn get_match_with_fallback(
        id: &EntityId,
        language: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError>;

    async fn get_with_fallback(
        id: &EntityId,
        language: &str,
        api: &RestApi,
    ) -> Result<Self, RestApiError>
    where
        Self: Sized,
    {
        Self::get_match_with_fallback(id, language, api, RevisionMatch::default()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::Sitelinks;

    use super::*;
    use wiremock::matchers::{method as wm_method, path as wm_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // A type relying entirely on the default `HttpMisc` methods.
    struct NoPath;
    impl HttpMisc for NoPath {}

    #[test]
    fn test_get_rest_api_path_default_errors() {
        let id = EntityId::item("Q1");
        assert!(matches!(
            NoPath::get_rest_api_path(&id),
            Err(RestApiError::PathNotImplemented(_))
        ));
        // The default `get_my_rest_api_path` delegates to `get_rest_api_path`.
        assert!(NoPath.get_my_rest_api_path(&id).is_err());
    }

    #[test]
    fn test_add_metadata_to_json_with_comment() {
        let mut em = EditMetadata::default();
        em.set_comment(Some("a comment".to_string()));
        em.set_bot(true);
        em.set_tags(vec!["tag1".to_string()]);
        let mut j = json!({});
        Sitelinks::add_metadata_to_json(&mut j, &em);
        assert_eq!(j["comment"], "a comment");
        assert_eq!(j["bot"], true);
        assert_eq!(j["tags"], json!(["tag1"]));
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_get_match_internal() {
        // Exercises get_match_internal -> api_execute -> parse_response (success path).
        let mock_server = MockServer::start().await;
        Mock::given(wm_method("GET"))
            .and(wm_path(
                "/w/rest.php/wikibase/v1/entities/items/Q1/sitelinks",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"enwiki": {}})))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let (j, _header_info) = Sitelinks::get_match_internal(
            &api,
            "/entities/items/Q1/sitelinks",
            RevisionMatch::default(),
        )
        .await
        .unwrap();
        assert!(j.is_object());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_run_json_query() {
        // Exercises run_json_query -> generate_json_request (PUT branch) -> filter_response_error.
        let mock_server = MockServer::start().await;
        Mock::given(wm_method("PUT"))
            .and(wm_path(
                "/w/rest.php/wikibase/v1/entities/items/Q1/sitelinks",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let sl = Sitelinks::default();
        let (j, _header_info) = sl
            .run_json_query(
                &EntityId::item("Q1"),
                reqwest::Method::PUT,
                json!({}),
                &api,
                &EditMetadata::default(),
            )
            .await
            .unwrap();
        assert!(j.is_object());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_filter_response_error() {
        let sl = Sitelinks::default();
        let response = reqwest::Response::from(http::Response::new("body text"));
        let result = sl.filter_response_error(response).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_filter_response_error2() {
        let sl = Sitelinks::default();
        let response = reqwest::Response::from(
            http::Response::builder()
                .status(400)
                .body(r#"{"code":"foo","message":"bar"}"#)
                .unwrap(),
        );
        let result = sl.filter_response_error(response).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "ApiError: 400 Bad Request Bad Request / RestApiErrorPayload { code: \"foo\", message: \"bar\", context: {} }"
        );
    }
}
