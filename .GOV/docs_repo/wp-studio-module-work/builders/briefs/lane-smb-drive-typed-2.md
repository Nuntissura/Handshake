---
file_id: LANE-SMB-DRIVE-TYPED-2
file_kind: lane_brief
updated_at: 2026-11-07T17:22:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-DRIVE-TYPED-2
lane_file: builders/lanes/hsk-studio-drive-typed-2.json
crates: harness, harness-core
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Third drive slice: rows for MixerEdit, WaveformEdit, AutomationCommand, SpectralMaskOp, DesignCommand, source edit, bind_program_media and prototype play, so audio and product design can run as a scri">

## Target

Third drive slice: rows for MixerEdit, WaveformEdit, AutomationCommand, SpectralMaskOp, DesignCommand, source edit, bind_program_media and prototype play, so audio and product design can run as a script.

Why this lane: the eight professional project scenarios in `S/workflows/scenario-*.json` show engines that exist and cannot be reached. This row of `S/workflows/backlog-20261107.json` ranks it: steps unblocked 63, scenarios {"audio-post": 35, "motion-comp": 3, "product-design": 14, "video-edit": 11}. Read that row and the scenario steps it unblocks: they are your acceptance. A step is done when a model can run it through a command, in the JSONL loop of `harness/src/cli/drive.rs` once SMB-HARNESS-DRIVE-1 lands, and the result reaches save and export.

Facts verified at c4d06841 by the backlog agent, read-only, not run. Treat them as claims and check them:
- hsk-studio-harness-view/src/view/mixer.rs:145 enum MixerEdit, :297 mixer_edit
- hsk-studio-harness-view/src/view/waveform.rs:100 enum WaveformEdit, :743 waveform_edit
- hsk-studio-harness-view/src/view/timeline/audio.rs:399 open_multitrack generic over AudioSources, :422 multitrack_command
- hsk-studio-harness-view/src/view/design/instance.rs:76 enum DesignCommand, :400 design_command
- hsk-studio-harness-view/src/view/spectral_display.rs:194 apply_spectral
- hsk-studio-harness-view/src/view/source_monitor.rs:561 source_edit, :749 open_source_media
- hsk-studio-harness-view/src/view/timeline/export_media.rs:464 bind_program_media takes file bytes

Depends on: ["SMB-HARNESS-DRIVE-1 merged", "SMB-PR-ROUTING-REPAIR-1 merged (it edits mixer and id mints)"]. If a dependency is not on origin yet, build what does not need it and tell the lead what waits.

How to work:
1. Read the code first. Use the spec's names and cite clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`). Use only names already in the spec, MTs or code.
2. Red test and focused test per item. Engine, command and test: no draw code and no panel. The GUI is being rebuilt in pure Vulkan, and a panel lane will call your command.
3. Commands register in the existing catalog (`RUN_COMMANDS` in `harness-core/src/action.rs`, or a `viewer.<method>` row) with typed parameters and typed refusals. Each mutating command is one named undo step, carries the caller's attribution, and survives save and reopen.
4. V4-06: a short list of sibling sites, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. About 5 commits. Width over polish. State what you cut.
MTs: read those your slice proves and name them in your report; the lead records them. Do not edit MT files.

Out of scope: panels, drawing, RF-01, render-gpu, any Cargo.lock change without asking the lead.

LEAD ADDITIONS (named by landed lanes, cheap while you are in these files):
1. SMB-DOC-NEW-1's catalog rows (document.new, artboard.*, page.*) mark their optional parameters as required, because SMB-COMMAND-SURFACE-1 added the `required` field after they were written: add .opt() where the method takes an Option.
2. SMB-DOC-NEW-1's viewer rows fall into viewer_domain's default and read vector.*: give them a document arm in harness-core manual.rs viewer_domain (catalog ids document.new, artboard.*, page.* already route correctly; keep them).
3. clippy large_const_arrays on ALL_ROWS at harness-core action/commands.rs:184 (SMB-WEB-HOST-1 noted it): make ALL_ROWS a static or split it per domain, whichever the corpus test allows.
Follow 98e4099c's typed pattern (typed(), .opt(), per-domain rows files, viewer_domain arms) for every new row.

LEAD NOTE: the code facts in this brief were verified at an earlier origin (c4d06841 or the sha named). About 15 lanes landed since, including SMB-COMMAND-SURFACE-1 (typed rows), SMB-DOC-NEW-1 (document.new, artboards, pages, HistoryStep.containers), SMB-LAYOUT-PDF-HOST-1, SMB-DOC-EXPORT-1, SMB-PROJECT-BIN-1 and SMB-WEB-HOST-1. Re-verify every file:line at your base before relying on it, and say in your report which facts had moved. C: is over its cap: build only on D:. Lanes land one at a time: hold your push for the lead's release.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-drive-typed-2`, base aa68164c)

hsk-studio-harness/src/cli/drive_audio.rs (new); hsk-studio-harness/src/cli/drive_design.rs (new); hsk-studio-harness/src/cli/drive_viewer.rs (dispatch arm only); hsk-studio-harness-core/src/manual.rs (rows); hsk-studio-harness/tests/harness_drive_audio.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-pr-edit-footprint-1`): step 0; the suites of the crates you edit, with named targets only for harness and harness-view unless you edit their src; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-edit-footprint-1 --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
