use crate::{aliases_patch::AliasesPatch, FromJson, HeaderInfo, LanguageString, RestApiError};
use derivative::Derivative;
use serde::ser::{Serialize, SerializeMap};
use serde_json::{json, Value};
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

// ______________________________________________________________________________________________________________

#[derive(Derivative, Debug, Clone, Default)]
#[derivative(PartialEq)]
pub struct LanguageStringsMultiple {
    ls: HashMap<String, Vec<String>>,
    #[derivative(PartialEq = "ignore")]
    header_info: HeaderInfo,
}

impl LanguageStringsMultiple {
    /// Returns the list of values for a language
    pub fn get_lang<S: Into<String>>(&self, language: S) -> Vec<&str> {
        self.ls
            .get(&language.into())
            .map_or_else(Vec::new, |v| v.iter().map(|s| s.as_str()).collect())
    }

    /// Returns the list of values for a language, mutable
    pub fn get_lang_mut<S: Into<String>>(&mut self, language: S) -> &mut Vec<String> {
        self.ls.entry(language.into()).or_default()
    }

    /// Generates a patch to transform `other` into `self`
    pub fn patch(&self, other: &Self) -> Result<AliasesPatch, RestApiError> {
        let patch = json_patch::diff(&json!(&other), &json!(&self));
        let patch = AliasesPatch::from_json(&json!(patch))?;
        Ok(patch)
    }

    /// Returns the number of languages
    pub fn len(&self) -> usize {
        self.ls.len()
    }

    /// Returns true if there are no language strings
    pub fn is_empty(&self) -> bool {
        self.ls.is_empty()
    }

    fn from_json_header_info_part(
        language: &str,
        values: &[Value],
    ) -> Result<(String, Vec<String>), RestApiError> {
        let values = values
            .iter()
            .map(|v| {
                Ok(v.as_str()
                    .ok_or_else(|| RestApiError::MissingOrInvalidField {
                        field: "LanguageStringsMultiple".into(),
                        j: v.to_owned(),
                    })?
                    .to_string())
            })
            .collect::<Result<Vec<String>, RestApiError>>()?;
        Ok((language.to_owned(), values))
    }
}

impl FromJson for LanguageStringsMultiple {
    fn header_info(&self) -> &HeaderInfo {
        &self.header_info
    }

    fn from_json_header_info(j: &Value, header_info: HeaderInfo) -> Result<Self, RestApiError> {
        let ls = j
            .as_object()
            .ok_or_else(|| RestApiError::MissingOrInvalidField {
                field: "LanguageStringsMultiple".into(),
                j: j.to_owned(),
            })?
            .iter()
            .map(|(language, value)| {
                value.as_array().map_or_else(
                    || {
                        Err(RestApiError::MissingOrInvalidField {
                            field: "LanguageStringsMultiple".into(),
                            j: value.to_owned(),
                        })
                    },
                    |v| Self::from_json_header_info_part(language, v),
                )
            })
            .collect::<Result<HashMap<String, Vec<String>>, RestApiError>>()?;
        let ret = Self { ls, header_info };
        Ok(ret)
    }
}

impl LanguageStrings for LanguageStringsMultiple {
    fn has_language<S: Into<String>>(&self, language: S) -> bool {
        self.ls.contains_key(&language.into())
    }

    fn insert(&mut self, ls: LanguageString) {
        let entry = self.ls.entry(ls.language().to_string()).or_default();
        if !entry.contains(ls.value()) {
            entry.push(ls.value().to_owned());
        }
    }
}

impl Serialize for LanguageStringsMultiple {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.ls.len()))?;
        for (language, values) in &self.ls {
            s.serialize_entry(language, &values)?;
        }
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_language_strings_multiple() {
        let j = json!({
            "en": ["Hello", "Hi"],
            "de": ["Hallo", "Hi"],
        });
        let ls = LanguageStringsMultiple::from_json(&j).unwrap();
        assert_eq!(ls.get_lang("en"), vec!["Hello", "Hi"]);
        assert_eq!(ls.get_lang("de"), vec!["Hallo", "Hi"]);
        assert!(ls.get_lang("fr").is_empty());
    }

    #[test]
    fn test_language_strings_multiple_insert() {
        let mut ls = LanguageStringsMultiple::default();
        ls.insert(LanguageString::new("en", "Hello"));
        ls.insert(LanguageString::new("de", "Hallo"));
        ls.insert(LanguageString::new("en", "Hi"));
        assert_eq!(ls.get_lang("en"), vec!["Hello", "Hi"]);
        assert_eq!(ls.get_lang("de"), vec!["Hallo"]);
    }

    #[test]
    fn test_patch_aliases() {
        let mut l1 = LanguageStringsMultiple::default();
        l1.insert(LanguageString::new("en", "Foo"));
        l1.insert(LanguageString::new("en", "Bar"));
        l1.insert(LanguageString::new("en", "Baz"));
        l1.insert(LanguageString::new("de", "Foobar"));
        let mut l2 = l1.clone();
        l2.get_lang_mut("en")[2] = "Boo".to_string();
        l2.get_lang_mut("en").remove(1);
        l2.insert(LanguageString::new("de", "Foobaz"));

        let patch = l2.patch(&l1).unwrap();
        let patch_json = json!(patch);
        assert_eq!(
            patch_json,
            json!({"patch":[{"op":"add","path":"/de/1","value":"Foobaz"},{"op":"replace","path":"/en/1","value":"Boo"},{"op":"remove","path":"/en/2"}]})
        );
    }

    #[test]
    fn test_header_info_multiple() {
        let l = LanguageStringsMultiple::default();
        assert_eq!(l.header_info(), &HeaderInfo::default());
    }

    #[test]
    fn test_serialize2() {
        let mut l = LanguageStringsMultiple::default();
        l.insert(LanguageString::new("en", "Foo"));
        l.insert(LanguageString::new("en", "Bar"));
        l.insert(LanguageString::new("de", "Baz"));
        let s = serde_json::to_string(&l).unwrap();
        assert!(s.contains(r#""en":["Foo","Bar"]"#));
        assert!(s.contains(r#""de":["Baz"]"#));
    }
}
