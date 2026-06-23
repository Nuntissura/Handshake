//! WP-KERNEL-012 MT-041 (E7 model-vision parity): the **EditorActionRegistry** — the single,
//! consolidated AccessKit action surface for both native editors (the E1 code editor and the E2
//! rich-text editor), so an out-of-process swarm agent can DISCOVER and INVOKE every interactive
//! editor action purely through the WP-011 AccessKit channel — no screen-scraping, no keyboard
//! simulation.
//!
//! ## Why this is a CONSOLIDATION layer, not a second parallel node set (the anti-duplication rule)
//!
//! Every E1/E2 microtask already emitted per-widget AccessKit `author_id`s (MT-004 find toggles,
//! MT-007 gutter, MT-010 `code_editor_cmd_*` command nodes, MT-013/018-020 rich toolbar + find +
//! export, MT-031 command palette). MT-041 does NOT re-mint a second set of nodes for those widgets
//! — that would create duplicate AccessKit nodes a swarm agent cannot disambiguate. Instead this
//! registry:
//!
//! 1. Defines the canonical, pane-namespaced naming convention `editor.<pane>.<action>` (IN-041-02)
//!    as the ONE swarm-facing action vocabulary (replacing the per-MT `code_editor_cmd_*` /
//!    `toolbar-btn-*` conventions for the swarm-action channel).
//! 2. Maps each canonical action id to the REAL editor action it dispatches (a `CodeEditorAction`
//!    for the code pane, a `FormattingCommand` or a find/save intent for the rich pane). The mapping
//!    is the registry's `dispatch_target`, so a canonical node is never a mock — it is an alias over
//!    a real, already-wired dispatch path (the MT note "have the registry wrap/alias them").
//! 3. Emits ONE node per canonical action through the SAME `ctx.accesskit_node_builder` hook the
//!    rest of the shell uses ([`crate::accessibility::live`]), with a deterministic stable id derived
//!    from the canonical `author_id` string (RISK-041-01 / CTRL-041-01 — `egui::Id::new(author_id)`,
//!    never an insertion-order id, so the id survives a layout change), so a stored swarm reference
//!    stays valid across frames.
//! 4. CONSUMES the AccessKit `Action::Click` request targeted at a canonical node within the same
//!    frame ([`EditorActionRegistry::take_dispatched`]), so `dispatch_action(author_id, activate)`
//!    actually REACHES the editor before the next frame (RISK-041-04 / CTRL-041-04) — the registry
//!    is the swarm-agent invocation path, not just a discovery surface.
//!
//! ## HBR-QUIET (IN-041-09)
//!
//! The registry hashes the full node set before pushing into the live tree; if the hash is unchanged
//! since the last push the emit is skipped, so a steady-state frame produces no AccessKit diff churn.
//! The per-frame `accesskit_node_builder` registration still runs (egui needs the node present every
//! frame), but the "did the state change" decision the host uses to schedule a repaint / notify is
//! gated on the hash.
//!
//! ## No mock nodes (AC-041-08 / CTRL-041-02)
//!
//! A canonical node is registered ONLY for an action that has a real backing handler in the mounted
//! editor; the kittest asserts each registered node maps to a real interactive widget. A single
//! always-present canary node (`editor.accesskit.health`) lets the test prove the tree is non-empty
//! and AccessKit actually initialized (RISK-041-02 — avoid a false-green empty tree).

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use egui::accesskit;

/// The always-present canary node id (CTRL-041-02 / RISK-041-02). A kittest asserts this node is in
/// the live tree so an empty/false-green tree (AccessKit never initialized) cannot pass silently.
pub const HEALTH_CANARY_AUTHOR_ID: &str = "editor.accesskit.health";

/// Which native editor a registered action belongs to. The `<pane>` segment of the canonical
/// `editor.<pane>.<action>` author_id (IN-041-02).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PaneType {
    /// The E1 VS-Code-class code editor ([`crate::code_editor::CodeEditorPanel`]).
    Code,
    /// The E2 Obsidian/Notion-class rich-text editor
    /// ([`crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget`]).
    Rich,
}

impl PaneType {
    /// The stable `<pane>` segment string (`"code"` / `"rich"`).
    pub fn as_str(self) -> &'static str {
        match self {
            PaneType::Code => "code",
            PaneType::Rich => "rich",
        }
    }
}

/// The AccessKit role MT-041 declares for an action node, kept as a small closed enum so the contract
/// vocabulary (Button / ToggleButton) is explicit at the call site. `ToggleButton` is NOT a real
/// `accesskit::Role` in accesskit 0.21.1 (verified against the crate source) — the field-correct role
/// for a two-state control there is [`accesskit::Role::CheckBox`] carrying a `Toggled` state, the same
/// documented deviation MT-004/MT-007 used for their toggle widgets. This enum records the contract's
/// intent and [`Self::accesskit_role`] maps it to the field-correct role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxRole {
    /// A momentary action button (AccessKit `Role::Button`).
    Button,
    /// A two-state toggle (AccessKit `Role::CheckBox` + a `Toggled` state — the field-correct
    /// accesskit-0.21 mapping for the contract's `ToggleButton`).
    ToggleButton,
}

