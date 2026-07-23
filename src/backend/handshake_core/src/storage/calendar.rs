use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarSourceProviderType {
    Local,
    Google,
    Ics,
    Caldav,
    Other,
}

impl CalendarSourceProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarSourceProviderType::Local => "local",
            CalendarSourceProviderType::Google => "google",
            CalendarSourceProviderType::Ics => "ics",
            CalendarSourceProviderType::Caldav => "caldav",
            CalendarSourceProviderType::Other => "other",
        }
    }
}

impl FromStr for CalendarSourceProviderType {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "local" => Ok(CalendarSourceProviderType::Local),
            "google" => Ok(CalendarSourceProviderType::Google),
            "ics" => Ok(CalendarSourceProviderType::Ics),
            "caldav" => Ok(CalendarSourceProviderType::Caldav),
            "other" => Ok(CalendarSourceProviderType::Other),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid calendar source provider_type",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarSourceWritePolicy {
    ReadOnlyImport,
    TwoWayMirror,
    PublishFromHandshake,
}

impl CalendarSourceWritePolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarSourceWritePolicy::ReadOnlyImport => "read_only_import",
            CalendarSourceWritePolicy::TwoWayMirror => "two_way_mirror",
            CalendarSourceWritePolicy::PublishFromHandshake => "publish_from_handshake",
        }
    }
}

impl FromStr for CalendarSourceWritePolicy {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "read_only_import" => Ok(CalendarSourceWritePolicy::ReadOnlyImport),
            "two_way_mirror" => Ok(CalendarSourceWritePolicy::TwoWayMirror),
            "publish_from_handshake" => Ok(CalendarSourceWritePolicy::PublishFromHandshake),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid calendar source write_policy",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CalendarSyncStateStage {
    Idle,
    Pulling,
    Applying,
    Pushing,
    Conflicted,
    ErrorBackoff,
    Disabled,
}

impl CalendarSyncStateStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarSyncStateStage::Idle => "IDLE",
            CalendarSyncStateStage::Pulling => "PULLING",
            CalendarSyncStateStage::Applying => "APPLYING",
            CalendarSyncStateStage::Pushing => "PUSHING",
            CalendarSyncStateStage::Conflicted => "CONFLICTED",
            CalendarSyncStateStage::ErrorBackoff => "ERROR_BACKOFF",
            CalendarSyncStateStage::Disabled => "DISABLED",
        }
    }
}

impl FromStr for CalendarSyncStateStage {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "IDLE" => Ok(CalendarSyncStateStage::Idle),
            "PULLING" => Ok(CalendarSyncStateStage::Pulling),
            "APPLYING" => Ok(CalendarSyncStateStage::Applying),
            "PUSHING" => Ok(CalendarSyncStateStage::Pushing),
            "CONFLICTED" => Ok(CalendarSyncStateStage::Conflicted),
            "ERROR_BACKOFF" => Ok(CalendarSyncStateStage::ErrorBackoff),
            "DISABLED" => Ok(CalendarSyncStateStage::Disabled),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid calendar source sync state",
            )),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CalendarSourceSyncState {
    pub state: Option<CalendarSyncStateStage>,
    pub sync_token: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub last_full_sync_at: Option<DateTime<Utc>>,
    pub last_ok_at: Option<DateTime<Utc>>,
    pub last_pull_at: Option<DateTime<Utc>>,
    pub last_push_at: Option<DateTime<Utc>>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
    pub last_error: Option<String>,
    pub backoff_until: Option<DateTime<Utc>>,
    pub consecutive_failures: Option<i64>,
    pub last_remote_watermark: Option<String>,
    pub last_local_applied_rev: Option<i64>,
}

fn default_calendar_event_status() -> CalendarEventStatus {
    CalendarEventStatus::Confirmed
}

fn default_calendar_event_visibility() -> CalendarEventVisibility {
    CalendarEventVisibility::Private
}

