//! Mutation tests: each one removes a single modeled obligation, branch or
//! input and requires the verdict to change. A compiler that silently drops an
//! unsafe attempt, or a verifier that cannot see it, fails at least one of these.

use cegar_domain::compile::Analysis;
use cegar_domain::fixtures;
use cegar_domain::{AnalysisOutcome, CompileLimits, Plan, Rule, analyze};

fn run(plan: &Plan, context: &cegar_domain::Context) -> Analysis {
    analyze(plan, context, CompileLimits::default())
}

fn rules(analysis: &Analysis) -> Vec<Rule> {
    match &analysis.outcome {
        AnalysisOutcome::ConcreteCounterexample { rules, .. } => rules.clone(),
        other => panic!("expected a counterexample, got {}", other.tag()),
    }
}

#[test]
fn retargeting_the_publish_subject_removes_the_violation() {
    let (plan, context) = fixtures::subject_mismatch();
    assert_eq!(rules(&run(&plan, &context)), vec![Rule::SubjectMismatch]);

    // Same plan, but the publish now names the attested subject.
    let mut repaired = plan.clone();
    for node in &mut repaired.nodes {
        if let cegar_domain::NodeKind::Publish(spec) = &mut node.kind {
            spec.subject = cegar_domain::InputRef::Initial(fixtures::candidate_a());
        }
    }
    let analysis = run(&repaired, &context);
    assert_eq!(analysis.outcome.tag(), "verified_within_model");
    let receipt = analysis.receipt.expect("receipt");
    assert_eq!(receipt.bad_edges, 0);
}

#[test]
fn removing_one_permitted_stage_output_removes_the_bad_branch() {
    let (plan, context) = fixtures::ambiguous_stage_output();
    assert_eq!(rules(&run(&plan, &context)), vec![Rule::SubjectMismatch]);

    let mut narrowed = context.clone();
    narrowed.stage_outputs[0].candidates = vec![fixtures::candidate_a()];
    let analysis = run(&plan, &narrowed);
    assert_eq!(analysis.outcome.tag(), "verified_within_model");
    assert_eq!(analysis.receipt.expect("receipt").bad_edges, 0);
}

#[test]
fn aligning_the_required_suite_removes_the_violation() {
    let (plan, context) = fixtures::wrong_required_suite();
    assert_eq!(rules(&run(&plan, &context)), vec![Rule::SuiteNotRequired]);

    let mut aligned = context.clone();
    aligned.policy.required_suite = fixtures::suite_s1();
    assert_eq!(run(&plan, &aligned).outcome.tag(), "verified_within_model");
}

#[test]
fn trusting_the_modeled_issuer_removes_the_violation() {
    let (plan, context) = fixtures::untrusted_issuer();
    assert_eq!(rules(&run(&plan, &context)), vec![Rule::IssuerUntrusted]);

    let mut trusting = context.clone();
    trusting
        .policy
        .trusted_issuers
        .push(cegar_domain::PrincipalId::new("issuer-rogue"));
    assert_eq!(run(&plan, &trusting).outcome.tag(), "verified_within_model");
}

#[test]
fn permitting_sharing_removes_the_violation() {
    let (plan, context) = fixtures::cross_principal_without_rule();
    assert_eq!(rules(&run(&plan, &context)), vec![Rule::PrincipalNotShared]);

    let mut sharing = context.clone();
    sharing.policy.evidence_sharing =
        cegar_domain::context::EvidenceSharing::PolicyPermitted(vec![
            cegar_domain::PrincipalId::new("p2"),
        ]);
    assert_eq!(run(&plan, &sharing).outcome.tag(), "verified_within_model");
}

#[test]
fn supplying_the_absent_observation_changes_only_the_classification() {
    let (plan, context) = fixtures::missing_observation();
    assert_eq!(run(&plan, &context).outcome.tag(), "needs_evidence");

    let mut supplied = context.clone();
    let mut record = fixtures::observation(cegar_domain::TestStatus::Pass, fixtures::NOW - 30);
    record.run = cegar_domain::RunId::new("run-absent");
    supplied.observations = vec![record];
    // The observation now exists, so the plan reaches the graph and verifies.
    let analysis = run(&plan, &supplied);
    assert_eq!(analysis.outcome.tag(), "verified_within_model");
    assert!(!analysis.outcome.execution_authorized());
}

#[test]
fn staging_the_missing_candidate_removes_the_violation() {
    let (plan, context) = fixtures::two_unsafe_ready_attempts();
    let first = run(&plan, &context);
    assert_eq!(first.outcome.tag(), "concrete_counterexample");

    // Stage B and allow the destination: both previously unsafe publishes are
    // now permitted attempts.
    let mut repaired = context.clone();
    repaired.staged = vec![fixtures::candidate_a(), fixtures::candidate_b()];
    repaired.policy.allowed_destinations = vec![fixtures::destination()];
    // The observed record must attest the artifact that node 3 publishes.
    repaired.observations[0].subject = fixtures::candidate_b();
    let analysis = run(&plan, &repaired);
    assert_eq!(analysis.outcome.tag(), "verified_within_model");
    assert_eq!(analysis.receipt.expect("receipt").bad_edges, 0);
}

#[test]
fn making_a_modeled_pass_invalid_turns_a_verified_plan_into_a_violation() {
    let (plan, context) = fixtures::safe_chain();
    assert_eq!(run(&plan, &context).outcome.tag(), "verified_within_model");

    // The pass still happens, but the record's tenant no longer matches.
    let mut foreign = context.clone();
    foreign.hypothetical_pass.tenant = cegar_domain::TenantId::new("tenant-b");
    assert_eq!(rules(&run(&plan, &foreign)), vec![Rule::TenantMismatch]);
}
