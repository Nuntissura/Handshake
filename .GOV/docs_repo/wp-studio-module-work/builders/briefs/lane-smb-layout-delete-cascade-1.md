---
file_id: LANE-SMB-LAYOUT-DELETE-CASCADE-1
file_kind: lane_brief
updated_at: 2026-11-13T05:13:41Z
wp: WP-KERNEL-STUDIO
lane: SMB-LAYOUT-DELETE-CASCADE-1
lane_file: builders/lanes/hsk-studio-layout-delete-cascade-1.json
crates: harness-core, harness-view, layout
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree, ONE HOLD PER PHASE, CARGO DISCIPLINE (Operator 2026-11-13))
---

<topic id="target" status="active" summary="page.delete with content and text-frame delete re-link threads in one undo step; frame-holding group, page and spread deletes; autoflow and smart reflow">

## Target

Layout deletes that InDesign performs: page.delete with content and a text-frame layer delete re-link the thread in one undo step; groups, pages and spreads holding frame layers; autoflow placement modes and smart text reflow

Why this lane: the spec-coverage audit of 2026-11-11 (`S/research/spec-coverage-20261111-rollup.json` and the group file named below) judged every requirement against origin 87f331f9. Rows named here as MISSING or PARTIAL are your acceptance: a row moves when a model can run it through a typed command (JSONL loop of `harness/src/cli/drive.rs`) and the result reaches save and export.

Facts are READ by the wave planner at origin b57c82d1 with git grep, read-only, not run, unless marked 'planner READ at 0a733db8' (research/width-plan-20261108-*.json, older pin: re-check). Treat them as claims and check them.

Group file: research/spec-coverage-20261111-g4-layout-io-docs.json. Rows: STU-LAY-020 autoflow placement modes (MISSING, spec :5851), STU-LAY-123 autoflow and smart text reflow (MISSING, :5855); page and spread rows STU-LAY-001, 003, 107 (PARTIAL). Ledger candidates (2026-11-12T17:43:00Z, 2026-11-11T10:39:51Z, 2026-11-11T12:51:10Z): page.delete with content; text-frame layer delete with re-linking; deleting a group, page or spread that holds a frame layer; asset.place with frame_to_content keeps the given bounds.

Facts:
- page.delete refuses `page_not_empty` for content (`harness-core/src/harness/containers.rs:54-58`, :513-522 docs, :590 graphic frame case): replace it with the cascade in one undo step.
- delete_layer refuses `layer_bound_by_layout_records` (`harness-view/src/view/layers.rs:219`), added by SMB-R60-FOLLOWUPS-1 (this tree's previous lane, e29d933a): replace it with thread re-linking; undo restores; reopen editable.
- Threads: `folio/src/layout.rs:392` text_threads; rows layout.thread / layout.unthread at `harness-core/src/action/commands/layout.rs:14-15`; the Viewer maps them at `harness-view/src/view/commands/layout.rs:67-68`; thread code in `harness-view/src/layout_ui/threads.rs`.
- SMB-TYPE-CHARACTER-ATTRIBUTES-1 runs in parallel and owns `layout_ui.rs`, `view/commands/layout.rs` and `action/commands/layout.rs`: do not edit them. New rows go in a new domain file. If the re-link needs a new LayoutEdit variant (layout_ui.rs), tell the lead and land after that lane.

How to work:
1. Read the code first. Use the spec's names and cite clauses (`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`). Use only names already in the spec, MTs or code.
2. Red test and focused test per item. Typed commands with typed parameters and typed refusals; each mutating command is one named undo step, carries the caller's attribution and survives save and reopen.
3. Commands register in the existing catalog: your domain file under `harness-core/src/action/commands/` (the per-domain `ROWS` tables that `commands.rs` concatenates into `RUN_COMMANDS`), or a `viewer.<method>` row.
4. V4-06: a short list of sibling sites, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. About 6 commits. Width over polish. State what you cut.
MTs: read those your slice proves and name them in your report; the lead records them. Do not edit MT files.

Out of scope: panels and drawing (except where this lane is the GUI lane), RF-01, render-gpu, any Cargo.lock change without asking the lead.

R60 attempt 3 takes the D: lock first. Until it releases, read the code, write red tests and commit locally; your first hold queues behind the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16).

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-layout-delete-cascade-1`, base b57c82d1)

hsk-studio-harness-core/src/harness/containers.rs; hsk-studio-harness-view/src/view/layers.rs; hsk-studio-harness-view/src/view/layout_place.rs; hsk-studio-harness-view/src/layout_ui/threads.rs; hsk-studio-layout/src/threaded_reflow.rs; hsk-studio-harness-core/src/action/commands/layout_flow.rs (new; one DOMAINS line in commands.rs)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-edit-footprint-1 --disk d`): step 0; harness-core and harness-view [viewer] suites; layout suite; harness [viewer] layout and layers targets; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-edit-footprint-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
