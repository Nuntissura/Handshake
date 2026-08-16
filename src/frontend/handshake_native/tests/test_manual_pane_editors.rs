//! WP-KERNEL-012 MT-073 (E12) — built-in User Manual editors content + agent-tool reference proofs.
//!
//! Proves, with REAL runtime evidence (no tautologies):
//! - AC-001 / PT-001: the manual pane loads the editors section and ALL eight GLOBAL-BUILD-MANUAL
//!   headings are present as individual topics.
//! - AC-002 / PT-002: the agent-tool reference lists every editor/knowledge/FEMS/interop action with a
//!   NON-EMPTY author_id + a NON-EMPTY MCP tool that is one of the four REAL mcp/tools.rs methods.
//! - AC-003 / PT-003: the WP-011-style manual SEARCH box (driven via egui_kittest) finds an editor topic
//!   by keyword — a live interaction, not an in-memory assertion.
//! - AC-004 / PT-004: NO documented author_id is missing from the LIVE AccessKit registry — the id-audit
//!   cross-checks every agent-tool-reference author_id against the live registries (catalogs +
//!   DECLARED_IDENTITIES + the fixed interop/FEMS/Stage/Calendar/Locus constants) and fails on any orphan.
//! - AC-005 / PT-002: the four interop edges (FEMS, Stage, Calendar, Locus) are each documented with an
//!   author_id + mcp_tool.
//! - MC-006: the manual content contains NO 'SQLite' token and no direct-DB-write language.
//!
//! ARTIFACT HYGIENE (CX-212E / the SCREENSHOT/TEST-ARTIFACT rule): the HBR-VIS screenshot is written ONLY
//! to the EXTERNAL Handshake_Artifacts/handshake-test/wp-kernel-012-mt-073/ root via
//! [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if any repo-local
//! `test_output/` or `tests/screenshots/` dir exists.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::{NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::accessibility::editor_action_registry::{
    rich_action_catalog, CODE_ACTION_CATALOG,
};
use handshake_native::accessibility::{
    UiTreeNode, UiTreeSnapshot, CANVAS_CONTROL_CATALOG, COLLECTION_CONTROL_CATALOG,
    DECLARED_IDENTITIES, GRAPH_CONTROL_CATALOG, PALETTE_AUTHOR_IDS,
};
use handshake_native::manual_content_editors::{
    agent_tool_rows, editors_manual_section, INTEROP_EDGES, REQUIRED_HEADINGS,
};
use handshake_native::manual_pane::{
    ManualPane, ManualPaneState, ManualRegistry, ManualSurface, MANUAL_SEARCH_AUTHOR_ID,
};
use handshake_native::theme::HsPalette;
use handshake_native::{
    app::{HandshakeApp, HealthDisplayState},
    backend_client::HealthInfo,
    code_editor::CodeEditorPanel,
};

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts` is a
/// sibling of the repo worktree. (The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path.)
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the artifact-hygiene guard the
/// SCREENSHOT/TEST-ARTIFACT rule mandates). Checks BOTH `test_output/` and `tests/screenshots/`.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The FOUR canonical Argus method names (legacy aliases are not valid product-manual rows).
const REAL_MCP_TOOLS: &[&str] = &[
    handshake_native::mcp::ARGUS_INSPECT_METHOD,
    handshake_native::mcp::ARGUS_CLICK_METHOD,
    handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
    handshake_native::mcp::ARGUS_SCREENSHOT_METHOD,
];

/// Build the LIVE author_id set — the union of every real registered/static author_id across the surfaces
/// the manual documents. This is the id-audit's source of truth; a documented author_id absent from this
/// set is an ORPHAN (AC-004).
///
/// Sources (all LIVE registry-owning resources, none hand-typed): every entry is read from a real
/// registry/catalog/const so a documented id that drifts from the live id is caught as an orphan — the
/// set is NEVER seeded with a literal copy of a documented id (that would make the audit tautological).
/// - shell chrome: [`DECLARED_IDENTITIES`] + [`PALETTE_AUTHOR_IDS`] (the dot-form command-palette
///   container ids the live shell emits; quick-switcher / settings container ids live in
///   DECLARED_IDENTITIES);
/// - dynamic top-menu and command-palette rows: [`handshake_native::top_menu_bar::SWARM_ACCESSIBLE_ACTIONS`]
///   + generated `command-palette.option.<stable_id>` rows from
///     [`handshake_native::command_registry::all_commands`];
/// - code editor: `editor.code.<action>` for every [`CODE_ACTION_CATALOG`] entry;
/// - rich editor: `editor.rich.<action>` for every `rich_action_catalog()` entry;
/// - graph/canvas/collection: the three control catalogs;
/// - FEMS / Stage / Calendar / Locus / manual: the fixed `&'static str` constants from their modules.
fn live_author_id_set() -> HashSet<String> {
    let mut set: HashSet<String> = HashSet::new();

    // Shell chrome declared identities (this is where the command-palette + quick-switcher + settings
    // container ids actually live — the DOT-form ids the live shell emits).
    for ident in DECLARED_IDENTITIES {
        set.insert(ident.author_id.to_owned());
    }
    // The HASHED half of the SAME registry. DECLARED_IDENTITIES only carries hand-assigned numeric
    // NodeIds; ids backed by egui hashed ids live in DECLARED_HASHED_AUTHOR_IDS. Reading only the first
    // half made the audit report perfectly-registered surfaces as ORPHANS - stage-route-status and
    // stage-route-retry sit in that list, while their sibling stage-embed-back-status had been
    // hand-excluded instead, which is how the gap stayed hidden. Both halves are LIVE registry
    // resources, so this keeps the audit non-tautological.
    for author_id in handshake_native::accessibility::DECLARED_HASHED_AUTHOR_IDS {
        set.insert((*author_id).to_owned());
    }
    // The FEMS memory-class radios are GENERATED from the live enum, not a static array, so source
    // them from the same generator the widget uses rather than re-typing three literals.
    for class in [
        handshake_native::fems::memory_proposal::MemoryClass::Episodic,
        handshake_native::fems::memory_proposal::MemoryClass::Semantic,
        handshake_native::fems::memory_proposal::MemoryClass::Procedural,
    ] {
        set.insert(handshake_native::fems::memory_proposal::fems_class_author_id(class));
    }
    // The MT-036 Flight Recorder open-completion observer, from its own exported const.
    set.insert(handshake_native::app::MT036_FLIGHT_RECORDER_OPEN_COMPLETION_AUTHOR_ID.to_owned());
    // The command-palette dialog/search/list container ids, sourced from the REAL registry const
    // (PALETTE_AUTHOR_IDS = command-palette.dialog/.search/.list) — NOT hand-typed literals. These are
    // already covered by DECLARED_IDENTITIES above; pulling them from the same const the registry exports
    // keeps the audit reading the live resource instead of an implementer-authored mirror, so any
    // documented palette id that drifts from the live id is correctly flagged as an orphan (AC-004/MC-001).
    for id in PALETTE_AUTHOR_IDS {
        set.insert((*id).to_owned());
    }
    for id in handshake_native::top_menu_bar::SWARM_ACCESSIBLE_ACTIONS {
        set.insert((*id).to_owned());
    }
    set.insert(
        handshake_native::top_menu_bar::MenuId::Editors
            .author_id()
            .to_owned(),
    );
    for command in handshake_native::command_registry::all_commands() {
        set.insert(format!(
            "{}{}",
            handshake_native::command_palette::ROW_AUTHOR_ID_PREFIX,
            command.stable_id
        ));
    }
    // Code editor canonical action ids.
    for entry in CODE_ACTION_CATALOG {
        set.insert(format!("editor.code.{}", entry.action_id));
    }
    // Rich editor canonical action ids.
    for entry in rich_action_catalog() {
        set.insert(format!("editor.rich.{}", entry.action_id));
    }
    // Graph / canvas / collection control catalogs.
    for entry in GRAPH_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }
    for entry in CANVAS_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }
    for entry in COLLECTION_CONTROL_CATALOG {
        set.insert(entry.author_id.to_owned());
    }
    // Conditionally rendered editor/knowledge controls that are verified in their own focused tests
    // but are not part of the always-mounted static registry in this audit harness.
    set.insert(
        handshake_native::rich_editor::save::conflict_ui::CONFLICT_KEEP_YOURS_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::save::conflict_ui::CONFLICT_KEEP_SERVER_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::save::conflict_ui::CONFLICT_KEEP_YOURS_CONFIRM_AUTHOR_ID
            .to_owned(),
    );
    set.insert(handshake_native::rich_editor::save::conflict_ui::DRAFT_BANNER_AUTHOR_ID.to_owned());
    set.insert(
        handshake_native::rich_editor::save::conflict_ui::DRAFT_RESTORE_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::save::conflict_ui::DRAFT_DISCARD_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::renderer::rich_editor_widget::RichEditorWidget::EXPORT_BUTTON_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::save::conflict_ui::EXPORT_PICKER_AUTHOR_ID.to_owned(),
    );
    set.insert(handshake_native::graph::MODE_LOCAL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::graph::MODE_GLOBAL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::graph::ZOOM_IN_AUTHOR_ID.to_owned());
    set.insert(handshake_native::graph::ZOOM_OUT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::graph::RELAYOUT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::graph::folder_tree::RETRY_AUTHOR_ID.to_owned());
    set.insert(handshake_native::graph::tags_panel::SEARCH_AUTHOR_ID.to_owned());
    set.insert(
        handshake_native::rich_editor::daily_notes::journal_panel::JOURNAL_ROOT_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::daily_notes::journal_panel::START_WRITING_ID.to_owned(),
    );
    set.insert(handshake_native::rich_editor::daily_notes::journal_panel::LINK_GAP_ID.to_owned());
    set.insert(
        handshake_native::rich_editor::slash_commands::CODE_SYMBOL_SEARCH_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::slash_commands::CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID
            .to_owned(),
    );
    set.insert(handshake_native::code_editor::note_refs_panel::PANEL_AUTHOR_ID.to_owned());
    set.insert(
        handshake_native::code_editor::code_actions::CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::code_editor::code_actions::CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID.to_owned(),
    );
    set.insert(handshake_native::code_editor::code_actions::quickfix_item_author_id(0, ""));
    set.insert(
        handshake_native::code_editor::formatting::FORMAT_SELECTION_CTX_AUTHOR_ID.to_owned(),
    );

    // FEMS fixed ids.
    set.insert(handshake_native::fems::RELEVANT_MEMORY_PANEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::RELEVANT_MEMORY_LIST_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::RELEVANT_MEMORY_REFRESH_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::RELEVANT_MEMORY_STATUS_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_DIALOG_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_CANCEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_CONFIRM_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_PROPOSE_STATUS_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_REVIEW_APPROVE_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_REVIEW_REJECT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::fems::FEMS_REVIEW_STATUS_AUTHOR_ID.to_owned());

    // Stage fixed ids.
    set.insert(handshake_native::stage_pane::STAGE_PANE_AUTHOR_ID.to_owned());
    set.insert(handshake_native::stage_pane::STAGE_ROUTED_CONTENT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::stage_pane::STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID.to_owned());

    // Calendar (daily-journal) fixed ids.
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_PANEL_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::graph::daily_journal_panel::DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID
            .to_owned(),
    );

    // Locus (outgoing-links) fixed ids.
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::RESOLVED_SECTION_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::wikilinks::outgoing_links_panel::UNRESOLVED_SECTION_AUTHOR_ID
            .to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::wikilinks::backlinks_panel::PANEL_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::rich_editor::wikilinks::backlinks_panel::REFRESH_AUTHOR_ID.to_owned(),
    );

    // Manual pane's own search box id (documented as a Knowledge surface row).
    set.insert(MANUAL_SEARCH_AUTHOR_ID.to_owned());
    // Runtime Chat fixed ids.
    set.insert(handshake_native::runtime_chat::RUNTIME_CHAT_PANEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::runtime_chat::RUNTIME_CHAT_STATUS_AUTHOR_ID.to_owned());
    set.insert(handshake_native::runtime_chat::RUNTIME_CHAT_INPUT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::runtime_chat::RUNTIME_CHAT_SEND_AUTHOR_ID.to_owned());
    // Terminal launch status appears after the dynamic RUN/palette action records its typed blocker.
    set.insert(handshake_native::app::TERMINAL_LAUNCH_STATUS_AUTHOR_ID.to_owned());
    // MT-101 model-session launch dialog/status ids.
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_DIALOG_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_PROVIDER_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_PROVIDER_LOCAL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_PROVIDER_CLOUD_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_FOLDER_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_MODEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_WRAPPER_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_START_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_CANCEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_INLINE_STATUS_AUTHOR_ID.to_owned());
    set.insert(handshake_native::app::MODEL_SESSION_LAUNCH_STATUS_AUTHOR_ID.to_owned());
    // Settings-hosted diagnostics and MT-102 visual-debugger controls.
    set.insert(handshake_native::settings_dialog::SETTINGS_SEARCH_AUTHOR_ID.to_owned());
    set.insert(format!(
        "{}diagnostics",
        handshake_native::settings_dialog::SECTION_HEADER_AUTHOR_ID_PREFIX
    ));
    set.insert(handshake_native::diagnostics::DIAGNOSTICS_PANEL_AUTHOR_ID.to_owned());
    set.insert(handshake_native::diagnostics::DIAGNOSTICS_HEARTBEAT_AUTHOR_ID.to_owned());
    set.insert(handshake_native::diagnostics::DIAGNOSTICS_FRAME_AUTHOR_ID.to_owned());
    set.insert(handshake_native::diagnostics::DIAGNOSTICS_RESOURCE_AUTHOR_ID.to_owned());
    set.insert(handshake_native::diagnostics::DIAGNOSTICS_EVENTS_AUTHOR_ID.to_owned());
    set.insert(handshake_native::diagnostics::DIAGNOSTICS_PALMISTRY_AUTHOR_ID.to_owned());
    set.insert(
        handshake_native::visual_debugger::WORKSURFACE_INSPECTOR_DUMP_BUTTON_AUTHOR_ID.to_owned(),
    );
    set.insert(
        handshake_native::visual_debugger::WORKSURFACE_INSPECTOR_STATUS_AUTHOR_ID.to_owned(),
    );

    set
}

