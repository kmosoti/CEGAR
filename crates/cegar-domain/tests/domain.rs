//! Behavioral tests for the experimental domain compiler.
//!
//! These are synthetic fixtures only. A passing test says the compiler and the
//! core agree about a hand-written model; it says nothing about a real system,
//! a real test attestation or execution authority.

use cegar_domain::compile::Analysis;
use cegar_domain::fixtures;
use cegar_domain::{AnalysisOutcome, CompileLimits, Plan, Rule, analyze};

fn run(plan: &Plan, context: &cegar_domain::Context) -> Analysis {
    analyze(plan, context, CompileLimits::default())
}

fn counterexample_rules(outcome: &AnalysisOutcome) -> Vec<Rule> {
    match outcome {
        AnalysisOutcome::ConcreteCounterexample { rules, .. } => rules.clone(),
        other => panic!("expected a modeled counterexample, got {}", other.tag()),
    }
}

#[test]
fn safe_chain_verifies_only_within_the_specified_model() {
    let (plan, context) = fixtures::safe_chain();
    let analysis = run(&plan, &context);
    assert_eq!(analysis.outcome.tag(), "verified_within_model");
    let receipt = analysis.receipt.expect("receipt");
    assert!(!receipt.execution_authorized);
    assert_eq!(receipt.bad_edges, 0);
    assert_eq!(receipt.scope, "supplied_synthetic_domain_model_only");
    assert_eq!(receipt.provenance, "synthetic_fixture");
    assert!(receipt.state_count > 0 && receipt.state_count <= 128);
    assert!(analysis.report.is_some());
}

#[test]
fn subject_mismatch_reaches_an_explicit_violation() {
    let (plan, context) = fixtures::subject_mismatch();
    let analysis = run(&plan, &context);
    assert_eq!(
        counterexample_rules(&analysis.outcome),
        vec![Rule::SubjectMismatch]
    );
    match &analysis.outcome {
        AnalysisOutcome::ConcreteCounterexample { trace, .. } => {
            let attempt = trace.last().expect("terminal attempt");
            assert_eq!(attempt.outcome, "unsafe_attempt");
            assert_eq!(attempt.node.0, 2);
            assert_eq!(attempt.rules, vec![Rule::SubjectMismatch]);
        }
        _ => unreachable!(),
    }
    let receipt = analysis.receipt.expect("receipt");
    assert!(receipt.bad_edges >= 1);
}

#[test]
fn ambiguous_stage_output_enumerates_every_binding() {
    let (plan, context) = fixtures::ambiguous_stage_output();
    let analysis = run(&plan, &context);
    // The A branch can satisfy publication; the B branch must not be hidden.
    assert_eq!(
        counterexample_rules(&analysis.outcome),
        vec![Rule::SubjectMismatch]
    );
    let receipt = analysis.receipt.expect("receipt");
    let stage_labels: Vec<&String> = receipt
        .action_table
        .iter()
        .filter(|label| label.contains("stage_patch:success"))
        .collect();
    assert_eq!(
        stage_labels.len(),
        2,
        "both permitted stage outputs must be enumerated: {:?}",
        receipt.action_table
    );
    assert!(
        receipt
            .assumptions
            .iter()
            .any(|item| item.contains("permitted outputs")),
        "the receipt must record the stage-output assumption"
    );
}

#[test]
fn suite_required_for_publication_is_checked_against_the_attempt() {
    let (plan, context) = fixtures::wrong_required_suite();
    let analysis = run(&plan, &context);
    assert_eq!(
        counterexample_rules(&analysis.outcome),
        vec![Rule::SuiteNotRequired]
    );
}

#[test]
fn modeled_pass_from_an_untrusted_issuer_is_not_permission() {
    let (plan, context) = fixtures::untrusted_issuer();
    let analysis = run(&plan, &context);
    assert_eq!(
        counterexample_rules(&analysis.outcome),
        vec![Rule::IssuerUntrusted]
    );
}

#[test]
fn cross_principal_evidence_use_needs_an_explicit_rule() {
    let (plan, context) = fixtures::cross_principal_without_rule();
    let analysis = run(&plan, &context);
    assert_eq!(
        counterexample_rules(&analysis.outcome),
        vec![Rule::PrincipalNotShared]
    );

    let mut permitting = context;
    permitting.policy.evidence_sharing =
        cegar_domain::context::EvidenceSharing::PolicyPermitted(vec![
            cegar_domain::PrincipalId::new("p2"),
        ]);
    let analysis = run(&plan, &permitting);
    assert_eq!(analysis.outcome.tag(), "verified_within_model");
}

#[test]
fn declared_suite_for_the_test_is_not_the_required_publication_suite() {
    let (plan, context) = fixtures::declared_output_not_minimal();
    let analysis = run(&plan, &context);
    assert_eq!(
        counterexample_rules(&analysis.outcome),
        vec![Rule::SuiteNotRequired]
    );
}

#[test]
fn stale_observation_is_classified_before_the_graph() {
    let (plan, context) = fixtures::stale_observation();
    let analysis = run(&plan, &context);
    assert_eq!(analysis.outcome.tag(), "stale_snapshot");
    assert!(analysis.receipt.is_none());
}

