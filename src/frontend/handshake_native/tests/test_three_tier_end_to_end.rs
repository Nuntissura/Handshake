//! WP-KERNEL-012 MT-096 (G2 end-to-end CAPSTONE) — the integrated three-tier diagnostic proof on the
//! REAL binaries (Master Spec v02.196 §5.8 + §6.13 + §10.12.5; HBR-INT-009).
//!
//! This is the WP's INTEGRATION gate: it WIRES the tiers built in MT-081..095 and proves them as a WHOLE,
//! it does NOT re-implement any tier. It runs the four §6.13 incident scenarios and asserts each on the
//! real runtime, then emits a proof manifest + a Diagnostics-Panel proof.
//!
//! # The two-crate split (why this is the handshake-native HALF)
//!
//! handshake-native (the Tier-2 writer) and `palmistry` (the Tier-3 reader) are SIBLING crates with NO
//! dependency edge — the ONLY shared crate is `handshake-diag-ring` (the ring substrate, compiled
//! identically into both). So the integrated proof MEETS AT THE RING CONTRACT and has two halves:
//!
//! - THIS file (handshake-native side) drives the REAL `HandshakeApp` through egui_kittest's `build_eframe`
//!   path — the SAME `eframe::App::update` loop the shipped binary runs — and proves: the Tier-2 writer
//!   publishes an ADVANCING-then-STALE heartbeat into the REAL MT-081 ring that a ZERO-COOPERATION reader
//!   observes (the FREEZE write-side, SCENARIO 1); the app stays RESPONSIVE with the backend DOWN (the
//!   2026-06-26 freeze does NOT recur, SCENARIO 3); the Diagnostics Panel renders live and is asserted
//!   through AccessKit by default (SCENARIO 5; PNG gated behind `wgpu_screenshots`); and it emits the
//!   whole-WP proof manifest.
//! - `palmistry/tests/test_end_to_end_support.rs` drives the REAL `palmistry` types (ring reader MT-090,
//!   double-signal freeze detector MT-091, crash record MT-092, durable survivor store + FR forwarder
//!   MT-093, lifecycle MT-089) against a ring written EXACTLY as Handshake writes it, proving the Tier-3
//!   READ -> DETECT -> SURVIVE -> RECORD half (capture + survive + record for FREEZE and CRASH).
//!
//! Together the default halves prove the deterministic ring-contract integration. The real spawned
//! palmistry.exe freeze/crash capture remains an explicit real-host gate and is not labeled PASS here.
//!
//! # Honest gating (AC-016-6) + bounded tests (packet `palmistry_test_bound_policy`)
//!
//! The FR-forward LIVE round-trip (MT-093) needs a managed PostgreSQL/backend; it is gated and recorded
//! `NEEDS_MANAGED_RESOURCE_PROOF` (the palmistry support test proves it returns a typed blocker, not a
//! faked success). The LIVE cross-process spawn of the real `palmistry.exe` (the known IPC hazard) is the
//! ONLY test here that spawns a child; it is hard-bounded AND `#[ignore]`d (run with `--include-ignored`
//! on a real host after `cargo build -p palmistry`). Every default-run proof here is in-process + bounded.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_diag_ring::{
    DiagEventCode, DiagRingReader, DiagTier, Heartbeat, ThreeTierDiagnosticWiringRecord, TierWiring,
};
use handshake_native::app::HandshakeApp;
use handshake_native::backend_client::BACKEND_CONNECT_TIMEOUT;
use handshake_native::diagnostics::{self, BUFFER_CAP, DIAGNOSTICS_PANEL_AUTHOR_ID};

// ── artifact root + hygiene (CX-212E / the SCREENSHOT-TEST-ARTIFACT rule) ───────────────────────────────

/// The MT-096 external artifact leaf under the disk-agnostic `Handshake_Artifacts/handshake-test` root.
const MT096_ARTIFACT_SUBDIR: &str = "wp-kernel-012-mt-096";

/// The crate-relative EXTERNAL artifacts root (CX-212E), disk-agnostic: the crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is a
/// sibling of the repo worktree (the SAME convention `test_code_editor_panel.rs` / `test_keymap.rs` use).
/// The manifest, optional screenshot, and three-tier evidence land HERE — never repo-local.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Fail if a repo-local `test_output/` OR `tests/screenshots/` dir exists — artifacts go to the EXTERNAL
/// root ONLY (CX-212E). A tracked artifact under `src/` is a hygiene FAILURE the reviewer also catches
/// with `git ls-files "src/**/*.png"`.
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

