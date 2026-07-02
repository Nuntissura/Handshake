//! Code Actions / Quick Fixes — the lightbulb affordance that turns a read-only diagnostic into an
//! actionable fix for the native code editor (WP-KERNEL-012 MT-049 — E1 VS Code parity).
//!
//! When the cursor rests on (or a selection overlaps) a diagnostic line, the editor asks the attached
//! language server for `textDocument/codeAction` over that range, draws a gutter lightbulb on the
//! affected line, and on click (or `Ctrl+.`) shows a popup menu of the available actions. Choosing an
//! action applies its `WorkspaceEdit` to the buffer by REUSING the MT-048 apply path
//! ([`super::rename::apply_text_edits_to_buffer`]) — it does NOT re-implement WorkspaceEdit application
//! (RISK-002 / MC-002 / AC-002). When no language server is attached (or the server returns no actions)
//! the feature degrades silently: empty action list, no lightbulb, no menu, no error (AC-006).
//!
//! ## Why reuse the MT-048 apply path, never re-implement (RISK-002 / MC-002)
//!
//! MT-048's [`super::rename::apply_text_edits_to_buffer`] already encodes the two data-integrity
//! invariants that mutate source files safely: DESCENDING-offset apply (so an earlier edit's byte
//! offsets are not invalidated by a later edit on the same file) and — through
//! [`super::rename::apply_preview`] — atomic temp+rename multi-file disk writes. A code action's
//! `WorkspaceEdit` is the SAME shape a rename's is, so [`CodeActionController::apply_selected`] converts
//! the chosen action's edit into MT-048's [`super::rename::WorkspaceEditPreview`] and delegates. A
//! parallel apply would mean inconsistent edits, broken undo grouping, off-by-one ranges, and silently
//! dropped cross-file fixes — exactly the regressions MT-048 was built to prevent. Because MT-048's apply
//! already handles MULTI-FILE edits, a cross-file code-action fix DOES apply (it is not silently dropped —
//! RISK-005 / MC-005).
//!
//! ## Stale-action data-integrity guard (RISK-007 / MC-007)
//!
//! Each action list is tagged with the buffer VERSION and the line it was requested for. If the buffer
//! changed between the request and the apply, [`CodeActionController::apply_selected`] REJECTS the apply
//! with [`CodeActionError::StaleBuffer`] (the caller re-requests) rather than applying ranges computed
//! against a now-mutated buffer — the same offset-corruption lesson MT-048 records. Applying a stale
//! range to a changed buffer would corrupt the document.
//!
//! ## Both LSP response shapes (RISK-003)
//!
//! The `textDocument/codeAction` response array mixes a bare `Command` and a full `CodeAction` (LSP
//! allows both). [`CodeActionItem::from_lsp`] normalizes BOTH: a bare `Command` becomes an item with
//! `command: Some(..)`, `edit: None` (applied via `workspace/executeCommand`, a graceful no-op if the
//! server cannot execute it — never silently dropped); a full `CodeAction` carries `edit`, `command`,
//! `is_preferred`, and `diagnostics`.

use std::sync::mpsc::Receiver;

use egui::accesskit;

use super::buffer::TextBuffer;
use super::cursor::CursorSet;
use super::rename::{apply_text_edits_to_buffer, WorkspaceEditPreview};

/// The stable AccessKit author_ids for the quick-fix surface, exactly as the MT-049 contract names them
/// so a swarm agent drives quick-fix without keystrokes (AC-004 / HBR-SWARM). The `_{i}`/`_{line}` suffix
/// forms are built by [`quickfix_item_author_id`] / [`quickfix_lightbulb_author_id`] so the indices flow
/// through one place (and the pane-id scope suffix is appended consistently — RISK-004 / MC-004).
pub const CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID: &str = "code_editor_quickfix_menu";
pub const CODE_EDITOR_QUICKFIX_ITEM_AUTHOR_PREFIX: &str = "code_editor_quickfix_item_";
pub const CODE_EDITOR_QUICKFIX_LIGHTBULB_AUTHOR_PREFIX: &str = "code_editor_quickfix_lightbulb_";
/// The editor-body context-menu 'Quick Fix...' entry author_id (AC-007 — the always-addressable swarm
/// node that routes to the same request+open_menu flow as Ctrl+. / the gutter lightbulb).
pub const CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID: &str = "code_editor_ctx_quick_fix";

/// The exact text the menu shows (and a swarm agent reads off the menu node) when a `Ctrl+.` trigger
/// produced no actions (AC-005 — the degraded path is observable, never a panic or a silently-empty
/// popup).
pub const NO_ACTIONS_TEXT: &str = "No quick fixes available";

/// Suffix an author_id with the pane instance so two open code-editor panes do not collide on the
/// quick-fix node ids (RISK-004 / MC-004 — the same scheme MT-008 overlays + MT-048 rename use).
pub fn scoped_author_id(base: &str, instance: &str) -> String {
    if instance.is_empty() {
        base.to_owned()
    } else {
        format!("{base}#{instance}")
    }
}

/// The author_id for the `i`-th menu item (`code_editor_quickfix_item_{i}`), pane-scoped.
pub fn quickfix_item_author_id(index: usize, instance: &str) -> String {
    scoped_author_id(
        &format!("{CODE_EDITOR_QUICKFIX_ITEM_AUTHOR_PREFIX}{index}"),
        instance,
    )
}

/// The author_id for the gutter lightbulb on `line` (`code_editor_quickfix_lightbulb_{line}`),
/// pane-scoped.
pub fn quickfix_lightbulb_author_id(line: usize, instance: &str) -> String {
    scoped_author_id(
        &format!("{CODE_EDITOR_QUICKFIX_LIGHTBULB_AUTHOR_PREFIX}{line}"),
        instance,
    )
}

