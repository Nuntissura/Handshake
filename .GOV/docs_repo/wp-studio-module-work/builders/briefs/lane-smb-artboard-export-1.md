---
file_id: LANE-SMB-ARTBOARD-EXPORT-1
file_kind: lane_brief
updated_at: 2026-11-07T17:22:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-ARTBOARD-EXPORT-1
lane_file: builders/lanes/hsk-studio-artboard-export-1.json
crates: harness-view, folio, interop-vector-svg, interop-vector-core, interop-raster
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Replace the first-artboard assumption at its nine sites with a named board target, per-board render and export, and object-level export settings with SCALE, WIDTH and HEIGHT constraints writing many n">

## Target

Replace the first-artboard assumption at its nine sites with a named board target, per-board render and export, and object-level export settings with SCALE, WIDTH and HEIGHT constraints writing many named files in one run.

Why this lane: the eight professional project scenarios in `S/workflows/scenario-*.json` show engines that exist and cannot be reached. This row of `S/workflows/backlog-20261107.json` ranks it: steps unblocked 12, scenarios {"board-slides-web": 1, "product-design": 3, "vector-brand": 8}. Read that row and the scenario steps it unblocks: they are your acceptance. A step is done when a model can run it through a command, in the JSONL loop of `harness/src/cli/drive.rs` once SMB-HARNESS-DRIVE-1 lands, and the result reaches save and export.

Facts verified at c4d06841 by the backlog agent, read-only, not run. Treat them as claims and check them:
- artboards.first() at the nine sites listed in paths, by git grep
- hsk-studio-harness-view/src/view/export.rs:27 ExportFormat: Png, Jpeg; view/mod.rs:672 Viewer::export takes one path
- hsk-studio-harness-view/src/view/vector/interop.rs:605 export_vector_svg, view/vector/pdf_export.rs:364 export_vector_pdf take one path
- git grep export_settings over folio: 0 hits; harness-view hits are develop's TiffExportSettings and DngExportSettings

Depends on: ["SMB-DOC-NEW-1"]. If a dependency is not on origin yet, build what does not need it and tell the lead what waits.

How to work:
1. Read the code first. Use the spec's names and cite clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`). Use only names already in the spec, MTs or code.
2. Red test and focused test per item. Engine, command and test: no draw code and no panel. The GUI is being rebuilt in pure Vulkan, and a panel lane will call your command.
3. Commands register in the existing catalog (`RUN_COMMANDS` in `harness-core/src/action.rs`, or a `viewer.<method>` row) with typed parameters and typed refusals. Each mutating command is one named undo step, carries the caller's attribution, and survives save and reopen.
4. V4-06: a short list of sibling sites, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. About 5 commits. Width over polish. State what you cut.
MTs: read those your slice proves and name them in your report; the lead records them. Do not edit MT files.

Out of scope: panels, drawing, RF-01, render-gpu, any Cargo.lock change without asking the lead.

LEAD NOTE: the code facts in this brief were verified at an earlier origin (c4d06841 or the sha named). About 15 lanes landed since, including SMB-COMMAND-SURFACE-1 (typed rows), SMB-DOC-NEW-1 (document.new, artboards, pages, HistoryStep.containers), SMB-LAYOUT-PDF-HOST-1, SMB-DOC-EXPORT-1, SMB-PROJECT-BIN-1 and SMB-WEB-HOST-1. Re-verify every file:line at your base before relying on it, and say in your report which facts had moved. C: is over its cap: build only on D:. Lanes land one at a time: hold your push for the lead's release.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-distort-effects-1`, branch `lane/smb-artboard-export-1`, base aa68164c)

hsk-studio-harness-view/src/view/render.rs:268; hsk-studio-harness-view/src/view/edit.rs:463; hsk-studio-harness-view/src/view/select_liquify.rs:151; hsk-studio-harness-view/src/view/vector/arrange.rs:159; hsk-studio-harness-view/src/view/vector/interop.rs:587; hsk-studio-harness-view/src/view/vector/pdf_export.rs:339; hsk-studio-harness-view/src/view/vector/separations.rs:305; hsk-studio-interop-vector-core/src/eps_export.rs:386; hsk-studio-interop-vector-svg/src/svg_export.rs:1167; hsk-studio-harness-view/src/view/export_set.rs (new); hsk-studio-folio/src/export_settings.rs (new); hsk-studio-package/tests/externals.rs (rows)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-ai-distort-effects-1`): step 0; the suites of the crates you edit, with named targets only for harness and harness-view unless you edit their src; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-ai-distort-effects-1 --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
