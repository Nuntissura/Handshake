---
file_id: LANE-SMB-AI-AREA-TYPE-3
file_kind: lane_brief
updated_at: 2026-10-30T05:00:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-AREA-TYPE-3
lane_file: builders/lanes/hsk-studio-ai-area-type-3.json
crates: [hsk-studio-harness-view, hsk-studio-harness-core (additive, only if needed)]
model: opus
effort: medium
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Make area/path type usable end to end in the window">

## Target

SMB-AI-AREA-TYPE-1 (engine, c951bbf7) and SMB-AI-AREA-TYPE-2 (viewer shaping + `bind_live_text`, 9ee2b546) are READY. Read both lane files (gaps sections) first. Close the gaps they list, in this order (push after each green slice):

1. Window UI to create bindings: from the vector/type tools the window already has, turn a selected closed path into area text and an open path into path text, and convert point <-> area, through `bind_live_text` and the AREA-TYPE-1 conversion; one undo step each.
2. Create Outlines as one typed, undoable command in the existing vector/type command list (`src/view/hosted.rs` / `src/view/shell.rs` command registries and descriptors; the key-uniqueness test exists, no modifier-less collisions). It must replace the text with outlines in ONE undo step: if the viewer cannot do the layer change and the entry write in one step today, add the smallest additive step API in `hsk-studio-harness-core` (`src/harness.rs`, `Run`), with its own tests, and name it in the notes.
3. Exports (`src/view/interop.rs`, `pdf_export.rs`, `separations.rs`) use the same shaping assets as the canvas so exported files carry live text; a story that cannot be shaped keeps its typed report in the export result.
4. Small: status line echoes the live-text report; fix the stale `path_text.rs` module doc.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths

- harness-view: `src/view/vector.rs`, `src/view/text.rs`, `src/view/path_text.rs` (if that is where the doc lives), `src/view/hosted.rs` and `src/view/shell.rs` (vector/type command hunks only), `src/view/interop.rs`, `src/view/pdf_export.rs`, `src/view/separations.rs`, NEW tests `tests/viewer_vector_text_*.rs`.
- harness-core: additive step API only if slice 2 needs it.
- Shared with SMB-AU-MULTITRACK-1 (running, owns `src/view/timeline*`): it may add one registration hunk in `shell.rs`/`mod.rs`; keep your hunks separate and rebase carefully. Do NOT touch `src/view/timeline*`, `src/view/design/**`, engine crates other than harness-core, or `RUN-UI.md` (list the lines in the notes).

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Viewer tests per slice, red-first where practical; for exports, a test that exported output contains the text geometry and that an unshapeable story reports.
- One bundled lane-check on C: per push over the WHOLE harness-view and harness suites with the round's viewer features, plus harness-core's whole suite and its direct dependents if harness-core changes (rule 4a).
- A validation round may hold the C: lock with priority while you work; wait for it, never touch locks.
- Illustrator behaviour UNVERIFIED (D5). GUI lane 2 of max 2.

</topic>
