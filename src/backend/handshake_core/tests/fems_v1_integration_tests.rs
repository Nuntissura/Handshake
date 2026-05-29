//! Integration test suite for MT-157 through MT-165 (cluster D.3) + MT-166
//! (spawn tree harvester).
//!
//! Exercises the FEMS V1 feedback chain:
//! - Bitemporal indexing (MT-157)
//! - Outcome feedback loop (MT-158)
//! - Pinned core memory (MT-159)
//! - Hygiene manager job (MT-160)
//! - Injection scoring + progressive retrieval (MT-161)
//! - Calibration dashboard data feed (MT-162)
//! - Acceptance replay eval (MT-163)
//! - Retrieval-mode policy (MT-164)
//! - Retrieval trace bundle exporter (MT-165)
//! - Session-spawn distillation (MT-166)
//!
//! Spec-Realism Gate compliance:
//! - All paths execute real logic; no placeholders.
//! - Mocks compose real types end-to-end through the production trait
//!   surfaces.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use handshake_core::distillation::spawn_tree_harvester::{
    ContentReviewVerdict, ContentReviewer, DistillationCandidate, DistillationCandidateSubmission,
    DistillationCandidateSubmitter, EventLedgerReader, HarvestError, SpawnPair, SpawnTreeHarvester,
};
use handshake_core::memory::bitemporal::{
    AsOfQuery, BitemporalIndex, BitemporalItem, BitemporalStamps,
};
use handshake_core::memory::builder::{
    BuildContext, CapsuleBuilder, FemsError, FemsRetriever, RetrievedItem,
};
use handshake_core::memory::calibration::{
    CalibrationCollector, CalibrationError, CalibrationMetrics, CalibrationThresholds,
    FemsCalibrationSource, SignalStatus,
};
use handshake_core::memory::capsule::DegradationTier;
use handshake_core::memory::hygiene::{
    fingerprint_for_text, HygieneActionSubmitter, HygieneConfig, HygieneError, HygieneItemView,
    HygieneJobRunner, HygieneTask, InMemoryFemsAccessor, ProceduralPromotion,
};
use handshake_core::memory::outcome_feedback::{
    CapsuleOutcome, FailureClass, MemoryPackItemRef, OutcomeAttachSubmitter, OutcomeAttribution,
    OutcomeError, OutcomeFeedbackLoop, OutcomeReceipt, OutcomeScoringTuner, TuningParams,
    OUTCOME_ATTACH_ACTION_ID,
};
use handshake_core::memory::pinned_core::{PinnedBudget, PinnedCoreSelector};
use handshake_core::memory::policy_table::{CapsulePolicyTable, RETRIEVAL_SCORING_FORMULA_V0};
use handshake_core::memory::progressive_retrieval::{
    LoadSignal, ProgressiveRetriever, RetrievalError, RetrievalTier, RetrievedItem as PrItem,
    TIER_FULL_TEXT, TIER_GRAPH_EXPANSION, TIER_RERANK, TIER_VECTOR,
};
use handshake_core::memory::replay_eval::{
    BitemporalAccessor, CapsuleRecordReader, ReplayEvaluator, ReplayRequest,
};
use handshake_core::memory::retrieval_mode::{
    OperationClass, RetrievalContext, RetrievalMode, RetrievalModePolicy, RetrievalModeRouter,
};
use handshake_core::memory::scoring::{
    FormulaWeights, InjectionScoringFormula, ScoreInputs, INJECTION_SCORING_FORMULA_VERSION,
};
use handshake_core::memory::trace_export::{
    ArtifactRef, CacheMarkers, ExportFormat, MemoryPackBudgets, QueryPlan, RedactionPolicy,
    RetrievalTrace, RetrievalTraceExporter, RouteDecision, ScoringInputSnapshot, SpanRef,
    TraceBundle, TraceSource, TRACE_EXPORT_VERSION,
};
use handshake_core::memory::{CapsuleRecord, RetrievalPolicy, TaskType};
use uuid::Uuid;

fn at(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
}

