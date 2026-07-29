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
    MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID, NOTES_LOAD_ERROR_AUTHOR_ID, NOTES_LOAD_RETRY_AUTHOR_ID,
    TERMINAL_LAUNCH_STATUS_AUTHOR_ID,
};
use handshake_native::graph::block_collection_view::RETRY_AUTHOR_ID as BCV_RETRY_AUTHOR_ID;
use handshake_native::graph::wiki_page_panel::{
    CANCEL_AUTHOR_ID_PREFIX as WIKI_CANCEL_AUTHOR_ID_PREFIX,
    CONTENT_AUTHOR_ID_PREFIX as WIKI_CONTENT_AUTHOR_ID_PREFIX,
    EDIT_AREA_AUTHOR_ID_PREFIX as WIKI_EDIT_AREA_AUTHOR_ID_PREFIX,
    EDIT_AUTHOR_ID_PREFIX as WIKI_EDIT_AUTHOR_ID_PREFIX,
    ERROR_AUTHOR_ID_PREFIX as WIKI_ERROR_AUTHOR_ID_PREFIX,
    METADATA_AUTHOR_ID_PREFIX as WIKI_METADATA_AUTHOR_ID_PREFIX,
    REBUILD_AUTHOR_ID_PREFIX as WIKI_REBUILD_AUTHOR_ID_PREFIX,
    RETRY_AUTHOR_ID_PREFIX as WIKI_RETRY_AUTHOR_ID_PREFIX,
    SAVE_AUTHOR_ID_PREFIX as WIKI_SAVE_AUTHOR_ID_PREFIX,
    STALE_AUTHOR_ID_PREFIX as WIKI_STALE_AUTHOR_ID_PREFIX,
    TITLE_AUTHOR_ID_PREFIX as WIKI_TITLE_AUTHOR_ID_PREFIX,
};
use handshake_native::graph::{
    MODE_GLOBAL_AUTHOR_ID as GRAPH_MODE_GLOBAL_AUTHOR_ID,
    MODE_LOCAL_AUTHOR_ID as GRAPH_MODE_LOCAL_AUTHOR_ID,
    RELAYOUT_AUTHOR_ID as GRAPH_RELAYOUT_AUTHOR_ID, ZOOM_IN_AUTHOR_ID as GRAPH_ZOOM_IN_AUTHOR_ID,
    ZOOM_OUT_AUTHOR_ID as GRAPH_ZOOM_OUT_AUTHOR_ID,
};
use handshake_native::manual_content_editors::{
    agent_tool_rows, editors_manual_section, ArgusAutomationStatus, CONFLICT_KEEP_SERVER_AUTHOR_ID,
    CONFLICT_KEEP_YOURS_AUTHOR_ID, CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID,
    CONFLICT_OPEN_MERGE_AUTHOR_ID, DIAGNOSTIC_TOOL_HEADINGS, DRAFT_BANNER_AUTHOR_ID,
    DRAFT_DISCARD_AUTHOR_ID, DRAFT_RESTORE_AUTHOR_ID, E8_PERFORMANCE_INTERCONNECTION_HEADING,
    EXPORT_FORMAT_PICKER_AUTHOR_ID, FLIGHT_RECORDER_MENU_AUTHOR_ID,
    FLIGHT_RECORDER_PALETTE_AUTHOR_ID, FOLDER_TREE_COLOR_AUTHOR_ID_PATTERN,
    FOLDER_TREE_NODE_AUTHOR_ID_PATTERN, FOLDER_TREE_RETRY_AUTHOR_ID, INFERENCE_LAB_MENU_AUTHOR_ID,
    INFERENCE_LAB_PALETTE_AUTHOR_ID, MODEL_SESSION_LAUNCH_MENU_AUTHOR_ID,
    MODEL_SESSION_LAUNCH_PALETTE_AUTHOR_ID, MT108_ARGUS_EVIDENCE_MATRIX,
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

const REAL_MCP_TOOLS: &[&str] = &[
    handshake_native::mcp::ARGUS_INSPECT_METHOD,
    handshake_native::mcp::ARGUS_CLICK_METHOD,
    handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
    handshake_native::mcp::ARGUS_SCREENSHOT_METHOD,
];
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

fn canonical_tool_name(name: &str) -> &str {
    match name {
        "list_widgets" => handshake_native::mcp::ARGUS_INSPECT_METHOD,
        "click_widget" => handshake_native::mcp::ARGUS_CLICK_METHOD,
        "set_value" => handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        "screenshot" => handshake_native::mcp::ARGUS_SCREENSHOT_METHOD,
        canonical => canonical,
    }
}

fn manual_body_contains(body: &str, expected: &str) -> bool {
    fn replace_token(input: &str, legacy: &str, canonical: &str) -> String {
        let mut output = String::with_capacity(input.len());
        let mut copied_through = 0;
        for (start, _) in input.match_indices(legacy) {
            let end = start + legacy.len();
            let is_identifier = |ch: char| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.');
            let before = input[..start]
                .chars()
                .next_back()
                .is_some_and(is_identifier);
            let after = input[end..].chars().next().is_some_and(is_identifier);
            if before || after {
                continue;
            }
            output.push_str(&input[copied_through..start]);
            output.push_str(canonical);
            copied_through = end;
        }
        output.push_str(&input[copied_through..]);
        output
    }

    let canonical = replace_token(
        expected,
        "list_widgets",
        handshake_native::mcp::ARGUS_INSPECT_METHOD,
    );
    let canonical = replace_token(
        &canonical,
        "click_widget",
        handshake_native::mcp::ARGUS_CLICK_METHOD,
    );
    let canonical = replace_token(
        &canonical,
        "set_value",
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
    );
    let canonical = replace_token(
        &canonical,
        "screenshot",
        handshake_native::mcp::ARGUS_SCREENSHOT_METHOD,
    );
    body.contains(&canonical)
}

#[test]
fn mt045_mt046_manual_covers_large_documents_interconnection_settings_and_menu() {
    let section = editors_manual_section();
    let body = topic_body(&section, E8_PERFORMANCE_INTERCONNECTION_HEADING);

    for needle in [
        "10,000-line buffer",
        "1,000 blocks",
        "KNOWLEDGE_RICH_DOCUMENT_SAVED",
        "persisted cyclic-5",
        "1,000-node/~2,000-edge LoomGraphView",
        "unavailable RSS sample",
        "settings-editor-word-wrap",
        "settings-editor-minimap",
        "menu.view.open-code-editor",
        "menu.view.open-rich-note",
        "menu.edit.quick-switcher",
        "DragPayload",
        "LoomCanvasBoard",
        "FindReplaceState",
        "LoomSearchV2",
        "QuickSwitcher",
        "RichEditorState/CodeEditorPanel",
        "test_perf_large_rich",
        "test_interconnect_*",
        "Handshake_Artifacts/wp-kernel-012/mt-046/measurements",
        "worst-of-three process",
        "contract-authoritative runtime-updated projection",
        "run_mt045_perf_proof.ps1",
        "Cargo release mode",
        "1,200-second ceiling",
        "existing internal PostgreSQL process is never stopped",
    ] {
        assert!(
            manual_body_contains(body, needle),
            "MT-045/046 manual must document {needle:?}"
        );
    }
}

#[test]
fn mt025_wiki_projection_manual_uses_exact_runtime_accesskit_patterns() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Wiki Projection");

    for prefix in [
        WIKI_TITLE_AUTHOR_ID_PREFIX,
        WIKI_CONTENT_AUTHOR_ID_PREFIX,
        WIKI_METADATA_AUTHOR_ID_PREFIX,
        WIKI_EDIT_AUTHOR_ID_PREFIX,
        WIKI_EDIT_AREA_AUTHOR_ID_PREFIX,
        WIKI_SAVE_AUTHOR_ID_PREFIX,
        WIKI_CANCEL_AUTHOR_ID_PREFIX,
        WIKI_REBUILD_AUTHOR_ID_PREFIX,
        WIKI_STALE_AUTHOR_ID_PREFIX,
        WIKI_ERROR_AUTHOR_ID_PREFIX,
        WIKI_RETRY_AUTHOR_ID_PREFIX,
    ] {
        let expected = format!("{prefix}{{sanitized_projection_id}}");
        assert!(
            body.contains(&expected),
            "Wiki Projection manual must document the exact runtime AccessKit pattern {expected:?}"
        );
    }

    assert!(
        !body.contains("wiki-page."),
        "the obsolete nonexistent wiki-page.* selector namespace must not return"
    );
}

