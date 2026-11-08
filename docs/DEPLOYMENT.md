# Deployment model: local CEGAR service

**Status:** architectural candidate. OS-specific details deliberately deferred until the G15 host is ready and characterized.

## Target hardware assumptions

Initial research envelope: Dell G15 with Ryzen 7 5800H, NVIDIA RTX 3050 Ti Laptop GPU with 4 GiB VRAM, and approximately 7.1 GiB OS-visible RAM. These are previously reported configuration values, not live measured budgets. No claims are made about negotiated PCIe bandwidth, current driver, CUDA compatibility, thermal headroom, display routing, or RAM channel configuration.

The verifier must work **without GPU inference**. A model merely assists with classification and untrusted candidate generation.

## Logical deployment

```text
Trusted LAN client / authorized private overlay peer
                   |
         TLS + authentication gateway
                   |
       CEGAR Rust service [private]
        /                 |                \
 trusted snapshot   bounded verifier   optional model worker
 collector          process            mistral.rs /v1/systemone
       \                 |                /
        +--------- receipt/evidence store+
                   |
         review result / diagnostics
                   |
         NO EXECUTION CREDENTIALS IN V0
```

The API, verifier, and GPU worker should have separate failure domains. The GPU worker must not contain host mutation credentials; the verifier must remain available when neural inference fails. Internal worker endpoints bind to loopback or a private local IPC channel.

Do not expose raw mistral.rs or solver interfaces directly to untrusted LAN devices. Overlay access is not equivalent to per-client authorization.

## Model admission

Optional first experiment: **LiquidAI d1-2B**, text-only question templates through mistral.rs `POST /v1/systemone`. The upstream docs describe groups of up to 16 questions about a state; that is not the same as 16 independent HTTP requests or an assurance of available batch capacity.

A complete runtime profile requires:
- exact model revision, tokenizer/config/calibration and any projector artifacts;
- engine source and binary revision, quantization and kernel build;
- actual loading and steady-state RAM/VRAM peaks;
- source-text serialization and span-offset parity tests;
- held-out question-level error rates and calibration;
- warm latency and sustained thermal qualification.

Do not assume GGUF file size equals resident memory. Do not enable simultaneous co-residency of d1, GLiNER2, LLMLingua and a planner by default.

## Memory and capacity bounds

For GPU memory, require:

```text
other_processes + driver_context + model_weights + auxiliary_artifacts
+ peak_activations + workspaces + cached_graphs + allocator_slack
+ safety_headroom <= measured_available_VRAM
```

Track host RAM independently, including model-loader peaks, solver processes, native libraries and the API. Do not add free VRAM to free host RAM as though they form one pool.

Reserve memory for running tasks before accepting new tasks. A configured maximum batch size does not create actual ready work.

## Scheduling and batching

Keep three axes separate:
1. Multiple **questions about one state** grouped by d1.
2. Independent **verification requests** in the bounded API queue.
3. Optional **planner generations**, if a generative model is ever introduced.

Record dependency-ready request counts before investing in batching. A single sequential agent loop may benefit more from deterministic extraction and protected context reuse than from large batches.

Interactive and background admission may have different quotas, but a gateway cannot preempt a batch already executing. Report queue delay and tail latency. Use per-class **quality-qualified goodput** rather than an unqualified tokens/second target for CEGAR.

## Networking and secrets

Default loopback. Private TLS/auth ingress on the selected LAN or Headscale/WireGuard overlay; deny-by-default per-client policy. No public Internet exposure by default. Host-scoped secrets, no model URL override from requests, no raw document uploads to remote inference without an export policy.

## OS integration boundary

Host service manager, isolation primitives, driver/toolchain/CUDA packaging, GPU persistence and power settings, log storage, zram/swap strategy, firewall and overlay configuration are **host-specific future work**.

The repository should expose explicit ports/config contracts without embedding NixOS, Fedora, a particular distribution, or hardware serials into the pure domain crate.

## Operational status gates

```text
Configured -> Source-audited -> Built -> Contract-tested
-> Resource-measured -> Qualified -> Ready
```

A worker that fails a gate remains unavailable. No silent fallback to unapproved quantization or new model source. A verification result must remain possible without an optional model worker.

## What to benchmark

- p50/p95/p99 request latency; CPU verifier time; d1 model stage time and queue delay.
- Model precision/quantization drift, grouping 1/4/8/16 same-state questions, false positives and abstention.
- Loading peak, warm-up peak, steady memory, clocks/temperature and valid power telemetry.
- Closed-loop concurrency and open-loop arrival tests with intended arrival timestamps to prevent coordinated omission.
- Workload-specific quality-qualified goodput, rejected/failed tasks, and evidence integrity.

No 60+ tokens/s claim is relevant to a non-generative decision worker; measure **verified useful tasks per unit time** instead.

