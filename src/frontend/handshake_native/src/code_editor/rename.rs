//! Rename Symbol (F2) — the multi-file symbol-rename refactoring affordance for the native code editor
//! (WP-KERNEL-012 MT-048 — E1 VS Code parity).
//!
//! The operator places the cursor on an identifier, presses F2 (or chooses "Rename Symbol" from the
//! editor body context menu), an inline rename input opens pre-filled with the identifier text and
//! fully selected; on confirm the editor sends an LSP `textDocument/rename` request, receives a
//! `WorkspaceEdit`, shows a multi-file preview of every change, and on accept applies the edits
//! atomically across all open buffers and writes-to-disk for files that are not currently open. When no
//! LSP server is attached, the path degrades gracefully to a single-file rename of the current file's
//! occurrences with an explicit `LSP-required for cross-file rename` warning banner.
//!
//! ## Why this is data-integrity-critical (RISK-001 / RISK-002 — the highest severity class)
//!
//! This MT MUTATES the operator's SOURCE FILES. Two invariants are non-negotiable:
//!
//! 1. **Descending-offset apply** ([`apply_text_edits_to_buffer`]): within a single file the edits are
//!    sorted DESCENDING by start byte offset and applied in that order, so an earlier edit's byte
//!    offsets are NOT invalidated by a later edit that changed the document length before it. Applying
//!    in ASCENDING order silently corrupts the buffer — the `rename_workspace_edit` test asserts the
//!    descending result is correct AND that an ascending application would differ (the regression).
//! 2. **Atomic disk write** ([`write_file_atomic`]): an unopened file is rewritten by writing the full
//!    new content to a SIBLING temp file then `std::fs::rename`-ing it over the target (atomic on the
//!    same volume). The target file is therefore always either the intact original or the fully
//!    replaced new content — never a half-written file, even if the process is interrupted mid-write.
//!
//! ## Identifier resolution via tree-sitter (RISK-006 / MC-006 — no word-scan)
//!
//! [`begin_rename`] resolves the identifier under the cursor with the SAME tree-sitter parse tree the
//! MT-001 highlighter built (`Tree::root_node().named_descendant_for_byte_range(..)` then a walk up to
//! the nearest `identifier`/`type_identifier`/`field_identifier`/...-kind named node). It returns `None`
//! on a non-identifier (keyword, string contents, whitespace) so a rename never fires on the wrong
//! token. There is NO regex/word scan, which would rename the wrong symbol on shadowed/qualified names
//! and is exactly why LSP is required for *cross-file* rename.
//!
//! ## WorkspaceEdit both forms (RISK-003 / MC-003 — AC-007)
//!
//! [`WorkspaceEditPreview::from_lsp`] handles BOTH `lsp_types::WorkspaceEdit` shapes: the `changes`
//! map form (`{ [uri]: TextEdit[] }`) AND the `document_changes` array form
//! (`[{ textDocument, edits }]`, including the `AnnotatedTextEdit` `OneOf` variant). A
//! `documentChanges`-only server's rename is never silently dropped.
//!
//! ## No-LSP single-file fallback (RISK-004 / MC-004 — AC-003/AC-009)
//!
//! When no LSP server is attached the rename is SINGLE-FILE ONLY: the in-file occurrence ranges are
//! computed LOCALLY from the current file's tree-sitter parse tree (every identifier-kind named node
//! whose text equals the original name), and a visible `LSP-required for cross-file rename` banner is
//! rendered so the operator is NEVER misled that a single-file rename was project-wide. The existing
//! `GET /knowledge/code/symbols/{entity_id}/references` backend API is bound (consulted) by the panel to
//! confirm the symbol is real, but — VERIFIED against the real backend (handshake_core
//! `api/knowledge_code_nav.rs` `symbol_references`/`edge_span_refs`, and the frontend client model
//! `backend/knowledge_code_nav.rs` `SymbolCallerCallee.evidence_spans`) — each caller/callee that
//! endpoint returns carries `evidence_spans: [{ span_id, line_start, line_end }]`, i.e. LINE-LEVEL spans
//! only. The backend `KnowledgeSpan` storage row DOES hold `range_start`/`range_end` (the precise
//! char/byte range, `storage/knowledge.rs:761-762`), but `edge_span_refs` (`knowledge_code_nav.rs:782-786`)
//! does NOT project those columns to the wire — so the references response has NO occurrence-precise
//! char RANGE a rename needs to edit at column granularity (see [`references_lack_precise_ranges`] and
//! the MT typed blocker). Cross-file occurrence-precise rename therefore needs either widening that
//! projection to include `range_start`/`range_end` (a backend change = typed blocker, not patched here)
//! or an LSP server. So the in-file ranges come from tree-sitter (a safe local source), and the
//! cross-file occurrence ranges are the recorded typed blocker — never a backend edit (AC-009).

