//! Loom folder tree + color labels (WP-KERNEL-012 MT-022, cluster E3).
//!
//! ## What this is
//!
//! [`LoomFolderTree`] is a native, collapsible tree of Loom **folders** (the Obsidian-parity
//! hierarchical navigation for the knowledge surface). Each folder node carries a small color swatch
//! (rect) before its label; expanding a folder lazy-loads its child blocks from the backend and renders
//! them indented beneath it; clicking a folder selects/expands that organizational overlay in this surface,
//! while clicking a leaf block opens the real LoomBlock through the shell. Both interactions emit typed
//! [`FolderTreeEvent`] values for the host to apply. It is the
//! native peer of the React `WorkspaceSidebar.tsx` document/canvas lists, but a real recursive tree.
//!
//! ## Backend reality (Spec-Realism Gate — the MT-008/021/023 "verify, don't trust the contract" rule)
//!
//! The MT-022 contract's assumed surface — `content_type='folder'` LoomBlocks, color stored in
//! `content_json.metadata.color_label`, children via `views/sorted?tag_ids=` — does **NOT** exist in the
//! running backend. Verified READ-ONLY against `src/backend/handshake_core/src/{api,storage}/loom.rs`:
//!   - `LoomBlockContentType` has NO `Folder` variant (note/file/annotated_file/tag_hub/journal/canvas/
//!     view_def only), and the PATCH `LoomBlockUpdate` exposes NO `content_json`/`metadata`/`color`
//!     field (only title/pinned/favorite/journal_date/pin_order). So the contract's color-in-content_json
//!     write path is impossible — there is nothing to clobber and nothing to write there.
//!   - The REAL folder authority is the dedicated **`loom_folders`** subsystem (MT-181
//!     FolderTreeAndColorLabels, Master Spec §7.1.4.3), an organizational overlay over LoomBlocks with a
//!     first-class `color: Option<String>` column. Its verified routes (mounted in `loom::routes`):
//!       * `GET    /workspaces/{ws}/loom/folders`                       -> `Vec<LoomFolder>` (the tree
//!         rows; the parent/child shape is `parent_folder_id`, so the tree is built CLIENT-side from the
//!         flat row list — this is the standalone-testable `build_tree` core, PROOF1).
//!       * `GET    /workspaces/{ws}/loom/folders/{folder_id}/blocks`    -> `Vec<LoomBlock>` (the lazy
//!         child-block load, AC2).
//!       * `PATCH  /workspaces/{ws}/loom/folders/{folder_id}`           body `{ "color": "#rrggbb" }`
//!         -> the recolor (AC4). `LoomFolderUpdate.color` is `Option<Option<String>>` server-side, i.e. a
//!         TRUE JSON merge-patch: sending ONLY `color` leaves name/sort/parent untouched. This satisfies
//!         RISK-2/MC-2 (no whole-record clobber) at the backend-contract level — far stronger than the
//!         contract's content_json approach.
//!
//! The widget therefore models a folder node by its `folder_id` (the recolor + child-load key) and the
//! leaf children by their `block_id` (the open key). The split the MT `implementation_notes` describe is
//! honored: the flat-rows -> tree build, cycle/depth guard (RISK-1/MC-1), hex-color parse, lazy-load
//! caching (RISK-3/MC-3), empty "No folders" (AC7), and backend-error + Retry (AC8) are ALL unit/kittest
//! testable STANDALONE with row lists; the live-backend integration test is gated behind the
//! `integration` feature and requires the managed backend resource — it never fakes the backend.
//!
//! ## Repaint discipline (the MT-015 idle-repaint lesson)
//!
//! A folder's child-load spinner animates ONLY while a genuine in-flight fetch is dispatched: the host
//! sets [`FolderNode::loading`] = true ONLY when it actually spawns a `list_folder_blocks` request (a
//! runtime-backed call). A headless / no-runtime render therefore never enters a perpetual `Loading…`;
//! it shows the static collapsed/expanded state. The widget itself requests a repaint ONLY for the one
//! frame a spinner is genuinely active, mirroring the graph view's bounded-animation rule.
//!
//! ## AccessKit (HBR-SWARM)
//!
//! Every folder row emits a live AccessKit node (`folder-tree.node.{sanitized_folder_id}`,
//! Role::TreeItem, with Expand/Collapse + the Click default-open action) and every color swatch an
//! actionable button (`folder-tree.color.{sanitized_folder_id}`, Role::Button); every leaf block row a
//! TreeItem
//! (`folder-tree.node.{sanitized_block_id}`). The error banner's Retry button is `folder-tree.retry`. Ids
//! are sanitized to `[a-z0-9-]` via [`crate::project_tree::stable_part`] (RISK-4 / MC-4) so a raw id with
//! slashes or colons can never break the AccessKit tree. Manual indented rows (NOT
//! `egui::CollapsingHeader`) are used precisely because CollapsingHeader does not expose a stable
//! author_id (the contract's RISK-4 control).

use egui::accesskit;
use egui::{Color32, Sense, Stroke, Vec2};

use crate::mcp::action::{serialize_same_target_click_completion, ClickCompletionState};
use crate::theme::HsPalette;

/// AccessKit author_id prefix for a tree row (folder OR leaf block). The full id is
/// `folder-tree.node.{sanitized_id}`.
pub const NODE_AUTHOR_ID_PREFIX: &str = "folder-tree.node.";
pub const STATUS_AUTHOR_ID_PREFIX: &str = "folder-tree.status.";

/// AccessKit author_id prefix for a folder's actionable color-swatch button:
/// `folder-tree.color.{sanitized_id}`.
pub const COLOR_AUTHOR_ID_PREFIX: &str = "folder-tree.color.";

