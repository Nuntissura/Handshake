---
file_id: BUILDER-BRIEF-LANE-SMB-PS-INFO-HISTOGRAM-1
file_kind: builder_brief
updated_at: 2026-10-24T08:30:00Z
wp: WP-KERNEL-STUDIO
lane: SMB-PS-INFO-HISTOGRAM-1
lane_file: builders/lanes/hsk-studio-ps-info-histogram-1.json
crates: [hsk-studio-pigment (new file + src/lib.rs lines), hsk-studio-harness-view, hsk-studio-session (src/source.rs token only), hsk-studio-harness (RUN-UI.md only)]
model: opus
effort: medium
author_role: lane planner brief (read-only planning, origin tip bfb5843c); no product code written
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, loop cap, lane worktree), builders/DISPATCH_TEMPLATES.md topics builder + v4-rules, builders/WORKSPACE_RULES.md
---

<topic id="target" status="active" summary="App, row, expected gain, model">

## Target

- App: Photoshop. Row PS-30 "Histogram, info and colour sampler diagnostics" is MISSING (audit `parity-photoshop.json`: "Grep [Hh]istogram over pigment, prism, harness-view, develop: only an EqualizeHistogram refusal (adjust_colour.rs:163); no info/sampler code"; matrix_ui false), rank 2 of the Photoshop gaps.
- Plus the check the lead asked for: does the right SidePanel (Properties / Develop, `view/mod.rs:968-990`) narrow like the Layers panel did? SMB-SHELL-ROW-WRAP-1 notes: "Right properties/develop SidePanel not touched (develop lane's rows; may narrow the same way when no wide fixed row is present: UNVERIFIED)".
- Expected gain: PS-30 MISSING -> BUILT_NOT_VALIDATED (+1.4 pp Photoshop engine, 36 rows; +2.8 pp validated) with the numbers computed by pigment and the surface in the window; strict UI row PS-30 doable (+2.8 pp Photoshop GUI strict). No pooled workflow.
- Model / effort: opus / medium (pure computation over planes plus one panel; the narrowing check is a measured test).

</topic>

<topic id="paths" status="active" summary="Owned paths, files not to touch">

## Owned paths (lane worktree `wt-studio-lanes/smb-ps-info-histogram-1`)

- Pigment: NEW `crates/hsk-studio-pigment/src/diagnostics.rs` (histogram per composite and per channel with bin and clip counts; averaged eyedropper samples for the seven sizes; colour-model readouts through prism where a profile is involved), `src/lib.rs` `mod`/`pub use` lines, NEW `tests/diagnostics.rs`.
- Harness-view: NEW `src/view/info.rs` (+ `src/view/info/**`), additive hunks in `src/view/shell.rs` (`Panel::Info` variant, `ALL` 24 -> 25 after `Panel::Properties`, key `info`, label "Info", `key_code`/`shortcut_text`/`modifiers` Ctrl+Shift+F4 (free; Studio choice: no binding declared for `info`), visibility/availability/open/close arms), `src/view/mod.rs` (`pub mod info;`, Viewer field, panel draw, canvas hover readout hook (pointer position + selection bounds), sampler placement in `canvas_ui` as a typed action, `state_json` key `info`; the right-panel narrowing fix if the check finds it), NEW `tests/viewer_info.rs`, NEW `tests/viewer_side_panels.rs` (the narrowing check). Reuse `src/view/wrap.rs` (`wrap_item`, `wrap_measured`) for every wrapping row; edit `wrap.rs` only to fix a helper defect (record it).
- `src/view/develop.rs`: only if the narrowing check shows a Develop row cause (additive `wrap_item` adoption at the measured site).
- Session: `crates/hsk-studio-session/src/source.rs` add `"info"` to `DOCUMENT_ENTRY_TOKENS` (persistent sample points; JSON validated in harness-view on set and read, the `proof.rs` pattern).
- Harness: `crates/hsk-studio-harness/RUN-UI.md` (Info panel section).

## Do NOT touch

- `hsk-studio-type`, `hsk-studio-composite`, folio (`vector.rs`, `vector_path_text.rs`: SMB-AI-AREA-TYPE-1; `lib.rs` and new files: SMB-ID-CJK-KINSOKU-1), `hsk-studio-layout`, package, harness-view `view/vector/**` (AI-AREA-TYPE-1 may touch `vector/path_text.rs`), `view/timeline*`, `view/design*`, `view/proof.rs` (read only).

