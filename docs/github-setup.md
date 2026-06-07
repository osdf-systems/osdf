# GitHub repository setup

Checklist for making **osdf-systems/osdf** look and operate like a production open-source project. Do this once before or right after the first public release.

---

## 1. Default branch

Use **`main`** as the default branch (GitHub standard).

1. Merge your bootstrap branch (e.g. `chore/bootstrap-ci`) into `main`.
2. GitHub → **Settings** → **General** → **Default branch** → switch to `main`.
3. Delete merged feature branches when done.

---

## 2. Branch protection on `main`

**Settings** → **Branches** → **Add branch protection rule** (or **Rules** → **Rulesets** on newer GitHub UI).

### Recommended settings (solo or small team)

| Setting | Value |
| --- | --- |
| Branch name pattern | `main` |
| Require a pull request before merging | **On** |
| Required approvals | **0** (solo) or **1** (once you have reviewers) |
| Dismiss stale approvals | On (when you use reviewers) |
| Require status checks to pass | **On** |
| Required check | **`CI success`** (job name from `.github/workflows/ci.yml`) |
| Require branches to be up to date | **On** (recommended once others contribute) |
| Require conversation resolution | **On** |
| Include administrators | **Off** (so rules apply to you too) |
| Allow force pushes | **Off** |
| Allow deletions | **Off** |

### Why `CI success`?

The workflow runs Rust on Ubuntu and macOS, then a final **`CI success`** job fails if either platform fails. Require only that one check in branch protection instead of tracking each matrix job separately.

### Solo workflow

Even alone, use short-lived branches and PRs:

```bash
git checkout -b fix/my-change
# commit
git push -u origin fix/my-change
gh pr create --fill
gh pr merge --squash   # after CI green
```

### `gh` CLI (classic protection API)

After `main` exists and CI has run at least once (so the check name appears):

```bash
gh api repos/osdf-systems/osdf/branches/main/protection -X PUT \
  -f required_status_checks[strict]=true \
  -f required_status_checks[contexts][]="CI success" \
  -f enforce_admins=true \
  -f required_pull_request_reviews[dismiss_stale_reviews]=true \
  -f required_pull_request_reviews[required_approving_review_count]=0 \
  -f restrictions=null
```

If the API rejects unknown check names, merge one PR first so **CI success** appears under **Settings → Branches → status checks**.

---

## 3. Repository About (landing page)

**Settings** → **General** → scroll to **About** (or edit on the repo home page):

| Field | Suggested value |
| --- | --- |
| **Description** | Open Secure Document Format: local, fail-closed cryptographic document verification (Rust + WASM). |
| **Website** | `https://github.com/osdf-systems/osdf` (or your docs site later) |
| **Topics** | `rust`, `cryptography`, `zero-trust`, `document-format`, `merkle-tree`, `ed25519`, `wasm`, `security`, `verification` |

Enable **Releases**, **Issues**, and **Security advisories**. Disable **Wiki** unless you plan to use it.

---

## 4. Organization profile (optional)

**github.com/osdf-systems** → **Settings** → add org README, description, and public email if you have one. Matches the repo branding.

---

## 5. Security

| Item | Location |
| --- | --- |
| [SECURITY.md](../SECURITY.md) | Already in repo; GitHub surfaces it under **Security** |
| Private advisories | **Security** → **Advisories** → enable |
| Dependabot alerts | **Settings** → **Code security** → enable Dependabot alerts + security updates |
| Secret scanning | Enable for public repos (free) |

Update `security@osdf.systems` in SECURITY.md when that mailbox is live.

---

## 6. Files already in this repo

| File | Purpose |
| --- | --- |
| `.github/workflows/ci.yml` | Ubuntu + macOS CI, **`CI success`** gate |
| `.github/workflows/release.yml` | Tag-triggered releases |
| `.github/dependabot.yml` | Weekly Cargo + Actions update PRs |
| `.github/ISSUE_TEMPLATE/` | Bug and feature forms |
| `.github/pull_request_template.md` | PR checklist |
| `CODE_OF_CONDUCT.md` | Community standard |
| `CONTRIBUTING.md` | Contributor guide |
| `CHANGELOG.md` | Release notes |
| `LICENSE-APACHE` / `LICENSE-MIT` | Dual license |
| `.editorconfig` / `.gitattributes` | Consistent formatting |

---

## 7. First release (professional signal)

When ready for alpha:

```bash
git tag v0.1.0-alpha.1
git push origin v0.1.0-alpha.1
```

GitHub Actions release workflow publishes assets. Edit the release notes to summarize verification scope (see CHANGELOG).

---

## 8. What not to commit here

Do not commit credentials, customer-specific policy packs, or internal-only documentation to this repository. Keep those in separate private storage with appropriate access controls.

---

## Quick verification

- [ ] Default branch is `main`
- [ ] Branch protection requires PR + **`CI success`**
- [ ] About section and topics filled in
- [ ] Dependabot and security advisories enabled
- [ ] No secrets in git history (`ledger-key.json` stays gitignored)
- [ ] First tag release when alpha-ready
