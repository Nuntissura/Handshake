//! Top-level application menu bar for the native work surface (WP-KERNEL-011 MT-015).
//!
//! ## What this provides (no-context model navigation — HBR-MAN)
//!
//! A classic horizontal application menu bar — `FILE`, `EDIT`, `VIEW`, `GO`, `RUN`, `HELP` — rendered
//! as the VERY FIRST top panel in the shell, above the title bar / module switcher (MT-012), the
//! project-tab strip (MT-011), and the tiled work surface. It is DISTINCT from:
//! - the module switcher ([`crate::module_switcher`], MT-012), which switches a pane's MODULE, and
//! - the project (workspace) tabs ([`crate::project_tabs`], MT-011), which switch whole projects, and
//! - the per-pane document tab bar ([`crate::tab_bar`], MT-007), which switches documents in a pane.
//!
//! Clicking a leaf item returns a [`MenuBarAction`] (one variant per leaf). The shell ([`crate::app`])
//! matches on that action and routes it into the SAME state-mutation helpers the keyboard shortcuts and
//! toolbar buttons use — the menu bar itself never mutates app state and never pops a foreground dialog
//! (HBR-QUIET): it only reports the leaf the user triggered this frame.
//!
//! ## Menu structure and each action's wiring status
//!
//! Leaf items whose target already exists in the shell are ENABLED and dispatched. Leaf items whose
//! target is a FUTURE microtask (a document/editor model, a file drawer, a terminal panel) are rendered
//! DISABLED with a disclosed reason in their tooltip — they are NOT fake-enabled. The action enum still
//! carries every leaf so the wiring is mechanical once the target MT lands.
//!
//! ```text
//! FILE
//!   New Document            DISABLED (needs the document model — future MT)
//!   Open Workspace…         DISABLED (needs the workspace picker — future MT)
//!   ──────
//!   Save            Ctrl+S  DISABLED (needs the document model — future MT)
//!   Save All                DISABLED (needs the document model — future MT)
//!   ──────
//!   Close Tab               ENABLED  -> CloseActiveTab (closes the active pane's active tab)
//!   Quit                    ENABLED  -> QuitApp (sends the viewport Close command)
//! EDIT  (all DISABLED — needs the editor surface, a future MT)
//!   Undo Ctrl+Z / Redo Ctrl+Shift+Z / Cut Ctrl+X / Copy Ctrl+C / Paste Ctrl+V
//!   Find / Replace Ctrl+F / Find in All Documents Ctrl+Shift+F
//! VIEW
//!   Theme: Dark / Theme: Light  (✔ on current) ENABLED -> ToggleTheme (flat checkmark items)
//!   ──────
//!   View Mode: NSFW / View Mode: SFW (✔ on current) ENABLED -> ToggleViewMode (flat checkmark items)
//!   ──────
//!   Toggle Project Drawer   ENABLED -> ToggleProjectDrawer (left activity rail, MT-014)
//!   Toggle File Drawer      DISABLED (no native file drawer yet — future MT)
//!   Toggle Bottom Panel     ENABLED -> ToggleBottomPanel (bottom stash drawer, MT-014)
//!   ──────
//!   Reset Layout            ENABLED -> ResetLayout (confirm-then-reset; MC7)
//! GO
//!   Quick Switcher  Ctrl+P        ENABLED -> OpenQuickSwitcher (sets quick_switcher_open; UI = MT-016)
//!   Command Palette Ctrl+Shift+P  ENABLED -> OpenCommandPalette (sets command_palette_open; UI = MT-016)
//!   ──────
//!   Go to Next Pane               ENABLED -> FocusNextPane
//!   Go to Previous Pane           ENABLED -> FocusPrevPane
//! RUN
//!   Open Swarm Board        ENABLED -> OpenSwarmBoard (opens the Swarm surface on the active pane)
//!   Open Inference Lab      ENABLED -> NavigateToTab("inference-lab")
//!   Open Flight Recorder    ENABLED -> NavigateToTab("flight-recorder")
//!   Launch Model Session in Workspace Folder
//!                           ENABLED -> OpenModelSessionLaunch (compact in-app launch dialog)
//!   Open Terminal in Workspace Folder
//!                           -> surfaces EndpointMissing until native HTTP terminal route exists
//! HELP
//!   Open User Manual        ENABLED -> NavigateToTab("user-manual")
//!   Open Settings…          ENABLED -> OpenSettings (sets settings_open; UI = MT-018)
//!   ──────
//!   About Handshake         ENABLED -> ShowAbout (sets about_open)
//! ```
//!
//! ## Stable AccessKit ids (out-of-process steering — HBR-VIS)
//!
//! The menu count is FIXED at six, so — like the module switcher — each TOP-LEVEL menu button gets a
//! fixed `NodeId` in a dedicated fresh band ([`MENU_BAR_NODE_ID_BASE`] = 92..=97), strictly below the
//! pane id base (100) and disjoint from every other declared identity (theme toggle 10, chrome 20/21,
//! dividers 30/31, scrollbar rails 40..43, project-tab strip 50, module buttons 51..56, tab-bar
//! containers 60..63, merge-back 64..67, pane locks 70..73, pane titles 74..77, left rail 80..88,
//! project tree 89, quick links 90, bookmarks 91). The collision test in
//! [`crate::accessibility::registry`] proves the disjointness across the whole declared set; the six
//! menu ids are registered in `DECLARED_IDENTITIES` there.
//!
//! Each top-level menu button is a real `Role::MenuItem` node (egui derives `Action::Click`/
//! `Action::Focus` from its `Sense::click()`) carrying an `author_id` equal to its [`MenuId::author_id`]
//! (e.g. `menu-file`). Individual LEAF items inside an OPEN menu are dynamic (they exist only while the
//! menu is open) and are addressed by an `egui::Id` derived from their stable author_id STRING
//! (`menu.{menu}.{leaf}`), in egui's hashed id space — the same pattern the dynamic per-tab nodes use —
//! so they are not enumerated in the fixed-band `DECLARED_IDENTITIES`. Every leaf still carries an
//! author_id so it is discoverable + clickable out-of-process and never trips the MT-025
//! interactive-naming gate.
//!
//! ## Swarm-accessible action registry (HBR-SWARM)
//!
//! [`SWARM_ACCESSIBLE_ACTIONS`] is the const list of action author-keys a swarm agent may dispatch
//! (overlay-opening + navigation actions). This MT only declares the list; wiring it into the broader
//! swarm action registry is a later MT's job.

use egui::accesskit;

/// Fixed AccessKit/egui `NodeId` of the FIRST top-level menu button (`FILE`). The six menu buttons
/// occupy the FRESH band 92..=97: above the MT-014 FIX-A bookmarks container (91), strictly below the
/// pane id base (100). Each button's id is `MENU_BAR_NODE_ID_BASE + index_in_MENU_DEFINITIONS`. A
/// fixed-value `egui::Id` (`from_high_entropy_bits`) yields a fixed `NodeId` across frames + process
/// restarts — the same convention the theme toggle, chrome, dividers, and module switcher use.
pub const MENU_BAR_NODE_ID_BASE: u64 = 92;

/// WP-KERNEL-012 MT-052 GO-menu editor-navigation leaf author_ids (the exact ids the MT contract names
/// in step 6 so a swarm agent invokes navigation deterministically). These are LEAF items (dynamic —
/// they exist only while the GO menu is open), so they live in egui's hashed id space addressed by these
/// stable strings, NOT a fixed-band `NodeId` (the same pattern as every other leaf item). Each is a
/// `Role::MenuItem` node carrying its author_id, so it is discoverable + passes the MT-025 gate even
/// while rendered disabled-until-E11.
pub const GO_NEXT_DIAGNOSTIC_AUTHOR_ID: &str = "menu-go-next-diagnostic";
pub const GO_PREV_DIAGNOSTIC_AUTHOR_ID: &str = "menu-go-prev-diagnostic";
pub const GO_BACK_AUTHOR_ID: &str = "menu-go-back";
pub const GO_FORWARD_AUTHOR_ID: &str = "menu-go-forward";

/// WP-KERNEL-012 MT-053 GO-menu in-file "Go to Symbol in File…" leaf author_id (the exact id the MT
/// contract's menu wiring names so a swarm agent can SEE the item). Like the MT-052 editor-navigation
/// leaves it is a `Role::MenuItem` LEAF (dynamic — exists only while the GO menu is open), addressed by
/// this stable string in egui's hashed id space, rendered DISABLED with a disclosed reason until the
/// editor is host-mounted (E11 MT-069). Once live the host wires it to the SAME `open_symbol_palette`
/// entry point the Ctrl+Shift+O keybind reaches (AC-005); until then the LIVE path is the keybind.
pub const GO_SYMBOL_IN_FILE_AUTHOR_ID: &str = "menu-go-symbol-in-file";

/// WP-KERNEL-012 MT-101 RUN-menu launch leaf. It opens the compact in-app model-session launch dialog;
/// the actual reachable backend path is `POST /jobs`, and direct repo-folder spawn remains IPC-only.
pub const MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID: &str = "menu.run.model-session-launch";

/// The disclosed reason shown on the disabled MT-052 GO-menu editor-navigation leaves until the editor is
/// host-mounted (E11 MT-069), matching the MT-050 disabled-until-mounted precedent.
pub const MENU_GO_EDITOR_DISABLED_REASON: &str =
    "Needs the live code editor (host-mounted in E11 MT-069)";

/// A top-level menu in the menu bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuId {
    File,
    Edit,
    View,
    Go,
    Run,
    Help,
}

