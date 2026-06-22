//! Typed transform/step system for atomic document mutations (WP-KERNEL-012 MT-011).
//!
//! A [`Step`] is the smallest atomic edit; a [`Transaction`] is an ordered batch of
//! steps plus actor metadata. [`apply_transaction`] applies a transaction to a `doc`
//! with two hard invariants:
//!
//! 1. **Atomicity (red-team RISK-2):** the doc is CLONED before any step runs; if
//!    any step fails to apply OR the post-apply schema validation fails, the clone
//!    is restored and an `Err` is returned, so the doc never enters a corrupt
//!    intermediate state.
//! 2. **Invertibility (red-team RISK-4):** each step's inverse is computed from the
//!    OLD content captured BEFORE the step is applied, and collected into a
//!    [`TransactionReceipt`]. Re-applying the inverse steps in REVERSE order exactly
//!    restores the pre-transaction doc — the property the [`super::history`]
//!    undo manager relies on.
//!
//! All node addressing is by a `Vec<usize>` child-index path from the doc root
//! (the same path shape [`super::position::DocPosition`] uses); text addressing is
//! by CHAR range inside the leaf at the end of the path (never byte — RISK-1).

use thiserror::Error;

use super::node::{BlockNode, Child, Mark, NodeKind, TextLeaf};
use super::schema::{self, SchemaError};

/// Who/what produced a transaction. Threaded onto the [`Transaction`] so the event
/// ledger (HBR-INT) and swarm attribution (HBR-SWARM) can record provenance once
/// the editor wires transactions to the Flight Recorder in a later MT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorKind {
    /// A human operator editing in the GUI.
    Operator,
    /// An automated agent/model driving the editor via AccessKit/MCP.
    Agent,
    /// The system (migrations, programmatic load).
    System,
}

/// A single atomic document edit. Text steps address a CHAR range inside the text
/// leaf at `path`; structural steps address a node by `path`. The inverse of each
/// step is computed by `apply_transaction` from the old content, so a step does NOT
/// store its own inverse.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Insert `text` at `char_offset` inside the text leaf at `path`.
    InsertText {
        path: Vec<usize>,
        char_offset: usize,
        text: String,
    },
    /// Delete the `[start, end)` CHAR range from the text leaf at `path`.
    DeleteText {
        path: Vec<usize>,
        start: usize,
        end: usize,
    },
    /// Insert `node` as a child at `index` of the block node at `parent_path`.
    InsertNode {
        parent_path: Vec<usize>,
        index: usize,
        node: BlockNode,
    },
    /// Delete the child at `index` of the block node at `parent_path`.
    DeleteNode {
        parent_path: Vec<usize>,
        index: usize,
    },
    /// Split the text leaf inside the block at `path` at `char_offset`, producing a
    /// new sibling block of the SAME kind after it carrying the tail text. Used for
    /// "Enter splits a paragraph". `path` addresses the BLOCK (e.g. the paragraph),
    /// whose single text leaf is split.
    SplitNode { path: Vec<usize>, char_offset: usize },
    /// Merge the block at `index` of `parent_path` into its previous sibling
    /// (appending the merged node's text leaf content). Used for "Backspace at start
    /// of paragraph merges into the previous one". Requires `index >= 1`.
    MergeNodes {
        parent_path: Vec<usize>,
        index: usize,
    },
    /// Add `mark` to the text leaf at `path` (whole-run mark — MT-011 models marks
    /// at run granularity; sub-run mark ranges come with the renderer split in a
    /// later MT). Replaces an existing mark of the same type.
    AddMark { path: Vec<usize>, mark: Mark },
    /// Remove every mark of `mark`'s type from the text leaf at `path`.
    RemoveMark { path: Vec<usize>, mark: Mark },
}

/// An ordered batch of steps with actor metadata. Applied atomically.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    /// The steps, applied in order.
    pub steps: Vec<Step>,
    /// Who produced the transaction.
    pub actor_kind: ActorKind,
    /// Stable actor id (operator id, agent author_id, …). Free-form; used for
    /// attribution, not addressing.
    pub actor_id: String,
}

