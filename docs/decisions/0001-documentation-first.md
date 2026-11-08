# ADR 0001: Documentation-first and verification-first development

- Status: **Proposed**
- Context: public CEGAR project and G15 exploration
- Date: 2026-11-07

## Context

CEGAR combines formal verification, evidence provenance, agent proposals, and optional local inference. The workshop produced a substantial architecture and an experimental reference bundle, but no qualified Rust service or GPU model deployment has been demonstrated.

Committing uncompiled Rust and labeling it operational would obscure what was proven. Building four neural components before a trusted plan model exists would increase failure surfaces without establishing that the verifier is useful.

## Decision

Land an explicit architecture, formal model, public contribution policy, trust/security model, research claim audit, draft schemas, and an actionable first-code milestone.

Implement the smallest **sans-I/O Rust finite-model verifier first**, with independent concrete reachability and adversarial test fixtures. Add the trusted snapshot/contract compiler and bounded HTTP service next. Neural integration is an optional later phase. V0 has **no execution endpoint**.

Keep host-specific NixOS/Fedora/CUDA/service-manager details outside the pure core and defer until the G15 is ready.

## Consequences

Benefits: honest public maturity, smaller reviewable implementation slices, testable proof boundaries, portable core, and explicit separation of prediction from authority.

Costs: no ready-to-run server from the documentation PR; first implementation PR must pin dependencies, validate the draft contracts, compile, test, and publish raw evidence.

## Revisit when

A compiled reference verifier passes differential tests, or a workload proves neural assistance is necessary for a specific task. Changes to trusted authority or execution privileges require a new ADR and security review.

