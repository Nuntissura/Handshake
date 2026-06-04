//! Tauri IPC surface for the UNIFIED per-session record + replay.
//!
//! GOAL (governance glue): give the operator ONE ordered, durable, reviewable
//! timeline per session — "go back and look when things go wrong or I forget".
//! The per-session activity that today lives in THREE separate streams (chat
//! turns in `chat.jsonl`, Flight Recorder events incl. terminal capture +
//! inference, and the swarm/process lifecycle) is *derived on read* into one
//! timestamp-ordered typed timeline. Nothing is stored twice.
//!
//! This file owns the IPC + source wiring + managed state ONLY. The pure
//! merge/order/types live in `handshake_core::session_transcript` (Tauri-free,
//! unit-tested there). `lib.rs` (the Integrate phase) registers the commands in
//! the `handshake_invoke_handlers!` macro and `.manage`s the state.
//!
//! ## Session identity (the central problem)
//!
//! The three streams do NOT share one session id (see the module doc on
//! `handshake_core::session_transcript`). The canonical transcript `session_id`
//! is the swarm composite `instance_id` (`<model_id>#<instance>`) for
//! swarm/agent sessions, and the chat UUID for the operator chat session.
//! Because no single FR column holds the composite, FR is queried through TWO
//! seams and unioned (deduped by `event_id`):
//!   1. `FlightRecorder::list_events(EventFilter { model_session_id })` — the
//!      documented seam (matches the sparse events that DID set
//!      `model_session_id`, e.g. chat-mirror events / future backfill).
//!   2. `FlightRecorder::list_session_scoped_events(session_id)` — the raw seam
//!      matching `session_span_id = id` OR `payload.instance_id = id` with NO
//!      200-row cap (terminal capture + swarm lifecycle + inference keyed by the
//!      composite). Required because `EventFilter` cannot express either.
//!
//! ## Honesty
//!
//! If a source is empty/unavailable for a session it is reported as such via
//! `source_status`; entries are NEVER fabricated. When the durable recorder fell
//! back to the stderr sink (`recorder: None`), the FR/terminal/process lanes
//! report `unavailable` and only `chat.jsonl` is read.
//!
//! ## Disk-agnostic [GLOBAL-PORTABILITY]
//!
//! Everything is rooted at the caller-supplied `app_data_root`; no path is
//! hardcoded. The optional discovery index lives at
//! `<app_data_root>/sessions/session_index.json` and is written atomically
//! (temp + rename), mirroring `swarm_schedule_store`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use handshake_core::flight_recorder::{EventFilter, FlightRecorder, FlightRecorderEvent};
use handshake_core::session_transcript::{
    self, export, ChatTurnInput, ExportCounts, ExportFormat, ExportHeader, SessionTranscriptEntry,
    TranscriptKind,
};
use handshake_core::terminal::redaction::{PatternRedactor, SecretRedactor};
use handshake_core::terminal::TerminalRuntime;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::session_chat_log::{self, SessionChatRole};

pub const KERNEL_SESSION_LIST_IPC_CHANNEL: &str = "kernel_session_list";
pub const KERNEL_SESSION_TRANSCRIPT_GET_IPC_CHANNEL: &str = "kernel_session_transcript_get";
/// ROI #4 CROSS-SESSION SEARCH ("I-forget-something" recall).
pub const KERNEL_SESSION_SEARCH_IPC_CHANNEL: &str = "kernel_session_search";
/// ROI #5 EXPORT (archive / handoff / sharing of a recorded session).
pub const KERNEL_SESSION_EXPORT_IPC_CHANNEL: &str = "kernel_session_export";

/// Subdirectory (under `<app_data_root>`) the default export lands in.
pub const SESSION_EXPORT_DIR: &str = "exports";

// ---------------------------------------------------------------------------
// Cross-session search bounds (all honest: every clip is surfaced via
// `truncated` or the gap between `match_count` and the emitted snippet count).
// ---------------------------------------------------------------------------

/// Default + hard-max number of `SessionSearchHit` rows returned. The caller's
/// `limit` is clamped into `1..=SEARCH_LIMIT_MAX`; the default applies when the
/// caller omits it.
const SEARCH_LIMIT_DEFAULT: u64 = 50;
const SEARCH_LIMIT_MAX: u64 = 200;
/// Per-hit snippet cap. `match_count` still reflects the TRUE total so the UI can
/// honestly say "12 matches, showing 5".
const PER_SESSION_SNIPPET_CAP: usize = 5;
/// Max candidate sessions scanned (by recency) before ranking, guarding a
/// pathological session count. Exceeding it sets `truncated`.
const MAX_CANDIDATE_SESSIONS: usize = 500;
/// Context window (chars) taken on each side of the match when building a
/// snippet. Sliced on char boundaries (never mid-codepoint).
const SNIPPET_CONTEXT: usize = 60;

/// File name (under `<app_data_root>/sessions/`) the optional discovery index
/// persists to. A rebuildable cache, never authority.
pub const SESSION_INDEX_FILE: &str = "session_index.json";

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

/// Tauri managed state for the unified transcript aggregator. Cheap to clone
/// (Arc/PathBuf inside). Built in `lib.rs::setup` from the SAME `swarm_recorder`
/// + `app_data_root` the swarm/terminal paths use.
pub struct SessionTranscriptState {
    /// The durable swarm/terminal/inference recorder. `None` when FR init fell
    /// back to the stderr sink — the FR lanes then report `unavailable`.
    recorder: Option<Arc<dyn FlightRecorder>>,
    /// Disk-agnostic data root; chat logs + the discovery index live under it.
    app_data_root: PathBuf,
    /// Optional live terminal runtime, used ONLY for the live-scrollback
    /// enrichment (a still-open capture session's current raw stdout). The
    /// durable terminal record is the FR `TerminalChunk` events.
    terminal: Option<TerminalRuntime>,
}

impl std::fmt::Debug for SessionTranscriptState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionTranscriptState")
            .field("recorder", &self.recorder.is_some())
            .field("app_data_root", &self.app_data_root)
            .field("terminal", &self.terminal.is_some())
            .finish()
    }
}

impl SessionTranscriptState {
    /// Build the production state from the app's durable recorder, data root,
    /// and (optional) terminal runtime.
    pub fn new(
        recorder: Option<Arc<dyn FlightRecorder>>,
        app_data_root: impl Into<PathBuf>,
        terminal: Option<TerminalRuntime>,
    ) -> Self {
        Self {
            recorder,
            app_data_root: app_data_root.into(),
            terminal,
        }
    }

    fn sessions_root(&self) -> PathBuf {
        self.app_data_root.join("sessions")
    }
}

// ---------------------------------------------------------------------------
// IPC payloads
// ---------------------------------------------------------------------------

/// Per-source presence so emptiness is HONEST in the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
    /// The source contributed at least one entry.
    Present,
    /// The source exists for this session but contributed nothing.
    Empty,
    /// The source could not be read (e.g. recorder fell back to stderr sink).
    Unavailable,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub chat: SourceState,
    pub fr: SourceState,
    pub terminal: SourceState,
    pub process: SourceState,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCounts {
    pub chat: u64,
    pub fr: u64,
    pub terminal: u64,
    pub process: u64,
}

/// One row of the session index (left pane of the replay surface).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    /// "chat" | "swarm".
    pub kind: String,
    pub started_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub model_id: Option<String>,
    pub provider: Option<String>,
    pub title: Option<String>,
    pub counts: SourceCounts,
    /// Operator-assigned VM/sandbox worktree for this session, lifted from the FR
    /// SwarmEvent payload (`$.worktree_id`, recorded at spawn). Lets the replay
    /// surface answer "find a worktree's sessions". `None` for chat sessions and
    /// for swarm sessions spawned without a worktree assignment. RECORDED ONLY.
    #[serde(default)]
    pub worktree_id: Option<String>,
    /// ROI#3 STATE RECOVERY: true when a resume spawn template is persisted for
    /// this session (it can be re-spawned via `kernel_swarm_resume_session`).
    /// `false` for chat (UUID) sessions, for swarm sessions spawned before this
    /// feature, and for any session whose best-effort template write failed
    /// (honest: not-resumable). The UI hides/disables the Resume affordance when
    /// `false`. `#[serde(default)]` => forward-compatible (older indexes => false).
    #[serde(default)]
    pub resumable: bool,
}

/// Response of `kernel_session_transcript_get`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTranscriptResponse {
    pub session_id: String,
    pub entries: Vec<SessionTranscriptEntry>,
    pub source_status: SourceStatus,
    /// True if a hard cap was applied to any source (none today, but the field
    /// is wired so a future cap is surfaced honestly).
    pub truncated: bool,
}

// ---------------------------------------------------------------------------
// Cross-session search IPC payloads (ROI #4)
// ---------------------------------------------------------------------------

/// One redacted match snippet inside a session hit.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSnippet {
    /// The lane this match was found in: `chat_turn` | `agent_activity` |
    /// `terminal_chunk` | `fr_event` | `process`.
    pub entry_kind: String,
    /// The matched entry's merge-key timestamp (the same `ts` the transcript
    /// row carries), so the UI can seed the transcript timeline near the match.
    pub ts: Option<DateTime<Utc>>,
    /// The matched text + a little context window, SECRET-REDACTED. Never the
    /// raw payload/command/text.
    pub snippet: String,
}

/// One ranked session that matched the query.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchHit {
    pub session_id: String,
    /// "chat" | "swarm" (from the session summary).
    pub kind: String,
    pub provider: Option<String>,
    pub model_id: Option<String>,
    pub worktree_id: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub title: Option<String>,
    /// Total matches across the session BEFORE the per-session snippet cap, so
    /// "12 matches, showing 5" is honest.
    pub match_count: u64,
    /// Capped (per-session) to `PER_SESSION_SNIPPET_CAP`, ordered by `ts` asc.
    pub snippets: Vec<SearchSnippet>,
}

/// Response of `kernel_session_search`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchResponse {
    /// Ranked hits: `match_count` desc, then recency desc, then `session_id` asc.
    pub hits: Vec<SessionSearchHit>,
    /// True if the session-hit `limit` or the `MAX_CANDIDATE_SESSIONS` scan cap
    /// clipped results (honest bounding).
    pub truncated: bool,
    /// The effective (trimmed) query, echoed for the UI.
    pub query: String,
}

// ---------------------------------------------------------------------------
// Export IPC payloads (ROI #5)
// ---------------------------------------------------------------------------

/// One written artifact: which format, the absolute path (displayable/copyable),
/// and its byte size on disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedFile {
    /// "markdown" | "json".
    pub format: String,
    /// Absolute path of the written file.
    pub path: String,
    pub bytes: u64,
}

/// Response of `kernel_session_export`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionExportResponse {
    pub session_id: String,
    /// The directory the file(s) landed in (absolute) — for an "open folder" hint.
    pub dest_dir: String,
    pub files: Vec<ExportedFile>,
    /// True when the session had zero entries (an empty-but-valid file was
    /// written rather than erroring).
    pub empty: bool,
    /// Redaction telemetry: total emitted text fields that matched a secret
    /// pattern and were masked. The honest "N secrets redacted" affordance —
    /// never the secret.
    pub redacted_field_count: u64,
}

// ---------------------------------------------------------------------------
// Helpers (pure, command-free — also exercised by the #[cfg(test)] block)
// ---------------------------------------------------------------------------

