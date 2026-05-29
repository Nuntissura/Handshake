//! MT-168: FEMS V1 feedback integration end-to-end test.
//!
//! This test composes the D.3 feature surfaces through their production
//! trait/API boundaries with deterministic in-memory adapters.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use handshake_core::distillation::spawn_tree_harvester::{
    ContentReviewVerdict, ContentReviewer, DistillationCandidateSubmission,
    DistillationCandidateSubmitter, EventLedgerReader, HarvestError, SpawnPair, SpawnTreeHarvester,
    FR_EVT_DISTILL_CANDIDATE_CREATED,
};
use handshake_core::memory::bitemporal::{
    AsOfQuery, BitemporalIndex, BitemporalItem, BitemporalStamps,
};
use handshake_core::memory::builder::{BuildContext, FemsError, FemsRetriever, RetrievedItem};
use handshake_core::memory::calibration::{
    CalibrationCollector, CalibrationError, CalibrationMetrics, CalibrationThresholds,
    FemsCalibrationSource, SignalStatus,
};
use handshake_core::memory::hygiene::{
    fingerprint_for_text, HygieneActionSubmitter, HygieneConfig, HygieneError, HygieneItemView,
    HygieneJobRunner, HygieneTask, InMemoryFemsAccessor, ProceduralPromotion,
};
use handshake_core::memory::outcome_feedback::{
    CapsuleOutcome, FailureClass, MemoryPackItemRef, OutcomeAttachSubmitter, OutcomeAttribution,
    OutcomeError, OutcomeFeedbackLoop, OutcomeReceipt, TuningParams, OUTCOME_ATTACH_ACTION_ID,
};
use handshake_core::memory::progressive_retrieval::{
    LoadSignal, ProgressiveRetriever, RetrievalError, RetrievalTier, RetrievedItem as PrItem,
    TIER_FULL_TEXT, TIER_GRAPH_EXPANSION, TIER_RERANK, TIER_VECTOR,
};
use handshake_core::memory::replay_eval::{CapsuleRecordReader, ReplayEvaluator, ReplayRequest};
use handshake_core::memory::retrieval_mode::{
    OperationClass, RetrievalContext, RetrievalMode, RetrievalModePolicy, RetrievalModeRouter,
};
use handshake_core::memory::trace_export::{
    ArtifactRef, CacheMarkers, ExportFormat, MemoryPackBudgets, QueryPlan, RedactionPolicy,
    RetrievalTrace, RetrievalTraceExporter, RouteDecision, ScoringInputSnapshot, SpanRef,
    TraceBundle, TraceSource, TRACE_EXPORT_VERSION,
};
use handshake_core::memory::{
    CapsuleBuilder, CapsulePolicyTable, CapsuleRecord, CapsuleRecorder, DegradationTier,
    KernelActionRejection, KernelActionSubmission, KernelActionSubmitter, PinError, PinIpcService,
    PinReceipt, PinSubmitter, PinnedItem, RetrievalPolicy, SetPinRequest, TaskType,
    FR_EVT_MEMORY_PIN, MEMORY_CAPSULE_RECORD_ACTION_ID, PIN_MEMORY_ACTION_ID,
    RETRIEVAL_SCORING_FORMULA_V0,
};
use uuid::Uuid;

fn at(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
}

fn retrieved(id: Uuid, score: f64, bytes: u64, pinned: bool) -> RetrievedItem {
    let id = id.to_string();
    RetrievedItem {
        item_id: id.clone(),
        memory_class: "operator_memory".to_string(),
        item_type: "doc".to_string(),
        summary: format!("summary {id}"),
        content: format!("content {id}"),
        structured: None,
        trust_level: "trusted".to_string(),
        confidence: score,
        scope_refs: Vec::new(),
        source_refs: Vec::new(),
        score,
        score_breakdown: BTreeMap::from([("fixture_score".to_string(), score)]),
        capsule_bytes: bytes,
        token_estimate: (bytes / 4) as u32,
        pinned,
    }
}

fn bitemporal_item(item: &RetrievedItem) -> BitemporalItem {
    BitemporalItem {
        item_id: Uuid::parse_str(&item.item_id).unwrap(),
        stamps: BitemporalStamps {
            valid_from: at(100),
            valid_until: None,
            recorded_at: at(50),
            invalidated_at: None,
        },
        payload: serde_json::to_value(item).unwrap(),
    }
}

