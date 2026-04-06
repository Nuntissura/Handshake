use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use handshake_core::{
    ace::ArtifactHandle,
    api::role_mailbox as role_mailbox_api,
    bundles::zip::sha256_hex,
    capabilities::CapabilityRegistry,
    flight_recorder::{
        duckdb::DuckDbFlightRecorder, EventFilter, FlightRecorder, FlightRecorderEventType,
    },
    llm::{CompletionRequest, CompletionResponse, LlmClient, LlmError, ModelProfile, TokenUsage},
    role_mailbox::{
        CreateRoleMailboxMessageRequest, GovernanceMode, RoleId, RoleMailbox, RoleMailboxContext,
        RoleMailboxAnnounceBackMessage, RoleMailboxAnnounceBackStatus, RoleMailboxMessageType,
        TranscriptionLink, TranscriptionTargetKind,
    },
    runtime_governance::RuntimeGovernancePaths,
    storage::{sqlite::SqliteDatabase, Database},
    workflows::locus::{
        validate_structured_collaboration_record, StructuredCollaborationRecordFamily,
        StructuredCollaborationValidationCode, StructuredCollaborationValidationResult,
    },
    workflows::{SessionRegistry, SessionSchedulerConfig},
    AppState,
};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use tempfile::tempdir;
use uuid::Uuid;

static TEST_SERIAL_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

struct WorkspaceEnvGuard {
    prev_workspace_root: Option<String>,
    prev_governance_root: Option<String>,
}

impl WorkspaceEnvGuard {
    fn activate(root: &Path) -> Self {
        let prev_workspace_root = std::env::var("HANDSHAKE_WORKSPACE_ROOT").ok();
        let prev_governance_root = std::env::var("HANDSHAKE_GOVERNANCE_ROOT").ok();
        std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", root);
        std::env::set_var("HANDSHAKE_GOVERNANCE_ROOT", ".handshake/gov");
        Self {
            prev_workspace_root,
            prev_governance_root,
        }
    }
}

impl Drop for WorkspaceEnvGuard {
    fn drop(&mut self) {
        match &self.prev_workspace_root {
            Some(value) => std::env::set_var("HANDSHAKE_WORKSPACE_ROOT", value),
            None => std::env::remove_var("HANDSHAKE_WORKSPACE_ROOT"),
        }
        match &self.prev_governance_root {
            Some(value) => std::env::set_var("HANDSHAKE_GOVERNANCE_ROOT", value),
            None => std::env::remove_var("HANDSHAKE_GOVERNANCE_ROOT"),
        }
    }
}

fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_SERIAL_LOCK.lock().expect("test serial mutex poisoned")
}

async fn setup_api_state(
    recorder: Arc<DuckDbFlightRecorder>,
) -> Result<AppState, Box<dyn std::error::Error>> {
    let sqlite = SqliteDatabase::connect("sqlite::memory:", 5).await?;
    sqlite.run_migrations().await?;
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();

    Ok(AppState {
        storage: sqlite.into_arc(),
        flight_recorder: flight_recorder.clone(),
        diagnostics: recorder,
        llm_client: Arc::new(TestLlmClient::new()),
        capability_registry: Arc::new(CapabilityRegistry::new()),
        session_registry: Arc::new(SessionRegistry::new(SessionSchedulerConfig::default())),
    })
}

async fn start_role_mailbox_api_server(
    state: AppState,
) -> Result<(String, tokio::task::JoinHandle<()>), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = role_mailbox_api::routes(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("role mailbox api server");
    });
    Ok((format!("http://{addr}"), server))
}

struct TestLlmClient {
    profile: ModelProfile,
}

impl TestLlmClient {
    fn new() -> Self {
        Self {
            profile: ModelProfile::new("role-mailbox-test-model".to_string(), 4096),
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for TestLlmClient {
    async fn completion(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            text: "ok".to_string(),
            usage: TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            },
            latency_ms: 0,
        })
    }

    async fn swap_model(
        &self,
        _req: handshake_core::workflows::ModelSwapRequestV0_4,
    ) -> Result<(), LlmError> {
        Ok(())
    }

    fn profile(&self) -> &ModelProfile {
        &self.profile
    }
}

fn dummy_artifact(path: &str) -> ArtifactHandle {
    ArtifactHandle::new(Uuid::new_v4(), path.to_string())
}

