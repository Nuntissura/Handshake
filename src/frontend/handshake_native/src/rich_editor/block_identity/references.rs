//! Host references use persisted block attributes within the authorized owning note.
use crate::rich_editor::document_model::{BlockNode, Child, HsLinkNode, ReferenceTargetKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub block_id: String,
    pub kind: ReferenceTargetKind,
}

pub(crate) struct IndexedBlock {
    pub target: Target,
    pub path: Vec<usize>,
    pub label: String,
}

pub(crate) fn index(document: &BlockNode) -> Result<Vec<IndexedBlock>, String> {
    crate::rich_editor::document_model::validate_tree(document).map_err(|error| error.to_string())?;
    let mut ids = std::collections::HashSet::new();
    let mut indexed = Vec::new();
    for (path, block) in super::blocks(document) {
        if path.is_empty() { continue; }
        let Some(id) = super::block_id(block).map_err(|error| error.to_string())?
            .filter(|id| !id.trim().is_empty()) else { continue };
        if !ids.insert(id.to_owned()) { return Err("The note contains ambiguous persisted block identities".into()); }
        let kind = if block.kind.to_json_type() == "heading" {
            ReferenceTargetKind::Heading
        } else { ReferenceTargetKind::Block };
        let label = crate::rich_editor::save::export::export_plain_text(&BlockNode::doc(vec![block.clone()]));
        indexed.push(IndexedBlock { target: Target { block_id: id.to_owned(), kind }, path, label });
    }
    Ok(indexed)
}

pub(crate) fn structural_link(link: &HsLinkNode) -> bool {
    link.target_block_id.is_some() || matches!(link.target_kind,
        Some(ReferenceTargetKind::Heading | ReferenceTargetKind::Block))
}

impl Target {
    pub(crate) fn from_link(link: &HsLinkNode) -> Result<Self, String> {
        if link.ref_kind != "note" || !link.resolved || link.ref_value.trim().is_empty() {
            return Err("The note reference is unavailable".into());
        }
        let kind = link.target_kind.filter(|kind| matches!(kind,
            ReferenceTargetKind::Heading | ReferenceTargetKind::Block))
            .ok_or("The note reference has no supported target kind")?;
        let block_id = link.target_block_id.as_ref().filter(|id| !id.trim().is_empty())
            .ok_or("The note reference has no persisted block identity")?.clone();
        Ok(Self { block_id, kind })
    }

    pub(crate) fn link(&self, note_id: &str, label: &str) -> HsLinkNode {
        let mut link = HsLinkNode::new("note", note_id, label);
        link.target_kind = Some(self.kind);
        link.target_block_id = Some(self.block_id.clone());
        link
    }

    pub(crate) fn locate(&self, document: &BlockNode) -> Result<Vec<usize>, String> {
        index(document)?.into_iter().find(|block| block.target == *self).map(|block| block.path)
            .ok_or_else(|| "The referenced block or heading is missing or changed".into())
    }
}

pub(crate) fn contains_link(document: &BlockNode, expected: &HsLinkNode) -> bool {
    super::blocks(document).iter().any(|(_, block)| block.children.iter()
        .any(|child| matches!(child, Child::HsLink(link) if link == expected)))
}

#[cfg(test)]
mod tests;
