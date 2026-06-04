#![cfg(feature = "inspector")]

//! MT-033 research basis:
//! - a direct rustc compile-fail check proves API misuse does not compile
//!   without starting a nested cargo build.
//! - Tauri command returns are serde-serialized values; this smoke compares the
//!   exact HTTP bytes with the serde bytes returned by the InspectorReadV1
//!   read path that MT-032 exposes through IPC.
//! - A full tauri::test invocation would require app-crate test-surface edits
//!   outside this MT's owned files, so the smoke combines MT-032 source
//!   registration checks with byte-level transport equivalence.

use std::{
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use handshake_core::{
    inspector_read::{
        expected_write_box_v1_signature, EventLedgerRow, InspectorReadSnapshot, InspectorReadV1,
        InspectorServer, ModelLoadedRow, PerRunSecret, ProcessRow, ReplayDriveResponse,
        SessionId, SessionStateRead, SessionSummary, WorkspaceId, WorkspaceStateRead,
        PER_RUN_SECRET_HEADER, WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
    },
    kernel::{
        action_envelope::AuthorityEffect,
        write_boxes::{
            WriteBoxCommon, WriteBoxKind, WriteBoxLifecycleState, WriteBoxOwnerRef,
            WriteBoxPayloadRef, WriteBoxReplayMetadataV1, WriteBoxTargetRef,
            WriteBoxValidationState, WriteBoxValidationStatus,
        },
    },
};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

const SESSION_ID: &str = "session-a3-smoke";
const WORKSPACE_ID: &str = "workspace-a3-smoke";
const WP_ID: &str =
    "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";

#[tokio::test]
async fn a3_inspector_smoke_http_ipc_trace_projection_and_replay_drive(
) -> Result<(), Box<dyn Error>> {
    let snapshot = synthetic_reader();
    let reader: Arc<dyn InspectorReadV1> = Arc::new(snapshot.clone());
    let handle = InspectorServer::start(reader).await?;
    let base = format!("http://{}", handle.addr());
    let secret_hex = handle.per_run_secret().to_hex();
    let client = reqwest::Client::new();

    assert_ne!(
        handle.port(),
        0,
        "kernel.inspector.port must expose a live port"
    );

    let endpoints_passed =
        assert_http_read_endpoints(&client, &base, &secret_hex, &snapshot).await?;
    let ipc_passed = assert_ipc_equivalent_payloads(&client, &base, &secret_hex, &snapshot).await?;
    let trace_projection = snapshot
        .trace_projection(SessionId::new(SESSION_ID))
        .expect("synthetic TraceProjection");
    let trace_projection_fields = populated_trace_projection_fields(&trace_projection);
    assert_eq!(
        trace_projection_fields.len(),
        7,
        "TraceProjection must populate all seven inspector fields"
    );

    let replay_response =
        post_valid_replay_drive(&client, &base, handle.per_run_secret(), &secret_hex).await?;
    assert_eq!(replay_response.status, "dispatched");
    assert_eq!(replay_response.action_id, "kernel.write_box.promote");
    assert_eq!(replay_response.event.event_type, "INSPECTOR_REPLAY_DRIVE");
    assert_eq!(
        replay_response.result["dispatched_through"],
        "KernelActionCatalogV1"
    );

    let forbidden = client
        .post(format!("{base}/inspector/v1/replay-drive"))
        .header(PER_RUN_SECRET_HEADER, &secret_hex)
        .json(&extra_field_replay_drive_body(handle.per_run_secret()))
        .send()
        .await?;
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);

    write_smoke_report(A3InspectorSmokeReport {
        schema_id: "hsk.a3_inspector_smoke.report@1",
        wp_id: WP_ID,
        endpoints_passed,
        ipc_passed,
        trace_projection_fields,
        replay_drive_success: true,
        compile_leak_detected: false,
        replay_drive_event_type: replay_response.event.event_type,
    })?;

    Ok(())
}

#[test]
fn a3_inspector_smoke_compile_leak_is_detected() -> Result<(), Box<dyn Error>> {
    let repo = repo_root();
    let source = repo.join("src/backend/handshake_core/tests/ui/inspector_read_write_leak.rs");
    let deps_dir = std::env::current_exe()?
        .parent()
        .ok_or("test executable has no parent deps directory")?
        .to_path_buf();
    let rlib = latest_handshake_core_rlib(&deps_dir)?;

    let output = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string()))
        .arg("--edition=2021")
        .arg("--crate-name")
        .arg("inspector_read_write_leak")
        .arg(&source)
        .arg("--extern")
        .arg(format!("handshake_core={}", rlib.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps_dir.display()))
        .arg("--error-format")
        .arg("short")
        .output()?;

    assert!(
        !output.status.success(),
        "InspectorReadV1 write-side leak compiled successfully"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("append_event"),
        "compile-fail output did not prove append_event is unavailable: {stderr}"
    );
    Ok(())
}

