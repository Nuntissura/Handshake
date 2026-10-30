---
file_id: LANE-SMB-R42-FOLLOWUPS-1
file_kind: lane_brief
updated_at: 2026-10-30T12:45:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-R42-FOLLOWUPS-1
lane_file: builders/lanes/hsk-studio-r42-followups-1.json
crates: [hsk-studio-harness-core, hsk-studio-layout (NOTICE only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Close the three LOW findings of STUDIO-R42">

## Target

Close three LOW findings from round STUDIO-R42. They are in `validation/round-R42.json` → `findings_not_failures`; read each finding's `finding`, `repair_direction` and `file_line` first.

1. **F-R42-2.** When the entry write is refused, four `Run` methods undo their committed step and leave behind a redo step that has no entry. A later redo then re-applies the structure or sequence change without its entry, and the user's earlier redo history is lost. The four methods are `Run::restructure_and_entry`, `submit_sequence_and_entry`, `set_tiles_and_entry` and `set_vector_paths_and_entry` (`crates/hsk-studio-harness-core/src/harness.rs`).
   - Fix: save the redo stack before the edit and restore it after the rollback undo, as `play_action` does.
   - Test each method: after a refused entry write, the redo history equals what it was before the call. That means `nothing_to_redo` when it was empty, and the earlier redo step intact when there was one.
   - Write the tests red first at a259a3df.
2. **F-R42-3.** `Run::submit_sequence_and_entry` is missing from the STU-AUT-147 command corpus `RUN_COMMANDS` (`src/action.rs`).
   - Add `document.submit_sequence_and_entry` with `recordable: false`.
   - Call `note_unrecordable` under its own id, and run the inner submit and the rollback quietly, as `restructure_and_entry` does.
   - Turn `corpus_lists_every_run_operation_with_recordability` (`tests/action_records.rs`) into a check that cannot miss a new mutating `pub fn` of `impl Run`. If a fully mechanical check is not practical, at least add the new row and say why in the notes.
3. **F-R42-4.** `crates/hsk-studio-layout/NOTICE` links the two W3C licences but does not include them.
   - Copy the full texts of the W3C Software and Document License 2015 (https://www.w3.org/Consortium/Legal/2015/copyright-software-and-document) and 2023 (https://www.w3.org/copyright/software-license-2023/) verbatim, fetched today, into NOTICE or into a `LICENSES-W3C` file that NOTICE names.
   - Keep the per-document notices, dated URIs and notice of changes that are already there.
   - Keep the pin test `cjk_sources_cite_the_w3c_software_and_document_licences` green, and extend it so it checks that the texts are present.
   - These licences are permissive and copying them is what they require. Do not read any GPL, LGPL or MPL text.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths

- `hsk-studio-harness-core`: `src/harness.rs` (only the four `*_and_entry` methods and the history helpers they need), `src/action.rs` (the corpus row), `tests/run_restructure_and_entry.rs`, `tests/action_records.rs`, and a new `tests/run_and_entry_redo.rs` if that is cleaner.
- `hsk-studio-layout`: `NOTICE`, an optional new `LICENSES-W3C`, and the pin test file.
- Do not touch:
  - `hsk-studio-harness-view` (SMB-VIEW-POLISH-1 is running);
  - `hsk-studio-composite` (SMB-AE-EFFECT-FAMILIES-2 is running);
  - `hsk-studio-pulse` (SMB-AU-MULTITRACK-3);
  - any root input (Cargo.lock, `.config/**`, `tools/**`).

</topic>

<topic id="proof" status="active" summary="Proof and push order">

## Proof and push order

- Run one bundled lane-check on C: (`--disk ssd`, your own target `smb-r42-followups-1`), with the log named per RUN LOGS. It covers:
  - harness-core's whole suite and its direct dependents (harness-view, harness) with the round's viewer features (rules 4a and 4e);
  - layout's whole suite.
- Other builders hold the C: lock in slices, and that is fine. If a validation round holds C:, wait for it and never touch locks.
- Push order: harness-core sits upstream of harness-view, so push after SMB-VIEW-POLISH-1 has pushed.
  - Rebase onto origin before you push.
  - A rebase that brings only harness-view or composite code does not touch harness-core's dependencies, so it needs no re-run.
- History semantics follow `[STU-AUT-...]` / STU history clauses as harness-core already cites them. Photoshop/Illustrator behaviour stays UNVERIFIED (D5).

</topic>