/// Why applying a chosen code action could not complete. Distinct from a graceful no-op (an action with
/// no edit + no executable command is NOT an error; it is a silent no-op). These are the cases the caller
/// must surface or re-request, never silently swallow (the MT degradation contract).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeActionError {
    /// The selected index is out of range (no such action). The caller closes the menu.
    NoSuchAction,
    /// The buffer changed since the action list was requested (RISK-007 / MC-007). The action's ranges
    /// no longer match the document, so the apply is REJECTED and the caller re-requests rather than
    /// applying a stale range to a mutated buffer.
    StaleBuffer,
    /// An edit's line/character range did not resolve to a valid byte range in the current buffer (a
    /// malformed server edit). Returned instead of silently corrupting the text (the MT-048
    /// `EditApplyError` cause, surfaced here).
    BadRange,
}

impl std::fmt::Display for CodeActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodeActionError::NoSuchAction => write!(f, "no code action at the selected index"),
            CodeActionError::StaleBuffer => {
                write!(f, "the buffer changed since the code action was requested")
            }
            CodeActionError::BadRange => {
                write!(
                    f,
                    "a code action edit range did not resolve to a valid buffer range"
                )
            }
        }
    }
}

impl std::error::Error for CodeActionError {}

/// What [`CodeActionController::apply_selected`] did, so the caller surfaces a truthful result (and a
/// command-only action that needs `workspace/executeCommand` is routed, not dropped — RISK-003 / MC-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppliedAction {
    /// The action carried a `WorkspaceEdit`; it was applied to the active buffer (in-file edits) and the
    /// `cross_file` URIs were carried forward to the caller's multi-file apply path (RISK-005 / MC-005 —
    /// cross-file fixes apply via MT-048's multi-file path, never silently dropped). `in_file_edits` is
    /// the count applied to the active buffer.
    Edit {
        in_file_edits: usize,
        cross_file: WorkspaceEditPreview,
    },
    /// The action carried only a `command` (no edit). The caller must route it through
    /// `workspace/executeCommand`; the `command`/`arguments` are carried here. If the server cannot
    /// execute commands the caller no-ops gracefully (RISK-003 — never silently dropped).
    Command { command: LspCommand },
    /// The action carried neither an edit nor a command (a disabled/empty action): a graceful no-op.
    NoOp,
}

/// A normalized LSP `Command` (the bare-`Command` form of a code action, and the `command` field of a
/// full `CodeAction`). Kept transport-agnostic so the controller never depends on `lsp_types` at the
/// menu/apply boundary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LspCommand {
    /// The human title of the command (e.g. "Organize Imports").
    pub title: String,
    /// The command identifier the server's `workspace/executeCommand` handler dispatches on.
    pub command: String,
    /// The JSON arguments the command is invoked with (passed through verbatim).
    pub arguments: Vec<serde_json::Value>,
}

/// One code action, normalized from EITHER the bare `Command` or the full `CodeAction` LSP form
/// (RISK-003). The internal projection the menu renders + the apply path consumes; `CodeAction`/`Command`
/// from `lsp_types` are normalized into this so the rest of the editor never branches on the wire shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeActionItem {
    /// The action title shown in the popup menu (e.g. "Add missing semicolon").
    pub title: String,
    /// The LSP `CodeActionKind` (e.g. `quickfix`, `refactor`), or `None` for a bare command / a server
    /// that omitted it.
    pub kind: Option<String>,
    /// The `WorkspaceEdit` this action applies, or `None` for a command-only action (RISK-003 — a
    /// command-only action routes via `workspace/executeCommand`, never silently dropped).
    pub edit: Option<lsp_types::WorkspaceEdit>,
    /// The command this action executes (a bare `Command`, or the `command` field of a full
    /// `CodeAction`). Per LSP, when both `edit` and `command` are present the edit applies first, then
    /// the command.
    pub command: Option<LspCommand>,
    /// Whether the server marked this the preferred fix (the lightbulb's default / the menu's
    /// first-selected entry).
    pub is_preferred: bool,
}

impl CodeActionItem {
    /// Normalize a single `lsp_types::CodeActionOrCommand` into a [`CodeActionItem`] (RISK-003): a bare
    /// `Command` -> `command: Some(..)`, `edit: None`; a full `CodeAction` -> carries `edit`, `command`,
    /// `is_preferred`, `kind`. Neither form is ever dropped.
    pub fn from_lsp(item: lsp_types::CodeActionOrCommand) -> Self {
        match item {
            lsp_types::CodeActionOrCommand::Command(cmd) => Self {
                title: cmd.title.clone(),
                kind: None,
                edit: None,
                command: Some(LspCommand::from_lsp(cmd)),
                is_preferred: false,
            },
            lsp_types::CodeActionOrCommand::CodeAction(action) => Self {
                title: action.title,
                kind: action.kind.map(|k| k.as_str().to_owned()),
                edit: action.edit,
                command: action.command.map(LspCommand::from_lsp),
                is_preferred: action.is_preferred.unwrap_or(false),
            },
        }
    }

    /// Whether this action carries a `WorkspaceEdit` (drives the menu's apply branch).
    pub fn has_edit(&self) -> bool {
        self.edit.is_some()
    }
}

impl LspCommand {
    /// Normalize an `lsp_types::Command` into the transport-agnostic [`LspCommand`].
    fn from_lsp(cmd: lsp_types::Command) -> Self {
        Self {
            title: cmd.title,
            command: cmd.command,
            arguments: cmd.arguments.unwrap_or_default(),
        }
    }
}

