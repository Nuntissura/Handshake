//! Portal-compatible block identity policy, completed inside a prepared document edit.
use std::collections::{HashMap, HashSet};

use super::document_model::{
    node::{BlockNode, Child},
    path_trace::TracedTransactionReceipt,
    transform::{apply_transaction, Step, Transaction, TransactionReceipt, TransformError},
};
use serde_json::Value;

pub(crate) mod fragment;
pub(crate) mod references;

pub(crate) fn fresh_template_copy(template: &BlockNode, destination: &BlockNode)
    -> Result<BlockNode, TransformError> {
    copy_with_generator(template, destination, &mut || uuid::Uuid::new_v4().to_string())
}

/// Call before pushing the receipt and committing the original prepared owner candidate.
pub(crate) fn finish_block_identity(before: &BlockNode, edited: &mut BlockNode,
    traced: TracedTransactionReceipt) -> Result<TransactionReceipt, TransformError> {
    finish_with_generator(before, edited, traced, &mut || uuid::Uuid::new_v4().to_string())
}

fn identity_block(node: &BlockNode) -> bool {
    matches!(node.kind.to_json_type(), "paragraph" | "heading" | "blockquote" | "bulletList"
        | "orderedList" | "listItem" | "taskList" | "taskItem" | "codeBlock"
        | "horizontalRule" | "table" | "tableRow" | "tableCell" | "tableHeader"
        | "callout" | "hsEmbed")
}

fn block_id(node: &BlockNode) -> Result<Option<&str>, TransformError> {
    match node.attrs.get("block_id") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(id)) => Ok((!id.is_empty()).then_some(id.as_str())),
        Some(_) => Err(invalid("block_id must be a string or null")),
    }
}

fn blocks(document: &BlockNode) -> Vec<(Vec<usize>, &BlockNode)> {
    fn visit<'a>(node: &'a BlockNode, path: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, &'a BlockNode)>) {
        out.push((path.clone(), node));
        for (index, child) in node.children.iter().enumerate() {
            if let Child::Block(block) = child {
                path.push(index);
                visit(block, path, out);
                path.pop();
            }
        }
    }
    let mut out = Vec::new();
    visit(document, &mut Vec::new(), &mut out);
    out
}

fn collect_ids(document: &BlockNode, ids: &mut HashSet<String>) -> Result<(), TransformError> {
    for (_, block) in blocks(document) {
        if let Some(id) = block_id(block)? { ids.insert(id.to_owned()); }
    }
    Ok(())
}

fn fresh_id(ids: &mut HashSet<String>, generate: &mut impl FnMut() -> String)
    -> Result<String, TransformError> {
    for _ in 0..16 {
        let id = generate();
        if !id.is_empty() && ids.insert(id.clone()) { return Ok(id); }
    }
    Err(invalid("could not allocate a fresh block identity"))
}

fn copy_with_generator(template: &BlockNode, destination: &BlockNode,
    generate: &mut impl FnMut() -> String) -> Result<BlockNode, TransformError> {
    let mut occupied = HashSet::new();
    collect_ids(template, &mut occupied)?;
    collect_ids(destination, &mut occupied)?;
    let paths: Vec<_> = blocks(template).into_iter()
        .filter_map(|(path, block)| block_id(block).ok().flatten().map(|_| path)).collect();
    let mut copy = template.clone();
    for path in paths {
        let id = fresh_id(&mut occupied, generate)?;
        block_at_mut(&mut copy, &path)?.attrs.insert("block_id".into(), Value::String(id));
    }
    Ok(copy)
}

fn finish_with_generator(before: &BlockNode, edited: &mut BlockNode,
    traced: TracedTransactionReceipt, generate: &mut impl FnMut() -> String)
    -> Result<TransactionReceipt, TransformError> {
    let TracedTransactionReceipt { mut receipt, trace } = traced;
    if receipt.forward.is_empty() { return Ok(receipt); }
    let mut originals = HashMap::new();
    for (path, block) in blocks(before) {
        if let Some(id) = block_id(block)? {
            if let Some(mapped) = trace.map_block_path(&path) {
                originals.insert(id.to_owned(), mapped);
            }
        }
    }
    let mut positions: HashMap<String, Vec<Vec<usize>>> = HashMap::new();
    let mut occupied = HashSet::new();
    collect_ids(edited, &mut occupied)?;
    for (path, block) in blocks(edited) {
        if identity_block(block) {
            if let Some(id) = block_id(block)? { positions.entry(id.to_owned()).or_default().push(path); }
        }
    }
    let mut fixes = Vec::new();
    for (path, block) in blocks(edited) {
        if !identity_block(block) { continue; }
        let keep = block_id(block)?.and_then(|id| positions.get(id).map(|group| {
            originals.get(id).filter(|original| group.contains(original)).unwrap_or(&group[0])
        }));
        if keep == Some(&path) { continue; }
        let mut replacement = block.clone();
        replacement.attrs.insert("block_id".into(), Value::String(fresh_id(&mut occupied, generate)?));
        let (&index, parent) = path.split_last().ok_or_else(|| invalid("document root cannot receive a block identity"))?;
        fixes.push(Step::DeleteNode { parent_path: parent.to_vec(), index });
        fixes.push(Step::InsertNode { parent_path: parent.to_vec(), index, node: replacement });
    }
    if fixes.is_empty() { return Ok(receipt); }
    let mut candidate = edited.clone();
    let correction = apply_transaction(&mut candidate,
        Transaction::new(fixes, receipt.actor_kind.clone(), receipt.actor_id.clone()))?;
    receipt.forward.extend(correction.forward);
    let mut inverse = correction.inverse;
    inverse.extend(receipt.inverse);
    receipt.inverse = inverse;
    *edited = candidate;
    Ok(receipt)
}

fn block_at_mut<'a>(mut document: &'a mut BlockNode, path: &[usize]) -> Result<&'a mut BlockNode, TransformError> {
    for &index in path {
        document = document.children.get_mut(index).and_then(Child::as_block_mut)
            .ok_or_else(|| TransformError::NoNodeAtPath { path: path.to_vec() })?;
    }
    Ok(document)
}

fn invalid(reason: &str) -> TransformError {
    TransformError::InvalidStructuralStep { reason: reason.into() }
}

#[cfg(test)]
mod tests;
