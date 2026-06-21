// WP-KERNEL-011 Handshake native GUI — binary entrypoint (thin; logic lives in the lib).
// Opens a real native wgpu window (no webview/Tauri/Electron) and runs the egui shell.

use handshake_native::app::HandshakeApp;
use handshake_native::quiet_mode::focus_guard;

fn main() -> eframe::Result<()> {
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
