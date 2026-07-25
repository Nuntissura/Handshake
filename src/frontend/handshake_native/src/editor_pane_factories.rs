//! WP-KERNEL-012 MT-079 (E11 host-mount): the session-threaded editor pane factories that mount the
//! REAL native editors into the running `HandshakeApp` shell.
//!
//! ## Why this module exists
//!
//! Through MT-001..MT-068 the code editor (`code_editor::panel::CodeEditorPanel`) and the rich-text
//! editor (`rich_editor::renderer::rich_editor_widget::RichEditorState`) were each built + proven at
//! the egui_kittest WIDGET level, and each ships a thin `PaneFactory` wrapper
//! ([`crate::code_editor::panel::CodeEditorPaneFactory`] /
//! [`crate::rich_editor::renderer::rich_editor_widget::RichEditorPaneFactory`]). But `app.rs` never
//! REGISTERED those factories: `build_default_factories` / `build_factories_with_loom_search_v2`
//! installed a `PlaceholderPaneFactory` for `PaneType::CodeSymbol` (the code surface) and
//! `PaneType::LoomWikiPage` (the Notes surface), so a mounted editor pane rendered a centered
//! placeholder label, never the real editor. This module closes that structural gap.
//!
//! ## What it does (the CORE mount, AC-079-1..AC-079-5)
//!
//! It builds two SESSION-THREADED wrapper factories that:
//!
//! 1. wrap the EXISTING `CodeEditorPaneFactory` / `RichEditorPaneFactory` (no editor logic is
//!    re-implemented — REUSE, not fork);
//! 2. hold a shared [`EditorSessionContext`] cell (active `workspace_id` + tokio `runtime` handle),
//!    threaded in on mount through the SAME `Arc<Mutex<_>>` shared-cell pattern `app.rs` already uses
//!    for `LoomSearchV2PaneFactory` / `FindInFilesPaneFactory` — the `PaneFactory::render` signature
//!    is UNCHANGED (RISK-079-5 / MC-079-3);
//! 3. on the FIRST render with a live session context, call the prior-MT hooks with real session
//!    context: code pane `set_runtime` + `set_workspace_id` (MT-008/010); rich pane
//!    `set_embed_context` (MT-014) + `set_wikilink_context` (MT-057) (AC-079-2 / PT-079-B);
//! 4. wire the shell command `Sender<CodeEditorAction>` into the code pane so Save / Undo / Redo /
//!    OpenCommandPalette reach the WP-011 command bus + MT-035 unified undo (AC-079-3 / PT-079-C);
//! 5. DRAIN `RichEditorState.pending_events` each frame AFTER the editor renders and push the drained
//!    [`EditorEvent`]s into a shared outbound queue ([`RichPaneEvents`]) the shell routes to the nav
//!    bus (WikilinkActivated / BacklinkActivated / TagActivated) (AC-079-5 / PT-079-E).
//!
//! Both editors use interior mutability (`&self` `set_*` methods / `Arc<Mutex<RichEditorState>>`), so
//! threading session context through the established shared-cell pattern needs no trait change.
//!
//! HONESTY (MC-079-5): this module mounts the CORE code + rich panes LIVE, including MT-043's exact
//! rich-code-block -> native-code-panel save bridge. The FULLER mounts
//! (canvas/graph/side panes, MT-060/061/062/063/064/066/067) keep their existing concrete factories
//! or honest empty states. No `todo!()`/`unimplemented!()` is added on any live dispatch path.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::code_editor::panel::{CodeEditorHostCommand, CodeEditorPaneFactory, CodeEditorPanel};
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::rich_editor::renderer::rich_editor_widget::{RichEditorPaneFactory, RichEditorState};
use crate::rich_editor::wikilinks::inline_view::EditorEvent;

/// The live session context the shell pushes into the editor factories each time the active workspace
/// changes (the SAME shared-cell idea `LoomSearchV2PaneShared` / `FindInFilesPaneShared` use). A factory
/// reads it on render and threads it into its editor's prior-MT `set_*` hooks on mount.
///
/// `None` runtime / empty `workspace_id` is the honest unbound state: a headless/test shell that never
/// installs a context leaves the editor in its existing runtime-less graceful-degradation mode (no
/// perpetual spinner, no panic) exactly as the widget-level tests already prove.
#[derive(Clone, Default)]
pub struct EditorSessionContext {
    /// The active workspace id the editors scope backend lookups to (code-nav, embeds, wikilink
    /// resolution). Empty until the shell installs the active project.
    pub workspace_id: String,
    /// The tokio runtime handle the editors spawn their off-thread backend work onto. `None` until the
    /// shell installs it (the production shell always does; a current-thread test harness may not).
    pub runtime: Option<tokio::runtime::Handle>,
}

impl EditorSessionContext {
    /// A bound context (the production wiring point: `workspace_id` + the app runtime handle).
    pub fn new(workspace_id: impl Into<String>, runtime: tokio::runtime::Handle) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            runtime: Some(runtime),
        }
    }

    /// Whether this context carries enough to thread real session state into an editor (a non-empty
    /// workspace AND a runtime handle). The factory only calls the `set_*` hooks once this is true, so a
    /// half-built context never installs a partial (and misleading) wired state.
    pub fn is_bound(&self) -> bool {
        self.runtime.is_some() && !self.workspace_id.is_empty()
    }
}

/// The shared cell holding the live [`EditorSessionContext`]. The shell owns an `Arc<Mutex<_>>` clone
/// and overwrites it whenever the active workspace changes; each factory holds the SAME `Arc` and reads
/// it on render. This is the established `&self`-render shared-cell threading pattern (the factory map
/// stores `Box<dyn PaneFactory>` and `render` takes `&self`, so per-frame state arrives via this cell,
/// not a `&mut self`).
pub type SharedSessionContext = Arc<Mutex<EditorSessionContext>>;

/// A FNV-1a / lock-free outbound queue of the rich editor's drained [`EditorEvent`]s. The rich pane
/// factory drains `RichEditorState.pending_events` after the editor renders and pushes them here; the
/// shell drains THIS queue once per frame (after the pane host) and routes each event to the MT-030
/// navigation bus (AC-079-5). Keeping the queue here (not inside the editor state) means the editor
/// stays a pure widget and the routing stays the shell's responsibility — the exact ownership split the
/// MT-015 `pending_events` doc comment already names ("routing is owned by the shell").
#[derive(Clone, Default)]
pub struct RichPaneEvents {
    inner: Arc<Mutex<Vec<EditorEvent>>>,
}

impl RichPaneEvents {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append the events the rich pane drained this frame (called by [`RichEditorPaneMount::render`]).
    fn push_all(&self, events: Vec<EditorEvent>) {
        if events.is_empty() {
            return;
        }
        if let Ok(mut q) = self.inner.lock() {
            q.extend(events);
        }
    }

    /// Take every queued event (the shell calls this once per frame to route them). Leaves the queue
    /// empty so an event is routed exactly once (no double-route, no leak).
    pub fn take(&self) -> Vec<EditorEvent> {
        match self.inner.lock() {
            Ok(mut q) => std::mem::take(&mut *q),
            Err(p) => std::mem::take(&mut *p.into_inner()),
        }
    }

    /// Whether any event is currently queued (tests / diagnostics).
    pub fn is_empty(&self) -> bool {
        self.inner.lock().map(|q| q.is_empty()).unwrap_or(true)
    }
}

/// The mounted code editor's document-to-panel map. Each open code tab owns an independent
/// [`CodeEditorPanel`], so opening a definition in another file cannot replace the source tab's
/// unsaved buffer, undo history, breakpoints, language mode, or LSP state. The empty key is the
/// original untitled panel; file-backed tabs use their normalized path as `content_id`.
pub struct CodeEditorDocumentStore {
    base_panel: Arc<CodeEditorPanel>,
    panels: Mutex<BTreeMap<String, Arc<CodeEditorPanel>>>,
    session: SharedSessionContext,
    command_sender: std::sync::mpsc::Sender<CodeEditorHostCommand>,
    editor_action_registry: Mutex<
        Option<Arc<Mutex<crate::accessibility::editor_action_registry::EditorActionRegistry>>>,
    >,
    code_nav_client: Mutex<Option<crate::code_editor::code_nav::CodeNavClient>>,
    lsp_clients_by_language:
        Mutex<BTreeMap<String, Arc<crate::code_editor::lsp_client::LspClient>>>,
    /// Quiet off-UI-thread retirement workers for clients whose last matching language panel closed or
    /// changed language. App Drop joins every outstanding worker before the Tokio runtime is destroyed.
    retired_lsp_shutdown_workers: Mutex<Vec<std::thread::JoinHandle<()>>>,
}

/// The mounted rich editor's document-to-state map. The original untitled/demo Notes surface keeps
/// `base_state`; every non-empty tab view owns a separate [`RichEditorState`] for selection, scroll,
/// popups, and accessibility state. Split views of one document synchronize through one deterministic
/// document authority state that alone owns the document tree, undo, save, and draft coordinators.
pub struct RichEditorDocumentStore {
    base_state: Arc<Mutex<RichEditorState>>,
    /// `(document_id, pane_id)` keeps two split views of the same document from sharing selection,
    /// scroll, popup, or accessibility-registration state.
    states: Mutex<BTreeMap<(String, String), Arc<Mutex<RichEditorState>>>>,
    /// One document-level authority view per document. Its `doc`, `undo`, `save`, and `draft` fields
    /// are the canonical shared editing core; every other split view keeps only view-local state.
    document_authority_views: Mutex<BTreeMap<String, String>>,
    /// The one visible rich view that owns the unsuffixed AccessKit/action namespace. The host prepares
    /// visible bindings in sorted pane/layout order before render, so this never depends on render order.
    canonical_accessibility_view: Mutex<Option<(String, String)>>,
    ready_views: Mutex<std::collections::BTreeSet<(String, String)>>,
    workspace_id: Mutex<String>,
    active_view: Mutex<Option<(String, String)>>,
    editor_action_registry: Mutex<
        Option<Arc<Mutex<crate::accessibility::editor_action_registry::EditorActionRegistry>>>,
    >,
}

impl RichEditorDocumentStore {
    pub fn new(base_state: Arc<Mutex<RichEditorState>>) -> Self {
        Self {
            base_state,
            states: Mutex::new(BTreeMap::new()),
            document_authority_views: Mutex::new(BTreeMap::new()),
            canonical_accessibility_view: Mutex::new(None),
            ready_views: Mutex::new(std::collections::BTreeSet::new()),
            workspace_id: Mutex::new(String::new()),
            active_view: Mutex::new(None),
            editor_action_registry: Mutex::new(None),
        }
    }

    pub fn base_state(&self) -> Arc<Mutex<RichEditorState>> {
        Arc::clone(&self.base_state)
    }

    /// Return the state for a tab content id, creating a document-isolated state on first use. A new
    /// state inherits operator-wide presentation/input preferences from the base state, but none of
    /// its document, selection, undo, save/draft, popup, or async-document state.
    pub fn state_for_content_id(&self, content_id: Option<&str>) -> Arc<Mutex<RichEditorState>> {
        let Some(key) = content_id.filter(|key| !key.trim().is_empty()) else {
            return self.base_state();
        };
        if let Some(state) = self.canonical_state_for_document(key) {
            return state;
        }
        self.state_for_view(Some(key), "primary")
    }

