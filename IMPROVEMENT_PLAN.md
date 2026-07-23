# Improvement Plan

Findings from a code review (2026-07-23), organized so they can be worked through over time.
Priorities: **reliability, usability, speed** — in that order. This is a crate used by others,
so external-facing changes are made deliberately and batched (see "Versioning strategy" at the end).

Guiding principle: **KISS for the user.** The default behavior of every API must be the sensible
one; configuration exists for the exceptions, not the common case. No silent data loss, ever.

Status legend: `[ ]` open · `[x]` done · `[~]` in progress

---

## Phase 0 — Design: entity loading, errors, and retries (do this before Phase 1)

### 0.1 `[ ]` Design the EntityContainer retry/error model

**Problem.** `EntityContainer::load()` silently swallows per-entity fetch errors
(`entity_container.rs:89` and `:121` — `Vec<Result<...>>` is `.flatten()`ed, dropping every `Err`).
A 429 that exhausted retries, a 5xx, or a network failure is indistinguishable from
"entity never requested". The doc comment even claims an error is returned. Bulk jobs
silently process incomplete data.

**Design constraints:**

- KISS: `load()` stays a one-call, no-configuration entry point with safe defaults.
- 429 is a special case: with N concurrent loaders, one rate-limit event tends to fail
  *many* in-flight requests at once. Retrying them all immediately at full concurrency
  makes it worse.
- 404 is *not* an error for bulk loading: deleted/missing entities are routine on Wikidata.
- Every knob must have a sensible default and be overridable via the builder.

**Proposed model — two layers of retry, each with one job:**

1. **Request layer (already exists):** `RestApi::execute()` retries 429/5xx per request,
   honoring `Retry-After`, with exponential backoff. Keep this as the only place that
   retries an *individual* request. (Hardened separately in 1.5.)

2. **Container layer (new):** handles the *fleet* problem — what to do when requests still
   fail after the request layer gave up.
   - Classify each per-entity outcome: `Loaded`, `Missing` (HTTP 404), or `Failed(RestApiError)`.
   - After a sweep, if any failures were rate-limits (429), wait out the largest observed
     `Retry-After` (fallback: container backoff delay), then re-sweep **only the failed IDs**
     at **halved concurrency**. Repeat up to `container_retries` times.
   - Non-429 failures are *not* retried at container level (the request layer already
     retried them 3×); they go straight into the failure report.

**Proposed API (KISS-first):**

```rust
// The simple path: all-or-error. Missing (404) entities are simply absent afterwards;
// any other failure aborts with the first error. No silent gaps, no configuration needed.
pub async fn load(&self, entity_ids: &[EntityId]) -> Result<(), RestApiError>;

// The bulk path: never aborts, reports everything.
pub async fn load_report(&self, entity_ids: &[EntityId]) -> LoadReport;

pub struct LoadReport {
    pub loaded: Vec<EntityId>,
    pub missing: Vec<EntityId>,                    // 404 — not an error
    pub failed: Vec<(EntityId, RestApiError)>,     // everything else, after all retries
}
```

**Defaults (all overridable on `EntityContainerBuilder`):**

| Knob | Default | Notes |
|---|---|---|
| `max_concurrent` | 10 | exists today |
| `container_retries` | 2 | re-sweeps of 429-failed IDs |
| `container_backoff` | 5 s | used when no `Retry-After` was seen |
| treat 404 as missing | always | not configurable — it's the only correct bulk semantic |

**Decisions to make before implementing:**
- [ ] Does `RestApiError` need to expose "was this a 429?" cleanly? (Today: match
      `ApiError { status, .. }`. Probably add `RestApiError::is_rate_limited(&self) -> bool`
      and `retry_after(&self) -> Option<Duration>` helpers — additive, no break.)
- [ ] Should `load()` be implemented as a thin wrapper over `load_report()`? (Yes, one code path.)
- [ ] `LoadReport` field visibility: public fields (simple) vs. getters (consistent with the
      rest of the crate). Lean getters for consistency.

### 0.2 `[ ]` Implement it

- Refactor `fetch_items`/`fetch_properties` into one generic sweep that returns
  `Vec<(EntityId, Result<E, RestApiError>)>` — no more `.flatten()`.
