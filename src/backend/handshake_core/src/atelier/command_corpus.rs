//! Command-corpus and action-catalog parity contract (MT-206, WP-KERNEL-005).
//!
//! Spec authority: Section 10.19 "Command Corpus and Action Catalog Parity"
//! (Normative) [ADD v02.189] of `10-product-surfaces.md`. This module is the
//! backend-authority + projection implementation of that contract:
//!   * 10.19.2 -- the per-command action-catalog parity descriptor
//!     (`PRIM-CommandCorpusEntryV1`): `action_id`, `corpus_source`, `owner`,
//!     `actor_eligibility`, `params` schema ref + version, `capabilities`,
//!     `execution_class`, `receipt_shape`, `errors`, `foreground_flag`,
//!     `manual_anchor`, `evidence_class`.
//!   * 10.19.4 -- the durable BLOCKED-anchor record
//!     (`PRIM-CommandCorpusBlockedRecordV1`): a command with no valid product
//!     anchor MUST carry a BLOCKED record, never be silently omitted.
//!   * 10.19.3 -- the deterministic, mechanical ModelManual coverage
//!     cross-check producing a parity report
//!     (`PRIM-CommandCorpusParityReportV1`): total / covered / BLOCKED /
//!     orphaned-manual counts plus per-defect rows.
//!   * 10.19.5 -- the invocation evidence guard: an invocable command can emit
//!     only a descriptor-declared EventLedger evidence family; BLOCKED commands
//!     and undeclared evidence families are denied before any event is written.
//!
//! legacy source (INTENT ONLY; the SQLite/Electron/localhost/polling originals are
//! never copied): `app/backend/automationCommandMap.js`
//! (`getAutomationCommandMap`, `TOP_LEVEL_AUTOMATION_IPC`,
//! `getAllWiredAutomationCommands`, `classifyAutomationCommand` -- the single
//! canonical command surface) and `app/backend/automationManual.js`
//! (`featureGroups[].commands`, `commandReference[].id` -- the code-truth rule
//! that every advertised manual command MUST resolve to a wired command). legacy source
//! enforced this as a JS self-consistency test over an in-process command list;
//! Handshake promotes it to a typed, PostgreSQL-backed parity contract.
//!
//! Storage authority is PostgreSQL only (AtelierStore::pool()); SQLite is
//! forbidden (MT-004). This module is enumeration + parity + projection only:
//! it stores and queries the governed catalog/manual/blocked records and emits
//! events. It NEVER opens a socket, spawns a process, or calls an external
//! endpoint. Every catalog entry whose work is external (LLM/ComfyUI/ASR/
//! download) only DECLARES that execution routes through a governed
//! Workflow-Engine / AI job (LAW-CORPUS-PARITY-004); the actual execution is
//! out of this module.
//!
//! Redaction (10.19.5 EVIDENCE-003): `params_schema_ref`, `receipt_shape`, and
//! every other descriptor field are typed handles / ids, never raw values, so
//! no secret material is stored. Recorded events echo only ids and counts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use super::{AtelierError, AtelierResult, AtelierStore};

/// Command-corpus parity event families (MT-206, MT-005). Defined here so the
/// parent folds these into [`super::event_family::ALL`] and the MT-005 coverage
/// check picks up corpus-parity mutations.
pub mod command_corpus_event_family {
    /// A command-corpus catalog descriptor was registered or updated.
    pub const CORPUS_ENTRY_UPSERTED: &str = "atelier.command_corpus.entry_upserted";
    /// A command's manual anchor was bound to a live ModelManual entry.
    pub const CORPUS_ENTRY_ANCHORED: &str = "atelier.command_corpus.entry_anchored";
    /// A durable BLOCKED record was opened or refreshed for a command.
    pub const CORPUS_BLOCKED_RECORDED: &str = "atelier.command_corpus.blocked_recorded";
    /// A BLOCKED record was cleared because its anchor was supplied.
    pub const CORPUS_BLOCKED_CLEARED: &str = "atelier.command_corpus.blocked_cleared";
    /// A deterministic parity report was materialized over the corpus.
    pub const CORPUS_PARITY_REPORTED: &str = "atelier.command_corpus.parity_reported";

    /// All command-corpus event families (for parity/coverage folding).
    pub const ALL: &[&str] = &[
        CORPUS_ENTRY_UPSERTED,
        CORPUS_ENTRY_ANCHORED,
        CORPUS_BLOCKED_RECORDED,
        CORPUS_BLOCKED_CLEARED,
        CORPUS_PARITY_REPORTED,
    ];
}

/// Re-export at module root so callers can write `command_corpus::CORPUS_*`.
pub use command_corpus_event_family::{
    CORPUS_BLOCKED_CLEARED, CORPUS_BLOCKED_RECORDED, CORPUS_ENTRY_ANCHORED, CORPUS_ENTRY_UPSERTED,
    CORPUS_PARITY_REPORTED,
};

/// Mechanical source a command was discovered in (10.19.2 `corpus_source`).
///
/// Mirrors legacy source `classifyAutomationCommand` returning preload/renderer vs
/// backend dispatcher origin: `Preload` is the renderer-exposed command name,
/// `IpcHandler` is the backend `#[tauri::command]` / kernel action handler,
/// `Both` is a command discovered in both mechanical sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusSource {
    Preload,
    IpcHandler,
    Both,
}

