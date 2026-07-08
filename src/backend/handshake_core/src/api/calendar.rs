//! WP-KERNEL-012 MT-045 (Calendar): the read-only calendar HTTP surface for the
//! native calendar view. Wires the EXISTING calendar window query
//! (`storage::calendar` / `query_calendar_events`) over PostgreSQL/EventLedger
//! authority — no new store, no SQLite.
//!
//! Routes (workspace-scoped, P1):
//!   * `GET /workspaces/:workspace_id/calendar/events?from=&to=`
//!       -> `Vec<CalendarEventWire{id,title,start_utc,end_utc,all_day,daily_note_doc_id}>`
//!
//! `from`/`to` accept either an RFC3339 datetime or a plain `YYYY-MM-DD` date;
//! a date lower bound is 00:00:00Z of that day and a date upper bound is
//! 00:00:00Z of the following day (inclusive end-of-day). An empty `source_ids`
//! window query returns every source in the workspace.
//!
//! DELIBERATELY NOT SERVED HERE: the `activity-spans` surface the frontend also
//! expects (per-calendar-event `edited_doc_ids` provenance) has NO backing store
//! today. The existing `flight_recorder::spans::ActivitySpan` is a swarm /
//! MT-iteration span (activity_kind = mt_iteration/model_swap/…) with no
//! `calendar_event_id` and no `edited_documents`. Serving that endpoint would
//! require a real capture-provenance store (a separate follow-up); it is left
//! unimplemented rather than fabricated with empty data.

use crate::models::ErrorResponse;
use crate::storage::{CalendarEventWindowQuery, StorageError};
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
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    from: String,
    to: String,
}

/// The wire shape the native calendar view decodes. `daily_note_doc_id` is
/// surfaced for shape parity but is always `null` today: no calendar-event ->
/// daily-note linkage exists in storage yet (a follow-up would add it).
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
    let wire = events
        .into_iter()
        .map(|event| CalendarEventWire {
            id: event.id,
            title: event.title,
            start_utc: event.start_ts_utc,
            end_utc: event.end_ts_utc,
            all_day: event.all_day,
            daily_note_doc_id: None,
        })
        .collect();
    Ok(Json(wire))
}