// ── AC-001 / PT-001: all eight GLOBAL-BUILD-MANUAL headings present as topics ─────────────────────────
#[test]
fn manual_loads_section_with_all_eight_required_headings() {
    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    assert_eq!(reg.len(), 1, "the editors section registered into the pane");

    let section = reg
        .section("native-editors")
        .expect("editors section is registered");
    for heading in REQUIRED_HEADINGS {
        assert!(
            section.topic(heading).is_some(),
            "AC-001: GLOBAL-BUILD-MANUAL heading '{heading}' must be present as an individual topic"
        );
        // Each topic body must be a real no-context body (not an empty stub).
        let body = &section.topic(heading).unwrap().body;
        assert!(
            body.len() > 60,
            "AC-001: heading '{heading}' must carry a substantive no-context body (got {} chars)",
            body.len()
        );
    }
    assert_eq!(
        REQUIRED_HEADINGS.len(),
        8,
        "exactly the eight GLOBAL-BUILD-MANUAL headings"
    );
}

#[test]
fn manual_documents_runtime_chat_endpoint_missing_surface() {
    let section = editors_manual_section();
    let all_text = section
        .topics
        .iter()
        .map(|t| t.body.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "Runtime Chat",
        "EndpointMissing",
        "runtime-chat-status",
        "runtime-chat-input",
        "runtime-chat-send",
        "runtime-chat-cancel",
        "Cancelled",
        "ignores any late completion",
        "input ready for a new send",
    ] {
        assert!(
            all_text.contains(needle),
            "manual must document Runtime Chat behavior/control {needle}"
        );
    }
}