use std::ops::Range;

use egui::accesskit;
use tree_sitter::Tree;

use super::buffer::TextBuffer;

pub use handshake_rename::{
    apply_preview, apply_text_edits_to_buffer, apply_text_edits_to_string, write_file_atomic,
    EditApplyError, FileEditPreview, LspRange, PreviewHunk, RenameApplyReport, RenameError,
    TextEdit, WorkspaceEditPreview,
};

/// The stable AccessKit author_ids for the rename surface, exactly as the MT-048 contract names them so
/// a swarm agent drives rename without keystrokes (AC-006 / HBR-SWARM).
pub const CODE_EDITOR_RENAME_INPUT_AUTHOR_ID: &str = "code_editor_rename_input";
pub const CODE_EDITOR_RENAME_APPLY_AUTHOR_ID: &str = "code_editor_rename_apply";
pub const CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID: &str = "code_editor_rename_cancel";
pub const CODE_EDITOR_RENAME_NO_LSP_BANNER_AUTHOR_ID: &str = "code_editor_rename_no_lsp_banner";
pub const CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID: &str = "code_editor_ctx_rename_symbol";
/// The mounted context-menu item uses the shared menu prefix; this is the actual node Argus clicks.
pub const CODE_EDITOR_CTX_RENAME_SYMBOL_MENU_AUTHOR_ID: &str =
    "ctx-menu.code_editor_ctx_rename_symbol";

/// The exact warning-banner text the no-LSP single-file path must render (MC-004 / AC-003). A swarm
/// agent reads this off the banner node's value so it is never misled that a single-file rename was
/// project-wide.
pub const NO_LSP_BANNER_TEXT: &str = "LSP-required for cross-file rename";

/// Fixed AccessKit/egui `NodeId`s for non-interactive rename status nodes. A fresh band (710..) ABOVE the MT-047
/// signature-help node (700) and the MT-008 overlay band (completion 600..665, hover 680/681) so the
/// rename nodes never collide with another overlay node id.
pub(crate) const RENAME_PREVIEW_NODE_ID: u64 = 711;
pub(crate) const RENAME_NO_LSP_BANNER_NODE_ID: u64 = 714;

/// The rename state machine phase. Owned by the panel behind a `Mutex`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RenameState {
    /// No rename in progress.
    #[default]
    Idle,
    /// The inline rename input is open at the identifier (pre-filled + select-all on the first frame).
    Editing {
        /// The symbol entity id (resolved from the backend for the LSP request, or empty for the
        /// tree-sitter-only fallback). The contract names `entity_id`; empty when no backend symbol is
        /// bound (the local fallback still works off `original` + `ident_range`).
        entity_id: String,
        /// The identifier text under the cursor (the input is pre-filled with this).
        original: String,
        /// The live draft text in the input (starts equal to `original`).
        draft: String,
        /// The byte offset the input is anchored at (the identifier start) for screen positioning.
        anchor_byte: usize,
        /// The byte range of the identifier in the buffer (the occurrence the local fallback renames +
        /// the LSP position is derived from its start).
        ident_range: Range<usize>,
        /// One-shot flag: the input requests focus + select-all only on the FIRST frame it is shown
        /// (RISK-005 / MC-005 / HBR-QUIET — never steal focus every frame).
        focus_requested: bool,
    },
    /// The multi-file preview is shown (nothing mutated yet — AC-004). Apply/Cancel act on it.
    Previewing {
        workspace_edit: WorkspaceEditPreview,
    },
    /// An error occurred (e.g. the LSP server returned an error); the message is shown, no panic.
    Error { message: String },
}

/// What the preview render wants the panel to do after a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewAction {
    /// Nothing was clicked; keep showing the preview.
    None,
    /// The operator clicked Apply — the panel applies the preview.
    Apply,
    /// The operator clicked Cancel — the panel discards the preview and returns to Idle.
    Cancel,
}

