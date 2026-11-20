//! Notes coordinates and transactions for the published Handpick completion adapter.
use super::*;
use crate::rich_editor::document_owner::{DocumentEditOwner, EditorSaveContext};
use handpick::{CompletionSnapshotId, NativeCompletionPlan, NativeCompletionRange,
    NativeCompletionSelection, NativeCompletionSnapshot, WritingFence, WritingSession};

#[derive(Clone)]
pub(crate) struct NotesCompletionSnapshot {
    owner: DocumentEditOwner,
    context: EditorSaveContext,
    selection: Selection,
    native: NativeCompletionSnapshot,
    boundaries: Vec<(usize, DocPosition)>,
}

impl NotesCompletionSnapshot {
    pub(crate) fn native(&self) -> &NativeCompletionSnapshot { &self.native }

    pub(crate) fn current(&self, state: &RichEditorState, fence: WritingFence)
        -> Result<NativeCompletionSnapshot, String> {
        if !state.document_owner.same_owner(&self.owner)
            || state.document_owner.current_context().ok() != Some(self.context)
            || state.selection != self.selection || state.preedit.anchor.is_some() {
            return Err("Notes selection or authority changed; invoke completion again".into());
        }
        state.capture_handpick_completion(fence, self.native.id().clone()).map(|snapshot| snapshot.native)
    }

    fn replacement(&self, range: &NativeCompletionRange) -> Result<Selection, String> {
        match range {
            NativeCompletionRange::Selection => Ok(self.selection.clone()),
            NativeCompletionRange::BeforeCaret(span) => {
                let at = |offset| self.boundaries.iter().find(|(byte, _)| *byte == offset)
                    .map(|(_, position)| position.clone());
                if span.start > span.end { return Err("Completion range is invalid".into()); }
                let start = at(span.start).ok_or("Completion start is not a native text boundary")?;
                let end = at(span.end).ok_or("Completion end is not a native text boundary")?;
                Ok(Selection::text(start, end))
            }
        }
    }
}

impl RichEditorState {
    pub(crate) fn begin_handpick_template(&mut self) -> Result<(), String> {
        if self.handpick_template_open || self.handpick_inline_open || self.preedit.anchor.is_some()
            || self.slash_menu_opened_explicitly
            || self.slash_menu.as_ref().is_some_and(|menu| menu.prompt_active()) {
            return Err("Finish the current editor interaction first".into());
        }
        self.handpick_template_open = true;
        self.slash_menu = None;
        Ok(())
    }

    pub(crate) fn end_handpick_template(&mut self) {
        self.handpick_template_open = false;
        if self.preedit.anchor.is_none()
            && self.wikilink_autocomplete.is_none() && self.tag_autocomplete.is_none() {
            RichEditorWidget::refresh_slash_trigger(self);
        }
    }
    pub(crate) fn begin_handpick_inline(&mut self) -> Result<(), String> {
        if self.handpick_inline_open || self.handpick_template_open || self.preedit.anchor.is_some()
            || self.slash_menu_opened_explicitly
            || self.slash_menu.as_ref().is_some_and(|menu| menu.prompt_active()) {
            return Err("Finish the current editor interaction first".into());
        }
        self.handpick_inline_open = true;
        Ok(())
    }

    pub(crate) fn end_handpick_inline(&mut self) {
        self.handpick_inline_open = false;
        if self.document_snapshot().is_err() {
            self.wikilink_autocomplete = None;
            self.tag_autocomplete = None;
            self.slash_menu = None;
            return;
        }
        if self.preedit.anchor.is_some() || self.slash_menu_opened_explicitly
            || self.slash_menu.as_ref().is_some_and(|menu| menu.prompt_active()) { return; }
        RichEditorWidget::refresh_autocomplete_trigger(self);
        if self.wikilink_autocomplete.is_none() { RichEditorWidget::refresh_slash_trigger(self); }
        if self.wikilink_autocomplete.is_none() && self.slash_menu.is_none() {
            RichEditorWidget::refresh_tag_trigger(self);
        } else { self.tag_autocomplete = None; }
    }

    pub(crate) fn capture_handpick_completion(&self, fence: WritingFence, id: CompletionSnapshotId)
        -> Result<NotesCompletionSnapshot, String> {
        if self.preedit.anchor.is_some() { return Err("Finish text composition first".into()); }
        let context = self.document_owner.current_context().map_err(|error| error.to_string())?;
        if fence.revision != context.edit_revision { return Err("Notes revision changed".into()); }
        let selection = self.selection.clone();
        let (before, boundaries, selected) = self.with_document(|document| {
            let Selection::Text { anchor, head } = &selection else {
                return Err("Select editable text for completion".to_owned());
            };
            if anchor != head {
                let selected = selected_text(document, anchor, head)?;
                return Ok((String::new(), Vec::new(), NativeCompletionSelection::Text(selected)));
            }
            let (before, boundaries) = prefix(document, head)?;
            Ok((before, boundaries, NativeCompletionSelection::Caret))
        }).map_err(|error| error.to_string())??;
        if self.document_owner.current_context().ok() != Some(context) {
            return Err("Notes changed during completion capture".into());
        }
        let native = NativeCompletionSnapshot::new(fence, id, before, selected)
            .map_err(|error| error.to_string())?;
        Ok(NotesCompletionSnapshot { owner: self.document_owner.clone(), context, selection, native, boundaries })
    }

    pub(crate) fn apply_handpick_template(&mut self, snapshot: &NotesCompletionSnapshot,
        session: &WritingSession, plan: &NativeCompletionPlan, template: BlockNode) -> Result<(), String> {
        let current = snapshot.current(self, session.fence().map_err(|error| error.to_string())?)?;
        plan.validate(session, &current).map_err(|error| error.to_string())?;
        if plan.choice().key.provider.as_str() != "native.notes"
            || !matches!(&plan.choice().value, handpick::NativeCompletionValue::Template) {
            return Err("Resolved template no longer matches the completion action".into());
        }
        self.replace_handpick_blocks(snapshot, plan, template)
    }

