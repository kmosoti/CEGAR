# Primary research and implementation sources

Sources below establish **published claims or documented APIs**, not validation of a deployed CEGAR build. Where a source is mutable, record a revision in any implementation PR relying on it.

1. Clarke et al., **Counterexample-Guided Abstraction Refinement (CEGAR)**: https://web.stanford.edu/class/cs357/cegar.pdf
2. **mistral.rs decision-model guide**, audited source revision `addea2961deaafe59ab3d8894303a9ff2eca1204`: https://github.com/EricLBuehler/mistral.rs/blob/addea2961deaafe59ab3d8894303a9ff2eca1204/docs/src/content/docs/guides/models/use-decision-models.mdx
3. **LiquidAI d1-2B model card**: https://huggingface.co/LiquidAI/d1-2B
4. **GLiNER2 base checkpoint**: https://huggingface.co/fastino/gliner2-base-v1
5. **gliner2-rs**: https://github.com/dariofinardi/gliner2-rs
6. **gline-rs** (separate generic GLiNER implementation): https://github.com/fbilhaut/gline-rs
7. **LLMLingua-2**: https://arxiv.org/abs/2403.12968
8. **LLMLingua-2 checkpoint**: https://huggingface.co/microsoft/llmlingua-2-bert-base-multilingual-cased-meetingbank
9. **FunctionGemma model card**: https://huggingface.co/google/functiongemma-270m-it
10. **mistral.rs structured output**: https://docs.mistralrs.dev/guides/serve/structured-output/
11. **llguidance**: https://github.com/guidance-ai/llguidance
12. **Z3 basic operations / outcomes**: https://microsoft.github.io/z3guide/docs/logic/basiccommands/
13. **Z3 parameters and limits**: https://microsoft.github.io/z3guide/programming/Parameters/
14. **egglog paper**: https://arxiv.org/abs/2304.04332
15. **Axum**: https://docs.rs/axum/latest/axum/
16. **Rust release and stability information**: https://forge.rust-lang.org/

## Source reporting policy

For each new claim, record the exact source, inspected revision/date, whether the claim is documented, derived, or measured, and the evidence that would falsify it. Source agreement is not equivalent to independent experimental verification. The source list is not an endorsement of a model's suitability, performance, safety, or license terms.