#[test]
fn absent_observation_is_needs_evidence_not_a_guess() {
    let (plan, context) = fixtures::missing_observation();
    let analysis = run(&plan, &context);
    assert_eq!(analysis.outcome.tag(), "needs_evidence");
    match &analysis.outcome {
        AnalysisOutcome::NeedsEvidence(diagnostics) => {
            assert_eq!(diagnostics[0].rule, Rule::EvidenceMissing);
        }
        _ => unreachable!(),
    }
}

#[test]
fn contradictory_authoritative_records_are_invalid_input() {
    let (plan, context) = fixtures::contradictory_records();
    let analysis = run(&plan, &context);
    assert_eq!(analysis.outcome.tag(), "invalid_input");
    match &analysis.outcome {
        AnalysisOutcome::InvalidInput(diagnostics) => {
            assert!(
                diagnostics
                    .iter()
                    .any(|item| item.rule == Rule::ContradictoryPremises)
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn cyclic_plan_has_no_graph_at_all() {
    let (plan, context) = fixtures::cyclic_plan();
    let analysis = run(&plan, &context);
    assert_eq!(analysis.outcome.tag(), "invalid_input");
    match &analysis.outcome {
        AnalysisOutcome::InvalidInput(diagnostics) => {
            assert!(diagnostics.iter().any(|item| item.rule == Rule::CyclicPlan));
        }
        _ => unreachable!(),
    }
}

#[test]
fn unknown_stage_outputs_make_the_scope_inconclusive() {
    let (plan, context) = fixtures::unknown_stage_outputs();
    let analysis = run(&plan, &context);
    assert_eq!(analysis.outcome.tag(), "inconclusive");
    assert!(analysis.receipt.is_none());
}

#[test]
fn oversized_graph_is_refused_rather_than_truncated() {
    let (plan, context) = fixtures::oversized_graph();
    let analysis = run(&plan, &context);
    assert_eq!(analysis.outcome.tag(), "inconclusive");
    match &analysis.outcome {
        AnalysisOutcome::Inconclusive(diagnostics) => {
            assert!(
                diagnostics.iter().any(|item| item.rule == Rule::GraphLimit),
                "expected a graph-limit diagnostic, got {diagnostics:?}"
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn two_unsafe_ready_attempts_are_both_represented() {
    let (plan, context) = fixtures::two_unsafe_ready_attempts();
    let analysis = run(&plan, &context);
    let rules = counterexample_rules(&analysis.outcome);
    let expected: [Vec<Rule>; 2] = [
        vec![Rule::DestinationUnauthorized],
        vec![Rule::SubjectNotStaged],
    ];
    assert!(
        expected.contains(&rules),
        "unexpected violation set {rules:?}"
    );
    let receipt = analysis.receipt.expect("receipt");
    // Both ready publishes must contribute an edge to the monitor, even though
    // the core returns only one witness.
    assert!(
        receipt.bad_edges >= 2,
        "expected at least two modeled unsafe attempts, got {}",
        receipt.bad_edges
    );
    let publish_violations: Vec<&String> = receipt
        .action_table
        .iter()
        .filter(|label| label.contains("publish:unsafe_attempt"))
        .collect();
    assert_eq!(publish_violations.len(), 2, "{:?}", receipt.action_table);
    assert!(
        publish_violations
            .iter()
            .any(|label| label.contains("policy.destination_unauthorized")),
        "{:?}",
        receipt.action_table
    );
    assert!(
        publish_violations
            .iter()
            .any(|label| label.contains("attempt.subject_not_staged")),
        "{:?}",
        receipt.action_table
    );
}

#[test]
fn reordering_the_plan_does_not_change_the_compiled_receipt() {
    let (plan, context) = fixtures::ambiguous_stage_output();
    let analysis = run(&plan, &context);

    let mut reordered = plan.clone();
    reordered.nodes.reverse();
    let shuffled = run(&reordered, &context);

    assert_eq!(analysis.outcome.tag(), shuffled.outcome.tag());
    let left = analysis.receipt.expect("receipt");
    let right = shuffled.receipt.expect("receipt");
    assert_eq!(left.state_count, right.state_count);
    assert_eq!(left.transition_count, right.transition_count);
    assert_eq!(left.action_table, right.action_table);
    assert_eq!(left.bad_edges, right.bad_edges);
    assert_eq!(left.assumptions, right.assumptions);
}

#[test]
fn identical_inputs_repeat_identically() {
    let (plan, context) = fixtures::safe_chain();
    let first = run(&plan, &context);
    let second = run(&plan, &context);
    assert_eq!(first.outcome, second.outcome);
    assert_eq!(first.receipt, second.receipt);
}

#[test]
fn every_analysis_reports_no_execution_authority() {
    for (plan, context) in [
        fixtures::safe_chain(),
        fixtures::subject_mismatch(),
        fixtures::missing_observation(),
        fixtures::stale_observation(),
        fixtures::cyclic_plan(),
        fixtures::oversized_graph(),
    ] {
        let analysis = run(&plan, &context);
        assert!(!analysis.outcome.execution_authorized());
    }
}
