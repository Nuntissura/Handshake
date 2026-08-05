//! WP-KERNEL-012 MT-073 (E12) — the built-in **User Manual content for the native editors** (HBR-MAN),
//! plus the agent-vision/steering reference (HBR-VIS / HBR-SWARM).
//!
//! This module is CONTENT ONLY: it returns [`editors_manual_section`], a [`ManualSection`] (the data
//! type from [`crate::manual_pane`]) that the manual pane registers. It builds NO new manual subsystem —
//! it supplies the eight GLOBAL-BUILD-MANUAL topics + the four interop-edge documentation + the
//! `author_id -> MCP tool` steering index, all authored for a no-context model.
//!
//! ## Sourcing discipline (RISK-001/002/004 — VERIFIED against live code, not memory)
//!
//! - Every documented `author_id` is a LIVE registered id sourced from the real surfaces:
//!   * shell chrome — [`crate::accessibility::DECLARED_IDENTITIES`] (theme toggle, status bar, settings,
//!     quick-switcher containers); the command-palette container ids come from
//!     [`crate::command_palette::PALETTE_SEARCH_AUTHOR_ID`] / [`crate::command_palette::PALETTE_LIST_AUTHOR_ID`]
//!     (the DOT-form ids the live palette emits — `command-palette.search` / `command-palette.list`),
//!     NOT the interop hyphen-form that only fires inside a unit-test harness;
//!   * code editor — [`crate::accessibility::editor_action_registry::CODE_ACTION_CATALOG`] mapped through
//!     the `editor.code.<action>` convention ([`crate::accessibility::editor_action_registry::RegistrationHandle::author_id`]);
//!   * rich-text editor — `rich_action_catalog()` mapped through `editor.rich.<action>`;
//!   * graph / canvas / collection — the
//!     [`crate::accessibility::GRAPH_CONTROL_CATALOG`] / `CANVAS_CONTROL_CATALOG` / `COLLECTION_CONTROL_CATALOG`;
//!   * FEMS — `relevant-memory-panel` / `relevant-memory-list` /
//!     `editor.fems.memorypack-refresh` / `fems-propose-dialog` / `fems-propose-confirm` /
//!     `fems-review-approve` / `fems-review-reject`
//!     ([`crate::fems`]);
//!   * Stage — `stage-pane` / `stage-routed-content` / `stage-capture-embed-back` ([`crate::stage_pane`]);
//!   * Calendar — `daily-journal-panel` / `daily-journal-date-header` /
//!     `daily-journal-calendar-event-chip` / `daily-journal-activity-strip` ([`crate::graph::daily_journal_panel`]);
//!   * Locus — `outgoing.panel` / `outgoing.section.resolved` / `outgoing.section.unresolved`
//!     ([`crate::rich_editor::wikilinks::outgoing_links_panel`]) — the locus-ref chip lives inline.
//! - Every documented `mcp_tool` is one of the FOUR canonical [`crate::mcp::argus`] methods:
//!   `argus.inspect` / `argus.click` / `argus.set_value` / `argus.screenshot`. The older MCP method
//!   spellings remain transport aliases only and are not presented as the product contract.
//!
//! ## Honest interop-edge failure semantics (RISK-007)
//!
//! FEMS has a live PostgreSQL/EventLedger-backed read, review, and explicit approved-proposal commit
//! round trip. Stage,
//! Calendar, and Locus have live cross-edge routes; endpoint, fetch, and record failures remain typed and
//! visible so the manual never reports a fabricated success.

use crate::accessibility::editor_action_registry::{rich_action_catalog, CODE_ACTION_CATALOG};
use crate::accessibility::{
    CANVAS_CONTROL_CATALOG, COLLECTION_CONTROL_CATALOG, GRAPH_CONTROL_CATALOG,
};
use crate::app::NOTES_LOAD_RETRY_AUTHOR_ID;
use crate::command_palette::{PALETTE_LIST_AUTHOR_ID, PALETTE_SEARCH_AUTHOR_ID};
use crate::manual_pane::{
    AgentToolReference, AgentToolRow, ManualSection, ManualSurface, ManualTopic,
};
use crate::settings_editor_section::{
    editor_action_catalog, editor_keybind_row_author_id, EditorActionSurface,
    EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX, EDITOR_SETTINGS_CONTROL_AUTHOR_IDS,
    EDITOR_SETTINGS_OPTION_AUTHOR_IDS, SYNTAX_SWATCH_AUTHOR_IDS,
};

/// The stable section id for the native-editors manual section.
pub const EDITORS_SECTION_ID: &str = "native-editors";

/// The agent-tool reference heading (an addressable topic).
pub const AGENT_TOOL_REFERENCE_HEADING: &str = "Agent Tool Reference";

/// The eight required GLOBAL-BUILD-MANUAL headings, each an individual topic so the heading-presence test
/// can assert every one by name (AC-001 / MC-003). Order matters only for display; presence is the gate.
pub const REQUIRED_HEADINGS: &[&str] = &[
    "Purpose",
    "Core Workflows",
    "Startup and Run",
    "Inputs and Outputs",
    "Navigation Paths",
    "Safety Constraints",
    "Common Failure Modes",
    "Recovery Steps",
];

/// The four interop-edge names that MUST each appear in the interop topic with an associated author_id +
/// mcp_tool (AC-005 / MC-007).
pub const INTEROP_EDGES: &[&str] = &["FEMS", "Stage", "Calendar", "Locus"];

/// Dedicated MT-045/MT-046 operator/model topic: contract-sized large-document behavior and the four
/// editor-to-editor interconnection edge families are one addressable manual page.
pub const E8_PERFORMANCE_INTERCONNECTION_HEADING: &str =
    "Large Documents and Editor Interconnection";

/// WP-KERNEL-012 MT-104 product-manual topics added after the notes+chat, diagnostics, visual-debugger,
/// and foreground-safe navigation work landed.
pub const WP104_PRODUCT_HEADINGS: &[&str] = &[
    "Notes Worksurface and Chat",
    "Opening Editing and Saving Notes",
    "Terminal Launch",
    "Model Session Launch",
    "Settings Diagnostics",
    "Visual Debugger",
    "Foreground-Safe Navigation",
];

/// Dedicated diagnostic-tool topics. These are deliberately separate topics so a no-context model can
/// choose the correct tier without reading a long mixed diagnostics blob.
pub const DIAGNOSTIC_TOOL_HEADINGS: &[&str] =
    &["Flight Recorder", "internal_diagnostics", "Palmistry"];

/// WP-KERNEL-012 wave-5 full-WP surface topics: one dedicated, selectable, no-context topic per native
/// editor surface so a fresh model/operator can operate the WHOLE WP (VS Code code editor, Obsidian rich
/// editor, knowledge graph, canvas, search, wikilinks/backlinks, daily journal, diff/merge, the shared
/// i18n text layer, the operator menu bar, and the editor Settings section) — not only the generic
/// GLOBAL-BUILD-MANUAL topics. Each has its own heading so the heading-presence test asserts it by name.
pub const WP_SURFACE_HEADINGS: &[&str] = &[
    "Code Editor",
    "Rich Text Editor",
    "Wiki Projection",
    "Knowledge Graph",
    "Folder Tree",
    "Tags and Tag Hubs",
    "Block Collection Views",
    "Canvas",
    "Search",
    "Wikilinks and Backlinks",
    "Daily Journal",
    "Diff and Merge",
    "Internationalization",
    "Menu Bar and Commands",
    "Editor Settings",
    // WP-KERNEL-012 MT-035 wave (native-editor surfacing): three additional per-surface topics for the
    // code-editor language features (signature help / rename / quick fix), the document outline, and the
    // FEMS relevant-memory pane — each a dedicated no-context topic asserted by the heading-presence test.
    "Signature Help, Rename, and Quick Fix",
    "Outline and Table of Contents",
    "Relevant Memory (FEMS)",
];

pub const TERMINAL_MENU_AUTHOR_ID: &str = "menu.run.terminal";
pub const MODEL_SESSION_LAUNCH_MENU_AUTHOR_ID: &str =
    crate::top_menu_bar::MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID;
pub const MODEL_SESSION_LAUNCH_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-model-session-palette-launch-workspace";
pub const INFERENCE_LAB_MENU_AUTHOR_ID: &str = "menu.run.inference-lab";
pub const INFERENCE_LAB_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-inference-palette-open";
pub const FLIGHT_RECORDER_MENU_AUTHOR_ID: &str = "menu.run.flight-recorder";
pub const FLIGHT_RECORDER_PALETTE_AUTHOR_ID: &str = "command-palette.option.hs-flight-palette-open";
pub const SETTINGS_DIAGNOSTICS_SECTION_AUTHOR_ID: &str = "settings.section.diagnostics";
/// Exact seeded Gamma heading targeted by the MT-108 outline server-loop proof (`block path [3]`).
pub const MT108_ARGUS_OUTLINE_PROOF_AUTHOR_ID: &str = "outline.heading.re-block-3710791291";

/// One row in the original seven-surface MT-108 contract subset. The complete remediation matrix is
/// source-controlled in `tests/mt108_argus_matrix.json`; these rows retain the detailed command/operator
/// reference for the seven surfaces named directly by the microtask contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgusEvidenceRow {
    pub surface: &'static str,
    pub inspect_author_id: &'static str,
    pub steer_method: &'static str,
    pub steer_author_id: &'static str,
    pub proof_binary: &'static str,
    pub proof_test: &'static str,
    /// Automation path implemented by the named binary.
    pub automation_status: ArgusAutomationStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgusAutomationStatus {
    CanonicalServerLoop,
}

/// Original seven-surface Argus inventory named directly by MT-108. The V3 remediation expands this
/// subset through the source-controlled manifest to every WP-owned panel, window, and material state.
pub const MT108_ARGUS_EVIDENCE_MATRIX: &[ArgusEvidenceRow] = &[
    ArgusEvidenceRow {
        surface: "find bar",
        inspect_author_id: crate::code_editor::panel::CODE_EDITOR_FIND_BAR_AUTHOR_ID,
        steer_method: crate::mcp::argus::ARGUS_SET_VALUE_METHOD,
        steer_author_id: crate::code_editor::panel::CODE_EDITOR_FIND_BAR_AUTHOR_ID,
        proof_binary: "test_find_bar_accesskit",
        proof_test: "mt108_argus_find_bar_real_server_loop",
        automation_status: ArgusAutomationStatus::CanonicalServerLoop,
    },
    ArgusEvidenceRow {
        surface: "formatting toolbar",
        inspect_author_id: "toolbar-btn-toggle_bold",
        steer_method: crate::mcp::argus::ARGUS_CLICK_METHOD,
        steer_author_id: "toolbar-btn-toggle_bold",
        proof_binary: "test_formatting_toolbar",
        proof_test: "mt108_argus_formatting_toolbar_real_server_loop",
        automation_status: ArgusAutomationStatus::CanonicalServerLoop,
    },
    ArgusEvidenceRow {
        surface: "slash menu",
        inspect_author_id: crate::rich_editor::slash_commands::SLASH_MENU_AUTHOR_ID,
        steer_method: crate::mcp::argus::ARGUS_CLICK_METHOD,
        steer_author_id: "slash-item-paragraph",
        proof_binary: "test_slash_commands",
        proof_test: "mt108_argus_slash_menu_real_server_loop",
        automation_status: ArgusAutomationStatus::CanonicalServerLoop,
    },
    ArgusEvidenceRow {
        surface: "outline pane",
        inspect_author_id: crate::rich_editor::outline_panel::OUTLINE_CONTAINER_AUTHOR_ID,
        steer_method: crate::mcp::argus::ARGUS_CLICK_METHOD,
        steer_author_id: MT108_ARGUS_OUTLINE_PROOF_AUTHOR_ID,
        proof_binary: "test_outline",
        proof_test: "mt108_argus_outline_real_server_loop",
        automation_status: ArgusAutomationStatus::CanonicalServerLoop,
    },
    ArgusEvidenceRow {
        surface: "rich find/replace panel",
        inspect_author_id: crate::rich_editor::find_replace::FIND_PANEL_AUTHOR_ID,
        steer_method: crate::mcp::argus::ARGUS_SET_VALUE_METHOD,
        steer_author_id: crate::rich_editor::find_replace::FIND_INPUT_AUTHOR_ID,
        proof_binary: "test_rich_find_replace",
        proof_test: "mt108_argus_rich_find_replace_real_server_loop",
        automation_status: ArgusAutomationStatus::CanonicalServerLoop,
    },
    ArgusEvidenceRow {
        surface: "runtime chat pane",
        inspect_author_id: crate::runtime_chat::RUNTIME_CHAT_PANEL_AUTHOR_ID,
        steer_method: crate::mcp::argus::ARGUS_SET_VALUE_METHOD,
        steer_author_id: crate::runtime_chat::RUNTIME_CHAT_INPUT_AUTHOR_ID,
        proof_binary: "test_runtime_chat_pane",
        proof_test: "mt108_argus_runtime_chat_real_server_loop",
        automation_status: ArgusAutomationStatus::CanonicalServerLoop,
    },
    ArgusEvidenceRow {
        surface: "diagnostics panel",
        inspect_author_id: crate::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
        steer_method: crate::mcp::argus::ARGUS_CLICK_METHOD,
        steer_author_id: crate::visual_debugger::WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID,
        proof_binary: "test_diagnostics_panel",
        proof_test: "mt108_argus_diagnostics_panel_real_server_loop",
        automation_status: ArgusAutomationStatus::CanonicalServerLoop,
    },
];
pub const VIEW_OPEN_CODE_EDITOR_MENU_AUTHOR_ID: &str = "menu.view.open-code-editor";
pub const VIEW_OPEN_RICH_NOTE_MENU_AUTHOR_ID: &str = "menu.view.open-rich-note";
pub const VIEW_OPEN_WIKI_PROJECTION_MENU_AUTHOR_ID: &str = "menu.view.open-wiki-projection";
pub const VIEW_OPEN_KNOWLEDGE_GRAPH_MENU_AUTHOR_ID: &str = "menu.view.open-knowledge-graph";
pub const VIEW_OPEN_FOLDERS_MENU_AUTHOR_ID: &str = "menu.view.open-folders";
pub const VIEW_OPEN_TAGS_MENU_AUTHOR_ID: &str = "menu.view.open-tags";
pub const VIEW_OPEN_BLOCK_COLLECTIONS_MENU_AUTHOR_ID: &str = "menu.view.open-block-collections";
pub const VIEW_OPEN_CANVAS_MENU_AUTHOR_ID: &str = "menu.view.open-canvas";
pub const VIEW_OPEN_LOOM_SEARCH_MENU_AUTHOR_ID: &str = "menu.view.open-loom-search";
pub const VIEW_OPEN_FIND_IN_FILES_MENU_AUTHOR_ID: &str = "menu.view.open-find-in-files";
pub const VIEW_OPEN_QUICK_SWITCHER_MENU_AUTHOR_ID: &str = "menu.view.open-quick-switcher";
pub const VIEW_OPEN_DAILY_JOURNAL_MENU_AUTHOR_ID: &str = "menu.view.open-daily-journal";
pub const VIEW_OPEN_DIFF_EDITOR_MENU_AUTHOR_ID: &str = "menu.view.open-diff-editor";
pub const VIEW_OPEN_CODE_EDITOR_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-code-editor";
pub const VIEW_OPEN_RICH_NOTE_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-rich-note";
pub const VIEW_OPEN_WIKI_PROJECTION_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-wiki-projection";
pub const VIEW_OPEN_KNOWLEDGE_GRAPH_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-graph";
pub const VIEW_OPEN_FOLDERS_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-folders";
pub const VIEW_OPEN_TAGS_PALETTE_AUTHOR_ID: &str = "command-palette.option.hs-view-palette-tags";
pub const VIEW_OPEN_CANVAS_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-canvas";
pub const VIEW_OPEN_LOOM_SEARCH_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-loom-search";
pub const VIEW_OPEN_FIND_IN_FILES_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-find-in-files";
pub const VIEW_OPEN_QUICK_SWITCHER_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-editor-menu-quick-open";
pub const VIEW_OPEN_DAILY_JOURNAL_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-journal";
pub const VIEW_OPEN_DIFF_EDITOR_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-view-palette-diff-merge";
pub const CONFLICT_KEEP_YOURS_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::CONFLICT_KEEP_YOURS_AUTHOR_ID;
pub const CONFLICT_KEEP_SERVER_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::CONFLICT_KEEP_SERVER_AUTHOR_ID;
pub const CONFLICT_OPEN_MERGE_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::CONFLICT_OPEN_MERGE_AUTHOR_ID;
pub const CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID;
pub const DRAFT_BANNER_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::DRAFT_BANNER_AUTHOR_ID;
pub const DRAFT_RESTORE_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::DRAFT_RESTORE_AUTHOR_ID;
pub const DRAFT_DISCARD_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::DRAFT_DISCARD_AUTHOR_ID;
pub const RICH_EDITOR_EXPORT_BUTTON_AUTHOR_ID: &str =
    crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget::EXPORT_BUTTON_AUTHOR_ID;
pub const EXPORT_FORMAT_PICKER_AUTHOR_ID: &str =
    crate::rich_editor::save::conflict_ui::EXPORT_PICKER_AUTHOR_ID;
pub const GRAPH_MODE_LOCAL_AUTHOR_ID: &str = crate::graph::MODE_LOCAL_AUTHOR_ID;
pub const GRAPH_MODE_GLOBAL_AUTHOR_ID: &str = crate::graph::MODE_GLOBAL_AUTHOR_ID;
pub const GRAPH_ZOOM_IN_AUTHOR_ID: &str = crate::graph::ZOOM_IN_AUTHOR_ID;
pub const GRAPH_ZOOM_OUT_AUTHOR_ID: &str = crate::graph::ZOOM_OUT_AUTHOR_ID;
pub const GRAPH_RELAYOUT_AUTHOR_ID: &str = crate::graph::RELAYOUT_AUTHOR_ID;
pub const GRAPH_RETRY_AUTHOR_ID: &str = crate::graph::graph_view::RETRY_AUTHOR_ID;
pub const GRAPH_NODE_AUTHOR_ID_PATTERN: &str = "graph.node.{block_id}";
pub const FOLDER_TREE_NODE_AUTHOR_ID_PATTERN: &str = "folder-tree.node.{folder_id}";
pub const FOLDER_TREE_COLOR_AUTHOR_ID_PATTERN: &str = "folder-tree.color.{folder_id}";
pub const FOLDER_TREE_RETRY_AUTHOR_ID: &str = crate::graph::RETRY_AUTHOR_ID;
pub const TAGS_SEARCH_AUTHOR_ID: &str = crate::graph::tags_panel::SEARCH_AUTHOR_ID;
pub const TAG_ROW_AUTHOR_ID_PATTERN: &str = "tags.row.{block_id}";
pub const TAG_HUB_TITLE_AUTHOR_ID_PATTERN: &str = "tag-hub.title.{block_id}";
pub const TAG_HUB_MEMBER_AUTHOR_ID_PATTERN: &str = "tag-hub.member.{block_id}";
pub const TAG_HUB_ADD_TAG_AUTHOR_ID_PATTERN: &str = "tag-hub.add-tag.{block_id}";

