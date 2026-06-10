# OpenTDF benchmark fixtures

Third-party **Trusted Data Format (TDF)** samples for comparison benchmarks documented in [docs/benchmarks.md](../../docs/benchmarks.md).

These files are **not OSDF packages**. They exist so you can compare wall-clock or structural parsing against OpenTDF golden vectors without running a full OpenTDF platform locally.

## Files in this directory

| File | Source | Size | Notes |
| --- | --- | --- | --- |
| `small-java-4.3.0-e0f8caf.tdf` | [opentdf/tests xtest/golden](https://github.com/opentdf/tests/tree/main/xtest/golden) | ~7 KiB | ZIP TDF: `0.manifest.json` + `0.payload` |
| `spec-nosign.ntdf` | [opentdf/spec nanotdf "No Signature Example"](https://github.com/opentdf/spec) | 197 B | NanoTDF test vector (spec base64) |

## Optional large fixture (not committed)

`big-java-4.3.0-e0f8caf.tdf` (~10 MiB) is available from the same golden folder. Download locally:

```bash
./scripts/fetch-opentdf-fixtures.sh
```

```powershell
.\scripts\fetch-opentdf-fixtures.ps1
```

## Decrypt / full OpenTDF verify

Golden ZIP TDFs are **encrypted**. Decrypting them requires a running [OpenTDF platform](https://github.com/opentdf/platform) and matching keys from the xtest harness. For OSDF benchmark docs we use:

- **Structural parse:** list ZIP entries, read `0.manifest.json` (no KAS)
- **Decrypt benchmark:** only after you provision platform + `otdfctl` (see OpenTDF quickstart)

## Regenerate `spec-nosign.ntdf`

Base64 from the spec (whitespace stripped):

```
TDFMAQ9rYXMuZXhhbXBsZS5jb22ANQABHWthcy5leGFtcGxlLmNvbS9wb2xpY3kvYWJjZGVmYaoGjXbC
DfOlY3YzmGKfUjBy0IbUTUvmbiV04TvDLMcCKkzceqfvy6YDwZg/h3LvHRDoLg1ABvS93ZJ4eTVmcwPo
sz9EmnOSdxPUpKK05elFLi8FNDOdNZEb36Fe4Ys62wAAK1DknPqraRhSJhstY2CDGsvV8gP77xf5Rr7+
x57lEZugkjM7LA7qy54vjcg=
```

## License

OpenTDF golden files are maintained by the OpenTDF project under their repository license. Do not redistribute modified TDF binaries as OSDF artifacts.