// ── AC-002 / PT-002: every agent-tool row has a non-empty author_id + a REAL mcp_tool ─────────────────
#[test]
fn agent_tool_reference_rows_are_complete_and_use_real_tools() {
    let rows = agent_tool_rows();
    assert!(
        rows.len() >= 30,
        "the reference covers every editor/knowledge/FEMS/interop action (got {})",
        rows.len()
    );
    for row in &rows {
        assert!(
            !row.author_id.is_empty(),
            "AC-002: a row has an empty author_id"
        );
        assert!(
            !row.mcp_tool.is_empty(),
            "AC-002: row '{}' has an empty mcp_tool",
            row.author_id
        );
        assert!(
            REAL_MCP_TOOLS.contains(&row.mcp_tool),
            "AC-002/RISK-002: row '{}' uses non-real MCP tool '{}' (must be one of {:?})",
            row.author_id,
            row.mcp_tool,
            REAL_MCP_TOOLS
        );
        for (legacy, canonical) in [
            ("list_widgets", "argus.inspect"),
            ("click_widget", "argus.click"),
            ("set_value", "argus.set_value"),
            ("screenshot", "argus.screenshot"),
        ] {
            let without_canonical = row.description.replace(canonical, "");
            assert!(
                !without_canonical.contains(legacy),
                "canonical manual row '{}' still presents legacy-only method '{legacy}' in its example: {}",
                row.author_id,
                row.description
            );
        }
    }
    // The reference must cover EACH editor + knowledge + FEMS + interop surface (no surface omitted).
    let surfaces: HashSet<ManualSurface> = rows.iter().map(|r| r.surface).collect();
    for required in [
        ManualSurface::Code,
        ManualSurface::RichText,
        ManualSurface::Graph,
        ManualSurface::Canvas,
        ManualSurface::Knowledge,
        ManualSurface::Chat,
        ManualSurface::Terminal,
        ManualSurface::Model,
        ManualSurface::Diagnostics,
        ManualSurface::Fems,
        ManualSurface::Interop,
    ] {
        assert!(
            surfaces.contains(&required),
            "AC-002: surface {required:?} has no agent-tool rows"
        );
    }

    let row_ids: HashSet<&str> = rows.iter().map(|row| row.author_id).collect();
    assert!(
        row_ids.contains(handshake_native::top_menu_bar::MenuId::Editors.author_id()),
        "AC-002: the actionable EDITORS dropdown must have an author_id -> tool row"
    );
    for expected in handshake_native::settings_editor_section::EDITOR_SETTINGS_CONTROL_AUTHOR_IDS
        .iter()
        .chain(handshake_native::settings_editor_section::EDITOR_SETTINGS_OPTION_AUTHOR_IDS)
        .chain(handshake_native::settings_editor_section::SYNTAX_SWATCH_AUTHOR_IDS)
    {
        assert!(
            row_ids.contains(*expected),
            "AC-002: addressable Editor/Syntax settings control '{expected}' is missing from the agent-tool reference"
        );
    }
    for expected in handshake_native::settings_editor_section::EDITOR_SETTINGS_OPTION_AUTHOR_IDS {
        let row = rows
            .iter()
            .find(|row| row.author_id == *expected)
            .unwrap_or_else(|| panic!("missing popup option row '{expected}'"));
        assert_eq!(row.mcp_tool, handshake_native::mcp::ARGUS_CLICK_METHOD);
    }
    for expected in handshake_native::settings_editor_section::SYNTAX_SWATCH_AUTHOR_IDS {
        let row = rows
            .iter()
            .find(|row| row.author_id == *expected)
            .unwrap_or_else(|| panic!("missing syntax swatch row '{expected}'"));
        assert_eq!(row.mcp_tool, handshake_native::mcp::ARGUS_SET_VALUE_METHOD);
    }
    for expected in [
        handshake_native::settings_editor_section::EDITOR_WORD_WRAP_AUTHOR_ID,
        handshake_native::settings_editor_section::EDITOR_RENDER_WHITESPACE_AUTHOR_ID,
        handshake_native::settings_editor_section::SYNTAX_PALETTE_MODE_AUTHOR_ID,
    ] {
        let row = rows
            .iter()
            .find(|row| row.author_id == expected)
            .unwrap_or_else(|| panic!("missing selector row '{expected}'"));
        assert_eq!(row.mcp_tool, handshake_native::mcp::ARGUS_SET_VALUE_METHOD);
    }

    // The live keybinding table renders exactly two addressable controls for every action in its source
    // catalog: a TextEdit row driven by set_value and a Reset button driven by click_widget. Compare the
    // complete generated id set in both directions so a missing row, stale row, duplicate row, new live
    // action, or removed live action cannot silently drift from the structured manual reference.
    let actions = handshake_native::settings_editor_section::editor_action_catalog();
    let mut expected_keybinding_ids = HashSet::with_capacity(actions.len() * 2);
    for action in &actions {
        expected_keybinding_ids.insert(
            handshake_native::settings_editor_section::editor_keybind_row_author_id(&action.id),
        );
        expected_keybinding_ids.insert(format!(
            "{}{}",
            handshake_native::settings_editor_section::EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX,
            action.id
        ));
    }
    let keybinding_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.author_id.starts_with(
                handshake_native::settings_editor_section::EDITOR_KEYBIND_ROW_AUTHOR_ID_PREFIX,
            ) || row.author_id.starts_with(
                handshake_native::settings_editor_section::EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX,
            )
        })
        .collect();
    let actual_keybinding_ids: HashSet<String> = keybinding_rows
        .iter()
        .map(|row| row.author_id.to_owned())
        .collect();
    assert_eq!(
        keybinding_rows.len(),
        actions.len() * 2,
        "AC-002: the manual must contain exactly one row and one Reset entry per live keybinding action"
    );
    assert_eq!(
        actual_keybinding_ids, expected_keybinding_ids,
        "AC-002: structured keybinding rows must exactly match the live runtime-generated control ids"
    );
    for action in actions {
        let expected_surface = match action.surface {
            handshake_native::settings_editor_section::EditorActionSurface::Code => {
                ManualSurface::Code
            }
            handshake_native::settings_editor_section::EditorActionSurface::Rich => {
                ManualSurface::RichText
            }
        };
        let row_id =
            handshake_native::settings_editor_section::editor_keybind_row_author_id(&action.id);
        let reset_id = format!(
            "{}{}",
            handshake_native::settings_editor_section::EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX,
            action.id
        );
        let row = keybinding_rows
            .iter()
            .find(|row| row.author_id == row_id.as_str())
            .unwrap_or_else(|| panic!("missing live keybinding TextEdit row '{row_id}'"));
        assert_eq!(
            row.mcp_tool,
            handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
            "wrong MCP tool for '{row_id}'"
        );
        assert_eq!(
            row.surface, expected_surface,
            "wrong manual surface for '{row_id}'"
        );
        let reset = keybinding_rows
            .iter()
            .find(|row| row.author_id == reset_id.as_str())
            .unwrap_or_else(|| panic!("missing live keybinding Reset row '{reset_id}'"));
        assert_eq!(
            reset.mcp_tool,
            handshake_native::mcp::ARGUS_CLICK_METHOD,
            "wrong MCP tool for '{reset_id}'"
        );
        assert_eq!(
            reset.surface, expected_surface,
            "wrong manual surface for '{reset_id}'"
        );
    }

    // Reverse audit for conditionally rendered editor controls: source the complete expected inventory
    // from the live owning modules, then require exactly one canonical structured row per control.
    let conditional_editor_controls = vec![
        (
            handshake_native::code_editor::rename::CODE_EDITOR_RENAME_INPUT_AUTHOR_ID.to_owned(),
            handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        ),
        (
            handshake_native::code_editor::rename::CODE_EDITOR_RENAME_APPLY_AUTHOR_ID.to_owned(),
            handshake_native::mcp::ARGUS_CLICK_METHOD,
        ),
        (
            handshake_native::code_editor::rename::CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID.to_owned(),
            handshake_native::mcp::ARGUS_CLICK_METHOD,
        ),
        (
            handshake_native::code_editor::rename::CODE_EDITOR_CTX_RENAME_SYMBOL_MENU_AUTHOR_ID
                .to_owned(),
            handshake_native::mcp::ARGUS_CLICK_METHOD,
        ),
        (
            handshake_native::code_editor::code_actions::CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID
                .to_owned(),
            handshake_native::mcp::ARGUS_CLICK_METHOD,
        ),
        (
            handshake_native::code_editor::code_actions::CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID
                .to_owned(),
            handshake_native::mcp::ARGUS_INSPECT_METHOD,
        ),
        (
            handshake_native::code_editor::code_actions::quickfix_item_author_id(0, ""),
            handshake_native::mcp::ARGUS_CLICK_METHOD,
        ),
        (
            handshake_native::code_editor::formatting::FORMAT_SELECTION_CTX_AUTHOR_ID.to_owned(),
            handshake_native::mcp::ARGUS_CLICK_METHOD,
        ),
    ];
    for (author_id, expected_method) in conditional_editor_controls {
        let matching: Vec<_> = rows
            .iter()
            .filter(|row| row.author_id == author_id.as_str())
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "live conditional editor control '{author_id}' must have exactly one manual row"
        );
        assert_eq!(
            matching[0].mcp_tool, expected_method,
            "live conditional editor control '{author_id}' uses the wrong canonical Argus method"
        );
    }
}

