# Implementation agent handoff

## Current contribution

[Draft PR #3](https://github.com/kmosoti/CEGAR/pull/3), branch `feat/2-bounded-verifier`, contains the first dependency-free Rust finite-state checker for [issue #2](https://github.com/kmosoti/CEGAR/issues/2). It is stacked on `docs/cegar-design-agent-handoff` / draft PR #1. The owner's linked documentation branch is unchanged. A PR body reference is not claimed to establish a new branch Development association.

The core and the Rust-only xtask now build and pass Clippy, formatting, debug/release tests, and the independent Rust reference check in [CI run 42439569614](https://github.com/kmosoti/CEGAR/actions/runs/42439569614). The former Python-to-Rust test is historical, not an active tool. Read [Rust xtask validation evidence](../knowledge/evidence/0003-rust-xtask-validation.md) and [the historical original](../knowledge/evidence/0002-bounded-core-validation.md), [findings](../knowledge/research/0002-finite-checker-findings.md) and [autonomous decisions](../knowledge/decisions/0003-finite-cegar-kernel.md). These are scoped CI results, not G15 measurements or proof of a real host's safety.

## Read order

1. `AGENTS.md`, `CONTRIBUTING.md`, `README.md`.
2. `crates/cegar-core/README.md` and source/tests.
3. `docs/FORMAL_MODEL.md`, `docs/CONTRACTS.md`, `docs/SECURITY.md`.
4. Knowledge-base evidence and ADR 0003, including failed attempts and limitations.
5. Issue #2 acceptance criteria and the original phased `docs/ROADMAP.md`.

## What is implemented

Validated dense-ID finite models; action-labeled quotient reachability; concrete frontier propagation; spurious-trace refinement; concrete witness replay; bounded work/refinement and cooperative cancellation; independent Rust transitive closure and a separate Rust xtask backward-reachability oracle. Tests use synthetic data only. No runtime dependencies are required.

The initial model contract is a Rust API, not the proposed bit-width/explicit-ID JSON schema. The removed test-only process driver is not the production API. Rust xtask checks the core directly. Do not present it as the authenticated service.

## Issue #7 experiment (in progress, separate from Issue #2 review)

An experimental, non-core slice now exists for [issue #7](https://github.com/kmosoti/CEGAR/issues/7):
`crates/cegar-domain` compiles synthetic `stage → registered test → publish`
plans into a bounded first-violation monitor graph for `cegar-core`, and
`cargo run -p xtask -- domain` checks it against an independently written
reference interpreter over fixtures plus a seeded corpus. Read
[ADR 0008](../knowledge/decisions/0008-experimental-domain-compiler.md),
[research 0007](../knowledge/research/0007-domain-compiler-findings.md) and
[validation 0007](../knowledge/evidence/0007-fedora-domain-compiler.md). It has
no collector, no model adapter yet, and no execution authority; nothing here
changes the Issue #2 acceptance criteria or promotes anything to `main`.

## Next contribution: review before adding machinery

- Independently inspect the abstraction constructor, concrete feasibility logic and strict refinement progress argument.
- Add direct invariant/property tests for intermediate partitions and targeted mutations; passing differential tests up to three states is not a proof of all larger cases.
- Evaluate the public JSON schema to internal-ID mapping as a precise, bounded adapter with rejected unknown references and byte/nesting limits.
- Design the artifact workflow compiler with immutable digests and a separate concrete oracle before implementing trusted collector authority.
- Preserve explicit incomplete/invalid/inconclusive outcomes. Missing or contradictory real-world observations are not expressible as permissions in this graph core.
- Keep hard process RAM/wall-clock isolation outside the pure checker. A cancellation callback must remain bounded and cannot replace process isolation.

## Validation commands

Use the committed toolchain and lockfile:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline -- --nocapture
cargo test --workspace --release --locked --offline -- --nocapture
cargo run -p xtask --locked --offline -- verify
```

Record exact source/compiler versions and output. Raw sanitized outputs belong in the issue's linked CI artifacts; durable interpretations go in `knowledge/evidence/`. Do not silently rewrite the pin to obtain green without documenting the decision.

## Rust-only tooling requirement

The owner explicitly requires Rust-native development; do not add Python code or Python CI glue. Use `xtask` for orchestration and an independently implemented Rust reference algorithm for differential checks. See [ADR 0004](../knowledge/decisions/0004-rust-only-xtask-tooling.md) and [validation 0003](../knowledge/evidence/0003-rust-xtask-validation.md). Historical Python-era evidence remains in validation 0002 solely as provenance.

## Subsequent milestones

Typed artifact workflow and read-only trusted snapshots; bounded authenticated service and receipts; SMT only where an obligation requires it; one qualified optional d1 adapter emitting predictions, not observations. OS and GPU tuning remain deferred. No executor, arbitrary shell, broad model installation or general agent framework belongs in this core contribution.

Use a narrow issue-linked draft PR with `Refs #2` until reviewed acceptance criteria are actually complete. Do not automatically merge PR #3 or close the issue. Reverting the implementation commits is the rollback; no external infrastructure state has been mutated by the verifier.
