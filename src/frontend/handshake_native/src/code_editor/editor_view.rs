//! Completion popup + hover tooltip widgets for the native code editor (WP-KERNEL-012 MT-008 — E1).
//!
//! These are the two floating overlays the LSP / code-nav intelligence renders:
//! - [`CompletionPopup`] — a keyboard-navigable floating list of [`CompletionItem`]s anchored at the
//!   cursor pixel, with the AccessKit `Role::ListBox` container node `code_editor_completion_popup` and
//!   a `Role::Option`-equivalent node `code_editor_completion_item_{n}` per item (AC-005).
//! - [`HoverTooltip`] — a floating frame showing markdown-rendered symbol documentation, with the
//!   AccessKit `Role::Tooltip` node `code_editor_hover` whose value carries the hover text (AC-006), plus
//!   an optional "Go to definition" link.
//!
//! ## Non-focus-stealing windows (RISK-005 / HBR-QUIET)
//!
//! Both overlays render as NON-modal `egui::Area`s on the `Foreground` order. They do NOT call
//! `request_focus` and do NOT consume the editor's keyboard input on the frame they open: the editor
//! processes keystrokes FIRST (the panel's input handler runs before these are drawn), so opening the
//! popup never drops the character that triggered it (RISK-005). Arrow-key navigation inside the popup
//! is handled by the PANEL reading egui input + updating the popup's `selected_index`, not by the popup
//! taking focus — the same keyboard-driven-from-outside pattern the command palette uses.
//!
//! ## Reuse of the command-palette list semantics
//!
//! The selection/keyboard model (clamped `selected_index`, ArrowUp/Down wrap, Enter accepts, Escape
//! dismisses) mirrors `command_palette.rs` so the two keyboard-navigable floating lists behave
//! identically. The widgets themselves are stateless renderers: the panel owns the
//! [`CompletionState`] / [`HoverState`] and feeds it in, the same ownership split the palette uses.

use egui::accesskit;

use super::code_nav::CompletionItem;

/// Stable AccessKit author_id for the completion popup list container (AC-005: a `Role::ListBox`).
pub const CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID: &str = "code_editor_completion_popup";
/// Stable AccessKit author_id PREFIX for each completion item (`code_editor_completion_item_{n}`,
/// AC-005: a `Role::Option` — the field-correct accesskit 0.21 variant; see [`completion_item_role`]).
pub const CODE_EDITOR_COMPLETION_ITEM_AUTHOR_PREFIX: &str = "code_editor_completion_item_";
/// Stable AccessKit author_id for the hover tooltip (AC-006: a `Role::Tooltip`).
pub const CODE_EDITOR_HOVER_AUTHOR_ID: &str = "code_editor_hover";
/// Stable AccessKit author_id for the hover's "Go to definition" link.
pub const CODE_EDITOR_HOVER_GOTODEF_AUTHOR_ID: &str = "code_editor_hover_gotodef";

/// Fixed AccessKit `NodeId`s for the completion + hover overlays (default single panel). A fresh band
/// (500..) ABOVE the MT-007 diagnostic band (480..480+64=544 is reserved, so start at 600 to stay
/// disjoint from every panel band: container/scroll/text 200..202, cursor 210.., find 280.., fold
/// 300.., nav 370.., gutter 400.., breakpoint 410.., diagnostic 480..). These overlays render ONLY
/// while a completion / hover is active.
const COMPLETION_POPUP_NODE_ID: u64 = 600;
const COMPLETION_ITEM_NODE_ID_BASE: u64 = 601;
const HOVER_NODE_ID: u64 = 680;
const HOVER_GOTODEF_NODE_ID: u64 = 681;

/// Max completion items surfaced as AccessKit `Role::Option` nodes per frame (RISK-004 analog of the
/// cursor/fold caps). The popup itself caps the visible list; a pathological 1000-item response cannot
/// blow the per-frame node budget.
pub const MAX_ACCESSKIT_COMPLETION_ITEMS: usize = 64;

