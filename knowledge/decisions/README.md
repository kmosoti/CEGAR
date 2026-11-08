# Architecture decisions

Record significant choices about authority, verification semantics, adapters, safety and public contracts as ADRs. A decision implemented by an automated contributor is not automatically reviewed or approved by the owner.

- [ADR 0001: Documentation-first](../../docs/decisions/0001-documentation-first.md)
- [ADR 0002: Bounded Sans-I/O core](0002-bounded-sans-io-core.md)
- [ADR 0003: Finite CEGAR kernel (historical testing choice superseded)](0003-finite-cegar-kernel.md)
- [ADR 0004: Owner-directed Rust-only xtask verification](0004-rust-only-xtask-tooling.md)
- [ADR 0005: Raw-fixture oracle independence](0005-raw-fixture-reference.md), audit work reviewed in PR #4
- [ADR 0006: Development-first integration workflow](0006-development-branch-integration.md), owner-directed
- [ADR 0007: Proposed artifact domain semantics](0007-artifact-domain-semantics.md), owner review pending

Include status, date, decision origin, evidence, alternatives, consequences and reversal criteria. Link executed tests separately from mathematical arguments and source audits. Never present a proposed ADR as proof of implementation correctness.
