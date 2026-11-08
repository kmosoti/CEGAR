# CEGAR

**Counterexample-Guided Abstraction Refinement for evidence-grounded agent plans.**

CEGAR is an experimental Rust-based local service for turning proposed agent actions into **explicit, bounded, inspectable verification problems**. Optional neural models may extract claims, classify intent, or propose candidate plans. Trusted observations, operator policy, and deterministic verification remain the authority for whether a modeled plan satisfies its specified constraints.

> **Project status: design / research, not a running production service.** This branch establishes the architecture, contracts, security boundaries, evaluation methodology, and agent handoff. No neural adapter, trusted snapshot collector, production solver service, or executor is claimed to be implemented, compiled, or benchmarked.

## Why this project exists

A model can produce valid JSON for a plan that is nevertheless unauthorized, stale, unsound, or based on invented facts. A symbolic solver can prove a property of an incorrect model with perfect consistency. CEGAR studies the boundary between probabilistic interpretation and verifiable evidence, then makes that boundary visible in its API and experiments.

The motivating use case is a **bounded artifact workflow**:

1. Stage a candidate artifact.
2. Test the *exact* candidate.
3. Publish only an artifact whose identity matches the passed test attestation.

CEGAR v0 **reviews** such workflows. It does **not** execute them.

## Core principle

**Models propose; trusted collectors observe; policies constrain; verifiers establish scoped results; executors, if added later, independently enforce live authorization.**

A neural prediction is not an authoritative fact. A source span is not proof that its statement is true. A solver result applies to its encoded assumptions and horizon, not necessarily to the real machine. Missing evidence produces an explicit unresolved outcome.

## Architecture at a glance

```text
Authenticated private client
           |
   CEGAR HTTP service (Rust)
           |
  +--------+---------+-------------------+
  |                  |                   |
Evidence / snapshot  |          Optional neural adapters
collector             |          (predictions only)
  |                  |
  +--> Typed plan and policy IR
                         |
                Deterministic validation
                         |
                 Bounded CEGAR / SMT
                         |
           Scoped receipt or counterexample
                         |
                 NO EXECUTION IN v0
```

The core should be **sans-I/O**: pure data types, finite-state semantics, planning obligations, refinement decisions, and testable algorithms. Network access, persistence, GPU work, subprocess isolation, and collection are adapters outside the core.

## Documents

| Document | Purpose |
| --- | --- |
| [AGENTS.md](AGENTS.md) | General instructions and guardrails for coding agents |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Public contribution and review expectations |
| [Architecture](docs/ARCHITECTURE.md) | Components, boundaries, authority and lifecycle |
| [Formal model](docs/FORMAL_MODEL.md) | Meaning of CEGAR, proof obligations and solver outcomes |
| [Contracts](docs/CONTRACTS.md) | Proposed types, API, receipt and compatibility rules |
| [Security](docs/SECURITY.md) | Threat model, secrets, network and eventual executor boundary |
| [Research audit](docs/RESEARCH_AUDIT.md) | Verified capabilities versus unverified workshop claims |
| [Deployment model](docs/DEPLOYMENT.md) | Resource-aware G15 candidate, with OS configuration deferred |
| [Evaluation](docs/EVALUATION.md) | Benchmarks, correctness, evidence levels, and promotion gates |
| [Roadmap](docs/ROADMAP.md) | Milestones and acceptance criteria |
| [Agent handoff](docs/AGENT_HANDOFF.md) | First engineering slice and definition of done |
| [Sources](docs/SOURCES.md) | Primary-source reading list |

Proposed machine-readable examples live under [schemas](schemas/) and [examples](examples/). These are **design inputs** until the compiler, verifier, and integration tests validate their semantics.

## Initial scope

- One closed-world, explicitly modeled artifact workflow.
- Typed evidence with source identities and snapshot revisions.
- A bounded safety check and explainable counterexample or inconclusive outcome.
- A read-only HTTP review API with authenticated private access.
- A testable reference model and differential/property tests.
- Optional LiquidAI d1-2B integration **after** the non-neural verifier is useful.

Out of scope: unconstrained web browsing, arbitrary shell execution, automatically repairing policies, general natural-language truth verification, unrestricted local model loading, and automatic tool execution.

## Target environment

Development target: **Rust 1.100, edition 2024**. Actual compiler and dependency versions must be pinned by the first implementation PR, then evidenced by a successful build. Initial hardware research concerns a Dell G15 with RTX 3050 Ti Laptop GPU (4 GiB VRAM) and roughly 7.1 GiB OS-visible RAM. The operating system, CUDA/runtime packaging, and host deployment configuration are deliberately left open.

The service architecture should not assume a particular Linux distribution. Machine-specific configuration belongs in infrastructure-as-code, not in CEGAR's domain core.

## Status vocabulary

Use **Proposed**, **Source-audited**, **Built**, **Tested**, **Measured-on-G15**, and **Qualified** precisely. No file or benchmark number is upgraded by implication. See [Evaluation](docs/EVALUATION.md).

## Public research and provenance

Record hypotheses, contrary evidence, model/runtime revisions, reproducible inputs, failed tests, and architectural decisions. Avoid publishing private chat messages, workplace artifacts, credentials, proprietary code, access policies, or model weights. The repository's MIT license applies to its original code and documentation, not automatically to third-party checkpoints.

## Start here

Read [AGENTS.md](AGENTS.md), [Agent handoff](docs/AGENT_HANDOFF.md), and [Formal model](docs/FORMAL_MODEL.md). The first useful contribution is **not** an AI model integration: it is an independently testable, bounded, non-neural verification slice.

