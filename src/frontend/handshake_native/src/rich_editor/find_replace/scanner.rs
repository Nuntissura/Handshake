//! Host find-scanner API backed by the canonical document model.

pub use crate::rich_editor::document_model::find_scan::{
    scan, FindMatch, FindQuery, FindScanResult, MatchKind, MAX_MATCHES, REGEX_SIZE_LIMIT,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rich_editor::document_model::node::{BlockNode, Child, NodeKind, TextLeaf};

    fn multi_para_doc() -> BlockNode {
        // doc > [ "foo bar foo", "a foo here", code_block("let foo = foo;") ]
        BlockNode::doc(vec![
            BlockNode::paragraph("foo bar foo"),
            BlockNode::paragraph("a foo here"),
            BlockNode::with_children(
                NodeKind::CodeBlock,
                vec![Child::Text(TextLeaf::new("let foo = foo;"))],
            ),
        ])
    }

    #[test]
    fn test_replace_all_single_undo() {
        // PT-4 (contract proof_target `find_replace::scanner::test_replace_all_single_undo`):
        // Replace All replaces every match across the document in ONE Transaction, so a SINGLE undo
        // restores all of them at once. Exercises the full scan -> replace_all -> undo round-trip.
        use crate::rich_editor::document_model::history::UndoManager;
        use crate::rich_editor::document_model::position::DocPosition;
        use crate::rich_editor::document_model::selection::Selection;
        use crate::rich_editor::find_replace::replace_all;

        let doc0 = multi_para_doc();
        let mut doc = doc0.clone();
        let mut undo = UndoManager::new();
        let mut sel = Selection::caret(DocPosition::new(vec![0, 0], 0));
        let result = scan(&doc, &FindQuery::literal("foo"));
        assert_eq!(result.len(), 5, "five 'foo' across prose + code");

        let n = replace_all(&mut doc, &mut undo, &mut sel, &result.matches, "X");
        assert_eq!(n, 5, "all five matches replaced in one pass");
        // Confirm the replacements landed (first paragraph + code block).
        let para0 = doc.children[0].as_block().unwrap().children[0]
            .as_text()
            .unwrap()
            .text
            .to_string();
        assert_eq!(para0, "X bar X");
        let code = doc.children[2].as_block().unwrap().children[0]
            .as_text()
            .unwrap()
            .text
            .to_string();
        assert_eq!(code, "let X = X;");

        // A SINGLE undo restores the WHOLE document — replace-all is exactly one undo entry.
        assert!(undo.undo(&mut doc).unwrap(), "one undo applies");
        assert_eq!(
            doc, doc0,
            "a single Ctrl+Z restores every replacement at once"
        );
        assert!(
            !undo.can_undo(),
            "replace-all consumed exactly one undo step"
        );
    }

}
