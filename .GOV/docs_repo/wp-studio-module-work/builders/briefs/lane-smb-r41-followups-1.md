---
file_id: LANE-SMB-R41-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-30T08:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-R41-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r41-followups-1.json
crates: [hsk-studio-layout, hsk-studio-harness-view]
model: opus
effort: medium
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Repair R41 findings F-R41-1, -3, -4 and the F-R41-5 residual">

## Target

Source: `validation/round-R41.json` `findings_not_failures` (read F-R41-1, -3, -4, -5 in full first). Base: origin c719b1dc.

1. F-R41-1 (MEDIUM, hsk-studio-layout): `NOTICE:13-14`, `src/cjk.rs:6-11` and the DESCRIPTOR `cjk.sources` (`src/lib.rs:214`) cite the W3C Document License. Correct: JLREQ (W3C Working Group Note 2020-08-11) is under the W3C Software and Document Notice and License (2015, https://www.w3.org/Consortium/Legal/2015/copyright-software-and-document); CLREQ (Group Note Draft 2026-10-07) under the W3C Software and Document License (2023, https://www.w3.org/copyright/software-license-2023/). Fetch both documents' copyright blocks and both licence pages (WebFetch; W3C pages are permissive text, not code) and carry the licence's required elements in all three places: licence name + link (or the licence's short notice), the copyright line with years exactly as each document states it, the dated version URI of each document and its status. Keep the claim "data, no code copied". A test that pins the DESCRIPTOR source entries to the corrected licence ids is welcome (cheap).
2. F-R41-3 (LOW, harness-view): `tests/viewer_side_panels.rs:1-9, 119-123`. Preferred: find the fixed-width row that opens Properties at 445 px (one bounded search; it is a Properties panel row, not timeline or design code), wrap it, and restore the Info minimum-width check at 900x700. If that row is not found or the fix touches files another lane owns, assert the clamp width instead and narrow the file doc to what is proven. State which in the notes.
3. F-R41-4 (LOW): print `NOT_PROVEN:` lines (pattern: `println!("NOT_PROVEN: ...")`, see `tests/viewer_fig_edit.rs:493`) from `crates/hsk-studio-harness-view/tests/viewer_info.rs` (Photoshop Histogram / Info / colour sampler parity: UNVERIFIED, no vendor oracle) and `crates/hsk-studio-layout/tests/cjk_kinsoku.rs` (InDesign kinsoku / mojikumi set parity and the real-shaper CJK path, per the CJK lane notes). Use the wording the two lanes' notes already state; do not claim new gaps.
4. F-R41-5 residual (INFO, harness-view): `src/view/vector.rs` render paths read `self.text_entry().ok()` (lines ~1232, 1260, 1664 at c719b1dc), so a corrupt text entry reports only `text_entry_unavailable`; carry the decode error text into the unshaped reason (typed, no panic). Test: a corrupt text entry yields a report naming the decode error.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths

- hsk-studio-layout: `NOTICE`, `src/cjk.rs` (doc block only), `src/lib.rs` (DESCRIPTOR source strings only), `tests/cjk_kinsoku.rs`.
- hsk-studio-harness-view: `src/view/vector.rs` (text-entry reason only), the Properties row file if found (item 2), `tests/viewer_side_panels.rs`, `tests/viewer_info.rs`, NEW test file for item 4 if needed.
- Running lane SMB-AU-MULTITRACK-1 owns `src/view/timeline*` and may add one registration hunk in `src/view/shell.rs` / `src/view/mod.rs`: do not touch those. Rebase onto origin before every push and keep the diff to your hunks.

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- One bundled lane-check on C: (`--disk ssd`) per push over the WHOLE suites of hsk-studio-layout and hsk-studio-harness-view + hsk-studio-harness with the round's viewer features (`.config/round-features.toml`, rule 4e). No public signature changes: say so in the notes (if one changes, add layout's direct dependents).
- Step-0 literals (rule 4c): the new licence URLs are https W3C URLs, not machine paths; run `--step0-only` first anyway.
- Red-first where practical (item 4 test fails at c719b1dc; item 1 pin test fails on the old ids).

</topic>
