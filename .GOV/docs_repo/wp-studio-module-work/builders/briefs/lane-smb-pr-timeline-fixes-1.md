---
file_id: LANE-SMB-PR-TIMELINE-FIXES-1
file_kind: lane_brief
updated_at: 2026-11-13T17:08:44Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-TIMELINE-FIXES-1
lane_file: builders/lanes/hsk-studio-pr-timeline-fixes-1.json
crates: pulse, harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Open Nested lands on the frame the nested layer shows, Next Marker never sticks, and a group move of both clips of a transition keeps it">

## Target

Video timeline correctness: Open Nested lands on the frame the nested layer shows, Next Marker never sticks, and a group move of both clips of a transition keeps it whatever the source length (hsk-studio-pulse, hsk-studio-harness-view timeline)

Why this lane: three validator findings still open as ledger candidates, each with a probe to port, verbatim: 2026-11-06T16:40:02Z "F-R58-6: Open Nested on a reversed nested clip lands one frame past the picture (harness-view timeline/nest.rs:370); open at the nested layer's source_tick. Small, egui-free, view state"; 2026-11-06T17:40:58Z "F-PR59-2: NextMarker can stick on a reversed audio in point at a sequence's end or on a position off the frame grid; forward 2x has the same class at every commit (annotate.rs:597, playback.rs:542)"; 2026-11-05T14:24:14Z "F-R56-12: a group move of both clips of a transition keeps it with a known source length and drops it with an unknown one, though the cut does not change".

Facts are READ by the wave-next-2 planner at origin 89f27330 (read-only, not run; `git diff a77c4ee4 89f27330` touches only harness-view src/view/menu_tree.rs). Treat them as claims and check them.
- Spec: STU-VID-035 (module 14 :23855, the playhead and marker navigation), STU-VID-029 (:23758, group move).
- `harness-view/src/view/timeline/nest.rs:362-383` timeline_open_nested: the playhead inside the clip maps through `clip_source_at(clip, p, &ext, true)` with NestedExtents and pulse UnknownExtents (`:369-374`); the R58 repair direction is to open at the nested layer's source_tick (the program monitor's frame_layers_nested path). nest.rs last changed e76bfe0f (2026-11-07, id minting), not the mapping.
- `pulse/src/annotate.rs:593-604` next_marker returns the first marker_spans start strictly after `after`; prev_marker `:606`. harness-view `view/timeline.rs:1472-1476` dispatches NextMarker and PreviousMarker. F-PR59-2 file:line are at R59 (annotate.rs:597, playback.rs:542); at 89f27330 pulse `src/playback.rs:536` scrub and `:557` step. annotate.rs has no commit after 329e2b9a (2026-11-06T19:07Z); R59 claim probe c5 recorded F-PR59-2 unchanged.
- `pulse/src/merge_sync.rs:434-450` plan_move_clips (`:438`) runs the crate's propose(..) (imported at `:82`; its module not read here); the transition between two moved clips is kept or listed in dropped_transitions depending on the source extent. merge_sync.rs last changed c4d06841 (2026-11-06, marker alignment). State at the base UNVERIFIED by run.
- Probes to port: `../Handshake_Artifacts/WP-KERNEL-STUDIO/validator-r58-probe/hsk-studio-harness-view/r58_probe_viewer.rs:168-200` r58_v1_open_nested_on_a_reversed_nested_clip (prints, and uses egui::Context: port it egui-free); `validator-r59-probe/hsk-studio-pulse/tests/r59_probe_claim.rs:328` r59_c5_next_marker_at_the_end; `validator-r56-probe/p4_merge_tail.rs:189-205` r56_m7_group_move_keeps_a_transition_between_its_members. They print rows; the ports assert the finding's expected behaviour.

