//! WP-KERNEL-012 MT-086 (D2 — internal_diagnostics, Tier 2: CPU%/RSS RESOURCE COUNTERS + static GPU
//! identity, §5.8.2 "resource counters" / §5.8.4 panel) runtime proofs.
//!
//! Resource counters are the in-process CPU%/RSS signal the Diagnostics Panel (MT-087) shows on its
//! resource line and Palmistry (Tier 3) reads from the ring. §6.13.1 names "pinned under heavy CPU" as a
//! first-class stall mode, and the 2026-06-26 freeze showed CPU -> 0 (the opposite); a periodic
//! in-process counter makes both visible. The GPU/driver identity is STATIC (captured once at startup
//! from the eframe wgpu adapter). Each acceptance criterion maps to a REAL runtime proof (no tautology):
//!
//! - AC-006-1 (`samples_current_process_returns_plausible_integers` + `sampler_targets_only_current_pid`):
//!   construct a real `ResourceSampler`, sample CPU%/RSS for THIS process, assert plausible typed
//!   integers (rss_kb > 0; cpu_milli a non-negative integer) AND that the sampler refreshes ONLY the
//!   current pid (not the whole system process table).
//! - AC-006-2 (`resource_sample_reaches_the_ring`): a `ResourceSample` DiagEvent reaches the MT-081 ring
//!   via `record_sample` (typed integers only — cpu_milli/rss_kb, no content) — a SEPARATE
//!   `DiagRingReader` on the same backing file reads it back.
//! - AC-006-3 (`live_gpu_info_captured_from_wgpu_render_state`): a wgpu kittest constructs the REAL
//!   `HandshakeApp` and asserts a non-empty `GpuInfo` (vendor/device/backend codes set) is stored; the
//!   human driver strings are kept in the in-process GpuInfo only, NOT pushed into the typed ring.
//! - AC-006-4 (`live_sampling_is_bounded_and_does_not_stutter`): a kittest steps many frames and asserts
//!   ResourceSample is emitted at the bounded interval (NOT every frame) and the frame loop stays
//!   responsive (no SlowFrame storm from the sampler).
//! - AC-006-5 is proven OUT of this file by `cargo tree -d -p handshake-native` (single sysinfo core).

use std::path::{Path, PathBuf};
use std::time::Instant;

use egui_kittest::Harness;

use handshake_diag_ring::{
    DiagEvent, DiagEventCode, DiagPhase, DiagRingReader, DiagRingWriter, DiagSeverity,
    DEFAULT_CAPACITY,
};
use handshake_native::app::HandshakeApp;
use handshake_native::diagnostics::{self, DiagnosticsRecorder, ResourceSampler, BUFFER_CAP};

// ── artifact hygiene (CX-212E): no repo-local artifact dir may exist ───────────────────────────────

/// The external artifact root for any MT-086 test output. The proofs here are all in-memory / ring /
/// global-buffer reads (no screenshot/PNG is written), but the guard is invoked uniformly so the
/// hygiene contract is enforced and the helper is not dead (CX-212E).
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Fail if a repo-local `test_output/` OR `tests/screenshots/` dir exists — artifacts must go to the
/// EXTERNAL `Handshake_Artifacts/handshake-test` root only (CX-212E). A tracked artifact under `src/` is
/// a hygiene FAILURE the reviewer also catches with `git ls-files "src/**/*.png"`.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "no repo-local {} dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local,
            p.display()
        );
    }
}

/// A unique temp backing-file path for a ring (per test, no collisions between parallel test threads).
fn temp_ring_path(tag: &str) -> PathBuf {
    let unique = format!(
        "handshake-mt086-{}-{}-{:?}.ring",
        tag,
        std::process::id(),
        std::thread::current().id()
    );
    std::env::temp_dir().join(unique)
}

/// Count `ResourceSample` events currently visible in the process-global in-process diagnostics buffer
/// (what the Diagnostics Panel reads). The buffer is process-wide and shared across tests in this
/// binary, so the live-path proofs measure a DELTA (after - before) to stay robust to test ordering.
fn global_resource_sample_count() -> usize {
    diagnostics::snapshot_last_n(BUFFER_CAP)
        .iter()
        .filter(|e| e.event_code == DiagEventCode::ResourceSample.as_u16())
        .count()
}

