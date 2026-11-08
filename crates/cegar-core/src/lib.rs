//! Finite-state, dependency-free CEGAR. No filesystem, clock, network, or GPU I/O.
//!
//! Results apply only to a complete, caller-supplied finite transition system.
//! They do not establish model fidelity, real-world policy, or execution permission.
//! States are dense indices `0..state_count`; action IDs have no external authority.
//! Every quotient edge comes from a concrete edge. A spurious abstract trace splits
//! one block along its concretely reachable frontier, strictly increasing precision.
//!
//! ```
//! use cegar_core::{Limits, Model, Outcome, Transition, verify};
//! let model = Model::new(3, &[0], &[2], &[Transition::new(0, 7, 1)]).unwrap();
//! let report = verify(&model, Limits::default());
//! assert!(matches!(report.outcome, Outcome::VerifiedWithinModel));
//! assert!(!report.execution_authorized());
//! ```
#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::fmt;

pub type State = usize;
pub type Action = u32;
pub const MAX_STATES: usize = 128;
pub const MAX_TRANSITIONS: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transition {
    pub source: State,
    pub action: Action,
    pub target: State,
}

impl Transition {
    pub const fn new(source: State, action: Action, target: State) -> Self {
        Self {
            source,
            action,
            target,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputError {
    StateCount,
    TransitionCount,
    EmptyInitial,
    SetTooLarge(&'static str),
    UnknownState { field: &'static str, state: State },
    DuplicateState { field: &'static str, state: State },
}

impl fmt::Display for InputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid finite model: {self:?}")
    }
}
impl std::error::Error for InputError {}

/// Validated immutable input. Inconsistent/empty initial sets cannot prove safety.
/// Parallel and duplicate edges are legal; action labels are part of trace identity.
#[derive(Clone, Debug)]
pub struct Model {
    state_count: usize,
    initial: Vec<State>,
    bad: Vec<bool>,
    transitions: Vec<Transition>,
}

impl Model {
    /// Validates cardinality and indices before allocating internal state arrays.
    /// Callers must separately bound their transport/parser input allocations.
    pub fn new(
        state_count: usize,
        initial: &[State],
        bad: &[State],
        transitions: &[Transition],
    ) -> Result<Self, InputError> {
        if state_count == 0 || state_count > MAX_STATES {
            return Err(InputError::StateCount);
        }
        if transitions.len() > MAX_TRANSITIONS {
            return Err(InputError::TransitionCount);
        }
        if initial.is_empty() {
            return Err(InputError::EmptyInitial);
        }
        fn check_set(
            n: usize,
            values: &[State],
            field: &'static str,
        ) -> Result<Vec<bool>, InputError> {
            if values.len() > n {
                return Err(InputError::SetTooLarge(field));
            }
            let mut seen = vec![false; n];
            for &state in values {
                let slot = seen
                    .get_mut(state)
                    .ok_or(InputError::UnknownState { field, state })?;
                if *slot {
                    return Err(InputError::DuplicateState { field, state });
                }
                *slot = true;
            }
            Ok(seen)
        }
        check_set(state_count, initial, "initial")?;
        let bad = check_set(state_count, bad, "bad")?;
        for edge in transitions {
            for (field, state) in [("source", edge.source), ("target", edge.target)] {
                if state >= state_count {
                    return Err(InputError::UnknownState { field, state });
                }
            }
        }
        Ok(Self {
            state_count,
            initial: initial.to_vec(),
            bad,
            transitions: transitions.to_vec(),
        })
    }

    pub fn state_count(&self) -> usize {
        self.state_count
    }
    pub fn initial(&self) -> &[State] {
        &self.initial
    }
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }
    pub fn bad_states(&self) -> impl Iterator<Item = State> + '_ {
        self.bad
            .iter()
            .enumerate()
            .filter_map(|(state, &bad)| bad.then_some(state))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Deterministic algorithmic visits, not CPU cycles or a wall-clock deadline.
    pub max_work: u64,
    /// Every successful refinement adds one nonempty block; at most n - 1 exist.
    pub max_refinements: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_work: 2_000_000,
            max_refinements: MAX_STATES - 1,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub work: u64,
    pub refinements: usize,
    pub abstract_checks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    WorkLimit,
    RefinementLimit,
    Cancelled,
    InternalInvariant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {
    pub states: Vec<State>,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WitnessError {
    Shape,
    TooLong,
    UnknownState,
    NotInitial,
    NotBad,
    MissingTransition(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    VerifiedWithinModel,
    ConcreteCounterexample(Witness),
    Inconclusive(StopReason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub outcome: Outcome,
    pub stats: Stats,
}
impl Report {
    pub const fn scope(&self) -> &'static str {
        "supplied_finite_model_only"
    }
    pub const fn execution_authorized(&self) -> bool {
        false
    }
}

struct Budget<F> {
    limits: Limits,
    stats: Stats,
    cancel: F,
}
impl<F: FnMut() -> bool> Budget<F> {
    fn tick(&mut self) -> Result<(), StopReason> {
        if (self.cancel)() {
            return Err(StopReason::Cancelled);
        }
        if self.stats.work >= self.limits.max_work {
            return Err(StopReason::WorkLimit);
        }
        self.stats.work += 1;
        Ok(())
    }
}

struct Partition {
    block_of: Vec<usize>,
    count: usize,
}
impl Partition {
    fn coarse(n: usize) -> Self {
        Self {
            block_of: vec![0; n],
            count: 1,
        }
    }

    /// Split only a nonempty proper subset of a single block. No concrete state
    /// or transition is removed. Partial progress on cancellation is discarded.
    fn split<F: FnMut() -> bool>(
        &mut self,
        block: usize,
        subset: &[bool],
        budget: &mut Budget<F>,
    ) -> Result<(), StopReason> {
        if subset.len() != self.block_of.len() {
            return Err(StopReason::InternalInvariant);
        }
        let mut selected = 0;
        let mut other = 0;
        for (&assigned, &inside) in self.block_of.iter().zip(subset) {
            budget.tick()?;
            if inside && assigned != block {
                return Err(StopReason::InternalInvariant);
            }
            if assigned == block {
                if inside {
                    selected += 1;
                } else {
                    other += 1;
                }
            }
        }
        if selected == 0 || other == 0 {
            return Err(StopReason::InternalInvariant);
        }
        for (assigned, &inside) in self.block_of.iter_mut().zip(subset) {
            budget.tick()?;
            if inside {
                *assigned = self.count;
            }
        }
        self.count += 1;
        Ok(())
    }
}

struct AbstractTrace {
    blocks: Vec<usize>,
    actions: Vec<Action>,
}

/// BFS of the existential quotient. Abstract bad/initial sets use existential
/// membership, preserving every concrete bad trace (possibly adding spurious ones).
fn abstract_trace<F: FnMut() -> bool>(
    model: &Model,
    partition: &Partition,
    budget: &mut Budget<F>,
) -> Result<Option<AbstractTrace>, StopReason> {
    let mut bad = vec![false; partition.count];
    for (state, &is_bad) in model.bad.iter().enumerate() {
        budget.tick()?;
        if is_bad {
            bad[partition.block_of[state]] = true;
        }
    }
    let mut queue = VecDeque::new();
    let mut seen = vec![false; partition.count];
    let mut parent = vec![None; partition.count];
    for &state in &model.initial {
        budget.tick()?;
        let block = partition.block_of[state];
        if !seen[block] {
            seen[block] = true;
            queue.push_back(block);
        }
    }
    while let Some(block) = queue.pop_front() {
        budget.tick()?;
        if bad[block] {
            let mut blocks = vec![block];
            let mut actions = Vec::new();
            let mut here = block;
            while let Some((previous, action)) = parent[here] {
                budget.tick()?;
                blocks.push(previous);
                actions.push(action);
                here = previous;
            }
            blocks.reverse();
            actions.reverse();
            return Ok(Some(AbstractTrace { blocks, actions }));
        }
        for edge in &model.transitions {
            budget.tick()?;
            let target = partition.block_of[edge.target];
            if partition.block_of[edge.source] == block && !seen[target] {
                seen[target] = true;
                parent[target] = Some((block, edge.action));
                queue.push_back(target);
            }
        }
    }
    Ok(None)
}

enum Feasibility {
    Real(Witness),
    Spurious { block: usize, frontier: Vec<bool> },
}

fn concretize<F: FnMut() -> bool>(
    model: &Model,
    partition: &Partition,
    trace: &AbstractTrace,
    budget: &mut Budget<F>,
) -> Result<Feasibility, StopReason> {
    let mut current = vec![false; model.state_count];
    for &state in &model.initial {
        budget.tick()?;
        current[state] = partition.block_of[state] == trace.blocks[0];
    }
    let mut parents: Vec<Vec<Option<State>>> = Vec::new();
    for (step, &action) in trace.actions.iter().enumerate() {
        let mut next = vec![false; model.state_count];
        let mut predecessors = vec![None; model.state_count];
        for edge in &model.transitions {
            budget.tick()?;
            if current[edge.source]
                && edge.action == action
                && partition.block_of[edge.target] == trace.blocks[step + 1]
                && !next[edge.target]
            {
                next[edge.target] = true;
                predecessors[edge.target] = Some(edge.source);
            }
        }
        if !next.iter().any(|&reachable| reachable) {
            return Ok(Feasibility::Spurious {
                block: trace.blocks[step],
                frontier: current,
            });
        }
        parents.push(predecessors);
        current = next;
    }
    for (state, &reachable) in current.iter().enumerate() {
        budget.tick()?;
        if reachable && model.bad[state] {
            let mut states = vec![state];
            let mut here = state;
            for row in parents.iter().rev() {
                budget.tick()?;
                here = row[here].ok_or(StopReason::InternalInvariant)?;
                states.push(here);
            }
            states.reverse();
            return Ok(Feasibility::Real(Witness {
                states,
                actions: trace.actions.clone(),
            }));
        }
    }
    Ok(Feasibility::Spurious {
        block: *trace.blocks.last().ok_or(StopReason::InternalInvariant)?,
        frontier: current,
    })
}

fn replay_inner(
    model: &Model,
    witness: &Witness,
    mut tick: impl FnMut() -> Result<(), StopReason>,
) -> Result<Result<(), WitnessError>, StopReason> {
    tick()?;
    if witness.states.is_empty() || witness.actions.len() != witness.states.len() - 1 {
        return Ok(Err(WitnessError::Shape));
    }
    if witness.states.len() > MAX_STATES {
        return Ok(Err(WitnessError::TooLong));
    }
    for &state in &witness.states {
        tick()?;
        if state >= model.state_count {
            return Ok(Err(WitnessError::UnknownState));
        }
    }
    if !model.initial.contains(&witness.states[0]) {
        return Ok(Err(WitnessError::NotInitial));
    }
    if !model.bad[*witness.states.last().ok_or(StopReason::InternalInvariant)?] {
        return Ok(Err(WitnessError::NotBad));
    }
    for (step, pair) in witness.states.windows(2).enumerate() {
        let expected = Transition::new(pair[0], witness.actions[step], pair[1]);
        let mut found = false;
        for edge in &model.transitions {
            tick()?;
            if *edge == expected {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(Err(WitnessError::MissingTransition(step)));
        }
    }
    Ok(Ok(()))
}

/// Independently replay a bounded witness, checking action labels as well as states.
pub fn replay(model: &Model, witness: &Witness) -> Result<(), WitnessError> {
    replay_inner(model, witness, || Ok(())).unwrap_or(Err(WitnessError::Shape))
}

fn run<F: FnMut() -> bool>(model: &Model, budget: &mut Budget<F>) -> Result<Outcome, StopReason> {
    let mut partition = Partition::coarse(model.state_count);
    loop {
        budget.tick()?;
        budget.stats.abstract_checks += 1;
        let Some(trace) = abstract_trace(model, &partition, budget)? else {
            return Ok(Outcome::VerifiedWithinModel);
        };
        match concretize(model, &partition, &trace, budget)? {
            Feasibility::Real(witness) => {
                if replay_inner(model, &witness, || budget.tick())?.is_err() {
                    return Err(StopReason::InternalInvariant);
                }
                return Ok(Outcome::ConcreteCounterexample(witness));
            }
            Feasibility::Spurious { block, frontier } => {
                if budget.stats.refinements >= budget.limits.max_refinements {
                    return Err(StopReason::RefinementLimit);
                }
                partition.split(block, &frontier, budget)?;
                budget.stats.refinements += 1;
            }
        }
    }
}

#[must_use]
pub fn verify(model: &Model, limits: Limits) -> Report {
    verify_with_cancel(model, limits, || false)
}

/// Explicit cooperative cancellation hook. The caller may check a deadline here;
/// this crate reads no clock. A blocking callback cannot be preempted by the core.
/// Process memory ceilings and hard wall-time limits belong in a worker adapter.
#[must_use]
pub fn verify_with_cancel(model: &Model, limits: Limits, cancel: impl FnMut() -> bool) -> Report {
    let mut budget = Budget {
        limits,
        stats: Stats::default(),
        cancel,
    };
    let outcome = run(model, &mut budget).unwrap_or_else(Outcome::Inconclusive);
    Report {
        outcome,
        stats: budget.stats,
    }
}

#[cfg(test)]
mod abstraction_tests {
    use super::*;

    fn budget() -> Budget<impl FnMut() -> bool> {
        Budget {
            limits: Limits::default(),
            stats: Stats::default(),
            cancel: || false,
        }
    }

    #[test]
    fn every_partition_projects_initial_bad_and_labeled_edges() {
        // All assignments of four states to four possible blocks, retaining only
        // contiguous nonempty block IDs. Cross-block edges are tested in isolation
        // so no alternate edge can mask a dropped projection or wrong action.
        // A same-block edge is unobservable in a simple BFS trace: the initial
        // block is already bad, so that case checks zero-step reachability only.
        let n: usize = 4;
        for assignment in 0usize..n.pow(n as u32) {
            let mut value = assignment;
            let mut block_of = Vec::new();
            for _ in 0..n {
                block_of.push(value % n);
                value /= n;
            }
            let count = 1 + *block_of.iter().max().unwrap();
            if (0..count).any(|b| !block_of.contains(&b)) {
                continue;
            }
            let partition = Partition { block_of, count };
            for source in 0..n {
                for target in 0..n {
                    for action in [0, u32::MAX] {
                        let edge = Transition::new(source, action, target);
                        let model = Model::new(n, &[source], &[target], &[edge]).unwrap();
                        let trace = abstract_trace(&model, &partition, &mut budget())
                            .unwrap_or_else(|reason| panic!("abstract search stopped ({reason:?}): partition={:?}, initial=[{source}], bad=[{target}], edge=({source},{action},{target})", partition.block_of))
                            .unwrap_or_else(|| panic!("missing projected trace: partition={:?}, initial=[{source}], bad=[{target}], edge=({source},{action},{target})", partition.block_of));
                        let fixture = || {
                            format!(
                                "partition={:?}, initial=[{source}], bad=[{target}], edge=({source},{action},{target})",
                                partition.block_of
                            )
                        };
                        if partition.block_of[source] == partition.block_of[target] {
                            assert_eq!(trace.blocks, [partition.block_of[source]], "{}", fixture());
                            assert!(trace.actions.is_empty(), "{}", fixture());
                        } else {
                            assert_eq!(
                                trace.blocks,
                                [partition.block_of[source], partition.block_of[target]],
                                "{}",
                                fixture()
                            );
                            assert_eq!(trace.actions, [action], "{}", fixture());
                        }
                    }
                }
            }
            // With no edges, all and only initial/bad block overlaps are reachable.
            for initial_mask in 1..(1 << n) {
                let initial: Vec<_> = (0..n).filter(|s| initial_mask & (1 << s) != 0).collect();
                for bad_mask in 0..(1 << n) {
                    let bad: Vec<_> = (0..n).filter(|s| bad_mask & (1 << s) != 0).collect();
                    let model = Model::new(n, &initial, &bad, &[]).unwrap();
                    let trace = abstract_trace(&model, &partition, &mut budget())
                        .unwrap_or_else(|reason| panic!("abstract search stopped ({reason:?}): partition={:?}, initial={initial:?}, bad={bad:?}, edges=[]", partition.block_of));
                    let overlap = initial.iter().any(|&i| {
                        bad.iter()
                            .any(|&b| partition.block_of[i] == partition.block_of[b])
                    });
                    assert_eq!(
                        trace.is_some(),
                        overlap,
                        "partition={:?} initial={initial:?} bad={bad:?}",
                        partition.block_of
                    );
                }
            }
        }
    }

    // Mirror the run prefix for a zero-step real counterexample. Using the
    // measured prefix rather than the completed run's total ensures these
    // tests fail if replay is accidentally removed from budget accounting.
    fn work_before_zero_step_replay(model: &Model) -> u64 {
        let mut prefix = budget();
        prefix.tick().unwrap(); // run's loop tick
        let partition = Partition::coarse(1);
        let trace = abstract_trace(model, &partition, &mut prefix)
            .unwrap()
            .unwrap();
        let Feasibility::Real(witness) =
            concretize(model, &partition, &trace, &mut prefix).unwrap()
        else {
            panic!("zero-step initial bad state must be feasible");
        };
        let mut replay_budget = budget();
        assert_eq!(
            replay_inner(model, &witness, || replay_budget.tick()),
            Ok(Ok(()))
        );
        let complete = verify(model, Limits::default());
        assert!(matches!(
            complete.outcome,
            Outcome::ConcreteCounterexample(_)
        ));
        assert_eq!(
            complete.stats.work,
            prefix.stats.work + replay_budget.stats.work,
            "run prefix or replay accounting changed; revisit this boundary test"
        );
        prefix.stats.work
    }

    #[test]
    fn witness_replay_spends_budget() {
        let model = Model::new(1, &[0], &[0], &[]).unwrap();
        let before_replay = work_before_zero_step_replay(&model);
        let report = verify(
            &model,
            Limits {
                max_work: before_replay,
                ..Limits::default()
            },
        );
        assert_eq!(report.outcome, Outcome::Inconclusive(StopReason::WorkLimit));
    }

    #[test]
    fn witness_replay_observes_cancellation() {
        let model = Model::new(1, &[0], &[0], &[]).unwrap();
        let before_replay = work_before_zero_step_replay(&model);
        let mut visits = 0;
        let report = verify_with_cancel(&model, Limits::default(), || {
            let cancel = visits == before_replay;
            visits += 1;
            cancel
        });
        assert_eq!(report.outcome, Outcome::Inconclusive(StopReason::Cancelled));
    }

    #[test]
    fn concretize_keeps_all_nondeterministic_successors() {
        let a = Transition::new(0, 5, 1);
        let b = Transition::new(0, 5, 2);
        let tail = Transition::new(2, 6, 3);
        for edges in [[a, b, tail], [b, a, tail]] {
            let model = Model::new(4, &[0], &[3], &edges).unwrap();
            let partition = Partition {
                block_of: vec![0, 1, 1, 2],
                count: 3,
            };
            let trace = AbstractTrace {
                blocks: vec![0, 1, 2],
                actions: vec![5, 6],
            };
            let Feasibility::Real(witness) =
                concretize(&model, &partition, &trace, &mut budget()).unwrap()
            else {
                panic!("nondeterministic successor 2 reaches bad: {model:?}");
            };
            assert_eq!(witness.states, [0, 2, 3]);
            assert_eq!(witness.actions, [5, 6]);
            assert_eq!(replay(&model, &witness), Ok(()));
        }
    }

    fn replay_scan_fixture() -> (Model, Witness) {
        let model = Model::new(
            2,
            &[0],
            &[1],
            &[Transition::new(0, 8, 0), Transition::new(0, 9, 1)],
        )
        .unwrap();
        let witness = Witness {
            states: vec![0, 1],
            actions: vec![9],
        };
        (model, witness)
    }

    #[test]
    fn replay_transition_scan_spends_budget() {
        let (model, witness) = replay_scan_fixture();
        // Shape tick + two state ticks + first edge tick = 4. The matching
        // second edge requires another tick and must not be returned unchecked.
        let mut b = Budget {
            limits: Limits {
                max_work: 4,
                ..Limits::default()
            },
            stats: Stats::default(),
            cancel: || false,
        };
        assert_eq!(
            replay_inner(&model, &witness, || b.tick()),
            Err(StopReason::WorkLimit)
        );
    }

    #[test]
    fn replay_transition_scan_observes_cancellation() {
        let (model, witness) = replay_scan_fixture();
        let mut visits = 0;
        let mut b = Budget {
            limits: Limits::default(),
            stats: Stats::default(),
            cancel: || {
                let cancel = visits == 4;
                visits += 1;
                cancel
            },
        };
        assert_eq!(
            replay_inner(&model, &witness, || b.tick()),
            Err(StopReason::Cancelled)
        );
    }

    fn work_before_edge_replay(model: &Model) -> u64 {
        let mut prefix = budget();
        let mut partition = Partition::coarse(2);
        prefix.tick().unwrap();
        let trace = abstract_trace(model, &partition, &mut prefix)
            .unwrap()
            .unwrap();
        let Feasibility::Spurious { block, frontier } =
            concretize(model, &partition, &trace, &mut prefix).unwrap()
        else {
            panic!("expected initial spurious zero-step trace");
        };
        partition.split(block, &frontier, &mut prefix).unwrap();
        prefix.tick().unwrap();
        let trace = abstract_trace(model, &partition, &mut prefix)
            .unwrap()
            .unwrap();
        let Feasibility::Real(witness) =
            concretize(model, &partition, &trace, &mut prefix).unwrap()
        else {
            panic!("expected real one-edge trace after split");
        };
        assert_eq!(witness.states, [0, 1]);
        assert_eq!(witness.actions, [9]);
        let mut replay_budget = budget();
        assert_eq!(
            replay_inner(model, &witness, || replay_budget.tick()),
            Ok(Ok(()))
        );
        assert_eq!(replay_budget.stats.work, 5);
        assert_eq!(
            verify(model, Limits::default()).stats.work,
            prefix.stats.work + replay_budget.stats.work
        );
        prefix.stats.work
    }

    #[test]
    fn verify_replay_transition_scan_spends_budget() {
        let (model, _) = replay_scan_fixture();
        let cutoff = work_before_edge_replay(&model) + 4;
        let report = verify(
            &model,
            Limits {
                max_work: cutoff,
                ..Limits::default()
            },
        );
        assert_eq!(report.outcome, Outcome::Inconclusive(StopReason::WorkLimit));
    }

    #[test]
    fn verify_replay_transition_scan_observes_cancellation() {
        let (model, _) = replay_scan_fixture();
        let cutoff = work_before_edge_replay(&model) + 4;
        let mut visits = 0;
        let report = verify_with_cancel(&model, Limits::default(), || {
            let cancel = visits == cutoff;
            visits += 1;
            cancel
        });
        assert_eq!(report.outcome, Outcome::Inconclusive(StopReason::Cancelled));
    }

    #[test]
    fn blocked_frontier_splits_without_losing_states() {
        let model = Model::new(
            5,
            &[0],
            &[4],
            &[
                Transition::new(0, 10, 1),
                Transition::new(2, 20, 3),
                Transition::new(3, 30, 4),
            ],
        )
        .unwrap();
        let mut partition = Partition {
            block_of: vec![0, 1, 1, 2, 3],
            count: 4,
        };
        let trace = abstract_trace(&model, &partition, &mut budget())
            .unwrap()
            .unwrap();
        assert_eq!(trace.blocks, [0, 1, 2, 3]);
        assert_eq!(trace.actions, [10, 20, 30]);
        let Feasibility::Spurious { block, frontier } =
            concretize(&model, &partition, &trace, &mut budget()).unwrap()
        else {
            panic!("expected spurious trace");
        };
        assert_eq!(block, 1);
        assert_eq!(frontier, [false, true, false, false, false]);
        let old = partition.block_of.clone();
        partition.split(block, &frontier, &mut budget()).unwrap();
        assert_eq!(partition.count, 5);
        for state in 0..model.state_count {
            assert_eq!(
                partition.block_of[state],
                if frontier[state] { 4 } else { old[state] }
            );
        }
        assert!(
            abstract_trace(&model, &partition, &mut budget())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn bad_endpoint_frontier_splits_from_unreachable_bad_state() {
        let model = Model::new(3, &[0], &[2], &[Transition::new(0, 42, 1)]).unwrap();
        let mut partition = Partition {
            block_of: vec![0, 1, 1],
            count: 2,
        };
        let trace = abstract_trace(&model, &partition, &mut budget())
            .unwrap()
            .unwrap();
        assert_eq!(trace.blocks, [0, 1]);
        assert_eq!(trace.actions, [42]);
        let Feasibility::Spurious { block, frontier } =
            concretize(&model, &partition, &trace, &mut budget()).unwrap()
        else {
            panic!("expected spurious bad endpoint");
        };
        assert_eq!(block, 1);
        assert_eq!(frontier, [false, true, false]);
        partition.split(block, &frontier, &mut budget()).unwrap();
        assert_eq!(partition.block_of, [0, 2, 1]);
        assert!(
            abstract_trace(&model, &partition, &mut budget())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn refinement_can_leave_same_abstract_word_via_a_different_spurious_prefix() {
        let model = Model::new(
            6,
            &[0],
            &[5],
            &[
                Transition::new(0, 10, 1),
                Transition::new(2, 20, 3),
                Transition::new(1, 20, 4),
                Transition::new(3, 30, 5),
            ],
        )
        .unwrap();
        let mut partition = Partition {
            block_of: vec![0, 1, 1, 2, 2, 3],
            count: 4,
        };
        let trace = abstract_trace(&model, &partition, &mut budget())
            .unwrap()
            .unwrap();
        assert_eq!(trace.blocks, [0, 1, 2, 3]);
        assert_eq!(trace.actions, [10, 20, 30]);
        let Feasibility::Spurious { block, frontier } =
            concretize(&model, &partition, &trace, &mut budget()).unwrap()
        else {
            panic!("expected spurious trace");
        };
        assert_eq!(block, 2);
        assert_eq!(frontier, [false, false, false, false, true, false]);
        partition.split(block, &frontier, &mut budget()).unwrap();
        let next = abstract_trace(&model, &partition, &mut budget())
            .unwrap()
            .unwrap();
        assert_eq!(next.blocks, trace.blocks);
        assert_eq!(next.actions, trace.actions);
        // Nonetheless the split eliminated the particular concrete-frontier
        // mismatch and increased the bounded number of nonempty blocks.
        assert_eq!(partition.count, 5);
    }
}
