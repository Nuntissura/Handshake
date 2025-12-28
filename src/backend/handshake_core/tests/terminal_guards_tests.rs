use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::{duckdb::DuckDbFlightRecorder, EventFilter, FlightRecorder};
use handshake_core::terminal::config::TerminalConfig;
use handshake_core::terminal::guards::{DefaultTerminalGuard, TerminalGuard};
use handshake_core::terminal::redaction::PatternRedactor;
use handshake_core::terminal::{
    JobContext, TerminalMode, TerminalRequest, TerminalService, TerminalSessionType,
};
use tempfile::tempdir;
use uuid::Uuid;

fn slow_command() -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        (
            "ping".to_string(),
            vec!["127.0.0.1".to_string(), "-n".to_string(), "5".to_string()],
        )
    } else {
        ("sleep".to_string(), vec!["2".to_string()])
    }
}

fn echo_command(msg: &str) -> (String, Vec<String>) {
    if cfg!(target_os = "windows") {
        (
            "cmd".to_string(),
            vec!["/C".into(), "echo".into(), msg.to_string()],
        )
    } else {
        ("echo".to_string(), vec![msg.to_string()])
    }
}

fn default_request(command: String, args: Vec<String>, cwd: Option<PathBuf>) -> TerminalRequest {
    TerminalRequest {
        command,
        args,
        cwd,
        mode: TerminalMode::NonInteractive,
        timeout_ms: Some(5_000),
        max_output_bytes: None,
        env_overrides: HashMap::new(),
        capture_stdout: true,
        capture_stderr: true,
        stdin_chunks: Vec::new(),
        idempotency_key: None,
        job_context: JobContext {
            job_id: Some("job-1".to_string()),
            model_id: None,
            session_id: None,
            capability_profile_id: Some("Coder".to_string()),
            capability_id: Some("terminal.exec".to_string()),
            wsids: Vec::new(),
        },
        granted_capabilities: Vec::new(),
        requested_capability: Some("terminal.exec".to_string()),
        session_type: TerminalSessionType::AiJob,
        human_consent_obtained: false,
    }
}

#[allow(clippy::type_complexity)]
fn default_deps(
    workspace_root: PathBuf,
) -> (
    TerminalConfig,
    CapabilityRegistry,
    Arc<dyn FlightRecorder>,
    Vec<Box<dyn TerminalGuard>>,
    PatternRedactor,
) {
    let cfg = TerminalConfig::new(workspace_root);
    let registry = CapabilityRegistry::new();
    let recorder: Arc<dyn FlightRecorder> =
        Arc::new(DuckDbFlightRecorder::new_in_memory(7).unwrap());
    let guards: Vec<Box<dyn TerminalGuard>> = vec![Box::new(DefaultTerminalGuard)];
    let redactor = PatternRedactor;
    (cfg, registry, recorder, guards, redactor)
}

#[tokio::test]
async fn allows_cwd_inside_workspace() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let workspace_root = dir.path().to_path_buf();
    std::fs::create_dir_all(workspace_root.join("subdir"))?;

    let (cmd, args) = echo_command("hello");
    let request = default_request(cmd, args, Some(PathBuf::from("subdir")));
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let result = TerminalService::run_command(
        request,
        &cfg,
        &registry,
        recorder.as_ref(),
        Uuid::new_v4(),
        &redactor,
        &guards,
    )
    .await?;

    assert!(result.stdout.to_lowercase().contains("hello"));
    assert!(!result.timed_out);
    Ok(())
}

#[tokio::test]
async fn blocks_cwd_escape() {
    let dir = tempdir().unwrap();
    let workspace_root = dir.path().to_path_buf();
    let (cmd, args) = echo_command("blocked");
    let request = default_request(cmd, args, Some(PathBuf::from("..")));
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let result = TerminalService::run_command(
        request,
        &cfg,
        &registry,
        recorder.as_ref(),
        Uuid::new_v4(),
        &redactor,
        &guards,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn enforces_timeout_and_kill_grace() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let workspace_root = dir.path().to_path_buf();
    let (cmd, args) = slow_command();
    let mut request = default_request(cmd, args, None);
    request.timeout_ms = Some(200);
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let result = TerminalService::run_command(
        request,
        &cfg,
        &registry,
        recorder.as_ref(),
        Uuid::new_v4(),
        &redactor,
        &guards,
    )
    .await?;

    assert!(result.timed_out);
    Ok(())
}

#[tokio::test]
async fn flags_truncation_when_output_exceeds_limit() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let workspace_root = dir.path().to_path_buf();
    let big_content = "A".repeat(1_200_000);
    let file_path = workspace_root.join("big.txt");
    std::fs::write(&file_path, big_content)?;

    let (cmd, args) = if cfg!(target_os = "windows") {
        (
            "cmd".to_string(),
            vec![
                "/C".into(),
                "type".into(),
                file_path.to_string_lossy().to_string(),
            ],
        )
    } else {
        (
            "cat".to_string(),
            vec![file_path.to_string_lossy().to_string()],
        )
    };

    let mut request = default_request(cmd, args, None);
    request.max_output_bytes = Some(1_000_000);
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let result = TerminalService::run_command(
        request,
        &cfg,
        &registry,
        recorder.as_ref(),
        Uuid::new_v4(),
        &redactor,
        &guards,
    )
    .await?;

    assert!(result.truncated_bytes > 0);
    assert!(result.stdout.len() as u64 <= 1_000_000);
    Ok(())
}

#[tokio::test]
async fn redacts_secrets_in_logged_command() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let workspace_root = dir.path().to_path_buf();
    let (cmd, args) = echo_command("API_KEY=supersecret");
    let request = default_request(cmd, args, None);
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let _ = TerminalService::run_command(
        request,
        &cfg,
        &registry,
        recorder.as_ref(),
        Uuid::new_v4(),
        &redactor,
        &guards,
    )
    .await?;

    let events = recorder
        .list_events(EventFilter::default())
        .await
        .unwrap_or_default();
    let payload = events
        .last()
        .and_then(|evt| evt.payload.get("command").cloned())
        .unwrap_or_default();

    let command_str = payload.as_str().unwrap_or_default().to_string();
    assert!(!command_str.contains("supersecret"));
    assert!(command_str.contains("***REDACTED***"));

    Ok(())
}
