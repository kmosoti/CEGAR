# Roadmap and acceptance gates

This document describes an implementation sequence, not completed work. Feature maturity must be reported precisely.

## Phase 0: documentary foundation (this PR)

- Establish scope, authority boundaries, formal semantics, security model, contracts, and research audit.
- Provide an actionable agent handoff, sample schema, and a public contribution process.
- Explicitly label all APIs and implementation claims as proposals.
- Exit when documents are internally consistent, source references are pinned where needed, and schemas parse.

## Phase 1: bounded non-neural kernel (first implementation PR)

- Rust 1.100 workspace and reviewed `Cargo.lock`; edition 2024.
- Pure finite-state model types, validation, exact concrete reachability and witness replay.
- Sound coarse abstraction, spurious-trace check and refinement budget.
- Tri-state outcomes including `inconclusive` under all limits.
- Differential/property tests against exhaustive enumeration for bounded graphs.
- No network access, GPU requirement, or execution action in the core.
- Exit when compiler/tests pass with captured commands and modeled counterexamples replay correctly.

## Phase 2: trusted artifact workflow

- Bounded read-only collector and revocable workspace capability.
- Versioned tool contracts and typed artifact IDs; no arbitrary paths from model output.
- Validated DAG, preconditions/effects, snapshot and policy freshness.
- Exact test-attestation to published-artifact digest binding.
- Negative fixtures for stale data, missing evidence, contradictory assumptions, unmodeled failures and identity swaps.
- Exit when trusted-input origin cannot be spoofed through request JSON and failing cases yield specific diagnostics.

## Phase 3: service and bounded verifier workers

- Axum API with authenticated private ingress, limits, cancellation, backpressure and audit fields.
- Isolated SMT process only if an obligation needs more than finite-state/graph checking.
- Immutable result receipts, counterexample serialization, operational monitoring.
- Deny direct public exposure or any execution endpoint.
- Exit when integration, crash, timeout, stale-token and denial-of-service tests pass.

## Phase 4: optional neural interpretation

- Qualify one text-only d1-2B/mistral.rs profile, including complete model artifacts and calibrations.
- Fixed question templates; deterministic state serialization and Unicode span validation.
- Evaluate abstention, false positives, grouping of questions and G15 memory/thermal peaks.
- Adapter emits `Prediction`/`Claim` only. Never authorized policy facts.
- Exit when measured benefit outweighs complexity on labeled tasks.

## Phase 5: optional plan proposals and context packaging

- Start with deterministic templates and frontier-agent proposals.
- Consider a small local planner only if it improves qualified task completion.
- Grammar-constrained syntax plus separate type/permission/DAG verification.
- Context packaging protects binding evidence and records omissions.
- Compare LLMLingua-2 and GLiNER2 only as measured alternatives; do not treat them as mandatory.
- Exit when real workloads demonstrate improved quality-qualified goodput without new authority paths.

## Phase 6: potential execution capability (separate project decision)

- Explicit principal/resource capabilities, live precondition checks and artifact identity.
- Race-safe enforceable operations, cancellation and compensation design, credential confinement.
- Independent security review and adversarial tests.
- No `POST /execute` until requirements and authority have been intentionally approved.

## Decision gates

Any phase may be rejected if its complexity or memory footprint exceeds its benefit. Favor a small transparent verifier over a fully featured model pipeline with weak provenance. Keep status labels and raw failed experiments public where disclosure is safe.

