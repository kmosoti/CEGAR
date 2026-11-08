# AGENTS.md

This file guides coding agents and automation working on CEGAR. It applies repository-wide unless a more specific nested AGENTS.md explicitly narrows its scope. Human contributors should also read CONTRIBUTING.md.

## Repository workflow (required)

Use **JJ with colocated Git** for local development; see [CONTRIBUTING.md](CONTRIBUTING.md). Before creating an issue, search existing issues and `knowledge/`. Link development PRs to their issue using `Refs #N` for partial work and `Closes #N` only when all acceptance conditions are met. Keep incomplete work in draft PRs. Inspect `jj status` and `jj diff`, describe finished changes with `jj describe -m "..."`, and move/push **only your work bookmark** in ordinary development. Advancing the shared development bookmark requires owner-authorized, validated integration as specified below. Never advance `main` or force-push shared/infrastructure history without authorization.

**Artifact policy:** durable schemas and synthetic regression fixtures stay in Git. Raw logs, profiles, temporary reports, local dumps and generated experiment outputs belong as **sanitized issue attachments or CI artifacts** with a short `knowledge/evidence/` manifest; never fabricate attachment links or claim tests were run when they were not.

**Preferred architecture:** Rust-native core with precise external dependency adapters and Sans-I/O where it genuinely improves deterministic verification. Choose concrete functions and small ports over a general plugin framework. Consult [ADR 0002](knowledge/decisions/0002-bounded-sans-io-core.md) for the tradeoff and reversal criteria.

## Development branch integration (required)

**Integration is not promotion to `main`.** For the current Issue #2 milestone, the shared development integration branch is `feat/2-bounded-verifier`; its design ancestor is `docs/cegar-design-agent-handoff`. These are *current* branch identities, not a permanent naming policy. Read the tracking issue and active PRs before assuming the same integration branch for a later milestone. The owner decides when to establish a long-lived `develop` or promote anything to `main`.

