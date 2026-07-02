//! Slash-command menu for the native rich-text editor (WP-KERNEL-012 MT-016).
//!
//! When the operator types `/` at the start of a blank line or after whitespace, an egui
//! popup menu appears listing every insertable block type, embed kind, wikilink action,
//! template, and the advanced manual-insert command (Obsidian / Notion parity). Selecting a
//! command executes it via the MT-013 formatting commands, the MT-014 embed logic, or the
//! MT-015 wikilink autocomplete. This is the primary keyboard-driven block-creation surface.
//!
//! ## Module layout (REUSE, never fork)
//!
//! - [`registry`] — the static `&'static` [`registry::SlashCommand`] catalog + the 2-pass
//!   filter/ranking algorithm. Zero per-frame allocation (MT impl note 1). The catalog is
//!   parity-derived from the React `EDITOR_COMMANDS` categories.
//! - [`menu`] — the egui popup widget: a grouped, keyboard-navigable, filtered list anchored
//!   at the caret pixel, mirroring the WP-011 `command_palette.rs` pattern (the existing
//!   keyboard-driven command surface). Each row is an AccessKit `slash-item-{id}` node.
//! - [`executor`] — dispatches a selected command's [`registry::SlashAction`] to the right
//!   MT-013 (`set_block_kind` / `InsertNode`), MT-014 (embed atom), or MT-015 (wikilink
//!   autocomplete activation) handler, after first removing the `/`+filter trigger text.
//!
//! ## Trigger discipline (red-team RISK-1 / MC-001)
//!
//! The `/` opens the menu ONLY at the start of an empty paragraph or after a WHITESPACE
//! char — never mid-word and never inside a URL/path (`http://`, `src/app.ts`). The
//! [`slash_trigger_fires`] guard inspects the char immediately before the `/` and refuses
//! when it is a non-whitespace URL/path char (a letter, digit, `:`, `.`, `/`, `-`, `_`).
//! A `cargo test` with `http://foo` proves no menu opens.
//!
//! ## Caret-snapshot discipline (red-team RISK-3 / MC-003)
//!
//! The executor computes the delete range of the `/`+filter text from the trigger position
//! and filter length SNAPSHOTTED into [`SlashMenuState`] when the menu opened/refined, NOT
//! from a live (possibly stale) selection — so the delete never removes more than the
//! `/`+filter the operator typed.
//!
//! ## Focus discipline (red-team RISK-4 / MC-004)
//!
//! The widget loop closes the menu when the editor surface loses focus (the host calls
//! [`SlashMenuState`]'s close on `is_focused() == false`), so a click outside the window
//! never strands an open popup blocking other surfaces.

// WP-KERNEL-012 MT-034 (E5 — code<->note cross-refs): the `/code-ref` code-symbol search dialog state
// + off-thread lookup. On select it inserts a `code`-kind hsLink atom (executor::insert_code_ref_atom).
pub mod code_symbol_search;
pub mod executor;
pub mod menu;
pub mod registry;

use egui::accesskit;

use crate::rich_editor::document_model::node::{BlockNode, Child};
use crate::rich_editor::document_model::selection::Selection;

/// The AccessKit author_id of the slash-menu popup container (AC-6 / MT scope: popup =
/// `slash-menu`, Role::Menu). A swarm agent addresses the open menu by this stable key.
pub const SLASH_MENU_AUTHOR_ID: &str = "slash-menu";

/// The author_id PREFIX for each menu item row (`slash-item-{id}`, AC-7 / MT scope:
/// Role::MenuItem). The full id is `{SLASH_ITEM_AUTHOR_ID_PREFIX}{cmd.id}`.
pub const SLASH_ITEM_AUTHOR_ID_PREFIX: &str = "slash-item-";

/// The AccessKit role for the slash-menu popup container. The MT scope names `Role::Menu`;
/// it exists in accesskit 0.21.x and is used verbatim (no fallback needed — the ACCESSKIT
/// VARIANT impl note's `List` fallback is unnecessary because `Menu` is present).
pub const SLASH_MENU_ROLE: accesskit::Role = accesskit::Role::Menu;

/// The AccessKit role for each slash-menu item row (the MT scope's `Role::MenuItem`).
pub const SLASH_ITEM_ROLE: accesskit::Role = accesskit::Role::MenuItem;

