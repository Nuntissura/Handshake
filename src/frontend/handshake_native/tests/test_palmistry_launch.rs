//! WP-KERNEL-012 MT-094 — Handshake LAUNCHES Palmistry + the startup IPC HANDSHAKE
//! (Master Spec v02.196 §6.13.3 "launched WITH Handshake at startup" + §6.13.2).
//!
//! Proof structure (the MT-094 anti-hang rules baked in — NOTHING here ever launches the real windowed
//! handshake-native binary or calls `eframe::run_native`; EVERY child/IPC wait is hard-bounded):
//!
//! - UNCONDITIONAL (always run): the `main()` WIRING (launch before `run_native`) is proven by a
//!   SOURCE-SCAN on `main.rs` (AC-014-1); HBR-QUIET spawn flags (CREATE_NO_WINDOW, no focus steal) +
//!   the not-kill-on-job-close commitment are proven by SOURCE-SCANs on the launcher (AC-014-2/3); the
//!   install_payload carries `palmistry.exe` (AC-014-6); and GRACEFUL DEGRADATION is proven by pointing
//!   the launcher at a MISSING binary and asserting it returns `None` WITHOUT panicking (AC-014-5).
//! - LIVE SPAWN PROOF (AC-014-1 + AC-014-4): when a built `palmistry` binary is resolvable (the
//!   `HANDSHAKE_PALMISTRY_EXE` env override or a discovered build output), the test calls
//!   `launch_palmistry_at()` DIRECTLY against the REAL binary and asserts (a) a real child process
//!   spawns, (b) the startup handshake ACKS over the MT-089 control socket, (c) a clean `Shutdown` reaps
//!   it with a SUCCESS exit and NO crash record written (a clean shutdown is not a crash). The real
//!   windowed binary is NEVER executed (it would hang the suite — the MT-094 HANG lesson). This live
//!   proof is `#[ignore]`d so a DEFAULT `cargo test` reports it as not-run (a visible "ignored", never a
//!   FALSE green that masks an un-run handshake proof); the governed lane MUST build palmistry
//!   (into `../../../../Handshake_Artifacts/handshake-cargo-target/debug`) and run it via
//!   `cargo test ... -- --include-ignored`. When the live test IS run but no `palmistry` binary is
//!   discoverable it HARD-FAILS (build `-p palmistry` or set `HANDSHAKE_PALMISTRY_EXE`) — it NEVER
//!   silently skips, so a passing live result is durable proof the real cross-process handshake ran.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use handshake_diag_ring::{DiagRingWriter, DEFAULT_CAPACITY};
use handshake_native::diagnostics::{
    self, control_socket_name, launch_palmistry_at, launch_palmistry_or_degrade, DiagSession,
    ShutdownOutcome, ENV_PALMISTRY_EXE,
};

// ── Source-of-truth scans (compile-time embeds; disk-agnostic, no cwd dependency) ────────────────────

const MAIN_RS: &str = include_str!("../src/main.rs");
const LAUNCHER_RS: &str = include_str!("../src/diagnostics/palmistry_launch.rs");
const CARGO_TOML: &str = include_str!("../Cargo.toml");

/// Strip full-line `//` / `///` / `//!` comments so a SOURCE-SCAN matches CODE, not prose. (The doc
/// comments deliberately NAME the APIs they ban / the ordering they require, which would false-positive a
/// naive `contains` scan.) Trailing inline comments are kept with their code line, but none of the
/// scanned tokens appear in a trailing inline comment in these files.
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The crate-relative external artifacts root (CX-212E), disk-agnostic: four `..` reach `<repo>/..`
/// where `Handshake_Artifacts` sits beside the repo worktree. The ring + any Palmistry sibling records
/// land HERE (never repo-local) — the same convention `test_code_editor_panel.rs` uses.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// CX-212E hygiene guard: NO repo-local artifact dir may exist under the crate (artifacts go external).
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

// ── AC-014-1 (wiring): main() launches Palmistry BEFORE eframe::run_native ───────────────────────────

