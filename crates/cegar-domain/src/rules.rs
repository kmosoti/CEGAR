//! Stable rule identifiers used by diagnostics, first-violation monitor labels
//! and receipts. Ordering is by rule code so that canonical encodings do not
//! depend on Rust declaration order.

use core::fmt;

macro_rules! rules {
    ($($(#[$doc:meta])* $variant:ident => $code:literal),* $(,)?) => {
        /// One modeled obligation.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Rule {
            $($(#[$doc])* $variant),*
        }

        impl Rule {
            pub const ALL: &'static [Rule] = &[$(Rule::$variant),*];

            pub const fn code(self) -> &'static str {
                match self {
                    $(Rule::$variant => $code),*
                }
            }
        }
    };
}

rules![
    /// Structural: a node identifier appears more than once.
    DuplicateNode => "structure.duplicate_node",
    /// Structural: a dependency identifier does not resolve.
    UnknownDependency => "structure.unknown_dependency",
    /// Structural: a node depends on itself.
    SelfDependency => "structure.self_dependency",
    /// Structural: the dependency graph is cyclic.
    CyclicPlan => "structure.cyclic_plan",
    /// Structural: the plan contains no nodes.
    EmptyPlan => "structure.empty_plan",
    /// Structural: a node count exceeds the experimental bound
    PlanTooLarge => "structure.plan_too_large",
    /// Structural: a subject reference has the wrong typed role.
    SubjectKind => "structure.subject_kind",
    /// Structural: a producer output does not exist or has the wrong role.
    UnknownProducerOutput => "structure.unknown_producer_output",
    /// Structural: an operation is not registered for this context.
    UnknownOperation => "structure.unknown_operation",
    /// Structural: a test suite is not registered for this context.
    SuiteUnregistered => "structure.suite_unregistered",
    /// Structural: an initial artifact is absent from the snapshot.
    InputMissing => "structure.input_missing",
    /// Structural: two authoritative records disagree on one scoped fact.
    ContradictoryPremises => "premise.contradictory",
    /// Structural: a required initial observation is absent.
    EvidenceMissing => "premise.evidence_missing",
    /// Structural: an initial observation or snapshot is known stale.
    StaleSnapshot => "premise.stale",
    /// Compilation: requested behavior cannot be represented (unknown outputs,
    /// unsupported scheduler semantics, unimplemented failure mode).
    UnsupportedSemantics => "compilation.unsupported_semantics",
    /// Compilation: the graph or exploration exceeded a declared bound.
    GraphLimit => "compilation.graph_limit",
    /// Compilation: a live state had pending work but no permitted transition.
    UnexplainedStall => "compilation.unexplained_stall",
    /// Precondition: the parked test suite is not permitted for test execution.
    TestSuiteNotPermitted => "policy.test_suite_not_permitted",
    /// Precondition: the acting principal may not invoke this operation.
    OperationUnauthorized => "policy.operation_unauthorized",
    /// Precondition: the principal is not authorized to act.
    ActorUnauthorized => "policy.actor_unauthorized",
    /// Precondition: the principal may not publish to this destination.
    DestinationUnauthorized => "policy.destination_unauthorized",
    /// Precondition: no staged candidate is bound at this attempt.
    SubjectNotStaged => "attempt.subject_not_staged",
    /// Publish subject is not the exact attested subject.
    SubjectMismatch => "evidence.subject_mismatch",
    /// Designated attestation status is not `pass`.
    StatusNotPass => "evidence.status_not_pass",
    /// The attestation's suite is not the policy-required suite.
    SuiteNotRequired => "evidence.suite_not_required",
    /// The test runner is not authorized for the pinned run/scope.
    RunnerUnauthorized => "evidence.runner_unauthorized",
    /// The record issuer is not trusted for this scope.
    IssuerUntrusted => "evidence.issuer_untrusted",
    /// Cross-principal evidence use lacks an explicit policy sharing rule.
    PrincipalNotShared => "evidence.principal_not_shared",
    /// Evidence tenant is incompatible with the requesting principal.
    TenantMismatch => "evidence.tenant_mismatch",
    /// Evidence audience is incompatible with the intended use.
    AudienceMismatch => "evidence.audience_mismatch",
    /// Evidence validity interval does not cover the modeled attempt.
    EvidenceExpired => "evidence.expired",
    /// Evidence was collected against a different snapshot/policy context.
    SnapshotMismatch => "evidence.snapshot_mismatch",
];

impl Rule {
    pub fn sort_codes(rules: &mut Vec<Rule>) {
        rules.sort_by_key(|rule| rule.code());
        rules.dedup();
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl Ord for Rule {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.code().cmp(other.code())
    }
}

impl PartialOrd for Rule {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Explain one modeled obligation without asserting real-world behavior.
pub const fn explain(rule: Rule) -> &'static str {
    match rule {
        Rule::DuplicateNode => "node identifiers must be unique",
        Rule::UnknownDependency => "every dependency must resolve to one node",
        Rule::SelfDependency => "a node may not depend on itself",
        Rule::CyclicPlan => "the v0.1 scheduler models a finite DAG only",
        Rule::EmptyPlan => "a plan must contain at least one node",
        Rule::PlanTooLarge => "the experimental node bound was exceeded",
        Rule::SubjectKind => "subjects must be candidate-role artifacts",
        Rule::UnknownProducerOutput => "producer output references must resolve",
        Rule::UnknownOperation => "the operation is not in the pinned tool registry",
        Rule::SuiteUnregistered => "the suite is not in the pinned suite registry",
        Rule::InputMissing => "the referenced immutable input is not in the snapshot",
        Rule::ContradictoryPremises => "authoritative records disagree on one scoped fact",
        Rule::EvidenceMissing => "a required initial observation is absent",
        Rule::StaleSnapshot => "a pinned snapshot or record is known stale",
        Rule::UnsupportedSemantics => "requested behavior is not representable in v0.1",
        Rule::GraphLimit => "the monitored graph exceeded the declared limit",
        Rule::UnexplainedStall => "a live state had work but no permitted transition",
        Rule::TestSuiteNotPermitted => "the suite is not permitted for test execution",
        Rule::OperationUnauthorized => "the operation is not authorized by policy",
        Rule::ActorUnauthorized => "the acting principal is not authorized by policy",
        Rule::DestinationUnauthorized => "the destination is not authorized by policy",
        Rule::SubjectNotStaged => "the publish subject must be staged",
        Rule::SubjectMismatch => "the attestation subject is not the publish subject",
        Rule::StatusNotPass => "the designated attestation did not pass",
        Rule::SuiteNotRequired => "the suite is not the policy-required suite",
        Rule::RunnerUnauthorized => "the runner is not authorized for this test",
        Rule::IssuerUntrusted => "the issuer is not trusted for this scope",
        Rule::PrincipalNotShared => "cross-principal evidence use is not permitted",
        Rule::TenantMismatch => "the evidence tenant is incompatible",
        Rule::AudienceMismatch => "the evidence audience is incompatible",
        Rule::EvidenceExpired => "the evidence validity interval does not cover the attempt",
        Rule::SnapshotMismatch => "the evidence was collected against another context",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_sorted_deterministically() {
        let mut codes: Vec<&str> = Rule::ALL.iter().map(|rule| rule.code()).collect();
        let count = codes.len();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(count, codes.len());
    }

    #[test]
    fn ordering_follows_codes_not_declaration() {
        assert!(Rule::EvidenceExpired < Rule::StatusNotPass);
        assert!(Rule::GraphLimit < Rule::InputMissing);
        assert!(Rule::OperationUnauthorized < Rule::StaleSnapshot);
    }

    #[test]
    fn no_rule_code_is_empty() {
        for rule in Rule::ALL {
            assert!(!rule.code().is_empty());
            assert!(!explain(*rule).is_empty());
        }
    }
}
