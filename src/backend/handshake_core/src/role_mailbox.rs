use chrono::{DateTime, SecondsFormat, Timelike, Utc};
use duckdb::Connection as DuckDbConnection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::ace::ArtifactHandle;
use crate::bundles::redactor::SecretRedactor;
use crate::bundles::schemas::RedactionMode;
use crate::flight_recorder::{
    FlightRecorder, FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
};

pub const ROLE_MAILBOX_EXPORT_SCHEMA_VERSION: &str = "role_mailbox_export_v1";
pub const ROLE_MAILBOX_EXPORT_ROOT: &str = "docs/ROLE_MAILBOX/";

#[derive(thiserror::Error, Debug)]
pub enum RoleMailboxError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("duckdb error: {0}")]
    DuckDb(String),
    #[error("flight recorder error: {0}")]
    FlightRecorder(String),
}

impl From<duckdb::Error> for RoleMailboxError {
    fn from(value: duckdb::Error) -> Self {
        Self::DuckDb(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceMode {
    GovStrict,
    GovStandard,
    GovLight,
}

impl GovernanceMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            GovernanceMode::GovStrict => "gov_strict",
            GovernanceMode::GovStandard => "gov_standard",
            GovernanceMode::GovLight => "gov_light",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoleId {
    Operator,
    Orchestrator,
    Coder,
    Validator,
    Advisory(String),
}

impl fmt::Display for RoleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoleId::Operator => write!(f, "operator"),
            RoleId::Orchestrator => write!(f, "orchestrator"),
            RoleId::Coder => write!(f, "coder"),
            RoleId::Validator => write!(f, "validator"),
            RoleId::Advisory(id) => write!(f, "advisory:{id}"),
        }
    }
}

impl RoleId {
    pub fn parse(value: &str) -> Result<Self, RoleMailboxError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(RoleMailboxError::InvalidInput(
                "role id must be non-empty".to_string(),
            ));
        }

        match trimmed {
            "operator" => Ok(RoleId::Operator),
            "orchestrator" => Ok(RoleId::Orchestrator),
            "coder" => Ok(RoleId::Coder),
            "validator" => Ok(RoleId::Validator),
            _ => {
                let suffix = trimmed
                    .strip_prefix("advisory:")
                    .ok_or_else(|| RoleMailboxError::InvalidInput("invalid role id".to_string()))?;
                if !is_safe_id(suffix, 128) {
                    return Err(RoleMailboxError::InvalidInput(
                        "advisory role id must be a safe id".to_string(),
                    ));
                }
                Ok(RoleId::Advisory(suffix.to_string()))
            }
        }
    }

    pub fn is_advisory(&self) -> bool {
        matches!(self, RoleId::Advisory(_))
    }
}