#[test]
fn mt033_ckc_stage_manual_is_no_context_and_matches_runtime_ids() {
    let section = editors_manual_section();
    let interop = topic_body(&section, "Interop Edges");
    for required in [
        "VIEW > Toggle Atelier / CKC Panel",
        "menu.view.toggle-atelier",
        "atelier-side-panel",
        "atelier-batch-*",
        "atelier-item-*",
        "AccessKit 0.21.1 has no StartDrag action",
        "Click on the atelier-item-* ListItem",
        "atelier-corpus-*",
        "GET /atelier/intake/batches",
        "atelier-character-list-blocker",
        "atelier-moodboard-list-blocker",
        "atelier-items-retry-*",
        "editor.rich.text",
        "empty paragraph with no text leaf",
        "EDITORS > View: Stage",
        "menu.editors.stage",
        "EDITORS > Capture and embed from Stage",
        "menu.editors.embed-stage-capture",
        "Route selection to Stage",
        "menu.editors.route-to-stage",
        "one docked stage-pane Role::GenericContainer",
        "rich-editor.route-to-stage",
        "Route to Stage",
        "ctx-menu.ctxmenu-node-route-to-stage",
        "only when the clicked node has a stable id and its mounted board has",
        "matching workspace + canvas projection confirmed by a completed board load",
        "pending, failed, rebound, or stale projections",
        "Graph-view nodes do not carry a live Canvas board route",
        "Use argus.inspect to read the current disabled state before argus.click",
        "stage-route-status",
        "stage-capture-embed-back",
        "stage-embed-back-status",
        "privileged create -> exact-byte descriptor/content retrieval",
        "Job History",
        "EventLedger",
        "Retry exact EventLedger receipt",
        "LedgerPending",
        "HsLink is already saved",
        "instead of starting a new capture or minting a new receipt",
        "same immutable event_id",
        "does not insert another hsLink",
        "No Stage-specific setting exists",
        "rich-editor-interop-status",
        "cargo test -p handshake-native --test test_ckc_embed -- --nocapture",
        "cargo test -p handshake-native --features integration --test test_ckc_embed -- --nocapture",
        "self-seed PostgreSQL",
        "workspace_id plus canvas_id",
        "late success/failure for board A cannot reload or paint board B",
    ] {
        assert!(
            interop.contains(required),
            "MT-033 no-context manual must contain runtime fact '{required}'"
        );
    }
    let canvas = topic_body(&section, "Canvas");
    for required in [
        "ResolveAtelierAndPlace",
        "PUT /atelier/intake/items/{item_id}/loom-projection",
        "batch-items response carries loom_block_id",
        "accepts only that backend-provided identity",
        "never fabricates a Loom block",
        "posts only placed_block_id",
        "freshly reloads the board",
    ] {
        assert!(
            canvas.contains(required),
            "Canvas manual must remain consistent with MT-033 runtime: '{required}'"
        );
    }
}

