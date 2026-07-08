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
use handshake_native::graph::{
    MODE_GLOBAL_AUTHOR_ID as GRAPH_MODE_GLOBAL_AUTHOR_ID,
    MODE_LOCAL_AUTHOR_ID as GRAPH_MODE_LOCAL_AUTHOR_ID,
    RELAYOUT_AUTHOR_ID as GRAPH_RELAYOUT_AUTHOR_ID, ZOOM_IN_AUTHOR_ID as GRAPH_ZOOM_IN_AUTHOR_ID,
    ZOOM_OUT_AUTHOR_ID as GRAPH_ZOOM_OUT_AUTHOR_ID,
};
use handshake_native::manual_content_editors::{
    agent_tool_rows, editors_manual_section, CONFLICT_KEEP_SERVER_AUTHOR_ID,
    CONFLICT_KEEP_YOURS_AUTHOR_ID, CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID,
    CONFLICT_OPEN_MERGE_AUTHOR_ID, DIAGNOSTIC_TOOL_HEADINGS, DRAFT_BANNER_AUTHOR_ID,
    DRAFT_DISCARD_AUTHOR_ID, DRAFT_RESTORE_AUTHOR_ID, EXPORT_FORMAT_PICKER_AUTHOR_ID,
    FLIGHT_RECORDER_MENU_AUTHOR_ID, FLIGHT_RECORDER_PALETTE_AUTHOR_ID,
    FOLDER_TREE_COLOR_AUTHOR_ID_PATTERN, FOLDER_TREE_NODE_AUTHOR_ID_PATTERN,
    FOLDER_TREE_RETRY_AUTHOR_ID, INFERENCE_LAB_MENU_AUTHOR_ID, INFERENCE_LAB_PALETTE_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_MENU_AUTHOR_ID, MODEL_SESSION_LAUNCH_PALETTE_AUTHOR_ID,
    RICH_EDITOR_EXPORT_BUTTON_AUTHOR_ID, SETTINGS_DIAGNOSTICS_SECTION_AUTHOR_ID,
    TAGS_SEARCH_AUTHOR_ID, TAG_HUB_ADD_TAG_AUTHOR_ID_PATTERN, TAG_HUB_MEMBER_AUTHOR_ID_PATTERN,
    TAG_HUB_TITLE_AUTHOR_ID_PATTERN, TAG_ROW_AUTHOR_ID_PATTERN, TERMINAL_MENU_AUTHOR_ID,
    VIEW_OPEN_BLOCK_COLLECTIONS_MENU_AUTHOR_ID, VIEW_OPEN_CANVAS_MENU_AUTHOR_ID,
    VIEW_OPEN_CANVAS_PALETTE_AUTHOR_ID, VIEW_OPEN_CODE_EDITOR_MENU_AUTHOR_ID,
    VIEW_OPEN_CODE_EDITOR_PALETTE_AUTHOR_ID, VIEW_OPEN_DAILY_JOURNAL_MENU_AUTHOR_ID,
    VIEW_OPEN_DAILY_JOURNAL_PALETTE_AUTHOR_ID, VIEW_OPEN_DIFF_EDITOR_MENU_AUTHOR_ID,
    VIEW_OPEN_DIFF_EDITOR_PALETTE_AUTHOR_ID, VIEW_OPEN_FIND_IN_FILES_MENU_AUTHOR_ID,
    VIEW_OPEN_FIND_IN_FILES_PALETTE_AUTHOR_ID, VIEW_OPEN_FOLDERS_MENU_AUTHOR_ID,
    VIEW_OPEN_FOLDERS_PALETTE_AUTHOR_ID, VIEW_OPEN_KNOWLEDGE_GRAPH_MENU_AUTHOR_ID,
    VIEW_OPEN_KNOWLEDGE_GRAPH_PALETTE_AUTHOR_ID, VIEW_OPEN_LOOM_SEARCH_MENU_AUTHOR_ID,
    VIEW_OPEN_LOOM_SEARCH_PALETTE_AUTHOR_ID, VIEW_OPEN_QUICK_SWITCHER_MENU_AUTHOR_ID,
    VIEW_OPEN_QUICK_SWITCHER_PALETTE_AUTHOR_ID, VIEW_OPEN_RICH_NOTE_MENU_AUTHOR_ID,
    VIEW_OPEN_RICH_NOTE_PALETTE_AUTHOR_ID, VIEW_OPEN_TAGS_MENU_AUTHOR_ID,
    VIEW_OPEN_TAGS_PALETTE_AUTHOR_ID, WP104_PRODUCT_HEADINGS, WP_SURFACE_HEADINGS,
};
use handshake_native::manual_pane::{
    manual_topic_author_id, ManualPane, ManualPaneState, ManualRegistry, ManualSection,
    ManualSurface,
};
use handshake_native::theme::HsPalette;