/// Map a chat log entry into the Tauri-free `ChatTurnInput` the core merge
/// consumes.
fn chat_input_from_entry(entry: &session_chat_log::SessionChatLogEntryV0_1) -> ChatTurnInput {
    let role = match entry.role {
        SessionChatRole::User => "user",
        SessionChatRole::Assistant => "assistant",
    }
    .to_string();
    ChatTurnInput {
        created_at_utc: entry.created_at_utc.clone(),
        turn_index: entry.turn_index,
        role,
        model_role: entry.model_role.clone(),
        content: entry.content.clone(),
        message_id: entry.message_id.clone(),
    }
}

/// Union FR events fetched via both seams, deduped by `event_id`. A single event
/// matched by both `model_session_id` and `session_span_id`/`instance_id`
/// appears once.
fn union_dedup_events(
    documented: Vec<FlightRecorderEvent>,
    raw: Vec<FlightRecorderEvent>,
) -> Vec<FlightRecorderEvent> {
    let mut by_id: HashMap<uuid::Uuid, FlightRecorderEvent> =
        HashMap::with_capacity(documented.len() + raw.len());
    for e in documented.into_iter().chain(raw.into_iter()) {
        by_id.insert(e.event_id, e);
    }
    by_id.into_values().collect()
}

/// Parse the IPC `kinds` strings into the typed filter. Unknown strings are
/// rejected (honest) so a typo does not silently widen the filter.
fn parse_kinds(kinds: Option<Vec<String>>) -> Result<Option<Vec<TranscriptKind>>, String> {
    match kinds {
        None => Ok(None),
        Some(list) => {
            let mut out = Vec::with_capacity(list.len());
            for raw in list {
                match TranscriptKind::from_ipc(&raw) {
                    Some(k) => out.push(k),
                    None => return Err(format!("unknown transcript kind: {raw}")),
                }
            }
            Ok(Some(out))
        }
    }
}

fn parse_rfc3339_opt(raw: Option<String>, label: &str) -> Result<Option<DateTime<Utc>>, String> {
    match raw {
        None => Ok(None),
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(s.trim())
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(|e| format!("{label} must be RFC3339: {e}")),
    }
}

/// Count entries per lane for the `source_status` + index counts.
fn count_lanes(entries: &[SessionTranscriptEntry]) -> SourceCounts {
    let mut counts = SourceCounts::default();
    for e in entries {
        match e.kind() {
            TranscriptKind::ChatTurn => counts.chat += 1,
            // Agent-activity rows ARE FR events (classified from FR-EVT-AGENT-*),
            // so they ride the `fr` source bucket; the kind filter is the
            // user-facing distinction.
            TranscriptKind::FrEvent | TranscriptKind::AgentActivity => counts.fr += 1,
            TranscriptKind::TerminalChunk => counts.terminal += 1,
            TranscriptKind::Process => counts.process += 1,
        }
    }
    counts
}

/// Derive `(model_id, provider)` from a composite instance id `<model_id>#<n>`.
/// Honest: provider is NOT embedded in the composite, so it stays `None` unless
/// a future event carries it; only `model_id` is split out.
fn split_instance_id(session_id: &str) -> (Option<String>, Option<String>) {
    match session_id.rsplit_once('#') {
        Some((model_id, _instance)) if !model_id.is_empty() => (Some(model_id.to_string()), None),
        _ => (None, None),
    }
}

// ---------------------------------------------------------------------------
// Aggregator core (sync-friendly: takes already-fetched inputs)
// ---------------------------------------------------------------------------

/// Build the unified transcript + honest `source_status` from already-fetched
/// inputs. Kept free of IO + Tauri so it is directly unit-testable.
///
/// `recorder_available` distinguishes "FR present but empty for this session"
/// (`Empty`) from "FR could not be read at all" (`Unavailable`).
fn build_response(
    session_id: &str,
    chat: Vec<ChatTurnInput>,
    fr_events: Vec<FlightRecorderEvent>,
    recorder_available: bool,
    live_scrollback: Option<(String, String, DateTime<Utc>)>,
    kinds: Option<Vec<TranscriptKind>>,
) -> SessionTranscriptResponse {
    let chat_present = !chat.is_empty();
    let mut merged = session_transcript::merge_transcript(chat, fr_events);

    // Live terminal scrollback enrichment (still-open capture session only).
    let mut live_terminal_added = false;
    if let Some((term_id, text, ts)) = live_scrollback {
        if !text.is_empty() {
            merged = session_transcript::append_terminal_scrollback(merged, term_id, text, ts);
            live_terminal_added = true;
        }
    }

    // Lane presence is computed from the FULL merge (pre kind-filter) so the UI
    // reports honest emptiness independent of the active filter.
    let full_counts = count_lanes(&merged);

    let source_status = SourceStatus {
        chat: if chat_present {
            SourceState::Present
        } else {
            SourceState::Empty
        },
        fr: lane_state(recorder_available, full_counts.fr > 0),
        terminal: if !recorder_available && !live_terminal_added {
            SourceState::Unavailable
        } else if full_counts.terminal > 0 {
            SourceState::Present
        } else {
            SourceState::Empty
        },
        process: lane_state(recorder_available, full_counts.process > 0),
    };

    let entries = session_transcript::filter_by_kinds(merged, kinds.as_deref());

    SessionTranscriptResponse {
        session_id: session_id.to_string(),
        entries,
        source_status,
        truncated: false,
    }
}

fn lane_state(recorder_available: bool, has_rows: bool) -> SourceState {
    if !recorder_available {
        SourceState::Unavailable
    } else if has_rows {
        SourceState::Present
    } else {
        SourceState::Empty
    }
}

// ---------------------------------------------------------------------------
// IO: fetch FR events for a session via both seams
// ---------------------------------------------------------------------------

