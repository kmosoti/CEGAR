# Contributing to CEGAR

CEGAR is a public research-and-development project for trustworthy local agent-plan verification. Small, well-evidenced contributions are more valuable than broad implementations that hide assumptions.

## Before contributing

Read README.md, AGENTS.md, docs/ARCHITECTURE.md, docs/FORMAL_MODEL.md, and docs/SECURITY.md. Open an issue or draft PR for major interface, policy, model, or architecture changes. Discuss changes to the trusted computing base with an explicit failure model.

## Scope and workflow

1. Branch from the current main branch with a descriptive name such as `feat/finite-checker` or `docs/trust-model`.
2. Keep PRs focused on one behavior, invariant, or experiment.
3. Explain the problem, alternatives considered, scope, security impact, and evidence.
4. Include tests for new semantics. Update docs, schemas, examples, and migration notes when a contract changes.
5. Mark unfinished work as draft; do not merge undocumented placeholders as though they were implemented.
6. Request review before merging changes that affect authority, verifier soundness, credentials, resource access, or public APIs.

Conventional commit-style subjects are welcome (e.g. `feat(core): add concrete trace replay`), but clarity matters more than ceremony.

## Rust code expectations

Target Rust 1.100 and edition 2024 after the toolchain is pinned and validated. Keep pure models in the core; place I/O, adapters, subprocesses, and GPU integration behind explicit interfaces. Treat external inputs as adversarial. Prefer typed errors, deterministic tests, and finite resource budgets. Add property and differential tests to changes affecting soundness.

When a toolchain exists, ordinarily run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Do not claim these commands passed until executed with a documented toolchain. Future benchmark or integration checks may be added to CI.

## Reproducibility and results

Include exact compiler and dependency versions, model/runtime revisions and hashes, workloads, hardware/driver details, relevant settings, and raw results. Label estimates, predictions, source audits, and measurements separately. Prefer negative findings and documented limitations over selectively reporting favorable numbers.

Do not make production claims based on static inspection or synthetic-only results.

## Security and responsible disclosure

Use only synthetic or authorized, non-sensitive data in PRs and issues. Never upload tokens, private infrastructure configuration, confidential customer/workplace content, privileged paths, model weights, or personally identifying traces.

For suspected vulnerabilities, **do not publish a working exploit, secret, or sensitive details in a public issue**. Contact the repository maintainer privately through an available GitHub private-reporting channel or other private contact identified by the maintainer. If no private channel is configured, request one without posting vulnerability details.

A verification receipt never grants execution authority. Any proposed executor requires a separate reviewed design and enforcement tests.

## Research citations and attribution

Prefer the paper, original implementation, and pinned source revision. State what was actually inspected versus inferred. Cite upstream licenses and do not assume the repository MIT license extends to external models or data. Record provenance for benchmark fixtures and third-party code.

## PR checklist

- [ ] Objective, constraints, and boundaries are documented.
- [ ] Change is scoped; API/IR changes include compatibility considerations.
- [ ] Failure modes include stale/unknown/inconclusive outcomes.
- [ ] Relevant tests were added and executed, or their absence explained.
- [ ] Evidence claims correspond to actual tool output.
- [ ] No secrets, private data, or unlicensed artifacts were included.
- [ ] Documentation, examples, and research references are updated.