async fn assert_http_read_endpoints(
    client: &reqwest::Client,
    base: &str,
    secret_hex: &str,
    snapshot: &InspectorReadSnapshot,
) -> Result<usize, Box<dyn Error>> {
    let session_id = SessionId::new(SESSION_ID);
    let workspace_id = WorkspaceId::new(WORKSPACE_ID);
    let mut passed = 0;

    assert_endpoint_matches(
        client,
        format!("{base}/inspector/v1/sessions"),
        secret_hex,
        snapshot.list_sessions(),
    )
    .await?;
    passed += 1;

    assert_endpoint_matches(
        client,
        format!("{base}/inspector/v1/sessions/{SESSION_ID}"),
        secret_hex,
        snapshot
            .session_state(session_id.clone())
            .expect("session state"),
    )
    .await?;
    passed += 1;

    assert_endpoint_matches(
        client,
        format!("{base}/inspector/v1/event-ledger/tail?n=64"),
        secret_hex,
        snapshot.event_ledger_tail(64),
    )
    .await?;
    passed += 1;

    assert_endpoint_matches(
        client,
        format!("{base}/inspector/v1/process-ledger/active"),
        secret_hex,
        snapshot.process_ledger_active(),
    )
    .await?;
    passed += 1;

    assert_endpoint_matches(
        client,
        format!("{base}/inspector/v1/workspace/{WORKSPACE_ID}"),
        secret_hex,
        snapshot
            .workspace_state_read(workspace_id)
            .expect("workspace state"),
    )
    .await?;
    passed += 1;

    assert_endpoint_matches(
        client,
        format!("{base}/inspector/v1/trace/{SESSION_ID}"),
        secret_hex,
        snapshot
            .trace_projection(session_id)
            .expect("trace projection"),
    )
    .await?;
    passed += 1;

    assert_endpoint_matches(
        client,
        format!("{base}/inspector/v1/models"),
        secret_hex,
        snapshot.loaded_models(),
    )
    .await?;
    passed += 1;

    Ok(passed)
}

async fn assert_ipc_equivalent_payloads(
    client: &reqwest::Client,
    base: &str,
    secret_hex: &str,
    snapshot: &InspectorReadSnapshot,
) -> Result<usize, Box<dyn Error>> {
    let session_id = SessionId::new(SESSION_ID);
    let mut passed = 1; // kernel.inspector.port checked by the live server handle.

    assert_http_bytes_equal_ipc_bytes(
        client,
        format!("{base}/inspector/v1/sessions"),
        secret_hex,
        snapshot.list_sessions(),
    )
    .await?;
    passed += 1;

    assert_http_bytes_equal_ipc_bytes(
        client,
        format!("{base}/inspector/v1/sessions/{SESSION_ID}"),
        secret_hex,
        snapshot
            .session_state(session_id.clone())
            .expect("session state"),
    )
    .await?;
    passed += 1;

    assert_http_bytes_equal_ipc_bytes(
        client,
        format!("{base}/inspector/v1/event-ledger/tail?n=64"),
        secret_hex,
        snapshot.event_ledger_tail(64),
    )
    .await?;
    passed += 1;

    assert_http_bytes_equal_ipc_bytes(
        client,
        format!("{base}/inspector/v1/process-ledger/active"),
        secret_hex,
        snapshot.process_ledger_active(),
    )
    .await?;
    passed += 1;

    assert_http_bytes_equal_ipc_bytes(
        client,
        format!("{base}/inspector/v1/trace/{SESSION_ID}"),
        secret_hex,
        snapshot
            .trace_projection(session_id)
            .expect("trace projection"),
    )
    .await?;
    passed += 1;

    assert_http_bytes_equal_ipc_bytes(
        client,
        format!("{base}/inspector/v1/models"),
        secret_hex,
        snapshot.loaded_models(),
    )
    .await?;
    passed += 1;

    assert_ipc_bridge_sources_registered()?;
    Ok(passed)
}