impl CorpusSource {
    /// Stable DB token.
    pub fn as_token(self) -> &'static str {
        match self {
            CorpusSource::Preload => "preload",
            CorpusSource::IpcHandler => "ipc_handler",
            CorpusSource::Both => "both",
        }
    }

    /// Parse a stored token. Unknown tokens are a validation error rather than
    /// a silent default so a corrupt row never masquerades as a valid source.
    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "preload" => Ok(CorpusSource::Preload),
            "ipc_handler" => Ok(CorpusSource::IpcHandler),
            "both" => Ok(CorpusSource::Both),
            other => Err(AtelierError::Validation(format!(
                "unknown corpus_source token: {other}"
            ))),
        }
    }
}

/// Execution class of a catalog command (10.19.2 `execution_class`).
///
/// `PureProjection` is read-only; `WriteBox` routes through the write-box +
/// promotion path; `WorkflowJob` / `AiJob` route through the governed
/// Workflow-Engine (LAW-CORPUS-PARITY-004). No variant ever describes a
/// process-local hidden call or a localhost-as-authority path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionClass {
    PureProjection,
    WriteBox,
    WorkflowJob,
    AiJob,
}

impl ExecutionClass {
    pub fn as_token(self) -> &'static str {
        match self {
            ExecutionClass::PureProjection => "pure_projection",
            ExecutionClass::WriteBox => "write_box",
            ExecutionClass::WorkflowJob => "workflow_job",
            ExecutionClass::AiJob => "ai_job",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "pure_projection" => Ok(ExecutionClass::PureProjection),
            "write_box" => Ok(ExecutionClass::WriteBox),
            "workflow_job" => Ok(ExecutionClass::WorkflowJob),
            "ai_job" => Ok(ExecutionClass::AiJob),
            other => Err(AtelierError::Validation(format!(
                "unknown execution_class token: {other}"
            ))),
        }
    }

    /// Whether this class performs external/governed-execution work and so
    /// MUST declare a Workflow-Engine route (LAW-CORPUS-PARITY-004). A
    /// `WorkflowJob`/`AiJob` descriptor without an event-evidence class is a
    /// parity defect surfaced as BLOCKED.
    pub fn requires_governed_execution(self) -> bool {
        matches!(self, ExecutionClass::WorkflowJob | ExecutionClass::AiJob)
    }
}

/// Reason a command carries a BLOCKED anchor record (10.19.4 BLOCKED-002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockedReason {
    /// No live ModelManual entry covers this command.
    NoManualAnchor,
    /// The handler currently performs ungoverned external execution.
    UngovernedExecution,
    /// External work without a required capability gate.
    NoCapabilityGate,
    /// Unbounded foreground interaction (violates HBR-QUIET).
    ForegroundUnbounded,
    /// No typed receipt shape, or a receipt that can leak secrets.
    NoTypedReceipt,
    /// No EventLedger / Flight Recorder evidence class.
    NoEventEvidence,
}

impl BlockedReason {
    pub fn as_token(self) -> &'static str {
        match self {
            BlockedReason::NoManualAnchor => "no_manual_anchor",
            BlockedReason::UngovernedExecution => "ungoverned_execution",
            BlockedReason::NoCapabilityGate => "no_capability_gate",
            BlockedReason::ForegroundUnbounded => "foreground_unbounded",
            BlockedReason::NoTypedReceipt => "no_typed_receipt",
            BlockedReason::NoEventEvidence => "no_event_evidence",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "no_manual_anchor" => Ok(BlockedReason::NoManualAnchor),
            "ungoverned_execution" => Ok(BlockedReason::UngovernedExecution),
            "no_capability_gate" => Ok(BlockedReason::NoCapabilityGate),
            "foreground_unbounded" => Ok(BlockedReason::ForegroundUnbounded),
            "no_typed_receipt" => Ok(BlockedReason::NoTypedReceipt),
            "no_event_evidence" => Ok(BlockedReason::NoEventEvidence),
            other => Err(AtelierError::Validation(format!(
                "unknown blocked_reason token: {other}"
            ))),
        }
    }
}

/// Sentinel manual-anchor value meaning "no valid product anchor; see BLOCKED
/// record" (10.19.2 `manual_anchor`, 10.19.4). Stored as the literal
/// `manual_anchor` column value when a command is blocked.
pub const MANUAL_ANCHOR_BLOCKED: &str = "BLOCKED";

