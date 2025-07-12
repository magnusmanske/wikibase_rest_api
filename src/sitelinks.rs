use crate::{
    sitelinks_patch::SitelinksPatch, EntityId, FromJson, HeaderInfo, HttpGetEntity, HttpMisc,
    RestApi, RestApiError, RevisionMatch, Sitelink,
};
use async_trait::async_trait;
use derive_where::DeriveWhere;
use serde::ser::{Serialize, SerializeMap};
use serde_json::{json, Value};

#[derive(DeriveWhere, Debug, Clone, Default)]
#[derive_where(PartialEq)]
pub struct Sitelinks {
    sitelinks: Vec<Sitelink>,
    #[derive_where(skip)]
    header_info: HeaderInfo,
}

impl HttpMisc for Sitelinks {
    fn get_rest_api_path(id: &EntityId) -> Result<String, RestApiError> {
        Ok(format!(
            "/entities/{group}/{id}/sitelinks",
            group = id.group()?
        ))
    }
}

impl FromJson for Sitelinks {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(json: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let sitelinks = json
            .as_object()
            .ok_or(RestApiError::MissingOrInvalidField {
                field: "Sitelinks".to_string(),
                j: json.clone(),
            })?
            .iter()
            .map(|(wiki, j)| Sitelink::from_json(wiki, j))
            .collect::<Result<Vec<Sitelink>, RestApiError>>()?;
        Ok(Sitelinks {
            sitelinks,
            header_info,
        })
    }
}

#[async_trait]
impl HttpGetEntity for Sitelinks {
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

impl Sitelinks {
    /// Returns the sitelinks
    pub const fn sitelinks(&self) -> &Vec<Sitelink> {
        &self.sitelinks
    }

    /// Returns the sitelink for a given wiki
    pub fn get_wiki<S: Into<String>>(&self, wiki: S) -> Option<&Sitelink> {
        let wiki = wiki.into();
        self.sitelinks.iter().find(|s| s.wiki() == wiki)
    }

    /// Sets the sitelink for a given wiki
    pub fn set_wiki(&mut self, sitelink: Sitelink) {
        self.sitelinks.retain(|s| s.wiki() != sitelink.wiki());
        self.sitelinks.push(sitelink);
    }

    /// Deletes the sitelink for a given wiki
    pub fn remove_wiki<S: Into<String>>(&mut self, wiki: S) {
        let wiki = wiki.into();
        self.sitelinks.retain(|s| s.wiki() != wiki);
    }

    /// Returns the number of sitelinks
    pub fn len(&self) -> usize {
        self.sitelinks.len()
    }

    /// Returns true if there are no sitelinks
    pub fn is_empty(&self) -> bool {
        self.sitelinks.is_empty()
    }

