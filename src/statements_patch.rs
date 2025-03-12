use crate::{patch_entry::PatchEntry, Patch};
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct StatementsPatch {
    patch: Vec<PatchEntry>,
}

impl Patch for StatementsPatch {
    fn patch(&self) -> &Vec<PatchEntry> {
        &self.patch
    }

    fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
        &mut self.patch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_statements_patch() {
        let mut patch = StatementsPatch::default();
        patch.add("/foo/bar", json!("baz"));
        assert_eq!(patch.patch().len(), 1);
        let expected = PatchEntry::new("add", "/foo/bar", json!("baz"));
        assert_eq!(patch.patch()[0], expected);

        patch.patch_mut().remove(0);
        assert_eq!(patch.patch().len(), 0);
    }
}