    /// Return the independent editor/view state for one concrete pane rendering one document.
    pub fn state_for_view(
        &self,
        content_id: Option<&str>,
        pane_id: &str,
    ) -> Arc<Mutex<RichEditorState>> {
        let Some(document_id) = content_id.filter(|key| !key.trim().is_empty()) else {
            return self.base_state();
        };
        let key = (document_id.to_owned(), pane_id.to_owned());
        // Hold the map lock through construction/registration. PaneFactory rendering is single-threaded
        // today, but this keeps a future parallel host from constructing the same first document twice
        // and assigning two states the canonical registry instance 0.
        let mut states = self.states.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = states.get(&key).cloned() {
            return state;
        }

        // A file-backed document must never expose the demo fixture while its authoritative GET is in
        // flight. The host renders this blank state through a non-editable loading gate until install.
        let mut state =
            RichEditorState::new(crate::rich_editor::document_model::node::BlockNode::doc(
                vec![crate::rich_editor::document_model::node::BlockNode::paragraph("")],
            ));
        {
            let base = self.base_state.lock().unwrap_or_else(|e| e.into_inner());
            state.theme = base.theme;
            state.editor_font_size = base.editor_font_size;
            state.rich_keymap = base.rich_keymap.clone();
            state.reading_mode_default = base.reading_mode_default;
            state.actor_id = base.actor_id.clone();
            state.tag_list = base.tag_list.clone();
        }
        // Start namespaced. `prepare_visible_views` deterministically promotes exactly one visible
        // pane to the canonical unsuffixed namespace after the complete restored layout is known.
        state.accessibility_namespace = Some(document_instance_namespace(pane_id));
        if let Some(registry) = self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            let namespace = document_instance_namespace(&format!("{document_id}\0{pane_id}"));
            let handle = registry
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .register_named(
                    crate::accessibility::editor_action_registry::PaneType::Rich,
                    namespace,
                );
            state.editor_actions = Some((registry, handle));
        }
        let state = Arc::new(Mutex::new(state));
        states.insert(key, Arc::clone(&state));
        self.document_authority_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(document_id.to_owned())
            .or_insert_with(|| pane_id.to_owned());
        state
    }

    /// Prepare the complete set of visible Notes bindings before any pane renders. This makes both the
    /// unsuffixed accessibility/action owner and each document's shared editing authority deterministic
    /// from stable pane identity, independent of construction or render order.
    pub fn prepare_visible_views(&self, bindings: &[(String, String)]) {
        let mut ordered = bindings.to_vec();
        ordered.sort();
        ordered.dedup();
        for (pane_id, document_id) in &ordered {
            self.state_for_view(Some(document_id), pane_id);
        }

        let mut authority_by_document = BTreeMap::<String, String>::new();
        for (pane_id, document_id) in &ordered {
            authority_by_document
                .entry(document_id.clone())
                .and_modify(|current| {
                    if pane_id < current {
                        *current = pane_id.clone();
                    }
                })
                .or_insert_with(|| pane_id.clone());
        }
        for (document_id, pane_id) in authority_by_document {
            self.set_document_authority_view(&document_id, &pane_id);
        }

        let desired_canonical = ordered
            .first()
            .cloned()
            .map(|(pane_id, document_id)| (document_id, pane_id));
        let mut current = self
            .canonical_accessibility_view
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if *current == desired_canonical {
            return;
        }
        *current = desired_canonical.clone();
        drop(current);

        let Some(registry) = self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        else {
            return;
        };

        // Once a file-backed Notes view exists, the untitled state moves to a stable named action
        // namespace so the canonical unsuffixed registration belongs to the deterministic visible view.
        {
            let mut base = self.base_state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((base_registry, handle)) = base.editor_actions.take() {
                base_registry
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove_registration(&handle);
            }
            if desired_canonical.is_some() {
                let mut guard = registry.lock().unwrap_or_else(|e| e.into_inner());
                let handle = guard.register_named(
                    crate::accessibility::editor_action_registry::PaneType::Rich,
                    "untitled",
                );
                drop(guard);
                base.editor_actions = Some((Arc::clone(&registry), handle));
            } else {
                base.install_editor_action_registry(Arc::clone(&registry), 0);
            }
        }

        let states: Vec<((String, String), Arc<Mutex<RichEditorState>>)> = self
            .states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|(key, state)| (key.clone(), Arc::clone(state)))
            .collect();
        for ((document_id, pane_id), state) in states {
            let is_canonical =
                desired_canonical.as_ref() == Some(&(document_id.clone(), pane_id.clone()));
            let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
            state.accessibility_namespace =
                (!is_canonical).then(|| document_instance_namespace(&pane_id));
            if let Some((old_registry, handle)) = state.editor_actions.take() {
                old_registry
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove_registration(&handle);
            }
            if is_canonical {
                state.install_editor_action_registry(Arc::clone(&registry), 0);
            } else {
                let namespace = document_instance_namespace(&format!("{document_id}\0{pane_id}"));
                let handle = registry
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .register_named(
                        crate::accessibility::editor_action_registry::PaneType::Rich,
                        namespace,
                    );
                state.editor_actions = Some((Arc::clone(&registry), handle));
            }
        }
    }

    fn set_document_authority_view(&self, document_id: &str, pane_id: &str) {
        let previous_pane = self
            .document_authority_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(document_id)
            .cloned();
        if previous_pane.as_deref() == Some(pane_id) {
            return;
        }
        let states = self.states.lock().unwrap_or_else(|e| e.into_inner());
        let previous = previous_pane
            .as_ref()
            .and_then(|previous_pane| states.get(&(document_id.to_owned(), previous_pane.clone())))
            .cloned();
        let next = states
            .get(&(document_id.to_owned(), pane_id.to_owned()))
            .cloned();
        drop(states);
        let Some(next) = next else { return };
        if let Some(previous) = previous {
            let (doc, undo, save, draft, pending_stage_embed_save, pending_stage_embed_completion) = {
                let mut previous = previous.lock().unwrap_or_else(|e| e.into_inner());
                (
                    previous.doc.clone(),
                    previous.undo.clone(),
                    previous.save.take(),
                    previous.draft.take(),
                    previous.pending_stage_embed_save.take(),
                    previous.pending_stage_embed_completion.take(),
                )
            };
            let mut next = next.lock().unwrap_or_else(|e| e.into_inner());
            next.doc = doc;
            next.undo = undo;
            next.save = save;
            next.draft = draft;
            next.pending_stage_embed_save = pending_stage_embed_save;
            next.pending_stage_embed_completion = pending_stage_embed_completion;
        }
        self.document_authority_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(document_id.to_owned(), pane_id.to_owned());
    }

    /// Insert a Stage hsLink at one retained split view's caret, publish that exact mutation into the
    /// stable document authority, and request the canonical save there. Authority is deliberately NOT
    /// reassigned: `prepare_visible_views` may recompute it next frame, so durability must already live on
    /// the deterministic canonical state before this method returns.
    pub fn insert_stage_embed_at_view_and_request_canonical_save(
        &self,
        document_id: &str,
        pane_id: &str,
        pending: crate::rich_editor::renderer::rich_editor_widget::PendingStageEmbedSave,
    ) -> Result<(), String> {
        let canonical = self
            .canonical_state_for_document(document_id)
            .ok_or_else(|| "Stage embed target has no canonical document authority".to_owned())?;
        {
            let canonical = canonical.lock().unwrap_or_else(|e| e.into_inner());
            crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget::ensure_stage_embed_save_available(
                &canonical,
            )?;
        }

        self.synchronize_view_from_canonical(document_id, pane_id);
        let target = self.state_for_view(Some(document_id), pane_id);
        {
            let mut target = target.lock().unwrap_or_else(|e| e.into_inner());
            if !crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget::insert_atelier_embed_at_caret(
                &mut target,
                pending.link.clone(),
            ) {
                return Err(
                    "Stage capture fetched but the retained note view rejected the embed insertion"
                        .to_owned(),
                );
            }
        }
        self.publish_view_to_canonical(document_id, pane_id);

        let canonical = self
            .canonical_state_for_document(document_id)
            .ok_or_else(|| "Stage embed canonical authority disappeared before save".to_owned())?;
        let mut canonical = canonical.lock().unwrap_or_else(|e| e.into_inner());
        crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget::request_existing_stage_embed_save(
            &mut canonical,
            pending,
        )
    }

    pub fn canonical_view_key(&self, document_id: &str) -> Option<(String, String)> {
        self.document_authority_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(document_id)
            .cloned()
            .map(|pane_id| (pane_id, document_id.to_owned()))
    }

    pub fn canonical_state_for_document(
        &self,
        document_id: &str,
    ) -> Option<Arc<Mutex<RichEditorState>>> {
        let pane_id = self
            .document_authority_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(document_id)
            .cloned()?;
        self.states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&(document_id.to_owned(), pane_id))
            .cloned()
    }

    pub fn is_document_ready(&self, document_id: &str) -> bool {
        self.ready_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|(ready_document_id, _)| ready_document_id == document_id)
    }

    pub fn has_other_ready_view(&self, document_id: &str, pane_id: &str) -> bool {
        self.ready_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|(ready_document_id, ready_pane_id)| {
                ready_document_id == document_id && ready_pane_id != pane_id
            })
    }

    pub fn mark_document_views_ready(&self, document_id: &str) {
        let panes: Vec<String> = self
            .states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .filter_map(|(state_document_id, pane_id)| {
                (state_document_id == document_id).then(|| pane_id.clone())
            })
            .collect();
        let mut ready = self.ready_views.lock().unwrap_or_else(|e| e.into_inner());
        ready.extend(
            panes
                .into_iter()
                .map(|pane_id| (document_id.to_owned(), pane_id)),
        );
    }

    /// Copy only the shared document/undo core into one non-authority view. Selection, scroll,
    /// popups, find state, and other interaction state remain owned by that pane.
    pub fn synchronize_view_from_canonical(&self, document_id: &str, pane_id: &str) {
        let Some((authority_pane, _)) = self.canonical_view_key(document_id) else {
            return;
        };
        if authority_pane == pane_id {
            return;
        }
        let Some(authority) = self.canonical_state_for_document(document_id) else {
            return;
        };
        let view = self.state_for_view(Some(document_id), pane_id);
        let (doc, undo) = {
            let authority = authority.lock().unwrap_or_else(|e| e.into_inner());
            (authority.doc.clone(), authority.undo.clone())
        };
        let mut view = view.lock().unwrap_or_else(|e| e.into_inner());
        view.doc = doc;
        view.undo = undo;
        view.save = None;
        view.draft = None;
    }

    /// Publish a rendered view's document mutation back to the one canonical document/save authority.
    pub fn publish_view_to_canonical(&self, document_id: &str, pane_id: &str) {
        let Some((authority_pane, _)) = self.canonical_view_key(document_id) else {
            return;
        };
        if authority_pane == pane_id {
            return;
        }
        let view = self.state_for_view(Some(document_id), pane_id);
        let (doc, undo) = {
            let view = view.lock().unwrap_or_else(|e| e.into_inner());
            (view.doc.clone(), view.undo.clone())
        };
        let Some(authority) = self.canonical_state_for_document(document_id) else {
            return;
        };
        let mut authority = authority.lock().unwrap_or_else(|e| e.into_inner());
        let changed = authority.doc != doc;
        authority.doc = doc;
        authority.undo = undo;
        if changed {
            if let Some(save) = authority.save.as_mut() {
                save.mark_dirty();
            }
            if let Some(draft) = authority.draft.as_mut() {
                draft.mark_dirty(std::time::Instant::now());
            }
        }
    }

    /// WP-KERNEL-012 MT-043: replace one exact PostgreSQL-backed rich-note code block with the text
    /// authored in its mounted native [`CodeEditorPanel`], then dispatch the SAME MT-020
    /// [`SaveManager`](crate::rich_editor::save::save_manager::SaveManager) used by rich-editor
    /// Ctrl+S. The caller must supply the original model path, complete owning-document structural
    /// snapshot, and exact text snapshot bound when the code panel opened. If any is stale, the
    /// operation fails visibly instead of finding another code block or overwriting unrelated content.
    ///
    /// Returns `(expected_doc_version, canonical_block_path, post_edit_document_snapshot)` for host
    /// completion correlation. The save itself remains asynchronous and is completed by the ordinary
    /// rich-state frame drain.
    pub fn replace_code_block_and_request_save(
        &self,
        document_id: &str,
        block_path: &[usize],
        expected_document_snapshot: &serde_json::Value,
        expected_text: &str,
        replacement: &str,
    ) -> Result<(u64, Vec<usize>, serde_json::Value), String> {
        use crate::rich_editor::document_model::node::{Child, NodeKind};
        use crate::rich_editor::document_model::transform::{
            apply_transaction, ActorKind, Step, Transaction,
        };

        let state = self
            .canonical_state_for_document(document_id)
            .ok_or_else(|| format!("rich document '{document_id}' is not mounted"))?;
        let mut state = state.lock().unwrap_or_else(|error| error.into_inner());
        let save = state.save.as_ref().ok_or_else(|| {
            format!("rich document '{document_id}' has no canonical save binding")
        })?;
        if save.is_saving() {
            return Err(format!(
                "rich document '{document_id}' already has a canonical save in flight"
            ));
        }
        if save.has_conflict() {
            return Err(format!(
                "rich document '{document_id}' has an unresolved save conflict"
            ));
        }
        let expected_doc_version = save.doc_version;

        let current_document_snapshot =
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
        if &current_document_snapshot != expected_document_snapshot {
            return Err(format!(
                "rich document '{document_id}' changed structurally after the Code Editor opened"
            ));
        }

        let mut block = &state.doc;
        for &index in block_path {
            block = block
                .children
                .get(index)
                .and_then(Child::as_block)
                .ok_or_else(|| format!("rich code block path {block_path:?} is stale"))?;
        }
        if !matches!(block.kind, NodeKind::CodeBlock) {
            return Err(format!(
                "rich block at path {block_path:?} is no longer a code block"
            ));
        }
        if block.children.len() != 1 {
            return Err(format!(
                "rich code block at path {block_path:?} has an invalid inline shape"
            ));
        }
        let current_text = block
            .children
            .first()
            .and_then(Child::as_text)
            .ok_or_else(|| format!("rich code block at path {block_path:?} has no text leaf"))?
            .text
            .to_string();
        if current_text != expected_text {
            return Err(format!(
                "rich code block at path {block_path:?} changed after the Code Editor opened"
            ));
        }

        if current_text != replacement {
            let mut leaf_path = block_path.to_vec();
            leaf_path.push(0);
            let mut steps = Vec::with_capacity(2);
            let current_chars = current_text.chars().count();
            if current_chars != 0 {
                steps.push(Step::DeleteText {
                    path: leaf_path.clone(),
                    start: 0,
                    end: current_chars,
                });
            }
            if !replacement.is_empty() {
                steps.push(Step::InsertText {
                    path: leaf_path,
                    char_offset: 0,
                    text: replacement.to_owned(),
                });
            }

            let before =
                crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
            let actor_id = state.actor_id.clone();
            let transaction = Transaction::new(steps, ActorKind::Agent, actor_id);
            let receipt = apply_transaction(&mut state.doc, transaction).map_err(|error| {
                format!("could not update rich code block at path {block_path:?}: {error}")
            })?;
            state.undo.push(receipt);
            let after =
                crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
            state.pending_bus_undo.push((before, after));
            if let Some(save) = state.save.as_mut() {
                save.mark_dirty();
            }
            if let Some(draft) = state.draft.as_mut() {
                draft.mark_dirty(std::time::Instant::now());
            }
        }

        if !state.request_save_for_host() {
            return Err(format!(
                "rich document '{document_id}' lost its canonical save binding"
            ));
        }
        let post_edit_document_snapshot =
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc);
        Ok((
            expected_doc_version,
            block_path.to_vec(),
            post_edit_document_snapshot,
        ))
    }

    pub fn contains(&self, content_id: &str) -> bool {
        !content_id.trim().is_empty()
            && self
                .states
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .keys()
                .any(|(document_id, _)| document_id == content_id)
    }

    pub fn states(&self) -> Vec<Arc<Mutex<RichEditorState>>> {
        let mut states = vec![self.base_state()];
        states.extend(
            self.states
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .values()
                .cloned(),
        );
        states
    }

    pub fn states_for_content_id(&self, content_id: &str) -> Vec<Arc<Mutex<RichEditorState>>> {
        self.states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter_map(|((document_id, _), state)| {
                (document_id == content_id).then(|| Arc::clone(state))
            })
            .collect()
    }

    pub fn view_keys_for_content_id(&self, content_id: &str) -> Vec<(String, String)> {
        self.states
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .filter_map(|(document_id, pane_id)| {
                (document_id == content_id).then(|| (pane_id.clone(), document_id.clone()))
            })
            .collect()
    }

    pub fn set_active_view(&self, content_id: Option<&str>, pane_id: Option<&str>) {
        *self.active_view.lock().unwrap_or_else(|e| e.into_inner()) = content_id
            .filter(|id| !id.trim().is_empty())
            .zip(pane_id)
            .map(|(document_id, pane_id)| (document_id.to_owned(), pane_id.to_owned()));
    }

    pub fn is_view_ready(&self, content_id: &str, pane_id: &str) -> bool {
        self.ready_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&(content_id.to_owned(), pane_id.to_owned()))
    }

    pub fn mark_view_ready(&self, content_id: &str, pane_id: &str) {
        self.ready_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((content_id.to_owned(), pane_id.to_owned()));
    }

    pub fn mark_view_loading(&self, content_id: &str, pane_id: &str) {
        self.ready_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&(content_id.to_owned(), pane_id.to_owned()));
    }

    /// Bind the document cache to one workspace. A workspace switch retires every file-backed state
    /// and resets the base/untitled state while preserving operator-global presentation/input prefs and
    /// the shared action registry. This prevents identical document ids in two workspaces from sharing
    /// editor, save/draft, resolver, or async state.
    pub fn bind_workspace(&self, workspace_id: &str) -> bool {
        let mut bound = self.workspace_id.lock().unwrap_or_else(|e| e.into_inner());
        if bound.as_str() == workspace_id {
            return false;
        }
        *bound = workspace_id.to_owned();

        let registry = self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let mut states = self.states.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(registry) = registry.as_ref() {
            let mut registry = registry.lock().unwrap_or_else(|e| e.into_inner());
            for state in states.values() {
                if let Some((_, handle)) = state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .editor_actions
                    .take()
                {
                    registry.remove_registration(&handle);
                }
            }
        }
        states.clear();
        self.document_authority_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        *self
            .canonical_accessibility_view
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.ready_views
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        *self.active_view.lock().unwrap_or_else(|e| e.into_inner()) = None;

        let mut base = self.base_state.lock().unwrap_or_else(|e| e.into_inner());
        let theme = base.theme;
        let editor_font_size = base.editor_font_size;
        let rich_keymap = base.rich_keymap.clone();
        let reading_mode_default = base.reading_mode_default;
        let actor_id = base.actor_id.clone();
        let tag_list = base.tag_list.clone();
        if let Some((base_registry, handle)) = base.editor_actions.take() {
            base_registry
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove_registration(&handle);
        }
        let mut reset = RichEditorState::demo();
        reset.theme = theme;
        reset.editor_font_size = editor_font_size;
        reset.rich_keymap = rich_keymap;
        reset.reading_mode_default = reading_mode_default;
        reset.actor_id = actor_id;
        reset.tag_list = tag_list;
        if let Some(registry) = registry {
            reset.install_editor_action_registry(registry, 0);
        }
        *base = reset;
        true
    }

    pub fn active_state(&self) -> Arc<Mutex<RichEditorState>> {
        let active_view = self
            .active_view
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        match active_view {
            Some((document_id, pane_id)) => self.state_for_view(Some(&document_id), &pane_id),
            None => self.base_state(),
        }
    }

    pub fn install_editor_action_registry(
        &self,
        registry: Arc<Mutex<crate::accessibility::editor_action_registry::EditorActionRegistry>>,
    ) {
        {
            let mut base = self.base_state.lock().unwrap_or_else(|e| e.into_inner());
            base.install_editor_action_registry(Arc::clone(&registry), 0);
        }
        *self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(registry);
    }
}

/// Encode the complete normalized document identity into the panel namespace. Unlike an insertion
/// counter or a truncated hash, this mapping is deterministic, independent of open order, and
/// collision-free for distinct UTF-8 identities.
fn document_instance_namespace(content_id: &str) -> String {
    let mut namespace = String::with_capacity(9 + content_id.len() * 2);
    namespace.push_str("document-");
    for byte in content_id.as_bytes() {
        use std::fmt::Write as _;
        let _ = write!(namespace, "{byte:02x}");
    }
    namespace
}

impl CodeEditorDocumentStore {
    pub fn new(
        base_panel: Arc<CodeEditorPanel>,
        session: SharedSessionContext,
        command_sender: std::sync::mpsc::Sender<CodeEditorHostCommand>,
    ) -> Self {
        Self {
            base_panel,
            panels: Mutex::new(BTreeMap::new()),
            session,
            command_sender,
            editor_action_registry: Mutex::new(None),
            code_nav_client: Mutex::new(None),
            lsp_clients_by_language: Mutex::new(BTreeMap::new()),
            retired_lsp_shutdown_workers: Mutex::new(Vec::new()),
        }
    }

    pub fn base_panel(&self) -> Arc<CodeEditorPanel> {
        Arc::clone(&self.base_panel)
    }

    /// Return the panel for a tab content id. An absent/empty id is the original untitled panel.
    pub fn panel_for_content_id(&self, content_id: Option<&str>) -> Arc<CodeEditorPanel> {
        let Some(key) = content_id.filter(|key| !key.is_empty()) else {
            return self.base_panel();
        };
        self.panels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
            .unwrap_or_else(|| self.base_panel())
    }

