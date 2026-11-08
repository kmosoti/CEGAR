# Finding 0002: A bounded Sans-I/O CEGAR kernel is testable without inference

- Date: 2026-11-07 US Central
- Origin: automated implementation and investigation; human review pending
- Evidence: [validation record](../evidence/0002-bounded-core-validation.md), [PR #3](https://github.com/kmosoti/CEGAR/pull/3)
- Scope: finite supplied labeled graphs, not a host or natural-language truth verifier

## Observations

The dependency-free Rust implementation compiled on the pinned Rust 1.100 beta and passed formatting, Clippy, debug/release tests and a Python-to-Rust differential test. Exhaustive comparisons on all specified graphs up to three states found no disagreement with transitive closure. Every unsafe witness emitted in the Python comparison replayed against the original labeled graph. See the evidence record for exact counts, seeds, revisions, failures and commands.

## Deductions that informed the implementation

**Preserve action identity.** A path through state IDs alone is insufficient when different actions connect the same states. The quotient edges, concrete feasibility calculation and witness checker therefore all preserve action labels. Labels in this library are uninterpreted numeric IDs, not capabilities or execution authority.

**Keep the entire feasible frontier.** For a nondeterministic action, choosing one representative successor can lose a real bad path. Feasibility propagates all matching successors, with predecessor links only for reconstructing a concrete witness. A targeted regression exercises the case where only the second matching successor reaches Bad.

**Refinement needs a strict progress measure.** A spurious trace splits a nonempty proper subset from one abstract block. Each split increases the number of blocks; at most n-1 splits occur. All concrete edges are reprojected into the refined quotient. This supplies a finite progress argument without a solver or dependency-heavy predicate abstraction.

**Soundness and computational advantage are different claims.** Direct concrete reachability is simpler and may be faster at these small sizes. CEGAR here is an inspectable research implementation, not a demonstrated performance improvement over BFS. Keep the independent exact oracle even if a future optimized checker replaces the current algorithm.

**Cancellation and budgets must apply before returning a witness.** The core spends work budget on final replay as well as search/refinement. Running out during replay is inconclusive, not a partially checked unsafe result or accidental approval. Tests vary the budget at every work cutoff for a fixture and cancellation at every visit for a separate fixture; this is not exhaustive coverage of all cancellation schedules on all graphs.

**A finite-model library is not the full service.** Dense internal IDs make bounded allocation and traversal straightforward. They do not implement the draft JSON schemas, trusted collector, artifact plan compiler or policy rules. The example process protocol is deliberately test-only. Keep adapters explicit rather than making a library constructor accept authority-bearing JSON.

## Autonomous decisions and boundaries

[ADR 0003](../decisions/0003-finite-cegar-kernel.md) records the choice of a no-dependency core, frontier partition refinement, dense IDs, a dated beta toolchain, explicit work/cancellation limits, isolated test transport, independent oracles and API-based authoring exception. These decisions are implemented proposals awaiting human review, not owner-approved policy changes.

## What would change these conclusions?

- A generated model whose exact reachability disagrees with `verify` would invalidate implementation confidence and become a minimized permanent regression fixture.
- A reported witness with an absent labeled edge, wrong initial state or non-bad endpoint would invalidate witness handling.
- A refinement that drops a concrete edge or fails to increase block count would invalidate the soundness/progress argument.
- A workload demonstrating excessive work compared with direct BFS would justify an alternate algorithm; no speed superiority is presently claimed.
- A genuine domain/compiler requirement could justify SMT behind a bounded adapter. No such dependency is needed merely to check these graphs.

## Next highest-information work

Independent review of the abstraction/refinement code, direct tests of intermediate partition invariants, mutation testing, and a typed artifact-workflow compiler with a separate exact oracle. Only then introduce trusted snapshots and service isolation. Neural adapters are not required to establish the finite checker's correctness.
