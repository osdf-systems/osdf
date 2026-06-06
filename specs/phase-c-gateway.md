# Phase C — Transparent Gateway (PoC)

A **viewing gateway** that verifies an OSDF package locally, then renders human-readable content inside a web frame after MFA.

## Demo flow

1. User completes MFA (PoC demo code: `847291`).
2. Gateway loads `Taxes.osdf` or `taxes-template.osdf` from fixtures (or user upload).
3. WASM verifies structure + signatures (+ optional trust JSON).
4. Gateway extracts `content/document.json` and renders a **tax form** layout.

## Tax form fixtures

| File | Revision | Content |
| --- | --- | --- |
| `fixtures/valid/taxes-template.osdf` | 1 | Blank simplified 1040-style form |
| `fixtures/valid/Taxes.osdf` | 2 | Same form with demo filler taxpayer data |

Content schema: `fixtures/content/taxes-template.json` → `type: "taxForm"`.

Revision 1 is the agency-issued blank copy; revision 2 is the taxpayer submission (demo filler only — not real PII).

## Run locally

```powershell
.\scripts\build-wasm.ps1
.\scripts\serve-demo.ps1
```

Open http://localhost:8081/gateway/

## Not production

- MFA is a static demo code stored in `sessionStorage`.
- No server-side session, OIDC, or WebAuthn yet.
- Gateway renders `taxForm` JSON only — not PDF import.

## Next steps

- Real IdP / WebAuthn MFA
- Hosted ledger fetch after verify
- Editable submission → `commit-revision` back to OSDF
- PDF/HTML rendering profiles beyond `taxForm`
