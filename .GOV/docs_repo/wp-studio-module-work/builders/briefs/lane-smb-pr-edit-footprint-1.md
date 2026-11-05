---
file_id: LANE-SMB-PR-EDIT-FOOTPRINT-1
file_kind: lane_brief
updated_at: 2026-11-05T10:05:08Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-EDIT-FOOTPRINT-1
lane_file: builders/lanes/hsk-studio-pr-edit-footprint-1.json
crates: [hsk-studio-pulse (edit proposal change list and write footprint)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Every real edit reports what it writes; a new clip field cannot escape the diff">

## Target

Two lanes each found the same defect by accident: a real edit whose `EditProposal::changes()` and `write_footprint()` (`crates/hsk-studio-pulse/src/timeline_edit.rs:255, :264`) were both empty. Fixed for link-only edits (`ClipChange::Relink`, SMB-PR-MERGE-SYNC-1) and retime edits (`ClipChange::RetimeRecord`, SMB-PR-RETIME-HOLD-1). Read from the code, not run: a reverse-only speed change, and marker, transition and work-area record edits. Close the class.

1. **Readers.** List every non-test reader of both methods. At base these are `examples/pulse-timeline-consumer.rs` and, for the separate nest change type, `harness-view/src/view/timeline/nest.rs`; session has no pulse edge. Grep the spec for the write-footprint and change-list requirement and quote it in `notes.spec_basis`. State what a wrong empty answer costs a host; do not assume it.
2. **Table test, red first.** One table over every planner that returns an `EditProposal` (V4-06: `git grep -nE "fn plan_|EditProposal" crates/hsk-studio-pulse/src`, every site listed fixed or not applicable with a reason). For each, on a fixture where the edit does something: if the successor differs from the base, `changes()` and `write_footprint()` are non-empty and name every changed record. Also the reverse: an edit that changes nothing reports nothing.
3. **Fix the class.** Make the clip diff destructure `StudioClip` exhaustively, with no `..`, so a new field stops compiling until someone decides how it is reported. Do the same for any other record the diff compares. Where the footprint has no way to name a record (markers, transitions, work area), add what it needs, following `Relink` and `RetimeRecord`; every addition is reported, none silently folded into another change.
4. **MTs.** This lane is a correction: claim an MT row only if you read it and this proves it. Do not edit MT files.

Hostile input (V4-01/02): no new unbounded allocation; a change list is bounded by the document's existing limits. Vendor parity is not involved.

Out of scope: nest's own `SequenceChange` unless it has the same defect (then report it); linked-selection toggle reach (recorded separately); any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change; no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: pulse plans return typed proposals and refusal codes, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: pulse plans and commits nothing", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-edit-footprint-1`, branch `lane/smb-pr-edit-footprint-1`, base 27cba13b)

- pulse: `src/timeline_edit.rs`, every planner module the table proves wrong, `lib.rs` DESCRIPTOR, `tests/`.
- pulse `examples/pulse-timeline-consumer.rs` only if a new change kind needs listing there.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-pr-edit-footprint-1 --disk d`; C: is reserved for STUDIO-R56): step 0; clippy over pulse and its direct dependents; nextest WHOLE suites of pulse and session; doctests if added.
- Push only after STUDIO-R56 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