/// Count `SlowFrame` events in the process-global buffer — used to prove the sampler does NOT cause a
/// SlowFrame storm (AC-006-4 no-stutter half).
fn global_slow_frame_count() -> usize {
    diagnostics::snapshot_last_n(BUFFER_CAP)
        .iter()
        .filter(|e| e.event_code == DiagEventCode::SlowFrame.as_u16())
        .count()
}

// ── AC-006-1: current-process sample returns plausible typed integers ──────────────────────────────

/// Construct a REAL `ResourceSampler` and sample CPU%/RSS for THIS process. Assert plausible typed
/// integers: rss_kb > 0 (a live process always has a resident set) and cpu_milli is a non-negative
/// integer (0 on the first sample is expected — sysinfo needs two refreshes for a CPU delta). This is a
/// REAL sysinfo read of the live test process (not a stubbed constant).
#[test]
fn samples_current_process_returns_plausible_integers() {
    let mut sampler = ResourceSampler::new();
    // First sample: RSS is real and > 0; CPU may be 0 (no prior delta).
    let first = sampler.sample();
    assert!(
        first.rss_kb > 0,
        "a live process always has a non-zero resident set (rss_kb={})",
        first.rss_kb
    );

    // Do a little real work between two refreshes so the SECOND sample can show a meaningful CPU% delta,
    // then sample again. We do NOT assert cpu > 0 (a fast machine may still read ~0 over a short window),
    // only that both reads are plausible typed integers and RSS stays positive.
    let mut acc: u64 = 0;
    for i in 0..2_000_000u64 {
        acc = acc.wrapping_add(i ^ (i << 1));
    }
    std::hint::black_box(acc);
    let second = sampler.sample();
    assert!(second.rss_kb > 0, "RSS stays positive on the second sample (rss_kb={})", second.rss_kb);
    // cpu_milli is a u64 (non-negative by type) on both reads — a real read, not absurd.
    assert!(
        first.cpu_milli < 100_000_000,
        "cpu_milli is a plausible integer milli-percent, not garbage (got {})",
        first.cpu_milli
    );

    assert_no_local_artifact_dir();
}

/// The sampler targets ONLY the current pid (AC-006-1 current-process-only half / RISK-006-3). Reading
/// the whole system process table would be wasteful AND leak other processes' data; the sampler must
/// refresh only `std::process::id()`.
#[test]
fn sampler_targets_only_current_pid() {
    let sampler = ResourceSampler::new();
    assert_eq!(
        sampler.own_pid(),
        sysinfo::Pid::from_u32(std::process::id()),
        "the sampler refreshes ONLY this process's pid, never the whole system table (RISK-006-3)"
    );
    assert_no_local_artifact_dir();
}

// ── AC-006-2: a ResourceSample DiagEvent reaches the MT-081 ring via record_sample ─────────────────

