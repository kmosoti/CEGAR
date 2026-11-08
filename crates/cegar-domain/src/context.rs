//! Pinned experimental context: policy, registries, snapshot facts and the
//! evidence records a trusted collector would have produced.
//!
//! Every fact in this slice is synthetic. [`Provenance::SyntheticFixture`] is
//! carried into every receipt so no reader can mistake a fixture for a real
//! observation, and no deserialized field can create a trusted record here.

use crate::plan::Diagnostic;
use crate::rules::Rule;
use crate::types::{
    ArtifactId, AudienceId, DestinationId, OperationRef, PolicyRef, PrincipalId, RunId,
    SnapshotRef, SuiteRef, TenantId,
};

/// Where the initial facts came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provenance {
    /// Hand-written synthetic fixture. Not a collector, not authenticated.
    SyntheticFixture,
}

impl Provenance {
    pub const fn tag(self) -> &'static str {
        match self {
            Self::SyntheticFixture => "synthetic_fixture",
        }
    }
}

/// Modeled test outcome. `Error` is distinct from `Fail`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestStatus {
    Pass,
    Fail,
    Error,
}

impl TestStatus {
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Error => "error",
        }
    }
}

/// One authoritative test-run record. In this slice it is fixture data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub run: RunId,
    pub subject: ArtifactId,
    pub suite: SuiteRef,
    pub status: TestStatus,
    pub runner: PrincipalId,
    pub issuer: PrincipalId,
    /// Principal that owns the evidence record; may differ from the publisher.
    pub evidence_principal: PrincipalId,
    pub tenant: TenantId,
    pub audience: AudienceId,
    pub issued_at: u64,
    pub validity: u64,
}

/// Explicit policy for cross-principal evidence use. Absent a rule, evidence
/// never transfers between principals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceSharing {
    SamePrincipalOnly,
    /// The listed principals may use another principal's record when every
    /// other scope check also holds.
    PolicyPermitted(Vec<PrincipalId>),
}

/// The single explicit synthetic policy profile used by this experiment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    pub reference: PolicyRef,
    /// Principal that performs the modeled verification and publish attempts.
    pub verification_principal: PrincipalId,
    pub authorized_operations: Vec<OperationRef>,
    pub authorized_actors: Vec<PrincipalId>,
    /// Suite the publication rule requires. Distinct from suites a test node
    /// may be permitted to run.
    pub required_suite: SuiteRef,
    /// Suites a test node may be permitted to run.
    pub permitted_test_suites: Vec<SuiteRef>,
    pub trusted_issuers: Vec<PrincipalId>,
    /// Runners/collectors authorized to produce test outcomes.
    pub authorized_runners: Vec<PrincipalId>,
    pub allowed_destinations: Vec<DestinationId>,
    pub tenant: TenantId,
    pub audience: AudienceId,
    pub evidence_sharing: EvidenceSharing,
    /// Maximum modeled evidence age in seconds; zero disables the check.
    pub max_evidence_age: u64,
}

impl Policy {
    pub fn operation_authorized(&self, operation: &OperationRef) -> bool {
        self.authorized_operations.contains(operation)
    }

    pub fn actor_authorized(&self, actor: &PrincipalId) -> bool {
        self.authorized_actors.contains(actor)
    }

    pub fn destination_authorized(&self, destination: &DestinationId) -> bool {
        self.allowed_destinations.contains(destination)
    }

    pub fn issuer_trusted(&self, issuer: &PrincipalId) -> bool {
        self.trusted_issuers.contains(issuer)
    }

    pub fn runner_authorized(&self, runner: &PrincipalId) -> bool {
        self.authorized_runners.contains(runner)
    }

    pub fn test_suite_permitted(&self, suite: &SuiteRef) -> bool {
        self.permitted_test_suites.contains(suite)
    }

    /// A record owned by another principal may be used only under an explicit
    /// policy rule that names the requesting principal.
    pub fn sharing_permitted(&self, evidence_principal: &PrincipalId) -> bool {
        match &self.evidence_sharing {
            EvidenceSharing::SamePrincipalOnly => false,
            EvidenceSharing::PolicyPermitted(principals) => principals.contains(evidence_principal),
        }
    }
}

