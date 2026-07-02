//! Static command catalog for the native Command Palette (WP-KERNEL-011 MT-016).
//!
//! ## What this is (the Rust port of `app/src/lib/app_command_registry.ts`)
//!
//! The React shell builds its command-palette action list at runtime from `buildAppCommandRegistry`
//! (the app-level `usermanual.*` / `settings.*` / `theme.*` etc. entries) plus the editor command
//! catalog (`editor_commands.ts`). The native shell does not have a runtime document/editor model yet,
//! so the catalog is COMPILE-TIME static: a `&'static [AppCommand]` computed once via [`OnceLock`]
//! (red-team MC2 — no per-frame allocation). Every entry maps to a REAL dispatch arm in
//! `app.rs::dispatch_palette_action`; there are no fake commands with no target (the contract's
//! "do NOT fake commands" rule).
//!
//! ## App vs Editor kind
//!
//! - [`CommandKind::App`] entries dispatch into existing shell state mutations (theme toggle, view-mode
//!   toggle, layout reset, settings open, navigate-to-tab). These are wired and runnable now.
//! - [`CommandKind::Editor`] entries are a representative subset of the React `EDITOR_COMMANDS` catalog
//!   (id/label/keywords ported verbatim). The native editor surface is a FUTURE MT, so editor commands
//!   are rendered DISABLED (`disabled: true`) — they appear in the palette so a model can SEE the full
//!   action surface, but they cannot be run until the editor pane lands. This mirrors how the React
//!   registry sets `disabled: !editorCommandsEnabled` when no editor is active. No fake-enable.
//!
//! ## Stable ids (HBR-SWARM)
//!
//! Each command carries a `stable_id` (the React `stableId`, kebab-case, `hs-` prefixed) — the
//! out-of-process address a swarm agent uses to dispatch the command through the palette. [`all_commands`]
//! IS the canonical list of actions a swarm agent may dispatch through the palette; it is the single
//! source of truth shared by the palette UI, the `matches_query` filter, and the `app.rs` dispatch match.

use std::sync::OnceLock;

/// Whether a command targets the shell (runnable now) or the editor surface (a future MT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    /// A shell-level action wired into existing `app.rs` state mutations (runnable now).
    App,
    /// An editor-surface action ported from the React `EDITOR_COMMANDS` catalog (rich-text block/format
    /// commands like Bold / Heading / Insert table). These need an ACTIVE rich-text DOCUMENT to mutate,
    /// which the native shell does not pin yet, so they stay disabled until that document model lands; the
    /// row is present so the full action surface is discoverable. No fake-enable.
    Editor,
    /// WP-KERNEL-012 MT-069 (E11 menu wire-up): an editor FILE/EDIT/GO menu + palette action that the
    /// WP-011 shell shipped as an honestly-DISABLED placeholder and that MT-079 has since made live by
    /// host-mounting the editor panes. Each one dispatches through the EXISTING shell single-substrate
    /// path by its stable command id — Undo/Redo reach the MT-035 unified-undo scope, Cut/Copy/Paste/
    /// SelectAll the MT-031 shared clipboard, Save the MT-020 editor save path, Find/Replace the focused
    /// editor's find family — NOT inline editor logic in the menu. Enabled when an editor pane is the
    /// focusable/active target (the live enable-predicate). DISTINCT from [`CommandKind::Editor`]: those
    /// are rich-text block authoring; these are the menu/palette editor commands MT-069 owns.
    EditorMenu,
}

/// One palette command. A direct port of the React `AppCommandDescriptor` shape (id, kind, label,
/// description, keywords, stableId, disabled). All fields are `&'static` so the whole catalog is a
/// compile-time constant the palette reads without allocating per frame.
#[derive(Debug, Clone, Copy)]
pub struct AppCommand {
    /// Stable command id the dispatcher matches on (e.g. `"usermanual.open"`, `"editor.format.bold"`).
    pub id: &'static str,
    /// Whether this is a shell (App) or editor (Editor) command.
    pub kind: CommandKind,
    /// Operator/model-facing label shown as the row's bold title.
    pub label: &'static str,
    /// Muted secondary description shown on the right of the row.
    pub description: &'static str,
    /// Search keywords folded into the `matches_query` haystack alongside label + description.
    pub keywords: &'static [&'static str],
    /// Stable out-of-process address (kebab-case, `hs-` prefixed) — the swarm-dispatch key.
    pub stable_id: &'static str,
    /// When true the row renders grayed and cannot be executed (Enter / click is a no-op). Editor
    /// commands are disabled until the native editor surface lands; no fake-enable.
    pub disabled: bool,
}

pub const CMD_TERMINAL_OPEN_WORKSPACE: &str = "terminal.open-workspace";
pub const TERMINAL_OPEN_WORKSPACE_STABLE_ID: &str = "hs-terminal-palette-open-workspace";
pub const CMD_MODEL_SESSION_LAUNCH_WORKSPACE: &str = "model-session.launch-workspace";
pub const MODEL_SESSION_LAUNCH_WORKSPACE_STABLE_ID: &str =
    "hs-model-session-palette-launch-workspace";

/// The canonical command catalog a swarm agent may dispatch through the palette (HBR-SWARM).
///
/// Returns a `&'static [AppCommand]` computed once (red-team MC2: static-ref access, no per-frame
/// allocation). The list is the single source of truth shared by:
/// - the palette UI (renders one row per command, filtered by [`matches_query`]),
/// - the `app.rs` `dispatch_palette_action` match (every enabled id has a real dispatch arm), and
/// - tests (no-duplicate-id proof + the live kittest open/type/run cases).
///
/// App commands are enabled (wired to existing shell state). Editor commands are a representative
/// subset of the React `EDITOR_COMMANDS` (id/label/keywords ported verbatim) and are disabled until the
/// native editor pane lands.
pub fn all_commands() -> &'static [AppCommand] {
    static CATALOG: OnceLock<Vec<AppCommand>> = OnceLock::new();
    CATALOG
        .get_or_init(|| {
            let mut v = Vec::new();
            v.extend_from_slice(APP_COMMANDS);
            v.extend_from_slice(EDITOR_MENU_COMMANDS);
            v.extend_from_slice(EDITOR_COMMANDS);
            v
        })
        .as_slice()
}

