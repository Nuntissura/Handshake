use chrono::{DateTime, Duration, Utc};
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

impl CalendarSourceSyncState {
    pub fn backoff_active_until(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.backoff_until.filter(|deadline| *deadline > now)
    }

    pub fn begin_attempt(&self, direction: &CalendarSyncDirection, now: DateTime<Utc>) -> Self {
        let mut next = self.clone();
        next.state = Some(direction.active_stage());
        next.backoff_until = None;
        match direction {
            CalendarSyncDirection::Pull => {
                next.last_pull_at = Some(now);
            }
            CalendarSyncDirection::Push => {
                next.last_push_at = Some(now);
            }
            CalendarSyncDirection::Mirror => {
                next.last_pull_at = Some(now);
                next.last_push_at = Some(now);
            }
        }
        next
    }

    pub fn mark_success(&self, now: DateTime<Utc>) -> Self {
        let mut next = self.clone();
        next.state = Some(CalendarSyncStateStage::Idle);
        next.last_synced_at = Some(now);
        next.last_ok_at = Some(now);
        if next.sync_token.is_none() {
            next.last_full_sync_at = Some(now);
        }
        next.last_error_at = None;
        next.last_error_code = None;
        next.last_error = None;
        next.backoff_until = None;
        next.consecutive_failures = Some(0);
        next
    }

    pub fn mark_failure(
        &self,
        now: DateTime<Utc>,
        error_code: impl Into<String>,
        error_message: impl Into<String>,
    ) -> Self {
        let mut next = self.clone();
        let failures = self
            .consecutive_failures
            .unwrap_or(0)
            .saturating_add(1)
            .max(1);
        next.state = Some(CalendarSyncStateStage::ErrorBackoff);
        next.last_error_at = Some(now);
        next.last_error_code = Some(error_code.into());
        next.last_error = Some(error_message.into());
        next.backoff_until = Some(now + Self::backoff_delay(failures));
        next.consecutive_failures = Some(failures);
        next
    }

    fn backoff_delay(failures: i64) -> Duration {
        let bounded_failures = failures.clamp(1, 5) - 1;
        let multiplier = 1_i64 << bounded_failures;
        Duration::minutes(5 * multiplier)
    }
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
    pub was_floating: bool,
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
    pub was_floating: bool,
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
    pub window_start_utc: DateTime<Utc>,
    pub window_end_utc: DateTime<Utc>,
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarSyncDirection {
    Pull,
    Push,
    Mirror,
}

impl CalendarSyncDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarSyncDirection::Pull => "pull",
            CalendarSyncDirection::Push => "push",
            CalendarSyncDirection::Mirror => "mirror",
        }
    }

    pub fn active_stage(&self) -> CalendarSyncStateStage {
        match self {
            CalendarSyncDirection::Pull => CalendarSyncStateStage::Pulling,
            CalendarSyncDirection::Push => CalendarSyncStateStage::Pushing,
            CalendarSyncDirection::Mirror => CalendarSyncStateStage::Applying,
        }
    }
}

impl FromStr for CalendarSyncDirection {
    type Err = crate::storage::StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pull" => Ok(CalendarSyncDirection::Pull),
            "push" => Ok(CalendarSyncDirection::Push),
            "mirror" => Ok(CalendarSyncDirection::Mirror),
            _ => Err(crate::storage::StorageError::Validation(
                "invalid calendar sync direction",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarSyncTimeWindow {
    pub start_utc: DateTime<Utc>,
    pub end_utc: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarSyncRequest {
    pub workspace_id: String,
    pub source_id: String,
    pub direction: CalendarSyncDirection,
    pub time_window: CalendarSyncTimeWindow,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarSyncResultStatus {
    Succeeded,
    Failed,
}

pub const CALENDAR_SYNC_RESULT_SCHEMA_VERSION: &str = "calendar_sync_result@1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CalendarSyncResult {
    pub schema_version: String,
    pub workspace_id: String,
    pub source_id: String,
    pub provider_type: CalendarSourceProviderType,
    pub write_policy: CalendarSourceWritePolicy,
    pub direction: CalendarSyncDirection,
    pub time_window: CalendarSyncTimeWindow,
    pub source_sync_state: CalendarSourceSyncState,
    pub updated_events: Vec<CalendarEvent>,
    pub updated_event_count: usize,
    pub status: CalendarSyncResultStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}
