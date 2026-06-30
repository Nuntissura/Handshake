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
//!   * FEMS — `relevant-memory-panel` / `relevant-memory-list` / `fems-propose-dialog` /
//!     `fems-propose-confirm` ([`crate::fems`]);
//!   * Stage — `stage-pane` / `stage-routed-content` / `stage-capture-embed-back` ([`crate::stage_pane`]);
//!   * Calendar — `daily-journal-panel` / `daily-journal-date-header` /
//!     `daily-journal-calendar-event-chip` / `daily-journal-activity-strip` ([`crate::graph::daily_journal_panel`]);
//!   * Locus — `outgoing.panel` / `outgoing.section.resolved` / `outgoing.section.unresolved`
//!     ([`crate::rich_editor::wikilinks::outgoing_links_panel`]) — the locus-ref chip lives inline.
//! - Every documented `mcp_tool` is one of the FOUR REAL [`crate::mcp::tools`] methods:
//!   `list_widgets` / `click_widget` / `set_value` / `screenshot`. The contract's invented
//!   `gui.invoke_action` / `gui.read_state` are NOT used.
//!
//! ## Honest interop-edge gap note (RISK-007)
//!
//! FEMS, Stage, Calendar, and Locus each have an editor-side AccessKit surface that an agent can drive
//! TODAY, but the backend HTTP route that completes the cross-edge round-trip is ABSENT in the current
//! `handshake_core` build (verified: the FEMS read route, the Stage embed-back route, the Calendar
//! activity-span route, and the Locus read route return a typed `EndpointMissing`/gated empty-state).
//! The manual states this as a typed blocker rather than fabricating live cross-edge behavior.

use crate::accessibility::editor_action_registry::{rich_action_catalog, CODE_ACTION_CATALOG};
use crate::accessibility::{
    CANVAS_CONTROL_CATALOG, COLLECTION_CONTROL_CATALOG, GRAPH_CONTROL_CATALOG,
};
use crate::command_palette::{PALETTE_LIST_AUTHOR_ID, PALETTE_SEARCH_AUTHOR_ID};
use crate::manual_pane::{
    AgentToolReference, AgentToolRow, ManualSection, ManualSurface, ManualTopic,
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

pub const TERMINAL_MENU_AUTHOR_ID: &str = "menu.run.terminal";
pub const INFERENCE_LAB_MENU_AUTHOR_ID: &str = "menu.run.inference-lab";
pub const INFERENCE_LAB_PALETTE_AUTHOR_ID: &str =
    "command-palette.option.hs-inference-palette-open";
pub const FLIGHT_RECORDER_MENU_AUTHOR_ID: &str = "menu.run.flight-recorder";
pub const FLIGHT_RECORDER_PALETTE_AUTHOR_ID: &str = "command-palette.option.hs-flight-palette-open";
pub const SETTINGS_DIAGNOSTICS_SECTION_AUTHOR_ID: &str = "settings.section.diagnostics";

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
    ] {
        topics.push(ManualTopic { heading, body });
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
a button with click_widget{target:<author_id>}, types into a field with set_value{target,value}, and sees \
the pixels with screenshot — no screen-scraping and no keyboard simulation."
        .to_owned()
}

fn core_workflows_body() -> String {
    "Open a file in the code editor: select it in the project tree (left rail 'files' button \
left-rail.activity.files), then the file mounts in the code pane; save with editor.code.save (Ctrl+S). \
Open an existing knowledge note through the project tree or quick switcher: the shell opens a \
LoomWikiPage tab carrying the document id, performs GET /knowledge/documents/:id, and installs that \
backend content into the mounted rich-editor-surface. Edit rich-text/knowledge notes by typing in the \
rich pane; toggle bold with editor.rich.format-bold (Ctrl+B), insert a block with \
editor.rich.insert-slash-command ('/'), then save with FILE > Save, Ctrl+S, or editor.rich.save. The \
save path is the MT-020 SaveManager backed by PUT /knowledge/documents/:id/save with the loaded \
doc_version; reopening the same note invalidates the mounted state and forces a fresh GET before the \
editor is considered current. Build a graph: pan with graph.pan-left/\
graph.pan-right, zoom with graph.zoom-in/graph.zoom-out, open a node with graph.open-node, connect blocks \
with graph.add-edge. Sketch on the canvas: add a card with canvas.add-card, place a Loom block with \
canvas.place-block, connect with canvas.add-edge. Drive FEMS: the relevant-memory-panel shows the \
retrieval capsule; propose a memory write with the fems-propose-dialog and confirm with \
fems-propose-confirm (NEVER an editor-direct commit). Move a selection between panes: select in code, \
copy (Ctrl+C), focus the rich pane, paste (Ctrl+V) — the shared clipboard + command/event bus carries it. \
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
    cargo run -p handshake-native\n\
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
fems-propose-confirm), never an editor-direct commit."
        .to_owned()
}

