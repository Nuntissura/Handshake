//! WP-KERNEL-012 MT-045 / MT-067 (Calendar): the calendar HTTP surface for the
//! native calendar view + the Editors<->Calendar interop edge. Wires the
//! EXISTING calendar window query (`storage::calendar` / `query_calendar_events`)
//! plus the MT-067 calendar activity-span store over single-store/EventLedger
//! authority — no SQLite.
//!
//! Both calendar-window queries and activity-span persistence use the shared
//! embedded SurrealDB authority through `CalendarActivityStore`.
//!
//! Routes (workspace-scoped, P1):
//!   * `PUT  /workspaces/:workspace_id/calendar/sources/:source_id`
//!       -> idempotently register typed source configuration in the embedded
//!       SurrealDB authority; path identities are authoritative and no generic
//!       query or storage handle is exposed
//!   * `GET  /workspaces/:workspace_id/calendar/events?from_date=&to_date_exclusive=&from_utc=&to_utc=&view_tzid=`
//!       -> typed timed/all-day `CalendarEventWire` values with lossless temporal intent
//!   * `GET  /workspaces/:workspace_id/calendar/activity-spans?event_id=`
//!       -> `Vec<ActivitySpanWire{span_id,calendar_event_id,started_utc,ended_utc,edited_doc_ids}>`
//!   * `POST /workspaces/:workspace_id/calendar/activity-spans`
//!       `{calendar_event_id, started_utc, ended_utc?, edited_doc_ids, span_id?}`
//!       -> the created span (how a native editor records edit activity during a
//!       calendar event).
//!
//! The local-date window is half-open: `[from_date, to_date_exclusive)` in the
//! selected IANA `view_tzid`. Callers also send the derived UTC half-open bounds;
//! the route rejects invalid zones or contradictory local/UTC windows. An empty
//! `source_ids` window query returns every source in the workspace.
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
//! WIRED. A calendar event is linked to the daily note date selected in the
//! requested Calendar view (clamped to the event's first overlapping local day)
//! when a daily journal LoomBlock exists for that date (discoverable via
//! `loom_blocks(content_type = 'journal', journal_date)` — the MT-019 / MT-257
//! daily journal). Absent a journal for the date, it stays `null`.

use crate::models::ErrorResponse;
use crate::storage::{
    calendar_date_start_utc, CalendarActivitySpan, CalendarActivityStore, CalendarEvent,
    CalendarEventWindowQuery, CalendarNormalizationNote, CalendarSource,
    CalendarSourceProviderType, CalendarSourceSyncState, CalendarSourceUpsert,
    CalendarSourceWritePolicy, NewCalendarActivitySpan, StorageError, WriteActorKind, WriteContext,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, put},
    Json, Router,
};
use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

const HSK_HEADER_ACTOR_KIND: &str = "x-hsk-actor-kind";
const HSK_HEADER_ACTOR_ID: &str = "x-hsk-actor-id";
const HSK_HEADER_JOB_ID: &str = "x-hsk-job-id";
const HSK_HEADER_WORKFLOW_ID: &str = "x-hsk-workflow-id";

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<T, ApiError>;

fn bad_request(code: &'static str) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: code }))
}

fn not_found(code: &'static str) -> ApiError {
    (StatusCode::NOT_FOUND, Json(ErrorResponse { error: code }))
}

fn conflict(code: &'static str) -> ApiError {
    (StatusCode::CONFLICT, Json(ErrorResponse { error: code }))
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
        StorageError::Conflict(code) => conflict(code),
        StorageError::Guard(code) => (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse { error: code }),
        ),
        StorageError::Validation(_) => bad_request("HSK-400-CALENDAR"),
        other => internal_error(other),
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_actor_kind(raw: Option<&str>) -> Result<WriteActorKind, StorageError> {
    match raw.map(str::trim).map(str::to_ascii_uppercase).as_deref() {
        None | Some("HUMAN") | Some("OPERATOR") => Ok(WriteActorKind::Human),
        Some("AI") => Ok(WriteActorKind::Ai),
        Some("SYSTEM") => Ok(WriteActorKind::System),
        Some(_) => Err(StorageError::Validation("invalid_actor_kind")),
    }
}

fn parse_uuid(raw: Option<&str>) -> Option<uuid::Uuid> {
    raw.and_then(|value| uuid::Uuid::parse_str(value).ok())
}