/// Explicit assumption: the exact candidate identities a stage node may bind
/// on a successful branch. A declaration in the plan is not sufficient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageOutputs {
    pub node: crate::plan::NodeId,
    pub candidates: Vec<ArtifactId>,
}

/// Explicit assumption: the pinned facts a hypothetical future pass would
/// carry. These are not observations and establish no real attestation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HypotheticalPass {
    pub issuer: PrincipalId,
    pub evidence_principal: PrincipalId,
    pub tenant: TenantId,
    pub audience: AudienceId,
    /// Modeled validity window in seconds from the modeled issue time.
    pub validity: u64,
}

/// Immutable pinned input to one compilation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub provenance: Provenance,
    pub policy: Policy,
    pub snapshot: SnapshotRef,
    /// Operations pinned in the experimental tool registry.
    pub tool_registry: Vec<OperationRef>,
    /// Suites pinned in the experimental suite registry.
    pub suite_registry: Vec<SuiteRef>,
    /// Immutable inputs present in the snapshot.
    pub artifacts: Vec<ArtifactId>,
    /// Candidates already staged before the modeled horizon.
    pub staged: Vec<ArtifactId>,
    /// Validated initial evidence records.
    pub observations: Vec<Observation>,
    /// Per-node stage output assumptions.
    pub stage_outputs: Vec<StageOutputs>,
    /// Pinned assumption for modeled future test passes.
    pub hypothetical_pass: HypotheticalPass,
    /// Modeled evaluation time in seconds since the Unix epoch.
    pub now: u64,
}

impl Context {
    pub fn stage_candidates(&self, node: crate::plan::NodeId) -> Option<&[ArtifactId]> {
        self.stage_outputs
            .iter()
            .find(|entry| entry.node == node)
            .map(|entry| entry.candidates.as_slice())
    }

    pub fn observation(&self, run: &RunId) -> Option<&Observation> {
        self.observations
            .iter()
            .find(|observation| &observation.run == run)
    }

    /// Premise defects that make a compilation unsound or unanswerable:
    /// contradictory authoritative records, undecidable registry membership and
    /// known-stale inputs. Absent evidence is *not* raised here; it is a
    /// `needs_evidence` classification.
    pub fn premise_defects(&self) -> Vec<Diagnostic> {
        let mut defects = Vec::new();

        let mut runs: Vec<&RunId> = self.observations.iter().map(|item| &item.run).collect();
        runs.sort();
        for pair in runs.windows(2) {
            if pair[0] == pair[1] {
                defects.push(Diagnostic::new(
                    Rule::ContradictoryPremises,
                    format!("two authoritative records claim test run {}", pair[0]),
                ));
            }
        }

        for observation in &self.observations {
            if !self.suite_registry.contains(&observation.suite) {
                defects.push(Diagnostic::new(
                    Rule::SuiteUnregistered,
                    format!(
                        "record {} references unregistered suite {}",
                        observation.run, observation.suite.name
                    ),
                ));
            }
            if self.policy.max_evidence_age > 0 {
                let age = self.now.saturating_sub(observation.issued_at);
                if age
                    > self
                        .policy
                        .max_evidence_age
                        .saturating_add(observation.validity)
                {
                    defects.push(Diagnostic::new(
                        Rule::StaleSnapshot,
                        format!(
                            "record {} is {age}s old; the policy bound is {}s",
                            observation.run, self.policy.max_evidence_age
                        ),
                    ));
                }
            }
        }

        for entry in &self.stage_outputs {
            for candidate in &entry.candidates {
                if candidate.format != crate::types::FormatId::Candidate {
                    defects.push(Diagnostic::new(
                        Rule::SubjectKind,
                        format!(
                            "stage node {} claims a {} output; only candidate-role outputs are modeled",
                            entry.node.0,
                            candidate.format.tag()
                        ),
                    ));
                }
            }
        }

        defects.sort_by(|left, right| {
            left.rule
                .code()
                .cmp(right.rule.code())
                .then_with(|| left.detail.cmp(&right.detail))
        });
        defects.dedup();
        defects
    }
}
