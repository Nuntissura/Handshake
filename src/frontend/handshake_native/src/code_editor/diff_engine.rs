//! Diff + three-way merge engine for the native code editor (WP-KERNEL-012 MT-009).
//!
//! This module ports the data structures and algorithms of the React
//! `app/src/lib/editor/document_diff_merge.ts` into native Rust over [`TextBuffer`]. The MT-009
//! deliverable is the LINE-LEVEL diff + three-way merge for code/prose text buffers (the E1 code
//! editor). The React file's JSON-block-level prose diff (`buildRichDocumentDiff` over
//! `RichDocumentJson`) targets the rich-text editor's native document format, which DOES NOT EXIST
//! yet (E2 builds it, MT-011+); per the MT contract that block-level prose entry point is DEFERRED.
//! [`diff_json_blocks`] is provided as the design seam so the prose entry point can be added later
//! without reshaping this module, and it is exercised by a unit test against `serde_json::Value`
//! pairs, but the editor surface (MT-009) drives the line-level path only.
//!
//! ## Why `similar`
//!
//! [`similar`](https://crates.io/crates/similar) is the production-grade, MIT-licensed Rust diff
//! library used across the ecosystem (`insta`, `cargo-insta`, many CLIs). It implements Myers' diff
//! (the algorithm the MT contract names) over slices and exposes per-line change tags via
//! `TextDiff::from_lines` + `ChangeTag`. Re-implementing Myers from scratch would add a hand-rolled,
//! less-tested algorithm where a field-hardened one exists (GLOBAL-RESEARCH-039: prefer proven
//! implementations). The MT contract explicitly sanctions `similar = "2"`.
//!
//! ## Grouping `ChangeTag`s into `DiffBlock`s (MT step 1)
//!
//! `TextDiff::from_lines` yields a flat sequence of `Change { tag, old_index, new_index }`. The MT
//! contract's mapping is:
//!   - `ChangeTag::Equal`  -> [`DiffStatus::Equal`]
//!   - `ChangeTag::Insert` -> [`DiffStatus::Added`]
//!   - `ChangeTag::Delete` -> [`DiffStatus::Removed`]
//!   - a `Delete` run immediately followed by an `Insert` run at the same position -> a single
//!     [`DiffStatus::Modified`] block (the "changed line" case, not a separate remove + add).
//!
//! Each [`DiffBlock`] records the half-open LINE ranges it covers on the left and right buffers, so
//! the panel can paint a background rect over exactly the affected rows and the sync-scroll line map
//! can align the two panes.

use std::ops::Range;

use super::buffer::TextBuffer;

/// The change classification of a [`DiffBlock`]. Mirrors the React
/// `RichDocumentDiffStatus` union (`unchanged | modified | added | removed`); `Equal` is the native
/// name for `unchanged` so the enum reads naturally next to `similar::ChangeTag::Equal`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffStatus {
    /// The lines are identical on both sides.
    Equal,
    /// The lines exist only on the right (an insertion).
    Added,
    /// The lines exist only on the left (a deletion).
    Removed,
    /// The lines differ on both sides (a delete run fused with the insert run that replaced it).
    Modified,
}

/// One contiguous run of the diff: the half-open LINE range it occupies on each side plus its
/// [`DiffStatus`]. A `Removed` block has an EMPTY `right_lines` (nothing on the right); an `Added`
/// block has an EMPTY `left_lines`. `Equal`/`Modified` blocks carry both ranges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffBlock {
    /// Half-open line range on the LEFT (old) buffer this block covers. Empty for `Added`.
    pub left_lines: Range<usize>,
    /// Half-open line range on the RIGHT (new) buffer this block covers. Empty for `Removed`.
    pub right_lines: Range<usize>,
    /// The change classification.
    pub status: DiffStatus,
}

/// Stateless line-level diff engine. Computes a [`Vec<DiffBlock>`] for two [`TextBuffer`]s using
/// Myers' diff via the `similar` crate.
pub struct DiffEngine;

