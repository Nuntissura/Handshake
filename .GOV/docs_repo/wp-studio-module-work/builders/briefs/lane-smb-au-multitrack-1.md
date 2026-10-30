---
file_id: LANE-SMB-AU-MULTITRACK-1
file_kind: lane_brief
updated_at: 2026-10-30T03:50:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-AU-MULTITRACK-1
lane_file: builders/lanes/hsk-studio-au-multitrack-1.json
crates: [hsk-studio-harness-view]
model: opus
effort: medium
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules
---

<topic id="target" status="active" summary="Audition AU-14 multitrack GUI">

## Target

- App: Audition. Top gap rank 1 in `research/parity-20261023-video-audio.json` (per_app audition, top_gaps): "AU-14 multitrack timeline / clip view / automation lanes (GUI)", engine VALIDATED, GUI absent. GUI pooled doable today lists "Mixer / routing (F8; tracks-with-clips view still absent)".
- Deliver in Studio's own window: (1) audio tracks of the pulse timeline shown with their clips (waveform overview per clip from the existing waveform/peak data path), clip move/trim/gain on audio clips through the existing typed timeline commands; (2) automation lanes per track for volume and pan, and for bus/track effect parameters the engine already automates (SMB-SCORE-EFFECTS-3/4, SMB-AUDITION-SESSION-2): show the envelope, add/move/delete points as typed, undoable commands; (3) playback/mixdown of what is shown uses the existing mixdown path (no second mixer).
- Expected gain: Audition GUI pooled 60 -> 70 % unjudged (one workflow), strict row AU-14.

</topic>

<topic id="spec" status="active" summary="Spec basis and honesty">

## spec_basis

- Module 14 has no Audition multitrack-editor clause (parity file: grep of clause headings for multitrack: 0). Cite what exists: `[STU-FX-137]` audio routing is part of the document, `[STU-FX-135]` non-destructive audio stacks, and the timeline clauses the existing timeline panel already cites (read `builders/lanes/hsk-studio-timeline-ui-1.json` and `hsk-studio-timeline-trim-1.json` spec_basis). Grep only; do not read the whole module.
- Audition's own UI behaviour (point shapes, hold/bezier, lane colours, shortcuts) is UNVERIFIED unless a clause states it; do not invent parity. List the missing clauses in the lane notes as a spec-delta request.

</topic>

<topic id="paths" status="active" summary="Owned paths and do-not-touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-au-multitrack-1`)

- `crates/hsk-studio-harness-view/src/view/timeline*` (the timeline panel files; additive), NEW files under `src/view/timeline/` for the audio-track and automation-lane UI, NEW `tests/viewer_timeline_audio.rs` (do not edit existing test files except to keep them compiling).
- Engine crates are read-only for this lane. If a needed engine API is missing (e.g. automation point edit as a typed command), stop and report it to the lead with the exact signature rather than editing pulse/score/session.
- Do NOT touch `src/view/design/**` (SMB-R40-FOLLOWUPS-1), `src/view/shell.rs` / `src/view/mod.rs` beyond one additive registration hunk if unavoidable (name it in the notes), `RUN-UI.md` (write the manual text into rustdoc and list RUN-UI lines in the notes; the lead merges).

</topic>

<topic id="proof" status="active" summary="Tests and run">

## Proof

- Tests drive the real viewer headlessly like `tests/viewer_timeline*.rs` do (read one of them first and reuse its harness; aim drags after the first egui frame, R1 lesson in the timeline-ui-1 lane file).
- One bundled lane-check on C: covering the WHOLE harness-view and harness suites with the features the round units use (`.config/round-features.toml`; rule 4e: viewer tests compile only with the viewer feature unit).
- Model tier: opus/medium. At most 2 GUI lanes run at once; this is one of them.

</topic>
