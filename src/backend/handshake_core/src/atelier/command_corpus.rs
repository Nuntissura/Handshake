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

use super::{reject_legacy_runtime_ref, AtelierError, AtelierResult, AtelierStore};

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

// ===========================================================================
// Model-Workflow-Diagnostics typed runtime surfaces (WP-KERNEL-005).
//
// Appended after the command-corpus parity contract. These are product/runtime
// surfaces for no-context models, never governance markdown. Storage is
// PostgreSQL only (AtelierStore::pool()); SQLite is forbidden (MT-004).
//
//   * MT-140 -- structured ErrorTaxonomy: 10 canonical diagnostics error classes,
//     each with a mandatory recovery hint, in atelier_diagnostics_error_taxonomy.
//   * MT-207 -- CKC WP-0118 model prompt-response matrix preserved as a DEFERRED
//     contract (prompt set + expected-response shape + scoring schema) with no
//     live scoring, in atelier_diagnostics_prompt_response_matrix.
//   * MT-166 -- Installer Reset And Orphan Evidence Projection: a read projection
//     over the existing atelier_reset_operation / atelier_orphan_manifest_item
//     tables (migration 0089) that surfaces reset/orphan behavior for model
//     diagnostics. No new table; emits a projection event over canonical rows.
//
// Event families are defined in `diagnostics_event_family` below. The parent
// (mod.rs event_family::ALL) folds these in; this module re-exports them.
// ===========================================================================

/// Model-Workflow-Diagnostics event families (MT-140 / MT-166 / MT-207).
/// Defined here so the parent folds these into [`super::event_family::ALL`] and
/// the MT-005 coverage check picks up diagnostics mutations.
pub mod diagnostics_event_family {
    /// A diagnostics error-taxonomy class was registered or refreshed (MT-140).
    pub const DIAGNOSTICS_ERROR_TAXONOMY_RECORDED: &str =
        "atelier.diagnostics.error_taxonomy_recorded";
    /// A WP-0118 prompt-response matrix entry was preserved as a deferred
    /// contract (MT-207).
    pub const DIAGNOSTICS_PROMPT_RESPONSE_MATRIX_RECORDED: &str =
        "atelier.diagnostics.prompt_response_matrix_recorded";
    /// A reset/orphan evidence projection was materialized for model
    /// diagnostics (MT-166).
    pub const DIAGNOSTICS_RESET_ORPHAN_PROJECTED: &str =
        "atelier.diagnostics.reset_orphan_projected";

    /// All diagnostics event families (for parity/coverage folding).
    pub const ALL: &[&str] = &[
        DIAGNOSTICS_ERROR_TAXONOMY_RECORDED,
        DIAGNOSTICS_PROMPT_RESPONSE_MATRIX_RECORDED,
        DIAGNOSTICS_RESET_ORPHAN_PROJECTED,
    ];
}

pub use diagnostics_event_family::{
    DIAGNOSTICS_ERROR_TAXONOMY_RECORDED, DIAGNOSTICS_PROMPT_RESPONSE_MATRIX_RECORDED,
    DIAGNOSTICS_RESET_ORPHAN_PROJECTED,
};

// ---------------------------------------------------------------------------
// MT-140: structured ErrorTaxonomy with 10 error classes + recovery hints.
// ---------------------------------------------------------------------------

/// The 10 canonical Model-Workflow-Diagnostics error classes. A diagnostics
/// failure always maps to exactly one of these classes, and every class carries
/// a recovery hint so a no-context model has an actionable next step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticsErrorClass {
    Validation,
    CapabilityDenied,
    MissingState,
    StaleLease,
    Timeout,
    ArtifactMissing,
    Parse,
    VisualMismatch,
    PackageGuard,
    StaleDocs,
}

