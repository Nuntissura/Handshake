---
file_id: LANE-SMB-PR-MERGE-SYNC-1
file_kind: lane_brief
updated_at: 2026-11-05T04:36:56Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-MERGE-SYNC-1
lane_file: builders/lanes/hsk-studio-pr-merge-sync-1.json
crates: [hsk-studio-pulse (merge clips, synchronize, link operations)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Link, group and sync-lock relationships; merge clips and synchronize (STU-VID-029, STU-VID-029a)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-VID-029 and STU-VID-029a (:23758-23771). Read MT-33726 and MT-33727. At base, `StudioClip.link_group` and `StudioTrack.sync_locked` exist; no merge or synchronize operation exists (`git grep -E "merge_clips|synchron"` in pulse: none).

1. **Audit STU-VID-029 first.** List each of its statements (link: selecting one selects all, trimming one trims all, unlink dissolves, linked-selection master toggle suspends without dissolving; group: arbitrary set moved together, no sync implication; sync lock: per-track participation in the shift) as present with file:line and test, or missing. Add only what is missing and belongs in pulse or folio without a GUI. Selection itself is a host concern: state it and stop there.
2. **Merge clips.** A planner that combines separately-recorded video and audio clips into one linked unit: an ordinary `link_group`, no new primitive. One proposal, one `TimelineInverse`, like every other pulse edit. Refuse typed: a clip already in a group, clips on the same track, no video and audio pair, a locked track, an overlap the alignment would cause.
3. **Synchronize.** A planner that aligns selected clips by in point, out point, timecode, or marker, and produces ordinary linked clips. Reuse pulse's existing time and timecode code; never a second copy of the arithmetic. Alignment by audio waveform: search first (`git grep -iE "correlat|waveform"`). If a correlator already exists in a crate pulse may depend on, use it; if none, the waveform mode is a typed `sync_waveform_unavailable` and the MT row stays partly met. Do not write a new DSP correlator in this lane; report what it needs.
4. **MTs.** Claim only what is met: MT-33726, MT-33727.

Hostile input (V4-01/02): the selection size is budgeted with a typed refusal; missing timecode or marker is a typed refusal per clip, never a silent zero. Vendor parity UNVERIFIED (D5).

Out of scope: GUI; selection state; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: pulse plans return typed proposals and refusal codes, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: pulse plans and commits nothing; the commit goes through session submit_timeline with the caller's context", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-merge-sync-1`, branch `lane/smb-pr-merge-sync-1`, base ce644577)

- pulse: a new merge-and-sync module, `lib.rs` exports and DESCRIPTOR, tests.
- folio: `src/timeline.rs` only if the STU-VID-029 audit proves a validation rule missing.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of the changed crates and direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-pr-merge-sync-1`; C: is reserved for STUDIO-R55): step 0; clippy over pulse and its direct dependents; nextest WHOLE suites of pulse and, if folio changed, folio and package (4b); doctests if added.
- Push only after STUDIO-R55 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- SMB-PR-RETIME-HOLD-1 pushes before you and adds one field to `StudioClip`: after your rebase, any `StudioClip` literal in your tests needs that field. The lead tells you when to push.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