impl DiffEngine {
    /// Diff two buffers at the LINE level, returning one [`DiffBlock`] per contiguous run of equal /
    /// added / removed / modified lines (MT step 1). The blocks are returned in document order and
    /// their left/right ranges tile the two buffers without gaps or overlaps.
    pub fn diff(left: &TextBuffer, right: &TextBuffer) -> Vec<DiffBlock> {
        let left_text = left.to_string();
        let right_text = right.to_string();
        Self::diff_text(&left_text, &right_text)
    }

    /// Diff two `&str`s at the line level. Split out from [`diff`](Self::diff) so the unit tests and
    /// the prose entry point share one implementation without forcing a `TextBuffer` allocation.
    pub fn diff_text(left_text: &str, right_text: &str) -> Vec<DiffBlock> {
        let diff = similar::TextDiff::from_lines(left_text, right_text);

        // First, flatten `similar`'s changes into per-line (tag, side-index) tuples in document order.
        // `similar` already groups by tag in its iterator, but we re-walk so we can FUSE an adjacent
        // Delete-run + Insert-run into a single Modified block (MT step 1 — the "changed line" case).
        let mut blocks: Vec<DiffBlock> = Vec::new();

        // Track the next unallocated line index on each side so each block gets contiguous ranges.
        let mut left_cursor = 0usize;
        let mut right_cursor = 0usize;

        // Collect the change tags into runs of the same tag (a "run" = maximal consecutive same-tag).
        let mut runs: Vec<(similar::ChangeTag, usize)> = Vec::new();
        for change in diff.iter_all_changes() {
            let tag = change.tag();
            match runs.last_mut() {
                Some((last_tag, count)) if *last_tag == tag => *count += 1,
                _ => runs.push((tag, 1)),
            }
        }

        let mut i = 0usize;
        while i < runs.len() {
            let (tag, count) = runs[i];
            match tag {
                similar::ChangeTag::Equal => {
                    let l = left_cursor..left_cursor + count;
                    let r = right_cursor..right_cursor + count;
                    left_cursor += count;
                    right_cursor += count;
                    blocks.push(DiffBlock {
                        left_lines: l,
                        right_lines: r,
                        status: DiffStatus::Equal,
                    });
                    i += 1;
                }
                similar::ChangeTag::Delete => {
                    // A Delete run immediately followed by an Insert run is a MODIFIED block (lines
                    // replaced in place), not a separate Removed + Added (MT step 1).
                    if let Some((similar::ChangeTag::Insert, ins_count)) = runs.get(i + 1).copied()
                    {
                        let l = left_cursor..left_cursor + count;
                        let r = right_cursor..right_cursor + ins_count;
                        left_cursor += count;
                        right_cursor += ins_count;
                        blocks.push(DiffBlock {
                            left_lines: l,
                            right_lines: r,
                            status: DiffStatus::Modified,
                        });
                        i += 2; // consumed both the Delete and the Insert run.
                    } else {
                        let l = left_cursor..left_cursor + count;
                        let r = right_cursor..right_cursor; // empty on the right.
                        left_cursor += count;
                        blocks.push(DiffBlock {
                            left_lines: l,
                            right_lines: r,
                            status: DiffStatus::Removed,
                        });
                        i += 1;
                    }
                }
                similar::ChangeTag::Insert => {
                    let l = left_cursor..left_cursor; // empty on the left.
                    let r = right_cursor..right_cursor + count;
                    right_cursor += count;
                    blocks.push(DiffBlock {
                        left_lines: l,
                        right_lines: r,
                        status: DiffStatus::Added,
                    });
                    i += 1;
                }
            }
        }

        blocks
    }
}

