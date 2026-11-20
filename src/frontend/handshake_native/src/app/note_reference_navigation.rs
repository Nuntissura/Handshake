//! Resolve typed note references through the existing authenticated Notes loader.
use super::*;
use crate::rich_editor::block_identity::references::{self, Target};
use crate::rich_editor::document_model::{BlockNode, HsLinkNode};
use crate::rich_editor::document_owner::{DocumentEditOwner, EditorSaveContext};

pub(super) struct PendingReference {
    reference: HsLinkNode,
    embed: Option<BlockNode>,
    target: Target,
    workspace: String,
    endpoint: String,
    account: Arc<crate::local_account::AuthenticatedContext>,
    source: DocumentEditOwner,
    source_context: EditorSaveContext,
    destination: DocumentEditOwner,
}

impl PendingReference {
    fn contains_source(&self, document: &BlockNode) -> bool {
        match &self.embed {
            Some(embed) => crate::rich_editor::hs_embeds::Reference::from_block(embed)
                .and_then(|reference| reference.locate_occurrence(document)).is_ok(),
            None => references::contains_link(document, &self.reference),
        }
    }

    pub(super) fn validate_loaded(&self, doc: &backend_client::RichDocBody,
        content: &BlockNode) -> Result<u64, String> {
        if doc.document_id != self.reference.ref_value || doc.workspace_id != self.workspace {
            return Err("The reference owner differs from the loaded note".into());
        }
        self.target.locate(content).map(|_| doc.doc_version)
    }

    fn validate_source(&self) -> Result<(), String> {
        if !self.source.has_active_binding(&self.workspace, &self.endpoint, &self.account) {
            return Err("The reference account or source note is unavailable".into());
        }
        self.source.with_save_gate(|gate| {
            if gate.current_context() != Some(self.source_context)
                || !self.contains_source(gate.document()) {
                return Err("The source note changed before reference navigation completed".into());
            }
            Ok(())
        })
    }
    fn select_loaded(&self, state: &mut crate::rich_editor::renderer::rich_editor_widget::RichEditorState,
        note: &str, version: u64) -> Result<(), String> {
        self.validate_source()?;
        let owner = state.document_owner();
        if !owner.same_owner(&self.destination)
            || !owner.has_active_binding(&self.workspace, &self.endpoint, &self.account)
            || state.preedit.is_active() {
            return Err("The destination note lease changed".to_owned());
        }
        owner.with_save_gate(|gate| {
            if gate.current_context().is_none() || !gate.document_is_loaded()
                || gate.known_server_version() != version
                || !gate.document_identity().is_some_and(|identity| identity.document_id() == note
                    && identity.workspace_id() == self.workspace) {
                return Err("The destination note is unavailable".to_owned());
            }
            let path = self.target.locate(gate.document())?;
            let context = gate.current_context().ok_or("The destination note lease expired")?;
            state.selection = crate::rich_editor::document_model::selection::Selection::node(path.clone());
            state.request_reference_scroll(path, context);
            state.request_editor_focus();
            Ok(())
        })
    }

}

impl HandshakeApp {
    fn active_note_tab(&self) -> Option<RichDocumentViewKey> {
        let pane = self.active_pane.as_ref()?;
        let tab = self.tab_bar_states.get(pane)?.active()?;
        if !matches!(tab.pane_type, PaneType::LoomWikiPage) { return None; }
        Some((pane.to_string(), tab.content_id.clone()?))
    }

    pub(super) fn open_note_reference(&mut self, origin: (String, Option<String>),
        reference: HsLinkNode) -> Result<(), String> {
        self.open_note_target(origin, reference, None)
    }

    pub(super) fn open_note_embed(&mut self, origin: (String, Option<String>),
        embed: serde_json::Value, context: EditorSaveContext) -> Result<(), String> {
        let document = crate::rich_editor::document_model::from_json_value(&embed)
            .map_err(|error| error.to_string())?;
        if document.children.len() != 1 { return Err("The embed source is malformed".into()); }
        let embed = document.children[0].as_block().ok_or("The embed source is not a block")?.clone();
        let reference = crate::rich_editor::hs_embeds::Reference::from_block(&embed)?.link();
        self.open_note_target(origin, reference, Some((embed, context)))
    }

