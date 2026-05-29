use std::{fs, path::Path};

use handshake_core::operator_foreground::keyboard_inject_test::{
    assert_keyboard_injection_negative, record_keyboard_hook_flags, CmdTracker,
    KeyboardInjectionProbeReport, MutationSentinel, LLKHF_INJECTED_FLAG,
};

#[test]
fn keyboard_injection_contract_wires_ll_hook_and_sendinput() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = fs::read_to_string(manifest.join("Cargo.toml")).expect("read Cargo.toml");
    assert!(cargo.contains("windows = { version = \"0.62.2\""));
    assert!(cargo.contains("Win32_UI_WindowsAndMessaging"));
    assert!(cargo.contains("Win32_UI_Input_KeyboardAndMouse"));

    let source =
        fs::read_to_string(manifest.join("src/operator_foreground/keyboard_inject_test.rs"))
            .expect("read keyboard_inject_test.rs");
    for required in [
        "SetWindowsHookExW",
        "WH_KEYBOARD_LL",
        "KBDLLHOOKSTRUCT",
        "LLKHF_INJECTED",
        "CallNextHookEx",
        "UnhookWindowsHookEx",
        "SendInput",
        "INPUT_KEYBOARD",
        "KEYEVENTF_KEYUP",
        "keyboard_injection_live_probe_requires_explicit_env",
    ] {
        assert!(
            source.contains(required),
            "keyboard_inject_test.rs missing required fragment: {required}"
        );
    }

    for forbidden in ["SetForegroundWindow", ".set_focus(", "SendMessageW"] {
        assert!(
            !source.contains(forbidden),
            "keyboard injection negative test must not foreground or message-drive windows: {forbidden}"
        );
    }
}

#[test]
fn llkhf_injected_flag_detection_is_explicit() {
    let counters = record_keyboard_hook_flags(0);
    assert_eq!(counters.injected_event_count, 0);
    assert_eq!(counters.total_event_count, 1);

    let counters = record_keyboard_hook_flags(LLKHF_INJECTED_FLAG);
    assert_eq!(counters.injected_event_count, 1);
    assert_eq!(counters.total_event_count, 2);
}

#[test]
fn negative_probe_requires_injected_event_and_no_command_or_mutation() {
    let good = KeyboardInjectionProbeReport {
        injected_event_count: 1,
        command_invocation_count: 0,
        state_mutated: false,
    };
    assert_keyboard_injection_negative(&good).expect("good negative probe");

    assert!(
        assert_keyboard_injection_negative(&KeyboardInjectionProbeReport {
            injected_event_count: 0,
            command_invocation_count: 0,
            state_mutated: false,
        })
        .expect_err("missing injected event must fail")
        .to_string()
        .contains("LL hook did not observe injected input")
    );

    assert!(
        assert_keyboard_injection_negative(&KeyboardInjectionProbeReport {
            injected_event_count: 1,
            command_invocation_count: 1,
            state_mutated: false,
        })
        .expect_err("command invocation must fail")
        .to_string()
        .contains("Tauri command handler fired")
    );

    assert!(
        assert_keyboard_injection_negative(&KeyboardInjectionProbeReport {
            injected_event_count: 1,
            command_invocation_count: 0,
            state_mutated: true,
        })
        .expect_err("state mutation must fail")
        .to_string()
        .contains("state mutation occurred")
    );
}

#[test]
fn command_tracker_and_mutation_sentinel_start_clean_and_record_only_explicit_changes() {
    let tracker = CmdTracker::new();
    let sentinel = MutationSentinel::new();

    assert_eq!(tracker.invocation_count(), 0);
    assert!(!sentinel.state_mutated());

    tracker.record_invocation();
    sentinel.mark_mutated();

    assert_eq!(tracker.invocation_count(), 1);
    assert!(sentinel.state_mutated());
}

#[test]
#[ignore = "requires HANDSHAKE_RUN_KEYBOARD_INJECT_LIVE=1 on Windows desktop"]
fn keyboard_injection_live_probe_requires_explicit_env() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/operator_foreground/keyboard_inject_test.rs"),
    )
    .expect("read keyboard_inject_test.rs");
    assert!(source.contains("HANDSHAKE_RUN_KEYBOARD_INJECT_LIVE"));
}
