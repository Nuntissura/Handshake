//! WP-KERNEL-005 MT-168: FEMS V1 feedback chain end-to-end against managed
//! PostgreSQL.
//!
//! Integration validation v2 failed MT-168 because the prior e2e composed
//! every surface through deterministic in-memory adapters (FixtureFems +
//! catalog/outcome/pin/hygiene mocks + TraceSourceStub) and never touched a
//! managed resource. This test re-runs the same production chain with the
//! REAL Postgres-backed adapters in the proof path:
//!
//!   CapsuleBuilder -> CapsuleRecorder over [`PostgresKernelActionSubmitter`]
//!     (real KernelActionCatalogV1 validation + `kernel_event_ledger` row)
//!   -> [`PostgresMemoryCapsuleStore`] save + FRESH-instance re-read
//!   -> OutcomeFeedbackLoop over the SAME Postgres submitter (ledger row)
//!   -> PinIpcService over the Postgres submitter; pinned state RE-READ from
//!      PostgreSQL via a FRESH submitter's `list_pinned`
//!   -> HygieneJobRunner submitting consolidation/prune candidates to the
//!      Postgres submitter (ledger rows; pinned item protected)
//!   -> trace export via [`PostgresTraceSource`] + [`export_persisted_trace`]
//!      (persisted bundle re-read from PostgreSQL, redacted tarball embedding
//!      REAL ArtifactStore bytes, auditable export receipt).
//!
//! Every stage's EventLedger evidence is asserted directly on the `Database`
//! ledger reader. The FEMS retriever fixture supplies only the INPUT corpus;
//! every persistence/read surface in the proof path is the managed resource.
//!
//! The Postgres adapters bridge sync traits over async storage, so the test
//! runs on a multi-thread runtime. Gated on
//! `atelier_pg_support::database_url()`: when no PostgreSQL is available the
//! test prints SKIP and returns (never SQLite).

mod atelier_pg_support;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use atelier_pg_support::{database_url, write_native_media_artifact_from_stored_payload};
use chrono::{DateTime, Utc};
use handshake_core::memory::bitemporal::{BitemporalIndex, BitemporalItem, BitemporalStamps};
use handshake_core::memory::builder::{BuildContext, FemsError, FemsRetriever, RetrievedItem};
use handshake_core::memory::hygiene::{
    fingerprint_for_text, HygieneConfig, HygieneItemView, HygieneJobRunner, HygieneTask,
    InMemoryFemsAccessor, HYGIENE_CONSOLIDATION_ACTION_ID, HYGIENE_PRUNE_ACTION_ID,
};
use handshake_core::memory::outcome_feedback::{
    CapsuleOutcome, MemoryPackItemRef, OutcomeFeedbackLoop, TuningParams,
    OUTCOME_ATTACH_ACTION_ID,
};
use handshake_core::memory::progressive_retrieval::DegradationReport;
use handshake_core::memory::retrieval_mode::RetrievalMode;
use handshake_core::memory::trace_export::{
    ArtifactRef, CacheMarkers, ExportFormat, MemoryPackBudgets, QueryPlan, RedactionPolicy,
    RetrievalTrace, RouteDecision, ScoringInputSnapshot, SpanRef, TraceBundle, TraceSource,
    TRACE_EXPORT_VERSION,
};
use handshake_core::memory::{
    export_persisted_trace, CapsuleBuilder, CapsulePolicyTable, CapsuleRecord, CapsuleRecorder,
    DegradationTier, MemoryCapsuleIpcStore, PinIpcService, PinSubmitter,
    PostgresKernelActionSubmitter, PostgresMemoryCapsuleStore, PostgresTraceSource,
    RetrievalPolicy, SetPinRequest, TaskType, FR_EVT_MEMORY_PIN, MEMORY_CAPSULE_AGGREGATE_TYPE,
    MEMORY_CAPSULE_RECORD_ACTION_ID, MEMORY_TRACE_AGGREGATE_TYPE,
    MEMORY_TRACE_EXPORTED_PAYLOAD_SCHEMA_ID, PIN_MEMORY_ACTION_ID, RETRIEVAL_SCORING_FORMULA_V0,
};
use handshake_core::kernel::KernelEvent;
use handshake_core::storage::{postgres::PostgresDatabase, Database};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