async fn write_context_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<WriteContext, StorageError> {
    let actor_kind = parse_actor_kind(header_str(headers, HSK_HEADER_ACTOR_KIND))?;
    let actor_id = header_str(headers, HSK_HEADER_ACTOR_ID).map(ToOwned::to_owned);
    match actor_kind {
        WriteActorKind::Human => Ok(WriteContext::human(actor_id)),
        WriteActorKind::System => Ok(WriteContext::system(actor_id)),
        WriteActorKind::Ai => {
            let job_id = parse_uuid(header_str(headers, HSK_HEADER_JOB_ID));
            let workflow_id = parse_uuid(header_str(headers, HSK_HEADER_WORKFLOW_ID));
            let (Some(job_id), Some(workflow_id)) = (job_id, workflow_id) else {
                return Ok(WriteContext::ai(actor_id, job_id, workflow_id));
            };
            let job = state.storage.get_ai_job(&job_id.to_string()).await?;
            if job.workflow_run_id != Some(workflow_id) {
                return Err(StorageError::Guard("HSK-403-SILENT-EDIT"));
            }
            Ok(WriteContext::ai(
                actor_id,
                Some(job_id),
                Some(workflow_id),
            ))
        }
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
            "/workspaces/:workspace_id/calendar/sources/:source_id",
            put(upsert_calendar_source),
        )
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

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UpsertCalendarSourceBody {
    display_name: String,
    provider_type: CalendarSourceProviderType,
    write_policy: CalendarSourceWritePolicy,
    default_tzid: String,
    #[serde(default)]
    auto_export: bool,
    credentials_ref: Option<String>,
    provider_calendar_id: Option<String>,
    capability_profile_id: Option<String>,
    #[serde(default = "empty_object")]
    config: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct CalendarSourceWire {
    id: String,
    workspace_id: String,
    display_name: String,
    provider_type: CalendarSourceProviderType,
    write_policy: CalendarSourceWritePolicy,
    default_tzid: String,
    auto_export: bool,
    provider_calendar_id: Option<String>,
    capability_profile_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<CalendarSource> for CalendarSourceWire {
    fn from(source: CalendarSource) -> Self {
        Self {
            id: source.id,
            workspace_id: source.workspace_id,
            display_name: source.display_name,
            provider_type: source.provider_type,
            write_policy: source.write_policy,
            default_tzid: source.default_tzid,
            auto_export: source.auto_export,
            provider_calendar_id: source.provider_calendar_id,
            capability_profile_id: source.capability_profile_id,
            created_at: source.created_at,
            updated_at: source.updated_at,
        }
    }
}

/// Register or update one source while preserving its workflow-owned sync state.
async fn upsert_calendar_source(
    State(state): State<AppState>,
    Path((workspace_id, source_id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(body): Json<UpsertCalendarSourceBody>,
) -> ApiResult<Json<CalendarSourceWire>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    if source_id.trim().is_empty() {
        return Err(bad_request("HSK-400-CALENDAR-SOURCE-ID"));
    }
    let sync_state = state
        .storage
        .get_calendar_source(&workspace_id, &source_id)
        .await
        .map_err(map_storage_error)?
        .map(|source| source.sync_state)
        .unwrap_or_else(CalendarSourceSyncState::default);
    let ctx = write_context_from_headers(&state, &headers)
        .await
        .map_err(map_storage_error)?;
    let source = state
        .storage
        .upsert_calendar_source(
            &ctx,
            CalendarSourceUpsert {
                id: source_id,
                workspace_id,
                display_name: body.display_name,
                provider_type: body.provider_type,
                write_policy: body.write_policy,
                default_tzid: body.default_tzid,
                auto_export: body.auto_export,
                credentials_ref: body.credentials_ref,
                provider_calendar_id: body.provider_calendar_id,
                capability_profile_id: body.capability_profile_id,
                config: body.config,
                sync_state,
            },
        )
        .await
        .map_err(map_storage_error)?;
    Ok(Json(source.into()))
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    from_date: NaiveDate,
    to_date_exclusive: NaiveDate,
    from_utc: DateTime<Utc>,
    to_utc: DateTime<Utc>,
    view_tzid: String,
}

/// The wire shape the native calendar view decodes. `daily_note_doc_id` is
/// WIRED (MT-067): it is the daily-note doc for the selected view-local date when
/// a daily journal LoomBlock exists for that date, else `null`.
#[derive(Debug, Serialize)]
struct CalendarEventWire {
    id: String,
    title: String,
    temporal: CalendarEventTemporalWire,
    daily_note_doc_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CalendarEventTemporalWire {
    Timed {
        start_utc: DateTime<Utc>,
        end_utc: DateTime<Utc>,
        start_local: String,
        end_local: String,
        tzid: String,
        was_floating: bool,
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
        recovery: &'static str,
    },
}

fn temporal_wire(event: &CalendarEvent) -> CalendarEventTemporalWire {
    if event.all_day {
        let (Some(start_date), Some(end_date_exclusive)) =
            (event.start_date, event.end_date_exclusive)
        else {
            return CalendarEventTemporalWire::LegacyIncomplete {
                start_utc: event.start_ts_utc,
                end_utc: event.end_ts_utc,
                tzid: event.tzid.clone(),
                all_day: true,
                recovery: "reimport_from_calendar_source",
            };
        };
        return CalendarEventTemporalWire::AllDay {
            start_date,
            end_date_exclusive,
            tzid: event.tzid.clone(),
        };
    }
    let (Some(start_local), Some(end_local)) = (event.start_local.clone(), event.end_local.clone())
    else {
        return CalendarEventTemporalWire::LegacyIncomplete {
            start_utc: event.start_ts_utc,
            end_utc: event.end_ts_utc,
            tzid: event.tzid.clone(),
            all_day: false,
            recovery: "reimport_from_calendar_source",
        };
    };
    CalendarEventTemporalWire::Timed {
        start_utc: event.start_ts_utc,
        end_utc: event.end_ts_utc,
        start_local,
        end_local,
        tzid: event.tzid.clone(),
        was_floating: event.was_floating,
        normalization_note: event.normalization_note.clone(),
    }
}

async fn list_calendar_events(
    State(state): State<AppState>,
    Path(workspace_id): Path<String>,
    Query(query): Query<EventsQuery>,
) -> ApiResult<Json<Vec<CalendarEventWire>>> {
    ensure_workspace_exists(&state, &workspace_id).await?;
    let view_tz: Tz = query
        .view_tzid
        .parse()
        .map_err(|_| bad_request("HSK-400-CALENDAR-TZID"))?;
    if query.to_date_exclusive <= query.from_date {
        return Err(bad_request("HSK-400-CALENDAR-WINDOW"));
    }
    let window_start_utc = calendar_date_start_utc(query.from_date, &query.view_tzid)
        .map_err(|_| bad_request("HSK-400-CALENDAR-WINDOW"))?;
    let window_end_utc = calendar_date_start_utc(query.to_date_exclusive, &query.view_tzid)
        .map_err(|_| bad_request("HSK-400-CALENDAR-WINDOW"))?;
    if query.from_utc != window_start_utc || query.to_utc != window_end_utc {
        return Err(bad_request("HSK-400-CALENDAR-WINDOW-CONTRADICTION"));
    }
    if window_end_utc <= window_start_utc {
        return Err(bad_request("HSK-400-CALENDAR-WINDOW"));
    }
    let events = state
        .storage
        .query_calendar_events(CalendarEventWindowQuery {
            workspace_id: workspace_id.clone(),
            query_start_date: query.from_date,
            query_end_date_exclusive: query.to_date_exclusive,
            window_start_utc,
            window_end_utc,
            // Empty = every calendar source in the workspace.
            source_ids: Vec::new(),
        })
        .await
        .map_err(map_storage_error)?;

    // MT-067 daily-note linkage: for each event, look up the daily-note doc for
    // the requested view-local date, clamped to the event's first overlapping
    // local day. Lookups are cached by date so repeated events query once.
    let activity_store = CalendarActivityStore::new(state.surreal.clone());
    let mut daily_note_by_date: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    let mut wire = Vec::with_capacity(events.len());
    for event in events {
        let event_start_date = if event.all_day {
            event.start_date.unwrap_or(query.from_date)
        } else if event.start_local.is_some() && event.end_local.is_some() {
            event.start_ts_utc.with_timezone(&view_tz).date_naive()
        } else {
            // Legacy rows do not have sufficient temporal intent for a local
            // reconstruction. Bind to the explicitly requested canonical day.
            query.from_date
        };
        let journal_date = std::cmp::max(event_start_date, query.from_date)
            .format("%Y-%m-%d")
            .to_string();
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
        let temporal = temporal_wire(&event);
        wire.push(CalendarEventWire {
            id: event.id,
            title: event.title,
            temporal,
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
    ended_utc: Option<DateTime<Utc>>,
    edited_doc_ids: Vec<String>,
}

/// Project a stored span without fabricating an end for in-progress work.
fn span_to_wire(span: CalendarActivitySpan) -> ActivitySpanWire {
    ActivitySpanWire {
        span_id: span.span_id,
        calendar_event_id: Some(span.calendar_event_id),
        ended_utc: span.ended_utc,
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
    let store = CalendarActivityStore::new(state.surreal.clone());
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
    let store = CalendarActivityStore::new(state.surreal.clone());
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
