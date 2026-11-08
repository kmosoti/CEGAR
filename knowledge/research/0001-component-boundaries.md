# Research note 0001: Evidence and runtime component boundaries

- **Date:** 2026-11-07
- **Status:** Source-audited, **not** runtime qualified
- **Scope:** CEGAR candidate components on a 4 GiB-VRAM / approximately 7.1 GiB-RAM Dell G15
- **Tracking:** [#2](https://github.com/kmosoti/CEGAR/issues/2)

This note distills claims previously assessed in [the research audit](../../docs/RESEARCH_AUDIT.md). The audit is a source-reading record, not an independently run benchmark.

## Established architectural distinctions

1. **mistral.rs d1-2B** supports a documented structured `/v1/systemone` decision interface with yes-probabilities, choice, score and extractive questions. Same-state question sets default to groups of up to 16; this is not 16 independently scheduled HTTP requests. For some GGUF question types, auxiliary tokenizer/calibration configuration is required. **d1 output is predictive, never authorization evidence.**
2. **GLiNER2** is an optional schema-driven extraction *alternative*. ONNX/Rust support requires the exact exported architecture and decoder; a generic GLiNER wrapper is not proof of GLiNER2 parity.
3. **LLMLingua-2** performs token-selection compression, not verified semantic equivalence. Exact policy constraints, paths, resource IDs, arithmetic, and negation must not be removed from authoritative inputs.
4. **FunctionGemma-270M** offers a specialized function-call candidate, not automatic trustworthy DAG synthesis. `llguidance` can enforce a supported grammar, but syntax is not authorization or semantic soundness.
5. **Z3** can report satisfiable/unsatisfiable/unknown for a stated formula. `UNSAT` in a bad-trace query is a scoped absence-of-counterexample conclusion, not a counterexample. `egglog` is a separate Datalog/equality-saturation tool, not an SMT replacement.
6. **Counterexample-guided refinement** requires sound overapproximation and concrete trace feasibility, not simply asking a model to fix its last answer. Evidence provenance and model fidelity remain part of the trusted computing base.

## Measurements still missing

- Actual G15 VRAM/RAM peaks for any component; driver and engine kernels.
- Latency and quality of d1 question grouping and optional quantization.
- GLiNER2 Rust/ONNX parity against the checkpoint.
- LLMLingua effect on exact technical evidence and downstream task outcomes.
- Any implementable Rust verifier build, soundness property tests or G15 service measurements.

## High-value next action

Build the non-neural, bounded Rust checker first. Measure independent concrete reachability agreement and trace replay. Only after useful domain tests exist should the d1 worker be introduced as an untrusted interpretation adapter.

## Primary sources

- [Pinned mistral.rs decision-model guide, revision addea296](https://github.com/EricLBuehler/mistral.rs/blob/addea2961deaafe59ab3d8894303a9ff2eca1204/docs/src/content/docs/guides/models/use-decision-models.mdx)
- [LiquidAI d1-2B checkpoint](https://huggingface.co/LiquidAI/d1-2B)
- [GLiNER2](https://huggingface.co/fastino/gliner2-base-v1) and [gliner2-rs](https://github.com/dariofinardi/gliner2-rs)
- [LLMLingua-2 paper](https://arxiv.org/abs/2403.12968)
- [FunctionGemma model card](https://huggingface.co/google/functiongemma-270m-it)
- [llguidance](https://github.com/guidance-ai/llguidance)
- [Z3 guide](https://microsoft.github.io/z3guide/docs/logic/basiccommands/)
- [egglog](https://arxiv.org/abs/2304.04332)
- [CEGAR method](https://web.stanford.edu/class/cs357/cegar.pdf)

**Falsification focus:** if a source claim cannot be reproduced against the selected checkpoint/runtime and test corpus, mark it unsupported and remove the candidate from a qualified deployment profile.
