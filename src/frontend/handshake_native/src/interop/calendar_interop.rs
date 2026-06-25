//! Editors <-> Calendar (Pillar 2) interop edge (WP-KERNEL-012 MT-067, cluster E10).
//!
//! ## What this is — the "melt-together" edge between the daily-note system and the Calendar
//!
//! This module wires three correlations the Notes pillar needs against the time-structured workspace:
//!
//! 1. **date -> daily note (idempotent open-or-create):** [`CalendarInteropService::open_or_create_daily_note`]
//!    DELEGATES to the MT-019 daily-note service ([`crate::rich_editor::daily_notes::journal_store::JournalBackend::open_daily_journal`],
//!    the verified `PUT /workspaces/{ws}/loom/journals/{date}` get-or-create) so a given date always maps
//!    to exactly ONE doc. It does NOT re-derive the daily-note path/template/doc-id scheme (RISK-1/MC-1) —
//!    that ownership is MT-019's. Calling it twice for the same date returns the SAME [`DocId`] and creates
//!    no duplicate document.
//!
//! 2. **daily note <-> CalendarEvent window:** [`CalendarInteropService::resolve_event_for_daily_note`] /
//!    [`CalendarInteropService::events_for_range`] GET the calendar events for the date so the
//!    [`crate::graph::daily_journal_panel`] can render a clickable CalendarEvent chip; the click emits the
//!    [`CMD_FOCUS_CALENDAR_EVENT`] command on the WP-011 command bus targeting MT-030's calendar pane (bus
//!    only — NO calendar-pane internal import, RISK-4/MC-4).
//!
//! 3. **ActivitySpan correlation (read-only):** [`CalendarInteropService::activity_spans_for_event`] GETs
//!    the activity-spans correlation so the panel can render which documents were edited during a calendar
//!    block as READ-ONLY chips (RISK-5/MC-5 — no mutation path). A chip-click emits the navigation command
//!    [`CMD_OPEN_DOCUMENT`] only.
//!
//! ## VERIFIED BACKEND REALITY (KERNEL_BUILDER gate 2026-06-25): NO `/calendar/` routes exist at all
//!
//! VERIFIED against `src/backend/handshake_core`: there are **NO `/calendar/` HTTP routes** in this
//! handshake_core build — BOTH `GET /workspaces/{ws}/calendar/events` AND
//! `GET /workspaces/{ws}/calendar/activity-spans` are ABSENT (Calendar = Pillar 2, like FEMS = Pillar 12 and
//! Stage = Pillar 17 — a separate system not yet wired into the frozen handshake_core HTTP surface). The
//! contract's original assumption ("`/calendar/events` exists, only activity-spans maybe-absent") is WRONG:
//! the WHOLE Calendar/Pillar-2 HTTP surface is absent. So BOTH calendar reads map to the typed blocker
//! [`InteropError::EndpointUnavailable`] over a 404 / 501 / route-not-registered probe (BROAD detection,
//! RISK-3/MC-3) — never a fabricated event, never a fabricated span, never a DB query.
//!
//! ## HONEST SPLIT — the daily-note half is REAL and FULLY PROVABLE; the calendar halves are typed blockers
//!
//! - **REAL (provable now):** [`Self::open_or_create_daily_note`] delegating to the MT-019 service
//!   (idempotent, single doc/date) + the panel render + the MT-019 date nav.
//! - **TYPED BLOCKER (the designed primary path in this build):** [`Self::events_for_range`],
//!   [`Self::resolve_event_for_daily_note`], and [`Self::activity_spans_for_event`] all return
//!   [`InteropError::EndpointUnavailable`] because the `/calendar/` routes are absent. The panel renders the
//!   typed empty-states for the CalendarEvent chip + the activity strip and KEEPS the daily-note binding
//!   alive — the panel never dies on the absent calendar routes (AC-4).
//!
//! ## Reuse, no second HTTP stack / no DB / no SQLite / no new endpoint
//!
//! - The calendar reads share the process-wide [`crate::backend_client::shared_http_client`] pool + the
//!   config-resolved [`crate::backend_client::BACKEND_BASE_URL`] (the exact MT-066 `StageClient` /
//!   `MemoryClient` pattern) — NO new reqwest stack, NO new async runtime.
//! - The daily-note delegation reuses the MT-019 [`crate::rich_editor::daily_notes::journal_store::JournalBackend`]
//!   trait — NO re-implemented creation path.
//! - NO `sqlx`/`rusqlite`/`diesel`/SQLite anywhere — gaps are typed blockers, not local DB work
//!   (RISK-2/MC-2). The activity strip is GET-only (read-only correlation, RISK-5/MC-5).
//! - All date handling is chrono [`chrono::NaiveDate`] / [`chrono::DateTime<chrono::Utc>`]; the events query,
//!   the daily-note key, and the activity window all resolve to the SAME calendar day (RISK-6/MC-6).

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

