//! Independent reference interpreter for the experimental artifact domain.
//!
//! This module deliberately shares nothing with `cegar_domain::compile`: it does
//! not build a `Model`, does not keep the compiler's canonical state encoding,
//! and re-implements every precondition and outcome rule directly from the raw
//! plan and pinned context. It exists to falsify the compiler, so it must be
//! able to disagree with it.
//!
//! The search is a depth-first enumeration of schedules (one attempt per node,
//! at most `nodes!` orders), stopping each branch at its first violation. That
//! is a different algorithm from the compiler's canonical-state BFS.

use std::collections::BTreeSet;

use cegar_domain::compile::ActionKey;
use cegar_domain::context::{Context, EvidenceSharing, TestStatus};
use cegar_domain::plan::{Disposition, EvidenceRef, InputRef, NodeId, NodeKind, Plan};
use cegar_domain::types::{
    ArtifactId, AudienceId, DestinationId, OperationRef, PrincipalId, RunId, SnapshotRef, SuiteRef,
    TenantId,
};
use cegar_domain::{Analysis, AnalysisOutcome, Rule};

/// Bound on explored schedule steps so malformed input cannot hang the check.
const MAX_VISITS: usize = 2_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Todo,
    Ok,
    Fail,
    Blocked,
}

impl Slot {
    fn tag(self) -> char {
        match self {
            Self::Todo => 'T',
            Self::Ok => 'O',
            Self::Fail => 'F',
            Self::Blocked => 'B',
        }
    }
}

#[derive(Clone, Debug)]
enum Op {
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
        subject: Subject,
        suite: SuiteRef,
        runner: PrincipalId,
    },
    Publish {
        operation: OperationRef,
        actor: PrincipalId,
        subject: Subject,
        evidence: Evidence,
        destination: DestinationId,
    },
}

#[derive(Clone, Copy, Debug)]
enum Subject {
    From(usize),
    Fixed(ArtifactId),
}

#[derive(Clone, Debug)]
enum Evidence {
    From(usize),
    Observed(RunId),
}

#[derive(Clone, Debug)]
struct NodeRef {
    id: NodeId,
    deps: Vec<usize>,
    op: Op,
}

/// One modeled pass record produced on a test node's success branch.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PassRecord {
    node: u32,
    subject: ArtifactId,
    suite: SuiteRef,
    runner: PrincipalId,
}

#[derive(Clone, Debug)]
struct Config {
    slots: Vec<Slot>,
    bound: Vec<Option<ArtifactId>>,
    staged: Vec<ArtifactId>,
    passes: Vec<PassRecord>,
    published: Vec<ArtifactId>,
}

impl Config {
    fn key(&self) -> String {
        let slots: String = self.slots.iter().map(|slot| slot.tag()).collect();
        let bound: Vec<String> = self
            .bound
            .iter()
            .map(|entry| match entry {
                Some(artifact) => artifact.to_string(),
                None => "-".to_owned(),
            })
            .collect();
        let staged: Vec<String> = self.staged.iter().map(|item| item.to_string()).collect();
        let passes: Vec<String> = self
            .passes
            .iter()
            .map(|pass| {
                format!(
                    "{}|{}|{}|{}",
                    pass.node, pass.subject, pass.suite.name, pass.runner
                )
            })
            .collect();
        let published: Vec<String> = self.published.iter().map(|item| item.to_string()).collect();
        format!(
            "{slots}#{bound}#[{staged}]#[{passes}]#[{published}]",
            bound = bound.join(","),
            staged = staged.join(","),
            passes = passes.join(";"),
            published = published.join(",")
        )
    }
}

/// One permitted branch out of a node attempt.
struct Step {
    outcome: &'static str,
    binding: Option<ArtifactId>,
    next: Config,
}

/// Result of the independent exhaustive reference search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleReport {
    /// Rule codes of every violation reachable as a first violation.
    pub violations: BTreeSet<&'static str>,
    /// Distinct reachable configurations that were not first violations.
    pub live_configurations: usize,
    /// Branches that ended with every node decided and no violation.
    pub safe_terminals: usize,
    /// False when a bound or an unexplained stall prevented completeness.
    pub exhaustive: bool,
    pub note: Option<String>,
}

struct Oracle<'a> {
    nodes: Vec<NodeRef>,
    context: &'a Context,
    disposition: Disposition,
    visits: usize,
    violated: BTreeSet<&'static str>,
    seen: BTreeSet<String>,
    safe_terminals: usize,
    truncated: Option<String>,
}