// ── live-app + live-ring helpers ────────────────────────────────────────────────────────────────────────

/// The process-global LIVE ring path: the backing file the FIRST `HandshakeApp::new` in this test binary
/// installed onto the process-global diagnostics recorder. EVERY later `HandshakeApp`'s `bump_heartbeat`
/// writes the SAME global ring (the recorder is a one-shot `OnceLock`), so this single memoized path is
/// the live ring regardless of which test constructs an app — the freeze scenario reads it back. `None`
/// only if no app ever installed a ring (impossible once any app is built in this binary).
fn live_ring_path() -> &'static std::sync::Mutex<Option<PathBuf>> {
    static LIVE: std::sync::OnceLock<std::sync::Mutex<Option<PathBuf>>> =
        std::sync::OnceLock::new();
    LIVE.get_or_init(|| std::sync::Mutex::new(None))
}

/// Record this app's installed ring path into the process-global memo if it has not been set yet. Only the
/// app whose construction WON the one-shot install carries `diag_session()`; the first such app sets the
/// memo and every later heartbeat (from any app) writes that same ring.
fn memoize_ring_from(harness: &Harness<'_, HandshakeApp>) {
    if let Some(session) = harness.state().diag_session() {
        let mut slot = live_ring_path().lock().unwrap_or_else(|p| p.into_inner());
        if slot.is_none() {
            *slot = Some(session.ring_path.clone());
        }
    }
}

