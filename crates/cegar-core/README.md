# cegar-core

Dependency-free Rust finite-state verification experiment. **Not an executor, workspace auditor, model adapter or production service.** See the repository's formal model and security design.

## Current API

`Model::new(n, initial, bad, transitions)` validates an immutable model with dense state IDs `0..n`. It rejects an empty initial set, unknown/duplicate set members, invalid edges and size overflow. Duplicate edges and self-loops are semantically legal. Hard structural caps are 128 states and 512 transitions. Action IDs are `u32` labels, not tool permissions.

`verify(&model, limits)` returns a `Report` with `VerifiedWithinModel`, `ConcreteCounterexample(witness)`, or `Inconclusive(reason)`. `verify_with_cancel` accepts an explicit cooperative cancellation/deadline hook. `replay` checks a bounded concrete witness independently against initial states, labeled transitions and bad endpoints.

A positive result exhausts reachability of the supplied finite model; it is not merely a depth-limited search. No real-world completeness, policy correctness, permission or trusted snapshot is asserted. Every report's execution-authorization accessor is false.

## Algorithm

Start with one block containing all states. Project every concrete edge and mark a block initial/bad when it contains an initial/bad state. BFS the existential quotient. Propagate concrete initial-state frontiers along the proposed **labeled** abstract trace. If no concrete continuation exists, split the blocking source block by the feasible frontier. If the final frontier misses Bad, split the final block instead. If the trace is real, reconstruct and replay it before returning it.

Each split partitions one nonempty block into two nonempty pieces. No concrete state or edge is removed. At most `n - 1` refinements are possible; the singleton quotient is exact. This is an intentionally small reference implementation, not an assertion that abstraction is faster than direct BFS for 128 states.

## Bounds

`max_work` counts algorithmic visits, not instructions, allocations or wall time. `max_refinements` caps splits. Exhaustion and cancellation are inconclusive, including during witness replay. Memory inside the checker is structurally bounded by the fixed graph caps and O(n^2 + m) temporary storage. The input caller, OS allocator, and cancellation callback are outside that bound. A future worker adapter must enforce transport-byte limits, hard process deadlines and RAM ceilings. A blocking callback cannot be interrupted by the core.

## Validation

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline -- --nocapture
cargo test --workspace --release --locked --offline -- --nocapture
cargo run -p xtask --locked --offline -- verify
```

Run from the repository root. The Rust-only xtask implements independent backward-reachability and witness checks. The previous example-only whitespace transport has been removed; production JSON/HTTP remains unimplemented. Public JSON schema parity, trusted observations, workflow compilation, Z3 and model adapters remain unimplemented. Historical validation is in `knowledge/evidence/0002-bounded-core-validation.md`; the Rust-only follow-up is `knowledge/evidence/0003-rust-xtask-validation.md`; no test result is implied by the presence of these commands.
