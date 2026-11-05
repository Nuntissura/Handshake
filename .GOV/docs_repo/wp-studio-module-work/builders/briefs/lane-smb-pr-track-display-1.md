---
file_id: LANE-SMB-PR-TRACK-DISPLAY-1
file_kind: lane_brief
updated_at: 2026-11-05T10:05:08Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-TRACK-DISPLAY-1
lane_file: builders/lanes/hsk-studio-pr-track-display-1.json
crates: [hsk-studio-folio (track display records), hsk-studio-session (document entry and commit)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Track display state persisted per sequence and model-visible (STU-VID-036)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-VID-036 (:23870-23876). Read MT-33739. At base no code exists (`git grep -E "TrackDisplay|track_height|show_waveform"`: none). Follow the sequence settings shape, do not invent one: `crates/hsk-studio-folio/src/sequence_preset/settings.rs`, `crates/hsk-studio-session/src/sequence_records.rs`, `sequence_records_step.rs`, and the lane notes of SMB-PR-SEQUENCE-SESSION-1 (`declared_readings`, `lead_decisions`).

1. **Folio records.** A record set keyed by sequence id, beside the document (lead decision on the sequence settings placement, SD-47). Every declared switch by its spec name: video thumbnails (off, head only, head and tail, continuous), clip names, FX badges, proxy badges, content-credential badges, duplicate-frame markers, clip markers, source clip name and label, through-edits, audio waveforms (logarithmic or dynamic scaling, rectified option), audio names, audio channel labels, audio type badges, audio and video fade handles, audio and video keyframes, audio clip headers on small tracks. Track heights per track kind, named height presets with save and recall, expand-all and minimize-all as typed commands. A sequence with no entry resolves to declared defaults; declare them and label vendor parity UNVERIFIED (D5). Serde: closed enumerations, unknown fields refused the way the sequence settings refuse them.
2. **Session.** One document entry token, typed read and write, validated on read: refuse on read as sequence settings do (the file opens, the read gives the typed refusal, save keeps the bytes, a `LossReason::Opaque` note). Extend `SequenceRecordsStep` and its `RecordEdit` to carry this entry rather than adding a third copy of the step, unless reading it shows that cannot work; then state why. One revision, one receipt, undo and redo, keyed and `_as` forms, one Observe frame per operation, codes and counters only.
3. **Sequence removal.** Apply the existing rule: removal that would orphan an entry is refused on a timeline step, and the records step can remove a sequence with its entries in one revision. If reading it shows display state makes that rule hurt in a way settings does not, say so in the notes with the case; do not add a second rule.
4. **Package round trip** (rule 4b) and save-and-reopen through the session.
5. **MTs.** Read MT-33739 first; claim only rows this proves. Do not edit MT files; report proof-command problems in the lane notes.

Hostile input (V4-01/02): entry bytes and preset counts bounded by a declared limit, typed refusals. Vendor parity UNVERIFIED (D5).

Out of scope: drawing anything; thumbnails and waveform rendering; controls registry or RUN_COMMANDS rows; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal session frame per operation, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the commit step takes the caller's context (CommandContext / ActorContext); never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-track-display-1`, branch `lane/smb-pr-track-display-1`, base 27cba13b)

- folio: a new track display module, `lib.rs` exports and DESCRIPTOR, tests.
- session: `src/source.rs` (token), `src/sequence_records.rs`, `src/sequence_records_step.rs`, `src/session.rs` (entry points and guards), `src/lib.rs` exports and DESCRIPTOR, tests.
- package: externals test only (4b).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-pr-track-display-1 --disk d`; C: is reserved for STUDIO-R56): step 0; clippy over folio, session and session's direct dependents; nextest WHOLE suites of folio, session and package (4b); doctests if added.
- Push only after STUDIO-R56 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