/// The author_id of the embed/transclusion/manual-insert PROMPT modal dialog (the
/// `egui::Window` opened by `OpenEmbedPrompt` / `OpenTransclusionPrompt` /
/// `OpenManualInsertPrompt`). A single fixed id covers the one-at-a-time modal.
pub const SLASH_PROMPT_DIALOG_AUTHOR_ID: &str = "slash-prompt-dialog";

/// The author_id of the prompt modal's text input field.
pub const SLASH_PROMPT_INPUT_AUTHOR_ID: &str = "slash-prompt-input";

/// The author_id of the prompt modal's confirm (Ok) button.
pub const SLASH_PROMPT_OK_AUTHOR_ID: &str = "slash-prompt-ok";

/// The author_id of the prompt modal's cancel button.
pub const SLASH_PROMPT_CANCEL_AUTHOR_ID: &str = "slash-prompt-cancel";

/// WP-KERNEL-012 MT-034: the author_id of the code-symbol search dialog (`/code-ref`), Role::Dialog.
pub const CODE_SYMBOL_SEARCH_AUTHOR_ID: &str = "code-symbol-search";

/// WP-KERNEL-012 MT-034: the author_id of the code-symbol search dialog's text input, Role::TextField.
pub const CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID: &str = "code-symbol-search-input";

/// WP-KERNEL-012 MT-034: the author_id PREFIX for one code-symbol search result row
/// (`code-symbol-result-{symbol_entity_id}`), Role::ListItem.
pub const CODE_SYMBOL_RESULT_AUTHOR_ID_PREFIX: &str = "code-symbol-result-";

/// The AccessKit role for the code-symbol search dialog container (the contract's `Role::Dialog`).
pub const CODE_SYMBOL_SEARCH_ROLE: accesskit::Role = accesskit::Role::Dialog;

/// The AccessKit role for the code-symbol search input (the contract's `Role::TextField` — the
/// field-correct accesskit 0.21.1 variant is `TextInput`; both the contract `TextField` name and the
/// real role are reconciled to `TextInput` here, the same documented-deviation pattern MT-003 used).
pub const CODE_SYMBOL_SEARCH_INPUT_ROLE: accesskit::Role = accesskit::Role::TextInput;

/// Build the stable AccessKit author_id for a code-symbol search result row.
pub fn code_symbol_result_author_id(symbol_entity_id: &str) -> String {
    format!("{CODE_SYMBOL_RESULT_AUTHOR_ID_PREFIX}{symbol_entity_id}")
}

/// Build the stable AccessKit author_id for the menu row of command `id`.
pub fn slash_item_author_id(id: &str) -> String {
    format!("{SLASH_ITEM_AUTHOR_ID_PREFIX}{id}")
}

/// The live state of an open slash-command menu, stored on `RichEditorState.slash_menu`
/// (`Some` while the menu is open). Owns the trigger position, the typed filter, the
/// selected row index, and — when a command opened a prompt — the active [`SlashPrompt`].
///
/// The trigger position is captured at OPEN time and never recomputed from a live selection
/// (RISK-3 / MC-003): `trigger_leaf_path` + `trigger_char` address the `/` char, so the
/// executor deletes exactly `[trigger_char, trigger_char + 1 + filter.chars().count())`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashMenuState {
    /// The block path of the text leaf the `/` trigger lives in.
    pub trigger_leaf_path: Vec<usize>,
    /// The CHAR offset of the `/` trigger char within that leaf (the delete-range anchor).
    pub trigger_char: usize,
    /// The filter text typed AFTER the `/` (drives the catalog filter; its char length is
    /// the second half of the delete range — RISK-3).
    pub filter: String,
    /// The highlighted row index into the CURRENTLY filtered list (clamped before use).
    pub selected: usize,
    /// When a selected command opened a prompt modal (embed/transclusion/manual insert),
    /// this carries its state; the menu list is hidden while a prompt is active.
    pub prompt: Option<SlashPrompt>,
}

impl SlashMenuState {
    /// Open a fresh menu for a `/` at `trigger_char` in the leaf at `trigger_leaf_path`,
    /// with an empty filter and the first row selected.
    pub fn open(trigger_leaf_path: Vec<usize>, trigger_char: usize) -> Self {
        Self {
            trigger_leaf_path,
            trigger_char,
            filter: String::new(),
            selected: 0,
            prompt: None,
        }
    }

