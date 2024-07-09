#[derive(Debug, Clone, Default, PartialEq, Copy)]
pub struct HeaderInfo {
    revision_id: Option<u64>,
    last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

impl HeaderInfo {
    /// Constructs a new `HeaderInfo` object from a `HeaderMap`` (from a `reqwest::Response``).
    pub fn from_header(header: &reqwest::header::HeaderMap) -> Self {
        let revision_id = header
            .get("ETag")
            .map(|v| v.to_str().ok())
            .flatten()
            .map(|s|s.replace('"',"").parse::<u64>().ok())
            .flatten();
        let last_modified = header
            .get("Last-Modified")
            .map(|v| v.to_str().ok())
            .flatten()
            .map(|s| chrono::DateTime::parse_from_rfc2822(s).ok())
            .flatten()
            .map(|dt| dt.to_utc());
        Self {
            revision_id,
            last_modified,
        }
    }

    /// Returns the revision ID.
    pub fn revision_id(&self) -> Option<u64> {
        self.revision_id
    }

    /// Returns the last modified date.
    pub fn last_modified(&self) -> Option<&chrono::DateTime<chrono::Utc>> {
        self.last_modified.as_ref()
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
        headers.insert("Last-Modified", HeaderValue::from_str("Wed, 21 Oct 2015 07:28:00 GMT").unwrap());
        let hi = HeaderInfo::from_header(&headers);
        assert_eq!(hi.revision_id(), Some(1234567890));
        assert_eq!(hi.last_modified().unwrap().to_rfc2822(), "Wed, 21 Oct 2015 07:28:00 +0000");
    }
}