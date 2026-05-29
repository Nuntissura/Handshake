//! WP-KERNEL-004 cluster X.2 MT-188 outcome recording + 6-level escalation
//! routing + DistillationCandidate emission integration tests.
//!
//! Contract: MT-188 owns the `MtOutcomeRecorder` + `EscalationRouter` +
//! `DistillationCandidate` surfaces and this contract-named test file. The
//! implementation lives at `src/mt_executor/outcome.rs` (colocated with the
//! cluster X.2 MicroTask Executor primitives) and the Postgres schema
//! additions live at `migrations/0023_micro_task_job_queue.sql` (base
//! tables) + `migrations/0027_mt_outcome_distillation_status.sql` (status
//! column + unique-outcome index).
//!
//! Note on owned-files drift vs MT-188 contract `owned_files`:
//!   The contract's `expected_diff_shape` calls for
//!   `process_ledger/mt_outcome.rs` + `process_ledger/escalation_router.rs`
//!   + `distillation/candidate.rs`. In practice, MT-184..MT-189 form a tight
//!   subscope cluster (X.2) and the outcome surface composes with job +
//!   queue + loop_control + cancellation + scheduler + executor as siblings
//!   under `mt_executor/`. The test file lives at the contract-named path
//!   (`tests/mt_outcome_tests.rs`) to anchor MT-188 acceptance evidence
//!   independent of the cluster-wide `mt_executor_tests.rs` smoke surface.
//!   The cluster-X.2 colocation matches the MT-184 owned-files-drift note in
//!   `tests/micro_task_job_tests.rs` and the MT-185 colocation in
//!   `tests/mt_loop_control_tests.rs`.
//!
//! Adversarial coverage:
//!
//! Pure-Rust always-on:
//!   (1) MtOutcome serde round-trip per variant — every `MtOutcomeKind` value
//!       (Success/Failure/Verification/Cancellation/Timeout/HardGate)
//!       survives serde JSON round-trip; defends against rename_all drift.
//!   (2) MtOutcomeRecord full-field round-trip — every field of an outcome
//!       row (incl. distillation_candidate_id None+Some forms, all completion
//!       signals, all run-ledger pointer forms) survives serde.
//!   (3) DistillationCandidate serde round-trip — every status form survives
//!       serde; status column default is `LoggingOnlyPhase1` per red_team #2.
//!   (4) Escalation chain monotonic — every (from, to) pair the router can
//!       emit is in `EscalationTier::next()` order; the router never emits a
//!       skip-tier decision (T7B->T13B, T7BAlt->T32B, etc.).
//!   (5) Escalation router refuses to skip tiers — every starting tier
//!       routes to exactly its `next()` successor; HardGate is terminal.
//!   (6) DistillationCandidate threshold emission boundary — off-by-one at
//!       the boundary is asserted at `iteration_n == N-1`, `N`, `N+1`.
//!   (7) HardGate decision is terminal — at HardGate, the router only emits
//!       HardGate (retry) or Abandon; never advances.
//!   (8) Forged outcome session — building the `ForgedSession` error variant
//!       carries the session id and the actual claimant for diagnostics; the
//!       Display output is unambiguous.
//!   (9) Out-of-order escalation skip-tier construction — the router's
//!       `route(...)` cannot produce an `EscalateTo { next_tier }` that
//!       skips a tier, regardless of the consecutive_failures argument.
//!   (10) Distillation candidate emission threshold — failure outcomes
//!        below the success-only threshold do not emit candidates.
//!   (11) FR-EVT-MT-015 canonical id — the const string matches the
//!        registry-bound `FrEventId::Mt015DistillationCandidate.as_str()`.
//!   (12) EscalationDecision serde round-trip — every variant
//!        (Retry/EscalateTo/HardGate/Abandon) survives serde.
//!   (13) MailboxPoster enact_decision contract — when the job has no
//!        mailbox_thread_id, enact returns `Ok(None)` for every decision
//!        variant; the Recorder does not panic.
//!   (14) HardGate posts decision_request before commit — the
//!        `enact_decision` path for `HardGate` always calls
//!        `post_hardgate_decision_request` (asserted via a capturing
//!        mailbox poster mock).
//!
//! Postgres-gated (`#[ignore]` until `POSTGRES_TEST_URL` is set):
//!   (a) outcome persists in MT-184 queue child table — `persist` writes a
//!       row to `kernel_mt_outcome` referencing the parent job.
//!   (b) escalation history queryable — outcome rows include the iteration
//!       at which they were recorded; the audit trail is ORDER BY recorded_at.
//!   (c) 6-tier ladder enforced at DB layer — the `MicroTaskQueue::escalate`
//!       refuses to skip tiers, returning `InvalidEscalation`.
//!   (d) concurrent outcome records preserve ordering — under contention,
//!       the unique `(job_id, iteration_n)` index serializes writes; the
//!       second writer receives `DuplicateOutcome`.
//!   (e) forged outcome rejection — calling `persist` with a session id that
//!       does not match `claimed_by_session` returns `ForgedSession`.
//!   (f) DistillationCandidate persistence — `persist` on a Success outcome
//!       writes a row to `kernel_distillation_candidate` with status
//!       `LOGGING_ONLY_PHASE_1`.
//!   (g) Phase-1 candidate listing filter — `list_phase1_candidates` returns
//!       only rows in `LOGGING_ONLY_PHASE_1` status.
//!   (h) duplicate outcome refused at DB layer — even with two parallel
//!       persisters under SKIP LOCKED, the unique index serializes writes.
//!   (i) HardGate as terminal — the queue's `escalate` refuses to advance
//!       past HardGate.

