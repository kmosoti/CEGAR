//! Bounded measurement of one real local-model path.
//!
//! Reports only what this process can actually observe: per-request latency,
//! peak resident memory of the harness process, and the verdict each selected
//! template produced. It makes **no claim** about the memory used by the
//! inference service, which runs in a separate process it cannot inspect
//! without privileges.

use std::time::Instant;

use crate::http::{AdapterError, Endpoint};
use crate::route::{Claim, Route, baseline_route, classify_route, evaluate};

/// Bounds on a probe run.
pub const MAX_RUNS: usize = 64;

/// Percentile helper over already-collected samples (milliseconds).
pub fn percentile(samples: &[f64], percentile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((percentile / 100.0) * sorted.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[index]
}

/// Peak resident set size of this process in KiB, when the host exposes it.
pub fn peak_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            let digits: String = rest.chars().filter(char::is_ascii_digit).collect();
            return digits.parse().ok();
        }
    }
    None
}

/// One measured probe.
#[derive(Clone, Debug, PartialEq)]
pub struct ProbeReport {
    pub runs: usize,
    pub warmup_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub peak_rss_kib: Option<u64>,
    /// Route the model selected on each measured run (by tag).
    pub routes: Vec<&'static str>,
    /// Verdict of the selected template on each measured run.
    pub verdicts: Vec<&'static str>,
    /// Route the deterministic baseline selects for the same observation.
    pub baseline: &'static str,
    pub baseline_verdict: &'static str,
    pub identical_route_every_run: bool,
    pub errors: usize,
    pub observation: String,
}

/// Run one warmup call and `runs` measured calls, then summarize.
pub fn probe(
    endpoint: &Endpoint,
    observation: &str,
    runs: usize,
) -> Result<ProbeReport, AdapterError> {
    if runs == 0 || runs > MAX_RUNS {
        return Err(AdapterError::Malformed("runs bound"));
    }
    // Warm-up is recorded separately so the distribution is not polluted by
    // first-call effects, but it is still bounded to a single request.
    let warmup = Instant::now();
    let first: Claim<Route> = classify_route(endpoint, observation)?;
    let warmup_ms = warmup.elapsed().as_secs_f64() * 1_000.0;

    let mut samples = Vec::with_capacity(runs);
    let mut routes = Vec::with_capacity(runs);
    let mut verdicts = Vec::with_capacity(runs);
    let mut errors = 0usize;
    for _ in 0..runs {
        let started = Instant::now();
        match classify_route(endpoint, observation) {
            Ok(claim) => {
                samples.push(started.elapsed().as_secs_f64() * 1_000.0);
                routes.push(claim.value.tag());
                verdicts.push(claim.value.verdict());
            }
            Err(_) => errors += 1,
        }
    }
    if samples.is_empty() {
        return Err(AdapterError::Malformed("no successful measured runs"));
    }
    let identical =
        routes.iter().all(|route| *route == routes[0]) && routes[0] == first.value.tag();

    let baseline = baseline_route(observation);
    let evaluation = evaluate(Some(endpoint), observation);

    Ok(ProbeReport {
        runs: samples.len(),
        warmup_ms,
        p50_ms: percentile(&samples, 50.0),
        p95_ms: percentile(&samples, 95.0),
        min_ms: samples.iter().copied().fold(f64::INFINITY, f64::min),
        max_ms: samples.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        peak_rss_kib: peak_rss_kib(),
        routes,
        verdicts,
        baseline: baseline.tag(),
        baseline_verdict: evaluation.baseline_verdict,
        identical_route_every_run: identical,
        errors,
        observation: observation.to_owned(),
    })
}

impl ProbeReport {
    /// Single-line JSON record suitable for a CI artifact.
    pub fn to_json(&self) -> String {
        let routes: Vec<String> = self.routes.iter().map(|tag| format!("\"{tag}\"")).collect();
        let verdicts: Vec<String> = self
            .verdicts
            .iter()
            .map(|tag| format!("\"{tag}\""))
            .collect();
        format!(
            "{{\"model\":\"{}\",\"runs\":{},\"errors\":{},\"p50_ms\":{:.3},\"p95_ms\":{:.3},\"min_ms\":{:.3},\"max_ms\":{:.3},\"warmup_ms\":{:.3},\"peak_rss_kib\":{},\"identical_route_every_run\":{},\"routes\":[{}],\"verdicts\":[{}],\"baseline\":\"{}\",\"baseline_verdict\":\"{}\",\"execution_authorized\":false}}",
            crate::route::MODEL_NAME,
            self.runs,
            self.errors,
            self.p50_ms,
            self.p95_ms,
            self.min_ms,
            self.max_ms,
            self.warmup_ms,
            self.peak_rss_kib
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.identical_route_every_run,
            routes.join(","),
            verdicts.join(","),
            self.baseline,
            self.baseline_verdict,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_is_stable_and_bounded() {
        let samples = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&samples, 50.0), 3.0);
        assert_eq!(percentile(&samples, 95.0), 5.0);
        assert_eq!(percentile(&samples, 0.0), 1.0);
        assert_eq!(percentile(&[], 50.0), 0.0);
    }

    #[test]
    fn report_json_is_single_line_and_declares_no_authority() {
        let report = ProbeReport {
            runs: 2,
            warmup_ms: 1.0,
            p50_ms: 2.0,
            p95_ms: 3.0,
            min_ms: 2.0,
            max_ms: 3.0,
            peak_rss_kib: Some(1234),
            routes: vec!["blocked", "blocked"],
            verdicts: vec!["needs_evidence", "needs_evidence"],
            baseline: "blocked",
            baseline_verdict: "needs_evidence",
            identical_route_every_run: true,
            errors: 0,
            observation: "blocked".to_owned(),
        };
        let json = report.to_json();
        assert!(!json.contains('\n'));
        assert!(json.contains("\"execution_authorized\":false"));
        assert!(json.contains("\"runs\":2"));
    }
}