fn retrieved(id: &str, score: f64, bytes: u64, pinned: bool) -> RetrievedItem {
    RetrievedItem {
        item_id: id.to_string(),
        memory_class: "test".to_string(),
        item_type: "doc".to_string(),
        summary: id.to_string(),
        content: id.to_string(),
        structured: None,
        trust_level: "trusted".to_string(),
        confidence: score,
        scope_refs: Vec::new(),
        source_refs: Vec::new(),
        score,
        score_breakdown: BTreeMap::new(),
        capsule_bytes: bytes,
        token_estimate: bytes as u32 / 4,
        pinned,
    }
}

#[test]
fn bitemporal_filters_items_independently_on_both_axes() {
    let mut idx = BitemporalIndex::new();
    idx.insert(BitemporalItem {
        item_id: Uuid::from_u128(1),
        stamps: BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        },
        payload: serde_json::json!({}),
    });
    let q1 = AsOfQuery {
        as_of_world_time: at(150),
        as_of_recorded_time: at(150),
    };
    assert_eq!(idx.items_visible_at(&q1).unwrap().len(), 1);
    idx.invalidate(Uuid::from_u128(1), at(200));
    let q2 = AsOfQuery {
        as_of_world_time: at(150),
        as_of_recorded_time: at(250),
    };
    assert_eq!(idx.items_visible_at(&q2).unwrap().len(), 0);
    let q3 = AsOfQuery {
        as_of_world_time: at(150),
        as_of_recorded_time: at(150),
    };
    assert_eq!(idx.items_visible_at(&q3).unwrap().len(), 1);
}

struct StubOutcomeSubmitter {
    items: Mutex<Vec<OutcomeAttribution>>,
}
impl OutcomeAttachSubmitter for StubOutcomeSubmitter {
    fn attach_outcome(
        &self,
        attribution: OutcomeAttribution,
    ) -> Result<OutcomeReceipt, OutcomeError> {
        self.items.lock().unwrap().push(attribution.clone());
        Ok(OutcomeReceipt {
            receipt_id: Uuid::now_v7(),
            capsule_id: attribution.capsule_id,
            action_id: OUTCOME_ATTACH_ACTION_ID.to_string(),
            recorded_at_utc: Utc::now(),
        })
    }
}

#[test]
fn outcome_feedback_attaches_and_tunes_pinned_items_skipped() {
    let submitter = StubOutcomeSubmitter {
        items: Mutex::new(Vec::new()),
    };
    let loop_ = OutcomeFeedbackLoop::new(&submitter);
    let capsule_id = Uuid::now_v7();
    let mt_id = "MT-test".to_string();
    let pack = vec![MemoryPackItemRef {
        memory_id: Uuid::from_u128(2),
        pinned: false,
    }];
    let mut attach_scores: HashMap<Uuid, f64> = HashMap::new();
    attach_scores.insert(pack[0].memory_id, 0.5);
    let receipt = loop_
        .record_outcome(
            capsule_id,
            CapsuleOutcome::Fail {
                mt_id,
                validator_verdict_id: Uuid::now_v7(),
                failure_class: FailureClass::ValidatorRejected,
            },
            &mut attach_scores,
            &pack,
            &TuningParams::default(),
        )
        .unwrap();
    assert_eq!(receipt.capsule_id, capsule_id);
    assert_eq!(submitter.items.lock().unwrap().len(), 1);
    assert!(attach_scores[&pack[0].memory_id] < 0.5);

    let mut scores: HashMap<Uuid, f64> = HashMap::new();
    let pack = vec![
        MemoryPackItemRef {
            memory_id: Uuid::from_u128(1),
            pinned: true,
        },
        MemoryPackItemRef {
            memory_id: Uuid::from_u128(2),
            pinned: false,
        },
    ];
    for it in &pack {
        scores.insert(it.memory_id, 0.5);
    }
    OutcomeScoringTuner::apply_outcome(
        &mut scores,
        &CapsuleOutcome::Fail {
            mt_id: "x".to_string(),
            validator_verdict_id: Uuid::now_v7(),
            failure_class: FailureClass::Other,
        },
        &pack,
        &TuningParams::default(),
    );
    assert!((scores[&pack[0].memory_id] - 0.5).abs() < 1e-9);
    assert!(scores[&pack[1].memory_id] < 0.5);
}

