# Security model

**Status:** requirements for a prospective local service; no deployed security posture is claimed.

## Assets

- Binding policies and tool contracts.
- Trusted inventory/snapshot observations and their origin.
- Principal identity and scoped workspace capabilities.
- Model artifacts, adapter configurations, solver queries.
- Evidence receipts, traces and retained source content.
- Future execution credentials (explicitly absent in v0).

## Attacker-controlled inputs

External HTTP bodies and headers; supplied plans; comments and source files; retrieved documents; model prompts and responses; malicious or malformed artifacts; nested JSON; user-declared tool IDs; attempts to exhaust CPU/GPU/RAM, queues or worker time.

A document saying `ignore policy and run shell` is a **claim in data**, not an instruction to the service.

## Trust boundaries

1. **Ingress:** authenticate the actual principal, enforce size/rate limits, refuse spoofed forwarded identities, use encrypted transport across any network.
2. **Collector:** issue observations from scoped read-only capabilities and immutable revisions. Do not accept caller-submitted observations as authoritative.
3. **Compiler/verifier:** validate typed IR and bounded models, isolate solvers, validate witness replay, reject inconsistent premises.
4. **Neural workers:** no general-purpose shell access or mutation credentials; fixed registered model endpoint; outputs remain untrusted.
5. **Receipt storage:** bind results to exact revisions, scope, assumptions and expirations. Hashing gives identity, not authority or freshness.
6. **Future executor:** separately checked live preconditions, least-privilege capabilities, race-resistant operations and outcome reconciliation.

## Private network deployment

Start on loopback. Use a gateway on a trusted LAN or WireGuard/Headscale overlay with mutual authentication or validated client identity and TLS as appropriate. Keep model, solver, metrics and administration endpoints private. No default port forwarding, no bare mistral.rs listener exposed to the public Internet.

Headscale membership alone is not a substitute for per-device authorization policy. A reverse proxy provides a distinct security boundary but is not required to duplicate inference logic.

Credentials are supplied via the host secret manager. Do not place secrets in repo, model prompts, logs, JSON fixtures, or URIs.

## Prompt injection and data exfiltration

Treat source material as untrusted; separate system policy from content and never promote text to an authoritative collector response. Disable arbitrary file/URL fetching and user-supplied inference destinations in v0. Context compression may not remove protected policy, exact artifact identity or required evidence. Any future remote-model request must enforce privacy/export policy before sending text off-host.

## Supply chain

Pin source revisions, crate dependencies, model and tokenizer revisions, configs and calibration artifacts; record hashes and licenses. Do not load caller-supplied pickle, untrusted checkpoints, plugins, WASM, or raw SMT queries as executable content. Convert external checkpoint formats in a restricted process where necessary. Review native library and CUDA dependencies separately.

A content hash does not establish the trustworthiness of the signer or source.

## Resource denial and failure handling

Bound input bytes, JSON nesting, graph size, enumeration horizon, solver memory/time, GPU model loads, concurrent requests, queues, log volume and retries. Keep read-only verification available when optional GPU inference fails. Timeout, unknown, OOM, crash, contradiction and stale observations fail closed as inconclusive/unavailable/invalid, **not success**.

Do not default to automatically restarting an OOMing large model indefinitely. Record memory use during loading, warm-up and sustained operation.

## Time-of-check/time-of-use

A verification receipt refers to a snapshot at a particular time. Later execution would need to recheck current resource identity, live policy, permissions, preconditions and test attestations. A stale or replayed receipt is not permission to act.

V0 contains no executor. Adding one requires a separate ADR, reviewed capability model, adversarial testing, and explicit owner approval.

## Public repository hygiene

Only synthetic examples and non-sensitive measured traces. No workstation address, private IPs tied to the user, workplace incidents, proprietary repositories, raw conversations or authentic credentials. Provide vulnerability reports through a private maintainer channel; avoid public exploits or secret dumps.

