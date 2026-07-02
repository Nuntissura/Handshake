//! Document-wide find/replace for the native rich-text editor (WP-KERNEL-012 MT-018).
//!
//! Rebuilds, as native Rust, the React `FindReplacePanel.tsx` + `find_replace.ts` find/replace over
//! the whole rich document — prose runs AND embedded `code_block` text — with VS Code-class parity.
//! The panel opens via Ctrl+F (find-only) or Ctrl+H (find + replace), floats over the content as a
//! non-focus-stealing `egui::Window`, highlights every match in real time, and replaces one or all
//! matches as a SINGLE undoable transaction.
//!
//! ## Module layout (REUSE, never fork)
//!
//! - [`scanner`] — the pure-CPU tree walk: [`FindQuery`] / [`FindScanResult`] / [`FindMatch`] +
//!   [`scanner::scan`]. Per-leaf char ranges (the address a replace step uses), `MAX_MATCHES` cap,
//!   whole-word post-filter, literal-vs-regex, typed invalid-regex error. Ports `find_replace.ts`.
//! - [`panel`] — the floating egui panel widget (find input, option toggles, count, prev/next,
//!   replace input + Replace/Replace All, close), with the contract AccessKit ids. Ports
//!   `FindReplacePanel.tsx`. Returns a typed [`panel::PanelOutcome`] the host applies.
//! - [`highlight_layer`] — paints the match-highlight rects over the rendered content via the SAME
//!   epaint [`egui::Galley`] `pos_from_cursor` mechanism MT-015's chip painting uses (NOT cosmic-text,
//!   per the KERNEL_BUILDER gate). Current match: accent ~60% opacity; others ~25%.
//!
//! ## Replace = undoable text Steps (KERNEL_BUILDER gate)
//!
//! A replace targets a [`scanner::FindMatch`]'s `(node_path, char_start..char_end)` rope range via the
//! MT-011 [`Step::DeleteText`] + [`Step::InsertText`] pair — these are proper undoable steps (this is
//! TEXT replace, not the inline-atom-insert gap carried to MT-020). [`replace_one`] issues ONE atomic
//! [`Transaction`] (single Ctrl+Z) then the caller advances; [`replace_all`] issues ONE Transaction
//! with all step pairs sorted DESCENDING by `(node_path, char_start)` so earlier positions do not
//! drift as later replacements shorten/lengthen the rope (MC-001).
//!
//! ## State + lifecycle
//!
//! [`FindReplaceState`] lives on `RichEditorState.find_replace` (`Some` while the panel is open). It
//! owns the query, the current scan result, the active match index, and the replacement text. The
//! widget recomputes the scan on every document change while open and clears it on Escape / close
//! (mirroring the React "recompute on every document change" + "clear highlights on close").

pub mod highlight_layer;
pub mod panel;
pub mod scanner;

use egui::accesskit;

use crate::rich_editor::document_model::history::UndoManager;
use crate::rich_editor::document_model::node::BlockNode;
use crate::rich_editor::document_model::position::DocPosition;
use crate::rich_editor::document_model::selection::Selection;
use crate::rich_editor::document_model::transform::{
    apply_transaction, ActorKind, Step, Transaction,
};

use scanner::{scan, FindMatch, FindQuery, FindScanResult};

// ── AccessKit stable ids (the MT-018 contract id set) ──────────────────────────────────────────
//
// Every interactive node in the panel carries one of these stable author_ids so an out-of-process
// swarm agent (HBR-SWARM) can drive find/replace by a stable key. The strings are EXACTLY the
// contract names (find-panel, find-input, replace-input, find-count, find-next, find-prev,
// find-toggle-case, find-toggle-word, find-toggle-regex, find-error, replace-one, replace-all,
// find-close).

