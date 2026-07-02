//! WP-KERNEL-012 MT-104: internal manual content for notes/chat/terminal/model/diagnostics.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::app::{
    HandshakeApp, MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID, MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID, MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_START_AUTHOR_ID, MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID, TERMINAL_LAUNCH_STATUS_AUTHOR_ID,
};
use handshake_native::manual_content_editors::{
    agent_tool_rows, editors_manual_section, DIAGNOSTIC_TOOL_HEADINGS,
    FLIGHT_RECORDER_MENU_AUTHOR_ID, FLIGHT_RECORDER_PALETTE_AUTHOR_ID,
    INFERENCE_LAB_MENU_AUTHOR_ID, INFERENCE_LAB_PALETTE_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_MENU_AUTHOR_ID, MODEL_SESSION_LAUNCH_PALETTE_AUTHOR_ID,
    SETTINGS_DIAGNOSTICS_SECTION_AUTHOR_ID, TERMINAL_MENU_AUTHOR_ID, WP104_PRODUCT_HEADINGS,
};
use handshake_native::manual_pane::{
    manual_topic_author_id, ManualPane, ManualPaneState, ManualRegistry, ManualSection,
    ManualSurface,
};
use handshake_native::theme::HsPalette;

const REAL_MCP_TOOLS: &[&str] = &["list_widgets", "click_widget", "set_value", "screenshot"];

fn mt104_headings() -> impl Iterator<Item = &'static str> {
    WP104_PRODUCT_HEADINGS
        .iter()
        .chain(DIAGNOSTIC_TOOL_HEADINGS.iter())
        .copied()
}

fn topic_body<'a>(section: &'a ManualSection, heading: &str) -> &'a str {
    section
        .topic(heading)
        .unwrap_or_else(|| panic!("MT-104 manual topic '{heading}' must exist"))
        .body
        .as_str()
}

fn row_by_id() -> HashMap<&'static str, handshake_native::manual_pane::AgentToolRow> {
    agent_tool_rows()
        .into_iter()
        .map(|row| (row.author_id, row))
        .collect()
}

fn body_marker(heading: &str) -> &'static str {
    match heading {
        "Notes Worksurface and Chat" => "pane-a is the Code editor",
        "Opening Editing and Saving Notes" => "GET /knowledge/documents/:id",
        "Terminal Launch" => {
            "EndpointMissing: native terminal launch needs HTTP /terminal/sessions"
        }
        "Model Session Launch" => "NEEDS_MANAGED_RESOURCE_PROOF",
        "Settings Diagnostics" => "diagnostics_heartbeat",
        "Visual Debugger" => "hsk.native_worksurface_inspector@1",
        "Foreground-Safe Navigation" => "NavigationSequence::dispatch_step",
        "Flight Recorder" => "canonical replay/audit record",
        "internal_diagnostics" => "process-global diagnostic-event API",
        "Palmistry" => "external out-of-process watcher",
        other => panic!("unknown MT-104 topic '{other}'"),
    }
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> HashSet<String> {
    let mut ids = HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(author_id) = node.accesskit_node().author_id() {
            ids.insert(author_id.to_owned());
        }
    }
    ids
}

fn live_author_node_state(
    harness: &Harness<'_, HandshakeApp>,
    author_id: &str,
) -> Option<(String, bool, Option<String>)> {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .map(|node| {
            let ak = node.accesskit_node();
            (format!("{:?}", ak.role()), ak.is_disabled(), ak.label())
        })
}

