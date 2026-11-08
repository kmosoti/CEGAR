# Rust-only xtask validation record

- Date: 2026-11-07 US Central / 2026-11-08 UTC
- Tracking: [issue #2](https://github.com/kmosoti/CEGAR/issues/2), [draft PR #3](https://github.com/kmosoti/CEGAR/pull/3)
- Change: remove Python differential driver and test-only Rust transport, add independent Rust xtask reference check
- **Status: Built and tested in CI; no G15 measurement or production qualification.** Older Python-era evidence remains historical only.

## First all-green Rust-only run

[Run 42439569614](https://github.com/kmosoti/CEGAR/actions/runs/42439569614) completed successfully against source head `faff9bcea606649250d6ee1e2a26859da1274209`, stacked on design branch `docs/cegar-design-agent-handoff`.

- Environment: GitHub Actions Ubuntu 24.04 x86_64; `rustc 1.100.0-beta.12 (b0aaa0fca 2026-11-06)`; `cargo 1.100.0-beta.12 (3acc79c0c 2026-11-05)`; toolchain `beta-2026-11-07`.
- `cargo fmt --all --check`: **pass**.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: **pass**.
- `cargo test --workspace --locked --offline -- --nocapture`: **15 integration tests and one documentation test passed**.
- `cargo test --workspace --release --locked --offline -- --nocapture`: **same suite passed**.
- `cargo run -p xtask --locked --offline -- verify`: **pass**, Rust backward fixed-point reference checker with 28,868 exhaustive cases, 1,000 seeded cases, 24,187 action-labeled witnesses independently validated, and eight malformed Model constructor cases rejected. `seed=288925106514` (`0x4345474152`).
- `cegar-core` remains the same tested finite-state checker; the new `xtask` package and CI pipeline perform no Python calls.
- Repeating the exhaustive suite in debug/release/xtask does **not** create additional unique exhaustive models.

Raw sanitized CI evidence: [artifact 13601335933](https://github.com/kmosoti/CEGAR/actions/runs/42439569614/artifacts/13601335933), ZIP SHA-256 `fa1da6c156234f8a57511b80f55b713937e980203e29eb76a9f2771dbe8e812c`, expires **2026-12-08 02:41:23 UTC**. The artifact is from Actions, not a binary attachment uploaded to the issue.

## Failures observed during transition

The initial Rust-only run [42439511512](https://github.com/kmosoti/CEGAR/actions/runs/42439511512) compiled and passed debug/release and xtask comparison, but failed rustfmt and Clippy (`manual_is_multiple_of` in the seeded generator). Commit `faff9bcea606649250d6ee1e2a26859da1274209` applied the exact style corrections and used `is_multiple_of(7)`. Every gate was retained; the following run passed.

The current branch may receive further documentation/CI artifact-output changes. Those later commits require their **own** run confirmation rather than inheriting an earlier green status.

## Reproduction commands

\`\`\`sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline -- --nocapture
cargo test --workspace --release --locked --offline -- --nocapture
cargo run -p xtask --locked --offline -- verify
\`\`\`

The xtask oracle must compare exactly 28,868 exhaustive models and 1,000 deterministically seeded labeled models, replay each returned concrete witness independently, and reject eight malformed finite **constructor** cases. Its random generator differs from the former Python driver, so previous witness counts do not carry forward.

## Source/authority boundaries

Only the provided finite graph is being checked. No execution authority, no trusted collector, no live API, no model inference, and no G15 benchmark is implied. Rust 1.100 remains the target, currently represented by the pinned beta toolchain. Work-count limits are not hard wall-time/RAM limits.

## Evidence recording

Once the new GitHub Actions run is complete, append the exact run, source/merge SHA, compiler version, per-command results, stdout, artifact ID/hash/expiry, and any failed/remediated steps. Keep generated logs and snapshots in sanitized Actions artifacts, not Git.

Historical [0002 validation](0002-bounded-core-validation.md) remains a valid record of the *previous implementation*, including Python usage and its removal rationale. Do not erase or rewrite that evidence.