    /// The char length of the `/`+filter trigger text the executor must delete: 1 (the `/`)
    /// plus the filter's char count. Computed from the SNAPSHOTTED filter (RISK-3 / MC-003),
    /// never a live selection.
    pub fn trigger_delete_len(&self) -> usize {
        1 + self.filter.chars().count()
    }

    /// True when a prompt modal is currently active (the menu list is suppressed).
    pub fn prompt_active(&self) -> bool {
        self.prompt.is_some()
    }
}

/// An active prompt modal opened by a slash command (embed ref-value, transclusion ref, or
/// raw manual node JSON). A single `egui::Window` with one text input + Ok/Cancel, reusing
/// the `context_menu_surfaces.rs` dialog pattern — NOT a file picker (the operator types or
/// pastes the value directly, MT impl note 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashPrompt {
    /// Which prompt this is (drives the title, placeholder, and the confirm action).
    pub kind: SlashPromptKind,
    /// The current text the operator has typed into the modal's input.
    pub input: String,
}

impl SlashPrompt {
    /// Build a fresh empty prompt of `kind`.
    pub fn new(kind: SlashPromptKind) -> Self {
        Self {
            kind,
            input: String::new(),
        }
    }

    /// The modal title for this prompt kind.
    pub fn title(&self) -> &'static str {
        match self.kind {
            SlashPromptKind::Embed(ek) => match ek {
                registry::EmbedKind::Image => "Insert image embed",
                registry::EmbedKind::Slideshow => "Insert slideshow embed",
                registry::EmbedKind::Album => "Insert album embed",
                registry::EmbedKind::Video => "Insert video embed",
            },
            SlashPromptKind::Transclusion => "Insert transclusion",
            SlashPromptKind::ManualInsert => "Manual node insert (raw JSON)",
        }
    }

    /// The input field placeholder/hint for this prompt kind.
    pub fn hint(&self) -> &'static str {
        match self.kind {
            SlashPromptKind::Embed(_) => "asset id (or comma-separated ids)",
            SlashPromptKind::Transclusion => "loom block id",
            SlashPromptKind::ManualInsert => r#"{"type":"paragraph","content":[…]}"#,
        }
    }
}

/// The kind of prompt modal a slash command opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashPromptKind {
    /// Embed a CKC media asset of the given [`registry::EmbedKind`] (MT-014 `hsLink` atom).
    Embed(registry::EmbedKind),
    /// Insert a `loomTransclusion` atom (MT-015) referencing a loom block id.
    Transclusion,
    /// Insert a raw node JSON (advanced; swarm-agent surface).
    ManualInsert,
}

/// True when typing a `/` at char offset `caret_char` inside `leaf` SHOULD open the slash
/// menu (red-team RISK-1 / MC-001). The trigger fires when the `/` is at the very start of
/// the leaf OR the char immediately before it is whitespace; it does NOT fire when the
/// preceding char is a URL/path char (a letter, digit, `:`, `.`, `/`, `-`, `_`), so a `/`
/// typed inside `http://foo` or `src/app.ts` never opens the menu.
///
/// `caret_char` is the offset of the `/` ITSELF (the char position just typed). `leaf_text`
/// is the leaf's text AFTER the `/` was inserted, so the char at `caret_char` is the `/` and
/// `caret_char - 1` is the char before it.
pub fn slash_trigger_fires(leaf_text: &str, caret_char: usize) -> bool {
    // The char immediately before the `/`. If the `/` is at the document/leaf start, there
    // is no preceding char and the trigger fires (start of a blank line).
    if caret_char == 0 {
        return true;
    }
    let prev = leaf_text.chars().nth(caret_char - 1);
    match prev {
        None => true, // defensive: offset past the text -> treat as start.
        Some(c) if c.is_whitespace() => true,
        // A URL/path/word char before the `/` -> do NOT trigger (RISK-1).
        Some(_) => false,
    }
}

/// True when `c` is a char that, when it precedes a `/`, marks the `/` as part of a URL or
/// path rather than a fresh slash-command trigger. Exposed for the unit test that documents
/// the guard's vocabulary; the trigger itself only needs the whitespace check above (any
/// non-whitespace preceding char suppresses the trigger), but this names the intent.
pub fn is_url_or_path_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, ':' | '.' | '/' | '-' | '_')
}

