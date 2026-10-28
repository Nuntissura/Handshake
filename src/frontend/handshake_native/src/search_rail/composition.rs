//! Handpick presentation over actual mounted Notes editor handles.
//! Notes retains document identity, transactions, completion, save and draft recovery.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::rich_editor::renderer::rich_editor_widget::RichEditorState;

#[derive(Clone)]
struct NoteHandle {
    document_id: String,
    view_id: String,
    editor: Arc<Mutex<RichEditorState>>,
    authority: Arc<Mutex<RichEditorState>>,
}

struct MountedSurface {
    editor: handpick::EditorId,
    view_id: String,
    document_id: String,
    response: egui::Response,
}

struct WritingSearchInput {
    editor: handpick::EditorId,
    view_id: String,
    document_id: String,
    owner: crate::rich_editor::document_owner::DocumentEditOwner,
    context: crate::rich_editor::document_owner::EditorSaveContext,
    selection: crate::rich_editor::document_model::selection::Selection,
}

#[derive(Default)]
pub(super) struct MountedNotes {
    core: Option<handpick::WritingSession>,
    editors: HashMap<handpick::EditorId, NoteHandle>,
    composition: Option<handpick::CompositionToken>,
    presented: Option<(handpick::EditorId, u64, String)>,
    surface: Option<MountedSurface>,
    search_input: Option<Result<WritingSearchInput, String>>,
    pub(super) space_limited: bool,
    released: Vec<(String, String)>,
    bus: Option<Arc<Mutex<crate::interop::InteractionBus>>>,
    pub(super) requested: bool,
    error: Option<String>,
    observations: bool,
}

impl Drop for MountedNotes {
    fn drop(&mut self) {
        if let Some(core) = &mut self.core {
            core.revoke();
        }
        // These handles belong to the app's Notes owner; do not retire that owner here.
    }
}

impl MountedNotes {
    pub(super) fn retained_bindings(&self) -> Vec<(String, String)> {
        let mut bindings: Vec<_> = self.editors.values()
            .map(|handle| (handle.view_id.clone(), handle.document_id.clone())).collect();
        bindings.sort();
        bindings
    }

    pub(super) fn focused_binding(&self) -> Option<(String, String)> {
        self.focused_binding_for_search_button(None)
    }

    fn focused_binding_for_search_button(&self, search_button: Option<egui::Rect>) -> Option<(String, String)> {
        if !self.expanded() { return None; }
        let active = self.active()?;
        let surface = self.surface.as_ref()?;
        if self.core.as_ref()?.editor()?.0 != &surface.editor
            || active.view_id != surface.view_id || active.document_id != surface.document_id
            || !surface.response.has_focus()
            || !surface.response.ctx.content_rect().contains_rect(surface.response.rect)
            || surface.response.ctx.input(|input| input.pointer.any_pressed()
                && input.pointer.interact_pos().is_none_or(|pos| !surface.response.rect.contains(pos)
                    && search_button.is_none_or(|button| !button.contains(pos)))) {
            return None;
        }
        let focused = crate::interop::InteractionBus::with_try_lock(self.bus.as_ref()?, |bus| {
            bus.focus_owner().is_some_and(|id| id.as_ref() == active.view_id)
        }).unwrap_or(false);
        focused.then(|| (active.view_id, active.document_id))
    }

