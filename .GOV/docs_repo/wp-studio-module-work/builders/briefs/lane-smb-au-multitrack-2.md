---
file_id: LANE-SMB-AU-MULTITRACK-2
file_kind: lane_brief
updated_at: 2026-10-30T10:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AU-MULTITRACK-2
lane_file: builders/lanes/hsk-studio-au-multitrack-2.json
crates: [hsk-studio-harness-view (timeline files), hsk-studio-folio (audio_routing.rs), hsk-studio-pulse (bus_mix.rs)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e incl. 4b package suite, PUSH SAFETY, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Clip envelopes survive timeline edits; track volume/pan automation over time">

## Target

Follow-up of SMB-AU-MULTITRACK-1 (READY at 1c57c17a; read its lane file `builders/lanes/hsk-studio-au-multitrack-1.json` notes `engine_gaps_for_lead`, `spec_basis`, `spec_deltas` first). Spec: `[STU-FX-137]` (routing is part of the document; Track carries volume, pan, keyframe_mode), `[STU-FX-135]`, `[STU-VID-040]` (one reversible history entry per operation). No AU-14 clause: Audition behaviour UNVERIFIED (D5), declared readings in the notes.

1. DATA INTEGRITY FIRST: timeline edits keep clip volume envelopes (`StudioAudioRouting.clip_gains`) consistent in the SAME undo step as the timeline edit: trim-in shifts the envelope offsets by the trimmed amount and drops points before the new in-point (declared: insert an interpolated point at the new edge), trim-out drops points past the out-point; razor splits the envelope (both parts get their points plus an interpolated edge point); lift / ripple delete removes the clip's envelope; slip shifts offsets; roll/slide follow from trim. A routing that `validate_audio_routing` would refuse must never be committed. Tests: one per edit kind, plus `lift_never_leaves_a_dangling_envelope` (save, reopen, validate). Find how the timeline commit writes its entries (`src/view/timeline*.rs`, pulse `timeline_edit.rs`); if two session entries cannot be written as one undo step today, use or extend the existing one-step API (harness-core `Run::restructure_and_entry` precedent) minimally and name it in the notes.
2. Track automation record: folio `StudioAudioRouting.track_automation: Vec<StudioTrackAutomation>` (track id, param Volume | Pan, points of offset_ticks + value; bounded like clip_gains; unknown track refused typed; absent = byte-identical documents) and pulse `bus_mix.rs` builds score `AutomationLane::new(points, Linear, range)` instead of `AutomationLane::constant(...)` when points exist (constant otherwise, byte-identical mixdown). Tests: record validate/refuse/roundtrip, `package every_folio_reference_property_is_classified` (rule 4b), mixdown follows a volume ramp and a pan ramp.
3. Window: the Volume and Pan lanes of MULTITRACK-1 become editable through the same typed commands (add / move / delete point, one undo step), mixer rebuilds after edits.
4. (time left) clip pan envelope record + lane, and a master row with master effect parameter lanes.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-au-multitrack-2`)

- harness-view: `src/view/timeline.rs`, `src/view/timeline_ui.rs`, `src/view/timeline/**`, `src/view/mixer.rs` (rebuild hook only), NEW tests `tests/viewer_timeline_audio_2.rs` (existing timeline tests: keep compiling, extend only if a pinned behaviour changes, say which).
- folio: `src/audio_routing.rs` (+ its tests); pulse: `src/bus_mix.rs` (+ tests); harness-core only for a minimal one-step API if slice 1 needs it.
- Do NOT touch: harness-view `src/view/vector*` (SMB-AI-AREA-TYPE-3), folio `src/composition.rs` (SMB-AE-MASK-FEATHER-1), harness-core `src/action.rs` / `src/batch.rs` / recording hunks (SMB-PS-ACTIONS-1), pigment / develop (SMB-HDR-MERGE-1), root Cargo.toml, Cargo.lock, tools/, .config/. RUN-UI.md: list lines in notes.

</topic>

<topic id="proof" status="active" summary="Proof and push order">

## Proof

- Rebase onto origin right BEFORE each bundled run. Four held lanes push in order AREA-TYPE-3 -> PS-ACTIONS -> AE-MASK-FEATHER -> HDR-MERGE in the next hours; your folio change is upstream of all of them, so you push after them: commit locally, run your bundled check only after the lead tells you the chain has landed (or when you are otherwise done coding), then push with PUSH SAFETY.
- One bundled lane-check on C: (`--disk ssd`) over the WHOLE suites of folio, pulse, hsk-studio-package (rule 4b), harness-view + harness with the round's viewer/audio features (rule 4e), plus harness-core and its dependents if touched, plus direct dependents of any existing public item whose signature changes.
- Estimated gain: AU-14 from GUI-partial to track automation editable; data-integrity defect removed.

</topic>
