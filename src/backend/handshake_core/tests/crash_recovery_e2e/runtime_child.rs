use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use handshake_core::api::account_scope::{
    ProductLocalResourceScope, PRODUCT_LOCAL_RESOURCE_SCOPE_ENV,
};
use handshake_core::process_ledger::ReclaimResourceScope;
use handshake_core::swarm_orchestration::resource_scope::ExactResourceScopeAttribution;
use handshake_core::{
    kernel::{KernelActor, KernelEventType, NewKernelEvent, SessionRun, SessionRunState},
    process_ledger::{ProcessEngineKind, ProcessStart},
    session_checkpoint::{CheckpointStateKind, SessionCheckpoint},
    storage::{
        surreal::{bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig},
        Database,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use surrealdb::types::{Datetime, RecordId, SurrealValue};
use uuid::Uuid;

const CHILD_MODE_ENV: &str = "HS_MT195_SURREAL_CHILD";
const CHILD_DATA_DIR_ENV: &str = "HS_MT195_SURREAL_DATA_DIR";
const CHILD_READY_FILE_ENV: &str = "HS_MT195_SURREAL_READY_FILE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildReadyEvidence {
    pub original_scope: ExactResourceScopeAttribution,
    pub session_id: Uuid,
    pub checkpoint_id: Uuid,
    pub process_uuid: Uuid,
    pub os_pid: u32,
}

#[derive(Clone)]
pub struct KilledChildEvidence {
    pub data_dir: PathBuf,
    pub ready: ChildReadyEvidence,
    pub exit_status: std::process::ExitStatus,
}

pub struct SeededRecoveryEvidence {
    pub original_scope: ExactResourceScopeAttribution,
    pub session_id: Uuid,
    pub checkpoint_id: Uuid,
}

pub struct ProductStartupRecoveryEvidence {
    pub status: std::process::ExitStatus,
    pub report_file: PathBuf,
}

pub fn spawn_and_hard_kill_child(root: &Path) -> KilledChildEvidence {
    let data_dir = root.join("surreal-store");
    let ready_file = root.join("child-ready.json");
    let stdout = File::create(root.join("child-stdout.log")).expect("create child stdout");
    let stderr = File::create(root.join("child-stderr.log")).expect("create child stderr");
    let mut command = Command::new(std::env::current_exe().expect("resolve current test binary"));
    command
        .arg("--exact")
        .arg("runtime_child::mt195_runtime_child_entrypoint")
        .arg("--ignored")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_MODE_ENV, "1")
        .env(CHILD_DATA_DIR_ENV, &data_dir)
        .env(CHILD_READY_FILE_ENV, &ready_file)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    apply_quiet_process_flags(&mut command);
    let mut child = command.spawn().expect("spawn MT-195 embedded child");

    let started = Instant::now();
    while !ready_file.exists() && started.elapsed() < Duration::from_secs(120) {
        if let Some(status) = child.try_wait().expect("poll MT-195 child") {
            panic!("MT-195 child exited before readiness: {status:?}");
        }
        thread::sleep(Duration::from_millis(25));
    }
    if !ready_file.exists() {
        let _ = child.kill();
        let _ = child.wait();
        panic!("MT-195 child did not publish readiness within 120 seconds");
    }
    let ready: ChildReadyEvidence =
        serde_json::from_slice(&std::fs::read(&ready_file).expect("read MT-195 child readiness"))
            .expect("decode MT-195 child readiness");

    validate_original_scope(&ready.original_scope);
    assert_eq!(
        ready.os_pid,
        child.id(),
        "kill receipt binds only the child this test spawned"
    );
    child
        .kill()
        .expect("kill only the MT-195 child started by this test");
    let exit_status = child.wait().expect("reap killed MT-195 child");
    KilledChildEvidence {
        data_dir,
        ready,
        exit_status,
    }
}

pub fn validate_original_scope(origin: &ExactResourceScopeAttribution) {
    ProductLocalResourceScope::from_exact(origin.clone())
        .expect("schema1 nonnil/nonblank original fixture origin");
}

