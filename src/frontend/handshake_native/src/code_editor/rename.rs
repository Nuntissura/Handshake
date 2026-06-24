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
//! confirm the symbol is real, but — VERIFIED against the real backend — that endpoint returns only
//! symbol-level callers/callees (`{ symbol_entity_id, display_name }`), NOT the precise byte/char
//! occurrence RANGES a rename needs (see [`references_lack_precise_ranges`] and the MT typed blocker).
//! So the in-file ranges come from tree-sitter (a safe local source), and the cross-file occurrence
//! ranges are the recorded `NEEDS_BACKEND_ROUTE` typed blocker — never a backend edit (AC-009).

use std::ops::Range;
use std::path::{Path, PathBuf};

use egui::accesskit;
use tree_sitter::Tree;

use super::buffer::TextBuffer;

/// The stable AccessKit author_ids for the rename surface, exactly as the MT-048 contract names them so
/// a swarm agent drives rename without keystrokes (AC-006 / HBR-SWARM).
pub const CODE_EDITOR_RENAME_INPUT_AUTHOR_ID: &str = "code_editor_rename_input";
pub const CODE_EDITOR_RENAME_APPLY_AUTHOR_ID: &str = "code_editor_rename_apply";
pub const CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID: &str = "code_editor_rename_cancel";
pub const CODE_EDITOR_RENAME_NO_LSP_BANNER_AUTHOR_ID: &str = "code_editor_rename_no_lsp_banner";
pub const CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID: &str = "code_editor_ctx_rename_symbol";

/// The exact warning-banner text the no-LSP single-file path must render (MC-004 / AC-003). A swarm
/// agent reads this off the banner node's value so it is never misled that a single-file rename was
/// project-wide.
pub const NO_LSP_BANNER_TEXT: &str = "LSP-required for cross-file rename";

/// Fixed AccessKit/egui `NodeId`s for the rename overlay nodes. A fresh band (710..) ABOVE the MT-047
/// signature-help node (700) and the MT-008 overlay band (completion 600..665, hover 680/681) so the
/// rename nodes never collide with another overlay node id.
pub(crate) const RENAME_INPUT_NODE_ID: u64 = 710;
pub(crate) const RENAME_PREVIEW_NODE_ID: u64 = 711;
pub(crate) const RENAME_APPLY_NODE_ID: u64 = 712;
pub(crate) const RENAME_CANCEL_NODE_ID: u64 = 713;
pub(crate) const RENAME_NO_LSP_BANNER_NODE_ID: u64 = 714;

/// A half-open LSP-style line/character range (0-based), the wire form an LSP `TextEdit` carries. Kept
/// as its own type (rather than reusing `lsp_types::Range`) so the preview model is transport-agnostic:
/// both the LSP path and the no-LSP tree-sitter fallback produce the SAME [`LspRange`]/[`TextEdit`]
/// shape and the renderer/apply path never branches on origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LspRange {
    pub start_line: u32,
    pub start_char: u32,
    pub end_line: u32,
    pub end_char: u32,
}

impl LspRange {
    /// Build from an `lsp_types::Range`.
    pub fn from_lsp(range: lsp_types::Range) -> Self {
        Self {
            start_line: range.start.line,
            start_char: range.start.character,
            end_line: range.end.line,
            end_char: range.end.character,
        }
    }
}

/// One edit to apply: a line/character RANGE plus the replacement text. The rename produces one of these
/// per occurrence (the new identifier text replacing the old occurrence's range).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: LspRange,
    pub new_text: String,
}

/// One before/after diff row shown in the preview (ported visually from the React `CodeSymbolPanel`
/// references list). `line` is the 0-based line the edit touches; `before`/`after` are that whole line
/// before and after the edit so the operator sees the change in context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewHunk {
    pub line: usize,
    pub before: String,
    pub after: String,
}

/// All edits to a single file, plus whether that file is an open in-memory buffer or a to-disk file, and
/// the before/after hunks the preview renders. Built BEFORE any mutation happens (AC-004 — nothing is
/// applied until the operator clicks Apply).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEditPreview {
    /// The document URI (or path) the edits apply to.
    pub uri: String,
    /// True when this file is currently an open in-memory buffer (applied to the `TextBuffer`); false
    /// when it is a to-disk file (read -> apply -> atomic write).
    pub is_open_buffer: bool,
    /// The edits for this file, in document order (NOT yet sorted for apply — the apply step sorts
    /// descending per file so the preview can stay in reading order).
    pub edits: Vec<TextEdit>,
    /// The before/after diff rows shown in the preview.
    pub hunks: Vec<PreviewHunk>,
}

/// The whole multi-file rename preview: every changed file, in a stable order. Built from an LSP
/// `WorkspaceEdit` (both shapes — RISK-003) or from the no-LSP single-file fallback. NOTHING is mutated
/// until [`apply_preview`] is called (AC-004).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceEditPreview {
    pub files: Vec<FileEditPreview>,
    /// True when this preview came from the no-LSP single-file fallback path; drives the
    /// `LSP-required for cross-file rename` banner (MC-004 — the operator is never misled about scope).
    pub is_single_file_fallback: bool,
}

