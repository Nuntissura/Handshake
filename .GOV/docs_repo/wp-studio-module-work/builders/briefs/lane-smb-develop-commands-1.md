---
file_id: LANE-SMB-DEVELOP-COMMANDS-1
file_kind: lane_brief
updated_at: 2026-11-08T08:51:44Z
wp: WP-KERNEL-STUDIO
lane: SMB-DEVELOP-COMMANDS-1
lane_file: builders/lanes/hsk-studio-develop-commands-1.json
crates: hsk-studio-harness-view (develop typed rows for every DevelopOp and FamilyOp, and the develop must-haves: group enables, point curves, snapshots, preset save, crop and straighten, over hsk-studio-deve
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Every develop operation the Viewer has is a typed command, and group enables, point curves, snapshots, preset save and crop are new ops, each one undo step, saved in the develop entry">

## Target

## Lead rulings 2026-11-08 (binding)

Source: S/research/width-plan-20261108-raster-vector-design.json. Every open question below takes the planner's recommended default as a lead ruling. Its code facts were checked at 0a733db8; this lane's base is eb17fad6. Re-read every file:line at your base before you edit it.

- Group enables: STU-RAW-105 makes them state, the panel derives them (develop_more.rs:144, :179). Override, or replace the derivation? -> An optional override per flag, derived when absent: existing documents render unchanged and the panel keeps working.
- Row ids: photo.develop.<op> for the new rows, or a develop.* domain? -> photo.develop.<op>, the existing pattern (photo.rs:27-42); no new domain file.
- Crop and straighten: the tile API refuses angle and flips (recipe.rs:86-89) while the viewer re-develops by tiles. Apply crop in the viewer, or store it and apply at develop export only? -> Store crop edges, angle and flips, apply them at develop export and in the DNG XMP, and show the uncropped preview with a note crop_preview_not_applied; a cropped preview is a GUI-lane item. If the lead rejects a stored-but-not-previewed crop, cut item 3d.
- save_preset writes a file: is it an output row (output_path shape, no-clobber) like export rows? -> Yes: output_path shape (harness-core commands.rs output_path), refused over the opened package and without overwrite.

- Process: red first per item, then focused runs on D:. Stop at REVIEW READY for the read-only code-check and red-team review, then run ONE bundled run, then stop at BUNDLED GREEN. Lanes land one at a time on the lead's RELEASED (HANDOFF topic workflow, items 10, 11 and 16).
- Any lane that changes a Viewer command's refusal order also runs every harness process test that drives that command. This is the lesson from the 2026-11-08 trunk break at 35c50d3b.

## Target

Width plan 20261108 rank 2. Eight photo-retouch develop steps (27, 28, 29, 30, 40, 41, 46, 51) work in a Rust test and not from a command: `Viewer::develop_op` (`harness-view/src/view/develop.rs:1001`) takes every `DevelopOp` (:408), but the typed rows (`harness-core/src/action/commands/photo.rs:27-42`, decoders `harness-view/src/view/commands/photo.rs:77-99`) cover Kelvin through RemoveDistraction only. Missing rows: `AddColourRange`, `AddCorrection(MaskSource)`, `AddMask`, and every `Family(FamilyOp)` member (`develop_more.rs:166`: LensProfile, ClearLensProfile, LensToggles, Upright, UprightGuides, GrainSeed, VignetteStyle, Look, ClearLook, Preset). Five more steps (31, 35, 48, 49, 50) have engine code and no op at all: crop and straighten (`develop/src/recipe.rs:91` `NormalizedCrop`, field :192), point curves (`ResolvedCurves` :68, field :189, `Curve::new` `develop/src/curves.rs:23`), group enables (`Enables` `recipe.rs:23`, derived today by `develop_more.rs:144` `family_enables`, comment :179), snapshots (`develop/src/preset.rs:76` `Snapshot`) and preset save (`DevelopPreset::capture` :348, `to_text` :406). All file:line below are at 0a733db8 (origin tip, read with git show / git grep in wt-studio-lanes/smb-pr-sequence-session-1); paths are under `handshake-studio-assets/crates/hsk-studio-`. Spec lines are module `14-studio-creative-suite.md` of v02.229.

Clauses: STU-RAW-004 :2126, STU-RAW-005 :2139, STU-RAW-009 :2166, STU-RAW-010 :2172, STU-RAW-011 :2177, STU-RAW-014a :2535, STU-RAW-014b :2543, STU-RAW-015 :2616, STU-RAW-018 :2710 (every develop group, mask source and profile is a command), STU-RAW-105 :2086, STU-RAW-113 :2267, STU-RAW-121 :2549, STU-RAW-126 :2481, STU-RAW-127 :2493, STU-RAW-128 :2522, STU-RAW-131 :2679, STU-AUT-147 :21905, STU-CON-007 :22896.

1. **Red first.** New `harness/tests/viewer_develop_commands.rs` (feature viewer), shaped like `viewer_develop_lens.rs` and `viewer_develop_3.rs` (corpus DNG). For each new row: refused `unknown_command` at red; at green, the row's tiles digest and `develop_json` (`develop.rs:941`) equal a direct `develop_op` call with the same op (differential, so the decoder cannot drift from the op). One process test through `command --stdin` running steps 27-30, 40, 41, 46 and 51 in order. Prove red by running.
2. **Typed rows for the existing ops** (ids under `photo.develop.`, no new domain): `add_colour_range {samples, breadth}`, `add_correction {source: linear|radial|brush, ...fields}` from `default_linear` / `default_radial` / `default_brush` (`develop.rs:630`, :635, :640) with optional field values, `add_mask {correction, source, subtract}`, `lens_profile {path, focal_mm, aperture_value?, distance_m?}`, `clear_lens_profile`, `lens_toggles {profile_enable, auto_ca}`, `upright {mode}` (`develop/src/geometry.rs:36` `UprightMode` keys), `upright_guides {guides}`, `grain_seed {seed}`, `vignette_style {style}` (`develop/src/effects.rs:53`), `look {path, name, domain}` (`develop/src/look.rs:18` `LookDomain`), `clear_look`, `apply_preset {path, amount}`, plus `set {control, value}` and `set_many {changes}` over `set_develop` / `set_develop_many` (`develop.rs:973`, :982) so step 32 is one undo step and the GUI binds one id. File parameters use the `path` shape and are read by the decoder under a named byte cap before parsing (Look `.cube`, preset, lens profile); a missing or oversized file is a typed refusal. Each mutating row is ONE chronicle tiles step, as `develop_op` is today (harness DESCRIPTOR develop.persistence).
3. **New ops for the must-haves** (each a `DevelopOp` variant, a typed row and one undo step):
   - `set_enable {group, on}` (STU-RAW-105): an explicit operator override per `Enables` flag (12, `recipe.rs:23-36`); absent override = today's derived value, so existing documents render byte-identically. `enable_red_eye` stays refused (`recipe.rs:60-62` `first_unsupported`).
   - `set_point_curve {channel: composite|red|green|blue, points}` (STU-RAW-113): through `Curve::new` validation, stored in `DevelopRecipe.curves`; `enable_tone_curve` follows unless overridden.
   - `save_preset {path, name, keys}` (STU-RAW-128): `DevelopPreset::capture` + `to_text`, written no-clobber through the export writer (`view/export.rs` `write_new`), `overwrite` flag as other output rows; `snapshot.save {name}`, `snapshot.apply {name}`, `snapshot.delete {name}` with the list in the develop entry (`Snapshot` fields NOT_INSPECTED: read preset.rs:76-110 first).
   - `set_crop {left, top, right, bottom, angle, flip_horizontal, flip_vertical}` (STU-RAW-015, STU-RAW-131): into `NormalizedCrop`. Risk read in code: `recipe.rs:86-89` says angle and flips run only in the whole-frame stage and the tile API refuses them (`unsupported_contribution:transform_whole_frame`), while the viewer re-develops through tiles. See open question 3; this item is cut first if the lane passes 7 commits.
4. **Persistence.** `develop_entry.rs` (`hsk.studio.harness.develop@1`, :32; header :5 says every `Adjustments` field is written, exact keys) gains optional members `enables_override`, `point_curves`, `crop`, `snapshots`, absent when default, so every existing entry and package stays byte-identical. The hostile decoder bounds each: points per curve, snapshots per document, name length, finite numbers, the 8 MiB entry cap it already has.
5. **Failure-class sweep (V4-06): every reader of the develop recipe that would ignore a new member.** List each, fixed or not applicable: develop export (`export_develop`), CLI `develop --adjust` (`harness/src/cli.rs:803`), the document export recipe's develop stage (`view/doc_export.rs:3-4`), DNG XMP write (`set_develop_dng_xmp`, scenario evidence develop.rs:913, re-read it), `develop_json` and `state_json` projections, the develop panel draw code (read only: no egui line in this lane), batch through `cli batch`. A member a reader cannot honour is a typed refusal or a named note, never silently dropped.
6. **Hostile input (V4-01/02).** Non-finite numbers, 10^6 guides, a 2 GiB `.cube` path, a preset naming unknown keys, a crop with left > right, a curve with unsorted or duplicate x, 10^5 snapshots: typed refusal, no abort, no allocation before the count check.

Out of scope: catalog, ratings and sync of settings (needs multi-image selection, STU-RAW-017), before/after view, clipping overlay, white balance picker by click, Auto Tone (no engine function found: git grep auto_tone over crates/*/src finds only an unrelated folio field, scenario step 25), camera profile browser (step 26: the profile model of STU-RAW-125 NOT_INSPECTED), red-eye, HDR merge, AI masks, any panel or egui line, any Cargo.toml change, any Folio change, any MT or Master Spec edit. Lightroom and Camera Raw parity UNVERIFIED (D5).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: develop Refusal {code, detail} through CommandRefusal (view/commands/photo.rs maps it today); no new observe API", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: develop_op commits one chronicle tiles step with the develop entry through the viewer run context; no default_actor() or literal actor in new code", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-folio-angle-unbounded-1`, branch `lane/smb-develop-commands-1`, base eb17fad6)

- harness-core: `src/action/commands/photo.rs` (rows only).
- harness-view: `src/view/commands/photo.rs` (decoders), `src/view/develop.rs` (DevelopOp variants, develop_op arms), `src/view/develop_more.rs` (FamilyOp only if a variant is missing a decode form), `src/view/develop_entry.rs` (optional members, hostile decoder).
- harness: new `tests/viewer_develop_commands.rs`.
- develop: only if a test proves an engine defect.
- Not touched: view/place.rs, view/vector*, view/design*, any panel draw code.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-folio-angle-unbounded-1 --disk d`): step 0; clippy over harness-core, harness-view, harness; nextest WHOLE suites of hsk-studio-harness-core and hsk-studio-harness-view with [viewer]; hsk-studio-harness viewer_develop* (all eight files), viewer_doc_export* if present, the new file, and the drive tests; rule 4e for units; doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-folio-angle-unbounded-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
