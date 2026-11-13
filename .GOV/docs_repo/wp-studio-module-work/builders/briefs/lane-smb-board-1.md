---
file_id: LANE-SMB-BOARD-1
file_kind: lane_brief
updated_at: 2026-11-13T05:13:41Z
wp: WP-KERNEL-STUDIO
lane: SMB-BOARD-1
lane_file: builders/lanes/hsk-studio-board-1.json
crates: folio, composite, harness-core, harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree, ONE HOLD PER PHASE, CARGO DISCIPLINE (Operator 2026-11-13))
---

<topic id="target" status="active" summary="Whiteboard first slice: board profile, container, sticky, shape_with_text, section and connector with factory, read and mutate commands and PNG export">

## Target

Whiteboard mode first slice: document.new board profile, board container, sticky, shape_with_text, section and connector (STRAIGHT, ELBOWED; node and magnet endpoints) with factory, read and mutate commands and PNG export

Why this lane: the spec-coverage audit of 2026-11-11 (`S/research/spec-coverage-20261111-rollup.json` and the group file named below) judged every requirement against origin 87f331f9. Rows named here as MISSING or PARTIAL are your acceptance: a row moves when a model can run it through a typed command (JSONL loop of `harness/src/cli/drive.rs`) and the result reaches save and export.

Facts are READ by the wave planner at origin b57c82d1 with git grep, read-only, not run, unless marked 'planner READ at 0a733db8' (research/width-plan-20261108-*.json, older pin: re-check). Treat them as claims and check them.

Group file: research/spec-coverage-20261111-g5-web-whiteboard.json. Rows: STU-WB-001, 002, 102..117, 121, 134 (board mode and node kinds, 13 of 14 MISSING; command-surface obligation STU-WB-134 MISSING). Origin: research/width-plan-20261108-print-web.json rank 4, start gate 'after SMB-AI-LIVE-EFFECTS-1 merges' (met: 1e652818 is on origin). Scenario: workflows/scenario-board-slides-web.json steps 1, 3, 6, 8-13, 26.

Facts:
- `harness-core/src/action/commands/board.rs` exists with an empty `ROWS` table (5 lines): this lane owns it.
- No board code: grep StudioBoard|board_node|BoardNode|sticky_note|shape_with_text: 0 (audit).
- STU-WB-102 says board node kinds are StudioLayer kinds. `folio/src/lib.rs:268` is `pub enum Payload`; `Payload::Group` appears in 41 files. Before coding, list every exhaustive Payload match your design would touch and send it to the lead: if it adds Payload variants, this lane lands last in the wave.
- Open spec question (workflows/backlog-20261107.json spec_questions): a board has no signature, default layout, Layout Preset or tool family (OD-4 at STU-SHL-136). It blocks the board GUI only; this slice is engine and commands.
STU-WB-101 calls whiteboarding a lower-priority parity domain; keep the slice to the node kinds above. Rule 4b: new Folio records also run the hsk-studio-package suite.

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
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-sentinels-native-1`, branch `lane/smb-board-1`, base b57c82d1)

hsk-studio-folio/src (board node records); hsk-studio-composite/src (board drawing); hsk-studio-harness-core/src/action/commands/board.rs; hsk-studio-harness-view/src/view/board.rs (new, egui-free; placeholder name); hsk-studio-harness/tests/board.rs (new)

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-sentinels-native-1 --disk d`): step 0; folio, composite and package suites; harness-core and harness-view [viewer] suites; harness board tests; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-sentinels-native-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