</topic>

<topic id="spec-basis" status="active" summary="Clauses, grep strings, quoted text">

## spec_basis (v02.222 `spec-modules/14-studio-creative-suite.md`; grep only)

- `[STU-RAS-049]`: "Studio MUST provide the raster tonal-diagnostics feedback surface: a live histogram (per-composite and per-channel, with clip warnings), a persistent multi-point colour sampler, an eyedropper sampling into the active swatch with sample-size and sample-layer scopes, and an info/readout of pixel values, position and dimensions under the pointer or over a selection."
- `[STU-RAS-163]`: "The colour sampler MUST support at least four simultaneous persistent sample points, each storing a document-space position and reading live values in a selectable colour model. The eyedropper's sample size is a closed enumeration of averaged square neighbourhoods — `point`, `3x3`, `5x5`, `11x11`, `31x31`, `51x51`, `101x101` — and its sample scope is `current_layer`, `current_and_below`, or `all_layers`. The histogram MUST expose its underlying bin counts and its clip counts to the model command surface as structured data, not only as a picture, because a no-context model verifying an exposure edit needs the numbers ([STU-VAL-004]). Measurement and count tooling reads pixel length in `pixels` with a document measurement scale."
- Panel registry (grep "| Info | `info`"): "| Info | `info` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (2); illustrator (2); indesign (2); premiere (2); affinity (1) |"; spine panel list (grep "`history`, `info`"): "`context-bar`, `tools`, `properties`, `layers`, `colour`, `swatches`, `history`, `info`".
- `[STU-SHL-091]`-style sizing is not declared for `info`; `[STU-SHL-008]` naming law applies (label "Info").

</topic>

<topic id="predecessors" status="active" summary="What exists">

## Predecessors to read (bounded)

- `builders/lanes/hsk-studio-shell-row-wrap-1.json` (wrap helpers, the measured toolbar/Layers sizes, `tests/viewer_shell_rows.rs` method: PanelState rects over 20 frames after 3 warm-up frames, `not_fixed` list), `hsk-studio-harness-shell-2.json` (panel registration), `hsk-studio-ps-soft-proof-2.json` (`view/proof.rs`: document-entry read/validate pattern, canvas read-back for display data).
- Code: harness-view `view/wrap.rs:15 wrap_item`, `:30 wrap_measured`; `view/mod.rs:910-990` (left/right SidePanels), `:1044 properties_ui`, canvas `canvas_ui` (pointer -> document pixels `to_doc`); `view/shell.rs:104 Panel::ALL`; pigment `src/lib.rs` (RgbaPlane types), develop `src/levels.rs` (a plateau/histogram helper for levels: reuse its binning if it fits, do not copy).

</topic>

<topic id="slices" status="active" summary="Minimal vertical slices in order, each with tests">

## Slices (commit locally per slice; one bundled run per push)

1. Pigment diagnostics (engine): `histogram(plane, channels, bins) -> Histogram { bins per channel + composite (luminosity, declared weights), clip_low, clip_high, count }`, `sample(plane, x, y, size) -> averaged value` for the seven sizes (edge handling: average of the in-bounds pixels, declared), `selection_stats(plane, coverage)`; deterministic, bounded (V4-01/02). Tests: `histogram_bins_and_clip_counts_are_exact`, `sample_sizes_average_their_neighbourhoods`, `edge_samples_average_in_bounds_pixels`, `hostile_planes_are_refused_bounded`.
2. Info panel (WORKSPACE > Panels > Info, Ctrl+Shift+F4): readout of pixel values under the pointer (document RGB and one selectable model: RGB / HSB / Lab / CMYK through prism, declared set), position, document dimensions, selection bounds; live histogram (composite and per channel, clip warnings) of the displayed composite; every number in `state_json()["info"]`. Tests (headless egui + AccessKit, settle layout before pointer events, F-TL-2): `info_opens_from_the_workspace_menu_and_shortcut`, `hovering_the_canvas_reads_the_pixel_under_the_pointer`, `histogram_numbers_are_in_state_json`, `histogram_updates_after_an_edit`.
3. Colour sampler and eyedropper: at least four persistent sample points placed on the canvas (typed `Viewer::info_command(InfoCommand::{AddSampler, MoveSampler, RemoveSampler, SetSampleSize, SetSampleScope, SetModel})`), live values per point, the eyedropper sampling into the active swatch with size and scope (`current_layer` / `current_and_below` / `all_layers`); sample points persist in the `info` document entry (save/reopen). Tests: `four_samplers_read_live_values`, `eyedropper_scope_reads_the_right_layers`, `samplers_survive_save_and_reopen`.
4. Right SidePanel narrowing check: measure the Properties and Develop panel widths over 20 frames after 3 warm-up frames with only narrow rows present (method of `viewer_shell_rows.rs`); if the width shrinks or grows frame over frame, fix the causing rows with `wrap_item`/`wrap_measured` and keep the panel at its default or dragged width (as the Layers panel fix did). Tests: `properties_panel_width_is_stable`, `develop_panel_width_is_stable`, `info_panel_width_is_stable`. If no defect reproduces, the tests still land as the proof and the lane notes say "checked, not reproduced".