impl MenuId {
    /// The display title rendered on the menu-bar button.
    pub const fn title(self) -> &'static str {
        match self {
            MenuId::File => "FILE",
            MenuId::Edit => "EDIT",
            MenuId::View => "VIEW",
            MenuId::Go => "GO",
            MenuId::Run => "RUN",
            MenuId::Help => "HELP",
        }
    }

    /// Stable out-of-process author_id for the top-level menu button (kebab-case, `menu-` prefixed).
    pub const fn author_id(self) -> &'static str {
        match self {
            MenuId::File => "menu-file",
            MenuId::Edit => "menu-edit",
            MenuId::View => "menu-view",
            MenuId::Go => "menu-go",
            MenuId::Run => "menu-run",
            MenuId::Help => "menu-help",
        }
    }

    /// The `Alt+<letter>` access-key (mnemonic) that OPENS this menu — the underlined first letter of
    /// the title, matching the classic Windows menu-bar convention and the React mnemonics
    /// (`F`ile / `E`dit / `V`iew / `G`o / `R`un / `H`elp). Pressing `Alt+<this>` programmatically opens
    /// the menu's popup (see [`handle_menu_mnemonics`]); thereafter the open menu is keyboard-navigable
    /// (arrows + Enter) via egui's native menu popup focus handling.
    pub const fn mnemonic_key(self) -> egui::Key {
        match self {
            MenuId::File => egui::Key::F,
            MenuId::Edit => egui::Key::E,
            MenuId::View => egui::Key::V,
            MenuId::Go => egui::Key::G,
            MenuId::Run => egui::Key::R,
            MenuId::Help => egui::Key::H,
        }
    }
}

/// The fixed `egui::Id` of a top-level menu BUTTON, derived purely from its index in
/// [`MENU_DEFINITIONS`] (so it is identical whether computed at render time inside [`MenuBar::menu`] or
/// ahead of render in [`handle_menu_mnemonics`]). A fixed-value `Id` (`from_high_entropy_bits`) yields
/// a stable `NodeId` across frames + process restarts — the same convention every other fixed-band node
/// in this crate uses.
fn menu_button_id(index: usize) -> egui::Id {
    // SAFETY: `from_high_entropy_bits` only requires the value to be high-entropy enough to avoid
    // accidental collisions; these ids share the documented disjoint fixed band (92..=97) proven by the
    // accessibility registry collision test, so they never collide with another declared identity.
    unsafe { egui::Id::from_high_entropy_bits(MENU_BAR_NODE_ID_BASE + index as u64) }
}

/// The memory id of a top-level menu's POPUP, matching what [`egui::Popup::menu`] stores for that
/// button (`button_id.with("popup")`, see [`egui::Popup::default_response_id`]). Opening THIS id via
/// [`egui::Popup::open_id`] makes the corresponding [`MenuBar::menu`] popup render this frame, because
/// `Popup::menu` reads its open-state from egui memory.
fn menu_popup_id(index: usize) -> egui::Id {
    menu_button_id(index).with("popup")
}

/// Handle the `Alt+<letter>` menu mnemonics (AC2). Call this ONCE per frame, BEFORE the menu bar panel
/// is shown, with the same [`egui::Context`] the bar renders into. For each menu it consumes a pressed
/// `Alt+<mnemonic>` chord and opens that menu's popup via egui memory ([`egui::Popup::open_id`]); the
/// popup then renders open this frame and is keyboard-navigable thereafter. Returns the [`MenuId`] that
/// was opened (if any) so the caller can request a repaint.
///
/// This is a REAL keyboard path, not a comment: egui 0.33 exposes `Popup::open_id`, and `Popup::menu`
/// stores its open-state in egui memory under `menu_popup_id`, so writing that memory before the bar
/// renders is exactly how an out-of-band opener drives a native menu popup. `consume_key` swallows the
/// chord so it does not also reach the global keymap handler (red-team R3 — no double-fire).
pub fn handle_menu_mnemonics(ctx: &egui::Context) -> Option<MenuId> {
    let mut opened = None;
    for (index, menu) in MENU_DEFINITIONS.iter().enumerate() {
        let pressed = ctx.input_mut(|i| i.consume_key(egui::Modifiers::ALT, menu.mnemonic_key()));
        if pressed {
            // Open this menu's popup (closing any other open popup) so it renders open this frame.
            egui::Popup::open_id(ctx, menu_popup_id(index));
            opened = Some(*menu);
        }
    }
    opened
}

/// The six top-level menus in display order. The fixed-count array drives both rendering and the
/// fixed-band id assignment (`MENU_BAR_NODE_ID_BASE + index`).
pub const MENU_DEFINITIONS: [MenuId; 6] = [
    MenuId::File,
    MenuId::Edit,
    MenuId::View,
    MenuId::Go,
    MenuId::Run,
    MenuId::Help,
];

/// The typed action a leaf menu item dispatches. Returned by [`MenuBar::show`] when a leaf is clicked
/// this frame (`None` otherwise). The shell ([`crate::app`]) matches EXHAUSTIVELY on this enum and
/// routes each variant into the existing state-mutation path — the menu bar never mutates state itself.
///
/// Variants marked "(disabled in MT-015)" correspond to leaf items whose target surface does not yet
/// exist in the native shell; those leaves are rendered DISABLED, so these variants are part of the
/// exhaustive contract (the compiler enforces the match) but are not produced by a click yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuBarAction {
    // FILE
    NewDocument,         // disabled in MT-015 (needs document model)
    OpenWorkspacePicker, // disabled in MT-015 (needs workspace picker)
    SaveActiveDocument,  // disabled in MT-015 (needs document model)
    SaveAllDocuments,    // disabled in MT-015 (needs document model)
    CloseActiveTab,
    QuitApp,
    // EDIT (all disabled in MT-015 — needs the editor surface)
    EditorUndo,
    EditorRedo,
    EditCut,
    EditCopy,
    EditPaste,
    OpenFindReplace,
    OpenWorkspaceSearch,
    // VIEW
    ToggleTheme,
    ToggleViewMode,
    ToggleProjectDrawer,
    ToggleFileDrawer, // disabled in MT-015 (no native file drawer yet)
    ToggleBottomPanel,
    ResetLayout,
    // GO
    OpenQuickSwitcher,
    OpenCommandPalette,
    FocusNextPane,
    FocusPrevPane,
    // RUN
    OpenSwarmBoard,
    /// Navigate the active pane to a named tab/surface (the React `PaneTabId` string).
    NavigateToTab(String),
    OpenModelSessionLaunch,
    OpenTerminal, // surfaces typed EndpointMissing until a native HTTP terminal route exists.
    // HELP
    OpenSettings,
    ShowAbout,
    /// WP-KERNEL-012 MT-069 (E11 menu wire-up): dispatch the editor FILE/EDIT command identified by the
    /// carried stable command id (e.g. [`crate::command_registry::CMD_EDITOR_FILE_SAVE`]) through the ONE
    /// shared shell editor-command dispatcher (`app.rs::dispatch_editor_command`). The menu handler routes
    /// by COMMAND ID only — it contains no inline editor logic — so menu-driven and palette-driven editor
    /// actions share one path (RISK-001). The leaf is enabled only when the command's live predicate holds
    /// (an editor pane is available; Undo only when `can_undo`; Paste only when the clipboard has content).
    EditorCommand(&'static str),
}

/// Action author-keys a swarm agent may dispatch out-of-process (HBR-SWARM). These are the overlay-
/// opening + navigation actions relevant to autonomous agents; destructive/document actions are
/// deliberately excluded. This MT only declares the list — wiring it into the broader swarm action
/// registry is a later MT.
pub const SWARM_ACCESSIBLE_ACTIONS: &[&str] = &[
    "menu.go.command-palette",
    "menu.go.quick-switcher",
    "menu.run.swarm-board",
    MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID,
    "menu.run.inference-lab",
    "menu.run.flight-recorder",
    "menu.help.user-manual",
    "menu.help.settings",
    // WP-KERNEL-012 MT-052 GO-menu editor navigation (discoverable by swarm agents; they dispatch once
    // the editor is host-mounted in E11).
    GO_NEXT_DIAGNOSTIC_AUTHOR_ID,
    GO_PREV_DIAGNOSTIC_AUTHOR_ID,
    GO_BACK_AUTHOR_ID,
    GO_FORWARD_AUTHOR_ID,
];

/// Read-only view of the live shell state the menu bar needs to render checkmarks + enable/disable
/// leaves. The menu bar takes this by value so it never holds a `&mut` to the app while egui's menu
/// closures borrow `ui` (sidesteps the red-team R1 accumulator-in-closure borrow problem).
#[derive(Debug, Clone, Copy)]
pub struct MenuBarState {
    /// True when the active base theme is Dark (drives the Theme submenu checkmark).
    pub theme_is_dark: bool,
    /// True when the active view mode is NSFW (drives the View Mode submenu checkmark).
    pub view_mode_is_nsfw: bool,
    /// True when the left activity rail (project drawer) is open (drives its checkmark).
    pub project_drawer_open: bool,
    /// True when the bottom stash drawer is open (drives its checkmark).
    pub bottom_drawer_open: bool,
    /// True when at least one pane has an active tab that can be closed (enables FILE > Close Tab).
    pub has_active_tab: bool,
    /// WP-KERNEL-012 MT-069: true when an editor pane is the focusable/active target (a CodeSymbol code
    /// editor or LoomWikiPage Notes editor is mounted). The live ENABLE PREDICATE for the FILE/EDIT editor
    /// menu items WP-011 shipped disabled and MT-079 host-mounted: New/Save/Save All/Save As/Export, Cut/
    /// Copy/Select All/Find/Replace/Find in Files/Toggle Comment/Format Document. When `false` those items
    /// render DISABLED (honest, not fake-enabled).
    pub editor_available: bool,
    /// WP-KERNEL-012 MT-069: true when the MT-035 unified-undo scope reports an undoable action for the
    /// focused pane (or the cross-pane ring) — the live ENABLE PREDICATE for EDIT > Undo (VS Code semantics:
    /// Undo enabled only when there is something to undo).
    pub editor_can_undo: bool,
    /// WP-KERNEL-012 MT-069: true when the MT-035 unified-undo scope reports a redoable action for the
    /// focused pane — the live ENABLE PREDICATE for EDIT > Redo.
    pub editor_can_redo: bool,
    /// WP-KERNEL-012 MT-069: true when the MT-031 shared clipboard holds a consumable payload — the live
    /// ENABLE PREDICATE for EDIT > Paste (VS Code enables Paste only when the clipboard has content).
    pub editor_can_paste: bool,
    /// MT-069 REMEDIATION: true when the mounted code panel's jump history can navigate BACK — the live
    /// ENABLE PREDICATE for GO > Back (`can_navigate_back` on the mounted panel, VS Code semantics).
    pub editor_can_nav_back: bool,
    /// MT-069 REMEDIATION: true when the mounted code panel's jump history can navigate FORWARD — the
    /// live ENABLE PREDICATE for GO > Forward (`can_navigate_forward` on the mounted panel).
    pub editor_can_nav_forward: bool,
}

