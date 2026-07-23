# Development

Notes for working on `wikibase_rest_api` itself. If you just want to *use* the
crate, the [README](README.md) is the place to start. For contribution
guidelines (coding standards, PR process), see [CONTRIBUTING.md](CONTRIBUTING.md).

## Prerequisites

- A recent stable Rust toolchain (see `rust-version` in [`Cargo.toml`](Cargo.toml) for the MSRV).
- Optional tooling for the checks below: `cargo-tarpaulin` (coverage), [`lizard`](https://github.com/terryyin/lizard) (complexity), and a `nightly` toolchain with the `miri` component (undefined-behaviour checks).

## Everyday commands

```bash
cargo build                 # Build
cargo test                  # Run the whole test suite
cargo test <name>           # Run a single test by name
cargo fmt --all             # Format
cargo clippy --all-targets  # Lint (CI runs with -D warnings)
```

Tests use [`wiremock`](https://docs.rs/wiremock) to mock HTTP — there are **no
live network calls** in the test suite. Fixtures live in `test_data/`.

## Checking for undefined behaviour (Miri)

```bash
rustup toolchain install nightly --component miri   # once
rustup run nightly cargo miri test --lib
```

`--lib` is required: Miri cannot build the doctests on current nightly, and all
of the crate's tests are colocated in `src/` anyway. Network tests are marked
`#[cfg_attr(miri, ignore)]` and are skipped automatically.

## Code coverage

```bash
cargo install cargo-tarpaulin   # once
cargo tarpaulin -o html         # detailed HTML report
```

## Complexity & other metrics

```bash
lizard src -C 7 -V -L 40        # cyclomatic complexity per function
./analysis.sh                   # writes lizard + rust-code-analysis output to code-metrics/
```

`code-metrics/` is a local, git-ignored scratch directory for generated
reports; nothing in it is tracked.

## Badges

The **AvgCCN** and **coverage** badges in the README are generated. Refresh both
with a single command:

```bash
./scripts/update-badges.sh      # = update-ccn.sh + update-coverage.sh
```

Individual refresh scripts also exist (`./scripts/update-ccn.sh`,
`./scripts/update-coverage.sh`).

A pre-commit hook regenerates both badges automatically whenever a commit
changes the crate version in `Cargo.toml`. Enable it once per clone:

```bash
git config core.hooksPath .githooks
```

## Continuous integration

CI (`.github/workflows/rust.yml`) enforces, on every push and pull request:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo miri test --lib`

Coverage is tracked via the README badge but is **not** gated in CI.
Dependency and GitHub-Actions updates are proposed automatically by Dependabot
(`.github/dependabot.yml`).

## Releasing

1. Bump `version` in `Cargo.toml` (the pre-commit hook refreshes the badges).
2. Add a dated entry to [`CHANGELOG.md`](CHANGELOG.md).
3. Commit, then tag: `git tag -a vX.Y.Z -m "Release X.Y.Z"`.
4. `git push origin main && git push origin vX.Y.Z`.
5. `cargo publish`.