/// Build the native-editors manual section: the eight GLOBAL-BUILD-MANUAL topics, an interop topic
/// naming all four cross-pillar edges, and the `author_id -> MCP tool` agent-tool reference.
pub fn editors_manual_section() -> ManualSection {
    let mut topics = vec![
        ManualTopic {
            heading: "Purpose",
            body: purpose_body(),
        },
        ManualTopic {
            heading: "Core Workflows",
            body: core_workflows_body(),
        },
        ManualTopic {
            heading: "Startup and Run",
            body: startup_and_run_body(),
        },
        ManualTopic {
            heading: "Inputs and Outputs",
            body: inputs_and_outputs_body(),
        },
        ManualTopic {
            heading: "Navigation Paths",
            body: navigation_paths_body(),
        },
        ManualTopic {
            heading: "Safety Constraints",
            body: safety_constraints_body(),
        },
        ManualTopic {
            heading: "Common Failure Modes",
            body: common_failure_modes_body(),
        },
        ManualTopic {
            heading: "Recovery Steps",
            body: recovery_steps_body(),
        },
    ];
    // The interop topic (its own addressable topic). AC-005/MC-007 assert all four edge names + an
    // author_id + mcp_tool appear in this topic's body.
    topics.push(ManualTopic {
        heading: "Interop Edges",
        body: interop_edges_body(),
    });
    topics.push(ManualTopic {
        heading: E8_PERFORMANCE_INTERCONNECTION_HEADING,
        body: large_documents_interconnection_body(),
    });
    for (heading, body) in [
        (
            "Notes Worksurface and Chat",
            notes_worksurface_and_chat_body(),
        ),
        (
            "Opening Editing and Saving Notes",
            opening_editing_saving_notes_body(),
        ),
        ("Terminal Launch", terminal_launch_body()),
        ("Model Session Launch", model_session_launch_body()),
        ("Settings Diagnostics", settings_diagnostics_body()),
        ("Visual Debugger", visual_debugger_body()),
        (
            "Foreground-Safe Navigation",
            foreground_safe_navigation_body(),
        ),
        ("Flight Recorder", flight_recorder_body()),
        ("internal_diagnostics", internal_diagnostics_body()),
        ("Palmistry", palmistry_body()),
        (
            "Residual Hardening and Argus Evidence",
            mt108_hardening_body(),
        ),
    ] {
        topics.push(ManualTopic { heading, body });
    }
    // WP-KERNEL-012 wave-5: one dedicated topic per native editor surface (full-WP coverage), so a
    // no-context model can operate each surface directly from its own manual topic.
    for (heading, body) in [
        ("Code Editor", code_editor_body()),
        ("Rich Text Editor", rich_text_editor_body()),
        ("Wiki Projection", wiki_projection_body()),
        ("Knowledge Graph", knowledge_graph_body()),
        ("Folder Tree", folder_tree_body()),
        ("Tags and Tag Hubs", tags_and_tag_hubs_body()),
        ("Block Collection Views", block_collection_views_body()),
        ("Canvas", canvas_body()),
        ("Search", search_body()),
        ("Wikilinks and Backlinks", wikilinks_backlinks_body()),
        ("Daily Journal", daily_journal_body()),
        ("Diff and Merge", diff_and_merge_body()),
        ("Internationalization", internationalization_body()),
        ("Menu Bar and Commands", menu_bar_and_commands_body()),
        ("Editor Settings", editor_settings_body()),
        // MT-035 wave: surfacing topics for language features, outline, and relevant memory.
        (
            "Signature Help, Rename, and Quick Fix",
            signature_rename_quickfix_body(),
        ),
        ("Outline and Table of Contents", outline_toc_body()),
        ("Relevant Memory (FEMS)", relevant_memory_body()),
    ] {
        topics.push(ManualTopic { heading, body });
    }
    // Historical body sources predate the canonical Argus namespace. Normalize the rendered manual
    // projection in one place so no no-context workflow presents a compatibility alias as primary.
    for topic in &mut topics {
        topic.body = canonical_argus_prose(&topic.body);
    }
    // The agent-tool reference is also a searchable/selectable topic (so the search box surfaces it), and
    // its structured rows live in `agent_tools`.
    topics.push(ManualTopic {
        heading: AGENT_TOOL_REFERENCE_HEADING,
        body: agent_tool_reference_body(),
    });

    ManualSection {
        id: EDITORS_SECTION_ID,
        title: "Native Editors",
        topics,
        agent_tools: Some(AgentToolReference {
            heading: AGENT_TOOL_REFERENCE_HEADING,
            rows: agent_tool_rows(),
        }),
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────────
// GLOBAL-BUILD-MANUAL topic bodies (no-context: concrete commands, panes, AccessKit ids, keybinds).
// ─────────────────────────────────────────────────────────────────────────────────────────────────────

fn mt108_hardening_body() -> String {
    let matrix = MT108_ARGUS_EVIDENCE_MATRIX
        .iter()
        .map(|row| {
            format!(
                "{}: inspect author_id={}, steer with {} target={}, fresh re-inspect, then argus.screenshot through the real localhost JSON-RPC/session/action-channel route; exact command cargo test --test {} {} -- --exact --nocapture ({:?})",
                row.surface,
                row.inspect_author_id,
                row.steer_method,
                row.steer_author_id,
                row.proof_binary,
                row.proof_test,
                row.automation_status
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let body = format!(
        "Replace All is deliberately bounded to 1000 matches per click from the ORIGINAL match set so one frame cannot perform an unbounded edit. After a capped click the find bar reports '<N> more not yet replaced — click Replace All again'. The continuation retains the original match set and advances by buffer-version-checked offsets, so x->x and x->xx terminate without reprocessing replacement-generated matches. Each click is one recoverable editor mutation and the normal undo command reverses it. Changing the query, replacement, toggles, buffer, or closing/reopening Find invalidates the prior continuation.\n\nThe operation watchdog has independent progress-gap and hard total-runtime clocks. The progress-gap deadline resets only when progress ticks; the hard total-runtime cap never resets, so a forever-ticking backend operation is still reported. Production health and layout requests use both bounds through register_backend_operation. StalledOperation is observational: it does not cancel, retry, or duplicate a request. Completion clears the active stalled count; retry receives a new operation id.\n\nHBR-INT-009 posture: Tier 1 Flight Recorder remains the business EventLedger. Tier 2 internal_diagnostics is WIRED and projects the typed allowlisted StalledOperation row through status and Settings Diagnostics without project content. Tier 3 Palmistry is WIRED at the shared diagnostic ring and can retain the last-N event while the UI is frozen.\n\nV3 Argus closure is manifest-driven. `tests/mt108_argus_matrix.json` owns 33 required GUI scenarios: the seven contract surfaces plus code/rich editor hosts, Canvas, Graph, Folders, Tags, Sidebar, Outgoing Links, Relevant Memory, Atelier, Stage, Calendar, Block Collections, Diff/Merge, Loom Search, Find in Files, Wiki Projection, User Manual, Flight Recorder, Settings, FEMS Propose to Memory, Command Palette, Quick Switcher, enabled/disabled context menus, and Locus resolved/missing navigation. Every row names a stable exact author_id or prefix, an edge/material state, an exact test binary/test, and required capture. Each action runs through the real localhost SwarmMcpServer binding, retains the client_session_id-derived agent_id, then a fresh inspect proves the declared post-state. `terminal.open-workspace` and `model-session.launch-workspace` are explicitly excluded because they are process-launch commands with no WP-owned panel or window; their runtime contracts are not substituted for GUI proof.\n\nThe runner must be launched from `src/frontend/handshake_native` with a fresh RunId: `$runId = 'mt108-' + [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(); powershell -NoProfile -ExecutionPolicy Bypass -File tests/run_mt108_argus_proof.ps1 -RunId $runId`. It accepts only the allocated external roots `Handshake_Artifacts\\handshake-cargo-target` and `Handshake_Artifacts\\handshake-test\\wp-kernel-012-mt-108\\integrated`, rejects reparse-point escapes, requires clean committed MT source, snapshots the exact manifest, and runs every matrix row plus `cargo test --test test_mt108_argus_aggregate mt108_verify_argus_evidence_manifest` in a hidden bounded Cargo child.\n\nEvery child emits `hsk.native_gui.external_process_receipt@3` STARTED and COMPLETED rows joined by source SHA, scenario, correlation id, Cargo PID, test-executable PID/start identity, exact command, and deadline. COMPLETED is legal only after an identity-aware post-exit inventory proves zero owned survivors; timeout/nonzero/indeterminate cleanup emits BLOCKED, FAILED, RECLAIMED, or RECLAIM_FAILED and cannot close proof. Every screenshot emits `hsk.native_gui.screenshot_marker@4` with the same source SHA, scenario, correlation id, test PID, and exact preceding Argus action receipt id. Pixel closure requires CAPTURED PNGs of at least 320x180 inside the exact run directory. `-Headless` instead requires typed DEFERRED rows with no frame and reports `NOT pixel closure`; it proves AC-108-2 but cannot replace the real-GPU V3 capture run.\n\nThe verifier rejects missing/extra scenarios, dirty current source, mismatched source/process identities, unbound AccessKit surface ids, nonzero exits, surviving children, foreign screenshot rows, duplicate outcomes, weak frames, and any captured path outside the fresh run directory. Read the matrix snapshot, `canonical-argus-matrix.jsonl`, the legacy `hsk.native_gui.argus_surface_evidence@4` rows in `argus-seven-surface.jsonl`, `screenshot_marker.jsonl`, `external_process_receipts.jsonl`, frames, and per-process stdout/stderr logs. Use a new run id for every rerun; never reuse or merge artifacts.\n\nOriginal seven contract-surface command detail (the complete 33-row command set is derived from the manifest):\n{matrix}"
    );
    body.replace(
        "`hsk.native_gui.screenshot_marker@4` with the same source SHA, scenario, correlation id, test PID, and exact preceding Argus action receipt id.",
        "`hsk.native_gui.screenshot_marker@5` with the same source SHA, scenario, correlation id, test PID, and a process-local monotonic proof-event sequence. A post-action frame must follow the joined Argus receipt's terminal reinspection event; a pre-action frame cannot close scenario proof.",
    )
    .replace(
        "COMPLETED is legal only after an identity-aware post-exit inventory proves zero owned survivors;",
        "The runner-only `hsk.native_gui.process_observation_ack@1` handshake keeps a short-lived test executable alive until the external supervisor has captured its PID/start/executable identity. COMPLETED is legal only after an identity-aware post-exit inventory proves zero owned survivors;",
    )
}

fn purpose_body() -> String {
    "The native editors are Handshake's Notes pillar. They REPLACE the legacy React/Monaco/Excalidraw/\
graph surfaces (kept read-only under app/src as the parity reference) with native egui + AccessKit panes \
that share ONE selection, ONE clipboard, ONE command bus, and ONE undo scope — the WP-012 melt-together \
substrate built on command_registry.rs + event_bus.rs. The default work surface opens the CODE editor, the \
RICH-TEXT Notes editor, and Runtime Chat side by side; secondary panes include the Loom GRAPH view, the \
CANVAS board, and the knowledge surfaces (folder tree / backlinks / outgoing links / collections). Runtime \
Chat is input-ready but backend-blocked in this build: sending returns ChatSendError::EndpointMissing and \
does not append a fabricated assistant reply. Every pane is addressable by a stable AccessKit author_id and \
steerable by the MCP swarm tools. A swarm agent discovers controls with list_widgets, drives \
a button with click_widget{target:<author_id>} or a parameterized control with \
click_widget{target:<author_id>,payload:{...}}, types into a field with set_value{target,value}, and sees \
the pixels with screenshot — no screen-scraping and no keyboard simulation."
        .to_owned()
}

fn core_workflows_body() -> String {
    "Open a file in the code editor: select it in the project tree (left rail 'files' button \
left-rail.activity.files), then the file mounts in the code pane; save with editor.code.save (Ctrl+S). \
Open an existing knowledge note through the project tree or quick switcher: the shell opens a \
LoomWikiPage tab carrying the document id, performs GET /knowledge/documents/:id, and installs that \
backend content into the mounted editor.rich.text surface. Edit rich-text/knowledge notes by typing in the \
rich pane; toggle bold with editor.rich.format-bold (Ctrl+B), insert a block with \
editor.rich.insert-slash-command ('/'). A model creates a note through that same mounted action with \
click_widget{target:editor.rich.insert-slash-command,payload:{kind:note,title:<title>}}; direct model inserts also accept \
{kind:wikilink,ref_kind:<kind>,ref_value:<exact-id>,label:<label>} and \
{kind:code_block,language:<language>,code:<exact-code>}, without transient picker-row ids. Success for note creation appears as \
editor.rich.created-document with the real backend document id as its value. Then save with FILE > Save, Ctrl+S, or editor.rich.save. The \
save path is the MT-020 SaveManager backed by PUT /knowledge/documents/:id/save with the loaded \
doc_version; reopening the same note invalidates the mounted state and forces a fresh GET before the \
editor is considered current. To edit a slash-inserted note code block in the native Code Editor, activate \
that block's editor.rich.code-block.open.re-block-* Edit-code action, set editor.code.text, and dispatch \
editor.code.save; the exact block is persisted into the same note content_json through that SaveManager. \
Build a graph: pan with graph.pan-left/\
graph.pan-right, zoom with graph.zoom-in/graph.zoom-out, open a node with graph.open-node, connect blocks \
with graph.add-edge. Sketch on the canvas: add a card with canvas.add-card, place a Loom block with \
canvas.place-block, connect with canvas.add-edge. Drive FEMS: the relevant-memory-panel shows the \
retrieval capsule; propose a memory write with the fems-propose-dialog and confirm with \
fems-propose-confirm (NEVER an editor-direct commit). Move a selection between panes: select in code, \
copy (Ctrl+C, menu.edit.copy, or command-palette.option.hs-editor-menu-edit-copy), focus the rich pane, \
paste (Ctrl+V, menu.edit.paste, or command-palette.option.hs-editor-menu-edit-paste) — the shared \
clipboard + command/event bus carries the in-session payload. \
Jump from a knowledge backlink to its target: click a wikilink chip or an outgoing.resolved.* row. Open \
the command palette (Ctrl+Shift+P, command-palette.dialog) and run a command by typing into \
command-palette.search. Use Runtime Chat: read runtime-chat-status for the current EndpointMissing \
blocker, type into runtime-chat-input, then click runtime-chat-send; no assistant turn is generated \
until a real native HTTP chat route exists."
        .to_owned()
}

fn startup_and_run_body() -> String {
    "The editor panes mount inside the WP-011 shell as named tiles in the docking layout managed by \
split_layout.rs + pane_registry.rs + layout_persistence.rs (the layout persists per workspace). Run the \
native frontend from the crate directory src/frontend/handshake_native with:\n\
\n\
    cargo run --manifest-path src/frontend/handshake_native/Cargo.toml -p handshake-native\n\
\n\
The cargo package is 'handshake-native' and the binary target is also 'handshake-native' (verified \
against src/frontend/handshake_native/Cargo.toml [[bin]] name). For a swarm/headless session the MCP \
steering surface (mcp/server.rs) speaks the JSON-RPC tools list_widgets / click_widget / set_value / \
screenshot over the per-session token written into the binding file. A fresh MT-098 layout seeds pane-a \
as Code, pane-b as Notes, and pane-c as Runtime Chat; a stale two-pane persisted layout is rejected by the \
canonical-pane validator and falls back to this default. To open the manual itself, surface the manual-pane \
and type a keyword into manual-search."
        .to_owned()
}

fn inputs_and_outputs_body() -> String {
    "Inputs: a file path (code editor), a loom:// block reference (everything-is-a-block addressing, \
loom_address.rs), an atelier:// CKC ref dragged in from the atelier_side_panel, a graph node block id \
(graph.open-node), or a locus:// WP/MT reference. Outputs: edited buffers PERSISTED through the existing \
handshake_core APIs — PostgreSQL/EventLedger is the only durable authority, and the editors never write \
to a database directly; clipboard payloads on the shared clipboard; and command-bus / event-ledger \
events (event_bus.rs + the Flight Recorder) that record each editor action. A rich-text document saves \
to the knowledge-documents route family: GET /knowledge/documents/:id loads content_json/doc_version, \
PUT /knowledge/documents/:id/save writes {expected_version, content_json}, GET/PUT/DELETE \
/knowledge/documents/:id/draft owns crash recovery, and reopening a note re-GETs the authoritative \
document instead of trusting a cached mounted editor. The code editor saves the buffer through the same \
backend client. Nothing the editors emit bypasses handshake_core."
        .to_owned()
        + " Runtime Chat input is local UI state only in this build; a send probes the planned native chat route \
and returns EndpointMissing because no assistant chat HTTP endpoint is present."
}

fn navigation_paths_body() -> String {
    "Keyboard + AccessKit navigation between panes: Tab/Shift+Tab moves focus across the live AccessKit \
tree; an agent moves focus with click_widget (Focus is a declared action on every control). The command \
palette (Ctrl+Shift+P) is command_palette.rs + command_registry.rs — its container is \
command-palette.dialog, its input is command-palette.search, its list is command-palette.list. The \
quick-switcher (quick-switcher.dialog / quick-switcher.search) jumps between open docs/blocks/symbols. \
The manual search box (manual-search) filters topics by keyword. Backlink / graph jump: click a wikilink \
chip, an outgoing.resolved.* / outgoing.unresolved.* row, or a graph node (graph.open-node) to navigate \
to the target document/block. The bottom status bar exposes the VS-Code-class editor segments \
status-bar-language-mode / status-bar-eol / status-bar-indent / status-bar-encoding / \
status-bar-render-whitespace."
        .to_owned()
}

fn safety_constraints_body() -> String {
    "The editors NEVER write to .GOV/** (it is a live governance junction). They NEVER touch the legacy \
app/src/** React surface except as a read-only parity reference. ALL persistence goes through \
handshake_core — PostgreSQL/EventLedger only; there are no direct database writes from the editors. \
Destructive actions are bounded and QUIET (HBR-QUIET, quiet_mode/focus_guard.rs): no focus-stealing \
popup appears while a swarm agent is driving, no window grabs the keyboard, and background work does not \
steal OS focus. FEMS memory writes are ALWAYS review-gated proposals (fems-propose-dialog -> \
fems-propose-confirm), never an editor-direct commit. Approval uses a separate governed commit route; \
rejection performs no commit."
        .to_owned()
}

fn common_failure_modes_body() -> String {
    "A pane fails to mount (the docking layout could not place the tile, or the host-mount carry MT-080 is \
not yet live). The OS clipboard daemon can be absent on a headless CI runner, so the external system \
clipboard mirror may not update; same-session editor copy/paste still uses the InteractionBus clipboard \
cache and is not a no-op for mounted editor panes. A pane_id is stale after the pane was closed, so a \
stored swarm reference points at a node that is gone (deletion is signalled by ABSENCE from the AccessKit \
tree, not a tombstone). An AccessKit node is not found by an agent because its backing widget is not \
rendered this frame (a transient control like find-next while the find panel is closed is marked \
present=false and suppressed). The backend persistence API returns a typed error (e.g. a \
knowledge-document save conflict, or a temporarily unavailable FEMS/Stage/Calendar/Locus route). \
Runtime Chat send also returns EndpointMissing in this build; this is the \
expected typed blocker, not a spinner or silent failure."
        .to_owned()
}

fn recovery_steps_body() -> String {
    "Re-mount the pane from the docking menu (top_menu_bar.rs view menu / pane_registry.rs), or reset the \
layout from Settings. Re-run with a present display + clipboard (a GPU/clipboard host) when a headless \
runner lacked them. Re-query the live AccessKit registry with list_widgets to get the CURRENT author_id \
for a node after a layout change — never reuse a stale id; the canonical id source is \
accessibility/registry.rs + the live editor/knowledge action registries. For a note that appears stale or \
unusable, reopen its document tab through the project tree/quick switcher; the shell invalidates the \
mounted rich state and issues a fresh GET /knowledge/documents/:id before rebinding SaveManager/DraftManager \
to that id. Retry persistence after the typed backend error clears (a save conflict resolves once the newer \
    revision is loaded). A Stage route or embed-back failure remains visible at stage-route-status or \
stage-embed-back-status; restore the endpoint and use stage-route-retry for the retained route. If the \
document saved but EventLedger acknowledgement failed, the HsLink is already saved and the stable \
stage-embed-back-status exposes LedgerPending with the exact event id. stage-capture-embed-back is then \
relabelled Retry exact EventLedger receipt and replays the same immutable event_id; use that action instead \
of starting a new capture or minting a new receipt. It does not insert another hsLink. Retry the full \
capture/embed action only for failures that happened before the document save. A failed \
CalendarEvent and ActivitySpan reads retry transport, 408, 425, 429, and 5xx failures at most three times; \
404 and other terminal 4xx responses are not retried. While the mounted JournalStore performs the one \
open/create PUT for a date, Calendar shows Waiting for daily note; a journal failure points to the editor's \
typed daily-note error and is never relabelled as a Calendar endpoint failure. Retry exhaustion, invalid \
responses, and endpoint absence remain distinct. A failed ActivitySpan fetch preserves the resolved event \
chip and daily-note binding, marks only the activity strip with its typed failure, and suppresses correlation \
receipts until a fresh successful load. Date/workspace changes cancel retry backoff and queued old-generation \
receipts; retries reuse one immutable event envelope and same-id queued retries coalesce, while a fresh \
successful load records a fresh receipt. A Locus record-not-found stays an unresolved chip, while a \
route failure is a distinct typed unavailable state; restore the route and reload the document. Runtime Chat \
assistant generation remains EndpointMissing in this build and must not be reported as a completed reply."
        .to_owned()
}

fn interop_edges_body() -> String {
    "The native editors melt together with CKC/Loom and four named pillars. Open the operator-facing CKC \
drag source from VIEW > Toggle Atelier / CKC Panel (menu.view.toggle-atelier). The mounted \
atelier-side-panel loads real GET /atelier/intake/batches and GET /atelier/command-corpus data; expand an \
atelier-batch-* row to load GET /atelier/intake/batches/{batch_id}/items, then drag an atelier-item-* row. \
Dropping on editor.rich.text inserts the existing hsLink atom (refKind=media, refValue=item UUID), which \
survives PUT /knowledge/documents/:id/save and a fresh GET, including when the note starts as an empty \
paragraph with no text leaf. AccessKit 0.21.1 has no StartDrag action: Click on the atelier-item-* ListItem \
inserts that exact item into the active rich editor, while atelier-item-insert-* and atelier-item-canvas-* \
remain explicit insert/place controls and pointer users retain typed drag-and-drop. Canvas \
placement requires an already-published Loom file block carrying a real document_id or asset_id. A raw \
intake row with no durable relation shows a typed unsupported blocker; the editor never creates an empty \
synthetic file block. Placement retries reconcile against a fresh board and the backend one-block-per-canvas \
uniqueness constraint before reporting success or registering undo. Async placement completion is keyed by \
workspace_id plus canvas_id: a late success/failure for board A cannot reload or paint board B, and returning \
to A restores A's retained error until a successful A retry clears it. The panel displays source_path as \
metadata only; it does not load or claim a thumbnail/media preview. The current Atelier backend exposes no character-list or moodboard-list route, so \
atelier-character-list-blocker and atelier-moodboard-list-blocker are visible typed blockers rather than \
fabricated rows. A failed items request shows Items unavailable plus atelier-items-retry-*; it is never \
reported as '(no items)'. Item, batch, and corpus AccessKit ids use injective UTF-8 hex suffixes, so copy the \
live id from list_widgets instead of deriving a lossy slug. \
From the mounted Code Editor, place the caret on a tree-sitter identifier or select exactly that identifier, \
then choose the context-menu action Copy as note reference; \
the mounted pane writes [[code:path#symbol]] to the shared InteractionBus clipboard. Activate the Rich \
Text Editor and Paste (Ctrl+V): a complete canonical code reference replaces the active rich selection \
with the existing persisted hsLink atom (refKind=code, refValue=path#symbol) in one undo transaction; \
Ctrl+Z restores the exact pre-paste document. Empty or malformed code targets, non-reference tokens, and \
mixed clipboard text keep ordinary lossless plain-text paste behavior and replace an active same-block or \
sibling cross-block selection in the same single undo transaction. A structurally nested cross-container \
selection fails closed without changing the document; collapse it or select within sibling text blocks before \
retrying Paste. Arbitrary selected prose is never emitted as a \
symbol reference. If no exact identifier or canonical parser-encodable file path is available, the menu \
action is disabled; save the buffer to establish its path. \
If Paste produces no change, re-focus editor.rich.text, confirm the shared clipboard through \
Argus diagnostics, and retry. The paste is local editor state until the normal Save action returns its \
SaveManager/EventLedger receipt. \
For Stage, EDITORS > View: Stage (menu.editors.stage) opens or focuses the one docked stage-pane Role::GenericContainer; \
EDITORS > Route selection to Stage (menu.editors.route-to-stage) sends the active rich selection or document \
through the same interop.route-to-stage command used by the palette and context menu. Right-click editor.rich.text \
and choose Route to Stage (rich-editor.route-to-stage): a same-block or cross-block selection \
routes selected text, while no selection routes the whole active document. The same interop.route-to-stage \
command is available in the Command Palette. A Canvas node exposes the live AccessKit menu target \
ctx-menu.ctxmenu-node-route-to-stage only when the clicked node has a stable id and its mounted board has \
the matching workspace + canvas projection confirmed by a completed board load; pending, failed, rebound, \
or stale projections keep Route to Stage visible but disabled with a reason. \
Graph-view nodes do not carry a live Canvas board route, so their Route to Stage entry is always disabled. \
Use argus.inspect to read the current disabled state before argus.click; never infer availability from the \
node id alone. These paths \
use the shared InteractionBus, open stage-pane, and expose the payload at stage-routed-content. If the bus is \
busy, stage-route-status remains visible and stage-route-retry / rich-editor-stage-route-retry retries the \
retained exact request with the same causal action id. In stage-pane, activate stage-capture-embed-back to \
run the live privileged create -> exact-byte descriptor/content retrieval -> SHA-256 verification -> note \
or Canvas embed workflow; EDITORS > Capture and embed from Stage \
(menu.editors.embed-stage-capture) invokes the same command. Stage authenticates create and both reads with \
the running native app's owner-restricted MCP session token; the backend derives actor/capability/approval \
identity from that binding and never trusts caller-supplied privilege headers. The capture is idempotent, bounded to 16 KiB, and returns stable artifact, Job History, \
EventLedger, manifest, correlation, and digest ids; stage-embed-back-status exposes the exact success or \
typed failure, including busy and runtime-unavailable outcomes that dispatch no request. No Stage-specific \
persisted setting exists or is required: Settings exposes the read-only \
settings-editor-atelier-ckc-stage-posture row, while Atelier/CKC visibility and Stage routing remain live \
VIEW/EDITORS commands that follow the active workspace, note/Canvas target, and authenticated native \
session. With no active document, stage-route-status visibly reports the typed failure; a \
failed CKC insertion similarly appears at rich-editor-interop-status. \
Verification and diagnostics: discover atelier-side-panel / atelier-batch-* / atelier-item-* / \
atelier-item-insert-* / atelier-item-canvas-* / \
atelier-corpus-* / editor.rich.text / stage-pane with argus.inspect. The canonical MT-033 sequence is \
argus.inspect -> argus.click menu-view -> fresh argus.inspect -> argus.click menu.view.toggle-atelier -> \
fresh argus.inspect of atelier-side-panel. Copy the exact dynamic atelier-item-* id from that tree, activate \
that exact row or its atelier-item-insert-* control, retain the attributed action receipt, save, and verify \
the persisted hsLink through a fresh knowledge-document GET. Then argus.click menu-editors and \
menu.editors.route-to-stage (or rich-editor.route-to-stage), retain each attributed receipt, and perform a \
fresh argus.inspect of stage-pane, stage-routed-content, and stage-route-status. After that terminal \
inspection, capture the same mounted WGPU frame with the test harness renderer; the canonical Argus \
screenshot callback is intentionally unavailable in this headless binding. \
Repeat from a shell with no active rich document through menu-operator -> \
menu.operator.command-palette -> command-palette.option.hs-stage-palette-route: that always-enabled \
canonical route command must expose the typed `activate a saved rich document first` stage-route-status \
failure and no routed-content success. An immediate action receipt or stale tree is not terminal proof. \
Store every action's attributed receipt plus observation.before, observation.after, verified terminal tree, \
and the post-terminal PNG outside the worktree in a fresh unique \
HANDSHAKE_ARTIFACTS_ROOT/handshake-test/wp-kernel-012-mt-033/canonical-argus-v4/run-<uuid>/ directory. \
The success branch must contain exactly five Applied receipts. The typed-unavailable branch must contain \
exactly two Applied receipts followed by one typed Rejected receipt carrying `activate a saved rich document \
first`; all eight receipts must be terminal with zero Indeterminate outcomes. The run manifest binds the \
before/after whole-worktree candidate identity (HEAD plus binary tracked diff and sorted untracked file \
content hashes), asserts that identity did not change during the run, and records the canonical path and \
SHA-256 of the exact running test executable. When \
HANDSHAKE_ARTIFACTS_ROOT is unset, the proof derives the one sibling Handshake_Artifacts root from \
CARGO_MANIFEST_DIR; when set, it must equal that derived root. It never accepts process-CWD or an alternate \
absolute artifact root, so crate-root and repo-root invocations converge on the same location. From \
src/frontend/handshake_native, run the focused proof as `cargo test \
-p handshake-native --test test_ckc_embed -- --nocapture` with CARGO_TARGET_DIR set to the standardized \
outside-repo artifacts folder; run the canonical GPU/Argus proof as `cargo test -p handshake-native \
--features wgpu_screenshots --test test_ckc_embed atelier_panel_screenshot -- --nocapture \
--test-threads=1`; run its managed PostgreSQL proof as `cargo test -p handshake-native --features \
integration --test test_ckc_embed -- --nocapture --test-threads=1`. From repo root, add `--manifest-path \
src/frontend/handshake_native/Cargo.toml` to either command. The integration-gated cases self-seed PostgreSQL, require nonzero \
batches/items/corpus, use the real pointer drag source, save/reload the hsLink with a fresh client, read the \
exact typed `KE-UUID` `KNOWLEDGE_RICH_DOCUMENT_SAVED` EventLedger receipt and exact route_to_stage \
Flight Recorder/EventLedger receipt, and clean their workspace, Atelier, save-receipt, and exact \
native-editor receipt rows. Backend-down, \
malformed response, projection failure, placement failure, and insertion failure remain visible with \
Retry/reload guidance; never infer success from a spinner or old row. \
HBR-INT-009 posture for MT-033: Tier 1 Flight Recorder/EventLedger is NOT_APPLICABLE-with-reason for \
read-only Atelier batch/item/corpus GETs and WIRED for CKC embed/save, Canvas placement, route_to_stage, \
and stage_embed_back mutations. Tier 2 internal_diagnostics is WIRED for Atelier HTTP work through the \
shared bounded BackendCall watchdog and for shared heartbeat/frame/resource/backend-health state; the local \
in-process Route-to-Stage bus hop is NOT_APPLICABLE-with-reason because it performs no blocking backend \
operation. Tier 3 Palmistry is WIRED because the same typed watchdog event reaches the shared diagnostic \
ring and the app-wide watcher covers process freeze/crash state; no project content or dynamic item identity \
is written to that ring. \
Each additional edge has a live editor-side AccessKit surface and bound backend route an agent drives today.\n\
\n\
- FEMS (Pillar 12, typed memory): the relevant-memory-panel renders the retrieval capsule \
(relevant-memory-list); an agent reads it with list_widgets and screenshot. A review-gated memory-write \
proposal is opened through command-palette.option.hs-fems-palette-propose-to-memory, reviewed at \
fems-propose-dialog, cancelled at fems-propose-cancel, or confirmed at fems-propose-confirm. Read \
editor.fems.memorypack-status and proposal fems-propose-status values for structured outcomes and IDs. \
After durable proposal acceptance, click fems-review-approve or fems-review-reject; read \
fems-review-status for the exact decision and durable EventLedger/Flight Recorder receipt identities. \
Approval then calls the separate explicit commit route and publishes the committed item plus strict \
MemoryPack; rejection performs no commit. If \
the canonical pending queue cannot be loaded, click fems-review-refresh-retry; creating another proposal \
remains blocked until that queue is known.\n\
- Stage (Pillar 17): selection/document/Canvas-node content is routed over the shared bus to stage-pane \
(stage-routed-content); stage-route-status carries failures and stage-route-retry retries contention without \
changing causal attribution. The agent activates the live create/retrieve/embed workflow with \
stage-capture-embed-back (argus.click); stage-embed-back-status exposes the exact artifact id, verified \
SHA-256 provenance, target, or typed failure. When it reports LedgerPending, the HsLink is already saved: \
activate the relabelled Retry exact EventLedger receipt action instead of starting a new capture or minting \
a new receipt; that action replays the same immutable event_id and does not insert another hsLink. Capture writes are visible in Job History, EventLedger, and \
Flight Recorder; retrieval verifies the dedicated content bytes before embedding. \
A missing or unimplemented artifact route is a typed endpoint-absent result, never an embed success. \
HBR-INT-009 diagnostic posture: Flight Recorder/EventLedger = WIRED because route_to_stage and \
stage_embed_back persist immutable causal receipts. Shared internal_diagnostics = WIRED for heartbeat, \
frame/resource, backend-health, and the bounded BackendCall watchdog used by Atelier loads; the local Stage \
route bus hop is NOT_APPLICABLE-with-reason because it performs no blocking backend operation, while deeper \
Stage capture instrumentation remains owned by MT-066. Shared Palmistry = WIRED for process freeze/crash \
observation and the same typed diagnostic ring; the MT-033 local route carries no Stage-specific process \
child or payload, so a separate route tracker is NOT_APPLICABLE-with-reason. \
After each asynchronous retry or capture, use a fresh argus.inspect instead of treating the immediate \
action receipt as terminal state.\n\
- Calendar (Pillar 2): the daily-journal-panel binds the mounted JournalStore's single selected-date \
  open/create result to a CalendarEvent (daily-journal-date-header, daily-journal-calendar-event-chip) and \
  shows a read-only activity strip (daily-journal-activity-strip). Waiting for the daily note, daily-note \
  failure, Calendar loading, successful no-event, event success, endpoint absence, retry exhaustion, and \
  invalid-response states are distinct. Calendar and ActivitySpan reads retry only transient transport/ \
  408/425/429/5xx failures, at most three attempts. An ActivitySpan failure keeps the event chip and \
  selected-date daily-note binding usable, switches only the activity strip to its typed failure, and \
  suppresses correlation success receipts; a failed fetch is never rendered as an authoritative zero-span \
  result. Date/workspace navigation cancels stale deliveries and queued receipts; transport retries reuse \
  the same immutable event id/timestamp and duplicate queued copies of that exact receipt coalesce. Each \
  successful selected-date journal binding emits at most one accepted CalendarEvent/activity receipt set; \
  a multi-day event can therefore emit one distinct set for each date because its bound daily-note document \
  differs. HBR-INT-009 diagnostic posture for Calendar: Flight Recorder/EventLedger = WIRED through \
  calendar_event_bound and activity_span_correlated; internal_diagnostics = DEFERRED-with-reason because \
  the generic backend-health surface has no Calendar-specific diagnostic row; Palmistry = \
  DEFERRED-with-reason because the global process watcher has no Calendar-scoped tracker or recovery proof.\n\
- Locus (Pillar 6): a locus:// WP/MT reference renders as an inline locus-ref chip in the rich editor, and \
the outgoing-links pane (outgoing.panel) lists resolved (outgoing.section.resolved) and unresolved \
(outgoing.section.unresolved) references. Bound WP and MT reads resolve through the shared navigation seam; \
record-not-found remains a grey unresolved chip, while a missing route is the distinct typed unavailable state \
and never fabricates a record. Persisted locus refs, including original-case WP/MT identities, survive a \
backend restart and rich-document reload; reverse lookup is read-only and returns the exact persisted \
document once per ref. For canonical operation, use a fresh argus.inspect to discover \
locus-ref-chip-wp-{WP_ID} or locus-ref-chip-mt-{MT_ID}, call argus.click on that exact stable target, then \
use a second fresh argus.inspect. The immediate action receipt may be indeterminate; navigation is proven \
only when the fresh inspection carries the same receipt/agent attribution and the mounted navigator focuses \
WP:{WP_ID} or MT::{MT_ID}. For a grey unresolved chip, inspect it but do not infer that the route is absent: \
querying the live ref distinguishes record NotFound from LocusReadApiUnavailable. Restore an unavailable \
route and reload the document; repair or remove a genuinely stale record id instead of retrying it as a \
transport failure. HBR-INT-009 diagnostic posture for Locus: Flight Recorder/EventLedger = \
WIRED through the structured locus_ref_resolved event after successful forward resolution and the \
locus_reverse_lookup event when persisted referencing documents are found. Inspect those events by \
workspace and locus_uri to confirm the read sequence; failed resolution emits no fabricated success event, \
so restore the route or record and repeat the canonical click/re-observation flow. The knowledge-document \
save remains a separate operation with its existing EventLedger receipt. \
internal_diagnostics = DEFERRED-with-reason because the current generic backend-health surface has no \
Locus-specific diagnostic row. Palmistry = DEFERRED-with-reason because the global process watcher has no \
Locus-scoped tracker or recovery proof. These diagnostic deferrals do not weaken the operator-visible typed \
NotFound/unavailable states or the canonical Argus receipt and re-observation gate. An agent drives all of \
these with argus.click / argus.inspect.\n\
\n\
MT-074 aggregate proof matrix: from src/frontend/handshake_native run \
`$env:CARGO_TARGET_DIR='..\\..\\..\\..\\Handshake_Artifacts\\handshake-cargo-target\\mt074-v3'; cargo test \
-p handshake-native --test test_other_pillar_interop_proofs other_pillar_op -- --nocapture --test-threads=1`. \
Every scenario uses the canonical Argus sequence argus.inspect -> argus.click -> attributed action receipt \
-> fresh argus.inspect. OP01 drives menu-editors, menu.editors.route-to-stage, \
stage-capture-embed-back, and editor.rich.save, then proves route_to_stage and stage_embed_back have the \
exact same causal_action_id. OP02 drives daily-journal-calendar-event-chip and \
calendar-event-tab-activity and proves calendar_event_bound plus activity_span_correlated. OP03 drives \
locus-ref-chip-wp-{id} and proves locus_ref_resolved plus locus_reverse_lookup. OP04 repeats the three \
surfaces against real localhost Argus servers and proves zero residual Argus leases after cleanup. Durable \
proofs are written outside the worktree under \
../../../../Handshake_Artifacts/handshake-test/wp-kernel-012-mt-074/canonical-argus/<scenario>/run-*/ as \
<scenario>-canonical-argus.json plus non-empty PNG screenshots and before/action/after accessibility trees. The \
current flush mechanism is the ActionChannel raw_input_hook drain plus bounded Harness::run_steps; there is \
no separate flush_pending_updates API. Direct PostgreSQL rows in this proof are fixture-only setup and \
cleanup boundaries; user interactions and readback use the production Stage, Calendar, Locus, knowledge, \
EventLedger, and Argus routes. A typed failure remains visible and must be recovered through the owning \
edge's documented retry/reload path; never infer success from a stale tree or immediate indeterminate \
receipt. Run serially so process-wide Argus binding and environment leases remain attributable."
        .to_owned()
}

fn large_documents_interconnection_body() -> String {
    "Large-document handling is built into the native editors; there is no performance-mode switch and no \
hidden debug budget multiplier. The code editor virtualizes a 10,000-line buffer, minimap rows cover all \
10,000 lines, Find/Replace runs on the native buffer, and multi-cursor edits use the mounted \
CodeEditorPanel. The rich editor parses persisted content_json into the native DocModel, renders and scrolls \
1,000 blocks through editor.rich.text, finds through FindReplaceState, saves with an exact \
KNOWLEDGE_RICH_DOCUMENT_SAVED EventLedger receipt, and detects both a live 50-hop transclusion chain and a \
persisted cyclic-5 as cycle_detected. Knowledge proofs exercise a 1,000-node/~2,000-edge LoomGraphView pass, \
5,000-block tags/search, and a 10-level 200-folder/1,000-child tree. RSS is the hard worst-of-three process \
delta, so allocator reuse cannot hide the cold-load cost; an \
unavailable RSS sample or receipt write fails validation instead of recording zero or PASS. Checked-in \
perf_manifest.json is the contract-authoritative runtime-updated projection, and the current external \
receipt plus immutable run summary bind its result to one committed source SHA and the exact release test \
binaries. A canonical PASS forbids PERF_BUDGET overrides and requires all 20 scenarios from one run id. \
Operator controls remain the normal Editor settings: settings-editor-font-size, settings-editor-word-wrap, \
settings-editor-wrap-column, settings-editor-minimap, and settings-editor-sticky-scroll. These affect the \
mounted editors immediately; they do not widen validation budgets. Open the operator-facing VIEW menu and \
use menu.view.open-code-editor, menu.view.open-rich-note, menu.view.open-knowledge-graph, \
menu.view.open-canvas, menu.view.open-loom-search, or menu.view.open-find-in-files. EDIT > Quick Open \
(menu.edit.quick-switcher) opens the same QuickSwitcher graph-search used for note and file hits. \
The interconnection paths use shipped surfaces, not parallel copies: CKC/Atelier DragPayload becomes a rich \
hsLink insertion transaction or a LoomCanvasBoard placement; a code hsLink dispatches open-code-symbol and \
the code editor focuses the definition selected by the shipped code-nav resolver; one product Find bus \
dispatch fans out to mounted native code FindState and rich \
FindReplaceState with the same query; LoomSearchV2 facets include note and file; the exact persisted Loom \
graph projection is laid out by LoomGraphView; QuickSwitcher maps typed LoomGraphSearchHit rows to real \
navigation targets; diagnostic related-note chips in the code gutter open and focus the exact rich-note \
destination; and InteractionBus undo restores native RichEditorState/CodeEditorPanel snapshots per focused \
pane through the canonical Ctrl+Z key-command route. Rich undo proof saves EDIT_A, sends Ctrl+Z to the \
AccessKit-focused rich surface, saves the restored snapshot, and \
confirms absence with a backend GET. Save and cross-surface receipts are PostgreSQL/EventLedger authority, \
never a cached widget. \
For MT-045 validation, run tests/run_mt045_perf_proof.ps1 from src/frontend/handshake_native. The \
source-controlled supervisor uses the existing sibling Handshake_Artifacts/handshake-cargo-target, builds \
the exact committed product backend, requires the existing internal PostgreSQL authority, and invokes \
test_perf_large_code, test_perf_large_rich, and test_perf_large_knowledge serially in Cargo release mode. \
For broader WP-012 validation, run those focused binaries and the \
four test_interconnect_* binaries from src/frontend/handshake_native. The `perf_proof` test-name filter runs \
the performance proof suite; `perf_lr05_transclusion_chain` selects separate linear and cycle-detected LR05 \
paths. IC-13 skips only when SKIP_AI_TESTS is exactly 1; when unset or any other value it runs the real AI + \
PostgreSQL suggestion/accept/backlink path and fails closed if the configured model is unavailable. Managed tests attach to HSK_TEST_BASE \
or start the already-built HSK_TEST_BACKEND_BIN, create one owned workspace, and never stop an attached \
backend. Current run receipts live outside the repo under Handshake_Artifacts/wp-kernel-012/mt-045/measurements \
and Handshake_Artifacts/wp-kernel-012/mt-046/measurements. Each rerun writes RUNNING with a unique attempt id \
before any skip gate, health/setup work, or assertion, then terminal PASS/SKIPPED or FAIL; an exact skip is \
terminal SKIPPED, while panic/drop records FAIL. This current receipt supersedes an earlier verdict, so a \
stale PASS cannot survive a failed rerun. Large fixtures traverse production HTTP mutation routes against \
real PostgreSQL; fixture setup is outside measured query time and has a hard 1,200-second ceiling \
(HSK_PROOF_SETUP_TIMEOUT_SECS may lower it). Owned product backends are killed and reaped on timeout, but the \
existing internal PostgreSQL process is never stopped. PASS is written only after workspace/process/temp-dir \
cleanup assertions succeed. Recovery: a budget miss, missing EventLedger receipt, unavailable RSS sample, \
fixture timeout, malformed native parse, unresolved drag payload, missing graph root, or absent search hit is \
a failing proof. Fix the product/backend cause and rerun the exact scenario; do not edit the catalog or \
manufacture a PASS receipt."
        .to_owned()
}

fn notes_worksurface_and_chat_body() -> String {
    "The default WP-KERNEL-012 worksurface is editor-first and minimal: pane-a is the Code editor, \
pane-b is the Notes rich editor (LoomWikiPage / loom.wikipage class), and pane-c is Runtime Chat beside \
the editors. The manual and diagnostics are not docked into this default worksurface. A model discovers \
the current panes with list_widgets and addresses the seeded panes by pane-a / pane-b / pane-c, then uses \
the stable widget ids inside them: editor.code.* for code actions, editor.rich.* for Notes actions, \
runtime-chat-panel for the chat pane container, runtime-chat-status for the current chat route state, \
runtime-chat-input for the draft, runtime-chat-send for the send button, and runtime-chat-cancel for the \
Cancel control shown while a request is active. Cancelling aborts the exact active request generation, \
changes runtime-chat-status to Cancelled, ignores any late completion from that generation, and leaves the \
input ready for a new send. Runtime Chat is honest in this build: no native HTTP assistant-chat endpoint \
exists, so a send probes the planned route and returns EndpointMissing instead of fabricating an assistant \
reply. Keep the main screen quiet and work-focused; advanced diagnostics stay behind Settings -> Diagnostics."
        .to_owned()
}

fn opening_editing_saving_notes_body() -> String {
    "Open an existing note from the project tree, quick switcher, a wikilink, or a graph/outgoing-link row. \
The shell opens a LoomWikiPage tab with the document id, performs GET /knowledge/documents/:id, parses \
content_json into the rich-editor document model, and binds SaveManager / DraftManager to that id and \
doc_version. Editing is live in the rich editor: type into the Notes pane, use editor.rich.format-bold, \
editor.rich.format-italic, editor.rich.insert-slash-command, wikilinks, backlinks, and properties exactly \
like the Obsidian-class note surface. The Properties header is default-collapsed above the note body; expand \
properties-header to reach properties-title (POST /knowledge/documents/:id/rename), \
properties-project-ref and properties-folder-ref (POST /knowledge/documents/:id/move), properties-doc-id \
click-to-copy, properties-tags, and the visible 'Editor chip tags are local-only' banner for \
the note-body chip editor path; persisted Loom tag hubs are covered by the Tags and Tag Hubs topic. Save through Ctrl+S, FILE > Save, or editor.rich.save. The authoritative \
save route is PUT /knowledge/documents/:id/save with expected_version and content_json; drafts use \
GET/PUT/DELETE /knowledge/documents/:id/draft for crash recovery. Reopening the same note invalidates stale \
mounted state and issues a fresh GET, so a no-context model should trust the reopened document and the \
EventLedger receipt, not an old widget value or cached editor state. If that GET or its document payload \
fails, the exact document/generation failure stays visibly latched at notes-document-load-error and the \
shell does not spin an automatic GET/repaint retry loop. Read the error with list_widgets, restore the \
backend or document payload, then click notes-document-load-retry; that explicit Retry issues one new GET \
for the still-active document, and another retry requires another explicit click."
        .to_owned()
}

fn terminal_launch_body() -> String {
    "Terminal launch is documented as an honest typed blocker in this native frontend build. The top-menu \
Run item menu.run.terminal is visible as 'Open Terminal in Workspace Folder' and is clickable. Selecting it \
does not fabricate a PTY: it records terminal-launch-status with 'EndpointMissing: native terminal launch \
needs HTTP /terminal/sessions' because current PTY reach is Tauri IPC-only. The backend PTY runtime exists in \
handshake_core terminal/** and its TerminalRequest carries cwd plus command/args for the shell wrapper, but \
native Handshake currently has no reachable HTTP /terminal/sessions route and no native terminal client; the \
typed native reach is EndpointMissing / IPC-only, with Tauri IPC kernel_terminal_create_session as the \
existing working reach in the legacy app path. The command palette exposes the same runnable Terminal: Open \
in Workspace Folder row as terminal.open-workspace and lands in the same terminal-launch-status blocker. A \
model should click menu.run.terminal or run terminal.open-workspace, then read terminal-launch-status. Do not \
claim a terminal opened, do not expect fake terminal output, and do not synthesize a cwd. The correct future \
behavior is Terminal: Open in Workspace Folder issuing a real spawn in the repo folder through a native HTTP \
route or bridge, using the configured platform wrapper such as pwsh/cmd on Windows."
        .to_owned()
}

fn model_session_launch_body() -> String {
    "Model/session launch is a compact native dialog, not a worksurface pane. Open it from Run -> Launch \
Model Session in Workspace Folder (author_id menu.run.model-session-launch) or the command palette row \
command-palette.option.hs-model-session-palette-launch-workspace (command id \
model-session.launch-workspace). The dialog exposes provider, workspace folder, model, wrapper, Launch, \
Cancel, and inline status through model-session-launch.dialog, model-session-launch.provider, \
model-session-launch.provider.local, model-session-launch.provider.cloud, \
model-session-launch.folder, model-session-launch.model, model-session-launch.wrapper, \
model-session-launch.start, model-session-launch.cancel, and model-session-launch.inline-status. \
Submitting issues a real POST /jobs body with job_kind=model_run, protocol_id=protocol-default, no doc_id \
for a folder-only launch, and job_inputs carrying session_id, workspace_id, workspace_folder, working_dir, \
model_provider, model_id, backend, and wrapper. MT-101 remediation: operator launches omit wp_id, mt_id, \
prompt, and simulate_duration_ms; governed-work attribution is present only when a governed launch surface \
sets it explicitly. The launch is a promptless session bootstrap; operator messages must arrive through a \
real follow-up message path, not a canned prompt in job_inputs. The same session_id is preserved in \
model-session-launch-status for state recovery. That proves job creation plus durable model-session \
correlation; live local or cloud execution remains NEEDS_MANAGED_RESOURCE_PROOF until a managed \
handshake_core returns runtime state. \
The direct repo-folder-bound session spawn with wrapper remains IPC-only via kernel_swarm_spawn_session in \
app/src-tauri/src/commands/swarm_runtime.rs, and local GGUF load remains IPC-only via kernel_model_runtime_load \
in app/src-tauri/src/commands/model_runtime.rs. For Local provider launches, model-session-launch-status must \
therefore include LocalModelLoadEndpointMissing kernel_model_runtime_load unless a real managed backend has \
returned loaded/running local-model proof through a native route. The native frontend surfaces these blockers \
rather than fabricating a running model, local GGUF load, or cloud run result. A model must not fabricate a \
session id as proof of runtime state; the request session_id is correlation only until a managed backend returns \
runtime proof. A model must not claim 'model running' state unless a managed backend returns that runtime proof. \
Inference Lab remains available at menu.run.inference-lab \
for broader model inspection, but it is not the MT-101 launch path. Settings -> Model Session exposes \
settings.model-session.open-launch, which closes Settings and opens this same one-shot dialog; the initial \
provider and wrapper shown there are launch-dialog seeds, not persistent hidden model defaults."
        .to_owned()
}

fn settings_diagnostics_body() -> String {
    "Diagnostics live in Settings -> Diagnostics, not in the notes+chat worksurface. Open Settings from \
Help -> Open Settings, command palette settings.open, or the settings chrome, then search for diagnostics \
with settings.search and expand settings.section.diagnostics. The Diagnostics panel itself is the \
diagnostics_panel AccessKit region with child groups diagnostics_heartbeat, diagnostics_frame, \
diagnostics_resource, diagnostics_events, and diagnostics_palmistry. It is a read-only projection over \
internal_diagnostics state: heartbeat, frame-time, resource/GPU, last-N diagnostic events, ring-writer \
status, and Tier-3 Palmistry survivor records for freeze, crash, and child-process stall. The section \
changes no settings and owns no durable state. If a model is debugging a UI freeze, crash, child hang, \
backend-down condition, or slow frame, it should first open \
Settings -> Diagnostics and read the appropriate group instead of looking for a diagnostics pane in the \
main worksurface."
        .to_owned()
}

fn visual_debugger_body() -> String {
    "The Visual Debugger is the MT-102 Worksurface Inspector inside Settings -> Diagnostics. Use \
click_widget on settings.diagnostics.worksurface-inspector.dump to write a JSON artifact outside the repo. \
The dump schema is hsk.native_worksurface_inspector@1 and includes pane_tree, widget_inventory, layout_tree, \
screenshot evidence, and an internal_diagnostics event summary. The status row \
settings.diagnostics.worksurface-inspector.status reports the last dump filename/size. Screenshot capture is \
best-effort in headless GPU environments: the JSON still records screenshot_deferred_headless_gpu when \
pixel readback is unavailable, so a model should rely on the pane tree and widget inventory rather than \
pretending a missing screenshot is visual proof. Use this tool when the model needs to inspect mounted panes, \
author_ids, layout state, or whether the worksurface matches the expected minimal notes+chat design."
        .to_owned()
}

fn foreground_safe_navigation_body() -> String {
    "Foreground-safe navigation is the MT-103 path for model-driven GUI work without stealing the operator's \
mouse, keyboard, or foreground window. A model discovers controls with list_widgets, resolves stable \
author_id targets, then drives each step through NavigationSequence::dispatch_step: open a pane by clicking \
a known quick-link/menu id, click a widget by author_id, set_value into a text input by author_id, and focus \
a pane through ActionChannel. The driver composes the real MCP click_widget/set_value path and egui \
AccessKit/Text events; it never calls SendInput, mouse_event, keybd_event, SetForegroundWindow, or similar \
Win32 APIs. Use a fresh snapshot between steps and read back the live tree after each action, especially \
runtime-chat-input values and focused pane author_ids. Unknown, disabled, unauthorized, and queue-full paths \
return typed NavigationError values instead of panicking, so a parallel model can recover without guessing."
        .to_owned()
}

fn flight_recorder_body() -> String {
    "Flight Recorder is Tier 1: the backend business-event ledger and canonical replay/audit record for \
governed application events. Open the actual operator surface with OPERATOR -> Open Flight Recorder \
(menu.operator.flight-recorder), RUN -> Open Flight Recorder (menu.run.flight-recorder), or the \
command-palette alternative \
command-palette.option.hs-flight-palette-open. The mounted pane is flight-recorder-pane; each accepted row is \
fr-event-{event_id} and shows action, actor_id, and RFC3339 timestamp. A canonical menu activation settles at \
mt036.flight-recorder-open-completion only after a fresh projected tree contains flight-recorder-pane. On open, and whenever Refresh is \
pressed through flight-recorder.refresh, the shell issues GET /api/flight_recorder with only \
wsid=<active workspace>. While that one bounded request is active, flight-recorder.loading-status is the \
readable JSON loading authority with its exact active_request_generation. A Refresh pressed while a GET is active remains queued and runs after that \
delivery; it does not start an unbounded parallel fetch or leave a perpetual spinner. The workspace filter \
is the runtime-derived ownership boundary; the closed reader then accepts native-editor rows and exact canonical \
FEMS lifecycle rows: FR-EVT-MEM-001 memory_write_proposed, FR-EVT-MEM-002 memory_write_reviewed, \
FR-EVT-MEM-003 memory_write_committed, FR-EVT-MEM-004 memory_pack_built, and FR-EVT-MEM-005 \
memory_item_status_changed. The pane shows each FEMS event code in its row and quarantines a mismatched \
event_type/event_code or malformed payload at flight-recorder.quarantine-status. Load failures remain at \
flight-recorder.load-failure and expose flight-recorder.retry. Restore or rebind the backend, then activate \
Retry to issue one new bounded GET; do not infer recovery from an old row. Refresh and Retry declare their \
exact action through flight-recorder.action-completion, which reaches Applied only after the matching fetch \
generation renders its current event ids and reaches typed Failed with the matching fetch generation/error \
when loading fails. Recent native emit failures are listed under flight-recorder.error-ring as \
flight-recorder.emit-error-{index}. Editor Settings therefore exposes \
settings-editor-flight-recorder-posture: Flight Recorder has no dedicated preference or disable toggle. \
The native POST envelope is closed: schema_version=hsk.native_editor@0.1; event_id=non-nil UUID; \
ts_utc=RFC3339; kind=one accepted action; actor_id, pane_id, and workspace_id=non-empty strings; \
actor_kind=optional human|agent|system; surface=optional non-empty string (otherwise pane_id); \
session_id=optional non-nil UUID; work_packet_id=optional non-empty string; payload=an object no larger than 64 KiB. \
Each action payload is also closed and required-key typed: document_saved={document_id:string, \
content_hash:sha256,save_receipt_event_id:string,actor_kind:string,kernel_task_run_id:string, \
session_run_id:string,correlation_id:string}; the backend accepts it only when that receipt is an immutable \
KNOWLEDGE_RICH_DOCUMENT_SAVED EventLedger row matching the exact workspace/document/action/actor/run/correlation/hash. \
Missing, fabricated, or cross-save receipts fail closed. code_edit={file_path:string,line_delta:i64}; embed_created={embed_kind:string, \
item_id:string,target_document_id:string}; canvas_node_placed={canvas_id:string,node_id:string,node_kind:string}; \
cross_ref_inserted={ref_kind:string,symbol_entity_id:string,target_document_id:string}; \
undo_fired={scope:local|cross_pane}; route_to_stage={content_kind:string,causal_action_id?:non-empty string}; \
stage_embed_back={artifact_id:string,target_pane_id:string,sha256:sha256, \
manifest_ref:string,causal_action_id?:non-empty string}; calendar_event_bound={date:YYYY-MM-DD,document_id:string,calendar_event_id:string}; \
activity_span_correlated={calendar_event_id:string,activity_span_id:string,edited_document_ids:non-empty string[]}; \
locus_ref_resolved={locus_uri:string,target_kind:work_packet|microtask,target_id:string}; \
locus_reverse_lookup={locus_uri:string,document_ids:non-empty string[]}. Accepted storage rows use \
event_type=system with payload.event_family=native_editor, payload.schema and payload.schema_version \
hsk.native_editor@0.1, matching action/kind, editor_surface, pane_id, workspace_id, actor_id, ts_utc, ops, and \
native_payload. The 12 accepted actions are document_saved, code_edit, \
embed_created, canvas_node_placed, cross_ref_inserted, undo_fired, route_to_stage, \
stage_embed_back, calendar_event_bound, activity_span_correlated, locus_ref_resolved, and \
locus_reverse_lookup. Unknown actions, unknown payload fields, missing required fields, wrong types, malformed \
UUIDs/timestamps, and cross-identity rows fail closed. The reader skips unrelated traffic and quarantines \
malformed native-editor or FEMS candidates with an operator-visible rejection reason instead of displaying \
them as trusted history. Actor attribution is explicit: the current shell binds human edits to \
native_editor_human and never reuses the last model-launch request as if it were a live actor lease. An \
identity-aware emitter may use a model/session actor only when the shell has an authoritative live binding. \
Emit work stays off the egui frame thread and uses a bounded queue. Transport, backpressure, no-runtime, \
closed-worker, and workspace-mismatch failures enter the shared in-memory error ring (latest 20), which the \
pane renders below the durable rows; this ring explains recent local failures but is not durable authority. \
Recovery: restore handshake_core/PostgreSQL reachability, inspect the error ring and quarantine message, then \
press Refresh. The backend's durable pending-mirror receipt and reconciler repair interrupted EventLedger -> \
Flight Recorder mirror windows; never fabricate a row or treat an empty pane as proof that no action occurred. \
Use internal_diagnostics for in-process health/backend-down evidence and Palmistry for freeze/crash survival; \
they supplement, never replace, this Tier-1 business ledger. HBR-INT-009 posture for this pane is exact: \
Tier 1 Flight Recorder/EventLedger is WIRED through the durable workspace-scoped rows; Tier 2 \
internal_diagnostics is WIRED through request-generation-correlated Start, Recovered, and Degraded load events; \
Tier 3 Palmistry is WIRED through that same shared diagnostic ring without copying event payloads. The \
canonical recovery sequence is argus.inspect -> click menu-operator -> fresh inspect -> click \
menu.operator.flight-recorder -> fresh inspect exact fr-event-{event_id} rows -> click Refresh and inspect \
the bounded failure -> fresh inspect flight-recorder.retry -> click Retry -> fresh inspect the recovered exact \
rows. Require terminal receipts from mt036.flight-recorder-open-completion and \
flight-recorder.action-completion and finish with zero Indeterminate actions."
        .to_owned()
}

fn internal_diagnostics_body() -> String {
    "internal_diagnostics is Tier 2: the in-app self-diagnostics layer. It owns the process-global \
diagnostic-event API, the bounded last-N event buffer, the optional shared-memory ring writer, heartbeat, \
frame-time, CPU/RSS/GPU/resource counters, panic hook, backend-down events, the operation watchdog, and \
the Settings -> Diagnostics projection. Use it when the app is still running and you need to understand UI \
health, slow frames, resource pressure, backend reachability, stalled in-app operations, or a typed \
diagnostic event emitted by a feature. A deadline-bounded operation registers an OperationCode with the \
watchdog, ticks progress, and completes when done; the first shipped consumer is the backend health/layout \
path using OperationCode::BackendCall. If progress stops past the deadline, the watchdog emits one typed \
StalledOperation event through the diagnostic-event API: sequence_id is the opaque operation id, counter_a \
is the OperationCode discriminant, counter_b is last_progress_ms, metric_micros is elapsed_ms * 1000, and \
timestamp_nanos is monotonic. It never records names, command lines, arguments, or paths. A model reads it \
through Settings -> Diagnostics: diagnostics_panel for the surface, diagnostics_heartbeat for liveness, \
diagnostics_frame for slow-frame stats, diagnostics_resource for CPU/RSS/GPU, diagnostics_events for recent \
StalledOperation rows, and diagnostics_palmistry for Tier-3 survivor projection. The status bar also shows \
Stalled ops while an operation is actively stalled and clears when it completes. Recovery is to inspect the \
typed event, identify the OperationCode lane, let or force the operation to finish/cancel, then verify the \
status bar clears and no new StalledOperation event is emitted for a ticking/completed operation. It does \
not replace Flight Recorder's business ledger and it cannot by itself survive a fully dead process. When \
handshake_core is unavailable, backend work remains off the egui frame thread: the status bar changes to a \
finite Disconnected/degraded state, the heartbeat and editor input continue, and Settings -> Diagnostics \
records one BackendUnreachable row on the down edge instead of flooding every frame. The shared native \
HTTP pool bounds connection setup at 1.5 seconds and a silent accepted request at 10 seconds; these are \
fixed safety bounds, not operator preferences, so this WP adds no timeout setting. Handshake continues \
bounded health probes; after the backend responds again the surface reconnects, the degraded state clears, \
and exactly one BackendRecovered edge is recorded. Operators reach this surface through Operator -> Open \
Settings… (or Help -> Open Settings…), then search for diagnostics. A model diagnosing backend loss should \
read the status bar plus diagnostics_events, keep editing local buffers, avoid repeated commands while a write outcome is \
unknown, and verify BackendRecovered before retrying a mutation.\n\nThe exact V4 integrated recovery \
proof binds the current worktree candidate and one mounted run: it starts a fixture-owned handshake_core against \
real PostgreSQL, launches the real out-of-process Palmistry binary on the app's exact shared diagnostics \
ring, observes the connected status through canonical localhost Argus, OS-suspends only that owned backend \
process so its real listener becomes half-open/silent, starts a fresh production layout load against that \
silent listener, and proves the layout worker drains while frames plus the Palmistry-shared heartbeat \
continue and one endpoint-attributed BackendUnreachable edge plus the finite Disconnected state appear. \
Canonical Argus also opens Settings -> Diagnostics and observes diagnostics_panel, diagnostics_events, \
diagnostics_palmistry, the BackendUnreachable row, and the active shared-memory ring. It then resumes and \
restarts handshake_core on the exact listener and PostgreSQL authority, proves one BackendRecovered edge, \
and has canonical Argus re-observe Backend: OK plus the recovered Diagnostics projection. Palmistry must \
stay alive across both phases and persist a CleanShutdown survivor receipt beside the exact ring for the \
same session. From src/frontend/handshake_native, use the configured single \
Handshake_Artifacts/handshake-cargo-target and never set CARGO_TARGET_DIR or pass --target-dir to another, \
nested, or per-run target. On Windows, canonical just recipes resolve that same target with absolute_path \
so a literal worktree\\.. segment cannot cross MSVC MAX_PATH; a direct command may pass --target-dir only \
to the normalized absolute form of that exact canonical root. Set \
HANDSHAKE_TEST_PG_DSN for an isolated real database, HSK_TEST_BACKEND_BIN to that canonical target's \
current-source handshake_core binary, HANDSHAKE_PALMISTRY_EXE to its current-source Palmistry binary, and \
HANDSHAKE_TEST_STAGE_BINDING_ROOT to a fresh external binding root; then run `cargo test --test \
test_backend_down_responsive backend_down_responsive_real_pg_palmistry_argus -- --ignored --exact \
--nocapture --test-threads=1`. Evidence is written under \
Handshake_Artifacts/handshake-test/wp-kernel-012-mt-088/integrated/run-*/ and includes status and Settings \
Diagnostics Argus trees, four unique action receipts with fresh terminal snapshots and passing predicates, \
connected/disconnected/reconnected render hashes paired to the degraded/recovered receipts, a deterministic \
HEAD-worktree candidate identity plus a separate deterministic SHA-256 candidate digest, with canonical path plus SHA-256 for every proof-driving input, full \
binary input manifests and executable hashes, per-frame elapsed-microsecond plus strictly advancing heartbeat \
samples, exact backend process/listener/workspace identities, endpoint-attributed typed event records, and the \
Palmistry control-socket/ring/session binding. Palmistry must report the test process as parent, write \
parent_exit_code null with CleanShutdown, and produce zero Freeze/Crash/ChildStall incident survivors while \
the heartbeat advances. The canonical Argus trace contains exactly four terminal-refreshed rows. Evidence is \
published only after Argus finish, app/layout-worker teardown, Palmistry clean shutdown/reaping, backend/PG \
fixture cleanup, and deletion of every fixture runtime root. HBR-INT-009 posture for backend \
reachability is explicit: Tier 1 Flight Recorder/EventLedger is NOT_APPLICABLE-with-reason for local \
reachability edges because it remains the PostgreSQL-backed business-event ledger and is not repurposed as \
a health log; Tier 2 internal_diagnostics is WIRED through the shared heartbeat plus BackendUnreachable and \
BackendRecovered; Tier 3 Palmistry is WIRED as the external child reading that exact ring and surviving the \
backend fault/restart."
        .to_owned()
}

fn palmistry_body() -> String {
    "Palmistry is Tier 3: the external out-of-process watcher. It exists for the failures the app cannot \
reliably report about itself: UI-thread freeze, crash, heavy CPU, dead process, or a spawned child process \
that stays alive while progress stops. Palmistry reads the shared-memory ring for Handshake liveness and \
uses the held control socket only for control messages such as RegisterChild/DeregisterChild. A watched \
child supplies a passive file-counter liveness source; Palmistry confirms ChildStall only when the child \
process is alive and that counter has stopped advancing past the threshold. Missing progress before a \
baseline is not a stall; missing or malformed progress after a baseline is suspected only, not durable \
ChildStall. Palmistry persists typed freeze/crash/ChildStall survivor records under the portable survivor \
store (`dirs::data_local_dir()/handshake/palmistry/survivors`) unless HANDSHAKE_PALMISTRY_SURVIVOR_DIR \
points at a scoped test/recovery directory. ChildStall survivor records carry child_process_id, \
child_session_id, stale_ms, last_progress_counter, last_progress_ts_nanos, and \
child_stall_reason_code; the minimal Settings row projects child_process_id, child_session_id, stale_ms, \
last_progress_counter, and child_stall_reason_code. Reason code 1 means progress stale while the child \
process was alive. It captures crash minidump/debris metadata where \
available, and the recovered app projects durable records in diagnostics_palmistry under Settings -> \
Diagnostics. Runtime proof path: build Palmistry, set HANDSHAKE_PALMISTRY_EXE if it is not side-by-side \
with the native exe, then run `cargo test --manifest-path src/frontend/handshake_native/Cargo.toml --test \
test_no_silent_hang_end_to_end -- --include-ignored --nocapture` to exercise the real watcher, real child \
process, real ring, scoped survivor store, and global operation watchdog together. Recovery is to read \
Settings -> Diagnostics -> diagnostics_palmistry, inspect the typed child ids/reason/progress fields, and \
only then decide whether to kill/restart the child or app. Use Palmistry when the app is frozen, crashed, \
too busy to update internal_diagnostics, or supervising a long-running child whose terminal/model/subprocess \
work could silently hang. The three-tier choice is: Flight Recorder for business events while healthy, \
internal_diagnostics for in-app health/stalled operations while the app still runs, and Palmistry for \
freeze/crash/child-stall survival when the app itself or its child process is not trustworthy."
        .to_owned()
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────────
// WP-KERNEL-012 wave-5 per-surface topic bodies (one dedicated no-context topic per native editor
// surface). Every author_id/route named here is a LIVE surface verified against the source, and
// persistence in every body is described only as handshake_core PostgreSQL/EventLedger so the content
// guard (which bans the local-store token + the direct-write phrase) stays green.
// ─────────────────────────────────────────────────────────────────────────────────────────────────────

fn code_editor_body() -> String {
    "The Code Editor is the VS Code-parity native code pane (PaneType::CodeSymbol, seeded as pane-a on the \
default worksurface). Open a file from the project tree (left-rail.activity.files) or Quick Switcher; the \
buffer mounts with syntax highlighting, line numbers/gutter diagnostics, code folding, a minimap, and a \
symbol outline. Large files are virtualized automatically: the scroll area paints only the visible row range, \
so there is no operator setting to enable 100000-line handling. Editing parity: multi-cursor (editor.code.multi-cursor-add / editor.code.multi-cursor-clear), \
find + replace (editor.code.find-open Ctrl+F, editor.code.replace-open, with editor.code.find-toggle-case/ \
-word/-regex), Format Document (editor.code.format, Alt+Shift+F), and language selection \
(editor.code.language-picker-open). Folding uses gutter triangles (code_editor_fold_target_0), the fold node \
code_editor_fold_0 with Expand/Collapse actions, EDIT menu leaves menu.edit.fold-region / \
menu.edit.unfold-region / menu.edit.fold-all / menu.edit.unfold-all, palette commands editor.fold.atCursor / \
editor.fold.unfoldAtCursor / editor.fold.all / editor.fold.unfoldAll, Ctrl+Shift+[ / Ctrl+Shift+] for the \
    region at cursor, and Ctrl+K Ctrl+0 / Ctrl+K Ctrl+J for Fold All / Unfold All; a collapsed region renders its ellipsis summary \
    label while hidden body rows are absent. Code intelligence currently discovers rust-analyzer on PATH for Rust \
    buffers; unsupported languages and missing or failed servers remain usable in an honest typed absent state. The \
    visible AccessKit status node code-editor.lsp-status reports configured, initializing, attached, restarting, or \
    absent for the detected language. Server processes launch without a foreground console, restart after transport \
    failure, and receive bounded shutdown/exit cleanup when the editor closes. Document synchronization is serialized \
    as didOpen/didChange/didClose before completion, hover, definition, and reference requests. Completion opens \
    immediately from Ctrl+Space or after a bound trigger-character debounce into code_editor_completion_popup with \
    code_editor_completion_item_{n}; moving the caret, changing the document, or dismissing the popup invalidates stale \
    work. Hover dwell opens code_editor_hover with a same-file or cross-file go-to-definition link. A same-file \
    target moves the caret in the current panel; a cross-file target is percent-decoded, resolves relative to the \
    current document, loads on the background runtime, and opens an independent file-backed editor tab without \
    replacing the source buffer, undo history, breakpoints, or draft edits. Duplicate exact CodeNav symbol names are \
    resolved only when the active source path identifies one candidate; otherwise navigation remains ambiguous instead \
    of opening an arbitrary definition. LSP publishDiagnostics is URI-scoped and uses normalized URI identity before it reaches the \
    gutter, and diagnostic UI remains evidence-only under the \
    HBR-INT disposition model. F12 and Shift+F12 prefer LSP results, then use the Handshake code-nav fallback through \
    /knowledge/code/symbols and related routes. References from either source share the actionable \
    code_editor_references list; enrichment is capped at 20 results with at most four backend requests in flight and \
    is cancelled when the request generation changes or the overlay closes. code_editor_reference_{n} opens the exact \
    same-file or cross-file target and \
    code_editor_references_close dismisses it. Backend-populated CodeNav content requires an indexed PostgreSQL \
    workspace and is honestly treated as NEEDS_MANAGED_RESOURCE_PROOF when no indexed workspace is seeded. Code \
    navigation is reachable from the GO \
    menu or keys: Go to Definition (F12), Go to References (Shift+F12), Go to Symbol in File (Ctrl+Shift+O), Go to Line \
    (Ctrl+G), and jump Back/Forward (Alt+Left / Alt+Right). Plain typing, IME commits, Backspace/Delete, \
paste/cut, completion accept, and whole-buffer edits record into the MT-035 focused-pane undo ring; Ctrl+Z, \
menu.edit.undo, and the header indicator undo-count-{pane_id} all read the same shared InteractionBus depth. \
Save with editor.code.save (Ctrl+S); the buffer \
    persists through the handshake_core backend client onto PostgreSQL/EventLedger, never bypassing handshake_core. \
For a code block inside a mounted rich note, activate its stable editor.rich.code-block.open.re-block-* \
Edit-code action (a model uses click_widget on that exact author_id). The selected block opens as an \
independent native Code Editor tab; editor.code.text accepts AccessKit SetValue/ReplaceSelectedText, and \
editor.code.save writes that exact block back into the SAME note's content_json through the note's existing \
MT-020 SaveManager and PUT /knowledge/documents/:id/save route. If the note/block changed after opening, or \
if a version conflict/save is already active, save fails visibly and leaves the code buffer dirty; reconcile \
the note/conflict, reopen the exact block, and retry rather than overwriting another block. The binding captures \
the owning document's complete structural snapshot when Edit code is activated and verifies it at save time; \
even inserting another identical-text code block before the selected path is rejected as positional drift. \
The bottom status bar exposes the editor segments status-bar-language-mode / status-bar-eol / \
status-bar-indent / status-bar-encoding / status-bar-render-whitespace. Indent width, tabs-vs-spaces, word \
wrap, and render-whitespace are driven live from Settings -> Editor (see the Editor Settings topic)."
        .to_owned()
}

fn rich_text_editor_body() -> String {
    "The Rich Text Editor is the Obsidian/Notion-parity native Notes pane (PaneType::LoomWikiPage, the \
loom.wikipage class, seeded as pane-b). Create a fresh note from FILE > New Document (command editor.file.new) \
or open an existing one from the project tree, Quick Switcher, a wikilink, or a graph/outgoing-link row; the \
shell performs GET /knowledge/documents/:id and binds the MT-020 SaveManager + DraftManager to that id and \
doc_version. Formatting commands: editor.rich.format-bold (Ctrl+B), editor.rich.format-italic (Ctrl+I), \
editor.rich.format-code (Ctrl+E), editor.rich.format-heading-1..6, plus lists, blockquotes, code blocks, \
horizontal rules, and tables from the toolbar. Insert blocks/embeds/wikilinks with the slash menu \
    (editor.rich.insert-slash-command, '/'). For headless creation, dispatch ClickWithPayload JSON exactly \
    {\"kind\":\"note\",\"title\":\"<title>\"}; insert an exact persisted wikilink with \
    {\"kind\":\"wikilink\",\"ref_kind\":\"note\",\"ref_value\":\"<exact-id>\",\"label\":\"<display>\"}, or an exact code block with \
    {\"kind\":\"code_block\",\"language\":\"rust\",\"code\":\"<exact-code>\"}. Wikilink and code-block payloads use the existing \
    transactional slash executor and immediately join model undo, unified undo, and save/draft dirty state; no transient \
    slash-row or autocomplete-candidate id is required. Direct wikilinks use the same canonical prefix-to-kind \
    classification table as typed wikilinks; unknown kinds and whitespace-only identities are rejected without \
    mutating the document or recording undo. For note creation, the existing WikilinkRuntime performs the production POST and the mounted \
    result node editor.rich.created-document exposes the created id. Blank/malformed payloads and missing runtime/workspace \
    stay visible as interop_error; correct the payload or restore backend reachability and retry. Reading (preview) view is the Obsidian reading-view parity toggle \
rich-reading-mode-toggle with segments rich-reading-mode-edit and rich-reading-mode-reading; the chosen mode \
is per-document and reuses the ONE MT-011 document model (no second render path). Save with editor.rich.save \
(Ctrl+S) or FILE > Save; the authoritative route is PUT /knowledge/documents/:id/save with expected_version \
and content_json, and drafts use GET/PUT/DELETE /knowledge/documents/:id/draft for crash recovery. All \
    persistence is handshake_core PostgreSQL/EventLedger; a successful mounted save automatically emits document_saved \
    only when its canonical save receipt plus actor/task/session/correlation attribution are present and backend-authentic. \
    Every desktop/headless HandshakeApp constructor allocates a distinct hsk:native_editor:host:<uuid> save-participant \
    actor automatically; an embedding host may override it only before any rich document mounts. \
    If receipt correlation is unavailable, the save remains committed but interop_error explains why no FR claim was emitted; \
    restore EventLedger health and save again rather than fabricating a receipt. Reopening a note re-GETs the authoritative document \
rather than trusting a cached editor buffer. Save conflicts render conflict-dialog with conflict-keep-yours, \
editor.rich.conflict.keep-server, conflict-keep-yours-confirm, and conflict-open-merge; the evidence posture is \
Flight Recorder/EventLedger plus internal_diagnostics and Palmistry per HBR-INT-009. Draft recovery renders \
draft-recovery-banner when the second open serves a non-null GET /knowledge/documents/:id/draft response; \
draft-restore loads the recovered content into the editor without canonical-saving it, and draft-discard \
clears the draft through DELETE /knowledge/documents/:id/draft. Wikilinks, tags, embeds, transclusions, and \
code refs insert inline atoms through Step::InsertInlineChild with a pushed history receipt, so Ctrl+Z restores \
    the exact pre-insert content before later text undo. Live wikilink chips use \
    editor.rich.wikilink.chip.* and autocomplete candidates use editor.rich.wikilink.candidate.*; discover exact \
    dynamic suffixes with list_widgets. A generic chip id encodes the complete target UTF-8 bytes plus its \
    document occurrence path, so repeated identical wikilinks remain individually addressable without hash \
    collision or full-tree author-id overlap. Media embed blocks support image, slideshow, album, \
    and video assets from the active workspace. A sequence can use an ordered comma-separated asset list or \
    collection:<id>; the resolver uses backend metadata plus thumb, preview, poster, and full content tiers. \
    Album cells load thumbnails and fetch a full image only after selection, slideshows fetch the active image, \
    and videos render a poster with an in-app play state instead of launching a foreground player. All metadata, \
    body fetch, and decode work shares a six-operation limit; oversized bodies, dimensions, and pixel counts fail \
    with a typed visible error. Click a single image to open embed-image-modal-{asset_id}, close it with \
    embed-image-modal-close-{asset_id}, or retry a transient asset failure with embed-retry-{asset_id}. Argus raw \
    click receipts are `Applied` only for controls that explicitly opt into the strict \
    handshake.click-completion/v1 token: the same effect/context must advance by exactly one generation from a \
    settled Ready or Applied state through optional Pending to Applied, or a durable observer must acknowledge the \
    exact transient target and semantic value. Generic clicks, payload clicks, malformed tokens, identity/context \
    drift, and generation jumps remain `Indeterminate`; a visible UI change alone is not causal proof. Changing \
    workspace cancels owned work and clears metadata, decoded-image, texture, sequence, modal, and failure caches so \
    an asset id cannot leak content from the prior workspace. A still-resolving embed is the addressable loading \
    state embed-loading-{asset_id}; a missing or undecodable asset renders the typed chip embed-error-{asset_id} \
    (never blank, never a panic). HBR-INT-009 diagnostic posture for embeds, verified against the current worktree: \
    Tier 1 Flight Recorder is NOT repurposed for the embed read/render path — resolving and decoding an asset is a \
    read, so no embed-render business event is fabricated; the underlying asset/Loom block the embed resolves \
    already carries its handshake_core Flight Recorder/EventLedger events. Tier 2 internal_diagnostics is SHIPPED in \
    this worktree (NOT deferred): embed failures degrade to typed visible chips and all metadata, body-fetch, and \
    decode work is bounded by a shared six-operation budget plus a 15s per-operation timeout, so a stuck asset \
    becomes a typed timed-out chip rather than a permanent spinner, and Settings -> Diagnostics projects live in-app \
    health; the embed pipeline does not yet emit dedicated internal_diagnostics rows (a follow-up), but the tier \
    itself exists here. Tier 3 Palmistry is SHIPPED in this worktree (NOT deferred): image decode runs off the UI \
    thread (tokio::spawn_blocking) so the frame loop stays responsive, and a genuine freeze/crash is covered by the \
    external out-of-process watcher at the app boundary; there is no embed-specific Palmistry child. Export starts \
    at rich-editor-export-button and opens export-format-picker for HTML/MD/TXT/JSON output."
        .to_owned()
}

fn knowledge_graph_body() -> String {
    "The Knowledge Graph (Loom graph view) renders the block/note link graph for the workspace. Open the \
graph SURFACE from VIEW > Knowledge Graph (menu.view.open-knowledge-graph), the Command Palette option \
command-palette.option.hs-view-palette-graph, or command id view.graph. The mounted pane performs its initial \
fetch when the Graph View pane is visible, drains the shared backend delivery cell into LoomGraphView::set_graph_projection, \
and refreshes the same mounted view after graph mutations. Global mode uses graph.mode.global and \
GET /workspaces/{id}/loom/graph/global?node_limit=5000&hub_degree_threshold=0; \
GET /workspaces/{id}/loom/views/all is the independent block-count oracle used to verify the projection. \
Local mode uses graph.mode.local and GET /workspaces/{id}/loom/graph/local?start_block_id={block_id}&max_depth=N&node_limit=200. \
The Link-depth/max_depth control re-queries Local mode for the chosen depth. Zoom with graph.zoom.in / graph.zoom.out; pan by dragging \
the empty canvas or by the catalog actions graph.pan-left / graph.pan-right; click graph.relayout to restart \
layout. For canonical model control, use argus.inspect before argus.click on graph.relayout. The click target \
carries handshake.click-completion/v1 and advances the same effect/context by exactly one layout generation \
through Pending to Applied; graph.relayout.status is a separate stable Status node whose JSON reports \
layout_generation, running/stable status, layout_state_sha256, iterations, node_count, and edge_count. Treat \
the action as complete only when the raw receipt is Applied and a fresh status has the exact prior generation + 1, \
stable state, expected counts, and the matching SHA-256. If the receipt expires or is Indeterminate, re-run \
argus.inspect, confirm workspace/mode identity and backend health, and issue a new click; never infer success from \
unchanged visible nodes. Open a rendered TreeItem node by clicking its dynamic AccessKit id graph.node.{block_id}. Already-safe \
block ids remain literal; every other raw UTF-8 id is encoded injectively as a u8- hexadecimal suffix so two blocks cannot alias, \
so use argus.inspect to discover the emitted id instead of guessing it. The legacy catalog \
action graph.open-node is the registry action shape for node-open automation. ModeChanged re-fetches Local or \
Global data through the LoomGraphClient. AddEdge and RemoveEdge dispatch the existing /loom/edges backend \
mutation requests and then re-fetch the graph; no graph read or write bypasses handshake_core. The backend's \
truncated flag and suppressed_hub_ids make a bounded projection visible; an omitted suppressed_hub_ids field \
means no hubs were suppressed. Empty workspaces show 0 nodes. Backend failures stay visible as Graph error: ... \
with graph.retry instead of clearing the surface. A workspace switch clears nodes, selection, local focus, errors, \
and queued request identity even while the graph tab is closed, so an older A response cannot re-enter after \
A -> B -> A. The integration-gated graph_view_live_pg_self_seeds_local_global proof creates an isolated \
Handshake-managed PostgreSQL workspace, verifies the real pre-seed 0-node Global projection, seeds linked \
Loom blocks, verifies populated Global and Local projections, forces a bounded typed transport failure, and \
retries the exact same workspace/mode/depth before its cleanup guard removes the seeded workspace. A missing \
backend fails that proof instead of skipping it. Recovery: prefer argus.inspect (list_widgets remains a compatibility inventory) to verify the \
toolbar and graph.node.* ids, switch graph.mode.local / graph.mode.global to re-query, click graph.relayout \
after layout confusion, seed or restart the backend when the live graph is empty, click graph.retry, and inspect \
internal_diagnostics if Graph error: remains. Relayout, pan, zoom, and graph reads are ephemeral UI/read \
state, so Tier 1 Flight Recorder/EventLedger is NOT_APPLICABLE-with-reason for those non-durable actions; \
AddEdge and RemoveEdge are separate durable mutations and retain their handshake_core EventLedger evidence. \
Canonical model actions use the stable parameterized targets graph.add-edge, graph.remove-edge, \
canvas.place-block, canvas.remove-placement, and collection.kanban-move discovered by argus.inspect. \
Supply their exact JSON payload and accept only an Applied receipt: edge creation binds the backend-minted \
edge_id and a newer authoritative graph refresh, Canvas placement binds the backend-minted placement_id and \
the refreshed board, removal proves exact absence, and a Kanban move proves target-lane membership plus \
source-lane absence. Clicking graph.node.{block_id} completes only after that exact rich document and load \
generation are active. Malformed or stale payload/context is Rejected without a write. On Graph error, inspect \
graph.retry; a successful Retry removes that control and records a newer request generation, while another \
typed failure leaves the exact Retry control mounted for a fresh attempt. \
Tier 2 internal_diagnostics observes in-process health, and Tier 3 Palmistry supplies the external \
freeze/crash watcher."
        .to_owned()
}

fn block_collection_views_body() -> String {
    "Block Collection Views are the mounted saved table, Kanban, and calendar projections over real \
Loom blocks. Open the pane from VIEW > Block Collections (menu.view.open-block-collections), command \
view.block-collections, or a Search result whose block.content_type is view_def. An unbound pane shows \
an honest no-view state and still exposes bcv.new-view. Activate it, set bcv.new-view.title, select \
bcv.new-view.kind.table, bcv.new-view.kind.kanban, or bcv.new-view.kind.calendar, then activate \
bcv.new-view.confirm (or bcv.new-view.cancel). Creation sends a stable client-generated block_id to \
POST /workspaces/{workspace_id}/loom/views/definitions and retains it across Retry. The backend \
atomically commits the final view_def block, search projection, ProjectKnowledgeIndex bridge, \
EventLedger mutation receipt, and Flight Recorder outbox; an ambiguous response retry converges on \
the same saved view instead of creating a duplicate. Creating view… remains visible while authority \
responds and the returned id is rebound before definition/results reload.\n\n\
For a table, activate bcv.table.sort.{field}; the host PATCHes \
/workspaces/{workspace_id}/loom/views/definitions/{block_id}, then POSTs \
/workspaces/{workspace_id}/loom/views/definitions/{block_id}/results. Rows are \
bcv.table.row.{block_id}; sorting is backend-authoritative, never a local reorder. Kanban lanes/cards \
are bcv.kanban.lane.{key} and bcv.kanban.card.{block_id}; the sanitized lane key is a tag id for \
tag grouping, a field value for field grouping, or untagged for the synthetic untagged lane. Moving a card writes real add_tags and \
remove_tags mutations, then re-queries; the card does not move locally before PostgreSQL confirms it. \
Calendar inputs bcv.calendar.date-from and bcv.calendar.date-to accept YYYY-MM-DD; activate \
bcv.calendar.apply-range to persist the definition and re-query. Switch and persist kinds with \
bcv.kind.table, bcv.kind.kanban, and bcv.kind.calendar.\n\n\
Every steerable collection control publishes an opt-in handshake.click-completion/v1 observer \
declaration in its own AccessKit value and terminalizes through the durable Role::Status observer \
bcv.action-completion, so a canonical Argus receipt is causally Applied or typed-Rejected instead of \
indeterminate. Because a declaring button's value now carries that declaration, its selected / \
not_selected projection is published on a dedicated sibling Role::Status node named \
{author_id}.state - read bcv.kind.table.state, bcv.kind.kanban.state, bcv.kind.calendar.state, \
bcv.new-view.kind.table.state, bcv.new-view.kind.kanban.state, and bcv.new-view.kind.calendar.state \
instead of the button value. Retry, kind switch, sort, Kanban card move, calendar range, and create \
finish Applied only when an authoritative getBlockView plus queryBlockViewResults readback lands at a \
fresh load generation; the terminal detail records the workspace id, view_def block id, prior and \
resulting load generation, persisted kind/sort/range, and the returned result and lane identities. A \
backend failure finishes Rejected with the typed error, never Applied. The text inputs \
bcv.new-view.title, bcv.calendar.date-from, and bcv.calendar.date-to publish \
handshake.set-value-completion/v1 observers at {author_id}.set-value-completion that advance one \
generation only when the widget genuinely consumes the AccessKit SetValue request; a value echo alone \
is never proof.\n\n\
Empty states are exact and visible: No blocks match this view.; No Kanban lanes.; No blocks in this \
date range. Backend or malformed-response failures remain at bcv.status as View error: ... . Activate \
bcv.retry to replay a retained create with the same block id or to reload the same view with one \
bounded definition fetch and one bounded results query. Workspace/generation guards discard stale \
deliveries. Diagnostic posture: Tier 1 Flight Recorder is WIRED for create/update through the \
transactional PostgreSQL outbox and restart reconciler; query events are observational. Tier 2 \
internal_diagnostics is WIRED at the shared host/watchdog but collection-specific counters are \
deferred. Tier 3 Palmistry is WIRED at the shared out-of-process freeze/crash boundary with no \
collection-specific child. Canonical proof: run tests/run_mt027_argus_proof.ps1 with a fresh RunId; \
it drives creation, mutation, switching, empty/error/retry, and post-action inspection through the \
real localhost Argus transport against real PostgreSQL and stores source-bound evidence only in the \
allocated external Handshake_Artifacts MT-027 root."
        .to_owned()
}

fn wiki_projection_body() -> String {
    "Wiki Projection is the dedicated generated Loom wiki-page surface. It is distinct from Rich Note: Rich Note opens PaneType::LoomWikiPage for an editable document, while Wiki Projection opens the mounted PaneType::Placeholder(\"Wiki Page\") host for a backend LoomWikiProjection. VIEW > Open Wiki Projection (menu.view.open-wiki-projection), the Command Palette row command-palette.option.hs-view-palette-wiki-projection, and command id view.wiki-projection reopen the concrete mounted projection when one exists; otherwise they open Quick Switcher with wiki discovery and the truthful status No active wiki projection instead of creating an empty pane. Selecting a wiki_page result opens its concrete projection id. The host strictly validates every GET /workspaces/{workspace_id}/loom/wiki/{projection_id} response: required fields must exist and returned workspace/projection ids must match the request. The title, page type, rebuild time, source-block count, and rendered_content are derived and read-only. Persisted overlay annotations are loaded through GET /overlays and rendered below the projection as wiki.overlays.{sanitized_projection_id} with each annotation at wiki.overlay.{sanitized_overlay_id}. Edit opens an additive annotation buffer. Save POSTs to /overlays and only exits after the identity-matched projection-plus-overlay reload succeeds. While Save and reload are in flight, Cancel and editing are locked so an old completion cannot clear a newer same-pane buffer. Cancel otherwise discards the unsaved buffer and performs no write. Rebuild calls /regenerate only for untyped Loom projections; typed project-wiki pages display that rebuild belongs to the project wiki engine. A rebuild failure retains the last-good page and appears at wiki.error.{sanitized_projection_id}; Retry repeats an initial failed load. Every asynchronous load, save, rebuild, and post-save reload carries workspace id, projection id, pane generation, and Save action generation. A late delivery for A is rejected after A -> B or A -> B -> A and cannot replace B or clear B's edit buffer. Edit, Cancel, and Save expose target declarations plus the durable observer wiki.action-status.{sanitized_projection_id}. Terminal receipts bind workspace, projection, pane generation, action generation, edit-mode generation, draft identity and SHA-256, source projection updated_at revision, source staleness hash, and source content SHA-256. Edit and Cancel finish Applied with write_count=0; Cancel additionally proves draft_discarded, edit_closed, and original_source_authoritative. Save remains Pending through POST and GET and finishes Applied only when the exact overlay_id, annotation, created_at, and updated_at returned by POST are present unchanged in GET /overlays and the source revision/hash/content are unchanged. The Applied receipt records write_count=1 plus overlay_persisted_revision and overlay_readback_revision. A POST failure finishes Rejected with typed wiki_save_transport, exact draft retained, edit still open, and write_count=0. A POST success followed by GET failure or source/readback conflict finishes Rejected as committed-overlay reconciliation failure with write_count=1; Save and Cancel stay locked and Retry Reload is GET-only. Stable AccessKit targets are wiki.title.{sanitized_projection_id}, wiki.content.{sanitized_projection_id}, wiki.metadata.{sanitized_projection_id}, wiki.edit.{sanitized_projection_id}, wiki.edit-area.{sanitized_projection_id}, wiki.save.{sanitized_projection_id}, wiki.cancel.{sanitized_projection_id}, wiki.rebuild.{sanitized_projection_id}, wiki.stale.{sanitized_projection_id}, wiki.error.{sanitized_projection_id}, wiki.retry.{sanitized_projection_id}, wiki.overlays.{sanitized_projection_id}, wiki.overlay.{sanitized_overlay_id}, and wiki.action-status.{sanitized_projection_id}. The Editor Settings section exposes settings-editor-wiki-projection-posture to state the contract truth: Wiki Projection has no dedicated preference; it uses the active workspace/theme and does not invent a second setting. Recovery: Save or reload failures preserve the annotation and expose wiki.error.{sanitized_projection_id}; restore the backend and press Save again or Cancel after the in-flight operation ends. Initial load failures use wiki.retry.{sanitized_projection_id}. The managed-PostgreSQL proof retires the previous canonical receipt before starting, self-seeds generated live ids, drives the mounted HandshakeApp host through canonical localhost Argus, proves Cancel no-write and Save persisted/readback terminal receipts, verifies overlays through the visible mounted panel after reload, captures the GPU frame when enabled, cleans up, and writes current evidence only under Handshake_Artifacts. The overlay route does not claim a Flight Recorder/EventLedger business event; internal_diagnostics and Palmistry remain general runtime recovery surfaces rather than MT-025 acceptance evidence."
        .replace(
            "Recovery: Save or reload failures preserve the annotation and expose wiki.error.{sanitized_projection_id}; restore the backend and press Save again or Cancel after the in-flight operation ends. Initial load failures use wiki.retry.{sanitized_projection_id}.",
            "Recovery: a POST failure preserves the annotation and allows Save retry or Cancel because no overlay was committed. If the overlay was saved but its follow-up reload fails, Save and Cancel remain locked, the panel says the overlay is already saved, and wiki.retry.{sanitized_projection_id} performs only Retry Reload; restore the backend and activate that control so no duplicate overlay is posted. Initial load failures use the same stable retry id for a normal load.",
        )
        .replace(
            "The overlay route does not claim a Flight Recorder/EventLedger business event; internal_diagnostics and Palmistry remain general runtime recovery surfaces rather than MT-025 acceptance evidence.",
            "The overlay insert and KNOWLEDGE_LOOM_WIKI_MUTATED EventLedger business event are committed atomically in PostgreSQL; the event is projected into Flight Recorder for replay and audit. If EventLedger append fails, the overlay insert rolls back. internal_diagnostics and Palmistry remain general runtime recovery surfaces for transport, freeze, and crash investigation.",
        )
}

fn folder_tree_body() -> String {
    "The Folder Tree is the native Obsidian-style folder surface for Loom blocks. Open it from VIEW > Open \
Folders (menu.view.open-folders), the Command Palette option \
command-palette.option.hs-view-palette-folders, or command id view.folders. The mounted pane is the real \
PaneType::Placeholder(\"Folders\") host, backed by LoomFolderTree, not a placeholder. When the Folders pane \
is visible the host performs GET /workspaces/{id}/loom/folders, builds the folder forest, and renders each \
folder row as folder-tree.node.{folder_id}; each folder color swatch exposes current color as the stable, \
actionable AccessKit Button folder-tree.color.{folder_id}; its Click opens the controlled picker. Use New folder to create a root. For row-scoped operations, open the stable \
folder-tree.node.{folder_id} context menu: New subfolder opens a create dialog with that row as parent; \
Rename opens the rename dialog; Move to root clears parent and order; Move under opens a target-folder \
submenu whose choices are addressable as folder-tree.move-target.{source_folder_id}.{target_folder_id}, so \
same-title targets remain unambiguous; Delete opens an explicit confirmation. The confirmation reports the \
current in-memory descendant-folder count because PostgreSQL recursively deletes that subtree and its folder \
memberships. Loom blocks are not deleted. These emit FolderTreeEvent::CreateFolder, RenameFolder, \
MoveFolder, or DeleteFolder and use the production POST, PATCH, and DELETE folder routes. Every successful \
write triggers an authoritative list refetch, so labels, hierarchy, ordering, selection removal, moves, and \
deletes reflect persisted state. Recolor follows the same rule: PATCH success only triggers the list \
refetch; the host does not apply the response color directly, and the displayed swatch changes only when \
that authoritative refetch delivers. Expand a folder to emit FolderTreeEvent::ExpandFolder; the host sets a \
bounded loading state and lazily fetches its children with GET \
/workspaces/{id}/loom/folders/{folder_id}/blocks?limit=500&offset={n}; the client continues until a short \
page, so folders above 100 members are not silently truncated. A defensive 100,000-member ceiling fails \
closed as a visible error instead of hanging or showing a partial folder. Click a folder to select and expand that \
organizational overlay inside the Folder Tree and reveal its member-block rows; an LFD-* folder id is never \
opened as a LoomBlock, and folder selection does not globally filter another pane. Click a child block to \
emit FolderTreeEvent::OpenBlock, which opens that real block through the shell LoomBlock navigation path. \
Each leaf is visibly indented at least one step right of its parent folder label and its stable TreeItem \
advertises Click only, never folder Expand or Collapse actions. \
The folder row's stable TreeItem accepts real AccessKit Expand and Collapse actions; models should target \
folder-tree.node.{folder_id}, not the visual disclosure glyph. For canonical causal expansion, use argus.inspect \
then argus.click on that row. Every row click revalidates membership with a fresh backend child-list request, \
including when cached children are already visible. Its handshake.click-completion/v1 token advances the exact \
workspace/folder generation through Pending while that request runs and reaches Applied only after a selected, \
expanded, non-loading terminal result from the request sequence bound to that generation. An older in-flight \
response is discarded and cannot satisfy a repeated click. Reinspect folder-tree.status.{folder_id}: its \
handshake.folder-expansion-status/v1 JSON exposes workspace_id, folder_id, generation, selected, expanded, loading, \
request_sequence, terminal_request_sequence, child_state, child_count, and a typed error. Require exact prior \
generation + 1, equal non-null request/terminal sequences, raw Applied, child_state loaded, and \
the expected folder-tree.node.{block_id}; a failed request terminates as child_state failed with the prior visible \
membership preserved. Concurrent folder loads retain each node-owned failure in the global Retry banner; success \
for one folder cannot hide another folder's still-live error. An expired or Indeterminate receipt requires a fresh inspect and retry, never inference from \
continued row presence. Right-click a folder row and choose Change color to open the explicitly controlled picker; \
normal primary folder-row clicks never open it, while an explicit color-swatch Button click does. Choosing a color emits FolderTreeEvent::ChangeColor and \
the host sends PATCH /workspaces/{id}/loom/folders/{folder_id} with only {\"color\":\"#rrggbb\"}, so name, \
sort, and parent fields are not clobbered. The prior swatch remains when PATCH or its authoritative refetch \
fails. Empty workspaces show No folders. Missing-parent, \
self-parent, and descendant-cycle move conflicts stay visible as Folder move failed and do not mutate the \
persisted hierarchy. Backend failures stay visible \
with folder-tree.retry as a Retry button; clicking it emits FolderTreeEvent::Retry and re-runs the folder \
list fetch. Failed mutations remain visible across authoritative reconciliation and clear only when a new \
mutation begins or Retry is chosen. Dialog controls are addressable as folder-tree.create.name, \
folder-tree.create.submit, folder-tree.create.cancel, folder-tree.rename.name, \
folder-tree.rename.submit, folder-tree.rename.cancel, folder-tree.delete.confirm, and \
folder-tree.delete.cancel. Sibling-name conflicts return typed HTTP 409 loom_folder_sibling_name. During a \
pre-0342 database upgrade, every duplicate folder is preserved and later duplicates receive a deterministic \
[recovered-{folder_id}] suffix before the truthful sibling indexes are installed. Recovery: use \
folder-tree.retry after backend loss, wait for the authoritative list, then retry \
the row action. Deletion has no automatic undo: cancel before confirming, or recover by recreating the folder \
subtree and reassigning surviving Loom blocks from authoritative block data. Populated live folder, child, \
CRUD, conflict, move-to-root, delete, and recolor persistence \
is covered by the self-seeding folder_tree_live_pg_self_seeded_round_trip proof against a \
Handshake-managed PostgreSQL backend. It records exact seed ids and cleanup_verified=true in the external \
MT-022-live-pg-seed.json receipt; a missing backend fails the proof rather than skipping it. Folder and membership \
mutations commit their matching EventLedger receipt atomically with the PostgreSQL change; a ledger failure rolls \
the mutation back rather than leaving unaudited state. HBR-INT-009 diagnostic posture: Flight Recorder/EventLedger \
= SHIPPED for those durable mutations; internal_diagnostics = DEFERRED-with-reason because this host has no folder-operation-specific event \
code yet; Palmistry = DEFERRED-with-reason because no folder-tree-specific external tracker is registered. \
Folder errors remain visible in-pane while those feature-specific diagnostic links are deferred. A model \
should use list_widgets to enumerate folder-tree.node.*, folder-tree.move-target.*, and \
folder-tree.color.* ids, click_widget on folder-tree.retry after an error, use the row context path for Change \
color/New subfolder/Rename/Move/Delete, and screenshot the tree when verifying \
swatch color or hierarchy."
        .to_owned()
}

fn tags_and_tag_hubs_body() -> String {
    "Tags and Tag Hubs are the native Obsidian-style tag navigation surface for Loom blocks. Open it from \
VIEW > Open Tags (menu.view.open-tags), the Command Palette option \
command-palette.option.hs-view-palette-tags, or command id view.tags. The mounted pane is \
PaneType::Placeholder(\"Tags\") backed by LoomTagsPanel plus an optional LoomTagHubPanel, not a placeholder. \
When visible, the host performs GET /workspaces/{id}/loom/tags and renders the filter box tags.search plus \
one row per tag hub as tags.row.{block_id}. Each row shows the hub title and an exact member count when the \
list response or GET /workspaces/{id}/loom/tags/{tag_block_id} detail provides member evidence; the exact \
member list is loaded when a tag hub opens. Type into tags.search to prefix-filter tag titles. \
Click a tag row to emit TagsPanelEvent::OpenTag and open that tag hub page in the same pane. Every \
model-facing row click declares the durable observer tags.navigation-status. Its terminal receipt contains \
source_tag_id, destination_tag_hub_id, workspace_id, workspace_generation, completion_generation, and \
completion_kind=authoritative-hub-membership-query-complete. Applied is published only after the exact \
current workspace/tag/request sequence returns the authoritative hub-membership result; stale workspace, \
stale tag, failed, or superseded deliveries never acknowledge the click. List member-count enrichment uses \
separate request authority: it may update a tags.row.* badge but can never populate the mounted hub, \
supersede the exact navigation request, or acknowledge its receipt. Choosing < All tags gives Back priority \
for that frame, retires any pending hub navigation without Applied, and ignores simultaneous input from the \
abandoned hub. The hub page \
loads GET /workspaces/{id}/loom/tags/{tag_block_id}, renders tag-hub.title.{block_id}, lists member blocks \
as tag-hub.member.{block_id}, and opens a member through the same shell LoomBlock navigation path. The Add \
tag to block button tag-hub.add-tag.{block_id} opens an in-process popup, searches with \
GET /workspaces/{id}/loom/search?q={query}&limit=20, and selecting a candidate POSTs \
/workspaces/{id}/loom/edges with edge_type='tag', source_block_id as the selected block, target_block_id as \
the hub, and created_by='user'; the host re-queries the hub only after the POST response resolves, with no \
fixed sleep. Switching projects clears the previous workspace's tag rows, search text, open hub, queued \
events, and stale async deliveries before refetching, so a no-context model should trust the active \
workspace in the visible pane rather than cached row text. Empty workspaces show No tags. The \
integration-gated tags_tag_hub_live_pg_self_seeds_mounted_round_trip proof creates an isolated workspace \
against Handshake-managed PostgreSQL/EventLedger, drives the mounted pane through empty/list/filter/open/add, verifies \
rename and tag removal with a fresh client, checks bounded backend loss, writes an external receipt, and \
deletes the workspace before reporting success. The exact canonical visual proof is \
mt023_mounted_tags_panel_canonical_argus_inspect_steer_reobserve with feature integration, the current \
backend, real PostgreSQL, one test thread, and GPU capture. Use argus.inspect to copy the emitted \
tags.row.* id, argus.click that exact id, require receipt status applied, then use a fresh argus.inspect \
to bind the same receipt and tags.navigation-status semantic value to the fresh tag-hub.title.* plus \
tag-hub.member.* state; disappearing list rows or a loading skeleton alone are not proof. Use \
argus.set_value on tags.search for filtering and argus.screenshot only after that terminal predicate. \
Failure recovery is in-pane: Retry repeats reads, while a failed add-tag write preserves the prior visible \
membership and exposes the typed backend error. HBR-INT-009 posture: Tier 1 Flight Recorder/EventLedger = \
NOT_APPLICABLE-with-reason for read-only tag navigation, and WIRED atomically for the durable tag-edge \
mutation (a ledger failure rolls the PostgreSQL write back); Tier 2 internal_diagnostics = \
DEFERRED-with-reason because there is no tag-navigation-specific diagnostic event; Tier 3 Palmistry = \
DEFERRED-with-reason because no tag-pane-specific external tracker is registered. Legacy list_widgets, \
set_value, and click_widget names remain compatibility inventory only; canonical operation uses the \
argus.* methods above."
        .to_owned()
}

fn canvas_body() -> String {
    "The Canvas is the free-form spatial board (PaneType::AtelierEditor / the CKC atelier surface) for \
arranging Loom blocks and text cards. Add a text card with canvas.add-card, place an existing Loom block with \
canvas.place-block, switch semantic/visual edge authoring with canvas.edge-mode, and connect items with \
canvas.add-edge. Pan and zoom with canvas.pan-left, canvas.pan-right, canvas.zoom-in, and canvas.zoom-out; \
canvas.zoom-value exposes the persisted zoom. Cards can be resized, grouped into sections, moved by dragging, \
and edited inline when the card-edit route exists. A completed card drag sends its canvas-space x/y and its \
resolved section together in one placement PATCH, so a fresh getCanvasBoard reload cannot snap the card back \
or retain a half-applied group change. Each persisted mutation (placement/card creation, move, resize, \
section assignment, semantic/visual edge, remove placement) emits a typed canvas event that the host turns into the \
real backend call — POST/PATCH/DELETE through the handshake_core canvas routes — followed by a \
getCanvasBoard refresh, all on PostgreSQL/EventLedger. Creation responses carry the backend-minted \
placement id; the host registers a cross-pane MT-035 compensating undo, so Ctrl+Shift+Z removes that \
  created placement with DELETE /workspaces/{id}/loom/canvas-placements/{placement_id} and redo re-places \
  the same block geometry. Undo and redo are provisional until the backend responds; while one compensation \
  is in flight, another Ctrl+Shift+Z returns a typed already-in-flight result instead of reordering history, \
  and focused local Ctrl+Z stays scoped to the active editor pane. Each completion reloads getCanvasBoard so \
  the mounted Canvas immediately reflects PostgreSQL truth. Redo accepts the newly minted \
  replacement placement id, so a later Ctrl+Shift+Z removes that replacement rather than retrying a stale id. \
  Restart recovery is session-scoped: a fresh app process starts with an empty undo history and cannot replay \
  an interrupted in-memory compensation without a new operator/model action. A failed compensation remains \
  visible in the Canvas status, restores the action to its original undo/redo ring, and can be retried without \
  losing history. Atelier items reach Canvas through a durable canonical relation: publish or confirm \
the relation with PUT /atelier/intake/items/{item_id}/loom-projection, then the batch-items response carries \
loom_block_id. The canvas emits ResolveAtelierAndPlace, accepts only that backend-provided identity, posts only \
placed_block_id to the canonical placements route, and freshly reloads the board. A missing or conflicting \
relation stays a visible typed blocker and never fabricates a Loom block. Inline text-card edit remains a typed blocker when the required persistence route \
is absent. If getCanvasBoard fails, the Canvas status shows the typed error and exposes canvas.retry; \
click_widget on canvas.retry re-runs the authoritative getCanvasBoard request through the host. Retry uses \
a bounded loading state: another failure stops loading and restores the error/Retry surface, while success \
replaces it with the fresh PostgreSQL board. Removing canvas.placement.{placement_id}.remove deletes only the \
placement reference; the canonical Loom source block remains available to other panes. Two durable \
Role::Status completion observers make those two actions CAUSALLY PROVABLE instead of Indeterminate: \
canvas.viewport-completion acknowledges canvas.pan-left/canvas.pan-right/canvas.zoom-in/canvas.zoom-out and \
publishes the board id, the prior and resulting viewport revision/scale/offset, the action id, the \
PUT /workspaces/{workspace_id}/loom/canvas-boards/{block_id}/viewport route, and authority=persisted; \
canvas.placement-mutation-completion acknowledges canvas.placement.{placement_id}.remove and publishes the \
workspace/board/placement/block ids, the prior and refreshed board generation, the \
DELETE /workspaces/{workspace_id}/loom/canvas-placements/{placement_id} route, \
placement_absent_after_refresh, and an explicit source_block_present confirmation read back through \
GET /workspaces/{workspace_id}/loom/blocks/{block_id}. Both observers terminalize ONLY on an authoritative \
getCanvasBoard refresh (the removal additionally requires the source-block read-back); an optimistic \
in-widget zoom, a vanished remove button, or an unchanged sibling node never terminalizes them, and a \
source block that did NOT survive the removal publishes a typed terminal failure instead of applied. \
The canvas DELETE route answers 204 with no body, so the receipt records event_ledger_event_id=null \
explicitly rather than inventing a correlation id. Open the canvas from \
the CKC module or the Command Palette; the editor never bypasses handshake_core. A no-context model should \
discover canvas.placement.*, canvas.edge-mode, canvas.place-block, canvas.add-card, and canvas.retry with \
list_widgets, verify exact resolved placement titles and section ids in AccessKit, use click_widget/set_value \
for deterministic controls, and use screenshot when spatial placement itself must be judged."
        .to_owned()
}

fn search_body() -> String {
    "Handshake has three complementary search surfaces for the melt-together worksurface. (1) Notes Search is \
the operator-facing name for the native Loom Search v2 engine. Open it from VIEW > Open Notes Search \
(menu.view.open-loom-search) or the Command Palette row View: Notes Search (command id view.loom-search; \
stable row command-palette.option.hs-view-palette-loom-search). Type the query into search.query, activate \
search.run, narrow with loom-search-v2.facet.* facets, read loom-search-v2.status, and open a hit from \
search.result.*. The request uses the handshake_core Loom hybrid-search route. The first pane retains those \
canonical ids; additional panes append --pane-<full-pane-id-UTF-8-hex> to query, run, save, status, facet, and \
result author ids so the AccessKit tree remains globally unique. Input is a non-empty query plus an optional \
content-type facet. Output is a clickable Notes block row with title, content-type badge, score, and a highlighted \
preview; marked excerpts render as highlighted text, never raw mark tags. The status reports semantic-on versus \
keyword/fuzzy-only from the backend's semantic_available truth. \
loom-search-v2.save-view is disabled until results exist. After an optional facet rerun, activate it to persist \
the current facet as a table view through POST /workspaces/{workspace_id}/loom/views/definitions with title \
Search: {query}; the status then shows the reloadable view block id. Saving does not open the view automatically. \
To open or reopen it, choose VIEW > Open Quick Switcher (menu.view.open-quick-switcher), enter the exact saved \
title Search: {query} in quick-switcher.search, verify the returned loom_block has content_type=view_def and the \
expected block id, and activate that row. The typed route opens the mounted Block Collections pane for that exact \
view id. After closing the tab, repeat the same Quick Switcher path to reopen it. If the definition or result load \
shows View error: ..., restore the backend and activate bcv.retry; that reloads the same view id with one bounded \
definition fetch and one bounded results query. \
An empty query reports Search query is required without transport. A successful no-hit query reports 0 results and \
keeps Save as view disabled. A backend error remains visible instead of spinning forever; restore the backend and \
activate Search again. Workspace switches clear results, facets, errors, save receipts, and pending deliveries so \
an old workspace or superseded query cannot overwrite the active panel. HBR-INT-009 posture for Notes Search: \
Tier 1 Flight Recorder/EventLedger = WIRED: each successful search records LoomSearchExecuted, while save-as-view \
uses the canonical transactional view-definition mutation receipt and Flight Recorder outbox. Tier 2 \
internal_diagnostics = WIRED at the shared backend-health boundary through typed BackendUnreachable and \
BackendRecovered edges plus the advancing UI heartbeat; a Notes-Search-specific route-failure event code is \
DEFERRED-with-reason because none is registered. Tier 3 Palmistry = WIRED through the shared diagnostic ring for \
freeze/crash survival; it has no Notes-Search-specific payload or tracker. \
The current durable managed proof is run from src/frontend/handshake_native after setting HSK_TEST_BACKEND_BIN to \
the current-source external handshake_core executable, HANDSHAKE_TEST_PG_DSN to a real PostgreSQL DSN, and \
HANDSHAKE_ARTIFACTS_ROOT to the allocated external artifact root: cargo test --features integration --test \
test_loom_search_v2 loom_search_v2_managed_mounted_search_facet_save_reload_cleanup -- --exact --nocapture. \
That command proves the mounted live search/save transport and persisted definition reload; it is not canonical \
Argus reopened-view closure. (2) Find in Files is workspace-wide text search + \
replace. The always-available route is VIEW > Open Find in Files (author_id menu.view.open-find-in-files) or \
the command-palette command view.find-in-files; both open PaneType::FindInFiles without requiring editor focus. \
EDIT > Find in Files (author_id menu.edit.find-all, Ctrl+Shift+F; command editor.find.findInFiles) is the \
editor-context route and is enabled only while a focusable code or rich editor is active. Enter text in \
find-in-files.query; optionally use find-in-files.kind-filter, find-in-files.tag-filter, \
find-in-files.path-filter, find-in-files.toggle-case, find-in-files.toggle-word, or \
find-in-files.toggle-regex; then activate find-in-files.search. Replacement text uses find-in-files.replace; \
the destructive controls are find-in-files.preview-replace, find-in-files.apply, and find-in-files.cancel. \
Bookmark Search is find-in-files.save-bookmark. Operator-visible terminal state is exposed at \
find-in-files.status and bookmark lifecycle state at find-in-files.bookmark-status. These controls and \
bookmarks are panel-local; no dedicated Settings preference is required. The panel follows every page from \
GET /workspaces/{workspace_id}/loom/graph-search \
(q, limit, offset, source_kinds, tag_ids, path, case_sensitive, whole_word, regex), rejects malformed producer \
payloads, and lists the complete bounded result set with stable reversible targets. The actual row author_id is \
find-in-files.result.{hex(source_kind UTF-8 bytes)}.{hex(ref_id UTF-8 bytes)}: it is hex-encoded, \
each byte is lowercase two-digit hex, decoding is exact, and a no-context model should discover dynamic row \
ids with argus.inspect (legacy list_widgets is secondary) instead of guessing them. Exact fixtures: source_kind=document with \
ref_id=KRD-1:/foo?x=1 becomes \
find-in-files.result.646f63756d656e74.4b52442d313a2f666f6f3f783d31; source_kind=文档 with \
ref_id=résumé/東京 becomes \
find-in-files.result.e69687e6a1a3.72c3a973756dc3a92fe69db1e4baac. The ten production destinations are exact: document \
(result_kind knowledge_entity) opens the native Rich Note at PaneType::LoomWikiPage; loom_block, file, and \
tag_hub open PaneType::LoomBlock; symbol (knowledge_entity) opens PaneType::CodeSymbol; work_packet \
(knowledge_entity) opens PaneType::KernelDcc at WP:{wp_id}; micro_task (knowledge_entity) opens \
PaneType::KernelDcc at MT:{wp_id}:{mt_id}; user_manual_page opens PaneType::UserManual at page_slug; \
wiki_page opens the dedicated Wiki Page projection placeholder pane and never PaneType::LoomWikiPage; and \
a loom_block whose block.content_type is view_def opens the mounted Block Collections pane. Result navigation \
retains the exact origin pane and workspace; the host rejects a queued click after that workspace or pane is gone \
instead of retargeting mutable global focus. \
Bookmark Search persists the exact query/filter/options blob through \
GET/PUT /workspaces/{workspace_id}/search-bookmarks and rejects workspace-mismatched or partial responses. \
The producer bookmark id is bookmark-v1 followed by one .{utf8_len}-{hex(component UTF-8 bytes)} frame for \
each exact semantic component in order: trimmed query, kind, trimmed tag, trimmed path, case, whole-word, and regex. \
The bytewise codec never lowercases semantic content, so case-sensitive Foo/foo and Unicode-only 文/東 searches \
remain distinct saved rows instead of evicting one another. \
Every saved row exposes \
find-in-files.bookmark-restore.{hex(bookmark_id UTF-8 bytes)} and \
find-in-files.bookmark-remove.{hex(bookmark_id UTF-8 bytes)}; bookmark id saved:文/1 becomes the exact \
suffix 73617665643ae696872f31 on both routes. A failed mount-time GET exposes \
find-in-files.bookmark-retry; Retry reissues the bounded GET for the active workspace, and another failure \
returns to the visible Retry state. Restore repopulates query, kind, tag, path, case, whole-word, and regex; \
Remove persists the shortened list, so a fresh panel mount must not rediscover the removed row. \
For replacement, enter find-in-files.replace, activate find-in-files.preview-replace, inspect each document's \
before/after preview and match count, then activate find-in-files.apply. Each preview row is addressed as \
find-in-files.preview.{hex(document_id UTF-8 bytes)} using the same lowercase bytewise codec; for example \
KRD-文/1 becomes find-in-files.preview.4b52442de696872f31. Its expanded exact-content nodes are \
find-in-files.preview-before.{hex(document_id UTF-8 bytes)} and \
find-in-files.preview-after.{hex(document_id UTF-8 bytes)}. Preview loads only KRD- rich documents; \
Apply PUTs each /knowledge/documents/{id}/save sequentially with the previewed expected_version and changes \
only text plus attrs.code strings while preserving all other JSON nodes. Preview requires the loaded KRD id \
and workspace_id to match the active workspace, carries that workspace authority in every plan, and reloads \
the document to reverify both identities immediately before save. A version conflict never overwrites \
the newer document. Full and partial mutation outcomes retain per-document before/after SHA-256 audit rows; \
every usable save_receipt_event_id is nonblank, and a committed save whose EventLedger receipt failed is \
reported explicitly as CommittedWithoutReceipt with receipt_error instead of inventing an id. After any \
committed full or partial Apply, the panel automatically re-runs the same search so visible results are current. \
Cancel is cooperative between saves: it keeps the old workspace-attributed operation active across a workspace \
switch until the in-flight save reports, preserves all already committed receipts, blocks another destructive \
Apply, and reports how many plans were skipped. Search never overlaps Apply; a Search intent during Preview \
first detaches that read-only preview and requires Search again, preventing same-input completion reordering. \
If a query/filter/replacement or workspace generation changed, run Search then Preview Replace again; stale \
results/plans are blocked. On regex or malformed-response failure, correct the input and run a fresh Search. \
After backend loss, conflict, partial success, or unknown response loss, do not reuse or blindly retry the old \
preview: restore the backend; preserve and read every visible audit row/save receipt; reload each affected \
document to distinguish committed originals from unchanged documents; run a fresh Search; run a fresh Preview \
Replace; inspect the new before/after plan; and Apply only that fresh plan. Never assume an unseen save rolled \
back. Diagnostic posture: Flight Recorder / EventLedger = WIRED at Tier 1: every graph-search page emits \
LoomSearchExecuted, bookmark mutation returns an event_ledger_event_id, and document save returns a \
save_receipt_event_id or explicit receipt_error. Shared Tier 2 internal_diagnostics = WIRED through typed \
BackendUnreachable/BackendRecovered state plus the advancing UI heartbeat. Search, bookmark load/save, Preview, \
and Apply register and tick the shared BackendCall operation watchdog, so a bounded progress gap or hard runtime \
cap produces the shared StalledOperation diagnostic; no Find-in-Files-specific diagnostic event code is registered. \
Shared Tier 3 Palmistry = WIRED through the diagnostic ring for freeze/crash survival; \
no Find-in-Files-specific payload or tracker is registered. Managed verification self-seeds PostgreSQL, \
drives the mounted factory UI through Search -> result click/open_requests shell target -> a fully backend-loaded \
RichEditorPaneMount with exact id/title/content/version and stable rich-editor root/block ids -> Preview -> Apply, \
proves 501-row pagination, positive and negative tag filters, match options, workspace rejection, stale/conflict/ \
partial/full/cancel-after-first-commit behavior, production Bookmark Search saves for case-sensitive case variants \
and Unicode-only variants -> fresh production remount -> UI Restore-all-fields -> \
UI Remove -> fresh backend absence -> second fresh-remount absence bookmark lifecycle, bounded bookmark-load Retry, \
bounded backend-loss recovery, receipt integrity, and a real \
PNG render, then proves cleanup with fresh GET /workspaces list absence and a failed graph-search refetch. (3) The Quick Switcher \
(quick-switcher.dialog, input quick-switcher.search, list quick-switcher.list, Ctrl+P) jumps between open \
documents, blocks, and code symbols; the Command Palette (command-palette.dialog / command-palette.search / \
command-palette.list, Ctrl+Shift+P) runs any registered command including the View: * surface-open commands. \
All results resolve through handshake_core (PostgreSQL/EventLedger); nothing is read from a local database \
directly."
        .to_owned()
}

fn wikilinks_backlinks_body() -> String {
    "Wikilinks tie notes together the Obsidian way. Type [[ in the Rich Text Editor to open the wikilink \
autocomplete (seeded from the Loom title index via GET /loom/graph-search), pick a target, and a resolvable \
link chip is inserted; a link to a title that does not exist yet offers create-from-unresolved, which POSTs a \
  new note through the knowledge create backend. If multiple notes share the same normalized title or alias, the chip \
  shows an explicit N-matches ambiguity badge and disables both navigation and create; choose an exact note \
  identity instead. Alias resolution is explicitly local-only and in-session: add_local_alias is the sole alias \
  source because the backend enumeration has no aliases field. A restart or fresh resolver seed restores titles but \
  cannot restore aliases; re-enter aliases for the new session. Duplicate-alias ambiguity therefore protects the \
  current in-memory resolver only and is not durable backend alias coverage. Create completion is shell-owned: \
  switching or hiding the originating document does not lose success navigation or failure status, and a late \
  completion never rewrites the newly mounted document. The backend created flag is retained end-to-end: operator \
  status says Created only when created=true, and Opened existing/reused when created=false. Clicking \
  an unambiguous wikilink chip navigates to its target through the \
MT-030 ShellNavigator (open_document / open_loom_block). Code references are the code branch of the same \
hsLink atom. An operator types /code-ref to open code-symbol-search, enters a query through \
code-symbol-search-input, and selects code-symbol-result-{symbol_entity_id}; the result inserts \
code-ref-chip-{symbol_entity_id}. A model first uses argus.inspect, then either drives that dialog with \
argus.set_value plus argus.click or creates the exact same hsLink through \
argus.click{target:'editor.rich.insert-slash-command',payload:{kind:'wikilink',ref_kind:'code',\
ref_value:'<symbol_entity_id>',label:'<display_name>'}}. It must verify the attributed receipt and perform a \
fresh argus.inspect before continuing. Clicking code-ref-chip-{symbol_entity_id} dispatches \
open-code-symbol / CMD_OPEN_CODE_SYMBOL through dispatch_code_ref_open. Hand-authored \
[[code:path/to/file.rs#MyStruct]] carries path#Symbol in the same ref_value; when the live shell drains \
take_pending_code_symbol in HandshakeApp::drive_ckc_interop, ShellNavigator::open_code_symbol resolves \
entity ids through GET /knowledge/code/symbols/{symbol_entity_id} and resolves path#Symbol refs through \
lookup_symbols_by_name_path / GET /knowledge/code/symbols?workspace_id=&name=&path=&limit=1, then loads \
the canonical readable file path derived from symbol_key into the mounted Code Editor and scrolls until the \
visible line range contains line_start; source_id is opaque provenance and is never treated as a filesystem \
path. Every served symbol must carry staleness. fresh=false or a missing staleness projection stops navigation \
with typed stale_source status and the recovery instruction re-index before navigation; it never silently opens \
the last persisted span. A never-existing or deleted symbol remains a distinct typed unresolved result. \
Pane-addressed code-location navigation retains the requested pane and exact byte offset through \
  asynchronous symbol resolution; resolver generations, pending state, and disk-load invalidation are scoped by \
  origin pane plus source content. A new B intent cancels B-old only, while pending A can still land at A's exact \
  pane and byte; changing focus while resolution is in flight cannot redirect or replace either target. Explorer \
  document rows come from GET /knowledge/documents?workspace_id=..., so Rename carries the displayed KRD id and \
  updated_at token to the same RichDocument authority. Rename disables reentry while its operation is in flight; \
  cancel/reopen and reverse completion cannot apply an older operation to the current dialog. A deleted or \
no-definition symbol renders an \
unresolved chip and surfaces a typed navigation/backend status instead of crashing. The Code Editor reverse \
edge is NoteRefsPanel: note-refs-panel lists rich documents mentioning the current symbol, keeps block_id as \
the matched hit identity, rows are dynamic note-ref-{document_id} ListItems, and clicking a row dispatches \
CMD_OPEN_DOCUMENT with document_id through the shared InteractionBus so the shell opens the mounted Notes \
pane. The canonical model flow is argus.inspect -> argus.click the exact create target -> fresh inspect of the \
chip -> argus.click the exact chip -> fresh inspection of the canonical code text and note-refs-panel -> \
argus.click the exact note-ref-{document_id} row -> fresh inspection of editor.rich.text in that document. \
Every action must retain its caller attribution and terminal receipt; do not reuse a pre-action tree as proof. \
The live persistence proof saves the exact hsLink through /knowledge/documents/{id}/save, reloads it, rejects a \
stale expected_version with HTTP 409 while proving committed content unchanged, restarts only the fixture-owned \
current-source backend on the same listener, and freshly reads back the document, symbol, and exact reverse \
lookup. It then deletes the indexed source, re-indexes, proves marked_stale/fresh=false blocks navigation, and \
separately proves a missing symbol remains unresolved. Open the mounted Loom navigation sidebar from EDITORS > Open Sidebar \
(menu.editors.sidebar) or the view.sidebar command. Its independent Pins, Favorites, Backlinks, and \
Unlinked Mentions sections load from handshake_core for the active workspace/block; each section keeps its \
own loading/error state and Retry control, so one failed route never disables the other sections. Pin removal \
is ONE atomic POST /workspaces/{workspace_id}/loom/blocks/{block_id}/remove-pin that clears the pin ordinal and \
unpins the block in a single PostgreSQL transaction together with its durable EventLedger receipt, so the partial \
'ordinal cleared but still pinned' state the retired two-call flow risked is impossible; favorite removal uses \
PATCH {favorite:false}. Every removal returns ONE authoritative operation receipt \
(hsk.wp_kernel_012.mt_024.sidebar_mutation_receipt@1) carrying workspace_id, block_id, the post-write \
mutation_revision, the backend outcome and HTTP status, the EventLedger correlation read back through \
GET /kernel/events/aggregates/loom_block/{block_id}, and the final persisted pin-order revision. Both removals \
retain an exact rollback row until that receipt confirms persistence, restore it immediately on failure, show the \
typed section error ABOVE the still-listed rows (a failed removal never hides a pin that is still pinned), and \
refetch server truth. If the runtime/backend is unavailable, the row is not removed and the affected section \
exposes Retry. The durable mt024.sidebar-pin-removal-completion observer terminalizes a model-driven pin removal: \
it reports applied ONLY after the authoritative refreshed PostgreSQL pin list no longer contains the block, and \
reports a typed failure while the exact sidebar.pin.{encoded_block_id}.remove control stays mounted, so a row that \
merely disappears can never be read as success. Each collapsible header sidebar.{section}.header publishes a \
same-target collapse completion and the collapse state lives on the panel itself, not in per-context egui memory, \
so an inspected tree shows the same collapsed/expanded state the operator sees. Backlinks use the dedicated incoming-edge \
route and retain the source title plus edge type; Unlinked Mentions use the active-block textual scan and exclude \
formal backlinks. Clicking any row opens that Loom block and appends its real title to the five-entry breadcrumb \
trail. Model operators can inspect sidebar.pin.{encoded_block_id}, sidebar.favorite.{encoded_block_id}, \
sidebar.backlink.{encoded_block_id}, sidebar.unlinked.{encoded_block_id}, and sidebar.breadcrumb.{index}; backend \
ids made only from letters, digits, hyphens, and underscores remain literal; unsafe ids and the reserved u8- prefix \
use injective u8-hex encoding. The readable kind chip beside each pin/favorite title exposes content_type. There is \
no sidebar preference in the WP contract: visibility is \
the persisted pane/menu state, not a second settings toggle. The Outgoing Links pane (outgoing.panel) lists the \
active note's links bucketed into outgoing.section.resolved and outgoing.section.unresolved; clicking a \
resolved row jumps to that document/block. Backlinks (which notes point AT this one) surface through the same \
knowledge routes. The reused MT-015 backlinks panel emits backlinks-panel and backlinks-refresh, and each \
loaded row emits a clickable Role::ListItem named backlink-{source_document_id}; the panel is Role::List. \
The production client supplies x-hsk-actor-id, x-hsk-kernel-task-run-id, and x-hsk-session-run-id; a 404 is \
an empty projection shown as No backlinks yet, while network/server failures stay typed and retryable. \
Backlink completions are generation plus workspace/document stamped, so refreshes and context switches cannot \
apply stale or cross-workspace rows. Transclusion, autocomplete, resolver-seed, and create-note completions use \
the same context boundary; all four completion paths are queued so concurrent or reverse completion order cannot \
lose the current result. Create-note guards are keyed by workspace plus normalized title and survive A -> B -> A \
workspace navigation; every completion clears its originating guard, and a successful off-workspace create is cached \
so returning to that workspace resolves the title without issuing a duplicate create. Resolver seeding has one guard \
for the current workspace: document navigation preserves it, while a workspace switch clears it and rejects the old \
workspace's seed completion. Transclusion responses must also echo the requested workspace and block identity. A successful document save broadcasts a projection invalidation to every already-mounted \
backlink panel. If the document commits but server backlink indexing reports an error or skip, every mounted panel \
shows a sticky typed indexing failure instead of presenting stale rows as current. A read-only Backlinks Refresh \
cannot hide or repair that write-time failure; the warning clears only after a later save reports successful backlink \
indexing for the same source document. Invalidation state is workspace-stamped and retains a bounded revision window \
with a current-warning snapshot plus the latest revision per workspace, so hidden panes still refresh after queue \
eviction and another workspace or document's successful save cannot overwrite an unobserved warning. \
Clicking a row dispatches interop.open-document \
(CMD_OPEN_DOCUMENT) via EditorEvent::BacklinkActivated -> dispatch_backlink_open, stages pending_navigation \
on the shared InteractionBus, and the live shell drain HandshakeApp::drive_ckc_interop routes that target \
through ShellNavigator::open_document into the mounted Notes pane. Everything-is-a-block addressing is \
loom://{workspace_id}/{block_id} through loom_address.rs; \
canvas placements with placed_block_id show the full wrapped loom:// chip, graph nodes expose loom:// plus backlink count, \
and content_hash is read from the backend LoomBlock / ContentHash::from_backend rather than PATCHed by the \
client. The managed proof command cargo test --manifest-path src/frontend/handshake_native/Cargo.toml \
-p handshake-native --features integration --test test_loom_address \
live_pg_self_seeded_loom_block_backlink_hash_and_ui_proof -- --exact --nocapture --test-threads=1 creates A \
and B in Handshake-managed PostgreSQL, creates/removes/restores A -> B only through normal saves, loads B \
through ReqwestWikilinkBackend/WikilinkRuntime, deletes A, and compares the fresh RichDocument/LoomBlock \
identity plus backend-computed content_hash. It writes the strict live visual to \
Handshake_Artifacts/handshake-test/wp-kernel-012-mt-032/MT-032-canvas-live-B.png. When the backend omits a Loom \
block id or content_hash the proof reports that typed backend-shape gap, not green. HBR-INT-009 posture for \
this editor navigation: Flight Recorder/EventLedger = NOT_APPLICABLE-with-reason for local read-only tab \
navigation and backlink refresh, which do not mutate authority; save-time backlink mutation has Tier 1 \
Flight Recorder/EventLedger = WIRED through KnowledgeRichDocumentSaved, its \
save_receipt_event_id, and backlink persistence in the normal knowledge-document save. MT-034 has the same \
split: code-symbol navigation and reverse lookup are read-only, so Tier 1 Flight Recorder/EventLedger = \
NOT_APPLICABLE-with-reason; the RichDocument save is Tier 1 WIRED through \
KnowledgeRichDocumentSaved/save_receipt_event_id. CodeNavClient and FindNotesHttp failures remain typed and \
operator-visible, but MT-034 Tier 2 internal_diagnostics = DEFERRED-with-reason because neither client \
registers an MT-034-specific operation watchdog or diagnostic event. MT-034 Tier 3 Palmistry = \
DEFERRED-with-reason because no code-reference-specific survivor payload or ring registration exists. Do not \
infer either feature-specific diagnostic tier from generic app health. For the separate backlinks-list client, \
Tier 2 internal_diagnostics = WIRED through the shared BackendCall operation watchdog registered by \
ReqwestWikilinkBackend::list_backlinks; a bounded progress gap emits the typed StalledOperation diagnostic \
surfaced through the shared diagnostic status. Tier 3 Palmistry = WIRED through the shared process-global \
diagnostic ring, which retains the last-N typed events for the external watcher across a UI freeze or crash \
without inventing a Loom-specific tracker. All link/backlink data \
lives in handshake_core (PostgreSQL/EventLedger) via the Loom + knowledge-documents routes. A swarm agent reads \
the panel with argus.inspect and follows a link with argus.click{target:'outgoing.section.resolved'} (or the \
specific backlink-{source_document_id} row id), then requires the attributed receipt and a fresh inspection."
        .to_owned()
}

fn daily_journal_body() -> String {
    "The Daily Journal is the date-addressed note surface. Open it from VIEW > Open Daily Journal \
(menu.view.open-daily-journal), the command palette command view.journal, the left rail Notes button, or an \
Agenda drawer card; the real app route mounts PaneType::LoomDailyJournal and exposes the editor root \
journal-panel-root. The MT-067 calendar strip remains addressable as daily-journal-panel. Settings > Appearance > Calendar timezone stores a \
validated IANA tzid per workspace (defaulting to the system IANA timezone); changing it invalidates the old \
request generation so a late response from the previous timezone cannot replace the visible day. Today is \
derived in that persisted view timezone on both the outer and embedded date navigators, not from the process \
timezone. The strip: \
daily-journal-date-header selects a day, daily-journal-calendar-event-chip opens the exact CalendarEvent through \
the named loom.daily-note.focus-calendar-event InteractionBus command, and daily-journal-activity-strip shows a \
read-only day overview. Previous/next/calendar navigation reloads the selected day's journal, CalendarEvent, and \
ActivitySpans as one workspace-plus-date-plus-view-timezone request; rapid navigation discards late responses from an older day, so \
the visible chips cannot fall back to today or be overwritten by a stale request. The content-addressed Calendar \
Event destination exposes Details, Notes, and Activity \
tabs as calendar-event-tab-details, calendar-event-tab-notes, and calendar-event-tab-activity. Details shows the \
exact event id/title/start/end; Notes opens the primary linked document without mutating the event; Activity shows \
the nested exact correlated spans and edited-document links. If activation does not open calendar-event-pane, \
retry from the still-visible journal event chip; the one-shot typed bus payload prevents a prior event id from \
being reused. The MT-019 editor surface opens or creates the \
selected day's Loom journal block with PUT /workspaces/:workspace_id/loom/journals/:date, loads a linked \
RichDocument from GET /knowledge/documents/:id when the block has document_id, offers journal-start-writing \
when no document exists, and saves through PUT /knowledge/documents/:id/save after the 3-second idle debounce \
or Ctrl+S. Knowledge-document load/create/save calls carry the x-hsk-* document headers. The managed-backend \
proof starts with the navigated date absent, drives the mounted next-day control, and verifies that the UI creates \
one durable journal identity that reopens idempotently. For a multi-day event, each selected date owns its \
session's exact journal binding; navigating to day two never retains the start day's document id. The outer Calendar nav uses daily-journal-prev-day, \
daily-journal-next-day, daily-journal-today, daily-journal-calendar-toggle, and daily-journal-date-display; the \
embedded editor keeps the distinct journal-* ids, so agents never select a control by tree order. If Start writing \
creates a rich document but the backend cannot durably attach it to the Loom block, the editor keeps the session \
document visible and surfaces journal-document-link-gap instead of pretending the block has a persisted \
 document_id. Timed events preserve UTC instants, IANA tzid, original local wall times, floating-time \
provenance, and any explicit DST-overlap normalization note; all-day events preserve date-only \
`[start_date, end_date_exclusive)` boundaries. Selected local dates are converted to UTC half-open windows \
with the IANA timezone database, so Europe/Brussels 23-hour and 25-hour days, near-midnight events, timed \
midnight-exclusive endings, and multi-day events bind to the visible date instead of a UTC calendar date. \
Invalid IANA zones, contradictory typed payloads, and nonexistent DST-gap local times are rejected; ambiguous \
DST-overlap times retain the explicit earlier/later-offset normalization note shown with event details rather \
than being silently coerced. Such events expose the stable daily-journal-calendar-normalization-badge and \
calendar-event-normalization-badge addresses. Historic rows missing authoritative local/date intent remain \
visible as typed Legacy temporal data with daily-journal-calendar-legacy-badge and \
calendar-event-legacy-badge; reimport from the calendar source is the recovery path and the UI never guesses \
missing intent. An ActivitySpan without ended_utc is rendered as In progress. The CalendarEvent chip and \
activity strip use the live /calendar/events and \
 /calendar/activity-spans routes. The mounted workspace-plus-date request retries its idempotent journal PUT and \
 Calendar GETs at most three times only for transport failures or HTTP 408/425/429/5xx. Every attempt remains \
 generation/date guarded, no intermediate failure replaces the selected day, and success emits one accepted set of \
 binding/correlation receipts. Before the journal binding is ready the CalendarProjectionState is \
 WaitingForDailyNote; journal failure becomes DailyNoteError and prevents Calendar reads. A successful empty event \
 list becomes NoEvent rather than an endless Loading state. Terminal Calendar failures retain their exact \
 CalendarReadFailure: 404/501 is EndpointUnavailable, malformed JSON is InvalidResponse, an exhausted transient \
 budget is RetryExhausted, and other terminal failures are RequestFailed. They emit no binding/correlation success; \
 restore the backend and navigate away and back to issue a fresh selected-date request. An ActivitySpan failure preserves the event chip and its selected-date \
document binding, marks only the activity strip unavailable, and emits no correlation success; neither failure \
is rendered as an authoritative empty event or zero-span result."
        .to_owned()
}

fn diff_and_merge_body() -> String {
    "The Diff and Merge editor shows VS Code-style side-by-side and inline diffs, plus a three-pane \
base/local/remote merge view with Accept Local, Accept Remote, and Accept Both buttons. Open it by running \
'View: Diff/Merge' from the Command Palette (command id view.diff-merge), clicking VIEW > Open Diff Editor \
(menu.view.open-diff-editor), or clicking the conflict dialog's Open merge button (conflict-open-merge). When the mounted document's \
SaveManager is sitting in a save CONFLICT (the local buffer versus the server revision — the two real buffers \
the shell holds), the pane constructs and shows that real diff; otherwise it opens on an HONEST empty state \
('open one from a conflict dialog or the palette') rather than pretending to have a diff. Side-by-side diff \
scrolling follows the live user-scrolled pane through the diff line map, diff tints are positioned from the \
current visible top line, and inputs over 10k lines dispatch their line diff through a background worker before \
publishing blocks back to the pane. Visual proof must be real screenshot/pixel evidence; a missing GPU render is \
not accepted as a passed screenshot proof. Resolving a conflict \
reloads the newer revision and re-saves through PUT /knowledge/documents/:id/save on handshake_core \
(PostgreSQL/EventLedger). This is the native equivalent of a VS Code diff/merge view; it never writes to a \
database directly."
        .to_owned()
}

fn internationalization_body() -> String {
    "Internationalization (i18n, the E13 text_intl layer, MT-077/078) is the SINGLE shared Unicode text-mechanics \
module both editors reuse — it is never duplicated per editor, and it is pure logic (no egui, no backend, no \
color). It corrects three things a naive editor gets wrong: (1) grapheme-cluster caret movement (UAX#29) so \
Left/Right/Backspace cross a WHOLE user-perceived character — a family ZWJ emoji, a combining accent, a \
regional-indicator flag, or a Hangul syllable is never torn in half; (2) CJK + Korean-Hangul line breaking \
(UAX#14 + the kinsoku 'no break after an opening bracket' rule) so spaceless ideograph runs wrap correctly; \
and (3) Unicode-correct word/character counts (a family emoji counts as one character, a Chinese sentence \
counts words by UAX#29, not by whitespace). Right-to-left and bidirectional text (MT-078) is reordered ONLY at \
render/caret time; the document rope stays in LOGICAL order, so the handshake_core backend round-trip is \
byte-for-byte unaffected. No operator action is needed — the correct behavior is automatic in both the Code \
Editor and the Rich Text Editor."
        .to_owned()
}

fn menu_bar_and_commands_body() -> String {
    "The operator menu bar has eight top-level dropdowns, each a stable AccessKit MenuItem: FILE \
(menu-file, Alt+F), EDIT (menu-edit, Alt+E), VIEW (menu-view, Alt+V), GO (menu-go, Alt+G), RUN \
(menu-run, Alt+R), HELP (menu-help, Alt+H), EDITORS (menu-editors, Alt+I; E remains Edit's mnemonic), \
and OPERATOR (menu-operator, Alt+O). \
FILE opens/creates and persists documents: New Document (editor.file.new), Save \
(menu.file.save, Ctrl+S), Save All, Save As, Export Document HTML/MD/TXT/JSON, Close Tab, Quit. EDIT drives the \
focused editor: Undo/Redo (menu.edit.undo / menu.edit.redo — the ONE MT-035 unified stack shared with the \
keyboard and the pane header undo-count-{pane_id} indicator), Cut/Copy/Paste/Select All, Toggle Comment, Format Document, Find (Ctrl+F), Replace (Ctrl+H), Find \
in Files (menu.edit.find-all, Ctrl+Shift+F), Replace in Files, plus Command Palette and Quick Switcher. GO \
    navigates: Quick Switcher (menu.go.quick-switcher, Ctrl+P; palette row \
  command-palette.option.hs-editor-menu-quick-open, command id workbench.action.quickOpen), Command Palette \
  (menu.go.command-palette, Ctrl+Shift+P), Next/Previous Pane, Go to Next/Previous Problem (F8 / Shift+F8), \
  Back/Forward (Alt+Left / Alt+Right), Go to Symbol in File (Ctrl+Shift+O), Go to Definition (F12), Go to \
  References (Shift+F12), Go to Symbol in Workspace (Ctrl+T), and Go to Line (Ctrl+G). VIEW opens mounted \
  native-editor surfaces directly \
  from the Open Editor Surfaces section: menu.view.open-code-editor, menu.view.open-rich-note, menu.view.open-wiki-projection, \
  menu.view.open-knowledge-graph, menu.view.open-folders, menu.view.open-tags, menu.view.open-block-collections, menu.view.open-canvas, menu.view.open-loom-search, \
  menu.view.open-find-in-files, menu.view.open-quick-switcher, menu.view.open-daily-journal, \
  menu.view.open-diff-editor, and menu.view.open-runtime-chat (the MT-098 editor+chat work surface). The pane-opening entries dispatch the same open command ids the Command Palette uses \
  (view.code-editor, view.rich-note, view.wiki-projection, view.graph, view.folders, view.tags, view.block-collections, view.canvas, view.loom-search, view.find-in-files, \
  view.journal, view.diff-merge, view.runtime-chat). Quick Switcher uses menu.view.open-quick-switcher to open the same overlay \
  reached by the palette command workbench.action.quickOpen; the palette rows are \
  command-palette.option.hs-view-palette-code-editor, command-palette.option.hs-view-palette-rich-note, command-palette.option.hs-view-palette-wiki-projection, \
  command-palette.option.hs-view-palette-graph, command-palette.option.hs-view-palette-folders, \
  command-palette.option.hs-view-palette-tags, command-palette.option.hs-view-palette-canvas, command-palette.option.hs-view-palette-loom-search, \
  command-palette.option.hs-view-palette-find-in-files, command-palette.option.hs-editor-menu-quick-open, \
  command-palette.option.hs-view-palette-journal, and command-palette.option.hs-view-palette-diff-merge. A model can click the menu item or run the palette \
  command and reach the same mounted pane or overlay. EDITORS opens editor-specific side panes and commands: \
  Outline, Relevant Memory, Outgoing Links, Stage, Route selection to Stage, Capture and embed from Stage, Sidebar, Daily Journal, Format \
  Document, Next/Previous Diagnostic, Rename Symbol, and Quick Fix. RUN launches operational surfaces: Open Swarm Board, \
Open Inference Lab, Open Flight Recorder, Launch Model Session in Workspace Folder \
(menu.run.model-session-launch), and Open Terminal in Workspace Folder (menu.run.terminal). HELP opens the \
User Manual, Settings, and About. Every enabled item dispatches its REAL command by id through the one shell \
dispatcher and every disabled item is honestly greyed with a reason (no lying-enabled entries). OPERATOR is \
the consolidated operational route: menu.operator.command-palette opens command-palette.dialog; \
menu.operator.swarm-board selects the Swarm pane; menu.operator.flight-recorder opens flight-recorder-pane; \
menu.operator.model-session-launch opens model-session-launch.dialog; menu.operator.user-manual opens \
manual-pane; and menu.operator.settings opens settings.dialog."
        .to_owned()
}

fn editor_settings_body() -> String {
    "Editor preferences live in the Settings dialog (open from HELP > Open Settings, the command settings.open, \
or the settings chrome; filter with settings.search). The Editor section exposes settings-editor-font-size, \
settings-editor-tab-size, settings-editor-insert-spaces (tabs vs spaces), settings-editor-word-wrap (Off / On / \
Bounded column, with settings-editor-wrap-column), settings-editor-render-whitespace (None / Boundary / All), \
settings-editor-minimap, settings-editor-sticky-scroll, settings-editor-line-numbers, \
settings-editor-line-height, settings-editor-bracket-matching, settings-editor-indent-guides, and \
settings-editor-reading-mode-default. The reading-mode preference seeds freshly opened notes; an explicit \
per-document Edit/Reading choice remains the active document choice. The read-only \
settings-editor-wiki-projection-posture row explicitly states that Wiki Projection has no dedicated preference; \
it uses the active workspace/theme and generated content remains read-only, so there is no invented second setting. \
The settings-editor-flight-recorder-posture row likewise states that Flight Recorder has no dedicated preference: \
its workspace filter is runtime-derived from the active shell binding and the Tier-1 audit surface \
cannot be disabled from Editor Settings. The Syntax section is \
settings-syntax-palette-mode (Muted / Standard / Custom) with one settings-syntax-swatch-<scope> color picker \
per highlight scope in Custom mode. The Keybindings section extends the editor bindings with one \
settings-keybind-row-<action_id> per code chord and every rich formatting command, plus \
settings-keybind-reset-<action_id> to restore its built-in default. Custom code and rich chords replace that \
action's default in the mounted editor immediately; an invalid stored rich chord is rejected while its working \
default remains available. Editor preferences persist through canonical PreferenceRecord ids, not an opaque \
workspace-settings blob: scalar editor values use view-defaults.editor.font-size, tab-size, insert-spaces, \
word-wrap, word-wrap-column, render-whitespace, minimap-enabled, sticky-scroll, line-numbers, line-height, \
bracket-matching, indent-guides, and reading-mode-default; syntax uses view-defaults.editor.syntax-palette-mode \
and view-defaults.editor.syntax-custom-colors; keybindings use view-defaults.editor.keybinding-overrides. The \
runtime writes them with PUT /workspaces/:id/preferences/:pref_id, restores defaults with \
POST /workspaces/:id/preferences/:pref_id/reset, and lists them back on reopen through the preferences API. The \
Settings rows settings-editor-prefs-reset and settings-syntax-palette-reset reset their whole Editor and Syntax \
groups through that same PreferenceRecord route; there is no second editor-settings store. \
PROVENANCE: every editor control renders a settings-pref-source-<preference_id> chip beside its value \
showing the resolved PreferenceRecord source and revision, reading 'default \u{b7} rev 0' while the value is the \
registry default, 'custom (operator) \u{b7} rev N' once an operator write has been committed, and \
'not resolved yet' before the workspace preference projection has been read. The chip is a read-only display \
projection of the canonical record (it is refreshed from the preferences GET and from each set/reset response); \
it is never a second settings authority and it cannot be edited. Read it with list_widgets on \
settings-pref-source-view-defaults.editor.font-size (and the matching id for any other preference) to see \
whether a value is customised and at which revision before changing or resetting it. \
LIVE-EFFECT STATE: tab/indent settings, wrap, whitespace, minimap, sticky scroll, line-number visibility, \
line height, bracket matching, indent guides, code keybindings, and rich keybindings apply to the mounted editors \
without restart. Editor font size also applies LIVE to the mounted code editor and rich editor: a font-size change \
resizes the running code rows/glyph advance and rich document text layout on the next frame. Muted, Standard, and \
Custom palette selections repaint the mounted code editor and minimap through the live highlight resolver; Custom \
uses the configured per-scope swatches and missing Custom entries fall back to Standard. Each mode change therefore \
repaints the mounted code editor immediately rather than waiting for restart. Gutter line-number and fold glyphs \
use the same live editor font size, so their geometry changes with the code rows. A failed GET or \
PUT keeps the current in-memory edits, exposes the typed settings.persist.error status, and offers the exact \
settings.persist.retry control to repeat the failed load or save."
        .to_owned()
}

fn signature_rename_quickfix_body() -> String {
    "The code editor has VS Code-parity symbol-intelligence actions on the mounted code pane. Signature help \
is the parameter-hints popup: while typing inside a call's argument list the editor shows the active \
signature and highlights the current parameter (an LSP textDocument/signatureHelp request when a language \
server is attached; a typed graceful absent state otherwise). Rename Symbol (F2, \
CodeEditorAction::RenameSymbol) opens the in-place rename box at the cursor identifier and renames every \
occurrence as one undo step; the code path is the panel's begin_rename_at_cursor, reached from the F2 \
keybind, the code-editor right-click 'Rename Symbol' entry, and the EDITORS menu leaf \
menu.editors.rename-symbol which dispatches command id editor.rename.symbol. Quick Fix (Ctrl+.) requests \
code actions for the cursor range and opens the quick-fix menu; the code path is the panel's \
quick_fix_request flag, reached from the Ctrl+. keybind, the right-click 'Quick Fix...' entry, and the \
EDITORS menu leaf menu.editors.quick-fix which dispatches command id editor.quickFix. Both menu leaves are \
enabled only while the active/focused pane is the mounted code editor (honest enable predicate, never a \
fake-enabled leaf); with no code editor active they render disabled. A no-context model surfaces the \
EDITORS menu (menu-editors), reads the leaves with list_widgets, and clicks menu.editors.rename-symbol or \
menu.editors.quick-fix with click_widget. Nothing here bypasses the mounted panel — the menu and keybind \
share ONE dispatch path (dispatch_editor_command -> code_panel.dispatch_action)."
        .to_owned()
}

fn outline_toc_body() -> String {
    "The Outline (table of contents) is the document-structure side pane. Open it from the EDITORS menu \
leaf 'View: Outline' (menu.editors.outline), the Command Palette option \
command-palette.option.hs-view-palette-outline, or command id view.outline; the mounted surface is built \
by code_editor/outline.rs and lists the headings/symbols of the active document in source order. Click a \
row to navigate the mounted editor to that heading or symbol (the SAME fold-aware navigate-to-line path \
the minimap and go-to-line use), so the outline is a jump index, not just a static list. The code editor \
also has its own inline outline toggle; the view.outline command opens the document outline beside the \
rich editor. A no-context model opens it with click_widget on menu.editors.outline (or dispatches \
view.outline through the palette), reads the rows with list_widgets, and clicks a row to jump. Empty or \
heading-less documents show an empty outline rather than an error. Persistence of the underlying document \
remains handshake_core PostgreSQL/EventLedger; the outline itself is a read-only projection over the live \
document model and stores nothing."
        .to_owned()
}

fn relevant_memory_body() -> String {
    "Relevant Memory is the FEMS (Pillar 12) retrieval side pane. Open it from the EDITORS menu leaf \
'View: Relevant Memory' (menu.editors.relevant-memory), the Command Palette option \
command-palette.option.hs-view-palette-relevant-memory, or command id view.relevant-memory. The mounted \
pane is the relevant-memory-panel container with the relevant-memory-list of retrieved memory items — the \
FEMS retrieval capsule for the active context (typed memory the model has stored: episodic/semantic/\
procedural entries relevant to the current document/task). Opening the pane starts a live MemoryPack read; \
changing workspace/document/selection/cursor context refreshes it automatically and clears the previous \
context's pack before the new read completes. Use click_widget on \
editor.fems.memorypack-refresh to force a same-context retry or refresh. While a request is running the \
Refresh control is disabled; success replaces the capsule, an empty result is shown honestly, and a \
backend failure remains visible until Refresh or a context change completes successfully. The pane is \
READ-ONLY navigation: a memory WRITE is never an editor-direct commit. Select source content, open \
'Propose to Memory' through the Command Palette option \
command-palette.option.hs-fems-palette-propose-to-memory, inspect fems-propose-dialog, choose \
fems-class-episodic, fems-class-semantic, or fems-class-procedural, then activate fems-propose-confirm, \
or cancel with fems-propose-cancel. The result is a pending_review proposal for a \
reviewer, not a committed memory item. Read editor.fems.memorypack-status and fems-propose-status with \
list_widgets: their structured values expose state, refresh generation/count, operation_id, proposal_id, \
event_id, and outcome without scraping visible labels. `state=completed;outcome=event_persisted` means the \
proposal and canonical backend-projected FR-EVT-MEM-001 are both durable; the backend does not acknowledge \
this operation before that projection is durable. FR-EVT-MEM-001 is backend-owned rather than a native-editor \
action and carries event_code, a non-nil UUID proposal_id, the canonical proposal_hash, \
artifact_ref=artifact://sha256/{proposal_hash}, scope_refs, op_count, and requires_review_count. The resolved \
artifact uses schema_version=hsk.memory_write_proposal@0.1; the event contains no raw memory content. A \
transport or backend rejection is reported as a failed \
submit, not as a successful proposal with a frontend-only partial event outcome. Cancel reports \
`state=cancelled;outcome=cancelled_before_submit` with its stable operation_id and leaves both the \
canonical proposal-row count and committed-memory count unchanged. An exact workspace/class/content/source \
replay is one logical proposal identity: retries converge across native-process restarts, while a terminal \
identical proposal remains the same reviewed intent rather than silently creating another row. Change the \
selection, content, class, or source when the operator intends a distinct proposal. Once a durable \
    proposal is pending review, activate fems-review-approve or fems-review-reject. Pending review controls \
cannot be dismissed; after restart or workspace rebinding the shell reloads the bounded canonical pending \
proposal list and restores the next review target. If that queue refresh fails, activate \
fems-review-refresh-retry; new proposal creation remains blocked until the exact workspace queue recovers. \
The native control calls the closed review route \
off-frame with an operator identity; fems-review-status and fems-propose-status expose proposal_id, \
decision, actor_id, correlation_id, event_ledger_event_id, flight_recorder_event_id, and reviewed_at. \
    Missing/conflicted targets and mismatched acknowledgement identities retire the stale control and refresh \
    the canonical queue; transport failures retain the target for explicit retry. Rejection ends at \
    `state=reviewed;outcome=rejected;terminal=true` and performs no commit. Approval first records the review, then calls the \
    separate explicit approved-proposal commit route. A successful approval ends at \
    `state=committed;outcome=approved`; fems-propose-status exposes memory_id, commit_id, memory_pack_id, \
    memory_pack_hash, commit_report_hash, the commit event_ledger_event_id, flight_recorder_event_id, and \
    committed_at; the commit is projected as FR-EVT-MEM-003. That committed strict MemoryPack supersedes an older context-specific empty pack, so the \
    mounted Relevant Memory pane refreshes and renders the exact committed item and provenance. Exact retries \
    of an already approved or rejected decision converge on the original immutable review receipt; an approved \
    retry also returns the original explicit commit receipt. If commit transport fails after approval, retry the \
    same fems-review-approve action for that proposal; the backend does not create a second item, pack, or \
    receipt. A commit accepted by PostgreSQL but interrupted before FR-EVT-MEM-003 is projected is recovered \
automatically by the backend startup projector; the original commit timestamp, pack identity/hash, report \
artifact, and EventLedger receipt are reused. FR-EVT-MEM-003 carries the canonical content-addressed \
artifact_ref=artifact://sha256/{commit_report_hash}; resolve the report through the separate authenticated \
workspace commit-report route and re-hash it to commit_report_hash. FR-EVT-MEM-004 likewise carries \
artifact_ref=artifact://sha256/{memory_pack_hash}, never a host or workspace-relative path. Text-range proposals fail \
    closed unless the active mounted tab supplies canonical same-workspace provenance: rich-text tabs cite their \
    persisted RichDocument id, while code tabs cite the KSRC-* KnowledgeSource bound by production code-symbol \
    navigation (the filesystem path remains a separate navigation key and is never used as provenance). Switching \
    the pane to a different code or rich editor document invalidates the prior text selection, even when both \
    documents contain identical bytes at the identical range. Opening the Relevant Memory utility tab deliberately \
    retains the preceding editor context; returning to that same editor keeps it, while activating another editor \
    document clears it. For code, \
    open Edit -> Quick Switcher (menu.edit.quick-switcher), search an indexed symbol, and activate its result before \
    selecting text. The backend requires a current, non-stale file KnowledgeSource plus its current \
    knowledge_code_files row, then verifies the submitted full-buffer SHA-256, UTF-8 range, and exact selected slice \
    against that canonical indexed source. A pane id, path suffix, or stale source is never substituted as authority. Clicking \
    a memory item's source control navigates to its real Loom block, never a fabricated or dangling source id. \
    While a submit is in flight, running Propose to Memory again reports \
    `state=submitting;outcome=reentry_blocked` and cannot replace the captured operation/emitter. Switching \
    workspaces invalidates the dialog, selection, captured emitter, pending operation, and status; an A -> B -> A \
    round trip cannot revive A's old UI operation. A POST already accepted by the backend can still complete \
    durably in A; A rebind starts canonical pending-queue recovery, and any later stale completion schedules \
    another current-generation queue read. If the first read is still in flight, the second is marked deferred and \
    starts immediately after the first drains, so the late commit cannot hide behind an earlier empty snapshot. Completed workers enter an operation-tagged FIFO, so \
    an old-A and fresh-A result can coexist without overwriting each other; the UI drains both, the late delivery is discarded \
    by operation identity, and the fresh proposal keeps its own proposal_id and event_id. \
Three durable Role::Status observers make every canonical step of that flow CAUSALLY PROVABLE instead of \
Indeterminate. fems-propose-class-state publishes selected_class, proposal_class, the per-class booleans, \
and content_hash, so selecting fems-class-episodic/semantic/procedural is proven by an authoritative \
selection value rather than by an unrelated later state change; each radio also exposes its AccessKit selected state. \
mt064.shared-selection-state publishes the shared-selection pane_id, surface, start, end, len, and loom \
content_hash, so Edit -> Select All (menu.edit.select-all) is proven by the selection CHANGING to the exact \
full-document range even though the menu item itself disappears. \
mt064.fems-proposal-flow-completion is the click-completion observer that acknowledges menu.edit.select-all, \
command-palette.option.hs-fems-palette-propose-to-memory, fems-class-{episodic|semantic|procedural}, and \
fems-propose-confirm; its terminal_detail carries the exact proven post-state (selection range/hash, the \
fresh proposal operation identity plus every required dialog author_id, the selected/previewed class, and \
the confirm proposal_id/event_id). The confirm observer is a SUCCESSOR predicate across target \
disappearance: the button is gone by the time it terminalizes, and it applies only when fems-propose-status \
reports state=completed;outcome=event_persisted for that exact operation_id with non-empty proposal_id and \
event_id. A partial, failed, or blocked terminal status publishes a typed terminal FAILURE instead of \
success, and an unmounted status node or a mismatched operation identity never terminalizes the receipt. \
The GO menu (menu-go) and GO -> Command Palette (menu.go.command-palette) carry the same menu-open and \
command-palette-open completion tokens the OPERATOR entries already use. \
A no-context model can drive panel-open -> refresh -> propose -> confirm entirely through these stable \
AccessKit author_ids and inspect the result with list_widgets + screenshot. Durable memory authority and \
FR-EVT-MEM-001 proposal provenance live in handshake_core PostgreSQL/EventLedger. The workspace-scoped Flight \
Recorder pane exposes that proposal plus exact FR-EVT-MEM-002 review, FR-EVT-MEM-003 commit, FR-EVT-MEM-004 \
pack-build, and FR-EVT-MEM-005 status-transition rows. Diagnostic dispositions for this surface are explicit: \
Flight Recorder/EventLedger=WIRED for the durable proposal/review/commit/pack lifecycle; \
internal_diagnostics=WIRED because proposal submission registers with the shipped backend-operation watchdog; \
Palmistry=WIRED at the shared diagnostic-ring/app boundary for freeze/crash survival, without inventing a \
FEMS-specific child process."
        .to_owned()
}

fn agent_tool_reference_body() -> String {
    "The agent-vision / steering index pairs every addressable editor/knowledge/FEMS/interop action with \
the REAL MCP swarm tool that drives it. The four canonical tools are: argus.inspect{} (discover the live \
AccessKit tree), argus.click{target:<author_id>,payload?:<json|string>} (activate a button/toggle/row), \
argus.set_value{target:<author_id>,value:<string>} (replace a text field's whole value), and \
argus.screenshot{} (capture the pixels without foreground focus changes). The legacy list_widgets, \
click_widget, set_value, and screenshot spellings are compatibility aliases only. Read the structured \
rows in the pane below; each row is author_id -> canonical mcp_tool for a real, live-registered control. \
Every JSON-RPC request must carry the owner-restricted session_token. Parallel clients should also carry \
a stable top-level client_session_id (1..=64 ASCII letters, digits, '-', '_' or '.') so receipts retain \
the same participant identity across reconnects; callers that omit it receive a connection-scoped id."
        .to_owned()
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────────
// Agent-tool reference rows: author_id -> REAL MCP tool, for every addressable surface.
// Every author_id here is cross-checked by the id-audit test against the live registries.
// ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Build the full `author_id -> MCP tool` steering reference. Covers shell chrome, code-editor actions,
/// rich-text actions, graph actions, canvas actions, collection actions, FEMS, and the four interop edges
/// (Stage / Calendar / Locus / FEMS). Every `author_id` is a LIVE registered id (the id-audit asserts no
/// orphan); every `mcp_tool` is a real `mcp/tools.rs` method.
pub fn agent_tool_rows() -> Vec<AgentToolRow> {
    // ── Shell chrome (the panes a swarm agent first reaches) ─────────────────────────────────────────
    // The LIVE command palette (command_palette.rs) emits the DOT-form ids (PALETTE_SEARCH_AUTHOR_ID =
    // "command-palette.search", PALETTE_LIST_AUTHOR_ID = "command-palette.list"), registered in
    // DECLARED_IDENTITIES + PALETTE_AUTHOR_IDS. Source the row author_ids from those consts so the steering
    // index always tracks the id the running app actually exposes (the interop hyphen-form
    // "command-palette-search" is emitted only inside a unit-test harness, never the live render loop).
    let mut rows: Vec<AgentToolRow> = vec![
        AgentToolRow {
            author_id: PALETTE_SEARCH_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Type a command into the palette",
            mcp_tool: "set_value",
            description: "set_value{target:'command-palette.search', value:'<command>'} filters the palette.",
        },
        AgentToolRow {
            author_id: PALETTE_LIST_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Read palette results",
            mcp_tool: "list_widgets",
            description: "list_widgets reveals the command-palette.list rows for the agent to click.",
        },
        AgentToolRow {
            author_id: "manual-search",
            surface: ManualSurface::Knowledge,
            action_label: "Search the manual",
            mcp_tool: "set_value",
            description: "set_value{target:'manual-search', value:'<keyword>'} filters manual topics.",
        },
        AgentToolRow {
            author_id: NOTES_LOAD_RETRY_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Retry the active Notes document load",
            mcp_tool: "click_widget",
            description: "After list_widgets reads notes-document-load-error, click_widget{target:'notes-document-load-retry'} issues one new GET for the still-active document.",
        },
        AgentToolRow {
            author_id: crate::top_menu_bar::MenuId::Editors.author_id(),
            surface: ManualSurface::Code,
            action_label: "Open the EDITORS dropdown",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu-editors'} opens the operator-facing EDITORS dropdown so list_widgets can discover its live leaves.",
        },
        AgentToolRow {
            author_id: crate::top_menu_bar::MenuId::Operator.author_id(),
            surface: ManualSurface::Diagnostics,
            action_label: "Open the OPERATOR dropdown",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu-operator'} opens the consolidated OPERATOR dropdown.",
        },
        AgentToolRow {
            author_id: "menu.operator.command-palette",
            surface: ManualSurface::Diagnostics,
            action_label: "Open Command Palette from OPERATOR",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.operator.command-palette'} opens command-palette.dialog.",
        },
        AgentToolRow {
            author_id: "menu.operator.swarm-board",
            surface: ManualSurface::Model,
            action_label: "Open Swarm Board from OPERATOR",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.operator.swarm-board'} selects the Swarm pane.",
        },
        AgentToolRow {
            author_id: "menu.operator.flight-recorder",
            surface: ManualSurface::Diagnostics,
            action_label: "Open Flight Recorder from OPERATOR",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.operator.flight-recorder'} opens flight-recorder-pane.",
        },
        AgentToolRow {
            author_id: crate::app::MT036_FLIGHT_RECORDER_OPEN_COMPLETION_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read Flight Recorder open completion",
            mcp_tool: "list_widgets",
            description: "list_widgets reads mt036.flight-recorder-open-completion after the canonical menu action mounts flight-recorder-pane.",
        },
        AgentToolRow {
            author_id: "menu.operator.model-session-launch",
            surface: ManualSurface::Model,
            action_label: "Launch Model Session from OPERATOR",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.operator.model-session-launch'} opens model-session-launch.dialog.",
        },
        AgentToolRow {
            author_id: "menu.operator.user-manual",
            surface: ManualSurface::Knowledge,
            action_label: "Open User Manual from OPERATOR",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.operator.user-manual'} opens manual-pane.",
        },
        AgentToolRow {
            author_id: "menu.operator.settings",
            surface: ManualSurface::Diagnostics,
            action_label: "Open Settings from OPERATOR",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.operator.settings'} opens settings.dialog.",
        },
        AgentToolRow {
            author_id: crate::flight_recorder_pane::FLIGHT_RECORDER_REFRESH_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Refresh Flight Recorder",
            mcp_tool: "click_widget",
            description: "click_widget{target:'flight-recorder.refresh'} retries the workspace-scoped ledger read.",
        },
        AgentToolRow {
            author_id: crate::flight_recorder_pane::FLIGHT_RECORDER_LOADING_STATUS_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read Flight Recorder loading state",
            mcp_tool: "list_widgets",
            description: "list_widgets reads flight-recorder.loading-status only while one bounded workspace-scoped GET is in flight.",
        },
        AgentToolRow {
            author_id: crate::flight_recorder_pane::FLIGHT_RECORDER_RETRY_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Retry a failed Flight Recorder load",
            mcp_tool: "click_widget",
            description: "click_widget{target:'flight-recorder.retry'} issues one new bounded workspace-scoped read after a visible failure.",
        },
        AgentToolRow {
            author_id: crate::flight_recorder_pane::FLIGHT_RECORDER_ACTION_COMPLETION_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read Flight Recorder load action completion",
            mcp_tool: "list_widgets",
            description: "list_widgets reads flight-recorder.action-completion for the exact Refresh/Retry action generation, fetch generation, and terminal load result.",
        },
        AgentToolRow {
            author_id: crate::flight_recorder_pane::FLIGHT_RECORDER_LOAD_FAILURE_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read a Flight Recorder load failure",
            mcp_tool: "list_widgets",
            description: "list_widgets reads flight-recorder.load-failure before retrying the same pane.",
        },
        AgentToolRow {
            author_id: crate::flight_recorder_pane::FLIGHT_RECORDER_QUARANTINE_STATUS_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read quarantined Flight Recorder rows",
            mcp_tool: "list_widgets",
            description: "list_widgets reads flight-recorder.quarantine-status and its exact rejected-row reasons.",
        },
        AgentToolRow {
            author_id: crate::flight_recorder_pane::FLIGHT_RECORDER_ERROR_RING_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read recent Flight Recorder emit failures",
            mcp_tool: "list_widgets",
            description: "list_widgets reads flight-recorder.error-ring and flight-recorder.emit-error-{index} rows.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_CODE_EDITOR_MENU_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Open Code Editor from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-code-editor'} opens the mounted native code editor pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_RICH_NOTE_MENU_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Open Rich Note from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-rich-note'} opens the mounted rich Notes editor pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_WIKI_PROJECTION_MENU_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Wiki Projection from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-wiki-projection'} reopens the concrete active Wiki Projection, or opens wiki discovery when none is active.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_KNOWLEDGE_GRAPH_MENU_AUTHOR_ID,
            surface: ManualSurface::Graph,
            action_label: "Open Knowledge Graph from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-knowledge-graph'} opens the mounted knowledge graph pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_FOLDERS_MENU_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Folders from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-folders'} opens the mounted Loom folder-tree pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_TAGS_MENU_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Tags from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-tags'} opens the mounted Loom tags and tag-hub pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_BLOCK_COLLECTIONS_MENU_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Block Collections from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-block-collections'} opens the mounted block-collections pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_CANVAS_MENU_AUTHOR_ID,
            surface: ManualSurface::Canvas,
            action_label: "Open Canvas from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-canvas'} opens the mounted Loom canvas pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_LOOM_SEARCH_MENU_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Notes Search from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-loom-search'} opens the mounted Notes Search pane through the internal view.loom-search route.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_FIND_IN_FILES_MENU_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Open Find in Files from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-find-in-files'} opens the mounted workspace search pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_QUICK_SWITCHER_MENU_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Quick Switcher from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-quick-switcher'} opens the quick switcher overlay.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_DAILY_JOURNAL_MENU_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Daily Journal from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-daily-journal'} opens the mounted daily journal pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_DIFF_EDITOR_MENU_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Open Diff Editor from VIEW",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.view.open-diff-editor'} opens the mounted Diff/Merge editor pane.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_CODE_EDITOR_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Open Code Editor from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-code-editor'} opens the same native code editor pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_RICH_NOTE_PALETTE_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Open Rich Note from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-rich-note'} opens the same rich Notes editor pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_WIKI_PROJECTION_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Wiki Projection from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-wiki-projection'} reopens the same concrete active Wiki Projection, or opens wiki discovery when none is active.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_KNOWLEDGE_GRAPH_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Graph,
            action_label: "Open Knowledge Graph from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-graph'} opens the same knowledge graph pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_FOLDERS_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Folders from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-folders'} opens the same Loom folder-tree pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_TAGS_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Tags from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-tags'} opens the same Loom tags and tag-hub pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_CANVAS_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Canvas,
            action_label: "Open Canvas from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-canvas'} opens the same Loom canvas pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_LOOM_SEARCH_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Notes Search from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-loom-search'} opens the same Notes Search pane after palette filtering; the stable internal row id retains loom-search.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_FIND_IN_FILES_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Open Find in Files from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-find-in-files'} opens the same workspace search pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_QUICK_SWITCHER_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Quick Switcher from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-editor-menu-quick-open'} opens the same quick switcher overlay after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_DAILY_JOURNAL_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Knowledge,
            action_label: "Open Daily Journal from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-journal'} opens the same mounted daily journal pane after palette filtering.",
        },
        AgentToolRow {
            author_id: VIEW_OPEN_DIFF_EDITOR_PALETTE_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Open Diff Editor from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-view-palette-diff-merge'} opens the same mounted Diff/Merge editor pane after palette filtering.",
        },
        AgentToolRow {
            author_id: CONFLICT_KEEP_YOURS_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Resolve a save conflict by keeping the local document",
            mcp_tool: "click_widget",
            description: "click_widget{target:'conflict-keep-yours'} selects the local buffer in the SaveManager conflict dialog.",
        },
        AgentToolRow {
            author_id: CONFLICT_KEEP_SERVER_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Resolve a save conflict by keeping the server document",
            mcp_tool: "click_widget",
            description: "click_widget{target:'editor.rich.conflict.keep-server'} reloads the newer server revision from the SaveManager conflict dialog.",
        },
        AgentToolRow {
            author_id: CONFLICT_OPEN_MERGE_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Open a conflict merge diff from the conflict dialog",
            mcp_tool: "click_widget",
            description: "click_widget{target:'conflict-open-merge'} opens the mounted Diff/Merge editor pane for the current SaveManager conflict.",
        },
        AgentToolRow {
            author_id: CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Confirm keeping the local document after conflict warning",
            mcp_tool: "click_widget",
            description: "click_widget{target:'conflict-keep-yours-confirm'} confirms the secondary keep-yours warning before the retry save.",
        },
        AgentToolRow {
            author_id: DRAFT_BANNER_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Read the draft recovery banner",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces draft-recovery-banner when GET /knowledge/documents/:id/draft returns a recoverable draft.",
        },
        AgentToolRow {
            author_id: DRAFT_RESTORE_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Restore recovered rich-document draft content",
            mcp_tool: "click_widget",
            description: "click_widget{target:'draft-restore'} loads the recovered draft into the mounted editor without canonical-saving it.",
        },
        AgentToolRow {
            author_id: DRAFT_DISCARD_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Discard recovered rich-document draft content",
            mcp_tool: "click_widget",
            description: "click_widget{target:'draft-discard'} clears the recoverable draft through the draft manager.",
        },
        AgentToolRow {
            author_id: RICH_EDITOR_EXPORT_BUTTON_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Open the rich-document export picker",
            mcp_tool: "click_widget",
            description: "click_widget{target:'rich-editor-export-button'} opens the MT-020 export format picker.",
        },
        AgentToolRow {
            author_id: EXPORT_FORMAT_PICKER_AUTHOR_ID,
            surface: ManualSurface::RichText,
            action_label: "Read rich-document export format choices",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces export-format-picker and its HTML/MD/TXT/JSON export choices.",
        },
        AgentToolRow {
            author_id: crate::runtime_chat::RUNTIME_CHAT_PANEL_AUTHOR_ID,
            surface: ManualSurface::Chat,
            action_label: "Read Runtime Chat state",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces the Runtime Chat pane container.",
        },
        AgentToolRow {
            author_id: crate::runtime_chat::RUNTIME_CHAT_STATUS_AUTHOR_ID,
            surface: ManualSurface::Chat,
            action_label: "Read Runtime Chat endpoint status",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces runtime-chat-status with EndpointMissing and the probed route.",
        },
        AgentToolRow {
            author_id: crate::runtime_chat::RUNTIME_CHAT_INPUT_AUTHOR_ID,
            surface: ManualSurface::Chat,
            action_label: "Type a Runtime Chat message",
            mcp_tool: "set_value",
            description: "set_value{target:'runtime-chat-input', value:'<message>'} fills the chat draft.",
        },
        AgentToolRow {
            author_id: crate::runtime_chat::RUNTIME_CHAT_SEND_AUTHOR_ID,
            surface: ManualSurface::Chat,
            action_label: "Send Runtime Chat message",
            mcp_tool: "click_widget",
            description: "click_widget{target:'runtime-chat-send'} is enabled after text is entered and returns EndpointMissing until the backend route exists.",
        },
        AgentToolRow {
            author_id: crate::runtime_chat::RUNTIME_CHAT_CANCEL_AUTHOR_ID,
            surface: ManualSurface::Chat,
            action_label: "Cancel active Runtime Chat request",
            mcp_tool: "click_widget",
            description: "click_widget{target:'runtime-chat-cancel'} aborts the exact active request generation; read runtime-chat-status for Cancelled, then enter a new draft to recover.",
        },
        AgentToolRow {
            author_id: TERMINAL_MENU_AUTHOR_ID,
            surface: ManualSurface::Terminal,
            action_label: "Open terminal launch blocker",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.run.terminal'} records terminal-launch-status as EndpointMissing until a native HTTP /terminal/sessions route exists; current reach is legacy Tauri IPC / IPC-only.",
        },
        AgentToolRow {
            author_id: crate::app::TERMINAL_LAUNCH_STATUS_AUTHOR_ID,
            surface: ManualSurface::Terminal,
            action_label: "Read terminal launch status",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces terminal-launch-status after menu.run.terminal or terminal.open-workspace records the EndpointMissing blocker.",
        },
        AgentToolRow {
            author_id: MODEL_SESSION_LAUNCH_MENU_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Open model-session launch dialog",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.run.model-session-launch'} opens the compact MT-101 launch dialog; it does not claim a running model.",
        },
        AgentToolRow {
            author_id: MODEL_SESSION_LAUNCH_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Open model-session launch from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-model-session-palette-launch-workspace'} opens the same MT-101 launch dialog after palette filtering.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_PROVIDER_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Choose Local or Cloud provider",
            mcp_tool: "click_widget",
            description: "click_widget{target:'model-session-launch.provider'} opens the provider picker; use local/cloud rows for the exact provider.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Choose Local provider row",
            mcp_tool: "click_widget",
            description: "click_widget{target:'model-session-launch.provider.local'} selects the Local provider while the provider picker is open.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Choose Cloud provider row",
            mcp_tool: "click_widget",
            description: "click_widget{target:'model-session-launch.provider.cloud'} selects the Cloud provider while the provider picker is open.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Set model-session workspace folder",
            mcp_tool: "set_value",
            description: "set_value{target:'model-session-launch.folder', value:'<repo folder>'} sets the explicit working folder included in job_inputs.workspace_folder and job_inputs.working_dir.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Set model id",
            mcp_tool: "set_value",
            description: "set_value{target:'model-session-launch.model', value:'<model id or cloud model name>'} sets job_inputs.model_id/backend target; empty values are rejected before POST /jobs.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Set wrapper",
            mcp_tool: "set_value",
            description: "set_value{target:'model-session-launch.wrapper', value:'<wrapper>'} sets the wrapper attribution carried in /jobs and the direct-spawn blocker request.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_START_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Submit model-session launch",
            mcp_tool: "click_widget",
            description: "click_widget{target:'model-session-launch.start'} issues real POST /jobs when fields are valid and records EndpointMissing for direct kernel_swarm_spawn_session spawn.",
        },
        AgentToolRow {
            author_id: crate::app::MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Read model-session launch status",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces model-session-launch-status after launch; it distinguishes POST /jobs job creation from NEEDS_MANAGED_RESOURCE_PROOF and EndpointMissing kernel_swarm_spawn_session.",
        },
        AgentToolRow {
            author_id: INFERENCE_LAB_MENU_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Open Inference Lab from the Run menu",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.run.inference-lab'} opens the current model/inference surface.",
        },
        AgentToolRow {
            author_id: INFERENCE_LAB_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Model,
            action_label: "Open Inference Lab from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-inference-palette-open'} opens Inference Lab after filtering the command palette.",
        },
        AgentToolRow {
            author_id: FLIGHT_RECORDER_MENU_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Open Flight Recorder from the Run menu",
            mcp_tool: "click_widget",
            description: "click_widget{target:'menu.run.flight-recorder'} opens the Tier-1 Flight Recorder pane.",
        },
        AgentToolRow {
            author_id: FLIGHT_RECORDER_PALETTE_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Open Flight Recorder from the command palette",
            mcp_tool: "click_widget",
            description: "click_widget{target:'command-palette.option.hs-flight-palette-open'} opens the Tier-1 Flight Recorder pane after palette filtering.",
        },
        AgentToolRow {
            author_id: crate::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Search Settings for Diagnostics",
            mcp_tool: "set_value",
            description: "set_value{target:'settings.search', value:'diagnostics'} filters Settings to the Diagnostics section.",
        },
        AgentToolRow {
            author_id: SETTINGS_DIAGNOSTICS_SECTION_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Expand Settings Diagnostics",
            mcp_tool: "click_widget",
            description: "click_widget{target:'settings.section.diagnostics'} expands the Settings->Diagnostics section.",
        },
        AgentToolRow {
            author_id: crate::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read Diagnostics panel",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces diagnostics_panel, the Settings-hosted diagnostics region.",
        },
        AgentToolRow {
            author_id: crate::diagnostics::DIAGNOSTICS_HEARTBEAT_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read diagnostics heartbeat",
            mcp_tool: "list_widgets",
            description: "list_widgets reads diagnostics_heartbeat for Tier-2 UI liveness.",
        },
        AgentToolRow {
            author_id: crate::diagnostics::DIAGNOSTICS_FRAME_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read diagnostics frame timing",
            mcp_tool: "list_widgets",
            description: "list_widgets reads diagnostics_frame for slow-frame/p50/p95 timing.",
        },
        AgentToolRow {
            author_id: crate::diagnostics::DIAGNOSTICS_RESOURCE_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read diagnostics resources",
            mcp_tool: "list_widgets",
            description: "list_widgets reads diagnostics_resource for CPU/RSS/GPU state.",
        },
        AgentToolRow {
            author_id: crate::diagnostics::DIAGNOSTICS_EVENTS_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read recent diagnostic events",
            mcp_tool: "list_widgets",
            description: "list_widgets reads diagnostics_events for the Tier-2 last-N event projection.",
        },
        AgentToolRow {
            author_id: crate::diagnostics::DIAGNOSTICS_PALMISTRY_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read Palmistry survivor projection",
            mcp_tool: "list_widgets",
            description: "list_widgets reads diagnostics_palmistry for Tier-3 freeze/crash/child-stall survivor records.",
        },
        AgentToolRow {
            author_id: crate::visual_debugger::WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Dump the visual-debugger worksurface JSON",
            mcp_tool: "click_widget",
            description: "click_widget{target:'settings.diagnostics.worksurface-inspector.dump'} writes the MT-102 JSON artifact.",
        },
        AgentToolRow {
            author_id: crate::visual_debugger::WORKSURFACE_INSPECTOR_STATUS_AUTHOR_ID,
            surface: ManualSurface::Diagnostics,
            action_label: "Read the visual-debugger dump status",
            mcp_tool: "list_widgets",
            description: "list_widgets reads settings.diagnostics.worksurface-inspector.status for the last dump filename/size.",
        },
    ];

    for &author_id in EDITOR_SETTINGS_CONTROL_AUTHOR_IDS {
        let (surface, action_label, mcp_tool, description) = match author_id {
            crate::settings_editor_section::EDITOR_FONT_SIZE_AUTHOR_ID => (
                ManualSurface::Code,
                "Set editor font size",
                "set_value",
                "set_value targets settings-editor-font-size; the value applies live and persists with workspace settings.",
            ),
            crate::settings_editor_section::EDITOR_TAB_SIZE_AUTHOR_ID => (
                ManualSurface::Code,
                "Set editor tab size",
                "set_value",
                "set_value targets settings-editor-tab-size; the value applies live and persists with workspace settings.",
            ),
            crate::settings_editor_section::EDITOR_WRAP_COLUMN_AUTHOR_ID => (
                ManualSurface::Code,
                "Set bounded wrap column",
                "set_value",
                "set_value targets settings-editor-wrap-column while bounded word wrap is selected.",
            ),
            crate::settings_editor_section::EDITOR_LINE_HEIGHT_AUTHOR_ID => (
                ManualSurface::Code,
                "Set editor line height",
                "set_value",
                "set_value targets settings-editor-line-height; the value applies to mounted editors.",
            ),
            crate::settings_editor_section::WIKI_PROJECTION_SETTINGS_POSTURE_AUTHOR_ID => (
                ManualSurface::Knowledge,
                "Read Wiki Projection settings posture",
                "list_widgets",
                "list_widgets reads settings-editor-wiki-projection-posture; Wiki Projection intentionally has no dedicated preference.",
            ),
            crate::settings_editor_section::FLIGHT_RECORDER_SETTINGS_POSTURE_AUTHOR_ID => (
                ManualSurface::Diagnostics,
                "Read Flight Recorder settings posture",
                "list_widgets",
                "list_widgets reads settings-editor-flight-recorder-posture; the audit surface cannot be disabled here.",
            ),
            crate::settings_editor_section::SYNTAX_PALETTE_MODE_AUTHOR_ID => (
                ManualSurface::Code,
                "Choose syntax palette mode",
                "set_value",
                "set_value targets settings-syntax-palette-mode with muted, standard, or custom; the selected palette repaints mounted editors.",
            ),
            crate::settings_editor_section::EDITOR_WORD_WRAP_AUTHOR_ID => (
                ManualSurface::Code,
                "Choose word-wrap mode",
                "set_value",
                "set_value targets settings-editor-word-wrap with off, on, or bounded; bounded mounts settings-editor-wrap-column.",
            ),
            crate::settings_editor_section::EDITOR_RENDER_WHITESPACE_AUTHOR_ID => (
                ManualSurface::Code,
                "Choose whitespace rendering",
                "set_value",
                "set_value targets settings-editor-render-whitespace with none, boundary, or all.",
            ),
            _ => (
                ManualSurface::Code,
                "Change an Editor setting",
                "click_widget",
                "click_widget toggles or opens this live Editor setting control; inspect the resulting state with list_widgets.",
            ),
        };
        rows.push(AgentToolRow {
            author_id,
            surface,
            action_label,
            mcp_tool,
            description,
        });
    }

    for &author_id in EDITOR_SETTINGS_OPTION_AUTHOR_IDS {
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::Code,
            action_label: "Choose a mounted Editor setting option",
            mcp_tool: "click_widget",
            description: "Open the owning Editor/Syntax selector, inspect the mounted option rows, then click_widget targets this exact option author_id.",
        });
    }

    for &author_id in SYNTAX_SWATCH_AUTHOR_IDS {
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::Code,
            action_label: "Edit a Custom syntax scope color",
            mcp_tool: "set_value",
            description: "set_value replaces this Custom palette swatch with #RRGGBB, #RRGGBBAA, or [r,g,b,a]; mounted editors and minimaps repaint in the same frame.",
        });
    }

    // The keybinding table is generated from `editor_action_catalog`, and each live action renders two
    // addressable controls: a TextEdit row and a Reset button. Generate the steering rows from that same
    // catalog and the same author-id constructor/prefix so a newly added action cannot become an
    // operator-visible control without an exact author_id -> MCP tool entry in the manual.
    rows.extend_from_slice(editor_keybinding_agent_tool_rows());

    for &author_id in crate::top_menu_bar::EDITOR_MENU_LEAF_AUTHOR_IDS {
        rows.push(AgentToolRow {
            author_id,
            surface: editor_menu_leaf_surface(author_id),
            action_label: "Use an operator editor menu leaf",
            mcp_tool: "click_widget",
            description: "Open the matching FILE, EDIT, or GO dropdown, then click_widget targets this real menu leaf by author_id.",
        });
    }

    for &author_id in crate::top_menu_bar::EDITORS_MENU_LEAF_AUTHOR_IDS {
        rows.push(AgentToolRow {
            author_id,
            surface: editors_menu_leaf_surface(author_id),
            action_label: "Use an EDITORS dropdown leaf",
            mcp_tool: "click_widget",
            description: "Open the EDITORS dropdown with menu-editors, then click_widget targets this real WP-012 menu leaf by author_id.",
        });
    }

    for &(author_id, surface, action_label, description) in &[
        (
            crate::settings_editor_section::EDITOR_PREFS_RESET_AUTHOR_ID,
            ManualSurface::Code,
            "Reset Editor preferences to defaults",
            "click_widget{target:'settings-editor-prefs-reset'} POSTs reset for the canonical scalar view-defaults.editor.* PreferenceRecords.",
        ),
        (
            crate::settings_editor_section::SYNTAX_PALETTE_RESET_AUTHOR_ID,
            ManualSurface::Code,
            "Reset Syntax palette to defaults",
            "click_widget{target:'settings-syntax-palette-reset'} POSTs reset for syntax-palette-mode and syntax-custom-colors PreferenceRecords.",
        ),
    ] {
        rows.push(AgentToolRow {
            author_id,
            surface,
            action_label,
            mcp_tool: "click_widget",
            description,
        });
    }

    // ── Code editor: every CODE_ACTION_CATALOG entry as editor.code.<action> ─────────────────────────
    // Both momentary Buttons and ToggleButtons are ACTIVATED by a click (a toggle carries its toggled
    // state separately), so every code action is driven by click_widget{target:<author_id>}.
    for entry in CODE_ACTION_CATALOG {
        let author_id: &'static str = code_author_id_static(entry.action_id);
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::Code,
            action_label: entry.label,
            mcp_tool: "click_widget",
            description: "click_widget{target:<author_id>} activates this code-editor action.",
        });
    }

    // ── Rich-text editor: every rich_action_catalog() entry as editor.rich.<action> ──────────────────
    for entry in rich_action_catalog() {
        let author_id: &'static str = rich_author_id_static(entry.action_id);
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::RichText,
            action_label: entry.label,
            mcp_tool: "click_widget",
            description: "click_widget{target:<author_id>} activates this rich-text editor action.",
        });
    }

    // ── Graph controls (mounted LoomGraphView toolbar + GRAPH_CONTROL_CATALOG) ──────────────────────
    for (author_id, action_label, description) in [
        (
            GRAPH_MODE_LOCAL_AUTHOR_ID,
            "Switch graph to Local mode",
            "click_widget{target:graph.mode.local} switches to the focused-block neighbourhood and re-fetches /loom/graph/local with start_block_id and max_depth.",
        ),
        (
            GRAPH_MODE_GLOBAL_AUTHOR_ID,
            "Switch graph to Global mode",
            "click_widget{target:graph.mode.global} switches to workspace-wide graph data and re-fetches /loom/graph/global.",
        ),
        (
            GRAPH_ZOOM_IN_AUTHOR_ID,
            "Zoom graph in",
            "click_widget{target:graph.zoom.in} increases the Loom graph zoom level.",
        ),
        (
            GRAPH_ZOOM_OUT_AUTHOR_ID,
            "Zoom graph out",
            "click_widget{target:graph.zoom.out} decreases the Loom graph zoom level.",
        ),
        (
            GRAPH_RELAYOUT_AUTHOR_ID,
            "Relayout graph",
            "click_widget{target:graph.relayout} restarts the graph layout after a data or layout change.",
        ),
        (
            GRAPH_RETRY_AUTHOR_ID,
            "Retry graph load",
            "click_widget{target:graph.retry} retries the current workspace, mode, focus, and max_depth after a visible graph error.",
        ),
    ] {
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::Graph,
            action_label,
            mcp_tool: "click_widget",
            description,
        });
    }

    for entry in GRAPH_CONTROL_CATALOG {
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Graph,
            action_label: entry.label,
            mcp_tool: "click_widget",
            description: "click_widget{target:<author_id>} drives this Loom graph control.",
        });
    }

    // ── Folder-tree controls (mounted LoomFolderTree static controls; dynamic rows are documented by pattern)
    rows.push(AgentToolRow {
        author_id: FOLDER_TREE_RETRY_AUTHOR_ID,
        surface: ManualSurface::Knowledge,
        action_label: "Retry folder-tree load",
        mcp_tool: "click_widget",
        description: "click_widget{target:'folder-tree.retry'} retries GET /workspaces/{id}/loom/folders after the mounted folder tree shows an error.",
    });
    rows.push(AgentToolRow {
        author_id: TAGS_SEARCH_AUTHOR_ID,
        surface: ManualSurface::Knowledge,
        action_label: "Filter tags",
        mcp_tool: "set_value",
        description: "set_value{target:'tags.search', value:'<prefix>'} filters the mounted tag-hub list by title prefix.",
    });

    // ── Canvas controls (CANVAS_CONTROL_CATALOG) ─────────────────────────────────────────────────────
    for entry in CANVAS_CONTROL_CATALOG {
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Canvas,
            action_label: entry.label,
            mcp_tool: "click_widget",
            description: "click_widget{target:<author_id>} drives this canvas-board control.",
        });
    }
    rows.push(AgentToolRow {
        author_id: crate::graph::canvas_board::RETRY_AUTHOR_ID,
        surface: ManualSurface::Canvas,
        action_label: "Retry Canvas load",
        mcp_tool: "click_widget",
        description: "click_widget{target:'canvas.retry'} retries getCanvasBoard after the mounted Canvas shows a typed backend error; loading is bounded and failure restores this control.",
    });

    // ── Collection controls (COLLECTION_CONTROL_CATALOG) ─────────────────────────────────────────────
    for entry in COLLECTION_CONTROL_CATALOG {
        let description = if entry.parameterized {
            "click_widget{target:<author_id>, payload:{...}} drives this parameterized block-collection control."
        } else {
            "click_widget{target:<author_id>} drives this block-collection control."
        };
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Knowledge,
            action_label: entry.label,
            mcp_tool: "click_widget",
            description,
        });
    }

    // ── FEMS interop (Pillar 12) ─────────────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::fems::RELEVANT_MEMORY_PANEL_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Read the FEMS retrieval capsule",
        mcp_tool: "list_widgets",
        description:
            "list_widgets surfaces the relevant-memory-panel + its items for the agent to read.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::RELEVANT_MEMORY_LIST_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Enumerate memory items",
        mcp_tool: "list_widgets",
        description:
            "list_widgets reveals the relevant-memory-list rows (provenance-first capsule items).",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::RELEVANT_MEMORY_REFRESH_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Refresh the live FEMS MemoryPack",
        mcp_tool: "click_widget",
        description: "click_widget{target:'editor.fems.memorypack-refresh'} retries or refreshes the mounted pane's live MemoryPack read for the current context.",
    });
    rows.push(AgentToolRow {
        author_id: "command-palette.option.hs-fems-palette-propose-to-memory",
        surface: ManualSurface::Fems,
        action_label: "Open a review-gated memory-write proposal",
        mcp_tool: "click_widget",
        description: "Open the command palette, then click_widget this option to dispatch fems.propose_to_memory through the shared command bus.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::RELEVANT_MEMORY_STATUS_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Read the structured MemoryPack outcome",
        mcp_tool: "list_widgets",
        description: "Read the status node value for state, context, generation, completed refresh count, and item count without label scraping.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Inspect the review-gated proposal dialog",
        mcp_tool: "list_widgets",
        description: "list_widgets reads fems-propose-dialog and its current captured source context before confirmation.",
    });
    for (author_id, class) in [
        ("fems-class-episodic", "episodic"),
        ("fems-class-semantic", "semantic"),
        ("fems-class-procedural", "procedural"),
    ] {
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::Fems,
            action_label: "Choose the FEMS proposal class",
            mcp_tool: "click_widget",
            description: match class {
                "episodic" => "click_widget{target:'fems-class-episodic'} selects episodic memory.",
                "semantic" => "click_widget{target:'fems-class-semantic'} selects semantic memory.",
                _ => "click_widget{target:'fems-class-procedural'} selects procedural memory.",
            },
        });
    }
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_PROPOSE_CANCEL_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Cancel the memory-write proposal",
        mcp_tool: "click_widget",
        description: "click_widget{target:'fems-propose-cancel'} closes the review dialog without submitting; fems-propose-status reports the stable cancelled operation.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_PROPOSE_CONFIRM_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Confirm the memory-write proposal",
        mcp_tool: "click_widget",
        description:
            "click_widget{target:'fems-propose-confirm'} submits the review-gated proposal and waits for the bounded correlated EventLedger persistence receipt.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_PROPOSE_STATUS_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Read the structured proposal outcome",
        mcp_tool: "list_widgets",
        description: "Read state, outcome, operation_id, proposal_id, and proposal event_id. After approval, state=committed;outcome=approved also carries memory_id, commit_id, memory_pack_id/hashes, commit EventLedger/Flight Recorder identities, and committed_at.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_REVIEW_APPROVE_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Approve a pending FEMS proposal",
        mcp_tool: "click_widget",
        description: "click_widget{target:'fems-review-approve'} records the live review decision and then invokes the separate explicit approved-proposal commit route; exact retry returns the original review and commit receipts.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_REVIEW_REJECT_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Reject a pending FEMS proposal",
        mcp_tool: "click_widget",
        description: "click_widget{target:'fems-review-reject'} records an operator rejection through the live FEMS review route.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_REVIEW_STATUS_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Read the structured FEMS review outcome",
        mcp_tool: "list_widgets",
        description: "Read the pending/terminal review state and exact proposal, actor, correlation, EventLedger, Flight Recorder, and reviewed-at identities. Approved completion is reported by fems-propose-status as state=committed;outcome=approved with the explicit commit receipt.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_REVIEW_REFRESH_RETRY_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Retry canonical pending-review discovery",
        mcp_tool: "click_widget",
        description: "click_widget{target:'fems-review-refresh-retry'} retries the exact failed workspace queue refresh; new proposal creation stays blocked until recovery succeeds.",
    });

    // ── Stage interop edge (Pillar 17) ───────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: "menu.editors.stage",
        surface: ManualSurface::Interop,
        action_label: "Stage edge: open the Stage pane",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'menu.editors.stage'} opens or focuses the one docked Stage pane.",
    });
    rows.push(AgentToolRow {
        author_id: "menu.editors.route-to-stage",
        surface: ManualSurface::Interop,
        action_label: "Stage edge: route the active editor content",
        mcp_tool: "argus.click",
        description: "argus.click{target:'menu.editors.route-to-stage'} submits the active selection/document through the shared InteractionBus.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_PANE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: the Stage pane container",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect surfaces the stage-pane; an agent reads what was routed to Stage.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_ROUTE_STATUS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: read route state",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads stage-route-status for busy, unavailable, retained-retry, and terminal route state.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_ROUTE_RETRY_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: retry the retained route",
        mcp_tool: "argus.click",
        description: "argus.click{target:'stage-route-retry'} retries the exact retained request without changing its causal action id.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_ROUTED_CONTENT_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: routed content region",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads the exact routed-content summary at stage-routed-content.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: embed a capture back into notes",
        mcp_tool: "argus.click",
        description: "argus.click{target:'stage-capture-embed-back'} runs privileged capture, exact-byte retrieval and SHA-256 verification, then embeds into the live note target; perform a fresh argus.inspect of stage-embed-back-status for the exact artifact/provenance success or typed failure.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_EMBED_BACK_STATUS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: read capture/embed result",
        mcp_tool: "argus.inspect",
        description: "Use a fresh argus.inspect to read stage-embed-back-status for the stable artifact id, verified SHA-256, target pane, endpoint blocker, provenance refusal, stale target, or insertion failure.",
    });

    // ── Calendar interop edge (Pillar 2) ─────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_PANEL_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: the daily-journal panel",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect surfaces the daily-journal-panel (daily-note <-> CalendarEvent binding).",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::daily_notes::journal_panel::JOURNAL_ROOT_ID,
        surface: ManualSurface::Knowledge,
        action_label: "Daily Journal editor root",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces journal-panel-root after view.journal mounts the bound journal editor.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::daily_notes::journal_panel::START_WRITING_ID,
        surface: ManualSurface::Knowledge,
        action_label: "Daily Journal start writing",
        mcp_tool: "argus.click",
        description: "argus.click{target:'journal-start-writing'} creates the session RichDocument for a blank daily journal block.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::daily_notes::journal_panel::LINK_GAP_ID,
        surface: ManualSurface::Knowledge,
        action_label: "Daily Journal missing durable link banner",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces journal-document-link-gap when a created session document cannot be durably linked to the Loom block.",
    });
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: the daily-note date header",
        mcp_tool: "argus.click",
        description: "argus.click{target:'daily-journal-date-header'} opens the bound date.",
    });
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: a bound CalendarEvent chip",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'daily-journal-calendar-event-chip'} opens the bound event; use a fresh argus.inspect for the attributed receipt and exact CalendarEvent destination.",
    });
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: read-only ActivitySpan strip",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces the live daily-journal-activity-strip read-only correlation; fetch failures expose the typed unavailable state instead of an empty success.",
    });

    // ── Code<->note interop edge (MT-034) ────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: "editor.rich.insert-slash-command",
        surface: ManualSurface::RichText,
        action_label: "Code refs: create an exact code hsLink",
        mcp_tool: "argus.click",
        description: "After argus.inspect, use argus.click with target editor.rich.insert-slash-command and payload {\"kind\":\"wikilink\",\"ref_kind\":\"code\",\"ref_value\":\"<symbol_entity_id>\",\"label\":\"<display_name>\"}; require the attributed receipt and a fresh argus.inspect containing code-ref-chip-{symbol_entity_id}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::slash_commands::CODE_SYMBOL_SEARCH_AUTHOR_ID,
        surface: ManualSurface::RichText,
        action_label: "Code refs: read the code-symbol search dialog",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces code-symbol-search; after a safe result action, require an attributed receipt and fresh inspection of code-ref-chip-{symbol_entity_id}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::slash_commands::CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID,
        surface: ManualSurface::RichText,
        action_label: "Code refs: filter code symbols",
        mcp_tool: "argus.set_value",
        description: "argus.set_value{target:'code-symbol-search-input', value:'<symbol>'} filters backend code-symbol lookup results; require the attributed receipt and fresh result-row inspection.",
    });
    rows.push(AgentToolRow {
        author_id: "code-symbol-result-{symbol_entity_id}",
        surface: ManualSurface::RichText,
        action_label: "Code refs: select an exact symbol result",
        mcp_tool: "argus.click",
        description: "After argus.inspect identifies the exact dynamic result id, argus.click selects it; require the attributed receipt and fresh code-ref-chip-{symbol_entity_id} observation.",
    });
    rows.push(AgentToolRow {
        author_id: "code-ref-chip-{symbol_entity_id}",
        surface: ManualSurface::Interop,
        action_label: "Code refs: open and reveal the exact source",
        mcp_tool: "argus.click",
        description: "After argus.inspect confirms the exact chip, argus.click resolves the backend symbol, opens the canonical file-backed Code tab, and reveals line_start; require the attributed receipt and a fresh editor.code.text observation.",
    });
    rows.push(AgentToolRow {
        author_id: crate::code_editor::note_refs_panel::PANEL_AUTHOR_ID,
        surface: ManualSurface::Code,
        action_label: "Code refs: inspect notes mentioning the current symbol",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces note-refs-panel and its exact dynamic note-ref-{document_id} rows after symbol dwell.",
    });
    rows.push(AgentToolRow {
        author_id: "note-ref-{document_id}",
        surface: ManualSurface::Interop,
        action_label: "Code refs: reveal a referencing note",
        mcp_tool: "argus.click",
        description: "After argus.inspect selects the exact dynamic row, argus.click routes document_id through the shared open-document command; require the attributed receipt and a fresh editor.rich.text observation for that document.",
    });

    // ── Locus interop edge (Pillar 6) ────────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Locus edge: the outgoing-links pane",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect surfaces the outgoing.panel listing locus:// and wikilink references.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::wikilinks::outgoing_links_panel::RESOLVED_SECTION_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Locus edge: resolved references section",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reveals outgoing.section.resolved rows (each navigable by argus.click).",
    });
    rows.push(AgentToolRow {
        author_id:
            crate::rich_editor::wikilinks::outgoing_links_panel::UNRESOLVED_SECTION_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Locus edge: unresolved (dangling) references section",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reveals outgoing.section.unresolved rows; record-not-found remains unresolved while route-unavailable is a distinct typed failure.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::wikilinks::backlinks_panel::PANEL_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Backlinks edge: the backlinks panel",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces the Role::List backlinks-panel; rows are clickable \
                      Role::ListItem nodes named backlink-{source_document_id}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::wikilinks::backlinks_panel::REFRESH_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Backlinks edge: refresh backlinks",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'backlinks-refresh'} refreshes the current document backlinks list.",
    });

    // Conditionally rendered code-editor controls are sourced from the live owning constants. The
    // default editor instance exposes these exact ids; secondary editor instances append `#<instance>`.
    rows.extend_from_slice(&[
        AgentToolRow {
            author_id: crate::code_editor::rename::CODE_EDITOR_RENAME_INPUT_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Replace the inline rename value",
            mcp_tool: "set_value",
            description: "set_value replaces code_editor_rename_input while the rename overlay is open; inspect the preview before applying.",
        },
        AgentToolRow {
            author_id: crate::code_editor::rename::CODE_EDITOR_RENAME_APPLY_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Apply the inspected rename preview",
            mcp_tool: "click_widget",
            description: "click_widget targets code_editor_rename_apply after the multi-file preview has been inspected.",
        },
        AgentToolRow {
            author_id: crate::code_editor::rename::CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Cancel the rename preview",
            mcp_tool: "click_widget",
            description: "click_widget targets code_editor_rename_cancel and applies no previewed edits.",
        },
        AgentToolRow {
            author_id: crate::code_editor::rename::CODE_EDITOR_CTX_RENAME_SYMBOL_MENU_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Open Rename Symbol from the code context menu",
            mcp_tool: "click_widget",
            description: "click_widget targets ctx-menu.code_editor_ctx_rename_symbol, then set_value replaces the live rename input.",
        },
        AgentToolRow {
            author_id: crate::code_editor::code_actions::CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Open Quick Fix from the code context menu",
            mcp_tool: "click_widget",
            description: "click_widget targets code_editor_ctx_quick_fix; list_widgets then reveals the live quick-fix menu and its action rows.",
        },
        AgentToolRow {
            author_id: crate::code_editor::code_actions::CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Inspect available Quick Fix actions",
            mcp_tool: "list_widgets",
            description: "list_widgets reads code_editor_quickfix_menu while it is open; click the discovered action row by its live author_id.",
        },
        AgentToolRow {
            author_id: default_quickfix_first_item_author_id(),
            surface: ManualSurface::Code,
            action_label: "Apply the first available Quick Fix",
            mcp_tool: "click_widget",
            description: "click_widget targets code_editor_quickfix_item_0 when at least one action is listed; inspect the menu first because item ids are generated by current result index.",
        },
        AgentToolRow {
            author_id: crate::code_editor::formatting::FORMAT_SELECTION_CTX_AUTHOR_ID,
            surface: ManualSurface::Code,
            action_label: "Format the current code selection",
            mcp_tool: "click_widget",
            description: "click_widget targets code_editor_ctx_format_selection and applies the returned edits as one undo group.",
        },
    ]);

    for row in &mut rows {
        row.mcp_tool = canonical_argus_method(row.mcp_tool);
        row.description = canonical_argus_description(row.description);
    }
    rows
}