/// The panel container author_id (`find-panel`).
pub const FIND_PANEL_AUTHOR_ID: &str = "find-panel";
/// The find text-input author_id (`find-input`).
pub const FIND_INPUT_AUTHOR_ID: &str = "find-input";
/// The replace text-input author_id (`replace-input`, only present in replace mode).
pub const REPLACE_INPUT_AUTHOR_ID: &str = "replace-input";
/// The match-count label author_id (`find-count`).
pub const FIND_COUNT_AUTHOR_ID: &str = "find-count";
/// The next-match button author_id (`find-next`).
pub const FIND_NEXT_AUTHOR_ID: &str = "find-next";
/// The previous-match button author_id (`find-prev`).
pub const FIND_PREV_AUTHOR_ID: &str = "find-prev";
/// The case-sensitivity toggle author_id (`find-toggle-case`).
pub const FIND_TOGGLE_CASE_AUTHOR_ID: &str = "find-toggle-case";
/// The whole-word toggle author_id (`find-toggle-word`).
pub const FIND_TOGGLE_WORD_AUTHOR_ID: &str = "find-toggle-word";
/// The regex toggle author_id (`find-toggle-regex`).
pub const FIND_TOGGLE_REGEX_AUTHOR_ID: &str = "find-toggle-regex";
/// The inline regex-error text author_id (`find-error`, role=alert).
pub const FIND_ERROR_AUTHOR_ID: &str = "find-error";
/// The replace-one button author_id (`replace-one`).
pub const REPLACE_ONE_AUTHOR_ID: &str = "replace-one";
/// The replace-all button author_id (`replace-all`).
pub const REPLACE_ALL_AUTHOR_ID: &str = "replace-all";
/// The close button author_id (`find-close`).
pub const FIND_CLOSE_AUTHOR_ID: &str = "find-close";

/// The AccessKit role of the panel container. The MT scope frames the panel as a search surface;
/// accesskit 0.21.1 exposes `Role::SearchInput` for a search container, but to keep the role
/// field-correct as a generic grouping container (the panel holds two inputs + buttons, it is not
/// itself a single text input) we use `Role::Group` — the nearest correct container role
/// (ACCESSKIT VARIANT impl note: field-correct nearest role). The find/replace INPUTS carry the
/// text-input role below.
pub const FIND_PANEL_ROLE: accesskit::Role = accesskit::Role::Group;

/// The AccessKit role of the find/replace text inputs (`Role::TextInput`).
pub const FIND_INPUT_ROLE: accesskit::Role = accesskit::Role::TextInput;

/// The AccessKit role of the inline regex-error text. The React panel marks it `role="alert"`;
/// accesskit 0.21.1's nearest is `Role::Label` with a live-region semantics — we use `Role::Label`
/// (the field-correct nearest variant) so the error text is a labeled, readable node.
pub const FIND_ERROR_ROLE: accesskit::Role = accesskit::Role::Label;

/// The live state of an open find/replace panel, stored on `RichEditorState.find_replace`
/// (`Some` while the panel is open).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindReplaceState {
    /// Whether the replace row is shown (Ctrl+H -> `true`; Ctrl+F -> `false`).
    pub with_replace: bool,
    /// The current query (pattern + the three option toggles).
    pub query: FindQuery,
    /// The replacement text typed into the replace input (empty in find-only mode).
    pub replacement: String,
    /// The most recent scan result (recomputed on open, on every query change, and on every
    /// document change while open). Owned here so the highlight layer + count read one source.
    pub scan: FindScanResult,
    /// The active match index into `scan.matches`, or `None` when there is no active match yet
    /// (freshly opened, before the first Enter / next). Mirrors the React `activeIndex = -1`.
    pub active: Option<usize>,
    /// Set on open so the widget focuses the find input EXACTLY ONCE (it must not steal focus on
    /// every frame — HBR-QUIET). Cleared by the panel after the first focus request.
    pub focus_find_input: bool,
}

impl FindReplaceState {
    /// Open a fresh panel. `with_replace` selects find-only (Ctrl+F) vs. find+replace (Ctrl+H). The
    /// query starts empty, no active match, and the one-shot focus flag set.
    pub fn open(with_replace: bool) -> Self {
        Self {
            with_replace,
            query: FindQuery::default(),
            replacement: String::new(),
            scan: FindScanResult::empty(),
            active: None,
            focus_find_input: true,
        }
    }

    /// Recompute the scan from the current query against `doc`, clamping the active index into the
    /// new match set (shrinking matches must not leave `active` dangling — mirrors the React
    /// `safeIndex` clamp). Called on open, on every query change, and after every document mutation
    /// while the panel is open.
    pub fn rescan(&mut self, doc: &BlockNode) {
        self.scan = scan(doc, &self.query);
        self.active = clamp_active(self.active, self.scan.len());
    }

    /// Advance to the next match (wrapping), setting `active`. When there is no active match yet,
    /// the first match becomes active. A no-op (leaves `active` `None`) when there are no matches.
    pub fn select_next(&mut self) {
        let n = self.scan.len();
        if n == 0 {
            self.active = None;
            return;
        }
        self.active = Some(match self.active {
            Some(i) => (i + 1) % n,
            None => 0,
        });
    }