/// Stateless menu-bar widget. Construct per frame from a [`MenuBarState`] and call [`MenuBar::show`].
pub struct MenuBar {
    state: MenuBarState,
}

impl MenuBar {
    /// Build the per-frame menu bar from the live shell state.
    pub fn new(state: MenuBarState) -> Self {
        Self { state }
    }

    /// Render the full menu bar and return the [`MenuBarAction`] the user triggered this frame
    /// (`None` if nothing was clicked). Uses egui's native [`egui::MenuBar`] + [`egui::Popup::menu`]
    /// primitives (NOT hand-rolled popup geometry). The action is accumulated into a local `Option`
    /// declared BEFORE the bar so the nested menu closures only need a captured `&mut` to it
    /// (red-team MC1).
    pub fn show(&self, ui: &mut egui::Ui) -> Option<MenuBarAction> {
        let mut action: Option<MenuBarAction> = None;
        egui::MenuBar::new().ui(ui, |ui| {
            for (index, menu) in MENU_DEFINITIONS.iter().enumerate() {
                self.menu(ui, *menu, index, &mut action);
            }
        });
        action
    }

    /// Render one top-level menu button (pinned to its fixed AccessKit id) and its dropdown.
    fn menu(
        &self,
        ui: &mut egui::Ui,
        menu: MenuId,
        index: usize,
        action: &mut Option<MenuBarAction>,
    ) {
        // Fixed-value Id -> fixed AccessKit NodeId in the 92..=97 band (disjoint by construction; the
        // registry collision test proves it). We build the toggle button + popup ourselves (rather than
        // `ui.menu_button`, whose id is auto-allocated) so the button lands on this exact stable id.
        // Derived via the shared `menu_button_id` so the Alt+letter mnemonic opener computes the SAME
        // popup id (`button_id.with("popup")`) ahead of render — keeping the keyboard path in lockstep.
        let button_id = menu_button_id(index);

        let galley = ui.painter().layout_no_wrap(
            menu.title().to_owned(),
            egui::FontId::proportional(13.0),
            ui.visuals().text_color(),
        );
        let pad_x = 8.0;
        let pad_y = 4.0;
        let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        // Interact at the FIXED button_id so the Response, its widget_info, the AccessKit bounding box,
        // and the author_id all land on the SAME node (mirrors the module-switcher id discipline).
        let response = ui.interact(rect, button_id, egui::Sense::click());

        // The button is "open" when its popup is currently showing, so we can paint the open highlight.
        let popup_open =
            egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let bg = if popup_open {
                ui.visuals().widgets.open.bg_fill
            } else if response.hovered() {
                visuals.bg_fill
            } else {
                egui::Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, 4.0, bg);
            let text_pos = egui::pos2(rect.left() + pad_x, rect.center().y - galley.size().y * 0.5);
            ui.painter().galley(text_pos, galley, visuals.text_color());
        }