impl DiagnosticsErrorClass {
    /// Every error class, in canonical order. Length is exactly 10.
    pub const ALL: &'static [DiagnosticsErrorClass] = &[
        DiagnosticsErrorClass::Validation,
        DiagnosticsErrorClass::CapabilityDenied,
        DiagnosticsErrorClass::MissingState,
        DiagnosticsErrorClass::StaleLease,
        DiagnosticsErrorClass::Timeout,
        DiagnosticsErrorClass::ArtifactMissing,
        DiagnosticsErrorClass::Parse,
        DiagnosticsErrorClass::VisualMismatch,
        DiagnosticsErrorClass::PackageGuard,
        DiagnosticsErrorClass::StaleDocs,
    ];

    /// Canonical stable database token (the `class` PK).
    pub fn as_token(self) -> &'static str {
        match self {
            DiagnosticsErrorClass::Validation => "validation",
            DiagnosticsErrorClass::CapabilityDenied => "capability_denied",
            DiagnosticsErrorClass::MissingState => "missing_state",
            DiagnosticsErrorClass::StaleLease => "stale_lease",
            DiagnosticsErrorClass::Timeout => "timeout",
            DiagnosticsErrorClass::ArtifactMissing => "artifact_missing",
            DiagnosticsErrorClass::Parse => "parse",
            DiagnosticsErrorClass::VisualMismatch => "visual_mismatch",
            DiagnosticsErrorClass::PackageGuard => "package_guard",
            DiagnosticsErrorClass::StaleDocs => "stale_docs",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "validation" => Ok(DiagnosticsErrorClass::Validation),
            "capability_denied" => Ok(DiagnosticsErrorClass::CapabilityDenied),
            "missing_state" => Ok(DiagnosticsErrorClass::MissingState),
            "stale_lease" => Ok(DiagnosticsErrorClass::StaleLease),
            "timeout" => Ok(DiagnosticsErrorClass::Timeout),
            "artifact_missing" => Ok(DiagnosticsErrorClass::ArtifactMissing),
            "parse" => Ok(DiagnosticsErrorClass::Parse),
            "visual_mismatch" => Ok(DiagnosticsErrorClass::VisualMismatch),
            "package_guard" => Ok(DiagnosticsErrorClass::PackageGuard),
            "stale_docs" => Ok(DiagnosticsErrorClass::StaleDocs),
            other => Err(AtelierError::Validation(format!(
                "unknown diagnostics error class token: {other}"
            ))),
        }
    }

    /// Human-readable description of what the error class means.
    pub fn description(self) -> &'static str {
        match self {
            DiagnosticsErrorClass::Validation => {
                "Input failed a typed validation gate (shape, token, or constraint)."
            }
            DiagnosticsErrorClass::CapabilityDenied => {
                "The action was denied because a required capability was not granted."
            }
            DiagnosticsErrorClass::MissingState => {
                "A required prerequisite record or runtime state was absent."
            }
            DiagnosticsErrorClass::StaleLease => {
                "A held lease expired or was superseded before the action committed."
            }
            DiagnosticsErrorClass::Timeout => {
                "The operation exceeded its deadline before producing a result."
            }
            DiagnosticsErrorClass::ArtifactMissing => {
                "A referenced artifact could not be resolved in the ArtifactStore."
            }
            DiagnosticsErrorClass::Parse => {
                "A payload could not be parsed into its expected typed structure."
            }
            DiagnosticsErrorClass::VisualMismatch => {
                "A visual/structural comparison failed against the expected baseline."
            }
            DiagnosticsErrorClass::PackageGuard => {
                "A package/integrity guard rejected the operation to protect the workspace."
            }
            DiagnosticsErrorClass::StaleDocs => {
                "Documentation or a manual anchor drifted from the live product surface."
            }
        }
    }

    /// Actionable recovery hint for a no-context model that hit this class.
    pub fn recovery_hint(self) -> &'static str {
        match self {
            DiagnosticsErrorClass::Validation => {
                "Inspect the rejected field in the error, correct the value to satisfy the typed \
                 contract, and resubmit."
            }
            DiagnosticsErrorClass::CapabilityDenied => {
                "Request or register the required capability, then retry the action once the grant \
                 is recorded."
            }
            DiagnosticsErrorClass::MissingState => {
                "Create or re-run the prerequisite step that produces the missing record, then \
                 retry."
            }
            DiagnosticsErrorClass::StaleLease => {
                "Re-acquire a fresh lease and retry; do not reuse the expired lease handle."
            }
            DiagnosticsErrorClass::Timeout => {
                "Check the downstream job/queue health, increase the deadline if appropriate, and \
                 retry idempotently."
            }
            DiagnosticsErrorClass::ArtifactMissing => {
                "Re-materialize or re-import the artifact, confirm its content hash, then retry the \
                 reference."
            }
            DiagnosticsErrorClass::Parse => {
                "Validate the payload against its schema, fix the malformed input, and resubmit."
            }
            DiagnosticsErrorClass::VisualMismatch => {
                "Open the diff, decide whether to update the baseline or fix the regression, then \
                 re-run the comparison."
            }
            DiagnosticsErrorClass::PackageGuard => {
                "Resolve the integrity/guard violation (restore or repackage), then retry the \
                 guarded operation."
            }
            DiagnosticsErrorClass::StaleDocs => {
                "Refresh the manual/docs anchor to match the live surface, then re-run the \
                 coverage check."
            }
        }
    }
}

/// A persisted diagnostics error-taxonomy row.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticsErrorTaxonomyEntry {
    pub class: DiagnosticsErrorClass,
    pub description: String,
    pub recovery_hint: String,
    pub created_at_utc: DateTime<Utc>,
}

/// The full error-taxonomy catalog: every class + its description + recovery
/// hint, in canonical order. Always exactly 10 entries.
pub fn error_taxonomy_catalog() -> Vec<(DiagnosticsErrorClass, &'static str, &'static str)> {
    DiagnosticsErrorClass::ALL
        .iter()
        .map(|class| (*class, class.description(), class.recovery_hint()))
        .collect()
}

// ---------------------------------------------------------------------------
// MT-207: CKC WP-0118 prompt-response matrix preserved as a DEFERRED contract.
// ---------------------------------------------------------------------------

/// Deferral status of a preserved prompt-response matrix entry. The matrix is
/// preserved as a contract without live scoring, so entries default to
/// `Deferred`. `Active` exists only as a forward-compatible token for the future
/// WP that implements live scoring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromptResponseMatrixStatus {
    Deferred,
    Active,
}

impl PromptResponseMatrixStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            PromptResponseMatrixStatus::Deferred => "DEFERRED",
            PromptResponseMatrixStatus::Active => "ACTIVE",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "DEFERRED" => Ok(PromptResponseMatrixStatus::Deferred),
            "ACTIVE" => Ok(PromptResponseMatrixStatus::Active),
            other => Err(AtelierError::Validation(format!(
                "unknown prompt-response matrix status token: {other}"
            ))),
        }
    }
}

