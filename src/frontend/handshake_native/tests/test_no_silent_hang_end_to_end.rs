//! MT-106 capstone: no silent hangs across in-app operations and real child processes.
//!
//! Ignored by default because it launches the real `palmistry` binary and a real helper child. The
//! governed lane builds Palmistry and runs this test explicitly with `-- --include-ignored`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use handshake_diag_ring::{DiagEventCode, DiagRingWriter, DEFAULT_CAPACITY};
use handshake_native::diagnostics::{
    self, control_socket_name, launch_palmistry_at, DiagSession, OperationCode, ShutdownOutcome,
    ENV_PALMISTRY_EXE,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

fn unique_session_id(label: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{label}-{}-{nanos}", std::process::id())
}

fn find_palmistry_binary() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os(ENV_PALMISTRY_EXE) {
        let p = PathBuf::from(raw);
        if p.is_file() {
            return Some(p);
        }
    }
    let bin = if cfg!(windows) {
        "palmistry.exe"
    } else {
        "palmistry"
    };
    for base in [
        "../../../../Handshake_Artifacts/palmistry-target/debug",
        "../../../../Handshake_Artifacts/palmistry-target/release",
        "../../../../Handshake_Artifacts/target/debug",
        "../../../../Handshake_Artifacts/target/release",
    ] {
        let p = Path::new(base).join(bin);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn wait_until<F>(deadline: Duration, mut predicate: F) -> bool
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    while start.elapsed() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    predicate()
}

fn spawn_child_stall_helper(exe: &Path, liveness_path: &Path) -> std::io::Result<Child> {
    let mut command = Command::new(exe);
    command
        .arg("--child-stall-helper")
        .arg(liveness_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.spawn()
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    None
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child }
    }

    fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = wait_for_exit(&mut self.child, Duration::from_secs(2));
    }
}

struct EnvGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set_path(key: &'static str, value: &Path) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

struct DirGuard(PathBuf);

impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
#[ignore = "LIVE MT-106 capstone: builds/runs real Palmistry plus a real child process. Run with \
            `cargo test --test test_no_silent_hang_end_to_end -- --include-ignored --nocapture` after \
            building palmistry or setting HANDSHAKE_PALMISTRY_EXE."]
fn in_app_stall_and_real_child_stall_are_both_reported_in_bounded_time() {
    let exe = find_palmistry_binary().unwrap_or_else(|| {
        panic!(
            "MT-106 capstone needs a built palmistry binary. Build palmistry or set \
             {ENV_PALMISTRY_EXE}, then rerun with -- --include-ignored."
        )
    });

    let dir = external_artifact_dir("wp-kernel-012-mt-106");
    std::fs::create_dir_all(&dir).expect("create external artifact dir");
    let session_id = unique_session_id("mt106");
    let survivor_dir = dir.join(format!("survivors-{session_id}"));
    std::fs::create_dir_all(&survivor_dir).expect("create scoped survivor dir");
    let _survivor_dir_guard = DirGuard(survivor_dir.clone());
    let _survivor_env = EnvGuard::set_path(diagnostics::ENV_PALMISTRY_SURVIVOR_DIR, &survivor_dir);
    let ring_path = dir.join(format!("ring-{session_id}.ring"));
    let writer = DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY).expect("create diag ring");
    assert!(
        diagnostics::install(writer),
        "diagnostics recorder must install the MT-106 ring before the capstone records events"
    );

    let session = DiagSession {
        session_id: session_id.clone(),
        ring_path: ring_path.clone(),
    };
    let control_socket = control_socket_name(&session_id);
    let mut palmistry = launch_palmistry_at(&exe, &session, &ring_path, &control_socket)
        .expect("launch real palmistry");
    assert!(
        palmistry.handshake_acked(),
        "Palmistry handshake must ack before registering child stall watch; degrade reason: {:?}",
        palmistry.handshake_error()
    );

    let liveness_path = dir.join(format!("child-{session_id}.progress"));
    let child = spawn_child_stall_helper(&exe, &liveness_path).expect("spawn child-stall helper");
    let child = ChildGuard::new(child);
    assert!(
        wait_until(Duration::from_secs(3), || std::fs::read_to_string(
            &liveness_path
        )
        .map(|s| s.trim() == "1")
        .unwrap_or(false)),
        "child helper must publish its initial progress baseline before registration"
    );

    let child_session_id = 1_060_001;
    palmistry
        .register_child_liveness_file(
            child.id(),
            child_session_id,
            &liveness_path,
            Duration::from_millis(700),
        )
        .expect("register child liveness with Palmistry over held control socket");

    diagnostics::start_global_operation_watchdog();
    let operation = diagnostics::global_operation_watchdog().register(
        OperationCode::ChildProcess,
        Duration::from_millis(250),
        None,
    );

    let mut last_stalled_operation = false;
    let mut last_child_stall = false;
    let mut last_records = Vec::new();
    let observed = wait_until(Duration::from_secs(8), || {
        last_stalled_operation = diagnostics::snapshot_last_n(64).iter().any(|event| {
            event.event_code == DiagEventCode::StalledOperation.as_u16()
                && event.sequence_id == operation.operation_id()
                && event.counter_a == OperationCode::ChildProcess as u64
        });
        last_records = diagnostics::read_default_survivor_records();
        last_child_stall = last_records.iter().any(|record| {
            record.kind == diagnostics::PalmistrySurvivorKind::ChildStall
                && record.session_id == session_id
                && record.child_process_id == Some(child.id())
                && record.child_session_id == Some(child_session_id)
                && record.stale_ms >= 700
                && record.last_progress_counter == Some(1)
                && record.child_stall_reason_code == Some(1)
        });
        last_stalled_operation && last_child_stall
    });
    if !observed {
        let survivor_entries = std::fs::read_dir(&survivor_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path().display().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        panic!(
            "MT-106 capstone: global in-app operation stall and real alive child with stale passive \
             progress must both surface; stalled_operation={last_stalled_operation}, \
             child_stall={last_child_stall}, survivor_dir={}, survivor_entries={survivor_entries:?}, \
             decoded_records={last_records:?}",
            survivor_dir.display()
        );
    }
    operation.complete();

    let _ = palmistry.deregister_child(child.id(), child_session_id);
    drop(child);
    let outcome = palmistry.request_shutdown_and_wait(Duration::from_secs(10));
    match outcome {
        ShutdownOutcome::ExitedCleanly(status) => {
            assert!(
                status.success(),
                "Palmistry should exit cleanly: {status:?}"
            );
        }
        other => panic!("Palmistry should shutdown cleanly after MT-106 capstone, got {other:?}"),
    }

    let _ = std::fs::remove_file(&liveness_path);
    let _ = std::fs::remove_file(&ring_path);
}