/// The memoized live ring path (the global recorder's installed ring), if any app installed one.
fn memoized_ring_path() -> Option<PathBuf> {
    live_ring_path()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

/// SERIALIZE every test that constructs + STEPS a `HandshakeApp`. Each app writes the MT-084 heartbeat
/// into the SAME process-global ring, so two tests stepping concurrently would interleave heartbeat
/// writes — and the freeze scenario (which proves the heartbeat STOPS advancing once its frame loop
/// stalls) would see another test's heartbeat still advancing the shared ring. Holding this lock for the
/// whole body of each app-driving test makes the ring quiescent for the one test driving it (and also
/// serializes the single wgpu device, the documented Windows concurrent-device hazard). A poisoned lock
/// is recovered so a panicking test never wedges the rest.
fn app_drive_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

// ── SCENARIO 1 (FREEZE end-to-end, Tier-2 write-side): real app -> advancing-then-stale ring heartbeat ──

/// SCENARIO 1 (FREEZE), the handshake-native half. Drive the REAL `HandshakeApp` frame loop so the MT-084
/// heartbeat advances into the REAL MT-081 ring, then STALL the loop (stop stepping — a frozen frame loop)
/// and assert a SEPARATE zero-cooperation `DiagRingReader` observes the heartbeat go STALE while the
/// last-N typed events stay readable. This is the freeze write-side + the §6.13.4 zero-cooperation
/// observability the Tier-3 detector (proven on the palmistry side) keys on — the two halves meet at this
/// real ring. (Why "stop stepping" is a faithful freeze: see `diagnostics::test_seams::FREEZE_MODEL_DOC`.)
#[test]
fn scenario1_freeze_real_app_heartbeat_goes_stale_zero_cooperation() {
    let _drive = app_drive_lock(); // quiesce the shared ring: no other app may step during the freeze.
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    memoize_ring_from(&harness);

    // The in-process heartbeat oracle (always available): the frame counter advances as frames run.
    let counter_before = harness.state().frame_counter();
    let advance: u64 = 12;
    for _ in 0..advance {
        harness.step();
    }
    let counter_after = harness.state().frame_counter();
    assert_eq!(
        counter_after - counter_before,
        advance,
        "the MT-084 UI-thread heartbeat (frame counter) advanced by N over N frames (the app is alive)"
    );

    // The zero-cooperation RING observability: a SEPARATE reader maps the live ring Handshake wrote and
    // observes the heartbeat. If no ring was installed (a degenerate headless install failure), the
    // in-process oracle above still proves the write-side; the ring read is the integration touchpoint.
    let ring_path = memoized_ring_path().expect(
        "the live HandshakeApp must have installed an MT-081 ring (Tier-2 -> Palmistry visible)",
    );
    let reader = DiagRingReader::open(&ring_path)
        .expect("a zero-cooperation reader maps the SAME backing file Handshake wrote");

    // The reader observes a real, advancing heartbeat (the writer published it from the frame loop).
    let hb_running = wait_for_heartbeat(&reader)
        .expect("the live app published a readable heartbeat into the ring");
    assert!(
        hb_running.counter > 0,
        "the zero-cooperation reader observes the live heartbeat counter advancing (got {})",
        hb_running.counter
    );

    // FREEZE: stop stepping the frame loop. The heartbeat counter now stops advancing — the exact stale
    // signal Palmistry's detector confirms. Read twice across a real gap WITHOUT stepping: the counter
    // does NOT move (the writer is "frozen"). The ring stays mapped + readable (zero cooperation).
    let frozen_at = reader
        .read_heartbeat()
        .expect("heartbeat readable at the freeze instant");
    std::thread::sleep(Duration::from_millis(120));
    let still_frozen = reader
        .read_heartbeat()
        .expect("the stale heartbeat stays readable zero-coop");
    assert_eq!(
        frozen_at.counter, still_frozen.counter,
        "AC-016-1: with the frame loop stalled the heartbeat counter STOPS advancing — the stale signal \
         a zero-cooperation reader observes (the freeze the Tier-3 detector confirms)"
    );

    // The last-N typed events the writer published before the freeze stay readable zero-coop (the evidence
    // bundle Palmistry captures). They are POD integers — never text (the typed-allowlist by construction).
    let events = reader.read_last_n(8);
    assert!(
        !events.is_empty(),
        "the last-N typed events published before the freeze stay readable zero-cooperation"
    );

    drop(reader);
    assert_no_local_artifact_dir();
}

/// Step the live app until the ring carries a readable heartbeat (bounded); returns it. The first frames
/// after construction set up wgpu/fonts, so a couple of steps may precede the first published heartbeat.
fn wait_for_heartbeat(reader: &DiagRingReader) -> Option<Heartbeat> {
    for _ in 0..50 {
        if let Some(hb) = reader.read_heartbeat() {
            if hb.counter > 0 {
                return Some(hb);
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    reader.read_heartbeat()
}

fn count_backend_unreachable_events() -> usize {
    diagnostics::snapshot_last_n(BUFFER_CAP)
        .iter()
        .filter(|e| e.event_code == DiagEventCode::BackendUnreachable.as_u16())
        .count()
}

// ── SCENARIO 3 (BACKEND-DOWN re-prove, MT-088 at the integrated level): real app stays RESPONSIVE ───────

/// SCENARIO 3. Drive the REAL `HandshakeApp` with the backend DOWN (a genuinely connection-refusing dead
/// port — NOT a mock) and assert it stays RESPONSIVE: every frame completes far below the connect timeout
/// AND the MT-084 heartbeat advances by exactly N over N frames. This re-proves at the INTEGRATED level
/// that the 2026-06-26 freeze (a UI-thread `block_on` on an unreachable backend) does NOT recur. Mirrors
/// the MT-088 deliverable proof, run inside the capstone.
#[test]
fn scenario3_backend_down_real_app_stays_responsive() {
    const DEAD_BACKEND_URL: &str = "http://127.0.0.1:1"; // port 1 on loopback: connection reliably refused.

    let _drive = app_drive_lock();
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    memoize_ring_from(&harness);
    let unreachable_before = count_backend_unreachable_events();
    harness
        .state_mut()
        .set_backend_unreachable_for_test(DEAD_BACKEND_URL);

    let frame_budget = Duration::from_millis(1000);
    assert!(
        frame_budget < BACKEND_CONNECT_TIMEOUT,
        "the per-frame responsiveness budget ({frame_budget:?}) must be below the backend connect timeout \
         ({BACKEND_CONNECT_TIMEOUT:?}) — a blocked frame would take at least the connect timeout"
    );

    let mut worst_frame = Duration::ZERO;
    let deadline = Instant::now() + Duration::from_secs(20);
    while !harness.state().backend_is_down() && Instant::now() < deadline {
        let t0 = Instant::now();
        harness.step();
        let dt = t0.elapsed();
        worst_frame = worst_frame.max(dt);
        assert!(
            dt < frame_budget,
            "AC-016-3: backend-down detection frame took {dt:?} — a responsive frame must complete well under the connect \
             timeout ({frame_budget:?}); a frame near/above it means a UI-thread backend call is blocking \
             the frame loop (the 2026-06-26 freeze)"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        harness.state().backend_is_down(),
        "AC-016-3: the real app must enter the degraded backend-down state against {DEAD_BACKEND_URL}"
    );
    let unreachable_after = count_backend_unreachable_events();
    assert_eq!(
        unreachable_after - unreachable_before,
        1,
        "AC-016-3: exactly one typed BackendUnreachable event must be recorded on the down edge"
    );

    let indicator = harness.state().status_bar_health_text();
    assert!(
        indicator.contains("Disconnected"),
        "AC-016-3: the status bar must show the finite degraded backend state, got {indicator:?}"
    );

    let counter_before = harness.state().frame_counter();
    let n: u64 = 30;
    for i in 0..n {
        let t0 = Instant::now();
        harness.step();
        let dt = t0.elapsed();
        worst_frame = worst_frame.max(dt);
        assert!(
            dt < frame_budget,
            "AC-016-3: steady degraded frame {i} took {dt:?} — a responsive frame must complete well under \
             the connect timeout ({frame_budget:?})"
        );
    }
    let counter_after = harness.state().frame_counter();
    assert_eq!(
        counter_after - counter_before,
        n,
        "AC-016-3: the heartbeat advanced by exactly N over N frames with the backend DOWN — the UI thread \
         is never stalled (the 2026-06-26 CPU->0 freeze does NOT recur). Worst frame was {worst_frame:?}."
    );

    assert_no_local_artifact_dir();
}

// ── SCENARIO 5 (PROOF MANIFEST + whole-WP three-tier evidence + Diagnostics-Panel proof) ───────────────

/// One scenario's verdict line in the proof manifest.
#[derive(serde::Serialize)]
struct ScenarioVerdict {
    id: &'static str,
    name: &'static str,
    verdict: &'static str,
    proof: &'static str,
}

/// The MT-096 end-to-end PROOF MANIFEST (AC-016-5). Lists each scenario verdict, the whole-WP
/// `ThreeTierDiagnosticWiringRecord` (MT-095 format, all three tiers accounted), the artifacts produced,
/// and the HONEST open gates. Emitted to the external root.
#[derive(serde::Serialize)]
struct ProofManifest {
    schema_version: &'static str,
    wp_id: &'static str,
    mt_id: &'static str,
    generated_at: String,
    scenarios: Vec<ScenarioVerdict>,
    three_tier_wiring: ThreeTierDiagnosticWiringRecord,
    open_gates: Vec<&'static str>,
    artifacts: Vec<String>,
    screenshot: Option<String>,
}

/// SCENARIO 5. Emit the whole-WP three-tier evidence record + the end-to-end proof manifest to the
/// EXTERNAL root, and prove the live Diagnostics Panel (Settings -> Diagnostics) shows heartbeat/frame/
/// resource state. Default runs assert the panel through AccessKit and record the PNG as an open gate;
/// `wgpu_screenshots` runs also save a PNG for visual inspection.
#[test]
fn scenario5_proof_manifest_and_diagnostics_panel_screenshot() {
    let _drive = app_drive_lock(); // serializes the app + the single wgpu device.
    let out_dir = external_artifact_dir(MT096_ARTIFACT_SUBDIR);
    std::fs::create_dir_all(&out_dir).expect("create the external MT-096 artifact dir");

    // (1) The whole-WP three-tier wiring record (MT-095 format) — all three tiers accounted with the
    // honest FR-forward gating. Emitted as the canonical evidence file too.
    let wiring = whole_wp_three_tier_record();
    wiring
        .validate()
        .expect("the whole-WP three-tier record is well-formed HBR-INT-009 evidence");
    let evidence_path = wiring
        .emit(&out_dir)
        .expect("emit the whole-WP three-tier evidence file to the external root");

    // (2) The Diagnostics-Panel proof (live heartbeat/frame/resource). Reuses the MT-087 surfacing path
    // on the REAL app. Default runs are AccessKit-only; the optional feature saves a PNG.
    let screenshot = capture_diagnostics_panel_screenshot(&out_dir);

    // (3) The end-to-end proof manifest naming each scenario verdict + artifacts + honest gating.
    let mut open_gates = vec![
        "FR-forward LIVE round-trip (MT-093 §6.13.7): needs a managed PostgreSQL/backend on \
         127.0.0.1:37501; gated requires_pg; the kept-as-is route returns a typed blocker (AC-016-6).",
        "LIVE cross-process real palmistry.exe freeze/crash CAPTURE: still required for AC-016-1/2; \
         run a freshly built binary via HANDSHAKE_PALMISTRY_EXE and the ignored real-host proof, then \
         update this manifest from the emitted artifact paths.",
    ];
    if screenshot.is_none() {
        open_gates.push(
            "Diagnostics Panel PNG: default proof is AccessKit-only to avoid the Windows wgpu concurrency \
             hazard; run with --features wgpu_screenshots for a fresh PNG and visually inspect it.",
        );
    }

    let manifest = ProofManifest {
        schema_version: "hsk.mt096.end_to_end_proof_manifest@0.1",
        wp_id: "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
        mt_id: "MT-096",
        generated_at: handshake_diag_ring::run_at_now(),
        scenarios: vec![
            ScenarioVerdict {
                id: "AC-016-1",
                name: "FREEZE end-to-end",
                verdict: "NEEDS_REAL_HOST_PROOF",
                proof: "default tests prove the real-app write-side heartbeat becomes stale on the live ring \
                        and the Palmistry detector/store half records freeze on a real ring; the spawned \
                        palmistry.exe freeze-capture run is still a required real-host proof and is not \
                        counted as PASS here",
            },
            ScenarioVerdict {
                id: "AC-016-2",
                name: "CRASH end-to-end",
                verdict: "NEEDS_REAL_HOST_PROOF",
                proof: "default tests prove the post-mortem floor crash record and clean-shutdown-no-crash \
                        gate; a real spawned app crash with out-of-process minidump capture/validation is \
                        still a required real-host proof and is not counted as PASS here",
            },
            ScenarioVerdict {
                id: "AC-016-3",
                name: "BACKEND-DOWN re-prove (2026-06-26 does not recur)",
                verdict: "PASS",
                proof: "scenario3_backend_down_real_app_stays_responsive (real app, dead backend, \
                        frames bounded + heartbeat advances)",
            },
            ScenarioVerdict {
                id: "AC-016-4",
                name: "TYPED-ALLOWLIST system-wide",
                verdict: "REPRESENTATIVE_PASS_NEEDS_EMITTED_ARTIFACT_SCAN",
                proof: "test_three_tier_privacy_allowlist scans ring records plus representative \
                        survivor/crash/forward telemetry and opportunistically scans emitted survivors; \
                        full PASS requires mandatory nonzero emitted freeze/crash/forward artifact paths \
                        from the real-host capstone",
            },
            ScenarioVerdict {
                id: "AC-016-6",
                name: "HONEST GATING (FR-forward live half)",
                verdict: "NEEDS_MANAGED_RESOURCE_PROOF",
                proof: "palmistry fr_forward_live_half_is_an_honest_typed_blocker_not_faked (typed blocker, \
                        not faked); live round-trip needs managed PostgreSQL/backend",
            },
        ],
        three_tier_wiring: wiring,
        open_gates,
        artifacts: vec![
            path_display(&evidence_path),
            screenshot
                .clone()
                .unwrap_or_else(default_screenshot_artifact_status),
        ],
        screenshot: screenshot.clone(),
    };

    let manifest_path = out_dir.join("three_tier_end_to_end_proof_manifest.json");
    let json = serde_json::to_string_pretty(&manifest).expect("serialize the proof manifest");
    std::fs::write(&manifest_path, format!("{json}\n"))
        .expect("write the proof manifest externally");
    assert!(
        manifest_path.exists(),
        "the proof manifest was written to the external root"
    );
    println!(
        "MT-096 proof manifest: {}",
        std::fs::canonicalize(&manifest_path)
            .unwrap_or(manifest_path.clone())
            .display()
    );
    if let Some(shot) = &screenshot {
        println!("MT-096 Diagnostics-Panel screenshot: {shot}");
    }

    assert_no_local_artifact_dir();
}

/// The whole-WP `ThreeTierDiagnosticWiringRecord` (MT-095 format) for the three-tier diagnostic system:
/// all three tiers accounted exactly once, with proof_refs to the proving MTs and an HONEST DEFERRED for
/// the FR-forward live round-trip that needs a managed backend (NEEDS_MANAGED_RESOURCE_PROOF, AC-016-6).
fn whole_wp_three_tier_record() -> ThreeTierDiagnosticWiringRecord {
    ThreeTierDiagnosticWiringRecord::new(
        "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1",
        "MT-096",
        "three_tier_diagnostic_system_end_to_end",
        handshake_diag_ring::run_at_now(),
        vec![
            // Tier 1: the FR-forward recovery path is wired (MT-093) but the LIVE round-trip needs a
            // managed backend — DEFERRED honestly, never faked (AC-016-6).
            TierWiring::deferred(
                DiagTier::FlightRecorder,
                "FR-forward recovery path is wired (MT-093) and returns an HONEST typed blocker against \
                 the kept-as-is route; the LIVE round-trip needs managed PostgreSQL/backend \
                 (NEEDS_MANAGED_RESOURCE_PROOF, gated requires_pg) — proven by palmistry \
                 fr_forward_live_half_is_an_honest_typed_blocker_not_faked",
            ),
            // Tier 2: internal_diagnostics proven end-to-end on the real app (heartbeat, backend-down, panel).
            TierWiring::wired(
                DiagTier::InternalDiagnostics,
                "MT-096 scenario1 (real-app heartbeat -> stale ring, zero-coop) + scenario3 (backend-down \
                 responsive, 2026-06-26 re-prove) + scenario5 (live Diagnostics Panel AccessKit proof; \
                 PNG screenshot gated behind wgpu_screenshots); MT-084/088/087",
            ),
            // Tier 3: Palmistry proven end-to-end on the real ring (detect+survive+record freeze & crash).
            TierWiring::wired(
                DiagTier::Palmistry,
                "palmistry test_end_to_end_support: freeze detect+survive+record + crash floor record + \
                 clean-shutdown gate on a REAL ring; MT-089/090/091/092/093/094; real-host \
                 cross-process minidump capture remains an ignored gate",
            ),
        ],
    )
}

#[cfg(feature = "wgpu_screenshots")]
fn default_screenshot_artifact_status() -> String {
    "(screenshot: render unavailable/no GPU adapter; AccessKit panel proof passed)".to_string()
}

#[cfg(not(feature = "wgpu_screenshots"))]
fn default_screenshot_artifact_status() -> String {
    "(screenshot: PNG gated behind --features wgpu_screenshots; AccessKit panel proof passed)"
        .to_string()
}

fn drive_diagnostics_panel(harness: &mut Harness<HandshakeApp>) {
    // Surface the Diagnostics section the same deterministic way MT-087 does: run a few live frames so the
    // heartbeat/frame/resource state is real, open Settings, and filter to the Diagnostics section.
    harness.run_steps(4);
    harness.state_mut().open_settings();
    harness.step();
    let search = harness.get_by_label("Search settings");
    search.focus();
    harness.step();
    harness
        .get_by_label("Search settings")
        .type_text("diagnostics");
    harness.run_steps(3);

    // The REAL diagnostics panel container must be present in the live tree (not a placeholder).
    let panel_present = harness
        .root()
        .children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(DIAGNOSTICS_PANEL_AUTHOR_ID));
    assert!(
        panel_present,
        "AC-016-5: the live Diagnostics Panel ('{DIAGNOSTICS_PANEL_AUTHOR_ID}') must render before the \
         screenshot"
    );
}

struct NoopSettingsTransport;

impl handshake_native::workspace_settings::SettingsTransport for NoopSettingsTransport {
    fn load(
        &self,
        _workspace_id: &str,
    ) -> Result<
        Option<serde_json::Value>,
        handshake_native::workspace_settings::SettingsTransportError,
    > {
        Ok(None)
    }

    fn save(
        &self,
        _workspace_id: &str,
        _settings_state: serde_json::Value,
    ) -> Result<(), handshake_native::workspace_settings::SettingsTransportError> {
        Ok(())
    }
}

fn install_noop_settings_transport(harness: &mut Harness<HandshakeApp>) {
    harness
        .state_mut()
        .set_settings_transport(std::sync::Arc::new(NoopSettingsTransport));
}

/// Drive the live app, surface Settings -> Diagnostics (the MT-087 path), and, when explicitly enabled,
/// save a wgpu screenshot of the live panel to `out_dir`. The default MT-096 run stays AccessKit-only so
/// it does not add another real-GPU test binary to the known Windows wgpu concurrency hazard.
#[cfg(feature = "wgpu_screenshots")]
fn capture_diagnostics_panel_screenshot(out_dir: &Path) -> Option<String> {
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(900.0, 800.0))
        .wgpu()
        .build_eframe(|cc| HandshakeApp::new(cc));
    memoize_ring_from(&harness);
    install_noop_settings_transport(&mut harness);
    drive_diagnostics_panel(&mut harness);

    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(
                w > 0 && h > 0,
                "the rendered Diagnostics-Panel image is non-empty"
            );
            let png_path = out_dir.join("MT-096-diagnostics-panel-live.png");
            let saved = image.save(&png_path).is_ok();
            assert!(
                saved,
                "AC-016-5: the Diagnostics-Panel screenshot PNG saved to the external root"
            );
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path);
            println!(
                "PT-016-D Diagnostics-Panel screenshot: {w}x{h} -> {}",
                abs.display()
            );
            Some(abs.display().to_string())
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-096 Diagnostics-Panel screenshot render unavailable (no wgpu \
                 adapter): {e}. The AccessKit real-panel proof passed; the PNG is a GPU-host item."
            );
            None
        }
    }
}

