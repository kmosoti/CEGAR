# Knowledge and investigation index

Curated, dated research findings and architectural decisions for CEGAR. This is **not a place for raw logs, dumps, model weights or private host snapshots**.

## What belongs here

- `research/`: verifiable claims, pinned upstream revisions, observations, hypotheses, alternatives and unresolved questions.
- `evidence/`: durable metadata and interpretation for sanitized issue-linked CI artifacts or attachments. Raw evidence stays outside Git.
- `decisions/`: ADRs for architecture, trust boundaries and implementation tradeoffs, with provenance and review status.

Canonical design sources remain [the research audit](../docs/RESEARCH_AUDIT.md), [sources](../docs/SOURCES.md), [formal model](../docs/FORMAL_MODEL.md) and [ADR 0001](../docs/decisions/0001-documentation-first.md).

## Implementation and investigation tracking

- [Issue #2: finite-state core](https://github.com/kmosoti/CEGAR/issues/2)
- [Draft PR #1: architecture](https://github.com/kmosoti/CEGAR/pull/1)
- [Draft PR #3: bounded Rust implementation and tests](https://github.com/kmosoti/CEGAR/pull/3), stacked on the design branch
- [Source-audited component boundaries](research/0001-component-boundaries.md)
- [Finite-checker implementation findings](research/0002-finite-checker-findings.md)
- [Built/tested evidence, exact CI runs and failed attempts](evidence/0002-bounded-core-validation.md)
- [ADR 0002: pragmatic Sans-I/O preference](decisions/0002-bounded-sans-io-core.md)
- [ADR 0003: historical finite-kernel decisions](decisions/0003-finite-cegar-kernel.md)
- [ADR 0004: owner-directed Rust-only xtask validation](decisions/0004-rust-only-xtask-tooling.md)
- [ADR 0005: raw-fixture reference checks](decisions/0005-raw-fixture-reference.md)
- [ADR 0006: development-first integration and separate main promotion](decisions/0006-development-branch-integration.md)
- [Rust-only oracle research and limitations](research/0003-rust-reference-verification.md)
- [Rust xtask validation status](evidence/0003-rust-xtask-validation.md)
- [Independent soundness audit and refinement-word distinction](research/0004-independent-soundness-audit.md)
- [Independent audit validation](evidence/0004-independent-audit-validation.md)
- [Targeted fault-injection findings](research/0005-targeted-fault-injection.md)
- [Fault-injection validation](evidence/0005-fault-injection-validation.md)

The core is **built and tested in CI**, not deployed, G15-measured or production-qualified. Autonomous implementation decisions remain subject to human review. A positive checker result applies only to the supplied finite model, never execution authority.

## Evidence discipline

Each note should identify source/revision, observation command/date, claim type, scope, excluded conditions, falsification tests, and a verified artifact reference when evidence exists. State failed attempts and limitations as well as successes. Record artifact expiry and do not mistake a temporary download link for permanent provenance.

No invented citations, test results, attachment success or claim that a model's interpretation is a trusted observation. If evidence cannot be safely published, describe the gap rather than copying sensitive data.