impl Transaction {
    /// Build a transaction from steps and actor metadata.
    pub fn new(steps: Vec<Step>, actor_kind: ActorKind, actor_id: impl Into<String>) -> Self {
        Self {
            steps,
            actor_kind,
            actor_id: actor_id.into(),
        }
    }

    /// A convenience constructor for an operator transaction.
    pub fn operator(steps: Vec<Step>) -> Self {
        Self::new(steps, ActorKind::Operator, "operator")
    }
}

/// The result of applying a [`Transaction`]: the INVERSE steps needed to undo it
/// (in reverse-of-forward order, ready to apply as-is), plus the forward steps so
/// redo can re-apply. The [`super::history`] undo manager stores these.
#[derive(Debug, Clone, PartialEq)]
pub struct TransactionReceipt {
    /// The forward steps that were applied (for redo).
    pub forward: Vec<Step>,
    /// The inverse steps, ALREADY ordered so applying them in sequence undoes the
    /// transaction (i.e. reverse of the forward application order).
    pub inverse: Vec<Step>,
    /// Actor metadata copied from the transaction.
    pub actor_kind: ActorKind,
    /// Actor id copied from the transaction.
    pub actor_id: String,
}

/// Why a transaction could not be applied. A `Schema` error means the post-apply
/// tree was invalid and the doc was rolled back; an `Addressing` error means a step
/// pointed at a non-existent path/leaf/range and the doc was rolled back.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TransformError {
    /// A step addressed a path that does not resolve to a node.
    #[error("no node at path {path:?}")]
    NoNodeAtPath { path: Vec<usize> },
    /// A step addressed a node that is not a text leaf where one was required.
    #[error("expected a text leaf at path {path:?}")]
    NotATextLeaf { path: Vec<usize> },
    /// A node-insert/delete index was out of range for the parent's children.
    #[error("child index {index} out of range for parent {parent:?} (len {len})")]
    ChildIndexOutOfRange { parent: NodeKind, index: usize, len: usize },
    /// A split/merge precondition failed (e.g. merge at index 0, or split target is
    /// not an inline-content block with a text leaf).
    #[error("structural step invalid: {reason}")]
    InvalidStructuralStep { reason: String },
    /// The post-apply tree failed schema validation; the doc was rolled back.
    #[error("schema violation after apply: {0}")]
    Schema(#[from] SchemaError),
}

/// Apply `tx` to `doc` atomically. On success, `doc` is mutated in place and a
/// [`TransactionReceipt`] is returned. On ANY error, `doc` is left byte-for-byte
/// unchanged (it is restored from a clone taken before the first step) and the error
/// is returned.
///
/// The inverse of each step is captured from the OLD content BEFORE that step runs
/// (MT impl note 3 / RISK-4), then the inverse list is reversed so it applies as a
/// single undo batch.
pub fn apply_transaction(
    doc: &mut BlockNode,
    tx: Transaction,
) -> Result<TransactionReceipt, TransformError> {
    // RISK-2 atomicity: snapshot before touching anything.
    let snapshot = doc.clone();

    let mut inverse_in_apply_order: Vec<Step> = Vec::with_capacity(tx.steps.len());

    let result = (|| -> Result<(), TransformError> {
        for step in &tx.steps {
            // Capture the inverse from current (pre-step) state, THEN apply.
            let inverse = compute_inverse(doc, step)?;
            apply_step(doc, step)?;
            inverse_in_apply_order.push(inverse);
        }
        // Post-apply structural validation (RISK-2): a step batch that leaves the
        // tree invalid is rejected wholesale.
        schema::validate_tree(doc)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            // Undo applies the inverses in reverse of the forward order.
            let mut inverse = inverse_in_apply_order;
            inverse.reverse();
            Ok(TransactionReceipt {
                forward: tx.steps,
                inverse,
                actor_kind: tx.actor_kind,
                actor_id: tx.actor_id,
            })
        }
        Err(e) => {
            // Roll back to the pre-transaction snapshot.
            *doc = snapshot;
            Err(e)
        }
    }
}