fn policy() -> RetrievalPolicy {
    RetrievalPolicy {
        top_k: 2,
        capsule_budget_bytes: 4096,
        task_type: TaskType::GeneralRetrieval,
        scoring_formula_version: RETRIEVAL_SCORING_FORMULA_V0.to_string(),
        graceful_degradation_tier: DegradationTier::Tiered,
    }
}

fn build_context(query: impl Into<String>) -> BuildContext {
    BuildContext {
        task_type: TaskType::GeneralRetrieval,
        query: query.into(),
        role_id: "KERNEL_BUILDER".to_string(),
        session_id: "mt-168-session".to_string(),
        override_policy: Some(policy()),
    }
}

struct FixtureFems {
    items: Vec<RetrievedItem>,
}

impl FemsRetriever for FixtureFems {
    fn retrieve(&self, _query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        Ok(self.items.iter().take(top_k as usize).cloned().collect())
    }
}

struct CapsuleActionCatalog {
    submissions: Mutex<Vec<KernelActionSubmission>>,
}

impl KernelActionSubmitter for CapsuleActionCatalog {
    fn submit(&self, submission: KernelActionSubmission) -> Result<(), KernelActionRejection> {
        self.submissions.lock().unwrap().push(submission);
        Ok(())
    }
}

struct OutcomeCatalog {
    attributions: Mutex<Vec<OutcomeAttribution>>,
}

impl OutcomeAttachSubmitter for OutcomeCatalog {
    fn attach_outcome(
        &self,
        attribution: OutcomeAttribution,
    ) -> Result<OutcomeReceipt, OutcomeError> {
        self.attributions.lock().unwrap().push(attribution.clone());
        Ok(OutcomeReceipt {
            receipt_id: Uuid::now_v7(),
            capsule_id: attribution.capsule_id,
            action_id: OUTCOME_ATTACH_ACTION_ID.to_string(),
            recorded_at_utc: Utc::now(),
        })
    }
}

struct PinCatalog {
    items: Mutex<Vec<PinnedItem>>,
}

impl PinSubmitter for PinCatalog {
    fn set_pin(&self, item: PinnedItem) -> Result<PinReceipt, PinError> {
        self.items.lock().unwrap().push(item.clone());
        Ok(PinReceipt {
            receipt_id: Uuid::now_v7(),
            memory_id: item.memory_id,
            pinned: item.pinned,
            action_id: PIN_MEMORY_ACTION_ID.to_string(),
            fr_event_kind: FR_EVT_MEMORY_PIN.to_string(),
        })
    }

    fn list_pinned(&self) -> Result<Vec<PinnedItem>, PinError> {
        Ok(self.items.lock().unwrap().clone())
    }
}

struct HygieneCatalog {
    consolidations: Mutex<Vec<(Uuid, Uuid)>>,
    prunes: Mutex<Vec<Uuid>>,
    flags: Mutex<Vec<(Uuid, Uuid)>>,
    promotions: Mutex<Vec<ProceduralPromotion>>,
}

impl HygieneCatalog {
    fn new() -> Self {
        Self {
            consolidations: Mutex::new(Vec::new()),
            prunes: Mutex::new(Vec::new()),
            flags: Mutex::new(Vec::new()),
            promotions: Mutex::new(Vec::new()),
        }
    }
}

impl HygieneActionSubmitter for HygieneCatalog {
    fn submit_consolidation_candidate(
        &self,
        left: Uuid,
        right: Uuid,
    ) -> Result<Uuid, HygieneError> {
        self.consolidations.lock().unwrap().push((left, right));
        Ok(Uuid::now_v7())
    }

    fn submit_prune(&self, memory_id: Uuid, _at: DateTime<Utc>) -> Result<Uuid, HygieneError> {
        self.prunes.lock().unwrap().push(memory_id);
        Ok(Uuid::now_v7())
    }

    fn submit_contradiction_flag(&self, left: Uuid, right: Uuid) -> Result<Uuid, HygieneError> {
        self.flags.lock().unwrap().push((left, right));
        Ok(Uuid::now_v7())
    }