/// AccessKit author_id for the error-banner Retry button.
pub const RETRY_AUTHOR_ID: &str = "folder-tree.retry";

/// AccessKit author_ids for the create/rename dialog text inputs. TextEdit hint text is visual-only in
/// egui, so the live nodes also receive an explicit accessible label below.
pub const CREATE_NAME_INPUT_AUTHOR_ID: &str = "folder-tree.create.name";
pub const RENAME_NAME_INPUT_AUTHOR_ID: &str = "folder-tree.rename.name";
pub const NEW_FOLDER_AUTHOR_ID: &str = "folder-tree.new-folder";
pub const CREATE_SUBMIT_AUTHOR_ID: &str = "folder-tree.create.submit";
pub const CREATE_CANCEL_AUTHOR_ID: &str = "folder-tree.create.cancel";
pub const RENAME_SUBMIT_AUTHOR_ID: &str = "folder-tree.rename.submit";
pub const RENAME_CANCEL_AUTHOR_ID: &str = "folder-tree.rename.cancel";
pub const DELETE_CONFIRM_AUTHOR_ID: &str = "folder-tree.delete.confirm";
pub const DELETE_CANCEL_AUTHOR_ID: &str = "folder-tree.delete.cancel";
pub const MOVE_UNDER_MENU_AUTHOR_ID: &str = "folder-tree.move-under";
/// AccessKit author_id prefix for a concrete "Move under" target. The full id includes both the
/// source and target folder ids, so two folders with the same visible title remain unambiguous.
pub const MOVE_TARGET_AUTHOR_ID_PREFIX: &str = "folder-tree.move-target.";

/// Indent per nesting level, in px (the contract's "16px per depth level").
pub const INDENT_PER_LEVEL: f32 = 16.0;

/// Color swatch size (the contract's "12x12 filled rectangle").
pub const SWATCH_SIZE: f32 = 12.0;

/// Horizontal chrome occupied by a folder's disclosure glyph, swatch, and row gaps. Leaf content
/// reserves this width in addition to its structural depth so it begins at least one indent step to
/// the right of its parent folder label.
const LEAF_PARENT_CHROME_RESERVE: f32 = INDENT_PER_LEVEL * 3.0;

/// Deterministic color-picker swatches used by the right-click "Change color" flow and the swatch
/// shortcut. The built-in egui color editor still appears below these buttons for arbitrary colors, but
/// fixed swatches give kittest and model drivers stable labels to exercise the real pick path.
const COLOR_PICKER_SWATCHES: [(&str, &str); 4] = [
    ("Red", "#ff0000"),
    ("Green", "#22c55e"),
    ("Blue", "#3b82f6"),
    ("Amber", "#f59e0b"),
];

/// Render-depth limit: nesting deeper than this shows a `…(deep)` truncation row (contract: "up to 5
/// levels of nesting. Beyond that, show a '...(deep)' truncation row").
pub const MAX_RENDER_DEPTH: usize = 5;

/// Return an AccessKit-safe, collision-resistant suffix for a raw folder/block id. Preserve canonical
/// ids verbatim; when slugging changes the input, append deterministic FNV-1a identity so values such
/// as `a/b` and `a:b` cannot alias onto the same live node.
fn collision_safe_folder_part(id: &str) -> String {
    let slug = if id.chars().any(|ch| ch.is_ascii_alphanumeric()) {
        crate::project_tree::stable_part(id)
    } else {
        "id".to_owned()
    };
    if slug == id {
        return slug;
    }

    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let hash = id.as_bytes().iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(FNV_PRIME)
    });
    format!("{slug}-{hash:016x}")
}

/// The stable AccessKit author_id for a tree row, with a one-to-one safe suffix.
pub fn node_author_id(id: &str) -> String {
    format!("{NODE_AUTHOR_ID_PREFIX}{}", collision_safe_folder_part(id))
}

pub fn status_author_id(folder_id: &str) -> String {
    format!(
        "{STATUS_AUTHOR_ID_PREFIX}{}",
        collision_safe_folder_part(folder_id)
    )
}

/// The stable AccessKit author_id for a folder's actionable color-swatch button:
/// `folder-tree.color.{sanitized_folder_id}`.
pub fn color_author_id(folder_id: &str) -> String {
    format!(
        "{COLOR_AUTHOR_ID_PREFIX}{}",
        collision_safe_folder_part(folder_id)
    )
}

/// The stable AccessKit author_id for moving `source_folder_id` under `target_folder_id`.
pub fn move_target_author_id(source_folder_id: &str, target_folder_id: &str) -> String {
    format!(
        "{MOVE_TARGET_AUTHOR_ID_PREFIX}{}.{}",
        collision_safe_folder_part(source_folder_id),
        collision_safe_folder_part(target_folder_id)
    )
}

pub use handshake_folder_tree::{
    build_tree, color_to_hex, parse_folder_color_token, parse_hex_color, FolderNode, FolderRow,
    LeafBlock, MAX_BUILD_DEPTH,
};
use handshake_folder_tree::FolderOpenCompletion;