const REAL_MCP_TOOLS: &[&str] = &["list_widgets", "click_widget", "set_value", "screenshot"];
const GRAPH_NODE_AUTHOR_ID_PATTERN: &str = "graph.node.{block_id}";

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
                "properties-header",
                "properties-title",
                "properties-project-ref",
                "Editor chip tags are local-only",
                "Tags and Tag Hubs",
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
                "kernel_model_runtime_load",
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
    assert!(
        model.contains("operator launches omit wp_id, mt_id, prompt, and simulate_duration_ms"),
        "model topic must document the MT-101 remediation: operator launches carry no WP/MT attribution, canned prompt, or simulation knob"
    );
    assert!(
        model.contains("promptless session bootstrap"),
        "model topic must state that the launch does not smuggle an initial prompt"
    );
    assert!(
        model.contains("LocalModelLoadEndpointMissing kernel_model_runtime_load"),
        "model topic must name the exact local-model-load typed blocker when live local proof is unavailable"
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
fn mt020_agent_tool_reference_covers_save_draft_and_export_controls() {
    let rows = row_by_id();
    let required = [
        (
            CONFLICT_KEEP_YOURS_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            CONFLICT_KEEP_SERVER_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            CONFLICT_OPEN_MERGE_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            DRAFT_BANNER_AUTHOR_ID,
            ManualSurface::RichText,
            "list_widgets",
        ),
        (
            DRAFT_RESTORE_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            DRAFT_DISCARD_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            RICH_EDITOR_EXPORT_BUTTON_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            EXPORT_FORMAT_PICKER_AUTHOR_ID,
            ManualSurface::RichText,
            "list_widgets",
        ),
    ];

    for (author_id, surface, tool) in required {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("MT-020 agent-tool row '{author_id}' must exist"));
        assert_eq!(row.surface, surface, "MT-020 row '{author_id}' surface");
        assert_eq!(row.mcp_tool, tool, "MT-020 row '{author_id}' tool");
    }
}

