# xtask: Rust-only development tooling

\`xtask\` is the **developer-side** Rust runner. It is not linked into the verifier core or deployed as a network service. It replaces the former Python subprocess oracle and its custom Rust whitespace/JSON driver.

Run from repository root:

\`\`\`sh
cargo run -p xtask --locked --offline -- verify
\`\`\`

The single \`verify\` task enumerates 28,868 finite models up to 3 states, plus 1,000 reproducible labeled models (\`seed=0x4345474152\`). It compares \`cegar-core\` results with independently implemented **backward fixed-point reachability**, validates every returned action-labeled witness without calling \`cegar_core::replay\`, and checks eight invalid Model constructors.

It emits a single JSON summary and exits nonzero on the first disagreement. The first green run is [42439569614](https://github.com/kmosoti/CEGAR/actions/runs/42439569614) (28,868 exhaustive, 1,000 seeded, 24,187 replayed witnesses). The independent oracle and witness checker are ordinary Rust functions in \`src/main.rs\`, with no third-party dependencies.

These are **finite synthetic model checks**, not proof that arbitrary real infrastructure is correctly represented or that execution should be authorized. The removed custom test-transport behavior is not covered by xtask and will not be reintroduced unless a real transport contract needs testing.

Raw CI outputs belong in GitHub Actions artifacts. Reproducibility details and historical evidence: [evidence 0003](../knowledge/evidence/0003-rust-xtask-validation.md).
