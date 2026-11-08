//! Rust-only development tasks. This deliberately does not depend on the
//! CEGAR implementation's abstraction, trace replay, or reachability code.
//! The reference oracle uses a backwards least fixed point instead.
#![forbid(unsafe_code)]

use cegar_core::{
    Limits, MAX_STATES, MAX_TRANSITIONS, Model, Outcome, Transition, Witness, verify,
};

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

/// Unmodified generated input, kept separate from the constructor under test.
#[derive(Debug)]
struct Fixture<'a> {
    n: usize,
    initial: &'a [usize],
    bad: &'a [usize],
    edges: &'a [Transition],
}

/// Backwards fixed point over raw input, not the verifier's constructed Model.
fn reference_reaches_bad(fixture: &Fixture<'_>) -> bool {
    let mut can_reach = vec![false; fixture.n];
    for &state in fixture.bad {
        can_reach[state] = true;
    }
    loop {
        let mut changed = false;
        for edge in fixture.edges {
            if can_reach[edge.target] && !can_reach[edge.source] {
                can_reach[edge.source] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    fixture.initial.iter().any(|&state| can_reach[state])
}

/// Independent labeled witness check against raw input, not core replay.
fn reference_witness_valid(fixture: &Fixture<'_>, witness: &Witness) -> bool {
    if witness.states.is_empty()
        || witness.states.len() != witness.actions.len() + 1
        || witness.states.len() > MAX_STATES
    {
        return false;
    }
    if !fixture.initial.contains(&witness.states[0]) {
        return false;
    }
    let Some(&last) = witness.states.last() else {
        return false;
    };
    if !fixture.bad.contains(&last) {
        return false;
    }
    if witness.states.iter().any(|&state| state >= fixture.n) {
        return false;
    }
    witness
        .states
        .windows(2)
        .zip(witness.actions.iter())
        .all(|(states, &action)| {
            fixture.edges.iter().any(|edge| {
                edge.source == states[0] && edge.action == action && edge.target == states[1]
            })
        })
}

fn compare(
    fixture: &Fixture<'_>,
    group: &str,
    index: usize,
    totals: &mut Totals,
) -> Result<(), String> {
    let model = Model::new(fixture.n, fixture.initial, fixture.bad, fixture.edges)
        .map_err(|error| format!("{group} #{index}: fixture rejected: {error}; {fixture:?}"))?;
    let mut expected_bad = fixture.bad.to_vec();
    expected_bad.sort_unstable();
    if model.state_count() != fixture.n
        || model.initial() != fixture.initial
        || model.transitions() != fixture.edges
        || model.bad_states().collect::<Vec<_>>() != expected_bad
    {
        return Err(format!(
            "{group} #{index}: constructor changed input: {fixture:?}; {model:?}"
        ));
    }
    let expected = reference_reaches_bad(fixture);
    let report = verify(&model, Limits::default());
    if report.execution_authorized() || report.scope() != "supplied_finite_model_only" {
        return Err(format!(
            "{group} #{index}: authority/scope invariant violated; fixture {fixture:?}"
        ));
    }

    match (expected, report.outcome) {
        (false, Outcome::VerifiedWithinModel) => Ok(()),
        (true, Outcome::ConcreteCounterexample(witness)) => {
            if !reference_witness_valid(fixture, &witness) {
                return Err(format!(
                    "{group} #{index}: non-replayable witness {witness:?}; fixture {fixture:?}"
                ));
            }
            totals.replayed += 1;
            Ok(())
        }
        (wanted, actual) => Err(format!(
            "{group} #{index}: oracle says reachable_bad={wanted}, got {actual:?}; fixture {fixture:?}; model {model:?}"
        )),
    }
}

fn subset(n: usize, bits: usize) -> Vec<usize> {
    (0..n)
        .filter(|&state| bits & (1usize << state) != 0)
        .collect()
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
                    let fixture = Fixture {
                        n,
                        initial: &initial,
                        bad: &bad,
                        edges: &edges,
                    };
                    compare(&fixture, "exhaustive", totals.exhaustive, totals)?;
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
                if rng.next().is_multiple_of(7) {
                    edges.push(Transition::new(source, (rng.next() % 4) as u32, target));
                }
            }
        }
        let initial_bits = 1 + (rng.next() % ((1u64 << n) - 1)) as usize;
        let bad_bits = (rng.next() % (1u64 << n)) as usize;
        let initial = subset(n, initial_bits);
        let bad = subset(n, bad_bits);
        let fixture = Fixture {
            n,
            initial: &initial,
            bad: &bad,
            edges: &edges,
        };
        compare(&fixture, "seeded", index, totals)?;
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
    if let Some(index) = rejected.iter().position(|&is_rejected| !is_rejected) {
        return Err(format!(
            "malformed constructor fixture #{index} was admitted"
        ));
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
        totals.exhaustive, totals.seeded, SEED, totals.replayed, totals.rejected
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask verification failed: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_oracle_detects_removed_initial_bad_edges_and_changed_labels() {
        let edges = [Transition::new(0, 11, 1), Transition::new(1, 12, 2)];
        let fixture = Fixture {
            n: 4,
            initial: &[0, 3],
            bad: &[2],
            edges: &edges,
        };
        let witness = Witness {
            states: vec![0, 1, 2],
            actions: vec![11, 12],
        };
        assert!(reference_reaches_bad(&fixture));
        assert!(reference_witness_valid(&fixture, &witness));
        let no_initial = Fixture {
            initial: &[3],
            ..fixture
        };
        assert!(!reference_reaches_bad(&no_initial));
        let no_bad = Fixture {
            bad: &[],
            ..fixture
        };
        assert!(!reference_reaches_bad(&no_bad));
        for removed in 0..edges.len() {
            let remaining: Vec<_> = edges
                .iter()
                .enumerate()
                .filter_map(|(i, e)| (i != removed).then_some(*e))
                .collect();
            let broken = Fixture {
                initial: &[0],
                bad: &[2],
                edges: &remaining,
                ..fixture
            };
            assert!(!reference_reaches_bad(&broken));
            assert!(!reference_witness_valid(&broken, &witness));
        }
        let changed = [edges[0], Transition::new(1, 99, 2)];
        let relabeled = Fixture {
            edges: &changed,
            ..fixture
        };
        assert!(reference_reaches_bad(&relabeled));
        assert!(!reference_witness_valid(&relabeled, &witness));

        let cyclic_edges = [Transition::new(0, 1, 0), Transition::new(0, 2, 1)];
        let cyclic = Fixture {
            n: 2,
            initial: &[0],
            bad: &[1],
            edges: &cyclic_edges,
        };
        let cyclic_witness = Witness {
            states: vec![0, 0, 1],
            actions: vec![1, 2],
        };
        assert!(reference_witness_valid(&cyclic, &cyclic_witness));
        let mut long = Witness {
            states: vec![0; MAX_STATES - 1],
            actions: vec![1; MAX_STATES - 2],
        };
        long.states.push(1);
        long.actions.push(2);
        assert!(reference_witness_valid(&cyclic, &long));
        long.states.insert(0, 0);
        long.actions.insert(0, 1);
        assert!(!reference_witness_valid(&cyclic, &long));
        assert!(!reference_witness_valid(
            &cyclic,
            &Witness {
                states: vec![0, 1],
                actions: vec![]
            }
        ));
        assert!(!reference_witness_valid(
            &cyclic,
            &Witness {
                states: vec![0, 2, 1],
                actions: vec![1, 2]
            }
        ));
    }

    #[test]
    fn compare_raw_parallel_edges_and_noncanonical_set_order() {
        let edges = [
            Transition::new(0, 1, 1),
            Transition::new(0, 2, 1),
            Transition::new(0, 1, 1),
        ];
        let fixture = Fixture {
            n: 4,
            initial: &[3, 0],
            bad: &[2, 1],
            edges: &edges,
        };
        let mut totals = Totals::default();
        compare(&fixture, "parallel", 0, &mut totals).unwrap();
        assert_eq!(totals.replayed, 1);
    }
}
