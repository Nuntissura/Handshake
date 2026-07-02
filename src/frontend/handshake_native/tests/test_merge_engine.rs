//! MT-009 three-way merge-engine proofs (WP-KERNEL-012 E1 code editor).
//!
//! AC-002 / PT-002 (`cargo test -p handshake-native merge_engine`): port the merge test cases from
//! `app/src/lib/editor/document_diff_merge.test.ts` as Rust integration tests over the public
//! [`MergeEngine`] API:
//!   - local adds line 2, remote adds line 5 to the same base -> two LocalOnly / RemoteOnly blocks
//!     (no conflict)
//!   - both modify line 3 differently -> one Conflict block
//!
//! RISK-003: the apply() merged-buffer correctness is also proven here (Accept Local/Remote/Both)
//! so a three-way-merge bug surfaces before the panel ships.

use handshake_native::code_editor::{MergeChoice, MergeEngine, MergeStatus, TextBuffer};

#[test]
fn merge_engine_non_overlapping_edits_no_conflict() {
    // local edits line index 2; remote edits line index 5 (the contract's "line 2" / "line 5").
    let base = TextBuffer::new("l0\nl1\nl2\nl3\nl4\nl5");
    let local = TextBuffer::new("l0\nl1\nLOCAL\nl3\nl4\nl5");
    let remote = TextBuffer::new("l0\nl1\nl2\nl3\nl4\nREMOTE");
    let blocks = MergeEngine::three_way(&base, &local, &remote);

    let local_only = blocks
        .iter()
        .filter(|b| b.status == MergeStatus::LocalOnly)
        .count();
    let remote_only = blocks
        .iter()
        .filter(|b| b.status == MergeStatus::RemoteOnly)
        .count();
    let conflict = blocks
        .iter()
        .filter(|b| b.status == MergeStatus::Conflict)
        .count();

    assert_eq!(local_only, 1, "AC-002: one LocalOnly block; got {blocks:?}");
    assert_eq!(
        remote_only, 1,
        "AC-002: one RemoteOnly block; got {blocks:?}"
    );
    assert_eq!(
        conflict, 0,
        "AC-002: non-overlapping edits produce NO conflict; got {blocks:?}"
    );
    println!("AC-002 non-overlapping: 1 LocalOnly + 1 RemoteOnly, 0 conflicts");
}

#[test]
fn merge_engine_both_modify_same_line_is_conflict() {
    let base = TextBuffer::new("l0\nl1\nl2\nl3");
    let local = TextBuffer::new("l0\nl1\nLOCAL_EDIT\nl3");
    let remote = TextBuffer::new("l0\nl1\nREMOTE_EDIT\nl3");
    let blocks = MergeEngine::three_way(&base, &local, &remote);

    let conflicts: Vec<_> = blocks
        .iter()
        .filter(|b| b.status == MergeStatus::Conflict)
        .collect();
    assert_eq!(
        conflicts.len(),
        1,
        "AC-002: both modify line 3 -> one Conflict block; got {blocks:?}"
    );
    assert_eq!(
        conflicts[0].chosen, None,
        "AC-002: an unresolved conflict has no chosen side"
    );
    assert_eq!(
        conflicts[0].local_lines.start, 2,
        "AC-002: the conflict is at line index 2"
    );
    println!("AC-002 conflict: one Conflict block at line index 2, unresolved");
}

#[test]
fn merge_engine_identical_edits_is_both_same() {
    let base = TextBuffer::new("l0\nl1\nl2");
    let local = TextBuffer::new("l0\nSAME_EDIT\nl2");
    let remote = TextBuffer::new("l0\nSAME_EDIT\nl2");
    let blocks = MergeEngine::three_way(&base, &local, &remote);
    assert_eq!(
        blocks.len(),
        1,
        "one block for the single changed line; got {blocks:?}"
    );
    assert_eq!(
        blocks[0].status,
        MergeStatus::BothSame,
        "AC-002: identical edits -> BothSame"
    );
    println!("AC-002 identical: one BothSame block (no conflict)");
}

#[test]
fn merge_engine_apply_accept_local_then_remote_then_both() {
    let base = TextBuffer::new("l0\nl1\nl2");
    let local = TextBuffer::new("l0\nLOCAL_EDIT\nl2");
    let remote = TextBuffer::new("l0\nREMOTE_EDIT\nl2");

    // Accept Local.
    let mut blocks = MergeEngine::three_way(&base, &local, &remote);
    for b in blocks
        .iter_mut()
        .filter(|b| b.status == MergeStatus::Conflict)
    {
        b.chosen = Some(MergeChoice::Local);
    }
    let merged = MergeEngine::apply(&base, &local, &remote, &blocks).to_string();
    assert!(
        merged.contains("LOCAL_EDIT") && !merged.contains("REMOTE_EDIT"),
        "RISK-003: Accept Local merged buffer has local only; got {merged:?}"
    );

    // Accept Remote.
    let mut blocks = MergeEngine::three_way(&base, &local, &remote);
    for b in blocks
        .iter_mut()
        .filter(|b| b.status == MergeStatus::Conflict)
    {
        b.chosen = Some(MergeChoice::Remote);
    }
    let merged = MergeEngine::apply(&base, &local, &remote, &blocks).to_string();
    assert!(
        merged.contains("REMOTE_EDIT") && !merged.contains("LOCAL_EDIT"),
        "RISK-003: Accept Remote merged buffer has remote only; got {merged:?}"
    );

    // Accept Both (local line first, then remote line).
    let mut blocks = MergeEngine::three_way(&base, &local, &remote);
    for b in blocks
        .iter_mut()
        .filter(|b| b.status == MergeStatus::Conflict)
    {
        b.chosen = Some(MergeChoice::Both);
    }
    let merged = MergeEngine::apply(&base, &local, &remote, &blocks).to_string();
    let lp = merged.find("LOCAL_EDIT").expect("local present");
    let rp = merged.find("REMOTE_EDIT").expect("remote present");
    assert!(
        lp < rp,
        "RISK-003: Accept Both -> local line before remote line; got {merged:?}"
    );
    println!("RISK-003: apply() correct for Accept Local / Remote / Both");
}

#[test]
fn merge_engine_apply_total_on_unresolved_conflict() {
    // Robustness: apply() must NOT panic on an unresolved conflict (defaults to local). The panel
    // forces a choice via the buttons, but the engine stays total.
    let base = TextBuffer::new("l0\nl1\nl2");
    let local = TextBuffer::new("l0\nLOCAL_EDIT\nl2");
    let remote = TextBuffer::new("l0\nREMOTE_EDIT\nl2");
    let blocks = MergeEngine::three_way(&base, &local, &remote); // left unresolved
    let merged = MergeEngine::apply(&base, &local, &remote, &blocks).to_string();
    assert!(
        merged.contains("LOCAL_EDIT"),
        "unresolved conflict defaults to local (total): {merged:?}"
    );
    println!("RISK-003: apply() is total on an unresolved conflict (default = local, no panic)");
}
