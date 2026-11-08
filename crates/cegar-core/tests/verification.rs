use cegar_core::{
    InputError, Limits, MAX_STATES, MAX_TRANSITIONS, Model, Outcome, StopReason,
    Transition as Edge, Witness, WitnessError, replay, verify, verify_with_cancel,
};

fn model(n: usize, initial: &[usize], bad: &[usize], edges: &[Edge]) -> Model {
    Model::new(n, initial, bad, edges).expect("synthetic fixture must be valid")
}

#[test]
fn spurious_trace_refines_to_safe() {
    let m = model(4, &[0], &[3], &[Edge::new(0, 1, 2), Edge::new(1, 2, 3)]);
    let r = verify(&m, Limits::default());
    assert_eq!(r.outcome, Outcome::VerifiedWithinModel);
    assert!(r.stats.refinements > 0);
    assert!(r.stats.refinements < m.state_count());
    assert_eq!(r.scope(), "supplied_finite_model_only");
    assert!(!r.execution_authorized());
}

#[test]
fn real_counterexample_replays_with_action_identity() {
    let m = model(3, &[0], &[2], &[Edge::new(0, 17, 1), Edge::new(1, 23, 2)]);
    let r = verify(&m, Limits::default());
    let Outcome::ConcreteCounterexample(mut w) = r.outcome else {
        panic!("expected witness")
    };
    assert_eq!(w.states, [0, 1, 2]);
    assert_eq!(w.actions, [17, 23]);
    assert_eq!(replay(&m, &w), Ok(()));
    w.actions[1] = 99;
    assert_eq!(replay(&m, &w), Err(WitnessError::MissingTransition(1)));
}

#[test]
fn initial_bad_state_has_zero_step_witness() {
    let m = model(3, &[0, 2], &[2], &[]);
    let r = verify(&m, Limits::default());
    assert_eq!(
        r.outcome,
        Outcome::ConcreteCounterexample(Witness {
            states: vec![2],
            actions: vec![]
        })
    );
}

#[test]
fn empty_bad_set_is_safe_but_empty_initial_is_invalid() {
    assert_eq!(
        verify(&model(1, &[0], &[], &[]), Limits::default()).outcome,
        Outcome::VerifiedWithinModel
    );
    assert!(matches!(
        Model::new(1, &[], &[], &[]),
        Err(InputError::EmptyInitial)
    ));
}

#[test]
fn malformed_models_are_rejected() {
    assert!(matches!(
        Model::new(0, &[0], &[], &[]),
        Err(InputError::StateCount)
    ));
    assert!(matches!(
        Model::new(MAX_STATES + 1, &[0], &[], &[]),
        Err(InputError::StateCount)
    ));
    assert!(Model::new(2, &[2], &[], &[]).is_err());
    assert!(Model::new(2, &[0, 0], &[], &[]).is_err());
    assert!(Model::new(2, &[0], &[1, 1], &[]).is_err());
    assert!(Model::new(2, &[0], &[usize::MAX], &[]).is_err());
    assert!(Model::new(2, &[0], &[], &[Edge::new(usize::MAX, 0, 1)]).is_err());
    assert!(Model::new(2, &[0], &[], &[Edge::new(0, 0, usize::MAX)]).is_err());
    assert!(Model::new(2, &[0], &[], &vec![Edge::new(0, 0, 0); MAX_TRANSITIONS + 1]).is_err());
}

#[test]
fn malformed_witnesses_do_not_panic_or_certify() {
    let m = model(2, &[0], &[1], &[Edge::new(0, 5, 1)]);
    for w in [
        Witness {
            states: vec![],
            actions: vec![],
        },
        Witness {
            states: vec![0, 1],
            actions: vec![],
        },
        Witness {
            states: vec![usize::MAX],
            actions: vec![],
        },
        Witness {
            states: vec![1],
            actions: vec![],
        },
        Witness {
            states: vec![0],
            actions: vec![],
        },
        Witness {
            states: vec![0, 1],
            actions: vec![6],
        },
        Witness {
            states: vec![0; MAX_STATES + 1],
            actions: vec![5; MAX_STATES],
        },
    ] {
        assert!(replay(&m, &w).is_err());
    }
}

