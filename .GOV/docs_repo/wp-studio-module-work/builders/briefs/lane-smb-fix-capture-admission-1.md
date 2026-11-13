---
file_id: LANE-SMB-FIX-CAPTURE-ADMISSION-1
file_kind: lane_brief
updated_at: 2026-11-13T14:09:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-CAPTURE-ADMISSION-1
lane_file: builders/lanes/hsk-studio-fix-capture-admission-1.json
crates: harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="prototype.capture_frame admits its PNG write through the viewer's bound JobPort before the first byte">

## Target

prototype.capture_frame admits its file write through the viewer's bound JobPort before the first byte (harness-view commands/prototype.rs)

Why this lane: R60 attempt 3 probe failure on READY lane SMB-DRIVE-TYPED-2 (validation/round-R60.json attempt_3.probes.failures['SMB-DRIVE-TYPED-2'], HIGH export-job-not-admitted-jobport; ledger PRIORITY candidate 2026-11-13T08:33:54Z). Probe hsk-studio-harness::r60a3_j2_capture_probe r60a3_j2_prototype_capture_frame_asks_the_bound_job_port_before_writing: Viewer::set_job_port(a port refusing every admit); prototype.start_session on fig-prototype-pages.fig Ok; prototype.capture_frame with a path Ok, port asked 0 times, frame.png written. Log ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/logs/h4-j2.log. Common lead ruling 2: every export is admitted through the Viewer's bound JobPort before its first byte, with typed overloaded, canceled and stale outcomes.

Facts are READ by the fix-wave planner at origin c8fca900 (read-only, not run). R60 file:line are at b57c82d1; `git diff b57c82d1 c8fca900` touches none of the files named below unless stated. Treat them as claims and check them.
- `harness-view/src/view/commands/prototype.rs:121-131` capture_frame renders, then write_png (`:41-56`) calls export::write_atomic (576c4d5e); nothing asks the port.
- Helpers to call (do not edit doc_export.rs; SMB-FIX-EXPORT-ADMISSION-1 owns it): `view/doc_export.rs:893` admit_export_job_with, `:902` settle_export_text, `:912` settle_export_job; `view/job_admission.rs:223` job_admission_of(resource) (lead ruling 2026-11-12: resource-less admission is not accepted, so a play session with no open document names a derived resource).
- Kept green: `harness/tests/harness_drive_audio.rs:791-837` drives prototype.capture_frame (written, overwrite_required, parameter_conflict).
- Sibling sites: any other prototype or play path that writes a file.

Red test (must fail first):
Port the probe (source ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/files/hsk-studio-harness/tests/r60a3_j2_capture_probe.rs) into `harness/tests/fix_capture_admission.rs`: a saturated port gives a typed overloaded refusal, no file, the port asked exactly once; an open port writes the file, asked once, ticket released, settled once. It fails at the base.

Fix contract:
- With a path: the cheap checks (path, overwrite_required, parameter_conflict) first, then admit, then render and write, then settle once.
- Without a path no file is written; say in the lane notes whether you admit it (a reading for the lead).
- 1 to 2 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: this defect only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Out of scope: RF-01, render-gpu, panels and drawing, any Cargo.lock change without asking the lead, and the other R60 findings named here.
Named out of scope: F-R60-35 (multitrack.open restore) and other drive-typed findings.

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-FIX-DIAG-SINK-ABORT-1, 2 SMB-FIX-NEGATIVE-SPACE-1, 3 SMB-FIX-HOST-TERMINAL-FRAME-1, 4 SMB-FIX-ANIMATION-VALUES-1, 5 SMB-FIX-CAPTURE-ADMISSION-1, 6 SMB-FIX-EXPORT-ADMISSION-1, 7 SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 5 of 7.

NO CARGO PHASE 1 WAIT (fix wave 2026-11-13): the validation round is stopped (C: is at the grant limit), so no round holds or queues for the lock; the fix lanes share the D: lock only with each other. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16), in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: the job settlement frame of the admitted capture (completed, canceled or failed), codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-distort-effects-1`, branch `lane/smb-fix-capture-admission-1`, base c8fca900)

hsk-studio-harness-view/src/view/commands/prototype.rs; hsk-studio-harness/tests/fix_capture_admission.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-ai-distort-effects-1 --disk d`): step 0; harness-view [viewer] suite; harness [viewer] --test harness_drive_audio and --test fix_capture_admission, with a freshly built examples/studio-harness.exe (common ruling 1: capture_frame's refusal order changes); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-ai-distort-effects-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
