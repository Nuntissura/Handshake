mod surreal_test_store_support;

use std::fs;
#[cfg(windows)]
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use surreal_test_store_support::{
    cleanup_embedded_surreal_scopes, independent_namespace_databases_for_proof,
    independent_root_namespaces_for_proof, measure_owned_scopes, sweep_stale_orphans,
    EmbeddedSurrealTestScope, DEFAULT_EMBEDDED_SCOPE_TIMEOUT,
};
use tokio::sync::Notify;

const EXIT_CHILD_ROOT_ENV: &str = "HANDSHAKE_MT024_EXIT_CHILD_ROOT";
const EXIT_CHILD_RECEIPT: &str = "mt024-exit-child-scope.receipt";

#[tokio::test]
async fn exit_recovery_child_allocates_scope_without_normal_teardown() {
    let Some(root) = std::env::var_os(EXIT_CHILD_ROOT_ENV) else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let scope = EmbeddedSurrealTestScope::create_in(&root)
        .await
        .expect("allocate child-owned exit-recovery scope");
    fs::write(
        root.join(EXIT_CHILD_RECEIPT),
        format!(
            "{}\n{}\n{}\n",
            scope.namespace(),
            scope.database(),
            scope.store_path().display()
        ),
    )
    .expect("write child allocation receipt");
    drop(scope);
}

#[cfg(windows)]
#[tokio::test]
async fn process_exit_recovery_is_observable_exact_and_foreign_safe() {
    let root = tempfile::tempdir().expect("create MT-024 exit-recovery root");
    let mut foreign = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate live foreign scope");
    let foreign_path = foreign.store_path().to_path_buf();
    let foreign_scope_path = foreign_path
        .parent()
        .expect("foreign store has allocator-owned scope parent")
        .to_path_buf();
    foreign
        .write_foreign_survival_sentinel()
        .await
        .expect("write foreign sentinel before child exit");

    let output = Command::new(std::env::current_exe().expect("resolve current test executable"))
        .arg("--exact")
        .arg("exit_recovery_child_allocates_scope_without_normal_teardown")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(EXIT_CHILD_ROOT_ENV, root.path())
        .output()
        .expect("run exit-recovery child");
    assert!(
        output.status.success(),
        "exit-recovery child must exit normally: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let child_stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        child_stderr.contains("embedded_surreal_cleanup_pending"),
        "missing observable non-panicking exit diagnostic: {child_stderr}"
    );

    let receipt = fs::read_to_string(root.path().join(EXIT_CHILD_RECEIPT))
        .expect("read child allocation receipt");
    let mut receipt_lines = receipt.lines();
    let child_namespace = receipt_lines.next().expect("child namespace receipt");
    let child_database = receipt_lines.next().expect("child database receipt");
    let child_store_path =
        std::path::PathBuf::from(receipt_lines.next().expect("child physical-store receipt"));
    assert!(child_namespace.starts_with("hs_test_ns_"));
    assert!(child_database.starts_with("hs_test_db_"));
    assert!(receipt_lines.next().is_none(), "unexpected receipt fields");
    let child_scope_path = child_store_path
        .parent()
        .expect("child store has allocator-owned scope parent")
        .to_path_buf();
    assert!(
        child_scope_path.exists(),
        "child scope must remain after exit"
    );
    assert_eq!(
        measure_owned_scopes(root.path())
            .expect("measure child and foreign scopes")
            .scope_count,
        2
    );

    let report = sweep_stale_orphans(root.path(), Duration::ZERO)
        .expect("recover stale child scope after process exit");
    assert_eq!(report.reclaimed, vec![child_scope_path]);
    assert!(report.reclaimed_owner_markers.is_empty());
    assert!(report.skipped_recent.is_empty());
    assert!(report.skipped_unproven.is_empty());
    assert!(report.rejected_unsafe.is_empty());
    assert!(report.errors.is_empty());
    assert!(
        report
            .skipped_live
            .iter()
            .any(|path| path == &foreign_scope_path),
        "live foreign scope must be recognized and skipped"
    );
    assert!(
        foreign_path.exists(),
        "foreign scope path must survive recovery"
    );
    assert!(
        foreign
            .foreign_survival_sentinel_exists()
            .await
            .expect("reread foreign sentinel after child recovery"),
        "foreign scope data must survive child recovery"
    );
    assert_eq!(
        measure_owned_scopes(root.path())
            .expect("measure foreign scope after child recovery")
            .scope_count,
        1
    );

    foreign
        .cleanup()
        .await
        .expect("normally clean foreign scope early");
    assert_eq!(
        measure_owned_scopes(root.path())
            .expect("measure empty allocator root")
            .scope_count,
        0
    );
}