    pub fn activate_base_document(&self) {
        if self.base_panel.has_editor_action_registry() {
            return;
        }
        if let Some(registry) = self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            self.base_panel.install_editor_action_registry(registry, 0);
        }
    }

    pub fn contains(&self, content_id: &str) -> bool {
        !content_id.is_empty()
            && self
                .panels
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(content_id)
    }

    pub fn panels(&self) -> Vec<Arc<CodeEditorPanel>> {
        let mut panels = vec![self.base_panel()];
        panels.extend(
            self.panels
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .values()
                .cloned(),
        );
        panels
    }

    /// Every mounted code document paired with its stable tab/store identity. The base panel uses the
    /// empty identity; file-backed cross-file panels use their normalized content id.
    pub fn document_panels(&self) -> Vec<(String, Arc<CodeEditorPanel>)> {
        let mut documents = vec![(String::new(), self.base_panel())];
        documents.extend(
            self.panels
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .iter()
                .map(|(content_id, panel)| (content_id.clone(), Arc::clone(panel))),
        );
        documents
    }

    /// Deterministic collision-free AccessKit namespace for a document identity (test/diagnostic
    /// seam used to prove the same files receive the same identities regardless of open order).
    pub fn instance_namespace_for_content_id(content_id: &str) -> String {
        document_instance_namespace(content_id)
    }

    /// Remove a file-backed panel only after its tab has actually closed. The base/untitled panel is
    /// never removed. Returns the removed panel so the host can issue `textDocument/didClose` using
    /// its exact URI and client without guessing which language server owned it.
    pub fn remove_document(&self, content_id: &str) -> Option<Arc<CodeEditorPanel>> {
        if content_id.is_empty() {
            return None;
        }
        let removed = self
            .panels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(content_id);
        if let Some(panel) = removed.as_ref() {
            panel.uninstall_editor_action_registry();
        }
        removed
    }

    /// Install one backend code-navigation client across all current panels and retain it for every
    /// subsequently opened document.
    pub fn install_code_nav_client(&self, client: crate::code_editor::code_nav::CodeNavClient) {
        for panel in self.panels() {
            panel.set_code_nav_client(client.clone());
        }
        *self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(client);
    }

    /// Install an LSP client only on documents with the matching detected language. The binding stays
    /// shared across every open same-language tab, but registry entries with no matching panel are
    /// retired so an inactive language process cannot outlive its last consumer.
    pub fn install_lsp_client_for_language(
        &self,
        language_id: &str,
        client: Arc<crate::code_editor::lsp_client::LspClient>,
    ) {
        let panels = self.panels();
        for panel in &panels {
            if panel.resolved_language().detected.as_str() == language_id {
                panel.set_lsp_client(Arc::clone(&client));
            }
        }
        let mounted_languages: std::collections::HashSet<String> = panels
            .iter()
            .map(|panel| panel.resolved_language().detected.as_str().to_owned())
            .collect();
        let retired = {
            let mut clients = self
                .lsp_clients_by_language
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut retirement_candidates = clients
                .insert(language_id.to_owned(), Arc::clone(&client))
                .filter(|old| !Arc::ptr_eq(old, &client))
                .into_iter()
                .collect::<Vec<_>>();
            let stale_languages: Vec<String> = clients
                .keys()
                .filter(|registered| {
                    registered.as_str() != language_id
                        && !mounted_languages.contains(registered.as_str())
                })
                .cloned()
                .collect();
            retirement_candidates.extend(
                stale_languages
                    .into_iter()
                    .filter_map(|stale| clients.remove(&stale)),
            );
            retirement_candidates.retain(|candidate| {
                !clients
                    .values()
                    .any(|current| Arc::ptr_eq(candidate, current))
            });
            retirement_candidates
        };
        // Rebinding one language retires its previous process, and a panel language change retires
        // registry entries that no mounted document can consume. Explicit shutdown is required even
        // when an observer/test still holds an Arc. A client shared by another language remains owned.
        self.retire_lsp_clients(retired);
    }

    /// Reclaim every language-server transport owned by the document store while the app's Tokio
    /// runtime is still alive. Clients are gathered from both the language registry and every mounted
    /// panel because a panel may transiently retain a replaced client. Pointer de-duplication keeps a
    /// same-language multi-panel binding from running the bounded shutdown sequence more than once.
    pub fn shutdown_all_lsp_clients(&self) {
        let mut clients: Vec<Arc<crate::code_editor::lsp_client::LspClient>> = {
            let mut registered = self
                .lsp_clients_by_language
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *registered).into_values().collect()
        };
        clients.extend(self.panels().into_iter().map(|panel| panel.lsp_client()));

        let mut seen = std::collections::HashSet::new();
        for client in clients {
            if seen.insert(Arc::as_ptr(&client)) {
                client.shutdown_for_host();
            }
        }
        let workers = std::mem::take(
            &mut *self
                .retired_lsp_shutdown_workers
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        );
        for worker in workers {
            let _ = worker.join();
        }
    }

    /// Reconcile retained language clients with the languages of currently open document panels.
    /// This is the app-frame language-change path: a panel that changes Rust -> Python is rebound to
    /// an already-retained Python client when available, and a Rust client with no remaining open Rust
    /// panel is removed and explicitly shut down before the host runtime can outlive it.
    pub fn reconcile_lsp_clients_for_languages(
        &self,
        open_languages: &std::collections::BTreeSet<String>,
    ) -> Vec<String> {
        let panels = self.panels();
        let (bindings, retired_languages, retired) = {
            let mut registered = self
                .lsp_clients_by_language
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let stale_languages: Vec<String> = registered
                .keys()
                .filter(|language| !open_languages.contains(language.as_str()))
                .cloned()
                .collect();
            let retired: Vec<_> = stale_languages
                .iter()
                .filter_map(|language| registered.remove(language))
                .collect();
            let bindings = registered
                .iter()
                .map(|(language, client)| (language.clone(), Arc::clone(client)))
                .collect::<BTreeMap<_, _>>();
            (bindings, stale_languages, retired)
        };

        for panel in panels {
            let language = panel.resolved_language().detected.as_str().to_owned();
            let Some(client) = bindings.get(&language) else {
                continue;
            };
            let current = panel.lsp_client();
            if !Arc::ptr_eq(&current, client) {
                panel.set_lsp_client(Arc::clone(client));
            }
        }

        let retired = retired
            .into_iter()
            .filter(|client| {
                !bindings
                    .values()
                    .any(|retained| Arc::ptr_eq(client, retained))
            })
            .collect();
        self.retire_lsp_clients(retired);
        retired_languages
    }

    /// Start bounded client shutdown away from the egui frame path. Each LSP client owns the runtime
    /// handle needed for graceful shutdown; the app-owned worker handles are joined by
    /// [`shutdown_all_lsp_clients`](Self::shutdown_all_lsp_clients) before runtime teardown.
    fn retire_lsp_clients(&self, clients: Vec<Arc<crate::code_editor::lsp_client::LspClient>>) {
        let mut seen = std::collections::HashSet::new();
        let mut workers = self
            .retired_lsp_shutdown_workers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // Finished threads no longer need a join handle retained between frame-driven rebinds.
        workers.retain(|worker| !worker.is_finished());
        for client in clients {
            if !seen.insert(Arc::as_ptr(&client)) {
                continue;
            }
            if let Ok(worker) = std::thread::Builder::new()
                .name("handshake-lsp-retire".to_owned())
                .spawn(move || client.shutdown_for_host())
            {
                workers.push(worker);
            }
        }
    }

    /// Install the shared AccessKit editor-action registry on every existing panel and remember it for
    /// future file-backed panels. Instance 0 belongs to the base panel; file-backed action-registry
    /// instances derive from document identity rather than insertion order.
    pub fn install_editor_action_registry(
        &self,
        registry: Arc<Mutex<crate::accessibility::editor_action_registry::EditorActionRegistry>>,
    ) {
        self.base_panel
            .install_editor_action_registry(Arc::clone(&registry), 0);
        let panels = self.panels.lock().unwrap_or_else(|e| e.into_inner());
        for (content_id, panel) in panels.iter() {
            panel.install_editor_action_registry_named(
                Arc::clone(&registry),
                document_instance_namespace(content_id),
            );
        }
        *self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(registry);
    }

    fn wire_panel(&self, panel: &CodeEditorPanel, document_id: &str) -> bool {
        panel.set_command_palette_sender(self.command_sender.clone(), document_id);
        let session = self.session.lock().map(|c| c.clone()).unwrap_or_default();
        if let (true, Some(runtime)) = (session.is_bound(), session.runtime) {
            panel.set_runtime(runtime);
            panel.set_workspace_id(session.workspace_id);
            true
        } else {
            false
        }
    }

    /// Insert a successfully loaded file as an independent editor document. If the file is already
    /// open, its live panel is returned unchanged: a second definition jump must never overwrite edits
    /// made in that target tab with a fresh disk snapshot.
    pub fn insert_loaded_document(
        &self,
        content_id: String,
        file_path: &Path,
        text: &str,
    ) -> Arc<CodeEditorPanel> {
        if let Some(existing) = self
            .panels
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&content_id)
            .cloned()
        {
            return existing;
        }

        let extension = file_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let instance_namespace = document_instance_namespace(&content_id);
        let panel = Arc::new(CodeEditorPanel::with_instance(
            text,
            extension,
            instance_namespace,
        ));
        panel.load_file(file_path.to_string_lossy().to_string());
        self.wire_panel(&panel, &content_id);
        if let Some(client) = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            panel.set_code_nav_client(client);
        }
        let language_id = panel.resolved_language().detected.as_str().to_owned();
        if let Some(client) = self
            .lsp_clients_by_language
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&language_id)
            .cloned()
        {
            panel.set_lsp_client(client);
        }
        if let Some(registry) = self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            panel.install_editor_action_registry_named(
                registry,
                document_instance_namespace(&content_id),
            );
        }

        let mut panels = self.panels.lock().unwrap_or_else(|e| e.into_inner());
        Arc::clone(
            panels
                .entry(content_id)
                .or_insert_with(|| Arc::clone(&panel)),
        )
    }

    /// Mount one rich-note code block as a first-class native code document without assigning a
    /// filesystem path. Its durable authority remains the owning rich document; the host recognizes
    /// the stable `content_id` binding and routes `editor.code.save` through that document's E6
    /// SaveManager path rather than the local-file atomic-save path.
    pub fn insert_rich_code_block(
        &self,
        content_id: String,
        language: &str,
        text: &str,
    ) -> Arc<CodeEditorPanel> {
        if let Some(existing) = self
            .panels
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&content_id)
            .cloned()
        {
            return existing;
        }

        let panel = Arc::new(CodeEditorPanel::with_instance(
            text,
            language,
            document_instance_namespace(&content_id),
        ));
        self.wire_panel(&panel, &content_id);
        if let Some(client) = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
        {
            panel.set_code_nav_client(client);
        }
        let language_id = panel.resolved_language().detected.as_str().to_owned();
        if let Some(client) = self
            .lsp_clients_by_language
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&language_id)
            .cloned()
        {
            panel.set_lsp_client(client);
        }
        if let Some(registry) = self
            .editor_action_registry
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
        {
            panel.install_editor_action_registry_named(
                registry,
                document_instance_namespace(&content_id),
            );
        }

        let mut panels = self
            .panels
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Arc::clone(
            panels
                .entry(content_id)
                .or_insert_with(|| Arc::clone(&panel)),
        )
    }
}

/// The session-threaded CODE-editor pane factory. Registered over `PaneType::CodeSymbol` (the code
/// surface the WP-011 shell already routes a "code" pane to — NOT a new `PaneType` variant, which would
/// ripple through every exhaustive `PaneType` match; RISK-079-5). Wraps the existing
/// [`CodeEditorPaneFactory`] (the real per-frame bus-consumer + undo-recording render) and, on the first
/// render with a bound session context, threads `set_runtime` + `set_workspace_id` into the panel and
/// installs the shell command sender. The wrap keeps the bus/undo wiring the inner factory already
/// proves; this layer only adds the session-context threading the host-mount needs.
pub struct CodeEditorPaneMount {
    documents: Arc<CodeEditorDocumentStore>,
    /// `true` once the panel has been threaded with a BOUND session context (so the threading runs once,
    /// not every frame). Atomic because `render` is `&self`.
    wired: std::sync::atomic::AtomicBool,
}

impl CodeEditorPaneMount {
    /// Build the mount over `panel`, the live `session` cell, and the shell `command_sender`. The inner
    /// [`CodeEditorPaneFactory`] is constructed from a CLONE of the same `Arc<CodeEditorPanel>`, so the
    /// mount's `set_*` calls and the inner factory's render drive the SAME panel state.
    pub fn new(
        panel: Arc<CodeEditorPanel>,
        session: SharedSessionContext,
        command_sender: std::sync::mpsc::Sender<CodeEditorHostCommand>,
    ) -> Self {
        let documents = Arc::new(CodeEditorDocumentStore::new(panel, session, command_sender));
        Self::from_document_store(documents)
    }

