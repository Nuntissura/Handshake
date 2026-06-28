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
//! - Every documented `mcp_tool` prefers the canonical Argus methods:
//!   `argus.inspect` / `argus.click` / `argus.set_value` / `argus.screenshot`. The compatibility
//!   primitives `list_widgets` / `click_widget` / `set_value` / `screenshot` remain valid aliases.
//!   The contract's invented `gui.invoke_action` / `gui.read_state` are NOT used.
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
    topics.push(ManualTopic {
        heading: "Argus Visual Inspection",
        body: argus_visual_inspection_body(),
    });
    topics.push(ManualTopic {
        heading: "Atelier Tools",
        body: atelier_tools_body(),
    });
    // The interop topic (its own addressable topic). AC-005/MC-007 assert all four edge names + an
    // author_id + mcp_tool appear in this topic's body.
    topics.push(ManualTopic {
        heading: "Interop Edges",
        body: interop_edges_body(),
    });
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
substrate built on command_registry.rs + event_bus.rs. The panes are: the VS-Code-class CODE editor, the \
Obsidian/Notion-class RICH-TEXT editor, the Loom GRAPH view, the CANVAS board, and the knowledge surfaces \
(folder tree / backlinks / outgoing links / collections). Every pane is addressable by a stable AccessKit \
author_id and steerable by Argus. A swarm agent discovers controls with argus.inspect, drives a button \
with argus.click{target:<author_id>}, types into a field with argus.set_value{target,value}, and sees \
the pixels with argus.screenshot — no screen-scraping and no keyboard simulation."
        .to_owned()
}

fn core_workflows_body() -> String {
    "Open a file in the code editor: select it in the project tree (left rail 'files' button \
left-rail.activity.files), then the file mounts in the code pane; save with editor.code.save (Ctrl+S). \
Edit rich-text/knowledge notes: type in the rich pane; toggle bold with editor.rich.format-bold (Ctrl+B), \
insert a block with editor.rich.insert-slash-command ('/'). Build a graph: pan with graph.pan-left/\
graph.pan-right, zoom with graph.zoom-in/graph.zoom-out, open a node with graph.open-node, connect blocks \
with graph.add-edge. Sketch on the canvas: add a card with canvas.add-card, place a Loom block with \
canvas.place-block, connect with canvas.add-edge. Drive FEMS: the relevant-memory-panel shows the \
retrieval capsule; propose a memory write with the fems-propose-dialog and confirm with \
fems-propose-confirm (NEVER an editor-direct commit). Move a selection between panes: select in code, \
copy (Ctrl+C), focus the rich pane, paste (Ctrl+V) — the shared clipboard + command/event bus carries it. \
Jump from a knowledge backlink to its target: click a wikilink chip or an outgoing.resolved.* row. Open \
the command palette (Ctrl+Shift+P, command-palette.dialog) and run a command by typing into \
command-palette.search."
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
steering surface (mcp/server.rs) speaks the JSON-RPC Argus tools argus.inspect / argus.click / \
argus.set_value / argus.screenshot over the per-session token written into the binding file. The older \
list_widgets / click_widget / set_value / screenshot names remain compatibility aliases. To open the \
manual itself, surface the manual-pane and type a keyword into manual-search."
        .to_owned()
}

fn inputs_and_outputs_body() -> String {
    "Inputs: a file path (code editor), a loom:// block reference (everything-is-a-block addressing, \
loom_address.rs), an atelier:// CKC ref dragged in from the atelier_side_panel, a graph node block id \
(graph.open-node), or a locus:// WP/MT reference. Outputs: edited buffers PERSISTED through the existing \
handshake_core APIs — PostgreSQL/EventLedger is the only durable authority, and the editors never write \
to a database directly; clipboard payloads on the shared clipboard; and command-bus / event-ledger \
events (event_bus.rs + the Flight Recorder) that record each editor action. A rich-text document saves \
to the knowledge-documents route family; the code editor saves the buffer through the same backend \
client. Nothing the editors emit bypasses handshake_core."
        .to_owned()
}