- Implement the container-level 429 re-sweep (subset of IDs, halved concurrency, waited delay).
- `load()` = `load_report()` + "return first `failed` error, ignore `missing`".
- Fix the `load()` doc comment to describe the real semantics.
- Tests (wiremock): mixed sweep (ok + 404 + 500); 429-then-success re-sweep proves only
  failed IDs are refetched; report classification; `load()` aborts on 500 but not on 404.

---

## Phase 1 — Reliability

### 1.1 `[ ]` Search: stop dropping malformed results
`search.rs:192` — `filter_map(.ok())` silently shortens result lists when the server schema
shifts. Collect into `Result<Vec<_>, _>` and return the error. Also deserialize with
`SearchResult::deserialize(value)` to drop the per-result `.clone()`.

### 1.2 `[ ]` Remove panic paths
- `HttpMisc::get_rest_api_path` default impl is `panic!` (`get_put_delete.rs:12`).
  Remove the default body (compile-time enforcement) or return `Err`. *(Breaking for
  external implementors — batch into 0.2.0.)*
- `Display for EntityId::None` returns `fmt::Error` (`entity_id.rs:106`), which makes
  `format!("{id}")` panic. Render an empty string instead.

### 1.3 `[ ]` `RestApiBuilder::build()` must not silently degrade
`rest_api_builder.rs:142` — on client-build failure, configured timeouts are silently
dropped via `unwrap_or_default()` (and `Client::default()` can itself panic).
Make `build()` return `Result<RestApi, RestApiError>`. *(Breaking — 0.2.0.)*

### 1.4 `[ ]` Unify the error surface
GETs via `api_execute` (`get_put_delete.rs:51`) and `Statement::put_match`
(`statement.rs:259`) surface HTTP failures as `RestApiError::Reqwest` (no body), while
other paths parse the payload into `ApiError { status, payload }`. Users can't match
"item-not-found" uniformly. Route **every** non-success response through
`RestApiError::from_response`; fold `filter_response_error` and `api_execute` into one
helper. Behavioral change only in error *variants* — document in changelog.

### 1.5 `[ ]` Harden request-level retry (`rest_api.rs:75–130`)
- Parse HTTP-date `Retry-After` (the `httpdate` dep is already there).
- Cap the honored `Retry-After` (default 60 s, builder-configurable) — never let a server
  header sleep the client for a day.
- Add jitter to exponential backoff (±25 %); avoids synchronized retry storms from bots.
  Note: `Math.random`-style jitter needs a seed source — `rand` is a new dep; a hash of
  attempt+time via `std` is enough, keep it dependency-free if possible.
- Non-cloneable request on attempt 0: execute the original request once (no retry)
  instead of failing with the misleading `EmptyValue("request not cloneable")`.
  Give the "can't retry" case its own error variant if it survives.

### 1.6 `[ ]` Remove the live-network test
`search.rs` `test_search` hits production Wikidata → flaky CI, contradicts the crate's own
wiremock convention. Replace with a wiremock fixture (a captured response already fits the
`test_data/` pattern).

### 1.7 `[ ]` Verify statement add/remove patch paths
`statements.rs:152,169` emit `/statements/{statement_id}` with `// TODO check`. The entity
JSON keys statements as `/statements/{property_id}/{index}`, so adds are likely rejected by
the server. Verify against the REST API spec / a test instance, fix the paths, and pin the
exact patch document in a wiremock test. Also: iterate properties in sorted order so patch
output is deterministic (reproducibility + golden tests).

---

## Phase 2 — Usability

### 2.1 `[ ]` Drop `&mut RestApi` from write operations
`HttpPut::put_meta`, `HttpDelete::delete_meta`, `PatchApply::apply(_match)`,
`Statement::put/delete(_match)` all take `&mut RestApi` but mutate nothing (token state is
behind `Arc<RwLock>`; `execute()` takes `&self`). This blocks sharing one `&RestApi` across
concurrent edits. Change to `&RestApi`. *(Technically breaking, but `&mut` coerces to `&`,
so nearly all call sites keep compiling — 0.2.0.)*

### 2.2 `[ ]` Validate `EntityId` at construction
`EntityId::item("Q42/labels/en")` currently reaches the URL path unescaped (path injection),
and `new()` only checks the first letter. Validate `^[QP][1-9][0-9]*$` (respecting `Config`
letters) in `new`/`item`/`property`, returning `Result`. `nutype` + regex is already the
crate's pattern. *(`item`/`property` becoming fallible is breaking — 0.2.0. Alternative if
that's too disruptive: keep constructors infallible, validate/percent-encode at path build
in `entity_path()` — decide during implementation.)*