    pub fn from_document_store(documents: Arc<CodeEditorDocumentStore>) -> Self {
        Self {
            documents,
            wired: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// The Arc-shared panel behind this mount (so a test/host can drive the SAME panel state the mounted
    /// pane shows — the AC-079 proofs need the real panel behind the factory).
    pub fn panel(&self) -> Arc<CodeEditorPanel> {
        self.documents.base_panel()
    }

    pub fn document_store(&self) -> Arc<CodeEditorDocumentStore> {
        Arc::clone(&self.documents)
    }

    /// Whether the panel has been threaded with a bound session context (tests / PT-079-B).
    pub fn is_wired(&self) -> bool {
        self.wired.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Thread the session context + command sender into the panel if not already done. Called at the top
    /// of `render` (and directly by tests). The command sender is installed unconditionally on the first
    /// render (it works even without a runtime — it is a plain channel); the runtime/workspace threading
    /// waits until the session context is BOUND so a half-built context never installs a misleading wired
    /// state (MC-079-1: the mount is honest about what is actually wired).
    pub fn wire_if_needed(&self) {
        if self.documents.wire_panel(&self.documents.base_panel(), "") {
            self.wired.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

impl PaneFactory for CodeEditorPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::CodeSymbol
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        // Thread session context + command sender BEFORE the inner render, so the first live frame
        // already has the runtime/workspace/command bus wired (AC-079-2/3).
        self.wire_if_needed();
        let requested_id = ctx.record.content_id.as_deref().unwrap_or_default();
        let document_id = if self.documents.contains(requested_id) {
            requested_id
        } else {
            ""
        };
        if document_id.is_empty() {
            self.documents.activate_base_document();
        }
        let panel = self
            .documents
            .panel_for_content_id((!document_id.is_empty()).then_some(document_id));
        panel.set_host_render_pane_id(Some(ctx.record.pane_id.clone()));
        self.documents.wire_panel(&panel, document_id);
        // Delegate to the EXISTING code factory render: it publishes selection to the shared bus,
        // registers the code command set, runs the panel, and records the unified-undo entries
        // (push_code_edit_undo) — the real per-frame consumers MT-031/035/050/051 already prove. The
        // mount adds ONLY the session-context threading above; it does not re-implement editor logic.
        CodeEditorPaneFactory::from_arc(panel).render(ui, ctx);
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Document
    }
}

/// The session-threaded RICH-text pane factory. Registered over `PaneType::LoomWikiPage` (the Notes /
/// Obsidian-class wiki surface the WP-011 shell routes the rich editor to — NOT a new `PaneType`
/// variant; RISK-079-5). Wraps the existing [`RichEditorPaneFactory`] (the real per-frame bus-consumer +
/// unified-undo pane-id install) and, on the first render with a bound session context, threads
/// `set_embed_context` (MT-014) + `set_wikilink_context` (MT-057) into the editor state. Each frame,
/// AFTER the editor renders, it DRAINS `RichEditorState.pending_events` and pushes them into the shared
/// [`RichPaneEvents`] queue the shell routes to the nav bus (AC-079-5).
pub struct RichEditorPaneMount {
    documents: Arc<RichEditorDocumentStore>,
    /// The live session-context cell the shell overwrites; read on render.
    session: SharedSessionContext,
    /// The outbound queue the shell drains + routes (AC-079-5). The mount pushes the editor's drained
    /// `pending_events` here after each render.
    events: RichPaneEvents,
    /// Context identity used only by the original untitled/base Notes state. File-backed tabs always
    /// bind their exact `content_id`.
    base_document_id: String,
    /// `true` once the editor state has been threaded with a BOUND session context.
    wired: std::sync::atomic::AtomicBool,
    /// Last workspace/runtime identity applied to the rich editor. The shell may replace the shared
    /// session when switching projects; a one-shot boolean would leave embeds, wikilinks, and code-ref
    /// resolution permanently bound to the prior workspace.
    applied_sessions: Mutex<BTreeMap<String, (String, tokio::runtime::Id)>>,
}

impl RichEditorPaneMount {
    /// Build the mount over the shared editor `state`, the live `session` cell, the shared outbound
    /// `events` queue, and the `document_id` the wikilink context binds to. The inner
    /// [`RichEditorPaneFactory`] wraps a CLONE of the same `Arc<Mutex<RichEditorState>>` so the mount's
    /// threading + drain and the inner render drive the SAME state.
    pub fn new(
        state: Arc<Mutex<RichEditorState>>,
        session: SharedSessionContext,
        events: RichPaneEvents,
        document_id: impl Into<String>,
    ) -> Self {
        let documents = Arc::new(RichEditorDocumentStore::new(state));
        Self::from_document_store(documents, session, events, document_id)
    }

    pub fn from_document_store(
        documents: Arc<RichEditorDocumentStore>,
        session: SharedSessionContext,
        events: RichPaneEvents,
        base_document_id: impl Into<String>,
    ) -> Self {
        Self {
            documents,
            session,
            events,
            base_document_id: base_document_id.into(),
            wired: std::sync::atomic::AtomicBool::new(false),
            applied_sessions: Mutex::new(BTreeMap::new()),
        }
    }

    /// The Arc-shared editor state behind this mount (so a test/host drives the SAME state the mounted
    /// pane shows — the AC-079 proofs need the real state behind the factory).
    pub fn state(&self) -> Arc<Mutex<RichEditorState>> {
        self.documents.base_state()
    }

    pub fn document_store(&self) -> Arc<RichEditorDocumentStore> {
        Arc::clone(&self.documents)
    }

    /// The shared outbound event queue (the shell holds a clone to drain + route).
    pub fn events(&self) -> RichPaneEvents {
        self.events.clone()
    }

    /// Whether the editor state has been threaded with a bound session context (tests / PT-079-B).
    pub fn is_wired(&self) -> bool {
        self.wired.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Thread the session context into the editor state if not already done. Called at the top of
    /// `render` (and directly by tests). Waits until the session context is BOUND (non-empty workspace +
    /// runtime) so a half-built context never installs a misleading wired state. Calls the prior-MT
    /// hooks `set_embed_context` (MT-014) + `set_wikilink_context` (MT-057) — REUSE, not re-implement.
    pub fn wire_if_needed(&self) {
        self.wire_state_if_needed(&self.documents.base_state(), &self.base_document_id);
    }

    fn wire_state_if_needed(&self, state: &Arc<Mutex<RichEditorState>>, document_id: &str) {
        use std::sync::atomic::Ordering;
        let ctx = self.session.lock().map(|c| c.clone()).unwrap_or_default();
        if !ctx.is_bound() {
            return;
        }
        let Some(runtime) = ctx.runtime else { return };
        let identity = (ctx.workspace_id.clone(), runtime.id());
        let state_key = if document_id.trim().is_empty() {
            String::new()
        } else {
            document_id.to_owned()
        };
        let already_applied = self
            .applied_sessions
            .lock()
            .map(|applied| applied.get(&state_key) == Some(&identity))
            .unwrap_or(false);
        if already_applied {
            return;
        }
        if let Ok(mut s) = state.lock() {
            s.set_embed_context(ctx.workspace_id.clone(), runtime.clone());
            s.set_wikilink_context(ctx.workspace_id, document_id.to_owned(), runtime);
        } else {
            return;
        }
        self.applied_sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(state_key, identity);
        self.wired.store(true, Ordering::Relaxed);
    }

    /// Drain the editor's `pending_events` into the shared outbound queue (AC-079-5). Called AFTER the
    /// inner render so a click handled THIS frame is routed THIS frame. Pushing them to the shared queue
    /// (rather than routing here) keeps the editor a pure widget and the routing the shell's job.
    fn drain_events(&self, state: &Arc<Mutex<RichEditorState>>) {
        let drained = match state.lock() {
            Ok(mut s) => std::mem::take(&mut s.pending_events),
            Err(p) => std::mem::take(&mut p.into_inner().pending_events),
        };
        self.events.push_all(drained);
    }
}

impl PaneFactory for RichEditorPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomWikiPage
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        // Thread session context BEFORE the inner render, so the first live frame already has the
        // embed + wikilink context wired (AC-079-2).
        let content_id = ctx
            .record
            .content_id
            .as_deref()
            .filter(|id| !id.trim().is_empty());
        let pane_id = ctx.record.pane_id.as_ref();
        let state = self.documents.state_for_view(content_id, pane_id);
        if let Some(document_id) = content_id {
            self.documents
                .synchronize_view_from_canonical(document_id, pane_id);
        }
        let _accessibility_namespace = {
            let state = state.lock().unwrap_or_else(|e| e.into_inner());
            crate::rich_editor::push_accessibility_view_namespace(
                state.accessibility_namespace.clone(),
            )
        };
        let document_id = content_id.unwrap_or(&self.base_document_id);
        self.wire_state_if_needed(&state, document_id);
        // MT-042: the shell has already opened the exact rich-document identity as soon as the tab is
        // inserted. Publish that authoritative target immediately instead of waiting for the async body
        // load to construct SaveManager. The widget reuses this same node id once loading completes.
        if let Some(document_id) = ctx
            .record
            .content_id
            .as_deref()
            .filter(|document_id| !document_id.trim().is_empty())
        {
            let document_id = document_id.to_owned();
            let author_id =
                crate::rich_editor::scoped_author_id(format!("rich-editor.document.{document_id}"));
            ui.ctx().accesskit_node_builder(
                egui::Id::new(("rich-editor-opened-document", &document_id)),
                move |node| {
                    node.set_role(accesskit::Role::Document);
                    node.set_author_id(author_id);
                    node.set_label("Opened rich document");
                    node.set_value(document_id);
                },
            );
        }
        if let Some(document_id) = content_id {
            if !self.documents.is_view_ready(document_id, pane_id) {
                let loading = ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!("Loading Notes document {document_id}…"));
                });
                let author_id = crate::rich_editor::scoped_author_id(format!(
                    "rich-editor.loading.{document_id}"
                ));
                ui.ctx()
                    .accesskit_node_builder(loading.response.id, |node| {
                        node.set_author_id(author_id);
                        node.set_role(accesskit::Role::Status);
                    });
                // Do not invoke the editable renderer before the exact workspace/document/view GET
                // has installed. This makes the initial blank state an internal placeholder only.
                // The state is still the live mounted view, however: a retry or document switch can
                // leave events queued on it before the next GET completes. Drain those events even
                // across the loading gate so stale link/tag activations are routed instead of being
                // stranded until the view becomes ready (MT-079 remediation).
                self.drain_events(&state);
                return;
            }
        }
        // WP-KERNEL-012 MT-055 REMEDIATION (reading mode reachable in the mounted editor): render the
        // Edit|Reading segmented toggle in the mounted editor CHROME (above the editor body), persist the
        // choice per document in the egui-persisted `ReadingModeStore`, and pass `store.get(document_id)`
        // into the widget's read-only flag. The store key is the open document's content id (per-document
        // isolation — RISK-004/MC-004); a fresh Notes pane with no document yet keys on its stable pane id
        // so the toggle is still operable there without leaking state onto a later real document.
        let doc_key = ctx
            .record
            .content_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(|id| id.to_owned())
            .unwrap_or_else(|| format!("pane:{}", ctx.record.pane_id));
        let mut store = crate::rich_editor::reading_mode::reading_mode_store(ui.ctx());
        // WP-KERNEL-012 MT-035 wave-7: seed a FRESHLY-opened document (one with no remembered per-document
        // choice) from the operator's `editor_prefs.reading_mode_default` preference (threaded onto the
        // shared rich state via `RichEditorState::set_reading_mode_default`). A document the operator has
        // already toggled keeps its remembered choice — the default never overrides a per-document toggle.
        if !store.contains(&doc_key) {
            let default_reading = {
                let s = state.lock().unwrap_or_else(|e| e.into_inner());
                s.reading_mode_default()
            };
            store.ensure_seeded(&doc_key, default_reading);
        }
        let mode = crate::rich_editor::reading_mode::view_mode_toggle(ui, &doc_key, &mut store);
        crate::rich_editor::reading_mode::write_reading_mode_store(ui.ctx(), &store);
        if mode.is_read_only() {
            // Reading view: render the SAME shared state through the widget's read-only branch (MT-055's
            // `with_read_only` path — no second renderer). The editable inner-factory render is skipped
            // this frame: reading mode applies no edit dispatch, so the per-frame bus command
            // registration/selection publish (an editable-surface concern) honestly pauses with it.
            crate::rich_editor::renderer::rich_editor_widget::RichEditorWidget::new(Arc::clone(
                &state,
            ))
            .with_read_only(true)
            .show(ui);
        } else {
            // Delegate to the EXISTING rich factory render: it installs the unified-undo pane id,
            // publishes selection to the shared bus, registers the rich command set, and runs the editor
            // widget — the real per-frame consumers MT-031/035 already prove. The mount adds session
            // threading + the pending_events drain; it does not re-implement editor logic.
            RichEditorPaneFactory::new(Arc::clone(&state)).render(ui, ctx);
        }
        // DRAIN + route (AC-079-5): the editor enqueued any WikilinkActivated/BacklinkActivated/
        // TagActivated this frame; move them to the shell's outbound queue so the shell routes them to
        // the nav bus after the pane host. No event is left unrouted (reading mode keeps link chips
        // interactive, so the drain runs in both branches).
        self.drain_events(&state);
        if let Some(document_id) = content_id {
            self.documents
                .publish_view_to_canonical(document_id, pane_id);
            let forward_save = state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take_deferred_shared_save_request();
            if forward_save {
                if let Some(authority) = self.documents.canonical_state_for_document(document_id) {
                    authority
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .request_save_for_host();
                }
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Document
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 MT-080 (E11 host-mount, part 2): the SECONDARY pane factories.
//
// MT-079 mounted the CORE code + rich editors. This MT mounts the rest of the widget-proven panes —
// the canvas board (MT-026), the graph view (MT-021/060), and the side panes (outgoing-links MT-062,
// relevant-memory MT-063, Stage MT-066, daily-journal MT-067, manual MT-073) — over their
// `PlaceholderPaneFactory` entries so they render LIVE in the running shell.
//
// SAME shared-cell pattern as MT-079: each factory holds an `Arc<Mutex<_>>` to the widget state the
// shell also owns, so the shell drives the SAME state the mounted pane shows (the AC-080 proofs need
// the real widget behind the factory) and a `&self` `render` reads the live palette each frame. The
// `PaneFactory` trait signature is UNCHANGED (RISK-080-5 / MC-080-3). No widget logic is
// re-implemented — every factory CALLS the existing widget's `show`.
//
// HONESTY (MC-080-5 / Spec-Realism Gate): every factory below is CONSUMED by the live render loop
// (registered in `app.rs` over a placeholder and rendered each frame). Where a backend route is absent
// (FEMS/Stage/Calendar/Locus), the wrapped widget shows its own honest empty-state — no factory fakes a
// live wiring, and none uses `todo!()`/`unimplemented!()` on a live path.
// ════════════════════════════════════════════════════════════════════════════════════════════════

use crate::theme::HsPalette;

/// The live theme palette the shell pushes into the secondary pane factories each frame (the widgets
/// read theme tokens, never hardcoded hex — CONTROL-4). One shared cell shared by every secondary
/// factory; the shell overwrites it from the active theme each frame, exactly like the MT-079 session
/// cell. Starts at the dark palette so a headless/test render (which may not push a palette) still has
/// real tokens.
pub type SharedPalette = Arc<Mutex<HsPalette>>;

/// Read the current palette out of the shared cell (a clone, so the lock is released before render).
fn palette_of(cell: &SharedPalette) -> HsPalette {
    cell.lock()
        .map(|p| p.clone())
        .unwrap_or_else(|p| p.into_inner().clone())
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-026): the live CANVAS-board pane factory. Registered over
/// `PaneType::AtelierEditor` (the canvas/atelier surface the shell already routes a canvas-id open to).
/// Wraps the existing [`crate::graph::canvas_board::LoomCanvasBoard`] widget and renders it each frame;
/// any [`crate::graph::canvas_board::CanvasEvent`] the board dispatches this frame is pushed into a shared
/// outbound queue the shell drains + maps to the real canvas PATCH/POST (AC-080-2). The board state is the
/// SAME `Arc<Mutex<_>>` the shell holds, so the shell's getCanvasBoard refresh feeds back into the pane.
pub struct CanvasBoardPaneMount {
    board: Arc<Mutex<crate::graph::canvas_board::LoomCanvasBoard>>,
    palette: SharedPalette,
    /// The outbound queue of canvas events the shell drains each frame. W3 (MT-026 remediation): the
    /// host (`route_canvas_events`) now maps EVERY mutation kind — place/card/resize/move-section/
    /// group/remove-placement/semantic-edge/visual-edge/remove-edge/viewport — to its verified
    /// `CanvasBoardClient` request + tracked op cell (re-fetch reconcile on Ok, rollback on Err);
    /// `NodeMenu` routes to the MT-070 nav bus and `TextCardEditBlocked` stays the honest typed blocker.
    events: Arc<Mutex<Vec<crate::graph::canvas_board::CanvasEvent>>>,
}

impl CanvasBoardPaneMount {
    pub fn new(
        board: Arc<Mutex<crate::graph::canvas_board::LoomCanvasBoard>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::canvas_board::CanvasEvent>>>,
    ) -> Self {
        Self {
            board,
            palette,
            events,
        }
    }
}

impl PaneFactory for CanvasBoardPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::AtelierEditor
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        // Render the REAL board (the toolbar + placements + AccessKit `canvas.*` subtree). The widget owns
        // its own per-frame consumers; the mount only collects the dispatched events for the shell.
        let mut event = None;
        if let Ok(mut board) = self.board.lock() {
            board.set_render_source_pane_id(ctx.record.pane_id.clone());
            event = board.show(ui, &palette);
            // Also drain any swarm-dispatched knowledge events the single `show` return cannot carry
            // (the MT-042 anti-scaffolding drain) so a canvas dispatch reaches the shell too.
            let drained = board.drain_knowledge_events();
            if !drained.is_empty() {
                if let Ok(mut q) = self.events.lock() {
                    q.extend(drained);
                }
            }
        }
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Group
    }
}

/// WP-KERNEL-012 MT-080 REMEDIATION (PaneType collision fix): the live GRAPH-view pane factory. Now
/// registered over its OWN key `PaneType::Placeholder("Graph View")` — NOT `PaneType::KernelDcc`. The old
/// KernelDcc registration hijacked the quick-switcher WP/MT navigation (open_work_packet / open_micro_task
/// open `KernelDcc` tabs with `WP:`/`MT:` content ids) by rendering the generic graph view with the WP/MT
/// id ignored. KernelDcc now falls back to the honest content-aware placeholder (which SHOWS the WP/MT id)
/// and the graph view opens via its own operator route (`view.graph` palette command / VIEW menu). Wraps
/// the existing [`crate::graph::graph_view::LoomGraphView`] and renders it each frame; any
/// [`crate::graph::graph_view::GraphEvent`] (notably `DepthChanged`) is pushed into a shared outbound
/// queue the shell drains to re-query the depth-parameterized graph-search (AC-080-3).
pub struct GraphViewPaneMount {
    view: Arc<Mutex<crate::graph::graph_view::LoomGraphView>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::graph_view::GraphEvent>>>,
}

impl GraphViewPaneMount {
    pub fn new(
        view: Arc<Mutex<crate::graph::graph_view::LoomGraphView>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::graph_view::GraphEvent>>>,
    ) -> Self {
        Self {
            view,
            palette,
            events,
        }
    }
}

impl PaneFactory for GraphViewPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(GRAPH_VIEW_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let mut event = None;
        if let Ok(mut view) = self.view.lock() {
            view.set_render_source_pane_id(ctx.record.pane_id.clone());
            event = view.show(ui, &palette);
            let drained = view.drain_knowledge_events();
            if !drained.is_empty() {
                if let Ok(mut q) = self.events.lock() {
                    q.extend(drained);
                }
            }
        }
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

/// WP-KERNEL-012 MT-080 REMEDIATION (PaneType collision fix): the live OUTGOING-LINKS side pane. Now
/// registered over its OWN key `PaneType::Placeholder("Outgoing Links")` — NOT `PaneType::LoomBlock`. The
/// old LoomBlock registration made EVERY loom-block open (quick-switcher hit, wikilink chip, search result)
/// render the same content-blind OutgoingLinksPanel instead of block-appropriate content, hijacking
/// navigation. LoomBlock now falls back to the honest content-aware placeholder (which SHOWS the block id)
/// and the outgoing-links pane opens via its own operator route. Wraps the existing
/// [`crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel`]; an `on_open(NavTarget)`
/// click is pushed into a shared outbound queue the shell routes to the MT-030 nav bus.
pub struct OutgoingLinksPaneMount {
    panel: Arc<Mutex<crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel>>,
    palette: SharedPalette,
    nav: Arc<Mutex<Vec<crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget>>>,
}

impl OutgoingLinksPaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::rich_editor::wikilinks::outgoing_links_panel::OutgoingLinksPanel>>,
        palette: SharedPalette,
        nav: Arc<Mutex<Vec<crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget>>>,
    ) -> Self {
        Self {
            panel,
            palette,
            nav,
        }
    }
}

impl PaneFactory for OutgoingLinksPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(OUTGOING_LINKS_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let nav = Arc::clone(&self.nav);
        if let Ok(mut panel) = self.panel.lock() {
            let mut on_open =
                |target: crate::rich_editor::wikilinks::outgoing_links_panel::NavTarget| {
                    if let Ok(mut q) = nav.lock() {
                        q.push(target);
                    }
                };
            panel.show(ui, &palette, &mut on_open);
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-063): the live RELEVANT-MEMORY side pane. Registered over
/// `PaneType::Placeholder("Relevant Memory")` (the distinct placeholder key the pane registers under).
/// Wraps the existing [`crate::fems::relevant_memory_panel::RelevantMemoryPanel`]; a "Go to source" click
/// routes through the shared nav queue. The FEMS read route EXISTS (WP-009 MT-109 shipped
/// `GET /workspaces/{ws}/memory/pack`); the live round-trip is NEEDS_MANAGED_RESOURCE_PROOF. When no
/// backend is reachable the panel renders its own typed-blocker empty-state (`EndpointMissing`/Transport)
/// — the mount never fakes a pack.
pub struct RelevantMemoryPaneMount {
    panel: Arc<Mutex<crate::fems::relevant_memory_panel::RelevantMemoryPanel>>,
    palette: SharedPalette,
    nav: Arc<Mutex<Vec<crate::fems::relevant_memory_panel::MemoryNavTarget>>>,
}

impl RelevantMemoryPaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::fems::relevant_memory_panel::RelevantMemoryPanel>>,
        palette: SharedPalette,
        nav: Arc<Mutex<Vec<crate::fems::relevant_memory_panel::MemoryNavTarget>>>,
    ) -> Self {
        Self {
            panel,
            palette,
            nav,
        }
    }
}

