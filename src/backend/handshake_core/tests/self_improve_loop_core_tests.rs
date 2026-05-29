//! MT-148 contract proof: SelfImprovementLoop core module + LoopIteration
//! state machine.
//!
//! Per MT-148 contract `owned_files` this file is the dedicated proof
//! surface for MT-148. The other test file
//! `tests/self_improve_loop_tests.rs` proves the MT-149..MT-156 chain
//! composes on top of MT-148; this file proves MT-148 in isolation.
//!
//! Test scenarios required by the MT-148 brief reset:
//!   1. Happy-path full 7-state cycle round-trip; iteration_id v7 and
//!      iteration_number monotonic.
//!   2. Invalid transition: skipping stages returns a typed error
//!      (not panic).
//!   3. State-machine fuzzing: every (from, to) pair in `LoopStage::ALL ×
//!      LoopStage::ALL` is classified correctly — exactly the 6 forward
//!      edges are valid; every other edge is invalid.
//!   4. Concurrency: `Arc<Mutex<SelfImprovementLoop>>` driven from 2
//!      threads produces no torn state and a consistent counter.
//!   5. Bounded history: inserting 1000 finalized iterations retains only
//!      the last `history_capacity` (defaults to 100; tests override to 4
//!      for fast assertion).
//!   6. Idempotent transition: `current()` called twice returns the same
//!      value without mutating state.
//!   7. Goodhart-pause slot: `pause(...)` causes `begin_iteration` to
//!      return `LoopPausedError`; `unpause` clears the pause.
//!   8. Promotion-gate slot: `enqueue_review_token` and
//!      `dequeue_review_token` round-trip the token, `last_review_token`
//!      tracks the most recent issuance.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use handshake_core::self_improve::iteration::{
    LoopIteration, LoopIterationError, LoopStage, LoopTarget, PolicyParameterRef,
};
use handshake_core::self_improve::{
    shared_self_improvement_loop, LoopMetricSnapshot, LoopPauseReason, LoopPausedError,
    ReviewToken, SelfImprovementLoop, SELF_IMPROVEMENT_LOOP_HISTORY_DEFAULT_CAPACITY,
};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// 1. Happy-path full 7-state cycle round-trip
// ---------------------------------------------------------------------------

#[test]
fn happy_path_iteration_walks_all_seven_stages_in_order() {
    let mut it = LoopIteration::new(0);
    assert_eq!(it.current(), LoopStage::ChooseTarget);
    let expected_chain = [
        LoopStage::IsolateEditableSurface,
        LoopStage::RunInSandbox,
        LoopStage::ExecuteEval,
        LoopStage::AcceptReject,
        LoopStage::RecordAsMemory,
        LoopStage::Complete,
    ];
    for stage in expected_chain {
        let prev = it.advance().unwrap();
        assert_eq!(stage, it.current());
        assert_eq!(prev.next(), Some(stage));
    }
    assert!(it.current().is_terminal());
    assert!(it.completed_at_utc.is_some());
    // Confirm further advance is rejected as AlreadyComplete.
    assert!(matches!(
        it.advance().unwrap_err(),
        LoopIterationError::AlreadyComplete
    ));
}

#[test]
fn happy_path_iteration_id_is_v7_and_iteration_number_is_monotonic_within_loop() {
    let mut loop_inst = SelfImprovementLoop::new();
    let first_iter_number;
    let first_iter_id;
    {
        let it = loop_inst.begin_iteration().unwrap();
        first_iter_id = it.iteration_id;
        first_iter_number = it.iteration_number;
        assert_eq!(it.iteration_id.get_version_num(), 7);
    }
    loop_inst.finalize_current();
    let second_iter_number;
    let second_iter_id;
    {
        let it = loop_inst.begin_iteration().unwrap();
        second_iter_id = it.iteration_id;
        second_iter_number = it.iteration_number;
        assert_eq!(it.iteration_id.get_version_num(), 7);
    }
    assert_eq!(second_iter_number, first_iter_number + 1);
    assert_ne!(first_iter_id, second_iter_id);
    assert_eq!(loop_inst.iteration_counter(), 2);
}

// ---------------------------------------------------------------------------
// 2. Invalid transition returns typed error
// ---------------------------------------------------------------------------

#[test]
fn skipping_stages_returns_typed_illegal_transition_error() {
    let mut it = LoopIteration::new(0);
    // ChooseTarget -> ExecuteEval skips two stages and must error.
    let err = it.transition_to(LoopStage::ExecuteEval).unwrap_err();
    match err {
        LoopIterationError::IllegalTransition { from, to } => {
            assert_eq!(from, LoopStage::ChooseTarget);
            assert_eq!(to, LoopStage::ExecuteEval);
        }
        other => panic!("expected IllegalTransition, got {other:?}"),
    }
    // The state must NOT have mutated.
    assert_eq!(it.current(), LoopStage::ChooseTarget);
}