    pub(crate) fn apply_handpick_block_embed(&mut self, snapshot: &NotesCompletionSnapshot,
        session: &WritingSession, plan: &NativeCompletionPlan, note_id: &str,
        target: &crate::rich_editor::block_identity::references::Target, label: &str) -> Result<(), String> {
        let current = snapshot.current(self, session.fence().map_err(|error| error.to_string())?)?;
        plan.validate(session, &current).map_err(|error| error.to_string())?;
        let matches = match &plan.choice().value {
            handpick::NativeCompletionValue::Reference { target: address, label: selected, embed: true }
                if selected == label && plan.choice().key.provider.as_str() == "native.notes" => {
                match address {
                    handpick::NoteAddress::Heading { note, block } => note.as_str() == note_id
                        && block.as_str() == target.block_id
                        && target.kind == crate::rich_editor::document_model::ReferenceTargetKind::Heading,
                    handpick::NoteAddress::Block { note, block } => note.as_str() == note_id
                        && block.as_str() == target.block_id
                        && target.kind == crate::rich_editor::document_model::ReferenceTargetKind::Block,
                    _ => false,
                }
            }
            _ => false,
        };
        if !matches { return Err("Resolved block embed no longer matches the completion action".into()); }
        let embed = crate::rich_editor::hs_embeds::new_block(note_id, target, label)?;
        self.replace_handpick_blocks(snapshot, plan, BlockNode::doc(vec![embed]))
    }

    fn replace_handpick_blocks(&mut self, snapshot: &NotesCompletionSnapshot,
        plan: &NativeCompletionPlan, template: BlockNode) -> Result<(), String> {
        let replacement = snapshot.replacement(plan.replacement_range())?;
        let owner = self.document_owner();
        let mut candidate = owner.prepare_edit().map_err(|error| error.to_string())?;
        if candidate.context() != snapshot.context {
            return Err("Notes changed before template insertion; writing retained".into());
        }
        let (document, history) = candidate.parts_mut();
        let before = crate::rich_editor::document_model::to_content_json_value(document);
        let applied = crate::rich_editor::block_identity::fragment::replace_template(
            document, &replacement, &template,
            crate::rich_editor::document_model::ActorKind::Operator, self.actor_id.as_str().to_owned(),
        ).map_err(|error| error.to_string())?;
        let selection = applied.selection;
        if !applied.receipt.forward.is_empty() { history.push(applied.receipt); }
        let after = crate::rich_editor::document_model::to_content_json_value(document);
        let commit = owner.commit_edit(candidate).map_err(|error| error.to_string())?;
        if commit.content_changed {
            self.pending_bus_undo.push((before, after));
            self.mark_document_dirty();
        }
        self.selection = selection;
        self.wikilink_autocomplete = None;
        self.tag_autocomplete = None;
        self.slash_menu = None;
        self.request_editor_focus();
        Ok(())
    }

    pub(crate) fn apply_handpick_inline(&mut self, snapshot: &NotesCompletionSnapshot,
        session: &WritingSession, plan: &NativeCompletionPlan, child: Child) -> Result<(), String> {
        let current = snapshot.current(self, session.fence().map_err(|error| error.to_string())?)?;
        plan.validate(session, &current).map_err(|error| error.to_string())?;
        let matches_choice = match (&plan.choice().value, &child) {
            (handpick::NativeCompletionValue::Text { text, .. }, Child::Text(leaf)) => leaf.text.to_string() == *text && leaf.marks.is_empty(),
            (handpick::NativeCompletionValue::Tag { label, .. }, Child::HsLink(link)) => {
                plan.choice().key.provider.as_str() == "native.loom.tags"
                    && *link == crate::rich_editor::document_model::node::HsLinkNode::tag_hub_id(
                        plan.choice().key.result.as_str(), label)
            }
            (handpick::NativeCompletionValue::Reference { target: handpick::NoteAddress::Note { note }, label, embed: false }, Child::HsLink(link)) => {
                *link == crate::rich_editor::document_model::node::HsLinkNode::new("note", note.as_str(), label)
            }
            (handpick::NativeCompletionValue::Reference { target, label, embed: false }, Child::HsLink(link)) => {
                let (note, block_id, kind) = match target {
                    handpick::NoteAddress::Heading { note, block } => (note, block,
                        crate::rich_editor::document_model::ReferenceTargetKind::Heading),
                    handpick::NoteAddress::Block { note, block } => (note, block,
                        crate::rich_editor::document_model::ReferenceTargetKind::Block),
                    handpick::NoteAddress::Note { .. } => return Err("Reference target changed".into()),
                };
                let target = crate::rich_editor::block_identity::references::Target {
                    block_id: block_id.as_str().into(), kind,
                };
                plan.choice().key.provider.as_str() == "native.notes"
                    && *link == target.link(note.as_str(), label)
            }
            _ => false,
        };
        if !matches_choice { return Err("Resolved content no longer matches the completion action".into()); }
        self.replace_handpick_child(snapshot, plan, child)
    }