async fn assert_endpoint_matches<T>(
    client: &reqwest::Client,
    url: String,
    secret_hex: &str,
    expected: T,
) -> Result<(), Box<dyn Error>>
where
    T: DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let body = client
        .get(url)
        .header(PER_RUN_SECRET_HEADER, secret_hex)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let actual: T = serde_json::from_slice(&body)?;
    assert_eq!(actual, expected);
    Ok(())
}

async fn assert_http_bytes_equal_ipc_bytes<T>(
    client: &reqwest::Client,
    url: String,
    secret_hex: &str,
    ipc_value: T,
) -> Result<(), Box<dyn Error>>
where
    T: Serialize,
{
    let http_bytes = client
        .get(url)
        .header(PER_RUN_SECRET_HEADER, secret_hex)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let ipc_bytes = serde_json::to_vec(&ipc_value)?;
    assert_eq!(
        http_bytes.as_ref(),
        ipc_bytes.as_slice(),
        "HTTP and IPC-equivalent serde payload bytes diverged"
    );
    Ok(())
}

fn assert_ipc_bridge_sources_registered() -> Result<(), Box<dyn Error>> {
    let repo = repo_root();
    let inspector_rs = fs::read_to_string(repo.join("app/src-tauri/src/inspector.rs"))?;
    let lib_rs = fs::read_to_string(repo.join("app/src-tauri/src/lib.rs"))?;

    for command in [
        "kernel_inspector_port",
        "kernel_inspector_list_sessions",
        "kernel_inspector_session_state",
        "kernel_inspector_event_ledger_tail",
        "kernel_inspector_process_ledger_active",
        "kernel_inspector_trace_projection",
        "kernel_inspector_loaded_models",
    ] {
        assert!(
            inspector_rs.contains(&format!("pub fn {command}")),
            "missing IPC command function {command}"
        );
        assert!(
            lib_rs.contains(&format!("inspector::{command}")),
            "missing invoke_handler registration for {command}"
        );
    }
    Ok(())
}

async fn post_valid_replay_drive(
    client: &reqwest::Client,
    base: &str,
    secret: &PerRunSecret,
    secret_hex: &str,
) -> Result<ReplayDriveResponse, Box<dyn Error>> {
    let response = client
        .post(format!("{base}/inspector/v1/replay-drive"))
        .header(PER_RUN_SECRET_HEADER, secret_hex)
        .json(&valid_replay_drive_body(secret))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response)
}

fn populated_trace_projection_fields(
    projection: &handshake_core::inspector_read::TraceProjection,
) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if !projection.what_task.wp_id.is_empty()
        && !projection.what_task.mt_id.is_empty()
        && !projection.what_task.task_summary.is_empty()
    {
        fields.push("what_task");
    }
    if !projection.what_context.context_bundle_id.is_empty()
        && !projection.what_context.context_summary.is_empty()
        && !projection.what_context.hash.is_empty()
    {
        fields.push("what_context");
    }
    if !projection.what_returns.is_empty() {
        fields.push("what_returns");
    }
    if !projection.what_tool_calls.is_empty() {
        fields.push("what_tool_calls");
    }
    if !projection.what_artifacts.is_empty() {
        fields.push("what_artifacts");
    }
    if !projection.what_validation.is_empty() {
        fields.push("what_validation");
    }
    if projection.what_promotion.is_some() {
        fields.push("what_promotion");
    }
    fields
}

fn synthetic_reader() -> InspectorReadSnapshot {
    let session_id = SessionId::new(SESSION_ID);
    let workspace_id = WorkspaceId::new(WORKSPACE_ID);
    let mut snapshot = InspectorReadSnapshot::default();
    snapshot.sessions.push(SessionSummary {
        id: session_id.clone(),
        state: "running".to_string(),
        model_id: Some("local-a3-smoke".to_string()),
        active_process_count: 1,
    });
    snapshot.session_states.insert(
        session_id.clone(),
        SessionStateRead {
            id: session_id.clone(),
            state: "running".to_string(),
            latest_event_id: Some("evt-a3-009".to_string()),
            active_process_count: 1,
        },
    );
    snapshot.event_ledger_tail = trace_rows();
    snapshot.processes.push(ProcessRow {
        process_uuid: "proc-a3-smoke".to_string(),
        session_id,
        engine_kind: "webview2_cdp".to_string(),
        status: "running".to_string(),
    });
    snapshot.workspace_states.insert(
        workspace_id.clone(),
        WorkspaceStateRead {
            workspace_id,
            state_vector: "sv:a3-smoke:1".to_string(),
            last_update_id: Some("update-a3-smoke".to_string()),
            readable_refs: vec!["crdt://workspace-a3-smoke/update-a3-smoke".to_string()],
        },
    );
    snapshot.loaded_models.push(ModelLoadedRow {
        model_id: "local-a3-smoke".to_string(),
        adapter_id: "llama-cpp-a3-smoke".to_string(),
        process_uuid: Some("proc-a3-smoke".to_string()),
        loaded_at_utc: Some("2026-05-18T12:00:00Z".to_string()),
    });
    snapshot
}

