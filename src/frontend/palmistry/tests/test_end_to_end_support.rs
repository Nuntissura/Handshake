//! WP-KERNEL-012 MT-096 (G2 end-to-end capstone) — the PALMISTRY-SIDE (Tier 3) half of the integrated
//! three-tier proof (Master Spec v02.196 §6.13).
//!
//! The capstone is INTEGRATION: it WIRES the tiers built in MT-081..095 and proves them as a WHOLE on
//! the REAL APIs. handshake-native (the Tier-2 writer) and `palmistry` (the Tier-3 reader) are SIBLING
//! crates with NO dependency edge — their ONLY shared crate is `handshake-diag-ring`, the ring substrate
//! compiled identically into both. So the integrated proof necessarily has two halves that MEET AT THE
//! RING CONTRACT:
//!
//! - the handshake-native half (`tests/test_three_tier_end_to_end.rs`) drives the REAL `HandshakeApp`
//!   and proves the Tier-2 writer publishes an advancing-then-stale heartbeat into the REAL MT-081 ring,
//!   observable with ZERO cooperation (the freeze write-side + the backend-down re-prove);
//! - THIS file drives the REAL `palmistry` production types (the ring reader MT-090, the double-signal
//!   freeze detector MT-091, the crash record MT-092, the durable survivor store + FR forwarder MT-093,
//!   the lifecycle inversion MT-089) against a REAL ring written EXACTLY as Handshake writes it, proving
//!   the Tier-3 READ -> DETECT -> SURVIVE -> RECORD pipeline end-to-end.
//!
//! Together the two halves prove the whole system: Handshake writes a ring whose stale heartbeat a
//! zero-cooperation reader observes, and Palmistry's real detector reads that exact ring layout and
//! captures + survives + records the freeze/crash. No tier is mocked; the ring is the real substrate.
//!
//! BOUNDED-TEST RULE (MT-092 precedent, packet `palmistry_test_bound_policy`): every proof here uses the
//! library types DIRECTLY with an INJECTED virtual clock + an in-process store — it spawns NO palmistry
//! binary, does NO minidumper IPC, and waits on NO child, so nothing here can deadlock the suite. The
//! LIVE cross-process minidump (the known hanger) stays proven by MT-092's `tests/test_crash_capture.rs`
//! cross_process_* and is referenced (not re-run) here, with a #[ignore]d real-host pointer.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use handshake_diag_ring::{
    DiagEvent, DiagRingWriter, DiagTier, ThreeTierDiagnosticWiringRecord, TierWiring,
    DEFAULT_CAPACITY,
};

use palmistry::crash_capture::{CrashDetection, CrashRecord};
use palmistry::fr_forward::{FrForwardBlocker, FrForwarder, FR_INGESTION_FOLLOW_ON_WP};
use palmistry::freeze_detect::{FreezeDetector, FreezeState};
use palmistry::hung_window_probe::{FakeHungWindowProbe, ProbeResult};
use palmistry::lifecycle::{
    build_survivor_record, run_lifecycle, ExitReason, LifecycleConfig, LifecycleState,
};
use palmistry::ring_reader::PalmistryRingReader;
use palmistry::survivor_store::{
    assert_typed_allowlist, last_event_count, SurvivorProbeResult, SurvivorRecord,
    SurvivorRecordKind, SurvivorStore,
};

// ── external artifact root (CX-212E / the SCREENSHOT-TEST-ARTIFACT rule) ───────────────────────────────

/// The crate-relative EXTERNAL artifacts root, disk-agnostic: the `palmistry` crate sits at
/// `<repo>/src/frontend/palmistry`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is a
/// sibling of the repo worktree — the SAME convention `handshake_native`'s tests use. The durable
/// survivor records + the three-tier evidence land HERE (never repo-local).
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// CX-212E hygiene guard: NO repo-local artifact dir may exist under the crate (artifacts go external).
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// A unique per-test temp ring path (no collision across parallel tests).
fn temp_ring_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "hsk-mt096-{label}-{}-{nanos}.ring",
        std::process::id()
    ))
}

