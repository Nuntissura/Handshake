---
file_id: LANE-SMB-PR-RETIME-OUTPUT-1
file_kind: lane_brief
updated_at: 2026-11-05T15:58:35Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-RETIME-OUTPUT-1
lane_file: builders/lanes/hsk-studio-pr-retime-output-1.json
crates: [hsk-studio-pulse (frame layers and timeline mix), hsk-studio-harness-view (monitor frame output)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="The interpolation choice and Maintain audio pitch change what plays, or answer typed">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-VID-031 (:23788). Read MT-33730 and MT-33731 and STUDIO-R56 F-R56-2 (MEDIUM) in `S/validation/round-R56.json`, and the notes of `builders/lanes/hsk-studio-pr-retime-hold-1.json` (`gaps_outside_lane`).

At base: `pulse/src/retime.rs` answers an exact `SourceFrameAnswer` (`Sampled`, `Blended`, `Estimated`, `Held`) with `estimation_mark`. Nothing outside it reads `frame_interpolation`. `playback.rs` `frame_layers` calls `clip_source_at` (`transition.rs:274`), which floors, and `FrameLayer` carries only `source_tick`, `weight` and `held`. So `frame_blending` and `optical_flow` show a floor-rounded frame with no mark and no refusal. `FrameLayer` readers: `harness-view/src/view/media.rs`, `view/monitor.rs`, `pulse/src/frame_cache.rs`, `pulse/src/nest.rs`. Pigment's `frame_mix` and `pixel_motion` exist and `composite/src/composition/frame_content.rs` calls them; harness-view already depends on pigment and composite, so no new edge is needed. `maintain_audio_pitch` is stored in folio and `build_timeline_mix` does not read it: `TrackSpec::from_source_at_speed` uses Score's resampler, so pitch moves with speed. Score exports a phase vocoder `stretch`, and pulse already depends on score.

1. **Table, red first.** Sampling, blending, flow, hold and reverse at mid-frame positions, through `frame_layers` and through the monitor: what each shows today. Prove red by running.
2. **Layers carry the answer.** `FrameLayer` carries the `SourceFrameAnswer`, including the estimation mark. The spec says `frame_sampling` is the nearest source frame and `clip_source_at` floors; read whether these disagree at a non-unit speed. If they do, the spec wins: list every changed assertion in `notes.changed_assertions` with the reason. Unit-speed output must not change. Keep `frame_cache` prefetch and its budget correct when a layer needs two frames.
3. **The monitor renders it.** `Blended` through pigment `frame_mix` at the answer's weight; `Estimated` through `pixel_motion`, carrying the mark into any diagnostic that compares it to a source frame. Use the CPU path that exists; no GPU work. Reuse what `frame_content.rs` does rather than copying it. If a path cannot render the choice, it answers a typed unavailable outcome, never a silently floored frame. State what the existing drop policy does when `pixel_motion` misses the frame budget.
4. **Maintain audio pitch.** With the flag on and a non-unit speed, the mix keeps pitch through Score's `stretch`; with it off, today's resampler output stays byte-identical. Read whether `stretch` can serve the block-based mix (`tests/stretch_stream.rs`). If it cannot without a Score change, stop and send me the case and the options; do not refuse the whole mix on your own. State what reverse with the flag on does. Proof by an independent pitch and duration measurement, not by re-running the same code.
5. **False text.** `view/timeline/rate.rs:89-90` says "unavailable: no clip field" and `view/timeline_ui.rs:463` says "No clip field stores this". `StudioClip.retime` holds them now. Correct both strings to the true state; `tests/viewer_timeline_rate.rs:336` pins only the "unavailable" prefix. That edit to `timeline_ui.rs` is the string only.
6. **MTs.** Read MT-33730 and MT-33731 first; claim only rows this proves. Do not edit MT files; report proof-command problems in the lane notes.

Hostile input (V4-01/02): no new unbounded allocation; two-frame decode stays inside the existing frame cache budget; a huge or degenerate speed answers the existing typed refusal. Vendor parity UNVERIFIED (D5). Watch V4-03: harness-view holds two audio wall-clock tests with fixed 400 ms deadlines, and a new load can move them; report it if they move, do not widen them.

Out of scope: clip time remapping (STU-MOT-045); exporters, including F-R56-10's EDL; wiring the Speed/Duration form; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: pulse returns typed answers and the estimation mark, emission stays with the host; the monitor keeps its existing report", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: pulse plans and commits nothing; the monitor reads", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-retime-output-1`, branch `lane/smb-pr-retime-output-1`, base 41d0e0da)

- pulse: `src/playback.rs`, `src/transition.rs`, `src/retime.rs`, `src/frame_cache.rs`, `src/nest.rs`, `src/audio.rs`, `lib.rs` DESCRIPTOR, `tests/`.
- harness-view: `src/view/media.rs`, `src/view/monitor.rs`, `src/view/timeline/rate.rs`, `src/view/timeline_ui.rs` (the one string), `tests/`.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-pr-retime-output-1`): step 0; clippy over pulse, harness-view and their direct dependents; nextest WHOLE suites of pulse and harness-view (with `desktop,audio-batch`) and score only if Score is read differently; doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-retime-output-1 --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