#[test]
fn mt104_topics_exist_and_include_no_context_runtime_facts() {
    let section = editors_manual_section();

    for heading in mt104_headings() {
        let body = topic_body(&section, heading);
        assert!(
            body.len() > 220,
            "MT-104 topic '{heading}' must be substantive no-context guidance"
        );
    }

    for (heading, needles) in [
        (
            "Notes Worksurface and Chat",
            &[
                "pane-a",
                "pane-b",
                "pane-c",
                "runtime-chat-input",
                "EndpointMissing",
            ][..],
        ),
        (
            "Opening Editing and Saving Notes",
            &[
                "GET /knowledge/documents/:id",
                "PUT /knowledge/documents/:id/save",
                "EventLedger",
            ],
        ),
        (
            "Terminal Launch",
            &[
                "menu.run.terminal",
                "EndpointMissing",
                "IPC-only",
                "/terminal",
                "terminal-launch-status",
            ],
        ),
        (
            "Model Session Launch",
            &[
                "menu.run.model-session-launch",
                "command-palette.option.hs-model-session-palette-launch-workspace",
                "model-session-launch.folder",
                "model-session-launch.model",
                "model-session-launch.wrapper",
                "POST /jobs",
                "IPC-only",
                "kernel_swarm_spawn_session",
                "NEEDS_MANAGED_RESOURCE_PROOF",
                "EndpointMissing",
            ],
        ),
        (
            "Settings Diagnostics",
            &[
                "Settings -> Diagnostics",
                "settings.search",
                "settings.section.diagnostics",
                "diagnostics_panel",
                "diagnostics_palmistry",
                "child-process stall",
            ],
        ),
        (
            "Visual Debugger",
            &[
                "settings.diagnostics.worksurface-inspector.dump",
                "hsk.native_worksurface_inspector@1",
                "screenshot_deferred_headless_gpu",
            ],
        ),
        (
            "Foreground-Safe Navigation",
            &[
                "NavigationSequence::dispatch_step",
                "list_widgets",
                "set_value",
                "SendInput",
                "NavigationError",
            ],
        ),
        (
            "Flight Recorder",
            &[
                "Tier 1",
                "GET /events",
                "menu.run.flight-recorder",
                "command-palette.option.hs-flight-palette-open",
            ],
        ),
        (
            "internal_diagnostics",
            &[
                "Tier 2",
                "diagnostics_heartbeat",
                "diagnostics_events",
                "operation watchdog",
                "StalledOperation",
                "OperationCode::BackendCall",
                "last_progress_ms",
                "Stalled ops",
                "Settings -> Diagnostics",
            ],
        ),
        (
            "Palmistry",
            &[
                "Tier 3",
                "freeze",
                "crash",
                "ChildStall",
                "RegisterChild",
                "file-counter",
                "HANDSHAKE_PALMISTRY_SURVIVOR_DIR",
                "child_session_id",
                "child_stall_reason_code",
                "test_no_silent_hang_end_to_end",
                "diagnostics_palmistry",
            ],
        ),
    ] {
        let body = topic_body(&section, heading);
        for needle in needles {
            assert!(
                body.contains(needle),
                "topic '{heading}' must include concrete runtime fact '{needle}'"
            );
        }
    }
}

#[test]
fn mt104_terminal_and_model_topics_are_honest_blockers() {
    let section = editors_manual_section();
    let terminal = topic_body(&section, "Terminal Launch");
    let model = topic_body(&section, "Model Session Launch");

    for bad in ["fully working terminal", "terminal opened successfully"] {
        assert!(
            !terminal.contains(bad),
            "terminal topic must not advertise a fabricated terminal path: {bad}"
        );
    }
    for bad in ["fully working model", "model session is running"] {
        assert!(
            !model.contains(bad),
            "model topic must not advertise fabricated model execution: {bad}"
        );
    }
    assert!(
        terminal.contains("click menu.run.terminal") && terminal.contains("terminal-launch-status"),
        "terminal blocker should be clickable into a typed status, not disabled-only guidance"
    );
    assert!(
        model.contains("must not fabricate a session id"),
        "model topic must explicitly forbid fabricated session state"
    );
    assert!(
        model.contains("model-session-launch.provider.local")
            && model.contains("model-session-launch.provider.cloud"),
        "model topic must document provider row ids for no-context steering"
    );
    assert!(
        model.contains("settings.model-session.open-launch")
            && model.contains("launch-dialog seeds, not persistent hidden model defaults"),
        "model topic must document the wired Settings action without implying hidden durable defaults"
    );
}