impl Serialize for RoleId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for RoleId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        RoleId::parse(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoleMailboxMessageType {
    ClarificationRequest,
    ClarificationResponse,
    ScopeRisk,
    ScopeChangeProposal,
    ScopeChangeApproval,
    WaiverProposal,
    WaiverApproval,
    ValidationFinding,
    Handoff,
    Blocker,
    ToolingRequest,
    ToolingResult,
    #[serde(rename = "fyi")]
    FYI,
}

impl RoleMailboxMessageType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleMailboxMessageType::ClarificationRequest => "clarification_request",
            RoleMailboxMessageType::ClarificationResponse => "clarification_response",
            RoleMailboxMessageType::ScopeRisk => "scope_risk",
            RoleMailboxMessageType::ScopeChangeProposal => "scope_change_proposal",
            RoleMailboxMessageType::ScopeChangeApproval => "scope_change_approval",
            RoleMailboxMessageType::WaiverProposal => "waiver_proposal",
            RoleMailboxMessageType::WaiverApproval => "waiver_approval",
            RoleMailboxMessageType::ValidationFinding => "validation_finding",
            RoleMailboxMessageType::Handoff => "handoff",
            RoleMailboxMessageType::Blocker => "blocker",
            RoleMailboxMessageType::ToolingRequest => "tooling_request",
            RoleMailboxMessageType::ToolingResult => "tooling_result",
            RoleMailboxMessageType::FYI => "fyi",
        }
    }

    pub fn requires_transcription_links(&self) -> bool {
        matches!(
            self,
            RoleMailboxMessageType::ScopeChangeApproval
                | RoleMailboxMessageType::WaiverApproval
                | RoleMailboxMessageType::ValidationFinding
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMailboxContext {
    pub spec_id: Option<String>,
    pub work_packet_id: Option<String>,
    pub task_board_id: Option<String>,
    pub governance_mode: GovernanceMode,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMailboxThread {
    pub thread_id: String,
    pub subject: String,
    pub context: RoleMailboxContext,
    pub participants: Vec<RoleId>,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionTargetKind {
    Refinement,
    TaskPacket,
    TaskBoard,
    GateState,
    SignatureAudit,
    Waiver,
    SpecArtifact,
}

impl TranscriptionTargetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranscriptionTargetKind::Refinement => "refinement",
            TranscriptionTargetKind::TaskPacket => "task_packet",
            TranscriptionTargetKind::TaskBoard => "task_board",
            TranscriptionTargetKind::GateState => "gate_state",
            TranscriptionTargetKind::SignatureAudit => "signature_audit",
            TranscriptionTargetKind::Waiver => "waiver",
            TranscriptionTargetKind::SpecArtifact => "spec_artifact",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionLink {
    pub target_kind: TranscriptionTargetKind,
    pub target_ref: ArtifactHandle,
    pub target_sha256: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleMailboxMessage {
    pub message_id: String,
    pub thread_id: String,
    pub created_at: DateTime<Utc>,
    pub from_role: RoleId,
    pub to_roles: Vec<RoleId>,
    pub message_type: RoleMailboxMessageType,
    pub body_ref: ArtifactHandle,
    pub body_sha256: String,
    pub attachments: Vec<ArtifactHandle>,
    pub relates_to_message_id: Option<String>,
    pub transcription_links: Vec<TranscriptionLink>,
    pub idempotency_key: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn sha256_hex_utf8(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn now_utc_seconds() -> DateTime<Utc> {
    let now = Utc::now();
    now.with_nanosecond(0).unwrap_or(now)
}

fn format_rfc3339_seconds(timestamp: DateTime<Utc>) -> String {
    timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn bounded_single_line(value: &str, max_len: usize) -> String {
    let mut out = String::with_capacity(value.len().min(max_len));
    for ch in value.chars() {
        if ch == '\r' || ch == '\n' {
            out.push(' ');
        } else {
            out.push(ch);
        }
        if out.chars().count() >= max_len {
            break;
        }
    }
    out.trim().to_string()
}

fn redact_bounded_single_line(redactor: &SecretRedactor, value: &str) -> String {
    let (redacted, _logs) = redactor.redact_value(
        &serde_json::Value::String(value.to_string()),
        RedactionMode::SafeDefault,
        "role_mailbox",
    );

    let text = redacted.as_str().unwrap_or("");
    let bounded = bounded_single_line(text, 160);
    if bounded.is_empty() {
        "[REDACTED]".to_string()
    } else {
        bounded
    }
}

fn is_safe_id(value: &str, max_len: usize) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len {
        return false;
    }

    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn is_safe_token(value: &str, max_len: usize) -> bool {
    let value = value.trim();
    if value.is_empty() || value.len() > max_len {
        return false;
    }

    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.' | '/'))
}

fn is_sha256_hex(value: &str) -> bool {
    let value = value.trim();
    if value.len() != 64 {
        return false;
    }

    value
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, 'a'..='f'))
}

fn ensure_advisory_not_solo(
    mode: GovernanceMode,
    participants: &[RoleId],
) -> Result<(), RoleMailboxError> {
    if matches!(
        mode,
        GovernanceMode::GovStandard | GovernanceMode::GovStrict
    ) && participants.iter().all(RoleId::is_advisory)
    {
        return Err(RoleMailboxError::InvalidInput(
            "advisory roles must not be the only participants in GOV_STANDARD/GOV_STRICT"
                .to_string(),
        ));
    }
    Ok(())
}

fn repo_root_from_manifest() -> Result<PathBuf, RoleMailboxError> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| RoleMailboxError::InvalidInput("failed to resolve repo root".to_string()))
}

fn init_role_mailbox_schema(conn: &DuckDbConnection) -> Result<(), RoleMailboxError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS role_mailbox_body_artifacts (
            body_sha256 TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL,
            path TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS role_mailbox_threads (
            thread_id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL,
            closed_at TEXT,
            participants JSON NOT NULL,
            context_spec_id TEXT,
            context_work_packet_id TEXT,
            context_task_board_id TEXT,
            context_governance_mode TEXT NOT NULL,
            context_project_id TEXT,
            subject_redacted TEXT NOT NULL,
            subject_sha256 TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS role_mailbox_messages (
            message_id TEXT PRIMARY KEY,
            thread_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            from_role TEXT NOT NULL,
            to_roles JSON NOT NULL,
            message_type TEXT NOT NULL,
            body_ref TEXT NOT NULL,
            body_sha256 TEXT NOT NULL,
            attachments JSON NOT NULL,
            relates_to_message_id TEXT,
            transcription_links JSON NOT NULL,
            idempotency_key TEXT NOT NULL UNIQUE,
            context_spec_id TEXT,
            context_work_packet_id TEXT,
            context_task_board_id TEXT,
            context_governance_mode TEXT NOT NULL,
            context_project_id TEXT
        );

        CREATE TABLE IF NOT EXISTS spec_session_log_entries (
            entry_id TEXT PRIMARY KEY,
            spec_id TEXT NOT NULL,
            task_board_id TEXT NOT NULL,
            work_packet_id TEXT,
            event_type TEXT NOT NULL,
            governance_mode TEXT NOT NULL,
            actor TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            summary TEXT NOT NULL,
            linked_artifacts JSON NOT NULL
        );
        "#,
    )?;

    Ok(())
}

fn load_optional_string(
    conn: &DuckDbConnection,
    query: &str,
    params: &[&dyn duckdb::ToSql],
) -> Result<Option<String>, RoleMailboxError> {
    let mut stmt = conn.prepare(query)?;
    match stmt.query_row(params, |row| row.get::<_, String>(0)) {
        Ok(value) => Ok(Some(value)),
        Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

#[derive(Debug, Clone)]
pub struct CreateRoleMailboxMessageRequest {
    pub thread_id: Option<String>,
    pub thread_subject: Option<String>,
    pub thread_participants: Option<Vec<RoleId>>,
    pub context: RoleMailboxContext,
    pub from_role: RoleId,
    pub to_roles: Vec<RoleId>,
    pub message_type: RoleMailboxMessageType,
    pub body: String,
    pub attachments: Vec<ArtifactHandle>,
    pub relates_to_message_id: Option<String>,
    pub transcription_links: Vec<TranscriptionLink>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone)]
pub struct AddTranscriptionLinkRequest {
    pub thread_id: String,
    pub message_id: String,
    pub link: TranscriptionLink,
}

#[derive(Clone)]
pub struct RoleMailbox {
    root_dir: PathBuf,
    export_dir: PathBuf,
    conn: Arc<Mutex<DuckDbConnection>>,
    flight_recorder: Arc<dyn FlightRecorder>,
    redactor: SecretRedactor,
}

impl RoleMailbox {
    pub fn new_for_repo(
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Result<Self, RoleMailboxError> {
        let root_dir = repo_root_from_manifest()?;
        Self::new_for_root(root_dir, flight_recorder)
    }

    pub fn new_for_root(
        root_dir: PathBuf,
        flight_recorder: Arc<dyn FlightRecorder>,
    ) -> Result<Self, RoleMailboxError> {
        let data_dir = root_dir.join("data");
        fs::create_dir_all(&data_dir)?;

        let conn = flight_recorder.duckdb_connection().ok_or_else(|| {
            RoleMailboxError::DuckDb(
                "flight_recorder does not expose DuckDB connection".to_string(),
            )
        })?;
        {
            let conn = conn
                .lock()
                .map_err(|_| RoleMailboxError::DuckDb("lock error".to_string()))?;
            init_role_mailbox_schema(&conn)?;
        }

        Ok(Self {
            export_dir: root_dir.join("docs").join("ROLE_MAILBOX"),
            root_dir,
            conn,
            flight_recorder,
            redactor: SecretRedactor::new(),
        })
    }

    pub async fn create_message(
        &self,
        req: CreateRoleMailboxMessageRequest,
    ) -> Result<RoleMailboxMessage, RoleMailboxError> {
        if !is_safe_token(&req.idempotency_key, 256) {
            return Err(RoleMailboxError::InvalidInput(
                "idempotency_key must be a bounded safe token".to_string(),
            ));
        }
        if req.to_roles.is_empty() {
            return Err(RoleMailboxError::InvalidInput(
                "to_roles must be non-empty".to_string(),
            ));
        }
        if req.message_type.requires_transcription_links() && req.transcription_links.is_empty() {
            return Err(RoleMailboxError::InvalidInput(format!(
                "message_type {} requires transcription_links",
                req.message_type.as_str()
            )));
        }

        let thread_id = req
            .thread_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        if !is_safe_id(&thread_id, 128) {
            return Err(RoleMailboxError::InvalidInput(
                "thread_id must be a safe id".to_string(),
            ));
        }
        if let Some(relates) = req.relates_to_message_id.as_deref() {
            if !is_safe_id(relates, 128) {
                return Err(RoleMailboxError::InvalidInput(
                    "relates_to_message_id must be a safe id".to_string(),
                ));
            }
        }

        let created_at = now_utc_seconds();
        let message_id = Uuid::new_v4().to_string();

        enum CreateMessageDbOutcome {
            Existing {
                message_id: String,
            },
            Created {
                body_ref: ArtifactHandle,
                body_sha256: String,
            },
        }

        let outcome = {
            let conn = self
                .conn
                .lock()
                .map_err(|_| RoleMailboxError::DuckDb("lock error".to_string()))?;
            conn.execute_batch("BEGIN TRANSACTION")?;

            let existing = load_optional_string(
                &conn,
                "SELECT message_id FROM role_mailbox_messages WHERE idempotency_key = ?",
                &[&req.idempotency_key],
            )?;
            if let Some(existing_id) = existing {
                conn.execute_batch("COMMIT")?;
                CreateMessageDbOutcome::Existing {
                    message_id: existing_id,
                }
            } else {
                self.ensure_thread_exists(&conn, &thread_id, &req, created_at)?;

                let (body_ref, body_sha256) =
                    self.ensure_body_artifact(&conn, req.body.as_bytes())?;

                let to_roles_json = serde_json::to_string(
                    &req.to_roles
                        .iter()
                        .map(|r| r.to_string())
                        .collect::<Vec<_>>(),
                )?;
                let attachments_json = serde_json::to_string(
                    &req.attachments
                        .iter()
                        .map(ArtifactHandle::canonical_id)
                        .collect::<Vec<_>>(),
                )?;
                let transcription_links_json = serde_json::to_string(
                    &req.transcription_links
                        .iter()
                        .map(|link| self.export_link_shape(link))
                        .collect::<Result<Vec<_>, RoleMailboxError>>()?,
                )?;

                conn.execute(
                    r#"
                    INSERT INTO role_mailbox_messages (
                        message_id,
                        thread_id,
                        created_at,
                        from_role,
                        to_roles,
                        message_type,
                        body_ref,
                        body_sha256,
                        attachments,
                        relates_to_message_id,
                        transcription_links,
                        idempotency_key,
                        context_spec_id,
                        context_work_packet_id,
                        context_task_board_id,
                        context_governance_mode,
                        context_project_id
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                    duckdb::params![
                        message_id,
                        thread_id,
                        format_rfc3339_seconds(created_at),
                        req.from_role.to_string(),
                        to_roles_json,
                        req.message_type.as_str(),
                        body_ref.canonical_id(),
                        body_sha256,
                        attachments_json,
                        req.relates_to_message_id,
                        transcription_links_json,
                        req.idempotency_key,
                        req.context.spec_id,
                        req.context.work_packet_id,
                        req.context.task_board_id,
                        req.context.governance_mode.as_str(),
                        req.context.project_id
                    ],
                )?;

                conn.execute_batch("COMMIT")?;

                CreateMessageDbOutcome::Created {
                    body_ref,
                    body_sha256,
                }
            }
        };

        let (body_ref, body_sha256) = match outcome {
            CreateMessageDbOutcome::Existing { message_id } => {
                return self.load_message_by_id(&message_id).await;
            }
            CreateMessageDbOutcome::Created {
                body_ref,
                body_sha256,
            } => (body_ref, body_sha256),
        };

        self.emit_fr_message_created(
            &req.context,
            &thread_id,
            &message_id,
            &req.from_role,
            &req.to_roles,
            req.message_type,
            &body_ref,
            &body_sha256,
            &req.idempotency_key,
        )
        .await?;

        self.append_spec_session_log(
            &req.context,
            req.from_role.to_string(),
            "mailbox_message_created",
            bounded_single_line(
                &format!(
                    "mailbox_message_created {} thread={} message={}",
                    req.message_type.as_str(),
                    thread_id,
                    message_id
                ),
                256,
            ),
            self.collect_linked_artifacts(&body_ref, &req.attachments, &req.transcription_links),
        )?;

        if matches!(
            req.context.governance_mode,
            GovernanceMode::GovStandard | GovernanceMode::GovStrict
        ) {
            let _ = self
                .export_repo(&req.context, req.from_role.to_string())
                .await?;
        }

        Ok(RoleMailboxMessage {
            message_id,
            thread_id,
            created_at,
            from_role: req.from_role,
            to_roles: req.to_roles,
            message_type: req.message_type,
            body_ref,
            body_sha256,
            attachments: req.attachments,
            relates_to_message_id: req.relates_to_message_id,
            transcription_links: req
                .transcription_links
                .into_iter()
                .map(|mut link| {
                    link.note = redact_bounded_single_line(&self.redactor, &link.note);
                    link
                })
                .collect(),
            idempotency_key: req.idempotency_key,
        })
    }

    pub async fn add_transcription_link(
        &self,
        req: AddTranscriptionLinkRequest,
    ) -> Result<(), RoleMailboxError> {
        if !is_safe_id(&req.thread_id, 128) {
            return Err(RoleMailboxError::InvalidInput(
                "thread_id must be a safe id".to_string(),
            ));
        }
        if !is_safe_id(&req.message_id, 128) {
            return Err(RoleMailboxError::InvalidInput(
                "message_id must be a safe id".to_string(),
            ));
        }
        if !is_sha256_hex(&req.link.target_sha256) {
            return Err(RoleMailboxError::InvalidInput(
                "target_sha256 must be a 64-char lowercase hex sha256".to_string(),
            ));
        }

        let (context, actor) = {
            let conn = self
                .conn
                .lock()
                .map_err(|_| RoleMailboxError::DuckDb("lock error".to_string()))?;

            let mut stmt = conn.prepare(
                r#"
                SELECT
                    context_spec_id,
                    context_work_packet_id,
                    context_task_board_id,
                    context_governance_mode,
                    context_project_id,
                    from_role,
                    transcription_links
                FROM role_mailbox_messages
                WHERE thread_id = ? AND message_id = ?
                "#,
            )?;

            let row = stmt.query_row(duckdb::params![req.thread_id, req.message_id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })?;

            let mode = match row.3.as_str() {
                "gov_strict" => GovernanceMode::GovStrict,
                "gov_standard" => GovernanceMode::GovStandard,
                "gov_light" => GovernanceMode::GovLight,
                other => {
                    return Err(RoleMailboxError::InvalidInput(format!(
                        "stored governance_mode invalid: {other}"
                    )))
                }
            };

            let mut links_value: Value = serde_json::from_str(&row.6)?;
            let links_arr = links_value.as_array_mut().ok_or_else(|| {
                RoleMailboxError::InvalidInput("transcription_links must be array".to_string())
            })?;
            links_arr.push(json!(self.export_link_shape(&req.link)?));

            conn.execute(
                "UPDATE role_mailbox_messages SET transcription_links = ? WHERE thread_id = ? AND message_id = ?",
                duckdb::params![serde_json::to_string(&links_value)?, req.thread_id, req.message_id],
            )?;

            (
                RoleMailboxContext {
                    spec_id: row.0,
                    work_packet_id: row.1,
                    task_board_id: row.2,
                    governance_mode: mode,
                    project_id: row.4,
                },
                row.5,
            )
        };

        self.emit_fr_transcribed(&req.thread_id, &req.message_id, &req.link)
            .await?;

        self.append_spec_session_log(
            &context,
            actor.clone(),
            "mailbox_transcribed",
            bounded_single_line(
                &format!(
                    "mailbox_transcribed kind={} thread={} message={}",
                    req.link.target_kind.as_str(),
                    req.thread_id,
                    req.message_id
                ),
                256,
            ),
            vec![req.link.target_ref.clone()],
        )?;

        if matches!(
            context.governance_mode,
            GovernanceMode::GovStandard | GovernanceMode::GovStrict
        ) {
            let _ = self.export_repo(&context, actor).await?;
        }

        Ok(())
    }

    async fn load_message_by_id(
        &self,
        message_id: &str,
    ) -> Result<RoleMailboxMessage, RoleMailboxError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RoleMailboxError::DuckDb("lock error".to_string()))?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                message_id,
                thread_id,
                created_at,
                from_role,
                to_roles,
                message_type,
                body_ref,
                body_sha256,
                attachments,
                relates_to_message_id,
                transcription_links,
                idempotency_key
            FROM role_mailbox_messages
            WHERE message_id = ?
            "#,
        )?;

        let row = stmt.query_row(duckdb::params![message_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, String>(11)?,
            ))
        })?;

        let created_at = DateTime::parse_from_rfc3339(&row.2)
            .map_err(|e| RoleMailboxError::InvalidInput(format!("invalid created_at: {e}")))?
            .with_timezone(&Utc);
        let from_role = RoleId::parse(&row.3)?;
        let to_roles: Vec<String> = serde_json::from_str(&row.4)?;
        let to_roles = to_roles
            .iter()
            .map(|v| RoleId::parse(v))
            .collect::<Result<Vec<_>, _>>()?;
        let message_type = match row.5.as_str() {
            "clarification_request" => RoleMailboxMessageType::ClarificationRequest,
            "clarification_response" => RoleMailboxMessageType::ClarificationResponse,
            "scope_risk" => RoleMailboxMessageType::ScopeRisk,
            "scope_change_proposal" => RoleMailboxMessageType::ScopeChangeProposal,
            "scope_change_approval" => RoleMailboxMessageType::ScopeChangeApproval,
            "waiver_proposal" => RoleMailboxMessageType::WaiverProposal,
            "waiver_approval" => RoleMailboxMessageType::WaiverApproval,
            "validation_finding" => RoleMailboxMessageType::ValidationFinding,
            "handoff" => RoleMailboxMessageType::Handoff,
            "blocker" => RoleMailboxMessageType::Blocker,
            "tooling_request" => RoleMailboxMessageType::ToolingRequest,
            "tooling_result" => RoleMailboxMessageType::ToolingResult,
            "fyi" => RoleMailboxMessageType::FYI,
            other => {
                return Err(RoleMailboxError::InvalidInput(format!(
                    "stored message_type invalid: {other}"
                )))
            }
        };

        let body_ref = parse_artifact_handle_string(&row.6)?;
        let attachments: Vec<String> = serde_json::from_str(&row.8)?;
        let attachments = attachments
            .iter()
            .map(|v| parse_artifact_handle_string(v))
            .collect::<Result<Vec<_>, _>>()?;

        let transcription_links: Vec<ExportTranscriptionLinkV1> = serde_json::from_str(&row.10)?;
        let transcription_links = transcription_links
            .into_iter()
            .map(|link| {
                Ok(TranscriptionLink {
                    target_kind: parse_transcription_target_kind(&link.target_kind)?,
                    target_ref: parse_artifact_handle_string(&link.target_ref)?,
                    target_sha256: link.target_sha256,
                    note: link.note_redacted,
                })
            })
            .collect::<Result<Vec<_>, RoleMailboxError>>()?;

        Ok(RoleMailboxMessage {
            message_id: row.0,
            thread_id: row.1,
            created_at,
            from_role,
            to_roles,
            message_type,
            body_ref,
            body_sha256: row.7,
            attachments,
            relates_to_message_id: row.9,
            transcription_links,
            idempotency_key: row.11,
        })
    }

    fn ensure_thread_exists(
        &self,
        conn: &DuckDbConnection,
        thread_id: &str,
        req: &CreateRoleMailboxMessageRequest,
        created_at: DateTime<Utc>,
    ) -> Result<(), RoleMailboxError> {
        let exists = load_optional_string(
            conn,
            "SELECT thread_id FROM role_mailbox_threads WHERE thread_id = ?",
            &[&thread_id],
        )?
        .is_some();
        if exists {
            return Ok(());
        }

        let subject = req.thread_subject.as_deref().ok_or_else(|| {
            RoleMailboxError::InvalidInput(
                "thread_subject required when creating a new thread".to_string(),
            )
        })?;

        let participants = match req.thread_participants.clone() {
            Some(value) => value,
            None => {
                let mut out = Vec::new();
                out.push(req.from_role.clone());
                out.extend(req.to_roles.clone());
                out
            }
        };

        if participants.is_empty() {
            return Err(RoleMailboxError::InvalidInput(
                "thread participants must be non-empty".to_string(),
            ));
        }
        ensure_advisory_not_solo(req.context.governance_mode, &participants)?;

        let subject_sha256 = sha256_hex_utf8(subject);
        let subject_redacted = redact_bounded_single_line(&self.redactor, subject);
        let participants_json = serde_json::to_string(
            &participants
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>(),
        )?;

        conn.execute(
            r#"
            INSERT INTO role_mailbox_threads (
                thread_id,
                created_at,
                closed_at,
                participants,
                context_spec_id,
                context_work_packet_id,
                context_task_board_id,
                context_governance_mode,
                context_project_id,
                subject_redacted,
                subject_sha256
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            duckdb::params![
                thread_id,
                format_rfc3339_seconds(created_at),
                Option::<String>::None,
                participants_json,
                req.context.spec_id.clone(),
                req.context.work_packet_id.clone(),
                req.context.task_board_id.clone(),
                req.context.governance_mode.as_str(),
                req.context.project_id.clone(),
                subject_redacted,
                subject_sha256
            ],
        )?;

        Ok(())
    }

    fn ensure_body_artifact(
        &self,
        conn: &DuckDbConnection,
        body_bytes: &[u8],
    ) -> Result<(ArtifactHandle, String), RoleMailboxError> {
        let body_sha256 = sha256_hex(body_bytes);

        let mut stmt = conn.prepare(
            "SELECT artifact_id, path FROM role_mailbox_body_artifacts WHERE body_sha256 = ?",
        )?;
        let existing = match stmt.query_row(duckdb::params![body_sha256.clone()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }) {
            Ok(found) => Some(found),
            Err(duckdb::Error::QueryReturnedNoRows) => None,
            Err(err) => return Err(err.into()),
        };

        if let Some((artifact_id, path)) = existing {
            let uuid = Uuid::parse_str(&artifact_id).map_err(|e| {
                RoleMailboxError::InvalidInput(format!("stored artifact id invalid: {e}"))
            })?;
            return Ok((ArtifactHandle::new(uuid, path), body_sha256));
        }

        let rel_path = format!("data/role_mailbox_bodies/{}.bin", body_sha256);
        let abs_path = self.root_dir.join(&rel_path);
        if let Some(parent) = abs_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !abs_path.exists() {
            fs::write(&abs_path, body_bytes)?;
        }

        let artifact_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO role_mailbox_body_artifacts (body_sha256, artifact_id, path) VALUES (?, ?, ?)",
            duckdb::params![body_sha256.clone(), artifact_id.to_string(), rel_path.clone()],
        )?;

        Ok((ArtifactHandle::new(artifact_id, rel_path), body_sha256))
    }

    fn export_link_shape(
        &self,
        link: &TranscriptionLink,
    ) -> Result<ExportTranscriptionLinkV1, RoleMailboxError> {
        if !is_sha256_hex(&link.target_sha256) {
            return Err(RoleMailboxError::InvalidInput(
                "transcription target_sha256 must be a 64-char lowercase hex sha256".to_string(),
            ));
        }
        Ok(ExportTranscriptionLinkV1 {
            target_kind: link.target_kind.as_str().to_string(),
            target_ref: link.target_ref.canonical_id(),
            target_sha256: link.target_sha256.clone(),
            note_redacted: redact_bounded_single_line(&self.redactor, &link.note),
            note_sha256: sha256_hex_utf8(&link.note),
        })
    }

    fn collect_linked_artifacts(
        &self,
        body_ref: &ArtifactHandle,
        attachments: &[ArtifactHandle],
        transcription_links: &[TranscriptionLink],
    ) -> Vec<ArtifactHandle> {
        let mut out = Vec::new();
        out.push(body_ref.clone());
        out.extend(attachments.iter().cloned());
        out.extend(transcription_links.iter().map(|l| l.target_ref.clone()));
        out
    }

    fn append_spec_session_log(
        &self,
        context: &RoleMailboxContext,
        actor: String,
        event_type: &str,
        summary: String,
        linked_artifacts: Vec<ArtifactHandle>,
    ) -> Result<(), RoleMailboxError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| RoleMailboxError::DuckDb("lock error".to_string()))?;

        let entry_id = Uuid::new_v4().to_string();
        let spec_id = context
            .spec_id
            .clone()
            .unwrap_or_else(|| "UNKNOWN".to_string());
        let task_board_id = context
            .task_board_id
            .clone()
            .unwrap_or_else(|| "docs/TASK_BOARD.md".to_string());
        let timestamp = format_rfc3339_seconds(now_utc_seconds());
        let linked_json = serde_json::to_string(&linked_artifacts)?;

        conn.execute(
            r#"
            INSERT INTO spec_session_log_entries (
                entry_id,
                spec_id,
                task_board_id,
                work_packet_id,
                event_type,
                governance_mode,
                actor,
                timestamp,
                summary,
                linked_artifacts
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            duckdb::params![
                entry_id,
                spec_id,
                task_board_id,
                context.work_packet_id.clone(),
                event_type,
                context.governance_mode.as_str(),
                actor,
                timestamp,
                summary,
                linked_json
            ],
        )?;

        Ok(())
    }

    async fn emit_fr_message_created(
        &self,
        context: &RoleMailboxContext,
        thread_id: &str,
        message_id: &str,
        from_role: &RoleId,
        to_roles: &[RoleId],
        message_type: RoleMailboxMessageType,
        body_ref: &ArtifactHandle,
        body_sha256: &str,
        idempotency_key: &str,
    ) -> Result<(), RoleMailboxError> {
        let actor = match from_role {
            RoleId::Operator => FlightRecorderActor::Human,
            _ => FlightRecorderActor::Agent,
        };

        let payload = json!({
            "type": "gov_mailbox_message_created",
            "spec_id": context.spec_id,
            "work_packet_id": context.work_packet_id,
            "governance_mode": context.governance_mode.as_str(),
            "thread_id": thread_id,
            "message_id": message_id,
            "from_role": from_role.to_string(),
            "to_roles": to_roles.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
            "message_type": message_type.as_str(),
            "body_ref": body_ref.canonical_id(),
            "body_sha256": body_sha256,
            "idempotency_key": idempotency_key,
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::GovMailboxMessageCreated,
            actor,
            Uuid::new_v4(),
            payload,
        )
        .with_actor_id(from_role.to_string());

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|e| RoleMailboxError::FlightRecorder(e.to_string()))
    }

    async fn emit_fr_transcribed(
        &self,
        thread_id: &str,
        message_id: &str,
        link: &TranscriptionLink,
    ) -> Result<(), RoleMailboxError> {
        let payload = json!({
            "type": "gov_mailbox_transcribed",
            "thread_id": thread_id,
            "message_id": message_id,
            "transcription_target_kind": link.target_kind.as_str(),
            "target_ref": link.target_ref.canonical_id(),
            "target_sha256": link.target_sha256,
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::GovMailboxTranscribed,
            FlightRecorderActor::Agent,
            Uuid::new_v4(),
            payload,
        )
        .with_actor_id("role_mailbox".to_string());

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|e| RoleMailboxError::FlightRecorder(e.to_string()))
    }

    pub async fn export_repo(
        &self,
        context: &RoleMailboxContext,
        actor: String,
    ) -> Result<RoleMailboxExportSummary, RoleMailboxError> {
        fs::create_dir_all(&self.export_dir)?;

        let (index_bytes, thread_files, thread_count, message_count, generated_at) = {
            let conn = self
                .conn
                .lock()
                .map_err(|_| RoleMailboxError::DuckDb("lock error".to_string()))?;

            let mut stmt = conn.prepare(
                r#"
                SELECT
                    thread_id,
                    created_at,
                    closed_at,
                    participants,
                    context_spec_id,
                    context_work_packet_id,
                    context_task_board_id,
                    context_governance_mode,
                    context_project_id,
                    subject_redacted,
                    subject_sha256
                FROM role_mailbox_threads
                ORDER BY created_at ASC, thread_id ASC
                "#,
            )?;

            let thread_rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                ))
            })?;

            let mut threads_out: Vec<Value> = Vec::new();
            let mut thread_files_out: Vec<(String, Vec<u8>, u64)> = Vec::new();
            let mut max_ts = "1970-01-01T00:00:00Z".to_string();
            let mut total_messages: u64 = 0;

            for thread_row in thread_rows {
                let (
                    thread_id,
                    created_at,
                    closed_at,
                    participants_json,
                    context_spec_id,
                    context_work_packet_id,
                    context_task_board_id,
                    context_governance_mode,
                    context_project_id,
                    subject_redacted,
                    subject_sha256,
                ) = thread_row?;

                if !is_safe_id(&thread_id, 128) {
                    return Err(RoleMailboxError::InvalidInput(format!(
                        "thread_id not safe for export: {thread_id}"
                    )));
                }

                max_ts = max_ts.max(created_at.clone());
                if let Some(value) = closed_at.as_deref() {
                    max_ts = max_ts.max(value.to_string());
                }

                let participants: Vec<String> = serde_json::from_str(&participants_json)?;

                let thread_file_rel = format!("threads/{}.jsonl", thread_id);

                let mut msg_stmt = conn.prepare(
                    r#"
                    SELECT
                        message_id,
                        created_at,
                        from_role,
                        to_roles,
                        message_type,
                        body_ref,
                        body_sha256,
                        attachments,
                        relates_to_message_id,
                        transcription_links,
                        idempotency_key
                    FROM role_mailbox_messages
                    WHERE thread_id = ?
                    ORDER BY created_at ASC, message_id ASC
                    "#,
                )?;

                let msg_rows = msg_stmt.query_map(duckdb::params![thread_id.clone()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                    ))
                })?;

                let mut thread_bytes: Vec<u8> = Vec::new();
                let mut thread_message_count: u64 = 0;

                for msg_row in msg_rows {
                    let (
                        message_id,
                        msg_created_at,
                        from_role,
                        to_roles_json,
                        message_type,
                        body_ref,
                        body_sha256,
                        attachments_json,
                        relates_to_message_id,
                        transcription_links_json,
                        idempotency_key,
                    ) = msg_row?;

                    if !is_safe_id(&message_id, 128) {
                        return Err(RoleMailboxError::InvalidInput(format!(
                            "message_id not safe for export: {message_id}"
                        )));
                    }

                    max_ts = max_ts.max(msg_created_at.clone());

                    let to_roles: Vec<String> = serde_json::from_str(&to_roles_json)?;
                    let attachments: Vec<String> = serde_json::from_str(&attachments_json)?;
                    let transcription_links: Vec<ExportTranscriptionLinkV1> =
                        serde_json::from_str(&transcription_links_json)?;

                    let line = json!({
                        "message_id": message_id,
                        "thread_id": thread_id,
                        "created_at": msg_created_at,
                        "from_role": from_role,
                        "to_roles": to_roles,
                        "message_type": message_type,
                        "body_ref": body_ref,
                        "body_sha256": body_sha256,
                        "attachments": attachments,
                        "relates_to_message_id": relates_to_message_id,
                        "transcription_links": transcription_links,
                        "idempotency_key": idempotency_key,
                    });

                    thread_bytes.extend(canonical_json_bytes(&line));
                    thread_message_count += 1;
                    total_messages += 1;
                }

                if thread_message_count == 0 {
                    thread_bytes = Vec::new();
                }

                thread_files_out.push((
                    thread_file_rel.clone(),
                    thread_bytes,
                    thread_message_count,
                ));

                threads_out.push(json!({
                    "thread_id": thread_id,
                    "created_at": created_at,
                    "closed_at": closed_at,
                    "participants": participants,
                    "context": {
                        "spec_id": context_spec_id,
                        "work_packet_id": context_work_packet_id,
                        "task_board_id": context_task_board_id,
                        "governance_mode": context_governance_mode,
                        "project_id": context_project_id,
                    },
                    "subject_redacted": subject_redacted,
                    "subject_sha256": subject_sha256,
                    "message_count": thread_message_count,
                    "thread_file": thread_file_rel,
                }));
            }

            thread_files_out.sort_by(|a, b| a.0.cmp(&b.0));

            let thread_count = threads_out.len() as u64;
            let index = json!({
                "schema_version": ROLE_MAILBOX_EXPORT_SCHEMA_VERSION,
                "generated_at": max_ts,
                "threads": threads_out,
            });
            let index_bytes = canonical_json_bytes(&index);

            Ok::<_, RoleMailboxError>((
                index_bytes,
                thread_files_out,
                thread_count,
                total_messages,
                max_ts,
            ))
        }?;

        // Write thread files first, then index+manifest for sync.
        let mut thread_manifest_entries: Vec<Value> = Vec::new();
        for (rel_path, bytes, msg_count) in &thread_files {
            let abs = self.export_dir.join(rel_path);
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&abs, bytes)?;
            thread_manifest_entries.push(json!({
                "path": rel_path,
                "sha256": sha256_hex(bytes),
                "message_count": msg_count,
            }));
        }

        let index_path = self.export_dir.join("index.json");
        fs::write(&index_path, &index_bytes)?;
        let index_sha256 = sha256_hex(&index_bytes);

        let manifest = json!({
            "schema_version": ROLE_MAILBOX_EXPORT_SCHEMA_VERSION,
            "export_root": ROLE_MAILBOX_EXPORT_ROOT,
            "generated_at": generated_at,
            "index_sha256": index_sha256,
            "thread_files": thread_manifest_entries,
        });
        let manifest_bytes = canonical_json_bytes(&manifest);
        let manifest_path = self.export_dir.join("export_manifest.json");
        fs::write(&manifest_path, &manifest_bytes)?;
        let export_manifest_sha256 = sha256_hex(&manifest_bytes);

        self.emit_fr_exported(&export_manifest_sha256, thread_count, message_count)
            .await?;
        self.append_spec_session_log(
            context,
            actor,
            "mailbox_exported",
            bounded_single_line(
                &format!(
                    "mailbox_exported threads={} messages={} manifest_sha256={}",
                    thread_count, message_count, export_manifest_sha256
                ),
                256,
            ),
            vec![ArtifactHandle::new(
                Uuid::new_v4(),
                "docs/ROLE_MAILBOX/export_manifest.json".to_string(),
            )],
        )?;

        Ok(RoleMailboxExportSummary {
            export_manifest_sha256,
            thread_count,
            message_count,
        })
    }

    async fn emit_fr_exported(
        &self,
        export_manifest_sha256: &str,
        thread_count: u64,
        message_count: u64,
    ) -> Result<(), RoleMailboxError> {
        let payload = json!({
            "type": "gov_mailbox_exported",
            "export_root": ROLE_MAILBOX_EXPORT_ROOT,
            "export_manifest_sha256": export_manifest_sha256,
            "thread_count": thread_count,
            "message_count": message_count,
        });

        let event = FlightRecorderEvent::new(
            FlightRecorderEventType::GovMailboxExported,
            FlightRecorderActor::Agent,
            Uuid::new_v4(),
            payload,
        )
        .with_actor_id("role_mailbox".to_string());

        self.flight_recorder
            .record_event(event)
            .await
            .map_err(|e| RoleMailboxError::FlightRecorder(e.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct RoleMailboxExportSummary {
    pub export_manifest_sha256: String,
    pub thread_count: u64,
    pub message_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportTranscriptionLinkV1 {
    target_kind: String,
    target_ref: String,
    target_sha256: String,
    note_redacted: String,
    note_sha256: String,
}

fn parse_transcription_target_kind(
    value: &str,
) -> Result<TranscriptionTargetKind, RoleMailboxError> {
    match value.trim() {
        "refinement" => Ok(TranscriptionTargetKind::Refinement),
        "task_packet" => Ok(TranscriptionTargetKind::TaskPacket),
        "task_board" => Ok(TranscriptionTargetKind::TaskBoard),
        "gate_state" => Ok(TranscriptionTargetKind::GateState),
        "signature_audit" => Ok(TranscriptionTargetKind::SignatureAudit),
        "waiver" => Ok(TranscriptionTargetKind::Waiver),
        "spec_artifact" => Ok(TranscriptionTargetKind::SpecArtifact),
        other => Err(RoleMailboxError::InvalidInput(format!(
            "invalid transcription_target_kind: {other}"
        ))),
    }
}

fn parse_artifact_handle_string(value: &str) -> Result<ArtifactHandle, RoleMailboxError> {
    let trimmed = value.trim();
    let mut parts = trimmed.splitn(3, ':');
    let tag = parts.next().unwrap_or_default();
    let id_part = parts.next().unwrap_or_default();
    let path_part = parts.next().unwrap_or_default();

    if tag != "artifact" || id_part.is_empty() || path_part.is_empty() {
        return Err(RoleMailboxError::InvalidInput(
            "invalid artifact handle string".to_string(),
        ));
    }

    let artifact_id = Uuid::parse_str(id_part).map_err(|e| {
        RoleMailboxError::InvalidInput(format!("invalid artifact handle uuid: {e}"))
    })?;

    Ok(ArtifactHandle::new(artifact_id, path_part.to_string()))
}

fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical_json_value(&mut out, value);
    out.push('\n');
    out.into_bytes()
}

fn write_canonical_json_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(v) => out.push_str(if *v { "true" } else { "false" }),
        Value::Number(num) => out.push_str(&num.to_string()),
        Value::String(s) => write_canonical_json_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                write_canonical_json_string(out, key);
                out.push(':');
                if let Some(v) = map.get(*key) {
                    write_canonical_json_value(out, v);
                } else {
                    out.push_str("null");
                }
            }
            out.push('}');
        }
    }
}

fn write_canonical_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c if (c as u32) <= 0x7F => out.push(c),
            c if (c as u32) <= 0xFFFF => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => {
                let code = (c as u32) - 0x1_0000;
                let high = 0xD800 + ((code >> 10) & 0x3FF);
                let low = 0xDC00 + (code & 0x3FF);
                out.push_str(&format!("\\u{:04X}\\u{:04X}", high, low));
            }
        }
    }
    out.push('"');
}
