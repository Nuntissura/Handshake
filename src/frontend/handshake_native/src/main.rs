// WP-KERNEL-011 Handshake native GUI — binary entrypoint (thin; logic lives in the lib).
// Opens a real native wgpu window (no webview/Tauri/Electron) and runs the egui shell.

use handshake_native::app::HandshakeApp;
use handshake_native::diagnostics;
use handshake_native::installer;
use handshake_native::quiet_mode::focus_guard;

// WP-KERNEL-012 MT-092 (CLIENT side of §6.13.6 crash detection). handshake-native is the CLIENT in the
// Embark out-of-process crash pipeline; Palmistry (the separate watcher process) is the SERVER/writer.
//
// HOW THE CLIENT FINDS THE SOCKET (MT-092/MT-094 remediation): in PRODUCTION the crash-socket path is
// DERIVED, not injected — `diagnostics::crash_socket_path(control_socket)` mirrors palmistry's exact
// derivation rule (pinned equal by a cross-crate wire test), and `main()` arms the client AFTER the
// MT-094 launcher has spawned Palmistry (whose crash server binds that same derived path). This env var
// is an explicit OVERRIDE seam only (tests / operator config): when set, it wins over the derivation
// and the client connects EARLY at handler-install time. It is set by no production code path.
//
// When no client is armed (no watcher this run), the crash-handler is still installed but its callback
// no-ops the dump request — the MT-083 in-process panic hook + Palmistry's post-mortem floor still
// cover the death modes.
const ENV_CRASH_SOCKET: &str = "HANDSHAKE_CRASH_SOCK";

/// The process-global MT-092 minidumper CLIENT slot. The OS exception handler is installed EARLY in
/// `main()` (so a startup crash is covered), but in production the crash socket only EXISTS after the
/// MT-094 launcher spawns Palmistry (whose crash server binds the derived path) — so the client is
/// armed LATE via [`arm_crash_client_late`] and the crash callback reads THIS slot at crash time.
/// `OnceLock` is the right primitive for a compromised-context read: `get()` after initialization is a
/// lock-free atomic load (no mutex a dead thread could hold).
static CRASH_CLIENT: std::sync::OnceLock<std::sync::Arc<minidumper::Client>> =
    std::sync::OnceLock::new();

/// The minidumper user-message kind the CLIENT uses to report the faulting thread id to Palmistry ahead
/// of the dump request (mirrors `palmistry::crash_capture::MSG_KIND_FAULTING_THREAD_ID`). minidumper does
/// not surface the CrashContext thread id to the server handler, so the client sends it as a typed 8-byte
/// LE u64 user message (never text) so Palmistry's typed crash record can name the thread.
const MSG_KIND_FAULTING_THREAD_ID: u32 = 1;

/// MT-031 single-installer self-check. `--self-check` proves, on the installed machine, that the
/// single-installer bundle is self-contained (HBR-STOP): it verifies every required bundled asset
/// relative to the running exe, prints a machine-readable JSON verdict, and exits 0 (all present) or 1
/// (a required asset is missing) WITHOUT starting the egui event loop, opening a window, or touching
/// postgres — so it is safe in a minimal/headless CI sandbox. `--version` / `--help` are the standard
/// headless-launch smokes the build proof uses to assert the single binary actually runs.
///
/// Returns `Some(exit_code)` when a flag was handled (caller should exit with it); `None` to fall
/// through to the normal GUI launch.
fn handle_cli_flags() -> Option<i32> {
    // Skip argv[0] (the program path). Only the first recognised flag is acted on.
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--self-check" => {
                let (json, code) = installer::run_self_check();
                println!("{json}");
                return Some(code);
            }
            "--version" | "-V" => {
                println!(
                    "handshake-native {} (build {})",
                    env!("HANDSHAKE_NATIVE_VERSION"),
                    env!("HANDSHAKE_BUILD_DATE"),
                );
                return Some(0);
            }
            "--help" | "-h" => {
                println!(
                    "handshake-native {}\n\nUSAGE:\n  handshake-native [FLAGS]\n\nFLAGS:\n  \
                     --self-check   Verify the installed single-installer bundle is self-contained and exit\n  \
                     --version, -V  Print version and exit\n  --help, -h     Print this help and exit\n\n\
                     With no flags, launches the native work-surface shell.",
                    env!("HANDSHAKE_NATIVE_VERSION"),
                );
                return Some(0);
            }
            // WP-KERNEL-012 MT-092 (AC-012-4 proof seam): a HEADLESS, in-process end-to-end check that
            // the crash-handler install + its callback genuinely signal a minidumper client and a dump is
            // written OUT-OF-PROCESS — WITHOUT crashing this process (uses crash-handler's
            // `simulate_exception` test seam). It stands up a minidumper SERVER on its OWN thread in this
            // same process on a temp socket, installs the crash-handler pointed at that socket, fires a
            // simulated exception, and asserts a real minidump file was written + the callback ran. Prints
            // a machine-readable JSON verdict and exits 0 (ok) / 1 (failed). No GUI, no event loop, no
            // postgres — safe in a headless sandbox. This is the SAME "drive the compiled binary" proof
            // shape MT-089 uses (the binary's items are not importable by tests/).
            "--crash-client-selftest" => {
                let (json, code) = run_crash_client_selftest();
                println!("{json}");
                return Some(code);
            }
            // Unknown args are ignored (eframe/winit may pass platform args); fall through to GUI.
            _ => {}
        }
    }
    None
}