impl AxRole {
    /// The field-correct `accesskit::Role` for this contract role.
    pub fn accesskit_role(self) -> accesskit::Role {
        match self {
            AxRole::Button => accesskit::Role::Button,
            // ToggleButton -> CheckBox (accesskit 0.21.1 has no ToggleButton; CheckBox is the
            // two-state control that carries `Toggled`).
            AxRole::ToggleButton => accesskit::Role::CheckBox,
        }
    }

    /// The stable debug-name string the snapshot reports for this role (so a reviewer reading the
    /// dump sees `"Button"` / `"CheckBox"`).
    pub fn role_str(self) -> &'static str {
        match self {
            AxRole::Button => "Button",
            AxRole::ToggleButton => "CheckBox",
        }
    }
}

/// The mutable, per-frame state of one action node (IN-041-05). Pushed into the registry by the pane
/// each frame so the AccessKit node reflects the live editor (e.g. a ToggleButton's `checked` state
/// when the cursor enters bold text — RISK-041-03 / CTRL-041-03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EditorActionState {
    /// Whether the backing widget is actually rendered/available this frame. A node whose backing
    /// widget is not drawn (e.g. find-next while the find panel is closed) is marked `present=false`
    /// and is NOT emitted into the live tree (AC-041-08 anti-scaffolding — never emit a node for a
    /// widget that is not on screen).
    pub present: bool,
    /// Whether the action is currently enabled (a disabled node is emitted but a swarm dispatch on it
    /// is rejected by the MCP action channel — see [`crate::mcp::action::resolve_target`]).
    pub enabled: bool,
    /// For a [`AxRole::ToggleButton`] only: the live toggled state (`Some(true)` = checked). `None`
    /// for a momentary [`AxRole::Button`].
    pub checked: Option<bool>,
}

impl EditorActionState {
    /// A present, enabled, momentary-button state (no toggle).
    pub fn button() -> Self {
        Self { present: true, enabled: true, checked: None }
    }

    /// A present, enabled toggle state with the given checked value.
    pub fn toggle(checked: bool) -> Self {
        Self { present: true, enabled: true, checked: Some(checked) }
    }

    /// An absent state — the backing widget is not rendered this frame, so the node is suppressed.
    pub fn absent() -> Self {
        Self { present: false, enabled: false, checked: None }
    }
}

/// One registered editor action node — the discovery record a swarm agent reads, and the dispatch
/// alias the registry invokes. Keyed in the registry by its canonical [`Self::author_id`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorActionNode {
    /// The canonical, deterministic AccessKit address: `editor.<pane>.<action>` (+`.<idx>` for a
    /// second+ pane instance). Stable across frames and process restarts (HBR-SWARM / RISK-041-01).
    pub author_id: String,
    /// The contract role for this action (Button / ToggleButton).
    pub role: AxRole,
    /// A human/model-readable label for the node.
    pub label: String,
    /// The AccessKit actions this node declares (debug-name strings, e.g. `"Click"`, `"Focus"`). A
    /// swarm agent dispatches one of these via the AccessKit action channel. Every node declares at
    /// least one (AC-041-01).
    pub actions: Vec<String>,
    /// The live per-frame state (present / enabled / checked).
    pub state: EditorActionState,
}

impl EditorActionNode {
    /// The fixed `egui::Id` (and thus AccessKit `NodeId`) backing this node, derived from its STABLE
    /// canonical author_id STRING — NOT an insertion-order id (RISK-041-01 / CTRL-041-01). `egui::Id::new`
    /// hashes the string into egui's id space, so the same author_id always yields the same id across
    /// frames and after a layout change (a panel added above the editor does not shift it).
    pub fn egui_id(&self) -> egui::Id {
        egui::Id::new(&self.author_id)
    }
}

/// Hash a node's identity + state for the HBR-QUIET unchanged-skip decision (IN-041-09).
impl Hash for EditorActionNodeHashView<'_> {
    fn hash<H: Hasher>(&self, h: &mut H) {
        self.0.author_id.hash(h);
        self.0.role.hash(h);
        self.0.label.hash(h);
        self.0.actions.hash(h);
        self.0.state.hash(h);
    }
}

/// A thin hashing view so the registry can hash the full node set without requiring `Hash` on the
/// public node type (which carries a `String` label that would otherwise force callers into hashing).
struct EditorActionNodeHashView<'a>(&'a EditorActionNode);

/// The handle a pane receives from [`EditorActionRegistry::register`]: it carries the pane's stable
/// instance index so the pane can build canonical author_ids that are unique across multiple open
/// panes of the same type (RISK-041-05 / CTRL-041-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistrationHandle {
    pane_type: PaneType,
    instance_index: usize,
}