fn test_context() -> RoleMailboxContext {
    RoleMailboxContext {
        spec_id: Some("spec-1".to_string()),
        work_packet_id: Some("WP-1-Role-Mailbox-v1".to_string()),
        task_board_id: Some(".handshake/gov/TASK_BOARD.md".to_string()),
        governance_mode: GovernanceMode::GovStandard,
        project_id: None,
    }
}

fn validate_runtime_mailbox_record(
    root: &std::path::Path,
    family: StructuredCollaborationRecordFamily,
    value: &Value,
) -> StructuredCollaborationValidationResult {
    let runtime_paths = RuntimeGovernancePaths::from_workspace_root(root.to_path_buf()).unwrap();
    let mut validation = validate_structured_collaboration_record(family, value);
    let authority_refs = value
        .get("authority_refs")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let invalid_refs = runtime_paths.invalid_runtime_authority_refs(&authority_refs);
    if !invalid_refs.is_empty() {
        validation.push_issue(
            StructuredCollaborationValidationCode::AuthorityScopeMismatch,
            "authority_refs",
            Some(runtime_paths.governance_root_display()),
            Some(invalid_refs.join(",")),
            "authority_refs must stay within the product-runtime .handshake/gov boundary",
        );
    }
    validation
}

#[tokio::test]
async fn role_mailbox_export_empty_is_deterministic() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("data")).unwrap();
    let db_path = root.join("data").join("flight_recorder.db");

    let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&db_path, 7).unwrap());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();

    let mailbox = RoleMailbox::new_for_root(root.clone(), flight_recorder).unwrap();
    let ctx = test_context();

    mailbox
        .export_repo(&ctx, "operator".to_string())
        .await
        .unwrap();

    let index_path = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("index.json");
    let manifest_path = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("export_manifest.json");

    let index_1 = fs::read(&index_path).unwrap();
    let manifest_1 = fs::read(&manifest_path).unwrap();

    mailbox
        .export_repo(&ctx, "operator".to_string())
        .await
        .unwrap();

    let index_2 = fs::read(&index_path).unwrap();
    let manifest_2 = fs::read(&manifest_path).unwrap();

    assert_eq!(index_1, index_2);
    assert_eq!(manifest_1, manifest_2);

    let manifest_json: Value = serde_json::from_slice(&manifest_1).unwrap();
    let index_sha = manifest_json
        .get("index_sha256")
        .and_then(|v| v.as_str())
        .unwrap();
    assert_eq!(index_sha, sha256_hex(&index_1));

    let index_json: Value = serde_json::from_slice(&index_1).unwrap();
    assert_eq!(
        index_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.role_mailbox_index@1")
    );
    assert_eq!(
        index_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert!(
        index_json
            .get("profile_extension")
            .is_some_and(Value::is_null)
    );
    assert_eq!(
        index_json.get("schema_version").and_then(Value::as_str),
        Some("role_mailbox_export_v1")
    );
    assert_eq!(
        manifest_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert!(
        manifest_json
            .get("profile_extension")
            .is_some_and(Value::is_null)
    );
    assert_eq!(
        index_json
            .get("authority_refs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some(".handshake/gov/ROLE_MAILBOX/index.json")
    );
    let validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxIndex,
        &index_json,
    );
    assert!(validation.ok, "{validation:?}");
}

#[tokio::test]
async fn role_mailbox_create_message_emits_events_and_export() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("data")).unwrap();
    let db_path = root.join("data").join("flight_recorder.db");

    let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&db_path, 7).unwrap());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mailbox = RoleMailbox::new_for_root(root.clone(), flight_recorder).unwrap();

    let ctx = test_context();
    let body = "secret body content";
    let body_sha = sha256_hex(body.as_bytes());

    let note = "Recorded in GOV/task_packets/WP-1-Role-Mailbox-v1.md (password=abc)";
    let note_sha = sha256_hex(note.as_bytes());

    let request = CreateRoleMailboxMessageRequest {
        thread_id: None,
        thread_subject: Some("Subject password=123".to_string()),
        thread_participants: Some(vec![RoleId::Operator, RoleId::Coder]),
        context: ctx.clone(),
        from_role: RoleId::Operator,
        to_roles: vec![RoleId::Coder],
        message_type: RoleMailboxMessageType::ValidationFinding,
        body: body.to_string(),
        attachments: Vec::new(),
        relates_to_message_id: None,
        transcription_links: vec![TranscriptionLink {
            target_kind: TranscriptionTargetKind::TaskPacket,
            target_ref: dummy_artifact("/GOV/task_packets/WP-1-Role-Mailbox-v1.md"),
            target_sha256: "0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            note: note.to_string(),
        }],
        idempotency_key: "idempotency-1".to_string(),
    };

    let created = mailbox.create_message(request).await.unwrap();
    assert_eq!(created.body_sha256, body_sha);

    let thread_file = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("threads")
        .join(format!("{}.jsonl", created.thread_id));
    let thread_text = fs::read_to_string(&thread_file).unwrap();
    assert!(!thread_text.contains(body));

    let first_line = thread_text.lines().next().unwrap();
    let line_json: Value = serde_json::from_str(first_line).unwrap();
    assert_eq!(
        line_json
            .get("body_sha256")
            .and_then(|v| v.as_str())
            .unwrap(),
        body_sha
    );
    assert_eq!(
        line_json.get("schema_id").and_then(Value::as_str),
        Some("hsk.role_mailbox_thread_line@1")
    );
    assert_eq!(
        line_json.get("schema_version").and_then(Value::as_str),
        Some("role_mailbox_export_v1")
    );
    assert_eq!(
        line_json.get("record_kind").and_then(Value::as_str),
        Some("role_mailbox_message")
    );
    assert_eq!(
        line_json
            .get("project_profile_kind")
            .and_then(Value::as_str),
        Some("generic")
    );
    assert!(
        line_json
            .get("profile_extension")
            .is_some_and(Value::is_null)
    );
    assert_eq!(
        line_json
            .get("authority_refs")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(Value::as_str),
        Some(".handshake/gov/ROLE_MAILBOX/index.json")
    );
    let expected_thread_ref = format!(
        ".handshake/gov/ROLE_MAILBOX/threads/{}.jsonl",
        created.thread_id
    );
    let evidence_refs = line_json
        .get("evidence_refs")
        .and_then(Value::as_array)
        .expect("evidence refs");
    assert!(evidence_refs
        .iter()
        .any(|value| { value.as_str() == Some(expected_thread_ref.as_str()) }));
    let validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine,
        &line_json,
    );
    assert!(validation.ok, "{validation:?}");

    let index_path = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("index.json");
    let index_json: Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    let index_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxIndex,
        &index_json,
    );
    assert!(index_validation.ok, "{index_validation:?}");

    let links = line_json
        .get("transcription_links")
        .and_then(|v| v.as_array())
        .unwrap();
    assert_eq!(links.len(), 1);
    let link0 = links[0].as_object().unwrap();
    assert_eq!(
        link0.get("note_sha256").and_then(|v| v.as_str()).unwrap(),
        note_sha
    );
    let note_redacted = link0.get("note_redacted").and_then(|v| v.as_str()).unwrap();
    assert!(!note_redacted.contains("abc"));

    let events = recorder.list_events(EventFilter::default()).await.unwrap();
    assert!(events.iter().any(|e| matches!(
        e.event_type,
        FlightRecorderEventType::GovMailboxMessageCreated
    )));
    assert!(events
        .iter()
        .any(|e| matches!(e.event_type, FlightRecorderEventType::GovMailboxExported)));

    let conn = recorder.connection();
    let conn = conn.lock().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM spec_session_log_entries WHERE event_type = 'mailbox_exported'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

