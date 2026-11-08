# Formal model and proof obligations

**Status:** design of a bounded verifier. These arguments do not certify an implementation that has not been built and tested.

## Two loops, two meanings

CEGAR stands for **Counterexample-Guided Abstraction Refinement**. Its *inner loop* refines an overapproximating model after a spurious counterexample. An *outer loop* can ask a human or planner to propose a new plan in response to a **real** counterexample. Changing a plan is not refinement of an abstraction.

## Concrete safety problem

Model a closed, finitely represented system:

- `S`: concrete states.
- `I ⊆ S`: possible initial states.
- `T ⊆ S × S`: permitted concrete transition relation, including modeled failure branches.
- `Bad ⊆ S`: states violating a versioned invariant.
- `H`: explicitly checked schedule/horizon bound, or an indication that the finite model was exhausted.

A *bad trace* is a sequence `s₀, …, sₙ` where `s₀ ∈ I`, each `(sᵢ,sᵢ₊₁) ∈ T`, and `sₙ ∈ Bad`. Checking this trace answers **the safety property as encoded**, not whether the entire real system is faithfully modeled.

First validate input consistency. `I = ∅`, incompatible policy facts, missing required failure transitions, or unresolved critical observations must not yield a positive receipt through vacuity.

## Sound abstraction

Let `αᵢ: S → Aᵢ` map concrete states to abstract states, with abstract initial set `Iᵢ#`, transitions `Tᵢ#`, and bad set `Badᵢ#`.

Required simulation obligations:

1. `αᵢ(I) ⊆ Iᵢ#`.
2. If `(s,t) ∈ T` then `(αᵢ(s),αᵢ(t)) ∈ Tᵢ#`.
3. `αᵢ(Bad) ⊆ Badᵢ#`.

Then every concrete bad trace maps to an abstract bad trace. Therefore **if there is no reachable abstract bad state, there is no concrete bad trace in the modeled scope**.

This proof fails if the abstraction drops a transition, initial state, or bad-state possibility. An implementation must test its abstraction constructor and preserve these obligations.

## CEGAR refinement

```text
Construct sound abstraction
         |
Check abstract reachability of Bad#
         |
    +----+-------------------+
    |                        |
No bad trace         Abstract bad trace
    |                        |
VerifiedWithinModel    Concrete feasibility check
                              |
                     +--------+---------+
                     |                  |
                   Real              Spurious
                     |                  |
            ConcreteCounterexample   Refine alpha, repeat
```

The refinement must exclude the spurious behavior **without excluding any concrete behavior**. If its budget is exhausted, return `inconclusive`. Never declare a spurious abstract trace to be a real exploit without replay.

For finite exact reachability, breadth-first search already establishes safety of the provided model; abstraction is most useful when it reduces a larger problem. Do not introduce a solver merely to label a graph search as formal.

## Meaning of solver outcomes

When the solver is asked if a **bad execution exists**:

- `sat`: the encoding permits a bad execution; retrieve and replay a candidate witness against the concrete modeled transitions before reporting a concrete counterexample.
- `unsat`: no bad execution satisfies *this encoding and scope*, subject to correct encoding, consistent assumptions, and solver correctness.
- `unknown` / timeout / resource limit: `inconclusive`; no execution authorization.

An **unsat core** is a subset of conflicting assertions, not a bad execution trace. The meaning of `sat` and `unsat` reverses if the user asks a different logical question. Record query type in the receipt.

## Example: artifact identity and stale attestations

Let `Staged(a)` mean candidate artifact digest `a` exists. Let `Passed(a,t)` represent an attestation for artifact `a` produced by an authorized test suite `t` and passing under the snapshot's trust rules.

A publish operation for digest `b` requires:

```text
Staged(b)
AND Passed(b, required_suite)
AND authorized(principal, publish, destination)
AND current(snapshot, policy, tool_contract)
```

A test of `a` does **not** establish `Passed(b,t)` when `a ≠ b`. A mutable file path is not an immutable digest. A model's statement that tests passed is not an attestation.

The test must be against the exact bytes eventually published, with source-of-truth rechecks at the eventual executor boundary. V0 does not provide that executor.

## Uncertainty and three-valued critical predicates

For a critical proposition `p`, use known true, known false, or unresolved. Equivalently, an unresolved Boolean denotes the set `{true,false}`. A solver may enumerate possibilities within its scope, but **unknown permission does not become allow** simply because one satisfying assignment exists.

Authority comes from trusted collection and policy, not the serialization label or a numeric model probability.

## Plans, concurrency, and failures

A plan is a DAG of typed operations, resources, effects, dependencies, and potential failure outcomes. Validate graph structure separately (Kahn's algorithm or DFS). Model all permitted interleavings that affect safety or explicitly state a sequential/horizon restriction.

When a precondition check and actual execution can race, a verification receipt cannot replace an executor-side live check. Otherwise a valid proof about time `t₀` is misapplied at time `t₁`.

## Receipts and soundness boundaries

A positive result must say `verified_within_model`, not `globally_safe`. Include plan digest, trusted snapshot/policy/tool-contract digests, compiler/verifier revisions, query semantics, properties, assumptions, horizon, modeled failures, proof kind, timestamps, and unresolved facts.

A result may be correct as a proof of a flawed model. Validation of the trusted computing base requires differential testing, mutation tests of the encoder, explicit failure models, and conservative result types.

## Test obligations

- Abstraction simulation preserves initial, transition and bad-state coverage.
- Every reported concrete counterexample replays successfully.
- Differential tests compare against exact concrete reachability for small finite models.
- Missing evidence, contradictory assumptions, stale snapshots and exhausted budgets never return positive status.
- Repair cannot weaken policy or mutate trusted evidence.
- A positive receipt remains bound to exact model and source revisions.

**Reference:** [CEGAR paper](https://web.stanford.edu/class/cs357/cegar.pdf), [Z3 guide](https://microsoft.github.io/z3guide/docs/logic/basiccommands/).

