//! One scoped validity predicate shared by observed records and hypothetical
//! modeled passes.
//!
//! A matching subject digest is never sufficient: status, suite, runner,
//! issuer trust, principal relationship, tenant, audience, destination,
//! freshness and pinned context must all hold. The same predicate is evaluated
//! for a real (fixture) record and for a modeled future pass, so a hypothetical
//! pass alone never becomes permission.

use crate::context::{Policy, TestStatus};
use crate::rules::Rule;
use crate::types::{
    ArtifactId, AudienceId, DestinationId, PrincipalId, SnapshotRef, SuiteRef, TenantId,
};

/// The scoped facts one record or hypothetical pass carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceView<'a> {
    pub status: TestStatus,
    pub subject: ArtifactId,
    pub suite: &'a SuiteRef,
    pub runner: &'a PrincipalId,
    pub issuer: &'a PrincipalId,
    pub evidence_principal: &'a PrincipalId,
    pub tenant: &'a TenantId,
    pub audience: &'a AudienceId,
    pub issued_at: u64,
    pub validity: u64,
    pub snapshot: &'a SnapshotRef,
}

/// Every violated scoped obligation, sorted by rule code. Empty means the
/// predicate holds for the modeled attempt.
pub fn violations(
    view: &EvidenceView<'_>,
    candidate: &ArtifactId,
    destination: &DestinationId,
    context_snapshot: &SnapshotRef,
    policy: &Policy,
    now: u64,
) -> Vec<Rule> {
    let mut violated = Vec::new();

    if view.status != TestStatus::Pass {
        violated.push(Rule::StatusNotPass);
    }
    if view.subject != *candidate {
        violated.push(Rule::SubjectMismatch);
    }
    if view.suite != &policy.required_suite {
        violated.push(Rule::SuiteNotRequired);
    }
    if !policy.runner_authorized(view.runner) {
        violated.push(Rule::RunnerUnauthorized);
    }
    if !policy.issuer_trusted(view.issuer) {
        violated.push(Rule::IssuerUntrusted);
    }
    let same_principal = view.evidence_principal == &policy.verification_principal;
    if !same_principal && !policy.sharing_permitted(view.evidence_principal) {
        violated.push(Rule::PrincipalNotShared);
    }
    if view.tenant != &policy.tenant {
        violated.push(Rule::TenantMismatch);
    }
    if view.audience != &policy.audience {
        violated.push(Rule::AudienceMismatch);
    }
    if !policy.destination_authorized(destination) {
        violated.push(Rule::DestinationUnauthorized);
    }
    if view.issued_at > now {
        // A record issued after the modeled evaluation time cannot be modeled
        // as a known-valid fact; treating it as valid would smuggle in an
        // unverified temporal claim.
        violated.push(Rule::EvidenceExpired);
    } else if now > view.issued_at.saturating_add(view.validity) {
        violated.push(Rule::EvidenceExpired);
    }
    if view.snapshot != context_snapshot {
        violated.push(Rule::SnapshotMismatch);
    }

    Rule::sort_codes(&mut violated);
    violated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FormatId, synthetic_digest};

    struct Facts {
        suite: SuiteRef,
        snapshot: SnapshotRef,
        runner: PrincipalId,
        issuer: PrincipalId,
        tenant: TenantId,
        audience: AudienceId,
    }

    impl Facts {
        fn new() -> Self {
            Self {
                suite: SuiteRef::new(crate::types::SuiteName::new("s1"), 1, synthetic_digest(9)),
                snapshot: SnapshotRef::new(3, synthetic_digest(2)),
                runner: PrincipalId::new("runner"),
                issuer: PrincipalId::new("issuer"),
                tenant: TenantId::new("t"),
                audience: AudienceId::new("a"),
            }
        }

        fn view<'a>(
            &'a self,
            status: TestStatus,
            subject: ArtifactId,
            principal: &'a PrincipalId,
            issued_at: u64,
        ) -> EvidenceView<'a> {
            EvidenceView {
                status,
                subject,
                suite: &self.suite,
                runner: &self.runner,
                issuer: &self.issuer,
                evidence_principal: principal,
                tenant: &self.tenant,
                audience: &self.audience,
                issued_at,
                validity: 600,
                snapshot: &self.snapshot,
            }
        }
    }

    fn candidate() -> ArtifactId {
        ArtifactId::new(synthetic_digest(1), FormatId::Candidate)
    }

    fn policy() -> Policy {
        Policy {
            reference: crate::types::PolicyRef::new(1, synthetic_digest(5)),
            verification_principal: PrincipalId::new("p1"),
            authorized_operations: Vec::new(),
            authorized_actors: vec![PrincipalId::new("p1")],
            required_suite: SuiteRef::new(
                crate::types::SuiteName::new("s1"),
                1,
                synthetic_digest(9),
            ),
            permitted_test_suites: Vec::new(),
            trusted_issuers: vec![PrincipalId::new("issuer")],
            authorized_runners: vec![PrincipalId::new("runner")],
            allowed_destinations: vec![DestinationId::new("d")],
            tenant: TenantId::new("t"),
            audience: AudienceId::new("a"),
            evidence_sharing: crate::context::EvidenceSharing::SamePrincipalOnly,
            max_evidence_age: 3_600,
        }
    }

    fn check(view: &EvidenceView<'_>, now: u64) -> Vec<Rule> {
        violations(
            view,
            &candidate(),
            &DestinationId::new("d"),
            view.snapshot,
            &policy(),
            now,
        )
    }

    #[test]
    fn matching_context_has_no_violations() {
        let facts = Facts::new();
        let principal = PrincipalId::new("p1");
        let view = facts.view(TestStatus::Pass, candidate(), &principal, 1_000);
        assert!(check(&view, 1_100).is_empty());
    }

    #[test]
    fn wrong_subject_is_reported() {
        let facts = Facts::new();
        let principal = PrincipalId::new("p1");
        let other = ArtifactId::new(synthetic_digest(4), FormatId::Candidate);
        let view = facts.view(TestStatus::Pass, other, &principal, 1_000);
        assert_eq!(check(&view, 1_100), vec![Rule::SubjectMismatch]);
    }

    #[test]
    fn failing_status_is_not_evidence_of_pass() {
        let facts = Facts::new();
        let principal = PrincipalId::new("p1");
        let view = facts.view(TestStatus::Fail, candidate(), &principal, 1_000);
        assert_eq!(check(&view, 1_100), vec![Rule::StatusNotPass]);
    }

    #[test]
    fn stale_and_future_records_are_rejected() {
        let facts = Facts::new();
        let principal = PrincipalId::new("p1");
        {
            let view = facts.view(TestStatus::Pass, candidate(), &principal, 1_000);
            assert_eq!(check(&view, 9_999), vec![Rule::EvidenceExpired]);
        }
        {
            let view = facts.view(TestStatus::Pass, candidate(), &principal, 5_000);
            assert_eq!(check(&view, 1_000), vec![Rule::EvidenceExpired]);
        }
    }

    #[test]
    fn cross_principal_use_needs_an_explicit_rule() {
        let facts = Facts::new();
        let other = PrincipalId::new("p2");
        let view = facts.view(TestStatus::Pass, candidate(), &other, 1_000);
        assert_eq!(check(&view, 1_100), vec![Rule::PrincipalNotShared]);

        let mut permitting = policy();
        permitting.evidence_sharing =
            crate::context::EvidenceSharing::PolicyPermitted(vec![PrincipalId::new("p2")]);
        assert!(
            violations(
                &view,
                &candidate(),
                &DestinationId::new("d"),
                view.snapshot,
                &permitting,
                1_100,
            )
            .is_empty()
        );
    }

    #[test]
    fn tenant_and_audience_are_separate_obligations() {
        let facts = Facts::new();
        let principal = PrincipalId::new("p1");
        let mut view = facts.view(TestStatus::Pass, candidate(), &principal, 1_000);
        let foreign_tenant = TenantId::new("t2");
        view.tenant = &foreign_tenant;
        assert_eq!(check(&view, 1_100), vec![Rule::TenantMismatch]);
        view.tenant = &facts.tenant;
        let foreign_audience = AudienceId::new("a2");
        view.audience = &foreign_audience;
        assert_eq!(check(&view, 1_100), vec![Rule::AudienceMismatch]);
    }

    #[test]
    fn unauthorized_destination_is_a_violation() {
        let facts = Facts::new();
        let principal = PrincipalId::new("p1");
        let view = facts.view(TestStatus::Pass, candidate(), &principal, 1_000);
        let policy = policy();
        assert_eq!(
            violations(
                &view,
                &candidate(),
                &DestinationId::new("d-other"),
                view.snapshot,
                &policy,
                1_100,
            ),
            vec![Rule::DestinationUnauthorized]
        );
    }
}