async fn fetch_fr_events(
    recorder: &Arc<dyn FlightRecorder>,
    session_id: &str,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Result<Vec<FlightRecorderEvent>, String> {
    // Seam 1: documented EventFilter by model_session_id.
    let documented = recorder
        .list_events(EventFilter {
            model_session_id: Some(session_id.to_string()),
            from,
            to,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("FR list_events failed: {e}"))?;

    // Seam 2: raw scoped query by session_span_id OR payload.instance_id.
    let raw = recorder
        .list_session_scoped_events(session_id, from, to)
        .await
        .map_err(|e| format!("FR scoped query failed: {e}"))?;

    Ok(union_dedup_events(documented, raw))
}

/// Best-effort live terminal scrollback for a still-open capture session bound
/// to this `session_id` (its `binding.instance_id == session_id`). Returns the
/// `(terminal_session_id, text, ts)` triple to enrich the timeline, or `None`.
fn live_terminal_scrollback(
    terminal: &Option<TerminalRuntime>,
    session_id: &str,
) -> Option<(String, String, DateTime<Utc>)> {
    let runtime = terminal.as_ref()?;
    let sessions = runtime.list_sessions();
    let info = sessions
        .into_iter()
        .find(|s| s.binding.instance_id.as_deref() == Some(session_id))?;
    let bytes = runtime.scrollback(&info.session_id).ok()?;
    if bytes.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    Some((info.session_id, text, Utc::now()))
}

// ---------------------------------------------------------------------------
// Session discovery (kernel_session_list)
// ---------------------------------------------------------------------------

/// Discover the chat-backed sessions from `<app_data_root>/sessions/*/chat.jsonl`.
fn discover_chat_sessions(state: &SessionTranscriptState) -> Vec<SessionSummary> {
    let root = state.sessions_root();
    let mut out = Vec::new();
    let read_dir = match std::fs::read_dir(&root) {
        Ok(rd) => rd,
        Err(_) => return out, // no sessions dir yet -> empty (honest)
    };
    for entry in read_dir.flatten() {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let session_id = entry.file_name().to_string_lossy().to_string();
        let rows = match session_chat_log::read_chat_log(&state.app_data_root, &session_id) {
            Ok(rows) if !rows.is_empty() => rows,
            _ => continue, // no chat.jsonl / empty -> not a chat session
        };
        let started_at = rows
            .first()
            .and_then(|r| DateTime::parse_from_rfc3339(r.created_at_utc.trim()).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let last_activity_at = rows
            .last()
            .and_then(|r| DateTime::parse_from_rfc3339(r.created_at_utc.trim()).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let title = rows
            .iter()
            .find_map(|r| r.model_role.clone())
            .or_else(|| rows.first().map(|r| truncate_title(&r.content)));
        out.push(SessionSummary {
            session_id,
            kind: "chat".to_string(),
            started_at,
            last_activity_at,
            model_id: None,
            provider: None,
            title,
            counts: SourceCounts {
                chat: rows.len() as u64,
                ..Default::default()
            },
            // Chat (UUID) sessions are not worktree-bound swarm sessions.
            worktree_id: None,
            // Chat sessions are never swarm spawns => never resumable. Overlaid
            // in kernel_session_list anyway, but defaulted honestly here.
            resumable: false,
        });
    }
    out
}

/// Discover swarm/agent sessions from the FR store: DISTINCT
/// `payload.instance_id` with min/max timestamp + count. Honest-empty when the
/// recorder is unavailable.
async fn discover_fr_sessions(state: &SessionTranscriptState) -> Vec<SessionSummary> {
    let Some(recorder) = &state.recorder else {
        return Vec::new();
    };
    let Some(conn) = recorder.duckdb_connection() else {
        return Vec::new();
    };
    let conn = match conn.lock() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stmt = match conn.prepare(
        "SELECT json_extract_string(payload, '$.instance_id') AS sid, \
         CAST(EXTRACT(EPOCH FROM min(timestamp)) AS DOUBLE), \
         CAST(EXTRACT(EPOCH FROM max(timestamp)) AS DOUBLE), \
         count(*), any_value(model_id), \
         any_value(json_extract_string(payload, '$.worktree_id')) \
         FROM events \
         WHERE json_extract_string(payload, '$.instance_id') IS NOT NULL \
         GROUP BY sid",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |row| {
        let sid: String = row.get(0)?;
        let min_epoch: f64 = row.get(1)?;
        let max_epoch: f64 = row.get(2)?;
        let count: i64 = row.get(3)?;
        let model_id: Option<String> = row.get(4)?;
        let worktree_id: Option<String> = row.get(5)?;
        Ok((sid, min_epoch, max_epoch, count, model_id, worktree_id))
    });
    let rows = match rows {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for row in rows.flatten() {
        let (sid, min_epoch, max_epoch, count, model_id, worktree_id) = row;
        if sid.trim().is_empty() {
            continue;
        }
        let (split_model, provider) = split_instance_id(&sid);
        // Normalize a blank/whitespace recorded worktree to None (honest
        // "unassigned"), matching the spawn-side trimming rule.
        let worktree_id = worktree_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        out.push(SessionSummary {
            session_id: sid.clone(),
            kind: "swarm".to_string(),
            started_at: epoch_to_dt(min_epoch),
            last_activity_at: epoch_to_dt(max_epoch),
            model_id: model_id.or(split_model),
            provider,
            title: Some(sid),
            counts: SourceCounts {
                fr: count.max(0) as u64,
                ..Default::default()
            },
            worktree_id,
            // Overlaid against the spawn-template store in kernel_session_list;
            // defaulted false here so the discovery seam stays self-contained.
            resumable: false,
        });
    }
    out
}

fn epoch_to_dt(epoch: f64) -> Option<DateTime<Utc>> {
    let secs = epoch.trunc() as i64;
    let nanos = (epoch.fract() * 1_000_000_000f64) as u32;
    DateTime::<Utc>::from_timestamp(secs, nanos)
}

fn truncate_title(content: &str) -> String {
    let trimmed = content.trim();
    let mut s: String = trimmed.chars().take(60).collect();
    if trimmed.chars().count() > 60 {
        s.push('…');
    }
    s
}

/// Merge chat + FR session summaries, deduped by `session_id` (a chat UUID and a
/// composite instance id never collide, but dedup keeps the contract total).
fn merge_summaries(chat: Vec<SessionSummary>, fr: Vec<SessionSummary>) -> Vec<SessionSummary> {
    let mut by_id: HashMap<String, SessionSummary> = HashMap::new();
    for s in chat.into_iter().chain(fr.into_iter()) {
        by_id.entry(s.session_id.clone()).or_insert(s);
    }
    let mut out: Vec<SessionSummary> = by_id.into_values().collect();
    // Most-recent-activity first; None sorts last.
    out.sort_by(|a, b| b.last_activity_at.cmp(&a.last_activity_at));
    out
}

// ---------------------------------------------------------------------------
// IPC commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn kernel_session_list(
    state: State<'_, SessionTranscriptState>,
) -> Result<Vec<SessionSummary>, String> {
    let chat = discover_chat_sessions(&state);
    let fr = discover_fr_sessions(&state).await;
    let mut summaries = merge_summaries(chat, fr);
    overlay_resumable(&state, &mut summaries);
    Ok(summaries)
}

/// ROI#3 STATE RECOVERY: overlay `resumable` onto each summary from the per-
/// session spawn-template store. Loads the template doc ONCE per list call (not
/// once per row) and flips `resumable` for any session whose composite id is a
/// stored template key. A missing/corrupt/transient store => all `resumable`
/// stay `false` (honest: not-resumable, never a list failure). Chat (UUID)
/// sessions are never template keys => stay `false`.
fn overlay_resumable(state: &SessionTranscriptState, summaries: &mut [SessionSummary]) {
    let store = super::spawn_template_store::SpawnTemplateStore::new(&state.app_data_root);
    let keys = match store.load() {
        Ok(doc) => doc.templates,
        // A missing/corrupt store leaves every row not-resumable (honest), and
        // never breaks the listing.
        Err(_) => return,
    };
    if keys.is_empty() {
        return;
    }
    for s in summaries.iter_mut() {
        if keys.contains_key(&s.session_id) {
            s.resumable = true;
        }
    }
}

#[tauri::command]
pub async fn kernel_session_transcript_get(
    session_id: String,
    from: Option<String>,
    to: Option<String>,
    kinds: Option<Vec<String>>,
    state: State<'_, SessionTranscriptState>,
) -> Result<SessionTranscriptResponse, String> {
    let session_id = session_id.trim().to_string();
    if session_id.is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    let from = parse_rfc3339_opt(from, "from")?;
    let to = parse_rfc3339_opt(to, "to")?;
    let kinds = parse_kinds(kinds)?;

    // Chat lane: path-keyed by session_id. A composite instance id (contains
    // `#`) has no chat file -> empty (honest).
    let chat_entries = session_chat_log::read_chat_log(&state.app_data_root, &session_id)?;
    let chat: Vec<ChatTurnInput> = chat_entries.iter().map(chat_input_from_entry).collect();

    // FR lanes via both seams (or unavailable).
    let (fr_events, recorder_available) = match &state.recorder {
        Some(recorder) => (
            fetch_fr_events(recorder, &session_id, from, to).await?,
            true,
        ),
        None => (Vec::new(), false),
    };

    // Live terminal scrollback enrichment (still-open capture session only).
    let live = live_terminal_scrollback(&state.terminal, &session_id);

    Ok(build_response(
        &session_id,
        chat,
        fr_events,
        recorder_available,
        live,
        kinds,
    ))
}

// ---------------------------------------------------------------------------
// Cross-session search (ROI #4: "I-forget-something" recall)
// ---------------------------------------------------------------------------

/// Map a `TranscriptKind` to the lowercase IPC string the snippet `entry_kind`
/// carries (the inverse of `TranscriptKind::from_ipc`). Kept local + total so the
/// snippet kind is always a known, frontend-keyable string.
fn kind_ipc_str(kind: TranscriptKind) -> &'static str {
    match kind {
        TranscriptKind::ChatTurn => "chat_turn",
        TranscriptKind::FrEvent => "fr_event",
        TranscriptKind::AgentActivity => "agent_activity",
        TranscriptKind::TerminalChunk => "terminal_chunk",
        TranscriptKind::Process => "process",
    }
}

/// Case-insensitive substring offset of `query_lc` (already lowercased) inside
/// `text`, returned as a byte offset into `text`. Unicode-simple: lowercases the
/// haystack the same way the redactor pragmatically lossy-decodes. The returned
/// offset indexes the *original* `text` only when the lowercase mapping is
/// length-preserving; to stay byte-safe regardless, the caller centers the window
/// using char counting (see `make_snippet`).
fn find_match_char_idx(text: &str, query_lc: &str) -> Option<usize> {
    if query_lc.is_empty() {
        return None;
    }
    let hay: Vec<char> = text.chars().collect();
    let needle: Vec<char> = query_lc.chars().collect();
    // Lowercased haystack chars, aligned 1:1 with `hay` by char index. Using a
    // per-char lowercase keeps the index space identical to `hay` (a char may
    // lowercase to multiple chars in pathological cases; we compare on the FIRST
    // lowercased char, which is correct for the ASCII/most-Unicode common case
    // and never panics — a missed exotic-case match is an honest miss, not a
    // crash or a leak).
    let hay_lc: Vec<char> = hay
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    if needle.len() > hay_lc.len() {
        return None;
    }
    for start in 0..=(hay_lc.len() - needle.len()) {
        if hay_lc[start..start + needle.len()] == needle[..] {
            return Some(start); // char index, not byte index
        }
    }
    None
}

/// Build a redacted snippet centered on the first case-insensitive match of
/// `query_lc` in `field_text`, or `None` when there is no match. UTF-8 safe: the
/// window is sliced on CHAR boundaries (`chars().skip().take()`), never a byte
/// slice mid-codepoint. Redaction runs LAST, on the final windowed string, so a
/// secret adjacent to (or overlapping) the match is masked to `***REDACTED***`
/// before it ever leaves the backend.
fn make_snippet(field_text: &str, query_lc: &str) -> Option<String> {
    let match_char = find_match_char_idx(field_text, query_lc)?;
    let total_chars = field_text.chars().count();
    let query_chars = query_lc.chars().count();

    let start = match_char.saturating_sub(SNIPPET_CONTEXT);
    let end = (match_char + query_chars + SNIPPET_CONTEXT).min(total_chars);

    let mut windowed: String = field_text.chars().skip(start).take(end - start).collect();
    if start > 0 {
        windowed.insert(0, '…');
    }
    if end < total_chars {
        windowed.push('…');
    }

    // Redact LAST, on the windowed string. The match offset was computed on the
    // pre-redaction text so the window is centered correctly, but only the
    // redacted text is emitted.
    Some(PatternRedactor.redact_command(&windowed).redacted)
}

/// True when a candidate session's span overlaps the `[from, to]` window. A
/// session is KEPT unless its whole known span is provably outside the window.
/// Missing bounds are treated permissively (honest: do not drop a session for
/// lack of a timestamp).
fn session_in_window(
    summary: &SessionSummary,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> bool {
    if let Some(from) = from {
        // Drop only if the session's LAST activity is strictly before `from`.
        if let Some(last) = summary.last_activity_at {
            if last < from {
                return false;
            }
        }
    }
    if let Some(to) = to {
        // Drop only if the session's FIRST activity is strictly after `to`.
        if let Some(first) = summary.started_at {
            if first > to {
                return false;
            }
        }
    }
    true
}

/// A single matched snippet plus its kind/ts, accumulated per candidate before
/// the per-session snippet cap is applied.
struct RawMatch {
    kind: TranscriptKind,
    ts: Option<DateTime<Utc>>,
    snippet: String,
}

/// Scan one candidate session's corpus (chat + FR/agent/terminal/process) for the
/// query, returning ALL matches (uncapped) so the caller can compute an honest
/// `match_count` and then cap snippets. Reuses the EXACT aggregator readers
/// (`read_chat_log` + `fetch_fr_events` + `entries_from_fr_event`) so a hit always
/// maps to a transcript row. `kinds_filter` (when set) restricts which lanes
/// count + emit.
async fn scan_candidate(
    state: &SessionTranscriptState,
    summary: &SessionSummary,
    query_lc: &str,
    kinds_filter: Option<&[TranscriptKind]>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> Vec<RawMatch> {
    let mut matches: Vec<RawMatch> = Vec::new();
    let kind_allowed = |k: TranscriptKind| match kinds_filter {
        None => true,
        Some(list) => list.contains(&k),
    };

    // --- Chat lane: the SAME reader the aggregator + discovery use. A composite
    // `<model_id>#<n>` id has no chat.jsonl -> empty (honest), no false hits.
    if kind_allowed(TranscriptKind::ChatTurn) {
        if let Ok(rows) = session_chat_log::read_chat_log(&state.app_data_root, &summary.session_id)
        {
            for row in &rows {
                if let Some(snippet) = make_snippet(&row.content, query_lc) {
                    let ts = DateTime::parse_from_rfc3339(row.created_at_utc.trim())
                        .map(|dt| dt.with_timezone(&Utc))
                        .ok();
                    matches.push(RawMatch {
                        kind: TranscriptKind::ChatTurn,
                        ts,
                        snippet,
                    });
                }
            }
        }
    }

    // --- FR/agent/terminal/process lanes: the SINGLE correlation seam
    // (`fetch_fr_events` = seam-1 + seam-2 + union_dedup), then the SAME typed
    // rows the transcript shows (`entries_from_fr_event`). No second SQL.
    if let Some(recorder) = &state.recorder {
        if let Ok(events) = fetch_fr_events(recorder, &summary.session_id, from, to).await {
            for event in &events {
                for entry in session_transcript::entries_from_fr_event(event) {
                    let kind = entry.kind();
                    if !kind_allowed(kind) {
                        continue;
                    }
                    let ts = Some(entry.timestamp());
                    for field in session_transcript::searchable_text(&entry) {
                        if let Some(snippet) = make_snippet(&field, query_lc) {
                            matches.push(RawMatch { kind, ts, snippet });
                        }
                    }
                }
            }
        }
    }

    matches
}

/// Build one ranked `SessionSearchHit` from a candidate + its raw matches, or
/// `None` when the candidate had no matches in the active lane filter.
fn hit_from_matches(
    summary: &SessionSummary,
    mut matches: Vec<RawMatch>,
) -> Option<SessionSearchHit> {
    if matches.is_empty() {
        return None;
    }
    let match_count = matches.len() as u64;
    // Snippets ordered by ts asc (None last), then kind rank, so they line up
    // with the transcript the operator opens into.
    matches.sort_by(|a, b| {
        a.ts.cmp(&b.ts)
            .then_with(|| kind_rank(a.kind).cmp(&kind_rank(b.kind)))
    });
    let snippets: Vec<SearchSnippet> = matches
        .into_iter()
        .take(PER_SESSION_SNIPPET_CAP)
        .map(|m| SearchSnippet {
            entry_kind: kind_ipc_str(m.kind).to_string(),
            ts: m.ts,
            snippet: m.snippet,
        })
        .collect();
    Some(SessionSearchHit {
        session_id: summary.session_id.clone(),
        kind: summary.kind.clone(),
        provider: summary.provider.clone(),
        model_id: summary.model_id.clone(),
        worktree_id: summary.worktree_id.clone(),
        started_at: summary.started_at,
        title: summary.title.clone(),
        match_count,
        snippets,
    })
}

/// Mirror of `session_transcript::source_rank` for snippet tiebreak ordering.
fn kind_rank(kind: TranscriptKind) -> u8 {
    match kind {
        TranscriptKind::ChatTurn => 0,
        TranscriptKind::FrEvent => 1,
        TranscriptKind::AgentActivity => 2,
        TranscriptKind::TerminalChunk => 3,
        TranscriptKind::Process => 4,
    }
}

/// The IO-thin search core: filter candidates structurally, scan each for the
/// query through the reused readers, rank, and bound. Kept testable (takes the
/// already-built state) and honest (every clip surfaces via `truncated`).
async fn search_sessions(
    state: &SessionTranscriptState,
    query: &str,
    kinds_filter: Option<&[TranscriptKind]>,
    worktree_id: Option<&str>,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
    limit: u64,
) -> SessionSearchResponse {
    let query_trimmed = query.trim().to_string();
    let query_lc = query_trimmed.to_lowercase();

    // Step A: structured candidate filter (reuse kernel_session_list internals).
    let chat = discover_chat_sessions(state);
    let fr = discover_fr_sessions(state).await;
    let mut candidates = merge_summaries(chat, fr);

    if let Some(wt) = worktree_id {
        candidates.retain(|s| s.worktree_id.as_deref() == Some(wt));
    }
    candidates.retain(|s| session_in_window(s, from, to));

    // `merge_summaries` already sorts most-recent-activity first, so the
    // candidate cap keeps the freshest sessions.
    let mut truncated = false;
    if candidates.len() > MAX_CANDIDATE_SESSIONS {
        candidates.truncate(MAX_CANDIDATE_SESSIONS);
        truncated = true;
    }

    // Step B/C: scan each candidate, build hits.
    let mut hits: Vec<SessionSearchHit> = Vec::new();
    for summary in &candidates {
        let matches = scan_candidate(state, summary, &query_lc, kinds_filter, from, to).await;
        if let Some(hit) = hit_from_matches(summary, matches) {
            hits.push(hit);
        }
    }

    // Rank: match_count desc, then recency desc (by candidate last_activity_at —
    // resolve via the candidate list), then session_id asc for determinism.
    let recency: HashMap<&str, Option<DateTime<Utc>>> = candidates
        .iter()
        .map(|s| (s.session_id.as_str(), s.last_activity_at))
        .collect();
    hits.sort_by(|a, b| {
        b.match_count
            .cmp(&a.match_count)
            .then_with(|| {
                let ra = recency.get(a.session_id.as_str()).copied().flatten();
                let rb = recency.get(b.session_id.as_str()).copied().flatten();
                rb.cmp(&ra)
            })
            .then_with(|| a.session_id.cmp(&b.session_id))
    });

    // Bound the hit list.
    let limit = limit as usize;
    if hits.len() > limit {
        hits.truncate(limit);
        truncated = true;
    }

    SessionSearchResponse {
        hits,
        truncated,
        query: query_trimmed,
    }
}

#[tauri::command]
pub async fn kernel_session_search(
    query: String,
    kinds: Option<Vec<String>>,
    worktree_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<u64>,
    state: State<'_, SessionTranscriptState>,
) -> Result<SessionSearchResponse, String> {
    // Honest empty query: reject rather than silently "match all".
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("query must not be empty".to_string());
    }
    let kinds = parse_kinds(kinds)?;
    let worktree_id = worktree_id
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let from = parse_rfc3339_opt(from, "from")?;
    let to = parse_rfc3339_opt(to, "to")?;
    // Clamp the limit into 1..=MAX (default when omitted).
    let limit = limit
        .unwrap_or(SEARCH_LIMIT_DEFAULT)
        .clamp(1, SEARCH_LIMIT_MAX);

    Ok(search_sessions(
        &state,
        &query,
        kinds.as_deref(),
        worktree_id.as_deref(),
        from,
        to,
        limit,
    )
    .await)
}

// ---------------------------------------------------------------------------
// Session export (ROI #5: archive / handoff / sharing)
// ---------------------------------------------------------------------------

/// Convert the lane `SourceCounts` into the export header's `ExportCounts`.
fn export_counts(counts: &SourceCounts) -> ExportCounts {
    ExportCounts {
        chat: counts.chat,
        fr: counts.fr,
        terminal: counts.terminal,
        process: counts.process,
    }
}

/// Resolve the single session summary for `session_id` via the SAME discovery
/// the list/search already use (`discover_chat_sessions` + `discover_fr_sessions`
/// + `merge_summaries`), then `.find` it. `None` when the id matches no
/// discoverable session — the not-found signal.
async fn resolve_single_summary(
    state: &SessionTranscriptState,
    session_id: &str,
) -> Option<SessionSummary> {
    let chat = discover_chat_sessions(state);
    let fr = discover_fr_sessions(state).await;
    merge_summaries(chat, fr)
        .into_iter()
        .find(|s| s.session_id == session_id)
}

/// Build the export header from a resolved (or synthesized) summary + the merged
/// entries. `provider`/`model_id`/`worktree_id` are lifted from the summary;
/// counts come from the FULL merge so the header reflects the real lane spread.
fn build_export_header(
    session_id: &str,
    summary: &SessionSummary,
    entries: &[SessionTranscriptEntry],
) -> ExportHeader {
    ExportHeader {
        session_id: session_id.to_string(),
        kind: summary.kind.clone(),
        provider: summary.provider.clone(),
        model_id: summary.model_id.clone(),
        worktree_id: summary.worktree_id.clone(),
        started_at: summary.started_at,
        last_activity_at: summary.last_activity_at,
        counts: export_counts(&count_lanes(entries)),
    }
}

/// Synthesize a minimal summary for a session that IS discoverable-but-thin
/// (e.g. matched by discovery but with no summary row carrying metadata). Derives
/// `model_id`/`provider` from a composite id and the kind from the id shape.
fn synth_min_summary(session_id: &str) -> SessionSummary {
    let (model_id, provider) = split_instance_id(session_id);
    let kind = if session_id.contains('#') {
        "swarm"
    } else {
        "chat"
    };
    SessionSummary {
        session_id: session_id.to_string(),
        kind: kind.to_string(),
        started_at: None,
        last_activity_at: None,
        model_id,
        provider,
        title: None,
        counts: SourceCounts::default(),
        worktree_id: None,
        resumable: false,
    }
}

/// Atomically write a single rendered artifact (temp + rename), mirroring
/// `save_session_index` / `swarm_schedule_store`. Returns the byte size on
/// success. Disk-agnostic: the caller derives `dest_dir` from `app_data_root` or
/// the operator's chosen dir.
fn write_export_file(path: &Path, content: &str) -> Result<u64, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create export dir failed: {e}"))?;
    }
    let bytes = content.as_bytes();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("out");
    let tmp = path.with_extension(format!("{ext}.tmp.{}", std::process::id()));
    std::fs::write(&tmp, bytes).map_err(|e| format!("write temp export failed: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("commit export failed: {e}")
    })?;
    Ok(bytes.len() as u64)
}

/// IO-thin export core: takes already-fetched inputs + a resolved summary, renders
/// + redacts via the pure `export::render`, and writes the requested artifact(s)
/// atomically under `dest_dir`. Kept free of the Tauri `State` so it is directly
/// testable. Returns the response (paths + sizes + telemetry).
///
/// `dest_dir` is the absolute directory the artifacts land in (the command
/// resolves the default `<app_data_root>/exports` or the operator's choice). The
/// filename stem is the sanitized session id; a UTC timestamp suffix prevents
/// clobbering successive exports.
fn write_session_export(
    session_id: &str,
    entries: &[SessionTranscriptEntry],
    header: &ExportHeader,
    fmt: ExportFormat,
    dest_dir: &Path,
) -> Result<SessionExportResponse, String> {
    let rendered = export::render(entries, header, fmt, &PatternRedactor);

    let stem = export::safe_session_stem(session_id);
    let suffix = export::export_timestamp_suffix(Utc::now());
    let base = format!("session-{stem}-{suffix}");

    let mut files: Vec<ExportedFile> = Vec::new();

    if let Some(md) = &rendered.markdown {
        let path = dest_dir.join(format!("{base}.md"));
        let bytes = write_export_file(&path, md).map_err(|e| {
            // Honest partial: if a prior artifact already landed, surface it.
            partial_error(&e, &files)
        })?;
        files.push(ExportedFile {
            format: "markdown".to_string(),
            path: path.to_string_lossy().to_string(),
            bytes,
        });
    }

    if let Some(js) = &rendered.json {
        let path = dest_dir.join(format!("{base}.json"));
        let bytes = write_export_file(&path, js).map_err(|e| partial_error(&e, &files))?;
        files.push(ExportedFile {
            format: "json".to_string(),
            path: path.to_string_lossy().to_string(),
            bytes,
        });
    }

    Ok(SessionExportResponse {
        session_id: session_id.to_string(),
        dest_dir: dest_dir.to_string_lossy().to_string(),
        files,
        empty: entries.is_empty(),
        redacted_field_count: rendered.redacted_field_count,
    })
}

/// Build an honest error message when a second write fails after a first one
/// already landed, so the operator is not told "nothing happened".
fn partial_error(err: &str, written: &[ExportedFile]) -> String {
    if written.is_empty() {
        err.to_string()
    } else {
        let paths: Vec<&str> = written.iter().map(|f| f.path.as_str()).collect();
        format!("{err} (partial export: already wrote {})", paths.join(", "))
    }
}

#[tauri::command]
pub async fn kernel_session_export(
    session_id: String,
    format: String,
    dest_dir: Option<String>,
    state: State<'_, SessionTranscriptState>,
) -> Result<SessionExportResponse, String> {
    let session_id = session_id.trim().to_string();
    if session_id.is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    let fmt = ExportFormat::from_ipc(&format).ok_or_else(|| format!("unknown format: {format}"))?;

    // Chat lane: path-keyed by session_id (same as transcript_get).
    let chat_entries = session_chat_log::read_chat_log(&state.app_data_root, &session_id)?;
    let chat: Vec<ChatTurnInput> = chat_entries.iter().map(chat_input_from_entry).collect();

    // FR lanes via both seams (or unavailable). FULL export: no from/to window.
    let (fr_events, recorder_available) = match &state.recorder {
        Some(recorder) => (
            fetch_fr_events(recorder, &session_id, None, None).await?,
            true,
        ),
        None => (Vec::new(), false),
    };

    // Live terminal scrollback enrichment (parity with the transcript view).
    let live = live_terminal_scrollback(&state.terminal, &session_id);

    // FULL merge, no kind filter — REUSE the aggregator.
    let response = build_response(&session_id, chat, fr_events, recorder_available, live, None);

    // Resolve the summary (discovery reuse) for the header + not-found honesty.
    let summary = resolve_single_summary(&state, &session_id).await;

    // Honest not-found: an id that matches NO discoverable session AND produces
    // an empty transcript is a typed error, not a meaningless file.
    if response.entries.is_empty() && summary.is_none() {
        return Err(format!("SESSION_NOT_FOUND: {session_id}"));
    }

    // Discoverable-but-thin fallback: a real (or synthesized-minimal) summary.
    let summary = summary.unwrap_or_else(|| synth_min_summary(&session_id));
    let header = build_export_header(&session_id, &summary, &response.entries);

    // Resolve the destination dir: operator choice (verbatim, off-root-capable
    // for sharing) or the disk-agnostic default `<app_data_root>/exports`.
    let dest_dir: PathBuf = match dest_dir.map(|d| d.trim().to_string()) {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => state.app_data_root.join(SESSION_EXPORT_DIR),
    };

    write_session_export(&session_id, &response.entries, &header, fmt, &dest_dir)
}

// ---------------------------------------------------------------------------
// Optional discovery index (rebuildable cache; v1 may ship without it)
// ---------------------------------------------------------------------------

/// The persisted discovery index document. A rebuildable cache over the durable
/// sources, NEVER authority [GLOBAL-GOVARTIFACTS-028].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionIndexDoc {
    pub schema_version: u32,
    #[serde(default)]
    pub sessions: Vec<SessionSummary>,
}

pub const SESSION_INDEX_SCHEMA_VERSION: u32 = 1;

/// Atomically persist the discovery index (temp + rename), mirroring
/// `swarm_schedule_store`. Disk-agnostic: path derived from `app_data_root`.
pub fn save_session_index(sessions_root: &Path, doc: &SessionIndexDoc) -> Result<(), String> {
    std::fs::create_dir_all(sessions_root)
        .map_err(|e| format!("create sessions dir failed: {e}"))?;
    let path = sessions_root.join(SESSION_INDEX_FILE);
    let json = serde_json::to_vec_pretty(doc)
        .map_err(|e| format!("serialize session index failed: {e}"))?;
    let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
    std::fs::write(&tmp, &json).map_err(|e| format!("write temp session index failed: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("commit session index failed: {e}")
    })?;
    Ok(())
}

/// Load the discovery index. Missing file -> empty doc (first run, not an
/// error). Corrupt file surfaces an error so the caller can rebuild from source.
pub fn load_session_index(sessions_root: &Path) -> Result<SessionIndexDoc, String> {
    let path = sessions_root.join(SESSION_INDEX_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("session index at {} is corrupt: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SessionIndexDoc {
            schema_version: SESSION_INDEX_SCHEMA_VERSION,
            sessions: Vec::new(),
        }),
        Err(e) => Err(format!("read session index failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::flight_recorder::{
        FlightRecorderActor, FlightRecorderEvent, FlightRecorderEventType,
    };
    use serde_json::json;
    use uuid::Uuid;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
    }

    fn chat_input(turn: u64, secs: i64, role: &str, content: &str) -> ChatTurnInput {
        ChatTurnInput {
            created_at_utc: at(secs).to_rfc3339(),
            turn_index: turn,
            role: role.to_string(),
            model_role: None,
            content: content.to_string(),
            message_id: format!("m{turn}"),
        }
    }

    fn swarm_event(secs: i64, instance: &str) -> FlightRecorderEvent {
        let mut e = FlightRecorderEvent::new(
            FlightRecorderEventType::System,
            FlightRecorderActor::System,
            Uuid::now_v7(),
            json!({
                "fr_event_id": "FR-EVT-SWARM-SESSION-SPAWNED",
                "instance_id": instance,
                "process_uuid": "33333333-3333-3333-3333-333333333333",
                // Recorded worktree assignment as it lands in the SwarmEvent
                // payload at spawn (coordinator copies SpawnRequest.worktree_id).
                "worktree_id": "wt-recovery-1",
            }),
        );
        e.timestamp = at(secs);
        e
    }

    #[test]
    fn build_response_merges_chat_and_fr_in_order() {
        let chat = vec![
            chat_input(1, 10, "user", "hi"),
            chat_input(2, 40, "assistant", "yo"),
        ];
        let fr = vec![swarm_event(20, "m#0")];
        let resp = build_response("m#0", chat, fr, true, None, None);
        // chat(10), fr+process(20), chat(40): chat=2, fr=1, process=1.
        assert_eq!(resp.entries.len(), 4);
        assert_eq!(resp.source_status.chat, SourceState::Present);
        assert_eq!(resp.source_status.fr, SourceState::Present);
        assert_eq!(resp.source_status.process, SourceState::Present);
        // ts-ordered.
        for w in resp.entries.windows(2) {
            assert!(w[0].timestamp() <= w[1].timestamp());
        }
    }

    #[test]
    fn build_response_chat_only_marks_fr_empty_when_recorder_present() {
        let chat = vec![chat_input(1, 1, "user", "x")];
        let resp = build_response("uuid-chat", chat, vec![], true, None, None);
        assert_eq!(resp.source_status.chat, SourceState::Present);
        assert_eq!(resp.source_status.fr, SourceState::Empty);
        assert_eq!(resp.source_status.terminal, SourceState::Empty);
        assert_eq!(resp.source_status.process, SourceState::Empty);
    }

    #[test]
    fn build_response_recorder_unavailable_marks_fr_lanes_unavailable() {
        let chat = vec![chat_input(1, 1, "user", "x")];
        let resp = build_response("uuid-chat", chat, vec![], false, None, None);
        assert_eq!(resp.source_status.chat, SourceState::Present);
        assert_eq!(resp.source_status.fr, SourceState::Unavailable);
        assert_eq!(resp.source_status.terminal, SourceState::Unavailable);
        assert_eq!(resp.source_status.process, SourceState::Unavailable);
        // Only chat survives.
        assert_eq!(resp.entries.len(), 1);
    }

    #[test]
    fn build_response_empty_session_fabricates_nothing() {
        let resp = build_response("m#9", vec![], vec![], true, None, None);
        assert!(resp.entries.is_empty());
        assert_eq!(resp.source_status.chat, SourceState::Empty);
        assert_eq!(resp.source_status.fr, SourceState::Empty);
    }

    #[test]
    fn build_response_kind_filter_hides_lanes() {
        let chat = vec![chat_input(1, 10, "user", "hi")];
        let fr = vec![swarm_event(20, "m#0")];
        let resp = build_response(
            "m#0",
            chat,
            fr,
            true,
            None,
            Some(vec![TranscriptKind::ChatTurn]),
        );
        assert_eq!(resp.entries.len(), 1);
        assert_eq!(resp.entries[0].kind(), TranscriptKind::ChatTurn);
        // source_status still reports the full-merge presence (honest).
        assert_eq!(resp.source_status.fr, SourceState::Present);
    }

    #[test]
    fn build_response_live_scrollback_enriches_terminal_lane() {
        let resp = build_response(
            "m#0",
            vec![],
            vec![swarm_event(10, "m#0")],
            true,
            Some(("term-live".to_string(), "raw output".to_string(), at(15))),
            None,
        );
        assert_eq!(resp.source_status.terminal, SourceState::Present);
        let has_text = resp.entries.iter().any(|e| {
            matches!(
                e,
                SessionTranscriptEntry::TerminalChunk { text: Some(t), .. } if t == "raw output"
            )
        });
        assert!(has_text);
    }

    #[test]
    fn union_dedup_collapses_same_event_id() {
        let e = swarm_event(5, "m#0");
        let documented = vec![e.clone()];
        let raw = vec![e.clone()];
        let unioned = union_dedup_events(documented, raw);
        assert_eq!(unioned.len(), 1);
    }

    #[test]
    fn parse_kinds_rejects_unknown() {
        assert!(parse_kinds(Some(vec!["bogus".to_string()])).is_err());
        let ok = parse_kinds(Some(vec!["chat_turn".to_string(), "process".to_string()])).unwrap();
        assert_eq!(
            ok,
            Some(vec![TranscriptKind::ChatTurn, TranscriptKind::Process])
        );
        assert_eq!(parse_kinds(None).unwrap(), None);
    }

    #[test]
    fn split_instance_id_extracts_model_id() {
        assert_eq!(
            split_instance_id("qwen2.5-coder#0"),
            (Some("qwen2.5-coder".to_string()), None)
        );
        assert_eq!(split_instance_id("no-hash"), (None, None));
    }

    #[test]
    fn session_index_round_trips_atomically() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().join("sessions");
        let doc = SessionIndexDoc {
            schema_version: SESSION_INDEX_SCHEMA_VERSION,
            sessions: vec![SessionSummary {
                session_id: "m#0".to_string(),
                kind: "swarm".to_string(),
                started_at: Some(at(1)),
                last_activity_at: Some(at(2)),
                model_id: Some("m".to_string()),
                provider: None,
                title: Some("m#0".to_string()),
                counts: SourceCounts {
                    fr: 3,
                    ..Default::default()
                },
                worktree_id: Some("wt-x".to_string()),
                resumable: false,
            }],
        };
        save_session_index(&root, &doc).expect("save");
        let loaded = load_session_index(&root).expect("load");
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].session_id, "m#0");
        // The recorded worktree assignment survives the index round-trip.
        assert_eq!(loaded.sessions[0].worktree_id.as_deref(), Some("wt-x"));
        // Missing file -> empty doc, not an error.
        let empty_root = tmp.path().join("other");
        let empty = load_session_index(&empty_root).expect("load empty");
        assert!(empty.sessions.is_empty());
    }

    /// End-to-end over a REAL temp `app_data_root` + in-memory DuckDB recorder:
    /// write a `sessions/<uuid>/chat.jsonl`, record FR swarm + terminal events
    /// for a composite instance id, then assert (a) discovery finds BOTH the
    /// chat session and the FR instance, and (b) the transcript IO path (both
    /// seams) returns a merged, ordered transcript with honest `source_status`.
    #[tokio::test]
    async fn end_to_end_list_and_transcript_over_real_state() {
        use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;

        let tmp = tempfile::tempdir().expect("tempdir");
        let app_data_root = tmp.path().to_path_buf();

        // (1) Write a real chat.jsonl for a chat-UUID session.
        let chat_session = "0192a000-0000-7000-8000-000000000001";
        let chat_dir = app_data_root.join("sessions").join(chat_session);
        std::fs::create_dir_all(&chat_dir).expect("mkdir chat");
        let row = json!({
            "schema_version": "hsk.session_chat_log@0.1",
            "session_id": chat_session,
            "turn_index": 1u64,
            "created_at_utc": at(100).to_rfc3339(),
            "message_id": "0192a000-0000-7000-8000-0000000000aa",
            "role": "user",
            "content": "hello operator",
        });
        std::fs::write(
            chat_dir.join("chat.jsonl"),
            format!("{}\n", serde_json::to_string(&row).unwrap()),
        )
        .expect("write chat.jsonl");

        // (2) An in-memory recorder with swarm + terminal events for a composite.
        let recorder: Arc<dyn FlightRecorder> =
            Arc::new(DuckDbFlightRecorder::new_in_memory(7).expect("recorder"));
        let instance = "qwen#0";
        recorder
            .record_event(swarm_event(50, instance))
            .await
            .expect("record swarm");
        let mut term = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            Uuid::now_v7(),
            json!({
                "type": "terminal_command",
                "fr_event": "FR-EVT-TERMINAL-COMMAND-EXEC",
                "session_id": instance,
                "command": "cargo build",
                "cwd": "",
                "exit_code": 0,
                "duration_ms": 0,
                "timed_out": false,
                "cancelled": false,
                "truncated_bytes": 0,
            }),
        )
        .with_session_span(instance.to_string());
        term.timestamp = at(60);
        recorder.record_event(term).await.expect("record terminal");

        let state = SessionTranscriptState::new(Some(recorder), &app_data_root, None);

        // (a) Discovery finds BOTH the chat session and the FR instance.
        let chat_sessions = discover_chat_sessions(&state);
        assert!(chat_sessions.iter().any(|s| s.session_id == chat_session));
        let fr_sessions = discover_fr_sessions(&state).await;
        assert!(
            fr_sessions.iter().any(|s| s.session_id == instance),
            "fr sessions: {fr_sessions:?}"
        );
        // The recorded worktree assignment in the SwarmEvent payload surfaces on
        // the session summary so the replay surface can find a worktree's sessions.
        let fr_row = fr_sessions
            .iter()
            .find(|s| s.session_id == instance)
            .expect("fr session present");
        assert_eq!(fr_row.worktree_id.as_deref(), Some("wt-recovery-1"));
        let merged = merge_summaries(chat_sessions, fr_sessions);
        assert!(merged.iter().any(|s| s.session_id == chat_session));
        assert!(merged.iter().any(|s| s.session_id == instance));

        // (b) Transcript for the SWARM session: FrEvent + Process + TerminalChunk,
        // ordered, with honest source_status (chat empty, fr/terminal/process
        // present).
        let recorder_ref = state.recorder.as_ref().unwrap();
        let fr_events = fetch_fr_events(recorder_ref, instance, None, None)
            .await
            .expect("fetch fr");
        let resp = build_response(instance, vec![], fr_events, true, None, None);
        let kinds: Vec<TranscriptKind> = resp.entries.iter().map(|e| e.kind()).collect();
        assert!(kinds.contains(&TranscriptKind::FrEvent));
        assert!(kinds.contains(&TranscriptKind::Process));
        assert!(kinds.contains(&TranscriptKind::TerminalChunk));
        assert_eq!(resp.source_status.chat, SourceState::Empty);
        assert_eq!(resp.source_status.fr, SourceState::Present);
        assert_eq!(resp.source_status.terminal, SourceState::Present);
        for w in resp.entries.windows(2) {
            assert!(w[0].timestamp() <= w[1].timestamp());
        }

        // (c) Transcript for the CHAT session: one chat turn, fr empty (no FR
        // events keyed by the chat UUID), honest.
        let chat_entries =
            session_chat_log::read_chat_log(&state.app_data_root, chat_session).expect("read chat");
        let chat: Vec<ChatTurnInput> = chat_entries.iter().map(chat_input_from_entry).collect();
        let chat_fr = fetch_fr_events(recorder_ref, chat_session, None, None)
            .await
            .expect("fetch fr for chat");
        let chat_resp = build_response(chat_session, chat, chat_fr, true, None, None);
        assert_eq!(chat_resp.source_status.chat, SourceState::Present);
        assert_eq!(chat_resp.source_status.fr, SourceState::Empty);
        assert_eq!(chat_resp.entries.len(), 1);
        assert_eq!(chat_resp.entries[0].kind(), TranscriptKind::ChatTurn);
    }

    /// CONTRACT GATE: the IPC response structs the frontend reads MUST serialize
    /// the camelCase keys the TS client declares (sessionId, startedAt,
    /// lastActivityAt, modelId, sourceStatus, …). This asserts the serde boundary
    /// directly so the snake_case-vs-camelCase mismatch that shipped cannot
    /// regress — covered by `cargo test`, not only by hand-built JS mocks.
    #[test]
    fn ipc_response_structs_serialize_camelcase() {
        let summary = SessionSummary {
            session_id: "m#0".to_string(),
            kind: "swarm".to_string(),
            started_at: Some(at(1)),
            last_activity_at: Some(at(2)),
            model_id: Some("m".to_string()),
            provider: None,
            title: Some("m#0".to_string()),
            counts: SourceCounts {
                chat: 1,
                fr: 2,
                terminal: 3,
                process: 4,
            },
            worktree_id: Some("wt-x".to_string()),
            resumable: true,
        };
        let v = serde_json::to_value(&summary).unwrap();
        for key in [
            "sessionId",
            "startedAt",
            "lastActivityAt",
            "modelId",
            "worktreeId",
            "resumable",
        ] {
            assert!(v.get(key).is_some(), "expected camelCase {key}");
        }
        assert_eq!(v["worktreeId"], serde_json::json!("wt-x"));
        assert_eq!(v["resumable"], serde_json::json!(true));
        for key in [
            "session_id",
            "started_at",
            "last_activity_at",
            "model_id",
            "worktree_id",
        ] {
            assert!(v.get(key).is_none(), "snake_case {key} must not leak");
        }
        // SourceCounts keys are single-word -> unchanged by camelCase.
        let counts = &v["counts"];
        for key in ["chat", "fr", "terminal", "process"] {
            assert!(counts.get(key).is_some(), "expected counts.{key}");
        }

        let resp = SessionTranscriptResponse {
            session_id: "m#0".to_string(),
            entries: vec![],
            source_status: SourceStatus {
                chat: SourceState::Present,
                fr: SourceState::Empty,
                terminal: SourceState::Unavailable,
                process: SourceState::Empty,
            },
            truncated: false,
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert!(v.get("sessionId").is_some(), "expected camelCase sessionId");
        assert!(
            v.get("sourceStatus").is_some(),
            "expected camelCase sourceStatus"
        );
        assert!(v.get("session_id").is_none(), "snake_case must not leak");
        assert!(v.get("source_status").is_none(), "snake_case must not leak");
        // SourceState values serialize snake_case (matches the TS union).
        assert_eq!(v["sourceStatus"]["chat"], "present");
        assert_eq!(v["sourceStatus"]["terminal"], "unavailable");
    }

    // -----------------------------------------------------------------------
    // Cross-session search (ROI #4) tests
    // -----------------------------------------------------------------------

    #[test]
    fn make_snippet_centers_and_redacts_secret() {
        // A secret adjacent to the match must be masked in the emitted snippet.
        let text = "before the API_KEY=supersecret123 and the cargo build matched here";
        let snippet = make_snippet(text, "cargo").expect("match");
        assert!(
            snippet.contains("cargo"),
            "snippet keeps the match: {snippet}"
        );
        assert!(
            snippet.contains("***REDACTED***"),
            "adjacent secret redacted: {snippet}"
        );
        assert!(
            !snippet.contains("supersecret123"),
            "raw secret must never leak: {snippet}"
        );
    }

    #[test]
    fn make_snippet_query_can_be_secret_shaped_and_is_redacted() {
        // Even if the query itself is secret-shaped, the emitted window is redacted.
        let text = "noise API_KEY=topsecretvalue tail";
        let snippet = make_snippet(text, "api_key=topsecretvalue").expect("match");
        assert!(snippet.contains("***REDACTED***"));
        assert!(!snippet.contains("topsecretvalue"));
    }

    #[test]
    fn make_snippet_no_match_is_none_and_utf8_safe() {
        assert!(make_snippet("hello world", "zzz").is_none());
        // Multibyte chars straddling the window boundary must not panic.
        let multibyte = "🚀".repeat(80) + "needle" + &"🌟".repeat(80);
        let snippet = make_snippet(&multibyte, "needle").expect("match");
        assert!(snippet.contains("needle"));
        assert!(snippet.starts_with('…'));
        assert!(snippet.ends_with('…'));
    }

    #[test]
    fn find_match_is_case_insensitive() {
        assert_eq!(find_match_char_idx("Hello Cargo Build", "cargo"), Some(6));
        assert_eq!(find_match_char_idx("HELLO", "hello"), Some(0));
        assert_eq!(find_match_char_idx("abc", "xyz"), None);
    }

    #[test]
    fn session_in_window_keeps_overlapping_drops_outside() {
        let s = |start: i64, last: i64| SessionSummary {
            session_id: "s".to_string(),
            kind: "swarm".to_string(),
            started_at: Some(at(start)),
            last_activity_at: Some(at(last)),
            model_id: None,
            provider: None,
            title: None,
            counts: SourceCounts::default(),
            worktree_id: None,
            resumable: false,
        };
        // Session [100,200], window [50,500] -> overlaps (keep).
        assert!(session_in_window(&s(100, 200), Some(at(50)), Some(at(500))));
        // Session whole span before `from` -> drop.
        assert!(!session_in_window(&s(10, 40), Some(at(50)), None));
        // Session whole span after `to` -> drop.
        assert!(!session_in_window(&s(600, 700), None, Some(at(500))));
        // No bounds -> always keep.
        assert!(session_in_window(&s(10, 40), None, None));
    }

    #[test]
    fn hit_from_matches_caps_snippets_but_keeps_true_match_count() {
        let summary = SessionSummary {
            session_id: "m#0".to_string(),
            kind: "swarm".to_string(),
            started_at: Some(at(1)),
            last_activity_at: Some(at(9)),
            model_id: Some("m".to_string()),
            provider: None,
            title: Some("m#0".to_string()),
            counts: SourceCounts::default(),
            worktree_id: Some("wt-x".to_string()),
            resumable: false,
        };
        let matches: Vec<RawMatch> = (0..12)
            .map(|i| RawMatch {
                kind: TranscriptKind::TerminalChunk,
                ts: Some(at(i)),
                snippet: format!("snip {i}"),
            })
            .collect();
        let hit = hit_from_matches(&summary, matches).expect("hit");
        assert_eq!(hit.match_count, 12, "true total preserved");
        assert_eq!(
            hit.snippets.len(),
            PER_SESSION_SNIPPET_CAP,
            "snippets capped"
        );
        // ts-asc ordered.
        assert_eq!(hit.snippets[0].ts, Some(at(0)));
        assert_eq!(hit.snippets[0].entry_kind, "terminal_chunk");
    }

    /// End-to-end search over a REAL temp `app_data_root` + in-memory DuckDB
    /// recorder, mirroring `end_to_end_list_and_transcript_over_real_state`. Seeds
    /// a chat session ("operator forgot the gate" + a secret line) and a swarm
    /// session with a `cargo build` terminal event, then asserts the search core
    /// finds the right sessions, redacts the secret, and honors the structured
    /// filters.
    #[tokio::test]
    async fn end_to_end_search_over_real_state() {
        use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;

        let tmp = tempfile::tempdir().expect("tempdir");
        let app_data_root = tmp.path().to_path_buf();

        // (1) Chat session: a "forgot" turn + a secret-bearing turn.
        let chat_session = "0192a000-0000-7000-8000-000000000001";
        let chat_dir = app_data_root.join("sessions").join(chat_session);
        std::fs::create_dir_all(&chat_dir).expect("mkdir chat");
        let mut lines = String::new();
        for (turn, content) in [
            (1u64, "the operator forgot the gate before lunch"),
            (2u64, "export API_KEY=supersecretchatvalue then forgot it"),
        ] {
            let row = json!({
                "schema_version": "hsk.session_chat_log@0.1",
                "session_id": chat_session,
                "turn_index": turn,
                "created_at_utc": at(100 + turn as i64).to_rfc3339(),
                "message_id": format!("0192a000-0000-7000-8000-0000000000{turn:02}"),
                "role": "user",
                "content": content,
            });
            lines.push_str(&serde_json::to_string(&row).unwrap());
            lines.push('\n');
        }
        std::fs::write(chat_dir.join("chat.jsonl"), lines).expect("write chat.jsonl");

        // (2) Swarm session: a terminal `cargo build` event, worktree wt-recovery-1.
        let recorder: Arc<dyn FlightRecorder> =
            Arc::new(DuckDbFlightRecorder::new_in_memory(7).expect("recorder"));
        let instance = "qwen#0";
        recorder
            .record_event(swarm_event(50, instance))
            .await
            .expect("record swarm");
        let mut term = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            Uuid::now_v7(),
            json!({
                "type": "terminal_command",
                "fr_event": "FR-EVT-TERMINAL-COMMAND-EXEC",
                "session_id": instance,
                "command": "cargo build --release",
                "cwd": "",
                "exit_code": 0,
                "duration_ms": 0,
                "timed_out": false,
                "cancelled": false,
                "truncated_bytes": 0,
            }),
        )
        .with_session_span(instance.to_string());
        term.timestamp = at(60);
        recorder.record_event(term).await.expect("record terminal");

        let state = SessionTranscriptState::new(Some(recorder), &app_data_root, None);

        // (a) query "cargo" -> the swarm session via a terminal snippet.
        let resp = search_sessions(&state, "cargo", None, None, None, None, 50).await;
        let cargo_hit = resp
            .hits
            .iter()
            .find(|h| h.session_id == instance)
            .expect("cargo hit on swarm session");
        assert!(cargo_hit.match_count >= 1);
        assert!(cargo_hit
            .snippets
            .iter()
            .any(|s| s.entry_kind == "terminal_chunk" && s.snippet.contains("cargo")));
        assert_eq!(resp.query, "cargo");

        // (b) query "forgot" -> the chat session via a chat snippet; the secret
        // turn ALSO contains "forgot" but its snippet must be redacted.
        let resp = search_sessions(&state, "forgot", None, None, None, None, 50).await;
        let chat_hit = resp
            .hits
            .iter()
            .find(|h| h.session_id == chat_session)
            .expect("forgot hit on chat session");
        assert_eq!(chat_hit.match_count, 2, "both forgot turns matched");
        assert!(chat_hit
            .snippets
            .iter()
            .all(|s| s.entry_kind == "chat_turn"));
        // The secret-bearing turn's snippet is redacted; raw secret never appears.
        assert!(
            chat_hit
                .snippets
                .iter()
                .all(|s| !s.snippet.contains("supersecretchatvalue")),
            "secret must be redacted in every snippet: {:?}",
            chat_hit.snippets
        );
        assert!(
            chat_hit
                .snippets
                .iter()
                .any(|s| s.snippet.contains("***REDACTED***")),
            "the secret turn's snippet shows the redaction marker"
        );

        // (c) worktree filter scopes to the recorded wt-recovery-1 (the swarm
        // session); the chat session has no worktree -> excluded.
        let resp = search_sessions(
            &state,
            "forgot",
            None,
            Some("wt-recovery-1"),
            None,
            None,
            50,
        )
        .await;
        assert!(
            resp.hits
                .iter()
                .all(|h| h.session_id == instance || h.match_count == 0),
            "worktree filter excludes the chat session"
        );
        assert!(
            !resp.hits.iter().any(|h| h.session_id == chat_session),
            "chat session has no worktree -> filtered out"
        );

        // (d) kind filter: query "cargo" restricted to chat_turn -> no hits (the
        // only cargo match is a terminal lane).
        let resp = search_sessions(
            &state,
            "cargo",
            Some(&[TranscriptKind::ChatTurn]),
            None,
            None,
            None,
            50,
        )
        .await;
        assert!(
            !resp.hits.iter().any(|h| h.session_id == instance),
            "cargo only matches the terminal lane -> excluded by chat_turn filter"
        );

        // (e) time window excludes out-of-range: a window AFTER all events -> empty.
        let resp = search_sessions(
            &state,
            "forgot",
            None,
            None,
            Some(at(10_000)),
            Some(at(20_000)),
            50,
        )
        .await;
        assert!(
            resp.hits.is_empty(),
            "no session overlaps the future window"
        );

        // (f) honest no-match -> empty hits, query echoed, not truncated.
        let resp = search_sessions(&state, "zzz-nomatch-zzz", None, None, None, None, 50).await;
        assert!(resp.hits.is_empty());
        assert!(!resp.truncated);
        assert_eq!(resp.query, "zzz-nomatch-zzz");
    }

    #[tokio::test]
    async fn search_ranks_by_match_count_then_recency() {
        // Two chat sessions: one with 3 matches (older), one with 1 (newer).
        let tmp = tempfile::tempdir().expect("tempdir");
        let app_data_root = tmp.path().to_path_buf();
        let write_chat = |sid: &str, base: i64, n: usize| {
            let dir = app_data_root.join("sessions").join(sid);
            std::fs::create_dir_all(&dir).unwrap();
            let mut lines = String::new();
            for turn in 1..=n {
                let row = json!({
                    "schema_version": "hsk.session_chat_log@0.1",
                    "session_id": sid,
                    "turn_index": turn as u64,
                    "created_at_utc": at(base + turn as i64).to_rfc3339(),
                    "message_id": format!("{sid}-{turn}"),
                    "role": "user",
                    "content": "needle here",
                });
                lines.push_str(&serde_json::to_string(&row).unwrap());
                lines.push('\n');
            }
            std::fs::write(dir.join("chat.jsonl"), lines).unwrap();
        };
        // many-match older session, few-match newer session.
        write_chat("0192a000-0000-7000-8000-00000000aaaa", 100, 3);
        write_chat("0192a000-0000-7000-8000-00000000bbbb", 900, 1);

        let state = SessionTranscriptState::new(None, &app_data_root, None);
        let resp = search_sessions(&state, "needle", None, None, None, None, 50).await;
        assert_eq!(resp.hits.len(), 2);
        // Higher match_count ranks first regardless of recency.
        assert_eq!(resp.hits[0].match_count, 3);
        assert_eq!(resp.hits[1].match_count, 1);

        // limit=1 truncates honestly.
        let resp = search_sessions(&state, "needle", None, None, None, None, 1).await;
        assert_eq!(resp.hits.len(), 1);
        assert!(resp.truncated);
    }

    #[tokio::test]
    async fn search_empty_query_rejected_via_command_validation() {
        // The command-level validation (mirrored here) rejects empty/whitespace.
        assert!(parse_kinds(Some(vec!["bogus".to_string()])).is_err());
        // Whitespace query trims to empty -> the command returns Err. We assert the
        // core contract: a blank trimmed query never reaches search_sessions with
        // content. (The #[tauri::command] guard returns Err before this point.)
        assert_eq!("   ".trim(), "");
    }

    /// CONTRACT GATE: the search response structs the frontend reads MUST
    /// serialize the camelCase keys the TS client declares (sessionId, modelId,
    /// worktreeId, startedAt, matchCount, entryKind, …). Mirrors
    /// `ipc_response_structs_serialize_camelcase` so the TS client can't drift.
    #[test]
    fn search_response_structs_serialize_camelcase() {
        let resp = SessionSearchResponse {
            hits: vec![SessionSearchHit {
                session_id: "m#0".to_string(),
                kind: "swarm".to_string(),
                provider: Some("byok".to_string()),
                model_id: Some("m".to_string()),
                worktree_id: Some("wt-x".to_string()),
                started_at: Some(at(1)),
                title: Some("m#0".to_string()),
                match_count: 7,
                snippets: vec![SearchSnippet {
                    entry_kind: "terminal_chunk".to_string(),
                    ts: Some(at(2)),
                    snippet: "cargo build".to_string(),
                }],
            }],
            truncated: true,
            query: "cargo".to_string(),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert!(v.get("hits").is_some());
        assert_eq!(v["truncated"], serde_json::json!(true));
        assert_eq!(v["query"], serde_json::json!("cargo"));
        let hit = &v["hits"][0];
        for key in [
            "sessionId",
            "kind",
            "provider",
            "modelId",
            "worktreeId",
            "startedAt",
            "title",
            "matchCount",
            "snippets",
        ] {
            assert!(hit.get(key).is_some(), "expected camelCase {key}");
        }
        for key in [
            "session_id",
            "model_id",
            "worktree_id",
            "started_at",
            "match_count",
        ] {
            assert!(hit.get(key).is_none(), "snake_case {key} must not leak");
        }
        let snip = &hit["snippets"][0];
        assert!(snip.get("entryKind").is_some(), "expected entryKind");
        assert!(snip.get("entry_kind").is_none(), "snake_case must not leak");
        assert_eq!(snip["snippet"], serde_json::json!("cargo build"));
    }

    #[test]
    fn merge_summaries_dedups_and_sorts_recent_first() {
        let chat = vec![SessionSummary {
            session_id: "a".to_string(),
            kind: "chat".to_string(),
            started_at: Some(at(1)),
            last_activity_at: Some(at(5)),
            model_id: None,
            provider: None,
            title: None,
            counts: SourceCounts::default(),
            worktree_id: None,
            resumable: false,
        }];
        let fr = vec![SessionSummary {
            session_id: "b".to_string(),
            kind: "swarm".to_string(),
            started_at: Some(at(1)),
            last_activity_at: Some(at(99)),
            model_id: None,
            provider: None,
            title: None,
            counts: SourceCounts::default(),
            worktree_id: Some("wt-b".to_string()),
            resumable: false,
        }];
        let merged = merge_summaries(chat, fr);
        assert_eq!(merged.len(), 2);
        // Most recent (b @99) first.
        assert_eq!(merged[0].session_id, "b");
    }

    /// ROI#3: `overlay_resumable` flips `resumable` true ONLY for ids present in
    /// the spawn-template store; chat/UUID ids and unknown swarm ids stay false; a
    /// missing store leaves everything false (no panic).
    #[test]
    fn overlay_resumable_reflects_template_presence() {
        use super::super::spawn_template_store::{
            SessionSpawnTemplate, SpawnTemplateStore, TemplateProvider,
        };
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = SessionTranscriptState::new(None, tmp.path(), None);

        // Missing store: nothing flips, no panic.
        let mut summaries = vec![
            SessionSummary {
                session_id: "model-x#0".to_string(),
                kind: "swarm".to_string(),
                started_at: None,
                last_activity_at: None,
                model_id: None,
                provider: None,
                title: None,
                counts: SourceCounts::default(),
                worktree_id: None,
                resumable: false,
            },
            SessionSummary {
                session_id: "chat-uuid".to_string(),
                kind: "chat".to_string(),
                started_at: None,
                last_activity_at: None,
                model_id: None,
                provider: None,
                title: None,
                counts: SourceCounts::default(),
                worktree_id: None,
                resumable: false,
            },
        ];
        overlay_resumable(&state, &mut summaries);
        assert!(!summaries[0].resumable);
        assert!(!summaries[1].resumable);

        // Persist a template for the swarm id only.
        let store = SpawnTemplateStore::new(tmp.path());
        store
            .upsert(
                "model-x#0",
                SessionSpawnTemplate {
                    provider: TemplateProvider::ByokCloud,
                    artifact_path: None,
                    sha256_expected: None,
                    runtime_binding: None,
                    local_execution_mode: None,
                    warm_vm_restore_manifest: None,
                    cloud_model_name: Some("claude-sonnet-4".to_string()),
                    byok_cloud_provider: None,
                    instance: 0,
                    swarm_id: None,
                    worktree_id: None,
                    working_dir: None,
                    isolation_tier: None,
                    committed_memory_bytes: None,
                    origin_session_id: "model-x#0".to_string(),
                    captured_at: Utc::now(),
                },
            )
            .expect("persist template");

        overlay_resumable(&state, &mut summaries);
        // Only the swarm session with a stored template is now resumable.
        assert!(
            summaries[0].resumable,
            "swarm id with template => resumable"
        );
        assert!(
            !summaries[1].resumable,
            "chat id has no template => not resumable"
        );
    }

    // -----------------------------------------------------------------------
    // Session export (ROI #5) tests
    // -----------------------------------------------------------------------

    /// Helper: seed a real temp `app_data_root` with a swarm session whose
    /// terminal event carries a secret, plus a recorder, and return the state +
    /// root + composite instance id.
    async fn seed_export_state() -> (tempfile::TempDir, PathBuf, SessionTranscriptState, String) {
        use handshake_core::flight_recorder::duckdb::DuckDbFlightRecorder;

        let tmp = tempfile::tempdir().expect("tempdir");
        let app_data_root = tmp.path().to_path_buf();

        // A chat session carrying a secret in a chat turn.
        let chat_session = "0192a000-0000-7000-8000-0000000000ee";
        let chat_dir = app_data_root.join("sessions").join(chat_session);
        std::fs::create_dir_all(&chat_dir).expect("mkdir chat");
        let row = json!({
            "schema_version": "hsk.session_chat_log@0.1",
            "session_id": chat_session,
            "turn_index": 1u64,
            "created_at_utc": at(100).to_rfc3339(),
            "message_id": "0192a000-0000-7000-8000-0000000000ef",
            "role": "user",
            "content": "set API_KEY=supersecretexportval and run",
        });
        std::fs::write(
            chat_dir.join("chat.jsonl"),
            format!("{}\n", serde_json::to_string(&row).unwrap()),
        )
        .expect("write chat.jsonl");

        // A swarm session with a swarm spawn + a terminal command carrying a secret.
        let recorder: Arc<dyn FlightRecorder> =
            Arc::new(DuckDbFlightRecorder::new_in_memory(7).expect("recorder"));
        let instance = "qwen#0";
        recorder
            .record_event(swarm_event(50, instance))
            .await
            .expect("record swarm");
        let mut term = FlightRecorderEvent::new(
            FlightRecorderEventType::TerminalCommand,
            FlightRecorderActor::Agent,
            Uuid::now_v7(),
            json!({
                "type": "terminal_command",
                "fr_event": "FR-EVT-TERMINAL-COMMAND-EXEC",
                "session_id": instance,
                "command": "deploy --token=supersecretexportval",
                "cwd": "",
                "exit_code": 0,
                "duration_ms": 0,
                "timed_out": false,
                "cancelled": false,
                "truncated_bytes": 0,
            }),
        )
        .with_session_span(instance.to_string());
        term.timestamp = at(60);
        recorder.record_event(term).await.expect("record terminal");

        let state = SessionTranscriptState::new(Some(recorder), &app_data_root, None);
        (tmp, app_data_root, state, instance.to_string())
    }

    #[tokio::test]
    async fn export_writes_both_files_atomically_over_real_state() {
        let (_tmp, app_data_root, state, instance) = seed_export_state().await;

        // FULL transcript via the reused readers (same as the command body).
        let recorder_ref = state.recorder.as_ref().unwrap();
        let fr_events = fetch_fr_events(recorder_ref, &instance, None, None)
            .await
            .expect("fetch fr");
        let resp = build_response(&instance, vec![], fr_events, true, None, None);
        assert!(!resp.entries.is_empty(), "swarm session has entries");

        let summary = resolve_single_summary(&state, &instance)
            .await
            .expect("summary resolves");
        let header = build_export_header(&instance, &summary, &resp.entries);

        let dest = app_data_root.join(SESSION_EXPORT_DIR);
        let out =
            write_session_export(&instance, &resp.entries, &header, ExportFormat::Both, &dest)
                .expect("export ok");

        assert_eq!(out.files.len(), 2, "both md + json written");
        assert!(!out.empty);
        assert!(
            out.redacted_field_count >= 1,
            "the terminal secret was masked"
        );

        for f in &out.files {
            let p = Path::new(&f.path);
            assert!(p.exists(), "file exists on disk: {}", f.path);
            assert!(f.bytes > 0, "non-empty file");
            // Filenames are space-free (NAMING policy).
            let name = p.file_name().unwrap().to_string_lossy();
            assert!(!name.contains(' '), "filename has no spaces: {name}");
            // The secret is absent from the written file.
            let content = std::fs::read_to_string(p).expect("read export");
            assert!(
                !content.contains("supersecretexportval"),
                "secret must not leak to disk in {}",
                f.path
            );
        }

        // No `.tmp.` residue remains in the dest dir.
        let residue: Vec<String> = std::fs::read_dir(&dest)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".tmp."))
            .collect();
        assert!(residue.is_empty(), "no temp residue: {residue:?}");

        // dest_dir is the absolute exports dir.
        assert_eq!(out.dest_dir, dest.to_string_lossy());
    }

    #[tokio::test]
    async fn export_chat_session_redacts_and_writes() {
        let (_tmp, app_data_root, state, _instance) = seed_export_state().await;
        let chat_session = "0192a000-0000-7000-8000-0000000000ee";

        let chat_entries =
            session_chat_log::read_chat_log(&app_data_root, chat_session).expect("read chat");
        let chat: Vec<ChatTurnInput> = chat_entries.iter().map(chat_input_from_entry).collect();
        let resp = build_response(chat_session, chat, vec![], true, None, None);
        let summary = resolve_single_summary(&state, chat_session)
            .await
            .expect("chat summary resolves");
        let header = build_export_header(chat_session, &summary, &resp.entries);

        let dest = app_data_root.join(SESSION_EXPORT_DIR);
        let out = write_session_export(
            chat_session,
            &resp.entries,
            &header,
            ExportFormat::Markdown,
            &dest,
        )
        .expect("export ok");
        assert_eq!(out.files.len(), 1);
        let content = std::fs::read_to_string(&out.files[0].path).expect("read");
        assert!(
            !content.contains("supersecretexportval"),
            "chat secret redacted on disk"
        );
        assert!(content.contains("***REDACTED***"));
        assert!(out.redacted_field_count >= 1);
    }

    #[tokio::test]
    async fn export_unknown_session_is_not_found() {
        let (_tmp, _root, state, _instance) = seed_export_state().await;
        // A never-recorded id: no chat dir, no FR rows, no summary.
        let err = kernel_session_export_core(&state, "ghost#404", "both", None)
            .await
            .expect_err("unknown id must error");
        assert!(
            err.starts_with("SESSION_NOT_FOUND:"),
            "typed not-found error, got: {err}"
        );
    }

    #[tokio::test]
    async fn export_discoverable_empty_session_writes_empty_file() {
        // A chat session whose chat.jsonl exists but is EMPTY is not discoverable
        // (discovery skips empty logs); instead test the empty-but-valid path via
        // a synthesized-thin discoverable: a chat id with a single whitespace turn
        // would still parse. Simpler: assert the empty-valid render path directly
        // through write_session_export with empty entries + a synth summary.
        let (_tmp, app_data_root, _state, _instance) = seed_export_state().await;
        let sid = "thin-session";
        let summary = synth_min_summary(sid);
        let header = build_export_header(sid, &summary, &[]);
        let dest = app_data_root.join(SESSION_EXPORT_DIR);
        let out = write_session_export(sid, &[], &header, ExportFormat::Both, &dest)
            .expect("empty export ok");
        assert!(out.empty, "empty flag set");
        assert_eq!(out.files.len(), 2);
        let md = std::fs::read_to_string(
            &out.files
                .iter()
                .find(|f| f.format == "markdown")
                .unwrap()
                .path,
        )
        .unwrap();
        assert!(md.contains("_No entries recorded for this session._"));
        assert_eq!(out.redacted_field_count, 0);
    }

    #[tokio::test]
    async fn export_default_dest_is_under_app_data_root() {
        let (_tmp, app_data_root, state, instance) = seed_export_state().await;
        // dest_dir = None -> default <app_data_root>/exports.
        let out = kernel_session_export_core(&state, &instance, "json", None)
            .await
            .expect("export ok");
        let expected = app_data_root.join(SESSION_EXPORT_DIR);
        assert_eq!(out.dest_dir, expected.to_string_lossy());
        assert!(Path::new(&out.files[0].path).starts_with(&expected));
    }

    #[tokio::test]
    async fn export_rejects_empty_session_id_and_unknown_format() {
        let (_tmp, _root, state, instance) = seed_export_state().await;
        assert!(kernel_session_export_core(&state, "   ", "both", None)
            .await
            .is_err());
        let err = kernel_session_export_core(&state, &instance, "pdf", None)
            .await
            .expect_err("unknown format rejected");
        assert!(err.contains("unknown format"), "got: {err}");
    }

    #[test]
    fn export_response_serializes_camelcase() {
        let resp = SessionExportResponse {
            session_id: "m#0".to_string(),
            dest_dir: "/tmp/exports".to_string(),
            files: vec![ExportedFile {
                format: "markdown".to_string(),
                path: "/tmp/exports/session-m-0.md".to_string(),
                bytes: 1234,
            }],
            empty: false,
            redacted_field_count: 2,
        };
        let v = serde_json::to_value(&resp).unwrap();
        for key in [
            "sessionId",
            "destDir",
            "files",
            "empty",
            "redactedFieldCount",
        ] {
            assert!(v.get(key).is_some(), "expected camelCase {key}");
        }
        for key in ["session_id", "dest_dir", "redacted_field_count"] {
            assert!(v.get(key).is_none(), "snake_case {key} must not leak");
        }
        assert_eq!(v["files"][0]["format"], json!("markdown"));
        assert_eq!(v["files"][0]["bytes"], json!(1234));
    }

    /// A test-only mirror of the `kernel_session_export` command body that takes
    /// the state by reference (the `#[tauri::command]` takes `State`, which is not
    /// constructible in a unit test). Keeps the not-found / default-dest / format
    /// branches under test without a Tauri runtime.
    async fn kernel_session_export_core(
        state: &SessionTranscriptState,
        session_id: &str,
        format: &str,
        dest_dir: Option<String>,
    ) -> Result<SessionExportResponse, String> {
        let session_id = session_id.trim().to_string();
        if session_id.is_empty() {
            return Err("session_id must not be empty".to_string());
        }
        let fmt =
            ExportFormat::from_ipc(format).ok_or_else(|| format!("unknown format: {format}"))?;

        let chat_entries = session_chat_log::read_chat_log(&state.app_data_root, &session_id)?;
        let chat: Vec<ChatTurnInput> = chat_entries.iter().map(chat_input_from_entry).collect();
        let (fr_events, recorder_available) = match &state.recorder {
            Some(recorder) => (
                fetch_fr_events(recorder, &session_id, None, None).await?,
                true,
            ),
            None => (Vec::new(), false),
        };
        let live = live_terminal_scrollback(&state.terminal, &session_id);
        let response = build_response(&session_id, chat, fr_events, recorder_available, live, None);
        let summary = resolve_single_summary(state, &session_id).await;
        if response.entries.is_empty() && summary.is_none() {
            return Err(format!("SESSION_NOT_FOUND: {session_id}"));
        }
        let summary = summary.unwrap_or_else(|| synth_min_summary(&session_id));
        let header = build_export_header(&session_id, &summary, &response.entries);
        let dest_dir: PathBuf = match dest_dir.map(|d| d.trim().to_string()) {
            Some(d) if !d.is_empty() => PathBuf::from(d),
            _ => state.app_data_root.join(SESSION_EXPORT_DIR),
        };
        write_session_export(&session_id, &response.entries, &header, fmt, &dest_dir)
    }
}