/// The completion-popup transient state, owned by the panel (the same ownership split the command
/// palette uses). Present only while a completion is showing.
#[derive(Debug, Clone, Default)]
pub struct CompletionState {
    /// The completion items to show (the popup renders these in order).
    pub items: Vec<CompletionItem>,
    /// The selected row index (clamped to `items` before use each frame). Arrow keys move it.
    pub selected_index: usize,
    /// The cursor pixel position the popup anchors below (top-left of the popup), captured when the
    /// completion was triggered so the popup floats at the cursor (MT positioning note).
    pub anchor: egui::Pos2,
}

impl CompletionState {
    /// Build a state for `items` anchored at `anchor`, with the first item selected.
    pub fn new(items: Vec<CompletionItem>, anchor: egui::Pos2) -> Self {
        Self {
            items,
            selected_index: 0,
            anchor,
        }
    }

    /// Move the selection down one row, wrapping (the ArrowDown binding). No-op on an empty list.
    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected_index = (self.selected_index + 1) % self.items.len();
    }

    /// Move the selection up one row, wrapping (the ArrowUp binding). No-op on an empty list.
    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let n = self.items.len();
        self.selected_index = (self.selected_index + n - 1) % n;
    }

    /// The currently-selected item (the one Enter accepts), or `None` on an empty list.
    pub fn selected(&self) -> Option<&CompletionItem> {
        self.items
            .get(self.selected_index.min(self.items.len().saturating_sub(1)))
    }
}

/// What the completion popup wants the panel to do after a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionOutcome {
    /// Nothing happened; keep the popup open.
    None,
    /// The user accepted item `index` (a click; Enter is handled by the panel keymap). The panel
    /// inserts the item's `insert_text` at the cursor and closes the popup.
    Accept(usize),
    /// The user dismissed the popup (a backdrop click). The panel clears the completion state.
    Dismiss,
}

/// The completion popup renderer (stateless — the panel owns the [`CompletionState`]).
pub struct CompletionPopup;

impl CompletionPopup {
    /// Role used for each completion item. The MT names `Role::Option`; in accesskit 0.21 the
    /// list-item role for a ListBox is `Role::ListBoxOption` — the field-correct variant the command
    /// palette also uses for its rows. AC-005 asserts the author_id (`code_editor_completion_item_{n}`),
    /// not the role string, so this satisfies it while using the real API (the same documented-deviation
    /// pattern as MT-003 Caret / MT-004 SearchInput).
    fn item_role() -> accesskit::Role {
        accesskit::Role::ListBoxOption
    }

