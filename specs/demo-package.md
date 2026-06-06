# Demonstration Package — 3-Month Build Plan

**North star:** Prove the logic in Supplemental Plan §23.1 — not sales slides. A working, scripted **un-bypassable gateway** demo is the magnet: verify structure, origin, ledger, freshness; quarantine tampering; show timeline and forensic output.

**Reference docs:** Supplemental Architecture Plan v0.4 §23; Threat Vector Plan v0.1 §12–14.

**Status:** Living plan — update the checklist as milestones land.

---

## The 10-step demo (Section 23.1)

| # | Demo beat | What the audience must see | Target month | Status |
| --- | --- | --- | --- | --- |
| 1 | Create contract revision 1 | Agency issues blank contract `.osdf` | M1 | Partial — use `taxes-template.osdf` or generate `contract-rev1.osdf` |
| 2 | Sign revision 1 | Ed25519 signature + revision chain PASS | M1 | Done |
| 3 | Submit to trusted log | Ledger append + inclusion proof in package | M1 | Done — `ledger append` / `attach-proof` |
| 4 | Gateway **send** event | Signed `DOCUMENT_TRANSMITTED` receipt (outbound mail/upload) | M2 | Not started |
| 5 | Gateway **receipt** event | Signed `DOCUMENT_RECEIVED` receipt (inbound) | M2 | Not started |
| 6 | Create revision 2 | Counterparty returns signed revision 2 | M1 | Partial — `Taxes.osdf` / `commit-revision` |
| 7 | Tamper with a fixture | Deliberately broken package (undeclared object, bad hash) | M1 | Done — `fixtures/invalid/*` |
| 8 | **Quarantine** tampered file | Gateway verdict `QUARANTINE` / `REJECT`, not silent delivery | M2 | Not started |
| 9 | Timeline + forensic report | Ordered events + exportable JSON report for auditors | M2–M3 | Partial — verifier export; no timeline UI |

---

## What “un-bypassable gateway” means for the PoC

The gateway is **not** a viewer with a login screen. It is a **headless policy engine** that runs the same checks as the verifier and returns a deterministic verdict **before** delivery:

```
Incoming .osdf
  → container safe?
  → manifest integrity?
  → signatures + identity (if policy requires)?
  → ledger proof + trusted log?
  → latest revision current (live or registry)?
  → revocation clear (when implemented)?
  → VERDICT: ALLOW | WARN | QUARANTINE | REJECT
  → SIEM / timeline event
```

The browser **Transparent Gateway** (`gateway/`) proves viewer UX. The **demonstration package** adds the enforcement path **email/upload ingress cannot bypass verification**.

---

## Three-month roadmap

### Month 1 — “The file tells the truth” (foundation)

**Goal:** Script steps 1–3, 6–7 end-to-end from CLI; rev1 outdated / rev2 current in verifier.

| Week | Deliverable |
| --- | --- |
| 1 | `fixtures/demo/contract-rev1.osdf`, `contract-rev2.osdf` with ledger proofs + `demo-ledger-trust.json` |
| 2 | `scripts/run-demo-package.ps1` — one command, narrated PASS/WARN/FAIL |
| 3 | Align tax gateway fixtures with ledger trust (optional parallel story) |
| 4 | Docs: demo walkthrough in `specs/demo-package.md`; update status table |

**Exit criteria:** Run script in &lt;2 minutes; show rev1 **WARNING** (outdated), rev2 **PASS**, tampered **FAIL**.

### Month 2 — “The gateway enforces policy” (the magnet)

**Goal:** Steps 4–5, 8–9 — headless inspect + transmission events + quarantine.

| Week | Deliverable |
| --- | --- |
| 5 | `osdf gateway inspect` — JSON verdict from `VerificationReport` + policy file |
| 6 | `POST /v1/inspect` (local HTTP, `osdf gateway serve`) |
| 7 | Event model: `events/transmission.schema.json`; sign send/receipt demo events |
| 8 | Timeline viewer page: contract lifecycle (create → send → receive → rev2 → quarantine attempt) |

**Policy example** (Supplemental §6.3):

```yaml
name: government-inbound-osdf
require:
  container_safe: true
  manifest_integrity: true
  signature_valid: true
  ledger_inclusion_proof_valid: true
  latest_revision_current: true
on_failure:
  action: quarantine
```

