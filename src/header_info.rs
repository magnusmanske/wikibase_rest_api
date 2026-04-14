use std::time::SystemTime;

#[derive(Debug, Clone, Default, PartialEq, Copy)]
pub struct HeaderInfo {
    revision_id: Option<u64>,
    last_modified: Option<SystemTime>,
}

impl HeaderInfo {
    /// Constructs a new `HeaderInfo` object from a `HeaderMap` (from a `reqwest::Response`).
    pub fn from_header(header: &reqwest::header::HeaderMap) -> Self {
        let revision_id = header
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.replace('"', "").parse::<u64>().ok());
        let last_modified = header
            .get("Last-Modified")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| httpdate::parse_http_date(s).ok());
        Self {
            revision_id,
            last_modified,
        }
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
}
