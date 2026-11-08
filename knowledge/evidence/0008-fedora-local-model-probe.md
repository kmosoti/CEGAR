# Local model probe validation record

- Date: 2026-11-08 (US Central)
- Tracking: [Issue #7](https://github.com/kmosoti/CEGAR/issues/7)
- Change: `crates/cegar-local-model`, an untrusted bounded adapter to the
  already-running local non-generative model, plus the deterministic baseline
  and measurement path
- **Status: Executed on the Fedora workstation against the real local service.
  Not reviewed, not qualified, not a G15 result. Inference is advisory only.**

## What was actually called

| Fact | Observed value |
| --- | --- |
| Service | pre-existing `local-retrieval` gateway, loopback `127.0.0.1:1235` (already running before this work; not started, installed or reconfigured here) |
| Model | `tinybert` — MS MARCO TinyBERT reranker served by ONNX Runtime through the gateway's admitted `POST /v1/rerank` route |
| Authentication | the existing local bearer-header file, read with a bounded read and never printed |
| Capability | **non-generative**: it scores candidate strings. It cannot emit a plan, a command, an attestation or an artifact identity |
| Client | `crates/cegar-local-model`, `std`-only HTTP/1.1, loopback-only, one request per call, 5 s timeout, 8 KiB request / 64 KiB response caps, strict single-shape JSON decoder |
| Rust | `rustc 1.100.0-beta.12 (b0aaa0fca 2026-11-06)`, pinned toolchain `beta-2026-11-07` |

The model's entire role in this experiment: choose one of **three pinned
templates** (`preflight_only`, `stage_test_publish`, `blocked`). The templates,
the synthetic policy profile and the verifier are deterministic Rust; a claim
selects a template and can change nothing inside it.

## Measured results

Command (once per observation):

```sh
cargo run -q -p cegar-local-model --locked --offline --bin probe -- \
  --header "$HOME/.config/local-inference/authorization.header" \
  --runs 9 --observation "<text>"
```

### Observation A — the modeled happy path

`"stage a patch, run the registered test suite, then publish the artifact"`

```json
{"model":"tinybert","runs":9,"errors":0,"p50_ms":1035.189,"p95_ms":1057.464,"min_ms":1022.809,"max_ms":1057.464,"warmup_ms":1041.915,"peak_rss_kib":3420,"identical_route_every_run":true,"routes":["stage_test_publish", ...],"verdicts":["verified_within_model", ...],"baseline":"stage_test_publish","baseline_verdict":"verified_within_model","execution_authorized":false}
```

Model route `stage_test_publish`, baseline route `stage_test_publish`, agreement
`true`.

### Observation B — evidence that this system cannot supply

`"blocked: missing evidence"`

```json
{"runs":5,"errors":0,"p50_ms":1037.293,"p95_ms":1046.406,"warmup_ms":1049.217,"peak_rss_kib":3124,"routes":["blocked", ...],"verdicts":["needs_evidence", ...],"baseline":"blocked","baseline_verdict":"needs_evidence","execution_authorized":false}
```

Both paths agree on `blocked`; the template's verdict is `needs_evidence`, which
is the required fail-closed answer.

### Observation C — where the model and the baseline disagree

`"read the manifest and report metadata"`

```json
{"runs":5,"errors":0,"p50_ms":1039.923,"p95_ms":1049.770,"warmup_ms":1040.990,"peak_rss_kib":3148,"routes":["preflight_only", ...],"verdicts":["verified_within_model", ...],"baseline":"blocked","baseline_verdict":"needs_evidence","execution_authorized":false}
```

This is the most important measurement in the record. The baseline treats the
request as unclassifiable and answers `blocked` / `needs_evidence`. The model
picks `preflight_only`, whose *pinned template* verifies.

Interpretation: a verified verdict here means "the fixed inspect-only plan has no
reachable modeled violation". It does **not** mean "the user's request was
understood, authorized or performed". A model claim changed which scenario was
analyzed, exactly as an untrusted heuristic may, and nothing else. Any design that
let a claim select the *policy* rather than a *template* would be unsound; that is
why the route set is closed at three variants and the policy profile is a constant
in `Route::template()`.

## Resource observations

- Harness peak RSS: 3.0–3.4 MiB (`VmHWM` of the probe process).
- GPU: `nvidia-smi` reported 181 MiB used before and after every run, 0 %
  utilization. **The inference service's own VRAM is not attributed here** — it
  is a separate process the adapter cannot inspect without privileges. The flat
  reading is consistent with CPU execution of these small ONNX models but does
  not prove it.
- Latency: p50 ≈ 1.03–1.04 s, p95 ≈ 1.05 s, min ≈ 1.02 s, max ≈ 1.06 s across 19
  measured calls; warm-up ≈ 1.04 s. Nearly all of that time is the gateway's
  per-request worker (fresh process, model load, 90 s worker timeout budget), not
  the adapter. Queueing under concurrency was not measured; the gateway admits
  one request at a time.
- No inference errors occurred (`errors: 0`); the route was identical on every
  run of a given observation, including the warm-up.

## Failure modes exercised

- **401 before normalization:** the header file contains a complete
  `Authorization: …` line, so the first implementation sent a duplicated header
  and the gateway correctly refused. Fixed by normalizing to exactly one header
  line; an explicit test now asserts the request contains `Authorization:` once.
- **401 with a bare credential:** rejected by the gateway as well; the adapter
  rejects empty/oversized/newline-bearing headers before sending.
- **Connection refused:** covered by an offline test; the pipeline still runs the
  deterministic baseline and reports `model_error` without blocking.
- **Malformed / oversize / wrong-model / duplicate / out-of-range responses:**
  covered by offline tests against a throw-away loopback listener; each is
  rejected instead of coerced.

## Not measured, not claimed

- No generative model was used; no plan, command or artifact identity was ever
  produced by a model.
- No real observation, attestation or collector exists; every initial fact
  remains a synthetic fixture.
- No concurrency, queue-depth, cold-start-after-eviction or long-run memory
  behavior was measured.
- No G15 deployment, quantization choice or host tuning is implied.

## Reproduction

```sh
cargo test -p cegar-local-model --locked --offline          # offline contract tests
cargo run -q -p cegar-local-model --locked --offline --bin probe -- \
  --header "$HOME/.config/local-inference/authorization.header" --runs 9
```

CI does **not** run the probe: no hosted runner has this loopback service. CI
compiles the adapter, runs its offline contract tests, and runs the domain
reference check — [run 42522675527](https://github.com/kmosoti/CEGAR/actions/runs/42522675527)
on `505d602de307d820baf3bc2248efc94c14cd7a78`, all gates green. The local run
above is the evidence that the real inference path works; the CI run is evidence
that the offline contract holds. Neither substitutes for the other.
