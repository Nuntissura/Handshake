//! WP-KERNEL-012 MT-045 / MT-067 (Calendar): the calendar HTTP surface for the
//! native calendar view + the Editors<->Calendar interop edge. Wires the
//! EXISTING calendar window query (`storage::calendar` / `query_calendar_events`)
//! plus the MT-067 calendar activity-span store over PostgreSQL/EventLedger
//! authority — no SQLite.
//!
//! Routes (workspace-scoped, P1):
//!   * `GET  /workspaces/:workspace_id/calendar/events?from=&to=`
//!       -> `Vec<CalendarEventWire{id,title,start_utc,end_utc,all_day,daily_note_doc_id}>`
//!   * `GET  /workspaces/:workspace_id/calendar/activity-spans?event_id=`
//!       -> `Vec<ActivitySpanWire{span_id,calendar_event_id,started_utc,ended_utc,edited_doc_ids}>`
//!   * `POST /workspaces/:workspace_id/calendar/activity-spans`
//!       `{calendar_event_id, started_utc, ended_utc?, edited_doc_ids, span_id?}`
//!       -> the created span (how a native editor records edit activity during a
//!       calendar event).
//!
//! `from`/`to` accept either an RFC3339 datetime or a plain `YYYY-MM-DD` date;
//! a date lower bound is 00:00:00Z of that day and a date upper bound is
//! 00:00:00Z of the following day (inclusive end-of-day). An empty `source_ids`
//! window query returns every source in the workspace.
//!
//! ACTIVITY SPANS (MT-067): a calendar activity span is the native editor's own
//! edit-provenance for a calendar block — which documents were edited during
//! that event. It is a DISTINCT concept from `flight_recorder::spans` (table
//! `kernel_activity_span`, a swarm / mt-iteration span with no calendar linkage),
//! so it is backed by its OWN store (`storage::calendar_activity`, table
//! `calendar_activity_spans`, migration 0340), never reusing the flight-recorder
//! span table.
//!
//! DAILY-NOTE LINKAGE (MT-067): `daily_note_doc_id` on the events response is
//! WIRED. A calendar event is linked to the daily note of its START UTC date
//! when a daily journal LoomBlock exists for that date (discoverable via
//! `loom_blocks(content_type = 'journal', journal_date)` — the MT-019 / MT-257
//! daily journal). Absent a journal for the date, it stays `null`.

use crate::models::ErrorResponse;
use crate::storage::{
    CalendarActivitySpan, CalendarActivityStore, CalendarEventWindowQuery,
    NewCalendarActivitySpan, StorageError,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

fn bad_request(code: &'static str) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: code }))
}

fn not_found(code: &'static str) -> ApiError {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}

fn internal_error(err: impl std::fmt::Display) -> ApiError {
    tracing::error!(target: "handshake_core::calendar_api", error = %err, "calendar_api_error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "HSK-500-CALENDAR",
        }),
    )
}

fn map_storage_error(err: StorageError) -> ApiError {
    match err {
        StorageError::NotFound(code) => not_found(code),
        StorageError::Validation(_) => bad_request("HSK-400-CALENDAR"),
        other => internal_error(other),
    }
}

async fn ensure_workspace_exists(state: &AppState, workspace_id: &str) -> ApiResult<()> {
    match state.storage.get_workspace(workspace_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("workspace_not_found")),
        Err(err) => Err(map_storage_error(err)),
    }
}

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route(
            "/workspaces/:workspace_id/calendar/events",
            get(list_calendar_events),
        )
        .route(
            "/workspaces/:workspace_id/calendar/activity-spans",
            get(list_activity_spans).post(create_activity_span),
        )
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    from: String,
    to: String,
}

/// The wire shape the native calendar view decodes. `daily_note_doc_id` is
/// WIRED (MT-067): it is the daily-note doc for the event's START UTC date when
/// a daily journal LoomBlock exists for that date, else `null`.
#[derive(Debug, Serialize)]
struct CalendarEventWire {
    id: String,
    title: String,
    start_utc: DateTime<Utc>,
    end_utc: DateTime<Utc>,
    all_day: bool,
    daily_note_doc_id: Option<String>,
}

/// Parse a single window bound. Accepts RFC3339 (`2026-07-01T09:00:00Z`) or a
/// plain date (`2026-07-01`). For a plain date the `upper` bound rolls to the
/// next day at 00:00:00Z so the whole `to` day is included.
fn parse_bound(value: &str, upper: bool) -> ApiResult<DateTime<Utc>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(bad_request("HSK-400-CALENDAR-WINDOW"));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let date = if upper {
            date.succ_opt()
                .ok_or_else(|| bad_request("HSK-400-CALENDAR-WINDOW"))?
        } else {
            date
        };
        let naive = date.and_time(NaiveTime::MIN);
        return Ok(Utc.from_utc_datetime(&naive));
    }
    Err(bad_request("HSK-400-CALENDAR-WINDOW"))
}

