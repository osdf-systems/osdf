# Phase A test vectors

## Valid fixtures

| File | Description |
| --- | --- |
| `valid/minimal-draft.osdf` | Revision 0 draft package with header, content, manifest, envelope |
| `valid/minimal-committed.osdf` | Revision 1 with signed revision chain |

Regenerate:

```bash
cargo run -p osdf-cli -- create test-vectors/valid/minimal-draft.osdf --title "Minimal Draft"
cargo run -p osdf-cli -- create test-vectors/valid/minimal-committed.osdf --title "Minimal Committed" --commit
```

## Invalid fixtures

Malformed packages are also generated in integration tests. Add additional `.osdf` files under `invalid/` to extend coverage.

Verify all valid fixtures:

```bash
cargo run -p osdf-cli -- verify test-vectors/valid/minimal-committed.osdf
```
