---
file_id: LANE-SMB-GUI-STEWARD-1
file_kind: lane_brief
updated_at: 2026-11-07T08:03:35Z
wp: WP-KERNEL-STUDIO
lane: SMB-GUI-STEWARD-1
lane_file: builders/lanes/hsk-studio-gui-steward-1.json
crates: root, hsk-studio-vulkan, hsk-studio-native-host, kumihimo-ui
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="One landing that makes the GUI lanes possible: skeleton crates, locked pins, gate allow list, GPU lock and test group, round config">

## Target

You are the workspace steward for this landing. Read first: `direction.blocker_decisions_20261107` in S/ledger.json (binding), S/research/blockers-20261107-red-team.json, S/research/blockers-20261107-vulkan-engineer.json, S/research/blockers-20261107-shell-architect.json, S/gui-design/build-order.json. Where the red team and a position differ, the decisions record wins. Also read S/builders/WORKSPACE_RULES.md, S/builders/lanes/workspace-steward.json and S/tools/steward-sync.sh.

One landing, as small as it can be:
1. **Skeleton crates**, each a real empty crate with a DESCRIPTOR, in the workspace: `hsk-studio-vulkan` (all Vulkan unsafe, ash =0.38.0; default build carries no wgpu, naga, winit, eframe or egui), `hsk-studio-native-host` (winit =0.30.13 plus windows-sys; the only crate allowed winit), `kumihimo-ui` (toolkit: no `hsk-studio-*` anywhere in its closure from birth).
2. **Pins at versions already in Cargo.lock.** The lock's dependency set should not change. If it would, stop and report the exact delta: no download happens without the lead's word. No Taffy. No accesskit patch.
3. **Quarantine gate:** `tools/workspace-probe.rs` allow list gains winit for `hsk-studio-native-host` only, with the decision as its reason. Add the rule that `kumihimo-*` has no `hsk-studio-*` in its closure. Ash is not on the banned list.
4. **GPU contention:** extend the GPU lock and the nextest test group to the new Vulkan crates, in `tools/run-round.sh` crate_gpu and `.config/nextest.toml`, because foreign GPU clients broke R39. A binary run of the host takes the GPU lock. The kernel's `builders/studio-env.sh` is the lead's: send the lead the exact lines it needs.
5. **Round config:** add `hsk-studio-controls = [["egui"]]` to `.config/round-features.toml` [crates], from SMB-CONTROLS-EGUI-GATE-1, which is on origin by the time you start. Add the run-round guard that refuses a `CARGO_TARGET_*_LINKER` from the environment. The 5-unit matrix cut is NOT in this landing unless the lead says so.

These root inputs force R60 to be full, which is already the plan. State exactly which root files changed. Run the probe's self-test, the registry and licence checks of steward-sync, and `cargo metadata --locked`. No new product behaviour.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none: workspace configuration", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not_applicable", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-retime-hold-1`, branch `lane/smb-gui-steward-1`, base 9e73d34cd5d24180fd7a56ff4453a066abb9a5ee)

root Cargo.toml, Cargo.lock if unchanged in dependency set, crates/hsk-studio-vulkan, crates/hsk-studio-native-host, crates/kumihimo-ui, tools/workspace-probe.rs, tools/run-round.sh, .config/nextest.toml, .config/round-features.toml

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-retime-hold-1 --disk d`): step 0; workspace-probe self-test and the three new crates; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-retime-hold-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
