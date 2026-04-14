# Contributing to wikibase_rest_api

Thank you for considering contributing to this project! This document explains how to contribute effectively.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally
3. Create a new branch for your changes
4. Make your changes
5. Push to your fork and submit a pull request

## Building

```bash
cargo build
```

## Coding Standards

- **Rust edition**: 2021
- **No unsafe code**: `unsafe` code is forbidden. The project uses `#[forbid(unsafe_code)]`.
- **No unwrap in production code**: Use proper error handling with `Result` and `?`. The clippy lint `unwrap_used` is denied. (`unwrap` is allowed in tests via `clippy.toml`.)
- **Formatting**: Run `cargo fmt` before submitting changes.
- **Linting**: Run `cargo clippy` and address all warnings before submitting.

## Testing

### Policy

All new functionality must be accompanied by tests. Bug fixes should include a test that reproduces the issue. The project aims to maintain its current high level of code coverage (100%).

### Running Tests

```bash
# Run the full test suite
cargo test

# Run tests with Miri (checks for undefined behavior)
cargo +nightly miri test

# Generate a code coverage report
cargo tarpaulin -o html
```

### Writing Tests

- Place unit tests in the same file as the code they test, inside a `#[cfg(test)]` module.
- Use the test data fixtures in the `test_data/` directory for mock responses.
- Use `wiremock` for testing HTTP interactions.

## Code Analysis

Run the full analysis suite:

```bash
./analysis.sh
```

This runs Lizard (cyclomatic complexity), Tarpaulin (code coverage), and rust-code-analysis.

## Reporting Bugs

- Use [GitHub Issues](https://github.com/magnusmanske/wikibase_rest_api/issues) to report bugs.
- Include steps to reproduce, expected behavior, and actual behavior.
- Include your Rust version (`rustc --version`) and OS.

## Suggesting Enhancements

- Open a [GitHub Issue](https://github.com/magnusmanske/wikibase_rest_api/issues) describing the enhancement.
- Explain the use case and why it would be useful.

## Pull Request Process

1. Ensure your code compiles without warnings.
2. Ensure all existing and new tests pass.
3. Run `cargo fmt` and `cargo clippy`.
4. Update documentation if your changes affect the public API.
5. Keep pull requests focused on a single change.

## Security Vulnerabilities

Please see [SECURITY.md](SECURITY.md) for how to report security vulnerabilities. Do not open public issues for security problems.