fn mint_original_scope() -> ExactResourceScopeAttribution {
    let origin = serde_json::from_value(serde_json::json!({
        "owner_account_id": Uuid::now_v7(), "actor_principal_id": Uuid::now_v7(),
        "authenticated_session_id": Uuid::now_v7(), "access_space_id": Uuid::now_v7(),
        "workspace_id": format!("mt195-original-workspace-{}", Uuid::now_v7()),
    }))
    .expect("typed original fixture origin");
    validate_original_scope(&origin);
    origin
}

#[derive(SurrealValue)]
struct ScopedQueueRow {
    session_run_id: String,
    kernel_task_run_id: String,
    adapter_id: String,
    state: String,
    claimed_by: Option<String>,
    lease_expires_at: Option<Datetime>,
    attempt_count: i64,
    available_at: Datetime,
    created_at: Datetime,
    updated_at: Datetime,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}
#[derive(SurrealValue)]
struct QueueCreate {
    record: RecordId,
    row: ScopedQueueRow,
}

async fn seed_original_queue(
    storage: &SurrealStorage,
    origin: &ExactResourceScopeAttribution,
    session: SessionRun,
) {
    validate_original_scope(origin);
    let expected = session.session_run_id.clone();
    let row = ScopedQueueRow {
        session_run_id: session.session_run_id.clone(),
        kernel_task_run_id: session.kernel_task_run_id,
        adapter_id: session.adapter_id,
        state: session.state.as_str().to_owned(),
        claimed_by: None,
        lease_expires_at: None,
        attempt_count: 0,
        available_at: session.created_at.into(),
        created_at: session.created_at.into(),
        updated_at: session.updated_at.into(),
        owner_account_id: origin.owner_account_id.to_string(),
        actor_principal_id: origin.actor_principal_id.to_string(),
        authenticated_session_id: origin.authenticated_session_id.to_string(),
        access_space_id: origin.access_space_id.to_string(),
        workspace_id: origin.workspace_id.to_string(),
    };
    let bindings = QueueCreate {
        record: RecordId::new("kernel_session_queue", expected.clone()),
        row,
    };
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<String, _>(
                        "CREATE $record CONTENT $row RETURN VALUE session_run_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("original scoped queue CREATE");
    assert_eq!(rows, vec![expected]);
}

#[derive(SurrealValue)]
struct ScopedCheckpointRow {
    checkpoint_id: Uuid,
    session_id: Uuid,
    model_session_id: Uuid,
    last_event_ledger_seq: i64,
    compact_state: Value,
    state_kind: String,
    pending_artifacts: Vec<String>,
    created_at_utc: DateTime<Utc>,
    created_by_process: i64,
    schema_version: i64,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}
#[derive(SurrealValue)]
struct CheckpointCreate {
    record: RecordId,
    row: ScopedCheckpointRow,
}

async fn seed_original_checkpoint(
    storage: &SurrealStorage,
    origin: &ExactResourceScopeAttribution,
    cp: &SessionCheckpoint,
) {
    validate_original_scope(origin);
    let row = ScopedCheckpointRow {
        checkpoint_id: cp.checkpoint_id.as_uuid(),
        session_id: cp.session_id,
        model_session_id: cp.model_session_id,
        last_event_ledger_seq: cp.last_event_ledger_seq,
        compact_state: cp.compact_state.clone(),
        state_kind: cp.state_kind.as_str().to_string(),
        pending_artifacts: cp.pending_artifacts.clone(),
        created_at_utc: cp.created_at_utc,
        created_by_process: i64::from(cp.created_by_process),
        schema_version: i64::from(cp.schema_version),
        owner_account_id: origin.owner_account_id.to_string(),
        actor_principal_id: origin.actor_principal_id.to_string(),
        authenticated_session_id: origin.authenticated_session_id.to_string(),
        access_space_id: origin.access_space_id.to_string(),
        workspace_id: origin.workspace_id.to_string(),
    };
    let bindings = CheckpointCreate {
        record: RecordId::new(
            "kernel_session_checkpoint",
            cp.checkpoint_id.as_uuid().to_string(),
        ),
        row,
    };
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<Uuid, _>(
                        "CREATE $record CONTENT $row RETURN VALUE checkpoint_id;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("original scoped checkpoint CREATE");
    assert_eq!(rows, vec![cp.checkpoint_id.as_uuid()]);
}

#[derive(SurrealValue)]
struct ScopedProcessRow {
    process_uuid: Uuid,
    os_pid: Option<i64>,
    parent_session_id: Option<String>,
    parent_process_id: Option<Uuid>,
    sandbox_adapter_id: Option<String>,
    sandbox_internal_id: Option<String>,
    engine_kind: String,
    started_at: DateTime<Utc>,
    stopped_at: Option<DateTime<Utc>>,
    exit_code: Option<i64>,
    stop_reason: Option<String>,
    model_artifact_sha256: Option<String>,
    work_profile_id: Option<String>,
    owner_role: String,
    owner_wp: Option<String>,
    role_id: Option<String>,
    wp_id: Option<String>,
    mt_id: Option<String>,
    sandbox_capabilities_snapshot: Value,
    metadata: Value,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}
#[derive(SurrealValue)]
struct ProcessCreate {
    record: RecordId,
    row: ScopedProcessRow,
}

async fn seed_original_process(
    storage: &SurrealStorage,
    origin: &ExactResourceScopeAttribution,
    start: &ProcessStart,
) {
    validate_original_scope(origin);
    let row = ScopedProcessRow {
        process_uuid: start.process_uuid,
        os_pid: start.os_pid.map(i64::from),
        parent_session_id: start.parent_session_id.clone(),
        parent_process_id: start.parent_process_id,
        sandbox_adapter_id: start.sandbox_adapter_id.clone(),
        sandbox_internal_id: start.sandbox_internal_id.clone(),
        engine_kind: start.engine_kind.as_str().to_string(),
        started_at: start.started_at,
        stopped_at: None,
        exit_code: None,
        stop_reason: None,
        model_artifact_sha256: start.model_artifact_sha256.clone(),
        work_profile_id: start.work_profile_id.clone(),
        owner_role: start.owner_role.clone(),
        owner_wp: start.owner_wp.clone(),
        role_id: start.role_id.clone(),
        wp_id: start.wp_id.clone(),
        mt_id: start.mt_id.clone(),
        sandbox_capabilities_snapshot: start.sandbox_capabilities_snapshot.clone(),
        metadata: start.metadata_jsonb.clone(),
        owner_account_id: origin.owner_account_id.to_string(),
        actor_principal_id: origin.actor_principal_id.to_string(),
        authenticated_session_id: origin.authenticated_session_id.to_string(),
        access_space_id: origin.access_space_id.to_string(),
        workspace_id: origin.workspace_id.to_string(),
    };
    let bindings = ProcessCreate {
        record: RecordId::new("kernel_process_lifecycle", start.process_uuid.to_string()),
        row,
    };
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<Uuid, _>(
                        "CREATE $record CONTENT $row RETURN VALUE process_uuid;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("original scoped process CREATE");
    assert_eq!(rows, vec![start.process_uuid]);
}

#[derive(SurrealValue)]
struct ScopedEventRow {
    event_id: String,
    event_version: String,
    kernel_task_run_id: String,
    session_run_id: String,
    aggregate_type: String,
    aggregate_id: String,
    idempotency_key: String,
    event_type: String,
    actor_kind: String,
    actor_id: String,
    causation_id: Option<String>,
    correlation_id: Option<String>,
    payload_hash: String,
    source_component: String,
    payload: serde_json::Value,
    wsids: Vec<String>,
    authority_resource_id: Option<RecordId>,
    authority_session_id: Option<RecordId>,
    authority_capability_id: Option<String>,
    authority_action: Option<String>,
    created_at: Datetime,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}
#[derive(SurrealValue)]
struct EventCreate {
    record: RecordId,
    row: ScopedEventRow,
}

pub async fn seed_original_replay_event(
    storage: &SurrealStorage,
    origin: &ExactResourceScopeAttribution,
    event: NewKernelEvent,
) -> i64 {
    validate_original_scope(origin);
    event.validate().expect("canonical typed replay envelope");
    let event = handshake_core::kernel::KernelEvent::from_new(event);
    let row = ScopedEventRow {
        event_id: event.event_id.clone(),
        event_version: event.event_version,
        kernel_task_run_id: event.kernel_task_run_id,
        session_run_id: event.session_run_id,
        aggregate_type: event.aggregate_type,
        aggregate_id: event.aggregate_id,
        idempotency_key: event.idempotency_key,
        event_type: event.event_type.as_str().to_owned(),
        actor_kind: event.actor.actor_kind().to_owned(),
        actor_id: event.actor.actor_id().to_owned(),
        causation_id: event.causation_id,
        correlation_id: event.correlation_id,
        payload_hash: event.payload_hash,
        source_component: event.source_component,
        payload: event.payload,
        wsids: vec![],
        authority_resource_id: None,
        authority_session_id: None,
        authority_capability_id: None,
        authority_action: None,
        created_at: event.created_at.into(),
        owner_account_id: origin.owner_account_id.to_string(),
        actor_principal_id: origin.actor_principal_id.to_string(),
        authenticated_session_id: origin.authenticated_session_id.to_string(),
        access_space_id: origin.access_space_id.to_string(),
        workspace_id: origin.workspace_id.to_string(),
    };
    let bindings = EventCreate {
        record: RecordId::new("kernel_event_ledger", row.event_id.clone()),
        row,
    };
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database
                    .query_values::<i64, _>(
                        "CREATE $record CONTENT $row RETURN VALUE event_sequence;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .expect("original scoped replay CREATE preserves engine sequence allocator");
    assert_eq!(rows.len(), 1);
    rows[0]
}

#[derive(SurrealValue)]
struct OriginalScopeRow {
    owner_account_id: Option<String>,
    actor_principal_id: Option<String>,
    authenticated_session_id: Option<String>,
    access_space_id: Option<String>,
    workspace_id: Option<String>,
}
#[derive(SurrealValue)]
struct OriginalScopeRead {
    record: RecordId,
}

pub async fn verify_original_seed_rows(
    storage: &SurrealStorage,
    origin: &ExactResourceScopeAttribution,
    session_id: Uuid,
    checkpoint_id: Uuid,
    process_uuid: Option<Uuid>,
) {
    validate_original_scope(origin);
    let scope = ReclaimResourceScope::from_exact(origin)
        .expect("original seed attribution is an exact reclaim scope");
    let mut records = vec![
        RecordId::new("kernel_session_queue", format!("SR-{session_id}")),
        RecordId::new("kernel_session_checkpoint", checkpoint_id.to_string()),
    ];
    if let Some(process_uuid) = process_uuid {
        records.push(RecordId::new(
            "kernel_process_lifecycle",
            process_uuid.to_string(),
        ));
    }
    for record in records {
        let rows = storage
            .with_data_operation(|database| {
                Box::pin(async move {
                    database.query_values::<OriginalScopeRow, _>(
                        "SELECT owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id FROM ONLY $record;",
                        OriginalScopeRead { record },
                    ).await
                })
            })
            .await
            .expect("canonical original scope reread");
        assert_eq!(
            rows.len(),
            1,
            "exact original CREATE row required before readiness/recovery"
        );
        assert_original_row(&rows[0], &scope);
    }
    #[derive(SurrealValue)]
    struct SessionRead {
        session_run_id: String,
    }
    let session_run_id = format!("SR-{session_id}");
    let rows = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                database.query_values::<OriginalScopeRow, _>(
                    "SELECT owner_account_id, actor_principal_id, authenticated_session_id, access_space_id, workspace_id FROM kernel_event_ledger WHERE session_run_id = $session_run_id ORDER BY event_sequence;",
                    SessionRead { session_run_id },
                ).await
            })
        })
        .await
        .expect("canonical original replay scope reread");
    assert!(!rows.is_empty(), "original replay row required");
    for row in rows {
        assert_original_row(&row, &scope);
    }
}

