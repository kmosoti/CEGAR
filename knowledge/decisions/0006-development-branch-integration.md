# ADR 0006: Integrate reviewed changes into the development branch before promotion

- **Date:** 2026-11-08 (US Central)
- **Status:** Owner-directed workflow; effective for ongoing Issue #2 development
- **Scope:** CEGAR repository branching and PR integration, not verifier semantics
- **Tracking:** [Issue #2](https://github.com/kmosoti/CEGAR/issues/2), [PR #1](https://github.com/kmosoti/CEGAR/pull/1), [PR #3](https://github.com/kmosoti/CEGAR/pull/3), [PR #4](https://github.com/kmosoti/CEGAR/pull/4)

## Decision

The owner clarified that **ongoing research and development changes are integrated into the shared development branch, not promoted automatically to `main`**. For the current milestone, the development integration branch is `feat/2-bounded-verifier`, inheriting the design baseline from `docs/cegar-design-agent-handoff`. PR #1 targets `main` and PR #3 targets the design branch, but both remain **draft and unmerged** until separate promotion approval.

Development tasks and audits should use isolated work branches with PRs targeting the current development integration branch. After review and validation, integrate by non-forced fast-forward when possible, retaining original commit identities. If refs have genuinely diverged, make a new merge commit, inspect conflicts, and rerun tests. **Do not rewrite published shared history** by force-pushing, squashing, rebasing or resetting it. Before integration inspect ancestry, current remote SHA and passing evidence; after integration confirm ancestry, branch head, PR states and CI on the updated development branch. No-op when a PR head is already integrated.

Agents may freely develop on their isolated work branches within authorization but **must not move the shared development branch without explicit integration authorization**. Moving `main` or merging stacked foundation PRs there requires a **separate, explicit owner decision**. Development CI success alone is not release authorization.

## Verified integration example

The previous feature-branch head was `8890fb03ce48ee96301df4e009818c6707e742c4`. PR #4's audit head was `6c3555eba9699cbfe5ebf53ee65b3c544d121d6b`, exactly four commits ahead and zero behind. The owner authorized integration and the shared development branch was advanced by a non-forced fast-forward with an expected-head check. No merge commit or history rewriting was required.

[PR #4 audit run 42499660450](https://github.com/kmosoti/CEGAR/actions/runs/42499660450) passed five gates; a subsequent [PR #3 integration run 42500825789](https://github.com/kmosoti/CEGAR/actions/runs/42500825789) passed formatting, Clippy, debug/release and Rust xtask checks at that integrated revision. GitHub marked PR #4 merged when the target acquired its head. PRs #1/#3 remained draft, open, unmerged, and `main` remained at `25de20c259f5a29610be790cdcc724fbcb5feb9d`. Historical facts must be rechecked before acting at a later date.

This ADR documents *workflow state and owner direction*, not a formal correctness result, G15 measurement or a permanent `develop` branch. No new merge of PR #4 should be manufactured after its commits already became ancestors.

## Tradeoffs and alternatives

- **Continuous merge to `main`: rejected for current R&D.** Would conflate tested research increments with owner-approved release scope.
- **Squash merging audits:** loses the individual audit and evidence commits from the branch's ancestry, undermining inspectability and the requested non-rewriting history.
- **Rebasing shared commits:** potentially creates alternate SHAs, breaks provenance and forces dependent branch repair; not justified merely for a linear graph.
- **Permanent `develop` immediately:** potentially useful later, but unnecessary churn while the project is organized around Issue #2. A future owner decision can introduce one without rewriting the current history.

## Reversal / future promotion criterion

Revisit the integration branch name when the current milestone is accepted or several parallel issues require a stable shared integration target. Promotion to `main` requires the owner's explicit approval, review of the stacked PR history, a validated destination diff, and documented acceptance conditions. A green CI badge is not enough.

**Validation boundary:** this is a documentation-only change. Its own GitHub Actions run is needed before claiming formatting/test coverage for the new head, though no Rust source or runtime behavior is modified.