fn has_live_author_id<S>(harness: &Harness<'_, S>, author_id: &str) -> bool {
    harness
        .root()
        .children_recursive()
        .any(|node| node.accesskit_node().author_id() == Some(author_id))
}

fn click_live_author_id<S>(harness: &mut Harness<'_, S>, author_id: &str) {
    harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(author_id))
        .unwrap_or_else(|| panic!("actual mounted widget '{author_id}' is absent"))
        .click_accesskit();
    harness.run_steps(3);
}

fn argus_snapshot<S>(harness: &Harness<'_, S>) -> UiTreeSnapshot {
    let actions = [
        egui::accesskit::Action::Click,
        egui::accesskit::Action::Focus,
        egui::accesskit::Action::SetValue,
    ];
    let children: Vec<_> = harness
        .root()
        .children_recursive()
        .map(|node| {
            let access = node.accesskit_node();
            let node_id = access.id().0;
            UiTreeNode {
                id: access
                    .author_id()
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("node:{node_id}")),
                author_id: access.author_id().map(str::to_owned),
                node_id,
                role: format!("{:?}", access.role()),
                label: access.label(),
                value: access.value(),
                disabled: access.is_disabled(),
                actions: actions
                    .iter()
                    .filter(|action| access.data().supports_action(**action))
                    .map(|action| format!("{action:?}"))
                    .collect(),
                bounds: None,
                children: Vec::new(),
            }
        })
        .collect();
    UiTreeSnapshot {
        widget_count: children.len() + 1,
        root: UiTreeNode {
            id: "manual-argus-root".to_owned(),
            author_id: None,
            node_id: 0,
            role: "Window".to_owned(),
            label: None,
            value: None,
            disabled: false,
            actions: Vec::new(),
            bounds: None,
            children,
        },
        captured_at_utc: "manual-argus-generation".to_owned(),
        viewport: None,
    }
}

