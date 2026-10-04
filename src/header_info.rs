use std::time::SystemTime;

#[derive(Debug, Clone, Default, PartialEq, Copy)]
pub struct HeaderInfo {
    revision_id: Option<u64>,
    last_modified: Option<SystemTime>,
}

impl HeaderInfo {
    /// Constructs a new `HeaderInfo` object from a `HeaderMap` (from a `reqwest::Response`).
    /// The revision ID is taken from the `ETag`, which may be strong (`"123"`) or weak (`W/"123"`).
    pub fn from_header(header: &reqwest::header::HeaderMap) -> Self {
        let revision_id = header
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .and_then(Self::revision_from_etag);
        let last_modified = header
            .get("Last-Modified")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| httpdate::parse_http_date(s).ok());
        Self {
            revision_id,
            last_modified,
        }
    }

    fn revision_from_etag(etag: &str) -> Option<u64> {
        let etag = etag.trim();
        let etag = etag.strip_prefix("W/").unwrap_or(etag);
        etag.trim_matches('"').parse().ok()
    }

    /// Returns the revision ID.
    pub const fn revision_id(&self) -> Option<u64> {
        self.revision_id
    }

    /// Returns the last modified date.
    pub const fn last_modified(&self) -> Option<SystemTime> {
        self.last_modified
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use reqwest::header::HeaderMap;

    #[test]
    fn test_header_info() {
        let mut headers = HeaderMap::new();
        headers.insert("ETag", HeaderValue::from_str("1234567890").unwrap());
        headers.insert(
            "Last-Modified",
            HeaderValue::from_str("Wed, 21 Oct 2015 07:28:00 GMT").unwrap(),
        );
        let hi = HeaderInfo::from_header(&headers);
        assert_eq!(hi.revision_id(), Some(1234567890));
        assert!(hi.last_modified().is_some());
        let formatted = httpdate::fmt_http_date(hi.last_modified().unwrap());
        assert_eq!(formatted, "Wed, 21 Oct 2015 07:28:00 GMT");
    }

    #[test]
    fn test_revision_from_etag_forms() {
        // Wikidata sends weak ETags, e.g. `W/"2550809167"`.
        assert_eq!(
            HeaderInfo::revision_from_etag(r#"W/"2550809167""#),
            Some(2550809167)
        );
        assert_eq!(HeaderInfo::revision_from_etag(r#""42""#), Some(42));
        assert_eq!(HeaderInfo::revision_from_etag("42"), Some(42));
        assert_eq!(HeaderInfo::revision_from_etag("W/\"abc\""), None);
    }

    #[test]
    fn test_header_info_weak_etag() {
        let mut headers = HeaderMap::new();
        headers.insert("ETag", HeaderValue::from_static(r#"W/"2550809167""#));
        assert_eq!(
            HeaderInfo::from_header(&headers).revision_id(),
            Some(2550809167)
        );
    }
}
