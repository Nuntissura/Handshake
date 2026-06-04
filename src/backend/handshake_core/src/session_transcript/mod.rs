//! Unified per-session transcript: pure merge/order/types.
//!
//! GOAL (governance glue): give the operator ONE ordered, durable, reviewable
//! timeline per session — "go back and look when things go wrong or I forget".
//! The per-session activity that today lives in THREE separate streams (chat
//! turns, Flight Recorder events incl. terminal capture + inference, and the
//! swarm/process lifecycle) is *derived on read* into one timestamp-ordered
//! typed timeline. Nothing is stored twice: this module joins the existing
//! durable sources.
//!
//! This file is the SHARED, Tauri-free piece: it owns the typed
//! [`SessionTranscriptEntry`] enum + the deterministic merge algorithm so it is
//! unit-testable in `cargo test -p handshake_core` without a Tauri runtime. The
//! IPC surface + source wiring lives in the app's
//! `commands/session_transcript.rs`.
//!
//! ## Session identity (the central problem)
//!
//! The three streams do NOT share one session id:
//!   * Chat: a fresh `Uuid::now_v7()` per app launch, keyed on disk by the chat
//!     UUID (`<app_data_root>/sessions/<session_id>/chat.jsonl`).
//!   * Swarm lifecycle FR events: carry the composite `instance_id`
//!     (`<model_id>#<instance>`) ONLY inside `payload.instance_id`; the top-level
//!     `model_session_id` column is NOT set.
//!   * Terminal capture FR events: key on `session_span_id` (a terminal UUID) and
//!     `job_id` (swarm_id); the swarm composite `instance_id` is in
//!     `payload.instance_id` (the capture binding sets it at spawn).
//!   * LLM inference FR events: set only `model_id` + `trace_id`; no session
//!     linkage unless a caller adds `model_session_id` downstream.
//!
//! The canonical transcript `session_id` is therefore the swarm composite
//! `instance_id` for swarm/agent sessions, and the chat UUID for the operator
//! chat session. Because no single FR column holds the composite, the aggregator
//! queries FR through TWO seams (documented `EventFilter` + a scoped raw SELECT)
//! and unions the result, deduped by `event_id`. This module is agnostic to how
//! the events were fetched: it merges whatever chat rows + FR events it is given.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::flight_recorder::{FlightRecorderEvent, FlightRecorderEventType};

/// ROI #5 EXPORT: pure markdown+json render + redaction pass + safe-filename
/// helper for the per-session export. Tauri-free; the IO glue lives in the app
/// crate's `commands/session_transcript.rs`.
pub mod export;

pub use export::{
    safe_session_stem, ExportCounts, ExportFormat, ExportHeader, RenderedExport,
    EXPORT_SCHEMA_VERSION,
};

/// Coarse kind of a transcript entry — the UI filter vocabulary + the IPC
/// `kinds` argument map onto this 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptKind {
    ChatTurn,
    FrEvent,
    /// Structured agent activity (tool call / thinking / text / other) parsed
    /// from a JSON-stream CLI bridge run. Surfaced from `FR-EVT-AGENT-*` events.
    AgentActivity,
    TerminalChunk,
    Process,
}

impl TranscriptKind {
    /// Parse the IPC string form (`"chat_turn"`, `"fr_event"`, …). Unknown
    /// strings yield `None` so the caller can reject honestly rather than
    /// silently widening the filter.
    pub fn from_ipc(raw: &str) -> Option<Self> {
        match raw.trim() {
            "chat_turn" => Some(Self::ChatTurn),
            "fr_event" => Some(Self::FrEvent),
            "agent_activity" => Some(Self::AgentActivity),
            "terminal_chunk" => Some(Self::TerminalChunk),
            "process" => Some(Self::Process),
            _ => None,
        }
    }
}

/// One row in the unified per-session timeline. Every variant carries a common
/// `ts` (the merge key) + a `seq` (assigned post-merge for stable scroll/test
/// anchoring).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionTranscriptEntry {
    /// A chat turn from `chat.jsonl`.
    ///
    /// Variant TAG stays snake_case (`"chat_turn"`, the `kind` discriminant the
    /// frontend union keys on); inner FIELDS are camelCase to match the repo-wide
    /// Tauri IPC convention (see `commands/terminal.rs`, `caa.rs`,
    /// `cli_bridge_config.rs`). Per-variant `rename_all` overrides the
    /// container-level snake_case for the fields only.
    #[serde(rename_all = "camelCase")]
    ChatTurn {
        ts: DateTime<Utc>,
        seq: u64,
        role: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        model_role: Option<String>,
        content: String,
        message_id: String,
    },
    /// A generic Flight Recorder event (lifecycle / inference / system / etc.).
    #[serde(rename_all = "camelCase")]
    FrEvent {
        ts: DateTime<Utc>,
        seq: u64,
        event_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        fr_event: Option<String>,
        actor: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        model_id: Option<String>,
        payload: Value,
        event_id: String,
    },
    /// A structured agent-activity row, derived from an `FR-EVT-AGENT-*` event
    /// emitted by the official-CLI bridge when run in a JSON-stream output mode.
    /// Captures the operator's "all toolcalls, visible thought processes" as
    /// typed records. NOTE: the raw CLI stdout still streams to the terminal lane
    /// byte-faithfully; these rows are the TYPED projection of the SAME run, so a
    /// viewer showing both lanes may see content twice — the kind filter lets the
    /// operator pick. `activity_kind` is one of `tool_call|thinking|text|other`.
    #[serde(rename_all = "camelCase")]
    AgentActivity {
        ts: DateTime<Utc>,
        seq: u64,
        /// `tool_call` | `thinking` | `text` | `other`.
        activity_kind: String,
        /// Tool name (only for `tool_call`).
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        /// Redacted tool input object (only for `tool_call`).
        #[serde(skip_serializing_if = "Option::is_none")]
        detail: Option<Value>,
        /// Body text for `thinking` / `text` / `other` (redacted).
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        event_id: String,
    },
    /// A terminal capture event (session open/close/command exec). The raw
    /// captured stdout stream — when a live terminal session is still attached —
    /// is appended by the aggregator as a `TerminalChunk` with `text` set.
    #[serde(rename_all = "camelCase")]
    TerminalChunk {
        ts: DateTime<Utc>,
        seq: u64,
        terminal_session_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        fr_event: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        command: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    /// A process-lifecycle row, derived from FR events carrying
    /// `payload.process_uuid` (swarm `SessionSpawned`/`SessionCompleted`). The
    /// Postgres process ledger is write-only + off-app-data-root, so the honest
    /// portable process signal is these FR-derived rows.
    #[serde(rename_all = "camelCase")]
    Process {
        ts: DateTime<Utc>,
        seq: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        process_uuid: Option<String>,
        phase: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        model_id: Option<String>,
        payload: Value,
    },
}