#[test]
fn wave6_agent_tool_reference_adds_view_open_surface_rows() {
    let rows = row_by_id();
    let required = [
        (
            VIEW_OPEN_CODE_EDITOR_MENU_AUTHOR_ID,
            ManualSurface::Code,
            "click_widget",
        ),
        (
            VIEW_OPEN_RICH_NOTE_MENU_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            VIEW_OPEN_KNOWLEDGE_GRAPH_MENU_AUTHOR_ID,
            ManualSurface::Graph,
            "click_widget",
        ),
        (
            VIEW_OPEN_FOLDERS_MENU_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_TAGS_MENU_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_BLOCK_COLLECTIONS_MENU_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_CANVAS_MENU_AUTHOR_ID,
            ManualSurface::Canvas,
            "click_widget",
        ),
        (
            VIEW_OPEN_LOOM_SEARCH_MENU_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_FIND_IN_FILES_MENU_AUTHOR_ID,
            ManualSurface::Code,
            "click_widget",
        ),
        (
            VIEW_OPEN_QUICK_SWITCHER_MENU_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_DAILY_JOURNAL_MENU_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_DIFF_EDITOR_MENU_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            VIEW_OPEN_CODE_EDITOR_PALETTE_AUTHOR_ID,
            ManualSurface::Code,
            "click_widget",
        ),
        (
            VIEW_OPEN_RICH_NOTE_PALETTE_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            VIEW_OPEN_KNOWLEDGE_GRAPH_PALETTE_AUTHOR_ID,
            ManualSurface::Graph,
            "click_widget",
        ),
        (
            VIEW_OPEN_FOLDERS_PALETTE_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_TAGS_PALETTE_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_CANVAS_PALETTE_AUTHOR_ID,
            ManualSurface::Canvas,
            "click_widget",
        ),
        (
            VIEW_OPEN_LOOM_SEARCH_PALETTE_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_FIND_IN_FILES_PALETTE_AUTHOR_ID,
            ManualSurface::Code,
            "click_widget",
        ),
        (
            VIEW_OPEN_QUICK_SWITCHER_PALETTE_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_DAILY_JOURNAL_PALETTE_AUTHOR_ID,
            ManualSurface::Knowledge,
            "click_widget",
        ),
        (
            VIEW_OPEN_DIFF_EDITOR_PALETTE_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
        (
            CONFLICT_OPEN_MERGE_AUTHOR_ID,
            ManualSurface::RichText,
            "click_widget",
        ),
    ];

    for (author_id, surface, tool) in required {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("wave-6 VIEW open-surface row '{author_id}' must exist"));
        assert_eq!(row.surface, surface, "row '{author_id}' surface");
        assert_eq!(row.mcp_tool, tool, "row '{author_id}' tool");
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
fn mt069_manual_agent_rows_cover_file_edit_go_menu_leaves() {
    let row_ids: HashSet<&str> = agent_tool_rows().iter().map(|row| row.author_id).collect();
    for author_id in handshake_native::top_menu_bar::EDITOR_MENU_LEAF_AUTHOR_IDS {
        assert!(
            row_ids.contains(author_id),
            "HBR-MAN/MT-069: agent-tool rows must document FILE/EDIT/GO menu leaf '{author_id}'"
        );
    }

    let rows = row_by_id();
    for author_id in [
        "menu.file.save",
        "menu.file.export-json",
        "menu.edit.select-all",
        "menu.edit.find-replace",
        handshake_native::top_menu_bar::GO_SYMBOL_IN_FILE_AUTHOR_ID,
        handshake_native::command_registry::CMD_EDITOR_GO_TO_DEFINITION,
    ] {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("missing MT-069 menu row {author_id}"));
        assert_eq!(
            row.mcp_tool, "click_widget",
            "menu leaf {author_id} must be driven by the real click_widget tool"
        );
    }
}

#[test]
fn mt021_agent_tool_reference_covers_graph_toolbar_controls() {
    let rows = row_by_id();

    for author_id in [
        GRAPH_MODE_LOCAL_AUTHOR_ID,
        GRAPH_MODE_GLOBAL_AUTHOR_ID,
        GRAPH_ZOOM_IN_AUTHOR_ID,
        GRAPH_ZOOM_OUT_AUTHOR_ID,
        GRAPH_RELAYOUT_AUTHOR_ID,
    ] {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("MT-021 graph toolbar row '{author_id}' must exist"));
        assert_eq!(
            row.surface,
            ManualSurface::Graph,
            "MT-021 graph toolbar row '{author_id}' must be grouped under the Graph surface"
        );
        assert_eq!(
            row.mcp_tool, "click_widget",
            "MT-021 graph toolbar row '{author_id}' must be driven by click_widget"
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

// ── WP-KERNEL-012 wave-5: full-WP per-surface manual topics ───────────────────────────────────────────

/// A distinctive body-start marker per wave-5 surface topic (renders as a manual body label).
fn wave5_body_marker(heading: &str) -> &'static str {
    match heading {
        "Code Editor" => "VS Code-parity native code pane",
        "Rich Text Editor" => "Obsidian/Notion-parity native Notes pane",
        "Knowledge Graph" => "Loom graph view",
        "Folder Tree" => "The Folder Tree is the native Obsidian-style folder surface",
        "Tags and Tag Hubs" => {
            "Tags and Tag Hubs are the native Obsidian-style tag navigation surface"
        }
        "Canvas" => "free-form spatial board",
        "Search" => "three complementary search surfaces",
        "Wikilinks and Backlinks" => "Wikilinks tie notes together",
        "Daily Journal" => "date-addressed note surface",
        "Diff and Merge" => "VS Code-style side-by-side and inline diffs",
        "Internationalization" => "SINGLE shared Unicode text-mechanics",
        "Menu Bar and Commands" => "six top-level dropdowns",
        "Editor Settings" => "Editor preferences live in the Settings dialog",
        other => panic!("unknown wave-5 surface topic '{other}'"),
    }
}

/// Concrete no-context facts (real author_ids / routes) each wave-5 topic must contain.
fn wave5_needles(heading: &str) -> &'static [&'static str] {
    match heading {
        "Code Editor" => &[
            "editor.code.save",
            "F12",
            "status-bar-language-mode",
            "Large files are virtualized automatically",
            "code_editor_fold_0",
            "menu.edit.fold-all",
            "Ctrl+K Ctrl+0",
            "code_editor_completion_popup",
            "code_editor_hover",
            "publishDiagnostics is URI-scoped",
            "Plain typing",
            "undo-count-{pane_id}",
            "NEEDS_MANAGED_RESOURCE_PROOF",
            "PostgreSQL/EventLedger",
        ],
        "Rich Text Editor" => &[
            "editor.rich.format-bold",
            "rich-reading-mode-toggle",
            "PUT /knowledge/documents/:id/save",
            "GET/PUT/DELETE /knowledge/documents/:id/draft",
            "draft-recovery-banner",
            "draft-restore",
            "draft-discard",
            "without canonical-saving",
            "Step::InsertInlineChild",
            "exact pre-insert content",
            "Flight Recorder/EventLedger",
            "internal_diagnostics",
            "Palmistry",
            "rich-editor-export-button",
            "export-format-picker",
        ],
        "Knowledge Graph" => &[
            "graph.open-node",
            "view.graph",
            "backlink_depth",
            "graph.mode.local",
            "graph.mode.global",
            "graph.zoom.in",
            "graph.zoom.out",
            "graph.relayout",
            "graph.node.{block_id}",
            "GET /workspaces/{id}/loom/views/all",
            "GET /workspaces/{id}/loom/graph-search",
            "LoomGraphView::set_graph",
            "NEEDS_MANAGED_RESOURCE_PROOF",
            "0 nodes",
            "Graph error:",
            "list_widgets",
        ],
        "Folder Tree" => &[
            "folder-tree.node.{folder_id}",
            "folder-tree.color.{folder_id}",
            "folder-tree.retry",
            "menu.view.open-folders",
            "command-palette.option.hs-view-palette-folders",
            "view.folders",
            "GET /workspaces/{id}/loom/folders",
            "GET /workspaces/{id}/loom/folders/{folder_id}/blocks",
            "PATCH /workspaces/{id}/loom/folders/{folder_id}",
            "Change color",
            "No folders",
            "NEEDS_MANAGED_RESOURCE_PROOF",
        ],
        "Tags and Tag Hubs" => &[
            "menu.view.open-tags",
            "command-palette.option.hs-view-palette-tags",
            "view.tags",
            "GET /workspaces/{id}/loom/tags",
            "GET /workspaces/{id}/loom/tags/{tag_block_id}",
            "GET /workspaces/{id}/loom/search",
            "/workspaces/{id}/loom/edges",
            "tags.search",
            "tags.row.{block_id}",
            "tag-hub.title.{block_id}",
            "tag-hub.member.{block_id}",
            "tag-hub.add-tag.{block_id}",
            "TagsPanelEvent::OpenTag",
            "No tags",
            "NEEDS_MANAGED_RESOURCE_PROOF",
        ],
        "Canvas" => &[
            "canvas.add-card",
            "getCanvasBoard",
            "cross-pane MT-035 compensating undo",
            "DELETE /workspaces/{id}/loom/canvas-placements/{placement_id}",
            "Inline text-card edit remains a typed blocker",
        ],
        "Search" => &[
            "loom-search-v2.query",
            "menu.edit.find-all",
            "quick-switcher.dialog",
            "command-palette.search",
        ],
        "Wikilinks and Backlinks" => &[
            "outgoing.panel",
            "ShellNavigator",
            "outgoing.section.resolved",
            "backlinks-panel",
            "backlinks-refresh",
            "backlink-{source_document_id}",
            "code-symbol-search",
            "code-symbol-search-input",
            "code-ref-chip-{symbol_entity_id}",
            "[[code:path/to/file.rs#MyStruct]]",
            "path#Symbol",
            "open-code-symbol",
            "CMD_OPEN_CODE_SYMBOL",
            "dispatch_code_ref_open",
            "take_pending_code_symbol",
            "ShellNavigator::open_code_symbol",
            "GET /knowledge/code/symbols/{symbol_entity_id}",
            "lookup_symbols_by_name_path",
            "GET /knowledge/code/symbols?workspace_id=&name=&path=&limit=1",
            "visible line range contains line_start",
            "unresolved chip",
            "note-refs-panel",
            "block_id",
            "note-ref-{document_id}",
            "document_id",
            "interop.open-document",
            "CMD_OPEN_DOCUMENT",
            "EditorEvent::BacklinkActivated",
            "dispatch_backlink_open",
            "pending_navigation",
            "HandshakeApp::drive_ckc_interop",
            "ShellNavigator::open_document",
            "loom://{workspace_id}/{block_id}",
            "placed_block_id",
            "ContentHash::from_backend",
            "Flight Recorder/EventLedger = NOT_APPLICABLE-with-reason",
            "internal_diagnostics = DEFERRED-with-reason",
            "Palmistry = DEFERRED-with-reason",
            "NEEDS_MANAGED_RESOURCE_PROOF",
            "typed backend-shape gap",
        ],
        "Daily Journal" => &[
            "daily-journal-panel",
            "journal-panel-root",
            "journal-start-writing",
            "journal-document-link-gap",
            "view.journal",
            "PUT /loom/journals/:date",
            "PUT /knowledge/documents/:id/save",
            "NEEDS_MANAGED_RESOURCE_PROOF",
            "EndpointUnavailable",
        ],
        "Diff and Merge" => &[
            "view.diff-merge",
            "menu.view.open-diff-editor",
            "conflict-open-merge",
            "base/local/remote",
            "Accept Local",
            "background worker",
            "screenshot/pixel evidence",
            "SaveManager",
            "conflict",
        ],
        "Internationalization" => &["text_intl", "UAX#29", "grapheme"],
        "Menu Bar and Commands" => &[
            "menu-file",
            "menu.edit.undo",
            "Open Editor Surfaces",
            "menu.view.open-code-editor",
            "menu.view.open-folders",
            "menu.view.open-tags",
            "menu.view.open-block-collections",
            "command-palette.option.hs-view-palette-code-editor",
            "command-palette.option.hs-view-palette-rich-note",
            "command-palette.option.hs-view-palette-graph",
            "command-palette.option.hs-view-palette-folders",
            "command-palette.option.hs-view-palette-tags",
            "command-palette.option.hs-view-palette-canvas",
            "command-palette.option.hs-view-palette-loom-search",
            "command-palette.option.hs-view-palette-find-in-files",
            "command-palette.option.hs-editor-menu-quick-open",
            "command-palette.option.hs-view-palette-journal",
            "command-palette.option.hs-view-palette-diff-merge",
            "view.graph",
            "workbench.action.quickOpen",
            "no lying-enabled",
        ],
        "Editor Settings" => &[
            "settings-editor-font-size",
            "settings-syntax-palette-mode",
            "PUT /workspaces/:id/settings",
            "mounted code editor and rich editor",
            "repaints the mounted code editor",
        ],
        other => panic!("unknown wave-5 surface topic '{other}'"),
    }
}

#[test]
fn mt021_knowledge_graph_manual_documents_live_ids_and_recovery() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Knowledge Graph");

    for needle in [
        GRAPH_MODE_LOCAL_AUTHOR_ID,
        GRAPH_MODE_GLOBAL_AUTHOR_ID,
        GRAPH_ZOOM_IN_AUTHOR_ID,
        GRAPH_ZOOM_OUT_AUTHOR_ID,
        GRAPH_RELAYOUT_AUTHOR_ID,
        GRAPH_NODE_AUTHOR_ID_PATTERN,
        "GET /workspaces/{id}/loom/views/all",
        "GET /workspaces/{id}/loom/graph-search",
        "ModeChanged",
        "AddEdge",
        "RemoveEdge",
        "LoomGraphView::set_graph",
        "NEEDS_MANAGED_RESOURCE_PROOF",
        "Handshake-managed PostgreSQL/EventLedger",
        "0 nodes",
        "Graph error:",
        "list_widgets",
        "switch graph.mode.local / graph.mode.global",
        "click graph.relayout",
    ] {
        assert!(
            body.contains(needle),
            "MT-021 Knowledge Graph manual must document '{needle}'"
        );
    }
}

