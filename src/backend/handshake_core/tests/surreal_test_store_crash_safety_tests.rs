//! MT-123 proof for isolated embedded Surreal test-store ownership and reclamation.

mod surreal_test_store_support;

use std::io::Write;
use std::process::Stdio;
use std::time::{Duration, Instant};

use handshake_core::storage::Workspace;
use surreal_test_store_support::{
    measure_owned_scopes, remaining_leak_modes, sweep_stale_orphans, IsolatedSurrealTestStore,
    TEST_STORE_ROOT_ENV, TEST_STORE_STALE_AGE_MS_ENV,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

const CHILD_MODE_ENV: &str = "HANDSHAKE_SURREAL_TEST_STORE_CHILD_MODE";
const CHILD_MODE_HOLD: &str = "hold";
const CHILD_MODE_DROP_GUARD: &str = "drop-guard";
const CHILD_MODE_PANIC_GUARD: &str = "panic-guard";
const CHILD_SCOPE_PREFIX: &str = "SURREAL_TEST_STORE_SCOPE=";
const CHILD_BACKLOG_STALE_AGE_MS: &str = "3600000";
const CHILD_WORKSPACE_PROBE_NAME: &str = "killed-child-populated-workspace";
const LIVE_WORKSPACE_PROBE_NAME: &str = "live-owner-populated-workspace";

fn assert_workspace_fields_match(expected: &Workspace, actual: &Workspace) {
    assert_eq!(actual.id, expected.id);
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.created_at, expected.created_at);
    assert_eq!(actual.updated_at, expected.updated_at);
}

#[tokio::test]
async fn graceful_shutdown_removes_the_owned_scope() {
    let root = tempfile::tempdir().expect("create isolated test root");
    let store = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("open real isolated embedded store");
    assert!(store.is_accepting_operations());
    let scope = store.scope_path().to_path_buf();

    store
        .shutdown_and_cleanup()
        .await
        .expect("gracefully close and remove owned scope");

    assert!(!scope.exists(), "graceful shutdown must remove its scope");
}

#[tokio::test]
#[cfg(windows)]
async fn parallel_sweep_skips_a_live_owner() {
    let root = tempfile::tempdir().expect("create isolated test root");
    let store = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("open real isolated embedded store");
    let scope = store.scope_path().to_path_buf();

    let report =
        sweep_stale_orphans(root.path(), Duration::ZERO).expect("sweep while owner is live");
    assert_eq!(report.skipped_live, vec![scope.clone()]);
    assert!(report.reclaimed.is_empty());
    assert!(
        scope.exists(),
        "a live parallel owner must survive the sweep"
    );

    store
        .shutdown_and_cleanup()
        .await
        .expect("clean up live-owner proof store");
}

#[tokio::test]
#[ignore = "MT-123 subprocess helper; launched and killed by the parent proof"]
async fn mt123_surreal_test_store_child_holds_owner_marker() {
    let Ok(mode) = std::env::var(CHILD_MODE_ENV) else {
        return;
    };
    let store = IsolatedSurrealTestStore::create()
        .await
        .expect("child opens real isolated embedded store");
    let scope = store.scope_path().to_path_buf();
    let marker = store.owner_marker_path_for_proof().to_path_buf();
    match mode.as_str() {
        CHILD_MODE_HOLD => {
            let killed_probe = store
                .create_workspace_probe(CHILD_WORKSPACE_PROBE_NAME)
                .await
                .expect("populate killed child's real Surreal workspace record");
            let killed_readback = store
                .get_workspace_probe(&killed_probe.id)
                .await
                .expect("read back killed child's Surreal workspace record")
                .expect("killed child's generated workspace id remains readable");
            assert_workspace_fields_match(&killed_probe, &killed_readback);
            std::mem::forget(store);
        }
        CHILD_MODE_DROP_GUARD => drop(store),
        CHILD_MODE_PANIC_GUARD => {
            let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _store_dropped_during_unwind = store;
                panic!("MT-123 deliberate owner-drop unwind proof");
            }));
            assert!(unwind.is_err(), "deliberate proof panic must be caught");
        }
        _ => panic!("unknown MT-123 subprocess mode: {mode}"),
    }
    if mode != CHILD_MODE_HOLD {
        let report = sweep_stale_orphans(
            std::path::PathBuf::from(
                std::env::var_os(TEST_STORE_ROOT_ENV).expect("child test-store root is configured"),
            ),
            Duration::ZERO,
        )
        .expect("sweep immediately after plain drop or caught unwind");
        assert_eq!(report.skipped_live, vec![scope.clone()]);
        assert!(report.reclaimed.is_empty());
        assert!(report.reclaimed_owner_markers.is_empty());
        assert!(scope.exists());
        assert!(marker.exists());
    }
    println!("{CHILD_SCOPE_PREFIX}{}", scope.display());
    std::io::stdout().flush().expect("flush child scope marker");
    std::future::pending::<()>().await;
}

