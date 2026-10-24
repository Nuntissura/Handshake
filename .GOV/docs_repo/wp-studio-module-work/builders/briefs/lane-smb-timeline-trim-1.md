---
file_id: BUILDER-BRIEF-LANE-SMB-TIMELINE-TRIM-1
file_kind: builder_brief
updated_at: 2026-10-24T01:17:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-TIMELINE-TRIM-1
lane_file: builders/lanes/hsk-studio-timeline-trim-1.json
crates: [hsk-studio-harness-view]
model: opus
effort: medium
author_role: lane planner brief (read-only planning, origin tip 1e3ae3d4); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, parity rows, expected gain, model">

## Target

- App: Premiere Pro. Workflow "Trimming (ripple/roll/slip/slide)" is ENGINE_ONLY: the timeline panel trims in/out with `ripple:false` only (parity-20261023-video-audio.json, assessment_vs_gui_rows.premiere_pro). Pulse already plans every needed edit (`crates/hsk-studio-pulse/src/timeline_edit.rs:61 enum TimelineEdit`: `TrimIn/TrimOut {ripple}`, `Roll`, `Slip`, `Slide`), all engine-VALIDATED.
- Rows: workflow "Trimming"; strict UI row PR-03 Trim (trim modes in the window); STU-VID-035a nudges.
- Expected gain: Premiere GUI pooled 45 -> 50 % unjudged (55 % judged); 108-workflow pool +0.5 point unjudged, +1 judged. No engine change.
- Model / effort: opus / medium (GUI over existing, validated pulse planners; no new edge, no new dependency).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch, single-writer rule">

## Owned paths (lane worktree `wt-studio-lanes/smb-timeline-trim-1`)

- `crates/hsk-studio-harness-view/src/view/timeline.rs` (additive: new `TimelineCommand` variants, `DragKind` members, key bindings in `key_command`, `spec_command` ids, state_json fields) and `src/view/timeline_ui.rs` (trim-mode toolbar, edit-point hit targets).
- NEW `src/view/timeline/trim.rs` and `src/view/timeline/trim_session.rs`, declared with `mod trim; mod trim_session;` inside `timeline.rs` (NOT in `view/mod.rs`).
- NEW `crates/hsk-studio-harness-view/tests/viewer_timeline_trim.rs` (own fixtures; do not edit `tests/viewer_timeline.rs`, also touched by READY SHELL-2).
- Single-writer rule: while this lane is open it is the ONLY writer of `timeline.rs`/`timeline_ui.rs`. If R39 returns a finding against SMB-TIMELINE-UI-1, the lead assigns that remediation to this lane (V4-15: as a separate commit set recorded under SMB-TIMELINE-UI-1), never to a second agent.

## Do NOT touch