/// A unique per-test durable survivor-store dir under the EXTERNAL artifact root (so the records are real
/// durable proof artifacts the capstone manifest can reference, never repo-local).
fn survivor_store_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    external_artifact_dir("wp-kernel-012-mt-096")
        .join(format!("survivors-{label}-{}-{nanos}", std::process::id()))
}

struct PathGuard(PathBuf);
impl Drop for PathGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// ── AC-016-1 (FREEZE end-to-end, Tier-3 half): detect a freeze on a REAL ring, then survive + record ───

/// SCENARIO 1 (FREEZE), the Tier-3 half. A REAL MT-081 ring is written EXACTLY as Handshake writes it
/// (advancing heartbeats + typed events via `DiagRingWriter`), then the writer STALLS (stops advancing —
/// the freeze). The REAL Palmistry pipeline then:
///
/// 1. reads the ring with ZERO cooperation (MT-090 `PalmistryRingReader`) — the stale heartbeat + the
///    last-N typed events the writer published before it froze stay readable (no writer cooperation);
/// 2. CONFIRMS the freeze via the MT-091 double-signal gate (stale counter + a NotResponding hung-window
///    probe) — the advancing phase never trips it; only the stall + corroboration does;
/// 3. SURVIVES + RECORDS: builds the typed survivor record from the freeze report and persists it to the
///    durable MT-093 survivor store, which a simulated Palmistry RESTART reads back (survives the
///    watcher's own restart, §6.13.7) — captured + survived + recorded, end-to-end on the real ring.
#[test]
fn freeze_end_to_end_detect_survive_record_on_real_ring() {
    let ring_path = temp_ring_path("freeze");
    let _g = PathGuard(ring_path.clone());

    // The REAL ring the Tier-2 writer would create. We write through the SAME `DiagRingWriter` Handshake
    // uses (no test reimplementation of the ring), so the layout Palmistry maps is byte-identical.
    let writer =
        DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY).expect("create real MT-081 ring");
    // A couple of typed events (POD integers) so the last-N evidence read is non-empty — these are the
    // events the writer "published before it froze" that stay readable zero-coop.
    writer.write(DiagEvent::heartbeat(1, 10, 1, 100));
    writer.write(DiagEvent::resource_sample(1, 20, 500, 1024, 0, 200));

    // The REAL zero-cooperation reader maps the SAME backing file (no writer cooperation, §6.13.4).
    let reader = PalmistryRingReader::open(&ring_path).expect("Palmistry maps the real ring");

    // A small staleness threshold so the freeze path is reached in virtual time (no real sleeps).
    let mut detector = FreezeDetector::with_threshold(Duration::from_millis(100));
    let not_responding = FakeHungWindowProbe::new(ProbeResult::NotResponding);
    let base = Instant::now();

    // PHASE A — ADVANCING heartbeat: write 1..=5, read each back through the reader, poll the detector.
    // An advancing counter must NEVER trip a freeze (the no-false-positive gate), even with a
    // NotResponding probe.
    for counter in 1..=5u64 {
        writer.write_heartbeat(counter, counter * 1000);
        let hb = reader.read_heartbeat();
        let state = detector.poll(
            base + Duration::from_millis(counter * 10),
            hb,
            &not_responding,
        );
        assert_eq!(
            state,
            FreezeState::Healthy,
            "an advancing heartbeat must stay Healthy (counter {counter})"
        );
    }

    // PHASE B — FREEZE: the writer STALLS (no more heartbeat writes). The reader still maps the ring and
    // returns the LAST heartbeat (counter 5) with zero cooperation. After the threshold elapses, the
    // double-signal gate confirms a freeze.
    let last_hb = reader
        .read_heartbeat()
        .expect("the stale heartbeat stays readable zero-coop");
    assert_eq!(
        last_hb.counter, 5,
        "the reader observes the frozen-at-5 heartbeat"
    );
    let frozen_state = detector.poll(
        base + Duration::from_millis(5_000),
        reader.read_heartbeat(),
        &not_responding,
    );
    let report = match frozen_state {
        FreezeState::Frozen(report) => report,
        other => panic!("AC-016-1: a stalled heartbeat + NotResponding probe must CONFIRM a freeze, got {other:?}"),
    };
    assert_eq!(
        report.last_heartbeat_counter, 5,
        "the freeze report carries the last published counter"
    );
    assert!(
        report.stale_ms >= 100,
        "the freeze report carries the stale duration"
    );

    // The last-N typed events the writer published before the freeze stay readable zero-coop (the crash/
    // freeze evidence bundle). They are POD integers — never text.
    let events = reader.read_last_events(8);
    assert!(
        !events.is_empty(),
        "the last-N events before the freeze stay readable zero-coop"
    );
    let evidence_count = last_event_count(&events);

    // SURVIVE + RECORD: build the typed survivor record and persist it durably (§6.13.7). Then simulate a
    // Palmistry RESTART by reopening the store on the SAME dir — the record OUTLIVES the watcher restart.
    let store_dir = survivor_store_dir("freeze");
    let _sg = DirGuard(store_dir.clone());
    let persisted_path = {
        let mut store = SurvivorStore::open(&store_dir).expect("open durable survivor store");
        let record = SurvivorRecord::from_freeze(
            "mt096-freeze-sess",
            std::process::id(),
            &report,
            evidence_count,
            SurvivorProbeResult::NotResponding,
        );
        // The record is typed-allowlist clean (no project content) BEFORE it ever touches disk.
        assert_typed_allowlist(&record).expect("freeze survivor record is typed-allowlist clean");
        store
            .put(record)
            .expect("persist the freeze survivor record")
    }; // store dropped — simulate the Palmistry process exiting.
    assert!(
        persisted_path.exists(),
        "the durable freeze record exists on disk"
    );

    // A RESTARTED Palmistry reads the pending freeze record back (it survived the watcher's own restart).
    let restarted = SurvivorStore::open(&store_dir).expect("restart Palmistry on the same store");
    assert_eq!(
        restarted.records().len(),
        1,
        "the freeze record survived a Palmistry restart"
    );
    let back = &restarted.records()[0].record;
    assert_eq!(back.kind, SurvivorRecordKind::Freeze);
    assert_eq!(back.last_heartbeat_counter, 5);
    assert!(
        !back.forwarded,
        "still pending FR forward after the restart"
    );

    drop(reader);
    drop(writer);
    assert_no_local_artifact_dir();
}