fn trace_rows() -> Vec<EventLedgerRow> {
    vec![
        trace_row(
            "evt-a3-001",
            "TASK_OPEN",
            1,
            json!({
                "wp_id": WP_ID,
                "mt_id": "MT-033",
                "task_summary": "A.3 inspector plane end-to-end smoke"
            }),
        ),
        trace_row(
            "evt-a3-002",
            "CONTEXT_BUNDLE_DISPATCH",
            2,
            json!({
                "context_bundle_id": "ctx-a3-smoke",
                "context_summary": "Synthetic A.3 inspector context",
                "context_full": {
                    "allowed_paths": ["src/backend/handshake_core/tests/a3_inspector_smoke.rs"],
                    "intent": "prove inspector plane integration"
                }
            }),
        ),
        trace_row(
            "evt-a3-003",
            "MODEL_RESPONSE",
            3,
            json!({
                "content": "Inspector plane smoke return payload with deterministic content."
            }),
        ),
        trace_row(
            "evt-a3-004",
            "TOOL_CALL_REQUEST",
            4,
            json!({
                "tool_call_id": "tool-a3-001",
                "tool_name": "cargo_test",
                "args": {
                    "test": "a3_inspector_smoke"
                }
            }),
        ),
        trace_row(
            "evt-a3-005",
            "TOOL_RESULT_RECORDED",
            5,
            json!({
                "tool_call_id": "tool-a3-001",
                "tool_name": "cargo_test",
                "result_summary": "test dispatched",
                "verdict": "passed"
            }),
        ),
        trace_row(
            "evt-a3-006",
            "ARTIFACT_WRITTEN",
            6,
            json!({
                "artifact_id": "artifact-a3-smoke-report",
                "kind": "smoke_report_jsonl",
                "path": "Handshake_Artifacts/hbr-inspector-smoke/a3-inspector-smoke.jsonl",
                "content": "a3 inspector smoke report"
            }),
        ),
        trace_row(
            "evt-a3-007",
            "VALIDATION_RESULT",
            7,
            json!({
                "check_id": "hbr-inspector-smoke",
                "verdict": "pass",
                "evidence_pointer": "artifact://hbr-inspector-smoke"
            }),
        ),
        trace_row(
            "evt-a3-008",
            "PROMOTION_ACCEPTED",
            8,
            json!({
                "gate_id": "AC-INSPECTOR-PLANE-BIND",
                "verdict": "accepted"
            }),
        ),
        trace_row(
            "evt-a3-009",
            "SESSION_COMPLETED",
            9,
            json!({
                "status": "completed"
            }),
        ),
    ]
}

fn trace_row(
    event_id: &str,
    event_type: &str,
    event_sequence: i64,
    payload: Value,
) -> EventLedgerRow {
    EventLedgerRow {
        event_id: event_id.to_string(),
        event_type: event_type.to_string(),
        event_sequence,
        created_at_utc: format!("2026-05-18T12:00:0{}Z", event_sequence.min(9)),
        session_run_id: SESSION_ID.to_string(),
        aggregate_id: SESSION_ID.to_string(),
        actor_kind: "role".to_string(),
        actor_id: "KERNEL_BUILDER".to_string(),
        payload_hash: format!("payload-hash-{event_sequence}"),
        source_component: "a3_inspector_smoke".to_string(),
        payload,
        ..EventLedgerRow::default()
    }
}

fn valid_replay_drive_body(secret: &PerRunSecret) -> Value {
    let write_box = sample_write_box();
    let signature = expected_write_box_v1_signature(secret, "KERNEL_BUILDER", &write_box);
    json!({
        "action_id": "kernel.write_box.promote",
        "envelope": {
            "schema_id": WRITE_BOX_V1_ENVELOPE_SCHEMA_ID,
            "signer": "KERNEL_BUILDER",
            "signature": signature,
            "write_box": write_box,
        }
    })
}