/// The recovery fence runs at allocation time and must report what it did.
///
/// A crashed run leaves its exact scope plus a released ownership marker. The
/// next allocation in the same root reclaims exactly that scope, leaves a live
/// foreign scope alone, and exposes both outcomes as a receipt rather than
/// discarding them.
#[cfg(windows)]
#[tokio::test]
async fn allocation_recovery_fence_reports_exact_reclaim_and_spares_live_scope() {
    let root = tempfile::tempdir().expect("create MT-024 recovery-fence root");
    let mut live = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate live foreign scope");
    let live_scope_path = live
        .store_path()
        .parent()
        .expect("live store has allocator-owned scope parent")
        .to_path_buf();
    live.write_foreign_survival_sentinel()
        .await
        .expect("write live foreign sentinel");

    let orphan = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate scope that will be orphaned");
    let orphan_scope_path = orphan
        .leave_closed_orphan_for_proof()
        .await
        .expect("leave crash-shaped orphan behind");
    assert!(
        orphan_scope_path.exists(),
        "orphan must survive until the next allocation sweeps"
    );

    let mut next = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate scope whose recovery fence reclaims the orphan");
    let report = next.startup_sweep_report();
    assert_eq!(
        report.reclaimed,
        vec![orphan_scope_path.clone()],
        "the fence must reclaim exactly the orphaned scope"
    );
    assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
    assert!(
        report.rejected_unsafe.is_empty(),
        "rejected: {:?}",
        report.rejected_unsafe
    );
    assert!(
        report.skipped_live.iter().any(|path| path == &live_scope_path),
        "the live foreign scope must be recognized and skipped: {report:?}"
    );
    assert!(
        !orphan_scope_path.exists(),
        "the orphaned scope path must be gone"
    );
    assert!(
        live.foreign_survival_sentinel_exists()
            .await
            .expect("reread live foreign sentinel after the fence ran"),
        "live foreign data must survive the recovery fence"
    );

    next.cleanup().await.expect("clean the fence-running scope");
    live.cleanup().await.expect("clean the live foreign scope");
    assert_eq!(
        measure_owned_scopes(root.path())
            .expect("measure emptied recovery-fence root")
            .scope_count,
        0
    );
}