#[tokio::test]
async fn role_mailbox_validation_reports_schema_and_authority_drift() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("data")).unwrap();
    let db_path = root.join("data").join("flight_recorder.db");

    let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&db_path, 7).unwrap());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mailbox = RoleMailbox::new_for_root(root.clone(), flight_recorder).unwrap();

    let created = mailbox
        .create_message(CreateRoleMailboxMessageRequest {
            thread_id: None,
            thread_subject: Some("Validation drift".to_string()),
            thread_participants: Some(vec![RoleId::Operator, RoleId::Coder]),
            context: test_context(),
            from_role: RoleId::Operator,
            to_roles: vec![RoleId::Coder],
            message_type: RoleMailboxMessageType::FYI,
            body: "hello".to_string(),
            attachments: Vec::new(),
            relates_to_message_id: None,
            transcription_links: Vec::new(),
            idempotency_key: "idempotency-validation-drift".to_string(),
        })
        .await
        .unwrap();

    let index_path = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("index.json");
    let thread_path = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("threads")
        .join(format!("{}.jsonl", created.thread_id));

    let mut index_json: Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    index_json["authority_refs"] = json!([".GOV/roles_shared/ROLE_MAILBOX/index.json"]);
    let index_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxIndex,
        &index_json,
    );
    assert!(!index_validation.ok);
    assert!(index_validation.issues.iter().any(|issue| {
        matches!(
            issue.code,
            StructuredCollaborationValidationCode::AuthorityScopeMismatch
        )
    }));

    let thread_text = fs::read_to_string(&thread_path).unwrap();
    let first_line = thread_text.lines().next().unwrap();
    let base_line_json: Value = serde_json::from_str(first_line).unwrap();

    let mut line_schema_id_json = base_line_json.clone();
    line_schema_id_json["schema_id"] =
        Value::String("hsk.role_mailbox_thread_line@999".to_string());
    let line_schema_id_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine,
        &line_schema_id_json,
    );
    assert!(!line_schema_id_validation.ok);
    assert!(line_schema_id_validation.issues.iter().any(|issue| {
        matches!(
            issue.code,
            StructuredCollaborationValidationCode::UnknownSchemaId
        ) && issue.field == "schema_id"
    }));

    let mut line_schema_version_json = base_line_json;
    line_schema_version_json["schema_version"] =
        Value::String("role_mailbox_export_v0".to_string());
    let line_schema_version_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine,
        &line_schema_version_json,
    );
    assert!(!line_schema_version_validation.ok);
    assert!(line_schema_version_validation.issues.iter().any(|issue| {
        matches!(
            issue.code,
            StructuredCollaborationValidationCode::SchemaVersionMismatch
        ) && issue.field == "schema_version"
    }));

    let mut line_unknown_profile_extension_json = line_schema_version_json;
    line_unknown_profile_extension_json["schema_version"] =
        Value::String("role_mailbox_export_v1".to_string());
    line_unknown_profile_extension_json["profile_extension"] = json!({
        "extension_schema_id": "hsk.profile.unknown@1",
        "extension_schema_version": "1",
        "compatibility": {
            "breaking": false,
        },
    });
    let line_unknown_profile_extension_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine,
        &line_unknown_profile_extension_json,
    );
    assert!(!line_unknown_profile_extension_validation.ok);
    assert!(line_unknown_profile_extension_validation
        .issues
        .iter()
        .any(|issue| {
            matches!(
                issue.code,
                StructuredCollaborationValidationCode::InvalidFieldValue
            ) && issue.field == "profile_extension.extension_schema_id"
        }));
}