#[cfg(not(feature = "wgpu_screenshots"))]
fn capture_diagnostics_panel_screenshot(_out_dir: &Path) -> Option<String> {
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(900.0, 800.0))
        .build_eframe(|cc| HandshakeApp::new(cc));
    memoize_ring_from(&harness);
    install_noop_settings_transport(&mut harness);
    drive_diagnostics_panel(&mut harness);
    println!(
        "MT-096 Diagnostics-Panel PNG gated: rerun with --features wgpu_screenshots for the real-GPU \
         screenshot; default proof asserted the live AccessKit panel only."
    );
    None
}

// ── AC-016-7 (source review): the freeze/crash injection seams are feature-gated + not shipped ─────────

/// AC-016-7. The crash-injection seam (`test_seams::force_crash_abort` / `process::abort`) MUST be
/// feature-gated and unreachable in the shipped binary. Proven by a SOURCE review: the seam module is
/// declared under `#[cfg(any(test, feature = "diag-test-seams"))]`, and neither the production `main.rs`
/// nor `app.rs` references the crash trigger or the seam module. (The freeze scenario uses NO production
/// hook at all — see `test_seams::FREEZE_MODEL_DOC`.)
#[test]
fn freeze_crash_injection_seams_are_feature_gated_not_shipped() {
    let mod_src = include_str!("../src/diagnostics/mod.rs");
    let main_src = include_str!("../src/main.rs");
    let app_src = include_str!("../src/app.rs");

    // The seam module is feature/test gated (never compiled into a default/release build).
    assert!(
        mod_src.contains("#[cfg(any(test, feature = \"diag-test-seams\"))]")
            && mod_src.contains("pub mod test_seams;"),
        "AC-016-7: the test_seams module must be gated behind cfg(test)/feature = diag-test-seams"
    );

    // The production entrypoints must NOT reference the crash trigger or the seam module.
    for (label, src) in [("main.rs", main_src), ("app.rs", app_src)] {
        let code = strip_line_comments(src);
        for banned in [
            "test_seams",
            "force_crash_abort",
            "maybe_force_crash_from_env",
        ] {
            assert!(
                !code.contains(banned),
                "AC-016-7: production {label} must not reference the test-only seam '{banned}' \
                 (the crash trigger is unreachable in the shipped binary)"
            );
        }
        // A literal process::abort() must not appear in production code either (the only abort lives in
        // the feature-gated seam). `strip_line_comments` keeps the doc text out of the scan.
        assert!(
            !code.contains("process::abort()") && !code.contains("std::process::abort()"),
            "AC-016-7: production {label} must contain no process abort (the crash trigger is the seam only)"
        );
    }

    assert_no_local_artifact_dir();
}

