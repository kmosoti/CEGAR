# ADR 0002: Prefer a bounded Rust Sans-I/O verification core

- **Status:** Proposed
- **Date:** 2026-11-07
- **Tracking:** [Issue #2](https://github.com/kmosoti/CEGAR/issues/2), [draft PR #1](https://github.com/kmosoti/CEGAR/pull/1)

## Context

CEGAR's most consequential requirement is not fast model inference but an auditable distinction between untrusted claims, trusted observations, policies, and scoped verification. Direct network, storage, model or system-process calls inside verification logic increase the trusted computing base and make deterministic adversarial testing difficult.

A pure functional core and explicit edge adapters allow exact finite-state reference checks, simulation and property testing. However, a large trait hierarchy or generalized orchestration framework could make the project slower to understand without increasing correctness.

## Decision

Make the first `cegar-core` an ordinary Rust library with no I/O, model runtime, clock, filesystem, network or host mutation dependencies. Express inputs, immutable transitions, obligations and typed outcomes as data. Put all real-world observation, authentication, resource loading and persistence behind explicit adapters in the service shell.

Use **concrete functions and data structures by default**. Define a trait only where an actual substitutable dependency or test double needs it. Consider hexagonal ports for stable external boundaries, not as an obligation to build a generic ports-and-adapters framework.

Require strict bounds on state cardinality, transition count, trace length, refinement attempts and worker budgets. Refuse contradictory inputs. Compare CEGAR results against an independent concrete reachability oracle. Do not confuse a finite-model safety result with authorization.

## Alternatives

- **Monolithic Axum service with embedded inference and solver:** initially simpler wiring, but harder to test and shared GPU/solver failure domains.
- **Generic plugin/event-bus architecture:** unnecessary complexity before two concrete implementations prove need.
- **Unconditional Z3/egraph integration:** adds native/solver complexity without improving finite-graph checks already decidable by direct reachability.
- **Thin native wrappers around Python:** useful as a research spike, but should not become the primary trusted core.

## Tradeoffs and reversal criterion

Sans-I/O introduces explicit state passing and some adapter code. If a future measured workflow shows a true need for stateful incremental solver objects or GPU-side batching, keep the core's public semantic contract pure while allowing a **bounded worker adapter** to manage state internally. Do not force purity across hardware APIs where it makes correctness worse.

This ADR is a **proposed architectural preference**, not a measured performance or formal proof. Revisit it after the first compiled differential tests and real integration bottlenecks.