    fn submit_procedural_promotion(
        &self,
        candidate: ProceduralPromotion,
    ) -> Result<Uuid, HygieneError> {
        self.promotions.lock().unwrap().push(candidate);
        Ok(Uuid::now_v7())
    }
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

struct CalibrationSource;

impl FemsCalibrationSource for CalibrationSource {
    fn calibration_metrics(&self) -> Result<CalibrationMetrics, CalibrationError> {
        Ok(CalibrationMetrics {
            total_items: 100,
            total_bytes: 65_536,
            bytes_growth_rate: 1024.0,
            items_older_than_30d: 60,
            average_trust: 0.7,
            items_without_embedding: 20,
            recent_retrievals_total: 10,
            recent_retrievals_degraded: 4,
            trust_histogram_current: vec![1, 2, 3, 4, 5],
            trust_histogram_baseline: vec![5, 4, 3, 2, 1],
            last_hygiene_run_at: Some(Utc::now() - chrono::Duration::hours(100)),
            observed_at_utc: Utc::now(),
        })
    }
}

struct CapsuleRecordStore {
    record: CapsuleRecord,
}

impl CapsuleRecordReader for CapsuleRecordStore {
    fn read_record(
        &self,
        capsule_id: Uuid,
    ) -> Result<CapsuleRecord, handshake_core::memory::replay_eval::ReplayError> {
        if self.record.capsule_id == capsule_id {
            Ok(self.record.clone())
        } else {
            Err(handshake_core::memory::replay_eval::ReplayError::CapsuleNotFound { capsule_id })
        }
    }
}

struct TraceSourceStub;

impl TraceSource for TraceSourceStub {
    fn load_trace(
        &self,
        trace_id: Uuid,
    ) -> Result<TraceBundle, handshake_core::memory::trace_export::ExportError> {
        Ok(TraceBundle {
            trace_id,
            query_plan: QueryPlan {
                plan_id: Uuid::now_v7(),
                query: "find contact user@example.com token=abc123".to_string(),
                task_type: "general".to_string(),
            },
            retrieval_trace: RetrievalTrace {
                trace_id,
                query_plan_id: Uuid::now_v7(),
                item_ids: Vec::new(),
            },
            budgets: MemoryPackBudgets {
                max_items: 2,
                max_bytes: 4096,
            },
            cache_markers: CacheMarkers {
                cache_hit_count: 0,
                cache_miss_count: 1,
            },
            retrieval_mode: RetrievalMode::Hybrid,
            route_decision: RouteDecision {
                matched_rule_id: "general_freeform_query".to_string(),
                rationale: "general freeform query uses hybrid retrieval".to_string(),
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
                span_id: "span-1".to_string(),
                text: "email user@example.com api_key=secret123".to_string(),
            }],
            referenced_artifacts: vec![ArtifactRef {
                artifact_id: "packet".to_string(),
                kind: "packet.json".to_string(),
                bytes_estimate: 1024,
            }],
            scoring_inputs: vec![ScoringInputSnapshot {
                item_id: "span-1".to_string(),
                importance: 0.5,
                recency: 0.5,
                trust: 0.5,
                outcome_tuned_weight: 0.5,
                embedding_similarity: 0.5,
                formula_version: "injection_scoring_formula_v1".to_string(),
            }],
            redactions_applied: Vec::new(),
            exported_at_utc: DateTime::<Utc>::MIN_UTC,
            exporter_version: TRACE_EXPORT_VERSION.to_string(),
        })
    }
}

struct SpawnFixtureLedger {
    pairs: Vec<SpawnPair>,
}

impl EventLedgerReader for SpawnFixtureLedger {
    fn read_spawn_pairs(&self, session_id: Uuid) -> Result<Vec<SpawnPair>, HarvestError> {
        Ok(self
            .pairs
            .iter()
            .filter(|pair| pair.session_id == session_id)
            .cloned()
            .collect())
    }
}

struct SpawnReviewer {
    reviewed: Mutex<Vec<Uuid>>,
}

impl ContentReviewer for SpawnReviewer {
    fn review_pair(&self, pair: &SpawnPair) -> Result<ContentReviewVerdict, HarvestError> {
        self.reviewed.lock().unwrap().push(pair.child_message_id);
        Ok(ContentReviewVerdict::Pass {
            license_provenance: "custom_internal".to_string(),
        })
    }
}

struct CandidateCatalog {
    submissions: Mutex<Vec<DistillationCandidateSubmission>>,
}

impl DistillationCandidateSubmitter for CandidateCatalog {
    fn submit_candidate(
        &self,
        submission: DistillationCandidateSubmission,
    ) -> Result<Uuid, HarvestError> {
        let id = submission.candidate.candidate_id;
        self.submissions.lock().unwrap().push(submission);
        Ok(id)
    }
}

