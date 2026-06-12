//! WP-KERNEL-009 MT-209..219 ParallelSwarmStateRecovery integration proof.
//!
//! These tests run against real PostgreSQL through the existing knowledge
//! PostgreSQL harness. They prove backend behavior, not status text:
//! typed lane identity, worktree/workspace claims, role-mailbox handoff refs,
//! deterministic backend navigation commands, restartable checkpoints and
//! recovery receipts, local/cloud attribution without secret leakage, and a
//! lease queue that serializes parallel index writers per scope.

mod knowledge_pg_support;

use std::{
    collections::HashSet,
    io::Cursor,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use handshake_core::api::kernel as kernel_api;
use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, RecorderError,
};
use handshake_core::governance_check_runner::{
    CheckDescriptor, CheckResult, CheckRunner, CheckRunnerError, QuietCheckRunRequest,
};
use handshake_core::kernel::product_screenshot_capture::{
    record_native_product_screenshot_quiet, NativeScreenshotEvidence,
    ProductScreenshotExecutionError, ProductScreenshotRequestV1,
    QuietProductScreenshotCaptureRequestV1, ScreenshotCaptureExecutionSurface,
    ScreenshotCaptureScope, ScreenshotCaptureTriggerKind, VisualEvidenceProtectionV1,
};
use handshake_core::kernel::KernelActor;
use handshake_core::knowledge_code_index::engine::{CodeIndexContext, CodeIndexEngine};
use handshake_core::knowledge_code_index::CodeIndexError;
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::postgres::PostgresDatabase;
use handshake_core::storage::{Database, NewWorkspace, WriteContext};
use handshake_core::swarm_orchestration::state_recovery::{
    validate_swarm_dashboard_projection, AgentCapability, AgentLaneIdentity, AgentLaneKind,
    AttributionMode, BackendNavigationCommand, ClaimScope, ClaimStatus, IndexLeaseStatus,
    IndexingLeaseRequest, LocalCloudAttribution, ModelProviderKind, NavigationCommandSet,
    ParallelSwarmDashboardProjectionV1, ParallelSwarmStateRecoveryStore, QuietBackgroundPolicy,
    QuietBackgroundWorkKind, QuietBackgroundWorkRequest, RecoveryCheckpointRequest,
    RecoveryResumePointer, RoleMailboxHandoffRequest, StateRecoveryError,
    SwarmDashboardProjectionRequest, SwarmEvidenceInspectionRequest, SwarmReceiptStatus,
    WorkClaimRequest,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::AppState;
use knowledge_pg_support::knowledge_pg;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Barrier;
use uuid::Uuid;

#[derive(Default)]
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

#[async_trait]
impl DiagnosticsStore for NoopRecorder {
    async fn record_diagnostic(
        &self,
        _diag: Diagnostic,
    ) -> Result<(), handshake_core::storage::StorageError> {
        Ok(())
    }

    async fn list_problems(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<ProblemGroup>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(
        &self,
        _id: Uuid,
    ) -> Result<Diagnostic, handshake_core::storage::StorageError> {
        Err(handshake_core::storage::StorageError::NotFound(
            "diagnostic",
        ))
    }

    async fn list_diagnostics(
        &self,
        _filter: DiagFilter,
    ) -> Result<Vec<Diagnostic>, handshake_core::storage::StorageError> {
        Ok(Vec::new())
    }
}

struct NoopLlmClient {
    profile: ModelProfile,
}

#[async_trait]
impl LlmClient for NoopLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: String::new(),
            usage: TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

#[derive(Default)]
struct CountingRecorder {
    events: AtomicUsize,
}

impl CountingRecorder {
    fn recorded_events(&self) -> usize {
        self.events.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl FlightRecorder for CountingRecorder {
    async fn record_event(&self, _event: FlightRecorderEvent) -> Result<(), RecorderError> {
        self.events.fetch_add(1, Ordering::SeqCst);
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

async fn recovery_store() -> Option<(sqlx::PgPool, ParallelSwarmStateRecoveryStore)> {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP parallel_swarm_state_recovery_tests: no PostgreSQL");
        return None;
    };
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect isolated parallel swarm schema");
    let event_db = Arc::new(PostgresDatabase::new(pool.clone()));
    let store = ParallelSwarmStateRecoveryStore::new(pool.clone(), event_db);
    Some((pool, store))
}

async fn app_state_for(schema_url: &str) -> AppState {
    let storage = PostgresDatabase::connect(schema_url, 5)
        .await
        .expect("connect AppState storage to isolated schema")
        .into_arc();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(schema_url)
        .await
        .expect("connect AppState pool to isolated schema");
    let recorder = Arc::new(NoopRecorder);
    AppState {
        storage,
        flight_recorder: recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(NoopLlmClient {
            profile: ModelProfile::new("parallel-swarm-dashboard-api-test".to_string(), 4096),
        }),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
        postgres_pool: pool,
    }
}

async fn start_kernel_server(state: AppState) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("local addr");
    let app = kernel_api::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("kernel api server");
    });
    (format!("http://{addr}"), server)
}

fn local_lane(actor_suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-local-{actor_suffix}"),
        format!("coder-{actor_suffix}"),
        AgentLaneKind::Local,
        LocalCloudAttribution::local("llama_cpp", "qwen-coder-32b"),
    )
    .expect("valid local lane")
}

fn cloud_lane(actor_suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-cloud-{actor_suffix}"),
        format!("cloud-{actor_suffix}"),
        AgentLaneKind::Cloud,
        LocalCloudAttribution::cloud(
            ModelProviderKind::OpenAi,
            "gpt-5.4-codex",
            "vault://providers/openai/default",
            json!({
                "api_key": "sk-test-secret-must-not-persist",
                "organization": "org-visible",
                "endpoint": "https://api.openai.example/v1"
            }),
        ),
    )
    .expect("valid cloud lane with scrubbed metadata")
}

fn raw_cloud_lane(actor_suffix: &str) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-cloud-raw-{actor_suffix}"),
        format!("cloud-raw-{actor_suffix}"),
        AgentLaneKind::Cloud,
        LocalCloudAttribution {
            mode: AttributionMode::Cloud,
            provider: Some(ModelProviderKind::OpenAi),
            runtime: None,
            model_label: "gpt-5.4-codex".to_string(),
            credential_ref: Some("vault://providers/openai/raw".to_string()),
            provider_metadata: json!({
                "api_key": "sk-raw-secret-must-not-persist",
                "nested": {
                    "session_token": "raw-token-must-not-persist"
                },
                "organization": "org-visible"
            }),
        },
    )
    .expect("valid raw cloud lane")
}

fn lane_with_kind(actor_suffix: &str, lane_kind: AgentLaneKind) -> AgentLaneIdentity {
    AgentLaneIdentity::new(
        format!("lane-{actor_suffix}"),
        format!("actor-{actor_suffix}"),
        lane_kind,
        LocalCloudAttribution::local("test-runtime", format!("test-model-{actor_suffix}")),
    )
    .expect("valid typed lane")
}

fn assert_invalid_input_contains<T>(result: Result<T, StateRecoveryError>, expected: &str) {
    match result {
        Err(StateRecoveryError::InvalidInput(message)) => assert!(
            message.contains(expected),
            "expected invalid input to mention {expected}, got {message}"
        ),
        Err(error) => panic!("expected invalid input containing {expected}, got {error}"),
        Ok(_) => panic!("expected invalid input containing {expected}, got success"),
    }
}

async fn ledger_count_for_payload_value(
    pool: &sqlx::PgPool,
    aggregate_type: &str,
    payload_key: &str,
    payload_value: &str,
) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = $1
          AND payload ->> $2 = $3
        "#,
    )
    .bind(aggregate_type)
    .bind(payload_key)
    .bind(payload_value)
    .fetch_one(pool)
    .await
    .expect("count parallel swarm ledger events")
}

async fn ledger_event_type_and_status(pool: &sqlx::PgPool, event_id: &str) -> (String, String) {
    sqlx::query_as(
        r#"
        SELECT event_type, payload ->> 'status'
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .fetch_one(pool)
    .await
    .expect("fetch ledger event type and status")
}

async fn ledger_event_type(pool: &sqlx::PgPool, event_id: &str) -> String {
    sqlx::query_scalar(
        r#"
        SELECT event_type
        FROM kernel_event_ledger
        WHERE event_id = $1
        "#,
    )
    .bind(event_id)
    .fetch_one(pool)
    .await
    .expect("fetch ledger event type")
}

async fn quiet_background_work_count(
    pool: &sqlx::PgPool,
    workspace_id: &str,
    subject_id: &str,
) -> i64 {
    sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_quiet_background_work
        WHERE workspace_id = $1
          AND subject_id = $2
        "#,
    )
    .bind(workspace_id)
    .bind(subject_id)
    .fetch_one(pool)
    .await
    .expect("count quiet background work")
}

async fn swarm_evidence_counts(
    pool: &sqlx::PgPool,
    workspace_id: &str,
) -> (i64, i64, i64, i64, i64, i64) {
    let claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count swarm claims");
    let checkpoints: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_state_recovery_checkpoints WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count swarm checkpoints");
    let leases: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count swarm indexing leases");
    let events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND payload ->> 'workspace_id' = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count swarm evidence events");
    let recovery_receipts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_recovery_receipts r
        INNER JOIN knowledge_agent_state_recovery_checkpoints c
                ON c.checkpoint_id = r.checkpoint_id
        WHERE c.workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count swarm recovery receipts");
    let recovery_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger e
        INNER JOIN knowledge_agent_recovery_receipts r
                ON r.event_ledger_event_id = e.event_id
        INNER JOIN knowledge_agent_state_recovery_checkpoints c
                ON c.checkpoint_id = r.checkpoint_id
        WHERE c.workspace_id = $1
          AND e.aggregate_type = 'parallel_swarm_recovery'
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count swarm recovery events");
    (
        claims,
        checkpoints,
        leases,
        events,
        recovery_receipts,
        recovery_events,
    )
}

async fn swarm_dashboard_authority_counts(
    pool: &sqlx::PgPool,
    workspace_id: &str,
) -> (i64, i64, i64, i64, i64, i64, i64) {
    let claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count dashboard claims");
    let handoffs: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_role_mailbox_handoffs h
        INNER JOIN knowledge_agent_worktree_claims c
                ON c.claim_id = h.claim_id
        WHERE c.workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count dashboard handoffs");
    let checkpoints: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_state_recovery_checkpoints WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count dashboard checkpoints");
    let recovery_receipts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_recovery_receipts r
        INNER JOIN knowledge_agent_state_recovery_checkpoints c
                ON c.checkpoint_id = r.checkpoint_id
        WHERE c.workspace_id = $1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count dashboard recoveries");
    let leases: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count dashboard leases");
    let quiet: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_quiet_background_work WHERE workspace_id = $1",
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count dashboard quiet work");
    let events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND (
              payload ->> 'workspace_id' = $1
              OR event_id IN (
                  SELECT r.event_ledger_event_id
                  FROM knowledge_agent_recovery_receipts r
                  INNER JOIN knowledge_agent_state_recovery_checkpoints c
                          ON c.checkpoint_id = r.checkpoint_id
                  WHERE c.workspace_id = $1
              )
          )
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .expect("count dashboard source events");
    (
        claims,
        handoffs,
        checkpoints,
        recovery_receipts,
        leases,
        quiet,
        events,
    )
}

async fn swarm_dashboard_global_source_counts(
    pool: &sqlx::PgPool,
) -> (i64, i64, i64, i64, i64, i64, i64) {
    let claims: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_agent_worktree_claims")
        .fetch_one(pool)
        .await
        .expect("count all dashboard claims");
    let handoffs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_agent_role_mailbox_handoffs")
            .fetch_one(pool)
            .await
            .expect("count all dashboard handoffs");
    let checkpoints: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_agent_state_recovery_checkpoints")
            .fetch_one(pool)
            .await
            .expect("count all dashboard checkpoints");
    let recovery_receipts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_agent_recovery_receipts")
            .fetch_one(pool)
            .await
            .expect("count all dashboard recoveries");
    let leases: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue")
            .fetch_one(pool)
            .await
            .expect("count all dashboard leases");
    let quiet: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_agent_quiet_background_work")
            .fetch_one(pool)
            .await
            .expect("count all dashboard quiet work");
    let events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM kernel_event_ledger WHERE source_component = 'parallel_swarm_state_recovery'",
    )
    .fetch_one(pool)
    .await
    .expect("count all dashboard EventLedger rows");
    (
        claims,
        handoffs,
        checkpoints,
        recovery_receipts,
        leases,
        quiet,
        events,
    )
}

async fn install_parallel_swarm_event_delay(pool: &sqlx::PgPool, aggregate_type: &str) {
    let suffix = aggregate_type.replace('_', "");
    let function_name = format!("slow_psr_{suffix}_event");
    let trigger_name = format!("slow_psr_{suffix}_trigger");
    let function_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION {function_name}()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            PERFORM pg_sleep(0.25);
            RETURN NEW;
        END
        $$
        "#
    );
    sqlx::query(&function_sql)
        .execute(pool)
        .await
        .expect("install event delay function");
    let trigger_sql = format!(
        r#"
        CREATE TRIGGER {trigger_name}
        BEFORE INSERT ON kernel_event_ledger
        FOR EACH ROW
        WHEN (
            NEW.source_component = 'parallel_swarm_state_recovery'
            AND NEW.aggregate_type = '{aggregate_type}'
        )
        EXECUTE FUNCTION {function_name}()
        "#
    );
    sqlx::query(&trigger_sql)
        .execute(pool)
        .await
        .expect("install event delay trigger");
}