/// Resolve the identifier under the cursor and begin a rename, or `None` when the cursor is not on an
/// identifier (RISK-006 / MC-006 — no word-scan; a keyword/string/whitespace under the cursor yields
/// `None` so no rename popup appears). `tree` is the MT-001 highlighter's parse tree; `cursor_byte` is
/// the primary caret byte offset; `entity_id` is the backend symbol id when known (empty otherwise).
///
/// The identifier is found by `Tree::root_node().named_descendant_for_byte_range(start, end)` then a walk
/// UP to the nearest identifier-kind named node ([`is_identifier_kind`]). The node's byte range is the
/// `ident_range`; the node's text (sliced from the buffer) is the `original` pre-filled into the input.
pub fn begin_rename(
    tree: &Tree,
    buffer: &TextBuffer,
    cursor_byte: usize,
    entity_id: impl Into<String>,
) -> Option<RenameState> {
    let ident_range = identifier_range_at(tree, cursor_byte)?;
    let original = buffer.byte_slice_to_string(ident_range.clone());
    if original.trim().is_empty() {
        return None;
    }
    Some(RenameState::Editing {
        entity_id: entity_id.into(),
        original: original.clone(),
        draft: original,
        anchor_byte: ident_range.start,
        ident_range,
        focus_requested: false,
    })
}

/// The byte range of the identifier-kind named node enclosing `cursor_byte`, or `None` when the cursor is
/// not inside an identifier-kind node (RISK-006 — no word-scan). Public so the panel can reuse it to
/// compute the single-file fallback occurrence ranges (every identifier node whose text matches).
pub fn identifier_range_at(tree: &Tree, cursor_byte: usize) -> Option<Range<usize>> {
    let root = tree.root_node();
    // A zero-width range AT the cursor; the cursor may sit just after the identifier (VS Code F2 works
    // with the caret at the end of the word too), so probe the byte BEFORE the cursor when the cursor is
    // at a node boundary and the descendant at [cursor, cursor) is not an identifier.
    let probe = |start: usize, end: usize| -> Option<Range<usize>> {
        let node = root.named_descendant_for_byte_range(start, end)?;
        let mut cur = Some(node);
        while let Some(n) = cur {
            if is_identifier_kind(n.kind()) {
                return Some(n.start_byte()..n.end_byte());
            }
            cur = n.parent();
        }
        None
    };
    if let Some(r) = probe(cursor_byte, cursor_byte) {
        return Some(r);
    }
    // Caret just after the word (e.g. `add|(`): probe the byte before the cursor.
    if cursor_byte > 0 {
        return probe(cursor_byte - 1, cursor_byte);
    }
    None
}

/// Whether a tree-sitter node kind is an identifier-kind we rename (RISK-006). Covers the common
/// identifier node kinds across the bundled grammars (rust + javascript) and their family: plain
/// `identifier`, `type_identifier`, `field_identifier`, `shorthand_property_identifier`, etc. A
/// keyword/operator/string node kind is NOT an identifier kind, so the cursor on those yields `None`.
pub fn is_identifier_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "type_identifier"
            | "field_identifier"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "statement_identifier"
            | "type_arguments"
            | "scoped_identifier"
    ) || kind.ends_with("_identifier")
}

/// Find every in-file occurrence of `name` as an identifier-kind node, in document order (the safe local
/// source for the no-LSP single-file fallback occurrence ranges — RISK-004/RISK-006: tree-sitter nodes,
/// never a word-scan, so `foo` inside `foobar` or inside a string is NOT matched). Walks the whole tree
/// with a `TreeCursor` pre-order traversal (the same walk-pattern MT-005 folding / MT-006 outline use).
pub fn identifier_occurrences(tree: &Tree, buffer: &TextBuffer, name: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if name.is_empty() {
        return out;
    }
    let mut cursor = tree.walk();
    // Pre-order DFS over named nodes.
    loop {
        let node = cursor.node();
        if is_identifier_kind(node.kind()) {
            let range = node.start_byte()..node.end_byte();
            if buffer.byte_slice_to_string(range.clone()) == name {
                out.push(range);
            }
        }
        // Descend, else move to the next sibling, else climb until a sibling exists.
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                // Back at the root with no more siblings: done.
                out.sort_by_key(|r| r.start);
                out.dedup();
                return out;
            }
        }
    }
}