    /// Generates a patch to transform `other` into `self`
    pub fn patch(&self, other: &Self) -> Result<SitelinksPatch, RestApiError> {
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = SitelinksPatch::from_json(&json!(patch))?;
        Ok(patch)
    }
}

impl Serialize for Sitelinks {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.sitelinks.len()))?;
        for sl in &self.sitelinks {
            s.serialize_entry(sl.wiki(), sl)?;
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
    async fn test_sitelinks_get() {
        let v = std::fs::read_to_string("test_data/Q42.json").unwrap();
        let v: Value = serde_json::from_str(&v).unwrap();
        let id = v["id"].as_str().unwrap();

        let mock_path = format!("/w/rest.php/wikibase/v1/entities/items/{id}/sitelinks");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v["sitelinks"]))
            .mount(&mock_server)
            .await;
        let api = RestApi::builder(&(mock_server.uri() + "/w/rest.php"))
            .unwrap()
            .build();

        let sitelinks = Sitelinks::get(&EntityId::item("Q42"), &api).await.unwrap();
        assert_eq!(sitelinks.sitelinks.len(), 122);
        assert_eq!(
            sitelinks.get_wiki("enwiki").unwrap().title(),
            "Douglas Adams"
        );
    }

    #[test]
    fn test_sitelinks_json() {
        let sitelinks = Sitelinks {
            sitelinks: vec![
                Sitelink::new_complete(
                    "enwiki".to_string(),
                    "Douglas Adams".to_string(),
                    vec![],
                    Some("https://en.wikipedia.org/wiki/Douglas_Adams".to_string()),
                ),
                Sitelink::new_complete(
                    "dewiki".to_string(),
                    "Douglas Adams".to_string(),
                    vec![],
                    Some("https://de.wikipedia.org/wiki/Douglas_Adams".to_string()),
                ),
            ],
            header_info: HeaderInfo::default(),
        };
        let j = json!(sitelinks);
        assert_eq!(j["enwiki"]["title"].as_str().unwrap(), "Douglas Adams");
        assert_eq!(j["dewiki"]["title"].as_str().unwrap(), "Douglas Adams");
    }

    #[test]
    fn test_sitelinks_set_wiki() {
        let mut sitelinks = Sitelinks::default();
        sitelinks.set_wiki(Sitelink::new_complete(
            "enwiki".to_string(),
            "Douglas Adams".to_string(),
            vec![],
            Some("https://en.wikipedia.org/wiki/Douglas_Adams".to_string()),
        ));
        assert_eq!(
            sitelinks.get_wiki("enwiki").unwrap().title(),
            "Douglas Adams"
        );
        sitelinks.set_wiki(Sitelink::new_complete(
            "enwiki".to_string(),
            "Douglas Noël Adams".to_string(),
            vec![],
            Some("https://en.wikipedia.org/wiki/Douglas_Adams".to_string()),
        ));
        assert_eq!(
            sitelinks.get_wiki("enwiki").unwrap().title(),
            "Douglas Noël Adams"
        );
    }

    #[test]
    fn test_sitelinks_get_wiki() {
        let sitelinks = Sitelinks {
            sitelinks: vec![
                Sitelink::new_complete(
                    "enwiki".to_string(),
                    "Douglas Adams".to_string(),
                    vec![],
                    Some("https://en.wikipedia.org/wiki/Douglas_Adams".to_string()),
                ),
                Sitelink::new_complete(
                    "dewiki".to_string(),
                    "Douglas Adams".to_string(),
                    vec![],
                    Some("https://de.wikipedia.org/wiki/Douglas_Adams".to_string()),
                ),
            ],
            header_info: HeaderInfo::default(),
        };
        assert_eq!(
            sitelinks.get_wiki("enwiki").unwrap().title(),
            "Douglas Adams"
        );
        assert_eq!(
            sitelinks.get_wiki("dewiki").unwrap().title(),
            "Douglas Adams"
        );
        assert!(sitelinks.get_wiki("frwiki").is_none());
    }

    #[test]
    fn test_patch() {
        let mut s1 = Sitelinks::default();
        s1.set_wiki(Sitelink::new("enwiki", "Foo"));
        s1.set_wiki(Sitelink::new("frwiki", "Le Foo"));
        let mut s2 = Sitelinks::default();
        s2.set_wiki(Sitelink::new("dewiki", "Bar"));
        s2.set_wiki(Sitelink::new("enwiki", "Baz"));

        let patch = s2.patch(&s1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"patch":[
                {"op":"add","path":"/dewiki","value":{"badges":[],"title":"Bar"}},
                {"op":"replace","path":"/enwiki/title","value":"Baz"},
                {"op":"remove","path":"/frwiki"}
            ]})
        );
    }

    #[test]
    fn test_len() {
        let mut sitelinks = Sitelinks::default();
        assert_eq!(sitelinks.len(), 0);
        sitelinks.set_wiki(Sitelink::new("enwiki", "Foo"));
        assert_eq!(sitelinks.len(), 1);
    }

    #[test]
    fn test_is_empty() {
        let mut sitelinks = Sitelinks::default();
        assert!(sitelinks.is_empty());
        sitelinks.set_wiki(Sitelink::new("enwiki", "Foo"));
        assert!(!sitelinks.is_empty());
    }

    #[test]
    fn test_get_rest_api_path() {
        let sitelinks = Sitelinks::default();
        assert_eq!(
            sitelinks
                .get_my_rest_api_path(&EntityId::item("Q42"))
                .unwrap(),
            "/entities/items/Q42/sitelinks"
        );
    }

    #[test]
    fn test_header_info() {
        let sitelinks = Sitelinks::default();
        assert_eq!(sitelinks.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_serialize() {
        let sitelinks = Sitelinks {
            sitelinks: vec![
                Sitelink::new_complete(
                    "enwiki".to_string(),
                    "Douglas Adams".to_string(),
                    vec![],
                    Some("https://en.wikipedia.org/wiki/Douglas_Adams".to_string()),
                ),
                Sitelink::new_complete(
                    "dewiki".to_string(),
                    "Douglas Adams".to_string(),
                    vec![],
                    Some("https://de.wikipedia.org/wiki/Douglas_Adams".to_string()),
                ),
            ],
            header_info: HeaderInfo::default(),
        };
        let j = serde_json::to_value(&sitelinks).unwrap();
        assert_eq!(j["enwiki"]["title"].as_str().unwrap(), "Douglas Adams");
        assert_eq!(j["dewiki"]["title"].as_str().unwrap(), "Douglas Adams");
    }

    #[test]
    fn test_sitelinks() {
        let mut s = Sitelinks::default();
        s.set_wiki(Sitelink::new("enwiki", "foo"));
        s.set_wiki(Sitelink::new("dewiki", "bar"));
        let mut pages = s
            .sitelinks()
            .iter()
            .map(|s| s.title().to_string())
            .collect::<Vec<String>>();
        pages.sort();
        assert_eq!(pages, vec!["bar", "foo"]);
    }

    #[test]
    fn test_remove_wiki() {
        let mut s = Sitelinks::default();
        s.set_wiki(Sitelink::new("enwiki", "foo"));
        s.set_wiki(Sitelink::new("dewiki", "bar"));
        s.remove_wiki("enwiki");
        let pages = s
            .sitelinks()
            .iter()
            .map(|s| s.title().to_string())
            .collect::<Vec<String>>();
        assert_eq!(pages, vec!["bar"]);
    }
}
