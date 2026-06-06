# Phase D M2 — Hosted ledger (local / LAN)

Follow-up to file-backed latest-revision registry (B.3 M2). Goal: run a transparency log + revision registry on a developer PC temporarily, with verifiers fetching live state instead of a static `ledger-trust.json` snapshot.

## Planned scope

- Minimal HTTP service exposing:
  - `GET /v1/log/{logId}/tree-head` — signed tree head
  - `GET /v1/log/{logId}/proof/{leafIndex}` — inclusion proof for a leaf
  - `GET /v1/log/{logId}/latest-revision/{documentId}` — registry lookup
  - `POST /v1/log/{logId}/append` — append revision event (authenticated with log operator key)
- `LedgerConfig` extension: `registryUrl` or `logBaseUrl` for online fetches
- Verifier: when URL configured, populate `latestRevisions` at verify time and set `verificationMode: onlineEnhanced` when live checks succeed
- CLI: `osdf ledger serve --store ledger.json --key ledger-key.json --port 8090`
- Local dev script: `scripts/serve-ledger.ps1`

## Out of scope (initial)

- TLS / production hardening
- Multi-log federation
- Revocation stream (B.3 M4)

## Current workaround

Use file-backed workflow:

```powershell
osdf ledger append --store ledger.json --package doc.osdf
osdf ledger attach-proof ... --trust-config trust.json
osdf verify doc.osdf --ledger-config trust.json
```

The trust JSON embeds `latestRevisions` from the store at export time.
