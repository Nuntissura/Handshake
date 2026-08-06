//! MT-015 v5 — HBR-QUIET-001 negative proofs for the operator-initiated
//! official-CLI login.
//!
//! ## What broke, and why a one-line fix was wrong
//!
//! The CLI-bridge login used to spawn the provider's login command with the
//! Windows `CREATE_NEW_CONSOLE` creation flag — the ONLY spawn site in the tree
//! that opened an OS console window, in a codebase that otherwise spawns with
//! `CREATE_NO_WINDOW`. That is an HBR-QUIET-001 violation: the operator's
//! foreground application does not stay in front, and MT-015 declares no
//! HBR-QUIET-004 foreground exception.
//!
//! Flipping the flag to `CREATE_NO_WINDOW` would have satisfied the letter of
//! the rule and destroyed the feature: `claude auth login` and `codex login` are
//! interactive device/OAuth flows, so hiding the console turns "steals focus"
//! into "hangs invisibly with no prompt and no way to finish". The login is now
//! run under a Handshake-hosted ConPTY and surfaced in the in-app Settings login
//! panel, so it is quiet AND completable.
//!
//! ## The two proofs here
//!
//! 1. [`in_app_cli_login_creates_no_new_visible_window_and_no_foreground_change`]
//!    is a REAL runtime proof: it installs the live Win32
//!    `WINEVENT_SYSTEM_FOREGROUND` hook (the same `FocusAuditHandle` the
//!    `focus-audit-probe` binary drives), runs the REAL production login launch
//!    through `LiveCliSpawner` — canonical executable pin, `env_clear` +
//!    `attached_child_env`, ConPTY spawn, OS generation attestation,
//!    process-ledger START/STOP — and then fails if EITHER
//!      * any foreground event during the window belongs to the login child or to
//!        any process created after the launch started (a `CREATE_NEW_CONSOLE`
//!        launch spawns a fresh `conhost.exe` that owns and activates the console
//!        window, so this catches it), OR
//!      * a new VISIBLE top-level window appeared that is owned either by a
//!        process this launch created OR by a console-host image
//!        (`conhost.exe` / `OpenConsole.exe` / `WindowsTerminal.exe` / a shell).
//!        The second clause closes a real hole in the first: Windows 11 can hand
//!        a newly created console to an ALREADY-RUNNING `WindowsTerminal.exe`,
//!        whose pid predates the launch.
//!
//!    The same test then proves the login is still COMPLETABLE, because "quiet"
//!    alone is satisfiable by a login that hangs invisibly: the fixture child's
//!    interactive prompt must reach the in-app transcript, and the operator's
//!    typed answer must reach the child's stdin (asserted on a string only the
//!    CHILD can emit, so the ConPTY input echo cannot forge it).
//!
//! 2. [`no_backend_spawn_site_creates_a_console_window`] is a source audit over
//!    the whole product tree. It fails the build the moment a console-creating
//!    creation flag reappears at any spawn site, so the invariant cannot regress
//!    silently between runtime proofs. This mirrors the rationale already
//!    documented in the native shell's `quiet_mode::focus_guard`.
//!
//! Residual honesty note for proof 1: it observes a real desktop. If an
//! unrelated application happens to create a brand-new visible top-level window
//! inside the sub-second audit window, the test reports a violation it did not
//! cause. That is a deliberate false-POSITIVE bias — a quiet-mode proof must
//! never be tuned so that a real console window can slip through.

use std::path::{Path, PathBuf};

/// The console-creating process-creation flag. `CREATE_NO_WINDOW`
/// (`0x0800_0000`) is the only acceptable console posture, so this token must
/// not appear in executable product code at all.
///
/// Scope note: the focus/input-injection Win32 APIs (`SetForegroundWindow`,
/// `BringWindowToTop`, …) are deliberately NOT re-banned here. The native
/// shell's `quiet_mode::focus_guard` + `tests/test_focus_audit_quiet.rs` already
/// own that invariant with a curated allow-list (screenshot capture, the
/// deliberate `handshake-foreground-inject-probe`, sandbox escape fixtures).
/// Re-implementing it here without that allow-list would produce a duplicate,
/// weaker audit. This audit owns exactly the invariant MT-015 regressed:
/// no product spawn site may create a console window.
const BANNED_CONSOLE_CREATION_TOKEN: &str = "CREATE_NEW_CONSOLE";

