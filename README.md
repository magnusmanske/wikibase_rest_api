[![Crates.io](https://img.shields.io/crates/v/wikibase_rest_api?style=flat-square)](https://crates.io/crates/wikibase_rest_api)
[![Crates.io](https://img.shields.io/crates/d/wikibase_rest_api?style=flat-square)](https://crates.io/crates/wikibase_rest_api)
[![MSRV](https://img.shields.io/crates/msrv/wikibase_rest_api?style=flat-square)](Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE-MIT)
[![License](https://img.shields.io/badge/license-APACHE2-blue?style=flat-square)](LICENSE-APACHE2)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/acffb6bb26d8407b8e82704843a4aa7e)](https://app.codacy.com/gh/magnusmanske/wikibase_rest_api/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
[![CI](https://github.com/magnusmanske/wikibase_rest_api/actions/workflows/rust.yml/badge.svg)](https://github.com/magnusmanske/wikibase_rest_api/actions/workflows/rust.yml)
[![docs.rs](https://img.shields.io/docsrs/wikibase_rest_api?style=flat-square)](https://docs.rs/wikibase_rest_api)
[![Dependencies](https://deps.rs/repo/github/magnusmanske/wikibase_rest_api/status.svg)](https://deps.rs/repo/github/magnusmanske/wikibase_rest_api)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/10599/badge)](https://www.bestpractices.dev/projects/10599)
[![Unsafe: forbidden](https://img.shields.io/badge/unsafe-forbidden-success?style=flat-square)](src/lib.rs)
[![Avg. CCN](https://img.shields.io/badge/avg%20CCN-1.6-brightgreen?style=flat-square)](README.md)
[![Coverage](https://img.shields.io/badge/coverage-95.31%25-brightgreen?style=flat-square)](README.md)

# wikibase_rest_api

A Rust client library for the [Wikibase REST API](https://doc.wikimedia.org/Wikibase/master/js/rest-api/).
It provides async, type-safe access to Wikibase instances (including Wikidata) for reading and writing items, properties, labels, descriptions, aliases, statements, and sitelinks.
It works on any MediaWiki installation with the Wikibase extension and an enabled REST API.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
wikibase_rest_api = "0.2"
```

Or install via cargo:

```bash
cargo add wikibase_rest_api
```

### Building from source

```bash
git clone https://github.com/magnusmanske/wikibase_rest_api.git
cd wikibase_rest_api
cargo build
```

## Documentation

- **API Reference**: [docs.rs/wikibase_rest_api](https://docs.rs/wikibase_rest_api)
- **Wikibase REST API spec**: [doc.wikimedia.org](https://doc.wikimedia.org/Wikibase/master/js/rest-api/)
- **Examples**: see the [examples](examples) directory
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

## Usage
See also the [examples](examples).
```rust
// Create an API (use the Wikidata API shortcut)
let api = RestApi::wikidata()?;

// Use Q42 (Douglas Adams) as an example item
let id = EntityId::new("Q42")?;

// Get the label and sitelink of Q42
let q42_label_en = Label::get(&id, "en", &api).await?.value().to_owned();
let q42_sitelink = Sitelink::get(&id, "enwiki", &api).await?.title().to_owned();
println!("Q42 '{q42_label_en}' => [[enwiki:{q42_sitelink}]]");

// Create a new item
let mut item = Item::default();
item.labels_mut()
    .insert(LanguageString::new("en", "My label"));
item.statements_mut()
    .insert(Statement::new_string("P31", "Q42"));
let item: Item = item.post(&api).await?;
println!("Created new item {}", item.id());

// Load multiple entities concurrently
let entity_ids = [
    "Q42", "Q1", "Q2", "Q3", "Q4", "Q5", "Q6", "Q7", "Q8", "Q9", "P214",
]
.iter()
.map(|id| EntityId::new(*id))
.collect::<Result<Vec<_>, RestApiError>>()?;

// A container will manage the concurrent loading of entities.
let api = Arc::new(api);
let entity_container = EntityContainer::builder()
    .api(api)
    .max_concurrent(50)
    .build()?;
// Missing entities (e.g. Q6-Q9 here) are simply absent; use load_report() to inspect failures.
entity_container.load(&entity_ids).await?;
if let Some(q42) = entity_container.get_item("Q42").await {
    if let Some(label) = q42.labels().get_lang("en") {
        println!("Q42 label[en]: {label}");
    }
}

// Search for "Tim Berners-Lee" (in English) on Wikidata.
let query = "Tim Berners-Lee";
let language = Language::try_new("en")?;
let api = RestApi::builder("https://www.wikidata.org/w/rest.php")?
    .with_api_version(0) // Currently only works with v0 not v1
    .build()?;
let results = Search::items(query, language).get(&api).await?;
println!("{}", results[0].id());
```

# Implemented REST API actions
## items
- [x] `post`
- [x] `get`
- [ ] `patch`
## properties
- [x] `post`
- [x] `get`
- [ ] `patch`
## sitelinks
- [x] `get item_id`
- [x] `patch`
- [x] `get itemid/sitelink_id`
- [x] `put itemid/sitelink_id`
- [x] `delete itemid/sitelink_id`
## labels
- [x] `get item_id`
- [x] `patch item_id`
- [x] `get property_id`
- [x] `patch property_id`
- [x] `get item_id/language_code`
- [x] `put item_id/language_code`
- [x] `delete item_id/language_code`
- [x] `get item_id/language_code` with fallback language
- [x] `get property_id/language_code`
- [x] `put property_id/language_code`
- [x] `delete property_id/language_code`
- [x] `get property_id/language_code` with fallback language
## descriptions
- [x] `get item_id`
- [x] `patch item_id`
- [x] `get property_id`
- [x] `patch property_id`
- [x] `get item_id/language_code`
- [x] `put item_id/language_code`
- [x] `delete item_id/language_code`
- [x] `get item_id/language_code` with fallback language
- [x] `get property_id/language_code`
- [x] `put property_id/language_code`
- [x] `delete property_id/language_code`
- [x] `get property_id/language_code` with fallback language
## aliases
- [x] `get item_id`
- [x] `patch item_id`
- [x] `get property_id`
- [x] `patch property_id`
- [x] `get item_id/language_code`
- [x] `post item_id/language_code`
- [x] `get property_id/language_code`
- [x] `post property_id/language_code`
## statements
- [x] `get item_id`
- [x] `post item_id`
- [x] `get item_id/statement_id` as `get statement_id`
- [x] `put item_id/statement_id` as `put statement_id`
- [x] `patch item_id/statement_id` as `patch statement_id`
- [x] `delete item_id/statement_id` as `delete statement_id`
- [x] `get property_id`
- [x] `post property_id`
- [x] `get property_id/statement_id` as `get statement_id`
- [x] `put property_id/statement_id` as `put statement_id`
- [x] `patch property_id/statement_id` as `patch statement_id`
- [x] `delete property_id/statement_id` as `delete statement_id`
- [x] `get statement_id`
- [x] `put statement_id`
- [x] `patch statement_id`
- [x] `delete statement_id`
## misc
- [x] `/openapi.json`
- [x] `/property-data-types`
- [x] `seach items` (for Wikidata currently only in v0)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

- **Bug reports**: [GitHub Issues](https://github.com/magnusmanske/wikibase_rest_api/issues)
- **Feature requests**: [GitHub Issues](https://github.com/magnusmanske/wikibase_rest_api/issues)
- **Security vulnerabilities**: see [SECURITY.md](SECURITY.md)

## Security

- **Reporting:** please follow the process in [SECURITY.md](SECURITY.md) — use private [GitHub Security Advisories](https://github.com/magnusmanske/wikibase_rest_api/security/advisories/new); do **not** open a public issue for vulnerabilities.
- **No unsafe code:** the crate is `#![forbid(unsafe_code)]`, so the library contains no `unsafe` blocks.
- **Transport security:** all requests go over HTTPS via `reqwest` (rustls TLS backend).

## License

This project is dual-licensed under the [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE2) licenses. You may choose either license at your option.

# Developer notes
## TODO
- Maxlag/rate limits?

Code analysis is run via `analysis.sh`.

## Badges (AvgCCN + coverage)
`./scripts/update-badges.sh` refreshes both the AvgCCN and coverage badges at the
top of this README in one go (it calls `update-ccn.sh` and `update-coverage.sh`).

A git hook regenerates them automatically whenever a commit changes the crate
version in `Cargo.toml`. Enable it once per clone:
```bash
git config core.hooksPath .githooks
```

## Code coverage
```bash
cargo install cargo-tarpaulin # Once
cargo tarpaulin -o html       # Detailed HTML report
./scripts/update-coverage.sh  # Refresh only the coverage badge
```

## Lizard
Lizard is a simple code analyzer, giving cyclomatic complexity etc.
https://github.com/terryyin/lizard
```bash
lizard src -C 7 -V -L 40
./scripts/update-ccn.sh # Refresh the AvgCCN badge at the top of this README
```

## Analysis
Run `rust-code-analysis.py` (requires `rust-code-analysis-cli` to be installed) to generate `analysis.tab`.
This contains many metrics on code complexity and quality.
```bash
./rust-code-analysis.py
```

## Tarpaulin
```bash
cargo tarpaulin -o html
```

## grcov
[grcov](https://github.com/mozilla/grcov)

## Miri
Installation and usage: https://github.com/rust-lang/miri
```bash
cargo +nightly miri test
```