#[test]
fn main_launches_palmistry_before_run_native() {
    let main_code = code_only(MAIN_RS);
    let launch_idx = main_code
        .find("launch_palmistry_or_degrade")
        .expect("AC-014-1: main.rs must call the Palmistry launcher");
    let run_idx = main_code
        .find("eframe::run_native")
        .expect("main.rs must call eframe::run_native");
    assert!(
        launch_idx < run_idx,
        "AC-014-1: the Palmistry launch MUST be wired BEFORE eframe::run_native (so the watcher is up \
         before the app can freeze, and so the source-scan order proves the wiring)"
    );
    // The ring is created in main() (so the kittest suite that builds HandshakeApp::new never spawns a
    // child) and reused by HandshakeApp::new via the preinstalled-session slot.
    assert!(
        MAIN_RS.contains("create_and_install_diag_ring")
            && MAIN_RS.contains("set_preinstalled_diag_session"),
        "AC-014-1: main() must create+install the ring and hand it to HandshakeApp via the preinstalled \
         slot (so new() does not create a second ring and the kittest suite never spawns palmistry)"
    );
    // The clean-shutdown path: HandshakeApp owns the handle (set_palmistry_handle) and sends Shutdown.
    assert!(
        MAIN_RS.contains("set_palmistry_handle"),
        "AC-014-4: the running shell must own the PalmistryHandle so on_exit sends Shutdown"
    );
}

// ── AC-014-2 (HBR-QUIET) + AC-014-3 (not kill-on-job-close): launcher source discipline ───────────────

#[test]
fn launcher_spawn_is_quiet_and_free_standing() {
    // Scan CODE only — the doc comments deliberately NAME the banned APIs they forbid, which would
    // false-positive a naive scan of the raw source.
    let launcher_code = code_only(LAUNCHER_RS);
    // HBR-QUIET (AC-014-2): CREATE_NO_WINDOW + null stdio, the SAME discipline as the MT-088 LSP spawn.
    assert!(
        launcher_code.contains("creation_flags") && launcher_code.contains("0x0800_0000"),
        "AC-014-2: the spawn must set CREATE_NO_WINDOW (0x0800_0000) so it never flashes a console"
    );
    assert!(
        launcher_code.contains("Stdio::null()"),
        "AC-014-2: stdio must be null (not inherited) so the watcher never steals the console"
    );
    // No focus-steal anywhere (also enforced crate-wide by tests/test_focus_audit_quiet.rs which scans
    // every src/**/*.rs — this file is covered there too).
    for banned in [
        "SetForegroundWindow",
        "BringWindowToTop",
        "AllowSetForegroundWindow",
    ] {
        assert!(
            !launcher_code.contains(banned),
            "AC-014-2: the launcher must not call the foreground/focus-steal API '{banned}'"
        );
    }
    // NOT kill-on-job-close (AC-014-3): the launcher adds NO Win32 Job Object membership — a plain
    // free-standing Command::spawn. Assert the job APIs are ABSENT and the commitment is greppable.
    for job_api in [
        "AssignProcessToJobObject",
        "SetInformationJobObject",
        "CreateJobObject",
        "JobObjectExtendedLimitInformation",
    ] {
        assert!(
            !launcher_code.contains(job_api),
            "AC-014-3: the launcher must NOT use the Win32 Job Object API '{job_api}' (a kill-on-close \
             job would terminate the watcher at the instant of parent death — the opposite of §6.13.3)"
        );
    }
    assert!(
        diagnostics::SPAWN_NOT_KILL_ON_JOB_CLOSE.contains("6.13.3"),
        "AC-014-3: the not-kill-on-job-close commitment must be greppable + name §6.13.3"
    );
    // Portability (AC-014-6): the exe resolves via current_exe()-relative sibling lookup (no hardcoded path).
    assert!(
        LAUNCHER_RS.contains("current_exe"),
        "AC-014-6: the palmistry binary must resolve relative to the running exe (portable, no hardcoded path)"
    );
}

// ── AC-014-6: install_payload ships palmistry.exe side-by-side ────────────────────────────────────────

#[test]
fn install_payload_includes_palmistry() {
    // The install metadata must list palmistry.exe so the installer ships it beside handshake-native.exe.
    let line = CARGO_TOML
        .lines()
        .find(|l| l.trim_start().starts_with("install_payload"))
        .expect("AC-014-6: Cargo.toml must declare install_payload");
    assert!(
        line.contains("palmistry.exe"),
        "AC-014-6: install_payload must include palmistry.exe (the installer ships it side-by-side): {line}"
    );
}

// ── AC-014-5 (graceful degradation): a missing palmistry.exe never blocks or crashes startup ──────────

