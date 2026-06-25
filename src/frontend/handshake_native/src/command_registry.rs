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
    /// An editor-surface action ported from the React `EDITOR_COMMANDS` catalog. Rendered disabled
    /// until the native editor pane lands (a future MT); present so the action surface is discoverable.
    Editor,
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
];

/// A representative subset (13 entries) of the React `EDITOR_COMMANDS` catalog
/// (`app/src/lib/editor/editor_commands.ts`), ported as `CommandKind::Editor` rows. The id/label/keywords
/// are ported verbatim; the native shell has no editor surface yet, so all are `disabled: true` (the
/// React registry sets `disabled: !editorCommandsEnabled` the same way). A follow-up MT can extend this
/// to the full catalog and flip `disabled` when the editor pane is active.
const EDITOR_COMMANDS: &[AppCommand] = &[
    editor_cmd("editor.format.bold", "Bold", &["bold", "strong", "format"], "hs-editor-command-format-bold"),
    editor_cmd("editor.format.italic", "Italic", &["italic", "emphasis", "format"], "hs-editor-command-format-italic"),
    editor_cmd("editor.format.code", "Inline code", &["code", "monospace", "inline", "format"], "hs-editor-command-format-code"),
    editor_cmd("editor.block.h1", "Heading 1", &["heading", "h1", "title", "block"], "hs-editor-command-block-h1"),
    editor_cmd("editor.block.h2", "Heading 2", &["heading", "h2", "block"], "hs-editor-command-block-h2"),
    editor_cmd("editor.block.h3", "Heading 3", &["heading", "h3", "block"], "hs-editor-command-block-h3"),
    editor_cmd("editor.block.quote", "Block quote", &["quote", "blockquote", "callout", "block"], "hs-editor-command-block-quote"),
    editor_cmd("editor.list.bullet", "Bullet list", &["bullet", "unordered", "list"], "hs-editor-command-list-bullet"),
    editor_cmd("editor.list.ordered", "Numbered list", &["numbered", "ordered", "list"], "hs-editor-command-list-ordered"),
    editor_cmd("editor.list.task", "Task list", &["task", "todo", "checkbox", "checklist", "list"], "hs-editor-command-list-task"),
    editor_cmd("editor.code.insert", "Insert code block", &["code", "monaco", "snippet", "fence", "block"], "hs-editor-command-code-insert"),
    editor_cmd("editor.table.insert", "Insert table", &["table", "grid"], "hs-editor-command-table-insert"),
    editor_cmd("editor.link.wikilink", "Insert link", &["link", "wikilink", "note", "reference"], "hs-editor-command-link-wikilink"),
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
    all_commands().iter().filter(|c| matches_query(c, query)).collect()
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
        assert!(ids.contains(&"usermanual.open"), "usermanual.open in 'manual' results: {ids:?}");
        assert!(ids.contains(&"usermanual.search"), "usermanual.search in 'manual' results: {ids:?}");
        // A command with no 'manual' token is excluded.
        assert!(!ids.contains(&"theme.toggle"), "theme.toggle excluded from 'manual' results: {ids:?}");
    }

    /// App commands are runnable; editor commands are disabled until the editor pane lands (no fake-enable).
    #[test]
    fn app_commands_enabled_editor_commands_disabled() {
        for cmd in all_commands() {
            match cmd.kind {
                CommandKind::App => assert!(!cmd.disabled, "App command '{}' is enabled", cmd.id),
                CommandKind::Editor => {
                    assert!(cmd.disabled, "Editor command '{}' is disabled (no editor yet)", cmd.id);
                    assert!(cmd.id.starts_with("editor."), "editor id prefix on '{}'", cmd.id);
                }
            }
        }
    }
}