/// The shell-level (App) commands — each wired to a real `app.rs` dispatch arm (runnable now).
/// Ported from the `usermanual.*` / `settings.*` / `theme.*` etc. entries the React
/// `buildAppCommandRegistry` returns, plus the native shell's own view/layout/swarm actions.
const APP_COMMANDS: &[AppCommand] = &[
    AppCommand {
        id: "usermanual.open",
        kind: CommandKind::App,
        label: "UserManual: Open",
        description: "Open the in-app UserManual diagnostics tab.",
        keywords: &["manual", "usermanual", "help", "diagnostics", "usermanual.open"],
        stable_id: "hs-usermanual-palette-open",
        disabled: false,
    },
    AppCommand {
        id: "usermanual.search",
        kind: CommandKind::App,
        label: "UserManual: Search",
        description: "Open UserManual search.",
        keywords: &["manual", "usermanual", "search", "help", "usermanual.search"],
        stable_id: "hs-usermanual-palette-search",
        disabled: false,
    },
    AppCommand {
        id: "settings.open",
        kind: CommandKind::App,
        label: "Settings: Open",
        description: "Open the Handshake settings dialog.",
        keywords: &["settings", "preferences", "options", "config"],
        stable_id: "hs-settings-palette-open",
        disabled: false,
    },
    AppCommand {
        id: "theme.toggle",
        kind: CommandKind::App,
        label: "View: Toggle Theme",
        description: "Switch between the dark and light theme.",
        keywords: &["theme", "dark", "light", "view", "appearance"],
        stable_id: "hs-theme-palette-toggle",
        disabled: false,
    },
    AppCommand {
        id: "viewmode.toggle",
        kind: CommandKind::App,
        label: "View: Toggle View Mode",
        description: "Switch between NSFW and SFW content mode.",
        keywords: &["viewmode", "sfw", "nsfw", "view", "content"],
        stable_id: "hs-viewmode-palette-toggle",
        disabled: false,
    },
    AppCommand {
        id: "layout.reset",
        kind: CommandKind::App,
        label: "Layout: Reset",
        description: "Reset the work-surface layout to its default.",
        keywords: &["layout", "reset", "panes", "default"],
        stable_id: "hs-layout-palette-reset",
        disabled: false,
    },
    AppCommand {
        id: "swarmboard.open",
        kind: CommandKind::App,
        label: "Swarm: Open Board",
        description: "Open the Swarm board on the active pane.",
        keywords: &["swarm", "board", "agents", "run"],
        stable_id: "hs-swarm-palette-open-board",
        disabled: false,
    },
    AppCommand {
        id: "inferencelab.open",
        kind: CommandKind::App,
        label: "Run: Open Inference Lab",
        description: "Open the Inference Lab on the active pane.",
        keywords: &["inference", "lab", "model", "run"],
        stable_id: "hs-inference-palette-open",
        disabled: false,
    },
    AppCommand {
        id: "flightrecorder.open",
        kind: CommandKind::App,
        label: "Run: Open Flight Recorder",
        description: "Open the Flight Recorder on the active pane.",
        keywords: &["flight", "recorder", "trace", "run"],
        stable_id: "hs-flight-palette-open",
        disabled: false,
    },
    AppCommand {
        id: CMD_MODEL_SESSION_LAUNCH_WORKSPACE,
        kind: CommandKind::App,
        label: "Model Session: Launch in Workspace Folder",
        description: "Open a compact launch dialog that issues real POST /jobs for model_run and reports EndpointMissing for direct repo-folder spawn while it remains Tauri IPC-only.",
        keywords: &[
            "model",
            "session",
            "launch",
            "workspace",
            "repo",
            "folder",
            "local",
            "cloud",
            "wrapper",
            "jobs",
            "endpointmissing",
            "kernel_swarm_spawn_session",
        ],
        stable_id: MODEL_SESSION_LAUNCH_WORKSPACE_STABLE_ID,
        disabled: false,
    },
    AppCommand {
        id: CMD_TERMINAL_OPEN_WORKSPACE,
        kind: CommandKind::App,
        label: "Terminal: Open in Workspace Folder",
        description: "Runs the native terminal launch affordance; until HTTP /terminal/sessions exists it surfaces EndpointMissing with the current Tauri IPC-only reach.",
        keywords: &[
            "terminal",
            "shell",
            "workspace",
            "repo",
            "folder",
            "wrapper",
            "endpointmissing",
        ],
        stable_id: TERMINAL_OPEN_WORKSPACE_STABLE_ID,
        disabled: false,
    },
    AppCommand {
        id: "pane.next",
        kind: CommandKind::App,
        label: "Go: Next Pane",
        description: "Move focus to the next pane.",
        keywords: &["pane", "next", "focus", "go"],
        stable_id: "hs-pane-palette-next",
        disabled: false,
    },
    AppCommand {
        id: "pane.prev",
        kind: CommandKind::App,
        label: "Go: Previous Pane",
        description: "Move focus to the previous pane.",
        keywords: &["pane", "previous", "prev", "focus", "go"],
        stable_id: "hs-pane-palette-prev",
        disabled: false,
    },
    AppCommand {
        id: "drawer.project.toggle",
        kind: CommandKind::App,
        label: "View: Toggle Project Drawer",
        description: "Show or hide the left project drawer.",
        keywords: &["drawer", "project", "rail", "toggle", "view"],
        stable_id: "hs-drawer-palette-project",
        disabled: false,
    },
    AppCommand {
        id: "drawer.bottom.toggle",
        kind: CommandKind::App,
        label: "View: Toggle Bottom Panel",
        description: "Show or hide the bottom stash drawer.",
        keywords: &["drawer", "bottom", "stash", "panel", "toggle", "view"],
        stable_id: "hs-drawer-palette-bottom",
        disabled: false,
    },
    // WP-KERNEL-012 MT-033 (E5 — route-to-Stage): the discoverable palette entry for the Route-to-Stage
    // melt-together command. The command itself is dispatched on the MT-031 InteractionBus (which carries
    // the StageContent payload via request_route_to_stage); this catalog row makes the action SEEABLE in
    // the palette (HBR-SWARM) and maps to the `interop.route-to-stage` dispatch. Enabled (the local Stage
    // pane + bus command need no editor surface to exist — they route whatever the focused pane staged).
    AppCommand {
        id: "interop.route-to-stage",
        kind: CommandKind::App,
        label: "Route to Stage",
        description: "Send the current selection, document, or CKC item to the Stage pane.",
        keywords: &["route", "stage", "send", "ckc", "selection", "interop"],
        stable_id: "hs-stage-palette-route",
        disabled: false,
    },
    // WP-KERNEL-012 MT-066 (E10 — Stage embed-back): the discoverable palette entry for the
    // "Embed Stage Capture" command — the embed-back leg of the Stage round-trip. It fetches a Stage
    // capture artifact (with its SHA-256 manifest provenance) and inserts it into the focused note/canvas
    // as an MT-014 embed NodeView. This is the NEW command this MT adds; the route-to-stage command
    // (`interop.route-to-stage`, above) is REUSED from MT-033, NOT duplicated (AC-005/MC-003). The Stage
    // embed-back backend route is ABSENT in this build, so the runtime handler raises the typed blocker
    // `StageInteropError::EmbedBackEndpointAbsent` and the Stage pane shows the empty-state — never a fake
    // artifact. Enabled (palette-driven; no keybind — does NOT steal a VS Code binding).
    AppCommand {
        id: "interop.embed-stage-capture",
        kind: CommandKind::App,
        label: "Embed Stage Capture",
        description: "Insert a Stage capture artifact (with SHA-256 manifest provenance) into the focused note or canvas.",
        keywords: &["embed", "stage", "capture", "artifact", "provenance", "interop", "pillar 17"],
        stable_id: "hs-stage-palette-embed-capture",
        disabled: false,
    },
    // WP-KERNEL-012 MT-064 (E9 — FEMS memory-write proposal): the discoverable palette entry for the
    // "Propose to Memory" command. It turns the current selection into a review-gated FEMS memory-write
    // PROPOSAL (never a direct commit) and submits it to the review-gated FEMS write path; the proposal
    // builder + dialog + typed blocker + FR payload shape live in `fems::memory_proposal`. The runtime
    // handler is registered on the MT-031 InteractionBus (the same WRAP-not-fork split route-to-stage
    // uses) via `fems::memory_proposal::register_propose_to_memory_command`; this catalog row makes the
    // action SEEABLE + addressable in the palette (HBR-SWARM, AC-006). Enabled (palette-driven; no
    // keybind — does NOT steal a VS Code binding, RISK-010). The single source of truth for the id +
    // label is `fems::memory_proposal::{FEMS_PROPOSE_COMMAND_ID, FEMS_PROPOSE_COMMAND_LABEL}`.
    AppCommand {
        id: crate::fems::memory_proposal::FEMS_PROPOSE_COMMAND_ID,
        kind: CommandKind::App,
        label: crate::fems::memory_proposal::FEMS_PROPOSE_COMMAND_LABEL,
        description: "Propose the current selection as a review-gated FEMS memory write (never a direct commit).",
        keywords: &["memory", "fems", "propose", "pillar 12", "review", "episodic", "semantic", "procedural"],
        stable_id: "hs-fems-palette-propose-to-memory",
        disabled: false,
    },
    // WP-KERNEL-012 MT-067 (E10 — Calendar/Pillar 2 interop): the three daily-note <-> Calendar bus
    // commands. The daily journal panel + MT-030's calendar pane communicate ONLY through these bus
    // commands (NO calendar-pane internal import — RISK-4/MC-4). The single source of truth for the ids is
    // `crate::interop::calendar_interop::{CMD_OPEN_DAILY_NOTE_FOR_DATE, CMD_FOCUS_CALENDAR_EVENT,
    // CMD_OPEN_DOCUMENT}`. All three are palette-driven (no keybind — does NOT steal a VS Code binding).
    AppCommand {
        id: crate::interop::calendar_interop::CMD_OPEN_DAILY_NOTE_FOR_DATE,
        kind: CommandKind::App,
        label: "Daily Note: Open for Date",
        description: "Open or create the daily note for the selected calendar date (idempotent — one note per date).",
        keywords: &["daily", "note", "journal", "calendar", "date", "open", "pillar 2", "interop"],
        stable_id: "hs-daily-note-palette-open-for-date",
        disabled: false,
    },
    AppCommand {
        id: crate::interop::calendar_interop::CMD_FOCUS_CALENDAR_EVENT,
        kind: CommandKind::App,
        label: "Daily Note: Focus Calendar Event",
        description: "Focus or open the linked calendar event for the current daily note in the calendar pane.",
        keywords: &["daily", "note", "calendar", "event", "focus", "pillar 2", "interop"],
        stable_id: "hs-daily-note-palette-focus-calendar-event",
        disabled: false,
    },
    AppCommand {
        id: crate::interop::calendar_interop::CMD_OPEN_DOCUMENT,
        kind: CommandKind::App,
        label: "Activity: Open Edited Document",
        description: "Navigate to a document edited during the calendar block (read-only activity correlation).",
        keywords: &["activity", "document", "open", "edited", "calendar", "correlation", "pillar 2", "interop"],
        stable_id: "hs-activity-palette-open-document",
        disabled: false,
    },
    // ── WP-KERNEL-012 E11 remediation wave: the OPERATOR OPEN ROUTES for the previously orphaned side
    // panes (2026-07-02 drift audit). Each `view.*` id maps to a real `dispatch_palette_action` arm that
    // opens the named pane on the active work surface through the SAME `open_content_on_active_pane`
    // primitive every other open route uses. No fake commands: every id below has a mounted factory.
    AppCommand {
        id: CMD_VIEW_RELEVANT_MEMORY,
        kind: CommandKind::App,
        label: "View: Relevant Memory",
        description: "Open the FEMS Relevant Memory side pane on the active work surface.",
        keywords: &["view", "memory", "fems", "relevant", "pane", "pillar 12"],
        stable_id: "hs-view-palette-relevant-memory",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_STAGE,
        kind: CommandKind::App,
        label: "View: Stage",
        description: "Open the Stage pane (the route-to-Stage round-trip surface with the embed-back action).",
        keywords: &["view", "stage", "pane", "interop", "pillar 17"],
        stable_id: "hs-view-palette-stage",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_TAGS,
        kind: CommandKind::App,
        label: "View: Tags",
        description: "Open the Loom tags panel (tag list + tag hub pages).",
        keywords: &["view", "tags", "tag", "hub", "loom", "pane"],
        stable_id: "hs-view-palette-tags",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_SIDEBAR,
        kind: CommandKind::App,
        label: "View: Pins & Backlinks Sidebar",
        description: "Open the Loom sidebar (pins, favorites, backlinks, unlinked mentions, breadcrumbs).",
        keywords: &["view", "sidebar", "pins", "favorites", "backlinks", "unlinked", "loom", "pane"],
        stable_id: "hs-view-palette-sidebar",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_BLOCK_COLLECTIONS,
        kind: CommandKind::App,
        label: "View: Block Collections",
        description: "Open the block-collections view (table / kanban / calendar over Loom blocks).",
        keywords: &["view", "collections", "blocks", "table", "kanban", "calendar", "pane"],
        stable_id: "hs-view-palette-block-collections",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_OUTLINE,
        kind: CommandKind::App,
        label: "View: Outline",
        description: "Open the document outline (table of contents) beside the rich editor.",
        keywords: &["view", "outline", "toc", "headings", "document", "pane"],
        stable_id: "hs-view-palette-outline",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_GRAPH,
        kind: CommandKind::App,
        label: "View: Graph",
        description: "Open the knowledge graph view on the active work surface.",
        keywords: &["view", "graph", "knowledge", "backlinks", "loom", "pane"],
        stable_id: "hs-view-palette-graph",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_FOLDERS,
        kind: CommandKind::App,
        label: "View: Folders",
        description: "Open the Loom folder tree (lazy-loaded folders with color swatches).",
        keywords: &["view", "folders", "folder", "tree", "loom", "pane"],
        stable_id: "hs-view-palette-folders",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_OUTGOING_LINKS,
        kind: CommandKind::App,
        label: "View: Outgoing Links",
        description: "Open the outgoing-links side pane for the active document.",
        keywords: &["view", "outgoing", "links", "wikilinks", "pane"],
        stable_id: "hs-view-palette-outgoing-links",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_JOURNAL,
        kind: CommandKind::App,
        label: "View: Daily Journal",
        description: "Open the daily journal pane (calendar chip + today's note editor with auto-save).",
        keywords: &["view", "journal", "daily", "note", "today", "pane"],
        stable_id: "hs-view-palette-journal",
        disabled: false,
    },
    AppCommand {
        id: CMD_VIEW_DIFF_MERGE,
        kind: CommandKind::App,
        label: "View: Diff / Merge Editor",
        description: "Open the diff/merge editor pane (renders the currently opened diff or an honest empty state).",
        keywords: &["view", "diff", "merge", "conflict", "compare", "pane"],
        stable_id: "hs-view-palette-diff-merge",
        disabled: false,
    },
];

