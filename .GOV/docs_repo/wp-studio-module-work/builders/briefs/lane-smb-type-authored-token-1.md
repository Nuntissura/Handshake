---
file_id: LANE-SMB-TYPE-AUTHORED-TOKEN-1
file_kind: lane_brief
updated_at: 2026-11-05T15:17:11Z
wp: WP-KERNEL-STUDIO
lane: SMB-TYPE-AUTHORED-TOKEN-1
lane_file: builders/lanes/hsk-studio-type-authored-token-1.json
crates: [hsk-studio-session (authored composition token and upgrade), hsk-studio-harness-view (reads the stored token), hsk-studio-type (DESCRIPTOR)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="A stored document recomposes under its authored token until a recorded, undoable upgrade">

## Target

Spec: Master Spec v02.229 `14-studio-creative-suite.md` STU-TYP-130 (:8096): a stored document reopened under a newer token MUST recompose under the token it was authored with unless explicitly upgraded, and the upgrade MUST be a recorded, undoable, diffable operation. Read MT-30898. STUDIO-R56 F-R56-4 (MEDIUM) and F-R56-6 (LOW): read both in `S/validation/round-R56.json`.

At base: `hsk-studio-type/src/validate.rs` declares `COMPOSITION_VERSION` (@3), `COMPOSITION_VERSION_V2` and `COMPOSITION_VERSION_V1`, and Type can compose under each. No record stores a token and no upgrade exists. The only non-Type use is `harness-view/src/view/text.rs:598`, which always passes the current constant, so every Viewer document recomposes under @3 today.

1. **Store.** The smallest record that holds the authored token, following the sequence settings shape (`folio/src/sequence_preset/settings.rs`, `session/src/sequence_records.rs`, the notes of `builders/lanes/hsk-studio-pr-sequence-session-1.json`). Decide per document or per story from the spec's wording and state why. A new document records the current token when it is created. Unknown fields are refused the way sequence settings refuse them. If you use a new entry token, add it to `DOCUMENT_ENTRY_TOKENS`.
2. **A document with no stored token.** Every existing file. The spec is silent. Lead reading: it reads as @1 with a typed note, because any later token would claim semantics the file never recorded. If the code shows that hurts (for example the Viewer refuses text it shows today), stop and send me the case. Record the reading in `notes.declared_readings` and `notes.spec_proposals`.
3. **Compose under it.** The Viewer reads the stored token at `text.rs:598` and whatever carries it there; list every other place a host composes from a stored document. A document authored under @1 or @2 gets the same lines Type's fixtures give for that token. A token string this build does not know gets a typed refusal, never the newest token: the file opens, the read gives the refusal, save keeps the bytes, a `LossReason::Opaque` note. Red first, by running.
4. **Upgrade.** A session operation in the existing commit shape: one revision, one receipt, undo and redo, keyed and `_as` forms, one Observe frame, codes and counters only. Read what the spec and session mean by diffable and follow it; at least from-token, to-token and the stories it affects. Refuse an unknown target and a move to an older token with typed codes, changing nothing.
5. **Round trip** (rule 4b) and save-and-reopen through the session: the token survives, and an upgraded document recomposes under the new token after reopen.
6. **F-R56-6.** Type's DESCRIPTOR still says `COMPOSITION_VERSION (v2)`: bring it to @3, naming `COMPOSITION_VERSION_V2` and the forced line break. The harfrust spike stays v1 only.
7. **MTs.** Read MT-30898 first; claim only rows this proves. Do not edit MT files; report proof-command problems in the lane notes.

Hostile input (V4-01/02): token bytes bounded by a declared limit, typed refusals. Vendor parity UNVERIFIED (D5).

Out of scope: changing any token's semantics or any parity row; the harfrust spike following later tokens; `SMB-PR-ENTRY-REFERENCES-1` also edits session, expect a rebase on `source.rs` and `session.rs`; controls registry or RUN_COMMANDS rows; any GUI; no egui, eframe, winit or wgpu line; no crate `Cargo.toml` change (a new edge is steward-owned: report it); no root input; no Master Spec edit.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe: one terminal session frame per operation, codes and counters only", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: the commit step takes the caller's context (CommandContext / ActorContext); never default_actor()", "jobs": "none", "settings": "none", "refactor_exposure": "low"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-type-authored-token-1`, branch `lane/smb-type-authored-token-1`, base c143625f)

- session: `src/source.rs` (token), a new module or `src/sequence_records.rs` beside it, `src/session.rs` (entry points), `lib.rs` exports and DESCRIPTOR, `tests/`.
- folio: only if the record belongs there, following the sequence settings shape; `lib.rs` exports and DESCRIPTOR, `tests/`.
- harness-view: `src/view/text.rs` and what carries the token to it, no GUI line, `tests/`.
- type: `lib.rs` DESCRIPTOR only.
- package: externals test only (4b).

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-type-authored-token-1 --disk d`): step 0; clippy over type, folio, session and harness-view and their direct dependents; nextest WHOLE suites of type, folio, session, harness-view and package (4b); doctests if added.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-type-authored-token-1 --disk d --per-sha origin/codex/handshake-studio-assets`, push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