/// Input to preserve one prompt-response matrix entry.
#[derive(Clone, Debug)]
pub struct NewPromptResponseMatrixEntry {
    /// Stable kebab-case entry id (the PK).
    pub entry_id: String,
    /// The prompt text given to the model.
    pub prompt_text: String,
    /// The expected-response *shape* (a typed schema descriptor, not a value).
    pub expected_response_shape: serde_json::Value,
    /// The scoring schema that a future live-scoring WP would apply.
    pub scoring_schema: serde_json::Value,
    /// Preserved status; defaults to DEFERRED (no live scoring yet).
    pub status: PromptResponseMatrixStatus,
}

/// A persisted prompt-response matrix entry.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptResponseMatrixEntry {
    pub entry_id: String,
    pub prompt_text: String,
    pub expected_response_shape: serde_json::Value,
    pub scoring_schema: serde_json::Value,
    pub status: PromptResponseMatrixStatus,
    pub created_at_utc: DateTime<Utc>,
}

/// The preserved CKC WP-0118 model prompt-response matrix. Each entry carries a
/// real prompt, an expected-response shape, and the scoring schema a future WP
/// would apply. All entries are DEFERRED: this preserves the contract without
/// implementing live scoring early.
pub fn prompt_response_matrix_catalog() -> Vec<NewPromptResponseMatrixEntry> {
    vec![
        NewPromptResponseMatrixEntry {
            entry_id: "wp-0118.action-catalog-lookup".to_string(),
            prompt_text:
                "List the available atelier actions you can invoke and, for each, name its required \
                 capability and execution class."
                    .to_string(),
            expected_response_shape: serde_json::json!({
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["action_id", "capability", "execution_class"],
                    "properties": {
                        "action_id": { "type": "string" },
                        "capability": { "type": "string" },
                        "execution_class": { "type": "string" }
                    }
                }
            }),
            scoring_schema: serde_json::json!({
                "dimensions": [
                    { "key": "coverage", "weight": 0.5, "scale": "0_to_1" },
                    { "key": "capability_accuracy", "weight": 0.3, "scale": "0_to_1" },
                    { "key": "no_hallucinated_actions", "weight": 0.2, "scale": "0_to_1" }
                ],
                "pass_threshold": 0.8
            }),
            status: PromptResponseMatrixStatus::Deferred,
        },
        NewPromptResponseMatrixEntry {
            entry_id: "wp-0118.error-recovery-routing".to_string(),
            prompt_text:
                "An action failed with error class `stale_lease`. Describe the correct recovery \
                 step before retrying."
                    .to_string(),
            expected_response_shape: serde_json::json!({
                "type": "object",
                "required": ["error_class", "recovery_step", "should_retry"],
                "properties": {
                    "error_class": { "type": "string", "const": "stale_lease" },
                    "recovery_step": { "type": "string" },
                    "should_retry": { "type": "boolean" }
                }
            }),
            scoring_schema: serde_json::json!({
                "dimensions": [
                    { "key": "recovery_correctness", "weight": 0.7, "scale": "0_to_1" },
                    { "key": "retry_decision", "weight": 0.3, "scale": "0_to_1" }
                ],
                "pass_threshold": 0.85,
                "reference_recovery": "re-acquire a fresh lease and retry"
            }),
            status: PromptResponseMatrixStatus::Deferred,
        },
        NewPromptResponseMatrixEntry {
            entry_id: "wp-0118.diagnostics-state-probe".to_string(),
            prompt_text:
                "Report the current atelier diagnostics state: name the reset modes available and \
                 whether any orphaned media is awaiting adoption."
                    .to_string(),
            expected_response_shape: serde_json::json!({
                "type": "object",
                "required": ["reset_modes", "orphaned_pending_count"],
                "properties": {
                    "reset_modes": { "type": "array", "items": { "type": "string" } },
                    "orphaned_pending_count": { "type": "integer", "minimum": 0 }
                }
            }),
            scoring_schema: serde_json::json!({
                "dimensions": [
                    { "key": "mode_enumeration", "weight": 0.5, "scale": "0_to_1" },
                    { "key": "orphan_count_accuracy", "weight": 0.5, "scale": "exact_match" }
                ],
                "pass_threshold": 0.9
            }),
            status: PromptResponseMatrixStatus::Deferred,
        },
        NewPromptResponseMatrixEntry {
            entry_id: "wp-0118.manual-anchor-resolution".to_string(),
            prompt_text:
                "Given the command `atelier.intake.classify`, resolve its ModelManual anchor and \
                 summarize the documented workflow in one sentence."
                    .to_string(),
            expected_response_shape: serde_json::json!({
                "type": "object",
                "required": ["command", "manual_anchor", "summary"],
                "properties": {
                    "command": { "type": "string" },
                    "manual_anchor": { "type": "string" },
                    "summary": { "type": "string" }
                }
            }),
            scoring_schema: serde_json::json!({
                "dimensions": [
                    { "key": "anchor_resolved", "weight": 0.6, "scale": "0_or_1" },
                    { "key": "summary_fidelity", "weight": 0.4, "scale": "0_to_1" }
                ],
                "pass_threshold": 0.8
            }),
            status: PromptResponseMatrixStatus::Deferred,
        },
    ]
}