// ── WP-KERNEL-012 E11 remediation wave: stable `view.*` open-command ids ─────────────────────────────
pub const CMD_VIEW_RELEVANT_MEMORY: &str = "view.relevant-memory";
pub const CMD_VIEW_STAGE: &str = "view.stage";
pub const CMD_VIEW_TAGS: &str = "view.tags";
pub const CMD_VIEW_SIDEBAR: &str = "view.sidebar";
pub const CMD_VIEW_BLOCK_COLLECTIONS: &str = "view.block-collections";
pub const CMD_VIEW_OUTLINE: &str = "view.outline";
pub const CMD_VIEW_GRAPH: &str = "view.graph";
pub const CMD_VIEW_FOLDERS: &str = "view.folders";
pub const CMD_VIEW_OUTGOING_LINKS: &str = "view.outgoing-links";
pub const CMD_VIEW_JOURNAL: &str = "view.journal";
pub const CMD_VIEW_DIFF_MERGE: &str = "view.diff-merge";

// ── WP-KERNEL-012 MT-069 (E11 menu wire-up): the editor FILE/EDIT/GO menu + palette command ids ──────
//
// Stable command ids for the FILE/EDIT/GO editor actions WP-011 shipped as honestly-disabled placeholders
// and MT-079 host-mounted. These are the EXACT ids the MT-069 contract names so the menu bar, the command
// palette, and the `app.rs` dispatch all address the SAME action by one stable string. Save/Undo/Redo/etc.
// map to the EXISTING shell single-substrate handlers (the MT-031 InteractionBus command ids + the MT-020
// editor save path); the menu/palette only ROUTE by id, they never re-implement editor logic.

