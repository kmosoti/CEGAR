# Research 0003: findings from the experimental domain compiler

- **Date:** 2026-11-08 (US Central)
- **Scope:** implementation notes for the Issue #7 deterministic slice
  (`crates/cegar-domain`). Synthetic fixtures only; no model adapter yet.
- **Status:** observations from a workstation prototype, not measurements of a
  service and not owner-approved semantics.

## 1. The independent reference disagreed with the compiler — and was right to

The first differential run failed:

```
fixture-two-unsafe-ready: step 0: recorded rules
["attempt.subject_not_staged"] differ from reference
["attempt.subject_not_staged", "premise.evidence_missing"]
```

The compiler skipped the scoped evidence predicate when a publish subject was not
staged; the reference evaluated "is the designated record present?" anyway and
added `premise.evidence_missing` because its own reachability code treated a
missing candidate as a missing record. Neither implementation was obviously
wrong, which is exactly the point: the design said *what* the predicate means but
not what is defined when the subject does not exist yet.

Resolution recorded in both implementations and in the crate README:

> When a publish subject is not staged, the attested-subject comparison is
> undefined. The attempt is still unsafe and still modeled. Presence of the
> designated record remains checkable.

Consequence for review: an attempt can carry several violated rules, and the
reported set must be the complete modeled set for that attempt, not just the
first one found. A witness that reports only part of the violated set is a
diagnostic defect, not a soundness defect, but it makes replay checks weaker.

## 2. Canonical ordering had to be an encoding, not a `derive(Ord)`

Ordering states by derived field order is deterministic but easy to break by
accidentally reordering a struct field or adding a field in the middle. Both
`StateKey` and `ActionKey` therefore compare *canonical encodings*
(length-prefixed fields, explicit tags, sorted collections). The same encoding
supplies equality, so a "same state" decision cannot depend on discovery order.
The reordering test (`reordering_the_plan_does_not_change_the_compiled_receipt`)
asserts that reversing the plan's node list produces an identical receipt.

## 3. Stage output coverage is the main source of state explosion

A stage node that may bind either of two candidates doubles every downstream
configuration, and independent stage nodes multiply permutations. Two stages with
two outputs each already approaches the 128-state cap; four stages exceed it and
must be `inconclusive` (`oversized_graph`). The compiler refuses rather than
truncating, so the failure mode is a lost verdict, not a false positive.

## 4. The corpus needed a "satisfiable" fraction

The first 96-case seeded corpus produced 5 verified and 98 counterexamples: the
random policy nearly always disabled some precondition, so the verified path and
its state-count comparison were barely exercised. Pinning roughly a third of the
cases to a satisfiable policy profile moved the split to 31 verified / 72
counterexamples. A differential corpus that only produces counterexamples cannot
detect an over-approximating compiler.

The state-count equality check (compiler live states versus reference live
configurations) is the strongest non-witness check in the suite: it compares two
reachable-set computations that share no code. It only applies to verified cases,
which is why the mix matters.

## 5. What is *not* established

- No real observation, collector, authentication or signature path exists; every
  initial fact is a synthetic fixture.
- No model adapter ran yet, so "model output is never evidence" is currently a
  property of the type vocabulary and the fixtures, not of an executed call.
- No latency, memory or GPU measurement is recorded here; those belong to the
  measurement step of Issue #7 and to sanitized CI artifacts.
- `cegar-core` was not modified, so nothing in this note changes the core's
  claimed scope.

## Follow-ups

1. Model adapter (bounded, local, `std`-only) and the no-model baseline comparison.
2. ~~Mutation tests that delete one precondition or one bad edge at a time and
   require the verdict to change.~~ Done: `crates/cegar-domain/tests/mutations.rs`
   holds eight single-obligation mutations, all of which flip the verdict.
3. A horizon-scoped result (`H`-bounded prefix) if the bounded-prefix question is
   answered by the owner.
