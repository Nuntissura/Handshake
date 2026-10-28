//! Handshake tree presentation and asynchronous request custody over the pinned Cabinet core.

use cabinet::{Cabinet, Completion, Context, Entry, Operation, Request, Snapshot};
use cabinet::interaction::InteractionState;

#[derive(Default)]
pub struct State {
    model: Option<Cabinet>,
    interaction: InteractionState,
    revision: u64,
    source_revision: u64,
    host_request: Option<Request>,
    binding: Option<(u64, u64)>,
    terminal: Option<Result<(), String>>,
    canonical_ready: bool,
    focus_target: Option<String>,
    scroll_target: Option<String>,
    navigation_changes: Vec<(String, bool)>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CabinetHostState")
            .field("context", &self.model.as_ref().map(|m| &m.snapshot().context))
            .field("binding", &self.binding)
            .finish_non_exhaustive()
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        // A cloned host projection never acquires an in-flight operation's ownership.
        Self::default()
    }
}

impl State {
    pub fn model(&self) -> Option<&Cabinet> { self.model.as_ref() }

    pub fn select(&mut self, id: &str, modifiers: cabinet::SelectionModifiers) -> Result<(), String> {
        self.model.as_mut().ok_or("Tree is not loaded")?.select(id, modifiers).map_err(|error| error.to_string())
    }

    pub fn projection_expand(&mut self, id: &str, expanded: bool) -> Result<(), String> {
        self.model.as_mut().ok_or("Tree is not loaded")?.expand(id, expanded).map_err(|error| error.to_string())
    }

    pub fn begin_create(&mut self, parent: Option<cabinet::EntryId>) {
        self.interaction.begin_create(parent);
    }

    pub fn begin_rename(&mut self, id: cabinet::EntryId, text: Option<String>) -> Result<(), String> {
        let model = self.model.as_ref().ok_or("Tree is not loaded")?;
        if model.entry(&id).is_none() { return Err("Rename target is no longer in the tree".to_owned()); }
        self.interaction.sync(model);
        self.interaction.begin_rename(model, id);
        if let Some(text) = text { self.interaction.text = text; }
        Ok(())
    }

    pub fn prepare_rename(&mut self, id: &str, title: &str) -> Result<Option<Request>, String> {
        self.begin_rename(id.to_owned(), Some(title.to_owned()))?;
        self.submit_edit()
    }

    pub fn load_children_and_expand(&mut self, id: &str) -> Result<Request, String> {
        let was_expanded = self.model.as_ref().ok_or("Tree is not loaded")?.expanded(id);
        self.projection_expand(id, true)?;
        match self.request(Operation::LoadChildren { id: id.to_owned() }) {
            Ok(request) => Ok(request),
            Err(error) => {
                let _ = self.projection_expand(id, was_expanded);
                Err(error)
            }
        }
    }

    pub fn edit(&self) -> Option<&cabinet::interaction::Edit> { self.interaction.edit.as_ref() }
    pub fn edit_text(&self) -> &str { &self.interaction.text }
    pub fn edit_text_mut(&mut self) -> &mut String { &mut self.interaction.text }
    pub fn cancel_edit(&mut self) { self.interaction.cancel(); }
    pub fn last_error(&self) -> Option<&str> { self.interaction.last_error.as_deref() }
    pub fn set_error(&mut self, message: String) { self.interaction.last_error = Some(message); }

    pub fn source_is_current(&mut self, scope: &str, generation: u64, revision: u64) -> bool {
        let Some(model) = &self.model else { return false; };
        let current = model.snapshot().context.scope == scope
            && model.snapshot().context.generation == generation
            && self.source_revision == revision;
        if current { self.interaction.sync(model); }
        current
    }