#[test]
fn mt088_manual_documents_backend_down_operation_and_recovery() {
    let section = editors_manual_section();
    let diagnostics = topic_body(&section, "internal_diagnostics");
    for required in [
        "Disconnected/degraded state",
        "BackendUnreachable",
        "BackendRecovered",
        "1.5 seconds",
        "10 seconds",
        "fixed safety bounds, not operator preferences",
        "keep editing local buffers",
        "verify BackendRecovered before retrying a mutation",
        "backend_down_responsive_real_pg_palmistry_argus",
        "configured single",
        "never set CARGO_TARGET_DIR or pass --target-dir to another",
        "absolute_path",
        "MSVC MAX_PATH",
        "normalized absolute form of that exact canonical root",
        "HANDSHAKE_TEST_PG_DSN",
        "HSK_TEST_BACKEND_BIN",
        "HANDSHAKE_PALMISTRY_EXE",
        "canonical localhost Argus",
        "Operator -> Open Settings",
        "starts a fresh production layout load",
        "diagnostics_panel",
        "diagnostics_events",
        "diagnostics_palmistry",
        "endpoint-attributed BackendUnreachable",
        "HEAD-matched source blobs",
        "binary SHA-256/freshness records",
        "ownership/orphan-reclamation metadata",
        "CleanShutdown survivor receipt",
        "NOT_APPLICABLE-with-reason for local reachability edges",
        "Tier 2 internal_diagnostics is WIRED",
        "Tier 3 Palmistry is WIRED",
        "wp-kernel-012-mt-088/integrated/run-*",
    ] {
        assert!(
            diagnostics.contains(required),
            "MT-088 no-context manual must contain runtime/recovery fact '{required}'"
        );
    }
}

fn row_by_id() -> HashMap<&'static str, handshake_native::manual_pane::AgentToolRow> {
    agent_tool_rows()
        .into_iter()
        .map(|row| (row.author_id, row))
        .collect()
}

#[test]
fn mt066_stage_manual_covers_canonical_argus_recovery_and_three_tier_posture() {
    let rows = row_by_id();
    for (author_id, method) in [
        ("menu.editors.stage", "argus.click"),
        ("menu.editors.route-to-stage", "argus.click"),
        ("stage-route-status", "argus.inspect"),
        ("stage-route-retry", "argus.click"),
        ("stage-routed-content", "argus.inspect"),
        ("stage-capture-embed-back", "argus.click"),
        ("stage-embed-back-status", "argus.inspect"),
    ] {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("MT-066 manual row missing for {author_id}"));
        assert_eq!(
            row.mcp_tool, method,
            "MT-066 {author_id} must use the canonical Argus method"
        );
    }
    let section = editors_manual_section();
    let interop = topic_body(&section, "Interop Edges");
    for required in [
        "HBR-INT-009 diagnostic posture",
        "Flight Recorder/EventLedger = WIRED",
        "route_to_stage",
        "stage_embed_back",
        "internal_diagnostics = DEFERRED-with-reason",
        "no Stage-specific diagnostic emission",
        "Palmistry = DEFERRED-with-reason",
        "no Stage-scoped tracker or recovery proof",
        "fresh argus.inspect",
        "same immutable event_id",
    ] {
        assert!(
            interop.contains(required),
            "MT-066 Stage manual must preserve '{required}'"
        );
    }
}

#[test]
fn mt068_locus_manual_covers_persisted_argus_recovery_and_three_tier_posture() {
    let section = editors_manual_section();
    let interop = topic_body(&section, "Interop Edges");
    for required in [
        "locus-ref-chip-wp-{WP_ID}",
        "locus-ref-chip-mt-{MT_ID}",
        "fresh argus.inspect",
        "argus.click",
        "immediate action receipt may be indeterminate",
        "same receipt/agent attribution",
        "WP:{WP_ID}",
        "MT::{MT_ID}",
        "backend restart and rich-document reload",
        "reverse lookup is read-only",
        "record NotFound from LocusReadApiUnavailable",
        "HBR-INT-009 diagnostic posture for Locus",
        "Flight Recorder/EventLedger = WIRED",
        "locus_ref_resolved",
        "locus_reverse_lookup",
        "workspace and locus_uri",
        "failed resolution emits no fabricated success event",
        "knowledge-document save remains a separate operation",
        "internal_diagnostics = DEFERRED-with-reason",
        "no Locus-specific diagnostic row",
        "Palmistry = DEFERRED-with-reason",
        "no Locus-scoped tracker or recovery proof",
    ] {
        assert!(
            interop.contains(required),
            "MT-068 Locus manual must preserve '{required}'"
        );
    }
    assert!(
        !interop.contains(
            "Flight Recorder/EventLedger = DEFERRED-with-reason because this MT's Locus edge"
        ),
        "MT-068 Locus manual must not regress the wired Tier-1 event family to a deferral"
    );
}