/// Run the independent reference search for one raw plan and context.
///
/// Returns `None` when a reference cannot be resolved at all (dangling
/// identifiers), which the compiler reports as `invalid_input`.
pub fn reference_search(plan: &Plan, context: &Context) -> Option<OracleReport> {
    let nodes = resolve(plan, context)?;
    let mut oracle = Oracle {
        nodes,
        context,
        disposition: plan.disposition,
        visits: 0,
        violated: BTreeSet::new(),
        seen: BTreeSet::new(),
        safe_terminals: 0,
        truncated: None,
    };
    oracle.walk(oracle.initial(), 0);
    Some(OracleReport {
        violations: oracle.violated,
        live_configurations: oracle.seen.len(),
        safe_terminals: oracle.safe_terminals,
        exhaustive: oracle.truncated.is_none(),
        note: oracle.truncated,
    })
}

fn resolve(plan: &Plan, context: &Context) -> Option<Vec<NodeRef>> {
    let mut ordered: Vec<&cegar_domain::plan::Node> = plan.nodes.iter().collect();
    ordered.sort_by_key(|node| node.id);
    let index_of = |id: NodeId| ordered.iter().position(|node| node.id == id);

    let mut nodes = Vec::with_capacity(ordered.len());
    for node in &ordered {
        let mut deps = Vec::new();
        for dep in &node.deps {
            deps.push(index_of(*dep)?);
        }
        deps.sort_unstable();
        deps.dedup();
        let op = match &node.kind {
            NodeKind::Stage(spec) => Op::Stage {
                operation: spec.operation.clone(),
                actor: spec.actor.clone(),
                base: spec.base,
                patch: spec.patch,
                outputs: context.stage_candidates(node.id)?.to_vec(),
            },
            NodeKind::Test(spec) => Op::Test {
                operation: spec.operation.clone(),
                actor: spec.actor.clone(),
                subject: match &spec.subject {
                    InputRef::Produced(id) => Subject::From(index_of(*id)?),
                    InputRef::Initial(artifact) => Subject::Fixed(*artifact),
                },
                suite: spec.suite.clone(),
                runner: spec.runner.clone(),
            },
            NodeKind::Publish(spec) => Op::Publish {
                operation: spec.operation.clone(),
                actor: spec.actor.clone(),
                subject: match &spec.subject {
                    InputRef::Produced(id) => Subject::From(index_of(*id)?),
                    InputRef::Initial(artifact) => Subject::Fixed(*artifact),
                },
                evidence: match &spec.evidence {
                    EvidenceRef::Produced(id) => Evidence::From(index_of(*id)?),
                    EvidenceRef::Observed(run) => Evidence::Observed(run.clone()),
                },
                destination: spec.destination.clone(),
            },
        };
        nodes.push(NodeRef {
            id: node.id,
            deps,
            op,
        });
    }
    Some(nodes)
}

#[derive(Clone, Debug)]
struct Record {
    status: TestStatus,
    subject: ArtifactId,
    suite: SuiteRef,
    runner: PrincipalId,
    issuer: PrincipalId,
    evidence_principal: PrincipalId,
    tenant: TenantId,
    audience: AudienceId,
    issued_at: u64,
    validity: u64,
    snapshot: SnapshotRef,
}