fn default_calendar_event_export_mode() -> CalendarEventExportMode {
    CalendarEventExportMode::LocalOnly
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarSyncInput {
    pub workspace_id: String,
    pub source_id: String,
    #[serde(default)]
    pub provider_events: Vec<CalendarSyncEventUpsert>,
    #[serde(default)]
    pub mutations: Vec<CalendarMutation>,
    pub next_sync_token: Option<String>,
    pub remote_watermark: Option<String>,
    #[serde(default)]
    pub full_sync: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarSyncEventUpsert {
    pub id: Option<String>,
    pub external_id: Option<String>,
    pub external_etag: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_ts_utc: DateTime<Utc>,
    pub end_ts_utc: DateTime<Utc>,
    pub start_local: Option<String>,
    pub end_local: Option<String>,
    #[serde(default)]
    pub tzid: String,
    #[serde(default)]
    pub all_day: bool,
    /// Canonical date-only lower bound for an all-day event.
    pub start_date: Option<NaiveDate>,
    /// Canonical date-only exclusive upper bound for an all-day event.
    pub end_date_exclusive: Option<NaiveDate>,
    #[serde(default)]
    pub was_floating: bool,
    /// Explicit, persisted explanation when a DST overlap selected one of two
    /// valid instants. Non-existent local times are rejected at ingest.
    pub normalization_note: Option<CalendarNormalizationNote>,
    #[serde(default = "default_calendar_event_status")]
    pub status: CalendarEventStatus,
    #[serde(default = "default_calendar_event_visibility")]
    pub visibility: CalendarEventVisibility,
    #[serde(default = "default_calendar_event_export_mode")]
    pub export_mode: CalendarEventExportMode,
    pub rrule: Option<String>,
    #[serde(default)]
    pub rdate: Vec<String>,
    #[serde(default)]
    pub exdate: Vec<String>,
    #[serde(default)]
    pub is_recurring: bool,
    pub series_id: Option<String>,
    pub instance_key: Option<String>,
    #[serde(default)]
    pub is_override: bool,
    pub source_last_seen_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub attendees: Value,
    #[serde(default)]
    pub links: Value,
    pub provider_payload: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarDstResolution {
    EarlierOffset,
    LaterOffset,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarBoundaryNormalization {
    pub boundary: String,
    pub original_local: String,
    pub resolution: CalendarDstResolution,
    pub resolved_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CalendarNormalizationNote {
    #[serde(default)]
    pub boundaries: Vec<CalendarBoundaryNormalization>,
}

fn parse_calendar_local(value: &str) -> Result<NaiveDateTime, crate::storage::StorageError> {
    ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"]
        .iter()
        .find_map(|format| NaiveDateTime::parse_from_str(value, format).ok())
        .ok_or(crate::storage::StorageError::Validation(
            "calendar local wall time must be an offset-free ISO local datetime",
        ))
}

/// Resolve the first real instant belonging to a local calendar date. This is
/// used only to derive query/index instants for date-only all-day events. It
/// does not rewrite the authoritative date boundaries.
pub fn calendar_date_start_utc(
    date: NaiveDate,
    tzid: &str,
) -> Result<DateTime<Utc>, crate::storage::StorageError> {
    let tz: Tz = tzid.parse().map_err(|_| {
        crate::storage::StorageError::Validation("calendar tzid must be a valid IANA timezone")
    })?;
    let midnight = date
        .and_hms_opt(0, 0, 0)
        .ok_or(crate::storage::StorageError::Validation(
            "calendar date boundary is invalid",
        ))?;
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
    Err(crate::storage::StorageError::Validation(
        "calendar date has no representable instant in tzdb",
    ))
}

fn resolve_timed_boundary(
    boundary: &str,
    local_text: &str,
    supplied_utc: DateTime<Utc>,
    tz: Tz,
) -> Result<Option<CalendarBoundaryNormalization>, crate::storage::StorageError> {
    let local = parse_calendar_local(local_text)?;
    match tz.from_local_datetime(&local) {
        LocalResult::Single(resolved) if resolved.with_timezone(&Utc) == supplied_utc => Ok(None),
        LocalResult::Single(_) => Err(crate::storage::StorageError::Validation(
            "calendar UTC instant contradicts its local wall time and tzid",
        )),
        LocalResult::None => Err(crate::storage::StorageError::Validation(
            "calendar local wall time does not exist in tzdb (DST gap)",
        )),
        LocalResult::Ambiguous(first, second) => {
            let mut candidates = [first.with_timezone(&Utc), second.with_timezone(&Utc)];
            candidates.sort();
            let resolution = if supplied_utc == candidates[0] {
                CalendarDstResolution::EarlierOffset
            } else if supplied_utc == candidates[1] {
                CalendarDstResolution::LaterOffset
            } else {
                return Err(crate::storage::StorageError::Validation(
                    "calendar UTC instant contradicts both DST-overlap candidates",
                ));
            };
            Ok(Some(CalendarBoundaryNormalization {
                boundary: boundary.to_owned(),
                original_local: local_text.to_owned(),
                resolution,
                resolved_utc: supplied_utc,
            }))
        }
    }
}

/// Normalize and validate one Calendar Workflow ingest event against the IANA
/// tzdb. Floating events bind to the source default timezone while preserving
/// `was_floating`; DST gaps fail explicitly and DST overlaps persist which
/// supplied UTC candidate was selected.
pub fn normalize_calendar_sync_event(
    mut event: CalendarSyncEventUpsert,
    source_default_tzid: &str,
) -> Result<CalendarSyncEventUpsert, crate::storage::StorageError> {
    let source_default_tz: Tz = source_default_tzid.parse().map_err(|_| {
        crate::storage::StorageError::Validation(
            "calendar source default_tzid must be a valid IANA timezone",
        )
    })?;
    if event.end_ts_utc <= event.start_ts_utc {
        return Err(crate::storage::StorageError::Validation(
            "calendar event end_ts_utc must be after start_ts_utc",
        ));
    }
    if event.tzid.trim().is_empty() {
        if event.was_floating {
            event.tzid = source_default_tz.name().to_owned();
        } else {
            return Err(crate::storage::StorageError::Validation(
                "calendar timed event tzid is required",
            ));
        }
    }
    if event.was_floating && event.tzid != source_default_tz.name() {
        return Err(crate::storage::StorageError::Validation(
            "floating calendar event must bind to the source default timezone",
        ));
    }
    let tz: Tz = event.tzid.parse().map_err(|_| {
        crate::storage::StorageError::Validation("calendar tzid must be a valid IANA timezone")
    })?;

    if event.all_day {
        let (Some(start_date), Some(end_date_exclusive)) =
            (event.start_date, event.end_date_exclusive)
        else {
            return Err(crate::storage::StorageError::Validation(
                "all-day calendar event requires start_date and end_date_exclusive",
            ));
        };
        if end_date_exclusive <= start_date {
            return Err(crate::storage::StorageError::Validation(
                "all-day calendar end_date_exclusive must be after start_date",
            ));
        }
        if event.start_local.is_some() || event.end_local.is_some() || event.was_floating {
            return Err(crate::storage::StorageError::Validation(
                "all-day calendar event cannot carry timed local/floating fields",
            ));
        }
        let derived_start = calendar_date_start_utc(start_date, &event.tzid)?;
        let derived_end = calendar_date_start_utc(end_date_exclusive, &event.tzid)?;
        if event.start_ts_utc != derived_start || event.end_ts_utc != derived_end {
            return Err(crate::storage::StorageError::Validation(
                "all-day calendar UTC index instants contradict date-only boundaries",
            ));
        }
        if event.normalization_note.is_some() {
            return Err(crate::storage::StorageError::Validation(
                "all-day calendar event cannot carry timed DST normalization",
            ));
        }
        return Ok(event);
    }

    if event.start_date.is_some() || event.end_date_exclusive.is_some() {
        return Err(crate::storage::StorageError::Validation(
            "timed calendar event cannot carry all-day date boundaries",
        ));
    }
    let start_local =
        event
            .start_local
            .as_deref()
            .ok_or(crate::storage::StorageError::Validation(
                "timed calendar event requires start_local",
            ))?;
    let end_local = event
        .end_local
        .as_deref()
        .ok_or(crate::storage::StorageError::Validation(
            "timed calendar event requires end_local",
        ))?;
    let mut boundaries = Vec::new();
    if let Some(note) = resolve_timed_boundary("start", start_local, event.start_ts_utc, tz)? {
        boundaries.push(note);
    }
    if let Some(note) = resolve_timed_boundary("end", end_local, event.end_ts_utc, tz)? {
        boundaries.push(note);
    }
    let derived_note = (!boundaries.is_empty()).then_some(CalendarNormalizationNote { boundaries });
    if event.normalization_note.is_some() && event.normalization_note != derived_note {
        return Err(crate::storage::StorageError::Validation(
            "calendar normalization note contradicts tzdb resolution",
        ));
    }
    event.normalization_note = derived_note;
    Ok(event)
}

/// Central storage-boundary validation for source configuration. Both the
/// Calendar Workflow and direct storage callers use this contract.
pub fn validate_calendar_source_contract(
    source: &CalendarSourceUpsert,
) -> Result<(), crate::storage::StorageError> {
    if source.id.trim().is_empty() {
        return Err(crate::storage::StorageError::Validation(
            "calendar source id is required",
        ));
    }
    if source.workspace_id.trim().is_empty() {
        return Err(crate::storage::StorageError::Validation(
            "calendar source workspace_id is required",
        ));
    }
    if source.display_name.trim().is_empty() {
        return Err(crate::storage::StorageError::Validation(
            "calendar source display_name is required",
        ));
    }
    source.default_tzid.parse::<Tz>().map_err(|_| {
        crate::storage::StorageError::Validation(
            "calendar source default_tzid must be a valid IANA timezone",
        )
    })?;
    Ok(())
}

/// Re-validate a normalized event at the storage boundary. This prevents a
/// caller from bypassing the workflow normalizer or persisting contradictory
/// local/UTC/date values through a direct upsert.
pub fn validate_calendar_event_contract(
    event: &CalendarEventUpsert,
    source_default_tzid: &str,
) -> Result<(), crate::storage::StorageError> {
    if event.id.trim().is_empty() {
        return Err(crate::storage::StorageError::Validation(
            "calendar event id is required",
        ));
    }
    if event.workspace_id.trim().is_empty() || event.source_id.trim().is_empty() {
        return Err(crate::storage::StorageError::Validation(
            "calendar event workspace_id and source_id are required",
        ));
    }
    if event.title.trim().is_empty() {
        return Err(crate::storage::StorageError::Validation(
            "calendar event title is required",
        ));
    }
    if event
        .external_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(crate::storage::StorageError::Validation(
            "calendar event external_id cannot be blank",
        ));
    }

    let normalized = normalize_calendar_sync_event(
        CalendarSyncEventUpsert {
            id: Some(event.id.clone()),
            external_id: event.external_id.clone(),
            external_etag: event.external_etag.clone(),
            title: event.title.clone(),
            description: event.description.clone(),
            location: event.location.clone(),
            start_ts_utc: event.start_ts_utc,
            end_ts_utc: event.end_ts_utc,
            start_local: event.start_local.clone(),
            end_local: event.end_local.clone(),
            tzid: event.tzid.clone(),
            all_day: event.all_day,
            start_date: event.start_date,
            end_date_exclusive: event.end_date_exclusive,
            was_floating: event.was_floating,
            normalization_note: event.normalization_note.clone(),
            status: event.status.clone(),
            visibility: event.visibility.clone(),
            export_mode: event.export_mode.clone(),
            rrule: event.rrule.clone(),
            rdate: event.rdate.clone(),
            exdate: event.exdate.clone(),
            is_recurring: event.is_recurring,
            series_id: event.series_id.clone(),
            instance_key: event.instance_key.clone(),
            is_override: event.is_override,
            source_last_seen_at: event.source_last_seen_at,
            attendees: event.attendees.clone(),
            links: event.links.clone(),
            provider_payload: event.provider_payload.clone(),
        },
        source_default_tzid,
    )?;

    if normalized.tzid != event.tzid
        || normalized.start_ts_utc != event.start_ts_utc
        || normalized.end_ts_utc != event.end_ts_utc
        || normalized.normalization_note != event.normalization_note
    {
        return Err(crate::storage::StorageError::Validation(
            "calendar event is not in canonical normalized form",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarMutationAction {
    UpsertEvent,
    DeleteSourceData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarMutation {
    pub mutation_id: Option<String>,
    pub action: CalendarMutationAction,
    pub event: Option<CalendarSyncEventUpsert>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarSource {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub provider_type: CalendarSourceProviderType,
    pub write_policy: CalendarSourceWritePolicy,
    pub default_tzid: String,
    pub auto_export: bool,
    pub credentials_ref: Option<String>,
    pub provider_calendar_id: Option<String>,
    pub capability_profile_id: Option<String>,
    pub config: Value,
    pub sync_state: CalendarSourceSyncState,
    pub last_job_id: Option<String>,
    pub last_workflow_id: Option<String>,
    pub last_actor_id: Option<String>,
    pub edit_event_id: String,
    pub last_actor_kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct CalendarSourceUpsert {
    pub id: String,
    pub workspace_id: String,
    pub display_name: String,
    pub provider_type: CalendarSourceProviderType,
    pub write_policy: CalendarSourceWritePolicy,
    pub default_tzid: String,
    pub auto_export: bool,
    pub credentials_ref: Option<String>,
    pub provider_calendar_id: Option<String>,
    pub capability_profile_id: Option<String>,
    pub config: Value,
    pub sync_state: CalendarSourceSyncState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarEventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

impl CalendarEventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarEventStatus::Confirmed => "confirmed",
            CalendarEventStatus::Tentative => "tentative",
            CalendarEventStatus::Cancelled => "cancelled",
        }
    }
}

impl FromStr for CalendarEventStatus {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "confirmed" => Ok(CalendarEventStatus::Confirmed),
            "tentative" => Ok(CalendarEventStatus::Tentative),
            "cancelled" => Ok(CalendarEventStatus::Cancelled),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid calendar event status",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarEventVisibility {
    Public,
    Private,
    BusyOnly,
}

impl CalendarEventVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarEventVisibility::Public => "public",
            CalendarEventVisibility::Private => "private",
            CalendarEventVisibility::BusyOnly => "busy_only",
        }
    }
}

impl FromStr for CalendarEventVisibility {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "public" => Ok(CalendarEventVisibility::Public),
            "private" => Ok(CalendarEventVisibility::Private),
            "busy_only" => Ok(CalendarEventVisibility::BusyOnly),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid calendar event visibility",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarEventExportMode {
    LocalOnly,
    BusyOnly,
    FullExport,
}

impl CalendarEventExportMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarEventExportMode::LocalOnly => "local_only",
            CalendarEventExportMode::BusyOnly => "busy_only",
            CalendarEventExportMode::FullExport => "full_export",
        }
    }
}

impl FromStr for CalendarEventExportMode {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "local_only" => Ok(CalendarEventExportMode::LocalOnly),
            "busy_only" => Ok(CalendarEventExportMode::BusyOnly),
            "full_export" => Ok(CalendarEventExportMode::FullExport),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid calendar event export_mode",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub external_id: Option<String>,
    pub external_etag: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_ts_utc: DateTime<Utc>,
    pub end_ts_utc: DateTime<Utc>,
    pub start_local: Option<String>,
    pub end_local: Option<String>,
    pub tzid: String,
    pub all_day: bool,
    pub start_date: Option<NaiveDate>,
    pub end_date_exclusive: Option<NaiveDate>,
    pub was_floating: bool,
    pub normalization_note: Option<CalendarNormalizationNote>,
    pub status: CalendarEventStatus,
    pub visibility: CalendarEventVisibility,
    pub export_mode: CalendarEventExportMode,
    pub rrule: Option<String>,
    pub rdate: Vec<String>,
    pub exdate: Vec<String>,
    pub is_recurring: bool,
    pub series_id: Option<String>,
    pub instance_key: Option<String>,
    pub is_override: bool,
    pub source_last_seen_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub attendees: Value,
    pub links: Value,
    pub provider_payload: Option<Value>,
    pub last_job_id: Option<String>,
    pub last_workflow_id: Option<String>,
    pub last_actor_id: Option<String>,
    pub edit_event_id: String,
    pub last_actor_kind: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct CalendarEventUpsert {
    pub id: String,
    pub workspace_id: String,
    pub source_id: String,
    pub external_id: Option<String>,
    pub external_etag: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_ts_utc: DateTime<Utc>,
    pub end_ts_utc: DateTime<Utc>,
    pub start_local: Option<String>,
    pub end_local: Option<String>,
    pub tzid: String,
    pub all_day: bool,
    pub start_date: Option<NaiveDate>,
    pub end_date_exclusive: Option<NaiveDate>,
    pub was_floating: bool,
    pub normalization_note: Option<CalendarNormalizationNote>,
    pub status: CalendarEventStatus,
    pub visibility: CalendarEventVisibility,
    pub export_mode: CalendarEventExportMode,
    pub rrule: Option<String>,
    pub rdate: Vec<String>,
    pub exdate: Vec<String>,
    pub is_recurring: bool,
    pub series_id: Option<String>,
    pub instance_key: Option<String>,
    pub is_override: bool,
    pub source_last_seen_at: Option<DateTime<Utc>>,
    pub attendees: Value,
    pub links: Value,
    pub provider_payload: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct CalendarEventWindowQuery {
    pub workspace_id: String,
    pub query_start_date: NaiveDate,
    pub query_end_date_exclusive: NaiveDate,
    pub window_start_utc: DateTime<Utc>,
    pub window_end_utc: DateTime<Utc>,
    pub source_ids: Vec<String>,
}

#[cfg(test)]
mod temporal_contract_tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    fn timed(
        start_utc: DateTime<Utc>,
        end_utc: DateTime<Utc>,
        start_local: &str,
        end_local: &str,
        tzid: &str,
    ) -> CalendarSyncEventUpsert {
        CalendarSyncEventUpsert {
            id: Some("event-temporal".into()),
            external_id: None,
            external_etag: None,
            title: "Temporal proof".into(),
            description: None,
            location: None,
            start_ts_utc: start_utc,
            end_ts_utc: end_utc,
            start_local: Some(start_local.into()),
            end_local: Some(end_local.into()),
            tzid: tzid.into(),
            all_day: false,
            start_date: None,
            end_date_exclusive: None,
            was_floating: false,
            normalization_note: None,
            status: CalendarEventStatus::Confirmed,
            visibility: CalendarEventVisibility::Private,
            export_mode: CalendarEventExportMode::LocalOnly,
            rrule: None,
            rdate: Vec::new(),
            exdate: Vec::new(),
            is_recurring: false,
            series_id: None,
            instance_key: None,
            is_override: false,
            source_last_seen_at: None,
            attendees: json!([]),
            links: json!([]),
            provider_payload: None,
        }
    }

    #[test]
    fn brussels_dst_gap_is_rejected_without_silent_coercion() {
        let event = timed(
            utc(2026, 3, 29, 1, 30),
            utc(2026, 3, 29, 2, 30),
            "2026-03-29T02:30:00",
            "2026-03-29T04:30:00",
            "Europe/Brussels",
        );
        let error = normalize_calendar_sync_event(event, "UTC").unwrap_err();
        assert!(error.to_string().contains("DST gap"));
    }

    #[test]
    fn brussels_dst_overlap_persists_earlier_and_later_outcomes() {
        for (supplied, expected) in [
            (
                utc(2026, 10, 25, 0, 30),
                CalendarDstResolution::EarlierOffset,
            ),
            (utc(2026, 10, 25, 1, 30), CalendarDstResolution::LaterOffset),
        ] {
            let normalized = normalize_calendar_sync_event(
                timed(
                    supplied,
                    utc(2026, 10, 25, 2, 30),
                    "2026-10-25T02:30:00",
                    "2026-10-25T03:30:00",
                    "Europe/Brussels",
                ),
                "UTC",
            )
            .unwrap();
            let note = normalized.normalization_note.unwrap();
            assert_eq!(note.boundaries.len(), 1);
            assert_eq!(note.boundaries[0].resolution, expected);
            assert_eq!(note.boundaries[0].resolved_utc, supplied);
        }
    }

    #[test]
    fn floating_event_binds_to_source_tzid_and_preserves_intent() {
        let mut event = timed(
            utc(2026, 7, 23, 21, 30),
            utc(2026, 7, 23, 22, 30),
            "2026-07-23T23:30:00",
            "2026-07-24T00:30:00",
            "",
        );
        event.was_floating = true;
        let normalized = normalize_calendar_sync_event(event, "Europe/Brussels").unwrap();
        assert_eq!(normalized.tzid, "Europe/Brussels");
        assert!(normalized.was_floating);
        assert_eq!(
            normalized.start_local.as_deref(),
            Some("2026-07-23T23:30:00")
        );
    }

    #[test]
    fn all_day_is_date_only_half_open_and_rejects_contradictory_instants() {
        let start = NaiveDate::from_ymd_opt(2026, 3, 29).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 30).unwrap();
        let start_utc = calendar_date_start_utc(start, "Europe/Brussels").unwrap();
        let end_utc = calendar_date_start_utc(end, "Europe/Brussels").unwrap();
        assert_eq!(end_utc - start_utc, chrono::Duration::hours(23));
        let mut event = timed(start_utc, end_utc, "unused", "unused", "Europe/Brussels");
        event.all_day = true;
        event.start_local = None;
        event.end_local = None;
        event.start_date = Some(start);
        event.end_date_exclusive = Some(end);
        assert!(normalize_calendar_sync_event(event.clone(), "UTC").is_ok());
        event.end_ts_utc += chrono::Duration::hours(1);
        assert!(normalize_calendar_sync_event(event, "UTC")
            .unwrap_err()
            .to_string()
            .contains("contradict"));
    }

    #[test]
    fn invalid_tzid_and_other_contradictory_timed_payloads_are_rejected() {
        let invalid = timed(
            utc(2026, 7, 23, 9, 0),
            utc(2026, 7, 23, 10, 0),
            "2026-07-23T09:00:00",
            "2026-07-23T10:00:00",
            "Europe/Not-A-Zone",
        );
        assert!(normalize_calendar_sync_event(invalid, "UTC").is_err());

        let contradiction = timed(
            utc(2026, 7, 23, 9, 0),
            utc(2026, 7, 23, 10, 0),
            "2026-07-23T12:00:00",
            "2026-07-23T13:00:00",
            "UTC",
        );
        assert!(normalize_calendar_sync_event(contradiction, "UTC").is_err());

        let reversed = timed(
            utc(2026, 7, 23, 10, 0),
            utc(2026, 7, 23, 9, 0),
            "2026-07-23T10:00:00",
            "2026-07-23T09:00:00",
            "UTC",
        );
        assert!(normalize_calendar_sync_event(reversed, "UTC").is_err());
    }
}