async fn install_parallel_swarm_event_failure(pool: &sqlx::PgPool, aggregate_type: &str) {
    let suffix = aggregate_type.replace('_', "");
    let function_name = format!("fail_psr_{suffix}_event");
    let trigger_name = format!("fail_psr_{suffix}_trigger");
    let function_sql = format!(
        r#"
        CREATE OR REPLACE FUNCTION {function_name}()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'forced parallel swarm event failure';
        END
        $$
        "#
    );
    sqlx::query(&function_sql)
        .execute(pool)
        .await
        .expect("install event failure function");
    let trigger_sql = format!(
        r#"
        CREATE TRIGGER {trigger_name}
        BEFORE INSERT ON kernel_event_ledger
        FOR EACH ROW
        WHEN (
            NEW.source_component = 'parallel_swarm_state_recovery'
            AND NEW.aggregate_type = '{aggregate_type}'
        )
        EXECUTE FUNCTION {function_name}()
        "#
    );
    sqlx::query(&trigger_sql)
        .execute(pool)
        .await
        .expect("install event failure trigger");
}

async fn install_claim_receipt_authority_failure(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION fail_psr_claim_receipt_authority()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.reason = 'forced claim receipt authority failure after receipt'
               AND NEW.event_ledger_event_id IS NOT NULL THEN
                RAISE EXCEPTION 'forced claim receipt authority failure';
            END IF;
            RETURN NEW;
        END
        $$
        "#,
    )
    .execute(pool)
    .await
    .expect("install claim receipt authority failure function");
    sqlx::query(
        r#"
        CREATE TRIGGER fail_psr_claim_receipt_authority_trigger
        BEFORE INSERT OR UPDATE ON knowledge_agent_worktree_claims
        FOR EACH ROW
        EXECUTE FUNCTION fail_psr_claim_receipt_authority()
        "#,
    )
    .execute(pool)
    .await
    .expect("install claim receipt authority failure trigger");
}

