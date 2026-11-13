---
file_id: LANE-SMB-FIX-ANIMATION-VALUES-1
file_kind: lane_brief
updated_at: 2026-11-13T14:09:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-ANIMATION-VALUES-1
lane_file: builders/lanes/hsk-studio-fix-animation-values-1.json
crates: harness-core
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="property.enable_animation keys the current value and property.disable_animation restores the prior static value">

## Target

property.enable_animation keys the current evaluated value and property.disable_animation returns the prior static value (STU-MOT-038, STU-MOT-020b; harness-core motion_commands.rs)

Why this lane: R60 attempt 3 probe failures on READY lane SMB-MOTION-COMMANDS-1 (validation/round-R60.json attempt_3.probes.failures['SMB-MOTION-COMMANDS-1'], both MEDIUM motion-row-takes-the-callers-value; ledger PRIORITY candidate 2026-11-13T08:33:54Z). Probes hsk-studio-harness::r60a3g_motion_values: r60a3g_enable_animation_keys_the_current_value (opacity static 40; enable_animation with value [90] at 1 s keys 90; property.read at 3 s gives [90]) and r60a3g_disable_animation_returns_an_absent_anchor (no anchor stream; keyframe.set anchor [37, 11] at 0, then disable_animation leaves an anchor stream with static [37, 11]). Log ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/logs/h2-g1.log.

Facts are READ by the fix-wave planner at origin c8fca900 (read-only, not run). R60 file:line are at b57c82d1; `git diff b57c82d1 c8fca900` touches none of the files named below unless stated. Treat them as claims and check them.
- Spec: STU-MOT-038 (:27695-27705) enabling animation creates the first keyframe at the current time, adding a keyframe captures the current evaluated value and never changes the render, disabling returns to static with the retained value of STU-MOT-020b (:27569-27571: exactly the prior static value).
- `harness-core/src/motion_commands.rs:874-884` enable_animation keys `value.unwrap_or(static)`; `:885-890` disable_animation clears the keys only; slot (`:1784`) creates a missing stream from the default or the caller's hint (`:1811-1818`), so keyframe.set (key_set `:2118`) on anchor or position stores the caller's value as the static value.
- Row: `harness-core/src/action/commands/motion.rs:92` property.enable_animation takes KEY_AT (value included, `:35`); KEY_AT_CURRENT (`:37`) has none.
- Kept green: `harness/tests/motion_commands.rs:260` (enable without value) and `:298-301` (disable, then nothing_to_change).
- Sibling sites: keyframe.add_at_current (`:2085`), current_slot (`:2081`), the text and time_remap slots.

Red test (must fail first):
Port both probes (source ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/files/hsk-studio-harness/tests/r60a3g_motion_values.rs) into `harness/tests/fix_animation_values.rs`. They fail at the base.

Fix contract:
- enable_animation keys the current evaluated value; a caller value is refused typed (if you change the row's parameters instead, ask the lead: manual and catalog tests pin rows).
- A stream a keyframe created keeps the prior static value (an implied anchor or position stays implied) and disable_animation restores it: in the probe, no anchor stream after the disable.
- Undo and redo exact; save and reopen keep it.
- 1 to 3 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: this defect only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Out of scope: RF-01, render-gpu, panels and drawing, any Cargo.lock change without asking the lead, and the other R60 findings named here.
Named out of scope: F-R60-42 (set_parent variants), F-R60-24 (adjustment layer remove, SMB-MOTION-COMMANDS-2).

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-FIX-DIAG-SINK-ABORT-1, 2 SMB-FIX-NEGATIVE-SPACE-1, 3 SMB-FIX-HOST-TERMINAL-FRAME-1, 4 SMB-FIX-ANIMATION-VALUES-1, 5 SMB-FIX-CAPTURE-ADMISSION-1, 6 SMB-FIX-EXPORT-ADMISSION-1, 7 SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 4 of 7.

NO CARGO PHASE 1 WAIT (fix wave 2026-11-13): the validation round is stopped (C: is at the grant limit), so no round holds or queues for the lock; the fix lanes share the D: lock only with each other. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16), in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-sequence-session-1`, branch `lane/smb-fix-animation-values-1`, base c8fca900)

hsk-studio-harness-core/src/motion_commands.rs; hsk-studio-harness-core/src/action/commands/motion.rs (the property.enable_animation row only, if the lead agrees); hsk-studio-harness/tests/fix_animation_values.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-sequence-session-1 --disk d`): step 0; harness-core suite; harness (default features) --test motion_commands and --test fix_animation_values; every process test that drives property.enable_animation if its refusal order changes (common ruling 1; git grep at c8fca900 finds only tests/motion_commands.rs, which drives Run directly); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-sequence-session-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
