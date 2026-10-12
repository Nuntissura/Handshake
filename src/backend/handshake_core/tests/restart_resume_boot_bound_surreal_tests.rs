//! Exact-scope embedded-Surreal proof that the startup restart-resume pass is
//! hard bounded and records fail-closed timeout evidence.

mod process_ledger_surreal_support;

use std::time::{Duration, Instant};

use handshake_core::process_ledger::restart_resume::{
    BoundedRestartResumeOutcome, SurrealRestartResumeRunner,
};
use process_ledger_surreal_support::ProcessLedgerSurrealHarness;
use surrealdb::types::{RecordId, SurrealValue};

#[derive(Debug, SurrealValue)]
struct ReportBindings {
    report: RecordId,
    owner_account_id: String,
    actor_principal_id: String,
    authenticated_session_id: String,
    access_space_id: String,
    workspace_id: String,
}

#[derive(Debug, SurrealValue)]
struct ReportProbe {
    fr_events_emitted: Vec<String>,
}

const READ_EXACT_REPORT: &str = r#"
SELECT fr_events_emitted FROM ONLY $report
WHERE owner_account_id = $owner_account_id
    AND actor_principal_id = $actor_principal_id
    AND authenticated_session_id = $authenticated_session_id
    AND access_space_id = $access_space_id
    AND workspace_id = $workspace_id;
"#;

