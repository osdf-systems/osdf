# OSDF

Open Secure Document Format implementation.

**Repository:** https://github.com/osdf-systems/osdf  
**Organization:** [OSDF Systems](https://github.com/osdf-systems)

<p align="center">
  <img src="docs/assets/demo-verify-pass.svg" alt="OSDF verification report — PASS with container, manifest, revision, and signature checks" width="720"/>
  <br><em>Verification report showing cryptographic chain-of-custody</em>
</p>

<p align="center">
  <img src="docs/assets/demo-verify-fail.svg" alt="OSDF verification report — FAIL after 1-byte tamper" width="720"/>
  <br><em>Detecting a 1-byte tamper in &lt;10ms</em>
</p>

Run the live demo (timings from your machine):

```powershell
cargo build --release -p osdf-cli
.\target\release\osdf.exe demo safety
```

Regenerate README images after verifier changes:

```powershell
.\target\release\osdf.exe demo safety --write-readme-assets docs/assets
```

---

## Quick start

```powershell
git clone https://github.com/osdf-systems/osdf.git
cd osdf
cargo build --release -p osdf-cli
.\target\release\osdf.exe verify fixtures\valid\valid-committed.osdf
.\target\release\osdf.exe version
```

Install from source (until first GitHub Release):

```powershell
cargo install --path crates/osdf-cli --locked
osdf verify Taxes.osdf
```

Release channels: stable tags (`v0.1.0`), prereleases (`v0.1.0-alpha.1`), and opt-in nightly CI artifacts. See [CHANGELOG.md](CHANGELOG.md).

## Phases

| Phase | Status | Deliverable |
| --- | --- | --- |
| **A** | Done | Rust core, CLI, fixtures |
| **B** | Done | WASM verifier + browser UI |
| C | PoC | Transparent Gateway + tax form demo |
| **D (M1)** | Done | Transparency log proofs + verifier |
| **B.3** | In progress | Latest-revision registry (M2 done); freshness/revocation next |
| **Demo package** | **Active (3 mo)** | Supplemental Plan §23 — scripted gateway enforcement demo |

**Current focus:** Build the [Demonstration Package](specs/demo-package.md) (§23) — prove un-bypassable gateway logic before sales. Run `.\scripts\run-demo-package.ps1` for today's partial walkthrough.

### Phase A — Core format + CLI

- Strict ZIP container parsing (duplicate paths, traversal rejection, compression limits)
- JCS canonicalization and SHA-256 object digests
- Signed Merkle manifest and revision chain
- Ed25519 scoped signatures (community baseline)
- CLI: `verify`, `inspect`, `create`, `commit-revision`, `demo safety`

### Phase B — Browser verifier (WASM)

- `crates/osdf-wasm` — read-only WASM bindings (`verify_osdf`, `version`)
- `web/` — drag-and-drop static UI (local-only, no upload)
- Structured `VerificationReport` shared by CLI and browser

See `specs/phase-a.md`, `specs/phase-b.md`, `specs/phase-b3.md`, `specs/phase-d.md`, and `docs/web-verifier.md`.

### Documentation

Implementer specs live in **`specs/`**; verifier notes in **`docs/`**. The HTML marketing site and docs site are maintained outside this repository.

## Build

```bash
cargo build --release
cargo test
```

## Browser verifier

OSDF includes a static browser verifier compiled from the same Rust verification library used by the CLI.

The verifier:

- runs locally in the browser;
- does not upload the selected file;
- validates the passive-content policy;
- validates the package manifest;
- rejects undeclared objects;
- verifies the revision chain;
- verifies cryptographic signatures.

### Build

Install once:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

```powershell
.\scripts\build-wasm.ps1
```

```bash
./scripts/build-wasm.sh
```

### Serve locally

```powershell
.\scripts\serve-web.ps1
```

Open `http://localhost:8080/`.

### Current limitations

The browser verifier currently performs local structural and cryptographic verification only.

It does not yet:

- resolve organizational credentials;
- check key revocation;
- verify trusted timestamps;
- import or render PDF files;
- create, revise, or sign OSDF files.

Transparency ledger verification is available when you supply a local `LedgerConfig` (CLI `--ledger-config` or browser textarea).

## Transparent Gateway (PoC)

View verified tax documents in a browser frame after demo MFA — verifies locally, then renders `content/document.json` as a readable form.

```powershell
.\scripts\build-wasm.ps1
.\scripts\serve-demo.ps1
```

Open http://localhost:8081/gateway/ — demo MFA code **847291**.

| Fixture | Revision | Description |
| --- | --- | --- |
| `fixtures/valid/taxes-template.osdf` | 1 | Blank simplified tax form |
| `fixtures/valid/Taxes.osdf` | 2 | Same form with demo filler data |

See `specs/phase-c-gateway.md`.

## CLI usage

### Install `osdf` on your PATH (Windows)

One-time setup installs the CLI to `%LOCALAPPDATA%\Programs\osdf\bin` and adds it to your **user** PATH:

```powershell
.\scripts\install-cli.ps1
```

After that, any **release** build of the CLI auto-refreshes the installed binary:

```powershell
cargo build --release -p osdf-cli   # copies to PATH automatically
osdf --version
```

Open a new terminal after the first install so PATH changes apply everywhere. To disable auto-install, unset `OSDF_AUTO_INSTALL` in `.cargo/config.toml`.

From the repo you can also run `cargo osdf -- …` without installing (uses `cargo run` under the hood).

```bash
# Create a draft package (revision 0)
osdf create output/example.osdf --title "Hello OSDF"

# Create and commit revision 1 with signature
osdf create output/signed.osdf --title "Signed" --commit

# Verify a package
osdf verify output/signed.osdf

# Inspect metadata
osdf inspect output/signed.osdf --json

# Commit a new revision
osdf commit-revision output/signed.osdf --output output/signed-rev2.osdf

# Transparency ledger (Phase D M1)
osdf ledger init --store ledger.json --key ledger-key.json
osdf ledger append --store ledger.json --package output/signed.osdf
osdf ledger attach-proof --store ledger.json --key ledger-key.json \
  --package output/signed.osdf --output output/with-proof.osdf --trust-config trust.json
osdf verify output/with-proof.osdf --ledger-config trust.json

# Verify current vs outdated revision (registry in trust.json)
osdf verify fixtures/valid/valid-rev2-with-ledger-proof.osdf --ledger-config fixtures/valid/ledger-trust.json
osdf verify fixtures/valid/valid-with-ledger-proof.osdf --ledger-config fixtures/valid/ledger-trust.json  # WARNING: outdated
```

## Layout

```text
osdf/
├── crates/
│   ├── osdf-core/     # parser, crypto, builder, verifier
│   ├── osdf-cli/      # command-line tool
│   └── osdf-wasm/     # browser WASM bindings (verify-only)
├── web/               # static verifier UI (+ generated pkg/)
├── gateway/           # Transparent Gateway viewer (MFA + tax form render)
├── fixtures/          # valid and invalid test packages
├── specs/             # phase specs and demo package plan
├── scripts/
│   ├── build-wasm.ps1
│   ├── build-wasm.sh
│   ├── install-cli.ps1
│   ├── serve-demo.ps1
│   └── serve-web.ps1
└── docs/
    └── web-verifier.md
```

## Profile

This build implements **OSDF-Core** with inline payload mode only. Encrypted packages, transparency ledger queries, and editors are deferred to later phases.

## Regenerate fixtures

```bash
cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored
```