fn navigation_paths_body() -> String {
    "Keyboard + AccessKit navigation between panes: Tab/Shift+Tab moves focus across the live AccessKit \
tree; an agent moves focus with argus.click (Focus is a declared action on every control). The command \
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

fn argus_visual_inspection_body() -> String {
    "Argus is the named Rust-native native Handshake visual inspection/control surface. Models and \
operators use Argus to inspect panels, tabs, buttons, labels, bounds, disabled state, stable author_id \
values, and screenshots before claiming GUI or behavior work is complete. Argus is not an external \
foreground automation habit; it is built on the native AccessKit tree and the MCP tools already wired \
into the shell. The product-facing method names are argus.inspect, argus.click, argus.set_value, and \
argus.screenshot. The compatibility primitives remain list_widgets, click_widget, set_value, and \
screenshot.\n\
\n\
Use Argus first for GUI validation. The required loop is: inspect widgets -> use stable ids/author ids \
-> click/set value -> inspect again -> take screenshot/snapshot evidence. Start with argus.inspect, \
assert the exact stable author_id values, then use argus.screenshot as the visual companion for layout, \
overlap, readability, and rendered pixels. For parallel agents, include a top-level agent_label such as \
codex-a or worker-3, share Argus snapshot reads, and use the existing MCP lease/receipt path for \
mutations instead of coordinating through screen coordinates or hidden chat context.\n\
\n\
Argus must be quiet. It must not bring Handshake to the foreground, must not steal keyboard focus, must \
not steal mouse input, must not move the cursor, must not bring the app foreground, and must not steal \
mouse, keyboard, or focus. If Argus cannot inspect a surface, report not inspected and keep the verdict \
unclaimed until a non-intrusive proof path exists. If Argus cannot see or steer a GUI surface, that is \
technical debt and accepted scope for remediation in the active WP.\n\
\n\
Verification proof for this surface is Argus evidence, not a foreground manual look: run argus.inspect, \
drive argus.click or argus.set_value, re-run argus.inspect, and pair that structured tree with \
argus.screenshot when layout/readability/pixels matter. The current native proof commands are the \
focused Argus MCP tests and TCP steer-loop tests in the handshake-native crate; their successful path \
returns Argus metadata plus agent_id/agent_label receipts and appends mutating calls to the MCP \
ActionLog.\n\
\n\
Flight Recorder/EventLedger linkage posture: Argus MT-007 writes native MCP ActionLog entries for \
mutating requests, but durable Flight Recorder/EventLedger mirroring is DEFERRED-with-reason because \
this MT establishes the native visual/control facade and does not add persistent event writes. Do not \
claim EventLedger persistence for Argus until a follow-up diagnostics MT wires it. HBR-INT-009 posture: \
Tier 1 Flight Recorder is DEFERRED-with-reason as above; Tier 2 internal_diagnostics is \
DEFERRED-with-reason until Argus health/error/action events are exposed as native diagnostic events; \
Tier 3 Palmistry is DEFERRED-with-reason until the external watcher ingests Argus action/screenshot \
health. Current recovery is to use typed JSON-RPC errors, the MCP ActionLog, the binding file, and the \
Argus inspect/steer/screenshot loop."
        .to_owned()
}

fn atelier_tools_body() -> String {
    "Atelier is the main filling panel for the CKC/Posekit/Ingest tool family. The visible top-level \
module button is module-ckc and displays Atelier. It opens atelier-main-panel, not a four-window grid. \
Inside that panel, Castkit Codex is atelier-tab-ckc, Posekit is atelier-tab-posekit, and Ingest is \
atelier-tab-ingest. Use Argus to inspect the panel, click a tab, then re-inspect the active content \
region before claiming the workflow works.\n\
\n\
CKC starts from atelier-content-ckc and keeps the character/media intake and drag-source workflow. \
Posekit starts from atelier-content-posekit and exposes model-addressable controls for the current \
placeholder split-view workflow: atelier-pose-yaw-minus, atelier-pose-yaw-plus, atelier-pose-reset, \
atelier-pose-face-toggle, atelier-pose-body-toggle, atelier-pose-hands-toggle, \
atelier-pose-yaw-slider, atelier-pose-pitch-slider, and atelier-pose-zoom-slider. Ingest starts from \
atelier-content-ingest and exposes review controls atelier-ingest-pass, atelier-ingest-reject, \
atelier-ingest-unsure, and atelier-ingest-batch-tags.\n\
\n\
For models, the expected navigation path is module-ckc -> atelier-main-panel -> one of \
atelier-tab-ckc / atelier-tab-posekit / atelier-tab-ingest -> the active content region. If a control \
needed for CKC, Posekit, or Ingest cannot be found by stable author_id through Argus, treat that as a \
product gap to remediate before claiming visual or behavioral completion."
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
the current handshake_core build)."
        .to_owned()
}