/// Build the [`crash_handler::CrashEvent`] that, on an unhandled OS exception, signals Palmistry (the
/// minidumper SERVER) to write a minidump of THIS process OUT-OF-PROCESS. The callback does the MINIMUM
/// in the crashing process (§6.13.6): report the faulting thread id, then `request_dump` — the EXTERNAL
/// writer does the heavy dump. The client is read from the process-global [`CRASH_CLIENT`] slot AT
/// CRASH TIME (a lock-free `OnceLock::get`), so a client armed LATE (after the MT-094 launcher brought
/// Palmistry up — [`arm_crash_client_late`]) is used by a handler installed EARLY. If no client was
/// ever armed (no watcher this run) the callback is a no-op dump request that still returns cleanly so
/// the exception propagates to the default handler (the panic/abort path is unaffected; the MT-083
/// panic hook covers the Rust-panic death mode separately).
///
/// SAFETY: `make_crash_event` is `unsafe` because `on_crash` runs in a compromised post-exception
/// context. The callback is written to do as little as possible (an atomic slot read + a single
/// `request_dump` IPC send) per the crash-handler safety guidance — no allocation-heavy work, no locks
/// that could be held by a dead thread (`OnceLock::get` never blocks once set).
#[allow(unsafe_code)]
fn make_crash_event() -> Box<dyn crash_handler::CrashEvent> {
    unsafe {
        crash_handler::make_crash_event(
            |cc: &crash_handler::CrashContext| -> crash_handler::CrashEventResult {
                let handled = match CRASH_CLIENT.get() {
                    Some(client) => {
                        // Report the faulting thread id first (typed 8-byte LE u64, never text) so
                        // Palmistry's crash record can name it, then request the out-of-process dump.
                        // On Windows the CrashContext carries `thread_id`; ship it best-effort.
                        #[cfg(windows)]
                        let tid: u64 = cc.thread_id as u64;
                        #[cfg(not(windows))]
                        let tid: u64 = 0;
                        let _ = client.send_message(MSG_KIND_FAULTING_THREAD_ID, tid.to_le_bytes());
                        // `request_dump` BLOCKS until Palmistry has written the minidump from OUTSIDE this
                        // process (§6.13.6). Returns Ok on a successful out-of-process dump.
                        client.request_dump(cc).is_ok()
                    }
                    // No client armed this run: do not attempt a dump (nothing is listening). Return
                    // not-handled so the default abort/unwind still runs — the death is not swallowed.
                    None => false,
                };
                crash_handler::CrashEventResult::Handled(handled)
            },
        )
    }
}

