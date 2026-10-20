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

use handshake_text_mechanics::formatting as mechanics;
pub use mechanics::FormatApplyError;

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
    mechanics::apply_text_edits(buffer, &mechanics_edits(edits))
}

/// Apply formatter edits through the canonical text mechanics.
pub fn apply_text_edits_to_string(
    text: &str,
    edits: &[LspTextEdit],
) -> Result<String, FormatApplyError> {
    mechanics::apply_text_edits_to_string(text, &mechanics_edits(edits))
}

/// Resolve a host LSP range against the canonical buffer.
pub fn lsp_range_to_byte_range(buffer: &TextBuffer, range: LspRange) -> Option<Range<usize>> {
    mechanics::lsp_range_to_byte_range(buffer, mechanics_range(range))
}

/// Convert a buffer byte offset to the host LSP position DTO.
pub fn byte_to_lsp_position(
    buffer: &TextBuffer,
    byte_offset: usize,
) -> Option<super::lsp_client::LspPosition> {
    mechanics::byte_to_lsp_position(buffer, byte_offset).map(host_position)
}

/// Resolve the formatter selection range, including the collapsed-current-line behavior.
pub fn selection_range_for(buffer: &TextBuffer, start: usize, end: usize) -> Option<LspRange> {
    mechanics::selection_range_for(buffer, start, end).map(|range| LspRange {
        start: host_position(range.start),
        end: host_position(range.end),
    })
}

fn mechanics_range(range: LspRange) -> mechanics::LspRange {
    mechanics::LspRange {
        start: mechanics::LspPosition {
            line: range.start.line,
            character: range.start.character,
        },
        end: mechanics::LspPosition {
            line: range.end.line,
            character: range.end.character,
        },
    }
}

fn host_position(position: mechanics::LspPosition) -> super::lsp_client::LspPosition {
    super::lsp_client::LspPosition {
        line: position.line,
        character: position.character,
    }
}

fn mechanics_edits(edits: &[LspTextEdit]) -> Vec<mechanics::LspTextEdit> {
    edits
        .iter()
        .map(|edit| mechanics::LspTextEdit {
            range: mechanics_range(edit.range),
            new_text: edit.new_text.clone(),
        })
        .collect()
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
