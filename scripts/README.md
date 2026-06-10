# Scripts

Cross-platform helpers for local development and demos. See [docs/getting-started.md](../docs/getting-started.md) for full platform walkthroughs.

| Script | macOS / Linux | Windows |
| --- | --- | --- |
| Build WASM | `build-wasm.sh` | `build-wasm.ps1` |
| Serve browser verifier | `serve-web.sh` | `serve-web.ps1` |
| Serve gateway + verifier | `serve-demo.sh` | `serve-demo.ps1` |
| Demo package tour | `run-demo-package.sh` | `run-demo-package.ps1` |
| Benchmarks (Hyperfine + Criterion) | `run-benchmarks.sh` | `run-benchmarks.ps1` |
| Fetch OpenTDF golden TDF fixtures | `fetch-opentdf-fixtures.sh` | `fetch-opentdf-fixtures.ps1` |
| Install CLI to PATH | `install-cli.sh` | `install-cli.ps1` |

On macOS/Linux, mark scripts executable once:

```bash
chmod +x scripts/*.sh
```
