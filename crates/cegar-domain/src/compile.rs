//! Deterministic Sans-I/O compiler from a validated synthetic plan and pinned
//! context to a bounded first-violation monitor graph for `cegar-core`.
//!
//! The compiler is the trusted safety-critical piece of this experiment. It
//! never drops an unsafe ready attempt: an attempt whose modeled preconditions
//! are false becomes an explicit edge to the terminal `Bad` monitor labeled with
//! the violated rules. A `Bad` state has no outgoing edges; that is a reduction
//! for the stated safety question, not a claim that a real scheduler stops.

use std::collections::BTreeMap;

use cegar_core::{
    Limits, MAX_STATES, MAX_TRANSITIONS, Model, Outcome, Report, State, Transition, Witness,
    replay, verify,
};

use crate::context::{Context, TestStatus};
use crate::evidence::{self, EvidenceView};
use crate::plan::{Diagnostic, Disposition, EvidenceRef, InputRef, NodeId, NodeKind, Plan};
use crate::rules::Rule;
use crate::types::{
    ArtifactId, DestinationId, OperationRef, PrincipalId, RunId, SnapshotRef, SuiteRef,
};

/// Hard bound on modeled domain states, matching the core's structural cap.
pub const MAX_DOMAIN_STATES: usize = MAX_STATES;
/// Hard bound on modeled transitions, matching the core's structural cap.
pub const MAX_DOMAIN_TRANSITIONS: usize = MAX_TRANSITIONS;

/// Modeled status of one plan node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeStatus {
    Pending,
    Succeeded,
    Failed,
    Blocked,
}

impl NodeStatus {
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        }
    }
}

/// Where a modeled evidence record came from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceOrigin {
    /// A validated initial record (synthetic fixture in this experiment).
    Observed(RunId),
    /// A hypothetical pass bound to the given test node's branch.
    Hypothetical(NodeId),
}

impl EvidenceOrigin {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Self::Observed(run) => {
                out.extend_from_slice(b"observed:");
                run.encode(out);
            }
            Self::Hypothetical(node) => {
                out.extend_from_slice(b"hypothetical:");
                out.extend_from_slice(&node.0.to_le_bytes());
            }
        }
    }
}

/// One evidence record available in a modeled state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceItem {
    pub origin: EvidenceOrigin,
    pub subject: ArtifactId,
    pub suite: SuiteRef,
    pub status: TestStatus,
}

impl EvidenceItem {
    fn encode(&self, out: &mut Vec<u8>) {
        self.origin.encode(out);
        self.subject.encode(out);
        self.suite.encode(out);
        out.extend_from_slice(self.status.tag().as_bytes());
    }
}

fn encode_list<T>(out: &mut Vec<u8>, items: &[T], mut each: impl FnMut(&T, &mut Vec<u8>)) {
    out.extend_from_slice(&(items.len() as u32).to_le_bytes());
    for item in items {
        each(item, out);
    }
}

/// One modeled configuration that preserves every fact needed to decide
/// readiness, outcomes and policy-validity. Fields are indexed by node order or
/// kept sorted, never merged across different associations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveState {
    /// Per-node status, indexed by canonical node index.
    pub statuses: Vec<NodeStatus>,
    /// Per-node bound candidate identity, if the success branch bound one.
    pub outputs: Vec<Option<ArtifactId>>,
    /// Sorted exact staged identities.
    pub staged: Vec<ArtifactId>,
    /// Sorted evidence records.
    pub evidence: Vec<EvidenceItem>,
    /// Sorted exact modeled published identities.
    pub published: Vec<ArtifactId>,
    /// Number of modeled attempts already taken.
    pub steps: u32,
}

impl LiveState {
    fn canonical(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"live/v1");
        encode_list(&mut out, &self.statuses, |status, out| {
            out.extend_from_slice(status.tag().as_bytes())
        });
        encode_list(&mut out, &self.outputs, |binding, out| match binding {
            Some(artifact) => {
                out.push(1);
                artifact.encode(out);
            }
            None => out.push(0),
        });
        encode_list(&mut out, &self.staged, |artifact, out| artifact.encode(out));
        encode_list(&mut out, &self.evidence, |item, out| item.encode(out));
        encode_list(&mut out, &self.published, |artifact, out| {
            artifact.encode(out)
        });
        out.extend_from_slice(&self.steps.to_le_bytes());
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StateKeyKind {
    Bad,
    Live(LiveState),
}

/// Canonical state key. Ordering and equality are defined by the canonical
/// encoding, so dense IDs never depend on discovery order.
#[derive(Clone, Debug)]
pub struct StateKey(StateKeyKind);

impl StateKey {
    pub fn bad() -> Self {
        Self(StateKeyKind::Bad)
    }

    pub fn live(state: LiveState) -> Self {
        Self(StateKeyKind::Live(state))
    }

    pub fn as_live(&self) -> Option<&LiveState> {
        match &self.0 {
            StateKeyKind::Live(state) => Some(state),
            StateKeyKind::Bad => None,
        }
    }

    pub fn is_bad(&self) -> bool {
        matches!(self.0, StateKeyKind::Bad)
    }

    pub fn encode(&self) -> Vec<u8> {
        match &self.0 {
            StateKeyKind::Bad => b"bad/v1".to_vec(),
            StateKeyKind::Live(state) => state.canonical(),
        }
    }

    /// Deterministic, inspectable summary used in receipts and failure output.
    pub fn describe(&self) -> String {
        match &self.0 {
            StateKeyKind::Bad => "bad".to_owned(),
            StateKeyKind::Live(state) => {
                let statuses: Vec<&str> = state.statuses.iter().map(|item| item.tag()).collect();
                let staged: Vec<String> =
                    state.staged.iter().map(|item| item.to_string()).collect();
                let mut text = format!("[{},steps={}]", statuses.join(","), state.steps);
                if !staged.is_empty() {
                    text.push_str(&format!(" staged={{{}}}", staged.join(";")));
                }
                text
            }
        }
    }
}

impl PartialEq for StateKey {
    fn eq(&self, other: &Self) -> bool {
        self.encode() == other.encode()
    }
}

impl Eq for StateKey {}

impl PartialOrd for StateKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StateKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.encode().cmp(&other.encode())
    }
}