impl SessionTranscriptEntry {
    /// The merge-key timestamp.
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::ChatTurn { ts, .. }
            | Self::FrEvent { ts, .. }
            | Self::AgentActivity { ts, .. }
            | Self::TerminalChunk { ts, .. }
            | Self::Process { ts, .. } => *ts,
        }
    }

    /// The coarse kind (for the UI filter).
    pub fn kind(&self) -> TranscriptKind {
        match self {
            Self::ChatTurn { .. } => TranscriptKind::ChatTurn,
            Self::FrEvent { .. } => TranscriptKind::FrEvent,
            Self::AgentActivity { .. } => TranscriptKind::AgentActivity,
            Self::TerminalChunk { .. } => TranscriptKind::TerminalChunk,
            Self::Process { .. } => TranscriptKind::Process,
        }
    }

    /// The post-merge sequence number (0-based, stable).
    pub fn seq(&self) -> u64 {
        match self {
            Self::ChatTurn { seq, .. }
            | Self::FrEvent { seq, .. }
            | Self::AgentActivity { seq, .. }
            | Self::TerminalChunk { seq, .. }
            | Self::Process { seq, .. } => *seq,
        }
    }

    fn set_seq(&mut self, value: u64) {
        match self {
            Self::ChatTurn { seq, .. }
            | Self::FrEvent { seq, .. }
            | Self::AgentActivity { seq, .. }
            | Self::TerminalChunk { seq, .. }
            | Self::Process { seq, .. } => *seq = value,
        }
    }
}

/// A minimal, Tauri-free mirror of a chat row used by the merge. The app maps
/// its `SessionChatLogEntryV0_1` into this shape (so the pure lib does not
/// depend on the Tauri crate). `created_at_utc` is the RFC3339 string straight
/// from the chat log; `turn_index` is the stable native-order tiebreak.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTurnInput {
    pub created_at_utc: String,
    pub turn_index: u64,
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_role: Option<String>,
    pub content: String,
    pub message_id: String,
}

/// FR `event_type` strings that indicate a terminal capture event. Terminal
/// session lifecycle + command-exec events are emitted as
/// `FlightRecorderEventType::TerminalCommand` with `payload.fr_event` starting
/// `FR-EVT-TERMINAL-`.
const FR_EVT_TERMINAL_PREFIX: &str = "FR-EVT-TERMINAL";

/// Read `payload.fr_event` (string) if present.
fn payload_fr_event(payload: &Value) -> Option<String> {
    payload
        .get("fr_event")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// Read `payload.event_id` (string) if present. The INFER and AGENT events write
/// the stable id under `event_id` (NOT `fr_event`), so the agent classify reads
/// this key. Mirrors `events_agent_activity::agent_activity_event`.
fn payload_event_id(payload: &Value) -> Option<String> {
    payload
        .get("event_id")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// The four stable `FR-EVT-AGENT-*` ids (kept local so this pure module does not
/// depend on the flight_recorder events module). Mapped to the `activity_kind`
/// string the frontend keys on.
fn agent_activity_kind_for(event_id: &str) -> Option<&'static str> {
    match event_id {
        "FR-EVT-AGENT-TOOLCALL" => Some("tool_call"),
        "FR-EVT-AGENT-THINKING" => Some("thinking"),
        "FR-EVT-AGENT-TEXT" => Some("text"),
        "FR-EVT-AGENT-OTHER" => Some("other"),
        _ => None,
    }
}

/// Build an `AgentActivity` row from an `FR-EVT-AGENT-*` event payload, copying
/// `name` / `detail` / `text` straight from the payload the runtime wrote.
fn agent_activity_entry(
    event: &FlightRecorderEvent,
    activity_kind: &str,
) -> SessionTranscriptEntry {
    let payload = &event.payload;
    SessionTranscriptEntry::AgentActivity {
        ts: event.timestamp,
        seq: 0,
        activity_kind: activity_kind.to_string(),
        name: payload
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string),
        detail: payload.get("detail").cloned(),
        text: payload
            .get("text")
            .and_then(Value::as_str)
            .map(str::to_string),
        event_id: event.event_id.to_string(),
    }
}