    pub(crate) fn apply_handpick_note_embed(&mut self, snapshot: &NotesCompletionSnapshot,
        session: &WritingSession, plan: &NativeCompletionPlan, note_id: &str, block_id: &str) -> Result<(), String> {
        let current = snapshot.current(self, session.fence().map_err(|error| error.to_string())?)?;
        plan.validate(session, &current).map_err(|error| error.to_string())?;
        if plan.choice().key.provider.as_str() != "native.notes"
            || plan.choice().key.result.as_str() != note_id || block_id.trim().is_empty()
            || !matches!(&plan.choice().value, handpick::NativeCompletionValue::Reference {
                target: handpick::NoteAddress::Note { note }, embed: true, ..
            } if note.as_str() == note_id) {
            return Err("Resolved note embed no longer matches the completion action".into());
        }
        self.replace_handpick_child(snapshot, plan, Child::Transclusion(
            crate::rich_editor::document_model::node::TransclusionNode::new(block_id)))
    }

    fn replace_handpick_child(&mut self, snapshot: &NotesCompletionSnapshot,
        plan: &NativeCompletionPlan, child: Child) -> Result<(), String> {
        let replacement = snapshot.replacement(plan.replacement_range())?;
        let previous = self.selection.clone();
        self.selection = replacement;
        let applied = RichEditorWidget::insert_inline_child_at_selection_at(
            self, child, snapshot.context);
        if !applied {
            self.selection = previous;
            return Err("Notes rejected the completion; writing retained".into());
        }
        self.wikilink_autocomplete = None;
        self.tag_autocomplete = None;
        self.slash_menu = None;
        self.request_editor_focus();
        Ok(())
    }
}