#[test]
fn mt022_folder_tree_manual_documents_live_ids_routes_and_recovery() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Folder Tree");

    for needle in [
        FOLDER_TREE_NODE_AUTHOR_ID_PATTERN,
        FOLDER_TREE_COLOR_AUTHOR_ID_PATTERN,
        FOLDER_TREE_RETRY_AUTHOR_ID,
        VIEW_OPEN_FOLDERS_MENU_AUTHOR_ID,
        VIEW_OPEN_FOLDERS_PALETTE_AUTHOR_ID,
        "view.folders",
        "GET /workspaces/{id}/loom/folders",
        "GET /workspaces/{id}/loom/folders/{folder_id}/blocks",
        "PATCH /workspaces/{id}/loom/folders/{folder_id}",
        "FolderTreeEvent::ExpandFolder",
        "FolderTreeEvent::ChangeColor",
        "FolderTreeEvent::OpenBlock",
        "FolderTreeEvent::Retry",
        "Change color",
        "No folders",
        "Retry",
        "NEEDS_MANAGED_RESOURCE_PROOF",
        "Handshake-managed PostgreSQL/EventLedger",
        "list_widgets",
        "click_widget",
    ] {
        assert!(
            body.contains(needle),
            "MT-022 Folder Tree manual must document '{needle}'"
        );
    }
}

