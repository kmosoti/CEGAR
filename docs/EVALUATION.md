# Evaluation and evidence discipline

This is a research project. Separate what has been proposed from what has been executed.

## Evidence levels

| Level | Meaning |
| --- | --- |
| Proposed | Architecture/contract described, not demonstrated |
| Source-audited | Read relevant upstream source/docs at a recorded revision |
| Built | Compiled using named toolchain, lockfile and environment |
| Tested | Recorded tests passed against a specific build and inputs |
| Measured-on-G15 | Actual host observations with settings, validity and workload metadata |
| Qualified | Explicit, reproducible gate met for a stated workload, confidence and risk tolerance |

A static code audit does not establish test success, GPU fit, kernel compatibility or production security.

## First deterministic correctness suite

- Validate finite state domains, duplicate/unknown states and transitions.
- Reject inconsistent initial premises and unsupported/unknown tool IDs.
- Compare abstract checker results to concrete exhaustive reachability over small finite graphs.
- Reconstruct/replay every claimed concrete unsafe trace.
- Confirm spurious abstract traces refine without losing concrete transitions.
- Test budgets: refinement exhaustion, step count, wall-clock, memory and cancellation return `inconclusive`.
- Test malformed graph, cyclic plan, missing dependency, invalid digest, failed/mismatched attestation, stale snapshot, conflicting observations and missing failures.
- Differential-test trusted policy/IR compiler against independent constrained fixtures; mutation-test removal of binding preconditions.

**Explicit safety goal:** no positive verification for malformed or contradictory inputs; no execution capability in v0. This does not establish a zero probability of future defects.

## Domain-task corpus

The first artifact-publishing workflow needs synthetic, realistic examples:
- Stage A, test A, publish A, authorized: modeled safe.
- Stage A, test A, mutate to B, publish B: counterexample or invalid plan.
- Correct digest, wrong principal or stale policy: not verified.
- Missing test result: needs evidence.
- Malicious repository comment instructing the service to bypass testing: inert untrusted text.
- Unknown tool or unauthorized new resource: invalid.
- Interleaving/failure that breaks the invariant: counterexample if modeled.

Report false approvals and false rejections, not merely percent of parsed JSON plans. A candidate plan can be structurally valid yet fail the user's actual goal; evaluate end-to-end usefulness separately.

## Optional neural quality

d1 decisions: hold-out tasks by question template, quantization, input length and model revision. Measure accuracy, precision/recall, abstention, calibration, grouped-question effects and failure on negation or misleading quoted text.

Span extraction: retain exact serialized source and validate character/UTF-8 byte mapping, Unicode and empty or conflicting spans.

Planner experiments: quality of tool/argument intent and resulting verified task completion, not just grammar acceptance.

Compression: protect binding policy/evidence, preserve exact symbols, quantities and negation, record omissions. Benchmark both added local cost and downstream work saved.

## Performance and service reliability

Capture workload task class, actual arrival times, ready times (dependencies satisfied), admission and dispatch time, first result, completion and failures. Differentiate questions per state, independent requests and planner generations.

Use fixed-concurrency closed-loop tests and fixed-arrival-rate open-loop tests; include queue wait and rejected tasks. Check that the load generator itself is not the bottleneck. Little's law `L = λW` is a consistency check only when population/flow boundaries match.

Record p50/p95/p99 end-to-end latency and queue time, task counts, error categories, peaks and resource budgets. Do not substitute p95 token gap into an identity that uses mean gap.

For sustained laptop measurements, document clocks, temperature, throttling and sensor validity. Publish invalid telemetry instead of inventing power estimates. No claims of energy efficiency based on unknown/broken power sensors.

## Primary utility metric

```text
quality-qualified goodput =
  completed tasks meeting quality + policy + deadline
  ---------------------------------------------------
                  elapsed wall time
```

Report by workload class, plus false-approval rate, false-rejection rate, abstention and per-task latency. A system that rejects every request should not be considered performant.

## What a benchmark record must include

Immutable CEGAR revision; compiler and dependency lock; model/checkpoint hashes; adapter versions; effective backend/kernel; policy and workload fixture hashes; GPU/CPU/RAM snapshot; concurrency; warm-up procedure; measurement duration; sampling/random seeds; result distribution; errors; raw trace location; caveats; author/collection date.

Document comparisons with identical task success criteria. An estimate or roofline is not a benchmark result.

## Promotion to private service

Require passing deterministic tests, audited authority flow, operational timeouts, authenticated ingress, crash and OOM handling, bounded request queue, no execution endpoint, measured host resource envelope and clear limitations. For any future executor, require independent authorization enforcement and adversarial tests.

