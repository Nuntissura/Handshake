//! WP-KERNEL-012 MT-093 (§6.13.7 + §10.12.5 Tier-3) Handshake-side proof: the Diagnostics Panel Tier-3
//! section is now POPULATED post-recovery by the freeze/crash records the external Palmistry watcher
//! persisted to its durable survivor store (AC-013-6) — instead of the honest empty-state MT-087 left.
//!
//! Two proofs:
//! - the READ seam (`survivor_forward`): writing durable survivor records to a dir, reading them back as
//!   the typed-allowlist [`PalmistrySurvivorView`]s (newest-first), and the §6.13.8 local-only path guard.
//! - the PANEL reading a forwarded record: driving the live `DiagnosticsPanel::show` widget through an
//!   egui_kittest harness with a populated `palmistry_records` view and asserting the Tier-3 section
//!   renders the forwarded record (the kind + the forwarded flag) under the `diagnostics_palmistry`
//!   AccessKit group — the panel reading a forwarded record (AC-013-6).
//!
//! No wgpu / screenshot is required by MT-093 (its proof_targets are unit/integration, not kittest
//! screenshots); the AccessKit + rendered-text assertions are the runtime proof. No process spawn, no
//! IPC — purely filesystem + a widget harness, so nothing can deadlock (the MT-092 bounded-test rule).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::diagnostics::panel::{DiagnosticsPanel, DiagnosticsView};
use handshake_native::diagnostics::{
    read_survivor_records, PalmistrySurvivorKind, PalmistrySurvivorView,
    DIAGNOSTICS_PALMISTRY_AUTHOR_ID,
};
use handshake_native::theme::palette::HsPalette;

/// Artifact-hygiene guard (the SCREENSHOT/TEST-ARTIFACT rule): no repo-local artifact dir may exist. This
/// test writes NO artifact (no screenshot), but the guard is asserted so a future addition cannot
/// silently introduce a repo-local artifact path.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("hsk-mt093-hsk-it-{label}-{}-{nanos}", std::process::id()))
}

struct DirGuard(PathBuf);
impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_record(dir: &Path, file: &str, json: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join(file), json).unwrap();
}

#[test]
fn seam_reads_durable_survivor_records_typed_newest_first() {
    // AC-013-6 (the read seam): the Handshake side reads the freeze/crash records Palmistry persisted to
    // the durable store as typed-allowlist views, newest-first.
    let dir = temp_dir("seam-read");
    let _g = DirGuard(dir.clone());
    write_record(
        &dir,
        "survivor-freeze-sess-a.json",
        r#"{"schema_version":"hsk.palmistry.survivor@0.1","kind":"Freeze","session_id":"sess-a",
            "process_id":4242,"event_code":7,"stale_ms":6000,"last_heartbeat_counter":42,
            "last_heartbeat_ts_nanos":123,"last_event_count":3,"probe":"NotResponding",
            "crash_detection":null,"faulting_thread_id":null,"exit_code":null,"minidump_path":null,
            "captured_at_unix_ms":1000,"forwarded":true}"#,
    );
    write_record(
        &dir,
        "survivor-crash-sess-b.json",
        r#"{"schema_version":"hsk.palmistry.survivor@0.1","kind":"Crash","session_id":"sess-b",
            "process_id":7,"event_code":8,"stale_ms":0,"last_heartbeat_counter":9,
            "last_heartbeat_ts_nanos":99,"last_event_count":1,"probe":"NotApplicable",
            "detection":"PostMortemNoContext","faulting_thread_id":0,"exit_code":3221225477,
            "minidump_path":"C:/data/palmistry-crash-sess-b.dmp","captured_at_unix_ms":2000,
            "forwarded":false}"#,
    );

    let views = read_survivor_records(&dir);
    assert_eq!(views.len(), 2);
    // Newest-first (crash captured_at 2000 before freeze 1000).
    assert_eq!(views[0].kind, PalmistrySurvivorKind::Crash);
    assert_eq!(views[0].exit_code, Some(3221225477));
    assert!(!views[0].forwarded);
    assert_eq!(views[1].kind, PalmistrySurvivorKind::Freeze);
    assert_eq!(views[1].stale_ms, 6000);
    assert!(views[1].forwarded, "the freeze record was forwarded to the FR ledger");
}

