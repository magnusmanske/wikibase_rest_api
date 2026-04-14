# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
