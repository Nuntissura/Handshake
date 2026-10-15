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

#[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
mod full_main_shutdown {
    use super::apply_quiet_process_flags;
    use handshake_core::flight_recorder::shutdown_proof::{
        ProofAction, ProofCommand, ProofObservation, ProofReceipt,
    };
    use std::{
        fs::{File, OpenOptions},
        io::{self, Read, Seek, SeekFrom, Write},
        path::Path,
        process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
        sync::{atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, SyncSender}, Arc},
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };
    use uuid::Uuid;

    const MAX_FRAME_BYTES: usize = 65_536;

    fn incomplete(reason: impl Into<String>) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, format!("MT188_INCOMPLETE: {}", reason.into()))
    }

    fn check_io_stop(stop: &AtomicBool, deadline: Instant) -> io::Result<()> {
        if stop.load(Ordering::Acquire) || Instant::now() >= deadline {
            return Err(incomplete("owned control I/O stopped or original deadline elapsed"));
        }
        Ok(())
    }

    fn read_control(input: &mut impl Read, bytes: &mut [u8], stop: &AtomicBool, deadline: Instant) -> io::Result<usize> {
        loop {
            check_io_stop(stop, deadline)?;
            match input.read(bytes) {
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {},
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(1)),
                result => return result,
            }
        }
    }

    fn read_control_exact(input: &mut impl Read, mut bytes: &mut [u8], stop: &AtomicBool, deadline: Instant) -> io::Result<()> {
        while !bytes.is_empty() {
            let count = read_control(input, bytes, stop, deadline)?;
            if count == 0 { return Err(incomplete("truncated owned control receipt")); }
            bytes = &mut bytes[count..];
        }
        Ok(())
    }

    fn write_control(output: &mut impl Write, mut bytes: &[u8], stop: &AtomicBool, deadline: Instant) -> io::Result<()> {
        while !bytes.is_empty() {
            check_io_stop(stop, deadline)?;
            match output.write(bytes) {
                Ok(0) => return Err(incomplete("owned control write made no progress")),
                Ok(count) => bytes = &bytes[count..],
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {},
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(1)),
                Err(error) => return Err(error),
            }
        }
        check_io_stop(stop, deadline)?;
        output.flush()
    }

    #[cfg(unix)]
    fn nonblocking_owned_pipe(pipe: &impl std::os::fd::AsRawFd) -> io::Result<()> {
        let fd = pipe.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    #[cfg(windows)]
    fn cancel_owned_thread(task: &JoinHandle<()>) -> io::Result<()> {
        use std::os::windows::thread::JoinHandleExt;
        use windows_sys::Win32::{Foundation::ERROR_NOT_FOUND, System::IO::CancelSynchronousIo};
        if task.is_finished() { return Ok(()); }
        if unsafe { CancelSynchronousIo(task.as_raw_handle()) } != 0 { return Ok(()); }
        let error = io::Error::last_os_error();
        // No pending syscall can mean completion, or the stop/cancel race before
        // the worker enters a syscall. The owner observes completion and cancels
        // again while retaining the same handle; this result is never join proof.
        if error.raw_os_error() == Some(ERROR_NOT_FOUND as i32) || task.is_finished() { return Ok(()); }
        Err(error)
    }

    #[derive(Clone, Copy)]
    enum Phase {
        Initial, Watching, Allocation, Selected, Pending, Armed, Released, Failed, Restored, Healthy,
        Shutdown,
    }

    #[derive(serde::Serialize)]
    pub struct FullMainChildIdentity {
        pid: u32,
        birth_source: &'static str,
        birth_value: u64,
        executable: std::path::PathBuf,
        executable_sha256: String,
    }

    fn image_digest(path: &Path) -> io::Result<String> {
        use sha2::{Digest, Sha256};
        let mut image = std::fs::File::open(path)?;
        let mut hash = Sha256::new();
        let mut bytes = [0u8; 65_536];
        loop {
            let count = image.read(&mut bytes)?;
            if count == 0 { break; }
            hash.update(&bytes[..count]);
        }
        Ok(format!("{:x}", hash.finalize()))
    }

    #[cfg(windows)]
    fn capture_identity(child: &Child) -> io::Result<FullMainChildIdentity> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{
            Foundation::FILETIME,
            System::Threading::{GetProcessTimes, QueryFullProcessImageNameW},
        };
        let empty = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
        let (mut birth, mut exit, mut kernel, mut user) = (empty, empty, empty, empty);
        if unsafe { GetProcessTimes(child.as_raw_handle(), &mut birth, &mut exit, &mut kernel, &mut user) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut image = vec![0u16; 32_768];
        let mut length = image.len() as u32;
        if unsafe { QueryFullProcessImageNameW(child.as_raw_handle(), 0, image.as_mut_ptr(), &mut length) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let reported = String::from_utf16(&image[..length as usize])
            .map_err(|_| incomplete("owned process image is not valid UTF16"))?;
        let executable = Path::new(&reported).canonicalize()?;
        if executable != Path::new(env!("CARGO_BIN_EXE_handshake_core")).canonicalize()? {
            return Err(incomplete("owned process image differs from the actual Cargo binary"));
        }
        let birth_value = (u64::from(birth.dwHighDateTime) << 32) | u64::from(birth.dwLowDateTime);
        if birth_value == 0 { return Err(incomplete("owned process kernel creation time is zero")); }
        Ok(FullMainChildIdentity { pid: child.id(), birth_source: "windows_get_process_times_filetime",
            birth_value, executable_sha256: image_digest(&executable)?, executable })
    }

    #[cfg(target_os = "linux")]
    fn capture_identity(child: &Child) -> io::Result<FullMainChildIdentity> {
        let process = std::path::PathBuf::from(format!("/proc/{}", child.id()));
        let stat = std::fs::read_to_string(process.join("stat"))?;
        let fields = stat.rsplit_once(')').ok_or_else(|| incomplete("invalid owned process stat"))?.1;
        let birth_value = fields.split_whitespace().nth(19)
            .ok_or_else(|| incomplete("owned process start ticks are missing"))?
            .parse::<u64>().map_err(|_| incomplete("owned process start ticks are invalid"))?;
        let executable = process.join("exe").canonicalize()?;
        if executable != Path::new(env!("CARGO_BIN_EXE_handshake_core")).canonicalize()? || birth_value == 0 {
            return Err(incomplete("owned process birth/image binding differs from the actual Cargo binary"));
        }
        Ok(FullMainChildIdentity { pid: child.id(), birth_source: "linux_proc_stat_start_ticks",
            birth_value, executable_sha256: image_digest(&executable)?, executable })
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    fn capture_identity(_child: &Child) -> io::Result<FullMainChildIdentity> {
        Err(io::Error::new(io::ErrorKind::Unsupported,
            "MT188_INCOMPLETE: this platform still requires an admitted kernel process birth/image identity reader"))
    }

    pub struct FullMainChild {
        child: Child,
        identity: Option<FullMainChildIdentity>,
        operation_id: Uuid,
        next_request: u64,
        phase: Phase,
        requests: Option<SyncSender<Vec<u8>>>,
        receipts: Option<Receiver<io::Result<ProofReceipt>>>,
        reader: Option<JoinHandle<()>>,
        writer: Option<JoinHandle<()>>,
        deadline: Instant,
        terminal: bool,
        stopping: Arc<AtomicBool>,
        failure_log: File,
        failure_readback: File,
        failure_sequence: u64,
        failure_errors: Vec<String>,
        observed_exit: Option<ExitStatus>,
        failure_snapshot: Option<String>,
        pending_stdin: Option<ChildStdin>,
        pending_stdout: Option<ChildStdout>,
        reader_state: &'static str,
        writer_state: &'static str,
    }

    impl FullMainChild {
        pub fn spawn(
            root: &Path,
            data_dir: &Path,
            listen_addr: &str,
            approval_config: &Path,
            deadline: Instant,
        ) -> io::Result<Self> {
            if !data_dir.is_absolute() || !approval_config.is_absolute() || deadline <= Instant::now() {
                return Err(incomplete("absolute genuine data/config paths and an unexpired deadline are required"));
            }
            let stderr = OpenOptions::new().append(true).create_new(true)
                .open(root.join("mt188-main-stderr.log"))?;
            let failure_log = stderr.try_clone()?;
            let failure_readback = File::open(root.join("mt188-main-stderr.log"))?;
            let mut command = Command::new(env!("CARGO_BIN_EXE_handshake_core"));
            command.env_remove("HANDSHAKE_STARTUP_RECOVERY_ONLY")
                .env_remove("HANDSHAKE_STARTUP_RECOVERY_REPORT_FILE")
                .env_remove(super::PRODUCT_LOCAL_RESOURCE_SCOPE_ENV)
                .env("HANDSHAKE_DATA_DIR", data_dir)
                .env("HANDSHAKE_BACKEND_LISTEN_ADDR", listen_addr)
                .env("HANDSHAKE_BACKEND_LISTEN_REPORT_FILE", root.join("mt188-main-listener.json"))
                .env("HANDSHAKE_CAPABILITY_REGISTRY_APPROVAL_CONFIG", approval_config)
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::from(stderr));
            apply_quiet_process_flags(&mut command);
            let mut child = command.spawn()?;
            let stdin = child.stdin.take().expect("owned piped child stdin");
            let stdout = child.stdout.take().expect("owned piped child stdout");
            let (request_tx, request_rx) = mpsc::sync_channel::<Vec<u8>>(1);
            let (receipt_tx, receipt_rx) = mpsc::sync_channel(1);
            let mut owned = Self {
                child, identity: None, operation_id: Uuid::now_v7(), next_request: 1, phase: Phase::Initial,
                requests: Some(request_tx), receipts: Some(receipt_rx), reader: None,
                writer: None, deadline, terminal: false, stopping: Arc::new(AtomicBool::new(false)),
                failure_log, failure_readback, failure_sequence: 0,
                failure_errors: Vec::new(), observed_exit: None, failure_snapshot: None,
                pending_stdin: Some(stdin), pending_stdout: Some(stdout),
                reader_state: "not_started", writer_state: "not_started",
            };
            let initialized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> io::Result<()> {
            owned.identity = Some(capture_identity(&owned.child)?);
            #[cfg(unix)]
            {
                nonblocking_owned_pipe(owned.pending_stdin.as_ref().expect("owned stdin"))?;
                nonblocking_owned_pipe(owned.pending_stdout.as_ref().expect("owned stdout"))?;
            }
            let writer_stop = Arc::clone(&owned.stopping);
            let mut stdin = owned.pending_stdin.take().expect("owned stdin");
            owned.writer = Some(thread::Builder::new().name("mt188-control-writer".into()).spawn(move || {
                while !writer_stop.load(Ordering::Acquire) {
                    let bytes = match request_rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(bytes) => bytes,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let result = write_control(&mut stdin, &(bytes.len() as u32).to_be_bytes(), &writer_stop, deadline)
                        .and_then(|_| write_control(&mut stdin, &bytes, &writer_stop, deadline));
                    if result.is_err() { break; }
                }
            })?);
            owned.writer_state = "live";
            let reader_stop = Arc::clone(&owned.stopping);
            let mut stdout = owned.pending_stdout.take().expect("owned stdout");
            owned.reader = Some(thread::Builder::new().name("mt188-receipt-reader".into()).spawn(move || loop {
                let result = (|| {
                    let mut size = [0u8; 4];
                    if read_control(&mut stdout, &mut size[..1], &reader_stop, deadline)? == 0 {
                        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "MT188_CONTROL_CLEAN_EOF"));
                    }
                    read_control_exact(&mut stdout, &mut size[1..], &reader_stop, deadline)?;
                    let size = u32::from_be_bytes(size) as usize;
                    if size == 0 || size > MAX_FRAME_BYTES { return Err(incomplete("invalid receipt frame length or stdout log pollution")); }
                    let mut bytes = vec![0; size];
                    read_control_exact(&mut stdout, &mut bytes, &reader_stop, deadline)?;
                    let text = std::str::from_utf8(&bytes).map_err(|_| incomplete("receipt is not UTF8"))?;
                    serde_json::from_str::<ProofReceipt>(text).map_err(|_| incomplete("invalid typed receipt JSON"))
                })();
                let failed = result.is_err();
                let mut pending = result;
                loop {
                    if reader_stop.load(Ordering::Acquire) { return; }
                    match receipt_tx.try_send(pending) {
                        Ok(()) => break,
                        Err(mpsc::TrySendError::Full(result)) => {
                            pending = result;
                            thread::sleep(Duration::from_millis(1));
                        },
                        Err(mpsc::TrySendError::Disconnected(_)) => return,
                    }
                }
                if failed { break; }
            })?);
            owned.reader_state = "live";
            Ok(())
            }));
            match initialized {
                Ok(Ok(())) => {},
                Ok(Err(error)) => {
                    use sha2::{Digest, Sha256};
                    let message = error.to_string();
                    let cause = format!(
                        "initialization_io kind={:?} raw_os={:?} message_bytes={} message_sha256={:x}",
                        error.kind(), error.raw_os_error(), message.len(), Sha256::digest(message.as_bytes()));
                    while !owned.observe_failure_settlement(&cause) {
                        thread::park_timeout(Duration::from_millis(10));
                    }
                    return Err(error);
                },
                Err(payload) => {
                    use sha2::{Digest, Sha256};
                    let cause = if let Some(message) = payload.downcast_ref::<String>() {
                        format!("initialization_panic type=String message_bytes={} message_sha256={:x}",
                            message.len(), Sha256::digest(message.as_bytes()))
                    } else if let Some(message) = payload.downcast_ref::<&str>() {
                        format!("initialization_panic type=str message_bytes={} message_sha256={:x}",
                            message.len(), Sha256::digest(message.as_bytes()))
                    } else {
                        "initialization_panic type=opaque message_unavailable=true".to_owned()
                    };
                    while !owned.observe_failure_settlement(&cause) {
                        thread::park_timeout(Duration::from_millis(10));
                    }
                    std::panic::resume_unwind(payload);
                },
            }
            Ok(owned)
        }

        pub fn identity(&self) -> io::Result<&FullMainChildIdentity> {
            self.identity.as_ref().ok_or_else(|| incomplete("actual kernel process identity is missing"))
        }

        pub fn await_listener(&mut self, root: &Path) -> io::Result<String> {
            let report = root.join("mt188-main-listener.json");
            loop {
                if let Ok(bytes) = std::fs::read(&report) {
                    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                        if value["schema_id"] != "handshake.backend-listen-report.v1"
                            || value["pid"].as_u64() != Some(u64::from(self.child.id())) {
                            return Err(incomplete("listener report does not belong to the owned main"));
                        }
                        let addr = value["listen_addr"].as_str().ok_or_else(|| incomplete("listener address missing"))?
                            .parse::<std::net::SocketAddr>().map_err(|_| incomplete("invalid listener address"))?;
                        if !addr.ip().is_loopback() || addr.port() == 0 { return Err(incomplete("listener is not an actual loopback address")); }
                        return Ok(format!("http://{addr}"));
                    }
                }
                if self.child.try_wait()?.is_some() || Instant::now() >= self.deadline {
                    return Err(incomplete("owned main exited or deadline elapsed before listener report"));
                }
                thread::sleep(Duration::from_millis(10));
            }
        }

        pub fn request(&mut self, action: ProofAction) -> io::Result<ProofReceipt> {
            let allowed = match &action {
                ProofAction::ObserveAllocation { .. } => matches!(self.phase, Phase::Initial),
                ProofAction::AwaitAllocation => matches!(self.phase, Phase::Watching),
                ProofAction::SelectAllocation { .. } => matches!(self.phase, Phase::Allocation),
                ProofAction::AwaitEvent => matches!(self.phase, Phase::Selected),
                ProofAction::Arm { .. } => matches!(self.phase, Phase::Pending),
                ProofAction::ReleaseEvent { .. } => matches!(self.phase, Phase::Pending | Phase::Armed),
                ProofAction::AwaitFailure => matches!(self.phase, Phase::Released),
                ProofAction::ConfirmRestored => matches!(self.phase, Phase::Failed),
                ProofAction::AwaitHealthy => matches!(self.phase, Phase::Released | Phase::Restored),
                ProofAction::Shutdown => matches!(self.phase, Phase::Healthy),
            };
            if !allowed { return Err(incomplete("out-of-phase parent command")); }
            let request_id = self.next_request;
            self.next_request = request_id.checked_add(1).ok_or_else(|| incomplete("request ID exhausted"))?;
            let command = ProofCommand {
                version: 1, request_id, operation_id: self.operation_id,
                causation_id: request_id.checked_sub(1).filter(|id| *id != 0), command: action,
            };
            let bytes = serde_json::to_vec(&command).map_err(|_| incomplete("command serialization"))?;
            if bytes.is_empty() || bytes.len() > MAX_FRAME_BYTES { return Err(incomplete("command frame exceeds limit")); }
            if self.child.try_wait()?.is_some() { return Err(incomplete("full main exited before requested observation")); }
            self.requests.as_ref().ok_or_else(|| incomplete("control closed"))?
                .try_send(bytes).map_err(|_| incomplete("control writer unavailable or backpressured"))?;
            let remaining = self.deadline.checked_duration_since(Instant::now())
                .ok_or_else(|| incomplete("proof deadline elapsed"))?;
            let receipt = self.receipts.as_ref().ok_or_else(|| incomplete("receipt reader closed"))?
                .recv_timeout(remaining).map_err(|_| incomplete("receipt EOF or deadline elapsed"))??;
            if receipt.version != 1 || receipt.request_id != request_id
                || receipt.operation_id != self.operation_id || receipt.causation_id != Some(request_id) {
                return Err(incomplete("duplicate or mismatched receipt correlation"));
            }
            self.phase = match (&command.command, &receipt.receipt) {
                (ProofAction::ObserveAllocation { .. }, ProofObservation::Watching) => Phase::Watching,
                (ProofAction::AwaitAllocation, ProofObservation::AllocationPending { .. }) => Phase::Allocation,
                (ProofAction::SelectAllocation { .. }, ProofObservation::Selected) => Phase::Selected,
                (ProofAction::AwaitEvent, ProofObservation::PendingEvent { .. }) => Phase::Pending,
                (ProofAction::Arm { .. }, ProofObservation::Armed) => Phase::Armed,
                (ProofAction::ReleaseEvent { .. }, ProofObservation::Released) => Phase::Released,
                (ProofAction::AwaitFailure, ProofObservation::StickyFailure { restore_error: None, .. }) => Phase::Failed,
                (ProofAction::ConfirmRestored, ProofObservation::Restored) => Phase::Restored,
                (ProofAction::AwaitHealthy, ProofObservation::HealthyEvent { .. }) => Phase::Healthy,
                (ProofAction::Shutdown, ProofObservation::ShutdownRequested) => Phase::Shutdown,
                _ => return Err(incomplete("unexpected phase, unsupported command, or failed local restoration")),
            };
            Ok(receipt)
        }

        pub fn wait_for_exit(&mut self) -> io::Result<ExitStatus> {
            if !matches!(self.phase, Phase::Shutdown) { return Err(incomplete("normal shutdown was not requested after genuine healthy recording")); }
            loop {
                if let Some(status) = self.child.try_wait()? {
                    self.terminal = true;
                    self.observed_exit = Some(status);
                    let remaining = self.deadline.checked_duration_since(Instant::now())
                        .ok_or_else(|| incomplete("deadline elapsed before control EOF"))?;
                    let trailing = self.receipts.as_ref().ok_or_else(|| incomplete("receipt stream disappeared"))?
                        .recv_timeout(remaining).map_err(|_| incomplete("receipt stream did not close with owned main"))?;
                    match trailing {
                        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof
                            && error.to_string() == "MT188_CONTROL_CLEAN_EOF" => {},
                        _ => return Err(incomplete("extra receipt, duplicate frame or stdout pollution after shutdown acknowledgment")),
                    }
                    self.join_io()?;
                    if status.code().is_none() { return Err(incomplete("main has no numeric exit status")); }
                    return Ok(status);
                }
                if Instant::now() >= self.deadline { return Err(incomplete("main did not exit before proof deadline")); }
                thread::sleep(Duration::from_millis(10));
            }
        }

        fn join_io(&mut self) -> io::Result<()> {
            self.stopping.store(true, Ordering::Release);
            let mut errors = Vec::new();
            while self.writer.as_ref().is_some_and(|task| !task.is_finished())
                || self.reader.as_ref().is_some_and(|task| !task.is_finished()) {
                if Instant::now() >= self.deadline {
                    return Err(incomplete("original deadline elapsed before owned I/O completion; closure pending"));
                }
                #[cfg(windows)]
                for (name, task) in [("writer", self.writer.as_ref()), ("reader", self.reader.as_ref())] {
                    if let Some(task) = task {
                        if let Err(error) = cancel_owned_thread(task) {
                            let observed = format!("owned {name} cancellation failed: {error}");
                            return Err(incomplete(observed));
                        }
                    }
                }
                // Expiry invalidates proof. It never authorizes detaching a
                // still-owned worker or treating a cancel return as completion.
                thread::park_timeout(Duration::from_millis(1));
            }
            if let Some(writer) = self.writer.take() {
                self.writer_state = if writer.join().is_err() {
                    errors.push("control writer panicked".to_owned()); "joined_panicked"
                } else { "joined" };
            }
            if let Some(reader) = self.reader.take() {
                self.reader_state = if reader.join().is_err() {
                    errors.push("receipt reader panicked".to_owned()); "joined_panicked"
                } else { "joined" };
            }
            if Instant::now() >= self.deadline {
                errors.push("original deadline elapsed before owned I/O completion".to_owned());
            }
            if errors.is_empty() { Ok(()) } else { Err(incomplete(errors.join("; "))) }
        }

        // The caller retains its complete case frame while this failure-only
        // observation returns false. Settlement never changes the failed verdict.
        pub fn observe_failure_settlement(&mut self, primary_error: &str) -> bool {
            self.stopping.store(true, Ordering::Release);
            // These exist only before a worker takes ownership. Closing an
            // unassigned pipe releases EOF without abandoning any worker.
            self.pending_stdin.take();
            self.pending_stdout.take();
            if !self.terminal {
                match self.child.try_wait() {
                    Ok(Some(status)) => {
                        self.terminal = true;
                        self.observed_exit = Some(status);
                    },
                    Ok(None) => {},
                    Err(error) => self.retain_cleanup_error(format!("owned child poll failed: {error}")),
                }
            }
            #[cfg(windows)]
            {
                let mut errors = Vec::new();
                for (name, task) in [("writer", self.writer.as_ref()), ("reader", self.reader.as_ref())] {
                    if let Some(task) = task {
                        if let Err(error) = cancel_owned_thread(task) {
                            errors.push(format!("owned {name} cancellation failed: {error}"));
                        }
                    }
                }
                for error in errors { self.retain_cleanup_error(error); }
            }
            if self.writer.as_ref().is_some_and(JoinHandle::is_finished) {
                if self.writer.take().expect("observed owned writer").join().is_err() {
                    self.writer_state = "joined_panicked";
                    self.retain_cleanup_error("owned control writer panicked".to_owned());
                } else { self.writer_state = "joined"; }
            }
            if self.reader.as_ref().is_some_and(JoinHandle::is_finished) {
                if self.reader.take().expect("observed owned reader").join().is_err() {
                    self.reader_state = "joined_panicked";
                    self.retain_cleanup_error("owned receipt reader panicked".to_owned());
                } else { self.reader_state = "joined"; }
            }
            let settled = self.terminal && self.reader.is_none() && self.writer.is_none();
            let snapshot = serde_json::json!({
                "schema_id": "handshake.mt188.failed-owned-closure.v1",
                "status": "FAILED_INCOMPLETE",
                "operation_id": self.operation_id,
                "identity": self.identity,
                "owned_pid": self.child.id(),
                "original_deadline_elapsed": Instant::now() >= self.deadline,
                "observed_numeric_exit": self.observed_exit.and_then(|status| status.code()),
                "child_terminal_observed": self.terminal,
                "reader_state": self.reader_state,
                "writer_state": self.writer_state,
                "closure_pending": !settled,
                "primary_error": primary_error,
                "cleanup_errors": self.failure_errors,
            }).to_string();
            if self.failure_snapshot.as_ref() != Some(&snapshot) {
                let result = self.persist_failure_snapshot(&snapshot);
                match result {
                    Ok(()) => self.failure_snapshot = Some(snapshot),
                    Err(error) => {
                        self.retain_cleanup_error(format!("failure diagnostic durability unproven: {error}"));
                        return false;
                    },
                }
            }
            settled
        }

        fn retain_cleanup_error(&mut self, error: String) {
            if !self.failure_errors.contains(&error) { self.failure_errors.push(error); }
        }

        fn persist_failure_snapshot(&mut self, snapshot: &str) -> io::Result<()> {
            use sha2::{Digest, Sha256};
            const READBACK_LIMIT: usize = 262_144;
            self.failure_sequence = self.failure_sequence.checked_add(1)
                .ok_or_else(|| incomplete("failure record sequence exhausted"))?;
            let digest = format!("{:x}", Sha256::digest(snapshot.as_bytes()));
            let frame = format!("\nMT188_FAILURE {} {} {}\n{}\nMT188_FAILURE_END {} {}\n",
                self.failure_sequence, snapshot.len(), digest, snapshot, self.failure_sequence, digest);
            if frame.len() > MAX_FRAME_BYTES {
                return Err(incomplete("failure diagnostic exceeds bounded frame size"));
            }
            // Child stderr may interleave: neither append nor sync is record
            // atomicity. Accept only this complete sequence/checksum frame read
            // through a separate handle to the same existing log.
            self.failure_log.write_all(frame.as_bytes())?;
            self.failure_log.flush()?;
            self.failure_log.sync_all()?;
            let end = self.failure_readback.metadata()?.len();
            let start = end.saturating_sub(READBACK_LIMIT as u64);
            self.failure_readback.seek(SeekFrom::Start(start))?;
            let mut tail = Vec::new();
            (&mut self.failure_readback).take(end - start).read_to_end(&mut tail)?;
            if !tail.windows(frame.len()).any(|bytes| bytes == frame.as_bytes()) {
                return Err(incomplete("failure record readback unproven: interleaved, truncated, or outside bounded tail"));
            }
            Ok(())
        }
    }
    impl Drop for FullMainChild {
        fn drop(&mut self) {
            if self.terminal && self.reader.is_none() && self.writer.is_none() { return; }
            while !self.observe_failure_settlement("fallback_drop actual_primary_cause_unavailable=true") {
                thread::park_timeout(Duration::from_millis(10));
            }
        }
    }
}

#[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
pub use full_main_shutdown::FullMainChild;
