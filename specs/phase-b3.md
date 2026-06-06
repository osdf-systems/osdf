# Phase B.3 — Freshness, identity, and online verification

Phase B.2 delivers embedded transparency proof verification (offline CT-style inclusion chain). Phase B.3 adds freshness semantics, organizational identity resolution, and optional online enhancement.

> **Public docs:** this spec and `docs/` (HTML docs site is maintained outside this repo).

## Completed (B.2 recap)

| Layer | Status |
| --- | --- |
| Container safety | ✓ |
| Manifest integrity | ✓ |
| Revision chain | ✓ |
| Signature structure + crypto | ✓ |
| Signer identity | ✓ local registry + delegation (B.3 M1) |
| Embedded ledger proof | ✓ |
| Ledger leaf → revision event | ✓ |
| Merkle inclusion proof | ✓ |
| Signed tree head | ✓ |
| Trusted log key | ✓ |
| Latest-revision freshness | ✓ file-backed registry (B.3 M2) |
| Tree-head freshness policy | ⓘ stub |
| Consistency proof | ⓘ stub |
| Revocation | ⓘ stub |
| Online verification mode | ⓘ stub |

## Verification layering (permanent)

The verifier **must always distinguish**:

1. **File integrity** — structure, manifest, revision chain
2. **Cryptographic signature** — valid signature from a known key material
3. **Organizational identity** — key bound to issuer (future)
4. **Ledger inclusion** — revision event appeared in a trusted log state
5. **Freshness** — this is the newest known revision (future live lookup)
6. **Revocation** — signing keys not revoked at signing time (future)

A passing offline result means **(1)+(2)+(4 embedded)** — not “issued by Colorado” and not “latest revision.”

## Current offline stubs (B.2.1)

When embedded ledger verification succeeds, the report also records:

| Code | Meaning |
| --- | --- |
| `OSDF_VERIFICATION_MODE_OFFLINE` | No live services queried |
| `OSDF_LEDGER_TREE_HEAD_FRESHNESS_NOT_CHECKED` | Timestamp not evaluated against policy |
| `OSDF_LEDGER_CONSISTENCY_PROOF_NOT_CHECKED` | No append-only extension proof |
| `OSDF_LEDGER_LATEST_REVISION_NOT_CHECKED` | Inclusion ≠ currentness |
| `OSDF_LIVE_LATEST_REVISION_NOT_CHECKED` | No live revision registry lookup |
| `OSDF_LIVE_REVOCATION_NOT_CHECKED` | No revocation log consulted |
| `OSDF_IDENTITY_NOT_RESOLVED` | Crypto-valid key, unknown organization |

## B.3 milestones

### 1. Signer organizational identity (B.3 M1 — done)

Local trust registry + delegated signing credentials via `IdentityConfig`:

```json
{
  "policy": "required",
  "organizations": [{
    "organizationId": "urn:osdf:org:colorado-state-demo",
    "displayName": "State of Colorado",
    "rootKeys": ["urn:osdf:key:ed25519:..."]
  }],
  "delegations": [{
    "credentialType": "OSDF_ORGANIZATION_SIGNING_DELEGATION",
    "organizationId": "urn:osdf:org:colorado-state-demo",
    "department": "Department of Revenue",
    "subjectKey": "urn:osdf:key:ed25519:...",
    "validFrom": "2026-01-01T00:00:00Z",
    "validUntil": "2027-01-01T00:00:00Z",
    "issuerKey": "urn:osdf:key:ed25519:...",
    "signature": "..."
  }]
}
```

CLI: `osdf verify doc.osdf --identity-config identity-trust.json`  
Browser: paste into **Identity config** textarea (or combined `VerifierConfig` JSON via WASM).

Fixtures: `fixtures/valid/identity-trust.json`, `fixtures/valid/valid-with-identity.osdf`

### 2. Live latest-revision lookup (B.3 M2 — done)

The ledger store maintains `latestRevisions[]` (updated on `osdf ledger append`). Trust configs exported via `ledger attach-proof --trust-config` include the registry with `latestRevisionPolicy: optional`.

| Result | Verifier |
| --- | --- |
| Local = registry latest | `OSDF_LATEST_REVISION_CONFIRMED` (PASS) |
| Local < registry latest | `OSDF_LATEST_REVISION_OUTDATED` (WARNING) |
| No registry entry | `OSDF_LATEST_REVISION_REGISTRY_UNAVAILABLE` (INFO/WARNING) |

CLI: `osdf verify doc.osdf --ledger-config trust.json`  
Override policy: `--latest-revision-policy optional|required`

Fixtures: `fixtures/valid/valid-rev2-with-ledger-proof.osdf` (current), `valid-with-ledger-proof.osdf` (outdated rev1 against shared `ledger-trust.json`)

**Next (hosted ledger):** serve the ledger store over HTTP on a local machine so verifiers can fetch fresh `latestRevisions` without embedding a static snapshot — see `specs/phase-d-m2-hosted.md`.

### 3. Tree-head freshness + consistency (next)

- Freshness window on signed tree head timestamp
- Consistency proof vs previously observed checkpoint
- Optional live checkpoint confirmation

### 4. Revocation event stream

Append-only signed events:

```json
{
  "eventType": "KEY_REVOKED",
  "subjectKey": "urn:osdf:key:ed25519:...",
  "effectiveAt": "2026-06-04T18:00:00Z",
  "reasonCode": "DEVICE_COMPROMISED",
  "issuerKey": "urn:osdf:key:ed25519:...",
  "signature": "..."
}
```

| Scenario | Result |
| --- | --- |
| Signed before revocation | May remain historically valid |
| Signed after revocation | Fail |
| Revocation not checked | Unresolved warning |

### 5. Online enhanced verification mode

Switch `verificationMode` to `onlineEnhanced` when live checks run:

```
Verification mode: Online enhanced verification
✓ Embedded ledger inclusion proof valid
✓ Live checkpoint verified
✓ Latest revision confirmed
✓ Revocation status checked
```

### 6. Origin provenance (after identity)

Organization-attested vs network-observed origin with visible evidence grade.

## Trust summary wording (UI)

Do **not** label documents simply “trusted.” Prefer:

> ✓ File integrity verified  
> This document passed structural, cryptographic-signature, and embedded-ledger-proof checks.  
> ⓘ Signer identity has not been resolved.  
> ⓘ A live latest-revision check has not been performed.  
> ⓘ Revocation status has not been checked.

## Consumer vs developer views

**Default:** pass/fail summary + unresolved caveats  
**Expanded:** ledger entry id, log key URN, document id, tree-head timestamp, export JSON