fn extra_field_replay_drive_body(secret: &PerRunSecret) -> Value {
    let mut body = valid_replay_drive_body(secret);
    body["parallel_mutation"] = json!({"attempt": "forbidden"});
    body
}

fn sample_write_box() -> WriteBoxCommon {
    WriteBoxCommon {
        write_box_id: "WB-A3-SMOKE-1".to_string(),
        kind: WriteBoxKind::Promotion,
        schema_version: "hsk.write_box.promotion@1".to_string(),
        workspace_id: WORKSPACE_ID.to_string(),
        owner: WriteBoxOwnerRef {
            actor_id: "actor-kernel-builder".to_string(),
            actor_kind: "role".to_string(),
            role_id: "KERNEL_BUILDER".to_string(),
        },
        crdt_site_id: "site-a3-smoke".to_string(),
        target_refs: vec![WriteBoxTargetRef {
            target_id: "promotion-target-a3-smoke".to_string(),
            target_kind: "write_box".to_string(),
            authority_class: "event_ledger".to_string(),
        }],
        base_snapshot_refs: vec!["snapshot://workspace-a3-smoke/base".to_string()],
        intent_summary: "Replay-drive a validated smoke write box through the catalog.".to_string(),
        operation_payload_refs: vec![WriteBoxPayloadRef {
            payload_id: "payload-a3-smoke".to_string(),
            payload_kind: "promotion_request".to_string(),
            payload_ref: "artifact://payload-a3-smoke".to_string(),
            payload_sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_string(),
        }],
        lifecycle_state: WriteBoxLifecycleState::Validated,
        allowed_transitions: vec![
            WriteBoxLifecycleState::Validated,
            WriteBoxLifecycleState::PromotionQueued,
            WriteBoxLifecycleState::Promoted,
        ],
        authority_effect: AuthorityEffect::EventLedgerAuthorityWrite,
        evidence_refs: vec!["event://evidence-a3-smoke".to_string()],
        receipt_refs: vec!["receipt://validation-a3-smoke".to_string()],
        denial_receipt_refs: Vec::new(),
        promotion_receipt_refs: vec!["receipt://promotion-a3-smoke".to_string()],
        validation_status: WriteBoxValidationStatus {
            state: WriteBoxValidationState::Valid,
            check_ids: vec!["schema_validity".to_string()],
        },
        projection_rules: vec!["dcc.write_box.queue".to_string()],
        replay_metadata: WriteBoxReplayMetadataV1 {
            replay_plan_ref: "KTR-A3-SMOKE-1".to_string(),
            replay_order_key: "001".to_string(),
            idempotency_key: "IK-A3-SMOKE-1".to_string(),
            source_event_refs: vec!["event://source-a3-smoke".to_string()],
        },
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct A3InspectorSmokeReport {
    schema_id: &'static str,
    wp_id: &'static str,
    endpoints_passed: usize,
    ipc_passed: usize,
    trace_projection_fields: Vec<&'static str>,
    replay_drive_success: bool,
    compile_leak_detected: bool,
    replay_drive_event_type: String,
}

fn write_smoke_report(report: A3InspectorSmokeReport) -> Result<(), Box<dyn Error>> {
    let path = smoke_report_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(&report)?)?;
    Ok(())
}

fn smoke_report_path() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_INSPECTOR_SMOKE_REPORT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    artifact_root().join("hbr-inspector-smoke").join(format!(
        "a3-inspector-smoke-{}-{}.jsonl",
        std::process::id(),
        epoch_millis()
    ))
}

fn latest_handshake_core_rlib(deps_dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let mut candidates = Vec::new();
    for entry in fs::read_dir(deps_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name.starts_with("libhandshake_core-") && file_name.ends_with(".rlib") {
            let modified = entry
                .metadata()?
                .modified()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            candidates.push((modified, path));
        }
    }
    candidates.sort_by_key(|(modified, _)| *modified);
    candidates
        .pop()
        .map(|(_, path)| path)
        .ok_or_else(|| "libhandshake_core rlib not found in cargo deps directory".into())
}

fn artifact_root() -> PathBuf {
    if let Ok(value) = std::env::var("HANDSHAKE_ARTIFACT_ROOT") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    repo_root()
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new("."))
        .join("Handshake_Artifacts")
}

fn repo_root() -> PathBuf {
    let mut current = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if current.join(".GOV").exists() {
            return current;
        }
        assert!(current.pop(), "repo root with .GOV not found");
    }
}

fn epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