/// The typed event a folder-tree interaction produces this frame, for the host to apply. The host owns
/// the backend wiring (lazy child fetch on `ExpandFolder`, recolor PATCH on `ChangeColor`, open on
/// `OpenBlock`) so the widget itself never touches the network (HBR-QUIET).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderTreeEvent {
    /// A folder row was expanded and its blocks are NOT yet loaded: the host should spawn the lazy
    /// `GET /loom/folders/{folder_id}/blocks` fetch (and set the node `loading`). Carries the folder id.
    ExpandFolder {
        folder_id: String,
    },
    /// A folder row was collapsed (no fetch; purely a UI toggle the host may persist).
    CollapseFolder {
        folder_id: String,
    },
    /// A folder row's label was clicked: select and expand that organizational overlay in the folder
    /// surface and reveal its member blocks. A folder id is not a LoomBlock id and must never be routed
    /// through generic LoomBlock open/navigation.
    OpenFolder {
        folder_id: String,
        /// UI generation created by the exact row click. The host binds this to a fresh backend
        /// child-list request sequence and ignores stale queued generations.
        action_generation: u64,
    },
    /// A leaf block row was clicked: fire `on_open(block_id)` (AC5). Carries the block id.
    OpenBlock {
        block_id: String,
    },
    /// The color swatch / "Change color" was used and a new color picked: the host should PATCH
    /// `{ "color": "#rrggbb" }` to `/loom/folders/{folder_id}`, then refetch the authoritative folder
    /// list. The refetch applies the persisted swatch; the requested color is never applied optimistically.
    ChangeColor {
        folder_id: String,
        color: Color32,
    },
    CreateFolder {
        parent_folder_id: Option<String>,
        name: String,
    },
    RenameFolder {
        folder_id: String,
        name: String,
    },
    MoveFolder {
        folder_id: String,
        parent_folder_id: Option<String>,
        sort_order: Option<i32>,
    },
    DeleteFolder {
        folder_id: String,
    },
    /// The error-banner Retry button was pressed: the host should re-fire the initial folder load (AC8).
    Retry,
}

/// The folder-tree widget state. Held by the host (the pane), mutated in place by
/// [`LoomFolderTree::show`]. `root_nodes` is the built forest; `loading`/`error` are the top-level
/// initial-load state.
#[derive(Debug, Clone, Default)]
pub struct LoomFolderTree {
    pub workspace_id: String,
    pub root_nodes: Vec<FolderNode>,
    /// True while the INITIAL `GET /loom/folders` is in flight (the top-level spinner). Bounded: the
    /// host clears it when the fetch resolves/fails.
    pub loading: bool,
    /// Set on a backend failure; renders the error banner + Retry (AC8). `None` => no error.
    pub error: Option<String>,
    /// Projection of the first live node-owned child-list failure in the current forest. Recomputed
    /// recursively after every exact child terminal so success for one folder cannot hide another
    /// folder's still-live failure.
    pub child_error_banner: Option<String>,
    /// The last failed mutation. Authoritative list reconciliation must not erase this message before
    /// an operator/model can perceive it; a new mutation or explicit Retry clears it.
    pub operation_error: Option<String>,
    pub selected_folder_id: Option<String>,
    create_draft: Option<(Option<String>, String)>,
    rename_draft: Option<(String, String)>,
    /// Folder id, visible title, and the number of descendant folders the SurrealDB store will cascade.
    delete_draft: Option<(String, String, usize)>,
}

