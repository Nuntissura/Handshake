//! MT-169: AC-DISTILL negative-path acceptance tests.

use std::sync::Mutex;

use chrono::{Duration, Utc};
use handshake_core::distillation::content_review::{
    ContentReview, ContentReviewConfig, QuarantineReason, ReviewVerdict, FR_EVT_DISTILL_PII_DETECT,
};
use handshake_core::distillation::corpus_extractor::TrainingTurn;
use handshake_core::distillation::spawn_tree_harvester::{
    ContentReviewVerdict, ContentReviewer, DistillationCandidateSubmission,
    DistillationCandidateSubmitter, EventLedgerReader, HarvestError, SpawnPair, SpawnTreeHarvester,
};
use handshake_core::memory::outcome_feedback::{CapsuleOutcome, FailureClass};
use handshake_core::memory::TaskType;
use handshake_core::self_improve::corpus::{
    CorpusError, CorpusItem, HbrTestPacketCorpus, KeyError, StaticKeyProvider, ValidatorVerdict,
};
use handshake_core::self_improve::editable_surface::{
    EditableSurfaceError, EditableSurfaceProvider, ForbidReason, ModelManualSurface,
    PolicyParameter, RetrievalPolicySurface,
};
use handshake_core::self_improve::evaluator::{EvalResult, SplitMetrics};
use handshake_core::self_improve::goodhart_sentinel::{
    GoodhartSentinel, PauseReason, SentinelDecision, SentinelEntry, SentinelHistory,
    FR_EVT_GOODHART_PAUSE,
};
use handshake_core::self_improve::iteration::LoopTarget;
use handshake_core::self_improve::promotion_floor::{
    FloorReason, MultiMetricPromotionFloor, PromotionDecision,
};
use handshake_core::self_improve::scheduler::{
    LoopScheduler, ScheduleDecision, SchedulerHistory, SchedulerHistoryEntry, SkipReason,
    FR_EVT_DISTILL_LOOP_CAP,
};
use serde_json::json;
use uuid::Uuid;

struct FixtureLedger {
    pairs: Vec<SpawnPair>,
    read_sessions: Mutex<Vec<Uuid>>,
}

impl EventLedgerReader for FixtureLedger {
    fn read_spawn_pairs(&self, session_id: Uuid) -> Result<Vec<SpawnPair>, HarvestError> {
        self.read_sessions.lock().unwrap().push(session_id);
        Ok(self
            .pairs
            .iter()
            .filter(|pair| pair.session_id == session_id)
            .cloned()
            .collect())
    }
}

struct PassReviewer;

impl ContentReviewer for PassReviewer {
    fn review_pair(&self, _pair: &SpawnPair) -> Result<ContentReviewVerdict, HarvestError> {
        Ok(ContentReviewVerdict::Pass {
            license_provenance: "custom_internal".to_string(),
        })
    }
}

struct CandidateSink {
    submissions: Mutex<Vec<DistillationCandidateSubmission>>,
}

impl DistillationCandidateSubmitter for CandidateSink {
    fn submit_candidate(
        &self,
        submission: DistillationCandidateSubmission,
    ) -> Result<Uuid, HarvestError> {
        let id = submission.candidate.candidate_id;
        self.submissions.lock().unwrap().push(submission);
        Ok(id)
    }
}

fn pass_pair(session_id: Uuid) -> SpawnPair {
    SpawnPair {
        session_id,
        parent_message: "candidate parent".to_string(),
        child_response: "candidate child".to_string(),
        parent_role: "orchestrator".to_string(),
        child_role: "coder".to_string(),
        parent_message_id: Uuid::now_v7(),
        child_message_id: Uuid::now_v7(),
        child_outcome: CapsuleOutcome::Pass {
            mt_id: "MT-169".to_string(),
            validator_verdict_id: Uuid::now_v7(),
        },
    }
}

fn fail_pair(session_id: Uuid) -> SpawnPair {
    SpawnPair {
        child_outcome: CapsuleOutcome::Fail {
            mt_id: "MT-169".to_string(),
            validator_verdict_id: Uuid::now_v7(),
            failure_class: FailureClass::ValidatorRejected,
        },
        ..pass_pair(session_id)
    }
}

#[test]
fn ac_distill_opt_in_default_off_and_non_transitive_across_sessions() {
    let session_a = Uuid::now_v7();
    let session_b = Uuid::now_v7();
    let ledger = FixtureLedger {
        pairs: vec![
            pass_pair(session_a),
            pass_pair(session_b),
            fail_pair(session_a),
        ],
        read_sessions: Mutex::new(Vec::new()),
    };
    let reviewer = PassReviewer;
    let sink = CandidateSink {
        submissions: Mutex::new(Vec::new()),
    };
    let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &sink);

    let err = harvester.harvest(session_a, false).unwrap_err();
    assert!(matches!(err, HarvestError::OptInRequired { .. }));
    assert!(ledger.read_sessions.lock().unwrap().is_empty());

    let candidates = harvester.harvest(session_a, true).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].session_id, session_a);
    assert_eq!(sink.submissions.lock().unwrap().len(), 1);

    let err = harvester.harvest(session_b, false).unwrap_err();
    assert!(matches!(err, HarvestError::OptInRequired { .. }));
    assert_eq!(
        sink.submissions.lock().unwrap().len(),
        1,
        "opt-in for session A must not transitively opt in session B"
    );
}