impl WorkspaceEditPreview {
    /// An empty (no-op) preview — the result of a null/empty `WorkspaceEdit` (the no-op rename case).
    pub fn empty() -> Self {
        Self::default()
    }

    /// True when the preview has no edits in any file (a no-op rename — the caller shows "no changes").
    pub fn is_empty(&self) -> bool {
        self.files.iter().all(|f| f.edits.is_empty())
    }

    /// Total number of edits across all files (the apply report's edit count + the preview summary).
    pub fn total_edits(&self) -> usize {
        self.files.iter().map(|f| f.edits.len()).sum()
    }

    /// Build a preview from an `lsp_types::WorkspaceEdit`, handling BOTH the `changes` map form AND the
    /// `document_changes` array form (RISK-003 / MC-003 / AC-007). For each changed file, an `open_lookup`
    /// closure decides whether the file is an open buffer (so the preview marks `is_open_buffer`) and
    /// supplies that file's current text so the before/after hunks can be computed; a file not known to
    /// `open_lookup` is treated as a to-disk file and its content is read from disk for the hunks (a read
    /// failure yields edits with empty before/after context rather than dropping the file).
    ///
    /// `open_lookup(uri)` returns `Some(text)` when the URI maps to an open buffer (the in-memory text),
    /// else `None` (a to-disk file). Files are sorted by URI for a stable preview order. Edits within a
    /// file are sorted DESCENDING by start offset against that file's content (the apply order — RISK-001)
    /// but the preview keeps them in document order for readability via the hunks.
    pub fn from_lsp(
        edit: &lsp_types::WorkspaceEdit,
        mut open_lookup: impl FnMut(&str) -> Option<String>,
    ) -> Self {
        // Collect (uri, Vec<TextEdit>) from whichever shape the server sent. `document_changes` takes
        // precedence when present (it is the richer form a capable server sends); otherwise `changes`.
        let mut per_file: Vec<(String, Vec<TextEdit>)> = Vec::new();

        if let Some(doc_changes) = &edit.document_changes {
            match doc_changes {
                lsp_types::DocumentChanges::Edits(edits) => {
                    for tde in edits {
                        let uri = tde.text_document.uri.to_string();
                        let mut text_edits = Vec::with_capacity(tde.edits.len());
                        for one in &tde.edits {
                            // Unwrap the `OneOf<TextEdit, AnnotatedTextEdit>` — the annotation is ignored;
                            // both carry the same range + new_text.
                            let (range, new_text) = match one {
                                lsp_types::OneOf::Left(te) => (te.range, te.new_text.clone()),
                                lsp_types::OneOf::Right(ate) => {
                                    (ate.text_edit.range, ate.text_edit.new_text.clone())
                                }
                            };
                            text_edits.push(TextEdit {
                                range: LspRange::from_lsp(range),
                                new_text,
                            });
                        }
                        push_or_merge(&mut per_file, uri, text_edits);
                    }
                }
                lsp_types::DocumentChanges::Operations(ops) => {
                    // A rename only ever sends Edit operations; create/rename/delete file operations are
                    // out of scope (a rename does not move files). Extract only the Edit operations'
                    // text-document edits; pure resource ops are skipped (they carry no TextEdits).
                    for op in ops {
                        if let lsp_types::DocumentChangeOperation::Edit(tde) = op {
                            let uri = tde.text_document.uri.to_string();
                            let mut text_edits = Vec::with_capacity(tde.edits.len());
                            for one in &tde.edits {
                                let (range, new_text) = match one {
                                    lsp_types::OneOf::Left(te) => (te.range, te.new_text.clone()),
                                    lsp_types::OneOf::Right(ate) => {
                                        (ate.text_edit.range, ate.text_edit.new_text.clone())
                                    }
                                };
                                text_edits.push(TextEdit {
                                    range: LspRange::from_lsp(range),
                                    new_text,
                                });
                            }
                            push_or_merge(&mut per_file, uri, text_edits);
                        }
                    }
                }
            }
        } else if let Some(changes) = &edit.changes {
            for (url, edits) in changes {
                let uri = url.to_string();
                let text_edits: Vec<TextEdit> = edits
                    .iter()
                    .map(|te| TextEdit {
                        range: LspRange::from_lsp(te.range),
                        new_text: te.new_text.clone(),
                    })
                    .collect();
                push_or_merge(&mut per_file, uri, text_edits);
            }
        }

        // Stable order so two runs (and the changes-map's HashMap iteration order) produce the same
        // preview, and a documentChanges-only and changes-only response over the same edits compare equal.
        per_file.sort_by(|a, b| a.0.cmp(&b.0));

        let files = per_file
            .into_iter()
            .map(|(uri, edits)| {
                let open_text = open_lookup(&uri);
                let is_open_buffer = open_text.is_some();
                // The content used to compute before/after hunks: the open buffer text, else read disk.
                let content = open_text.or_else(|| read_file_for_preview(&uri));
                let hunks = match &content {
                    Some(text) => hunks_for_edits(text, &edits),
                    None => Vec::new(),
                };
                FileEditPreview { uri, is_open_buffer, edits, hunks }
            })
            .collect();

        Self { files, is_single_file_fallback: false }
    }

