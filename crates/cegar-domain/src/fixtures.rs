//! Synthetic fixture corpus. Every identifier is invented, every fact is
//! hand-written, and no fixture is a real observation or an executed test.

use crate::context::{
    Context, EvidenceSharing, HypotheticalPass, Observation, Policy, Provenance, StageOutputs,
    TestStatus,
};
use crate::plan::{
    Disposition, EvidenceRef, InputRef, Node, NodeId, NodeKind, Plan, PublishSpec, StageSpec,
    TestSpec,
};
use crate::types::{
    ArtifactId, AudienceId, DestinationId, FormatId, OperationName, OperationRef, PolicyRef,
    PrincipalId, RunId, SnapshotRef, SuiteName, SuiteRef, TenantId, synthetic_digest,
};

pub const REVISION: u32 = 1;

pub fn stage_operation() -> OperationRef {
    OperationRef::new(OperationName::new("stage_patch"), 1, synthetic_digest(31))
}

pub fn test_operation() -> OperationRef {
    OperationRef::new(
        OperationName::new("run_registered_tests"),
        1,
        synthetic_digest(32),
    )
}

pub fn publish_operation() -> OperationRef {
    OperationRef::new(OperationName::new("publish"), 1, synthetic_digest(33))
}

pub fn suite_s1() -> SuiteRef {
    SuiteRef::new(SuiteName::new("suite-s1"), 1, synthetic_digest(41))
}

pub fn suite_s2() -> SuiteRef {
    SuiteRef::new(SuiteName::new("suite-s2"), 1, synthetic_digest(42))
}

pub fn base_artifact() -> ArtifactId {
    ArtifactId::new(synthetic_digest(11), FormatId::Base)
}

pub fn patch_artifact() -> ArtifactId {
    ArtifactId::new(synthetic_digest(12), FormatId::Patch)
}

pub fn candidate_a() -> ArtifactId {
    ArtifactId::new(synthetic_digest(21), FormatId::Candidate)
}

pub fn candidate_b() -> ArtifactId {
    ArtifactId::new(synthetic_digest(22), FormatId::Candidate)
}

pub fn snapshot() -> SnapshotRef {
    SnapshotRef::new(3, synthetic_digest(51))
}

pub fn principal() -> PrincipalId {
    PrincipalId::new("p1")
}

pub fn destination() -> DestinationId {
    DestinationId::new("registry-d")
}

pub fn tenant() -> TenantId {
    TenantId::new("tenant-a")
}

pub fn audience() -> AudienceId {
    AudienceId::new("audience-a")
}

pub fn runner() -> PrincipalId {
    PrincipalId::new("runner")
}

pub fn issuer() -> PrincipalId {
    PrincipalId::new("issuer")
}

pub const NOW: u64 = 1_000_000;

/// The single explicit synthetic policy profile used by the experiment.
pub fn policy() -> Policy {
    Policy {
        reference: PolicyRef::new(1, synthetic_digest(101)),
        verification_principal: principal(),
        authorized_operations: vec![stage_operation(), test_operation(), publish_operation()],
        authorized_actors: vec![principal()],
        required_suite: suite_s1(),
        permitted_test_suites: vec![suite_s1(), suite_s2()],
        trusted_issuers: vec![issuer()],
        authorized_runners: vec![runner()],
        allowed_destinations: vec![destination()],
        tenant: tenant(),
        audience: audience(),
        evidence_sharing: EvidenceSharing::SamePrincipalOnly,
        max_evidence_age: 3_600,
    }
}

pub fn context() -> Context {
    Context {
        provenance: Provenance::SyntheticFixture,
        policy: policy(),
        snapshot: snapshot(),
        tool_registry: vec![stage_operation(), test_operation(), publish_operation()],
        suite_registry: vec![suite_s1(), suite_s2()],
        artifacts: vec![
            base_artifact(),
            patch_artifact(),
            candidate_a(),
            candidate_b(),
        ],
        staged: Vec::new(),
        observations: Vec::new(),
        stage_outputs: vec![StageOutputs {
            node: NodeId(1),
            candidates: vec![candidate_a()],
        }],
        hypothetical_pass: HypotheticalPass {
            issuer: issuer(),
            evidence_principal: principal(),
            tenant: tenant(),
            audience: audience(),
            validity: 600,
        },
        now: NOW,
    }
}

pub fn stage_node(id: u32, deps: &[u32]) -> Node {
    Node {
        id: NodeId(id),
        deps: deps.iter().copied().map(NodeId).collect(),
        kind: NodeKind::Stage(StageSpec {
            operation: stage_operation(),
            actor: principal(),
            base: base_artifact(),
            patch: patch_artifact(),
        }),
    }
}

