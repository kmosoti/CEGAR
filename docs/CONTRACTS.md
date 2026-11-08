# Proposed contracts and public API

**Status:** candidate design, not an implemented API. The JSON schemas and examples are discussion artifacts until validated by the Rust compiler and tests.

## Core types

- `Observation<T>`: typed value produced by an authorized collector, bound to source, time, exact snapshot version and authenticated collector identity.
- `Claim<T>`: value asserted by a source, with immutable source reference.
- `Prediction<T>`: model-estimated value, with model/runtime identity, question/template identity and calibration scope.
- `Unknown`: no authoritative binding, conflict, stale data, or incomplete evidence.
- `EvidenceRef`: immutable content identity plus exact location/offset conventions.
- `PolicyRef` and `ToolContractRef`: operator-controlled, versioned constraints.
- `Plan`: typed, bounded DAG of actions, resources, preconditions, effects, and modeled failures.

**Serialization is not authority.** Clients can submit proposed plans and claims but cannot mint `Observation<T>` by setting a `kind` property. That capability belongs to the authenticated collector boundary.

## Candidate plan IR

An action references a registered operation ID and version, not an arbitrary executable string. Resource arguments refer to snapshot-bound opaque IDs or explicitly authorized new-resource constructors. Node IDs are unique. Every dependency must exist. The compiler validates DAG acyclicity and data/effect typing.

Example domain operations:

```text
stage_patch(base_artifact, patch) -> candidate_artifact
run_registered_tests(candidate_artifact, suite) -> attestation
publish(candidate_artifact, attestation) -> published_artifact
```

A test attestation is trusted only when its origin, integrity, target digest, suite and policy requirements are validated. `tested_digest == published_digest` is necessary but not alone sufficient for permission.

See [proposed plan schema](../schemas/plan.proposed.schema.json) and [example](../examples/stage-test-publish.proposed.json). The schema does **not** enforce all semantic obligations; the server must do that separately.

## Result algebra

Treat outcomes as distinct variants:

```text
VerificationOutcome =
    VerifiedWithinModel(receipt)
  | ConcreteCounterexample(trace, violated_rules)
  | NeedsEvidence(missing_refs, questions)
  | InvalidInput(diagnostics)
  | StaleSnapshot(refs)
  | Inconclusive(reason, consumed_budget)
  | Unavailable(reason)
```

Never collapse all variants into `approved: true|false` or a score. `VerifiedWithinModel` must include `execution_authorized: false` in v0. No HTTP response from CEGAR creates a permission to execute a tool.

Response examples and a draft schema are in [verification result schema](../schemas/verification-result.proposed.schema.json).

## Initial API proposal

| Endpoint | Purpose | Authority |
| --- | --- | --- |
| `GET /healthz` | Service liveness | No proof of model readiness |
| `GET /v1/capabilities` | Qualified implemented features and limits | Do not advertise candidate features as ready |
| `POST /v1/snapshots` | Scoped read-only snapshot request | Collector assigns observation authority |
| `POST /v1/plans` | Submit a typed plan proposal | No execution |
| `POST /v1/verifications` | Resolve trusted snapshot/policy and verify a plan | Scoped within-model result |
| `GET /v1/verifications/{id}` | Retrieve result/receipt | Caller-scoped |
| `POST /v1/verifications/{id}/cancel` | Cancel job and reclaim budget | No state mutation outside service |
| `POST /v1/assess` | Optional fixed-template neural decision | Predictions only |
| `POST /v1/context-packages` | Protected evidence and optional background | Never confers truth or export permission |

An intentionally separate experimental endpoint may check **client-supplied finite graphs**, but results must be labeled `supplied_finite_model_only` and cannot be confused with trusted workspace verification.

**Explicitly absent from v0:** `/execute`, arbitrary URL fetching, arbitrary user-supplied model paths, shell actions, file mutation and open model-selection endpoints.

## Span and evidence conventions

Model span indices may be Unicode character offsets; Rust commonly uses UTF-8 byte offsets. Preserve the exact serialization and build a tested character-to-byte mapping. Verify any extracted substring equals the referenced source slice. A copied span proves occurrence, not intent or truth.

Token compression cannot remove binding evidence or silently change a policy statement. Protected whole records are passed unchanged or through an independently validated structured representation.

## Identity, freshness and versioning

Snapshots and policy/tool-contract references are immutable and versioned. A verification job resolves them once and records their exact identities. Re-running with changed inputs produces a new result. Receipts include validity/freshness conditions and must not be replayed as runtime authorization.

Major contract changes require migration notes, synthetic fixtures and semantic tests. Document which results an older client may fail to understand and reject unknown result variants safely.

## Failure and resource semantics

Requests have bounded size, graph nodes, evidence count, worker time, queue depth and memory. API authentication is mandatory for private ingress. Missing data produces `needs_evidence`; a stale source produces `stale_snapshot`; exhausted budgets produce `inconclusive` or `unavailable`. Do not hide these as `verified`.

The client may cancel a job. Releasing a worker lease must occur only after termination/cancellation is accounted for.

## Observable operations

Include trace/request ID, sanitized operation class, stage durations, queue wait, worker failure counts, and result category. Avoid logging raw sensitive prompts or secrets. Store full evidence only under an explicit retention and redaction policy.

## Compatibility with local model servers

The optional d1-2B adapter may call mistral.rs `POST /v1/systemone` with fixed state serialization and named questions. This is an upstream inference API, **not CEGAR's verification endpoint**. Store its output as `Prediction` or checked `Claim` only.

Separate the neural worker failure domain from the service's deterministic core.