/// Normalize a whole `textDocument/codeAction` response array into [`CodeActionItem`]s, preserving order
/// (the server orders preferred/relevant fixes first) (AC-001 / RISK-003). Both wire shapes are handled
/// by [`CodeActionItem::from_lsp`]; neither is dropped.
pub fn normalize_code_actions(
    response: Vec<lsp_types::CodeActionOrCommand>,
) -> Vec<CodeActionItem> {
    response.into_iter().map(CodeActionItem::from_lsp).collect()
}

/// The live state of the quick-fix surface for one code-editor pane: the line the actions were requested
/// for, the normalized actions, whether the menu is open, the selected index, and the in-flight flag. The
/// `buffer_version` is the STALE-ACTION guard's anchor (RISK-007 / MC-007): the apply path rejects if the
/// pane's live buffer version no longer matches.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CodeActionState {
    /// The 0-based buffer line the actions were requested for (drives the gutter lightbulb's line).
    pub line: usize,
    /// The buffer version the actions were requested AGAINST (RISK-007 / MC-007 — the stale-action guard
    /// rejects an apply if the live buffer version has moved past this).
    pub buffer_version: u64,
    /// The normalized actions the server returned for `line` (empty = no actions / no server — no
    /// lightbulb is drawn for an empty list).
    pub actions: Vec<CodeActionItem>,
    /// Whether the popup menu is currently open (Ctrl+. / lightbulb click / context-menu entry).
    pub menu_open: bool,
    /// The selected menu index (arrow keys move it; Enter applies it). Clamped to `actions.len()`.
    pub selected_index: usize,
    /// Whether a `textDocument/codeAction` request is in flight (the debounce + cancel-on-move guard reads
    /// it so a second request is not fired while one is pending — RISK-001 / MC-001).
    pub request_in_flight: bool,
}

impl CodeActionState {
    /// True when there is at least one action available on `line` (drives the lightbulb visibility).
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// The currently-selected action, or `None` when the list is empty / the index is out of range.
    pub fn selected(&self) -> Option<&CodeActionItem> {
        self.actions.get(self.selected_index)
    }

    /// Move the selection down by one (Down arrow), wrapping at the end. A no-op on an empty list.
    pub fn select_next(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.actions.len();
    }

    /// Move the selection up by one (Up arrow), wrapping at the start. A no-op on an empty list.
    pub fn select_prev(&mut self) {
        if self.actions.is_empty() {
            return;
        }
        self.selected_index = if self.selected_index == 0 {
            self.actions.len() - 1
        } else {
            self.selected_index - 1
        };
    }
}

/// The owner of the quick-fix request lifecycle, action-list state, lightbulb decision, and the apply
/// call for one code-editor pane. Holds the current [`CodeActionState`] and the inbound result channel a
/// spawned `textDocument/codeAction` task delivers `(line, buffer_version, actions)` over (the same
/// off-thread-then-drain pattern MT-008 completion / MT-047 signature help use).
#[derive(Debug, Default)]
pub struct CodeActionController {
    /// The current quick-fix state (`None` when idle — no request has fired and no menu is open).
    state: Option<CodeActionState>,
    /// The inbound result channel a spawned code-action task delivers over. `None` until the first
    /// request installs a receiver via [`CodeActionController::set_result_receiver`].
    result_rx: Option<Receiver<CodeActionResult>>,
}

/// The payload a spawned `textDocument/codeAction` task delivers over the controller's result channel:
/// the line + buffer version the request was anchored to (so a stale result for a line the cursor has
/// since left is dropped — RISK-001) and the normalized actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeActionResult {
    /// The 0-based line the actions are for.
    pub line: usize,
    /// The buffer version the request was issued against (the stale-action guard's anchor).
    pub buffer_version: u64,
    /// The normalized actions (empty = no actions / no server).
    pub actions: Vec<CodeActionItem>,
    /// Whether the menu should open immediately when this result lands (the `Ctrl+.` / context-menu /
    /// lightbulb-click path opens the menu; the passive cursor-rest path only lights the bulb).
    pub open_menu: bool,
}

impl CodeActionController {
    /// A fresh, idle controller.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install the inbound result receiver (the receive half of the channel the spawned request task
    /// sends over). The pane creates the channel, hands the sender to the off-thread task, and parks the
    /// receiver here so [`CodeActionController::poll_results`] drains it each frame.
    pub fn set_result_receiver(&mut self, rx: Receiver<CodeActionResult>) {
        self.result_rx = Some(rx);
    }

    /// A read-only snapshot of the current state (the deterministic observation point for tests).
    pub fn state(&self) -> Option<&CodeActionState> {
        self.state.as_ref()
    }

    /// Install a resolved action list for `line` at `buffer_version` (the deterministic path tests +
    /// the off-thread drain use). `open_menu` opens the popup immediately (the `Ctrl+.` / lightbulb /
    /// context-menu trigger) vs only lighting the gutter bulb (the passive cursor-rest trigger). An empty
    /// `actions` list clears the lightbulb but, when `open_menu` is set, leaves a menu-open state so the
    /// `Ctrl+.` degraded path can show "No quick fixes available" (AC-005 / AC-006).
    pub fn set_actions(
        &mut self,
        line: usize,
        buffer_version: u64,
        actions: Vec<CodeActionItem>,
        open_menu: bool,
    ) {
        let menu_open = open_menu;
        self.state = Some(CodeActionState {
            line,
            buffer_version,
            actions,
            menu_open,
            selected_index: 0,
            request_in_flight: false,
        });
    }

