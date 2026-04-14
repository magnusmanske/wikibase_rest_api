# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |

Only the latest published version on [crates.io](https://crates.io/crates/wikibase_rest_api) is actively supported with security updates.

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please use [GitHub's private vulnerability reporting](https://github.com/magnusmanske/wikibase_rest_api/security/advisories/new) to submit your report. This ensures the vulnerability can be assessed and addressed before public disclosure.

### What to include

- A description of the vulnerability
- Steps to reproduce the issue
- The potential impact
- Any suggested fixes (optional)

### Response timeline

- **Initial response**: within 7 days of the report
- **Assessment**: within 14 days of the report
- **Fix**: critical vulnerabilities will be addressed as soon as possible; non-critical vulnerabilities will be fixed in the next release

### After a fix is released

Once a fix has been released, the vulnerability will be publicly disclosed in the [CHANGELOG](CHANGELOG.md) and, if applicable, via a GitHub Security Advisory.

## Security Design

This crate follows several security best practices:

- **No unsafe code**: `unsafe` code is forbidden via `#[forbid(unsafe_code)]`
- **No unwrap in production code**: `clippy::unwrap_used` is denied
- **Miri testing**: The test suite is run under [Miri](https://github.com/rust-lang/miri) to detect undefined behavior
- **Dependency auditing**: The project uses the [OpenSSF Scorecard](https://github.com/ossf/scorecard-action) for supply chain security analysis
