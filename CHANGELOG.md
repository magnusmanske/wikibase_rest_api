# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-10-04

### Fixed
- After an automatic `OAuth2` token renewal, the request that triggered it is now sent with the *renewed* token (previously the stale one was sent and only later requests benefited)
- `HeaderInfo::revision_id` now parses weak ETags (`W/"123"`), which Wikidata sends; it was always `None` before
- `Label` / `Description::get_with_fallback` report the language the server actually used (taken from the redirect target, e.g. `mul`) instead of echoing the requested one
- `Aliases::get` / `AliasesInLanguage::get` only treat a 404 as "no aliases" when the server says the *aliases* are missing; a missing item is now an error instead of silently empty aliases
- `OAuth2` token-endpoint failures surface as `RestApiError::ApiError` with the server's OAuth error (e.g. `invalid_grant`), instead of a misleading `AccessTokenRequired`
- Entity-level patches (`Item::patch` / `Property::patch` + apply) now target the real `PATCH /entities/{group}/{id}` endpoint, with sub-patch paths correctly prefixed (`/labels/en`, `/sitelinks/enwiki/title`, …); previously both the URL and the paths were wrong
- Statements whose property has been deleted (`"data_type": null`) no longer fail to parse — and with them, the whole entity
- `Statement::same_qualifiers_as` is now a symmetric, order-independent comparison (it was a subset test)
- `Statement::new_item` tags the property as `wikibase-item` (was the non-existent data type `item`)
- `Statement::put` documentation: it *replaces* an existing statement; use `Statements::post` to add one
- The README's patch example compiles with just the prelude (`PatchApply`/`FromJson` are now re-exported there)

### Added
- Entity-scoped statement endpoints: `Statement::{get,put,delete}_for_entity` (+ `_match_` variants) and `StatementPatch::apply_for_entity` / `apply_match_for_entity`
- `ItemPatch` / `PropertyPatch` (aliases of the new generic `EntityPatch<E>`), `PatchEntry`, `StatementsPatch` are now public and re-exported
- `LabelsPatch` / `DescriptionsPatch` are complete: `Labels::patch` / `Descriptions::patch` return them, they implement `PatchApply`, and gain `add(language, value)`
- `DataType::Other(String)` keeps data types unknown to this crate, so they survive a round trip
- `EntityId::kind()`, `Aliases::in_language()`, `PropertyValue::from_json()`, `RestApiErrorPayload::resource_type()`, `RestApiError::is_missing_resource()`
- `Entity::post_meta` (create with edit metadata); created entities now carry the response's `HeaderInfo`

### Changed
- **Breaking:** `Entity::get` / `get_match` / `get_fields` / `get_match_fields` take `&EntityId` (like every other `get`); `get_match_fields` takes `fields` before `api`
- **Breaking:** `Entity` now requires `FromJson` (its own `from_json*` methods are gone; use `FromJson::from_json(&value)`), and gains `const ENTITY_TYPE`; `post_with_type*` are replaced by `post` / `post_meta`
- **Breaking:** `Item::patch` / `Property::patch` return `ItemPatch` / `PropertyPatch`; `EntityPatch::apply_item` / `apply_property` and friends are replaced by `PatchApply::apply`
- **Breaking:** `LanguageStringsPatch` is removed in favour of `LabelsPatch` / `DescriptionsPatch`
- **Breaking:** `DataType` is no longer `Copy`; `DataType::new` is infallible; the bogus `Item` / `Property` variants are removed; `Property::data_type()` returns `Option<&DataType>`
- **Breaking:** `TimePrecision`'s `TryFrom` impls return `RestApiError::InvalidPrecision`
- **Breaking:** `BearerToken::set_renewal_interval` takes a `Duration` (no more "0 means default"); `access_token_renewal_interval()` reports the effective interval
- **Breaking:** `HttpMisc::filter_response_error` is removed (use `parse_response`)
- **Breaking:** getters return borrowed, non-allocating types:
  - `LanguageString::language` / `value` (and so `Label`, `Description`) → `&str`
  - `Statement::id` → `Option<&str>`; `PropertyType::datatype` → `Option<&DataType>`
  - `BearerToken::get` / `client_id` / `client_secret` / `refresh_token` → `Option<&str>`
  - `EditMetadata::comment` → `Option<&str>` (no longer clones)
  - `Sitelink::badges`, `AliasesInLanguage::values` → `&[String]`; `Sitelinks::sitelinks` → `&[Sitelink]`
  - `Statements::property` / `property_mut` → `&[Statement]` / `&mut [Statement]`; `Aliases::get_lang` → `&[String]` (no longer allocate a `Vec`), and `get_lang` takes `AsRef<str>`