fn default_quickfix_first_item_author_id() -> &'static str {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| crate::code_editor::code_actions::quickfix_item_author_id(0, ""))
        .as_str()
}

fn canonical_argus_method(method: &'static str) -> &'static str {
    match method {
        "list_widgets" => crate::mcp::argus::ARGUS_INSPECT_METHOD,
        "click_widget" => crate::mcp::argus::ARGUS_CLICK_METHOD,
        "set_value" => crate::mcp::argus::ARGUS_SET_VALUE_METHOD,
        "screenshot" => crate::mcp::argus::ARGUS_SCREENSHOT_METHOD,
        canonical => canonical,
    }
}

/// Canonicalize historical row prose once per distinct static description. This keeps compatibility
/// spellings out of the rendered examples without leaking a new allocation each time the manual opens.
fn canonical_argus_description(description: &'static str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    if !["list_widgets", "click_widget", "set_value", "screenshot"]
        .iter()
        .any(|legacy| description.contains(legacy))
    {
        return description;
    }

    static CACHE: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(canonical) = cache.get(description) {
        return canonical;
    }
    let canonical: &'static str = Box::leak(canonical_argus_prose(description).into_boxed_str());
    cache.insert(description, canonical);
    canonical
}

fn canonical_argus_prose(prose: &str) -> String {
    let prose = replace_method_token(prose, "list_widgets", "argus.inspect");
    let prose = replace_method_token(&prose, "click_widget", "argus.click");
    let prose = replace_method_token(&prose, "set_value", "argus.set_value");
    replace_method_token(&prose, "screenshot", "argus.screenshot")
}

fn replace_method_token(prose: &str, legacy: &str, canonical: &str) -> String {
    fn identifier_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || ch == '_'
    }

    let mut output = String::with_capacity(prose.len());
    let mut copied_through = 0;
    for (start, _) in prose.match_indices(legacy) {
        let end = start + legacy.len();
        // Already-canonical product names are idempotent. Without this guard, `argus.screenshot`
        // became `argus.argus.screenshot` because `.` is intentionally not an identifier character.
        if prose[..start].ends_with("argus.") {
            continue;
        }
        let before_is_identifier = prose[..start]
            .chars()
            .next_back()
            .is_some_and(identifier_char);
        let after_is_identifier = prose[end..].chars().next().is_some_and(identifier_char);
        if before_is_identifier || after_is_identifier {
            continue;
        }
        output.push_str(&prose[copied_through..start]);
        output.push_str(canonical);
        copied_through = end;
    }
    output.push_str(&prose[copied_through..]);
    output
}