/// Read `payload.process_uuid` (string) if present and non-empty.
fn payload_process_uuid(payload: &Value) -> Option<String> {
    payload
        .get("process_uuid")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Classify a swarm-lifecycle FR event into a process phase string. Derived from
/// the `fr_event_id`/`fr_event` marker when present, else the event_type.
fn process_phase(payload: &Value, event: &FlightRecorderEvent) -> String {
    if let Some(marker) = payload
        .get("fr_event_id")
        .or_else(|| payload.get("fr_event"))
        .and_then(Value::as_str)
    {
        return marker.to_string();
    }
    event.event_type.to_string()
}

fn is_terminal_event(event: &FlightRecorderEvent, fr_event: &Option<String>) -> bool {
    matches!(event.event_type, FlightRecorderEventType::TerminalCommand)
        && fr_event
            .as_deref()
            .map(|s| s.starts_with(FR_EVT_TERMINAL_PREFIX))
            .unwrap_or(false)
}

/// Build the transcript entries that a single FR event contributes. A swarm
/// lifecycle event carrying `payload.process_uuid` yields BOTH an `FrEvent`
/// (the raw lifecycle line) AND a synthesized `Process` row (the honest process
/// signal). A terminal capture event yields a single `TerminalChunk`. Everything
/// else yields a single `FrEvent`.
///
/// `pub` so the cross-session search (the app crate's
/// `commands/session_transcript.rs`) can re-derive the SAME typed rows from the
/// SAME correlation seam — a search hit always corresponds to a row the
/// transcript would show, never a second parallel correlation.
pub fn entries_from_fr_event(event: &FlightRecorderEvent) -> Vec<SessionTranscriptEntry> {
    let ts = event.timestamp;
    let fr_event = payload_fr_event(&event.payload);

    // Structured agent-activity: an `FR-EVT-AGENT-*` event (id under
    // `payload.event_id`) classifies to a typed `AgentActivity` row INSTEAD of a
    // bare `FrEvent`. This is the single point of change for agent capture — the
    // raw-seam fetch, union/dedup, and merge sort are all reused unchanged.
    if let Some(event_id) = payload_event_id(&event.payload) {
        if let Some(activity_kind) = agent_activity_kind_for(&event_id) {
            return vec![agent_activity_entry(event, activity_kind)];
        }
    }

    if is_terminal_event(event, &fr_event) {
        let terminal_session_id = event
            .session_span_id
            .clone()
            .or_else(|| {
                event
                    .payload
                    .get("session_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default();
        let command = event
            .payload
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_string);
        return vec![SessionTranscriptEntry::TerminalChunk {
            ts,
            seq: 0,
            terminal_session_id,
            fr_event,
            command,
            text: None,
        }];
    }

    let mut out = Vec::with_capacity(2);
    out.push(SessionTranscriptEntry::FrEvent {
        ts,
        seq: 0,
        event_type: event.event_type.to_string(),
        fr_event: fr_event.clone(),
        actor: event.actor.to_string(),
        model_id: event.model_id.clone(),
        payload: event.payload.clone(),
        event_id: event.event_id.to_string(),
    });

    if let Some(process_uuid) = payload_process_uuid(&event.payload) {
        out.push(SessionTranscriptEntry::Process {
            ts,
            seq: 0,
            process_uuid: Some(process_uuid),
            phase: process_phase(&event.payload, event),
            model_id: event.model_id.clone(),
            payload: event.payload.clone(),
        });
    }

    out
}

/// Parse an RFC3339 timestamp; on failure fall back to the Unix epoch so a
/// malformed row sorts first and is still surfaced (honest — never dropped).
fn parse_ts(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw.trim())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::from_timestamp(0, 0).unwrap())
}

/// Source rank for the stable tiebreak when two entries share a `ts`. Chat
/// before FR before terminal before process keeps the timeline deterministic.
fn source_rank(entry: &SessionTranscriptEntry) -> u8 {
    match entry {
        SessionTranscriptEntry::ChatTurn { .. } => 0,
        SessionTranscriptEntry::FrEvent { .. } => 1,
        SessionTranscriptEntry::AgentActivity { .. } => 2,
        SessionTranscriptEntry::TerminalChunk { .. } => 3,
        SessionTranscriptEntry::Process { .. } => 4,
    }
}

/// A stable native-order key within a source (chat uses `turn_index`, FR uses
/// the time-monotonic `event_id` UUIDv7 string). Carried alongside each entry
/// only during the sort so equal-`ts` rows order deterministically.
struct Ordered {
    entry: SessionTranscriptEntry,
    native_order: String,
}

/// Merge chat rows + FR events into one timestamp-ordered, typed transcript.
///
/// Determinism: a STABLE sort on `(ts, source_rank, native_order)` guarantees a
/// reproducible order even when multiple entries share a `ts` — required for the
/// merge unit tests and for stable UI scroll anchoring.
///
/// Honesty: an empty `chat` and/or `fr` slice simply contributes no rows; this
/// function NEVER fabricates entries. Dedup of FR events fetched via two seams
/// is the caller's responsibility (union by `event_id` before calling).
pub fn merge_transcript(
    chat: Vec<ChatTurnInput>,
    fr: Vec<FlightRecorderEvent>,
) -> Vec<SessionTranscriptEntry> {
    let mut staged: Vec<Ordered> = Vec::with_capacity(chat.len() + fr.len());

    for row in chat {
        let ts = parse_ts(&row.created_at_utc);
        // Zero-pad the turn index so lexical compare matches numeric order.
        let native_order = format!("{:020}", row.turn_index);
        staged.push(Ordered {
            entry: SessionTranscriptEntry::ChatTurn {
                ts,
                seq: 0,
                role: row.role,
                model_role: row.model_role,
                content: row.content,
                message_id: row.message_id,
            },
            native_order,
        });
    }

    for event in &fr {
        let native_order = event.event_id.to_string();
        for entry in entries_from_fr_event(event) {
            staged.push(Ordered {
                entry,
                native_order: native_order.clone(),
            });
        }
    }

    staged.sort_by(|a, b| {
        let ta = a.entry.timestamp();
        let tb = b.entry.timestamp();
        ta.cmp(&tb)
            .then_with(|| source_rank(&a.entry).cmp(&source_rank(&b.entry)))
            .then_with(|| a.native_order.cmp(&b.native_order))
    });

    let mut out: Vec<SessionTranscriptEntry> = staged.into_iter().map(|o| o.entry).collect();
    for (idx, entry) in out.iter_mut().enumerate() {
        entry.set_seq(idx as u64);
    }
    out
}

/// Append a live terminal-scrollback enrichment entry. The durable terminal
/// record is the FR `TerminalChunk` events; this is an *enrichment* for a
/// still-open session (closed capture sessions are reaped from the live runtime,
/// so their raw stdout is gone from memory). Re-numbers `seq` after appending so
/// the timeline stays contiguous. The chunk is placed at `ts` (the snapshot
/// time) and merged in order.
pub fn append_terminal_scrollback(
    mut entries: Vec<SessionTranscriptEntry>,
    terminal_session_id: String,
    text: String,
    ts: DateTime<Utc>,
) -> Vec<SessionTranscriptEntry> {
    entries.push(SessionTranscriptEntry::TerminalChunk {
        ts,
        seq: 0,
        terminal_session_id,
        fr_event: None,
        command: None,
        text: Some(text),
    });
    // Stable re-sort by ts + kind so the enrichment lands in chronological order
    // without disturbing equal-ts ordering of the existing rows.
    entries.sort_by(|a, b| {
        a.timestamp()
            .cmp(&b.timestamp())
            .then_with(|| source_rank(a).cmp(&source_rank(b)))
            .then_with(|| a.seq().cmp(&b.seq()))
    });
    for (idx, entry) in entries.iter_mut().enumerate() {
        entry.set_seq(idx as u64);
    }
    entries
}

/// Filter a merged transcript to the requested kinds. An empty/`None` kinds set
/// is a no-op (all kinds pass). Re-numbering is NOT applied — `seq` remains the
/// stable anchor from the full merge so the UI can correlate a filtered row back
/// to its position in the complete timeline.
pub fn filter_by_kinds(
    entries: Vec<SessionTranscriptEntry>,
    kinds: Option<&[TranscriptKind]>,
) -> Vec<SessionTranscriptEntry> {
    match kinds {
        None => entries,
        Some(allowed) if allowed.is_empty() => entries,
        Some(allowed) => entries
            .into_iter()
            .filter(|e| allowed.contains(&e.kind()))
            .collect(),
    }
}

/// The searchable text fields a single transcript entry contributes to the
/// cross-session search corpus, paired with the entry's coarse [`TranscriptKind`]
/// and merge-key timestamp.
///
/// This is the SINGLE place that knows which fields of each typed row are
/// human-searchable, so the search command never re-implements the per-variant
/// field extraction (and never searches text the transcript can't display). Each
/// returned `String` is RAW (un-redacted) — redaction is applied by the caller on
/// the final windowed snippet, AFTER the match offset is computed, so a secret
/// adjacent to the match is masked before it leaves the backend.
///
/// `FrEvent`/`Process` payloads are compact-stringified so a match inside the raw
/// JSON payload is searchable (and the same compact string is what the caller
/// windows + redacts).
pub fn searchable_text(entry: &SessionTranscriptEntry) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    match entry {
        SessionTranscriptEntry::ChatTurn { content, .. } => {
            if !content.is_empty() {
                out.push(content.clone());
            }
        }
        SessionTranscriptEntry::AgentActivity {
            name, detail, text, ..
        } => {
            if let Some(n) = name {
                if !n.is_empty() {
                    out.push(n.clone());
                }
            }
            if let Some(t) = text {
                if !t.is_empty() {
                    out.push(t.clone());
                }
            }
            if let Some(d) = detail {
                // Skip `null`/empty objects to avoid noise hits on "null"/"{}".
                if !d.is_null() {
                    let s = compact_json(d);
                    if s != "{}" && s != "[]" && !s.is_empty() {
                        out.push(s);
                    }
                }
            }
        }
        SessionTranscriptEntry::TerminalChunk { command, text, .. } => {
            if let Some(c) = command {
                if !c.is_empty() {
                    out.push(c.clone());
                }
            }
            if let Some(t) = text {
                if !t.is_empty() {
                    out.push(t.clone());
                }
            }
        }
        SessionTranscriptEntry::FrEvent { payload, .. }
        | SessionTranscriptEntry::Process { payload, .. } => {
            let s = compact_json(payload);
            if !s.is_empty() && s != "{}" && s != "null" {
                out.push(s);
            }
        }
    }
    out
}

