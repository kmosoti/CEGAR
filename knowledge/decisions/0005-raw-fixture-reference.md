# ADR 0005: Compare the verifier against raw synthetic fixtures

- Date: 2026-11-08 US Central
- Status: proposed and implemented in the issue-2 audit change; human review pending
- Tracks [issue #2](https://github.com/kmosoti/CEGAR/issues/2); extends [ADR 0004](0004-rust-only-xtask-tooling.md).

## Context and decision

The prior xtask reference reachability and witness validation read the same constructed `Model` as the verifier. A constructor that accidentally drops an initial state, bad state or transition could hide that loss from both. Retain generated state count, initial/bad sets and labeled edges separately, derive reference results and witness validity from those raw fixtures, then compare the constructor's exposed representation to the original inputs. Bad-state output is canonical ascending; initial and transition order and duplicate edges remain intact. The oracle remains a separate Rust backward fixed point and does not call quotient, concretization, or core replay.

The independent witness checker accepts bounded cyclic witnesses up to `MAX_STATES`, matching the public replay length bound, rather than assuming all witnesses are simple. Actual verifier paths remain simple abstract BFS paths. These changes affect development verification, not the library's public contracts or execution authority.

## Alternatives and tradeoffs

Continue consuming `Model` getters: simpler, but cannot independently detect input ingestion corruption. A second language or subprocess protocol would increase environmental surface contrary to owner-directed Rust-only tooling. A second parser, symbolic solver or fuzzing framework is not needed to compare these small typed fixtures. Raw fixture checks still share the compiler, generator, `Transition::new` and semantic assumptions; independence is relative to `Model::new`, and they are not independently verified software.

## Evidence, uncertainties and reversal

See [the audit](../research/0004-independent-soundness-audit.md) and the accompanying validation record. Unit tests deliberately remove an initial, bad or edge, and change a witness label, testing reference sensitivity; they are not a full mutation campaign against production code. A mismatched reference result, incorrectly validated witness, dropped constructor field, or failure to reproduce fixtures falsifies the relevant claim. Revisit if a real transport/domain compiler introduces a different source of authority or an independently encoded reference becomes necessary. Roll back by reverting the audit change; do not weaken safety or treat failed/inconclusive checks as success.