/// Structured steering rows for every live per-action keybinding TextEdit and Reset button.
///
/// [`AgentToolRow`] intentionally carries static strings. The live action catalog owns dynamic `String`
/// ids, so this cache interns each exact runtime-generated id once for the process lifetime instead of
/// leaking another copy every time the manual pane rebuilds its section.
fn editor_keybinding_agent_tool_rows() -> &'static [AgentToolRow] {
    static ROWS: std::sync::OnceLock<Vec<AgentToolRow>> = std::sync::OnceLock::new();

    ROWS.get_or_init(|| {
        let actions = editor_action_catalog();
        let mut rows = Vec::with_capacity(actions.len() * 2);
        for action in actions {
            let surface = match action.surface {
                EditorActionSurface::Code => ManualSurface::Code,
                EditorActionSurface::Rich => ManualSurface::RichText,
            };
            let row_author_id: &'static str =
                Box::leak(editor_keybind_row_author_id(&action.id).into_boxed_str());
            let reset_author_id: &'static str = Box::leak(
                format!("{EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX}{}", action.id).into_boxed_str(),
            );

            rows.push(AgentToolRow {
                author_id: row_author_id,
                surface,
                action_label: "Set an editor keybinding chord",
                mcp_tool: "set_value",
                description: "set_value replaces this live keybinding draft; a valid non-empty chord applies to mounted editors and persists with workspace settings.",
            });
            rows.push(AgentToolRow {
                author_id: reset_author_id,
                surface,
                action_label: "Reset an editor keybinding",
                mcp_tool: "click_widget",
                description: "click_widget clears this live keybinding override, restores the built-in default, and persists the reset.",
            });
        }
        rows
    })
}

