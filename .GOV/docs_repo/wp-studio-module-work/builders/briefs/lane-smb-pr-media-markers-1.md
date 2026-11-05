---
file_id: LANE-SMB-PR-MEDIA-MARKERS-1
file_kind: lane_brief
updated_at: 2026-11-05T15:58:35Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-MEDIA-MARKERS-1
lane_file: builders/lanes/hsk-studio-pr-media-markers-1.json
crates: [hsk-studio-folio (media item marker records), hsk-studio-session (document entry and commit), hsk-studio-pulse (resolved read)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Project-item markers live on a media item and appear on every clip cut from it (STU-VID-034)">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-VID-034 (:23849): "A PROJECT-ITEM marker lives on a media item and appears on every clip cut from it." Read MT-33736.

At base: clip and sequence scopes exist (`pulse/src/annotate.rs` `MarkerScope`; folio `StudioMarker` with name, comment, colour, duration and `MarkerRole` chapter, web-link, cue-point, caption). Nothing exists for the third scope: `git grep -E "project_item|ProjectItem|media_item"` finds none in folio or pulse. A clip's media identity is `ClipSource::Media { artifact_manifest_id, content_digest, asset_id }` (`folio/src/timeline.rs:346`). Follow the entry shape of `folio`'s track display and sequence settings records, `session/src/sequence_records.rs`, `sequence_records_step.rs`, and the notes of `builders/lanes/hsk-studio-pr-track-display-1.json`.

1. **Audit.** Read MT-33736 and record which rows the two existing scopes already prove, by run where you claim it.
2. **Folio records.** A record set of `StudioMarker`s keyed by media identity, in the media item's SOURCE ticks. Decide the key and state why (the content digest identifies the bytes; the manifest handle is caller-owned; `asset_id` is optional). Serde closed, unknown fields refused, counts bounded by a declared limit. `StudioMarker` says its id is unique among the document's markers: keep that true across all three scopes, and read how it is checked today.
3. **Session.** One document entry token, typed read and write, validated on read as sequence settings are. Carry it on `SequenceRecordsStep`'s `RecordEdit` rather than adding a step, unless reading it shows that cannot work; then say why. One revision, one receipt, undo and redo, keyed and `_as` forms, one Observe frame, codes and counters only. Typed add, replace and remove.
4. **Appears on every clip cut from it.** A read for a clip, and for a sequence, giving every marker that appears: the clip's own markers and the media item's markers inside the clip's source window, each labelled with its scope, at its timeline position through the same exact mapping playback uses (non-unit speed, reverse, hold). A marker outside the window is hidden, not lost, and reports nothing false. A clip whose source is a nested sequence has no media item: state that case. Session has no pulse edge, so the resolved read takes parsed records from the host. Red first, by a table.
5. **Removal.** The marker lives on the media item, not on a clip: removing the last clip cut from it leaves the entry valid and unchanged. A table row for that, and a row saying the entry names no sequence or clip.
6. **Round trip** (rule 4b) and save-and-reopen through the session.
7. **MTs.** Claim only rows this proves. Do not edit MT files; report proof-command problems in the lane notes.

Hostile input (V4-01/02): entry bytes and counts bounded; the resolved read bounded by existing limits and cancellable. Vendor parity UNVERIFIED (D5).

Out of scope: drawing markers; chapter, cue or caption export; a project or media pool surface; `SMB-PR-ENTRY-REFERENCES-1` and `SMB-TYPE-AUTHORED-TOKEN-1` also edit session, expect a rebase on `source.rs` and `session.rs`; controls registry or RUN_COMMANDS rows; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal session frame per operation, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the commit step takes the caller's context (CommandContext / ActorContext); never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-media-markers-1`, branch `lane/smb-pr-media-markers-1`, base 41d0e0da)

- folio: a new media marker module, `lib.rs` exports and DESCRIPTOR, tests.
- session: `src/source.rs` (token), `src/sequence_records.rs`, `src/sequence_records_step.rs`, `src/session.rs` (entry points and guards), `lib.rs` exports and DESCRIPTOR, tests.
- pulse: `src/annotate.rs` or a new module for the resolved read, `lib.rs` DESCRIPTOR, `tests/`.
- package: externals test only (4b).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on C: (`lane-check.sh --owner smb-pr-media-markers-1`): step 0; clippy over folio, session, pulse and their direct dependents; nextest WHOLE suites of folio, session, pulse and package (4b); doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-media-markers-1 --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
