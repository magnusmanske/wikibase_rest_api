#![forbid(unsafe_code)]
#![warn(
    clippy::cognitive_complexity,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_link_with_quotes,
    clippy::doc_markdown,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::float_cmp,
    clippy::float_cmp_const,
    clippy::float_equality_without_abs,
    keyword_idents,
    clippy::missing_const_for_fn,
    missing_copy_implementations,
    missing_debug_implementations,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::mod_module_files,
    non_ascii_idents,
    noop_method_call,
    clippy::option_if_let_else,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::semicolon_if_nothing_returned,
    clippy::unseparated_literal_suffix,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::suspicious_operation_groupings,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    clippy::unused_self,
    clippy::use_debug,
    clippy::used_underscore_binding,
    clippy::useless_let_if_seq,
    clippy::wildcard_dependencies,
    clippy::wildcard_imports
)]

//! **Wikibase REST API** is a Rust library for interacting with the
//! [Wikibase REST API](https://www.wikidata.org/wiki/Wikidata:REST_API)
//! for [Wikibase](https://www.mediawiki.org/wiki/Wikibase) instances.
//! It provides a set of types and methods for interacting with the API,
//! and implements all the [API endpoints](https://doc.wikimedia.org/Wikibase/master/js/rest-api/).

pub mod rest_api;
pub mod revision_match;
pub mod entity_id;
pub mod entity;
pub mod property;
pub mod item;
pub mod patch_entry;
pub mod sitelink;
pub mod sitelinks;
pub mod sitelinks_patch;
pub mod language_string;
pub mod language_strings;
pub mod language_strings_patch;
pub mod data_type;
pub mod statement;
pub mod reference;
pub mod statement_rank;
pub mod statement_value;
pub mod statement_patch;
pub mod property_value;
pub mod statements;
pub mod edit_metadata;
pub mod get_put_delete;
pub mod config;
pub mod label;
pub mod aliases;
pub mod aliases_patch;
pub mod description;
pub mod entity_container;
pub mod patch;
pub mod header_info;
pub mod bearer_token;
pub mod error;
pub mod prelude;

pub use sitelink::Sitelink;
pub use sitelinks::Sitelinks;
pub use rest_api::{RestApi, RestApiBuilder};
pub use revision_match::RevisionMatch;
pub use item::Item;
pub use property::Property;
pub use language_string::LanguageString;
pub use entity_id::EntityId;
pub use data_type::DataType;
pub use reference::Reference;
pub use statement_rank::StatementRank;
pub use statement::Statement;
pub use edit_metadata::EditMetadata;
pub use config::Config;
pub use get_put_delete::*;
pub use patch::*;
pub use header_info::HeaderInfo;
pub use error::RestApiError;
