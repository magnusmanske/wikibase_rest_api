[![Crates.io](https://img.shields.io/crates/v/wikibase_rest_api?style=flat-square)](https://crates.io/crates/wikibase_rest_api)
[![Crates.io](https://img.shields.io/crates/d/wikibase_rest_api?style=flat-square)](https://crates.io/crates/wikibase_rest_api)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE-MIT)
[![License](https://img.shields.io/badge/license-APACHE2-blue?style=flat-square)](LICENSE-APACHE2)
[![Codacy Badge](https://app.codacy.com/project/badge/Grade/acffb6bb26d8407b8e82704843a4aa7e)](https://app.codacy.com/gh/magnusmanske/wikibase_rest_api/dashboard?utm_source=gh&utm_medium=referral&utm_content=&utm_campaign=Badge_grade)
AvgCCN 2.1
Codecov 100.00%

This Rust crate provides a REST API for Wikibase.
It is based on the [Wikibase REST API](https://doc.wikimedia.org/Wikibase/master/js/rest-api/).
It works on any MediaWiki installation with the Wikibase extension and an enabled Wikibase REST API.

# Usage
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
let item: Item = item.post(&api).await.unwrap();
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
entity_container.load(&entity_ids).await?;
let q42 = entity_container
    .items()
    .read()
    .await
    .get("Q42")
    .unwrap()
    .to_owned();
let q42_label_en = q42.labels().get_lang("en").unwrap();
println!("Q42 label[en]: {q42_label_en}");

// Search for "Tim Berners-Lee" (in English) on Wikidata.
let query = "Tim Berners-Lee";
let language = Language::try_new("en").unwrap();
let api = RestApi::builder("https://www.wikidata.org/w/rest.php")
    .unwrap()
    .with_api_version(0) // Currently only works with v0 not v1
    .build();
let results = Search::items(query, language).get(&api).await.unwrap();
println!("{}",results[0].id());
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

# Developer notes
## TODO
- Maxlag/rate limits?

Code analysis is run via `analysis.sh`.

## Code coverage
```bash
cargo install cargo-tarpaulin # Once
cargo tarpaulin -o html
```

## Lizard
Lizard is a simple code analyzer, giving cyclomatic complexity etc.
https://github.com/terryyin/lizard
```bash
lizard src -C 7 -V -L 40
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
