//! Format Document (Alt+Shift+F) + Format Selection — the LSP-backed code-formatting affordance for the
//! native code editor (WP-KERNEL-012 MT-050 — E1 VS Code parity).
//!
//! The operator presses Alt+Shift+F (or chooses "Format Document" from the EDIT menu / editor body
//! context menu) and the editor asks the attached language server to reformat the whole document via
//! `textDocument/formatting`; "Format Selection" asks for `textDocument/rangeFormatting` over the
//! current selection. The server returns an array of LSP `TextEdit`s, which this module applies to the
//! ropey [`TextBuffer`] as a SINGLE undo step so one Ctrl+Z reverts the entire format (VS Code parity).
//!
//! ## Why this is data-integrity-critical (the same class as MT-048 rename)
//!
//! Formatting mutates the document. Three invariants are non-negotiable, and each is proven by a test:
//!
//! 1. **Descending-offset apply** ([`apply_text_edits`]): the incoming `TextEdit`s are resolved to byte
//!    ranges against the PRE-edit buffer, then sorted DESCENDING by start offset and applied in that
//!    order, so an earlier (lower-offset) edit's byte offsets are never invalidated by a later edit that
//!    changed the document length before it. This reuses the EXACT discipline MT-048's
//!    `rename::apply_text_edits_to_buffer` established (RISK-001 / MC-004 / AC-005). Applying ascending
//!    silently corrupts the buffer; the `format_descending_order` test proves the descending result is
//!    correct AND that a naive ascending application would differ.
//! 2. **Single undo group** ([`resolve_format_outcome`] feeds the panel boundary): the whole `TextEdit`
//!    array is applied to ONE working buffer, then the panel records ONE `UndoAction` whose `undo_fn`
//!    restores the pre-format snapshot. MT-010's undo model is a snapshot-based `UndoAction` ring with NO
//!    `begin_group`/`end_group` API (verified — `undo_stack.rs`), so the single-undo requirement is met by
//!    the single-transaction approach MT-048 used: one snapshot before, one after, one entry. The live path
//!    is `CodeEditorPanel::pump_formatting` -> off-thread `LspClient::format_document` -> this module's
//!    [`resolve_format_outcome`] -> `drain_format_result`, which installs the formatted text and records
//!    exactly ONE undo entry; the `format_document_single_undo` test proves the single entry + single-undo
//!    revert over the real MT-008 transport (RISK-002 / MC-001 / AC-001).
//! 3. **UTF-16 column semantics** ([`lsp_range_to_byte_range`]): LSP positions are UTF-16 code units by
//!    default (LSP 3.17 `general.positionEncodings` defaults to `utf-16`). Confusing them with raw byte
//!    columns mis-places every edit on a line that contains a non-ASCII character. This module converts
//!    line/UTF-16-character positions to byte offsets via the buffer's char API (NEVER `byte == column`),
//!    and the `format_utf16_columns` test proves an edit on a line where the byte offset != the UTF-16
//!    column lands at the correct character (RISK-003 / MC-005 / AC-007 — the load-bearing i18n gate).
//!
//! ## Typed, panic-free outcomes ([`FormatOutcome`])
//!
//! Every path returns a typed [`FormatOutcome`] (`Applied` | `NoChange` | `NoFormatter` | `LspError`);
//! there is NO `unwrap()`/`expect()`/panic on any production path. The no-formatter and LSP-error cases
//! are values the caller renders (a disabled menu item / a non-blocking toast), never a frame-blocking
//! dialog or a crash (RISK-004 / RISK-005 / MC-006 / AC-003 / AC-006).
//!
//! ## Capability gating ([`formatter_available`])
//!
//! A format action is only enabled when an LSP server is attached for the buffer's language AND the
//! server advertised `documentFormattingProvider` (for Format Document) / `documentRangeFormattingProvider`
//! (for Format Selection) in its `initialize` capabilities. With no LSP at all the actions are disabled
//! (greyed menu items with the "No formatter available for this language" tooltip; the keymap Alt+Shift+F
//! is a no-op). This module owns the capability check; the panel/menu render the disabled state +
//! AccessKit-disabled node so a swarm agent observes the same gating the human sees (RISK-004 / MC-003).
//!
//! ## Backend binding
//!
//! Binds the EXISTING MT-008 stdio LSP transport (`textDocument/formatting`,
//! `textDocument/rangeFormatting`) via the additive [`LspClient::format_document`] /
//! [`LspClient::format_range`] methods — NO second transport, no backend rewrite. This MT persists
//! nothing (formatting is buffer-local), so there is no datastore work and no SQLite anywhere (MC-007).