impl PaneFactory for RelevantMemoryPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder("Relevant Memory".to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        use crate::fems::relevant_memory_panel::FnNavigationBus;
        let palette = palette_of(&self.palette);
        let nav = Arc::clone(&self.nav);
        if let Ok(mut panel) = self.panel.lock() {
            let mut bus = FnNavigationBus(|target| {
                if let Ok(mut q) = nav.lock() {
                    q.push(target);
                }
            });
            panel.show(ui, &palette, &mut bus);
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-066): the live STAGE pane. Registered over
/// `PaneType::Placeholder("Stage")`. Wraps the existing [`crate::stage_pane::StagePane`] full round-trip
/// surface; the embed-back action is signalled through a shared flag the shell drains. The Stage embed-back
/// HTTP route is ABSENT, so the embed action surfaces the honest typed blocker — never a faked embed.
pub struct StagePaneMount {
    pane: Arc<Mutex<crate::stage_pane::StagePane>>,
    palette: SharedPalette,
    /// Set true on the frame the operator/agent pressed "Embed back into note" so the shell can surface
    /// the typed blocker / route it once.
    embed_requested: Arc<std::sync::atomic::AtomicBool>,
}

impl StagePaneMount {
    pub fn new(
        pane: Arc<Mutex<crate::stage_pane::StagePane>>,
        palette: SharedPalette,
        embed_requested: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            pane,
            palette,
            embed_requested,
        }
    }
}

impl PaneFactory for StagePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder("Stage".to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        if let Ok(mut pane) = self.pane.lock() {
            let embed = pane.show_round_trip(ui, &palette);
            if embed {
                self.embed_requested
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-067): the live DAILY-JOURNAL pane. Registered over
/// `PaneType::LoomDailyJournal`. Wraps the existing [`crate::graph::daily_journal_panel::DailyJournalPanel`]
/// (stateless `show`) over a shared [`crate::graph::daily_journal_panel::DailyJournalState`]; a date-nav
/// signal is pushed into a shared outbound queue the shell maps to `open_or_create_daily_note` (AC-080-5).
pub struct DailyJournalPaneMount {
    state: Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::daily_journal_panel::DailyJournalEvent>>>,
    /// WP-KERNEL-012 MT-019 REMEDIATION (journal EDITING surface folded into the LoomDailyJournal pane
    /// host): the shared MT-019 journal panel state (`JournalStore` + embedded rich editor + 3s
    /// auto-save). `None` until the shell binds it on the first frame with a live runtime + workspace
    /// (the store's production backend spawns off-thread loads/saves) — the honest unbound state renders
    /// a disclosure line, never a fake editor.
    journal: SharedJournalPanel,
}

/// The one-slot shared cell holding the BOUND MT-019 journal panel state (`None` until the shell binds
/// the production store). The inner `Arc<Mutex<JournalPanelState>>` is the exact handle
/// [`crate::rich_editor::daily_notes::journal_panel::JournalPanelWidget`] renders through, so the mount
/// and any test drive the SAME state across frames.
pub type SharedJournalPanel = Arc<
    Mutex<Option<Arc<Mutex<crate::rich_editor::daily_notes::journal_panel::JournalPanelState>>>>,
>;

impl DailyJournalPaneMount {
    pub fn new(
        state: Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::daily_journal_panel::DailyJournalEvent>>>,
        journal: SharedJournalPanel,
    ) -> Self {
        Self {
            state,
            palette,
            events,
            journal,
        }
    }
}

impl PaneFactory for DailyJournalPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::LoomDailyJournal
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        use crate::graph::daily_journal_panel::{DailyJournalEvent, DailyJournalPanel};
        let palette = palette_of(&self.palette);
        if let Ok(mut state) = self.state.lock() {
            let event = DailyJournalPanel::show(ui, &mut state, &palette);
            if !matches!(event, DailyJournalEvent::None) {
                if let Ok(mut q) = self.events.lock() {
                    q.push(event);
                }
            }
        }
        // MT-019: the journal EDITING surface (open/create today's note + embedded editor + auto-save),
        // folded below the MT-067 calendar-interop header. The JournalPanelWidget drives its own store
        // drain / edit-detection / auto-save each frame. Rendered only once the shell bound the
        // production store (runtime + workspace available); until then an honest disclosure renders.
        let journal = self
            .journal
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().map(Arc::clone));
        match journal {
            Some(journal_state) => {
                crate::rich_editor::daily_notes::journal_panel::JournalPanelWidget::new(
                    journal_state,
                )
                .show(ui);
            }
            None => {
                ui.label(
                    egui::RichText::new(
                        "Journal editor binds when a workspace and runtime are active.",
                    )
                    .color(palette.text_subtle),
                );
            }
        }
    }
}

/// MT-067: content-addressed CalendarEvent destination mounted over the same resolved journal state.
pub struct CalendarEventPaneMount {
    state: Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::daily_journal_panel::DailyJournalEvent>>>,
    tabs: Mutex<BTreeMap<String, crate::graph::daily_journal_panel::CalendarEventDetailTab>>,
    snapshots: Mutex<BTreeMap<String, crate::graph::daily_journal_panel::DailyJournalState>>,
}

impl CalendarEventPaneMount {
    pub fn new(
        state: Arc<Mutex<crate::graph::daily_journal_panel::DailyJournalState>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::daily_journal_panel::DailyJournalEvent>>>,
    ) -> Self {
        Self {
            state,
            palette,
            events,
            tabs: Mutex::new(BTreeMap::new()),
            snapshots: Mutex::new(BTreeMap::new()),
        }
    }
}

impl PaneFactory for CalendarEventPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::CalendarEvent
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        use crate::graph::daily_journal_panel::{
            CalendarEventDetailPanel, CalendarEventDetailTab, DailyJournalEvent,
        };
        let event_id = ctx.record.content_id.as_deref().unwrap_or_default();
        let palette = palette_of(&self.palette);
        let Ok(current_state) = self.state.lock().map(|state| state.clone()) else {
            return;
        };
        let Ok(mut snapshots) = self.snapshots.lock() else {
            return;
        };
        if current_state
            .event
            .as_ref()
            .is_some_and(|event| event.id == event_id)
        {
            snapshots.insert(event_id.to_owned(), current_state.clone());
        }
        let state = snapshots.get(event_id).unwrap_or(&current_state);
        let Ok(mut tabs) = self.tabs.lock() else {
            return;
        };
        let active_tab = tabs
            .entry(event_id.to_owned())
            .or_insert(CalendarEventDetailTab::Details);
        let event = CalendarEventDetailPanel::show(ui, event_id, state, active_tab, &palette);
        if !matches!(event, DailyJournalEvent::None) {
            if let Ok(mut events) = self.events.lock() {
                events.push(event);
            }
        }
    }
}

/// WP-KERNEL-012 MT-080 (GROUP A / MT-073): the live USER-MANUAL pane. Registered over
/// `PaneType::UserManual`. Wraps the existing [`crate::manual_pane::ManualPane`] over a shared
/// [`crate::manual_pane::ManualRegistry`] (immutable content) + [`crate::manual_pane::ManualPaneState`]
/// (search/selection). Pure in-pane widget (no backend) — it always renders its real `manual-pane` subtree.
pub struct ManualPaneMount {
    registry: Arc<crate::manual_pane::ManualRegistry>,
    state: Arc<Mutex<crate::manual_pane::ManualPaneState>>,
    palette: SharedPalette,
}

impl ManualPaneMount {
    pub fn new(
        registry: Arc<crate::manual_pane::ManualRegistry>,
        state: Arc<Mutex<crate::manual_pane::ManualPaneState>>,
        palette: SharedPalette,
    ) -> Self {
        Self {
            registry,
            state,
            palette,
        }
    }
}

impl PaneFactory for ManualPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::UserManual
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        if let Ok(mut state) = self.state.lock() {
            crate::manual_pane::ManualPane::new(&self.registry, &mut state, &palette).show(ui);
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Region
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// WP-KERNEL-012 E11 remediation wave (lane W1 — shell host wiring): the ORPHAN-WIDGET mounts.
//
// The 2026-07-02 per-MT drift audit found a class of widget-proven surfaces with NO host mount and NO
// operator open route (MT-022 folder tree, MT-023 tags, MT-024 sidebar/pins, MT-025/059 wiki page,
// MT-027 block collections, MT-056 outline, MT-036 flight recorder, MT-009 diff/merge). Each mount below
// follows the exact MT-079/080 shared-cell pattern: the shell owns the SAME `Arc<Mutex<_>>` state the
// registered factory renders, plus a shared outbound event queue the shell drains + routes each frame.
// Every mount is keyed on its OWN `PaneType::Placeholder(<label>)` key (the established side-pane keying:
// Relevant Memory / Stage) so no mount collides with a content-addressed navigation PaneType.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Stable pane-key labels for the Placeholder-keyed side panes (single source of truth shared by the
/// factory registrations, the `view.*` open commands in `app.rs`, and the route kittests).
pub const OUTGOING_LINKS_PANE_LABEL: &str = "Outgoing Links";
pub const GRAPH_VIEW_PANE_LABEL: &str = "Graph View";
pub const TAGS_PANE_LABEL: &str = "Tags";
pub const SIDEBAR_PANE_LABEL: &str = "Sidebar";
pub const BLOCK_COLLECTIONS_PANE_LABEL: &str = "Block Collections";
pub const OUTLINE_PANE_LABEL: &str = "Outline";
pub const WIKI_PAGE_PANE_LABEL: &str = "Wiki Page";
pub const FOLDER_TREE_PANE_LABEL: &str = "Folders";
pub const DIFF_MERGE_PANE_LABEL: &str = "Diff Merge";
pub const RELEVANT_MEMORY_PANE_LABEL: &str = "Relevant Memory";
pub const STAGE_PANE_LABEL: &str = "Stage";

/// The `PaneType` key for a Placeholder-keyed side pane label (convenience shared by app + tests).
pub fn placeholder_pane_type(label: &str) -> PaneType {
    PaneType::Placeholder(label.to_owned())
}

// ── MT-023: Tags panel + Tag Hub ─────────────────────────────────────────────────────────────────────

/// One drained tags-pane event: either a list-panel event or a hub-page event (the two MT-023 widgets
/// share one mounted pane; the hub opens over the list when the host consumes `OpenTag`).
#[derive(Debug, Clone)]
pub enum TagsPaneEvent {
    Panel(crate::graph::tags_panel::TagsPanelEvent),
    Hub(crate::graph::tags_panel::TagHubEvent),
    /// The operator pressed the "All tags" back affordance while a hub was open: the host clears the
    /// bound hub (pure UI state — no backend call).
    BackToList,
}

/// WP-KERNEL-012 MT-023 REMEDIATION: the live TAGS side pane (list + hub). Registered over
/// `PaneType::Placeholder("Tags")`. Renders the bound [`crate::graph::tags_panel::LoomTagHubPanel`] when
/// one is open, else the [`crate::graph::tags_panel::LoomTagsPanel`] list; every widget event is pushed
/// into the shared outbound queue the shell routes to the MT-023 `LoomTagClient` (fetch/open/tag-edge).
pub struct TagsPaneMount {
    tags: Arc<Mutex<crate::graph::tags_panel::LoomTagsPanel>>,
    hub: Arc<Mutex<Option<crate::graph::tags_panel::LoomTagHubPanel>>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<TagsPaneEvent>>>,
}

impl TagsPaneMount {
    pub fn new(
        tags: Arc<Mutex<crate::graph::tags_panel::LoomTagsPanel>>,
        hub: Arc<Mutex<Option<crate::graph::tags_panel::LoomTagHubPanel>>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<TagsPaneEvent>>>,
    ) -> Self {
        Self {
            tags,
            hub,
            palette,
            events,
        }
    }
}

impl PaneFactory for TagsPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(TAGS_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let mut out: Option<TagsPaneEvent> = None;
        let hub_open = self.hub.lock().map(|h| h.is_some()).unwrap_or(false);
        if hub_open {
            // Back affordance so the operator is never stuck on a hub page (stable AccessKit address).
            let back = ui.button(egui::RichText::new("< All tags").color(palette.text));
            ui.ctx().accesskit_node_builder(back.id, |node| {
                node.set_author_id("tags.back-to-list".to_owned());
            });
            if back.clicked() {
                out = Some(TagsPaneEvent::BackToList);
            }
            if let Ok(mut hub) = self.hub.lock() {
                if let Some(hub) = hub.as_mut() {
                    if let Some(ev) = hub.show(ui, &palette) {
                        out = Some(TagsPaneEvent::Hub(ev));
                    }
                }
            }
        } else if let Ok(mut tags) = self.tags.lock() {
            if let Some(ev) = tags.show(ui, &palette) {
                out = Some(TagsPaneEvent::Panel(ev));
            }
        }
        if let Some(ev) = out {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

// ── MT-024: Sidebar (pins / favorites / backlinks / unlinked / breadcrumbs) ──────────────────────────

/// WP-KERNEL-012 MT-024 REMEDIATION: the live SIDEBAR pane. Registered over
/// `PaneType::Placeholder("Sidebar")`. Wraps the existing
/// [`crate::graph::sidebar_panel::LoomSidebarPanel`]; every [`crate::graph::sidebar_panel::SidebarEvent`]
/// is pushed into the shared outbound queue the shell routes to the `LoomSidebarClient` (two-call pin
/// removal, unfavorite PATCH, section re-fetch) + the `ShellEvent::BookmarkRemoved` emission.
pub struct SidebarPaneMount {
    panel: Arc<Mutex<crate::graph::sidebar_panel::LoomSidebarPanel>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::sidebar_panel::SidebarEvent>>>,
}

impl SidebarPaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::graph::sidebar_panel::LoomSidebarPanel>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::sidebar_panel::SidebarEvent>>>,
    ) -> Self {
        Self {
            panel,
            palette,
            events,
        }
    }
}

impl PaneFactory for SidebarPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(SIDEBAR_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let event = match self.panel.lock() {
            Ok(mut p) => p.show(ui, &palette),
            Err(_) => None,
        };
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

// ── MT-027: Block Collections (table / kanban / calendar) ────────────────────────────────────────────

/// WP-KERNEL-012 MT-027 REMEDIATION: the live BLOCK-COLLECTIONS pane. Registered over
/// `PaneType::Placeholder("Block Collections")`. Wraps the existing
/// [`crate::graph::block_collection_view::BlockCollectionView`]; every
/// [`crate::graph::block_collection_view::BlockViewEvent`] (from the `show` return AND the MT-042 swarm
/// drain) is pushed into the shared outbound queue the shell routes to the `BlockViewClient`.
pub struct BlockCollectionPaneMount {
    view: Arc<Mutex<crate::graph::block_collection_view::BlockCollectionView>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::block_collection_view::BlockViewEvent>>>,
}

impl BlockCollectionPaneMount {
    pub fn new(
        view: Arc<Mutex<crate::graph::block_collection_view::BlockCollectionView>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::block_collection_view::BlockViewEvent>>>,
    ) -> Self {
        Self {
            view,
            palette,
            events,
        }
    }
}

impl PaneFactory for BlockCollectionPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(BLOCK_COLLECTIONS_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let mut drained: Vec<crate::graph::block_collection_view::BlockViewEvent> = Vec::new();
        if let Ok(mut view) = self.view.lock() {
            if let Some(ev) = view.show(ui, &palette) {
                drained.push(ev);
            }
            // MT-042 swarm dispatches the single Option return cannot carry.
            drained.extend(view.drain_knowledge_events());
        }
        if !drained.is_empty() {
            if let Ok(mut q) = self.events.lock() {
                q.extend(drained);
            }
        }
    }
}

// ── MT-056: Outline / table-of-contents side pane ────────────────────────────────────────────────────

/// WP-KERNEL-012 MT-056 REMEDIATION: the live OUTLINE side pane. Registered over
/// `PaneType::Placeholder("Outline")`. Wraps the existing
/// [`crate::rich_editor::outline_panel::OutlinePanel`] over the SAME mounted rich-editor state the Notes
/// pane renders, so heading clicks scroll the REAL mounted document (the panel stages the scroll target
/// on the shared state itself — no outbound queue needed).
pub struct OutlinePaneMount {
    panel: Arc<Mutex<crate::rich_editor::outline_panel::OutlinePanel>>,
    rich_documents: Arc<RichEditorDocumentStore>,
}

impl OutlinePaneMount {
    pub fn new(
        panel: Arc<Mutex<crate::rich_editor::outline_panel::OutlinePanel>>,
        rich_state: Arc<Mutex<RichEditorState>>,
    ) -> Self {
        Self::from_document_store(panel, Arc::new(RichEditorDocumentStore::new(rich_state)))
    }

    pub fn from_document_store(
        panel: Arc<Mutex<crate::rich_editor::outline_panel::OutlinePanel>>,
        rich_documents: Arc<RichEditorDocumentStore>,
    ) -> Self {
        Self {
            panel,
            rich_documents,
        }
    }
}

impl PaneFactory for OutlinePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(OUTLINE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        if let Ok(mut panel) = self.panel.lock() {
            let rich_state = self.rich_documents.active_state();
            // Sync the outline from the live document FIRST (cheap hash-guarded rebuild), then render.
            // The sync borrow is dropped before `show` re-locks the state for the click path.
            {
                let state = match rich_state.lock() {
                    Ok(s) => Some(s),
                    Err(_) => None,
                };
                if let Some(state) = state {
                    panel.sync(&state);
                }
            }
            panel.show(ui, &rich_state);
        }
    }
}