fn common_failure_modes_body() -> String {
    "A pane fails to mount (the docking layout could not place the tile, or the host-mount carry MT-080 is \
not yet live). The clipboard daemon is missing on a headless CI runner so a copy/paste no-ops. A pane_id \
is stale after the pane was closed, so a stored swarm reference points at a node that is gone (deletion is \
signalled by ABSENCE from the AccessKit tree, not a tombstone). An AccessKit node is not found by an agent \
because its backing widget is not rendered this frame (a transient control like find-next while the find \
panel is closed is marked present=false and suppressed). The backend persistence API returns a typed error \
(e.g. a knowledge-document save conflict, or a FEMS/Stage/Calendar/Locus route that is EndpointMissing in \
the current handshake_core build). Runtime Chat send also returns EndpointMissing in this build; this is the \
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
revision is loaded). Where a step needs a backend capability that does not yet exist — the FEMS read route, \
the Stage embed-back route, the Calendar activity-span route, the Locus read route, or Runtime Chat \
assistant generation — the editor surfaces a typed blocker and a visible empty-state rather than fabricating \
behavior; the cross-edge or chat round-trip completes once the backend packet lands."
        .to_owned()
}

fn interop_edges_body() -> String {
    "The native editors melt together with four named pillars beyond CKC/Loom. Each edge has an editor-side \
AccessKit surface an agent drives TODAY; the backend route that completes the cross-edge round-trip is \
gated (EndpointMissing) in the current handshake_core build — an HONEST typed blocker, not a silent no-op.\n\
\n\
- FEMS (Pillar 12, typed memory): the relevant-memory-panel renders the retrieval capsule \
(relevant-memory-list); an agent reads it with list_widgets and screenshot. A review-gated memory-write \
proposal is opened at fems-propose-dialog and confirmed at fems-propose-confirm (click_widget). Cross-edge \
read is gated until the FEMS pack route exists.\n\
- Stage (Pillar 17): content is routed to the stage-pane (stage-routed-content); the agent embeds a \
capture back with stage-capture-embed-back (click_widget). The embed-back backend route is gated.\n\
- Calendar (Pillar 2): the daily-journal-panel binds a daily note to a CalendarEvent \
(daily-journal-date-header, daily-journal-calendar-event-chip) and shows a read-only activity strip \
(daily-journal-activity-strip). The ActivitySpan correlation route is gated.\n\
- Locus (Pillar 6): a locus:// WP/MT reference renders as an inline locus-ref chip in the rich editor, and \
the outgoing-links pane (outgoing.panel) lists resolved (outgoing.section.resolved) and unresolved \
(outgoing.section.unresolved) references. The Locus read route is gated. An agent drives all of these with \
click_widget / list_widgets."
        .to_owned()
}

fn notes_worksurface_and_chat_body() -> String {
    "The default WP-KERNEL-012 worksurface is editor-first and minimal: pane-a is the Code editor, \
pane-b is the Notes rich editor (LoomWikiPage / loom.wikipage class), and pane-c is Runtime Chat beside \
the editors. The manual and diagnostics are not docked into this default worksurface. A model discovers \
the current panes with list_widgets and addresses the seeded panes by pane-a / pane-b / pane-c, then uses \
the stable widget ids inside them: editor.code.* for code actions, editor.rich.* for Notes actions, \
runtime-chat-panel for the chat pane container, runtime-chat-status for the current chat route state, \
runtime-chat-input for the draft, and runtime-chat-send for the send button. Runtime Chat is honest in \
this build: no native HTTP assistant-chat endpoint exists, so a send probes the planned route and returns \
EndpointMissing instead of fabricating an assistant reply. Keep the main screen quiet and work-focused; \
advanced diagnostics stay behind Settings -> Diagnostics."
        .to_owned()
}

