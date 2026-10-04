/// Generates a single-value language string type (Label or Description) with all trait implementations.
///
/// Parameters:
/// - `$type_name`: The struct name (e.g., `Label`, `Description`)
/// - `$path_segment`: REST API path segment (e.g., `"labels"`, `"descriptions"`)
/// - `$fallback_segment`: Fallback path segment (e.g., `"labels_with_language_fallback"`)
/// - `$json_field`: JSON field name for PUT requests (e.g., `"label"`, `"description"`)
/// - `$error_field`: Field name for error messages (e.g., `"Label"`, `"Description"`)
macro_rules! impl_language_string_value {
    (
        $type_name:ident,
        $path_segment:literal,
        $fallback_segment:literal,
        $json_field:literal,
        $error_field:literal
    ) => {
        use crate::{
            EditMetadata, EntityId, HeaderInfo, HttpDelete, HttpGet, HttpGetEntityWithFallback,
            HttpMisc, HttpPut, Language, LanguageString, RestApi, RestApiError, RevisionMatch,
        };
        use derive_where::DeriveWhere;
        use serde_json::{json, Value};
        use std::collections::HashMap;
        use std::ops::Deref;

        #[derive(DeriveWhere, Debug, Clone)]
        #[derive_where(PartialEq)]
        pub struct $type_name {
            ls: LanguageString,
            #[derive_where(skip)]
            header_info: HeaderInfo,
        }

        impl $type_name {
            pub fn new<S1: Into<String>, S2: Into<String>>(language: S1, value: S2) -> Self {
                Self {
                    ls: LanguageString::new(language, value),
                    header_info: HeaderInfo::default(),
                }
            }

            async fn get_match_response(
                id: &EntityId,
                language: &str,
                api: &RestApi,
                rm: RevisionMatch,
                mode: &str,
            ) -> Result<reqwest::Response, RestApiError> {
                let path = format!(
                    "/entities/{group}/{id}/{mode}/{language}",
                    group = id.group()?
                );
                let mut request = api
                    .wikibase_request_builder(&path, HashMap::new(), reqwest::Method::GET)
                    .await?
                    .build()?;
                rm.modify_headers(request.headers_mut())?;
                api.execute(request).await
            }

            /// Builds the value from a response body, which must be a JSON string.
            fn from_response_json(
                language: &str,
                j: &Value,
                header_info: HeaderInfo,
            ) -> Result<Self, RestApiError> {
                let s = j.as_str().ok_or_else(|| RestApiError::MissingOrInvalidField {
                    field: $error_field.into(),
                    j: j.to_owned(),
                })?;
                Ok(Self {
                    ls: LanguageString::new(language, s),
                    header_info,
                })
            }
        }

        impl Deref for $type_name {
            type Target = LanguageString;
            fn deref(&self) -> &Self::Target {
                &self.ls
            }
        }

        impl From<LanguageString> for $type_name {
            fn from(ls: LanguageString) -> Self {
                Self {
                    ls,
                    header_info: HeaderInfo::default(),
                }
            }
        }

        impl From<$type_name> for LanguageString {
            fn from(val: $type_name) -> Self {
                val.ls
            }
        }

        impl HttpMisc for $type_name {
            fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
                let group = id.group()?;
                let language = Language::validated(self.ls.language())?;
                Ok(format!("/entities/{group}/{id}/{}/{language}", $path_segment))
            }
        }

        impl HttpGetEntityWithFallback for $type_name {
            /// The server answers a fallback request with a redirect to the language it
            /// actually used (e.g. `/labels/mul`), so the returned value carries *that*
            /// language rather than the requested one.
            async fn get_match_with_fallback(
                id: &EntityId,
                language: &str,
                api: &RestApi,
                rm: RevisionMatch,
            ) -> Result<Self, RestApiError> {
                let language = Language::validated(language)?;
                let response =
                    Self::get_match_response(id, &language, api, rm, $fallback_segment).await?;
                let resolved_language = response
                    .url()
                    .path_segments()
                    .and_then(|mut segments| segments.next_back())
                    .filter(|segment| !segment.is_empty())
                    .map_or(language, str::to_string);
                let (j, header_info) = Self::parse_response(response).await?;
                Self::from_response_json(&resolved_language, &j, header_info)
            }
        }

        impl HttpGet for $type_name {
            async fn get_match(
                id: &EntityId,
                language: &str,
                api: &RestApi,
                rm: RevisionMatch,
            ) -> Result<Self, RestApiError> {
                let language = Language::validated(language)?;
                let response =
                    Self::get_match_response(id, &language, api, rm, $path_segment).await?;
                let (j, header_info) = Self::parse_response(response).await?;
                Self::from_response_json(&language, &j, header_info)
            }
        }

        impl HttpDelete for $type_name {
            async fn delete_meta(
                &self,
                id: &EntityId,
                api: &RestApi,
                em: EditMetadata,
            ) -> Result<(), RestApiError> {
                let j = json!({});
                self.run_json_query(id, reqwest::Method::DELETE, j, api, &em)
                    .await?;
                Ok(())
            }
        }

        impl HttpPut for $type_name {
            async fn put_meta(
                &self,
                id: &EntityId,
                api: &RestApi,
                em: EditMetadata,
            ) -> Result<Self, RestApiError> {
                let j = json!({$json_field: self.ls.value()});
                let (j, header_info) = self
                    .run_json_query(id, reqwest::Method::PUT, j, api, &em)
                    .await?;
                Self::from_response_json(self.language(), &j, header_info)
            }
        }
    };
}