use std::ops::Range;

use super::buffer::TextBuffer;
use super::lsp_client::{FormattingOptions, LspError, LspRange, LspTextEdit};

/// The exact hover-tooltip text the disabled (no-formatter) menu / context-menu items render (AC-003).
/// A swarm agent reads this off the disabled node so it is never misled that a formatter was available.
pub const NO_FORMATTER_TOOLTIP: &str = "No formatter available for this language";

/// The stable AccessKit author_ids for the EDIT-menu / context-menu format entries, exactly as the
/// MT-050 contract names them so a swarm agent drives formatting without keystrokes (HBR-SWARM). These
/// are referenced by the menu builders (`top_menu_bar.rs` EDIT menu, the editor body context menu) and by
/// the menu-descriptor helper [`menu_descriptors`].
pub const FORMAT_DOCUMENT_MENU_AUTHOR_ID: &str = "menu.edit.format-document";
pub const FORMAT_DOCUMENT_CTX_AUTHOR_ID: &str = "code_editor_ctx_format_document";
pub const FORMAT_SELECTION_CTX_AUTHOR_ID: &str = "code_editor_ctx_format_selection";

/// The typed result of a format action. Keeps the no-formatter + error paths VALUES (not panics) so the
/// caller renders them as a disabled control / a non-blocking toast (AC-006 / MC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatOutcome {
    /// The format applied `edit_count` TextEdits to the buffer as one undo step.
    Applied { edit_count: usize },
    /// The server returned no edits (the document is already formatted) — a successful no-op.
    NoChange,
    /// No formatter is available for the buffer's language (no LSP attached, or the server did not
    /// advertise the formatting capability) — the action is disabled; firing the keymap is a no-op.
    NoFormatter,
    /// The LSP request failed or returned an unparseable body — surfaced as a non-blocking toast, never
    /// a panic (AC-006). Carries a short human-readable reason.
    LspError(String),
}

impl FormatOutcome {
    /// True when text was actually changed (the `Applied` case with a non-zero edit count). Used by the
    /// caller to decide whether to push the single undo entry + re-highlight.
    pub fn changed(&self) -> bool {
        matches!(self, FormatOutcome::Applied { edit_count } if *edit_count > 0)
    }
}

/// Whether a formatter is available for `language_id` over `lsp`: an LSP server must be attached for the
/// buffer's language AND the server must have advertised `documentFormattingProvider` in its `initialize`
/// capabilities (RISK-004 / MC-003). When this is false the EDIT-menu / context-menu "Format Document"
/// item renders disabled and the Alt+Shift+F keymap is a no-op (AC-003). `language_id` is currently used
/// only to gate "an LSP is attached for THIS buffer's language"; a server is launched per-language by the
/// panel, so a configured+running client is, by construction, the server for the open buffer's language —
/// the explicit `language_id` argument keeps the capability surface honest for a future multi-language
/// client and matches the MT contract signature.
pub fn formatter_available(lsp: &super::lsp_client::LspClient, language_id: &str) -> bool {
    // A non-empty language id is required (an unknown/empty language has no server); the panel always
    // passes the buffer's resolved language id. An attached server with the capability gates the action.
    !language_id.trim().is_empty() && lsp.is_running() && lsp.supports_document_formatting()
}

/// Whether RANGE formatting (Format Selection) is available: same as [`formatter_available`] but gated on
/// `documentRangeFormattingProvider` (a server may support whole-document formatting but not range
/// formatting). Format Selection falls back to whole-document formatting only if the contract said so —
/// it does NOT here (the MT is explicit: empty selection sends the current line's range, and a server
/// without range formatting disables the Format Selection entry).
pub fn range_formatter_available(lsp: &super::lsp_client::LspClient, language_id: &str) -> bool {
    !language_id.trim().is_empty() && lsp.is_running() && lsp.supports_document_range_formatting()
}

/// The default [`FormattingOptions`] (`tab_size = 4`, `insert_spaces = true`) — the editor's indent
/// settings (MT-001/MT-010 use 4-space indents throughout). Sourced here rather than hardcoded at each
/// call site so a future per-language override has one place to feed (the MT note: do NOT hardcode at the
/// request site; read the editor's indent config).
pub fn default_formatting_options() -> FormattingOptions {
    FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
    }
}