// FILE
pub const CMD_EDITOR_FILE_NEW: &str = "editor.file.new";
pub const CMD_EDITOR_FILE_SAVE: &str = "editor.file.save";
pub const CMD_EDITOR_FILE_SAVE_ALL: &str = "editor.file.saveAll";
pub const CMD_EDITOR_FILE_SAVE_AS: &str = "editor.file.saveAs";
pub const CMD_EDITOR_FILE_EXPORT_HTML: &str = "editor.file.export.html";
pub const CMD_EDITOR_FILE_EXPORT_MD: &str = "editor.file.export.md";
pub const CMD_EDITOR_FILE_EXPORT_TXT: &str = "editor.file.export.txt";
pub const CMD_EDITOR_FILE_EXPORT_JSON: &str = "editor.file.export.json";
// EDIT
pub const CMD_EDITOR_EDIT_UNDO: &str = "editor.edit.undo";
pub const CMD_EDITOR_EDIT_REDO: &str = "editor.edit.redo";
pub const CMD_EDITOR_EDIT_CUT: &str = "editor.edit.cut";
pub const CMD_EDITOR_EDIT_COPY: &str = "editor.edit.copy";
pub const CMD_EDITOR_EDIT_PASTE: &str = "editor.edit.paste";
pub const CMD_EDITOR_EDIT_SELECT_ALL: &str = "editor.edit.selectAll";
pub const CMD_EDITOR_FIND_FIND: &str = "editor.find.find";
pub const CMD_EDITOR_FIND_REPLACE: &str = "editor.find.replace";
pub const CMD_EDITOR_FIND_IN_FILES: &str = "editor.find.findInFiles";
pub const CMD_EDITOR_REPLACE_IN_FILES: &str = "editor.find.replaceInFiles";
pub const CMD_EDITOR_EDIT_TOGGLE_COMMENT: &str = "editor.edit.toggleComment";
pub const CMD_EDITOR_EDIT_FORMAT_DOCUMENT: &str = "editor.edit.formatDocument";
pub const CMD_WORKBENCH_SHOW_COMMANDS: &str = "workbench.action.showCommands";
pub const CMD_WORKBENCH_QUICK_OPEN: &str = "workbench.action.quickOpen";
// GO (code-navigation): MT-069 REMEDIATION — the code-nav shell commands are now REGISTERED against the
// MOUNTED code panel (`app.rs::dispatch_editor_command` routes each id to the panel's own
// `dispatch_action` entry — the SAME path the F12/Shift+F12/F8/Alt+Left keymap chords reach), so the GO
// menu items are LIVE (enabled whenever an editor pane is the focusable target).
pub const CMD_EDITOR_GO_TO_DEFINITION: &str = "editor.go.toDefinition";
pub const CMD_EDITOR_GO_TO_REFERENCES: &str = "editor.go.toReferences";
pub const CMD_EDITOR_GO_TO_SYMBOL: &str = "editor.go.toSymbol";
pub const CMD_EDITOR_GO_TO_LINE: &str = "editor.go.toLine";
// MT-069 REMEDIATION: the MT-052/MT-053 GO-menu editor-navigation leaves, now dispatchable by stable
// command id against the mounted panel (F8 / Shift+F8 / Alt+Left / Alt+Right / Ctrl+Shift+O parity).
pub const CMD_EDITOR_GO_NEXT_DIAGNOSTIC: &str = "editor.go.nextDiagnostic";
pub const CMD_EDITOR_GO_PREV_DIAGNOSTIC: &str = "editor.go.prevDiagnostic";
pub const CMD_EDITOR_GO_BACK: &str = "editor.go.back";
pub const CMD_EDITOR_GO_FORWARD: &str = "editor.go.forward";
pub const CMD_EDITOR_GO_SYMBOL_IN_FILE: &str = "editor.go.symbolInFile";