#[test]
fn pinned_core_selector_keeps_pinned_first() {
    let items = vec![
        retrieved("p1", 0.1, 1000, true),
        retrieved("u1", 0.9, 1000, false),
        retrieved("u2", 0.8, 1000, false),
    ];
    let sel = PinnedCoreSelector::select_pack_with_pins(
        &items,
        PinnedBudget {
            max_items: 5,
            max_bytes: 100_000,
        },
    )
    .unwrap();
    assert!(sel.ordered_items[0].pinned);
    assert_eq!(sel.ordered_items[0].item_id, "p1");
}

struct StubHygieneSubmitter {
    pub consolidations: Mutex<Vec<(Uuid, Uuid)>>,
    pub prunes: Mutex<Vec<Uuid>>,
    pub flags: Mutex<Vec<(Uuid, Uuid)>>,
    pub promotions: Mutex<Vec<ProceduralPromotion>>,
}
impl StubHygieneSubmitter {
    fn new() -> Self {
        Self {
            consolidations: Mutex::new(Vec::new()),
            prunes: Mutex::new(Vec::new()),
            flags: Mutex::new(Vec::new()),
            promotions: Mutex::new(Vec::new()),
        }
    }
}
impl HygieneActionSubmitter for StubHygieneSubmitter {
    fn submit_consolidation_candidate(&self, l: Uuid, r: Uuid) -> Result<Uuid, HygieneError> {
        self.consolidations.lock().unwrap().push((l, r));
        Ok(Uuid::now_v7())
    }
    fn submit_prune(&self, id: Uuid, _at: DateTime<Utc>) -> Result<Uuid, HygieneError> {
        self.prunes.lock().unwrap().push(id);
        Ok(Uuid::now_v7())
    }
    fn submit_contradiction_flag(&self, l: Uuid, r: Uuid) -> Result<Uuid, HygieneError> {
        self.flags.lock().unwrap().push((l, r));
        Ok(Uuid::now_v7())
    }
    fn submit_procedural_promotion(&self, p: ProceduralPromotion) -> Result<Uuid, HygieneError> {
        self.promotions.lock().unwrap().push(p);
        Ok(Uuid::now_v7())
    }
}

#[test]
fn hygiene_emits_consolidation_candidates_and_skips_pinned() {
    let submitter = StubHygieneSubmitter::new();
    let index = Mutex::new(BitemporalIndex::new());
    let fp = fingerprint_for_text("duplicate content");
    let stats = vec![
        HygieneItemView {
            memory_id: Uuid::from_u128(1),
            recorded_at: Utc::now() - chrono::Duration::seconds(86_400),
            score: 0.1,
            use_count: 1,
            pass_count: 1,
            pinned: true,
            content_fingerprint: fp,
            embedding: None,
        },
        HygieneItemView {
            memory_id: Uuid::from_u128(2),
            recorded_at: Utc::now() - chrono::Duration::seconds(86_400),
            score: 0.1,
            use_count: 1,
            pass_count: 1,
            pinned: false,
            content_fingerprint: fp,
            embedding: None,
        },
        HygieneItemView {
            memory_id: Uuid::from_u128(3),
            recorded_at: Utc::now() - chrono::Duration::seconds(86_400),
            score: 0.1,
            use_count: 1,
            pass_count: 1,
            pinned: false,
            content_fingerprint: fp,
            embedding: None,
        },
    ];
    let accessor = InMemoryFemsAccessor {
        index: &index,
        stats,
    };
    let runner = HygieneJobRunner::new(&accessor, &submitter);
    let cfg = HygieneConfig {
        tasks: vec![
            HygieneTask::Consolidate { max_pairs: 10 },
            HygieneTask::PruneStale {
                older_than_secs: 60,
                min_score: 0.5,
            },
        ],
    };
    let report = runner.run_once(cfg).unwrap();
    assert_eq!(report.tasks.len(), 2);
    // Consolidations are pair-based; with 1 pinned + 2 unpinned identical
    // we get one pair (2,3).
    assert_eq!(submitter.consolidations.lock().unwrap().len(), 1);
    // Prune skips pinned, so prunes = 2 (ids 2 and 3 satisfy old+low).
    assert_eq!(submitter.prunes.lock().unwrap().len(), 2);
}