    pub fn sync(
        &mut self,
        scope: String,
        generation: u64,
        source_revision: u64,
        mut entries: Vec<Entry>,
        hierarchy_complete: bool,
    ) -> Result<(), String> {
        let context_changed = self.model.as_ref().is_some_and(|model| {
            model.snapshot().context.scope != scope
                || model.snapshot().context.generation != generation
        });
        if context_changed {
            *self = Self::default();
        }
        let changed = self.model.as_ref().is_none_or(|model| {
            let mut previous = model.snapshot().entries.clone();
            for entry in &mut previous { entry.revision = 0; }
            previous != entries
                || model.snapshot().hierarchy_complete != hierarchy_complete
                || self.source_revision != source_revision
        });
        if !changed {
            if let Some(model) = &self.model { self.interaction.sync(model); }
            return Ok(());
        }
        self.revision = self.revision.checked_add(1).ok_or("Tree revision exhausted")?;
        for entry in &mut entries { entry.revision = self.revision; }
        let snapshot = Snapshot {
            context: Context { scope, generation, revision: self.revision },
            entries,
            hierarchy_complete,
        };
        match self.model.as_mut() {
            None => self.model = Some(Cabinet::new(snapshot).map_err(|e| e.to_string())?),
            Some(model) => {
                if self.host_request.is_some() && self.terminal.is_some() && self.canonical_ready {
                    let request = self.host_request.as_ref().ok_or("Tree operation identity missing")?;
                    let still_pending = model.pending().is_some_and(|pending| pending.id == request.id && pending.context == request.context);
                    if still_pending {
                    match self.terminal.clone() {
                        Some(Ok(())) => model.complete(Completion { request_id: request.id, context: request.context.clone(), result: Ok(snapshot), undo_token: None }).map_err(|e| e.to_string())?,
                        Some(Err(error)) => {
                            model.complete(Completion { request_id: request.id, context: request.context.clone(), result: Err(error), undo_token: None }).map_err(|e| e.to_string())?;
                            model.replace(snapshot).map_err(|e| e.to_string())?;
                        }
                        None => return Err("Tree operation has no host terminal".to_owned()),
                    }
                    } else {
                        // A prior canonical update already invalidated Cabinet's local waiting.
                        // The host still owns the dispatched operation until this exact terminal
                        // and its subsequent canonical reread resolve it.
                        model.replace(snapshot).map_err(|e| e.to_string())?;
                    }
                    self.host_request = None;
                    self.binding = None;
                    self.terminal = None;
                    self.canonical_ready = false;
                } else {
                    model.replace(snapshot).map_err(|e| e.to_string())?;
                }
            }
        }
        self.source_revision = source_revision;
        if let Some(model) = &self.model { self.interaction.sync(model); }
        Ok(())
    }

    pub fn request(&mut self, operation: Operation) -> Result<Request, String> {
        let read = matches!(&operation, Operation::Open { .. } | Operation::LoadChildren { .. } | Operation::LoadPage { .. });
        if self.host_request.as_ref().is_some_and(|previous| !read || !matches!(previous.operation, Operation::Open { .. } | Operation::LoadChildren { .. } | Operation::LoadPage { .. })) {
            return Err("Wait for the current tree operation to be reconciled".to_owned());
        }
        let model = self.model.as_mut().ok_or("Tree is not loaded")?;
        self.interaction.sync(model);
        if read {
            if let Some(previous) = model.pending().filter(|request| matches!(request.operation, Operation::Open { .. } | Operation::LoadChildren { .. } | Operation::LoadPage { .. })).cloned() {
                model.abandon_pending(previous.id).map_err(|error| error.to_string())?;
            }
        }
        // A failed replacement retains the actual dispatched read's host identity and terminal.
        let request = self.interaction.request(model, operation).map_err(|error| error.to_string())?;
        self.host_request = Some(request.clone());
        self.binding = None;
        self.terminal = None;
        self.canonical_ready = false;
        Ok(request)
    }

    pub fn bind(&mut self, epoch: u64, sequence: u64) -> bool {
        let Some(request) = &self.host_request else { return true; };
        if !self.model.as_ref().is_some_and(|model| model.snapshot().context == request.context && request.context.generation == epoch) {
            self.fail_unstarted("Tree changed before the operation was dispatched; retry from current data".to_owned());
            return false;
        }
        self.binding = Some((epoch, sequence));
        true
    }

    pub fn navigation_available(&self) -> bool {
        self.host_request.as_ref().is_none_or(|request| matches!(request.operation, Operation::Open { .. } | Operation::LoadChildren { .. } | Operation::LoadPage { .. }))
    }

