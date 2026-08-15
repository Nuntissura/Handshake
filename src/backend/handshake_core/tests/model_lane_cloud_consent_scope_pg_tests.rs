//! WP-1 MT-006 HBR-PRIV proof for the CLOUD projection/consent authority path.
//!
//! # Why this file exists separately from `model_lane_resource_scope_pg_tests`
//!
//! That suite proves account-bound scope on lane rows, navigation routes,
//! registry enumeration, derived non-widening and boot recovery. It does NOT
//! touch `model_lane_cloud_projection_plans` or
//! `model_lane_cloud_consent_receipts`, which are the two tables that decide
//! whether operator data may be shipped to a third-party provider. Borrowing
//! that suite's green for MT-006 would have been exactly the vacuous-pass the
//! PRIV pillar exists to prevent, so the cloud path is proven here on its own
//! terms:
//!
//! * **HBR-PRIV-001/002** — two distinct owning accounts; neither can read nor
//!   REUSE the other's ProjectionPlan or ConsentReceipt.
//! * **HBR-PRIV-005/007** — the receipt's approver is a typed, account-bound
//!   value; a self-issued governance-role-label string is refused at write time;
//!   an unattributed approval cannot authorize an account-scoped launch even if
//!   the row's owning-account column is backfilled underneath it.
//! * **HBR-PRIV-006** — after revocation a subsequent launch is refused, and the
//!   already-running lane stays pinned to its ORIGINAL immutable projection/
//!   consent context instead of being silently retargeted onto a newer grant.
//! * **HBR-PRIV-007** — the ProjectionPlan carries audience + local source scope
//!   + authorization-receipt provenance, and an audience that widens beyond the
//!   disclosed fan-out is rejected.
//!
//! Real PostgreSQL only. A missing cluster panics; it never skips green.
//!
//! # Falsifiability
//!
//! Every negative is paired with a positive control on the same data, so a
//! denial can never pass because nothing was written. Every `FALSIFIABILITY`
//! comment records an inversion that was actually applied, run against real
//! PostgreSQL, observed to fail, and then restored — the quoted text is the
//! observed panic message, not a prediction.

#![cfg(feature = "test-utils")]

mod knowledge_pg_support;

use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use async_trait::async_trait;

use handshake_core::api::account_scope::ProductLocalResourceScope;
use handshake_core::api::operator_chat::{routes, scoped_routes, OperatorChatState};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::model_runtime::catalog::ModelCatalog;

use handshake_core::model_runtime::{
    CancellationToken, Embedding, GenerateRequest, KvCacheHandle, LoraStackHandle,
    ModelCapabilities, ModelId, ModelRuntime, ModelRuntimeError, ProviderKind,
    RuntimeBinding as SwarmRuntimeBinding, Score, SteeringHookHandle, TokenStream,
};
use handshake_core::process_ledger::{
    drain_and_join_ledger_writer, LedgerBatcher, LedgerBatcherConfig, LedgerDrainJoinOutcome,
    NoopOverflowSink, PostgresProcessLedgerStore, ProcessLedgerError,
};

use axum::Extension;
use handshake_core::swarm_orchestration::model_lane::{
    CloudExportDelegation, LaunchAuthority, ModelLaneCloudConsentReceiptStatus,
    ModelLaneCloudConsentScope, ModelLaneCloudExportPosture, ModelLaneCloudProjectionPlanStatus,
    ModelLaneCloudRetentionPolicy, ModelLaneError, ModelLaneKind, ModelLaneLocusBinding,
    ModelLaneProviderKind, ModelLaneRecoveryState, ModelLaneStatus, ModelLaneStore, NewModelLane,
    NewModelLaneCloudConsentReceipt, NewModelLaneCloudProjectionPlan, NewModelLaneRun,
    RuntimeBinding,
};
use handshake_core::swarm_orchestration::operator_chat::{
    OperatorChatLaneKind, OperatorChatLaunchService, OperatorChatSelection,
    OperatorChatSingleRunCloudConsentGrant, OperatorChatSingleRunCloudLaunchRequest,
};
use handshake_core::swarm_orchestration::resource_scope::{
    stored_resource_scope_from_row, AccessSpaceRef, AccountBoundAuthority, ActorPrincipalId,
    AuthenticatedSessionRef, ExactResourceScopeAttribution, OwnerAccountId, ResourceAccessContext,
    ResourceScope, ResourceScopeQuery, StoredResourceScope, WorkspaceScopeRef,
    RESOURCE_SCOPE_SELECT_COLUMNS,
};
use handshake_core::swarm_orchestration::{
    CloudLaneFactoryConfig, CloudLiveRuntime, CloudRuntimeBuilder, ModelInstanceId,
    ModelSessionFactory, ProductionModelSessionFactory, RecordingSwarmSink, RunBudget,
    SpawnRequest, SwarmConfig, SwarmCoordinator,
};
use serde_json::json;
use sqlx::PgPool;

const WP_ID: &str = "WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1";
const MT_ID: &str = "MT-006";
const TASK_BOARD_ID: &str = "task-board://wp-1";
const USERMANUAL_BEHAVIOR: &str = "usermanual://model-lane-cloud-projection-consent#launch";
const PROVIDER_KIND: &str = "openai";
const REQUESTED_MODEL_ID: &str = "model://dexterity/byok_cloud/gpt-4o-mini";
const SCOPE_HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PAYLOAD_SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

async fn pg_pool(test_name: &str) -> PgPool {
    let pg = knowledge_pg_support::knowledge_pg().await.unwrap_or_else(|| {
        panic!(
            "PostgreSQL unavailable for {test_name}: MT-006 HBR-PRIV cloud-consent proof requires live Handshake-managed PostgreSQL"
        )
    });
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect isolated cloud consent scope schema")
}

fn scope_for(owner: OwnerAccountId) -> ResourceScope {
    ResourceScope::new(owner, ActorPrincipalId::mint())
}

fn exact_scope_for(slug: &str) -> (ResourceScope, ExactResourceScopeAttribution) {
    let scope = ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint())
        .with_session(AuthenticatedSessionRef::mint())
        .with_access_space(AccessSpaceRef::mint())
        .with_workspace(
            WorkspaceScopeRef::new(format!("workspace-mt006-{slug}"))
                .expect("nonblank workspace scope"),
        );
    let exact = ExactResourceScopeAttribution::try_from_resource_scope(&scope)
        .expect("the fixture supplies all five scope dimensions");
    (scope, exact)
}

#[derive(Debug, Default)]
struct NoopRecorder;

#[async_trait]
impl FlightRecorder for NoopRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(Vec::new())
    }
}

async fn cloud_launch_service(
    pool: &PgPool,
    store: ModelLaneStore,
) -> (
    Arc<OperatorChatLaunchService>,
    LedgerBatcher,
    tokio::task::JoinHandle<Result<(), ProcessLedgerError>>,
    Arc<AtomicUsize>,
) {
    let process_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    process_store
        .apply_migration()
        .await
        .expect("real PostgreSQL ProcessLedger authority is ready");
    let (ledger, writer) = LedgerBatcher::spawn(
        process_store,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 16,
            batch_size: 1,
            flush_interval: Duration::from_millis(1),
        },
    );
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let factory = ProductionModelSessionFactory::new(
        ledger.clone(),
        CloudLaneFactoryConfig {
            anthropic: Some(Arc::new(CountingScopeCloudBuilder {
                calls: provider_calls.clone(),
            })),
            openai: Some(Arc::new(CountingScopeCloudBuilder {
                calls: provider_calls.clone(),
            })),
            official_cli: None,
            official_cli_by_provider: Default::default(),
        },
        None,
    )
    .with_durable_worktree_vm_store(&store);
    let coordinator = SwarmCoordinator::new_with_model_lane_store(
        SwarmConfig::new(RunBudget::defaulted(2)),
        Arc::new(factory),
        Arc::new(RecordingSwarmSink::new()),
        ledger.clone(),
        store,
    );
    let service = Arc::new(OperatorChatLaunchService::new(
        Arc::new(coordinator),
        ModelCatalog::empty(),
        Arc::new(NoopRecorder),
    ));
    (service, ledger, writer, provider_calls)
}

fn single_run_grant(
    slug: &str,
    source_scope: AccountBoundAuthority,
) -> OperatorChatSingleRunCloudConsentGrant {
    let receipt = receipt_id(slug);
    let mut plan = projection_plan(slug, source_scope, Some(receipt.clone()));
    plan.consent_scope = ModelLaneCloudConsentScope::SingleRun;
    plan.lane_id = None;
    plan.model_session_id = None;
    plan.provider_kind = None;
    plan.requested_model_id = None;
    plan.target_bindings.clear();
    let now = chrono::Utc::now();
    OperatorChatSingleRunCloudConsentGrant {
        projection_plan: plan,
        consent_receipt_id: receipt,
        approved_by_ref: "operator://mt006/http-scope-proof".into(),
        approved_at_utc: now.to_rfc3339(),
        valid_from_utc: (now - chrono::Duration::minutes(1)).to_rfc3339(),
        valid_until_utc: (now + chrono::Duration::hours(12)).to_rfc3339(),
        consent_idempotency_key: format!("idem-consent-http-{slug}"),
        diagnostic_payload: json!({"scope_boundary": "http_exact"}),
    }
}