#[test]
fn mt022_agent_tool_reference_covers_folder_tree_controls() {
    let rows = row_by_id();
    for (author_id, tool, label) in [
        (
            VIEW_OPEN_FOLDERS_MENU_AUTHOR_ID,
            "click_widget",
            "Open Folders from VIEW",
        ),
        (
            VIEW_OPEN_FOLDERS_PALETTE_AUTHOR_ID,
            "click_widget",
            "Open Folders from the command palette",
        ),
        (
            FOLDER_TREE_RETRY_AUTHOR_ID,
            "click_widget",
            "Retry folder-tree load",
        ),
    ] {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("missing MT-022 agent-tool row '{author_id}'"));
        assert_eq!(row.surface, ManualSurface::Knowledge);
        assert_eq!(row.mcp_tool, tool);
        assert_eq!(row.action_label, label);
    }
}

#[test]
fn mt023_tags_manual_documents_live_ids_routes_and_recovery() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Tags and Tag Hubs");

    for needle in [
        VIEW_OPEN_TAGS_MENU_AUTHOR_ID,
        VIEW_OPEN_TAGS_PALETTE_AUTHOR_ID,
        TAGS_SEARCH_AUTHOR_ID,
        TAG_ROW_AUTHOR_ID_PATTERN,
        TAG_HUB_TITLE_AUTHOR_ID_PATTERN,
        TAG_HUB_MEMBER_AUTHOR_ID_PATTERN,
        TAG_HUB_ADD_TAG_AUTHOR_ID_PATTERN,
        "view.tags",
        "GET /workspaces/{id}/loom/tags",
        "GET /workspaces/{id}/loom/tags/{tag_block_id}",
        "GET /workspaces/{id}/loom/search",
        "POSTs /workspaces/{id}/loom/edges",
        "TagsPanelEvent::OpenTag",
        "Switching projects clears",
        "No tags",
        "NEEDS_MANAGED_RESOURCE_PROOF",
        "Handshake-managed PostgreSQL/EventLedger",
        "list_widgets",
        "set_value",
        "click_widget",
    ] {
        assert!(
            body.contains(needle),
            "MT-023 Tags manual must document '{needle}'"
        );
    }
}