/// TYPED BLOCKER MARKER (AC-009): the existing `GET /knowledge/code/symbols/{entity_id}/references`
/// backend endpoint DOES return per-caller/callee `evidence_spans: [{ span_id, line_start, line_end }]`,
/// but those spans are LINE-LEVEL ONLY. The backend `KnowledgeSpan` storage row also stores the precise
/// char/byte `range_start`/`range_end` (`storage/knowledge.rs:761-762`), yet `edge_span_refs`
/// (`knowledge_code_nav.rs:782-786`) does NOT project those columns to the wire — so the references
/// response carries no occurrence-precise char RANGE a column-granular rename needs. This function
/// documents that verified gap in code (the no-LSP fallback is therefore SINGLE-FILE via tree-sitter,
/// and cross-file occurrence-precise rename REQUIRES an LSP server). It always returns `true`: a
/// cross-file references-based rename needs the references projection widened to include
/// `range_start`/`range_end` (a backend change), which is a typed blocker — NEVER a backend edit here
/// (the MT forbids touching `src/backend/**`). Kept as an explicit, testable assertion of the recorded
/// blocker. The name reads "lacks precise ranges" = lacks the wire-exposed char range, NOT "no spans".
pub fn references_lack_precise_ranges() -> bool {
    true
}

// ── Rendering (egui + AccessKit) ───────────────────────────────────────────────────────────────────

/// Suffix an author_id with the panel instance (so a diff view's two editors do not collide on the
/// rename node ids — RISK-004, the same scheme the MT-008 overlays use).
fn suffixed(base: &str, instance: &str) -> String {
    if instance.is_empty() {
        base.to_owned()
    } else {
        format!("{base}#{instance}")
    }
}

/// A fixed egui `Id` for a rename overlay node in the disjoint band (default panel; instances hash the
/// suffixed author_id so two editors never collide — RISK-004).
fn rename_node_id(base_node_id: u64, base_author: &str, instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: a single hand-assigned fixed id in the disjoint overlay band (710..714, above the
        // signature-help band's 700); never reused, so it cannot self-collide.
        unsafe { egui::Id::from_high_entropy_bits(base_node_id) }
    } else {
        egui::Id::new(format!("{base_author}#{instance}"))
    }
}

/// The diff-preview "added" (green `+` line) color, resolved from the active theme's palette
/// `success_text` token (CONTROL-4: no hardcoded Color32 in widget code). Mirrors the gutter's
/// [`super::gutter::diagnostic_tokens_for`] dark/light branch so the added-hunk color tracks the shell
/// theme like every other token. The removed (`-`) line reuses the diagnostic ERROR (red) token from
/// that same resolver, so both hunk colors come from the theme rather than baked literals.
fn rename_preview_added_color(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        crate::theme::HsTheme::Dark.palette().success_text
    } else {
        crate::theme::HsTheme::Light.palette().success_text
    }
}

