//! Host block identities are settled before the original prepared edit commits.
use super::{finish_block_identity, fresh_template_copy, invalid};
use crate::rich_editor::document_model::{
    replace_selection_with_fragment_with_trace, validate_tree, ActorKind, BlockNode, Child,
    FragmentReplaceError, FragmentReplaceOptions, FragmentReplacement, RichFragment, Selection,
    TracedTransactionReceipt, TransformError,
};

pub(crate) fn replace_fragment(
    document: &mut BlockNode,
    selection: &Selection,
    fragment: RichFragment,
    options: FragmentReplaceOptions,
) -> Result<FragmentReplacement, FragmentReplaceError> {
    let before = document.clone();
    let applied = replace_selection_with_fragment_with_trace(document, selection, fragment, options)?;
    let traced = TracedTransactionReceipt { receipt: applied.receipt, trace: applied.trace };
    match finish_block_identity(&before, document, traced) {
        Ok(receipt) => Ok(FragmentReplacement { receipt, selection: applied.selection }),
        Err(error) => {
            *document = before;
            Err(error.into())
        }
    }
}

pub(crate) fn replace_template(
    document: &mut BlockNode,
    selection: &Selection,
    template: &BlockNode,
    actor_kind: ActorKind,
    actor_id: String,
) -> Result<FragmentReplacement, FragmentReplaceError> {
    validate_tree(template).map_err(TransformError::from)?;
    let copied = fresh_template_copy(template, document)?;
    let blocks = copied.children.into_iter().map(|child| match child {
        Child::Block(block) => Ok(block),
        _ => Err(invalid("template document must contain blocks")),
    }).collect::<Result<Vec<_>, _>>()?;
    let split_tail_attrs = match selection {
        Selection::Text { anchor, .. } => {
            // The canonical operation uses these attrs only when both sides of
            // this same block survive. Identity is assigned with the final edit.
            let block = anchor.path.split_last().and_then(|(_, parent)| {
                parent.iter().try_fold(&*document, |node, &index|
                    node.children.get(index).and_then(Child::as_block))
            });
            match block {
                Some(block) => {
                    let mut attrs = block.attrs.clone();
                    attrs.remove("block_id");
                    Some(attrs)
                }
                None => None,
            }
        }
        Selection::Node { .. } => None,
    };
    replace_fragment(document, selection, RichFragment::Blocks(blocks), FragmentReplaceOptions {
        actor_kind, actor_id, split_tail_attrs,
    })
}
