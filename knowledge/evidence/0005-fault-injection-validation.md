# Fault-injection follow-up validation

- Date: 2026-11-08 US Central; tracks [issue #2](https://github.com/kmosoti/CEGAR/issues/2) and [draft PR #4](https://github.com/kmosoti/CEGAR/pull/4).
- Baseline: `60d3aa3a62d50f138b4c9ceabf64d639f90ee46b`; earlier validation is preserved in [evidence 0004](0004-independent-audit-validation.md). The fault experiments and exact reproducing fixtures, including surviving mutations, are in [finding 0005](../research/0005-targeted-fault-injection.md).
- Environment: pinned `beta-2026-11-07`, `rustc 1.100.0-beta.12 (b0aaa0fca 2026-11-06)`, x86_64 Linux. Synthetic mutations were isolated and removed; no production mutation or raw terminal logs were committed. Local raw outputs were not uploaded; the finding records reproducible mutations and observations, not a fabricated artifact.

## Local gates after adding regressions

All five commands exited 0:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- `cargo test --workspace --locked --offline -- --nocapture`
- `cargo test --workspace --release --locked --offline -- --nocapture`
- `cargo run -p xtask --locked --offline -- verify`

Debug and release each passed 11 core unit, 19 integration, 2 xtask unit and 1 doc test. Xtask checked 28,868 exhaustive and 1,000 seeded cases (`0x4345474152`), independently validating 24,187 witnesses and rejecting eight invalid constructors. The exhaustive family is repeated across test modes, not additional unique models. The local validated `crates/cegar-core/src/lib.rs` SHA-256 is `bb48586f0b9960ed0853b74bcfbf032ecfa96b34610cbde4ba8ffdf1e8af778c`; integration test SHA-256 `6f127fc0b6abd498f17ddf192720f26ab9592dacf18b1088b9e07411284d91d3`; xtask source SHA-256 `1d8ac413e253b8c7c7bbb6e790b7e05db7a3604bdab0a9094130f7a2b07a2dad`.

The earlier [run 42497588971](https://github.com/kmosoti/CEGAR/actions/runs/42497588971) tested the baseline, not these follow-up source changes. Inspect PR #4's later Actions check for its precise head, merge parents and artifact before asserting CI validation of this follow-up. Local passing results do not establish real-world model completeness, authenticated receipts, OS isolation, execution authority, or formal implementation correctness.