/// AC-4 as an assertion, not a manual step.
///
/// After the task-owned scopes are cleaned, each exact physical store is
/// re-opened with the official SDK and required to report zero of this run's
/// namespaces and databases. The re-read never consults the allocator state
/// that performed the cleanup, so a green cleanup receipt cannot make it
/// agree; a regression fails the build instead of waiting to be noticed.
///
/// The foreign-owned scope is staged by this test rather than inherited from
/// whatever another lane happened to leave behind, and it must survive
/// untouched. That is what makes the zero count evidence of EXACT teardown
/// instead of evidence of broad deletion, which would also read as zero.
#[tokio::test]
async fn independent_reread_finds_zero_task_owned_scopes_and_spares_a_foreign_scope() {
    let root = tempfile::tempdir().expect("create MT-024 leak-proof root");

    let mut foreign = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("stage foreign-owned scope");
    foreign
        .write_foreign_survival_sentinel()
        .await
        .expect("write staged foreign sentinel");
    let foreign_namespace = foreign.namespace().to_owned();
    let foreign_database = foreign.database().to_owned();
    let foreign_store = foreign.store_path().to_path_buf();
    // Release the engine handle so the independent re-read can open the same
    // physical store. The ownership marker stays held, so every allocation
    // below still sees this scope as live and must leave it alone.
    foreign
        .close_for_reopen()
        .await
        .expect("release foreign engine handle before independent reread");

    let mut owned = Vec::new();
    for _ in 0..3 {
        owned.push(
            EmbeddedSurrealTestScope::create_in(root.path())
                .await
                .expect("allocate task-owned scope"),
        );
    }
    let owned_identity: Vec<(String, String, std::path::PathBuf)> = owned
        .iter()
        .map(|scope| {
            (
                scope.namespace().to_owned(),
                scope.database().to_owned(),
                scope.store_path().to_path_buf(),
            )
        })
        .collect();

    let attempts = cleanup_embedded_surreal_scopes(&mut owned).await;
    assert_eq!(attempts.len(), owned_identity.len());
    for attempt in &attempts {
        assert!(
            attempt.succeeded,
            "every task-owned scope must clean up: {attempt:?}"
        );
    }

    let mut surviving_task_owned = Vec::new();
    for (namespace, database, store_path) in &owned_identity {
        let namespaces =
            independent_root_namespaces_for_proof(store_path, DEFAULT_EMBEDDED_SCOPE_TIMEOUT)
                .await
                .expect("independent canonical reread of a task-owned store");
        if namespaces.contains(namespace) {
            surviving_task_owned.push(format!("namespace {namespace}"));
        }
        let databases = independent_namespace_databases_for_proof(
            store_path,
            namespace,
            DEFAULT_EMBEDDED_SCOPE_TIMEOUT,
        )
        .await
        .expect("independent canonical database reread of a task-owned store");
        if databases.contains(database) {
            surviving_task_owned.push(format!("database {database}"));
        }
    }
    assert!(
        surviving_task_owned.is_empty(),
        "independent canonical reread must find zero task-owned scopes, found: {surviving_task_owned:?}"
    );

    let foreign_namespaces =
        independent_root_namespaces_for_proof(&foreign_store, DEFAULT_EMBEDDED_SCOPE_TIMEOUT)
            .await
            .expect("independent canonical reread of the staged foreign store");
    assert!(
        foreign_namespaces.contains(&foreign_namespace),
        "the staged foreign namespace must survive exact teardown"
    );
    let foreign_databases = independent_namespace_databases_for_proof(
        &foreign_store,
        &foreign_namespace,
        DEFAULT_EMBEDDED_SCOPE_TIMEOUT,
    )
    .await
    .expect("independent canonical database reread of the staged foreign store");
    assert!(
        foreign_databases.contains(&foreign_database),
        "the staged foreign database must survive exact teardown"
    );

    // Measured while the foreign engine is still closed. `measure_owned_scopes`
    // walks each scope's contained tree, and a LIVE embedded store rewrites that
    // tree underneath the walk, so a file can vanish between `read_dir` and
    // `symlink_metadata`. Counting a quiescent root is the honest measurement.
    assert_eq!(
        measure_owned_scopes(root.path())
            .expect("measure the root after task-owned teardown")
            .scope_count,
        1,
        "only the staged foreign scope may remain"
    );

    foreign
        .reopen()
        .await
        .expect("reopen the staged foreign scope");
    assert!(
        foreign
            .foreign_survival_sentinel_exists()
            .await
            .expect("reread the staged foreign sentinel"),
        "staged foreign data must survive exact teardown"
    );

    foreign
        .cleanup()
        .await
        .expect("clean the staged foreign scope by its own authority");
    assert_eq!(
        measure_owned_scopes(root.path())
            .expect("measure the emptied root")
            .scope_count,
        0
    );
}

#[tokio::test]
async fn cleanup_of_earlier_scope_preserves_later_foreign_scope_path() {
    let root = tempfile::tempdir().expect("create MT-024 scope root");
    let mut earlier = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate earlier scope");
    let mut later = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate later foreign scope");
    let earlier_path = earlier.store_path().to_path_buf();
    let later_path = later.store_path().to_path_buf();

    later
        .write_foreign_survival_sentinel()
        .await
        .expect("write fixed sentinel through test-support capability");
    let _later_storage = later
        .activate_storage()
        .await
        .expect("activate later production storage");
    later
        .shutdown_storage_for_reopen()
        .await
        .expect("close later production storage before proof reopen");

    let first_receipt = earlier.cleanup().await.expect("clean earlier exact scope");
    let repeated_receipt = earlier.cleanup().await.expect("repeat earlier cleanup");
    assert_eq!(
        repeated_receipt, first_receipt,
        "cleanup must be idempotent"
    );
    assert!(
        !earlier_path.exists(),
        "earlier private path must be removed"
    );
    assert!(later_path.exists(), "later foreign path must survive");

    later.reopen().await.expect("reopen later exact scope");
    let observed = later
        .foreign_survival_sentinel_exists()
        .await
        .expect("reread fixed sentinel through test-support capability");
    assert!(observed, "fixed foreign-scope sentinel must survive");

    later.cleanup().await.expect("clean later exact scope");
    assert!(
        !later_path.exists(),
        "later path must be removed by its own cleanup"
    );
}