/// Compute the inverse of `step` against the CURRENT (pre-step) `doc`. The caller
/// guarantees this runs before `apply_step` for the same step, so the old content is
/// still present.
fn compute_inverse(doc: &BlockNode, step: &Step) -> Result<Step, TransformError> {
    match step {
        Step::InsertText { path, char_offset, text } => {
            // Inverse of inserting `text` is deleting the char range it occupies.
            // `RopeText::insert` CLAMPS the offset into `0..=len_chars`, so the text
            // actually lands at `min(char_offset, len)` — the inverse must address
            // the CLAMPED start, not the raw `char_offset` (which may be usize::MAX
            // for an "append at end" insert and would overflow `start + n_chars`).
            let leaf = text_leaf_at(doc, path)?;
            let start = (*char_offset).min(leaf.text.len_chars());
            let n_chars = text.chars().count();
            Ok(Step::DeleteText {
                path: path.clone(),
                start,
                end: start + n_chars,
            })
        }
        Step::DeleteText { path, start, end } => {
            // Inverse of deleting [start,end) is re-inserting the deleted text.
            let leaf = text_leaf_at(doc, path)?;
            let removed = leaf.text.slice_chars(*start, *end);
            Ok(Step::InsertText {
                path: path.clone(),
                char_offset: *start,
                text: removed,
            })
        }
        Step::InsertNode { parent_path, index, .. } => {
            // Inverse of inserting a node is deleting it.
            Ok(Step::DeleteNode {
                parent_path: parent_path.clone(),
                index: *index,
            })
        }
        Step::DeleteNode { parent_path, index } => {
            // Inverse of deleting a node is re-inserting the captured node.
            let parent = block_at(doc, parent_path)?;
            let parent_kind = parent.kind;
            let parent_len = parent.children.len();
            let child = parent
                .children
                .get(*index)
                .ok_or(TransformError::ChildIndexOutOfRange {
                    parent: parent_kind,
                    index: *index,
                    len: parent_len,
                })?;
            let node = child
                .as_block()
                .ok_or_else(|| TransformError::InvalidStructuralStep {
                    reason: format!("child {index} of {:?} is text, not a block", parent.kind),
                })?
                .clone();
            Ok(Step::InsertNode {
                parent_path: parent_path.clone(),
                index: *index,
                node,
            })
        }
        Step::SplitNode { path, .. } => {
            // Splitting block at `path` creates a sibling at parent[index+1]; the
            // inverse merges that new sibling back into `path`.
            let (parent_path, index) = split_path(path)?;
            Ok(Step::MergeNodes {
                parent_path,
                index: index + 1,
            })
        }
        Step::MergeNodes { parent_path, index } => {
            // Merging child[index] into child[index-1] appends index's text to
            // index-1; the inverse splits child[index-1] at the join point (the
            // pre-merge length of child[index-1]'s text leaf).
            let parent = block_at(doc, parent_path)?;
            if *index == 0 {
                return Err(TransformError::InvalidStructuralStep {
                    reason: "cannot merge the first child into a previous sibling".to_string(),
                });
            }
            let prev = parent
                .children
                .get(*index - 1)
                .and_then(Child::as_block)
                .ok_or_else(|| TransformError::InvalidStructuralStep {
                    reason: format!("previous sibling of index {index} is not a block"),
                })?;
            let join_at = first_text_leaf_len(prev);
            let mut prev_path = parent_path.clone();
            prev_path.push(*index - 1);
            Ok(Step::SplitNode {
                path: prev_path,
                char_offset: join_at,
            })
        }
        Step::AddMark { path, mark } => {
            // Inverse of adding a mark: if a mark of the same type already existed,
            // restoring it is the true inverse; but MT-011 models AddMark as
            // "set mark of this type", and the common case (no prior mark) inverts to
            // RemoveMark. We capture the displaced mark so undo restores it exactly.
            let leaf = text_leaf_at(doc, path)?;
            match leaf.marks.iter().find(|m| m.same_type(mark)) {
                Some(existing) => Ok(Step::AddMark {
                    path: path.clone(),
                    mark: existing.clone(),
                }),
                None => Ok(Step::RemoveMark {
                    path: path.clone(),
                    mark: mark.clone(),
                }),
            }
        }
        Step::RemoveMark { path, mark } => {
            // Inverse of removing a mark: re-add the exact mark that was present (so
            // a link's href survives the round-trip — RISK-3). If none was present,
            // the inverse is a no-op AddMark of the requested mark, which undo will
            // simply add; to keep undo lossless we capture the actual present mark.
            let leaf = text_leaf_at(doc, path)?;
            match leaf.marks.iter().find(|m| m.same_type(mark)) {
                Some(existing) => Ok(Step::AddMark {
                    path: path.clone(),
                    mark: existing.clone(),
                }),
                None => Ok(Step::RemoveMark {
                    path: path.clone(),
                    mark: mark.clone(),
                }),
            }
        }
    }
}