fn empty_launch_request(
    slug: &str,
    source_scope: AccountBoundAuthority,
) -> OperatorChatSingleRunCloudLaunchRequest {
    OperatorChatSingleRunCloudLaunchRequest {
        grant: single_run_grant(slug, source_scope),
        selections: Vec::new(),
    }
}

fn two_lane_launch_request(
    slug: &str,
    source_scope: AccountBoundAuthority,
    working_dirs: [&str; 2],
) -> OperatorChatSingleRunCloudLaunchRequest {
    let selection = |ordinal: usize| OperatorChatSelection {
        lane_kind: OperatorChatLaneKind::Cloud,
        model_id: if ordinal == 1 {
            "gpt-4o-mini".into()
        } else {
            "claude-sonnet-4".into()
        },
        cloud_provider: Some(if ordinal == 1 {
            "openai".into()
        } else {
            "anthropic".into()
        }),
        cli_provider: None,
        working_dir: working_dirs[ordinal - 1].into(),
        worktree_id: Some(format!("wtc-multi-model-orchestration-v1-{ordinal}")),
        prompt: format!("MT-006 exact-scope cloud proof lane {ordinal}"),
        owner_session: "KERNEL_BUILDER-MT006-HTTP".into(),
        parent_session_id: "KERNEL_BUILDER-MT006".into(),
        work_packet_id: Some(WP_ID.into()),
        micro_task_id: Some(MT_ID.into()),
    };
    OperatorChatSingleRunCloudLaunchRequest {
        grant: single_run_grant(slug, source_scope),
        selections: vec![selection(1), selection(2)],
    }
}

async fn start_scoped_server(
    state: OperatorChatState,
    exact: ExactResourceScopeAttribution,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind scoped operator-chat proof listener");
    let addr = listener.local_addr().expect("scoped listener address");
    let product_scope =
        ProductLocalResourceScope::from_exact(exact).expect("valid product-local scope");
    let app = scoped_routes(state).layer(Extension(product_scope));
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve scoped operator-chat proof");
    });
    (format!("http://{addr}"), handle)
}

async fn start_unscoped_server(state: OperatorChatState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind unscoped operator-chat proof listener");
    let addr = listener.local_addr().expect("unscoped listener address");
    let handle = tokio::spawn(async move {
        axum::serve(listener, routes(state))
            .await
            .expect("serve unscoped operator-chat proof");
    });
    (format!("http://{addr}"), handle)
}

fn account_store(pool: &PgPool, scope: &ResourceScope) -> ModelLaneStore {
    ModelLaneStore::new_scoped(pool.clone(), scope.clone())
}

/// The pre-account legacy store: no write scope, so it stamps NULL owners and
/// can only record explicitly unattributed authority.
fn legacy_store(pool: &PgPool) -> ModelLaneStore {
    ModelLaneStore::new(pool.clone())
}

fn approver_for(scope: &ResourceScope) -> AccountBoundAuthority {
    AccountBoundAuthority::from_scope(scope)
}

fn unattributed(reason: &str) -> AccountBoundAuthority {
    AccountBoundAuthority::unattributed(reason)
}

// ---------------------------------------------------------------------------
// Fixtures — every identifier derives from `slug`, so two owners can seed
// structurally identical cloud authority that differs only in ownership.
// ---------------------------------------------------------------------------

fn run_id(slug: &str) -> String {
    format!("run-cloud-{slug}")
}

fn lane_id(slug: &str) -> String {
    format!("lane-cloud-{slug}")
}

fn plan_id(slug: &str) -> String {
    format!("cloud-projection-plan://{}/{}", run_id(slug), lane_id(slug))
}

fn receipt_id(slug: &str) -> String {
    format!("cloud-consent-receipt://{}/{}", run_id(slug), lane_id(slug))
}

fn stream_id(slug: &str) -> String {
    format!("mlane-stream-{}", run_id(slug))
}

fn model_session_id(slug: &str) -> String {
    format!("model-session-{}", lane_id(slug))
}

fn owner_session(slug: &str) -> String {
    // Deliberately a governance ROLE LABEL, exactly like production. It is the
    // value the old code mistook for an owner; keeping it here means every
    // isolation assertion below has to come from the real account columns.
    format!("KERNEL_BUILDER-{slug}")
}

fn fan_out_targets() -> Vec<String> {
    vec![format!("provider://{PROVIDER_KIND}/byok")]
}

