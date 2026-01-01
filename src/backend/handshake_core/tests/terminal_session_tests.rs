use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use handshake_core::capabilities::CapabilityRegistry;
use handshake_core::flight_recorder::{duckdb::DuckDbFlightRecorder, EventFilter, FlightRecorder};
use handshake_core::terminal::config::TerminalConfig;
use handshake_core::terminal::guards::{DefaultTerminalGuard, TerminalGuard};
use handshake_core::terminal::redaction::{PatternRedactor, SecretRedactor};
use handshake_core::terminal::{
    JobContext, TerminalError, TerminalMode, TerminalRequest, TerminalService, TerminalSessionType,
};
use tempfile::tempdir;
use tokio::time::sleep;
use uuid::Uuid;

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

fn ai_request(command: String, args: Vec<String>) -> TerminalRequest {
    TerminalRequest {
        command,
        args,
        cwd: None,
        mode: TerminalMode::NonInteractive,
        timeout_ms: Some(5_000),
        max_output_bytes: None,
        env_overrides: HashMap::new(),
        capture_stdout: true,
        capture_stderr: true,
        stdin_chunks: Vec::new(),
        idempotency_key: None,
        job_context: JobContext {
            job_id: Some("job-cancel".to_string()),
            model_id: None,
            session_id: None,
            capability_profile_id: Some("Coder".to_string()),
            capability_id: Some("terminal.exec".to_string()),
            wsids: vec!["ws-1".to_string()],
        },
        granted_capabilities: Vec::new(),
        requested_capability: Some("terminal.exec".to_string()),
        session_type: TerminalSessionType::AiJob,
        human_consent_obtained: false,
    }
}

fn human_target_request(command: String, args: Vec<String>) -> TerminalRequest {
    TerminalRequest {
        command,
        args,
        cwd: None,
        mode: TerminalMode::NonInteractive,
        timeout_ms: Some(5_000),
        max_output_bytes: None,
        env_overrides: HashMap::new(),
        capture_stdout: true,
        capture_stderr: true,
        stdin_chunks: Vec::new(),
        idempotency_key: None,
        job_context: JobContext {
            job_id: Some("job-attach".to_string()),
            model_id: None,
            session_id: Some("session-123".to_string()),
            capability_profile_id: None,
            capability_id: Some("terminal.exec".to_string()),
            wsids: vec!["ws-1".to_string()],
        },
        granted_capabilities: vec!["terminal.exec".to_string()],
        requested_capability: Some("terminal.exec".to_string()),
        session_type: TerminalSessionType::HumanDev,
        human_consent_obtained: false,
    }
}

#[tokio::test]
async fn blocks_ai_from_human_session_without_attach_capability() {
    let dir = tempdir().unwrap();
    let workspace_root = dir.path().to_path_buf();
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let (cmd, args) = echo_command("nope");
    let request = human_target_request(cmd, args);

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

    assert!(matches!(result, Err(TerminalError::IsolationViolation(_))));
}

#[tokio::test]
async fn allows_ai_with_attach_capability_and_logged_consent(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let workspace_root = dir.path().to_path_buf();
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let (cmd, args) = echo_command("ok");
    let mut request = human_target_request(cmd, args);
    request.human_consent_obtained = true;
    request
        .granted_capabilities
        .push("terminal.attach_human".to_string());

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

    assert!(result.stdout.to_lowercase().contains("ok"));
    Ok(())
}

#[tokio::test]
async fn flight_recorder_captures_session_type_and_consent(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let workspace_root = dir.path().to_path_buf();
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let (cmd, args) = echo_command("audit");
    let mut request = human_target_request(cmd, args);
    request.human_consent_obtained = true;
    request
        .granted_capabilities
        .push("terminal.attach_human".to_string());

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

    let events = recorder.list_events(EventFilter::default()).await?;
    // Find the TerminalCommandEvent specifically (not CapabilityAction events)
    let terminal_event = events.iter().find(|evt| {
        evt.payload
            .get("type")
            .and_then(|v| v.as_str())
            .map(|t| t == "terminal_command")
            .unwrap_or(false)
    });
    let payload = terminal_event
        .and_then(|evt| evt.payload.as_object())
        .cloned()
        .unwrap_or_default();

    assert_eq!(
        payload.get("session_type").and_then(|v| v.as_str()),
        Some("HUMAN_DEV")
    );
    assert_eq!(
        payload
            .get("human_consent_obtained")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    let capability_set = payload
        .get("capability_set")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(capability_set
        .iter()
        .any(|cap| cap.as_str() == Some("terminal.attach_human")));
    Ok(())
}

#[tokio::test]
async fn redaction_handles_non_utf8_output() {
    let redactor = PatternRedactor;
    let mut stdout = vec![0xff, 0xfe];
    stdout.extend_from_slice(b"API_KEY=secret");

    let result = redactor.redact_output(&stdout, &[]);
    assert!(result.redacted.contains("***REDACTED***"));
}

#[tokio::test]
async fn cancels_inflight_command() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let workspace_root = dir.path().to_path_buf();
    let (cfg, registry, recorder, guards, redactor) = default_deps(workspace_root);

    let (cmd, args) = slow_command();
    let mut request = ai_request(cmd, args);
    request.idempotency_key = Some("cancel-1".to_string());

    let handle = tokio::spawn(async move {
        TerminalService::run_command(
            request,
            &cfg,
            &registry,
            recorder.as_ref(),
            Uuid::new_v4(),
            &redactor,
            &guards,
        )
        .await
    });

    sleep(Duration::from_millis(200)).await;
    assert!(TerminalService::request_cancel("cancel-1"));

    let result = handle.await??;
    assert!(result.cancelled);
    Ok(())
}
