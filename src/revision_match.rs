use crate::RestApiError;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::SystemTime;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RevisionMatch {
    modified_since_revisions: Vec<u64>,
    modified_since_date: Option<SystemTime>,
    unmodified_since_revisions: Vec<u64>,
    unmodified_since_date: Option<SystemTime>,
    if_match: Vec<String>,
    if_none_match: Vec<String>,
}

impl RevisionMatch {
    pub fn modify_headers(&self, headers: &mut HeaderMap) -> Result<(), RestApiError> {
        if let Some(date) = self.modified_since_date {
            let hv = HeaderValue::from_str(&httpdate::fmt_http_date(date))?;
            headers.insert("If-Modified-Since", hv);
        }
        if let Some(date) = self.unmodified_since_date {
            let hv = HeaderValue::from_str(&httpdate::fmt_http_date(date))?;
            headers.insert("If-Unmodified-Since", hv);
        }
        // TODO FIXME complete
        Ok(())
    }

    pub fn modified_since_revisions(&self) -> &[u64] {
        &self.modified_since_revisions
    }

    pub fn set_modified_since_revisions(&mut self, modified_since_revisions: Vec<u64>) {
        self.modified_since_revisions = modified_since_revisions;
    }

    pub const fn modified_since_date(&self) -> Option<SystemTime> {
        self.modified_since_date
    }

    pub fn set_modified_since_date(&mut self, modified_since_date: Option<SystemTime>) {
        self.modified_since_date = modified_since_date;
    }

    pub fn unmodified_since_revisions(&self) -> &[u64] {
        &self.unmodified_since_revisions
    }

    pub fn set_unmodified_since_revisions(&mut self, unmodified_since_revisions: Vec<u64>) {
        self.unmodified_since_revisions = unmodified_since_revisions;
    }

    pub const fn unmodified_since_date(&self) -> Option<SystemTime> {
        self.unmodified_since_date
    }

    pub fn set_unmodified_since_date(&mut self, unmodified_since_date: Option<SystemTime>) {
        self.unmodified_since_date = unmodified_since_date;
    }

    pub fn if_match(&self) -> &[String] {
        &self.if_match
    }

    pub fn set_if_match(&mut self, if_match: Vec<String>) {
        self.if_match = if_match;
    }

    pub fn if_none_match(&self) -> &[String] {
        &self.if_none_match
    }

    pub fn set_if_none_match(&mut self, if_none_match: Vec<String>) {
        self.if_none_match = if_none_match;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_revision_match() {
        // #lizard forgives the complexity
        let mut revision_match = RevisionMatch::default();
        assert!(revision_match.modified_since_revisions().is_empty());
        assert_eq!(revision_match.modified_since_date(), None);
        assert!(revision_match.unmodified_since_revisions().is_empty());
        assert_eq!(revision_match.unmodified_since_date(), None);
        assert!(revision_match.if_match().is_empty());
        assert!(revision_match.if_none_match().is_empty());

        revision_match.set_modified_since_revisions(vec![1, 2, 3]);
        assert_eq!(revision_match.modified_since_revisions(), &[1, 2, 3]);

        let date = SystemTime::UNIX_EPOCH + Duration::from_secs(1609459200); // 2021-01-01 00:00:00 UTC
        revision_match.set_modified_since_date(Some(date));
        assert_eq!(revision_match.modified_since_date(), Some(date));

        revision_match.set_unmodified_since_revisions(vec![4, 5, 6]);
        assert_eq!(revision_match.unmodified_since_revisions(), &[4, 5, 6]);

        let date2 = SystemTime::UNIX_EPOCH + Duration::from_secs(1609545600); // 2021-01-02 00:00:00 UTC
        revision_match.set_unmodified_since_date(Some(date2));
        assert_eq!(revision_match.unmodified_since_date(), Some(date2));

        revision_match.set_if_match(vec!["1".to_string(), "2".to_string()]);
        assert_eq!(
            revision_match.if_match(),
            &["1".to_string(), "2".to_string()]
        );

        revision_match.set_if_none_match(vec!["3".to_string(), "4".to_string()]);
        assert_eq!(
            revision_match.if_none_match(),
            &["3".to_string(), "4".to_string()]
        );
    }
}