/// GO-menu code-navigation command ids whose owning command is NOT yet registered on the shell command
/// bus. MT-069 REMEDIATION: EMPTY — the code-nav commands are now registered against the mounted code
/// panel, so no GO id is pending. The list + predicate remain so a future genuinely-unowned id has a
/// typed logged no-op path (never `todo!()`/`unimplemented!()`/`panic!()` — AC-003).
pub const EDITOR_GO_NAV_PENDING_IDS: &[&str] = &[];

/// True when `id` is a GO-menu code-navigation command whose owner has not yet registered the live
/// handler — the dispatcher logs a typed "command not yet available" no-op for these rather than panicking.
pub fn is_go_nav_pending(id: &str) -> bool {
    EDITOR_GO_NAV_PENDING_IDS.contains(&id)
}

/// The editor FILE/EDIT menu + palette commands MT-069 makes LIVE (MT-079 host-mounted the panes). Each
/// dispatches through the EXISTING shell single-substrate path by its stable id (`app.rs` routes them to
/// the MT-031 InteractionBus / MT-020 save path); none re-implements editor logic. Enabled at the catalog
/// level (`disabled: false`); the per-frame ENABLE PREDICATE (an editor pane is the focusable target) is
/// applied by the menu bar + palette via [`editor_menu_commands_enabled`] so a stale-state row is honest.
/// The GO-nav rows are NOT in this catalog: they stay disabled placeholders in the GO menu (AC-003).
const EDITOR_MENU_COMMANDS: &[AppCommand] = &[
    editor_menu_cmd(
        CMD_EDITOR_FILE_NEW,
        "Editor: New Document",
        &["new", "document", "file", "editor"],
        "hs-editor-menu-file-new",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FILE_SAVE,
        "Editor: Save",
        &["save", "file", "document", "editor"],
        "hs-editor-menu-file-save",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FILE_SAVE_ALL,
        "Editor: Save All",
        &["save", "all", "file", "documents", "editor"],
        "hs-editor-menu-file-save-all",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FILE_SAVE_AS,
        "Editor: Save As",
        &["save", "as", "file", "export", "editor"],
        "hs-editor-menu-file-save-as",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FILE_EXPORT_HTML,
        "Editor: Export Document (HTML)",
        &["export", "html", "document", "editor"],
        "hs-editor-menu-file-export-html",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FILE_EXPORT_MD,
        "Editor: Export Document (Markdown)",
        &["export", "markdown", "md", "document", "editor"],
        "hs-editor-menu-file-export-md",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FILE_EXPORT_TXT,
        "Editor: Export Document (Text)",
        &["export", "text", "txt", "document", "editor"],
        "hs-editor-menu-file-export-txt",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FILE_EXPORT_JSON,
        "Editor: Export Document (JSON)",
        &["export", "json", "document", "editor"],
        "hs-editor-menu-file-export-json",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_UNDO,
        "Editor: Undo",
        &["undo", "revert", "edit", "editor"],
        "hs-editor-menu-edit-undo",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_REDO,
        "Editor: Redo",
        &["redo", "edit", "editor"],
        "hs-editor-menu-edit-redo",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_CUT,
        "Editor: Cut",
        &["cut", "clipboard", "edit", "editor"],
        "hs-editor-menu-edit-cut",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_COPY,
        "Editor: Copy",
        &["copy", "clipboard", "edit", "editor"],
        "hs-editor-menu-edit-copy",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_PASTE,
        "Editor: Paste",
        &["paste", "clipboard", "edit", "editor"],
        "hs-editor-menu-edit-paste",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_SELECT_ALL,
        "Editor: Select All",
        &["select", "all", "edit", "editor"],
        "hs-editor-menu-edit-select-all",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FIND_FIND,
        "Editor: Find",
        &["find", "search", "edit", "editor"],
        "hs-editor-menu-find-find",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FIND_REPLACE,
        "Editor: Replace",
        &["replace", "find", "search", "edit", "editor"],
        "hs-editor-menu-find-replace",
    ),
    editor_menu_cmd(
        CMD_EDITOR_FIND_IN_FILES,
        "Editor: Find in Files",
        &["find", "files", "workspace", "search", "editor"],
        "hs-editor-menu-find-in-files",
    ),
    editor_menu_cmd(
        CMD_EDITOR_REPLACE_IN_FILES,
        "Editor: Replace in Files",
        &["replace", "files", "workspace", "search", "editor"],
        "hs-editor-menu-replace-in-files",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_TOGGLE_COMMENT,
        "Editor: Toggle Comment",
        &["comment", "toggle", "line", "edit", "editor"],
        "hs-editor-menu-edit-toggle-comment",
    ),
    editor_menu_cmd(
        CMD_EDITOR_EDIT_FORMAT_DOCUMENT,
        "Editor: Format Document",
        &["format", "document", "edit", "editor"],
        "hs-editor-menu-edit-format-document",
    ),
    editor_menu_cmd(
        CMD_WORKBENCH_SHOW_COMMANDS,
        "Show All Commands",
        &["command", "palette", "commands", "workbench"],
        "hs-editor-menu-show-commands",
    ),
    editor_menu_cmd(
        CMD_WORKBENCH_QUICK_OPEN,
        "Go to File (Quick Open)",
        &["quick", "open", "switcher", "file", "workbench"],
        "hs-editor-menu-quick-open",
    ),
    // MT-069 REMEDIATION: the GO code-navigation shell commands, registered against the mounted panel.
    editor_menu_cmd(
        CMD_EDITOR_GO_TO_DEFINITION,
        "Editor: Go to Definition",
        &["go", "definition", "navigate", "editor"],
        "hs-editor-menu-go-to-definition",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_TO_REFERENCES,
        "Editor: Go to References",
        &["go", "references", "navigate", "editor"],
        "hs-editor-menu-go-to-references",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_TO_SYMBOL,
        "Editor: Go to Symbol in Workspace",
        &["go", "symbol", "workspace", "navigate", "editor"],
        "hs-editor-menu-go-to-symbol",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_TO_LINE,
        "Editor: Go to Line",
        &["go", "line", "navigate", "editor"],
        "hs-editor-menu-go-to-line",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_NEXT_DIAGNOSTIC,
        "Editor: Go to Next Problem",
        &["go", "next", "problem", "diagnostic", "error", "editor"],
        "hs-editor-menu-go-next-diagnostic",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_PREV_DIAGNOSTIC,
        "Editor: Go to Previous Problem",
        &["go", "previous", "problem", "diagnostic", "error", "editor"],
        "hs-editor-menu-go-prev-diagnostic",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_BACK,
        "Editor: Navigate Back",
        &["go", "back", "navigate", "history", "editor"],
        "hs-editor-menu-go-back",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_FORWARD,
        "Editor: Navigate Forward",
        &["go", "forward", "navigate", "history", "editor"],
        "hs-editor-menu-go-forward",
    ),
    editor_menu_cmd(
        CMD_EDITOR_GO_SYMBOL_IN_FILE,
        "Editor: Go to Symbol in File",
        &["go", "symbol", "file", "outline", "navigate", "editor"],
        "hs-editor-menu-go-symbol-in-file",
    ),
];