#[test]
fn scoring_formula_is_deterministic_and_version_pinned() {
    assert_eq!(
        InjectionScoringFormula::version(),
        INJECTION_SCORING_FORMULA_VERSION
    );
    let inputs = ScoreInputs {
        importance: 0.5,
        recency_age_secs: 3600,
        trust: 0.8,
        outcome_weight: 0.6,
        embedding_similarity: 0.7,
    };
    let weights = FormulaWeights::default();
    let a = InjectionScoringFormula::score(&inputs, &weights);
    let b = InjectionScoringFormula::score(&inputs, &weights);
    assert_eq!(a, b);
}

struct ConstLoad(f64);
impl LoadSignal for ConstLoad {
    fn current_pressure(&self) -> f64 {
        self.0
    }
}
struct StubTier {
    name: &'static str,
    items: Vec<PrItem>,
}
impl RetrievalTier for StubTier {
    fn tier_name(&self) -> &'static str {
        self.name
    }
    fn execute(&self, _query: &str, _carry: &[PrItem]) -> Result<Vec<PrItem>, RetrievalError> {
        Ok(self.items.clone())
    }
}

#[test]
fn progressive_retrieval_degrades_under_load() {
    let ft = StubTier {
        name: TIER_FULL_TEXT,
        items: vec![PrItem {
            item_id: "ft-1".to_string(),
            score: 0.9,
            tier: TIER_FULL_TEXT.to_string(),
        }],
    };
    let v = StubTier {
        name: TIER_VECTOR,
        items: vec![PrItem {
            item_id: "v-1".to_string(),
            score: 0.8,
            tier: TIER_VECTOR.to_string(),
        }],
    };
    let g = StubTier {
        name: TIER_GRAPH_EXPANSION,
        items: vec![PrItem {
            item_id: "g-1".to_string(),
            score: 0.7,
            tier: TIER_GRAPH_EXPANSION.to_string(),
        }],
    };
    let r = StubTier {
        name: TIER_RERANK,
        items: vec![PrItem {
            item_id: "r-1".to_string(),
            score: 0.6,
            tier: TIER_RERANK.to_string(),
        }],
    };
    let retriever = ProgressiveRetriever::new(&ft, &v, &g, &r);
    let (_items, report) = retriever
        .retrieve_progressive("q", 10, DegradationTier::Tiered, &ConstLoad(0.95))
        .unwrap();
    assert!(report
        .tiers_skipped
        .contains(&TIER_GRAPH_EXPANSION.to_string()));
    assert!(report.tiers_skipped.contains(&TIER_RERANK.to_string()));
}

struct StubFemsSource;
impl FemsCalibrationSource for StubFemsSource {
    fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError> {
        Ok(CalibrationMetrics {
            total_items: 100,
            total_bytes: 16_384,
            bytes_growth_rate: 512.0,
            items_older_than_30d: 80,
            average_trust: 0.6,
            items_without_embedding: 25,
            recent_retrievals_total: 10,
            recent_retrievals_degraded: 4,
            trust_histogram_current: vec![10, 0, 0, 0, 0],
            trust_histogram_baseline: vec![0, 0, 0, 0, 10],
            last_hygiene_run_at: Some(Utc::now() - chrono::Duration::hours(100)),
            observed_at_utc: Utc::now(),
        })
    }
}

#[test]
fn calibration_signals_alert_when_metrics_exceed_thresholds() {
    let src = StubFemsSource;
    let snap =
        CalibrationCollector::collect_snapshot(&src, CalibrationThresholds::default()).unwrap();
    assert_eq!(snap.signals.stale_dominance.status, SignalStatus::Alert);
    assert_eq!(snap.signals.embedding_gap.status, SignalStatus::Alert);
    assert_eq!(snap.signals.degradation_rate.status, SignalStatus::Alert);
    assert_eq!(snap.signals.hygiene_lag.status, SignalStatus::Alert);
}