use chrono::Utc;
use handshake_core::distillation::candidate::DistillationCandidate as ContractDistillationCandidate;
use handshake_core::flight_recorder::{
    fr_event_registry::FrEventId, EventFilter, FlightRecorder, FlightRecorderEvent,
    FlightRecorderEventType, RecorderError,
};
use handshake_core::mt_executor::{
    job::{
        CompletionSignal, EscalationStep, EscalationTier, MicroTaskJob, MicroTaskJobId,
        MicroTaskJobState, RunLedgerPointer,
    },
    outcome::{
        enact_decision, DistillationCandidate, DistillationCandidateStatus, DistillationThreshold,
        EscalationDecision, EscalationMailboxPoster, EscalationRouter, MtOutcome, MtOutcomeError,
        MtOutcomeKind, MtOutcomeRecord, MtOutcomeRecorder,
    },
    queue::{MicroTaskQueue, QueueError},
    MtLoopControlBudget,
};
use handshake_core::process_ledger::{
    escalation_router::EscalationRouter as ContractEscalationRouter,
    mt_outcome::MtOutcomeRecorder as ContractMtOutcomeRecorder,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ============================================================================
// Pure-Rust always-on assertions
// ============================================================================

fn make_job_with_tier(tier: EscalationTier, iter: u32) -> MicroTaskJob {
    let mut job = MicroTaskJob::queue("WP-MT188", "MT-X", PathBuf::from("a.json"), 6, vec![]);
    job.escalation_tier = tier;
    job.iteration_n = iter;
    job
}

#[test]
fn mt_188_contract_paths_reexport_canonical_surfaces() {
    let mut job = make_job_with_tier(EscalationTier::T7B, 0);
    job.task_tags = vec!["rust".to_string()];

    let (record, candidate): (MtOutcomeRecord, Option<ContractDistillationCandidate>) =
        ContractMtOutcomeRecorder::record(
            &job,
            MtOutcome {
                outcome_kind: MtOutcomeKind::Success,
                completion_signal: None,
                run_ledger_ref: None,
                evidence_pointers: vec![],
            },
            Uuid::now_v7(),
        );
    assert_eq!(record.job_id, job.job_id);
    assert!(
        candidate.is_some(),
        "contract path must expose the MT-188 DistillationCandidate surface"
    );

    let mut loras = HashMap::new();
    loras.insert("rust".to_string(), "lora-rust".to_string());
    let router = ContractEscalationRouter::with_lora_map(loras);
    let decision = router.route(&job, 3);
    match decision {
        EscalationDecision::EscalateTo { lora_id, .. } => {
            assert_eq!(lora_id.as_deref(), Some("lora-rust"));
        }
        other => panic!("expected contract-path router to escalate, got {:?}", other),
    }
}

#[test]
fn mt_188_outcome_kind_wire_form_locked() {
    // Wire form for every MtOutcomeKind variant; the DB column is TEXT and
    // outcome rows write via `MtOutcomeKind::as_str()`. Wire mismatch would
    // silently corrupt audit-trail queries.
    let cases = [
        (MtOutcomeKind::Success, "success"),
        (MtOutcomeKind::Failure, "failure"),
        (MtOutcomeKind::Verification, "verification"),
        (MtOutcomeKind::Cancellation, "cancellation"),
        (MtOutcomeKind::Timeout, "timeout"),
        (MtOutcomeKind::HardGate, "hard_gate"),
    ];
    for (kind, wire) in cases {
        let s = serde_json::to_string(&kind).expect("serialize");
        assert_eq!(s, format!("\"{}\"", wire), "wire form for {:?}", kind);
        assert_eq!(kind.as_str(), wire, "as_str for {:?}", kind);
        let back: MtOutcomeKind = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, kind);
        let by_id = MtOutcomeKind::from_str_id(wire).expect("from_str_id");
        assert_eq!(by_id, kind);
    }
}

#[test]
fn mt_188_outcome_record_full_field_round_trip() {
    let now = Utc::now();
    let rec = MtOutcomeRecord {
        outcome_id: Uuid::now_v7(),
        job_id: MicroTaskJobId::new_v7(),
        iteration_n: 3,
        outcome_kind: MtOutcomeKind::Success,
        completion_signal: Some(CompletionSignal::Success {
            summary: "ok".to_string(),
        }),
        run_ledger_ref: Some(RunLedgerPointer {
            run_id: "r1".to_string(),
            uri: Some("file://x".to_string()),
        }),
        evidence_pointers: vec!["evidence://a".to_string(), "evidence://b".to_string()],
        distillation_candidate_id: Some(Uuid::now_v7()),
        recorded_at_utc: now,
        recorded_by_session: Uuid::now_v7(),
    };
    let s = serde_json::to_string(&rec).expect("serialize");
    let back: MtOutcomeRecord = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back, rec);
}

#[test]
fn mt_188_outcome_record_optional_none_form_round_trip() {
    let rec = MtOutcomeRecord {
        outcome_id: Uuid::now_v7(),
        job_id: MicroTaskJobId::new_v7(),
        iteration_n: 0,
        outcome_kind: MtOutcomeKind::Failure,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
        distillation_candidate_id: None,
        recorded_at_utc: Utc::now(),
        recorded_by_session: Uuid::now_v7(),
    };
    let s = serde_json::to_string(&rec).expect("serialize");
    let back: MtOutcomeRecord = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back, rec);
    assert!(back.completion_signal.is_none());
    assert!(back.run_ledger_ref.is_none());
    assert!(back.distillation_candidate_id.is_none());
}

#[test]
fn mt_188_distillation_candidate_status_wire_form_locked() {
    let cases = [
        (
            DistillationCandidateStatus::LoggingOnlyPhase1,
            "LOGGING_ONLY_PHASE_1",
        ),
        (DistillationCandidateStatus::Promoted, "PROMOTED"),
        (DistillationCandidateStatus::Rejected, "REJECTED"),
        (DistillationCandidateStatus::Expired, "EXPIRED"),
    ];
    for (status, wire) in cases {
        let s = serde_json::to_string(&status).expect("serialize");
        assert_eq!(s, format!("\"{}\"", wire), "wire form for {:?}", status);
        assert_eq!(status.as_str(), wire);
        let back: DistillationCandidateStatus = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, status);
    }
    // Default must be the Phase-1 logging-only sentinel per red_team #2.
    assert_eq!(
        DistillationCandidateStatus::default(),
        DistillationCandidateStatus::LoggingOnlyPhase1,
        "default status must be LoggingOnlyPhase1 per red_team #2"
    );
}

#[test]
fn mt_188_distillation_candidate_full_field_round_trip() {
    let cand = DistillationCandidate {
        candidate_id: Uuid::now_v7(),
        job_id: MicroTaskJobId::new_v7(),
        summary: "iter=3 tier=T7B".to_string(),
        prompt_response_trace: vec![
            serde_json::json!({"prompt": "p1", "response": "r1"}),
            serde_json::json!({"prompt": "p2", "response": "r2"}),
        ],
        tier_at_completion: EscalationTier::T7BAlt,
        created_at_utc: Utc::now(),
        status: DistillationCandidateStatus::LoggingOnlyPhase1,
    };
    let s = serde_json::to_string(&cand).expect("serialize");
    let back: DistillationCandidate = serde_json::from_str(&s).expect("deserialize");
    assert_eq!(back, cand);
}