#[tokio::test]
#[cfg(windows)]
async fn killed_child_orphan_is_reclaimed_without_touching_live_owners() {
    let root = tempfile::tempdir().expect("create isolated test root");
    let live_store = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("open parallel live embedded store");
    let live_scope = live_store.scope_path().to_path_buf();
    let live_probe = live_store
        .create_workspace_probe(LIVE_WORKSPACE_PROBE_NAME)
        .await
        .expect("populate live owner's real Surreal workspace record");

    let orphan_scope = spawn_and_kill_owned_store(root.path(), CHILD_MODE_HOLD).await;
    assert!(
        orphan_scope.exists(),
        "killed child must reproduce an orphan before reclamation"
    );

    let recovered_store = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("later normal store creation runs automatic orphan recovery");
    let report = recovered_store.startup_sweep_report();
    assert_eq!(report.reclaimed, vec![orphan_scope.clone()]);
    assert_eq!(report.skipped_live, vec![live_scope.clone()]);
    assert!(
        report.errors.is_empty(),
        "sweep errors: {:?}",
        report.errors
    );
    assert!(
        report.rejected_unsafe.is_empty(),
        "unsafe candidates: {:?}",
        report.rejected_unsafe
    );
    assert!(
        !orphan_scope.exists(),
        "killed-child orphan was not reclaimed"
    );
    assert!(live_scope.exists(), "parallel live owner was touched");
    let live_readback = live_store
        .get_workspace_probe(&live_probe.id)
        .await
        .expect("re-read live owner's workspace after sweep")
        .expect("live owner's generated workspace id remains readable");
    assert_workspace_fields_match(&live_probe, &live_readback);

    recovered_store
        .shutdown_and_cleanup()
        .await
        .expect("clean up recovering store");
    live_store
        .shutdown_and_cleanup()
        .await
        .expect("clean up parallel live store");
}

async fn spawn_and_kill_owned_store(
    root: &std::path::Path,
    child_mode: &str,
) -> std::path::PathBuf {
    let executable = std::env::current_exe().expect("resolve current MT-123 test executable");
    let mut command = Command::new(executable);
    command
        .arg("--exact")
        .arg("mt123_surreal_test_store_child_holds_owner_marker")
        .arg("--ignored")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_MODE_ENV, child_mode)
        .env(TEST_STORE_ROOT_ENV, root)
        .env(TEST_STORE_STALE_AGE_MS_ENV, CHILD_BACKLOG_STALE_AGE_MS)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = command.spawn().expect("spawn MT-123 store-owner child");
    let stdout = child.stdout.take().expect("capture child stdout");
    let mut lines = BufReader::new(stdout).lines();

    let orphan_scope = match tokio::time::timeout(Duration::from_secs(300), async {
        while let Some(line) = lines.next_line().await.expect("read child output") {
            if let Some(index) = line.find(CHILD_SCOPE_PREFIX) {
                return std::path::PathBuf::from(line[(index + CHILD_SCOPE_PREFIX.len())..].trim());
            }
        }
        panic!("child exited before publishing its isolated scope");
    })
    .await
    {
        Ok(scope) => scope,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            panic!("child did not publish its isolated scope within 300 seconds");
        }
    };
    assert!(orphan_scope.exists());

    child
        .kill()
        .await
        .expect("kill only the child process started by this proof");
    child.wait().await.expect("reap killed child process");
    orphan_scope
}

