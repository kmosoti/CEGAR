# Fedora domain-compiler validation record

- Date: 2026-11-08 (US Central)
- Tracking: [Issue #7](https://github.com/kmosoti/CEGAR/issues/7)
- Change: new pure `cegar-domain` crate (experimental stage → registered test →
  publish compiler) plus an independent reference check in `xtask`, with the CI
  gate extended to run it
- **Status: Built and tested on the Fedora workstation. Not reviewed, not
  qualified, and not a G15 result. No model adapter has been executed yet.**

## Step A — actual Fedora environment (measured, sanitized)

Discovered on the developer workstation, not inferred from older notes:

| Fact | Observed value |
| --- | --- |
| OS | Fedora Linux 44 (Workstation Edition), stable |
| CPU | Intel Core i7-10750H, 6 cores / 12 threads |
| RAM | 31 GiB total, 11 GiB available at the time of the run |
| GPU | NVIDIA GeForce RTX 3060 Laptop GPU, 6144 MiB VRAM, driver 615.78.08 |
| Rust | `rustc 1.100.0-beta.12 (b0aaa0fca 2026-11-06)`; `cargo 1.100.0-beta.12`; pinned toolchain `beta-2026-11-07` |
| JJ / Git | `jj 0.46.0`, colocated Git checkout |
| GitHub CLI | `gh 2.97.0` |
| Existing local inference | `local-inference` checkout present; retrieval gateway listening on `127.0.0.1:1235`; generation gateway (mistral.rs behind a private namespace) not started at inspection time |

The existing local inference stack was inspected read-only. No host service was
started, reconfigured or installed, and no model was downloaded for this step.

## Step B — deterministic Rust slice

Baseline: integration tip `33424250f926a173396e8e95ee638f6be7c6cf94`
(`feat/2-bounded-verifier`, after PR #6). `cegar-core` is unchanged; the new crate
consumes it.

Local run identity: the Fedora working-copy tree that was published on
`issue/7-domain-compiler`. CI identity for that branch head (domain
compiler, adapter, mutation tests and the first evidence revision) is
[run 42522760646](https://github.com/kmosoti/CEGAR/actions/runs/42522760646) on
`e82325c11ad780a83cd892262e174879b1af7347`, which passed every gate above —
including the new *Independent domain reference check* step:
`{"status":"passed","oracle":"rust_domain_reference_dfs","cases":103,"verified":31,"counterexamples":72,"skipped_limits":0,"seed":19227491642723889}`.
Earlier heads also passed: `e8c5988` ([run 42522270328](https://github.com/kmosoti/CEGAR/actions/runs/42522270328)),
`22974def` ([run 42522618744](https://github.com/kmosoti/CEGAR/actions/runs/42522618744)) and
`505d602d` ([run 42522675527](https://github.com/kmosoti/CEGAR/actions/runs/42522675527)); the
last two were superseded by commits that also carry the mutation tests and the CI
identity text, so those runs are provenance for earlier heads, not for this one. Local results are not CI, and CI is not Fedora model execution; the
local-model adapter path is covered by [validation 0008](0008-fedora-local-model-probe.md).

### Gates executed

All commands were run from the repository root with the committed lockfile and
the pinned toolchain, offline.

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | pass |
| `cargo test --workspace --locked --offline` | pass — 11 `cegar-core` unit, 19 `cegar-core` integration, 14 `cegar-domain` unit, 17 `cegar-domain` integration, 8 `cegar-domain` mutation, 20 `cegar-local-model` unit, 6 adapter integration, 2 `xtask` unit, 1 doc test (98 total) |
| `cargo test --workspace --release --locked --offline` | pass — same suite |
| `cargo run -p xtask --locked --offline -- verify` | pass — unchanged finite-graph check: 28,868 exhaustive models, 1,000 seeded models, 24,187 witnesses replayed, 8 malformed models rejected |
| `cargo run -p xtask --locked --offline -- domain` | pass — `{"status":"passed","oracle":"rust_domain_reference_dfs","cases":103,"verified":31,"counterexamples":72,"skipped_limits":0,"seed":19227491642723889}` |

The `domain` check is the important new evidence: for 7 hand-written fixtures and
96 deterministically seeded cases it compares the compiled graph's verdict
against a separately written depth-first reference interpreter, requires state
counts to agree on verified cases, and replays each compiler witness through the
reference's own step function with exact rule-set equality.

### Behavioral coverage in the suite

Verified: the safe chain; the ambiguous stage-output case under a permitting
policy; cross-principal use under an explicit sharing rule.

Modeled counterexamples: subject mismatch; ambiguous stage output (second
binding); required-suite mismatch; untrusted issuer on a modeled pass;
cross-principal use without a rule; two independently ready unsafe publishes
(both represented, distinct rule sets).

Gate classifications: stale snapshot; needs-evidence; contradictory premises;
cyclic plan; unknown stage outputs; four-stage graph exceeding the 128-state
bound.

Mutation tests (8): each removes exactly one modeled obligation, branch or input
and requires the verdict to change — retargeting the publish subject, narrowing
the stage-output set, aligning the required suite, trusting the modeled issuer,
permitting sharing, supplying the absent observation, staging the missing
candidate, and invalidating a modeled pass's tenant.

Also asserted: reordering the plan does not change the receipt; identical inputs
repeat identically; no analysis reports execution authority.

### Failures observed during development

1. The first differential run failed and exposed a genuine ambiguity: the
   compiler skipped the evidence-presence check when a publish subject was not
   staged, while the reference reported both `attempt.subject_not_staged` and
   `premise.evidence_missing`. Resolved by defining the subject comparison as
   undefined without a staged candidate, while still checking record presence.
   See [research 0007](../research/0007-domain-compiler-findings.md).
2. Two local test expectations were wrong about ordering/encoding details
   (`Rule` order follows codes, and `Digest` encoding is length-prefixed); the
   tests were corrected, not the encoding.
3. The first seeded corpus was 95 % counterexamples, so the verified branch and
   its state-count comparison were barely exercised; roughly a third of the
   corpus is now pinned to a satisfiable profile.

## What this record does **not** claim

- No model adapter, no inference call, no latency, RAM/VRAM or queue measurement
  is recorded here. Those belong to the remaining Issue #7 steps and will be
  added only after they actually run.
- No collector, authentication, signature or time authority exists. Every initial
  fact is a synthetic fixture and each receipt says `synthetic_fixture` and
  `execution_authorized = false`.
- Passing locally is not CI, and CI passing on hosted Linux would not be Fedora
  model execution.

## Reproduction

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline -- --nocapture
cargo test --workspace --release --locked --offline -- --nocapture
cargo run -p xtask --locked --offline -- verify
cargo run -p xtask --locked --offline -- domain
```