        // AccessKit: egui derived Action::Click/Action::Focus from Sense::click(); set the MenuItem role
        // + label + the stable author_id on the SAME node so an out-of-process agent addresses the menu
        // by `menu-file`.. and the MT-025 interactive-naming gate passes.
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), menu.title())
        });
        ui.ctx().accesskit_node_builder(button_id, |node| {
            node.set_role(accesskit::Role::MenuItem);
            node.set_author_id(menu.author_id().to_owned());
            node.set_label(menu.title().to_owned());
        });

        // Standard egui menu popup, toggled by the button response (open on click, close on click of an
        // item via the default CloseOnClick behavior). This is egui's own menu primitive — the same one
        // `ui.menu_button` uses internally — so the dropdown feels native and closes correctly (R6).
        egui::Popup::menu(&response).show(|ui| {
            ui.set_min_width(220.0);
            self.menu_items(ui, menu, action);
        });
    }

    /// Render the leaf items for one menu into the open popup.
    fn menu_items(&self, ui: &mut egui::Ui, menu: MenuId, action: &mut Option<MenuBarAction>) {
        match menu {
            MenuId::File => {
                // WP-KERNEL-012 MT-069 (E11): the editor FILE items WP-011 shipped disabled are now LIVE
                // (MT-079 host-mounted the editors). Each dispatches its real editor command by id through
                // the shared shell dispatcher; enabled only when an editor pane is the focusable target. The
                // WP-011 AccessKit author_ids (`menu.file.*`) are REUSED (flip to enabled, no new id minted).
                let ed = self.state.editor_available;
                self.item(
                    ui,
                    "menu.file.new-document",
                    "New Document",
                    Some("Ctrl+N"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FILE_NEW),
                    action,
                );
                self.disabled_item(
                    ui,
                    "menu.file.open-workspace",
                    "Open Workspace…",
                    None,
                    "Needs the workspace picker (future MT)",
                );
                ui.separator();
                self.item(
                    ui,
                    "menu.file.save",
                    "Save",
                    Some("Ctrl+S"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FILE_SAVE),
                    action,
                );
                self.item(
                    ui,
                    "menu.file.save-all",
                    "Save All",
                    Some("Ctrl+K S"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FILE_SAVE_ALL),
                    action,
                );
                self.item(
                    ui,
                    "menu.file.save-as",
                    "Save As…",
                    Some("Ctrl+Shift+S"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FILE_SAVE_AS),
                    action,
                );
                ui.separator();
                // Export Document: HTML / Markdown / Text / JSON — each routes to the MT-020 editor save/
                // export path by its stable command id.
                self.item(
                    ui,
                    "menu.file.export-html",
                    "Export Document: HTML",
                    None,
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_FILE_EXPORT_HTML,
                    ),
                    action,
                );
                self.item(
                    ui,
                    "menu.file.export-md",
                    "Export Document: Markdown",
                    None,
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_FILE_EXPORT_MD,
                    ),
                    action,
                );
                self.item(
                    ui,
                    "menu.file.export-txt",
                    "Export Document: Text",
                    None,
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_FILE_EXPORT_TXT,
                    ),
                    action,
                );
                self.item(
                    ui,
                    "menu.file.export-json",
                    "Export Document: JSON",
                    None,
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_FILE_EXPORT_JSON,
                    ),
                    action,
                );
                ui.separator();
                self.item(
                    ui,
                    "menu.file.close-tab",
                    "Close Tab",
                    None,
                    self.state.has_active_tab,
                    MenuBarAction::CloseActiveTab,
                    action,
                );
                self.item(
                    ui,
                    "menu.file.quit",
                    "Quit",
                    None,
                    true,
                    MenuBarAction::QuitApp,
                    action,
                );
            }
            MenuId::Edit => {
                // WP-KERNEL-012 MT-069 (E11): the editor EDIT items WP-011 shipped disabled are now LIVE
                // (MT-079 host-mounted the editors). Each dispatches its real editor command by id through
                // the shared shell dispatcher; the WP-011 AccessKit author_ids (`menu.edit.*`) are REUSED
                // (flip to enabled, no new id minted). Undo/Redo route to the SAME MT-035 unified-undo stack
                // the keyboard path uses; Cut/Copy/Paste/Select All to the MT-031 shared clipboard; Find/
                // Replace to the focused editor's find family. Enable predicates are LIVE (RISK-006): Undo
                // only when `can_undo`, Redo only when `can_redo`, Paste only when the clipboard has content.
                let ed = self.state.editor_available;
                self.item(
                    ui,
                    "menu.edit.undo",
                    "Undo",
                    Some("Ctrl+Z"),
                    self.state.editor_can_undo,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_EDIT_UNDO),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.redo",
                    "Redo",
                    Some("Ctrl+Shift+Z"),
                    self.state.editor_can_redo,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_EDIT_REDO),
                    action,
                );
                ui.separator();
                self.item(
                    ui,
                    "menu.edit.cut",
                    "Cut",
                    Some("Ctrl+X"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_EDIT_CUT),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.copy",
                    "Copy",
                    Some("Ctrl+C"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_EDIT_COPY),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.paste",
                    "Paste",
                    Some("Ctrl+V"),
                    self.state.editor_can_paste,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_EDIT_PASTE),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.select-all",
                    "Select All",
                    Some("Ctrl+A"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_EDIT_SELECT_ALL,
                    ),
                    action,
                );
                ui.separator();
                // WP-KERNEL-012 MT-051 / MT-050: Toggle Comment + Format Document. The Format Document leaf
                // KEEPS its MT-050 AccessKit author_id (`FORMAT_DOCUMENT_MENU_AUTHOR_ID`); it now dispatches
                // the real editor.edit.formatDocument command when an editor pane is the target (RISK-007:
                // no new menu infra, the existing leaf flips to enabled).
                self.item(
                    ui,
                    "menu.edit.toggle-comment",
                    "Toggle Comment",
                    Some("Ctrl+/"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_EDIT_TOGGLE_COMMENT,
                    ),
                    action,
                );
                self.item(
                    ui,
                    crate::code_editor::FORMAT_DOCUMENT_MENU_AUTHOR_ID,
                    "Format Document",
                    Some("Alt+Shift+F"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_EDIT_FORMAT_DOCUMENT,
                    ),
                    action,
                );
                ui.separator();
                self.item(
                    ui,
                    "menu.edit.find-replace",
                    "Find",
                    Some("Ctrl+F"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FIND_FIND),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.replace",
                    "Replace",
                    Some("Ctrl+H"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FIND_REPLACE),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.find-all",
                    "Find in Files",
                    Some("Ctrl+Shift+F"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FIND_IN_FILES),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.replace-all",
                    "Replace in Files",
                    Some("Ctrl+Shift+H"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_REPLACE_IN_FILES,
                    ),
                    action,
                );
                ui.separator();
                // Command Palette + Quick Switcher are also reachable from EDIT (AC-002 lists them here);
                // they open the ONE WP-011 palette / switcher — always available (no editor needed).
                self.item(
                    ui,
                    "menu.edit.command-palette",
                    "Command Palette",
                    Some("Ctrl+Shift+P"),
                    true,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_WORKBENCH_SHOW_COMMANDS,
                    ),
                    action,
                );
                self.item(
                    ui,
                    "menu.edit.quick-switcher",
                    "Quick Switcher",
                    Some("Ctrl+P"),
                    true,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_WORKBENCH_QUICK_OPEN),
                    action,
                );
            }
            MenuId::View => {
                // Theme: two FLAT checkmark items (a check on the currently active theme; selectable_label
                // draws the native check), matching AC5's "VIEW > Theme: Dark / Theme: Light". Clicking
                // the NON-active option toggles; clicking the already-active one is a no-op (no action
                // emitted) so the theme never flickers (R4 handled by the same-frame apply on dispatch).
                if self.check_item(
                    ui,
                    "menu.view.theme-dark",
                    "Theme: Dark",
                    self.state.theme_is_dark,
                ) && !self.state.theme_is_dark
                {
                    *action = Some(MenuBarAction::ToggleTheme);
                    ui.close();
                }
                if self.check_item(
                    ui,
                    "menu.view.theme-light",
                    "Theme: Light",
                    !self.state.theme_is_dark,
                ) && self.state.theme_is_dark
                {
                    *action = Some(MenuBarAction::ToggleTheme);
                    ui.close();
                }
                ui.separator();
                // View Mode: two FLAT checkmark items (a check on the active mode), matching AC's
                // "VIEW > View Mode: NSFW / SFW".
                if self.check_item(
                    ui,
                    "menu.view.mode-nsfw",
                    "View Mode: NSFW",
                    self.state.view_mode_is_nsfw,
                ) && !self.state.view_mode_is_nsfw
                {
                    *action = Some(MenuBarAction::ToggleViewMode);
                    ui.close();
                }
                if self.check_item(
                    ui,
                    "menu.view.mode-sfw",
                    "View Mode: SFW",
                    !self.state.view_mode_is_nsfw,
                ) && self.state.view_mode_is_nsfw
                {
                    *action = Some(MenuBarAction::ToggleViewMode);
                    ui.close();
                }
                ui.separator();
                // Drawer toggles show a checkmark for the current open/closed state (a check = open).
                if self.check_item(
                    ui,
                    "menu.view.toggle-project-drawer",
                    "Toggle Project Drawer",
                    self.state.project_drawer_open,
                ) {
                    *action = Some(MenuBarAction::ToggleProjectDrawer);
                    ui.close();
                }
                self.disabled_item(
                    ui,
                    "menu.view.toggle-file-drawer",
                    "Toggle File Drawer",
                    None,
                    "No native file drawer yet (future MT)",
                );
                if self.check_item(
                    ui,
                    "menu.view.toggle-bottom-panel",
                    "Toggle Bottom Panel",
                    self.state.bottom_drawer_open,
                ) {
                    *action = Some(MenuBarAction::ToggleBottomPanel);
                    ui.close();
                }
                ui.separator();
                self.item(
                    ui,
                    "menu.view.reset-layout",
                    "Reset Layout…",
                    None,
                    true,
                    MenuBarAction::ResetLayout,
                    action,
                );
            }
            MenuId::Go => {
                self.item(
                    ui,
                    "menu.go.quick-switcher",
                    "Quick Switcher",
                    Some("Ctrl+P"),
                    true,
                    MenuBarAction::OpenQuickSwitcher,
                    action,
                );
                self.item(
                    ui,
                    "menu.go.command-palette",
                    "Command Palette",
                    Some("Ctrl+Shift+P"),
                    true,
                    MenuBarAction::OpenCommandPalette,
                    action,
                );
                ui.separator();
                self.item(
                    ui,
                    "menu.go.next-pane",
                    "Go to Next Pane",
                    None,
                    true,
                    MenuBarAction::FocusNextPane,
                    action,
                );
                self.item(
                    ui,
                    "menu.go.prev-pane",
                    "Go to Previous Pane",
                    None,
                    true,
                    MenuBarAction::FocusPrevPane,
                    action,
                );
                ui.separator();
                // WP-KERNEL-012 MT-052 / MT-069 REMEDIATION: the editor navigation leaves are LIVE against
                // the MOUNTED code panel. Each dispatches its stable command id through the ONE shell
                // editor-command dispatcher, which routes to the panel's own `dispatch_action` — the SAME
                // path the F8/Shift+F8/Alt+Left/Alt+Right keymap chords reach (RISK-007: no forked nav
                // logic). Back/Forward reflect the live `can_navigate_back`/`can_navigate_forward` jump
                // history state (no fake-enable — MT-050 precedent). Author_ids are unchanged.
                let ed = self.state.editor_available;
                self.item(
                    ui,
                    GO_NEXT_DIAGNOSTIC_AUTHOR_ID,
                    "Go to Next Problem",
                    Some("F8"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_GO_NEXT_DIAGNOSTIC,
                    ),
                    action,
                );
                self.item(
                    ui,
                    GO_PREV_DIAGNOSTIC_AUTHOR_ID,
                    "Go to Previous Problem",
                    Some("Shift+F8"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_GO_PREV_DIAGNOSTIC,
                    ),
                    action,
                );
                self.item(
                    ui,
                    GO_BACK_AUTHOR_ID,
                    "Back",
                    Some("Alt+Left"),
                    ed && self.state.editor_can_nav_back,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_GO_BACK),
                    action,
                );
                self.item(
                    ui,
                    GO_FORWARD_AUTHOR_ID,
                    "Forward",
                    Some("Alt+Right"),
                    ed && self.state.editor_can_nav_forward,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_GO_FORWARD),
                    action,
                );
                // WP-KERNEL-012 MT-053 / MT-069 REMEDIATION: in-file Go to Symbol is LIVE — dispatches to
                // the SAME open_symbol_palette entry point the Ctrl+Shift+O keybind reaches (AC-005).
                // DISTINCT from the Quick Switcher leaf above (global, Ctrl+P).
                self.item(
                    ui,
                    GO_SYMBOL_IN_FILE_AUTHOR_ID,
                    "Go to Symbol in File…",
                    Some("Ctrl+Shift+O"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_GO_SYMBOL_IN_FILE,
                    ),
                    action,
                );
                ui.separator();
                // WP-KERNEL-012 MT-069 REMEDIATION: the four code-navigation GO items are LIVE — their
                // owning code-nav shell commands are now REGISTERED against the mounted panel
                // (`dispatch_editor_command` routes each id to the panel's `dispatch_action` /
                // quick-switcher). The author_ids are the stable command ids (unchanged), so a swarm
                // agent addresses the same node it saw as pending before.
                self.item(
                    ui,
                    crate::command_registry::CMD_EDITOR_GO_TO_DEFINITION,
                    "Go to Definition",
                    Some("F12"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_GO_TO_DEFINITION,
                    ),
                    action,
                );
                self.item(
                    ui,
                    crate::command_registry::CMD_EDITOR_GO_TO_REFERENCES,
                    "Go to References",
                    Some("Shift+F12"),
                    ed,
                    MenuBarAction::EditorCommand(
                        crate::command_registry::CMD_EDITOR_GO_TO_REFERENCES,
                    ),
                    action,
                );
                self.item(
                    ui,
                    crate::command_registry::CMD_EDITOR_GO_TO_SYMBOL,
                    "Go to Symbol in Workspace…",
                    Some("Ctrl+T"),
                    true,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_GO_TO_SYMBOL),
                    action,
                );
                self.item(
                    ui,
                    crate::command_registry::CMD_EDITOR_GO_TO_LINE,
                    "Go to Line…",
                    Some("Ctrl+G"),
                    ed,
                    MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_GO_TO_LINE),
                    action,
                );
            }
            MenuId::Run => {
                self.item(
                    ui,
                    "menu.run.swarm-board",
                    "Open Swarm Board",
                    None,
                    true,
                    MenuBarAction::OpenSwarmBoard,
                    action,
                );
                self.item(
                    ui,
                    "menu.run.inference-lab",
                    "Open Inference Lab",
                    None,
                    true,
                    MenuBarAction::NavigateToTab("inference-lab".to_owned()),
                    action,
                );
                self.item(
                    ui,
                    "menu.run.flight-recorder",
                    "Open Flight Recorder",
                    None,
                    true,
                    MenuBarAction::NavigateToTab("flight-recorder".to_owned()),
                    action,
                );
                self.item(
                    ui,
                    MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID,
                    "Launch Model Session in Workspace Folder",
                    None,
                    true,
                    MenuBarAction::OpenModelSessionLaunch,
                    action,
                );
                self.item(
                    ui,
                    "menu.run.terminal",
                    "Open Terminal in Workspace Folder",
                    None,
                    true,
                    MenuBarAction::OpenTerminal,
                    action,
                );
            }
            MenuId::Help => {
                self.item(
                    ui,
                    "menu.help.user-manual",
                    "Open User Manual",
                    None,
                    true,
                    MenuBarAction::NavigateToTab("user-manual".to_owned()),
                    action,
                );
                self.item(
                    ui,
                    "menu.help.settings",
                    "Open Settings…",
                    None,
                    true,
                    MenuBarAction::OpenSettings,
                    action,
                );
                ui.separator();
                self.item(
                    ui,
                    "menu.help.about",
                    "About Handshake",
                    None,
                    true,
                    MenuBarAction::ShowAbout,
                    action,
                );
            }
        }
    }

    /// Render one enabled leaf item with an optional right-aligned shortcut hint. Sets the `action`
    /// accumulator + closes the menu when clicked. The button is pinned to a stable `egui::Id` derived
    /// from its author_id string (hashed id space, like the dynamic per-tab nodes) and carries that
    /// author_id + `Role::MenuItem` so it is discoverable/clickable out-of-process and passes the
    /// MT-025 interactive-naming gate.
    #[allow(clippy::too_many_arguments)]
    fn item(
        &self,
        ui: &mut egui::Ui,
        author_id: &str,
        label: &str,
        shortcut: Option<&str>,
        enabled: bool,
        emit: MenuBarAction,
        action: &mut Option<MenuBarAction>,
    ) {
        if !enabled {
            // An enabled-call with a runtime-false condition (e.g. Close Tab with no tab) still renders
            // the leaf greyed so its presence is stable (AC2) and its reason readable.
            self.disabled_item(
                ui,
                author_id,
                label,
                shortcut,
                "Unavailable in the current state",
            );
            return;
        }
        let mut button = egui::Button::new(label);
        if let Some(s) = shortcut {
            button = button.shortcut_text(s);
        }
        let response = ui.add(button.min_size(egui::vec2(ui.available_width(), 0.0)));
        Self::name_node(ui, response.id, author_id, label);
        if response.clicked() {
            *action = Some(emit);
            ui.close();
        }
    }

    /// Render one DISABLED leaf item with a disclosed reason tooltip. Still emits an addressable
    /// `Role::MenuItem` node (carrying its author_id) so an out-of-process agent can SEE the item and
    /// read that it is disabled — it just cannot be clicked into an action (no fake-enable).
    fn disabled_item(
        &self,
        ui: &mut egui::Ui,
        author_id: &str,
        label: &str,
        shortcut: Option<&str>,
        reason: &str,
    ) {
        let mut button = egui::Button::new(label);
        if let Some(s) = shortcut {
            button = button.shortcut_text(s);
        }
        let response = ui
            .add_enabled(
                false,
                button.min_size(egui::vec2(ui.available_width(), 0.0)),
            )
            .on_disabled_hover_text(reason);
        Self::name_node(ui, response.id, author_id, label);
    }

    /// Render a checkmark leaf via `selectable_label` (egui draws the native check when `checked`).
    /// Returns `true` if it was clicked this frame. Carries an addressable `Role::MenuItem` node.
    fn check_item(&self, ui: &mut egui::Ui, author_id: &str, label: &str, checked: bool) -> bool {
        let response = ui.selectable_label(checked, label);
        Self::name_node(ui, response.id, author_id, label);
        response.clicked()
    }

    /// Attach the stable author_id + `Role::MenuItem` to a leaf's live node. `widget_node_id` is the
    /// real egui-allocated response id of the leaf button (so the node we enrich is exactly the one
    /// egui emitted into the frame's accessibility tree). Leaf items are DYNAMIC — they exist only
    /// while their menu is open — so they live in egui's hashed id space (like the per-tab nodes) and
    /// are addressed out-of-process by their stable `author_id`, not a fixed-band NodeId.
    fn name_node(ui: &mut egui::Ui, widget_node_id: egui::Id, author_id: &str, label: &str) {
        let author_id = author_id.to_owned();
        let label = label.to_owned();
        ui.ctx()
            .accesskit_node_builder(widget_node_id, move |node| {
                node.set_role(accesskit::Role::MenuItem);
                node.set_author_id(author_id);
                node.set_label(label);
            });
    }
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-071 (E11 — VS Code status-bar parity): editor file-metadata segments.
//
// Five interactive, editor-aware segments that bring VS-Code-class file-metadata controls to the
// shell's status/menu bar: LanguageMode, Eol, Indent, Encoding, RenderWhitespace (left-to-right). They
// reuse the WP-011 STATUS-BAR SEGMENT rendering pattern (a galley + a fixed-id interactive node + a
// stable author_id — the same shape `app::status_bar_segment` / `top_menu_bar` menu buttons use; NO new
// status bar, NO new segment TYPE — RISK-001/MC-001/AC-008) and the WP-011 SEGMENT CONTEXT-MENU infra
// ([`crate::context_menu`] + [`crate::context_menu_surfaces::status_bar_context_items`]) for the
// right-click menus. Each segment reads the ACTIVE code document's metadata
// ([`EditorMetaSegmentState::from_panel`]); when no code-editor document is active the whole cluster
// HIDES (returns early — RISK-006/AC-005), never stale data.
//
// State LIVES on the MT-010 doc model ([`crate::code_editor::panel::CodeEditorPanel`]) — these segments
// only render it + emit a typed [`EditorSegmentAction`] the host applies back onto that model, so the
// metadata persists across re-render + re-focus and the MT-001 draw + Tab-key path reads it
// (RISK-004/MC-004). The segment never mutates the document itself (the same "report the action, the
// shell applies it" discipline the menu bar uses).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

