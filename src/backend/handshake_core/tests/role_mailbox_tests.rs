use std::{fs, sync::Arc};

use handshake_core::{
    ace::ArtifactHandle,
    bundles::zip::sha256_hex,
    flight_recorder::{
        duckdb::DuckDbFlightRecorder, EventFilter, FlightRecorder, FlightRecorderEventType,
    },
    role_mailbox::{
        CreateRoleMailboxMessageRequest, GovernanceMode, RoleId, RoleMailbox, RoleMailboxContext,
        RoleMailboxMessageType, TranscriptionLink, TranscriptionTargetKind,
    },
};
use tempfile::tempdir;
use uuid::Uuid;

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

    let manifest_json: serde_json::Value = serde_json::from_slice(&manifest_1).unwrap();
    let index_sha = manifest_json
        .get("index_sha256")
        .and_then(|v| v.as_str())
        .unwrap();
    assert_eq!(index_sha, sha256_hex(&index_1));
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

    let note = "Recorded in .GOV/task_packets/WP-1-Role-Mailbox-v1.md (password=abc)";
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
            target_ref: dummy_artifact("/.GOV/task_packets/WP-1-Role-Mailbox-v1.md"),
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
    let line_json: serde_json::Value = serde_json::from_str(first_line).unwrap();
    assert_eq!(
        line_json
            .get("body_sha256")
            .and_then(|v| v.as_str())
            .unwrap(),
        body_sha
    );

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