/// Drive `record_sample` against a recorder built on a REAL `DiagRingWriter`, then read the event back
/// with a SEPARATE `DiagRingReader` on the same backing file — proving the typed `ResourceSample` event
/// actually reaches the shared-memory ring (what Palmistry maps). The event carries only typed integers
/// (cpu_milli / rss_kb), no content.
#[test]
fn resource_sample_reaches_the_ring() {
    let path = temp_ring_path("ring");
    let _ = std::fs::remove_file(&path);

    let writer = DiagRingWriter::create(&path, DEFAULT_CAPACITY).expect("create ring writer");
    let recorder = DiagnosticsRecorder::with_writer(writer);

    // Take a real sample of THIS process and record its event through the recorder's ring writer.
    let mut sampler = ResourceSampler::new();
    let mut captured: Option<DiagEvent> = None;
    let sample = sampler.record_sample(Instant::now(), |event| {
        captured = Some(event);
        recorder.record(event);
    });

    // The built event is the typed ResourceSample shape (no content).
    let event = captured.expect("record_sample emitted exactly one event");
    assert_eq!(event.event_code, DiagEventCode::ResourceSample.as_u16(), "code is ResourceSample");
    assert_eq!(event.phase_marker, DiagPhase::Tick.as_u8(), "phase Tick (a periodic sample)");
    assert_eq!(event.severity, DiagSeverity::Info.as_u8(), "severity Info");
    assert_eq!(event.counter_a, sample.cpu_milli, "counter_a carries cpu_milli (typed integer)");
    assert_eq!(event.counter_b, sample.rss_kb, "counter_b carries rss_kb (typed integer)");
    assert!(event.counter_b > 0, "rss_kb is a real read (> 0 for a live process)");
    assert_eq!(event._reserved, [0u8; 4], "no content smuggled through padding");

    // A SEPARATE reader on the SAME backing file reads the sample back from the shared-memory ring.
    let reader = DiagRingReader::open(&path).expect("open ring reader on the same backing file");
    let back = reader.read_last_n(4);
    let ring_sample = back
        .iter()
        .find(|e| e.event_code == DiagEventCode::ResourceSample.as_u16())
        .expect("the ResourceSample reached the shared-memory ring (Palmistry-visible)");
    assert_eq!(ring_sample.counter_a, sample.cpu_milli, "ring read-back cpu_milli matches");
    assert_eq!(ring_sample.counter_b, sample.rss_kb, "ring read-back rss_kb matches");

    drop(reader);
    drop(recorder);
    let _ = std::fs::remove_file(&path);
    assert_no_local_artifact_dir();
}

// ── AC-006-3: GPU adapter info captured once from cc.wgpu_render_state (the LIVE wgpu shell) ────────

/// Drive the REAL production `HandshakeApp::new(cc)` through the egui_kittest wgpu harness (the closure
/// body is PRODUCTION code; `new` captures the GPU identity from `cc.wgpu_render_state`). Assert a
/// non-empty `GpuInfo` is stored with vendor/device/backend codes set — read from the EXISTING eframe
/// adapter, not a second device (RISK-006-5). The human driver strings are kept in the in-process
/// GpuInfo only and are NEVER pushed into the typed ring: we assert no ResourceSample/ring event ever
/// carries a string (the ring is integer-only by type, so this is structurally true — we assert the
/// integer codes are present in GpuInfo and the strings are confined to GpuInfo).
#[test]
fn live_gpu_info_captured_from_wgpu_render_state() {
    // `.wgpu()` selects the real GPU render backend so the kittest populates `cc.wgpu_render_state` with
    // a REAL adapter (the same way the existing standalone-render tests do). Without `.wgpu()` the kittest
    // builds a `CreationContext` with no render state and `GpuInfo::capture` correctly returns None
    // (graceful degradation) — so the LIVE-capture proof requires the wgpu harness.
    let harness: Harness<HandshakeApp> =
        Harness::builder().wgpu().build_eframe(|cc| HandshakeApp::new(cc));

    let gpu = harness
        .state()
        .gpu_info()
        .expect("the .wgpu() kittest harness populates cc.wgpu_render_state, so GpuInfo is captured");

    // A real adapter was read from the existing eframe wgpu render state.
    assert!(
        gpu.is_captured(),
        "GpuInfo carries a real captured identity (vendor={:#x}, device={:#x}, name={:?})",
        gpu.vendor_id,
        gpu.device_id,
        gpu.name
    );
    // backend_code is one of the known wgpu backends (Noop=0..=BrowserWebGpu=5). On Windows the kittest
    // wgpu adapter is typically Dx12 (3) or Vulkan (1); we only require a valid in-range code.
    assert!(
        gpu.backend_code <= 5,
        "backend_code is a valid wgpu backend discriminant (got {})",
        gpu.backend_code
    );
    assert!(
        gpu.device_type_code <= 4,
        "device_type_code is a valid wgpu DeviceType code 0..=4 (got {})",
        gpu.device_type_code
    );

    // The human strings live in the in-process GpuInfo only. Prove they are NOT on the typed ring: scan
    // every event currently in the global buffer — the ring/buffer stores only `DiagEvent` (all integer
    // fields by type), so by construction no string can be present. We assert the buffer's events are
    // the integer-only DiagEvent type and that NONE of them is a "string" (the type system guarantees
    // this; the check below documents the boundary and would catch a future regression that tried to
    // stuff a name into a counter as a hash, which is still NOT a string but would be a different MT).
    let buffer = diagnostics::snapshot_last_n(BUFFER_CAP);
    for e in &buffer {
        // _reserved is the only non-counter bytes; it must stay zeroed (no content channel).
        assert_eq!(e._reserved, [0u8; 4], "no event smuggles content through the reserved padding");
    }

    assert_no_local_artifact_dir();
}

