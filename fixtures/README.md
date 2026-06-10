# Fixtures

Committed test packages for CLI, native, and WASM verification parity.

## Valid

| File | Description |
| --- | --- |
| `valid/valid-draft.osdf` | Revision 0 draft package |
| `valid/valid-committed.osdf` | Revision 1 with signed revision chain |

## Invalid

| File | Expected failure area |
| --- | --- |
| `invalid/missing-magic.osdf` | Container — invalid header magic |
| `invalid/duplicate-path.osdf` | Container — duplicate ZIP path |
| `invalid/path-traversal.osdf` | Container — unsafe path |
| `invalid/trailing-bytes.osdf` | Container — bytes after ZIP end |
| `invalid/undeclared-object.osdf` | Manifest — undeclared object |
| `invalid/missing-declared-object.osdf` | Manifest — missing declared object |
| `invalid/tampered-content-hash.osdf` | Manifest — content hash mismatch |
| `invalid/tampered-signature.osdf` | Signatures — invalid signature bytes |
| `invalid/missing-signature.osdf` | Signatures — missing signature file |
| `invalid/fake-parent-commitment.osdf` | Revision — parent commitment mismatch |
| `invalid/deleted-parent-revision.osdf` | Revision — missing revision record |

## OpenTDF comparison (benchmarks)

Third-party TDF golden files for [docs/benchmarks.md](../docs/benchmarks.md): see [benchmarks/opentdf/](benchmarks/opentdf/README.md).

## Regenerate

```bash
cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored
```

## Verify

```bash
cargo run -p osdf-cli -- verify fixtures/valid/valid-committed.osdf
cargo run -p osdf-cli -- verify fixtures/invalid/undeclared-object.osdf
```

Each invalid fixture should produce a distinct, stable error code in the structured verification report.
