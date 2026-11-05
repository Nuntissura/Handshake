---
file_id: LANE-SMB-PR-SEQUENCE-SESSION-1
file_kind: lane_brief
updated_at: 2026-11-05T04:03:12Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-SEQUENCE-SESSION-1
lane_file: builders/lanes/hsk-studio-pr-sequence-session-1.json
crates: [hsk-studio-session (document entry and commit step for sequence settings and sequence presets), hsk-studio-folio (doc corrections only), hsk-studio-pulse (time-display agreement test)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Make sequence settings and presets survive a session save and reopen, with attributed undoable commits">

## Target

SMB-PR-SEQUENCE-PRESETS-1 (cc58a511, READY) added `StudioSequenceSettingsRecords` and the preset section in folio, with a package-level round trip only. Read its lane file `builders/lanes/hsk-studio-pr-sequence-presets-1.json`, especially `notes.gaps_outside_lane`, `notes.known_doc_defects_at_cc58a511`, `notes.declared_readings`, `notes.lead_decisions`. Its gap: session `DOCUMENT_ENTRY_TOKENS` has no token for these records and nothing produces the entries, so a session save and reopen loses them.

Follow the existing pattern, do not invent one: `crates/hsk-studio-session/src/multicam.rs` (typed document entry `studio/multicam.<sha256>`) and `multicam_step.rs` (commit step), `Session::submit_multicam` / `submit_multicam_keyed` / `submit_multicam_as`.

1. **Document entries.** Add the tokens to `DOCUMENT_ENTRY_TOKENS` and typed read / write in the session, validated against the document on open. A package without the entries opens unchanged. An entry that does not validate is a typed refusal, never a silent drop.
2. **Commit step.** A step that applies a records change as one revision, one receipt, with undo and redo, with the keyed and `_as` (`CommandContext`) variants, one Observe frame per operation, codes and counters only. It carries the 3 folio command ids `sequence.preset.save_from_sequence`, `sequence.preset.apply`, `sequence.editing_mode.set` with the caller's context. Refusals leave the document and the entry untouched. A sequence removal that would orphan a settings entry is refused the way multicam refuses it; read how multicam handles this and match it.
3. **Round trip.** Save, reopen, compare, through the session, for settings with immersive and preview, a preset library, and an editing mode. Undo after reopen is out of scope if multicam does not do it; state what you found.
4. **Time-display agreement.** The folio lane matched its 15 time-display members and codes to Pulse's `TimeDisplay` by reading `pulse/src/display.rs`; no test pins it. Pulse already depends on folio, so add a test in pulse that fails on any disagreement, in both directions, code and member count.
5. **Doc defects.** Fix the 4 listed in `known_doc_defects_at_cc58a511`. For `shipped.rs`, say where the rows come from (STU-VID-016a) and what pins them; do not add the generator script to the repo.
6. **MTs.** Read MT-33701 and MT-33702 first. Claim only rows this proves. Do not edit MT files; report proof-command problems in the lane notes.

Hostile input (V4-01/02): entry bytes bounded by the existing limits; typed refusals. Vendor parity UNVERIFIED (D5).

Out of scope: moving the records onto `StudioSequence` (lead decision); controls registry or RUN_COMMANDS rows; importers (STU-VID-013a); any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal session frame per operation, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the commit step takes the caller's context (CommandContext / ActorContext); never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-sequence-session-1`, branch `lane/smb-pr-sequence-session-1`, base cc58a511)

- session: a new sequence-records module and step module, `src/source.rs` (tokens only), `src/session.rs` (the submit entry points), `src/lib.rs` exports and DESCRIPTOR, tests.
- folio: `src/sequence_preset.rs`, `src/sequence_preset/**` doc comments only.
- pulse: one new test file.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; a pushed red commit must compile the round units of session and its direct dependents.
- One bundled run on D: (`lane-check.sh --owner smb-pr-sequence-session-1`; C: is reserved for STUDIO-R55): step 0; clippy over session, folio, pulse and session's direct dependents; nextest WHOLE suites of session, folio, package (4b) and pulse; doctests if added.
- Push only after STUDIO-R55 is recorded and the lead releases you: rebase, `--per-sha`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
