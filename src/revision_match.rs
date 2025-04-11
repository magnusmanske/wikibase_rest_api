use chrono::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue};

use crate::RestApiError;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RevisionMatch {
    modified_since_revisions: Vec<u64>,
    modified_since_date: Option<NaiveDateTime>,
    unmodified_since_revisions: Vec<u64>,
    unmodified_since_date: Option<NaiveDateTime>,
    if_match: Vec<String>,
    if_none_match: Vec<String>,
}

impl RevisionMatch {
    pub fn modify_headers(&self, headers: &mut HeaderMap) -> Result<(), RestApiError> {
        if let Some(date) = self.modified_since_date {
            let hvs = format!("{}", date.format("%c"));
            let hv = HeaderValue::from_str(&hvs)?;
            headers.insert("If-Modified-Since", hv);
        }
        if let Some(date) = self.unmodified_since_date {
            let hvs = format!("{}", date.format("%c"));
            let hv = HeaderValue::from_str(&hvs)?;
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

    pub const fn modified_since_date(&self) -> Option<NaiveDateTime> {
        self.modified_since_date
    }

    pub const fn set_modified_since_date(&mut self, modified_since_date: Option<NaiveDateTime>) {
        self.modified_since_date = modified_since_date;
    }

    pub fn unmodified_since_revisions(&self) -> &[u64] {
        &self.unmodified_since_revisions
    }

    pub fn set_unmodified_since_revisions(&mut self, unmodified_since_revisions: Vec<u64>) {
        self.unmodified_since_revisions = unmodified_since_revisions;
    }

    pub const fn unmodified_since_date(&self) -> Option<NaiveDateTime> {
        self.unmodified_since_date
    }

    pub const fn set_unmodified_since_date(
        &mut self,
        unmodified_since_date: Option<NaiveDateTime>,
    ) {
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

        revision_match.set_modified_since_date(Some(
            NaiveDate::from_ymd_opt(2021, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        ));
        assert_eq!(
            revision_match.modified_since_date(),
            Some(
                NaiveDate::from_ymd_opt(2021, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            )
        );

        revision_match.set_unmodified_since_revisions(vec![4, 5, 6]);
        assert_eq!(revision_match.unmodified_since_revisions(), &[4, 5, 6]);

        revision_match.set_unmodified_since_date(Some(
            NaiveDate::from_ymd_opt(2021, 1, 2)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        ));
        assert_eq!(
            revision_match.unmodified_since_date(),
            Some(
                NaiveDate::from_ymd_opt(2021, 1, 2)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
            )
        );

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