/// Apply `step` to `doc` in place. Pure mechanics — atomicity/validation are the
/// caller's job. Returns an addressing error if a path/index/range is invalid.
fn apply_step(doc: &mut BlockNode, step: &Step) -> Result<(), TransformError> {
    match step {
        Step::InsertText { path, char_offset, text } => {
            let leaf = text_leaf_at_mut(doc, path)?;
            leaf.text.insert(*char_offset, text);
            Ok(())
        }
        Step::DeleteText { path, start, end } => {
            let leaf = text_leaf_at_mut(doc, path)?;
            leaf.text.remove(*start, *end);
            Ok(())
        }
        Step::InsertNode { parent_path, index, node } => {
            let parent = block_at_mut(doc, parent_path)?;
            if *index > parent.children.len() {
                return Err(TransformError::ChildIndexOutOfRange {
                    parent: parent.kind,
                    index: *index,
                    len: parent.children.len(),
                });
            }
            parent.children.insert(*index, Child::Block(node.clone()));
            Ok(())
        }
        Step::DeleteNode { parent_path, index } => {
            let parent = block_at_mut(doc, parent_path)?;
            if *index >= parent.children.len() {
                return Err(TransformError::ChildIndexOutOfRange {
                    parent: parent.kind,
                    index: *index,
                    len: parent.children.len(),
                });
            }
            parent.children.remove(*index);
            Ok(())
        }
        Step::SplitNode { path, char_offset } => apply_split(doc, path, *char_offset),
        Step::MergeNodes { parent_path, index } => apply_merge(doc, parent_path, *index),
        Step::AddMark { path, mark } => {
            let leaf = text_leaf_at_mut(doc, path)?;
            leaf.add_mark(mark.clone());
            Ok(())
        }
        Step::RemoveMark { path, mark } => {
            let leaf = text_leaf_at_mut(doc, path)?;
            leaf.remove_marks_of_type(mark);
            Ok(())
        }
    }
}

/// Split the inline-content block at `path` at `char_offset`: the block keeps the
/// head `[0, char_offset)`, and a new sibling of the SAME kind+attrs holding the
/// tail `[char_offset, len)` is inserted right after it.
fn apply_split(doc: &mut BlockNode, path: &[usize], char_offset: usize) -> Result<(), TransformError> {
    let (parent_path, index) = split_path(path)?;
    // First, build the tail node from the target (immutable read), then mutate.
    let tail_node = {
        let target = block_at(doc, path)?;
        if !target.kind.holds_inline_content() {
            return Err(TransformError::InvalidStructuralStep {
                reason: format!("cannot split non-inline-content block {:?}", target.kind),
            });
        }
        // The target's first text leaf is split; the tail leaf inherits its marks.
        let (tail_text, tail_marks) = match target.children.first().and_then(Child::as_text) {
            Some(leaf) => (
                leaf.text.slice_chars(char_offset, leaf.text.len_chars()),
                leaf.marks.clone(),
            ),
            None => (String::new(), Vec::new()),
        };
        let mut tail = BlockNode::new(target.kind);
        tail.attrs = target.attrs.clone();
        tail.children
            .push(Child::Text(TextLeaf::with_marks(&tail_text, tail_marks)));
        tail
    };
    // Truncate the target's text to the head, then insert the tail as a sibling.
    {
        let target = block_at_mut(doc, path)?;
        if let Some(leaf) = target.children.first_mut().and_then(Child::as_text_mut) {
            let len = leaf.text.len_chars();
            leaf.text.remove(char_offset, len);
        }
    }
    let parent = block_at_mut(doc, &parent_path)?;
    parent.children.insert(index + 1, Child::Block(tail_node));
    Ok(())
}

