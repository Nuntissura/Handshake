---
file_id: LANE-SMB-COMMAND-SURFACE-1
file_kind: lane_brief
updated_at: 2026-11-07T08:10:03Z
wp: WP-KERNEL-STUDIO
lane: SMB-COMMAND-SURFACE-1
lane_file: builders/lanes/hsk-studio-command-surface-1.json
crates: harness-core, harness, controls, harness-view
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="One typed command catalog split by domain, reaching about 100 more project steps through the process loop">

## Target

FIRST STEP, before any code: this is the trunk check after three landings. Run `cargo check --workspace --tests` through lane-check or a sourced studio-env at your base, and message the lead the result at once. If trunk is red, say which commit and stop.

Then make Studio's commands reachable, typed and one catalog. Read `direction.blocker_decisions_20261107` in S/ledger.json, S/research/blockers-20261107-red-team.json (the foundation decision), and S/workflows/backlog-20261107.json row SMB-DRIVE-TYPED-1: about 101 steps across the eight scenarios stay Rust-only until this lands.

1. **First commit, nothing else in it:** split `RUN_COMMANDS` (`harness-core/src/action.rs:843`) into per-domain files, so many lanes can add rows without colliding. Every later width lane depends on this commit, so land it as soon as it is green and tell the lead.
2. **One registry:** a `UiDescriptor` seed with `command_id`, from existing enumerations, with alias tables for old ids (RF-05's seed). The GUI will bind panels to this: a panel calls a command id, never a private method.
3. **Typed decoders and rows** for what stays Rust-only: LayoutEdit (10 members), PanelAction (41 variants), ProofCommand and the remaining vector methods taking whole typed records, through the JSONL loop in `harness/src/cli/drive.rs`. Parameters are JSON, decoded with typed refusals. Note harness-view has no serde derive: read the backlog row on this and pick the smallest route.

Facts verified at c4d06841 by the backlog agent, read-only, not run. Treat them as claims:
- branch lane/smb-harness-drive-1 95af749d hsk-studio-harness/src/cli/drive_viewer.rs:1-5 says methods taking a whole typed spec are not here yet; its 86 rows name neither hosted_action nor any LayoutEdit call; drive.rs has 0 matches for layout
- hsk-studio-harness-view/src/layout_ui.rs:146 enum LayoutEdit
- hsk-studio-harness-view/src/view/hosted.rs:217 enum PanelAction, 41 variants; :1348 hosted_action
- hsk-studio-harness-view/src/view/proof.rs:389 proof_command
- git grep Deserialize over hosted.rs, layout_ui.rs, mixer.rs: 0 hits; hsk-studio-harness-view/Cargo.toml:64 serde_json only

Each command: named undo where it mutates, caller's attribution, typed refusal, a JSONL test through the process. Width over polish. About 5 commits after the split. State what you cut. Out of scope: draw code, panels, RF-01, render-gpu, STU-AUT-003 and STU-AUT-005.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal frame per command, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: --actor and --session reach the context of every mutating command; never a default actor", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-command-surface-1`, base b954d4c51d9256573b5a35cf234bf6de26475e7d)

harness-core/src/action.rs and new per-domain files, harness/src/cli/drive*.rs, harness/tests, controls descriptors, harness-view command decoding without draw code. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-edit-footprint-1 --disk d`): step 0; harness and harness-core, with named harness-view targets; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-edit-footprint-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
