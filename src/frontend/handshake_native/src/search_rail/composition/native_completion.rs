//! Completion rendering and event wiring over the Notes owner and Handpick plans.
use super::*;
use super::completion_provider::{Material, Page, Provider, Row};
use crate::rich_editor::renderer::rich_editor_widget::handpick_completion::NotesCompletionSnapshot;
use handpick::{CompletionInvocation, NativeCompletionPlan, NativeCompletionTrigger};

enum Delivery { Lookup(Result<Page, String>), Resolved(Result<Material, String>) }
type Location = (handpick::EditorId, crate::rich_editor::document_owner::EditorSaveContext,
    crate::rich_editor::document_model::selection::Selection);

#[derive(Default)]
pub(super) struct NativeCompletion {
    observed: Option<Location>,
    lease: Option<TemplatePopupLease>,
    inline_lease: Option<InlinePopupLease>,
    snapshot: Option<NotesCompletionSnapshot>,
    trigger: Option<NativeCompletionTrigger>,
    provider: Option<Provider>,
    rows: Vec<Row>,
    selected: usize,
    reveal_selection: bool,
    plan: Option<NativeCompletionPlan>,
    task: Option<tokio::task::JoinHandle<()>>,
    cell: Arc<Mutex<Option<Delivery>>>,
    status: Option<handpick::DeliveryStatus>,
    #[cfg(test)]
    after_apply: Option<fn(&mut MountedNotes)>,
}

struct TemplatePopupLease(std::sync::Weak<Mutex<RichEditorState>>);

impl TemplatePopupLease {
    fn acquire(editor: &Arc<Mutex<RichEditorState>>, snapshot: &NotesCompletionSnapshot,
        fence: handpick::WritingFence) -> Result<Self, String> {
        let mut state = editor.lock().map_err(|_| "Notes editor unavailable")?;
        snapshot.current(&state, fence)?;
        state.begin_handpick_template()?;
        Ok(Self(Arc::downgrade(editor)))
    }
}

impl Drop for TemplatePopupLease {
    fn drop(&mut self) {
        if let Some(editor) = self.0.upgrade() {
            if let Ok(mut state) = editor.lock() { state.end_handpick_template(); }
        }
    }
}

struct InlinePopupLease(std::sync::Weak<Mutex<RichEditorState>>);

impl InlinePopupLease {
    fn acquire(editor: &Arc<Mutex<RichEditorState>>, snapshot: &NotesCompletionSnapshot,
        fence: handpick::WritingFence) -> Result<Self, String> {
        let mut state = editor.lock().map_err(|_| "Notes editor unavailable")?;
        snapshot.current(&state, fence)?;
        state.begin_handpick_inline()?;
        Ok(Self(Arc::downgrade(editor)))
    }
}

impl Drop for InlinePopupLease {
    fn drop(&mut self) {
        if let Some(editor) = self.0.upgrade() {
            if let Ok(mut state) = editor.lock() { state.end_handpick_inline(); }
        }
    }
}

impl Drop for NativeCompletion {
    fn drop(&mut self) { if let Some(task) = self.task.take() { task.abort(); } }
}

impl NativeCompletion {
    fn close(&mut self) {
        let observed = self.observed.take();
        *self = Self::default();
        self.observed = observed;
    }
    pub(super) fn is_open(&self) -> bool { self.trigger.is_some() }
}

impl MountedNotes {
    fn fail_completion(&mut self, error: String) {
        self.completion.close();
        self.error = Some(error);
    }

    fn completion_location(&self) -> Option<Location> {
        let handle = self.active()?;
        let state = handle.editor.lock().ok()?;
        let editor = self.core.as_ref()?.editor()?.0.clone();
        Some((editor, state.document_owner().current_context().ok()?, state.selection.clone()))
    }

    fn completion_owner(&self) -> Result<NoteHandle, String> {
        let provider = self.completion.provider.as_ref().ok_or("Completion provider unavailable")?;
        self.completion_owner_for(provider)
    }

    fn completion_owner_for(&self, provider: &Provider) -> Result<NoteHandle, String> {
        let handle = self.active().ok_or("Notes editor unavailable")?;
        let owner = handle.editor.lock().map_err(|_| "Notes editor unavailable")?.document_owner();
        let authority = handle.authority.lock().map_err(|_| "Notes authority unavailable")?;
        if !owner.same_owner(&authority.document_owner())
            || !owner.has_active_binding(&provider.workspace, &provider.base, &provider.account)
            || authority.save.as_ref().is_none_or(|save| save.is_saving() || save.requires_recovery())
            || authority.pending_stage_embed_save.is_some() {
            return Err("Notes authority or save state changed; writing retained".into());
        }
        drop(authority);
        Ok(handle)
    }