</topic>

<topic id="tests" status="active" summary="Test shape, rules 4a-4e, bundled lane-check">

## Test shape

- Pigment tests exact on synthetic planes; harness-view headless tests compare panel numbers with pigment's own results on the rendered composite.
- NOT_PROVEN (`NOT_PROVEN:` lines): equality with Photoshop's Histogram/Info panels (cache levels, luminosity weights, CMYK readout profile; no D5 case for PS-30); real desktop window.

## Round-parity rules (common 4c-4e, stated for this lane)

- 4c: `lane-check.sh` runs the round's step-0 policy first; exit 71/72 = the round would refuse; no drive-letter or user-profile literals; `--step0-only` for a quick scan.
- 4d: doc examples added or changed in pigment or harness-view -> `cargo test --doc -p <crate>` through studio-env.
- 4e: new harness-view test files start `#![cfg(feature = "viewer")]` like the others; `.config/round-features.toml` has `hsk-studio-harness-view = [["viewer"], ["desktop", "audio-batch"]]`, so they count; an AccessKit test needing `desktop` is covered by the second unit. Pigment tests are default-feature.

## Bundled lane-check (D: while R40 holds C:; rule 4a)

```bash
cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-lanes/smb-ps-info-histogram-1/handshake-studio-assets" && bash "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/tools/lane-check.sh" --owner smb-ps-info-histogram-1 --disk d -- cargo nextest run -p hsk-studio-pigment -p hsk-studio-session -p hsk-studio-harness-view -p hsk-studio-harness --features hsk-studio-harness/desktop,hsk-studio-harness/audio-batch --no-fail-fast
```

- Rule 4a: pigment, session (token list), harness-view, harness (RUN-UI.md; shell suites walk `Panel::ALL`). Rule 4b: no Folio field: package suite not required. Loop cap: two failed runs on the same failure, then stop and report verbatim. On C: later: `--disk ssd` with `CARGO_BUILD_JOBS=6`.

</topic>

<topic id="risks" status="active" summary="Edges, deps, unverified, conflicts, deadline">

## Edges and dependencies

- None: pigment -> prism, harness-view -> pigment/prism/session declared. No crates.io dependency.

## UNVERIFIED (D5)

- Photoshop histogram luminosity weights and bin count, Info panel colour-model readouts, sampler limit (Photoshop allows more than four; the spec floor is four), eyedropper edge handling.

## Risks and conflicts

- `view/mod.rs` and `shell.rs` are free tonight (SHELL-ROW-WRAP-1 READY): additive hunks only; V4-14 for READY harness-view lanes in R40 (lead pins or lists).
- Pigment `lib.rs` carries READY-lane lines (roto, content-aware): additive only.
- AI-AREA-TYPE-1 may edit harness-view `view/vector/path_text.rs`; disjoint from this lane; both run the harness-view suites, later pusher rebases and re-runs once.
- The histogram must be computed on the displayed composite without blocking the frame: compute on change (revision) and cache, bounded.

## Deadline rule

- First pushed commit within 30 minutes, then one at least every 30 minutes, each after its green bundled run; while a run is queued or running at a deadline, commit locally and write a lane-file heartbeat (`phase`, local shas, run status, `date -u`).

</topic>