/// One action-catalog parity descriptor (10.19.2, `PRIM-CommandCorpusEntryV1`).
///
/// This is the projection of a single corpus command into a typed descriptor.
/// `capabilities` is an explicit list (empty means "explicitly no capability",
/// never absent, per 10.19.2). `manual_anchor` is either a live ModelManual id
/// or [`MANUAL_ANCHOR_BLOCKED`]. All fields are storage-portable handles/ids:
/// no drive-letter, user-profile, or machine-local path appears in any field.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandCorpusEntry {
    pub entry_id: Uuid,
    /// Stable kebab/dotted action id, e.g. `media.download`, `version.commit`.
    pub action_id: String,
    pub corpus_source: CorpusSource,
    /// Owning subsystem/module id; MUST be a real backend owner, never "ui".
    pub owner: String,
    /// Actor classes that MAY invoke (e.g. `operator`, `model`, `mechanical`).
    pub actor_eligibility: Vec<String>,
    /// Typed input schema reference id (no free-form blob).
    pub params_schema_ref: String,
    pub input_schema_version: i32,
    /// Required capability set; empty list is meaningful (explicitly none).
    pub capabilities: Vec<String>,
    pub execution_class: ExecutionClass,
    /// Typed receipt/output schema id emitted on success.
    pub receipt_shape: String,
    /// Enumerated typed error variants + recovery instruction per variant.
    pub errors: Vec<CorpusErrorVariant>,
    /// True ONLY if the command unavoidably requires foreground interaction.
    pub foreground_flag: bool,
    /// Live ModelManual command id, or [`MANUAL_ANCHOR_BLOCKED`].
    pub manual_anchor: String,
    /// EventLedger event type(s) / Flight Recorder span(s) the command emits.
    pub evidence_class: Vec<String>,
    pub created_at_utc: DateTime<Utc>,
    pub updated_at_utc: DateTime<Utc>,
}

/// One enumerated error variant for a catalog command (10.19.2 `errors`).
/// A denied or failed invocation MUST yield a typed error, never an untyped
/// throw; each variant carries its own recovery instruction.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorpusErrorVariant {
    pub code: String,
    pub recovery_instruction: String,
}

/// Input to register (create-or-update) a catalog descriptor.
#[derive(Clone, Debug)]
pub struct UpsertCommandCorpusEntry {
    pub action_id: String,
    pub corpus_source: CorpusSource,
    pub owner: String,
    pub actor_eligibility: Vec<String>,
    pub params_schema_ref: String,
    pub input_schema_version: i32,
    pub capabilities: Vec<String>,
    pub execution_class: ExecutionClass,
    pub receipt_shape: String,
    pub errors: Vec<CorpusErrorVariant>,
    pub foreground_flag: bool,
    /// Live ModelManual id, or [`MANUAL_ANCHOR_BLOCKED`] when no anchor exists.
    pub manual_anchor: String,
    pub evidence_class: Vec<String>,
}

/// Input to record one command-corpus invocation evidence event. This is a
/// ref-only receipt bridge: it does not execute the command and does not store
/// raw params/results. Dispatchers call this after their own execution path has
/// produced typed input/receipt refs and before exposing the invocation as
/// auditable evidence.
#[derive(Clone, Debug)]
pub struct RecordCommandCorpusInvocation {
    pub invocation_id: Uuid,
    pub action_id: String,
    pub actor_id: String,
    /// Must exactly match one event family declared in the descriptor's
    /// `evidence_class`.
    pub evidence_event_family: String,
    pub input_ref: String,
    pub receipt_ref: String,
}

/// Ref-only receipt returned after an invocation evidence event is appended.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandCorpusInvocationReceipt {
    pub invocation_id: Uuid,
    pub action_id: String,
    pub actor_id: String,
    pub evidence_event_family: String,
    pub execution_class: ExecutionClass,
    pub receipt_shape: String,
    pub input_ref: String,
    pub receipt_ref: String,
    pub accepted_at_utc: DateTime<Utc>,
}

/// A durable BLOCKED-anchor record (10.19.4, `PRIM-CommandCorpusBlockedRecordV1`).
///
/// Persists until the underlying anchor is supplied or the command leaves the
/// corpus (BLOCKED-003). BLOCKED is a parity state, not an execution grant
/// (BLOCKED-004): the entry remains non-invocable while a record exists.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandCorpusBlockedRecord {
    pub blocked_id: Uuid,
    pub action_id: String,
    pub blocked_reason: BlockedReason,
    /// The corpus source(s) the command was discovered in.
    pub discovered_in: CorpusSource,
    pub recovery_instruction: String,
    pub first_seen_utc: DateTime<Utc>,
    pub last_seen_utc: DateTime<Utc>,
}

/// A single defect row inside a parity report (10.19.3 PARITY-MANUAL-004).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParityDefectRow {
    pub action_id: String,
    /// Kind of defect, e.g. `blocked`, `orphaned_manual`,
    /// `missing_event_evidence`, `ungoverned_external`.
    pub defect_kind: String,
    pub detail: String,
}

/// A deterministic parity report over the corpus (10.19.3,
/// `PRIM-CommandCorpusParityReportV1`). This is a build artifact projection,
/// never authority and never written into `.GOV` (EVIDENCE-002/004).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandCorpusParityReport {
    pub report_id: Uuid,
    pub total_corpus: i64,
    pub covered_count: i64,
    pub blocked_count: i64,
    /// ModelManual entries naming an `action_id` absent from the corpus
    /// (PARITY-MANUAL-002). These are passed in by the mechanical manual scan.
    pub orphaned_manual_count: i64,
    pub defects: Vec<ParityDefectRow>,
    pub created_at_utc: DateTime<Utc>,
}

fn parse_str_list(value: serde_json::Value) -> AtelierResult<Vec<String>> {
    serde_json::from_value::<Vec<String>>(value)
        .map_err(|e| AtelierError::Validation(format!("expected JSON string array: {e}")))
}

fn parse_error_variants(value: serde_json::Value) -> AtelierResult<Vec<CorpusErrorVariant>> {
    serde_json::from_value::<Vec<CorpusErrorVariant>>(value)
        .map_err(|e| AtelierError::Validation(format!("invalid errors array: {e}")))
}

