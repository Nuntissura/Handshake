---
file_id: BUILDER-BRIEF-LANE-SMB-PR-RATE-STRETCH-1
file_kind: builder_brief
updated_at: 2026-10-24T04:23:07Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-RATE-STRETCH-1
lane_file: builders/lanes/hsk-studio-pr-rate-stretch-1.json
crates: [hsk-studio-pulse, hsk-studio-harness-view]
model: opus
effort: medium
author_role: lane planner brief (read-only planning, origin tip b9c311e5); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
depends_on: [SMB-TIMELINE-TRIM-1 READY at b9c311e5 (trim tools, achieved-delta search)]
---

<topic id="target" status="active" summary="App, rows, expected gain, model">

## Target

- App: Premiere Pro. TRIM-1 left "rate stretch not built (pulse has no speed planner)" and "slip delta = frames x sequence ticks per frame (exact only at unit speed)" (lane file gaps). Rows: PR-09 "Speed / duration, time remapping, optical-flow retiming" (PARTIAL; audit anchor `folio/src/timeline.rs:260 (Speed)`) and the rate-stretch member of the trim vocabulary (strict UI row PR-03).
- Expected gain: modest. PR-09 gains its speed/duration edit (planner + Speed/Duration command + Rate Stretch tool) but stays PARTIAL unless the auditors drop clip time-remapping and the frame-interpolation choice (not stored, see gaps): engine 0 to +1.0 pp Premiere; GUI: rate stretch and Speed/Duration doable in the window (strict rows PR-03/PR-09), no new pooled workflow (Trimming is already counted for TRIM-1).
- Model / effort: opus / medium (one pulse planner with exact rational arithmetic over existing `SpeedMath`, plus a tool in a panel that already has the trim grammar).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-rate-stretch-1`)

- Pulse: NEW `crates/hsk-studio-pulse/src/rate.rs` (`plan_rate_stretch`, `plan_speed_duration`), `src/lib.rs` two lines (`mod rate;` / `pub use rate::*;`), NEW `tests/rate_stretch.rs`. Do not add a variant to `TimelineEdit` (a public enum built and matched across crates); a separate planner keeps every existing caller unchanged.
- Harness-view: `src/view/timeline.rs`, `src/view/timeline_ui.rs`, `src/view/timeline/trim.rs` (slip delta at speed), NEW `src/view/timeline/rate.rs` (declared inside `timeline.rs`), NEW `tests/viewer_timeline_rate.rs`. Single writer of the timeline files tonight; a R40 finding against SMB-TIMELINE-TRIM-1 or SMB-TIMELINE-UI-1 is remediated by this lane (separate commits recorded under that lane id, V4-15).

## Do NOT touch

- SMB-FIG-EDIT-1 files: `src/view/shell.rs`, `src/view/mod.rs`, `src/view/file_dialog.rs`, `src/view/vector.rs`, `src/view/design*`, `crates/hsk-studio-harness/RUN-UI.md` (write the manual text in the rustdoc of `timeline/rate.rs` and list the RUN-UI.md lines for the lead).
- SMB-LR-HEALING-1 files: `src/view/develop_entry.rs`, develop crates.
- `pulse/src/multicam.rs`, `pulse/src/timeline_edit.rs`, `pulse/src/assemble.rs`, `pulse/src/rational.rs` (call them), Folio (no clip field: `StudioClip {..}` literals span 30 files), session, package, root files.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-VID-026]` row (grep "| Rate stretch |"): "| Rate stretch | Change a clip's `duration` by changing its `speed`, keeping the full source window. | Bounded by the speed limits of [STU-VID-031]. |"; clause head: "Each is a typed command on `event_family = studio.sequence`, each is one reversible history step, and each has a defined behaviour when it would produce an illegal result."
- `[STU-VID-031]`: "`speed` is a rational multiplier and retiming is a stated algorithm choice. A clip's speed and duration are two views of one value: fixing either derives the other. Reverse is a negative speed. `Maintain audio pitch` is an independent boolean." (+ the `frame_sampling` / `frame_blending` / `optical_flow` table).
- `[STU-VID-090]` obligation 1: "The typed form takes the edit point, the mode ([STU-VID-026b]) and a signed tick or frame delta, and returns the achieved delta, which may be smaller than requested when a limit was reached." Obligation 3: "A determinate result is required for every refusal."
- `[STU-VID-040]`: "A trim is one step, not one step per frame dragged."
- Command rows (grep each): "| `cmd.clip.speed` | Speed/Duration... | `Ctrl+R` |"; "| `cmd.tools.05ratestretch` | _(no menu label; executable-only)_ | `R` |". Tool registry (grep "| Rate Stretch | `rate_stretch`"): "| Rate Stretch | `rate_stretch` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Rate Stretch |"; family row: "| `timeline_edit` | Timeline edit | The trim grammar: ripple edit, rolling edit, slip, slide, rate stretch, razor, remix, track select forward, track select backward. | 9 |".

</topic>

