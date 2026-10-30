---
file_id: LANE-SMB-VIEW-POLISH-1
file_kind: lane_brief
updated_at: 2026-10-30T13:45:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-VIEW-POLISH-1
lane_file: builders/lanes/hsk-studio-view-polish-1.json
crates: [hsk-studio-harness-view]
model: opus
effort: medium
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Window gaps left by SMB-AI-AREA-TYPE-3 and F-R41-3 (view/mod.rs, shell.rs)">

## Target

Read first: `builders/lanes/hsk-studio-ai-area-type-3.json` notes `gaps` and `run_ui_md_lines`, `builders/lanes/hsk-studio-r41-followups-1.json` notes for F-R41-3, `validation/round-R41.json` finding F-R41-3. Illustrator behaviour UNVERIFIED (D5): declared readings.

1. F-R41-3: find (measure, do not guess) the fixed-width row that opens Properties at 445 px at 900x700; the R41 follow-up lane suspected the import receipt's loss grid (`receipt_ui` in `src/view/mod.rs`). Wrap or truncate-with-tooltip it so panels keep their default widths, then restore the Info minimum-width check (`INFO_MIN`) at 900x700 in `tests/viewer_side_panels.rs` and the file doc's original claim.
2. Canvas gestures for live text: with the Text tool, a click on an open vector path creates path text on it and a click inside a closed path creates area text in it (through `bind_live_text`, one undo step); a click elsewhere still places point text. Tests drive the real viewer headlessly (aim clicks after the first egui frame; reuse `tests/viewer_vector_text_tools.rs` helpers).
3. Create Outlines as a shell menu leaf (Type menu) with a key binding that passes the existing key-uniqueness test (no modifier-less collisions; the shell's Ctrl+O handling must not swallow it), dispatching the same typed command as the Path Operations button.
4. Overset / live-text echo after path edits: when a path edit changes a live text's placed/overset count, the status line echoes it (render_if_dirty path), and the Path Operations summary line shows the live-text state.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-view-polish-1`)

- harness-view: `src/view/mod.rs` (receipt grid, text-tool click, render_if_dirty echo hunks), `src/view/shell.rs` (Type menu leaf + key), `src/view/hosted.rs` (Path Operations summary line), `src/view/vector.rs` only for a helper the gestures need, `tests/viewer_side_panels.rs`, NEW `tests/viewer_view_polish.rs`.
- Do NOT touch: `src/view/timeline*`, `src/view/design/**`, engine crates (read-only; report missing APIs), RUN-UI.md (list lines in notes).

</topic>

<topic id="proof" status="active" summary="Proof">

## Proof

- Build on C: (`--disk ssd`, owner smb-view-polish-1). Validation round R42 holds the C: lock with priority: code first, your run waits (expected).
- One bundled lane-check per push over the WHOLE harness-view and harness suites with the round's viewer features (`.config/round-features.toml`, rule 4e). Rebase onto origin right before the run. Run log path contains the lane id and the sha.
- GUI lane 1 of max 2.

</topic>
