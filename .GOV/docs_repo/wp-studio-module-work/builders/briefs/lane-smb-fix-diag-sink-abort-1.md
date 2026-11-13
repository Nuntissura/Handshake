---
file_id: LANE-SMB-FIX-DIAG-SINK-ABORT-1
file_kind: lane_brief
updated_at: 2026-11-13T14:09:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-DIAG-SINK-ABORT-1
lane_file: builders/lanes/hsk-studio-fix-diag-sink-abort-1.json
crates: harness-core
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The diag_sink abort test passes deterministically under load with every assertion kept">

## Target

Make hsk-studio-harness-core::diag_sink an_abort_leaves_the_lines_written_and_no_end_of_run deterministic under D: load without weakening what it asserts

Why this lane: common lead ruling DIAG_SINK FLAKE (2026-11-13, research/lead-rulings-20261108-by-lane.json common[13]) accepts the test only when it passes alone at the same sha and raises it to a priority lane (ledger direction.next_lane_candidates 2026-11-13T08:33:54Z 'PRIORITY: diag_sink abort test load flake'). It failed twice with 'child abort did not end in 60 s': SMB-SCORE-EFFECTS-6 bundled run 1 (C:, 2026-11-11; builders/lanes/hsk-studio-score-effects-6.json) and SMB-LAYOUT-DELETE-CASCADE-1 bundled at 81de18b9 (60.76 s under D: load); alone at 81de18b9 it passed in 13.1 s (builders/lanes/hsk-studio-layout-delete-cascade-1.json known_flake).

Facts are READ by the fix-wave planner at origin c8fca900 (read-only, not run). R60 file:line are at b57c82d1; `git diff b57c82d1 c8fca900` touches none of the files named below unless stated. Treat them as claims and check them.
- `harness-core/tests/diag_sink.rs:567-572` the test; `:460-494` run_child: the child is this test binary run again (`--exact child_entry`), polled with try_wait every 20 ms against a 60 s wall-clock deadline counted from spawn (`:481-490`), then killed, and the test panics.
- `:438-452` child_entry: start_process, record_run_start, one record_op, then `std::process::abort()` (`:447`). `:544-554` assert_died_without_a_record; the stack overflow test (`:557`) uses the same run_child.
- Cause NOT_INSPECTED: where the 13 to 60 s go (start of the test binary, the child's own work, or the abort's exit path) has not been measured; any explanation is UNVERIFIED until measured.

Red test (must fail first):
- First measure: timestamps for spawn, child_entry entered, lines written, abort called and exit observed, for the clean, abort and overflow children, alone and inside your bundled run; record them in the lane notes.
- Then a deterministic red test that shows the current guard fails a child that behaves correctly but slowly (for example an injected delay before the abort, with the deadline as a run_child parameter). It fails at the base.

Fix contract:
- Every assertion stays as strong: nonzero exit, no panic file, lines in order, last line `op`, survey counts (0, 1, 1).
- No #[ignore], no retry loop, and raising the 60 s deadline alone is not the fix (still load-dependent). A guard against a real hang stays.
- If the measured cause is in `harness-core/src/diag_sink.rs`, name it to the lead before editing that file.
- 1 to 3 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: this defect only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Out of scope: RF-01, render-gpu, panels and drawing, any Cargo.lock change without asking the lead, and the other R60 findings named here.
Named out of scope: R60-D-1 (actor and session text in diag lines), any other flake.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-FIX-DIAG-SINK-ABORT-1, 2 SMB-FIX-NEGATIVE-SPACE-1, 3 SMB-FIX-HOST-TERMINAL-FRAME-1, 4 SMB-FIX-ANIMATION-VALUES-1, 5 SMB-FIX-CAPTURE-ADMISSION-1, 6 SMB-FIX-EXPORT-ADMISSION-1, 7 SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 1 of 7.

NO CARGO PHASE 1 WAIT (fix wave 2026-11-13): the validation round is stopped (C: is at the grant limit), so no round holds or queues for the lock; the fix lanes share the D: lock only with each other. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16), in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none new: test harness only; diag_sink's own frames unchanged", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not_applicable: no document mutation (test harness)", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-fix-diag-sink-abort-1`, base c8fca900)

hsk-studio-harness-core/tests/diag_sink.rs; hsk-studio-harness-core/src/diag_sink.rs (only after naming the measured cause to the lead)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-edit-footprint-1 --disk d`): step 0; harness-core --test diag_sink (the whole binary, run once); the harness-core suite only if you change its src; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-edit-footprint-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