#[tokio::test]
async fn cleanup_batch_attempts_later_scope_after_earlier_escaped_use_error() {
    let root = tempfile::tempdir().expect("create MT-024 batch root");
    let mut earlier = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate earlier bounded scope");
    earlier
        .set_storage_shutdown_timeout_for_proof(Duration::from_millis(100))
        .expect("set deterministic cleanup timeout");
    let later = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate later scope");
    let mut foreign = EmbeddedSurrealTestScope::create_in(root.path())
        .await
        .expect("allocate excluded foreign scope");
    let later_path = later.store_path().to_path_buf();
    let later_namespace = later.namespace().to_owned();
    let later_database = later.database().to_owned();
    let foreign_path = foreign.store_path().to_path_buf();
    foreign
        .write_foreign_survival_sentinel()
        .await
        .expect("write excluded foreign sentinel");

    let escaped = earlier
        .activate_storage()
        .await
        .expect("activate earlier production storage");
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let task_entered = Arc::clone(&entered);
    let task_release = Arc::clone(&release);
    let in_flight = tokio::spawn(async move {
        escaped
            .with_data_operation(|_database| {
                Box::pin(async move {
                    task_entered.notify_one();
                    task_release.notified().await;
                    Ok(())
                })
            })
            .await
    });
    entered.notified().await;

    let mut scopes = [earlier, later];
    let attempts = cleanup_embedded_surreal_scopes(&mut scopes).await;
    assert_eq!(attempts.len(), 2, "every included scope gets one attempt");
    assert!(
        !attempts[0].succeeded,
        "escaped in-flight use must fail loudly"
    );
    let first_error = attempts[0]
        .diagnostics
        .error
        .as_deref()
        .expect("failed cleanup exposes its error");
    // Bounded means the injected 100 ms deadline is what stopped the wait. The
    // receipt names that exact bound, so an unbounded wait or a silently
    // widened deadline breaks this assertion instead of hanging the suite.
    assert!(
        first_error.contains("still draining operations after 100 ms"),
        "first failure must be the injected 100 ms bounded-shutdown receipt: {first_error}"
    );
    assert!(
        attempts[0].diagnostics.elapsed < Duration::from_secs(5),
        "bounded failure must return near its own deadline, not the 120 s scope timeout: {:?}",
        attempts[0].diagnostics.elapsed
    );
    assert!(
        !attempts[0].diagnostics.database_absent
            && !attempts[0].diagnostics.namespace_absent_after_reopen,
        "a bounded failure must not claim the exact scope was removed"
    );
    assert!(
        attempts[1].succeeded,
        "later scope must still receive cleanup"
    );
    assert_eq!(attempts[1].namespace, later_namespace);
    assert_eq!(attempts[1].database, later_database);
    assert!(attempts[1].diagnostics.database_absent);
    assert!(attempts[1].diagnostics.namespace_absent_after_reopen);
    assert!(
        !later_path.exists(),
        "later cleanup must remove its exact path"
    );
    assert!(
        foreign_path.exists(),
        "excluded foreign path must survive the failed batch"
    );
    assert!(
        foreign
            .foreign_survival_sentinel_exists()
            .await
            .expect("reread excluded foreign sentinel"),
        "excluded foreign data must survive the failed batch"
    );

    release.notify_waiters();
    in_flight
        .await
        .expect("join escaped operation")
        .expect("escaped operation completes after release");
    // The bounded failure is recoverable, not terminal: the receipt says the
    // closure "continues in the background", so the caller retries against a
    // DEADLINE rather than assuming one attempt must now succeed. This scope
    // still carries the injected 100 ms shutdown bound, so a single retry can
    // legitimately land while the background close is mid-drain. The deadline
    // is what keeps this a bounded wait instead of a hang.
    let recovery_deadline = std::time::Instant::now() + Duration::from_secs(120);
    let recovered = loop {
        match scopes[0].cleanup().await {
            Ok(receipt) => break receipt,
            Err(error) => {
                assert!(
                    std::time::Instant::now() < recovery_deadline,
                    "recovery from the bounded failure exceeded its deadline: {error}"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    };
    assert_eq!(
        scopes[0].cleanup().await.expect("repeat recovered cleanup"),
        recovered,
        "recovered cleanup must remain idempotent"
    );
    foreign
        .cleanup()
        .await
        .expect("clean excluded foreign scope by its own authority");
}
