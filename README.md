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

# TODO
- Maxlag/rate limits?

# Notes
## Code coverage
```bash
cargo install cargo-tarpaulin # Once
cargo tarpaulin -o html
```

## Lizard
Lizard is a simple code analyzer, giving cyclomatic complexity etc.
https://github.com/terryyin/lizard
```bash
lizard src -C 7 -V -L 35
```
