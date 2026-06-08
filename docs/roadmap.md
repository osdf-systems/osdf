# Public roadmap

This document describes planned work for the **public source-available alpha** of OSDF-Core. Items marked *planned* or *design* are not shipped unless noted in [CHANGELOG.md](../CHANGELOG.md).

Verification profiles (portable full, portable fast, parsed revalidation) are documented in code via `VerificationProfile` and benchmarked separately with `scale_bench` - do not mix throughput numbers across profiles.

---

## Shipped in current alpha


| Capability                             | Notes                                                                 |
| -------------------------------------- | --------------------------------------------------------------------- |
| ZIP-backed OSDF-Core JSON packages     | Fail-closed container walk, manifest, revision, signatures            |
| Full forensic verification report      | CLI, library, WASM                                                    |
| Fast verify API                        | Same fail-closed crypto path; compact `FastVerifyResult` for gateways |
| Parsed-container fast revalidation     | `parse_package` once; hot-path policy re-checks                       |
| Embedded transparency-log proofs       | Offline CT-style inclusion verification                               |
| Organizational identity (configured)   | Local trust registry + delegation credentials                         |
| Latest-revision registry (file-backed) | Outdated-revision warnings                                            |
| Side-channel hardening (audit path)    | Constant-time digests; full-scan manifest audit                       |


See [SECURITY.md](../SECURITY.md) for the timing threat model.

---

## Build next

These three features extend provenance, presentation integrity, and offline trust without replacing the portable package baseline.

### 1. Verified transformation receipts

**Goal:** Prove how one document or evidence copy was **derived** from another (redact, extract, merge, export, render, submit), not merely that it is a new revision.

**Status:** Schema draft - [specs/transformation-receipt.md](../specs/transformation-receipt.md)

**Relationship to today:** Revision commit proves *what changed* in the chain; transformation receipts prove *why and how* a derived artifact exists relative to a named source commitment.

### 2. Reproducible rendering hash

**Goal:** Detect cases where identical signed bytes **present differently** in approved viewers (layout profile mismatch, not pixel-perfect PDF across all engines in v1).

**Status:** Planned - initial profile: gateway `taxForm` JSON (see [specs/phase-c-gateway.md](../specs/phase-c-gateway.md))

**v1 scope:** Profile-bound digest over canonical render inputs used by the reference renderer; mismatch code `OSDF_RENDER_DIGEST_MISMATCH`.

### 3. Offline verification bundle

**Goal:** Preserve trust during outages, litigation, archives, and degraded operations - one artifact auditors can re-verify without network access.

**Status:** Schema draft - [specs/offline-verification-bundle.md](../specs/offline-verification-bundle.md)

**Relationship to today:** Embedded proofs and trust JSON exist inside packages; the bundle **packages the package + pinned trust snapshot + full report + verifier metadata** for export.

---

## Design now

Document architecture and standards mapping before large implementations.


| Topic                                            | Document                                                              |
| ------------------------------------------------ | --------------------------------------------------------------------- |
| SCITT, C2PA, W3C VC interoperability             | [docs/interoperability.md](interoperability.md)                       |
| Cryptographic agility and post-quantum migration | [docs/interoperability.md](interoperability.md#cryptographic-agility) |


**Principle:** OSDF-Core remains the **durable portable capsule**; high-QPS authorization objects are a **sibling profile** (OSDF-ZT-Token), not a ZIP shortcut.

---

## Later

| Feature | Purpose |
| --- | --- |
| Cross-ledger witness gossip | Detect a log showing conflicting histories to different recipients |
| Forward-secure collaboration groups | Secure shared working groups after membership or endpoint compromise |

---

## Verification profile ladder (performance)

Benchmark each profile separately (`scale_bench --profile …`):


| Profile  | Use case                                     |
| -------- | -------------------------------------------- |
| `full`   | Portable ingest + forensic report (baseline) |
| `fast`   | Gateway allow/deny on newly received bytes   |
| `parsed` | Repeated checks on an already-parsed capsule |


Future: `OSDF-Core-Binary` (encoding optimization), `OSDF-ZT-Token` (authorization capsule) - see verification profile ladder in code and `scale_bench`.

---

## Out of scope for public alpha

- Production hosted ledger or live revocation service
- Claiming sub-millisecond verify implies timing-attack immunity (see SECURITY.md)

The hosted ledger service idea is still exploratory and will be re-evaluated at the first major release (v1.0.0).

