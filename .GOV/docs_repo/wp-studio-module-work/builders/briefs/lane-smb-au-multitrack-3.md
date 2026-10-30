---
file_id: LANE-SMB-AU-MULTITRACK-3
file_kind: lane_brief
updated_at: 2026-10-30T15:40:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AU-MULTITRACK-3
lane_file: builders/lanes/hsk-studio-au-multitrack-3.json
crates: [hsk-studio-pulse, hsk-studio-harness-view, hsk-studio-folio (only if a field is needed)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Audition multitrack: clip pan envelope, master row, automated-fader hint">

## Target

SMB-AU-MULTITRACK-1 and SMB-AU-MULTITRACK-2 are PASS_V1 in STUDIO-R42. Read the `notes.engine_gaps_for_lead`, `envelope_rules_declared`, `track_automation_declared` and `spec_deltas` sections of `builders/lanes/hsk-studio-au-multitrack-2.json` first. Then close the gaps they list, in this order, and push after each green slice:

1. **Clip pan envelope.** Pulse's timeline mix (`crates/hsk-studio-pulse/src/audio.rs`, `TimelineMixStream`) gets a per-clip pan lane next to the existing per-clip volume envelope.
   - It follows edits by the same envelope rules MULTITRACK-2 declared for volume.
   - The timeline shows it as an editable lane with typed, undoable point add/move/delete, through the existing envelope commands and `Run::submit_sequence_and_entry`.
   - Test: pan automation changes the stereo balance of the mixdown sample by sample against a hand-computed pan law. Use pulse's existing pan law; do not invent one.
2. **Master row.** Folio already has `StudioAudioMaster { volume, effects }` (`crates/hsk-studio-folio/src/audio_routing.rs:115`).
   - Show a master row in the timeline with master volume and the master effect parameter lanes the engine already automates for buses (SMB-SCORE-EFFECTS-3/4, SMB-AUDITION-SESSION-2).
   - If the master needs an automation record folio does not have, add it additively and run the package suite (rule 4b).
   - If that needs a schema decision folio cannot take additively, stop and report the exact signature to the lead.
   - The mixdown path stays the single existing one.
3. **Automated-fader hint.** On a track with an automation lane, the mixer fader (`src/view/mixer.rs`) cannot be heard, because the lane replaces the fader.
   - Show that in the mixer as a visible state, not a tooltip only.
   - A fader move on an automated track either edits the lane at the playhead as one undo step, or is refused with a typed report. Choose one, name it in the notes as a Studio reading, and test it.

</topic>

<topic id="spec" status="active" summary="Spec basis and honesty">

## spec_basis

- Cite only what exists: `[STU-FX-137]` (Track volume/pan, `keyframe_mode`), `[STU-FX-135]`, `[STU-VID-040]` (one reversible step per operation), and the clauses MULTITRACK-2 cites.
- Module 14 has no AU-14 multitrack clause set. Audition's own behaviour (pan law, master row layout, fader behaviour under automation) is UNVERIFIED (D5).
- Add any missing clause to the lane notes as a spec-delta request. Do not edit the Master Spec.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-au-multitrack-3`, base a259a3df)

- pulse: `src/audio.rs` and the mix and automation modules it needs, plus new tests.
- harness-view: `src/view/timeline*`, `src/view/timeline/**`, `src/view/mixer.rs`, and new `tests/viewer_timeline_audio_3.rs`.
- folio: `audio_routing.rs`, additive, only if slice 2 needs it.
- Do not touch:
  - harness-view files outside the timeline and the mixer (SMB-VIEW-POLISH-1 is running there);
  - `hsk-studio-composite` (SMB-AE-EFFECT-FAMILIES-2);
  - `hsk-studio-harness-core` (SMB-R42-FOLLOWUPS-1). If you need a new `Run` step API, report it to the lead with the exact signature.
  - `RUN-UI.md`: list its lines in the notes instead.
- GUI lane 2 of max 2.

</topic>

<topic id="proof" status="active" summary="Proof and run timing">

## Proof and run timing

- Code and write the tests now. Pulse depends on composite, so SMB-AE-EFFECT-FAMILIES-2's push changes your closure.
  - Do not start your bundled run until the lead messages you that AE-EFFECT-FAMILIES-2 has pushed.
  - Then rebase onto origin and run once.
- The lead will name the disk for that run in the message: C: or D: (`--disk ssd|d`). A C: validation round may be running, and no builder runs on C: during a C: round.
- The run covers the whole suites of pulse and its dependents (harness-view, harness, interop-media) with the round's viewer features, plus the package suite if folio changed (rules 4a, 4b and 4e). Name the log per RUN LOGS.
- Headless viewer tests drive the real viewer the way `tests/viewer_timeline_audio_2.rs` does. Aim drags after the first egui frame.

</topic>