#[test]
fn mt074_other_pillar_manual_documents_canonical_argus_matrix_and_calendar_diagnostics() {
    let rows = row_by_id();
    for (author_id, method) in [
        ("daily-journal-panel", "argus.inspect"),
        ("journal-panel-root", "argus.inspect"),
        ("journal-start-writing", "argus.click"),
        ("journal-document-link-gap", "argus.inspect"),
        ("daily-journal-date-header", "argus.click"),
        ("daily-journal-calendar-event-chip", "argus.click"),
        ("daily-journal-activity-strip", "argus.inspect"),
        ("outgoing.panel", "argus.inspect"),
        ("outgoing.section.resolved", "argus.inspect"),
        ("outgoing.section.unresolved", "argus.inspect"),
        ("backlinks-panel", "argus.inspect"),
        ("backlinks-refresh", "argus.click"),
    ] {
        let row = rows
            .get(author_id)
            .unwrap_or_else(|| panic!("MT-074 manual row missing for {author_id}"));
        assert_eq!(
            row.mcp_tool, method,
            "MT-074 {author_id} must use the canonical Argus method"
        );
    }

    let section = editors_manual_section();
    let interop = topic_body(&section, "Interop Edges");
    for required in [
        "MT-074 aggregate proof matrix",
        "test_other_pillar_interop_proofs other_pillar_op",
        "--test-threads=1",
        "canonical Argus sequence",
        "argus.inspect -> argus.click -> attributed action receipt -> fresh argus.inspect",
        "menu-editors",
        "menu.editors.route-to-stage",
        "editor.rich.save",
        "daily-journal-calendar-event-chip",
        "calendar-event-tab-activity",
        "locus-ref-chip-wp-{id}",
        "exact same causal_action_id",
        "route_to_stage",
        "stage_embed_back",
        "calendar_event_bound",
        "activity_span_correlated",
        "locus_ref_resolved",
        "locus_reverse_lookup",
        "zero residual Argus leases",
        "wp-kernel-012-mt-074/canonical-argus/<scenario>/run-*",
        "<scenario>-canonical-argus.json",
        "ActionChannel raw_input_hook drain plus bounded Harness::run_steps",
        "fixture-only setup and cleanup boundaries",
        "HBR-INT-009 diagnostic posture for Calendar",
        "no Calendar-specific diagnostic row",
        "no Calendar-scoped tracker or recovery proof",
    ] {
        assert!(
            interop.contains(required),
            "MT-074 manual must preserve proof/recovery fact '{required}'"
        );
    }
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
                "OPERATOR -> Open Flight Recorder",
                "RUN -> Open Flight Recorder",
                "GET /api/flight_recorder",
                "wsid=<active workspace>",
                "with only",
                "menu.operator.flight-recorder",
                "menu.run.flight-recorder",
                "command-palette.option.hs-flight-palette-open",
                "flight-recorder.refresh",
                "flight-recorder.load-failure",
                "flight-recorder.quarantine-status",
                "flight-recorder.error-ring",
                "FR-EVT-MEM-001 memory_write_proposed",
                "FR-EVT-MEM-002 memory_write_reviewed",
                "FR-EVT-MEM-003 memory_write_committed",
                "FR-EVT-MEM-004 memory_pack_built",
                "FR-EVT-MEM-005",
                "memory_item_status_changed",
                "settings-editor-flight-recorder-posture",
                "The native POST envelope is closed",
                "actor_kind=optional human|agent|system",
                "payload=an object no larger than 64 KiB",
                "document_saved={document_id:string",
                "route_to_stage={content_kind:string,causal_action_id?:non-empty string}",
                "manifest_ref:string,causal_action_id?:non-empty string",
                "edited_document_ids:non-empty string[]",
                "target_kind:work_packet|microtask",
                "Accepted storage rows use",
                "12 accepted actions",
                "locus_reverse_lookup",
                "malformed native-editor or FEMS candidates",
                "latest 20",
                "never reuses the last model-launch request",
                "pending-mirror receipt and reconciler",
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
                manual_body_contains(body, needle),
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
        assert_eq!(
            row.mcp_tool,
            canonical_tool_name(tool),
            "row '{author_id}' tool"
        );
    }

    assert_eq!(
        rows.get(TERMINAL_MENU_AUTHOR_ID).unwrap().mcp_tool,
        handshake_native::mcp::argus::ARGUS_CLICK_METHOD,
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
        assert_eq!(
            row.mcp_tool,
            canonical_tool_name(tool),
            "MT-020 row '{author_id}' tool"
        );
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
        assert_eq!(
            row.mcp_tool,
            canonical_tool_name(tool),
            "row '{author_id}' tool"
        );
    }
}

