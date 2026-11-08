# cegar-domain (experimental)

**Prototype, not a service, an API contract or a policy decision.** This crate is
the deterministic trust-critical half of the [Issue #7](https://github.com/kmosoti/CEGAR/issues/7)
Fedora-first experiment. It implements the *proposed* semantics of
[Artifact Domain Semantics v0.1](../../docs/ARTIFACT_DOMAIN_SEMANTICS.md) for one
narrow slice: `stage_patch → run_registered_tests → publish`.

It is Sans-I/O: no clock, no network, no filesystem, no ambient capability. It
compiles a synthetic plan plus a pinned context into a bounded first-violation
monitor graph and hands that graph to [`cegar-core`](../cegar-core/README.md).
It never returns execution authority, and every analysis result carries
`execution_authorized = false`.

## What it does

```
Plan (typed nodes, DAG, disposition) ─┐
Pinned Context (policy, registries,   ├─► analyze() ─► Analysis + Receipt
snapshot facts, fixture evidence)     ┘        │
                                               └─► cegar-core Model → verify()
```

1. **Structural validation** (`plan`): unique nodes, resolvable dependencies,
   acyclic DAG, typed subject/evidence producers, node bound.
2. **Premise validation** (`context`): contradictory authoritative records,
   unregistered suites, known-stale facts.
3. **Resolution** (`compile::resolve`): registry membership, exact input
   presence, producer dependency paths, declared stage-output assumptions.
4. **Enumeration** (`compile::explore`): every ready node in every order, every
   modeled outcome and bound output identity, blocked-closure after each
   non-violating step, and an explicit terminal `Bad` edge for every ready
   attempt with a false precondition.
5. **Canonicalization**: states and action labels are sorted by their canonical
   encodings before dense renumbering, so results do not depend on discovery
   order or plan ordering.
6. **Core verification and replay**: the core's verdict is not trusted on its
   own; witnesses are decoded back to action labels and re-checked.

Classification order is `invalid_input` → `stale_snapshot` → `needs_evidence` →
`inconclusive` → `concrete_counterexample` / `verified_within_model`.

## Invariants

- An unsafe ready attempt is never pruned. It becomes a labeled edge to the
  terminal `Bad` monitor with the violated rules attached.
- A hypothetical modeled test pass is not an observation. It is labeled as
  hypothetical, and the same scoped validity predicate is applied to it as to a
  real record.
- Unknown is never true: absent evidence is `needs_evidence`; missing behavior
  or coverage is `inconclusive`; neither is reported as verified.
- Exceeding the 128-state / 512-transition bound is `inconclusive`, never a
  truncated positive result.
- Cross-principal evidence use requires an explicit policy rule *and* the same
  tenant/audience/destination scope checks.
- When a publish subject is not staged, the attested-subject comparison is
  undefined; only `structure`/presence rules are reported for that attempt, and
  the attempt is still modeled as unsafe.

## Limits of this slice

- Initial facts are synthetic fixtures (`Provenance::SyntheticFixture`). There is
  no collector, no authentication route, no signature validation and no time
  authority. A receipt says so.
- Stage success bindings come from an explicit pinned assumption; a plan's
  declared identity is not evidence that it is the only permitted output.
- Only the outcomes listed above exist. Partial effects, retries, cancellation
  races, real concurrency and revocation during an action are not modeled, so a
  request that needs them is inconclusive.
- Node count is capped at 6 for the experiment.
- `verified_within_model` means "no reachable first violation in this specified
  graph" — not real-world fidelity, not liveness, not authorization.

## Validation

```sh
cargo test -p cegar-domain --locked --offline
cargo run -p xtask --locked --offline -- domain   # differential check vs the independent reference
```

The differential check compares this compiler against a separately written
depth-first reference interpreter over the same raw fixtures and a seeded
corpus, and independently replays every compiler witness
([`xtask/src/domain_oracle.rs`](../../xtask/src/domain_oracle.rs)). It found a
real semantic ambiguity during development: see
[research note 0007](../../knowledge/research/0007-domain-compiler-findings.md).