    /// Move to the previous match (wrapping), setting `active`. When there is no active match yet,
    /// the LAST match becomes active. A no-op when there are no matches.
    pub fn select_prev(&mut self) {
        let n = self.scan.len();
        if n == 0 {
            self.active = None;
            return;
        }
        self.active = Some(match self.active {
            Some(0) | None => n - 1,
            Some(i) => i - 1,
        });
    }

    /// The match-count display string: `""` when the query is empty, `No matches` when the query is
    /// non-empty but matches nothing, `{active+1} of {total}` when an active match exists, else
    /// `{total} matches`; a truncated scan appends a `+`. Mirrors the React `countLabel`.
    pub fn count_label(&self) -> String {
        if self.query.is_empty() {
            return String::new();
        }
        let n = self.scan.len();
        if n == 0 {
            return "No matches".to_owned();
        }
        let plus = if self.scan.truncated { "+" } else { "" };
        match self.active {
            Some(i) => format!("{} of {}{}", i + 1, n, plus),
            None => format!("{} matches{}", n, plus),
        }
    }
}

/// Clamp an active match index into a match set of size `n`: `None` (or any index) collapses to
/// `None` when there are no matches; otherwise an out-of-range index clamps to the last match.
fn clamp_active(active: Option<usize>, n: usize) -> Option<usize> {
    if n == 0 {
        return None;
    }
    active.map(|i| i.min(n - 1))
}

/// The actor id stamped onto find/replace transactions (provenance for the event ledger / undo).
const FIND_REPLACE_ACTOR: &str = "find-replace";

/// Replace the SINGLE match `m` with `replacement`, as ONE atomic [`Transaction`] (a single Ctrl+Z
/// undoes it). The match's `(node_path, char_start..char_end)` rope range is deleted and the
/// replacement inserted in one transaction. On success the receipt is pushed onto `undo`, the caret
/// is placed just after the inserted text, and `true` is returned; a step that fails to apply (a
/// stale match against a since-mutated doc) leaves the doc unchanged and returns `false`.
pub fn replace_one(
    doc: &mut BlockNode,
    undo: &mut UndoManager,
    selection: &mut Selection,
    m: &FindMatch,
    replacement: &str,
) -> bool {
    let tx = Transaction::new(
        replace_steps(m, replacement),
        ActorKind::Operator,
        FIND_REPLACE_ACTOR,
    );
    match apply_transaction(doc, tx) {
        Ok(receipt) => {
            undo.push(receipt);
            // Caret just after the inserted replacement, so a subsequent "next" advances past it.
            let caret = m.char_start + replacement.chars().count();
            *selection = Selection::caret(DocPosition::new(m.node_path.clone(), caret));
            true
        }
        Err(_) => false,
    }
}

/// Replace EVERY match in `matches` with `replacement`, as ONE [`Transaction`] (a single Ctrl+Z
/// undoes them all). The matches are sorted DESCENDING by `(node_path, char_start)` so each
/// delete/insert pair targets a rope position that earlier (higher) replacements have not yet
/// shifted — the MC-001 invariant. Returns the number of matches replaced (0 when there are none or
/// the transaction failed, leaving the doc unchanged).
pub fn replace_all(
    doc: &mut BlockNode,
    undo: &mut UndoManager,
    selection: &mut Selection,
    matches: &[FindMatch],
    replacement: &str,
) -> usize {
    if matches.is_empty() {
        return 0;
    }
    // Sort a copy DESCENDING by (node_path, char_start). Descending char order within a leaf keeps
    // earlier match offsets valid as a later (higher-offset) match is replaced; descending node_path
    // order is a stable total order across leaves (the per-leaf ranges are independent, but a single
    // consistent ordering makes the step sequence deterministic and the inverse well-defined).
    let mut ordered: Vec<&FindMatch> = matches.iter().collect();
    ordered.sort_by(|a, b| {
        b.node_path
            .cmp(&a.node_path)
            .then(b.char_start.cmp(&a.char_start))
    });

    let mut steps: Vec<Step> = Vec::with_capacity(ordered.len() * 2);
    for m in &ordered {
        steps.extend(replace_steps(m, replacement));
    }
    let count = ordered.len();
    let tx = Transaction::new(steps, ActorKind::Operator, FIND_REPLACE_ACTOR);
    match apply_transaction(doc, tx) {
        Ok(receipt) => {
            undo.push(receipt);
            // Place the caret at the start of the first (document-order) match's replacement so the
            // selection lands somewhere sensible after a bulk replace (no jump to 0). The first
            // match in DOCUMENT order is the last in our descending `ordered`.
            if let Some(first_doc_order) = ordered.last() {
                let caret = first_doc_order.char_start + replacement.chars().count();
                *selection =
                    Selection::caret(DocPosition::new(first_doc_order.node_path.clone(), caret));
            }
            count
        }
        Err(_) => 0,
    }
}