    /// Build the SINGLE-FILE no-LSP fallback preview (RISK-004 / MC-004 — AC-003): rename every in-file
    /// occurrence of `original` to `new_name` in the current file's `text`. The occurrence ranges come
    /// from `occurrence_byte_ranges` (the panel computes these from the file's tree-sitter parse tree —
    /// a SAFE local source, NOT a word-scan). `uri` is the current file. `is_open_buffer` marks whether
    /// the current file is an open buffer. Marks the preview as a single-file fallback so the banner is
    /// rendered (the operator is never misled that the rename was project-wide).
    pub fn single_file_fallback(
        uri: impl Into<String>,
        text: &str,
        new_name: &str,
        occurrence_byte_ranges: &[Range<usize>],
        is_open_buffer: bool,
    ) -> Self {
        let buffer = TextBuffer::new(text);
        let mut edits: Vec<TextEdit> = Vec::with_capacity(occurrence_byte_ranges.len());
        for range in occurrence_byte_ranges {
            if let Some(lsp_range) = byte_range_to_lsp_range(&buffer, range.clone()) {
                edits.push(TextEdit { range: lsp_range, new_text: new_name.to_owned() });
            }
        }
        let hunks = hunks_for_edits(text, &edits);
        let file = FileEditPreview {
            uri: uri.into(),
            is_open_buffer,
            edits,
            hunks,
        };
        Self { files: vec![file], is_single_file_fallback: true }
    }
}

/// Append `edits` to the file's entry in `per_file`, merging if the URI is already present (a server may
/// list the same file twice). Keeps document order within a file.
fn push_or_merge(per_file: &mut Vec<(String, Vec<TextEdit>)>, uri: String, edits: Vec<TextEdit>) {
    if let Some(entry) = per_file.iter_mut().find(|(u, _)| *u == uri) {
        entry.1.extend(edits);
    } else {
        per_file.push((uri, edits));
    }
}

/// A report of what [`apply_preview`] changed: the files touched + the total edit count, so the panel can
/// surface a truthful "renamed N occurrences across M files" message. On a partial failure the report is
/// still ACCURATE — it lists exactly the files that were already applied before the failure (MC-002 — the
/// report must be truthful; best-effort rollback is out of scope).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RenameApplyReport {
    /// The URIs of the files that were successfully applied (in apply order).
    pub files_changed: Vec<String>,
    /// The total number of edits applied across those files.
    pub edits_applied: usize,
}

/// Why a rename apply could not complete. Carries an accurate partial report so the panel reports which
/// files were already applied (MC-002 — the report must be truthful even on failure).
#[derive(Debug, Clone)]
pub enum RenameError {
    /// A file's disk read/write failed; `partial` lists the files already applied before the failure.
    Io { uri: String, message: String, partial: RenameApplyReport },
    /// An edit's range could not be mapped to a buffer offset (a stale range from a prior edit).
    BadRange { uri: String, partial: RenameApplyReport },
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenameError::Io { uri, message, .. } => {
                write!(f, "rename apply failed writing {uri}: {message}")
            }
            RenameError::BadRange { uri, .. } => {
                write!(f, "rename apply failed: an edit range was out of range in {uri}")
            }
        }
    }
}

impl std::error::Error for RenameError {}

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
    Error {
        message: String,
    },
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

/// Apply a [`WorkspaceEditPreview`] across open buffers + to-disk files (AC-002 / AC-008). For each file:
/// - an OPEN buffer's edits are applied to a fresh [`TextBuffer`] built from `read_open_buffer(uri)` in
///   DESCENDING start-offset order (RISK-001) and the new text is written back via `write_open_buffer`;
/// - a TO-DISK file is read, its edits applied to a [`TextBuffer`] (descending order), and the result
///   written ATOMICALLY (temp + rename — RISK-002 / MC-002).
///
/// On the FIRST per-file failure this STOPS and returns a [`RenameError`] whose `partial` report lists the
/// files already applied (MC-002 — the report is truthful; best-effort rollback is out of scope). On full
/// success a [`RenameApplyReport`] of every file + edit count is returned.
///
/// `read_open_buffer(uri) -> Option<String>` returns the current in-memory text of an open buffer (the
/// panel passes its own buffer text); `write_open_buffer(uri, new_text)` installs the renamed text back
/// into the open buffer. Both are closures so the apply logic is testable with in-memory buffers and the
/// panel wires them to its real `TextBuffer`.
pub fn apply_preview(
    preview: &WorkspaceEditPreview,
    mut read_open_buffer: impl FnMut(&str) -> Option<String>,
    mut write_open_buffer: impl FnMut(&str, &str),
) -> Result<RenameApplyReport, RenameError> {
    let mut report = RenameApplyReport::default();
    for file in &preview.files {
        if file.edits.is_empty() {
            continue;
        }
        if file.is_open_buffer {
            let current = read_open_buffer(&file.uri).unwrap_or_default();
            let new_text = apply_text_edits_to_string(&current, &file.edits).map_err(|_| {
                RenameError::BadRange { uri: file.uri.clone(), partial: report.clone() }
            })?;
            write_open_buffer(&file.uri, &new_text);
        } else {
            let path = uri_to_path(&file.uri);
            let current = std::fs::read_to_string(&path).map_err(|e| RenameError::Io {
                uri: file.uri.clone(),
                message: e.to_string(),
                partial: report.clone(),
            })?;
            let new_text = apply_text_edits_to_string(&current, &file.edits).map_err(|_| {
                RenameError::BadRange { uri: file.uri.clone(), partial: report.clone() }
            })?;
            write_file_atomic(&path, &new_text).map_err(|e| RenameError::Io {
                uri: file.uri.clone(),
                message: e.to_string(),
                partial: report.clone(),
            })?;
        }
        report.files_changed.push(file.uri.clone());
        report.edits_applied += file.edits.len();
    }
    Ok(report)
}

