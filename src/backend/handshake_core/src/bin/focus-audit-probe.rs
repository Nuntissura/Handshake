//! MT-027 — `focus-audit-probe`
//!
//! Real, headless driver for the foreground focus audit so the a2 visual smoke
//! (`tests/visual/a2_smoke.spec.ts`) can produce a **genuine**
//! `FocusAuditReport` instead of hardcoding `handshake_owned_events: []`.
//!
//! The a2 smoke runs in the Playwright + WebView2-CDP harness. When it is
//! attached to the live Handshake WebView2 app it drives the focus audit via
//! the Tauri IPC commands
//! (`kernel_operator_foreground_focus_audit_start` / `_stop`). When it runs
//! against the headless capture fixtures (no `__TAURI__` bridge), it shells out
//! to this binary, which runs the **same** core `FocusAuditHandle::start` /
//! `stop` over the supplied `run_id` and prints the real
//! `FocusAuditReport` as JSON on stdout. Either path exercises the real
//! `operator_foreground::focus_audit` ledger — neither fakes the result.
//!
//! Usage:
//!   focus-audit-probe --run-id <RUN_ID> --runtime-root <DIR>
//!                     [--hold-ms <MS>] [--stop-signal-file <PATH>]
//!
//! Stop synchronization (MT-027 remediation):
//!   The hook stays open until the **driver** says the scenario is done, so the
//!   audit can never unhook before the visual run finishes (which would silently
//!   miss a late foreground steal — a false-negative for HBR-QUIET-001). The
//!   probe stops on the FIRST of these causally-synchronized signals:
//!     * a line (or EOF) arrives on stdin, OR
//!     * `--stop-signal-file` is provided and that file appears on disk.
//!   `--hold-ms` is now only a MINIMUM floor (so the hook is not unhooked in the
//!   same scheduler tick it was installed) and an optional MAXIMUM safety cap
//!   (`--hold-ms 0` disables the cap and waits indefinitely for the stop
//!   signal). When neither stop signal is wired (legacy callers), the probe
//!   falls back to holding for `--hold-ms` and then stopping, preserving the
//!   prior behavior.
//!
//! Output (stdout): a single line of JSON — the serialized `FocusAuditReport`.
//! Exit code 0 on a successful audit cycle (report produced, regardless of
//! whether owned events were seen), non-zero on a real audit error.
//!
//! On platforms without the Win32 foreground hook (the Linux dev lane), the
//! core `FocusAuditHandle::start` returns `FOCUS_AUDIT_UNSUPPORTED_PLATFORM`.
//! The probe surfaces that real error on stderr and exits non-zero so callers
//! can gate honestly rather than fabricate an empty report.

use std::{path::PathBuf, time::Duration};

use handshake_core::operator_foreground::focus_audit::{
    FocusAuditHandle, OwnedProcessPidSet,
};

/// Minimum floor (ms) so the hook is not installed and unhooked in the same
/// scheduler tick. Applied even when an explicit stop signal arrives early.
const MIN_HOLD_FLOOR_MS: u64 = 150;

struct ProbeArgs {
    run_id: String,
    runtime_root: PathBuf,
    /// Maximum hold (ms). `0` means "no cap — wait for the stop signal".
    /// When no stop signal is wired, this doubles as the fixed legacy hold.
    hold_ms: u64,
    /// Optional sentinel file: when it appears on disk the probe stops.
    stop_signal_file: Option<PathBuf>,
}

fn parse_args() -> Result<ProbeArgs, String> {
    let mut run_id: Option<String> = None;
    let mut runtime_root: Option<PathBuf> = None;
    let mut hold_ms: u64 = 300;
    let mut stop_signal_file: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--run-id" => {
                run_id = Some(args.next().ok_or("--run-id requires a value")?);
            }
            "--runtime-root" => {
                runtime_root =
                    Some(PathBuf::from(args.next().ok_or("--runtime-root requires a value")?));
            }
            "--hold-ms" => {
                let raw = args.next().ok_or("--hold-ms requires a value")?;
                hold_ms = raw.parse::<u64>().map_err(|e| format!("invalid --hold-ms: {e}"))?;
            }
            "--stop-signal-file" => {
                stop_signal_file = Some(PathBuf::from(
                    args.next().ok_or("--stop-signal-file requires a value")?,
                ));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(ProbeArgs {
        run_id: run_id.ok_or("--run-id is required")?,
        runtime_root: runtime_root.ok_or("--runtime-root is required")?,
        hold_ms,
        stop_signal_file,
    })
}