use crate::code_editor::file_meta::{Encoding, Eol, IndentKind, IndentStyle};
use crate::code_editor::language_mode::{DetectionSource, LanguageId};

/// The five editor-metadata segments, left-to-right in the right cluster. The order is the MT-071
/// contract order (LanguageMode, Eol, Indent, Encoding, RenderWhitespace).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorSegment {
    LanguageMode,
    Eol,
    Indent,
    Encoding,
    RenderWhitespace,
}

impl EditorSegment {
    /// The five segments in render order.
    pub const ALL: [EditorSegment; 5] = [
        EditorSegment::LanguageMode,
        EditorSegment::Eol,
        EditorSegment::Indent,
        EditorSegment::Encoding,
        EditorSegment::RenderWhitespace,
    ];

    /// The stable AccessKit author_id a swarm agent addresses the segment by (the EXACT ids the MT-071
    /// contract names: `status-bar-language-mode` / `-eol` / `-indent` / `-encoding` /
    /// `-render-whitespace`, each role=Button).
    pub const fn author_id(self) -> &'static str {
        match self {
            EditorSegment::LanguageMode => "status-bar-language-mode",
            EditorSegment::Eol => "status-bar-eol",
            EditorSegment::Indent => "status-bar-indent",
            EditorSegment::Encoding => "status-bar-encoding",
            EditorSegment::RenderWhitespace => "status-bar-render-whitespace",
        }
    }

    /// The stable segment id used by the WP-011 status-bar context-menu state (Copy / Hide / Refresh).
    const fn segment_id(self) -> &'static str {
        match self {
            EditorSegment::LanguageMode => "language-mode",
            EditorSegment::Eol => "eol",
            EditorSegment::Indent => "indent",
            EditorSegment::Encoding => "encoding",
            EditorSegment::RenderWhitespace => "render-whitespace",
        }
    }

    /// The author_id of one picker LIST ITEM (`status-bar-{segment}-item-{value}`, role=ListItem). The
    /// `value` is the stable id for the option (the language family id, `lf`/`crlf`, the indent key, the
    /// encoding id, or `on`/`off`).
    fn item_author_id(self, value: &str) -> String {
        format!("{}-item-{value}", self.author_id())
    }
}

/// A typed action a segment click / picker selection / context-menu confirm produces. The host applies
/// it to the active [`CodeEditorPanel`](crate::code_editor::panel::CodeEditorPanel) AFTER the picker /
/// menu closes (so the closure never holds a `&mut` to the doc model). Mirrors the menu bar's
/// "report the action, the shell applies it" discipline (RISK-004 — the doc model owns the mutation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorSegmentAction {
    /// Set the user language override to this family id (the picker selection).
    SetLanguage(LanguageId),
    /// Convert the document's line endings to this EOL as one undo step.
    ConvertEol(Eol),
    /// Set the active indent style (tabs-vs-spaces + size); flips the Tab-key behavior.
    SetIndent(IndentStyle),
    /// Reopen the document re-decoded under this encoding (in-process; no backend).
    ReopenWithEncoding(Encoding),
    /// Toggle render-whitespace to this value.
    SetRenderWhitespace(bool),
    /// Copy the segment's display text to the clipboard (the WP-011 status-bar Copy action).
    CopySegmentText(String),
}

/// A read-only snapshot of the active code document's file metadata, built from the live
/// [`CodeEditorPanel`](crate::code_editor::panel::CodeEditorPanel) each frame. The segments render from
/// this and never hold a reference to the panel while egui's picker/menu closures borrow `ui`. When the
/// focused pane is NOT a code editor, the host passes `None` and the whole cluster hides (AC-005).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorMetaSegmentState {
    /// The resolved language label (e.g. `Rust`) + its detection source (drives the `(override)`/`(auto)`
    /// hint).
    pub language_label: String,
    pub language_source: DetectionSource,
    /// The picker option list (family ids), sourced from the MT-001 registry (AC-001).
    pub available_languages: Vec<LanguageId>,
    pub eol: Eol,
    pub indent: IndentStyle,
    pub encoding: Encoding,
    pub render_whitespace: bool,
}

impl EditorMetaSegmentState {
    /// Build the snapshot from a live code-editor panel (the MT-010 doc model). Reads the resolved
    /// language, EOL, indent, encoding, and whitespace flag off the panel — the SINGLE source of truth
    /// (no parallel store — RISK-004).
    pub fn from_panel(panel: &crate::code_editor::panel::CodeEditorPanel) -> Self {
        let detection = panel.resolved_language();
        Self {
            language_label: detection.detected.display_label(),
            language_source: detection.source,
            available_languages: crate::code_editor::language_mode::available_languages(),
            eol: panel.eol(),
            indent: panel.indent_style(),
            encoding: panel.encoding(),
            render_whitespace: panel.render_whitespace(),
        }
    }

