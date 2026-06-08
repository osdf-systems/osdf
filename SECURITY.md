# Security Policy

OSDF is a security-focused document verification platform. We take vulnerability reports seriously.

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x alpha | Best effort during active development |
| < 0.1.0 | No |

## Reporting a vulnerability

**Do not open public GitHub issues for security vulnerabilities.**

Email: [dan@osdfsystems.com](mailto:dan@osdfsystems.com)

Include:

- Description of the issue
- Steps to reproduce
- Impact assessment
- Affected components (core, CLI, WASM, gateway)
- Proof-of-concept if available

We aim to acknowledge reports within 5 business days.

## Scope

In scope:

- Container parser bypasses (path traversal, undeclared objects, zip bombs)
- Signature or manifest verification bypasses
- Ledger proof validation errors
- WASM verifier sandbox escapes
- Supply-chain or release pipeline weaknesses

Out of scope (for now):

- Missing enterprise features not yet shipped (Companion, Key Broker, hosted ledger)
- Social engineering or physical access
- Authorized user screenshot or transcription of plaintext

## Safe harbor

Good-faith security research on public alpha releases is welcome. Do not access data you do not own or disrupt production systems.

## Side channels and timing

OSDF verification is designed for high throughput, but **fast verification is not the same as side-channel resistance**. Cryptographic primitives (Ed25519, SHA-256) come from well-audited libraries; the verifier wrapper applies additional hardening:

- **Constant-time digest comparison** — 32-byte digests and parsed `sha256:…` strings are compared with constant-time equality (`subtle::ConstantTimeEq`) so byte-by-byte early-exit does not leak how much of a guessed digest matched.
- **Full-scan audit path** — The structured audit verifier (`verify_audit`) hashes every declared object and checks every signature file before returning, collecting all failures in one report instead of stopping at the first error. This reduces timing oracles that reveal which object or signature failed first.
- **Deployment-layer controls for online gateways** — Internet-facing verification endpoints should not expose raw verifier timing to untrusted callers. Use fixed response delays, rate limiting, and isolated verifier workers in addition to the in-process hardening above.

**Threat model split:**

| Environment | Timing sensitivity |
| --- | --- |
| CLI / local WASM | Low — verification runs in the user's own environment |
| Offline batch / CI | Low — no adversary observing per-request timing |
| Online gateway | High — assume an adversary can measure response timing; combine full-scan audit mode with deployment controls |

Report suspected timing or side-channel issues through the contact above. We treat observable verification oracles in gateway deployments as in-scope.
