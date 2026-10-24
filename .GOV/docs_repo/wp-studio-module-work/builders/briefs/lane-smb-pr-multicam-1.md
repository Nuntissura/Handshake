---
file_id: BUILDER-BRIEF-LANE-SMB-PR-MULTICAM-1
file_kind: builder_brief
updated_at: 2026-10-24T01:17:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-PR-MULTICAM-1
lane_file: builders/lanes/hsk-studio-pr-multicam-1.json
crates: [hsk-studio-folio, hsk-studio-pulse, hsk-studio-session, hsk-studio-package (tests/externals.rs schema entry only)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip 1e3ae3d4); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, parity rows, expected gain, model">

## Target

- App: Premiere Pro. Row PR-10 Multicam is MISSING (weight 1.0 lost), rank 1 of the Premiere gaps, "lane_exists": false (parity-20261023-video-audio.json per_app premiere_pro top_gaps; spec STU-VID-033 :23597).
- Expected gain: PR-10 MISSING -> BUILT_NOT_VALIDATED (+2.0 pp Premiere engine, 25 rows; +4.0 pp once validated) if the auditors accept planners + records + flatten without playback resolution; PARTIAL (+1.0 pp) if they require the monitor to play the active angle (pulse resolves no nested sequence today, see risks). 9-app engine pool +0.08 to +0.16 pp.
- Model / effort: opus / high (cross-crate contract Folio record + Pulse planners + Session entry, rule 4b).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-pr-multicam-1`)

- Folio: NEW `crates/hsk-studio-folio/src/multicam.rs` (standalone record set `StudioMulticamRecords`, schema `hsk.studio.multicam_records@1`, `validate_multicam_records`, `multicam_records_schema`), NEW `tests/multicam_records.rs`; in `src/lib.rs` ONLY the two lines `mod multicam;` / `pub use multicam::*;` beside `mod clip_finishing;` (:86-87). Do NOT add fields to `StudioClip`, `StudioSequence`, `StudioTrack` or variants to `ClipSource`: 30 files across 6 crates build `StudioClip {..}` by literal and harness-view `timeline.rs:161` matches `ClipSource` exhaustively (lane SMB-TIMELINE-TRIM-1's file). Follow the `clip_finishing.rs` pattern ("`StudioDocument` and `StudioClip` gain no fields, so every existing document and clip literal is unchanged"). Do not edit the Folio DESCRIPTOR string tonight (lib.rs was touched by READY SMB-PS-FILL-LAYERS); list the descriptor text in lane notes.
- Pulse: NEW `crates/hsk-studio-pulse/src/multicam.rs`, NEW `tests/multicam.rs`; in `src/lib.rs` ONLY `mod multicam;` and `pub use multicam::*;` (lib.rs was touched by READY SMB-TIMELINE-UI-1 for `snap`: additive lines only).
- Session: NEW `crates/hsk-studio-session/src/multicam.rs` (`multicam_records` / `set_multicam_records`, validated on every set and read, pattern of `src/motion.rs:37-76`), `src/lib.rs` `mod multicam;` + entry-token re-export, `src/source.rs:150` add `"multicam"` to `DOCUMENT_ENTRY_TOKENS`; NEW `tests/multicam_entry.rs`.
- Package: `crates/hsk-studio-package/tests/externals.rs` ONE additive line in the schema list of `every_folio_reference_property_is_classified` (:390-408): `("multicam_records", serde_json::to_value(folio::multicam_records_schema()).unwrap()),` plus any classification the test then demands (READY roto and fill-layer lanes touched this file: additive hunks only).

## Do NOT touch

- `hsk-studio-harness-view/**` (two GUI lanes tonight; the multicam preview grid and camera keys in the window are a follow-up), `hsk-studio-composite/**`, `hsk-studio-prism/**`, `pulse/src/timeline_edit.rs` and `pulse/src/snap.rs` (call them), Folio `timeline.rs` (read only), any root file.

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-VID-033]`: "Multicam is a clip mode, not a separate editor. A multicam source sequence holds N synchronised angles; a clip referencing it exposes an `active_angle` that is switchable over time, producing a cut list. The normative contract:" — "Angle count is at least 16 in the addressable command surface; Studio imposes no fixed maximum beyond a document performance budget." — "Two switch modes: switch WITH a cut (creates an edit at the playhead and changes the angle after it) and switch WITHOUT a cut (changes the angle for the whole clip)." — "A record mode captures live angle switches during playback into a cut list; toggling record is a single command." — "A preview surface shows all angles in a paged grid with first/last/next/previous page navigation, next/previous angle, and next/previous edit navigation." — "`audio_follows_video` is a boolean: when true an angle switch also switches the audio." — "`flatten` bakes the current cut list into ordinary clips and discards the multicam mode; it is one reversible history step." — "A selection-order option (top-down versus source order) decides angle numbering."
- `[STU-VID-030a]`: "multicam clips MAY be set to follow or ignore the nest setting. Both are explicit booleans on the nested clip, never implicit."
- `[STU-VID-040]`: "A multicam record pass is one step."
- `[STU-VID-041]` command rows (grep each id): `cmd.clip.create.multicam` "Create Multi-Camera Source Sequence...", `cmd.clip.multicam.flatten` "Flatten", `cmd.clip.multicam.toggle` "Enable"; `multicam` namespace (53 commands): `cmd.multicam.choose.camera.1`..`.16` (`Ctrl+1`..`Ctrl+8` bound), `cmd.multicam.choosenocut.camera1`..`camera16` (`1`..`9` bound), `cmd.multicam.toggle.record` (`0`), `cmd.multicam.audio.follows.video`, `cmd.multicam.follows.nest.setting`, `cmd.multicam.page.first|last|next|previous`, `cmd.multicam.next.camera`, `cmd.multicam.previous.camera`, `cmd.multicam.next.edit`, `cmd.multicam.prev.edit`, `cmd.multicam.selection.top.down`. Engine commands carry these ids in their receipts; bindings belong to the later GUI lane.

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-folio-timeline-records.json`, `hsk-studio-pulse-editing.json` (gaps list names "PR-10 multicam"), `hsk-studio-premiere-color-titles.json` (standalone per-clip record set pattern), `hsk-studio-timeline-ui-1.json` (`Run::submit_sequence`, one step per edit).
- Code: Folio `src/timeline.rs:304 StudioClip`, `:394 StudioSequence`, `ClipSource::Sequence { sequence_id }` (the multicam clip's reference to its source sequence: reuse it), `src/clip_finishing.rs` (record set beside the document); Pulse `src/timeline_edit.rs:61 TimelineEdit::Razor` (switch with a cut), `src/snap.rs:168 adjacent_edit` (next/previous edit), `src/frame_cache.rs:127` (nested sequences only keyed, not resolved); Session `src/motion.rs`, `src/timeline.rs` (one timeline edit = one undo step via `TimelineStep`), `src/source.rs:150`.

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; push per green bundled run)

1. Folio record set (data + validation only): `StudioMulticamRecords { document_id, sources: [MulticamSource { sequence_id, angle_order: [track id], selection_order: top_down|source_order }], clips: [MulticamClipMode { clip_id, active_angle (1-based), audio_follows_video, follows_nest_setting, enabled }] }`; validation against the document: the source sequence exists and is not the clip's own sequence, the clip's `source_ref` is `ClipSource::Sequence` of a declared source, angle in 1..=angle count, >= 16 angles accepted, counts charged to `Budget` nodes, typed refusals (`InvalidField`, `DanglingReference`, `DuplicateIdentity`, `BudgetExceeded`). Studio reading (declared): one angle = one video track of the source sequence, audio taken from the paired audio track of the same index. Keep field names clear of the externals VOCAB substrings where possible (`source`, `path`, `media` ...); where unavoidable, classify. Tests: `multicam_records_validate_against_the_document`, `sixteen_angles_are_addressable`, `dangling_and_duplicate_entries_are_refused`, `records_absent_keep_documents_byte_identical`.
2. Pulse planners: `create_multicam_source` (angles from given clips aligned by caller-supplied offsets; audio-waveform or timecode sync is not specified by the spec: typed refusal `multicam_sync_unsupported`), `switch_angle { clip_id, angle, cut_at: Option<Ticks> }` (with a cut = Razor at the playhead + the right half takes the angle; without = whole clip), `toggle_audio_follows_video`, `toggle_follows_nest`, `angle_numbering(selection_order)`; each returns the new sequence + records as ONE step with its inverse. Tests: `switch_with_cut_razors_and_sets_the_later_angle`, `switch_without_cut_changes_the_whole_clip`, `audio_follows_video_switches_audio_with_video`, `selection_order_renumbers_angles`.
3. Record pass, navigation, flatten: `apply_record_pass(clip_id, events: [(Ticks, angle)])` -> one step (bounded event count, events outside the clip refused); preview paging model `MulticamPage { page_size, page }` with first/last/next/previous page, next/previous angle, next/previous edit (via `adjacent_edit`); `resolve_active_angle(clip, tick)` and `flatten(clip_ids)` baking the cut list into ordinary media clips with mapped source windows (audio per `audio_follows_video`), one step. Tests: `record_pass_is_one_step_with_one_inverse`, `paging_and_navigation_are_bounded`, `flatten_renders_the_same_cut_list_as_ordinary_clips`, `hostile_record_passes_are_refused_or_bounded`.
4. Session persistence: `multicam` document entry (`set_multicam_records` validates; a stored entry that no longer validates reads as the Folio diagnostic), saved and reopened byte-identically; package externals schema entry. Tests: `multicam_records_survive_save_and_reopen`, `stale_multicam_entry_reads_as_a_diagnostic`, plus the existing `every_folio_reference_property_is_classified` green with the new schema.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Engine tests only; fixtures build sequences with media clips (as `pulse/tests/timeline_edit.rs` does). Prove "one step" by applying the inverse and comparing the exact prior sequence and records.
- NOT_PROVEN (`NOT_PROVEN:` lines): playback of the active angle in the program monitor and render through composite (no nested-sequence resolution exists in pulse playback or composite); the paged preview grid in the window; Premiere behaviour (no Premiere oracle).

## Bundled lane-check (rule 4a whole suites of every changed crate + rule 4b package)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-pr-multicam-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-pr-multicam-1 --disk d -- cargo nextest run -p hsk-studio-folio -p hsk-studio-pulse -p hsk-studio-session -p hsk-studio-package --no-fail-fast
```

- Rule 4a: folio, pulse, session, package (all changed). Rule 4b: new Folio record set -> package suite included. The new session entry token may be pinned by harness tests that read entries (`crates/hsk-studio-harness/tests/audio_session.rs:274` pins its own token only); if a harness test fails on the token list, add `-p hsk-studio-harness` to the next run and report. Clippy covers folio's many direct dependents (long hold on D:). Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, gaps, conflicts, deadline">

## Edges and dependencies

- None (pulse -> folio, session -> folio/package already allowed). No crates.io dependency.

## UNVERIFIED (D5)

- Premiere multicam: angle numbering defaults, audio-follows-video default, record-pass cut placement at frame boundaries, flatten source-window mapping of speed-changed angles. No Premiere oracle.

## Spec gaps found (Operator decides Master Spec edits)

- STU-VID-033 does not specify how angles are synchronised when a source is created (audio waveform, timecode, in points), the angle <-> track mapping, or the default of `audio_follows_video` and the selection order. The lane uses declared Studio readings and a typed refusal for unsupported sync; list them for the HANDOFF spec-delta list.

## Risks and conflicts

- READY overlap: folio `lib.rs` (SMB-PS-FILL-LAYERS), pulse `lib.rs` (SMB-TIMELINE-UI-1), package `tests/externals.rs` (SMB-AE-ROTO-WIRING, SMB-PS-FILL-LAYERS): only the named additive lines; if R39 remediation edits these files first, rebase and re-run once.
- Same-crate sequencing: pulse is also read by SMB-TIMELINE-TRIM-1 (no edits there) and folio is the base of every Studio crate: never change an existing pub type.
- Expected-gain risk: without playback/render resolution of the active angle the row may be judged PARTIAL; `flatten` gives a renderable result today and is the mitigation.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes; a push needs its green bundled run, so while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`) instead of pushing an unverified commit.

</topic>