/// Strip `//`-style comments so a doc comment that NAMES the banned flag (this
/// change deliberately documents why the flag was removed) is not mistaken for a
/// live call site. String literals containing `//` are not a concern here: the
/// banned token is an identifier, and the `creation_flags` check reads the
/// argument text.
fn strip_line_comments(line: &str) -> &str {
    match line.find("//") {
        Some(index) => &line[..index],
        None => line,
    }
}

fn repo_src_roots() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")); // <repo>/src/backend/handshake_core
    let repo_src = manifest
        .parent()
        .and_then(Path::parent)
        .expect("<repo>/src resolves from the backend manifest dir")
        .to_path_buf();
    vec![
        repo_src.join("backend/handshake_core/src"),
        repo_src.join("frontend/handshake_native/src"),
    ]
}

fn rust_sources(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// HBR-QUIET-001 source invariant: no product spawn site may create a console
/// window, and every `creation_flags` argument must be a `CREATE_NO_WINDOW`
/// composition.
///
/// This is the regression fence for the MT-015 defect. It runs on every platform
/// so a non-Windows CI lane still catches a reintroduced console flag.
#[test]
fn no_backend_spawn_site_creates_a_console_window() {
    let mut files = Vec::new();
    for root in repo_src_roots() {
        assert!(root.is_dir(), "source root missing: {}", root.display());
        rust_sources(&root, &mut files);
    }
    assert!(
        files.len() > 100,
        "source scan found only {} files; the roots are wrong",
        files.len()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        // This audit file necessarily NAMES the banned tokens in order to ban
        // them; skipping it is what keeps the invariant self-consistent.
        if file.ends_with("cli_bridge_login_quiet_tests.rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for (index, raw_line) in text.lines().enumerate() {
            let line_no = index + 1;
            let line = strip_line_comments(raw_line);
            if line.contains(BANNED_CONSOLE_CREATION_TOKEN) {
                violations.push(format!(
                    "{}:{line_no}: {BANNED_CONSOLE_CREATION_TOKEN}",
                    file.display()
                ));
            }
            // Every creation-flags argument must be a CREATE_NO_WINDOW
            // composition (optionally OR'd with non-window flags such as
            // CREATE_NEW_PROCESS_GROUP / CREATE_BREAKAWAY_FROM_JOB).
            if let Some(rest) = line.split_once(".creation_flags(") {
                let argument = rest.1;
                let is_no_window_composition = argument.contains("CREATE_NO_WINDOW")
                    || argument.contains("palmistry_creation_flags");
                if !is_no_window_composition {
                    violations.push(format!(
                        "{}:{line_no}: creation_flags argument is not a CREATE_NO_WINDOW composition: {}",
                        file.display(),
                        argument.trim()
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "HBR-QUIET-001: no product spawn site may create a console window; every \
         creation_flags argument must be a CREATE_NO_WINDOW composition. Offending sites:\n{}",
        violations.join("\n")
    );
}

/// The audit must be able to FAIL. If a scan of the real tree finds no
/// `.creation_flags(` call at all, the pattern drifted (a refactor renamed the
/// call, or the roots are wrong) and the audit above would pass vacuously — a
/// false PASS on the exact invariant MT-015 regressed.
#[test]
fn console_flag_audit_actually_inspects_real_spawn_sites() {
    let mut files = Vec::new();
    for root in repo_src_roots() {
        rust_sources(&root, &mut files);
    }
    let mut creation_flag_sites = 0usize;
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        creation_flag_sites += text.matches(".creation_flags(").count();
    }
    assert!(
        creation_flag_sites >= 8,
        "the console-flag audit found only {creation_flag_sites} creation_flags call sites; \
         the scan pattern or the source roots have drifted and the audit would pass vacuously"
    );
    // The banned token must still be detectable when it IS present in code: this
    // synthetic line is exactly what a regression would look like.
    assert!(
        strip_line_comments("        .creation_flags(CREATE_NEW_CONSOLE);")
            .contains(BANNED_CONSOLE_CREATION_TOKEN),
        "the comment stripper must not hide a real regression"
    );
    assert!(
        !strip_line_comments("    /// never use CREATE_NEW_CONSOLE here")
            .contains(BANNED_CONSOLE_CREATION_TOKEN),
        "the comment stripper must ignore documentation that names the banned flag"
    );
}

#[cfg(target_os = "windows")]
mod windows_runtime {
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;

    use handshake_core::model_runtime::cloud::{
        CliBridgeConfig, CliKind, CliOutputFormat, CliSubprocessSpawner, InteractiveLoginTransport,
        LiveCliSpawner,
    };
    use handshake_core::operator_foreground::focus_audit::{
        FocusAuditHandle, FocusAuditLedger, FocusAuditReport, OwnedProcessPidSet,
    };
    use async_trait::async_trait;
    use handshake_core::process_ledger::{
        LedgerBatcher, LedgerBatcherConfig, LedgerEvent, NoopOverflowSink, ProcessLedgerError,
        ProcessLedgerStore,
    };
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HWND, LPARAM};
    use windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    /// Image names that host a console window. A reintroduced `CREATE_NEW_CONSOLE`
    /// launch always ends with ONE of these owning a visible window on behalf of
    /// the login child: classic `conhost.exe`, or — when Windows 11 hands the new
    /// console off to the configured default terminal — `OpenConsole.exe` /
    /// `WindowsTerminal.exe`.
    ///
    /// This list exists because the "process created during the audit window"
    /// attribution has a real hole on Windows 11: the default-terminal handoff can
    /// render the new console in an ALREADY-RUNNING `WindowsTerminal.exe`, whose
    /// pid predates the launch, so the created-after filter alone would let the
    /// regression through. A console host that gains a BRAND-NEW visible top-level
    /// window inside the audit window is the regression signature regardless of
    /// which process renders it; the quiet ConPTY route gives its pseudo-console
    /// host no visible window at all.
    const CONSOLE_HOST_IMAGES: &[&str] = &[
        "conhost.exe",
        "openconsole.exe",
        "windowsterminal.exe",
        "cmd.exe",
        "powershell.exe",
        "pwsh.exe",
    ];

    /// Image file name for `pid`, or `None` when the process is gone or not
    /// queryable. Same mechanism `operator_foreground::focus_audit` uses to label
    /// a foreground event.
    fn process_exe_name(pid: u32) -> Option<String> {
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return None;
        }
        let mut buffer = vec![0u16; 32_768];
        let mut size = buffer.len() as u32;
        let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
        let _ = unsafe { CloseHandle(handle) };
        if ok == 0 || size == 0 {
            return None;
        }
        buffer.truncate(size as usize);
        let full_path = String::from_utf16_lossy(&buffer);
        Some(
            std::path::Path::new(&full_path)
                .file_name()?
                .to_string_lossy()
                .into_owned(),
        )
    }

    fn is_console_host_image(name: &str) -> bool {
        CONSOLE_HOST_IMAGES
            .iter()
            .any(|host| name.eq_ignore_ascii_case(host))
    }

    /// The fixture child publishes its pid here and then sleeps, so the audit
    /// window covers a genuinely live login process rather than a flash.
    const FIXTURE_READY_ENV: &str = "NO_COLOR";

    /// The interactive prompt the fixture child writes into the pty, standing in
    /// for a provider's device-code / OAuth prompt. The operator must be able to
    /// READ this out of the in-app transcript.
    const FIXTURE_PROMPT: &str = "HANDSHAKE-LOGIN-PROMPT>";

    /// What the fixture child prints AFTER it has actually read the operator's
    /// answer off the pty. Only the CHILD can produce this string, so asserting
    /// on it cannot be satisfied by the ConPTY's own input echo — the echo alone
    /// would prove nothing about the answer reaching the login process.
    const FIXTURE_ECHO_PREFIX: &str = "HANDSHAKE-LOGIN-RECEIVED:";

    /// The operator's typed answer. `write_input` receives it with the same
    /// trailing `\r` that `CliBridgeLoginSessionRegistry::send_input` appends in
    /// production, so this proof drives the real terminal encoding.
    const FIXTURE_ANSWER: &str = "quiet-login-probe";

    /// Time (ms) allowed for any console window to appear and activate after the
    /// launch. A `CREATE_NEW_CONSOLE` console window is created and foregrounded
    /// well inside this budget.
    const WINDOW_SETTLE: Duration = Duration::from_millis(900);

    fn now_filetime_100ns() -> u64 {
        // WAIVER [CX-573E]: wall-clock read used only to bound "was this process
        // created during the audit window"; no determinism-bearing authority.
        let mut file_time = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        unsafe { GetSystemTimeAsFileTime(&mut file_time) };
        (u64::from(file_time.dwHighDateTime) << 32) | u64::from(file_time.dwLowDateTime)
    }

    unsafe extern "system" fn collect_visible_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        if IsWindowVisible(hwnd) != 0 {
            let windows = &mut *(lparam as *mut Vec<(isize, u32)>);
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid != 0 {
                windows.push((hwnd as isize, pid));
            }
        }
        1
    }

    /// Every visible top-level window as `(hwnd, owning pid)`.
    fn visible_top_level_windows() -> Vec<(isize, u32)> {
        let mut windows: Vec<(isize, u32)> = Vec::new();
        unsafe {
            EnumWindows(
                Some(collect_visible_window),
                &mut windows as *mut Vec<(isize, u32)> as LPARAM,
            );
        }
        windows
    }

    /// `true` when `pid` names a live process that was created at or after
    /// `since_100ns` — i.e. a process this launch brought into existence.
    fn process_created_after(pid: u32, since_100ns: u64) -> bool {
        handshake_core::sandbox::handshake_native::process_creation_time_100ns(pid)
            .map(|created| created >= since_100ns)
            .unwrap_or(false)
    }

    /// In-memory ledger store so the REAL background writer can make the login's
    /// START row durable. The success path is the point of this proof, so the
    /// process-ledger authority must actually complete rather than be stubbed.
    #[derive(Clone, Default)]
    struct CapturingLedgerStore {
        events: Arc<std::sync::Mutex<Vec<LedgerEvent>>>,
    }

    #[async_trait]
    impl ProcessLedgerStore for CapturingLedgerStore {
        async fn write_batch(&self, events: Vec<LedgerEvent>) -> Result<(), ProcessLedgerError> {
            self.events
                .lock()
                .expect("ledger store lock")
                .extend(events);
            Ok(())
        }
    }

    fn spawned_ledger() -> (Arc<LedgerBatcher>, CapturingLedgerStore) {
        let store = CapturingLedgerStore::default();
        let (batcher, _join) = LedgerBatcher::spawn(
            Arc::new(store.clone()) as Arc<dyn ProcessLedgerStore>,
            Arc::new(NoopOverflowSink),
            LedgerBatcherConfig::default(),
        );
        (Arc::new(batcher), store)
    }

    /// A REAL production login launch must produce no foreground change and no
    /// new visible window.
    ///
    /// Falsifiability: restoring `CREATE_NEW_CONSOLE` at the launch site makes
    /// this fail on both assertions — the fresh `conhost.exe` is a
    /// created-during-window process that both owns a visible top-level window
    /// and raises it to the foreground.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn in_app_cli_login_creates_no_new_visible_window_and_no_foreground_change() {
        let temp = tempfile::Builder::new()
            .prefix("cli-login-quiet-")
            .tempdir()
            .expect("quiet-login tempdir");
        let fixture = temp.path().join("login-fixture.exe");
        std::fs::copy(
            std::env::current_exe().expect("current test executable"),
            &fixture,
        )
        .expect("copy the login fixture executable");
        let ready_path = temp.path().join("child.ready");

        let mut config = CliBridgeConfig {
            cli_kind: CliKind::Other,
            executable_path: fixture,
            args_template: vec!["{prompt}".to_string()],
            output_format: CliOutputFormat::RawText,
            env_vars: std::collections::HashMap::new(),
            working_dir: None,
            timeout_seconds: 30,
        };
        config.env_vars.insert(
            FIXTURE_READY_ENV.to_string(),
            ready_path.to_string_lossy().into_owned(),
        );

        let (ledger, ledger_store) = spawned_ledger();
        let spawner = LiveCliSpawner::new(ledger, LiveCliSpawner::native_cli_registry());
        spawner
            .pin_config(&config)
            .expect("pin the quiet-login fixture through the canonical launch builder");

        let run_id = format!("cli-login-quiet-{}", uuid::Uuid::now_v7());
        let runtime_root = temp.path().to_path_buf();
        let audit = FocusAuditHandle::start(
            run_id.clone(),
            &runtime_root,
            OwnedProcessPidSet::default(),
        )
        .await
        .expect("the live Win32 foreground hook installs (Windows host required)");

        let launch_started_100ns = now_filetime_100ns();
        let before: HashSet<isize> = visible_top_level_windows()
            .into_iter()
            .map(|(hwnd, _)| hwnd)
            .collect();

        let login = spawner
            .launch_interactive_login_for_tests(
                &config,
                &[
                    "--ignored",
                    "--exact",
                    "windows_runtime::quiet_login_fixture_child",
                    "--nocapture",
                ],
            )
            .expect("the real interactive login launch succeeds");

        // Wait for the child to prove it is actually running before judging the
        // desktop: a proof that measured an already-exited child would be a
        // false PASS.
        let mut child_pid = None;
        for _ in 0..200 {
            if let Ok(text) = std::fs::read_to_string(&ready_path) {
                if let Ok(pid) = text.trim().parse::<u32>() {
                    child_pid = Some(pid);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let child_pid = child_pid.expect("the login fixture child started and published its pid");
        tokio::time::sleep(WINDOW_SETTLE).await;

        let after = visible_top_level_windows();
        let new_windows: Vec<(isize, u32)> = after
            .iter()
            .copied()
            .filter(|(hwnd, _)| !before.contains(hwnd))
            .collect();
        let mut window_violations: Vec<String> = Vec::new();
        for (hwnd, pid) in &new_windows {
            let exe = process_exe_name(*pid);
            let console_host = exe.as_deref().is_some_and(is_console_host_image);
            if *pid == child_pid
                || process_created_after(*pid, launch_started_100ns)
                || console_host
            {
                window_violations.push(format!("hwnd={hwnd:#x} pid={pid} exe={exe:?}"));
            }
        }

        // COMPLETABLE half of the contract, proven as a full round trip through
        // the production transport rather than asserted in prose. A "quiet" login
        // that could not be read or answered would be a WORSE product than the
        // console it replaced: it would turn "steals focus" into "hangs invisibly".
        //
        //   1. the provider's interactive prompt reaches the in-app transcript
        //      (the operator can SEE what to answer), and
        //   2. the operator's typed answer reaches the login process's stdin
        //      (the operator can RESPOND, and the child actually receives it).
        //
        // (2) asserts on a string only the CHILD can emit, so the ConPTY's own
        // input echo cannot forge it.
        assert!(
            wait_for_transcript(&login, FIXTURE_PROMPT).await,
            "the login prompt never reached the in-app transcript, so the quiet login is not \
             completable — the operator would have nothing to answer. Transcript: {:?}",
            String::from_utf8_lossy(&login.transcript())
        );
        login
            .write_input(format!("{FIXTURE_ANSWER}\r").as_bytes())
            .expect("the login PTY accepts operator input");
        let echo_marker = format!("{FIXTURE_ECHO_PREFIX}{FIXTURE_ANSWER}");
        assert!(
            wait_for_transcript(&login, &echo_marker).await,
            "the operator's answer never reached the login process's stdin, so the quiet login \
             cannot be finished. Transcript: {:?}",
            String::from_utf8_lossy(&login.transcript())
        );

        login.cancel();
        let report = collect_report(audit, &run_id, &runtime_root, child_pid).await;

        // The process-ledger authority is intact across the PTY route: the login
        // recorded a durable START row (it could not have launched otherwise) and
        // the watcher records the matching STOP after the cancel.
        for _ in 0..100 {
            let has_stop = ledger_store
                .events
                .lock()
                .expect("ledger store lock")
                .iter()
                .any(|event| matches!(event, LedgerEvent::Stop(_)));
            if has_stop {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let ledger_events = ledger_store.events.lock().expect("ledger store lock").clone();
        assert!(
            ledger_events
                .iter()
                .any(|event| matches!(event, LedgerEvent::Start(_))),
            "the in-app login must record a process-ledger START row"
        );
        assert!(
            ledger_events
                .iter()
                .any(|event| matches!(event, LedgerEvent::Stop(_))),
            "the in-app login must record the matching process-ledger STOP row after cancel"
        );

        assert!(
            window_violations.is_empty(),
            "HBR-QUIET-001: the official-CLI login must open NO window. New visible top-level \
             windows owned by a process this launch created, or by a console host: {}",
            window_violations.join(", ")
        );

        let foreground_violations: Vec<String> = report
            .handshake_owned_events
            .iter()
            .chain(report.foreign_events.iter())
            .filter(|event| {
                event.pid == child_pid || process_created_after(event.pid, launch_started_100ns)
            })
            .map(|event| {
                format!(
                    "pid={} exe={:?} hwnd={}",
                    event.pid, event.exe_name, event.hwnd
                )
            })
            .collect();
        assert!(
            foreground_violations.is_empty(),
            "HBR-QUIET-001: the official-CLI login must cause NO foreground/Z-order change. \
             Foreground events from processes this launch created: {}",
            foreground_violations.join(", ")
        );
    }

    /// Poll the live login transcript for `needle`, bounded. Reads through the
    /// SAME [`InteractiveLoginTransport::transcript`] the HTTP poll route serves
    /// to the Settings login panel, so a pass here means the operator's surface
    /// would have shown it.
    async fn wait_for_transcript(login: &impl InteractiveLoginTransport, needle: &str) -> bool {
        for _ in 0..200 {
            if String::from_utf8_lossy(&login.transcript()).contains(needle) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        false
    }

    /// Stop the hook and rebuild the report with the login child registered as a
    /// Handshake-owned pid, so an activation by the child itself is classified
    /// as a Handshake-owned violation rather than foreign desktop noise.
    async fn collect_report(
        audit: FocusAuditHandle,
        run_id: &str,
        runtime_root: &PathBuf,
        child_pid: u32,
    ) -> FocusAuditReport {
        let _ = audit.stop().await.expect("the foreground hook unhooks");
        let ledger = FocusAuditLedger::new(run_id.to_string(), runtime_root)
            .expect("reopen the focus-audit ledger");
        let events = ledger.events().expect("read the focus-audit ledger");
        let mut owned = OwnedProcessPidSet::default();
        owned.insert(child_pid);
        FocusAuditReport::from_events(run_id.to_string(), std::process::id(), &owned, events)
    }

    /// The fixture child. It is `#[ignore]`d so the normal suite never runs it;
    /// the login launch invokes this binary with `--ignored --exact` so a REAL,
    /// long-lived process exists for the duration of the audit window.
    ///
    /// It stands in for `claude auth login` / `codex login`: it publishes its
    /// pid, writes an interactive PROMPT into the pty, BLOCKS reading the
    /// operator's answer back off the pty, and then echoes what it actually
    /// received. That makes both halves of the MT-015 contract observable in one
    /// process — quiet (it opens nothing) AND completable (it can be read and
    /// answered). A fixture that only slept would prove the quiet half and leave
    /// the half that a bare `CREATE_NO_WINDOW` flip would have destroyed
    /// unproven.
    #[test]
    #[ignore]
    fn quiet_login_fixture_child() {
        use std::io::{BufRead, Write};

        let ready = std::env::var_os(FIXTURE_READY_ENV)
            .expect("the quiet-login fixture child receives its readiness path");
        std::fs::write(&ready, std::process::id().to_string())
            .expect("publish the quiet-login child pid");

        print!("{FIXTURE_PROMPT} ");
        std::io::stdout()
            .flush()
            .expect("the fixture prompt reaches the pty");

        let mut answer = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut answer)
            .expect("the fixture child reads the operator answer off the pty");
        println!("{FIXTURE_ECHO_PREFIX}{}", answer.trim());
        std::io::stdout()
            .flush()
            .expect("the fixture echo reaches the pty");

        std::thread::sleep(Duration::from_secs(30));
    }
}