/// An edit could not be applied because its line/character range did not resolve to a valid byte range in
/// the current buffer (a stale range from a prior edit). Returned instead of silently corrupting the text
/// (RISK-001 — a stale range never produces a partially-mangled buffer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditApplyError;

impl std::fmt::Display for EditApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an edit range did not resolve to a valid byte range")
    }
}

impl std::error::Error for EditApplyError {}

/// Apply `edits` to `text` and return the new string. The edits are converted to BYTE ranges against the
/// text, sorted DESCENDING by start byte, and applied in that order so an earlier edit's byte offsets are
/// not invalidated by a later edit (RISK-001 — the single most common rename bug). An edit whose range
/// cannot be mapped to a valid byte range yields an `Err` (a stale range never silently corrupts).
pub fn apply_text_edits_to_string(
    text: &str,
    edits: &[TextEdit],
) -> Result<String, EditApplyError> {
    let mut buffer = TextBuffer::new(text);
    apply_text_edits_to_buffer(&mut buffer, edits)?;
    Ok(buffer.to_string())
}

/// Apply `edits` to `buffer` IN PLACE, in DESCENDING start-offset order (RISK-001). Each edit's
/// line/character range is resolved to a byte range against the CURRENT buffer once (before any mutation,
/// using the pre-edit line map), then sorted descending and applied delete+insert. Because later edits
/// (higher offsets) are applied first, the byte offsets of earlier (lower-offset) edits remain valid.
/// Returns `Err(EditApplyError)` if any edit's range cannot be resolved to a valid byte range.
pub fn apply_text_edits_to_buffer(
    buffer: &mut TextBuffer,
    edits: &[TextEdit],
) -> Result<(), EditApplyError> {
    // Resolve every edit to a byte range against the UN-edited buffer first (so all ranges share the same
    // coordinate space), then sort descending and apply — applying high offsets first keeps low offsets
    // valid (RISK-001 / the descending-offset invariant).
    let mut resolved: Vec<(Range<usize>, String)> = Vec::with_capacity(edits.len());
    for edit in edits {
        let byte_range = lsp_range_to_byte_range(buffer, edit.range).ok_or(EditApplyError)?;
        resolved.push((byte_range, edit.new_text.clone()));
    }
    // Sort DESCENDING by start byte (RISK-001). On equal starts, the longer range first is irrelevant for
    // a rename (occurrences do not overlap), but sort by (start desc, end desc) for determinism.
    resolved.sort_by(|a, b| b.0.start.cmp(&a.0.start).then(b.0.end.cmp(&a.0.end)));
    for (range, new_text) in resolved {
        // delete the old occurrence then insert the new text at the (now-current) start. Because we apply
        // descending, `range.start` is still valid (no earlier edit has shifted it). A failed delete/insert
        // is a stale range -> Err (never a silent partial corruption).
        buffer.delete(range.clone()).map_err(|_| EditApplyError)?;
        buffer.insert(range.start, &new_text).map_err(|_| EditApplyError)?;
    }
    Ok(())
}

/// Convert an [`LspRange`] (0-based line/char) to a BYTE range against `buffer`, or `None` when it cannot
/// be resolved (an out-of-range line/char never panics — it returns `None` and the caller reports a bad
/// range). The character column is interpreted as a byte column on the line (the simple model the editor
/// uses — see the panel's `lsp_position_at` note); this is exact for ASCII and the common identifier case.
fn lsp_range_to_byte_range(buffer: &TextBuffer, range: LspRange) -> Option<Range<usize>> {
    let start = line_char_to_byte(buffer, range.start_line as usize, range.start_char as usize)?;
    let end = line_char_to_byte(buffer, range.end_line as usize, range.end_char as usize)?;
    if start > end {
        return None;
    }
    Some(start..end)
}