#[test]
fn replay_eval_succeeds_for_identical_capsule_record_and_bitemporal_state() {
    struct StubFems {
        items: Vec<RetrievedItem>,
    }
    impl FemsRetriever for StubFems {
        fn retrieve(&self, _query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
            Ok(self.items.iter().take(top_k as usize).cloned().collect())
        }
    }
    struct StubRecorder {
        record: CapsuleRecord,
    }
    impl CapsuleRecordReader for StubRecorder {
        fn read_record(
            &self,
            capsule_id: Uuid,
        ) -> Result<CapsuleRecord, handshake_core::memory::replay_eval::ReplayError> {
            if self.record.capsule_id == capsule_id {
                Ok(self.record.clone())
            } else {
                Err(
                    handshake_core::memory::replay_eval::ReplayError::CapsuleNotFound {
                        capsule_id,
                    },
                )
            }
        }
    }
    struct StubBi {
        items: Vec<BitemporalItem>,
    }
    impl BitemporalAccessor for StubBi {
        fn items_visible_at(
            &self,
            query: &AsOfQuery,
        ) -> Result<Vec<BitemporalItem>, handshake_core::memory::replay_eval::ReplayError> {
            Ok(self
                .items
                .iter()
                .filter(|item| item.stamps.visible_at(query))
                .cloned()
                .collect())
        }
    }

    let item_id = Uuid::from_u128(1);
    let item = retrieved(&item_id.to_string(), 0.5, 128, false);
    let policy = RetrievalPolicy {
        top_k: 4,
        capsule_budget_bytes: 2048,
        task_type: TaskType::GeneralRetrieval,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Strict,
    };
    let fems = StubFems {
        items: vec![item.clone()],
    };
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);
    let capsule = builder
        .build(BuildContext {
            task_type: TaskType::GeneralRetrieval,
            query: "replay smoke".to_string(),
            role_id: "KERNEL_BUILDER".to_string(),
            session_id: "session-1".to_string(),
            override_policy: Some(policy),
        })
        .unwrap();
    let record = CapsuleRecord::from_capsule(&capsule, Utc::now(), "session-1", "KERNEL_BUILDER");
    let replay_as_of = AsOfQuery {
        as_of_world_time: at(120),
        as_of_recorded_time: at(120),
    };
    let req = ReplayRequest {
        capsule_id: record.capsule_id,
        replay_as_of,
        expected_source_hash: record.capsule_source_hash.clone(),
    };
    let result = ReplayEvaluator::replay(
        req,
        &StubRecorder { record },
        &StubBi {
            items: vec![BitemporalItem {
                item_id,
                stamps: BitemporalStamps {
                    valid_from: at(100),
                    valid_until: None,
                    recorded_at: at(50),
                    invalidated_at: None,
                },
                payload: serde_json::to_value(item).unwrap(),
            }],
        },
        &builder,
    )
    .unwrap();

    assert!(result.replay_succeeded);
    assert!(result.replay_difference.is_empty());
}

#[test]
fn retrieval_mode_router_routes_typed_queries() {
    let policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&policy);

    let (mode_uuid, _) = router.route(&RetrievalContext {
        query: "01938b67-1234-7abc-89ef-0123456789ab".to_string(),
        task_type: TaskType::GeneralRetrieval,
        operation_class: OperationClass::QueryPlan,
    });
    assert_eq!(mode_uuid, RetrievalMode::NoRag);

    let (mode_wp, _) = router.route(&RetrievalContext {
        query: "load WP-KERNEL-004 packet".to_string(),
        task_type: TaskType::GeneralRetrieval,
        operation_class: OperationClass::WorkPacketLoad,
    });
    assert_eq!(mode_wp, RetrievalMode::AuthoritativeOnly);

    let (mode_hbr, _) = router.route(&RetrievalContext {
        query: "check HBR-INT-006".to_string(),
        task_type: TaskType::GeneralRetrieval,
        operation_class: OperationClass::QueryPlan,
    });
    assert_eq!(mode_hbr, RetrievalMode::AuthoritativeOnly);
}