fn prefix(document: &BlockNode, caret: &DocPosition) -> Result<(String, Vec<(usize, DocPosition)>), String> {
    let (index, parent_path) = caret.path.split_last().ok_or("Notes caret has no text path")?;
    let block = RichEditorWidget::block_at_path(document, parent_path).ok_or("Notes block changed")?;
    if block.kind.holds_inline_content() && block.children.is_empty() && *index == 0 && caret.char_offset == 0 {
        return Ok((String::new(), vec![(0, caret.clone())]));
    }
    let leaf = block.children.get(*index).and_then(Child::as_text).ok_or("Notes caret is not text")?;
    if caret.char_offset > leaf.text.len_chars() { return Err("Notes caret changed".into()); }
    let mut reversed = Vec::new();
    let mut bytes = 0;
    'runs: for child_index in (0..=*index).rev() {
        let Some(leaf) = block.children.get(child_index).and_then(Child::as_text) else { break };
        let end = if child_index == *index { caret.char_offset } else { leaf.text.len_chars() };
        for offset in (0..end).rev() {
            let ch = leaf.text.char_at(offset).ok_or("Notes text changed")?;
            if bytes + ch.len_utf8() > handpick::MAX_COMPLETION_SNAPSHOT_BYTES { break 'runs; }
            let path = [parent_path, &[child_index]].concat();
            reversed.push((ch, DocPosition::new(path, offset)));
            bytes += ch.len_utf8();
            if ch == '\n' { break 'runs; }
        }
    }
    reversed.reverse();
    let mut text = String::with_capacity(bytes);
    let mut boundaries = Vec::with_capacity(reversed.len() + 1);
    for (ch, position) in reversed {
        boundaries.push((text.len(), position));
        text.push(ch);
    }
    boundaries.push((text.len(), caret.clone()));
    Ok((text, boundaries))
}

fn selected_text(document: &BlockNode, anchor: &DocPosition, head: &DocPosition) -> Result<String, String> {
    let (start, end) = if (&anchor.path, anchor.char_offset) <= (&head.path, head.char_offset) {
        (anchor, head)
    } else { (head, anchor) };
    let mut output = String::new();
    let mut endpoints = [false; 2];
    let mut previous_block: Option<Vec<usize>> = None;
    fn visit(node: &BlockNode, path: &mut Vec<usize>, start: &DocPosition, end: &DocPosition,
        output: &mut String, endpoints: &mut [bool; 2], previous_block: &mut Option<Vec<usize>>) -> Result<(), String> {
        for (index, child) in node.children.iter().enumerate() {
            path.push(index);
            match child {
                Child::Text(leaf) if *path >= start.path && *path <= end.path => {
                    let from = if *path == start.path { endpoints[0] = true; start.char_offset } else { 0 };
                    let to = if *path == end.path { endpoints[1] = true; end.char_offset } else { leaf.text.len_chars() };
                    if from > to || to > leaf.text.len_chars() { return Err("Notes selection changed".into()); }
                    let block = path[..path.len() - 1].to_vec();
                    if previous_block.as_ref().is_some_and(|previous| *previous != block) {
                        if output.len() == handpick::MAX_COMPLETION_QUERY_BYTES { return Err("Select a shorter phrase for completion".into()); }
                        output.push('\n');
                    }
                    *previous_block = Some(block);
                    for at in from..to {
                        let ch = leaf.text.char_at(at).ok_or("Notes selection changed")?;
                        if output.len() + ch.len_utf8() > handpick::MAX_COMPLETION_QUERY_BYTES {
                            return Err("Select a shorter phrase for completion".into());
                        }
                        output.push(ch);
                    }
                }
                Child::Block(block) => {
                    visit(block, path, start, end, output, endpoints, previous_block)?;
                }
                _ => {}
            }
            path.pop();
            if endpoints[1] { break; }
        }
        Ok(())
    }
    visit(document, &mut Vec::new(), start, end, &mut output, &mut endpoints, &mut previous_block)?;
    if endpoints != [true, true] { return Err("Notes selection is unavailable".into()); }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::TextLeaf;

    fn document(runs: &[&str]) -> BlockNode {
        BlockNode::doc(vec![BlockNode::with_children(NodeKind::Paragraph,
            runs.iter().map(|text| Child::Text(TextLeaf::new(text))).collect())])
    }

    #[test]
    fn prefix_maps_original_utf8_across_formatted_runs() {
        let document = document(&["前 [[", "Cafe\u{301}"]);
        let caret = DocPosition::new(vec![0, 1], 5);
        let (text, boundaries) = prefix(&document, &caret).unwrap();
        assert_eq!(text, "前 [[Cafe\u{301}");
        assert_eq!(boundaries.iter().find(|(byte, _)| *byte == 4).unwrap().1,
            DocPosition::new(vec![0, 0], 2));
        assert_eq!(boundaries.last().unwrap(), &(text.len(), caret));
        assert!(!boundaries.iter().any(|(byte, _)| *byte == 1));
    }

    #[test]
    fn reverse_selection_keeps_original_unicode_text() {
        let document = document(&["prefix ", "界é", " suffix"]);
        assert_eq!(selected_text(&document, &DocPosition::new(vec![0, 2], 3),
            &DocPosition::new(vec![0, 1], 0)).unwrap(), "界é su");
    }

    #[test]
    fn selection_rejects_stale_offset_and_oversized_query() {
        let short = document(&["text"]);
        assert!(selected_text(&short, &DocPosition::new(vec![0, 0], 0),
            &DocPosition::new(vec![0, 0], 5)).is_err());
        let long = document(&[&"x".repeat(handpick::MAX_COMPLETION_QUERY_BYTES + 1)]);
        assert!(selected_text(&long, &DocPosition::new(vec![0, 0], 0),
            &DocPosition::new(vec![0, 0], handpick::MAX_COMPLETION_QUERY_BYTES + 1)).is_err());
    }

    fn offered_filter(state: &RichEditorState) -> (WritingSession, NotesCompletionSnapshot, NativeCompletionPlan) {
        let mut session = WritingSession::new(handpick::ContextId::new("test-account").unwrap(),
            handpick::SessionId::new("test-session").unwrap(), 8);
        session.bind(handpick::EditorId::new("test-editor").unwrap(),
            state.document_owner.current_context().unwrap().edit_revision).unwrap();
        let snapshot = state.capture_handpick_completion(session.fence().unwrap(),
            CompletionSnapshotId::new("test-selection").unwrap()).unwrap();
        let trigger = session.native_completion_trigger(snapshot.native(), handpick::CompletionInvocation::Manual)
            .unwrap().unwrap();
        let key = handpick::ResultKey { provider: handpick::ProviderId::new("native.query").unwrap(),
            result: handpick::ResultId::new("type:note").unwrap() };
        let action = handpick::Action { id: handpick::ActionId::new("complete:type:note").unwrap(),
            kind: handpick::ActionKind::ApplyCompletion };
        session.deliver(&session.fence().unwrap(), handpick::Channel::Suggestions,
            handpick::DeliveryStatus::Complete, vec![handpick::SearchItem { key: key.clone(),
                label: "type:note".into(), detail: None, actions: vec![action.clone()] }]).unwrap();
        session.select(&session.fence().unwrap(), key.clone()).unwrap();
        let plan = session.plan_native_completion(&trigger, snapshot.native(), handpick::NativeCompletionChoice {
            key, value: handpick::NativeCompletionValue::Text { action, text: "type:note ".into() }
        }).unwrap();
        (session, snapshot, plan)
    }

    #[test]
    fn filter_commit_preserves_suffix_and_has_one_undo_record() {
        let mut state = RichEditorState::new(document(&["界 type:no tail"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 9));
        let before = state.document_snapshot().unwrap();
        let (session, snapshot, plan) = offered_filter(&state);
        state.apply_handpick_inline(&snapshot, &session, &plan,
            Child::Text(TextLeaf::new("type:note "))).unwrap();
        let after = state.document_snapshot().unwrap();
        assert_eq!(crate::rich_editor::save::export::export_plain_text(&after), "界 type:note  tail");
        assert_eq!(state.pending_bus_undo.len(), 1);
        assert_eq!(state.history_snapshot().unwrap().len(), 1);
        state.edit_document(|document, history| history.undo(document)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), before);
    }

    #[test]
    fn moved_selection_rejects_without_retargeting_the_completion() {
        let mut state = RichEditorState::new(document(&["type:no tail"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 7));
        let (session, snapshot, plan) = offered_filter(&state);
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let selection = state.selection.clone();
        let before = state.document_snapshot().unwrap();
        assert!(state.apply_handpick_inline(&snapshot, &session, &plan,
            Child::Text(TextLeaf::new("type:note "))).is_err());
        assert_eq!(state.selection, selection);
        assert_eq!(state.document_snapshot().unwrap(), before);
        assert!(state.pending_bus_undo.is_empty());
    }

    #[test]
    fn substituted_material_cannot_use_an_offered_completion_action() {
        let mut state = RichEditorState::new(document(&["type:no tail"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 7));
        let (session, snapshot, plan) = offered_filter(&state);
        let before = state.document_snapshot().unwrap();
        assert!(state.apply_handpick_inline(&snapshot, &session, &plan,
            Child::Text(TextLeaf::new("different content"))).is_err());
        assert_eq!(state.document_snapshot().unwrap(), before);
        assert_eq!(state.selection, snapshot.selection);
        assert!(state.pending_bus_undo.is_empty());
        assert!(state.history_snapshot().unwrap().is_empty());
    }

    fn offered_tag(state: &RichEditorState, id: &str, label: &str)
        -> (WritingSession, NotesCompletionSnapshot, NativeCompletionPlan) {
        let mut session = WritingSession::new(handpick::ContextId::new("test-account").unwrap(),
            handpick::SessionId::new("test-session").unwrap(), 8);
        session.bind(handpick::EditorId::new("test-editor").unwrap(),
            state.document_owner.current_context().unwrap().edit_revision).unwrap();
        let snapshot = state.capture_handpick_completion(session.fence().unwrap(),
            CompletionSnapshotId::new("tag-selection").unwrap()).unwrap();
        let trigger = session.native_completion_trigger(snapshot.native(), handpick::CompletionInvocation::Automatic)
            .unwrap().unwrap();
        let key = handpick::ResultKey { provider: handpick::ProviderId::new("native.loom.tags").unwrap(),
            result: handpick::ResultId::new(id).unwrap() };
        let action = handpick::Action { id: handpick::ActionId::new(format!("tag:{id}")).unwrap(),
            kind: handpick::ActionKind::ApplyCompletion };
        session.deliver(&session.fence().unwrap(), handpick::Channel::Suggestions,
            handpick::DeliveryStatus::Complete, vec![handpick::SearchItem { key: key.clone(),
                label: format!("#{label}"), detail: None, actions: vec![action.clone()] }]).unwrap();
        session.select(&session.fence().unwrap(), key.clone()).unwrap();
        let plan = session.plan_native_completion(&trigger, snapshot.native(), handpick::NativeCompletionChoice {
            key, value: handpick::NativeCompletionValue::Tag { action, label: label.into() }
        }).unwrap();
        (session, snapshot, plan)
    }

    #[test]
    fn tag_completion_retains_hub_identity_full_label_reload_and_single_undo() {
        use crate::rich_editor::document_model::{doc_json, node::HsLinkNode};
        let mut state = RichEditorState::new(document(&["#C tail"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 2));
        let original = state.document_snapshot().unwrap();
        let label = "Cafe\u{301} / \u{6a21}\u{578b}";
        let id = "selected-hub-id";
        let (session, snapshot, plan) = offered_tag(&state, id, label);
        let link = HsLinkNode::tag_hub_id(id, label);
        state.apply_handpick_inline(&snapshot, &session, &plan, Child::HsLink(link.clone())).unwrap();
        let edited = state.document_snapshot().unwrap();
        assert!(edited.children[0].as_block().unwrap().children.contains(&Child::HsLink(link)));
        let reloaded = doc_json::from_json_value(&doc_json::to_content_json_value(&edited)).unwrap();
        assert_eq!(reloaded, edited);
        assert_eq!(state.pending_bus_undo.len(), 1);
        assert_eq!(state.history_snapshot().unwrap().len(), 1);
        state.edit_document(|doc, history| history.undo(doc)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), original);
        state.edit_document(|doc, history| history.redo(doc)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), edited);
    }

    #[test]
    fn same_name_tag_cannot_substitute_a_different_hub_identity() {
        use crate::rich_editor::document_model::node::HsLinkNode;
        let mut state = RichEditorState::new(document(&["#C tail"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 2));
        let original = state.document_snapshot().unwrap();
        let (session, snapshot, plan) = offered_tag(&state, "chosen-id", "Caf\u{e9} / \u{6a21}\u{578b}");
        assert!(state.apply_handpick_inline(&snapshot, &session, &plan,
            Child::HsLink(HsLinkNode::tag_hub_id("other-id", "Caf\u{e9} / \u{6a21}\u{578b}"))).is_err());
        assert_eq!(state.document_snapshot().unwrap(), original);
        assert_eq!(state.selection, snapshot.selection);
        assert!(state.pending_bus_undo.is_empty());
        assert!(state.history_snapshot().unwrap().is_empty());
    }

    #[test]
    fn ime_and_replaced_owner_reject_the_captured_snapshot() {
        let mut state = RichEditorState::new(document(&["type:no"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 7));
        let (session, snapshot, _) = offered_filter(&state);
        state.preedit.anchor = Some(DocPosition::new(vec![0, 0], 7));
        assert!(snapshot.current(&state, session.fence().unwrap()).is_err());
        state.preedit.clear();
        state.open_document_owner(document(&["type:no"]));
        assert!(snapshot.current(&state, session.fence().unwrap()).is_err());
    }
    fn offered_template(state: &RichEditorState) -> (WritingSession, NotesCompletionSnapshot, NativeCompletionPlan) {
        let mut session = WritingSession::new(handpick::ContextId::new("test-account").unwrap(),
            handpick::SessionId::new("template-session").unwrap(), 8);
        session.bind(handpick::EditorId::new("test-editor").unwrap(),
            state.document_owner.current_context().unwrap().edit_revision).unwrap();
        let snapshot = state.capture_handpick_completion(session.fence().unwrap(),
            CompletionSnapshotId::new("template-selection").unwrap()).unwrap();
        let trigger = session.native_completion_trigger(snapshot.native(), handpick::CompletionInvocation::Manual)
            .unwrap().unwrap();
        let key = handpick::ResultKey { provider: handpick::ProviderId::new("native.notes").unwrap(),
            result: handpick::ResultId::new("source-note").unwrap() };
        session.deliver(&session.fence().unwrap(), handpick::Channel::Suggestions,
            handpick::DeliveryStatus::Complete, vec![handpick::SearchItem { key: key.clone(),
                label: "Road".into(), detail: None, actions: vec![] }]).unwrap();
        session.select(&session.fence().unwrap(), key.clone()).unwrap();
        let plan = session.plan_native_completion(&trigger, snapshot.native(), handpick::NativeCompletionChoice {
            key, value: handpick::NativeCompletionValue::Template,
        }).unwrap();
        (session, snapshot, plan)
    }

    #[test]
    fn rich_template_completion_uses_captured_range_and_one_owner_revision() {
        use serde_json::json;
        let mut initial = document(&["before /Road after"]);
        initial.children[0].as_block_mut().unwrap().attrs.insert("block_id".into(), json!("original-id"));
        let mut state = RichEditorState::new(initial);
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 12));
        let before = state.document_snapshot().unwrap();
        let context = state.document_owner.current_context().unwrap();
        let (session, snapshot, plan) = offered_template(&state);
        let mut content = BlockNode::heading(2, "inserted");
        content.attrs.insert("block_id".into(), json!("source-id"));
        let template = BlockNode::doc(vec![content]);
        state.apply_handpick_template(&snapshot, &session, &plan, template).unwrap();
        let after = state.document_snapshot().unwrap();
        assert_eq!(after.children.len(), 3);
        assert_eq!(after.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(), "before ");
        assert_eq!(after.children[1].as_block().unwrap().kind, NodeKind::Heading(crate::rich_editor::document_model::HeadingLevel::new(2)));
        assert_eq!(after.children[2].as_block().unwrap().children[0].as_text().unwrap().text.to_string(), " after");
        assert_ne!(after.children[1].as_block().unwrap().attrs["block_id"], json!("source-id"));
        assert_eq!(state.pending_bus_undo.len(), 1);
        assert_eq!(state.history_snapshot().unwrap().len(), 1);
        assert_eq!(state.document_owner.current_context().unwrap().edit_revision, context.edit_revision + 1);
        state.edit_document(|document, history| history.undo(document)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), before);
    }

    #[test]
    fn stale_template_completion_preserves_document_selection_and_history() {
        let mut state = RichEditorState::new(document(&["/Road after"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 5));
        let (session, snapshot, plan) = offered_template(&state);
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let before = state.document_snapshot().unwrap();
        let selection = state.selection.clone();
        assert!(state.apply_handpick_template(&snapshot, &session, &plan,
            BlockNode::doc(vec![BlockNode::paragraph("template")])).is_err());
        assert_eq!(state.document_snapshot().unwrap(), before);
        assert_eq!(state.selection, selection);
        assert!(state.pending_bus_undo.is_empty());
        assert!(state.history_snapshot().unwrap().is_empty());
    }

    #[test]
    fn ime_and_replaced_owner_reject_the_template_without_history_or_save() {
        let mut state = RichEditorState::new(document(&["/Road"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 5));
        let (session, snapshot, plan) = offered_template(&state);
        let before = state.document_snapshot().unwrap();
        state.preedit.anchor = Some(DocPosition::new(vec![0, 0], 5));
        assert!(state.apply_handpick_template(&snapshot, &session, &plan,
            document(&["template"])).is_err());
        assert_eq!(state.document_snapshot().unwrap(), before);
        state.preedit.clear();
        state.open_document_owner(document(&["/New"]));
        let replacement = state.document_snapshot().unwrap();
        assert!(state.apply_handpick_template(&snapshot, &session, &plan,
            document(&["template"])).is_err());
        assert_eq!(state.document_snapshot().unwrap(), replacement);
        assert!(state.history_snapshot().unwrap().is_empty());
        assert!(state.pending_bus_undo.is_empty());
    }


    #[test]
    fn slash_refresh_clears_the_menu_when_the_owner_is_unavailable() {
        let mut state = RichEditorState::new(document(&["/daily"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 6));
        let endpoint = "http://127.0.0.1:9911";
        let authentication = crate::local_account::mock_account_context(endpoint);
        state.document_owner.bind_context("workspace", "note", endpoint, None,
            Some(authentication.clone())).unwrap();
        RichEditorWidget::refresh_slash_trigger(&mut state);
        assert_eq!(state.slash_menu.as_ref().unwrap().filter, "daily");
        authentication.invalidate();
        assert!(state.document_snapshot().is_err());
        RichEditorWidget::refresh_slash_trigger(&mut state);
        assert!(state.slash_menu.is_none());
        assert!(state.pending_bus_undo.is_empty());
    }

    #[test]
    fn revoked_between_cleanup_reads_makes_legacy_trigger_refresh_fail_closed() {
        let mut state = RichEditorState::new(document(&["[[Road"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 6));
        let endpoint = "http://127.0.0.1:9911";
        let account = crate::local_account::mock_account_context(endpoint);
        state.document_owner().bind_context("workspace", "note", endpoint, None, Some(account.clone())).unwrap();
        RichEditorWidget::refresh_autocomplete_trigger(&mut state);
        assert!(state.wikilink_autocomplete.is_some());
        assert!(state.document_snapshot().is_ok());
        account.invalidate();
        assert!(RichEditorWidget::caret_text_before(&state).is_none());
        RichEditorWidget::refresh_autocomplete_trigger(&mut state);
        RichEditorWidget::refresh_tag_trigger(&mut state);
        assert!(state.wikilink_autocomplete.is_none() && state.tag_autocomplete.is_none());
        assert!(state.pending_bus_undo.is_empty());
    }

    fn offered_note(state: &RichEditorState) -> (WritingSession, NotesCompletionSnapshot, NativeCompletionPlan) {
        let mut session = WritingSession::new(handpick::ContextId::new("test-account").unwrap(),
            handpick::SessionId::new("embed-session").unwrap(), 8);
        session.bind(handpick::EditorId::new("test-editor").unwrap(),
            state.document_owner.current_context().unwrap().edit_revision).unwrap();
        let snapshot = state.capture_handpick_completion(session.fence().unwrap(),
            CompletionSnapshotId::new("embed-selection").unwrap()).unwrap();
        let trigger = session.native_completion_trigger(snapshot.native(), handpick::CompletionInvocation::Manual)
            .unwrap().unwrap();
        let key = handpick::ResultKey { provider: handpick::ProviderId::new("native.notes").unwrap(),
            result: handpick::ResultId::new("source-note").unwrap() };
        session.deliver(&session.fence().unwrap(), handpick::Channel::Suggestions,
            handpick::DeliveryStatus::Complete, vec![handpick::SearchItem { key: key.clone(),
                label: "Road".into(), detail: None, actions: vec![] }]).unwrap();
        session.select(&session.fence().unwrap(), key.clone()).unwrap();
        let plan = session.plan_native_completion(&trigger, snapshot.native(), handpick::NativeCompletionChoice {
            key, value: handpick::NativeCompletionValue::Reference {
                target: handpick::NoteAddress::Note { note: handpick::NoteId::new("source-note").unwrap() },
                label: "Road".into(), embed: false,
            },
        }).unwrap();
        (session, snapshot, plan)
    }

    #[test]
    fn whole_note_embed_preserves_surrounding_text_wire_identity_and_single_undo_redo() {
        use crate::rich_editor::document_model::doc_json;
        let mut state = RichEditorState::new(document(&["前 ![[Road tail"]));
        state.selection = Selection::caret(DocPosition::new(vec![0, 0], 9));
        let before = state.document_snapshot().unwrap();
        let (session, snapshot, plan) = offered_note(&state);
        state.apply_handpick_note_embed(&snapshot, &session, &plan,
            "source-note", "loom-note-projection").unwrap();
        let after = state.document_snapshot().unwrap();
        let wire = doc_json::to_content_json_value(&after);
        let children = wire["content"][0]["content"].as_array().unwrap();
        let embed = children.iter().find(|child| child["type"] == "loomTransclusion").unwrap();
        assert_eq!(embed, &serde_json::json!({"type":"loomTransclusion", "attrs":{"refValue":"loom-note-projection"}}));
        assert_eq!(children.iter().filter_map(|child| child["text"].as_str()).collect::<String>(), "前  tail");
        assert_eq!(doc_json::from_json_value(&wire).unwrap(), after);
        assert_eq!(state.history_snapshot().unwrap().len(), 1);
        assert_eq!(state.pending_bus_undo.len(), 1);
        state.edit_document(|document, history| history.undo(document)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), before);
        state.edit_document(|document, history| history.redo(document)).unwrap();
        assert_eq!(state.document_snapshot().unwrap(), after);
    }

    #[test]
    fn whole_note_embed_rejects_wrong_note_missing_block_and_non_embed_plan_without_mutation() {
        for (text, note, block) in [("![[Road", "other-note", "loom-note-projection"),
            ("![[Road", "source-note", ""), ("[[Road", "source-note", "loom-note-projection")] {
            let mut state = RichEditorState::new(document(&[text]));
            state.selection = Selection::caret(DocPosition::new(vec![0, 0], text.chars().count()));
            let before = state.document_snapshot().unwrap();
            let (session, snapshot, plan) = offered_note(&state);
            assert!(state.apply_handpick_note_embed(&snapshot, &session, &plan, note, block).is_err());
            assert_eq!(state.document_snapshot().unwrap(), before);
            assert_eq!(state.selection, snapshot.selection);
            assert!(state.history_snapshot().unwrap().is_empty());
            assert!(state.pending_bus_undo.is_empty());
        }
    }

    fn offered_structural(state: &RichEditorState, heading: bool)
        -> (WritingSession, NotesCompletionSnapshot, NativeCompletionPlan, Child) {
        let mut session = WritingSession::new(handpick::ContextId::new("account").unwrap(),
            handpick::SessionId::new("block-session").unwrap(), 8);
        session.bind(handpick::EditorId::new("editor").unwrap(),
            state.document_owner.current_context().unwrap().edit_revision).unwrap();
        let snapshot = state.capture_handpick_completion(session.fence().unwrap(),
            CompletionSnapshotId::new("block-selection").unwrap()).unwrap();
        let trigger = session.native_completion_trigger(snapshot.native(), handpick::CompletionInvocation::Manual)
            .unwrap().unwrap();
        let key = handpick::ResultKey { provider:handpick::ProviderId::new("native.notes").unwrap(),
            result:handpick::ResultId::new("opaque-result-key").unwrap() };
        session.deliver(&session.fence().unwrap(), handpick::Channel::Suggestions,
            handpick::DeliveryStatus::Complete, vec![handpick::SearchItem { key:key.clone(),
                label:"Source label".into(), detail:None, actions:vec![] }]).unwrap();
        session.select(&session.fence().unwrap(), key.clone()).unwrap();
        let note = handpick::NoteId::new("owning-note").unwrap();
        let block = handpick::BlockId::new("persisted-node").unwrap();
        let address = if heading { handpick::NoteAddress::Heading {note,block} }
            else { handpick::NoteAddress::Block {note,block} };
        let plan = session.plan_native_completion(&trigger, snapshot.native(), handpick::NativeCompletionChoice {
            key, value:handpick::NativeCompletionValue::Reference { target:address, label:"Source label".into(), embed:false }
        }).unwrap();
        let handpick::NativeCompletionValue::Reference { label, .. } = &plan.choice().value else { unreachable!() };
        let target = crate::rich_editor::block_identity::references::Target { block_id:"persisted-node".into(),
            kind:if heading { crate::rich_editor::document_model::ReferenceTargetKind::Heading }
                else { crate::rich_editor::document_model::ReferenceTargetKind::Block } };
        let child = Child::HsLink(target.link("owning-note", label));
        (session,snapshot,plan,child)
    }

    #[test]
    fn typed_heading_and_block_insertions_round_trip_alias_identity_and_one_undo_redo() {
        for (query, heading) in [("[[Road#heading|Alias",true),("[[Road^body|Alias",false)] {
            let mut state = RichEditorState::new(document(&[query]));
            state.selection = Selection::caret(DocPosition::new(vec![0,0],query.chars().count()));
            let before = state.document_snapshot().unwrap();
            let (session,snapshot,plan,child) = offered_structural(&state,heading);
            state.apply_handpick_inline(&snapshot,&session,&plan,child.clone()).unwrap();
            let after = state.document_snapshot().unwrap();
            assert!(after.children[0].as_block().unwrap().children.contains(&child));
            let wire = crate::rich_editor::document_model::doc_json::to_content_json_value(&after);
            assert_eq!(crate::rich_editor::document_model::doc_json::from_json_value(&wire).unwrap(),after);
            let Child::HsLink(link) = child else { unreachable!() };
            assert_eq!(link.label,"Alias");
            assert_eq!(link.ref_value,"owning-note");
            assert_eq!(link.target_block_id.as_deref(),Some("persisted-node"));
            assert_eq!(state.history_snapshot().unwrap().len(),1);
            assert_eq!(state.pending_bus_undo.len(),1);
            state.edit_document(|document,history|history.undo(document)).unwrap();
            assert_eq!(state.document_snapshot().unwrap(),before);
            state.edit_document(|document,history|history.redo(document)).unwrap();
            assert_eq!(state.document_snapshot().unwrap(),after);
        }
    }

    #[test]
    fn wrong_owner_kind_identity_and_stale_selection_reject_reference_insertion() {
        for change in ["owner","kind","identity","selection"] {
            let mut state = RichEditorState::new(document(&["[[Road#heading"]));
            state.selection = Selection::caret(DocPosition::new(vec![0,0],14));
            let (session,snapshot,plan,mut child) = offered_structural(&state,true);
            let Child::HsLink(link) = &mut child else { unreachable!() };
            match change {
                "owner" => link.ref_value="different-owner".into(),
                "kind" => link.target_kind=Some(crate::rich_editor::document_model::ReferenceTargetKind::Block),
                "identity" => link.target_block_id=Some("different-block".into()),
                "selection" => state.selection=Selection::caret(DocPosition::new(vec![0,0],0)),
                _ => unreachable!(),
            }
            let before=state.document_snapshot().unwrap();
            let selection=state.selection.clone();
            assert!(state.apply_handpick_inline(&snapshot,&session,&plan,child).is_err(),"accepted {change}");
            assert_eq!(state.document_snapshot().unwrap(),before);
            assert_eq!(state.selection,selection);
            assert!(state.history_snapshot().unwrap().is_empty());
            assert!(state.pending_bus_undo.is_empty());
        }
    }
    #[test]
    fn block_embed_completion_retains_distinct_identities_surrounding_text_reload_and_single_undo() {
        for (trigger,heading) in [("![[Road#heading|Alias",true),("!((body|Alias",false)] {
            let prefix=format!("before {trigger}");
            let mut state=RichEditorState::new(document(&[&format!("{prefix} after")]));
            state.selection=Selection::caret(DocPosition::new(vec![0,0],prefix.chars().count()));
            let before=state.document_snapshot().unwrap();
            let (session,snapshot,plan,child)=offered_structural(&state,heading);
            let Child::HsLink(link)=child else {unreachable!()};
            let target=crate::rich_editor::block_identity::references::Target::from_link(&link).unwrap();
            state.apply_handpick_block_embed(&snapshot,&session,&plan,&link.ref_value,&target,&link.label).unwrap();
            let after=state.document_snapshot().unwrap();
            assert_eq!(after.children.len(),3);
            let embed=after.children[1].as_block().unwrap();
            assert_eq!(embed.kind,NodeKind::HsEmbed);
            assert_eq!(embed.attrs["refValue"],serde_json::json!("owning-note"));
            assert_eq!(embed.attrs["target_block_id"],serde_json::json!("persisted-node"));
            assert_ne!(embed.attrs["block_id"],embed.attrs["target_block_id"]);
            assert_eq!(embed.attrs["label"],serde_json::json!("Alias"));
            assert_eq!(after.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),"before ");
            assert_eq!(after.children[2].as_block().unwrap().children[0].as_text().unwrap().text.to_string()," after");
            let wire=crate::rich_editor::document_model::to_content_json_value(&after);
            assert_eq!(crate::rich_editor::document_model::from_json_value(&wire).unwrap(),after);
            assert_eq!(state.history_snapshot().unwrap().len(),1);
            assert_eq!(state.pending_bus_undo.len(),1);
            state.edit_document(|doc,history|history.undo(doc)).unwrap();
            assert_eq!(state.document_snapshot().unwrap(),before);
            state.edit_document(|doc,history|history.redo(doc)).unwrap();
            assert_eq!(state.document_snapshot().unwrap(),after);
        }
    }

    #[test]
    fn block_embed_completion_rejects_wrong_material_and_stale_or_ime_snapshots() {
        for change in ["note","target","kind","label","selection","ime","link"] {
            let query=if change=="link" {"[[Road#heading"} else {"![[Road#heading"};
            let mut state=RichEditorState::new(document(&[query]));
            state.selection=Selection::caret(DocPosition::new(vec![0,0],query.chars().count()));
            let (session,snapshot,plan,child)=offered_structural(&state,true);
            let Child::HsLink(link)=child else {unreachable!()};
            let mut target=crate::rich_editor::block_identity::references::Target::from_link(&link).unwrap();
            let mut note=link.ref_value;let mut label=link.label;
            match change {
                "note"=>note="other".into(),
                "target"=>target.block_id="other".into(),
                "kind"=>target.kind=crate::rich_editor::document_model::ReferenceTargetKind::Block,
                "label"=>label="other".into(),
                "selection"=>state.selection=Selection::caret(DocPosition::new(vec![0,0],0)),
                "ime"=>state.preedit.anchor=Some(DocPosition::new(vec![0,0],0)),
                _=>(),
            }
            let before=state.document_snapshot().unwrap();let selection=state.selection.clone();
            assert!(state.apply_handpick_block_embed(&snapshot,&session,&plan,&note,&target,&label).is_err(),"{change}");
            assert_eq!(state.document_snapshot().unwrap(),before);assert_eq!(state.selection,selection);
            assert!(state.pending_bus_undo.is_empty());assert!(state.history_snapshot().unwrap().is_empty());
        }
    }

}
