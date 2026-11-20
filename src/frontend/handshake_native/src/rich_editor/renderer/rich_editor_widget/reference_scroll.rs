//! One-shot reference scrolling uses the same painted geometry and editor owner lease.
use super::RichEditorState;
use crate::rich_editor::document_owner::EditorSaveContext;

impl RichEditorState {
    pub(crate) fn request_reference_scroll(&mut self, path: Vec<usize>, context: EditorSaveContext) {
        self.pending_scroll_block = Some(path);
        self.reference_scroll_context = Some(context);
        self.last_consumed_scroll_block = None;
    }

    fn reject_reference_scroll(&mut self, message: &str) {
        self.pending_scroll_block = None;
        self.reference_scroll_context = None;
        self.interop_error = Some(message.to_owned());
    }

    pub(super) fn validate_reference_scroll(&mut self) -> bool {
        let Some(context) = self.reference_scroll_context else { return false };
        if self.document_owner.current_context().ok() != Some(context) || self.preedit.is_active() {
            self.reject_reference_scroll("The note changed before its reference could be shown");
            return false;
        }
        true
    }

    pub(super) fn consume_painted_scroll(&mut self, ui: &egui::Ui, rect: Option<egui::Rect>) {
        if self.reference_scroll_context.is_some() && !self.validate_reference_scroll() { return; }
        let Some(rect) = rect.filter(|rect| rect.is_finite() && rect.is_positive()) else { return };
        if self.pending_scroll_block.is_none() { return; }
        ui.scroll_to_rect(rect, Some(egui::Align::TOP));
        self.last_consumed_scroll_block = self.pending_scroll_block.take();
        self.reference_scroll_context = None;
        ui.ctx().request_repaint();
    }

    pub(super) fn finish_reference_scroll(&mut self, ui: &egui::Ui) {
        if self.reference_scroll_context.is_some() {
            self.reject_reference_scroll("The referenced block has no rendered target in this view");
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests;