fn assert_original_row(row: &OriginalScopeRow, scope: &ReclaimResourceScope) {
    assert_eq!(
        row.owner_account_id.as_deref(),
        Some(scope.account_uuid.to_string().as_str())
    );
    assert_eq!(
        row.actor_principal_id.as_deref(),
        Some(scope.actor_uuid.to_string().as_str())
    );
    assert_eq!(
        row.authenticated_session_id.as_deref(),
        Some(scope.session_uuid.to_string().as_str())
    );
    assert_eq!(
        row.access_space_id.as_deref(),
        Some(scope.access_space_uuid.to_string().as_str())
    );
    assert_eq!(
        row.workspace_id.as_deref(),
        Some(scope.workspace_id.as_str())
    );
}

pub async fn seed_closed_recovery_store(data_dir: &Path) -> SeededRecoveryEvidence {
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(data_dir)
            .expect("configure real-binary recovery seed store")
            .with_test_datastore_sync_durable(),
    )
    .await
    .expect("open real-binary recovery seed store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap real-binary recovery seed store");
    let original_scope = mint_original_scope();
    let session_id = Uuid::now_v7();
    let session_run_id = format!("SR-{session_id}");
    let database = SurrealDatabase::new(storage.clone());
    let now = Utc::now();
    seed_original_queue(
        &storage,
        &original_scope,
        SessionRun {
            session_run_id: session_run_id.clone(),
            kernel_task_run_id: "KTR-MT195-REAL-BINARY".to_owned(),
            adapter_id: "mt195-real-binary-seed".to_owned(),
            state: SessionRunState::Queued,
            created_at: now,
            updated_at: now,
        },
    )
    .await;
    database
        .claim_kernel_session_run(&session_run_id, "mt195-real-binary-seed", 300)
        .await
        .expect("claim real-binary recovery candidate")
        .expect("real-binary recovery candidate is claimable");
    database
        .update_kernel_session_run_state(&session_run_id, SessionRunState::Running)
        .await
        .expect("mark real-binary recovery candidate running");
    let replay_event = NewKernelEvent::builder(
        "KTR-MT195-REAL-BINARY",
        &session_run_id,
        KernelEventType::ModelResponseRecorded,
        KernelActor::System("mt195-real-binary-seed".to_owned()),
    )
    .aggregate("session_run", &session_run_id)
    .idempotency_key(format!("mt195-real-binary-replay-{session_id}"))
    .source_component("mt195-real-binary-seed")
    .payload(serde_json::json!({"by": 3}))
    .build()
    .expect("build real-binary replay event");
    let replay_sequence = seed_original_replay_event(&storage, &original_scope, replay_event).await;
    assert_eq!(replay_sequence, 1);
    let checkpoint = SessionCheckpoint::new(
        session_id,
        Uuid::now_v7(),
        0,
        serde_json::json!({"counter": 0, "source": "mt195-real-binary-seed"}),
        CheckpointStateKind::Periodic,
    )
    .expect("construct real-binary recovery checkpoint");
    let checkpoint_id = checkpoint.checkpoint_id.as_uuid();
    seed_original_checkpoint(&storage, &original_scope, &checkpoint).await;
    verify_original_seed_rows(&storage, &original_scope, session_id, checkpoint_id, None).await;
    storage
        .shutdown()
        .await
        .expect("close real-binary recovery seed store");
    SeededRecoveryEvidence {
        original_scope,
        session_id,
        checkpoint_id,
    }
}