#[test]
fn mt104_terminal_menu_author_id_is_live_clickable_run_leaf() {
    let rows = row_by_id();
    let terminal = rows
        .get(TERMINAL_MENU_AUTHOR_ID)
        .expect("terminal agent-tool row exists");
    assert_eq!(terminal.mcp_tool, handshake_native::mcp::ARGUS_CLICK_METHOD);
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
fn operator_and_fems_agent_rows_cover_live_visibility_controls() {
    let row_ids: HashSet<&str> = agent_tool_rows().iter().map(|row| row.author_id).collect();
    for author_id in [
        "menu-operator",
        "menu.operator.command-palette",
        "menu.operator.swarm-board",
        "menu.operator.flight-recorder",
        "menu.operator.model-session-launch",
        "menu.operator.user-manual",
        "menu.operator.settings",
        "flight-recorder.refresh",
        "flight-recorder.load-failure",
        "flight-recorder.quarantine-status",
        "flight-recorder.error-ring",
        "fems-propose-dialog",
        "fems-class-episodic",
        "fems-class-semantic",
        "fems-class-procedural",
        "fems-propose-confirm",
        "fems-propose-cancel",
        "fems-propose-status",
        "fems-review-approve",
        "fems-review-reject",
        "fems-review-status",
        "fems-review-refresh-retry",
    ] {
        assert!(
            row_ids.contains(author_id),
            "agent-tool index must include live operator/FEMS id '{author_id}'"
        );
    }

    let section = editors_manual_section();
    let fems = topic_body(&section, "Relevant Memory (FEMS)");
    for unreachable in [
        "event_rejected",
        "event_persistence_failed",
        "event_persistence_timeout",
    ] {
        assert!(
            !fems.contains(unreachable),
            "manual must not present unreachable frontend proposal outcome '{unreachable}'"
        );
    }
}

#[test]
fn notes_load_recovery_manual_matches_latched_runtime_contract() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Opening Editing and Saving Notes");
    for required in [
        NOTES_LOAD_ERROR_AUTHOR_ID,
        NOTES_LOAD_RETRY_AUTHOR_ID,
        "does not spin an automatic GET/repaint retry loop",
        "issues one new GET",
        "another retry requires another explicit click",
    ] {
        assert!(
            body.contains(required),
            "Notes load recovery manual must document runtime contract '{required}'"
        );
    }

    let rows = row_by_id();
    let retry = rows
        .get(NOTES_LOAD_RETRY_AUTHOR_ID)
        .expect("Notes load Retry agent-tool row exists");
    assert_eq!(retry.surface, ManualSurface::RichText);
    assert_eq!(retry.mcp_tool, handshake_native::mcp::ARGUS_CLICK_METHOD);
    assert!(retry.description.contains(NOTES_LOAD_ERROR_AUTHOR_ID));
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
            row.mcp_tool,
            handshake_native::mcp::ARGUS_CLICK_METHOD,
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
            row.mcp_tool,
            handshake_native::mcp::ARGUS_CLICK_METHOD,
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
        "Wiki Projection" => "dedicated generated Loom wiki-page surface",
        "Knowledge Graph" => "Loom graph view",
        "Folder Tree" => "The Folder Tree is the native Obsidian-style folder surface",
        "Tags and Tag Hubs" => {
            "Tags and Tag Hubs are the native Obsidian-style tag navigation surface"
        }
        "Block Collection Views" => {
            "Block Collection Views are the mounted saved table, Kanban, and calendar projections"
        }
        "Canvas" => "free-form spatial board",
        "Search" => "three complementary search surfaces",
        "Wikilinks and Backlinks" => "Wikilinks tie notes together",
        "Daily Journal" => "date-addressed note surface",
        "Diff and Merge" => "VS Code-style side-by-side and inline diffs",
        "Internationalization" => "SINGLE shared Unicode text-mechanics",
        "Menu Bar and Commands" => "eight top-level dropdowns",
        "Editor Settings" => "Editor preferences live in the Settings dialog",
        "Signature Help, Rename, and Quick Fix" => {
            "The code editor has VS Code-parity symbol-intelligence actions"
        }
        "Outline and Table of Contents" => {
            "The Outline (table of contents) is the document-structure side pane"
        }
        "Relevant Memory (FEMS)" => "Relevant Memory is the FEMS (Pillar 12) retrieval side pane",
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
            "max_depth",
            "graph.mode.local",
            "graph.mode.global",
            "graph.zoom.in",
            "graph.zoom.out",
            "graph.relayout",
            "graph.retry",
            "graph.node.{block_id}",
            "TreeItem",
            "GET /workspaces/{id}/loom/views/all",
            "GET /workspaces/{id}/loom/graph/global",
            "GET /workspaces/{id}/loom/graph/local",
            "start_block_id",
            "max_depth",
            "truncated",
            "suppressed_hub_ids",
            "LoomGraphView::set_graph_projection",
            "graph_view_live_pg_self_seeds_local_global",
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
            "folder_tree_live_pg_self_seeded_round_trip",
            "cleanup_verified=true",
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
            "tags_tag_hub_live_pg_self_seeds_mounted_round_trip",
        ],
        "Block Collection Views" => &[
            "menu.view.open-block-collections",
            "view.block-collections",
            "block.content_type is view_def",
            "bcv.new-view",
            "bcv.new-view.title",
            "bcv.new-view.kind.table",
            "bcv.new-view.kind.kanban",
            "bcv.new-view.kind.calendar",
            "bcv.new-view.confirm",
            "stable client-generated block_id",
            "transactional PostgreSQL outbox",
            "bcv.table.sort.{field}",
            "bcv.table.row.{block_id}",
            "bcv.kanban.lane.{key}",
            "bcv.kanban.card.{block_id}",
            "bcv.calendar.apply-range",
            "bcv.kind.table",
            "No blocks match this view.",
            "No Kanban lanes.",
            "No blocks in this date range.",
            "bcv.status",
            "View error: ...",
            "bcv.retry",
            "one bounded definition fetch and one bounded results query",
            "Tier 1 Flight Recorder is WIRED",
            "Tier 2 internal_diagnostics is WIRED",
            "Tier 3 Palmistry is WIRED",
            "tests/run_mt027_argus_proof.ps1",
            "real localhost Argus transport",
        ],
        "Canvas" => &[
            "canvas.add-card",
            "canvas.retry",
            "getCanvasBoard",
            "cross-pane MT-035 compensating undo",
            "DELETE /workspaces/{id}/loom/canvas-placements/{placement_id}",
            "Inline text-card edit remains a typed blocker",
        ],
        "Search" => &[
            "search.query",
            "loom-search-v2.save-view",
            "semantic_available",
            "POST /workspaces/{workspace_id}/loom/views/definitions",
            "reloadable view block id",
            "Workspace switches clear results, facets, errors, save receipts, and pending deliveries",
            "menu.edit.find-all",
            "find-in-files.query",
            "find-in-files.preview-replace",
            "find-in-files.apply",
            "/workspaces/{workspace_id}/loom/graph-search",
            "find-in-files.result.{hex(source_kind UTF-8 bytes)}.{hex(ref_id UTF-8 bytes)}",
            "find-in-files.result.646f63756d656e74.4b52442d313a2f666f6f3f783d31",
            "find-in-files.result.e69687e6a1a3.72c3a973756dc3a92fe69db1e4baac",
            "hex-encoded",
            "list_widgets instead of guessing",
            "PaneType::LoomWikiPage",
            "PaneType::LoomBlock",
            "PaneType::CodeSymbol",
            "PaneType::KernelDcc at WP:{wp_id}",
            "PaneType::KernelDcc at MT:{wp_id}:{mt_id}",
            "PaneType::UserManual at page_slug",
            "dedicated Wiki Page projection placeholder pane",
            "block.content_type is view_def",
            "mounted Block Collections pane",
            "View error: ...",
            BCV_RETRY_AUTHOR_ID,
            "clears stale deliveries",
            "rebinds the same view id",
            "one bounded definition fetch plus one bounded result query",
            "/workspaces/{workspace_id}/search-bookmarks",
            "bookmark-v1 followed by one .{utf8_len}-{hex(component UTF-8 bytes)} frame",
            "never lowercases semantic content",
            "case-sensitive Foo/foo and Unicode-only 文/東 searches",
            "find-in-files.bookmark-restore.{hex(bookmark_id UTF-8 bytes)}",
            "find-in-files.bookmark-remove.{hex(bookmark_id UTF-8 bytes)}",
            "73617665643ae696872f31",
            "find-in-files.bookmark-retry",
            "find-in-files.preview.{hex(document_id UTF-8 bytes)}",
            "find-in-files.preview.4b52442de696872f31",
            "/knowledge/documents/{id}/save",
            "expected_version",
            "CommittedWithoutReceipt",
            "reloads the document to reverify both identities immediately before save",
            "automatically re-runs the same search",
            "Cancel is cooperative between saves",
            "Search never overlaps Apply",
            "result click/open_requests shell target",
            "exact id/title/content/version",
            "production Bookmark Search saves for case-sensitive case variants",
            "UI Restore-all-fields",
            "second fresh-remount absence",
            "fresh GET /workspaces list absence and a failed graph-search refetch",
            "every graph-search page emits LoomSearchExecuted",
            "internal_diagnostics = DEFERRED-with-reason",
            "Palmistry = DEFERRED-with-reason",
            "quick-switcher.dialog",
            "command-palette.search",
        ],
        "Wikilinks and Backlinks" => &[
            "outgoing.panel",
            "ShellNavigator",
            "outgoing.section.resolved",
            "backlinks-panel",
            "backlinks-refresh",
            "sticky typed indexing failure",
            "same source document",
            "Invalidation state is workspace-stamped",
            "bounded revision window",
            "latest revision per workspace",
            "workspace plus normalized title",
            "successful off-workspace create is cached",
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
            "same normalized title or alias",
            "add_local_alias is the sole alias source",
            "A restart or fresh resolver seed restores titles but cannot restore aliases",
            "not durable backend alias coverage",
            "status says Created only when created=true",
            "Opened existing/reused when created=false",
            "scoped by origin pane plus source content",
            "A new B intent cancels B-old only",
            "GET /knowledge/documents?workspace_id=...",
            "Rename disables reentry",
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
            "live_pg_self_seeded_loom_block_backlink_hash_and_ui_proof",
            "ReqwestWikilinkBackend/WikilinkRuntime",
            "MT-032-canvas-live-B.png",
            "every already-mounted backlink panel",
            "typed indexing failure",
            "typed backend-shape gap",
        ],
        "Daily Journal" => &[
            "daily-journal-panel",
            "journal-panel-root",
            "journal-start-writing",
            "journal-document-link-gap",
            "view.journal",
            "PUT /workspaces/:workspace_id/loom/journals/:date",
            "PUT /knowledge/documents/:id/save",
            "managed-backend proof starts with the navigated date absent",
            "one durable journal identity that reopens idempotently",
            "EndpointUnavailable",
            "workspace-plus-date request",
            "rapid navigation discards late responses",
            "Settings > Appearance > Calendar timezone",
            "workspace-plus-date-plus-view-timezone request",
            "validated IANA tzid per workspace",
            "[start_date, end_date_exclusive)",
            "23-hour and 25-hour days",
            "timed midnight-exclusive endings",
            "nonexistent DST-gap local times are rejected",
            "explicit earlier/later-offset normalization note",
            "Today is derived in that persisted view timezone",
            "daily-journal-calendar-normalization-badge",
            "calendar-event-legacy-badge",
            "Legacy temporal data",
            "reimport from the calendar source",
            "without ended_utc is rendered as In progress",
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
        "Wiki Projection" => &[
            "menu.view.open-wiki-projection",
            "view.wiki-projection",
            "workspace id, projection id, and pane generation",
            "last-good rebuild preservation",
            "no dedicated preference",
            "generated live ids",
        ],
        "Menu Bar and Commands" => &[
            "menu-file",
            "menu-editors, Alt+I",
            "menu-operator, Alt+O",
            "menu.operator.command-palette",
            "menu.operator.swarm-board",
            "menu.operator.flight-recorder",
            "menu.operator.model-session-launch",
            "menu.operator.user-manual",
            "menu.operator.settings",
            "command-palette.dialog",
            "flight-recorder-pane",
            "model-session-launch.dialog",
            "manual-pane",
            "settings.dialog",
            "menu.edit.undo",
            "Open Editor Surfaces",
            "menu.view.open-code-editor",
            "menu.view.open-folders",
            "menu.view.open-tags",
            "menu.view.open-block-collections",
            "command-palette.option.hs-view-palette-code-editor",
            "command-palette.option.hs-view-palette-rich-note",
            "command-palette.option.hs-view-palette-wiki-projection",
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
            "settings-editor-minimap",
            "settings-editor-sticky-scroll",
            "settings-editor-line-height",
            "settings-editor-reading-mode-default",
            "settings-editor-wiki-projection-posture",
            "settings-editor-flight-recorder-posture",
            "workspace filter is runtime-derived",
            "no dedicated preference",
            "settings-syntax-palette-mode",
            "PUT /workspaces/:id/settings",
            "mounted code editor and rich editor",
            "repaints the mounted code editor",
            "settings.persist.retry",
        ],
        "Signature Help, Rename, and Quick Fix" => &[
            "Signature help",
            "F2",
            "Rename Symbol",
            "begin_rename_at_cursor",
            "Quick Fix",
            "Ctrl+.",
            "quick_fix_request",
            "editor.rename.symbol",
            "editor.quickFix",
            "menu.editors.rename-symbol",
            "menu.editors.quick-fix",
            "dispatch_editor_command",
        ],
        "Outline and Table of Contents" => &[
            "view.outline",
            "code_editor/outline.rs",
            "menu.editors.outline",
            "command-palette.option.hs-view-palette-outline",
            "table of contents",
        ],
        "Relevant Memory (FEMS)" => &[
            "view.relevant-memory",
            "relevant-memory-panel",
            "relevant-memory-list",
            "menu.editors.relevant-memory",
            "command-palette.option.hs-view-palette-relevant-memory",
            "editor.fems.memorypack-refresh",
            "editor.fems.memorypack-status",
            "pending_review",
            "FR-EVT-MEM-001",
            "FR-EVT-MEM-002",
            "FR-EVT-MEM-003",
            "FR-EVT-MEM-004",
            "FR-EVT-MEM-005",
            "fems-propose-dialog",
            "fems-class-episodic",
            "fems-class-semantic",
            "fems-class-procedural",
            "fems-propose-confirm",
            "fems-propose-cancel",
            "fems-propose-status",
            "FR-EVT-MEM-001 is backend-owned",
            "artifact_ref=artifact://sha256/{proposal_hash}",
            "artifact_ref=artifact://sha256/{commit_report_hash}",
            "artifact_ref=artifact://sha256/{memory_pack_hash}",
            "schema_version=hsk.memory_write_proposal@0.1",
            "the event contains no raw memory content",
            "retries converge across native-process restarts",
            "terminal identical proposal remains the same reviewed intent",
            "fems-review-approve",
            "fems-review-reject",
            "fems-review-status",
            "fems-review-refresh-retry",
            "state=reviewed;outcome=rejected;terminal=true",
            "state=committed;outcome=approved",
            "memory_id",
            "commit_id",
            "memory_pack_id",
            "commit_report_hash",
            "separate explicit approved-proposal commit route",
            "Exact retries",
            "event_ledger_event_id",
            "flight_recorder_event_id",
            "KSRC-* KnowledgeSource",
            "knowledge_code_files",
            "full-buffer SHA-256",
            "canonical proposal-row count",
            "identical range",
            "Relevant Memory utility tab",
            "outcome=reentry_blocked",
            "late delivery is discarded",
            "Flight Recorder/EventLedger=WIRED",
            "internal_diagnostics=WIRED",
            "Palmistry=WIRED",
        ],
        other => panic!("unknown wave-5 surface topic '{other}'"),
    }
}