    /// The compact display label for one segment (what the status-bar text shows).
    fn segment_label(&self, segment: EditorSegment) -> String {
        match segment {
            EditorSegment::LanguageMode => {
                let hint = match self.language_source {
                    DetectionSource::UserOverride => " (override)",
                    _ => "",
                };
                format!("{}{hint}", self.language_label)
            }
            EditorSegment::Eol => self.eol.label().to_owned(),
            EditorSegment::Indent => self.indent.label(),
            EditorSegment::Encoding => self.encoding.label().to_owned(),
            EditorSegment::RenderWhitespace => {
                if self.render_whitespace {
                    "Whitespace ✓".to_owned()
                } else {
                    "Whitespace".to_owned()
                }
            }
        }
    }
}

/// The stateless editor-metadata status-bar segment cluster. Construct from an optional
/// [`EditorMetaSegmentState`] (None = no code document active -> the cluster HIDES, AC-005) and call
/// [`show`](EditorStatusSegments::show). The widget reuses the WP-011 segment rendering pattern + the
/// WP-011 context-menu infra; it introduces NO new status bar and NO new segment type (AC-008).
pub struct EditorStatusSegments {
    state: Option<EditorMetaSegmentState>,
}

impl EditorStatusSegments {
    /// Build the per-frame cluster. `state == None` hides every segment (no code-editor document
    /// active).
    pub fn new(state: Option<EditorMetaSegmentState>) -> Self {
        Self { state }
    }

    /// Render the five segments left-to-right and return the typed action the user triggered this frame
    /// (`None` if nothing fired or no code document is active). When `state` is `None`, renders NOTHING
    /// (returns early — RISK-006/AC-005), so the bar carries no stale editor metadata when a non-code
    /// pane is focused.
    pub fn show(&self, ui: &mut egui::Ui) -> Option<EditorSegmentAction> {
        let state = self.state.as_ref()?; // hide all five when no code-editor document is active.
        let mut action: Option<EditorSegmentAction> = None;
        // Render in the contract order; a separator between segments mirrors the WP-011 segment styling.
        // Each segment owns its own `Popup::menu(&response)` picker (the WP-011 segment pattern); no outer
        // `MenuBar` wrapper is needed (the pickers are independent per-segment popups, like the WP-011
        // status-bar segment context menus, not a shared menu-bar row).
        for (idx, segment) in EditorSegment::ALL.iter().enumerate() {
            if idx > 0 {
                ui.separator();
            }
            self.segment(ui, *segment, state, &mut action);
        }
        action
    }

