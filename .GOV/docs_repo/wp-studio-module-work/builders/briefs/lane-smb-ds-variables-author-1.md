---
file_id: LANE-SMB-DS-VARIABLES-AUTHOR-1
file_kind: lane_brief
updated_at: 2026-11-13T05:13:41Z
wp: WP-KERNEL-STUDIO
lane: SMB-DS-VARIABLES-AUTHOR-1
lane_file: builders/lanes/hsk-studio-ds-variables-author-1.json
crates: harness-view, harness-core, components
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree, ONE HOLD PER PHASE, CARGO DISCIPLINE (Operator 2026-11-13))
---

<topic id="target" status="active" summary="Design variables as typed commands, plus history-aware component text compaction and .fig design-state prune at save">

## Target

Design variables as typed commands (collections, modes, variables, aliases, scopes, code syntax, explicit modes; resolve and validate as reads) plus history-aware component text compaction and .fig design-state prune at save

Why this lane: the spec-coverage audit of 2026-11-11 (`S/research/spec-coverage-20261111-rollup.json` and the group file named below) judged every requirement against origin 87f331f9. Rows named here as MISSING or PARTIAL are your acceptance: a row moves when a model can run it through a typed command (JSONL loop of `harness/src/cli/drive.rs`) and the result reaches save and export.

Facts are READ by the wave planner at origin b57c82d1 with git grep, read-only, not run, unless marked 'planner READ at 0a733db8' (research/width-plan-20261108-*.json, older pin: re-check). Treat them as claims and check them.

Group file: research/spec-coverage-20261111-g8-designsystems-automation-assets.json. Rows: STU-DS-123..133 (MISSING 131, 133; PARTIAL 123, 124, 126-128, 130, 132), STU-DS-024..034 styles versus variables (PARTIAL). Origin: research/width-plan-20261108-raster-vector-design.json rank 3, depends on SMB-DS-COMPONENTS-AUTHOR-1 (met: b57c82d1 is origin). Ledger candidates 2026-11-12T17:58:12Z: history-aware text-record compaction for component syncs; prune superseded .fig instance design states at save time, keeping reopen byte-identical.

Facts:
- planner READ at 0a733db8: StudioVariableCollection `folio/src/components.rs:246`, StudioVariable :265, explicit_variable_modes `folio/src/records.rs:83`; nothing writes them (git grep variable_collection|create_variable|set_code_syntax over the harness crates: 0); the vector entry refuses them (`harness-view/src/view/vector/style.rs:297-300`, changed since by SMB-DS-COMPONENTS-AUTHOR-1: re-read); `components::resolve_variable` (`components/src/lib.rs:36`) has no host caller. Command names from STU-DS-133 (spec :20068).
- builders/lanes/hsk-studio-ds-components-author-1.json (this tree's previous lane): not_done lists 'in a session, about 127 syncs of a component with one text layer and two instances reach text_layer_over_cap; a save and reopen compacts (R1)' and '.fig instance states with superseded contents are not pruned on open'; lead_rulings_20261112 R1 (no text compaction an undo can reach) and R2 (design file states kept as saved because viewer_fig_edit_persist of lane SMB-FIG-EDIT-2 pins a byte-identical entry on reopen; cap MAX_DESIGN_STATES 1024). Compaction must never drop a record a retained undo step can reach; prune at save must keep save-then-reopen byte-identical.
Bindings of variables to paints and node fields are out (no Folio home); tokens export follows them.

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

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-distort-effects-1`, branch `lane/smb-ds-variables-author-1`, base b57c82d1)

hsk-studio-harness-view/src/view/vector/style.rs (the member check near :291-300); hsk-studio-harness-view/src/view/design/component.rs and instance.rs (sync compaction); hsk-studio-harness-view/src/view/design/variables.rs (new; placeholder name); hsk-studio-harness-view/src/view/design/persist.rs (prune at save; design_opened is at :315); hsk-studio-harness-core/src/action/commands/design.rs; hsk-studio-harness/tests (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-ai-distort-effects-1 --disk d`): step 0; harness-core and harness-view [viewer] suites; harness [viewer] design, components and fig edit persist targets; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-ai-distort-effects-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
