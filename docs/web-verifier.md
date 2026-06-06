# Browser verifier

OSDF includes a static browser verifier compiled from the same Rust verification library used by the CLI.

The verifier:

- runs locally in the browser;
- does not upload the selected file;
- validates the passive-content policy;
- validates the package manifest;
- rejects undeclared objects;
- verifies the revision chain;
- verifies cryptographic signatures.

## Build

```powershell
.\scripts\build-wasm.ps1
```

## Serve locally

```powershell
.\scripts\serve-web.ps1
```

Open `http://localhost:8080/`.

## Test fixtures

Generate committed fixtures under `fixtures/`:

```bash
cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored
```

## Parity testing

```bash
cargo test
wasm-pack test --node crates/osdf-wasm
```

## Current limitations

The browser verifier currently performs local structural and cryptographic verification only.

It does not yet:

- query a transparency ledger;
- resolve organizational credentials;
- check key revocation;
- verify trusted timestamps;
- import or render PDF files;
- create, revise, or sign OSDF files.
