## Tracking issue

Refs #<!-- issue number; use Closes #N only when all issue acceptance criteria are complete -->

## Summary

What changed and why? Which architectural/authority boundary is affected?

## Scope and assumptions

What is deliberately out of scope? Which assumptions, snapshots, or model revisions matter?

## Evidence

- Evidence/benchmark artifacts are sanitized and attached or linked on the tracking issue; versioned synthetic fixtures remain in Git.
- Inspected local change and feature bookmark with JJ; did not move `main` or force-push shared history.


- Commands executed and results (or explicit reason they were not run):
- Tests / fixtures / benchmarks:
- Model or upstream source revisions:
- Hardware environment, if relevant:

## Security and trust impact

- Does this affect trusted observation, policy, verification soundness, or execution authority?
- Could unknown, stale, or malformed inputs be accidentally approved?
- Are all examples synthetic and free of secrets or workplace content?

## Checklist

- [ ] Docs, schemas and tests updated as needed
- [ ] No unverified claims of runtime success or safety
- [ ] Inconclusive/unknown outcomes handled explicitly
- [ ] Public data, model licensing and dependency provenance reviewed
- [ ] New privileged capability or source of authority discussed in an ADR

