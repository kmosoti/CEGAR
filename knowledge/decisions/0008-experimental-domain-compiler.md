# ADR 0008: Experimental domain compiler on Fedora

- **Status:** Experimental; prototype implemented and self-tested on the Fedora
  workstation. Not reviewed, not qualified, not a policy approval.
- **Date:** 2026-11-08 (US Central)
- **Tracking:** [Issue #7](https://github.com/kmosoti/CEGAR/issues/7); depends on
  the proposed semantics of [ADR 0007](0007-artifact-domain-semantics.md) and
  [Issue #5](https://github.com/kmosoti/CEGAR/issues/5).
- **Baseline inspected:** integration tip `3342425` (`feat/2-bounded-verifier`
  after PR #6). Nothing here changes `cegar-core` or `main`.

## Context

Issue #7 asks for an *executed* vertical slice rather than another design
document: measure the real workstation, compile the proposed stage → registered
test → publish semantics into the existing finite checker, and check the
compiler against an independently written oracle. The design left two structural
questions open for whoever implemented it: where the compiler lives relative to
the trusted core, and how the oracle stays independent without duplicating the
implementation it is meant to falsify.

## Decision

1. **A new pure crate, `crates/cegar-domain`, holds the compiler.** It depends
   only on `cegar-core`, keeps `#![forbid(unsafe_code)]`, and performs no I/O.
   `cegar-core` is unchanged: it still verifies a supplied graph and still
   reports `supplied_finite_model_only` with `execution_authorized = false`.
   Putting the compiler in the core crate would have silently widened the core's
   claimed scope.
2. **Zero external dependencies, including for parsing.** CI runs
   `cargo ... --offline`; adding `serde`/`http` crates would require vendoring and
   a supply-chain decision this issue does not authorize. The domain crate has
   no dependencies; the later local-model adapter is restricted to `std` and a
   bounded, hand-written request/response path.
3. **The independent oracle lives in `xtask`, outside the crate it checks.** It
   re-implements every precondition and outcome rule from the raw plan and
   context, uses a depth-first schedule enumeration instead of the compiler's
   canonical-state breadth-first search, and replays every compiler witness
   against its own step function. Sharing a rule table would have made the check
   circular; sharing the rule *identifier* enum does not, because agreement about
   names is not agreement about reachability.
4. **Fixtures carry explicit provenance.** `Provenance::SyntheticFixture` is a
   variant, not a boolean flag, and it is copied into every receipt. There is no
   constructor in this slice that produces a "validated observation" from
   serialized input, so a client-supplied `kind: observation` cannot exist yet.
5. **Assumptions are data, not defaults.** Stage success bindings come from
   `Context::stage_outputs`; a hypothetical pass profile supplies issuer,
   evidence principal, tenant, audience and validity. If a stage node has no
   declared output assumption the compilation is `inconclusive` rather than
   assuming the plan's declared identity.
6. **Bounded first-violation monitor, single `Bad` state.** All unsafe attempts
   share one terminal monitor state but keep distinct action labels with their
   violated rules, so the core's witness still names the exact attempt.

## Consequences

- The compiler can disagree with the core or the oracle, and the differential
  check turns that into a test failure rather than a comment. It already found
  one real ambiguity (see [research 0007](../research/0007-domain-compiler-findings.md)).
- A single `Bad` state means the graph does not distinguish post-violation
  behavior. That is intentional for a safety-only question and is recorded in
  every receipt; an answer about post-violation effects must be `inconclusive`.
- No dependency means no JSON schema adapter yet: the crate consumes typed Rust
  values, so public schema compatibility is still unimplemented and must not be
  claimed from this slice.
- The 6-node experiment bound is far below the 128-state cap's practical ceiling;
  larger plans are rejected before enumeration, and moderate plans can exceed the
  state bound and become `inconclusive` (exercised by `oversized_graph`).

## Alternatives considered

- **Compile inside `cegar-core`.** Rejected: it would mix a domain policy
  interpretation into the audited graph kernel and imply the core validates
  semantics it does not.
- **Accept a JSON plan through a dependency.** Deferred: CI is offline and the
  schema is still `*.proposed.json`; a dependency decision belongs to the
  schema/contract review, not this experiment.
- **Reuse the compiler to build the oracle.** Rejected: circular.
- **Trust the plan's declared stage output.** Rejected by the design and by an
  explicit fixture (`ambiguous_stage_output`): both bindings are enumerated and
  the second one reaches a modeled violation.
- **Treat a publish without a staged candidate as a pruned edge.** Rejected: it
  would hide a real unsafe attempt; it is modeled as `Bad`.

## Unresolved owner decisions (unchanged by this ADR)

Digest algorithm and canonical bytes; collector authentication, key rotation and
time authority; policy language and suite applicability; cross-principal, tenant
and audience sharing; stage-output contract evidence; scheduler atomicity and
failure contracts; bounded-prefix presentation; the trust boundary for collecting
real test outcomes; and whether any of this survives promotion out of the
experiment. This prototype chooses *a* consistent synthetic policy profile for
the experiment and records it in each receipt; that choice is not a proposal for
production.

## Reversal

Delete `crates/cegar-domain`, the `domain` subcommand and the CI step. No
runtime behavior of `cegar-core` depends on this crate.