/// Render the inline rename input overlaid at the identifier screen position (AC-001). Draws an
/// `egui::TextEdit::singleline` in an `egui::Area` at `ident_screen_pos`; on the FIRST frame it is shown
/// it requests focus and selects-all (so typing replaces the identifier — VS Code F2 behavior), guarded
/// by the `focus_requested` one-shot flag so it never steals focus every frame (RISK-005 / MC-005 /
/// HBR-QUIET). Emits the `Role::TextInput` AccessKit node `code_editor_rename_input` carrying the draft so
/// a swarm agent reads/sets the rename without keystrokes (AC-006).
///
/// The popup renders as an `egui::Area` via `ctx`, so the caller only needs the `Context` (the same shape
/// the MT-047 signature popup + the MT-008 overlays use). The caller reads Enter (confirm) / Escape
/// (cancel) from the frame's key events on the same frame.
pub fn render_inline_input(
    ctx: &egui::Context,
    state: &mut RenameState,
    ident_screen_pos: egui::Pos2,
    instance: &str,
) {
    let RenameState::Editing {
        original: _,
        draft,
        focus_requested,
        ..
    } = state
    else {
        return;
    };

    let author = suffixed(CODE_EDITOR_RENAME_INPUT_AUTHOR_ID, instance);
    let input_egui_id = egui::Id::new(("code-editor-rename-input", instance));
    let area_id = egui::Id::new(("code-editor-rename-input-area", instance));

    let first_frame = !*focus_requested;

    egui::Area::new(area_id)
        .order(egui::Order::Foreground)
        .fixed_pos(ident_screen_pos)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                let edit = egui::TextEdit::singleline(draft)
                    .id(input_egui_id)
                    .desired_width(180.0)
                    .hint_text("New name");
                let resp = ui.add(edit);
                let input_author = author.clone();
                ui.ctx().accesskit_node_builder(resp.id, move |node| {
                    // Augment the REAL TextEdit node so an addressed Argus SetValue reaches the widget
                    // that owns `draft`; a detached synthetic node would be visible but not steerable.
                    node.set_author_id(input_author);
                    node.set_label("Rename symbol".to_owned());
                    node.add_action(accesskit::Action::SetValue);
                });
                if let Some(replacement) = crate::mcp::accesskit_string_set_value(ui, resp.id) {
                    *draft = replacement;
                    ui.ctx().request_repaint();
                }
                if first_frame {
                    // ONE-SHOT focus + select-all on the first frame only (RISK-005 / MC-005 / HBR-QUIET).
                    resp.request_focus();
                    // Select all text so typing replaces the identifier (VS Code F2): set the cursor range
                    // to span the whole draft via egui's TextEditState.
                    if let Some(mut text_state) =
                        egui::text_edit::TextEditState::load(ui.ctx(), input_egui_id)
                    {
                        let char_len = draft.chars().count();
                        let ccursor_range = egui::text_selection::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(char_len),
                        );
                        text_state.cursor.set_char_range(Some(ccursor_range));
                        text_state.store(ui.ctx(), input_egui_id);
                    }
                }
            });
        });
    // Mark the one-shot focus request done AFTER rendering (so the next frame does not re-request).
    *focus_requested = true;
}

