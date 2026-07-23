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
//! ## Backend reality
//!
//! `handshake_core` exposes both workspace-scoped reads used here:
//! `GET /workspaces/{ws}/calendar/events` and
//! `GET /workspaces/{ws}/calendar/activity-spans`. A deployment where either route is unavailable still
//! maps 404/501 to [`InteropError::EndpointUnavailable`] (RISK-3/MC-3), never to a fabricated event/span.
//!
//! ## Runtime split
//!
//! - [`Self::open_or_create_daily_note`] delegates to the MT-019 service (idempotent, single doc/date).
//! - [`Self::events_for_range`], [`Self::resolve_event_for_daily_note`], and
//!   [`Self::activity_spans_for_event`] read the live Calendar routes. The panel retains a typed unavailable
//!   state when a route cannot be reached or is not exposed (AC-4).
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

use chrono::{DateTime, LocalResult, NaiveDate, SecondsFormat, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

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

/// A calendar event window (the contract's `CalendarEvent`). Decoded from the live
/// `GET /calendar/events` body shape; `daily_note_doc_id` is the persisted, date-derived reverse link to
/// the canonical daily journal returned by the backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// The stable calendar-event id.
    pub id: String,
    /// The event title shown on the chip.
    #[serde(default)]
    pub title: String,
    /// Lossless timed or date-only temporal intent from the backend.
    pub temporal: CalendarEventTemporal,
    /// The linked daily-note document id decoded from the backend. `#[serde(default)]` preserves decoding
    /// for dates that do not have a daily journal yet.
    #[serde(default)]
    pub daily_note_doc_id: Option<DocId>,
    /// The selected Calendar view timezone used for this query. It is runtime
    /// projection context, not part of the persisted event wire.
    #[serde(skip, default = "system_view_tzid")]
    pub view_tzid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CalendarEventTemporal {
    Timed {
        start_utc: DateTime<Utc>,
        end_utc: DateTime<Utc>,
        start_local: String,
        end_local: String,
        tzid: String,
        was_floating: bool,
        #[serde(default)]
        normalization_note: Option<CalendarNormalizationNote>,
    },
    AllDay {
        start_date: NaiveDate,
        end_date_exclusive: NaiveDate,
        tzid: String,
    },
    LegacyIncomplete {
        start_utc: DateTime<Utc>,
        end_utc: DateTime<Utc>,
        tzid: String,
        all_day: bool,
        recovery: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarNormalizationNote {
    #[serde(default)]
    pub boundaries: Vec<CalendarBoundaryNormalization>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarBoundaryNormalization {
    pub boundary: String,
    pub original_local: String,
    pub resolution: CalendarDstResolution,
    pub resolved_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalendarDstResolution {
    EarlierOffset,
    LaterOffset,
}

impl CalendarNormalizationNote {
    fn operator_summary(&self) -> String {
        self.boundaries
            .iter()
            .map(|boundary| {
                let resolution = match boundary.resolution {
                    CalendarDstResolution::EarlierOffset => "earlier offset",
                    CalendarDstResolution::LaterOffset => "later offset",
                };
                format!(
                    "{} {} => {} ({})",
                    boundary.boundary,
                    boundary.original_local,
                    resolution,
                    boundary.resolved_utc.to_rfc3339()
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

fn system_view_tzid() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".to_owned())
}

pub fn selected_date_window_utc(
    date: NaiveDate,
    view_tzid: &str,
) -> InteropResult<(DateTime<Utc>, DateTime<Utc>)> {
    let end_date = date.succ_opt().ok_or(InteropError::InvalidDateWindow)?;
    Ok((
        selected_date_boundary_utc(date, view_tzid)?,
        selected_date_boundary_utc(end_date, view_tzid)?,
    ))
}

fn selected_date_boundary_utc(date: NaiveDate, view_tzid: &str) -> InteropResult<DateTime<Utc>> {
    let tz: Tz = view_tzid
        .parse()
        .map_err(|_| InteropError::InvalidTimezone(view_tzid.to_owned()))?;
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or(InteropError::InvalidDateWindow)?;
    for minute in 0..=(24 * 60) {
        let Some(candidate) = midnight.checked_add_signed(chrono::Duration::minutes(minute)) else {
            break;
        };
        match tz.from_local_datetime(&candidate) {
            LocalResult::Single(value) => return Ok(value.with_timezone(&Utc)),
            LocalResult::Ambiguous(first, second) => {
                return Ok(std::cmp::min(
                    first.with_timezone(&Utc),
                    second.with_timezone(&Utc),
                ))
            }
            LocalResult::None => {}
        }
    }
    Err(InteropError::InvalidDateWindow)
}

impl CalendarEvent {
    /// True when this event overlaps `date` in the selected Calendar view timezone.
    /// Both timed and all-day comparisons are half-open: a timed event overlaps
    /// `[local midnight, next local midnight)`, while an all-day event owns
    /// `[start_date, end_date_exclusive)`. This keeps near-midnight and DST days
    /// aligned with the visible daily-note key.
    pub fn contains_date(&self, date: NaiveDate) -> bool {
        match &self.temporal {
            CalendarEventTemporal::AllDay {
                start_date,
                end_date_exclusive,
                ..
            } => date >= *start_date && date < *end_date_exclusive,
            CalendarEventTemporal::Timed {
                start_utc, end_utc, ..
            } => selected_date_window_utc(date, &self.view_tzid)
                .map(|(day_start, day_end)| *start_utc < day_end && *end_utc > day_start)
                .unwrap_or(false),
            CalendarEventTemporal::LegacyIncomplete {
                start_utc, end_utc, ..
            } => selected_date_window_utc(date, &self.view_tzid)
                .map(|(day_start, day_end)| *start_utc < day_end && *end_utc > day_start)
                .unwrap_or(false),
        }
    }

    pub fn is_all_day(&self) -> bool {
        matches!(&self.temporal, CalendarEventTemporal::AllDay { .. })
    }

    pub fn is_legacy_incomplete(&self) -> bool {
        matches!(
            &self.temporal,
            CalendarEventTemporal::LegacyIncomplete { .. }
        )
    }

    pub fn has_dst_normalization(&self) -> bool {
        matches!(
            &self.temporal,
            CalendarEventTemporal::Timed {
                normalization_note: Some(note),
                ..
            } if !note.boundaries.is_empty()
        )
    }

    pub fn temporal_summary(&self) -> String {
        match &self.temporal {
            CalendarEventTemporal::Timed {
                start_utc,
                end_utc,
                start_local,
                end_local,
                tzid,
                was_floating,
                normalization_note,
            } => format!(
                "Timed\nStart local: {start_local}\nEnd local: {end_local}\nTimezone: {tzid}\nStart UTC: {}\nEnd UTC: {}\nWas floating: {was_floating}\nDST normalization: {}",
                start_utc.to_rfc3339(),
                end_utc.to_rfc3339(),
                normalization_note
                    .as_ref()
                    .map(CalendarNormalizationNote::operator_summary)
                    .filter(|summary| !summary.is_empty())
                    .unwrap_or_else(|| "none".to_owned())
            ),
            CalendarEventTemporal::AllDay {
                start_date,
                end_date_exclusive,
                tzid,
            } => format!(
                "All day\nStart date: {start_date}\nEnd date (exclusive): {end_date_exclusive}\nTimezone: {tzid}"
            ),
            CalendarEventTemporal::LegacyIncomplete {
                start_utc,
                end_utc,
                tzid,
                all_day,
                recovery,
            } => format!(
                "Legacy temporal data incomplete\nUTC fallback: {} – {}\nTimezone: {tzid}\nWas all-day: {all_day}\nRecovery: {recovery}",
                start_utc.to_rfc3339(),
                end_utc.to_rfc3339(),
            ),
        }
    }
}

/// A read-only activity span (the contract's `ActivitySpan`) — which documents were edited during a
/// calendar block. Decoded from the live `GET /calendar/activity-spans` body. The interop +
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
    pub ended_utc: Option<DateTime<Utc>>,
    /// The documents edited during the span — rendered as read-only chips.
    #[serde(default)]
    pub edited_doc_ids: Vec<DocId>,
}

/// The binding of a date to its canonical daily-note doc (and, when resolvable, its calendar event). The
/// output of [`CalendarInteropService::open_or_create_daily_note`]. The Calendar events route derives the
/// reverse [`CalendarEvent::daily_note_doc_id`] from the persisted journal row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyNoteBinding {
    /// The calendar day this binding is for.
    pub date: NaiveDate,
    /// The single daily-note document id for that date (from the idempotent MT-019 open-or-create).
    pub doc_id: DocId,
    /// The calendar event id linked to this date, if one resolves.
    pub calendar_event_id: Option<String>,
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Typed error — EndpointUnavailable is the first-class typed blocker (DISTINCT from generic Http).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The typed outcome of any calendar interop operation.
///
/// [`Self::EndpointUnavailable`] is the FIRST-CLASS TYPED BLOCKER (RISK-3/MC-3, AC-4): a `/calendar/`
/// route is unavailable in the attached deployment (404 / 501 / route-not-registered). It is DISTINCT from
/// [`Self::Http`] so the panel can tell "feature not exposed" apart from "transient failure" and render
/// the correct empty-state, and the validator can prove the blocker path. The daily-note half maps a
/// failed delegation to [`Self::DailyNoteServiceError`] (the MT-019 error, propagated — not swallowed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteropError {
    /// A non-success HTTP status that is NOT the typed endpoint-unavailable blocker (e.g. a 500, a 403). Carries
    /// the status code.
    Http { status: u16 },
    /// A decode failure on a success body (the wire shape did not match the domain type). Carries the
    /// serde reason.
    Decode(String),
    /// THE TYPED BLOCKER: the probed `/calendar/` route is unavailable (404 / 501 /
    /// route-not-registered). Carries the probed path so the validator + operator see exactly which route
    /// is unavailable. NO event/span is fabricated.
    EndpointUnavailable { probed_path: String },
    /// The MT-019 daily-note service failed (propagated from [`JournalError`], NOT swallowed). The
    /// open-or-create delegates to MT-019; its failure surfaces here distinctly so the panel can show the
    /// MT-019 error chip rather than a calendar empty-state.
    DailyNoteServiceError(String),
    /// The idempotent MT-019 daily-note PUT hit a retryable transport/HTTP failure. Mounted callers may
    /// retry it within the same date/generation; non-retryable service and decode errors use the variant above.
    DailyNoteTransient(String),
    /// A resource was addressed but not found in a way distinct from the typed endpoint-unavailable blocker
    /// (reserved for a future per-resource 404 that is NOT a missing `/calendar/` route). Carries no
    /// payload; the typed blocker for an unavailable route is [`Self::EndpointUnavailable`], not this.
    NotFound,
    /// A transport-layer failure (connect / timeout / TLS). Carries the reason.
    Transport(String),
    /// The selected Calendar view timezone is not an IANA tzdb identifier.
    InvalidTimezone(String),
    /// The requested local date cannot be represented as a half-open window.
    InvalidDateWindow,
}

impl std::fmt::Display for InteropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http { status } => write!(f, "calendar interop: HTTP {status}"),
            Self::Decode(why) => write!(f, "calendar interop decode error: {why}"),
            Self::EndpointUnavailable { probed_path } => {
                write!(f, "Calendar endpoint unavailable (probed {probed_path})")
            }
            Self::DailyNoteServiceError(why) => {
                write!(f, "daily-note service error: {why}")
            }
            Self::DailyNoteTransient(why) => {
                write!(f, "daily-note service transient error: {why}")
            }
            Self::NotFound => write!(f, "calendar interop: resource not found"),
            Self::Transport(why) => write!(f, "calendar interop transport error: {why}"),
            Self::InvalidTimezone(tzid) => {
                write!(f, "calendar interop invalid IANA timezone: {tzid}")
            }
            Self::InvalidDateWindow => write!(f, "calendar interop invalid date window"),
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

    /// True only for operations safe to retry inside one mounted request generation: transport errors,
    /// HTTP 408/425/429/5xx, and the typed idempotent daily-note PUT transient. EndpointUnavailable,
    /// other 4xx responses, and schema decode failures are deliberately terminal.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            InteropError::Transport(_)
                | InteropError::DailyNoteTransient(_)
                | InteropError::Http {
                    status: 408 | 425 | 429 | 500..=599
                }
        )
    }

    /// The stable empty-state message the panel shows for a typed-blocker activity strip (AC-4). A fixed
    /// string so the panel + the User Manual + the tests reference the same copy.
    pub const ACTIVITY_UNAVAILABLE_MSG: &'static str =
        "Activity correlation not available — backend endpoint not exposed";

    /// The stable empty-state message for an unavailable CalendarEvent read.
    pub const EVENT_UNAVAILABLE_MSG: &'static str =
        "Calendar event not available — backend endpoint not exposed";
}

/// A typed result alias for calendar interop operations.
pub type InteropResult<T> = Result<T, InteropError>;

/// Map a [`JournalError`] (MT-019) into the calendar interop error model. A failed daily-note delegation
/// surfaces as [`InteropError::DailyNoteServiceError`] (propagated, never swallowed — RISK-1).
impl From<JournalError> for InteropError {
    fn from(e: JournalError) -> Self {
        match e {
            JournalError::OpenTransient(why) => InteropError::DailyNoteTransient(why),
            other => InteropError::DailyNoteServiceError(other.to_string()),
        }
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
/// All four contract methods are async and use the production daily-note and Calendar routes; typed
/// unavailable results preserve the mounted panel when an attached backend lacks either Calendar read.
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
    /// IANA timezone selected by the Calendar view. Date windows are converted
    /// to UTC with tzdb before the query is sent.
    view_tzid: String,
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
            view_tzid: system_view_tzid(),
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
            view_tzid: system_view_tzid(),
        }
    }

    /// Override the session run id on the read identity headers (builder-style).
    pub fn with_session_run_id(mut self, session_run_id: impl Into<String>) -> Self {
        self.session_run_id = session_run_id.into();
        self
    }

    pub fn with_view_tzid(mut self, view_tzid: impl Into<String>) -> Self {
        self.view_tzid = view_tzid.into();
        self
    }

    /// The workspace this service binds.
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// The events read path for the workspace + date range.
    /// Built here so [`InteropError::EndpointUnavailable`] can report the exact probed path.
    pub fn events_path(
        workspace_id: &str,
        from: NaiveDate,
        to: NaiveDate,
        view_tzid: &str,
    ) -> InteropResult<String> {
        let to_date_exclusive = to.succ_opt().ok_or(InteropError::InvalidDateWindow)?;
        let from_utc = selected_date_boundary_utc(from, view_tzid)?;
        let to_utc = selected_date_boundary_utc(to_date_exclusive, view_tzid)?;
        Ok(format!(
            "/workspaces/{workspace_id}/calendar/events?from_date={}&to_date_exclusive={}&from_utc={}&to_utc={}&view_tzid={}",
            from.format(DATE_STORAGE_FMT),
            to_date_exclusive.format(DATE_STORAGE_FMT),
            from_utc.to_rfc3339_opts(SecondsFormat::Secs, true),
            to_utc.to_rfc3339_opts(SecondsFormat::Secs, true),
            encode_query_component(view_tzid),
        ))
    }

    /// The activity-spans read path for the workspace + event.
    pub fn activity_spans_path(workspace_id: &str, event_id: &str) -> String {
        format!("/workspaces/{workspace_id}/calendar/activity-spans?event_id={event_id}")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    /// Issue a read-only GET at `path`, mapping the response to a decoded `T` or the typed error model. A
    /// 404 / 501 (route unavailable / not implemented) maps to [`InteropError::EndpointUnavailable`] — the TYPED
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

        // THE TYPED BLOCKER (BROAD detection — RISK-3/MC-3): 404 (unavailable) OR 501 (not implemented)
        // both mean the attached backend cannot serve this /calendar/ read. Surface the typed blocker
        // DISTINCT from a generic Http error; never panic, never fabricate.
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::NOT_IMPLEMENTED
        {
            return Err(InteropError::EndpointUnavailable {
                probed_path: path.to_owned(),
            });
        }
        if !status.is_success() {
            return Err(InteropError::Http {
                status: status.as_u16(),
            });
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
    pub async fn open_or_create_daily_note(
        &self,
        date: NaiveDate,
    ) -> InteropResult<DailyNoteBinding> {
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

    // ── daily note <-> CalendarEvent window (live route with typed unavailable fallback) ────────────────

    /// Fetch the calendar events overlapping `[from, to]` (the contract's `events_for_range`) from the
    /// live route. An unavailable route returns [`InteropError::EndpointUnavailable`], never fabricated data.
    pub async fn events_for_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> InteropResult<Vec<CalendarEvent>> {
        if to < from {
            return Err(InteropError::InvalidDateWindow);
        }
        let path = Self::events_path(&self.workspace_id, from, to, &self.view_tzid)?;
        let mut events = self.get_json::<Vec<CalendarEvent>>(&path).await?;
        for event in &mut events {
            event.view_tzid.clone_from(&self.view_tzid);
        }
        Ok(events)
    }

    /// Resolve the CalendarEvent for a daily note's `date` (the contract's `resolve_event_for_daily_note`):
    /// fetch the events for that single day and pick the event whose window contains the date (or the
    /// all-day event for that date). Returns `Ok(None)` when the route exists but no event matches; returns
    /// [`InteropError::EndpointUnavailable`] when the route is unavailable so the panel renders the
    /// unavailable chip empty-state while the daily-note binding stays alive (AC-2 / AC-4). The day-window
    /// match uses selected-view IANA timezone and half-open overlap semantics (RISK-6/MC-6).
    pub async fn resolve_event_for_daily_note(
        &self,
        date: NaiveDate,
    ) -> InteropResult<Option<CalendarEvent>> {
        let events = self.events_for_range(date, date).await?;
        Ok(pick_event_for_date(&events, date))
    }

    // ── ActivitySpan correlation (READ-ONLY live route with typed unavailable fallback) ─────────────────

    /// Fetch the read-only ActivitySpan correlation for `event_id` (the contract's
    /// `activity_spans_for_event`): which documents were edited during the calendar block. An unavailable
    /// `/calendar/activity-spans` route returns [`InteropError::EndpointUnavailable`], so the panel shows the typed
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

fn encode_query_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(*byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Pick the CalendarEvent whose window contains `date` (the resolve-for-daily-note selection rule, RISK-6/
/// MC-6). Prefers an ALL-DAY event for the date over a timed one (the daily note is the day's anchor), then
/// the first timed event whose window overlaps the selected view-local date. Half-open local-day semantics
/// apply throughout. Pure (no IO) so it is unit-testable with fixture events.
pub fn pick_event_for_date(events: &[CalendarEvent], date: NaiveDate) -> Option<CalendarEvent> {
    // An all-day event for the date is the strongest match (the day's anchor).
    if let Some(all_day) = events
        .iter()
        .find(|e| e.is_all_day() && e.contains_date(date))
    {
        return Some(all_day.clone());
    }
    // Else the first timed event whose UTC interval overlaps the selected local-day window.
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
            let a = svc
                .open_or_create_daily_note(date)
                .await
                .expect("first open");
            let b = svc
                .open_or_create_daily_note(date)
                .await
                .expect("second open");
            (a, b)
        });
        // Same DocId both times (idempotent), and the doc id is the LINKED document (delegation result).
        assert_eq!(
            a.doc_id, b.doc_id,
            "AC-1: same date -> same DocId (idempotent)"
        );
        assert_eq!(a.doc_id, DocId("DOC-2026-06-21".to_owned()));
        assert_eq!(a.date, date);
        // The MT-019 backend was the creation path (delegation, RISK-1): it was called, not bypassed.
        assert_eq!(
            backend.opens.load(Ordering::SeqCst),
            2,
            "delegated to MT-019 both times"
        );
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
        let svc = CalendarInteropService::with_base_url(
            "http://unused",
            "WS-1",
            Arc::new(FailingBackend),
        );
        let err = rt()
            .block_on(async { svc.open_or_create_daily_note(d(2026, 6, 21)).await })
            .unwrap_err();
        assert!(
            matches!(err, InteropError::DailyNoteServiceError(_)),
            "RISK-1: MT-019 error propagates as DailyNoteServiceError, got {err:?}"
        );
    }

    fn timed_event(
        id: &str,
        start_utc: DateTime<Utc>,
        end_utc: DateTime<Utc>,
        view_tzid: &str,
    ) -> CalendarEvent {
        CalendarEvent {
            id: id.into(),
            title: id.into(),
            temporal: CalendarEventTemporal::Timed {
                start_utc,
                end_utc,
                start_local: start_utc.to_rfc3339(),
                end_local: end_utc.to_rfc3339(),
                tzid: "UTC".into(),
                was_floating: false,
                normalization_note: None,
            },
            daily_note_doc_id: None,
            view_tzid: view_tzid.into(),
        }
    }

    fn all_day_event(
        id: &str,
        start_date: NaiveDate,
        end_date_exclusive: NaiveDate,
    ) -> CalendarEvent {
        CalendarEvent {
            id: id.into(),
            title: id.into(),
            temporal: CalendarEventTemporal::AllDay {
                start_date,
                end_date_exclusive,
                tzid: "Europe/Brussels".into(),
            },
            daily_note_doc_id: None,
            view_tzid: "Europe/Brussels".into(),
        }
    }

    /// Timed membership uses the selected Calendar view timezone and half-open
    /// overlap semantics, including a Europe/Brussels near-midnight boundary.
    #[test]
    fn contains_date_uses_selected_view_timezone_and_half_open_bounds() {
        let timed = timed_event(
            "E-1",
            utc(2026, 6, 21, 22, 30),
            utc(2026, 6, 22, 2, 0),
            "Europe/Brussels",
        );
        assert!(!timed.contains_date(d(2026, 6, 21)));
        assert!(timed.contains_date(d(2026, 6, 22)));
        assert!(!timed.contains_date(d(2026, 6, 20)));
        assert!(!timed.contains_date(d(2026, 6, 23)));

        let midnight_end = timed_event(
            "E-2",
            utc(2026, 6, 21, 20, 0),
            utc(2026, 6, 21, 22, 0),
            "Europe/Brussels",
        );
        assert!(midnight_end.contains_date(d(2026, 6, 21)));
        assert!(!midnight_end.contains_date(d(2026, 6, 22)));

        let all_day = all_day_event("E-3", d(2026, 6, 21), d(2026, 6, 23));
        assert!(all_day.contains_date(d(2026, 6, 21)));
        assert!(all_day.contains_date(d(2026, 6, 22)));
        assert!(!all_day.contains_date(d(2026, 6, 23)));
    }

    #[test]
    fn brussels_selected_days_are_23_and_25_hours_across_dst() {
        let (spring_start, spring_end) =
            selected_date_window_utc(d(2026, 3, 29), "Europe/Brussels").unwrap();
        assert_eq!(spring_end - spring_start, chrono::Duration::hours(23));
        let (fall_start, fall_end) =
            selected_date_window_utc(d(2026, 10, 25), "Europe/Brussels").unwrap();
        assert_eq!(fall_end - fall_start, chrono::Duration::hours(25));
        assert!(matches!(
            selected_date_window_utc(d(2026, 6, 21), "Europe/Not-A-Zone"),
            Err(InteropError::InvalidTimezone(_))
        ));
    }

    #[test]
    fn temporal_summary_exposes_explicit_dst_overlap_outcome() {
        let mut event = timed_event(
            "E-overlap",
            utc(2026, 10, 25, 0, 30),
            utc(2026, 10, 25, 2, 30),
            "Europe/Brussels",
        );
        event.temporal = CalendarEventTemporal::Timed {
            start_utc: utc(2026, 10, 25, 0, 30),
            end_utc: utc(2026, 10, 25, 2, 30),
            start_local: "2026-10-25T02:30:00".into(),
            end_local: "2026-10-25T03:30:00".into(),
            tzid: "Europe/Brussels".into(),
            was_floating: false,
            normalization_note: Some(CalendarNormalizationNote {
                boundaries: vec![CalendarBoundaryNormalization {
                    boundary: "start".into(),
                    original_local: "2026-10-25T02:30:00".into(),
                    resolution: CalendarDstResolution::EarlierOffset,
                    resolved_utc: utc(2026, 10, 25, 0, 30),
                }],
            }),
        };
        let summary = event.temporal_summary();
        assert!(summary.contains("start 2026-10-25T02:30:00 => earlier offset"));
        assert!(summary.contains("2026-10-25T00:30:00+00:00"));
    }

    /// pick_event_for_date prefers an all-day event for the date, then the first timed match.
    #[test]
    fn pick_event_prefers_all_day_then_timed() {
        let timed = timed_event("T", utc(2026, 6, 21, 9, 0), utc(2026, 6, 21, 10, 0), "UTC");
        let all_day = all_day_event("A", d(2026, 6, 21), d(2026, 6, 22));
        let events = vec![timed.clone(), all_day.clone()];
        assert_eq!(
            pick_event_for_date(&events, d(2026, 6, 21)).unwrap().id,
            "A"
        );
        // Only the timed event present -> it is picked.
        assert_eq!(
            pick_event_for_date(&[timed], d(2026, 6, 21)).unwrap().id,
            "T"
        );
        // No match -> None.
        assert!(pick_event_for_date(&[all_day], d(2026, 6, 22)).is_none());
    }

    /// The read paths are the documented `/calendar/` route shapes (so the typed blocker names them).
    #[test]
    fn read_paths_are_documented_routes() {
        assert_eq!(
            CalendarInteropService::events_path(
                "WS-1",
                d(2026, 6, 21),
                d(2026, 6, 21),
                "Europe/Brussels"
            )
            .unwrap(),
            "/workspaces/WS-1/calendar/events?from_date=2026-06-21&to_date_exclusive=2026-06-22&from_utc=2026-06-20T22:00:00Z&to_utc=2026-06-21T22:00:00Z&view_tzid=Europe%2FBrussels"
        );
        assert!(
            CalendarInteropService::events_path(
                "WS-1",
                d(2026, 6, 21),
                d(2026, 6, 21),
                "Etc/GMT+5"
            )
            .unwrap()
            .ends_with("view_tzid=Etc%2FGMT%2B5"),
            "a plus sign must not be decoded as a query-space"
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

    #[test]
    fn retryability_is_limited_to_transient_idempotent_failures() {
        for retryable in [
            InteropError::Transport("timeout".into()),
            InteropError::DailyNoteTransient("HTTP 503".into()),
            InteropError::Http { status: 408 },
            InteropError::Http { status: 425 },
            InteropError::Http { status: 429 },
            InteropError::Http { status: 503 },
        ] {
            assert!(retryable.is_retryable(), "must retry {retryable:?}");
        }
        for terminal in [
            InteropError::EndpointUnavailable {
                probed_path: "/calendar/events".into(),
            },
            InteropError::Http { status: 400 },
            InteropError::Http { status: 409 },
            InteropError::Decode("invalid body".into()),
            InteropError::DailyNoteServiceError("invalid body".into()),
            InteropError::NotFound,
        ] {
            assert!(!terminal.is_retryable(), "must not retry {terminal:?}");
        }
    }

    /// A CalendarEvent body decodes from the live route's documented wire shape.
    #[test]
    fn calendar_event_decodes_from_wire() {
        let body = serde_json::json!({
            "id": "E-7",
            "title": "Sprint planning",
            "temporal": {
                "kind": "timed",
                "start_utc": "2026-06-21T09:00:00Z",
                "end_utc": "2026-06-21T10:00:00Z",
                "start_local": "2026-06-21T11:00:00",
                "end_local": "2026-06-21T12:00:00",
                "tzid": "Europe/Brussels",
                "was_floating": false,
                "normalization_note": null
            }
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
        assert!(span.ended_utc.is_some());
        assert_eq!(
            span.edited_doc_ids,
            vec![DocId("DOC-A".into()), DocId("DOC-B".into())]
        );

        let in_progress: ActivitySpan = serde_json::from_value(serde_json::json!({
            "span_id": "S-open",
            "calendar_event_id": "E-7",
            "started_utc": "2026-06-21T09:05:00Z",
            "ended_utc": null,
            "edited_doc_ids": []
        }))
        .expect("open span decodes without a fabricated end");
        assert!(in_progress.ended_utc.is_none());
    }

    #[test]
    fn legacy_incomplete_temporal_wire_remains_visible_and_typed() {
        let event: CalendarEvent = serde_json::from_value(serde_json::json!({
            "id": "E-legacy",
            "title": "Historic import",
            "temporal": {
                "kind": "legacy_incomplete",
                "start_utc": "2026-06-21T09:00:00Z",
                "end_utc": "2026-06-21T10:00:00Z",
                "tzid": "UTC",
                "all_day": false,
                "recovery": "reimport_from_calendar_source"
            }
        }))
        .expect("legacy row decodes as a typed recovery state");
        assert!(event.is_legacy_incomplete());
        assert!(event
            .temporal_summary()
            .contains("Legacy temporal data incomplete"));
        assert!(event
            .temporal_summary()
            .contains("reimport_from_calendar_source"));
    }
}