struct StubTraceSource;
impl TraceSource for StubTraceSource {
    fn load_trace(
        &self,
        trace_id: Uuid,
    ) -> Result<TraceBundle, handshake_core::memory::trace_export::ExportError> {
        Ok(TraceBundle {
            trace_id,
            query_plan: QueryPlan {
                plan_id: Uuid::now_v7(),
                query: "find user@example.com".to_string(),
                task_type: "general".to_string(),
            },
            retrieval_trace: RetrievalTrace {
                trace_id,
                query_plan_id: Uuid::now_v7(),
                item_ids: Vec::new(),
            },
            budgets: MemoryPackBudgets {
                max_items: 10,
                max_bytes: 65_536,
            },
            cache_markers: CacheMarkers {
                cache_hit_count: 0,
                cache_miss_count: 1,
            },
            retrieval_mode: RetrievalMode::Hybrid,
            route_decision: RouteDecision {
                matched_rule_id: "default".to_string(),
                rationale: "fems_v1_integration_tests stub trace bundle uses default fallback rule"
                    .to_string(),
            },
            degradation_report: handshake_core::memory::progressive_retrieval::DegradationReport {
                tiers_completed: vec![TIER_FULL_TEXT.to_string()],
                tiers_skipped: Vec::new(),
                load_signal_at_start: 0.0,
                started_at_utc: Utc::now(),
                total_duration_ms: 0,
                tier_chosen: DegradationTier::Tiered,
            },
            selected_spans: vec![SpanRef {
                span_id: "s1".to_string(),
                text: "email user@example.com token=abc123".to_string(),
            }],
            referenced_artifacts: vec![ArtifactRef {
                artifact_id: "art-1".to_string(),
                kind: "packet.json".to_string(),
                bytes_estimate: 1024,
            }],
            scoring_inputs: vec![ScoringInputSnapshot {
                item_id: "s1".to_string(),
                importance: 0.5,
                recency: 0.5,
                trust: 0.5,
                outcome_tuned_weight: 0.5,
                embedding_similarity: 0.5,
                formula_version: "injection_scoring_formula_v1".to_string(),
            }],
            redactions_applied: Vec::new(),
            exported_at_utc: Utc::now(),
            exporter_version: TRACE_EXPORT_VERSION.to_string(),
        })
    }
}

#[test]
fn trace_export_redacts_pii_by_default() {
    let exporter = RetrievalTraceExporter::new(&StubTraceSource);
    let bundle = exporter
        .export(
            Uuid::now_v7(),
            &RedactionPolicy::default(),
            ExportFormat::Json,
        )
        .unwrap();
    let decoded: TraceBundle = serde_json::from_slice(&bundle.bytes).unwrap();
    let txt = &decoded.selected_spans[0].text;
    assert!(txt.contains("[REDACTED-EMAIL]"));
    assert!(!decoded.redactions_applied.is_empty());
}

struct StubLedger;
impl EventLedgerReader for StubLedger {
    fn read_spawn_pairs(&self, session_id: Uuid) -> Result<Vec<SpawnPair>, HarvestError> {
        Ok(vec![
            SpawnPair {
                session_id,
                parent_message: "what is X".to_string(),
                child_response: "X is Y".to_string(),
                parent_role: "parent".to_string(),
                child_role: "child".to_string(),
                parent_message_id: Uuid::from_u128(1),
                child_message_id: Uuid::from_u128(2),
                child_outcome: CapsuleOutcome::Pass {
                    mt_id: "MT-1".to_string(),
                    validator_verdict_id: Uuid::now_v7(),
                },
            },
            SpawnPair {
                session_id,
                parent_message: "what is Z".to_string(),
                child_response: "Z is wrong".to_string(),
                parent_role: "parent".to_string(),
                child_role: "child".to_string(),
                parent_message_id: Uuid::from_u128(3),
                child_message_id: Uuid::from_u128(4),
                child_outcome: CapsuleOutcome::Fail {
                    mt_id: "MT-2".to_string(),
                    validator_verdict_id: Uuid::now_v7(),
                    failure_class: FailureClass::Other,
                },
            },
        ])
    }
}
struct StubReviewer;
impl ContentReviewer for StubReviewer {
    fn review_pair(&self, _pair: &SpawnPair) -> Result<ContentReviewVerdict, HarvestError> {
        Ok(ContentReviewVerdict::Pass {
            license_provenance: "custom_internal".to_string(),
        })
    }
}
struct StubCandidateSub {
    candidates: Mutex<Vec<DistillationCandidate>>,
}
impl DistillationCandidateSubmitter for StubCandidateSub {
    fn submit_candidate(
        &self,
        submission: DistillationCandidateSubmission,
    ) -> Result<Uuid, HarvestError> {
        let id = submission.candidate.candidate_id;
        self.candidates.lock().unwrap().push(submission.candidate);
        Ok(id)
    }
}