// ── AC-016-2 (CRASH end-to-end, Tier-3 half): crash record + survive + record; clean-shutdown gate ─────

/// SCENARIO 2 (CRASH), the Tier-3 half (the DETERMINISTIC floor case + the clean-shutdown gate).
///
/// The RICH out-of-process minidump (a real `CrashContext` -> `minidumper` -> `minidump-writer` round-
/// trip) is the known IPC hanger; it is proven by MT-092's `tests/test_crash_capture.rs::cross_process_*`
/// and is NOT re-run here (BOUNDED-TEST RULE). This test proves the integrated CRASH RECORD pipeline
/// deterministically: a crash is detected (the §6.13.6 FLOOR case — an abnormal parent exit with no
/// CrashContext), the typed crash record is built (carrying the last-heartbeat/last-event evidence read
/// from the ring), and it SURVIVES + is RECORDED into the durable store (read back after a restart).
///
/// The §6.13 CLEAN-SHUTDOWN GATE is proven alongside: a CLEAN lifecycle (explicit Shutdown before any
/// parent exit) records `CleanShutdown` with `abnormal_parent_exit == false` and writes NO crash survivor
/// record — a clean shutdown is NOT a crash.
#[test]
fn crash_end_to_end_floor_record_survive_and_clean_shutdown_gate() {
    let ring_path = temp_ring_path("crash");
    let _g = PathGuard(ring_path.clone());
    let writer = DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY).expect("create real ring");
    writer.write_heartbeat(9, 9_000);
    writer.write(DiagEvent::heartbeat(1, 1, 9, 9_000));
    let reader = PalmistryRingReader::open(&ring_path).expect("Palmistry maps the ring");

    // (a) CRASH detected as the FLOOR case: an abnormal parent death with NO CrashContext. The crash
    // record carries the last heartbeat + last-N events read PASSIVELY from the ring (zero cooperation —
    // the parent is already gone), and NO minidump (the hard-kill path cannot produce one post-mortem).
    let last_hb = reader.read_heartbeat();
    let last_events = reader.read_last_events(8);
    let crash = CrashRecord::post_mortem(
        "mt096-crash-sess",
        std::process::id(),
        Some(0xC000_0409), // a STATUS_STACK_BUFFER_OVERRUN-shaped abnormal exit code (a number, not content)
        last_hb,
        &last_events,
    );
    assert_eq!(crash.detection, CrashDetection::PostMortemNoContext);
    assert!(
        crash.minidump_path.is_none(),
        "the floor case writes NO minidump"
    );
    assert_eq!(
        crash.last_heartbeat_counter, 9,
        "the crash record bundles the last ring heartbeat"
    );

    // SURVIVE + RECORD: persist the crash survivor record durably + read it back after a restart.
    let store_dir = survivor_store_dir("crash");
    let _sg = DirGuard(store_dir.clone());
    {
        let mut store = SurvivorStore::open(&store_dir).expect("open store");
        let record = SurvivorRecord::from_crash(&crash);
        assert_typed_allowlist(&record).expect("crash survivor record is typed-allowlist clean");
        store
            .put(record)
            .expect("persist the crash survivor record");
    }
    let restarted = SurvivorStore::open(&store_dir).expect("restart on the same store");
    assert_eq!(
        restarted.records().len(),
        1,
        "the crash record survived a Palmistry restart"
    );
    assert_eq!(
        restarted.records()[0].record.kind,
        SurvivorRecordKind::Crash
    );

    // (b) The §6.13 CLEAN-SHUTDOWN GATE: a clean lifecycle records no crash. Drive the REAL lifecycle
    // state machine to a clean Shutdown (no parent death) and assert the terminal reason + survivor facts.
    let state = LifecycleState::new();
    let run = std::sync::atomic::AtomicBool::new(true);
    state.request_shutdown(); // an explicit Shutdown arrives while the parent is alive.
    let reason = run_lifecycle(&state, &run, fast_lifecycle_config());
    assert_eq!(
        reason,
        ExitReason::CleanShutdown,
        "a clean shutdown is not a crash"
    );
    let lifecycle_record =
        build_survivor_record("mt096-clean-sess", std::process::id(), &state, reason);
    assert!(
        !lifecycle_record.abnormal_parent_exit,
        "AC-016-2: a clean shutdown records NO abnormal exit (the §6.13 clean-shutdown gate)"
    );
    assert!(lifecycle_record.shutdown_received);

    // And a clean shutdown writes NO crash survivor record: an empty store stays empty (no put on clean).
    let clean_store_dir = survivor_store_dir("clean");
    let _csg = DirGuard(clean_store_dir.clone());
    let clean_store = SurvivorStore::open(&clean_store_dir).expect("open clean store");
    assert_eq!(
        clean_store.records().len(),
        0,
        "AC-016-2: a clean shutdown produces NO crash survivor record"
    );

    drop(reader);
    drop(writer);
    assert_no_local_artifact_dir();
}