#[test]
fn missing_palmistry_degrades_to_none_without_blocking() {
    assert_no_local_artifact_dir();
    // Point the launcher at a binary that does not exist. The production entrypoint
    // (launch_palmistry_or_degrade) must log + record a typed diagnostic and return None — Handshake
    // would then start anyway. This is the AC-014-5 graceful-degradation proof at the function boundary.
    let bogus = std::env::temp_dir().join("handshake-no-such-palmistry-mt094.exe");
    let _ = std::fs::remove_file(&bogus);

    let session = DiagSession {
        session_id: "mt094-degrade".to_string(),
        ring_path: std::env::temp_dir().join("mt094-degrade.ring"),
    };
    let control_socket = control_socket_name(&session.session_id);

    // Save + restore any pre-existing override so this test cannot disturb the live-spawn test's
    // binary discovery (which prefers the env override).
    let prev = std::env::var_os(ENV_PALMISTRY_EXE);
    std::env::set_var(ENV_PALMISTRY_EXE, &bogus);
    let started = std::time::Instant::now();
    let handle = launch_palmistry_or_degrade(&session, &control_socket);
    let elapsed = started.elapsed();
    match prev {
        Some(v) => std::env::set_var(ENV_PALMISTRY_EXE, v),
        None => std::env::remove_var(ENV_PALMISTRY_EXE),
    }

    assert!(
        handle.is_none(),
        "AC-014-5: a missing palmistry.exe must degrade to None (Handshake still starts), not block/crash"
    );
    // The spawn fails fast (no spawn -> no handshake wait), so degradation is near-instant: it must NOT
    // hang on the watcher (no startup stall reintroduced).
    assert!(
        elapsed < Duration::from_secs(2),
        "AC-014-5: graceful degradation must be prompt (a missing exe must not block startup); took {elapsed:?}"
    );
}

// ── AC-014-1 + AC-014-4 (LIVE): real spawn + handshake ack + clean shutdown + no crash record ─────────

/// Resolve a built `palmistry` binary for the LIVE proof: the `HANDSHAKE_PALMISTRY_EXE` override first
/// (what the build pipeline / coder sets), then the conventional external build output dirs Palmistry's
/// own `.cargo/config` targets. `None` => the live proof soft-skips (build `-p palmistry` to enable it).
fn find_palmistry_binary() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os(ENV_PALMISTRY_EXE) {
        let p = PathBuf::from(raw);
        if p.is_file() {
            return Some(p);
        }
    }
    let bin = if cfg!(windows) {
        "palmistry.exe"
    } else {
        "palmistry"
    };
    // MT-094 remediation: the discovery fallbacks previously pointed at
    // `Handshake_Artifacts/palmistry-target/{debug,release}`, a layout that NEVER existed — real
    // builds land in the shared `Handshake_Artifacts/handshake-cargo-target/{debug,release}` (the
    // repo-root + palmistry `.cargo/config.toml` target dir). Point at the REAL layout so the live
    // proof finds a built binary without HANDSHAKE_PALMISTRY_EXE.
    for base in [
        "../../../../Handshake_Artifacts/handshake-cargo-target/debug",
        "../../../../Handshake_Artifacts/handshake-cargo-target/release",
    ] {
        let p = Path::new(base).join(bin);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn unique_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("mt094-{}-{}", std::process::id(), nanos)
}

#[test]
#[ignore = "LIVE cross-process proof (AC-014-1/AC-014-4): needs a built palmistry binary, so it is \
            #[ignore]d to keep a default `cargo test` from reporting a FALSE green that masks an un-run \
            handshake proof. The governed lane builds `-p palmistry` then runs \
            `cargo test ... -- --include-ignored`. Run without a discoverable binary => HARD FAIL, never \
            a silent skip."]