    /// Render ONE segment as a WP-011-style status-bar segment: a galley + an interactive node carrying
    /// the stable author_id (`Role::Button`), a LEFT-click picker/toggle, and a RIGHT-click context menu
    /// (the WP-011 segment context-menu infra). Accumulates the typed action into `action`.
    fn segment(
        &self,
        ui: &mut egui::Ui,
        segment: EditorSegment,
        state: &EditorMetaSegmentState,
        action: &mut Option<EditorSegmentAction>,
    ) {
        let label = state.segment_label(segment);

        if matches!(segment, EditorSegment::RenderWhitespace) {
            // RenderWhitespace is a DIRECT toggle (no picker): a plain clickable segment button.
            let response = ui.add(egui::Button::new(&label).frame(false));
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
            });
            Self::name_segment_node(ui, response.id, segment.author_id(), &label);
            if response.clicked() {
                *action = Some(EditorSegmentAction::SetRenderWhitespace(
                    !state.render_whitespace,
                ));
            }
            // RIGHT-click context menu (WP-011 infra + the toggle quick action).
            if let Some(menu_action) = self.segment_context_menu(segment, state, &response) {
                *action = Some(menu_action);
            }
            return;
        }

        // The other four segments open a PICKER on LEFT-click, built with the EXACT WP-011 `MenuBar::menu`
        // pattern: a galley-sized rect + `ui.interact(..., Sense::click())` (so the ONE interactive node
        // carries the stable author_id) + `egui::Popup::menu(&response)` (egui's own menu primitive — opens
        // on the segment's primary click, closes on item-click / Escape / click-outside). This is the same
        // open path the menu-bar buttons use, proven click-driveable out-of-process.
        let galley = ui.painter().layout_no_wrap(
            label.clone(),
            egui::FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
        let pad = egui::vec2(6.0, 3.0);
        let desired = galley.size() + pad * 2.0;
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let response = ui.interact(
            rect,
            ui.id().with(("editor-segment", segment.author_id())),
            egui::Sense::click(),
        );
        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            let popup_open =
                egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response));
            if popup_open || response.hovered() {
                ui.painter().rect_filled(rect, 3.0, visuals.bg_fill);
            }
            let text_pos = egui::pos2(rect.left() + pad.x, rect.center().y - galley.size().y * 0.5);
            ui.painter().galley(text_pos, galley, visuals.text_color());
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
        });
        Self::name_segment_node(ui, response.id, segment.author_id(), &label);

        // A STABLE picker popup id derived from the segment author_id (not the auto response id), so a
        // left-click opens it AND an out-of-process driver / test can open it deterministically via
        // `Popup::open_id(ctx, EditorStatusSegments::picker_popup_id(segment))` (the same programmatic-open
        // seam the WP-011 context-menu `request_open` provides).
        let popup_id = Self::picker_popup_id(segment);
        if response.clicked() {
            egui::Popup::open_id(&response.ctx, popup_id);
        }
        egui::Popup::menu(&response).id(popup_id).show(|ui| {
            ui.set_min_width(160.0);
            // Tag the popup container as a List for the AccessKit dump (the picker is a list of options —
            // the MT names role=List on the container).
            let container_id = ui.id();
            ui.ctx().accesskit_node_builder(container_id, |node| {
                node.set_role(accesskit::Role::List);
                node.set_author_id(format!("{}-picker", segment.author_id()));
            });
            self.picker_rows(ui, segment, state, action);
        });

        // RIGHT-click context menu: the WP-011 status-bar segment menu (Copy / Hide / Open Panel /
        // Refresh) PLUS the segment-specific quick actions (e.g. Convert to LF/CRLF on EOL). Reuses
        // `context_menu_surfaces::status_bar_context_items` (no hand-rolled menu — AC-008).
        if let Some(menu_action) = self.segment_context_menu(segment, state, &response) {
            *action = Some(menu_action);
        }
    }

    /// The STABLE egui popup id of a segment's LEFT-click picker, derived from the segment's author_id
    /// (process-stable, not the per-frame auto response id). A left-click opens it; an out-of-process
    /// driver or test opens it deterministically via `egui::Popup::open_id(ctx, picker_popup_id(segment))`
    /// — the programmatic-open seam (mirrors the WP-011 context-menu `request_open`).
    pub fn picker_popup_id(segment: EditorSegment) -> egui::Id {
        egui::Id::new(("editor-segment-picker", segment.author_id()))
    }

    /// Render the picker LIST ROWS for a segment inside the open picker popup. Each row carries
    /// `status-bar-{segment}-item-{value}` (`Role::ListItem`). A confirmed row sets `action`.
    fn picker_rows(
        &self,
        ui: &mut egui::Ui,
        segment: EditorSegment,
        state: &EditorMetaSegmentState,
        action: &mut Option<EditorSegmentAction>,
    ) {
        match segment {
            EditorSegment::LanguageMode => {
                for lang in &state.available_languages {
                    let selected = lang.display_label() == state.language_label;
                    if self.picker_row(ui, segment, lang.as_str(), &lang.display_label(), selected)
                    {
                        *action = Some(EditorSegmentAction::SetLanguage(lang.clone()));
                        ui.close();
                    }
                }
            }
            EditorSegment::Eol => {
                for eol in [Eol::Lf, Eol::Crlf] {
                    let selected = eol == state.eol;
                    if self.picker_row(ui, segment, eol_value(eol), eol.label(), selected) {
                        *action = Some(EditorSegmentAction::ConvertEol(eol));
                        ui.close();
                    }
                }
            }
            EditorSegment::Indent => {
                // Tabs + Spaces 2/4/8 (the VS Code indent picker set).
                let tabs = IndentStyle {
                    kind: IndentKind::Tabs,
                    size: state.indent.size.max(1),
                };
                let tab_selected = matches!(state.indent.kind, IndentKind::Tabs);
                if self.picker_row(ui, segment, "tabs", "Indent Using Tabs", tab_selected) {
                    *action = Some(EditorSegmentAction::SetIndent(tabs));
                    ui.close();
                }
                for size in [2usize, 4, 8] {
                    let style = IndentStyle {
                        kind: IndentKind::Spaces,
                        size,
                    };
                    let selected = matches!(state.indent.kind, IndentKind::Spaces)
                        && state.indent.size == size;
                    let value = format!("spaces-{size}");
                    let label = format!("Indent Using {size} Spaces");
                    if self.picker_row(ui, segment, &value, &label, selected) {
                        *action = Some(EditorSegmentAction::SetIndent(style));
                        ui.close();
                    }
                }
            }
            EditorSegment::Encoding => {
                for enc in Encoding::ALL {
                    let selected = enc == state.encoding;
                    if self.picker_row(ui, segment, enc.id(), enc.label(), selected) {
                        *action = Some(EditorSegmentAction::ReopenWithEncoding(enc));
                        ui.close();
                    }
                }
            }
            EditorSegment::RenderWhitespace => {} // toggle, no picker
        }
    }

    /// Render one picker row as a `selectable_label` carrying the stable `status-bar-{segment}-item-{value}`
    /// author_id (`Role::ListItem`). Returns `true` when clicked this frame.
    fn picker_row(
        &self,
        ui: &mut egui::Ui,
        segment: EditorSegment,
        value: &str,
        label: &str,
        selected: bool,
    ) -> bool {
        let response = ui.selectable_label(selected, label);
        let author = segment.item_author_id(value);
        let label_owned = label.to_owned();
        ui.ctx().accesskit_node_builder(response.id, move |node| {
            node.set_role(accesskit::Role::ListItem);
            node.set_author_id(author);
            node.set_label(label_owned);
        });
        response.clicked()
    }

    /// Build + show the segment's RIGHT-click context menu, reusing the WP-011 status-bar segment items
    /// (Copy / Hide / Open Panel / Refresh) plus the segment-specific quick actions the MT names
    /// (Convert to LF/CRLF on EOL; Convert indentation + Change tab size on Indent; Reopen with Encoding
    /// on Encoding; toggle on RenderWhitespace). Returns the confirmed typed action this frame.
    fn segment_context_menu(
        &self,
        segment: EditorSegment,
        state: &EditorMetaSegmentState,
        response: &egui::Response,
    ) -> Option<EditorSegmentAction> {
        use crate::context_menu::{ContextMenu, ContextMenuItem};
        use crate::context_menu_surfaces::{
            status_bar_action_for_id, status_bar_context_items, statusbar_ids, StatusBarMenuAction,
            StatusBarSegmentState,
        };

        let label = state.segment_label(segment);
        let base_state = StatusBarSegmentState {
            segment_id: segment.segment_id().to_owned(),
            segment_label: label.clone(),
            visible: true,
            related_panel_name: None,
        };
        // Start from the shared WP-011 status-bar items (Copy / Hide / Open Panel / Refresh), then append
        // the segment-specific quick actions as additional stable-id items in the SAME menu model (no new
        // menu system — AC-008).
        let mut menu = ContextMenu::new("statusbar").items(status_bar_context_items(&base_state));
        match segment {
            EditorSegment::Eol => {
                menu = menu
                    .separator()
                    .item(ContextMenuItem::action(EOL_CONVERT_LF_ID, "Convert to LF"))
                    .item(ContextMenuItem::action(
                        EOL_CONVERT_CRLF_ID,
                        "Convert to CRLF",
                    ));
            }
            EditorSegment::Indent => {
                menu = menu
                    .separator()
                    .item(ContextMenuItem::action(
                        INDENT_TO_TABS_ID,
                        "Convert Indentation to Tabs",
                    ))
                    .item(ContextMenuItem::action(
                        INDENT_TO_SPACES_ID,
                        "Convert Indentation to Spaces",
                    ));
            }
            EditorSegment::Encoding => {
                menu = menu
                    .separator()
                    .item(ContextMenuItem::action(
                        ENCODING_REOPEN_UTF8_ID,
                        "Reopen with UTF-8",
                    ))
                    .item(ContextMenuItem::action(
                        ENCODING_REOPEN_UTF16LE_ID,
                        "Reopen with UTF-16 LE",
                    ));
            }
            EditorSegment::RenderWhitespace => {
                let toggle_label = if state.render_whitespace {
                    "Hide Whitespace"
                } else {
                    "Render Whitespace"
                };
                menu = menu
                    .separator()
                    .item(ContextMenuItem::action(WHITESPACE_TOGGLE_ID, toggle_label));
            }
            EditorSegment::LanguageMode => {} // language change is the left-click picker
        }

        let confirmed = menu.show_on(response)?;
        // Segment-specific quick actions first; fall back to the shared status-bar mapping.
        match confirmed {
            EOL_CONVERT_LF_ID => Some(EditorSegmentAction::ConvertEol(Eol::Lf)),
            EOL_CONVERT_CRLF_ID => Some(EditorSegmentAction::ConvertEol(Eol::Crlf)),
            INDENT_TO_TABS_ID => Some(EditorSegmentAction::SetIndent(IndentStyle {
                kind: IndentKind::Tabs,
                size: state.indent.size.max(1),
            })),
            INDENT_TO_SPACES_ID => Some(EditorSegmentAction::SetIndent(IndentStyle {
                kind: IndentKind::Spaces,
                size: if state.indent.size == 0 {
                    4
                } else {
                    state.indent.size
                },
            })),
            ENCODING_REOPEN_UTF8_ID => {
                Some(EditorSegmentAction::ReopenWithEncoding(Encoding::Utf8))
            }
            ENCODING_REOPEN_UTF16LE_ID => {
                Some(EditorSegmentAction::ReopenWithEncoding(Encoding::Utf16Le))
            }
            WHITESPACE_TOGGLE_ID => Some(EditorSegmentAction::SetRenderWhitespace(
                !state.render_whitespace,
            )),
            // The shared status-bar items (Copy / Hide / Refresh): only Copy maps to a doc action here
            // (Hide/Refresh are status-bar-chrome concerns the host can ignore for these editor segments).
            statusbar_ids::COPY_SEGMENT => match status_bar_action_for_id(confirmed, &base_state) {
                Some(StatusBarMenuAction::CopySegment) => {
                    Some(EditorSegmentAction::CopySegmentText(label))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Attach the stable author_id + `Role::Button` to a segment's live node (the same `name_node`
    /// pattern the menu leaves use — egui keys node builders by the response id, so this enriches the
    /// exact node egui emitted). Segments are a fixed-count, count-stable set addressed by their stable
    /// author_id STRING in egui's hashed id space (the MT-007 dynamic-author_id pattern), so they need no
    /// fixed-band DECLARED_IDENTITIES entry.
    fn name_segment_node(
        ui: &mut egui::Ui,
        widget_node_id: egui::Id,
        author_id: &str,
        label: &str,
    ) {
        let author_id = author_id.to_owned();
        let label = label.to_owned();
        ui.ctx()
            .accesskit_node_builder(widget_node_id, move |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(author_id);
                node.set_label(label);
            });
    }
}

/// The stable id (the `value` suffix) for an EOL picker row.
fn eol_value(eol: Eol) -> &'static str {
    match eol {
        Eol::Lf => "lf",
        Eol::Crlf => "crlf",
    }
}

// Stable ids for the MT-071 segment-specific context-menu quick actions (asserted by the segment test
// so a typo cannot drift between the builder and the dispatcher).
const EOL_CONVERT_LF_ID: &str = "statusbar.eol.convert_lf";
const EOL_CONVERT_CRLF_ID: &str = "statusbar.eol.convert_crlf";
const INDENT_TO_TABS_ID: &str = "statusbar.indent.to_tabs";
const INDENT_TO_SPACES_ID: &str = "statusbar.indent.to_spaces";
const ENCODING_REOPEN_UTF8_ID: &str = "statusbar.encoding.reopen_utf8";
const ENCODING_REOPEN_UTF16LE_ID: &str = "statusbar.encoding.reopen_utf16le";
const WHITESPACE_TOGGLE_ID: &str = "statusbar.whitespace.toggle";

#[cfg(test)]
mod tests {
    use super::*;

    fn full_state() -> MenuBarState {
        MenuBarState {
            theme_is_dark: true,
            view_mode_is_nsfw: true,
            project_drawer_open: true,
            bottom_drawer_open: false,
            has_active_tab: true,
            editor_available: true,
            editor_can_undo: true,
            editor_can_redo: true,
            editor_can_paste: true,
        }
    }

    /// Exhaustive-match proof (AC10): every `MenuBarAction` variant is handled by a match, so the
    /// compiler enforces that a new variant cannot be added without the shell handling it. If a variant
    /// is added, this match fails to compile until it is covered — the contract's compile-time guard.
    #[test]
    fn every_action_variant_is_covered_by_an_exhaustive_match() {
        fn dispatch(a: &MenuBarAction) -> &'static str {
            match a {
                MenuBarAction::NewDocument => "new-document",
                MenuBarAction::OpenWorkspacePicker => "open-workspace",
                MenuBarAction::SaveActiveDocument => "save",
                MenuBarAction::SaveAllDocuments => "save-all",
                MenuBarAction::CloseActiveTab => "close-tab",
                MenuBarAction::QuitApp => "quit",
                MenuBarAction::EditorUndo => "undo",
                MenuBarAction::EditorRedo => "redo",
                MenuBarAction::EditCut => "cut",
                MenuBarAction::EditCopy => "copy",
                MenuBarAction::EditPaste => "paste",
                MenuBarAction::OpenFindReplace => "find-replace",
                MenuBarAction::OpenWorkspaceSearch => "find-all",
                MenuBarAction::ToggleTheme => "toggle-theme",
                MenuBarAction::ToggleViewMode => "toggle-view-mode",
                MenuBarAction::ToggleProjectDrawer => "toggle-project-drawer",
                MenuBarAction::ToggleFileDrawer => "toggle-file-drawer",
                MenuBarAction::ToggleBottomPanel => "toggle-bottom-panel",
                MenuBarAction::ResetLayout => "reset-layout",
                MenuBarAction::OpenQuickSwitcher => "quick-switcher",
                MenuBarAction::OpenCommandPalette => "command-palette",
                MenuBarAction::FocusNextPane => "next-pane",
                MenuBarAction::FocusPrevPane => "prev-pane",
                MenuBarAction::OpenSwarmBoard => "swarm-board",
                MenuBarAction::NavigateToTab(_) => "navigate",
                MenuBarAction::OpenModelSessionLaunch => "model-session-launch",
                MenuBarAction::OpenTerminal => "terminal",
                MenuBarAction::OpenSettings => "settings",
                MenuBarAction::ShowAbout => "about",
                MenuBarAction::EditorCommand(_) => "editor-command",
            }
        }
        // Spot-check a representative sample so the match is also exercised at runtime.
        assert_eq!(dispatch(&MenuBarAction::ToggleTheme), "toggle-theme");
        assert_eq!(
            dispatch(&MenuBarAction::NavigateToTab("inference-lab".to_owned())),
            "navigate"
        );
        assert_eq!(
            dispatch(&MenuBarAction::OpenCommandPalette),
            "command-palette"
        );
    }

    /// The six fixed menu ids sit in the 92..=97 band, are sequential, and stay strictly below the pane
    /// id base — the disjoint-fresh-band invariant the registry collision test relies on.
    #[test]
    fn menu_ids_sit_in_a_disjoint_fresh_band() {
        assert_eq!(MENU_BAR_NODE_ID_BASE, 92);
        for (index, _menu) in MENU_DEFINITIONS.iter().enumerate() {
            let id = MENU_BAR_NODE_ID_BASE + index as u64;
            assert!((92..=97).contains(&id), "menu id {id} in band 92..=97");
            assert!(
                id < crate::accessibility::PANE_NODE_ID_BASE,
                "menu id {id} below pane base {}",
                crate::accessibility::PANE_NODE_ID_BASE
            );
        }
    }

    /// Author_ids are stable kebab-case keys an out-of-process model addresses the menus by.
    #[test]
    fn menu_author_ids_are_stable_kebab_case() {
        let ids: Vec<&str> = MENU_DEFINITIONS.iter().map(|m| m.author_id()).collect();
        assert_eq!(
            ids,
            vec![
                "menu-file",
                "menu-edit",
                "menu-view",
                "menu-go",
                "menu-run",
                "menu-help"
            ]
        );
        // No duplicates.
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "menu author_ids are unique");
    }

    /// Each menu's Alt+<letter> mnemonic is the underlined first letter of its title (the classic
    /// Windows menu-bar convention + the React mnemonics). The shell consumes exactly these chords.
    #[test]
    fn menu_mnemonics_are_the_title_initials() {
        let pairs = [
            (MenuId::File, egui::Key::F),
            (MenuId::Edit, egui::Key::E),
            (MenuId::View, egui::Key::V),
            (MenuId::Go, egui::Key::G),
            (MenuId::Run, egui::Key::R),
            (MenuId::Help, egui::Key::H),
        ];
        for (menu, key) in pairs {
            assert_eq!(menu.mnemonic_key(), key, "{:?} mnemonic", menu);
        }
        // The mnemonics are unique (no two menus share an Alt+letter chord).
        let keys: Vec<egui::Key> = MENU_DEFINITIONS.iter().map(|m| m.mnemonic_key()).collect();
        let mut sorted = keys.clone();
        sorted.sort_by_key(|k| format!("{k:?}"));
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "menu mnemonics are unique");
    }

    /// The popup id the Alt+letter opener writes is exactly `button_id.with("popup")` — what
    /// `egui::Popup::menu` reads for that button. If these two derivations ever diverged, the keyboard
    /// path would open a popup id no menu renders, so this pins the contract.
    #[test]
    fn mnemonic_popup_id_matches_menu_button_popup_id() {
        for index in 0..MENU_DEFINITIONS.len() {
            let button = menu_button_id(index);
            let popup = menu_popup_id(index);
            assert_eq!(
                popup,
                button.with("popup"),
                "popup id derives from the menu button id"
            );
        }
    }

    /// The swarm-accessible action list names only overlay/navigation keys and is non-empty (HBR-SWARM).
    #[test]
    fn swarm_accessible_actions_listed() {
        assert!(SWARM_ACCESSIBLE_ACTIONS.contains(&"menu.go.command-palette"));
        assert!(SWARM_ACCESSIBLE_ACTIONS.contains(&"menu.run.swarm-board"));
        assert!(SWARM_ACCESSIBLE_ACTIONS.contains(&MENU_RUN_MODEL_SESSION_LAUNCH_AUTHOR_ID));
        // 8 base overlay/navigation actions + the 4 MT-052 GO-menu editor-navigation leaves.
        assert_eq!(
            SWARM_ACCESSIBLE_ACTIONS.len(),
            12,
            "all overlay/navigation actions listed"
        );
        // MT-052 GO-menu editor navigation is swarm-discoverable.
        for id in [
            GO_NEXT_DIAGNOSTIC_AUTHOR_ID,
            GO_PREV_DIAGNOSTIC_AUTHOR_ID,
            GO_BACK_AUTHOR_ID,
            GO_FORWARD_AUTHOR_ID,
        ] {
            assert!(
                SWARM_ACCESSIBLE_ACTIONS.contains(&id),
                "{id} is swarm-accessible"
            );
        }
        // Destructive/document actions are NOT swarm-exposed.
        assert!(!SWARM_ACCESSIBLE_ACTIONS.contains(&"menu.file.quit"));
        assert!(!SWARM_ACCESSIBLE_ACTIONS.contains(&"menu.view.reset-layout"));
    }

    /// `MenuBar::show` paints the six top-level menu buttons as live `Role::MenuItem` nodes with stable
    /// `menu-*` author_ids on an idle (no-click) frame. (The click->action path is proven end-to-end in
    /// tests/test_top_menu_bar.rs against the real shell.)
    #[test]
    fn show_paints_six_menu_buttons() {
        use egui_kittest::kittest::{NodeT, Queryable};
        let state = full_state();
        let mut harness = egui_kittest::Harness::builder().build_ui(move |ui| {
            // The returned action is None on an idle frame; the widget still paints all six menus.
            let _ = MenuBar::new(state).show(ui);
        });
        harness.run();

        for label in ["FILE", "EDIT", "VIEW", "GO", "RUN", "HELP"] {
            let _ = harness.get_by_label(label);
        }
        let menu_nodes = harness
            .root()
            .children_recursive()
            .filter(|n| {
                n.accesskit_node()
                    .author_id()
                    .map(|a| a.starts_with("menu-"))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(menu_nodes, 6, "six top-level menu buttons in the live tree");
    }

    // ── MT-071 editor status-bar segment unit tests ──────────────────────────────────────────────────

    /// The five segment author_ids are exactly the ids the MT-071 contract names (role=Button), in the
    /// contract order, with no duplicates (MC-001 — the segment identity gate).
    #[test]
    fn segment_author_ids_match_the_contract() {
        let ids: Vec<&str> = EditorSegment::ALL.iter().map(|s| s.author_id()).collect();
        assert_eq!(
            ids,
            vec![
                "status-bar-language-mode",
                "status-bar-eol",
                "status-bar-indent",
                "status-bar-encoding",
                "status-bar-render-whitespace",
            ],
            "segment author_ids + order match the MT-071 contract",
        );
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "segment author_ids are unique");
    }

    /// Picker item author_ids follow the `status-bar-{segment}-item-{value}` scheme the MT names.
    #[test]
    fn picker_item_author_ids_follow_the_scheme() {
        assert_eq!(
            EditorSegment::LanguageMode.item_author_id("rust"),
            "status-bar-language-mode-item-rust",
        );
        assert_eq!(
            EditorSegment::Eol.item_author_id("lf"),
            "status-bar-eol-item-lf"
        );
        assert_eq!(
            EditorSegment::Encoding.item_author_id("utf8"),
            "status-bar-encoding-item-utf8",
        );
    }

    /// The segment label reflects the active document metadata, and the language segment shows the
    /// `(override)` hint only when the source is a user override (so the operator/agent sees provenance).
    #[test]
    fn segment_labels_reflect_metadata_and_override_hint() {
        let auto = EditorMetaSegmentState {
            language_label: "Rust".to_owned(),
            language_source: DetectionSource::Extension,
            available_languages: vec![],
            eol: Eol::Crlf,
            indent: IndentStyle {
                kind: IndentKind::Tabs,
                size: 4,
            },
            encoding: Encoding::Utf8Bom,
            render_whitespace: false,
        };
        assert_eq!(auto.segment_label(EditorSegment::LanguageMode), "Rust");
        assert_eq!(auto.segment_label(EditorSegment::Eol), "CRLF");
        assert_eq!(auto.segment_label(EditorSegment::Indent), "Tab Size: 4");
        assert_eq!(
            auto.segment_label(EditorSegment::Encoding),
            "UTF-8 with BOM"
        );
        assert_eq!(
            auto.segment_label(EditorSegment::RenderWhitespace),
            "Whitespace"
        );

        let overridden = EditorMetaSegmentState {
            language_source: DetectionSource::UserOverride,
            render_whitespace: true,
            ..auto
        };
        assert_eq!(
            overridden.segment_label(EditorSegment::LanguageMode),
            "Rust (override)",
            "an override shows the provenance hint",
        );
        assert_eq!(
            overridden.segment_label(EditorSegment::RenderWhitespace),
            "Whitespace ✓"
        );
    }

    /// The segment-specific context-menu quick-action ids are stable + distinct (a typo gate, MC-001).
    #[test]
    fn segment_context_menu_quick_action_ids_are_stable() {
        let ids = [
            EOL_CONVERT_LF_ID,
            EOL_CONVERT_CRLF_ID,
            INDENT_TO_TABS_ID,
            INDENT_TO_SPACES_ID,
            ENCODING_REOPEN_UTF8_ID,
            ENCODING_REOPEN_UTF16LE_ID,
            WHITESPACE_TOGGLE_ID,
        ];
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            assert!(
                id.starts_with("statusbar."),
                "quick-action id namespaced: {id}"
            );
            assert!(seen.insert(id), "quick-action id is unique: {id}");
        }
    }

    /// `EditorStatusSegments::show` paints the five segment buttons with their stable author_ids when a
    /// code document is active, and NOTHING when none is (AC-005).
    #[test]
    fn show_paints_five_segments_and_hides_when_none() {
        use egui_kittest::kittest::NodeT;
        let state = EditorMetaSegmentState {
            language_label: "Rust".to_owned(),
            language_source: DetectionSource::Extension,
            available_languages: crate::code_editor::language_mode::available_languages(),
            eol: Eol::Lf,
            indent: IndentStyle::DEFAULT,
            encoding: Encoding::Utf8,
            render_whitespace: false,
        };
        let mut shown = egui_kittest::Harness::builder().build_ui(move |ui| {
            let _ = EditorStatusSegments::new(Some(state.clone())).show(ui);
        });
        shown.run();
        let seg_nodes = shown
            .root()
            .children_recursive()
            .filter(|n| {
                n.accesskit_node()
                    .author_id()
                    .map(|a| a.starts_with("status-bar-"))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            seg_nodes, 5,
            "five editor segments painted when a code document is active"
        );

        let mut hidden = egui_kittest::Harness::builder().build_ui(move |ui| {
            let _ = EditorStatusSegments::new(None).show(ui);
        });
        hidden.run();
        let hidden_nodes = hidden
            .root()
            .children_recursive()
            .filter(|n| {
                n.accesskit_node()
                    .author_id()
                    .map(|a| a.starts_with("status-bar-"))
                    .unwrap_or(false)
            })
            .count();
        assert_eq!(
            hidden_nodes, 0,
            "no segments painted when no code document is active"
        );
    }
}
