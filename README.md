# CEGAR

**Counterexample-Guided Abstraction Refinement for evidence-grounded agent plans.**

CEGAR explores a local Rust service that turns proposed agent actions into explicit, bounded, inspectable verification problems. Optional neural components may propose plans or identify claims. Trusted collection, versioned policy and deterministic checking must remain separate from those predictions.

> **Implementation branch status:** the dependency-free [finite-state core](crates/cegar-core/README.md) is built and tested in GitHub Actions on a pinned Rust 1.100 beta. The planned HTTP service, trusted snapshot collector, artifact-workflow compiler, SMT/neural adapters and executor are **not implemented**. This is a draft research contribution, not a production service or G15 deployment.

## What exists now

`cegar-core` validates an immutable supplied finite model, builds an overapproximating quotient, checks concrete feasibility of abstract counterexamples, refines spurious traces and replays real witnesses. Work/refinement limits and cancellation produce explicit inconclusive outcomes. It uses no third-party crates, GPU, filesystem, network or hidden clock.

The current bounds are 128 states and 512 transitions. Results apply only to the supplied finite model, whose completeness and real-world fidelity are not established by the checker. `execution_authorized()` is always false.

[Historical validation evidence](knowledge/evidence/0002-bounded-core-validation.md) covers the earlier design; [Rust-only xtask validation](knowledge/evidence/0003-rust-xtask-validation.md) records the current compiler, tests, failures, and passing CI run. Fifteen integration tests and one doc test pass in debug and release. The tests include 28,868 exhaustive small models; a separate Rust xtask independently checks reachability and witnesses. Test coverage is not a proof of every possible implementation behavior.

## Run the validation

The committed toolchain is `beta-2026-11-07`, observed in CI as `rustc 1.100.0-beta.12`, edition 2024. It is not mislabeled as a stable 1.100 release. With Rustup available:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline -- --nocapture
cargo test --workspace --release --locked --offline -- --nocapture
cargo run -p xtask --locked --offline -- verify
```

The xtask is development-only; no production JSON/HTTP adapter has been implemented.

## Why this project exists

A grammar-valid tool plan can still be unauthorized, stale, or based on invented facts. A solver can correctly prove a property of an incorrect model. CEGAR studies these boundaries and makes scope and unresolved evidence visible.

The first intended domain workflow is: stage an immutable artifact, run registered tests against its exact digest, then publish that same digest. The compiler and trusted evidence path for that workflow remain future work. V0 is intended to review plans, not execute them.

**Models propose; trusted collectors observe; policies constrain; verifiers establish scoped results.** A future executor must independently enforce live authorization. A source span proves occurrence, not truth. Unknown or stale evidence must never quietly become permission.

## Proposed service architecture

```text
Authenticated private client
           |
   Rust transport / lifecycle shell           [planned]
           |
   Trusted snapshot + typed plan compiler    [planned]
           |           ^
           |     Optional neural suggestions  [planned, untrusted]
           v
   Bounded finite CEGAR kernel                [implemented/tested]
           |
   Scoped receipt or concrete counterexample
           |
   NO EXECUTION AUTHORITY
```

The core is Sans-I/O. External dependencies belong behind small explicit adapters, not a general plugin framework. Real transport deadlines, process RAM ceilings, trusted observation provenance and eventual runtime authorization remain outside this first library slice.

## Design and research

| Document | Purpose |
| --- | --- |
| [AGENTS.md](AGENTS.md) | Agent guardrails and scope |
| [CONTRIBUTING.md](CONTRIBUTING.md) | JJ workflow, issues and evidence |
| [Rust xtask](xtask/README.md) | Rust-only development oracle and reproducibility |
| [Knowledge index](knowledge/README.md) | Findings, evidence and autonomous decisions |
| [Core README](crates/cegar-core/README.md) | Implemented library contract and limits |
| [Architecture](docs/ARCHITECTURE.md) | Proposed full-service boundaries |
| [Formal model](docs/FORMAL_MODEL.md) | Abstraction and verification obligations |
| [Contracts](docs/CONTRACTS.md) | Proposed service contracts, not implemented endpoints |
| [Security](docs/SECURITY.md) | Threat model and authority boundaries |
| [Research audit](docs/RESEARCH_AUDIT.md) | Source claims and unsupported shortcuts |
| [Deployment](docs/DEPLOYMENT.md) | Candidate G15 environment; OS tuning deferred |
| [Evaluation](docs/EVALUATION.md) | Quality and operational qualification |
| [Roadmap](docs/ROADMAP.md) | Staged acceptance gates |
| [Agent handoff](docs/AGENT_HANDOFF.md) | Current contribution and next review work |
| [Sources](docs/SOURCES.md) | Primary literature and upstream implementations |

Draft [schemas](schemas/) and [examples](examples/) are design artifacts. The library's dense state IDs do not silently implement those JSON contracts; a future adapter must validate and map them.

## Hardware and public provenance

The initial deployment target is a Dell G15 with RTX 3050 Ti Laptop GPU, 4 GiB VRAM and roughly 7.1 GiB OS-visible RAM. No performance, model fit, driver readiness or thermal claim is established by CI. The library requires no GPU. OS/CUDA/service-manager choices remain deferred and outside the core.

Record hypotheses, failed tests, counterexamples, source identities and negative findings. Keep raw transient logs in sanitized issue-linked CI artifacts, not Git. Never publish secrets, workplace data, private conversations or model weights. MIT covers this project's original work, not third-party artifacts automatically.

## Development tracking

[Issue #2](https://github.com/kmosoti/CEGAR/issues/2) tracks the first core milestone. [Draft PR #3](https://github.com/kmosoti/CEGAR/pull/3) is stacked on [draft design PR #1](https://github.com/kmosoti/CEGAR/pull/1). Human review is pending; no merge or issue closure is implied by passing CI.

Preferred local workflow is **JJ with colocated Git**. The automated contribution's restricted-environment GitHub API exception is recorded in [ADR 0003](knowledge/decisions/0003-finite-cegar-kernel.md). No main or shared-branch history was rewritten.