#[tokio::test]
async fn role_mailbox_validation_reports_redacted_field_drift() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("data")).unwrap();
    let db_path = root.join("data").join("flight_recorder.db");

    let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&db_path, 7).unwrap());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mailbox = RoleMailbox::new_for_root(root.clone(), flight_recorder).unwrap();

    let created = mailbox
        .create_message(CreateRoleMailboxMessageRequest {
            thread_id: None,
            thread_subject: Some("Subject password=123".to_string()),
            thread_participants: Some(vec![RoleId::Operator, RoleId::Coder]),
            context: test_context(),
            from_role: RoleId::Operator,
            to_roles: vec![RoleId::Coder],
            message_type: RoleMailboxMessageType::ValidationFinding,
            body: "hello".to_string(),
            attachments: Vec::new(),
            relates_to_message_id: None,
            transcription_links: vec![TranscriptionLink {
                target_kind: TranscriptionTargetKind::TaskPacket,
                target_ref: dummy_artifact("/GOV/task_packets/WP-1-Role-Mailbox-v1.md"),
                target_sha256:
                    "0000000000000000000000000000000000000000000000000000000000000000"
                        .to_string(),
                note: "note password=abc".to_string(),
            }],
            idempotency_key: "idempotency-redacted-drift".to_string(),
        })
        .await
        .unwrap();

    let index_path = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("index.json");
    let thread_path = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("threads")
        .join(format!("{}.jsonl", created.thread_id));

    let index_json: Value = serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    let original_subject = index_json["threads"][0]["subject_redacted"]
        .as_str()
        .unwrap()
        .to_string();

    let mut single_line_index_json = index_json.clone();
    single_line_index_json["threads"][0]["subject_redacted"] =
        Value::String(format!("{original_subject} password=123"));
    let single_line_index_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxIndex,
        &single_line_index_json,
    );
    assert!(!single_line_index_validation.ok);
    assert!(single_line_index_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "threads[0].subject_redacted"
    }));

    let mut multiline_index_json = index_json.clone();
    multiline_index_json["threads"][0]["subject_redacted"] = Value::String(
        "Subject password=123\nsecond line".to_string(),
    );
    let index_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxIndex,
        &multiline_index_json,
    );
    assert!(!index_validation.ok);
    assert!(index_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "threads[0].subject_redacted"
    }));

    let thread_text = fs::read_to_string(&thread_path).unwrap();
    let first_line = thread_text.lines().next().unwrap();
    let line_json: Value = serde_json::from_str(first_line).unwrap();
    let original_note = line_json["transcription_links"][0]["note_redacted"]
        .as_str()
        .unwrap()
        .to_string();

    let mut single_line_line_json = line_json.clone();
    single_line_line_json["transcription_links"][0]["note_redacted"] =
        Value::String(format!("{original_note} password=abc"));
    let single_line_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine,
        &single_line_line_json,
    );
    assert!(!single_line_validation.ok);
    assert!(single_line_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "transcription_links[0].note_redacted"
    }));

    let mut multiline_line_json = line_json.clone();
    multiline_line_json["transcription_links"][0]["note_redacted"] = Value::String(
        "note password=abc\nsecond line".to_string(),
    );
    let line_validation = validate_runtime_mailbox_record(
        &root,
        StructuredCollaborationRecordFamily::RoleMailboxThreadLine,
        &multiline_line_json,
    );
    assert!(!line_validation.ok);
    assert!(line_validation.issues.iter().any(|issue| {
        issue.code == StructuredCollaborationValidationCode::InvalidFieldValue
            && issue.field == "transcription_links[0].note_redacted"
    }));
}