impl LoomFolderTree {
    /// A fresh tree for `workspace_id` with no folders loaded yet.
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            ..Self::default()
        }
    }

    /// Install the built forest from a `GET /loom/folders` result (the host calls this when the initial
    /// fetch resolves), clearing loading/error.
    pub fn set_folders(&mut self, rows: &[FolderRow]) {
        self.root_nodes = build_tree(rows);
        if self
            .selected_folder_id
            .as_deref()
            .is_some_and(|id| !rows.iter().any(|row| row.folder_id == id))
        {
            self.selected_folder_id = None;
        }
        self.loading = false;
        self.error = None;
        self.refresh_child_error_banner();
    }

    /// Find a folder node by id anywhere in the forest (mutable). The host uses this to flip `expanded`,
    /// install loaded `child_blocks`, clear `loading`, or update `color` after an async op resolves.
    pub fn find_folder_mut(&mut self, folder_id: &str) -> Option<&mut FolderNode> {
        for r in &mut self.root_nodes {
            if let Some(found) = r.find_mut(folder_id) {
                return Some(found);
            }
        }
        None
    }

    /// Recompute the visible global child-error banner from current node-owned truth.
    pub fn refresh_child_error_banner(&mut self) {
        self.child_error_banner = self
            .root_nodes
            .iter()
            .find_map(FolderNode::first_child_error)
            .map(str::to_owned);
    }

    /// Total folder nodes across the whole forest (for tests + the empty-state check).
    pub fn folder_count(&self) -> usize {
        self.root_nodes.iter().map(FolderNode::folder_count).sum()
    }

    /// Render the tree and return the typed event (if any) this frame produced. The host applies the
    /// event (lazy fetch, recolor PATCH, open). Requests a repaint ONLY for a frame where a genuine
    /// child-load spinner is active (idle-repaint discipline).
    pub fn show(&mut self, ui: &mut egui::Ui, palette: &HsPalette) -> Option<FolderTreeEvent> {
        let mut event: Option<FolderTreeEvent> = None;
        self.refresh_child_error_banner();

        let new_folder = ui.button("New folder");
        emit_button_accesskit(ui, new_folder.id, NEW_FOLDER_AUTHOR_ID, "New folder");
        if new_folder.clicked() {
            self.create_draft = Some((None, String::new()));
        }

        // ── Error banner (AC8) ─────────────────────────────────────────────────────────────────────
        if self.error.is_some()
            || self.child_error_banner.is_some()
            || self.operation_error.is_some()
        {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    if let Some(err) = self.operation_error.as_deref() {
                        ui.colored_label(palette.error_text, format!("⚠ {err}"));
                    }
                    if let Some(err) = self.error.as_deref() {
                        ui.colored_label(palette.error_text, format!("⚠ {err}"));
                    }
                    if let Some(err) = self.child_error_banner.as_deref() {
                        ui.colored_label(palette.error_text, format!("⚠ {err}"));
                    }
                });
                let retry = ui.button("Retry");
                emit_button_accesskit(ui, retry.id, RETRY_AUTHOR_ID, "Retry");
                if retry.clicked() {
                    event = Some(FolderTreeEvent::Retry);
                }
            });
            ui.separator();
        }

        // ── Top-level loading spinner (bounded: only while the initial fetch is in flight) ──────────
        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading folders…");
            });
            // A genuine in-flight fetch is the ONE case we keep animating so the spinner advances; the
            // host clears `loading` when the fetch resolves/fails, so this is bounded.
            ui.ctx().request_repaint();
        }

        // ── Empty state (AC7) ───────────────────────────────────────────────────────────────────────
        if !self.loading
            && self.error.is_none()
            && self.child_error_banner.is_none()
            && self.operation_error.is_none()
            && self.root_nodes.is_empty()
        {
            ui.weak("No folders");
        }

        // ── The tree ────────────────────────────────────────────────────────────────────────────────
        // Render into a stable order; mutate-in-place is done after the borrow ends to avoid aliasing.
        let mut any_spinner = false;
        // We collect the produced event from the recursive render (last write wins per frame; a single
        // pointer event can only hit one row).
        fn collect_choices(nodes: &[FolderNode], choices: &mut Vec<(String, String)>) {
            for node in nodes {
                choices.push((node.folder_id.clone(), node.title.clone()));
                collect_choices(&node.child_folders, choices);
            }
        }
        let mut folder_choices = Vec::new();
        collect_choices(&self.root_nodes, &mut folder_choices);
        for i in 0..self.root_nodes.len() {
            // SAFETY of borrow: render one root subtree at a time with a fresh mutable borrow.
            let (ev, spin) = render_folder(
                &mut self.root_nodes[i],
                ui,
                palette,
                0,
                &self.workspace_id,
                self.selected_folder_id.as_deref(),
                &mut self.create_draft,
                &mut self.rename_draft,
                &mut self.delete_draft,
                &folder_choices,
            );
            if ev.is_some() {
                event = ev;
            }
            any_spinner |= spin;
        }
        if any_spinner {
            // A child-load spinner is genuinely active this frame; advance it (bounded — host clears the
            // node's `loading` when the fetch resolves).
            ui.ctx().request_repaint();
        }

        if let Some((parent_folder_id, draft)) = &mut self.create_draft {
            let mut close = false;
            egui::Window::new("Create folder")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    let name_input =
                        ui.add(egui::TextEdit::singleline(draft).hint_text("Folder name"));
                    set_accessible_text_input(
                        ui,
                        name_input.id,
                        CREATE_NAME_INPUT_AUTHOR_ID,
                        "Folder name",
                    );
                    ui.horizontal(|ui| {
                        let submit =
                            ui.add_enabled(!draft.trim().is_empty(), egui::Button::new("Create"));
                        emit_button_accesskit(
                            ui,
                            submit.id,
                            CREATE_SUBMIT_AUTHOR_ID,
                            "Create folder",
                        );
                        if submit.clicked() {
                            event = Some(FolderTreeEvent::CreateFolder {
                                parent_folder_id: parent_folder_id.clone(),
                                name: draft.trim().to_owned(),
                            });
                            close = true;
                        }
                        let cancel = ui.button("Cancel");
                        emit_button_accesskit(
                            ui,
                            cancel.id,
                            CREATE_CANCEL_AUTHOR_ID,
                            "Cancel create folder",
                        );
                        if cancel.clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                self.create_draft = None;
            }
        }
        if let Some((folder_id, draft)) = &mut self.rename_draft {
            let mut close = false;
            egui::Window::new("Rename folder")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    let name_input =
                        ui.add(egui::TextEdit::singleline(draft).hint_text("Folder name"));
                    set_accessible_text_input(
                        ui,
                        name_input.id,
                        RENAME_NAME_INPUT_AUTHOR_ID,
                        "Folder name",
                    );
                    ui.horizontal(|ui| {
                        let submit =
                            ui.add_enabled(!draft.trim().is_empty(), egui::Button::new("Rename"));
                        emit_button_accesskit(
                            ui,
                            submit.id,
                            RENAME_SUBMIT_AUTHOR_ID,
                            "Rename folder",
                        );
                        if submit.clicked() {
                            event = Some(FolderTreeEvent::RenameFolder {
                                folder_id: folder_id.clone(),
                                name: draft.trim().to_owned(),
                            });
                            close = true;
                        }
                        let cancel = ui.button("Cancel");
                        emit_button_accesskit(
                            ui,
                            cancel.id,
                            RENAME_CANCEL_AUTHOR_ID,
                            "Cancel rename folder",
                        );
                        if cancel.clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                self.rename_draft = None;
            }
        }
        if let Some((folder_id, title, descendant_count)) = self.delete_draft.clone() {
            let mut close = false;
            egui::Window::new("Delete folder")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    let descendant_word = if descendant_count == 1 {
                        "folder"
                    } else {
                        "folders"
                    };
                    ui.label(format!(
                        "Delete folder '{title}' and its {descendant_count} descendant {descendant_word}?"
                    ));
                    ui.label(
                        "This permanently removes the folder subtree and its folder memberships. The Loom blocks themselves remain.",
                    );
                    ui.horizontal(|ui| {
                        let confirm = ui.button("Delete");
                        emit_button_accesskit(
                            ui,
                            confirm.id,
                            DELETE_CONFIRM_AUTHOR_ID,
                            "Confirm delete folder",
                        );
                        if confirm.clicked() {
                            event = Some(FolderTreeEvent::DeleteFolder {
                                folder_id: folder_id.clone(),
                            });
                            close = true;
                        }
                        let cancel = ui.button("Cancel");
                        emit_button_accesskit(
                            ui,
                            cancel.id,
                            DELETE_CANCEL_AUTHOR_ID,
                            "Cancel delete folder",
                        );
                        if cancel.clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                self.delete_draft = None;
            }
        }
        if let Some(FolderTreeEvent::OpenFolder { folder_id, .. }) = &event {
            self.selected_folder_id = Some(folder_id.clone());
        }

        event
    }
}

fn set_accessible_text_input(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author_id = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_author_id(author_id);
        node.set_label(label);
    });
}

