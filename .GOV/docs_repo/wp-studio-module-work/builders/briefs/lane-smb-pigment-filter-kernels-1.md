---
file_id: LANE-SMB-PIGMENT-FILTER-KERNELS-1
file_kind: lane_brief
updated_at: 2026-11-07T06:23:52Z
wp: WP-KERNEL-STUDIO
lane: SMB-PIGMENT-FILTER-KERNELS-1
lane_file: builders/lanes/hsk-studio-pigment-filter-kernels-1.json
crates: [hsk-studio-pigment (filter kernels), hsk-studio-folio (smart filter records), hsk-studio-render-gpu (one refusal arm)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Nineteen raster filter kinds covering 17 catalogue rows reach the live filter stack and saved documents, CPU only, with a typed refusal on the GPU backend">

## Target

HANDOFF next-work item 7, filters. Verified at c4d06841 (crates under `handshake-studio-assets/crates/`). Evidence: `S/research/truth-check-20261105-mt-raster-effects-motion.json`, family Photoshop-style filter families (7 built, 13 partial, 111 missing of 132 sampled).

**The gap.** A raster layer can store only five filter kinds plus liquify (`hsk-studio-folio/src/smart_filter.rs:63-70`; `hsk-studio-pigment/src/filter_math.rs:63-74`). Pigment already holds tested box, directional, radial and Find Edges maths (`blur_math.rs:441`, `:711`, `:822`; `filter_math.rs:545`, tests `tests/ae_kernels.rs`), but only composition effects can reach them (`hsk-studio-composite/src/composition/effect_stack.rs:535`, `:701`, `:714`, `:725`). Convolution, morphology, Dust & Scratches, Surface and Bilateral Blur, Emboss, Solarize and Mosaic have no raster kernel at all. Spec requires them: STU-FX-013 (module 14 line 19439), STU-FX-015 (19495), STU-FX-016 (19512), STU-FX-019 (19552), STU-FX-020 (19569), STU-FX-022 (19664), STU-RAS-037 (1357).

**What already works and must be reused.** The record path is whole: `StudioRasterLayerRecord::smart_filters` (`folio/src/raster.rs:763`, validated `:1792`) lowers through `SmartFilterOp::from_record` (`pigment/src/smart_filter.rs:306`; `composite/src/lower_render_plan/smart_filter.rs:41-79`; placed layers `composite/src/placed.rs:62-100`; clip effects `composite/src/clip_finishing.rs:692`, `:699-711`), render-cpu calls `SmartFilterStack::render` once over the whole plane (`render-cpu/src/plan_sink/smart_filter.rs:197-208`), and a saved document re-renders after reopen (`harness/tests/smart_filter_package.rs:131-164`). Enable, opacity, entry mask and stack mask all work (`smart_filter.rs:210-235`). Do not build a second path.

**Design, decided by the code.** Add the kernels as ONE new variant of `StackFilter` (`smart_filter.rs:100-105`), holding a new enum in a new module `pigment/src/filter_kernels.rs`. Do NOT add variants to `LayerFilter` or `FilterKind`: `LayerFilter` carries a window contract and GPU parity (`LayerExtension::filters`, `render-cpu/src/plan_sink/extended.rs:131-143` `stage_reach`, `render-gpu/src/plan_sink.rs:337-381`, `render_plan/output.rs:223-227`), and no production code calls `with_filters` (only the definition, `render_plan/extensions.rs:257`, git grep over `*/src/*`). Whole-plane kernels need no window reach.

**The one render-gpu line.** `render-gpu/src/plan_sink.rs:279-282` matches `StackFilter` exhaustively, so a new variant does not compile without one new arm there: a refusal beside the liquify one, `return Err(unsupported(<kernel key>))`, typed and before any device work (precedent `smart_filter_liquify`, test `render-gpu/tests/gpu_smart_filter.rs:241-253`). It holds no wgpu type and no WGSL. The mechanical gate of SMB-CONTROLS-EGUI-GATE-1 reads manifests and `.wgsl` files only (`S/builders/briefs/lane-smb-controls-egui-gate-1.md` item 4). BUT `wave-parity-2-common.md` HOST INTEGRATION (1) says a lane that would add render-gpu code waits. This arm therefore needs the lead's ruling before dispatch (open_questions 1). Nothing else in render-gpu changes. The only host falls back to render-cpu on that refusal with the note `gpu_fallback` (`harness-view/src/view/render.rs:333-358`, line 345), so documents still render.

Every row below has GPU `Opt` or `No` in its 14.9 group row. No GPU `Req` row of that table is in this slice (STU-FX-036, line 19865). STU-FX-130 (line 13163) marks the compositing copies of Find Edges, Dust & Scratches and Mosaic `Req`, and defines that as: a CPU fallback must be proven promotion-equivalent before it is claimed. No such proof is possible while GPU kernels are frozen, so claim no such equivalence.

## Items (red first per item; proven red by running)

1. **Plumbing, with the four reuse kinds.** Red: JSON records of the four kinds refused today (`deny_unknown_fields`, `folio/src/smart_filter.rs:62`). Add `FilterKernel` (validate, key, cost, plane form), `StackFilter::Kernel`, the arms in `smart_filter.rs:199-207`, `:211-219`, `:278-281`, `:316-348`; Folio variants in `StudioSmartFilterKind` with arms in `folio/src/smart_filter.rs:184-205` and `folio/src/clip_finishing.rs:1115-1121`; the render-gpu arm. Then the four kinds, calling the existing owners with no copied maths (CX-CAP-001).
2. **Convolution family:** one 5x5 replicate-border primitive, then its fixed kernels.
3. **Edge-gated and point kinds:** extract the Sobel magnitude out of `find_edges_window` (`filter_math.rs:571-595`) with its goldens unchanged, then Sharpen Edges, Despeckle, Emboss, Solarize, Mosaic.
4. **Morphology:** Maximum and Minimum.
5. **Rank and edge-preserving:** Dust & Scratches (reuse `disc`, `filter_math.rs:217`, made `pub(crate)`), Surface Blur, Bilateral Blur (one engine, two weight functions).
6. **Document path proof.** New test file `harness/tests/filter_kernels_package.rs`, shape of `smart_filter_package.rs`: one stack using every new kind saves, reopens and renders through composite and render-cpu, equal to `SmartFilterStack::render` of the same content. New `render-cpu/tests/cpu_filter_kernels.rs`: bytes equal for every chunking and thread count (shape `tests/cpu_smart_filter.rs`). One composite test: a clip effect `PigmentFilter` with a new kind renders (shape `composite/tests/clip_effect_stack.rs`). One row in `render-gpu/tests/gpu_smart_filter.rs:253`: the typed refusal.
7. **Correct the stale docs you touch.** `smart_filter.rs:20-21` says composite does not lower the record and render-gpu refuses; `:242` DESCRIPTOR lists both as gaps. Both false now (`lower_render_plan.rs:1477`, `:1521`; `render-gpu/src/plan_sink.rs:2141-2145`). Add a `filter_kernels` DESCRIPTOR: every kind, formula reading, bound provenance, gaps, parity NOT_PROVEN.

## Parameter tables

Bound rule, binding: STU-FX-101 (line 12729): a 14.9 table value wins over the group row. STU-FX-104 (12785): outside a declared hard bound is an error, never a clamp. STU-FX-106 (12826): where the source declares no bound, do NOT invent one. So a row without a table takes the parameter names from its own row (STU-FX-128, line 13140: raise the gap, do not invent a list), checks only finiteness and mathematical domain, and is stopped by the work budget. All values are record fields; every kind is class Approximate, formulas are Studio readings, vendor parity UNVERIFIED (D5).

| `filter_kind` | MT | Parameters | Bounds and source | Formula |
|---|---|---|---|---|
| `box_blur` | MT-32811 | `radius_px` f32, `iterations` u32, `dimensions` horizontal_and_vertical / horizontal / vertical | radius hard 0..=30000, iterations hard 1..=50: Fast Box Blur table line 13887 (`declared_both`) | half = floor(radius + 0.5), as `composite/src/temporal/composition/effects.rs:478`; `box_kernel` then `blur_window_in` or `blur_window_axis`. Above `MAX_BOX_KERNEL_TAPS` (`blur_math.rs:398`) is `filter_over_budget`, not a parameter error, because the value is inside its hard bound |
| `motion_blur` | MT-32817 | `angle_deg` f32, `length_px` f32 | length hard 0..=1000: Directional Blur table line 13877, wins over row 19451's intensity 1-999 (open_questions 3). Angle: any finite, table declares none | `directional_blur_window`, angle from +x towards screen-up (`blur_math.rs:705`) |
| `radial_blur` | MT-32818 | `centre_x_px`, `centre_y_px` f32, `amount` f32, `kind` spin / zoom | amount hard 0..=1000: table line 13914. Centre: any finite | `radial_blur_window`, plan coordinates |
| `find_edges` | MT-32889 | `invert` bool | none | `find_edges_window` |
| `custom_convolution` | MT-32941 | `weights` [i32; 25], `scale` i32, `offset` i32 | row 19668 has no table. `scale == 0` is `filter_parameter` (division domain). No other range | premultiplied per channel, replicate border: `sum(w q) / scale`, un-premultiplied by the source alpha, plus `offset / 255`, clamped; alpha is the source's; source alpha 0 gives `[0,0,0,0]` |
| `sharpen`, `sharpen_more` | MT-32879 | none | parameterless, row 19558 | the primitive with 3x3 `[0,-1,0; -1,5,-1; 0,-1,0]` and `[-1,-1,-1; -1,9,-1; -1,-1,-1]`, scale 1 |
| `basic_blur`, `blur_more` | MT-32810 | none | parameterless, row 19444: more is about 3-4x stronger | 3x3 binomial `[1 2 1]` squared, scale 16; 5x5 `[1 2 2 2 1]` squared, scale 64. Test pins impulse variance per axis 0.5 and 1.5: ratio 3 |
| `sharpen_edges` | MT-32880 | none | parameterless, row 19559 | `sharpen` where Sobel magnitude >= 0.1, else source. The 0.1 is a named constant, Studio reading |
| `despeckle` | MT-32850 | none | parameterless, row 19502 | `basic_blur` where magnitude < 0.1, else source |
| `emboss` | MT-32887 | `angle_deg` f32, `height_px` f32, `amount_pct` f32 | names from row 19575; catalogue row 13451 has no typed params (STU-FX-128 gap, raise it). Finite, height and amount >= 0 | gray `g = 0.5 + amount/100 * (L(p + h d) - L(p - h d))`, bilinear (`SUBPIXEL_FILTER`), `d = (cos, -sin)`; alpha is the source's. Row's color-amount: declare as a gap if not modelled, never silently ignored |
| `solarize` | MT-32892 | `threshold_levels` u8 | hard 0..=254: table line 18430 | per colour channel: `v * 255 > threshold` gives `1 - v`, else `v`; alpha kept |
| `mosaic` | MT-32863 | `horizontal_blocks`, `vertical_blocks` u32 | hard 1..=4000 each: table line 15472 (rows 1, 2; row 3 is unnamed, not implemented, name it in gaps) | plane split at `floor(i * W / n)`; each cell the premultiplied mean. Grid anchored to the plane |
| `maximum`, `minimum` | MT-32942, MT-32943 | `radius_px` u32, `preserve` squareness / roundness | radius hard 0..=32000: Minimax table line 14572. `preserve` from row 19669; Operation, Channel, Direction rows are a later slice, named in gaps | per premultiplied channel, transparent beyond the source (as median, `filter_math.rs:24`). Squareness: separable van Herk / Gil-Werman, work independent of radius. Roundness: disc `dx^2 + dy^2 <= r^2`. Radius 0 is the identity. Both proven equal to a brute-force oracle on small planes |
| `dust_and_scratches` | MT-32851 | `radius_px` u32, `threshold` f32 | radius hard 0..=255, threshold hard 0..=1: table line 15233 | per premultiplied channel: median over the disc `m`; `|o - m| > threshold` gives `m`, else `o`. Threshold 0 equals `median` wherever radius >= 1: pin it |
| `surface_blur` | MT-32822 | `radius_px` u32, `threshold_levels` f32 | names from row 19456, no table. Threshold > 0 (domain) | square window; per channel weight `max(0, 1 - |q - o| * 255 / (2.5 t))`, normalised. Studio reading; this weight is folklore, public sources did not confirm it |
| `bilateral_blur` | MT-32814 | `radius_px` u32, `range_sigma` f32 | names from row 19448, no table. Sigma > 0 (domain) | same engine: Gaussian spatial `sigma = radius / 2` times Gaussian range; state that `exp` is platform libm, so byte stability is per platform (as `composite/src/composition/effect_stack.rs:19-21`) |

Common contract: straight RGBA unit range in and out; input domain checked with `check` (`filter_math.rs:207`); `FilterLimits` and a `*_cost` with checked arithmetic before any allocation; `try_reserve_exact` (as `filter_math.rs:565`); binary64 in a fixed order; a non-finite result is `filter_out_of_domain`. The stack refuses extended range already (`render_plan/extensions.rs:845`), so these kinds declare unit range only (STU-FX-006a).

## V4-06 failure-class sweep

Classes, each with a list of sites and fixed / not applicable with a reason in the lane notes:
(a) Every `match` over `StudioSmartFilterKind` and `StackFilter`, read rather than found by the compiler where a wildcard hides it: `if let` at `render-cpu/src/plan_sink/extended.rs:507`; `SMART_ENTRY_BYTES` charge at `:502-505` must still cover the larger entry. NOT_INSPECTED: the constant's value.
(b) Folio's non-negative rule `folio/src/smart_filter.rs:183` would refuse legal negative `angle_deg`, and `clip_finishing.rs:1114` caps at `MAX_CLIP_BLUR_PX`. New arms must not inherit either wrongly (STU-FX-106). Range belongs to pigment; lowering refuses (`lower_render_plan/smart_filter.rs:75`).
(c) Plane dependence. `plan_sink.rs:978-981` makes the plane content bounds intersected with the plan extent, so a kernel sees the extent. Mosaic's grid moves with it, and any neighbourhood kind differs near a cropped edge. Pin what a cropped extent does for mosaic and one neighbourhood kind, and write it in the module docs; do not claim extent independence.
(d) A cost that loops on an unchecked radius: `disc_len` is O(r) (`filter_math.rs:233`), safe at today's 500. New unbounded radii (surface, bilateral) must compute their cost in O(1). Pin `u32::MAX` refused quickly.
(e) Existing bound drift, report only, change nothing: `Median` 1..=500 (`filter_math.rs:169`) against table hard 0..255 (line 15291); `GaussianBlur` at most `MAX_BLUR_RADIUS_PX` 1000 (`blur_math.rs:41`) against table hard 0..30000 (line 13901); `UnsharpMask` Studio ranges with no table check. Stored documents depend on them.

## Hostile input (V4-01/02)

Typed before allocation, one test per kind: NaN and infinity in every float; hard bound plus and minus one step; `scale` 0; zero-size plane; 65536 x 65536 plane; radius `u32::MAX`; a stack of 17 entries (`MAX_SMART_FILTERS`, `folio/src/smart_filter.rs:22`); unknown field and unknown kind on the wire. Work goes against `StackLimits.filter.max_work` (`smart_filter.rs:91`, passed at `render-cpu/src/plan_sink/smart_filter.rs:169`). Known gap, report only: `SmartFilterStack::render` takes no cancel token and `run_smart` checks cancel per job (`render-cpu/src/plan_sink/smart_filter.rs:177`), so one stack can run up to the work budget uncancelled. Vendor parity UNVERIFIED (D5); do not bless any row.

## MT rows to read

MT-32811, 32817, 32818, 32889, 32941, 32879, 32880, 32810, 32850, 32887, 32942, 32943, 32851, 32822, 32814, 32892, 32863; clauses MT-32808, 32831, 32848, 32855, 32877, 32885, 32940. Catalogue rows have five criteria: AC-001..004 are behaviour, AC-005 needs an operator surface and headless path (read on MT-32941), so this lane cannot fully close any row: claim only criteria you read and prove. Their proof commands name composite test targets that do not exist (MT-32941: `custom_convolution_five_square`); do not create those names and do not edit MT files.

## Out of scope

Any GUI, gallery browser or filter dialog; egui, eframe, winit, wgpu, naga or WGSL; GPU kernels; every render-gpu change beyond the one arm and the one test row; any GPU `Req` row (Lens, Depth-of-Field, Spin, Field, Iris, Tilt-Shift, Path, Oil Paint, Lighting, Flame, Picture Frame, Tree); `LayerFilter` and `FilterKind` changes; Average (MT-32809), because masks mix after the filter (`smart_filter.rs:231`) so its mean cannot be limited to the selection without a stack change; typed commands that write the record (next slice); apply-to-pixels and `EFFECT_BAKE`; per-entry blend modes (STU-FX-008, `smart_filter.rs:21`); distort family; Crystallize, Voronoi and the tessellation core (STU-FX-016a: no shared cell core exists; nib has only gradient-mesh tessellation, `nib/src/mesh.rs:677`, and Mosaic's plain grid does not pretend to be it); composition effect kinds; a new crate edge, any `Cargo.toml`, Master Spec edit or MT edit.

LEAD RULINGS: (1) yes: one refusal arm with no wgpu type at render-gpu plan_sink.rs:279-282 plus one test row, and no more. (2) Ship before filter commands; commands are the next slice. (3) Motion Blur hard 0..=1000 under STU-FX-127 and STU-FX-101; record for the spec owner. (4) Keep Mosaic as a plain grid and record STU-FX-016 as unmet. PUSH HOLD: STUDIO-R59 is judging c4d06841 on C:. Build on D: only and push nothing until the lead says R59 is recorded.

OPERATOR DIRECTION, 2026-11-07 (HANDOFF topic workflow, items 9 to 12): rapid implementation, width and depth first. Minimal testing: red test and focused tests per item, one bundled run, no busy work. Message the lead with your tip before the bundled run, so review agents inspect it first. Tree: this lane reuses a completed lane's worktree and cargo target (Codex CX-GIT-001A, root HSK-ART-005); the folder name and `--owner` are the old lane's; your branch, log names and records are this lane's. No new egui, eframe, winit, wgpu, naga or WGSL line (D1, D7). No Master Spec edit. Never touch WP-KERNEL-012 files. Never touch refund-claim-20261021.md.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block for this lane">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.229", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "report_for_host: pigment and folio return typed refusals and work counters, emission stays with the host", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "unchanged: filters render, the record is written by its existing callers, and this lane adds no mutating operation", "jobs": "none", "settings": "none", "refactor_exposure": "medium"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ps-mode-convert-1`, branch `lane/smb-pigment-filter-kernels-1`, base c4d06841)

`crates/hsk-studio-pigment/src/filter_kernels.rs` (new), `src/smart_filter.rs`, `src/filter_math.rs` (visibility and the Sobel extraction only), `src/lib.rs`, `tests/`; `crates/hsk-studio-folio/src/smart_filter.rs`, `src/clip_finishing.rs` (the PigmentFilter arm only), `tests/`; `crates/hsk-studio-render-gpu/src/plan_sink.rs` (the one refusal arm in `compile_smart`, on the lead's ruling), `tests/gpu_smart_filter.rs` (one refusal row); new test files only in `crates/hsk-studio-render-cpu/tests/`, `crates/hsk-studio-harness/tests/`; `crates/hsk-studio-composite/tests/` (one new test). Anything else: message the lead.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- Red first per item, proven red by running it; keep compiling red commits, a pushed red commit must compile the round units of the changed crates and direct dependents and fail only its own new tests.
- One bundled run on D: (`lane-check.sh --owner smb-ps-mode-convert-1 --disk d`): step 0; clippy over pigment, folio and their direct dependents; nextest WHOLE suites of pigment, folio, package (rule 4b, new Folio fields and schema), composite and render-cpu; harness `--test smart_filter_package` and `--test filter_kernels_package`; render-gpu `--test gpu_smart_filter` (waits for the GPU test lock); doctests if added.
- Before that bundled run, message the lead with your tip: read-only code-check and red-team agents inspect the diff first (HANDOFF topic workflow, item 10). Until then use `cargo check` and focused tests only, through `lane-check.sh` or a shell that sourced `builders/studio-env.sh`. A repo-local `target/` is a governance violation.
- Push when that run is green: rebase on `origin/codex/handshake-studio-assets`, `lane-check.sh --owner smb-ps-mode-convert-1 --disk d --per-sha HEAD~1` (the tip only, item 11), push on `PER_SHA_RESULT OK` with the guarded sha form.
- If origin moves after your bundled run: when the commits touch none of your crates, rebase, `--per-sha`, your new tests only, push; when they touch one of your crates, run that crate's suite once, then the same. Do not repeat whole runs for a move. The next full round proves the union.
- Set READY_FOR_VALIDATION on a pushed commit. Never claim PASS.

</topic>