fn opening_editing_saving_notes_body() -> String {
    "Open an existing note from the project tree, quick switcher, a wikilink, or a graph/outgoing-link row. \
The shell opens a LoomWikiPage tab with the document id, performs GET /knowledge/documents/:id, parses \
content_json into the rich-editor document model, and binds SaveManager / DraftManager to that id and \
doc_version. Editing is live in the rich editor: type into the Notes pane, use editor.rich.format-bold, \
editor.rich.format-italic, editor.rich.insert-slash-command, wikilinks, backlinks, and properties exactly \
like the Obsidian-class note surface. Save through Ctrl+S, FILE > Save, or editor.rich.save. The authoritative \
save route is PUT /knowledge/documents/:id/save with expected_version and content_json; drafts use \
GET/PUT/DELETE /knowledge/documents/:id/draft for crash recovery. Reopening the same note invalidates stale \
mounted state and issues a fresh GET, so a no-context model should trust the reopened document and the \
EventLedger receipt, not an old widget value or cached editor state."
        .to_owned()
}

fn terminal_launch_body() -> String {
    "Terminal launch is documented as an honest typed blocker in this native frontend build. The top-menu \
Run item menu.run.terminal is visible as 'Open Terminal' but disabled, with the disclosed reason 'No native \
terminal panel yet'. The backend PTY runtime exists in handshake_core terminal/** and its TerminalRequest \
carries cwd plus command/args for the shell wrapper, but native Handshake currently has no reachable HTTP \
/terminal spawn route and no native terminal client; the typed native reach is EndpointMissing / IPC-only, \
with Tauri IPC as the existing working reach in the legacy app path. A model should use list_widgets on \
menu.run.terminal to read the disabled state and blocker. Do \
not claim a terminal opened, do not expect fake terminal output, and do not synthesize a cwd. The correct \
future behavior is 'Terminal: Open in Workspace Folder' issuing a real spawn in the repo folder through a \
native HTTP route or bridge, using the configured platform wrapper such as pwsh/cmd on Windows."
        .to_owned()
}