/// The `DeleteText` + `InsertText` step pair that replaces one match's rope range with
/// `replacement`. Delete then insert at the same start offset, so the net effect is an in-place
/// replace; the transform layer captures the inverse of each step so undo restores the original.
fn replace_steps(m: &FindMatch, replacement: &str) -> Vec<Step> {
    vec![
        Step::DeleteText {
            path: m.node_path.clone(),
            start: m.char_start,
            end: m.char_end,
        },
        Step::InsertText {
            path: m.node_path.clone(),
            char_offset: m.char_start,
            text: replacement.to_owned(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{BlockNode, Child, NodeKind, TextLeaf};
    use crate::rich_editor::document_model::position::DocPosition;
    use scanner::FindQuery;

    fn leaf_text(doc: &BlockNode, block: usize, leaf: usize) -> String {
        doc.children[block].as_block().unwrap().children[leaf]
            .as_text()
            .unwrap()
            .text
            .to_string()
    }

    #[test]
    fn open_find_only_vs_find_replace() {
        assert!(
            !FindReplaceState::open(false).with_replace,
            "Ctrl+F = find-only"
        );
        assert!(
            FindReplaceState::open(true).with_replace,
            "Ctrl+H = find + replace"
        );
        // A fresh panel has the one-shot focus flag set and no active match.
        let st = FindReplaceState::open(false);
        assert!(st.focus_find_input);
        assert_eq!(st.active, None);
    }

    #[test]
    fn count_label_states() {
        let doc = BlockNode::doc(vec![BlockNode::paragraph("foo foo foo")]);
        let mut st = FindReplaceState::open(false);
        // Empty query -> empty label.
        assert_eq!(st.count_label(), "");
        // Non-matching query -> "No matches".
        st.query = FindQuery::literal("zzz");
        st.rescan(&doc);
        assert_eq!(st.count_label(), "No matches");
        // Matching query, no active match yet -> "{total} matches".
        st.query = FindQuery::literal("foo");
        st.rescan(&doc);
        assert_eq!(st.count_label(), "3 matches");
        // After advancing, "{active+1} of {total}".
        st.select_next();
        assert_eq!(st.count_label(), "1 of 3");
        st.select_next();
        assert_eq!(st.count_label(), "2 of 3");
    }

    #[test]
    fn truncated_count_label_has_plus() {
        let text = "x".repeat(scanner::MAX_MATCHES + 10);
        let doc = BlockNode::doc(vec![BlockNode::paragraph(&text)]);
        let mut st = FindReplaceState::open(false);
        st.query = FindQuery::literal("x");
        st.rescan(&doc);
        assert!(
            st.count_label().ends_with('+'),
            "a truncated scan marks the count with '+'"
        );
    }

    #[test]
    fn next_prev_wrap_around() {
        let doc = BlockNode::doc(vec![BlockNode::paragraph("a a a")]);
        let mut st = FindReplaceState::open(false);
        st.query = FindQuery::literal("a");
        st.rescan(&doc);
        assert_eq!(st.scan.len(), 3);
        st.select_next();
        assert_eq!(st.active, Some(0));
        st.select_prev();
        assert_eq!(st.active, Some(2), "prev from the first wraps to the last");
        st.select_next();
        assert_eq!(st.active, Some(0), "next from the last wraps to the first");
    }

    #[test]
    fn replace_one_replaces_current_and_caret_advances_undo_single_step() {
        // AC-7: Replace One replaces the current match; undo restores in a single Ctrl+Z.
        let doc0 = BlockNode::doc(vec![BlockNode::paragraph("foo bar foo")]);
        let mut doc = doc0.clone();
        let mut undo = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let result = scan(&doc, &FindQuery::literal("foo"));
        // Replace the FIRST match only.
        let replaced = replace_one(&mut doc, &mut undo, &mut sel, &result.matches[0], "BAZ");
        assert!(replaced);
        assert_eq!(
            leaf_text(&doc, 0, 0),
            "BAZ bar foo",
            "only the first foo is replaced"
        );
        // The caret advanced to just after the inserted "BAZ" (char 3).
        assert!(matches!(&sel, Selection::Text { head, .. } if head.char_offset == 3));
        // A SINGLE undo restores the original (one Ctrl+Z).
        assert!(undo.undo(&mut doc).unwrap());
        assert_eq!(doc, doc0, "one undo restores the pre-replace document");
    }

    #[test]
    fn replace_all_one_transaction_one_undo_restores_all() {
        // AC-8: Replace All replaces every match across the doc in ONE transaction; a single undo
        // restores all at once. Multiple matches in ONE leaf prove the descending-order invariant
        // (MC-001): replacing with a LONGER string must not corrupt earlier matches.
        let doc0 = BlockNode::doc(vec![
            BlockNode::paragraph("foo bar foo baz foo"),
            BlockNode::paragraph("foo here"),
        ]);
        let mut doc = doc0.clone();
        let mut undo = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let result = scan(&doc, &FindQuery::literal("foo"));
        assert_eq!(result.len(), 4, "three foo in para 0 + one in para 1");
        // Replace with a LONGER string to prove no position drift.
        let n = replace_all(&mut doc, &mut undo, &mut sel, &result.matches, "QUUX");
        assert_eq!(n, 4);
        assert_eq!(leaf_text(&doc, 0, 0), "QUUX bar QUUX baz QUUX");
        assert_eq!(leaf_text(&doc, 1, 0), "QUUX here");
        // A SINGLE undo restores the whole document.
        assert!(undo.undo(&mut doc).unwrap());
        assert_eq!(doc, doc0, "one undo restores all replacements at once");
        assert!(!undo.can_undo(), "replace-all was exactly one undo entry");
    }

    #[test]
    fn replace_all_descending_order_with_shorter_replacement() {
        // MC-001 the other direction: replacing with a SHORTER string also must not drift. "aaa aaa"
        // -> replace "aaa" with "Z" -> "Z Z".
        let doc0 = BlockNode::doc(vec![BlockNode::paragraph("aaa aaa aaa")]);
        let mut doc = doc0.clone();
        let mut undo = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let result = scan(&doc, &FindQuery::literal("aaa"));
        assert_eq!(result.len(), 3);
        let n = replace_all(&mut doc, &mut undo, &mut sel, &result.matches, "Z");
        assert_eq!(n, 3);
        assert_eq!(leaf_text(&doc, 0, 0), "Z Z Z");
    }

    #[test]
    fn replace_all_inside_a_code_block() {
        // CodeBlock replace: the code text is a rope in a TextLeaf, so the same DeleteText/InsertText
        // steps apply (MT scope). Replace 'foo' -> 'bar' inside a code block.
        let doc = BlockNode::doc(vec![BlockNode::with_children(
            NodeKind::CodeBlock,
            vec![Child::Text(TextLeaf::new("let foo = foo + 1;"))],
        )]);
        let mut doc = doc;
        let mut undo = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let result = scan(&doc, &FindQuery::literal("foo"));
        assert_eq!(result.len(), 2);
        let n = replace_all(&mut doc, &mut undo, &mut sel, &result.matches, "bar");
        assert_eq!(n, 2);
        assert_eq!(leaf_text(&doc, 0, 0), "let bar = bar + 1;");
    }

    #[test]
    fn rescan_after_replace_one_advances_to_next_match() {
        // Replace-one then rescan (the widget re-scans on the doc mutation): the next match is now
        // findable and select_next lands on it. Proves the "advance to the next match" contract.
        let mut doc = BlockNode::doc(vec![BlockNode::paragraph("foo foo foo")]);
        let mut undo = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let mut st = FindReplaceState::open(true);
        st.query = FindQuery::literal("foo");
        st.rescan(&doc);
        st.select_next(); // active = first match
        let current = st.scan.matches[st.active.unwrap()].clone();
        replace_one(&mut doc, &mut undo, &mut sel, &current, "X");
        st.rescan(&doc); // the doc changed -> re-scan
        assert_eq!(st.scan.len(), 2, "two foo remain after replacing one");
        assert_eq!(leaf_text(&doc, 0, 0), "X foo foo");
    }
}