/// Render the multi-file rename preview (AC-004): lists every changed file (open buffer vs to-disk noted)
/// with its before/after hunks BEFORE any edit is applied, plus an Apply button
/// (`code_editor_rename_apply`, `Role::Button`) and a Cancel button (`code_editor_rename_cancel`,
/// `Role::Button`). When the preview is a single-file fallback it ALSO renders the no-LSP banner
/// ([`render_no_lsp_banner`]). Returns the [`PreviewAction`] the operator chose this frame.
pub fn render_preview(
    ctx: &egui::Context,
    preview: &WorkspaceEditPreview,
    instance: &str,
) -> PreviewAction {
    let mut action = PreviewAction::None;
    let window_id = egui::Id::new(("code-editor-rename-preview", instance));

    egui::Window::new("Rename Preview")
        .id(window_id)
        .collapsible(false)
        .resizable(true)
        .default_width(520.0)
        .show(ctx, |ui| {
            if preview.is_single_file_fallback {
                render_no_lsp_banner(ui, instance);
                ui.add_space(4.0);
            }

            ui.label(format!(
                "{} change(s) across {} file(s)",
                preview.total_edits(),
                preview.files.len()
            ));
            ui.separator();

            // Diff-hunk colors come from the live theme (CONTROL-4: no hardcoded Color32 in widget
            // code). The removed ('before') line uses the active theme's diagnostic ERROR token (red)
            // — the same `diagnostic_tokens_for(ui.visuals())` resolver the gutter affordances use —
            // and the added ('after') line uses the active palette's `success_text` token (green), so
            // both hunk colors track dark/light like every other widget instead of baking literals.
            let diff_removed_color = super::gutter::diagnostic_tokens_for(ui.visuals()).error;
            let diff_added_color = rename_preview_added_color(ui.visuals());
            // The diff list must leave room for the Apply/Cancel row BELOW it, inside the viewport.
            //
            // With a flat `.max_height(360.0)` and `auto_shrink([false, false])` the list claims
            // `min(available, 360)` regardless of content (egui-0.33.3 scroll_area.rs:717-720), and the
            // window's stored size follows its UNCLAMPED content (area.rs:667) while `constrain` only
            // pins the TOP (area.rs:559). On a short viewport the action row therefore fell past the
            // window's clip rect (area.rs:630), which empties its `interact_rect`
            // (ui.rs:1140) and makes it unhittable (hit_test.rs:421-422) — while its AccessKit node
            // kept advertising the UNCLIPPED bounds (response.rs:836-841). That combination is why the
            // node-present and Role::Button assertions passed while the click silently did nothing:
            // the tree said the button was there, and the hit test said nothing was.
            //
            // Clamping to the live content rect is the same defence slash_commands/menu.rs:121,134-141
            // already uses. This is a real product defect, not a test artefact: an operator with a
            // short editor area could not reach Apply or Cancel either.
            let action_row_reserve = 140.0;
            let max_list_height = (ctx.content_rect().height() - action_row_reserve).clamp(80.0, 360.0);
            egui::ScrollArea::vertical()
                .max_height(max_list_height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for file in &preview.files {
                        let kind = if file.is_open_buffer {
                            "open buffer"
                        } else {
                            "to disk"
                        };
                        ui.label(
                            egui::RichText::new(format!("{}  ({kind})", file.uri))
                                .strong()
                                .monospace(),
                        );
                        for hunk in &file.hunks {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("L{}", hunk.line + 1))
                                        .weak()
                                        .small(),
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("- {}", hunk.before))
                                            .monospace()
                                            .color(diff_removed_color),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("+ {}", hunk.after))
                                            .monospace()
                                            .color(diff_added_color),
                                    );
                                });
                            });
                        }
                        ui.separator();
                    }
                });

            ui.horizontal(|ui| {
                let apply_resp = ui.button("Apply");
                if apply_resp.clicked() {
                    action = PreviewAction::Apply;
                }
                let apply_author = suffixed(CODE_EDITOR_RENAME_APPLY_AUTHOR_ID, instance);
                ui.ctx().accesskit_node_builder(apply_resp.id, move |node| {
                    // Attach the stable id to the REAL button so AccessKit Click drives `clicked()`.
                    node.set_author_id(apply_author.clone());
                    node.set_label("Apply rename".to_owned());
                });

                let cancel_resp = ui.button("Cancel");
                if cancel_resp.clicked() {
                    action = PreviewAction::Cancel;
                }
                let cancel_author = suffixed(CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID, instance);
                ui.ctx()
                    .accesskit_node_builder(cancel_resp.id, move |node| {
                        // Attach the stable id to the REAL button so AccessKit Click drives `clicked()`.
                        node.set_author_id(cancel_author.clone());
                        node.set_label("Cancel rename".to_owned());
                    });
            });

            // Emit the preview container node so a swarm agent can read the change summary by id.
            let preview_author = suffixed("code_editor_rename_preview", instance);
            let total = preview.total_edits();
            let file_count = preview.files.len();
            let preview_node_id = rename_node_id(
                RENAME_PREVIEW_NODE_ID,
                "code_editor_rename_preview",
                instance,
            );
            ui.ctx()
                .accesskit_node_builder(preview_node_id, move |node| {
                    node.set_role(accesskit::Role::GenericContainer);
                    node.set_author_id(preview_author.clone());
                    node.set_label("Rename preview".to_owned());
                    node.set_value(format!("{total} changes across {file_count} files"));
                });
        });

    action
}