/// Const helper building one ENABLED `CommandKind::EditorMenu` entry (MT-069). `disabled: false` at the
/// catalog level; the per-frame enable predicate is applied by the menu/palette so a stale row is honest.
const fn editor_menu_cmd(
    id: &'static str,
    label: &'static str,
    keywords: &'static [&'static str],
    stable_id: &'static str,
) -> AppCommand {
    AppCommand {
        id,
        kind: CommandKind::EditorMenu,
        label,
        description: "Editor menu command (dispatches through the shell command bus).",
        keywords,
        stable_id,
        disabled: false,
    }
}

/// A representative subset (13 entries) of the React `EDITOR_COMMANDS` catalog
/// (`app/src/lib/editor/editor_commands.ts`), ported as `CommandKind::Editor` rows. The id/label/keywords
/// are ported verbatim; the native shell has no active rich-text DOCUMENT pinned yet, so all are
/// `disabled: true` (the React registry sets `disabled: !editorCommandsEnabled` the same way). A follow-up
/// MT can flip `disabled` when a rich-text document is the active edit target.
const EDITOR_COMMANDS: &[AppCommand] = &[
    editor_cmd(
        "editor.format.bold",
        "Bold",
        &["bold", "strong", "format"],
        "hs-editor-command-format-bold",
    ),
    editor_cmd(
        "editor.format.italic",
        "Italic",
        &["italic", "emphasis", "format"],
        "hs-editor-command-format-italic",
    ),
    editor_cmd(
        "editor.format.code",
        "Inline code",
        &["code", "monospace", "inline", "format"],
        "hs-editor-command-format-code",
    ),
    editor_cmd(
        "editor.block.h1",
        "Heading 1",
        &["heading", "h1", "title", "block"],
        "hs-editor-command-block-h1",
    ),
    editor_cmd(
        "editor.block.h2",
        "Heading 2",
        &["heading", "h2", "block"],
        "hs-editor-command-block-h2",
    ),
    editor_cmd(
        "editor.block.h3",
        "Heading 3",
        &["heading", "h3", "block"],
        "hs-editor-command-block-h3",
    ),
    editor_cmd(
        "editor.block.quote",
        "Block quote",
        &["quote", "blockquote", "callout", "block"],
        "hs-editor-command-block-quote",
    ),
    editor_cmd(
        "editor.list.bullet",
        "Bullet list",
        &["bullet", "unordered", "list"],
        "hs-editor-command-list-bullet",
    ),
    editor_cmd(
        "editor.list.ordered",
        "Numbered list",
        &["numbered", "ordered", "list"],
        "hs-editor-command-list-ordered",
    ),
    editor_cmd(
        "editor.list.task",
        "Task list",
        &["task", "todo", "checkbox", "checklist", "list"],
        "hs-editor-command-list-task",
    ),
    editor_cmd(
        "editor.code.insert",
        "Insert code block",
        &["code", "monaco", "snippet", "fence", "block"],
        "hs-editor-command-code-insert",
    ),
    editor_cmd(
        "editor.table.insert",
        "Insert table",
        &["table", "grid"],
        "hs-editor-command-table-insert",
    ),
    editor_cmd(
        "editor.link.wikilink",
        "Insert link",
        &["link", "wikilink", "note", "reference"],
        "hs-editor-command-link-wikilink",
    ),
];