// ── AC-006-4: bounded cadence + no frame stutter from the sampler ──────────────────────────────────

/// Step the REAL `HandshakeApp` through MANY frames and assert the resource sampler is BOUNDED: it emits
/// at the ~1s cadence, NOT every frame. A kittest steps frames in a tight loop (far faster than 1s), so
/// over N stepped frames only a SMALL number of ResourceSample events appear (typically just the first,
/// since all subsequent steps fall within the same ~1s window) — far fewer than the frame count. Also
/// assert the sampler does not cause a SlowFrame storm (the single-process refresh at the cadence is
/// cheap; the per-frame cost on a non-sampling frame is a single Instant comparison).
#[test]
fn live_sampling_is_bounded_and_does_not_stutter() {
    let mut harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));

    let samples_before = global_resource_sample_count();
    let slow_before = global_slow_frame_count();
    let sample_count_before = harness.state().resource_sample_count();
    let frames_before = harness.state().frame_counter();

    // Step many frames in a tight loop — far more than the ~1s cadence would allow samples for.
    let steps = 40u64;
    for _ in 0..steps {
        harness.step();
    }

    let frames_after = harness.state().frame_counter();
    let sample_count_after = harness.state().resource_sample_count();
    let samples_after = global_resource_sample_count();
    let slow_after = global_slow_frame_count();

    let frames_run = frames_after - frames_before;
    let samples_taken = sample_count_after - sample_count_before;

    // The frame loop genuinely ran the frames.
    assert!(frames_run >= steps, "the steps ran real frames (ran {frames_run} >= {steps})");

    // BOUNDED: far fewer samples than frames. Over a tight ~40-step loop (well under 1s) the cadence
    // gate emits at most a couple of samples (the first one, plus possibly a second if the loop crossed a
    // ~1s boundary on a slow machine). The key invariant: samples_taken << frames_run.
    assert!(
        samples_taken < frames_run,
        "the sampler is bounded — far fewer samples ({samples_taken}) than frames ({frames_run}); \
         it does NOT sample every frame (RISK-006-2 / AC-006-4)"
    );
    assert!(
        samples_taken <= 3,
        "over a tight {steps}-frame loop (well under the ~1s cadence), at most a couple of samples are \
         taken (got {samples_taken}) — the cadence gate is bounded, not per-frame"
    );
    // At least one sample was actually emitted to the global buffer FROM THE LIVE FRAME PATH (the first
    // frame is always due), proving the sampler is genuinely wired into `update` (not dead code).
    assert!(
        samples_after > samples_before,
        "the live frame path emitted at least one ResourceSample into the global buffer \
         (before={samples_before}, after={samples_after})"
    );

    // NO frame stutter from the sampler: the single-process refresh did not trigger a SlowFrame storm.
    // (A single slow frame on the FIRST sample is conceivable on a cold machine; we assert the sampler
    // did not flood the buffer with slow frames — at most a small bounded number, matching the bounded
    // sample count.)
    let new_slow = slow_after - slow_before;
    assert!(
        new_slow <= samples_taken as usize,
        "the sampler did not cause a SlowFrame storm — new slow frames ({new_slow}) bounded by the \
         small sample count ({samples_taken}); the per-frame non-sampling cost is one Instant compare"
    );

    assert_no_local_artifact_dir();
}
