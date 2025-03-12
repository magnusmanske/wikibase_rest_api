use crate::LanguageString;

pub trait LanguageStrings {
    fn insert(&mut self, ls: LanguageString);
    fn has_language<S: Into<String>>(&self, language: S) -> bool;
}
