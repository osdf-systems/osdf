# Contributing to OSDF

Thank you for helping build the Open Secure Document Format.

## Repository

https://github.com/osdf-systems/osdf

## Branch model

- `main` — always compiles, tests pass, source of release tags
- `feat/*`, `fix/*`, `docs/*`, `chore/*` — short-lived branches merged via pull request

Do not commit directly to `main` once branch protection is enabled.

## Development setup

```powershell
git clone https://github.com/osdf-systems/osdf.git
cd osdf
cargo test --workspace
```

Build the CLI:

```powershell
cargo build --release -p osdf-cli
.\target\release\osdf.exe verify fixtures\valid\valid-committed.osdf
```

Build the browser verifier:

```powershell
.\scripts\build-wasm.ps1
.\scripts\serve-web.ps1
```

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

## Regenerate fixtures

```bash
cargo test -p osdf-core --test generate_fixtures write_fixtures -- --ignored
```

## Security

Report vulnerabilities privately — see [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions are licensed under the project's Apache-2.0 OR MIT license terms.
