# Bounded-core validation record

- Tracks issue #2; automated contribution, human review pending
- Scope: finite caller-supplied labeled transition models only
- Initial status: **source written; Rust execution not yet established**

## Environment observations

The authoring sandbox has Python 3.13.5 and Git, but no rustc, Cargo or JJ on PATH; a search of normal tool directories found no compiler. A public Git clone failed due to unavailable outbound DNS. Code is therefore published through the connected GitHub Git-data API, without advancing main or rewriting any shared history.

The CI workflow installs the dated target toolchain, reports `rustc -Vv`, and runs formatting, Clippy, debug/release tests and an independent Python comparison against the compiled Rust test driver. Exact workflow/run/commit and final results must be added after inspection. The presence of a workflow is not evidence it ran.

## Planned evidence

- 28,868 exhaustive models: all directed graphs with 1..3 states, all nonempty initial subsets and all bad subsets, including self-loops and zero-step violations.
- 1,000 seeded action-labeled models in Rust, independently checked by transitive closure.
- Separate Python closure oracle driving the compiled Rust executable across the exhaustive set plus 1,000 seeded models; concrete replay of every unsafe witness and malformed-input checks.
- Boundary tests for invalid states, duplicate set entries, malformed witnesses, zero work/refinement budgets, cancellation and maximum-size chains.

These counts describe test coverage authored, not passing execution results until CI evidence is linked below.

## Raw artifacts

Raw output is stored by the workflow as a sanitized GitHub Actions artifact with 30-day retention and linked from the issue. Do not commit generated logs, formatted-source copies, profiler output or benchmark dumps. The evidence bundle includes tested commit, source hashes, compiler version, per-stage output and any rustfmt patch.

No G15 model execution, latency, GPU/RAM measurement, transport authentication or production security claim is part of this contribution.
