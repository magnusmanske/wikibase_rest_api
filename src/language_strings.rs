use crate::LanguageString;
use std::collections::HashMap;

pub trait LanguageStrings {
    fn insert(&mut self, ls: LanguageString);
    fn has_language<S: Into<String>>(&self, language: S) -> bool;
}

pub trait LanguageStringsSingle {
    fn ls(&self) -> &HashMap<String, String>;

    /// Returns the value for a language
    fn get_lang<S: Into<String>>(&self, language: S) -> Option<&str> {
        self.ls().get(&language.into()).map(|s| s.as_str())
    }

    /// Returns the number of labels/languages
    fn len(&self) -> usize {
        self.ls().len()
    }

    /// Returns true if there are no labels/languages
    fn is_empty(&self) -> bool {
        self.ls().is_empty()
    }
}