/// The change classification of a [`MergeBlock`]. Mirrors the React `RichDocumentMergeStatus`
/// (`local_only | remote_only | both_same | conflict`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeStatus {
    /// Only the LOCAL side changed this line range relative to base.
    LocalOnly,
    /// Only the REMOTE side changed this line range relative to base.
    RemoteOnly,
    /// Both sides changed this line range to the SAME value (no conflict).
    BothSame,
    /// Both sides changed this line range to DIFFERENT values (a conflict needing a choice).
    Conflict,
}

/// Which side's content the operator (or an agent) chose for a [`MergeBlock`]. Mirrors the React
/// `RichDocumentMergeChoice` (`local | remote | both`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeChoice {
    /// Keep the local version.
    Local,
    /// Keep the remote version.
    Remote,
    /// Keep both versions (local lines followed by remote lines).
    Both,
}

/// One block of a three-way merge plan: the half-open LINE ranges it covers in the base, local, and
/// remote buffers, its [`MergeStatus`], and the operator's [`MergeChoice`] once made. Mirrors the
/// React `RichDocumentMergeBlock`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeBlock {
    /// Half-open line range in the BASE buffer this block aligns to.
    pub base_lines: Range<usize>,
    /// Half-open line range in the LOCAL buffer.
    pub local_lines: Range<usize>,
    /// Half-open line range in the REMOTE buffer.
    pub remote_lines: Range<usize>,
    /// The change classification relative to base.
    pub status: MergeStatus,
    /// The chosen side, set when the operator/agent resolves a `Conflict` (or pre-set for the
    /// non-conflict statuses so [`MergeEngine::apply`] can emit a merged buffer without a choice).
    pub chosen: Option<MergeChoice>,
}

/// Stateless three-way merge engine. Aligns local-vs-base and remote-vs-base line diffs into merge
/// blocks (MT step computing local/remote diffs then aligning) and applies the chosen sides into a
/// merged buffer.
pub struct MergeEngine;

impl MergeEngine {
    /// Compute the three-way merge blocks for `base` / `local` / `remote` (MT contract
    /// `three_way`). Computes base->local and base->remote diff spans, then aligns those spans by
    /// base range / insertion point. This is deliberately NOT a raw same-index line walk: insertions
    /// and deletions before a later edit must not shift the other side into a false conflict. For each
    /// aligned change group it classifies whether local and/or remote changed relative to base:
    ///   - neither changed -> NO block emitted (the line is unchanged, like the React port which
    ///     `continue`s when `!localChanged && !remoteChanged`).
    ///   - only local changed  -> [`MergeStatus::LocalOnly`] (chosen pre-set to `Local`).
    ///   - only remote changed -> [`MergeStatus::RemoteOnly`] (chosen pre-set to `Remote`).
    ///   - both changed to the same value -> [`MergeStatus::BothSame`] (chosen pre-set to `Local`).
    ///   - both changed differently -> [`MergeStatus::Conflict`] (chosen `None` — needs a choice).
    pub fn three_way(
        base: &TextBuffer,
        local: &TextBuffer,
        remote: &TextBuffer,
    ) -> Vec<MergeBlock> {
        let base_text = base.to_string();
        let local_text = local.to_string();
        let remote_text = remote.to_string();
        let local_lines = lines_from_text(&local_text);
        let remote_lines = lines_from_text(&remote_text);

        let local_changes = side_changes_from_base(&base_text, &local_text);
        let remote_changes = side_changes_from_base(&base_text, &remote_text);
        let groups = collect_change_groups(local_changes, remote_changes);

        groups
            .into_iter()
            .map(|group| {
                let (status, chosen) = match (group.has_local, group.has_remote) {
                    (true, false) => (MergeStatus::LocalOnly, Some(MergeChoice::Local)),
                    (false, true) => (MergeStatus::RemoteOnly, Some(MergeChoice::Remote)),
                    (true, true) => {
                        let local_content = line_slice(&local_lines, &group.local_lines);
                        let remote_content = line_slice(&remote_lines, &group.remote_lines);
                        if local_content == remote_content {
                            (MergeStatus::BothSame, Some(MergeChoice::Local))
                        } else {
                            (MergeStatus::Conflict, None)
                        }
                    }
                    (false, false) => {
                        unreachable!("change groups always contain at least one side")
                    }
                };
                MergeBlock {
                    base_lines: group.base_lines,
                    local_lines: group.local_lines,
                    remote_lines: group.remote_lines,
                    status,
                    chosen,
                }
            })
            .collect()
    }