#[test]
fn mt067_calendar_temporal_manual_documents_lossless_local_date_workflow() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Daily Journal");
    for needle in wave5_needles("Daily Journal") {
        assert!(
            manual_body_contains(body, needle),
            "MT-067 Daily Journal manual must include concrete temporal fact '{needle}'"
        );
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
        "graph.retry",
        GRAPH_NODE_AUTHOR_ID_PATTERN,
        "encoded injectively",
        "GET /workspaces/{id}/loom/views/all",
        "GET /workspaces/{id}/loom/graph/global",
        "GET /workspaces/{id}/loom/graph/local",
        "start_block_id",
        "max_depth",
        "truncated",
        "suppressed_hub_ids",
        "ModeChanged",
        "AddEdge",
        "RemoveEdge",
        "LoomGraphView::set_graph_projection",
        "graph_view_live_pg_self_seeds_local_global",
        "real pre-seed 0-node Global projection",
        "bounded typed transport failure",
        "cleanup guard removes the seeded workspace",
        "missing backend fails that proof",
        "0 nodes",
        "Graph error:",
        "list_widgets",
        "switch graph.mode.local / graph.mode.global",
        "click graph.relayout",
        "click graph.retry",
        "A -> B -> A",
        "Palmistry",
    ] {
        assert!(
            manual_body_contains(body, needle),
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
        "Handshake-managed PostgreSQL backend",
        "Flight Recorder/EventLedger",
        "folder_tree_live_pg_self_seeded_round_trip",
        "cleanup_verified=true",
        "list_widgets",
        "click_widget",
    ] {
        assert!(
            manual_body_contains(body, needle),
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
        assert_eq!(row.mcp_tool, canonical_tool_name(tool));
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
            manual_body_contains(body, needle),
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
        assert_eq!(row.mcp_tool, canonical_tool_name(tool));
        assert_eq!(row.action_label, label);
    }
}

#[test]
fn mt024_sidebar_manual_documents_runtime_routes_ids_and_recovery() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Wikilinks and Backlinks");

    for needle in [
        "EDITORS > Open Sidebar",
        "menu.editors.sidebar",
        "view.sidebar",
        "Pins, Favorites, Backlinks, and Unlinked Mentions",
        "PUT /pin-order {pin_order:null}",
        "PATCH {pinned:false}",
        "PATCH {favorite:false}",
        "sidebar.pin.{encoded_block_id}",
        "sidebar.favorite.{encoded_block_id}",
        "sidebar.backlink.{encoded_block_id}",
        "sidebar.unlinked.{encoded_block_id}",
        "sidebar.breadcrumb.{index}",
        "injective u8-hex",
        "five-entry breadcrumb",
        "own loading/error state and Retry",
        "There is no sidebar preference in the WP contract",
    ] {
        assert!(
            manual_body_contains(body, needle),
            "MT-024 sidebar manual must document '{needle}'"
        );
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
        assert_eq!(row.mcp_tool, canonical_tool_name(tool));
        assert_eq!(row.action_label, label);
    }
}