// ---------------------------------------------------------------------------
// MT-166: Installer Reset And Orphan Evidence Projection.
//
// A read projection over the existing atelier_reset_operation /
// atelier_orphan_manifest_item tables (migration 0089). It surfaces reset and
// orphan-adoption behavior for model diagnostics WITHOUT a new table.
// ---------------------------------------------------------------------------

/// One reset operation, projected for model diagnostics.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResetDiagnosticsRow {
    pub reset_id: Uuid,
    pub mode: String,
    pub requested_by: String,
    pub reason: String,
    pub preferences_deleted_count: i64,
    pub original_media_preserved_count: i64,
    pub orphan_manifest_id: Option<Uuid>,
    pub created_at_utc: DateTime<Utc>,
}

/// The aggregate reset/orphan evidence projection a model reads to understand
/// installer reset/orphan behavior at a glance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResetOrphanDiagnostics {
    /// All reset operations (newest first).
    pub resets: Vec<ResetDiagnosticsRow>,
    /// Available reset modes, surfaced for diagnostics enumeration.
    pub reset_modes: Vec<String>,
    /// Count of orphan-manifest items still awaiting adoption.
    pub orphaned_pending_count: i64,
    /// Count of orphan-manifest items already adopted back into intake.
    pub adopted_count: i64,
}