/// Compact (no-whitespace) JSON stringification used to make a payload object
/// searchable as one string. Falls back to an empty string on the (practically
/// impossible) serialize error so the caller simply contributes no payload text.
fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight_recorder::{FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType};
    use serde_json::json;
    use uuid::Uuid;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
    }

    fn chat_row(turn: u64, secs: i64, role: &str, content: &str) -> ChatTurnInput {
        ChatTurnInput {
            created_at_utc: at(secs).to_rfc3339(),
            turn_index: turn,
            role: role.to_string(),
            model_role: None,
            content: content.to_string(),
            message_id: format!("msg-{turn}"),
        }
    }

    fn system_event(secs: i64, payload: Value) -> FlightRecorderEvent {
        let mut e = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            payload,
        );
        e.timestamp = at(secs);
        e
    }

    fn terminal_event(secs: i64, session_span: &str, command: &str) -> FlightRecorderEvent {
        let mut e = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            Uuid::now_v7(),
            json!({
                "type": "terminal_command",
                "fr_event": "FR-EVT-TERMINAL-COMMAND-EXEC",
                "session_id": session_span,
                "command": command,
                "cwd": "",
                "exit_code": 0,
                "duration_ms": 0,
                "timed_out": false,
                "cancelled": false,
                "truncated_bytes": 0,
            }),
        )
        .with_session_span(session_span.to_string());
        e.timestamp = at(secs);
        e
    }

    #[test]
    fn merge_orders_by_timestamp_stable() {
        // Interleaved chat + fr + terminal with distinct ts.
        let chat = vec![chat_row(1, 10, "user", "hello"), chat_row(2, 40, "assistant", "hi")];
        let fr = vec![
            system_event(20, json!({ "fr_event_id": "FR-EVT-SWARM-X", "instance_id": "m#0" })),
            terminal_event(30, "term-1", "ls"),
        ];
        let merged = merge_transcript(chat, fr);
        let kinds: Vec<TranscriptKind> = merged.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                TranscriptKind::ChatTurn,
                TranscriptKind::FrEvent,
                TranscriptKind::TerminalChunk,
                TranscriptKind::ChatTurn,
            ]
        );
        // seq is contiguous 0..n.
        for (idx, e) in merged.iter().enumerate() {
            assert_eq!(e.seq(), idx as u64);
        }
    }

    #[test]
    fn merge_equal_timestamps_order_deterministically() {
        // Two chat rows + one fr event ALL at the same ts. Stable tiebreak:
        // chat (rank 0, by turn_index) before fr (rank 1).
        let chat = vec![chat_row(2, 5, "assistant", "second"), chat_row(1, 5, "user", "first")];
        let fr = vec![system_event(5, json!({ "fr_event_id": "FR-EVT-A", "instance_id": "m#0" }))];
        let merged = merge_transcript(chat, fr);
        match &merged[0] {
            SessionTranscriptEntry::ChatTurn { content, .. } => assert_eq!(content, "first"),
            other => panic!("expected first chat turn, got {other:?}"),
        }
        match &merged[1] {
            SessionTranscriptEntry::ChatTurn { content, .. } => assert_eq!(content, "second"),
            other => panic!("expected second chat turn, got {other:?}"),
        }
        assert_eq!(merged[2].kind(), TranscriptKind::FrEvent);
    }

    #[test]
    fn chat_only_session_returns_chat_entries() {
        let merged = merge_transcript(vec![chat_row(1, 1, "user", "x")], vec![]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].kind(), TranscriptKind::ChatTurn);
    }

    #[test]
    fn swarm_only_session_returns_fr_entries() {
        let fr = vec![system_event(1, json!({ "fr_event_id": "FR-EVT-A", "instance_id": "m#0" }))];
        let merged = merge_transcript(vec![], fr);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].kind(), TranscriptKind::FrEvent);
    }

    #[test]
    fn empty_session_returns_no_entries() {
        let merged = merge_transcript(vec![], vec![]);
        assert!(merged.is_empty());
    }

    #[test]
    fn terminal_event_classified_as_terminal_chunk() {
        let fr = vec![terminal_event(1, "term-42", "cargo build")];
        let merged = merge_transcript(vec![], fr);
        assert_eq!(merged.len(), 1);
        match &merged[0] {
            SessionTranscriptEntry::TerminalChunk {
                terminal_session_id,
                command,
                ..
            } => {
                assert_eq!(terminal_session_id, "term-42");
                assert_eq!(command.as_deref(), Some("cargo build"));
            }
            other => panic!("expected TerminalChunk, got {other:?}"),
        }
    }

    #[test]
    fn process_uuid_event_yields_process_entry() {
        // A swarm SessionSpawned carries process_uuid -> BOTH an FrEvent and a
        // synthesized Process row.
        let fr = vec![system_event(
            1,
            json!({
                "fr_event_id": "FR-EVT-SWARM-SESSION-SPAWNED",
                "instance_id": "qwen#0",
                "process_uuid": "11111111-1111-1111-1111-111111111111",
            }),
        )];
        let merged = merge_transcript(vec![], fr);
        let kinds: Vec<TranscriptKind> = merged.iter().map(|e| e.kind()).collect();
        assert!(kinds.contains(&TranscriptKind::FrEvent));
        assert!(kinds.contains(&TranscriptKind::Process));
        let process = merged
            .iter()
            .find(|e| e.kind() == TranscriptKind::Process)
            .unwrap();
        match process {
            SessionTranscriptEntry::Process {
                process_uuid,
                phase,
                ..
            } => {
                assert_eq!(
                    process_uuid.as_deref(),
                    Some("11111111-1111-1111-1111-111111111111")
                );
                assert_eq!(phase, "FR-EVT-SWARM-SESSION-SPAWNED");
            }
            other => panic!("expected Process, got {other:?}"),
        }
    }

    #[test]
    fn filter_by_kinds_hides_other_lanes() {
        let chat = vec![chat_row(1, 1, "user", "x")];
        let fr = vec![terminal_event(2, "t", "ls")];
        let merged = merge_transcript(chat, fr);
        let only_chat = filter_by_kinds(merged.clone(), Some(&[TranscriptKind::ChatTurn]));
        assert_eq!(only_chat.len(), 1);
        assert_eq!(only_chat[0].kind(), TranscriptKind::ChatTurn);
        // seq is preserved from the full merge (chat row was seq 0).
        assert_eq!(only_chat[0].seq(), 0);

        // Empty kinds set is a no-op (all pass).
        let all = filter_by_kinds(merged.clone(), Some(&[]));
        assert_eq!(all.len(), merged.len());
        let none_arg = filter_by_kinds(merged.clone(), None);
        assert_eq!(none_arg.len(), merged.len());
    }

    #[test]
    fn append_terminal_scrollback_inserts_in_order() {
        let fr = vec![
            system_event(10, json!({ "fr_event_id": "FR-EVT-A", "instance_id": "m#0" })),
            system_event(30, json!({ "fr_event_id": "FR-EVT-B", "instance_id": "m#0" })),
        ];
        let merged = merge_transcript(vec![], fr);
        let enriched =
            append_terminal_scrollback(merged, "term-live".to_string(), "raw output".to_string(), at(20));
        assert_eq!(enriched.len(), 3);
        // The scrollback chunk (ts=20) lands between the two fr events.
        match &enriched[1] {
            SessionTranscriptEntry::TerminalChunk { text, .. } => {
                assert_eq!(text.as_deref(), Some("raw output"));
            }
            other => panic!("expected TerminalChunk at index 1, got {other:?}"),
        }
        for (idx, e) in enriched.iter().enumerate() {
            assert_eq!(e.seq(), idx as u64);
        }
    }

    #[test]
    fn malformed_chat_timestamp_sorts_first_not_dropped() {
        let mut bad = chat_row(1, 100, "user", "broken-ts");
        bad.created_at_utc = "not-a-timestamp".to_string();
        let good = chat_row(2, 50, "assistant", "ok");
        let merged = merge_transcript(vec![bad, good], vec![]);
        // Both surface (nothing dropped); the malformed row sorts to epoch (first).
        assert_eq!(merged.len(), 2);
        match &merged[0] {
            SessionTranscriptEntry::ChatTurn { content, .. } => assert_eq!(content, "broken-ts"),
            other => panic!("expected malformed row first, got {other:?}"),
        }
    }

    #[cfg(feature = "duckdb-flight-recorder")]
    #[tokio::test]
    async fn raw_seam_scopes_by_composite_instance_id() {
        use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
        use crate::flight_recorder::FlightRecorder;

        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("in-memory recorder");
        let instance = "qwen2.5-coder#0";
        let other = "qwen2.5-coder#1";

        // (1) A swarm SessionSpawned keyed by composite instance_id in payload.
        recorder
            .record_event(system_event(
                10,
                json!({
                    "fr_event_id": "FR-EVT-SWARM-SESSION-SPAWNED",
                    "instance_id": instance,
                    "process_uuid": "22222222-2222-2222-2222-222222222222",
                }),
            ))
            .await
            .expect("record spawn");

        // (2) A terminal capture event keyed by session_span_id; the capture
        // binding sets session_span = the composite instance id at spawn, so the
        // raw seam's `session_span_id = ?` branch matches.
        recorder
            .record_event(terminal_event(20, instance, "cargo test"))
            .await
            .expect("record terminal");

        // (3) An llm-infer event for a DIFFERENT instance — must NOT match.
        recorder
            .record_event(system_event(
                30,
                json!({ "fr_event_id": "FR-EVT-A", "instance_id": other }),
            ))
            .await
            .expect("record other");

        let scoped = recorder
            .list_session_scoped_events(instance, None, None)
            .await
            .expect("scoped query");

        // Exactly the two events for `instance` (spawn + terminal), ascending.
        assert_eq!(scoped.len(), 2, "expected 2 scoped events, got {scoped:?}");
        assert!(scoped[0].timestamp <= scoped[1].timestamp);

        // Merge them and confirm the lanes materialize: FrEvent + Process (from
        // the spawn) and TerminalChunk (from the terminal event).
        let merged = merge_transcript(vec![], scoped);
        let kinds: Vec<TranscriptKind> = merged.iter().map(|e| e.kind()).collect();
        assert!(kinds.contains(&TranscriptKind::FrEvent));
        assert!(kinds.contains(&TranscriptKind::Process));
        assert!(kinds.contains(&TranscriptKind::TerminalChunk));
    }

    #[cfg(feature = "duckdb-flight-recorder")]
    #[tokio::test]
    async fn raw_seam_time_window_filters() {
        use crate::flight_recorder::duckdb::DuckDbFlightRecorder;
        use crate::flight_recorder::FlightRecorder;

        let recorder = DuckDbFlightRecorder::new_in_memory(7).expect("in-memory recorder");
        let instance = "m#0";
        for secs in [10_i64, 100, 1000] {
            recorder
                .record_event(system_event(
                    secs,
                    json!({ "fr_event_id": "FR-EVT-A", "instance_id": instance }),
                ))
                .await
                .expect("record");
        }
        let scoped = recorder
            .list_session_scoped_events(instance, Some(at(50)), Some(at(500)))
            .await
            .expect("scoped windowed query");
        assert_eq!(scoped.len(), 1);
        assert_eq!(scoped[0].timestamp, at(100));
    }

    #[test]
    fn fr_events_deduped_across_two_seams() {
        // Simulate the caller union: the SAME event_id returned by both seams.
        // The aggregator dedups by event_id before merge; here we assert merge of
        // a deduped vec yields one FrEvent (the merge itself does not dedup, so
        // this documents the contract the caller must honor).
        let mut e = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            json!({ "fr_event_id": "FR-EVT-A", "instance_id": "m#0" }),
        );
        e.timestamp = at(5);
        let event_id = e.event_id;

        // Caller-side dedup by event_id.
        let mut map = std::collections::HashMap::new();
        map.insert(event_id, e.clone());
        map.insert(event_id, e.clone()); // second seam, same id
        let deduped: Vec<FlightRecorderEvent> = map.into_values().collect();
        assert_eq!(deduped.len(), 1);

        let merged = merge_transcript(vec![], deduped);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].kind(), TranscriptKind::FrEvent);
    }

    /// CONTRACT GATE: the serialized JSON the frontend reads MUST use the
    /// camelCase field keys the TS client (`app/src/lib/ipc/session_transcript.ts`)
    /// declares, while the variant TAG (`kind`) stays snake_case. This asserts the
    /// serde casing boundary directly (not against a hand-built JS mock) so the
    /// mismatch that shipped (snake_case fields vs camelCase TS) cannot regress.
    #[test]
    fn entry_serialization_uses_camelcase_fields_with_snakecase_kind() {
        let chat = SessionTranscriptEntry::ChatTurn {
            ts: at(1),
            seq: 0,
            role: "user".to_string(),
            model_role: Some("coder".to_string()),
            content: "hi".to_string(),
            message_id: "m1".to_string(),
        };
        let v = serde_json::to_value(&chat).unwrap();
        assert_eq!(v["kind"], "chat_turn"); // variant tag stays snake_case
        assert!(v.get("modelRole").is_some(), "expected camelCase modelRole");
        assert!(v.get("messageId").is_some(), "expected camelCase messageId");
        assert!(v.get("model_role").is_none(), "snake_case must not leak");
        assert!(v.get("message_id").is_none(), "snake_case must not leak");

        let fr = SessionTranscriptEntry::FrEvent {
            ts: at(1),
            seq: 1,
            event_type: "system".to_string(),
            fr_event: Some("FR-EVT-A".to_string()),
            actor: "system".to_string(),
            model_id: Some("m".to_string()),
            payload: json!({}),
            event_id: "e1".to_string(),
        };
        let v = serde_json::to_value(&fr).unwrap();
        assert_eq!(v["kind"], "fr_event");
        for key in ["eventType", "frEvent", "modelId", "eventId"] {
            assert!(v.get(key).is_some(), "expected camelCase {key}");
        }
        for key in ["event_type", "fr_event", "model_id", "event_id"] {
            assert!(v.get(key).is_none(), "snake_case {key} must not leak");
        }

        let term = SessionTranscriptEntry::TerminalChunk {
            ts: at(1),
            seq: 2,
            terminal_session_id: "t1".to_string(),
            fr_event: None,
            command: Some("ls".to_string()),
            text: Some("out".to_string()),
        };
        let v = serde_json::to_value(&term).unwrap();
        assert_eq!(v["kind"], "terminal_chunk");
        assert!(v.get("terminalSessionId").is_some(), "expected terminalSessionId");
        assert!(v.get("terminal_session_id").is_none(), "snake_case must not leak");

        let proc = SessionTranscriptEntry::Process {
            ts: at(1),
            seq: 3,
            process_uuid: Some("p1".to_string()),
            phase: "spawned".to_string(),
            model_id: Some("m".to_string()),
            payload: json!({}),
        };
        let v = serde_json::to_value(&proc).unwrap();
        assert_eq!(v["kind"], "process");
        assert!(v.get("processUuid").is_some(), "expected processUuid");
        assert!(v.get("modelId").is_some(), "expected modelId");
        assert!(v.get("process_uuid").is_none(), "snake_case must not leak");
    }

    #[test]
    fn transcript_kind_ipc_round_trip() {
        assert_eq!(TranscriptKind::from_ipc("chat_turn"), Some(TranscriptKind::ChatTurn));
        assert_eq!(TranscriptKind::from_ipc("fr_event"), Some(TranscriptKind::FrEvent));
        assert_eq!(
            TranscriptKind::from_ipc("terminal_chunk"),
            Some(TranscriptKind::TerminalChunk)
        );
        assert_eq!(TranscriptKind::from_ipc("process"), Some(TranscriptKind::Process));
        assert_eq!(TranscriptKind::from_ipc("bogus"), None);
        assert_eq!(
            TranscriptKind::from_ipc("agent_activity"),
            Some(TranscriptKind::AgentActivity)
        );
    }

    /// An `FR-EVT-AGENT-*` event classifies to an `AgentActivity` row (NOT a bare
    /// `FrEvent`), copying name/detail/text from the payload.
    fn agent_event(secs: i64, event_id: &str, payload: Value) -> FlightRecorderEvent {
        let mut e = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::Agent,
            Uuid::now_v7(),
            payload,
        )
        .with_session_span("m#0".to_string());
        e.timestamp = at(secs);
        // The id lives under payload.event_id (matching the runtime emit).
        let _ = event_id;
        e
    }

    #[test]
    fn fr_evt_agent_toolcall_classifies_to_agent_activity() {
        let ev = agent_event(
            5,
            "FR-EVT-AGENT-TOOLCALL",
            json!({
                "event_id": "FR-EVT-AGENT-TOOLCALL",
                "activity_kind": "tool_call",
                "name": "Bash",
                "detail": {"command": "ls -la"},
                "instance_id": "m#0",
            }),
        );
        let entries = entries_from_fr_event(&ev);
        assert_eq!(entries.len(), 1, "single AgentActivity row");
        match &entries[0] {
            SessionTranscriptEntry::AgentActivity {
                activity_kind,
                name,
                detail,
                text,
                ..
            } => {
                assert_eq!(activity_kind, "tool_call");
                assert_eq!(name.as_deref(), Some("Bash"));
                assert_eq!(
                    detail.as_ref().and_then(|d| d.get("command")).unwrap(),
                    "ls -la"
                );
                assert!(text.is_none());
            }
            other => panic!("expected AgentActivity, got {other:?}"),
        }
    }

    #[test]
    fn fr_evt_agent_thinking_classifies_with_text() {
        let ev = agent_event(
            5,
            "FR-EVT-AGENT-THINKING",
            json!({"event_id": "FR-EVT-AGENT-THINKING", "activity_kind": "thinking",
                   "text": "let me reason"}),
        );
        match &entries_from_fr_event(&ev)[0] {
            SessionTranscriptEntry::AgentActivity {
                activity_kind, text, ..
            } => {
                assert_eq!(activity_kind, "thinking");
                assert_eq!(text.as_deref(), Some("let me reason"));
            }
            other => panic!("expected AgentActivity, got {other:?}"),
        }
    }

    #[test]
    fn agent_activity_merges_in_timestamp_order_and_filters() {
        let chat = vec![chat_row(1, 10, "user", "hello")];
        let fr = vec![
            agent_event(
                20,
                "FR-EVT-AGENT-TEXT",
                json!({"event_id": "FR-EVT-AGENT-TEXT", "activity_kind": "text",
                       "text": "answer"}),
            ),
            terminal_event(30, "term-1", "ls"),
        ];
        let merged = merge_transcript(chat, fr);
        let kinds: Vec<TranscriptKind> = merged.iter().map(|e| e.kind()).collect();
        assert_eq!(
            kinds,
            vec![
                TranscriptKind::ChatTurn,
                TranscriptKind::AgentActivity,
                TranscriptKind::TerminalChunk,
            ]
        );
        // Isolate the agent lane.
        let only = filter_by_kinds(merged, Some(&[TranscriptKind::AgentActivity]));
        assert_eq!(only.len(), 1);
        assert_eq!(only[0].kind(), TranscriptKind::AgentActivity);
    }

    #[test]
    fn searchable_text_extracts_per_variant_fields() {
        // ChatTurn -> content.
        let chat = SessionTranscriptEntry::ChatTurn {
            ts: at(1),
            seq: 0,
            role: "user".to_string(),
            model_role: None,
            content: "operator forgot the gate".to_string(),
            message_id: "m1".to_string(),
        };
        assert_eq!(searchable_text(&chat), vec!["operator forgot the gate"]);

        // AgentActivity -> name + text + compact(detail).
        let agent = SessionTranscriptEntry::AgentActivity {
            ts: at(1),
            seq: 0,
            activity_kind: "tool_call".to_string(),
            name: Some("Bash".to_string()),
            detail: Some(json!({ "command": "cargo build" })),
            text: None,
            event_id: "ev".to_string(),
        };
        let texts = searchable_text(&agent);
        assert!(texts.iter().any(|t| t == "Bash"));
        assert!(texts.iter().any(|t| t.contains("cargo build")));

        // TerminalChunk -> command + text.
        let term = SessionTranscriptEntry::TerminalChunk {
            ts: at(1),
            seq: 0,
            terminal_session_id: "t1".to_string(),
            fr_event: None,
            command: Some("npm test".to_string()),
            text: Some("3 passing".to_string()),
        };
        let texts = searchable_text(&term);
        assert!(texts.iter().any(|t| t == "npm test"));
        assert!(texts.iter().any(|t| t == "3 passing"));

        // FrEvent -> compact payload JSON.
        let fr = SessionTranscriptEntry::FrEvent {
            ts: at(1),
            seq: 0,
            event_type: "system".to_string(),
            fr_event: Some("FR-EVT-A".to_string()),
            actor: "system".to_string(),
            model_id: None,
            payload: json!({ "needle": "haystack" }),
            event_id: "e1".to_string(),
        };
        let texts = searchable_text(&fr);
        assert_eq!(texts.len(), 1);
        assert!(texts[0].contains("needle"));
        assert!(texts[0].contains("haystack"));

        // Empty / null payloads contribute nothing (no noise hits).
        let empty_fr = SessionTranscriptEntry::FrEvent {
            ts: at(1),
            seq: 0,
            event_type: "system".to_string(),
            fr_event: None,
            actor: "system".to_string(),
            model_id: None,
            payload: json!({}),
            event_id: "e2".to_string(),
        };
        assert!(searchable_text(&empty_fr).is_empty());
    }

    #[test]
    fn agent_activity_serde_casing() {
        let entry = SessionTranscriptEntry::AgentActivity {
            ts: at(1),
            seq: 0,
            activity_kind: "tool_call".to_string(),
            name: Some("Bash".to_string()),
            detail: Some(json!({"command": "ls"})),
            text: None,
            event_id: "ev-1".to_string(),
        };
        let v = serde_json::to_value(&entry).unwrap();
        assert_eq!(v["kind"], "agent_activity");
        assert_eq!(v["activityKind"], "tool_call");
        assert!(v.get("activity_kind").is_none(), "snake_case must not leak");
        assert!(v.get("eventId").is_some(), "expected eventId");
        // text is None -> skipped.
        assert!(v.get("text").is_none(), "None text must be skipped");
    }
}