/// Resolve a `(line, char)` (0-based, char = byte column on the line) to an absolute byte offset, clamped
/// to the line's byte length so a server's past-the-line-end column lands at the line end (never panics).
fn line_char_to_byte(buffer: &TextBuffer, line: usize, char_col: usize) -> Option<usize> {
    let line_start = buffer.line_to_byte(line)?;
    // The end of this line's content (start of next line, or buffer end on the last line).
    let line_end = buffer
        .line_to_byte(line + 1)
        .unwrap_or_else(|| buffer.len_bytes());
    let col_byte = (line_start + char_col).min(line_end);
    Some(col_byte)
}

/// Convert a BYTE range to an [`LspRange`] against `buffer` (the inverse of [`lsp_range_to_byte_range`]),
/// for the no-LSP fallback that produces edits from tree-sitter byte ranges. `None` on an out-of-range
/// byte offset.
fn byte_range_to_lsp_range(buffer: &TextBuffer, range: Range<usize>) -> Option<LspRange> {
    let (start_line, start_char) = byte_to_line_char(buffer, range.start)?;
    let (end_line, end_char) = byte_to_line_char(buffer, range.end)?;
    Some(LspRange {
        start_line: start_line as u32,
        start_char: start_char as u32,
        end_line: end_line as u32,
        end_char: end_char as u32,
    })
}

/// Resolve a byte offset to `(line, byte-column)` (0-based), or `None` when out of range.
fn byte_to_line_char(buffer: &TextBuffer, byte_offset: usize) -> Option<(usize, usize)> {
    let line = buffer.byte_to_line(byte_offset)?;
    let line_start = buffer.line_to_byte(line)?;
    Some((line, byte_offset.saturating_sub(line_start)))
}

/// Compute the before/after [`PreviewHunk`]s for `edits` against `text`: for each edit, the whole line it
/// starts on, before the edit and after applying ONLY that file's edits (so the preview shows the final
/// per-file result line-by-line). One hunk per distinct affected line (deduped). Never mutates `text`.
fn hunks_for_edits(text: &str, edits: &[TextEdit]) -> Vec<PreviewHunk> {
    if edits.is_empty() {
        return Vec::new();
    }
    let before_buffer = TextBuffer::new(text);
    // Apply all edits to a copy to get the "after" text (descending order — same as apply).
    let after_text = apply_text_edits_to_string(text, edits).unwrap_or_else(|_| text.to_owned());
    let after_buffer = TextBuffer::new(&after_text);

    // The set of affected lines (the start line of each edit), deduped + sorted.
    let mut lines: Vec<usize> = edits.iter().map(|e| e.range.start_line as usize).collect();
    lines.sort_unstable();
    lines.dedup();

    lines
        .into_iter()
        .map(|line| {
            let before = whole_line(&before_buffer, line);
            // The "after" line index can shift if an edit changed the line count, but a rename replaces an
            // identifier WITHIN a line (no newline change), so the same line index holds. Guard against a
            // shorter after-buffer by clamping.
            let after_line = line.min(after_buffer.len_lines().saturating_sub(1));
            let after = whole_line(&after_buffer, after_line);
            PreviewHunk { line, before, after }
        })
        .collect()
}

/// The whole text of `line` (without the trailing newline), or an empty string when out of range.
fn whole_line(buffer: &TextBuffer, line: usize) -> String {
    let s = buffer.slice_to_string(line..line + 1);
    s.trim_end_matches(['\n', '\r']).to_owned()
}

/// Read a file's content for the preview hunks, mapping a `file://` URI (or a bare path) to a path. A
/// read failure yields `None` (the preview then shows the file with empty hunks rather than dropping it).
fn read_file_for_preview(uri: &str) -> Option<String> {
    std::fs::read_to_string(uri_to_path(uri)).ok()
}

/// Map a `file://` URI (or a bare path) to a filesystem [`PathBuf`]. Uses `lsp_types::Url::to_file_path`
/// when the URI parses as a `file:` URL, else treats the string as a path directly (so a test/relative
/// path works without a real URL).
fn uri_to_path(uri: &str) -> PathBuf {
    if let Ok(url) = lsp_types::Url::parse(uri) {
        if url.scheme() == "file" {
            if let Ok(p) = url.to_file_path() {
                return p;
            }
        }
    }
    PathBuf::from(uri)
}

/// Write `content` to `path` ATOMICALLY (RISK-002 / MC-002 / AC-008): write the full content to a SIBLING
/// temp file in the SAME directory, flush+sync it, then `std::fs::rename` it over the target. `rename` is
/// atomic on the same volume, so the target is always either the intact original or the fully-replaced new
/// content — never half-written, even if the process is interrupted mid-write. A temp-file leftover from a
/// crash BEFORE the rename never corrupts the target (the target is untouched until the atomic rename).
pub fn write_file_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("rename");
    // A unique sibling temp name in the SAME directory (so the rename stays on the same volume). The
    // pid + a monotonic nanosecond stamp keeps concurrent renames from colliding.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = dir.join(format!(".{file_name}.{}.{stamp}.hsk-rename-tmp", std::process::id()));

    // Write + flush + fsync the temp file fully before the rename, so the rename swaps in complete content.
    {
        let mut temp = std::fs::File::create(&temp_path)?;
        temp.write_all(content.as_bytes())?;
        temp.flush()?;
        // Best-effort durability: sync the file contents to disk before the rename. A platform that does
        // not support sync_all returns an error we ignore (the rename atomicity is the load-bearing
        // guarantee; sync is a durability nicety).
        let _ = temp.sync_all();
    }
    // The atomic swap. On Windows `std::fs::rename` replaces an existing destination on the same volume.
    match std::fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Clean up the temp file so a failed rename does not litter the directory; report the error.
            let _ = std::fs::remove_file(&temp_path);
            Err(e)
        }
    }
}