pub fn run_real_handshake_core_startup_recovery(
    root: &Path,
    data_dir: &Path,
    seed: &SeededRecoveryEvidence,
) -> ProductStartupRecoveryEvidence {
    let report_file = root.join("startup-recovery-report.json");
    let stdout = File::create(root.join("handshake-core-stdout.log"))
        .expect("create real handshake_core stdout");
    let stderr = File::create(root.join("handshake-core-stderr.log"))
        .expect("create real handshake_core stderr");
    validate_original_scope(&seed.original_scope);
    let handoff =
        serde_json::to_string(&serde_json::json!({"schema_version":1,"scope":seed.original_scope}))
            .expect("validated original product handoff");
    let mut command = Command::new(env!("CARGO_BIN_EXE_handshake_core"));
    command
        .env(PRODUCT_LOCAL_RESOURCE_SCOPE_ENV, handoff)
        .env("HANDSHAKE_DATA_DIR", data_dir)
        .env("HANDSHAKE_STARTUP_RECOVERY_ONLY", "1")
        .env("HANDSHAKE_STARTUP_RECOVERY_REPORT_FILE", &report_file)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    apply_quiet_process_flags(&mut command);
    let status = command
        .status()
        .expect("run real handshake_core startup recovery");
    ProductStartupRecoveryEvidence {
        status,
        report_file,
    }
}