fn recovery_steps_body() -> String {
    "Re-mount the pane from the docking menu (top_menu_bar.rs view menu / pane_registry.rs), or reset the \
layout from Settings. Re-run with a present display + clipboard (a GPU/clipboard host) when a headless \
runner lacked them. Re-query the live AccessKit registry with list_widgets to get the CURRENT author_id \
for a node after a layout change — never reuse a stale id; the canonical id source is \
accessibility/registry.rs + the live editor/knowledge action registries. Retry persistence after the \
typed backend error clears (a save conflict resolves once the newer revision is loaded). Where a step \
needs a backend capability that does not yet exist — the FEMS read route, the Stage embed-back route, the \
Calendar activity-span route, or the Locus read route — the editor surfaces a typed blocker and a visible \
empty-state rather than fabricating behavior; the cross-edge completes once the backend packet lands."
        .to_owned()
}

fn interop_edges_body() -> String {
    "The native editors melt together with four named pillars beyond CKC/Loom. Each edge has an editor-side \
AccessKit surface an agent drives TODAY; the backend route that completes the cross-edge round-trip is \
gated (EndpointMissing) in the current handshake_core build — an HONEST typed blocker, not a silent no-op.\n\
\n\
- FEMS (Pillar 12, typed memory): the relevant-memory-panel renders the retrieval capsule \
(relevant-memory-list); an agent reads it with argus.inspect and argus.screenshot. A review-gated memory-write \
proposal is opened at fems-propose-dialog and confirmed at fems-propose-confirm (argus.click). Cross-edge \
read is gated until the FEMS pack route exists.\n\
- Stage (Pillar 17): content is routed to the stage-pane (stage-routed-content); the agent embeds a \
capture back with stage-capture-embed-back (argus.click). The embed-back backend route is gated.\n\
- Calendar (Pillar 2): the daily-journal-panel binds a daily note to a CalendarEvent \
(daily-journal-date-header, daily-journal-calendar-event-chip) and shows a read-only activity strip \
(daily-journal-activity-strip). The ActivitySpan correlation route is gated.\n\
- Locus (Pillar 6): a locus:// WP/MT reference renders as an inline locus-ref chip in the rich editor, and \
the outgoing-links pane (outgoing.panel) lists resolved (outgoing.section.resolved) and unresolved \
(outgoing.section.unresolved) references. The Locus read route is gated. An agent drives all of these with \
argus.click / argus.inspect."
        .to_owned()
}