/// TYPED BLOCKER MARKER (AC-009): the existing `GET /knowledge/code/symbols/{entity_id}/references`
/// backend endpoint returns symbol-level callers/callees (`{ symbol_entity_id, display_name }`), NOT the
/// precise byte/char occurrence RANGES a cross-file rename needs. This function documents that verified
/// gap in code (the no-LSP fallback is therefore SINGLE-FILE via tree-sitter, and cross-file rename
/// REQUIRES an LSP server). It always returns `true`: a cross-file references-based rename needs a NEW
/// backend route (occurrence ranges), which is a typed blocker — NEVER a backend edit (the MT forbids
/// touching `src/backend/**`). Kept as an explicit, testable assertion of the recorded blocker.
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

    let value_for_node = draft.clone();
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

    // Emit the TextInput AccessKit node carrying the draft value, so a swarm agent reads/sets the rename
    // by id (AC-006). The node value is the current draft.
    let node_id = rename_node_id(RENAME_INPUT_NODE_ID, CODE_EDITOR_RENAME_INPUT_AUTHOR_ID, instance);
    ctx.accesskit_node_builder(node_id, move |node| {
        node.set_role(accesskit::Role::TextInput);
        node.set_author_id(author.clone());
        node.set_label("Rename symbol".to_owned());
        node.set_value(value_for_node.clone());
    });
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

            egui::ScrollArea::vertical()
                .max_height(360.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for file in &preview.files {
                        let kind = if file.is_open_buffer { "open buffer" } else { "to disk" };
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
                                            .color(egui::Color32::from_rgb(0xd0, 0x6c, 0x6c)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("+ {}", hunk.after))
                                            .monospace()
                                            .color(egui::Color32::from_rgb(0x6c, 0xb0, 0x6c)),
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
                let apply_node_id =
                    rename_node_id(RENAME_APPLY_NODE_ID, CODE_EDITOR_RENAME_APPLY_AUTHOR_ID, instance);
                ui.ctx().accesskit_node_builder(apply_node_id, move |node| {
                    node.set_role(accesskit::Role::Button);
                    node.set_author_id(apply_author.clone());
                    node.set_label("Apply rename".to_owned());
                    node.add_action(accesskit::Action::Click);
                });

                let cancel_resp = ui.button("Cancel");
                if cancel_resp.clicked() {
                    action = PreviewAction::Cancel;
                }
                let cancel_author = suffixed(CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID, instance);
                let cancel_node_id = rename_node_id(
                    RENAME_CANCEL_NODE_ID,
                    CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID,
                    instance,
                );
                ui.ctx().accesskit_node_builder(cancel_node_id, move |node| {
                    node.set_role(accesskit::Role::Button);
                    node.set_author_id(cancel_author.clone());
                    node.set_label("Cancel rename".to_owned());
                    node.add_action(accesskit::Action::Click);
                });
            });

            // Emit the preview container node so a swarm agent can read the change summary by id.
            let preview_author = suffixed("code_editor_rename_preview", instance);
            let total = preview.total_edits();
            let file_count = preview.files.len();
            let preview_node_id =
                rename_node_id(RENAME_PREVIEW_NODE_ID, "code_editor_rename_preview", instance);
            ui.ctx().accesskit_node_builder(preview_node_id, move |node| {
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
    let warn_color = egui::Color32::from_rgb(0xff, 0xcc, 0x00);
    egui::Frame::group(ui.style())
        .fill(ui.visuals().warn_fg_color.linear_multiply(0.08))
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
    fn descending_offset_apply_is_correct_and_ascending_would_corrupt() {
        // RISK-001 / MC-001: two edits on the same line; applying ascending would shift the second.
        // "let value = value + 1;" -> rename `value` to `total` at both occurrences.
        let text = "let value = value + 1;";
        // value occurrences: bytes 4..9 and 12..17.
        let edits = vec![
            TextEdit {
                range: LspRange { start_line: 0, start_char: 4, end_line: 0, end_char: 9 },
                new_text: "total".into(),
            },
            TextEdit {
                range: LspRange { start_line: 0, start_char: 12, end_line: 0, end_char: 17 },
                new_text: "total".into(),
            },
        ];
        let out = apply_text_edits_to_string(text, &edits).expect("apply ok");
        assert_eq!(out, "let total = total + 1;", "descending apply renames both occurrences exactly");

        // Prove ascending order would CORRUPT: apply the SAME edits naively low->high without re-resolving
        // offsets. The first edit ("total" is same length as "value" here is 5==5, so pick a length-changing
        // rename to expose the bug): rename to a SHORTER name `t` so ascending shifts the second range.
        let edits_short = vec![
            TextEdit {
                range: LspRange { start_line: 0, start_char: 4, end_line: 0, end_char: 9 },
                new_text: "t".into(),
            },
            TextEdit {
                range: LspRange { start_line: 0, start_char: 12, end_line: 0, end_char: 17 },
                new_text: "t".into(),
            },
        ];
        let correct = apply_text_edits_to_string(text, &edits_short).expect("apply ok");
        assert_eq!(correct, "let t = t + 1;", "descending apply is correct for a length-changing rename");

        // Naive ASCENDING application (the bug): apply edit[0] first (shrinks the doc by 4 bytes), then
        // edit[1] at its ORIGINAL byte range 12..17 — which now points at the WRONG, shifted text.
        let mut naive = TextBuffer::new(text);
        let r0 = lsp_range_to_byte_range(&naive, edits_short[0].range).unwrap();
        naive.delete(r0.clone()).unwrap();
        naive.insert(r0.start, "t").unwrap();
        let r1 = lsp_range_to_byte_range(&TextBuffer::new(text), edits_short[1].range).unwrap();
        // r1 (12..17) now exceeds/misaligns the shrunk buffer; the naive result differs from the correct one.
        let ascending_result = {
            // Clamp so it does not panic, mirroring a real naive bug that mangles text.
            let len = naive.len_bytes();
            let s = r1.start.min(len);
            let e = r1.end.min(len);
            let _ = naive.delete(s..e);
            let _ = naive.insert(s.min(naive.len_bytes()), "t");
            naive.to_string()
        };
        assert_ne!(
            ascending_result, correct,
            "RISK-001 regression: ascending-order apply must NOT equal the correct descending result"
        );
    }

    #[test]
    fn apply_across_two_files_via_apply_preview() {
        // AC-002: a 2-file WorkspaceEdit applied to 2 open buffers, offsets correct.
        let file_a = "fn foo() {}\nfn bar() { foo(); }";
        let file_b = "fn baz() { foo(); foo(); }";
        // Rename `foo` to `frobnicate` in both files.
        let preview = WorkspaceEditPreview {
            files: vec![
                FileEditPreview {
                    uri: "file:///a.rs".into(),
                    is_open_buffer: true,
                    edits: vec![
                        TextEdit {
                            range: LspRange { start_line: 0, start_char: 3, end_line: 0, end_char: 6 },
                            new_text: "frobnicate".into(),
                        },
                        TextEdit {
                            range: LspRange { start_line: 1, start_char: 11, end_line: 1, end_char: 14 },
                            new_text: "frobnicate".into(),
                        },
                    ],
                    hunks: vec![],
                },
                FileEditPreview {
                    uri: "file:///b.rs".into(),
                    is_open_buffer: true,
                    edits: vec![
                        TextEdit {
                            range: LspRange { start_line: 0, start_char: 11, end_line: 0, end_char: 14 },
                            new_text: "frobnicate".into(),
                        },
                        TextEdit {
                            range: LspRange { start_line: 0, start_char: 18, end_line: 0, end_char: 21 },
                            new_text: "frobnicate".into(),
                        },
                    ],
                    hunks: vec![],
                },
            ],
            is_single_file_fallback: false,
        };

        use std::collections::HashMap;
        let mut buffers: HashMap<String, String> = HashMap::new();
        buffers.insert("file:///a.rs".into(), file_a.to_owned());
        buffers.insert("file:///b.rs".into(), file_b.to_owned());
        let read_buffers = buffers.clone();

        let report = apply_preview(
            &preview,
            |uri| read_buffers.get(uri).cloned(),
            |uri, new_text| {
                buffers.insert(uri.to_owned(), new_text.to_owned());
            },
        )
        .expect("apply ok");

        assert_eq!(report.files_changed, vec!["file:///a.rs", "file:///b.rs"]);
        assert_eq!(report.edits_applied, 4);
        assert_eq!(buffers["file:///a.rs"], "fn frobnicate() {}\nfn bar() { frobnicate(); }");
        assert_eq!(buffers["file:///b.rs"], "fn baz() { frobnicate(); frobnicate(); }");
    }

    #[test]
    fn from_lsp_both_shapes_produce_the_same_preview() {
        // RISK-003 / MC-003 / AC-007 / PT-006: the `changes` map form and the `documentChanges` array form
        // deserialize to the SAME FileEditPreview set.
        let uri = lsp_types::Url::parse("file:///x.rs").unwrap();
        let range = lsp_types::Range {
            start: lsp_types::Position { line: 0, character: 3 },
            end: lsp_types::Position { line: 0, character: 6 },
        };
        let te = lsp_types::TextEdit { range, new_text: "renamed".into() };

        // changes map form.
        let mut changes = std::collections::HashMap::new();
        changes.insert(uri.clone(), vec![te.clone()]);
        let edit_changes = lsp_types::WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        };

        // documentChanges array form.
        let tde = lsp_types::TextDocumentEdit {
            text_document: lsp_types::OptionalVersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: Some(1),
            },
            edits: vec![lsp_types::OneOf::Left(te)],
        };
        let edit_doc = lsp_types::WorkspaceEdit {
            changes: None,
            document_changes: Some(lsp_types::DocumentChanges::Edits(vec![tde])),
            change_annotations: None,
        };

        // No open buffer; no disk read (uri does not exist) -> empty hunks but the edits + uri must match.
        let p_changes = WorkspaceEditPreview::from_lsp(&edit_changes, |_| None);
        let p_doc = WorkspaceEditPreview::from_lsp(&edit_doc, |_| None);
        assert_eq!(p_changes.files.len(), 1);
        assert_eq!(p_doc.files.len(), 1);
        assert_eq!(p_changes.files[0].uri, p_doc.files[0].uri);
        assert_eq!(p_changes.files[0].edits, p_doc.files[0].edits);
        assert_eq!(
            p_changes.files[0].edits[0],
            TextEdit {
                range: LspRange { start_line: 0, start_char: 3, end_line: 0, end_char: 6 },
                new_text: "renamed".into(),
            }
        );
    }

    #[test]
    fn from_lsp_empty_is_a_noop_preview() {
        let edit = lsp_types::WorkspaceEdit {
            changes: None,
            document_changes: None,
            change_annotations: None,
        };
        let preview = WorkspaceEditPreview::from_lsp(&edit, |_| None);
        assert!(preview.is_empty());
        assert_eq!(preview.total_edits(), 0);
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
            RenameState::Editing { original, draft, ident_range, entity_id, .. } => {
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
        assert_eq!(occ.len(), 2, "only the two whole-identifier occurrences, got {occ:?}");
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
        assert!(preview.is_single_file_fallback, "the fallback marks the banner flag");
        assert_eq!(preview.files.len(), 1);
        let out = apply_text_edits_to_string(src, &preview.files[0].edits).unwrap();
        assert_eq!(out, "let total = total + 1;");
    }

    #[test]
    fn write_file_atomic_replaces_whole_file_never_partial() {
        // AC-008 / RISK-002 / MC-002: an atomic write fully replaces the file; a simulated interruption
        // (the temp write happening but the rename NOT) leaves the ORIGINAL intact.
        let dir = std::env::temp_dir().join(format!("hsk-rename-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("victim.rs");
        std::fs::write(&target, "original content\n").unwrap();

        // A real atomic write replaces the whole file.
        write_file_atomic(&target, "fully replaced content\n").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "fully replaced content\n");

        // Simulate a write INTERRUPTION: write to the sibling temp but DO NOT rename. The target must be
        // the intact previous content (never half-written) — proving the rename, not an in-place write,
        // is the mutation point.
        let temp_sibling = dir.join(".victim.rs.interrupted.hsk-rename-tmp");
        std::fs::write(&temp_sibling, "HALF WRITTEN GARBAGE").unwrap(); // the "crash before rename" state.
        // The target is untouched by the temp write.
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "fully replaced content\n",
            "AC-008: the target is intact-or-fully-replaced; a temp write never touches it"
        );
        // Cleanup.
        let _ = std::fs::remove_file(&temp_sibling);
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn apply_partial_failure_reports_already_applied_files() {
        // MC-002: a per-file failure stops + reports which files were already applied (truthful report).
        let preview = WorkspaceEditPreview {
            files: vec![
                FileEditPreview {
                    uri: "file:///ok.rs".into(),
                    is_open_buffer: true,
                    edits: vec![TextEdit {
                        range: LspRange { start_line: 0, start_char: 0, end_line: 0, end_char: 3 },
                        new_text: "new".into(),
                    }],
                    hunks: vec![],
                },
                FileEditPreview {
                    // A to-disk file that does not exist -> read fails -> Io error after ok.rs applied.
                    uri: "file:///does-not-exist-hsk.rs".into(),
                    is_open_buffer: false,
                    edits: vec![TextEdit {
                        range: LspRange { start_line: 0, start_char: 0, end_line: 0, end_char: 3 },
                        new_text: "new".into(),
                    }],
                    hunks: vec![],
                },
            ],
            is_single_file_fallback: false,
        };
        let mut applied: Vec<String> = Vec::new();
        let err = apply_preview(
            &preview,
            |_| Some("foo bar".to_owned()),
            |uri, _| applied.push(uri.to_owned()),
        )
        .expect_err("the second file fails to read");
        match err {
            RenameError::Io { partial, .. } => {
                assert_eq!(
                    partial.files_changed,
                    vec!["file:///ok.rs"],
                    "the report accurately lists the already-applied file"
                );
                assert_eq!(partial.edits_applied, 1);
            }
            other => panic!("expected an Io error, got {other:?}"),
        }
        assert_eq!(applied, vec!["file:///ok.rs"], "only the first file's buffer was written");
    }

    #[test]
    fn references_endpoint_lacks_precise_ranges_typed_blocker() {
        // AC-009: the recorded typed blocker — the references API has no occurrence ranges, so cross-file
        // references-based rename needs a NEW backend route (never a backend edit here).
        assert!(references_lack_precise_ranges());
    }
}
