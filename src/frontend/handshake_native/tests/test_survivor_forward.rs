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
    std::env::temp_dir().join(format!(
        "hsk-mt093-hsk-it-{label}-{}-{nanos}",
        std::process::id()
    ))
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
    assert!(
        views[1].forwarded,
        "the freeze record was forwarded to the FR ledger"
    );
}

/// MT-093 remediation — the GOLDEN-FILE CROSS-CRATE ROUND-TRIP (the §6.13.7 file contract, pinned by
/// BOTH real implementations instead of hand-mirrored fixtures).
///
/// The audited defect: the durable survivor-record schema was hand-mirrored in this crate
/// (`SurvivorRecordOnDisk`) with NO shared schema and no test driving both real sides — the reader's
/// untagged `kind` fallback shows drift already happened once (flat `"kind":"Freeze"` fixtures vs the
/// live tagged `"kind":{"kind":"Freeze"}` serde shape). This test closes that gap WITHOUT coupling the
/// two binaries at runtime (the §6.13 lifecycle-inversion stance keeps them file-decoupled):
///
/// - WRITE through the REAL `palmistry::survivor_store::SurvivorStore` (the palmistry crate is a
///   dev-dependency here for exactly this) — a freeze, a CrashContext-minidump crash, and a child-stall
///   record, with the freeze marked forwarded through the real idempotent flag path;
/// - PIN the on-disk GOLDEN SHAPE (schema_version vocabulary + the LIVE tagged `kind` object) from the
///   raw JSON bytes, so a serde-shape change on the writer side fails HERE, not silently in the panel;
/// - READ back through the REAL `handshake_native::diagnostics::read_survivor_records` and assert every
///   typed-allowlist field survived the crate boundary.
///
/// A schema drift on EITHER side now fails this test instead of silently rendering `Other`/defaults in
/// the Tier-3 panel.
#[test]
fn golden_cross_crate_roundtrip_real_store_writes_native_reader_reads() {
    use palmistry::child_stall::{ChildStallReasonCode, ChildStallReport};
    use palmistry::crash_capture::{CrashDetection, CrashRecord};
    use palmistry::freeze_detect::FreezeReport;
    use palmistry::survivor_store::{SurvivorProbeResult, SurvivorRecord, SurvivorStore};

    let dir = temp_dir("golden-roundtrip");
    let _g = DirGuard(dir.clone());

    let mut store = SurvivorStore::open(&dir).expect("open the REAL palmistry survivor store");

    // 1) FREEZE — through the real MT-091 report type; marked forwarded through the real flag path.
    let freeze = SurvivorRecord::from_freeze(
        "sess-golden-freeze",
        4242,
        &FreezeReport {
            stale_ms: 6543,
            last_heartbeat_counter: 77,
            last_heartbeat_ts_nanos: 77_000,
        },
        3,
        SurvivorProbeResult::NotResponding,
    );
    let freeze_event_code = freeze.event_code;
    let freeze_path = store.put(freeze).expect("persist the freeze record");
    assert!(
        store
            .mark_forwarded(&freeze_path)
            .expect("mark the freeze forwarded"),
        "the freeze record must be known to the store"
    );

    // 2) CRASH — the rich CrashContext-minidump case, with a LOCAL dump path reference.
    let dump_path = dir.join("palmistry-crash-sess-golden-crash.dmp");
    let crash = SurvivorRecord::from_crash(&CrashRecord {
        session_id: "sess-golden-crash".to_owned(),
        detection: CrashDetection::CrashContextMinidump,
        crash_event_code: 8,
        process_id: 7,
        faulting_thread_id: 99,
        exit_code: None,
        last_heartbeat_counter: 9,
        last_heartbeat_ts_nanos: 9_000,
        last_event_count: 1,
        minidump_path: Some(dump_path.clone()),
        recorded_at_unix_ms: 0,
    });
    store.put(crash).expect("persist the crash record");

    // 3) CHILD-STALL — through the real MT-106 report type.
    let stall = SurvivorRecord::from_child_stall(
        "sess-golden-child",
        4242,
        &ChildStallReport {
            child_pid: 333,
            child_session_id: 90,
            stale_ms: 6100,
            last_progress_counter: 5,
            last_progress_ts_nanos: 777,
            reason_code: ChildStallReasonCode::ProgressStaleWhileAlive,
        },
        2,
    );
    store.put(stall).expect("persist the child-stall record");

    // GOLDEN-SHAPE PIN (the writer side): the raw on-disk JSON carries the fixed schema-version
    // vocabulary and the LIVE serde shape for the typed kind — an OBJECT with its own `"kind"` tag
    // (`"kind":{"kind":"Freeze"}`), the exact shape whose earlier drift motivated this remediation.
    let raw = std::fs::read_to_string(&freeze_path).expect("read the raw freeze record bytes");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("the record is valid JSON");
    assert_eq!(
        json["schema_version"], "hsk.palmistry.survivor@0.1",
        "the on-disk schema-version vocabulary is the golden contract"
    );
    assert_eq!(
        json["kind"]["kind"], "Freeze",
        "the LIVE tagged kind shape ('kind' is an object carrying its own tag) is the golden contract \
         the native reader's Tagged variant decodes; a writer-side serde change must fail here"
    );
    assert_eq!(json["forwarded"], true, "mark_forwarded persisted durably");

    // READ back through the REAL native reader (the panel's projection path).
    let views = read_survivor_records(&dir);
    assert_eq!(views.len(), 3, "all three real records decode");

    let freeze_view = views
        .iter()
        .find(|v| v.kind == PalmistrySurvivorKind::Freeze)
        .expect("the freeze record decodes to the typed Freeze kind (NOT Other) across the boundary");
    assert_eq!(freeze_view.session_id, "sess-golden-freeze");
    assert_eq!(freeze_view.process_id, 4242);
    assert_eq!(freeze_view.event_code, freeze_event_code);
    assert_eq!(freeze_view.stale_ms, 6543);
    assert_eq!(freeze_view.exit_code, None);
    assert_eq!(freeze_view.minidump_path, None);
    assert!(
        freeze_view.forwarded,
        "the idempotent forwarded flag survives the crate boundary"
    );

    let crash_view = views
        .iter()
        .find(|v| v.kind == PalmistrySurvivorKind::Crash)
        .expect("the crash record decodes to the typed Crash kind across the boundary");
    assert_eq!(crash_view.session_id, "sess-golden-crash");
    assert_eq!(crash_view.process_id, 7);
    assert_eq!(crash_view.event_code, 8);
    assert_eq!(crash_view.exit_code, None);
    assert_eq!(
        crash_view.minidump_path.as_deref(),
        Some(dump_path.to_string_lossy().as_ref()),
        "the LOCAL minidump path reference survives the crate boundary (a path string, never bytes)"
    );
    assert!(!crash_view.forwarded, "the crash record stays pending");

    let stall_view = views
        .iter()
        .find(|v| v.kind == PalmistrySurvivorKind::ChildStall)
        .expect("the child-stall record decodes to the typed ChildStall kind across the boundary");
    assert_eq!(stall_view.session_id, "sess-golden-child");
    assert_eq!(stall_view.stale_ms, 6100);
    assert_eq!(stall_view.child_process_id, Some(333));
    assert_eq!(stall_view.child_session_id, Some(90));
    assert_eq!(stall_view.last_progress_counter, Some(5));
    assert_eq!(stall_view.last_progress_ts_nanos, Some(777));
    assert_eq!(stall_view.child_stall_reason_code, Some(1));

    assert_no_local_artifact_dir();
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
                child_process_id: None,
                child_session_id: None,
                last_progress_counter: None,
                last_progress_ts_nanos: None,
                child_stall_reason_code: None,
                captured_at_unix_ms: 2000,
                forwarded: true,
            },
            PalmistrySurvivorView {
                kind: PalmistrySurvivorKind::ChildStall,
                session_id: "sess-child".to_owned(),
                process_id: 4242,
                event_code: 12,
                stale_ms: 6100,
                exit_code: None,
                minidump_path: None,
                child_process_id: Some(333),
                child_session_id: Some(90),
                last_progress_counter: Some(5),
                last_progress_ts_nanos: Some(777),
                child_stall_reason_code: Some(1),
                captured_at_unix_ms: 1500,
                forwarded: false,
            },
            PalmistrySurvivorView {
                kind: PalmistrySurvivorKind::Crash,
                session_id: "sess-crash".to_owned(),
                process_id: 7,
                event_code: 8,
                stale_ms: 0,
                exit_code: Some(0xC000_0005),
                minidump_path: Some("C:/data/palmistry-crash-sess-crash.dmp".to_owned()),
                child_process_id: None,
                child_session_id: None,
                last_progress_counter: None,
                last_progress_ts_nanos: None,
                child_stall_reason_code: None,
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
        ids.iter()
            .filter(|i| i.contains("palmistry") || i.contains("diagnostics"))
            .collect::<Vec<_>>()
    );

    // The panel rendered the forwarded freeze record (its kind label + the forwarded flag) and the crash
    // record's exit code + LOCAL minidump path — proving the panel READ a forwarded record (AC-013-6),
    // not the empty-state. `get_by_label` finds the rendered text node.
    assert!(
        harness.query_by_label("Freeze").is_some(),
        "the freeze record kind must render"
    );
    assert!(
        harness.query_by_label("Crash").is_some(),
        "the crash record kind must render"
    );
    assert!(
        harness.query_by_label("ChildStall").is_some(),
        "the child-stall record kind must render"
    );
    assert!(
        harness.query_by_label("forwarded").is_some(),
        "the forwarded-to-ledger flag must render for the forwarded record"
    );
    assert!(
        harness.query_by_label("stale 6000ms").is_some(),
        "the freeze stale-duration evidence must render"
    );
    assert!(
        harness.query_by_label("child 333 session 90").is_some(),
        "the child-stall pid/session evidence must render compactly"
    );
    assert!(
        harness
            .query_by_label("stale 6100ms progress 5 reason 1")
            .is_some(),
        "the child-stall stale/progress/reason evidence must render compactly"
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
        harness
            .query_by_label("No freeze/crash/child-stall records.")
            .is_none(),
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
        harness
            .query_by_label("No freeze/crash/child-stall records.")
            .is_some(),
        "the honest empty-state must render when no survivor records exist"
    );
}
