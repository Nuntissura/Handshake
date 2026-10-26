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

#[derive(Default)]
pub(super) struct MountedNotes {
    core: Option<handpick::WritingSession>,
    editors: HashMap<handpick::EditorId, NoteHandle>,
    composition: Option<handpick::CompositionToken>,
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
        let active = self.active()?;
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
                    ui.button(format!("Bookmark {}", index + 1))
                })
                .inner;
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_author_id(format!("handpick.bookmark.{}", pin.id.as_str()));
            });
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
                    ui.add_enabled(can_remove, egui::Button::new("Remove"))
                }).inner.on_disabled_hover_text("Open or save this note before removing its bookmark; unsaved edits are retained");
                ui.ctx().accesskit_node_builder(remove.id, |node| {
                    node.set_author_id(format!("handpick.bookmark.remove.{}", pin.id.as_str()));
                    node.set_label(format!("Remove bookmark {}", index + 1));
                });
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

    pub(super) fn show(
        &mut self,
        ctx: &egui::Context,
        workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>,
        base: &str,
        current: Option<Arc<Mutex<RichEditorState>>>,
        store: &crate::editor_pane_factories::RichEditorDocumentStore,
        mount: &crate::editor_pane_factories::RichEditorPaneMount,
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
        let expanded = self.core.as_ref().is_some_and(|core| core.expanded());
        let bookmarked = self
            .core
            .as_ref()
            .is_some_and(|core| !core.pins().is_empty() || !core.bookmarks().is_empty());
        if !expanded && !bookmarked && self.error.is_none() {
            return;
        }
        egui::TopBottomPanel::top("handpick.notes.composition")
            .resizable(expanded)
            .default_height(if expanded { 240.0 } else { 32.0 })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(account) = account {
                        self.bookmarks(ui, workspace, base, account);
                    }
                    if let Some(state) = self.active() {
                        let response = ui.button("Bookmark");
                        ctx.accesskit_node_builder(response.id, |node| {
                            node.set_author_id("handpick.notes.bookmark")
                        });
                        if response.clicked() {
                            let result = self
                                .core
                                .as_mut()
                                .ok_or_else(|| "Notes session unavailable".to_owned())
                                .and_then(|core| {
                                    Self::bookmark_active(core)
                                });
                            self.observe(if result.is_ok() { crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted }
                                else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
                            if let Err(error) = result {
                                self.error = Some(error);
                            }
                        }
                        let composing = state
                            .editor
                            .lock()
                            .is_ok_and(|state| state.preedit.is_active());
                        let can_save = state
                            .authority
                            .lock()
                            .is_ok_and(|state| state.has_save_context())
                            && !composing;
                        if let Ok(authority) = state.authority.lock() {
                            if let Some(save) = &authority.save {
                                use crate::rich_editor::save::save_manager::SaveState;
                                ui.small(match &save.state {
                                    SaveState::Saving { .. } => "Saving note",
                                    SaveState::Conflict { .. } | SaveState::ConfirmKeepYours { .. } => "Save conflict; open the note to resolve it",
                                    SaveState::Error(_) => "Save failed; open the note for recovery",
                                    SaveState::Idle => if authority.document_owner().with_save_gate(|gate| gate.is_dirty()) { "Unsaved changes" } else { "Saved note" },
                                });
                            }
                        }
                        let save = ui.add_enabled(can_save, egui::Button::new("Save note"));
                        ctx.accesskit_node_builder(save.id, |node| {
                            node.set_author_id("handpick.notes.save")
                        });
                        if save.clicked() {
                            if let Ok(mut state) = state.authority.lock() {
                                if !account.is_some_and(|account| {
                                    state
                                        .document_owner()
                                        .has_active_binding(workspace, base, account)
                                }) {
                                    self.error = Some(
                                        "Notes authority changed; save was not dispatched".into(),
                                    );
                                } else if !state.request_save_for_host() {
                                    self.error =
                                        Some("Notes save is unavailable; edits retained".into());
                                }
                            }
                        }
                        let close = ui.button("Close writing");
                        ctx.accesskit_node_builder(close.id, |node| {
                            node.set_author_id("handpick.notes.close")
                        });
                        if close.clicked() {
                            let result = self
                                .core
                                .as_mut()
                                .ok_or_else(|| "Notes session unavailable".to_owned())
                                .and_then(|core| {
                                    core.close_quicknote(
                                        &core.fence().map_err(|error| error.to_string())?,
                                    )
                                    .map_err(|error| error.to_string())
                                });
                            self.observe(if matches!(result, Ok(handpick::QuicknoteClose::Closed)) {
                                crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted
                            } else { crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected });
                            match result {
                                Ok(handpick::QuicknoteClose::Closed) => {}
                                Ok(handpick::QuicknoteClose::NeedsSaveOrPin) => {
                                    self.error = Some(
                                        "Save or bookmark the current edit before closing".into(),
                                    )
                                }
                                Err(error) => self.error = Some(error),
                            }
                        }
                    }
                });
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    let dismiss = ui.button("Dismiss message");
                    ctx.accesskit_node_builder(dismiss.id, |node| {
                        node.set_author_id("handpick.notes.dismiss")
                    });
                    if dismiss.clicked() {
                        self.error = None;
                    }
                }
                if self.core.as_ref().is_some_and(|core| core.expanded()) {
                    if let Some(state) = self.active() {
                        ui.push_id(&state.view_id, |ui| {
                            let _ = mount.render_view(ui, Some(&state.document_id), &state.view_id);
                        });
                    }
                }
            });
        if let Err(error) = self.synchronize() {
            self.error = Some(error);
        }
        self.prune_released();
    }
}
