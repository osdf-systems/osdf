# osdf-wasm

Read-only WebAssembly bindings for browser-based OSDF verification.

## Exports

| Function | Description |
| --- | --- |
| `verify_osdf(bytes)` | Returns a structured `VerificationReport` as a JavaScript object |
| `version()` | Crate version string |

## Build

```bash
wasm-pack build crates/osdf-wasm --target web --release --out-dir ../../web/pkg
```

## Tests

```bash
cargo test -p osdf-wasm
wasm-pack test --node crates/osdf-wasm
```

The WASM crate depends on `osdf-core` with `default-features = false` and `features = ["verify-only"]` so the browser build excludes document creation, signing, and entropy-dependent code paths.
