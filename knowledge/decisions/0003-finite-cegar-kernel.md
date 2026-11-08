# ADR 0003: Dependency-free finite CEGAR kernel and explicit adapter limits

- Date: 2026-11-07 US Central
- Status: Implemented in a draft contribution; **human review pending**
- Decision origin: automated engineering contribution requested by the repository owner
- Tracks: issue #2; stacked on the documentation branch, not a main-branch change

## Decisions made autonomously

1. Implement one ordinary Rust crate with **no third-party dependencies**. Typed immutable input and typed results come before HTTP, JSON serialization, SMT, inference or a plugin framework.
2. Represent state IDs as dense indices; numeric action labels are part of trace identity. This differs from the draft bit-width/explicit-state JSON design. It is a library contract, not a silent implementation of that draft JSON schema. A later transport adapter must validate and map external IDs.
3. Use an existential quotient over a partition of all states. Split a spurious trace's concrete frontier from the rest of the blocking block. Rebuild quotient reachability after each split.
4. Verify finite reachability to exhaustion, not a fixed-depth approximation. Work/refinement exhaustion returns inconclusive. Empty initial sets are invalid rather than a vacuous safety success.
5. Separate deterministic work/cancellation from hard time/RAM enforcement. The pure core does not read a clock. The future worker owns OS resource enforcement; arbitrary blocking callbacks are not preemptible.
6. Use a Rust transitive-closure oracle and a separate Python-to-Rust process comparison. They must check action-labeled witnesses, not only safe/unsafe flags.
7. Pin a dated Rust 1.100 beta candidate (`beta-2026-11-07`) and require CI to establish availability and report the actual compiler. An unavailable toolchain is a failed gate, never an implicit successful build. The dependency-free lock is initially specified directly and must pass Cargo `--locked` checks.
8. Use a stacked implementation PR so the owner's linked design branch and main history remain unchanged. The sandbox lacks JJ/Cargo and outbound DNS; this contribution uses the authenticated GitHub Git-data API with non-forced, expected-head checks. This is an environment exception, not a change to the preferred colocated JJ workflow.

## Proof obligations

For partition pi, every concrete edge (s,a,t) projects to (pi(s),a,pi(t)); abstract initial and bad sets are existential images. Thus concrete bad traces are included in abstract bad traces. A safe quotient is safe for the supplied model.

Concrete trace feasibility uses all matching successors, not one representative. On an infeasible step, the frontier is a nonempty proper subset of its block: an abstract edge exists, but none of the feasible concrete sources can take it. At the final step, a frontier disjoint from Bad is proper because its block contains a bad state. Splitting preserves every state and projected concrete edge and strictly increases block count. At most n-1 splits are possible.

These are mathematical arguments about the algorithm. Tests and review are needed to validate the implementation. They do not prove the input model faithfully represents a host, policy, or complete failure semantics.

## Alternatives rejected for this slice

- Bit-predicate refinement: potentially useful later; partition/frontier splitting is smaller and admits a finite progress argument.
- Z3/egglog: not needed for finite graph reachability and would expand the trusted dependency surface.
- Always run exact BFS in production: useful as a reference, but would not exercise actual spurious-counterexample refinement. The current kernel is explicitly educational/research; no performance advantage is claimed.
- A trait per component or no_std conversion: unnecessary friction without a demonstrated target.
- Implementing all proposed JSON/HTTP contracts immediately: risks conflating untrusted supplied graphs with trusted workspace verification.

## Revisit

After measured larger-state workloads, contract review, or a genuine need for symbolic states. Preserve witness replay, conservative outcomes, dependency provenance and the explicit distinction between model safety and execution authority.