#[test]
fn wave5_surface_topics_exist_and_carry_real_no_context_facts() {
    let section = editors_manual_section();
    assert_eq!(
        WP_SURFACE_HEADINGS.len(),
        18,
        "one dedicated topic per native editor surface (15 wave-5 + 3 MT-035 surfacing topics)"
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
                manual_body_contains(body, needle),
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
            "Muted, Standard, and Custom palette selections repaint the mounted code editor and minimap"
        ),
        "Editor Settings topic must document every live syntax-palette mode"
    );
    assert!(
        body.contains("Gutter line-number and fold glyphs use the same live editor font size"),
        "Editor Settings topic must document live gutter font sizing"
    );
    assert!(
        !body.contains("does not yet apply a live font size"),
        "Editor Settings topic must not keep the old stale inert-font-size claim"
    );
    assert!(
        body.contains("code keybindings, and rich keybindings apply to the mounted editors"),
        "Editor Settings topic must document both live runtime keymaps"
    );
    assert!(
        !body.contains("no live rich keymap seam"),
        "Editor Settings topic must not retain the retired rich-keymap blocker"
    );
    assert!(
        body.contains("settings.persist.error") && body.contains("settings.persist.retry"),
        "Editor Settings topic must document visible persistence failure and exact retry recovery"
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

// ── WP-KERNEL-012 MT-035 wave: the 3 native-editor surfacing topics are present with substantive bodies ──

/// Heading-presence proof for the three MT-035 surfacing topics (Signature Help/Rename/Quick Fix, the
/// document Outline, and the FEMS Relevant Memory pane): each is a registered topic in the editors
/// section with a substantive no-context body, and its heading is enumerated in WP_SURFACE_HEADINGS.
#[test]
fn mt035_surfacing_topics_present_with_substantive_bodies() {
    let section = editors_manual_section();
    for heading in [
        "Signature Help, Rename, and Quick Fix",
        "Outline and Table of Contents",
        "Relevant Memory (FEMS)",
    ] {
        assert!(
            WP_SURFACE_HEADINGS.contains(&heading),
            "MT-035 heading '{heading}' must be enumerated in WP_SURFACE_HEADINGS"
        );
        let body = topic_body(&section, heading);
        assert!(
            body.len() > 220,
            "MT-035 topic '{heading}' must carry a substantive no-context body (got {} chars)",
            body.len()
        );
        for needle in wave5_needles(heading) {
            assert!(
                manual_body_contains(body, needle),
                "MT-035 topic '{heading}' must include concrete runtime fact '{needle}'"
            );
        }
    }
}

#[test]
fn mt108_manual_documents_replace_cap_watchdog_recovery_and_three_tier_posture() {
    let section = editors_manual_section();
    let body = topic_body(&section, "Residual Hardening and Argus Evidence");
    for needle in [
        "1000 matches per click",
        "click Replace All again",
        "normal undo command reverses it",
        "progress-gap deadline",
        "hard total-runtime cap",
        "register_backend_operation",
        "Completion clears the active stalled count",
        "Tier 1 Flight Recorder",
        "Tier 2 internal_diagnostics is WIRED",
        "Tier 3 Palmistry is WIRED",
        "hsk.native_gui.screenshot_marker@5",
        "33 required GUI scenarios",
        "tests/run_mt108_argus_proof.ps1",
        "hsk.native_gui.external_process_receipt@3",
        "hsk.native_gui.process_observation_ack@1",
        "zero owned survivors",
        "test-executable PID/start identity",
        "RECLAIM_FAILED",
        "hsk.native_gui.argus_surface_evidence@4",
        "source SHA, scenario, correlation id",
        "CAPTURED PNGs of at least 320x180",
        "NOT pixel closure",
        "test_mt108_argus_aggregate",
        "mt108_verify_argus_evidence_manifest",
        "Handshake_Artifacts\\handshake-cargo-target",
        "Handshake_Artifacts\\handshake-test\\wp-kernel-012-mt-108\\integrated",
        "x->x and x->xx terminate",
        "canonical-argus-matrix.jsonl",
        "argus-seven-surface.jsonl",
    ] {
        assert!(
            body.contains(needle),
            "MT-108 no-context manual must document '{needle}'"
        );
    }
}

#[test]
fn mt108_argus_evidence_matrix_retains_the_seven_contract_surface_subset() {
    let expected = [
        "find bar",
        "formatting toolbar",
        "slash menu",
        "outline pane",
        "rich find/replace panel",
        "runtime chat pane",
        "diagnostics panel",
    ];
    assert_eq!(MT108_ARGUS_EVIDENCE_MATRIX.len(), expected.len());
    assert_eq!(
        MT108_ARGUS_EVIDENCE_MATRIX
            .iter()
            .map(|row| row.surface)
            .collect::<Vec<_>>(),
        expected
    );
    for row in MT108_ARGUS_EVIDENCE_MATRIX {
        assert!(!row.inspect_author_id.is_empty());
        assert!(!row.steer_author_id.is_empty());
        assert!(
            matches!(
                row.steer_method,
                handshake_native::mcp::ARGUS_CLICK_METHOD
                    | handshake_native::mcp::ARGUS_SET_VALUE_METHOD
            ),
            "{} uses a canonical bounded Argus mutation method",
            row.surface
        );
        assert!(row.proof_binary.starts_with("test_"));
        assert!(row.proof_test.starts_with("mt108_argus_"));
        assert_eq!(
            row.automation_status,
            ArgusAutomationStatus::CanonicalServerLoop,
            "every matrix row must route through the canonical server loop"
        );
    }

    let section = editors_manual_section();
    let body = topic_body(&section, "Residual Hardening and Argus Evidence");
    for row in MT108_ARGUS_EVIDENCE_MATRIX {
        for needle in [
            row.surface,
            row.inspect_author_id,
            row.steer_method,
            row.steer_author_id,
            row.proof_binary,
            row.proof_test,
        ] {
            assert!(
                body.contains(needle),
                "rendered MT-108 matrix must include {needle}"
            );
        }
    }
    assert!(body.contains("fresh re-inspect, then argus.screenshot"));
    assert!(body.contains("real localhost SwarmMcpServer binding"));
    assert!(body.contains("client_session_id-derived agent_id"));
    assert!(body.contains("argus-seven-surface.jsonl"));
    assert!(
        !body.contains("argus.argus."),
        "canonical Argus tool names must remain idempotent in the rendered manual"
    );
}

#[test]
fn mt108_supervisor_is_hard_bounded_and_invokes_the_mandatory_verifier() {
    let runner = include_str!("run_mt108_argus_proof.ps1");
    for needle in [
        "$process.WaitForExit(25)",
        "$correlationId.exit-code",
        "wrapper-owned Cargo exit-code sidecar",
        "[int]::TryParse(",
        "$childStartedAt -lt $parentStartedAt",
        "Chronologically impossible",
        "ProcessInventoryErrors = @()",
        "owned process-tree capture was indeterminate",
        "$errors = @($ProcessContext.ProcessInventoryErrors)",
        "[AllowEmptyCollection()][object[]]$Snapshot",
        "$process.Kill($true)",
        "taskkill.exe",
        "'/PID', $process.Id, '/T', '/F'",
        "EXTERNAL_PROCESS_TIMEOUT",
        "PROCESS_TREE_RECLAIMED",
        "PROCESS_TREE_RECLAIM_FAILED",
        "hsk.native_gui.external_process_receipt@3",
        "child_pid",
        "owned_process_tree_pids",
        "command_arguments",
        "deadline_at_utc",
        "Assert-FinalProcessReceipts",
        "PROCESS_TREE_NOT_CLOSED",
        "cleanup_verified",
        "survivor_count_at_receipt",
        "handshake-test\\wp-kernel-012-mt-108\\integrated",
        "RunId '$RunId' is not fresh",
        "Assert-NoReparsePointEscape",
        "Resolve-ExternalPath",
        "external_process_receipts.jsonl",
        "mt108_argus_matrix.json",
        "test_mt108_argus_aggregate",
        "mt108_verify_argus_evidence_manifest",
    ] {
        assert!(
            runner.contains(needle),
            "MT-108 runner must retain '{needle}'"
        );
    }
}