impl AtelierStore {
    /// MT-140: persist the structured error-taxonomy catalog (10 classes, each
    /// with a recovery hint). Idempotent on `class`; re-recording refreshes the
    /// description/recovery hint. Emits `DIAGNOSTICS_ERROR_TAXONOMY_RECORDED`
    /// once per recorded class. Returns the reloaded entry.
    pub async fn record_diagnostics_error_class(
        &self,
        class: DiagnosticsErrorClass,
        description: &str,
        recovery_hint: &str,
    ) -> AtelierResult<DiagnosticsErrorTaxonomyEntry> {
        if description.trim().is_empty() || description.trim() != description {
            return Err(AtelierError::Validation(
                "diagnostics error taxonomy description must be non-empty and unpadded".into(),
            ));
        }
        if recovery_hint.trim().is_empty() || recovery_hint.trim() != recovery_hint {
            return Err(AtelierError::Validation(
                "diagnostics error taxonomy recovery_hint must be non-empty and unpadded".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_diagnostics_error_taxonomy (class, description, recovery_hint)
               VALUES ($1, $2, $3)
               ON CONFLICT (class) DO UPDATE SET
                 description = EXCLUDED.description,
                 recovery_hint = EXCLUDED.recovery_hint
               RETURNING class, description, recovery_hint, created_at_utc"#,
        )
        .bind(class.as_token())
        .bind(description)
        .bind(recovery_hint)
        .fetch_one(&mut *tx)
        .await?;

        let entry = DiagnosticsErrorTaxonomyEntry {
            class: DiagnosticsErrorClass::from_token(row.get("class"))?,
            description: row.get("description"),
            recovery_hint: row.get("recovery_hint"),
            created_at_utc: row.get("created_at_utc"),
        };
        self.record_event_in_tx(
            &mut tx,
            DIAGNOSTICS_ERROR_TAXONOMY_RECORDED,
            "atelier_diagnostics_error_taxonomy",
            entry.class.as_token(),
            serde_json::json!({
                "class": entry.class.as_token(),
                "description": entry.description,
                "recovery_hint": entry.recovery_hint,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(entry)
    }

    /// MT-140: persist the entire error-taxonomy catalog in one call.
    pub async fn record_error_taxonomy_catalog(
        &self,
    ) -> AtelierResult<Vec<DiagnosticsErrorTaxonomyEntry>> {
        let mut out = Vec::new();
        for (class, description, recovery_hint) in error_taxonomy_catalog() {
            out.push(
                self.record_diagnostics_error_class(class, description, recovery_hint)
                    .await?,
            );
        }
        Ok(out)
    }

    /// MT-140: list the recorded error-taxonomy classes, in canonical token order.
    pub async fn list_diagnostics_error_taxonomy(
        &self,
    ) -> AtelierResult<Vec<DiagnosticsErrorTaxonomyEntry>> {
        let rows = sqlx::query(
            r#"SELECT class, description, recovery_hint, created_at_utc
               FROM atelier_diagnostics_error_taxonomy
               ORDER BY class ASC"#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(DiagnosticsErrorTaxonomyEntry {
                    class: DiagnosticsErrorClass::from_token(row.get("class"))?,
                    description: row.get("description"),
                    recovery_hint: row.get("recovery_hint"),
                    created_at_utc: row.get("created_at_utc"),
                })
            })
            .collect()
    }

    /// MT-207: preserve one prompt-response matrix entry as a deferred contract.
    /// Idempotent on `entry_id`. Emits
    /// `DIAGNOSTICS_PROMPT_RESPONSE_MATRIX_RECORDED`. Does NOT score anything.
    pub async fn record_prompt_response_matrix_entry(
        &self,
        new: &NewPromptResponseMatrixEntry,
    ) -> AtelierResult<PromptResponseMatrixEntry> {
        if new.entry_id.trim().is_empty() || new.entry_id.trim() != new.entry_id {
            return Err(AtelierError::Validation(
                "prompt-response matrix entry_id must be non-empty and unpadded".into(),
            ));
        }
        if new.prompt_text.trim().is_empty() {
            return Err(AtelierError::Validation(
                "prompt-response matrix prompt_text must be non-empty".into(),
            ));
        }
        if !new.expected_response_shape.is_object() && !new.expected_response_shape.is_array() {
            return Err(AtelierError::Validation(
                "prompt-response matrix expected_response_shape must be a JSON object or array \
                 (a shape descriptor)"
                    .into(),
            ));
        }
        if !new.scoring_schema.is_object() {
            return Err(AtelierError::Validation(
                "prompt-response matrix scoring_schema must be a JSON object".into(),
            ));
        }

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_diagnostics_prompt_response_matrix
                 (entry_id, prompt_text, expected_response_shape, scoring_schema, status)
               VALUES ($1, $2, $3::jsonb, $4::jsonb, $5)
               ON CONFLICT (entry_id) DO UPDATE SET
                 prompt_text = EXCLUDED.prompt_text,
                 expected_response_shape = EXCLUDED.expected_response_shape,
                 scoring_schema = EXCLUDED.scoring_schema,
                 status = EXCLUDED.status
               RETURNING entry_id, prompt_text, expected_response_shape, scoring_schema,
                         status, created_at_utc"#,
        )
        .bind(&new.entry_id)
        .bind(&new.prompt_text)
        .bind(&new.expected_response_shape)
        .bind(&new.scoring_schema)
        .bind(new.status.as_token())
        .fetch_one(&mut *tx)
        .await?;

        let entry = PromptResponseMatrixEntry {
            entry_id: row.get("entry_id"),
            prompt_text: row.get("prompt_text"),
            expected_response_shape: row.get("expected_response_shape"),
            scoring_schema: row.get("scoring_schema"),
            status: PromptResponseMatrixStatus::from_token(row.get("status"))?,
            created_at_utc: row.get("created_at_utc"),
        };
        self.record_event_in_tx(
            &mut tx,
            DIAGNOSTICS_PROMPT_RESPONSE_MATRIX_RECORDED,
            "atelier_diagnostics_prompt_response_matrix",
            &entry.entry_id,
            serde_json::json!({
                "entry_id": entry.entry_id,
                "status": entry.status.as_token(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(entry)
    }

    /// MT-207: preserve the whole WP-0118 catalog in one call.
    pub async fn record_prompt_response_matrix_catalog(
        &self,
    ) -> AtelierResult<Vec<PromptResponseMatrixEntry>> {
        let mut out = Vec::new();
        for new in prompt_response_matrix_catalog() {
            out.push(self.record_prompt_response_matrix_entry(&new).await?);
        }
        Ok(out)
    }

    /// MT-207: list preserved prompt-response matrix entries, by `entry_id`.
    pub async fn list_prompt_response_matrix(
        &self,
    ) -> AtelierResult<Vec<PromptResponseMatrixEntry>> {
        let rows = sqlx::query(
            r#"SELECT entry_id, prompt_text, expected_response_shape, scoring_schema,
                      status, created_at_utc
               FROM atelier_diagnostics_prompt_response_matrix
               ORDER BY entry_id ASC"#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(PromptResponseMatrixEntry {
                    entry_id: row.get("entry_id"),
                    prompt_text: row.get("prompt_text"),
                    expected_response_shape: row.get("expected_response_shape"),
                    scoring_schema: row.get("scoring_schema"),
                    status: PromptResponseMatrixStatus::from_token(row.get("status"))?,
                    created_at_utc: row.get("created_at_utc"),
                })
            })
            .collect()
    }

    /// MT-166: build the reset/orphan evidence projection over the existing
    /// reset + orphan tables and emit `DIAGNOSTICS_RESET_ORPHAN_PROJECTED`.
    /// Read-only over canonical rows; adds no new table.
    pub async fn record_reset_orphan_diagnostics_projection(
        &self,
    ) -> AtelierResult<ResetOrphanDiagnostics> {
        let projection = self.list_reset_orphan_diagnostics().await?;
        self.record_event(
            DIAGNOSTICS_RESET_ORPHAN_PROJECTED,
            "atelier_diagnostics_reset_orphan_projection",
            "reset-orphan-evidence",
            serde_json::json!({
                "reset_count": projection.resets.len(),
                "orphaned_pending_count": projection.orphaned_pending_count,
                "adopted_count": projection.adopted_count,
                "reset_modes": projection.reset_modes,
            }),
        )
        .await?;
        Ok(projection)
    }

    /// MT-166: read the reset/orphan evidence projection without emitting an event.
    pub async fn list_reset_orphan_diagnostics(&self) -> AtelierResult<ResetOrphanDiagnostics> {
        let reset_rows = sqlx::query(
            r#"SELECT reset_id, mode, requested_by, reason,
                      preferences_deleted_count, original_media_preserved_count,
                      orphan_manifest_id, created_at_utc
               FROM atelier_reset_operation
               ORDER BY created_at_utc DESC, reset_id DESC"#,
        )
        .fetch_all(self.pool())
        .await?;
        let resets = reset_rows
            .into_iter()
            .map(|row| ResetDiagnosticsRow {
                reset_id: row.get("reset_id"),
                mode: row.get("mode"),
                requested_by: row.get("requested_by"),
                reason: row.get("reason"),
                preferences_deleted_count: row.get("preferences_deleted_count"),
                original_media_preserved_count: row.get("original_media_preserved_count"),
                orphan_manifest_id: row.get("orphan_manifest_id"),
                created_at_utc: row.get("created_at_utc"),
            })
            .collect::<Vec<_>>();

        let counts = sqlx::query(
            r#"SELECT
                 COUNT(*) FILTER (WHERE adoption_status = 'orphaned') AS orphaned_pending,
                 COUNT(*) FILTER (WHERE adoption_status = 'adopted')  AS adopted
               FROM atelier_orphan_manifest_item"#,
        )
        .fetch_one(self.pool())
        .await?;
        let orphaned_pending_count: i64 = counts.get("orphaned_pending");
        let adopted_count: i64 = counts.get("adopted");

        Ok(ResetOrphanDiagnostics {
            resets,
            reset_modes: vec![
                "preferences_only".to_string(),
                "full_preserve_original_media".to_string(),
            ],
            orphaned_pending_count,
            adopted_count,
        })
    }
}

// ===========================================================================
// MT-145 / MT-144: Command Log + stale-session detection (WP-KERNEL-005).
//
// Two more typed Model-Workflow-Diagnostics runtime surfaces, appended after
// the MT-140/MT-166/MT-207 surfaces above. PostgreSQL only; SQLite forbidden.
//
//   * MT-145 -- atelier_command_log: an APPEND-ONLY queryable command log tied
//     to sessions and receipts. record_command_log_entry validates the session/
//     receipt/evidence refs through reject_legacy_runtime_ref, persists the row,
//     and emits COMMAND_LOG_RECORDED. Append-only: re-recording the same
//     command_log_id is REJECTED (not upserted). list_command_log_for_session
//     queries the log for one session.
//   * MT-144 -- atelier_diagnostics_session: heartbeat-bearing session records.
//     record_session_heartbeat advances last_heartbeat_utc; detect_stale_sessions
//     flags sessions whose last_heartbeat is older than the timeout as STALE;
//     list_stale_sessions surfaces them. The KEY INVARIANT is that a stale
//     session's evidence is PRESERVED -- marking STALE is a status flag, never a
//     delete of the session row or its atelier_command_log evidence rows.
// ===========================================================================

use chrono::Duration;

/// Command-log + session-heartbeat event families (MT-145 / MT-144). Defined
/// here so the parent folds these into [`super::event_family::ALL`] and the
/// MT-005 coverage check picks up these mutations.
pub mod command_log_event_family {
    /// An append-only command-log entry was recorded (MT-145).
    pub const COMMAND_LOG_RECORDED: &str = "atelier.command_log.recorded";
    /// A session heartbeat was recorded / refreshed (MT-144).
    pub const SESSION_HEARTBEAT_RECORDED: &str = "atelier.diagnostics.session_heartbeat_recorded";
    /// One or more sessions were flagged STALE by heartbeat-timeout detection;
    /// their evidence is preserved (MT-144).
    pub const SESSION_FLAGGED_STALE: &str = "atelier.diagnostics.session_flagged_stale";

    /// All command-log / session-heartbeat event families (for parity folding).
    pub const ALL: &[&str] = &[
        COMMAND_LOG_RECORDED,
        SESSION_HEARTBEAT_RECORDED,
        SESSION_FLAGGED_STALE,
    ];
}

pub use command_log_event_family::{
    COMMAND_LOG_RECORDED, SESSION_FLAGGED_STALE, SESSION_HEARTBEAT_RECORDED,
};

// ---------------------------------------------------------------------------
// MT-145: append-only command log tied to sessions + receipts.
// ---------------------------------------------------------------------------

/// Input to record one append-only command-log entry (MT-145). `receipt_ref`
/// and `evidence_ref` are optional portable handles; when present they are
/// validated through [`super::reject_legacy_runtime_ref`] so no SQLite path,
/// drive letter, localhost authority, or CKC/Electron legacy ref crosses the
/// persistence boundary.
#[derive(Clone, Debug)]
pub struct NewCommandLogEntry {
    /// Stable, caller-supplied unique id (the PK). Re-recording the same id is
    /// rejected (append-only).
    pub command_log_id: String,
    /// The session this command invocation is tied to.
    pub session_ref: String,
    /// The command/action id that was invoked.
    pub command_id: String,
    /// Terminal/observed status token, e.g. `ok`, `error`, `denied`.
    pub status: String,
    /// Optional typed receipt handle produced by the invocation.
    pub receipt_ref: Option<String>,
    /// Optional EventLedger/Flight-Recorder evidence handle.
    pub evidence_ref: Option<String>,
}

/// A persisted append-only command-log row (MT-145).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandLogEntry {
    pub command_log_id: String,
    pub session_ref: String,
    pub command_id: String,
    pub status: String,
    pub receipt_ref: Option<String>,
    pub evidence_ref: Option<String>,
    pub recorded_at_utc: DateTime<Utc>,
}

fn command_log_from_row(row: &sqlx::postgres::PgRow) -> CommandLogEntry {
    CommandLogEntry {
        command_log_id: row.get("command_log_id"),
        session_ref: row.get("session_ref"),
        command_id: row.get("command_id"),
        status: row.get("status"),
        receipt_ref: row.get("receipt_ref"),
        evidence_ref: row.get("evidence_ref"),
        recorded_at_utc: row.get("recorded_at_utc"),
    }
}

// ---------------------------------------------------------------------------
// MT-144: heartbeat-bearing session records + stale detection.
// ---------------------------------------------------------------------------

/// Lifecycle status of a heartbeat-bearing diagnostics session (MT-144).
/// `Stale` is a flag only: it never implies the session row or its evidence was
/// deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SessionStatus {
    Active,
    Stale,
}