// ── MT-025/059: Loom wiki-projection page pane ───────────────────────────────────────────────────────

/// A host request drained from the wiki pane: the shell maps each to the verified `LoomWikiClient`
/// routes (GET load / POST overlays / POST regenerate) and re-delivers into the bound panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WikiPaneRequest {
    /// A (re)load is needed for `projection_id` (first bind, Retry, or after a save/regenerate).
    Load {
        identity: crate::backend_client::WikiPaneIdentity,
    },
    /// The overlay is already committed; retry only the failed follow-up projection/overlay GET.
    ReloadAfterSave {
        identity: crate::backend_client::WikiPaneIdentity,
    },
    /// The Save button was pressed with `annotation` (the verified overlay-annotation write).
    Save {
        identity: crate::backend_client::WikiPaneIdentity,
        annotation: String,
    },
    /// The Rebuild button was pressed (`POST /loom/wiki/{id}/regenerate`).
    Regenerate {
        identity: crate::backend_client::WikiPaneIdentity,
    },
}

/// WP-KERNEL-012 MT-025/059 REMEDIATION: the live WIKI-PAGE pane. Registered over its OWN key
/// `PaneType::Placeholder("Wiki Page")` — `open_wiki_page` now routes wiki projection ids HERE instead of
/// feeding them into the rich-document loader (the audited nav misroute). The mount binds one
/// [`crate::graph::wiki_page_panel::LoomWikiPagePanel`] per open projection id (rebinding when the tab's
/// `content_id` changes) and pushes load/save/regenerate requests into the shared outbound queue.
pub struct WikiPagePaneMount {
    /// The bound panel + its projection id (`None` until a wiki tab with a content id renders).
    bound: Arc<
        Mutex<
            Option<(
                crate::backend_client::WikiPaneIdentity,
                crate::graph::wiki_page_panel::LoomWikiPagePanel,
            )>,
        >,
    >,
    session: SharedSessionContext,
    palette: SharedPalette,
    requests: Arc<Mutex<Vec<WikiPaneRequest>>>,
    pane_generation: Arc<std::sync::atomic::AtomicU64>,
}

impl WikiPagePaneMount {
    pub fn new(
        bound: Arc<
            Mutex<
                Option<(
                    crate::backend_client::WikiPaneIdentity,
                    crate::graph::wiki_page_panel::LoomWikiPagePanel,
                )>,
            >,
        >,
        session: SharedSessionContext,
        palette: SharedPalette,
        requests: Arc<Mutex<Vec<WikiPaneRequest>>>,
        pane_generation: Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        Self {
            bound,
            session,
            palette,
            requests,
            pane_generation,
        }
    }

    fn push_request(&self, req: WikiPaneRequest) {
        if let Ok(mut q) = self.requests.lock() {
            q.push(req);
        }
    }
}

impl PaneFactory for WikiPagePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(WIKI_PAGE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let projection_id = ctx
            .record
            .content_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
            .map(|id| id.to_owned());
        let Some(projection_id) = projection_id else {
            // Honest empty state: a wiki pane with no projection bound (no fake page).
            let empty = ui.label(
                egui::RichText::new("No wiki page open. Open one via the quick switcher.")
                    .color(palette.text_subtle),
            );
            ui.ctx().accesskit_node_builder(empty.id, |node| {
                node.set_author_id("wiki-page.empty".to_owned());
            });
            return;
        };
        let workspace_id = self
            .session
            .lock()
            .map(|s| s.workspace_id.clone())
            .unwrap_or_default();
        if let Ok(mut bound) = self.bound.lock() {
            let needs_rebind = bound
                .as_ref()
                .map(|(identity, _)| {
                    identity.projection_id != projection_id || identity.workspace_id != workspace_id
                })
                .unwrap_or(true);
            if needs_rebind {
                let pane_generation = self
                    .pane_generation
                    .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
                    .wrapping_add(1);
                let identity = crate::backend_client::WikiPaneIdentity {
                    workspace_id: workspace_id.clone(),
                    projection_id: projection_id.clone(),
                    pane_generation,
                };
                *bound = Some((
                    identity.clone(),
                    crate::graph::wiki_page_panel::LoomWikiPagePanel::new(
                        workspace_id,
                        projection_id.clone(),
                    ),
                ));
                // First bind: request the real GET load (the shell fires the LoomWikiClient fetch).
                self.push_request(WikiPaneRequest::Load { identity });
            }
            if let Some((identity, panel)) = bound.as_mut() {
                use crate::graph::wiki_page_panel::WikiPageEvent;
                if let Some(event) = panel.show(ui, &palette) {
                    match event {
                        WikiPageEvent::Save { annotation } => {
                            self.push_request(WikiPaneRequest::Save {
                                identity: identity.clone(),
                                annotation,
                            });
                        }
                        WikiPageEvent::Rebuild => {
                            self.push_request(WikiPaneRequest::Regenerate {
                                identity: identity.clone(),
                            });
                        }
                        WikiPageEvent::Retry => {
                            self.push_request(WikiPaneRequest::Load {
                                identity: identity.clone(),
                            });
                        }
                        WikiPageEvent::RetryReloadAfterSave => {
                            self.push_request(WikiPaneRequest::ReloadAfterSave {
                                identity: identity.clone(),
                            });
                        }
                        // Edit/Cancel are local panel state (observability-only events).
                        WikiPageEvent::EditBegan | WikiPageEvent::Cancel => {}
                    }
                }
            }
        }
    }
}

// ── MT-022: Loom folder tree pane ────────────────────────────────────────────────────────────────────

/// WP-KERNEL-012 MT-022 REMEDIATION: the live FOLDER-TREE pane. Registered over
/// `PaneType::Placeholder("Folders")`. Wraps the existing [`crate::graph::folder_tree::LoomFolderTree`];
/// every [`crate::graph::folder_tree::FolderTreeEvent`] (lazy-fetch expand, recolor, open, retry) is
/// pushed into the shared outbound queue the shell routes to the `LoomFolderClient`.
pub struct FolderTreePaneMount {
    tree: Arc<Mutex<crate::graph::folder_tree::LoomFolderTree>>,
    palette: SharedPalette,
    events: Arc<Mutex<Vec<crate::graph::folder_tree::FolderTreeEvent>>>,
}

impl FolderTreePaneMount {
    pub fn new(
        tree: Arc<Mutex<crate::graph::folder_tree::LoomFolderTree>>,
        palette: SharedPalette,
        events: Arc<Mutex<Vec<crate::graph::folder_tree::FolderTreeEvent>>>,
    ) -> Self {
        Self {
            tree,
            palette,
            events,
        }
    }
}

impl PaneFactory for FolderTreePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(FOLDER_TREE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let event = match self.tree.lock() {
            Ok(mut t) => t.show(ui, &palette),
            Err(_) => None,
        };
        if let Some(ev) = event {
            if let Ok(mut q) = self.events.lock() {
                q.push(ev);
            }
        }
    }
}

// ── MT-009: Diff / merge editor pane (own key — no CodeSymbol collision) ─────────────────────────────

/// WP-KERNEL-012 MT-009 REMEDIATION: the live DIFF/MERGE pane, registered over its OWN key
/// `PaneType::Placeholder("Diff Merge")`. The widget's own `DiffEditorPaneFactory::pane_type()` uses
/// the same placeholder key, so direct registration and the shell mount both avoid replacing the mounted
/// code editor. The slot holds the currently-open
/// [`crate::code_editor::DiffEditorPanel`] (set by the shell's open-diff/open-merge routes); an empty
/// slot renders an honest empty state, never a fake diff.
pub struct DiffMergePaneMount {
    slot: Arc<Mutex<Option<Arc<crate::code_editor::DiffEditorPanel>>>>,
    palette: SharedPalette,
}

impl DiffMergePaneMount {
    pub fn new(
        slot: Arc<Mutex<Option<Arc<crate::code_editor::DiffEditorPanel>>>>,
        palette: SharedPalette,
    ) -> Self {
        Self { slot, palette }
    }
}

impl PaneFactory for DiffMergePaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::Placeholder(DIFF_MERGE_PANE_LABEL.to_owned())
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        let palette = palette_of(&self.palette);
        let panel = self.slot.lock().ok().and_then(|s| s.clone());
        match panel {
            Some(panel) => {
                panel.show(ui);
            }
            None => {
                let resp = ui.label(
                    egui::RichText::new(
                        "No diff or merge open. Open one from a conflict dialog or the palette.",
                    )
                    .color(palette.text_subtle),
                );
                ui.ctx().accesskit_node_builder(resp.id, |node| {
                    node.set_author_id("diff-merge-empty".to_owned());
                });
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::GenericContainer
    }
}

// ── MT-036: Flight Recorder observability pane ───────────────────────────────────────────────────────

/// The one-slot delivery cell a spawned `GET /flight_recorder` fetch resolves into; doubles as the
/// pane's [`crate::flight_recorder_pane::FlightRecorderQuery`] impl (the pane's `load_now` reads the
/// resolved value off the frame thread — never blocking).
#[derive(Debug, Clone)]
struct FlightRecorderFetchDelivery {
    generation: u64,
    workspace_id: String,
    result: Result<crate::flight_recorder_pane::FlightRecorderQueryRows, String>,
}

#[derive(Clone, Default)]
pub struct FlightRecorderFetchCell {
    cell: Arc<Mutex<Option<FlightRecorderFetchDelivery>>>,
    generation: Arc<std::sync::atomic::AtomicU64>,
    workspace_id: Arc<Mutex<String>>,
}

impl FlightRecorderFetchCell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a fetch result (ok or err) has been delivered.
    pub fn is_resolved(&self) -> bool {
        self.cell.lock().map(|c| c.is_some()).unwrap_or(false)
    }

    /// Start a new workspace-scoped request and invalidate every older in-flight completion.
    pub fn begin(&self, workspace_id: impl Into<String>) -> u64 {
        let generation = self
            .generation
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
            .wrapping_add(1)
            .max(1);
        if let Ok(mut current) = self.workspace_id.lock() {
            *current = workspace_id.into();
        }
        self.clear();
        generation
    }

    /// Deliver only if both generation and workspace still match the active request. Returns false for
    /// a stale completion so an A request can never overwrite a newer B request.
    pub fn deliver_if_current(
        &self,
        generation: u64,
        workspace_id: impl Into<String>,
        result: Result<crate::flight_recorder_pane::FlightRecorderQueryRows, String>,
    ) -> bool {
        let workspace_id = workspace_id.into();
        let current_generation = self.generation.load(std::sync::atomic::Ordering::Acquire);
        let current_workspace = self
            .workspace_id
            .lock()
            .map(|workspace| workspace.clone())
            .unwrap_or_default();
        if generation != current_generation || workspace_id != current_workspace {
            return false;
        }
        if let Ok(mut c) = self.cell.lock() {
            *c = Some(FlightRecorderFetchDelivery {
                generation,
                workspace_id,
                result,
            });
            true
        } else {
            false
        }
    }

    pub fn clear(&self) {
        if let Ok(mut c) = self.cell.lock() {
            *c = None;
        }
    }
}

impl crate::flight_recorder_pane::FlightRecorderQuery for FlightRecorderFetchCell {
    fn rows(&self) -> Result<crate::flight_recorder_pane::FlightRecorderQueryRows, String> {
        match self.cell.lock() {
            Ok(c) => match c.as_ref() {
                Some(delivery)
                    if delivery.generation
                        == self.generation.load(std::sync::atomic::Ordering::Acquire)
                        && self
                            .workspace_id
                            .lock()
                            .map(|workspace| workspace.as_str() == delivery.workspace_id.as_str())
                            .unwrap_or(false) =>
                {
                    delivery.result.clone()
                }
                Some(_) => Err("stale flight recorder fetch completion".to_owned()),
                None => Err("flight recorder fetch not resolved yet".to_owned()),
            },
            Err(_) => Err("flight recorder cell poisoned".to_owned()),
        }
    }
}