/// Apply `edits` (an LSP `TextEdit` array) to `buffer` IN PLACE, in DESCENDING start-offset order
/// (RISK-001 / MC-004 / AC-005), and return the number of edits applied. This is the CORE data-integrity
/// primitive, mirroring MT-048's `rename::apply_text_edits_to_buffer`:
///
/// 1. Every edit's UTF-16 line/character range is resolved to a BYTE range against the CURRENT (un-edited)
///    buffer ONCE, so all ranges share the same coordinate space.
/// 2. The resolved (range, text) pairs are sorted DESCENDING by start byte. Applying high offsets first
///    keeps the byte offsets of lower-offset edits valid (the descending-offset invariant).
/// 3. Each edit is applied as delete-then-insert at the (still-valid) start.
///
/// An edit whose UTF-16 range cannot be resolved to a valid byte range yields `Err(FormatApplyError)`
/// (a stale/garbled range never silently corrupts the buffer — the same safety MT-048 holds). On success
/// the buffer is the fully-formatted text; the caller wraps the whole call in ONE undo snapshot so a
/// single Ctrl+Z reverts the entire format (AC-001).
///
/// NOTE: this operates on `(buffer, edits)`. The MT contract sketched an `undo: &mut UndoStack` parameter,
/// but MT-010's undo authority is the snapshot-based host `UndoAction` ring (NO group API — verified in
/// `undo_stack.rs`), so the single-undo grouping is recorded by the PANEL via the existing
/// `interop_adapter::push_code_edit_undo` before/after-snapshot path (the same place every code edit
/// records undo). Keeping the applier undo-free makes it a pure, independently-testable transform and
/// avoids forking a second undo stack (the wrap-not-fork discipline).
pub fn apply_text_edits(
    buffer: &mut TextBuffer,
    edits: &[LspTextEdit],
) -> Result<usize, FormatApplyError> {
    // (1) Resolve every edit to a byte range against the UN-edited buffer (UTF-16 column conversion).
    let mut resolved: Vec<(Range<usize>, String)> = Vec::with_capacity(edits.len());
    for edit in edits {
        let byte_range = lsp_range_to_byte_range(buffer, edit.range).ok_or(FormatApplyError)?;
        resolved.push((byte_range, edit.new_text.clone()));
    }
    // (2) Sort DESCENDING by start byte (RISK-001). On equal starts, longer range first for determinism.
    resolved.sort_by(|a, b| b.0.start.cmp(&a.0.start).then(b.0.end.cmp(&a.0.end)));
    // (3) Apply delete+insert descending so earlier (lower) offsets stay valid.
    let mut applied = 0usize;
    for (range, new_text) in resolved {
        buffer.delete(range.clone()).map_err(|_| FormatApplyError)?;
        buffer
            .insert(range.start, &new_text)
            .map_err(|_| FormatApplyError)?;
        applied += 1;
    }
    Ok(applied)
}

/// Apply `edits` to `text` and return the formatted string (a convenience over [`apply_text_edits`] for
/// the panel's whole-buffer install path and for tests). Sorted-descending + UTF-16-correct, identical to
/// the in-place applier.
pub fn apply_text_edits_to_string(
    text: &str,
    edits: &[LspTextEdit],
) -> Result<String, FormatApplyError> {
    let mut buffer = TextBuffer::new(text);
    apply_text_edits(&mut buffer, edits)?;
    Ok(buffer.to_string())
}

/// An edit could not be applied because its UTF-16 line/character range did not resolve to a valid byte
/// range in the current buffer (a stale or garbled range). Returned instead of silently corrupting the
/// text (RISK-001 — a bad range never produces a partially-mangled buffer).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatApplyError;

impl std::fmt::Display for FormatApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "a format edit range did not resolve to a valid byte range"
        )
    }
}

impl std::error::Error for FormatApplyError {}