    /// Apply a resolved merge plan to produce the merged buffer text (MT step 5). The plan is walked by
    /// base ranges, so insertions and deletions do not corrupt later line indexes. Unchanged base ranges
    /// are emitted from `base`; changed ranges are emitted from the selected side. A `Conflict` block
    /// with no `chosen` preserves BOTH sides (local first, then remote), so an unresolved preview is total
    /// without silently dropping either side.
    pub fn apply(
        base: &TextBuffer,
        local: &TextBuffer,
        remote: &TextBuffer,
        blocks: &[MergeBlock],
    ) -> TextBuffer {
        let base_lines = buffer_lines(base);
        let local_lines = buffer_lines(local);
        let remote_lines = buffer_lines(remote);

        let mut ordered: Vec<&MergeBlock> = blocks.iter().collect();
        ordered.sort_by_key(|b| (b.base_lines.start, b.base_lines.end));
        let mut merged: Vec<String> = Vec::new();
        let mut base_cursor = 0usize;

        for block in ordered {
            let start = block.base_lines.start.min(base_lines.len());
            if start > base_cursor {
                merged.extend(base_lines[base_cursor..start].iter().cloned());
            }

            let choice = block.chosen.unwrap_or(MergeChoice::Both);
            match choice {
                MergeChoice::Local => extend_range(&mut merged, &local_lines, &block.local_lines),
                MergeChoice::Remote => {
                    extend_range(&mut merged, &remote_lines, &block.remote_lines)
                }
                MergeChoice::Both => {
                    extend_range(&mut merged, &local_lines, &block.local_lines);
                    extend_range(&mut merged, &remote_lines, &block.remote_lines);
                }
            }
            base_cursor = base_cursor.max(block.base_lines.end.min(base_lines.len()));
        }

        if base_cursor < base_lines.len() {
            merged.extend(base_lines[base_cursor..].iter().cloned());
        }

        TextBuffer::new(&merged.join("\n"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum MergeSide {
    Local,
    Remote,
}

#[derive(Clone, Debug)]
struct SideChange {
    base_lines: Range<usize>,
    side_lines: Range<usize>,
}

#[derive(Clone, Debug)]
struct PendingChange {
    side: MergeSide,
    base_lines: Range<usize>,
    side_lines: Range<usize>,
}

#[derive(Clone, Debug)]
struct ChangeGroup {
    base_lines: Range<usize>,
    local_lines: Range<usize>,
    remote_lines: Range<usize>,
    has_local: bool,
    has_remote: bool,
}

impl ChangeGroup {
    fn from_change(change: PendingChange) -> Self {
        match change.side {
            MergeSide::Local => Self {
                base_lines: change.base_lines,
                local_lines: change.side_lines,
                remote_lines: 0..0,
                has_local: true,
                has_remote: false,
            },
            MergeSide::Remote => Self {
                base_lines: change.base_lines,
                local_lines: 0..0,
                remote_lines: change.side_lines,
                has_local: false,
                has_remote: true,
            },
        }
    }

    fn absorb(&mut self, change: PendingChange) {
        self.base_lines = union_ranges(&self.base_lines, &change.base_lines);
        match change.side {
            MergeSide::Local => {
                self.local_lines = if self.has_local {
                    union_ranges(&self.local_lines, &change.side_lines)
                } else {
                    change.side_lines
                };
                self.has_local = true;
            }
            MergeSide::Remote => {
                self.remote_lines = if self.has_remote {
                    union_ranges(&self.remote_lines, &change.side_lines)
                } else {
                    change.side_lines
                };
                self.has_remote = true;
            }
        }
    }
}

fn side_changes_from_base(base_text: &str, side_text: &str) -> Vec<SideChange> {
    DiffEngine::diff_text(base_text, side_text)
        .into_iter()
        .filter(|block| block.status != DiffStatus::Equal)
        .map(|block| SideChange {
            base_lines: block.left_lines,
            side_lines: block.right_lines,
        })
        .collect()
}

fn collect_change_groups(
    local_changes: Vec<SideChange>,
    remote_changes: Vec<SideChange>,
) -> Vec<ChangeGroup> {
    let mut pending = Vec::with_capacity(local_changes.len() + remote_changes.len());
    pending.extend(local_changes.into_iter().map(|change| PendingChange {
        side: MergeSide::Local,
        base_lines: change.base_lines,
        side_lines: change.side_lines,
    }));
    pending.extend(remote_changes.into_iter().map(|change| PendingChange {
        side: MergeSide::Remote,
        base_lines: change.base_lines,
        side_lines: change.side_lines,
    }));
    pending.sort_by_key(|change| {
        (
            change.base_lines.start,
            change.base_lines.end,
            change.side,
            change.side_lines.start,
            change.side_lines.end,
        )
    });

    let mut groups: Vec<ChangeGroup> = Vec::new();
    for change in pending {
        if let Some(group) = groups.last_mut() {
            if ranges_interact(&group.base_lines, &change.base_lines) {
                group.absorb(change);
                continue;
            }
        }
        groups.push(ChangeGroup::from_change(change));
    }
    groups
}

fn ranges_interact(a: &Range<usize>, b: &Range<usize>) -> bool {
    let a_empty = a.start == a.end;
    let b_empty = b.start == b.end;
    match (a_empty, b_empty) {
        (true, true) => a.start == b.start,
        (true, false) => a.start > b.start && a.start < b.end,
        (false, true) => b.start > a.start && b.start < a.end,
        (false, false) => a.start < b.end && b.start < a.end,
    }
}

fn union_ranges(a: &Range<usize>, b: &Range<usize>) -> Range<usize> {
    a.start.min(b.start)..a.end.max(b.end)
}

fn line_slice(lines: &[String], range: &Range<usize>) -> Vec<String> {
    lines
        .get(range.start.min(lines.len())..range.end.min(lines.len()))
        .unwrap_or(&[])
        .to_vec()
}

fn extend_range(out: &mut Vec<String>, lines: &[String], range: &Range<usize>) {
    if let Some(slice) = lines.get(range.start.min(lines.len())..range.end.min(lines.len())) {
        out.extend(slice.iter().cloned());
    }
}

/// Split a [`TextBuffer`] into its logical lines (without trailing newlines), for the line-aligned
/// three-way merge. A buffer ending in `\n` does NOT yield a trailing empty line here (the merge
/// aligns content lines), matching the React port which aligns `content` array entries.
fn buffer_lines(buffer: &TextBuffer) -> Vec<String> {
    let text = buffer.to_string();
    lines_from_text(&text)
}

fn lines_from_text(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.lines().map(|l| l.to_owned()).collect()
}

// ── Deferred prose/JSON block-level diff seam (E2, MT-011+) ──────────────────────────────────────

/// DEFERRED prose/JSON-block diff entry point (MT-009 contract: the JSON-block-level prose diff
/// targets the E2 native rich-document format that does not exist yet). Provided as the design seam
/// (and exercised by a unit test) so the block-level path can be added later without reshaping this
/// module. It aligns two arrays of JSON block nodes by index and classifies each position the same
/// way the line diff does (`Added` / `Removed` / `Modified` / `Equal`), returning [`DiffBlock`]s
/// whose ranges are block INDICES rather than line indices.
///
/// This is intentionally a simple index alignment (not the React LCS block alignment) because the
/// native rich-document format is not yet defined; when E2 lands, this is the hook to replace with
/// the format-aware alignment ported from `buildRichDocumentDiff`.
pub fn diff_json_blocks(left: &[serde_json::Value], right: &[serde_json::Value]) -> Vec<DiffBlock> {
    let count = left.len().max(right.len());
    let mut blocks = Vec::new();
    for index in 0..count {
        let l = left.get(index);
        let r = right.get(index);
        let status = match (l, r) {
            (None, Some(_)) => DiffStatus::Added,
            (Some(_), None) => DiffStatus::Removed,
            (Some(a), Some(b)) if a == b => DiffStatus::Equal,
            (Some(_), Some(_)) => DiffStatus::Modified,
            (None, None) => continue,
        };
        let l_range = if l.is_some() {
            index..index + 1
        } else {
            index..index
        };
        let r_range = if r.is_some() {
            index..index + 1
        } else {
            index..index
        };
        blocks.push(DiffBlock {
            left_lines: l_range,
            right_lines: r_range,
            status,
        });
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DiffEngine line-level (AC-001: ported from document_diff_merge.test.ts) ──────────────────

    #[test]
    fn identical_texts_are_all_equal() {
        let left = TextBuffer::new("line0\nline1\nline2");
        let right = TextBuffer::new("line0\nline1\nline2");
        let blocks = DiffEngine::diff(&left, &right);
        assert!(
            blocks.iter().all(|b| b.status == DiffStatus::Equal),
            "identical texts -> all Equal blocks; got {blocks:?}"
        );
        // And the Equal block(s) cover every line.
        let covered: usize = blocks
            .iter()
            .filter(|b| b.status == DiffStatus::Equal)
            .map(|b| b.left_lines.len())
            .sum();
        assert_eq!(covered, 3, "all 3 lines covered by Equal blocks");
    }

    #[test]
    fn added_line_yields_one_added_block() {
        let left = TextBuffer::new("a\nb\nc");
        let right = TextBuffer::new("a\nb\nNEW\nc");
        let blocks = DiffEngine::diff(&left, &right);
        let added: Vec<_> = blocks
            .iter()
            .filter(|b| b.status == DiffStatus::Added)
            .collect();
        assert_eq!(added.len(), 1, "exactly one Added block; got {blocks:?}");
        assert_eq!(
            added[0].left_lines.len(),
            0,
            "Added block is empty on the left"
        );
        assert_eq!(
            added[0].right_lines.len(),
            1,
            "Added block covers one right line"
        );
    }

    #[test]
    fn removed_line_yields_one_removed_block() {
        let left = TextBuffer::new("a\nb\nc\nd");
        let right = TextBuffer::new("a\nb\nd");
        let blocks = DiffEngine::diff(&left, &right);
        let removed: Vec<_> = blocks
            .iter()
            .filter(|b| b.status == DiffStatus::Removed)
            .collect();
        assert_eq!(
            removed.len(),
            1,
            "exactly one Removed block; got {blocks:?}"
        );
        assert_eq!(
            removed[0].right_lines.len(),
            0,
            "Removed block is empty on the right"
        );
        assert_eq!(
            removed[0].left_lines.len(),
            1,
            "Removed block covers one left line"
        );
    }

    #[test]
    fn modified_line_yields_one_modified_block() {
        let left = TextBuffer::new("a\nMID\nc");
        let right = TextBuffer::new("a\nCHANGED\nc");
        let blocks = DiffEngine::diff(&left, &right);
        let modified: Vec<_> = blocks
            .iter()
            .filter(|b| b.status == DiffStatus::Modified)
            .collect();
        assert_eq!(
            modified.len(),
            1,
            "a delete+insert at the same position is ONE Modified block; got {blocks:?}"
        );
        assert_eq!(
            modified[0].left_lines.len(),
            1,
            "Modified covers one left line"
        );
        assert_eq!(
            modified[0].right_lines.len(),
            1,
            "Modified covers one right line"
        );
    }

    #[test]
    fn diff_blocks_tile_both_buffers_without_gaps() {
        // RISK-002 basis: the left/right ranges must tile each buffer contiguously so the sync-scroll
        // line map and the background rects line up. Use a mix of add/remove/modify/equal.
        let left = TextBuffer::new("keep1\nremoveme\nmod_old\nkeep2");
        let right = TextBuffer::new("keep1\nmod_new\nkeep2\nadded");
        let blocks = DiffEngine::diff(&left, &right);

        let mut next_left = 0usize;
        let mut next_right = 0usize;
        for b in &blocks {
            assert_eq!(
                b.left_lines.start, next_left,
                "left ranges are contiguous: {blocks:?}"
            );
            assert_eq!(
                b.right_lines.start, next_right,
                "right ranges are contiguous: {blocks:?}"
            );
            next_left = b.left_lines.end;
            next_right = b.right_lines.end;
        }
        assert_eq!(
            next_left,
            left.to_string().lines().count(),
            "left fully tiled"
        );
        assert_eq!(
            next_right,
            right.to_string().lines().count(),
            "right fully tiled"
        );
    }

    // ── MergeEngine three-way (AC-002: ported from document_diff_merge.test.ts) ───────────────────

    #[test]
    fn non_overlapping_edits_are_local_only_and_remote_only() {
        // local adds to line 2, remote adds to line 5 of the same base -> two non-conflict blocks.
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
        let conflicts = blocks
            .iter()
            .filter(|b| b.status == MergeStatus::Conflict)
            .count();
        assert_eq!(local_only, 1, "one LocalOnly block; got {blocks:?}");
        assert_eq!(remote_only, 1, "one RemoteOnly block; got {blocks:?}");
        assert_eq!(
            conflicts, 0,
            "no conflicts for non-overlapping edits; got {blocks:?}"
        );
    }

    #[test]
    fn both_modify_same_line_differently_is_a_conflict() {
        let base = TextBuffer::new("l0\nl1\nl2");
        let local = TextBuffer::new("l0\nLOCAL_EDIT\nl2");
        let remote = TextBuffer::new("l0\nREMOTE_EDIT\nl2");
        let blocks = MergeEngine::three_way(&base, &local, &remote);
        let conflicts: Vec<_> = blocks
            .iter()
            .filter(|b| b.status == MergeStatus::Conflict)
            .collect();
        assert_eq!(conflicts.len(), 1, "one Conflict block; got {blocks:?}");
        assert_eq!(
            conflicts[0].chosen, None,
            "an unresolved conflict has no chosen side"
        );
    }

    #[test]
    fn both_modify_same_line_identically_is_both_same() {
        let base = TextBuffer::new("l0\nl1\nl2");
        let local = TextBuffer::new("l0\nSAME\nl2");
        let remote = TextBuffer::new("l0\nSAME\nl2");
        let blocks = MergeEngine::three_way(&base, &local, &remote);
        assert_eq!(
            blocks.len(),
            1,
            "one block for the single changed line; got {blocks:?}"
        );
        assert_eq!(
            blocks[0].status,
            MergeStatus::BothSame,
            "identical edits -> BothSame"
        );
    }

    #[test]
    fn apply_accept_local_emits_local_version_of_conflict() {
        // AC-005 basis (engine half): a conflict resolved to Local must put the local line in the
        // merged output.
        let base = TextBuffer::new("l0\nl1\nl2");
        let local = TextBuffer::new("l0\nLOCAL_EDIT\nl2");
        let remote = TextBuffer::new("l0\nREMOTE_EDIT\nl2");
        let mut blocks = MergeEngine::three_way(&base, &local, &remote);
        // Resolve the single conflict to Local.
        for b in &mut blocks {
            if b.status == MergeStatus::Conflict {
                b.chosen = Some(MergeChoice::Local);
            }
        }
        let merged = MergeEngine::apply(&base, &local, &remote, &blocks);
        let merged_text = merged.to_string();
        assert!(
            merged_text.contains("LOCAL_EDIT"),
            "Accept Local -> merged buffer contains the local version; got {merged_text:?}"
        );
        assert!(
            !merged_text.contains("REMOTE_EDIT"),
            "Accept Local -> merged buffer does NOT contain the remote version; got {merged_text:?}"
        );
    }

    #[test]
    fn apply_accept_remote_emits_remote_version_of_conflict() {
        let base = TextBuffer::new("l0\nl1\nl2");
        let local = TextBuffer::new("l0\nLOCAL_EDIT\nl2");
        let remote = TextBuffer::new("l0\nREMOTE_EDIT\nl2");
        let mut blocks = MergeEngine::three_way(&base, &local, &remote);
        for b in &mut blocks {
            if b.status == MergeStatus::Conflict {
                b.chosen = Some(MergeChoice::Remote);
            }
        }
        let merged = MergeEngine::apply(&base, &local, &remote, &blocks);
        let merged_text = merged.to_string();
        assert!(
            merged_text.contains("REMOTE_EDIT"),
            "Accept Remote -> remote version; {merged_text:?}"
        );
        assert!(
            !merged_text.contains("LOCAL_EDIT"),
            "Accept Remote -> not local; {merged_text:?}"
        );
    }

    #[test]
    fn apply_accept_both_emits_local_then_remote() {
        let base = TextBuffer::new("l0\nl1\nl2");
        let local = TextBuffer::new("l0\nLOCAL_EDIT\nl2");
        let remote = TextBuffer::new("l0\nREMOTE_EDIT\nl2");
        let mut blocks = MergeEngine::three_way(&base, &local, &remote);
        for b in &mut blocks {
            if b.status == MergeStatus::Conflict {
                b.chosen = Some(MergeChoice::Both);
            }
        }
        let merged = MergeEngine::apply(&base, &local, &remote, &blocks);
        let merged_text = merged.to_string();
        let local_pos = merged_text.find("LOCAL_EDIT").expect("local present");
        let remote_pos = merged_text.find("REMOTE_EDIT").expect("remote present");
        assert!(
            local_pos < remote_pos,
            "Accept Both -> local line before remote line; {merged_text:?}"
        );
    }

    #[test]
    fn apply_unchanged_lines_pass_through() {
        let base = TextBuffer::new("keep0\nl1\nkeep2");
        let local = TextBuffer::new("keep0\nLOCAL\nkeep2");
        let remote = TextBuffer::new("keep0\nl1\nkeep2");
        let blocks = MergeEngine::three_way(&base, &local, &remote);
        let merged = MergeEngine::apply(&base, &local, &remote, &blocks).to_string();
        assert!(
            merged.contains("keep0") && merged.contains("keep2"),
            "unchanged lines survive: {merged:?}"
        );
        assert!(
            merged.contains("LOCAL"),
            "the local-only edit is applied: {merged:?}"
        );
    }

    // ── Deferred prose/JSON block seam ────────────────────────────────────────────────────────────

    #[test]
    fn json_block_diff_classifies_by_index() {
        let left = vec![
            serde_json::json!({"type":"p","text":"a"}),
            serde_json::json!({"type":"p","text":"b"}),
        ];
        let right = vec![
            serde_json::json!({"type":"p","text":"a"}),
            serde_json::json!({"type":"p","text":"CHANGED"}),
            serde_json::json!({"type":"p","text":"NEW"}),
        ];
        let blocks = diff_json_blocks(&left, &right);
        assert_eq!(blocks[0].status, DiffStatus::Equal, "block 0 identical");
        assert_eq!(blocks[1].status, DiffStatus::Modified, "block 1 changed");
        assert_eq!(
            blocks[2].status,
            DiffStatus::Added,
            "block 2 only on the right"
        );
    }
}