#[test]
fn mt023_agent_tool_reference_covers_tags_controls() {
    let rows = row_by_id();
    for (author_id, tool, label) in [
        (
            VIEW_OPEN_TAGS_MENU_AUTHOR_ID,
            "click_widget",
            "Open Tags from VIEW",
        ),
        (
            VIEW_OPEN_TAGS_PALETTE_AUTHOR_ID,
            "click_widget",
            "Open Tags from the command palette",
        ),
        (TAGS_SEARCH_AUTHOR_ID, "set_value", "Filter tags"),
    ] {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("missing MT-023 agent-tool row '{author_id}'"));
        assert_eq!(row.surface, ManualSurface::Knowledge);
        assert_eq!(row.mcp_tool, tool);
        assert_eq!(row.action_label, label);
    }
}

#[test]
fn mt032_agent_tool_reference_covers_fixed_backlinks_controls() {
    let rows = row_by_id();
    for (author_id, tool, label) in [
        (
            handshake_native::rich_editor::wikilinks::backlinks_panel::PANEL_AUTHOR_ID,
            "list_widgets",
            "Backlinks edge: the backlinks panel",
        ),
        (
            handshake_native::rich_editor::wikilinks::backlinks_panel::REFRESH_AUTHOR_ID,
            "click_widget",
            "Backlinks edge: refresh backlinks",
        ),
    ] {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("missing MT-032 agent-tool row '{author_id}'"));
        assert_eq!(row.surface, ManualSurface::Interop);
        assert_eq!(row.mcp_tool, tool);
        assert_eq!(row.action_label, label);
    }
}