/// Merge child `index` of the block at `parent_path` into child `index-1` by
/// appending the merged node's first text-leaf content to the previous node's first
/// text leaf, then removing the merged node.
fn apply_merge(doc: &mut BlockNode, parent_path: &[usize], index: usize) -> Result<(), TransformError> {
    if index == 0 {
        return Err(TransformError::InvalidStructuralStep {
            reason: "cannot merge the first child into a previous sibling".to_string(),
        });
    }
    // Read the merged node's text first.
    let merged_text = {
        let parent = block_at(doc, parent_path)?;
        let merged = parent.children.get(index).and_then(Child::as_block).ok_or_else(|| {
            TransformError::InvalidStructuralStep {
                reason: format!("child {index} is not a block to merge"),
            }
        })?;
        merged
            .children
            .first()
            .and_then(Child::as_text)
            .map(|l| l.text.to_string())
            .unwrap_or_default()
    };
    // Append it to the previous sibling's first text leaf.
    {
        let parent = block_at_mut(doc, parent_path)?;
        let prev = parent
            .children
            .get_mut(index - 1)
            .and_then(Child::as_block_mut)
            .ok_or_else(|| TransformError::InvalidStructuralStep {
                reason: format!("previous sibling of index {index} is not a block"),
            })?;
        match prev.children.first_mut().and_then(Child::as_text_mut) {
            Some(leaf) => {
                let at = leaf.text.len_chars();
                leaf.text.insert(at, &merged_text);
            }
            None => {
                prev.children.push(Child::Text(TextLeaf::new(&merged_text)));
            }
        }
    }
    // Remove the merged node.
    let parent = block_at_mut(doc, parent_path)?;
    if index >= parent.children.len() {
        return Err(TransformError::ChildIndexOutOfRange {
            parent: parent.kind,
            index,
            len: parent.children.len(),
        });
    }
    parent.children.remove(index);
    Ok(())
}

/// Split a node path into (parent_path, index_within_parent). Errors on the empty
/// path (the root has no parent).
fn split_path(path: &[usize]) -> Result<(Vec<usize>, usize), TransformError> {
    match path.split_last() {
        Some((&last, head)) => Ok((head.to_vec(), last)),
        None => Err(TransformError::InvalidStructuralStep {
            reason: "cannot address the root with a structural step".to_string(),
        }),
    }
}

/// The char length of the first text leaf of `node` (0 if it has none).
fn first_text_leaf_len(node: &BlockNode) -> usize {
    node.children
        .first()
        .and_then(Child::as_text)
        .map(|l| l.text.len_chars())
        .unwrap_or(0)
}

/// Resolve a child-index `path` to a shared block reference. The empty path is the
/// root `doc` itself.
fn block_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Result<&'a BlockNode, TransformError> {
    let mut node = doc;
    for &idx in path {
        node = node
            .children
            .get(idx)
            .and_then(Child::as_block)
            .ok_or_else(|| TransformError::NoNodeAtPath { path: path.to_vec() })?;
    }
    Ok(node)
}

/// Resolve a child-index `path` to a mutable block reference. The empty path is the
/// root `doc` itself.
fn block_at_mut<'a>(doc: &'a mut BlockNode, path: &[usize]) -> Result<&'a mut BlockNode, TransformError> {
    let mut node = doc;
    for &idx in path {
        node = node
            .children
            .get_mut(idx)
            .and_then(Child::as_block_mut)
            .ok_or_else(|| TransformError::NoNodeAtPath { path: path.to_vec() })?;
    }
    Ok(node)
}