    pub(super) fn take_released_bindings(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.released)
    }

    pub(super) fn retire_session(&mut self) {
        self.released.extend(self.retained_bindings());
        if let Some(core) = &mut self.core { core.revoke(); }
        self.core = None;
        self.editors.clear();
        self.composition = None;
        self.presented = None;
        self.surface = None;
        self.search_input = None;
        self.space_limited = false;
        self.requested = false;
        self.error = None;
    }

    pub(super) fn visible_binding(&self) -> Option<(String, String)> {
        if !self.core.as_ref()?.expanded() { return None }
        let active = self.active()?;
        Some((active.view_id, active.document_id))
    }

    fn can_release(handle: &NoteHandle) -> bool {
        let clean = handle.editor.lock().is_ok_and(|state| {
            !state.preedit.is_active() && state.document_owner().with_save_gate(|gate| {
                gate.document_is_loaded() && !gate.is_dirty()
            })
        });
        clean && handle.authority.lock().is_ok_and(|authority| {
            use crate::rich_editor::save::save_manager::SaveState;
            authority.save.as_ref().is_some_and(|save| {
                matches!(&save.state, SaveState::Idle) && !save.requires_recovery()
            }) && authority.draft.as_ref().is_none_or(|draft| draft.release_ready()) && authority.pending_stage_embed_save.is_none()
                && authority.pending_stage_embed_completion.is_none()
        })
    }

    fn prune_released(&mut self) {
        let Some(core) = &self.core else { return };
        let mut pending = false;
        self.editors.retain(|id, handle| {
            if core.editor().is_some_and(|(active, _)| active == id)
                || core.pins().iter().chain(core.bookmarks()).any(|pin| &pin.editor == id) {
                return true;
            }
            if !Self::can_release(handle) {
                pending = true;
                return true;
            }
            self.released.push((handle.view_id.clone(), handle.document_id.clone()));
            false
        });
        if pending && self.error.is_none() {
            self.error = Some("Notes save or recovery is still pending. Open the note in Notes to finish it; its editor is retained".into());
        }
    }

    fn observe(&self, phase: crate::diagnostics::gui_events::GuiDiagnosticPhase) {
        if self.observations {
            crate::diagnostics::gui_events::record_gui_observation(
                crate::diagnostics::gui_events::GuiDiagnosticOperation::HandpickWriting, phase);
        }
    }

    fn attach(
        &mut self,
        state: Arc<Mutex<RichEditorState>>,
        workspace: &str,
        account: &Arc<crate::local_account::AuthenticatedContext>,
        base: &str,
        store: &crate::editor_pane_factories::RichEditorDocumentStore,
        runtime: Option<&tokio::runtime::Handle>,
    ) -> Result<(), String> {
        let (revision, committed, document_id, selection) = {
            let state = state.lock().map_err(|_| "Notes editor unavailable")?;
            if state.preedit.is_active() { return Err("Finish the current text composition first".into()) }
            let owner = state.document_owner();
            let identity = owner
                .document_identity()
                .ok_or("Open an existing note first")?;
            if !owner.has_active_binding(workspace, base, account) {
                return Err("Notes context is no longer active".into());
            }
            let context = owner.current_context().map_err(|error| error.to_string())?;
            let committed = owner.with_save_gate(|gate| {
                if !gate.document_is_loaded() {
                    return Err("Wait for the note to load");
                }
                Ok((!gate.is_dirty()).then(|| {
                    (
                        identity.document_id().to_owned(),
                        gate.known_server_version(),
                    )
                }))
            })?;
            (
                context.edit_revision,
                committed,
                identity.document_id().to_owned(),
                state.selection.clone(),
            )
        };
        runtime.ok_or("Notes runtime unavailable")?;
        let authority = store.canonical_state_for_document(&document_id)
            .ok_or("Notes authority unavailable")?;
        let id = self.editors.iter()
            .find_map(|(id, handle)| (handle.document_id == document_id).then(|| id.clone()))
            .map(Ok)
            .unwrap_or_else(|| handpick::EditorId::new(uuid::Uuid::new_v4().to_string()))
            .map_err(str::to_owned)?;
        if self.core.is_none() {
            self.core = Some(handpick::WritingSession::new(
                handpick::ContextId::new(account.session_id.clone()).map_err(str::to_owned)?,
                handpick::SessionId::new(uuid::Uuid::new_v4().to_string())
                    .map_err(str::to_owned)?,
                8,
            ));
        }
        let core = self.core.as_mut().ok_or("Notes session unavailable")?;
        if core.editor().is_some_and(|(active, _)| active == &id) {
            core.expand().map_err(|error| error.to_string())?;
            return Ok(());
        }
        let parked = core.pins().iter().find(|pin| pin.editor == id).map(|pin| pin.id.clone());
        let bookmark = core.bookmarks().iter().find(|pin| pin.editor == id).map(|pin| pin.id.clone());
        if let Some(pin) = parked {
            Self::restore_parked(core, &pin)?;
            core.expand().map_err(|error| error.to_string())?;
            return Ok(());
        }
        Self::park_active(core)?;
        if let Some(pin) = bookmark {
            core.open_bookmark(&pin).map_err(|error| error.to_string())?;
            return Ok(());
        }
        match committed {
            Some((note, version)) => core.bind_committed(
                id.clone(),
                revision,
                handpick::NoteId::new(note).map_err(str::to_owned)?,
                handpick::RevisionId::new(version.to_string()).map_err(str::to_owned)?,
            ),
            None => core.bind(id.clone(), revision),
        }
        .map_err(|error| error.to_string())?;
        let view_id = format!("handpick.notes.{}", id.as_str());
        let state = store.state_for_view(Some(&document_id), &view_id);
        store.synchronize_view_from_canonical(&document_id, &view_id);
        {
            let mut view = state.lock().map_err(|_| "Notes view unavailable")?;
            view.bind_authenticated_context(Some(account.clone()), base);
            view.selection = selection;
        }
        // attach admitted an already-loaded owner, not an empty new document.
        store.mark_view_ready(&document_id, &view_id);
        self.editors.insert(id, NoteHandle { document_id, view_id, editor: state, authority });
        core.expand().map_err(|error| error.to_string())
    }

    fn close_active(core: &mut handpick::WritingSession) -> Result<bool, String> {
        if core.editor().is_none() { return Ok(true) }
        let fence = core.fence().map_err(|error| error.to_string())?;
        core.close_quicknote(&fence)
            .map(|closed| closed == handpick::QuicknoteClose::Closed)
            .map_err(|error| error.to_string())
    }

    fn park_active(core: &mut handpick::WritingSession) -> Result<(), String> {
        if Self::close_active(core)? { return Ok(()) }
        let pin = handpick::PinId::new(uuid::Uuid::new_v4().to_string()).map_err(str::to_owned)?;
        core.park(pin).map_err(|error| error.to_string())
    }

    fn restore_parked(core: &mut handpick::WritingSession, pin: &handpick::PinId) -> Result<(), String> {
        let replacement = if Self::close_active(core)? { None } else {
            Some(handpick::PinId::new(uuid::Uuid::new_v4().to_string()).map_err(str::to_owned)?)
        };
        core.restore(pin, replacement).map_err(|error| error.to_string())
    }

    fn open_bookmark(core: &mut handpick::WritingSession, pin: &handpick::PinId) -> Result<(), String> {
        let target = core.bookmarks().iter().find(|bookmark| &bookmark.id == pin)
            .ok_or("Bookmark is no longer available")?.editor.clone();
        if !core.editor().is_some_and(|(active, _)| active == &target) {
            Self::park_active(core)?;
        }
        core.open_bookmark(pin).map_err(|error| error.to_string())
    }

    fn bookmark_active(core: &mut handpick::WritingSession) -> Result<(), String> {
        let fence = core.fence().map_err(|error| error.to_string())?;
        let existing = core.bookmarks().iter().find(|pin| pin.editor == fence.editor).cloned();
        let pin = if let Some(existing) = existing {
            if existing.revision == fence.revision { return Ok(()) }
            core.remove_bookmark(&existing.id).map_err(|error| error.to_string())?;
            existing.id
        } else {
            handpick::PinId::new(uuid::Uuid::new_v4().to_string()).map_err(str::to_owned)?
        };
        core.bookmark(&core.fence().map_err(|error| error.to_string())?, pin)
            .map_err(|error| error.to_string())
    }

    fn can_remove_bookmark(&self, editor: &handpick::EditorId) -> bool {
        if self.core.as_ref().is_some_and(|core| core.editor().is_some_and(|(active, _)| active == editor)) {
            return true;
        }
        self.editors.get(editor).is_some_and(Self::can_release)
    }

    fn active(&self) -> Option<NoteHandle> {
        self.core
            .as_ref()?
            .editor()
            .and_then(|(id, _)| self.editors.get(id))
            .cloned()
    }

    fn synchronize(&mut self) -> Result<(), String> {
        let Some(state) = self.active() else {
            return Ok(());
        };
        let state = state
            .editor
            .lock()
            .map_err(|_| "Notes editor unavailable")?;
        let owner = state.document_owner();
        let context = owner.current_context().map_err(|error| error.to_string())?;
        let core = self.core.as_mut().ok_or("Notes session unavailable")?;
        let fence = core.fence().map_err(|error| error.to_string())?;
        if context.edit_revision != fence.revision {
            core.edited(&fence, context.edit_revision)
                .map_err(|error| error.to_string())?;
        }
        if state.preedit.is_active() && self.composition.is_none() {
            self.composition = Some(
                core.begin_composition(&core.fence().map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?,
            );
        } else if !state.preedit.is_active() {
            if let Some(token) = self.composition.take() {
                core.end_composition(&token)
                    .map_err(|error| error.to_string())?;
            }
        }
        if let Some(properties) = &state.properties {
            let fence = core.fence().map_err(|error| error.to_string())?;
            let presentation = (fence.editor.clone(), context.edit_revision, properties.doc_metadata.title.clone());
            if self.presented.as_ref() != Some(&presentation) {
                let text = state.with_document(crate::rich_editor::save::export::export_plain_text)
                    .map_err(|error| error.to_string())?;
                core.present_draft(&fence,
                    u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX),
                    u32::try_from(text.lines().count()).unwrap_or(u32::MAX),
                    text.is_empty(), &properties.doc_metadata.title).map_err(|error| error.to_string())?;
                self.presented = Some(presentation);
            }
        }
        let clean = owner.with_save_gate(|gate| gate.document_is_loaded() && !gate.is_dirty());
        if clean && self.composition.is_none() && core.committed_note().is_none() {
            if let Some(identity) = owner.document_identity() {
                let version = owner.with_save_gate(|gate| gate.known_server_version());
                core.bind_committed(
                    fence.editor,
                    context.edit_revision,
                    handpick::NoteId::new(identity.document_id()).map_err(str::to_owned)?,
                    handpick::RevisionId::new(version.to_string()).map_err(str::to_owned)?,
                )
                .map_err(|error| error.to_string())?;
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Completed);
            }
        }
        Ok(())
    }

    pub(super) fn bookmarks_on_rim(
        &mut self,
        ui: &mut egui::Ui,
        rim: egui::Rect,
        workspace: &str,
        base: &str,
        account: &Arc<crate::local_account::AuthenticatedContext>,
    ) {
        if !self.has_bookmarks() || !rim.is_positive() { return; }
        // This child shares the field's layer and does not allocate another header row.
        let mut tabs = ui.new_child(egui::UiBuilder::new()
            .id_salt("handpick.bookmarks")
            .max_rect(rim)
            .layout(egui::Layout::left_to_right(egui::Align::Center)));
        tabs.set_clip_rect(rim.intersect(ui.clip_rect()));
        tabs.spacing_mut().button_padding = egui::vec2(3.0, 0.0);
        tabs.spacing_mut().item_spacing.x = 2.0;
        tabs.spacing_mut().interact_size.y = 0.0;
        tabs.style_mut().text_styles.insert(egui::TextStyle::Button,
            egui::FontId::proportional((rim.height() * 0.65).min(11.0)));
        egui::ScrollArea::horizontal()
            .id_salt("overflow")
            .min_scrolled_height(0.0)
            .max_height(rim.height())
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(&mut tabs, |ui| {
                ui.horizontal(|ui| self.bookmarks(ui, workspace, base, account));
            });
    }

    fn bookmarks(
        &mut self,
        ui: &mut egui::Ui,
        workspace: &str,
        base: &str,
        account: &Arc<crate::local_account::AuthenticatedContext>,
    ) {
        let Some(core) = &self.core else { return };
        let parked = core.pins().to_vec();
        let bookmarks = core.bookmarks().to_vec();
        for (index, (pin, parked)) in parked
            .into_iter()
            .map(|pin| (pin, true))
            .chain(bookmarks.into_iter().map(|pin| (pin, false)))
            .enumerate()
        {
            let response = ui
                .push_id(pin.id.as_str(), |ui| {
                    ui.add(egui::Button::new((index + 1).to_string()).small())
                })
                .inner;
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_author_id(format!("handpick.bookmark.{}", pin.id.as_str()));
                node.set_label(format!("Bookmark {}", index + 1));
            });
            if response.has_focus() { response.scroll_to_me(Some(egui::Align::Center)); }
            if response.hovered() || response.has_focus() {
                let excerpt = self
                    .editors
                    .get(&pin.editor)
                    .and_then(|state| state.editor.lock().ok())
                    .and_then(|state| state.block_plain_text(0))
                    .unwrap_or_default();
                egui::Tooltip::for_widget(&response).show(|ui| {
                    ui.label(excerpt.chars().take(256).collect::<String>());
                });
            }
            if response.clicked() {
                let authorized = self.editors.get(&pin.editor).is_some_and(|state| {
                    state.editor.lock().is_ok_and(|state| {
                        state
                            .document_owner()
                            .has_active_binding(workspace, base, account)
                    })
                });
                if !authorized {
                    self.error = Some("Bookmark authority changed; reopen through Notes".into());
                    continue;
                }
                let core = self.core.as_mut().expect("existing Notes session");
                let result = if parked {
                    Self::restore_parked(core, &pin.id)
                } else { Self::open_bookmark(core, &pin.id) };
                self.observe(if result.is_ok() { crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted }
                    else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
                if let Err(error) = result {
                    self.error = Some(error);
                }
            }
            if !parked {
                let can_remove = self.can_remove_bookmark(&pin.editor);
                let remove = ui.push_id(("remove", pin.id.as_str()), |ui| {
                    ui.add_enabled(can_remove, egui::Button::new("×").small())
                }).inner.on_disabled_hover_text("Open or save this note before removing its bookmark; unsaved edits are retained");
                ui.ctx().accesskit_node_builder(remove.id, |node| {
                    node.set_author_id(format!("handpick.bookmark.remove.{}", pin.id.as_str()));
                    node.set_label(format!("Remove bookmark {}", index + 1));
                });
                if remove.has_focus() { remove.scroll_to_me(Some(egui::Align::Center)); }
                if remove.clicked() {
                    let result = self.core.as_mut().expect("existing Notes session")
                        .remove_bookmark(&pin.id).map_err(|error| error.to_string());
                    self.observe(if result.is_ok() { crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted }
                        else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
                    if let Err(error) = result { self.error = Some(error); }
                }
            }
        }
    }

    pub(super) fn prepare(
        &mut self,
        ctx: &egui::Context,
        workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>,
        base: &str,
        current: Option<Arc<Mutex<RichEditorState>>>,
        store: &crate::editor_pane_factories::RichEditorDocumentStore,
        runtime: Option<&tokio::runtime::Handle>,
        observations: bool,
    ) {
        self.observations = observations;
        self.bus = Some(crate::interop::InteractionBus::get_or_init(ctx));
        for handle in self.editors.values_mut() {
            let document_id = handle
                .editor
                .lock()
                .ok()
                .and_then(|state| state.document_owner().document_identity())
                .map(|identity| identity.document_id().to_owned());
            if let Some(authority) =
                document_id.and_then(|id| store.canonical_state_for_document(&id))
            {
                handle.authority = authority;
            }
        }
        if !self.editors.values().all(|handle| {
            let owner = handle.editor.lock().ok().map(|state| state.document_owner());
            let authority = handle.authority.lock().ok().map(|state| state.document_owner());
            match (owner, authority, account) {
                (Some(owner), Some(authority), Some(account)) => owner.same_owner(&authority)
                    && owner.has_active_binding(workspace, base, account)
                    && authority.has_active_binding(workspace, base, account),
                _ => false,
            }
        }) {
            self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Stale);
            self.retire_session();
            self.observations = observations;
            self.error =
                Some("Notes authority changed; reopen the note through its current owner".into());
        }
        if std::mem::take(&mut self.requested) {
            let result = match (account.filter(|account| account.is_active()), current) {
                (Some(account), Some(current)) => {
                    self.attach(current, workspace, account, base, store, runtime)
                }
                _ => Err("Open an authorized note in Notes to edit it here".into()),
            };
            self.observe(if result.is_ok() { crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted }
                else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
            if let Err(error) = result {
                self.error = Some(error);
            }
        }
        if let Err(error) = self.synchronize() {
            self.error = Some(error);
        }
        if !account.is_some_and(|account| account.is_active()) {
            return;
        }
        self.prune_released();
    }

    pub(super) fn expanded(&self) -> bool {
        self.core.as_ref().is_some_and(|core| core.expanded())
    }

    pub(super) fn has_bookmarks(&self) -> bool {
        self.core.as_ref().is_some_and(|core| !core.pins().is_empty() || !core.bookmarks().is_empty())
    }

    pub(super) fn toggle(&mut self) {
        if self.core.as_ref().is_none_or(|core| core.editor().is_none()) {
            self.request_attach_current();
            return;
        }
        let Some(core) = &mut self.core else { return };
        let result = if core.expanded() { core.compact() } else { core.expand() };
        if let Err(error) = result { self.error = Some(error.to_string()); }
    }

    pub(super) fn request_attach_current(&mut self) {
        self.requested = true;
    }

    pub(super) fn set_query_purpose(&mut self, purpose: handpick::QueryPurpose) {
        if let Some(core) = &mut self.core {
            if core.query_purpose() != purpose {
                if let Err(error) = core.set_query_purpose(purpose) { self.error = Some(error.to_string()); }
            }
        }
    }

    pub(super) fn take_selected_query(&self, ctx: &egui::Context, frame_composing: bool) -> Option<String> {
        if frame_composing || self.composing() || self.focused_binding().is_none() { return None; }
        let requested = ctx.input(|input| input.events.iter().any(|event| matches!(event,
            egui::Event::Key { key: egui::Key::Enter, modifiers, pressed: true, .. }
                if modifiers.matches_logically(egui::Modifiers::ALT))));
        if !requested { return None; }
        let handle = self.active()?;
        let text = handle.editor.lock().ok()?.selected_text_for_stage().filter(|text| !text.trim().is_empty())?;
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::Enter)).then_some(text)
    }

    pub(super) fn prepare_search_button(&mut self, ctx: &egui::Context, frame_composing: bool) {
        if frame_composing || self.composing() || !self.expanded() {
            self.search_input = None;
            return;
        }
        if self.search_input.is_some() && ctx.input(|input|
            !input.pointer.primary_pressed() && (input.pointer.primary_down() || input.pointer.primary_released())) {
            return;
        }
        let search_id = egui::Id::new("handpick.search");
        // Keyboard focus can move directly from Notes onto Search. Retain only metadata until
        // the button's real clicked response, including Space/Enter and AccessKit, is consumed.
        if self.search_input.is_some() && ctx.memory(|memory| memory.has_focus(search_id)) {
            return;
        }
        let button = ctx.read_response(search_id).map(|response| response.rect);
        let targeted_press = ctx.input(|input| input.pointer.primary_pressed()
            && input.pointer.interact_pos().is_some_and(|pos| button.is_some_and(|rect| rect.contains(pos))));
        let focused = self.focused_binding_for_search_button(if targeted_press { button } else { None });
        if focused.is_some() {
            self.search_input = Some((|| {
                let active = self.active().ok_or("Notes editor unavailable")?;
                let state = active.editor.lock().map_err(|_| "Notes editor unavailable")?;
                let owner = state.document_owner();
                let context = owner.current_context().map_err(|_| "Notes authority unavailable")?;
                Ok(WritingSearchInput { editor: self.surface.as_ref().ok_or("Notes view unavailable")?.editor.clone(),
                    view_id: active.view_id, document_id: active.document_id, owner, context,
                    selection: state.selection.clone() })
            })());
        } else if targeted_press && self.search_input.is_some() {
            self.search_input = Some(Err("Notes focus changed; search again from the current editor".into()));
        } else if !ctx.input(|input| input.pointer.primary_down() || input.pointer.primary_released()) {
            self.search_input = None;
        } else if ctx.input(|input| input.pointer.any_pressed()) && !targeted_press {
            self.search_input = None;
        }
    }

    pub(super) fn take_search_button_input(&mut self, ctx: &egui::Context, workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>, base: &str,
        frame_composing: bool) -> Result<Option<String>, String> {
        let Some(input) = self.search_input.take() else { return Ok(None) };
        let input = input?;
        let valid = (|| {
            if frame_composing || self.composing() || !self.expanded() { return None; }
            let active = self.active()?;
            let surface = self.surface.as_ref()?;
            if surface.editor != input.editor || surface.view_id != input.view_id
                || surface.document_id != input.document_id || active.view_id != input.view_id
                || active.document_id != input.document_id || surface.response.ctx != *ctx
                || !ctx.content_rect().contains_rect(surface.response.rect) { return None; }
            let (owner, selection) = {
                let state = active.editor.lock().ok()?;
                (state.document_owner(), state.selection.clone())
            };
            let authority = active.authority.lock().ok()?.document_owner();
            if !owner.same_owner(&input.owner) || !owner.same_owner(&authority)
                || owner.current_context().ok()? != input.context || selection != input.selection
                || !account.is_some_and(|account| owner.has_active_binding(workspace, base, account)) {
                return None;
            }
            Some(())
        })().is_some();
        if !valid { return Err("Notes selection or authority changed; search again from the current editor".into()); }
        // Only an explicit clicked Search reaches this read. Idle frames never copy document text.
        let active = self.active().ok_or("Notes editor unavailable")?;
        let (owner, text) = {
            let state = active.editor.lock().map_err(|_| "Notes editor unavailable")?;
            let owner = state.document_owner();
            if !owner.same_owner(&input.owner) || state.selection != input.selection
                || owner.current_context().ok() != Some(input.context) {
                return Err("Notes selection or authority changed; search again from the current editor".into());
            }
            let has_selection = matches!(&state.selection,
                crate::rich_editor::document_model::selection::Selection::Text { anchor, head } if anchor != head);
            let text = if has_selection {
                state.selected_text_for_stage().ok_or("Notes selection unavailable; select the current writing again")?
            } else {
                owner.with_document(crate::rich_editor::save::export::export_plain_text)
                    .map_err(|_| "Notes content unavailable")?
            };
            (owner, text)
        };
        let authority = active.authority.lock().map_err(|_| "Notes authority unavailable")?.document_owner();
        if !owner.same_owner(&authority) || owner.current_context().ok() != Some(input.context)
            || !account.is_some_and(|account| owner.has_active_binding(workspace, base, account)) {
            return Err("Notes selection or authority changed; search again from the current editor".into());
        }
        if text.trim().is_empty() { return Err("Enter text or select writing to search".into()); }
        if text.len() > 4096 { return Err("Select a shorter phrase for search; the composition is retained".into()); }
        Ok(Some(text))
    }

    pub(super) fn completion_open(&self) -> bool {
        self.active().is_some_and(|handle| handle.editor.lock().is_ok_and(|state|
            state.wikilink_autocomplete.is_some() || state.tag_autocomplete.is_some() || state.slash_menu.is_some()))
    }

    pub(super) fn composing(&self) -> bool {
        self.active().is_some_and(|handle| handle.editor.lock().is_ok_and(|state| state.preedit.is_active()))
    }

    pub(super) fn focus_editor(&mut self) {
        if let Some(handle) = self.active() {
            if let Ok(mut state) = handle.editor.lock() { state.request_editor_focus(); }
        }
    }

    pub(super) fn controls(
        &mut self, ui: &mut egui::Ui, workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>, base: &str, frame_composing: bool,
    ) {
        let active = self.active();
        let composing = frame_composing || active.as_ref().is_some_and(|handle|
            handle.editor.lock().is_ok_and(|state| state.preedit.is_active()));
        let can_save = !composing && active.as_ref().is_some_and(|handle|
            handle.authority.lock().is_ok_and(|state| state.has_save_context()));
        let save = ui.add_enabled(can_save, egui::Button::new("Save"))
            .on_disabled_hover_text("Open a note to save it here");
        ui.ctx().accesskit_node_builder(save.id, |node| node.set_author_id("handpick.notes.save"));
        let save_key = !composing && self.focused_binding().is_some()
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Enter));
        if save.clicked() || save_key {
            self.request_save(workspace, account, base, frame_composing);
        }
        let bookmark = ui.add_enabled(active.is_some() && !composing, egui::Button::new("Bookmark"));
        ui.ctx().accesskit_node_builder(bookmark.id, |node| node.set_author_id("handpick.notes.bookmark"));
        if bookmark.clicked() {
            let result = self.core.as_mut().ok_or_else(|| "Notes session unavailable".to_owned())
                .and_then(Self::bookmark_active);
            self.observe(if result.is_ok() { crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted }
                else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
            if let Err(error) = result { self.error = Some(error); }
        }
        if active.is_some() {
            let close = ui.add_enabled(!composing, egui::Button::new("Close"));
            ui.ctx().accesskit_node_builder(close.id, |node| node.set_author_id("handpick.notes.close"));
            if close.clicked() {
                let result = self.core.as_mut().ok_or_else(|| "Notes session unavailable".to_owned())
                    .and_then(|core| core.close_quicknote(&core.fence().map_err(|error| error.to_string())?)
                        .map_err(|error| error.to_string()));
                self.observe(if matches!(result, Ok(handpick::QuicknoteClose::Closed)) {
                    crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted
                } else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
                match result {
                    Ok(handpick::QuicknoteClose::Closed) => {}
                    Ok(handpick::QuicknoteClose::NeedsSaveOrPin) =>
                        self.error = Some("Save or bookmark the current edit before closing".into()),
                    Err(error) => self.error = Some(error),
                }
            }
        }
    }

    fn request_save(&mut self, workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>, base: &str, frame_composing: bool) {
        let result = (|| {
            if frame_composing { return Err("Finish the current text composition first"); }
            let handle = self.active().ok_or("Notes editor unavailable")?;
            let composing = handle.editor.lock().map_err(|_| "Notes editor unavailable")?.preedit.is_active();
            if composing { return Err("Finish the current text composition first"); }
            let mut state = handle.authority.lock().map_err(|_| "Notes authority unavailable")?;
            if !account.is_some_and(|account| state.document_owner().has_active_binding(workspace, base, account)) {
                return Err("Notes authority changed; save was not dispatched");
            }
            if !state.request_save_for_host() { return Err("Notes save is unavailable; edits retained"); }
            Ok(())
        })();
        self.observe(if result.is_ok() { crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted }
            else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
        if let Err(error) = result { self.error = Some(error.into()); }
    }

    fn save_status(&self) -> Option<String> {
        let handle = self.active()?;
        let state = handle.authority.lock().ok()?;
        let save = state.save.as_ref()?;
        use crate::rich_editor::save::save_manager::SaveState;
        Some(match &save.state {
            SaveState::Saving { .. } => "Saving note",
            SaveState::Conflict { .. } | SaveState::ConfirmKeepYours { .. } => "Save conflict; open the note to resolve it",
            SaveState::Error(_) => "Save failed; open the note for recovery",
            SaveState::Idle => if state.document_owner().with_save_gate(|gate| gate.is_dirty()) { "Unsaved changes" } else { "Saved note" },
        }.into())
    }

    pub(super) fn body(&mut self, ui: &mut egui::Ui, mount: &crate::editor_pane_factories::RichEditorPaneMount) {
        self.surface = None;
        self.space_limited = false;
        if let Some(status) = self.save_status() { ui.small(status); }
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
            let dismiss = ui.button("Dismiss message");
            ui.ctx().accesskit_node_builder(dismiss.id, |node| node.set_author_id("handpick.notes.dismiss"));
            if dismiss.clicked() { self.error = None; }
        }
        if self.expanded() {
            if let Some(state) = self.active() {
                if let Some(height) = notes_body_height(ui.ctx().content_rect().height(),
                    ui.ctx().content_rect().bottom() - ui.cursor().top()) {
                    let response = ui.allocate_ui(egui::vec2(ui.available_width(), height), |ui|
                        ui.push_id(&state.view_id, |ui|
                            mount.render_view(ui, Some(&state.document_id), &state.view_id)).inner).inner;
                    if let (Some(response), Some((editor, _))) = (response, self.core.as_ref().and_then(|core| core.editor())) {
                        self.surface = Some(MountedSurface { editor: editor.clone(), view_id: state.view_id,
                            document_id: state.document_id, response });
                    }
                } else { self.space_limited = true; }
            }
        }
        if let Err(error) = self.synchronize() { self.error = Some(error); }
        self.prune_released();
    }
}