#[tokio::test]
#[cfg(windows)]
async fn plain_drop_and_caught_unwind_retain_ownership_until_process_exit() {
    for child_mode in [CHILD_MODE_DROP_GUARD, CHILD_MODE_PANIC_GUARD] {
        let root = tempfile::tempdir().expect("create drop-guard subprocess root");
        let scope = spawn_and_kill_owned_store(root.path(), child_mode).await;
        let report = sweep_stale_orphans(root.path(), Duration::ZERO)
            .expect("reclaim drop-guard scope after child process exits");
        assert_eq!(report.reclaimed, vec![scope.clone()]);
        assert!(!scope.exists());
    }
}

#[tokio::test]
#[cfg(windows)]
async fn interrupted_quarantine_is_recovered_by_the_next_normal_creation() {
    let root = tempfile::tempdir().expect("create quarantine-recovery root");
    let quarantine = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("open real embedded store")
        .leave_interrupted_quarantine_for_proof()
        .await
        .expect("simulate interruption after atomic quarantine rename");
    assert!(quarantine.exists());

    let recovered = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("normal creation recovers interrupted quarantine");
    assert_eq!(
        recovered.startup_sweep_report().reclaimed,
        vec![quarantine.clone()]
    );
    assert!(!quarantine.exists());
    recovered
        .shutdown_and_cleanup()
        .await
        .expect("clean up quarantine recovery store");
}

#[tokio::test]
#[cfg(windows)]
async fn live_quarantine_keeps_its_marker_until_owner_release() {
    let root = tempfile::tempdir().expect("create live-quarantine root");
    let mut store = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("open real embedded store");
    let quarantine = store
        .quarantine_while_owner_is_held_for_proof()
        .await
        .expect("move owned scope to recognized quarantine");
    let marker = store.owner_marker_path_for_proof().to_path_buf();

    let live_report = sweep_stale_orphans(root.path(), Duration::ZERO)
        .expect("sweep while quarantine owner remains held");
    assert_eq!(live_report.skipped_live, vec![quarantine.clone()]);
    assert!(live_report.reclaimed.is_empty());
    assert!(live_report.reclaimed_owner_markers.is_empty());
    assert!(quarantine.exists());
    assert!(
        marker.exists(),
        "live quarantine marker must remain present"
    );

    let released_quarantine = store.release_quarantine_owner_for_proof();
    assert_eq!(released_quarantine, quarantine);
    let stale_report = sweep_stale_orphans(root.path(), Duration::ZERO)
        .expect("sweep after quarantine owner release");
    assert_eq!(stale_report.reclaimed, vec![quarantine.clone()]);
    assert!(!quarantine.exists());
    assert!(
        !marker.exists(),
        "released quarantine marker must be reclaimed"
    );
}

#[test]
#[cfg(windows)]
fn reparse_candidate_is_rejected_and_external_content_survives() {
    let root = tempfile::tempdir().expect("create isolated test root");
    let outside = tempfile::tempdir().expect("create external sentinel root");
    let sentinel = outside.path().join("must-survive.txt");
    std::fs::write(&sentinel, b"untouched").expect("write external sentinel");
    let runtime = tokio::runtime::Runtime::new().expect("create proof runtime");
    let scope = runtime.block_on(async {
        IsolatedSurrealTestStore::create_in(root.path())
            .await
            .expect("open real embedded store for nested reparse proof")
            .leave_closed_orphan_for_proof()
            .await
            .expect("leave valid closed orphan for nested reparse proof")
    });
    let candidate = scope.join("nested-escape");
    create_directory_reparse(outside.path(), &candidate);

    let report =
        sweep_stale_orphans(root.path(), Duration::ZERO).expect("sweep with reparse candidate");

    assert!(report.reclaimed.is_empty());
    assert_eq!(report.rejected_unsafe.len(), 1);
    assert!(candidate.exists());
    assert_eq!(
        std::fs::read(&sentinel).expect("read external sentinel"),
        b"untouched"
    );

    std::fs::remove_dir(&candidate).expect("remove proof reparse without following it");
    let cleanup = sweep_stale_orphans(root.path(), Duration::ZERO)
        .expect("reclaim valid scope after removing reparse boundary");
    assert_eq!(cleanup.reclaimed, vec![scope]);
}

