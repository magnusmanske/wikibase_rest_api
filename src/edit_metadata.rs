use crate::RevisionMatch;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditMetadata {
    comment: Option<String>,
    bot: bool,
    tags: Vec<String>,
    revision_match: RevisionMatch,
}

impl EditMetadata {
    pub fn comment(&self) -> Option<&str> {
        self.comment.as_deref()
    }

    pub const fn bot(&self) -> bool {
        self.bot
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub const fn revision_match(&self) -> &RevisionMatch {
        &self.revision_match
    }

    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
    }

    pub const fn set_bot(&mut self, bot: bool) {
        self.bot = bot;
    }

    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
    }

    pub fn set_revision_match(&mut self, revision_match: RevisionMatch) {
        self.revision_match = revision_match;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_metadata() {
        let mut edit_metadata = EditMetadata::default();
        assert_eq!(edit_metadata.comment(), None);
        assert!(!edit_metadata.bot());
        assert!(edit_metadata.tags().is_empty());

        edit_metadata.set_comment(Some("Test".to_string()));
        assert_eq!(edit_metadata.comment(), Some("Test"));

        edit_metadata.set_bot(true);
        assert!(edit_metadata.bot());

        edit_metadata.set_tags(vec!["Test".to_string()]);
        assert_eq!(edit_metadata.tags(), &["Test".to_string()]);
    }

    #[test]
    fn test_set_revision_match() {
        let mut edit_metadata = EditMetadata::default();
        let mut revision_match = RevisionMatch::default();
        revision_match.set_modified_since_revisions(vec![1]);
        edit_metadata.set_revision_match(revision_match.clone());
        assert_eq!(edit_metadata.revision_match(), &revision_match);
    }
}
