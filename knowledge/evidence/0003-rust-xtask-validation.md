# Rust-only xtask validation record

- Date: 2026-11-07 US Central / 2026-11-08 UTC
- Tracking: [issue #2](https://github.com/kmosoti/CEGAR/issues/2), [draft PR #3](https://github.com/kmosoti/CEGAR/pull/3)
- Change: remove Python differential driver and test-only Rust transport, add independent Rust xtask reference check
- **Status: Pending new CI execution at time of authoring.** Earlier passing Python-era CI must not be treated as validation of xtask.

## Planned validation gates

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