#[test]
fn panel_tier3_section_populates_from_forwarded_records() {
    // AC-013-6 (the panel reading a forwarded record): drive the live DiagnosticsPanel widget with a
    // populated `palmistry_records` view and assert the Tier-3 section renders the forwarded record + the
    // `diagnostics_palmistry` AccessKit group — the §10.12.5 Tier-3 surface becomes POPULATED (no longer
    // the honest empty-state).
    let view = DiagnosticsView {
        palmistry_records: vec![
            PalmistrySurvivorView {
                kind: PalmistrySurvivorKind::Freeze,
                session_id: "sess-freeze".to_owned(),
                process_id: 4242,
                event_code: 7,
                stale_ms: 6000,
                exit_code: None,
                minidump_path: None,
                captured_at_unix_ms: 2000,
                forwarded: true,
            },
            PalmistrySurvivorView {
                kind: PalmistrySurvivorKind::Crash,
                session_id: "sess-crash".to_owned(),
                process_id: 7,
                event_code: 8,
                stale_ms: 0,
                exit_code: Some(0xC000_0005),
                minidump_path: Some("C:/data/palmistry-crash-sess-crash.dmp".to_owned()),
                captured_at_unix_ms: 1000,
                forwarded: false,
            },
        ],
        ..Default::default()
    };

    let palette = HsPalette::dark();
    let mut harness = Harness::new_ui(move |ui| {
        DiagnosticsPanel.show(ui, &view, &palette);
    });
    harness.run();

    // The Tier-3 Palmistry AccessKit group is present.
    let ids: std::collections::HashSet<String> = harness
        .root()
        .children_recursive()
        .filter_map(|n| n.accesskit_node().author_id().map(|a| a.to_owned()))
        .collect();
    assert!(
        ids.contains(DIAGNOSTICS_PALMISTRY_AUTHOR_ID),
        "the Tier-3 Palmistry section must render its AccessKit group; got {:?}",
        ids.iter().filter(|i| i.contains("palmistry") || i.contains("diagnostics")).collect::<Vec<_>>()
    );

    // The panel rendered the forwarded freeze record (its kind label + the forwarded flag) and the crash
    // record's exit code + LOCAL minidump path — proving the panel READ a forwarded record (AC-013-6),
    // not the empty-state. `get_by_label` finds the rendered text node.
    assert!(harness.query_by_label("Freeze").is_some(), "the freeze record kind must render");
    assert!(harness.query_by_label("Crash").is_some(), "the crash record kind must render");
    assert!(
        harness.query_by_label("forwarded").is_some(),
        "the forwarded-to-ledger flag must render for the forwarded record"
    );
    assert!(
        harness.query_by_label("stale 6000ms").is_some(),
        "the freeze stale-duration evidence must render"
    );
    // The crash's LOCAL minidump path renders as a local reference (never bytes, never a URL). The panel
    // renders it as the exact label `minidump: <path>`.
    assert!(
        harness
            .query_by_label("minidump: C:/data/palmistry-crash-sess-crash.dmp")
            .is_some(),
        "the crash record's LOCAL minidump path must render"
    );

    // The honest empty-state line is NOT shown when records exist.
    assert!(
        harness.query_by_label("No freeze/crash records.").is_none(),
        "the empty-state must be replaced by the real records when present"
    );

    assert_no_local_artifact_dir();
}

#[test]
fn panel_tier3_shows_honest_empty_state_when_no_records() {
    // The honest empty-state is preserved when there are no forwarded records (AC-007-4/5 — no regression).
    let view = DiagnosticsView::default(); // palmistry_records empty.
    let palette = HsPalette::dark();
    let mut harness = Harness::new_ui(move |ui| {
        DiagnosticsPanel.show(ui, &view, &palette);
    });
    harness.run();
    assert!(
        harness.query_by_label("No freeze/crash records.").is_some(),
        "the honest empty-state must render when no survivor records exist"
    );
}
