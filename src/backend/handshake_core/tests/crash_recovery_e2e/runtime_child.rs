use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use chrono::Utc;
use handshake_core::{
    kernel::{KernelActor, KernelEventType, NewKernelEvent, SessionRun, SessionRunState},
    process_ledger::{
        LedgerEvent, ProcessEngineKind, ProcessLedgerStore, ProcessStart, SurrealProcessLedgerStore,
    },
    session_checkpoint::{
        CheckpointSink, CheckpointStateKind, SessionCheckpoint, SurrealCheckpointSink,
    },
    storage::{
        surreal::{bootstrap_schema, SurrealDatabase, SurrealStorage, SurrealStorageConfig},
        Database,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const CHILD_MODE_ENV: &str = "HS_MT195_SURREAL_CHILD";
const CHILD_DATA_DIR_ENV: &str = "HS_MT195_SURREAL_DATA_DIR";
const CHILD_READY_FILE_ENV: &str = "HS_MT195_SURREAL_READY_FILE";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildReadyEvidence {
    pub session_id: Uuid,
    pub checkpoint_id: Uuid,
    pub process_uuid: Uuid,
    pub os_pid: u32,
}

pub struct KilledChildEvidence {
    pub data_dir: PathBuf,
    pub ready: ChildReadyEvidence,
    pub exit_status: std::process::ExitStatus,
}

pub struct SeededRecoveryEvidence {
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

pub async fn seed_closed_recovery_store(data_dir: &Path) -> SeededRecoveryEvidence {
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(data_dir)
            .expect("configure real-binary recovery seed store"),
    )
    .await
    .expect("open real-binary recovery seed store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap real-binary recovery seed store");
    let session_id = Uuid::now_v7();
    let session_run_id = format!("SR-{session_id}");
    let database = SurrealDatabase::new(storage.clone());
    let now = Utc::now();
    database
        .enqueue_kernel_session_run(SessionRun {
            session_run_id: session_run_id.clone(),
            kernel_task_run_id: "KTR-MT195-REAL-BINARY".to_owned(),
            adapter_id: "mt195-real-binary-seed".to_owned(),
            state: SessionRunState::Queued,
            created_at: now,
            updated_at: now,
        })
        .await
        .expect("enqueue real-binary recovery candidate");
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
    let replay_event = database
        .append_kernel_event(replay_event)
        .await
        .expect("persist real-binary replay event");
    assert_eq!(replay_event.event_sequence, 1);
    let checkpoint = SessionCheckpoint::new(
        session_id,
        Uuid::now_v7(),
        0,
        serde_json::json!({"counter": 0, "source": "mt195-real-binary-seed"}),
        CheckpointStateKind::Periodic,
    )
    .expect("construct real-binary recovery checkpoint");
    let checkpoint_id = checkpoint.checkpoint_id.as_uuid();
    SurrealCheckpointSink::new(storage.clone())
        .write_batch(vec![checkpoint])
        .await
        .expect("persist real-binary recovery checkpoint");
    storage
        .shutdown()
        .await
        .expect("close real-binary recovery seed store");
    SeededRecoveryEvidence {
        session_id,
        checkpoint_id,
    }
}

pub fn run_real_handshake_core_startup_recovery(
    root: &Path,
    data_dir: &Path,
) -> ProductStartupRecoveryEvidence {
    let report_file = root.join("startup-recovery-report.json");
    let stdout = File::create(root.join("handshake-core-stdout.log"))
        .expect("create real handshake_core stdout");
    let stderr = File::create(root.join("handshake-core-stderr.log"))
        .expect("create real handshake_core stderr");
    let mut command = Command::new(env!("CARGO_BIN_EXE_handshake_core"));
    command
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
        SurrealStorageConfig::for_data_dir(&data_dir).expect("configure MT-195 child store"),
    )
    .await
    .expect("open MT-195 child store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap MT-195 child store");

    let session_id = Uuid::now_v7();
    let session_run_id = format!("SR-{session_id}");
    let database = SurrealDatabase::new(storage.clone());
    let now = Utc::now();
    database
        .enqueue_kernel_session_run(SessionRun {
            session_run_id: session_run_id.clone(),
            kernel_task_run_id: "KTR-MT195-HARD-KILL".to_owned(),
            adapter_id: "mt195-runtime-child".to_owned(),
            state: SessionRunState::Queued,
            created_at: now,
            updated_at: now,
        })
        .await
        .expect("enqueue MT-195 child session");
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
    let replay_event = database
        .append_kernel_event(replay_event)
        .await
        .expect("persist MT-195 replay event");
    assert_eq!(replay_event.event_sequence, 1);
    let checkpoint = SessionCheckpoint::new(
        session_id,
        Uuid::now_v7(),
        0,
        serde_json::json!({"counter": 0, "source": "mt195-hard-kill-child"}),
        CheckpointStateKind::Periodic,
    )
    .expect("construct MT-195 child checkpoint");
    let checkpoint_id = checkpoint.checkpoint_id.as_uuid();
    SurrealCheckpointSink::new(storage.clone())
        .write_batch(vec![checkpoint])
        .await
        .expect("persist MT-195 child checkpoint");

    let process = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "mt195-runtime-child",
        Some("WP-KERNEL-004".to_owned()),
    )
    .with_os_pid(std::process::id())
    .with_parent_session_id(&session_run_id)
    .with_mt_id("MT-195");
    let process_uuid = process.process_uuid;
    SurrealProcessLedgerStore::new(storage.clone())
        .write_batch(vec![LedgerEvent::Start(process)])
        .await
        .expect("persist MT-195 active process row");

    let ready = ChildReadyEvidence {
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
