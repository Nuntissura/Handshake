use super::*;

pub(super) async fn perform_startup_recovery(
    surreal: &crate::storage::surreal::SurrealStorage,
    reclaim_resource_scope: ReclaimResourceScope,
) -> Result<(), Box<dyn std::error::Error>> {
    let sandbox_registry = Arc::new(build_startup_recovery_registry_async().await);
    let process_store = Arc::new(SurrealProcessLedgerStore::new(
        surreal.clone(),
    ));
    let startup_cleanup = Arc::new(RegistryStartupProcessCleanup::new(sandbox_registry));
    let orphan_reclaimer = Arc::new(SurrealRestartOrphanReclaimer::new(
        process_store,
        startup_cleanup,
    ));
    let restart_outcome = SurrealRestartResumeRunner::open(
        surreal.clone(),
        reclaim_resource_scope.clone(),
        orphan_reclaimer,
    )
    .await?
    .run_with_bound(restart_resume_boot_timeout())
    .await?;
    let restart_report = match restart_outcome {
        BoundedRestartResumeOutcome::Completed(report) => report,
        BoundedRestartResumeOutcome::TimedOut {
            timeout,
            report,
            evidence_persisted,
        } => {
            tracing::error!(target: "handshake_core::restart_resume",
                report_id = %report.report_id, timeout_ms = timeout.as_millis() as u64,
                evidence_persisted, "startup restart-resume timed out; incomplete report persistence is confirmed only when acknowledged");
            report
        }
    };
    write_startup_recovery_report(&restart_report)?;
    tracing::info!(
        target: "handshake_core::restart_resume",
        sessions_examined = restart_report.sessions_examined,
        sessions_resumed = restart_report.sessions_resumed.len(),
        sessions_recovery_failed = restart_report.sessions_recovery_failed.len(),
        "startup restart-resume outcome recorded against embedded SurrealDB"
    );
    Ok(())
}


pub(super) fn startup_recovery_only_requested() -> bool {
    std::env::var("HANDSHAKE_STARTUP_RECOVERY_ONLY")
        .ok()
        .as_deref()
        == Some("1")
}

fn write_startup_recovery_report(
    report: &crate::session_checkpoint::ResumeReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = std::env::var_os("HANDSHAKE_STARTUP_RECOVERY_REPORT_FILE").map(PathBuf::from)
    else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let payload = serde_json::json!({
        "report_id": report.report_id,
        "sessions_examined": report.sessions_examined,
        "sessions_resumed": report.sessions_resumed.len(),
        "sessions_recovery_failed": report.sessions_recovery_failed.len(),
        "fr_events_emitted": report.fr_events_emitted,
    });
    std::fs::write(path, serde_json::to_vec_pretty(&payload)?)?;
    Ok(())
}


fn restart_resume_boot_timeout() -> Duration {
    std::env::var("HANDSHAKE_RESTART_RESUME_BOOT_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .map(Duration::from_millis)
        .unwrap_or(RESTART_RESUME_BOOT_TIMEOUT_DEFAULT)
}