/// Resolve the text leaf addressed by a `Selection`'s caret head, returning the leaf's path
/// and its full text. `None` when the selection is not a text caret resolving to a leaf.
/// Used by the widget to detect a `/` trigger and to refresh the filter as the operator
/// continues typing.
pub fn caret_leaf_text(doc: &BlockNode, selection: &Selection) -> Option<(Vec<usize>, String)> {
    let Selection::Text { head, .. } = selection else {
        return None;
    };
    let (leaf_idx, block_path) = head.path.split_last()?;
    let mut node = doc;
    for &idx in block_path {
        node = node.children.get(idx)?.as_block()?;
    }
    let leaf = node.children.get(*leaf_idx)?.as_text()?;
    Some((head.path.clone(), leaf.text.to_string()))
}

/// The caret's in-leaf char offset, or `None` for a non-text selection.
pub fn caret_char_offset(selection: &Selection) -> Option<usize> {
    match selection {
        Selection::Text { head, .. } => Some(head.char_offset),
        Selection::Node { .. } => None,
    }
}

/// Inspect the caret leaf for an OPEN `/` trigger token and return `(trigger_char, filter)`
/// when one is present BEFORE the caret. The trigger is the LAST `/` before the caret that
/// (a) passes [`slash_trigger_fires`] and (b) has no whitespace between it and the caret
/// (typing a space after `/foo` closes the menu — the token ended). `filter` is the text
/// between the `/` and the caret.
///
/// Returns `None` when there is no open trigger (no `/`, the `/` was URL/path-embedded, or a
/// space/newline broke the token), which the widget uses to CLOSE the menu.
pub fn open_slash_trigger(leaf_text: &str, caret_char: usize) -> Option<(usize, String)> {
    let chars: Vec<char> = leaf_text.chars().collect();
    if caret_char == 0 || caret_char > chars.len() {
        return None;
    }
    // Scan backwards from just before the caret for the `/` that opened the token. Stop at
    // the first whitespace (the token cannot span a space) — if we hit whitespace before a
    // `/`, there is no open trigger.
    let mut i = caret_char; // exclusive upper bound of the filter text
    while i > 0 {
        let c = chars[i - 1];
        if c == '/' {
            // Found the candidate trigger at i-1. It must satisfy the trigger guard
            // (start-of-leaf or whitespace-preceded, not URL/path-embedded).
            let trigger_char = i - 1;
            // Build the leaf-prefix string up to and including the `/` so the guard sees the
            // same preceding-char context.
            if slash_trigger_fires(leaf_text, trigger_char) {
                let filter: String = chars[i..caret_char].iter().collect();
                return Some((trigger_char, filter));
            }
            return None; // a URL/path `/` — no menu.
        }
        if c.is_whitespace() {
            return None; // whitespace between caret and any `/` -> token closed.
        }
        i -= 1;
    }
    None
}

/// The outcome of rendering the `/code-ref` code-symbol search dialog one frame (MT-034).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeSymbolSearchOutcome {
    /// Nothing happened (the dialog stays open).
    None,
    /// The operator picked a result: insert a `code` hsLink atom with this `(symbol_entity_id,
    /// display_name)` and close the dialog.
    Selected {
        symbol_entity_id: String,
        display_name: String,
    },
    /// The operator cancelled (Escape / close button); close the dialog without inserting.
    Cancelled,
}

