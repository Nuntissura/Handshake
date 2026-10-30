---
file_id: LANE-SMB-PS-GRADIENT-BUCKET-1
file_kind: lane_brief
updated_at: 2026-10-30T13:47:06Z
wp: WP-KERNEL-STUDIO
lane: SMB-PS-GRADIENT-BUCKET-1
lane_file: builders/lanes/hsk-studio-ps-gradient-bucket-1.json
crates: [hsk-studio-pigment, hsk-studio-harness-view (edit.rs, mod.rs tool dispatch, new view/fill_tools.rs), hsk-studio-harness (new viewer test only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (rules 4a-4e, PUSH SAFETY, RUN LOGS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Photoshop raster Gradient tool and Paint Bucket (flood fill) on pixel layers">

## Target

Gap source: `research/parity-20261023-adobe-print.json`, key `apps.photoshop.top_gaps_remaining` rank 5, row `PS-19` ("No raster gradient tool or bucket fill"), and the matching `apps.photoshop.changed_rows` entry for `PS-19` ("gradient/pattern render as fill layers (READY); still no raster gradient tool or bucket; PARTIAL").

Still open at d208bc0e (checked by planner, read-only):
- `hsk-studio-pigment` has the pieces but no tool: the magic-wand flood (`src/select_tools/wand.rs`, `magic_wand`), the five-style gradient maths of the gradient overlay (`src/effect_math/style.rs`, `GradientStyle`, `gradient_at`), the brush coverage path and `masked_replace` (`src/masked_patch.rs`).
- The viewer's `Tool::PaintBucket` (`harness-view src/view/edit.rs:67`) is the Illustrator live-paint bucket on vector networks (`src/view/vector.rs:2270`, `network_paint_bucket`). No raster gradient tool and no raster flood fill exist in the window.
- No lane in `builders/lanes/` covers either tool; SMB-PS-FILL-LAYERS covers fill-layer rendering only.

Close the Photoshop workflow "pick the Gradient tool, drag across a layer or selection" and "pick the Paint Bucket, click a region", in this order, with a local commit per slice:

1. **Engine: raster gradient op** (pigment, new `src/fill_tools.rs`, or a `fill_tools/` folder if it grows).
   - Input: a drag line (start and end in document pixels), geometry `linear | radial | angle | reflected | diamond`, a colour stop list and an INDEPENDENT transparency stop list (STU-RAS-160 forbids folding opacity into the colour stop), stop midpoints, `reverse`, `dither`, blend mode and opacity.
   - Output: replacement tile bodies plus a coverage plane clipped to the current selection, published through the existing `masked_replace` path (the same route the brush uses). Never write document state from pigment.
   - Reuse the overlay's stop and midpoint interpolation and its blend modes. Expose a `pub(crate)` helper if needed; do not copy the maths (V4-05 frozen-duplicate sites).
   - `noise` form gradients and `shape_burst` are declared gaps with a typed refusal. Do not drop them silently.
   - Dither, if implemented, must be seeded and deterministic (STU-CON-007 (c), cited by STU-RAS-160). Otherwise refuse `dither=true` with a typed code.
2. **Engine: flood fill op** (same module).
   - Seed point, `tolerance`, `contiguous` (default true, STU-RAS-119), `anti_alias` (default true, STU-RAS-119), sample the active layer or a caller-supplied merged frame. The region comes from the existing `magic_wand`, not a second flood.
   - Content: a colour, or a pattern tile given as pixels by the caller (STU-RAS-161; the id is authoritative and the name is a label). Blend mode and opacity as in slice 1.
   - Add an "Edit > Fill" form, a fill of the current selection with no flood (STU-RAS-025 "foreground/background fill"), in the same op family.
3. **Viewer** (harness-view).
   - Add two raster tools to `Tool`, with keys taken from the STU-SHL-155 catalogue: `gradient` and `flood_fill`. Leave the existing vector `paint_bucket` key and behaviour untouched.
   - Add headless `Viewer` methods first, in new `src/view/fill_tools.rs`: `fill_gradient(layer, from, to, opts)`, `flood_fill(layer, x, y, opts)` and `fill_selection(layer, opts)`. Each is ONE undoable chronicle `tiles` edit, staged the way `paint` and `liquify` stage tiles (`edit.rs` `paint`, `select_liquify.rs` `liquify`). They go in the operation log.
   - The egui tool only calls these methods: a drag for the gradient, a click for the bucket. Add an option row for geometry, mode, opacity, tolerance, contiguous, anti-alias and sample merged, plus a hover string.
   - The foreground colour is the brush colour (Studio reading, state it). The default gradient is foreground to transparent.
   - On a non-pixel layer (fill, placed asset, vector, text), return a typed refusal naming the layer kind (STU-RAS-170, STU-RAS-117). Never rasterize implicitly.
4. **Pattern source in the window, only if cheap.** If the document already carries a resolvable pattern tile (a fill-layer `pattern_tile` or a pattern-overlay tile), offer it to the bucket. Otherwise the pattern choice is absent from the GUI and the headless method refuses `pattern_unavailable`. Do not invent a pattern library: STU-RAS-167 is a separate registry.

Photoshop behaviour is UNVERIFIED (D5): tolerance metric, anti-alias kernel, dither pattern, midpoint curve, and the radial, angle and diamond mapping from a drag line. State each one as a Studio reading in the module docs and the `DESCRIPTOR`.

</topic>

<topic id="spec" status="active" summary="Spec basis">

## spec_basis (grepped in `master-spec-v02.225/spec-modules/14-studio-creative-suite.md`)

- `[STU-RAS-025]` (:1227): bucket/flood fill by tolerance and anti-alias with a colour or `StudioPattern`; a gradient tool on layers, fill layers and masks; geometries Linear, Radial, Angle, Reflected and Diamond; foreground/background fill commands.
- `[STU-RAS-160]` (:1234): the `StudioGradient` parameter table. Independent colour and transparency stop lists, `stop.midpoint` default 50, `geometry`, `reverse`, `dither`, the noise form, and `shape_burst` as normative and not to be dropped.
- `[STU-RAS-161]` (:1265): `StudioPattern`. Tile bitmap, RGB, grayscale or indexed, 1x1 legal, id authoritative.
- `[STU-RAS-119]` (:729): `anti_alias` default true; `contiguous` default true (flood select).
- `[STU-RAS-170]` (:1218) and `[STU-RAS-117]` (:629): no pixel edit on non-pixel layers without an explicit rasterize.
- `[STU-SHL-155]` (:32410): tool catalogue rows `flood_fill` and `gradient` (family `fill_color`).
- The Folio `StudioFill::Gradient` record has no transparency stop list (`folio src/fill_layer.rs` module note). That is irrelevant here, because a destructive gradient writes pixels, not a record.
  - Do NOT add a Folio field.
  - If you believe one is needed, stop and report the exact signature to the lead.
- Gradients on masks (STU-RAS-025): in scope only if the viewer already has a mask edit target. Otherwise list it in the notes as remaining.

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ps-gradient-bucket-1`, base d208bc0e)

- `hsk-studio-pigment`:
  - new `src/fill_tools.rs` (or a `src/fill_tools/` folder);
  - one `pub mod` line in `src/lib.rs`;
  - `pub(crate)` visibility hunks in `src/effect_math/style.rs` and `src/select_tools/wand.rs`, only if reuse needs them;
  - new `tests/fill_tools.rs`.
- `hsk-studio-harness-view`:
  - new `src/view/fill_tools.rs`;
  - additive hunks in `src/view/edit.rs` (`Tool` enum, `ALL`, `key`, `label`);
  - additive hunks in `src/view/mod.rs` (module line, gesture and click dispatch near the `Tool::Brush` arm, hover text).
  - `mod.rs` is a shared hot file: stage only your own hunks.
- `hsk-studio-harness`: new `tests/viewer_fill_tools.rs` (`#![cfg(feature = "viewer")]`, driving the real viewer like `tests/viewer_select_liquify_2.rs`).
  - That file pins `Tool::ALL.len()` through `state_json`. It must stay green; it compares against `Tool::ALL`, so adding tools is fine.
- Do not touch:
  - `hsk-studio-pulse`, `src/view/timeline*`, `src/view/timeline/**`, `src/view/mixer.rs` (SMB-AU-MULTITRACK-3 is running);
  - `hsk-studio-type`, `src/view/vector.rs`, `src/view/text.rs` and the harness-view vector and text tests (SMB-AI-AREA-TYPE-4 is running);
  - `hsk-studio-harness-core` and `hsk-studio-layout` (SMB-R42-FOLLOWUPS-1 is running). Call the existing `Run` staging API only; if you need a new `Run` method, report the exact signature to the lead;
  - `hsk-studio-folio`, the Master Spec, `RUN-UI.md` (list its lines in the notes instead), and root inputs.
- This is the GUI lane 2 of max 2 (MULTITRACK-3 holds the other).

</topic>

<topic id="proof" status="active" summary="Proof and run timing">

## Proof and run timing

Red first: write each slice's tests and commit them red before the code. Record each red run per RUN LOGS.

Focused tests, 5 or fewer per slice:

- **Gradient.** Hand-computed pixels on a small layer:
  - linear and reflected along a 45-degree drag;
  - radial at the radius;
  - angle at 90 degrees;
  - diamond on an axis;
  - a midpoint of 25;
  - a transparency list of 2 stops against a colour list of 3;
  - `reverse`.
- **Clipping.** A feathered selection limits the change to coverage. Pixels outside the selection stay byte-identical.
- **Flood.**
  - Contiguous against global fill on a two-island image.
  - Tolerance 0 against 32 at a known boundary.
  - Anti-alias off gives a hard edge.
  - A 1x1 pattern tile and a 3x2 pattern tile at a phase you compute by hand.
- **Hostile input (V4-01/02).** Each case gets a typed refusal, never a panic and never a silent green (V4-03):
  - NaN or infinite coordinates;
  - zero-length drag;
  - seed outside the canvas;
  - more stops than the cap;
  - stops out of order;
  - opacity out of range;
  - a 0x0 pattern;
  - a non-pixel target layer.
- **Viewer.**
  - Each tool in the window is one undo step that undo restores byte-identical.
  - The operation log names the op.
  - Choosing the vector live-paint `paint_bucket` on a vector layer still works (regression).

Bundled run (rules 4a-4e):
- Code and test now, with focused check, clippy and nextest of pigment.
- Pigment is upstream of composite, develop, pulse, render-cpu, render-gpu, the interop crates and the harness crates, so the bundled run is large:
  - clippy over pigment and all its direct dependents;
  - the WHOLE suites of pigment, harness-view and harness, with the round's viewer features (rule 4e: `.config/round-features.toml` has `hsk-studio-harness = [["viewer"], ...]`; confirm the harness-view unit compiles your test file, and note it if not);
  - doctests of pigment if you add doc examples (rule 4d).
- Step 0: `lane-check.sh --step0-only` before the run. Use no drive-letter literals in tests (rule 4c).
- Do NOT start the bundled run until the lead messages you naming the disk (C: `--disk ssd` or D: `--disk d`) and the base.
  - A C: validation round (R43) may be open, and no builder runs on C: during a C: round.
  - SMB-AU-MULTITRACK-3's push changes harness-view under you. Rebase onto the base the lead names, then run once.
- Name the log per RUN LOGS: the lane id and sha in the path, and the sha on the first line.
- Set READY_FOR_VALIDATION per slice on a pushed commit in the lane file (workflow-v3 keys). Never claim PASS.

</topic>