#[test]
fn work_limit_is_inconclusive_including_before_reporting_a_witness() {
    let m = model(3, &[0], &[2], &[Edge::new(0, 7, 1), Edge::new(1, 9, 2)]);
    let complete = verify(&m, Limits::default());
    assert!(matches!(
        complete.outcome,
        Outcome::ConcreteCounterexample(_)
    ));
    for max_work in 0..complete.stats.work {
        let r = verify(
            &m,
            Limits {
                max_work,
                ..Limits::default()
            },
        );
        assert_eq!(r.outcome, Outcome::Inconclusive(StopReason::WorkLimit));
        assert!(r.stats.work <= max_work);
        assert!(!r.execution_authorized());
    }
}

#[test]
fn insufficient_refinements_never_approve() {
    let m = model(2, &[0], &[1], &[]);
    let r = verify(
        &m,
        Limits {
            max_refinements: 0,
            ..Limits::default()
        },
    );
    assert_eq!(
        r.outcome,
        Outcome::Inconclusive(StopReason::RefinementLimit)
    );
}

#[test]
fn cancellation_is_explicit_and_repeatable_at_every_visit() {
    let m = model(4, &[0], &[3], &[Edge::new(0, 1, 2), Edge::new(1, 2, 3)]);
    let complete = verify(&m, Limits::default());
    for stop_at in 0..complete.stats.work {
        let mut visits = 0;
        let r = verify_with_cancel(&m, Limits::default(), || {
            let stop = visits == stop_at;
            visits += 1;
            stop
        });
        assert_eq!(r.outcome, Outcome::Inconclusive(StopReason::Cancelled));
        assert_eq!(r.stats.work, stop_at);
    }
    assert_eq!(verify(&m, Limits::default()), complete);
}

#[test]
fn maximum_length_chain_replays_without_recursion() {
    let edges: Vec<_> = (1..MAX_STATES).map(|s| Edge::new(s - 1, 1, s)).collect();
    let m = model(MAX_STATES, &[0], &[MAX_STATES - 1], &edges);
    let r = verify(
        &m,
        Limits {
            max_work: 20_000_000,
            max_refinements: MAX_STATES - 1,
        },
    );
    let Outcome::ConcreteCounterexample(w) = r.outcome else {
        panic!("expected full-chain witness: {r:?}")
    };
    assert_eq!(w.states.len(), MAX_STATES);
    assert_eq!(replay(&m, &w), Ok(()));
}

/// Independent transitive-closure oracle; does not use the verifier's BFS,
/// quotient, feasibility propagation, or refinement routines.
fn closure_has_bad(m: &Model) -> bool {
    let n = m.state_count();
    let mut reach = vec![vec![false; n]; n];
    for (state, row) in reach.iter_mut().enumerate() {
        row[state] = true;
    }
    for edge in m.transitions() {
        reach[edge.source][edge.target] = true;
    }
    for via in 0..n {
        for source in 0..n {
            for target in 0..n {
                let through_via = reach[source][via] && reach[via][target];
                reach[source][target] |= through_via;
            }
        }
    }
    m.initial()
        .iter()
        .any(|&s| m.bad_states().any(|b| reach[s][b]))
}

fn agrees_with_oracle(m: &Model) {
    let r = verify(m, Limits::default());
    match &r.outcome {
        Outcome::VerifiedWithinModel => assert!(!closure_has_bad(m), "false safe: {m:?}"),
        Outcome::ConcreteCounterexample(w) => {
            assert!(closure_has_bad(m), "false unsafe: {m:?}");
            assert_eq!(replay(m, w), Ok(()));
        }
        Outcome::Inconclusive(reason) => {
            panic!("small fixture exhausted default budget: {reason:?}; {m:?}")
        }
    }
    assert!(r.stats.refinements < m.state_count());
    assert!(r.stats.work <= Limits::default().max_work);
    assert!(!r.execution_authorized());
}

