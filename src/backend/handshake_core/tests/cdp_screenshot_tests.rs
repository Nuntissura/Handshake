use base64::{engine::general_purpose, Engine as _};
use serde_json::json;

use handshake_core::{
    operator_foreground::cdp_client::{
        build_webview2_cdp_process_start, decode_capture_screenshot_response, page_capture_request,
        ScreenshotClip, ScreenshotOptions, ScreenshotScope, VisualDebugLaunchConfig,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS, WEBVIEW2_USER_DATA_FOLDER,
    },
    process_ledger::ProcessEngineKind,
};

#[test]
fn cdp_screenshot_tests_launch_config_sets_random_port_and_isolated_webview2_env() {
    let temp = tempfile::tempdir().expect("temp root");

    let config = VisualDebugLaunchConfig::new(temp.path(), "KERNEL_BUILDER", Some("MT-018".into()))
        .expect("launch config");

    assert!(config.remote_debugging_port > 0);
    assert!(config.user_data_folder.starts_with(temp.path()));
    assert!(config.user_data_folder.ends_with(format!(
        "handshake-webview2-cdp-{}",
        config.remote_debugging_port
    )));
    assert_eq!(
        config.env.get(WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS),
        Some(&format!(
            "--remote-debugging-port={}",
            config.remote_debugging_port
        ))
    );
    assert_eq!(
        config.env.get(WEBVIEW2_USER_DATA_FOLDER),
        Some(&config.user_data_folder.to_string_lossy().to_string())
    );
    assert_eq!(
        config.ledger_start.engine_kind,
        ProcessEngineKind::Webview2Cdp
    );
    assert_eq!(
        config.ledger_start.sandbox_adapter_id.as_deref(),
        Some("webview2-cdp")
    );
}

#[test]
fn cdp_screenshot_tests_capture_screenshot_request_and_png_decoding_are_cdp_shaped() {
    let request = page_capture_request(
        7,
        ScreenshotScope::Region(ScreenshotClip {
            x: 10.0,
            y: 20.0,
            width: 320.0,
            height: 180.0,
            scale: 1.0,
        }),
        ScreenshotOptions {
            capture_beyond_viewport: false,
            from_surface: true,
        },
    )
    .expect("request");

    assert_eq!(request["id"], 7);
    assert_eq!(request["method"], "Page.captureScreenshot");
    assert_eq!(request["params"]["format"], "png");
    assert_eq!(request["params"]["clip"]["x"], 10.0);
    assert_eq!(request["params"]["clip"]["width"], 320.0);
    assert_eq!(request["params"]["captureBeyondViewport"], false);
    assert_eq!(request["params"]["fromSurface"], true);

    let png_1x1 = general_purpose::STANDARD.encode([
        0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 0, 0, 0, 0,
    ]);
    let response = json!({
        "id": 7,
        "result": {
            "data": png_1x1
        }
    });
    let bytes = decode_capture_screenshot_response(&response).expect("png response");

    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
}

#[test]
fn cdp_screenshot_tests_process_ledger_registration_uses_webview2_cdp_engine() {
    let start = build_webview2_cdp_process_start(
        Some(4242),
        "SR-CDP-SCREENSHOT",
        "KERNEL_BUILDER",
        Some("WP-KERNEL-004".to_string()),
    );

    assert_eq!(start.engine_kind, ProcessEngineKind::Webview2Cdp);
    assert_eq!(start.engine_kind.as_str(), "webview2_cdp");
    assert_eq!(start.os_pid, Some(4242));
    assert_eq!(
        start.parent_session_id.as_deref(),
        Some("SR-CDP-SCREENSHOT")
    );
    assert_eq!(start.sandbox_adapter_id.as_deref(), Some("webview2-cdp"));
    assert_eq!(start.owner_role, "KERNEL_BUILDER");
    assert_eq!(start.owner_wp.as_deref(), Some("WP-KERNEL-004"));
}
