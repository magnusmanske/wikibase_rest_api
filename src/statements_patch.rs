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
