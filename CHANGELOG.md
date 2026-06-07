# Changelog

All notable changes to this project are documented here.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- License: PolyForm Noncommercial 1.0.0 (planned relicense to Apache-2.0 OR MIT at v1.0; see `docs/licensing.md`)

### Added

- GitHub CI, nightly builds, and release workflow scaffolding
- `osdf version` subcommand
- `osdf verify --format json` (alias: `--json`)
- Demonstration package plan (`specs/demo-package.md`) and script (`scripts/run-demo-package.ps1`)
- Transparent Gateway PoC and tax form fixtures
- Latest-revision registry (B.3 M2) with outdated-revision warnings

## [0.1.0-alpha.1] - TBD

First public alpha — verification-only release.

### Included

- Rust verification core (`osdf-core`)
- CLI verifier (`osdf`)
- WASM browser verifier
- Container safety, manifest, revision, and signature checks
- Ed25519 signature verification
- Organizational identity resolution from configured trust material
- Embedded transparency-log proof verification
- Human-readable and JSON verification output
- Valid and invalid test fixtures
- Specification drafts under `specs/`

### Not included

- Production hosted ledger or live latest-revision service
- Revocation service

[Unreleased]: https://github.com/osdf-systems/osdf/compare/v0.1.0-alpha.1...HEAD
[0.1.0-alpha.1]: https://github.com/osdf-systems/osdf/releases/tag/v0.1.0-alpha.1
