//! Seeded, reproducible case generator for the domain differential check.
//!
//! The corpus is synthetic and deliberately small: the point is to exercise
//! competing policy/evidence/schedule combinations against two independent
//! implementations, not to sample a realistic workload. Every case keeps the
//! raw inputs complete (no absent observation, no stale premise, no bound
//! exhaustion) so the compiler's verdict and the reference search are directly
//! comparable.

use cegar_domain::context::{Context, EvidenceSharing, StageOutputs, TestStatus};
use cegar_domain::fixtures;
use cegar_domain::plan::{Disposition, EvidenceRef, InputRef, NodeId, Plan};
use cegar_domain::types::{DestinationId, PrincipalId, RunId};

pub struct Case {
    pub name: String,
    pub plan: Plan,
    pub context: Context,
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        // xorshift64*
        let mut value = self.0;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.0 = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

/// Build `count` deterministic cases from one seed.
pub fn corpus(seed: u64, count: usize) -> Vec<Case> {
    let mut rng = Rng::new(seed);
    (0..count).map(|index| build(&mut rng, index)).collect()
}

fn build(rng: &mut Rng, index: usize) -> Case {
    let mut context = fixtures::context();
    // Roughly a third of the corpus is pinned so that the plan can actually be
    // satisfied; the rest is deliberately adversarial. Without both kinds the
    // differential check never exercises the verified branch or its state-count
    // comparison.
    let satisfiable = rng.chance(35);
    if satisfiable {
        context.policy.required_suite = fixtures::suite_s1();
        context.policy.permitted_test_suites = vec![fixtures::suite_s1(), fixtures::suite_s2()];
        context.policy.allowed_destinations = vec![fixtures::destination()];
        context.policy.evidence_sharing = EvidenceSharing::SamePrincipalOnly;
        context.staged = vec![fixtures::candidate_a(), fixtures::candidate_b()];
    }
    context.policy.required_suite = if satisfiable || rng.chance(50) {
        fixtures::suite_s1()
    } else {
        fixtures::suite_s2()
    };
    context.policy.permitted_test_suites = vec![fixtures::suite_s1(), fixtures::suite_s2()];
    if !satisfiable {
        context.policy.allowed_destinations = if rng.chance(60) {
            vec![fixtures::destination()]
        } else {
            Vec::new()
        };
        context.policy.evidence_sharing = if rng.chance(40) {
            EvidenceSharing::PolicyPermitted(vec![PrincipalId::new("p2")])
        } else {
            EvidenceSharing::SamePrincipalOnly
        };
        if rng.chance(20) {
            context.hypothetical_pass.issuer = PrincipalId::new("issuer-rogue");
        }
        if rng.chance(20) {
            context.hypothetical_pass.evidence_principal = PrincipalId::new("p2");
        }
        if rng.chance(30) {
            context.hypothetical_pass.audience =
                cegar_domain::AudienceId::new("audience-elsewhere");
        }
    }

    let stage_outputs = match rng.below(4) {
        0 => vec![fixtures::candidate_a()],
        1 => vec![fixtures::candidate_b()],
        _ => vec![fixtures::candidate_a(), fixtures::candidate_b()],
    };

    if !satisfiable {
        // Initial staging: sometimes A, sometimes B, sometimes both.
        context.staged = match rng.below(5) {
            0 => vec![fixtures::candidate_a()],
            1 => vec![fixtures::candidate_b()],
            2 => vec![fixtures::candidate_a(), fixtures::candidate_b()],
            _ => Vec::new(),
        };
    }

    let observation_status = match rng.below(4) {
        0 => TestStatus::Pass,
        1 => TestStatus::Fail,
        2 => TestStatus::Error,
        _ => TestStatus::Pass,
    };
    let observation_status = if satisfiable {
        TestStatus::Pass
    } else {
        observation_status
    };
    context.observations = vec![fixtures::observation(
        observation_status,
        fixtures::NOW - 30,
    )];

    let shape = index % 4;
    let nodes = match shape {
        0 => vec![
            fixtures::stage_node(1, &[]),
            fixtures::test_node(
                2,
                &[1],
                InputRef::Produced(NodeId(1)),
                test_suite(rng, satisfiable),
            ),
            fixtures::publish_node(
                3,
                &[2],
                InputRef::Produced(NodeId(1)),
                EvidenceRef::Produced(NodeId(2)),
                destination_for(rng, satisfiable),
            ),
        ],
        1 => vec![
            fixtures::test_node(
                1,
                &[],
                InputRef::Initial(fixtures::candidate_a()),
                test_suite(rng, satisfiable),
            ),
            fixtures::publish_node(
                2,
                &[1],
                InputRef::Initial(fixtures::candidate_a()),
                EvidenceRef::Produced(NodeId(1)),
                destination_for(rng, satisfiable),
            ),
            fixtures::stage_node(3, &[]),
        ],
        2 => vec![
            fixtures::stage_node(1, &[]),
            fixtures::stage_node(2, &[]),
            fixtures::test_node(
                3,
                &[2],
                InputRef::Produced(NodeId(2)),
                test_suite(rng, satisfiable),
            ),
            fixtures::publish_node(
                4,
                &[3],
                InputRef::Produced(NodeId(2)),
                EvidenceRef::Produced(NodeId(3)),
                destination_for(rng, satisfiable),
            ),
        ],
        _ => vec![
            fixtures::stage_node(1, &[]),
            fixtures::test_node(
                2,
                &[1],
                InputRef::Produced(NodeId(1)),
                test_suite(rng, satisfiable),
            ),
            fixtures::publish_node(
                3,
                &[2],
                InputRef::Initial(fixtures::candidate_b()),
                EvidenceRef::Observed(RunId::new("run-1")),
                destination_for(rng, satisfiable),
            ),
            fixtures::publish_node(
                4,
                &[2],
                InputRef::Produced(NodeId(1)),
                EvidenceRef::Produced(NodeId(2)),
                destination_for(rng, satisfiable),
            ),
        ],
    };

    let stage_nodes: Vec<NodeId> = nodes
        .iter()
        .filter(|node| matches!(node.kind, cegar_domain::NodeKind::Stage(_)))
        .map(|node| node.id)
        .collect();
    context.stage_outputs = stage_nodes
        .into_iter()
        .map(|node| StageOutputs {
            node,
            candidates: stage_outputs.clone(),
        })
        .collect();

    let mut plan = fixtures::plan(nodes);
    plan.disposition = if rng.chance(50) {
        Disposition::AbortAll
    } else {
        Disposition::ContinueIndependent
    };

    Case {
        name: format!("case-{index}-shape-{shape}"),
        plan,
        context,
    }
}

fn test_suite(rng: &mut Rng, satisfiable: bool) -> cegar_domain::SuiteRef {
    if satisfiable || rng.chance(50) {
        fixtures::suite_s1()
    } else {
        fixtures::suite_s2()
    }
}

fn publish_destination(rng: &mut Rng) -> DestinationId {
    if rng.chance(70) {
        fixtures::destination()
    } else {
        DestinationId::new("registry-elsewhere")
    }
}

/// Destination choice that respects a satisfiable case's pinning.
fn destination_for(rng: &mut Rng, satisfiable: bool) -> DestinationId {
    if satisfiable {
        fixtures::destination()
    } else {
        publish_destination(rng)
    }
}