/// Convert an [`LspRange`] (0-based line + UTF-16-code-unit character) to a BYTE range against `buffer`,
/// or `None` when it cannot be resolved (an out-of-range line/char never panics — it returns `None` and
/// the caller reports a bad range). UTF-16 column semantics are LOAD-BEARING (AC-007): the character
/// component is a count of UTF-16 code units on the line, NOT a byte offset and NOT a char index, so on a
/// line with a non-ASCII character (e.g. an emoji = 2 UTF-16 code units, 4 bytes) the byte offset differs
/// from the UTF-16 column and a byte-based conversion would mis-place the edit.
pub fn lsp_range_to_byte_range(buffer: &TextBuffer, range: LspRange) -> Option<Range<usize>> {
    let start = line_utf16_to_byte(
        buffer,
        range.start.line as usize,
        range.start.character as usize,
    )?;
    let end = line_utf16_to_byte(
        buffer,
        range.end.line as usize,
        range.end.character as usize,
    )?;
    if start > end {
        return None;
    }
    Some(start..end)
}

/// Resolve a `(line, utf16_column)` (both 0-based; `utf16_column` is a count of UTF-16 code units into the
/// line) to an absolute BYTE offset, clamped to the line's content so a server's past-the-line-end column
/// lands at the line end (never panics). The conversion walks the line's characters once, accumulating
/// UTF-16 code-unit width (`ch.len_utf16()`) until the target column is reached, then maps the resulting
/// char position to a byte offset via the buffer's char API. This is the UTF-16 column math the contract
/// requires (RISK-003 / AC-007): a BMP char is 1 UTF-16 unit, a supplementary-plane char (e.g. an emoji)
/// is 2 — so the byte offset and the UTF-16 column diverge exactly when a non-BMP / multi-byte char
/// precedes the column.
fn line_utf16_to_byte(buffer: &TextBuffer, line: usize, utf16_col: usize) -> Option<usize> {
    let line_start_byte = buffer.line_to_byte(line)?;
    // The end byte of this line's content (start of next line, or buffer end on the last line). Used to
    // bound the walk so a column past the line end clamps to the line end (server may over-report).
    let line_end_byte = buffer
        .line_to_byte(line + 1)
        .unwrap_or_else(|| buffer.len_bytes());
    // Materialize ONLY this line's text (O(line-length), not O(document)) to walk its chars for UTF-16
    // widths. `slice_to_string(line..line+1)` includes the trailing newline; we stop at the content end.
    let line_text = buffer.slice_to_string(line..line + 1);

    let mut utf16_seen = 0usize;
    let mut byte_in_line = 0usize;
    for ch in line_text.chars() {
        // Stop at the line's content boundary (do not let a trailing '\n'/'\r' consume column budget).
        if line_start_byte + byte_in_line >= line_end_byte {
            break;
        }
        if utf16_seen >= utf16_col {
            break;
        }
        utf16_seen += ch.len_utf16();
        byte_in_line += ch.len_utf8();
    }
    // Clamp to the line end (a column past end-of-line maps to the line end — never past it).
    let byte = (line_start_byte + byte_in_line).min(line_end_byte);
    Some(byte)
}

/// Convert a BYTE offset to an [`LspRange`]-compatible `(line, utf16_column)` (the inverse of
/// [`line_utf16_to_byte`]), used to build the Format Selection range from the editor's byte-offset
/// selection. `None` on an out-of-range byte offset. The UTF-16 column is computed by walking the line up
/// to the byte offset and summing `ch.len_utf16()` — the SAME UTF-16 discipline as the forward direction
/// so a round-trip is exact for the request the server receives (AC-007).
pub fn byte_to_lsp_position(
    buffer: &TextBuffer,
    byte_offset: usize,
) -> Option<super::lsp_client::LspPosition> {
    let clamped = byte_offset.min(buffer.len_bytes());
    let line = buffer.byte_to_line(clamped)?;
    let line_start_byte = buffer.line_to_byte(line)?;
    let line_text = buffer.slice_to_string(line..line + 1);
    let target_in_line = clamped.saturating_sub(line_start_byte);

    let mut utf16_col = 0usize;
    let mut byte_in_line = 0usize;
    for ch in line_text.chars() {
        if byte_in_line >= target_in_line {
            break;
        }
        byte_in_line += ch.len_utf8();
        utf16_col += ch.len_utf16();
    }
    Some(super::lsp_client::LspPosition {
        line: line as u32,
        character: utf16_col as u32,
    })
}