/// Render one folder subtree at `depth`, returning `(event, any_spinner_active)`. Mutates the node's
/// `expanded` flag on a disclosure-triangle click. A `depth >= MAX_RENDER_DEPTH` row is replaced by a
/// single `…(deep)` truncation label (RISK-1 render guard) so a deep tree can never hang the layout.
#[allow(clippy::too_many_arguments)]
fn render_folder(
    node: &mut FolderNode,
    ui: &mut egui::Ui,
    palette: &HsPalette,
    depth: usize,
    workspace_id: &str,
    selected_folder_id: Option<&str>,
    create_draft: &mut Option<(Option<String>, String)>,
    rename_draft: &mut Option<(String, String)>,
    delete_draft: &mut Option<(String, String, usize)>,
    folder_choices: &[(String, String)],
) -> (Option<FolderTreeEvent>, bool) {
    if depth >= MAX_RENDER_DEPTH {
        ui.horizontal(|ui| {
            ui.add_space(INDENT_PER_LEVEL * depth as f32);
            ui.weak("…(deep)");
        });
        return (None, false);
    }

    let mut event: Option<FolderTreeEvent> = None;
    let mut spinner_active = false;

    // The folder row: indent, disclosure triangle, color swatch, label.
    let row = ui.horizontal(|ui| {
        ui.add_space(INDENT_PER_LEVEL * depth as f32);

        // Disclosure triangle (▸ collapsed / ▾ expanded). Clicking toggles expand/collapse.
        let tri = if node.expanded { "▾" } else { "▸" };
        let tri_resp = ui.add(egui::Label::new(tri).sense(Sense::click()));

        // Color swatch (12x12 rect) painted with the folder color or the neutral theme token. The swatch
        // is an addressable Button and opens the same explicitly controlled picker as the context menu.
        let swatch_color = node.color.unwrap_or(palette.border_strong);
        let (sw_rect, sw_resp) = ui.allocate_exact_size(Vec2::splat(SWATCH_SIZE), Sense::click());
        if ui.is_rect_visible(sw_rect) {
            ui.painter().rect_filled(sw_rect, 2.0, swatch_color);
            ui.painter().rect_stroke(
                sw_rect,
                2.0,
                Stroke::new(1.0_f32, palette.border),
                egui::StrokeKind::Inside,
            );
        }
        emit_color_swatch_accesskit(
            ui,
            sw_resp.id,
            &color_author_id(&node.folder_id),
            &format!(
                "Folder color for {}: {}",
                node.title,
                color_to_hex(swatch_color)
            ),
            swatch_color,
        );

        // The folder label (clicking it opens/navigates; the row id is the addressable TreeItem).
        let label_resp = ui.selectable_label(
            selected_folder_id == Some(node.folder_id.as_str()),
            &node.title,
        );

        // Inline child-load spinner while a genuine fetch is in flight (AC2 / no-perpetual-spinner rule).
        if node.loading {
            ui.spinner();
            spinner_active = true;
        }

        (tri_resp, sw_resp, label_resp)
    });
    let (tri_resp, sw_resp, label_resp) = row.inner;

    let label_clicked = label_resp.clicked();
    let open_action_generation = if label_clicked {
        // Apply the causal generation in the same frame that consumes the AccessKit Click. The host
        // owns the real child fetch and binds this exact generation to its request sequence.
        Some(node.begin_open_action())
    } else {
        None
    };

    let selected = selected_folder_id == Some(node.folder_id.as_str());
    let completion_state = match node.expansion_completion_state(selected) {
        FolderOpenCompletion::Ready => ClickCompletionState::Ready,
        FolderOpenCompletion::Pending => ClickCompletionState::Pending,
        FolderOpenCompletion::Applied => ClickCompletionState::Applied,
    };
    let completion_context = format!("{workspace_id}:{}", node.folder_id);
    let completion_value = serialize_same_target_click_completion(
        "folder-open-expand",
        &completion_context,
        node.expansion_generation(),
        completion_state,
    );

    // Emit the folder row's AccessKit TreeItem (Expand/Collapse + Click default-open). The row id is the
    // label's id (the primary clickable), carrying the stable `folder-tree.node.{id}` author_id.
    emit_tree_item_accesskit(
        ui,
        label_resp.id,
        &node.folder_id,
        &node.title,
        node.expanded,
        true,
        completion_value,
    );
    emit_folder_status_accesskit(
        ui,
        label_resp.id.with("status"),
        workspace_id,
        node,
        selected,
    );

    let current_color = node.color.unwrap_or(palette.border_strong);
    let row_picker_id = label_resp.id.with("folder-color-picker");

    // ── Row right-click => context-menu "Change color" => color picker (AC4) ────────────────────────
    // This is the contract-required row secondary-click path. It opens an in-process egui picker; the
    // picker returns a real picked color, which emits the ChangeColor event the host persists via
    // `PATCH /loom/folders/{id}`. The actionable swatch below opens this same controlled popup id.
    let mut open_row_picker = false;
    egui::Popup::context_menu(&label_resp).show(|ui| {
        if ui.button("Change color").clicked() {
            open_row_picker = true;
        }
        if ui.button("New subfolder").clicked() {
            *create_draft = Some((Some(node.folder_id.clone()), String::new()));
            ui.close();
        }
        if ui.button("Rename").clicked() {
            *rename_draft = Some((node.folder_id.clone(), node.title.clone()));
            ui.close();
        }
        if ui.button("Move to root").clicked() {
            event = Some(FolderTreeEvent::MoveFolder {
                folder_id: node.folder_id.clone(),
                parent_folder_id: None,
                sort_order: None,
            });
            ui.close();
        }
        let move_under = ui.menu_button("Move under", |ui| {
            for (target_id, target_title) in folder_choices {
                if target_id == &node.folder_id {
                    continue;
                }
                let target = ui.button(target_title);
                emit_button_accesskit(
                    ui,
                    target.id,
                    &move_target_author_id(&node.folder_id, target_id),
                    &format!("Move {} under {}", node.title, target_title),
                );
                if target.clicked() {
                    event = Some(FolderTreeEvent::MoveFolder {
                        folder_id: node.folder_id.clone(),
                        parent_folder_id: Some(target_id.clone()),
                        sort_order: None,
                    });
                    ui.close();
                }
            }
        });
        emit_button_accesskit(
            ui,
            move_under.response.id,
            MOVE_UNDER_MENU_AUTHOR_ID,
            "Move under",
        );
        if ui.button("Delete").clicked() {
            *delete_draft = Some((
                node.folder_id.clone(),
                node.title.clone(),
                node.folder_count().saturating_sub(1),
            ));
            ui.close();
        }
    });
    if open_row_picker || sw_resp.clicked() {
        egui::Popup::open_id(ui.ctx(), row_picker_id);
        ui.ctx().request_repaint();
    }
    if let Some(picked) = color_picker_popup_for_id(row_picker_id, &label_resp, current_color) {
        event = Some(FolderTreeEvent::ChangeColor {
            folder_id: node.folder_id.clone(),
            color: picked,
        });
    }

    // ── Disclosure / label clicks ────────────────────────────────────────────────────────────────────
    let accessible_expand = ui.input(|input| {
        input.has_accesskit_action_request(label_resp.id, accesskit::Action::Expand)
    });
    let accessible_collapse = ui.input(|input| {
        input.has_accesskit_action_request(label_resp.id, accesskit::Action::Collapse)
    });
    let requested_expanded = if accessible_expand && !node.expanded {
        Some(true)
    } else if accessible_collapse && node.expanded {
        Some(false)
    } else if tri_resp.clicked() {
        Some(!node.expanded)
    } else {
        None
    };
    if let Some(expanded) = requested_expanded {
        node.expanded = expanded;
        if node.expanded {
            // First expand with no cached blocks => host should lazy-fetch (AC2). Re-expanding a folder
            // whose blocks are already cached does NOT re-fetch (the lazy-load caching rule).
            if node.child_blocks.is_none() {
                event = Some(FolderTreeEvent::ExpandFolder {
                    folder_id: node.folder_id.clone(),
                });
            }
        } else {
            node.collapse();
            event = Some(FolderTreeEvent::CollapseFolder {
                folder_id: node.folder_id.clone(),
            });
        }
    }
    if let Some(action_generation) = open_action_generation {
        event = Some(FolderTreeEvent::OpenFolder {
            folder_id: node.folder_id.clone(),
            action_generation,
        });
    }

    // ── Children (only when expanded) ────────────────────────────────────────────────────────────────
    if node.expanded {
        // Child folders first (the structural tree), then leaf blocks (the lazy membership).
        for child in &mut node.child_folders {
            let (ev, spin) = render_folder(
                child,
                ui,
                palette,
                depth + 1,
                workspace_id,
                selected_folder_id,
                create_draft,
                rename_draft,
                delete_draft,
                folder_choices,
            );
            if ev.is_some() {
                event = ev;
            }
            spinner_active |= spin;
        }
        let mut reveal_children = node.child_blocks.is_some() && node.take_reveal_children_once();
        if let Some(blocks) = &node.child_blocks {
            if blocks.is_empty() {
                let empty = ui.horizontal(|ui| {
                    ui.add_space(
                        INDENT_PER_LEVEL * (depth + 1) as f32 + LEAF_PARENT_CHROME_RESERVE,
                    );
                    ui.weak("(empty)");
                });
                if reveal_children {
                    empty.response.scroll_to_me(None);
                    reveal_children = false;
                }
            }
            for leaf in blocks {
                let (leaf_event, leaf_response) = render_leaf(leaf, ui, palette, depth + 1);
                if reveal_children {
                    leaf_response.scroll_to_me(None);
                    reveal_children = false;
                }
                if let Some(ev) = leaf_event {
                    event = Some(ev);
                }
            }
        }
    }

    (event, spinner_active)
}

