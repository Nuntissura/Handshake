//! MT-009 diff-engine line-level proofs (WP-KERNEL-012 E1 code editor).
//!
//! AC-001 / PT-001 (`cargo test -p handshake-native diff_engine`): port the diff test cases from
//! `app/src/lib/editor/document_diff_merge.test.ts` as Rust integration tests over the public
//! [`DiffEngine`] API:
//!   - identical texts -> all Equal blocks
//!   - added line -> one Added block
//!   - removed line -> one Removed block
//!   - modified line -> one Modified block
//!
//! These run against the real `similar`-backed Myers diff (not a mock) so removing the grouping
//! logic in `diff_engine.rs` makes them fail (anti-tautology — they assert the exact block shape the
//! panel renders + the sync-scroll line map consumes).

use handshake_native::code_editor::{diff_json_blocks, DiffEngine, DiffStatus, TextBuffer};

#[test]
fn diff_engine_identical_texts_all_equal() {
    let left = TextBuffer::new("alpha\nbeta\ngamma");
    let right = TextBuffer::new("alpha\nbeta\ngamma");
    let blocks = DiffEngine::diff(&left, &right);
    assert!(
        !blocks.is_empty(),
        "a non-empty diff must produce at least one block"
    );
    assert!(
        blocks.iter().all(|b| b.status == DiffStatus::Equal),
        "AC-001: identical texts -> all Equal blocks; got {blocks:?}"
    );
    let equal_lines: usize = blocks.iter().map(|b| b.left_lines.len()).sum();
    assert_eq!(equal_lines, 3, "AC-001: all 3 lines covered as Equal");
    println!(
        "AC-001 identical: {} Equal block(s) covering 3 lines",
        blocks.len()
    );
}

#[test]
fn diff_engine_added_line_one_added_block() {
    let left = TextBuffer::new("alpha\nbeta\ngamma");
    let right = TextBuffer::new("alpha\nINSERTED\nbeta\ngamma");
    let blocks = DiffEngine::diff(&left, &right);
    let added: Vec<_> = blocks
        .iter()
        .filter(|b| b.status == DiffStatus::Added)
        .collect();
    assert_eq!(
        added.len(),
        1,
        "AC-001: added line -> exactly one Added block; got {blocks:?}"
    );
    assert_eq!(
        added[0].left_lines.len(),
        0,
        "AC-001: Added block empty on the left"
    );
    assert_eq!(
        added[0].right_lines.len(),
        1,
        "AC-001: Added block covers one right line"
    );
    println!(
        "AC-001 added: one Added block at right lines {:?}",
        added[0].right_lines
    );
}

#[test]
fn diff_engine_removed_line_one_removed_block() {
    let left = TextBuffer::new("alpha\nDOOMED\nbeta\ngamma");
    let right = TextBuffer::new("alpha\nbeta\ngamma");
    let blocks = DiffEngine::diff(&left, &right);
    let removed: Vec<_> = blocks
        .iter()
        .filter(|b| b.status == DiffStatus::Removed)
        .collect();
    assert_eq!(
        removed.len(),
        1,
        "AC-001: removed line -> exactly one Removed block; got {blocks:?}"
    );
    assert_eq!(
        removed[0].right_lines.len(),
        0,
        "AC-001: Removed block empty on the right"
    );
    assert_eq!(
        removed[0].left_lines.len(),
        1,
        "AC-001: Removed block covers one left line"
    );
    println!(
        "AC-001 removed: one Removed block at left lines {:?}",
        removed[0].left_lines
    );
}

#[test]
fn diff_engine_modified_line_one_modified_block() {
    let left = TextBuffer::new("alpha\nOLD_VALUE\ngamma");
    let right = TextBuffer::new("alpha\nNEW_VALUE\ngamma");
    let blocks = DiffEngine::diff(&left, &right);
    let modified: Vec<_> = blocks
        .iter()
        .filter(|b| b.status == DiffStatus::Modified)
        .collect();
    assert_eq!(
        modified.len(),
        1,
        "AC-001: a delete+insert at the same position is ONE Modified block; got {blocks:?}"
    );
    assert_eq!(
        modified[0].left_lines.len(),
        1,
        "AC-001: Modified covers one left line"
    );
    assert_eq!(
        modified[0].right_lines.len(),
        1,
        "AC-001: Modified covers one right line"
    );
    // And there is NO separate Added or Removed block for the modify (it is fused).
    assert!(
        !blocks
            .iter()
            .any(|b| b.status == DiffStatus::Added || b.status == DiffStatus::Removed),
        "AC-001: a single-line modify does not split into Added + Removed; got {blocks:?}"
    );
    println!("AC-001 modified: one Modified block (fused delete+insert)");
}

#[test]
fn diff_engine_ranges_tile_both_buffers() {
    // RISK-002 basis: the left/right ranges must tile each buffer contiguously so the sync-scroll
    // line map and the background rects align. Mixed add/remove/modify/equal.
    let left = TextBuffer::new("keep1\nremoveme\nold\nkeep2\nkeep3");
    let right = TextBuffer::new("keep1\nnew\nkeep2\nkeep3\nadded");
    let blocks = DiffEngine::diff(&left, &right);
    let mut nl = 0usize;
    let mut nr = 0usize;
    for b in &blocks {
        assert_eq!(b.left_lines.start, nl, "left contiguous: {blocks:?}");
        assert_eq!(b.right_lines.start, nr, "right contiguous: {blocks:?}");
        nl = b.left_lines.end;
        nr = b.right_lines.end;
    }
    assert_eq!(nl, 5, "left fully tiled (5 lines)");
    assert_eq!(nr, 5, "right fully tiled (5 lines)");
    println!("RISK-002: diff ranges tile both 5-line buffers without gaps");
}

#[test]
fn diff_engine_json_block_seam_deferred_path() {
    // The deferred prose/JSON block seam (E2, MT-011+) classifies by index. Proven here so the seam
    // is real (design exists) without blocking on the not-yet-built rich-document format.
    let left = vec![serde_json::json!({"type":"p","text":"a"})];
    let right = vec![
        serde_json::json!({"type":"p","text":"a"}),
        serde_json::json!({"type":"p","text":"b"}),
    ];
    let blocks = diff_json_blocks(&left, &right);
    assert_eq!(blocks[0].status, DiffStatus::Equal);
    assert_eq!(blocks[1].status, DiffStatus::Added);
    println!("Deferred JSON-block seam present and index-aligned (E2 will replace with format-aware alignment)");
}
