# Phase D — Transparency ledger (M1)

Phase D M1 adds append-only transparency log proofs to committed OSDF revisions. Verifiers can optionally or mandatorily check that a revision’s `revisionEventHash` appears in a trusted log with a valid Merkle inclusion proof and signed tree head.

> **Public docs:** this spec and `docs/` (HTML docs site is maintained outside this repo).

## Scope (M1)

- File-backed `LedgerStore` for development and CLI workflows
- Merkle append-only log over `revisionEventHash` digests (domain-separated leaf hashing)
- Per-revision proof objects at `transparency/rev-NNNNNN.proof.json`
- `LedgerConfig` trust registry (`logId` + `logPublicKeyUrn`)
- Verifier transparency audit section with PASS/FAIL checks
- CLI: `ledger init`, `ledger append`, `ledger attach-proof`, `verify --ledger-config`
- WASM: `verify_osdf_with_ledger(bytes, ledgerConfigJson)`

Out of scope for M1: HTTP log servers, key revocation, multi-log federation.

## Log leaf semantics

Each log entry is the existing revision record’s `revisionEventHash` (32-byte SHA-256 digest). The Merkle tree hashes leaves with domain separation:

- Leaf: `OSDF-LOG-LEAF-v1 || revisionEventHash`
- Node: `OSDF-LOG-NODE-v1 || left || right`

## Proof object

Path: `transparency/rev-000001.proof.json` (sibling to revision/signature objects, declared in `manifest.objects[]`, **not** part of the revision Merkle root).

```json
{
  "proofVersion": "1",
  "logId": "urn:osdf:log:fixture",
  "logEntryId": "urn:osdf:log:fixture#0",
  "leafIndex": 0,
  "treeSize": 1,
  "revisionEventHash": "sha256:…",
  "inclusionProof": ["sha256:…"],
  "signedTreeHead": {
    "logId": "urn:osdf:log:fixture",
    "treeSize": 1,
    "rootHash": "sha256:…",
    "timestamp": "2026-06-04T12:00:00Z",
    "logKeyReference": "urn:osdf:key:ed25519:…",
    "algorithm": "Ed25519",
    "signature": "…"
  }
}
```

## Trust configuration

```json
{
  "policy": "required",
  "trustedLogs": [
    {
      "logId": "urn:osdf:log:fixture",
      "logPublicKeyUrn": "urn:osdf:key:ed25519:…"
    }
  ]
}
```

Policies:

| Policy | Behavior |
| --- | --- |
| `disabled` | Transparency section emits INFO stubs only (default) |
| `optional` | Verify proof when present; trust registry enforced when checking |
| `required` | Committed revisions must include a valid proof |

## CLI workflow

```powershell
# 1. Create committed package
cargo run -p osdf-cli -- create out.osdf --title "Doc" --commit

# 2. Initialize ledger
cargo run -p osdf-cli -- ledger init --store ledger.json --key ledger-key.json

# 3. Append revision event
cargo run -p osdf-cli -- ledger append --store ledger.json --package out.osdf

# 4. Attach proof + write trust config
cargo run -p osdf-cli -- ledger attach-proof `
  --store ledger.json --key ledger-key.json --package out.osdf `
  --output out-with-proof.osdf --trust-config trust.json

# 5. Verify with ledger
cargo run -p osdf-cli -- verify out-with-proof.osdf --ledger-config trust.json
```

## Verification checks

When ledger policy is enabled:

| Code | Description |
| --- | --- |
| `OSDF_LEDGER_PROOF_PRESENT` | Proof file exists for committed revision |
| `OSDF_LEDGER_LEAF_MATCHES_EVENT_HASH` | Proof leaf matches revision record |
| `OSDF_LEDGER_INCLUSION_PROOF_VALID` | Merkle path recomputes to signed root |
| `OSDF_LEDGER_SIGNED_ROOT_VALID` | Signed tree head verifies under log key |
| `OSDF_LEDGER_LOG_KEY_TRUSTED` | Log id/key in configured trust registry |

Revocation remains `OSDF_REVOCATION_NOT_CONFIGURED` (INFO stub).

## Fixtures

- `fixtures/valid/valid-with-ledger-proof.osdf`
- `fixtures/valid/ledger-trust.json`
- `fixtures/invalid/ledger-leaf-mismatch.osdf`
- `fixtures/invalid/ledger-bad-inclusion.osdf`

Regenerate: `cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored`

## Browser verifier

Optional ledger trust JSON can be passed to `verify_osdf_with_ledger`. No network access is required; configuration is supplied locally by the page or user.