fn editor_menu_leaf_surface(author_id: &str) -> ManualSurface {
    match author_id {
        "menu.file.save-as"
        | "menu.file.export-html"
        | "menu.file.export-md"
        | "menu.file.export-txt"
        | "menu.file.export-json" => ManualSurface::RichText,
        "menu.edit.command-palette"
        | "menu.edit.quick-switcher"
        | crate::command_registry::CMD_EDITOR_GO_TO_SYMBOL => ManualSurface::Knowledge,
        crate::command_registry::CMD_EDITOR_GO_TO_DEFINITION
        | crate::command_registry::CMD_EDITOR_GO_TO_REFERENCES
        | crate::command_registry::CMD_EDITOR_GO_TO_LINE => ManualSurface::Code,
        id if id.starts_with("menu.edit.") || id.starts_with("menu-go-") => ManualSurface::Code,
        id if id.starts_with("menu.file.") => ManualSurface::Knowledge,
        _ => ManualSurface::Knowledge,
    }
}

fn editors_menu_leaf_surface(author_id: &str) -> ManualSurface {
    match author_id {
        "menu.editors.stage"
        | "menu.editors.route-to-stage"
        | "menu.editors.embed-stage-capture" => ManualSurface::Canvas,
        "menu.editors.outline"
        | "menu.editors.relevant-memory"
        | "menu.editors.outgoing-links"
        | "menu.editors.sidebar"
        | "menu.editors.journal" => ManualSurface::Knowledge,
        "menu.editors.format-document"
        | "menu.editors.next-diagnostic"
        | "menu.editors.prev-diagnostic"
        | "menu.editors.rename-symbol"
        | "menu.editors.quick-fix" => ManualSurface::Code,
        _ => ManualSurface::Knowledge,
    }
}

