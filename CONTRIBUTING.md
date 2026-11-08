# Contributing to CEGAR

CEGAR is a public Rust-first systems research project exploring evidence-grounded agent-plan verification, counterexample-guided abstraction refinement, and optional local neural assistance. We value inspectable assumptions, small changes, reproducible failures, and specific tests over claims of production readiness.

## Read first

Read [README.md](README.md), [AGENTS.md](AGENTS.md), [docs/FORMAL_MODEL.md](docs/FORMAL_MODEL.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/SECURITY.md](docs/SECURITY.md), and the [knowledge index](knowledge/README.md).

Before opening an issue, search [existing issues](https://github.com/kmosoti/CEGAR/issues) and `knowledge/` for prior findings. One independently verifiable problem, hypothesis, or proposal per issue.

## Version control: Jujutsu (JJ) with colocated Git

**Preferred development interface: JJ backed by a colocated Git repository.** GitHub remains the source of truth for reviews and the protected `main` integration history. Colocation preserves ordinary Git tooling and permits JJ-managed local revisions; it does not mean contributors should maintain competing Git and JJ mutation workflows.

For an existing Git checkout, initialize the colocated JJ workspace:

```sh
cd CEGAR
jj git init --colocate
jj git colocation status
```

Alternatively, use `jj git clone https://github.com/kmosoti/CEGAR.git` (normally colocated under current JJ defaults). Check your installed JJ version, as configuration and command flags can change.

Inspect history and changes:

```sh
jj status
jj diff
jj log -r 'ancestors(@, 5)'
jj bookmark list
```

Describe the change you have actually made:

```sh
jj describe -m "feat(core): verify concrete counterexamples"
```

**Work-branch workflow:** create changes from the *current development integration branch* identified by the tracking issue/PR, not automatically from `main`; advance **only your own work bookmark** until a reviewed, authorized integration. For Issue #2 this integration branch is `feat/2-bounded-verifier`, but verify it before subsequent milestones. JJ does not automatically advance bookmarks when commits are created or described. Example, after checking the intended revision:

```sh
jj git fetch
jj new feat/2-bounded-verifier@origin
# edit, test, and inspect
jj status
jj diff
jj describe -m "feat(core): add bounded reachability"
# create/move a feature bookmark to the validated revision, then
jj bookmark create feat/bounded-reachability -r @
jj git push --bookmark feat/bounded-reachability
```

If a feature bookmark already exists, use `jj bookmark move feat/bounded-reachability --to @` instead of creating it twice. Use `jj git push --dry-run --bookmark feat/bounded-reachability` before pushing consequential updates, and verify that the target is the intended remote bookmark. If the checkout already contains work, inspect it before using `jj new` or rebasing.

**Do not move or push the `main` bookmark as part of feature work.** PRs for the current milestone target the approved development integration branch; promotion to `main` is an independent owner-authorized decision, not the default next step. Never force-push `main`, rewrite shared history, or force-push infrastructure history without explicit owner authorization. Avoid mixing mutating Git commands with JJ changes in the same worktree unless you understand colocation's automatic import/export behavior.

Primary references: [JJ Git compatibility](https://docs.jj-vcs.dev/latest/git-compatibility/), [bookmarks](https://docs.jj-vcs.dev/latest/bookmarks/), and [GitHub workflow](https://docs.jj-vcs.dev/latest/github/).

## Development integration and promotion

The current shared integration branch for Issue #2 is `feat/2-bounded-verifier`. Its audit PRs target that branch. The design PR #1 and implementation PR #3 remain stacked and draft while development continues. This arrangement is not a mandate to reuse an issue-named branch forever.

Before integration, verify the source PR's head/base, reviewed diff, relevant tests, current remote development tip, and ancestor relationship. Prefer a **non-forced fast-forward with expected-head verification** when source descends from development; this retains the original commit SHAs. If histories diverge, use a **new, reviewed merge commit** and revalidate; do not squash/rebase/force-update published shared history. An already-ancestor PR needs no duplicate merge. Avoid running an integration if explicit authorization is missing.

After integration, verify remote refs and commit ancestry, check resulting CI, update the tracking issue and `knowledge/evidence/`, and note the PR's actual GitHub state. Do not confuse GitHub marking a PR merged with merging any *other* stacked PR into `main`.

**Never automatically promote `main`, merge PR #1/#3 there, or close Issue #2.** The owner separately authorizes release/promotion after review and acceptance. [ADR 0006](knowledge/decisions/0006-development-branch-integration.md) records this workflow and its rationale.

## Issue guidelines

Use a descriptive title such as `bug(core): ...`, `feat(verifier): ...`, `research(inference): ...`, or `docs: ...`. Choose a specific domain rather than carrying over unrelated infrastructure issue types.

Use the following lightweight outline. Omit irrelevant sections, but state unknowns explicitly.

### Goal / problem
What behavior is desired? Why does it matter?

### Current state and evidence
What was actually observed? Include environment, reproduction steps, exact redacted errors, relevant revisions, and links to evidence. Distinguish **verified facts**, **deductions**, **hypotheses**, and **unresolved uncertainty**.

### Proposed direction (optional)
Architecture, mechanisms, alternatives, invariants, constraints, and relevant tradeoffs.

### Acceptance criteria
- [ ] Observable outcome under clearly stated assumptions
- [ ] Validation, regression, differential, or property tests
- [ ] Recovery/rollback and negative cases where applicable

### Risks and recovery
Security implications, irreversible behavior, availability, privacy, data loss, and safeguards.

### References
Related issues and PRs, `knowledge/` records, artifacts, pinned upstream revisions/tags, and dates for mutable documentation.

For investigations, maintain **durable findings and provenance** in `knowledge/`; store **curated evidence references and interpretation** in `knowledge/evidence/` and ADRs under `knowledge/decisions/` as warranted. Store *raw or transient* output as **sanitized GitHub issue attachments or linked run artifacts**, not committed repository files. Distinguish “not tested” from an actual passing test.

A starter issue template is in [.github/ISSUE_TEMPLATE/engineering.md](.github/ISSUE_TEMPLATE/engineering.md).

## Evidence and ephemeral artifacts

**Keep in Git:** Rust source, versioned contracts/schemas, reproducible synthetic regression fixtures, concise audited research, stable test drivers, ADRs, and sanitised small reference examples.

**Do not commit:** benchmark dumps, model checkpoints, generated reports, one-off traces, temporary scripts, profiler files, logs, caches, secrets, private datasets, local device inventories or unredacted terminal captures.

For an investigation, attach sanitized evidence to its GitHub issue (or link a retained CI artifact), record collection command/environment and timestamp, and create a short permanent `knowledge/evidence/` note that links the issue and explains what the evidence does and does not establish. Ensure retention/expiry is not confused with durable source evidence. If raw attachments cannot be uploaded programmatically, say so; do not paste sensitive material into the repository or invent a link.

Before pushing, inspect `jj status` / `jj diff --stat` and check for generated files. Remove accidental ephemeral files from the change, not by rewriting published shared history. Reusable regression fixtures that encode bugs **are source**, not ephemeral artifacts.

## Pull requests

Every development PR should reference its tracking issue.

- Use `Refs #N` for partial or design-only work.
- Use `Closes #N` only when merging fully satisfies the issue's acceptance criteria.
- Keep design-only, research, uncompiled or incomplete implementations as **draft PRs**.
- Include alternatives, explicit unsolved constraints, security impact, tested behaviors, and exact commands/results.
- Do not silently expand a narrowly scoped issue into a new inference stack or execution platform.

Human review is required for trusted collection, authority paths, policy, soundness claims, schemas, model access, networking, or any future executor. Only advance the owner-approved development integration history after the review, validations and explicit integration authorization. Promotion to `main` requires separate approval.

## Rust-only development and test tooling

Use `cargo` and the Rust `xtask` crate for verification, synthetic model generation and development tooling. Do not add Python scripts, an extra CLI/runtime, or a disposable subprocess transport solely to run tests. Keep the test oracle algorithmically independent of CEGAR internals even when both are Rust. See [ADR 0004](knowledge/decisions/0004-rust-only-xtask-tooling.md).

## Rust-native and Sans-I/O design preference

**Prefer Rust-native code for service and reasoning logic.** Use a **Sans-I/O / functional-core, imperative-shell boundary** where it reduces the cost of reasoning, testing and simulation.

- Represent state, transitions, obligations, errors and outcomes as typed Rust values and pure transformations.
- Let the core accept inputs and produce decisions/effects without directly touching the network, storage, CUDA, clocks, subprocesses or environment.
- Put external dependencies behind **small, explicit adapters**: authenticated snapshot collector, clock/identity source, optional mistral.rs decision worker, optional ONNX extractor, optional Z3 worker, and host lifecycle.
- No general plugin framework or tower of trait objects until at least two concrete implementations create a real need.
- Use deterministic finite-state/graph algorithms first; introduce Z3 when a property needs SMT, and egglog only for demonstrated rewrite/equality work.
- Keep initial code small enough that independent reachability tests can audit semantics. Do not turn Sans-I/O into an abstract architecture costume.

Rust 1.100 and edition 2024 are the intended target. Pin the exact toolchain and dependencies when implementation begins; report actual toolchain availability and test output. When a workspace exists, ordinarily run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Record exact commands and exit codes. Never claim checks passed unless they were executed.

## Verification and research quality

No model confidence is permission. A source-matching span is not a verified statement. Grammar-valid output can still be semantically unsafe. A solver proving a property about a flawed encoding does not establish reality. Bound checks, replay concrete witnesses, and return **inconclusive** on missing facts, stale context, solver unknown, resource limits or contradictions. A `verified_within_model` receipt is never execution authorization.

Use exact source citations, revisions, model/checkpoint hashes, bounded problem definitions, and raw measured evidence. Record counterexamples, rejected hypotheses, and negative findings. Do not transplant performance claims between hardware and model versions.

Public documentation must distinguish **Proposed**, **Source-audited**, **Built**, **Tested**, **Measured-on-G15**, and **Qualified**.

## Security and disclosure

Never commit or post secret values, tokens, private keys, proprietary or workplace data, personal traces, or unredacted process output. Use synthetic examples and redact evidence before issue attachment.

Do not report exploitable vulnerabilities through public issues. Use a private reporting channel or request one without revealing details. Model inputs and external documents are untrusted data. V0 exposes no execution endpoint, and review receipts are not permissions.

The repository MIT license does not automatically grant rights to third-party models, checkpoints, datasets or code.

## PR checklist

- [ ] Linked issue and narrowly defined objective
- [ ] Trusted boundary and policy invariants preserved
- [ ] Alternatives, rejected assumptions and unknowns recorded
- [ ] Relevant tests run (or explicitly **not tested**)
- [ ] Artifacts sanitised and attached to issue, not added as ephemeral files
- [ ] Documentation, contracts and synthetic fixtures updated
- [ ] Review/rollback plan appropriate to change risk