#[tokio::main]
async fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("focus-audit-probe: {error}");
            std::process::exit(2);
        }
    };

    match run(args).await {
        Ok(json) => {
            println!("{json}");
        }
        Err(error) => {
            eprintln!("focus-audit-probe: {error}");
            std::process::exit(1);
        }
    }
}

async fn run(args: ProbeArgs) -> Result<String, String> {
    let handle = FocusAuditHandle::start(
        args.run_id.clone(),
        &args.runtime_root,
        OwnedProcessPidSet::default(),
    )
    .await
    .map_err(|error| error.to_string())?;

    // Hold the hook open until the driver signals the scenario completed, so
    // the live foreground-event drain task observes EVERY window the visual run
    // brings to the foreground — including late steals. The stop is causally
    // synchronized to the scenario (stdin line/EOF or sentinel file), not a
    // fixed timer that could expire mid-scenario.
    wait_for_stop(args.hold_ms, args.stop_signal_file.as_deref()).await;

    let report = handle.stop().await.map_err(|error| error.to_string())?;
    serde_json::to_string(&report).map_err(|error| error.to_string())
}

/// Block until the driver signals the scenario is complete.
///
/// Stops on the FIRST of:
///   * a line or EOF on stdin (the primary, causally-synchronized signal the
///     driver sends after `await scenario()`),
///   * the `stop_signal_file` appearing on disk (a fallback signal),
///   * the `hold_ms` safety cap elapsing (when `hold_ms > 0`).
///
/// A `MIN_HOLD_FLOOR_MS` floor is always honored so the hook is never unhooked
/// in the same tick it was installed, even if a stop signal is already pending.
///
/// Legacy behavior: when neither stdin nor a sentinel is usable as a signal and
/// `hold_ms > 0`, this collapses to "sleep `hold_ms` then stop".
async fn wait_for_stop(hold_ms: u64, stop_signal_file: Option<&std::path::Path>) {
    let floor = Duration::from_millis(MIN_HOLD_FLOOR_MS.min(hold_ms.max(MIN_HOLD_FLOOR_MS)));
    let started = std::time::Instant::now();

    // Primary signal: a line or EOF on stdin. The driver closes/ writes stdin
    // after the scenario resolves. We run the blocking stdin read on a dedicated
    // thread so it does not stall the tokio runtime / drain task.
    let (tx, mut rx) = tokio::sync::oneshot::channel::<()>();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let mut line = String::new();
        // read_line returns Ok(0) on EOF and Ok(>0) on a line; either is "stop".
        let _ = std::io::stdin().lock().read_line(&mut line);
        let _ = tx.send(());
    });

    let stop_file = stop_signal_file.map(|p| p.to_path_buf());

    loop {
        // Always honor the minimum floor before stopping for any reason.
        if started.elapsed() < floor {
            tokio::time::sleep(floor - started.elapsed()).await;
            continue;
        }

        // 1) stdin line / EOF (primary, scenario-synchronized).
        match rx.try_recv() {
            Ok(()) | Err(tokio::sync::oneshot::error::TryRecvError::Closed) => return,
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
        }

        // 2) sentinel file appeared (fallback, scenario-synchronized).
        if let Some(ref path) = stop_file {
            if path.exists() {
                return;
            }
        }

        // 3) safety cap: only when hold_ms > 0. hold_ms == 0 waits indefinitely
        //    for an explicit stop signal.
        if hold_ms > 0 && started.elapsed() >= Duration::from_millis(hold_ms) {
            return;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
