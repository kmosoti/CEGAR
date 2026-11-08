# ADR 0004: Rust-only development tooling and verification oracle

- Date: 2026-11-07 US Central / 2026-11-08 UTC
- Status: **Owner-directed change, implementation pending new CI qualification**
- Tracking: [issue #2](https://github.com/kmosoti/CEGAR/issues/2), [draft PR #3](https://github.com/kmosoti/CEGAR/pull/3)
- Supersedes: the Python process-oracle tooling decision in [ADR 0003](0003-finite-cegar-kernel.md), **not** its historical test evidence

## Context and owner direction

The first Rust kernel used \`scripts/differential.py\` to drive a test-only Rust whitespace/JSON example. Although it provided a distinct implementation language, it introduced an unnecessary Python toolchain, subprocess protocol, and extra serialization surface into a repository explicitly intended to be **Rust-native**.

The owner rejected that choice and directed **Rust \`xtask\` for development drivers**. This is an explicit requirement, not a speculative autonomous preference.

## Decision

1. Remove the Python comparison script and the test-only Rust oracle process driver it existed to serve.
2. Add one dependency-light Rust \`xtask\` workspace package invoked as \`cargo run -p xtask --locked --offline -- verify\`.
3. Put the independent oracle in \`xtask\`, **outside the core crate**. Use a different algorithm (backward least fixed-point reachability) from the CEGAR abstraction/refinement code and from the existing Rust transitive-closure tests.
4. Check every returned unsafe witness in \`xtask\` using an independent edge/label/initial/Bad validator; **do not call \`cegar_core::replay\` as the verification oracle**.
5. Reproduce 28,868 exhaustive small-model cases and 1,000 deterministic seeded, labeled cases. Keep invalid model-constructor checks; remove claims about malformed *whitespace transport* cases because that transport is removed.
6. Keep CI, developer commands, source hashing and machine-readable result records entirely Rust-based. Do not introduce Python for glue, automation, code generation, test driving, or CI diagnostics when \`cargo\`/\`xtask\` suffices.

## Trade-offs

- **Gained:** one Rust toolchain, no Python runtime or subprocess-driver protocol, simpler build and reproducibility, strong owner-intent alignment.
- **Lost:** the separate-language, cross-process and JSON/whitespace-transport differential test. That was *real evidence*, but it did not validate the future production HTTP/JSON contract.
- **Retained:** algorithmically independent reference checks, reproducible enumeration, explicit witness validation, debug/release tests, pinned CI logs and history of previous findings.

The new oracle shares Rust and the \`Model\` constructor with the implementation, so it is **not independently verified software** or proof of the complete service. A future real transport adapter must receive its own **Rust** contract, mutation, boundary, fuzz and integration tests.

## Boundaries

- \`cegar-core\`: no I/O, external dependencies or validation harness.
- \`xtask\`: development/test infrastructure, permitted CLI output; **not imported by production code**.
- CI: no Python installed as a project requirement or invoked by project steps.
- No extra generic task runner framework, external shell scripting layer or codegen service.

## Validation and reversal

Publish actual Cargo/Clippy/format/xtask outcomes in a new evidence record and Actions artifact before claiming this implementation is qualified. A disagreement or invalid witness is a failing gate. Revisit only if a clearly specified, non-Rust integration requirement cannot reasonably be tested through Rust code; obtain explicit owner approval before adding another language.
