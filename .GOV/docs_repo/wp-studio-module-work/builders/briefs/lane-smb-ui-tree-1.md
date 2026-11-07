---
file_id: LANE-SMB-UI-TREE-1
file_kind: lane_brief
updated_at: 2026-11-07T09:57:08Z
wp: WP-KERNEL-STUDIO
lane: SMB-UI-TREE-1
lane_file: builders/lanes/hsk-studio-ui-tree-1.json
crates: kumihimo-ui
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The GUI toolkit core: node tree, stack layout, display list, CPU painter, hit test, semantic tree and text port">

## Target

Read first: `direction.blocker_decisions_20261107` in S/ledger.json (binding), S/research/blockers-20261107-red-team.json, S/research/blockers-20261107-vulkan-engineer.json, S/research/blockers-20261107-shell-architect.json, S/gui-design/build-order.json. Where the red team and a position differ, the decisions record wins. Design authority for look and components: S/gui-design/design-system.json, components.json, acceptance-checks.json, build-order.json (steps B0 to B5), mock-raster.html.

Build `kumihimo-ui`, with no `hsk-studio-*` crate anywhere in its closure (D8, O5), so it can publish as a Kumihimo module later:
1. Generated design tokens from `design-system.json` (B0). Edge-based pixel snapping (B1).
2. Retained node arena and builder: one tree rebuilt per panel, no render when idle. Stack layout with spec tokens behind a layout port. No Taffy.
3. Display list as one enum, from the designer's primitive list. A CPU painter draws it into RGBA, so builders and agents see pixels with no window and no GPU. The Vulkan pass comes later and draws the same list.
4. Hit test, focus order, and a semantic tree with stable element ids: this is what `gui.tree` will return to a model.
5. Text behind a port: shaping stays in `hsk-studio-type`, reached through a trait this crate defines, with a Studio-side adapter. Include a glyph cache. Text sharpness is the designer's top quality lever: small unhinted greyscale text at 96 dpi is the top risk, so produce a real text sample PNG early and send the lead its path for the designer to judge. Read `handshake_text_mechanics` and `handshake_context_menu` in the Kumihimo material the red team names.
6. Red tests: layout at the designer's sizes, no overlap and no clipped label by the acceptance checks, golden pixel hashes for a toolbar, a tab row, a slider and a layer row.

No window code. No Vulkan. About 5 commits.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "none in the core: a port, the host supplies observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "not_applicable", "jobs": "none", "settings": "none", "refactor_exposure": "high"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-type-sentinels-registry-1`, branch `lane/smb-ui-tree-1`, base 95ee277458b18fc333ba3fb8bc6231291ec10172)

crates/kumihimo-ui and its tests, plus a thin text adapter beside hsk-studio-type only if needed. Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-type-sentinels-registry-1`): step 0; kumihimo-ui; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-type-sentinels-registry-1 --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
