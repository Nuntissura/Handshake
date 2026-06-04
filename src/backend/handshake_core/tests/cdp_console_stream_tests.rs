use serde_json::json;

use handshake_core::{
    model_manual::{model_manual, CommandStatus},
    operator_foreground::cdp_client::{
        decode_console_stream_event, runtime_enable_request, ConsoleEvent, ConsoleScope,
    },
};

#[test]
fn cdp_console_stream_tests_runtime_enable_request_is_cdp_shaped() {
    let request = runtime_enable_request(11);

    assert_eq!(request["id"], 11);
    assert_eq!(request["method"], "Runtime.enable");
}

#[test]
fn cdp_console_stream_tests_decodes_console_api_calls() {
    let event = decode_console_stream_event(&json!({
        "method": "Runtime.consoleAPICalled",
        "params": {
            "type": "error",
            "timestamp": 42.5,
            "args": [
                { "type": "string", "value": "failed" },
                { "type": "number", "value": 7 },
                { "type": "object", "description": "Error: boom" }
            ]
        }
    }))
    .expect("decode")
    .expect("event");

    assert_eq!(
        event,
        ConsoleEvent::Log {
            level: "error".to_string(),
            message: "failed 7 Error: boom".to_string(),
            timestamp: 42.5,
        }
    );
}

#[test]
fn cdp_console_stream_tests_decodes_unhandled_exceptions() {
    let event = decode_console_stream_event(&json!({
        "method": "Runtime.exceptionThrown",
        "params": {
            "timestamp": 43.0,
            "exceptionDetails": {
                "text": "Uncaught",
                "exception": {
                    "description": "Error: inject-test-runtime-exception"
                },
                "stackTrace": {
                    "callFrames": [{
                        "functionName": "fixture",
                        "url": "app://local"
                    }]
                }
            }
        }
    }))
    .expect("decode")
    .expect("event");

    let ConsoleEvent::Exception {
        message,
        stack,
        timestamp,
    } = event
    else {
        panic!("expected exception event");
    };
    assert_eq!(message, "Uncaught");
    assert_eq!(timestamp, 43.0);
    assert!(stack.expect("stack").contains("callFrames"));
}

#[test]
fn cdp_console_stream_tests_ignores_non_runtime_events_and_serializes_scope() {
    let event = decode_console_stream_event(&json!({
        "method": "Page.loadEventFired",
        "params": { "timestamp": 1.0 }
    }))
    .expect("decode");
    assert!(event.is_none());

    let scope = serde_json::to_value(ConsoleScope::All).expect("scope");
    assert_eq!(scope["kind"], "all");
}

#[test]
fn cdp_console_stream_tests_manual_and_tauri_registration_are_paired() {
    let manual = model_manual();
    for (id, ipc, tauri_command) in [
        (
            "visual_debug_console_stream_start",
            "kernel.visual_debug.console_stream.start",
            "kernel_visual_debug_console_stream_start",
        ),
        (
            "visual_debug_console_stream_stop",
            "kernel.visual_debug.console_stream.stop",
            "kernel_visual_debug_console_stream_stop",
        ),
    ] {
        let command = manual
            .command_reference
            .iter()
            .find(|command| command.id == id)
            .unwrap_or_else(|| panic!("manual entry for {id}"));
        assert_eq!(command.status, CommandStatus::Wired);
        assert_eq!(command.ipc_channel, Some(ipc));
        assert_eq!(command.tauri_command, Some(tauri_command));
    }

    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .and_then(std::path::Path::parent)
        .expect("repo root");
    let lib_rs = std::fs::read_to_string(repo_root.join("app/src-tauri/src/lib.rs")).unwrap();
    let visual_debug_rs =
        std::fs::read_to_string(repo_root.join("app/src-tauri/src/visual_debug.rs")).unwrap();

    assert!(lib_rs.contains("visual_debug::kernel_visual_debug_console_stream_start"));
    assert!(lib_rs.contains("visual_debug::kernel_visual_debug_console_stream_stop"));
    assert!(visual_debug_rs.contains("pub async fn kernel_visual_debug_console_stream_start"));
    assert!(visual_debug_rs.contains("pub fn kernel_visual_debug_console_stream_stop"));
    assert!(visual_debug_rs.contains("console_event"));
}