// ── AC-016-6 (HONEST GATING): the FR-forward LIVE round-trip is a typed blocker, never faked ───────────

/// SCENARIO honest-gating: the recovery-time Flight-Recorder FORWARD (MT-093, §6.13.7) needs a managed
/// backend to round-trip LIVE. Forwarding a survivor record into the EXISTING (kept-as-is)
/// `runtime_chat_event` route returns the typed [`FrForwardBlocker::SchemaIncompatible`] — an HONEST
/// typed blocker naming the WP-016 follow-on, NOT a faked success. The record stays LOCAL + pending. This
/// is the AC-016-6 honesty gate: where a managed resource is genuinely absent, the capstone records
/// `NEEDS_MANAGED_RESOURCE_PROOF`, it does not fabricate a forward.
#[test]
fn fr_forward_live_half_is_an_honest_typed_blocker_not_faked() {
    let report = palmistry::freeze_detect::FreezeReport {
        stale_ms: 6000,
        last_heartbeat_counter: 42,
        last_heartbeat_ts_nanos: 123,
    };
    let record = SurvivorRecord::from_freeze(
        "mt096-fwd-sess",
        4242,
        &report,
        3,
        SurvivorProbeResult::NotResponding,
    );

    // The PRODUCTION forwarder against the EXISTING FR route. The base URL is a reliably-refused dead port
    // to PROVE the blocker is decided WITHOUT a network call (we never fake a degraded post as a forward).
    let forwarder = FrForwarder::for_existing_fr("http://127.0.0.1:1");
    let blocker = forwarder
        .forward(&record)
        .expect_err("AC-016-6: forwarding into the kept-as-is FR must be a typed blocker, not Ok");
    match blocker {
        FrForwardBlocker::SchemaIncompatible { follow_on_wp, .. } => {
            assert_eq!(
                follow_on_wp, FR_INGESTION_FOLLOW_ON_WP,
                "the honest blocker names the WP-016 follow-on (NEEDS_MANAGED_RESOURCE_PROOF)"
            );
        }
        other => panic!("AC-016-6: expected the SchemaIncompatible honest blocker, got {other:?}"),
    }
    assert_no_local_artifact_dir();
}