pub fn test_node(id: u32, deps: &[u32], subject: InputRef, suite: SuiteRef) -> Node {
    Node {
        id: NodeId(id),
        deps: deps.iter().copied().map(NodeId).collect(),
        kind: NodeKind::Test(TestSpec {
            operation: test_operation(),
            actor: principal(),
            subject,
            suite,
            runner: runner(),
        }),
    }
}

pub fn publish_node(
    id: u32,
    deps: &[u32],
    subject: InputRef,
    evidence: EvidenceRef,
    destination_id: DestinationId,
) -> Node {
    Node {
        id: NodeId(id),
        deps: deps.iter().copied().map(NodeId).collect(),
        kind: NodeKind::Publish(PublishSpec {
            operation: publish_operation(),
            actor: principal(),
            subject,
            evidence,
            destination: destination_id,
        }),
    }
}

pub fn plan(nodes: Vec<Node>) -> Plan {
    Plan {
        revision: REVISION,
        disposition: Disposition::ContinueIndependent,
        nodes,
    }
}

/// stage → test(S1) → publish with the modeled pass.
pub fn safe_chain() -> (Plan, Context) {
    let nodes = vec![
        stage_node(1, &[]),
        test_node(2, &[1], InputRef::Produced(NodeId(1)), suite_s1()),
        publish_node(
            3,
            &[2],
            InputRef::Produced(NodeId(1)),
            EvidenceRef::Produced(NodeId(2)),
            destination(),
        ),
    ];
    (plan(nodes), context())
}

/// Test A, then publish B while B is also staged: the exact subject mismatch.
pub fn subject_mismatch() -> (Plan, Context) {
    let mut context = context();
    context.staged = vec![candidate_a(), candidate_b()];
    let nodes = vec![
        test_node(1, &[], InputRef::Initial(candidate_a()), suite_s1()),
        publish_node(
            2,
            &[1],
            InputRef::Initial(candidate_b()),
            EvidenceRef::Produced(NodeId(1)),
            destination(),
        ),
    ];
    (plan(nodes), context)
}

/// The stage operation may bind A or B; both branches must be enumerated.
pub fn ambiguous_stage_output() -> (Plan, Context) {
    let mut context = context();
    context.stage_outputs = vec![StageOutputs {
        node: NodeId(1),
        candidates: vec![candidate_a(), candidate_b()],
    }];
    // A is already staged, so the test in this plan certifies A while the
    // publish uses whatever identity the stage branch actually bound.
    context.staged = vec![candidate_a()];
    let nodes = vec![
        stage_node(1, &[]),
        test_node(2, &[], InputRef::Initial(candidate_a()), suite_s1()),
        publish_node(
            3,
            &[1, 2],
            InputRef::Produced(NodeId(1)),
            EvidenceRef::Produced(NodeId(2)),
            destination(),
        ),
    ];
    (plan(nodes), context)
}

/// The plan declares a candidate identity, but the pinned assumption lists a
/// different possible output. The declaration is not evidence of a singleton.
pub fn declared_output_not_minimal() -> (Plan, Context) {
    let mut context = context();
    context.stage_outputs = vec![StageOutputs {
        node: NodeId(1),
        candidates: vec![candidate_a(), candidate_b()],
    }];
    let nodes = vec![
        stage_node(1, &[]),
        test_node(2, &[1], InputRef::Produced(NodeId(1)), suite_s2()),
        publish_node(
            3,
            &[2],
            InputRef::Produced(NodeId(1)),
            EvidenceRef::Produced(NodeId(2)),
            destination(),
        ),
    ];
    (plan(nodes), context)
}

/// Rule out the S1-only case: a required-suite mismatch is a bad attempt.
pub fn wrong_required_suite() -> (Plan, Context) {
    let mut context = context();
    context.policy.required_suite = suite_s2();
    let nodes = vec![
        stage_node(1, &[]),
        test_node(2, &[1], InputRef::Produced(NodeId(1)), suite_s1()),
        publish_node(
            3,
            &[2],
            InputRef::Produced(NodeId(1)),
            EvidenceRef::Produced(NodeId(2)),
            destination(),
        ),
    ];
    (plan(nodes), context)
}

/// A modeled pass whose issuer is not trusted for the scope.
pub fn untrusted_issuer() -> (Plan, Context) {
    let mut context = context();
    context.hypothetical_pass.issuer = PrincipalId::new("issuer-rogue");
    safe_chain_with(context)
}