1. **Branch from the current development tip:** inspect `jj status`, `jj diff`, `jj bookmark list` and fetch current refs before creating an isolated change. Do not silently modify someone else's worktree or shared branch.
2. **Review through a PR targeting development:** audit, test, and implementation PRs should normally target the *current shared development branch*, not `main`. Link the issue with `Refs #N`, keep incomplete work draft, and preserve the design/implementation PR stack until an owner-authorized promotion. Check actual PR base/head, changes, reviewers, and CI.
3. **Integrate without rewriting history:** after review and applicable tests, use a **non-forced fast-forward** when the PR head descends from the current development tip; keep original commits and SHAs. For genuinely diverged branches, resolve via a **new merge commit** after inspecting conflicts and rerunning checks. Never rebase, squash, reset, or force-push published shared development history to manufacture a linear history. Do not create a redundant merge when the change is already an ancestor.
4. **Protect against races:** inspect the current remote integration SHA immediately before mutation; require an expected-head check / lease and a fast-forward. If the shared tip changes, stop and reassess rather than overwriting it. Ordinary contributors advance their own work bookmarks; only an explicitly authorized integration action advances the shared development bookmark.
5. **Verify the outcome:** re-read GitHub refs and PR states, confirm integrated commit ancestry, and check the resulting development-branch PR CI (not only the source PR's CI). Preserve failed results, exact revision IDs and artifact references in `knowledge/evidence/`. Mark a PR merged only when GitHub reports it; a linked issue is not automatically resolved.
6. **Release is a separate decision:** never merge PR #1 or PR #3 into `main`, retarget their bases, close Issue #2, or move `main` merely because development checks passed. Promotion requires explicit owner authorization and its own review/validation. Preserve audit history and keep `main` unchanged during ongoing development.

Example of the current stack: audit PR #4 was fast-forwarded (four original commits intact) into `feat/2-bounded-verifier`; GitHub marked #4 merged automatically, while design PR #1 and implementation PR #3 remain open and unmerged. See [ADR 0006](knowledge/decisions/0006-development-branch-integration.md). Do not repeat the integration when the commit is already present.

## Rust-only development tooling

The owner requires **Rust-native source and testing**. Use a dedicated Rust `xtask` crate for development orchestration; do not introduce Python scripts, Python subprocess test drivers, or another language for routine validation. Put independent reference oracles in Rust outside the trusted core. Historical non-Rust evidence is retained in knowledge records, not executed by CI. A genuine external dependency requires a specific adapter contract and explicit rationale.

## Mission

Build an inspectable, resource-bounded verification service in Rust. Neural adapters may propose plans and extract claims but may never certify permissions, rewrite binding policy, or authorize execution. Prioritize reproducible evidence over feature count.

## First actions in any task

1. Read README.md and relevant documents under docs/.
2. Inspect actual source, tests, manifests, and toolchain pins before claiming a feature exists.
3. Identify whether the requested change is to the **trusted computing base**, an **untrusted adapter**, or a **public contract**. State the boundary in the PR.
4. Make the smallest coherent, independently testable change. Avoid building unrelated infrastructure in the same PR.
5. Run available checks and report commands and exit status accurately. Do not claim a build/test when it was not executed.

## Non-negotiable invariants

- **No unverified execution:** v0 has no endpoint that executes an agent plan, shell command, or arbitrary program.
- **Authority cannot be deserialized from client input:** a client-supplied flag such as `"kind": "observation"` must never create a trusted observation. Only an authenticated collector/route with validated provenance may do that.
- **Predictions are advisory:** no threshold on d1, GLiNER2, a planner, or an LLM constitutes a permission decision.
- **Unknown is not allow:** timeout, truncated output, stale snapshot, missing evidence, solver `unknown`, exhausted refinement budget, or conflicting assumptions must not become `verified`.
- **All proof claims are scoped:** bind results to plan, policy, snapshot, tool-contract identity, checked properties, horizon, schedule semantics, assumptions, and verifier implementation.
- **No policy weakening as repair:** plan revision may change the candidate plan; abstract-model refinement may add precision; neither may silently remove a binding safety requirement.
- **Exact identity:** tested artifact digest must match published artifact digest; evidence and preconditions are not interchangeable across revisions.
- **No ambient privileges in the core:** filesystem, network, CUDA, model downloads, subprocesses, and secrets stay outside the pure domain core.
- **Bound every resource:** inputs, graph size, solver time, memory, queue depth, model requests, retries, and output. Return explicit inconclusive/unavailable results on limits.
- **Public data hygiene:** use synthetic fixtures only. Never commit real credentials, private traces, workplace information, arbitrary user conversations, or downloaded model weights.

## Architecture guidance

Preferred boundaries: pure `cegar-core`; API and lifecycle shell; trusted snapshot adapter; typed plan compiler; bounded verifier worker; optional inference adapter; read-only evidence/receipt storage.

Start with finite-state reachability and plain graph algorithms where they suffice. Introduce Z3 only for obligations whose expressiveness needs SMT. Add egglog only when a measured rewriting/equivalence problem exists. Avoid building a new model router, vector database, UI, or agent framework as a prerequisite.

Do not confuse:
- CEGAR abstraction refinement with counterexample-guided *plan repair*.
- A copied span with a true assertion.
- Parse/grammar validity with semantic or authorization validity.
- Solver `unsat` with a proof about unmodeled reality.
- Model size with total GPU/host resident memory.
- Throughput with useful completed tasks.

## Development targets

- Rust 1.100, edition 2024, once the chosen compiler and dependency lock are available and actually tested.
- Small crates with explicit, testable interfaces; prefer deterministic algorithms and sans-I/O domain logic.
- Type states and evidence provenance so untrusted predictions cannot silently cross authority boundaries.
- Make effect and freshness dependencies explicit rather than relying on mutable globals.
- No panic for malformed user input, no silent fallback from `inconclusive` to approval.
- Document invariants and important failure modes near the implementation.

## Testing and evidence requirements

For trusted-core changes, add tests for both safe and unsafe traces, spurious abstract counterexamples, stale attestations, contradictory premises, malformed graphs, timeouts, and absent evidence. Differential-test bounded CEGAR results against exhaustive concrete reachability on small generated models.

For neural adapters, test exact artifact/runtime versions, input serialization, Unicode span offsets, thresholds/calibration, missing fields, quantization drift, and resource peaks. Model output is never an oracle for policy tests.

Record evidence as: **Proposed → Source-audited → Built → Tested → Measured-on-G15 → Qualified**. These are distinct milestones, not one automatically advancing pipeline.

Before a PR: execute formatting, static checks, and relevant tests if available; include exact commands and failures. If checks cannot be run, say so.

## Scope control and review

- New external services, model artifacts, dependencies, network routes, or privileged actions require a written rationale and a security impact note.
- Changes to public schemas, verifier soundness assumptions, trust boundary, or execution authority require a design decision record and compatibility review.
- Keep operating-system choices and secrets out of the domain architecture. Host configuration is deferred until the G15 deployment is ready.
- Prefer documented rejection to unsupported promises of performance, proof, privacy, or production readiness.

## Agent handoff

The next implementation slice and acceptance criteria are defined in [docs/AGENT_HANDOFF.md](docs/AGENT_HANDOFF.md). Do not implement every milestone at once. Produce a PR with a narrow diff, runnable checks, and explicit unresolved risks.