fn entry_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<CommandCorpusEntry> {
    let corpus_source: String = row.get("corpus_source");
    let execution_class: String = row.get("execution_class");
    Ok(CommandCorpusEntry {
        entry_id: row.get("entry_id"),
        action_id: row.get("action_id"),
        corpus_source: CorpusSource::from_token(&corpus_source)?,
        owner: row.get("owner"),
        actor_eligibility: parse_str_list(row.get("actor_eligibility"))?,
        params_schema_ref: row.get("params_schema_ref"),
        input_schema_version: row.get("input_schema_version"),
        capabilities: parse_str_list(row.get("capabilities"))?,
        execution_class: ExecutionClass::from_token(&execution_class)?,
        receipt_shape: row.get("receipt_shape"),
        errors: parse_error_variants(row.get("errors"))?,
        foreground_flag: row.get("foreground_flag"),
        manual_anchor: row.get("manual_anchor"),
        evidence_class: parse_str_list(row.get("evidence_class"))?,
        created_at_utc: row.get("created_at_utc"),
        updated_at_utc: row.get("updated_at_utc"),
    })
}

fn blocked_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<CommandCorpusBlockedRecord> {
    let blocked_reason: String = row.get("blocked_reason");
    let discovered_in: String = row.get("discovered_in");
    Ok(CommandCorpusBlockedRecord {
        blocked_id: row.get("blocked_id"),
        action_id: row.get("action_id"),
        blocked_reason: BlockedReason::from_token(&blocked_reason)?,
        discovered_in: CorpusSource::from_token(&discovered_in)?,
        recovery_instruction: row.get("recovery_instruction"),
        first_seen_utc: row.get("first_seen_utc"),
        last_seen_utc: row.get("last_seen_utc"),
    })
}

fn validate_invocation_ref(field: &str, value: &str) -> AtelierResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AtelierError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.contains("localhost")
        || lower.contains("127.0.0.1")
        || trimmed.contains('\\')
        || trimmed.starts_with('/')
        || trimmed.contains(":\\")
    {
        return Err(AtelierError::Validation(format!(
            "{field} must be a portable opaque ref, not a URL or machine-local path"
        )));
    }
    Ok(())
}

const ENTRY_COLUMNS: &str = "entry_id, action_id, corpus_source, owner, actor_eligibility, \
                             params_schema_ref, input_schema_version, capabilities, \
                             execution_class, receipt_shape, errors, foreground_flag, \
                             manual_anchor, evidence_class, created_at_utc, updated_at_utc";

const BLOCKED_COLUMNS: &str = "blocked_id, action_id, blocked_reason, discovered_in, \
                               recovery_instruction, first_seen_utc, last_seen_utc";