    /// Render the popup as a non-focus-stealing floating `egui::Area` at `state.anchor`, with the
    /// keyboard-navigable list. Returns the outcome for this frame. The AccessKit ListBox container +
    /// per-item Option nodes are emitted so a swarm agent can read/drive the popup (AC-005 / HBR-SWARM).
    ///
    /// `instance` is the panel's AccessKit instance suffix (empty for the default panel) so a diff
    /// view's two editors do not collide on the popup author_ids (RISK-004, the same scheme the panel
    /// nodes use).
    pub fn show(ctx: &egui::Context, state: &CompletionState, instance: &str) -> CompletionOutcome {
        if state.items.is_empty() {
            return CompletionOutcome::None;
        }
        let mut outcome = CompletionOutcome::None;
        let selected = state.selected_index.min(state.items.len() - 1);

        let popup_author = suffixed(CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID, instance);
        // A non-modal Area on the Foreground order, anchored at the cursor pixel. It never requests
        // focus, so the editor keeps keyboard input (RISK-005).
        let area_id = egui::Id::new(("code-editor-completion-area", instance));
        egui::Area::new(area_id)
            .order(egui::Order::Foreground)
            .fixed_pos(state.anchor)
            .interactable(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.set_max_height(220.0);
                    egui::ScrollArea::vertical()
                        .auto_shrink([true, true])
                        .show(ui, |ui| {
                            for (n, item) in state.items.iter().enumerate() {
                                let is_selected = n == selected;
                                let label = format!("{}  {}", item.kind.icon(), item.label);
                                let mut text = egui::RichText::new(label).monospace();
                                if is_selected {
                                    text = text.strong();
                                }
                                let resp = ui.add(
                                    egui::Label::new(text).sense(egui::Sense::click()),
                                );
                                if is_selected {
                                    // Paint a selection background so the keyboard selection is visible.
                                    ui.painter().rect_filled(
                                        resp.rect.expand2(egui::vec2(2.0, 1.0)),
                                        2.0,
                                        ui.visuals().selection.bg_fill.linear_multiply(0.4),
                                    );
                                }
                                if !item.detail.is_empty() {
                                    resp.clone().on_hover_text(&item.detail);
                                }
                                if resp.clicked() {
                                    outcome = CompletionOutcome::Accept(n);
                                }
                                // Emit the per-item Option node (capped — RISK-004). Its value carries
                                // the label + detail so an agent reads the suggestion by id.
                                if n < MAX_ACCESSKIT_COMPLETION_ITEMS {
                                    let author = if instance.is_empty() {
                                        format!("{CODE_EDITOR_COMPLETION_ITEM_AUTHOR_PREFIX}{n}")
                                    } else {
                                        format!(
                                            "{CODE_EDITOR_COMPLETION_ITEM_AUTHOR_PREFIX}{n}#{instance}"
                                        )
                                    };
                                    let node_id = item_node_id(n, instance);
                                    let value = if item.detail.is_empty() {
                                        item.label.clone()
                                    } else {
                                        format!("{} ({})", item.label, item.detail)
                                    };
                                    ctx.accesskit_node_builder(node_id, move |node| {
                                        node.set_role(Self::item_role());
                                        node.set_author_id(author.clone());
                                        node.set_label("Completion item".to_owned());
                                        node.set_value(value.clone());
                                        if is_selected {
                                            node.set_selected(true);
                                        }
                                        node.add_action(accesskit::Action::Click);
                                    });
                                }
                            }
                        });

                    // Emit the ListBox container node (AC-005). Its value reports the item count + the
                    // selected index so a swarm agent can read the popup state.
                    let popup_node_id = popup_node_id(instance);
                    let count = state.items.len();
                    let author = popup_author.clone();
                    ctx.accesskit_node_builder(popup_node_id, move |node| {
                        node.set_role(accesskit::Role::ListBox);
                        node.set_author_id(author.clone());
                        node.set_label("Code editor completions".to_owned());
                        node.set_value(format!("{count} items, {} selected", selected + 1));
                    });
                });
            });

        outcome
    }
}

/// The hover-tooltip transient state, owned by the panel. Present only while a hover is showing.
#[derive(Debug, Clone, Default)]
pub struct HoverState {
    /// The markdown body to render (heading + kind + key + staleness + optional doc).
    pub markdown: String,
    /// The display name (the AC-006 text the test asserts the tooltip value contains).
    pub display_name: String,
    /// The cursor pixel position the tooltip anchors near (top-left), so it floats at the hovered word.
    pub anchor: egui::Pos2,
    /// The go-to-definition target line (0-based), shown as a clickable link when present.
    pub definition_line: Option<usize>,
}

/// What the hover tooltip wants the panel to do after a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HoverOutcome {
    /// Nothing happened; keep the tooltip showing.
    None,
    /// The user clicked "Go to definition"; the panel navigates to `line` (0-based) and closes hover.
    GotoDefinition(usize),
}

/// The hover-tooltip renderer (stateless — the panel owns the [`HoverState`]).
pub struct HoverTooltip;