fn agent_tool_reference_body() -> String {
    "The agent-vision / steering index pairs every addressable editor/knowledge/FEMS/interop action with \
the real native Argus/MCP swarm tool that drives it. Use the Argus methods first: argus.inspect \
(discover the live AccessKit tree), argus.click{target:<author_id>} (activate a button/toggle/row), \
argus.set_value{target,value} (type into a text field), and argus.screenshot (capture the pixels). \
The compatibility primitives list_widgets, click_widget, set_value, and screenshot still work for older \
clients. Read the structured rows in the pane below; each row is author_id -> mcp_tool for a real, \
live-registered control. When multiple models share the live binding token, each request should include \
agent_label, for example agent_label:'codex-a', so receipts and the action log distinguish parallel \
agents without treating the label as authorization."
        .to_owned()
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────────
// Agent-tool reference rows: author_id -> REAL MCP tool, for every addressable surface.
// Every author_id here is cross-checked by the id-audit test against the live registries.
// ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// Build the full `author_id -> MCP tool` steering reference. Covers shell chrome, code-editor actions,
/// rich-text actions, graph actions, canvas actions, collection actions, FEMS, and the four interop edges
/// (Stage / Calendar / Locus / FEMS). Every `author_id` is a LIVE registered id (the id-audit asserts no
/// orphan); every `mcp_tool` is a real Argus/MCP method.
pub fn agent_tool_rows() -> Vec<AgentToolRow> {
    let mut rows: Vec<AgentToolRow> = Vec::new();

    // ── Shell chrome (the panes a swarm agent first reaches) ─────────────────────────────────────────
    // The LIVE command palette (command_palette.rs) emits the DOT-form ids (PALETTE_SEARCH_AUTHOR_ID =
    // "command-palette.search", PALETTE_LIST_AUTHOR_ID = "command-palette.list"), registered in
    // DECLARED_IDENTITIES + PALETTE_AUTHOR_IDS. Source the row author_ids from those consts so the steering
    // index always tracks the id the running app actually exposes (the interop hyphen-form
    // "command-palette-search" is emitted only inside a unit-test harness, never the live render loop).
    rows.push(AgentToolRow {
        author_id: PALETTE_SEARCH_AUTHOR_ID,
        surface: ManualSurface::Code,
        action_label: "Type a command into the palette",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'command-palette.search', value:'<command>'} filters the palette.",
    });
    rows.push(AgentToolRow {
        author_id: PALETTE_LIST_AUTHOR_ID,
        surface: ManualSurface::Code,
        action_label: "Read palette results",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reveals the command-palette.list rows for the agent to click.",
    });
    rows.push(AgentToolRow {
        author_id: "manual-search",
        surface: ManualSurface::Knowledge,
        action_label: "Search the manual",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'manual-search', value:'<keyword>'} filters manual topics.",
    });

    // ── Code editor: every CODE_ACTION_CATALOG entry as editor.code.<action> ─────────────────────────
    // Both momentary Buttons and ToggleButtons are ACTIVATED by a click (a toggle carries its toggled
    // state separately), so every code action is driven by argus.click{target:<author_id>}.
    for entry in CODE_ACTION_CATALOG {
        let author_id: &'static str = code_author_id_static(entry.action_id);
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::Code,
            action_label: entry.label,
            mcp_tool: "argus.click",
            description: "argus.click{target:<author_id>} activates this code-editor action.",
        });
    }

    // ── Rich-text editor: every rich_action_catalog() entry as editor.rich.<action> ──────────────────
    for entry in rich_action_catalog() {
        let author_id: &'static str = rich_author_id_static(entry.action_id);
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::RichText,
            action_label: entry.label,
            mcp_tool: "argus.click",
            description: "argus.click{target:<author_id>} activates this rich-text editor action.",
        });
    }

    // ── Graph controls (GRAPH_CONTROL_CATALOG) ───────────────────────────────────────────────────────
    for entry in GRAPH_CONTROL_CATALOG {
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Graph,
            action_label: entry.label,
            mcp_tool: "argus.click",
            description: "argus.click{target:<author_id>} drives this Loom graph control.",
        });
    }

    // ── Canvas controls (CANVAS_CONTROL_CATALOG) ─────────────────────────────────────────────────────
    for entry in CANVAS_CONTROL_CATALOG {
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Canvas,
            action_label: entry.label,
            mcp_tool: "argus.click",
            description: "argus.click{target:<author_id>} drives this canvas-board control.",
        });
    }

    // ── Collection controls (COLLECTION_CONTROL_CATALOG) ─────────────────────────────────────────────
    for entry in COLLECTION_CONTROL_CATALOG {
        rows.push(AgentToolRow {
            author_id: entry.author_id,
            surface: ManualSurface::Knowledge,
            action_label: entry.label,
            mcp_tool: "argus.click",
            description: "argus.click{target:<author_id>} drives this block-collection control.",
        });
    }

    // ── FEMS interop (Pillar 12) ─────────────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::fems::RELEVANT_MEMORY_PANEL_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Read the FEMS retrieval capsule",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect surfaces the relevant-memory-panel + its items for the agent to read.",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::RELEVANT_MEMORY_LIST_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Enumerate memory items",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reveals the relevant-memory-list rows (provenance-first capsule items).",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_PROPOSE_DIALOG_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Open a review-gated memory-write proposal",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'fems-propose-dialog'} opens the proposal (never a direct commit).",
    });
    rows.push(AgentToolRow {
        author_id: crate::fems::FEMS_PROPOSE_CONFIRM_AUTHOR_ID,
        surface: ManualSurface::Fems,
        action_label: "Confirm the memory-write proposal",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'fems-propose-confirm'} submits the review-gated proposal.",
    });

    // ── Stage interop edge (Pillar 17) ───────────────────────────────────────────────────────────────
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_PANE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: the Stage pane container",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect surfaces the stage-pane; an agent reads what was routed to Stage.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_ROUTED_CONTENT_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: routed content region",
        mcp_tool: "argus.screenshot",
        description: "argus.screenshot captures the stage-routed-content region for vision.",
    });
    rows.push(AgentToolRow {
        author_id: crate::stage_pane::STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Stage edge: embed a capture back into notes",
        mcp_tool: "argus.click",
        description: "argus.click{target:'stage-capture-embed-back'} embeds the Stage capture back (route gated).",
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
            "argus.click{target:'daily-journal-calendar-event-chip'} opens the bound event.",
    });
    rows.push(AgentToolRow {
        author_id: crate::graph::daily_journal_panel::DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Calendar edge: read-only ActivitySpan strip",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces the daily-journal-activity-strip (read-only correlation; route gated).",
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
            "argus.inspect reveals outgoing.section.unresolved rows (Locus read route gated).",
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

    #[test]
    fn argus_visual_inspection_topic_documents_native_model_flow() {
        let section = editors_manual_section();
        let topic = section
            .topic("Argus Visual Inspection")
            .expect("Argus Visual Inspection topic must exist");
        let body = topic.body.as_str();

        for required in [
            "native Handshake visual inspection/control surface",
            "argus.inspect",
            "argus.click",
            "argus.set_value",
            "argus.screenshot",
            "agent_label",
            "inspect widgets -> use stable ids/author ids -> click/set value -> inspect again -> take screenshot/snapshot evidence",
            "must not bring the app foreground",
            "must not steal mouse, keyboard, or focus",
            "technical debt",
            "accepted scope for remediation in the active WP",
            "Verification proof for this surface is Argus evidence",
            "Argus metadata plus agent_id/agent_label receipts",
            "MCP ActionLog",
            "Flight Recorder/EventLedger linkage posture",
            "DEFERRED-with-reason",
            "Tier 2 internal_diagnostics",
            "Tier 3 Palmistry",
            "Do not claim EventLedger persistence for Argus",
        ] {
            assert!(
                body.contains(required),
                "Argus manual topic must document: {required}"
            );
        }
    }
}
