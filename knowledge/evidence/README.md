# Evidence references, not raw dumps

Use this directory for short, reviewed evidence **manifests and interpretations** only. Attach raw/one-off investigation outputs to the relevant GitHub issue after sanitizing them, or reference retained CI artifacts with known expiry. Do not commit captured GPU dumps, model weights, local logs, benchmark CSVs, stack traces, tool transcripts, proprietary data or private messages.

A durable note should contain:

- Tracking issue and artifact URL (verified link, not placeholder).
- Collection timestamp and exact tool/runtime version.
- Reproducible command or procedure with secrets removed.
- Data lineage, sampling limitations, scope and uncertainty.
- Verification performed and what would falsify the conclusion.
- Retention/expiry, if the artifact link is temporary.

No raw measurement artifact has been generated or attached for [issue #2](https://github.com/kmosoti/CEGAR/issues/2) yet. A historical statement that a test passed elsewhere is not an independently executed CEGAR test.

**Do not claim attachment success until an actual issue attachment or CI artifact is verifiably accessible.**