async fn run_child_entrypoint_from_env() {
    if std::env::var(CHILD_MODE_ENV).ok().as_deref() != Some("1") {
        return;
    }
    let data_dir =
        PathBuf::from(std::env::var_os(CHILD_DATA_DIR_ENV).expect("MT-195 child data directory"));
    let ready_file =
        PathBuf::from(std::env::var_os(CHILD_READY_FILE_ENV).expect("MT-195 child ready file"));
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&data_dir)
            .expect("configure MT-195 child store")
            .with_test_datastore_sync_durable(),
    )
    .await
    .expect("open MT-195 child store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap MT-195 child store");

    let original_scope = mint_original_scope();
    let session_id = Uuid::now_v7();
    let session_run_id = format!("SR-{session_id}");
    let database = SurrealDatabase::new(storage.clone());
    let now = Utc::now();
    seed_original_queue(
        &storage,
        &original_scope,
        SessionRun {
            session_run_id: session_run_id.clone(),
            kernel_task_run_id: "KTR-MT195-HARD-KILL".to_owned(),
            adapter_id: "mt195-runtime-child".to_owned(),
            state: SessionRunState::Queued,
            created_at: now,
            updated_at: now,
        },
    )
    .await;
    database
        .claim_kernel_session_run(&session_run_id, "mt195-runtime-child", 300)
        .await
        .expect("claim MT-195 child session")
        .expect("queued MT-195 child session is claimable");
    database
        .update_kernel_session_run_state(&session_run_id, SessionRunState::Running)
        .await
        .expect("mark MT-195 child session running");
    let replay_event = NewKernelEvent::builder(
        "KTR-MT195-HARD-KILL",
        &session_run_id,
        KernelEventType::ModelResponseRecorded,
        KernelActor::System("mt195-runtime-child".to_owned()),
    )
    .aggregate("session_run", &session_run_id)
    .idempotency_key(format!("mt195-hard-kill-replay-{session_id}"))
    .source_component("mt195-runtime-child")
    .payload(serde_json::json!({"by": 3}))
    .build()
    .expect("build MT-195 replay event");
    let replay_sequence = seed_original_replay_event(&storage, &original_scope, replay_event).await;
    assert_eq!(replay_sequence, 1);
    let checkpoint = SessionCheckpoint::new(
        session_id,
        Uuid::now_v7(),
        0,
        serde_json::json!({"counter": 0, "source": "mt195-hard-kill-child"}),
        CheckpointStateKind::Periodic,
    )
    .expect("construct MT-195 child checkpoint");
    let checkpoint_id = checkpoint.checkpoint_id.as_uuid();
    seed_original_checkpoint(&storage, &original_scope, &checkpoint).await;

    let process = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "mt195-runtime-child",
        Some("WP-KERNEL-004".to_owned()),
    )
    .with_os_pid(std::process::id())
    .with_parent_session_id(&session_run_id)
    .with_mt_id("MT-195");
    let process_uuid = process.process_uuid;
    seed_original_process(&storage, &original_scope, &process).await;
    verify_original_seed_rows(
        &storage,
        &original_scope,
        session_id,
        checkpoint_id,
        Some(process_uuid),
    )
    .await;

    let ready = ChildReadyEvidence {
        original_scope,
        session_id,
        checkpoint_id,
        process_uuid,
        os_pid: std::process::id(),
    };
    let mut file = File::create(&ready_file).expect("create MT-195 readiness file");
    serde_json::to_writer(&mut file, &ready).expect("write MT-195 readiness JSON");
    file.write_all(b"\n").expect("finish readiness JSON");
    file.sync_all().expect("sync MT-195 readiness evidence");

    std::future::pending::<()>().await;
}

#[cfg(windows)]
fn apply_quiet_process_flags(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn apply_quiet_process_flags(_command: &mut Command) {}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "internal MT-195 child entrypoint; the parent test hard-kills this process"]
async fn mt195_runtime_child_entrypoint() {
    run_child_entrypoint_from_env().await;
}