    /// Mark a request in flight for `line` at `buffer_version` (so the lightbulb line is known while the
    /// request resolves, and the debounce guard does not fire a second request — RISK-001 / MC-001). The
    /// actions stay empty until [`CodeActionController::poll_results`] delivers them.
    pub fn mark_request_in_flight(&mut self, line: usize, buffer_version: u64) {
        self.state = Some(CodeActionState {
            line,
            buffer_version,
            actions: Vec::new(),
            menu_open: false,
            selected_index: 0,
            request_in_flight: true,
        });
    }

    /// Whether a code-action request is currently in flight (the debounce guard reads it).
    pub fn request_in_flight(&self) -> bool {
        self.state
            .as_ref()
            .map(|s| s.request_in_flight)
            .unwrap_or(false)
    }

    /// The line a request is in flight / actions are loaded for, or `None` when idle.
    pub fn active_line(&self) -> Option<usize> {
        self.state.as_ref().map(|s| s.line)
    }

    /// Drain the inbound result channel into [`CodeActionState`] (called each frame). The MOST RECENT
    /// result wins (a server can answer twice as the cursor moves). A result whose `line` no longer
    /// matches a freshly-fired request is still installed — the caller's cursor-move guard decides
    /// relevance; here we just deliver what the server returned. Returns `true` when a result was drained.
    pub fn poll_results(&mut self) -> bool {
        let mut latest: Option<CodeActionResult> = None;
        if let Some(rx) = self.result_rx.as_ref() {
            while let Ok(result) = rx.try_recv() {
                latest = Some(result);
            }
        }
        match latest {
            Some(result) => {
                self.set_actions(
                    result.line,
                    result.buffer_version,
                    result.actions,
                    result.open_menu,
                );
                true
            }
            None => false,
        }
    }

    /// True when the line `line` carries at least one available code action (drives the lightbulb — AC-003).
    /// Only the EXACT requested line lights up; a line with no actions (or while idle) returns false.
    pub fn has_actions_on_line(&self, line: usize) -> bool {
        match &self.state {
            Some(s) => s.line == line && s.has_actions(),
            None => false,
        }
    }

    /// True when the popup menu is currently open.
    pub fn is_menu_open(&self) -> bool {
        self.state.as_ref().map(|s| s.menu_open).unwrap_or(false)
    }

    /// Open the popup menu for the current action list (the lightbulb click / `Ctrl+.` / context-menu
    /// entry). A no-op when there is no state at all (no request has fired). Opening an EMPTY action list
    /// is allowed (the degraded "No quick fixes available" path — AC-005).
    pub fn open_menu(&mut self) {
        if let Some(s) = self.state.as_mut() {
            s.menu_open = true;
            s.selected_index = 0;
        }
    }

    /// Close the popup menu (Escape / apply / focus loss). The action list + lightbulb stay until the
    /// cursor moves to a different line (so the bulb does not flicker while the menu is dismissed).
    pub fn close_menu(&mut self) {
        if let Some(s) = self.state.as_mut() {
            s.menu_open = false;
        }
    }

    /// Reset the controller to idle (cursor moved to a line with no diagnostic, file swap): clears the
    /// state so no lightbulb is drawn and no stale menu lingers.
    pub fn clear(&mut self) {
        self.state = None;
    }

    /// Move the menu selection down (Down arrow).
    pub fn select_next(&mut self) {
        if let Some(s) = self.state.as_mut() {
            s.select_next();
        }
    }

    /// Move the menu selection up (Up arrow).
    pub fn select_prev(&mut self) {
        if let Some(s) = self.state.as_mut() {
            s.select_prev();
        }
    }

    /// Set the menu selection to `index` (a mouse click on a row).
    pub fn select_index(&mut self, index: usize) {
        if let Some(s) = self.state.as_mut() {
            if index < s.actions.len() {
                s.selected_index = index;
            }
        }
    }

    /// The currently-selected action title, or `None`.
    pub fn selected_title(&self) -> Option<String> {
        self.state
            .as_ref()
            .and_then(|s| s.selected())
            .map(|a| a.title.clone())
    }