/// Parse the `GET /flight_recorder` JSON array into native-editor and exact FEMS lifecycle rows.
/// Other Flight Recorder traffic is excluded. Native rows retain their closed action kind; FEMS rows
/// retain both the canonical event type and `FR-EVT-MEM-001..005` code.
pub fn flight_recorder_rows_from_json(
    body: &serde_json::Value,
) -> Result<crate::flight_recorder_pane::FlightRecorderQueryRows, String> {
    let arr = body
        .as_array()
        .ok_or_else(|| "flight recorder response is not a JSON array".to_owned())?;
    let mut rows = Vec::new();
    let mut quarantined = Vec::new();
    const ACTIONS: &[&str] = &[
        "document_saved",
        "code_edit",
        "embed_created",
        "canvas_node_placed",
        "cross_ref_inserted",
        "undo_fired",
        "route_to_stage",
        "memory_write_proposed",
        "stage_embed_back",
        "calendar_event_bound",
        "activity_span_correlated",
        "locus_ref_resolved",
        "locus_reverse_lookup",
    ];
    for e in arr {
        let payload = e.get("payload");
        let recorder_event_type = e.get("event_type").and_then(serde_json::Value::as_str);
        let fems_expected_code = match recorder_event_type {
            Some("memory_write_proposed") => Some("FR-EVT-MEM-001"),
            Some("memory_write_reviewed") => Some("FR-EVT-MEM-002"),
            Some("memory_write_committed") => Some("FR-EVT-MEM-003"),
            Some("memory_pack_built") => Some("FR-EVT-MEM-004"),
            Some("memory_item_status_changed") => Some("FR-EVT-MEM-005"),
            _ => None,
        };
        let payload_event_code = payload
            .and_then(|payload| payload.get("event_code"))
            .and_then(serde_json::Value::as_str);
        let fems_candidate = fems_expected_code.is_some()
            || payload_event_code.is_some_and(|code| code.starts_with("FR-EVT-MEM-"));
        let event_family = payload
            .and_then(|payload| payload.get("event_family"))
            .and_then(serde_json::Value::as_str);
        let actor_hint = e
            .get("actor_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let schema_hint = payload
            .and_then(|payload| payload.get("schema"))
            .and_then(serde_json::Value::as_str);
        let native_candidate = event_family == Some("native_editor")
            || schema_hint == Some(crate::event_emitter::NATIVE_EDITOR_SCHEMA_VERSION)
            || actor_hint == crate::event_emitter::DEFAULT_ACTOR_ID
            || actor_hint.starts_with("hsk:native_editor:");
        if !native_candidate && !fems_candidate {
            continue;
        }
        let parsed = (|| -> Result<crate::flight_recorder_pane::FlightRecorderRow, String> {
            let required = |value: Option<&str>, field: &str| -> Result<String, String> {
                let value = value.unwrap_or_default().trim();
                if value.is_empty() {
                    Err(format!("native editor Flight Recorder row missing {field}"))
                } else {
                    Ok(value.to_owned())
                }
            };
            let event_id = required(e.get("event_id").and_then(|v| v.as_str()), "event_id")?;
            uuid::Uuid::parse_str(&event_id)
                .map_err(|_| "native editor Flight Recorder row has invalid event_id".to_owned())?;
            let actor_id = required(e.get("actor_id").and_then(|v| v.as_str()), "actor_id")?;
            let ts_utc = required(e.get("timestamp").and_then(|v| v.as_str()), "timestamp")?;
            chrono::DateTime::parse_from_rfc3339(&ts_utc)
                .map_err(|_| "Flight Recorder row has invalid timestamp".to_owned())?;

            if fems_candidate {
                let event_type = required(recorder_event_type, "event_type")?;
                let expected_code = fems_expected_code.ok_or_else(|| {
                    "FEMS Flight Recorder row has an unknown event_type".to_owned()
                })?;
                if payload_event_code != Some(expected_code) {
                    return Err("FEMS Flight Recorder row has mismatched event_code".to_owned());
                }
                let map = payload
                    .and_then(serde_json::Value::as_object)
                    .ok_or_else(|| "FEMS Flight Recorder payload is not an object".to_owned())?;
                let required_keys: &[&str] = match expected_code {
                    "FR-EVT-MEM-001" => &[
                        "type",
                        "event_code",
                        "proposal_id",
                        "proposal_hash",
                        "artifact_ref",
                        "scope_refs",
                        "op_count",
                        "requires_review_count",
                    ],
                    "FR-EVT-MEM-002" => &[
                        "type",
                        "event_code",
                        "proposal_id",
                        "decision",
                        "reviewer_kind",
                    ],
                    "FR-EVT-MEM-003" => &[
                        "type",
                        "event_code",
                        "commit_id",
                        "proposal_id",
                        "commit_report_hash",
                        "artifact_ref",
                        "changed_memory_ids_hash",
                    ],
                    "FR-EVT-MEM-004" => &[
                        "type",
                        "event_code",
                        "pack_id",
                        "memory_pack_hash",
                        "artifact_ref",
                        "memory_policy",
                        "scope_refs",
                        "item_count",
                        "token_estimate",
                        "truncation_occurred",
                    ],
                    "FR-EVT-MEM-005" => &[
                        "type",
                        "event_code",
                        "memory_id",
                        "previous_status",
                        "new_status",
                        "reason",
                        "actor",
                    ],
                    _ => unreachable!("closed FEMS event-code match"),
                };
                let optional_keys: &[&str] = match expected_code {
                    "FR-EVT-MEM-002" => &["commit_report_ref"],
                    _ => &[],
                };
                if map.len() < required_keys.len()
                    || map.len() > required_keys.len() + optional_keys.len()
                    || !required_keys.iter().all(|key| map.contains_key(*key))
                    || !map.keys().all(|key| {
                        required_keys.contains(&key.as_str())
                            || optional_keys.contains(&key.as_str())
                    })
                {
                    return Err("FEMS Flight Recorder payload has non-canonical fields".to_owned());
                }
                if map.get("type").and_then(serde_json::Value::as_str) != Some(event_type.as_str())
                {
                    return Err(
                        "FEMS Flight Recorder payload type does not match event_type".to_owned(),
                    );
                }
                let non_empty = |key: &str| {
                    map.get(key)
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|value| !value.trim().is_empty())
                };
                let sha256 = |key: &str| {
                    map.get(key)
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|value| {
                            value.len() == 64
                                && value
                                    .chars()
                                    .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
                        })
                };
                let valid_artifact = |key: &str| {
                    map.get(key)
                        .and_then(serde_json::Value::as_object)
                        .is_some_and(|artifact| {
                            artifact.len() == 2
                                && artifact
                                    .get("artifact_id")
                                    .and_then(serde_json::Value::as_str)
                                    .and_then(|id| uuid::Uuid::parse_str(id).ok())
                                    .is_some_and(|id| !id.is_nil())
                                && artifact
                                    .get("path")
                                    .and_then(serde_json::Value::as_str)
                                    .is_some_and(|path| !path.trim().is_empty())
                        })
                };
                let valid_content_artifact = |key: &str| {
                    map.get(key)
                        .and_then(serde_json::Value::as_str)
                        .and_then(|value| value.strip_prefix("artifact://sha256/"))
                        .is_some_and(|digest| {
                            digest.len() == 64
                                && digest.bytes().all(|byte| {
                                    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                                })
                        })
                };
                let valid_scope_refs = || {
                    map.get("scope_refs")
                        .and_then(serde_json::Value::as_array)
                        .is_some_and(|refs| {
                            refs.iter().all(|value| {
                                value.as_object().is_some_and(|entity| {
                                    entity.len() == 3
                                        && entity
                                            .get("artefact_type")
                                            .and_then(serde_json::Value::as_str)
                                            .is_some_and(|value| !value.trim().is_empty())
                                        && entity
                                            .get("artefact_id")
                                            .and_then(serde_json::Value::as_str)
                                            .and_then(|id| uuid::Uuid::parse_str(id).ok())
                                            .is_some_and(|id| !id.is_nil())
                                        && entity
                                            .get("selector")
                                            .and_then(serde_json::Value::as_str)
                                            .is_some_and(|value| !value.trim().is_empty())
                                })
                            })
                        })
                };
                let valid = match expected_code {
                    "FR-EVT-MEM-001" => {
                        map.get("proposal_id")
                            .and_then(serde_json::Value::as_str)
                            .and_then(|id| uuid::Uuid::parse_str(id).ok())
                            .is_some_and(|id| !id.is_nil())
                            && sha256("proposal_hash")
                            && valid_content_artifact("artifact_ref")
                            && map
                                .get("proposal_hash")
                                .and_then(serde_json::Value::as_str)
                                .zip(map.get("artifact_ref").and_then(serde_json::Value::as_str))
                                .is_some_and(|(hash, artifact_ref)| {
                                    artifact_ref == format!("artifact://sha256/{hash}")
                                })
                            && valid_scope_refs()
                            && map
                                .get("op_count")
                                .and_then(serde_json::Value::as_u64)
                                .is_some()
                            && map
                                .get("requires_review_count")
                                .and_then(serde_json::Value::as_u64)
                                .is_some()
                            && map
                                .get("op_count")
                                .and_then(serde_json::Value::as_u64)
                                .zip(
                                    map.get("requires_review_count")
                                        .and_then(serde_json::Value::as_u64),
                                )
                                .is_some_and(|(op_count, review_count)| review_count <= op_count)
                    }
                    "FR-EVT-MEM-002" => {
                        non_empty("proposal_id")
                            && map
                                .get("decision")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|value| {
                                    matches!(value, "approved" | "rejected" | "partial")
                                })
                            && map
                                .get("reviewer_kind")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|value| matches!(value, "user" | "policy"))
                            && (!map.contains_key("commit_report_ref")
                                || valid_artifact("commit_report_ref"))
                    }
                    "FR-EVT-MEM-003" => {
                        non_empty("commit_id")
                            && non_empty("proposal_id")
                            && sha256("commit_report_hash")
                            && valid_artifact("artifact_ref")
                            && sha256("changed_memory_ids_hash")
                    }
                    "FR-EVT-MEM-004" => {
                        non_empty("pack_id")
                            && sha256("memory_pack_hash")
                            && valid_artifact("artifact_ref")
                            && map
                                .get("memory_policy")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|value| {
                                    matches!(
                                        value,
                                        "EPHEMERAL" | "SESSION_SCOPED" | "WORKSPACE_SCOPED"
                                    )
                                })
                            && valid_scope_refs()
                            && map
                                .get("item_count")
                                .and_then(serde_json::Value::as_u64)
                                .is_some()
                            && map
                                .get("token_estimate")
                                .and_then(serde_json::Value::as_u64)
                                .is_some()
                            && map
                                .get("truncation_occurred")
                                .and_then(serde_json::Value::as_bool)
                                .is_some()
                    }
                    "FR-EVT-MEM-005" => {
                        non_empty("memory_id")
                            && non_empty("previous_status")
                            && non_empty("new_status")
                            && map
                                .get("reason")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|value| {
                                    matches!(
                                        value,
                                        "pin"
                                            | "unpin"
                                            | "invalidate"
                                            | "tombstone"
                                            | "supersede"
                                            | "merge"
                                    )
                                })
                            && map
                                .get("actor")
                                .and_then(serde_json::Value::as_str)
                                .is_some_and(|value| matches!(value, "user" | "job" | "policy"))
                    }
                    _ => false,
                };
                if !valid {
                    return Err("FEMS Flight Recorder payload values are invalid".to_owned());
                }
                if !e
                    .get("wsids")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|wsids| {
                        !wsids.is_empty()
                            && wsids.iter().all(|wsid| {
                                wsid.as_str().is_some_and(|value| !value.trim().is_empty())
                            })
                    })
                {
                    return Err("FEMS Flight Recorder row has no workspace scope".to_owned());
                }
                return Ok(crate::flight_recorder_pane::FlightRecorderRow {
                    event_id,
                    action: event_type,
                    event_code: Some(expected_code.to_owned()),
                    actor_id,
                    ts_utc,
                });
            }
            if event_family != Some("native_editor") {
                return Err("native editor Flight Recorder row has wrong event_family".to_owned());
            }
            if e.get("event_type").and_then(|value| value.as_str()) != Some("system") {
                return Err("native editor Flight Recorder row has wrong event_type".to_owned());
            }
            let schema = required(
                payload
                    .and_then(|payload| payload.get("schema"))
                    .and_then(|value| value.as_str()),
                "payload.schema",
            )?;
            let schema_version = required(
                payload
                    .and_then(|payload| payload.get("schema_version"))
                    .and_then(|value| value.as_str()),
                "payload.schema_version",
            )?;
            if schema != crate::event_emitter::NATIVE_EDITOR_SCHEMA_VERSION
                || schema_version != crate::event_emitter::NATIVE_EDITOR_SCHEMA_VERSION
            {
                return Err("native editor Flight Recorder row has wrong schema".to_owned());
            }
            let action = required(
                payload
                    .and_then(|payload| payload.get("action"))
                    .and_then(|value| value.as_str()),
                "payload.action",
            )?;
            let kind = required(
                payload
                    .and_then(|payload| payload.get("kind"))
                    .and_then(|value| value.as_str()),
                "payload.kind",
            )?;
            if action != kind || !ACTIONS.contains(&action.as_str()) {
                return Err(
                    "native editor Flight Recorder row has unknown/mismatched action".to_owned(),
                );
            }
            required(
                payload
                    .and_then(|payload| payload.get("pane_id"))
                    .and_then(|value| value.as_str()),
                "payload.pane_id",
            )?;
            let workspace_id = required(
                payload
                    .and_then(|payload| payload.get("workspace_id"))
                    .and_then(|value| value.as_str()),
                "payload.workspace_id",
            )?;
            let payload_actor_id = required(
                payload
                    .and_then(|payload| payload.get("actor_id"))
                    .and_then(|value| value.as_str()),
                "payload.actor_id",
            )?;
            if payload_actor_id != actor_id {
                return Err(
                    "native editor Flight Recorder row has mismatched actor identity".to_owned(),
                );
            }
            if !e
                .get("wsids")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|wsids| {
                    wsids
                        .iter()
                        .any(|wsid| wsid.as_str() == Some(workspace_id.as_str()))
                })
            {
                return Err(
                    "native editor Flight Recorder row has mismatched workspace identity"
                        .to_owned(),
                );
            }
            let timestamp = chrono::DateTime::parse_from_rfc3339(&ts_utc).map_err(|_| {
                "native editor Flight Recorder row has invalid timestamp".to_owned()
            })?;
            let payload_ts = required(
                payload
                    .and_then(|payload| payload.get("ts_utc"))
                    .and_then(|value| value.as_str()),
                "payload.ts_utc",
            )?;
            let payload_timestamp =
                chrono::DateTime::parse_from_rfc3339(&payload_ts).map_err(|_| {
                    "native editor Flight Recorder row has invalid payload timestamp".to_owned()
                })?;
            // DuckDB stores the recorder's typed TIMESTAMPTZ column at microsecond precision while
            // the immutable native payload retains the producer's RFC3339 nanoseconds. Match the
            // backend ingestion verifier's exact storage boundary: sub-microsecond spelling loss is
            // the same instant for the recorder column, but the next microsecond is a real mismatch.
            if payload_timestamp.timestamp_micros() != timestamp.timestamp_micros() {
                return Err("native editor Flight Recorder row has mismatched timestamp".to_owned());
            }
            Ok(crate::flight_recorder_pane::FlightRecorderRow {
                event_id,
                action,
                event_code: None,
                actor_id,
                ts_utc,
            })
        })();
        match parsed {
            Ok(row) => rows.push(row),
            Err(error) => {
                let event_id = e
                    .get("event_id")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown-event");
                let diagnostic = format!("{event_id}: {error}");
                tracing::warn!(error = %diagnostic, "quarantined malformed native-editor Flight Recorder row");
                quarantined.push(diagnostic);
            }
        }
    }
    Ok(crate::flight_recorder_pane::FlightRecorderQueryRows { rows, quarantined })
}

/// WP-KERNEL-012 MT-036 REMEDIATION: the live FLIGHT-RECORDER pane, registered over the REAL
/// `PaneType::FlightRecorder` key (the existing `flightrecorder.open` palette command + RUN menu entry
/// already open that key — mounting the real factory makes that operator route render the real pane).
/// The mount signals visibility through `load_requested` so the shell fires ONE `GET /flight_recorder`
/// per open (the production `FlightRecorderQuery` impl over the verified route) and calls the pane's
/// `load_now` when the fetch cell resolves.
pub struct FlightRecorderPaneMount {
    pane: Arc<Mutex<crate::flight_recorder_pane::FlightRecorderPane>>,
    palette: SharedPalette,
    /// Set true on the first render (the pane became visible) so the shell fires the fetch once.
    load_requested: Arc<std::sync::atomic::AtomicBool>,
    initial_load_requested: std::sync::atomic::AtomicBool,
}

