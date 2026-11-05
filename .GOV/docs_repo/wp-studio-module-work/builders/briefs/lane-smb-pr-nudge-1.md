---
file_id: LANE-SMB-PR-NUDGE-1
file_kind: lane_brief
updated_at: 2026-11-05T15:58:35Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-NUDGE-1
lane_file: builders/lanes/hsk-studio-pr-nudge-1.json
crates: [hsk-studio-pulse (track nudge planner), hsk-studio-harness-view (timeline command)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Every nudge the spec names exists and shares one multi-frame step (STU-VID-035a)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-VID-035a (:23863): a selected clip nudges left or right by one frame or by the configured multi-frame step, and up or down by one track; slip and slide have their own one-and-many pairs; the multi-frame step is a single preference shared by every "many" variant. Read MT-33738.

At base, much exists in the Viewer: `harness-view/src/view/timeline.rs` commands `TrimNudge`, `SlipNudge`, `SlideNudge`, `ClipNudge` (:818-821), the one step (:947), `src/view/timeline/trim.rs`, `tests/viewer_timeline_trim.rs`. Pulse has no nudge planner, and `plan_move_clips` (`pulse/src/merge_sync.rs:409`) cannot change track. Read the notes of `builders/lanes/hsk-studio-pr-edit-footprint-1.json` on `plan_move` against `plan_move_clips`, and those of `hsk-studio-pr-merge-sync-1.json` on link partners.

1. **Audit first, by run.** A table of every action the clause names: clip left and right by one and by step; clip up and down by one track; slip one and many; slide one and many; the shared step. For each, what reaches it (Viewer command, pulse planner) and what it does. Report what is missing before you build. Do not rebuild what exists.
2. **Up and down by one track,** if the audit confirms it is missing. A pulse planner keeping each clip's time, moving it to the next track of the same kind. Typed refusals: no track that way, the track is locked, the range is occupied. Vendor behaviour is UNVERIFIED (D5): state your reading for an occupied range and for link partners, following merge-sync's. Every clip field survives, retime and markers included, through the exhaustive carry the footprint lane added. The planner joins the `EditProposal` table test in `pulse/tests/edit_footprint.rs`, so its footprint is proven like the others. Red first.
3. **Viewer.** One command in the existing timeline command set, through the proposal path, with undo. No GUI line and no new key binding unless the spec names one; if a binding is needed, report it.
4. **The step.** Read where the one multi-frame step lives, whether it persists, and whether any "many" variant has its own constant (grep). A second constant is a defect: fix it. Whether the preference must persist across sessions: quote the spec, or declare a reading with a `notes.spec_proposals` entry. Do not add a settings store on your own; report the need.
5. **MTs.** Read MT-33738 first; claim only rows this proves. Do not edit MT files; its proof command names a test target that does not exist, report that in the lane notes.

Hostile input (V4-01/02): no new unbounded allocation; selections bounded by existing limits. Vendor parity UNVERIFIED (D5).

Out of scope: the Viewer's drag `plan_move` against `plan_move_clips` (D5, recorded); controls registry or RUN_COMMANDS rows; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: pulse returns typed proposals and refusal codes, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: pulse plans and commits nothing; the Viewer's existing commit path carries its context", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-nudge-1`, branch `lane/smb-pr-nudge-1`, base 41d0e0da)

- pulse: `src/merge_sync.rs` or a new nudge module, `lib.rs` DESCRIPTOR, `tests/`.
- harness-view: `src/view/timeline.rs`, `src/view/timeline/trim.rs`, `tests/`.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-pr-nudge-1`): step 0; clippy over pulse, harness-view and their direct dependents; nextest WHOLE suites of pulse and harness-view (with `desktop,audio-batch`); doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-nudge-1 --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