use crate::backend_client::{
    shared_http_client, BACKEND_BASE_URL, HSK_HEADER_ACTOR_ID, HSK_HEADER_KERNEL_TASK_RUN_ID,
    HSK_HEADER_SESSION_RUN_ID,
};
use crate::rich_editor::daily_notes::journal_store::{JournalBackend, JournalError};

/// The bus command id a clicked CalendarEvent chip emits to focus/open MT-030's calendar pane for the
/// event (the contract's `loom.daily-note.focus-calendar-event`). Bus-only cross-pane communication —
/// the daily journal panel never imports calendar-pane internals (RISK-4/MC-4).
pub const CMD_FOCUS_CALENDAR_EVENT: &str = "loom.daily-note.focus-calendar-event";

/// The bus command id that opens-or-creates the daily note for a date (the contract's
/// `loom.daily-note.open-for-date`). Emitted when a date is selected in the calendar (MT-030) so the
/// daily journal panel opens-or-creates that date's note via the MT-019 delegation.
pub const CMD_OPEN_DAILY_NOTE_FOR_DATE: &str = "loom.daily-note.open-for-date";

/// The bus command id a clicked read-only activity document chip emits to NAVIGATE to that document (the
/// contract's `loom.activity.open-document`). It is a navigation command ONLY — the activity strip has NO
/// mutation path (RISK-5/MC-5).
pub const CMD_OPEN_DOCUMENT: &str = "loom.activity.open-document";

/// The read timeout for a calendar read (a bounded timeout so a hung backend cannot stall the editor
/// frame loop — the same bound the MT-066 Stage client uses).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

/// The least-privileged read-only actor id used for the calendar reads (no `x-hsk-actor-kind` =>
/// read-only server-side, the same least-privilege default the FEMS/Stage/knowledge read paths use).
const CALENDAR_READ_ACTOR_ID: &str = "native-editor-calendar-reader";

/// The `YYYY-MM-DD` storage format the calendar query + daily-note key share (matches the MT-019
/// [`crate::rich_editor::daily_notes::date_nav::DATE_STORAGE_FMT`]).
pub const DATE_STORAGE_FMT: &str = "%Y-%m-%d";

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Domain types
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A stable knowledge-document id. The MT contract names a `DocId`; the MT-019 daily-note service models a
/// document id as a plain `String` (`JournalBlock::document_id`), so this is a thin newtype over that
/// `String` — it gives the contract's named type while binding to the REAL doc-id the journal returns
/// (no parallel id scheme). `Ord`/`Hash` so it keys maps + sorts deterministically.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, Deserialize)]
pub struct DocId(pub String);

impl DocId {
    /// Borrow the underlying id string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DocId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for DocId {
    fn from(s: String) -> Self {
        DocId(s)
    }
}

impl From<&str> for DocId {
    fn from(s: &str) -> Self {
        DocId(s.to_owned())
    }
}

/// A calendar event window (the contract's `CalendarEvent`). Decoded from the (currently absent)
/// `GET /calendar/events` body shape; `daily_note_doc_id` is the SESSION-LOCAL bidirectional link the
/// interop writes back after opening the daily note (persisting it to the backend is out of scope unless
/// the events endpoint accepts the field — it does not exist yet, so the link stays session-local).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CalendarEvent {
    /// The stable calendar-event id.
    pub id: String,
    /// The event title shown on the chip.
    #[serde(default)]
    pub title: String,
    /// The event window start (UTC).
    pub start_utc: DateTime<Utc>,
    /// The event window end (UTC).
    pub end_utc: DateTime<Utc>,
    /// True for an all-day event (an all-day event for a date matches that date even though its window may
    /// be encoded as a midnight-to-midnight span).
    #[serde(default)]
    pub all_day: bool,
    /// The linked daily-note document id (SESSION-LOCAL — written back by the interop after open-or-create;
    /// not decoded from the backend, which has no such field). `#[serde(default)]` so a backend body
    /// without the field still decodes.
    #[serde(default)]
    pub daily_note_doc_id: Option<DocId>,
}