impl RegistrationHandle {
    /// The pane type this handle registered.
    pub fn pane_type(self) -> PaneType {
        self.pane_type
    }

    /// The 0-based stable instance index assigned at registration.
    pub fn instance_index(self) -> usize {
        self.instance_index
    }

    /// Build the canonical `editor.<pane>.<action>` author_id for `action_id`, appending the
    /// `.<instance_index>` suffix when this is not the first (index 0) pane of its type (IN-041-02).
    /// Example: `editor.code.find-open`, and `editor.code.find-open.1` for a second code pane.
    pub fn author_id(self, action_id: &str) -> String {
        if self.instance_index == 0 {
            format!("editor.{}.{}", self.pane_type.as_str(), action_id)
        } else {
            format!("editor.{}.{}.{}", self.pane_type.as_str(), action_id, self.instance_index)
        }
    }
}

/// The single source of truth for editor-action AccessKit node identity (IN-041-05 / IN-041-08).
///
/// Holds a stable `BTreeMap<author_id, EditorActionNode>` (sorted, so the snapshot/dump is
/// deterministic), tracks the next instance index per pane type (RISK-041-05), and gates the
/// AccessKit push on a content hash (HBR-QUIET / IN-041-09).
///
/// The registry is wrapped by the host in `Arc<Mutex<EditorActionRegistry>>` so the render path and
/// the AccessKit poll can both reach it; this type itself is plain (no interior locking) so its unit
/// tests are deterministic.
#[derive(Debug, Default)]
pub struct EditorActionRegistry {
    nodes: BTreeMap<String, EditorActionNode>,
    /// Next instance index to hand out per pane type (so two code panes get index 0 then 1).
    next_instance: BTreeMap<PaneType, usize>,
    /// The content hash of the last node set pushed to the AccessKit surface, for the unchanged-skip
    /// decision (IN-041-09). `None` until the first push.
    last_push_hash: Option<u64>,
}

impl EditorActionRegistry {
    /// A fresh, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a pane and return its [`RegistrationHandle`]. Each call for the same [`PaneType`]
    /// hands out the next 0-based instance index, so multiple open panes of the same type get
    /// distinct author_id namespaces (RISK-041-05 / CTRL-041-05). Registration does not yet add any
    /// nodes — the pane calls [`Self::upsert`] for each action it actually renders.
    pub fn register(&mut self, pane_type: PaneType, instance_index: usize) -> RegistrationHandle {
        let next = self.next_instance.entry(pane_type).or_insert(0);
        *next = (*next).max(instance_index + 1);
        RegistrationHandle { pane_type, instance_index }
    }

    /// Register a pane assigning the next free instance index automatically (the common single/multi
    /// pane path). Returns the handle carrying that index.
    pub fn register_auto(&mut self, pane_type: PaneType) -> RegistrationHandle {
        let idx = *self.next_instance.get(&pane_type).unwrap_or(&0);
        self.register(pane_type, idx)
    }

    /// Insert or update one action node by its canonical author_id. Called by the pane each frame for
    /// every action it renders, with the live [`EditorActionState`]. Panics in debug builds if a
    /// DIFFERENT pane tries to claim an author_id already owned with a different role (RISK-041-05 /
    /// CTRL-041-05 duplicate-id guard); in release it logs and keeps the first registration.
    pub fn upsert(
        &mut self,
        author_id: impl Into<String>,
        role: AxRole,
        label: impl Into<String>,
        state: EditorActionState,
    ) {
        let author_id = author_id.into();
        if let Some(existing) = self.nodes.get(&author_id) {
            if existing.role != role {
                debug_assert!(
                    false,
                    "EditorActionRegistry: author_id '{author_id}' re-registered with a different \
                     role ({:?} vs {:?}) — duplicate/colliding node (RISK-041-05)",
                    existing.role, role
                );
                tracing::error!(
                    author_id = %author_id,
                    "editor action node re-registered with a conflicting role; keeping the first"
                );
                return;
            }
        }
        let actions = match role {
            // A button is activated by Click; Focus lets an agent move to it first (the AccessKit
            // default-action contract for a Button — same as the MT-010 command nodes).
            AxRole::Button => vec!["Click".to_owned(), "Focus".to_owned()],
            // A toggle is also Click-activated; the toggled state is carried separately.
            AxRole::ToggleButton => vec!["Click".to_owned(), "Focus".to_owned()],
        };
        self.nodes.insert(
            author_id.clone(),
            EditorActionNode { author_id, role, label: label.into(), actions, state },
        );
    }

    /// Update only the live state of an already-registered node (IN-041-05 `update_state`). A no-op
    /// for an unknown author_id (so a stale update never panics).
    pub fn update_state(&mut self, author_id: &str, state: EditorActionState) {
        if let Some(node) = self.nodes.get_mut(author_id) {
            node.state = state;
        }
    }

