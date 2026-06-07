# Licensing

This document describes the **current** license for the public `osdf` repository and the **planned** move to Apache-2.0 OR MIT.

This is product documentation, not legal advice. For commercial use, OEM embedding, or redistribution in a paid product, contact [dan@osdfsystems.com](mailto:dan@osdfsystems.com).

---

## Current license (now)

The open core in this repository is licensed under the **[PolyForm Noncommercial License 1.0.0](../LICENSE)**.

**You may (noncommercial):**

- use, modify, and study the code
- distribute copies and derivatives, with license terms preserved
- rely on research, education, hobby, and qualifying nonprofit / government / academic use as described in the license

**You may not (without a separate agreement):**

- use the software in a **commercial** product or service (including selling access, support, or hosted verification built primarily on this code)
- assume Apache/MIT rights apply to this repository **today**

GitHub forks are allowed by GitHub’s platform; forked copies remain under **PolyForm Noncommercial** for the snapshot they forked unless and until the upstream license changes.

---

## Planned license (future open core)

OSDF Systems LLC intends to relicense the **open core** in this repository under **Apache-2.0 OR MIT** (recipient’s choice), matching the verifier-always-public product strategy.

**Planned trigger (whichever comes first):**

1. **Stable v1.0** release of the open core (format + verifier + CLI + WASM), or
2. **24 months** after the repository first became public, announced in [CHANGELOG.md](../CHANGELOG.md)

We will announce the change in the changelog, update `LICENSE`, and tag the first commit under the new license (for example `v1.0.0`).

Draft texts that will apply after relicense (not in effect yet):

- [licenses/future/LICENSE-APACHE](../licenses/future/LICENSE-APACHE)
- [licenses/future/LICENSE-MIT](../licenses/future/LICENSE-MIT)

---

## After relicense


| Audience                                           | Effect                                                                                                                                                            |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **New downloads** from OSDF after the announcement | Apache-2.0 OR MIT                                                                                                                                                 |
| **Existing forks** taken under PolyForm            | Stay PolyForm for that **historical snapshot**; merge upstream after relicense to pick up Apache/MIT for new commits                                              |
| **Contributors**                                   | Contributions before relicense were under PolyForm Noncommercial; after relicense, contributions follow Apache/MIT terms in [CONTRIBUTING.md](../CONTRIBUTING.md) |


---

## What stays outside this license

- **Trademarks** (`OSDF`, `OSDF Systems`) are not granted by the software license.

---

## Commercial licensing

Commercial use, dual-licensing questions, or early Apache/MIT grant: **[dan@osdfsystems.com](mailto:dan@osdfsystems.com)**.