fn projection_plan(
    slug: &str,
    source_scope: AccountBoundAuthority,
    authorization_receipt_ref: Option<String>,
) -> NewModelLaneCloudProjectionPlan {
    let run = run_id(slug);
    let lane = lane_id(slug);
    NewModelLaneCloudProjectionPlan {
        projection_plan_id: plan_id(slug),
        run_id: run.clone(),
        trace_id: format!("trace-{run}"),
        lane_id: Some(lane.clone()),
        model_session_id: Some(model_session_id(slug)),
        provider_kind: Some(PROVIDER_KIND.into()),
        requested_model_id: Some(REQUESTED_MODEL_ID.into()),
        scope_hash: SCOPE_HASH.into(),
        source_artifact_refs: vec![
            format!("artifact-store://mt006-priv/{run}/{lane}/context.json"),
            "context-bundle://mt006-priv/cloud-safe".into(),
        ],
        payload_artifact_ref: format!("artifact-store://mt006-priv/{run}/{lane}/payload.json"),
        payload_sha256: PAYLOAD_SHA256.into(),
        redaction_policy_ref: "redaction-policy://mt006-priv/cloud-safe".into(),
        redaction_summary: "workspace-local secrets and local-only memory are excluded".into(),
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        provider_profile_ref: format!("provider-profile://mt006-priv/{PROVIDER_KIND}"),
        fan_out_targets: fan_out_targets(),
        export_delegation: CloudExportDelegation {
            audience_refs: fan_out_targets(),
            source_scope,
            authorization_receipt_ref,
        },
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
        status: ModelLaneCloudProjectionPlanStatus::Active,
        event_ledger_stream_id: stream_id(slug),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: owner_session(slug),
        idempotency_key: format!("idem-projection-{run}-{lane}"),
        created_at_utc: "2026-08-04T09:00:00Z".into(),
        user_manual_behavior_ref: USERMANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "internal_diagnostics": "deferred: native internal_diagnostics surface ships separately",
            "palmistry": "deferred: external watcher links by run_id/lane_id when available",
            "locus": format!("locus://wp1/mt006/{run}/{lane}")
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn consent_receipt(
    slug: &str,
    projection_plan_id: &str,
    projection_plan_hash: &str,
    approver: AccountBoundAuthority,
    approved_by_ref: &str,
) -> NewModelLaneCloudConsentReceipt {
    let run = run_id(slug);
    let lane = lane_id(slug);
    NewModelLaneCloudConsentReceipt {
        consent_receipt_id: receipt_id(slug),
        projection_plan_id: projection_plan_id.into(),
        projection_plan_hash: projection_plan_hash.into(),
        run_id: run.clone(),
        trace_id: format!("trace-{run}"),
        lane_id: Some(lane.clone()),
        model_session_id: Some(model_session_id(slug)),
        provider_kind: Some(PROVIDER_KIND.into()),
        requested_model_id: Some(REQUESTED_MODEL_ID.into()),
        scope_hash: SCOPE_HASH.into(),
        consent_scope: ModelLaneCloudConsentScope::SingleLane,
        target_bindings: vec![],
        retention_policy: ModelLaneCloudRetentionPolicy::NoTrainingEphemeral,
        export_posture: ModelLaneCloudExportPosture::RedactedContextOnly,
        fan_out_targets: fan_out_targets(),
        approved: true,
        approver,
        approved_by_ref: approved_by_ref.into(),
        approved_at_utc: "2026-08-04T09:00:10Z".into(),
        // Wide enough to be current on any wall clock during the test run. The
        // BOUNDED window is an operator-chat mint-site policy
        // (`OPERATOR_CHAT_CLOUD_CONSENT_VALIDITY_HOURS`), not a storage rule, so
        // it is asserted there rather than smuggled in here.
        valid_from_utc: "2020-01-01T00:00:00Z".into(),
        valid_until_utc: "2099-01-01T00:00:00Z".into(),
        revoked_at_utc: None,
        revocation_ref: None,
        revocation_input_hash: None,
        status: ModelLaneCloudConsentReceiptStatus::Approved,
        event_ledger_stream_id: stream_id(slug),
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: TASK_BOARD_ID.into(),
        owner_session: owner_session(slug),
        idempotency_key: format!("idem-consent-{run}-{lane}"),
        created_at_utc: "2026-08-04T09:00:15Z".into(),
        user_manual_behavior_ref: USERMANUAL_BEHAVIOR.into(),
        diagnostic_payload: json!({
            "flight_recorder": "EventLedger",
            "provider_call_attempted": false,
            "locus": format!("locus://wp1/mt006/{run}/{lane}")
        }),
    }
}

fn locus(slug: &str) -> ModelLaneLocusBinding {
    ModelLaneLocusBinding {
        work_packet_id: WP_ID.into(),
        micro_task_id: MT_ID.into(),
        task_board_id: Some(TASK_BOARD_ID.into()),
        coordinator_session_id: format!("coordinator-session-{}", run_id(slug)),
        session_id: format!("session-{}", lane_id(slug)),
        model_session_id: model_session_id(slug),
        owner_session: owner_session(slug),
        locus_binding_ref: format!("locus://wp1/mt006/{}/{}", run_id(slug), lane_id(slug)),
    }
}

/// A cloud run + lane bound to `slug`'s projection/consent authority. `plan_ref`
/// and `receipt_ref` are explicit so a test can deliberately point a launch at
/// ANOTHER account's authority.
fn cloud_run_lane(
    slug: &str,
    status: ModelLaneStatus,
    plan_ref: &str,
    receipt_ref: &str,
) -> (NewModelLaneRun, NewModelLane) {
    let run = run_id(slug);
    let lane = lane_id(slug);
    (
        NewModelLaneRun {
            run_id: run.clone(),
            trace_id: format!("trace-{run}"),
            run_span_id: format!("span-{run}"),
            coordinator_session_id: format!("coordinator-session-{run}"),
            routing_policy: "cloud_plan_local_execute".into(),
            context_bundle_id: format!("context-bundle://mt006-priv/{run}/bootstrap"),
            lane_ids: vec![lane.clone()],
            event_ledger_stream_id: stream_id(slug),
            artifact_namespace: format!("artifact://model-lane/mt006-priv/{run}"),
            projection_plan_ref: Some(plan_ref.into()),
            consent_receipt_ref: Some(receipt_ref.into()),
            work_packet_id: Some(WP_ID.into()),
            micro_task_id: Some(MT_ID.into()),
            task_board_id: Some(TASK_BOARD_ID.into()),
            owner_session: owner_session(slug),
            idempotency_key: format!("idem-run-{run}"),
            replay_order_key: format!("00000000/{run}/run"),
            replay_after_event_ledger_seq: None,
            recovery_state: ModelLaneRecoveryState::Restartable,
            failstate_code: None,
            reason_ref: None,
            recovery_hint_ref: Some(
                "usermanual://model-lane-cloud-projection-consent#recovery".into(),
            ),
            locus_binding: Some(locus(slug)),
            memory_pack_ref: format!("memory-pack://fems/mt006-priv/{run}"),
            memory_pack_hash: PAYLOAD_SHA256.into(),
            determinism_mode: "deterministic_replay".into(),
            budget_summary_ref: format!("budget://mt006-priv/{run}"),
            selected_model_id: Some(REQUESTED_MODEL_ID.into()),
            candidate_model_ids: vec![REQUESTED_MODEL_ID.into()],
            procedural_review_status: "cloud_projection_consent_preflighted".into(),
            truncation_warning_ref: None,
            rejection_reason_refs: vec![],
        },
        NewModelLane {
            lane_id: lane.clone(),
            run_id: run.clone(),
            trace_id: format!("trace-{run}"),
            lane_span_id: format!("span-{lane}"),
            event_ledger_stream_id: stream_id(slug),
            kind: ModelLaneKind::CloudModel,
            role: "cloud-review-lane".into(),
            backend: "cloud_lane_openai".into(),
            model_id: Some(REQUESTED_MODEL_ID.into()),
            session_id: format!("session-{lane}"),
            model_session_id: model_session_id(slug),
            adapter_id: "openai_byok".into(),
            runtime_binding: RuntimeBinding::Cloud,
            launch_authority: LaunchAuthority::CloudLane,
            provider_kind: ModelLaneProviderKind::OpenAi,
            capability_token_ids: vec!["capability://dexterity/cloud-generate".into()],
            effective_capability_snapshot_ref: Some(format!("capability-snapshot://{lane}")),
            capability_negotiation_ref: Some(format!("capability-negotiation://{lane}")),
            provider_feature_profile_ref: Some(format!(
                "provider-profile://mt006-priv/{PROVIDER_KIND}"
            )),
            requested_execution_policy_ref: Some(format!("execution-policy://requested/{lane}")),
            effective_execution_policy_ref: Some(format!("execution-policy://effective/{lane}")),
            projection_plan_ref: Some(plan_ref.into()),
            consent_receipt_ref: Some(receipt_ref.into()),
            tool_gate_decision_refs: vec!["toolgate://mt006-priv/cloud-read-context".into()],
            status,
            recovery_state: ModelLaneRecoveryState::Restartable,
            heartbeat_at_utc: Some("2026-08-04T09:01:00Z".into()),
            lease_expires_at_utc: Some("2026-08-04T09:10:00Z".into()),
            reclaim_after_utc: Some("2026-08-04T09:11:00Z".into()),
            restart_generation: 0,
            cancellation_ref: Some(format!("cancel-token://{lane}")),
            reclaim_policy_ref: Some("reclaim-policy://mt006-priv/cloud".into()),
            terminal_status_mapping_ref: Some("terminal-status://mt006-priv/cloud".into()),
            process_ownership_ref: Some(format!("process-ledger://{lane}")),
            no_os_process_reason_ref: None,
            backpressure_ref: None,
            loop_counter_ref: Some("loop-counter://mt006-priv".into()),
            last_runtime_status_ref: Some("runtime-status://cloud-ready".into()),
            last_recovery_event_ref: None,
            failstate_code: None,
            startup_failure_ref: None,
            reason_ref: None,
            recovery_hint_ref: Some(
                "usermanual://model-lane-cloud-projection-consent#recovery".into(),
            ),
            work_packet_id: Some(WP_ID.into()),
            micro_task_id: Some(MT_ID.into()),
            task_board_id: Some(TASK_BOARD_ID.into()),
            owner_session: owner_session(slug),
            locus_binding: Some(locus(slug)),
        },
    )
}

/// Seed a coherent ProjectionPlan + ConsentReceipt pair owned by `scope`.
async fn seed_cloud_authority(store: &ModelLaneStore, slug: &str, scope: &ResourceScope) {
    let plan = store
        .record_cloud_projection_plan(projection_plan(
            slug,
            approver_for(scope),
            Some(receipt_id(slug)),
        ))
        .await
        .unwrap_or_else(|error| panic!("seed cloud ProjectionPlan for {slug}: {error}"));
    store
        .record_cloud_consent_receipt(consent_receipt(
            slug,
            &plan.projection_plan_id,
            &plan.projection_plan_hash,
            approver_for(scope),
            "operator://mt006-priv/approval",
        ))
        .await
        .unwrap_or_else(|error| panic!("seed cloud ConsentReceipt for {slug}: {error}"));
}

/// Read a row's stored scope columns with NO owner predicate: the deliberate
/// simulation of layer 1 being deleted by a future edit.
async fn stored_scope_without_predicate(
    pool: &PgPool,
    table: &str,
    key_column: &str,
    key: &str,
) -> StoredResourceScope {
    let sql =
        format!("SELECT {RESOURCE_SCOPE_SELECT_COLUMNS} FROM {table} WHERE {key_column} = $1");
    let row = sqlx::query(&sql)
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|error| panic!("unpredicated read of {table}.{key_column}={key}: {error}"));
    stored_resource_scope_from_row(&row).expect("decode stored scope columns")
}

async fn lane_row_refs(pool: &PgPool, lane: &str) -> (String, String, String) {
    let row: (serde_json::Value,) =
        sqlx::query_as("SELECT record_json FROM model_lanes WHERE lane_id = $1")
            .bind(lane)
            .fetch_one(pool)
            .await
            .unwrap_or_else(|error| panic!("read lane row {lane}: {error}"));
    let record = row.0;
    (
        record["projection_plan_ref"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        record["consent_receipt_ref"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        record["status"].as_str().unwrap_or_default().to_string(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CloudAuthorityCounts {
    plans: i64,
    receipts: i64,
    events: i64,
}

async fn cloud_authority_counts(pool: &PgPool) -> CloudAuthorityCounts {
    let plans: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_cloud_projection_plans")
        .fetch_one(pool)
        .await
        .expect("count cloud projection plans");
    let receipts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM model_lane_cloud_consent_receipts")
            .fetch_one(pool)
            .await
            .expect("count cloud consent receipts");
    let events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE aggregate_type = ANY($1)",
    )
    .bind(vec![
        "model_lane_cloud_projection_plan",
        "model_lane_cloud_consent_receipt",
        "model_lane_cloud_consent_denial",
        "model_lane_terminal",
    ])
    .fetch_one(pool)
    .await
    .expect("count cloud EventLedger rows");
    CloudAuthorityCounts {
        plans,
        receipts,
        events,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unscoped_cloud_grant_fails_before_postgres_or_provider_authority() {
    let pool = pg_pool("unscoped HTTP cloud grant denial").await;
    let (service, ledger, writer, provider_calls) =
        cloud_launch_service(&pool, legacy_store(&pool)).await;
    let denied = service
        .grant_single_run_cloud_consent(single_run_grant(
            "unscoped-http",
            unattributed("LEGACY_UNSCOPED_HTTP"),
        ))
        .await
        .expect_err("an unscoped store must not mint cloud authority");
    assert!(
        denied.to_string().contains("RESOURCE_SCOPE_REQUIRED"),
        "the denial must be stable and must not disclose a resource id: {denied}"
    );
    assert_eq!(
        cloud_authority_counts(&pool).await,
        CloudAuthorityCounts {
            plans: 0,
            receipts: 0,
            events: 0,
        },
        "scope denial must happen before ProjectionPlan, ConsentReceipt, or EventLedger authority"
    );
    assert_eq!(
        provider_calls.load(Ordering::SeqCst),
        0,
        "an unscoped grant must fail before a provider builder side effect"
    );
    let state = OperatorChatState::production().with_launch_service(service);
    let (base, server) = start_unscoped_server(state).await;
    let response = reqwest::Client::new()
        .post(format!(
            "{base}/operator-chat/cloud/single-run/grant-launch"
        ))
        .json(&empty_launch_request(
            "unscoped-http-route",
            unattributed("UNSCOPED_HTTP_ROUTE"),
        ))
        .send()
        .await
        .expect("send unscoped Axum cloud launch");
    assert_eq!(response.status().as_u16(), 403);
    let body: serde_json::Value = response.json().await.expect("unscoped denial JSON");
    assert_eq!(body["error"], "resource_scope_unavailable");
    assert_eq!(
        cloud_authority_counts(&pool).await,
        CloudAuthorityCounts {
            plans: 0,
            receipts: 0,
            events: 0,
        },
        "unscoped Axum denial must leave PostgreSQL and EventLedger untouched"
    );
    assert_eq!(provider_calls.load(Ordering::SeqCst), 0);
    server.abort();
    ledger.begin_close();
    let outcome = drain_and_join_ledger_writer(&ledger, writer, Duration::from_secs(5)).await;
    assert!(matches!(outcome, LedgerDrainJoinOutcome::Flushed));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_cloud_launch_binds_extracted_exact_scope_to_the_durable_store() {
    let pool = pg_pool("HTTP cloud launch exact scope binding").await;
    let (store_scope, store_exact) = exact_scope_for("http-store");
    let (foreign_scope, foreign_exact) = exact_scope_for("http-foreign");
    let (service, ledger, writer, provider_calls) =
        cloud_launch_service(&pool, account_store(&pool, &store_scope)).await;
    let state = OperatorChatState::production().with_launch_service(service);

    let (foreign_base, foreign_server) =
        start_scoped_server(state.clone(), foreign_exact.clone()).await;
    let foreign_response = reqwest::Client::new()
        .post(format!(
            "{foreign_base}/operator-chat/cloud/single-run/grant-launch"
        ))
        .json(&empty_launch_request(
            "http-foreign",
            approver_for(&foreign_scope),
        ))
        .send()
        .await
        .expect("send foreign scoped cloud launch");
    assert_eq!(foreign_response.status().as_u16(), 400);
    let foreign_body: serde_json::Value =
        foreign_response.json().await.expect("foreign denial JSON");
    assert_eq!(foreign_body["error"], "bad_request");
    assert!(
        foreign_body["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("RESOURCE_SCOPE_MISMATCH")),
        "the handler must compare its extracted exact scope with the store: {foreign_body}"
    );
    assert_eq!(
        cloud_authority_counts(&pool).await,
        CloudAuthorityCounts {
            plans: 0,
            receipts: 0,
            events: 0,
        }
    );
    assert_eq!(
        provider_calls.load(Ordering::SeqCst),
        0,
        "foreign HTTP scope must fail before provider dispatch"
    );
    foreign_server.abort();

    // Positive control: matching exact scope drives the complete shipped
    // Axum -> service -> coordinator -> provider -> PostgreSQL path.
    let checkout_one = tempfile::tempdir().expect("create first real checkout directory");
    let checkout_two = tempfile::tempdir().expect("create second real checkout directory");
    let checkout_one_path = checkout_one.path().to_string_lossy();
    let checkout_two_path = checkout_two.path().to_string_lossy();
    let (owner_base, owner_server) = start_scoped_server(state.clone(), store_exact).await;
    let owner_response = reqwest::Client::new()
        .post(format!(
            "{owner_base}/operator-chat/cloud/single-run/grant-launch"
        ))
        .json(&two_lane_launch_request(
            "http-owner",
            approver_for(&store_scope),
            [checkout_one_path.as_ref(), checkout_two_path.as_ref()],
        ))
        .send()
        .await
        .expect("send owner scoped cloud launch");
    let owner_status = owner_response.status().as_u16();
    let owner_body: serde_json::Value = owner_response.json().await.expect("owner launch JSON");
    assert_eq!(owner_status, 200, "owner launch failed: {owner_body}");
    assert_eq!(
        owner_body["instance_ids"]
            .as_array()
            .expect("launched instance ids")
            .len(),
        2
    );
    assert_eq!(
        provider_calls.load(Ordering::SeqCst),
        2,
        "the exact owner reaches each enumerated provider target once"
    );
    let lane_rows: Vec<(serde_json::Value,)> =
        sqlx::query_as("SELECT record_json FROM model_lanes WHERE run_id = $1 ORDER BY lane_id")
            .bind(run_id("http-owner"))
            .fetch_all(&pool)
            .await
            .expect("read exact-owner lane identities");
    assert_eq!(lane_rows.len(), 2);
    let distinct = |field: &str| {
        lane_rows
            .iter()
            .map(|(record,)| {
                record[field]
                    .as_str()
                    .unwrap_or_else(|| panic!("lane record requires {field}"))
                    .to_string()
            })
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        distinct("lane_id").len(),
        2,
        "lane identities stay distinct"
    );
    assert_eq!(
        distinct("model_session_id").len(),
        2,
        "model-session identities stay distinct"
    );
    assert_eq!(
        distinct("model_id"),
        BTreeSet::from([
            "model://dexterity/byok_cloud/gpt-4o-mini".to_string(),
            "model://dexterity/byok_cloud/claude-sonnet-4".to_string(),
        ]),
        "model identities stay distinct"
    );
    assert_eq!(
        distinct("provider_kind"),
        BTreeSet::from(["openai".to_string(), "anthropic".to_string()]),
        "provider identities stay distinct"
    );
    let owner_counts = cloud_authority_counts(&pool).await;
    assert_eq!(
        (owner_counts.plans, owner_counts.receipts),
        (1, 1),
        "one ProjectionPlan and one ConsentReceipt are durable"
    );
    assert!(
        owner_counts.events >= 2,
        "the exact owner launch must append ProjectionPlan/ConsentReceipt EventLedger authority"
    );
    let stored_scope = stored_scope_without_predicate(
        &pool,
        "model_lane_cloud_projection_plans",
        "projection_plan_id",
        owner_body["projection_plan_id"]
            .as_str()
            .expect("projection plan id"),
    )
    .await;
    assert_eq!(
        stored_scope,
        StoredResourceScope::from(&store_scope),
        "the HTTP owner scope must survive the full launch into PostgreSQL"
    );

    let receipt = owner_body["consent_receipt_id"]
        .as_str()
        .expect("consent receipt id")
        .to_string();
    let before_foreign_revoke = cloud_authority_counts(&pool).await;
    let (foreign_revoke_base, foreign_revoke_server) =
        start_scoped_server(state, foreign_exact).await;
    let foreign_revoke = reqwest::Client::new()
        .post(format!(
            "{foreign_revoke_base}/operator-chat/cloud/single-run/revoke"
        ))
        .json(&json!({
            "consent_receipt_id": receipt,
            "revoked_by_ref": "operator://mt006/foreign-context",
            "reason": "foreign context must not revoke"
        }))
        .send()
        .await
        .expect("send foreign scoped revoke");
    assert_eq!(foreign_revoke.status().as_u16(), 400);
    let foreign_revoke_body: serde_json::Value = foreign_revoke
        .json()
        .await
        .expect("foreign revoke denial JSON");
    assert!(
        foreign_revoke_body["detail"]
            .as_str()
            .is_some_and(|detail| detail.contains("RESOURCE_SCOPE_MISMATCH")),
        "foreign context must fail at the exact scope/store boundary: {foreign_revoke_body}"
    );
    let still_running: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_lanes WHERE run_id = $1 AND status = 'running'",
    )
    .bind(run_id("http-owner"))
    .fetch_one(&pool)
    .await
    .expect("count owner lanes after foreign revoke denial");
    assert_eq!(
        still_running, 2,
        "foreign revoke must not mutate owner lanes"
    );
    let receipt_state: (String, bool, Option<String>) = sqlx::query_as(
        "SELECT status, approved, revoked_at_utc \
         FROM model_lane_cloud_consent_receipts \
         WHERE consent_receipt_id = $1",
    )
    .bind(&receipt)
    .fetch_one(&pool)
    .await
    .expect("read receipt after foreign revoke denial");
    assert_eq!(receipt_state.0, "approved");
    assert!(receipt_state.1);
    assert_eq!(receipt_state.2, None);
    assert_eq!(
        cloud_authority_counts(&pool).await,
        before_foreign_revoke,
        "foreign revoke must not mutate ProjectionPlan, ConsentReceipt, or EventLedger authority"
    );
    assert_eq!(provider_calls.load(Ordering::SeqCst), 2);
    foreign_revoke_server.abort();

    let owner_revoke = reqwest::Client::new()
        .post(format!(
            "{owner_base}/operator-chat/cloud/single-run/revoke"
        ))
        .json(&json!({
            "consent_receipt_id": receipt,
            "revoked_by_ref": "operator://mt006/owner-context",
            "reason": "owner ends cloud grant"
        }))
        .send()
        .await
        .expect("send owner scoped revoke");
    assert_eq!(owner_revoke.status().as_u16(), 200);
    let owner_revoke_body: serde_json::Value =
        owner_revoke.json().await.expect("owner revoke JSON");
    assert_eq!(
        owner_revoke_body["cancelled_lanes"]
            .as_array()
            .expect("cancelled lanes")
            .len(),
        2,
        "the exact owner revokes every covered live lane"
    );
    owner_server.abort();
    ledger.begin_close();
    let outcome = drain_and_join_ledger_writer(&ledger, writer, Duration::from_secs(5)).await;
    assert!(
        matches!(outcome, LedgerDrainJoinOutcome::Flushed),
        "HTTP scope proof must leave no detached ledger writer: {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// 1. HBR-PRIV-001/002 — cross-account read AND reuse
// ---------------------------------------------------------------------------

#[tokio::test]
async fn two_accounts_cannot_read_or_reuse_each_others_cloud_consent_authority() {
    let pool = pg_pool("cross-account cloud consent isolation").await;

    let alice_account = OwnerAccountId::mint();
    let bob_account = OwnerAccountId::mint();
    assert_ne!(alice_account, bob_account);

    let alice_scope = scope_for(alice_account);
    let bob_scope = scope_for(bob_account);
    let alice = account_store(&pool, &alice_scope);
    let bob = account_store(&pool, &bob_scope);

    seed_cloud_authority(&alice, "alice", &alice_scope).await;
    seed_cloud_authority(&bob, "bob", &bob_scope).await;

    // -- POSITIVE CONTROL ---------------------------------------------------
    // Without this every negative below could pass because nothing was written
    // or because enforcement is simply "deny everything".
    let own = alice
        .replay_cloud_consent_authority(&run_id("alice"))
        .await
        .expect("the owning account must replay its own cloud authority");
    assert_eq!(own.projection_plans.len(), 1, "alice's own plan is visible");
    assert_eq!(
        own.consent_receipts.len(),
        1,
        "alice's own receipt is visible"
    );
    assert_eq!(own.projection_plans[0].projection_plan_id, plan_id("alice"));

    // -- LAYER 1 (SQL): cross-account READ ----------------------------------
    //
    // FALSIFIABILITY (inverted, run, observed, restored): expecting `1` here
    // instead of `0` produced
    //   assertion `left == right` failed: alice must not see bob's cloud
    //   projection plans / left: 0 / right: 1
    // so the emptiness is enforcement, not an unwritten row (the positive
    // control above already proved the rows exist).
    let across = alice
        .replay_cloud_consent_authority(&run_id("bob"))
        .await
        .expect("a scoped replay of another account's run must be empty, not an error");
    assert_eq!(
        across.projection_plans.len(),
        0,
        "alice must not see bob's cloud projection plans"
    );
    assert_eq!(
        across.consent_receipts.len(),
        0,
        "alice must not see bob's cloud consent receipts"
    );

    // And symmetric, so the result is not an artifact of insertion order.
    let reverse = bob
        .replay_cloud_consent_authority(&run_id("alice"))
        .await
        .expect("scoped replay");
    assert_eq!(reverse.projection_plans.len(), 0);
    assert_eq!(reverse.consent_receipts.len(), 0);

    // -- LAYER 1: cross-account REUSE (the sharper failure) ------------------
    // Reading someone else's receipt is a disclosure; USING it is an
    // authorization break. Alice attaches bob's durable plan/consent refs to her
    // own cloud lane and launches.
    let (run, lane) = cloud_run_lane(
        "alice-steal",
        ModelLaneStatus::Ready,
        &plan_id("bob"),
        &receipt_id("bob"),
    );
    //
    // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)` produced
    //   InvalidInput("CX-MM-007 cloud lane launch denied for run_id
    //   run-cloud-alice-steal lane_id lane-cloud-alice-steal: final cloud launch
    //   insertion fence denied: invalid model lane input: ProjectionPlan
    //   cloud-projection-plan://run-cloud-bob/lane-cloud-bob is not durable")
    let denied = alice
        .record_prepared_launch((run, lane))
        .await
        .expect_err("alice must not be able to launch on bob's cloud consent");
    let message = denied.to_string();
    assert!(
        message.contains("CX-MM-007"),
        "cross-account cloud reuse must fail closed with CX-MM-007: {message}"
    );
    assert!(
        message.contains("is not durable"),
        "bob's authority must be unresolvable to alice, not merely rejected later: {message}"
    );

    // A receipt cannot be minted against another account's plan either, so the
    // denial is not a launch-only check.
    let cross_receipt = bob
        .record_cloud_consent_receipt(consent_receipt(
            "bob-steal",
            &plan_id("alice"),
            SCOPE_HASH,
            approver_for(&bob_scope),
            "operator://mt006-priv/approval",
        ))
        .await
        // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)`
        // produced AuthorityDenied("CX-MM-007 ProjectionPlan
        // cloud-projection-plan://run-cloud-alice/lane-cloud-alice is not durable")
        .expect_err("bob must not be able to consent to alice's projection plan");
    assert!(
        matches!(cross_receipt, ModelLaneError::AuthorityDenied(_)),
        "cross-account consent minting must be an authority denial: {cross_receipt}"
    );

    // Nor can an account claim to be another account when writing.
    let forged = alice
        .record_cloud_projection_plan(projection_plan(
            "alice-forge",
            approver_for(&bob_scope),
            None,
        ))
        .await
        // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)`
        // produced AuthorityDenied("CX-MM-007
        // ProjectionPlan.export_delegation.source_scope names an owning account
        // this store is not authorized to write as")
        .expect_err("alice must not stamp bob as the export source scope");
    assert!(
        forged.to_string().contains("not authorized to write as"),
        "server-derived identity must reject a client-claimed owner: {forged}"
    );

    // -- LAYER 2 (post-deserialization) -------------------------------------
    // Simulate the SQL predicate being dropped. The row comes back; the second
    // layer must still refuse it with the stable reason code.
    let bobs_plan_scope = stored_scope_without_predicate(
        &pool,
        "model_lane_cloud_projection_plans",
        "projection_plan_id",
        &plan_id("bob"),
    )
    .await;
    assert_eq!(
        ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(alice_account))
            .authorize_row(&bobs_plan_scope)
            // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)`
            // produced OwnerMismatch { requested: OwnerAccountId(019fcd38-2fbc-
            // 76a1-85c6-d5511d6a2cbe), stored: OwnerAccountId(019fcd38-2fbc-76a1-
            // 85c6-d561133e76eb) }
            .expect_err("layer 2 must deny a cross-account projection plan")
            .reason_code(),
        "RESOURCE_SCOPE_OWNER_MISMATCH"
    );

    let bobs_receipt_scope = stored_scope_without_predicate(
        &pool,
        "model_lane_cloud_consent_receipts",
        "consent_receipt_id",
        &receipt_id("bob"),
    )
    .await;
    assert_eq!(
        ResourceAccessContext::for_reader(ResourceScopeQuery::for_owner(alice_account))
            .authorize_row(&bobs_receipt_scope)
            .expect_err("layer 2 must deny a cross-account consent receipt")
            .reason_code(),
        "RESOURCE_SCOPE_OWNER_MISMATCH"
    );

    // -- The write path really did stamp distinct owners --------------------
    // Otherwise every assertion above would be testing nothing.
    assert_eq!(
        bobs_plan_scope.owner_account_id,
        Some(bob_account),
        "bob's projection plan row must be stamped with bob's account"
    );
    assert_eq!(bobs_receipt_scope.owner_account_id, Some(bob_account));
    let alices_receipt_scope = stored_scope_without_predicate(
        &pool,
        "model_lane_cloud_consent_receipts",
        "consent_receipt_id",
        &receipt_id("alice"),
    )
    .await;
    assert_eq!(alices_receipt_scope.owner_account_id, Some(alice_account));
    assert_ne!(
        alices_receipt_scope.owner_account_id,
        bobs_receipt_scope.owner_account_id
    );
    assert!(
        alices_receipt_scope.actor_principal_id.is_some(),
        "HBR-PRIV-005 keeps the acting principal separately recoverable from the owner"
    );
}

// ---------------------------------------------------------------------------
// 2. HBR-PRIV-006 — revocation and context switch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn revoked_cloud_consent_refuses_relaunch_and_leaves_the_inflight_lane_pinned() {
    let pool = pg_pool("cloud consent revocation and context switch").await;

    let account = OwnerAccountId::mint();
    let scope = scope_for(account);
    let store = account_store(&pool, &scope);

    seed_cloud_authority(&store, "revoke", &scope).await;

    // An in-flight cloud lane running under that grant.
    let (run, lane) = cloud_run_lane(
        "revoke",
        ModelLaneStatus::Running,
        &plan_id("revoke"),
        &receipt_id("revoke"),
    );
    let (_stored_run, stored_lane) = store
        .record_prepared_launch((run, lane))
        .await
        .expect("a valid grant must allow the cloud lane to launch");
    assert_eq!(stored_lane.status, ModelLaneStatus::Running);
    let original_plan_ref = stored_lane
        .projection_plan_ref
        .clone()
        .expect("in-flight lane carries its projection plan");
    let original_receipt_ref = stored_lane
        .consent_receipt_ref
        .clone()
        .expect("in-flight lane carries its consent receipt");

    // -- Revoke -------------------------------------------------------------
    let cancelled = store
        .test_finalize_cloud_consent_revocation(
            &receipt_id("revoke"),
            "operator://mt006-priv/revoker",
            "operator withdrew cloud consent",
            &BTreeSet::from([lane_id("revoke")]),
        )
        .await
        .expect("revocation must terminate the covered lanes");
    assert_eq!(
        cancelled.len(),
        1,
        "the covered in-flight lane is cancelled"
    );
    assert_eq!(cancelled[0].status, ModelLaneStatus::Cancelled);
    assert_eq!(cancelled[0].failstate_code.as_deref(), Some("CX-MM-007"));

    // -- A SUBSEQUENT launch attempt is refused -----------------------------
    //
    // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)` produced
    //   InvalidInput("CX-MM-007 cloud lane launch denied for run_id
    //   run-cloud-revoke lane_id lane-cloud-revoke: final cloud launch insertion
    //   fence denied: invalid model lane input: ConsentReceipt is revoked")
    let (retry_run, retry_lane) = cloud_run_lane(
        "revoke",
        ModelLaneStatus::Ready,
        &plan_id("revoke"),
        &receipt_id("revoke"),
    );
    let refused = store
        .record_prepared_launch((retry_run, retry_lane))
        .await
        .expect_err("a revoked consent receipt must not authorize another launch");
    let refused_message = refused.to_string();
    assert!(
        refused_message.contains("CX-MM-007"),
        "post-revocation launch must fail closed with CX-MM-007: {refused_message}"
    );
    assert!(
        refused_message.contains("ConsentReceipt is revoked"),
        "the denial must name revocation as the cause, not a generic 'not approved': {refused_message}"
    );
    // ...and the durable row agrees with the gate rather than still claiming it
    // is an approved authorization.
    let receipt_row: (String, bool) = sqlx::query_as(
        "SELECT status, approved FROM model_lane_cloud_consent_receipts WHERE consent_receipt_id = $1",
    )
    .bind(receipt_id("revoke"))
    .fetch_one(&pool)
    .await
    .expect("read the revoked receipt row");
    assert_eq!(receipt_row.0, "revoked");
    assert!(
        !receipt_row.1,
        "a revoked receipt must not keep claiming approved: true"
    );

    // -- CONTEXT SWITCH: a NEW valid grant must not adopt the old lane -------
    // The operator grants fresh consent for the same account. The already-
    // terminated lane must stay pinned to the context it actually ran under; if
    // it were silently retargeted, its audit trail would claim it ran under a
    // grant that did not exist at the time.
    seed_cloud_authority(&store, "revoke-next", &scope).await;
    let (pinned_plan, pinned_receipt, pinned_status) =
        lane_row_refs(&pool, &lane_id("revoke")).await;
    assert_eq!(
        pinned_plan, original_plan_ref,
        "the cancelled lane must keep its ORIGINAL projection plan, not the newer grant"
    );
    // FALSIFIABILITY (inverted, run, observed, restored): expecting
    // `receipt_id("revoke-next")` here produced
    //   assertion `left == right` failed / left:
    //   "cloud-consent-receipt://run-cloud-revoke/lane-cloud-revoke" / right:
    //   "cloud-consent-receipt://run-cloud-revoke-next/lane-cloud-revoke-next"
    // i.e. the lane really is pinned to the context it ran under.
    assert_eq!(
        pinned_receipt, original_receipt_ref,
        "the cancelled lane must keep its ORIGINAL consent receipt, not the newer grant"
    );
    assert_ne!(
        pinned_receipt,
        receipt_id("revoke-next"),
        "a new grant must not retarget an already-run lane"
    );
    assert_eq!(
        pinned_status, "cancelled",
        "the revoked lane stays terminal under the new grant"
    );

    // The new grant is genuinely usable — otherwise "pinned" could just mean
    // "revocation broke the whole account".
    let (fresh_run, fresh_lane) = cloud_run_lane(
        "revoke-next",
        ModelLaneStatus::Ready,
        &plan_id("revoke-next"),
        &receipt_id("revoke-next"),
    );
    let fresh = store
        .record_prepared_launch((fresh_run, fresh_lane))
        .await
        .expect("a fresh grant must still authorize a new lane after a revocation");
    assert_eq!(
        fresh.1.consent_receipt_ref.as_deref(),
        Some(receipt_id("revoke-next").as_str())
    );
}

// ---------------------------------------------------------------------------
// 3. HBR-PRIV-007 — audience, source scope and authorization provenance
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cloud_projection_records_audience_scope_and_authorization_provenance() {
    let pool = pg_pool("cloud projection delegation provenance").await;

    let account = OwnerAccountId::mint();
    let scope = scope_for(account);
    let store = account_store(&pool, &scope);

    // -- POSITIVE: the delegation record is durable and complete ------------
    seed_cloud_authority(&store, "delegation", &scope).await;
    let replay = store
        .replay_cloud_consent_authority(&run_id("delegation"))
        .await
        .expect("replay the delegation authority");
    let plan = &replay.projection_plans[0];
    assert_eq!(
        plan.export_delegation.audience_refs,
        fan_out_targets(),
        "the audience the export may reach is recorded, not implied"
    );
    assert_eq!(
        plan.export_delegation.source_scope.owner_account_id(),
        Some(account),
        "the LOCAL visibility the export derives from is account-bound"
    );
    assert_eq!(
        plan.export_delegation.authorization_receipt_ref.as_deref(),
        Some(receipt_id("delegation").as_str()),
        "the receipt that authorizes the delegation is named by the plan"
    );
    assert_eq!(
        replay.consent_receipts[0].approver.owner_account_id(),
        Some(account),
        "the approver is a typed account, not a formatted role label"
    );

    // -- NEGATIVE: the audience may not widen beyond the disclosed fan-out ---
    //
    // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)` produced
    //   InvalidInput("export_delegation.audience_refs must not widen beyond
    //   fan_out_targets: provider://anthropic/byok is not a disclosed fan-out
    //   target")
    let mut widened = projection_plan("delegation-widen", approver_for(&scope), None);
    widened
        .export_delegation
        .audience_refs
        .push("provider://anthropic/byok".into());
    let widening_error = store
        .record_cloud_projection_plan(widened)
        .await
        .expect_err("a remote export must not name an undisclosed audience");
    assert!(
        widening_error
            .to_string()
            .contains("must not widen beyond fan_out_targets"),
        "the denial must name the non-widening rule: {widening_error}"
    );

    // -- NEGATIVE: a plan may not be paired with a receipt it did not name ---
    // The plan for `delegation-crosslink` declares it is authorized by the
    // `delegation` receipt, but the receipt written against it is its own. The
    // pair must be refused at every gate that consults it.
    let crosslinked = store
        .record_cloud_projection_plan(projection_plan(
            "delegation-crosslink",
            approver_for(&scope),
            Some(receipt_id("delegation")),
        ))
        .await
        .expect("an incoherent plan is still durable evidence");
    store
        .record_cloud_consent_receipt(consent_receipt(
            "delegation-crosslink",
            &crosslinked.projection_plan_id,
            &crosslinked.projection_plan_hash,
            approver_for(&scope),
            "operator://mt006-priv/approval",
        ))
        .await
        .expect("an incoherent receipt is still durable evidence");
    // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)` produced
    //   AuthorityDenied("CX-MM-007 ProjectionPlan cloud-projection-plan://
    //   run-cloud-delegation-crosslink/lane-cloud-delegation-crosslink is
    //   authorized by cloud-consent-receipt://run-cloud-delegation/
    //   lane-cloud-delegation, not by ConsentReceipt cloud-consent-receipt://
    //   run-cloud-delegation-crosslink/lane-cloud-delegation-crosslink")
    let pair_error = store
        .replay_cloud_consent_authority(&run_id("delegation-crosslink"))
        .await
        .expect_err("a receipt the plan did not name must not authorize it");
    assert!(
        pair_error.to_string().contains("is authorized by"),
        "the denial must name the authorization-provenance mismatch: {pair_error}"
    );
}

// ---------------------------------------------------------------------------
// 4. HBR-PRIV-005 — the self-issued approver defect itself
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_self_issued_role_label_approver_is_refused_at_write_time() {
    let pool = pg_pool("self-issued approver rejection").await;

    let account = OwnerAccountId::mint();
    let scope = scope_for(account);
    let store = account_store(&pool, &scope);

    let plan = store
        .record_cloud_projection_plan(projection_plan(
            "selfissue",
            approver_for(&scope),
            Some(receipt_id("selfissue")),
        ))
        .await
        .expect("record the projection plan");

    // This is the EXACT string the operator-chat cloud path used to mint:
    // `operator://{owner_session}/cloud-selection`, where `owner_session` is a
    // governance role label. Its identity component is the row's own role label,
    // so it recorded that the requester approved its own export.
    //
    // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)` produced
    //   InvalidInput("approved_by_ref
    //   operator://KERNEL_BUILDER-selfissue/cloud-selection is self-issued: its
    //   identity component is this row's own owner_session governance role
    //   label, which authorizes nothing. Record a typed approver instead.")
    let self_issued = format!("operator://{}/cloud-selection", owner_session("selfissue"));
    let rejected = store
        .record_cloud_consent_receipt(consent_receipt(
            "selfissue",
            &plan.projection_plan_id,
            &plan.projection_plan_hash,
            approver_for(&scope),
            &self_issued,
        ))
        .await
        .expect_err("a self-issued role-label approver must be refused at write time");
    assert!(
        rejected.to_string().contains("self-issued"),
        "the denial must say why the value authorizes nothing: {rejected}"
    );

    // POSITIVE CONTROL: the same receipt with an honest provenance label is
    // accepted, so the rule targets self-issuance and not the `operator://`
    // scheme (which real deployed receipts legitimately use).
    store
        .record_cloud_consent_receipt(consent_receipt(
            "selfissue",
            &plan.projection_plan_id,
            &plan.projection_plan_hash,
            approver_for(&scope),
            "operator://ticket-4471/cloud-export-approval",
        ))
        .await
        .expect("an honest provenance label must still be accepted");
}

#[tokio::test]
async fn an_unattributed_approval_cannot_authorize_an_account_scoped_cloud_launch() {
    let pool = pg_pool("unattributed approval cannot authorize").await;

    // Seed through the pre-account legacy store: the resulting rows carry a NULL
    // owning account AND an explicitly unattributed approver.
    let legacy = legacy_store(&pool);
    let plan = legacy
        .record_cloud_projection_plan(projection_plan(
            "legacy",
            unattributed("LEGACY_CALL_SITE_WITHOUT_ACCOUNT"),
            Some(receipt_id("legacy")),
        ))
        .await
        .expect("a legacy store may still record unattributed authority");
    legacy
        .record_cloud_consent_receipt(consent_receipt(
            "legacy",
            &plan.projection_plan_id,
            &plan.projection_plan_hash,
            unattributed("LEGACY_CALL_SITE_WITHOUT_ACCOUNT"),
            "unattributed://operator-chat/no-authenticated-account",
        ))
        .await
        .expect("a legacy store may still record unattributed authority");

    // A legacy store can still launch: this is the documented pre-WP-KERNEL-006
    // posture, not a new hole. Proving it keeps the negative below honest — the
    // refusal must come from the APPROVER, not from the row being broken.
    let (run, lane) = cloud_run_lane(
        "legacy",
        ModelLaneStatus::Ready,
        &plan_id("legacy"),
        &receipt_id("legacy"),
    );
    legacy
        .record_prepared_launch((run, lane))
        .await
        .expect("the unscoped legacy path is unchanged by this MT");

    // Now simulate the dangerous "fix": a backfill that grandfathers legacy
    // cloud-consent rows into a real account by setting the owning-account
    // column, WITHOUT anyone having actually approved. Layer 1 now lets the rows
    // through; the typed approver must still refuse them.
    let account = OwnerAccountId::mint();
    for (table, key_column, key) in [
        (
            "model_lane_cloud_projection_plans",
            "projection_plan_id",
            plan_id("legacy"),
        ),
        (
            "model_lane_cloud_consent_receipts",
            "consent_receipt_id",
            receipt_id("legacy"),
        ),
    ] {
        let sql = format!("UPDATE {table} SET owner_account_id = $1 WHERE {key_column} = $2");
        sqlx::query(&sql)
            .bind(account.as_uuid())
            .bind(&key)
            .execute(&pool)
            .await
            .unwrap_or_else(|error| panic!("backfill {table}: {error}"));
    }

    let scope = scope_for(account);
    let store = account_store(&pool, &scope);

    // Layer 1 no longer hides the row — prove that, so the denial below is
    // demonstrably the approver check and not "not durable" again.
    let visible = store
        .replay_cloud_consent_authority(&run_id("legacy"))
        .await
        .expect("the backfilled rows are now inside the account's scope");
    assert_eq!(
        visible.consent_receipts.len(),
        1,
        "the backfill must have made the receipt visible, or this test proves nothing"
    );

    // FALSIFIABILITY (inverted, run, observed, restored): `.expect(..)` produced
    //   InvalidInput("CX-MM-007 cloud lane launch denied for run_id
    //   run-cloud-legacy lane_id lane-cloud-legacy: final cloud launch insertion
    //   fence denied: model lane authority denied: CX-MM-007 ConsentReceipt
    //   cloud-consent-receipt://run-cloud-legacy/lane-cloud-legacy carries no
    //   approval usable by this account: RESOURCE_SCOPE_UNATTRIBUTED")
    let (retry_run, retry_lane) = cloud_run_lane(
        "legacy",
        ModelLaneStatus::Ready,
        &plan_id("legacy"),
        &receipt_id("legacy"),
    );
    let denied = store
        .record_prepared_launch((retry_run, retry_lane))
        .await
        .expect_err("an unattributed approval must not authorize an account-scoped launch");
    let message = denied.to_string();
    assert!(
        message.contains("CX-MM-007"),
        "the refusal must be the fail-closed cloud code: {message}"
    );
    assert!(
        message.contains("RESOURCE_SCOPE_UNATTRIBUTED"),
        "the refusal must name the missing approval, not a generic mismatch: {message}"
    );
}

// ---------------------------------------------------------------------------
// 5. HBR-PRIV-005/007/008 — provider dispatch diagnostics carry exact scope
// ---------------------------------------------------------------------------

/// The cloud provider boundary is a derivative of the account-scoped launch.
/// Its durable ProcessLedger START is therefore both dispatch provenance and a
/// diagnostic projection: dropping session, AccessSpace, or workspace here
/// makes two same-account contexts indistinguishable and creates an unscoped
/// side channel even when the lane tables themselves remain protected.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_provider_start_receipt_preserves_exact_server_owned_scope() {
    let pool = pg_pool("cloud provider START exact scope").await;

    let owner = OwnerAccountId::mint();
    let principal = ActorPrincipalId::mint();
    let session = AuthenticatedSessionRef::mint();
    let access_space = AccessSpaceRef::mint();
    let workspace =
        WorkspaceScopeRef::new("workspace-mt006-cloud-dispatch").expect("nonblank workspace scope");
    let scope = ResourceScope::new(owner, principal)
        .with_session(session)
        .with_access_space(access_space)
        .with_workspace(workspace.clone());
    let lane_store = ModelLaneStore::new_scoped(pool.clone(), scope);

    let process_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    process_store
        .apply_migration()
        .await
        .expect("real PostgreSQL ProcessLedger authority is ready");
    let (ledger, writer) = LedgerBatcher::spawn(
        process_store,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 8,
            batch_size: 1,
            flush_interval: Duration::from_millis(1),
        },
    );

    let factory = ProductionModelSessionFactory::new(
        ledger.clone(),
        CloudLaneFactoryConfig {
            anthropic: None,
            openai: Some(Arc::new(ExactScopeCloudBuilder {
                provider: ProviderKind::ByokCloud,
            })),
            official_cli: None,
            official_cli_by_provider: std::collections::HashMap::from([(
                "codex".to_string(),
                Arc::new(ExactScopeCloudBuilder {
                    provider: ProviderKind::OfficialCli,
                }) as Arc<dyn CloudRuntimeBuilder>,
            )]),
        },
        None,
    )
    .with_durable_worktree_vm_store(&lane_store);
    for (ordinal, provider) in [ProviderKind::ByokCloud, ProviderKind::OfficialCli]
        .into_iter()
        .enumerate()
    {
        let mut request = SpawnRequest::new(
            ModelInstanceId::new(ModelId::new_v7(), 6 + ordinal as u32),
            SwarmRuntimeBinding::Candle,
            "KERNEL_BUILDER-MT006",
            format!("mt006-{provider:?}-parent-session"),
        )
        .with_cloud_provider(provider, "mt006-cloud-model");
        if provider == ProviderKind::OfficialCli {
            request = request
                .with_official_cli_provider("codex")
                .with_sandbox_posture(
                    handshake_core::sandbox::TrustClass::Trusted,
                    handshake_core::sandbox::IsolationTier::Tier1Container,
                    BTreeSet::from([
                        handshake_core::sandbox::RequiredCapability::HighStdioThroughput,
                    ]),
                    handshake_core::sandbox::NetPolicy::HostInherited,
                    handshake_core::sandbox::CLI_BRIDGE_REQUESTED_EXECUTION_POLICY_REF,
                );
        }

        let live = factory
            .create(&request)
            .await
            .unwrap_or_else(|error| panic!("scoped {provider:?} dispatch failed: {error}"));
        let process_uuid = live.process_record_id.as_uuid();
        let start_metadata: serde_json::Value = sqlx::query_scalar(
            "SELECT metadata_jsonb FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(process_uuid)
        .fetch_one(&pool)
        .await
        .expect("read the authoritative cloud START receipt from PostgreSQL");

        assert_exact_cloud_process_scope(
            &start_metadata,
            owner,
            principal,
            session,
            access_space,
            &workspace,
            provider,
            "START",
        );

        live.ledger_lifecycle
            .as_ref()
            .expect("pidless cloud dispatch reserves its complete lifecycle")
            .stop_with_durable_ack(
                Some(0),
                "mt006-cloud-provider-scope-proof-complete",
                Duration::from_secs(5),
            )
            .await
            .expect("matching cloud STOP is durable");
        let stopped: bool = sqlx::query_scalar(
            "SELECT stopped_at IS NOT NULL FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(process_uuid)
        .fetch_one(&pool)
        .await
        .expect("fresh PostgreSQL consumer observes terminal cloud receipt");
        assert!(stopped, "{provider:?} lifecycle must reach durable STOP");
        let stop_metadata: serde_json::Value = sqlx::query_scalar(
            "SELECT metadata_jsonb FROM kernel_process_lifecycle WHERE process_uuid = $1",
        )
        .bind(process_uuid)
        .fetch_one(&pool)
        .await
        .expect("fresh PostgreSQL consumer reloads the final cloud receipt");
        assert_exact_cloud_process_scope(
            &stop_metadata,
            owner,
            principal,
            session,
            access_space,
            &workspace,
            provider,
            "STOP",
        );
    }
    ledger.begin_close();
    let outcome = drain_and_join_ledger_writer(&ledger, writer, Duration::from_secs(5)).await;
    assert!(
        matches!(outcome, LedgerDrainJoinOutcome::Flushed),
        "cloud scope proof must leave no detached ledger writer: {outcome:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_provider_requires_exact_scope_before_builder_side_effects() {
    let pool = pg_pool("cloud provider missing scope fails before builder").await;
    let process_store = Arc::new(PostgresProcessLedgerStore::new(pool.clone()));
    process_store
        .apply_migration()
        .await
        .expect("real PostgreSQL ProcessLedger authority is ready");
    let (ledger, writer) = LedgerBatcher::spawn(
        process_store,
        Arc::new(NoopOverflowSink),
        LedgerBatcherConfig {
            capacity: 8,
            batch_size: 1,
            flush_interval: Duration::from_millis(1),
        },
    );
    let builder_calls = Arc::new(AtomicUsize::new(0));
    let factory = ProductionModelSessionFactory::new(
        ledger.clone(),
        CloudLaneFactoryConfig {
            anthropic: None,
            openai: Some(Arc::new(CountingScopeCloudBuilder {
                calls: builder_calls.clone(),
            })),
            official_cli: None,
            official_cli_by_provider: Default::default(),
        },
        None,
    );
    let request = SpawnRequest::new(
        ModelInstanceId::new(ModelId::new_v7(), 7),
        SwarmRuntimeBinding::Candle,
        "KERNEL_BUILDER-MT006",
        "mt006-unscoped-cloud-parent-session",
    )
    .with_cloud_provider(ProviderKind::ByokCloud, "mt006-cloud-model");

    let denied = match factory.create(&request).await {
        Ok(_) => panic!("cloud dispatch without exact server scope must fail closed"),
        Err(error) => error,
    };
    assert!(
        denied.to_string().contains("RESOURCE_SCOPE"),
        "missing-scope denial must be stable and identifier-free: {denied}"
    );
    assert_eq!(
        builder_calls.load(Ordering::SeqCst),
        0,
        "scope authority must be checked before any provider builder side effect"
    );

    let incomplete_store = ModelLaneStore::new_scoped(
        pool,
        ResourceScope::new(OwnerAccountId::mint(), ActorPrincipalId::mint()),
    );
    let incomplete_factory = ProductionModelSessionFactory::new(
        ledger.clone(),
        CloudLaneFactoryConfig {
            anthropic: None,
            openai: Some(Arc::new(CountingScopeCloudBuilder {
                calls: builder_calls.clone(),
            })),
            official_cli: None,
            official_cli_by_provider: Default::default(),
        },
        None,
    )
    .with_durable_worktree_vm_store(&incomplete_store);
    let incomplete_denied = match incomplete_factory.create(&request).await {
        Ok(_) => panic!("cloud dispatch with incomplete server scope must fail closed"),
        Err(error) => error,
    };
    assert!(
        incomplete_denied.to_string().contains("RESOURCE_SCOPE"),
        "incomplete-scope denial must be stable and identifier-free: {incomplete_denied}"
    );
    assert_eq!(
        builder_calls.load(Ordering::SeqCst),
        0,
        "incomplete scope must also be denied before provider builder work"
    );

    ledger.begin_close();
    let outcome = drain_and_join_ledger_writer(&ledger, writer, Duration::from_secs(5)).await;
    assert!(
        matches!(outcome, LedgerDrainJoinOutcome::Flushed),
        "missing-scope proof must leave no detached ledger writer: {outcome:?}"
    );
}

struct CountingScopeCloudBuilder {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl CloudRuntimeBuilder for CountingScopeCloudBuilder {
    fn provider(&self) -> ProviderKind {
        ProviderKind::ByokCloud
    }

    async fn build_loaded(
        &self,
        _model_name: &str,
        _invocation_context: Option<handshake_core::model_runtime::cloud::CliInvocationContext>,
        _working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(CloudLiveRuntime {
            runtime: Arc::new(ExactScopeRuntime),
            model_id: ModelId::new_v7(),
        })
    }
}

fn assert_exact_cloud_process_scope(
    metadata: &serde_json::Value,
    owner: OwnerAccountId,
    principal: ActorPrincipalId,
    session: AuthenticatedSessionRef,
    access_space: AccessSpaceRef,
    workspace: &WorkspaceScopeRef,
    provider: ProviderKind,
    phase: &str,
) {
    for (field, expected) in [
        ("owner_account_id", owner.as_uuid().to_string()),
        ("actor_principal_id", principal.as_uuid().to_string()),
        ("authenticated_session_id", session.as_uuid().to_string()),
        ("access_space_id", access_space.as_uuid().to_string()),
        ("workspace_id", workspace.as_str().to_string()),
    ] {
        assert_eq!(
            metadata[field].as_str(),
            Some(expected.as_str()),
            "{provider:?} {phase} metadata must preserve exact server-owned {field}; metadata={metadata}"
        );
    }
}

struct ExactScopeCloudBuilder {
    provider: ProviderKind,
}

#[async_trait]
impl CloudRuntimeBuilder for ExactScopeCloudBuilder {
    fn provider(&self) -> ProviderKind {
        self.provider
    }

    async fn build_loaded(
        &self,
        _model_name: &str,
        _invocation_context: Option<handshake_core::model_runtime::cloud::CliInvocationContext>,
        _working_dir: Option<&str>,
    ) -> Result<CloudLiveRuntime, String> {
        Ok(CloudLiveRuntime {
            runtime: Arc::new(ExactScopeRuntime),
            model_id: ModelId::new_v7(),
        })
    }
}

struct ExactScopeRuntime;

#[async_trait]
impl ModelRuntime for ExactScopeRuntime {
    async fn load(
        &mut self,
        _spec: handshake_core::model_runtime::LoadSpec,
    ) -> Result<ModelId, ModelRuntimeError> {
        Ok(ModelId::new_v7())
    }

    async fn unload(&mut self, _id: ModelId) -> Result<(), ModelRuntimeError> {
        Ok(())
    }

    fn generate(&self, _request: GenerateRequest) -> TokenStream {
        Box::pin(futures::stream::empty())
    }

    async fn score(&self, _id: ModelId, _sequence: Vec<u32>) -> Result<Score, ModelRuntimeError> {
        Ok(Score {
            token_logprobs: Vec::new(),
            mean_logprob: 0.0,
        })
    }

    async fn embed(&self, _id: ModelId, _text: &str) -> Result<Embedding, ModelRuntimeError> {
        Ok(Embedding { vector: Vec::new() })
    }

    fn capabilities(&self, _id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError> {
        Err(ModelRuntimeError::CapabilityNotSupported {
            capability: "mt006-scope-proof".into(),
            adapter: "mt006-exact-scope-runtime".into(),
        })
    }

    fn kv_cache(&self, _id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::KvCacheError("mt006-scope-proof".into()))
    }

    fn lora_stack(&self, _id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::LoraStackError(
            "mt006-scope-proof".into(),
        ))
    }

    fn steering_hooks(&self, _id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError> {
        Err(ModelRuntimeError::SteeringHookError(
            "mt006-scope-proof".into(),
        ))
    }

    fn cancel(&self, token: CancellationToken) {
        token.cancel();
    }
}
