use handshake_core::diagnostics::{DiagFilter, Diagnostic, DiagnosticsStore, ProblemGroup};
use handshake_core::flight_recorder::{
    EventFilter, FlightRecorder, FlightRecorderEvent, FlightRecorderEventType, RecorderError,
};
use handshake_core::kernel::{
    DccMvpRuntimeSurfaceV1, DummyEchoModelAdapter, KernelEventType, KernelProofRunner,
    KernelTraceInspector, OperatorPromotionApproval, StructuredSummaryModelAdapter,
    TraceProjection,
};
use handshake_core::llm::{
    CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage,
};
use handshake_core::storage::{
    postgres::PostgresDatabase, tests::postgres_backend_from_env, Database, SessionMessageRole,
    StorageError,
};
use handshake_core::workflows::{SessionRegistry, SessionSchedulerConfig};
use handshake_core::{capabilities::CapabilityRegistry, AppState};
use serde_json::json;
use sqlx::Connection;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

async fn postgres_or_environment_blocked() -> Arc<dyn handshake_core::storage::Database> {
    match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!(
                "ENVIRONMENT_BLOCKED: Kernel V1 Postgres proof requires POSTGRES_TEST_URL; {msg}"
            );
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    }
}

async fn postgres_reopenable_or_environment_blocked() -> (String, Arc<dyn Database>) {
    match postgres_reopenable_backend_from_env().await {
        Ok(pair) => pair,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            panic!(
                "ENVIRONMENT_BLOCKED: Kernel V1 Postgres restart proof requires POSTGRES_TEST_URL; {msg}"
            );
        }
        Err(err) => panic!("failed to init reopenable postgres backend: {err:?}"),
    }
}

async fn postgres_reopenable_backend_from_env() -> Result<(String, Arc<dyn Database>), StorageError>
{
    let url = std::env::var("POSTGRES_TEST_URL")
        .map_err(|_| StorageError::Validation("POSTGRES_TEST_URL not set for postgres tests"))?;
    let mut conn = sqlx::PgConnection::connect(&url).await?;
    let schema = format!("kernel_restart_{}", Uuid::now_v7().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&mut conn)
        .await?;
    drop(conn);

    let sep = if url.contains('?') { "&" } else { "?" };
    let schema_url = format!("{url}{sep}options=-csearch_path%3D{schema}");
    let db = PostgresDatabase::connect(&schema_url, 5).await?;
    db.run_migrations().await?;
    Ok((schema_url, db.into_arc()))
}

#[derive(Default)]
struct CapturingFlightRecorder {
    events: Mutex<Vec<FlightRecorderEvent>>,
}

impl CapturingFlightRecorder {
    fn events(&self) -> Vec<FlightRecorderEvent> {
        self.events.lock().expect("recorder lock").clone()
    }
}

#[async_trait::async_trait]
impl FlightRecorder for CapturingFlightRecorder {
    async fn record_event(&self, event: FlightRecorderEvent) -> Result<(), RecorderError> {
        event.validate()?;
        self.events
            .lock()
            .map_err(|_| RecorderError::LockError)?
            .push(event);
        Ok(())
    }

    async fn enforce_retention(&self) -> Result<u64, RecorderError> {
        Ok(0)
    }

    async fn list_events(
        &self,
        _filter: EventFilter,
    ) -> Result<Vec<FlightRecorderEvent>, RecorderError> {
        Ok(self.events())
    }
}

struct EmptyDiagnosticsStore;

#[async_trait::async_trait]
impl DiagnosticsStore for EmptyDiagnosticsStore {
    async fn record_diagnostic(&self, _diag: Diagnostic) -> Result<(), StorageError> {
        Ok(())
    }

    async fn list_problems(&self, _filter: DiagFilter) -> Result<Vec<ProblemGroup>, StorageError> {
        Ok(Vec::new())
    }

    async fn get_diagnostic(&self, _id: uuid::Uuid) -> Result<Diagnostic, StorageError> {
        Err(StorageError::NotFound("diagnostic"))
    }

    async fn list_diagnostics(&self, _filter: DiagFilter) -> Result<Vec<Diagnostic>, StorageError> {
        Ok(Vec::new())
    }
}

struct TestLlmClient {
    profile: ModelProfile,
}