<topic id="predecessors" status="active" summary="What exists and constraints">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-timeline-trim-1.json` (capabilities 1-4, gaps), `hsk-studio-pr-multicam-2.json` `notes.declared_readings` (speed readings this lane must not contradict: piece speed = exact reduced product, `multicam_speed_inexact`, reverse refused for multicam flatten/resolve; a rate stretch on a multicam clip changes its `speed`, which multicam resolves at non-unit forward speed: keep reverse rate stretch on multicam clips a named refusal `rate_stretch_multicam_reverse` so no state is created that multicam refuses to read).
- Code: pulse `src/rational.rs:16 reduced_speed`, `:33 trait SpeedMath` (`duration_for_span`), `src/assemble.rs:40 ChangeSpeed` ("Keep both windows; the clip speed becomes source span / sequence span (sign kept)": the same rule as rate stretch; reuse its helpers, do not copy); folio `src/timeline.rs:260 Speed` (non-zero, reduced; duration derived exactly, `derived_duration`); harness-view `view/timeline.rs:85` tool ids `timeline.tool.<key>`, `:429-526` drag targets and commands, `view/timeline/trim.rs` (achieved-delta halving search, slip delta).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Pulse planners: `plan_rate_stretch(sequence, clip_id, edge: Head|Tail, new_edge: Ticks)` keeps `source_in/source_out`, sets `speed = source span / new duration` (sign kept, exact reduced rational), moves only the dragged edge; `plan_speed_duration(clip_id, speed | duration, reverse: bool)` for the Speed/Duration command (non-ripple). Limits (declared reading, see spec gaps): the duration is at least one sequence frame and frame-aligned, the speed is representable (`i64` numerator, reduced), no overlap with the neighbour, linked partners stretch together; refusals named (`rate_stretch_overlap`, `rate_stretch_below_one_frame`, `rate_stretch_unrepresentable`, `rate_stretch_multicam_reverse`). One step with inverse, as the other pulse edits. Tests: `rate_stretch_keeps_the_source_window_and_derives_speed`, `speed_duration_round_trip_exact`, `reverse_is_a_negative_speed`, `stretch_refusals_are_named`, `linked_clips_stretch_together`.
2. Timeline tool and command: tool `rate_stretch` in the trim-mode selector (`timeline.tool.rate_stretch`, binding `R` from `cmd.tools.05ratestretch`), edge drag = rate stretch with the TRIM-1 achieved-delta search, one undo step per drag; `TimelineCommand::RateStretch { clip_id, edge, delta_frames }` and `SpeedDuration { clip_id, speed | duration, reverse }` (Ctrl+R opens a Speed/Duration form in the panel; `cmd.clip.speed`); clip blocks show the speed when not 1 (label). Tests (settle before aiming, F-TL-2): `rate_stretch_drag_changes_speed_not_source`, `ctrl_r_speed_duration_sets_speed_and_undo_restores`, `r_selects_the_rate_stretch_tool`, `achieved_delta_stops_at_the_neighbour`.
3. Slip at non-unit speed (TRIM-1 gap): slip source delta = frames x ticks per frame x |speed| exactly; inexact -> named refusal `slip_inexact_at_speed`. Test: `slip_at_double_speed_moves_two_source_frames_per_frame`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Pulse engine tests with exact tick arithmetic; harness-view headless egui + AccessKit (pattern of `tests/viewer_timeline.rs`), assertions through the Folio sequence read back and EDIT > Undo.
- NOT_PROVEN (`NOT_PROVEN:` lines): playback/render of a stretched clip's frames in the window (program monitor has no frame decoder on document sequences, RUN-UI.md:187-188); audio at non-unit speed; Premiere's rate-stretch limits and cursor zones (no Premiere oracle).

## Bundled lane-check (C: SSD, rule 4a)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-pr-rate-stretch-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-pr-rate-stretch-1 --disk ssd -- cargo nextest run -p hsk-studio-pulse -p hsk-studio-harness-view -p hsk-studio-harness --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: pulse, harness-view, harness (its shell suites open the timeline). Rule 4b: no Folio field: package suite not required. Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, gaps, conflicts, deadline">

## Edges and dependencies

- None. No crates.io dependency.

## UNVERIFIED (D5)

- Premiere rate-stretch limits (vendor percentage range), whether rate stretch ripples, Speed/Duration dialog defaults; no Premiere oracle.

## Spec gaps (Operator decides Master Spec edits)

- STU-VID-026 says rate stretch is "Bounded by the speed limits of [STU-VID-031]", but STU-VID-031 declares no numeric speed limits: the lane bounds by representability, one-frame minimum and neighbours (declared) and lists the gap.
- STU-VID-031's `Maintain audio pitch` boolean and frame-interpolation choice have no storage on `StudioClip` (no Folio field; a record set beside the document is the pattern, a later lane): the Speed/Duration form shows them as unavailable with that reason, not as silent defaults.

## Risks and conflicts

- SMB-TIMELINE-TRIM-1 is READY (R40) on these files: single-writer rule above. V4-14: the lead pins R40's candidate at or before b9c311e5 for TRIM-1 or lists these commits.
- Pulse is shared with SMB-PR-MULTICAM-2 (READY, `multicam.rs`): disjoint files; `lib.rs` additive lines only.
- SMB-FIG-EDIT-1 and SMB-LR-HEALING-1 also run the harness-view/harness suites: disjoint files; later pushers rebase and re-run once.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