// ── #[ignore]d LIVE cross-process: launch the REAL palmistry.exe against the capstone ring ─────────────

/// Resolve a built `palmistry` binary for the LIVE proof. MT-096 requires an explicit
/// `HANDSHAKE_PALMISTRY_EXE` so the real-host capstone cannot accidentally run a stale artifact from a
/// prior external target dir.
fn find_palmistry_binary() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os(diagnostics::ENV_PALMISTRY_EXE) {
        let p = PathBuf::from(raw);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// LIVE cross-process integration touchpoint (#[ignore]d — the known IPC hazard): launch the REAL
/// `palmistry.exe` against a REAL MT-081 ring (the capstone's ring contract) and prove the launched-with-
/// Handshake startup handshake ACKs over the MT-089 control socket, then a clean Shutdown reaps it with NO
/// crash record. This is the real-binary touchpoint for the whole three-tier system; the freeze/crash
/// CAPTURE round-trip is proven by the palmistry support test (real types on a real ring) + MT-092/094.
/// Every wait is hard-bounded; run with `cargo build -p palmistry` then `cargo test ... -- --include-ignored`.
#[test]
#[ignore = "LIVE cross-process: needs a built palmistry binary + spawns a child (the known IPC hazard). \
            #[ignore]d so a default `cargo test` never spawns palmistry. Build palmistry, set \
            HANDSHAKE_PALMISTRY_EXE to that fresh binary, then run with `-- --include-ignored` on a real \
            host; reaching here with no binary is a HARD FAIL, never a silent skip."]
fn live_real_palmistry_launched_with_handshake_on_capstone_ring() {
    use handshake_diag_ring::{DiagRingWriter, DEFAULT_CAPACITY};
    use handshake_native::diagnostics::{
        control_socket_name, launch_palmistry_at, DiagSession, ShutdownOutcome,
    };

    let _drive = app_drive_lock();
    assert_no_local_artifact_dir();
    let exe = find_palmistry_binary().unwrap_or_else(|| {
        panic!(
            "MT-096 LIVE proof requires a built palmistry binary; build `cargo build -p palmistry` or set \
             {} to the freshly built binary (this test is #[ignore]d — reaching here means it was \
             explicitly invoked, so a missing binary is a hard failure, never a silent skip).",
            diagnostics::ENV_PALMISTRY_EXE
        )
    });

    let dir = external_artifact_dir(MT096_ARTIFACT_SUBDIR).join("live");
    std::fs::create_dir_all(&dir).expect("create external live dir");
    let session_id = format!(
        "mt096-live-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let ring_path = dir.join(format!("ring-{session_id}.ring"));
    let writer =
        DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY).expect("create capstone ring");
    let now_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    writer.write_heartbeat(1, now_nanos);

    let session = DiagSession {
        session_id: session_id.clone(),
        ring_path: ring_path.clone(),
    };
    let control_socket = control_socket_name(&session_id);

    let mut handle = launch_palmistry_at(&exe, &session, &ring_path, &control_socket)
        .expect("MT-096 LIVE: launching the real palmistry binary must succeed (spawn ok)");
    assert!(
        handle.child_id() > 0,
        "a real palmistry.exe child must have spawned"
    );
    assert!(
        handle.handshake_acked(),
        "MT-096 LIVE: the launched-with-Handshake startup IPC must ACK over the MT-089 control socket"
    );

    // Advance the ring heartbeat a few times (the watcher is now reading the capstone ring).
    for c in 2..=6u64 {
        writer.write_heartbeat(c, now_nanos + c);
        std::thread::sleep(Duration::from_millis(10));
    }

    // Clean shutdown (bounded): the watcher exits cleanly with NO crash record (a clean shutdown is not a
    // crash — §6.13).
    let outcome = handle.request_shutdown_and_wait(Duration::from_secs(10));
    match outcome {
        ShutdownOutcome::ExitedCleanly(status) => {
            assert!(
                status.success(),
                "a clean Shutdown must make palmistry exit success (got {status:?})"
            )
        }
        other => panic!("MT-096 LIVE: palmistry must exit cleanly on Shutdown, got {other:?}"),
    }
    let crash_json = dir.join(format!("palmistry-crash-{session_id}.json"));
    assert!(
        !crash_json.exists(),
        "a clean shutdown must write NO crash record"
    );

    drop(writer);
    let _ = std::fs::remove_file(&ring_path);
    assert_no_local_artifact_dir();
}

// ── helpers ────────────────────────────────────────────────────────────────────────────────────────────

fn path_display(p: &Path) -> String {
    std::fs::canonicalize(p)
        .unwrap_or_else(|_| p.to_path_buf())
        .display()
        .to_string()
}

/// Strip `//` line comments so a source-review scan checks CODE, not the doc comments that legitimately
/// NAME the banned seam tokens. Conservative: cuts from the first `//` not inside a string literal.
fn strip_line_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let mut in_str = false;
        let mut prev = '\0';
        let chars: Vec<char> = line.chars().collect();
        let mut cut = chars.len();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' && prev != '\\' {
                in_str = !in_str;
            }
            if !in_str && c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                cut = i;
                break;
            }
            prev = c;
            i += 1;
        }
        out.extend(chars[..cut].iter());
        out.push('\n');
    }
    out
}