    fn open_note_target(&mut self, origin: (String, Option<String>), reference: HsLinkNode,
        embed: Option<(BlockNode, EditorSaveContext)>) -> Result<(), String> {
        let target = Target::from_link(&reference)?;
        let source_id = origin.1.ok_or("The reference source note is unavailable")?;
        if !self.editor_mounts.rich_documents.view_keys_for_content_id(&source_id)
            .contains(&(origin.0.clone(), source_id.clone())) {
            return Err("The reference source view is unavailable".into());
        }
        let account = self.local_account.context.clone().filter(|account| account.is_active())
            .ok_or("The reference account is unavailable")?;
        let source_state = self.editor_mounts.rich_documents.state_for_view(Some(&source_id), &origin.0);
        let source = source_state.lock().map_err(|_| "The source note is unavailable")?.document_owner();
        if !source.document_identity().is_some_and(|identity| identity.document_id() == source_id
            && identity.workspace_id() == self.active_project_id) {
            return Err("The reference source owner differs from its view".into());
        }
        let source_context = source.current_context().map_err(|error| error.to_string())?;
        if embed.as_ref().is_some_and(|(_, context)| *context != source_context) {
            return Err("The embed source changed before navigation".into());
        }
        let mut pending = PendingReference { reference, embed: embed.map(|(node, _)| node), target, workspace: self.active_project_id.clone(),
            endpoint: self.rich_doc_base_url.clone(), account, source_context,
            destination: source.clone(), source };
        pending.validate_source()?;
        self.nav_pending_label = Some(pending.reference.label.clone());
        let outcome = crate::quick_switcher::ShellNavigator::open_document(self, &pending.reference.ref_value);
        self.nav_pending_label = None;
        if !matches!(outcome, crate::quick_switcher::NavDispatchOutcome::Opened { .. }) {
            return Err("The Notes pane is unavailable".into());
        }
        let key = self.active_note_tab().filter(|(_, note)| note == &pending.reference.ref_value)
            .ok_or("The reference destination view changed")?;
        let state = self.editor_mounts.rich_documents.state_for_view(Some(&key.1), &key.0);
        pending.destination = state.lock().map_err(|_| "The destination note is unavailable")?.document_owner();
        self.pending_note_references.insert(key, pending);
        Ok(())
    }

    pub(super) fn finish_note_reference(&mut self, pane: &str, note: &str,
        loaded: Option<Result<u64, String>>) {
        let key = (pane.to_owned(), note.to_owned());
        let Some(pending) = self.pending_note_references.remove(&key) else { return };
        let result = (|| {
            let version = loaded.ok_or("The reference has no authoritative note response")??;
            if self.active_note_tab().as_ref() != Some(&key)
                || self.active_project_id != pending.workspace || self.rich_doc_base_url != pending.endpoint
                || !self.local_account.context.as_ref().is_some_and(|account|
                    Arc::ptr_eq(account, &pending.account) && account.is_active()) {
                return Err("The reference destination or account changed".to_owned());
            }
            let state = self.editor_mounts.rich_documents.state_for_view(Some(note), pane);
            let mut state = state.lock().map_err(|_| "The destination note is unavailable")?;
            pending.select_loaded(&mut state, note, version)
        })();
        if pending.embed.is_some() {
            use crate::diagnostics::gui_events::{record_gui_observation, GuiDiagnosticOperation, GuiDiagnosticPhase};
            let _ = record_gui_observation(GuiDiagnosticOperation::NoteEmbedAction,
                if result.is_ok() { GuiDiagnosticPhase::Completed } else { GuiDiagnosticPhase::Failed });
        }
        self.quick_switcher_nav_status = result.err();
    }
}

#[cfg(test)]
mod tests;
