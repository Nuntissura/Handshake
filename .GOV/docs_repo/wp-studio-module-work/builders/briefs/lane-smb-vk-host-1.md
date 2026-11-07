---
file_id: LANE-SMB-VK-HOST-1
file_kind: lane_brief
updated_at: 2026-11-07T08:49:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-VK-HOST-1
lane_file: builders/lanes/hsk-studio-vk-host-1.json
crates: hsk-studio-vulkan, hsk-studio-native-host
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The first real Vulkan window: offscreen capture with no window, a quiet window, swapchain recovery, and a real document on screen">

## Target

Read first: `direction.blocker_decisions_20261107` in S/ledger.json (binding), S/research/blockers-20261107-red-team.json, S/research/blockers-20261107-vulkan-engineer.json, S/research/blockers-20261107-shell-architect.json, S/gui-design/build-order.json. Where the red team and a position differ, the decisions record wins. Spec: `15-native-desktop-gui-vulkan-gpu-ownership.md` under K/.GOV/spec/master-spec-v02.229/spec-modules/.

Goal: the Operator can see a real document in a real pure-Vulkan window. No egui, eframe or wgpu in the default build. No shaders, pipelines or descriptors in this lane.
1. **Offscreen first, red first:** device on Vulkan 1.3, clear an image, read it back and write a PNG, with no window and no HWND. Agents screenshot through this, so it is the proof path.
2. **Quiet window:** winit in `hsk-studio-native-host` only, with a typed HostEvent stream past that crate. `window --quiet` shows focus_changes 0 and its capture hash equals the offscreen one. Read WS_EX_NOACTIVATE and quiet handling from the Kumihimo native code named in the red team's file. The Operator uses this machine, so nothing may take focus or capture input in any test (HBR-QUIET).
3. **Swapchain state machine:** explicit states for surface loss, out-of-date, minimise and resize, with injected faults in tests. PMv2 DPI. No Vulkan means a typed refusal, with no fallback.
4. **Buffer present:** present a CPU-painted RGBA buffer with vkCmdCopyBufferToImage or vkCmdBlitImage.
5. **Document on screen:** a `document` feature links harness-view[viewer] with Prefer::Cpu and BackendChoice::None. `demo.handshake` from the repo shows in the window, through `Viewer`'s public open and render. A paused, focus-free run writes a capture to the artifact root. Send the lead that PNG's path: the Operator wants to see it.

Device: transient owner behind `GpuClientPort`, leasing raw handles, one device per process, panes draw into an owner image, so a later hand-over to Handshake swaps owner and presenter. All unsafe in `hsk-studio-vulkan`. Bounded fence waits. Host disables overlay layers in its own process. Host binary runs take the GPU lock.

Validation layers are absent on this machine: every receipt says `validation: absent`. Use the stand-ins the red team lists. 3 to 5 commits.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: typed frames for device, swapchain state and refusals", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not_applicable: no document mutation", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-entry-references-1`, branch `lane/smb-vk-host-1`, base 7879e7e974121079e5ac1708a6a1835d064d7465)

crates/hsk-studio-vulkan, crates/hsk-studio-native-host and their tests. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-entry-references-1 --disk d`): step 0; the two new crates; harness-view is linked, not edited; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-entry-references-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
