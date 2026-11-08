//! Rust-only development tasks. This deliberately does not depend on the
//! CEGAR implementation's abstraction, trace replay, or reachability code.
//! The reference oracle uses a backwards least fixed point instead.
#![forbid(unsafe_code)]

use cegar_core::{Limits, MAX_STATES, MAX_TRANSITIONS, Model, Outcome, Transition, Witness, verify};

const SEED: u64 = 0x43_45_47_41_52;
const SEEDED_CASES: usize = 1_000;
const EXPECTED_EXHAUSTIVE: usize = 28_868;

#[derive(Default)]
struct Totals {
    exhaustive: usize,
    seeded: usize,
    replayed: usize,
    rejected: usize,
}

/// Independent algorithm: iteratively find all states that can reach Bad.
/// Does not use the CEGAR quotient, concretization, or the core's replay.
fn reference_reaches_bad(model: &Model) -> bool {
    let mut can_reach = vec![false; model.state_count()];
    for state in model.bad_states() {
        can_reach[state] = true;
    }
    loop {
        let mut changed = false;
        for edge in model.transitions() {
            if can_reach[edge.target] && !can_reach[edge.source] {
                can_reach[edge.source] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    model.initial().iter().any(|&state| can_reach[state])
}

/// Independent witness checker; no call to cegar_core::replay.
fn reference_witness_valid(model: &Model, witness: &Witness) -> bool {
    if witness.states.is_empty()
        || witness.states.len() != witness.actions.len() + 1
        || witness.states.len() > model.state_count()
    {
        return false;
    }
    if !model.initial().contains(&witness.states[0]) {
        return false;
    }
    let Some(&last) = witness.states.last() else {
        return false;
    };
    if !model.bad_states().any(|state| state == last) {
        return false;
    }
    if witness.states.iter().any(|&state| state >= model.state_count()) {
        return false;
    }
    witness
        .states
        .windows(2)
        .zip(witness.actions.iter())
        .all(|(states, &action)| {
            model.transitions().iter().any(|edge| {
                edge.source == states[0]
                    && edge.action == action
                    && edge.target == states[1]
            })
        })
}

fn compare(model: &Model, group: &str, index: usize, totals: &mut Totals) -> Result<(), String> {
    let expected = reference_reaches_bad(model);
    let report = verify(model, Limits::default());
    if report.execution_authorized() || report.scope() != "supplied_finite_model_only" {
        return Err(format!("{group} #{index}: authority/scope invariant violated"));
    }

    match (expected, report.outcome) {
        (false, Outcome::VerifiedWithinModel) => Ok(()),
        (true, Outcome::ConcreteCounterexample(witness)) => {
            if !reference_witness_valid(model, &witness) {
                return Err(format!(
                    "{group} #{index}: non-replayable witness {witness:?}; model {model:?}"
                ));
            }
            totals.replayed += 1;
            Ok(())
        }
        (wanted, actual) => Err(format!(
            "{group} #{index}: oracle says reachable_bad={wanted}, got {actual:?}; model {model:?}"
        )),
    }
}

fn subset(n: usize, bits: usize) -> Vec<usize> {
    (0..n).filter(|&state| bits & (1usize << state) != 0).collect()
}

fn exhaustive(totals: &mut Totals) -> Result<(), String> {
    for n in 1..=3 {
        for edges_bits in 0..(1usize << (n * n)) {
            let mut edges = Vec::new();
            for source in 0..n {
                for target in 0..n {
                    if edges_bits & (1usize << (source * n + target)) != 0 {
                        edges.push(Transition::new(source, 0, target));
                    }
                }
            }
            for initial_bits in 1..(1usize << n) {
                let initial = subset(n, initial_bits);
                for bad_bits in 0..(1usize << n) {
                    let bad = subset(n, bad_bits);
                    let model = Model::new(n, &initial, &bad, &edges)
                        .map_err(|error| format!("unexpected fixture rejection: {error}"))?;
                    compare(&model, "exhaustive", totals.exhaustive, totals)?;
                    totals.exhaustive += 1;
                }
            }
        }
    }
    if totals.exhaustive != EXPECTED_EXHAUSTIVE {
        return Err(format!(
            "expected {EXPECTED_EXHAUSTIVE} exhaustive models, got {}",
            totals.exhaustive
        ));
    }
    Ok(())
}

/// A fixed, documented PRNG for reproducibility without a dependency.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

fn seeded(totals: &mut Totals) -> Result<(), String> {
    let mut rng = Rng(SEED);
    for index in 0..SEEDED_CASES {
        let n = 2 + (rng.next() % 11) as usize;
        let mut edges = Vec::new();
        for source in 0..n {
            for target in 0..n {
                if rng.next() % 7 == 0 {
                    edges.push(Transition::new(source, (rng.next() % 4) as u32, target));
                }
            }
        }
        let initial_bits = 1 + (rng.next() as usize % ((1usize << n) - 1));
        let bad_bits = rng.next() as usize % (1usize << n);
        let initial = subset(n, initial_bits);
        let bad = subset(n, bad_bits);
        let model = Model::new(n, &initial, &bad, &edges)
            .map_err(|error| format!("seeded fixture #{index} rejected: {error}"))?;
        compare(&model, "seeded", index, totals)?;
        totals.seeded += 1;
    }
    Ok(())
}

/// Constructor rejection tests replace the removed test-only whitespace
/// transport checks. When a real transport exists, it needs its own Rust tests.
fn invalid_models(totals: &mut Totals) -> Result<(), String> {
    let too_many = vec![Transition::new(0, 0, 0); MAX_TRANSITIONS + 1];
    let rejected = [
        Model::new(0, &[0], &[], &[]).is_err(),
        Model::new(MAX_STATES + 1, &[0], &[], &[]).is_err(),
        Model::new(2, &[], &[], &[]).is_err(),
        Model::new(2, &[0, 0], &[], &[]).is_err(),
        Model::new(2, &[2], &[], &[]).is_err(),
        Model::new(2, &[0], &[2], &[]).is_err(),
        Model::new(2, &[0], &[], &[Transition::new(2, 0, 0)]).is_err(),
        Model::new(2, &[0], &[], &too_many).is_err(),
    ];
    if rejected.iter().any(|&is_rejected| !is_rejected) {
        return Err("one or more malformed finite models were admitted".into());
    }
    totals.rejected = rejected.len();
    Ok(())
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some("verify") || args.next().is_some() {
        return Err("usage: cargo run -p xtask --locked --offline -- verify".into());
    }

    let mut totals = Totals::default();
    exhaustive(&mut totals)?;
    seeded(&mut totals)?;
    invalid_models(&mut totals)?;

    // A single stable JSON record: suitable for CI artifacts without Python.
    println!(
        "{{\"status\":\"passed\",\"oracle\":\"rust_backward_fixed_point\",\"exhaustive_models\":{},\"seeded_models\":{},\"seed\":{},\"witnesses_replayed\":{},\"invalid_models\":{}}}",
        totals.exhaustive,
        totals.seeded,
        SEED,
        totals.replayed,
        totals.rejected
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask verification failed: {error}");
        std::process::exit(1);
    }
}
