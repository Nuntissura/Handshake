//! The typed, readable unavailable state (MT-214 AC214-9): no device at the start, or none after a loss. No device, no
//! window, no desktop and no thread: the state is a plain value, so it runs on any machine and cannot touch the
//! Operator's desktop.
//!
//! The drawn presentation waits for the UI layer (Kumihimo, not published), and no other renderer exists to draw it with;
//! what is checked here is what the shell can state without drawing: the typed exit, the sentence in the accessibility
//! tree, and the receipt line.

use std::collections::BTreeSet;

use handshake_shell::{
    unavailable_after, Cause, ExitKind, UnavailableState, When, HEADLINE,
};
use handshake_shell_surface::{DeviceEvent, GaveUp};
use handshake_vulkan_renderer::Unavailable;

fn after_loss() -> [UnavailableState; 3] {
    [
        UnavailableState::after_loss(GaveUp::RepeatedLoss),
        UnavailableState::after_loss(GaveUp::Rebind("device_not_quiescent")),
        UnavailableState::after_loss(GaveUp::Rebind("owner_closed")),
    ]
}

#[test]
fn every_reason_the_owner_gives_is_a_typed_state_that_ends_with_code_4() {
    for reason in Unavailable::ALL {
        let state = UnavailableState::at_start(reason);
        assert_eq!(state.cause(), Cause::Device(reason));
        assert_eq!(state.when(), When::AtStart);
        assert_eq!(state.code(), reason.key(), "the owner's own code is the state's code");
        assert_eq!(state.end(), ExitKind::DeviceUnavailable, "{}", reason.key());
        assert_eq!(state.end().code(), 4);
    }
    for state in after_loss() {
        assert_eq!(state.when(), When::AfterLoss);
        assert_eq!(state.end(), ExitKind::DeviceUnavailable);
    }
}

#[test]
fn the_sentence_is_readable_says_why_and_that_nothing_else_will_draw() {
    let mut sentences = BTreeSet::new();
    for reason in Unavailable::ALL {
        let state = UnavailableState::at_start(reason);
        let text = state.text();
        assert!(text.starts_with(HEADLINE), "{text}");
        assert!(text.contains(&format!("reason {}", reason.key())), "{text}");
        assert!(text.contains("at start"), "{text}");
        assert!(text.contains("Nothing else will draw it"), "{text}");
        assert!(text.contains("exits with code 4"), "{text}");
        assert!(text.ends_with('.'), "{text}");
        assert!(text.len() > HEADLINE.len() + 60, "a sentence, not a code: {text}");
        // Without the code: the words alone tell the reason apart.
        sentences.insert(text.replace(reason.key(), ""));
    }
    assert_eq!(
        sentences.len(),
        Unavailable::ALL.len(),
        "each reason has its own sentence: a reader learns which one it was"
    );
    let loss: Vec<String> = after_loss().iter().map(UnavailableState::text).collect();
    assert!(loss.iter().all(|text| text.contains("after it was lost")), "{loss:?}");
    assert!(loss[0].contains("again and again"), "{}", loss[0]);
    assert!(loss[1].contains("could not be recovered") && loss[1].contains("device_not_quiescent"), "{}", loss[1]);
    assert_ne!(loss[0], loss[1]);
}

#[test]
fn the_accessibility_tree_carries_the_sentence() {
    for state in after_loss()
        .into_iter()
        .chain(Unavailable::ALL.into_iter().map(UnavailableState::at_start))
    {
        let frame = state.frame();
        assert_eq!(frame.name.as_deref(), Some(HEADLINE), "the window is named by the headline");
        assert_eq!(frame.top.len(), 1, "one group in the window");
        assert_eq!(frame.nodes.len(), 1);
        let text = state.text();
        assert_eq!(
            frame.nodes[0].node.label(),
            Some(text.as_str()),
            "the group's name is the sentence an assistive tool reads"
        );
    }
}

#[test]
fn the_receipt_line_names_codes_and_says_nothing_was_drawn_and_nothing_else_will_be() {
    for state in after_loss()
        .into_iter()
        .chain(Unavailable::ALL.into_iter().map(UnavailableState::at_start))
    {
        let line = state.line();
        assert!(
            line.starts_with("HSK_SHELL_UNAVAILABLE kind=device_unavailable code=4 "),
            "{line}"
        );
        assert!(line.contains(&format!("reason={}", state.code())), "{line}");
        assert!(line.contains(&format!("when={}", state.when().name())), "{line}");
        assert!(line.contains("drawn=false drawn_waits_for=ui_layer"), "{line}");
        assert!(line.contains("fallback_renderer=none"), "{line}");
        assert!(!line.contains('\n'), "one line");
    }
    assert_eq!(When::AtStart.name(), "at_start");
    assert_eq!(When::AfterLoss.name(), "after_loss");
}

#[test]
fn only_a_device_that_is_gone_for_good_ends_the_program() {
    assert_eq!(unavailable_after(DeviceEvent::Lost { generation: Some(1) }), None, "a loss is recovered");
    assert_eq!(unavailable_after(DeviceEvent::Lost { generation: None }), None);
    assert_eq!(
        unavailable_after(DeviceEvent::Rebound { generation: 2, windows: 3 }),
        None,
        "a rebind means every window draws again"
    );
    for why in [GaveUp::RepeatedLoss, GaveUp::Rebind("device_held")] {
        let state = unavailable_after(DeviceEvent::GaveUp(why)).expect("the end of recovery ends the program");
        assert_eq!(state, UnavailableState::after_loss(why));
        assert_eq!(state.end(), ExitKind::DeviceUnavailable);
        assert_eq!(state.code(), why.code());
    }
}