impl SessionStatus {
    pub fn as_token(self) -> &'static str {
        match self {
            SessionStatus::Active => "ACTIVE",
            SessionStatus::Stale => "STALE",
        }
    }

    pub fn from_token(token: &str) -> AtelierResult<Self> {
        match token {
            "ACTIVE" => Ok(SessionStatus::Active),
            "STALE" => Ok(SessionStatus::Stale),
            other => Err(AtelierError::Validation(format!(
                "unknown session status token: {other}"
            ))),
        }
    }
}

/// A persisted heartbeat-bearing diagnostics session record (MT-144).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticsSession {
    pub session_ref: String,
    pub status: SessionStatus,
    pub last_heartbeat_utc: DateTime<Utc>,
    pub created_at_utc: DateTime<Utc>,
}

fn diagnostics_session_from_row(row: &sqlx::postgres::PgRow) -> AtelierResult<DiagnosticsSession> {
    let status: String = row.get("status");
    Ok(DiagnosticsSession {
        session_ref: row.get("session_ref"),
        status: SessionStatus::from_token(&status)?,
        last_heartbeat_utc: row.get("last_heartbeat_utc"),
        created_at_utc: row.get("created_at_utc"),
    })
}

/// Pure stale-session detection over already-loaded session records (MT-144).
///
/// A session is STALE when its `last_heartbeat_utc` is strictly older than
/// `now - timeout`. This is a pure projection (no I/O, no deletion): callers use
/// it to decide which sessions to FLAG. Marking a session stale never removes
/// the session or any of its evidence rows.
pub fn detect_stale_sessions(
    sessions: &[DiagnosticsSession],
    now: DateTime<Utc>,
    timeout: Duration,
) -> Vec<DiagnosticsSession> {
    let cutoff = now - timeout;
    sessions
        .iter()
        .filter(|s| s.last_heartbeat_utc < cutoff)
        .cloned()
        .collect()
}

