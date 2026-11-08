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