    pub(in crate::search_rail) fn completion_before_input(&mut self, ctx: &egui::Context, workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>, base: &str,
        runtime: Option<&tokio::runtime::Handle>, composing: bool) {
        if composing || self.composing() || !self.expanded() {
            if self.completion.is_open() {
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Stale);
            }
            self.completion.close();
            self.completion.observed = None;
            return;
        }
        if self.completion.snapshot.is_some() && !self.completion_is_current() {
            self.completion.close();
            self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Stale);
        }
        self.drain_completion(ctx);
        if self.completion.is_open()
            && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.completion.close();
            self.focus_editor();
            self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Completed);
            return;
        }
        if self.focused_binding().is_none() { return; }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::Space)) {
            self.start_completion(ctx, workspace, account, base, runtime, CompletionInvocation::Manual);
            return;
        }
        if !self.completion.is_open() { return; }
        if self.completion.plan.is_some() { return; }
        let length = self.completion.rows.len();
        if length > 0 {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)) {
                ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                self.completion.selected = (self.completion.selected + 1) % length;
                self.completion.reveal_selection = true;
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted);
            }
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)) {
                ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
                self.completion.selected = (self.completion.selected + length - 1) % length;
                self.completion.reveal_selection = true;
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Accepted);
            }
        }
        if length > 0 && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
            || input.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
            ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
            self.accept_completion(ctx);
        }
    }

    fn completion_is_current(&self) -> bool {
        (|| {
            let handle = self.completion_owner().ok()?;
            let state = handle.editor.lock().ok()?;
            if state.slash_menu_opened_explicitly
                || state.slash_menu.as_ref().is_some_and(|menu| menu.prompt_active()) { return None; }
            let snapshot = self.completion.snapshot.as_ref()?;
            let fence = self.core.as_ref()?.fence().ok()?;
            (snapshot.current(&state, fence).ok()?.eq(snapshot.native())).then_some(())
        })().is_some()
    }

    pub(in crate::search_rail) fn completion_after_input(&mut self, ctx: &egui::Context, workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>, base: &str,
        runtime: Option<&tokio::runtime::Handle>, composing: bool) {
        if composing || self.composing() || self.focused_binding().is_none() { return; }
        let location = self.completion_location();
        if self.completion.is_open() && location.is_some() && location != self.completion.observed {
            self.start_completion(ctx, workspace, account, base, runtime, CompletionInvocation::Manual);
        }
    }

    fn start_completion(&mut self, ctx: &egui::Context, workspace: &str,
        account: Option<&Arc<crate::local_account::AuthenticatedContext>>, base: &str,
        runtime: Option<&tokio::runtime::Handle>, invocation: CompletionInvocation) {
        self.completion.close();
        self.completion.observed = self.completion_location();
        let result = (|| {
            let account = account.filter(|account| account.is_active()).ok_or("Completion authorization unavailable")?;
            let runtime = runtime.ok_or("Notes runtime unavailable")?;
            let provider = Provider { workspace: workspace.into(), account: account.clone(), base: base.into(), runtime: runtime.clone() };
            let handle = self.completion_owner_for(&provider)?;
            let state = handle.editor.lock().map_err(|_| "Notes editor unavailable")?;
            if state.preedit.anchor.is_some() || state.slash_menu_opened_explicitly { return Ok(None); }
            let core = self.core.as_mut().ok_or("Notes session unavailable")?;
            let id = handpick::CompletionSnapshotId::new(uuid::Uuid::new_v4().to_string()).map_err(str::to_owned)?;
            let trial = state.capture_handpick_completion(core.fence().map_err(|error| error.to_string())?, id.clone())?;
            let Some(trigger) = core.native_completion_trigger(trial.native(), invocation)
                .map_err(|error| error.to_string())? else { return Ok(None) };
            let template = matches!(trigger.lookup(), handpick::NativeCompletionLookup::Templates { .. });
            let fence = core.begin_lookup().map_err(|error| error.to_string())?;
            let snapshot = state.capture_handpick_completion(fence, id)?;
            let trigger = core.native_completion_trigger(snapshot.native(), invocation)
                .map_err(|error| error.to_string())?.ok_or("Completion trigger changed")?;
            let declaration = provider.declaration()?;
            let query = trigger.composition_query(declaration.provider.clone(), super::completion_provider::PAGE_SIZE, None);
            declaration.validate_native_completion_query(core, &trigger, &query).map_err(|error| error.to_string())?;
            core.deliver(trigger.snapshot().fence(), handpick::Channel::Suggestions, handpick::DeliveryStatus::Pending, Vec::new())
                .map_err(|error| error.to_string())?;
            drop(state);
            let fence = core.fence().map_err(|error| error.to_string())?;
            let (lease, inline_lease) = if template {
                (Some(TemplatePopupLease::acquire(&handle.editor, &snapshot, fence)?), None)
            } else {
                (None, Some(InlinePopupLease::acquire(&handle.editor, &snapshot, fence)?))
            };
            Ok::<_, String>(Some((snapshot, trigger, provider, lease, inline_lease)))
        })();
        match result {
            Ok(Some((snapshot, trigger, provider, lease, inline_lease))) => {
                self.completion.lease = lease;
                self.completion.inline_lease = inline_lease;
                self.completion.snapshot = Some(snapshot);
                self.completion.trigger = Some(trigger.clone());
                self.completion.provider = Some(provider.clone());
                self.completion.status = Some(handpick::DeliveryStatus::Pending);
                let cell = self.completion.cell.clone();
                let repaint = ctx.clone();
                self.completion.task = Some(provider.runtime.clone().spawn(async move {
                    let result = provider.lookup(&trigger).await;
                    if let Ok(mut slot) = cell.lock() { *slot = Some(Delivery::Lookup(result)); }
                    repaint.request_repaint();
                }));
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Pending);
            }
            Ok(None) => {}
            Err(error) => {
                if invocation == CompletionInvocation::Manual {
                    self.fail_completion(error);
                    self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected);
                }
            }
        }
    }

    fn accept_completion(&mut self, ctx: &egui::Context) {
        if self.completion.task.is_some() { return; }
        let result = (|| {
            let handle = self.completion_owner()?;
            let state = handle.editor.lock().map_err(|_| "Notes editor unavailable")?;
            let row = self.completion.rows.get(self.completion.selected).cloned().ok_or("Choose an available completion")?;
            let snapshot = self.completion.snapshot.as_ref().ok_or("Completion snapshot unavailable")?;
            let trigger = self.completion.trigger.as_ref().ok_or("Completion trigger unavailable")?;
            let core = self.core.as_mut().ok_or("Notes session unavailable")?;
            let current = snapshot.current(&state, core.fence().map_err(|error| error.to_string())?)?;
            core.select(trigger.snapshot().fence(), row.item.key.clone()).map_err(|error| error.to_string())?;
            let plan = core.plan_native_completion(trigger, &current, row.choice.clone()).map_err(|error| error.to_string())?;
            let provider = self.completion.provider.clone().ok_or("Completion provider unavailable")?;
            Ok::<_, String>((row, plan, provider))
        })();
        match result {
            Ok((row, plan, provider)) => {
                self.completion.plan = Some(plan.clone());
                let cell = self.completion.cell.clone();
                let repaint = ctx.clone();
                self.completion.task = Some(provider.runtime.clone().spawn(async move {
                    let resolved = provider.resolve(&row, &plan).await;
                    if let Ok(mut slot) = cell.lock() { *slot = Some(Delivery::Resolved(resolved)); }
                    repaint.request_repaint();
                }));
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Pending);
            }
            Err(error) => {
                self.fail_completion(error);
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected);
            }
        }
    }

    fn drain_completion(&mut self, ctx: &egui::Context) {
        let delivery = self.completion.cell.lock().ok().and_then(|mut slot| slot.take());
        let Some(delivery) = delivery else { return };
        self.completion.task = None;
        if !self.completion_is_current() {
            self.completion.close();
            self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Stale);
            return;
        }
        match delivery {
            Delivery::Lookup(Ok(page)) => {
                let result = self.core.as_mut().ok_or_else(|| "Notes session unavailable".to_owned())
                    .and_then(|core| core.deliver(self.completion.trigger.as_ref().unwrap().snapshot().fence(),
                        handpick::Channel::Suggestions, page.status, page.rows.iter().map(|row| row.item.clone()).collect())
                        .map_err(|error| error.to_string()));
                if let Err(error) = result {
                    self.fail_completion(error);
                    self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected);
                    return;
                }
                self.completion.rows = page.rows;
                self.completion.reveal_selection = true;
                self.completion.status = Some(page.status);
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Completed);
            }
            Delivery::Lookup(Err(error)) | Delivery::Resolved(Err(error)) => {
                self.fail_completion(error);
                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Failed);
            }
            Delivery::Resolved(Ok(material)) => {
                let result = (|| {
                    let handle = self.completion_owner()?;
                    let mut state = handle.editor.lock().map_err(|_| "Notes editor unavailable")?;
                    let snapshot = self.completion.snapshot.as_ref().ok_or("Completion snapshot unavailable")?;
                    let plan = self.completion.plan.as_ref().ok_or("Completion plan unavailable")?;
                    let core = self.core.as_ref().ok_or("Notes session unavailable")?;
                    match material {
                        Material::Template(template) => state.apply_handpick_template(snapshot, core, plan, template),
                        Material::Inline(child) => state.apply_handpick_inline(snapshot, core, plan, child),
                        Material::BlockEmbed { note_id, target, label } =>
                            state.apply_handpick_block_embed(snapshot, core, plan, &note_id, &target, &label),
                        Material::NoteEmbed { note_id, block_id } =>
                            state.apply_handpick_note_embed(snapshot, core, plan, &note_id, &block_id),
                    }
                })();
                match result {
                    Ok(()) => {
                        #[cfg(test)]
                        if let Some(after_apply) = self.completion.after_apply.take() { after_apply(self); }
                        self.completion.close();
                        match self.synchronize() {
                            Ok(()) => self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Completed),
                            Err(error) => {
                                self.error = Some(error);
                                self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Failed);
                            }
                        }
                        self.completion.observed = self.completion_location();
                    }
                    Err(error) => {
                        self.fail_completion(error);
                        self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Rejected);
                    }
                }
                ctx.request_repaint();
            }
        }
    }

    pub(super) fn render_completion(&mut self, ui: &mut egui::Ui) {
        if !self.completion.is_open() { return; }
        let Some(surface) = self.surface.as_ref() else { return };
        let viewport = ui.ctx().content_rect().shrink(8.0);
        let width = surface.response.rect.width().min(420.0).min(viewport.width());
        let anchor = surface.response.rect.left_bottom();
        let mut accept = None;
        let mut dismiss = false;
        egui::Area::new(egui::Id::new("handpick.native-completion"))
            .order(egui::Order::Foreground).sense(egui::Sense::hover()).fixed_pos(anchor).constrain_to(viewport)
            .show(ui.ctx(), |ui| egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_width(width);
            egui::ScrollArea::vertical().id_salt("results").max_height(viewport.height().min(220.0)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong("Complete");
                let close = ui.small_button("Close");
                ui.ctx().accesskit_node_builder(close.id, |node| node.set_author_id("handpick.completion.close"));
                dismiss = close.clicked();
            });
            if self.completion.task.is_some() {
                completion_pending(ui, self.completion.plan.is_some());
            }
            if self.completion.status == Some(handpick::DeliveryStatus::Partial) { ui.small("Showing the first matches. Type more to narrow the list."); }
            if self.completion.status == Some(handpick::DeliveryStatus::Complete) && self.completion.rows.is_empty() { ui.small("No matching completion"); }
                for (index, row) in self.completion.rows.iter().enumerate() {
                    let response = ui.push_id((row.item.key.provider.as_str(), row.item.key.result.as_str()), |ui|
                        ui.add_enabled(self.completion.task.is_none(), egui::Button::new(&row.item.label)
                            .selected(index == self.completion.selected))).inner;
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_author_id(format!("handpick.completion.{}:{}", row.item.key.provider.as_str(), row.item.key.result.as_str()));
                        node.set_selected(index == self.completion.selected);
                    });
                    if index == self.completion.selected && self.completion.reveal_selection {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                    if response.clicked() { accept = Some(index); }
                }
            });
        }));
        self.completion.reveal_selection = false;
        if dismiss {
            self.completion.close();
            self.focus_editor();
            self.observe(crate::diagnostics::gui_events::GuiDiagnosticPhase::Completed);
        }
        if let Some(index) = accept { self.completion.selected = index; self.accept_completion(ui.ctx()); }
    }
}

fn completion_pending(ui: &mut egui::Ui, resolving: bool) -> egui::Response {
    let status = if resolving { "Preparing completion…" } else { "Finding completions…" };
    ui.horizontal(|ui| {
        let size = ui.spacing().interact_size.y;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let mark = if motion_preference::is_reduced_motion(ui.ctx()) {
            ui.place(rect, egui::Label::new("…").selectable(false).sense(egui::Sense::hover()))
        } else {
            ui.place(rect, egui::Spinner::new().size(size))
        };
        mark.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ProgressIndicator, ui.is_enabled(), status));
        ui.ctx().accesskit_node_builder(mark.id, |node| {
            node.set_author_id("handpick.completion.pending");
            node.set_label(status);
        });
        ui.add(egui::Label::new(status).truncate().selectable(false));
        mark
    }).inner
}

#[cfg(test)]
mod tests;