#[test]
fn back_edge_transition_is_rejected_even_though_previous_returns_value() {
    let mut it = LoopIteration::new(0);
    it.advance().unwrap();
    assert_eq!(it.current(), LoopStage::IsolateEditableSurface);
    // `previous` exists for inspection but is not a valid forward edge.
    assert_eq!(
        LoopStage::IsolateEditableSurface.previous(),
        Some(LoopStage::ChooseTarget)
    );
    let err = it
        .transition_to(LoopStage::ChooseTarget)
        .expect_err("back-edge must be rejected");
    assert!(matches!(err, LoopIterationError::IllegalTransition { .. }));
}

#[test]
fn self_edge_transition_is_rejected() {
    let mut it = LoopIteration::new(0);
    // ChooseTarget -> ChooseTarget is a no-op and must be rejected as
    // illegal so callers cannot accidentally double-execute a stage.
    let err = it.transition_to(LoopStage::ChooseTarget).unwrap_err();
    assert!(matches!(err, LoopIterationError::IllegalTransition { .. }));
    assert_eq!(it.current(), LoopStage::ChooseTarget);
}

// ---------------------------------------------------------------------------
// 3. State-machine fuzzing: full transition matrix
// ---------------------------------------------------------------------------

#[test]
fn transition_matrix_classifies_every_edge_correctly() {
    // Hand-written 7×7 = 49-edge matrix. Exactly the 6 forward edges are
    // valid; every other (from, to) pair must classify as invalid.
    let valid: HashSet<(LoopStage, LoopStage)> = [
        (LoopStage::ChooseTarget, LoopStage::IsolateEditableSurface),
        (LoopStage::IsolateEditableSurface, LoopStage::RunInSandbox),
        (LoopStage::RunInSandbox, LoopStage::ExecuteEval),
        (LoopStage::ExecuteEval, LoopStage::AcceptReject),
        (LoopStage::AcceptReject, LoopStage::RecordAsMemory),
        (LoopStage::RecordAsMemory, LoopStage::Complete),
    ]
    .into_iter()
    .collect();

    let mut valid_count = 0usize;
    let mut invalid_count = 0usize;
    for from in LoopStage::ALL {
        for to in LoopStage::ALL {
            let expected = valid.contains(&(from, to));
            assert_eq!(
                from.is_valid_transition(to),
                expected,
                "edge ({from:?} -> {to:?}) classified incorrectly"
            );
            if expected {
                valid_count += 1;
            } else {
                invalid_count += 1;
            }
        }
    }
    assert_eq!(valid_count, 6, "expected exactly 6 valid edges");
    assert_eq!(invalid_count, 49 - 6, "expected 43 invalid edges");

    // Cross-check: every invalid edge raised through `transition_to` must
    // return IllegalTransition and not panic.
    for from in LoopStage::ALL {
        for to in LoopStage::ALL {
            if from.is_valid_transition(to) {
                continue;
            }
            let mut it = LoopIteration::new(0);
            // Fast-forward `it` to `from`.
            while it.current() != from {
                if it.advance().is_err() {
                    break;
                }
            }
            // If we cannot fast-forward to `from` (e.g. `from == Complete`),
            // skip — `Complete` is exercised separately as AlreadyComplete.
            if it.current() != from {
                continue;
            }
            match it.transition_to(to) {
                Err(LoopIterationError::IllegalTransition { .. }) => {}
                Err(LoopIterationError::AlreadyComplete) if from == LoopStage::Complete => {}
                other => panic!("({from:?} -> {to:?}) expected IllegalTransition, got {other:?}"),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Concurrency: two threads driving the same SelfImprovementLoop
// ---------------------------------------------------------------------------

#[test]
fn two_threads_driving_loop_through_arc_mutex_produce_consistent_state() {
    // Capacity comfortably exceeds 2 * iters_per_thread so history retains
    // every iteration; counter monotonicity is the independent invariant.
    let loop_handle = shared_self_improvement_loop(64);
    let iters_per_thread = 5;
    let h1 = {
        let loop_handle = Arc::clone(&loop_handle);
        thread::spawn(move || {
            for _ in 0..iters_per_thread {
                let mut guard = loop_handle.lock().unwrap();
                guard.begin_iteration().expect("begin not paused");
                // Walk the iteration through every stage.
                {
                    let it = guard.current_iteration_mut().unwrap();
                    while !it.current().is_terminal() {
                        it.advance().unwrap();
                    }
                }
                guard.finalize_current();
            }
        })
    };
    let h2 = {
        let loop_handle = Arc::clone(&loop_handle);
        thread::spawn(move || {
            for _ in 0..iters_per_thread {
                let mut guard = loop_handle.lock().unwrap();
                guard.begin_iteration().expect("begin not paused");
                let it = guard.current_iteration_mut().unwrap();
                while !it.current().is_terminal() {
                    it.advance().unwrap();
                }
                guard.finalize_current();
            }
        })
    };
    h1.join().unwrap();
    h2.join().unwrap();
    let guard = loop_handle.lock().unwrap();
    assert_eq!(guard.iteration_counter() as usize, 2 * iters_per_thread);
    assert_eq!(guard.history().len(), 2 * iters_per_thread);

    // No torn state: every iteration in history must have reached
    // `Complete` and have a unique iteration_id.
    let mut seen = HashSet::new();
    for it in guard.history() {
        assert!(it.current().is_terminal());
        assert!(it.completed_at_utc.is_some());
        assert!(seen.insert(it.iteration_id), "duplicate iteration_id");
    }
    // Iteration numbers must form a 1..=10 set (some interleaving allowed).
    let numbers: HashSet<u32> = guard.history().iter().map(|it| it.iteration_number).collect();
    let expected: HashSet<u32> = (1u32..=(2 * iters_per_thread as u32)).collect();
    assert_eq!(numbers, expected);
}

// ---------------------------------------------------------------------------
// 5. Bounded history: 1000 inserts retain only the cap
// ---------------------------------------------------------------------------

#[test]
fn bounded_history_retains_only_capacity_entries() {
    let cap = 4usize;
    let mut loop_inst = SelfImprovementLoop::with_capacity(cap);
    let total = 1000;
    for _ in 0..total {
        loop_inst.begin_iteration().unwrap();
        // Just finalize without walking — the bounded history only cares
        // about count.
        loop_inst.finalize_current();
    }
    assert_eq!(loop_inst.history().len(), cap);
    assert_eq!(loop_inst.iteration_counter() as usize, total);
    assert_eq!(loop_inst.history_capacity(), cap);
    // The retained entries must be the LAST `cap`, by iteration number.
    let mut numbers: Vec<u32> = loop_inst
        .history()
        .iter()
        .map(|it| it.iteration_number)
        .collect();
    numbers.sort();
    let expected_low = (total as u32) - (cap as u32) + 1;
    assert_eq!(numbers[0], expected_low);
    assert_eq!(*numbers.last().unwrap(), total as u32);
}

#[test]
fn default_capacity_is_one_hundred_per_mt148_constant() {
    assert_eq!(SELF_IMPROVEMENT_LOOP_HISTORY_DEFAULT_CAPACITY, 100);
    let loop_inst = SelfImprovementLoop::new();
    assert_eq!(loop_inst.history_capacity(), 100);
}

// ---------------------------------------------------------------------------
// 6. Idempotent transition: `current()` is pure
// ---------------------------------------------------------------------------

#[test]
fn current_is_idempotent_and_does_not_mutate_state() {
    let mut it = LoopIteration::new(7);
    let s1 = it.current();
    let s2 = it.current();
    assert_eq!(s1, s2);
    assert_eq!(s1, LoopStage::ChooseTarget);
    // Walk forward one step and re-check.
    it.advance().unwrap();
    let s3 = it.current();
    let s4 = it.current();
    assert_eq!(s3, s4);
    assert_eq!(s3, LoopStage::IsolateEditableSurface);
}

// ---------------------------------------------------------------------------
// 7. Goodhart-pause slot
// ---------------------------------------------------------------------------

#[test]
fn goodhart_pause_slot_blocks_new_iterations_until_unpaused() {
    let mut loop_inst = SelfImprovementLoop::new();
    // Run one iteration baseline.
    loop_inst.begin_iteration().unwrap();
    loop_inst.finalize_current();
    assert_eq!(loop_inst.iteration_counter(), 1);

    // Sentinel pause.
    loop_inst.pause(LoopPauseReason::GoodhartGapWidening {
        detected_at_iteration: 1,
        gap: 0.18,
    });
    assert!(loop_inst.pause_reason().is_some());

    let err = loop_inst.begin_iteration().expect_err("must error while paused");
    let LoopPausedError { reason } = err;
    match reason {
        LoopPauseReason::GoodhartGapWidening { gap, .. } => {
            assert!((gap - 0.18).abs() < f64::EPSILON);
        }
        other => panic!("expected GoodhartGapWidening, got {other:?}"),
    }
    // Counter must NOT have advanced during the paused begin attempt.
    assert_eq!(loop_inst.iteration_counter(), 1);

    // Unpause and confirm new iterations succeed.
    let cleared = loop_inst.unpause();
    assert!(cleared.is_some());
    assert!(loop_inst.pause_reason().is_none());
    loop_inst.begin_iteration().unwrap();
    assert_eq!(loop_inst.iteration_counter(), 2);
}

#[test]
fn operator_pause_reason_round_trips() {
    let mut loop_inst = SelfImprovementLoop::new();
    let now = Utc::now();
    loop_inst.pause(LoopPauseReason::OperatorPause {
        rationale: "manual review".to_string(),
        paused_at_utc: now,
    });
    let err = loop_inst.begin_iteration().expect_err("must error");
    let LoopPausedError { reason } = err;
    match reason {
        LoopPauseReason::OperatorPause { rationale, .. } => {
            assert_eq!(rationale, "manual review");
        }
        _ => panic!("expected OperatorPause"),
    }
}

// ---------------------------------------------------------------------------
// 8. Promotion-gate slot via ReviewToken
// ---------------------------------------------------------------------------

#[test]
fn review_token_queue_round_trips_and_tracks_last_issued() {
    let mut loop_inst = SelfImprovementLoop::new();
    let t1 = ReviewToken::new(Uuid::now_v7(), Uuid::now_v7());
    let t2 = ReviewToken::new(Uuid::now_v7(), Uuid::now_v7());
    let t3 = ReviewToken::new(Uuid::now_v7(), Uuid::now_v7());

    assert_eq!(loop_inst.pending_review_queue_len(), 0);
    assert!(loop_inst.last_review_token().is_none());

    loop_inst.enqueue_review_token(t1.clone());
    loop_inst.enqueue_review_token(t2.clone());
    loop_inst.enqueue_review_token(t3.clone());
    assert_eq!(loop_inst.pending_review_queue_len(), 3);
    assert_eq!(loop_inst.last_review_token().unwrap(), &t3);

    // FIFO dequeue.
    assert_eq!(loop_inst.dequeue_review_token().unwrap(), t1);
    assert_eq!(loop_inst.dequeue_review_token().unwrap(), t2);
    assert_eq!(loop_inst.dequeue_review_token().unwrap(), t3);
    assert!(loop_inst.dequeue_review_token().is_none());
    // last_review_token retains the most recent issuance even after
    // dequeue — so MT-154 can re-query the latest gate ticket.
    assert_eq!(loop_inst.last_review_token().unwrap(), &t3);
}

// ---------------------------------------------------------------------------
// Per-target metric snapshot slot (MT-148 declares; MT-151 wires)
// ---------------------------------------------------------------------------

#[test]
fn per_target_metric_snapshot_round_trips() {
    let mut loop_inst = SelfImprovementLoop::new();
    let now = Utc::now();
    let target = LoopTarget::RetrievalPolicyParams {
        task_type: handshake_core::memory::TaskType::ValidatorHbrTestPacket,
        parameter: PolicyParameterRef::TopK,
    };
    let key = format!("{target:?}"); // MT-148 stores by stable string key.
    let snap = LoopMetricSnapshot {
        last_eval_dev_pass_rate: 0.72,
        last_eval_holdout_pass_rate: 0.65,
        last_eval_at_utc: now,
    };
    loop_inst.record_metric_snapshot(key.clone(), snap.clone());
    assert_eq!(loop_inst.metric_snapshot(&key).unwrap(), &snap);
}

// ---------------------------------------------------------------------------
// Concurrency adversarial: contended pause/unpause
// ---------------------------------------------------------------------------

#[test]
fn contended_pause_unpause_does_not_lose_iterations() {
    let loop_handle = Arc::new(Mutex::new(SelfImprovementLoop::with_capacity(64)));
    let iters = 20;
    // Thread A runs iterations.
    let runner = {
        let loop_handle = Arc::clone(&loop_handle);
        thread::spawn(move || {
            let mut completed = 0;
            for _ in 0..iters {
                let mut guard = loop_handle.lock().unwrap();
                let begun = guard.begin_iteration().is_ok();
                if begun {
                    {
                        let it = guard.current_iteration_mut().unwrap();
                        while !it.current().is_terminal() {
                            it.advance().unwrap();
                        }
                    }
                    guard.finalize_current();
                    completed += 1;
                } else {
                    // Pause race won; clear and retry next iter.
                    guard.unpause();
                }
            }
            completed
        })
    };
    // Thread B occasionally pauses + unpauses.
    let toggler = {
        let loop_handle = Arc::clone(&loop_handle);
        thread::spawn(move || {
            for i in 0..iters {
                let mut guard = loop_handle.lock().unwrap();
                if i % 2 == 0 {
                    guard.pause(LoopPauseReason::OperatorPause {
                        rationale: format!("toggle {i}"),
                        paused_at_utc: Utc::now(),
                    });
                } else {
                    guard.unpause();
                }
            }
        })
    };

    let completed = runner.join().unwrap();
    toggler.join().unwrap();
    let guard = loop_handle.lock().unwrap();
    // The counter must equal the number of *successful* begin_iteration
    // calls (no torn increments).
    assert_eq!(guard.iteration_counter(), completed as u32);
    assert_eq!(guard.history().len(), completed);
}