fn safe_chain_with(context: Context) -> (Plan, Context) {
    let nodes = vec![
        stage_node(1, &[]),
        test_node(2, &[1], InputRef::Produced(NodeId(1)), suite_s1()),
        publish_node(
            3,
            &[2],
            InputRef::Produced(NodeId(1)),
            EvidenceRef::Produced(NodeId(2)),
            destination(),
        ),
    ];
    (plan(nodes), context)
}

/// A modeled pass owned by another principal without a sharing rule.
pub fn cross_principal_without_rule() -> (Plan, Context) {
    let mut context = context();
    context.hypothetical_pass.evidence_principal = PrincipalId::new("p2");
    safe_chain_with(context)
}

/// An initial observation that is expired relative to the policy bound.
pub fn stale_observation() -> (Plan, Context) {
    let mut context = context();
    context.observations = vec![observation(TestStatus::Pass, NOW - 100_000)];
    context.staged = vec![candidate_a()];
    let nodes = vec![publish_node(
        1,
        &[],
        InputRef::Initial(candidate_a()),
        EvidenceRef::Observed(RunId::new("run-1")),
        destination(),
    )];
    (plan(nodes), context)
}

/// A publish that designates an observation the context never provides.
pub fn missing_observation() -> (Plan, Context) {
    let mut context = context();
    context.staged = vec![candidate_a()];
    let nodes = vec![publish_node(
        1,
        &[],
        InputRef::Initial(candidate_a()),
        EvidenceRef::Observed(RunId::new("run-absent")),
        destination(),
    )];
    (plan(nodes), context)
}

/// Two authoritative records for one run disagree.
pub fn contradictory_records() -> (Plan, Context) {
    let mut context = context();
    context.observations = vec![
        observation(TestStatus::Pass, NOW - 30),
        observation(TestStatus::Fail, NOW - 20),
    ];
    context.staged = vec![candidate_a()];
    let nodes = vec![publish_node(
        1,
        &[],
        InputRef::Initial(candidate_a()),
        EvidenceRef::Observed(RunId::new("run-1")),
        destination(),
    )];
    (plan(nodes), context)
}

pub fn observation(status: TestStatus, issued_at: u64) -> Observation {
    Observation {
        run: RunId::new("run-1"),
        subject: candidate_a(),
        suite: suite_s1(),
        status,
        runner: runner(),
        issuer: issuer(),
        evidence_principal: principal(),
        tenant: tenant(),
        audience: audience(),
        issued_at,
        validity: 600,
    }
}

/// A cyclic plan: no valid graph or proof exists.
pub fn cyclic_plan() -> (Plan, Context) {
    let nodes = vec![
        stage_node(1, &[2]),
        test_node(2, &[1], InputRef::Produced(NodeId(1)), suite_s1()),
    ];
    (plan(nodes), context())
}

/// Two unsafe ready attempts must both reach a first-violation monitor.
pub fn two_unsafe_ready_attempts() -> (Plan, Context) {
    let mut context = context();
    context.staged = vec![candidate_a()];
    context.observations = vec![observation(TestStatus::Pass, NOW - 30)];
    // Every destination in this plan is missing from the policy's allow list, so
    // both ready publishes are unsafe attempts.
    context.policy.allowed_destinations = vec![DestinationId::new("registry-other")];
    let nodes = vec![
        test_node(1, &[], InputRef::Initial(candidate_a()), suite_s1()),
        publish_node(
            2,
            &[1],
            InputRef::Initial(candidate_a()),
            EvidenceRef::Produced(NodeId(1)),
            destination(),
        ),
        publish_node(
            3,
            &[],
            InputRef::Initial(candidate_b()),
            EvidenceRef::Observed(RunId::new("run-1")),
            destination(),
        ),
    ];
    (plan(nodes), context)
}

/// A stage output assumption that is absent: the broader scope is inconclusive.
pub fn unknown_stage_outputs() -> (Plan, Context) {
    let mut context = context();
    context.stage_outputs = Vec::new();
    let nodes = vec![
        stage_node(1, &[]),
        test_node(2, &[1], InputRef::Produced(NodeId(1)), suite_s1()),
    ];
    (plan(nodes), context)
}

/// Independently ready stages with two outputs each drive the graph over its
/// bound; the compiler must refuse rather than truncate.
pub fn oversized_graph() -> (Plan, Context) {
    let mut context = context();
    context.stage_outputs = (1..=4)
        .map(|id| StageOutputs {
            node: NodeId(id),
            candidates: vec![candidate_a(), candidate_b()],
        })
        .collect();
    let nodes = (1..=4).map(|id| stage_node(id, &[])).collect();
    (plan(nodes), context)
}
