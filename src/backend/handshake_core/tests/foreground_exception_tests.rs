use std::{fs, time::Duration};

use chrono::Utc;
use handshake_core::operator_foreground::{
    focus_audit::{
        assert_no_handshake_foreground, FocusAuditEvent, FocusAuditReport, OwnedProcessPidSet,
    },
    foreground_exception::{
        ForegroundException, ForegroundExceptionLogRow, ForegroundPacketPolicy,
        RecordingForegroundWarningSink, DIAG_BANNER_REQUEST_EVENT_TYPE,
        FOREGROUND_EXCEPTION_LOG_DIR,
    },
};
use serde_json::Value;

fn read_log_rows(path: &std::path::Path) -> Vec<ForegroundExceptionLogRow> {
    fs::read_to_string(path)
        .expect("foreground log readable")
        .lines()
        .map(|line| serde_json::from_str(line).expect("foreground log row parses"))
        .collect()
}

#[test]
fn foreground_exception_declare_requires_packet_requires_foreground() {
    let temp = tempfile::tempdir().expect("temp dir");
    let sink = RecordingForegroundWarningSink::default();
    let err = ForegroundException::declare(
        ForegroundPacketPolicy::new("WP-KERNEL-004-NO-FOREGROUND", false),
        "operator-driven capture",
        Duration::from_millis(20),
        temp.path(),
        &sink,
    )
    .expect_err("packet must explicitly declare foreground requirement");

    assert!(err
        .to_string()
        .contains("FOREGROUND_REQUIRES_PACKET_DECLARATION"));
    assert!(sink.requests().is_empty());
    assert!(!temp.path().join(".GOV").join("runtime").exists());
}

#[test]
fn foreground_exception_logs_start_and_emits_notification_and_diag_banner_request() {
    let temp = tempfile::tempdir().expect("temp dir");
    let sink = RecordingForegroundWarningSink::default();
    let handle = ForegroundException::declare(
        ForegroundPacketPolicy::new("WP-KERNEL-004-FOREGROUND", true),
        "interactive operator verification",
        Duration::from_millis(25),
        temp.path(),
        &sink,
    )
    .expect("foreground exception declared");

    let requests = sink.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].wp_id, "WP-KERNEL-004-FOREGROUND");
    assert_eq!(
        requests[0].diagnostics_event_type,
        DIAG_BANNER_REQUEST_EVENT_TYPE
    );
    assert!(requests[0]
        .notification_title
        .contains("Foreground interaction"));
    assert!(requests[0]
        .diagnostics_banner_body
        .contains("interactive operator verification"));

    assert_eq!(
        handle.log_path(),
        temp.path()
            .join(".GOV")
            .join("runtime")
            .join(FOREGROUND_EXCEPTION_LOG_DIR)
            .join("WP-KERNEL-004-FOREGROUND.jsonl")
    );
    let rows = read_log_rows(handle.log_path());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event_type, "FOREGROUND_EXCEPTION_START");
    assert_eq!(rows[0].wp_id, "WP-KERNEL-004-FOREGROUND");
    assert_eq!(rows[0].expected_foreground, true);
    assert_eq!(rows[0].max_duration_ms, 25);
}

#[tokio::test]
async fn controlled_window_auto_dismisses_at_deadline_and_records_end_row() {
    let temp = tempfile::tempdir().expect("temp dir");
    let sink = RecordingForegroundWarningSink::default();
    let handle = ForegroundException::declare(
        ForegroundPacketPolicy::new("WP-KERNEL-004-FOREGROUND", true),
        "bounded controlled test window",
        Duration::from_millis(5),
        temp.path(),
        &sink,
    )
    .expect("foreground exception declared");

    let window = handle
        .bounded_window("foreground-test-window", "handshake://foreground-test")
        .expect("controlled window intent");
    assert_eq!(window.label(), "foreground-test-window");
    assert_eq!(window.url(), "handshake://foreground-test");
    assert!(window.expected_foreground());
    assert!(window.visible());
    assert!(window.focused());

    let dismissal = window
        .auto_dismiss_at_deadline()
        .await
        .expect("auto-dismiss succeeds");
    assert_eq!(dismissal.reason, "auto-dismiss-timeout");
    assert!(window.dismiss("second-close").is_err());

    let event_types: Vec<String> = read_log_rows(handle.log_path())
        .into_iter()
        .map(|row| row.event_type)
        .collect();
    assert_eq!(
        event_types,
        vec![
            "FOREGROUND_EXCEPTION_START",
            "CONTROLLED_WINDOW_OPEN",
            "CONTROLLED_WINDOW_DISMISSED",
            "FOREGROUND_EXCEPTION_END",
        ]
    );
}

#[test]
fn expected_foreground_focus_audit_events_do_not_trigger_unexpected_foreground_violation() {
    let current_pid = 42;
    let events = vec![FocusAuditEvent {
        run_id: "RUN-MT-019".to_string(),
        timestamp_utc: Utc::now(),
        hwnd: "0x0000000000000042".to_string(),
        pid: current_pid,
        exe_name: Some("handshake.exe".to_string()),
        expected_foreground: true,
    }];

    let report = FocusAuditReport::from_events(
        "RUN-MT-019",
        current_pid,
        &OwnedProcessPidSet::default(),
        events,
    );

    assert_eq!(report.expected_foreground_events.len(), 1);
    assert!(report.handshake_owned_events.is_empty());
    assert_no_handshake_foreground(&report).expect("expected foreground is not a violation");
}

#[test]
fn foreground_warning_shape_is_machine_readable_for_tauri_bridge() {
    let temp = tempfile::tempdir().expect("temp dir");
    let sink = RecordingForegroundWarningSink::default();
    let handle = ForegroundException::declare(
        ForegroundPacketPolicy::new("WP-KERNEL-004-FOREGROUND", true),
        "operator warning payload",
        Duration::from_millis(15),
        temp.path(),
        &sink,
    )
    .expect("foreground exception declared");

    let value = serde_json::to_value(&sink.requests()[0]).expect("warning request serializes");
    assert_eq!(value["event_type"], "FOREGROUND_WARNING_REQUEST");
    assert_eq!(
        value["diagnostics_event_type"],
        DIAG_BANNER_REQUEST_EVENT_TYPE
    );
    assert_eq!(value["exception_id"], handle.exception_id().to_string());
    assert_eq!(value["max_duration_ms"], Value::from(15));
}

#[test]
fn tauri_foreground_warning_bridge_is_wired_without_direct_window_show_calls() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("handshake_core lives under src/backend/handshake_core");
    let cargo = fs::read_to_string(repo_root.join("app/src-tauri/Cargo.toml"))
        .expect("read app Cargo.toml");
    let lib =
        fs::read_to_string(repo_root.join("app/src-tauri/src/lib.rs")).expect("read app lib.rs");
    let bridge = fs::read_to_string(repo_root.join("app/src-tauri/src/foreground_warning.rs"))
        .expect("read foreground_warning.rs");

    assert!(cargo.contains("tauri-plugin-notification"));
    assert!(lib.contains("mod foreground_warning;"));
    assert!(lib.contains("tauri_plugin_notification::init()"));
    assert!(lib.contains("foreground_warning::foreground_warning_emit"));
    assert!(bridge.contains("DIAG_BANNER_REQUEST"));
    assert!(bridge.contains("NotificationBuilder::show"));
    assert!(
        !bridge.contains(".show("),
        "foreground warning bridge must avoid direct .show( calls so HBR-QUIET-001 lint stays path-scoped to operator_foreground"
    );
}
