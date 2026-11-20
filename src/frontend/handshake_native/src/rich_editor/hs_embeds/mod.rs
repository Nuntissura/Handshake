//! Host admission and rendering for persisted note-block embeds.
use super::block_identity::references::{self, Target};
use super::document_model::{BlockNode, Child, HsLinkNode, NodeKind, ReferenceTargetKind};
use serde_json::json;

mod runtime;
mod view;
pub(crate) use runtime::Cache;
pub(crate) use view::{Action,Frame};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    occurrence: String,
    note: String,
    kind: String,
    block: String,
}

#[derive(Clone)]
pub(crate) struct Reference {
    key: Key,
    target: Target,
    label: String,
    pub(crate) node: BlockNode,
}

impl Reference {
    pub(crate) fn from_block(node: &BlockNode) -> Result<Self, String> {
        if node.kind != NodeKind::HsEmbed { return Err("This block is not a note embed".into()); }
        super::document_model::schema::validate_node(node).map_err(|error|error.to_string())?;
        let text = |name: &str| node.attrs.get(name).and_then(|value|value.as_str())
            .filter(|value|!value.trim().is_empty()).map(str::to_owned);
        if text("refKind").as_deref() != Some("note")
            || node.attrs.get("resolved").is_some_and(|value|value.as_bool()!=Some(true)) {
            return Err("The note embed target is unavailable".into());
        }
        let note=text("refValue").ok_or("The embed has no owning note")?;
        let occurrence=text("block_id").ok_or("The embed has no persisted occurrence identity")?;
        let block=text("target_block_id").ok_or("The embed has no persisted target identity")?;
        let kind=text("target_kind").and_then(|kind|ReferenceTargetKind::from_json_type(&kind))
            .filter(|kind|matches!(kind,ReferenceTargetKind::Block|ReferenceTargetKind::Heading))
            .ok_or("The embed has no supported block target kind")?;
        Ok(Self { key:Key { occurrence,note,kind:kind.as_str().into(),block:block.clone() },
            target:Target { block_id:block,kind },label:text("label").unwrap_or_default(),node:node.clone() })
    }

    pub(crate) fn link(&self)->HsLinkNode { self.target.link(&self.key.note,&self.label) }

    pub(crate) fn locate_occurrence(&self,document:&BlockNode)->Result<Vec<usize>,String> {
        locate_occurrence(&self.node, document)
    }
}

pub(crate) fn new_block(note:&str,target:&Target,label:&str)->Result<BlockNode,String> {
    if note.trim().is_empty() || target.block_id.trim().is_empty()
        || !matches!(target.kind,ReferenceTargetKind::Block|ReferenceTargetKind::Heading) {
        return Err("The embed target is incomplete".into());
    }
    let mut node=BlockNode::new(NodeKind::HsEmbed);
    node.attrs=json!({"refKind":"note","refValue":note,"label":label,"resolved":true,
        "target_kind":target.kind.as_str(),"target_block_id":target.block_id})
        .as_object().unwrap().iter().map(|(key,value)|(key.clone(),value.clone())).collect();
    super::document_model::schema::validate_node(&node).map_err(|error|error.to_string())?;
    // The existing traced fragment adapter assigns this occurrence's fresh block_id.
    Ok(node)
}

fn at_path<'a>(document:&'a BlockNode,path:&[usize])->Option<&'a BlockNode> {
    path.iter().try_fold(document,|node,index|node.children.get(*index).and_then(Child::as_block))
}

#[derive(Clone)]
enum Status {
    Idle,
    Loading,
    Ready { text:String,source_version:u64 },
    Missing,
    Denied,
    Failed,
}

fn source_preview(reference:&Reference,workspace:&str,document:&serde_json::Value)->Status {
    if document.get("rich_document_id").and_then(|v|v.as_str())!=Some(reference.key.note.as_str())
        || document.get("workspace_id").and_then(|v|v.as_str())!=Some(workspace) {
        return Status::Denied;
    }
    let Some(version)=document.get("doc_version").and_then(|v|v.as_u64()) else { return Status::Failed };
    let Some(content)=document.get("content_json") else { return Status::Failed };
    let Ok(content)=super::document_model::from_json_value(content) else { return Status::Failed };
    let Ok(index)=references::index(&content) else { return Status::Failed };
    let Some(path)=index.into_iter().find(|entry|entry.target == reference.target).map(|entry|entry.path)
        else { return Status::Missing };
    let Some(block)=at_path(&content,&path) else { return Status::Missing };
    let json=super::document_model::to_content_json_value(&BlockNode::doc(vec![block.clone()]));
    let text=super::wikilinks::transclusion_view::preview_text(&json,3);
    Status::Ready { text,source_version:version }
}

#[cfg(test)]
mod tests;

pub(crate) fn remove(document: &mut BlockNode, occurrence: &BlockNode, actor: &str)
    -> Result<(super::document_model::transform::TransactionReceipt, super::document_model::selection::Selection), String> {
    use super::document_model::{transform::{Step, Transaction, apply_transaction_with_trace}, ActorKind, selection::Selection};
    let path = locate_occurrence(occurrence, document)?;
    let (&index, parent) = path.split_last().ok_or("The document root is not an embed")?;
    let before = document.clone();
    let mut steps = vec![Step::DeleteNode { parent_path: parent.to_vec(), index }];
    if parent.is_empty() && document.children.len() == 1 {
        steps.push(Step::InsertNode { parent_path: vec![], index: 0, node: BlockNode::paragraph("") });
    }
    let traced = apply_transaction_with_trace(document, Transaction::new(steps, ActorKind::Operator, actor))
        .map_err(|error| error.to_string())?;
    let receipt = super::block_identity::finish_block_identity(&before, document, traced)
        .map_err(|error| error.to_string())?;
    let siblings = at_path(document, parent).ok_or("The embed parent changed")?;
    let selected = if siblings.children.is_empty() { parent.to_vec() }
        else { [parent, &[index.min(siblings.children.len() - 1)]].concat() };
    Ok((receipt, Selection::node(selected)))
}

pub(super) fn observe_preview(phase: crate::diagnostics::gui_events::GuiDiagnosticPhase) {
    use crate::diagnostics::gui_events::{record_gui_observation, GuiDiagnosticOperation};
    let _ = record_gui_observation(GuiDiagnosticOperation::NoteEmbedPreview, phase);
}

fn locate_occurrence(occurrence: &BlockNode, document: &BlockNode) -> Result<Vec<usize>, String> {
    if occurrence.kind != NodeKind::HsEmbed { return Err("The occurrence is not an embed".into()); }
    let id = occurrence.attrs.get("block_id").and_then(|v|v.as_str())
        .filter(|id| !id.trim().is_empty()).ok_or("The embed has no occurrence identity")?;
    let path = references::index(document)?.into_iter()
        .find(|entry| entry.target.block_id == id).map(|entry|entry.path)
        .ok_or("The embed occurrence is missing")?;
    if at_path(document, &path) != Some(occurrence) { return Err("The embed occurrence changed".into()); }
    Ok(path)
}