fn live_spawn_handshake_then_clean_shutdown_no_crash() {
    assert_no_local_artifact_dir();
    // HARD-FAIL (never soft-skip) once this live proof is actually invoked: if the live handshake was
    // asked for but no palmistry binary is discoverable, that is a proof-SETUP failure, not a pass. A
    // silent `return` here would let a green count mask the fact that the real HandshakeHello/Ack never
    // crossed the MT-089 socket — exactly the false-completion the #[ignore] gate + this assert prevent.
    let exe = find_palmistry_binary().unwrap_or_else(|| {
        panic!(
            "AC-014-1/AC-014-4 LIVE proof requires a built palmistry binary, but none was discoverable. \
             Build it (`cargo build -p palmistry`) or set {ENV_PALMISTRY_EXE} to the built binary, then \
             re-run with `-- --include-ignored`. (This test is #[ignore]d; reaching here means it was \
             explicitly invoked, so a missing binary is a hard failure — never a silent skip.)"
        )
    });

    // Real MT-081 ring (the file Palmistry maps) under the EXTERNAL artifact root, so Palmistry's sibling
    // survivor/crash records also land externally (CX-212E), never repo-local.
    let dir = external_artifact_dir("wp-kernel-012-mt-094");
    std::fs::create_dir_all(&dir).expect("create external artifact dir");
    let session_id = unique_session_id();
    let ring_path = dir.join(format!("ring-{session_id}.ring"));
    // Keep the writer alive for the whole test so the mapped ring file stays valid + carries a heartbeat
    // (Palmistry's initial passive liveness read finds a real value).
    let writer = DiagRingWriter::create(&ring_path, DEFAULT_CAPACITY).expect("create diag ring");
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

    // The Palmistry sibling record paths (clean shutdown must write NEITHER crash file).
    let crash_json = dir.join(format!("palmistry-crash-{session_id}.json"));
    let crash_dmp = dir.join(format!("palmistry-crash-{session_id}.dmp"));
    let survivor_json = dir.join(format!("palmistry-survivor-{session_id}.json"));
    for p in [&crash_json, &crash_dmp, &survivor_json] {
        let _ = std::fs::remove_file(p);
    }

    // (1) LAUNCH the REAL palmistry.exe via the FUNCTION (never the windowed app) + (2) complete the
    // startup HANDSHAKE — all bounded inside launch_palmistry_at.
    let mut handle = launch_palmistry_at(&exe, &session, &ring_path, &control_socket)
        .expect("AC-014-1: launching the real palmistry binary must succeed (spawn ok)");

    assert!(
        handle.child_id() > 0,
        "AC-014-1(a): a real palmistry.exe child process must have spawned"
    );
    assert!(
        handle.handshake_acked(),
        "AC-014-1(b): the startup IPC handshake must ACK over the MT-089 control socket (pid/session/ring)"
    );

    // (3) CLEAN SHUTDOWN: send the explicit Shutdown control message + reap (bounded). Palmistry must
    // exit cleanly (success) and write NO crash record (a clean shutdown is not a crash — §6.13).
    let outcome = handle.request_shutdown_and_wait(Duration::from_secs(10));
    match outcome {
        ShutdownOutcome::ExitedCleanly(status) => assert!(
            status.success(),
            "AC-014-4: a clean Shutdown must make palmistry exit with success (got {status:?})"
        ),
        other => panic!(
            "AC-014-4: palmistry must exit cleanly on Shutdown (not killed/timed-out); got {other:?}"
        ),
    }

    // NO crash record + NO minidump on a clean shutdown (the §6.13 clean-shutdown rule).
    assert!(
        !crash_json.exists(),
        "AC-014-4: a clean shutdown must write NO crash record ({} should not exist)",
        crash_json.display()
    );
    assert!(
        !crash_dmp.exists(),
        "AC-014-4: a clean shutdown must write NO minidump ({} should not exist)",
        crash_dmp.display()
    );
    // The survivor record (always written on a lifecycle end) must classify a CLEAN shutdown — not an
    // abnormal/crash exit. Substring assertion (dependency-free) over the pretty-JSON record.
    let survivor = std::fs::read_to_string(&survivor_json)
        .expect("AC-014-4: palmistry must persist a survivor record on shutdown");
    assert!(
        survivor.contains("CleanShutdown"),
        "AC-014-4: the survivor record must classify the exit as CleanShutdown; got: {survivor}"
    );
    assert!(
        survivor.contains("\"abnormal_parent_exit\": false"),
        "AC-014-4: the survivor record must record NO abnormal parent exit; got: {survivor}"
    );

    drop(writer);
    // Best-effort cleanup of this run's artifacts (the assertions already read them).
    for p in [&crash_json, &crash_dmp, &survivor_json, &ring_path] {
        let _ = std::fs::remove_file(p);
    }
    assert_no_local_artifact_dir();
}
