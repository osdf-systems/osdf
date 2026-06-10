# Architecture

**Status:** Public alpha reference model. Diagram reflects shipped verifier and gateway PoC behavior; planned layers are marked.

OSDF treats **trust as a pipeline**, not a single signature check. Data moves through explicit layers; each layer can pass, warn, or fail closed before the next step runs.

---

## Zero Trust document flow

```mermaid
flowchart TB
    subgraph ingress["Ingress (untrusted zone)"]
        A[Email / upload / API attachment]
        B[Gateway receive event planned]
    end

    subgraph gate["Policy gate (verify before deliver)"]
        C[Container safety<br/>ZIP walk, path rules, zip limits]
        D[Manifest integrity<br/>declared objects, SHA-256]
        E[Revision chain + Ed25519 signatures]
        F[Identity resolution<br/>configured trust registry]
        G[Ledger inclusion proof<br/>embedded transparency log]
        H[Freshness checks<br/>latest revision registry]
    end

    subgraph verdict["Verdict"]
        V{ALLOW / WARN / QUARANTINE / REJECT}
    end

    subgraph deliver["Authorized zone"]
        I[Parse once / cached inspect<br/>fast + parsed profiles]
        J[Human render profile<br/>gateway tax form PoC]
        K[Forensic export<br/>JSON report planned bundle]
    end

    subgraph egress["Egress (controlled export)"]
        L[Transformation receipt planned<br/>export / render / redact]
        M[PDF or Office view<br/>interop adapter planned]
    end

    A --> B --> C
    C --> D --> E --> F --> G --> H --> V
    V -->|pass or warn| I
    I --> J
    I --> K
    J --> L --> M

    V -->|fail| Q[Quarantine / reject<br/>no plaintext delivery]
    C -.->|fail closed| Q
    D -.->|fail closed| Q
    E -.->|fail closed| Q
```

---

## Verification layering (permanent)

The verifier **always separates** these guarantees. A passing offline result is not the same as “latest revision” or “issuer is who you think.”

| Layer | Question answered | Alpha status |
| --- | --- | --- |
| 1. Container safety | Is the archive structurally safe to parse? | Shipped |
| 2. Manifest integrity | Does every byte match declared digests? | Shipped |
| 3. Signatures | Is the revision chain cryptographically signed? | Shipped |
| 4. Organizational identity | Does the key map to configured trust? | Shipped (local registry) |
| 5. Ledger inclusion | Was this revision logged in a trusted append-only log? | Shipped (embedded proof) |
| 6. Freshness | Is this the newest known revision? | Partial (file registry; live optional) |
| 7. Revocation | Were keys valid at signing time? | Planned |

Details: [specs/phase-b3.md](../specs/phase-b3.md)

---

## Profile placement in the pipeline

```mermaid
flowchart LR
    subgraph wire["New bytes on the wire"]
        P1[OSDF-Core portable full<br/>ZIP + forensic report]
        P2[OSDF-Core portable fast<br/>ZIP + compact pass/fail]
    end

    subgraph hot["After parse_package once"]
        P3[OSDF-Core parsed fast<br/>revalidation only]
    end

    W[Incoming .osdf] --> P1
    W --> P2
    W -->|parse once| P3

    S[VerifyScheduler<br/>cores + RAM + package tier] -.-> P1
    S -.-> P2
    S -.-> P3
```

Benchmark each profile separately: [benchmarks.md](benchmarks.md)

---

## Component map

| Component | Role in the model |
| --- | --- |
| `osdf-core` | Layers 1-5 (+ partial 6) in Rust |
| `osdf-cli` / WASM | Same core, CLI or browser |
| `gateway/` | MFA + render after verify (PoC) |
| `VerifyPlan` / `scale_bench` | Thread and profile selection for throughput |
| Transformation receipt (draft) | Signed ingress/export provenance |
| Offline bundle (draft) | Point-in-time audit export |

Roadmap: [roadmap.md](roadmap.md)