/// Trace label. Labels are audit data, never capabilities.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ActionKey {
    pub node: NodeId,
    pub operation: &'static str,
    pub outcome: &'static str,
    pub binding: Option<ArtifactId>,
    pub rules: Vec<Rule>,
}

impl ActionKey {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.node.0.to_le_bytes());
        out.extend_from_slice(self.operation.as_bytes());
        out.push(0);
        out.extend_from_slice(self.outcome.as_bytes());
        out.push(0);
        match self.binding {
            Some(artifact) => {
                out.push(1);
                artifact.encode(&mut out);
            }
            None => out.push(0),
        }
        encode_list(&mut out, &self.rules, |rule, out| {
            out.extend_from_slice(rule.code().as_bytes())
        });
        out
    }

    pub fn describe(&self) -> String {
        let mut text = format!("n{}:{}:{}", self.node.0, self.operation, self.outcome);
        if let Some(binding) = self.binding {
            text.push_str(&format!("->{binding}"));
        }
        if !self.rules.is_empty() {
            let codes: Vec<&str> = self.rules.iter().map(|rule| rule.code()).collect();
            text.push_str(&format!(" [{}]", codes.join(",")));
        }
        text
    }
}

/// One dense compiled edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledEdge {
    pub source: State,
    pub action: u32,
    pub target: State,
}

/// Inspectable experimental receipt. It records assumptions and identities; it
/// grants nothing and always reports `execution_authorized = false`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub semantics: &'static str,
    pub provenance: &'static str,
    pub plan_revision: u32,
    pub disposition: Disposition,
    pub policy: crate::types::PolicyRef,
    pub snapshot: SnapshotRef,
    pub node_count: usize,
    pub state_count: usize,
    pub transition_count: usize,
    pub bad_edges: usize,
    pub terminal_states: usize,
    pub action_table: Vec<String>,
    pub assumptions: Vec<String>,
    pub scope: &'static str,
    pub execution_authorized: bool,
}

/// A complete bounded compilation ready for the core.
#[derive(Clone, Debug)]
pub struct Compiled {
    pub model: Model,
    pub states: Vec<StateKey>,
    pub actions: Vec<ActionKey>,
    pub edges: Vec<CompiledEdge>,
    pub receipt: Receipt,
}

impl Compiled {
    pub fn bad_state(&self) -> Option<State> {
        self.states.iter().position(|state| state.is_bad())
    }

    /// Decode a core witness into canonical states and action labels.
    pub fn decode(&self, witness: &Witness) -> Option<Vec<(State, &StateKey, Option<&ActionKey>)>> {
        if witness.states.len() != witness.actions.len() + 1 {
            return None;
        }
        let mut decoded = Vec::with_capacity(witness.states.len());
        for (index, state) in witness.states.iter().enumerate() {
            let key = self.states.get(*state)?;
            let action = if index < witness.actions.len() {
                let action = witness.actions[index];
                Some(self.actions.get(action as usize)?)
            } else {
                None
            };
            decoded.push((*state, key, action));
        }
        Some(decoded)
    }
}

/// Why a compilation stopped before producing a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// A declared bound was reached; no partial graph is returned.
    Limit { rule: Rule, detail: String },
    /// Requested behavior is not representable in v0.1.
    Unsupported { rule: Rule, detail: String },
    /// A live state had pending work but no permitted transition.
    Stall { detail: String },
    /// The core rejected the generated graph (internal error).
    Core(String),
}

impl CompileError {
    pub fn diagnostic(&self) -> Diagnostic {
        match self {
            Self::Limit { rule, detail } | Self::Unsupported { rule, detail } => {
                Diagnostic::new(*rule, detail.clone())
            }
            Self::Stall { detail } => Diagnostic::new(Rule::UnexplainedStall, detail.clone()),
            Self::Core(detail) => Diagnostic::new(Rule::UnexplainedStall, detail.clone()),
        }
    }
}

/// Deterministic analysis result for one plan and pinned context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalysisOutcome {
    InvalidInput(Vec<Diagnostic>),
    StaleSnapshot(Vec<Diagnostic>),
    NeedsEvidence(Vec<Diagnostic>),
    Inconclusive(Vec<Diagnostic>),
    ConcreteCounterexample {
        /// Core witness with dense IDs.
        witness: Witness,
        /// Decoded action labels in order; the last one is the first violation.
        trace: Vec<ActionKey>,
        /// Rules violated by the terminal attempt.
        rules: Vec<Rule>,
    },
    VerifiedWithinModel,
}

impl AnalysisOutcome {
    pub const fn tag(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::StaleSnapshot(_) => "stale_snapshot",
            Self::NeedsEvidence(_) => "needs_evidence",
            Self::Inconclusive(_) => "inconclusive",
            Self::ConcreteCounterexample { .. } => "concrete_counterexample",
            Self::VerifiedWithinModel => "verified_within_model",
        }
    }

    pub const fn execution_authorized(&self) -> bool {
        false
    }
}

/// Analysis result plus the receipt when a graph was produced.
#[derive(Clone, Debug)]
pub struct Analysis {
    pub outcome: AnalysisOutcome,
    pub receipt: Option<Receipt>,
    pub report: Option<Report>,
}

/// Compiler bounds. Defaults match the core's structural caps.
#[derive(Clone, Copy, Debug)]
pub struct CompileLimits {
    pub max_states: usize,
    pub max_transitions: usize,
    pub core: Limits,
}

