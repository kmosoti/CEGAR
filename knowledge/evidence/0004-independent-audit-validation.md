# Independent audit: local validation and CI handoff

- Date: 2026-11-08 US Central. Tracks [issue #2](https://github.com/kmosoti/CEGAR/issues/2) and [draft PR #3](https://github.com/kmosoti/CEGAR/pull/3).
- Base revision: `8890fb03ce48ee96301df4e009818c6707e742c4`, stacked on design head `6c55b715a5da21ea9600e935666fe688ef21ef95`. Local results below are for the audit working tree with the source hashes listed here; they are not a claim that a later commit passed CI.
- Host compiler: pinned `beta-2026-11-07`, `rustc 1.100.0-beta.12 (b0aaa0fca 2026-11-06)`, Cargo `1.100.0-beta.12 (3acc79c0c 2026-11-05)`, x86_64 Linux, LLVM 23.1.5. No G15 measurements.

## Locally executed gates

All five commands completed with exit 0 after the remediation below:

| Command | Observed result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | pass |
| `cargo test --workspace --locked --offline -- --nocapture` | pass: 4 core unit, 19 integration, 2 xtask unit, 1 doc test |
| `cargo test --workspace --release --locked --offline -- --nocapture` | pass: same counts |
| `cargo run -p xtask --locked --offline -- verify` | pass: 28,868 exhaustive, 1,000 seeded, 24,187 independently checked witnesses, 8 invalid constructors |

Xtask seed `0x4345474152` (decimal `288925106514`); integration seed `0x43454741525f3031`. Debug, release and xtask exhaustive tests repeat the same one-label family, not additional unique models. Seeded graphs have at most one edge per ordered pair; targeted integration tests include duplicate edges and multiple labels on identical endpoints. The new direct tests cover contiguous nonempty four-state partition assignments and observable isolated cross-block edge projection at actions 0 and `u32::MAX`, same-block zero-step reachability, set overlap, two kinds of spurious frontier and an arbitrary-partition word-survival reproducer. They do not observe self-block edge labels because BFS immediately finds the initial bad block. Mutation-style checks directly challenge the reference fixture logic; they are not a comprehensive production mutation campaign.

## Failures and remediation

The initial local `rustc -Vv` encountered rustup recovery of a partially installed dated toolchain and failed to rename several rollback component directories. A subsequent separate `rustc -Vv` succeeded and confirmed the exact compiler above; the final gates used that pin. An initial debug test build failed with Rust error E0689 for an ambiguous test variable used with `pow`; specifying `usize` corrected the test, then the gates above passed. No validation gate was disabled or relaxed. The initial JJ push failed because the HTTPS Git remote had no configured credential helper; it did not update the remote. The authenticated GitHub CLI was available, so a one-shot credential helper can be supplied to the Git process without changing global configuration or publishing credentials.

## Source identities and evidence limits

SHA-256 values at validation:

- `crates/cegar-core/src/lib.rs`: `d17edab50b83612d291fd3a805492d907825bc07fe2edaf3ecc0a362d17cd237`
- `crates/cegar-core/tests/verification.rs`: `6f127fc0b6abd498f17ddf192720f26ab9592dacf18b1088b9e07411284d91d3`
- `xtask/src/main.rs`: `1d8ac413e253b8c7c7bbb6e790b7e05db7a3604bdab0a9094130f7a2b07a2dad`
- `Cargo.toml`: `cef147af2876d6157b00e4b6dbc297ea227abcd1e4a992cca1ba9632a1098eee`
- `Cargo.lock`: `a97e795b474f61026c5663161908e49b5c5ea24aa6fbe3a0b397e7335c6a33c0`
- `rust-toolchain.toml`: `1d2d37f7eec5adc61d50344980093595fb4cfa12847c75777d5261de63096e59`

## GitHub Actions evidence

[Run 42497489395](https://github.com/kmosoti/CEGAR/actions/runs/42497489395) passed all five gates for [draft PR #4](https://github.com/kmosoti/CEGAR/pull/4) source head `43c68768287a452ce4fa1cbf54966bdb0d6b3948` and synthetic merge `d7c4fe691757629a8630334988b2d237091d998b`. GitHub identified its merge parents as base `8890fb03ce48ee96301df4e009818c6707e742c4` and that source head. The artifact's `tested-commit.txt`, compiler identity, source hashes, debug/release test counts and xtask JSON were inspected and matched the local source/results above. This documentation-only record is a later commit and requires its own run, not an implicit claim that the earlier run tested it.

[Sanitized Actions artifact 13623753990](https://github.com/kmosoti/CEGAR/actions/runs/42497489395/artifacts/13623753990): ZIP SHA-256 `f66e6f1bc6130828e0274f187903a6bcff6ea40cacf2116e6e3c27dc9ccde443`, expires 2026-12-08 15:58:54 UTC. GitHub warned the pinned checkout/upload actions target deprecated Node 20 and were forced onto Node 24; this is not a failed test. Prior [run 42439687284](https://github.com/kmosoti/CEGAR/actions/runs/42439687284) tested only base head `8890fb03` and is historical for the audit. No local raw logs are committed; Actions artifacts are time-limited.

Test success is evidence about supplied finite graphs, not a machine-checked proof, faithful domain encoding, authorized execution or proof of hard process isolation. A false-safe model, lost projected state/edge, wrong witness or non-conservative exhaustion falsifies the relevant conclusion.