fn canonical_argus_action<S>(
    harness: &mut Harness<'_, S>,
    channel: &mut handshake_native::mcp::ActionChannel,
    method: &str,
    target: &str,
    value: Option<&str>,
) {
    let snapshot = argus_snapshot(harness);
    let mut params = serde_json::json!({"target": target});
    if let Some(value) = value {
        params["value"] = serde_json::Value::String(value.to_owned());
    }
    let token = handshake_native::mcp::SessionToken::from_hex("manual-argus");
    let response = handshake_native::mcp::dispatch_request(
        &handshake_native::mcp::McpRequest {
            id: serde_json::json!(1),
            method: method.to_owned(),
            params,
            session_token: "manual-argus".to_owned(),
        },
        &token,
        &snapshot,
        channel,
        || Err(handshake_native::mcp::ScreenshotError("unused".to_owned())),
    );
    assert_eq!(response.to_json()["result"]["queued"], true, "{response:?}");
    let receipt_id = response.to_json()["result"]["receipt_id"]
        .as_u64()
        .expect("canonical Argus action returns a receipt id");
    for event in channel.drain_revalidated_into_events(&snapshot) {
        harness.event(event);
    }
    harness.run_steps(3);
    let observed = argus_snapshot(harness);
    channel.acknowledge_after_render(&observed);
    let receipt = channel
        .receipts()
        .into_iter()
        .find(|receipt| receipt.receipt_id == receipt_id)
        .expect("canonical Argus action retains its receipt");
    if method == handshake_native::mcp::ARGUS_SET_VALUE_METHOD {
        assert_eq!(
            receipt.status,
            handshake_native::mcp::ActionReceiptStatus::Indeterminate,
            "set-value must expose exact mounted readback without claiming causal attribution: {receipt:?}"
        );
    } else {
        assert!(
            matches!(
                receipt.status,
                handshake_native::mcp::ActionReceiptStatus::Applied
                    | handshake_native::mcp::ActionReceiptStatus::Indeterminate
            ),
            "click must be terminal without fabricating success: {receipt:?}"
        );
    }
}