#[tokio::test]
async fn restart_resume_boot_pass_is_hard_bounded_and_persists_incomplete_marker() {
    let harness = ProcessLedgerSurrealHarness::open().await;
    let storage = harness.storage();
    let scope = harness.resource_scope().clone();
    let runner = harness.restart_runner().await;
    let bound = Duration::ZERO;
    let started = Instant::now();
    let outcome = runner
        .run_with_bound(bound)
        .await
        .expect("bounded boot pass returns without hanging or panicking");
    let elapsed = started.elapsed();

    let (timeout, report, evidence_persisted) = match outcome {
        BoundedRestartResumeOutcome::TimedOut {
            timeout,
            report,
            evidence_persisted,
        } => (timeout, report, evidence_persisted),
        BoundedRestartResumeOutcome::Completed(_) => {
            panic!("a zero-duration bound must not allow the asynchronous store pass to complete")
        }
    };
    assert_eq!(timeout, bound);
    assert!(elapsed < Duration::from_secs(5));
    assert!(evidence_persisted);

    let probe = storage
        .with_data_operation(|database| {
            let bindings = ReportBindings {
                report: RecordId::new("kernel_restart_resume_report", report.report_id.to_string()),
                owner_account_id: scope.account_uuid.to_string(),
                actor_principal_id: scope.actor_uuid.to_string(),
                authenticated_session_id: scope.session_uuid.to_string(),
                access_space_id: scope.access_space_uuid.to_string(),
                workspace_id: scope.workspace_id,
            };
            Box::pin(async move {
                let values = database
                    .query_values::<surrealdb::types::Value, _>(READ_EXACT_REPORT, bindings)
                    .await?;
                if matches!(values.as_slice(), [surrealdb::types::Value::None]) {
                    return Ok(None);
                }
                let rows = values
                    .into_iter()
                    .map(|value| {
                        ReportProbe::from_value(value)
                            .map_err(|error| surrealdb::Error::internal(error.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows.into_iter().next())
            })
        })
        .await
        .expect("read exact-scope bounded-abort report")
        .expect("bounded-abort report exists");
    assert!(probe
        .fr_events_emitted
        .iter()
        .any(|event| event == "FR-EVT-RESTART-RESUME-STARTED"));
    assert!(!probe
        .fr_events_emitted
        .iter()
        .any(|event| event == "FR-EVT-RESTART-RESUME-COMPLETED"));
    // The exact healthy zero-bound assertions above remain the acceptance floor.
    // An unavailable report acknowledgement must not extend the admitted one-second grace.
    for fault in [
        handshake_core::process_ledger::restart_resume::AbortReportFault::Block,
        handshake_core::process_ledger::restart_resume::AbortReportFault::PersistenceFailure,
        handshake_core::process_ledger::restart_resume::AbortReportFault::AcknowledgementMismatch,
    ] {
        let fault_runner = harness.restart_runner().await;
        fault_runner.arm_abort_report_fault_for_test(fault);
        let began = Instant::now();
        let outcome = fault_runner
            .run_with_bound(Duration::ZERO)
            .await
            .expect("incomplete report failure remains a bounded outcome");
        assert!(began.elapsed() < Duration::from_secs(2));
        match outcome {
            BoundedRestartResumeOutcome::TimedOut {
                evidence_persisted,
                report,
                ..
            } => {
                assert!(
                    !evidence_persisted,
                    "missing/error acknowledgement is unconfirmed"
                );
                assert!(report.sessions_resumed.is_empty());
                assert!(!report
                    .fr_events_emitted
                    .iter()
                    .any(|event| event == "FR-EVT-RESTART-RESUME-COMPLETED"));
            }
            BoundedRestartResumeOutcome::Completed(_) => panic!("zero-bound fault completed"),
        }
    }
    #[derive(SurrealValue)]
    struct BlockedBindings {
        session: String,
        owner: String,
        actor: String,
        authenticated: String,
        space: String,
        workspace: String,
    }
    let candidate = uuid::Uuid::now_v7().to_string();
    let origin = harness.resource_scope().clone();
    storage.with_data_operation(|database| {
        let bindings = BlockedBindings { session: candidate.clone(), owner: origin.account_uuid.to_string(), actor: origin.actor_uuid.to_string(), authenticated: origin.session_uuid.to_string(), space: origin.access_space_uuid.to_string(), workspace: origin.workspace_id.clone() };
        Box::pin(async move { database.query_values::<surrealdb::types::Value, _>("CREATE type::record('kernel_session_queue', $session) SET session_run_id=$session, kernel_task_run_id='bounded-cleanup-fixture', adapter_id='bounded-cleanup-fixture', state='RUNNING', owner_account_id=$owner, actor_principal_id=$actor, authenticated_session_id=$authenticated, access_space_id=$space, workspace_id=$workspace;", bindings).await.map(|rows| rows.len()) })
    }).await.expect("seed exact-scope blocking-cleanup candidate");
    let old_lease = handshake_core::process_ledger::acquire_embedded_runtime_instance_lease(
        uuid::Uuid::now_v7(), harness.restart_host_scope())
        .expect("acquire prior runtime lease before fixture START");
    let process = handshake_core::process_ledger::ProcessStart::new(
        handshake_core::process_ledger::ProcessEngineKind::Candle,
        "bounded-cleanup-fixture",
        None,
    )
    .with_parent_session_id(candidate)
    .with_sandbox_adapter_id(handshake_core::sandbox::HANDSHAKE_NATIVE_SANDBOX_ADAPTER_ID)
    .with_runtime_owner(old_lease.descriptor().process_runtime_owner())
    .with_metadata_jsonb(harness.metadata());
    let process_uuid = process.process_uuid;
    let process = process.with_sandbox_internal_id(process_uuid.to_string());
    harness
        .write_batch(vec![handshake_core::process_ledger::LedgerEvent::Start(
            process,
        )])
        .await
        .expect("seed owned fixture process");
    drop(old_lease);
    let entered = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let fault = std::sync::Arc::new(process_ledger_surreal_support::BlockingScopedKill::new(
        origin, process_uuid, entered.clone()));
    let blocked_runner = harness.restart_runner_with_fault(Some(fault)).await;
    let began = Instant::now();
    let outcome = blocked_runner
        .run_with_bound(Duration::from_millis(100))
        .await
        .expect("blocking cleanup is bounded");
    assert!(began.elapsed() < Duration::from_secs(2));
    assert!(entered.load(std::sync::atomic::Ordering::Acquire));
    assert!(outcome.timed_out());
    assert!(outcome.report().sessions_resumed.is_empty());
    assert!(!outcome
        .report()
        .fr_events_emitted
        .iter()
        .any(|event| event == "FR-EVT-RESTART-RESUME-COMPLETED"));
    #[derive(SurrealValue)]
    struct ProcessProbeBindings {
        process: RecordId,
    }
    #[derive(SurrealValue)]
    struct ProcessProbe {
        stop_reason: Option<String>,
        exit_code: Option<i64>,
    }
    let row = storage
        .with_data_operation(|database| {
            Box::pin(async move {
                let values = database
                    .query_values::<surrealdb::types::Value, _>(
                        "SELECT stop_reason, exit_code FROM ONLY $process;",
                        ProcessProbeBindings {
                            process: RecordId::new(
                                "kernel_process_lifecycle",
                                process_uuid.to_string(),
                            ),
                        },
                    )
                    .await?;
                if matches!(values.as_slice(), [surrealdb::types::Value::None]) {
                    return Ok(None);
                }
                let rows = values
                    .into_iter()
                    .map(|value| {
                        ProcessProbe::from_value(value)
                            .map_err(|error| surrealdb::Error::internal(error.to_string()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(rows.into_iter().next())
            })
        })
        .await
        .expect("read canonical blocked cleanup row")
        .expect("fixture process remains");
    assert_ne!(row.stop_reason.as_deref(), Some("reclaim_killed"));
    assert!(
        row.exit_code.is_none(),
        "timed-out cleanup must not create a terminal STOP"
    );
    harness.close().await;
}
