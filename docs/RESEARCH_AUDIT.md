# Research audit: candidate neural and symbolic components

**Date:** 2026-11-07 (US Central). **Scope:** publicly available documentation/source inspection and theoretical deductions. **Not performed:** G15 model execution, CUDA performance tests, artifact licensing sign-off, live service testing, or benchmark replication.

This audit treats the workshop proposal as candidate hypotheses. No model component is required for the initial verifier.

| Candidate | Supported claim | Invalid shortcut | Role in CEGAR |
| --- | --- | --- | --- |
| LiquidAI d1-2B through mistral.rs | Documents non-generative decision endpoint `/v1/systemone` with `noul`, choice, scoring and extractive question types; defaults to question groups up to 16 | Treating a model yes-score as a verified proposition or assuming 16 independent requests batch automatically | Optional question-template classification and claim extraction |
| d1 GGUF variants | Upstream documents Q4 d1-2B GGUF and configuration/calibration requirements for some question types | Assuming a single GGUF weight file is a complete deployable artifact | Candidate after exact revision and adapter parity |
| GLiNER2 | Schema-oriented entities, relations and extraction; Rust ONNX adapters under development | Assuming generic GLiNER Rust packages work unchanged with GLiNER2 checkpoints or that spans establish truth | Alternative, separately qualified extractor |
| LLMLingua-2 | Bidirectional token-classification based compression | Assuming deletion preserves negation, paths, authority, meaning, or downstream latency universally | Optional compression of non-authoritative background |
| FunctionGemma-270M | Small function-call model intended for task-specific tuning | Assuming reliable zero-shot authorized plan synthesis | Research planner after templates and labeled fixtures |
| LFM family | Small hybrid architecture candidate for local generation | Assuming smaller growing KV automatically guarantees task correctness or performance | Optional planning challenger |
| llguidance | Supported constrained JSON/grammar decoding mechanisms | Assuming syntax membership proves DAG acyclicity, existence, permission, or semantic validity | Constrain proposal syntax only |
| Z3 | SAT/UNSAT/UNKNOWN and model/unsat-core interfaces | Interpreting UNSAT as a counterexample or solver output as real-world authorization | Bounded verifier when graph algorithms insufficient |
| egglog | Datalog plus equality saturation | Treating it as a drop-in SMT verifier | Optional rewriting/equivalence experiments |

## Pinned mistral.rs documentation observation

At source commit `addea2961deaafe59ab3d8894303a9ff2eca1204`, the [decision model guide](https://github.com/EricLBuehler/mistral.rs/blob/addea2961deaafe59ab3d8894303a9ff2eca1204/docs/src/content/docs/guides/models/use-decision-models.mdx) describes:

- `LiquidAI/d1-2B` for text/images/video, and `d1-Omni-2B` also for audio.
- `POST /v1/systemone` with `state` and named `questions`.
- Default `set_size = 16` questions per state read group; more than 16 questions cause more groups.
- Output of `noul` is a model probability/score for yes, not a formal true/false fact.
- Extractive text span information is copied from the submitted state, but intent and real-world truth remain unresolved.
- Certain decision types with GGUF require an additional `--tok-model-id` for configuration/calibration, and multi-modal artifacts may require a projector.

This source commit is **not** a pinned binary, model revision, CUDA build, GPU benchmark, or license audit. All those require separate qualification.

## Correctness distinctions

- A classifier threshold optimizes a measured error tradeoff; it is not a permission proof.
- A copied substring can carry negation or malicious instructions; verify its offsets and provenance.
- A valid JSON schema can still encode an invalid plan, stale artifact or unauthorized tool.
- A solver can establish a property of an unsound encoding; validate the encoder against concrete controls.
- An unsat core is an assertion conflict explanation, not a concrete unsafe execution.
- More co-resident models may make a 4 GiB service *slower*, particularly when model reload and memory peaks are counted.

## Implementation consequences

1. Build deterministic typed verification first.
2. Keep all neural output as predictions/claims, never observations.
3. Initially test **one** optional d1 worker rather than stacking d1, GLiNER2, LLMLingua and a planner.
4. Preserve exact evidence during context packaging.
5. Test all latency/VRAM claims on the G15 with complete runtime/model identity.
6. Attribute model evaluation separately from formal system safety.

See SOURCES.md and EVALUATION.md. Mark future statements as Proposed, Source-audited, Built, Tested, Measured-on-G15 or Qualified.