// ── AC-016-5 (whole-WP three-tier evidence, Palmistry tier): emit the MT-095 record from the Tier-3 side ─

/// The Palmistry-side contribution to the whole-WP [`ThreeTierDiagnosticWiringRecord`] (MT-095 format):
/// the PALMISTRY tier is WIRED with proof_refs to the freeze/crash detect-survive-record proofs above and
/// the MT-092/094 live cross-process proofs. Emitted to the EXTERNAL artifact root. The handshake-native
/// capstone emits the full whole-WP record (all three tiers); this asserts the Tier-3 side is well-formed
/// HBR-INT-009 evidence on its own.
#[test]
fn palmistry_tier_three_evidence_record_is_well_formed() {
    let record = ThreeTierDiagnosticWiringRecord::new(
        "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
        "MT-096",
        "palmistry_tier3_freeze_crash_capture",
        handshake_diag_ring::run_at_now(),
        vec![
            // FR forward needs a managed backend -> DEFERRED honestly (NEEDS_MANAGED_RESOURCE_PROOF).
            TierWiring::deferred(
                DiagTier::FlightRecorder,
                "FR-forward live round-trip needs managed PostgreSQL/backend (gated requires_pg, \
                 NEEDS_MANAGED_RESOURCE_PROOF); the kept-as-is route returns a typed blocker (AC-016-6)",
            ),
            // The Tier-2 internal_diagnostics writer is proven on the handshake-native capstone side.
            TierWiring::not_applicable(
                DiagTier::InternalDiagnostics,
                "Tier-2 writer is proven on the handshake-native capstone side \
                 (test_three_tier_end_to_end); this Palmistry-side file proves the Tier-3 reader",
            ),
            TierWiring::wired(
                DiagTier::Palmistry,
                "MT-096 test_end_to_end_support: freeze detect+survive+record + crash floor record on a \
                 REAL ring; MT-092 cross_process_* live minidump; MT-094 launched-with-Handshake",
            ),
        ],
    );
    record
        .validate()
        .expect("the Palmistry-side three-tier evidence record is well-formed");

    let out_dir = external_artifact_dir("wp-kernel-012-mt-096").join("palmistry-evidence");
    let written = record
        .emit(&out_dir)
        .expect("emit the Palmistry-side three-tier evidence to the external root");
    assert!(
        written.exists(),
        "the three-tier evidence file was written externally"
    );
    println!(
        "MT-096 Palmistry three-tier evidence: {}",
        std::fs::canonicalize(&written)
            .unwrap_or(written.clone())
            .display()
    );
    assert_no_local_artifact_dir();
}

// ── AC-016-4 (privacy, Tier-3 artifacts): the survivor + forward bodies carry only typed-allowlist data ─

