# Artifact Domain Semantics v0.1: validation record

- Date: 2026-11-08 US Central; tracks [issue #5](https://github.com/kmosoti/CEGAR/issues/5).
- Scope: proposed design and synthetic semantic cases. No artifact compiler, authenticated collector, HTTP service, executor or production schema change is implemented or tested by this design.
- Source baseline: development head `3abfcdac81ff65349cea232af10e3aa8f1f0b0cd`, with documentation-only work in progress. Reproduction uses the checked-in pin `beta-2026-11-07`; observed local `rustc 1.100.0-beta.12 (b0aaa0fca 2026-11-06)`, Cargo `1.100.0-beta.12 (3acc79c0c 2026-11-05)`, x86_64 Linux.

## Local Rust regression gates

After independent design review, all of the following commands completed with exit 0 on the documentation worktree against the unchanged Rust sources and lockfile:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline -- --nocapture
cargo test --workspace --release --locked --offline -- --nocapture
cargo run -p xtask --locked --offline -- verify
```

Debug and release each passed 11 core unit, 19 integration, 2 xtask unit and 1 doc test. The xtask backward reference compared 28,868 exhaustive and 1,000 seeded models (`seed=0x4345474152`), independently checked 24,187 witnesses and rejected eight malformed constructors. Repeated suites are not additional distinct exhaustive models.

The tested source hashes were: core `bb48586f0b9960ed0853b74bcfbf032ecfa96b34610cbde4ba8ffdf1e8af778c`; core integration tests `6f127fc0b6abd498f17ddf192720f26ab9592dacf18b1088b9e07411284d91d3`; xtask `1d8ac413e253b8c7c7bbb6e790b7e05db7a3604bdab0a9094130f7a2b07a2dad`; workspace manifest `cef147af2876d6157b00e4b6dbc297ea227abcd1e4a992cca1ba9632a1098eee`; lockfile `a97e795b474f61026c5663161908e49b5c5ea24aa6fbe3a0b397e7335c6a33c0`; toolchain pin `1d2d37f7eec5adc61d50344980093595fb4cfa12847c75777d5261de63096e59`.

These are finite graph regression results, not tests of the proposed domain contract or of future provenance. Raw outputs are not committed. The synthetic domain cases are expected-outcome proposals pending human review and a later independently implemented Rust domain oracle.

## GitHub Actions provenance

[Run 42512383739](https://github.com/kmosoti/CEGAR/actions/runs/42512383739) passed all five gates for [draft PR #6](https://github.com/kmosoti/CEGAR/pull/6) source head `554001b65a064b569b3e85fdd2cc4ce495a4195d` and synthetic merge `1a3a3bd7d56df6d1eff8a754bb4805d485d4f121`, whose parents are development baseline `3abfcdac81ff65349cea232af10e3aa8f1f0b0cd` and that source head. The artifact's checked-out commit, source hashes, compiler, debug/release test counts and xtask JSON were inspected and matched the local observations. [Sanitized artifact 13629863133](https://github.com/kmosoti/CEGAR/actions/runs/42512383739/artifacts/13629863133) has ZIP SHA-256 `dc328bedfe67f71cd315783291a606f26fa8565ddaf65af67b0175e92646a4f8` and expires 2026-12-08 18:55:34 UTC. The CI regression suite tests the existing graph core, not the proposed artifact-domain cases.

This provenance update is a later documentation-only commit; inspect the latest PR run separately. Earlier [run 42501958661](https://github.com/kmosoti/CEGAR/actions/runs/42501958661) tested the development baseline rather than the design proposal.

## Falsifiers and limits

A divergent graph result, absent labeled edge in a claimed witness, unexpected acceptance of malformed graph inputs, or non-conservative exhausted work/refinement invalidates the corresponding tested claim. A domain specification that promotes claims to observations, drops a possible unsafe action or failure branch, truncates a graph or conceals scope is not safe regardless of the Rust regression results. This note does not claim model fidelity, authorization, G15 measurement, hard OS resource enforcement or formal verification.
