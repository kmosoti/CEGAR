# Knowledge and investigation index

Curated, dated research findings and architectural decisions for CEGAR. This is **not a place for raw logs, dumps, model weights or private host snapshots**.

## What belongs here

- `research/`: verifiable claims, pinned upstream source revisions, assumptions, competing hypotheses, and explicit unanswered questions.
- `evidence/`: durable metadata and interpretation for sanitized issue-attached logs/traces or retained CI artifacts. Raw evidence should reside with the tracking issue or run artifact, **not in Git**.
- `decisions/`: ADRs for significant architecture, trust-boundary or implementation tradeoffs.

Existing source documents remain canonical at [docs/RESEARCH_AUDIT.md](../docs/RESEARCH_AUDIT.md), [docs/SOURCES.md](../docs/SOURCES.md), [docs/FORMAL_MODEL.md](../docs/FORMAL_MODEL.md), and [docs/decisions/0001-documentation-first.md](../docs/decisions/0001-documentation-first.md). Avoid duplicating their full contents without need.

## Current evidence / implementation tracking

- [Issue #2: Rust finite-state verification core](https://github.com/kmosoti/CEGAR/issues/2)
- [Draft PR #1: architecture and handoff](https://github.com/kmosoti/CEGAR/pull/1)
- [CEGAR validated component boundaries](research/0001-component-boundaries.md)
- [ADR 0002: bounded Sans-I/O core and external adapters](decisions/0002-bounded-sans-io-core.md)

## Evidence discipline

Prefer statements with:
1. **Source/provenance:** authoritative URL/revision, observation command, and date.
2. **Claim type:** documented fact, deduction, hypothesis, estimate, or experiment.
3. **Scope:** hardware, model, workflow, and excluded conditions.
4. **Falsifier:** a test or counterexample that would change the conclusion.
5. **Artifact reference:** sanitized GitHub issue attachment or CI artifact if collected.

Never fabricate citations, test output, or stable links to not-yet-uploaded artifacts. If evidence cannot be safely published, document the gap without copying private data.