async fn install_recovery_receipt_authority_failure(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION fail_psr_recovery_receipt_authority()
        RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NEW.new_session_id = 'session-recovery-authority-fail' THEN
                RAISE EXCEPTION 'forced recovery receipt authority failure';
            END IF;
            RETURN NEW;
        END
        $$
        "#,
    )
    .execute(pool)
    .await
    .expect("install recovery receipt authority failure function");
    sqlx::query(
        r#"
        CREATE TRIGGER fail_psr_recovery_receipt_authority_trigger
        BEFORE INSERT ON knowledge_agent_recovery_receipts
        FOR EACH ROW
        EXECUTE FUNCTION fail_psr_recovery_receipt_authority()
        "#,
    )
    .execute(pool)
    .await
    .expect("install recovery receipt authority failure trigger");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_lanes_and_work_claims_are_typed_attributable_and_exclusive() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let local = local_lane("claim-a");
    let cloud = cloud_lane("claim-b");
    let other_local = local_lane("claim-c");
    assert!(local
        .capabilities()
        .contains(&AgentCapability::ClaimWorktree));
    assert!(local
        .capabilities()
        .contains(&AgentCapability::WriteLocalIndex));
    assert!(cloud
        .capabilities()
        .contains(&AgentCapability::NavigateBackend));
    assert!(
        !cloud
            .capabilities()
            .contains(&AgentCapability::WriteLocalIndex),
        "cloud lanes must not silently become local index writers"
    );
    assert!(
        !serde_json::to_string(&cloud)
            .expect("serialize cloud lane")
            .contains("sk-test-secret"),
        "provider secrets must be scrubbed from lane attribution"
    );

    let scope = ClaimScope::Worktree {
        worktree_id: "wtc-kernel-009".to_string(),
    };
    let first = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-alpha".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: scope.clone(),
            lane: local.clone(),
            session_id: "session-local-a".to_string(),
            ttl_seconds: 600,
            reason: "claim the product worktree for MT-210".to_string(),
        })
        .await
        .expect("first claim");
    assert_eq!(first.status, ClaimStatus::Active);
    assert!(first.claim_id.starts_with("PSR-CLAIM-"));

    let second = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-alpha".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: scope.clone(),
            lane: other_local.clone(),
            session_id: "session-local-c".to_string(),
            ttl_seconds: 600,
            reason: "parallel local worker should wait".to_string(),
        })
        .await
        .expect("second claim returns held, not corruption");
    assert_eq!(second.status, ClaimStatus::Held);
    assert_eq!(
        second
            .active_holder
            .as_ref()
            .expect("held claim names active holder")
            .actor_id,
        local.actor_id
    );

    let active = store
        .list_active_claims("workspace-alpha")
        .await
        .expect("active claims");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].claim_id, first.claim_id);

    assert!(
        store
            .release_claim(&first.claim_id, &local, "MT-210 complete")
            .await
            .expect("release claim"),
        "claim holder can release the claim"
    );
    let release_event_id: Option<String> = sqlx::query_scalar(
        "SELECT to_jsonb(c) ->> 'release_event_ledger_event_id' FROM knowledge_agent_worktree_claims c WHERE claim_id = $1",
    )
    .bind(&first.claim_id)
    .fetch_one(&pool)
    .await
    .expect("fetch release receipt event id");
    let release_event_id = release_event_id.expect("released claim carries release receipt");
    assert!(
        release_event_id.starts_with("KE-"),
        "released claim must carry an EventLedger release receipt"
    );

    let after_release = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-alpha".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope,
            lane: other_local,
            session_id: "session-local-c".to_string(),
            ttl_seconds: 600,
            reason: "claim after release".to_string(),
        })
        .await
        .expect("claim after release");
    assert_eq!(after_release.status, ClaimStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_authority_failure_after_receipt_rolls_back_eventledger_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    install_claim_receipt_authority_failure(&pool).await;
    let workspace_id = format!("workspace-claim-receipt-fail-{}", Uuid::now_v7());
    let result = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-claim-fail-{}", Uuid::now_v7()),
            },
            lane: local_lane("claim-receipt-fail"),
            session_id: "session-claim-receipt-fail".to_string(),
            ttl_seconds: 600,
            reason: "forced claim receipt authority failure after receipt".to_string(),
        })
        .await;
    assert!(
        result.is_err(),
        "forced authority failure should reject claim receipt persistence"
    );

    let claim_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count failed claim rows");
    assert_eq!(claim_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim",
            "workspace_id",
            &workspace_id
        )
        .await,
        0,
        "failed claim authority write must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn release_claim_rolls_back_authority_state_if_receipt_insert_fails() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-release-receipt-fail-{}", Uuid::now_v7());
    let lane = local_lane("release-receipt-fail-holder");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-release-fail-{}", Uuid::now_v7()),
            },
            lane: lane.clone(),
            session_id: "session-release-receipt-fail-holder".to_string(),
            ttl_seconds: 600,
            reason: "claim that should remain active if release receipt fails".to_string(),
        })
        .await
        .expect("claim before forced release receipt failure");

    install_parallel_swarm_event_failure(&pool, "parallel_swarm_claim").await;
    let result = store
        .release_claim(
            &claim.claim_id,
            &lane,
            "forced release receipt insertion failure",
        )
        .await;
    assert!(
        result.is_err(),
        "forced EventLedger failure should reject release"
    );

    let (status, has_released_at, release_event_id): (String, bool, Option<String>) =
        sqlx::query_as(
            r#"
            SELECT status,
                   released_at_utc IS NOT NULL,
                   to_jsonb(c) ->> 'release_event_ledger_event_id'
            FROM knowledge_agent_worktree_claims c
            WHERE claim_id = $1
            "#,
        )
        .bind(&claim.claim_id)
        .fetch_one(&pool)
        .await
        .expect("fetch claim after failed release");
    assert_eq!(
        status, "active",
        "claim must not be left released when receipt insertion fails"
    );
    assert!(
        !has_released_at,
        "failed release must not stamp released_at_utc"
    );
    assert!(
        release_event_id.is_none(),
        "failed release must not attach a missing receipt id"
    );
    assert_eq!(
        ledger_count_for_payload_value(&pool, "parallel_swarm_claim", "claim_id", &claim.claim_id)
            .await,
        1,
        "failed release must not add a second false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cloud_lane_is_denied_worktree_claim_and_local_index_write_lease() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-capability-deny-{}", Uuid::now_v7());
    let cloud = cloud_lane("capability-deny");
    let claim_result = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-209".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: "wtc-kernel-009".to_string(),
            },
            lane: cloud.clone(),
            session_id: "session-cloud-capability-deny".to_string(),
            ttl_seconds: 600,
            reason: "cloud lanes must not claim local worktrees".to_string(),
        })
        .await;
    assert_invalid_input_contains(claim_result, "ClaimWorktree");

    let persisted_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied worktree claims");
    assert_eq!(persisted_claims, 0);

    let claim_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_swarm_claim'
          AND payload ->> 'workspace_id' = $1
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied claim events");
    assert_eq!(claim_events, 0);

    let lease_result = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-216".to_string(),
            scope: ClaimScope::IndexRun {
                workspace_id: workspace_id.clone(),
                source_root_id: "root-capability-deny".to_string(),
            },
            lane: cloud,
            session_id: "session-cloud-index-deny".to_string(),
            index_run_id: format!("index-run-{}", Uuid::now_v7()),
            priority: 10,
            ttl_seconds: 600,
            quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        })
        .await;
    assert_invalid_input_contains(lease_result, "WriteLocalIndex");

    let persisted_leases: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied indexing leases");
    assert_eq!(persisted_leases, 0);

    let lease_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_indexing_lease'
          AND payload ->> 'workspace_id' = $1
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied lease events");
    assert_eq!(lease_events, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn editor_document_and_graph_claims_serialize_parallel_mutations() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-editor-safety-{}", Uuid::now_v7());
    let doc_scope = ClaimScope::RichDocument {
        workspace_id: workspace_id.clone(),
        document_id: "note-alpha".to_string(),
    };
    let graph_scope = ClaimScope::GraphMutation {
        workspace_id: workspace_id.clone(),
        graph_id: "graph-main".to_string(),
    };
    let first_editor = lane_with_kind("editor-doc-a", AgentLaneKind::Editor);
    let second_editor = lane_with_kind("editor-doc-b", AgentLaneKind::Editor);

    let first_doc_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: doc_scope.clone(),
            lane: first_editor.clone(),
            session_id: "session-editor-doc-a".to_string(),
            ttl_seconds: 600,
            reason: "claim rich document before editing".to_string(),
        })
        .await
        .expect("first rich-document claim");
    assert_eq!(first_doc_claim.status, ClaimStatus::Active);

    let second_doc_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: doc_scope.clone(),
            lane: second_editor.clone(),
            session_id: "session-editor-doc-b".to_string(),
            ttl_seconds: 600,
            reason: "parallel editor should wait for same document".to_string(),
        })
        .await
        .expect("second rich-document claim returns held");
    assert_eq!(second_doc_claim.status, ClaimStatus::Held);
    assert_eq!(
        second_doc_claim
            .active_holder
            .as_ref()
            .expect("held document claim names active holder")
            .actor_id,
        first_editor.actor_id
    );

    let other_doc_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "note-beta".to_string(),
            },
            lane: second_editor.clone(),
            session_id: "session-editor-doc-b".to_string(),
            ttl_seconds: 600,
            reason: "different document can proceed in parallel".to_string(),
        })
        .await
        .expect("different document claim");
    assert_eq!(other_doc_claim.status, ClaimStatus::Active);

    let first_graph_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: graph_scope.clone(),
            lane: first_editor.clone(),
            session_id: "session-editor-graph-a".to_string(),
            ttl_seconds: 600,
            reason: "claim graph before mutation".to_string(),
        })
        .await
        .expect("first graph mutation claim");
    assert_eq!(first_graph_claim.status, ClaimStatus::Active);

    let second_graph_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: graph_scope.clone(),
            lane: second_editor.clone(),
            session_id: "session-editor-graph-b".to_string(),
            ttl_seconds: 600,
            reason: "parallel editor should wait for same graph".to_string(),
        })
        .await
        .expect("second graph claim returns held");
    assert_eq!(second_graph_claim.status, ClaimStatus::Held);
    assert_eq!(
        second_graph_claim
            .active_holder
            .as_ref()
            .expect("held graph claim names active holder")
            .actor_id,
        first_editor.actor_id
    );

    let persisted_scopes: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT scope_kind, scope_id
        FROM knowledge_agent_worktree_claims
        WHERE workspace_id = $1
        ORDER BY scope_kind ASC, scope_id ASC
        "#,
    )
    .bind(&workspace_id)
    .fetch_all(&pool)
    .await
    .expect("fetch persisted editor safety claims");
    assert!(
        persisted_scopes.contains(&(
            "rich_document".to_string(),
            format!("{workspace_id}/note-alpha")
        )),
        "rich-document claims must persist a stable per-document scope id"
    );
    assert!(
        persisted_scopes.contains(&(
            "graph_mutation".to_string(),
            format!("{workspace_id}/graph-main")
        )),
        "graph mutation claims must persist a stable per-graph scope id"
    );

    assert!(
        store
            .release_claim(
                &first_doc_claim.claim_id,
                &first_editor,
                "rich document edit complete"
            )
            .await
            .expect("release rich document claim"),
        "document claim holder can release the claim"
    );
    let after_release = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: doc_scope,
            lane: second_editor,
            session_id: "session-editor-doc-b".to_string(),
            ttl_seconds: 600,
            reason: "same document can proceed after release".to_string(),
        })
        .await
        .expect("claim released document");
    assert_eq!(after_release.status, ClaimStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_editor_lanes_cannot_claim_editor_mutation_scopes() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-editor-deny-{}", Uuid::now_v7());
    for (suffix, lane_kind, scope, expected_capability) in [
        (
            "validator-doc",
            AgentLaneKind::Validator,
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "note-denied".to_string(),
            },
            "EditRichDocument",
        ),
        (
            "validator-graph",
            AgentLaneKind::Validator,
            ClaimScope::GraphMutation {
                workspace_id: workspace_id.clone(),
                graph_id: "graph-denied".to_string(),
            },
            "MutateGraph",
        ),
        (
            "cloud-doc",
            AgentLaneKind::Cloud,
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "cloud-note-denied".to_string(),
            },
            "EditRichDocument",
        ),
        (
            "indexer-graph",
            AgentLaneKind::Indexer,
            ClaimScope::GraphMutation {
                workspace_id: workspace_id.clone(),
                graph_id: "indexer-graph-denied".to_string(),
            },
            "MutateGraph",
        ),
    ] {
        let claim_result = store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-217".to_string()),
                scope,
                lane: lane_with_kind(suffix, lane_kind),
                session_id: format!("session-{suffix}"),
                ttl_seconds: 600,
                reason: "non-editor lanes must not mutate editor surfaces".to_string(),
            })
            .await;
        assert_invalid_input_contains(claim_result, expected_capability);
    }

    let persisted_claims: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied editor safety claims");
    assert_eq!(persisted_claims, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim",
            "workspace_id",
            &workspace_id
        )
        .await,
        0,
        "denied mutation-scope claims must not emit false EventLedger receipts"
    );

    let allowed_workspace_id = format!("workspace-editor-allowed-{}", Uuid::now_v7());
    let editor_claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: allowed_workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-217".to_string()),
            scope: ClaimScope::RichDocument {
                workspace_id: allowed_workspace_id,
                document_id: "note-allowed".to_string(),
            },
            lane: lane_with_kind("editor-allowed", AgentLaneKind::Editor),
            session_id: "session-editor-allowed".to_string(),
            ttl_seconds: 600,
            reason: "editor lane may claim rich document mutation scope".to_string(),
        })
        .await
        .expect("editor may claim document mutation scope");
    assert_eq!(editor_claim.status, ClaimStatus::Active);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_editor_mutation_scopes_do_not_persist_claims_or_receipts() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-editor-malformed-{}", Uuid::now_v7());
    let editor = lane_with_kind("editor-malformed-scope", AgentLaneKind::Editor);
    for (suffix, scope, expected) in [
        (
            "empty-doc-workspace",
            ClaimScope::RichDocument {
                workspace_id: String::new(),
                document_id: "note-empty-workspace".to_string(),
            },
            "rich_document.workspace_id",
        ),
        (
            "empty-document-id",
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: String::new(),
            },
            "rich_document.document_id",
        ),
        (
            "slash-document-id",
            ClaimScope::RichDocument {
                workspace_id: workspace_id.clone(),
                document_id: "nested/note".to_string(),
            },
            "rich_document.document_id",
        ),
        (
            "mismatched-doc-workspace",
            ClaimScope::RichDocument {
                workspace_id: format!("other-{workspace_id}"),
                document_id: "note-mismatch".to_string(),
            },
            "workspace_id must match",
        ),
        (
            "empty-graph-id",
            ClaimScope::GraphMutation {
                workspace_id: workspace_id.clone(),
                graph_id: String::new(),
            },
            "graph_mutation.graph_id",
        ),
        (
            "mismatched-graph-workspace",
            ClaimScope::GraphMutation {
                workspace_id: format!("other-graph-{workspace_id}"),
                graph_id: "graph-mismatch".to_string(),
            },
            "workspace_id must match",
        ),
    ] {
        let claim_result = store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-217".to_string()),
                scope,
                lane: editor.clone(),
                session_id: format!("session-invalid-editor-scope-{suffix}"),
                ttl_seconds: 600,
                reason: "invalid editor mutation scope should not persist".to_string(),
            })
            .await;
        assert_invalid_input_contains(claim_result, expected);
    }

    let persisted_claims: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_worktree_claims
        WHERE session_id LIKE 'session-invalid-editor-scope-%'
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("count malformed editor scope claims");
    assert_eq!(
        persisted_claims, 0,
        "malformed editor mutation claims must be rejected before authority rows"
    );
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim",
            "workspace_id",
            &workspace_id
        )
        .await,
        0,
        "malformed editor mutation claims must not emit false EventLedger receipts"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validator_lanes_inspect_swarm_evidence_without_mutating_state() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-validator-isolation-{}", Uuid::now_v7());
    let local = local_lane("validator-isolation-source");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-218".to_string()),
            scope: ClaimScope::Workspace {
                workspace_id: workspace_id.clone(),
            },
            lane: local.clone(),
            session_id: "session-validator-isolation-source-claim".to_string(),
            ttl_seconds: 600,
            reason: "seed validator inspection evidence".to_string(),
        })
        .await
        .expect("seed workspace claim");
    let lease = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-218".to_string(),
            scope: ClaimScope::IndexRun {
                workspace_id: workspace_id.clone(),
                source_root_id: "validator-isolation-root".to_string(),
            },
            lane: local.clone(),
            session_id: "session-validator-isolation-source-lease".to_string(),
            index_run_id: format!("index-run-validator-isolation-{}", Uuid::now_v7()),
            priority: 1,
            ttl_seconds: 600,
            quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        })
        .await
        .expect("seed indexing lease");
    let checkpoint = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: local,
            session_id: "session-validator-isolation-source-checkpoint".to_string(),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-218".to_string(),
            claim_id: Some(claim.claim_id.clone()),
            mailbox_handoff_id: None,
            navigation_command_id: Some("validation_state".to_string()),
            resume_pointer: RecoveryResumePointer::Claim {
                claim_id: claim.claim_id.clone(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec!["parallel_swarm_state_recovery_tests::validator_lanes".to_string()],
            hbr_rows: vec!["HBR-SWARM-001".to_string()],
            next_step_context: "validator can inspect this evidence without mutating".to_string(),
            payload: json!({"validator_inspection_seed": true}),
            compaction_reason: "validator_isolation_seed".to_string(),
            git_head: "validator218".to_string(),
        })
        .await
        .expect("seed checkpoint");

    let before_counts = swarm_evidence_counts(&pool, &workspace_id).await;
    for lane_kind in [
        AgentLaneKind::Validator,
        AgentLaneKind::IntegrationValidator,
    ] {
        let lane = lane_with_kind(
            &format!("validator-isolation-{}", lane_kind.as_str()),
            lane_kind,
        );
        let capabilities = lane.capabilities();
        assert!(capabilities.contains(&AgentCapability::InspectEvidence));
        assert!(capabilities.contains(&AgentCapability::NavigateBackend));
        for forbidden in [
            AgentCapability::ClaimWorktree,
            AgentCapability::ClaimWorkspace,
            AgentCapability::EditRichDocument,
            AgentCapability::MutateGraph,
            AgentCapability::WriteLocalIndex,
            AgentCapability::WriteMailbox,
            AgentCapability::RecordCheckpoint,
        ] {
            assert!(
                !capabilities.contains(&forbidden),
                "{lane_kind:?} must not carry mutation capability {forbidden:?}"
            );
        }

        let snapshot = store
            .inspect_swarm_evidence(SwarmEvidenceInspectionRequest {
                lane: lane.clone(),
                workspace_id: workspace_id.clone(),
                limit: 50,
            })
            .await
            .expect("validator lane inspects swarm evidence");
        assert_eq!(snapshot.workspace_id, workspace_id);
        assert!(
            snapshot
                .claims
                .iter()
                .any(|row| row.claim_id == claim.claim_id),
            "validator inspection must expose existing claim evidence"
        );
        assert!(
            snapshot
                .indexing_leases
                .iter()
                .any(|row| row.lease_id == lease.lease_id),
            "validator inspection must expose existing indexing lease evidence"
        );
        assert!(
            snapshot
                .checkpoints
                .iter()
                .any(|row| row.checkpoint_id == checkpoint.checkpoint_id),
            "validator inspection must expose existing checkpoint evidence"
        );
        assert_eq!(
            swarm_evidence_counts(&pool, &workspace_id).await,
            before_counts,
            "validator evidence inspection must be SELECT-only"
        );

        let claim_result = store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-218".to_string()),
                scope: ClaimScope::Workspace {
                    workspace_id: format!("validator-mutating-{workspace_id}"),
                },
                lane: lane.clone(),
                session_id: format!("session-{}-claim-deny", lane.lane_id),
                ttl_seconds: 600,
                reason: "validator must not mutate claims".to_string(),
            })
            .await;
        assert_invalid_input_contains(claim_result, "ClaimWorkspace");

        let mailbox_thread_id = format!("thread-{}-deny", lane.lane_id);
        let handoff_result = store
            .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                from_lane: lane.clone(),
                to_role: "WP_VALIDATOR".to_string(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-218".to_string(),
                claim_id: None,
                mailbox_thread_id: mailbox_thread_id.clone(),
                mailbox_message_id: "message-validator-deny".to_string(),
                status: SwarmReceiptStatus::Blocked,
                summary: "validator lanes are read-only in product state".to_string(),
                body_sha256: "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                    .to_string(),
            })
            .await;
        assert_invalid_input_contains(handoff_result, "WriteMailbox");
        assert_eq!(
            ledger_count_for_payload_value(
                &pool,
                "parallel_swarm_handoff",
                "mailbox_thread_id",
                &mailbox_thread_id,
            )
            .await,
            0,
            "denied validator mailbox writes must not emit false receipts"
        );

        let checkpoint_result = store
            .record_checkpoint(RecoveryCheckpointRequest {
                lane,
                session_id: "session-validator-checkpoint-deny".to_string(),
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-218".to_string(),
                claim_id: None,
                mailbox_handoff_id: None,
                navigation_command_id: Some("validation_state".to_string()),
                resume_pointer: RecoveryResumePointer::MicroTask {
                    mt_id: "MT-218".to_string(),
                },
                touched_files: vec![],
                tests: vec![],
                hbr_rows: vec!["HBR-SWARM-001".to_string()],
                next_step_context: "validator checkpoint write should be denied".to_string(),
                payload: json!({"validator_write_denied": true}),
                compaction_reason: "validator_denied".to_string(),
                git_head: "validator218".to_string(),
            })
            .await;
        assert_invalid_input_contains(checkpoint_result, "RecordCheckpoint");

        let recovery_result = store
            .recover_from_checkpoint(
                &checkpoint.checkpoint_id,
                lane_with_kind(
                    &format!("validator-isolation-recover-{}", lane_kind.as_str()),
                    lane_kind,
                ),
                &format!("session-{}-recover-deny", lane_kind.as_str()),
            )
            .await;
        assert_invalid_input_contains(recovery_result, "RecordCheckpoint");
    }

    assert_eq!(
        swarm_evidence_counts(&pool, &workspace_id).await,
        before_counts,
        "denied validator mutations must not change inspected state"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn swarm_dashboard_projection_derives_from_postgres_eventledger_and_is_projection_only() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-dashboard-projection-{}", Uuid::now_v7());
    let local = local_lane("dashboard-projection");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-220".to_string()),
            scope: ClaimScope::Workspace {
                workspace_id: workspace_id.clone(),
            },
            lane: local.clone(),
            session_id: "session-dashboard-claim".to_string(),
            ttl_seconds: 600,
            reason: "seed durable dashboard claim".to_string(),
        })
        .await
        .expect("seed dashboard claim");
    assert_eq!(claim.status, ClaimStatus::Active);
    let claim_id = claim.claim_id.clone();

    let handoff = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: local.clone(),
            to_role: "WP_VALIDATOR".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-220".to_string(),
            claim_id: Some(claim_id.clone()),
            mailbox_thread_id: "thread-dashboard-claim-backed".to_string(),
            mailbox_message_id: "message-dashboard-claim-backed".to_string(),
            status: SwarmReceiptStatus::Progress,
            summary: "claim-backed dashboard handoff".to_string(),
            body_sha256: "a".repeat(64),
        })
        .await
        .expect("seed claim-backed handoff");
    let unscoped_handoff = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: local.clone(),
            to_role: "WP_VALIDATOR".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-220".to_string(),
            claim_id: None,
            mailbox_thread_id: "thread-dashboard-unscoped".to_string(),
            mailbox_message_id: "message-dashboard-unscoped".to_string(),
            status: SwarmReceiptStatus::Blocked,
            summary: "unscoped handoff must be warned and excluded".to_string(),
            body_sha256: "b".repeat(64),
        })
        .await
        .expect("seed unscoped handoff");
    assert!(
        !unscoped_handoff.event_ledger_event_id.is_empty(),
        "unscoped handoff is still durable, just not workspace-projectable"
    );

    let checkpoint = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: local.clone(),
            session_id: "session-dashboard-checkpoint".to_string(),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-220".to_string(),
            claim_id: Some(claim_id.clone()),
            mailbox_handoff_id: Some(handoff.handoff_id.clone()),
            navigation_command_id: Some("validation_state".to_string()),
            resume_pointer: RecoveryResumePointer::Claim {
                claim_id: claim_id.clone(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec![
                "parallel_swarm_state_recovery_tests::swarm_dashboard_projection".to_string(),
            ],
            hbr_rows: vec!["HBR-SWARM-001".to_string(), "HBR-SWARM-004".to_string()],
            next_step_context: "dashboard projection can recover from this checkpoint".to_string(),
            payload: json!({"dashboard_projection_seed": true}),
            compaction_reason: "dashboard_projection_seed".to_string(),
            git_head: "dashboard220".to_string(),
        })
        .await
        .expect("seed dashboard checkpoint");
    let recovered = store
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            local_lane("dashboard-recovery"),
            "session-dashboard-recovered",
        )
        .await
        .expect("seed dashboard recovery receipt");
    let lease = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-220".to_string(),
            scope: ClaimScope::IndexRun {
                workspace_id: workspace_id.clone(),
                source_root_id: "dashboard-source-root".to_string(),
            },
            lane: local.clone(),
            session_id: "session-dashboard-indexing".to_string(),
            index_run_id: format!("index-run-dashboard-{}", Uuid::now_v7()),
            priority: 3,
            ttl_seconds: 600,
            quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        })
        .await
        .expect("seed dashboard indexing lease");
    assert_eq!(lease.status, IndexLeaseStatus::Acquired);
    let quiet = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane: local.clone(),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-220".to_string(),
            work_kind: QuietBackgroundWorkKind::TestRun,
            subject_id: format!("dashboard-test-run-{}", Uuid::now_v7()),
            session_id: "session-dashboard-quiet-test".to_string(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::TestRun),
            evidence_ref: "cargo://parallel_swarm_state_recovery_tests/dashboard_projection"
                .to_string(),
        })
        .await
        .expect("seed dashboard quiet work");

    let before_counts = swarm_dashboard_authority_counts(&pool, &workspace_id).await;
    let before_global_counts = swarm_dashboard_global_source_counts(&pool).await;
    let projection = store
        .project_swarm_dashboard(SwarmDashboardProjectionRequest {
            lane: lane_with_kind("dashboard-validator", AgentLaneKind::Validator),
            workspace_id: workspace_id.clone(),
            wp_id: Some("WP-KERNEL-009".to_string()),
            mt_id: Some("MT-220".to_string()),
            limit: 50,
        })
        .await
        .expect("project swarm dashboard");
    assert_eq!(
        swarm_dashboard_authority_counts(&pool, &workspace_id).await,
        before_counts,
        "dashboard projection must be SELECT-only over PostgreSQL/EventLedger"
    );
    assert_eq!(
        swarm_dashboard_global_source_counts(&pool).await,
        before_global_counts,
        "dashboard projection must not write unrelated source rows or EventLedger events"
    );
    validate_swarm_dashboard_projection(&projection).expect("valid dashboard projection");
    assert_eq!(projection.workspace_id, workspace_id);
    assert!(projection.projection_contract.projection_only);
    assert!(!projection.projection_contract.authority_mutation_allowed);
    assert!(!projection.projection_contract.ui_state_authoritative);
    assert_eq!(projection.totals.claims, 1);
    assert_eq!(projection.totals.active_claims, 1);
    assert_eq!(projection.totals.mailbox_handoffs, 1);
    assert_eq!(projection.totals.recovery_checkpoints, 1);
    assert_eq!(projection.totals.recovery_receipts, 1);
    assert_eq!(projection.totals.indexing_leases, 1);
    assert_eq!(projection.totals.quiet_background_work, 1);
    assert_eq!(projection.totals.events, 6);
    assert_eq!(projection.totals.warnings, 1);
    assert!(projection
        .warnings
        .iter()
        .any(|warning| warning.code == "handoffs_without_workspace_source_ref_excluded"));
    assert!(projection
        .claims
        .iter()
        .any(|row| row.claim_id == claim_id && row.status == "active"));
    assert!(projection
        .mailbox_handoffs
        .iter()
        .any(|row| row.handoff_id == handoff.handoff_id));
    assert!(!projection
        .mailbox_handoffs
        .iter()
        .any(|row| row.handoff_id == unscoped_handoff.handoff_id));
    assert!(projection
        .recovery_checkpoints
        .iter()
        .any(|row| row.checkpoint_id == checkpoint.checkpoint_id));
    assert!(projection
        .recovery_receipts
        .iter()
        .any(|row| row.receipt_id == recovered.receipt.receipt_id));
    assert!(projection
        .indexing_leases
        .iter()
        .any(|row| row.lease_id == lease.lease_id && row.quiet_policy_ok));
    assert!(projection
        .quiet_background_work
        .iter()
        .any(|row| row.receipt_id == quiet.receipt_id && row.quiet_policy_ok));
    let aggregate_counts: HashSet<&str> = projection
        .source_watermark
        .aggregate_counts
        .iter()
        .map(|row| row.aggregate_type.as_str())
        .collect();
    for aggregate in [
        "parallel_swarm_claim",
        "parallel_swarm_handoff",
        "parallel_swarm_checkpoint",
        "parallel_swarm_recovery",
        "parallel_indexing_lease",
        "parallel_swarm_quiet_background_work",
    ] {
        assert!(
            aggregate_counts.contains(aggregate),
            "projection watermark must include {aggregate}: {:?}",
            projection.source_watermark.aggregate_counts
        );
    }

    let mut forged_ui_authority: ParallelSwarmDashboardProjectionV1 = projection.clone();
    forged_ui_authority
        .projection_contract
        .ui_state_authoritative = true;
    forged_ui_authority
        .projection_contract
        .authority_mutation_allowed = true;
    let errors = validate_swarm_dashboard_projection(&forged_ui_authority)
        .expect_err("UI-authoritative forged projection must fail validation");
    assert!(errors
        .iter()
        .any(|error| error.contains("ui_state_authoritative=false")));
    assert!(errors
        .iter()
        .any(|error| error.contains("authority_mutation_allowed=false")));

    let mut forged_ui_only_row = projection.clone();
    forged_ui_only_row.claims[0].source_refs.clear();
    let errors = validate_swarm_dashboard_projection(&forged_ui_only_row)
        .expect_err("UI-only row without source refs must fail validation");
    assert!(
        errors
            .iter()
            .any(|error| error.contains("has no source_refs")),
        "expected source ref validation error, got {errors:?}"
    );

    let mut forged_wrong_table = projection.clone();
    forged_wrong_table.claims[0].source_refs[0].table_name =
        "knowledge_agent_quiet_background_work".to_string();
    forged_wrong_table.claims[0].source_refs[0].row_source_ref = format!(
        "postgres://knowledge_agent_quiet_background_work/{}",
        forged_wrong_table.claims[0].claim_id
    );
    let errors = validate_swarm_dashboard_projection(&forged_wrong_table)
        .expect_err("wrong source table must fail validation");
    assert!(
        errors
            .iter()
            .any(|error| error.contains("source table must be knowledge_agent_worktree_claims")),
        "expected source table validation error, got {errors:?}"
    );

    let mut forged_wrong_event = projection.clone();
    forged_wrong_event.claims[0].source_refs[0].event_aggregate_id =
        Some("wrong-aggregate-id".to_string());
    let errors = validate_swarm_dashboard_projection(&forged_wrong_event)
        .expect_err("wrong EventLedger aggregate id must fail validation");
    assert!(
        errors
            .iter()
            .any(|error| error.contains("mismatched event aggregate_id")),
        "expected event aggregate validation error, got {errors:?}"
    );

    let mut forged_missing_watermark = projection.clone();
    forged_missing_watermark.source_watermark.events.clear();
    forged_missing_watermark.source_watermark.event_count = 0;
    forged_missing_watermark
        .source_watermark
        .aggregate_counts
        .clear();
    let errors = validate_swarm_dashboard_projection(&forged_missing_watermark)
        .expect_err("source refs missing from watermark must fail validation");
    assert!(
        errors
            .iter()
            .any(|error| error.contains("EventLedger ref missing from watermark")),
        "expected watermark validation error, got {errors:?}"
    );

    let bad_limit = store
        .project_swarm_dashboard(SwarmDashboardProjectionRequest {
            lane: lane_with_kind("dashboard-validator-bad-limit", AgentLaneKind::Validator),
            workspace_id: workspace_id.clone(),
            wp_id: Some("WP-KERNEL-009".to_string()),
            mt_id: Some("MT-220".to_string()),
            limit: 0,
        })
        .await;
    assert_invalid_input_contains(bad_limit, "inspection limit");
    assert_eq!(
        swarm_dashboard_authority_counts(&pool, &workspace_id).await,
        before_counts,
        "rejected dashboard projection requests must not mutate authority state"
    );
    assert_eq!(
        swarm_dashboard_global_source_counts(&pool).await,
        before_global_counts,
        "rejected dashboard projection requests must not mutate any source table or EventLedger"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn swarm_dashboard_projection_api_exposes_postgres_eventledger_read_model() {
    let Some(pg) = knowledge_pg().await else {
        eprintln!("SKIP swarm_dashboard_projection_api: no PostgreSQL");
        return;
    };
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&pg.schema_url)
        .await
        .expect("connect isolated parallel swarm schema");
    let event_db = Arc::new(PostgresDatabase::new(pool.clone()));
    let store = ParallelSwarmStateRecoveryStore::new(pool.clone(), event_db);
    let workspace_id = format!("workspace-dashboard-api-{}", Uuid::now_v7());
    let local = local_lane("dashboard-api");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-220".to_string()),
            scope: ClaimScope::Workspace {
                workspace_id: workspace_id.clone(),
            },
            lane: local.clone(),
            session_id: "session-dashboard-api-claim".to_string(),
            ttl_seconds: 600,
            reason: "seed dashboard API claim".to_string(),
        })
        .await
        .expect("seed dashboard API claim");
    let quiet = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane: local,
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-220".to_string(),
            work_kind: QuietBackgroundWorkKind::BackendNavigation,
            subject_id: format!("dashboard-api-nav-{}", Uuid::now_v7()),
            session_id: "session-dashboard-api-quiet".to_string(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::BackendNavigation),
            evidence_ref: "api://kernel/parallel_swarm/dashboard_projection".to_string(),
        })
        .await
        .expect("seed dashboard API quiet receipt");

    let before_counts = swarm_dashboard_authority_counts(&pool, &workspace_id).await;
    let before_global_counts = swarm_dashboard_global_source_counts(&pool).await;
    let state = app_state_for(&pg.schema_url).await;
    let (base, server) = start_kernel_server(state).await;
    let http = reqwest::Client::new();

    let response = http
        .get(format!("{base}/kernel/parallel_swarm/dashboard_projection"))
        .query(&[
            ("workspace_id", workspace_id.as_str()),
            ("wp_id", "WP-KERNEL-009"),
            ("mt_id", "MT-220"),
            ("limit", "25"),
        ])
        .send()
        .await
        .expect("dashboard projection API send");
    assert_eq!(response.status(), 200);
    let projection: ParallelSwarmDashboardProjectionV1 =
        response.json().await.expect("dashboard projection json");
    validate_swarm_dashboard_projection(&projection).expect("API projection validates");
    assert_eq!(projection.workspace_id, workspace_id);
    assert!(projection.projection_contract.projection_only);
    assert_eq!(projection.totals.claims, 1);
    assert_eq!(projection.totals.quiet_background_work, 1);
    assert!(projection
        .claims
        .iter()
        .any(|row| row.claim_id == claim.claim_id));
    assert!(projection
        .quiet_background_work
        .iter()
        .any(|row| row.receipt_id == quiet.receipt_id));
    assert_eq!(
        swarm_dashboard_authority_counts(&pool, &workspace_id).await,
        before_counts,
        "dashboard API must be read-only over durable state"
    );
    assert_eq!(
        swarm_dashboard_global_source_counts(&pool).await,
        before_global_counts,
        "dashboard API must not write unrelated source rows or EventLedger events"
    );

    let bad_limit = http
        .get(format!("{base}/kernel/parallel_swarm/dashboard_projection"))
        .query(&[
            ("workspace_id", workspace_id.as_str()),
            ("wp_id", "WP-KERNEL-009"),
            ("mt_id", "MT-220"),
            ("limit", "0"),
        ])
        .send()
        .await
        .expect("dashboard bad-limit API send");
    assert_eq!(bad_limit.status(), 400);
    let bad_body: serde_json::Value = bad_limit.json().await.expect("bad limit json");
    assert_eq!(bad_body["code"], "parallel_swarm_dashboard_invalid_request");
    assert_eq!(
        swarm_dashboard_authority_counts(&pool, &workspace_id).await,
        before_counts,
        "rejected dashboard API requests must not mutate durable state"
    );
    assert_eq!(
        swarm_dashboard_global_source_counts(&pool).await,
        before_global_counts,
        "rejected dashboard API requests must not mutate any source table or EventLedger"
    );
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn swarm_dashboard_projection_totals_remain_authoritative_when_rows_are_limited() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-dashboard-limit-{}", Uuid::now_v7());
    let local = local_lane("dashboard-limit");
    for index in 0..3 {
        let claim = store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-220".to_string()),
                scope: ClaimScope::Worktree {
                    worktree_id: format!("dashboard-limit-worktree-{index}"),
                },
                lane: local.clone(),
                session_id: format!("session-dashboard-limit-{index}"),
                ttl_seconds: 600,
                reason: format!("seed dashboard limited claim {index}"),
            })
            .await
            .expect("seed dashboard limited claim");
        assert_eq!(claim.status, ClaimStatus::Active);
    }

    let before_counts = swarm_dashboard_authority_counts(&pool, &workspace_id).await;
    let before_global_counts = swarm_dashboard_global_source_counts(&pool).await;
    let projection = store
        .project_swarm_dashboard(SwarmDashboardProjectionRequest {
            lane: lane_with_kind("dashboard-limit-validator", AgentLaneKind::Validator),
            workspace_id: workspace_id.clone(),
            wp_id: Some("WP-KERNEL-009".to_string()),
            mt_id: Some("MT-220".to_string()),
            limit: 1,
        })
        .await
        .expect("project limited dashboard");
    validate_swarm_dashboard_projection(&projection).expect("limited projection validates");
    assert_eq!(
        projection.claims.len(),
        1,
        "row array obeys requested limit"
    );
    assert_eq!(
        projection.source_watermark.event_count, 1,
        "watermark covers returned rows"
    );
    assert_eq!(
        projection.totals.claims, 3,
        "totals remain authoritative over all matching durable rows"
    );
    assert_eq!(projection.totals.active_claims, 3);
    assert_eq!(
        projection.totals.events, 3,
        "event total remains authoritative over all matching durable EventLedger rows"
    );
    assert!(projection.warnings.iter().any(|warning| {
        warning.code == "dashboard_section_truncated"
            && warning.detail.contains("claims returned 1 of 3")
    }));
    assert_eq!(
        swarm_dashboard_authority_counts(&pool, &workspace_id).await,
        before_counts,
        "limited dashboard projection must be SELECT-only over filtered authority state"
    );
    assert_eq!(
        swarm_dashboard_global_source_counts(&pool).await,
        before_global_counts,
        "limited dashboard projection must not write any source table or EventLedger"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quiet_background_work_receipts_reject_foreground_or_focus_stealing_work() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-quiet-background-{}", Uuid::now_v7());
    let local = local_lane("quiet-background");
    let mut loud_visual = QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::VisualCapture);
    loud_visual.no_foreground_window = false;
    let loud_subject = format!("visual-loud-{}", Uuid::now_v7());
    let loud_result = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane: local.clone(),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            work_kind: QuietBackgroundWorkKind::VisualCapture,
            subject_id: loud_subject.clone(),
            session_id: "session-quiet-loud-deny".to_string(),
            policy: loud_visual,
            evidence_ref: "visual-capture://foreground-attempt".to_string(),
        })
        .await;
    assert_invalid_input_contains(loud_result, "no_foreground_window");
    assert_eq!(
        quiet_background_work_count(&pool, &workspace_id, &loud_subject).await,
        0,
        "rejected foreground visual capture must not persist a quiet-work row"
    );
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_quiet_background_work",
            "subject_id",
            &loud_subject,
        )
        .await,
        0,
        "rejected foreground visual capture must not emit a false receipt"
    );

    let quiet_subject = format!("visual-headless-{}", Uuid::now_v7());
    let quiet_record = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane: local.clone(),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            work_kind: QuietBackgroundWorkKind::VisualCapture,
            subject_id: quiet_subject.clone(),
            session_id: "session-quiet-visual".to_string(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::VisualCapture),
            evidence_ref: "visual-capture://headless-loopback".to_string(),
        })
        .await
        .expect("quiet visual capture receipt");
    assert_eq!(
        quiet_record.work_kind,
        QuietBackgroundWorkKind::VisualCapture
    );
    assert!(quiet_record.policy.all_quiet());
    assert!(quiet_record.event_ledger_event_id.starts_with("KE-"));
    assert_eq!(
        ledger_event_type(&pool, &quiet_record.event_ledger_event_id).await,
        "KNOWLEDGE_QUIET_BACKGROUND_WORK_RECORDED"
    );

    let payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&quiet_record.event_ledger_event_id)
            .fetch_one(&pool)
            .await
            .expect("quiet work event payload");
    assert_eq!(payload["quiet_policy"]["work_kind"], "visual_capture");
    assert_eq!(payload["quiet_policy"]["no_foreground_window"], true);
    assert_eq!(payload["quiet_policy"]["no_focus_steal"], true);
    assert_eq!(payload["quiet_policy"]["no_os_shell_window"], true);

    let validator = lane_with_kind("quiet-inspector", AgentLaneKind::Validator);
    let snapshot = store
        .inspect_swarm_evidence(SwarmEvidenceInspectionRequest {
            lane: validator.clone(),
            workspace_id: workspace_id.clone(),
            limit: 50,
        })
        .await
        .expect("validator inspects quiet background evidence");
    assert!(
        snapshot
            .quiet_background_work
            .iter()
            .any(|row| row.receipt_id == quiet_record.receipt_id),
        "validator evidence inspection must expose quiet background work receipts"
    );

    let validator_subject = format!("validator-quiet-write-deny-{}", Uuid::now_v7());
    let validator_result = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane: validator,
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            work_kind: QuietBackgroundWorkKind::BackendNavigation,
            subject_id: validator_subject.clone(),
            session_id: "session-validator-quiet-write-deny".to_string(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::BackendNavigation),
            evidence_ref: "backend-nav://validation-state".to_string(),
        })
        .await;
    assert_invalid_input_contains(validator_result, "RunQuietBackgroundWork");
    assert_eq!(
        quiet_background_work_count(&pool, &workspace_id, &validator_subject).await,
        0,
        "validator lanes may inspect quiet evidence but must not write it"
    );

    let invalid_policy_cases = [
        ("no_focus_steal", {
            let mut policy = QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::TestRun);
            policy.no_focus_steal = false;
            policy
        }),
        ("bounded", {
            let mut policy = QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::TestRun);
            policy.bounded = false;
            policy
        }),
        ("observable", {
            let mut policy = QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::TestRun);
            policy.observable = false;
            policy
        }),
        (
            "work_kind",
            QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        ),
    ];
    for (expected, policy) in invalid_policy_cases {
        let subject_id = format!("quiet-invalid-{expected}-{}", Uuid::now_v7());
        let result = store
            .record_quiet_background_work(QuietBackgroundWorkRequest {
                lane: local.clone(),
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-219".to_string(),
                work_kind: QuietBackgroundWorkKind::TestRun,
                subject_id: subject_id.clone(),
                session_id: format!("session-quiet-invalid-{expected}"),
                policy,
                evidence_ref: format!("test-run://quiet-invalid-{expected}"),
            })
            .await;
        assert_invalid_input_contains(result, expected);
        assert_eq!(
            quiet_background_work_count(&pool, &workspace_id, &subject_id).await,
            0,
            "invalid quiet policy {expected} must not persist a quiet-work row"
        );
    }

    let missing_evidence_subject = format!("quiet-missing-evidence-{}", Uuid::now_v7());
    let missing_evidence_result = store
        .record_quiet_background_work(QuietBackgroundWorkRequest {
            lane: local,
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            work_kind: QuietBackgroundWorkKind::TestRun,
            subject_id: missing_evidence_subject.clone(),
            session_id: "session-quiet-missing-evidence".to_string(),
            policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::TestRun),
            evidence_ref: "".to_string(),
        })
        .await;
    assert_invalid_input_contains(missing_evidence_result, "evidence_ref");
    assert_eq!(
        quiet_background_work_count(&pool, &workspace_id, &missing_evidence_subject).await,
        0,
        "quiet-work receipts require a concrete evidence_ref"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn indexing_leases_and_backend_navigation_are_quiet_by_contract() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    let workspace_id = format!("workspace-quiet-index-{}", Uuid::now_v7());

    let nav = NavigationCommandSet::default();
    for command in nav.commands() {
        let policy = command.quiet_policy();
        assert_eq!(policy.work_kind, QuietBackgroundWorkKind::BackendNavigation);
        assert!(
            policy.all_quiet(),
            "backend navigation command {} must be quiet by contract",
            command.command_id
        );
    }
    let resolved = nav
        .resolve(
            BackendNavigationCommand::ValidationState,
            json!({"workspace_id": workspace_id.clone()}),
        )
        .expect("quiet backend navigation resolves");
    assert_eq!(
        resolved.quiet_policy.work_kind,
        QuietBackgroundWorkKind::BackendNavigation
    );
    assert!(resolved.quiet_policy.all_quiet());
    assert_eq!(
        quiet_background_work_count(&pool, &workspace_id, &resolved.deterministic_cache_key).await,
        0,
        "pure navigation resolution must not pretend to have durable quiet evidence"
    );
    let quiet_nav = store
        .resolve_backend_navigation_quiet(
            local_lane("quiet-nav"),
            "session-quiet-nav".to_string(),
            "WP-KERNEL-009".to_string(),
            "MT-219".to_string(),
            BackendNavigationCommand::ValidationState,
            json!({"workspace_id": workspace_id.clone()}),
        )
        .await
        .expect("quiet backend navigation resolves with durable receipt");
    assert_eq!(
        quiet_nav.resolved.deterministic_cache_key,
        resolved.deterministic_cache_key
    );
    assert_eq!(
        quiet_nav.quiet_receipt.work_kind,
        QuietBackgroundWorkKind::BackendNavigation
    );
    assert_eq!(
        quiet_nav.quiet_receipt.subject_id,
        quiet_nav.resolved.deterministic_cache_key
    );
    assert!(quiet_nav.quiet_receipt.policy.all_quiet());
    assert_eq!(
        ledger_event_type(&pool, &quiet_nav.quiet_receipt.event_ledger_event_id).await,
        "KNOWLEDGE_QUIET_BACKGROUND_WORK_RECORDED"
    );
    assert_eq!(
        quiet_background_work_count(
            &pool,
            &workspace_id,
            &quiet_nav.resolved.deterministic_cache_key,
        )
        .await,
        1,
        "persisted backend navigation must leave validator-inspectable quiet evidence"
    );

    let scope = ClaimScope::IndexRun {
        workspace_id: workspace_id.clone(),
        source_root_id: "quiet-index-root".to_string(),
    };
    let quiet_lease = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            scope: scope.clone(),
            lane: local_lane("quiet-index-a"),
            session_id: "session-quiet-index-a".to_string(),
            index_run_id: format!("index-run-quiet-{}", Uuid::now_v7()),
            priority: 10,
            ttl_seconds: 600,
            quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        })
        .await
        .expect("quiet indexing lease");
    assert_eq!(
        quiet_lease.quiet_policy.work_kind,
        QuietBackgroundWorkKind::Indexing
    );
    assert!(quiet_lease.quiet_policy.all_quiet());

    let stored_policy: serde_json::Value = sqlx::query_scalar(
        "SELECT quiet_policy_jsonb FROM knowledge_parallel_indexing_lease_queue WHERE lease_id = $1",
    )
    .bind(&quiet_lease.lease_id)
    .fetch_one(&pool)
    .await
    .expect("stored quiet indexing policy");
    assert_eq!(stored_policy["work_kind"], "indexing");
    assert_eq!(stored_policy["no_foreground_window"], true);
    assert_eq!(stored_policy["no_focus_steal"], true);
    assert_eq!(stored_policy["no_os_shell_window"], true);

    let event_payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&quiet_lease.event_ledger_event_id)
            .fetch_one(&pool)
            .await
            .expect("quiet lease event payload");
    assert_eq!(event_payload["quiet_policy"]["work_kind"], "indexing");
    assert_eq!(event_payload["quiet_policy"]["no_foreground_window"], true);

    let mut loud_index = QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing);
    loud_index.no_os_shell_window = false;
    let loud_run_id = format!("index-run-loud-{}", Uuid::now_v7());
    let loud_result = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            scope,
            lane: local_lane("quiet-index-loud"),
            session_id: "session-quiet-index-loud".to_string(),
            index_run_id: loud_run_id.clone(),
            priority: 20,
            ttl_seconds: 600,
            quiet_policy: loud_index,
        })
        .await;
    assert_invalid_input_contains(loud_result, "no_os_shell_window");
    let loud_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE index_run_id = $1",
    )
    .bind(&loud_run_id)
    .fetch_one(&pool)
    .await
    .expect("count rejected loud indexing lease");
    assert_eq!(loud_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_indexing_lease",
            "index_run_id",
            &loud_run_id,
        )
        .await,
        0,
        "rejected loud indexing work must not emit a false lease receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_product_entrypoints_emit_quiet_background_work_receipts() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    let event_db = Arc::new(PostgresDatabase::new(pool.clone()));
    let workspace = event_db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("real-quiet-{}", Uuid::now_v7()),
            },
        )
        .await
        .expect("create workspace for real quiet entrypoints");
    let workspace_id = workspace.id;

    let index_engine = CodeIndexEngine::new(event_db.clone());
    let code_context = CodeIndexContext {
        actor: KernelActor::System("quiet-code-index".to_string()),
        kernel_task_run_id: "KTR-quiet-code-index".to_string(),
        session_run_id: "SR-quiet-code-index".to_string(),
        correlation_id: Some("CORR-quiet-code-index".to_string()),
    };
    let quiet_run = index_engine
        .start_quiet_run(
            &code_context,
            &store,
            local_lane("real-quiet-index"),
            "WP-KERNEL-009",
            "MT-219",
            &workspace_id,
            None,
            10,
            600,
        )
        .await
        .expect("real code-index run starts through quiet path");
    assert_eq!(quiet_run.indexing_lease.status, IndexLeaseStatus::Acquired);
    assert_eq!(
        quiet_run.indexing_lease.index_run_id,
        quiet_run.index_run_id
    );
    assert_eq!(
        quiet_run.quiet_receipt.work_kind,
        QuietBackgroundWorkKind::Indexing
    );
    assert_eq!(quiet_run.quiet_receipt.subject_id, quiet_run.index_run_id);
    let index_run_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_index_runs WHERE index_run_id = $1")
            .bind(&quiet_run.index_run_id)
            .fetch_one(&pool)
            .await
            .expect("count quiet code-index run");
    assert_eq!(
        index_run_rows, 1,
        "quiet indexing proof must start a real knowledge_index_runs row"
    );

    let artifact_root = tempfile::tempdir().expect("temp visual artifact root");
    let screenshot_request = ProductScreenshotRequestV1 {
        request_id: format!("request.native.quiet.{}", Uuid::now_v7()),
        scope: ScreenshotCaptureScope::Module,
        target_ref: "module://quiet-background-work".to_string(),
        requested_by_role: "CODER".to_string(),
        trigger_kind: ScreenshotCaptureTriggerKind::DccApi,
        window_title: "Handshake Desktop Shell".to_string(),
        width: 1,
        height: 1,
        capture_adapter_ref: "capture-adapter://tauri-webview2-cdp".to_string(),
        flight_recorder_ref: "FR-EVT-VISUAL-CAPTURE-quiet-background-work".to_string(),
        execution_surface: ScreenshotCaptureExecutionSurface::GovernedAdapterApi,
        workdir_ref: "repo-root://".to_string(),
    };
    let quiet_capture = record_native_product_screenshot_quiet(
        &screenshot_request,
        NativeScreenshotEvidence {
            png_bytes: tiny_png_bytes(),
            width: 1,
            height: 1,
            scope: ScreenshotCaptureScope::Module,
            captured_at_utc: "2026-06-12T00:00:00Z".to_string(),
            from_surface: true,
            focus_audit_clean: true,
        },
        None,
        &VisualEvidenceProtectionV1::default(),
        artifact_root.path(),
        &store,
        QuietProductScreenshotCaptureRequestV1 {
            lane: local_lane("real-quiet-visual"),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            session_id: "session-real-quiet-visual".to_string(),
        },
    )
    .await
    .expect("real native screenshot capture records quiet receipt");
    assert_eq!(
        quiet_capture.quiet_receipt.work_kind,
        QuietBackgroundWorkKind::VisualCapture
    );
    assert_eq!(
        quiet_capture.quiet_receipt.subject_id,
        quiet_capture.capture.artifact.artifact_id
    );
    assert_eq!(
        quiet_capture.quiet_receipt.evidence_ref,
        quiet_capture.capture.artifact.screenshot_ref
    );

    let check_artifact_root = tempfile::tempdir().expect("temp check artifact root");
    let check_runner = CheckRunner::new(
        Arc::new(NoopRecorder),
        check_artifact_root.path().to_path_buf(),
    );
    let quiet_check = check_runner
        .run_quiet_check(
            &store,
            QuietCheckRunRequest {
                descriptor: CheckDescriptor::new(Uuid::now_v7(), "quiet native check", "native"),
                session_id: Uuid::now_v7(),
                granted_capabilities: vec!["governance.check.run".to_string()],
                lane: local_lane("real-quiet-test"),
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-219".to_string(),
            },
        )
        .await
        .expect("real check runner records quiet receipt");
    assert_eq!(quiet_check.result.status(), "pass");
    assert_eq!(
        quiet_check.quiet_receipt.work_kind,
        QuietBackgroundWorkKind::TestRun
    );
    let check_evidence_id = match &quiet_check.result {
        CheckResult::Pass(details) => details
            .evidence_artifact_id
            .as_deref()
            .expect("native quiet check pass must write evidence artifact"),
        other => panic!("expected quiet native check pass, got {other:?}"),
    };
    assert_eq!(
        quiet_check.quiet_receipt.evidence_ref,
        format!("artifact://governance-check/{check_evidence_id}"),
        "quiet check receipt must cite the real check artifact written by the runner"
    );

    let quiet_rows: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM knowledge_agent_quiet_background_work
        WHERE workspace_id = $1
          AND work_kind IN ('indexing', 'visual_capture', 'test_run')
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count real quiet entrypoint rows");
    assert_eq!(
        quiet_rows, 3,
        "real product entrypoints must leave durable quiet rows for indexing, visual capture, and tests"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quiet_entrypoint_denials_happen_before_product_side_effects() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    let event_db = Arc::new(PostgresDatabase::new(pool.clone()));
    let workspace = event_db
        .create_workspace(
            &WriteContext::human(None),
            NewWorkspace {
                name: format!("quiet-denied-{}", Uuid::now_v7()),
            },
        )
        .await
        .expect("create workspace for quiet denial proof");
    let workspace_id = workspace.id;

    let index_engine = CodeIndexEngine::new(event_db.clone());
    let code_context = CodeIndexContext {
        actor: KernelActor::System("quiet-code-index-denied".to_string()),
        kernel_task_run_id: "KTR-quiet-code-index-denied".to_string(),
        session_run_id: "SR-quiet-code-index-denied".to_string(),
        correlation_id: Some("CORR-quiet-code-index-denied".to_string()),
    };
    let denied_index = index_engine
        .start_quiet_run(
            &code_context,
            &store,
            lane_with_kind("denied-quiet-index", AgentLaneKind::Validator),
            "WP-KERNEL-009",
            "MT-219",
            &workspace_id,
            None,
            10,
            600,
        )
        .await;
    match denied_index {
        Err(CodeIndexError::Validation(message)) => assert!(
            message.contains("WriteLocalIndex"),
            "denied quiet index must fail before the KIR write on lane capability, got {message}"
        ),
        other => panic!("expected denied quiet index validation error, got {other:?}"),
    }
    let index_run_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_index_runs WHERE workspace_id = $1")
            .bind(&workspace_id)
            .fetch_one(&pool)
            .await
            .expect("count denied quiet KIR rows");
    assert_eq!(
        index_run_rows, 0,
        "denied quiet indexing must not leave an orphan knowledge_index_runs row"
    );

    let first_quiet_index = index_engine
        .start_quiet_run(
            &code_context,
            &store,
            local_lane("contention-quiet-index-a"),
            "WP-KERNEL-009",
            "MT-219",
            &workspace_id,
            None,
            10,
            600,
        )
        .await
        .expect("first quiet index acquires same-scope lease");
    let contended_index = index_engine
        .start_quiet_run(
            &code_context,
            &store,
            local_lane("contention-quiet-index-b"),
            "WP-KERNEL-009",
            "MT-219",
            &workspace_id,
            None,
            10,
            600,
        )
        .await;
    match contended_index {
        Err(CodeIndexError::Validation(message)) => assert!(
            message.contains("did not acquire index lease"),
            "contended quiet index must fail without queueing a future orphan, got {message}"
        ),
        other => panic!("expected contended quiet index validation error, got {other:?}"),
    }
    let index_lease_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count contended quiet index leases");
    assert_eq!(
        index_lease_rows, 1,
        "contended quiet indexing must not persist a queued lease without KIR/quiet receipt"
    );
    let index_run_rows_after_contention: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM knowledge_index_runs WHERE workspace_id = $1")
            .bind(&workspace_id)
            .fetch_one(&pool)
            .await
            .expect("count KIR rows after contention");
    assert_eq!(
        index_run_rows_after_contention, 1,
        "contended quiet indexing must leave only the acquired KIR row"
    );
    assert_eq!(
        first_quiet_index.quiet_receipt.subject_id,
        first_quiet_index.index_run_id
    );

    let artifact_root = tempfile::tempdir().expect("temp denied visual artifact root");
    let denied_capture = record_native_product_screenshot_quiet(
        &ProductScreenshotRequestV1 {
            request_id: format!("request.native.quiet.denied.{}", Uuid::now_v7()),
            scope: ScreenshotCaptureScope::Module,
            target_ref: "module://quiet-background-work-denied".to_string(),
            requested_by_role: "CODER".to_string(),
            trigger_kind: ScreenshotCaptureTriggerKind::DccApi,
            window_title: "Handshake Desktop Shell".to_string(),
            width: 1,
            height: 1,
            capture_adapter_ref: "capture-adapter://tauri-webview2-cdp".to_string(),
            flight_recorder_ref: "FR-EVT-VISUAL-CAPTURE-quiet-background-work-denied".to_string(),
            execution_surface: ScreenshotCaptureExecutionSurface::GovernedAdapterApi,
            workdir_ref: "repo-root://".to_string(),
        },
        NativeScreenshotEvidence {
            png_bytes: tiny_png_bytes(),
            width: 1,
            height: 1,
            scope: ScreenshotCaptureScope::Module,
            captured_at_utc: "2026-06-12T00:00:00Z".to_string(),
            from_surface: true,
            focus_audit_clean: true,
        },
        None,
        &VisualEvidenceProtectionV1::default(),
        artifact_root.path(),
        &store,
        QuietProductScreenshotCaptureRequestV1 {
            lane: lane_with_kind("denied-quiet-visual", AgentLaneKind::Validator),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            session_id: "session-denied-quiet-visual".to_string(),
        },
    )
    .await;
    match denied_capture {
        Err(ProductScreenshotExecutionError::QuietReceipt(message)) => assert!(
            message.contains("RunQuietBackgroundWork"),
            "denied quiet screenshot must fail on quiet capability before artifact write, got {message}"
        ),
        other => panic!("expected denied quiet screenshot receipt error, got {other:?}"),
    }
    assert!(
        std::fs::read_dir(artifact_root.path())
            .expect("read denied visual artifact root")
            .next()
            .is_none(),
        "denied quiet screenshot must not write ArtifactStore payloads"
    );

    let invalid_focus_root = tempfile::tempdir().expect("temp invalid focus artifact root");
    let invalid_focus_capture = record_native_product_screenshot_quiet(
        &ProductScreenshotRequestV1 {
            request_id: format!("request.native.quiet.invalid-focus.{}", Uuid::now_v7()),
            scope: ScreenshotCaptureScope::Module,
            target_ref: "module://quiet-background-work-invalid-focus".to_string(),
            requested_by_role: "CODER".to_string(),
            trigger_kind: ScreenshotCaptureTriggerKind::DccApi,
            window_title: "Handshake Desktop Shell".to_string(),
            width: 1,
            height: 1,
            capture_adapter_ref: "capture-adapter://tauri-webview2-cdp".to_string(),
            flight_recorder_ref: "FR-EVT-VISUAL-CAPTURE-quiet-background-work-invalid-focus"
                .to_string(),
            execution_surface: ScreenshotCaptureExecutionSurface::GovernedAdapterApi,
            workdir_ref: "repo-root://".to_string(),
        },
        NativeScreenshotEvidence {
            png_bytes: tiny_png_bytes(),
            width: 1,
            height: 1,
            scope: ScreenshotCaptureScope::Module,
            captured_at_utc: "2026-06-12T00:00:00Z".to_string(),
            from_surface: true,
            focus_audit_clean: false,
        },
        None,
        &VisualEvidenceProtectionV1::default(),
        invalid_focus_root.path(),
        &store,
        QuietProductScreenshotCaptureRequestV1 {
            lane: local_lane("invalid-focus-quiet-visual"),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-219".to_string(),
            session_id: "session-invalid-focus-quiet-visual".to_string(),
        },
    )
    .await;
    match invalid_focus_capture {
        Err(ProductScreenshotExecutionError::Validation(message)) => assert!(
            message.contains("focus audit was not clean"),
            "invalid focus capture must fail before quiet receipt, got {message}"
        ),
        other => panic!("expected invalid focus validation error, got {other:?}"),
    }
    assert!(
        std::fs::read_dir(invalid_focus_root.path())
            .expect("read invalid focus artifact root")
            .next()
            .is_none(),
        "invalid focus quiet screenshot must not write ArtifactStore payloads"
    );

    let check_recorder = Arc::new(CountingRecorder::default());
    let check_artifact_root = tempfile::tempdir().expect("temp denied check artifact root");
    let check_runner = CheckRunner::new(
        check_recorder.clone(),
        check_artifact_root.path().to_path_buf(),
    );
    let denied_check = check_runner
        .run_quiet_check(
            &store,
            QuietCheckRunRequest {
                descriptor: CheckDescriptor::new(Uuid::now_v7(), "quiet denied check", "native"),
                session_id: Uuid::now_v7(),
                granted_capabilities: vec!["governance.check.run".to_string()],
                lane: lane_with_kind("denied-quiet-test", AgentLaneKind::Validator),
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-219".to_string(),
            },
        )
        .await;
    match denied_check {
        Err(CheckRunnerError::Generic(message)) => assert!(
            message.contains("RunQuietBackgroundWork"),
            "denied quiet check must fail on quiet capability before run_check, got {message}"
        ),
        other => panic!("expected denied quiet check receipt error, got {other:?}"),
    }
    assert_eq!(
        check_recorder.recorded_events(),
        0,
        "denied quiet check must not emit Flight Recorder events"
    );
    assert!(
        std::fs::read_dir(check_artifact_root.path())
            .expect("read denied check artifact root")
            .next()
            .is_none(),
        "denied quiet check must not write check artifacts"
    );

    let missing_capability_recorder = Arc::new(CountingRecorder::default());
    let missing_capability_root =
        tempfile::tempdir().expect("temp missing-capability check artifact root");
    let missing_capability_runner = CheckRunner::new(
        missing_capability_recorder.clone(),
        missing_capability_root.path().to_path_buf(),
    );
    let missing_capability_check = missing_capability_runner
        .run_quiet_check(
            &store,
            QuietCheckRunRequest {
                descriptor: CheckDescriptor::new(
                    Uuid::now_v7(),
                    "quiet missing capability check",
                    "native",
                ),
                session_id: Uuid::now_v7(),
                granted_capabilities: Vec::new(),
                lane: local_lane("missing-capability-quiet-test"),
                workspace_id: workspace_id.clone(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-219".to_string(),
            },
        )
        .await;
    match missing_capability_check {
        Err(CheckRunnerError::CapabilityGate(missing)) => assert!(
            missing.contains(&"governance.check.run".to_string()),
            "quiet check preflight must report missing governance check capability"
        ),
        other => panic!("expected missing-capability quiet check error, got {other:?}"),
    }
    assert_eq!(
        missing_capability_recorder.recorded_events(),
        0,
        "missing-capability quiet check must not emit Flight Recorder events"
    );
    assert!(
        std::fs::read_dir(missing_capability_root.path())
            .expect("read missing-capability check artifact root")
            .next()
            .is_none(),
        "missing-capability quiet check must not write planned check artifacts"
    );

    let quiet_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_quiet_background_work WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count denied quiet rows");
    assert_eq!(
        quiet_rows, 1,
        "denied/invalid quiet entrypoints must not persist extra quiet receipts beyond the acquired contention seed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_handoff_requires_write_mailbox_capability() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    for (lane_kind, suffix) in [
        (AgentLaneKind::Indexer, "indexer-deny-mailbox"),
        (AgentLaneKind::Editor, "editor-deny-mailbox"),
    ] {
        let thread_id = format!("thread-{suffix}-{}", Uuid::now_v7());
        let result = store
            .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                from_lane: lane_with_kind(suffix, lane_kind),
                to_role: "WP_VALIDATOR".to_string(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-211".to_string(),
                claim_id: None,
                mailbox_thread_id: thread_id.clone(),
                mailbox_message_id: format!("message-{suffix}"),
                status: SwarmReceiptStatus::Blocked,
                summary: format!("{suffix} must not write role mailbox handoffs"),
                body_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .to_string(),
            })
            .await;
        assert_invalid_input_contains(result, "WriteMailbox");

        let handoff_rows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM knowledge_agent_role_mailbox_handoffs WHERE mailbox_thread_id = $1",
        )
        .bind(&thread_id)
        .fetch_one(&pool)
        .await
        .expect("count denied mailbox handoff rows");
        assert_eq!(handoff_rows, 0);
        assert_eq!(
            ledger_count_for_payload_value(
                &pool,
                "parallel_swarm_handoff",
                "mailbox_thread_id",
                &thread_id,
            )
            .await,
            0,
            "denied mailbox writers must not leave EventLedger handoff receipts"
        );
    }
}

