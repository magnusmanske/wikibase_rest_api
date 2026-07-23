use derive_where::DeriveWhere;
use nutype::nutype;
use serde::ser::{Serialize, SerializeStruct};
use serde_json::{json, Value};

use crate::{
    EditMetadata, EntityId, HeaderInfo, HttpDelete, HttpGet, HttpMisc, HttpPut, RestApi,
    RestApiError, RevisionMatch,
};

/// A validated site (wiki) identifier, e.g. `enwiki`.
#[nutype(
    sanitize(trim, lowercase),
    validate(regex = "^[a-z][a-z0-9_-]*$"),
    derive(Debug, Display, Clone, PartialEq)
)]
pub struct SiteId(String);

impl SiteId {
    /// Validates a raw site ID for use in a REST API path, returning the
    /// sanitized (trimmed, lower-cased) form. Rejects anything that is not a
    /// well-formed site ID — including strings that could inject extra URL
    /// path segments — as [`RestApiError::InvalidSiteId`].
    pub(crate) fn validated(input: &str) -> Result<String, RestApiError> {
        Self::try_new(input)
            .map(Self::into_inner)
            .map_err(|_| RestApiError::InvalidSiteId(input.to_string()))
    }
}

#[derive(DeriveWhere, Debug, Clone)]
#[derive_where(PartialEq)]
pub struct Sitelink {
    wiki: String,
    title: String,
    badges: Vec<String>,
    url: Option<String>,
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl Sitelink {
    /// Create a new sitelink with the given wiki and title
    pub fn new<S1: Into<String>, S2: Into<String>>(wiki: S1, title: S2) -> Sitelink {
        Self::new_complete(wiki.into(), title.into(), Vec::new(), None)
    }

    /// Create a new sitelink with the given wiki, title, badges, and URL
    pub fn new_complete(
        wiki: String,
        title: String,
        badges: Vec<String>,
        url: Option<String>,
    ) -> Sitelink {
        Sitelink {
            wiki,
            title,
            badges,
            url,
            header_info: HeaderInfo::default(),
        }
    }

    /// Create a new sitelink from a JSON object
    pub fn from_json<S: Into<String>>(wiki: S, j: &Value) -> Result<Self, RestApiError> {
        Self::from_json_header_info(wiki, j, HeaderInfo::default())
    }

    fn string_from_json_header_info(j: &Value, key: &str) -> Result<String, RestApiError> {
        j[key]
            .as_str()
            .ok_or(RestApiError::MissingOrInvalidField {
                field: key.to_string(),
                j: j.clone(),
            })
            .map(|s| s.to_string())
    }

    fn badges_from_json_header_info(j: &Value) -> Result<Vec<String>, RestApiError> {
        Ok(j["badges"]
            .as_array()
            .ok_or(RestApiError::MissingOrInvalidField {
                field: "badges".to_string(),
                j: j.clone(),
            })?
            .iter()
            .filter_map(|b| b.as_str())
            .map(|s| s.to_string())
            .collect())
    }

    /// Create a new sitelink from a JSON object with header info
    pub fn from_json_header_info<S: Into<String>>(
        wiki: S,
        j: &Value,
        header_info: HeaderInfo,
    ) -> Result<Self, RestApiError> {
        let wiki = wiki.into().to_string();
        let title = Self::string_from_json_header_info(j, "title")?;
        let badges = Self::badges_from_json_header_info(j)?;
        let url = Some(Self::string_from_json_header_info(j, "url")?);
        let mut ret = Sitelink::new_complete(wiki, title, badges, url);
        ret.header_info = header_info;
        Ok(ret)
    }

    /// Returns the wiki of the sitelink
    pub fn wiki(&self) -> &str {
        &self.wiki
    }

    /// Returns the title of the sitelink
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the badges of the sitelink
    pub const fn badges(&self) -> &Vec<String> {
        &self.badges
    }

    /// Returns the URL of the sitelink
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    fn get_rest_api_path_from_wiki(id: &EntityId, wiki: &str) -> Result<String, RestApiError> {
        let wiki = SiteId::validated(wiki)?;
        Ok(format!(
            "/entities/{group}/{id}/sitelinks/{wiki}",
            group = id.group()?
        ))
    }
}

impl Serialize for Sitelink {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // #lizard forgives the complexity
        let mut fields = 2;
        if self.url.is_some() {
            fields += 1;
        }
        let mut s = serializer.serialize_struct("Sitelink", fields)?;
        s.serialize_field("title", &self.title)?;
        s.serialize_field("badges", &self.badges)?;
        if let Some(url) = &self.url {
            s.serialize_field("url", url)?;
        }
        s.end()
    }
}