#[test]
fn wave5_surface_topics_exist_and_carry_real_no_context_facts() {
    let section = editors_manual_section();
    assert_eq!(
        WP_SURFACE_HEADINGS.len(),
        13,
        "one dedicated topic per native editor surface"
    );
    for heading in WP_SURFACE_HEADINGS {
        let body = topic_body(&section, heading);
        assert!(
            body.len() > 220,
            "wave-5 topic '{heading}' must be substantive no-context guidance (got {} chars)",
            body.len()
        );
        for needle in wave5_needles(heading) {
            assert!(
                body.contains(needle),
                "wave-5 topic '{heading}' must include concrete runtime fact '{needle}'"
            );
        }
    }
}

#[test]
fn wave6_editor_settings_topic_documents_live_font_and_palette_effects() {
    // Wave-6 S6 item 3 resolved the old inert host-wiring gap: font size and Custom palette now apply to
    // the running code editor. The manual must preserve the narrower cosmetic scope edge without reviving
    // the stale "does not yet apply" claim.
    let section = editors_manual_section();
    let body = topic_body(&section, "Editor Settings");
    assert!(
        body.contains("font-size change resizes the running code rows/glyph advance and rich document text layout"),
        "Editor Settings topic must document the live font-size effect"
    );
    assert!(
        body.contains(
            "Custom syntax palette also repaints the mounted code editor and minimap syntax rows"
        ),
        "Editor Settings topic must document the live Custom syntax-palette effect"
    );
    assert!(
        body.contains("gutter line numbers still use their base sizing"),
        "Editor Settings topic must keep the cosmetic scope edge explicit"
    );
    assert!(
        !body.contains("does not yet apply a live font size"),
        "Editor Settings topic must not keep the old stale inert-font-size claim"
    );
    assert!(
        !body.to_lowercase().contains("sqlite"),
        "no SQLite in the settings topic"
    );
}

#[test]
fn wave5_surface_topics_are_selectable_in_manual_pane() {
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

    for heading in WP_SURFACE_HEADINGS {
        {
            let mut state = state.borrow_mut();
            state.query = (*heading).to_owned();
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
            Some(("native-editors".to_owned(), (*heading).to_owned())),
            "clicking wave-5 topic '{heading}' should update ManualPaneState"
        );
        let marker = wave5_body_marker(heading);
        assert!(
            harness.query_by_label_contains(marker).is_some(),
            "selecting topic '{heading}' should render body marker '{marker}'"
        );
    }
}