- `src/view/shell.rs`, `src/view/mod.rs` (SMB-AE-KEYFRAME-TIMELINE-1 owns the additive shell hunks tonight), `src/view/keyframe_timeline*`, `src/view/hosted.rs`, `src/view/file_dialog.rs`, `src/view/timeline_export/**`, `crates/hsk-studio-harness/RUN-UI.md` (write the manual text into the rustdoc of `timeline/trim.rs` and list the RUN-UI.md lines to add in the lane notes; the lead merges them after SMB-AE-KEYFRAME-TIMELINE-1), and every other crate (`hsk-studio-pulse` included: SMB-PR-MULTICAM-1 owns pulse tonight; if a pulse change is unavoidable, stop and report).

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-VID-026]` "The normative trim vocabulary. Each is a typed command on `event_family = studio.sequence`, each is one reversible history step, and each has a defined behaviour when it would produce an illegal result." Rows used: "Ripple trim in / out | Move a clip's head or tail and shift all later content on sync-locked tracks by the same amount, so no gap opens. | Refuses when a sync-locked track cannot absorb the shift."; "Roll | Move the edit point BETWEEN two adjacent clips: one lengthens by exactly what the other shortens. Sequence duration unchanged. | Refuses when either side would exceed its available source."; "Slip | Change a clip's `source_in` and `source_out` by the same delta, keeping `timeline_start` and `duration` fixed."; "Slide | Move a clip along the timeline, lengthening the previous clip and shortening the next (or vice versa) by the same amount."; "Extend edit to playhead | Roll the nearest edit on targeted tracks to the playhead."; "Trim forward / backward by one or by many | Nudge the current trim by one frame or by the configured multi-frame step. | Clamps to the operation's own limits above." Rate stretch is NOT in this lane (pulse has no speed planner; declare gap).
- `[STU-VID-026a]` "A trim session may be entered on a selected edit point, showing the outgoing and incoming frames side by side, with plus and minus adjustments in both a minor (one frame) and a major (multi-frame) step, an in-only, out-only or both-sides focus, a composite preview during the trim, and an explicit rollback that abandons the session. The trim type toggles between ripple and roll on the selected edit. Every one of these is reachable as a typed command and therefore headlessly ([STU-VID-090])."
- `[STU-VID-026b]` "Selecting an edit point is itself a typed operation with a stated mode: select the nearest edit as a trim-in, as a trim-out, as a ripple-in, as a ripple-out, or as a roll."
- `[STU-VID-035a]` "a selected clip nudges left or right by one frame or by the configured multi-frame step ... Slip and slide have their own one-and-many nudge pairs. The multi-frame step is a single preference shared by every \"many\" variant".
- `[STU-VID-040]` "A trim is one step, not one step per frame dragged."
- `[STU-VID-090]` obligation 1 (grep "Every trim operation MUST have a headless form"): "The typed form takes the edit point, the mode ([STU-VID-026b]) and a signed tick or frame delta, and returns the achieved delta, which may be smaller than requested when a limit was reached." Obligation 3: "A determinate result is required for every refusal."
- `[STU-VID-041]` command rows (grep each id): `cmd.timeline.editpoint.trimin`, `.trimout`, `.rippletrimin`, `.rippletrimout`, `.rolledit` (no binding); `cmd.timeline.slip.left.one` `Ctrl+Alt+Left Arrow`, `.slip.left.several` `Ctrl+Alt+Shift+Left Arrow`, `.slip.right.one` `Ctrl+Alt+Right Arrow`, `.slip.right.several` `Ctrl+Alt+Shift+Right Arrow`; `cmd.timeline.slide.left.one` `Alt+,`, `.slide.left.several` `Alt+Shift+,`, `.slide.right.one` `Alt+.`, `.slide.right.several` `Alt+Shift+.`; `cmd.trim.plus.minor`, `cmd.trim.plus.major`, `cmd.trim.minus.minor`, `cmd.trim.minus.major`, `cmd.trim.focusin`, `cmd.trim.focusout`, `cmd.trim.focusinandout`, `cmd.timeline.composite.preview.during.trim` (no bindings).

</topic>

<topic id="predecessors" status="active" summary="What exists and test patterns">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-timeline-ui-1.json`: capabilities, gaps ("no ripple / roll / slip / slide trim modes, no trim session (STU-VID-026a/026b), no nudge (STU-VID-035a)"), F-TL-1/F-TL-2, waiver HBR-STOP-005, diagnosis R1 (drag tests aimed at the first egui frame).
- Code: `timeline.rs:61-76` widget ids (`clip_edge`), `:364 enum DragKind {Move, TrimIn, TrimOut}`, `:411 drag_target`, `:458 drag_command`, `:503-548 TimelineCommand::key/spec_command/key_command`, `:651-668 AtLeast` (SourceExtents port; unknown extents make slip/roll/slide refuse: handle that as a determinate refusal, not a silent no-op), `:789 open_timeline`; pulse `timeline_edit.rs:31 trait SourceExtents`, `:61 enum TimelineEdit`.
- Test helpers to copy: `tests/viewer_timeline.rs:168-248` (`input`, `frame`, `key`, `button`, `drag`, `settle`, `ak_find`, `ak_click`, fixture `sequence_package`).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; push per green bundled run)