impl FlightRecorderPaneMount {
    pub fn new(
        pane: Arc<Mutex<crate::flight_recorder_pane::FlightRecorderPane>>,
        palette: SharedPalette,
        load_requested: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            pane,
            palette,
            load_requested,
            initial_load_requested: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl PaneFactory for FlightRecorderPaneMount {
    fn pane_type(&self) -> PaneType {
        PaneType::FlightRecorder
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        if !self
            .initial_load_requested
            .swap(true, std::sync::atomic::Ordering::Relaxed)
        {
            self.load_requested
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let palette = palette_of(&self.palette);
        if let Ok(pane) = self.pane.lock() {
            pane.show(ui, &palette);
            if pane.take_refresh_requested() {
                self.load_requested
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::Region
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::BlockNode;

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    }

    #[test]
    fn session_context_is_bound_only_with_workspace_and_runtime() {
        let rt = rt();
        assert!(!EditorSessionContext::default().is_bound());
        assert!(!EditorSessionContext {
            workspace_id: "ws".into(),
            runtime: None
        }
        .is_bound());
        assert!(EditorSessionContext::new("ws-1", rt.handle().clone()).is_bound());
        // Empty workspace + a runtime is still UNbound (a half-built context never installs wired state).
        assert!(!EditorSessionContext {
            workspace_id: String::new(),
            runtime: Some(rt.handle().clone())
        }
        .is_bound());
    }

    #[test]
    fn stage_embed_split_caret_publishes_to_stable_authority_across_next_frame() {
        use crate::rich_editor::document_model::node::{Child, HsLinkNode};
        use crate::rich_editor::document_model::position::DocPosition;
        use crate::rich_editor::document_model::selection::Selection;
        use crate::rich_editor::renderer::rich_editor_widget::PendingStageEmbedSave;
        use crate::rich_editor::save::draft_manager::{
            DraftBackend, DraftError, DraftLoadFuture, DraftManager, DraftWriteFuture,
        };
        use crate::rich_editor::save::save_manager::{
            SaveBackend, SaveError, SaveFuture, SaveManager,
        };

        struct NoopSave;
        impl SaveBackend for NoopSave {
            fn save_document(
                &self,
                _document_id: &str,
                _content_json: serde_json::Value,
                _expected_version: u64,
            ) -> SaveFuture {
                Box::pin(async { Err(SaveError::Network("unused".to_owned())) })
            }
        }
        struct NoopDraft;
        impl DraftBackend for NoopDraft {
            fn load_draft(&self, _document_id: &str) -> DraftLoadFuture {
                Box::pin(async { Err(DraftError::Network("unused".to_owned())) })
            }
            fn upsert_draft(
                &self,
                _document_id: &str,
                _base_doc_version: u64,
                _base_content_sha256: String,
                _content_json: serde_json::Value,
            ) -> DraftWriteFuture {
                Box::pin(async { Err(DraftError::Network("unused".to_owned())) })
            }
            fn clear_draft(&self, _document_id: &str) -> DraftWriteFuture {
                Box::pin(async { Err(DraftError::Network("unused".to_owned())) })
            }
        }
        struct NoopLedger;
        impl crate::event_emitter::EventLedgerTransport for NoopLedger {
            fn build_post_body(
                &self,
                event: &crate::event_emitter::NativeEditorEvent,
            ) -> serde_json::Value {
                event.to_native_payload()
            }
            fn post(
                &self,
                _event: crate::event_emitter::NativeEditorEvent,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<(), crate::event_emitter::EmitError>>
                        + Send,
                >,
            > {
                Box::pin(async { Ok(()) })
            }
        }

        let store = RichEditorDocumentStore::new(Arc::new(Mutex::new(RichEditorState::demo())));
        let visible = vec![
            ("pane-a".to_owned(), "doc-1".to_owned()),
            ("pane-b".to_owned(), "doc-1".to_owned()),
        ];
        store.prepare_visible_views(&visible);
        let authority = store.state_for_view(Some("doc-1"), "pane-a");
        let target = store.state_for_view(Some("doc-1"), "pane-b");
        {
            let mut authority = authority.lock().unwrap();
            authority.doc = BlockNode::doc(vec![BlockNode::paragraph("abcdef")]);
            authority.selection = Selection::caret(DocPosition::new(vec![0, 0], 1));
            let content =
                crate::rich_editor::document_model::doc_json::to_content_json_value(&authority.doc);
            authority.save = Some(SaveManager::new(Arc::new(NoopSave), None, "doc-1", 42));
            authority.draft = Some(DraftManager::new(
                Arc::new(NoopDraft),
                None,
                "doc-1",
                42,
                &content,
            ));
        }
        {
            let mut target = target.lock().unwrap();
            target.selection = Selection::caret(DocPosition::new(vec![0, 0], 4));
        }

        let receipt = crate::event_emitter::NativeEditorEvent::stage_embed_back(
            "artifact-1",
            "pane-b",
            "a".repeat(64),
            "manifest-1",
            "actor-1",
            "workspace-1",
        );
        let event_id = receipt.event_id.clone();
        store
            .insert_stage_embed_at_view_and_request_canonical_save(
                "doc-1",
                "pane-b",
                PendingStageEmbedSave {
                    link: HsLinkNode::new("stage_capture", "artifact-1", "capture"),
                    artifact_id: "artifact-1".to_owned(),
                    sha256: "a".repeat(64),
                    target_pane: "pane-b".to_owned(),
                    workspace_id: "workspace-1".to_owned(),
                    target_epoch: 7,
                    emitter: crate::event_emitter::NativeEditorEventEmitter::new(
                        "workspace-1",
                        Arc::new(NoopLedger),
                        None,
                    ),
                    receipt,
                    in_flight_lease: None,
                    launch_runtime: None,
                },
            )
            .expect("target-caret insertion publishes into canonical save authority");
        let post_insert_selection = target.lock().unwrap().selection.clone();
        assert!(matches!(
            &post_insert_selection,
            Selection::Text { head, .. }
                if head.path == vec![0, 2] && head.char_offset == 0
        ));
        assert_eq!(
            store.canonical_view_key("doc-1"),
            Some(("pane-a".to_owned(), "doc-1".to_owned()))
        );
        // Simulate the next shell frame's deterministic authority recomputation. The canonical
        // document/save/draft/pending receipt must not move or disappear.
        store.prepare_visible_views(&visible);
        let canonical = store
            .canonical_state_for_document("doc-1")
            .expect("stable canonical authority");
        let canonical = canonical.lock().unwrap();
        assert_eq!(canonical.save.as_ref().unwrap().doc_version, 42);
        assert!(canonical.draft.is_some());
        assert!(matches!(
            &canonical.selection,
            Selection::Text { head, .. }
                if head.path == vec![0, 0] && head.char_offset == 1
        ));
        assert_eq!(
            canonical
                .pending_stage_embed_save
                .as_ref()
                .map(|pending| pending.receipt.event_id.as_str()),
            Some(event_id.as_str())
        );
        let Child::Block(paragraph) = &canonical.doc.children[0] else {
            panic!("expected paragraph block");
        };
        let children = &paragraph.children;
        assert!(
            matches!(children.as_slice(), [Child::Text(before), Child::HsLink(_), Child::Text(after)] if before.text.to_string() == "abcd" && after.text.to_string() == "ef")
        );
        drop(canonical);

        let target = target.lock().unwrap();
        assert_eq!(
            target.selection, post_insert_selection,
            "next-frame authority preparation must preserve the retained target view's post-insertion caret"
        );
    }

    #[test]
    fn code_mount_pane_type_and_unbound_stays_unwired() {
        let panel = Arc::new(CodeEditorPanel::new("fn main() {}", "rs"));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::default()));
        let (tx, _rx) = std::sync::mpsc::channel::<CodeEditorHostCommand>();
        let mount = CodeEditorPaneMount::new(panel, session, tx);
        assert_eq!(mount.pane_type(), PaneType::CodeSymbol);
        // No bound session yet: wire_if_needed installs the command sender but NOT the runtime/workspace.
        mount.wire_if_needed();
        assert!(
            !mount.is_wired(),
            "an unbound session must not mark the panel wired"
        );
    }

    #[test]
    fn code_mount_threads_runtime_and_workspace_when_bound() {
        let rt = rt();
        let panel = Arc::new(CodeEditorPanel::new("fn main() {}", "rs"));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::new(
            "ws-42",
            rt.handle().clone(),
        )));
        let (tx, _rx) = std::sync::mpsc::channel::<CodeEditorHostCommand>();
        let mount = CodeEditorPaneMount::new(Arc::clone(&panel), session, tx);
        mount.wire_if_needed();
        assert!(mount.is_wired());
        // The prior-MT hook actually ran: the panel now carries the bound workspace id.
        assert_eq!(panel.workspace_id(), "ws-42");
    }

    #[test]
    fn base_code_document_action_registry_tears_down_and_reactivates_symmetrically() {
        let panel = Arc::new(CodeEditorPanel::new("fn main() {}", "rs"));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::default()));
        let (tx, _rx) = std::sync::mpsc::channel::<CodeEditorHostCommand>();
        let store = CodeEditorDocumentStore::new(Arc::clone(&panel), session, tx);
        let registry = Arc::new(Mutex::new(
            crate::accessibility::editor_action_registry::EditorActionRegistry::new(),
        ));
        store.install_editor_action_registry(registry);
        assert!(panel.has_editor_action_registry());

        panel.uninstall_editor_action_registry();
        assert!(
            !panel.has_editor_action_registry(),
            "closing the base tab removes its bare editor.code.* action namespace"
        );

        store.activate_base_document();
        let reopened = store.panel_for_content_id(None);
        assert!(Arc::ptr_eq(&panel, &reopened));
        assert!(
            reopened.has_editor_action_registry(),
            "reopening the reusable base panel installs a fresh action namespace"
        );
    }

    #[test]
    fn rich_code_block_panel_is_virtual_and_keeps_exact_host_identity() {
        let base = Arc::new(CodeEditorPanel::new("", "rs"));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::default()));
        let (tx, rx) = std::sync::mpsc::channel::<CodeEditorHostCommand>();
        let store = CodeEditorDocumentStore::new(base, session, tx);
        let content_id = "rich-code-block:646f632d31:1".to_owned();
        let panel = store.insert_rich_code_block(content_id.clone(), "rs", "let before = 1;");

        assert!(
            panel.file_path().is_empty(),
            "rich code blocks are never local files"
        );
        assert_eq!(panel.buffer().to_string(), "let before = 1;");
        panel.request_save_for_host();
        let command = rx.recv().expect("virtual code panel save reaches host");
        assert_eq!(command.document_id, content_id);
        assert_eq!(
            command.action,
            crate::code_editor::keymap::CodeEditorAction::Save
        );
    }

    #[test]
    fn rich_code_save_replaces_only_bound_block_and_dispatches_canonical_save() {
        use crate::rich_editor::document_model::node::{Child, NodeKind, TextLeaf};
        use crate::rich_editor::save::save_manager::{
            SaveBackend, SaveError, SaveFuture, SaveManager, SaveState,
        };

        struct NoopSave;
        impl SaveBackend for NoopSave {
            fn save_document(
                &self,
                _document_id: &str,
                _content_json: serde_json::Value,
                _expected_version: u64,
            ) -> SaveFuture {
                Box::pin(async { Err(SaveError::Network("unused".to_owned())) })
            }
        }

        let code = |text: &str| {
            BlockNode::with_children(NodeKind::CodeBlock, vec![Child::Text(TextLeaf::new(text))])
        };
        let base = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
            BlockNode::paragraph("untitled"),
        ]))));
        let store = RichEditorDocumentStore::new(base);
        let state = store.state_for_view(Some("KRD-1"), "pane-a");
        {
            let mut state = state.lock().unwrap();
            state.doc = BlockNode::doc(vec![code("first"), code("second")]);
            state.save = Some(SaveManager::new(Arc::new(NoopSave), None, "KRD-1", 7));
        }
        let opened_document_snapshot = {
            let state = state.lock().unwrap();
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc)
        };

        let stale = store
            .replace_code_block_and_request_save(
                "KRD-1",
                &[0],
                &opened_document_snapshot,
                "not-first",
                "wrong overwrite",
            )
            .expect_err("a stale code snapshot must not overwrite any block");
        assert!(stale.contains("changed after the Code Editor opened"));

        let (expected_version, path, post_edit_snapshot) = store
            .replace_code_block_and_request_save(
                "KRD-1",
                &[1],
                &opened_document_snapshot,
                "second",
                "agent exact code",
            )
            .expect("exact code block update starts the canonical save");
        assert_eq!(expected_version, 7);
        assert_eq!(path, vec![1]);

        let state = state.lock().unwrap();
        let text_at = |index: usize| {
            state.doc.children[index].as_block().unwrap().children[0]
                .as_text()
                .unwrap()
                .text
                .to_string()
        };
        assert_eq!(text_at(0), "first", "unbound code block stays untouched");
        assert_eq!(text_at(1), "agent exact code");
        assert_eq!(
            post_edit_snapshot,
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc)
        );
        assert_eq!(state.pending_bus_undo.len(), 1);
        assert!(matches!(
            &state.save.as_ref().unwrap().state,
            SaveState::Saving {
                expected_version: 7
            }
        ));
    }

    #[test]
    fn rich_code_save_rejects_identical_text_positional_drift() {
        use crate::rich_editor::document_model::node::{Child, NodeKind, TextLeaf};
        use crate::rich_editor::save::save_manager::{
            SaveBackend, SaveError, SaveFuture, SaveManager,
        };

        struct NoopSave;
        impl SaveBackend for NoopSave {
            fn save_document(
                &self,
                _document_id: &str,
                _content_json: serde_json::Value,
                _expected_version: u64,
            ) -> SaveFuture {
                Box::pin(async { Err(SaveError::Network("unused".to_owned())) })
            }
        }

        let code = || {
            BlockNode::with_children(
                NodeKind::CodeBlock,
                vec![Child::Text(TextLeaf::new("identical"))],
            )
        };
        let base = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
            BlockNode::paragraph("untitled"),
        ]))));
        let store = RichEditorDocumentStore::new(base);
        let state = store.state_for_view(Some("KRD-DRIFT"), "pane-a");
        let opened_document_snapshot = {
            let mut state = state.lock().unwrap();
            state.doc = BlockNode::doc(vec![code(), code()]);
            state.save = Some(SaveManager::new(Arc::new(NoopSave), None, "KRD-DRIFT", 3));
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc)
        };

        // Insert another identical block before the bound path. Text-only validation would now
        // silently update the wrong occurrence at [1]; whole-document structural validation rejects.
        state
            .lock()
            .unwrap()
            .doc
            .children
            .insert(0, Child::Block(code()));
        let before_attempt = {
            let state = state.lock().unwrap();
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc)
        };
        let error = store
            .replace_code_block_and_request_save(
                "KRD-DRIFT",
                &[1],
                &opened_document_snapshot,
                "identical",
                "must-not-land",
            )
            .expect_err("identical-text positional drift must fail closed");
        assert!(error.contains("changed structurally"));
        let state = state.lock().unwrap();
        assert_eq!(
            crate::rich_editor::document_model::doc_json::to_content_json_value(&state.doc),
            before_attempt,
            "drift rejection must not mutate any identical block"
        );
        assert!(state.pending_bus_undo.is_empty());
    }

    #[test]
    fn rich_mount_threads_context_and_drains_events() {
        let rt = rt();
        let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
            BlockNode::paragraph("hi"),
        ]))));
        let session: SharedSessionContext = Arc::new(Mutex::new(EditorSessionContext::new(
            "ws-9",
            rt.handle().clone(),
        )));
        let events = RichPaneEvents::new();
        let mount = RichEditorPaneMount::new(Arc::clone(&state), session, events.clone(), "DOC-1");
        assert_eq!(mount.pane_type(), PaneType::LoomWikiPage);
        mount.wire_if_needed();
        assert!(mount.is_wired());
        // The wikilink context bound the workspace (the MT-057 hook ran).
        assert_eq!(state.lock().unwrap().wikilinks.workspace_id, "ws-9");

        // Enqueue an event the way the editor would, then drain: it reaches the shared outbound queue.
        state
            .lock()
            .unwrap()
            .pending_events
            .push(EditorEvent::WikilinkActivated {
                ref_kind: "note".into(),
                ref_value: "DOC-2".into(),
                resolved: true,
            });
        mount.drain_events(&state);
        assert!(
            state.lock().unwrap().pending_events.is_empty(),
            "drained from the editor state"
        );
        let routed = events.take();
        assert_eq!(
            routed.len(),
            1,
            "the event reached the shell's outbound queue"
        );
        assert!(
            events.is_empty(),
            "take() leaves the queue empty (routed exactly once)"
        );
    }

    #[test]
    fn rich_document_store_keeps_two_document_states_distinct() {
        let base = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
            BlockNode::paragraph("untitled"),
        ]))));
        let store = RichEditorDocumentStore::new(Arc::clone(&base));
        let first = store.state_for_content_id(Some("KRD-first"));
        let second = store.state_for_content_id(Some("KRD-second"));
        assert!(!Arc::ptr_eq(&first, &second));
        assert!(!Arc::ptr_eq(&first, &base));

        first.lock().unwrap().doc = BlockNode::doc(vec![BlockNode::paragraph("first edit")]);
        second.lock().unwrap().doc = BlockNode::doc(vec![BlockNode::paragraph("second edit")]);

        assert_eq!(
            store
                .state_for_content_id(Some("KRD-first"))
                .lock()
                .unwrap()
                .block_plain_text(0)
                .as_deref(),
            Some("first edit")
        );
        assert_eq!(
            store
                .state_for_content_id(Some("KRD-second"))
                .lock()
                .unwrap()
                .block_plain_text(0)
                .as_deref(),
            Some("second edit")
        );
        assert_eq!(
            base.lock().unwrap().block_plain_text(0).as_deref(),
            Some("untitled")
        );
    }

    #[test]
    fn flight_recorder_fetch_rejects_late_prior_workspace_completion() {
        use crate::flight_recorder_pane::{
            FlightRecorderQuery, FlightRecorderQueryRows, FlightRecorderRow,
        };
        let cell = FlightRecorderFetchCell::new();
        let generation_a = cell.begin("workspace-a");
        let generation_b = cell.begin("workspace-b");
        let row = |id: &str| FlightRecorderQueryRows {
            rows: vec![FlightRecorderRow {
                event_id: id.to_owned(),
                action: "document_saved".to_owned(),
                event_code: None,
                actor_id: crate::event_emitter::DEFAULT_ACTOR_ID.to_owned(),
                ts_utc: "2026-07-16T00:00:00Z".to_owned(),
            }],
            quarantined: Vec::new(),
        };
        assert!(cell.deliver_if_current(generation_b, "workspace-b", Ok(row("event-b"))));
        assert!(!cell.deliver_if_current(generation_a, "workspace-a", Ok(row("event-a"))));
        let current = cell.rows().expect("current workspace result");
        assert_eq!(current.rows[0].event_id, "event-b");
    }
}
