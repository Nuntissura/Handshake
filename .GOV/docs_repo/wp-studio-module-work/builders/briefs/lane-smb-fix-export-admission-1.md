---
file_id: LANE-SMB-FIX-EXPORT-ADMISSION-1
file_kind: lane_brief
updated_at: 2026-11-13T14:09:33Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIX-EXPORT-ADMISSION-1
lane_file: builders/lanes/hsk-studio-fix-export-admission-1.json
crates: harness-view, harness-core
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Batch per-target export admits before the first byte, once per target, and one port slot completes a batch target">

## Target

Batch per-target export: admit before the first byte, one admission per target, and a batch target never needs two slots (harness-view doc_export.rs RecipeTargets, harness-core batch.rs)

Why this lane: R60 attempt 3 probe failures on READY lanes SMB-R60-FOLLOWUPS-1 and SMB-DOC-EXPORT-1 (validation/round-R60.json attempt_3.probes.failures; ledger PRIORITY candidate 2026-11-13T08:33:54Z):
- HIGH batch-target-bytes-before-admission: probe hsk-studio-harness::r60a3_e_first_byte_probe r60a3_e1_batch_target_writes_nothing_in_the_destination_before_admission. A port bound with RecipeTargets::with_job_port lists the destination folder when asked: asks 1, and at admission the folder already held .hsk-77112-0.batch-src.handshake (the whole input document), then refused overloaded and the temp was removed. Log ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/logs/h1-e1.log.
- MEDIUM batch-double-admission-self-starvation: probe r60a3_e_batch_port_probe probe_e2_batch_with_the_callers_port_on_runner_and_exporter. A one-slot port bound as BatchPorts.jobs and RecipeTargets::with_job_port: the runner holds the input's ticket, the target export asks the same port and is refused overloaded (receipt code overloaded, stage output, outcome failed); no PNG. Log ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/logs/h1-e2.log.
Lead direction for this wave: admit before the first byte, with one admission per target. The count of the 2026-11-11 batch-port ruling stands (asks == number of targets; research/lead-rulings-20261108-by-lane.json lanes['SMB-R60-FOLLOWUPS-1']); its placement (the inner viewer's export_document admits) is what writes the checkpoint temp before the ask.

Facts are READ by the fix-wave planner at origin c8fca900 (read-only, not run). R60 file:line are at b57c82d1; `git diff b57c82d1 c8fca900` touches none of the files named below unless stated. Treat them as claims and check them.
- `harness-view/src/view/doc_export.rs:1013-1031` RecipeTargets::write: run.checkpoint(), then create_unique_sibling(path, '.batch-src.handshake') + write_all + sync_all (`:1015-1017`), then a viewer bound to the caller's port opens the temp and export_document admits (`:1021-1025`; export_document_admitted `:869-882`).
- `harness-core/src/batch.rs:957-967` process: with BatchPorts.jobs the runner admits one Edit ticket per input and holds it while write_target runs every target (`:992-997`).
- Kept green unless the lead approves a change (rule 4a): `harness-view/tests/r60_followups_jobs.rs:197-232` r60_j1 (saturated: asked once, no file; open: asks == targets, every ticket released) and `harness-core/tests/action_batch.rs:311-338` a_job_port_refusing_once_triggers_a_retry_and_every_ticket_is_released.
- Sibling sites: the CLI's batch wiring (`harness/src/cli.rs:664` RecipeTargets::new without a port, `:680` BatchPorts) and any other writer that checkpoints into a destination.

Red test (must fail first):
Port both probes (sources ../Handshake_Artifacts/WP-KERNEL-STUDIO/studio-round/validator-R60-a3/probe/files/hsk-studio-harness/tests/r60a3_e_first_byte_probe.rs and r60a3_e_batch_port_probe.rs) into `harness/tests/fix_export_admission.rs`. They fail at the base.

Fix contract:
- Per target, the one admission is asked before anything reaches the destination, the checkpoint temp included; the inner export does not ask again; a refusal is the target's typed code with nothing left in the destination.
- With one port bound as both BatchPorts.jobs and the exporter's port, a one-slot port completes a one-input, one-target batch: the runner's ticket and a target's never need two slots of one port at once.
- Before coding, send the lead the shape (for example the runner releasing or lending its slot for the output stage, or the target admission taking the runner's place) with the asks count it gives.
- 2 to 4 commits.

How to work:
1. Read the code at the file:line above first. Cite spec clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`); use only names already in the spec, MTs or code.
2. Red first: the red test above, run red at the base and committed compiling. A red test is never rewritten so that it passes; the code gets fixed (common ruling 11). No history rewriting (common ruling 12).
3. Then the fix and its focused tests. A mutating step stays one undo step named by its catalog id (common ruling 10) and carries the caller's context.
4. V4-06: the sibling sites named above, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. Small and focused: this defect only. State what you cut. MTs: name any your fix proves in your report; do not edit MT files.

Out of scope: RF-01, render-gpu, panels and drawing, any Cargo.lock change without asking the lead, and the other R60 findings named here.
Named out of scope: F-R60-39 (text-route typed refusals and canceled settlement), F-R60-40 (stem sum order), F-R60-25 (PNG module frame bound).

Landing: one lane at a time on the lead's RELEASED, order 1 SMB-FIX-DIAG-SINK-ABORT-1, 2 SMB-FIX-NEGATIVE-SPACE-1, 3 SMB-FIX-HOST-TERMINAL-FRAME-1, 4 SMB-FIX-ANIMATION-VALUES-1, 5 SMB-FIX-CAPTURE-ADMISSION-1, 6 SMB-FIX-EXPORT-ADMISSION-1, 7 SMB-FIX-LAYER-RECORD-PRUNE-1. This lane lands 6 of 7.

NO CARGO PHASE 1 WAIT (fix wave 2026-11-13): the validation round is stopped (C: is at the grant limit), so no round holds or queues for the lock; the fix lanes share the D: lock only with each other. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16), in the landing order above.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: the export's terminal frame and one job settlement per target, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-folio-angle-unbounded-1`, branch `lane/smb-fix-export-admission-1`, base c8fca900)

hsk-studio-harness-view/src/view/doc_export.rs (RecipeTargets and the export admission helpers); hsk-studio-harness-core/src/batch.rs (process admission, TargetExporter); hsk-studio-harness/tests/fix_export_admission.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-folio-angle-unbounded-1 --disk d`): step 0; harness-core suite; harness-view [viewer] suite; harness [viewer] --test fix_export_admission and the harness test binaries that run a batch (git grep run_batch and RecipeTargets; send the list with REVIEW READY); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-folio-angle-unbounded-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