#[test]
fn mt104_agent_tool_reference_adds_real_terminal_model_diagnostics_rows() {
    let rows = row_by_id();
    let required = [
        (
            TERMINAL_MENU_AUTHOR_ID,
            ManualSurface::Terminal,
            "click_widget",
        ),
        (
            TERMINAL_LAUNCH_STATUS_AUTHOR_ID,
            ManualSurface::Terminal,
            "list_widgets",
        ),
        (
            MODEL_SESSION_LAUNCH_MENU_AUTHOR_ID,
            ManualSurface::Model,
            "click_widget",
        ),
        (
            MODEL_SESSION_LAUNCH_PALETTE_AUTHOR_ID,
            ManualSurface::Model,
            "click_widget",
        ),
        (
            MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID,
            ManualSurface::Model,
            "click_widget",
        ),
        (
            MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID,
            ManualSurface::Model,
            "click_widget",
        ),
        (
            MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID,
            ManualSurface::Model,
            "set_value",
        ),
        (
            MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID,
            ManualSurface::Model,
            "set_value",
        ),
        (
            MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID,
            ManualSurface::Model,
            "set_value",
        ),
        (
            MODEL_SESSION_LAUNCH_START_AUTHOR_ID,
            ManualSurface::Model,
            "click_widget",
        ),
        (
            MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID,
            ManualSurface::Model,
            "list_widgets",
        ),
        (
            INFERENCE_LAB_MENU_AUTHOR_ID,
            ManualSurface::Model,
            "click_widget",
        ),
        (
            INFERENCE_LAB_PALETTE_AUTHOR_ID,
            ManualSurface::Model,
            "click_widget",
        ),
        (
            FLIGHT_RECORDER_MENU_AUTHOR_ID,
            ManualSurface::Diagnostics,
            "click_widget",
        ),
        (
            FLIGHT_RECORDER_PALETTE_AUTHOR_ID,
            ManualSurface::Diagnostics,
            "click_widget",
        ),
        (
            handshake_native::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID,
            ManualSurface::Diagnostics,
            "set_value",
        ),
        (
            SETTINGS_DIAGNOSTICS_SECTION_AUTHOR_ID,
            ManualSurface::Diagnostics,
            "click_widget",
        ),
        (
            handshake_native::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
            ManualSurface::Diagnostics,
            "list_widgets",
        ),
        (
            handshake_native::visual_debugger::WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID,
            ManualSurface::Diagnostics,
            "click_widget",
        ),
        (
            handshake_native::visual_debugger::WORKSURFACE_INSPECTOR_STATUS_AUTHOR_ID,
            ManualSurface::Diagnostics,
            "list_widgets",
        ),
    ];

    for (author_id, surface, tool) in required {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("agent-tool row '{author_id}' must exist"));
        assert_eq!(row.surface, surface, "row '{author_id}' surface");
        assert_eq!(row.mcp_tool, tool, "row '{author_id}' tool");
    }

    assert_eq!(
        rows.get(TERMINAL_MENU_AUTHOR_ID).unwrap().mcp_tool,
        "click_widget",
        "terminal menu item must be runnable into terminal-launch-status"
    );

    for row in rows.values() {
        assert!(
            REAL_MCP_TOOLS.contains(&row.mcp_tool),
            "row '{}' uses non-real MCP tool '{}'",
            row.author_id,
            row.mcp_tool
        );
        assert!(
            !row.mcp_tool.starts_with("gui."),
            "row '{}' must not use invented gui.* tools",
            row.author_id
        );
    }
}

#[test]
fn mt104_terminal_menu_author_id_is_live_clickable_run_leaf() {
    let rows = row_by_id();
    let terminal = rows
        .get(TERMINAL_MENU_AUTHOR_ID)
        .expect("terminal agent-tool row exists");
    assert_eq!(terminal.mcp_tool, "click_widget");
    assert_eq!(terminal.surface, ManualSurface::Terminal);

    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_eframe(|cc| HandshakeApp::new(cc));
    harness.run_steps(4);

    assert!(
        live_author_node_state(&harness, TERMINAL_MENU_AUTHOR_ID).is_none(),
        "terminal leaf should be dynamic and absent while RUN menu is closed"
    );

    harness.get_by_label("RUN").click();
    harness.step();
    harness.step();

    let (role, disabled, label) = live_author_node_state(&harness, TERMINAL_MENU_AUTHOR_ID)
        .unwrap_or_else(|| panic!("RUN menu must render '{TERMINAL_MENU_AUTHOR_ID}'"));
    assert_eq!(role, "MenuItem", "terminal leaf AccessKit role");
    assert!(
        !disabled,
        "terminal leaf must be clickable so it can surface terminal-launch-status"
    );
    assert_eq!(
        label.as_deref(),
        Some("Open Terminal in Workspace Folder"),
        "terminal leaf label"
    );
}