/// Render the `/code-ref` code-symbol search dialog (MT-034). Draws a floating [`egui::Window`] with a
/// search input + a results list, emits the `code-symbol-search` (Dialog), `code-symbol-search-input`
/// (TextField), and `code-symbol-result-{id}` (ListItem) AccessKit nodes, and returns the
/// [`CodeSymbolSearchOutcome`]. Re-runs the off-thread lookup when the query text changes (the caller
/// already drained the delivery cell into `state.results`). The actual atom insert + close is the
/// caller's job, driven by the returned outcome.
pub fn render_code_symbol_search_dialog(
    ctx: &egui::Context,
    state: &mut code_symbol_search::CodeSymbolSearchState,
    palette: &crate::theme::HsPalette,
) -> CodeSymbolSearchOutcome {
    let mut outcome = CodeSymbolSearchOutcome::None;
    let mut open = true;
    let prev_query = state.query.clone();

    egui::Window::new("Insert code reference")
        .id(egui::Id::new(CODE_SYMBOL_SEARCH_AUTHOR_ID))
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ctx, |ui| {
            // The search input.
            let input = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .hint_text("Search code symbols (function, struct, …)")
                    .desired_width(280.0),
            );
            ctx.accesskit_node_builder(input.id, |node| {
                node.set_role(CODE_SYMBOL_SEARCH_INPUT_ROLE);
                node.set_author_id(CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID.to_owned());
                node.set_label("Search code symbols".to_owned());
            });

            ui.separator();

            if state.loading {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    ui.colored_label(palette.text_subtle, "Searching symbols…");
                });
            } else if let Some(err) = &state.error {
                ui.colored_label(palette.error_text, format!("Symbol search failed: {err}"));
            } else if state.query.trim().is_empty() {
                ui.colored_label(palette.text_subtle, "Type to search code symbols.");
            } else if state.results.is_empty() {
                ui.colored_label(palette.text_subtle, "No matching symbols.");
            } else {
                for sym in state.results.clone() {
                    let label = ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "{}  ({})",
                                sym.display_name, sym.symbol_kind
                            ))
                            .color(palette.accent),
                        )
                        .sense(egui::Sense::click()),
                    );
                    let author = code_symbol_result_author_id(&sym.symbol_entity_id);
                    let name_for_node = sym.display_name.clone();
                    ctx.accesskit_node_builder(label.id, move |node| {
                        node.set_role(SLASH_ITEM_ROLE);
                        node.set_author_id(author.clone());
                        node.set_label(name_for_node.clone());
                        node.add_action(accesskit::Action::Click);
                    });
                    if label.clicked() {
                        outcome = CodeSymbolSearchOutcome::Selected {
                            symbol_entity_id: sym.symbol_entity_id.clone(),
                            display_name: sym.display_name.clone(),
                        };
                    }
                }
            }
        });

    // Emit the dialog container node (Role::Dialog) — attach to a stable id for the window area.
    let dialog_id = egui::Id::new(CODE_SYMBOL_SEARCH_AUTHOR_ID).with("dialog-node");
    ctx.accesskit_node_builder(dialog_id, |node| {
        node.set_role(CODE_SYMBOL_SEARCH_ROLE);
        node.set_author_id(CODE_SYMBOL_SEARCH_AUTHOR_ID.to_owned());
        node.set_label("Insert code reference".to_owned());
    });

    // Escape closes the dialog (the same cancel affordance the slash menu uses).
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        outcome = CodeSymbolSearchOutcome::Cancelled;
    }
    if !open && matches!(outcome, CodeSymbolSearchOutcome::None) {
        outcome = CodeSymbolSearchOutcome::Cancelled;
    }

    // Re-run the lookup when the query text changed this frame (the caller drained the cell first).
    if state.query != prev_query {
        state.spawn_lookup();
    }

    outcome
}