    /// Apply the SELECTED action to `buffer` + reconcile `cursors`, REUSING the MT-048 apply path
    /// ([`apply_text_edits_to_buffer`]) for the in-file edits — never re-implementing WorkspaceEdit
    /// application (RISK-002 / MC-002 / AC-002).
    ///
    /// `live_buffer_version` is the pane's CURRENT buffer version: if it no longer equals the version the
    /// action list was requested against, the apply is REJECTED with [`CodeActionError::StaleBuffer`]
    /// (RISK-007 / MC-007 — never apply stale ranges to a mutated buffer); the caller re-requests.
    ///
    /// `self_uri` is the active document's URI: the action's `WorkspaceEdit` edits targeting `self_uri`
    /// are applied to `buffer` here; edits targeting OTHER files are returned in the
    /// [`AppliedAction::Edit::cross_file`] preview so the caller routes them through MT-048's
    /// [`super::rename::apply_preview`] multi-file path (RISK-005 / MC-005 — cross-file fixes apply, not
    /// dropped). An action with only a `command` returns [`AppliedAction::Command`] so the caller runs
    /// `workspace/executeCommand` (graceful no-op if unsupported — RISK-003 / MC-003).
    pub fn apply_selected(
        &mut self,
        buffer: &mut TextBuffer,
        cursors: &mut CursorSet,
        self_uri: &str,
        live_buffer_version: u64,
    ) -> Result<AppliedAction, CodeActionError> {
        let state = self.state.as_ref().ok_or(CodeActionError::NoSuchAction)?;
        // RISK-007 / MC-007: reject if the buffer moved since the request — never apply a stale range.
        if state.buffer_version != live_buffer_version {
            return Err(CodeActionError::StaleBuffer);
        }
        let action = state
            .selected()
            .ok_or(CodeActionError::NoSuchAction)?
            .clone();

        // An action with a WorkspaceEdit: split it into THIS file's edits (applied here via MT-048) and
        // any cross-file edits (returned for the caller's multi-file apply — RISK-005 / MC-005).
        if let Some(edit) = action.edit.clone() {
            // Reuse the MT-048 preview builder to split the WorkspaceEdit (BOTH `changes` + `documentChanges`
            // shapes — RISK-003) into per-file edits, marking the active doc as the open buffer.
            let preview = WorkspaceEditPreview::from_lsp(&edit, |uri| {
                if uri == self_uri {
                    Some(buffer.to_string())
                } else {
                    None // a cross-file edit -> to-disk; the caller routes it via apply_preview.
                }
            });

            // Apply THIS file's edits to the live buffer via the MT-048 apply path (DESCENDING-offset —
            // RISK-001). Cross-file edits are carried forward unchanged.
            let mut in_file_edits = 0usize;
            let mut cross_file_files = Vec::new();
            for file in &preview.files {
                if file.uri == self_uri {
                    apply_text_edits_to_buffer(buffer, &file.edits)
                        .map_err(|_| CodeActionError::BadRange)?;
                    in_file_edits += file.edits.len();
                } else {
                    cross_file_files.push(file.clone());
                }
            }
            // Reconcile the primary caret into the (now-possibly-shorter) buffer so a shrink does not leave
            // a stale out-of-range cursor (set_primary clamps — the same reconcile set_text uses).
            let head = cursors.primary().min();
            cursors.set_primary(head.min(buffer.len_bytes()), buffer);

            self.close_menu();
            return Ok(AppliedAction::Edit {
                in_file_edits,
                cross_file: WorkspaceEditPreview {
                    files: cross_file_files,
                    is_single_file_fallback: false,
                },
            });
        }

        // No edit: a command-only action routes through workspace/executeCommand (RISK-003 / MC-003 — the
        // caller no-ops gracefully if the server cannot execute commands; never silently dropped).
        if let Some(command) = action.command.clone() {
            self.close_menu();
            return Ok(AppliedAction::Command { command });
        }

        // Neither edit nor command (a disabled/empty action): a graceful no-op.
        self.close_menu();
        Ok(AppliedAction::NoOp)
    }
}

// ── Rendering (egui + AccessKit) ─────────────────────────────────────────────────────────────────────

/// Fixed AccessKit/egui `NodeId`s for the quick-fix nodes (default single-instance pane). A fresh band
/// (720..) ABOVE the MT-048 rename band (710..715) and the MT-047 signature-help node (700), so the
/// quick-fix nodes never collide with another overlay node id. Multi-instance panes hash the
/// pane-scoped author_id instead (RISK-004 / MC-004), the same scheme every other editor overlay uses.
pub(crate) const QUICKFIX_MENU_NODE_ID: u64 = 720;
pub(crate) const QUICKFIX_LIGHTBULB_NODE_ID_BASE: u64 = 730;
pub(crate) const QUICKFIX_ITEM_NODE_ID_BASE: u64 = 760;

/// The maximum number of menu items + lightbulb nodes emitted per frame, so a pathological action list /
/// huge file cannot blow the per-frame AccessKit node budget (the RISK-004 cap the gutter markers use).
pub const MAX_QUICKFIX_NODES: usize = 32;

/// What the popup menu render wants the pane to do after a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    /// Nothing was chosen; keep showing the menu.
    None,
    /// Apply the action at this index (a click on a row or Enter on the selection).
    Apply(usize),
    /// Close the menu (Escape / click outside / the "No quick fixes" auto-dismiss).
    Close,
}

/// The theme-aware lightbulb color (RISK / CONTROL-4: no `Color32` literal in widget code). Uses the live
/// visuals' `warn_fg_color` (the amber the MT contract names), so the bulb tracks dark/light like every
/// other affordance.
pub fn lightbulb_color(visuals: &egui::Visuals) -> egui::Color32 {
    visuals.warn_fg_color
}

/// Render the popup menu for the current action list (a no-op when the menu is closed). Lists each
/// `CodeActionItem.title` as a selectable row; the selected row is highlighted. Returns the
/// [`MenuAction`] the operator chose this frame. When the action list is EMPTY (the `Ctrl+.` degraded
/// path) the menu shows [`NO_ACTIONS_TEXT`] and reports [`MenuAction::Close`] so the caller dismisses it
/// (AC-005). Emits the `Role::Menu` container node `code_editor_quickfix_menu` and a `Role::MenuItem`
/// node per item (AC-004), all pane-scoped (RISK-004 / MC-004).
pub fn render_menu(
    ctx: &egui::Context,
    state: &CodeActionState,
    anchor: egui::Pos2,
    instance: &str,
) -> MenuAction {
    if !state.menu_open {
        return MenuAction::None;
    }
    let mut action = MenuAction::None;
    let window_id = egui::Id::new(("code-editor-quickfix-menu", instance));

    egui::Area::new(window_id)
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(220.0);
                if state.actions.is_empty() {
                    // The degraded `Ctrl+.`-with-no-actions path (AC-005): show the message, then close.
                    ui.label(egui::RichText::new(NO_ACTIONS_TEXT).italics());
                    action = MenuAction::Close;
                } else {
                    for (i, item) in state.actions.iter().enumerate().take(MAX_QUICKFIX_NODES) {
                        let selected = i == state.selected_index;
                        let label = if item.is_preferred {
                            egui::RichText::new(&item.title).strong()
                        } else {
                            egui::RichText::new(&item.title)
                        };
                        if ui.selectable_label(selected, label).clicked() {
                            action = MenuAction::Apply(i);
                        }
                    }
                }
            });
        });

    // The keyboard verbs (Up/Down move selection; Enter applies; Escape closes) are read by the caller
    // from the frame events so they integrate with the pane's other key handling; this renderer only
    // emits the visual + AccessKit surface and the click outcome.

    // Emit the Menu container node + one MenuItem per row (AC-004 / HBR-SWARM).
    emit_menu_accesskit(ctx, state, instance);

    action
}

