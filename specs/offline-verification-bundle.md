# Offline verification bundle

**Status:** Draft. Export tooling not shipped in public alpha.

## Purpose

An **offline verification bundle** preserves everything needed to **re-run full portable verification** without network access: the package bytes, pinned trust material, the verification report produced at export time, and verifier metadata.

Use cases: litigation hold, archive handoff, degraded shipboard ops, auditor walkthrough, air-gapped lab.

## Layout

```
{bundleName}/
  manifest.json              # bundle manifest (this spec)
  package.osdf               # exact bytes verified (or symlink policy documented)
  verification-report.json   # full VerificationReport from portable full verify
  trust-snapshot.json        # LedgerConfig + IdentityConfig (+ latest-revision registry) at export time
  README.txt                 # human instructions for re-verification
```

Optional future members:

- `timeline.json`: transmission / transformation events
- `policy-snapshot.json`: policy version bound at verify time
- `render-evidence.json`: profile + render digest evidence

## Bundle manifest (`manifest.json`)

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `format` | string | yes | `"OSDF-Offline-Verification-Bundle"` |
| `formatVersion` | string | yes | `"1.0-draft"` |
| `exportedAt` | string | yes | RFC 3339 |
| `exporterVersion` | string | yes | `osdf-core` semver |
| `verificationProfile` | string | yes | e.g. `OSDF-Core-JSON portable (full report)` |
| `packageSha256` | string | yes | Digest of `package.osdf` |
| `packageFileName` | string | yes | `"package.osdf"` |
| `reportFileName` | string | yes | `"verification-report.json"` |
| `trustSnapshotFileName` | string | yes | `"trust-snapshot.json"` |
| `overallResult` | string | yes | `PASS` \| `FAIL` \| `WARNING` |

## Re-verification procedure (planned)

1. Verify `packageSha256` matches `package.osdf`.
2. Load `trust-snapshot.json` into `VerifierConfig`.
3. Run `verify_package_bytes_with_config` on package bytes.
4. Compare new report to exported report (or assert `overallResult` unchanged for PASS bundles).

**Note:** Freshness, live revocation, and live latest-revision checks remain explicitly offline unless the trust snapshot includes frozen registry state and policy documents from export time.

## Relationship to embedded proofs

Packages may already contain ledger inclusion proofs. The bundle adds **pinned trust anchors** and a **point-in-time forensic report** so third parties need not guess which trust JSON was used.

## JSON Schema

See [schemas/offline-verification-bundle.schema.json](schemas/offline-verification-bundle.schema.json).

Example: [schemas/offline-verification-bundle.example.json](schemas/offline-verification-bundle.example.json).
