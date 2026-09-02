mod surreal_test_store_support;

use std::fs;
#[cfg(windows)]
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use surreal_test_store_support::{
    cleanup_embedded_surreal_scopes, measure_owned_scopes, sweep_stale_orphans,
    EmbeddedSurrealTestScope,
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
    assert!(
        first_error.contains("timeout") || first_error.contains("exceeded"),
        "first failure must be the injected bounded timeout: {first_error}"
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
    let recovered = scopes[0].cleanup().await.expect("retry earlier cleanup");
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
