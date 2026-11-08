//! Offline tests for the untrusted local-model adapter.
//!
//! A throw-away loopback listener stands in for the real service, so these tests
//! need no external model, no network route and no credentials. They assert the
//! adapter's contract: one bounded request, strict decoding, and an inference
//! failure that never blocks the deterministic baseline.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use cegar_local_model::{AdapterError, Endpoint, Route, classify_route, evaluate};

fn serve_once(response: Vec<u8>) -> (u16, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buffer = [0u8; 8192];
        let read = stream.read(&mut buffer).expect("read request");
        stream.write_all(&response).expect("write response");
        String::from_utf8_lossy(&buffer[..read]).into_owned()
    });
    (port, handle)
}

fn http_ok(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

#[test]
fn classify_route_selects_the_best_ranked_pinned_route() {
    let body = r#"{"model":"tinybert","results":[{"index":0,"score":-11.3},{"index":1,"score":9.18},{"index":2,"score":-10.8}]}"#;
    let (port, handle) = serve_once(http_ok(body));
    let endpoint = Endpoint::loopback(port, "Authorization: Bearer test").expect("endpoint");
    let claim = classify_route(&endpoint, "stage and publish the artifact").expect("claim");

    assert_eq!(claim.value, Route::StageTestPublish);
    assert_eq!(claim.source.model, "tinybert");
    assert_eq!(claim.source.ranked.len(), 3);
    assert_eq!(claim.source.ranked[0].0, 1);

    let request = handle.join().expect("server thread");
    assert!(request.starts_with("POST /v1/rerank HTTP/1.1\r\n"));
    assert!(request.contains("Authorization: Bearer test\r\n"));
    assert!(request.contains("\"model\":\"tinybert\""));
    assert!(request.contains("\"top_n\":3"));
    // The observation is escaped JSON, not concatenated raw.
    assert!(request.contains("\"query\":\"stage and publish the artifact\""));
}

#[test]
fn unknown_response_shape_is_rejected() {
    let body = r#"{"model":"tinybert","results":[{"index":0,"score":1.0}],"extra":true}"#;
    let (port, handle) = serve_once(http_ok(body));
    let endpoint = Endpoint::loopback(port, "Authorization: Bearer test").expect("endpoint");
    let error = classify_route(&endpoint, "stage and publish").expect_err("must reject");
    assert!(matches!(error, AdapterError::Malformed(_)), "{error:?}");
    handle.join().expect("server thread");
}

#[test]
fn unauthorized_status_never_becomes_a_route() {
    let response = b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n".to_vec();
    let (port, handle) = serve_once(response);
    let endpoint = Endpoint::loopback(port, "Authorization: Bearer wrong").expect("endpoint");
    let error = classify_route(&endpoint, "stage and publish").expect_err("must fail");
    assert_eq!(error, AdapterError::Status(401));
    handle.join().expect("server thread");
}

#[test]
fn oversized_response_is_refused() {
    let big = format!(
        "{{\"model\":\"tinybert\",\"results\":[{}]}}",
        "0".repeat(70_000)
    );
    let (port, handle) = serve_once(http_ok(&big));
    let endpoint = Endpoint::loopback(port, "Authorization: Bearer test").expect("endpoint");
    let error = classify_route(&endpoint, "stage and publish").expect_err("must fail");
    assert_eq!(error, AdapterError::Oversize("response"));
    handle.join().expect("server thread");
}

#[test]
fn unavailable_inference_keeps_the_baseline_path_working() {
    // Reserve a port, then drop the listener so connections are refused.
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("addr").port()
    };
    let endpoint = Endpoint::loopback(port, "Authorization: Bearer test").expect("endpoint");
    let evaluation = evaluate(Some(&endpoint), "stage a patch and publish the artifact");

    assert!(evaluation.model.is_none());
    assert!(evaluation.model_error.is_some());
    assert_eq!(evaluation.baseline, Route::StageTestPublish);
    assert_eq!(evaluation.baseline_verdict, "verified_within_model");
    assert!(!evaluation.agree);

    // The baseline alone still classifies deterministically.
    let baseline_only = evaluate(None, "stage a patch and publish the artifact");
    assert_eq!(baseline_only.baseline_verdict, evaluation.baseline_verdict);
}

#[test]
fn model_claims_never_grant_authority() {
    let body = r#"{"model":"tinybert","results":[{"index":0,"score":9.0},{"index":1,"score":1.0},{"index":2,"score":-1.0}]}"#;
    let (port, handle) = serve_once(http_ok(body));
    let endpoint = Endpoint::loopback(port, "Authorization: Bearer test").expect("endpoint");
    let evaluation = evaluate(Some(&endpoint), "blocked: missing evidence");
    handle.join().expect("server thread");

    // The model picked a route the deterministic baseline disagrees with, and the
    // only thing that changed is which pinned template was analyzed.
    assert_eq!(evaluation.model.expect("claim").value, Route::PreflightOnly);
    assert_eq!(evaluation.baseline, Route::Blocked);
    assert_eq!(evaluation.model_verdict, Some("verified_within_model"));
    assert_eq!(evaluation.baseline_verdict, "needs_evidence");
    assert!(!evaluation.agree);
    for note in &evaluation.notes {
        assert!(!note.is_empty());
    }
}
