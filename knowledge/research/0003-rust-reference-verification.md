# Finding 0003: Algorithmic independence does not require another language

- Date: 2026-11-07 US Central / 2026-11-08 UTC
- Origin: owner correction and automated design revision
- Status: **Deduction and source inspection; new xtask execution pending CI**
- Tracking: [issue #2](https://github.com/kmosoti/CEGAR/issues/2)

## Problem with the previous comparison

The previous Python driver called a compiled Rust example over a custom line protocol, then checked reachability with a bitset closure and manually validated returned witnesses. The actual prior CI results remain documented in [evidence 0002](../evidence/0002-bounded-core-validation.md). The cross-language design was not required to test the semantics of this supplied finite-model checker. It added a language/runtime and a protocol that the intended production service did not use.

## Rust-only replacement

The test driver is now \`xtask\`, a separate Rust crate with no third-party dependencies. It calls the public \`cegar_core::verify\` API and computes expected safety using a **backwards least fixed point**: start from all Bad states, repeatedly mark any predecessor of a marked state, and ask whether an initial state is marked. This is different from the CEGAR implementation's existential quotient and frontier refinement and the existing Rust integration test's transitive closure.

For each counterexample, xtask checks initial and Bad endpoint membership, state bounds, path length, and exact action-labeled edge existence **without using the core replay helper**. It also asserts that reports never carry execution authorization.

The exhaustive input family remains:

\`\`\`text
sum(2^(n*n) * (2^n - 1) * 2^n, n = 1..3) = 28,868
\`\`\`

The additional 1,000 directed multiaction graph cases use an explicit fixed xorshift64* generator and seed \`0x4345474152\`. This does **not** reproduce Python's Mersenne Twister cases or their historical witness count, so results must be recorded independently. Eight invalid **Model constructor** fixtures replace the removed eight malformed custom-transport records; they are not the same checks.

## What this can and cannot establish

A passing xtask provides evidence that independently written Rust reference semantics agree with the implementation over the specified cases. It does **not** establish all possible 128-state models, the correctness of the model ingestion path, real-world policy fidelity, hard process isolation, or the production JSON API (not implemented).

Future adversarial test investment is better placed in mutation testing of abstractions and a real input adapter, rather than preserving a Python-specific transport solely to claim cross-language independence.

**Falsifier:** any mismatch, incorrectly accepted witness, or invalid fixture admitted by the model constructor should fail CI and be minimized into a durable Rust regression test.