impl CalendarEvent {
    /// True when this event's window contains `date` (UTC calendar-day semantics, RISK-6/MC-6). An all-day
    /// event matches when `date` equals its start date; a timed event matches when `date` falls on or
    /// between the start and end calendar days (inclusive). The comparison is on the UTC *calendar day*, so
    /// a 23:30-local boundary resolves to the SAME day the events query + daily-note key use.
    pub fn contains_date(&self, date: NaiveDate) -> bool {
        let start_day = self.start_utc.date_naive();
        let end_day = self.end_utc.date_naive();
        if self.all_day {
            return date == start_day;
        }
        date >= start_day && date <= end_day
    }
}

/// A read-only activity span (the contract's `ActivitySpan`) — which documents were edited during a
/// calendar block. Decoded from the (currently absent) `GET /calendar/activity-spans` body. The interop +
/// panel only ever READ this; there is NO write path (RISK-5/MC-5).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActivitySpan {
    /// The stable span id.
    pub span_id: String,
    /// The calendar event this span correlates to, if any.
    #[serde(default)]
    pub calendar_event_id: Option<String>,
    /// The span window start (UTC).
    pub started_utc: DateTime<Utc>,
    /// The span window end (UTC).
    pub ended_utc: DateTime<Utc>,
    /// The documents edited during the span — rendered as read-only chips.
    #[serde(default)]
    pub edited_doc_ids: Vec<DocId>,
}

/// The in-session binding of a date to its daily-note doc (and, when resolvable, its calendar event). The
/// output of [`CalendarInteropService::open_or_create_daily_note`]. The interop stores this so the linkage
/// is bidirectional in the session (the [`CalendarEvent::daily_note_doc_id`] is written from `doc_id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyNoteBinding {
    /// The calendar day this binding is for.
    pub date: NaiveDate,
    /// The single daily-note document id for that date (from the idempotent MT-019 open-or-create).
    pub doc_id: DocId,
    /// The calendar event id linked to this date, if one resolves (None while the calendar routes are
    /// absent — the typed-blocker reality in this build).
    pub calendar_event_id: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error — EndpointUnavailable is the first-class typed blocker (DISTINCT from generic Http).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed outcome of any calendar interop operation.
///
/// [`Self::EndpointUnavailable`] is the FIRST-CLASS TYPED BLOCKER (RISK-3/MC-3, AC-4): a `/calendar/`
/// route is absent in this handshake_core build (404 / 501 / route-not-registered). It is DISTINCT from
/// [`Self::Http`] so the panel can tell "feature not exposed" apart from "transient failure" and render
/// the correct empty-state, and the validator can prove the blocker path. The daily-note half maps a
/// failed delegation to [`Self::DailyNoteServiceError`] (the MT-019 error, propagated — not swallowed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteropError {
    /// A non-success HTTP status that is NOT the typed endpoint-absent blocker (e.g. a 500, a 403). Carries
    /// the status code.
    Http { status: u16 },
    /// A decode failure on a success body (the wire shape did not match the domain type). Carries the
    /// serde reason.
    Decode(String),
    /// THE TYPED BLOCKER: the probed `/calendar/` route is absent in this build (404 / 501 /
    /// route-not-registered). Carries the probed path so the validator + operator see exactly which route
    /// is missing. NO backend route is added; NO event/span is fabricated.
    EndpointUnavailable { probed_path: String },
    /// The MT-019 daily-note service failed (propagated from [`JournalError`], NOT swallowed). The
    /// open-or-create delegates to MT-019; its failure surfaces here distinctly so the panel can show the
    /// MT-019 error chip rather than a calendar empty-state.
    DailyNoteServiceError(String),
    /// A resource was addressed but not found in a way distinct from the typed endpoint-absent blocker
    /// (reserved for a future per-resource 404 that is NOT a missing `/calendar/` route). Carries no
    /// payload; the typed blocker for an absent route is [`Self::EndpointUnavailable`], not this.
    NotFound,
    /// A transport-layer failure (connect / timeout / TLS). Carries the reason.
    Transport(String),
}

impl std::fmt::Display for InteropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { status } => write!(f, "calendar interop: HTTP {status}"),
            Self::Decode(why) => write!(f, "calendar interop decode error: {why}"),
            Self::EndpointUnavailable { probed_path } => write!(
                f,
                "Calendar endpoint not present in this build (probed {probed_path})"
            ),
            Self::DailyNoteServiceError(why) => {
                write!(f, "daily-note service error: {why}")
            }
            Self::NotFound => write!(f, "calendar interop: resource not found"),
            Self::Transport(why) => write!(f, "calendar interop transport error: {why}"),
        }
    }
}

impl std::error::Error for InteropError {}

