# Phase B specification notes

Browser verifier using the same Rust core as the CLI.

## Deliverables

- `crates/osdf-wasm` — read-only wasm-bindgen exports
- `web/` — static drag-and-drop verifier UI
- `fixtures/` — valid and invalid committed test packages
- Parity tests in `crates/osdf-wasm/tests/parity.rs`

## WASM API (read-only)

| Export | Purpose |
| --- | --- |
| `verify_osdf(bytes)` | Returns structured `VerificationReport` object (camelCase fields) |
| `version()` | Crate version string |

No creation, signing, or mutation APIs are exported to the browser.

## Verification report

CLI and browser both consume `osdf_core::VerificationReport`:

- `overall`: `PASS` | `WARNING` | `FAIL`
- `checks[]`: coded structural and cryptographic checks
- `errors[]` / `warnings[]`: coded messages (no panics on malformed input)

## Build

```bash
# Windows
powershell -ExecutionPolicy Bypass -File scripts/build-wasm.ps1

# macOS / Linux
./scripts/build-wasm.sh
```

Serve `web/` over HTTP:

```powershell
.\scripts\serve-web.ps1
```

Open `http://localhost:8080/` and drop a `.osdf` file.

## Exit criterion

`fixtures/valid/valid-committed.osdf` must report **PASS** in:

1. `osdf verify …` (CLI)
2. Browser verifier (WASM)

Parity tests enforce matching reports for shared fixtures.

See `docs/web-verifier.md` for full build and limitation notes.
