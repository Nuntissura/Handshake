---
file_id: LANE-SMB-PR-RETIME-HOLD-1
file_kind: lane_brief
updated_at: 2026-11-05T04:36:56Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-RETIME-HOLD-1
lane_file: builders/lanes/hsk-studio-pr-retime-hold-1.json
crates: [hsk-studio-folio (clip retime property), hsk-studio-pulse (frame hold planners, source-frame resolution)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Clip retime choices and frame hold (STU-VID-031, STU-VID-031a)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-VID-031 and STU-VID-031a (:23788-23806). Read `crates/hsk-studio-pulse/src/rate.rs` first: its header declares the gap this lane closes (`Maintain audio pitch` and the frame-interpolation choice have no field on `StudioClip`). Read MT-33730, MT-33731, MT-33732.

1. **Clip retime property.** Add ONE field to `StudioClip` (`hsk-studio-folio/src/timeline.rs:304`): a `retime` record holding the frame-interpolation method (`frame_sampling` | `frame_blending` | `optical_flow`), `maintain_audio_pitch`, and the frame hold of item 3. Serde default and omitted from the wire at its default, so old-shape clips load unchanged and write byte-identically. Validate on the folio path. A field on the clip, not a record set beside the document: razor, duplicate and nest clone clips, so the property must travel with the clip without any planner remembering it. There are 92 `StudioClip {` literals outside folio; add the one field line at each and change nothing else in those files.
2. **Source-frame resolution.** Pulse maps a timeline tick to a typed answer for the clip: one source frame (`frame_sampling`), two neighbouring source frames and a weight (`frame_blending`), or, for `optical_flow`, the same two frames marked as an estimation (the spec requires the mark in any diagnostic comparing it to a source frame). Search first (`git grep -i "optical"`): reuse an existing motion estimator if one is present; if none, optical flow is a typed `optical_flow_unavailable` at render time and the MT row stays partly met. Reverse speed must work. Exact arithmetic on ticks, no floats in the mapping.
3. **Frame hold.** An explicit property freezing the clip at a chosen source frame: at the playhead, at a clip marker, at the in point, at the out point. Planners in pulse, one proposal and one `TimelineInverse` each like every other pulse edit: set hold, clear hold, and `insert frame hold segment`, which splits out a held span as its own clip. A held clip resolves to its one source frame at every timeline tick and its audio behaviour is stated (the spec is silent: choose the reading, declare it in `declared_readings`, raise it as a spec proposal). A hold on a multicam clip or a clip of a multicam source sequence is refused typed, as rate.rs does for reverse.
4. **Rate planners.** `plan_rate_stretch` and `plan_speed_duration` keep the retime record unchanged; update rate.rs's gap note.
5. **Package round trip** of the new field (rule 4b), old-shape clips load unchanged.
6. **MTs.** Claim only what is met: MT-33730, MT-33731, MT-33732.

Hostile input (V4-01/02): typed refusals for a hold outside the source window. Vendor parity UNVERIFIED (D5).

Out of scope: time remapping (the keyframed form in STU-VID-031a, a later lane: report what it needs); pixel blending or render-graph work beyond the typed answer; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: folio may depend only on accord; pulse plans return typed proposals and refusal codes, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: pulse plans and commits nothing; the commit goes through session submit_timeline with the caller's context", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-retime-hold-1`, branch `lane/smb-pr-retime-hold-1`, base ce644577)

- folio: `src/timeline.rs`, `lib.rs` exports and DESCRIPTOR, tests.
- pulse: `src/rate.rs` (gap note), a new retime module, `lib.rs` exports and DESCRIPTOR, tests.
- package: externals test only for the new field (4b).
- Any crate: the one added field line at each `StudioClip` literal, nothing else in that file.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of the changed crates and direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-pr-retime-hold-1`; C: is reserved for STUDIO-R55): step 0; clippy over folio and its direct dependents (wide); nextest WHOLE suites of folio, pulse, package (4b), and every crate whose literals you touched; doctests if added.
- Push only after STUDIO-R55 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Push order after the release: SMB-PR-SEQUENCE-SESSION-1 first, then this lane, then SMB-PR-MERGE-SYNC-1. The lead tells you when; rebase on what is then at origin.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