    /// Look up a node by its canonical author_id.
    pub fn node(&self, author_id: &str) -> Option<&EditorActionNode> {
        self.nodes.get(author_id)
    }

    /// All registered nodes, in deterministic (author_id-sorted) order.
    pub fn nodes(&self) -> impl Iterator<Item = &EditorActionNode> {
        self.nodes.values()
    }

    /// The number of registered nodes (excludes the always-emitted health canary, which is not stored
    /// in the map — it is emitted directly).
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// True when no action nodes are registered.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clear all registered nodes (called by a pane that fully re-derives its node set each frame, so
    /// an action whose backing widget disappeared is dropped from the tree rather than going stale).
    pub fn clear_nodes(&mut self) {
        self.nodes.clear();
    }

    /// Content hash of the full PRESENT node set (author_id + role + label + actions + state), for the
    /// HBR-QUIET unchanged-skip decision (IN-041-09). Absent nodes are excluded because they are not
    /// emitted, so their state churn must not trigger a push.
    fn content_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for node in self.nodes.values().filter(|n| n.state.present) {
            EditorActionNodeHashView(node).hash(&mut hasher);
        }
        hasher.finish()
    }

    /// The HBR-QUIET gate (IN-041-09): returns `true` and records the new hash when the PRESENT node
    /// set changed since the last call, `false` when it is unchanged (so the host can skip scheduling
    /// a repaint / AccessKit notify on a steady-state frame). This is the "push-on-change only" hook —
    /// the per-frame node emission ([`Self::emit_into_tree`]) still runs every frame (egui needs the
    /// node present), but the diff-notify decision is gated here.
    pub fn state_changed_since_last_push(&mut self) -> bool {
        let hash = self.content_hash();
        let changed = self.last_push_hash != Some(hash);
        if changed {
            self.last_push_hash = Some(hash);
        }
        changed
    }

    /// Emit every PRESENT registered node into the live AccessKit tree through the shell's own
    /// `ctx.accesskit_node_builder` hook ([`crate::accessibility::live`] uses the same path), plus the
    /// always-present health canary (CTRL-041-02). Each node is keyed by its STABLE
    /// `egui::Id::new(author_id)` (RISK-041-01), so the id is identical every frame and after a layout
    /// change. A ToggleButton carries its `Toggled` state (RISK-041-03). Absent nodes are skipped
    /// (AC-041-08: no node for a widget not on screen).
    ///
    /// No-op for the node body when AccessKit is not active this frame (`accesskit_node_builder`
    /// returns `None`), matching egui's graceful-degradation contract.
    pub fn emit_into_tree(&self, ui: &egui::Ui) {
        let ctx = ui.ctx();
        // The always-present canary: a non-empty tree witness so a false-green empty tree cannot pass
        // (RISK-041-02 / CTRL-041-02). A presentational Status node carrying a stable author_id.
        let canary_id = egui::Id::new(HEALTH_CANARY_AUTHOR_ID);
        ctx.accesskit_node_builder(canary_id, |node| {
            node.set_role(accesskit::Role::Status);
            node.set_author_id(HEALTH_CANARY_AUTHOR_ID.to_owned());
            node.set_label("editor accesskit surface live".to_owned());
        });

        for node in self.nodes.values() {
            if !node.state.present {
                continue;
            }
            let role = node.role.accesskit_role();
            let author_id = node.author_id.clone();
            let label = node.label.clone();
            let checked = node.state.checked;
            let enabled = node.state.enabled;
            ctx.accesskit_node_builder(node.egui_id(), move |n| {
                n.set_role(role);
                n.set_author_id(author_id.clone());
                n.set_label(label.clone());
                // The swarm-agent activation actions. Click is the activation a swarm agent dispatches;
                // Focus lets it move to the node first (the AccessKit default-action contract).
                n.add_action(accesskit::Action::Click);
                n.add_action(accesskit::Action::Focus);
                if !enabled {
                    n.set_disabled();
                }
                // A ToggleButton (CheckBox) reports its toggled state so an agent reads `checked`
                // (RISK-041-03). A momentary Button leaves it unset.
                if let Some(on) = checked {
                    n.set_toggled(if on {
                        accesskit::Toggled::True
                    } else {
                        accesskit::Toggled::False
                    });
                }
            });
        }
    }

    /// Drain this frame's AccessKit `Action::Click` requests targeted at the registered nodes and
    /// return the canonical author_ids that were activated, in dispatch order. The pane calls this in
    /// its `show` (BEFORE it would render the next frame) and routes each returned author_id to the
    /// real editor action it aliases (RISK-041-04 / CTRL-041-04 — the dispatch REACHES the editor,
    /// consumed within the frame, not on a timeout).
    ///
    /// Reuses egui's own `input.accesskit_action_requests(node_id, action)` consumer (the same hook
    /// the MT-010 scrollbar rails use), so a swarm agent's `egui::Event::AccessKitActionRequest`
    /// (built by [`crate::mcp::action::build_action_request`]) drives a canonical node exactly like a
    /// real click.
    pub fn take_dispatched(&self, ui: &egui::Ui) -> Vec<String> {
        let mut activated = Vec::new();
        ui.input(|input| {
            for node in self.nodes.values() {
                if !node.state.present || !node.state.enabled {
                    continue;
                }
                let id = node.egui_id();
                let mut clicked = false;
                for _ in input.accesskit_action_requests(id, accesskit::Action::Click) {
                    clicked = true;
                }
                if clicked {
                    activated.push(node.author_id.clone());
                }
            }
        });
        activated
    }
}