/// Build the [`LspRange`] for Format Selection from a primary selection BYTE range `(start, end)` against
/// `buffer`. When the selection is empty/collapsed (`start == end`), the range is the CURRENT LINE's range
/// (line start .. line end), matching VS Code's `editor.action.formatSelection` behavior — NOT a silent
/// fallback to whole-document formatting (the MT is explicit on this). Returns `None` only when the buffer
/// cannot resolve the positions (never panics).
pub fn selection_range_for(buffer: &TextBuffer, start: usize, end: usize) -> Option<LspRange> {
    let (s, e) = (start.min(end), start.max(end));
    if s == e {
        // Empty/collapsed selection: format the caret's current line (line start .. line end).
        let line = buffer.byte_to_line(s.min(buffer.len_bytes()))?;
        let line_start = buffer.line_to_byte(line)?;
        // Line end = start of next line minus its newline, or buffer end on the last line.
        let next_line_start = buffer
            .line_to_byte(line + 1)
            .unwrap_or_else(|| buffer.len_bytes());
        let line_text = buffer.slice_to_string(line..line + 1);
        let content_len = line_text.trim_end_matches(['\n', '\r']).len();
        let line_end = (line_start + content_len).min(next_line_start);
        let start_pos = byte_to_lsp_position(buffer, line_start)?;
        let end_pos = byte_to_lsp_position(buffer, line_end)?;
        return Some(LspRange {
            start: start_pos,
            end: end_pos,
        });
    }
    let start_pos = byte_to_lsp_position(buffer, s)?;
    let end_pos = byte_to_lsp_position(buffer, e)?;
    Some(LspRange {
        start: start_pos,
        end: end_pos,
    })
}

/// A descriptor for a menu / context-menu format entry the existing menu builders consume (RISK-007 —
/// expose a helper rather than forking a menu file outside this MT's allowed paths). Carries the stable
/// author_id, the human label, whether the entry is currently ENABLED (from [`formatter_available`] /
/// [`range_formatter_available`]), and the disabled-state tooltip. The builder renders an enabled item
/// (dispatching the format action) or a disabled item (greyed, with the tooltip + AccessKit-disabled
/// node) from this descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatMenuDescriptor {
    pub author_id: &'static str,
    pub label: &'static str,
    pub enabled: bool,
    /// The tooltip shown when the entry is disabled (the no-formatter reason).
    pub disabled_tooltip: &'static str,
}

/// Build the menu descriptors for the EDIT menu + editor body context menu from the current formatter
/// capability (RISK-007: the existing menu builders consume these rather than this MT editing a menu file
/// outside its allowed paths). Returns three descriptors: the EDIT-menu "Format Document", the
/// context-menu "Format Document", and the context-menu "Format Selection". Each reflects the live
/// enabled/disabled state so the menu greys the entry + sets its AccessKit node disabled when no formatter
/// is available (AC-003 / MC-003).
pub fn menu_descriptors(
    lsp: &super::lsp_client::LspClient,
    language_id: &str,
) -> [FormatMenuDescriptor; 3] {
    let doc_enabled = formatter_available(lsp, language_id);
    let sel_enabled = range_formatter_available(lsp, language_id);
    [
        FormatMenuDescriptor {
            author_id: FORMAT_DOCUMENT_MENU_AUTHOR_ID,
            label: "Format Document",
            enabled: doc_enabled,
            disabled_tooltip: NO_FORMATTER_TOOLTIP,
        },
        FormatMenuDescriptor {
            author_id: FORMAT_DOCUMENT_CTX_AUTHOR_ID,
            label: "Format Document",
            enabled: doc_enabled,
            disabled_tooltip: NO_FORMATTER_TOOLTIP,
        },
        FormatMenuDescriptor {
            author_id: FORMAT_SELECTION_CTX_AUTHOR_ID,
            label: "Format Selection",
            enabled: sel_enabled,
            disabled_tooltip: NO_FORMATTER_TOOLTIP,
        },
    ]
}

/// Map an [`LspError`] to the short reason carried by [`FormatOutcome::LspError`] (surfaced as a
/// non-blocking toast — AC-006). Kept here so the panel and tests share one phrasing.
pub fn lsp_error_reason(err: &LspError) -> String {
    format!("Formatting failed: {err}")
}

