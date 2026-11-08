//! Pinned routes, one bounded classification call, and the deterministic
//! baseline it is compared against.
//!
//! The local model is a **non-generative reranker**. It may only choose which of
//! a fixed set of already-built synthetic templates is analyzed. It cannot
//! invent a plan, supply evidence, change policy or relax a rule: the chosen
//! template is a hard-coded value in this module and the verifier's verdict is
//! whatever that template's graph yields.

use cegar_domain::{AnalysisOutcome, CompileLimits, Context, Plan, analyze};

use crate::http::{AdapterError, Endpoint, body_text};
use crate::json::{self, Value};

/// The one model this adapter is allowed to talk to.
pub const MODEL_NAME: &str = "tinybert";
/// Bound on the natural-language input the adapter will forward.
pub const MAX_OBSERVATION_BYTES: usize = 512;
/// Bound on the number of candidate documents in one request.
pub const MAX_CANDIDATES: usize = 8;

/// A pinned experimental route. The variant set is closed on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Route {
    /// Inspect the manifest only; nothing is staged, tested or published.
    PreflightOnly,
    /// Stage a patch, run the registered suite, then publish the artifact.
    StageTestPublish,
    /// The request needs evidence this experiment does not model.
    Blocked,
}

impl Route {
    pub const ALL: [Self; 3] = [Self::PreflightOnly, Self::StageTestPublish, Self::Blocked];

    pub const fn tag(self) -> &'static str {
        match self {
            Self::PreflightOnly => "preflight_only",
            Self::StageTestPublish => "stage_test_publish",
            Self::Blocked => "blocked",
        }
    }

    /// The exact text sent as a candidate document; changing it changes every
    /// ranking and therefore needs a new receipt.
    pub const fn description(self) -> &'static str {
        match self {
            Self::PreflightOnly => {
                "inspect only: read the manifest and report metadata, without staging, testing or publishing"
            }
            Self::StageTestPublish => {
                "stage a patch, run the registered test suite, then publish the artifact"
            }
            Self::Blocked => {
                "blocked: the request needs evidence or authority this system cannot supply"
            }
        }
    }

    /// The pinned synthetic template for this route. Deterministic: no model
    /// output, clock or environment contributes to it.
    pub fn template(self) -> (Plan, Context) {
        use cegar_domain::fixtures;
        match self {
            Self::PreflightOnly => {
                let mut context = fixtures::context();
                context.staged = vec![fixtures::candidate_a()];
                let nodes = vec![fixtures::test_node(
                    1,
                    &[],
                    cegar_domain::InputRef::Initial(fixtures::candidate_a()),
                    fixtures::suite_s1(),
                )];
                (fixtures::plan(nodes), context)
            }
            Self::StageTestPublish => fixtures::safe_chain(),
            Self::Blocked => fixtures::missing_observation(),
        }
    }

    /// Verdict this route's template yields under the pinned verifier settings.
    pub fn verdict(self) -> &'static str {
        let (plan, context) = self.template();
        let analysis = analyze(&plan, &context, CompileLimits::default());
        match analysis.outcome {
            AnalysisOutcome::InvalidInput(_) => "invalid_input",
            AnalysisOutcome::StaleSnapshot(_) => "stale_snapshot",
            AnalysisOutcome::NeedsEvidence(_) => "needs_evidence",
            AnalysisOutcome::Inconclusive(_) => "inconclusive",
            AnalysisOutcome::ConcreteCounterexample { .. } => "concrete_counterexample",
            AnalysisOutcome::VerifiedWithinModel => "verified_within_model",
        }
    }
}

/// Where a claim came from. Advisory metadata, never evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct ClaimSource {
    pub model: String,
    pub endpoint: String,
    pub latency_ms: f64,
    /// Ranked `(candidate index, score)` pairs, best first.
    pub ranked: Vec<(u32, f32)>,
}

/// An untrusted assertion about a pinned identifier set.
#[derive(Clone, Debug, PartialEq)]
pub struct Claim<T> {
    pub value: T,
    pub source: ClaimSource,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RankedEntry {
    pub index: u32,
    pub score: f32,
}

/// Build the exact request body. Strings are escaped here, so a caller cannot
/// inject JSON structure through the observation text.
pub fn build_rerank_request(
    model: &str,
    query: &str,
    documents: &[&str],
    top_n: usize,
) -> Result<String, AdapterError> {
    if query.is_empty() || query.len() > MAX_OBSERVATION_BYTES {
        return Err(AdapterError::Malformed("query size"));
    }
    if documents.is_empty() || documents.len() > MAX_CANDIDATES {
        return Err(AdapterError::Malformed("document count"));
    }
    if top_n == 0 || top_n > documents.len() {
        return Err(AdapterError::Malformed("top_n"));
    }
    let documents: Vec<String> = documents
        .iter()
        .map(|text| format!("\"{}\"", escape_json(text)))
        .collect();
    Ok(format!(
        "{{\"model\":\"{}\",\"query\":\"{}\",\"documents\":[{}],\"top_n\":{top_n}}}",
        escape_json(model),
        escape_json(query),
        documents.join(",")
    ))
}

fn escape_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}