/// Emit the `Role::Menu` container AccessKit node (`code_editor_quickfix_menu`) + a `Role::MenuItem` node
/// per action (`code_editor_quickfix_item_{i}`), pane-scoped (RISK-004 / MC-004). The menu node's value
/// carries the action count (or [`NO_ACTIONS_TEXT`] for the empty degraded path) so a swarm agent reads
/// the menu state by id (AC-004 / AC-005). Only emitted while the menu is open.
fn emit_menu_accesskit(ctx: &egui::Context, state: &CodeActionState, instance: &str) {
    if !state.menu_open {
        return;
    }
    let menu_author = scoped_author_id(CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID, instance);
    let menu_node_id = quickfix_menu_node_id(instance);
    let count = state.actions.len();
    let menu_value = if count == 0 {
        NO_ACTIONS_TEXT.to_owned()
    } else {
        format!("{count} quick fixes")
    };
    ctx.accesskit_node_builder(menu_node_id, move |node| {
        node.set_role(accesskit::Role::Menu);
        node.set_author_id(menu_author.clone());
        node.set_label("Code editor quick-fix menu".to_owned());
        node.set_value(menu_value.clone());
    });

    for (i, item) in state.actions.iter().enumerate().take(MAX_QUICKFIX_NODES) {
        let item_author = quickfix_item_author_id(i, instance);
        let item_node_id = quickfix_item_node_id(i, instance);
        let title = item.title.clone();
        ctx.accesskit_node_builder(item_node_id, move |node| {
            node.set_role(accesskit::Role::MenuItem);
            node.set_author_id(item_author.clone());
            node.set_label("Quick fix".to_owned());
            node.set_value(title.clone());
            node.add_action(accesskit::Action::Click);
        });
    }
}

/// Draw the gutter lightbulb glyph on `line` at `pos` (theme-aware — RISK / CONTROL-4) and emit its
/// `Role::Button` AccessKit node `code_editor_quickfix_lightbulb_{line}` (pane-scoped — RISK-004 / MC-004),
/// returning the clickable [`egui::Response`] (the caller opens the menu on a click). Only called when
/// [`CodeActionController::has_actions_on_line`] is true for `line` (AC-003). The glyph is the amber
/// lightbulb `U+1F4A1`; if the active font lacks it, the painter still draws the colored mark so the
/// affordance is never an invisible no-op.
pub fn draw_lightbulb(
    ui: &mut egui::Ui,
    line: usize,
    pos: egui::Pos2,
    instance: &str,
) -> egui::Response {
    let color = lightbulb_color(ui.visuals());
    let glyph_size = 13.0;
    // A small click target centered on the glyph.
    let rect = egui::Rect::from_center_size(pos, egui::vec2(glyph_size + 2.0, glyph_size + 2.0));
    let id = ui
        .id()
        .with(("code-editor-quickfix-lightbulb", instance, line));
    let response = ui.interact(rect, id, egui::Sense::click());
    ui.painter().text(
        pos,
        egui::Align2::CENTER_CENTER,
        "\u{1F4A1}",
        egui::FontId::proportional(glyph_size),
        color,
    );

    // The Role::Button AccessKit node so a swarm agent opens the quick-fix menu by id (AC-004).
    let author = quickfix_lightbulb_author_id(line, instance);
    let node_id = quickfix_lightbulb_node_id(line, instance);
    let value = format!("Quick fixes available on line {}", line + 1);
    ui.ctx().accesskit_node_builder(node_id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label("Quick fix lightbulb".to_owned());
        node.set_value(value.clone());
        node.add_action(accesskit::Action::Click);
    });
    response
}

/// The fixed `egui::Id` for the quick-fix menu container node (default pane; instances hash the
/// pane-scoped author_id — RISK-004 / MC-004).
fn quickfix_menu_node_id(instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: a single hand-assigned fixed id (720) in the disjoint quick-fix band (above the rename
        // band's 710..715); never reused, so it cannot self-collide.
        unsafe { egui::Id::from_high_entropy_bits(QUICKFIX_MENU_NODE_ID) }
    } else {
        egui::Id::new(scoped_author_id(
            CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID,
            instance,
        ))
    }
}

/// The fixed `egui::Id` for menu item `i` (default pane; instances hash the pane-scoped author_id).
fn quickfix_item_node_id(index: usize, instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: each item slot maps to a distinct fixed id in the disjoint quick-fix item band
        // (760..760+MAX_QUICKFIX_NODES); never reused.
        unsafe { egui::Id::from_high_entropy_bits(QUICKFIX_ITEM_NODE_ID_BASE + index as u64) }
    } else {
        egui::Id::new(quickfix_item_author_id(index, instance))
    }
}