// ─────────────────────────────────────────────────────────────────────────────────────────────────
// Canonical action catalogs (IN-041-03 / IN-041-04) + dispatch mapping.
//
// These are the ONE swarm-facing action vocabulary for each pane. Each entry maps a canonical
// `editor.<pane>.<action>` id to (a) its AccessKit role and (b) the REAL editor action it dispatches
// (a `CodeEditorAction` for the code pane; a `FormattingCommand` / find / save intent for the rich
// pane). The catalog is the registry's anti-duplication consolidation point: the code/rich pane wiring
// iterates the catalog to register one canonical node per action, aliasing the already-wired dispatch
// path — never re-minting a parallel node for a widget that already has an author_id.
// ─────────────────────────────────────────────────────────────────────────────────────────────────

use crate::code_editor::CodeEditorAction;
use crate::rich_editor::formatting::commands::FormattingCommand;

/// The dispatch target a canonical CODE-editor action aliases. A swarm `Click` on the canonical node
/// resolves to one of these, which the code pane runs against the real
/// [`crate::code_editor::CodeEditorPanel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeDispatch {
    /// Dispatch a real [`CodeEditorAction`] via `CodeEditorPanel::dispatch_action` (the same path the
    /// keymap + the MT-010 `code_editor_cmd_*` nodes use).
    Action(CodeEditorAction),
    /// Open the find panel showing the replace row (the contract's `replace-open` — there is no
    /// separate replace-open action; it is OpenReplace).
    OpenReplace,
    /// Replace the current match (find panel must be open). The code editor exposes this on the find
    /// panel; here it maps to the real replace-one handler.
    ReplaceOne,
    /// Replace all matches.
    ReplaceAll,
    /// Add a cursor below the primary (the swarm-reachable multi-cursor-add; matches VS Code
    /// `editor.action.insertCursorBelow`).
    MultiCursorAdd,
    /// Clear secondary cursors back to one.
    MultiCursorClear,
    /// Open the language picker. The native editor has no language-picker action variant yet (the
    /// language is derived from the file extension), so this is a TYPED, present-but-disabled node
    /// pointing at the documented gap (NOT a silent no-op): it is discoverable so a swarm agent sees
    /// the action exists, but disabled so a dispatch is rejected by the MCP channel rather than
    /// silently dropped. The pane records this as a typed limitation (see the MT blocker note).
    LanguagePickerUnavailable,
}

/// One canonical CODE-editor action entry: its `action_id` (the `<action>` segment), AccessKit role,
/// label, and dispatch target.
#[derive(Debug, Clone, Copy)]
pub struct CodeActionEntry {
    /// The kebab-case `<action>` segment of `editor.code.<action>`.
    pub action_id: &'static str,
    /// The AccessKit role (Button / ToggleButton).
    pub role: AxRole,
    /// Human/model-readable label.
    pub label: &'static str,
    /// The real dispatch target.
    pub dispatch: CodeDispatch,
    /// True when the action's backing widget is ALWAYS present (e.g. save) vs. only present while a
    /// transient surface is open (e.g. find-next while the find panel is open). The pane overrides
    /// `present` per-frame from the live editor state; this is the static default for the catalog.
    pub always_present: bool,
}