/// Decode the single response shape this adapter understands.
///
/// Rejects: wrong model tag, wrong number of results, unknown keys, duplicate
/// indices, out-of-range indices and non-finite scores.
pub fn decode_rerank(
    raw: &[u8],
    expected_model: &str,
    document_count: usize,
) -> Result<Vec<RankedEntry>, AdapterError> {
    let text = body_text(raw)?;
    let value = json::parse(text.as_bytes()).map_err(|error| AdapterError::Malformed(error.0))?;
    if value.object_len() != Some(2) {
        return Err(AdapterError::Malformed("response keys"));
    }
    let model = value
        .field("model")
        .and_then(Value::as_str)
        .ok_or(AdapterError::Malformed("model field"))?;
    if model != expected_model {
        return Err(AdapterError::Malformed("unexpected model"));
    }
    let results = value
        .field("results")
        .and_then(Value::as_array)
        .ok_or(AdapterError::Malformed("results field"))?;
    if results.len() != document_count {
        return Err(AdapterError::Malformed("result count"));
    }

    let mut ranked = Vec::with_capacity(results.len());
    for entry in results {
        if entry.object_len() != Some(2) {
            return Err(AdapterError::Malformed("result keys"));
        }
        let index = entry
            .field("index")
            .and_then(Value::as_f64)
            .ok_or(AdapterError::Malformed("index field"))?;
        if index.fract() != 0.0 || index < 0.0 || index >= document_count as f64 {
            return Err(AdapterError::Malformed("index range"));
        }
        let score = entry
            .field("score")
            .and_then(Value::as_f64)
            .ok_or(AdapterError::Malformed("score field"))?;
        if !score.is_finite() || score.abs() > f64::from(f32::MAX) {
            return Err(AdapterError::Malformed("score range"));
        }
        let index = index as u32;
        if ranked
            .iter()
            .any(|entry: &RankedEntry| entry.index == index)
        {
            return Err(AdapterError::Malformed("duplicate index"));
        }
        ranked.push(RankedEntry {
            index,
            score: score as f32,
        });
    }
    // Deterministic order: score descending, then index ascending.
    ranked.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then(left.index.cmp(&right.index))
    });
    Ok(ranked)
}

/// Ask the local model to classify the request among the pinned routes.
pub fn classify_route(
    endpoint: &Endpoint,
    observation: &str,
) -> Result<Claim<Route>, AdapterError> {
    let documents: Vec<&str> = Route::ALL.iter().map(|route| route.description()).collect();
    let body = build_rerank_request(MODEL_NAME, observation, &documents, documents.len())?;
    let (raw, elapsed) = endpoint.post_json("/v1/rerank", &body)?;
    let ranked = decode_rerank(&raw, MODEL_NAME, documents.len())?;
    let best = *ranked
        .first()
        .ok_or(AdapterError::Malformed("empty ranking"))?;
    // The index was validated against the pinned candidate set above.
    let route = Route::ALL[best.index as usize];
    Ok(Claim {
        value: route,
        source: ClaimSource {
            model: MODEL_NAME.to_owned(),
            endpoint: endpoint.address().to_string(),
            latency_ms: elapsed.as_secs_f64() * 1_000.0,
            ranked: ranked
                .iter()
                .map(|entry| (entry.index, entry.score))
                .collect(),
        },
    })
}

/// Deterministic, model-free baseline. Conservative by construction: an
/// unrecognized request is treated as blocked rather than promoted.
pub fn baseline_route(observation: &str) -> Route {
    let text = observation.to_ascii_lowercase();
    if text.contains("blocked") || text.contains("missing evidence") {
        Route::Blocked
    } else if text.contains("inspect only") || text.contains("preflight") {
        Route::PreflightOnly
    } else if text.contains("stage") || text.contains("publish") || text.contains("test") {
        Route::StageTestPublish
    } else {
        Route::Blocked
    }
}

/// Outcome of running both presentations through the same verifier.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    /// The model's claim, or `None` when inference was unavailable or rejected.
    pub model: Option<Claim<Route>>,
    /// Why the model claim is absent, when it is.
    pub model_error: Option<String>,
    pub baseline: Route,
    pub model_verdict: Option<&'static str>,
    pub baseline_verdict: &'static str,
    pub agree: bool,
    pub notes: Vec<&'static str>,
}