/// Const helper building one disabled `CommandKind::Editor` entry with the React-aligned description
/// ("Editor command."), keeping the editor table compact and consistent.
const fn editor_cmd(
    id: &'static str,
    label: &'static str,
    keywords: &'static [&'static str],
    stable_id: &'static str,
) -> AppCommand {
    AppCommand {
        id,
        kind: CommandKind::Editor,
        label,
        description: "Editor command (needs the editor surface — future MT).",
        keywords,
        stable_id,
        disabled: true,
    }
}

/// Port of the React `matchesQuery` (`CommandPalette.tsx`): join `label + description + keywords`,
/// lowercase, and substring-match the trimmed lowercased query. An empty (whitespace-only) query
/// matches everything — exactly the React behavior (`if (q.length === 0) return true`).
pub fn matches_query(cmd: &AppCommand, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    let mut haystack = String::new();
    haystack.push_str(cmd.label);
    haystack.push(' ');
    haystack.push_str(cmd.description);
    for kw in cmd.keywords {
        haystack.push(' ');
        haystack.push_str(kw);
    }
    haystack.to_lowercase().contains(&q)
}

/// All commands that pass [`matches_query`] for `query`, in catalog order (the palette's row order).
/// Allocates a fresh `Vec<&AppCommand>` per call; the palette calls it once per frame against the
/// small static catalog (well under the red-team MC2 frame-time budget).
pub fn filtered_commands(query: &str) -> Vec<&'static AppCommand> {
    all_commands()
        .iter()
        .filter(|c| matches_query(c, query))
        .collect()
}