/// Resolve a `path` whose LAST element addresses a text leaf inside the block at the
/// preceding path, returning a shared reference to that leaf.
fn text_leaf_at<'a>(doc: &'a BlockNode, path: &[usize]) -> Result<&'a TextLeaf, TransformError> {
    let (parent_path, leaf_idx) = split_path(path).map_err(|_| TransformError::NotATextLeaf {
        path: path.to_vec(),
    })?;
    let parent = block_at(doc, &parent_path)?;
    parent
        .children
        .get(leaf_idx)
        .and_then(Child::as_text)
        .ok_or_else(|| TransformError::NotATextLeaf { path: path.to_vec() })
}

/// Resolve a `path` whose LAST element addresses a text leaf, returning a mutable
/// reference to that leaf.
fn text_leaf_at_mut<'a>(doc: &'a mut BlockNode, path: &[usize]) -> Result<&'a mut TextLeaf, TransformError> {
    let (parent_path, leaf_idx) = split_path(path).map_err(|_| TransformError::NotATextLeaf {
        path: path.to_vec(),
    })?;
    let parent = block_at_mut(doc, &parent_path)?;
    let parent_kind = parent.kind;
    let len = parent.children.len();
    parent
        .children
        .get_mut(leaf_idx)
        .ok_or(TransformError::ChildIndexOutOfRange {
            parent: parent_kind,
            index: leaf_idx,
            len,
        })?
        .as_text_mut()
        .ok_or_else(|| TransformError::NotATextLeaf { path: path.to_vec() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::node::NodeKind;

    fn doc_two_paras() -> BlockNode {
        BlockNode::doc(vec![BlockNode::paragraph("hello"), BlockNode::paragraph("world")])
    }

    #[test]
    fn insert_text_then_inverse_round_trips() {
        let mut doc = BlockNode::doc(vec![BlockNode::paragraph("hello")]);
        let before = doc.clone();
        let tx = Transaction::operator(vec![Step::InsertText {
            path: vec![0, 0],
            char_offset: 5,
            text: " world".to_string(),
        }]);
        let receipt = apply_transaction(&mut doc, tx).unwrap();
        assert_eq!(
            doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
            "hello world"
        );
        // Apply the inverse to undo.
        let undo = Transaction::operator(receipt.inverse);
        apply_transaction(&mut doc, undo).unwrap();
        assert_eq!(doc, before);
    }

    #[test]
    fn schema_violation_rolls_back() {
        let mut doc = BlockNode::doc(vec![BlockNode::paragraph("hi")]);
        let before = doc.clone();
        // Insert a paragraph as a child of a paragraph's INLINE content -> invalid.
        let tx = Transaction::operator(vec![Step::InsertNode {
            parent_path: vec![0], // the paragraph
            index: 1,
            node: BlockNode::paragraph("nested"),
        }]);
        let err = apply_transaction(&mut doc, tx).unwrap_err();
        assert!(matches!(err, TransformError::Schema(_)));
        assert_eq!(doc, before, "doc must be unchanged after a rejected tx");
    }

    #[test]
    fn split_then_merge_inverse() {
        let mut doc = doc_two_paras();
        // Combine the two paras into "helloworld" via merge, then check inverse split.
        let before = doc.clone();
        let merge = Transaction::operator(vec![Step::MergeNodes { parent_path: vec![], index: 1 }]);
        let receipt = apply_transaction(&mut doc, merge).unwrap();
        // After merge: one paragraph "helloworld".
        assert_eq!(doc.children.len(), 1);
        assert_eq!(
            doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
            "helloworld"
        );
        // Inverse restores the two paragraphs.
        apply_transaction(&mut doc, Transaction::operator(receipt.inverse)).unwrap();
        assert_eq!(doc, before);
    }

    #[test]
    fn split_paragraph_creates_sibling() {
        let mut doc = BlockNode::doc(vec![BlockNode::paragraph("helloworld")]);
        let tx = Transaction::operator(vec![Step::SplitNode { path: vec![0], char_offset: 5 }]);
        apply_transaction(&mut doc, tx).unwrap();
        assert_eq!(doc.children.len(), 2);
        assert_eq!(
            doc.children[0].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
            "hello"
        );
        assert_eq!(
            doc.children[1].as_block().unwrap().children[0].as_text().unwrap().text.to_string(),
            "world"
        );
        assert_eq!(doc.children[1].as_block().unwrap().kind, NodeKind::Paragraph);
    }
}
