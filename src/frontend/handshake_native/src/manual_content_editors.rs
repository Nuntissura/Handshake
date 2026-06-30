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
with argus.click{target:<author_id>}, replaces a field value with argus.set_value{target,value}, and \
sees the pixels with argus.screenshot — no screen-scraping and no foreground keyboard simulation."
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
-> click/set value -> inspect again -> take screenshot/snapshot evidence. argus.set_value replaces the \
target text field value; it is not an append operation. Start with argus.inspect, assert the exact stable \
author_id values, then use argus.screenshot as the visual companion for layout, overlap, readability, \
and rendered pixels. For parallel agents, include a top-level agent_label such as codex-a or worker-3, \
share Argus snapshot reads, and use the existing MCP lease/receipt path for mutations instead of \
coordinating through screen coordinates or hidden chat context.\n\
\n\
Argus must be quiet. It must not bring Handshake to the foreground, must not steal keyboard focus, must \
not steal mouse input, must not move the cursor, must not bring the app foreground, and must not steal \
mouse, keyboard, or focus. If Argus cannot inspect a surface, report not inspected and keep the verdict \
unclaimed until a non-intrusive proof path exists. If Argus cannot see or steer a GUI surface, that is \
technical debt and accepted scope for remediation in the active WP.\n\
\n\
Verification proof for this surface is Argus evidence, not a foreground manual look: run argus.inspect, \
drive argus.click or argus.set_value on TextInput targets, re-run argus.inspect, and pair that structured tree with \
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
CKC starts from atelier-content-ckc and is the character/avatar database surface. Inspect \
atelier-ckc-book-layout first: default CKC is a book-style two-pane work surface, not a four-window \
grid. The left page is atelier-ckc-book-left-media with the selected image/media viewer \
atelier-ckc-media-viewer, linked albums, source folders, source URLs, and image notes/tags. The right \
page is atelier-ckc-book-right-sheet with the editable character sheet and sheet utilities. \
atelier-ckc-book-middle is absent in default Sheet mode; click atelier-ckc-mode-sheet to return to \
that two-pane state. The middle page appears only after clicking \
atelier-ckc-mode-story, atelier-ckc-mode-notes, or atelier-ckc-mode-moodboard; those mode controls \
must keep atelier-ckc-book-left-media and atelier-ckc-book-right-sheet visible so the old CKC \
large-picture-left / work-surface-right pattern stays intact. Argus proof for CKC layout requires \
argus.inspect plus argus.screenshot at desktop and constrained sizes. If a model sees pane-a, pane-b, \
pane-c, pane-d, SplitLayoutWidget, a four-window grid, or a foreground popout inside Atelier, treat \
that as a layout regression.\n\
\n\
Inspect \
atelier-ckc-character-list to select a character, use atelier-ckc-character-create-name and \
atelier-ckc-character-create to create a new character shell, edit the current sheet in \
atelier-ckc-sheet-editor, and use atelier-ckc-sheet-save-version to append a new sheet version rather \
than overwriting the previous version. The current selection is atelier-ckc-selected-character. \
Downstream tools should reuse the visible typed refs instead of copying sheet text: \
atelier-ckc-character-ref exposes atelier://character/{character_internal_id}, \
atelier-ckc-sheet-version-ref exposes atelier://sheet/{character_internal_id}/{sheet_version_id}, and \
atelier-ckc-typed-ref-kind exposes the hsLink refKind character_sheet. The matching backend routes are \
GET/POST /atelier/characters, GET /atelier/characters/{character_internal_id}, GET/POST \
/atelier/characters/{character_internal_id}/sheet-versions, POST \
/atelier/characters/{character_internal_id}/sheet-versions/import, GET \
/atelier/sheet-versions/{version_id}, GET \
/atelier/sheet-versions/{version_id}/export?format=txt|json|safe-txt|safe-json, \
GET/POST /atelier/sheet-versions/{version_id}/artifact-links, GET/DELETE \
/atelier/sheet-artifact-links/{link_id}, \
GET /atelier/sheet-templates/default, GET /atelier/sheet-templates/default/safe-subset, and GET \
/atelier/sheet-field-suggestions?field_id=CHAR-ID-006&limit=20; \
POST writes require x-hsk-actor-id so parallel agents are attributable; the native Atelier client can \
be created with an explicit actor id instead of relying on the fallback CKC actor. New CKC characters created from \
the native panel request the built-in CHARACTER_SHEET__v2.00.txt template, normalize public_id to a single line, and create the first append-only \
sheet version with CHAR-ID-001 set to public_id and CHAR-ID-002 set to display_name. The short/SFW-safe \
sheet surface is LLM_SAFE_SUBSET__v2.00.json: a curated Field ID whitelist for the same v2.00 template, \
not a separate replacement sheet. Argus-visible sheet utility controls are atelier-ckc-template-status, \
atelier-ckc-template-load, atelier-ckc-safe-subset-load, atelier-ckc-import-editor, \
atelier-ckc-import-sheet-version, atelier-ckc-export-txt, atelier-ckc-export-json, \
atelier-ckc-export-safe-txt, atelier-ckc-export-safe-json, atelier-ckc-export-status, \
atelier-ckc-export-ref, atelier-ckc-export-preview, atelier-ckc-field-suggestion-field, atelier-ckc-field-suggestions-load, \
atelier-ckc-field-suggestions-list, atelier-ckc-sheet-artifact-list, atelier-ckc-sheet-artifact-kind, \
atelier-ckc-sheet-artifact-ref, atelier-ckc-sheet-artifact-manifest, atelier-ckc-sheet-artifact-label, \
atelier-ckc-sheet-artifact-role, atelier-ckc-sheet-artifact-actor, atelier-ckc-sheet-artifact-attach, atelier-ckc-sheet-artifact-attach-posekit, \
atelier-ckc-sheet-artifact-detach, atelier-ckc-sheet-artifact-reuse-ref, and atelier-ckc-sheet-artifact-status. Import uses the /sheet-versions/import route and appends supplied \
raw sheet text or a JSON export envelope as a new version after checking CHAR-ID-001 against the selected \
character. Export returns deterministic txt, json, safe-txt, or safe-json content plus content_hash and \
sheet refs; it does not open a foreground file dialog. After export, Argus should inspect \
atelier-ckc-export-ref for the file/version/hash and atelier-ckc-export-preview for the exported content. \
Field suggestions remember prior non-empty values \
per exact Field ID, ignore template placeholders such as <string>, render stable atelier-ckc-field-suggestion-* rows, and never auto-fill or rewrite input. \
Reusable sheet artifacts link ComfyUI renders, Comfy receipts, conditioning PNGs, OpenPose PNGs, and OpenPose JSON to the selected CKC sheet version. \
The row typed_ref is atelier://sheet-artifact/{link_id}; resolve it with GET /atelier/sheet-artifact-links/{link_id}. artifact_kind is one of openpose_json, openpose_png, conditioning_png, comfy_render, or comfy_receipt. \
Use atelier-ckc-sheet-artifact-list to inspect active links, set atelier-ckc-sheet-artifact-actor to the current Argus/model actor id, fill atelier-ckc-sheet-artifact-kind/ref/manifest/label/role, then click atelier-ckc-sheet-artifact-attach. \
Click atelier-ckc-sheet-artifact-attach-posekit after a Posekit export to attach the latest OpenPose PNG as cui_openpose_conditioning. \
Use atelier-ckc-sheet-artifact-reuse-ref as the model-readable reuse handle and atelier-ckc-sheet-artifact-detach for a soft detach; parallel agents must keep distinct actor ids so linked_by/detach_actor event metadata stays attributable. Do not delete artifact files from CKC UI. \
Sheet appends use \
expected_parent_version_id as the stale-head guard for parallel agents. A stale append returns \
error=stale_sheet_version plus character_ref, expected_parent_version_id, \
expected_parent_sheet_version_ref, current_head_version_id, and current_head_sheet_version_ref; reload \
the current head before retrying. After Argus edits or clicks CKC controls, drain the queued action, \
re-inspect the panel, and verify the editor text or visible sheet_version_ref changed before claiming \
the UI was steered.\n\
\n\
CKC story documents are native character documents under \
/atelier/characters/{character_internal_id}/documents?doc_type=story, but their controls are \
mode-gated. \
Click atelier-ckc-mode-story before expecting story controls. Click atelier-ckc-mode-notes for \
character sheet notes; edit atelier-ckc-character-notes-editor and click \
atelier-ckc-character-notes-apply to write back to the selected sheet notes field, then append a \
sheet version if persistence is needed. Character sheet notes are not image notes: image notes stay in \
atelier-ckc-media-notes-editor on the left media page and must not change when sheet notes are edited. \
Story reusable refs are atelier://document/{document_id}. Plural story rows use atelier-ckc-story-document-{document_id}; \
click a row to make atelier-ckc-story-doc-ref and atelier-ckc-story-editor target that document. \
Inspect atelier-ckc-story-doc-ref, edit the story body in atelier-ckc-story-editor, and click \
atelier-ckc-story-save. Story cards and beats live under \
/atelier/character-documents/{document_id}/story-cards and \
/atelier/character-documents/{document_id}/story-beats: inspect atelier-ckc-story-card-list, set \
atelier-ckc-story-card-title and atelier-ckc-story-card-body, click atelier-ckc-story-card-save, set \
atelier-ckc-story-beat-editor, and click atelier-ckc-story-beat-save. Click \
atelier-ckc-mode-moodboard before expecting moodboard controls. CKC moodboards are also native \
character documents, under /atelier/characters/{character_internal_id}/documents?doc_type=moodboard. \
Moodboard snapshots use /atelier/character-documents/{document_id}/moodboard/snapshots, the latest \
snapshot uses /atelier/character-documents/{document_id}/moodboard/latest, and reusable refs are \
atelier://moodboard/{snapshot_id}. Plural moodboard rows use \
atelier-ckc-moodboard-document-{document_id}; click a row to make atelier-ckc-moodboard-doc-ref, \
atelier-ckc-moodboard-latest-ref, atelier-ckc-moodboard-editor, atelier-ckc-moodboard-save, and \
atelier-ckc-moodboard-open target that document. Inspect atelier-ckc-moodboard-doc-ref and \
atelier-ckc-moodboard-latest-ref, set native snapshot JSON in atelier-ckc-moodboard-editor, click \
atelier-ckc-moodboard-save, inspect atelier-ckc-moodboard-canvas, then click \
atelier-ckc-moodboard-open to reload the selected latest snapshot into the native canvas board. \
Story, sheet notes, image notes, tag \
notes, and moodboards stay distinct but cross-linked; do not merge their storage or refs. Argus must \
inspect/click/set_value the story and moodboard controls. If Argus cannot see or steer them, that is \
technical debt and must be treated as a product gap before claiming the workflow works.\n\
\n\
CKC also keeps linked images, folders, albums, and image-level notes/tags attached to character sheets. \
Inspect atelier-ckc-linked-media-list to see album rows (`atelier-ckc-album-*`), media rows \
(`atelier-ckc-media-*`), folder provenance rows (`atelier-ckc-folder-*`), and source URL rows \
(`atelier-ckc-source-url-*`). Album refs use hsLink \
refKind media_album so they remain CKC collection chips and do not collide with renderable rich-editor \
album embeds. Media rows use hsLink refKind media and folder provenance rows use hsLink refKind folder; \
source URL rows use hsLink refKind source_url. Argus descriptions must include draggable atelier-ref \
metadata before a model treats a row as reusable. \
Create albums with atelier-ckc-album-create-name, atelier-ckc-album-create-tags, \
atelier-ckc-album-create-notes, and atelier-ckc-album-create. Select an album row, paste existing \
atelier://media/{asset_id} refs or raw UUIDs into atelier-ckc-album-link-asset-ids, optionally set \
atelier-ckc-album-link-source-path and atelier-ckc-album-link-source-url for per-album link provenance, \
then click atelier-ckc-album-link-assets; atelier-ckc-album-status reports create/link results. When an album \
shows members_next_offset, click its dynamic atelier-ckc-album-load-more-* row to fetch the next page \
through GET /atelier/media-albums/{collection_id}/items?offset=...&limit=200. Link requests \
may carry link-scoped source_path_ref/source_url_ref through POST /atelier/media-albums/{collection_id}/items \
so the same image asset can belong to different albums without losing per-link provenance. Edit image notes in atelier-ckc-media-notes-editor and image tags in \
atelier-ckc-media-tags-editor, then click atelier-ckc-media-save. These image notes and image tags are \
stored on media metadata and must stay separate from the character sheet notes in \
atelier-ckc-sheet-editor. The matching backend routes are GET/POST \
/atelier/characters/{character_internal_id}/media-albums, POST \
/atelier/media-albums/{collection_id}/items, and POST /atelier/media-assets/{asset_id}/notes-tags; POST \
writes require x-hsk-actor-id. Folder and source URL links are source_path_ref/source_url_ref typed refs, and image refs \
are atelier://media/{asset_id}. CKC search lives in the same tab: set atelier-ckc-search-query, optional \
rich tag filters in atelier-ckc-search-tags, optionally toggle selected-character/album/media scope \
with atelier-ckc-search-filter-character, atelier-ckc-search-filter-collection, and \
atelier-ckc-search-filter-media, optionally use selected-media dHash similarity with \
atelier-ckc-search-filter-similarity, choose atelier-ckc-search-mode-fuzzy, \
atelier-ckc-search-mode-vector, or atelier-ckc-search-mode-combined, then click \
atelier-ckc-search-run. Results appear under atelier-ckc-search-results with stable \
atelier-ckc-search-result-* rows and cite target_ref plus character_ref, sheet_version_ref, \
collection_ref, media_ref, or tag_ref as available. The status line atelier-ckc-search-status tells \
whether semantic CKC projection is available; vector mode reports llm_embedding+pgvector_projection \
when it uses the configured embedding model plus native pgvector projection, degrades to \
semantic_unavailable_no_embedding_model when no embedding model is configured, and can still rank \
selected-media hits through dHash image similarity. Rich CKC tag notes are \
separate from sheet notes, album notes, and image notes: edit atelier-ckc-tag-note-tag, \
atelier-ckc-tag-note-scope, and atelier-ckc-tag-note-editor, then click atelier-ckc-tag-note-save. The \
native backend routes are POST /atelier/ckc/search and POST /atelier/ckc/tag-notes; tag-note writes \
require x-hsk-actor-id. \
Posekit starts from atelier-content-posekit and is the native OpenRepose-style split-view workflow. \
Set or inspect atelier-pose-source-ref and atelier-pose-rig-id first, then inspect atelier-pose-state-readout and atelier-pose-split-view; the \
left viewport is atelier-pose-3d-viewport, the native 3D projection preview bound to source_ref plus rig lineage, and the OpenPose output viewport is \
atelier-pose-openpose-viewport. Drive rotation with atelier-pose-yaw-minus, atelier-pose-yaw-plus, \
atelier-pose-reset, atelier-pose-yaw-slider, atelier-pose-pitch-slider, and atelier-pose-zoom-slider; \
toggle exported marker layers with atelier-pose-face-toggle, atelier-pose-body-toggle, and \
atelier-pose-hands-toggle. Marker editing is staged before export: set atelier-pose-marker-family \
(body, face, left_hand, right_hand), atelier-pose-marker-index, atelier-pose-marker-x, \
atelier-pose-marker-y, and atelier-pose-marker-confidence, then click atelier-pose-marker-apply to \
replace an existing marker, atelier-pose-marker-remove to zero a marker, or atelier-pose-marker-add \
only when validation finds a safe empty slot. Use atelier-pose-marker-nudge-left, \
atelier-pose-marker-nudge-right, atelier-pose-marker-nudge-up, and atelier-pose-marker-nudge-down for \
one-pixel coordinate adjustments; atelier-pose-marker-reset clears staged edits. Always inspect \
atelier-pose-marker-status after a click: a rejected edit must leave the previous export preview intact. \
Lens/framing controls are atelier-pose-framing-preset, atelier-pose-framing-lens, \
atelier-pose-framing-padding-top, atelier-pose-framing-padding-right, \
atelier-pose-framing-padding-bottom, atelier-pose-framing-padding-left, and \
atelier-pose-framing-readout. Use full_body_with_feet plus bottom padding to force black-space style \
composition for ComfyUI full-body outputs. Export with atelier-pose-export-openpose, then inspect \
atelier-pose-export-status, atelier-pose-export-ref, and atelier-pose-export-preview. The native Rust \
generator contract is hsk.atelier.posekit.openpose_export@1: image/png plus OpenPose JSON with body 18, \
face 70, and hand 21 keypoint arrays, marker_edits, framing metadata, source_ref provenance, rig_id lineage, content_hash, artifact_ref, and \
backend ArtifactStore receipt JSON metadata. Backend receipt JSON preserves the exact marker_edits payload and framing so \
parallel-agent recovery can reconstruct the last intended marker operation instead of only the edit count. Backend exports expose png_artifact_ref and json_artifact_ref through Argus, plus their manifests and receipt_ref. \
Return to atelier-tab-ckc and click atelier-ckc-sheet-artifact-attach-posekit to store the latest Posekit OpenPose PNG on the active CKC sheet as a reusable ComfyUI conditioning artifact. Stored-rig backend exports must show \
pose_state.source_keypoint_projection.mode=native-rig-to-openpose and rerender OpenPose coordinates plus \
PNG/hash evidence when yaw, pitch, or zoom changes; procedural or no-rig previews must identify as \
procedural preview evidence, not source-rig projection. No-backend harnesses expose preview://atelier/posekit/openpose/.../receipt \
metadata only. Use argus.inspect on viewport/control IDs and \
argus.screenshot{} for a full-frame visual proof; screenshot target cropping is not supported yet. Both \
paths must be headless/non-intrusive: no foreground window, no keyboard capture, no mouse steal. Ingest starts from \
atelier-content-ingest. Select batches through stable atelier-intake-batch-{stable_batch_id} buttons, \
inspect atelier-ingest-batch-summary for canonical lane counts, set atelier-ingest-dataset-ref, \
atelier-ingest-character-ref, and atelier-ingest-actor, choose \
atelier-ingest-pass, atelier-ingest-reject, or atelier-ingest-unsure, then add batch metadata with \
atelier-ingest-batch-tags, atelier-ingest-batch-note, atelier-ingest-event, atelier-ingest-date, and \
atelier-ingest-location. Toggle atelier-ingest-link-passed to persist link intent metadata for passed \
rows; object-level CKC media linking occurs only when the backend intake batch already has a target \
collection. Click atelier-ingest-apply-batch to apply the full canonical backend batch; currently \
loaded rows are preview rows and visible-row overrides only. Inspect atelier-ingest-queue-readout, \
atelier-ingest-batch-summary, atelier-ingest-status, and atelier-ingest-last-receipt for request id, \
requested_by, applied_count, applied_preview_count, total_item_count, capped applied item ids, \
truncated_count, and stale-response status. If atelier-ingest-batch-summary reports \
canonical_counts_loaded=false, wait for the batch projection before applying; placeholder zero lane counts \
are not canonical. \
contact sheet staging uses atelier-ingest-contact-rows, \
atelier-ingest-contact-columns, atelier-ingest-contact-dpi, and \
atelier-ingest-contact-export. Facial quality/dedupe/identity profile hints live in \
atelier-ingest-facial-profile so the future native Facial ingest bridge can reuse the same review queue. Real \
expanded intake rows are exposed as atelier-ingest-item-{stable_item_id}; per-item triage buttons are \
atelier-ingest-item-{stable_item_id}-pass, atelier-ingest-item-{stable_item_id}-reject, and \
atelier-ingest-item-{stable_item_id}-unsure so parallel agents can inspect and stage row decisions without \
relying on the visible row order.\n\
\n\
For models, the expected navigation path is module-ckc -> atelier-main-panel -> one of \
atelier-tab-ckc / atelier-tab-posekit / atelier-tab-ingest -> the active content region. module-ingest \
is a compatibility shortcut that also opens the Atelier panel; still verify the internal \
atelier-tab-ingest tab is selected before staging Ingest work. If a control needed for CKC, Posekit, or \
Ingest cannot be found by stable author_id through Argus, treat that as a product gap to remediate \
before claiming visual or behavioral completion."
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
argus.set_value{target,value} (replace a text field value), and argus.screenshot (capture the pixels). \
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
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_CHARACTER_LIST_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC character database",
        mcp_tool: "argus.inspect",
        description: "argus.inspect surfaces the CKC character list and selected sheet version.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_BOOK_LAYOUT_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC book layout",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect confirms CKC uses a left-media/right-sheet book layout, not a four-window grid.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_BOOK_LEFT_MEDIA_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC left media page",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads the CKC left page with character images and media notes.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_BOOK_RIGHT_SHEET_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC right sheet page",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads the CKC right page with the editable character sheet.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_BOOK_MIDDLE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC middle work page",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads the optional CKC middle page after Story, Notes, or Moodboard mode is selected.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MEDIA_VIEWER_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC selected image viewer",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads the selected CKC image/media preview and source refs in the left page.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MODE_SHEET_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Switch CKC to sheet mode",
        mcp_tool: "argus.click",
        description: "argus.click{target:'atelier-ckc-mode-sheet'} hides the optional middle page.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MODE_STORY_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Switch CKC to story mode",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-mode-story'} opens the story middle page while keeping media and sheet visible.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MODE_NOTES_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Switch CKC to notes mode",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-mode-notes'} opens character sheet notes in the middle page.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MODE_MOODBOARD_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Switch CKC to moodboard mode",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-mode-moodboard'} opens the moodboard middle page and native canvas.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_CHARACTER_CREATE_NAME_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Type a new CKC character name",
        mcp_tool: "argus.set_value",
        description: "argus.set_value{target:'atelier-ckc-character-create-name', value:'<name>'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_CHARACTER_CREATE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Create a CKC character",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-character-create'} creates/selects a character shell.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit the selected CKC sheet",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-sheet-editor', value:'<sheet text>'} edits the sheet buffer.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_SAVE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Append a CKC sheet version",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-sheet-save-version'} appends a new version, preserving the previous one.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_VERSION_REF_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read the CKC sheet-version ref",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier://sheet/{character_internal_id}/{sheet_version_id}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_CHARACTER_NOTES_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit CKC character sheet notes",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-character-notes-editor', value:'<notes>'} edits sheet notes, not image notes.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_CHARACTER_NOTES_APPLY_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Apply CKC character sheet notes",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-character-notes-apply'} writes the notes back into the selected sheet text.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_TEMPLATE_STATUS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read CKC template status",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads the bundled CHARACTER_SHEET__v2.00.txt load/hash status.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_TEMPLATE_LOAD_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Load CKC full template metadata",
        mcp_tool: "argus.click",
        description: "argus.click{target:'atelier-ckc-template-load'} loads template metadata.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SAFE_SUBSET_LOAD_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Load CKC safe subset metadata",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-safe-subset-load'} loads the LLM-safe Field ID subset.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_IMPORT_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Paste CKC sheet import text",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-import-editor', value:'<raw sheet or JSON export>'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_IMPORT_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Import a CKC sheet version",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-import-sheet-version'} appends the import with stale-head guard.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_EXPORT_TXT_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Export CKC sheet txt",
        mcp_tool: "argus.click",
        description: "argus.click{target:'atelier-ckc-export-txt'} exports deterministic full txt.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_EXPORT_JSON_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Export CKC sheet json",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-export-json'} exports deterministic JSON that can be imported back.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_EXPORT_SAFE_TXT_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Export CKC safe txt",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-export-safe-txt'} exports the short/SFW Field ID subset as txt.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_EXPORT_SAFE_JSON_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Export CKC safe json",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-export-safe-json'} exports the short/SFW Field ID subset as JSON.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_EXPORT_STATUS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read CKC export/import status",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads the latest CKC import/export status and content hash.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_EXPORT_REF_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read CKC export ref",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-export-ref for export file, version id, and hash.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_EXPORT_PREVIEW_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC export content",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-export-preview for the exported sheet content.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_FIELD_SUGGESTION_FIELD_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Type a CKC Field ID",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-field-suggestion-field', value:'CHAR-ID-006'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_FIELD_SUGGESTIONS_LOAD_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Load CKC field suggestions",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-field-suggestions-load'} loads prior values for the exact Field ID.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_FIELD_SUGGESTIONS_LIST_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC field suggestions",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-field-suggestions-list without auto-filling the sheet.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_LIST_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC sheet artifacts",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads reusable sheet artifact rows with atelier://sheet-artifact/{link_id}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_KIND_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Set CKC sheet artifact kind",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-sheet-artifact-kind', value:'openpose_png'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_REF_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Set CKC sheet artifact ref",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-sheet-artifact-ref', value:'artifact://...'} sets the reusable artifact ref.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_MANIFEST_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Set CKC sheet artifact manifest",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-sheet-artifact-manifest', value:'manifest://...'} sets optional manifest/receipt provenance.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_LABEL_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Label CKC sheet artifact",
        mcp_tool: "argus.set_value",
        description: "argus.set_value{target:'atelier-ckc-sheet-artifact-label', value:'<label>'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ROLE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Set CKC sheet artifact reuse role",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-sheet-artifact-role', value:'cui_openpose_conditioning'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ACTOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Set CKC sheet artifact actor",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-sheet-artifact-actor', value:'<agent-or-model-id>'} sets write attribution before attach/detach.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ATTACH_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Attach CKC sheet artifact",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-sheet-artifact-attach'} links the artifact to the active sheet version.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_ATTACH_POSE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Attach latest Posekit export to CKC",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-sheet-artifact-attach-posekit'} links the latest Posekit OpenPose PNG as CUI conditioning.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_DETACH_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Detach CKC sheet artifact",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-sheet-artifact-detach'} soft-detaches the selected sheet artifact link.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_REUSE_REF_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read CKC sheet artifact reuse ref",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads the selected atelier://sheet-artifact/{link_id} typed ref for downstream tools.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SHEET_ARTIFACT_STATUS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read CKC sheet artifact status",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads CKC sheet artifact attach/detach status.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_LINKED_MEDIA_LIST_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC linked media",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads CKC album rows, media rows, folder refs, and image metadata.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_STATUS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read CKC album status",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads atelier-ckc-album-status for album create/link results.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_CREATE_NAME_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Name a CKC album",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-album-create-name', value:'<album name>'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_CREATE_TAGS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Tag a CKC album",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-album-create-tags', value:'tag, tag'} edits album tags.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_CREATE_NOTES_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Write CKC album notes",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-album-create-notes', value:'<album notes>'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_CREATE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Create a CKC album",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-album-create'} creates an album for the selected character.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_LINK_ASSET_IDS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Enter media IDs to link",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-album-link-asset-ids', value:'atelier://media/<asset_id>'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_LINK_SOURCE_PATH_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Set CKC album link source path",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-album-link-source-path', value:'atelier://folder/<source>'} sets per-link folder provenance.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_LINK_SOURCE_URL_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Set CKC album link source URL",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-album-link-source-url', value:'https://...'} sets per-link source URL provenance.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_ALBUM_LINK_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Link media to CKC album",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-album-link-assets'} links existing media assets into the selected album.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MEDIA_NOTES_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit CKC image notes",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-media-notes-editor', value:'<image notes>'}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MEDIA_TAGS_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit CKC image tags",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-media-tags-editor', value:'tag, tag'} edits image tags.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MEDIA_SAVE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Save CKC image notes/tags",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-media-save'} writes media notes/tags with actor attribution.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_QUERY_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Type a CKC search query",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-search-query', value:'<query>'} sets fuzzy/vector/combined search text.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_TAGS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Filter CKC search by tags",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-search-tags', value:'tag, tag'} applies rich tag filters.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_FILTER_CHARACTER_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Scope CKC search to the selected character",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-filter-character'} toggles selected-character search scope.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_FILTER_COLLECTION_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Scope CKC search to the selected album",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-filter-collection'} toggles selected-album search scope.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_FILTER_MEDIA_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Scope CKC search to the selected media",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-filter-media'} toggles selected-media search scope.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_FILTER_SIMILARITY_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Use selected media similarity for CKC search",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-filter-similarity'} uses the selected media asset as an image-similarity source when backend dHash projection exists.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_MODE_FUZZY_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Select CKC fuzzy search",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-mode-fuzzy'} selects typo-tolerant CKC search.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_MODE_VECTOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Select CKC vector search",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-mode-vector'} selects CKC semantic search; status reports llm_embedding+pgvector_projection when a model is available or semantic_unavailable_no_embedding_model when it is not.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_MODE_COMBINED_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Select CKC combined search",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-mode-combined'} selects fuzzy plus vector intersection search.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_RUN_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Run CKC search",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-search-run'} runs search and fills atelier-ckc-search-results.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_RESULTS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC search results",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-search-results and stable atelier-ckc-search-result-* rows.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_SEARCH_STATUS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Read CKC search status",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads semantic availability, vector source, pending state, and result counts.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_TAG_NOTE_TAG_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Type the CKC tag-note tag",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-tag-note-tag', value:'training'} selects the tag.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_TAG_NOTE_SCOPE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Type the CKC tag-note scope ref",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-tag-note-scope', value:'atelier://collection/...'} scopes the note.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_TAG_NOTE_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit a CKC rich tag note",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-tag-note-editor', value:'<note>'} edits the tag-note body.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_TAG_NOTE_SAVE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Save a CKC rich tag note",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-tag-note-save'} writes a CKC tag note with actor attribution.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_DOC_REF_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect the CKC story document ref",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-story-doc-ref as atelier://document/{document_id}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit the CKC story document body",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-story-editor', value:'<story>'} edits the native story document.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_SAVE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Save the CKC story document",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-story-save'} persists the native story document.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_CARD_LIST_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect CKC story cards",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-story-card-list for cards under the selected story document.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_CARD_TITLE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Type a CKC story card title",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-story-card-title', value:'<title>'} edits the story-card title.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_CARD_BODY_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Type a CKC story card body",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-story-card-body', value:'<body>'} edits the story-card body.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_CARD_SAVE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Save a CKC story card",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-story-card-save'} writes a story card under the selected document.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_BEAT_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit CKC story beats",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-story-beat-editor', value:'<beat>'} edits a story beat.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_STORY_BEAT_SAVE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Save a CKC story beat",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-story-beat-save'} writes a story beat under the selected document.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MOODBOARD_DOC_REF_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect the CKC moodboard document ref",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-moodboard-doc-ref for the native moodboard document.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MOODBOARD_LATEST_REF_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect the latest CKC moodboard snapshot ref",
        mcp_tool: "argus.inspect",
        description:
            "argus.inspect reads atelier-ckc-moodboard-latest-ref as atelier://moodboard/{snapshot_id}.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MOODBOARD_EDITOR_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Edit CKC moodboard snapshot JSON",
        mcp_tool: "argus.set_value",
        description:
            "argus.set_value{target:'atelier-ckc-moodboard-editor', value:'<snapshot-json>'} edits the selected native CKC moodboard snapshot.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MOODBOARD_SAVE_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Save a CKC moodboard snapshot",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-moodboard-save'} appends the selected moodboard document and records a native moodboard snapshot.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MOODBOARD_OPEN_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Open the CKC moodboard",
        mcp_tool: "argus.click",
        description:
            "argus.click{target:'atelier-ckc-moodboard-open'} opens the native moodboard surface.",
    });
    rows.push(AgentToolRow {
        author_id: crate::atelier_panel::ATELIER_CKC_MOODBOARD_CANVAS_AUTHOR_ID,
        surface: ManualSurface::Interop,
        action_label: "Inspect the CKC moodboard canvas",
        mcp_tool: "argus.inspect",
        description: "argus.inspect reads atelier-ckc-moodboard-canvas for native moodboard state.",
    });
    for (author_id, action_label, mcp_tool, description) in [
        (
            crate::atelier_panel::ATELIER_POSE_SOURCE_REF_AUTHOR_ID,
            "Set the Posekit source image ref",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-source-ref', value:'atelier://media/...'} sets the source image reference.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_RIG_ID_AUTHOR_ID,
            "Set the Posekit stored rig id",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-rig-id', value:'018f...'} binds export to a stored atelier_pose_rig; leave blank for procedural preview/export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_STATE_READOUT_AUTHOR_ID,
            "Inspect Posekit state",
            "argus.inspect",
            "argus.inspect reads source_ref, rig_id, yaw, pitch, zoom, and marker-layer state.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_SPLIT_VIEW_AUTHOR_ID,
            "Inspect the Posekit split view",
            "argus.inspect",
            "argus.inspect confirms the rig/source preview and OpenPose viewport are present together.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_3D_VIEWPORT_AUTHOR_ID,
            "Inspect the Posekit rig/source viewport",
            "argus.inspect",
            "argus.inspect confirms atelier-pose-3d-viewport is present as the native 3D projection preview, exposes source_ref/rig_id lineage and source_fingerprint, and distinguishes procedural-posekit-preview from rig-linked-native-preview; pair with argus.screenshot{} for full-frame proof.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_OPENPOSE_VIEWPORT_AUTHOR_ID,
            "Inspect the Posekit OpenPose viewport",
            "argus.inspect",
            "argus.inspect confirms atelier-pose-openpose-viewport is present as the OpenPose conditioning preview and rerenders metadata after yaw/pitch/zoom changes; pair with argus.screenshot{} for full-frame proof.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_YAW_MINUS_AUTHOR_ID,
            "Rotate Posekit yaw left",
            "argus.click",
            "argus.click{target:'atelier-pose-yaw-minus'} rotates the pose preview left.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_YAW_PLUS_AUTHOR_ID,
            "Rotate Posekit yaw right",
            "argus.click",
            "argus.click{target:'atelier-pose-yaw-plus'} rotates the pose preview right.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_RESET_AUTHOR_ID,
            "Reset Posekit pose",
            "argus.click",
            "argus.click{target:'atelier-pose-reset'} restores yaw, pitch, zoom, and default marker layers.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FACE_TOGGLE_AUTHOR_ID,
            "Toggle Posekit face markers",
            "argus.click",
            "argus.click{target:'atelier-pose-face-toggle'} includes or hides face markers.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_BODY_TOGGLE_AUTHOR_ID,
            "Toggle Posekit body markers",
            "argus.click",
            "argus.click{target:'atelier-pose-body-toggle'} includes or hides body markers.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_HANDS_TOGGLE_AUTHOR_ID,
            "Toggle Posekit hand markers",
            "argus.click",
            "argus.click{target:'atelier-pose-hands-toggle'} includes or hides hand markers.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_YAW_SLIDER_AUTHOR_ID,
            "Set Posekit yaw",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-yaw-slider', value:'90'} sets yaw for the next OpenPose render.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_PITCH_SLIDER_AUTHOR_ID,
            "Set Posekit pitch",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-pitch-slider', value:'0'} sets pitch for the next OpenPose render.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_ZOOM_SLIDER_AUTHOR_ID,
            "Set Posekit zoom",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-zoom-slider', value:'1.0'} sets render zoom for the next OpenPose export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_FAMILY_AUTHOR_ID,
            "Set Posekit marker family",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-marker-family', value:'face'} selects body, face, left_hand, or right_hand for the staged marker edit.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_INDEX_AUTHOR_ID,
            "Set Posekit marker index",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-marker-index', value:'12'} selects the OpenPose keypoint index inside the chosen marker family.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_X_AUTHOR_ID,
            "Set Posekit marker x",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-marker-x', value:'321'} stages the marker x coordinate for the next edit.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_Y_AUTHOR_ID,
            "Set Posekit marker y",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-marker-y', value:'222'} stages the marker y coordinate for the next edit.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_CONFIDENCE_AUTHOR_ID,
            "Set Posekit marker confidence",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-marker-confidence', value:'0.87'} stages the confidence; values outside 0.0..1.0 are rejected without overwriting the last export preview.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_APPLY_AUTHOR_ID,
            "Apply Posekit marker edit",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-apply'} stages a set-marker edit into the next OpenPose export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_ADD_AUTHOR_ID,
            "Add Posekit marker safely",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-add'} stages an add-marker edit only when validation finds an empty safe slot.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_REMOVE_AUTHOR_ID,
            "Remove Posekit marker",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-remove'} stages a zeroed marker removal for the next OpenPose export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_RESET_AUTHOR_ID,
            "Clear Posekit staged marker edits",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-reset'} clears staged marker edits without hiding the last export preview.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_NUDGE_LEFT_AUTHOR_ID,
            "Nudge Posekit marker left",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-nudge-left'} moves the staged coordinate left by one pixel.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_NUDGE_RIGHT_AUTHOR_ID,
            "Nudge Posekit marker right",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-nudge-right'} moves the staged coordinate right by one pixel.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_NUDGE_UP_AUTHOR_ID,
            "Nudge Posekit marker up",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-nudge-up'} moves the staged coordinate up by one pixel.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_NUDGE_DOWN_AUTHOR_ID,
            "Nudge Posekit marker down",
            "argus.click",
            "argus.click{target:'atelier-pose-marker-nudge-down'} moves the staged coordinate down by one pixel.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_MARKER_STATUS_AUTHOR_ID,
            "Inspect Posekit marker edit status",
            "argus.inspect",
            "argus.inspect reads whether the last marker edit was staged or rejected; rejected edits preserve the previous export preview.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FRAMING_PRESET_AUTHOR_ID,
            "Set Posekit framing preset",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-framing-preset', value:'full_body_with_feet'} selects standard, full_body_with_feet, portrait, or custom framing.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FRAMING_LENS_AUTHOR_ID,
            "Set Posekit framing lens",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-framing-lens', value:'24'} changes lens_mm for the next export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FRAMING_PADDING_TOP_AUTHOR_ID,
            "Set Posekit top framing padding",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-framing-padding-top', value:'48'} adds top black-space padding to the next export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FRAMING_PADDING_RIGHT_AUTHOR_ID,
            "Set Posekit right framing padding",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-framing-padding-right', value:'32'} adds right black-space padding to the next export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FRAMING_PADDING_BOTTOM_AUTHOR_ID,
            "Set Posekit bottom framing padding",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-framing-padding-bottom', value:'96'} adds bottom black-space padding to force full-body/feet framing.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FRAMING_PADDING_LEFT_AUTHOR_ID,
            "Set Posekit left framing padding",
            "argus.set_value",
            "argus.set_value{target:'atelier-pose-framing-padding-left', value:'32'} adds left black-space padding to the next export.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_FRAMING_READOUT_AUTHOR_ID,
            "Inspect Posekit framing readout",
            "argus.inspect",
            "argus.inspect reads preset, lens_mm, and padding values that will be written into the next OpenPose export metadata.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_EXPORT_AUTHOR_ID,
            "Export Posekit OpenPose",
            "argus.click",
            "argus.click{target:'atelier-pose-export-openpose'} dispatches the backend OpenPose export when connected; no-backend harnesses expose preview:// metadata only.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_EXPORT_STATUS_AUTHOR_ID,
            "Inspect Posekit export status",
            "argus.inspect",
            "argus.inspect reads whether the latest OpenPose export succeeded and which yaw it used.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_EXPORT_REF_AUTHOR_ID,
            "Inspect Posekit artifact refs",
            "argus.inspect",
            "argus.inspect reads png_artifact_ref, json_artifact_ref, receipt_ref, and content_hash for the latest OpenPose export; backend refs are ArtifactStore payloads, while no-backend preview uses preview://.",
        ),
        (
            crate::atelier_panel::ATELIER_POSE_EXPORT_PREVIEW_AUTHOR_ID,
            "Inspect Posekit export preview",
            "argus.inspect",
            "argus.inspect reads the hsk.atelier.posekit.openpose_export@1 preview payload; stored-rig backend exports must show source_keypoint_projection.mode=native-rig-to-openpose, png_artifact_ref, json_artifact_ref, and rerender OpenPose JSON/PNG hash evidence after yaw, pitch, or zoom changes.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_DATASET_REF_AUTHOR_ID,
            "Set Ingest dataset ref",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-dataset-ref', value:'dataset://...'} selects the image dataset or source folder to review.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_CHARACTER_REF_AUTHOR_ID,
            "Set Ingest CKC character ref",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-character-ref', value:'atelier://character/...'} selects the CKC sheet target for passed-image links.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_ACTOR_AUTHOR_ID,
            "Set Ingest actor",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-actor', value:'ingest-agent-017'} sets the backend actor id used by full-batch apply receipts.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_PASS_AUTHOR_ID,
            "Stage loaded Ingest rows as pass",
            "argus.click",
            "argus.click{target:'atelier-ingest-pass'} stages pass for every currently loaded Ingest row; use atelier-ingest-item-{stable_item_id}-pass for a single row.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_REJECT_AUTHOR_ID,
            "Stage loaded Ingest rows as reject",
            "argus.click",
            "argus.click{target:'atelier-ingest-reject'} stages reject for every currently loaded Ingest row; use atelier-ingest-item-{stable_item_id}-reject for a single row.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_UNSURE_AUTHOR_ID,
            "Stage loaded Ingest rows as unsure",
            "argus.click",
            "argus.click{target:'atelier-ingest-unsure'} stages unsure for every currently loaded Ingest row; use atelier-ingest-item-{stable_item_id}-unsure for a single row.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_BATCH_TAGS_AUTHOR_ID,
            "Set Ingest batch tags",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-batch-tags', value:'event:i76, outfit:...'} stages reusable tags for reviewed images.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_BATCH_NOTE_AUTHOR_ID,
            "Set Ingest batch note",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-batch-note', value:'<note>'} stages review notes separately from CKC sheet notes and image notes.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_EVENT_AUTHOR_ID,
            "Set Ingest event",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-event', value:'<event>'} stages event metadata for the dataset.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_DATE_AUTHOR_ID,
            "Set Ingest date",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-date', value:'YYYY-MM-DD'} stages date metadata for the dataset.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_LOCATION_AUTHOR_ID,
            "Set Ingest location",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-location', value:'<location>'} stages location metadata for the dataset.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_LINK_PASSED_AUTHOR_ID,
            "Toggle CKC link intent",
            "argus.click",
            "argus.click{target:'atelier-ingest-link-passed'} toggles structured CKC link intent metadata; backend object linking requires a target collection on the intake batch.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_APPLY_BATCH_AUTHOR_ID,
            "Apply full Ingest batch",
            "argus.click",
            "argus.click{target:'atelier-ingest-apply-batch'} applies the full canonical backend intake batch with structured metadata and actor attribution; loaded rows are only preview/override rows. Select batches via atelier-intake-batch-{stable_batch_id}; inspect atelier-ingest-status and atelier-ingest-last-receipt for requested_by, applied_count, applied_preview_count, total_item_count, truncated_count, and failures.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_CONTACT_ROWS_AUTHOR_ID,
            "Set Ingest contact-sheet rows",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-contact-rows', value:'3'} sets contact-sheet rows.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_CONTACT_COLUMNS_AUTHOR_ID,
            "Set Ingest contact-sheet columns",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-contact-columns', value:'4'} sets contact-sheet columns.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_CONTACT_DPI_AUTHOR_ID,
            "Set Ingest contact-sheet DPI",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-contact-dpi', value:'300'} sets contact-sheet export DPI.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_CONTACT_EXPORT_AUTHOR_ID,
            "Stage Ingest contact-sheet settings",
            "argus.click",
            "argus.click{target:'atelier-ingest-contact-export'} stages contact-sheet export settings for the current dataset.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_FACIAL_PROFILE_AUTHOR_ID,
            "Set Ingest Facial profile",
            "argus.set_value",
            "argus.set_value{target:'atelier-ingest-facial-profile', value:'quality+dedupe+identity'} stages Facial quality/dedupe/identity hints.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_QUEUE_READOUT_AUTHOR_ID,
            "Inspect Ingest queue readout",
            "argus.inspect",
            "argus.inspect reads dataset, character, decision, batch metadata, contact-sheet shape, and Facial profile state.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_BATCH_SUMMARY_AUTHOR_ID,
            "Inspect Ingest batch summary",
            "argus.inspect",
            "argus.inspect reads the canonical lane counts for the expanded backend batch: total, pending, accepted, rejected, deferred, skipped, failed, and visible_items. If canonical_counts_loaded=false, the backend projection is still loading and the summary must not be treated as canonical counts.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_STATUS_AUTHOR_ID,
            "Inspect Ingest status",
            "argus.inspect",
            "argus.inspect reads the last staged Ingest action and recovery/status message.",
        ),
        (
            crate::atelier_panel::ATELIER_INGEST_LAST_RECEIPT_AUTHOR_ID,
            "Inspect Ingest apply receipt",
            "argus.inspect",
            "argus.inspect reads the last backend request id, batch id, requested_by actor, applied_count, applied_preview_count, total_item_count, capped applied item ids, truncated_count, stale-response marker, and failed row.",
        ),
    ] {
        rows.push(AgentToolRow {
            author_id,
            surface: ManualSurface::Interop,
            action_label,
            mcp_tool,
            description,
        });
    }

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
