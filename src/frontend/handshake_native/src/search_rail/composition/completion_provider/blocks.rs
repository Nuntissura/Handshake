//! Authorized persisted-block lookup. A block template inserts its entire owning note.
use super::*;
use crate::rich_editor::block_identity::references::{self, Target as BlockTarget};
use crate::rich_editor::document_model::ReferenceTargetKind;

pub(super) fn matches_address(target: &BlockTarget, note_id: &str, address: &NoteAddress) -> bool {
    match address {
        NoteAddress::Heading { note, block } => target.kind == ReferenceTargetKind::Heading
            && note.as_str() == note_id && block.as_str() == target.block_id,
        NoteAddress::Block { note, block } => target.kind == ReferenceTargetKind::Block
            && note.as_str() == note_id && block.as_str() == target.block_id,
        NoteAddress::Note { .. } => false,
    }
}

impl Provider {
    pub(super) async fn block_rows(&self, note_title: Option<&str>, text: &str,
        headings_only: bool, template: bool, embed: bool) -> Result<Page, String> {
        self.active()?;
        let headers = HskDocumentHeaders::for_read(&self.account.session_id, "handpick-completion");
        let mut documents = self.documents().list_documents(&headers, &self.workspace).await
            .map_err(|_| "Authorized Notes could not be loaded")?;
        self.active()?;
        if documents.len() > MAX_OWNER_ROWS { return Err("Notes owner returned more rows than this completion can safely inspect".into()); }
        documents.retain(|document| note_title.is_none_or(|title| normalized(&document.title).contains(title)));
        documents.sort_by(|left, right| left.title.cmp(&right.title)
            .then_with(|| left.rich_document_id.cmp(&right.rich_document_id)));
        let mut rows = Vec::new();
        for summary in documents {
            let document = self.source_document(&summary.rich_document_id, &summary.updated_at).await?;
            let content = crate::rich_editor::document_model::doc_json::from_json_value(&document.content_json)
                .map_err(|_| "A searched note is incompatible with Notes")?;
            for block in references::index(&content)? {
                if (headings_only && block.target.kind != ReferenceTargetKind::Heading)
                    || !normalized(&block.label).contains(text) { continue; }
                if block.label.len() > MAX_COMPLETION_QUERY_BYTES {
                    return Err("The matching block label exceeds completion limits; writing retained".into());
                }
                let identity = format!("{}:{}{}:{}", document.document_id.len(), document.document_id,
                    block.target.kind.as_str(), block.target.block_id);
                let key = key("native.notes", &opaque_id(&identity))?;
                let value = if template { NativeCompletionValue::Template } else {
                    let note = NoteId::new(&document.document_id).map_err(str::to_owned)?;
                    let id = BlockId::new(&block.target.block_id).map_err(str::to_owned)?;
                    let target = if block.target.kind == ReferenceTargetKind::Heading {
                        NoteAddress::Heading { note, block: id }
                    } else { NoteAddress::Block { note, block: id } };
                    NativeCompletionValue::Reference { target, label: block.label.clone(), embed }
                };
                rows.push(Row { item: SearchItem { key: key.clone(), label: block.label,
                    detail: Some(if template { format!("Insert owning note: {}", document.title) }
                        else { document.title.clone() }), actions: Vec::new() },
                    choice: NativeCompletionChoice { key, value }, target: Target::Block {
                        note_id: document.document_id.clone(), updated_at: document.updated_at.clone(),
                        doc_version: document.doc_version, target: block.target, template,
                    } });
                if rows.len() > PAGE_SIZE {
                    rows.truncate(PAGE_SIZE);
                    self.active()?;
                    return Ok(Page { rows, status: DeliveryStatus::Partial });
                }
            }
        }
        self.active()?;
        Ok(Page { rows, status: DeliveryStatus::Complete })
    }
}
