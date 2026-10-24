---
file_id: BUILDER-BRIEF-LANE-SMB-FIG-EDIT-2
file_kind: builder_brief
updated_at: 2026-10-24T06:40:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIG-EDIT-2
lane_file: builders/lanes/hsk-studio-fig-edit-2.json
crates: [hsk-studio-folio, hsk-studio-session (src/source.rs token only), hsk-studio-package (tests/externals.rs only), hsk-studio-harness-view (view/design*, tests/viewer_fig_edit*.rs), hsk-studio-harness (RUN-UI.md only)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip df252615); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
depends_on: [SMB-FIG-EDIT-1 READY at 74daa7a8]
sequencing: "this lane pushes its Folio/package slice FIRST; SMB-ID-LIQUID-LAYOUT-1 edits folio src/lib.rs and package tests/externals.rs only after that commit is on origin"
---

<topic id="target" status="active" summary="App, rows, expected gain, model">

## Target

- App: Figma. FIG-EDIT-1 made the instance inspector work, but its lane file declares: "instance records changed by a design command live for the session only: no session document entry holds design records (vector entry validate_entry_styles in vector/style.rs admits paint styles + fill nodes only; not owned; no new session token allowed) -> after save+reopen an edited instance reads state unknown and refuses commands until re-import." Lead decision 2026-10-24: a Folio design-state record + session token `design` + package classification (rule 4b), pattern of `hsk.studio.proof_records@1` / `hsk.studio.multicam_records@1`.
- Expected gain: the workflow "Components, variants, instances, overrides" (judged doable-unjudged with FIG-EDIT-1) stops losing edits on save/reopen, which an auditor would otherwise mark as not doable (edit-preserve fails). It protects FIG-EDIT-1's +0.5 pooled point (1 of 7 Figma workflows, 7.1 pp unjudged) rather than adding a new one; engine FG rows unchanged.
- Model / effort: opus / high (judged higher than the lead's default: cross-crate contract — a Folio record consumed by harness-view, a session token, and the package externals classifier, the class that failed three rounds before rule 4b).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch, sequencing">

## Owned paths (lane worktree `wt-studio-lanes/smb-fig-edit-2`)

- Folio: NEW `crates/hsk-studio-folio/src/design_state.rs` (record set `StudioDesignStateRecords`, schema `hsk.studio.design_state@1`, `validate_design_state_records(records, document, design: Option<&StudioDesignRecords>)`, `parse_design_state_records`, `design_state_records_schema`), NEW `tests/design_state_records.rs`; `src/lib.rs` `mod design_state;` / `pub use design_state::*;` + the DESCRIPTOR text for the set.
- Session: `crates/hsk-studio-session/src/source.rs` add `"design"` to `DOCUMENT_ENTRY_TOKENS` (:154) + its doc line. No other session file.
- Package: `crates/hsk-studio-package/tests/externals.rs` one schema line `("design_state", serde_json::to_value(folio::design_state_records_schema()).unwrap()),` + the classifications it demands.
- Harness-view: `src/view/design.rs`, `src/view/design/**` (write the `design` entry in the same step as `design/instance.rs:640-643` writes the vector/text entries; read it on open and restore each instance's state as `edited`), `tests/viewer_fig_edit.rs` (fix clippy `bool_assert_comparison` :271 and explicit deref :510 only), NEW `tests/viewer_fig_edit_persist.rs`.
- Harness: `crates/hsk-studio-harness/RUN-UI.md` (the instance-inspector section: persistence lines). FIG-EDIT-1 is READY, so RUN-UI.md is free; SMB-LR-HEALING-2 writes its lines to lane notes, not to this file.

## Sequencing with SMB-ID-LIQUID-LAYOUT-1 (shared files)

- Folio `src/lib.rs` and package `tests/externals.rs` are edited by both lanes. Rule: this lane commits slice 1 (Folio record + lib.rs + externals + session token) and pushes it as its FIRST push after one green bundled run; ID-LIQUID-LAYOUT-1 touches those two files only after that sha is on origin (it rebases onto it). Never both editing them at once.

## Do NOT touch

- `src/view/mod.rs`, `src/view/shell.rs`, `src/view/file_dialog.rs` (no hunk expected: design state hangs off the existing design module; if a hook in mod.rs is unavoidable, one additive line, recorded), `src/view/vector.rs` / `vector/style.rs` (the lead chose a separate record, not vector-entry members), `src/view/develop*` (SMB-LR-HEALING-2), `hsk-studio-layout`, `folio/src/layout_rules.rs` (SMB-ID-LIQUID-LAYOUT-1), `hsk-studio-interop-fig*`, `hsk-studio-components` (call them).

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-IO-006]` row 35: "| 35 | Figma design document (local copy) | `.fig` | Figma family | I·EP·X·RT | Frames, components, variants, variables, auto-layout, prototype flows transformed; **local-copy** round-trip ..." with "**EP** = edit-preserve" — an edit made in Studio must survive save and reopen.
- `[STU-DS-003]`: "`StudioComponentInstance` ... is a live reference to a `StudioComponent`. An instance renders the current component definition plus its own local overrides ([STU-DS-016]); it is not a copy."
- `[STU-DS-008]`: "setting a variant property re-selects the member definition in place while preserving compatible overrides ([STU-DS-017])."
- `[STU-DS-016]`: "Overridable surfaces are: text content, fills/strokes/paint, effects, visibility, nested instance swaps, and exposed-nested property values."
- `[STU-DS-017]`: "Overrides MUST persist across variant switches and component swaps when the target structure matches by layer name and hierarchy, and MUST be resettable both per-property and wholesale (reset-all-overrides)."
- `[STU-DS-163]`: "Concurrent design-system mutations MUST use the expected-revision precondition of [STU-SDB-004] at the granularity of the individual `component`, `component_set`, `variable`, `variable_collection` or style record".
- `[STU-IO-005]`: "Unsupported-feature handling MUST be lossless-of-intent where the format is a native round-trip target:" (a state the record cannot hold is a named loss, never silent).

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-fig-edit-1.json`: capabilities, `declared_readings` (records per instance-layer content signature; binding layers dropped, bindings in the receipt `design.instances`; redraw = translation only), `lint_debt`, `infra` (os error 1455).
- `builders/lanes/hsk-studio-ps-soft-proof-2.json` and `hsk-studio-pr-multicam-1.json` (record-set + token + externals pattern, `notes.shared_files`).
- Code: harness-view `view/design.rs`, `view/design/instance.rs:640-643` (entry writes per command), `view/design/fig_import.rs` (receipt `design` block), `view/proof.rs:383, :558` (read/validate pattern for a Folio-validated entry), `view/audio_session.rs:4-40` (`Run::set_document_entry` = one undoable step, saved); folio `src/proof_state.rs` / `src/multicam.rs` (record pattern), `src/records.rs:92 StudioDesignRecords` and `src/components.rs`; session `src/source.rs:154`.

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Record + token + classification (FIRST push, see sequencing): `StudioDesignStateRecords { document_id, instances: [StudioInstanceState { layer_id, instance_id, component_id, variant: { property: value }, properties: { name: typed value }, overrides: [{ node_id, field, value }], swap_target: Option<component_id>, content_signature }] }`; validation: `layer_id` is a group layer of the document, ids unique, at most one state per layer, override fields within the STU-DS-016 surfaces, and, with design records supplied, component/variant/property names exist; typed refusals, Budget nodes, absent = byte-identical; `"design"` token; externals schema line. Tests: `design_state_validates_against_document_and_design_records`, `unknown_component_or_variant_is_refused`, `absent_design_state_keeps_documents_byte_identical`, package `every_folio_reference_property_is_classified`.
2. Write path: every `DesignCommand` (variant switch, property, override, reset, reset-all, swap) writes the `design` entry together with the vector/text entries as ONE undo step (verify with the run history; if `Run` commits each entry separately, batch them through the existing API or report). Tests: `design_command_writes_one_undo_step_with_the_state_entry`, `undo_restores_the_previous_state_entry`.
3. Read path: on open, the `design` entry (Folio-validated; an invalid or stale entry reads as its Folio diagnostic and the instance shows `unknown` with that reason) restores each instance's state as `edited`, so commands work after reopen. Tests: `edited_instance_survives_save_and_reopen`, `reopened_instance_accepts_further_commands`, `stale_design_entry_reads_as_a_diagnostic`.
4. Lint debt: the two clippy fixes in `tests/viewer_fig_edit.rs` (:271, :510), no behaviour change.

</topic>

<topic id="tests" status="active" summary="Test shape, the bundled lane-check, infra">

## Test shape

- Headless egui + AccessKit as FIG-EDIT-1 (settle the layout before pointer events, F-TL-2); assertions through `state_json()["design"]` (state `edited` after reopen), the session `design` entry bytes and the run history.
- NOT_PROVEN (`NOT_PROVEN:` lines): Figma's own persistence of instance overrides in a `.fig` round trip (no `.fig` export from the window; no Figma oracle).

## Bundled lane-check (C: SSD, rule 4a + 4b)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-fig-edit-2/handshake-studio-assets" && CARGO_BUILD_JOBS=6 bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-fig-edit-2 --disk ssd -- cargo nextest run -p hsk-studio-folio -p hsk-studio-session -p hsk-studio-package -p hsk-studio-harness-view -p hsk-studio-harness --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: folio, session (token list), package, harness-view, harness (RUN-UI.md; viewer suites). Rule 4b: new Folio record set -> package suite included. Loop cap: two failed runs on the same failure, then stop and report verbatim.
- Infra (FIG-EDIT-1 finding): C: builds hit os error 1455 (paging file too small) at default parallelism; set `CARGO_BUILD_JOBS=6` in the environment of your own lane-check and cargo runs on C: (builders only, never rounds).

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, conflicts, deadline">

## Edges and dependencies

- None: harness-view -> folio/session/components declared. No crates.io dependency.

## UNVERIFIED (D5)

- Figma's instance-state model (which overrides survive a component edit upstream), variant property ordering.

## Risks and conflicts

- Shared with SMB-ID-LIQUID-LAYOUT-1: folio `lib.rs`, package `tests/externals.rs` (sequencing above). Shared harness-view/harness suites with SMB-LR-HEALING-2 (disjoint files; later pusher rebases and re-runs once).
- V4-14: FIG-EDIT-1 (harness-view design*) and the READY record lanes in folio/package/session are unjudged: the lead pins their round candidate at their READY shas or lists these commits.
- Design records themselves live in the import receipt (FIG-EDIT-1 reading); if validating the state needs them and the receipt is absent (package written elsewhere), validate against the document only and say so in the state (`design_records_absent`), never guess.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
