use crate::{prelude::RestApiError, EditMetadata, EntityId, HeaderInfo, RestApi, RevisionMatch};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

#[async_trait]
pub trait HttpMisc {
    fn get_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError>;

    fn add_metadata_to_json(j: &mut Value, em: &EditMetadata) {
        if j.get("tags").is_none() {
            j["tags"] = json!(em.tags());
        }
        if j.get("bot").is_none() {
            j["bot"] = json!(em.bot());
        }
        if j.get("comment").is_none() {
            j["comment"] = json!(em.comment().unwrap_or_default());
        }
    }

    async fn run_json_query(
        &self,
        id: &EntityId,
        method: reqwest::Method,
        j: Value,
        api: &mut RestApi,
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
        api: &mut RestApi,
        em: &EditMetadata,
    ) -> Result<reqwest::Request, RestApiError> {
        Self::add_metadata_to_json(&mut j, em);
        let path = self.get_rest_api_path(id)?;
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
        if !response.status().is_success() {
            return Err(RestApiError::from_response(response).await);
        }
        let header_info = HeaderInfo::from_header(response.headers());
        let j: Value = response.error_for_status()?.json().await?;
        Ok((j, header_info))
    }
}

/// A trait implementing a HTTP GET operation.
#[async_trait]
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
#[async_trait]
pub trait HttpPut: Sized + HttpMisc {
    async fn put_meta(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<Self, RestApiError>;

    async fn put(&self, id: &EntityId, api: &mut RestApi) -> Result<Self, RestApiError> {
        self.put_meta(id, api, EditMetadata::default()).await
    }
}

/// A trait implementing a HTTP DELETE operation.
#[async_trait]
pub trait HttpDelete: Sized + HttpMisc {
    async fn delete_meta(
        &self,
        id: &EntityId,
        api: &mut RestApi,
        em: EditMetadata,
    ) -> Result<(), RestApiError>;

    async fn delete(&self, id: &EntityId, api: &mut RestApi) -> Result<(), RestApiError> {
        self.delete_meta(id, api, EditMetadata::default()).await
    }
}

#[async_trait]
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

#[async_trait]
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