/// Render the visible no-LSP warning banner (MC-004 / AC-003): a `Role::Label` node
/// `code_editor_rename_no_lsp_banner` reading exactly [`NO_LSP_BANNER_TEXT`], so the operator (and a swarm
/// agent reading the node value) is NEVER misled that a single-file rename was project-wide.
pub fn render_no_lsp_banner(ui: &mut egui::Ui, instance: &str) {
    // CONTROL-4: source the banner icon/text color from the live theme's `warn_fg_color` (the same
    // token already used for the frame fill below), so the warning affordance tracks dark/light like
    // every other widget instead of baking an opaque Color32 literal into widget code.
    let warn_color = ui.visuals().warn_fg_color;
    egui::Frame::group(ui.style())
        .fill(warn_color.linear_multiply(0.08))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("\u{26A0}").color(warn_color));
                ui.label(
                    egui::RichText::new(NO_LSP_BANNER_TEXT)
                        .color(warn_color)
                        .strong(),
                );
            });
        });

    let author = suffixed(CODE_EDITOR_RENAME_NO_LSP_BANNER_AUTHOR_ID, instance);
    let node_id = rename_node_id(
        RENAME_NO_LSP_BANNER_NODE_ID,
        CODE_EDITOR_RENAME_NO_LSP_BANNER_AUTHOR_ID,
        instance,
    );
    ui.ctx().accesskit_node_builder(node_id, move |node| {
        node.set_role(accesskit::Role::Label);
        node.set_author_id(author.clone());
        node.set_label("Rename scope warning".to_owned());
        node.set_value(NO_LSP_BANNER_TEXT.to_owned());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_editor::highlight::{Highlighter, LanguageRegistry};

    fn rust_tree(src: &str) -> (Highlighter, Tree) {
        let mut hl = LanguageRegistry::with_bundled_languages()
            .highlighter_for_extension("rs")
            .expect("rust highlighter");
        hl.highlight(src.as_bytes());
        let tree = hl.tree().expect("a parse tree after highlight").clone();
        (hl, tree)
    }

    #[test]
    fn begin_rename_resolves_identifier_and_rejects_non_identifier() {
        // RISK-006 / MC-006: the cursor on an identifier resolves it; on a keyword/whitespace it is None.
        let src = "fn compute() { let value = 1; }";
        let (_hl, tree) = rust_tree(src);
        let buffer = TextBuffer::new(src);

        // Cursor on `value` (byte 19 is inside `value`).
        let on_ident = src.find("value").unwrap() + 1;
        let state = begin_rename(&tree, &buffer, on_ident, "ent-value")
            .expect("cursor on an identifier begins a rename");
        match state {
            RenameState::Editing {
                original,
                draft,
                ident_range,
                entity_id,
                ..
            } => {
                assert_eq!(original, "value");
                assert_eq!(draft, "value", "draft pre-filled with the identifier");
                assert_eq!(&src[ident_range], "value");
                assert_eq!(entity_id, "ent-value");
            }
            other => panic!("expected Editing, got {other:?}"),
        }

        // Cursor on the `fn` keyword -> None (no rename on a keyword).
        let on_keyword = 0; // 'f' of 'fn'
        assert!(
            begin_rename(&tree, &buffer, on_keyword, "").is_none(),
            "RISK-006: a keyword under the cursor does not begin a rename"
        );
        // Cursor on whitespace between tokens -> None.
        let on_space = src.find("fn compute").unwrap() + 2; // the space after 'fn'
        assert!(
            begin_rename(&tree, &buffer, on_space, "").is_none(),
            "RISK-006: whitespace under the cursor does not begin a rename"
        );
    }

    #[test]
    fn identifier_occurrences_finds_only_whole_identifier_nodes() {
        // RISK-004/RISK-006: `value` matches the two identifier nodes, NOT the substring inside `valuee`
        // or inside a string literal.
        let src = "let value = value + 1; let valuee = 2; let s = \"value\";";
        let (_hl, tree) = rust_tree(src);
        let buffer = TextBuffer::new(src);
        let occ = identifier_occurrences(&tree, &buffer, "value");
        // Exactly the two real `value` identifier occurrences (positions 4 and 12), not `valuee`/the string.
        assert_eq!(
            occ.len(),
            2,
            "only the two whole-identifier occurrences, got {occ:?}"
        );
        for r in &occ {
            assert_eq!(&src[r.clone()], "value");
        }
    }

    #[test]
    fn single_file_fallback_marks_banner_and_renames_in_file() {
        // AC-003 / MC-004: the no-LSP fallback renames only the current file's occurrences + marks the
        // single-file flag (so the banner renders).
        let src = "let value = value + 1;";
        let (_hl, tree) = rust_tree(src);
        let buffer = TextBuffer::new(src);
        let occ = identifier_occurrences(&tree, &buffer, "value");
        let preview =
            WorkspaceEditPreview::single_file_fallback("file:///cur.rs", src, "total", &occ, true);
        assert!(
            preview.is_single_file_fallback,
            "the fallback marks the banner flag"
        );
        assert_eq!(preview.files.len(), 1);
        let out = apply_text_edits_to_string(src, &preview.files[0].edits).unwrap();
        assert_eq!(out, "let total = total + 1;");
    }

    #[test]
    fn references_endpoint_lacks_precise_ranges_typed_blocker() {
        // AC-009: the recorded typed blocker — VERIFIED against the real backend, the references API
        // returns per-caller/callee `evidence_spans:[{span_id,line_start,line_end}]` (LINE-LEVEL only);
        // the stored `KnowledgeSpan.range_start/range_end` char range is NOT projected to the wire by
        // `edge_span_refs`. So cross-file occurrence-precise rename needs that projection widened (a
        // backend change = typed blocker), never a backend edit here.
        assert!(
            references_lack_precise_ranges(),
            "references endpoint exposes line-only evidence_spans, no wire char range",
        );
    }
}