fn notes_body_height(viewport_height: f32, remaining: f32) -> Option<f32> {
    (remaining >= 120.0).then(|| (viewport_height * 0.4).clamp(120.0, 240.0).min(remaining))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput { focused: true, screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO, egui::vec2(600.0, 500.0))), events, ..Default::default() }
    }

    fn enter(modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true,
            repeat: false, modifiers }
    }

    #[test]
    fn bookmark_rim_preserves_header_height_and_accessible_actions() {
        use super::super::{HandpickHost, RailView, RailVisuals, show_header};
        use crate::editor_pane_factories::{EditorSessionContext, RichEditorPaneMount, RichPaneEvents};
        use crate::rich_editor::document_model::node::BlockNode;
        for width in [640.0, 1280.0] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(Vec::new()))));
            let mount = RichEditorPaneMount::new(state,
                Arc::new(Mutex::new(EditorSessionContext::default())),
                RichPaneEvents::default(), "unbound-component-note");
            let store = mount.document_store();
            let base = "http://127.0.0.1:9911";
            let account = crate::local_account::mock_account_context(base);
            let mut host = HandpickHost::default();
            host.bind(&ctx, "workspace", Some(account.clone()), base);
            let mut baseline = None;
            for count in [0, 1, 8] {
                let mut core = session();
                for index in 0..count {
                    core.bind(handpick::EditorId::new(format!("editor-{index}")).unwrap(), 0).unwrap();
                    core.bookmark(&core.fence().unwrap(),
                        handpick::PinId::new(format!("pin-{index}")).unwrap()).unwrap();
                    assert_eq!(core.close_quicknote(&core.fence().unwrap()).unwrap(),
                        handpick::QuicknoteClose::Closed);
                }
                host.notes.core = Some(core);
                let mut field = egui::Rect::NOTHING;
                let mut remaining_top = 0.0;
                let output = ctx.run(egui::RawInput { focused: true,
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO,
                        egui::vec2(width, 480.0))), ..Default::default() }, |ctx| {
                    field = show_header(ctx, RailView { has_workspace: true, colors: RailVisuals {
                        active_pill_bg: egui::Color32::BLUE, inactive_pill_bg: egui::Color32::GRAY,
                        text: egui::Color32::WHITE, accent: egui::Color32::LIGHT_BLUE,
                    } }, &mut host, "workspace", Some(account.clone()), None, base, false,
                        None, &store, &mount).0.anchor;
                    remaining_top = ctx.available_rect().top();
                });
                let geometry = (field, remaining_top);
                if let Some(expected) = baseline {
                    assert_eq!(geometry, expected, "bookmarks must not add a header row or move the input");
                } else { baseline = Some(geometry); }
                let tree = output.platform_output.accesskit_update.unwrap();
                for index in 0..count {
                    let author = format!("handpick.bookmark.pin-{index}");
                    let (id, node) = tree.nodes.iter()
                        .find(|(_, node)| node.author_id() == Some(author.as_str())).unwrap();
                    assert_eq!(node.label(), Some(format!("Bookmark {}", index + 1).as_str()));
                    let response = ctx.read_response(unsafe { egui::Id::from_high_entropy_bits(id.0) }).unwrap();
                    assert!(response.rect.top() >= 0.0 && response.rect.bottom() <= field.top(),
                        "bookmark hitboxes stay on the rim above the editable text");
                    assert!(tree.nodes.iter().any(|(_, node)| node.author_id()
                        == Some(format!("handpick.bookmark.remove.pin-{index}").as_str())));
                }
                assert_eq!(host.notes.core.as_ref().unwrap().bookmarks().len(), count);
                assert!(host.notes.core.as_ref().unwrap().editor().is_none());
            }
        }
    }

    fn notes_with_live_surface(ctx: &egui::Context) -> MountedNotes {
        use crate::rich_editor::document_model::node::BlockNode;
        use crate::rich_editor::renderer::rich_editor_widget::RichEditorPaneFactory;
        let editor_id = handpick::EditorId::new("editor").unwrap();
        let editor = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph("text")]))));
        {
            use crate::rich_editor::document_model::{selection::Selection, position::DocPosition};
            let mut state = editor.lock().unwrap();
            state.selection = Selection::Text { anchor: DocPosition::new(vec![0, 0], 0),
                head: DocPosition::new(vec![0, 0], 4) };
            state.request_editor_focus();
        }
        let mut core = session();
        core.bind(editor_id.clone(), 0).unwrap();
        core.expand().unwrap();
        let factory = RichEditorPaneFactory::new(editor.clone());
        let mut surface = None;
        let _ = ctx.run(input(Vec::new()), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                surface = Some(factory.render_for_view(ui, Arc::from("view")));
            });
        });
        let mut notes = MountedNotes::default();
        notes.core = Some(core);
        notes.bus = Some(crate::interop::InteractionBus::get_or_init(ctx));
        notes.editors.insert(editor_id.clone(), NoteHandle { document_id: "note".into(), view_id: "view".into(),
            editor: editor.clone(), authority: editor });
        notes.surface = Some(MountedSurface { editor: editor_id, view_id: "view".into(), document_id: "note".into(),
            response: surface.unwrap() });
        notes
    }

    #[test]
    fn idle_search_metadata_keeps_large_compositions_unexported_until_explicit_dispatch() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        let base = "http://127.0.0.1:9911";
        let account = crate::local_account::mock_account_context(base);
        let active = notes.active().unwrap();
        let owner = active.editor.lock().unwrap().document_owner();
        owner.bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
        owner.edit(owner.current_context().unwrap(), |document, _| {
            *document = crate::rich_editor::document_model::node::BlockNode::doc(vec![
                crate::rich_editor::document_model::node::BlockNode::paragraph(&"x".repeat(4097))]);
            Ok::<_, ()>(())
        }).unwrap();
        active.editor.lock().unwrap().selection = crate::rich_editor::document_model::selection::Selection::caret(
            crate::rich_editor::document_model::position::DocPosition::new(vec![0, 0], 0));
        let expected = owner.current_context().unwrap();
        for _ in 0..3 {
            notes.prepare_search_button(&ctx, false);
            let input = notes.search_input.as_ref().unwrap().as_ref().expect("idle capture is metadata, not text admission");
            assert_eq!(input.context, expected);
            assert!(input.owner.same_owner(&owner));
        }
        assert_eq!(owner.current_context().unwrap(), expected);
        let error = notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).unwrap_err();
        assert!(error.contains("shorter phrase"), "the text limit belongs to explicit dispatch");
        assert_eq!(owner.current_context().unwrap(), expected);
    }

    #[test]
    fn stale_search_metadata_is_rejected_before_explicit_text_materialization() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        let base = "http://127.0.0.1:9911";
        let account = crate::local_account::mock_account_context(base);
        let active = notes.active().unwrap();
        let owner = active.editor.lock().unwrap().document_owner();
        owner.bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
        owner.edit(owner.current_context().unwrap(), |document, _| {
            *document = crate::rich_editor::document_model::node::BlockNode::doc(vec![
                crate::rich_editor::document_model::node::BlockNode::paragraph(&"x".repeat(4097))]);
            Ok::<_, ()>(())
        }).unwrap();
        active.editor.lock().unwrap().selection = crate::rich_editor::document_model::selection::Selection::caret(
            crate::rich_editor::document_model::position::DocPosition::new(vec![0, 0], 0));
        notes.prepare_search_button(&ctx, false);
        assert!(notes.search_input.as_ref().unwrap().is_ok());
        owner.advance_context().unwrap();
        let error = notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).unwrap_err();
        assert!(error.contains("selection or authority changed"));
        assert!(!error.contains("shorter phrase"));
    }

    #[test]
    fn keyboard_search_activation_materializes_retained_notes_metadata() {
        for key in [egui::Key::Space, egui::Key::Enter] {
            let ctx = egui::Context::default();
            let mut notes = notes_with_live_surface(&ctx);
            let base = "http://127.0.0.1:9911";
            let account = crate::local_account::mock_account_context(base);
            notes.active().unwrap().editor.lock().unwrap().document_owner()
                .bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
            draw_search_and_notes(&ctx, &mut notes, Vec::new());
            assert!(notes.search_input.as_ref().unwrap().is_ok());
            ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("handpick.search")));
            let clicked = draw_search_and_notes(&ctx, &mut notes, vec![egui::Event::Key {
                key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE,
            }]);
            assert!(clicked, "egui keyboard activation must retain the writing intent");
            assert_eq!(notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).unwrap(), Some("text".into()));
        }
    }

    #[test]
    fn accesskit_search_activation_materializes_current_notes_metadata() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        let base = "http://127.0.0.1:9911";
        let account = crate::local_account::mock_account_context(base);
        notes.active().unwrap().editor.lock().unwrap().document_owner()
            .bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
        draw_search_and_notes(&ctx, &mut notes, Vec::new());
        let clicked = draw_search_and_notes(&ctx, &mut notes,
            vec![egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Click,
                target: egui::Id::new("handpick.search").value().into(), data: None,
            })]);
        assert!(clicked, "the existing AccessKit Click route is an explicit search action");
        assert_eq!(notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).unwrap(), Some("text".into()));
    }

    #[test]
    fn explicit_search_uses_the_same_owner_selection_or_composition_without_editing_it() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        let base = "http://127.0.0.1:9911";
        let account = crate::local_account::mock_account_context(base);
        let active = notes.active().unwrap();
        let owner = active.editor.lock().unwrap().document_owner();
        owner.bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
        let before = owner.current_context().unwrap();
        let selection = active.editor.lock().unwrap().selection.clone();
        notes.prepare_search_button(&ctx, false);
        assert_eq!(notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).unwrap(), Some("text".into()));
        assert_eq!(owner.current_context().unwrap(), before);
        assert_eq!(active.editor.lock().unwrap().selection, selection);

        active.editor.lock().unwrap().selection = crate::rich_editor::document_model::selection::Selection::caret(crate::rich_editor::document_model::position::DocPosition::new(vec![0, 0], 0));
        notes.prepare_search_button(&ctx, false);
        assert_eq!(notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).unwrap(), Some("text".into()));
        assert_eq!(owner.current_context().unwrap(), before);
    }

    #[test]
    fn explicit_search_rejects_changed_selection_authority_ime_and_omitted_surface() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        let base = "http://127.0.0.1:9911";
        let account = crate::local_account::mock_account_context(base);
        let active = notes.active().unwrap();
        let owner = active.editor.lock().unwrap().document_owner();
        owner.bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
        notes.prepare_search_button(&ctx, false);
        active.editor.lock().unwrap().selection = crate::rich_editor::document_model::selection::Selection::caret(crate::rich_editor::document_model::position::DocPosition::new(vec![0, 0], 0));
        assert!(notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).is_err());

        notes.prepare_search_button(&ctx, false);
        let other = crate::local_account::mock_account_context(base);
        assert!(notes.take_search_button_input(&ctx, "workspace", Some(&other), base, false).is_err());
        notes.prepare_search_button(&ctx, false);
        assert!(notes.take_search_button_input(&ctx, "workspace", Some(&account), base, true).is_err());
        notes.prepare_search_button(&ctx, false);
        notes.surface = None;
        assert!(notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false).is_err());
    }

    fn draw_search_and_notes(ctx: &egui::Context, notes: &mut MountedNotes, events: Vec<egui::Event>) -> bool {
        use crate::rich_editor::renderer::rich_editor_widget::RichEditorPaneFactory;
        let active = notes.active().unwrap();
        let factory = RichEditorPaneFactory::new(active.editor);
        let mut clicked = false;
        let _ = ctx.run(input(events), |ctx| {
            notes.prepare_search_button(ctx, false);
            egui::TopBottomPanel::bottom("search-button-test").show(ctx, |ui| {
                clicked = super::super::fixed_id_button(ui, egui::Id::new("handpick.search"),
                    "handpick.search", "Search", "Search", egui::Color32::WHITE, egui::Color32::GRAY);
            });
            let response = egui::CentralPanel::default().show(ctx, |ui|
                factory.render_for_view(ui, Arc::from("view"))).inner;
            notes.surface.as_mut().unwrap().response = response;
        });
        clicked
    }

    #[test]
    fn actual_search_pointer_handoff_preserves_the_press_selection_and_rejects_later_changes() {
        for changed in [false, true] {
            let ctx = egui::Context::default();
            let mut notes = notes_with_live_surface(&ctx);
            let base = "http://127.0.0.1:9911";
            let account = crate::local_account::mock_account_context(base);
            let active = notes.active().unwrap();
            active.editor.lock().unwrap().document_owner()
                .bind_context("workspace", "note", base, None, Some(account.clone())).unwrap();
            draw_search_and_notes(&ctx, &mut notes, Vec::new());
            let pos = ctx.read_response(egui::Id::new("handpick.search")).unwrap().rect.center();
            assert!(!notes.surface.as_ref().unwrap().response.rect.contains(pos));
            assert!(notes.focused_binding().is_some());
            for pressed in [true, false] {
                if !pressed && changed {
                    active.editor.lock().unwrap().selection = crate::rich_editor::document_model::selection::Selection::caret(
                        crate::rich_editor::document_model::position::DocPosition::new(vec![0, 0], 0));
                }
                let clicked = draw_search_and_notes(&ctx, &mut notes,
                    vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {
                        pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
                    }]);
                if !pressed {
                    assert!(clicked);
                    let result = notes.take_search_button_input(&ctx, "workspace", Some(&account), base, false);
                    if changed { assert!(result.is_err()); }
                    else { assert_eq!(result.unwrap(), Some("text".into())); }
                }
            }
        }
    }

    #[test]
    fn query_focused_search_does_not_reuse_a_stale_notes_selection() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        notes.prepare_search_button(&ctx, false);
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("query")));
        notes.prepare_search_button(&ctx, false);
        assert!(notes.take_search_button_input(&ctx, "workspace", None, "base", false).unwrap().is_none());
    }

    #[test]
    fn stale_bus_owner_cannot_route_shortcuts_after_query_focus_or_compact() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        assert_eq!(notes.focused_binding(), Some(("view".into(), "note".into())));
        let mut query = "keep this query".to_owned();
        let _ = ctx.run(input(Vec::new()), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut query)).request_focus();
            });
        });
        assert_eq!(crate::interop::InteractionBus::with_try_lock(notes.bus.as_ref().unwrap(),
            |bus| bus.focus_owner().map(|id| id.to_string())), Some(Some("view".into())));
        assert!(notes.focused_binding().is_none());
        let _ = ctx.run(input(vec![enter(egui::Modifiers::CTRL), enter(egui::Modifiers::ALT)]), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                notes.controls(ui, "workspace", None, "http://127.0.0.1:9911", false);
                assert!(notes.take_selected_query(ctx, false).is_none(), "the old note must not replace the query");
                assert!(ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Enter)), "the old note must not consume Save");
                assert!(ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::Enter)), "the old note must not consume selected-text lookup");
                assert!(notes.error.is_none(), "no save attempt may reach the old authority");
            });
        });
        let id = notes.surface.as_ref().unwrap().response.id;
        ctx.memory_mut(|memory| memory.request_focus(id));
        notes.core.as_mut().unwrap().compact().unwrap();
        assert!(notes.focused_binding().is_none());
        assert_eq!(query, "keep this query");
    }

    #[test]
    fn first_frame_ime_fences_controls_and_explicit_save_before_owner_preedit_updates() {
        let ctx = egui::Context::default();
        let mut notes = notes_with_live_surface(&ctx);
        assert!(!notes.composing());
        let _ = ctx.run(input(vec![egui::Event::Ime(egui::ImeEvent::Preedit("preedit".into())),
            enter(egui::Modifiers::CTRL), enter(egui::Modifiers::ALT)]), |ctx| {
            let frame_composing = ctx.input(|input| input.events.iter().any(|event| matches!(event, egui::Event::Ime(_))));
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(!notes.composing(), "controls precede the actual editor render");
                notes.controls(ui, "workspace", None, "http://127.0.0.1:9911", frame_composing);
                assert!(notes.take_selected_query(ctx, frame_composing).is_none());
                assert!(ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Enter)));
                assert!(ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::Enter)));
                assert!(notes.error.is_none(), "the controls must not request save on the first IME frame");
                notes.request_save("workspace", None, "http://127.0.0.1:9911", frame_composing);
                assert_eq!(notes.error.as_deref(), Some("Finish the current text composition first"));
                assert!(!notes.composing(), "the frame fence works before persistent preedit is updated");
            });
        });
    }

    #[test]
    fn body_budget_never_allocates_the_native_minimum_when_it_does_not_fit() {
        for remaining in [-1.0, 0.0, 1.0, 64.0, 119.0] {
            assert_eq!(notes_body_height(500.0, remaining), None);
        }
        assert_eq!(notes_body_height(500.0, 120.0), Some(120.0));
        assert_eq!(notes_body_height(500.0, 150.0), Some(150.0));
        assert_eq!(notes_body_height(1000.0, 500.0), Some(240.0));
    }

    fn session() -> handpick::WritingSession {
        handpick::WritingSession::new(handpick::ContextId::new("account").unwrap(),
            handpick::SessionId::new("writing").unwrap(), 8)
    }

    #[test]
    fn expand_after_close_requests_the_current_note_again() {
        let mut notes = MountedNotes::default();
        notes.toggle();
        assert!(notes.requested);
        notes.requested = false;
        let mut core = session();
        core.bind_committed(handpick::EditorId::new("editor").unwrap(), 0,
            handpick::NoteId::new("note").unwrap(), handpick::RevisionId::new("1").unwrap()).unwrap();
        assert_eq!(core.close_quicknote(&core.fence().unwrap()).unwrap(), handpick::QuicknoteClose::Closed);
        notes.core = Some(core);
        notes.toggle();
        assert!(notes.requested);
        assert!(notes.core.as_ref().unwrap().editor().is_none());
    }

    #[test]
    fn notes_ime_state_remains_active_without_a_new_frame_event() {
        use crate::rich_editor::document_model::{node::BlockNode, position::DocPosition};
        let editor_id = handpick::EditorId::new("editor").unwrap();
        let mut core = session();
        core.bind(editor_id.clone(), 0).unwrap();
        let mut state = RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph("text")]));
        state.preedit.anchor = Some(DocPosition::new(vec![0, 0], 0));
        state.preedit.text = "preedit".into();
        let editor = Arc::new(Mutex::new(state));
        let mut notes = MountedNotes::default();
        notes.core = Some(core);
        notes.editors.insert(editor_id, NoteHandle { document_id: "note".into(), view_id: "view".into(),
            editor: editor.clone(), authority: editor.clone() });
        assert!(notes.composing());
        assert!(notes.composing(), "a quiet frame does not end the owning editor composition");
        editor.lock().unwrap().preedit.clear();
        assert!(!notes.composing());
    }
}