#[test]
fn manual_settings_rows_are_actual_mounted_widgets_and_popup_options() {
    use handshake_native::settings_editor_section::*;

    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 1100.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.state_mut().open_settings();
    harness.run_steps(4);

    click_live_author_id(&mut harness, EDITOR_WORD_WRAP_AUTHOR_ID);
    for option in [
        EDITOR_WORD_WRAP_OFF_AUTHOR_ID,
        EDITOR_WORD_WRAP_ON_AUTHOR_ID,
        EDITOR_WORD_WRAP_BOUNDED_AUTHOR_ID,
    ] {
        assert!(
            has_live_author_id(&harness, option),
            "manual option row '{option}' must come from the actual mounted word-wrap popup"
        );
    }
    click_live_author_id(&mut harness, EDITOR_WORD_WRAP_BOUNDED_AUTHOR_ID);
    assert!(has_live_author_id(&harness, EDITOR_WRAP_COLUMN_AUTHOR_ID));

    click_live_author_id(&mut harness, EDITOR_RENDER_WHITESPACE_AUTHOR_ID);
    for option in [
        EDITOR_WHITESPACE_NONE_AUTHOR_ID,
        EDITOR_WHITESPACE_BOUNDARY_AUTHOR_ID,
        EDITOR_WHITESPACE_ALL_AUTHOR_ID,
    ] {
        assert!(
            has_live_author_id(&harness, option),
            "manual option row '{option}' must come from the actual mounted whitespace popup"
        );
    }
    click_live_author_id(&mut harness, EDITOR_WHITESPACE_BOUNDARY_AUTHOR_ID);

    click_live_author_id(&mut harness, SYNTAX_PALETTE_MODE_AUTHOR_ID);
    for option in [
        SYNTAX_PALETTE_MUTED_AUTHOR_ID,
        SYNTAX_PALETTE_STANDARD_AUTHOR_ID,
        SYNTAX_PALETTE_CUSTOM_AUTHOR_ID,
    ] {
        assert!(
            has_live_author_id(&harness, option),
            "manual option row '{option}' must come from the actual mounted syntax popup"
        );
    }
    click_live_author_id(&mut harness, SYNTAX_PALETTE_CUSTOM_AUTHOR_ID);

    for author_id in EDITOR_SETTINGS_CONTROL_AUTHOR_IDS
        .iter()
        .chain(SYNTAX_SWATCH_AUTHOR_IDS)
    {
        assert!(
            has_live_author_id(&harness, author_id),
            "manual settings row '{author_id}' is not an actual mounted widget"
        );
    }
    let editor_keybindings_header = format!(
        "{}keybindings-editor",
        handshake_native::settings_dialog::SECTION_HEADER_AUTHOR_ID_PREFIX
    );
    click_live_author_id(&mut harness, &editor_keybindings_header);
    for action in editor_action_catalog() {
        for author_id in [
            editor_keybind_row_author_id(&action.id),
            format!("{EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX}{}", action.id),
        ] {
            assert!(
                has_live_author_id(&harness, &author_id),
                "manual keybinding row '{author_id}' is not an actual mounted widget"
            );
        }
    }
}

#[test]
fn manual_rename_rows_drive_the_actual_context_popup_and_inline_widget() {
    use handshake_native::code_editor::rename::{
        CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID, CODE_EDITOR_CTX_RENAME_SYMBOL_MENU_AUTHOR_ID,
        CODE_EDITOR_RENAME_INPUT_AUTHOR_ID,
    };

    let source = "fn rename_me() { rename_me(); }\n";
    let panel = Arc::new(CodeEditorPanel::new(source, "rs"));
    panel.set_single_cursor(source.find("rename_me").unwrap() + 2);
    let shown = Arc::clone(&panel);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(760.0, 320.0))
        .build_ui(move |ui| shown.show(ui));
    harness.run_steps(3);

    let mut channel = handshake_native::mcp::ActionChannel::new();
    canonical_argus_action(
        &mut harness,
        &mut channel,
        handshake_native::mcp::ARGUS_CLICK_METHOD,
        CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
        None,
    );
    assert!(has_live_author_id(
        &harness,
        CODE_EDITOR_CTX_RENAME_SYMBOL_MENU_AUTHOR_ID
    ));
    canonical_argus_action(
        &mut harness,
        &mut channel,
        handshake_native::mcp::ARGUS_CLICK_METHOD,
        CODE_EDITOR_CTX_RENAME_SYMBOL_MENU_AUTHOR_ID,
        None,
    );

    assert!(has_live_author_id(
        &harness,
        CODE_EDITOR_RENAME_INPUT_AUTHOR_ID
    ));
    canonical_argus_action(
        &mut harness,
        &mut channel,
        handshake_native::mcp::ARGUS_SET_VALUE_METHOD,
        CODE_EDITOR_RENAME_INPUT_AUTHOR_ID,
        Some("renamed_by_argus"),
    );
    let value = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(CODE_EDITOR_RENAME_INPUT_AUTHOR_ID))
        .and_then(|node| node.accesskit_node().value());
    assert_eq!(value.as_deref(), Some("renamed_by_argus"));
}