Red test (must fail first):
`pulse/tests/timeline_fixes.rs` (new): m7 asserts the transition between both moved clips is kept and dropped_transitions is empty for a known and an unknown extent, undo restoring; c5 asserts Next Marker from a reversed audio in point at a sequence's end and from a position off the frame grid moves forward or answers none, never the same position. `harness-view/tests/viewer_timeline_fixes.rs` (new, [viewer], egui-free) asserts Open Nested on a reversed nested clip opens at the nested frame the program monitor shows, for each reversed speed of the probe. If a port passes at the base, report it with its log as already fixed and drop that item; never invent a red.

Fix contract:
- Open Nested and Next Marker are view state: no undo step, no actor. The group move stays one undo step named by its catalog id (common ruling 10) with the caller's context.
- Open Nested maps through the same function the monitor uses; marker navigation compares on the presented frame grid; a group move keeps a transition both of whose clips move by the same delta, whatever the extents.
- 2 to 5 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: these items only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Cargo (wave-parity-2-common topic cargo-discipline, Operator 2026-11-13): build and test only the touched crates and test binaries (`-p <crate> --test <name>`); no --workspace outside the one bundled lane-check; no --release; never a new CARGO_TARGET_DIR: the mapped target only (`--owner smb-folio-angle-unbounded-1 --disk d`). ONE HOLD PER PHASE. Focused: `cargo nextest run -p hsk-studio-pulse --test timeline_fixes --test merge_sync --test media_markers` and `cargo nextest run -p hsk-studio-harness-view --features viewer --test viewer_timeline_fixes --test viewer_timeline_nest --test viewer_timeline_nest_2`. Bundled: the pulse suite and the harness-view [viewer] suite in one lane-check, plus the harness test binaries that drive the timeline (git grep NextMarker, OpenNested, plan_move_clips in crates/hsk-studio-harness/tests; send the list with REVIEW READY).
MENU COVERAGE: this lane adds or renames no catalog command; say so in the reports.

Out of scope: multicam active-angle playback (candidate 2026-11-07T07:57:24Z), marker synchronize at -2 (candidate 2026-11-06T18:47:15Z), the appearing-markers walk cost F-R58-3, timeline_ui.rs (egui, quarantined), harness-view view/mod.rs and view/commands.rs (SMB-FIX-LAYER-RECORD-PRUNE-1's), any Cargo.lock change without asking.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-PROTOTYPE-PLAY-CANCEL-1, 2 SMB-PR-TIMELINE-FIXES-1, 3 SMB-MOTION-IMPLIED-VALUES-1, 4 SMB-GUI-GESTURE-GATE-1; the lead may interleave SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 2 of 4.

ROUND (research/decision-20261113-r60-round-fit.json, option d): STUDIO-R60 attempt 4 runs on the D: round slot and holds the D: lock (class round, FIFO priority) for many hours. Your lock-needing steps queue behind it: that is expected. Start the lock-needing step in the background and wait to be notified. Never work around the lock, never poll in a loop, and never run cargo outside a hold. Read code and edit while you wait. ONE HOLD PER PHASE (wave-next-2, 2026-11-13): the four lanes share the D: lock with each other and with the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE and CARGO DISCIPLINE). Lanes land one at a time on the lead's release.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: the group move's existing edit frame; navigation is view state with no frame of its own; codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the group move's commit takes the caller's context; navigation and Open Nested are view state (no actor)", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-folio-angle-unbounded-1`, branch `lane/smb-pr-timeline-fixes-1`, base 89f27330)

hsk-studio-pulse/src/annotate.rs; hsk-studio-pulse/src/merge_sync.rs and src/timeline_edit.rs (transition keep path only); hsk-studio-harness-view/src/view/timeline/nest.rs; hsk-studio-harness-view/src/view/timeline.rs (NextMarker and PreviousMarker dispatch only); hsk-studio-pulse/tests/timeline_fixes.rs (new); hsk-studio-harness-view/tests/viewer_timeline_fixes.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-folio-angle-unbounded-1 --disk d`): step 0; pulse suite; harness-view [viewer] suite; the harness test binaries that drive the timeline (list them with REVIEW READY); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-folio-angle-unbounded-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