/// Collect every block child of a parsed template/manual `doc` node into an owned `Vec` of
/// [`BlockNode`]s (dropping any stray non-block children defensively). The executor inserts
/// these after the caret's block.
pub fn doc_block_children(doc: &BlockNode) -> Vec<BlockNode> {
    doc.children
        .iter()
        .filter_map(Child::as_block)
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::BlockNode;
    use crate::rich_editor::document_model::position::DocPosition;

    #[test]
    fn author_id_helpers_are_stable() {
        assert_eq!(SLASH_MENU_AUTHOR_ID, "slash-menu");
        assert_eq!(slash_item_author_id("heading-1"), "slash-item-heading-1");
        assert_eq!(SLASH_ITEM_AUTHOR_ID_PREFIX, "slash-item-");
    }

    #[test]
    fn code_symbol_search_author_ids_match_contract() {
        // WP-KERNEL-012 MT-034 (AC-5): the code-symbol search dialog ids + roles.
        assert_eq!(CODE_SYMBOL_SEARCH_AUTHOR_ID, "code-symbol-search");
        assert_eq!(
            CODE_SYMBOL_SEARCH_INPUT_AUTHOR_ID,
            "code-symbol-search-input"
        );
        assert_eq!(
            code_symbol_result_author_id("ent-1"),
            "code-symbol-result-ent-1"
        );
        assert_eq!(CODE_SYMBOL_SEARCH_ROLE, accesskit::Role::Dialog);
        assert_eq!(CODE_SYMBOL_SEARCH_INPUT_ROLE, accesskit::Role::TextInput);
    }

    #[test]
    fn trigger_fires_at_leaf_start() {
        // AC-1: `/` at offset 0 (start of an empty paragraph) fires.
        assert!(slash_trigger_fires("/", 0));
    }

    #[test]
    fn trigger_fires_after_whitespace() {
        // "foo /" — the `/` at offset 4 is preceded by a space -> fires.
        assert!(slash_trigger_fires("foo /", 4));
    }

    #[test]
    fn trigger_does_not_fire_mid_word() {
        // AC-2: `/` in the middle of "ab/cd" (offset 2, preceded by 'b') -> no menu.
        assert!(!slash_trigger_fires("ab/cd", 2));
    }

    #[test]
    fn trigger_does_not_fire_in_url() {
        // RISK-1 / MC-001: a `/` inside "http://foo" is preceded by ':' -> no menu.
        // "http:/" — the first `/` at offset 5 is preceded by ':'.
        assert!(!slash_trigger_fires("http:/", 5));
        // The second `/` of "http://" at offset 6 is preceded by '/'.
        assert!(!slash_trigger_fires("http://", 6));
        // A path "src/app" — `/` at offset 3 preceded by 'c'.
        assert!(!slash_trigger_fires("src/app", 3));
    }

    #[test]
    fn url_path_char_vocabulary() {
        for c in ['a', 'Z', '0', ':', '.', '/', '-', '_'] {
            assert!(is_url_or_path_char(c), "{c} should be a url/path char");
        }
        for c in [' ', '\t', '\n', '!', '('] {
            assert!(!is_url_or_path_char(c), "{c} should NOT be a url/path char");
        }
    }

    #[test]
    fn open_trigger_at_start_empty_filter() {
        // Just typed `/` at the start: caret at 1, trigger at 0, empty filter.
        assert_eq!(open_slash_trigger("/", 1), Some((0, String::new())));
    }

    #[test]
    fn open_trigger_with_filter() {
        // "/head" with caret at end (5): trigger at 0, filter "head".
        assert_eq!(
            open_slash_trigger("/head", 5),
            Some((0, "head".to_string()))
        );
    }

    #[test]
    fn open_trigger_closes_on_space() {
        // "/foo bar" caret at 8: a space between the `/` and the caret -> token closed.
        assert_eq!(open_slash_trigger("/foo bar", 8), None);
    }

    #[test]
    fn open_trigger_none_for_url_slash() {
        // "http://x" caret at 8: the nearest `/` before the caret is URL-embedded -> None.
        assert_eq!(open_slash_trigger("http://x", 8), None);
    }

    #[test]
    fn open_trigger_after_word_and_space() {
        // "note /h" caret at 7: trigger at offset 5 (after the space), filter "h".
        assert_eq!(open_slash_trigger("note /h", 7), Some((5, "h".to_string())));
    }

    #[test]
    fn menu_state_delete_len_is_slash_plus_filter() {
        // RISK-3: the delete length is exactly 1 (the `/`) + the filter char count.
        let mut st = SlashMenuState::open(vec![0, 0], 0);
        assert_eq!(
            st.trigger_delete_len(),
            1,
            "empty filter -> delete just the '/'"
        );
        st.filter = "head".to_string();
        assert_eq!(st.trigger_delete_len(), 5, "'/' + 'head' = 5 chars");
    }

    #[test]
    fn caret_leaf_text_resolves_the_leaf() {
        let doc = BlockNode::doc(vec![BlockNode::paragraph("hello")]);
        let sel = Selection::caret(DocPosition::new(vec![0, 0], 3));
        let (path, text) = caret_leaf_text(&doc, &sel).unwrap();
        assert_eq!(path, vec![0, 0]);
        assert_eq!(text, "hello");
    }

    #[test]
    fn doc_block_children_extracts_blocks() {
        let doc = BlockNode::doc(vec![BlockNode::paragraph("a"), BlockNode::heading(1, "b")]);
        let blocks = doc_block_children(&doc);
        assert_eq!(blocks.len(), 2);
    }
}