/// The fixed `egui::Id` for the lightbulb on `line` (default pane; instances hash the pane-scoped
/// author_id). The default-pane id is bucketed within the lightbulb band by `line % MAX_QUICKFIX_NODES`
/// so a huge file's high line numbers never escape the band (only one bulb is drawn at a time in practice,
/// but the bucket keeps the id inside the reserved 730..730+MAX_QUICKFIX_NODES range).
fn quickfix_lightbulb_node_id(line: usize, instance: &str) -> egui::Id {
    if instance.is_empty() {
        let slot = (line % MAX_QUICKFIX_NODES) as u64;
        // SAFETY: a fixed id in the disjoint lightbulb band (730..730+MAX_QUICKFIX_NODES); never reused.
        unsafe { egui::Id::from_high_entropy_bits(QUICKFIX_LIGHTBULB_NODE_ID_BASE + slot) }
    } else {
        egui::Id::new(quickfix_lightbulb_author_id(line, instance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare `Command` and a full `CodeAction` both normalize without dropping either (RISK-003).
    #[test]
    fn normalize_handles_both_command_and_code_action() {
        let command = lsp_types::CodeActionOrCommand::Command(lsp_types::Command {
            title: "Organize Imports".into(),
            command: "editor.organizeImports".into(),
            arguments: Some(vec![serde_json::json!({"uri": "file:///x.rs"})]),
        });
        let action = lsp_types::CodeActionOrCommand::CodeAction(lsp_types::CodeAction {
            title: "Add missing semicolon".into(),
            kind: Some(lsp_types::CodeActionKind::QUICKFIX),
            is_preferred: Some(true),
            ..Default::default()
        });
        let items = normalize_code_actions(vec![command, action]);
        assert_eq!(items.len(), 2, "neither form is dropped");
        // The bare Command -> command-only item.
        assert_eq!(items[0].title, "Organize Imports");
        assert!(items[0].edit.is_none(), "a bare Command has no edit");
        assert_eq!(
            items[0].command.as_ref().unwrap().command,
            "editor.organizeImports"
        );
        assert_eq!(items[0].command.as_ref().unwrap().arguments.len(), 1);
        // The full CodeAction -> kind + is_preferred preserved.
        assert_eq!(items[1].title, "Add missing semicolon");
        assert_eq!(items[1].kind.as_deref(), Some("quickfix"));
        assert!(items[1].is_preferred);
    }

    /// `has_actions_on_line` lights only the exact requested line with at least one action.
    #[test]
    fn has_actions_on_line_only_for_requested_line_with_actions() {
        let mut c = CodeActionController::new();
        // No state -> no lightbulb anywhere.
        assert!(!c.has_actions_on_line(0));
        // An action on line 4.
        let item = CodeActionItem {
            title: "Fix".into(),
            kind: None,
            edit: None,
            command: None,
            is_preferred: false,
        };
        c.set_actions(4, 1, vec![item], false);
        assert!(
            c.has_actions_on_line(4),
            "the requested line with an action lights up"
        );
        assert!(!c.has_actions_on_line(3), "a different line does not");
        assert!(!c.has_actions_on_line(5), "a different line does not");
        // An empty list on line 7 -> no lightbulb (AC-006 degradation shape).
        c.set_actions(7, 2, Vec::new(), false);
        assert!(
            !c.has_actions_on_line(7),
            "an empty action list draws no lightbulb"
        );
    }

    /// `apply_selected` REJECTS a stale buffer (RISK-007 / MC-007) instead of applying stale ranges.
    #[test]
    fn apply_selected_rejects_stale_buffer() {
        let mut c = CodeActionController::new();
        let edit = single_file_edit("file:///x.rs", 0, 3, 0, 6, "renamed");
        let item = CodeActionItem {
            title: "Rename token".into(),
            kind: Some("quickfix".into()),
            edit: Some(edit),
            command: None,
            is_preferred: true,
        };
        c.set_actions(0, /* requested at version */ 5, vec![item], true);
        let mut buffer = TextBuffer::new("let foo = 1;");
        let mut cursors = CursorSet::new();
        // The live buffer version (7) != the requested version (5) -> StaleBuffer, no mutation.
        let err = c
            .apply_selected(&mut buffer, &mut cursors, "file:///x.rs", 7)
            .expect_err("a stale buffer must be rejected");
        assert_eq!(err, CodeActionError::StaleBuffer);
        assert_eq!(
            buffer.to_string(),
            "let foo = 1;",
            "the buffer is untouched on a stale reject"
        );
    }

    /// `apply_selected` applies an in-file WorkspaceEdit via the MT-048 apply path (AC-002).
    #[test]
    fn apply_selected_applies_in_file_edit_via_mt048() {
        let mut c = CodeActionController::new();
        // Replace `foo` (col 4..7) with `bar` on line 0.
        let edit = single_file_edit("file:///x.rs", 0, 4, 0, 7, "bar");
        let item = CodeActionItem {
            title: "Replace foo with bar".into(),
            kind: Some("quickfix".into()),
            edit: Some(edit),
            command: None,
            is_preferred: true,
        };
        c.set_actions(0, 3, vec![item], true);
        let mut buffer = TextBuffer::new("let foo = 1;");
        let mut cursors = CursorSet::new();
        let applied = c
            .apply_selected(&mut buffer, &mut cursors, "file:///x.rs", 3)
            .expect("the in-file edit applies");
        match applied {
            AppliedAction::Edit {
                in_file_edits,
                cross_file,
            } => {
                assert_eq!(in_file_edits, 1);
                assert!(cross_file.files.is_empty(), "no cross-file edits");
            }
            other => panic!("expected an Edit apply, got {other:?}"),
        }
        assert_eq!(
            buffer.to_string(),
            "let bar = 1;",
            "AC-002: the buffer reflects the replacement"
        );
        assert!(!c.is_menu_open(), "the menu closes after apply");
    }

    /// A command-only action routes through workspace/executeCommand (RISK-003 / MC-003), not dropped.
    #[test]
    fn apply_selected_routes_command_only_action() {
        let mut c = CodeActionController::new();
        let item = CodeActionItem {
            title: "Organize Imports".into(),
            kind: None,
            edit: None,
            command: Some(LspCommand {
                title: "Organize Imports".into(),
                command: "editor.organizeImports".into(),
                arguments: vec![],
            }),
            is_preferred: false,
        };
        c.set_actions(0, 1, vec![item], true);
        let mut buffer = TextBuffer::new("use a;\nuse b;");
        let mut cursors = CursorSet::new();
        let applied = c
            .apply_selected(&mut buffer, &mut cursors, "file:///x.rs", 1)
            .expect("a command-only action routes to executeCommand");
        match applied {
            AppliedAction::Command { command } => {
                assert_eq!(command.command, "editor.organizeImports");
            }
            other => panic!("expected a Command route, got {other:?}"),
        }
    }

    /// A cross-file WorkspaceEdit applies the in-file part here and carries the cross-file part forward
    /// (RISK-005 / MC-005 — cross-file fixes are not silently dropped).
    #[test]
    fn apply_selected_carries_cross_file_edits_forward() {
        let mut c = CodeActionController::new();
        // An edit touching BOTH the active file (x.rs) and another file (y.rs).
        let mut changes = std::collections::HashMap::new();
        changes.insert(
            lsp_types::Url::parse("file:///x.rs").unwrap(),
            vec![lsp_types::TextEdit {
                range: lsp_range(0, 4, 0, 7),
                new_text: "bar".into(),
            }],
        );
        changes.insert(
            lsp_types::Url::parse("file:///y.rs").unwrap(),
            vec![lsp_types::TextEdit {
                range: lsp_range(0, 0, 0, 3),
                new_text: "baz".into(),
            }],
        );
        let edit = lsp_types::WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        };
        let item = CodeActionItem {
            title: "Cross-file fix".into(),
            kind: Some("quickfix".into()),
            edit: Some(edit),
            command: None,
            is_preferred: false,
        };
        c.set_actions(0, 1, vec![item], true);
        let mut buffer = TextBuffer::new("let foo = 1;");
        let mut cursors = CursorSet::new();
        let applied = c
            .apply_selected(&mut buffer, &mut cursors, "file:///x.rs", 1)
            .expect("the cross-file edit applies in-file + carries the rest forward");
        match applied {
            AppliedAction::Edit {
                in_file_edits,
                cross_file,
            } => {
                assert_eq!(in_file_edits, 1, "the active file's edit applied here");
                assert_eq!(
                    cross_file.files.len(),
                    1,
                    "the other file is carried forward, not dropped"
                );
                assert_eq!(cross_file.files[0].uri, "file:///y.rs");
                assert_eq!(cross_file.files[0].edits.len(), 1);
            }
            other => panic!("expected an Edit apply, got {other:?}"),
        }
        assert_eq!(
            buffer.to_string(),
            "let bar = 1;",
            "the active file is renamed in place"
        );
    }

    /// The selection wraps with Up/Down and the menu opens/closes.
    #[test]
    fn menu_selection_wraps_and_open_close() {
        let mut c = CodeActionController::new();
        let actions: Vec<CodeActionItem> = (0..3)
            .map(|i| CodeActionItem {
                title: format!("Fix {i}"),
                kind: None,
                edit: None,
                command: None,
                is_preferred: false,
            })
            .collect();
        c.set_actions(2, 1, actions, true);
        assert!(c.is_menu_open());
        assert_eq!(c.selected_title().as_deref(), Some("Fix 0"));
        c.select_next();
        assert_eq!(c.selected_title().as_deref(), Some("Fix 1"));
        c.select_prev();
        c.select_prev(); // wrap to the end.
        assert_eq!(c.selected_title().as_deref(), Some("Fix 2"));
        c.close_menu();
        assert!(!c.is_menu_open(), "close_menu closes the popup");
        assert!(
            c.has_actions_on_line(2),
            "the lightbulb stays lit after the menu closes"
        );
    }

    /// Author-id helpers are pane-scoped (RISK-004 / MC-004): two instances do not collide.
    #[test]
    fn author_ids_are_pane_scoped() {
        assert_eq!(
            quickfix_item_author_id(2, ""),
            "code_editor_quickfix_item_2"
        );
        assert_eq!(
            quickfix_item_author_id(2, "right"),
            "code_editor_quickfix_item_2#right"
        );
        assert_eq!(
            quickfix_lightbulb_author_id(5, "left"),
            "code_editor_quickfix_lightbulb_5#left"
        );
        assert_eq!(
            scoped_author_id(CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID, "p2"),
            "code_editor_quickfix_menu#p2"
        );
    }

    // ── test helpers ──

    fn lsp_range(sl: u32, sc: u32, el: u32, ec: u32) -> lsp_types::Range {
        lsp_types::Range {
            start: lsp_types::Position {
                line: sl,
                character: sc,
            },
            end: lsp_types::Position {
                line: el,
                character: ec,
            },
        }
    }

    fn single_file_edit(
        uri: &str,
        sl: u32,
        sc: u32,
        el: u32,
        ec: u32,
        new_text: &str,
    ) -> lsp_types::WorkspaceEdit {
        let mut changes = std::collections::HashMap::new();
        changes.insert(
            lsp_types::Url::parse(uri).unwrap(),
            vec![lsp_types::TextEdit {
                range: lsp_range(sl, sc, el, ec),
                new_text: new_text.into(),
            }],
        );
        lsp_types::WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }
    }
}
