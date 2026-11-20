use super::*;
use crate::rich_editor::hs_embeds::Action;
use crate::rich_editor::document_owner::EditorSaveContext;
use crate::diagnostics::gui_events::{record_gui_observation, GuiDiagnosticOperation, GuiDiagnosticPhase};

impl RichEditorState {
    pub(super) fn apply_hs_embed_action(&mut self, action: Action,
        context: Option<EditorSaveContext>, read_only: bool) {
        let removing = matches!(&action, Action::Remove(_));
        let result = self.commit_hs_embed_action(action, context, read_only);
        let phase = match (result.is_ok(), removing) {
            (true, true) => GuiDiagnosticPhase::Completed,
            (true, false) => GuiDiagnosticPhase::Accepted,
            (false, _) => GuiDiagnosticPhase::Rejected,
        };
        let _ = record_gui_observation(GuiDiagnosticOperation::NoteEmbedAction, phase);
        if let Err(error) = result { self.interop_error = Some(error); }
    }

    fn commit_hs_embed_action(&mut self, action: Action,
        context: Option<EditorSaveContext>, read_only: bool) -> Result<(), String> {
        if self.preedit.is_active() { return Err("Finish text composition before using an embed".into()); }
        let owner = self.document_owner();
        let context = context.ok_or("The embed account is unavailable")?;
        let account = self.authenticated_context.as_ref().ok_or("The embed account is unavailable")?;
        let identity = owner.document_identity().ok_or("The embed owner is unavailable")?;
        if owner.current_context().ok() != Some(context)
            || !owner.has_active_binding(identity.workspace_id(), &self.owner_backend_url, account) {
            return Err("The embed owner or account changed".into());
        }
        match action {
            Action::Open(reference) => {
                owner.with_save_gate(|gate| -> Result<(), String> {
                    if gate.current_context() != Some(context) { return Err("The embed owner changed".into()); }
                    reference.locate_occurrence(gate.document())?;
                    Ok(())
                })?;
                self.pending_events.push(crate::rich_editor::wikilinks::inline_view::EditorEvent::NoteEmbedActivated {
                    embed: crate::rich_editor::document_model::to_content_json_value(
                        &BlockNode::doc(vec![reference.node])), context,
                });
            }
            Action::Remove(reference) => {
                if read_only { return Err("This note is read only".into()); }
                let mut candidate = owner.prepare_edit().map_err(|error| error.to_string())?;
                if candidate.context() != context { return Err("The embed changed before removal".into()); }
                let (document, history) = candidate.parts_mut();
                let before = crate::rich_editor::document_model::to_content_json_value(document);
                let (receipt, selection) = crate::rich_editor::hs_embeds::remove(document, &reference, &self.actor_id)?;
                history.push(receipt);
                let after = crate::rich_editor::document_model::to_content_json_value(document);
                let committed = owner.commit_edit(candidate).map_err(|error| error.to_string())?;
                if committed.content_changed {
                    self.pending_bus_undo.push((before, after));
                    self.mark_document_dirty();
                }
                self.selection = selection;
                self.request_editor_focus();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