#[test]
fn ac_distill_content_review_blocks_pii_quarantines_license_and_dedups() {
    let pii: TrainingTurn = serde_json::from_str(include_str!(
        "fixtures/ac_distill_negative_path/pii_sample.json"
    ))
    .unwrap();
    let untaggable: TrainingTurn = serde_json::from_str(include_str!(
        "fixtures/ac_distill_negative_path/license_untaggable_sample.json"
    ))
    .unwrap();
    let near_dup_pair: Vec<TrainingTurn> = serde_json::from_str(include_str!(
        "fixtures/ac_distill_negative_path/near_dup_pair.json"
    ))
    .unwrap();

    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let pii_review = reviewer.review_with_events(&pii).unwrap();
    assert!(!matches!(pii_review.verdict, ReviewVerdict::Pass { .. }));
    assert!(pii_review
        .events
        .iter()
        .any(|event| event.event_kind == FR_EVT_DISTILL_PII_DETECT));

    let untaggable_review = reviewer.review_with_events(&untaggable).unwrap();
    match untaggable_review.verdict {
        ReviewVerdict::Quarantine { reasons, .. } => {
            assert!(reasons
                .iter()
                .any(|reason| matches!(reason, QuarantineReason::UntaggableLicense)));
        }
        other => panic!("expected untaggable license quarantine, got {other:?}"),
    }

    let first = reviewer.review_with_events(&near_dup_pair[0]).unwrap();
    assert!(matches!(first.verdict, ReviewVerdict::Pass { .. }));
    let second = reviewer.review_with_events(&near_dup_pair[1]).unwrap();
    match second.verdict {
        ReviewVerdict::Quarantine { reasons, .. } => {
            assert!(reasons.iter().any(|reason| {
                matches!(
                    reason,
                    QuarantineReason::NearDuplicateOfTurn {
                        existing_turn_id,
                        ..
                    } if existing_turn_id == &near_dup_pair[0].id
                )
            }));
        }
        other => panic!("expected near-duplicate quarantine, got {other:?}"),
    }
}

fn assert_loop_target_closed(target: &LoopTarget) -> &'static str {
    match target {
        LoopTarget::ModelManualCapsuleText { .. } => "model_manual_capsule_text",
        LoopTarget::RetrievalPolicyParams { .. } => "retrieval_policy_params",
    }
}

#[test]
fn ac_distill_editable_surface_forbidden_and_policy_bounds_fail_closed() {
    let manual = ModelManualSurface::new(
        |_section| Ok::<String, EditableSurfaceError>("UNREACHABLE".to_string()),
        |_section, _text| Ok::<(), EditableSurfaceError>(()),
    );
    let forbidden_targets = [
        (
            LoopTarget::ModelManualCapsuleText {
                manual_section_id: "spec.master.v02.186".to_string(),
            },
            ForbidReason::ShadowAuthority,
        ),
        (
            LoopTarget::ModelManualCapsuleText {
                manual_section_id: "role.orchestrator.system_prompt".to_string(),
            },
            ForbidReason::BlastRadiusTooWide,
        ),
        (
            LoopTarget::ModelManualCapsuleText {
                manual_section_id: "model.lora_weights.primary".to_string(),
            },
            ForbidReason::NoTrainingInfraInV0,
        ),
        (
            LoopTarget::ModelManualCapsuleText {
                manual_section_id: "tool_description.shell".to_string(),
            },
            ForbidReason::ToolDescriptionAuthority,
        ),
    ];
    for (target, expected_reason) in forbidden_targets {
        assert_eq!(
            assert_loop_target_closed(&target),
            "model_manual_capsule_text"
        );
        let err = manual.snapshot(&target).unwrap_err();
        assert!(matches!(
            err,
            EditableSurfaceError::Forbidden { reason, .. } if reason == expected_reason
        ));
    }

    let policy = RetrievalPolicySurface::new(
        |_task, _param| Ok::<u64, EditableSurfaceError>(0),
        |_task, _param, _value| Ok::<(), EditableSurfaceError>(()),
    );
    let top_k_target = LoopTarget::RetrievalPolicyParams {
        task_type: TaskType::ValidatorHbrTestPacket,
        parameter: PolicyParameter::TopK,
    };
    assert_eq!(
        assert_loop_target_closed(&top_k_target),
        "retrieval_policy_params"
    );
    let err = policy.snapshot(&top_k_target).unwrap_err();
    assert!(matches!(
        err,
        EditableSurfaceError::InvalidProposal { field: "top_k", .. }
    ));
}

