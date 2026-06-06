# Documentation updates

When implementing or changing OSDF features, update in-repo documentation in the same work stream.

## Required updates

- **Format / crypto / verification changes** → matching `specs/phase-*.md` and `docs/` as applicable.
- **New CLI flags or commands** → relevant phase spec and `README.md` CLI section.
- **New WASM exports** → `docs/web-verifier.md`.
- **New verification checks or report fields** → `specs/phase-b3.md` or `specs/phase-d.md` and fixture expectations.

The HTML docs site and marketing homepage live in a **separate repository** — do not add them to this public core repo.

Do not ship user-visible behavior without documenting it in `specs/` or `docs/`.