/// Render one leaf block row (a folder member). Click => OpenBlock(block_id) (AC5). The row is an
/// addressable TreeItem with the stable `folder-tree.node.{block_id}` author_id.
fn render_leaf(
    leaf: &LeafBlock,
    ui: &mut egui::Ui,
    palette: &HsPalette,
    depth: usize,
) -> (Option<FolderTreeEvent>, egui::Response) {
    let mut event = None;
    let resp = ui
        .horizontal(|ui| {
            ui.add_space(INDENT_PER_LEVEL * depth as f32 + LEAF_PARENT_CHROME_RESERVE);
            // Content-type icon char + title (the contract's "block title + content_type icon
            // character").
            let label = format!("{} {}", leaf.icon(), leaf.title);
            ui.add(
                egui::Label::new(egui::RichText::new(label).color(palette.text))
                    .sense(Sense::click()),
            )
        })
        .inner;
    // Leaf TreeItem (no Expand/Collapse — leaves do not expand). Click is the default open action.
    emit_tree_item_accesskit(ui, resp.id, &leaf.block_id, &leaf.title, false, false, None);
    if resp.clicked() {
        event = Some(FolderTreeEvent::OpenBlock {
            block_id: leaf.block_id.clone(),
        });
    }
    (event, resp)
}

/// The controlled color picker opened by the row's "Change color" context action or its color-swatch
/// button. It uses a stable popup id so model/kittest drivers can then pick a deterministic color.
fn color_picker_popup_for_id(
    popup_id: egui::Id,
    anchor: &egui::Response,
    current: Color32,
) -> Option<Color32> {
    let mut picked = None;
    // `Popup::menu(anchor)` toggles on `anchor.clicked()`, so it would incorrectly open this picker on a
    // normal primary folder-row Open. Memory state changes only through explicit `Popup::open_id` from
    // the context-menu command or swatch button above (or close behavior below).
    egui::Popup::from_response(anchor)
        .id(popup_id)
        .open_memory(None)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            picked = color_picker_contents(ui, current);
            if picked.is_some() {
                egui::Popup::close_id(ui.ctx(), popup_id);
            }
        });
    picked
}

