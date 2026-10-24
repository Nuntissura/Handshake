---
file_id: BUILDER-BRIEF-LANE-SMB-PR-MULTICAM-2
file_kind: builder_brief
updated_at: 2026-10-24T03:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-MULTICAM-2
lane_file: builders/lanes/hsk-studio-pr-multicam-2.json
crates: [hsk-studio-pulse, hsk-studio-session]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip 7a16f66d); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
depends_on: [SMB-PR-MULTICAM-1 READY at dbc31c18]
---

<topic id="target" status="active" summary="App, rows, expected gain, model">

## Target

- App: Premiere Pro, row PR-10 Multicam. SMB-PR-MULTICAM-1 built records, planners, flatten and the session entry; its `next` lists the engine follow-ups this lane closes: "session one-step commit of a CREATED source sequence (TimelineStep is one existing sequence), flatten of speed-changed angles and of clips with markers". Its declared readings are the current refusals: "unit forward speed only for resolve/flatten (multicam_speed_unsupported otherwise)" and "clips with markers refused multicam_markers_present".
- Expected gain: closes three of MULTICAM-1's declared engine gaps, so PR-10 is BUILT_NOT_VALIDATED without those caveats (+1.0 pp Premiere engine if MULTICAM-1 is judged PARTIAL for them; 0 if judged BUILT). Playback/render of the active angle (pulse playback and composite resolve no nested sequence) stays NOT_PROVEN and is the remaining PARTIAL risk. No GUI gain.
- Folio DESCRIPTOR text left by MULTICAM-1: ASSIGNED TO SMB-PS-SOFT-PROOF-2 (it already edits Folio `lib.rs` for its own record set; one Folio writer tonight). This lane does not touch Folio.
- Model / effort: opus / high (cross-crate contract: a new session commit kind for multi-sequence + record steps; rational speed mapping with exactness rules).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-multicam-2`)

- Pulse: `crates/hsk-studio-pulse/src/multicam.rs` (flatten, resolve, speed and marker mapping; split into `src/multicam/*.rs` children declared inside `multicam.rs` if it grows; no `lib.rs` edit needed while `pub use multicam::*` covers new names), `tests/multicam.rs` or NEW `tests/multicam_flatten.rs`.
- Session: `src/multicam.rs`, `src/timeline.rs` (or a NEW `src/multicam_step.rs`), `src/session.rs` (submit pipeline reuse only, additive), `src/lib.rs` (`mod`/`pub use` lines and the session DESCRIPTOR text for the new API), NEW `tests/multicam_commit.rs`.

## Do NOT touch

- `crates/hsk-studio-session/src/source.rs` (SMB-PS-SOFT-PROOF-2 adds the `proof` token there), every Folio file (SOFT-PROOF-2 owns Folio tonight, DESCRIPTOR included), `hsk-studio-package/**`, `hsk-studio-harness-view/**` (TRIM-1 and SOFT-PROOF-2), `pulse/src/lib.rs`, `pulse/src/timeline_edit.rs`, `pulse/src/snap.rs` (call them). If a Folio change turns out to be required, stop and report.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-VID-033]`: "`flatten` bakes the current cut list into ordinary clips and discards the multicam mode; it is one reversible history step." and "a clip referencing it exposes an `active_angle` that is switchable over time, producing a cut list."
- `[STU-VID-040]`: "Every operation in this sub-section is one reversible `StudioHistoryEntry` (14.19). ... A multicam record pass is one step."
- `[STU-VID-031]`: "`speed` is a rational multiplier ... A clip's speed and duration are two views of one value: fixing either derives the other. Reverse is a negative speed."
- `[STU-VID-034]`: "A CLIP marker travels with the clip and moves when the clip moves. A SEQUENCE marker is positioned in sequence time and does not move with clips. ... Each carries a name, a comment, a colour, a duration (a marker may be a span, not only a point)".
- `[STU-VID-090]` obligation 3 (grep "A determinate result is required for every refusal"): "Insufficient handle, sync-lock conflict, source exhaustion, ... each have a named result naming the specific obstruction." This is the basis for every typed refusal below; STU-VID-033 states no speed, marker or reverse mapping for flatten (MULTICAM-1 spec_deltas), so each mapping is a declared Studio reading.

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-pr-multicam-1.json`: capabilities (slices 1-4), `notes.declared_readings`, `notes.spec_deltas`, `next`.
- Code at 7a16f66d: pulse `src/multicam.rs:1-45` (module docs, semantics), `:164 MulticamProposal` (snapshot, records, inverse, splits, dropped transitions), `:207 sequence_change` (only one existing sequence), `:649 plan_create_multicam_source` (adds a sequence: `sequence_change` returns `None`), `:807` unit-speed check, `:882` markers refusal, `:939 plan_multicam_flatten`; session `src/timeline.rs` (`TimelineStep`: one existing sequence, expect -> write, inverse), `src/session.rs:744 submit_timeline` / `:764 submit_timeline_keyed`, `src/multicam.rs:30 multicam_records`, `:41 set_multicam_records`; Folio `StudioClip.markers` (clip markers in SOURCE ticks), `Speed` (rational, `UNIT`).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; push rules in risks)

1. Session one-step multicam commit: `Session::submit_multicam(step)` (+ `_keyed`) committing ANY `MulticamProposal` result, meaning zero or more sequence values (replace an existing one, or INSERT a created source sequence) plus the `multicam` document entry, as ONE revision, ONE receipt and ONE inverse (insert <-> remove; removing a sequence still referenced by a clip is refused by Folio validation), reusing the submit pipeline (grant, idempotency key, admission, commit authority, publish). Preconditions: the expected sequences and the expected records must equal the live ones (`value_conflict` otherwise); refusals leave the session untouched. Tests: `created_source_sequence_and_records_commit_as_one_step`, `undo_removes_the_created_sequence_and_redo_restores_it`, `flatten_and_record_pass_commit_as_one_step_with_their_records`, `stale_expectation_is_value_conflict_and_changes_nothing`, `committed_multicam_step_survives_save_and_reopen`.
2. Flatten and resolve at non-unit FORWARD speed: multicam clip speed `s` and angle-clip speed `a` (rationals): a piece keeps its source window, its timeline span maps through `s`, its speed is the exact product `s * a`; an inexact derived duration is refused `multicam_speed_inexact`; reverse (negative) speed refused `multicam_reverse_unsupported` (declared reading); `resolve_active_angle` uses the same mapping. Tests: `flatten_maps_a_double_speed_clip_exactly`, `angle_clip_speed_multiplies_into_the_piece`, `inexact_speed_products_are_refused_named`, `reverse_speed_is_refused_named`, `resolve_active_angle_at_non_unit_speed`.
3. Flatten with clip markers: each clip marker (source-sequence ticks of the multicam clip) moves to the piece whose span contains its start, converted to that piece's source ticks (through the slice 2 mapping); a marker in a gap of the angle track is refused `multicam_marker_in_gap`, a span marker crossing a piece edge is refused `multicam_marker_spans_pieces` (declared readings; never converted to a SEQUENCE marker, STU-VID-034 scopes differ); marker ids, name, comment, colour, duration and role kept. Tests: `flatten_carries_clip_markers_to_their_pieces`, `marker_in_a_gap_is_refused_named`, `span_marker_across_pieces_is_refused_named`, `hostile_marker_counts_are_bounded`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Engine tests only; prove "one step" by applying the inverse and comparing the exact prior document sequences and records; existing MULTICAM-1 tests that pinned `multicam_speed_unsupported` / `multicam_markers_present` must be updated to the new behaviour and said so in lane notes (rule 4a, F-R35-3).
- NOT_PROVEN (`NOT_PROVEN:` lines): program-monitor playback / composite render of the active angle; Premiere's flatten of speed-changed angles and marker placement (no Premiere oracle).

## Bundled lane-check (rule 4a whole suites of every changed crate)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-pr-multicam-2/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-pr-multicam-2 --disk d -- cargo nextest run -p hsk-studio-pulse -p hsk-studio-session --no-fail-fast
```

- Rule 4a: pulse, session. Rule 4b: no Folio field -> package suite not required (state it). Session public API is additive; if a direct dependent (harness-core, harness-view, harness, kernel-bindings) fails clippy on it, add its suite to the next run and report. Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, conflicts, deadline">

## Edges and dependencies

- None (session does not need pulse: it commits Folio values; pulse -> folio, session -> folio/package already allowed). No crates.io dependency.

## UNVERIFIED (D5)

- Premiere: flatten of speed-changed angles, marker placement after flatten, reverse-speed multicam behaviour.

## Spec gaps found (Operator decides Master Spec edits)

- STU-VID-033 still states no flatten mapping for speed, reverse speed or clip markers; this lane's readings and refusals go to the HANDOFF spec-delta list.

## Risks and conflicts

- V4-14: SMB-PR-MULTICAM-1 is READY on pulse and session and unjudged. Keep commits local until the lead writes the frozen round-candidate sha that judges it into this lane file (or lists your commits in its claim); then rebase, run, push. Pushing earlier makes MULTICAM-1's claim stale.
- SMB-PS-SOFT-PROOF-2 edits session `source.rs` (token list) in parallel: disjoint files; both run the session suite; the second to push rebases and re-runs once.
- One-step commit of a document value plus a document entry is new to the session: the refusal path must leave both untouched (test it), or the history and the entry can diverge.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, subject to the V4-14 hold above and to a green bundled run per push; while either blocks, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