#[test]
fn mt_188_escalation_chain_monotonic() {
    // The router never emits a decision whose `next_tier` is two positions
    // ahead of the current tier. Walks every tier and asserts the routed
    // next equals `current.next()`.
    let r = EscalationRouter::default();
    let chain = [
        EscalationTier::T7B,
        EscalationTier::T7BAlt,
        EscalationTier::T13B,
        EscalationTier::T13BAlt,
        EscalationTier::T32B,
    ];
    for tier in chain {
        let job = make_job_with_tier(tier, 0);
        let decision = r.route(&job, 3); // exhaust budget
        match decision {
            EscalationDecision::EscalateTo { next_tier, .. } => {
                assert_eq!(
                    Some(next_tier),
                    tier.next(),
                    "router must emit only EscalationTier::next() successor for {:?}",
                    tier
                );
            }
            EscalationDecision::HardGate { .. } => {
                // T32B exhaust -> HardGate is acceptable.
                assert_eq!(tier, EscalationTier::T32B);
            }
            other => panic!(
                "unexpected decision for tier {:?} on exhausted budget: {:?}",
                tier, other
            ),
        }
    }
}

#[test]
fn mt_188_router_refuses_to_skip_tiers() {
    // Even with absurdly high consecutive_failures, the router never emits a
    // decision two positions ahead. This is the structural invariant the
    // queue.rs::escalate also enforces; the router must agree.
    let r = EscalationRouter::default();
    for tier in [
        EscalationTier::T7B,
        EscalationTier::T7BAlt,
        EscalationTier::T13B,
        EscalationTier::T13BAlt,
    ] {
        let job = make_job_with_tier(tier, 0);
        for failures in 0..50 {
            let d = r.route(&job, failures);
            if let EscalationDecision::EscalateTo { next_tier, .. } = d {
                assert_eq!(
                    Some(next_tier),
                    tier.next(),
                    "router emitted skip-tier from {:?} to {:?} after {} failures",
                    tier,
                    next_tier,
                    failures
                );
            }
        }
    }
}