impl TestLlmClient {
    fn new() -> Self {
        Self {
            profile: ModelProfile::new("kernel-api-test".to_string(), 4096),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for TestLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: "ok".to_string(),
            usage: TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            },
            latency_ms: 0,
        })
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn test_app_state(db: Arc<dyn handshake_core::storage::Database>) -> AppState {
    let flight_recorder: Arc<dyn FlightRecorder> = Arc::new(CapturingFlightRecorder::default());
    AppState {
        storage: db,
        flight_recorder,
        diagnostics: Arc::new(EmptyDiagnosticsStore),
        llm_client: Arc::new(TestLlmClient::new()),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    }
}

fn write_trace_evidence(label: &str, result: &handshake_core::kernel::KernelProofResult) {
    let Ok(output_dir) = std::env::var("KERNEL_TRACE_PROOF_OUTPUT_DIR") else {
        return;
    };
    let output_dir = PathBuf::from(output_dir);
    std::fs::create_dir_all(&output_dir).expect("create trace evidence output dir");
    let output_path = output_dir.join(format!("{label}-trace-projection.json"));
    let event_ids: Vec<_> = result
        .trace
        .events
        .iter()
        .map(|event| event.event_id.clone())
        .collect();
    let artifact_ids: Vec<_> = result
        .trace
        .events
        .iter()
        .filter_map(|event| {
            event
                .payload
                .get("artifact_id")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .collect();
    let evidence = json!({
        "label": label,
        "kernel_task_run_id": result.kernel_task_run_id,
        "session_run_id": result.session_run_id,
        "event_ids": event_ids,
        "artifact_ids": artifact_ids,
        "trace_output_path": output_path.display().to_string(),
        "trace": result.trace
    });
    std::fs::write(
        &output_path,
        serde_json::to_vec_pretty(&evidence).expect("serialize trace evidence"),
    )
    .expect("write trace evidence");
}

#[tokio::test]
async fn end_to_end_kernel_proof() {
    let db = postgres_or_environment_blocked().await;

    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let result = KernelProofRunner::new(db.clone())
        .run_first_slice(
            "operator-import",
            json!({"intent": "prove postgresql kernel authority"}),
            &adapter,
            OperatorPromotionApproval::new("operator-ilja", "approve kernel proof"),
        )
        .await
        .expect("kernel proof run");

    assert!(result.kernel_task_run_id.starts_with("KTR-"));
    assert!(result.session_run_id.starts_with("SR-"));
    assert_eq!(result.trace.authority_source, "postgres_event_ledger");
    write_trace_evidence("end-to-end-kernel-proof", &result);
    for event_type in [
        KernelEventType::TaskIntentRecorded,
        KernelEventType::SessionQueued,
        KernelEventType::SessionClaimed,
        KernelEventType::SessionStarted,
        KernelEventType::ContextBundleRecorded,
        KernelEventType::ModelAdapterInvoked,
        KernelEventType::ModelResponseRecorded,
        KernelEventType::ToolRequestRecorded,
        KernelEventType::ToolDecisionRecorded,
        KernelEventType::ArtifactProposed,
        KernelEventType::ArtifactStored,
        KernelEventType::ValidationRecorded,
        KernelEventType::PromotionDecided,
        KernelEventType::SessionCompleted,
    ] {
        assert!(
            result.trace.contains_event_type(event_type.clone()),
            "trace missing {:?}",
            event_type
        );
    }

    let messages = db
        .list_session_messages(&result.session_run_id)
        .await
        .expect("session messages linked to kernel run");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, SessionMessageRole::User);
    assert_eq!(messages[1].role, SessionMessageRole::Assistant);
    assert!(messages[0]
        .attachments
        .iter()
        .any(|attachment| attachment.starts_with("kernel_event_id:")));
    assert!(messages[1]
        .attachments
        .iter()
        .any(|attachment| attachment.starts_with("kernel_event_id:")));
}

#[tokio::test]
async fn broker_dispatch_to_adapter_session_messages_ledger_link_toolgate_ledger_bridge_artifact_store_ledger_link(
) {
    let db = postgres_or_environment_blocked().await;

    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let result = KernelProofRunner::new(db.clone())
        .run_first_slice(
            "operator-import",
            json!({"intent": "link every broker phase"}),
            &adapter,
            OperatorPromotionApproval::new("operator-ilja", "approve kernel proof"),
        )
        .await
        .expect("kernel proof run");

    assert!(result
        .trace
        .contains_event_type(KernelEventType::ModelAdapterInvoked));
    assert!(result
        .trace
        .contains_event_type(KernelEventType::ModelResponseRecorded));

    let tool_decision = result
        .trace
        .events
        .iter()
        .find(|event| event.event_type == KernelEventType::ToolDecisionRecorded)
        .expect("toolgate decision event");
    assert_eq!(tool_decision.actor.actor_kind(), "toolgate");
    assert_eq!(tool_decision.payload["decision"], "allow");

    let artifact_stored = result
        .trace
        .events
        .iter()
        .find(|event| event.event_type == KernelEventType::ArtifactStored)
        .expect("artifact stored event");
    assert!(artifact_stored.payload["artifact_id"]
        .as_str()
        .expect("artifact_id string")
        .starts_with("ART-"));

    let messages = db
        .list_session_messages(&result.session_run_id)
        .await
        .expect("session messages linked to kernel run");
    assert_eq!(messages.len(), 2);
    assert!(messages
        .iter()
        .any(|message| message.role == SessionMessageRole::User));
    assert!(messages
        .iter()
        .any(|message| message.role == SessionMessageRole::Assistant));
    assert!(messages.iter().all(|message| message
        .attachments
        .iter()
        .any(|attachment| attachment.starts_with("kernel_message_payload_hash:"))));
}

#[tokio::test]
async fn restart_reconstruction_proof() {
    let (schema_url, db) = postgres_reopenable_or_environment_blocked().await;

    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let result = KernelProofRunner::new(db.clone())
        .run_first_slice(
            "operator-import",
            json!({"intent": "restart proof"}),
            &adapter,
            OperatorPromotionApproval::new("operator-ilja", "approve kernel proof"),
        )
        .await
        .expect("kernel proof run");

    let kernel_task_run_id = result.kernel_task_run_id.clone();
    let session_run_id = result.session_run_id.clone();
    let original_event_count = result.trace.event_count;
    drop(result);
    drop(db);

    let reopened_db = PostgresDatabase::connect(&schema_url, 5)
        .await
        .expect("reopen same postgres schema")
        .into_arc();
    let reopened_events = reopened_db
        .list_kernel_events_for_session(&session_run_id)
        .await
        .expect("reload events from durable store");
    let replay = TraceProjection::from_events(kernel_task_run_id, session_run_id, reopened_events)
        .expect("trace replay after storage reopen");

    assert!(replay.contains_event_type(KernelEventType::PromotionDecided));
    assert!(replay.contains_event_type(KernelEventType::SessionCompleted));
    assert_eq!(replay.event_count, original_event_count);
}

#[tokio::test]
async fn kernel_trace_inspector() {
    let db = postgres_or_environment_blocked().await;

    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let result = KernelProofRunner::new(db.clone())
        .run_first_slice(
            "operator-import",
            json!({"intent": "inspectable trace proof"}),
            &adapter,
            OperatorPromotionApproval::new("operator-ilja", "approve kernel proof"),
        )
        .await
        .expect("kernel proof run");

    let inspected = KernelTraceInspector::new(db)
        .inspect_session(&result.kernel_task_run_id, &result.session_run_id)
        .await
        .expect("inspect kernel trace");

    assert_eq!(inspected.kernel_task_run_id, result.kernel_task_run_id);
    assert_eq!(inspected.session_run_id, result.session_run_id);
    assert_eq!(inspected.authority_source, "postgres_event_ledger");
    assert!(inspected.contains_event_type(KernelEventType::ValidationRecorded));
    assert!(inspected.contains_event_type(KernelEventType::PromotionDecided));
}

#[tokio::test]
async fn kernel_trace_inspector_api_route_returns_trace_projection() {
    let db = postgres_or_environment_blocked().await;

    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let result = KernelProofRunner::new(db.clone())
        .run_first_slice(
            "operator-import",
            json!({"intent": "api inspectable trace proof"}),
            &adapter,
            OperatorPromotionApproval::new("operator-ilja", "approve kernel proof"),
        )
        .await
        .expect("kernel proof run");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test API listener");
    let addr = listener.local_addr().expect("test API listener addr");
    let app = handshake_core::api::kernel::routes(test_app_state(db));
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("kernel trace API server");
    });

    let url = format!(
        "http://{addr}/kernel/trace_projection?kernel_task_run_id={}&session_run_id={}",
        result.kernel_task_run_id, result.session_run_id
    );
    let projection = reqwest::get(url)
        .await
        .expect("trace projection response")
        .error_for_status()
        .expect("successful trace projection status")
        .json::<TraceProjection>()
        .await
        .expect("trace projection JSON");
    server.abort();

    assert_eq!(projection.kernel_task_run_id, result.kernel_task_run_id);
    assert_eq!(projection.session_run_id, result.session_run_id);
    assert_eq!(projection.authority_source, "postgres_event_ledger");
    assert!(projection.contains_event_type(KernelEventType::PromotionDecided));
}