impl InteropError {
    /// True when this is the typed-blocker variant (the panel renders the typed empty-state and the blocker
    /// is surfaced to the WP validator). DISTINCT from a generic [`Self::Http`] error (RISK-3/MC-3).
    pub fn is_endpoint_unavailable(&self) -> bool {
        matches!(self, InteropError::EndpointUnavailable { .. })
    }

    /// The stable empty-state message the panel shows for a typed-blocker activity strip (AC-4). A fixed
    /// string so the panel + the User Manual + the tests reference the same copy.
    pub const ACTIVITY_UNAVAILABLE_MSG: &'static str =
        "Activity correlation not available — backend endpoint not exposed";

    /// The stable empty-state message for an unresolved CalendarEvent chip (the calendar-events route is
    /// absent in this build).
    pub const EVENT_UNAVAILABLE_MSG: &'static str =
        "Calendar event not available — backend endpoint not exposed";
}

/// A typed result alias for calendar interop operations.
pub type InteropResult<T> = Result<T, InteropError>;

/// Map a [`JournalError`] (MT-019) into the calendar interop error model. A failed daily-note delegation
/// surfaces as [`InteropError::DailyNoteServiceError`] (propagated, never swallowed — RISK-1).
impl From<JournalError> for InteropError {
    fn from(e: JournalError) -> Self {
        InteropError::DailyNoteServiceError(e.to_string())
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// The interop service.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The Calendar interop service (the contract's `CalendarInteropService`). Holds:
/// - the shared [`reqwest::Client`] pool + the config-resolved base url for the calendar reads (NO second
///   HTTP stack), and
/// - the MT-019 [`JournalBackend`] (an `Arc<dyn …>`) the daily-note open-or-create DELEGATES to (RISK-1:
///   no re-implemented creation path).
///
/// All four contract methods are async; the daily-note half is REAL and provable, the calendar halves are
/// the designed typed blockers in this build (no `/calendar/` routes).
#[derive(Clone)]
pub struct CalendarInteropService {
    /// The shared HTTP pool (the WP-011 `backend_client` pool — no second stack).
    http: reqwest::Client,
    /// The config-resolved backend base URL (never hardcoded at a call site — GLOBAL-PORTABILITY-004).
    base_url: String,
    /// The workspace the calendar + daily notes belong to.
    workspace_id: String,
    /// The MT-019 daily-note backend the open-or-create delegates to (RISK-1/MC-1: single-owner creation).
    journal_backend: Arc<dyn JournalBackend>,
    /// The session run id on the read identity headers (so swarm/operator co-work is attributable).
    session_run_id: String,
}

impl CalendarInteropService {
    /// Construct against the production backend (the config-resolved [`BACKEND_BASE_URL`], the shared
    /// [`shared_http_client`] pool) for `workspace_id`, delegating the daily-note open-or-create to the
    /// production MT-019 backend.
    pub fn production(
        workspace_id: impl Into<String>,
        journal_backend: Arc<dyn JournalBackend>,
    ) -> Self {
        Self {
            http: shared_http_client(),
            base_url: BACKEND_BASE_URL.to_owned(),
            workspace_id: workspace_id.into(),
            journal_backend,
            session_run_id: "native-editor-session".to_owned(),
        }
    }

    /// Construct against an explicit base URL on a FRESH client (used by tests to point at a mock server
    /// with an isolated pool). The base URL is the host authority — never hardcoded at a call site
    /// (GLOBAL-PORTABILITY-004).
    pub fn with_base_url(
        base_url: impl Into<String>,
        workspace_id: impl Into<String>,
        journal_backend: Arc<dyn JournalBackend>,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            workspace_id: workspace_id.into(),
            journal_backend,
            session_run_id: "native-editor-session".to_owned(),
        }
    }

    /// Override the session run id on the read identity headers (builder-style).
    pub fn with_session_run_id(mut self, session_run_id: impl Into<String>) -> Self {
        self.session_run_id = session_run_id.into();
        self
    }

    /// The workspace this service binds.
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// The events read path for the workspace + date range (the documented — currently absent — route).
    /// Built here so [`InteropError::EndpointUnavailable`] can report the exact probed path.
    pub fn events_path(workspace_id: &str, from: NaiveDate, to: NaiveDate) -> String {
        format!(
            "/workspaces/{workspace_id}/calendar/events?from={}&to={}",
            from.format(DATE_STORAGE_FMT),
            to.format(DATE_STORAGE_FMT)
        )
    }

    /// The activity-spans read path for the workspace + event (the documented — currently absent — route).
    pub fn activity_spans_path(workspace_id: &str, event_id: &str) -> String {
        format!("/workspaces/{workspace_id}/calendar/activity-spans?event_id={event_id}")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Issue a read-only GET at `path`, mapping the response to a decoded `T` or the typed error model. A
    /// 404 / 501 (route absent / not implemented) maps to [`InteropError::EndpointUnavailable`] — the TYPED
    /// BLOCKER (BROAD detection, RISK-3/MC-3), never a panic or a fabricated value. READ-ONLY: a single GET,
    /// never a write verb (RISK-5/MC-5).
    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> InteropResult<T> {
        let url = self.url(path);
        let resp = self
            .http
            .get(&url)
            .timeout(REQUEST_TIMEOUT)
            // READ identity: least-privileged read-only actor (no x-hsk-actor-kind => read-only).
            .header(HSK_HEADER_ACTOR_ID, CALENDAR_READ_ACTOR_ID)
            .header(
                HSK_HEADER_KERNEL_TASK_RUN_ID,
                format!("native-editor-calendar-{}", self.workspace_id),
            )
            .header(HSK_HEADER_SESSION_RUN_ID, &self.session_run_id)
            .send()
            .await
            .map_err(|e| InteropError::Transport(e.to_string()))?;
        let status = resp.status();

        // THE TYPED BLOCKER (BROAD detection — RISK-3/MC-3): 404 (route absent) OR 501 (not implemented)
        // both mean the /calendar/ route is not present in this build. Surface it as the typed blocker
        // DISTINCT from a generic Http error; never panic, never fabricate.
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::NOT_IMPLEMENTED
        {
            return Err(InteropError::EndpointUnavailable {
                probed_path: path.to_owned(),
            });
        }
        if !status.is_success() {
            return Err(InteropError::Http { status: status.as_u16() });
        }
        resp.json::<T>()
            .await
            .map_err(|e| InteropError::Decode(e.to_string()))
    }

    // ── date -> daily note (idempotent open-or-create, REAL — delegates to MT-019) ──────────────────────

    /// Open-or-create the daily note for `date` IDEMPOTENTLY by DELEGATING to the MT-019 daily-note service
    /// (RISK-1/MC-1). This calls [`JournalBackend::open_daily_journal`] (the verified
    /// `PUT /workspaces/{ws}/loom/journals/{date}` get-or-create), so a date maps to exactly ONE doc:
    /// calling this twice for the same date returns the SAME [`DocId`] and creates no duplicate document.
    /// It does NOT re-derive the daily-note path/template/doc-id scheme — that is MT-019's single ownership.
    ///
    /// The returned [`JournalBlock`](crate::rich_editor::daily_notes::journal_store::JournalBlock) may have
    /// no linked document yet (a brand-new journal block before "Start writing"); in that case the block id
    /// itself is the stable date->doc key, so the binding's `doc_id` falls back to the block id (still
    /// idempotent — the same date returns the same block id). A failed delegation surfaces as
    /// [`InteropError::DailyNoteServiceError`] (propagated, never swallowed).
    pub async fn open_or_create_daily_note(&self, date: NaiveDate) -> InteropResult<DailyNoteBinding> {
        let journal_date = date.format(DATE_STORAGE_FMT).to_string();
        let block = self
            .journal_backend
            .open_daily_journal(&self.workspace_id, &journal_date)
            .await?;
        // The single date->doc id: the linked knowledge document when present, else the journal block id
        // (the stable get-or-create key for a not-yet-written journal). Both are idempotent for a date.
        let doc_id = block
            .document_id
            .clone()
            .unwrap_or_else(|| block.block_id.clone());
        Ok(DailyNoteBinding {
            date,
            doc_id: DocId(doc_id),
            calendar_event_id: None,
        })
    }

    // ── daily note <-> CalendarEvent window (TYPED BLOCKER in this build — no /calendar/events route) ────

    /// Fetch the calendar events overlapping `[from, to]` (the contract's `events_for_range`). In THIS
    /// build the `/calendar/events` route is ABSENT, so this returns [`InteropError::EndpointUnavailable`]
    /// (the typed blocker) — never a fabricated event. When the route lands, it decodes the events body.
    pub async fn events_for_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> InteropResult<Vec<CalendarEvent>> {
        let path = Self::events_path(&self.workspace_id, from, to);
        self.get_json::<Vec<CalendarEvent>>(&path).await
    }

    /// Resolve the CalendarEvent for a daily note's `date` (the contract's `resolve_event_for_daily_note`):
    /// fetch the events for that single day and pick the event whose window contains the date (or the
    /// all-day event for that date). Returns `Ok(None)` when the route exists but no event matches; returns
    /// [`InteropError::EndpointUnavailable`] when the route is absent (this build) so the panel renders the
    /// unavailable chip empty-state while the daily-note binding stays alive (AC-2 / AC-4). The day-window
    /// match is UTC calendar-day (RISK-6/MC-6).
    pub async fn resolve_event_for_daily_note(
        &self,
        date: NaiveDate,
    ) -> InteropResult<Option<CalendarEvent>> {
        let events = self.events_for_range(date, date).await?;
        Ok(pick_event_for_date(&events, date))
    }

    // ── ActivitySpan correlation (READ-ONLY, TYPED BLOCKER — no /calendar/activity-spans route) ──────────

    /// Fetch the read-only ActivitySpan correlation for `event_id` (the contract's
    /// `activity_spans_for_event`): which documents were edited during the calendar block. In THIS build
    /// the `/calendar/activity-spans` route is ABSENT, so this returns
    /// [`InteropError::EndpointUnavailable`] (the typed blocker) — the panel then shows the typed
    /// empty-state ([`InteropError::ACTIVITY_UNAVAILABLE_MSG`]) and the rest of the panel stays alive
    /// (AC-3 / AC-4). READ-ONLY: a single GET, never a write (RISK-5/MC-5).
    pub async fn activity_spans_for_event(
        &self,
        event_id: &str,
    ) -> InteropResult<Vec<ActivitySpan>> {
        let path = Self::activity_spans_path(&self.workspace_id, event_id);
        self.get_json::<Vec<ActivitySpan>>(&path).await
    }
}

/// Pick the CalendarEvent whose window contains `date` (the resolve-for-daily-note selection rule, RISK-6/
/// MC-6). Prefers an ALL-DAY event for the date over a timed one (the daily note is the day's anchor), then
/// the first timed event whose window contains the date. UTC calendar-day semantics throughout. Pure (no
/// IO) so it is unit-testable with fixture events.
pub fn pick_event_for_date(events: &[CalendarEvent], date: NaiveDate) -> Option<CalendarEvent> {
    // An all-day event for the date is the strongest match (the day's anchor).
    if let Some(all_day) = events.iter().find(|e| e.all_day && e.contains_date(date)) {
        return Some(all_day.clone());
    }
    // Else the first timed event whose UTC window contains the date.
    events.iter().find(|e| e.contains_date(date)).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::daily_notes::journal_store::{
        JournalBlock, JournalDocLoad, JournalFuture,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn utc(y: i32, m: u32, day: u32, h: u32, min: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, day)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
            .and_utc()
    }

    /// A counted mock MT-019 backend: `open_daily_journal` returns the SAME block for a given date (the
    /// real backend's get-or-create idempotency) and counts how many times it was called, so a test proves
    /// the delegation (RISK-1) and idempotency. It NEVER creates a second block for the same date.
    struct CountingJournalBackend {
        opens: AtomicUsize,
        /// The doc id the (single) block for any date carries (Some => a written journal; None => a blank
        /// journal block whose block_id is the date->doc key).
        document_id: Option<String>,
    }

    impl CountingJournalBackend {
        fn new(document_id: Option<&str>) -> Self {
            Self {
                opens: AtomicUsize::new(0),
                document_id: document_id.map(|s| s.to_owned()),
            }
        }
    }

    impl JournalBackend for CountingJournalBackend {
        fn open_daily_journal<'a>(
            &'a self,
            workspace_id: &'a str,
            journal_date: &'a str,
        ) -> JournalFuture<'a, JournalBlock> {
            self.opens.fetch_add(1, Ordering::SeqCst);
            let ws = workspace_id.to_owned();
            let date = journal_date.to_owned();
            let document_id = self.document_id.clone();
            Box::pin(async move {
                // The block id is DETERMINISTIC for a date (the get-or-create key): the same date always
                // yields the same block id and the same linked document_id => idempotent.
                Ok(JournalBlock {
                    block_id: format!("journal-{date}"),
                    workspace_id: ws,
                    content_type: Some("journal".to_owned()),
                    document_id,
                    title: Some(format!("Daily Note {date}")),
                    journal_date: Some(date),
                })
            })
        }

        fn load_document<'a>(&'a self, _document_id: &'a str) -> JournalFuture<'a, JournalDocLoad> {
            Box::pin(async move { Err(JournalError::DocLoadFailed("unused in this test".into())) })
        }

        fn create_document<'a>(
            &'a self,
            _workspace_id: &'a str,
            _title: &'a str,
        ) -> JournalFuture<'a, JournalDocLoad> {
            Box::pin(async move { Err(JournalError::CreateFailed("unused in this test".into())) })
        }
    }

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    }

    /// AC-1 / RISK-1: open_or_create_daily_note delegates to the MT-019 backend and is idempotent — twice
    /// for the same date returns the SAME DocId and creates NO duplicate (one block per date).
    #[test]
    fn open_or_create_is_idempotent_and_delegates() {
        let backend = Arc::new(CountingJournalBackend::new(Some("DOC-2026-06-21")));
        let svc = CalendarInteropService::with_base_url("http://unused", "WS-1", backend.clone());
        let date = d(2026, 6, 21);
        let (a, b) = rt().block_on(async {
            let a = svc.open_or_create_daily_note(date).await.expect("first open");
            let b = svc.open_or_create_daily_note(date).await.expect("second open");
            (a, b)
        });
        // Same DocId both times (idempotent), and the doc id is the LINKED document (delegation result).
        assert_eq!(a.doc_id, b.doc_id, "AC-1: same date -> same DocId (idempotent)");
        assert_eq!(a.doc_id, DocId("DOC-2026-06-21".to_owned()));
        assert_eq!(a.date, date);
        // The MT-019 backend was the creation path (delegation, RISK-1): it was called, not bypassed.
        assert_eq!(backend.opens.load(Ordering::SeqCst), 2, "delegated to MT-019 both times");
    }

    /// A blank journal block (no linked document yet) still yields a stable, idempotent date->doc key (the
    /// block id), so open-or-create stays single-doc-per-date even before "Start writing".
    #[test]
    fn open_or_create_blank_block_uses_stable_block_id() {
        let backend = Arc::new(CountingJournalBackend::new(None));
        let svc = CalendarInteropService::with_base_url("http://unused", "WS-1", backend);
        let date = d(2026, 1, 31);
        let binding = rt().block_on(async { svc.open_or_create_daily_note(date).await.unwrap() });
        assert_eq!(binding.doc_id, DocId("journal-2026-01-31".to_owned()));
    }

    /// A failed MT-019 delegation surfaces as DailyNoteServiceError (propagated, never swallowed, RISK-1).
    #[test]
    fn open_or_create_propagates_mt019_error() {
        struct FailingBackend;
        impl JournalBackend for FailingBackend {
            fn open_daily_journal<'a>(
                &'a self,
                _ws: &'a str,
                _date: &'a str,
            ) -> JournalFuture<'a, JournalBlock> {
                Box::pin(async move { Err(JournalError::OpenFailed("backend down".into())) })
            }
            fn load_document<'a>(&'a self, _id: &'a str) -> JournalFuture<'a, JournalDocLoad> {
                Box::pin(async move { Err(JournalError::DocLoadFailed("x".into())) })
            }
            fn create_document<'a>(
                &'a self,
                _ws: &'a str,
                _t: &'a str,
            ) -> JournalFuture<'a, JournalDocLoad> {
                Box::pin(async move { Err(JournalError::CreateFailed("x".into())) })
            }
        }
        let svc =
            CalendarInteropService::with_base_url("http://unused", "WS-1", Arc::new(FailingBackend));
        let err = rt()
            .block_on(async { svc.open_or_create_daily_note(d(2026, 6, 21)).await })
            .unwrap_err();
        assert!(
            matches!(err, InteropError::DailyNoteServiceError(_)),
            "RISK-1: MT-019 error propagates as DailyNoteServiceError, got {err:?}"
        );
    }

    /// RISK-6/MC-6: contains_date is UTC calendar-day. A timed event 2026-06-21 22:00 -> 2026-06-22 02:00
    /// (UTC) contains BOTH 06-21 and 06-22; a 23:30 boundary resolves to the same UTC day the query uses.
    #[test]
    fn contains_date_is_utc_calendar_day() {
        let timed = CalendarEvent {
            id: "E-1".into(),
            title: "Late block".into(),
            start_utc: utc(2026, 6, 21, 22, 0),
            end_utc: utc(2026, 6, 22, 2, 0),
            all_day: false,
            daily_note_doc_id: None,
        };
        assert!(timed.contains_date(d(2026, 6, 21)));
        assert!(timed.contains_date(d(2026, 6, 22)));
        assert!(!timed.contains_date(d(2026, 6, 20)));
        assert!(!timed.contains_date(d(2026, 6, 23)));

        // A 23:30 single-instant event resolves to its UTC day only.
        let late = CalendarEvent {
            id: "E-2".into(),
            title: "23:30".into(),
            start_utc: utc(2026, 6, 21, 23, 30),
            end_utc: utc(2026, 6, 21, 23, 30),
            all_day: false,
            daily_note_doc_id: None,
        };
        assert!(late.contains_date(d(2026, 6, 21)));
        assert!(!late.contains_date(d(2026, 6, 22)));

        // An all-day event matches only its start date.
        let all_day = CalendarEvent {
            id: "E-3".into(),
            title: "All day".into(),
            start_utc: utc(2026, 6, 21, 0, 0),
            end_utc: utc(2026, 6, 21, 23, 59),
            all_day: true,
            daily_note_doc_id: None,
        };
        assert!(all_day.contains_date(d(2026, 6, 21)));
        assert!(!all_day.contains_date(d(2026, 6, 22)));
    }

    /// pick_event_for_date prefers an all-day event for the date, then the first timed match.
    #[test]
    fn pick_event_prefers_all_day_then_timed() {
        let timed = CalendarEvent {
            id: "T".into(),
            title: "timed".into(),
            start_utc: utc(2026, 6, 21, 9, 0),
            end_utc: utc(2026, 6, 21, 10, 0),
            all_day: false,
            daily_note_doc_id: None,
        };
        let all_day = CalendarEvent {
            id: "A".into(),
            title: "all day".into(),
            start_utc: utc(2026, 6, 21, 0, 0),
            end_utc: utc(2026, 6, 21, 23, 59),
            all_day: true,
            daily_note_doc_id: None,
        };
        let events = vec![timed.clone(), all_day.clone()];
        assert_eq!(pick_event_for_date(&events, d(2026, 6, 21)).unwrap().id, "A");
        // Only the timed event present -> it is picked.
        assert_eq!(pick_event_for_date(&[timed], d(2026, 6, 21)).unwrap().id, "T");
        // No match -> None.
        assert!(pick_event_for_date(&[all_day], d(2026, 6, 22)).is_none());
    }

    /// The read paths are the documented `/calendar/` route shapes (so the typed blocker names them).
    #[test]
    fn read_paths_are_documented_routes() {
        assert_eq!(
            CalendarInteropService::events_path("WS-1", d(2026, 6, 21), d(2026, 6, 21)),
            "/workspaces/WS-1/calendar/events?from=2026-06-21&to=2026-06-21"
        );
        assert_eq!(
            CalendarInteropService::activity_spans_path("WS-1", "E-9"),
            "/workspaces/WS-1/calendar/activity-spans?event_id=E-9"
        );
    }

    /// The typed-blocker variant is DISTINCT from a generic Http error (RISK-3/MC-3) and names the probed
    /// path; its empty-state messages are stable.
    #[test]
    fn endpoint_unavailable_is_distinct_typed_blocker() {
        let blocker = InteropError::EndpointUnavailable {
            probed_path: "/workspaces/WS-1/calendar/activity-spans?event_id=E-1".into(),
        };
        assert!(blocker.is_endpoint_unavailable());
        assert!(!InteropError::Http { status: 500 }.is_endpoint_unavailable());
        assert!(blocker.to_string().contains("/calendar/activity-spans"));
        assert!(InteropError::ACTIVITY_UNAVAILABLE_MSG.contains("not available"));
    }

    /// A CalendarEvent body decodes from the documented wire shape (the route, once it lands).
    #[test]
    fn calendar_event_decodes_from_wire() {
        let body = serde_json::json!({
            "id": "E-7",
            "title": "Sprint planning",
            "start_utc": "2026-06-21T09:00:00Z",
            "end_utc": "2026-06-21T10:00:00Z",
            "all_day": false
        });
        let ev: CalendarEvent = serde_json::from_value(body).expect("decodes");
        assert_eq!(ev.id, "E-7");
        assert_eq!(ev.title, "Sprint planning");
        assert!(ev.daily_note_doc_id.is_none());
        assert!(ev.contains_date(d(2026, 6, 21)));
    }

    /// An ActivitySpan body decodes with its edited doc ids (the read-only correlation wire).
    #[test]
    fn activity_span_decodes_with_edited_docs() {
        let body = serde_json::json!({
            "span_id": "S-1",
            "calendar_event_id": "E-7",
            "started_utc": "2026-06-21T09:05:00Z",
            "ended_utc": "2026-06-21T09:45:00Z",
            "edited_doc_ids": ["DOC-A", "DOC-B"]
        });
        let span: ActivitySpan = serde_json::from_value(body).expect("decodes");
        assert_eq!(span.span_id, "S-1");
        assert_eq!(span.calendar_event_id.as_deref(), Some("E-7"));
        assert_eq!(
            span.edited_doc_ids,
            vec![DocId("DOC-A".into()), DocId("DOC-B".into())]
        );
    }
}
