# Transformation receipt

**Status:** Draft. Not verified by the public alpha core.

## Purpose

A **transformation receipt** proves that a target revision (or external artifact commitment) was produced from a **named source** by a **named operation** at a **named time**, signed by an **authorized operator key**.

This is distinct from:

- **Revision commit** - advances the signed chain inside one document lineage
- **Transmission events** - mail/upload/send/receive timeline (see `specs/demo-package.md`)

## Object placement

```
transformations/transform-{uuid}.json
```

Declared in `manifest.objects[]` like any other package object.

## Canonical fields


| Field                    | Type    | Required | Description                                              |
| ------------------------ | ------- | -------- | -------------------------------------------------------- |
| `format`                 | string  | yes      | `"OSDF-Transformation"`                                  |
| `formatVersion`          | string  | yes      | `"1.0-draft"`                                            |
| `transformationId`       | string  | yes      | URN, e.g. `urn:osdf:transform:…`                         |
| `documentId`             | string  | yes      | Target document URN                                      |
| `targetRevision`         | integer | yes      | Revision after transform                                 |
| `sourceDocumentId`       | string  | yes      | Source document URN                                      |
| `sourceRevision`         | integer | yes      | Source revision number                                   |
| `sourceRevisionRootHash` | string  | yes      | `sha256:…` Merkle root at source                         |
| `sourcePublicCommitment` | string  | yes      | `sha256:…` commitment at source                          |
| `operation`              | string  | yes      | Enum (see below)                                         |
| `operationParams`        | object  | no       | Canonical JSON parameters                                |
| `operatorKey`            | string  | yes      | `urn:osdf:key:ed25519:…`                                 |
| `operatedAt`             | string  | yes      | RFC 3339 timestamp                                       |
| `targetManifestDigest`   | string  | no       | Expected manifest digest post-transform                  |
| `signature`              | string  | yes      | Ed25519 over canonical receipt (JCS); empty when signing |


## Operation enum (v1)

- `submit`: taxpayer/agency submission (e.g. blank form → filled form)
- `redact`: remove or mask fields under policy
- `extract`: derive subset object(s)
- `merge`: combine sources
- `export`: produce external representation (PDF/HTML)
- `render`: produce profile-bound presentation digest

## Verification rules (planned)

1. Parse receipt; verify Ed25519 signature over canonical form (signature field cleared).
2. Confirm source revision exists in chain and commitments match recorded hashes.
3. Confirm target revision manifest is consistent with `targetRevision` and operation semantics.
4. For `render` operations, optional link to `renderDigest` object (see reproducible rendering hash).

## Example

See [schemas/transformation-receipt.example.json](schemas/transformation-receipt.example.json).

## JSON Schema

See [schemas/transformation-receipt.schema.json](schemas/transformation-receipt.schema.json).