/// Generates a language string collection type (Labels or Descriptions) with all trait implementations.
///
/// Parameters:
/// - `$type_name`: The struct name (e.g., `Labels`)
/// - `$path_segment`: REST API path segment (e.g., `"labels"`)
/// - `$error_field`: Field name for error messages (e.g., `"Labels"`)
/// - `$patch_type`: The matching patch type (e.g., `crate::labels_patch::LabelsPatch`)
macro_rules! impl_language_string_collection {
    (
        $type_name:ident,
        $path_segment:literal,
        $error_field:literal,
        $patch_type:path
    ) => {
        use crate::{
            language_strings::LanguageStrings, patch_entry::PatchEntry, EntityId, FromJson,
            HeaderInfo, HttpGetEntity, HttpMisc, LanguageString, Patch, RestApi, RestApiError,
            RevisionMatch,
        };
        use derive_where::DeriveWhere;
        use serde::ser::{Serialize, SerializeMap};
        use serde_json::Value;
        use std::collections::HashMap;

        #[derive(DeriveWhere, Debug, Clone, Default)]
        #[derive_where(PartialEq)]
        pub struct $type_name {
            ls: HashMap<String, String>,
            #[derive_where(skip)]
            header_info: HeaderInfo,
        }

        impl $type_name {
            pub fn get_lang<S: Into<String>>(&self, language: S) -> Option<&str> {
                self.ls.get(&language.into()).map(|s| s.as_str())
            }

            pub fn len(&self) -> usize {
                self.ls.len()
            }

            pub fn is_empty(&self) -> bool {
                self.ls.is_empty()
            }

            pub const fn list(&self) -> &HashMap<String, String> {
                &self.ls
            }

            pub const fn list_mut(&mut self) -> &mut HashMap<String, String> {
                &mut self.ls
            }

            /// Generates a patch to transform `other` into `self`
            pub fn patch(&self, other: &Self) -> Result<$patch_type, RestApiError> {
                let mut patch = <$patch_type>::default();
                *patch.patch_mut() = PatchEntry::diff(other, self, $error_field)?;
                Ok(patch)
            }
        }

        impl HttpMisc for $type_name {
            fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
                let group = id.group()?;
                Ok(format!("/entities/{group}/{id}/{}", $path_segment))
            }
        }

        impl HttpGetEntity for $type_name {
            async fn get_match(
                id: &EntityId,
                api: &RestApi,
                rm: RevisionMatch,
            ) -> Result<Self, RestApiError> {
                let group = id.group()?;
                let path = format!("/entities/{group}/{id}/{}", $path_segment);
                let (j, header_info) = Self::get_match_internal(api, &path, rm).await?;
                Self::from_json_header_info(&j, header_info)
            }
        }

        impl FromJson for $type_name {
            fn header_info(&self) -> &HeaderInfo {
                &self.header_info
            }

            fn from_json_header_info(
                j: &Value,
                header_info: HeaderInfo,
            ) -> Result<Self, RestApiError> {
                let ls =
                    j.as_object()
                        .ok_or_else(|| RestApiError::WrongType {
                            field: $error_field.to_string(),
                            j: j.to_owned(),
                        })?
                        .iter()
                        .map(|(language, value)| {
                            let value = value.as_str().ok_or_else(|| {
                                RestApiError::MissingOrInvalidField {
                                    field: $error_field.into(),
                                    j: value.to_owned(),
                                }
                            })?;
                            Ok((language.to_owned(), value.to_owned()))
                        })
                        .collect::<Result<HashMap<String, String>, RestApiError>>()?;
                Ok(Self { ls, header_info })
            }
        }

        impl LanguageStrings for $type_name {
            fn has_language<S: Into<String>>(&self, language: S) -> bool {
                self.ls.contains_key(&language.into())
            }

            fn insert(&mut self, ls: LanguageString) {
                self.ls
                    .insert(ls.language().to_owned(), ls.value().to_owned());
            }
        }

        impl Serialize for $type_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                let mut s = serializer.serialize_map(Some(self.ls.len()))?;
                for (language, ls) in &self.ls {
                    s.serialize_entry(language, ls)?;
                }
                s.end()
            }
        }
    };
}

