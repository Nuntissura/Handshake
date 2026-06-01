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
    self, ChatTurnInput, SessionTranscriptEntry, TranscriptKind,
};
use handshake_core::terminal::TerminalRuntime;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::session_chat_log::{self, SessionChatRole};

pub const KERNEL_SESSION_LIST_IPC_CHANNEL: &str = "kernel_session_list";
pub const KERNEL_SESSION_TRANSCRIPT_GET_IPC_CHANNEL: &str = "kernel_session_transcript_get";

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
            merged =
                session_transcript::append_terminal_scrollback(merged, term_id, text, ts);
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
fn merge_summaries(
    chat: Vec<SessionSummary>,
    fr: Vec<SessionSummary>,
) -> Vec<SessionSummary> {
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
        let chat = vec![chat_input(1, 10, "user", "hi"), chat_input(2, 40, "assistant", "yo")];
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
        assert_eq!(ok, Some(vec![TranscriptKind::ChatTurn, TranscriptKind::Process]));
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
        for key in ["sessionId", "startedAt", "lastActivityAt", "modelId", "worktreeId", "resumable"] {
            assert!(v.get(key).is_some(), "expected camelCase {key}");
        }
        assert_eq!(v["worktreeId"], serde_json::json!("wt-x"));
        assert_eq!(v["resumable"], serde_json::json!(true));
        for key in ["session_id", "started_at", "last_activity_at", "model_id", "worktree_id"] {
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
        assert!(v.get("sourceStatus").is_some(), "expected camelCase sourceStatus");
        assert!(v.get("session_id").is_none(), "snake_case must not leak");
        assert!(v.get("source_status").is_none(), "snake_case must not leak");
        // SourceState values serialize snake_case (matches the TS union).
        assert_eq!(v["sourceStatus"]["chat"], "present");
        assert_eq!(v["sourceStatus"]["terminal"], "unavailable");
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
                    cloud_model_name: Some("claude-sonnet-4".to_string()),
                    instance: 0,
                    worktree_id: None,
                    working_dir: None,
                    isolation_tier: None,
                    origin_session_id: "model-x#0".to_string(),
                    captured_at: Utc::now(),
                },
            )
            .expect("persist template");

        overlay_resumable(&state, &mut summaries);
        // Only the swarm session with a stored template is now resumable.
        assert!(summaries[0].resumable, "swarm id with template => resumable");
        assert!(!summaries[1].resumable, "chat id has no template => not resumable");
    }
}
