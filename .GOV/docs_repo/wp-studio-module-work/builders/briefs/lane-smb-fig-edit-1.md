---
file_id: BUILDER-BRIEF-LANE-SMB-FIG-EDIT-1
file_kind: builder_brief
updated_at: 2026-10-24T04:23:07Z
wp: WP-KERNEL-STUDIO
lane: SMB-FIG-EDIT-1
lane_file: builders/lanes/hsk-studio-fig-edit-1.json
crates: [hsk-studio-harness-view, hsk-studio-harness (RUN-UI.md only)]
model: opus
effort: high
author_role: lane planner brief (read-only planning, origin tip b9c311e5); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a/4b, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, rows, expected gain, edge answer, model">

## Target

- App: Figma. GUI pooled 28.6 % (2 of 7 workflows); top gap "design editing UI absent". Audit workflow "Open a .fig file": "ENGINE_ONLY ... import::sniff has no .fig route"; ".fig opens only into the prototype player" (parity-20261023-figma-affinity-shell.json). The engine import is VALIDATED (FG-23 ".fig file import into an editable native document", `interop-fig-native/src/native.rs:3177 to_native`).
- First spec-backed vertical slice: (A) File > Open turns a `.fig` into an editable native document in the window (layers, artboards, vector geometry and paints editable with the existing vector, layers and properties surfaces; text in the text entry; design records carried); (B) the instance inspector: select a component instance, switch its variant properties, edit and reset its overrides (STU-DS-008/016/017), bound contextually in the Properties panel (STU-DS-160).
- Expected gain: workflows "Open a .fig file" and "Components, variants, instances, overrides" become doable: Figma GUI 28.6 -> 42.9 % unjudged (57.1 % judged); 108-workflow pool +1.0 point unjudged, +2 judged. Engine unchanged.
- Edge answer: NO new crate edge. `hsk-studio-harness-view -> hsk-studio-interop-fig` is already declared and used (`view/prototype.rs:49-51` uses `import_fig`, `decode_document`, `SequentialIdMint`), and harness-view already writes an imported vector document as a native package with the `vector` and `text` session entries (`.ai` precedent `view/vector/interop.rs:1-12, :403 import_ai`). An edge `harness-core -> interop-fig` would also be acyclic (interop-fig's closure is accord, folio, components, layout, type, nib, observe, package, press, press-read, press-write, interop-raster, interop-fig-base, interop-fig-native: root Cargo.toml `workspace.metadata.studio.dependencies` :149-245; none lists harness-core, harness-view or harness), but it is not needed and is not requested.
- Model / effort: opus / high (import mapping into the vector/text entries, the instance inspector over components resolution, contextual binding, shell routing).

</topic>

<topic id="paths" status="active" summary="Owned paths, shared hunks, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-fig-edit-1`)

- NEW `crates/hsk-studio-harness-view/src/view/design.rs` and `src/view/design/**` (suggested `fig_import.rs`, `instance.rs`), NEW `tests/viewer_fig_edit.rs`.
- Additive hunks: `src/view/mod.rs` (`pub mod design;`, the Properties contextual section call inside `properties_ui` (:1032), `state_json` key `design`), `src/view/shell.rs` (`run_dialog_purpose` Purpose::Open: a `.fig` (interop-fig container prelude `fig-kiwi`, `interop-fig-base/src/container.rs:331 open_fig`) routes to `import_fig` instead of harness-core import; nothing else), `src/view/file_dialog.rs` (the Open filter list gains "Design file (.fig)"; the OpenPrototype filter is unchanged), `src/view/vector.rs` only if a `pub(crate)` accessor is needed for the entry writer (one hunk, recorded), `crates/hsk-studio-harness/RUN-UI.md` (one section).

## Do NOT touch

- SMB-PR-RATE-STRETCH-1 files: `src/view/timeline.rs`, `src/view/timeline_ui.rs`, `src/view/timeline/**`, `tests/viewer_timeline*.rs`.
- SMB-LR-HEALING-1 files: `src/view/develop_entry.rs`, all develop crates.
- `src/view/prototype.rs` (call it), `src/view/hosted.rs`, `src/view/proof*.rs`, `src/view/text.rs` (use its pub entry types), every other crate (`hsk-studio-interop-fig*`, `hsk-studio-components`, folio, session, package: if a needed capability is missing there, stop and report), root files, `Cargo.toml` of harness-view (no feature change needed: `viewer` already enables `dep:hsk-studio-interop-fig` and `dep:hsk-studio-components`).

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-IO-006]` matrix row 35 (grep "| 35 | Figma design document"): "| 35 | Figma design document (local copy) | `.fig` | Figma family | I·EP·X·RT | Frames, components, variants, variables, auto-layout, prototype flows transformed; **local-copy** round-trip; cloud sync = ADPT | NRT |"; token legend: "**I** = import, **EP** = edit-preserve, **X** = export, **RT** = round-trip ... **transformed** (converted to the nearest Studio primitive with a documented mapping)".
- `[STU-IO-005]`: "Unsupported-feature handling MUST be lossless-of-intent where the format is a native round-trip target:" (losses go to the import receipt, never silently).
- `[STU-DS-003]`: "`StudioComponentInstance` ... is a live reference to a `StudioComponent`. An instance renders the current component definition plus its own local overrides ([STU-DS-016]); it is not a copy."
- `[STU-DS-008]`: "An instance of a variant-set component exposes its variant properties as switchable fields; setting a variant property re-selects the member definition in place while preserving compatible overrides ([STU-DS-017])."
- `[STU-DS-009]`: "A `StudioComponent` MAY declare named component properties that surface on every instance's inspector as typed fields".
- `[STU-DS-016]`: "Overridable surfaces are: text content, fills/strokes/paint, effects, visibility, nested instance swaps, and exposed-nested property values."
- `[STU-DS-017]`: "Overrides MUST persist across variant switches and component swaps when the target structure matches by layer name and hierarchy, and MUST be resettable both per-property and wholesale (reset-all-overrides)."
- `[STU-DS-160]`: "Every design-system inspector panel named in this sub-section — ... the instance inspector ... — MUST declare its binding through the contextual property-panel contract ([STU-WEB-030] through [STU-WEB-044]) rather than through a hard-coded panel switch. Studio MUST NOT introduce a whole-UI persona or mode toggle".
- `[STU-DS-162]` command-surface obligation (typed, model-invokable) and `[STU-SHL-008]` naming law (no "Figma" in labels; F-R37-3 "Design file (.fig)").

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- Lane files `builders/lanes/hsk-studio-interop-fig-native.json`, `hsk-studio-interop-fig-instances-frames.json`, `hsk-studio-harness-prototype-play.json`, `hsk-studio-harness-shell-2.json` (dialog purposes, F-R37-3), `hsk-studio-harness-panels-1.json` (VectorImport hosted action = `import_ai`).
- Code: `view/vector/interop.rs:203-300` (proposal -> `VectorEntry` paths/design/records/icc/fonts + `text` entry -> `write_package_with` -> open), `:403 import_ai(source, dest, overwrite)`; `view/vector.rs:56 ENTRY`, `:129 PathRecord`, `:305 VectorEntry`; `view/prototype.rs:520-540` (`.fig` decode with limits and cancel); `interop-fig-native/src/native.rs:255 NativeProposal` (snapshot, design, stories, paths, type_styles, raster_records, vector_records, ...), `:3177 to_native`; components `src/instance_resolution.rs:565 resolve_instance`, `:711 rematch_overrides`, `:746 swap_component`, `:763 switch_variant`, `:837 propose_overrides`; `view/mod.rs:956, :1032 properties_ui`; `view/shell.rs:1194-1215 finish_dialog / run_dialog_purpose`.

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. `.fig` -> native package -> open (I·EP): `Viewer::import_fig(source, dest, overwrite)` = `open_fig` + `import_fig` + `decode_document` + `to_native`; paths -> `PathRecord`s (document px), design/vector/raster records into the `vector` entry, stories and type styles into the `text` entry, raster records as the package Records import entry, original + receipt with every loss (`NodeLoss` list, unresolved text styles) per STU-IO-005; File > Open routes `.fig` here (dest = the source stem `.handshake` beside it, refusal without overwrite, as `import_ai`). Tests: `opening_a_fig_yields_an_editable_native_document`, `fig_layers_and_paths_edit_through_the_vector_surface_and_undo`, `fig_import_receipt_lists_losses`, `hostile_fig_bytes_are_refused_bounded`, `prototype_player_still_opens_fig_files`.
2. Instance inspector (contextual, STU-DS-160): when the selection is an instance, the Properties panel shows a design-instance section (no new panel, no mode toggle): main component name, variant properties as switchable fields (`switch_variant`), component properties as typed fields (STU-DS-009), overrides list with per-property reset and reset-all (STU-DS-017, `rematch_overrides`/`propose_overrides`), component swap (`swap_component`); each change writes the `vector` entry's design records as one undo step; typed `Viewer::design_command(DesignCommand)`. Tests: `instance_section_appears_only_for_instances`, `switching_a_variant_keeps_compatible_overrides`, `reset_override_and_reset_all`, `component_swap_rematches_by_name_and_hierarchy`, `accesskit_drives_the_instance_section`.
3. Inspection: `state_json()["design"]` = selected instance, component, variant assignment, overrides, last command result. Test: `state_json_reports_instance_and_overrides`.

</topic>

<topic id="tests" status="active" summary="Test shape and the bundled lane-check">

## Test shape

- Headless egui + AccessKit (pattern of `tests/viewer_timeline.rs`; settle the layout before pointer events, F-TL-2). Fixtures: the checked-in `.fig` test files the interop-fig crates already use (find them via the interop-fig tests; never machine-local paths, V4-03); Operator fixtures absent -> `NOT_PROVEN:`.
- NOT_PROVEN: equality with Figma's instance panel behaviour (no Figma oracle); auto layout, constraints, variables/modes editors (later slices of the design-editing gap); `.fig` export (X/RT) from the window.

## Bundled lane-check (C: SSD, rule 4a)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-fig-edit-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-fig-edit-1 --disk ssd -- cargo nextest run -p hsk-studio-harness-view -p hsk-studio-harness --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: harness-view, harness (RUN-UI.md; its shell suites walk File > Open and the dialog filters). Rule 4b: no Folio field: package suite not required. Loop cap: two failed runs on the same failure, then stop and report verbatim.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, conflicts, deadline">

## Edges and dependencies

- None (see target: existing `harness-view -> interop-fig` and `harness-view -> components`). No crates.io dependency.

## UNVERIFIED (D5)

- Figma instance inspector layout, override reset semantics beyond STU-DS-017, variant field ordering. No Figma oracle; Operator `.fig` fixtures pending.

## Risks and conflicts

- Shared harness-view with SMB-PR-RATE-STRETCH-1 and SMB-LR-HEALING-1: disjoint files as listed; all three run the harness-view and harness suites; the second and third to push rebase and re-run once.
- `shell.rs` and `file_dialog.rs` were last edited by SMB-HARNESS-SHELL-2 and SOFT-PROOF-2 (READY): additive hunks only. V4-14: the lead pins R40's candidate for those READY lanes or lists these commits.
- If the `vector` entry cannot carry every `NativeProposal` record (for example type styles or image paints), the loss goes to the receipt and the lane notes; no new session token in this lane (session is not owned).

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