#[tokio::test]
async fn kernel_dcc_projection_api_route_returns_backend_validated_surface() {
    let db = postgres_or_environment_blocked().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test API listener");
    let addr = listener.local_addr().expect("test API listener addr");
    let app = handshake_core::api::kernel::routes(test_app_state(db));
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("kernel DCC projection API server");
    });

    let surface = reqwest::get(format!("http://{addr}/kernel/dcc_projection"))
        .await
        .expect("DCC projection response")
        .error_for_status()
        .expect("successful DCC projection status")
        .json::<DccMvpRuntimeSurfaceV1>()
        .await
        .expect("DCC projection JSON");
    server.abort();

    assert_eq!(surface.schema_id, "hsk.kernel.dcc_mvp_runtime_surface@1");
    assert!(!surface.panels.is_empty());
    assert!(!surface.work_items.is_empty());
    assert!(!surface.write_box_queue_rows.is_empty());
    assert!(!surface.direct_edit_denials.is_empty());
    assert!(!surface.promotion_previews.is_empty());
    assert!(!surface.freshness_badges.is_empty());
    assert!(surface
        .write_box_queue_rows
        .iter()
        .all(|row| !row.work_id.trim().is_empty()));
    assert!(!surface.direct_authority_mutation_allowed);
}

#[tokio::test]
async fn kernel_proof_records_flight_recorder_diagnostic_mirrors() {
    let db = postgres_or_environment_blocked().await;
    let recorder = Arc::new(CapturingFlightRecorder::default());

    let adapter = DummyEchoModelAdapter::new("dummy-echo");
    let result = KernelProofRunner::with_flight_recorder(db, recorder.clone())
        .run_first_slice(
            "operator-import",
            json!({"intent": "mirror proof"}),
            &adapter,
            OperatorPromotionApproval::new("operator-ilja", "approve mirror proof"),
        )
        .await
        .expect("kernel proof run");

    let mirror_events = recorder.events();
    assert!(mirror_events.iter().any(|event| {
        event.event_type == FlightRecorderEventType::Diagnostic
            && event.payload["diagnostic_id"] == "kernel_event_mirror"
            && event.payload["projection_only"] == true
            && event.payload["kernel_event_type"] == KernelEventType::PromotionDecided.as_str()
    }));
    assert!(result
        .trace
        .contains_event_type(KernelEventType::FlightRecorderMirrorRecorded));
    assert_eq!(result.trace.authority_source, "postgres_event_ledger");
}

#[tokio::test]
async fn adapter_replaceability_proof() {
    let db = postgres_or_environment_blocked().await;

    let first = KernelProofRunner::new(db.clone())
        .run_first_slice(
            "operator-import",
            json!({"intent": "adapter one"}),
            &DummyEchoModelAdapter::new("dummy-echo-one"),
            OperatorPromotionApproval::new("operator-ilja", "approve first adapter proof"),
        )
        .await
        .expect("first adapter run");
    let second = KernelProofRunner::new(db)
        .run_first_slice(
            "operator-import",
            json!({"intent": "adapter two"}),
            &StructuredSummaryModelAdapter::new("structured-summary-two"),
            OperatorPromotionApproval::new("operator-ilja", "approve second adapter proof"),
        )
        .await
        .expect("second adapter run");

    let first_types: Vec<_> = first
        .trace
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    let second_types: Vec<_> = second
        .trace
        .events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();

    assert_eq!(first_types, second_types);
}
