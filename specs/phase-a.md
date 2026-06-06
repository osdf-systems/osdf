# Phase A specification notes

Provisional implementation profile for OSDF v0.3 architecture Phase A.

> **Public docs:** this spec and `docs/` (HTML docs site is maintained outside this repo).

## Implemented

- Strict ZIP container parsing (duplicate paths, traversal, compression limits)
- Magic header `OSDF\0\x01` in `osdf-header.bin`
- `public-envelope.json` with inline payload mode
- `manifests/package-manifest.json` with declared object digests
- JCS (RFC 8785) canonicalization for JSON object digests
- SHA-256 domain-separated digests and Merkle root
- Append-only revision chain with salted public commitments
- Ed25519 scoped signatures (community baseline)
- CLI: `verify`, `inspect`, `create`, `commit-revision`

## Phase A simplifications

These differ from the full v0.3 draft and should be revised before stable release:

1. **`manifestDigest`**: Manifest integrity uses a top-level `manifestDigest` field computed over the manifest JSON with `manifestDigest` excluded (avoids impossible self-referential fixed points).

2. **Merkle scope**: `revisionRootHash` covers `content/**` and `osdf-header.bin` only. Metadata files (revisions, signatures, envelope) are integrity-checked via `objects[]` digests but excluded from the content Merkle tree.

3. **Envelope exclusion**: `public-envelope.json` is required in the package but not listed in `manifest.objects[]`. Declared archive size lives in the fixed 15-byte `osdf-header.bin` (`packageBytes` as u64 BE), not in JSON, to avoid size feedback loops.

4. **No encryption**: `payloadMode: encrypted` is rejected.

5. **No ledger**: Freshness / transparency proofs deferred to Phase C.

## Container layout (OSDF-Core)

```text
example.osdf
|-- osdf-header.bin
|-- public-envelope.json
|-- manifests/package-manifest.json
|-- content/document.json
|-- revisions/rev-000001.json          (when committed)
`-- signatures/revision-000001.sig.json (when committed)
```

## Conformance target

Phase A targets **Reader L1**, **Verifier L2**, and **Editor E1** subsets for local-only workflows.