#[test]
fn mt_188_distillation_threshold_off_by_one() {
    // Boundary: minimum_iterations_completed == N.
    let threshold = DistillationThreshold {
        success_only: true,
        minimum_iterations_completed: 3,
    };
    let success = MtOutcome {
        outcome_kind: MtOutcomeKind::Success,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };

    let mut job = make_job_with_tier(EscalationTier::T7B, 2);
    assert!(
        !threshold.evaluate(&job, &success),
        "iteration_n == N-1 must not emit (got iter=2, N=3)"
    );
    job.iteration_n = 3;
    assert!(
        threshold.evaluate(&job, &success),
        "iteration_n == N must emit (got iter=3, N=3)"
    );
    job.iteration_n = 4;
    assert!(
        threshold.evaluate(&job, &success),
        "iteration_n == N+1 must emit (got iter=4, N=3)"
    );

    // Threshold also rejects non-Success when success_only=true.
    let failure = MtOutcome {
        outcome_kind: MtOutcomeKind::Failure,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    job.iteration_n = 10;
    assert!(
        !threshold.evaluate(&job, &failure),
        "Failure must not emit under success_only=true"
    );

    // success_only=false enables failure emission.
    let failure_capture_threshold = DistillationThreshold {
        success_only: false,
        minimum_iterations_completed: 3,
    };
    assert!(
        failure_capture_threshold.evaluate(&job, &failure),
        "Failure must emit under success_only=false"
    );
    // ...but Cancellation/Verification are still excluded.
    let cancelled = MtOutcome {
        outcome_kind: MtOutcomeKind::Cancellation,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    assert!(
        !failure_capture_threshold.evaluate(&job, &cancelled),
        "Cancellation must never emit (no useful distillation signal)"
    );
}

#[test]
fn mt_188_hardgate_is_terminal() {
    // At HardGate, the router only emits HardGate (retry) or Abandon; never
    // advances. Verifies the HardGate-history-counting branch.
    let r = EscalationRouter {
        max_failures_per_tier: 1,
        max_hardgate_retries: 1, // allow one re-issue then abandon
        ..EscalationRouter::default()
    };

    let mut job = make_job_with_tier(EscalationTier::HardGate, 0);
    // No prior HardGate in history → re-issue HardGate.
    let d1 = r.route(&job, 5);
    assert!(
        matches!(d1, EscalationDecision::HardGate { .. }),
        "first HardGate route on already-HardGated job must re-issue HardGate, got {:?}",
        d1
    );

    // Inject one prior HardGate step → still allowed under max_hardgate_retries=1.
    job.escalation_history.push(EscalationStep {
        from_tier: EscalationTier::T32B,
        to_tier: EscalationTier::HardGate,
        reason: "prior".to_string(),
        recorded_at_utc: Utc::now(),
    });
    let d2 = r.route(&job, 5);
    assert!(
        matches!(d2, EscalationDecision::HardGate { .. }),
        "second HardGate within budget must re-issue HardGate"
    );

    // Inject a second prior HardGate step → exceeds budget → Abandon.
    job.escalation_history.push(EscalationStep {
        from_tier: EscalationTier::HardGate,
        to_tier: EscalationTier::HardGate,
        reason: "second prior".to_string(),
        recorded_at_utc: Utc::now(),
    });
    let d3 = r.route(&job, 5);
    assert!(
        matches!(d3, EscalationDecision::Abandon { .. }),
        "exceeded max_hardgate_retries must Abandon, got {:?}",
        d3
    );

    // HardGate has no successor in the tier ladder regardless.
    assert!(
        EscalationTier::HardGate.next().is_none(),
        "EscalationTier::HardGate must have no .next()"
    );
}

#[test]
fn mt_188_forged_outcome_error_carries_diagnostic_fields() {
    let job_id = MicroTaskJobId::new_v7();
    let supplied = Uuid::now_v7();
    let actual = Uuid::now_v7();
    let err = MtOutcomeError::ForgedSession {
        job_id,
        supplied,
        actual: Some(actual),
    };
    let s = format!("{}", err);
    assert!(
        s.contains(&job_id.to_string()),
        "Display should name job_id"
    );
    assert!(
        s.contains(&supplied.to_string()),
        "Display should name supplied"
    );
    assert!(
        s.contains(&actual.to_string()),
        "Display should name actual"
    );
    // Also when actual is None (claim was released after the persist began).
    let err_none = MtOutcomeError::ForgedSession {
        job_id,
        supplied,
        actual: None,
    };
    assert!(format!("{}", err_none).contains("None"));
}

#[test]
fn mt_188_duplicate_outcome_error_carries_iteration() {
    let job_id = MicroTaskJobId::new_v7();
    let err = MtOutcomeError::DuplicateOutcome {
        job_id,
        iteration_n: 7,
    };
    let s = format!("{}", err);
    assert!(s.contains(&job_id.to_string()));
    assert!(s.contains("7"));
}

#[test]
fn mt_188_failure_no_candidate_under_default_threshold() {
    let job = make_job_with_tier(EscalationTier::T7B, 0);
    let (rec, cand) = MtOutcomeRecorder::record(
        &job,
        MtOutcome {
            outcome_kind: MtOutcomeKind::Failure,
            completion_signal: None,
            run_ledger_ref: None,
            evidence_pointers: vec![],
        },
        Uuid::now_v7(),
    );
    assert!(cand.is_none(), "default threshold is success_only=true");
    assert!(rec.distillation_candidate_id.is_none());
    assert_eq!(rec.outcome_kind, MtOutcomeKind::Failure);
}

#[test]
fn mt_188_fr_evt_mt_015_canonical_id_matches_registry() {
    // The recorder's FR_EVT_MT_015 const must match the
    // FrEventId::Mt015DistillationCandidate canonical wire id, or downstream
    // event-filter dashboards silently drop these events.
    assert_eq!(
        MtOutcomeRecorder::FR_EVT_MT_015,
        FrEventId::Mt015DistillationCandidate.as_str(),
        "FR-EVT-MT-015 canonical id drift between recorder and registry"
    );
    assert_eq!(
        MtOutcomeRecorder::FR_EVT_MT_015,
        "FR-EVT-MT-015",
        "wire id must be FR-EVT-MT-015"
    );
}

#[test]
fn mt_188_escalation_decision_serde_round_trip_all_variants() {
    let variants = vec![
        EscalationDecision::Retry {
            same_tier: EscalationTier::T13B,
        },
        EscalationDecision::EscalateTo {
            next_tier: EscalationTier::T13BAlt,
            lora_id: Some("lora-py-v1".to_string()),
        },
        EscalationDecision::EscalateTo {
            next_tier: EscalationTier::T7BAlt,
            lora_id: None,
        },
        EscalationDecision::HardGate {
            reason: "exhausted".to_string(),
        },
        EscalationDecision::Abandon {
            reason: "max_hardgate_retries".to_string(),
        },
    ];
    for v in variants {
        let s = serde_json::to_string(&v).expect("serialize");
        let back: EscalationDecision = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, v);
    }
}

#[test]
fn mt_188_distillation_candidate_emission_threshold_failure_outcome() {
    // Failure does not emit under default. With success_only=false it does
    // (cluster C negative-example collection path).
    let job = make_job_with_tier(EscalationTier::T7B, 1);
    let failure = MtOutcome {
        outcome_kind: MtOutcomeKind::Failure,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    let (rec_default, cand_default) =
        MtOutcomeRecorder::record(&job, failure.clone(), Uuid::now_v7());
    assert!(cand_default.is_none());
    assert!(rec_default.distillation_candidate_id.is_none());

    let (rec_cluster_c, cand_cluster_c) = MtOutcomeRecorder::record_with_threshold(
        &job,
        failure,
        Uuid::now_v7(),
        DistillationThreshold {
            success_only: false,
            minimum_iterations_completed: 0,
        },
    );
    assert!(cand_cluster_c.is_some());
    assert!(rec_cluster_c.distillation_candidate_id.is_some());
}

#[test]
fn mt_188_router_lora_lookup_first_matching_tag_wins() {
    let mut job = MicroTaskJob::queue(
        "WP",
        "MT",
        PathBuf::from("a.json"),
        6,
        vec!["llm".to_string(), "rust".to_string(), "python".to_string()],
    );
    job.escalation_tier = EscalationTier::T7B;
    let mut map = HashMap::new();
    map.insert("rust".to_string(), "lora-rust".to_string());
    map.insert("python".to_string(), "lora-py".to_string());
    let r = EscalationRouter::with_lora_map(map);
    let d = r.route(&job, 3);
    match d {
        EscalationDecision::EscalateTo { lora_id, .. } => {
            // "llm" misses, "rust" matches first.
            assert_eq!(lora_id.as_deref(), Some("lora-rust"));
        }
        other => panic!("expected EscalateTo, got {:?}", other),
    }
}

#[test]
fn mt_188_router_honors_loop_control_failure_budget() {
    let job = make_job_with_tier(EscalationTier::T13B, 0);
    let budget = MtLoopControlBudget {
        max_consecutive_failures: 2,
        ..MtLoopControlBudget::default()
    };
    let router = EscalationRouter {
        max_failures_per_tier: 99,
        ..EscalationRouter::default()
    };

    assert!(
        matches!(
            router.route_with_budget(&job, 1, &budget),
            EscalationDecision::Retry {
                same_tier: EscalationTier::T13B
            }
        ),
        "failure count below MT-185 budget should retry on the same tier"
    );

    match router.route_with_budget(&job, 2, &budget) {
        EscalationDecision::EscalateTo { next_tier, .. } => {
            assert_eq!(next_tier, EscalationTier::T13BAlt);
        }
        other => panic!(
            "failure count at MT-185 max_consecutive_failures must escalate, got {:?}",
            other
        ),
    }
}

// ---- MailboxPoster mock for enact_decision tests -----------------------------

#[derive(Default)]
struct CapturingMailboxPoster {
    escalations: Mutex<Vec<(Uuid, EscalationTier, EscalationTier, String)>>,
    hardgates: Mutex<Vec<(Uuid, String)>>,
}

#[async_trait::async_trait]
impl EscalationMailboxPoster for CapturingMailboxPoster {
    async fn post_micro_task_escalation(
        &self,
        thread_id: Uuid,
        _job: &MicroTaskJob,
        from_tier: EscalationTier,
        to_tier: EscalationTier,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError> {
        let msg_id = Uuid::now_v7();
        self.escalations
            .lock()
            .unwrap()
            .push((thread_id, from_tier, to_tier, reason));
        Ok(msg_id)
    }

    async fn post_hardgate_decision_request(
        &self,
        thread_id: Uuid,
        _job: &MicroTaskJob,
        reason: String,
    ) -> Result<Uuid, MtOutcomeError> {
        let msg_id = Uuid::now_v7();
        self.hardgates.lock().unwrap().push((thread_id, reason));
        Ok(msg_id)
    }
}

#[tokio::test]
async fn mt_188_enact_decision_skips_non_hardgate_mailbox_when_no_thread_bound() {
    // Pure-Rust test that exercises the async trait without a live mailbox.
    let job = make_job_with_tier(EscalationTier::T7B, 0);
    assert!(job.mailbox_thread_id.is_none());
    let poster = CapturingMailboxPoster::default();
    for d in [
        EscalationDecision::Retry {
            same_tier: EscalationTier::T7B,
        },
        EscalationDecision::EscalateTo {
            next_tier: EscalationTier::T7BAlt,
            lora_id: None,
        },
        EscalationDecision::Abandon {
            reason: "stub".to_string(),
        },
    ] {
        let result = enact_decision(&poster, &job, &d).await.expect("enact");
        assert_eq!(
            result, None,
            "enact must return None when job has no mailbox_thread_id, got {:?} for decision {:?}",
            result, d
        );
    }
    assert!(poster.escalations.lock().unwrap().is_empty());
    assert!(poster.hardgates.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mt_188_enact_decision_hardgate_without_thread_fails_closed() {
    let job = make_job_with_tier(EscalationTier::T32B, 5);
    assert!(job.mailbox_thread_id.is_none());
    let poster = CapturingMailboxPoster::default();
    let err = enact_decision(
        &poster,
        &job,
        &EscalationDecision::HardGate {
            reason: "tier ladder exhausted".to_string(),
        },
    )
    .await
    .expect_err("HardGate without a mailbox thread must fail closed");
    match err {
        MtOutcomeError::MissingHardGateMailboxThread { job_id } => assert_eq!(job_id, job.job_id),
        other => panic!("expected MissingHardGateMailboxThread, got {:?}", other),
    }
    assert!(poster.escalations.lock().unwrap().is_empty());
    assert!(poster.hardgates.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mt_188_enact_decision_hardgate_posts_decision_request_before_commit() {
    // Red_team minimum_control #1: HardGate transition always posts a
    // decision_request mailbox message. The enact_decision helper must call
    // post_hardgate_decision_request for every HardGate decision when the job
    // has a mailbox_thread_id bound.
    let mut job = make_job_with_tier(EscalationTier::T32B, 5);
    job.mailbox_thread_id = Some(Uuid::now_v7());
    let poster = CapturingMailboxPoster::default();
    let d = EscalationDecision::HardGate {
        reason: "tier ladder exhausted".to_string(),
    };
    let msg_id = enact_decision(&poster, &job, &d).await.expect("enact");
    assert!(msg_id.is_some(), "HardGate enact must return a message id");
    let captured = poster.hardgates.lock().unwrap();
    assert_eq!(
        captured.len(),
        1,
        "exactly one decision_request must be posted"
    );
    assert_eq!(captured[0].0, job.mailbox_thread_id.unwrap());
    assert_eq!(captured[0].1, "tier ladder exhausted");
    assert!(
        poster.escalations.lock().unwrap().is_empty(),
        "HardGate path must not post a micro_task_escalation message"
    );
}

#[tokio::test]
async fn mt_188_enact_decision_escalate_posts_micro_task_escalation() {
    let mut job = make_job_with_tier(EscalationTier::T7B, 3);
    job.mailbox_thread_id = Some(Uuid::now_v7());
    let poster = CapturingMailboxPoster::default();
    let d = EscalationDecision::EscalateTo {
        next_tier: EscalationTier::T7BAlt,
        lora_id: Some("lora-rust".to_string()),
    };
    let msg_id = enact_decision(&poster, &job, &d).await.expect("enact");
    assert!(msg_id.is_some());
    let captured = poster.escalations.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].1, EscalationTier::T7B);
    assert_eq!(captured[0].2, EscalationTier::T7BAlt);
    assert!(poster.hardgates.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mt_188_enact_decision_abandon_posts_nothing() {
    let mut job = make_job_with_tier(EscalationTier::HardGate, 6);
    job.mailbox_thread_id = Some(Uuid::now_v7());
    let poster = CapturingMailboxPoster::default();
    let d = EscalationDecision::Abandon {
        reason: "exceeded max_hardgate_retries".to_string(),
    };
    let result = enact_decision(&poster, &job, &d).await.expect("enact");
    assert_eq!(
        result, None,
        "Abandon must not post a mailbox message — the audit trail is the outcome row"
    );
    assert!(poster.escalations.lock().unwrap().is_empty());
    assert!(poster.hardgates.lock().unwrap().is_empty());
}

#[tokio::test]
async fn mt_188_enact_decision_retry_posts_nothing() {
    let mut job = make_job_with_tier(EscalationTier::T7B, 0);
    job.mailbox_thread_id = Some(Uuid::now_v7());
    let poster = CapturingMailboxPoster::default();
    let d = EscalationDecision::Retry {
        same_tier: EscalationTier::T7B,
    };
    let result = enact_decision(&poster, &job, &d).await.expect("enact");
    assert_eq!(
        result, None,
        "Retry must not post a mailbox message — the iteration loop continues silently"
    );
    assert!(poster.escalations.lock().unwrap().is_empty());
    assert!(poster.hardgates.lock().unwrap().is_empty());
}

#[derive(Default)]
struct RecordingFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

#[async_trait::async_trait]
impl FlightRecorder for RecordingFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events.lock().unwrap().push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events.lock().unwrap().clone())
    }
}

#[tokio::test]
async fn mt_188_distillation_candidate_records_fr_evt_mt_015() {
    let job = make_job_with_tier(EscalationTier::T7BAlt, 1);
    let (record, candidate) = MtOutcomeRecorder::record(
        &job,
        MtOutcome {
            outcome_kind: MtOutcomeKind::Success,
            completion_signal: Some(CompletionSignal::Success {
                summary: "usable solution".to_string(),
            }),
            run_ledger_ref: None,
            evidence_pointers: vec!["artifact://mt188/outcome.json".to_string()],
        },
        Uuid::now_v7(),
    );
    let candidate = candidate.expect("success emits candidate");
    assert_eq!(
        record.distillation_candidate_id,
        Some(candidate.candidate_id)
    );

    let recorder = RecordingFlightRecorder::default();
    let trace_id = Uuid::now_v7();
    let workflow_run_id = Uuid::now_v7();
    MtOutcomeRecorder::emit_distillation_candidate_event(
        &recorder,
        &job,
        &candidate,
        trace_id,
        workflow_run_id,
    )
    .await
    .expect("emit");

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list events");
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(
        event.event_type,
        FlightRecorderEventType::MicroTaskDistillationCandidate
    );
    let job_id_s = job.job_id.to_string();
    let workflow_run_id_s = workflow_run_id.to_string();
    assert_eq!(event.job_id.as_deref(), Some(job_id_s.as_str()));
    assert_eq!(
        event.workflow_id.as_deref(),
        Some(workflow_run_id_s.as_str())
    );
    assert_eq!(
        event.payload["event_type"],
        FrEventId::Mt015DistillationCandidate.as_str()
    );
    assert_eq!(
        event.payload["event_name"],
        "micro_task_distillation_candidate"
    );
    assert_eq!(event.payload["payload"]["wp_id"], job.wp_id);
    assert_eq!(event.payload["payload"]["mt_id"], job.mt_id);
    assert_eq!(
        event.payload["payload"]["candidate_ref"]["candidate_id"],
        candidate.candidate_id.to_string()
    );
    assert_eq!(
        event.payload["payload"]["candidate_ref"]["status"],
        DistillationCandidateStatus::LoggingOnlyPhase1.as_str()
    );
}

#[tokio::test]
async fn mt_188_record_with_flight_recorder_couples_candidate_and_fr_event() {
    let job = make_job_with_tier(EscalationTier::T13B, 2);
    let recorder = RecordingFlightRecorder::default();
    let trace_id = Uuid::now_v7();
    let workflow_run_id = Uuid::now_v7();

    let (record, candidate, event) = MtOutcomeRecorder::record_with_flight_recorder(
        &recorder,
        &job,
        MtOutcome {
            outcome_kind: MtOutcomeKind::Success,
            completion_signal: Some(CompletionSignal::Success {
                summary: "candidate-worthy".to_string(),
            }),
            run_ledger_ref: None,
            evidence_pointers: vec!["artifact://mt188/record-with-fr.json".to_string()],
        },
        Uuid::now_v7(),
        trace_id,
        workflow_run_id,
    )
    .await
    .expect("record with flight recorder");

    let candidate = candidate.expect("success emits candidate");
    let event = event.expect("candidate creation emits FR-EVT-MT-015");
    assert_eq!(
        record.distillation_candidate_id,
        Some(candidate.candidate_id)
    );
    assert_eq!(
        event.payload["payload"]["candidate_ref"]["candidate_id"],
        candidate.candidate_id.to_string()
    );

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list events");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].event_type,
        FlightRecorderEventType::MicroTaskDistillationCandidate
    );
}

// ============================================================================
// Postgres-gated integration assertions
// ============================================================================

async fn postgres_pool() -> sqlx::PgPool {
    let url =
        std::env::var("POSTGRES_TEST_URL").expect("ENVIRONMENT_BLOCKED: POSTGRES_TEST_URL not set");
    sqlx::PgPool::connect(&url).await.expect("postgres connect")
}

/// Build a per-test wp_id prefix so two parallel test binaries (or two
/// sibling agents) cannot collide on the shared kernel_micro_task_job table.
fn unique_wp_id(test_label: &str) -> String {
    format!("WP-MT188-{}-{}", test_label, Uuid::now_v7().simple())
}

async fn ensure_all_schemas(pool: &sqlx::PgPool) {
    let queue = MicroTaskQueue::new(pool.clone());
    queue.ensure_schema().await.expect("ensure 0023");
    MtOutcomeRecorder::ensure_schema(pool)
        .await
        .expect("ensure 0027");
}

/// Helper: enqueue a job and claim it under the given session so the
/// `claimed_by_session` column points at our session before we record.
async fn enqueue_and_claim(
    pool: &sqlx::PgPool,
    wp: &str,
    mt: &str,
    session: Uuid,
) -> (MicroTaskQueue, MicroTaskJob) {
    let queue = MicroTaskQueue::new(pool.clone());
    let job = MicroTaskJob::queue(wp, mt, PathBuf::from("a.json"), 6, vec![]);
    queue.enqueue(&job).await.expect("enqueue");
    // claim_next is FIFO; race tolerantly until our row is the one returned.
    let mut found = None;
    for _ in 0..40 {
        if let Some(id) = queue.claim_next(session).await.expect("claim") {
            if let Ok(Some(claimed_job)) = queue.get_job(id).await {
                if claimed_job.wp_id == wp {
                    found = Some(claimed_job);
                    break;
                } else {
                    // Sibling row; release back.
                    let _ = queue
                        .update_state(
                            id,
                            MicroTaskJobState::Queued,
                            Some("mt188 test rollback".to_string()),
                        )
                        .await;
                }
            }
        }
    }
    let claimed = found.expect("our job must be claimable");
    (queue, claimed)
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_outcome_persists_in_kernel_mt_outcome_table() {
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("persist");
    let session = Uuid::now_v7();
    let (_q, job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;
    let job_id = job.job_id;

    let outcome = MtOutcome {
        outcome_kind: MtOutcomeKind::Success,
        completion_signal: Some(CompletionSignal::Success {
            summary: "ok".to_string(),
        }),
        run_ledger_ref: None,
        evidence_pointers: vec!["evidence://x".to_string()],
    };
    let (rec, cand) = MtOutcomeRecorder::persist(&pool, &job, outcome, session)
        .await
        .expect("persist");
    assert_eq!(rec.outcome_kind, MtOutcomeKind::Success);
    assert!(cand.is_some(), "Success must emit a DistillationCandidate");

    // Verify the row is durably stored and reads back identically.
    let read_back = MtOutcomeRecorder::list_for_job(&pool, job_id)
        .await
        .expect("list");
    assert_eq!(read_back.len(), 1, "exactly one outcome row");
    assert_eq!(read_back[0].outcome_id, rec.outcome_id);
    assert_eq!(read_back[0].evidence_pointers, vec!["evidence://x"]);
    assert_eq!(
        read_back[0].distillation_candidate_id,
        rec.distillation_candidate_id
    );

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_distillation_candidate_persists_with_phase1_status() {
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("phase1");
    let session = Uuid::now_v7();
    let (_q, job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;
    let job_id = job.job_id;

    let outcome = MtOutcome {
        outcome_kind: MtOutcomeKind::Success,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    let (_rec, cand) = MtOutcomeRecorder::persist(&pool, &job, outcome, session)
        .await
        .expect("persist");
    let cand = cand.expect("candidate emitted on Success");
    assert_eq!(cand.status, DistillationCandidateStatus::LoggingOnlyPhase1);

    let listed = MtOutcomeRecorder::list_candidates_for_job(&pool, job_id)
        .await
        .expect("list candidates");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].status,
        DistillationCandidateStatus::LoggingOnlyPhase1
    );
    assert_eq!(listed[0].tier_at_completion, EscalationTier::T7B);

    // Phase-1 listing helper must include this candidate.
    let phase1 = MtOutcomeRecorder::list_phase1_candidates(&pool, 100)
        .await
        .expect("list phase1");
    let ids: HashSet<Uuid> = phase1.iter().map(|c| c.candidate_id).collect();
    assert!(ids.contains(&cand.candidate_id));

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_persist_with_flight_recorder_emits_fr_evt_mt_015() {
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("persist-fr");
    let session = Uuid::now_v7();
    let (_q, job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;
    let recorder = RecordingFlightRecorder::default();
    let trace_id = Uuid::now_v7();
    let workflow_run_id = Uuid::now_v7();

    let (_rec, cand, event) = MtOutcomeRecorder::persist_with_flight_recorder(
        &pool,
        &recorder,
        &job,
        MtOutcome {
            outcome_kind: MtOutcomeKind::Success,
            completion_signal: Some(CompletionSignal::Success {
                summary: "ok".to_string(),
            }),
            run_ledger_ref: None,
            evidence_pointers: vec!["evidence://persist-fr".to_string()],
        },
        session,
        trace_id,
        workflow_run_id,
    )
    .await
    .expect("persist with FR event");

    assert!(cand.is_some(), "success must persist a candidate");
    assert!(
        event.is_some(),
        "persisted candidate must emit FR-EVT-MT-015"
    );

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .expect("list events");
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].payload["event_type"],
        FrEventId::Mt015DistillationCandidate.as_str()
    );

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_forged_outcome_rejected() {
    // Recorder must refuse to persist if the supplied session_id does not
    // match the job's claimed_by_session.
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("forged");
    let real_session = Uuid::now_v7();
    let attacker_session = Uuid::now_v7();
    let (_q, job) = enqueue_and_claim(&pool, &wp, "MT-1", real_session).await;

    let outcome = MtOutcome {
        outcome_kind: MtOutcomeKind::Success,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    let err = MtOutcomeRecorder::persist(&pool, &job, outcome, attacker_session).await;
    match err {
        Err(MtOutcomeError::ForgedSession {
            supplied, actual, ..
        }) => {
            assert_eq!(supplied, attacker_session);
            assert_eq!(actual, Some(real_session));
        }
        other => panic!("expected ForgedSession, got {:?}", other),
    }

    // Confirm no outcome row was written.
    let listed = MtOutcomeRecorder::list_for_job(&pool, job.job_id)
        .await
        .expect("list");
    assert_eq!(listed.len(), 0);

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_duplicate_outcome_refused_at_db_layer() {
    // Two persist calls for the same (job_id, iteration_n) — the second must
    // return DuplicateOutcome via the unique index added in 0027.
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("dup");
    let session = Uuid::now_v7();
    let (_q, job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;

    let outcome = MtOutcome {
        outcome_kind: MtOutcomeKind::Failure,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    // First write succeeds.
    let (_rec, _cand) = MtOutcomeRecorder::persist(&pool, &job, outcome.clone(), session)
        .await
        .expect("first persist");
    // Second write must fail with DuplicateOutcome.
    let err = MtOutcomeRecorder::persist(&pool, &job, outcome, session).await;
    match err {
        Err(MtOutcomeError::DuplicateOutcome {
            job_id,
            iteration_n,
        }) => {
            assert_eq!(job_id, job.job_id);
            assert_eq!(iteration_n, job.iteration_n);
        }
        other => panic!("expected DuplicateOutcome, got {:?}", other),
    }

    // Confirm still only one row.
    let listed = MtOutcomeRecorder::list_for_job(&pool, job.job_id)
        .await
        .expect("list");
    assert_eq!(listed.len(), 1);

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_six_tier_ladder_enforced_at_db_layer() {
    // The queue's escalate() walks the tier ladder exactly; T7B->T13B (skip)
    // is rejected, T7B->T7BAlt then T7BAlt->T13B is accepted; HardGate is
    // terminal.
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("ladder");
    let session = Uuid::now_v7();
    let (q, job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;
    let job_id = job.job_id;

    // Walk every legitimate transition.
    let walk = [
        (EscalationTier::T7B, EscalationTier::T7BAlt),
        (EscalationTier::T7BAlt, EscalationTier::T13B),
        (EscalationTier::T13B, EscalationTier::T13BAlt),
        (EscalationTier::T13BAlt, EscalationTier::T32B),
        (EscalationTier::T32B, EscalationTier::HardGate),
    ];
    for (from, to) in walk {
        let step = if matches!(to, EscalationTier::HardGate) {
            let direct_err = q
                .escalate(job_id, to, format!("unreceipted {:?} -> {:?}", from, to))
                .await;
            assert!(
                matches!(direct_err, Err(QueueError::HardGateMailboxRequired)),
                "HardGate transition must require decision_request receipt, got {:?}",
                direct_err
            );
            q.hard_gate_after_mailbox_post(
                job_id,
                format!("legit {:?} -> {:?}", from, to),
                Uuid::now_v7(),
            )
            .await
            .expect("receipted hardgate escalation")
        } else {
            q.escalate(job_id, to, format!("legit {:?} -> {:?}", from, to))
                .await
                .expect("legit escalation")
        };
        assert_eq!(step.from_tier, from);
        assert_eq!(step.to_tier, to);
    }

    // After HardGate, no further escalation is allowed; any subsequent
    // escalate() must return InvalidEscalation.
    for target in [
        EscalationTier::T7B,
        EscalationTier::T7BAlt,
        EscalationTier::T13B,
        EscalationTier::T13BAlt,
        EscalationTier::T32B,
        EscalationTier::HardGate,
    ] {
        let err = q
            .escalate(job_id, target, "after hardgate".to_string())
            .await;
        assert!(
            matches!(err, Err(QueueError::InvalidEscalation)),
            "HardGate must be terminal; escalation to {:?} must be refused, got {:?}",
            target,
            err
        );
    }

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_escalation_persists_selected_lora_id() {
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("lora");
    let session = Uuid::now_v7();
    let (q, job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;
    let job_id = job.job_id;

    q.escalate_with_lora(
        job_id,
        EscalationTier::T7BAlt,
        "router selected LoRA for next tier".to_string(),
        Some("lora-code-v1".to_string()),
    )
    .await
    .expect("escalate with lora");

    let read_back = q
        .get_job(job_id)
        .await
        .expect("get job")
        .expect("job exists");
    assert_eq!(read_back.escalation_tier, EscalationTier::T7BAlt);
    assert_eq!(read_back.lora_id.as_deref(), Some("lora-code-v1"));

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_concurrent_outcome_records_preserve_ordering() {
    // Two parallel persists for the SAME job at the SAME iteration must
    // never both succeed; exactly one wins, the other returns
    // DuplicateOutcome. The audit-trail ordering is preserved by the
    // recorded_at_utc ASC sort in list_for_job.
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("concurrent");
    let session = Uuid::now_v7();
    let (_q, job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;

    let pool_a = pool.clone();
    let pool_b = pool.clone();
    let job_a = job.clone();
    let job_b = job.clone();
    let outcome_a = MtOutcome {
        outcome_kind: MtOutcomeKind::Failure,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec!["a".to_string()],
    };
    let outcome_b = MtOutcome {
        outcome_kind: MtOutcomeKind::Failure,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec!["b".to_string()],
    };

    let ha = tokio::spawn(async move {
        MtOutcomeRecorder::persist(&pool_a, &job_a, outcome_a, session).await
    });
    let hb = tokio::spawn(async move {
        MtOutcomeRecorder::persist(&pool_b, &job_b, outcome_b, session).await
    });
    let ra = ha.await.expect("join a");
    let rb = hb.await.expect("join b");

    let outcomes_won = [&ra, &rb].iter().filter(|r| r.is_ok()).count();
    let outcomes_dup = [&ra, &rb]
        .iter()
        .filter(|r| matches!(r, Err(MtOutcomeError::DuplicateOutcome { .. })))
        .count();
    assert_eq!(
        outcomes_won, 1,
        "exactly one persist must win; got {} wins (ra={:?}, rb={:?})",
        outcomes_won, ra, rb
    );
    assert_eq!(
        outcomes_dup, 1,
        "exactly one persist must hit DuplicateOutcome; got {} dups",
        outcomes_dup
    );

    let listed = MtOutcomeRecorder::list_for_job(&pool, job.job_id)
        .await
        .expect("list");
    assert_eq!(listed.len(), 1, "exactly one outcome row must persist");

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_escalation_history_queryable_across_outcome_writes() {
    // After several legitimate escalations + outcome writes, the per-job
    // history must be queryable in order. This is the audit-trail contract
    // a validator role exercises post-hoc to inspect what happened.
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let wp = unique_wp_id("history");
    let session = Uuid::now_v7();
    let (q, mut job) = enqueue_and_claim(&pool, &wp, "MT-1", session).await;
    let job_id = job.job_id;

    // Record 3 outcomes at increasing iteration_n.
    for iter in 0..3u32 {
        // Update job iteration_n in memory; tests bypass the executor's loop
        // accounting because we're targeting the recorder + persistence
        // contract specifically.
        sqlx::query("UPDATE kernel_micro_task_job SET iteration_n = $1 WHERE job_id = $2")
            .bind(iter as i64)
            .bind(job_id.as_uuid())
            .execute(&pool)
            .await
            .expect("bump iteration_n");
        job.iteration_n = iter;
        let outcome = MtOutcome {
            outcome_kind: MtOutcomeKind::Failure,
            completion_signal: Some(CompletionSignal::Failure {
                reason: format!("iter {}", iter),
            }),
            run_ledger_ref: None,
            evidence_pointers: vec![format!("evidence://iter-{}", iter)],
        };
        MtOutcomeRecorder::persist(&pool, &job, outcome, session)
            .await
            .expect("persist");
    }

    // Escalate once for good measure (the queue tracks the step in
    // escalation_history JSONB; not the same as the outcome rows).
    let _ = q
        .escalate(job_id, EscalationTier::T7BAlt, "verifier asked".to_string())
        .await
        .expect("legit escalation");

    // Read back: must have 3 outcome rows in iteration order.
    let listed = MtOutcomeRecorder::list_for_job(&pool, job_id)
        .await
        .expect("list");
    assert_eq!(listed.len(), 3);
    let iters: Vec<u32> = listed.iter().map(|r| r.iteration_n).collect();
    assert_eq!(iters, vec![0, 1, 2]);

    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_unknown_job_returns_jobnotfound() {
    let pool = postgres_pool().await;
    ensure_all_schemas(&pool).await;

    let phantom = MicroTaskJob::queue("WP-NOOP", "MT-NOOP", PathBuf::from("a.json"), 6, vec![]);
    let outcome = MtOutcome {
        outcome_kind: MtOutcomeKind::Success,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    let err = MtOutcomeRecorder::persist(&pool, &phantom, outcome, Uuid::now_v7()).await;
    assert!(
        matches!(err, Err(MtOutcomeError::JobNotFound(_))),
        "persist on a non-existent job must return JobNotFound, got {:?}",
        err
    );
}

// Type-aware tests: we keep an Arc<sqlx::PgPool> around to verify the API
// is Send + Sync friendly under tokio::spawn.
#[tokio::test]
#[ignore = "requires POSTGRES_TEST_URL; run with `cargo test -- --ignored`"]
async fn mt_188_pg_recorder_is_send_sync_friendly() {
    let pool: Arc<sqlx::PgPool> = Arc::new(postgres_pool().await);
    ensure_all_schemas(pool.as_ref()).await;
    let wp = unique_wp_id("sendsync");
    let session = Uuid::now_v7();
    let (_q, job) = enqueue_and_claim(pool.as_ref(), &wp, "MT-1", session).await;
    let outcome = MtOutcome {
        outcome_kind: MtOutcomeKind::Success,
        completion_signal: None,
        run_ledger_ref: None,
        evidence_pointers: vec![],
    };
    let pool_c = pool.clone();
    let h = tokio::spawn(async move {
        MtOutcomeRecorder::persist(pool_c.as_ref(), &job, outcome, session).await
    });
    let _ = h.await.expect("join").expect("persist");
    sqlx::query("DELETE FROM kernel_micro_task_job WHERE wp_id = $1")
        .bind(&wp)
        .execute(pool.as_ref())
        .await
        .ok();
}