/// Map a code action_id to the corresponding `editor.code.<action>` `&'static str` literal. The literals
/// are spelled out (not `format!`) so they are `&'static str` for the id-audit's static cross-check. Every
/// arm corresponds 1:1 to a `CODE_ACTION_CATALOG` entry; a missing arm is a compile-time-visible panic in
/// debug (the catalog and this map are kept in lockstep by `code_author_ids_cover_catalog`).
fn code_author_id_static(action_id: &str) -> &'static str {
    match action_id {
        "save" => "editor.code.save",
        "find-open" => "editor.code.find-open",
        "find-next" => "editor.code.find-next",
        "find-prev" => "editor.code.find-prev",
        "find-toggle-case" => "editor.code.find-toggle-case",
        "find-toggle-word" => "editor.code.find-toggle-word",
        "find-toggle-regex" => "editor.code.find-toggle-regex",
        "replace-open" => "editor.code.replace-open",
        "replace-one" => "editor.code.replace-one",
        "replace-all" => "editor.code.replace-all",
        "format" => "editor.code.format",
        "go-to-line" => "editor.code.go-to-line",
        "multi-cursor-add" => "editor.code.multi-cursor-add",
        "multi-cursor-clear" => "editor.code.multi-cursor-clear",
        "command-palette-open" => "editor.code.command-palette-open",
        "language-picker-open" => "editor.code.language-picker-open",
        other => {
            panic!("code action_id '{other}' has no static editor.code.* literal — add it here")
        }
    }
}

