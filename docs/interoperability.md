# Interoperability and cryptographic agility

**Status:** Design document for public alpha. Mappings describe intent; adapters are not implemented unless listed in [CHANGELOG.md](../CHANGELOG.md).

OSDF-Core is a **durable signed container**. Real-time authorization capsules are planned as a **sibling profile**, not a replacement for the portable ZIP package.

---

## Standards mapping (planned)

### SCITT (Supply Chain Integrity, Transparency and Trust)


| OSDF today                             | SCITT concept                      |
| -------------------------------------- | ---------------------------------- |
| Append-only ledger store               | Transparent log / registry         |
| `SignedTreeHead`                       | Signed checkpoint                  |
| Merkle inclusion proof                 | Inclusion proof                    |
| `revision_event_hash` as log leaf      | Statement digest / entry payload   |
| Trusted log registry in `LedgerConfig` | Trust anchor for issuer / log keys |


**Direction:** Document equivalence in verification reports; optional SCITT API export for log submission in Phase II. Single-log correctness remains the gate before federation.

### C2PA (Content Credentials)


| OSDF                                  | C2PA                                |
| ------------------------------------- | ----------------------------------- |
| Revision chain + signatures           | Manifest + claim signatures         |
| Transformation receipts (planned)     | Ingredient / actions                |
| Reproducible rendering hash (planned) | Soft binding / rendering assertions |


**Direction:** Native OSDF provenance remains authoritative inside the container; C2PA import/export adapters for media workflows are a Phase II integration layer - not a fork of the core format.

### W3C Verifiable Credentials


| OSDF                                         | VC ecosystem                           |
| -------------------------------------------- | -------------------------------------- |
| Organizational identity + delegation (today) | Issuer / subject / verification method |
| OSDF-ZT-Token profile (planned)              | Short-lived authorization object       |


**Direction:** VC data model informs token field naming; OSDF packages remain the archive-grade artifact; VCs authorize **requests against** policy bound to that artifact.

---

## Cryptographic agility

### Today

- Signatures: **Ed25519** (`algorithm: "Ed25519"`, URL-safe base64 payload)
- Digests: **SHA-256** with domain-separated prefixes (`OSDF-OBJECT-v1`, etc.)
- Envelope and signature objects carry `signatureVersion` / format version strings

### Migration rules (design)

1. **Verify rejects unknown algorithms** - fail closed, no silent downgrade.
2. **Hybrid period:** packages may carry **parallel** signature blocks (classic + PQC) during migration; verifiers require at least one trusted suite per policy epoch.
3. **Archive policy:** long-retention deployments pin allowed algorithm sets in trust snapshots (see offline verification bundle).
4. **PQC:** ML-DSA / SLH-DSA evaluation for Phase II; not in hot-path alpha.

### Post-quantum (design now, implement in Version 1.5.0)

Long-retention archives need a **documented migration story** before algorithm rollout:

- Trust snapshot lists `acceptedSignatureAlgorithms`
- New revisions may dual-sign; old revisions remain verifiable with pinned policy
- Ledger tree heads and transformation receipts include `algorithmId` for leaf/event hashing when hash agility is required

---

## Profile summary


| Profile          | Role                            | Interop                        |
| ---------------- | ------------------------------- | ------------------------------ |
| OSDF-Core-JSON   | Portable baseline (today)       | ZIP + JSON; human inspection   |
| OSDF-Core-Binary | Optimized encoding (planned)    | Canonical CBOR or fixed layout |
| OSDF-ZT-Token    | High-QPS auth capsule (planned) | VC-aligned fields; no ZIP      |


Do not benchmark or market one profile's throughput as another's guarantee.