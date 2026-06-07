# Getting started by platform

OSDF runs on **Windows**, **macOS** (Intel and Apple Silicon), and **Linux**. The Rust core, CLI, and WASM verifier share the same codebase; helper scripts differ by platform.

| | Windows | macOS / Linux |
| --- | --- | --- |
| **Build & test** | `cargo build --release` | same |
| **Helper scripts** | `*.ps1` | `*.sh` |
| **CLI binary** | `target\release\osdf.exe` | `target/release/osdf` |
| **Install to PATH** | `scripts/install-cli.ps1` | `scripts/install-cli.sh` |

Jump to: [macOS](#macos) · [Linux](#linux) · [Windows](#windows)

---

## Requirements (all platforms)

- [Rust](https://rustup.rs/) **1.93+**
- **Python 3** (for local static servers)
- **Git**

For the browser verifier only:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

---

## macOS

Tested on **Apple Silicon (`aarch64-apple-darwin`)** and **Intel (`x86_64-apple-darwin`)**.

### Clone and verify

```bash
git clone https://github.com/osdf-systems/osdf.git
cd osdf
cargo build --release -p osdf-cli
./target/release/osdf verify fixtures/valid/valid-committed.osdf
./target/release/osdf demo safety
```

### Install CLI to PATH

```bash
chmod +x scripts/*.sh
./scripts/install-cli.sh
```

Installs to `~/.local/bin/osdf`. If `osdf` is not found, add to `~/.zshrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Alternative (works everywhere):

```bash
cargo install --path crates/osdf-cli --locked
```

### Browser verifier

```bash
./scripts/build-wasm.sh
./scripts/serve-web.sh
```

Open [http://localhost:8080/](http://localhost:8080/)

### Gateway demo (tax form PoC)

```bash
./scripts/build-wasm.sh
./scripts/serve-demo.sh
```

Open [http://localhost:8081/gateway/](http://localhost:8081/gateway/) · MFA code: `847291`

### Narrated demo walkthrough

```bash
./scripts/run-demo-package.sh
```

### Development

```bash
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Linux

Same commands as [macOS](#macos). Scripts use `python3` when available.

Install system packages if needed (Debian/Ubuntu example):

```bash
sudo apt update
sudo apt install build-essential pkg-config python3
```

Install CLI:

```bash
chmod +x scripts/*.sh
./scripts/install-cli.sh
```

Ensure `~/.local/bin` is on your `PATH` (many distros include it by default).

---

## Windows

### Clone and verify

```powershell
git clone https://github.com/osdf-systems/osdf.git
cd osdf
cargo build --release -p osdf-cli
.\target\release\osdf.exe verify fixtures\valid\valid-committed.osdf
.\target\release\osdf.exe demo safety
```

### Install CLI to PATH

```powershell
.\scripts\install-cli.ps1
```

Installs to `%LOCALAPPDATA%\Programs\osdf\bin`. Release builds can auto-refresh that copy when `OSDF_AUTO_INSTALL=1` in `.cargo/config.toml`.

Alternative:

```powershell
cargo install --path crates/osdf-cli --locked
```

### Browser verifier

```powershell
.\scripts\build-wasm.ps1
.\scripts\serve-web.ps1
```

Open [http://localhost:8080/](http://localhost:8080/)

### Gateway demo

```powershell
.\scripts\build-wasm.ps1
.\scripts\serve-demo.ps1
```

Open [http://localhost:8081/gateway/](http://localhost:8081/gateway/) · MFA code: `847291`

### Narrated demo walkthrough

```powershell
.\scripts\run-demo-package.ps1
```

### Development

```powershell
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Script reference

| Task | macOS / Linux | Windows |
| --- | --- | --- |
| Build WASM | `scripts/build-wasm.sh` | `scripts/build-wasm.ps1` |
| Serve verifier | `scripts/serve-web.sh` | `scripts/serve-web.ps1` |
| Serve gateway + verifier | `scripts/serve-demo.sh` | `scripts/serve-demo.ps1` |
| Demo package tour | `scripts/run-demo-package.sh` | `scripts/run-demo-package.ps1` |
| Install CLI | `scripts/install-cli.sh` | `scripts/install-cli.ps1` |

All scripts assume you run them from the repository root (they resolve paths relative to `scripts/`).

First time on macOS/Linux:

```bash
chmod +x scripts/*.sh
```

---

## Regenerate fixtures

```bash
cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored
```

---

## Troubleshooting

| Issue | Fix |
| --- | --- |
| `osdf: command not found` after install | Restart terminal; confirm `~/.local/bin` (Unix) or `%LOCALAPPDATA%\Programs\osdf\bin` (Windows) is on PATH |
| Port 8080 / 8081 in use | Stop the other `python -m http.server` process (Ctrl+C in that terminal) |
| WASM 404 in browser | Run `build-wasm` script first; confirm `web/pkg/osdf_wasm_bg.wasm` exists |
| Missing `.osdf` fixtures | Run fixture regeneration command above |

More: [web-verifier.md](web-verifier.md) · [CONTRIBUTING.md](../CONTRIBUTING.md)