impl Oracle<'_> {
    fn initial(&self) -> Config {
        let mut staged = self.context.staged.clone();
        staged.sort_unstable();
        staged.dedup();
        Config {
            slots: vec![Slot::Todo; self.nodes.len()],
            bound: vec![None; self.nodes.len()],
            staged,
            passes: Vec::new(),
            published: Vec::new(),
        }
    }

    fn walk(&mut self, config: Config, depth: usize) {
        if self.truncated.is_some() {
            return;
        }
        self.visits += 1;
        if self.visits > MAX_VISITS {
            self.truncated = Some(format!("reference search exceeded {MAX_VISITS} visits"));
            return;
        }
        let mut config = config;
        self.close(&mut config);
        self.seen.insert(config.key());

        let ready = self.ready(&config);
        if ready.is_empty() {
            if config.slots.iter().all(|slot| *slot != Slot::Todo) {
                self.safe_terminals += 1;
            } else {
                self.truncated = Some(format!(
                    "reference search found an unexplained stall at depth {depth}"
                ));
            }
            return;
        }
        for index in ready {
            let violated = self.attempt_rules(&config, index);
            if !violated.is_empty() {
                for rule in violated {
                    self.violated.insert(rule.code());
                }
                continue;
            }
            for step in self.successors(&config, index) {
                self.walk(step.next, depth + 1);
            }
        }
    }

    fn close(&self, config: &mut Config) {
        let abort_all = matches!(self.disposition, Disposition::AbortAll);
        loop {
            let mut changed = false;
            let any_failure = config.slots.contains(&Slot::Fail);
            for index in 0..config.slots.len() {
                if config.slots[index] != Slot::Todo {
                    continue;
                }
                let dependency_failed = self.nodes[index]
                    .deps
                    .iter()
                    .any(|dep| matches!(config.slots[*dep], Slot::Fail | Slot::Blocked));
                if dependency_failed || (abort_all && any_failure) {
                    config.slots[index] = Slot::Blocked;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn ready(&self, config: &Config) -> Vec<usize> {
        let mut ready = Vec::new();
        for (index, slot) in config.slots.iter().enumerate() {
            if *slot != Slot::Todo {
                continue;
            }
            if self.nodes[index]
                .deps
                .iter()
                .all(|dep| config.slots[*dep] == Slot::Ok)
            {
                ready.push(index);
            }
        }
        ready
    }

    fn staged_subject(&self, config: &Config, subject: Subject) -> Option<ArtifactId> {
        let artifact = match subject {
            Subject::From(index) => config.bound[index]?,
            Subject::Fixed(artifact) => artifact,
        };
        config.staged.contains(&artifact).then_some(artifact)
    }

    fn permission_rules(&self, operation: &OperationRef, actor: &PrincipalId) -> Vec<Rule> {
        let mut rules = Vec::new();
        if !self
            .context
            .policy
            .authorized_operations
            .contains(operation)
        {
            rules.push(Rule::OperationUnauthorized);
        }
        if !self.context.policy.authorized_actors.contains(actor) {
            rules.push(Rule::ActorUnauthorized);
        }
        rules
    }

    /// Every modeled precondition violated by this ready attempt.
    fn attempt_rules(&self, config: &Config, index: usize) -> Vec<Rule> {
        let mut rules = match &self.nodes[index].op {
            Op::Stage {
                operation,
                actor,
                base,
                patch,
                ..
            } => {
                let mut rules = self.permission_rules(operation, actor);
                for artifact in [base, patch] {
                    if !self.context.artifacts.contains(artifact) {
                        rules.push(Rule::InputMissing);
                    }
                }
                rules
            }
            Op::Test {
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
                if self.staged_subject(config, *subject).is_none() {
                    rules.push(Rule::SubjectNotStaged);
                }
                rules
            }
            Op::Publish {
                operation,
                actor,
                subject,
                evidence,
                destination,
            } => {
                let mut rules = self.permission_rules(operation, actor);
                let candidate = self.staged_subject(config, *subject);
                if candidate.is_none() {
                    rules.push(Rule::SubjectNotStaged);
                }
                // The scoped predicate compares the attested subject with the
                // exact publish subject, so it is undefined without a staged
                // candidate. Presence of the designated record is still
                // checkable, and its absence is its own violated rule.
                match self.attested_record(config, evidence) {
                    None => rules.push(Rule::EvidenceMissing),
                    Some(record) => {
                        if let Some(candidate) = candidate {
                            rules.extend(self.scope_violations(&record, &candidate, destination));
                        }
                    }
                }
                rules
            }
        };
        Rule::sort_codes(&mut rules);
        rules
    }

    fn attested_record(&self, config: &Config, evidence: &Evidence) -> Option<Record> {
        match evidence {
            Evidence::From(producer) => {
                let producer_id = self.nodes[*producer].id.0;
                let pass = config
                    .passes
                    .iter()
                    .find(|record| record.node == producer_id)?;
                let profile = &self.context.hypothetical_pass;
                Some(Record {
                    status: TestStatus::Pass,
                    subject: pass.subject,
                    suite: pass.suite.clone(),
                    runner: pass.runner.clone(),
                    issuer: profile.issuer.clone(),
                    evidence_principal: profile.evidence_principal.clone(),
                    tenant: profile.tenant.clone(),
                    audience: profile.audience.clone(),
                    issued_at: self.context.now,
                    validity: profile.validity,
                    snapshot: self.context.snapshot.clone(),
                })
            }
            Evidence::Observed(run) => {
                let observation = self.context.observation(run)?;
                Some(Record {
                    status: observation.status,
                    subject: observation.subject,
                    suite: observation.suite.clone(),
                    runner: observation.runner.clone(),
                    issuer: observation.issuer.clone(),
                    evidence_principal: observation.evidence_principal.clone(),
                    tenant: observation.tenant.clone(),
                    audience: observation.audience.clone(),
                    issued_at: observation.issued_at,
                    validity: observation.validity,
                    snapshot: self.context.snapshot.clone(),
                })
            }
        }
    }

    fn scope_violations(
        &self,
        record: &Record,
        candidate: &ArtifactId,
        destination: &DestinationId,
    ) -> Vec<Rule> {
        let policy = &self.context.policy;
        let mut rules = Vec::new();
        if record.status != TestStatus::Pass {
            rules.push(Rule::StatusNotPass);
        }
        if record.subject != *candidate {
            rules.push(Rule::SubjectMismatch);
        }
        if record.suite != policy.required_suite {
            rules.push(Rule::SuiteNotRequired);
        }
        if !policy.runner_authorized(&record.runner) {
            rules.push(Rule::RunnerUnauthorized);
        }
        if !policy.issuer_trusted(&record.issuer) {
            rules.push(Rule::IssuerUntrusted);
        }
        let same_principal = record.evidence_principal == policy.verification_principal;
        let shared = match &policy.evidence_sharing {
            EvidenceSharing::SamePrincipalOnly => false,
            EvidenceSharing::PolicyPermitted(list) => list.contains(&record.evidence_principal),
        };
        if !same_principal && !shared {
            rules.push(Rule::PrincipalNotShared);
        }
        if record.tenant != policy.tenant {
            rules.push(Rule::TenantMismatch);
        }
        if record.audience != policy.audience {
            rules.push(Rule::AudienceMismatch);
        }
        if !policy.destination_authorized(destination) {
            rules.push(Rule::DestinationUnauthorized);
        }
        let now = self.context.now;
        if record.issued_at > now || now > record.issued_at.saturating_add(record.validity) {
            rules.push(Rule::EvidenceExpired);
        }
        if record.snapshot != self.context.snapshot {
            rules.push(Rule::SnapshotMismatch);
        }
        Rule::sort_codes(&mut rules);
        rules
    }

    fn successors(&self, config: &Config, index: usize) -> Vec<Step> {
        let mut steps = Vec::new();
        match &self.nodes[index].op {
            Op::Stage { outputs, .. } => {
                for candidate in outputs {
                    let mut next = config.clone();
                    next.slots[index] = Slot::Ok;
                    next.bound[index] = Some(*candidate);
                    next.staged.push(*candidate);
                    next.staged.sort_unstable();
                    next.staged.dedup();
                    steps.push(Step {
                        outcome: "success",
                        binding: Some(*candidate),
                        next,
                    });
                }
                let mut failed = config.clone();
                failed.slots[index] = Slot::Fail;
                steps.push(Step {
                    outcome: "failure_no_effect",
                    binding: None,
                    next: failed,
                });
            }
            Op::Test {
                subject,
                suite,
                runner,
                ..
            } => {
                if let Some(artifact) = self.staged_subject(config, *subject) {
                    let mut passed = config.clone();
                    passed.slots[index] = Slot::Ok;
                    passed.passes.push(PassRecord {
                        node: self.nodes[index].id.0,
                        subject: artifact,
                        suite: suite.clone(),
                        runner: runner.clone(),
                    });
                    passed.passes.sort();
                    steps.push(Step {
                        outcome: "pass",
                        binding: None,
                        next: passed,
                    });
                }
                for outcome in ["fail", "error"] {
                    let mut failed = config.clone();
                    failed.slots[index] = Slot::Fail;
                    steps.push(Step {
                        outcome,
                        binding: None,
                        next: failed,
                    });
                }
            }
            Op::Publish { subject, .. } => {
                if let Some(candidate) = self.staged_subject(config, *subject) {
                    let mut published = config.clone();
                    published.slots[index] = Slot::Ok;
                    published.published.push(candidate);
                    published.published.sort_unstable();
                    published.published.dedup();
                    steps.push(Step {
                        outcome: "success",
                        binding: Some(candidate),
                        next: published,
                    });
                }
                let mut failed = config.clone();
                failed.slots[index] = Slot::Fail;
                steps.push(Step {
                    outcome: "failure_no_effect",
                    binding: None,
                    next: failed,
                });
            }
        }
        steps
    }
}

/// Replay one decoded compiler trace against the raw domain, independently.
///
/// Every non-terminal attempt must satisfy its preconditions and name a
/// permitted outcome; the final attempt must be the modeled first violation and
/// must carry exactly the rules the reference computes.
pub fn replay_trace(plan: &Plan, context: &Context, trace: &[ActionKey]) -> Result<(), String> {
    let Some(nodes) = resolve(plan, context) else {
        return Err("trace replay could not resolve the plan".to_owned());
    };
    let oracle = Oracle {
        nodes,
        context,
        disposition: plan.disposition,
        visits: 0,
        violated: BTreeSet::new(),
        seen: BTreeSet::new(),
        safe_terminals: 0,
        truncated: None,
    };
    let mut config = oracle.initial();
    oracle.close(&mut config);

    if trace.is_empty() {
        return Err("trace is empty".to_owned());
    }
    for (step, action) in trace.iter().enumerate() {
        let Some(index) = oracle.nodes.iter().position(|node| node.id == action.node) else {
            return Err(format!("step {step}: unknown node {}", action.node.0));
        };
        if config.slots[index] != Slot::Todo {
            return Err(format!(
                "step {step}: node {} was already attempted",
                action.node.0
            ));
        }
        if !oracle.nodes[index]
            .deps
            .iter()
            .all(|dep| config.slots[*dep] == Slot::Ok)
        {
            return Err(format!("step {step}: node {} was not ready", action.node.0));
        }
        let rules = oracle.attempt_rules(&config, index);
        if action.outcome == "unsafe_attempt" {
            if step + 1 != trace.len() {
                return Err(format!("step {step}: violation is not the final attempt"));
            }
            if rules.is_empty() {
                return Err(format!(
                    "step {step}: reference sees no violation for node {}",
                    action.node.0
                ));
            }
            let recorded: Vec<&str> = action.rules.iter().map(|rule| rule.code()).collect();
            let expected: Vec<&str> = rules.iter().map(|rule| rule.code()).collect();
            if recorded != expected {
                return Err(format!(
                    "step {step}: recorded rules {recorded:?} differ from reference {expected:?}"
                ));
            }
            return Ok(());
        }
        if !rules.is_empty() {
            return Err(format!(
                "step {step}: node {} violates {rules:?} but the trace continues",
                action.node.0
            ));
        }
        let next = oracle
            .successors(&config, index)
            .into_iter()
            .find(|candidate| {
                candidate.outcome == action.outcome && candidate.binding == action.binding
            });
        match next {
            Some(step) => {
                config = step.next;
                oracle.close(&mut config);
            }
            None => {
                return Err(format!(
                    "step {step}: no permitted successor matches {}",
                    action.describe()
                ));
            }
        }
    }
    Err("trace ended without a modeled first violation".to_owned())
}

/// Compare one compiler analysis against the independent reference.
///
/// Only cases the compiler either verified or refuted are comparable; the
/// classification gates (`invalid_input`, `stale_snapshot`, `needs_evidence`)
/// and bound-exhausted runs are checked by their own tests.
pub fn compare(plan: &Plan, context: &Context, analysis: &Analysis) -> Result<(), String> {
    let Some(report) = reference_search(plan, context) else {
        return Err("reference could not resolve the plan".to_owned());
    };
    match &analysis.outcome {
        AnalysisOutcome::ConcreteCounterexample { trace, rules, .. } => {
            if report.violations.is_empty() {
                return Err("compiler found a violation the reference cannot reach".to_owned());
            }
            let recorded: Vec<&str> = rules.iter().map(|rule| rule.code()).collect();
            if !recorded.iter().all(|code| report.violations.contains(code)) {
                return Err(format!(
                    "recorded rules {recorded:?} are not all reachable in the reference"
                ));
            }
            replay_trace(plan, context, trace)
        }
        AnalysisOutcome::VerifiedWithinModel => {
            if !report.exhaustive {
                return Err(format!(
                    "compiler verified but the reference was incomplete: {:?}",
                    report.note
                ));
            }
            if !report.violations.is_empty() {
                return Err(format!(
                    "compiler verified but the reference reaches {:?}",
                    report.violations
                ));
            }
            let receipt = analysis
                .receipt
                .as_ref()
                .ok_or_else(|| "verified analysis without a receipt".to_owned())?;
            let live_states = receipt.state_count.saturating_sub(1);
            if live_states != report.live_configurations {
                return Err(format!(
                    "state counts differ: compiler {live_states}, reference {}",
                    report.live_configurations
                ));
            }
            Ok(())
        }
        other => Err(format!(
            "case is not comparable: compiler classified it as {}",
            other.tag()
        )),
    }
}
