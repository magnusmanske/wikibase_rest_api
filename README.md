[![Crates.io](https://img.shields.io/crates/v/wikibase_rest_api?style=flat-square)](https://crates.io/crates/wikibase_rest_api)
[![Crates.io](https://img.shields.io/crates/d/wikibase_rest_api?style=flat-square)](https://crates.io/crates/wikibase_rest_api)
[![MSRV](https://img.shields.io/crates/msrv/wikibase_rest_api?style=flat-square)](Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE-MIT)
[![License](https://img.shields.io/badge/license-APACHE2-blue?style=flat-square)](LICENSE-APACHE2)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/acffb6bb26d8407b8e82704843a4aa7e)](https://app.codacy.com/gh/magnusmanske/wikibase_rest_api/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
[![CI](https://github.com/magnusmanske/wikibase_rest_api/actions/workflows/rust.yml/badge.svg)](https://github.com/magnusmanske/wikibase_rest_api/actions/workflows/rust.yml)
[![docs.rs](https://img.shields.io/docsrs/wikibase_rest_api?style=flat-square)](https://docs.rs/wikibase_rest_api)
[![Dependencies](https://deps.rs/crate/wikibase_rest_api/latest/status.svg)](https://deps.rs/crate/wikibase_rest_api)
[![OpenSSF Best Practices](https://www.bestpractices.dev/projects/10599/badge)](https://www.bestpractices.dev/projects/10599)
[![Unsafe: forbidden](https://img.shields.io/badge/unsafe-forbidden-success?style=flat-square)](src/lib.rs)
[![Avg. CCN](https://img.shields.io/badge/avg%20CCN-1.6-brightgreen?style=flat-square)](README.md)
[![Coverage](https://img.shields.io/badge/coverage-95.09%25-brightgreen?style=flat-square)](README.md)

# wikibase_rest_api

A Rust client library for the [Wikibase REST API](https://doc.wikimedia.org/Wikibase/master/js/rest-api/).
It provides async, type-safe access to Wikibase instances (including [Wikidata](https://www.wikidata.org)) for reading and writing items, properties, labels, descriptions, aliases, statements, and sitelinks.
It works on any MediaWiki installation with the Wikibase extension and an enabled REST API.

New to Wikibase data? Items (`Q…`) and properties (`P…`) are the core objects; each carries multilingual labels/descriptions/aliases, a set of statements (property–value pairs, optionally with qualifiers and references), and — for items — sitelinks to wiki pages. This crate maps all of those to Rust types.

## Features

- 🔍 **Read** items, properties, and their individual parts (labels, descriptions, aliases, statements, sitelinks).
- ✍️ **Write** with full CRUD — create entities, `PUT`/`DELETE` individual parts, and apply JSON Patch (RFC 6902) edits.
- 🌐 **Search** for items and properties by text.
- 🚀 **Bulk-load** many entities concurrently with a configurable parallelism limit.
- 🔁 **Automatic retries** that honour `Retry-After` on rate-limit / server errors.
- 🔑 **Authentication** via OAuth 2.0 access tokens.
- 🧭 **Edit metadata** — attach edit summaries and tags to every write.
- 🛡️ Type-safe IDs and values, `#![forbid(unsafe_code)]`, and no live network calls in the test suite.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
wikibase_rest_api = "0.3"
```

Or install via cargo:

```bash
cargo add wikibase_rest_api
```

You will also need an async runtime; the examples below use [`tokio`](https://tokio.rs):

```bash
cargo add tokio --features macros,rt-multi-thread
```

### Building from source

```bash
git clone https://github.com/magnusmanske/wikibase_rest_api.git
cd wikibase_rest_api
cargo build
```

## Quick start

Read the English label of [Q42](https://www.wikidata.org/wiki/Q42) (Douglas Adams) from Wikidata:

```rust
use wikibase_rest_api::prelude::*;

#[tokio::main]
async fn main() -> Result<(), RestApiError> {
    // Connect to the Wikidata REST API.
    let api = RestApi::wikidata()?;

    // Fetch a single label.
    let id = EntityId::new("Q42")?;
    let label = Label::get(&id, "en", &api).await?;
    println!("Q42 is '{}'", label.value());

    Ok(())
}
```

The [`prelude`](https://docs.rs/wikibase_rest_api/latest/wikibase_rest_api/prelude/) re-exports everything you need. All network methods are `async` and return `Result<_, RestApiError>`.

## Usage

The snippets below are fragments: assume `use wikibase_rest_api::prelude::*;`, an
async context, and an `api` value obtained as shown in
[Connecting to a Wikibase](#connecting-to-a-wikibase). Complete, runnable
programs live in [`examples`](examples).

### Connecting to a Wikibase

```rust
// Wikidata shortcut.
let api = RestApi::wikidata()?;

// Any Wikibase instance, with an (encouraged) descriptive user agent.
let api = RestApi::builder("https://www.wikidata.org/w/rest.php")?
    .with_user_agent("my-app/1.0 (https://example.org; me@example.org)")
    .build()?;
```

### Reading an entity

Load a whole item and read its parts. `Item` and `Property` both implement the
[`Entity`](https://docs.rs/wikibase_rest_api/latest/wikibase_rest_api/entity/trait.Entity.html) trait.

```rust
let item = Item::get(EntityId::new("Q42")?, &api).await?;

// Labels, descriptions and aliases are language-keyed.
if let Some(label) = item.labels().get_lang("en") {
    println!("label: {label}");
}
if let Some(description) = item.descriptions().get_lang("en") {
    println!("description: {description}");
}
println!("en aliases: {:?}", item.aliases().get_lang("en"));

// Sitelinks (items only).
if let Some(sitelink) = item.sitelinks().get_wiki("enwiki") {
    println!("English Wikipedia: {}", sitelink.title());
}
```

Individual parts can also be fetched on their own — cheaper than loading the whole entity:

```rust
let id = EntityId::new("Q42")?;
let label = Label::get(&id, "en", &api).await?;
let sitelink = Sitelink::get(&id, "enwiki", &api).await?;
println!("{} => [[enwiki:{}]]", label.value(), sitelink.title());
```

### Working with statements

Statements are property–value pairs. Fetch them, filter by property, then match
on the value enum.

```rust
let id = EntityId::new("Q42")?;
let statements = Statements::get(&id, &api).await?;

// "instance of" (P31) — the values are item IDs.
for statement in statements.property("P31") {
    if let StatementValue::Value(StatementValueContent::String(other_id)) = statement.value() {
        println!("Q42 is an instance of {other_id}");
    }
}
```

Build statements with the `new_*` constructors, and attach qualifiers and references fluently:

```rust
// A reference is a bundle of property-value "snaks".
let mut reference = Reference::default();
reference.parts_mut().push(
    Statement::new_url("P854", "https://example.org/source").as_property_value(),
);

// P106 "occupation" = Q36180 "writer", backed by that reference.
let statement = Statement::new_item("P106", "Q36180").with_reference(reference);
```

### Searching

```rust
let language = Language::try_new("en")?;
let results = Search::items("Douglas Adams", language).get(&api).await?;

for result in results.iter().take(5) {
    let label = result.display_label().map(|l| l.value()).unwrap_or_default();
    println!("{}: {label}", result.id());
}
```

Use `Search::properties`, `Search::suggest_items`, or `Search::suggest_properties` for the other search modes.

### Loading many entities at once

`EntityContainer` fetches a batch of entities concurrently, with a configurable
limit, and holds the results for you. Missing entities (e.g. deleted IDs) are
simply absent — use `load_report` when you need to know exactly what failed.

```rust
use std::sync::Arc;

let api = Arc::new(RestApi::wikidata()?);
let container = EntityContainer::builder()
    .api(api)
    .max_concurrent(10)
    .build()?;

let ids = ["Q42", "Q1", "P31"]
    .iter()
    .map(|id| EntityId::new(*id))
    .collect::<Result<Vec<_>, _>>()?;

let report = container.load_report(&ids).await;
println!("loaded {}, missing {}", report.loaded().len(), report.missing().len());

if let Some(q42) = container.get_item("Q42").await {
    println!("{:?}", q42.labels().get_lang("en"));
}
```

### Authentication

Read access is anonymous. Writing requires an OAuth 2.0 access token (create one
under *Special:OAuthConsumerRegistration* / *Special:AppManagement* on your wiki):

```rust
let api = RestApi::builder("https://test.wikidata.org/w/rest.php")?
    .with_access_token("YOUR_ACCESS_TOKEN")
    .with_user_agent("my-app/1.0 (me@example.org)")
    .build()?;
```

> **Tip:** experiment against [test.wikidata.org](https://test.wikidata.org) rather than live Wikidata while developing writes.

### Creating and editing

Create a new item:

```rust
let mut item = Item::default();
item.labels_mut().insert(LanguageString::new("en", "Douglas Adams"));
item.descriptions_mut().insert(LanguageString::new("en", "English author"));
item.statements_mut().insert(Statement::new_item("P31", "Q5")); // instance of human

let created = item.post(&api).await?;
println!("created {}", created.id());
```

Set or overwrite a single part with `PUT`; `DELETE` removes one (the language is
taken from the object, so the value can be empty when deleting):

```rust
let id = EntityId::new("Q42")?;

// Create or overwrite the German label.
Label::new("de", "Douglas Adams").put(&id, &api).await?;

// Remove the French description.
Description::new("fr", "").delete(&id, &api).await?;
```

### Edit metadata (summaries & tags)

Attach an edit summary and change tags to any write via the `*_meta` methods:

```rust
let id = EntityId::new("Q42")?;

let mut meta = EditMetadata::default();
meta.set_comment(Some("fix English label".to_string()));
meta.set_tags(vec!["my-bot".to_string()]);

Label::new("en", "Douglas Adams").put_meta(&id, &api, meta).await?;
```

### Patch-based edits

For finer-grained changes, diff two versions of a collection and apply the
resulting [JSON Patch](https://datatracker.ietf.org/doc/html/rfc6902):

```rust
let id = EntityId::new("Q42")?;
let before = Labels::get(&id, &api).await?;

let mut after = before.clone();
after.insert(LanguageString::new("es", "Douglas Adams"));

// Applies only the difference (adds the Spanish label).
let updated = after.patch(&before)?.apply(&id, &api).await?;
```

Whole entities can be patched too, via `Item::patch` / `Property::patch`.

### Error handling

Every network call returns `RestApiError`. Common cases have helpers, so you
don't have to match on message strings:

```rust
match Item::get(EntityId::new("Q0")?, &api).await {
    Ok(item) => println!("{}", item.id()),
    Err(e) if e.is_not_found() => println!("no such item"),
    Err(e) if e.is_rate_limited() => println!("slow down!"),
    Err(e) => return Err(e),
}
```

The client already retries automatically on `429`/`5xx` responses, honouring the
`Retry-After` header (capped via `RestApiBuilder::with_max_retry_after`).

Language codes and site IDs are validated before a request is issued: passing a
malformed value (including one that could inject extra URL path segments) fails
fast with `RestApiError::InvalidLanguageCode` / `RestApiError::InvalidSiteId`
rather than producing a garbled path. Values are trimmed and lower-cased, so
`"EN"` and `"en"` behave identically.

## Examples

Runnable programs live in the [`examples`](examples) directory — run them with
`cargo run --example <name>`:

| Example | What it shows |
| --- | --- |
| [`Q42`](examples/Q42.rs) | Read a label, a sitelink, and statements of a single item |
| [`read_entity`](examples/read_entity.rs) | Load a whole item and print all of its parts |
| [`search`](examples/search.rs) | Search Wikidata for items and print the results |
| [`container`](examples/container.rs) | Bulk-load many entities concurrently |
| [`create_item`](examples/create_item.rs) | Create a new item (needs a token; targets test.wikidata.org) |
| [`add_statement`](examples/add_statement.rs) | Add a statement with an edit summary (needs a token) |

The read-only examples run against live Wikidata as-is; the writing examples
target [test.wikidata.org](https://test.wikidata.org) and need an access token.

## Supported API endpoints

The crate implements the full Wikibase REST API surface.

### Items & properties
- [x] `GET` / `POST` items and properties
- [x] `PATCH` items and properties (`Item::patch` / `Property::patch`)

### Labels, descriptions, aliases
- [x] `GET` / `PATCH` the whole collection (per item and per property)
- [x] `GET` / `PUT` / `DELETE` a single label or description by language code
- [x] `GET` a single label / description with language fallback
- [x] `GET` / `POST` aliases in a given language

### Sitelinks
- [x] `GET` / `PATCH` all sitelinks of an item
- [x] `GET` / `PUT` / `DELETE` a single sitelink

### Statements
- [x] `GET` / `POST` statements of an item or property
- [x] `GET` / `PUT` / `PATCH` / `DELETE` a statement (by entity + statement ID, or by statement ID alone)

### Misc
- [x] `GET /openapi.json`
- [x] `GET /property-data-types`
- [x] Search items and properties (on Wikidata currently only via API `v0`)

## Documentation

- **API reference**: [docs.rs/wikibase_rest_api](https://docs.rs/wikibase_rest_api)
- **Examples**: the [`examples`](examples) directory
- **Wikibase REST API spec**: [doc.wikimedia.org](https://doc.wikimedia.org/Wikibase/master/js/rest-api/)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)

## Contributing

Contributions are very welcome — whether it's a bug report, a feature idea, docs, or code.

- **Bug reports & feature requests**: [GitHub Issues](https://github.com/magnusmanske/wikibase_rest_api/issues)
- **Contribution guidelines**: [CONTRIBUTING.md](CONTRIBUTING.md)
- **Working on the crate itself**: see [DEVELOPMENT.md](DEVELOPMENT.md) for build, test, coverage, and release workflows.

## Security

- **Reporting:** please follow the process in [SECURITY.md](SECURITY.md) — use private [GitHub Security Advisories](https://github.com/magnusmanske/wikibase_rest_api/security/advisories/new); do **not** open a public issue for vulnerabilities.
- **No unsafe code:** the crate is `#![forbid(unsafe_code)]`, so the library contains no `unsafe` blocks.
- **Transport security:** all requests go over HTTPS via `reqwest` (rustls TLS backend).
- **Path-segment validation:** entity IDs, language codes, and site IDs are validated before being interpolated into a REST path, rejecting values that could inject extra URL path segments.

## License

This project is dual-licensed under the [MIT](LICENSE-MIT) and [Apache 2.0](LICENSE-APACHE2) licenses. You may choose either license at your option.