struct EnvVarGuard {
    key: &'static str,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn set_true(key: &'static str) -> Self {
        let previous = std::env::var(key).ok();
        std::env::set_var(key, "true");
        Self { key, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.previous {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

#[test]
fn fems_feedback_chain_composes_end_to_end() {
    let item_a = Uuid::from_u128(0x7000_0000_0000_0000_0000_0000_0000_0001);
    let item_b = Uuid::from_u128(0x7000_0000_0000_0000_0000_0000_0000_0002);
    let item_c = Uuid::from_u128(0x7000_0000_0000_0000_0000_0000_0000_0003);
    let items = vec![
        retrieved(item_a, 0.90, 256, false),
        retrieved(item_b, 0.80, 256, false),
        retrieved(item_c, 0.70, 256, false),
    ];
    let index = Mutex::new({
        let mut idx = BitemporalIndex::new();
        for item in &items {
            idx.insert(bitemporal_item(item));
        }
        idx
    });

    let fems = FixtureFems {
        items: items.clone(),
    };
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);

    let route_policy = RetrievalModePolicy::default_v0();
    let router = RetrievalModeRouter::new(&route_policy);
    let general_query = "assemble FEMS feedback chain".to_string();
    let (general_mode, general_rule) = router.route(&RetrievalContext {
        query: general_query.clone(),
        task_type: TaskType::GeneralRetrieval,
        operation_class: OperationClass::GeneralFreeform,
    });
    assert_eq!(general_mode, RetrievalMode::Hybrid);
    assert_eq!(general_rule, "general_freeform_query");
    let capsule = builder.build(build_context(general_query)).unwrap();
    assert_eq!(capsule.pack.items.len(), 2);

    let (uuid_mode, uuid_rule) = router.route(&RetrievalContext {
        query: item_a.to_string(),
        task_type: TaskType::GeneralRetrieval,
        operation_class: OperationClass::QueryPlan,
    });
    assert_eq!(uuid_mode, RetrievalMode::NoRag);
    assert_eq!(uuid_rule, "exact_uuid_in_query");
    let uuid_capsule = builder.build(build_context(item_a.to_string())).unwrap();
    assert_eq!(uuid_capsule.source_hash, capsule.source_hash);

    let (hbr_mode, hbr_rule) = router.route(&RetrievalContext {
        query: "check HBR-INT-006".to_string(),
        task_type: TaskType::GeneralRetrieval,
        operation_class: OperationClass::QueryPlan,
    });
    assert_eq!(hbr_mode, RetrievalMode::AuthoritativeOnly);
    assert_eq!(hbr_rule, "hbr_id_pattern");
    let hbr_capsule = builder.build(build_context("check HBR-INT-006")).unwrap();
    assert_eq!(hbr_capsule.source_hash, capsule.source_hash);

    let capsule_record =
        CapsuleRecord::from_capsule(&capsule, Utc::now(), "mt-168-session", "KERNEL_BUILDER");
    let action_catalog = CapsuleActionCatalog {
        submissions: Mutex::new(Vec::new()),
    };
    let recorder = CapsuleRecorder {
        action_catalog: &action_catalog,
    };
    let receipt = recorder.record(capsule_record.clone()).unwrap();
    assert_eq!(
        action_catalog.submissions.lock().unwrap()[0]
            .request
            .action_id,
        MEMORY_CAPSULE_RECORD_ACTION_ID
    );
    assert!(!receipt.record_id.is_nil());

    let included_pack = capsule
        .audit
        .entries
        .iter()
        .filter(|entry| entry.included)
        .map(|entry| MemoryPackItemRef {
            memory_id: Uuid::parse_str(&entry.item_id).unwrap(),
            pinned: entry.pinned,
        })
        .collect::<Vec<_>>();
    let mut scores = included_pack
        .iter()
        .map(|item| (item.memory_id, 0.5))
        .collect::<HashMap<_, _>>();
    let outcome_catalog = OutcomeCatalog {
        attributions: Mutex::new(Vec::new()),
    };
    let feedback_loop = OutcomeFeedbackLoop::new(&outcome_catalog);
    feedback_loop
        .record_outcome(
            capsule.id,
            CapsuleOutcome::Pass {
                mt_id: "MT-168".to_string(),
                validator_verdict_id: Uuid::now_v7(),
            },
            &mut scores,
            &included_pack,
            &TuningParams {
                per_item_decay_per_use: 0.0,
                ..TuningParams::default()
            },
        )
        .unwrap();
    assert!(included_pack
        .iter()
        .all(|item| scores[&item.memory_id] > 0.5));

    let pin_catalog = PinCatalog {
        items: Mutex::new(Vec::new()),
    };
    let pin_service = PinIpcService::new(&pin_catalog);
    let pinned_id = included_pack[0].memory_id;
    let pin_receipt = pin_service
        .set(SetPinRequest {
            item_id: pinned_id,
            pinned: true,
            reason: "core memory for MT-168 regression".to_string(),
            actor_id: "KERNEL_BUILDER".to_string(),
            session_id: "mt-168-session".to_string(),
        })
        .unwrap();
    assert_eq!(pin_receipt.action_id, PIN_MEMORY_ACTION_ID);
    assert_eq!(pin_receipt.fr_event_kind, FR_EVT_MEMORY_PIN);

    let boosted_pinned_score = scores[&pinned_id];
    let pack_after_pin = included_pack
        .iter()
        .map(|item| MemoryPackItemRef {
            memory_id: item.memory_id,
            pinned: item.memory_id == pinned_id,
        })
        .collect::<Vec<_>>();
    feedback_loop
        .record_outcome(
            capsule.id,
            CapsuleOutcome::Fail {
                mt_id: "MT-168".to_string(),
                validator_verdict_id: Uuid::now_v7(),
                failure_class: FailureClass::ValidatorRejected,
            },
            &mut scores,
            &pack_after_pin,
            &TuningParams::default(),
        )
        .unwrap();
    assert!((scores[&pinned_id] - boosted_pinned_score).abs() < 1e-9);
    assert!(pack_after_pin
        .iter()
        .filter(|item| !item.pinned)
        .all(|item| scores[&item.memory_id] < boosted_pinned_score));
    assert_eq!(outcome_catalog.attributions.lock().unwrap().len(), 2);

    let visible_before_hygiene = index
        .lock()
        .unwrap()
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(150),
        })
        .unwrap()
        .into_iter()
        .map(|item| item.item_id)
        .collect::<Vec<_>>();
    assert_eq!(visible_before_hygiene.len(), 3);

    let fp = fingerprint_for_text("duplicate-fems-content");
    let hygiene_stats = vec![
        HygieneItemView {
            memory_id: item_a,
            recorded_at: Utc::now() - chrono::Duration::seconds(86_400),
            score: 0.1,
            use_count: 5,
            pass_count: 5,
            pinned: true,
            content_fingerprint: fp,
            embedding: None,
        },
        HygieneItemView {
            memory_id: item_b,
            recorded_at: Utc::now() - chrono::Duration::seconds(86_400),
            score: 0.1,
            use_count: 5,
            pass_count: 5,
            pinned: false,
            content_fingerprint: fp,
            embedding: None,
        },
        HygieneItemView {
            memory_id: item_c,
            recorded_at: Utc::now() - chrono::Duration::seconds(86_400),
            score: 0.1,
            use_count: 5,
            pass_count: 5,
            pinned: false,
            content_fingerprint: fp,
            embedding: None,
        },
    ];
    let hygiene_catalog = HygieneCatalog::new();
    let accessor = InMemoryFemsAccessor {
        index: &index,
        stats: hygiene_stats,
    };
    let hygiene_runner = HygieneJobRunner::new(&accessor, &hygiene_catalog);
    let hygiene_report = hygiene_runner
        .run_once(HygieneConfig {
            tasks: vec![
                HygieneTask::Consolidate { max_pairs: 10 },
                HygieneTask::PruneStale {
                    older_than_secs: 60,
                    min_score: 0.5,
                },
            ],
        })
        .unwrap();
    assert_eq!(hygiene_report.tasks.len(), 2);
    assert_eq!(
        hygiene_catalog.consolidations.lock().unwrap().as_slice(),
        &[(item_b, item_c)]
    );
    let prunes = hygiene_catalog.prunes.lock().unwrap().clone();
    assert_eq!(prunes.len(), 2);
    assert!(!prunes.contains(&item_a));

    index.lock().unwrap().invalidate(prunes[0], at(300));
    let visible_after_hygiene = index
        .lock()
        .unwrap()
        .items_visible_at(&AsOfQuery {
            as_of_world_time: at(150),
            as_of_recorded_time: at(350),
        })
        .unwrap()
        .into_iter()
        .map(|item| item.item_id)
        .collect::<Vec<_>>();
    assert_eq!(visible_after_hygiene.len(), 2);
    assert!(visible_after_hygiene.contains(&item_a));
    assert!(!visible_after_hygiene.contains(&prunes[0]));

    let full_text = StubTier {
        name: TIER_FULL_TEXT,
        items: vec![PrItem {
            item_id: "ft".to_string(),
            score: 0.9,
            tier: TIER_FULL_TEXT.to_string(),
        }],
    };
    let vector = StubTier {
        name: TIER_VECTOR,
        items: vec![PrItem {
            item_id: "vec".to_string(),
            score: 0.8,
            tier: TIER_VECTOR.to_string(),
        }],
    };
    let graph = StubTier {
        name: TIER_GRAPH_EXPANSION,
        items: vec![PrItem {
            item_id: "graph".to_string(),
            score: 0.7,
            tier: TIER_GRAPH_EXPANSION.to_string(),
        }],
    };
    let rerank = StubTier {
        name: TIER_RERANK,
        items: vec![PrItem {
            item_id: "rerank".to_string(),
            score: 0.6,
            tier: TIER_RERANK.to_string(),
        }],
    };
    let progressive = ProgressiveRetriever::new(&full_text, &vector, &graph, &rerank);
    let (_progressive_items, degradation_report) = progressive
        .retrieve_progressive(
            "mt-168 progressive query",
            10,
            DegradationTier::Tiered,
            &ConstLoad(0.9),
        )
        .unwrap();
    assert!(degradation_report
        .tiers_skipped
        .contains(&TIER_GRAPH_EXPANSION.to_string()));
    assert!(degradation_report
        .tiers_skipped
        .contains(&TIER_RERANK.to_string()));

    let calibration = CalibrationCollector::collect_snapshot(
        &CalibrationSource,
        CalibrationThresholds::default(),
    )
    .unwrap();
    assert_eq!(
        calibration.signals.degradation_rate.status,
        SignalStatus::Alert
    );

    let replay_result = {
        let guard = index.lock().unwrap();
        ReplayEvaluator::replay(
            ReplayRequest {
                capsule_id: capsule_record.capsule_id,
                replay_as_of: AsOfQuery {
                    as_of_world_time: at(150),
                    as_of_recorded_time: at(150),
                },
                expected_source_hash: capsule_record.capsule_source_hash.clone(),
            },
            &CapsuleRecordStore {
                record: capsule_record.clone(),
            },
            &*guard,
            &builder,
        )
        .unwrap()
    };
    assert!(replay_result.replay_succeeded);
    assert!(replay_result.replay_difference.is_empty());

    let exported = RetrievalTraceExporter::new(&TraceSourceStub)
        .export(
            Uuid::now_v7(),
            &RedactionPolicy::default(),
            ExportFormat::Json,
        )
        .unwrap();
    let trace_bundle: TraceBundle = serde_json::from_slice(&exported.bytes).unwrap();
    assert!(trace_bundle.query_plan.query.contains("[REDACTED-EMAIL]"));
    assert!(trace_bundle.query_plan.query.contains("[REDACTED-SECRET]"));
    assert!(trace_bundle.selected_spans[0]
        .text
        .contains("[REDACTED-EMAIL]"));
    assert!(trace_bundle.selected_spans[0]
        .text
        .contains("[REDACTED-SECRET]"));
    assert!(!trace_bundle.redactions_applied.is_empty());

    let fixture_pairs: Vec<SpawnPair> = serde_json::from_str(include_str!(
        "fixtures/fems_integration_e2e/spawn_tree.json"
    ))
    .unwrap();
    let session_id = Uuid::parse_str("00000000-0000-7000-8000-000000000001").unwrap();
    let ledger = SpawnFixtureLedger {
        pairs: fixture_pairs,
    };
    let reviewer = SpawnReviewer {
        reviewed: Mutex::new(Vec::new()),
    };
    let candidate_catalog = CandidateCatalog {
        submissions: Mutex::new(Vec::new()),
    };
    let _distill_guard = EnvVarGuard::set_true("DISTILL_CORPUS");
    let opt_in = std::env::var("DISTILL_CORPUS").as_deref() == Ok("true");
    let candidates = SpawnTreeHarvester::new(&ledger, &reviewer, &candidate_catalog)
        .harvest(session_id, opt_in)
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].child_role, "coder");
    assert_eq!(reviewer.reviewed.lock().unwrap().len(), 1);
    let submissions = candidate_catalog.submissions.lock().unwrap();
    assert_eq!(submissions.len(), 1);
    assert_eq!(
        submissions[0].event.event_name,
        FR_EVT_DISTILL_CANDIDATE_CREATED
    );
    assert!(!submissions[0].auto_promote);
    assert!(!submissions[0].conversation_text_authority);
}