/// WP-KERNEL-012 MT-069: apply the per-frame ENABLE PREDICATE to a catalog command, returning the
/// effective disabled state the palette/menu should render this frame. A [`CommandKind::EditorMenu`]
/// command is enabled only when `editor_available` (an editor pane is the focusable/active target) — the
/// honest precondition the contract requires (no fake-enabled rows when no editor is mounted). All other
/// kinds keep their static `disabled` flag. Centralizing this keeps the palette and the menu bar reading
/// ONE predicate so they never diverge (RISK-006: stale enable state).
pub fn effective_disabled(cmd: &AppCommand, editor_available: bool) -> bool {
    match cmd.kind {
        CommandKind::EditorMenu => cmd.disabled || !editor_available,
        _ => cmd.disabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// PT2 / contract test 2: `all_commands()` has no duplicate ids and no duplicate stable_ids — a
    /// collision would make a command unaddressable out-of-process or ambiguous to dispatch.
    #[test]
    fn all_commands_have_no_duplicate_ids() {
        let cmds = all_commands();
        assert!(!cmds.is_empty(), "catalog is non-empty");

        let mut ids: HashSet<&str> = HashSet::new();
        let mut stable_ids: HashSet<&str> = HashSet::new();
        for cmd in cmds {
            assert!(ids.insert(cmd.id), "duplicate command id '{}'", cmd.id);
            assert!(
                stable_ids.insert(cmd.stable_id),
                "duplicate stable_id '{}'",
                cmd.stable_id
            );
        }
        assert_eq!(ids.len(), cmds.len(), "every id is unique");
        assert_eq!(stable_ids.len(), cmds.len(), "every stable_id is unique");
    }

    /// `all_commands()` returns the SAME static slice on repeated calls (OnceLock, no realloc per call).
    #[test]
    fn all_commands_returns_a_stable_static_slice() {
        let a = all_commands().as_ptr();
        let b = all_commands().as_ptr();
        assert_eq!(a, b, "all_commands() returns the same memoized slice");
    }

    /// PT1 / contract test 1: `matches_query` filters correctly — exact, partial, keyword, and no-match.
    #[test]
    fn matches_query_filters_correctly() {
        let cmds = all_commands();
        let usermanual_open = cmds.iter().find(|c| c.id == "usermanual.open").unwrap();

        // Exact label substring (case-insensitive).
        assert!(matches_query(usermanual_open, "UserManual: Open"));
        // Partial label match.
        assert!(matches_query(usermanual_open, "manual"));
        // Keyword match (not in the label/description).
        assert!(matches_query(usermanual_open, "diagnostics"));
        // Case-insensitive + trimmed.
        assert!(matches_query(usermanual_open, "  HELP  "));
        // Empty query matches everything.
        assert!(matches_query(usermanual_open, ""));
        assert!(matches_query(usermanual_open, "   "));
        // No match.
        assert!(!matches_query(usermanual_open, "zzz-no-such-command"));
    }

    /// Typing "manual" surfaces both UserManual commands and excludes unrelated ones (AC3 logic).
    #[test]
    fn filtering_manual_returns_usermanual_commands() {
        let results = filtered_commands("manual");
        let ids: Vec<&str> = results.iter().map(|c| c.id).collect();
        assert!(
            ids.contains(&"usermanual.open"),
            "usermanual.open in 'manual' results: {ids:?}"
        );
        assert!(
            ids.contains(&"usermanual.search"),
            "usermanual.search in 'manual' results: {ids:?}"
        );
        // A command with no 'manual' token is excluded.
        assert!(
            !ids.contains(&"theme.toggle"),
            "theme.toggle excluded from 'manual' results: {ids:?}"
        );
    }

    /// App commands are runnable; rich-text Editor commands are disabled until a rich document is the
    /// active target (no fake-enable); EditorMenu commands (MT-069) are catalog-enabled and gated per
    /// frame by the editor-available predicate.
    #[test]
    fn app_commands_enabled_editor_commands_disabled() {
        for cmd in all_commands() {
            match cmd.kind {
                CommandKind::App => assert!(!cmd.disabled, "App command '{}' is enabled", cmd.id),
                CommandKind::Editor => {
                    assert!(
                        cmd.disabled,
                        "Editor command '{}' is disabled (no editor doc yet)",
                        cmd.id
                    );
                    assert!(
                        cmd.id.starts_with("editor."),
                        "editor id prefix on '{}'",
                        cmd.id
                    );
                }
                CommandKind::EditorMenu => {
                    // Catalog-enabled (the static flag is false); the live predicate gates it.
                    assert!(
                        !cmd.disabled,
                        "EditorMenu command '{}' is catalog-enabled",
                        cmd.id
                    );
                }
            }
        }
    }

    /// MT-069 enable predicate: an EditorMenu command is disabled when no editor pane is available and
    /// enabled when one is; non-editor-menu commands ignore the predicate (keep their static flag).
    #[test]
    fn editor_menu_commands_gated_by_editor_available() {
        let save = all_commands()
            .iter()
            .find(|c| c.id == CMD_EDITOR_FILE_SAVE)
            .unwrap();
        assert!(
            effective_disabled(save, false),
            "Editor Save disabled when no editor pane is available"
        );
        assert!(
            !effective_disabled(save, true),
            "Editor Save enabled when an editor pane is available"
        );
        // A disabled rich-text Editor command stays disabled regardless of editor availability.
        let bold = all_commands()
            .iter()
            .find(|c| c.id == "editor.format.bold")
            .unwrap();
        assert!(
            effective_disabled(bold, true),
            "rich-text Bold stays disabled (needs an active doc)"
        );
        // An App command is never gated by the editor predicate.
        let theme = all_commands()
            .iter()
            .find(|c| c.id == "theme.toggle")
            .unwrap();
        assert!(
            !effective_disabled(theme, false),
            "App command ignores the editor predicate"
        );
    }

    /// The 31 MT-069 menu/palette editor command ids are present, enabled at the catalog level, and use
    /// the EXACT ids the contract names. MT-069 REMEDIATION: the GO code-nav commands are now REGISTERED
    /// (live palette/menu commands against the mounted panel) and the pending list is EMPTY.
    #[test]
    fn editor_menu_command_ids_match_contract() {
        let menu_ids: Vec<&str> = all_commands()
            .iter()
            .filter(|c| c.kind == CommandKind::EditorMenu)
            .map(|c| c.id)
            .collect();
        for expected in [
            CMD_EDITOR_FILE_NEW,
            CMD_EDITOR_FILE_SAVE,
            CMD_EDITOR_FILE_SAVE_ALL,
            CMD_EDITOR_FILE_SAVE_AS,
            CMD_EDITOR_FILE_EXPORT_HTML,
            CMD_EDITOR_FILE_EXPORT_MD,
            CMD_EDITOR_FILE_EXPORT_TXT,
            CMD_EDITOR_FILE_EXPORT_JSON,
            CMD_EDITOR_EDIT_UNDO,
            CMD_EDITOR_EDIT_REDO,
            CMD_EDITOR_EDIT_CUT,
            CMD_EDITOR_EDIT_COPY,
            CMD_EDITOR_EDIT_PASTE,
            CMD_EDITOR_EDIT_SELECT_ALL,
            CMD_EDITOR_FIND_FIND,
            CMD_EDITOR_FIND_REPLACE,
            CMD_EDITOR_FIND_IN_FILES,
            CMD_EDITOR_REPLACE_IN_FILES,
            CMD_EDITOR_EDIT_TOGGLE_COMMENT,
            CMD_EDITOR_EDIT_FORMAT_DOCUMENT,
            CMD_WORKBENCH_SHOW_COMMANDS,
            CMD_WORKBENCH_QUICK_OPEN,
            CMD_EDITOR_GO_TO_DEFINITION,
            CMD_EDITOR_GO_TO_REFERENCES,
            CMD_EDITOR_GO_TO_SYMBOL,
            CMD_EDITOR_GO_TO_LINE,
            CMD_EDITOR_GO_NEXT_DIAGNOSTIC,
            CMD_EDITOR_GO_PREV_DIAGNOSTIC,
            CMD_EDITOR_GO_BACK,
            CMD_EDITOR_GO_FORWARD,
            CMD_EDITOR_GO_SYMBOL_IN_FILE,
        ] {
            assert!(
                menu_ids.contains(&expected),
                "menu command id '{expected}' present: {menu_ids:?}"
            );
        }
        assert_eq!(menu_ids.len(), 31, "exactly 31 EditorMenu commands");
        // MT-069 REMEDIATION: no GO id is pending anymore (the code-nav commands are registered).
        assert!(
            EDITOR_GO_NAV_PENDING_IDS.is_empty(),
            "GO-nav pending list is empty (code-nav commands registered)"
        );
        assert!(
            !is_go_nav_pending(CMD_EDITOR_GO_TO_DEFINITION),
            "Go to Definition is live, not pending"
        );
    }
}