impl Default for CompileLimits {
    fn default() -> Self {
        Self {
            max_states: MAX_DOMAIN_STATES,
            max_transitions: MAX_DOMAIN_TRANSITIONS,
            core: Limits::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum SubjectRef {
    Producer(usize),
    Initial(ArtifactId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EvidenceRefResolved {
    Producer(usize),
    Observed(RunId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ResolvedKind {
    Stage {
        operation: OperationRef,
        actor: PrincipalId,
        base: ArtifactId,
        patch: ArtifactId,
        outputs: Vec<ArtifactId>,
    },
    Test {
        operation: OperationRef,
        actor: PrincipalId,
        subject: SubjectRef,
        suite: SuiteRef,
        runner: PrincipalId,
    },
    Publish {
        operation: OperationRef,
        actor: PrincipalId,
        subject: SubjectRef,
        evidence: EvidenceRefResolved,
        destination: DestinationId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedNode {
    id: NodeId,
    deps: Vec<usize>,
    kind: ResolvedKind,
}

#[derive(Clone, Debug)]
struct Resolved {
    nodes: Vec<ResolvedNode>,
    disposition: Disposition,
    revision: u32,
}

#[derive(Default)]
struct Resolution {
    invalid: Vec<Diagnostic>,
    needs_evidence: Vec<Diagnostic>,
    unsupported: Vec<Diagnostic>,
}

impl Resolution {
    fn push(&mut self, bucket: Bucket, diagnostic: Diagnostic) {
        match bucket {
            Bucket::Invalid => self.invalid.push(diagnostic),
            Bucket::NeedsEvidence => self.needs_evidence.push(diagnostic),
            Bucket::Unsupported => self.unsupported.push(diagnostic),
        }
    }

    fn finish_buckets(&mut self) {
        for bucket in [
            &mut self.invalid,
            &mut self.needs_evidence,
            &mut self.unsupported,
        ] {
            sort_diagnostics(bucket);
        }
    }
}

enum Bucket {
    Invalid,
    NeedsEvidence,
    Unsupported,
}

fn sort_diagnostics(diagnostics: &mut Vec<Diagnostic>) {
    diagnostics.sort_by(|left, right| {
        left.rule
            .code()
            .cmp(right.rule.code())
            .then_with(|| left.detail.cmp(&right.detail))
    });
    diagnostics.dedup();
}

fn has_dependency_path(nodes: &[ResolvedNode], from: usize, to: usize) -> bool {
    let mut stack = vec![to];
    let mut seen = vec![false; nodes.len()];
    while let Some(current) = stack.pop() {
        if seen[current] {
            continue;
        }
        seen[current] = true;
        for dep in &nodes[current].deps {
            if *dep == from {
                return true;
            }
            stack.push(*dep);
        }
    }
    false
}

fn resolve(plan: &Plan, context: &Context) -> Result<Resolved, Resolution> {
    let mut problems = Resolution::default();

    // Canonical node order fixes every index used below.
    let mut ordered: Vec<&crate::plan::Node> = plan.nodes.iter().collect();
    ordered.sort_by_key(|node| node.id);
    let index_of: BTreeMap<NodeId, usize> = ordered
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect();

    let mut nodes = Vec::with_capacity(ordered.len());
    for node in &ordered {
        let mut deps: Vec<usize> = match node
            .deps
            .iter()
            .map(|dep| index_of.get(dep).copied())
            .collect::<Option<Vec<usize>>>()
        {
            Some(deps) => deps,
            None => {
                // Structural validation already reported the unknown dependency.
                return Err(problems);
            }
        };
        deps.sort_unstable();
        deps.dedup();

        let kind = match &node.kind {
            NodeKind::Stage(spec) => {
                if !context.tool_registry.contains(&spec.operation) {
                    problems.push(
                        Bucket::Invalid,
                        Diagnostic::new(
                            Rule::UnknownOperation,
                            format!(
                                "node {} uses operation {} which is not in the pinned registry",
                                node.id.0, spec.operation.name
                            ),
                        ),
                    );
                }
                for artifact in [&spec.base, &spec.patch] {
                    if !context.artifacts.contains(artifact) {
                        problems.push(
                            Bucket::Invalid,
                            Diagnostic::new(
                                Rule::InputMissing,
                                format!(
                                    "node {} references an input that is not in the snapshot: {artifact}",
                                    node.id.0
                                ),
                            ),
                        );
                    }
                }
                let outputs = match context.stage_candidates(node.id) {
                    Some(candidates) if !candidates.is_empty() => candidates.to_vec(),
                    Some(_) => {
                        problems.push(
                            Bucket::Unsupported,
                            Diagnostic::new(
                                Rule::UnsupportedSemantics,
                                format!(
                                    "node {} is assumed to have an empty stage-output set",
                                    node.id.0
                                ),
                            ),
                        );
                        Vec::new()
                    }
                    None => {
                        problems.push(
                            Bucket::Unsupported,
                            Diagnostic::new(
                                Rule::UnsupportedSemantics,
                                format!(
                                    "node {} has no declared stage-output assumption; every permitted output identity must be enumerated",
                                    node.id.0
                                ),
                            ),
                        );
                        Vec::new()
                    }
                };
                ResolvedKind::Stage {
                    operation: spec.operation.clone(),
                    actor: spec.actor.clone(),
                    base: spec.base,
                    patch: spec.patch,
                    outputs,
                }
            }
            NodeKind::Test(spec) => {
                if !context.tool_registry.contains(&spec.operation) {
                    problems.push(
                        Bucket::Invalid,
                        Diagnostic::new(
                            Rule::UnknownOperation,
                            format!(
                                "node {} uses operation {} which is not in the pinned registry",
                                node.id.0, spec.operation.name
                            ),
                        ),
                    );
                }
                if !context.suite_registry.contains(&spec.suite) {
                    problems.push(
                        Bucket::Invalid,
                        Diagnostic::new(
                            Rule::SuiteUnregistered,
                            format!(
                                "node {} references unregistered suite {}",
                                node.id.0, spec.suite.name
                            ),
                        ),
                    );
                }
                let subject = resolve_subject(&spec.subject, &index_of);
                ResolvedKind::Test {
                    operation: spec.operation.clone(),
                    actor: spec.actor.clone(),
                    subject,
                    suite: spec.suite.clone(),
                    runner: spec.runner.clone(),
                }
            }
            NodeKind::Publish(spec) => {
                if !context.tool_registry.contains(&spec.operation) {
                    problems.push(
                        Bucket::Invalid,
                        Diagnostic::new(
                            Rule::UnknownOperation,
                            format!(
                                "node {} uses operation {} which is not in the pinned registry",
                                node.id.0, spec.operation.name
                            ),
                        ),
                    );
                }
                let subject = resolve_subject(&spec.subject, &index_of);
                let evidence = match &spec.evidence {
                    EvidenceRef::Produced(id) => match index_of.get(id) {
                        Some(index) => EvidenceRefResolved::Producer(*index),
                        None => {
                            problems.push(
                                Bucket::Invalid,
                                Diagnostic::new(
                                    Rule::UnknownProducerOutput,
                                    format!(
                                        "node {} designates evidence from unknown node {}",
                                        node.id.0, id.0
                                    ),
                                ),
                            );
                            EvidenceRefResolved::Observed(RunId::default())
                        }
                    },
                    EvidenceRef::Observed(run) => {
                        if context.observation(run).is_none() {
                            problems.push(
                                Bucket::NeedsEvidence,
                                Diagnostic::new(
                                    Rule::EvidenceMissing,
                                    format!(
                                        "node {} requires observation {} which the context does not provide",
                                        node.id.0, run
                                    ),
                                ),
                            );
                        }
                        EvidenceRefResolved::Observed(run.clone())
                    }
                };
                ResolvedKind::Publish {
                    operation: spec.operation.clone(),
                    actor: spec.actor.clone(),
                    subject,
                    evidence,
                    destination: spec.destination.clone(),
                }
            }
        };

        nodes.push(ResolvedNode {
            id: node.id,
            deps,
            kind,
        });
    }

    // Producer references must have a dependency path from the unique producer;
    // otherwise the reference could never be satisfied and the model would be
    // unsound rather than merely unsafe.
    for (index, node) in nodes.iter().enumerate() {
        let mut producers: Vec<(usize, u32)> = Vec::new();
        match &node.kind {
            ResolvedKind::Stage { .. } => {}
            ResolvedKind::Test { subject, .. } => {
                if let SubjectRef::Producer(producer) = subject {
                    producers.push((*producer, node.id.0));
                }
            }
            ResolvedKind::Publish {
                subject, evidence, ..
            } => {
                if let SubjectRef::Producer(producer) = subject {
                    producers.push((*producer, node.id.0));
                }
                if let EvidenceRefResolved::Producer(producer) = evidence {
                    producers.push((*producer, node.id.0));
                }
            }
        }
        for (producer, _) in producers {
            if !has_dependency_path(&nodes, producer, index) {
                problems.push(
                    Bucket::Invalid,
                    Diagnostic::new(
                        Rule::UnknownProducerOutput,
                        format!(
                            "node {} references node {} without a dependency path",
                            node.id.0, nodes[producer].id.0
                        ),
                    ),
                );
            }
        }
    }

    problems.finish_buckets();
    if !problems.invalid.is_empty()
        || !problems.needs_evidence.is_empty()
        || !problems.unsupported.is_empty()
    {
        return Err(problems);
    }

    Ok(Resolved {
        nodes,
        disposition: plan.disposition,
        revision: plan.revision,
    })
}

fn resolve_subject(reference: &InputRef, index_of: &BTreeMap<NodeId, usize>) -> SubjectRef {
    match reference {
        InputRef::Produced(id) => match index_of.get(id) {
            Some(index) => SubjectRef::Producer(*index),
            // Structural validation already rejected unknown producers.
            None => SubjectRef::Initial(ArtifactId::new(
                crate::types::synthetic_digest(0),
                crate::types::FormatId::Candidate,
            )),
        },
        InputRef::Initial(artifact) => SubjectRef::Initial(*artifact),
    }
}

// ---------------------------------------------------------------------------
// Exploration
// ---------------------------------------------------------------------------

enum Branch {
    To { next: LiveState, action: ActionKey },
    Bad(ActionKey),
}

struct Explorer<'a> {
    resolved: &'a Resolved,
    context: &'a Context,
    limits: CompileLimits,
}

impl Explorer<'_> {
    fn initial_state(&self) -> LiveState {
        let mut staged = self.context.staged.clone();
        staged.sort_unstable();
        staged.dedup();
        let mut evidence: Vec<EvidenceItem> = self
            .context
            .observations
            .iter()
            .map(|observation| EvidenceItem {
                origin: EvidenceOrigin::Observed(observation.run.clone()),
                subject: observation.subject,
                suite: observation.suite.clone(),
                status: observation.status,
            })
            .collect();
        evidence.sort();
        LiveState {
            statuses: vec![NodeStatus::Pending; self.resolved.nodes.len()],
            outputs: vec![None; self.resolved.nodes.len()],
            staged,
            evidence,
            published: Vec::new(),
            steps: 0,
        }
    }

    fn node(&self, index: usize) -> &ResolvedNode {
        &self.resolved.nodes[index]
    }

    fn ready(&self, state: &LiveState) -> Vec<usize> {
        (0..self.resolved.nodes.len())
            .filter(|&index| state.statuses[index] == NodeStatus::Pending)
            .filter(|&index| {
                self.node(index)
                    .deps
                    .iter()
                    .all(|dep| state.statuses[*dep] == NodeStatus::Succeeded)
            })
            .collect()
    }

    fn terminal(&self, state: &LiveState) -> bool {
        state
            .statuses
            .iter()
            .all(|status| *status != NodeStatus::Pending)
    }

    fn permission_rules(&self, operation: &OperationRef, actor: &PrincipalId) -> Vec<Rule> {
        let mut rules = Vec::new();
        if !self.context.policy.operation_authorized(operation) {
            rules.push(Rule::OperationUnauthorized);
        }
        if !self.context.policy.actor_authorized(actor) {
            rules.push(Rule::ActorUnauthorized);
        }
        Rule::sort_codes(&mut rules);
        rules
    }

    fn subject_of(&self, state: &LiveState, subject: &SubjectRef) -> Option<ArtifactId> {
        match subject {
            SubjectRef::Producer(index) => state.outputs[*index],
            SubjectRef::Initial(artifact) => Some(*artifact),
        }
    }

    fn evidence_item<'a>(
        &self,
        state: &'a LiveState,
        reference: &EvidenceRefResolved,
    ) -> Option<&'a EvidenceItem> {
        let origin = match reference {
            EvidenceRefResolved::Producer(index) => {
                EvidenceOrigin::Hypothetical(self.node(*index).id)
            }
            EvidenceRefResolved::Observed(run) => EvidenceOrigin::Observed(run.clone()),
        };
        state.evidence.iter().find(|item| item.origin == origin)
    }

    fn evidence_view(&self, item: &EvidenceItem) -> Option<EvidenceView<'_>> {
        match &item.origin {
            EvidenceOrigin::Observed(run) => {
                let observation = self.context.observation(run)?;
                Some(EvidenceView {
                    status: observation.status,
                    subject: observation.subject,
                    suite: &observation.suite,
                    runner: &observation.runner,
                    issuer: &observation.issuer,
                    evidence_principal: &observation.evidence_principal,
                    tenant: &observation.tenant,
                    audience: &observation.audience,
                    issued_at: observation.issued_at,
                    validity: observation.validity,
                    snapshot: &self.context.snapshot,
                })
            }
            EvidenceOrigin::Hypothetical(node) => {
                let producer = self
                    .resolved
                    .nodes
                    .iter()
                    .find(|candidate| candidate.id == *node)?;
                let ResolvedKind::Test { suite, runner, .. } = &producer.kind else {
                    return None;
                };
                let profile = &self.context.hypothetical_pass;
                Some(EvidenceView {
                    status: TestStatus::Pass,
                    subject: item.subject,
                    suite,
                    runner,
                    issuer: &profile.issuer,
                    evidence_principal: &profile.evidence_principal,
                    tenant: &profile.tenant,
                    audience: &profile.audience,
                    issued_at: self.context.now,
                    validity: profile.validity,
                    snapshot: &self.context.snapshot,
                })
            }
        }
    }

    fn close(&self, state: &mut LiveState) {
        let abort_all = matches!(self.disposition(), Disposition::AbortAll);
        loop {
            let mut changed = false;
            let any_failure = state
                .statuses
                .iter()
                .any(|status| matches!(status, NodeStatus::Failed | NodeStatus::Blocked));
            for index in 0..state.statuses.len() {
                if state.statuses[index] != NodeStatus::Pending {
                    continue;
                }
                let dependency_failed = self.node(index).deps.iter().any(|dep| {
                    matches!(
                        state.statuses[*dep],
                        NodeStatus::Failed | NodeStatus::Blocked
                    )
                });
                if dependency_failed || (abort_all && any_failure) {
                    state.statuses[index] = NodeStatus::Blocked;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn disposition(&self) -> Disposition {
        self.resolved.disposition
    }

    fn action(
        &self,
        index: usize,
        outcome: &'static str,
        binding: Option<ArtifactId>,
        rules: Vec<Rule>,
    ) -> ActionKey {
        ActionKey {
            node: self.node(index).id,
            operation: match &self.node(index).kind {
                ResolvedKind::Stage { .. } => "stage_patch",
                ResolvedKind::Test { .. } => "run_registered_tests",
                ResolvedKind::Publish { .. } => "publish",
            },
            outcome,
            binding,
            rules,
        }
    }

    fn branches(&self, state: &LiveState, index: usize) -> Result<Vec<Branch>, CompileError> {
        let node = self.node(index);
        match &node.kind {
            ResolvedKind::Stage {
                operation,
                actor,
                base,
                patch,
                outputs,
            } => {
                let mut rules = self.permission_rules(operation, actor);
                for artifact in [base, patch] {
                    if !self.context.artifacts.contains(artifact) {
                        rules.push(Rule::InputMissing);
                    }
                }
                Rule::sort_codes(&mut rules);
                if !rules.is_empty() {
                    return Ok(vec![Branch::Bad(self.action(
                        index,
                        "unsafe_attempt",
                        None,
                        rules,
                    ))]);
                }
                let mut branches = Vec::new();
                for candidate in outputs {
                    let mut next = state.clone();
                    next.statuses[index] = NodeStatus::Succeeded;
                    next.outputs[index] = Some(*candidate);
                    next.staged.push(*candidate);
                    next.staged.sort_unstable();
                    next.staged.dedup();
                    next.steps += 1;
                    self.close(&mut next);
                    branches.push(Branch::To {
                        next,
                        action: self.action(index, "success", Some(*candidate), Vec::new()),
                    });
                }
                let mut failed = state.clone();
                failed.statuses[index] = NodeStatus::Failed;
                failed.steps += 1;
                self.close(&mut failed);
                branches.push(Branch::To {
                    next: failed,
                    action: self.action(index, "failure_no_effect", None, Vec::new()),
                });
                Ok(branches)
            }
            ResolvedKind::Test {
                operation,
                actor,
                subject,
                suite,
                runner,
            } => {
                let mut rules = self.permission_rules(operation, actor);
                if !self.context.policy.test_suite_permitted(suite) {
                    rules.push(Rule::TestSuiteNotPermitted);
                }
                if !self.context.policy.runner_authorized(runner) {
                    rules.push(Rule::RunnerUnauthorized);
                }
                match self.subject_of(state, subject) {
                    Some(artifact) if state.staged.contains(&artifact) => {}
                    _ => rules.push(Rule::SubjectNotStaged),
                }
                Rule::sort_codes(&mut rules);
                if !rules.is_empty() {
                    return Ok(vec![Branch::Bad(self.action(
                        index,
                        "unsafe_attempt",
                        None,
                        rules,
                    ))]);
                }
                let artifact =
                    self.subject_of(state, subject)
                        .ok_or_else(|| CompileError::Stall {
                            detail: format!(
                                "node {} resolved no subject after passing preconditions",
                                node.id.0
                            ),
                        })?;
                let mut branches = Vec::new();
                for (outcome, status) in [
                    ("pass", NodeStatus::Succeeded),
                    ("fail", NodeStatus::Failed),
                    ("error", NodeStatus::Failed),
                ] {
                    let mut next = state.clone();
                    next.statuses[index] = status;
                    if status == NodeStatus::Succeeded {
                        next.evidence.push(EvidenceItem {
                            origin: EvidenceOrigin::Hypothetical(node.id),
                            subject: artifact,
                            suite: suite.clone(),
                            status: TestStatus::Pass,
                        });
                        next.evidence.sort();
                    }
                    next.steps += 1;
                    self.close(&mut next);
                    branches.push(Branch::To {
                        next,
                        action: self.action(index, outcome, None, Vec::new()),
                    });
                }
                Ok(branches)
            }
            ResolvedKind::Publish {
                operation,
                actor,
                subject,
                evidence: evidence_ref,
                destination,
            } => {
                let mut rules = self.permission_rules(operation, actor);
                let candidate = match self.subject_of(state, subject) {
                    Some(artifact) if state.staged.contains(&artifact) => Some(artifact),
                    _ => {
                        rules.push(Rule::SubjectNotStaged);
                        None
                    }
                };
                let mut specific: Vec<Rule> = Vec::new();
                match self.evidence_item(state, evidence_ref) {
                    None => specific.push(Rule::EvidenceMissing),
                    Some(item) => {
                        // The scoped predicate compares the attested subject with
                        // the exact publish subject, so it can only be evaluated
                        // once a staged candidate is bound. An unbound candidate
                        // is already recorded as its own violated rule.
                        if let Some(candidate) = candidate {
                            match self.evidence_view(item) {
                                Some(view) => {
                                    specific = evidence::violations(
                                        &view,
                                        &candidate,
                                        destination,
                                        &self.context.snapshot,
                                        &self.context.policy,
                                        self.context.now,
                                    );
                                }
                                None => specific.push(Rule::EvidenceMissing),
                            }
                        }
                    }
                }
                rules.extend(specific);
                Rule::sort_codes(&mut rules);
                if !rules.is_empty() {
                    // A false precondition is itself the unsafe attempt; it is
                    // modeled explicitly rather than pruned.
                    return Ok(vec![Branch::Bad(self.action(
                        index,
                        "unsafe_attempt",
                        None,
                        rules,
                    ))]);
                }
                let Some(candidate) = candidate else {
                    return Err(CompileError::Stall {
                        detail: format!(
                            "node {} passed every publish precondition but bound no candidate",
                            node.id.0
                        ),
                    });
                };
                let mut branches = Vec::new();
                let mut succeeded = state.clone();
                succeeded.statuses[index] = NodeStatus::Succeeded;
                succeeded.published.push(candidate);
                succeeded.published.sort_unstable();
                succeeded.published.dedup();
                succeeded.steps += 1;
                self.close(&mut succeeded);
                branches.push(Branch::To {
                    next: succeeded,
                    action: self.action(index, "success", Some(candidate), Vec::new()),
                });
                let mut failed = state.clone();
                failed.statuses[index] = NodeStatus::Failed;
                failed.steps += 1;
                self.close(&mut failed);
                branches.push(Branch::To {
                    next: failed,
                    action: self.action(index, "failure_no_effect", None, Vec::new()),
                });
                Ok(branches)
            }
        }
    }
}

/// One compiled graph before dense renumbering.
struct Explored {
    states: Vec<StateKey>,
    edges: Vec<(usize, ActionKey, usize)>,
    initial: Vec<usize>,
    bad: usize,
}

fn explore(explorer: &Explorer<'_>) -> Result<Explored, CompileError> {
    let initial = StateKey::live(explorer.initial_state());
    let mut states: Vec<StateKey> = vec![StateKey::bad()];
    let mut index: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    index.insert(states[0].encode(), 0);
    states.push(initial.clone());
    index.insert(initial.encode(), 1);
    let mut queue = std::collections::VecDeque::from([1usize]);
    let mut edges: Vec<(usize, ActionKey, usize)> = Vec::new();

    while let Some(current) = queue.pop_front() {
        let key = states[current].clone();
        let Some(state) = key.as_live() else {
            continue;
        };
        if explorer.terminal(state) {
            continue;
        }
        let ready = explorer.ready(state);
        if ready.is_empty() {
            return Err(CompileError::Stall {
                detail: format!(
                    "state {} has pending nodes but no ready node",
                    key.describe()
                ),
            });
        }
        for node_index in ready {
            for branch in explorer.branches(state, node_index)? {
                match branch {
                    Branch::Bad(action) => {
                        edges.push((current, action, 0));
                    }
                    Branch::To { next, action } => {
                        let next_key = StateKey::live(next);
                        let encoded = next_key.encode();
                        let target = match index.get(&encoded) {
                            Some(target) => *target,
                            None => {
                                if states.len() >= explorer.limits.max_states {
                                    return Err(CompileError::Limit {
                                        rule: Rule::GraphLimit,
                                        detail: format!(
                                            "enumeration exceeded the {}-state bound",
                                            explorer.limits.max_states
                                        ),
                                    });
                                }
                                states.push(next_key);
                                let id = states.len() - 1;
                                index.insert(encoded, id);
                                queue.push_back(id);
                                id
                            }
                        };
                        edges.push((current, action, target));
                        if edges.len() > explorer.limits.max_transitions {
                            return Err(CompileError::Limit {
                                rule: Rule::GraphLimit,
                                detail: format!(
                                    "enumeration exceeded the {}-transition bound",
                                    explorer.limits.max_transitions
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(Explored {
        states,
        edges,
        initial: vec![1],
        bad: 0,
    })
}

fn assemble(
    explored: Explored,
    explorer: &Explorer<'_>,
    context: &Context,
) -> Result<Compiled, CompileError> {
    // Canonical dense renumbering: states sorted by canonical encoding.
    let mut order: Vec<usize> = (0..explored.states.len()).collect();
    order.sort_by(|left, right| {
        explored.states[*left]
            .encode()
            .cmp(&explored.states[*right].encode())
    });
    let mut dense_of = vec![0usize; explored.states.len()];
    for (dense, original) in order.iter().enumerate() {
        dense_of[*original] = dense;
    }
    let states: Vec<StateKey> = order
        .iter()
        .map(|index| explored.states[*index].clone())
        .collect();

    // Action table sorted by canonical label encoding.
    let labels: Vec<ActionKey> = explored
        .edges
        .iter()
        .map(|(_, action, _)| action.clone())
        .collect();
    let mut encoded: Vec<(Vec<u8>, usize)> = labels
        .iter()
        .enumerate()
        .map(|(index, action)| (action.encode(), index))
        .collect();
    encoded.sort();
    encoded.dedup_by(|left, right| left.0 == right.0);
    let mut actions = Vec::with_capacity(encoded.len());
    let mut action_id_of: BTreeMap<Vec<u8>, u32> = BTreeMap::new();
    for (bytes, original) in encoded {
        let id = actions.len() as u32;
        action_id_of.insert(bytes, id);
        actions.push(labels[original].clone());
    }

    let mut transitions: Vec<Transition> = explored
        .edges
        .iter()
        .map(|(source, action, target)| Transition {
            source: dense_of[*source],
            action: action_id_of
                .get(&action.encode())
                .copied()
                .unwrap_or_default(),
            target: dense_of[*target],
        })
        .collect();
    if transitions.len() > explorer.limits.max_transitions {
        return Err(CompileError::Limit {
            rule: Rule::GraphLimit,
            detail: format!(
                "{} transitions exceed the {}-transition bound",
                transitions.len(),
                explorer.limits.max_transitions
            ),
        });
    }
    transitions.sort_by_key(|edge| (edge.source, edge.action, edge.target));
    transitions.dedup();

    let initial: Vec<State> = explored
        .initial
        .iter()
        .map(|index| dense_of[*index])
        .collect();
    let bad: Vec<State> = vec![dense_of[explored.bad]];

    let model = Model::new(states.len(), &initial, &bad, &transitions).map_err(|error| {
        CompileError::Core(format!("core rejected the generated graph: {error}"))
    })?;

    let mut edges: Vec<CompiledEdge> = transitions
        .iter()
        .map(|edge| CompiledEdge {
            source: edge.source,
            action: edge.action,
            target: edge.target,
        })
        .collect();
    edges.sort_by_key(|edge| (edge.source, edge.action, edge.target));

    let bad_edges = edges
        .iter()
        .filter(|edge| states[edge.target].is_bad())
        .count();
    let terminal_states = states
        .iter()
        .filter_map(StateKey::as_live)
        .filter(|state| explorer.terminal(state))
        .count();

    let receipt = Receipt {
        semantics: crate::types::DOMAIN_SEMANTICS,
        provenance: context.provenance.tag(),
        plan_revision: explorer.resolved.revision,
        disposition: explorer.resolved.disposition,
        policy: context.policy.reference.clone(),
        snapshot: context.snapshot.clone(),
        node_count: explorer.resolved.nodes.len(),
        state_count: states.len(),
        transition_count: edges.len(),
        bad_edges,
        terminal_states,
        action_table: actions.iter().map(ActionKey::describe).collect(),
        assumptions: assumptions(explorer, context),
        scope: "supplied_synthetic_domain_model_only",
        execution_authorized: false,
    };

    Ok(Compiled {
        model,
        states,
        actions,
        edges,
        receipt,
    })
}

fn assumptions(explorer: &Explorer<'_>, context: &Context) -> Vec<String> {
    let mut items = vec![
        format!("disposition={:?}", explorer.resolved.disposition),
        format!("provenance={}", context.provenance.tag()),
        "atomic model: one attempt per node, no retries, no partial effects".to_owned(),
        "modeled outcomes: stage success/failure_no_effect, test pass/fail/error, publish success/failure_no_effect".to_owned(),
        format!(
            "hypothetical pass profile: issuer={}, evidence_principal={}, tenant={}, audience={}, validity={}s",
            context.hypothetical_pass.issuer,
            context.hypothetical_pass.evidence_principal,
            context.hypothetical_pass.tenant,
            context.hypothetical_pass.audience,
            context.hypothetical_pass.validity
        ),
        format!("modeled evaluation time={}s", context.now),
    ];
    for node in &explorer.resolved.nodes {
        if let ResolvedKind::Stage { outputs, .. } = &node.kind {
            let ids: Vec<String> = outputs.iter().map(|item| item.to_string()).collect();
            items.push(format!(
                "stage node {} permitted outputs: [{}]",
                node.id.0,
                ids.join("; ")
            ));
        }
    }
    items.push("a verified result is absence of Bad in this specified graph only".to_owned());
    items
}

// ---------------------------------------------------------------------------
// Public analysis entry point
// ---------------------------------------------------------------------------

/// Compile and analyze one plan against one pinned context.
///
/// Category precedence follows the proposed domain classification:
/// `invalid_input`, `stale_snapshot`, `needs_evidence`, `inconclusive`, then
/// replayed counterexample or model-scoped verification.
pub fn analyze(plan: &Plan, context: &Context, limits: CompileLimits) -> Analysis {
    let structural = crate::plan::structural_defects(plan);
    if !structural.is_empty() {
        return invalid(structural);
    }

    let mut invalid_premises = Vec::new();
    let mut stale = Vec::new();
    for defect in context.premise_defects() {
        if defect.rule == Rule::StaleSnapshot {
            stale.push(defect);
        } else {
            invalid_premises.push(defect);
        }
    }
    if !invalid_premises.is_empty() {
        return invalid(invalid_premises);
    }
    if !stale.is_empty() {
        return Analysis {
            outcome: AnalysisOutcome::StaleSnapshot(stale),
            receipt: None,
            report: None,
        };
    }

    let resolved = match resolve(plan, context) {
        Ok(resolved) => resolved,
        Err(resolution) => {
            if !resolution.invalid.is_empty() {
                return invalid(resolution.invalid);
            }
            if !resolution.needs_evidence.is_empty() {
                return Analysis {
                    outcome: AnalysisOutcome::NeedsEvidence(resolution.needs_evidence),
                    receipt: None,
                    report: None,
                };
            }
            return inconclusive(resolution.unsupported);
        }
    };

    let explorer = Explorer {
        resolved: &resolved,
        context,
        limits,
    };

    let explored = match explore(&explorer) {
        Ok(explored) => explored,
        Err(error) => return inconclusive(vec![error.diagnostic()]),
    };

    let compiled = match assemble(explored, &explorer, context) {
        Ok(compiled) => compiled,
        Err(error) => return inconclusive(vec![error.diagnostic()]),
    };

    let report = verify(&compiled.model, limits.core);
    let mut outcome = match &report.outcome {
        Outcome::VerifiedWithinModel => AnalysisOutcome::VerifiedWithinModel,
        Outcome::Inconclusive(reason) => {
            return Analysis {
                outcome: AnalysisOutcome::Inconclusive(vec![Diagnostic::new(
                    Rule::GraphLimit,
                    format!("core returned inconclusive: {reason:?}"),
                )]),
                receipt: Some(compiled.receipt.clone()),
                report: Some(report),
            };
        }
        Outcome::ConcreteCounterexample(witness) => {
            // Domain-level replay: decode labels and confirm the terminal
            // attempt is the recorded violation.
            if replay(&compiled.model, witness).is_err() {
                return Analysis {
                    outcome: AnalysisOutcome::Inconclusive(vec![Diagnostic::new(
                        Rule::UnexplainedStall,
                        "core witness failed independent replay",
                    )]),
                    receipt: Some(compiled.receipt.clone()),
                    report: Some(report),
                };
            }
            let Some(decoded) = compiled.decode(witness) else {
                return Analysis {
                    outcome: AnalysisOutcome::Inconclusive(vec![Diagnostic::new(
                        Rule::UnexplainedStall,
                        "witness could not be decoded against the compiled mapping",
                    )]),
                    receipt: Some(compiled.receipt.clone()),
                    report: Some(report),
                };
            };
            let mut trace = Vec::with_capacity(decoded.len());
            let mut rules = Vec::new();
            for (_, _state, action) in &decoded {
                if let Some(action) = action {
                    trace.push((*action).clone());
                }
            }
            if let Some(last) = trace.last()
                && last.outcome == "unsafe_attempt"
            {
                let mut violated = last.rules.clone();
                Rule::sort_codes(&mut violated);
                rules = violated;
            }
            if rules.is_empty() {
                return Analysis {
                    outcome: AnalysisOutcome::Inconclusive(vec![Diagnostic::new(
                        Rule::UnsupportedSemantics,
                        "counterexample does not terminate in a modeled first violation",
                    )]),
                    receipt: Some(compiled.receipt.clone()),
                    report: Some(report),
                };
            }
            AnalysisOutcome::ConcreteCounterexample {
                witness: witness.clone(),
                trace,
                rules,
            }
        }
    };

    // A verified result requires that every live state was fully expanded: the
    // explorer already rejects unexplained stalls, and the compiler never
    // truncates. Record the terminal-state count so a reader can see that a
    // blocked deadlock is not treated as completion.
    if matches!(outcome, AnalysisOutcome::VerifiedWithinModel) && compiled.receipt.bad_edges != 0 {
        outcome = AnalysisOutcome::Inconclusive(vec![Diagnostic::new(
            Rule::UnsupportedSemantics,
            "graph contains bad edges but the core reported safety",
        )]);
    }

    Analysis {
        outcome,
        receipt: Some(compiled.receipt.clone()),
        report: Some(report),
    }
}

fn invalid(diagnostics: Vec<Diagnostic>) -> Analysis {
    Analysis {
        outcome: AnalysisOutcome::InvalidInput(diagnostics),
        receipt: None,
        report: None,
    }
}

fn inconclusive(diagnostics: Vec<Diagnostic>) -> Analysis {
    Analysis {
        outcome: AnalysisOutcome::Inconclusive(diagnostics),
        receipt: None,
        report: None,
    }
}