/// Generates a language string patch type (`LabelsPatch` or `DescriptionsPatch`) with all trait implementations.
///
/// Parameters:
/// - `$type_name`: The struct name (e.g., `LabelsPatch`)
/// - `$collection`: The collection the patch applies to (e.g., `crate::labels::Labels`)
/// - `$path_segment`: REST API path segment (e.g., `"labels"`)
/// - `$error_field`: Field name for error messages (e.g., `"LabelsPatch"`)
macro_rules! impl_language_string_patch {
    (
        $type_name:ident,
        $collection:path,
        $path_segment:literal,
        $error_field:literal
    ) => {
        use crate::{patch_entry::PatchEntry, EntityId, HttpMisc, Patch, PatchApply, RestApiError};
        use serde::Serialize;
        use serde_json::Value;

        #[derive(Debug, Clone, PartialEq, Serialize, Default)]
        pub struct $type_name {
            patch: Vec<PatchEntry>,
        }

        impl $type_name {
            /// Generates a patch from JSON, presumably from `json_patch`
            pub fn from_json(j: &Value) -> Result<Self, RestApiError> {
                Ok(Self {
                    patch: PatchEntry::list_from_json(j, $error_field)?,
                })
            }

            /// Adds a command to add the value for a language that doesn't have one yet.
            pub fn add<S1: Into<String>, S2: Into<String>>(&mut self, language: S1, value: S2) {
                <Self as Patch>::add(self, format!("/{}", language.into()), value.into().into());
            }

            /// Adds a command to replace the existing value for a language.
            pub fn replace<S1: Into<String>, S2: Into<String>>(&mut self, language: S1, value: S2) {
                <Self as Patch>::replace(
                    self,
                    format!("/{}", language.into()),
                    value.into().into(),
                );
            }

            /// Adds a command to remove the value for a language.
            pub fn remove<S: Into<String>>(&mut self, language: S) {
                <Self as Patch>::remove(self, format!("/{}", language.into()));
            }
        }

        impl Patch for $type_name {
            fn patch(&self) -> &Vec<PatchEntry> {
                &self.patch
            }

            fn patch_mut(&mut self) -> &mut Vec<PatchEntry> {
                &mut self.patch
            }
        }

        impl PatchApply<$collection> for $type_name {}

        impl HttpMisc for $type_name {
            fn get_my_rest_api_path(&self, id: &EntityId) -> Result<String, RestApiError> {
                let group = id.group()?;
                Ok(format!("/entities/{group}/{id}/{}", $path_segment))
            }
        }
    };
}