async fn list_calendar_events(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> ApiResult<Json<Vec<CalendarEventWire>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let window_start_utc = parse_bound(&query.from, false)?;
    let window_end_utc = parse_bound(&query.to, true)?;
    if window_end_utc <= window_start_utc {
        return Err(bad_request("HSK-400-CALENDAR-WINDOW"));
    }
    let events = state
        .storage
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace_id.clone(),
            window_start_utc,
            window_end_utc,
            // Empty = every calendar source in the workspace.
            source_ids: Vec::new(),
        })
        .await
        .map_err(map_storage_error)?;

    // MT-067 daily-note linkage: for each event, look up the daily-note doc for
    // its START UTC date (a daily journal LoomBlock keyed by that date). The
    // lookups are cached by date so repeated events on the same day query once.
    let activity_store = CalendarActivityStore::new(state.postgres_pool.clone());
    let mut daily_note_by_date: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    let mut wire = Vec::with_capacity(events.len());
    for event in events {
        let journal_date = event.start_ts_utc.date_naive().format("%Y-%m-%d").to_string();
        let daily_note_doc_id = match daily_note_by_date.get(&journal_date) {
            Some(cached) => cached.clone(),
            None => {
                let resolved = activity_store
                    .find_daily_note_doc_id_for_date(&workspace_id, &journal_date)
                    .await
                    .map_err(map_storage_error)?;
                daily_note_by_date.insert(journal_date, resolved.clone());
                resolved
            }
        };
        wire.push(CalendarEventWire {
            id: event.id,
            title: event.title,
            start_utc: event.start_ts_utc,
            end_utc: event.end_ts_utc,
            all_day: event.all_day,
            daily_note_doc_id,
        });
    }
    Ok(Json(wire))
}

// ---------------------------------------------------------------------------
// Activity spans (MT-067) — the native editor's edit-provenance for a calendar
// block. GET reads the correlation; POST records a new span.
// ---------------------------------------------------------------------------

/// The wire shape the native `calendar_interop::ActivitySpan` decoder expects.
#[derive(Debug, Serialize)]
struct ActivitySpanWire {
    span_id: String,
    calendar_event_id: Option<String>,
    started_utc: DateTime<Utc>,
    ended_utc: DateTime<Utc>,
    edited_doc_ids: Vec<String>,
}

/// Project a stored span to the wire shape. The shipped native decoder
/// (`calendar_interop::ActivitySpan`) requires a NON-NULL `ended_utc`, so an
/// open/in-progress span (`ended_utc = NULL`) is projected as a zero-duration
/// span at its start until it closes — the wire always decodes and the stored
/// value is preserved once the span ends.
fn span_to_wire(span: CalendarActivitySpan) -> ActivitySpanWire {
    ActivitySpanWire {
        span_id: span.span_id,
        calendar_event_id: Some(span.calendar_event_id),
        ended_utc: span.ended_utc.unwrap_or(span.started_utc),
        started_utc: span.started_utc,
        edited_doc_ids: span.edited_doc_ids,
    }
}

#[derive(Debug, Deserialize)]
struct ActivitySpansQuery {
    event_id: String,
}

/// `GET /workspaces/:ws/calendar/activity-spans?event_id=` — the read-only
/// activity-span correlation for a calendar event.
async fn list_activity_spans(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<ActivitySpansQuery>,
) -> ApiResult<Json<Vec<ActivitySpanWire>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let event_id = query.event_id.trim();
    if event_id.is_empty() {
        return Err(bad_request("HSK-400-CALENDAR-EVENT-ID"));
    }
    let store = CalendarActivityStore::new(state.postgres_pool.clone());
    let spans = store
        .query_activity_spans_by_event(&workspace_id, event_id)
        .await
        .map_err(map_storage_error)?;
    Ok(Json(spans.into_iter().map(span_to_wire).collect()))
}

/// Body for `POST /workspaces/:ws/calendar/activity-spans`.
#[derive(Debug, Deserialize)]
struct CreateActivitySpanBody {
    calendar_event_id: String,
    started_utc: DateTime<Utc>,
    #[serde(default)]
    ended_utc: Option<DateTime<Utc>>,
    #[serde(default)]
    edited_doc_ids: Vec<String>,
    /// Optional caller-supplied span id; a server id is minted when absent.
    #[serde(default)]
    span_id: Option<String>,
}

/// `POST /workspaces/:ws/calendar/activity-spans` — record a native-editor edit
/// activity span for a calendar event.
async fn create_activity_span(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Json(body): Json<CreateActivitySpanBody>,
) -> ApiResult<(StatusCode, Json<ActivitySpanWire>)> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let calendar_event_id = body.calendar_event_id.trim().to_string();
    if calendar_event_id.is_empty() {
        return Err(bad_request("HSK-400-CALENDAR-EVENT-ID"));
    }
    let span_id = body
        .span_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("CAS-{}", uuid::Uuid::now_v7().simple()));
    let store = CalendarActivityStore::new(state.postgres_pool.clone());
    let span = store
        .upsert_activity_span(NewCalendarActivitySpan {
            span_id,
            workspace_id: workspace_id.clone(),
            calendar_event_id,
            started_utc: body.started_utc,
            ended_utc: body.ended_utc,
            edited_doc_ids: body.edited_doc_ids,
        })
        .await
        .map_err(map_storage_error)?;
    Ok((StatusCode::CREATED, Json(span_to_wire(span))))
}