**Exit criteria:** Tampered attachment → `QUARANTINE` via HTTP API; timeline shows 5+ signed events.

### Month 3 — “Live freshness + forensic close” (credibility)

**Goal:** Step 10 stub + live ledger; polish for pilot recordings.

| Week | Deliverable |
| --- | --- |
| 9 | `osdf ledger serve` — live latest-revision lookup (`specs/phase-d-m2-hosted.md`) |
| 10 | Verifier/gateway `onlineEnhanced` mode when registry URL configured |
| 11 | Forensic export bundle: verification report + timeline + policy snapshot |

**Exit criteria:** Replay demo with **live** registry (no stale trust JSON); export one ZIP auditors can open offline.

---

## Demo assets (current repo)

| Asset | Role in demo |
| --- | --- |
| `fixtures/valid/valid-with-ledger-proof.osdf` | Rev 1 + ledger proof → **outdated** with `ledger-trust.json` |
| `fixtures/valid/valid-rev2-with-ledger-proof.osdf` | Rev 2 + proof → **current** |
| `fixtures/valid/ledger-trust.json` | Latest-revision registry snapshot |
| `fixtures/valid/identity-trust.json` | Colorado org delegation demo |
| `fixtures/valid/taxes-template.osdf` / `Taxes.osdf` | Gateway viewer story (rev 1 blank / rev 2 filled) |
| `fixtures/invalid/tampered-content-hash.osdf` | Tamper → FAIL |
| `fixtures/invalid/undeclared-object.osdf` | Gateway quarantine candidate |
| `web/` | Browser verifier + report export |
| `gateway/` | MFA + tax form viewer (not enforcement yet) |

---

## Run today’s partial demo

```powershell
cd osdf
cargo build --release -p osdf-cli
.\scripts\run-demo-package.ps1
```

This script narrates what works now and prints the gaps for Month 2.

For the browser:

```powershell
.\scripts\build-wasm.ps1
.\scripts\serve-demo.ps1
```

- Verifier: http://localhost:8081/web/ — paste `ledger-trust.json`, drop rev1 then rev2 packages.
- Gateway: http://localhost:8081/gateway/ — MFA `847291`, load `Taxes.osdf`.

---

## Event codes the demo must surface

From Threat Plan Appendix A — minimum set for §23:

| Code | Demo moment |
| --- | --- |
| `OSDF_LATEST_REVISION_OUTDATED` | Old contract rev1 still “valid” but superseded |
| `OSDF_LATEST_REVISION_CONFIRMED` | Current rev2 |
| `OSDF_MANIFEST_UNDECLARED_OBJECT` | Quarantine tampered package |
| `OSDF_LEDGER_INCLUSION_PROOF_VALID` | Trusted log receipt |
| `OSDF_SIGNER_IDENTITY_RESOLVED` | Colorado delegation (optional beat) |
| `OSDF_LIVE_LATEST_REVISION_NOT_CHECKED` | Offline mode honesty (until M3 live serve) |

---

## Explicit non-goals (3 months)

Do **not** build yet:

- Sales site / pricing experiments
- Production SMTP transport agents (M365 milter, etc.)
- Multi-ledger federation, TUF trust updates, witness gossip


---

## Success = one recorded walkthrough

Record a single 8–12 minute session:

1. Issue contract rev1, log it, “send” via gateway (signed event).
2. Recipient verifies — PASS, issuer resolved, ledger OK.
3. Return rev2 — PASS, latest revision confirmed.
4. Attacker replays rev1 — WARN outdated (authentic but stale).
5. Attacker submits tampered file — QUARANTINE, never delivered.
6. Export timeline + forensic JSON.

If that recording is believable to a security engineer, the demonstration package succeeded.

---

## File map (to create)

| Path | Purpose |
| --- | --- |
| `specs/demo-package.md` | This plan |
| `specs/demo-events.md` | Transmission + timeline event schemas (M2) |
| `specs/demo-policy.example.yaml` | Gateway policy profile (M2) |
| `crates/osdf-gateway/` or `osdf-cli gateway` | Headless inspect + HTTP (M2) |
| `demo/timeline/` | Static timeline UI (M2) |
| `fixtures/demo/` | Contract packages + signed events (M1) |
| `scripts/run-demo-package.ps1` | Automated narrated demo (M1) |