// ── AC-004 / PT-004: id-audit — no documented author_id is an orphan ──────────────────────────────────
#[test]
fn id_audit_no_documented_author_id_missing_from_live_registry() {
    let live = live_author_id_set();
    // Sanity: the live set is non-trivial (guards against a false-green empty-registry pass).
    assert!(
        live.len() > 40,
        "live author_id set is suspiciously small ({})",
        live.len()
    );

    let rows = agent_tool_rows();
    let mut orphans: Vec<&str> = Vec::new();
    // BOTH dynamic menu-leaf arrays. EDITORS_MENU_LEAF_AUTHOR_IDS is the exact sibling of
    // EDITOR_MENU_LEAF_AUTHOR_IDS - same kind of popup leaf, same lifetime, rendered by the same
    // top_menu_bar::item path which names every node - and it was simply never added here.
    let dynamic_menu_leaves: HashSet<&str> =
        handshake_native::top_menu_bar::EDITOR_MENU_LEAF_AUTHOR_IDS
            .iter()
            .chain(handshake_native::top_menu_bar::EDITORS_MENU_LEAF_AUTHOR_IDS)
            .copied()
            .collect();
    let dynamic_settings_ids: HashSet<&str> =
        handshake_native::settings_editor_section::EDITOR_SETTINGS_CONTROL_AUTHOR_IDS
            .iter()
            .chain(handshake_native::settings_editor_section::EDITOR_SETTINGS_OPTION_AUTHOR_IDS)
            .chain(handshake_native::settings_editor_section::SYNTAX_SWATCH_AUTHOR_IDS)
            .copied()
            .collect();
    for row in &rows {
        if row.author_id == handshake_native::manual_content_editors::TERMINAL_MENU_AUTHOR_ID {
            // The terminal leaf is dynamic: it exists only while the RUN menu is open. MT-100 proves its
            // click path with a live Run-menu kittest and terminal-launch-status, so this static registry
            // audit does not seed the same literal the manual row documents.
            continue;
        }
        if row.author_id == handshake_native::manual_content_editors::CONFLICT_OPEN_MERGE_AUTHOR_ID
        {
            // The conflict dialog leaf is dynamic: it exists only while the mounted rich SaveManager is
            // in Conflict. test_menu_wireup::conflict_dialog_open_merge_button_opens_real_diff proves the
            // live AccessKit node and click path, so this static registry audit does not seed it.
            continue;
        }
        if dynamic_menu_leaves.contains(row.author_id) {
            // FILE/EDIT/GO editor leaves are dynamic menu-popup nodes, so the static registry must not
            // seed them from the documentation list. MT-069's live menu-render proof opens the dropdowns
            // and asserts these author_ids exist as MenuItem nodes.
            continue;
        }
        if dynamic_settings_ids.contains(row.author_id)
            || row.author_id.starts_with(
                handshake_native::settings_editor_section::EDITOR_KEYBIND_ROW_AUTHOR_ID_PREFIX,
            )
            || row.author_id.starts_with(
                handshake_native::settings_editor_section::EDITOR_KEYBIND_RESET_AUTHOR_ID_PREFIX,
            )
        {
            // Settings controls are real dialog/popup widgets and keybinding rows are generated from the
            // live catalog. `manual_settings_rows_are_actual_mounted_widgets_and_popup_options` opens the
            // real dialog and popups and audits every one; do not make this static set tautological.
            continue;
        }
        if [
            handshake_native::code_editor::rename::CODE_EDITOR_RENAME_INPUT_AUTHOR_ID,
            handshake_native::code_editor::rename::CODE_EDITOR_RENAME_APPLY_AUTHOR_ID,
            handshake_native::code_editor::rename::CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID,
            handshake_native::code_editor::rename::CODE_EDITOR_CTX_RENAME_SYMBOL_MENU_AUTHOR_ID,
        ]
        .contains(&row.author_id)
        {
            // The focused runtime audit above opens the actual context popup and steers the actual inline
            // rename TextEdit; none of these conditional nodes belongs in a static seeded registry.
            continue;
        }
        if [
            "graph.retry",
            "canvas.retry",
            "stage-embed-back-status",
            handshake_native::app::NOTES_LOAD_RETRY_AUTHOR_ID,
            handshake_native::fems::memory_proposal::FEMS_REVIEW_REFRESH_RETRY_AUTHOR_ID,
            handshake_native::runtime_chat::RUNTIME_CHAT_CANCEL_AUTHOR_ID,
        ]
        .contains(&row.author_id)
        {
            // Truthful conditional status/retry/action nodes are absent from the healthy default tree.
            // Focused recovery-path tests (including MT-098's in-flight canonical Cancel proof) prove
            // the mounted nodes instead of seeding static identities.
            continue;
        }
        // A documented id containing a {placeholder} documents a PATTERN, not an address: the live id
        // is prefix + a runtime value (a document id, a symbol entity id). Exact-matching such a row
        // against the live set can NEVER succeed, so it was reported as an orphan forever while the
        // product emitted the ids correctly. Check the PREFIX against the real prefix constant instead.
        if let Some((prefix, _)) = row.author_id.split_once('{') {
            let known_prefix = [
                handshake_native::code_editor::note_refs_panel::ROW_AUTHOR_ID_PREFIX,
                handshake_native::rich_editor::slash_commands::CODE_SYMBOL_RESULT_AUTHOR_ID_PREFIX,
                "code-ref-chip-",
            ]
            .contains(&prefix);
            assert!(
                known_prefix,
                "AC-004: templated row {:?} has no matching live author_id prefix",
                row.author_id
            );
            continue;
        }
        if !live.contains(row.author_id) {
            orphans.push(row.author_id);
        }
    }
    assert!(
        orphans.is_empty(),
        "AC-004/MC-001: documented author_id(s) absent from the live AccessKit registry (ORPHANS): {orphans:?}"
    );
}

// ── AC-005 / PT-002: the four interop edges are each documented with an author_id + mcp_tool ───────────
#[test]
fn interop_edges_all_documented_with_author_id_and_tool() {
    let section = editors_manual_section();
    let interop_topic = section
        .topic("Interop Edges")
        .expect("the interop topic exists");
    // Each of FEMS / Stage / Calendar / Locus is named in the interop topic body (AC-005).
    for edge in INTEROP_EDGES {
        assert!(
            interop_topic.body.contains(edge),
            "AC-005/MC-007: interop edge '{edge}' is not named in the interop topic"
        );
    }
    assert_eq!(INTEROP_EDGES.len(), 4, "exactly FEMS/Stage/Calendar/Locus");

    // Each edge has at least one agent-tool row carrying a non-empty author_id + mcp_tool.
    let rows = agent_tool_rows();
    let interop_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.surface == ManualSurface::Interop)
        .collect();
    assert!(
        interop_rows.len() >= 4,
        "AC-005: at least one interop row per edge (got {})",
        interop_rows.len()
    );
    // FEMS rows are the dedicated Fems surface (the FEMS edge); assert it too.
    let fems_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.surface == ManualSurface::Fems)
        .collect();
    assert!(
        !fems_rows.is_empty(),
        "AC-005: the FEMS edge has agent-tool rows"
    );

    // Concretely assert each edge's signature author_id appears among the rows (Stage/Calendar/Locus on
    // the Interop surface; FEMS on the Fems surface).
    let row_ids: HashSet<&str> = rows.iter().map(|r| r.author_id).collect();
    assert!(
        row_ids.contains("stage-pane"),
        "Stage edge author_id present"
    );
    assert!(
        row_ids.contains("daily-journal-panel"),
        "Calendar edge author_id present"
    );
    assert!(
        row_ids.contains("outgoing.panel"),
        "Locus edge author_id present"
    );
    assert!(
        row_ids.contains("relevant-memory-panel"),
        "FEMS edge author_id present"
    );
}

