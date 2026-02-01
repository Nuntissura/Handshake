use handshake_core::ace::validators::atelier_scope::{
    apply_selection_bounded_patchsets, sha256_hex, DocPatchsetV1, PatchOpV1, RangeUtf8,
    SelectionRangeV1,
};

fn selection_v1(doc_text: &str, start: usize, end: usize) -> SelectionRangeV1 {
    let doc_hash = sha256_hex(doc_text.as_bytes());
    let selection_hash = sha256_hex(doc_text[start..end].as_bytes());
    SelectionRangeV1 {
        schema_version: "hsk.selection_range@v1".to_string(),
        surface: "docs".to_string(),
        coordinate_space: "doc_text_utf8_v1".to_string(),
        start_utf8: start,
        end_utf8: end,
        doc_preimage_sha256: doc_hash,
        selection_preimage_sha256: selection_hash,
    }
}

#[test]
fn applies_in_range_replace_without_touching_outside_selection() {
    let doc_text_before = "Hello world\nSecond line";
    let selection = selection_v1(doc_text_before, 6, 11); // "world"

    let patchset = DocPatchsetV1 {
        schema_version: "hsk.doc_patchset@v1".to_string(),
        doc_id: "doc-1".to_string(),
        selection: selection.clone(),
        boundary_normalization: "disabled".to_string(),
        ops: vec![PatchOpV1::ReplaceRange {
            range_utf8: RangeUtf8 { start: 0, end: 5 },
            insert_text: "earth".to_string(),
        }],
        summary: None,
    };

    let doc_text_after =
        apply_selection_bounded_patchsets(doc_text_before, &selection, &[patchset]).unwrap();

    assert_eq!(doc_text_after, "Hello earth\nSecond line");
    assert!(doc_text_after.starts_with("Hello "));
    assert!(doc_text_after.ends_with("\nSecond line"));
}

#[test]
fn rejects_patchset_selection_mismatch() {
    let doc_text_before = "Hello world\nSecond line";
    let selection = selection_v1(doc_text_before, 6, 11);

    let mut mismatched = selection.clone();
    mismatched.end_utf8 = 12;

    let patchset = DocPatchsetV1 {
        schema_version: "hsk.doc_patchset@v1".to_string(),
        doc_id: "doc-1".to_string(),
        selection: mismatched,
        boundary_normalization: "disabled".to_string(),
        ops: vec![PatchOpV1::ReplaceRange {
            range_utf8: RangeUtf8 { start: 0, end: 5 },
            insert_text: "earth".to_string(),
        }],
        summary: None,
    };

    let err =
        apply_selection_bounded_patchsets(doc_text_before, &selection, &[patchset]).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("patchset selection does not match request selection"),
        "unexpected error: {message}"
    );
}

#[test]
fn rejects_doc_preimage_hash_mismatch() {
    let doc_text_before = "Hello world\nSecond line";
    let mut selection = selection_v1(doc_text_before, 6, 11);
    selection.doc_preimage_sha256 = "0".repeat(64);

    let patchset = DocPatchsetV1 {
        schema_version: "hsk.doc_patchset@v1".to_string(),
        doc_id: "doc-1".to_string(),
        selection: selection.clone(),
        boundary_normalization: "disabled".to_string(),
        ops: vec![PatchOpV1::ReplaceRange {
            range_utf8: RangeUtf8 { start: 0, end: 5 },
            insert_text: "earth".to_string(),
        }],
        summary: None,
    };

    let err =
        apply_selection_bounded_patchsets(doc_text_before, &selection, &[patchset]).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("doc_preimage_sha256 mismatch"),
        "unexpected error: {message}"
    );
}