/// Color-picking contents for the row context-menu path.
fn color_picker_contents(ui: &mut egui::Ui, current: Color32) -> Option<Color32> {
    let mut picked = None;
    ui.label("Pick a folder color");
    ui.horizontal_wrapped(|ui| {
        for (label, hex) in COLOR_PICKER_SWATCHES {
            let color = parse_hex_color(hex).unwrap_or(current);
            let button = egui::Button::new(format!("{label} {hex}")).fill(color);
            if ui.add(button).clicked() {
                picked = Some(color);
            }
        }
    });
    ui.separator();
    let mut srgba = current;
    let resp = egui::color_picker::color_edit_button_srgba(
        ui,
        &mut srgba,
        egui::color_picker::Alpha::Opaque,
    );
    if resp.changed() && srgba != current {
        picked = Some(srgba);
    }
    picked
}

/// Emit a generic button's live AccessKit node (Role::Button + Action::Click + author_id) so a swarm
/// agent can address it by stable id (HBR-SWARM).
fn emit_button_accesskit(ui: &egui::Ui, id: egui::Id, author_id: &str, label: &str) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.add_action(accesskit::Action::Click);
    });
}

/// Emit the contract-required actionable color-swatch button. It reports its current RGBA value and
/// advertises Click truthfully because the primary action opens the explicitly controlled picker.
fn emit_color_swatch_accesskit(
    ui: &egui::Ui,
    id: egui::Id,
    author_id: &str,
    label: &str,
    color: Color32,
) {
    let author = author_id.to_owned();
    let label = label.to_owned();
    let rgba = u32::from_be_bytes(color.to_array());
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Button);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_color_value(rgba);
        node.add_action(accesskit::Action::Click);
    });
}

/// Emit a tree row's live AccessKit node: Role::TreeItem, label = title, author_id =
/// `folder-tree.node.{sanitized_id}` (AC6 / HBR-SWARM). Folder rows carry Expand/Collapse actions plus
/// the default Click (open); leaf rows carry only Click. `id` is the row's primary clickable egui id.
fn emit_tree_item_accesskit(
    ui: &egui::Ui,
    id: egui::Id,
    raw_id: &str,
    title: &str,
    expanded: bool,
    expandable: bool,
    value: Option<String>,
) {
    let author = node_author_id(raw_id);
    let label = title.to_owned();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::TreeItem);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        if let Some(value) = &value {
            node.set_value(value.clone());
        }
        node.add_action(accesskit::Action::Click);
        // Expand/Collapse so a swarm agent can drive the tree open/closed by id (the RISK-4 control:
        // manual rows expose these; CollapsingHeader would not).
        if expandable {
            if expanded {
                node.add_action(accesskit::Action::Collapse);
            } else {
                node.add_action(accesskit::Action::Expand);
            }
        }
    });
}