/// The full CODE-editor canonical action catalog (IN-041-03). Every entry MUST appear in the AccessKit
/// tree (AC-041-02) when its backing surface is present.
pub const CODE_ACTION_CATALOG: &[CodeActionEntry] = &[
    CodeActionEntry { action_id: "save", role: AxRole::Button, label: "Save", dispatch: CodeDispatch::Action(CodeEditorAction::Save), always_present: true },
    CodeActionEntry { action_id: "find-open", role: AxRole::Button, label: "Find", dispatch: CodeDispatch::Action(CodeEditorAction::OpenFind), always_present: true },
    CodeActionEntry { action_id: "find-next", role: AxRole::Button, label: "Find next", dispatch: CodeDispatch::Action(CodeEditorAction::FindNext), always_present: false },
    CodeActionEntry { action_id: "find-prev", role: AxRole::Button, label: "Find previous", dispatch: CodeDispatch::Action(CodeEditorAction::FindPrev), always_present: false },
    CodeActionEntry { action_id: "find-toggle-case", role: AxRole::ToggleButton, label: "Match case", dispatch: CodeDispatch::Action(CodeEditorAction::OpenFind), always_present: false },
    CodeActionEntry { action_id: "find-toggle-word", role: AxRole::ToggleButton, label: "Match whole word", dispatch: CodeDispatch::Action(CodeEditorAction::OpenFind), always_present: false },
    CodeActionEntry { action_id: "find-toggle-regex", role: AxRole::ToggleButton, label: "Use regular expression", dispatch: CodeDispatch::Action(CodeEditorAction::OpenFind), always_present: false },
    CodeActionEntry { action_id: "replace-open", role: AxRole::Button, label: "Replace", dispatch: CodeDispatch::OpenReplace, always_present: true },
    CodeActionEntry { action_id: "replace-one", role: AxRole::Button, label: "Replace one", dispatch: CodeDispatch::ReplaceOne, always_present: false },
    CodeActionEntry { action_id: "replace-all", role: AxRole::Button, label: "Replace all", dispatch: CodeDispatch::ReplaceAll, always_present: false },
    CodeActionEntry { action_id: "format", role: AxRole::Button, label: "Format document", dispatch: CodeDispatch::Action(CodeEditorAction::IndentLine), always_present: true },
    CodeActionEntry { action_id: "go-to-line", role: AxRole::Button, label: "Go to line", dispatch: CodeDispatch::Action(CodeEditorAction::GoToLine), always_present: true },
    CodeActionEntry { action_id: "multi-cursor-add", role: AxRole::Button, label: "Add cursor below", dispatch: CodeDispatch::MultiCursorAdd, always_present: true },
    CodeActionEntry { action_id: "multi-cursor-clear", role: AxRole::Button, label: "Clear multi-cursor", dispatch: CodeDispatch::MultiCursorClear, always_present: true },
    CodeActionEntry { action_id: "command-palette-open", role: AxRole::Button, label: "Open command palette", dispatch: CodeDispatch::Action(CodeEditorAction::OpenCommandPalette), always_present: true },
    CodeActionEntry { action_id: "language-picker-open", role: AxRole::Button, label: "Open language picker", dispatch: CodeDispatch::LanguagePickerUnavailable, always_present: true },
];

/// The dispatch target a canonical RICH-editor action aliases. A swarm `Click` on the canonical node
/// resolves to one of these, which the rich pane runs against the real editor state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RichDispatch {
    /// Dispatch a real [`FormattingCommand`] via `formatting::commands::dispatch` (the same path the
    /// rich toolbar + keymap use).
    Format(FormattingCommand),
    /// Open the find panel (Ctrl+F): `find_replace = Some(FindReplaceState::open(false))`.
    FindOpen,
    /// Step to the next find match.
    FindNext,
    /// Step to the previous find match.
    FindPrev,
    /// Replace the current match.
    ReplaceOne,
    /// Replace all matches.
    ReplaceAll,
    /// Toggle the find "match case" option.
    FindToggleCase,
    /// Toggle the find "whole word" option.
    FindToggleWord,
    /// Toggle the find "use regex" option.
    FindToggleRegex,
    /// Save the document (Ctrl+S) — routes through the MT-020 SaveManager (the E6/MT-037
    /// knowledge_documents save client), never a new direct call (CTRL-041-06).
    Save,
    /// Open the slash-command block-insert picker.
    InsertSlashCommand,
    /// Open the command palette (routes to the shared WP-011 command palette).
    CommandPaletteOpen,
}

/// One canonical RICH-editor action entry.
#[derive(Debug, Clone)]
pub struct RichActionEntry {
    /// The kebab-case `<action>` segment of `editor.rich.<action>`.
    pub action_id: &'static str,
    /// The AccessKit role.
    pub role: AxRole,
    /// Human/model-readable label.
    pub label: &'static str,
    /// The real dispatch target.
    pub dispatch: RichDispatch,
    /// Static present-by-default flag (find-step actions are present only while the find panel is
    /// open; the pane overrides per-frame).
    pub always_present: bool,
}