fn model_session_launch_body() -> String {
    "Model/session launch is split. The reachable native concept today is the Inference Lab surface: open \
it with menu.run.inference-lab or the generated command-palette row \
command-palette.option.hs-inference-palette-open (command id inferencelab.open). The backend POST /jobs \
family exists for reachable HTTP job creation and can represent a workspace-scoped model_run request, but \
live local/cloud model execution requires a managed handshake_core and remains NEEDS_MANAGED_RESOURCE_PROOF \
in this frontend-only manual context. The direct repo-folder-bound session spawn with wrapper is still \
IPC-only via kernel_swarm_spawn_session / cloud escalation commands in the legacy Tauri layer, so the native \
frontend must describe that half as EndpointMissing / IPC-only rather than a running model. A no-context \
model should open Inference Lab to inspect available launch/status UI, but it must not fabricate a session \
id, 'model running' state, local GGUF load, or cloud run result unless the real /jobs path returns one."
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
    "Flight Recorder is Tier 1: the backend business-event ledger. It is the canonical replay/audit record \
for application-level events that successfully reached the backend while the system is healthy enough to \
emit them. Use it for questions like 'what editor/save/job/event happened' and for replay/audit trails, \
not for detecting a frozen UI thread. In the native shell it is opened from Run -> Open Flight Recorder \
(menu.run.flight-recorder) or the generated command-palette row \
command-palette.option.hs-flight-palette-open, and it surfaces the native editor event stream as a readable \
pane. Backend routes such as GET /events are the durable authority for ledger reads. Flight Recorder is kept \
as-is by the diagnostics MTs; internal_diagnostics supplements it with in-app health, and Palmistry survives \
cases where the app cannot emit events."
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
not replace Flight Recorder's business ledger and it cannot by itself survive a fully dead process."
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

fn agent_tool_reference_body() -> String {
    "The agent-vision / steering index pairs every addressable editor/knowledge/FEMS/interop action with \
the REAL MCP swarm tool that drives it. The four tools are: list_widgets (discover the live AccessKit \
tree), click_widget{target:<author_id>} (activate a button/toggle/row), set_value{target,value} (type \
into a text field), and screenshot (capture the pixels). Read the structured rows in the pane below; each \
row is author_id -> mcp_tool for a real, live-registered control."
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
            description:
                "set_value{target:'command-palette.search', value:'<command>'} filters the palette.",
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
            description:
                "list_widgets surfaces runtime-chat-status with EndpointMissing and the probed route.",
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
            author_id: TERMINAL_MENU_AUTHOR_ID,
            surface: ManualSurface::Terminal,
            action_label: "Read disabled terminal launch blocker",
            mcp_tool: "list_widgets",
            description: "list_widgets surfaces menu.run.terminal as disabled: native terminal launch has no HTTP route/client; current reach is legacy Tauri IPC / IPC-only.",
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

    // ── Graph controls (GRAPH_CONTROL_CATALOG) ───────────────────────────────────────────────────────
    for entry in GRAPH_CONTROL_CATALOG {
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Graph,
            action_label: entry.label,
            mcp_tool: "click_widget",
            description: "click_widget{target:<author_id>} drives this Loom graph control.",
        });
    }

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

    // ── Collection controls (COLLECTION_CONTROL_CATALOG) ─────────────────────────────────────────────
    for entry in COLLECTION_CONTROL_CATALOG {
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Knowledge,
            action_label: entry.label,
            mcp_tool: "click_widget",
            description: "click_widget{target:<author_id>} drives this block-collection control.",
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
        author_id: crate::fems::FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Open a review-gated memory-write proposal",
        mcp_tool: "click_widget",
        description: "click_widget{target:'fems-propose-dialog'} opens the proposal (never a direct commit).",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_PROPOSE_CONFIRM_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Confirm the memory-write proposal",
        mcp_tool: "click_widget",
        description:
            "click_widget{target:'fems-propose-confirm'} submits the review-gated proposal.",
    });

    // ── Stage interop edge (Pillar 17) ───────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_PANE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: the Stage pane container",
        mcp_tool: "list_widgets",
        description:
            "list_widgets surfaces the stage-pane; an agent reads what was routed to Stage.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_ROUTED_CONTENT_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: routed content region",
        mcp_tool: "screenshot",
        description: "screenshot captures the stage-routed-content region for vision.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: embed a capture back into notes",
        mcp_tool: "click_widget",
        description: "click_widget{target:'stage-capture-embed-back'} embeds the Stage capture back (route gated).",
    });

    // ── Calendar interop edge (Pillar 2) ─────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_PANEL_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: the daily-journal panel",
        mcp_tool: "list_widgets",
        description:
            "list_widgets surfaces the daily-journal-panel (daily-note <-> CalendarEvent binding).",
    });
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: the daily-note date header",
        mcp_tool: "click_widget",
        description: "click_widget{target:'daily-journal-date-header'} opens the bound date.",
    });
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: a bound CalendarEvent chip",
        mcp_tool: "click_widget",
        description:
            "click_widget{target:'daily-journal-calendar-event-chip'} opens the bound event.",
    });
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: read-only ActivitySpan strip",
        mcp_tool: "list_widgets",
        description: "list_widgets surfaces the daily-journal-activity-strip (read-only correlation; route gated).",
    });

    // ── Locus interop edge (Pillar 6) ────────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::wikilinks::outgoing_links_panel::PANEL_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Locus edge: the outgoing-links pane",
        mcp_tool: "list_widgets",
        description:
            "list_widgets surfaces the outgoing.panel listing locus:// and wikilink references.",
    });
    rows.push(AgentToolRow {
        author_id: crate::rich_editor::wikilinks::outgoing_links_panel::RESOLVED_SECTION_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Locus edge: resolved references section",
        mcp_tool: "list_widgets",
        description:
            "list_widgets reveals outgoing.section.resolved rows (each navigable by click_widget).",
    });
    rows.push(AgentToolRow {
        author_id:
            crate::rich_editor::wikilinks::outgoing_links_panel::UNRESOLVED_SECTION_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Locus edge: unresolved (dangling) references section",
        mcp_tool: "list_widgets",
        description:
            "list_widgets reveals outgoing.section.unresolved rows (Locus read route gated).",
    });

    rows
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
