# Contributing to OSDF

Thank you for helping build the Open Secure Document Format.

## Repository

https://github.com/osdf-systems/osdf

## Branch model

- `main`: always compiles, tests pass, source of release tags
- `feat/*`, `fix/*`, `docs/*`, `chore/*`: short-lived branches merged via pull request

Do not commit directly to `main` once branch protection is enabled. Setup guide: [docs/github-setup.md](docs/github-setup.md).

## Development setup

Platform-specific commands: [docs/getting-started.md](docs/getting-started.md)

**All platforms:**

```bash
git clone https://github.com/osdf-systems/osdf.git
cd osdf
cargo test --workspace
cargo build --release -p osdf-cli
```

<details>
<summary>macOS / Linux</summary>

```bash
./target/release/osdf verify fixtures/valid/valid-committed.osdf
chmod +x scripts/*.sh
./scripts/build-wasm.sh
./scripts/serve-web.sh
```

</details>

<details>
<summary>Windows</summary>

```powershell
.\target\release\osdf.exe verify fixtures\valid\valid-committed.osdf
.\scripts\build-wasm.ps1
.\scripts\serve-web.ps1
```

</details>

## Pull requests

1. Branch from `main`
2. Keep changes focused
3. Run before opening PR:

   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

4. Update `specs/` and `docs/` when behavior changes
5. If you add or change helper scripts, update both `.sh` and `.ps1` when behavior should match

## Regenerate fixtures

```bash
cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored
```

## Security

Report vulnerabilities privately. See [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions are licensed under the project's Apache-2.0 OR MIT license terms.