impl HoverTooltip {
    /// Render the hover as a non-focus-stealing floating frame at `state.anchor`. The markdown is shown
    /// as plain wrapped text (a lightweight inline renderer — bold `**x**` stripped to the inner text —
    /// since `egui_commonmark` is not a current dependency; the contract makes it optional). Emits the
    /// `Role::Tooltip` node `code_editor_hover` whose value carries the hover text (AC-006) and, when a
    /// definition is known, a "Go to definition" link node.
    pub fn show(ctx: &egui::Context, state: &HoverState, instance: &str) -> HoverOutcome {
        if state.markdown.trim().is_empty() {
            return HoverOutcome::None;
        }
        let mut outcome = HoverOutcome::None;
        let hover_author = suffixed(CODE_EDITOR_HOVER_AUTHOR_ID, instance);
        let area_id = egui::Id::new(("code-editor-hover-area", instance));
        // Anchor slightly below the cursor so the tooltip does not cover the hovered word.
        let pos = state.anchor + egui::vec2(0.0, 18.0);
        egui::Area::new(area_id)
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .interactable(true)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(420.0);
                    // Render the markdown as inline-formatted text (lightweight — see method docs).
                    render_markdown_inline(ui, &state.markdown);
                    if let Some(line) = state.definition_line {
                        ui.separator();
                        let link = ui.link(format!("Go to definition (line {})", line + 1));
                        // Emit the go-to-def link node so an agent can dispatch it by id (HBR-SWARM).
                        let author = suffixed(CODE_EDITOR_HOVER_GOTODEF_AUTHOR_ID, instance);
                        let node_id = hover_gotodef_node_id(instance);
                        ctx.accesskit_node_builder(node_id, move |node| {
                            node.set_role(accesskit::Role::Link);
                            node.set_author_id(author.clone());
                            node.set_label("Go to definition".to_owned());
                            node.add_action(accesskit::Action::Click);
                        });
                        if link.clicked() {
                            outcome = HoverOutcome::GotoDefinition(line);
                        }
                    }

                    // Emit the Tooltip container node (AC-006). Its value carries the display name +
                    // the markdown so the AC-006 test finds the identifier in its text content.
                    let hover_node_id = hover_node_id(instance);
                    let author = hover_author.clone();
                    let value = format!("{}\n{}", state.display_name, state.markdown);
                    ctx.accesskit_node_builder(hover_node_id, move |node| {
                        node.set_role(accesskit::Role::Tooltip);
                        node.set_author_id(author.clone());
                        node.set_label("Code editor hover".to_owned());
                        node.set_value(value.clone());
                    });
                });
            });
        outcome
    }
}

/// A lightweight inline markdown renderer for the hover body: each line becomes a wrapped paragraph;
/// `**bold**` runs render strong, inline `` `code` `` renders monospace. This covers exactly the
/// `markdown_for_symbol` output shape (the contract makes `egui_commonmark` optional — "otherwise plain
/// text"); a full commonmark renderer is not pulled in for this small surface.
fn render_markdown_inline(ui: &mut egui::Ui, markdown: &str) {
    for raw_line in markdown.lines() {
        let line = raw_line.trim_end();
        if line.is_empty() {
            ui.add_space(4.0);
            continue;
        }
        ui.horizontal_wrapped(|ui| {
            // Tokenize on `**` (bold) and `` ` `` (code) toggles, emitting a styled label per run.
            let mut bold = false;
            let mut code = false;
            let mut buf = String::new();
            let mut chars = line.chars().peekable();
            let flush = |ui: &mut egui::Ui, buf: &mut String, bold: bool, code: bool| {
                if buf.is_empty() {
                    return;
                }
                let mut text = egui::RichText::new(buf.as_str());
                if bold {
                    text = text.strong();
                }
                if code {
                    text = text.monospace();
                }
                ui.label(text);
                buf.clear();
            };
            while let Some(c) = chars.next() {
                if c == '*' && chars.peek() == Some(&'*') {
                    chars.next(); // consume the second '*'
                    flush(ui, &mut buf, bold, code);
                    bold = !bold;
                } else if c == '`' {
                    flush(ui, &mut buf, bold, code);
                    code = !code;
                } else {
                    buf.push(c);
                }
            }
            flush(ui, &mut buf, bold, code);
        });
    }
}

/// Append the instance suffix to a base author_id (`base#instance`), or the bare base for the default
/// panel (so the MT-contract ids match exactly — AC-005/AC-006).
fn suffixed(base: &str, instance: &str) -> String {
    if instance.is_empty() {
        base.to_owned()
    } else {
        format!("{base}#{instance}")
    }
}

