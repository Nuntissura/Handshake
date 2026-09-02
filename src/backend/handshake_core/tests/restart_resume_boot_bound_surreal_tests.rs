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
    let runner = SurrealRestartResumeRunner::new(storage.clone(), scope.clone());
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
                database
                    .query_first::<ReportProbe, _>(READ_EXACT_REPORT, bindings)
                    .await
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
    harness.close().await;
}