1. Typed trims with achieved delta (headless first): `TimelineCommand::{RippleTrimIn, RippleTrimOut, Roll, Slip, Slide} { clip_id, delta_frames }` and `SelectEdit { mode: EditMode::{TrimIn, TrimOut, RippleIn, RippleOut, Roll}, nearest_to: Ticks }`; each submits ONE pulse `TimelineEdit` through the existing `Run::submit_sequence` path (one undo step). Achieved delta: requested delta, or the largest delta toward zero that pulse accepts (bounded search, at most 64 planner calls, deterministic), returned in the command result and `state_json()["timeline"]["last_trim"]` with the named refusal when it is 0. Tests: `ripple_trim_shifts_later_clips_on_sync_locked_tracks`, `roll_moves_the_edit_between_neighbours`, `slip_changes_source_window_only`, `slide_keeps_source_and_resizes_neighbours`, `achieved_delta_is_clamped_and_refusals_are_named`.
2. Drag trim modes in the panel: a trim-mode selector (selection / ripple / roll / slip / slide) in the timeline toolbar; edge drags use the mode (ripple trim on a clip edge, roll on a shared edit, slip/slide by dragging the clip body); one undo step per drag; snapping as for existing drags. Tests (settle before aiming, F-TL-2): `ripple_drag_closes_no_gap_and_undo_restores`, `roll_drag_on_a_shared_edit`, `slip_drag_changes_frames_not_position`.
3. Nudges and bindings (STU-VID-035a, STU-VID-041 rows above): slip/slide one/several with the spec bindings; trim forward/backward one/several; extend edit to playhead (Roll of the nearest edit on targeted tracks); the shared multi-frame step preference (Studio default declared in lane notes as a Studio reading; Premiere default UNVERIFIED). Tests: `spec_bindings_nudge_slip_and_slide`, `extend_edit_to_playhead_rolls_the_nearest_edit`, `one_multi_frame_step_governs_every_several_variant`.
4. Trim session (STU-VID-026a): enter on the selected edit point; minor/major plus/minus, focus in/out/both, toggle ripple<->roll, rollback that abandons the session, commit as ONE history step; outgoing/incoming frame tiles labelled with their source ticks. Tests: `trim_session_adjusts_and_commits_one_step`, `rollback_abandons_the_session`, `focus_and_type_toggle_are_typed_commands`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Headless egui + AccessKit (pattern of `viewer_timeline.rs`; no `egui_kittest`). Settle the layout before every pointer event. Assert edits through the Folio sequence read back from the session and `state_json()`, plus EDIT > Undo restoring the exact prior sequence.
- NOT_PROVEN (`NOT_PROVEN:` lines): outgoing/incoming frame pixels and the composite preview during trim (the program monitor has no frame decoder bound on a document sequence, RUN-UI.md:187-188); real desktop window; Premiere trim-tool cursor zones, default multi-frame step and ripple/roll toggle feel (UNVERIFIED, no Premiere oracle).

## Bundled lane-check (rule 4a whole suites; no `--tests` on the push run)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-timeline-trim-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-timeline-trim-1 --disk d -- cargo nextest run -p hsk-studio-harness-view -p hsk-studio-harness --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: harness-view (changed); harness included because its viewer_shell/viewer_panels suites open the timeline (same set as the SMB-TIMELINE-UI-1 run). Rule 4b: no Folio field -> package suite not required (state it). Loop cap: two failed runs on the same failure, then stop and report verbatim; diagnostics re-runs may add `--tests <names>`.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, conflicts, deadline">

## Edges and dependencies

- None (harness-view -> pulse already allowed and declared).

## UNVERIFIED (D5)

- Premiere behaviour of every trim mode's cursor zones, the default multi-frame step, trim-session frame layout; no Premiere oracle exists (rollup per_app.premiere_pro.oracle "none").

## Risks and conflicts

- SMB-TIMELINE-UI-1 is READY for R39 on the same files: single-writer rule above; rebase after R39 if its remediation lands first.
- Unknown source extents (viewer default) make slip/roll/slide refuse for media without a known length: tests must supply extents through the fixture's `SourceExtents` (as `viewer_timeline.rs:383` does) and one test must prove the determinate refusal without them.
- Transitions pruned by moves (timeline-ui-1 gap) also affect ripple/roll: report pruned transitions in the status line, never silently.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes; a push needs its green bundled run, so while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`) instead of pushing an unverified commit.

</topic>