/// Install the MT-092 CLIENT crash handler EARLY in `main()` (§6.13.6) so even a STARTUP crash is
/// covered. The OS exception handler is attached immediately; the minidumper CLIENT that gives it an
/// out-of-process dump path is normally armed LATER by [`arm_crash_client_late`] (once the MT-094
/// launcher has brought Palmistry — and therefore the derived crash socket — up). If the
/// [`ENV_CRASH_SOCKET`] override seam is set (tests / operator config), the client is connected + armed
/// EARLY right here instead. Returns the attached [`crash_handler::CrashHandler`] (kept alive for the
/// process lifetime) so the OS exception handler stays installed, or `None` if attaching the OS handler
/// failed (logged; non-fatal — the MT-083 panic hook still covers Rust panics).
///
/// This COMPLEMENTS the MT-083 in-process panic hook installed just above: the panic hook catches Rust
/// `panic!`s (an orderly unwind/abort with a backtrace), while THIS catches a hard unhandled OS exception
/// / Windows SEH (an access violation, illegal instruction, stack overflow — a death the Rust panic
/// machinery never sees). Together they cover both death modes (AC-012-7); neither is silently uncovered.
fn install_crash_client() -> Option<crash_handler::CrashHandler> {
    // The explicit OVERRIDE seam (tests / operator config): a supplied socket wins over the production
    // derive-late path and is connected immediately.
    if let Ok(socket) = std::env::var(ENV_CRASH_SOCKET) {
        if !socket.trim().is_empty() {
            match minidumper::Client::with_name(minidumper::SocketName::path(&socket)) {
                Ok(c) => {
                    let _ = CRASH_CLIENT.set(std::sync::Arc::new(c));
                    tracing::info!(
                        crash_socket = %socket,
                        "MT-092 crash client connected via the {ENV_CRASH_SOCKET} override seam \
                         (out-of-process minidump armed early)"
                    );
                }
                Err(err) => {
                    // The override names a socket nothing is listening on (yet). Non-fatal: the handler
                    // still installs (no-op dump) and the late-arm step may still connect the derived path.
                    tracing::warn!(
                        %err,
                        crash_socket = %socket,
                        "MT-092 crash client could not connect to the {ENV_CRASH_SOCKET} override socket; \
                         installing the OS exception handler unarmed (the late-arm step may still connect)"
                    );
                }
            }
        }
    } else {
        tracing::debug!(
            "no {ENV_CRASH_SOCKET} override; the crash client arms LATE against the derived crash \
             socket once the MT-094 launcher has brought Palmistry up"
        );
    }

    match crash_handler::CrashHandler::attach(make_crash_event()) {
        Ok(handler) => {
            tracing::info!(
                "MT-092 OS exception handler installed (Windows SEH / unix signals) — complements the \
                 MT-083 Rust-panic hook; both death modes covered (§6.13.6 + §5.8.2)"
            );
            Some(handler)
        }
        Err(err) => {
            tracing::warn!(%err, "failed to attach the OS exception handler (MT-092); Rust panics are \
                 still covered by the MT-083 panic hook");
            None
        }
    }
}

/// How long [`arm_crash_client_late`] retries connecting to Palmistry's derived crash socket. The
/// launcher's bounded handshake has normally completed by the time this runs (so the crash server is
/// already bound); the retry window only covers the small startup race where the control-socket ACK is
/// served before the crash server binds. Bounded so a missing/failed watcher can never stall startup.
const CRASH_CLIENT_ARM_DEADLINE: std::time::Duration = std::time::Duration::from_secs(3);
/// Backoff between crash-socket connect attempts during the late-arm window.
const CRASH_CLIENT_ARM_RETRY: std::time::Duration = std::time::Duration::from_millis(100);

/// LATE-ARM the MT-092 minidumper CLIENT (the §6.13.6 rendezvous, MT-092/MT-094 remediation): connect
/// to the crash socket the launched Palmistry's crash server bound — the path DERIVED from the control
/// socket by the shared rule (`diagnostics::crash_socket_path`, mirrored by
/// `palmistry::crash_capture::crash_socket_path`, pinned equal by a cross-crate wire test) — and store
/// the client into the process-global [`CRASH_CLIENT`] slot the EARLY-installed exception handler reads
/// at crash time. Bounded retries cover the bind race; every failure path degrades gracefully (the
/// handler stays installed unarmed; Palmistry's post-mortem floor still records a crash).
///
/// No-op if a client is already armed (the [`ENV_CRASH_SOCKET`] override seam connected early).
fn arm_crash_client_late(crash_socket: &str) {
    if CRASH_CLIENT.get().is_some() {
        tracing::debug!("MT-092 crash client already armed (override seam); skipping the late-arm step");
        return;
    }
    let deadline = std::time::Instant::now() + CRASH_CLIENT_ARM_DEADLINE;
    let mut attempts = 0u32;
    loop {
        attempts += 1;
        match minidumper::Client::with_name(minidumper::SocketName::path(crash_socket)) {
            Ok(c) => {
                let _ = CRASH_CLIENT.set(std::sync::Arc::new(c));
                tracing::info!(
                    crash_socket,
                    attempts,
                    "MT-092 crash client connected to Palmistry's derived crash socket \
                     (out-of-process minidump armed, §6.13.6)"
                );
                return;
            }
            Err(err) => {
                if std::time::Instant::now() >= deadline {
                    tracing::warn!(
                        %err,
                        crash_socket,
                        attempts,
                        "MT-092 crash client could not connect to Palmistry's derived crash socket \
                         within the bounded arm window; running WITHOUT an out-of-process dump path \
                         this session (Palmistry's post-mortem floor still records a crash)"
                    );
                    return;
                }
                std::thread::sleep(CRASH_CLIENT_ARM_RETRY);
            }
        }
    }
}