#[test]
fn spawn_tree_harvester_emits_only_pass_pairs() {
    let ledger = StubLedger;
    let reviewer = StubReviewer;
    let submitter = StubCandidateSub {
        candidates: Mutex::new(Vec::new()),
    };
    let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);
    let candidates = harvester.harvest(Uuid::now_v7(), true).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].parent_role, "parent");
}

#[test]
fn spawn_tree_harvester_refuses_without_opt_in() {
    let ledger = StubLedger;
    let reviewer = StubReviewer;
    let submitter = StubCandidateSub {
        candidates: Mutex::new(Vec::new()),
    };
    let harvester = SpawnTreeHarvester::new(&ledger, &reviewer, &submitter);
    let err = harvester.harvest(Uuid::now_v7(), false).unwrap_err();
    assert!(matches!(err, HarvestError::OptInRequired { .. }));
}

#[test]
fn ac_distill_negative_path_pii_and_dedup_enforced() {
    use handshake_core::distillation::content_review::{
        ContentReview, ContentReviewConfig, ReviewVerdict,
    };
    use handshake_core::distillation::corpus_extractor::TrainingTurn;
    // PII: prompt with email is REJECTED for High severity (per
    // content_review semantics).
    let mut reviewer = ContentReview::new(ContentReviewConfig::defaults());
    let pii = TrainingTurn {
        id: "pii-1".to_string(),
        session_id: "s".to_string(),
        model_id: "m".to_string(),
        prompt: "contact me at me@example.com".to_string(),
        completion: "ok".to_string(),
        finish_reason: Some("stop".to_string()),
        license_tag: "MIT".to_string(),
        source_event_ids: vec!["e1".to_string()],
        sourced_at_utc: "2026-05-20T00:00:00Z".to_string(),
    };
    let verdict = reviewer.review(&pii).unwrap();
    assert!(matches!(
        verdict,
        ReviewVerdict::Quarantine { .. } | ReviewVerdict::Reject { .. }
    ));

    // Untaggable license: empty license_tag -> Quarantine.
    let untag = TrainingTurn {
        id: "u-1".to_string(),
        session_id: "s".to_string(),
        model_id: "m".to_string(),
        prompt: "hello".to_string(),
        completion: "world".to_string(),
        finish_reason: Some("stop".to_string()),
        license_tag: "".to_string(),
        source_event_ids: vec!["e1".to_string()],
        sourced_at_utc: "2026-05-20T00:00:00Z".to_string(),
    };
    let verdict = reviewer.review(&untag).unwrap();
    assert!(matches!(verdict, ReviewVerdict::Quarantine { .. }));

    // Near-dup (exact dedup hits the same hash).
    let a = TrainingTurn {
        id: "dup-1".to_string(),
        session_id: "s".to_string(),
        model_id: "m".to_string(),
        prompt: "q".to_string(),
        completion: "a".to_string(),
        finish_reason: Some("stop".to_string()),
        license_tag: "MIT".to_string(),
        source_event_ids: vec!["e1".to_string()],
        sourced_at_utc: "2026-05-20T00:00:00Z".to_string(),
    };
    let _ = reviewer.review(&a).unwrap();
    let dup = TrainingTurn {
        id: "dup-2".to_string(),
        ..a
    };
    let dup_verdict = reviewer.review(&dup).unwrap();
    assert!(matches!(dup_verdict, ReviewVerdict::Quarantine { .. }));
}