- **Breaking:** `EntityContainer::items()` / `properties()` are `async` and return a read guard over the map instead of handing out the internal `Arc<RwLock<…>>` (use `get_item` / `get_property` for single lookups)
- `RestApiBuilder::new` validates the URL properly (absolute `http(s)` URL with a `rest.php` path segment)
- `ApiError` messages show the server payload readably (`API error 404 Not Found: resource-not-found: …`), and JSON embedded in other error messages is truncated
- `Sitelink::delete` succeeds on any 2xx response instead of matching the response text
- Internal: shared JSON Patch helpers, derived `Serialize` for entities/statements, a single generic loader in `EntityContainer`, no duplicated `PatchApply` bodies

### Removed
- `EditMetadata::minor` / `set_minor` (never sent; the REST API has no minor flag)
- `BearerToken::check` (unused since renewal moved into `RestApi::execute`)
- `RestApiError::UnexpectedResponse` and `RestApiError::UnknownDataType` (no longer produced)

## [0.3.0] - 2026-07-23

### Added
- `SiteId` — a validated newtype for site (wiki) identifiers, exported from the crate root and prelude
- `RestApiError::InvalidLanguageCode` and `RestApiError::InvalidSiteId` variants

### Changed
- **Breaking:** language codes and site IDs are now validated before a request is issued. Malformed values — including ones that could inject extra URL path segments — fail fast with `RestApiError::InvalidLanguageCode` / `RestApiError::InvalidSiteId` instead of producing a garbled REST path. Values are trimmed and lower-cased, so previously a mixed-case code was sent verbatim; it is now normalised. Calls that used well-formed codes (`"en"`, `"enwiki"`, …) are unaffected. This applies to the `get`/`put`/`delete` operations on `Label`, `Description`, `AliasesInLanguage`, and `Sitelink`; data constructors (`new`, `from_json`) remain infallible and continue to accept whatever the server returns.

## [0.2.2] - 2026-07-23

### Security
- Raised minimum `tokio` (≥ 1.23.1) and `regex` (≥ 1.5.5) requirements to patched releases, clearing the RUSTSEC-2023-0001 and RUSTSEC-2022-0013 advisories (the loose `1` requirements permitted vulnerable versions even though the lockfile pinned patched ones)

### Changed
- Excluded test fixtures (`test_data/`) from the published package, shrinking it from ~3.1 MiB to ~1.4 MiB

## [0.2.1] - 2026-07-23

### Changed
- Upgraded `reqwest` to 0.13 — its default TLS backend is now **rustls** instead of native-tls; the `query` and `form` features are enabled explicitly (they became optional in 0.13)
- Upgraded `nutype` to 0.7 and `derive-where` to 1.6

### Added
- Substantially expanded the test suite (line coverage raised to ~95%)
- README badges for docs.rs, dependency status (deps.rs), MSRV, and `unsafe: forbidden`, plus generated AvgCCN and coverage badges with refresh scripts (`scripts/update-badges.sh`) and a version-bump git hook (`.githooks/pre-commit`)
- `.github/dependabot.yml` for automated dependency-update pull requests
- A Security section in the README

## [0.2.0] - 2026-07-23

### Added
- `EntityContainer::load_report()` and `LoadReport` — bulk loads report every ID as loaded, missing (404), or failed; nothing is dropped
- `EntityContainer` builder: `container_retries`, `container_backoff`; plus `get_item()` / `get_property()` accessors
- `RestApiBuilder::with_max_retry_after()` — caps how long a `Retry-After` can pause the client (default 60s)
- `RestApiError::is_rate_limited()` / `is_not_found()`
- Retry now parses HTTP-date `Retry-After` and adds ±25% backoff jitter

### Changed
- **Breaking:** `RestApiBuilder::build()` returns `Result` (no longer silently discards timeouts on client-build failure)
- **Breaking:** write operations (`put`/`delete`/`apply`/…) take `&RestApi` instead of `&mut RestApi`
- **Breaking:** `Entity::id()` returns `&EntityId` instead of a clone
- **Breaking:** `EntityId::new()` / `new_from_config()` validate the ID shape (type letter + digits); `item()` / `property()` remain unchecked
- **Breaking:** `RestApi.token` field is now `pub(crate)` — use `RestApi::token()`
- **Breaking:** glob re-exports replaced with explicit ones
- All non-success responses now surface as `RestApiError::ApiError` with the server payload (some paths previously returned a bare reqwest error)
- `HttpMisc::get_rest_api_path` default returns an error instead of panicking
- New error variants: `InvalidEntityId`, `PathNotImplemented`