impl AtelierStore {
    /// MT-145: record one APPEND-ONLY command-log entry tied to a session and an
    /// optional receipt/evidence ref. Validates refs through
    /// [`super::reject_legacy_runtime_ref`], persists the row, and emits
    /// `COMMAND_LOG_RECORDED`. Append-only: re-recording the same
    /// `command_log_id` is REJECTED with a typed Validation error (the row is
    /// never upserted, so prior evidence can never be silently overwritten).
    pub async fn record_command_log_entry(
        &self,
        new: &NewCommandLogEntry,
    ) -> AtelierResult<CommandLogEntry> {
        if new.command_log_id.trim().is_empty() || new.command_log_id.trim() != new.command_log_id {
            return Err(AtelierError::Validation(
                "command_log_id must be non-empty and unpadded".into(),
            ));
        }
        if new.command_id.trim().is_empty() || new.command_id.trim() != new.command_id {
            return Err(AtelierError::Validation(
                "command_id must be non-empty and unpadded".into(),
            ));
        }
        if new.status.trim().is_empty() || new.status.trim() != new.status {
            return Err(AtelierError::Validation(
                "status must be non-empty and unpadded".into(),
            ));
        }
        // session_ref / receipt_ref / evidence_ref are portable handles that
        // cross the persistence boundary: reject legacy/local-runtime refs.
        reject_legacy_runtime_ref("session_ref", &new.session_ref)?;
        if let Some(receipt_ref) = &new.receipt_ref {
            reject_legacy_runtime_ref("receipt_ref", receipt_ref)?;
        }
        if let Some(evidence_ref) = &new.evidence_ref {
            reject_legacy_runtime_ref("evidence_ref", evidence_ref)?;
        }

        // Append-only enforcement: a PK conflict means this command_log_id was
        // already recorded. We DO NOT upsert; we surface a typed Validation
        // error so the prior evidence row stays untouched.
        let already_exists: Option<String> = sqlx::query_scalar(
            "SELECT command_log_id FROM atelier_command_log WHERE command_log_id = $1",
        )
        .bind(&new.command_log_id)
        .fetch_optional(self.pool())
        .await?;
        if already_exists.is_some() {
            return Err(AtelierError::Validation(format!(
                "command_log is append-only: command_log_id={} already recorded; \
                 re-recording is rejected (not upserted)",
                new.command_log_id
            )));
        }

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_command_log
                 (command_log_id, session_ref, command_id, status, receipt_ref, evidence_ref)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING command_log_id, session_ref, command_id, status,
                         receipt_ref, evidence_ref, recorded_at_utc"#,
        )
        .bind(&new.command_log_id)
        .bind(&new.session_ref)
        .bind(&new.command_id)
        .bind(&new.status)
        .bind(&new.receipt_ref)
        .bind(&new.evidence_ref)
        .fetch_one(&mut *tx)
        .await?;