fn corpus_item(n: u128, hbr: &str) -> CorpusItem {
    CorpusItem {
        id: Uuid::from_u128(0x7000_0000_0000_0000_0000_0000_0000_0000 + n),
        hbr_rule_id: hbr.to_string(),
        packet_under_test: format!("packet-{n}"),
        expected_first_pass_verdict: ValidatorVerdict::Pass,
        fixtures: json!({ "n": n }),
    }
}

fn sample_corpus() -> HbrTestPacketCorpus {
    let hbrs = [
        "HBR-INT-006",
        "HBR-INT-008",
        "HBR-SWARM-002",
        "HBR-VIS-001",
        "HBR-MAN-001",
    ];
    HbrTestPacketCorpus::from_items(
        (1..=30)
            .map(|n| corpus_item(n, hbrs[(n as usize - 1) % hbrs.len()]))
            .collect(),
    )
    .unwrap()
}

fn metrics(pass_rate: f64, latency_ms: u64, capsule_bytes: u64) -> SplitMetrics {
    SplitMetrics {
        pass_rate,
        pass_count: (pass_rate * 10.0).round() as u32,
        total_count: 10,
        latency_p95_ms: latency_ms,
        capsule_bytes_p95: capsule_bytes,
        per_item_results: Vec::new(),
    }
}

fn eval_result(dev_pass: f64, holdout_pass: f64) -> EvalResult {
    EvalResult {
        train: SplitMetrics::empty(),
        dev: metrics(dev_pass, 100, 10_000),
        holdout: metrics(holdout_pass, 100, 10_000),
        evaluated_at_utc: Utc::now(),
        snapshot_hash: "0".repeat(64),
    }
}

fn sentinel_entry(iteration: u32, gap: f64) -> SentinelEntry {
    SentinelEntry {
        iteration_number: iteration,
        dev_pass_rate: gap,
        holdout_pass_rate: 0.0,
        gap,
        accepted_at_utc: Utc::now(),
    }
}

#[test]
fn ac_distill_loop_safeguards_reject_bad_inputs() {
    let corpus = sample_corpus();
    let key_provider = StaticKeyProvider::deterministic("holdout-key");
    let split = corpus.split(42, &key_provider, "holdout-key").unwrap();
    let wrong_key_provider = StaticKeyProvider::deterministic("wrong-key");
    let decrypt_err = split.decrypt_holdout(&wrong_key_provider).unwrap_err();
    assert!(matches!(
        decrypt_err,
        CorpusError::KeyProvider(KeyError::UnknownKey { .. })
    ));

    let floor = MultiMetricPromotionFloor::with_defaults();
    let baseline = eval_result(0.70, 0.70);
    let trial = eval_result(0.70, 0.72);
    let decision = floor.evaluate(&baseline, &trial);
    match decision {
        PromotionDecision::Rejected { reasons, .. } => {
            assert!(reasons
                .iter()
                .any(|reason| matches!(reason, FloorReason::DevPassRateNotImproved { .. })));
        }
        other => panic!("expected promotion floor rejection, got {other:?}"),
    }

    let mut history = SentinelHistory::new();
    history.push(sentinel_entry(1, 0.05));
    history.push(sentinel_entry(2, 0.10));
    history.push(sentinel_entry(3, 0.15));
    let goodhart = GoodhartSentinel::evaluate(&history, &eval_result(0.75, 0.55));
    let pause_receipt = match goodhart {
        SentinelDecision::Pause { reason, receipt } => {
            assert_eq!(receipt.fr_event_kind, FR_EVT_GOODHART_PAUSE);
            assert!(matches!(reason, PauseReason::MonotonicGapWidening { .. }));
            receipt
        }
        other => panic!("expected Goodhart pause, got {other:?}"),
    };

    let now = Utc::now();
    let mut scheduler_history = SchedulerHistory::new();
    for i in 0..25 {
        scheduler_history.push(SchedulerHistoryEntry {
            iteration_id: Uuid::now_v7(),
            scheduled_at_utc: now - Duration::seconds(i),
        });
    }
    let scheduler = LoopScheduler::with_defaults();
    let decision = scheduler.try_schedule_next(&scheduler_history, now, None, None, None);
    match decision {
        ScheduleDecision::Skip {
            reason:
                SkipReason::BudgetExhausted {
                    count_in_window,
                    cap,
                    ..
                },
        } => {
            assert_eq!(count_in_window, 25);
            assert_eq!(cap, 25);
            let event = LoopScheduler::cap_event(decision.skip_reason().unwrap()).unwrap();
            assert_eq!(event.kind, FR_EVT_DISTILL_LOOP_CAP);
        }
        other => panic!("expected budget-exhausted skip on 26th request, got {other:?}"),
    }

    let paused = scheduler.try_schedule_next(
        &SchedulerHistory::new(),
        now,
        None,
        Some(&pause_receipt),
        None,
    );
    assert!(matches!(
        paused.skip_reason(),
        Some(SkipReason::GoodhartPause { .. })
    ));
}
