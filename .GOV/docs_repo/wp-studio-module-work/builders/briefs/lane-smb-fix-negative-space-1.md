---
file_id: LANE-SMB-FIX-NEGATIVE-SPACE-1
file_kind: lane_brief
updated_at: 2026-11-13T14:09:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-NEGATIVE-SPACE-1
lane_file: builders/lanes/hsk-studio-fix-negative-space-1.json
crates: layout
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A negative space_before never places a line above its column top, and what does not fit is overset">

## Target

A negative space_before never places a line above its column top, and the lines that do not fit are reported as overset (hsk-studio-layout)

Why this lane: R60 attempt 3 probe failure on READY lane SMB-TYPE-INDENT-SPACING-1 (validation/round-R60.json attempt_3.probes.failures['SMB-TYPE-INDENT-SPACING-1'], MEDIUM negative-space-places-lines-outside-the-frame; ledger PRIORITY candidate 2026-11-13T08:33:54Z). Probe hsk-studio-layout::probe_r60a3_i_layout probe_negative_space_before_places_lines_above_the_frame: 40 one-line paragraphs, paragraph 2 with space_before -1000 pt, frame 120 x 100 pt, FirstFit: 40 lines placed, min baseline -966 against the first line's 10, overset None. Log ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/logs/h3-i34.log.

Facts are READ by the fix-wave planner at origin c8fca900 (read-only, not run). R60 file:line are at b57c82d1; `git diff b57c82d1 c8fca900` touches none of the files named below unless stated. Treat them as claims and check them.
- `layout/src/threaded_reflow.rs:1147` adds the paragraph gap as a signed offset (`cursor.extra = sum(cursor.extra, gap)`); `:373` baseline_of adds `cur.extra`; next_slot (`:396`) checks the bottom only, so a large negative gap lifts lines above the column top and lets every later line fit.
- STU-TYP-185 (spec :8823-8835) gives space_before no bounds (UNKNOWN). A clamp at the column top is a Studio reading: label it; InDesign parity UNVERIFIED (D5).
- Kept green: `layout/tests/indent_spacing.rs:354-368` space_before_and_after_add_one_gap_dropped_at_a_column_top (negative space moves a line up inside the column: baselines 10, 32, 42) and `:475-514` hostile_indent_and_space_values_give_typed_answers.
- Sibling sites: the anchored-object push after the gap (`:1152`), space_after, span and split space, and the space-before recompute after column balancing (SMB-TYPE-INDENT-SPACING-1 ruling F2).

Red test (must fail first):
Port the probe (source ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/files/hsk-studio-layout/tests/probe_r60a3_i_layout.rs, test at :846-861) into `layout/tests/fix_negative_space.rs` for all four composers: no baseline above the column's first baseline, and `overset` is Some. It fails at the base.

Fix contract:
- A negative gap never moves a baseline above the column's first baseline (clamp at the column top), on every composer and after column balancing.
- The lines that do not fit are overset, as for any other story.
- The two kept tests stay green unchanged.
- 1 to 3 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: this defect only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Out of scope: RF-01, render-gpu, panels and drawing, any Cargo.lock change without asking the lead, and the other R60 findings named here.
Named out of scope: F-R60-27 (whole-paragraph delete moves its override, textflow.rs), every other indent or spacing finding.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-FIX-DIAG-SINK-ABORT-1, 2 SMB-FIX-NEGATIVE-SPACE-1, 3 SMB-FIX-HOST-TERMINAL-FRAME-1, 4 SMB-FIX-ANIMATION-VALUES-1, 5 SMB-FIX-CAPTURE-ADMISSION-1, 6 SMB-FIX-EXPORT-ADMISSION-1, 7 SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 2 of 7.

NO CARGO PHASE 1 WAIT (fix wave 2026-11-13): the validation round is stopped (C: is at the grant limit), so no round holds or queues for the lock; the fix lanes share the D: lock only with each other. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16), in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none: pure layout engine; typed overset and refusal outcomes only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not_applicable: no document mutation (layout engine)", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-retime-hold-1`, branch `lane/smb-fix-negative-space-1`, base c8fca900)

hsk-studio-layout/src/threaded_reflow.rs; hsk-studio-layout/tests/fix_negative_space.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-retime-hold-1 --disk d`): step 0; layout suite (the crate you change); no dependent suites unless a public signature changes; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-retime-hold-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