        let entry = command_log_from_row(&row);
        self.record_event_in_tx(
            &mut tx,
            COMMAND_LOG_RECORDED,
            "atelier_command_log",
            &entry.command_log_id,
            serde_json::json!({
                "command_log_id": entry.command_log_id,
                "session_ref": entry.session_ref,
                "command_id": entry.command_id,
                "status": entry.status,
                "has_receipt_ref": entry.receipt_ref.is_some(),
                "has_evidence_ref": entry.evidence_ref.is_some(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(entry)
    }

    /// MT-145: list the append-only command log for one session, oldest first.
    pub async fn list_command_log_for_session(
        &self,
        session_ref: &str,
    ) -> AtelierResult<Vec<CommandLogEntry>> {
        let rows = sqlx::query(
            r#"SELECT command_log_id, session_ref, command_id, status,
                      receipt_ref, evidence_ref, recorded_at_utc
               FROM atelier_command_log
               WHERE session_ref = $1
               ORDER BY recorded_at_utc ASC, command_log_id ASC"#,
        )
        .bind(session_ref)
        .fetch_all(self.pool())
        .await?;
        Ok(rows.iter().map(command_log_from_row).collect())
    }

    /// MT-144: record (open or refresh) a session heartbeat. Idempotent on
    /// `session_ref`: a new session starts ACTIVE; an existing session's
    /// `last_heartbeat_utc` advances to NOW() and its status is reset to ACTIVE
    /// (a fresh heartbeat clears a prior STALE flag). Emits
    /// `SESSION_HEARTBEAT_RECORDED`. Never deletes evidence.
    pub async fn record_session_heartbeat(
        &self,
        session_ref: &str,
    ) -> AtelierResult<DiagnosticsSession> {
        reject_legacy_runtime_ref("session_ref", session_ref)?;

        let mut tx = self.pool().begin().await?;
        let row = sqlx::query(
            r#"INSERT INTO atelier_diagnostics_session (session_ref, status, last_heartbeat_utc)
               VALUES ($1, 'ACTIVE', NOW())
               ON CONFLICT (session_ref) DO UPDATE SET
                 status = 'ACTIVE',
                 last_heartbeat_utc = NOW()
               RETURNING session_ref, status, last_heartbeat_utc, created_at_utc"#,
        )
        .bind(session_ref)
        .fetch_one(&mut *tx)
        .await?;

        let session = diagnostics_session_from_row(&row)?;
        self.record_event_in_tx(
            &mut tx,
            SESSION_HEARTBEAT_RECORDED,
            "atelier_diagnostics_session",
            &session.session_ref,
            serde_json::json!({
                "session_ref": session.session_ref,
                "status": session.status.as_token(),
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(session)
    }

    /// MT-144: list all heartbeat-bearing diagnostics sessions, newest heartbeat
    /// first.
    pub async fn list_diagnostics_sessions(&self) -> AtelierResult<Vec<DiagnosticsSession>> {
        let rows = sqlx::query(
            r#"SELECT session_ref, status, last_heartbeat_utc, created_at_utc
               FROM atelier_diagnostics_session
               ORDER BY last_heartbeat_utc DESC, session_ref ASC"#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(diagnostics_session_from_row).collect()
    }

    /// MT-144: flag every session whose `last_heartbeat_utc` is older than
    /// `now - timeout` as STALE and return the now-stale sessions. This is the
    /// evidence-preserving path: it ONLY flips the `status` column to STALE; it
    /// never deletes the session row or any tied `atelier_command_log` evidence.
    /// Emits `SESSION_FLAGGED_STALE` when at least one session is flagged.
    pub async fn flag_stale_sessions(
        &self,
        timeout: Duration,
    ) -> AtelierResult<Vec<DiagnosticsSession>> {
        if timeout < Duration::zero() {
            return Err(AtelierError::Validation(
                "stale-session timeout must not be negative".into(),
            ));
        }
        // Postgres computes the cutoff against canonical server time so the
        // detection is consistent with NOW()-stamped heartbeats.
        let timeout_seconds = timeout.num_seconds();

        let mut tx = self.pool().begin().await?;
        let rows = sqlx::query(
            r#"UPDATE atelier_diagnostics_session
               SET status = 'STALE'
               WHERE last_heartbeat_utc < (NOW() - make_interval(secs => $1::double precision))
                 AND status <> 'STALE'
               RETURNING session_ref, status, last_heartbeat_utc, created_at_utc"#,
        )
        .bind(timeout_seconds as f64)
        .fetch_all(&mut *tx)
        .await?;

        let flagged: Vec<DiagnosticsSession> = rows
            .iter()
            .map(diagnostics_session_from_row)
            .collect::<AtelierResult<_>>()?;

        if !flagged.is_empty() {
            self.record_event_in_tx(
                &mut tx,
                SESSION_FLAGGED_STALE,
                "atelier_diagnostics_session",
                "stale-session-detection",
                serde_json::json!({
                    "flagged_count": flagged.len(),
                    "session_refs": flagged.iter().map(|s| &s.session_ref).collect::<Vec<_>>(),
                    "evidence_preserved": true,
                }),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(flagged)
    }

    /// MT-144: list the sessions currently flagged STALE, newest heartbeat first.
    /// Their evidence rows remain queryable via
    /// [`AtelierStore::list_command_log_for_session`].
    pub async fn list_stale_sessions(&self) -> AtelierResult<Vec<DiagnosticsSession>> {
        let rows = sqlx::query(
            r#"SELECT session_ref, status, last_heartbeat_utc, created_at_utc
               FROM atelier_diagnostics_session
               WHERE status = 'STALE'
               ORDER BY last_heartbeat_utc DESC, session_ref ASC"#,
        )
        .fetch_all(self.pool())
        .await?;
        rows.iter().map(diagnostics_session_from_row).collect()
    }
}
