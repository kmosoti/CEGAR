//! Synthetic plan vocabulary for the experimental stage → registered test →
//! publish slice. These types are inputs to a deterministic compiler; they are
//! not a production schema and they carry no authority.

use crate::rules::Rule;
use crate::types::{
    ArtifactId, DestinationId, FormatId, OperationRef, PrincipalId, RunId, SuiteRef,
};

/// Maximum node count this experimental slice will model. Larger plans are
/// rejected before any graph is built rather than truncated.
pub const MAX_NODES: usize = 6;

/// Typed node identifier. Unique within one plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

/// Reference to one immutable input: either an initial snapshot artifact or a
/// unique producer output. A reference resolves exactly once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputRef {
    Produced(NodeId),
    Initial(ArtifactId),
}

/// Reference to modeled evidence: either the hypothetical attestation bound to
/// a test node's pass branch, or an initial validated observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceRef {
    Produced(NodeId),
    Observed(RunId),
}

/// Plan-level failure disposition. There is no default: a caller must choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// After any failure, every not-yet-started node becomes blocked.
    AbortAll,
    /// After any failure, unrelated ready nodes may still proceed.
    ContinueIndependent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StageSpec {
    pub operation: OperationRef,
    pub actor: PrincipalId,
    pub base: ArtifactId,
    pub patch: ArtifactId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestSpec {
    pub operation: OperationRef,
    pub actor: PrincipalId,
    pub subject: InputRef,
    pub suite: SuiteRef,
    pub runner: PrincipalId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishSpec {
    pub operation: OperationRef,
    pub actor: PrincipalId,
    pub subject: InputRef,
    pub evidence: EvidenceRef,
    pub destination: DestinationId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Stage(StageSpec),
    Test(TestSpec),
    Publish(PublishSpec),
}

impl NodeKind {
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::Stage(_) => "stage_patch",
            Self::Test(_) => "run_registered_tests",
            Self::Publish(_) => "publish",
        }
    }

    /// Typed role of the value this node binds on a success branch.
    pub const fn output_format(&self) -> Option<FormatId> {
        match self {
            Self::Stage(_) => Some(FormatId::Candidate),
            Self::Test(_) | Self::Publish(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub deps: Vec<NodeId>,
    pub kind: NodeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Monotonic experimental plan revision, bound into every receipt.
    pub revision: u32,
    pub disposition: Disposition,
    pub nodes: Vec<Node>,
}

/// One bounded diagnostic. Sorted and deduplicated before being reported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: Rule,
    pub detail: String,
}

impl Diagnostic {
    pub fn new(rule: Rule, detail: impl Into<String>) -> Self {
        Self {
            rule,
            detail: detail.into(),
        }
    }
}

/// Deterministic structural defects: everything decidable without the pinned
/// context. Registry membership and identity resolution are checked later.
pub fn structural_defects(plan: &Plan) -> Vec<Diagnostic> {
    let mut defects = Vec::new();

    if plan.nodes.is_empty() {
        defects.push(Diagnostic::new(Rule::EmptyPlan, "plan has no nodes"));
    }
    if plan.nodes.len() > MAX_NODES {
        defects.push(Diagnostic::new(
            Rule::PlanTooLarge,
            format!(
                "plan has {} nodes; the experimental bound is {MAX_NODES}",
                plan.nodes.len()
            ),
        ));
    }

    let mut seen: Vec<NodeId> = plan.nodes.iter().map(|node| node.id).collect();
    let declared = seen.len();
    seen.sort_unstable();
    let mut duplicate = false;
    for pair in seen.windows(2) {
        if pair[0] == pair[1] {
            duplicate = true;
            defects.push(Diagnostic::new(
                Rule::DuplicateNode,
                format!("node {} is declared more than once", pair[0].0),
            ));
        }
    }
    if duplicate {
        // Node identity is ambiguous, so dependency resolution and reachability
        // answers would be undefined. Stop rather than pick one interpretation.
        finish(&mut defects);
        return defects;
    }
    debug_assert_eq!(declared, seen.len());

    for node in &plan.nodes {
        let mut deps = node.deps.clone();
        deps.sort_unstable();
        for pair in deps.windows(2) {
            if pair[0] == pair[1] {
                defects.push(Diagnostic::new(
                    Rule::UnknownDependency,
                    format!("node {} lists dependency {} twice", node.id.0, pair[0].0),
                ));
            }
        }
        for dep in &node.deps {
            if *dep == node.id {
                defects.push(Diagnostic::new(
                    Rule::SelfDependency,
                    format!("node {} depends on itself", node.id.0),
                ));
            } else if !seen.binary_search(dep).is_ok() {
                defects.push(Diagnostic::new(
                    Rule::UnknownDependency,
                    format!("node {} depends on unknown node {}", node.id.0, dep.0),
                ));
            }
        }
        check_reference_shape(node, plan, &mut defects);
    }

    for cycle_member in cycle_members(plan) {
        defects.push(Diagnostic::new(
            Rule::CyclicPlan,
            format!("node {} participates in a dependency cycle", cycle_member.0),
        ));
    }

    finish(&mut defects);
    defects
}

fn check_reference_shape(node: &Node, plan: &Plan, defects: &mut Vec<Diagnostic>) {
    let producer = |id: NodeId| plan.nodes.iter().find(|candidate| candidate.id == id);

    match &node.kind {
        NodeKind::Stage(_) => {}
        NodeKind::Test(spec) => match &spec.subject {
            InputRef::Initial(artifact) => {
                if artifact.format != FormatId::Candidate {
                    defects.push(Diagnostic::new(
                            Rule::SubjectKind,
                            format!(
                                "node {} tests an initial {} input; only candidate-role artifacts are modeled subjects",
                                node.id.0,
                                artifact.format.tag()
                            ),
                        ));
                }
            }
            InputRef::Produced(id) => match producer(*id) {
                Some(producer_node) if producer_node.kind.output_format().is_some() => {}
                Some(_) => defects.push(Diagnostic::new(
                    Rule::SubjectKind,
                    format!(
                        "node {} tests the output of node {}, which binds no artifact",
                        node.id.0, id.0
                    ),
                )),
                None => {}
            },
        },
        NodeKind::Publish(spec) => {
            match &spec.subject {
                InputRef::Initial(artifact) => {
                    if artifact.format != FormatId::Candidate {
                        defects.push(Diagnostic::new(
                            Rule::SubjectKind,
                            format!(
                                "node {} publishes an initial {} input; only candidate-role artifacts are modeled subjects",
                                node.id.0,
                                artifact.format.tag()
                            ),
                        ));
                    }
                }
                InputRef::Produced(id) => match producer(*id) {
                    Some(producer_node) if producer_node.kind.output_format().is_some() => {}
                    Some(_) => defects.push(Diagnostic::new(
                        Rule::SubjectKind,
                        format!(
                            "node {} publishes the output of node {}, which binds no artifact",
                            node.id.0, id.0
                        ),
                    )),
                    None => {}
                },
            }
            if let EvidenceRef::Produced(id) = spec.evidence
                && let Some(producer_node) = producer(id)
                && !matches!(producer_node.kind, NodeKind::Test(_))
            {
                defects.push(Diagnostic::new(
                    Rule::UnknownProducerOutput,
                    format!(
                        "node {} designates evidence from node {}, which is not a test node",
                        node.id.0, id.0
                    ),
                ));
            }
        }
    }
}

/// Node identifiers that lie on a cycle, computed by Kahn elimination on nodes.
fn cycle_members(plan: &Plan) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = plan.nodes.iter().map(|node| node.id).collect();
    ids.sort_unstable();
    let index_of = |id: NodeId| ids.binary_search(&id).ok();

    let count = ids.len();
    let mut indegree = vec![0usize; count];
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); count];
    for node in &plan.nodes {
        let Some(from) = index_of(node.id) else {
            continue;
        };
        for dep in &node.deps {
            let Some(to) = index_of(*dep) else {
                continue;
            };
            out[to].push(from);
            indegree[from] += 1;
        }
    }

    let mut queue: Vec<usize> = (0..count).filter(|index| indegree[*index] == 0).collect();
    queue.sort_unstable();
    let mut removed = 0usize;
    let mut cursor = 0usize;
    while cursor < queue.len() {
        let current = queue[cursor];
        cursor += 1;
        removed += 1;
        for &next in &out[current] {
            indegree[next] -= 1;
            if indegree[next] == 0 {
                queue.push(next);
            }
        }
    }

    if removed == count {
        return Vec::new();
    }
    (0..count)
        .filter(|index| indegree[*index] > 0)
        .map(|index| ids[index])
        .collect()
}

fn finish(defects: &mut Vec<Diagnostic>) {
    defects.sort_by(|left, right| {
        left.rule
            .code()
            .cmp(right.rule.code())
            .then_with(|| left.detail.cmp(&right.detail))
    });
    defects.dedup();
}