#[cfg(windows)]
fn create_directory_reparse(target: &std::path::Path, candidate: &std::path::Path) {
    use std::os::windows::process::CommandExt;

    let symlink_error = match std::os::windows::fs::symlink_dir(target, candidate) {
        Ok(()) => return,
        Err(error) => error,
    };

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let output = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(candidate)
        .arg(target)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .expect("run junction fallback after directory-symlink creation was unavailable");
    assert!(
        output.status.success(),
        "ENVIRONMENT_BLOCKED: neither a directory symlink nor junction could be created; symlink_error={} stdout={} stderr={}",
        symlink_error,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
#[cfg(windows)]
async fn bounded_backlog_recovery_records_counts_bytes_and_open_timings() {
    const BACKLOG_SIZE: usize = 3;
    const OPEN_BOUND: Duration = Duration::from_secs(120);

    let root = tempfile::tempdir().expect("create isolated backlog root");
    let mut first_cold_open = None;
    for _ in 0..BACKLOG_SIZE {
        let started = Instant::now();
        let orphan = spawn_and_kill_owned_store(root.path(), CHILD_MODE_HOLD).await;
        first_cold_open.get_or_insert(started.elapsed());
        assert!(
            orphan.exists(),
            "killed backlog member must remain orphaned"
        );
    }

    let before = measure_owned_scopes(root.path()).expect("measure backlog before recovery");
    assert_eq!(before.scope_count, BACKLOG_SIZE);
    assert!(
        before.contained_data_bytes > 0,
        "real embedded stores must contribute contained data bytes"
    );

    let recovery_started = Instant::now();
    let recovered = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("normal creation reclaims bounded backlog then cold-opens");
    let recovery_open_elapsed = recovery_started.elapsed();
    let after = measure_owned_scopes(root.path()).expect("measure backlog after recovery");
    assert_eq!(
        recovered.startup_sweep_report().reclaimed.len(),
        BACKLOG_SIZE
    );
    assert_eq!(
        recovered.startup_sweep_report().reclaimed_bytes,
        before.contained_data_bytes
    );
    assert_eq!(after.scope_count, 1, "only the new live scope may remain");
    assert!(
        after.contained_data_bytes < before.contained_data_bytes,
        "recovery must reduce the measured owned backlog bytes"
    );
    assert!(first_cold_open.expect("record first cold open") < OPEN_BOUND);
    assert!(recovery_open_elapsed < OPEN_BOUND);

    recovered
        .shutdown_and_cleanup()
        .await
        .expect("clean up recovered store");
    let reopen_started = Instant::now();
    let reopened = IsolatedSurrealTestStore::create_in(root.path())
        .await
        .expect("reopen after bounded recovery cleanup");
    let reopen_elapsed = reopen_started.elapsed();
    assert!(reopen_elapsed < OPEN_BOUND);
    eprintln!(
        "MT-123_BACKLOG_MEASUREMENT before_scopes={} before_bytes={} after_scopes={} after_bytes={} first_cold_open_ms={} recovery_open_ms={} reopen_ms={}",
        before.scope_count,
        before.contained_data_bytes,
        after.scope_count,
        after.contained_data_bytes,
        first_cold_open.expect("record first cold open").as_millis(),
        recovery_open_elapsed.as_millis(),
        reopen_elapsed.as_millis()
    );
    reopened
        .shutdown_and_cleanup()
        .await
        .expect("clean up reopened store");
}

#[test]
fn remaining_leak_modes_are_explicit_and_bounded() {
    assert_eq!(remaining_leak_modes().len(), 5);
    assert!(remaining_leak_modes()
        .iter()
        .all(|mode| mode.contains("skip") || mode.contains("until") || mode.contains("never")));
}