impl AtelierStore {
    /// Register (create or update) one action-catalog descriptor (10.19.2).
    ///
    /// Upsert key is `action_id` (the corpus is a single typed catalog;
    /// LAW-CORPUS-PARITY-001 forbids competing lists, so each action id appears
    /// exactly once). Re-projecting the same command updates its descriptor in
    /// place. The descriptor's typed fields are validated server-side: an
    /// external-execution command (`workflow_job`/`ai_job`) MUST declare at
    /// least one `evidence_class` (LAW-CORPUS-PARITY-004 / EVIDENCE-001); a
    /// `BLOCKED` manual anchor is allowed but does not by itself create the
    /// BLOCKED record (use [`AtelierStore::record_blocked_command`]). Emits
    /// `CORPUS_ENTRY_UPSERTED`. The event echoes only ids/flags, never raw
    /// param or receipt values (EVIDENCE-003 redaction).
    pub async fn upsert_command_corpus_entry(
        &self,
        input: &UpsertCommandCorpusEntry,
    ) -> AtelierResult<CommandCorpusEntry> {
        if input.action_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "action_id must not be empty".into(),
            ));
        }
        if input.owner.trim().is_empty() || input.owner.trim().eq_ignore_ascii_case("ui") {
            return Err(AtelierError::Validation(
                "owner must be a real backend owner, never empty or 'ui'".into(),
            ));
        }
        if input.params_schema_ref.trim().is_empty() {
            return Err(AtelierError::Validation(
                "params_schema_ref must reference a typed input schema".into(),
            ));
        }
        if input.receipt_shape.trim().is_empty() {
            return Err(AtelierError::Validation(
                "receipt_shape must reference a typed receipt schema".into(),
            ));
        }
        if input.execution_class.requires_governed_execution() && input.evidence_class.is_empty() {
            return Err(AtelierError::Validation(format!(
                "execution_class {} requires a governed-execution evidence_class \
                 (Workflow-Engine event/span); declare one or mark the command BLOCKED",
                input.execution_class.as_token()
            )));
        }

        let actor_json = serde_json::to_value(&input.actor_eligibility)
            .map_err(|e| AtelierError::Validation(format!("actor_eligibility: {e}")))?;
        let caps_json = serde_json::to_value(&input.capabilities)
            .map_err(|e| AtelierError::Validation(format!("capabilities: {e}")))?;
        let errors_json = serde_json::to_value(&input.errors)
            .map_err(|e| AtelierError::Validation(format!("errors: {e}")))?;
        let evidence_json = serde_json::to_value(&input.evidence_class)
            .map_err(|e| AtelierError::Validation(format!("evidence_class: {e}")))?;

        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_command_corpus_entry
                 (action_id, corpus_source, owner, actor_eligibility, params_schema_ref,
                  input_schema_version, capabilities, execution_class, receipt_shape,
                  errors, foreground_flag, manual_anchor, evidence_class)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
               ON CONFLICT (action_id) DO UPDATE SET
                  corpus_source        = EXCLUDED.corpus_source,
                  owner                = EXCLUDED.owner,
                  actor_eligibility    = EXCLUDED.actor_eligibility,
                  params_schema_ref    = EXCLUDED.params_schema_ref,
                  input_schema_version = EXCLUDED.input_schema_version,
                  capabilities         = EXCLUDED.capabilities,
                  execution_class      = EXCLUDED.execution_class,
                  receipt_shape        = EXCLUDED.receipt_shape,
                  errors               = EXCLUDED.errors,
                  foreground_flag      = EXCLUDED.foreground_flag,
                  manual_anchor        = EXCLUDED.manual_anchor,
                  evidence_class       = EXCLUDED.evidence_class,
                  updated_at_utc       = NOW()
               RETURNING {ENTRY_COLUMNS}"#
        ))
        .bind(&input.action_id)
        .bind(input.corpus_source.as_token())
        .bind(&input.owner)
        .bind(actor_json)
        .bind(&input.params_schema_ref)
        .bind(input.input_schema_version)
        .bind(caps_json)
        .bind(input.execution_class.as_token())
        .bind(&input.receipt_shape)
        .bind(errors_json)
        .bind(input.foreground_flag)
        .bind(&input.manual_anchor)
        .bind(evidence_json)
        .fetch_one(&mut *tx)
        .await?;

        let entry = entry_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            CORPUS_ENTRY_UPSERTED,
            "atelier_command_corpus_entry",
            &entry.action_id,
            serde_json::json!({
                "entry_id": entry.entry_id,
                "action_id": entry.action_id,
                "corpus_source": entry.corpus_source.as_token(),
                "owner": entry.owner,
                "execution_class": entry.execution_class.as_token(),
                "foreground_flag": entry.foreground_flag,
                "manual_anchor": entry.manual_anchor,
                "capability_count": entry.capabilities.len(),
                "evidence_count": entry.evidence_class.len(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(entry)
    }

    /// Fetch one catalog descriptor by its stable `action_id`.
    pub async fn get_command_corpus_entry(
        &self,
        action_id: &str,
    ) -> AtelierResult<Option<CommandCorpusEntry>> {
        let row = sqlx::query(&format!(
            "SELECT {ENTRY_COLUMNS} FROM atelier_command_corpus_entry WHERE action_id = $1"
        ))
        .bind(action_id)
        .fetch_optional(self.pool())
        .await?;
        match row {
            Some(r) => Ok(Some(entry_from_row(&r)?)),
            None => Ok(None),
        }
    }

    /// List catalog descriptors, ordered by `action_id`. Optionally filter to a
    /// single owning subsystem for the Dev Command Center projection.
    pub async fn list_command_corpus_entries(
        &self,
        owner: Option<&str>,
    ) -> AtelierResult<Vec<CommandCorpusEntry>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {ENTRY_COLUMNS}
               FROM atelier_command_corpus_entry
               WHERE ($1::TEXT IS NULL OR owner = $1)
               ORDER BY action_id ASC"#
        ))
        .bind(owner)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(entry_from_row).collect()
    }

    /// Bind a catalog entry's `manual_anchor` to a live ModelManual command id
    /// (10.19.3 PARITY-MANUAL-001 case (a)). This is the inverse of marking a
    /// command BLOCKED: when a real anchor is supplied, this method points the
    /// descriptor at it AND clears any open BLOCKED record whose reason was the
    /// missing manual anchor, in one transaction. Emits `CORPUS_ENTRY_ANCHORED`
    /// (and `CORPUS_BLOCKED_CLEARED` when a record is cleared).
    pub async fn anchor_command_manual(
        &self,
        action_id: &str,
        manual_command_id: &str,
    ) -> AtelierResult<CommandCorpusEntry> {
        if manual_command_id.trim().is_empty() || manual_command_id == MANUAL_ANCHOR_BLOCKED {
            return Err(AtelierError::Validation(
                "manual_command_id must be a real ModelManual id, not empty or BLOCKED".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(&format!(
            r#"UPDATE atelier_command_corpus_entry
               SET manual_anchor = $2, updated_at_utc = NOW()
               WHERE action_id = $1
               RETURNING {ENTRY_COLUMNS}"#
        ))
        .bind(action_id)
        .bind(manual_command_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AtelierError::NotFound(format!("command corpus entry action_id={action_id}"))
        })?;

        // Clearing the no-manual-anchor block is part of supplying the anchor.
        let cleared: Option<Uuid> = sqlx::query_scalar(
            r#"DELETE FROM atelier_command_corpus_blocked
               WHERE action_id = $1 AND blocked_reason = 'no_manual_anchor'
               RETURNING blocked_id"#,
        )
        .bind(action_id)
        .fetch_optional(&mut *tx)
        .await?;

        let entry = entry_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            CORPUS_ENTRY_ANCHORED,
            "atelier_command_corpus_entry",
            &entry.action_id,
            serde_json::json!({
                "entry_id": entry.entry_id,
                "action_id": entry.action_id,
                "manual_anchor": entry.manual_anchor,
            }),
        )
        .await?;
        if let Some(blocked_id) = cleared {
            self.record_event_in_tx(
                &mut tx,
                CORPUS_BLOCKED_CLEARED,
                "atelier_command_corpus_blocked",
                action_id,
                serde_json::json!({
                    "blocked_id": blocked_id,
                    "action_id": action_id,
                    "blocked_reason": BlockedReason::NoManualAnchor.as_token(),
                    "cleared_by": "manual_anchor_supplied",
                }),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(entry)
    }

    /// Record (open or refresh) a durable BLOCKED record for a command
    /// (10.19.4). A command with no valid product anchor MUST carry a BLOCKED
    /// record rather than be silently omitted (BLOCKED-001). The upsert key is
    /// `(action_id, blocked_reason)` so a command can carry several distinct
    /// block reasons at once; re-recording the same `(action_id, reason)`
    /// refreshes `last_seen_utc` and keeps the original `first_seen_utc`
    /// (BLOCKED-002 first/last-seen semantics). Also pins the entry's
    /// `manual_anchor` to the `BLOCKED` sentinel when the reason is a missing
    /// manual anchor, so the descriptor and record agree. Emits
    /// `CORPUS_BLOCKED_RECORDED`. Idempotent.
    pub async fn record_blocked_command(
        &self,
        action_id: &str,
        reason: BlockedReason,
        discovered_in: CorpusSource,
        recovery_instruction: &str,
    ) -> AtelierResult<CommandCorpusBlockedRecord> {
        if action_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "action_id must not be empty".into(),
            ));
        }
        if recovery_instruction.trim().is_empty() {
            return Err(AtelierError::Validation(
                "recovery_instruction must not be empty (BLOCKED-002)".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;

        let row = sqlx::query(&format!(
            r#"INSERT INTO atelier_command_corpus_blocked
                 (action_id, blocked_reason, discovered_in, recovery_instruction)
               VALUES ($1, $2, $3, $4)
               ON CONFLICT (action_id, blocked_reason) DO UPDATE SET
                  discovered_in        = EXCLUDED.discovered_in,
                  recovery_instruction = EXCLUDED.recovery_instruction,
                  last_seen_utc        = NOW()
               RETURNING {BLOCKED_COLUMNS}"#
        ))
        .bind(action_id)
        .bind(reason.as_token())
        .bind(discovered_in.as_token())
        .bind(recovery_instruction)
        .fetch_one(&mut *tx)
        .await?;

        // Keep the descriptor's manual_anchor consistent with the block when
        // the block is specifically the missing manual anchor. Only touches an
        // existing entry; a corpus command may be blocked before its descriptor
        // is projected, so a missing entry is not an error here.
        if reason == BlockedReason::NoManualAnchor {
            sqlx::query(
                r#"UPDATE atelier_command_corpus_entry
                   SET manual_anchor = $2, updated_at_utc = NOW()
                   WHERE action_id = $1"#,
            )
            .bind(action_id)
            .bind(MANUAL_ANCHOR_BLOCKED)
            .execute(&mut *tx)
            .await?;
        }

        let record = blocked_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            CORPUS_BLOCKED_RECORDED,
            "atelier_command_corpus_blocked",
            &record.action_id,
            serde_json::json!({
                "blocked_id": record.blocked_id,
                "action_id": record.action_id,
                "blocked_reason": record.blocked_reason.as_token(),
                "discovered_in": record.discovered_in.as_token(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(record)
    }

    /// List all open BLOCKED records, newest activity first. Optionally filter
    /// to one command. These remain visible in the Dev Command Center
    /// projection until the anchor is supplied (BLOCKED-003).
    pub async fn list_blocked_commands(
        &self,
        action_id: Option<&str>,
    ) -> AtelierResult<Vec<CommandCorpusBlockedRecord>> {
        let rows = sqlx::query(&format!(
            r#"SELECT {BLOCKED_COLUMNS}
               FROM atelier_command_corpus_blocked
               WHERE ($1::TEXT IS NULL OR action_id = $1)
               ORDER BY last_seen_utc DESC, action_id ASC"#
        ))
        .bind(action_id)
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(blocked_from_row).collect()
    }

    /// Read-only dispatch guard for BLOCKED-004. Callers that execute a
    /// catalog command can use this immediately before dispatch: any open
    /// BLOCKED record, or the descriptor's `manual_anchor=BLOCKED` sentinel,
    /// denies invocation with a typed validation error instead of silently
    /// treating BLOCKED as an execution grant.
    pub async fn guard_command_corpus_invocable(
        &self,
        action_id: &str,
    ) -> AtelierResult<CommandCorpusEntry> {
        if action_id.trim().is_empty() {
            return Err(AtelierError::Validation(
                "action_id must not be empty".into(),
            ));
        }

        let blocked = self.list_blocked_commands(Some(action_id)).await?;
        let entry = self.get_command_corpus_entry(action_id).await?;

        if !blocked.is_empty() {
            let mut details = Vec::new();
            if let Some(entry) = &entry {
                if entry.manual_anchor == MANUAL_ANCHOR_BLOCKED {
                    details.push("manual_anchor=BLOCKED".to_string());
                }
            }
            details.extend(blocked.iter().map(|record| {
                format!(
                    "{}: {}",
                    record.blocked_reason.as_token(),
                    record.recovery_instruction
                )
            }));

            return Err(AtelierError::Validation(format!(
                "command_blocked: action_id={action_id} is not invocable while BLOCKED ({})",
                details.join("; ")
            )));
        }

        let entry = entry.ok_or_else(|| {
            AtelierError::NotFound(format!("command corpus entry action_id={action_id}"))
        })?;
        if entry.manual_anchor == MANUAL_ANCHOR_BLOCKED {
            return Err(AtelierError::Validation(format!(
                "command_blocked: action_id={action_id} is not invocable while manual_anchor=BLOCKED"
            )));
        }

        Ok(entry)
    }

    /// Record a command invocation evidence event of the descriptor-declared
    /// event family. This is a dispatch-side proof hook, not an executor: it
    /// first applies [`AtelierStore::guard_command_corpus_invocable`], then
    /// verifies that the requested event family appears in the descriptor's
    /// `evidence_class`, and only then appends a canonical EventLedger-backed
    /// atelier domain event. Payloads are id/ref/count-only so invocation proof
    /// cannot leak params, receipts, URLs, local paths, or secrets.
    pub async fn record_command_corpus_invocation(
        &self,
        input: &RecordCommandCorpusInvocation,
    ) -> AtelierResult<CommandCorpusInvocationReceipt> {
        if input.invocation_id == Uuid::nil() {
            return Err(AtelierError::Validation(
                "invocation_id must not be nil".into(),
            ));
        }
        let action_id = input.action_id.trim();
        if action_id.is_empty() {
            return Err(AtelierError::Validation(
                "action_id must not be empty".into(),
            ));
        }
        let actor_id = input.actor_id.trim();
        if actor_id.is_empty() {
            return Err(AtelierError::Validation(
                "actor_id must not be empty".into(),
            ));
        }
        let evidence_event_family = input.evidence_event_family.trim();
        if evidence_event_family.is_empty() {
            return Err(AtelierError::Validation(
                "evidence_event_family must not be empty".into(),
            ));
        }
        validate_invocation_ref("input_ref", &input.input_ref)?;
        validate_invocation_ref("receipt_ref", &input.receipt_ref)?;

        let entry = self.guard_command_corpus_invocable(action_id).await?;
        if !entry
            .evidence_class
            .iter()
            .any(|declared| declared.as_str() == evidence_event_family)
        {
            return Err(AtelierError::Validation(format!(
                "evidence_event_family {evidence_event_family} is not in the declared evidence_class for action_id={action_id}"
            )));
        }

        let receipt = CommandCorpusInvocationReceipt {
            invocation_id: input.invocation_id,
            action_id: action_id.to_string(),
            actor_id: actor_id.to_string(),
            evidence_event_family: evidence_event_family.to_string(),
            execution_class: entry.execution_class,
            receipt_shape: entry.receipt_shape.clone(),
            input_ref: input.input_ref.trim().to_string(),
            receipt_ref: input.receipt_ref.trim().to_string(),
            accepted_at_utc: Utc::now(),
        };

        self.record_event(
            &receipt.evidence_event_family,
            "atelier_command_corpus_invocation",
            &receipt.invocation_id.to_string(),
            serde_json::json!({
                "invocation_id": receipt.invocation_id,
                "entry_id": entry.entry_id,
                "action_id": receipt.action_id,
                "actor_id": receipt.actor_id,
                "corpus_source": entry.corpus_source.as_token(),
                "owner": entry.owner,
                "execution_class": entry.execution_class.as_token(),
                "manual_anchor": entry.manual_anchor,
                "evidence_event_family": receipt.evidence_event_family,
                "input_ref": receipt.input_ref,
                "receipt_shape": receipt.receipt_shape,
                "receipt_ref": receipt.receipt_ref,
                "capability_count": entry.capabilities.len(),
                "declared_evidence_count": entry.evidence_class.len(),
            }),
        )
        .await?;
        Ok(receipt)
    }

    /// Materialize a deterministic parity report over the catalog (10.19.3
    /// PARITY-MANUAL-004). The scan is mechanical, not a judgement:
    ///   * `total_corpus` = number of catalog descriptors.
    ///   * `blocked_count` = distinct `action_id`s carrying any BLOCKED record.
    ///   * `covered_count` = descriptors whose `manual_anchor` is a live id
    ///     (not the `BLOCKED` sentinel) AND that carry no open BLOCKED record.
    ///   * per-defect rows: every blocked command (reason carried in `detail`),
    ///     plus every external-execution descriptor lacking an evidence class
    ///     (defensive: the upsert guard already forbids this), plus every
    ///     orphaned manual command id supplied by the caller's manual scan
    ///     (PARITY-MANUAL-002 -- a manual entry naming an `action_id` absent
    ///     from the corpus).
    ///
    /// `orphaned_manual_action_ids` is the output of the mechanical ModelManual
    /// scan (manual command ids whose named `action_id` is not in the corpus);
    /// this module does not own the manual surface, so the caller supplies it.
    /// The report is a build-artifact projection, never authority and never
    /// written to `.GOV` (EVIDENCE-002/004). Emits `CORPUS_PARITY_REPORTED`.
    pub async fn build_command_corpus_parity_report(
        &self,
        orphaned_manual_action_ids: &[String],
    ) -> AtelierResult<CommandCorpusParityReport> {
        let entries = self.list_command_corpus_entries(None).await?;
        let blocked = self.list_blocked_commands(None).await?;

        let total_corpus = entries.len() as i64;

        // Distinct action_ids that are blocked.
        let mut blocked_ids: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for b in &blocked {
            blocked_ids.insert(b.action_id.as_str());
        }
        let blocked_count = blocked_ids.len() as i64;

        let mut defects: Vec<ParityDefectRow> = Vec::new();

        // Defect rows: each open BLOCKED record (carry the reason in detail).
        for b in &blocked {
            defects.push(ParityDefectRow {
                action_id: b.action_id.clone(),
                defect_kind: "blocked".to_string(),
                detail: format!(
                    "{}: {}",
                    b.blocked_reason.as_token(),
                    b.recovery_instruction
                ),
            });
        }

        // Covered = live manual anchor AND not blocked. Also defensively flag
        // any external-execution descriptor missing an evidence class.
        let mut covered_count: i64 = 0;
        for e in &entries {
            let is_blocked = blocked_ids.contains(e.action_id.as_str());
            let has_live_anchor = !is_blocked && e.manual_anchor != MANUAL_ANCHOR_BLOCKED;
            if has_live_anchor {
                covered_count += 1;
            }
            if e.execution_class.requires_governed_execution() && e.evidence_class.is_empty() {
                defects.push(ParityDefectRow {
                    action_id: e.action_id.clone(),
                    defect_kind: "missing_event_evidence".to_string(),
                    detail: format!(
                        "execution_class {} requires a Workflow-Engine evidence_class",
                        e.execution_class.as_token()
                    ),
                });
            }
        }

        // Orphaned manual entries (PARITY-MANUAL-002): manual ids whose named
        // action_id has no corpus descriptor.
        let corpus_ids: std::collections::BTreeSet<&str> =
            entries.iter().map(|e| e.action_id.as_str()).collect();
        let mut orphaned_manual_count: i64 = 0;
        for orphan in orphaned_manual_action_ids {
            if !corpus_ids.contains(orphan.as_str()) {
                orphaned_manual_count += 1;
                defects.push(ParityDefectRow {
                    action_id: orphan.clone(),
                    defect_kind: "orphaned_manual".to_string(),
                    detail: "ModelManual advertises an action_id absent from the corpus"
                        .to_string(),
                });
            }
        }

        let defects_json = serde_json::to_value(&defects)
            .map_err(|e| AtelierError::Validation(format!("defects: {e}")))?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_command_corpus_parity_report
                 (total_corpus, covered_count, blocked_count, orphaned_manual_count, defects)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING report_id, total_corpus, covered_count, blocked_count,
                         orphaned_manual_count, defects, created_at_utc"#,
        )
        .bind(total_corpus)
        .bind(covered_count)
        .bind(blocked_count)
        .bind(orphaned_manual_count)
        .bind(defects_json)
        .fetch_one(&mut *tx)
        .await?;

        let report = CommandCorpusParityReport {
            report_id: row.get("report_id"),
            total_corpus: row.get("total_corpus"),
            covered_count: row.get("covered_count"),
            blocked_count: row.get("blocked_count"),
            orphaned_manual_count: row.get("orphaned_manual_count"),
            defects,
            created_at_utc: row.get("created_at_utc"),
        };

        self.record_event_in_tx(
            &mut tx,
            CORPUS_PARITY_REPORTED,
            "atelier_command_corpus_parity_report",
            &report.report_id.to_string(),
            serde_json::json!({
                "report_id": report.report_id,
                "total_corpus": report.total_corpus,
                "covered_count": report.covered_count,
                "blocked_count": report.blocked_count,
                "orphaned_manual_count": report.orphaned_manual_count,
                "defect_count": report.defects.len(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(report)
    }

    /// Fetch the most recent parity report, if one has been materialized.
    pub async fn latest_command_corpus_parity_report(
        &self,
    ) -> AtelierResult<Option<CommandCorpusParityReport>> {
        let row = sqlx::query(
            r#"SELECT report_id, total_corpus, covered_count, blocked_count,
                      orphaned_manual_count, defects, created_at_utc
               FROM atelier_command_corpus_parity_report
               ORDER BY created_at_utc DESC, report_id DESC
               LIMIT 1"#,
        )
        .fetch_optional(self.pool())
        .await?;
        match row {
            None => Ok(None),
            Some(r) => {
                let defects: Vec<ParityDefectRow> = serde_json::from_value(r.get("defects"))
                    .map_err(|e| {
                        AtelierError::Validation(format!("invalid defects payload: {e}"))
                    })?;
                Ok(Some(CommandCorpusParityReport {
                    report_id: r.get("report_id"),
                    total_corpus: r.get("total_corpus"),
                    covered_count: r.get("covered_count"),
                    blocked_count: r.get("blocked_count"),
                    orphaned_manual_count: r.get("orphaned_manual_count"),
                    defects,
                    created_at_utc: r.get("created_at_utc"),
                }))
            }
        }
    }
}