#[test]
fn mt104_agent_tool_reference_rejects_raw_command_stable_ids() {
    let row_ids: HashSet<&str> = agent_tool_rows().iter().map(|row| row.author_id).collect();

    for raw in [
        "hs-inference-palette-open",
        "hs-flight-palette-open",
        "hs-model-session-palette-launch-workspace",
    ] {
        assert!(
            !row_ids.contains(raw),
            "agent-tool rows must use generated command-palette option ids, not raw stable id '{raw}'"
        );
    }
    for generated in [
        "command-palette.option.hs-model-session-palette-launch-workspace",
        "command-palette.option.hs-inference-palette-open",
        "command-palette.option.hs-flight-palette-open",
    ] {
        assert!(
            row_ids.contains(generated),
            "agent-tool rows must include live generated id '{generated}'"
        );
    }
}

#[test]
fn mt104_manual_topics_are_selectable_in_manual_pane() {
    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    let reg: &'static ManualRegistry = Box::leak(Box::new(reg));
    let palette: &'static HsPalette = Box::leak(Box::new(HsPalette::dark()));
    let state = Rc::new(RefCell::new(ManualPaneState::default()));
    let ui_state = Rc::clone(&state);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(940.0, 680.0))
        .build_ui(move |ui| {
            let mut state = ui_state.borrow_mut();
            ManualPane::new(reg, &mut state, palette).show(ui);
        });
    harness.run();

    for heading in mt104_headings() {
        {
            let mut state = state.borrow_mut();
            state.query = heading.to_owned();
            state.selected = None;
        }
        harness.run();
        harness.run();

        let author_id = manual_topic_author_id("native-editors", heading);
        harness
            .get_by(|node| node.author_id() == Some(author_id.as_str()))
            .click();
        harness.run();
        harness.run();

        let selected = {
            let state = state.borrow();
            state.selected.clone()
        };
        assert_eq!(
            selected,
            Some(("native-editors".to_owned(), heading.to_owned())),
            "clicking topic '{heading}' should update ManualPaneState"
        );
        let marker = body_marker(heading);
        assert!(
            harness.query_by_label_contains(marker).is_some(),
            "selecting topic '{heading}' should render body marker '{marker}'"
        );
    }
}

#[test]
fn mt104_settings_diagnostics_ids_are_live_after_settings_search() {
    let mut harness: Harness<HandshakeApp> = Harness::builder()
        .with_size(egui::vec2(900.0, 760.0))
        .build_eframe(|cc| HandshakeApp::new(cc));

    harness.run_steps(4);
    harness.state_mut().open_settings();
    harness.step();

    harness.get_by_label("Search settings").focus();
    harness.step();
    harness
        .get_by_label("Search settings")
        .type_text("diagnostics");
    harness.run_steps(3);

    let ids = live_author_ids(&harness);
    for expected in [
        handshake_native::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID,
        SETTINGS_DIAGNOSTICS_SECTION_AUTHOR_ID,
        handshake_native::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
        handshake_native::diagnostics::DIAGNOSTICS_HEARTBEAT_AUTHOR_ID,
        handshake_native::diagnostics::DIAGNOSTICS_FRAME_AUTHOR_ID,
        handshake_native::diagnostics::DIAGNOSTICS_RESOURCE_AUTHOR_ID,
        handshake_native::diagnostics::DIAGNOSTICS_EVENTS_AUTHOR_ID,
        handshake_native::diagnostics::DIAGNOSTICS_PALMISTRY_AUTHOR_ID,
        handshake_native::visual_debugger::WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID,
        handshake_native::visual_debugger::WORKSURFACE_INSPECTOR_STATUS_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(expected),
            "Settings search for diagnostics must render live author_id '{expected}'; diagnostics-ish ids were {:?}",
            ids.iter()
                .filter(|id| id.contains("diagnostics"))
                .collect::<Vec<_>>()
        );
    }
}
