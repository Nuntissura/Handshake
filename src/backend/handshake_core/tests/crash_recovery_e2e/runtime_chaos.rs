#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres; run with POSTGRES_TEST_URL and `cargo test --test crash_recovery_e2e mt195_runtime -- --ignored`"]
async fn mt195_runtime_hard_kill_child_leaves_checkpoint_process_evidence_and_persists_report() {
    let evidence = crate::runtime_child::run_hard_kill_child_recovery().await;

    assert!(evidence.child_was_hard_killed);
    assert!(evidence.child_process_row_was_left_without_graceful_stop);
    assert_eq!(evidence.checkpoint_rows, 1);
    assert_eq!(evidence.process_rows, 1);
    assert_eq!(evidence.report_rows, 1);
    assert_eq!(evidence.resumed_sessions, 1);
    assert_eq!(evidence.failed_sessions, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres and app-runtime binary; run with POSTGRES_TEST_URL and `cargo test --features test-utils,app-runtime --test crash_recovery_e2e mt195_runtime -- --ignored`"]
async fn mt195_runtime_real_handshake_core_binary_runs_startup_recovery_and_persists_report() {
    let evidence = crate::runtime_child::run_real_handshake_core_startup_recovery().await;

    assert!(evidence.real_binary_was_spawned);
    assert!(evidence.startup_recovery_only_exit);
    assert_eq!(evidence.process_exit_success, Some(true));
    assert_eq!(evidence.report_rows, 1);
    assert_eq!(evidence.resumed_sessions, 1);
    assert_eq!(evidence.failed_sessions, 0);
    assert_eq!(evidence.retry_scheduled_queue_rows, 1);
    assert_eq!(evidence.post_failure_checkpoint_rows, 1);
    assert_eq!(evidence.final_counter, Some(3));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres; run with POSTGRES_TEST_URL and `cargo test --test crash_recovery_e2e mt195_runtime -- --ignored`"]
async fn mt195_runtime_event_gap_persists_failed_report_and_decision_evidence() {
    let evidence = crate::runtime_postgres::run_failed_recovery(
        crate::runtime_postgres::FailureScenario::EventSeqGap,
    )
    .await;

    assert_eq!(evidence.report_rows, 1);
    assert_eq!(evidence.resumed_sessions, 0);
    assert_eq!(evidence.failed_sessions, 1);
    assert_eq!(evidence.operator_decision_rows, 1);
    assert_eq!(evidence.failed_checkpoint_rows, 1);
    assert!(evidence.report_contains_recovery_failed_event);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres; run with POSTGRES_TEST_URL and `cargo test --test crash_recovery_e2e mt195_runtime -- --ignored`"]
async fn mt195_runtime_operator_cancel_persists_failed_report_and_decision_evidence() {
    let evidence = crate::runtime_postgres::run_failed_recovery(
        crate::runtime_postgres::FailureScenario::OperatorCancelDuringRecovery,
    )
    .await;

    assert_eq!(evidence.report_rows, 1);
    assert_eq!(evidence.resumed_sessions, 0);
    assert_eq!(evidence.failed_sessions, 1);
    assert_eq!(evidence.operator_decision_rows, 1);
    assert_eq!(evidence.failed_checkpoint_rows, 1);
    assert!(evidence.report_contains_recovery_failed_event);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres with pg_terminate_backend permission; run with POSTGRES_TEST_URL and `cargo test --test crash_recovery_e2e mt195_runtime -- --ignored`"]
async fn mt195_runtime_backend_termination_records_db_loss_without_shared_db_damage() {
    let evidence = crate::runtime_postgres::run_backend_termination_recovery().await;

    assert!(evidence.backend_termination_observed);
    assert_eq!(evidence.report_rows, 1);
    assert_eq!(evidence.resumed_sessions, 0);
    assert_eq!(evidence.failed_sessions, 1);
    assert_eq!(evidence.operator_decision_rows, 1);
    assert!(evidence.report_contains_db_unavailable_event);
    assert!(evidence.schema_isolation_verified);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires live Postgres; run with POSTGRES_TEST_URL and `cargo test --test crash_recovery_e2e mt195_runtime -- --ignored`"]
async fn mt195_runtime_transient_db_unavailable_backs_off_and_resumes_after_db_return_without_shared_db_damage(
) {
    let evidence = crate::runtime_postgres::run_transient_db_unavailable_backoff_recovery().await;

    assert_eq!(evidence.db_unavailable_attempts, 1);
    assert!(evidence.backoff_observed);
    assert_eq!(evidence.report_rows, 1);
    assert_eq!(evidence.resumed_sessions, 1);
    assert_eq!(evidence.failed_sessions, 0);
    assert!(evidence.report_contains_db_unavailable_event);
    assert!(evidence.schema_isolation_verified);
}