#[test]
fn exhaustive_all_graphs_initial_sets_and_bad_sets_up_to_three_states() {
    let mut count = 0;
    for n in 1usize..=3 {
        for edge_bits in 0..(1u64 << (n * n)) {
            let mut edges = Vec::new();
            for source in 0..n {
                for target in 0..n {
                    if edge_bits & (1 << (source * n + target)) != 0 {
                        edges.push(Edge::new(source, 0, target));
                    }
                }
            }
            for initial_bits in 1..(1u64 << n) {
                let initial: Vec<_> = (0..n).filter(|s| initial_bits & (1 << s) != 0).collect();
                for bad_bits in 0..(1u64 << n) {
                    let bad: Vec<_> = (0..n).filter(|s| bad_bits & (1 << s) != 0).collect();
                    agrees_with_oracle(&model(n, &initial, &bad, &edges));
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 28_868);
    eprintln!("exhaustive_model_comparisons={count}");
}

#[test]
fn seeded_labeled_graphs_match_independent_closure() {
    let mut seed: u64 = 0x4345_4741_525f_3031;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..1_000 {
        let n = 2 + (random() % 11) as usize;
        let mut edges = Vec::new();
        for source in 0..n {
            for target in 0..n {
                if random().is_multiple_of(7) {
                    edges.push(Edge::new(source, (random() % 4) as u32, target));
                }
            }
        }
        let initial = vec![(random() % n as u64) as usize];
        let bad: Vec<_> = (0..n).filter(|_| random().is_multiple_of(4)).collect();
        agrees_with_oracle(&model(n, &initial, &bad, &edges));
    }
}

#[test]
fn nondeterministic_frontier_keeps_all_matching_successors() {
    let m = model(
        4,
        &[0],
        &[3],
        &[Edge::new(0, 5, 1), Edge::new(0, 5, 2), Edge::new(2, 6, 3)],
    );
    let r = verify(&m, Limits::default());
    let Outcome::ConcreteCounterexample(w) = r.outcome else {
        panic!("unsafe nondeterministic branch was lost")
    };
    assert_eq!(replay(&m, &w), Ok(()));
}

#[test]
fn duplicate_edges_and_edge_order_do_not_change_safety() {
    let edges = [Edge::new(0, 1, 1), Edge::new(1, 2, 2)];
    for variant in [edges.to_vec(), vec![edges[1], edges[0], edges[0]]] {
        agrees_with_oracle(&model(3, &[0], &[2], &variant));
    }
}

#[test]
fn cycles_with_unreachable_bad_states_terminate_safely() {
    let m = model(4, &[0], &[3], &[Edge::new(0, 1, 1), Edge::new(1, 2, 0)]);
    agrees_with_oracle(&m);
    assert_eq!(
        verify(&m, Limits::default()).outcome,
        Outcome::VerifiedWithinModel
    );
}

#[test]
fn state_renaming_edge_order_and_added_bad_or_edges_preserve_unsafety() {
    // Self-loop, duplicated transition, parallel labels, disconnected component,
    // nondeterministic branch, and multiple initial and bad states.
    let edges = vec![
        Edge::new(0, 3, 0),
        Edge::new(0, 5, 1),
        Edge::new(0, 5, 2),
        Edge::new(0, 6, 2),
        Edge::new(2, 7, 3),
        Edge::new(2, 7, 3),
        Edge::new(4, 8, 4),
    ];
    let initial = [0, 4];
    let bad = [3, 5];
    let permutation = [4, 2, 5, 1, 0, 3];
    for order in [edges.clone(), edges.iter().rev().copied().collect()] {
        let renamed: Vec<_> = order
            .iter()
            .map(|e| Edge::new(permutation[e.source], e.action, permutation[e.target]))
            .collect();
        let renamed_initial: Vec<_> = initial.iter().map(|&s| permutation[s]).collect();
        let renamed_bad: Vec<_> = bad.iter().map(|&s| permutation[s]).collect();
        for m in [
            model(6, &initial, &bad, &order),
            model(6, &renamed_initial, &renamed_bad, &renamed),
        ] {
            agrees_with_oracle(&m);
            assert!(
                matches!(
                    verify(&m, Limits::default()).outcome,
                    Outcome::ConcreteCounterexample(_)
                ),
                "{m:?}"
            );
        }
    }
    let base = model(6, &[0], &[3], &edges);
    assert!(matches!(
        verify(&base, Limits::default()).outcome,
        Outcome::ConcreteCounterexample(_)
    ));
    let mut more_edges = edges.clone();
    more_edges.push(Edge::new(4, u32::MAX, 5));
    let more = model(6, &[0], &[3, 5], &more_edges);
    agrees_with_oracle(&more);
    assert!(matches!(
        verify(&more, Limits::default()).outcome,
        Outcome::ConcreteCounterexample(_)
    ));
}

#[test]
fn exact_boundaries_and_constructor_roundtrip_preserve_labels_and_duplicates() {
    let edges = vec![Edge::new(0, u32::MAX, MAX_STATES - 1); MAX_TRANSITIONS];
    let m = model(
        MAX_STATES,
        &[MAX_STATES - 1, 0],
        &[MAX_STATES - 1, 1],
        &edges,
    );
    assert_eq!(m.initial(), &[MAX_STATES - 1, 0]);
    assert_eq!(m.bad_states().collect::<Vec<_>>(), [1, MAX_STATES - 1]);
    assert_eq!(m.transitions(), edges);
    assert_eq!(
        Model::new(2, &[0, 0, 0], &[], &[]).unwrap_err(),
        InputError::SetTooLarge("initial")
    );
    assert_eq!(
        Model::new(2, &[0], &[0, 0, 0], &[]).unwrap_err(),
        InputError::SetTooLarge("bad")
    );
    assert_eq!(
        Model::new(2, &[0], &[], &[Edge::new(0, 0, 2)]).unwrap_err(),
        InputError::UnknownState {
            field: "target",
            state: 2
        }
    );
}

#[test]
fn cancellation_and_exact_work_boundary_cover_real_witness_replay() {
    let m = model(
        4,
        &[0],
        &[3],
        &[Edge::new(0, 7, 1), Edge::new(1, 8, 2), Edge::new(2, 9, 3)],
    );
    let complete = verify(&m, Limits::default());
    assert!(matches!(
        complete.outcome,
        Outcome::ConcreteCounterexample(_)
    ));
    for stop_at in 0..complete.stats.work {
        let mut visits = 0;
        let report = verify_with_cancel(&m, Limits::default(), || {
            let stop = visits == stop_at;
            visits += 1;
            stop
        });
        assert_eq!(
            report.outcome,
            Outcome::Inconclusive(StopReason::Cancelled),
            "stop_at={stop_at}"
        );
        assert_eq!(report.stats.work, stop_at);
    }
    for max_work in [complete.stats.work, complete.stats.work + 1] {
        assert_eq!(
            verify(
                &m,
                Limits {
                    max_work,
                    ..Limits::default()
                }
            )
            .outcome,
            complete.outcome
        );
    }
    assert_eq!(
        verify(
            &m,
            Limits {
                max_work: complete.stats.work - 1,
                ..Limits::default()
            }
        )
        .outcome,
        Outcome::Inconclusive(StopReason::WorkLimit)
    );
}

#[test]
fn refinement_budget_boundary_and_cyclic_replay() {
    let m = model(4, &[0], &[3], &[Edge::new(0, 1, 2), Edge::new(1, 2, 3)]);
    let complete = verify(&m, Limits::default());
    assert!(complete.stats.refinements >= 2, "{complete:?}");
    assert_eq!(
        verify(
            &m,
            Limits {
                max_refinements: complete.stats.refinements - 1,
                ..Limits::default()
            }
        )
        .outcome,
        Outcome::Inconclusive(StopReason::RefinementLimit)
    );
    assert_eq!(
        verify(
            &m,
            Limits {
                max_refinements: complete.stats.refinements,
                ..Limits::default()
            }
        )
        .outcome,
        Outcome::VerifiedWithinModel
    );
    let cyclic = model(2, &[0], &[1], &[Edge::new(0, 1, 0), Edge::new(0, 2, 1)]);
    assert_eq!(
        replay(
            &cyclic,
            &Witness {
                states: vec![0, 0, 1],
                actions: vec![1, 2]
            }
        ),
        Ok(())
    );
}
