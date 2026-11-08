# Implementation agent handoff

## Mission

Implement the **first non-neural CEGAR slice** from the architecture and contracts in this repository. Do not begin with d1 integration, an agent UI, a chat harness, or a privileged executor.

The public project is intended to demonstrate rigorous systems engineering: explicit assumptions, small testable components, formal semantics, resource bounds, provenance and reproducible negative results.

## Read order

1. `README.md` and `AGENTS.md`
2. `docs/FORMAL_MODEL.md`, especially abstraction soundness and outcome semantics
3. `docs/CONTRACTS.md` and the synthetic examples
4. `docs/SECURITY.md` and `docs/EVALUATION.md`
5. `docs/ROADMAP.md` for staged scope

## Initial code organization (suggestion, not mandated)

```text
crates/
  cegar-core/       Pure finite transition system, abstraction, trace replay
  cegar-server/     Axum shell later; do not add to first core PR unless justified
tests/
  fixtures/         Synthetic safe, unsafe and malformed systems
  differential/     Independent exact-reachability oracle
docs/
  decisions/        ADRs for changes to trust or solver semantics
```

Use Rust 1.100, edition 2024, and a reviewed Cargo.lock. The repo has no existing compiled source on this branch; **do not invent a passing build or lockfile**. If Rust 1.100 is not available in the environment, state that and use the appropriate available toolchain for preliminary checks, with qualified final build deferred.

## First PR target: pure finite-state checker

Inputs: bounded finite states, nonempty initial set, explicit bad-state set, labeled transitions and resource budgets. Outputs: `verified_within_model`, concrete counterexample with replayable states/actions, invalid input or inconclusive with reason.

1. Implement input validation with no panics for hostile JSON/model data.
2. Implement independent exact reachability as the test oracle.
3. Implement an overapproximating abstraction, check abstract reachability, test concrete feasibility, and refine only spurious traces.
4. Prove (in code/tests and documented argument) that abstraction transition coverage preserves concrete behavior.
5. Bound refinements/work/time/memory and make exhaustion explicitly inconclusive.
6. Generate deterministic fixtures: safe graph after spurious trace, real unsafe graph, initial bad state, empty initial invalid and low-budget inconclusive.
7. Run exhaustive or property-driven differential checks over small models; replay every returned unsafe trace against concrete edges.
8. Document precisely what a positive result means: **the supplied finite model only**.

If a direct exact checker is simpler, use it as the oracle first. Do not add Z3 merely for demonstration value. Do not ask a model to invent trusted preconditions.

## Expected PR evidence

- Exact compiler version and test/format/lint commands with exit codes.
- Changed-file summary and specification deltas.
- Tested fixture counts, seeds and known caveats.
- Bounded worst-case behavior and explicit rejection/error types.
- Scope statement: no host assurance, no execution authorization.
- A small ADR if the proposed semantics or authority model change.

## Subsequent PRs (in order)

1. Trusted snapshot / typed artifact contract compiler.
2. Authenticated read-only verification service with bounded workers and receipts.
3. One optional d1 adapter as untrusted predictions, with real G15 resource measurements.
4. Optional context packaging, local planner experiments and guarded workflow repair.

Each phase should preserve or extend tests. Do not combine all phases into one enormous agent-generated PR.

## Questions an implementation must answer

- Can any client field impersonate a trusted observation?
- Does a bad abstract trace always replay before being reported as concrete?
- Can a timeout/unknown/memory limit accidentally produce a positive status?
- Does a receipt name every assumption necessary for its narrow claim?
- Is the current state actually what the modeled policy assumed?
- What happens when model or solver workers fail?
- Is any claimed performance based on a real machine, or just arithmetic?

When unresolved, keep the system conservative and record the missing evidence. A visibly incomplete but sound boundary is better than a persuasive but unsafe one.

