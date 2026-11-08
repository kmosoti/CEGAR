# Evidence references, not raw dumps

This directory contains reviewed manifests and interpretation, not captured logs, private host snapshots or model artifacts. Store sanitized raw output as issue-linked CI artifacts or attachments. Reusable synthetic regression fixtures belong in source control because they are test inputs, not transient output.

## Records

- [Bounded-core validation](0002-bounded-core-validation.md): exact compiler, tested source/merge identities, passing commands, coverage, failed runs, artifact hashes and expiry. Tracks [issue #2](https://github.com/kmosoti/CEGAR/issues/2) and [PR #3](https://github.com/kmosoti/CEGAR/pull/3).
- [Artifact-domain semantics validation](0006-artifact-semantics-validation.md): local unchanged-core regression gates and the boundary between a proposed design and future domain verification. Tracks [issue #5](https://github.com/kmosoti/CEGAR/issues/5).

A new manifest must include provenance, collection command/date, input/source identities, scope, uncertainty, interpretation, falsifiers and verified artifact links. Do not claim an upload happened without read-back confirmation. GitHub Actions artifacts expire; the committed test procedures are the reproduction path, not a promise that every raw artifact is retained forever.