fn tiny_png_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();
    let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        1,
        1,
        image::Rgba([0, 0, 0, 255]),
    ));
    image
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("tiny png writes");
    bytes
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_mailbox_handoff_claim_ref_does_not_emit_false_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let thread_id = format!("thread-invalid-claim-{}", Uuid::now_v7());
    let result = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: local_lane("invalid-handoff-ref"),
            to_role: "WP_VALIDATOR".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-211".to_string(),
            claim_id: Some(format!("PSR-CLAIM-missing-{}", Uuid::now_v7())),
            mailbox_thread_id: thread_id.clone(),
            mailbox_message_id: "message-invalid-claim".to_string(),
            status: SwarmReceiptStatus::Blocked,
            summary: "invalid claim FK must fail without a false receipt".to_string(),
            body_sha256: "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                .to_string(),
        })
        .await;
    assert!(result.is_err(), "invalid claim FK should reject handoff");

    let handoff_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_role_mailbox_handoffs WHERE mailbox_thread_id = $1",
    )
    .bind(&thread_id)
    .fetch_one(&pool)
    .await
    .expect("count failed handoff rows");
    assert_eq!(handoff_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_handoff",
            "mailbox_thread_id",
            &thread_id,
        )
        .await,
        0,
        "failed handoff FK insert must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_checkpoint_refs_do_not_emit_false_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-invalid-checkpoint-{}", Uuid::now_v7());
    let result = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: local_lane("invalid-checkpoint-ref"),
            session_id: "session-invalid-checkpoint-ref".to_string(),
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-214".to_string(),
            claim_id: Some(format!("PSR-CLAIM-missing-{}", Uuid::now_v7())),
            mailbox_handoff_id: Some(format!("PSR-HANDOFF-missing-{}", Uuid::now_v7())),
            navigation_command_id: Some("symbols".to_string()),
            resume_pointer: RecoveryResumePointer::MicroTask {
                mt_id: "MT-214".to_string(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec!["cargo test --test parallel_swarm_state_recovery_tests".to_string()],
            hbr_rows: vec!["HBR-SWARM-004".to_string()],
            next_step_context: "invalid refs should fail before receipt".to_string(),
            payload: json!({"invalid_refs": true}),
            compaction_reason: "test_invalid_refs".to_string(),
            git_head: "deadbeef".to_string(),
        })
        .await;
    assert!(
        result.is_err(),
        "invalid claim/mailbox FKs should reject checkpoint"
    );

    let checkpoint_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_state_recovery_checkpoints WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count failed checkpoint rows");
    assert_eq!(checkpoint_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_checkpoint",
            "workspace_id",
            &workspace_id,
        )
        .await,
        0,
        "failed checkpoint FK insert must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_scope_claim_records_one_durable_claim_event() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    install_parallel_swarm_event_delay(&pool, "parallel_swarm_claim").await;

    let workspace_id = format!("workspace-claim-race-{}", Uuid::now_v7());
    let scope = ClaimScope::Worktree {
        worktree_id: format!("wtc-kernel-009-race-{}", Uuid::now_v7()),
    };
    let barrier = Arc::new(Barrier::new(2));
    let left_store = store.clone();
    let left_barrier = barrier.clone();
    let left_workspace = workspace_id.clone();
    let left_scope = scope.clone();
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        left_store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: left_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-210".to_string()),
                scope: left_scope,
                lane: local_lane("claim-race-a"),
                session_id: "session-claim-race-a".to_string(),
                ttl_seconds: 600,
                reason: "left concurrent claim".to_string(),
            })
            .await
    });
    let right_store = store.clone();
    let right_barrier = barrier.clone();
    let right_workspace = workspace_id.clone();
    let right_scope = scope.clone();
    let right = tokio::spawn(async move {
        right_barrier.wait().await;
        right_store
            .claim_work_surface(WorkClaimRequest {
                workspace_id: right_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: Some("MT-210".to_string()),
                scope: right_scope,
                lane: local_lane("claim-race-b"),
                session_id: "session-claim-race-b".to_string(),
                ttl_seconds: 600,
                reason: "right concurrent claim".to_string(),
            })
            .await
    });

    let left = left.await.expect("left claim task joins");
    let right = right.await.expect("right claim task joins");
    assert!(
        left.is_ok(),
        "left claim should resolve to an outcome: {left:?}"
    );
    assert!(
        right.is_ok(),
        "right claim should resolve to an outcome: {right:?}"
    );
    let outcomes = [left.expect("left outcome"), right.expect("right outcome")];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == ClaimStatus::Active)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.status == ClaimStatus::Held)
            .count(),
        1
    );

    let claim_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_worktree_claims WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count claim race rows");
    assert_eq!(claim_rows, 1);

    let claim_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_swarm_claim'
          AND payload ->> 'workspace_id' = $1
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count claim race events");
    assert_eq!(
        claim_events, claim_rows,
        "claim race losers must not leave false durable EventLedger claim events"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_navigation_checkpoint_and_recovery_are_restartable_from_postgres() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let lane = cloud_lane("recover");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: "workspace-recovery".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-213".to_string()),
            scope: ClaimScope::Workspace {
                workspace_id: "workspace-recovery".to_string(),
            },
            lane: lane.clone(),
            session_id: "session-cloud-recover".to_string(),
            ttl_seconds: 600,
            reason: "checkpoint recovery proof".to_string(),
        })
        .await
        .expect("workspace claim");

    let handoff = store
        .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
            from_lane: lane.clone(),
            to_role: "WP_VALIDATOR".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-213".to_string(),
            claim_id: Some(claim.claim_id.clone()),
            mailbox_thread_id: "thread-mt-213".to_string(),
            mailbox_message_id: "message-mt-213".to_string(),
            status: SwarmReceiptStatus::Blocked,
            summary: "validator should inspect checkpoint receipt shape".to_string(),
            body_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
        })
        .await
        .expect("mailbox handoff receipt");
    assert!(handoff.event_ledger_event_id.starts_with("KE-"));

    let nav = NavigationCommandSet::default();
    let ids: HashSet<_> = nav.commands().iter().map(|cmd| cmd.command_id).collect();
    assert_eq!(ids.len(), nav.commands().len(), "command ids are unique");
    for expected in [
        "sources",
        "symbols",
        "docs",
        "graph",
        "retrieval_traces",
        "user_manual_pages",
        "repair_queue",
        "validation_state",
    ] {
        assert!(
            ids.contains(expected),
            "missing navigation command {expected}"
        );
    }
    let resolved_once = nav
        .resolve(
            BackendNavigationCommand::Symbols,
            json!({"workspace_id": "workspace-recovery", "name": "AgentLaneIdentity"}),
        )
        .expect("symbols command resolves");
    let resolved_twice = nav
        .resolve(
            BackendNavigationCommand::Symbols,
            json!({"name": "AgentLaneIdentity", "workspace_id": "workspace-recovery"}),
        )
        .expect("symbols command resolves deterministically");
    assert_eq!(
        resolved_once.deterministic_cache_key, resolved_twice.deterministic_cache_key,
        "same command and params must produce a stable backend navigation key"
    );

    let checkpoint = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: lane.clone(),
            session_id: "session-cloud-recover".to_string(),
            workspace_id: "workspace-recovery".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-214".to_string(),
            claim_id: Some(claim.claim_id.clone()),
            mailbox_handoff_id: Some(handoff.handoff_id.clone()),
            navigation_command_id: Some(resolved_once.command_id.to_string()),
            resume_pointer: RecoveryResumePointer::MicroTask {
                mt_id: "MT-214".to_string(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec!["cargo test --test parallel_swarm_state_recovery_tests".to_string()],
            hbr_rows: vec!["HBR-SWARM-001".to_string(), "HBR-SWARM-004".to_string()],
            next_step_context: "resume at checkpoint recovery flow".to_string(),
            payload: json!({
                "pending": "implement minimal backend recovery flow",
                "chat_history_required": false
            }),
            compaction_reason: "session_compaction".to_string(),
            git_head: "abc1234".to_string(),
        })
        .await
        .expect("record checkpoint");
    assert!(checkpoint.event_ledger_event_id.starts_with("KE-"));

    let resumed_lane = local_lane("resume");
    let recovered = store
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            resumed_lane,
            "session-local-resume",
        )
        .await
        .expect("recover from checkpoint");
    assert_eq!(recovered.checkpoint.checkpoint_id, checkpoint.checkpoint_id);
    assert_eq!(
        recovered.resume_pointer,
        RecoveryResumePointer::MicroTask {
            mt_id: "MT-214".to_string()
        }
    );
    assert_eq!(recovered.checkpoint.touched_files, checkpoint.touched_files);
    assert_eq!(recovered.receipt.prior_session_id, "session-cloud-recover");
    assert_eq!(recovered.receipt.new_session_id, "session-local-resume");
    assert!(
        !serde_json::to_string(&recovered)
            .expect("serialize recovery")
            .contains("sk-test-secret"),
        "recovery evidence must not leak provider secrets"
    );

    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_recovery_receipts WHERE checkpoint_id = $1",
    )
    .bind(&checkpoint.checkpoint_id)
    .fetch_one(&pool)
    .await
    .expect("count recovery receipts");
    assert_eq!(receipt_count, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recovery_receipt_authority_failure_does_not_emit_false_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-recovery-authority-fail-{}", Uuid::now_v7());
    let checkpoint = store
        .record_checkpoint(RecoveryCheckpointRequest {
            lane: local_lane("recovery-authority-source"),
            session_id: "session-recovery-authority-source".to_string(),
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-214".to_string(),
            claim_id: None,
            mailbox_handoff_id: None,
            navigation_command_id: Some("validation_state".to_string()),
            resume_pointer: RecoveryResumePointer::MicroTask {
                mt_id: "MT-214".to_string(),
            },
            touched_files: vec![
                "src/backend/handshake_core/src/swarm_orchestration/state_recovery.rs".to_string(),
            ],
            tests: vec!["cargo test --test parallel_swarm_state_recovery_tests".to_string()],
            hbr_rows: vec!["HBR-SWARM-004".to_string()],
            next_step_context: "valid checkpoint before forced recovery insert failure".to_string(),
            payload: json!({"checkpoint": "valid"}),
            compaction_reason: "test_recovery_authority_failure".to_string(),
            git_head: "feedface".to_string(),
        })
        .await
        .expect("valid checkpoint before forced recovery authority failure");

    install_recovery_receipt_authority_failure(&pool).await;
    let result = store
        .recover_from_checkpoint(
            &checkpoint.checkpoint_id,
            local_lane("recovery-authority-fail"),
            "session-recovery-authority-fail",
        )
        .await;
    assert!(
        result.is_err(),
        "forced authority failure should reject recovery receipt"
    );

    let receipt_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_agent_recovery_receipts WHERE checkpoint_id = $1",
    )
    .bind(&checkpoint.checkpoint_id)
    .fetch_one(&pool)
    .await
    .expect("count failed recovery receipt rows");
    assert_eq!(receipt_rows, 0);
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_recovery",
            "new_session_id",
            "session-recovery-authority-fail",
        )
        .await,
        0,
        "failed recovery receipt insert must not leave a false EventLedger receipt"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mailbox_handoff_statuses_round_trip_from_postgres() {
    let Some((_pool, store)) = recovery_store().await else {
        return;
    };

    for status in [
        SwarmReceiptStatus::Started,
        SwarmReceiptStatus::Progress,
        SwarmReceiptStatus::Blocked,
        SwarmReceiptStatus::Pass,
        SwarmReceiptStatus::Fail,
    ] {
        let status_name = format!("{status:?}").to_ascii_lowercase();
        let handoff = store
            .record_role_mailbox_handoff(RoleMailboxHandoffRequest {
                from_lane: cloud_lane(&format!("handoff-{status_name}")),
                to_role: "WP_VALIDATOR".to_string(),
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-211".to_string(),
                claim_id: None,
                mailbox_thread_id: format!("thread-{status_name}"),
                mailbox_message_id: format!("message-{status_name}"),
                status,
                summary: format!("round-trip {status_name} handoff status"),
                body_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                    .to_string(),
            })
            .await
            .expect("record handoff status");
        assert_eq!(
            handoff.status, status,
            "mailbox handoff status must decode from the database row"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn raw_secret_like_provider_metadata_is_scrubbed_at_persist_time() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-raw-attribution-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-215".to_string()),
            scope: ClaimScope::Workspace {
                workspace_id: workspace_id.clone(),
            },
            lane: raw_cloud_lane("persist-time"),
            session_id: "session-raw-attribution".to_string(),
            ttl_seconds: 600,
            reason: "raw metadata should be scrubbed before persistence".to_string(),
        })
        .await
        .expect("workspace claim with raw cloud attribution");
    assert_eq!(claim.status, ClaimStatus::Active);

    let persisted_attribution: String = sqlx::query_scalar(
        "SELECT attribution_jsonb::text FROM knowledge_agent_worktree_claims WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .fetch_one(&pool)
    .await
    .expect("fetch persisted attribution");
    let persisted_event_payload: String =
        sqlx::query_scalar("SELECT payload::text FROM kernel_event_ledger WHERE event_id = $1")
            .bind(
                claim
                    .event_ledger_event_id
                    .as_deref()
                    .expect("claim has event receipt"),
            )
            .fetch_one(&pool)
            .await
            .expect("fetch persisted event payload");

    for persisted in [&persisted_attribution, &persisted_event_payload] {
        assert!(
            !persisted.contains("sk-raw-secret-must-not-persist"),
            "raw API key leaked into persisted attribution/event payload: {persisted}"
        );
        assert!(
            !persisted.contains("raw-token-must-not-persist"),
            "raw token leaked into persisted attribution/event payload: {persisted}"
        );
        assert!(
            persisted.contains("[REDACTED]"),
            "persisted metadata should retain redaction markers: {persisted}"
        );
        assert!(
            persisted.contains("org-visible"),
            "non-secret provider metadata should remain useful: {persisted}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_indexing_lease_queue_serializes_same_scope_writers_and_reclaims_orphans() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    let scope = ClaimScope::IndexRun {
        workspace_id: "workspace-index".to_string(),
        source_root_id: "root-a".to_string(),
    };
    let first = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: "workspace-index".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-216".to_string(),
            scope: scope.clone(),
            lane: local_lane("index-a"),
            session_id: "session-index-a".to_string(),
            index_run_id: format!("index-run-{}", Uuid::now_v7()),
            priority: 10,
            ttl_seconds: 600,
            quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        })
        .await
        .expect("first index lease");
    assert_eq!(first.status, IndexLeaseStatus::Acquired);

    let second = store
        .enqueue_indexing_lease(IndexingLeaseRequest {
            workspace_id: "workspace-index".to_string(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: "MT-216".to_string(),
            scope: scope.clone(),
            lane: local_lane("index-b"),
            session_id: "session-index-b".to_string(),
            index_run_id: format!("index-run-{}", Uuid::now_v7()),
            priority: 20,
            ttl_seconds: 600,
            quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
        })
        .await
        .expect("second index lease queues");
    assert_eq!(second.status, IndexLeaseStatus::Queued);
    assert_eq!(
        second.blocked_by_lease_id.as_deref(),
        Some(first.lease_id.as_str())
    );
    let (queued_event_type, queued_status) =
        ledger_event_type_and_status(&pool, &second.event_ledger_event_id).await;
    assert_eq!(queued_event_type, "SESSION_QUEUED");
    assert_eq!(queued_status, "queued");

    let active = store
        .active_index_writer_for_scope(&scope)
        .await
        .expect("active index writer")
        .expect("writer exists");
    assert_eq!(active.lease_id, first.lease_id);

    store
        .complete_indexing_lease(&first.lease_id, &local_lane("index-a"))
        .await
        .expect("complete first lease");
    let promoted = store
        .acquire_next_indexing_lease(&scope)
        .await
        .expect("acquire next")
        .expect("queued lease promoted");
    assert_eq!(promoted.lease_id, second.lease_id);
    assert_eq!(promoted.status, IndexLeaseStatus::Acquired);
    assert_ne!(
        promoted.event_ledger_event_id, second.event_ledger_event_id,
        "queued lease promotion must attach a fresh acquired receipt"
    );
    let (promoted_event_type, promoted_status) =
        ledger_event_type_and_status(&pool, &promoted.event_ledger_event_id).await;
    assert_eq!(promoted_event_type, "KNOWLEDGE_INDEX_RUN_STARTED");
    assert_eq!(promoted_status, "acquired");

    sqlx::query(
        "UPDATE knowledge_parallel_indexing_lease_queue SET expires_at_utc = NOW() - INTERVAL '1 second' WHERE lease_id = $1",
    )
    .bind(&promoted.lease_id)
    .execute(&pool)
    .await
    .expect("force orphaned lease in isolated test schema");
    let reclaimed = store
        .reclaim_orphaned_indexing_leases()
        .await
        .expect("reclaim orphans");
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].lease_id, promoted.lease_id);
    assert_eq!(reclaimed[0].status, IndexLeaseStatus::Reclaimed);
    assert_ne!(
        reclaimed[0].event_ledger_event_id, promoted.event_ledger_event_id,
        "orphan reclaim must attach a fresh reclaimed receipt"
    );
    let (reclaimed_event_type, reclaimed_status) =
        ledger_event_type_and_status(&pool, &reclaimed[0].event_ledger_event_id).await;
    assert_eq!(reclaimed_event_type, "KNOWLEDGE_INDEX_RUN_CANCELLED");
    assert_eq!(reclaimed_status, "reclaimed");
    let reclaimed_payload: serde_json::Value =
        sqlx::query_scalar("SELECT payload FROM kernel_event_ledger WHERE event_id = $1")
            .bind(&reclaimed[0].event_ledger_event_id)
            .fetch_one(&pool)
            .await
            .expect("reclaimed lease event payload");
    assert_eq!(reclaimed_payload["quiet_policy"]["work_kind"], "indexing");
    assert_eq!(
        reclaimed_payload["quiet_policy"]["no_foreground_window"],
        true
    );
    assert_eq!(reclaimed_payload["quiet_policy"]["no_focus_steal"], true);
    assert_eq!(
        reclaimed_payload["quiet_policy"]["no_os_shell_window"],
        true
    );
    assert!(
        store
            .active_index_writer_for_scope(&scope)
            .await
            .expect("active writer after reclaim")
            .is_none(),
        "expired writer must not remain active after reclaim"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_same_scope_indexing_lease_records_only_real_outcome_events() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };
    install_parallel_swarm_event_delay(&pool, "parallel_indexing_lease").await;

    let workspace_id = format!("workspace-lease-race-{}", Uuid::now_v7());
    let scope = ClaimScope::IndexRun {
        workspace_id: workspace_id.clone(),
        source_root_id: "root-lease-race".to_string(),
    };
    let barrier = Arc::new(Barrier::new(2));
    let left_store = store.clone();
    let left_barrier = barrier.clone();
    let left_workspace = workspace_id.clone();
    let left_scope = scope.clone();
    let left = tokio::spawn(async move {
        left_barrier.wait().await;
        left_store
            .enqueue_indexing_lease(IndexingLeaseRequest {
                workspace_id: left_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-216".to_string(),
                scope: left_scope,
                lane: local_lane("lease-race-a"),
                session_id: "session-lease-race-a".to_string(),
                index_run_id: format!("index-run-{}", Uuid::now_v7()),
                priority: 10,
                ttl_seconds: 600,
                quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
            })
            .await
    });
    let right_store = store.clone();
    let right_barrier = barrier.clone();
    let right_workspace = workspace_id.clone();
    let right_scope = scope.clone();
    let right = tokio::spawn(async move {
        right_barrier.wait().await;
        right_store
            .enqueue_indexing_lease(IndexingLeaseRequest {
                workspace_id: right_workspace,
                wp_id: "WP-KERNEL-009".to_string(),
                mt_id: "MT-216".to_string(),
                scope: right_scope,
                lane: local_lane("lease-race-b"),
                session_id: "session-lease-race-b".to_string(),
                index_run_id: format!("index-run-{}", Uuid::now_v7()),
                priority: 20,
                ttl_seconds: 600,
                quiet_policy: QuietBackgroundPolicy::quiet_for(QuietBackgroundWorkKind::Indexing),
            })
            .await
    });

    let left = left.await.expect("left lease task joins");
    let right = right.await.expect("right lease task joins");
    assert!(
        left.is_ok(),
        "left lease should resolve to an outcome: {left:?}"
    );
    assert!(
        right.is_ok(),
        "right lease should resolve to an outcome: {right:?}"
    );
    let outcomes = [left.expect("left lease"), right.expect("right lease")];
    assert_eq!(
        outcomes
            .iter()
            .filter(|lease| lease.status == IndexLeaseStatus::Acquired)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|lease| lease.status == IndexLeaseStatus::Queued)
            .count(),
        1
    );

    let lease_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM knowledge_parallel_indexing_lease_queue WHERE workspace_id = $1",
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count lease race rows");
    assert_eq!(lease_rows, 2);

    let acquired_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_indexing_lease'
          AND payload ->> 'workspace_id' = $1
          AND payload ->> 'status' = 'acquired'
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count acquired lease events");
    let queued_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_indexing_lease'
          AND payload ->> 'workspace_id' = $1
          AND payload ->> 'status' = 'queued'
        "#,
    )
    .bind(&workspace_id)
    .fetch_one(&pool)
    .await
    .expect("count queued lease events");
    assert_eq!(
        acquired_events, 1,
        "only the real acquired lease may emit an acquired event"
    );
    assert_eq!(
        queued_events, 1,
        "the race loser should persist as a queued lease with a queued event"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explicit_expired_claim_reclaim_records_event_receipt() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-explicit-reclaim-{}", Uuid::now_v7());
    let lane = local_lane("explicit-reclaim-holder");
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id: workspace_id.clone(),
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-reclaim-{}", Uuid::now_v7()),
            },
            lane: lane.clone(),
            session_id: "session-explicit-reclaim-holder".to_string(),
            ttl_seconds: 600,
            reason: "claim that will be made stale in the isolated test schema".to_string(),
        })
        .await
        .expect("claim to reclaim");
    sqlx::query(
        "UPDATE knowledge_agent_worktree_claims SET expires_at_utc = NOW() - INTERVAL '1 second' WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .execute(&pool)
    .await
    .expect("force expired claim in isolated test schema");

    let reclaimed = store
        .reclaim_expired_work_claims(
            &local_lane("explicit-reclaimer"),
            "session-explicit-reclaim",
            "explicit stale claim sweep",
        )
        .await
        .expect("explicit reclaim expired claims");
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].claim_id, claim.claim_id);
    assert_eq!(reclaimed[0].status, ClaimStatus::Reclaimed);
    assert!(
        reclaimed[0]
            .reclaim_event_ledger_event_id
            .as_deref()
            .is_some_and(|event_id| event_id.starts_with("KE-")),
        "reclaimed claims must carry a reclaim event receipt"
    );

    let receipt_events: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kernel_event_ledger
        WHERE source_component = 'parallel_swarm_state_recovery'
          AND aggregate_type = 'parallel_swarm_claim_reclaim'
          AND payload ->> 'claim_id' = $1
          AND payload ->> 'reason' = 'explicit stale claim sweep'
        "#,
    )
    .bind(&claim.claim_id)
    .fetch_one(&pool)
    .await
    .expect("count reclaim receipt events");
    assert_eq!(receipt_events, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn expired_claim_reclaim_rolls_back_if_receipt_insert_fails() {
    let Some((pool, store)) = recovery_store().await else {
        return;
    };

    let workspace_id = format!("workspace-reclaim-failure-{}", Uuid::now_v7());
    let claim = store
        .claim_work_surface(WorkClaimRequest {
            workspace_id,
            wp_id: "WP-KERNEL-009".to_string(),
            mt_id: Some("MT-210".to_string()),
            scope: ClaimScope::Worktree {
                worktree_id: format!("wtc-kernel-009-reclaim-fail-{}", Uuid::now_v7()),
            },
            lane: local_lane("reclaim-failure-holder"),
            session_id: "session-reclaim-failure-holder".to_string(),
            ttl_seconds: 600,
            reason: "claim that should remain active if reclaim receipt fails".to_string(),
        })
        .await
        .expect("claim before forced reclaim receipt failure");
    sqlx::query(
        "UPDATE knowledge_agent_worktree_claims SET expires_at_utc = NOW() - INTERVAL '1 second' WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .execute(&pool)
    .await
    .expect("force expired claim before failure injection");

    install_parallel_swarm_event_failure(&pool, "parallel_swarm_claim_reclaim").await;
    let result = store
        .reclaim_expired_work_claims(
            &local_lane("reclaim-failure-reclaimer"),
            "session-reclaim-failure",
            "forced reclaim receipt failure",
        )
        .await;
    assert!(
        result.is_err(),
        "forced EventLedger failure should reject reclaim"
    );

    let (status, reclaim_event_id): (String, Option<String>) = sqlx::query_as(
        "SELECT status, reclaim_event_ledger_event_id FROM knowledge_agent_worktree_claims WHERE claim_id = $1",
    )
    .bind(&claim.claim_id)
    .fetch_one(&pool)
    .await
    .expect("fetch claim after failed reclaim");
    assert_eq!(
        status, "active",
        "claim must not be left reclaimed when receipt insertion fails"
    );
    assert!(
        reclaim_event_id.is_none(),
        "failed reclaim must not attach a missing receipt id"
    );
    assert_eq!(
        ledger_count_for_payload_value(
            &pool,
            "parallel_swarm_claim_reclaim",
            "claim_id",
            &claim.claim_id,
        )
        .await,
        0,
        "failed reclaim must not leave a false EventLedger receipt"
    );
}