    pub fn expand(&mut self, id: &str, expanded: bool) -> Result<(), String> {
        if expanded && !self.navigation_available() {
            return Err("Wait for the current tree operation to finish".to_owned());
        }
        let model = self.model.as_mut().ok_or("Tree is not loaded")?;
        let was_expanded = model.expanded(id);
        if expanded {
            if let Some(previous) = model.pending().filter(|request| matches!(request.operation, Operation::Open { .. } | Operation::LoadChildren { .. } | Operation::LoadPage { .. })).cloned() {
                model.abandon_pending(previous.id).map_err(|error| error.to_string())?;
            }
        }
        self.interaction.sync(model);
        match self.interaction.expand(model, id, expanded) {
            Ok(Some(request)) => {
                self.host_request = Some(request);
                self.binding = None;
                self.terminal = None;
                self.canonical_ready = false;
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(error) => {
                let _ = model.expand(id, was_expanded);
                Err(error.to_string())
            }
        }
    }

    pub fn canonical_refetch(&mut self) {
        if self.terminal.is_some() { self.canonical_ready = true; }
    }

    pub fn bound_operation(&self, epoch: u64, sequence: u64) -> Option<&Operation> {
        if self.binding != Some((epoch, sequence)) { return None; }
        self.host_request.as_ref().filter(|request| request.context.generation == epoch).map(|request| &request.operation)
    }

    pub fn host_operation(&self) -> Option<&Operation> { self.host_request.as_ref().map(|request| &request.operation) }

    pub fn terminal(&mut self, epoch: u64, sequence: u64, result: Result<(), String>) -> bool {
        if self.binding != Some((epoch, sequence)) { return false; }
        let Some(model) = &mut self.model else { return false; };
        let Some(request) = self.host_request.clone() else { return false; };
        match result {
            Err(message) if matches!(request.operation, Operation::Open { .. } | Operation::LoadChildren { .. } | Operation::LoadPage { .. }) => {
                let result = if model.pending().is_some_and(|pending| pending.id == request.id && pending.context == request.context) { model.complete(Completion {
                    request_id: request.id,
                    context: request.context,
                    result: Err(message),
                    undo_token: None,
                }) } else { Ok(()) };
                if result.is_err() { return false; }
                self.binding = None;
                self.host_request = None;
                self.terminal = None;
                self.canonical_ready = false;
            }
            result => self.terminal = Some(result),
        }
        true
    }

    pub fn acknowledge_read(&mut self) {
        let Some(model) = &mut self.model else { return; };
        let Some(request) = self.host_request.clone() else { return; };
        if !matches!(request.operation, Operation::Open { .. }) { return; }
        if model.pending().is_none() || model.acknowledge(request.id, &request.context).is_ok() {
            self.host_request = None;
            self.binding = None;
            self.terminal = None;
        }
    }

    pub fn fail_unstarted(&mut self, message: String) {
        if self.binding.is_some() { return; }
        let Some(model) = &mut self.model else { return; };
        if let Some(request) = model.pending().cloned() {
            let _ = model.complete(Completion { request_id: request.id, context: request.context, result: Err(message.clone()), undo_token: None });
        }
        self.host_request = None;
        self.interaction.last_error = Some(message);
    }

    pub fn bound_unstarted(&mut self, epoch: u64, sequence: u64, message: String) {
        if self.binding != Some((epoch, sequence)) { return; }
        self.binding = None;
        self.fail_unstarted(message);
        self.terminal = None;
        self.canonical_ready = false;
    }

    pub fn submit_edit(&mut self) -> Result<Option<Request>, String> {
        if self.host_request.is_some() { return Err("Wait for the current tree operation to be reconciled".to_owned()); }
        let model = self.model.as_mut().ok_or("Tree is not loaded")?;
        self.interaction.sync(model);
        let request = self.interaction.submit_edit(model).map_err(|e| e.to_string())?;
        if let Some(request) = &request { self.host_request = Some(request.clone()); self.binding = None; self.terminal = None; self.canonical_ready = false; }
        Ok(request)
    }

    pub fn row_navigation(&mut self, ui: &egui::Ui, response: &egui::Response, id: &str) {
        if let Some(model) = &self.model { ui.ctx().accesskit_node_builder(response.id, |node| node.set_selected(model.is_selected(id))); }
        if self.scroll_target.as_deref() == Some(id) {
            response.scroll_to_me(None);
            self.scroll_target = None;
        }
        if self.focus_target.as_deref() == Some(id) {
            response.request_focus();
            response.scroll_to_me(None);
            self.focus_target = None;
        }
        if !response.has_focus() || self.interaction.edit.is_some() || !self.navigation_available() { return; }
        let modifiers = ui.input(|input| input.modifiers);
        for (key, direction) in [
            (egui::Key::ArrowUp, cabinet::Navigation::Up),
            (egui::Key::ArrowDown, cabinet::Navigation::Down),
            (egui::Key::ArrowLeft, cabinet::Navigation::Left),
            (egui::Key::ArrowRight, cabinet::Navigation::Right),
            (egui::Key::Home, cabinet::Navigation::Home),
            (egui::Key::End, cabinet::Navigation::End),
        ] {
            if ui.input_mut(|input| input.consume_key(modifiers, key)) {
                let selection = cabinet::SelectionModifiers { toggle: modifiers.command, range: modifiers.shift };
                if let Some(model) = &mut self.model {
                    if model.focused() != Some(id) { let _ = model.select(id, cabinet::SelectionModifiers::default()); }
                }
                let before = self.model.as_ref().and_then(|model| model.entry(id).filter(|entry| entry.kind == cabinet::EntryKind::Folder).map(|_| model.expanded(id)));
                let expanding = direction == cabinet::Navigation::Right && before == Some(false);
                let navigated = if expanding {
                    // InteractionState owns lazy-load request creation; Cabinet::navigate alone
                    // changes expansion without producing the host's required read intention.
                    self.expand(id, true).and_then(|()| self.model.as_mut().ok_or("Tree is not loaded")?.select(id, selection).map_err(|error| error.to_string()))
                } else {
                    self.model.as_mut().ok_or_else(|| "Tree is not loaded".to_owned()).and_then(|model| model.navigate(direction, selection).map_err(|error| error.to_string()))
                };
                if navigated.is_ok() {
                    if let Some(model) = &mut self.model {
                        if before.is_some_and(|expanded| expanded != model.expanded(id)) { self.navigation_changes.push((id.to_owned(), model.expanded(id))); }
                        self.focus_target = model.focused().map(str::to_owned);
                    }
                    ui.ctx().request_repaint();
                } else if let Err(error) = navigated { self.interaction.last_error = Some(error); }
                break;
            }
        }
    }

    pub fn take_navigation_changes(&mut self) -> Vec<(String, bool)> { std::mem::take(&mut self.navigation_changes) }

    pub fn select_for_open(&mut self, ui: &egui::Ui, response: &egui::Response, id: &str) -> bool {
        let modifiers = ui.input(|input| cabinet::SelectionModifiers { toggle: input.modifiers.command, range: input.modifiers.shift });
        let Some(model) = &mut self.model else { return false; };
        if model.select(id, modifiers).is_err() { return false; }
        ui.ctx().accesskit_node_builder(response.id, |node| node.set_selected(model.is_selected(id)));
        !modifiers.toggle && !modifiers.range
    }

    pub fn reveal(&mut self, id: &str) -> Result<(), String> {
        self.model.as_mut().ok_or("Tree is not loaded")?.reveal(id).map_err(|e| e.to_string())?;
        self.scroll_target = Some(id.to_owned());
        Ok(())
    }
}

pub fn capabilities(open: bool, write: bool, folder: bool) -> cabinet::Capabilities {
    // Existing host command affordances only; these never replace backend authorization.
    cabinet::Capabilities {
        open, create: write && folder, rename: write, move_entry: write && folder,
        delete: write && folder, accept_children: folder, duplicate: false,
        download: false, tag: false, color: write && folder,
        open_parent: false, copy_reference: open, properties: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet::{ChildrenState, EntryKind, SelectionModifiers};

    fn entries(title: &str, loaded: bool) -> Vec<Entry> {
        let mut folder = Entry::new("folder", None, title, EntryKind::Folder);
        folder.capabilities = capabilities(true, true, true);
        folder.children = if loaded { ChildrenState::Loaded } else { ChildrenState::Unloaded };
        let mut denied = Entry::new("denied", None, "Denied", EntryKind::Folder);
        denied.capabilities = capabilities(false, false, true);
        denied.children = ChildrenState::Unloaded;
        let mut leaf = Entry::new("leaf", Some("folder".to_owned()), "Leaf", EntryKind::Leaf);
        leaf.capabilities = capabilities(true, true, false);
        vec![folder, denied, leaf]
    }

    fn state() -> State {
        let mut state = State::default();
        state.sync("workspace".to_owned(), 7, 1, entries("Before", false), true).unwrap();
        state
    }

    #[test]
    fn stale_epoch_and_sequence_cannot_settle_the_bound_read() {
        let mut state = state();
        state.request(Operation::LoadChildren { id: "folder".to_owned() }).unwrap();
        assert!(state.bind(7, 11));
        assert!(!state.terminal(8, 11, Ok(())));
        assert!(!state.terminal(7, 12, Ok(())));
        assert!(matches!(state.bound_operation(7, 11), Some(Operation::LoadChildren { .. })));
        assert!(state.terminal(7, 11, Ok(())));
        state.canonical_refetch();
        state.sync("workspace".to_owned(), 7, 2, entries("After", true), true).unwrap();
        assert!(state.host_operation().is_none());
        assert_eq!(state.model().unwrap().entry("folder").unwrap().label, "After");
    }

    #[test]
    fn write_terminal_requires_a_later_canonical_marker_to_release_custody() {
        let mut state = state();
        state.prepare_rename("folder", "After").unwrap().unwrap();
        assert!(state.bind(7, 21));
        state.canonical_refetch();
        assert!(state.terminal(7, 21, Ok(())));
        state.sync("workspace".to_owned(), 7, 2, entries("Before", false), true).unwrap();
        assert!(state.bound_operation(7, 21).is_some());
        assert!(!state.navigation_available());
        state.canonical_refetch();
        state.sync("workspace".to_owned(), 7, 3, entries("After", false), true).unwrap();
        assert!(state.host_operation().is_none());
        assert!(state.navigation_available());
    }

    #[test]
    fn failed_read_replacement_and_expansion_retain_the_original_host_read() {
        let mut state = state();
        state.request(Operation::LoadChildren { id: "folder".to_owned() }).unwrap();
        assert!(state.bind(7, 31));
        assert!(state.request(Operation::Open { id: "missing".to_owned() }).is_err());
        assert!(state.expand("missing", true).is_err());
        assert!(state.load_children_and_expand("denied").is_err());
        assert!(!state.model().unwrap().expanded("denied"));
        assert!(matches!(state.bound_operation(7, 31), Some(Operation::LoadChildren { id }) if id == "folder"));
        assert!(state.terminal(7, 31, Ok(())));
        state.canonical_refetch();
        state.sync("workspace".to_owned(), 7, 2, entries("Canonical", true), true).unwrap();
        assert!(state.host_operation().is_none());
        assert_eq!(state.model().unwrap().entry("folder").unwrap().label, "Canonical");
    }

    #[test]
    fn successful_read_replacement_retires_only_the_local_old_binding() {
        let mut state = state();
        let first = state.request(Operation::Open { id: "leaf".to_owned() }).unwrap();
        assert!(state.bind(7, 41));
        let replacement = state.request(Operation::LoadChildren { id: "folder".to_owned() }).unwrap();
        assert_ne!(first.id, replacement.id);
        assert!(state.bound_operation(7, 41).is_none());
        assert!(!state.terminal(7, 41, Ok(())));
        assert!(state.bind(7, 42));
        assert!(matches!(state.bound_operation(7, 42), Some(Operation::LoadChildren { id }) if id == "folder"));
    }

    #[test]
    fn cloning_a_host_never_duplicates_projection_or_dispatched_ownership() {
        let mut state = state();
        state.request(Operation::LoadChildren { id: "folder".to_owned() }).unwrap();
        assert!(state.bind(7, 51));
        let mut copy = state.clone();
        assert!(copy.model().is_none());
        assert!(copy.host_operation().is_none());
        assert!(!copy.terminal(7, 51, Ok(())));
        assert!(state.bound_operation(7, 51).is_some());
    }

    #[test]
    fn projection_restore_arrow_expansion_and_label_read_remain_distinct() {
        let mut state = state();
        state.projection_expand("folder", true).unwrap();
        state.select("leaf", SelectionModifiers::default()).unwrap();
        assert_eq!(state.model().unwrap().focused(), Some("leaf"));
        assert!(state.host_operation().is_none());
        assert!(state.model().unwrap().pending().is_none());
        state.projection_expand("folder", false).unwrap();
        state.expand("folder", true).unwrap();
        assert!(matches!(state.host_operation(), Some(Operation::LoadPage { id, cursor: None }) if id == "folder"));
        state.load_children_and_expand("folder").unwrap();
        assert!(matches!(state.host_operation(), Some(Operation::LoadChildren { id }) if id == "folder"));
        assert!(state.model().unwrap().expanded("folder"));
    }

    #[test]
    fn controlled_drafts_keep_kind_text_and_error_visibility() {
        let mut state = state();
        state.begin_create(None);
        assert!(matches!(state.edit(), Some(cabinet::interaction::Edit::Create { parent: None })));
        *state.edit_text_mut() = "New folder".to_owned();
        state.set_error("Denied by host".to_owned());
        assert_eq!(state.edit_text(), "New folder");
        assert_eq!(state.last_error(), Some("Denied by host"));
        state.cancel_edit();
        assert!(state.edit().is_none());
        state.begin_rename("folder".to_owned(), Some("After".to_owned())).unwrap();
        assert!(matches!(state.edit(), Some(cabinet::interaction::Edit::Rename { id }) if id == "folder"));
        let request = state.submit_edit().unwrap().unwrap();
        assert!(matches!(request.operation, Operation::Rename { id, name } if id == "folder" && name == "After"));
        assert!(state.edit().is_none());
    }
}