const ROLE_ID: &str = "KERNEL_BUILDER";
const SESSION_ID: &str = "mt-168-postgres-session";
/// Aggregate type for memory items (pins + hygiene candidates) in the kernel
/// event ledger — see `aggregate_type_for_target_kind` in
/// `memory/persistence_postgres.rs`.
const MEMORY_ITEM_AGGREGATE_TYPE: &str = "memory_item";

async fn connected_database(url: &str) -> Arc<dyn Database> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(url)
        .await
        .expect("connect to PostgreSQL");
    let database = PostgresDatabase::new(pool);
    database
        .run_migrations()
        .await
        .expect("run kernel migrations");
    database.into_arc()
}

fn at(secs: i64) -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
}

fn retrieved(id: Uuid, score: f64, bytes: u64) -> RetrievedItem {
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
        pinned: false,
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

/// FEMS retriever fixture: supplies the INPUT corpus only. All persistence
/// surfaces downstream are the real Postgres-backed adapters.
struct FixtureFems {
    items: Vec<RetrievedItem>,
}

impl FemsRetriever for FixtureFems {
    fn retrieve(&self, _query: &str, top_k: u32) -> Result<Vec<RetrievedItem>, FemsError> {
        Ok(self.items.iter().take(top_k as usize).cloned().collect())
    }
}

fn has_catalog_action(events: &[KernelEvent], action_id: &str) -> bool {
    events
        .iter()
        .any(|event| event.payload["catalog_action_id"] == json!(action_id))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mt168_fems_feedback_chain_persists_through_postgres_stores() {
    let Some(url) = database_url().await else {
        eprintln!(
            "SKIP mt168_fems_feedback_chain_persists_through_postgres_stores: \
             PostgreSQL unavailable"
        );
        return;
    };
    let database = connected_database(&url).await;

    // Unique-per-run item ids so the append-only ledger from earlier central
    // runs can never satisfy (or poison) this run's assertions.
    let item_a = Uuid::now_v7();
    let item_b = Uuid::now_v7();
    let item_c = Uuid::now_v7();
    let items = vec![
        retrieved(item_a, 0.90, 256),
        retrieved(item_b, 0.80, 256),
        retrieved(item_c, 0.70, 256),
    ];

    // ---- Stage 1: build the capsule through the production builder. ----
    let fems = FixtureFems {
        items: items.clone(),
    };
    let table = CapsulePolicyTable;
    let builder = CapsuleBuilder::new(&fems, &table);
    let capsule = builder
        .build(BuildContext {
            task_type: TaskType::GeneralRetrieval,
            query: "assemble FEMS feedback chain over postgres".to_string(),
            role_id: ROLE_ID.to_string(),
            session_id: SESSION_ID.to_string(),
            override_policy: Some(policy()),
        })
        .expect("CapsuleBuilder must build over the fixture corpus");
    assert_eq!(capsule.pack.items.len(), 2);

    // ---- Stage 2: record through the REAL kernel action catalog dispatcher
    // (catalog validation + kernel_event_ledger append). ----
    let submitter = PostgresKernelActionSubmitter::with_db(Arc::clone(&database));
    let recorder = CapsuleRecorder {
        action_catalog: &submitter,
    };
    let record = CapsuleRecord::from_capsule(&capsule, Utc::now(), SESSION_ID, ROLE_ID);
    let receipt = recorder.record(record.clone()).expect("recorder.record");
    assert!(!receipt.record_id.is_nil());

    let capsule_events = database
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &record.capsule_id.to_string(),
        )
        .await
        .expect("list capsule ledger events");
    assert!(
        has_catalog_action(&capsule_events, MEMORY_CAPSULE_RECORD_ACTION_ID),
        "capsule record must land in kernel_event_ledger via the catalog action"
    );

    // ---- Stage 3: durable capsule store; RE-READ via a FRESH instance. ----
    let store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&database));
    store
        .save_capsule_record(record.clone())
        .expect("save capsule record to PostgreSQL");
    let fresh_store = PostgresMemoryCapsuleStore::with_db(Arc::clone(&database));
    let reloaded = fresh_store
        .get_capsule_record(record.capsule_id)
        .expect("re-read capsule record")
        .expect("capsule record must exist after restart-equivalent re-read");
    assert_eq!(reloaded.capsule_id, record.capsule_id);
    assert_eq!(reloaded.capsule_source_hash, record.capsule_source_hash);

    // ---- Stage 4: outcome feedback through the SAME Postgres submitter. ----
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
    assert!(!included_pack.is_empty());
    let mut scores = included_pack
        .iter()
        .map(|item| (item.memory_id, 0.5))
        .collect::<HashMap<_, _>>();
    let feedback_loop = OutcomeFeedbackLoop::new(&submitter);
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
        .expect("attach outcome through PostgresKernelActionSubmitter");
    assert!(included_pack
        .iter()
        .all(|item| scores[&item.memory_id] > 0.5));

    let capsule_events = database
        .list_kernel_events_for_aggregate(
            MEMORY_CAPSULE_AGGREGATE_TYPE,
            &capsule.id.to_string(),
        )
        .await
        .expect("list capsule ledger events after outcome");
    assert!(
        has_catalog_action(&capsule_events, OUTCOME_ATTACH_ACTION_ID),
        "outcome attachment must land in kernel_event_ledger"
    );

    // ---- Stage 5: pin through the Postgres submitter; RE-READ the pinned
    // state from PostgreSQL via a FRESH submitter. ----
    let pinned_id = included_pack[0].memory_id;
    let pin_service = PinIpcService::new(&submitter);
    let pin_receipt = pin_service
        .set(SetPinRequest {
            item_id: pinned_id,
            pinned: true,
            reason: "core memory for MT-168 postgres regression".to_string(),
            actor_id: ROLE_ID.to_string(),
            session_id: SESSION_ID.to_string(),
        })
        .expect("set pin through PostgresKernelActionSubmitter");
    assert_eq!(pin_receipt.action_id, PIN_MEMORY_ACTION_ID);
    assert_eq!(pin_receipt.fr_event_kind, FR_EVT_MEMORY_PIN);

    let fresh_submitter = PostgresKernelActionSubmitter::with_db(Arc::clone(&database));
    let pinned_items = fresh_submitter.list_pinned().expect("list pinned from PG");
    let persisted_pin = pinned_items
        .iter()
        .find(|item| item.memory_id == pinned_id)
        .expect("pinned item must be re-readable from PostgreSQL after restart");
    assert!(persisted_pin.pinned);

    let pin_events = database
        .list_kernel_events_for_aggregate(MEMORY_ITEM_AGGREGATE_TYPE, &pinned_id.to_string())
        .await
        .expect("list pin ledger events");
    assert!(
        has_catalog_action(&pin_events, PIN_MEMORY_ACTION_ID),
        "pin action must land in kernel_event_ledger"
    );

    // ---- Stage 6: hygiene candidates submitted to the Postgres catalog;
    // the pinned item is protected from pruning. ----
    let index = Mutex::new({
        let mut idx = BitemporalIndex::new();
        for item in &items {
            idx.insert(bitemporal_item(item));
        }
        idx
    });
    let fp = fingerprint_for_text("duplicate-fems-content-mt168");
    let hygiene_view = |memory_id: Uuid, pinned: bool| HygieneItemView {
        memory_id,
        recorded_at: Utc::now() - chrono::Duration::seconds(86_400),
        score: 0.1,
        use_count: 5,
        pass_count: 5,
        pinned,
        content_fingerprint: fp,
        embedding: None,
    };
    let accessor = InMemoryFemsAccessor {
        index: &index,
        stats: vec![
            hygiene_view(item_a, true),
            hygiene_view(item_b, false),
            hygiene_view(item_c, false),
        ],
    };
    let hygiene_runner = HygieneJobRunner::new(&accessor, &submitter);
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
        .expect("hygiene run over the Postgres submitter");
    assert_eq!(hygiene_report.tasks.len(), 2);

    let item_b_events = database
        .list_kernel_events_for_aggregate(MEMORY_ITEM_AGGREGATE_TYPE, &item_b.to_string())
        .await
        .expect("list item_b ledger events");
    let item_c_events = database
        .list_kernel_events_for_aggregate(MEMORY_ITEM_AGGREGATE_TYPE, &item_c.to_string())
        .await
        .expect("list item_c ledger events");
    assert!(
        has_catalog_action(&item_b_events, HYGIENE_CONSOLIDATION_ACTION_ID)
            || has_catalog_action(&item_c_events, HYGIENE_CONSOLIDATION_ACTION_ID),
        "duplicate-fingerprint consolidation candidate must land in kernel_event_ledger"
    );
    assert!(
        has_catalog_action(&item_b_events, HYGIENE_PRUNE_ACTION_ID),
        "stale unpinned item_b must produce a durable prune candidate"
    );
    assert!(
        has_catalog_action(&item_c_events, HYGIENE_PRUNE_ACTION_ID),
        "stale unpinned item_c must produce a durable prune candidate"
    );
    let item_a_events = database
        .list_kernel_events_for_aggregate(MEMORY_ITEM_AGGREGATE_TYPE, &item_a.to_string())
        .await
        .expect("list item_a ledger events");
    assert!(
        !has_catalog_action(&item_a_events, HYGIENE_PRUNE_ACTION_ID),
        "pinned item_a must never be pruned (pin protection on the durable path)"
    );

    // ---- Stage 7: trace export through the production Postgres source —
    // persisted bundle re-read from PostgreSQL, redacted tarball embedding
    // REAL ArtifactStore bytes, auditable export receipt in the ledger. ----
    let artifact = write_native_media_artifact_from_stored_payload(
        b"MT-168 packet evidence token:tok-mt168-artifact-secret end\n",
    );
    let artifact_trace_id = format!("L1/{}", artifact.artifact_id);
    let trace_id = Uuid::now_v7();
    let bundle = TraceBundle {
        trace_id,
        query_plan: QueryPlan {
            plan_id: Uuid::now_v7(),
            query: "find contact user@example.com token=abc123-mt168".to_string(),
            task_type: "general".to_string(),
        },
        retrieval_trace: RetrievalTrace {
            trace_id,
            query_plan_id: Uuid::now_v7(),
            item_ids: capsule
                .pack
                .items
                .iter()
                .map(|item| item.memory_id.clone())
                .collect(),
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
        degradation_report: DegradationReport {
            tiers_completed: vec!["full_text".to_string()],
            tiers_skipped: Vec::new(),
            load_signal_at_start: 0.0,
            started_at_utc: Utc::now(),
            total_duration_ms: 0,
            tier_chosen: DegradationTier::Tiered,
        },
        selected_spans: vec![SpanRef {
            span_id: "span-1".to_string(),
            text: "email user@example.com api_key=secret123-mt168".to_string(),
        }],
        referenced_artifacts: vec![ArtifactRef {
            artifact_id: artifact_trace_id.clone(),
            kind: "packet.json".to_string(),
            bytes_estimate: artifact.byte_len as u64,
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
    };

    let trace_source =
        PostgresTraceSource::new(Arc::clone(&database), artifact.workspace_root.clone());
    trace_source
        .persist_trace(&bundle)
        .expect("persist trace bundle to PostgreSQL");
    let fresh_trace_source =
        PostgresTraceSource::new(Arc::clone(&database), artifact.workspace_root.clone());
    assert_eq!(
        fresh_trace_source
            .load_trace(trace_id)
            .expect("re-read trace bundle from PostgreSQL"),
        bundle
    );

    let exported = export_persisted_trace(
        Arc::clone(&database),
        &artifact.workspace_root,
        trace_id,
        &RedactionPolicy::default(),
        ExportFormat::Tarball,
    )
    .expect("export persisted trace as tarball");
    let tar_text = String::from_utf8_lossy(&exported.bytes);
    assert!(!tar_text.contains("abc123-mt168"));
    assert!(!tar_text.contains("secret123-mt168"));
    assert!(!tar_text.contains("tok-mt168-artifact-secret"));
    assert!(!tar_text.contains("user@example.com"));
    assert!(tar_text.contains("[REDACTED-SECRET]"));
    assert!(tar_text.contains("[REDACTED-EMAIL]"));

    let trace_events = database
        .list_kernel_events_for_aggregate(MEMORY_TRACE_AGGREGATE_TYPE, &trace_id.to_string())
        .await
        .expect("list trace ledger events");
    let receipt_event = trace_events
        .iter()
        .find(|event| {
            event.payload["schema_id"] == json!(MEMORY_TRACE_EXPORTED_PAYLOAD_SCHEMA_ID)
        })
        .expect("trace export receipt must persist in kernel_event_ledger");
    assert_eq!(
        receipt_event.payload["content_hash"],
        json!(exported.content_hash)
    );
}
