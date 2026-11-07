---
file_id: LANE-SMB-CONTROLS-EGUI-GATE-1
file_kind: lane_brief
updated_at: 2026-11-07T05:20:57Z
wp: WP-KERNEL-STUDIO
lane: SMB-CONTROLS-EGUI-GATE-1
lane_file: builders/lanes/hsk-studio-controls-egui-gate-1.json
crates: controls, harness-core
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="controls builds without egui, and a mechanical gate stops new egui, eframe, winit, wgpu, naga and WGSL">

## Target

Studio-side pivot prep, work order item 3 in `S/HANDOFF.md`; read its `vulkan-gui-pivot` topic, including "What Studio can do now", and `S/research/truth-check-20261105-vulkan-pivot.json`. D1 and D7 hold today by reading only. This lane moves toolkit code behind a gate and adds none. RF-01 and the Viewer's public egui signatures are NOT in this lane: RF-01 waits for a lead ruling.

1. **Red first.** `controls` built with no egui feature fails today: `controls/Cargo.toml:13` has a non-optional `egui`, and `controls/src/lib.rs:14` declares `pub mod egui_adapter` with no gate. Show it.
2. **Make `egui` optional in `controls`,** gating `egui_adapter` and `menu_bar_ui`. Consumers (`harness-core/Cargo.toml:26`, and any other: list them) enable the feature, so their behaviour does not change yet.
3. **No proof may silently leave a round.** A default-feature union unit that loses `egui_adapter`'s tests is a proof loss. Count controls' tests before and after in each feature set. Decide with the lead how the union keeps running them: either the feature stays in the crate's default and headless consumers opt out, or `.config/round-features.toml` gains a unit. That file is workspace-steward owned: read `S/builders/WORKSPACE_RULES.md`, then send the lead your option. Do not edit it. If any manifest edit needs the steward under those rules, or `Cargo.lock` would change, stop and message the lead with the exact diff.
4. **Mechanical gate in `tools/workspace-probe.rs`.** It fails when any crate's manifest gains egui, eframe, winit, wgpu or naga, or when a `.wgsl` file is added, beyond an allow list of today's exact set, recorded at the base commit with the reason (D1, D7). Prove it red with a fixture. It runs where the probe already runs: state where that is. A gate line that names these crates is not a new use of them.
5. **Prove the headless build:** `cargo tree -p hsk-studio-controls --no-default-features -e normal` shows no egui, and its non-drawing tests pass.
6. **MTs.** Claim a row only if you read it and this proves it. Do not edit MT files.

Out of scope, recorded as candidates in your report: moving gesture semantics out of `egui_adapter.rs`, converting tests that assert no drawing to the headless path, RF-01, RF-05, RF-12, `render-gpu`, any panel or Viewer surface, any Kumihimo work, any change to WP-KERNEL-012.

PUSH HOLD: STUDIO-R59 is judging c4d06841 on C:. Build on D: only. Push nothing until the lead tells you R59 is recorded; when you are ready to push, message the lead with the local tip and your test counts. One lane pushes at a time.

Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005). The folder name and `--owner` are the old lane's; your branch, log names and records are this lane's.

No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D6, D7, D8), unless this brief names it. No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none: build configuration and a gate", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not_applicable: no command changes", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-retime-hold-1`, branch `lane/smb-controls-egui-gate-1`, base c4d06841)

`crates/hsk-studio-controls/Cargo.toml` and `src/lib.rs` gates, manifests of crates that consume controls (feature line only), `tools/workspace-probe.rs` and its fixture. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-retime-hold-1 --disk d`): step 0; controls in each feature set, harness-core, and the workspace probe; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-retime-hold-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