/// Map a rich action_id to the corresponding `editor.rich.<action>` `&'static str` literal (see
/// [`code_author_id_static`] for why static).
fn rich_author_id_static(action_id: &str) -> &'static str {
    match action_id {
        "save" => "editor.rich.save",
        "find-open" => "editor.rich.find-open",
        "find-next" => "editor.rich.find-next",
        "find-prev" => "editor.rich.find-prev",
        "find-toggle-case" => "editor.rich.find-toggle-case",
        "find-toggle-word" => "editor.rich.find-toggle-word",
        "find-toggle-regex" => "editor.rich.find-toggle-regex",
        "replace-one" => "editor.rich.replace-one",
        "replace-all" => "editor.rich.replace-all",
        "format-bold" => "editor.rich.format-bold",
        "format-italic" => "editor.rich.format-italic",
        "format-code" => "editor.rich.format-code",
        "format-heading-1" => "editor.rich.format-heading-1",
        "format-heading-2" => "editor.rich.format-heading-2",
        "format-heading-3" => "editor.rich.format-heading-3",
        "format-heading-4" => "editor.rich.format-heading-4",
        "format-heading-5" => "editor.rich.format-heading-5",
        "format-heading-6" => "editor.rich.format-heading-6",
        "insert-slash-command" => "editor.rich.insert-slash-command",
        "command-palette-open" => "editor.rich.command-palette-open",
        other => {
            panic!("rich action_id '{other}' has no static editor.rich.* literal — add it here")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_has_all_eight_required_headings() {
        let section = editors_manual_section();
        for h in REQUIRED_HEADINGS {
            assert!(
                section.topic(h).is_some(),
                "GLOBAL-BUILD-MANUAL heading '{h}' must be an individual topic"
            );
        }
        assert!(section.has_all_headings(REQUIRED_HEADINGS));
    }

    #[test]
    fn code_author_ids_cover_catalog() {
        // Every catalog entry must have a static literal (the map panics otherwise).
        for entry in CODE_ACTION_CATALOG {
            let id = code_author_id_static(entry.action_id);
            assert!(id.starts_with("editor.code."), "{id}");
        }
    }

    #[test]
    fn rich_author_ids_cover_catalog() {
        for entry in rich_action_catalog() {
            let id = rich_author_id_static(entry.action_id);
            assert!(id.starts_with("editor.rich."), "{id}");
        }
    }
}
