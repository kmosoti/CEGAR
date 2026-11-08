# Bounded-core validation record (historical, Python-era)

> **Historical evidence only.** The owner subsequently required a Rust-only development workflow. The Python driver and custom process protocol have been removed from the current branch. See [ADR 0004](../decisions/0004-rust-only-xtask-tooling.md) and [the new xtask validation record](0003-rust-xtask-validation.md). Keep the earlier results for provenance; do not treat them as tests of the replacement.

- Date: 2026-11-07 US Central / 2026-11-08 UTC
- Tracks [issue #2](https://github.com/kmosoti/CEGAR/issues/2) and [draft implementation PR #3](https://github.com/kmosoti/CEGAR/pull/3)
- Decision origin: automated contribution requested by the owner; human review pending
- Status: **Built and tested in GitHub Actions, not measured on G15 and not production-qualified**
- Scope: supplied finite labeled transition models only; no execution authority

## Verified green run

[Run 42438145560](https://github.com/kmosoti/CEGAR/actions/runs/42438145560) completed successfully. Its source head is `763d12adc85cc503a16b492042ee06e413eb03ae`; GitHub tested synthetic merge commit `0d57f07e69ec0b7c03da67a4c60d16c751a8ba38` against design head `6c55b715a5da21ea9600e935666fe688ef21ef95`. Subsequent documentation-only commits must not be described as separately tested without inspecting their runs.

The run used Ubuntu 24.04, x86_64, `rustc 1.100.0-beta.12 (b0aaa0fca 2026-11-06)`, compiler commit `b0aaa0fca8a4b445972b94416341beb2e5fcecce`, Cargo `1.100.0-beta.12 (3acc79c0c 2026-11-05)`, LLVM 23.1.5 and Python 3.12.3. The exact dated toolchain is `beta-2026-11-07`, not a claim about stable Rust 1.100.

| Executed command | Observed result |
| --- | --- |
| `cargo fmt --all --check` | Passed; generated formatting patch empty |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Passed |
| `cargo test --workspace --locked --offline -- --nocapture` | 15 integration tests and 1 doc test passed |
| `cargo test --workspace --release --locked --offline -- --nocapture` | Same 15 integration tests and 1 doc test passed |
| `cargo build --example oracle_driver --locked --offline` | Passed |
| `python3 scripts/differential.py --driver target/debug/examples/oracle_driver` | 29,868 model comparisons, 24,188 concrete witness replays and 8 malformed-input checks passed |

## Coverage and limitations

The Rust exhaustive test covers all directed graphs on 1, 2 and 3 states, all nonempty initial subsets, and every bad-state subset: `sum(2^(n*n) * (2^n - 1) * 2^n, n=1..3) = 28,868`. These exhaustive graphs have one action label. A separate Rust test uses 1,000 seeded multi-label graphs. Other tests cover a 128-state chain, nondeterministic successors, duplicate edges, cycles, malformed models/witnesses, empty initial sets, zero-step violations and budget/cancellation failures.

The Python process test independently computes bitset transitive closure and drives the compiled Rust executable over the same exhaustive family plus its own 1,000 seeded graphs (`seed=288925106514`). It checks endpoint membership and every state/action/target edge in returned witnesses. The Rust random generator uses a different seed/generator. Debug, release and Python repeats of the exhaustive family are **not additional distinct models**.

This is finite test evidence, not a machine-checked proof of the implementation. It does not establish all 128-state systems, adversarial allocator behavior, trusted snapshot fidelity, an HTTP service, task-domain policy correctness, production authorization or G15 performance. Hard process RAM/time limits remain a worker-adapter responsibility. Parser byte limits in the example are test-transport behavior, not implementation of the proposed JSON API.

## Raw evidence outside Git

[CI artifact 13600009377](https://github.com/kmosoti/CEGAR/actions/runs/42438145560/artifacts/13600009377) contains per-command logs, exact compiler identity, source hashes, tested commit and formatted-source copies. The downloaded ZIP SHA-256 was independently checked: `d4bc857873e85cbed7bc51422f4d0c89c15ff343f48168a8f3a27efb1f11101e`.

Artifact expiry reported by GitHub: **2026-12-08 02:21:07 UTC**. An expired link is not permanent evidence availability; rerun the committed test procedures or retain a sanitized copy in an approved artifact store. No raw logs, generated reports or copied CI source were committed.

Important source SHA-256 values from the run:
- `crates/cegar-core/src/lib.rs`: `47b5ed0a8514822ee206c3770ea3c71845475854b0d3be7b98edd6db93a27256`
- `crates/cegar-core/tests/verification.rs`: `2a825bf2cd83458691adb165439fc4d2d9fe11d9d9c70990ebc928361ac3227c`
- `scripts/differential.py`: `7c952fc75ffcc882c4b4d1fe8cee765da63959fd28f65ce85d26d48abcf58d2f`

## Failed attempts and remediation

1. [Run 42437394087](https://github.com/kmosoti/CEGAR/actions/runs/42437394087) failed before a job started. The workflow used the `runner` context in job-level `env`; the available-context rules do not permit it there. The workflow now uses ignored `artifacts/ci` for transient output. This was a workflow error, not a Rust test result.
2. [Run 42437500148](https://github.com/kmosoti/CEGAR/actions/runs/42437500148) installed the requested compiler, but failed formatting and compilation because the test-only driver contained an unescaped JSON closing brace in a Rust format string. Fixed the brace; applied the actual rustfmt output; added nondeterministic, duplicate-edge and cycle regressions. Its [artifact](https://github.com/kmosoti/CEGAR/actions/runs/42437500148/artifacts/13600212037) preserves the failed evidence.
3. [Run 42437784985](https://github.com/kmosoti/CEGAR/actions/runs/42437784985) passed Clippy, both test configurations and the Python comparison. One final assertion layout failed rustfmt. Applied the exact formatting-only patch; all gates remained enabled. Its [artifact](https://github.com/kmosoti/CEGAR/actions/runs/42437784985/artifacts/13599364832) preserves that intermediate result.
4. Run 42438145560 passed every gate. No validation step was weakened to obtain green.

The initial dependency-free Cargo.lock was specified directly, not represented as a locally generated lock; `--locked --offline` subsequently validated it. The pinned v4 GitHub actions run successfully but emit Node 20 deprecation warnings under the runner's Node 24 override. Treat action-version maintenance as separate follow-up, not a hidden runtime guarantee.

## Authoring environment and workflow exception

The authoring sandbox had Python 3.13.5 and Git but no Rust/Cargo/JJ on PATH, and public Git clone failed due to outbound DNS. Changes were therefore authored through the connected GitHub Git-data API with expected-head checks, on a separate implementation branch. No force push, main update or design-branch rewrite was performed. This does not replace the repository's preferred colocated JJ workflow for equipped development hosts.

See [ADR 0003](../decisions/0003-finite-cegar-kernel.md) and [findings](../research/0002-finite-checker-findings.md) for interpretation rather than raw output.