impl HttpMisc for Sitelink {
    fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
        Self::get_rest_api_path_from_wiki(id, self.wiki())
    }
}

impl HttpGet for Sitelink {
    async fn get_match(
        id: &EntityId,
        site_id: &str,
        api: &RestApi,
        rm: RevisionMatch,
    ) -> Result<Self, RestApiError> {
        let path = Self::get_rest_api_path_from_wiki(id, site_id)?;
        let (j, header_info) = Self::get_match_internal(api, &path, rm).await?;
        Self::from_json_header_info(site_id, &j, header_info)
    }
}

impl HttpDelete for Sitelink {
    async fn delete_meta(
        &self,
        id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<(), RestApiError> {
        let j = json!({});
        let (j, _revision_id) = self
            .run_json_query(id, reqwest::Method::DELETE, j, api, &em)
            .await?;
        match j.as_str() {
            Some("Sitelink deleted") => Ok(()),
            _ => Err(RestApiError::UnexpectedResponse(j.to_owned())),
        }
    }
}

impl HttpPut for Sitelink {
    async fn put_meta(
        &self,
        id: &EntityId,
        api: &RestApi,
        em: EditMetadata,
    ) -> Result<Sitelink, RestApiError> {
        let j = json!({
            "sitelink": {
                "title": self.title(),
                "badges": self.badges()
            }
        });
        let (j, header_info) = self
            .run_json_query(id, reqwest::Method::PUT, j, api, &em)
            .await?;
        let ret = Self::from_json_header_info(&self.wiki, &j, header_info)?;
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{bearer_token, body_partial_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_site_id_validated() {
        // Well-formed IDs are trimmed and lower-cased; malformed ones are rejected.
        assert_eq!(SiteId::validated(" ENWIKI ").unwrap(), "enwiki");
        assert_eq!(SiteId::validated("be_x_oldwiki").unwrap(), "be_x_oldwiki");
        for bad in ["", "1wiki", "en/sitelinks", "../enwiki", "en wiki"] {
            match SiteId::validated(bad).unwrap_err() {
                RestApiError::InvalidSiteId(input) => assert_eq!(input, bad),
                e => panic!("Wrong error type for {bad:?}: {e:?}"),
            }
        }
    }

    #[test]
    fn test_sitelink_invalid_wiki_path_is_rejected() {
        // A malformed wiki that could inject path segments errors before any request.
        let sl = Sitelink::new("en/../items/Q1", "whatever");
        match sl.get_my_rest_api_path(&EntityId::item("Q42")).unwrap_err() {
            RestApiError::InvalidSiteId(_) => {}
            e => panic!("Wrong error type: {e:?}"),
        }
    }

    #[test]
    fn test_sitelink() {
        let wiki = "enwiki".to_string();
        let title = "Foo".to_string();
        let badges = vec!["Q17437796".to_string()];
        let url = "https://en.wikipedia.org/wiki/Foo".to_string();
        let sitelink = Sitelink::new_complete(
            wiki.clone(),
            title.clone(),
            badges.clone(),
            Some(url.to_string()),
        );
        assert_eq!(sitelink.wiki(), wiki);
        assert_eq!(sitelink.title(), title);
        assert_eq!(sitelink.badges(), &badges);
        assert_eq!(sitelink.url().unwrap(), url);
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_sitelink_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/sitelinks/enwiki");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["sitelinks"]["enwiki"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build()
            .unwrap();

        let sitelink = Sitelink::get(&EntityId::item(id), "enwiki", &api)
            .await
            .unwrap();
        assert_eq!(sitelink.wiki(), "enwiki");
        assert_eq!(sitelink.title(), "Douglas Adams");
        assert_eq!(
            sitelink.url(),
            Some("https://en.wikipedia.org/wiki/Douglas_Adams")
        );
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_sitelink_put() {
        let page_title = "Foo Bar";
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/sitelinks/enwiki");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(body_partial_json(
            json!({"sitelink":{"badges":[],"title":page_title}}),
        ))
        .and(method("PUT"))
        .and(path(&mock_path))
        .and(bearer_token(token))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"badges":[],"title":page_title,"url":"dummy"})),
        )
        .mount(&mock_server)
        .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let sitelink = Sitelink::new("enwiki", page_title);
        let new_sitelink = sitelink.put(&id, &api).await.unwrap();
        assert_eq!(new_sitelink.wiki(), sitelink.wiki());
        assert_eq!(new_sitelink.title(), sitelink.title());
    }

    #[test]
    fn test_sitelink_from_json() {
        let j = json!({
            "title": "Douglas Adams",
            "badges": ["Q17437796"],
            "url": "https://en.wikipedia.org/wiki/Douglas_Adams"
        });
        let sitelink = Sitelink::from_json("enwiki", &j).unwrap();
        assert_eq!(sitelink.wiki(), "enwiki");
        assert_eq!(sitelink.title(), "Douglas Adams");
        assert_eq!(sitelink.badges(), &vec!["Q17437796".to_string()]);
        assert_eq!(
            sitelink.url(),
            Some("https://en.wikipedia.org/wiki/Douglas_Adams")
        );
    }

    #[test]
    fn test_sitelink_serialize() {
        // With a URL present, all three fields are serialized.
        let sitelink = Sitelink::new_complete(
            "enwiki".to_string(),
            "Foo".to_string(),
            vec!["Q17437796".to_string()],
            Some("https://en.wikipedia.org/wiki/Foo".to_string()),
        );
        let v: Value = serde_json::to_value(&sitelink).unwrap();
        assert_eq!(v["title"], json!("Foo"));
        assert_eq!(v["badges"], json!(["Q17437796"]));
        assert_eq!(v["url"], json!("https://en.wikipedia.org/wiki/Foo"));

        // Without a URL, only title and badges are serialized.
        let sitelink_no_url = Sitelink::new("enwiki", "Foo");
        let v_no_url: Value = serde_json::to_value(&sitelink_no_url).unwrap();
        assert_eq!(v_no_url["title"], json!("Foo"));
        assert_eq!(v_no_url["badges"], json!([]));
        assert!(v_no_url.get("url").is_none());
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_sitelink_delete_unexpected_response() {
        let id = "Q42";
        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/sitelinks/enwiki");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("something else")))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let sitelink = Sitelink::new("enwiki", "doesn't matter");
        // An unexpected response body must surface as UnexpectedResponse.
        match sitelink.delete(&id, &api).await.unwrap_err() {
            RestApiError::UnexpectedResponse(j) => assert_eq!(j, json!("something else")),
            e => panic!("Wrong error type: {e:?}"),
        }
    }

    #[tokio::test]
    #[cfg_attr(miri, ignore)]
    async fn test_sitelink_delete() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/sitelinks/enwiki");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("DELETE"))
            .and(path(&mock_path))
            .and(bearer_token(token))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!("Sitelink deleted")))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .with_access_token(token)
            .build()
            .unwrap();

        let id = EntityId::item(id);
        let new_sitelink = Sitelink::new("enwiki", "doesn't matter");
        new_sitelink.delete(&id, &api).await.unwrap();
    }
}
