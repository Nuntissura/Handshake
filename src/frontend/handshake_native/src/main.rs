// WP-KERNEL-011 Handshake native GUI — binary entrypoint (thin; logic lives in the lib).
// Opens a real native wgpu window (no webview/Tauri/Electron) and runs the egui shell.

use handshake_native::app::HandshakeApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "handshake_native=debug,eframe=info".into()),
        )
        .init();

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
