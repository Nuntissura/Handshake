// WP-KERNEL-011 Handshake native GUI — binary entrypoint (thin; logic lives in the lib).
// Opens a real native wgpu window (no webview/Tauri/Electron) and runs the egui shell.

use handshake_native::app::HandshakeApp;
use handshake_native::installer;
use handshake_native::quiet_mode::focus_guard;

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
            // Unknown args are ignored (eframe/winit may pass platform args); fall through to GUI.
            _ => {}
        }
    }
    None
}

fn main() -> eframe::Result<()> {
    // MT-031: handle headless CLI flags (--self-check / --version / --help) BEFORE any tracing init or
    // event-loop setup, so the self-check stays a pure path-existence probe with no GUI side effects.
    if let Some(code) = handle_cli_flags() {
        std::process::exit(code);
    }

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
        Box::new(|cc| Ok(Box::new(HandshakeApp::new(cc)))),
    )
}
