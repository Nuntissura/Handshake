use std::path::{Path, PathBuf};

use handshake_core::operator_foreground::foreground_exception::{
    ForegroundException, DIAG_BANNER_REQUEST_EVENT_TYPE, FOREGROUND_WARNING_REQUEST_EVENT_TYPE,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

pub const DIAG_BANNER_REQUEST: &str = "DIAG_BANNER_REQUEST";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForegroundWarningRequest {
    pub event_type: String,
    pub diagnostics_event_type: String,
    pub exception_id: String,
    pub wp_id: String,
    pub reason: String,
    pub max_duration_ms: u64,
    pub notification_title: String,
    pub notification_body: String,
    pub diagnostics_banner_body: String,
    pub timestamp_utc: String,
}

#[tauri::command]
pub fn foreground_warning_emit(
    app: AppHandle,
    request: ForegroundWarningRequest,
) -> Result<(), String> {
    validate_foreground_warning_request(&foreground_warning_repo_root()?, &request)?;
    emit_foreground_warning(&app, &request)
}

pub fn emit_foreground_warning(
    app: &AppHandle,
    request: &ForegroundWarningRequest,
) -> Result<(), String> {
    app.emit(DIAG_BANNER_REQUEST, request)
        .map_err(|error| format!("DIAG_BANNER_REQUEST emit failed: {error}"))?;

    let notification = app
        .notification()
        .builder()
        .title(request.notification_title.clone())
        .body(request.notification_body.clone());
    tauri_plugin_notification::NotificationBuilder::show(notification)
        .map_err(|error| format!("foreground notification failed: {error}"))?;
    Ok(())
}

fn validate_foreground_warning_request(
    repo_root: &Path,
    request: &ForegroundWarningRequest,
) -> Result<(), String> {
    if request.event_type != FOREGROUND_WARNING_REQUEST_EVENT_TYPE {
        return Err(format!(
            "FOREGROUND_WARNING_INVALID_EVENT_TYPE: expected {}, got {}",
            FOREGROUND_WARNING_REQUEST_EVENT_TYPE, request.event_type
        ));
    }
    if request.diagnostics_event_type != DIAG_BANNER_REQUEST_EVENT_TYPE {
        return Err(format!(
            "FOREGROUND_WARNING_INVALID_DIAGNOSTICS_EVENT_TYPE: expected {}, got {}",
            DIAG_BANNER_REQUEST_EVENT_TYPE, request.diagnostics_event_type
        ));
    }
    let exception_id = Uuid::parse_str(request.exception_id.trim())
        .map_err(|error| format!("FOREGROUND_WARNING_INVALID_EXCEPTION_ID: {error}"))?;
    let active =
        ForegroundException::has_active_declaration(repo_root, request.wp_id.trim(), exception_id)
            .map_err(|error| format!("FOREGROUND_WARNING_DECLARATION_CHECK_FAILED: {error}"))?;
    if !active {
        return Err(format!(
            "FOREGROUND_WARNING_UNDECLARED_EXCEPTION: wp_id={} exception_id={}",
            request.wp_id, request.exception_id
        ));
    }
    Ok(())
}

fn foreground_warning_repo_root() -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("HANDSHAKE_REPO_ROOT") {
        let root = PathBuf::from(value);
        if root.join(".GOV").exists() {
            return Ok(root);
        }
    }

    let mut current = std::env::current_dir()
        .map_err(|error| format!("FOREGROUND_WARNING_CWD_UNAVAILABLE: {error}"))?;
    for _ in 0..8 {
        if current.join(".GOV").exists() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }
    Err("FOREGROUND_WARNING_REPO_ROOT_UNAVAILABLE: no .GOV ancestor found".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use handshake_core::operator_foreground::foreground_exception::{
        ForegroundPacketPolicy, RecordingForegroundWarningSink,
    };
    use std::time::Duration;

    fn request_from_core(
        request: handshake_core::operator_foreground::foreground_exception::ForegroundWarningRequest,
    ) -> ForegroundWarningRequest {
        ForegroundWarningRequest {
            event_type: request.event_type.to_string(),
            diagnostics_event_type: request.diagnostics_event_type.to_string(),
            exception_id: request.exception_id.to_string(),
            wp_id: request.wp_id,
            reason: request.reason,
            max_duration_ms: request.max_duration_ms,
            notification_title: request.notification_title,
            notification_body: request.notification_body,
            diagnostics_banner_body: request.diagnostics_banner_body,
            timestamp_utc: request.timestamp_utc.to_rfc3339(),
        }
    }

    #[test]
    fn foreground_warning_rejects_undeclared_exception_id() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let request = ForegroundWarningRequest {
            event_type: FOREGROUND_WARNING_REQUEST_EVENT_TYPE.to_string(),
            diagnostics_event_type: DIAG_BANNER_REQUEST_EVENT_TYPE.to_string(),
            exception_id: Uuid::now_v7().to_string(),
            wp_id: "WP-KERNEL-004-FOREGROUND".to_string(),
            reason: "test".to_string(),
            max_duration_ms: 1000,
            notification_title: "title".to_string(),
            notification_body: "body".to_string(),
            diagnostics_banner_body: "banner".to_string(),
            timestamp_utc: "2026-05-21T06:00:00Z".to_string(),
        };

        let error = validate_foreground_warning_request(tempdir.path(), &request)
            .expect_err("undeclared warning must be rejected");

        assert!(
            error.contains("FOREGROUND_WARNING_UNDECLARED_EXCEPTION"),
            "{error}"
        );
    }

    #[test]
    fn foreground_warning_accepts_active_logged_exception() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let sink = RecordingForegroundWarningSink::default();
        let _handle = ForegroundException::declare(
            ForegroundPacketPolicy::new("WP-KERNEL-004-FOREGROUND", true),
            "bounded foreground warning test",
            Duration::from_secs(5),
            tempdir.path(),
            &sink,
        )
        .expect("declare foreground exception");
        let request =
            request_from_core(sink.requests().into_iter().next().expect("warning request"));

        validate_foreground_warning_request(tempdir.path(), &request)
            .expect("logged exception id accepted");
    }
}