/// Run one model-assisted path and one model-free path through the same pinned
/// templates and verifier. Inference failure never blocks the baseline.
pub fn evaluate(endpoint: Option<&Endpoint>, observation: &str) -> Evaluation {
    let baseline = baseline_route(observation);
    let outcome = endpoint.map(|endpoint| classify_route(endpoint, observation));
    let (model, model_error) = match outcome {
        Some(Ok(claim)) => (Some(claim), None),
        Some(Err(error)) => (None, Some(error.to_string())),
        None => (None, None),
    };
    let model_verdict = model.as_ref().map(|claim| claim.value.verdict());
    let baseline_verdict = baseline.verdict();
    Evaluation {
        agree: model.as_ref().is_some_and(|claim| claim.value == baseline),
        model,
        model_error,
        baseline,
        model_verdict,
        baseline_verdict,
        notes: vec![
            "a model claim selects a pinned template; it is not evidence and grants no authority",
            "both paths use the same verifier settings and the same synthetic policy profile",
            "every receipt reports execution_authorized = false",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_are_pinned_and_deterministic() {
        for route in Route::ALL {
            let (plan, context) = route.template();
            let again = route.template();
            assert_eq!(plan, again.0);
            assert_eq!(context, again.1);
        }
        assert_eq!(Route::PreflightOnly.verdict(), "verified_within_model");
        assert_eq!(Route::StageTestPublish.verdict(), "verified_within_model");
        assert_eq!(Route::Blocked.verdict(), "needs_evidence");
    }

    #[test]
    fn request_building_escapes_the_observation() {
        let body = build_rerank_request(MODEL_NAME, "quote\" and \\ backslash", &["a", "b"], 2)
            .expect("body");
        assert!(body.contains("quote\\\" and \\\\ backslash"));
        let value = crate::json::parse(body.as_bytes()).expect("valid json");
        assert_eq!(value.object_len(), Some(4));
    }

    #[test]
    fn baseline_is_conservative_and_deterministic() {
        assert_eq!(
            baseline_route("please publish the artifact"),
            Route::StageTestPublish
        );
        assert_eq!(
            baseline_route("inspect only, no changes"),
            Route::PreflightOnly
        );
        assert_eq!(baseline_route("blocked: missing evidence"), Route::Blocked);
        assert_eq!(baseline_route("do something unclear"), Route::Blocked);
        assert_eq!(
            baseline_route("do something unclear"),
            baseline_route("do something unclear")
        );
    }

    #[test]
    fn decoding_rejects_shapes_it_does_not_understand() {
        let good =
            br#"{"model":"tinybert","results":[{"index":0,"score":1.0},{"index":1,"score":2.0}]}"#;
        let ranked = decode_rerank(good, MODEL_NAME, 2).expect("decode");
        assert_eq!(
            ranked[0],
            RankedEntry {
                index: 1,
                score: 2.0
            }
        );
        assert_eq!(
            ranked[1],
            RankedEntry {
                index: 0,
                score: 1.0
            }
        );

        let wrong_model = br#"{"model":"other","results":[{"index":0,"score":1.0}]}"#;
        assert!(matches!(
            decode_rerank(wrong_model, MODEL_NAME, 1),
            Err(AdapterError::Malformed("unexpected model"))
        ));

        let short = br#"{"model":"tinybert","results":[{"index":0,"score":1.0}]}"#;
        assert!(matches!(
            decode_rerank(short, MODEL_NAME, 2),
            Err(AdapterError::Malformed("result count"))
        ));

        let out_of_range = br#"{"model":"tinybert","results":[{"index":5,"score":1.0}]}"#;
        assert!(matches!(
            decode_rerank(out_of_range, MODEL_NAME, 1),
            Err(AdapterError::Malformed("index range"))
        ));

        let duplicate =
            br#"{"model":"tinybert","results":[{"index":0,"score":1.0},{"index":0,"score":2.0}]}"#;
        assert!(matches!(
            decode_rerank(duplicate, MODEL_NAME, 2),
            Err(AdapterError::Malformed("duplicate index"))
        ));

        let extra = br#"{"model":"tinybert","results":[{"index":0,"score":1.0,"note":"x"}]}"#;
        assert!(matches!(
            decode_rerank(extra, MODEL_NAME, 1),
            Err(AdapterError::Malformed("result keys"))
        ));
    }

    #[test]
    fn evaluation_without_inference_still_runs_the_baseline() {
        let evaluation = evaluate(None, "stage a patch and publish");
        assert!(evaluation.model.is_none());
        assert!(evaluation.model_error.is_none());
        assert_eq!(evaluation.baseline, Route::StageTestPublish);
        assert_eq!(evaluation.baseline_verdict, "verified_within_model");
        assert!(!evaluation.agree);
    }
}