#[tokio::test]
async fn role_mailbox_idempotency_key_is_deduped() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("data")).unwrap();
    let db_path = root.join("data").join("flight_recorder.db");

    let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&db_path, 7).unwrap());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mailbox = RoleMailbox::new_for_root(root.clone(), flight_recorder).unwrap();

    let ctx = test_context();

    let request = CreateRoleMailboxMessageRequest {
        thread_id: None,
        thread_subject: Some("Idempotency test".to_string()),
        thread_participants: Some(vec![RoleId::Operator, RoleId::Coder]),
        context: ctx.clone(),
        from_role: RoleId::Operator,
        to_roles: vec![RoleId::Coder],
        message_type: RoleMailboxMessageType::FYI,
        body: "hello".to_string(),
        attachments: Vec::new(),
        relates_to_message_id: None,
        transcription_links: Vec::new(),
        idempotency_key: "idempotency-dup".to_string(),
    };

    let first = mailbox.create_message(request.clone()).await.unwrap();
    let events_1 = recorder
        .list_events(EventFilter::default())
        .await
        .unwrap()
        .len();

    let second = mailbox.create_message(request).await.unwrap();
    let events_2 = recorder
        .list_events(EventFilter::default())
        .await
        .unwrap()
        .len();

    assert_eq!(first.message_id, second.message_id);
    assert_eq!(events_1, events_2);

    let thread_file = root
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("threads")
        .join(format!("{}.jsonl", first.thread_id));
    let lines = fs::read_to_string(&thread_file).unwrap().lines().count();
    assert_eq!(lines, 1);
}

