# Changelog

All notable changes to this project are documented here.

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0-alpha.2](https://github.com/osdf-systems/osdf/releases/tag/v0.1.0-alpha.2) - TBD

Incremental alpha - verification profiles, hardening, and public roadmap.

### Added

- Verification profiles: portable full report, fast verify API, parsed-container revalidation (`verify_profile`, `verify_fast`)
- `scale_bench` example with `--profile full|fast|parsed`
- Public roadmap (`docs/roadmap.md`) and interoperability design (`docs/interoperability.md`)
- Draft specs: transformation receipt, offline verification bundle (`specs/` + JSON Schema stubs)

### Changed

- Side-channel hardening: constant-time digest comparison on audit paths; full-scan manifest audit (`SECURITY.md`)
- Manifest JSON stored uncompressed; compression-bomb ratio check scoped to large payloads (`MIN_COMPRESSION_RATIO_CHECK_BYTES`)

## [0.1.0-alpha.1](https://github.com/osdf-systems/osdf/releases/tag/v0.1.0-alpha.1) - TBD

First public alpha: verification-only release.

### Added

- Rust verification core (`osdf-core`), CLI verifier (`osdf`), WASM browser verifier
- Container safety, manifest, revision, and signature checks; Ed25519 signature verification
- Organizational identity resolution from configured trust material
- Embedded transparency-log proof verification
- Human-readable and JSON verification output; valid and invalid test fixtures
- GitHub CI, nightly builds, and release workflow scaffolding
- `osdf version` subcommand; `osdf verify --format json` (alias: `--json`)
- Demonstration package plan (`specs/demo-package.md`) and script (`scripts/run-demo-package.ps1`)
- Transparent Gateway PoC and tax form fixtures
- Latest-revision registry (B.3 M2) with outdated-revision warnings
- Specification drafts under `specs/`
- License: PolyForm Noncommercial 1.0.0 (planned relicense to Apache-2.0 OR MIT at v1.0; see `docs/licensing.md`)

### Not included

- Production hosted ledger or live latest-revision service
- Revocation service

