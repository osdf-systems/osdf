# Browser verifier

Static drag-and-drop verifier for `.osdf` packages.

## Build WASM

From the repository root:

```powershell
.\scripts\build-wasm.ps1
```

```bash
./scripts/build-wasm.sh
```

This writes generated bindings to `web/pkg/` (not committed).

## Serve locally

Browsers require HTTP for ES module and WASM loading:

```powershell
.\scripts\serve-web.ps1
```

```bash
python -m http.server 8080 --directory web
```

Open `http://localhost:8080/`.

## Architecture

```
Browser File API → Uint8Array → osdf-wasm → osdf-core → VerificationReport → HTML panel
```

The WASM layer exposes only `verify_osdf` and `version`. No creation, signing, or mutation APIs are compiled into the browser build.