### 2.3 `[ ]` Tighten the public surface (pre-1.0 housekeeping)
- Make `RestApi.token` field private (the `token()` getter already exists).
- Replace `pub use get_put_delete::*` / `pub use patch::*` with explicit re-exports.
- Audit `pub mod` list in `lib.rs`; plumbing modules → `pub(crate)`.
- Consider sealing `HttpMisc` (its `run_json_query`/`generate_json_request` are
  implementation details users shouldn't call).
*(All breaking in theory — 0.2.0. Cheap now, expensive after 1.0.)*

### 2.4 `[ ]` EntityContainer convenience accessors
Add `get_item(&EntityId) -> Option<Item>` / `get_property(...)` (clone under read lock) so
common use never touches `Arc<RwLock<HashMap<...>>>` directly. Additive. Pairs naturally
with Phase 0.

---

## Phase 3 — Speed

### 3.1 `[ ]` Slim tokio features
`Cargo.toml`: `tokio = { features = ["full"] }` drags process/signal/fs/net into every
downstream build. The crate uses `sync` (RwLock) and `time` (sleep) only. Move
`rt`/`macros`/`rt-multi-thread` to dev-dependencies for `#[tokio::test]`. Pure win, no API
impact. **Quick win — can be done any time.**

### 3.2 `[ ]` Read-lock fast path for the bearer token
`rest_api.rs:79` — every request (including the GET-heavy container fan-out) takes an
exclusive write lock just to have `check()` return immediately. Decide under a *read* lock
whether renewal is needed (GET, or token fresh); escalate to write only for actual renewal.
Re-check staleness after acquiring the write lock (double-checked locking).

### 3.3 `[ ]` Allocation/clone cleanups (internal, individually small)
- `request_builder` (`rest_api.rs:240–243`): stop attaching `.form(&params)` to
  PUT/PATCH/DELETE bodies that `generate_json_request` immediately overwrites; only GET
  genuinely uses `params`.
- `Statements::property` (`statements.rs:71`): take `&str`, not `Into<String>` — no
  allocation for a lookup.
- `Item::patch` (`item.rs:194–199`): consume the five sub-patches instead of `.to_owned()`.
- `Entity::id()`: return `&EntityId` instead of a clone. *(Breaking for trait implementors —
  0.2.0.)*

### 3.4 `[ ]` `Statements::len()` / `is_empty()` consistency
A property key with an empty `Vec` gives `len() == 0` but `is_empty() == false`. Make
`is_empty()` = `self.len() == 0`.

---

## Phase 4 — Small latent bugs / hygiene

### 4.1 `[ ]` Fix the malformed PATCH content type
`rest_api.rs:52` sets `"json-patch+json"` (missing `application/` prefix). Currently
unreachable through normal flows (overridden in `generate_json_request`), but
`wikibase_request_builder` is `pub`. Fix the literal; consider whether the PATCH branch
there is needed at all once 1.4 unifies request building.

### 4.2 `[ ]` Doc pass
- `EntityContainer::load` docs (done as part of 0.2).
- README / doc examples use `.unwrap()` liberally — fine in `no_run` snippets, but prefer
  `?` in the README main example since the lint philosophy of the crate is "no unwrap".
- Document retry behavior (defaults, `Retry-After` cap, jitter) on `RestApiBuilder`.

---

## Versioning strategy

- **Additive / internal items** (0.1.x patch releases): 0.1/0.2 design+impl, 1.1, 1.5, 1.6,
  1.7, 2.4, 3.1, 3.2, 3.3 (except `Entity::id()`), 3.4, 4.1, 4.2.
- **Breaking items — batch into one 0.2.0 release** with a migration section in the
  changelog: 1.2 (trait default removal), 1.3 (`build() -> Result`), 1.4 (error-variant
  changes, arguably behavioral), 2.1 (`&mut` → `&`), 2.2 (fallible constructors, if chosen),
  2.3 (surface tightening), `Entity::id()` from 3.3.

Suggested order of work: **0.1 → 0.2 → 3.1** (quick win) → **1.5 → 1.4 → 1.6** → rest of
Phase 1 → Phase 2 batched as the 0.2.0 push → Phase 3/4 as filler tasks.

Every item lands with: `cargo fmt`, `cargo clippy --all-targets` (zero warnings), tests
(wiremock, no live network), and coverage kept at 100 %.
