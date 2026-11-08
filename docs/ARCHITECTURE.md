# Architecture and system boundaries

Status: **proposed design**. The initial repository does not contain a qualified running service.

## Objective

CEGAR reviews **bounded candidate tool plans** against a trusted snapshot, explicit tool contracts, and versioned policy. It can report an invalid plan, a concrete counterexample, missing evidence, bounded inconclusive result, or a positive **within-model** verification. It does not infer that a plan achieves every unstated user objective.

## Design constraints

- Rust-first; pure, deterministic core isolated from I/O.
- No execution endpoint in v0.
- Optional neural interpretation, not neural authorization.
- Explicit, inspectable trust and evidence lineage.
- One independently qualified model worker at most in the initial constrained-hardware profile.
- OS-independent interfaces; defer service-manager, CUDA, network and host-specific tuning.
- Separate *verification* from *authorization* and *execution*.

## Conceptual data flow

```text
                     operator policy + tool contracts
                                      |
 authenticated request       trusted snapshot collector
           |                          |
           v                          v
      API validation --------> typed IR compiler
           |                          ^
           +-> optional extractor ----|  [Predictions are suggestions only]
                    |
              candidate plan
                    |
          exact reference/DAG checks
                    |
        bounded abstract-state checker
                    |
          trace-feasibility analysis
               /          \
        spurious          real
           |               |
    refine abstraction   counterexample
           |               |
        repeat            review output
           |
     no abstract bad trace
           |
   within-model verification receipt
           |
      NO EXECUTION IN V0
```

The outer repair loop takes a real counterexample and asks a planner, template, or human to **propose a new plan**, then repeats full verification. This is not the inner CEGAR loop.

## Components

| Component | Owns | Trust boundary |
| --- | --- | --- |
| `cegar-core` | Evidence types, immutable plan IR, abstract/concrete transitions, obligations | Must not perform I/O |
| HTTP shell | Authentication integration, request IDs, limits, job lifecycle | Public inputs remain untrusted |
| Snapshot collector | Read-only, scoped inventory and revision-bound observations | Only this boundary can issue authoritative observations |
| Plan compiler | Typed IDs, validated capabilities, exact effect semantics | Reject unknown operations, missing edges, contradictory premises |
| Verifier worker | Bounded reachability/SMT and trace replay | Isolated from HTTP process and execution credentials |
| Neural decision adapter | d1 or alternative extractor/classifier | May return `Prediction`/`Claim`, not policy facts |
| Context packager | Protected evidence, optional background compression and provenance | Never drops binding policy or source identities |
| Receipt store | Immutable result lineage, versioned schema, freshness | Stored results are not live authorization |
| Optional future executor | Enforces fresh capabilities and live preconditions | Out of scope until separate security/design review |

## Trusted computing base

The collector, policy/tool-contract registry, parser, IR compiler, abstraction construction, concrete trace checker, verifier, and receipt binder are safety-critical. Their correctness is not established by the solver alone.

A model response cannot create a signed collector observation by setting a JSON field. Treat all user, repository, retrieved-document, and model text as untrusted claims.

## First supported workflow

The initial domain is immutable artifact publishing:

```text
stage_patch(base_digest, patch_digest) -> candidate_digest
run_registered_tests(candidate_digest) -> test_attestation
publish(candidate_digest, test_attestation) -> published_digest
```

Required relation: `attestation.tested_digest == published_digest`. The attestation must refer to a registered test suite, pass status, trusted origin, and compatible policy/snapshot freshness. Testing A does not permit publishing B. Plan ordering is a directed acyclic graph; verify edges with a plain graph algorithm before SMT.

No arbitrary tool name, shell command, filesystem path, or repository mutation is inferred from model text.

## Service lifecycle

A request is admitted under a bounded budget, resolves immutable inputs, validates, compiles, checks, and produces a typed result. The service must distinguish `invalid_input`, `needs_evidence`, `stale_snapshot`, `counterexample`, `verified_within_model`, `inconclusive`, and `unavailable`.

No retry can quietly change the snapshot or policy while keeping the old result identity. A model worker failure does not disable deterministic verification. An SMT timeout returns `inconclusive`.

## Scaling and model isolation

Keep a bounded queue outside neural and solver workers. Separate interactive and background admission budgets without claiming hard real-time preemption. A model worker may crash/OOM independently of the API and core; its crash should not turn unknown into allow.

A heavy model switches only when expected task benefit justifies loading, warm-up, lost caches, and resource interference. Batch questions about one state separately from batching independent requests. The G15 configuration is a candidate capacity constraint, not a proven benchmark.

## Success criteria

A useful CEGAR service must improve reviewed-plan correctness and interpretability without taking authority away from deterministic policy or requiring a GPU for the safety path. Evaluate false approvals, false rejections, abstention, missing evidence, verification latency, quality-qualified goodput and trace usefulness. See EVALUATION.md.

