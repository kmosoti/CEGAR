# cegar-local-model (experimental, untrusted)

**Advisory only. Not a router, not a planner, not evidence and not an authority.**
This crate adapts one *locally running, non-generative* model into the
[Issue #7](https://github.com/kmosoti/CEGAR/issues/7) experiment. Its output is a
[`Claim`](src/route.rs): an untrusted assertion about which of three pinned
synthetic templates should be analyzed. The verifier's answer comes from
[`cegar-domain`](../cegar-domain/README.md); the model cannot change it.

## What the adapter may do

- One bounded `POST /v1/rerank` to `127.0.0.1:<port>` with the local bearer header.
- Ask the reranker to score exactly the three pinned `Route` descriptions.
- Report the best-scoring route as a `Claim<Route>` carrying the model name,
  endpoint, latency and the full ranking.

## What it may never do

- Target a non-loopback address (rejected at construction).
- Send more than 8 KiB, accept more than 64 KiB, follow redirects, use a proxy,
  retry, or decode chunked transfer encoding.
- Turn a malformed, oversized, wrong-model, duplicate-index or out-of-range
  response into a route: those are errors, not approximate answers.
- Invent a route: the variant set is closed at three, and an unrecognized request
  becomes `blocked` in the deterministic baseline.

## The decisive property

A model claim selects *which pinned template* is verified. It never selects the
policy, the evidence, the suite requirement or the destination. The templates in
`Route::template()` are hard-coded Rust built from synthetic fixtures, so the
worst a claim can do is cause the experiment to analyze a different fixed
scenario — it can make a request that the baseline treats as `blocked` land on a
template that verifies. That is exactly why every receipt says
`execution_authorized = false`, and why the measured divergence case is recorded
in [evidence 0008](../../knowledge/evidence/0008-fedora-local-model-probe.md).

## Usage

```sh
# offline contract tests (no model, no network route, throw-away loopback server)
cargo test -p cegar-local-model --locked --offline

# one measured real call against the existing local service
cargo run -q -p cegar-local-model --locked --offline --bin probe -- \
  --header "$HOME/.config/local-inference/authorization.header" \
  --runs 9 --observation "stage a patch, then publish"
```

The probe prints a single-line JSON measurement record and a second JSON line
comparing the model route with the deterministic baseline. `--runs` is bounded to
64. CI does not run the probe: no hosted runner has the loopback service.

## Measured on the Fedora workstation

19 measured calls across three observations: p50 ≈ 1.03 s, p95 ≈ 1.06 s, zero
inference errors, probe peak RSS 3.0–3.4 MiB. The model is a TinyBERT reranker
(not generative). Details, divergences and non-claims are in
[evidence 0008](../../knowledge/evidence/0008-fedora-local-model-probe.md).
