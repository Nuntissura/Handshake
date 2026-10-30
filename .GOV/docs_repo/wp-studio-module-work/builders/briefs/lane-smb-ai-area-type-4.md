---
file_id: LANE-SMB-AI-AREA-TYPE-4
file_kind: lane_brief
updated_at: 2026-10-30T13:40:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-AREA-TYPE-4
lane_file: builders/lanes/hsk-studio-ai-area-type-4.json
crates: [hsk-studio-type, hsk-studio-harness-view (test + drag fix only if the cause is there)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Area text in a non-rectangular frame places nothing">

## Target

SMB-AI-AREA-TYPE-1 (PASS_V1, lane file `builders/lanes/hsk-studio-ai-area-type-1.json`, `claim_scope`) claims area text in any closed container:
- non-zero fill rule;
- per-line bands intersected with the container on 5 scanlines;
- the widest interval is used.

SMB-VIEW-POLISH-1 run 2 found that dragging ONE corner of an area-text frame in the window, which turns the rectangle into a slanted quadrilateral, makes the story place 0 of 32 glyphs.
- The lane changed its test to grow the frame as a rectangle instead (commit d208bc0e, `crates/hsk-studio-harness-view/tests/viewer_view_polish.rs`; see the diff and its parent 863d5962).
- The cause is NOT_INSPECTED.

1. **Reproduce at engine level** in `hsk-studio-type`: build the same quadrilateral the viewer drag produced and lay out the same story.
   - Read the viewer drag code to get the exact coordinates.
   - Write the failing test first. It must be red at d208bc0e.
2. **Find the cause** from evidence. Candidates, none verified:
   - band intersection on slanted edges;
   - the inset shrink;
   - flattening;
   - the auto-size path, if it does not fall back to `off` on non-rectangles;
   - first-baseline placement;
   - the viewer drag itself producing degenerate geometry (self-intersection, zero area, reversed or duplicated points).
3. **Fix it where the cause is.**
   - Engine: fix in `hsk-studio-type`.
   - Viewer drag: fix in harness-view's frame-drag code.
   - Area text must place glyphs in convex and concave slanted frames whose bands are wide enough for a word.
   - Add engine tests:
     - a parallelogram;
     - a trapezoid;
     - a concave L-shape;
     - a triangle with a too-narrow tip, which must report overset, not drop text;
     - a hand-computed check of the first line's interval in one case.
4. **Restore the original one-corner-drag version** of the viewer echo test alongside the rectangle version. It must pass.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-area-type-4`, base d208bc0e)

- `hsk-studio-type`: the area-text layout module(s) and new `tests/area_text_shapes.rs`.
- `hsk-studio-harness-view`: `tests/viewer_view_polish.rs`. Touch the frame-drag code in `src/view/vector.rs` or `src/view/text.rs` only if step 2 places the cause there.
- Do not touch:
  - `hsk-studio-pulse`, the timeline and the mixer (SMB-AU-MULTITRACK-3 is running);
  - `hsk-studio-harness-core` and `hsk-studio-layout` (SMB-R42-FOLLOWUPS-1 is running);
  - root inputs and the Master Spec.

</topic>

<topic id="proof" status="active" summary="Proof and run timing">

## Proof and run timing

- Code and test now, with focused `cargo check`, clippy and nextest of `hsk-studio-type` on D: (`--disk d`).
- `hsk-studio-type` is upstream of composite, pulse and the viewer, so its bundled run is large.
  - Do not start the bundled run until the lead messages you with the disk and the base.
  - A C: validation round (R43) is about to start, and no builder runs on C: during a C: round.
- The bundled run covers:
  - type's whole suite and its direct dependents;
  - harness-view's whole suite with the round's viewer features, if you changed it (rules 4a and 4e).
  - Name the log per RUN LOGS.
- Illustrator behaviour stays UNVERIFIED (D5).

</topic>