#[tokio::test]
async fn role_mailbox_create_announce_back_message_carries_spawn_fields() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    fs::create_dir_all(root.join("data")).unwrap();
    let db_path = root.join("data").join("flight_recorder.db");

    let recorder = Arc::new(DuckDbFlightRecorder::new_on_path(&db_path, 7).unwrap());
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mailbox = RoleMailbox::new_for_root(root.clone(), flight_recorder).unwrap();

    let child_session_id = "session-child-001".to_string();
    let requester_session_id = "session-parent-001".to_string();
    let correlation_id = "spawn-correlation-id-001".to_string();
    let payload = RoleMailboxAnnounceBackMessage {
        child_session_id: child_session_id.clone(),
        requester_session_id: requester_session_id.clone(),
        status: RoleMailboxAnnounceBackStatus::Completed,
        summary_artifact_id: Some(dummy_artifact("/artifacts/summary.tar.gz")),
        correlation_id: correlation_id.clone(),
    };
    let body = serde_json::to_string(&payload).unwrap();
    let expected_sha = sha256_hex(body.as_bytes());

    let created = mailbox
        .create_message(CreateRoleMailboxMessageRequest {
            thread_id: None,
            thread_subject: Some("Announce back test".to_string()),
            thread_participants: Some(vec![RoleId::Operator, RoleId::Coder]),
            context: test_context(),
            from_role: RoleId::Operator,
            to_roles: vec![RoleId::Coder],
            message_type: RoleMailboxMessageType::AnnounceBack,
            body,
            attachments: Vec::new(),
            relates_to_message_id: None,
            transcription_links: Vec::new(),
            idempotency_key: "announce-back-idempotent-1".to_string(),
        })
        .await
        .unwrap();

    assert_eq!(created.message_type, RoleMailboxMessageType::AnnounceBack);
    assert_eq!(created.body_sha256, expected_sha);

    let body_path = root.join(&created.body_ref.path);
    let stored_body_bytes = fs::read(&body_path).unwrap();
    let stored_payload: RoleMailboxAnnounceBackMessage =
        serde_json::from_slice(&stored_body_bytes).unwrap();

    assert_eq!(stored_payload.child_session_id, child_session_id);
    assert_eq!(stored_payload.requester_session_id, requester_session_id);
    assert_eq!(stored_payload.status, RoleMailboxAnnounceBackStatus::Completed);
    assert_eq!(stored_payload.summary_artifact_id, payload.summary_artifact_id);
    assert_eq!(stored_payload.correlation_id, correlation_id);
}

#[tokio::test]
async fn role_mailbox_index_api_returns_valid_structured_export(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mailbox = RoleMailbox::new_for_root(dir.path().to_path_buf(), flight_recorder.clone())?;
    mailbox
        .export_repo(&test_context(), "operator".to_string())
        .await?;

    let state = setup_api_state(recorder.clone()).await?;
    let (base_url, server) = start_role_mailbox_api_server(state).await?;
    let response = reqwest::get(format!("{base_url}/role_mailbox/index")).await?;
    server.abort();
    let _ = server.await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: Value = response.json().await?;
    assert_eq!(
        payload.get("schema_id").and_then(Value::as_str),
        Some("hsk.role_mailbox_index@1")
    );
    let validation = validate_runtime_mailbox_record(
        dir.path(),
        StructuredCollaborationRecordFamily::RoleMailboxIndex,
        &payload,
    );
    assert!(validation.ok, "{validation:?}");

    Ok(())
}

#[tokio::test]
async fn role_mailbox_index_api_rejects_invalid_structured_export(
) -> Result<(), Box<dyn std::error::Error>> {
    let _test_guard = test_guard();
    let dir = tempdir()?;
    let _env = WorkspaceEnvGuard::activate(dir.path());
    let recorder = Arc::new(DuckDbFlightRecorder::new_in_memory(7)?);
    let flight_recorder: Arc<dyn FlightRecorder> = recorder.clone();
    let mailbox = RoleMailbox::new_for_root(dir.path().to_path_buf(), flight_recorder.clone())?;
    mailbox
        .export_repo(&test_context(), "operator".to_string())
        .await?;

    let index_path = dir
        .path()
        .join(".handshake")
        .join("gov")
        .join("ROLE_MAILBOX")
        .join("index.json");
    let mut index_json: Value = serde_json::from_slice(&fs::read(&index_path)?)?;
    index_json["authority_refs"] = json!([".GOV/roles_shared/ROLE_MAILBOX/index.json"]);
    fs::write(&index_path, serde_json::to_vec_pretty(&index_json)?)?;

    let state = setup_api_state(recorder.clone()).await?;
    let (base_url, server) = start_role_mailbox_api_server(state).await?;
    let response = reqwest::get(format!("{base_url}/role_mailbox/index")).await?;
    let status = response.status();
    let body = response.text().await?;
    server.abort();
    let _ = server.await;

    assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
    let validation: Value = serde_json::from_str(&body)?;
    assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
    let issues = validation
        .get("issues")
        .and_then(Value::as_array)
        .expect("validation issues");
    assert!(issues.iter().any(|issue| {
        issue.get("code").and_then(Value::as_str) == Some("authority_scope_mismatch")
            && issue.get("field").and_then(Value::as_str) == Some("authority_refs")
    }));

    Ok(())
}
