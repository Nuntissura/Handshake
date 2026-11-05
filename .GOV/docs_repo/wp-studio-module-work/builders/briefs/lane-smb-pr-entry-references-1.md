---
file_id: LANE-SMB-PR-ENTRY-REFERENCES-1
file_kind: lane_brief
updated_at: 2026-11-05T15:17:11Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-ENTRY-REFERENCES-1
lane_file: builders/lanes/hsk-studio-pr-entry-references-1.json
crates: [hsk-studio-session (document entries against sequence and clip removal), hsk-studio-folio (entry validation)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Removing a sequence or clip never leaves a document entry unreadable">

## Target

STUDIO-R56 findings F-R56-3 (MEDIUM) and F-R56-7 (LOW): read both in `S/validation/round-R56.json`, evidence PROBE-R56-SESSION-1. Removing a sequence named by `audio_routing` commits with no refusal, through `submit_multicam` (`session/src/multicam_step.rs`) and through the records step, and leaves the whole routing set invalid for every sequence: `parse_audio_routing` answers `InvalidId audio_routing/<seq>` (`folio/src/audio_routing.rs` :463-468). `audio_routing` also validates `clip_gains` and `clip_pans` against the sequence's clips (:363-420), so removing a clip may do the same: read it and run it. R56 read `composition`, `clip_finishing` and `waveform_edit` by grep only.

1. **Table, measured (V4-06).** One table test over every token in `session/src/source.rs` `DOCUMENT_ENTRY_TOKENS` that names a sequence or clip, against every path that can remove one (timeline step, `submit_multicam`, records step, and any other you find). Tokens that name neither: listed not applicable with a reason. Per cell record what happens today: refused, kept valid, or unreadable. Prove the unreadable cells red by running.
2. **Rule: reuse, do not invent.** Three rules exist. Sequence settings and multicam refuse on a timeline step, and the records step can remove a sequence with its entries in one revision (`keep_settings_entry_valid`, `session.rs:1609`). Track display is view state: an entry naming an absent sequence is inert and blocks nothing (SD-49). Lead reading: `audio_routing` and any other entry that changes what is heard or rendered is document state and takes the first rule at sequence level. Extend the records step to carry it rather than adding another step.
3. **Clip-level removal.** Deleting a clip is a routine edit, and refusing it because the clip has a gain envelope would be wrong to assume. Measure first, then read what envelope maintenance does at base. If no existing rule covers the clip case, stop and send me the case and your proposed rule before coding it. In no outcome may one stale id make the whole set unreadable for other sequences.
4. **F-R56-7.** `Applied::sequence_records_inverse` returns a step that can never apply after a commit that replaced a parsable invalid entry, and a step combining that repair with a sequence edit loses its whole undo. Report the true state: an inverse that can apply, or a typed answer that no undo exists, and the sequence edit keeps its undo.
5. **Round trip.** Undo and redo for each fixed cell, save and reopen through the session, loss notes where the file is already damaged (the file opens, the read gives the typed refusal, save keeps the bytes). Package round trip if a record changes (rule 4b).
6. **MTs.** This lane is a correction: claim a row only if you read it and this proves it. Do not edit MT files.

Hostile input (V4-01/02): no new unbounded allocation; the check is bounded by the entries' existing limits. Vendor parity UNVERIFIED (D5).

Out of scope: `SMB-TYPE-AUTHORED-TOKEN-1` also adds a session entry, expect a rebase on `source.rs` and `session.rs`; track display's rule stays as it is; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal session frame per operation, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the commit step takes the caller's context (CommandContext / ActorContext); never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-entry-references-1`, branch `lane/smb-pr-entry-references-1`, base c143625f)

- session: `src/multicam_step.rs`, `src/sequence_records.rs`, `src/sequence_records_step.rs`, `src/session.rs`, `lib.rs` DESCRIPTOR, `tests/`.
- folio: `src/audio_routing.rs` and any entry validation the table proves wrong, `lib.rs` DESCRIPTOR, `tests/`.
- package: externals test only (4b).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-pr-entry-references-1 --disk d`): step 0; clippy over folio, session and session's direct dependents; nextest WHOLE suites of folio, session and package (4b); doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-pr-entry-references-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