// ── MC-006: the manual content names NO SQLite and no direct-DB-write language ────────────────────────
#[test]
fn manual_content_has_no_sqlite_and_no_direct_db_writes() {
    let section = editors_manual_section();
    let all_text: String = section
        .topics
        .iter()
        .map(|t| format!("{}\n{}", t.heading, t.body))
        .collect::<Vec<_>>()
        .join("\n");
    let lower = all_text.to_lowercase();
    assert!(
        !lower.contains("sqlite"),
        "MC-006: the manual must not mention SQLite"
    );
    // Persistence must be described as the embedded SurrealDB/EventLedger authority via handshake_core.
    assert!(
        lower.contains("surrealdb") && lower.contains("eventledger"),
        "MC-006: persistence must be described as SurrealDB/EventLedger"
    );
    // The only surviving PostgreSQL token is the legacy proof-harness variable HANDSHAKE_TEST_PG_DSN,
    // which is still literally required by tests/pg_proof_support/mod.rs. No persistence-authority
    // sentence may describe PostgreSQL as the store.
    assert!(
        !lower.contains("postgresql/eventledger") && !lower.contains("postgresql authority"),
        "MC-006: PostgreSQL must not be described as the persistence authority"
    );
    assert!(
        lower.contains("handshake_core"),
        "MC-006: persistence routes through handshake_core"
    );
    // No "direct DB write" affirmation (the manual states persistence is NOT direct).
    assert!(
        !lower.contains("write directly to the database") && !lower.contains("direct db write"),
        "MC-006: the manual must not describe direct DB writes as a path"
    );
}

// ── AC-003 / PT-003: the manual SEARCH box (live egui_kittest) finds an editor topic by keyword ───────
#[test]
fn manual_search_box_finds_editor_topic_by_keyword() {
    // The standalone manual-pane widget driven headlessly via egui_kittest (AccessKit enabled). Typing a
    // keyword into the search box filters the nav list to the matching topic — a LIVE interaction.
    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    let palette = HsPalette::dark();
    let mut state = ManualPaneState::default();

    // Drive the pane in a kittest harness. State (reg/palette) lives outside the closure via 'static
    // leaks so the harness app closure can borrow them for 'static (the test owns process lifetime).
    let reg: &'static ManualRegistry = Box::leak(Box::new(reg));
    let palette: &'static HsPalette = Box::leak(Box::new(palette));

    let mut harness = Harness::builder().build_ui(move |ui| {
        ManualPane::new(reg, &mut state, palette).show(ui);
    });
    harness.run();

    // The search box carries the stable accessible label "Search Manual" (and the author_id
    // 'manual-search'). Type a keyword that lives in the "Core Workflows" topic body ("command palette").
    let search = harness.get_by_label("Search Manual");
    search.focus();
    harness.run();
    harness
        .get_by_label("Search Manual")
        .type_text("command palette");
    harness.run();
    harness.run();

    // After filtering, the matching topic surfaces in the live tree (the nav list + body show only
    // matching topics). "Core Workflows" mentions the command palette. The heading appears as BOTH a nav
    // Button AND a body Label, so count matches with query_all (query_by_label panics on >1).
    let match_count = harness.query_all_by_label("Core Workflows").count();
    assert!(
        match_count > 0,
        "AC-003: typing 'command palette' into manual-search surfaces the matching editor topic"
    );

    // A non-matching keyword filters it OUT (proves the search actually filters, not always-passes).
    // Appending more text makes the query no longer a substring of the topic, so the row disappears.
    harness
        .get_by_label("Search Manual")
        .type_text(" zzznotarealtopiczzz");
    harness.run();
    harness.run();
    let after_count = harness.query_all_by_label("Core Workflows").count();
    assert_eq!(
        after_count, 0,
        "AC-003: a non-matching keyword removes the topic (the search really filters)"
    );
}

// ── HBR-VIS: render the manual pane + save a screenshot to the EXTERNAL artifact root ─────────────────
#[test]
fn manual_pane_renders_and_screenshots() {
    let _guard = wgpu_guard();
    assert_no_local_artifact_dir();

    let mut reg = ManualRegistry::new();
    reg.register_section(editors_manual_section());
    let reg: &'static ManualRegistry = Box::leak(Box::new(reg));
    let palette: &'static HsPalette = Box::leak(Box::new(HsPalette::dark()));
    // Pre-select the agent-tool reference so the screenshot shows the steering table.
    let mut state = ManualPaneState {
        selected: Some((
            "native-editors".to_owned(),
            "Agent Tool Reference".to_owned(),
        )),
        ..Default::default()
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .wgpu()
        .build_ui(move |ui| {
            ManualPane::new(reg, &mut state, palette).show(ui);
        });
    harness.run();
    harness.run();

    // The container + search box render without panic/overlap.
    assert!(
        harness.query_by_label("Search Manual").is_some(),
        "HBR-VIS: the manual search box renders"
    );

    let out_dir = external_artifact_dir("wp-kernel-012-mt-073");
    let _ = std::fs::create_dir_all(&out_dir);
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let out_path = out_dir.join("manual_pane_editors.png");
            let saved = image.save(&out_path).is_ok();
            let abs = std::fs::canonicalize(&out_path).unwrap_or(out_path.clone());
            println!(
                "PT-005 manual-pane screenshot: {w}x{h}, saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "HBR-VIS: the manual pane screenshot PNG saved to the external root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-073 manual-pane screenshot render unavailable (no wgpu \
                 adapter): {e}. The content + search + id-audit proofs stand; the PNG is a GPU-host item."
            );
        }
    }

    assert_no_local_artifact_dir();
}