### Fixed
- `EntityContainer::load()` no longer silently discards per-entity fetch failures
- Entity-level statement patches now use valid `/statements/{property}/{index}` paths with correct add/remove classification and index-safe ordering (previously invalid and inverted)
- `Search` no longer silently drops malformed results; parse errors are returned
- `Retry-After` can no longer block the client indefinitely; non-cloneable requests are sent once instead of failing
- `Statements::is_empty()` is now consistent with `len()`
- `Display for EntityId::None` renders empty instead of risking a panic via `format!`
- PATCH content-type corrected to `application/json-patch+json`

### Performance
- `tokio` reduced to the `sync` + `time` features (smaller downstream builds)
- Token freshness is checked under a read lock, so concurrent requests no longer serialize on it
- Fewer allocations in request building, `Statements::property`, and entity patch merging

### Removed
- Live-network tests replaced with mocked (wiremock) equivalents

## [0.1.16] - 2024-11-15

### Added
- `Property::data_type()` / `set_data_type()` — `data_type` is now stored, serialised, and deserialised on `Property` (required field for `POST /entities/properties`)
- `Property::patch()` — generates a JSON Patch to transform one property into another
- `Search::suggest_items()` / `suggest_properties()` — prefix-based autocomplete via `/suggest/{items,properties}`
- `RestApi::get_property_data_types()` — exposes the `/property-data-types` endpoint publicly
- `Entity::get_fields()` / `get_match_fields()` — pass `_fields=` to request a subset of entity fields
- `Statements::get_for_property()` / `get_for_property_match()` — server-side filter by property ID via `?property=`
- More types exported from `prelude`: `Aliases`, `Descriptions`, `Labels`, their patch types, `EntityType`, `HeaderInfo`, `RevisionMatch`, `SitelinksPatch`, `StatementPatch`

### Fixed
- `RevisionMatch::modify_headers()` now emits `If-Match` and `If-None-Match` headers from revision IDs and raw ETag strings (these were stored but never sent)
- `Statement::delete_match()` uses the HTTP status code to determine success instead of matching on the response body string
- `EditMetadata` no longer sends `"comment": ""` when no comment is set
- `Item` and `Property` deserialisation tolerates missing fields (needed for partial `_fields` responses)

## [0.1.13] - 2026-02-10

### Fixed
- Updated dependencies to fix a dependency vulnerability

### Changed
- Internal refactoring

## [0.1.12] - 2025-07-28

### Added
- `Eq`, `PartialOrd`, `Ord`, and `Hash` trait implementations for types
- `same_qualifiers_as` method for statement comparison

### Changed
- Miri testing improvements

## [0.1.11] - 2025-07-28

### Added
- `property_mut` accessor
- Documentation improvements

### Changed
- Replaced unmaintained `derivative` crate with `derive_where`
- Replaced unmaintained `derive` crate with `derive_more`
- Updated dependency versions
- Internal refactoring

## [0.1.10] - 2025-06-17

### Changed
- Minor improvements

## [0.1.9] - 2025-06-17

### Added
- Item search API support

### Changed
- Updated dependency versions

## [0.1.8] - 2025-04-11

### Added
- Exposed `Patch` trait in prelude

### Changed
- Internal cleanup

## [0.1.7] - 2025-04-11

### Added
- OpenSSF Scorecard supply-chain security workflow
- `TimePrecision` for internal time handling
- Const improvements

### Changed
- Internal refactoring

## [0.1.6] - 2025-03-31

### Added
- Sitelinks improvements
- Code analysis metrics and badges in README
- Documentation examples

### Changed
- Refactoring for better code quality
- Miri test fixes
- Switched to test.wikidata for testing

## [0.1.5] - 2025-03-17

### Changed
- Minor improvements

## [0.1.4] - 2025-03-17

### Fixed
- Statement parsing bugfix

## [0.1.3] - 2025-03-17

### Fixed
- Statement ID generator and PUT operations
- Descriptions path handling

### Changed
- Internal refactoring

## [0.1.2] - 2025-03-15

### Added
- Miri CI testing workflow
- Miri test compatibility
- Code analysis tooling

## [0.1.1] - 2025-03-12

### Changed
- Cleanup

## [0.1.0] - 2025-03-12

### Added
- Initial release
- REST API client for Wikibase instances
- Support for items, properties, labels, descriptions, aliases, statements, and sitelinks
- Async/await API design
- Concurrent entity loading via `EntityContainer`
- Dual MIT/Apache-2.0 license