/// Resolve a delivered format `edits` array against the pre-format `before` text into
/// `(formatted_text_if_changed, FormatOutcome)` — the SINGLE format-apply core the live panel pump funnels
/// through (`CodeEditorPanel::pump_formatting` -> spawned task -> this -> `drain_format_result`). It takes
/// only `(&str, &[LspTextEdit])` so the spawned off-thread task can call it without borrowing the panel
/// (no `&self` captured across `.await`), and so it is a pure, independently-testable transform:
///
/// - An empty edit list -> `(None, NoChange)` (no buffer touch, no undo entry).
/// - A successful apply that did NOT change the text -> `(None, NoChange)`.
/// - A successful apply that changed the text -> `(Some(after), Applied { edit_count })`. The UI-thread
///   drain installs `after` via `set_text` (which re-clamps the cursor to the new length — RISK-006) and
///   records ONE undo entry (before -> after) so a single Ctrl+Z reverts the WHOLE format (AC-001).
/// - A resolve/apply failure -> `(None, LspError)` (the buffer is left untouched; never a partial
///   corruption — RISK-001).
///
/// This is the ONE applier on the live path: there is no parallel synchronous applier. The descending-offset
/// + UTF-16-correct apply lives in [`apply_text_edits`] / [`apply_text_edits_to_string`], which this calls.
pub fn resolve_format_outcome(
    before: &str,
    edits: &[LspTextEdit],
) -> (Option<String>, FormatOutcome) {
    if edits.is_empty() {
        return (None, FormatOutcome::NoChange);
    }
    match apply_text_edits_to_string(before, edits) {
        Ok(after) if after == before => (None, FormatOutcome::NoChange),
        Ok(after) => (
            Some(after),
            FormatOutcome::Applied {
                edit_count: edits.len(),
            },
        ),
        Err(e) => (
            None,
            FormatOutcome::LspError(format!("Formatting failed: {e}")),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_editor::lsp_client::{LspPosition, LspRange, LspTextEdit};

    fn edit(sl: u32, sc: u32, el: u32, ec: u32, new_text: &str) -> LspTextEdit {
        LspTextEdit {
            range: LspRange {
                start: LspPosition {
                    line: sl,
                    character: sc,
                },
                end: LspPosition {
                    line: el,
                    character: ec,
                },
            },
            new_text: new_text.to_owned(),
        }
    }

    // ── AC-005 / RISK-001 / MC-004: descending-offset apply (ascending would corrupt) ──────────────
    #[test]
    fn format_descending_order_apply() {
        // Two edits on the SAME line whose naive ascending application corrupts the offsets: replace
        // "a" (col 0..1) with "AAAA" (lengthens) and "b" (col 2..3) with "BBBB". Ascending would apply
        // col 0..1 first, shifting col 2..3 to the wrong text.
        let text = "a b\n";
        let edits = vec![edit(0, 0, 0, 1, "AAAA"), edit(0, 2, 0, 3, "BBBB")];
        let out = apply_text_edits_to_string(text, &edits).expect("applies cleanly");
        assert_eq!(
            out, "AAAA BBBB\n",
            "AC-005: descending apply yields the correct text"
        );

        // Regression: a naive ascending apply against the ORIGINAL offsets mangles the text.
        let mut naive = text.to_owned();
        naive.replace_range(0..1, "AAAA"); // col 0..1 first (lengthens)
        let len = naive.len();
        let (s, e) = (2.min(len), 3.min(len)); // col 2..3 against the now-longer string -> wrong span
        naive.replace_range(s..e, "BBBB");
        assert_ne!(
            naive, out,
            "RISK-001: ascending-order apply must NOT equal the correct result"
        );
    }

    // ── AC-007 / RISK-003 / MC-005: UTF-16 column conversion (byte offset != UTF-16 column) ────────
    #[test]
    fn format_utf16_columns() {
        // A line beginning with an emoji "😀" (U+1F600): 4 UTF-8 bytes, but 2 UTF-16 code units. So the
        // identifier "x" that visually follows it sits at UTF-16 column 2 but BYTE offset 4. An edit that
        // targets "x" via its UTF-16 range (col 2..3) MUST land at byte 4..5, not byte 2..3.
        let text = "😀x = 1\n";
        // Replace "x" (UTF-16 col 2..3) with "y".
        let edits = vec![edit(0, 2, 0, 3, "y")];
        let out = apply_text_edits_to_string(text, &edits).expect("applies cleanly");
        assert_eq!(
            out, "😀y = 1\n",
            "AC-007: the edit landed at the correct UTF-16 column, not the byte offset"
        );

        // Prove the BYTE-based (wrong) interpretation would have landed elsewhere: col 2..3 as bytes is
        // INSIDE the emoji's 4-byte sequence, which is not even a char boundary — a byte-based applier
        // would corrupt or reject. Confirm our UTF-16 conversion maps col 2 -> byte 4.
        let buffer = TextBuffer::new(text);
        let br = lsp_range_to_byte_range(&buffer, edits[0].range).expect("range resolves");
        assert_eq!(
            br,
            4..5,
            "AC-007: UTF-16 col 2..3 maps to byte 4..5 (after the 4-byte emoji)"
        );
    }

    // ── round-trip: byte_to_lsp_position is the inverse of line_utf16_to_byte ──────────────────────
    #[test]
    fn utf16_position_round_trips() {
        let text = "fn 名前() {}\n  return;\n"; // a CJK identifier (each kanji = 1 UTF-16 unit, 3 bytes)
        let buffer = TextBuffer::new(text);
        // Walk CHAR-BOUNDARY byte offsets only (a mid-kanji byte is not a valid position to round-trip).
        // Boundaries: 0,1,2,3 (start 名), 6 (start 前), 9 (')'), and into line 1.
        for byte in [0usize, 3, 6, 9, 13, 15] {
            if let Some(pos) = byte_to_lsp_position(&buffer, byte) {
                let back = lsp_range_to_byte_range(
                    &buffer,
                    LspRange {
                        start: pos,
                        end: pos,
                    },
                )
                .map(|r| r.start);
                assert_eq!(
                    back,
                    Some(byte.min(buffer.len_bytes())),
                    "byte {byte} round-trips via UTF-16"
                );
            }
        }
    }

    // ── empty selection -> current line range (NOT whole document) ─────────────────────────────────
    #[test]
    fn empty_selection_formats_current_line() {
        let text = "line0\nline1xyz\nline2\n";
        let buffer = TextBuffer::new(text);
        // A collapsed caret somewhere on line 1 (byte 8, inside "line1xyz").
        let caret = buffer.line_to_byte(1).unwrap() + 2;
        let range = selection_range_for(&buffer, caret, caret).expect("range");
        assert_eq!(range.start.line, 1, "empty selection -> current line start");
        assert_eq!(range.start.character, 0, "line start column 0");
        assert_eq!(
            range.end.line, 1,
            "empty selection -> current line end (same line)"
        );
        assert_eq!(
            range.end.character,
            "line1xyz".len() as u32,
            "line end = content length"
        );
    }

    // ── non-empty selection -> exact selected range ────────────────────────────────────────────────
    #[test]
    fn non_empty_selection_uses_exact_range() {
        let text = "abcdef\n";
        let buffer = TextBuffer::new(text);
        let range = selection_range_for(&buffer, 1, 4).expect("range"); // select "bcd"
        assert_eq!(
            range.start,
            LspPosition {
                line: 0,
                character: 1
            }
        );
        assert_eq!(
            range.end,
            LspPosition {
                line: 0,
                character: 4
            }
        );
    }

    // ── AC-002: text outside the applied edits is byte-for-byte unchanged ──────────────────────────
    #[test]
    fn edits_leave_surrounding_text_unchanged() {
        let text = "keep0\nEDIT_ME\nkeep2\n";
        // One edit replacing only "EDIT_ME" on line 1.
        let edits = vec![edit(1, 0, 1, 7, "done")];
        let out = apply_text_edits_to_string(text, &edits).unwrap();
        assert_eq!(
            out, "keep0\ndone\nkeep2\n",
            "AC-002: only the edited range changed"
        );
    }

    // ── FormatOutcome typing ───────────────────────────────────────────────────────────────────────
    #[test]
    fn outcome_changed_predicate() {
        assert!(FormatOutcome::Applied { edit_count: 3 }.changed());
        assert!(!FormatOutcome::Applied { edit_count: 0 }.changed());
        assert!(!FormatOutcome::NoChange.changed());
        assert!(!FormatOutcome::NoFormatter.changed());
        assert!(!FormatOutcome::LspError("x".into()).changed());
    }

    // ── menu descriptors reflect disabled state with the contract tooltip ──────────────────────────
    #[test]
    fn menu_descriptors_disabled_when_no_formatter() {
        let lsp = crate::code_editor::lsp_client::LspClient::disabled();
        let descs = menu_descriptors(&lsp, "rust");
        for d in &descs {
            assert!(!d.enabled, "no LSP -> every format menu entry disabled");
            assert_eq!(
                d.disabled_tooltip, NO_FORMATTER_TOOLTIP,
                "AC-003: contract tooltip text"
            );
        }
        assert_eq!(descs[0].author_id, FORMAT_DOCUMENT_MENU_AUTHOR_ID);
        assert_eq!(descs[1].author_id, FORMAT_DOCUMENT_CTX_AUTHOR_ID);
        assert_eq!(descs[2].author_id, FORMAT_SELECTION_CTX_AUTHOR_ID);
    }

    // ── formatter_available is false with no LSP (AC-003) ──────────────────────────────────────────
    #[test]
    fn formatter_unavailable_with_no_lsp() {
        let lsp = crate::code_editor::lsp_client::LspClient::disabled();
        assert!(
            !formatter_available(&lsp, "rust"),
            "AC-003: no LSP -> no formatter"
        );
        assert!(
            !range_formatter_available(&lsp, "rust"),
            "AC-003: no LSP -> no range formatter"
        );
        // An empty language id is never a formatter (defensive).
        assert!(
            !formatter_available(&lsp, ""),
            "empty language id -> no formatter"
        );
    }

    // ── resolve_format_outcome: the SINGLE live applier (the panel pump funnels through this) ───────
    // This is the one format-apply core; the live path is pump_formatting -> off-thread
    // LspClient::format_document -> resolve_format_outcome -> drain_format_result. Drive every arm here so
    // the live applier has direct unit coverage (no parallel synchronous applier exists).
    #[test]
    fn resolve_format_outcome_empty_edits_is_nochange() {
        // An empty edit array (server returned no edits) -> NoChange, no formatted text, no undo entry.
        let (formatted, outcome) = resolve_format_outcome("a = 1\n", &[]);
        assert_eq!(formatted, None, "empty edits install no text");
        assert_eq!(outcome, FormatOutcome::NoChange);
    }

    #[test]
    fn resolve_format_outcome_applied_multi_edit() {
        // Two edits on one line whose ASCENDING application would corrupt offsets: descending-apply via
        // the single applier yields the correct text AND the typed Applied { edit_count } the drain installs
        // as ONE undo step (AC-001 / AC-005). `before` is the undo source; `after` is what set_text installs.
        let before = "a b\n";
        let edits = vec![edit(0, 0, 0, 1, "AAAA"), edit(0, 2, 0, 3, "BBBB")];
        let (formatted, outcome) = resolve_format_outcome(before, &edits);
        assert_eq!(
            formatted.as_deref(),
            Some("AAAA BBBB\n"),
            "the single applier returns the formatted text the drain installs (descending, UTF-16-correct)"
        );
        assert_eq!(
            outcome,
            FormatOutcome::Applied { edit_count: 2 },
            "AC-001: a changed format reports Applied with the edit count (drain records ONE undo entry)"
        );
    }

    #[test]
    fn resolve_format_outcome_idempotent_is_nochange() {
        // A server edit that reproduces the same text (already formatted) -> NoChange, no text install, no
        // undo entry (the drain stays silent). Proves the after==before short-circuit on the live applier.
        let before = "x\n";
        let edits = vec![edit(0, 0, 0, 1, "x")]; // replace "x" with "x" — net no-op
        let (formatted, outcome) = resolve_format_outcome(before, &edits);
        assert_eq!(formatted, None, "an idempotent format installs no text");
        assert_eq!(outcome, FormatOutcome::NoChange);
    }

    #[test]
    fn resolve_format_outcome_bad_range_is_lsperror() {
        // A garbled edit range (line far past the buffer) cannot resolve to a byte range -> typed LspError,
        // never a panic and never a partial buffer corruption (RISK-001 / AC-006 / MC-006).
        let before = "one line\n";
        let edits = vec![edit(99, 0, 99, 3, "boom")];
        let (formatted, outcome) = resolve_format_outcome(before, &edits);
        assert_eq!(
            formatted, None,
            "a bad range installs no text (no partial corruption)"
        );
        assert!(
            matches!(outcome, FormatOutcome::LspError(_)),
            "AC-006: an unresolvable edit range is a typed LspError, not a panic"
        );
    }
}