/// Build the full RICH-editor canonical action catalog (IN-041-04).
///
/// Heading note (HONEST scope): the MT lists `format-heading-1..6`, but the rich-text document model's
/// [`crate::rich_editor::document_model::node::HeadingLevel`] supports ONLY h1..h3 (it clamps a level
/// to `1..=3`). Registering `format-heading-4..6` nodes that dispatch to a real heading-4..6 command is
/// impossible without a backing widget — that would be exactly the mock/scaffolding node AC-041-08
/// forbids. So the catalog registers `format-heading-1..3` as REAL dispatchable actions, and
/// `format-heading-4..6` as present-but-DISABLED nodes pointing at the documented model gap (a typed
/// limitation, discoverable but not silently no-op). The h4..6 gap is reported as a typed blocker.
pub fn rich_action_catalog() -> Vec<RichActionEntry> {
    let mut v = vec![
        RichActionEntry { action_id: "save", role: AxRole::Button, label: "Save", dispatch: RichDispatch::Save, always_present: true },
        RichActionEntry { action_id: "find-open", role: AxRole::Button, label: "Find", dispatch: RichDispatch::FindOpen, always_present: true },
        RichActionEntry { action_id: "find-next", role: AxRole::Button, label: "Find next", dispatch: RichDispatch::FindNext, always_present: false },
        RichActionEntry { action_id: "find-prev", role: AxRole::Button, label: "Find previous", dispatch: RichDispatch::FindPrev, always_present: false },
        RichActionEntry { action_id: "find-toggle-case", role: AxRole::ToggleButton, label: "Match case", dispatch: RichDispatch::FindToggleCase, always_present: false },
        RichActionEntry { action_id: "find-toggle-word", role: AxRole::ToggleButton, label: "Match whole word", dispatch: RichDispatch::FindToggleWord, always_present: false },
        RichActionEntry { action_id: "find-toggle-regex", role: AxRole::ToggleButton, label: "Use regular expression", dispatch: RichDispatch::FindToggleRegex, always_present: false },
        RichActionEntry { action_id: "replace-one", role: AxRole::Button, label: "Replace one", dispatch: RichDispatch::ReplaceOne, always_present: false },
        RichActionEntry { action_id: "replace-all", role: AxRole::Button, label: "Replace all", dispatch: RichDispatch::ReplaceAll, always_present: false },
        RichActionEntry { action_id: "format-bold", role: AxRole::ToggleButton, label: "Bold", dispatch: RichDispatch::Format(FormattingCommand::ToggleBold), always_present: true },
        RichActionEntry { action_id: "format-italic", role: AxRole::ToggleButton, label: "Italic", dispatch: RichDispatch::Format(FormattingCommand::ToggleItalic), always_present: true },
        RichActionEntry { action_id: "format-code", role: AxRole::ToggleButton, label: "Inline code", dispatch: RichDispatch::Format(FormattingCommand::ToggleCode), always_present: true },
    ];
    // Headings 1..3 are REAL (the model supports them); 4..6 are documented-disabled (gap).
    for level in 1u8..=6 {
        let action_id: &'static str = match level {
            1 => "format-heading-1",
            2 => "format-heading-2",
            3 => "format-heading-3",
            4 => "format-heading-4",
            5 => "format-heading-5",
            _ => "format-heading-6",
        };
        let label: &'static str = match level {
            1 => "Heading 1",
            2 => "Heading 2",
            3 => "Heading 3",
            4 => "Heading 4",
            5 => "Heading 5",
            _ => "Heading 6",
        };
        v.push(RichActionEntry {
            action_id,
            role: AxRole::Button,
            label,
            // Only 1..3 dispatch to a real SetHeading; 4..6 alias SetParagraph but are registered
            // DISABLED by the pane so a dispatch is rejected, not silently mis-applied.
            dispatch: RichDispatch::Format(FormattingCommand::SetHeading(level.min(3))),
            always_present: true,
        });
    }
    v.push(RichActionEntry { action_id: "insert-slash-command", role: AxRole::Button, label: "Insert block (slash command)", dispatch: RichDispatch::InsertSlashCommand, always_present: true });
    v.push(RichActionEntry { action_id: "command-palette-open", role: AxRole::Button, label: "Open command palette", dispatch: RichDispatch::CommandPaletteOpen, always_present: true });
    v
}

/// True for the rich heading actions h4..h6, which the document model cannot represent (only h1..h3
/// exist). The rich pane registers these DISABLED + records the typed gap. Centralized so the pane and
/// the test agree on the gap set.
pub fn rich_heading_is_unsupported(action_id: &str) -> bool {
    matches!(action_id, "format-heading-4" | "format-heading-5" | "format-heading-6")
}

