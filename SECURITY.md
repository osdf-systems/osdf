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

- Social engineering or physical access
- Authorized user screenshot or transcription of plaintext

## Safe harbor

Good-faith security research on public alpha releases is welcome. Do not access data you do not own or disrupt production systems.
