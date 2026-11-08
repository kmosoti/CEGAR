//! Untrusted, bounded adapter to one locally running non-generative model.
//!
//! **Boundary.** Nothing here may create evidence or authority. The adapter
//! produces [`route::Claim`] values only: advisory assertions about which of a
//! fixed, pinned set of synthetic templates to analyze. The templates, the
//! policy profile and the verifier are all deterministic Rust in
//! [`cegar_domain`]; a model claim selects one of them and cannot alter it.
//!
//! **Bounds.** Loopback only, one request per call, no redirects or proxies,
//! `std`-only HTTP/1.1 with an explicit timeout, hard request/response byte caps,
//! and a strict single-shape JSON decoder that rejects anything it does not fully
//! understand. Inference failure never blocks the deterministic baseline.
//!
//! This crate is not a router, not a planner and not an agent framework: the
//! route set is closed at three variants, and an unrecognized request becomes
//! `blocked` rather than a new capability.

#![forbid(unsafe_code)]

pub mod http;
pub mod json;
pub mod probe;
pub mod route;

pub use http::{AdapterError, Endpoint};
pub use probe::{ProbeReport, probe};
pub use route::{
    Claim, ClaimSource, Evaluation, MODEL_NAME, RankedEntry, Route, baseline_route, classify_route,
    decode_rerank, evaluate,
};