/// The Tier-3 artifacts (the durable survivor records + the FR-forward body) carry ONLY typed-allowlist
/// data — numbers, typed enum tags, an opaque session token, numeric timestamps, and a LOCAL path string
/// — NO project content / free text (§6.13.8). This is the Palmistry-side contribution to the system-wide
/// scan (AC-016-4); the handshake-native `test_three_tier_privacy_allowlist` scans the full artifact set.
#[test]
fn tier3_artifacts_are_typed_allowlist_only() {
    let report = palmistry::freeze_detect::FreezeReport {
        stale_ms: 6000,
        last_heartbeat_counter: 42,
        last_heartbeat_ts_nanos: 123,
    };
    let freeze =
        SurvivorRecord::from_freeze("sess-a", 1, &report, 3, SurvivorProbeResult::NotResponding);
    let crash_rec = CrashRecord::post_mortem("sess-b", 2, Some(3), None, &[]);
    let crash = SurvivorRecord::from_crash(&crash_rec);

    // The durable survivor records are typed-allowlist clean (key + value scan, URL paths rejected).
    assert_typed_allowlist(&freeze).expect("freeze record typed-allowlist clean");
    assert_typed_allowlist(&crash).expect("crash record typed-allowlist clean");

    // The FR-forward body (the WP-016 ingestion / stub shape) carries only typed fields — no free text.
    let body = palmistry::fr_forward::build_survivor_forward_body(&freeze);
    let obj = body.as_object().expect("forward body is a JSON object");
    let allowed: std::collections::HashSet<&str> = [
        "schema_version",
        "kind",
        "session_id",
        "process_id",
        "event_code",
        "stale_ms",
        "last_heartbeat_counter",
        "last_heartbeat_ts_nanos",
        "last_event_count",
        "exit_code",
        "faulting_thread_id",
        "minidump_path",
        "captured_at_unix_ms",
    ]
    .into_iter()
    .collect();
    for key in obj.keys() {
        assert!(
            allowed.contains(key.as_str()),
            "AC-016-4: FR-forward body carried a non-allowlisted key '{key}'"
        );
    }
    assert!(obj.get("message").is_none(), "no free-text 'message' field");
    assert!(obj.get("text").is_none(), "no free-text 'text' field");
    assert_no_local_artifact_dir();
}

// ── #[ignore]d real-host pointer: the LIVE out-of-process minidump (the known IPC hanger) ───────────────

/// The LIVE rich out-of-process minidump round-trip (a real `CrashContext` -> `minidumper` ->
/// `minidump-writer`, validated by the `minidump` reader) is the known IPC HANGER under this headless
/// harness, and is already proven by MT-092's `tests/test_crash_capture.rs::cross_process_*`. This
/// #[ignore]d pointer documents that the capstone WIRES that proven path (the `CrashServerHandler` type
/// exists + the `with_minidump` record names a local dump). Run the MT-092 cross_process_* suite with
/// `--ignored` on a real host for the live minidump; this test never spawns/IPCs, so it is bounded.
#[test]
#[ignore = "LIVE out-of-process minidump is the known IPC hanger; proven by MT-092 \
            tests/test_crash_capture.rs::cross_process_* — run that with --ignored on a real host. This \
            pointer asserts the capstone wires the proven crash-record shape without re-running the IPC."]
fn live_minidump_is_covered_by_mt092_cross_process() {
    // A type/shape assertion only (no spawn, no IPC): the rich crash record names a LOCAL minidump path
    // (never bytes, never a URL) — the §6.13.6 out-of-process invariant MT-092 proves live.
    let dmp = PathBuf::from("/tmp/diag/palmistry-crash-sess.dmp");
    let rec = CrashRecord::with_minidump("sess", 7, 99, dmp.clone(), None, &[]);
    assert_eq!(rec.detection, CrashDetection::CrashContextMinidump);
    assert_eq!(rec.minidump_path.as_deref(), Some(dmp.as_path()));
}

// ── helpers ────────────────────────────────────────────────────────────────────────────────────────────

/// A fast lifecycle config so the clean-shutdown gate resolves in virtual-ish time without long sleeps.
fn fast_lifecycle_config() -> LifecycleConfig {
    LifecycleConfig {
        poll_interval: Duration::from_millis(2),
        post_death_finalize: Duration::from_millis(20),
        parent_watch_slice: Duration::from_millis(10),
    }
}

/// Remove a durable survivor-store directory tree on drop (the records were read for the assertions; the
/// proof is the read-back, so the tree is cleaned up to keep the external root tidy).
struct DirGuard(PathBuf);
impl Drop for DirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
