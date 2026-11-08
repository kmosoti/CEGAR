//! Experimental Sans-I/O artifact-domain interpreter and deterministic compiler.
//!
//! **Status: prototype, not a service and not a policy decision.** This crate
//! turns a synthetic `stage → registered test → publish` plan plus a pinned
//! context into a bounded first-violation monitor graph for [`cegar_core`].
//! It reads no clock, performs no I/O, holds no ambient capability and grants no
//! execution authority. Every initial fact in this slice is a synthetic fixture;
//! nothing here is a trusted collector or an authentication route.
//!
//! Invariants this crate is built to keep:
//!
//! - an unsafe ready attempt is preserved as an explicit edge to a terminal
//!   `Bad` monitor, never pruned to make the remainder of a plan look safe;
//! - unknown never becomes true: absent evidence is `needs_evidence`, missing
//!   coverage is `inconclusive`, and neither is reported as verified;
//! - a hypothetical modeled test pass is not an observation and never becomes
//!   permission on its own;
//! - identity, policy, suite, snapshot and freshness bindings are compared
//!   exactly, and cross-principal evidence use needs an explicit policy rule;
//! - enumeration is deterministic and bounded; exceeding a bound is
//!   `inconclusive`, never a truncated positive result.

#![forbid(unsafe_code)]

pub mod compile;
pub mod context;
pub mod evidence;
pub mod fixtures;
pub mod plan;
pub mod rules;
pub mod types;

pub use compile::{
    Analysis, AnalysisOutcome, CompileError, CompileLimits, Compiled, LiveState, NodeStatus,
    Receipt, analyze,
};
pub use context::{Context, Observation, Policy, Provenance, StageOutputs, TestStatus};
pub use plan::{
    Diagnostic, Disposition, EvidenceRef, InputRef, Node, NodeId, NodeKind, Plan, PublishSpec,
    StageSpec, TestSpec,
};
pub use rules::Rule;
pub use types::{
    ArtifactId, AudienceId, DestinationId, Digest, DigestAlgorithm, FormatId, OperationName,
    OperationRef, PolicyRef, PrincipalId, RunId, SnapshotRef, SuiteName, SuiteRef, TenantId,
};