/// MT-092 AC-012-4 in-process end-to-end self-check (the `--crash-client-selftest` flag). Proves, against
/// the REAL Embark stack and WITHOUT crashing this process, that: (1) a `minidumper::Server` writes a
/// minidump OUT-OF-PROCESS when (2) a crash-handler-installed client's callback fires (via
/// `simulate_exception`, the crash-handler test seam) and calls `request_dump`. Returns a
/// machine-readable JSON verdict + an exit code (0 ok / 1 failed). All artifacts go to the OS temp dir
/// (never repo-local). Returns cleanly on every path (no panic) so the flag is safe in a headless CI.
fn run_crash_client_selftest() -> (String, i32) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    // A unique temp socket + dump path for this run (no collision across parallel runs).
    let tag = format!(
        "hsk-crash-selftest-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    // minidumper binds an AF_UNIX socket on EVERY platform (incl. Windows 10+) whose address is a real
    // FILESYSTEM path in `sun_path` (NOT a `\\.\pipe\` named pipe). Keep the path short (the `sun_path`
    // is 108 bytes) under the OS temp dir.
    let socket = std::env::temp_dir()
        .join(format!("hsk-cself-{}.sock", std::process::id()))
        .to_string_lossy()
        .into_owned();
    let _ = std::fs::remove_file(&socket); // a stale socket file from a prior run would block bind
    let dump_path = std::env::temp_dir().join(format!("{tag}.dmp"));

    // The server handler writes the dump to `dump_path` and latches `captured` on success.
    struct SelfTestHandler {
        dump_path: std::path::PathBuf,
        captured: Arc<AtomicBool>,
        error: Arc<Mutex<Option<String>>>,
    }
    impl minidumper::ServerHandler for SelfTestHandler {
        fn create_minidump_file(
            &self,
        ) -> Result<(std::fs::File, std::path::PathBuf), std::io::Error> {
            let f = std::fs::File::create(&self.dump_path)?;
            Ok((f, self.dump_path.clone()))
        }
        fn on_minidump_created(
            &self,
            result: Result<minidumper::MinidumpBinary, minidumper::Error>,
        ) -> minidumper::LoopAction {
            match result {
                Ok(mut b) => {
                    use std::io::Write as _;
                    let _ = b.file.flush();
                    self.captured.store(true, Ordering::SeqCst);
                }
                Err(e) => {
                    if let Ok(mut slot) = self.error.lock() {
                        *slot = Some(format!("{e}"));
                    }
                }
            }
            minidumper::LoopAction::Exit
        }
        fn on_message(&self, _kind: u32, _buffer: Vec<u8>) {}
    }

    let captured = Arc::new(AtomicBool::new(false));
    let server_error = Arc::new(Mutex::new(None));
    let shutdown = Arc::new(AtomicBool::new(false));

    // Bind the server BEFORE the client connects.
    let mut server = match minidumper::Server::with_name(minidumper::SocketName::path(&socket)) {
        Ok(s) => s,
        Err(e) => {
            return (
                format!(r#"{{"ok":false,"stage":"server_bind","error":"{e}"}}"#),
                1,
            )
        }
    };
    let handler = SelfTestHandler {
        dump_path: dump_path.clone(),
        captured: Arc::clone(&captured),
        error: Arc::clone(&server_error),
    };
    let server_shutdown = Arc::clone(&shutdown);
    let server_loop = std::thread::spawn(move || {
        let _ = server.run(
            Box::new(handler),
            &server_shutdown,
            Some(std::time::Duration::from_secs(5)),
        );
    });

    // Connect the client + install the crash handler pointed at it (the REAL production install path).
    let client = match minidumper::Client::with_name(minidumper::SocketName::path(&socket)) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            shutdown.store(true, Ordering::SeqCst);
            let _ = server_loop.join();
            return (
                format!(r#"{{"ok":false,"stage":"client_connect","error":"{e}"}}"#),
                1,
            );
        }
    };
    let callback_ran = Arc::new(AtomicBool::new(false));
    let cb_flag = Arc::clone(&callback_ran);
    let client_for_cb = Arc::clone(&client);
    #[allow(unsafe_code)]
    let handler_attach = crash_handler::CrashHandler::attach(unsafe {
        crash_handler::make_crash_event(move |cc: &crash_handler::CrashContext| {
            cb_flag.store(true, Ordering::SeqCst);
            #[cfg(windows)]
            let tid: u64 = cc.thread_id as u64;
            #[cfg(not(windows))]
            let tid: u64 = 0;
            let _ = client_for_cb.send_message(MSG_KIND_FAULTING_THREAD_ID, tid.to_le_bytes());
            crash_handler::CrashEventResult::Handled(client_for_cb.request_dump(cc).is_ok())
        })
    });
    let handler_attach = match handler_attach {
        Ok(h) => h,
        Err(e) => {
            shutdown.store(true, Ordering::SeqCst);
            let _ = server_loop.join();
            return (
                format!(r#"{{"ok":false,"stage":"attach","error":"{e}"}}"#),
                1,
            );
        }
    };

    // FIRE a SIMULATED exception (the crash-handler test seam) — this runs the callback WITHOUT crashing
    // the process. The callback signals the server, which writes the dump out-of-process.
    let sim = handler_attach.simulate_exception(None);
    let sim_handled = matches!(sim, crash_handler::CrashEventResult::Handled(true));

    // The server loop exits after writing the dump; join it (bounded by its stale-timeout).
    let _ = server_loop.join();
    drop(handler_attach); // detach the OS handler.

    let dump_exists = dump_path.exists();
    let dump_len = std::fs::metadata(&dump_path).map(|m| m.len()).unwrap_or(0);
    let cb = callback_ran.load(Ordering::SeqCst);
    let cap = captured.load(Ordering::SeqCst);
    let srv_err = server_error.lock().ok().and_then(|s| s.clone());

    // Clean up the temp dump (the proof already read its existence + size).
    let _ = std::fs::remove_file(&dump_path);

    let ok = cb && sim_handled && cap && dump_exists && dump_len > 0;
    let json = format!(
        r#"{{"ok":{ok},"callback_ran":{cb},"sim_handled":{sim_handled},"minidump_captured":{cap},"dump_existed":{dump_exists},"dump_len":{dump_len},"server_error":{}}}"#,
        match srv_err {
            Some(e) => format!(r#""{}""#, e.replace('"', "'")),
            None => "null".to_string(),
        }
    );
    (json, if ok { 0 } else { 1 })
}

fn main() -> eframe::Result<()> {
    // MT-031: handle headless CLI flags (--self-check / --version / --help) BEFORE any tracing init or
    // event-loop setup, so the self-check stays a pure path-existence probe with no GUI side effects.
    if let Some(code) = handle_cli_flags() {
        std::process::exit(code);
    }

    // WP-KERNEL-012 MT-083 (D2 — internal_diagnostics, Tier 2): install the durable-local-crash-record
    // panic hook EARLY — before tracing init and `eframe::run_native` — so a panic during STARTUP is
    // also captured. Generate the process-start session id HERE; the panic hook names the crash file
    // with it AND publishes it process-globally via `diagnostics::set_process_session_id` so a later
    // host-mount MT (which owns app.rs, outside MT-083's allowed_paths) can have the MT-081 ring REUSE
    // the SAME id (`diagnostics::process_session_id()`) and a watcher correlates the crash file to the
    // ring. The crash dir resolves via `dirs` to a PORTABLE per-user path (NOT a hardcoded absolute path
    // — GLOBAL-PORTABILITY); if no local-data dir exists (rare), fall back to a still-portable temp dir.
    let process_session_id = uuid::Uuid::new_v4().to_string();
    let crash_dir = diagnostics::default_crash_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("handshake").join("crash"));
    diagnostics::install_panic_hook(crash_dir, &process_session_id);

    // WP-KERNEL-012 MT-092 (CLIENT side, §6.13.6): install the OS exception handler (Windows SEH / unix
    // signals) EARLY so an UNHANDLED hard OS exception (access violation, illegal instruction, stack
    // overflow) during STARTUP is already covered. This COMPLEMENTS the MT-083 Rust-panic hook installed
    // just above: panics unwind/abort with a backtrace; OS exceptions never reach the panic machinery, so
    // both death modes are covered (AC-012-7). The returned handler is held alive for the whole process
    // lifetime (`_crash_handler`); dropping it would detach the OS handler. The minidumper CLIENT that
    // gives the handler an out-of-process dump path is armed LATER (`arm_crash_client_late`, below) once
    // the MT-094 launcher has brought Palmistry — and therefore the DERIVED crash socket — up; the
    // HANDSHAKE_CRASH_SOCK env var is an explicit override seam only (tests / operator config), set by no
    // production code path. Until a client is armed the handler does not request a dump (nothing is
    // listening) — non-fatal.
    let _crash_handler = install_crash_client();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "handshake_native=debug,eframe=info".into()),
        )
        .init();

    // HBR-QUIET (MT-030): assert the quiet-operation guard is installed before the event loop. In
    // debug builds this logs the active invariant; the binding enforcement is the source audit in
    // tests/test_focus_audit_quiet.rs (the shell makes no Win32 foreground/input-injection call).
    focus_guard::assert_quiet_mode_installed();

    // WP-KERNEL-012 MT-094 (§6.13.3 "launched WITH Handshake at startup"): create the MT-081 diagnostics
    // ring and LAUNCH the external Palmistry watcher HERE — in `main()`, BEFORE `eframe::run_native` — so
    // the watcher is up before the app can freeze. The ring is created here (not in `HandshakeApp::new`)
    // for two reasons: (1) we need the ring/session to hand to Palmistry BEFORE the event-loop closure
    // runs, and (2) the whole egui_kittest suite builds `HandshakeApp::new` directly and must NOT each
    // spawn + leak a palmistry child (the MT-094 anti-leak rule). `HandshakeApp::new` REUSES this
    // already-created ring via the process-global preinstalled-session slot. The launch is QUIET
    // (CREATE_NO_WINDOW, no focus steal), BOUNDED (a slow/absent watcher never blocks startup), preserves
    // the not-kill-on-job-close survives-parent-death inversion, and DEGRADES GRACEFULLY: a missing
    // palmistry.exe or a failed spawn leaves `palmistry = None` and Handshake starts anyway (§5.8.6).
    let palmistry = match HandshakeApp::create_and_install_diag_ring() {
        Some(session) => {
            let control_socket = diagnostics::control_socket_name(&session.session_id);
            // Make `HandshakeApp::new` reuse THIS ring instead of creating a second one.
            diagnostics::set_preinstalled_diag_session(session.clone());
            diagnostics::launch_palmistry_or_degrade(&session, &control_socket)
        }
        None => {
            tracing::warn!(
                "internal_diagnostics ring unavailable at startup; Palmistry not launched this session \
                 (diagnostics are in-process-only — graceful degradation)"
            );
            None
        }
    };

    // WP-KERNEL-012 MT-092/MT-094 remediation — LATE-ARM the crash CLIENT (§6.13.6 rendezvous): now
    // that Palmistry is up (its crash server binds the crash socket DERIVED from the control socket by
    // the shared rule), connect the minidumper client to the handle's derived crash-socket path and
    // store it in the slot the EARLY-installed exception handler reads at crash time. Bounded — a
    // slow/failed connect degrades to an unarmed handler (Palmistry's post-mortem floor still records a
    // crash) and never stalls startup. Skipped when the watcher was not launched (nothing listening).
    if let Some(handle) = &palmistry {
        arm_crash_client_late(handle.crash_socket());
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Handshake")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([640.0, 480.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "handshake-native",
        native_options,
        Box::new(move |cc| {
            let mut app = HandshakeApp::new(cc);
            // Hand the running shell the Palmistry handle so a clean exit (`HandshakeApp::on_exit`) sends
            // the explicit Shutdown control message (§6.13.3). `None` when the watcher was not launched
            // (graceful degradation) — the app simply runs without it.
            if let Some(handle) = palmistry {
                app.set_palmistry_handle(handle);
            }
            Ok(Box::new(app))
        }),
    )
}