fn emit_folder_status_accesskit(
    ui: &egui::Ui,
    id: egui::Id,
    workspace_id: &str,
    folder: &FolderNode,
    selected: bool,
) {
    let author = status_author_id(&folder.folder_id);
    let label = format!("Folder state for {}", folder.title);
    let child_state = if folder.loading {
        "loading"
    } else if folder.child_error.is_some() {
        "failed"
    } else if folder.child_blocks.is_some() {
        "loaded"
    } else {
        "not_requested"
    };
    let value = serde_json::json!({
        "schema": "handshake.folder-expansion-status/v1",
        "workspace_id": workspace_id,
        "folder_id": &folder.folder_id,
        "generation": folder.expansion_generation(),
        "request_sequence": folder.expansion_request_sequence(),
        "terminal_request_sequence": folder.expansion_terminal_sequence(),
        "selected": selected,
        "expanded": folder.expanded,
        "loading": folder.loading,
        "child_state": child_state,
        "child_count": folder.child_blocks.as_ref().map(Vec::len),
        "error": folder.child_error.as_deref(),
    })
    .to_string();
    ui.ctx().accesskit_node_builder(id, move |node| {
        node.set_role(accesskit::Role::Status);
        node.set_author_id(author.clone());
        node.set_label(label.clone());
        node.set_value(value.clone());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat folder-row fixture: 2 roots, one with a nested child, colors mixed.
    fn rows_fixture() -> Vec<FolderRow> {
        vec![
            FolderRow::new("folder-001", None, "Projects", Some("#ff0000".to_owned())),
            FolderRow::new("folder-002", None, "Archive", None),
            FolderRow::new(
                "folder-003",
                Some("folder-001".to_owned()),
                "Subproject",
                Some("00ff00".to_owned()),
            ),
        ]
    }

    /// Hex parse: valid #rrggbb and bare rrggbb both parse; malformed yields None (graceful neutral
    /// swatch, never a panic).
    #[test]
    fn parse_hex_color_handles_valid_and_invalid() {
        assert_eq!(
            parse_hex_color("#ff0000"),
            Some(Color32::from_rgba_unmultiplied(255, 0, 0, 255))
        );
        assert_eq!(
            parse_hex_color("00ff00"),
            Some(Color32::from_rgba_unmultiplied(0, 255, 0, 255))
        );
        assert_eq!(
            parse_hex_color("#0000FF"),
            Some(Color32::from_rgba_unmultiplied(0, 0, 255, 255))
        );
        assert_eq!(
            parse_hex_color("#fff"),
            Some(Color32::from_rgba_unmultiplied(255, 255, 255, 255)),
            "short CSS hex expands each nibble"
        );
        assert_eq!(
            parse_hex_color("red"),
            Some(Color32::from_rgba_unmultiplied(255, 0, 0, 255)),
            "defined CSS named colors render"
        );
        assert_eq!(parse_hex_color("ReD"), parse_hex_color("red"));
        assert_eq!(parse_hex_color("chartreuse"), None, "unknown named token");
        assert_eq!(parse_hex_color("#gggggg"), None, "non-hex");
        assert_eq!(parse_hex_color(""), None, "empty");
        assert_eq!(parse_hex_color("#ff00000"), None, "too long");

        let tree = build_tree(&[
            FolderRow::new(
                "unknown",
                None,
                "Unknown token",
                Some("chartreuse".to_owned()),
            ),
            FolderRow::new("valid", None, "Valid token", Some("red".to_owned())),
        ]);
        assert_eq!(
            tree.len(),
            2,
            "one unknown token must not reject the folder list"
        );
        assert_eq!(
            tree[0].color, None,
            "only the unknown row degrades to neutral"
        );
        assert_eq!(tree[1].color, parse_hex_color("red"));
    }

    /// Round-trip: color -> hex -> color is stable for opaque colors (the recolor PATCH body shape).
    #[test]
    fn color_hex_round_trip() {
        let c = Color32::from_rgba_unmultiplied(0x12, 0xab, 0xcd, 255);
        assert_eq!(color_to_hex(c), "#12abcd");
        assert_eq!(parse_hex_color(&color_to_hex(c)), Some(c));
    }

    /// AccessKit author_ids sanitize raw ids to `[a-z0-9-]` (RISK-4 / MC-4).
    #[test]
    fn author_ids_are_sanitized() {
        let node = node_author_id("ws:1/folder 7#x");
        assert!(node.starts_with(NODE_AUTHOR_ID_PREFIX));
        let suffix = &node[NODE_AUTHOR_ID_PREFIX.len()..];
        assert!(
            suffix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "node author_id suffix must be [a-z0-9-]; got '{suffix}'"
        );
        let color = color_author_id("a/b:c");
        assert!(color.starts_with(COLOR_AUTHOR_ID_PREFIX));
        assert_ne!(
            node_author_id("a/b"),
            node_author_id("a:b"),
            "distinct raw ids must not alias after sanitization"
        );
        assert_ne!(
            move_target_author_id("a/b", "target"),
            move_target_author_id("a:b", "target"),
            "move targets retain exact source identity"
        );
    }

    /// find_folder_mut reaches a nested node so the host can install lazy-loaded blocks after a fetch.
    #[test]
    fn find_folder_mut_reaches_nested() {
        let mut tree = LoomFolderTree::new("ws-1");
        tree.set_folders(&rows_fixture());
        let nested = tree
            .find_folder_mut("folder-003")
            .expect("nested folder-003 reachable");
        nested.child_blocks = Some(vec![LeafBlock::new("blk-1", "Block 1", "note")]);
        assert!(
            tree.find_folder_mut("folder-003")
                .unwrap()
                .child_blocks
                .is_some(),
            "installed child blocks persist on the node"
        );
    }

    /// folder_count over the forest matches the row count (no nodes dropped for an acyclic tree).
    #[test]
    fn folder_count_matches_rows() {
        let tree = LoomFolderTree {
            workspace_id: "ws".to_owned(),
            root_nodes: build_tree(&rows_fixture()),
            loading: false,
            error: None,
            ..LoomFolderTree::default()
        };
        assert_eq!(
            tree.folder_count(),
            3,
            "all three acyclic rows present in the forest"
        );
    }

    /// Empty rows => empty forest (drives the AC7 "No folders" empty state).
    #[test]
    fn empty_rows_empty_forest() {
        let tree = build_tree(&[]);
        assert!(tree.is_empty());
    }

    /// Leaf icon is content-type aware and never empty.
    #[test]
    fn leaf_icon_is_content_type_aware() {
        assert_eq!(LeafBlock::new("b", "B", "note").icon(), '📝');
        assert_eq!(LeafBlock::new("b", "B", "file").icon(), '📄');
        assert_eq!(LeafBlock::new("b", "B", "zzz").icon(), '•');
    }
}