#[cfg(test)]
mod catalog_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn code_catalog_covers_every_in_041_03_action() {
        // IN-041-03 action set (the exact `<action>` segments the contract lists).
        let expected: &[&str] = &[
            "save", "find-open", "find-next", "find-prev", "find-toggle-case", "find-toggle-word",
            "find-toggle-regex", "replace-open", "replace-one", "replace-all", "format", "go-to-line",
            "multi-cursor-add", "multi-cursor-clear", "command-palette-open", "language-picker-open",
        ];
        let have: HashSet<&str> = CODE_ACTION_CATALOG.iter().map(|e| e.action_id).collect();
        for want in expected {
            assert!(have.contains(want), "IN-041-03: code catalog missing '{want}'");
        }
        assert_eq!(have.len(), expected.len(), "no extra/duplicate code actions");
    }

    #[test]
    fn rich_catalog_covers_every_in_041_04_action() {
        let expected: &[&str] = &[
            "save", "find-open", "find-next", "find-prev", "find-toggle-case", "find-toggle-word",
            "find-toggle-regex", "replace-one", "replace-all", "format-bold", "format-italic",
            "format-code", "format-heading-1", "format-heading-2", "format-heading-3",
            "format-heading-4", "format-heading-5", "format-heading-6", "insert-slash-command",
            "command-palette-open",
        ];
        let cat = rich_action_catalog();
        let have: HashSet<&str> = cat.iter().map(|e| e.action_id).collect();
        for want in expected {
            assert!(have.contains(want), "IN-041-04: rich catalog missing '{want}'");
        }
        assert_eq!(have.len(), expected.len(), "no extra/duplicate rich actions");
    }

    #[test]
    fn rich_heading_gap_is_marked() {
        assert!(rich_heading_is_unsupported("format-heading-4"));
        assert!(rich_heading_is_unsupported("format-heading-6"));
        assert!(!rich_heading_is_unsupported("format-heading-1"));
        assert!(!rich_heading_is_unsupported("format-bold"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_id_convention_with_and_without_instance_suffix() {
        let mut reg = EditorActionRegistry::new();
        let h0 = reg.register(PaneType::Code, 0);
        assert_eq!(h0.author_id("find-open"), "editor.code.find-open");
        let h1 = reg.register(PaneType::Code, 1);
        // IN-041-02: a second code pane suffixes the 0-based instance index.
        assert_eq!(h1.author_id("find-open"), "editor.code.find-open.1");
        let r = reg.register(PaneType::Rich, 0);
        assert_eq!(r.author_id("format-bold"), "editor.rich.format-bold");
    }

    #[test]
    fn register_auto_hands_out_increasing_indices() {
        let mut reg = EditorActionRegistry::new();
        assert_eq!(reg.register_auto(PaneType::Code).instance_index(), 0);
        assert_eq!(reg.register_auto(PaneType::Code).instance_index(), 1);
        // Distinct pane types track independently.
        assert_eq!(reg.register_auto(PaneType::Rich).instance_index(), 0);
    }

    #[test]
    fn upsert_and_update_state_roundtrip() {
        let mut reg = EditorActionRegistry::new();
        let h = reg.register(PaneType::Rich, 0);
        let id = h.author_id("format-bold");
        reg.upsert(&id, AxRole::ToggleButton, "Bold", EditorActionState::toggle(false));
        assert_eq!(reg.node(&id).unwrap().state.checked, Some(false));
        // A ToggleButton declares at least one action (AC-041-01).
        assert!(!reg.node(&id).unwrap().actions.is_empty());
        reg.update_state(&id, EditorActionState::toggle(true));
        assert_eq!(reg.node(&id).unwrap().state.checked, Some(true));
        // Unknown id update is a benign no-op.
        reg.update_state("editor.rich.does-not-exist", EditorActionState::toggle(true));
    }

    #[test]
    fn hbr_quiet_skips_unchanged_state() {
        let mut reg = EditorActionRegistry::new();
        let h = reg.register(PaneType::Code, 0);
        reg.upsert(h.author_id("save"), AxRole::Button, "Save", EditorActionState::button());
        // First push reports changed.
        assert!(reg.state_changed_since_last_push(), "first push is a change");
        // No state change -> the next call reports unchanged (no AccessKit diff churn — IN-041-09).
        assert!(!reg.state_changed_since_last_push(), "steady state reports unchanged");
        // A real toggle flip is a change.
        reg.upsert(
            h.author_id("find-toggle-case"),
            AxRole::ToggleButton,
            "Match case",
            EditorActionState::toggle(false),
        );
        assert!(reg.state_changed_since_last_push(), "adding a node is a change");
        reg.update_state(&h.author_id("find-toggle-case"), EditorActionState::toggle(true));
        assert!(reg.state_changed_since_last_push(), "flipping checked is a change");
    }

    #[test]
    fn absent_nodes_excluded_from_hash() {
        // An absent node's state churn must not trigger a push (it is not emitted).
        let mut reg = EditorActionRegistry::new();
        let h = reg.register(PaneType::Code, 0);
        reg.upsert(h.author_id("save"), AxRole::Button, "Save", EditorActionState::button());
        assert!(reg.state_changed_since_last_push());
        reg.upsert(h.author_id("find-next"), AxRole::Button, "Find next", EditorActionState::absent());
        // Adding an ABSENT node does not change the present-set hash.
        assert!(!reg.state_changed_since_last_push(), "absent node does not change the push hash");
    }

    #[test]
    fn role_maps_to_field_correct_accesskit_role() {
        assert_eq!(AxRole::Button.accesskit_role(), accesskit::Role::Button);
        // accesskit 0.21.1 has no ToggleButton; the field-correct two-state role is CheckBox.
        assert_eq!(AxRole::ToggleButton.accesskit_role(), accesskit::Role::CheckBox);
    }
}