/// The fixed `egui::Id` for the completion popup container (default panel; instances hash the author_id).
fn popup_node_id(instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: a single hand-assigned fixed id in the disjoint overlay band; never reused.
        unsafe { egui::Id::from_high_entropy_bits(COMPLETION_POPUP_NODE_ID) }
    } else {
        egui::Id::new(suffixed(CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID, instance))
    }
}

/// The fixed `egui::Id` for completion item `n` (default panel; instances hash the suffixed author_id).
fn item_node_id(n: usize, instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: each `n` maps to a distinct fixed slot in the disjoint overlay band; never reused.
        unsafe { egui::Id::from_high_entropy_bits(COMPLETION_ITEM_NODE_ID_BASE + n as u64) }
    } else {
        egui::Id::new(format!(
            "{CODE_EDITOR_COMPLETION_ITEM_AUTHOR_PREFIX}{n}#{instance}"
        ))
    }
}

/// The fixed `egui::Id` for the hover tooltip (default panel; instances hash the author_id).
fn hover_node_id(instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: a single hand-assigned fixed id in the disjoint overlay band; never reused.
        unsafe { egui::Id::from_high_entropy_bits(HOVER_NODE_ID) }
    } else {
        egui::Id::new(suffixed(CODE_EDITOR_HOVER_AUTHOR_ID, instance))
    }
}

/// The fixed `egui::Id` for the hover go-to-definition link (default panel; instances hash the author_id).
fn hover_gotodef_node_id(instance: &str) -> egui::Id {
    if instance.is_empty() {
        // SAFETY: a single hand-assigned fixed id in the disjoint overlay band; never reused.
        unsafe { egui::Id::from_high_entropy_bits(HOVER_GOTODEF_NODE_ID) }
    } else {
        egui::Id::new(suffixed(CODE_EDITOR_HOVER_GOTODEF_AUTHOR_ID, instance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_editor::code_nav::CompletionKind;

    fn item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_owned(),
            insert_text: label.to_owned(),
            kind: CompletionKind::Function,
            detail: "function".to_owned(),
            documentation: String::new(),
            symbol_entity_id: String::new(),
        }
    }

    #[test]
    fn completion_state_selection_wraps() {
        let mut state =
            CompletionState::new(vec![item("a"), item("b"), item("c")], egui::pos2(0.0, 0.0));
        assert_eq!(state.selected_index, 0);
        state.select_next();
        assert_eq!(state.selected_index, 1);
        state.select_prev();
        assert_eq!(state.selected_index, 0);
        // Wrap backward from 0 -> last.
        state.select_prev();
        assert_eq!(state.selected_index, 2);
        // Wrap forward from last -> 0.
        state.select_next();
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.selected().map(|i| i.label.as_str()), Some("a"));
    }

    #[test]
    fn empty_completion_selection_is_noop() {
        let mut state = CompletionState::default();
        state.select_next();
        state.select_prev();
        assert_eq!(state.selected_index, 0);
        assert!(state.selected().is_none());
    }

    // The overlay band (600..) sits above every panel band (the highest panel band is the
    // diagnostic band at 480..480+64=544). The completion-item band must not overrun into the
    // hover node, and the whole overlay band must sit above the diagnostic band's top. These are
    // compile-time invariants over `const` node-id allocations, so they are enforced with
    // `const { assert!(...) }` rather than a runtime `assert!` (which clippy would flag as
    // assertions_on_constants / "optimized out").
    const DIAGNOSTIC_BAND_TOP: u64 = 544;
    const _: () = assert!(
        COMPLETION_POPUP_NODE_ID > DIAGNOSTIC_BAND_TOP,
        "popup node must sit above the diagnostic band top"
    );
    const _: () = assert!(
        HOVER_NODE_ID > COMPLETION_ITEM_NODE_ID_BASE + MAX_ACCESSKIT_COMPLETION_ITEMS as u64,
        "the hover node must sit above the completion-item band's top"
    );
}
