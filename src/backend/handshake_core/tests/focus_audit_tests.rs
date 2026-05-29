use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use handshake_core::{
    operator_foreground::focus_audit::{
        assert_no_handshake_foreground, sanitize_run_id, FocusAuditEvent, FocusAuditLedger,
        FocusAuditReport, OwnedProcessPidSet, FOCUS_AUDIT_LEDGER_DIR,
    },
    process_ledger::{
        ProcessEngineKind, ProcessStart, ProcessStop, ReclaimableProcess,
        POSTGRES_ACTIVE_RECLAIM_QUERY_SQL, PROCESS_LEDGER_MIGRATION_SQL, PROCESS_START_INSERT_SQL,
        PROCESS_STOP_UPSERT_SQL,
    },
};
use serde_json::Value;
use uuid::Uuid;

#[test]
fn focus_audit_contract_uses_wineventhook_foreground_hook_and_pid_resolution() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = fs::read_to_string(manifest.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(cargo.contains("[target.'cfg(windows)'.dependencies]"));
    assert!(cargo.contains("wineventhook = \"0.11.0\""));
    assert!(cargo.contains("windows-sys"));

    let source = fs::read_to_string(manifest.join("src/operator_foreground/focus_audit.rs"))
        .expect("read focus_audit.rs");
    for required in [
        "WindowEventHook::hook",
        "EventFilter::default()",
        ".event(raw_event::SYSTEM_FOREGROUND)",
        ".skip_own_process(true)",
        "GetWindowThreadProcessId",
        "QueryFullProcessImageNameW",
        "PROCESS_QUERY_LIMITED_INFORMATION",
        "CloseHandle",
    ] {
        assert!(
            source.contains(required),
            "focus_audit.rs missing required Windows focus-audit fragment: {required}"
        );
    }
}

#[test]
fn focus_audit_ledger_appends_jsonl_under_runtime_root_without_absolute_paths() {
    let temp = tempfile::tempdir().expect("temp dir");
    let ledger =
        FocusAuditLedger::new("RUN:focus audit/mt-015", temp.path()).expect("focus audit ledger");
    assert_eq!(
        sanitize_run_id("RUN:focus audit/mt-015"),
        "RUN-focus-audit-mt-015"
    );
    assert_eq!(
        ledger.path(),
        temp.path()
            .join("gov_runtime")
            .join(FOCUS_AUDIT_LEDGER_DIR)
            .join("RUN-focus-audit-mt-015.jsonl")
    );

    let event = FocusAuditEvent {
        run_id: "RUN:focus audit/mt-015".to_string(),
        timestamp_utc: Utc::now(),
        hwnd: "0x0000000000001234".to_string(),
        pid: 42,
        exe_name: Some("foreign.exe".to_string()),
        expected_foreground: false,
    };
    ledger.append(event.clone()).expect("append focus event");

    let raw = fs::read_to_string(ledger.path()).expect("read focus audit jsonl");
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 1);
    let parsed: Value = serde_json::from_str(lines[0]).expect("valid json line");
    assert_eq!(parsed["run_id"], event.run_id);
    assert_eq!(parsed["hwnd"], event.hwnd);
    assert_eq!(parsed["pid"], event.pid);

    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !ledger.path().starts_with(manifest_root),
        "focus audit evidence must be rooted in caller-provided runtime root"
    );
}

#[test]
fn focus_audit_report_flags_current_and_owned_process_pids_only() {
    let current_pid = 9001;
    let owned = OwnedProcessPidSet::new(HashSet::from([3210, 6540]));
    let events = vec![
        focus_event("RUN-MT-015", "0x1", current_pid, "handshake.exe"),
        focus_event("RUN-MT-015", "0x2", 3210, "handshake-helper.exe"),
        focus_event("RUN-MT-015", "0x3", 7777, "notepad.exe"),
    ];

    let report = FocusAuditReport::from_events("RUN-MT-015", current_pid, &owned, events.clone());
    assert_eq!(report.total_events, 3);
    assert_eq!(report.handshake_owned_events.len(), 2);
    assert_eq!(report.foreign_events.len(), 1);
    assert_eq!(report.foreign_events[0], events[2]);

    let violation =
        assert_no_handshake_foreground(&report).expect_err("owned foreground violation");
    assert!(violation.to_string().contains("RUN-MT-015"));
    assert!(violation
        .to_string()
        .contains("2 Handshake-owned foreground event"));
}

#[test]
fn process_ownership_ledger_exposes_optional_postgres_os_pid_for_focus_correlation() {
    let migration = PROCESS_LEDGER_MIGRATION_SQL;
    assert!(migration.contains("os_pid BIGINT"));
    assert!(migration.contains("idx_kernel_process_lifecycle_os_pid"));
    assert!(PROCESS_START_INSERT_SQL.contains("os_pid"));
    assert!(PROCESS_STOP_UPSERT_SQL.contains("os_pid"));
    assert!(POSTGRES_ACTIVE_RECLAIM_QUERY_SQL.contains("os_pid"));

    let start = ProcessStart::new(
        ProcessEngineKind::HelperSubprocess,
        "KERNEL_BUILDER",
        Some("WP-KERNEL-004".to_string()),
    )
    .with_os_pid(3210);
    let stop = ProcessStop::from_start(&start, Some(0));
    assert_eq!(start.os_pid, Some(3210));
    assert_eq!(stop.os_pid, Some(3210));

    let reclaimable = ReclaimableProcess {
        process_uuid: Uuid::now_v7(),
        os_pid: Some(6540),
        parent_session_id: "RUN-MT-015".to_string(),
        parent_process_id: None,
        sandbox_adapter_id: Some("sandbox-adapter-test".to_string()),
        sandbox_internal_id: Some("sandbox-internal-test".to_string()),
        engine_kind: ProcessEngineKind::HelperSubprocess,
        started_at: Utc::now(),
        model_artifact_sha256: None,
        work_profile_id: Some("work-profile-test".to_string()),
        owner_role: "KERNEL_BUILDER".to_string(),
        owner_wp: Some("WP-KERNEL-004".to_string()),
        role_id: Some("KERNEL_BUILDER".to_string()),
        wp_id: Some("WP-KERNEL-004".to_string()),
        mt_id: Some("MT-053".to_string()),
        sandbox_capabilities_snapshot: serde_json::json!({"adapter_id": "sandbox-adapter-test"}),
        metadata_jsonb: serde_json::json!({}),
    };
    assert_eq!(reclaimable.reclaim_stop(-1).os_pid, Some(6540));
}

fn focus_event(run_id: &str, hwnd: &str, pid: u32, exe_name: &str) -> FocusAuditEvent {
    FocusAuditEvent {
        run_id: run_id.to_string(),
        timestamp_utc: Utc::now(),
        hwnd: hwnd.to_string(),
        pid,
        exe_name: Some(exe_name.to_string()),
        expected_foreground: false,
    }
}
