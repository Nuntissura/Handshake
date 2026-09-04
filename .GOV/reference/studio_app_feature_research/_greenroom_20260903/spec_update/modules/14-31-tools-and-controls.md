---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-31"
section_id: "14.31"
title: "14.31 Tools, the Options Surface, and the Scrubbable Numeric Control"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
supersedes_section: "NONE — new sub-section. Extends 14.16 and 14.21 of .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md and continues the [STU-SHL-*] clause block opened in 14.30."
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-31-tools-and-controls.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.31 Tools, the Options Surface, and the Scrubbable Numeric Control

This sub-section specifies what lives inside the operator shell of 14.30: the tool registry and its families, the Tool Rail, the two-zone Context Bar, Task Scopes, the generated numeric parameter contract, and the one scrubbable numeric widget every value in Studio is edited through. It continues the `[STU-SHL-*]` clause block opened in 14.30 and the `[STU-MDL-1**]` block that extends [STU-MDL-002].

Every clause in 14.30 §0 applies here unchanged: one vocabulary ([STU-SHL-002]), no vendor names ([STU-SHL-008]), SurrealDB with the EventLedger only and no second store ([STU-SHL-007]), no GPU dependency in `handshake_core` ([STU-SHL-009]), and every surface observable and steerable out of process without stealing focus ([STU-SHL-009], [STU-MDL-109]).

---

### 1. The tool registry

**[STU-SHL-140] Tool count (normative).** Studio has **362 distinct tools in 22 families**. 549 raw tool names were recovered across six per-application registries and collapsed to 362 through an authored cross-vendor synonym table. The figures 353 (pre-fold-in) and 1,270 (the defective capability-registry bucket) are superseded and MUST NOT be cited. The realistic ceiling is roughly 375; one captured application's tool surface (approximately 14 tools) remains unrecovered. The 362 figure is an AUDITABLE JUDGEMENT, not a raw measurement — see [STU-SHL-135] — and every merge MUST ship inline on its tool row as `vendor_variants` so any merge is visible and reversible from the artefact alone.

**[STU-SHL-141] The 22 families (normative, closed).** Family is the TAXONOMIC axis and is closed. Every tool belongs to exactly one family. The family level is what makes 362 tools browsable and it is MANDATORY in the menu ([STU-SHL-039]).

| family_id | Display name | Definition | Members |
|---|---|---|---|
| `select_object` | Selection — object | Pick, sub-pick and group-pick whole objects, nodes and clips. | 11 |
| `select_region` | Selection — region | Produce or refine a pixel, range or colour selection rather than picking an object. | 22 |
| `navigate` | Navigation | Move the viewport. NEVER mutates the document. | 4 |
| `camera_3d` | Camera and 3D scene | Move and frame a camera layer, and manipulate 3D scene geometry. Distinct from `navigate` precisely because it MUTATES a camera layer's transform. | 4 |
| `paint_raster` | Paint | Deposit colour with a stroke, raster or vector. | 6 |
| `erase` | Erase | Remove coverage with a stroke. | 4 |
| `retouch` | Retouch and repair | Replace image content from elsewhere, from a model, or from a computed reconstruction. | 21 |
| `tone_brush` | Tone and local adjustment brushes | Modify tone, colour, focus or a live adjustment by painting a mask. | 12 |
| `draw_path` | Draw and edit paths | Construct and manipulate curve geometry and its nodes, handles and topology. | 21 |
| `shape` | Shapes | Parametric primitive construction. | 37 |
| `type` | Type and tables | Create and edit text runs, frames, type-on-path, grids and tables. | 18 |
| `fill_color` | Fill, gradient and colour | Apply and sample fills, gradients, meshes, transparency and styles. | 11 |
| `crop_frame` | Crop, frame and page | Define document, page, artboard, slice, frame and placement geometry. | 18 |
| `transform` | Transform and warp | Move, scale, rotate, distort, liquify and remap existing geometry or pixels. | 42 |
| `measure_annotate` | Measure and annotate | Sample distances, counts, notes and alignment WITHOUT changing render output. | 8 |
| `mask_channel` | Mask and channel | Author and edit masks, mattes and source-image masks as first-class targets. | 9 |
| `adjustment_live` | Live adjustment tools | Each non-destructive tonal or colour adjustment as a selectable tool. | 36 |
| `filter_live` | Live filter tools | Each non-destructive filter as a selectable tool. | 44 |
| `generative_ml` | Generative and model-backed | Model-backed tools: generative fill, expand, extend, background removal, model-driven selection. | 7 |
| `timeline_edit` | Timeline edit | The trim grammar: ripple edit, rolling edit, slip, slide, rate stretch, razor, remix, track select forward, track select backward. | 9 |
| `symbol_instance` | Symbol and instance | Spray and then modify symbol instances in place. | 8 |
| `data_graph` | Data and graph | Chart construction tools that build geometry from a data table. | 10 |

Total 362. Mean 16 members per family. Largest family `filter_live` at 44. This is the shape that `WORKSPACE > Tools > <family> > <tool>` renders.

**[STU-SHL-142] `adjustment_live` and `filter_live` are a declared vendor artefact.** These two families are 80 tools, 22.1% of the list, and exist because ONE captured vendor models every non-destructive adjustment and every live filter as a selectable TOOL as well as a menu command; the other captured vendors model the same operations as menu items and adjustment layers only. Whether they stay tools or become commands that create a layer is an open operator decision ([STU-SHL-136] OD-3) and decides whether the count is 362 or roughly 296. The recommended split, recorded as a recommendation and not a decision, is the GESTURE TEST: an operation that needs a canvas gesture to say WHERE it applies (adjustment brush, filter brush, tone brush, mask paint, the liquify set) stays a tool; an operation that is a parameter dialog applied to a whole layer (levels, curves, posterize, threshold, unsharp mask, the blurs) becomes a command that creates a layer. Until the decision is recorded, 362 stands and both families ship as tools.

**[STU-SHL-143] Tool record.** Every tool is one UiDescriptor ([STU-SHL-240]) with `kind = tool`, carrying at minimum `author_id` (`studio.tool.<family>.<tool>`), `command_id`, `family_id`, `group_id`, `display_name`, `summary` (nullable), `shortcut_id`, `manual_anchor`, `availability` (`requires`, `unavailable_reason_template`, `remedy_command_id`), `provenance` with `vendor_variants`, `declared_options` (the ordered ParamSpec ids of its Tool Zone controls), and `aliases[]`.

**[STU-SHL-144] Four projections, one command id.** Every tool is reachable from FOUR surfaces and all four dispatch the IDENTICAL command id: the Tool Rail slot, the menu leaf at `WORKSPACE > Tools > <family> > <tool>`, a tool-search or command-palette row, and family browse. A tool reachable from one and missing from another is a registry bug ([STU-SHL-013], MENU-INV-7). Absence of a model invocation path for a tool is a conformance defect under [STU-MDL-006], not a design choice.

---

### 2. The Tool Rail

**[STU-SHL-145] Shape.** The Tool Rail is a single-column vertical rail on the `left-rail` region holding ONE SLOT PER AVAILABLE TOOL GROUP. It is not a persona switcher, not a mode, not a ribbon, and not an index.

**[STU-SHL-146] Family versus group.** A tool GROUP is a set of tools that share a family AND are mutually exclusive alternatives for the same gesture. Family is the taxonomic axis (22, closed); group is the ERGONOMIC axis (one rail slot). One slot per family is wrong in both directions: `navigate` has 4 members and `filter_live` has 44.

**[STU-SHL-147] Visible at rest.** 20 group slots by default; hard cap 26. Every shipped reference surface with a measurable tool panel lands between 18 and 25 slots while carrying between 4 and 97 tools, so this band is measured rather than chosen.

**[STU-SHL-148] The honest architecture.** 20 visible slots must reach 362 tools. They cannot and MUST NOT try. Roughly 120 tools live in the rail's groups; the remaining roughly 240 are reached by context, by search, by family browse, or from the menu. The rail is an ergonomic surface, not a directory. This is the reason the rail may hide a `NOT_IN_THIS_DOCUMENT` tool while the menu may not ([STU-SHL-052]).

**[STU-SHL-149] Variant disclosure MUST be untimed.** A timed long-press MUST NOT be the only path to a tool variant. An untimed, VISIBLE disclosure is required. Four untimed paths MUST exist:

1. click the persistent corner marker on the slot, or secondary-click anywhere on the slot; the variant strip opens with NO timer;
2. press the group's shortcut repeatedly to cycle members forward, and `Shift`+the shortcut to cycle backward;
3. type the tool name into tool search or the command palette;
4. open `WORKSPACE > Tools > <family> > <tool>` from the menu.

A timed long-press MAY survive as a muscle-memory alias with identical behaviour and MUST NEVER be load-bearing. The reasons are stated because they generalise: a timed press is an invisible gesture with a hidden duration and nothing on screen states that it exists or how long it takes; it is nondeterministic for the out-of-process inspector to drive and flaky to assert, and Handshake validates GUI work through that inspector, so a gesture the inspector cannot reliably produce is a gesture the product cannot reliably test; and it costs the long-press latency on every variant switch. The largest captured tool group has 24 members, which is not usable as a flyout strip at all, so family browse is not optional.

**[STU-SHL-150] Adaptive face.** The last-used member of a group becomes that slot's face. The palette adapts to the operator without any mode, persona or preference.

**[STU-SHL-151] The rail may reorder; it may never gate.** No Layout Preset, preference or Task Scope may ADD or REMOVE a tool from what the availability predicate permits. A Layout Preset may only REORDER and PRIORITISE ([STU-SHL-093], [STU-SHL-055]). The rail renders the predicate per the `tool_rail` row of [STU-SHL-052]: `AVAILABLE` enabled, `INAPPLICABLE_HERE` dimmed with the remedy inline, `NOT_IN_THIS_DOCUMENT` absent from the rail but still present in the accessibility tree and still returned by search with its reason.

Worked examples that MUST hold, each exercising a different reason code:

| Worked example | `requires` | Situation | Result |
|---|---|---|---|
| node | `selection_kind in {object,node} AND layer_kind in {vector_curve, shape, text}` | on a pixel layer | `INAPPLICABLE_HERE`, `WRONG_LAYER_KIND`, remedy "convert this pixel layer to curves" |
| ripple edit | `container:timeline AND doc_feature:footage_clips` | in a print layout | `NOT_IN_THIS_DOCUMENT`, `NO_SUCH_CONTAINER` |
| orbit camera | `container:timeline AND layer_kind:camera present` | in a composition with no camera | `INAPPLICABLE_HERE`, `WRONG_LAYER_KIND`, remedy "add a camera layer" |
| crop | `container in {artboard, page_spread, timeline}` | anywhere | `AVAILABLE`. Present in five captured applications; under persona gating it would be implemented once and hidden four times |
| generative fill | `capability_flag:ml_model_present AND selection_kind:pixel_selection` | with no model present | `INAPPLICABLE_HERE`, `CAPABILITY_FLAG_ABSENT`, remedy "download model" |

**[STU-SHL-152] Task Scope (normative).** A Task Scope is a named, document-scoped, TEMPORARY filter over the availability predicate, entered deliberately and exited deliberately. It:

- narrows the visible tool set and swaps the Context Bar Tool Zone;
- is read by the predicate through the `task_scope:<id|null>` clause kind of [STU-SHL-049], so its effect is INSIDE the one predicate and is visible in the reason code — it is not a second gating mechanism;
- MUST NOT swap panels, menus, shortcuts or chrome;
- MUST ALWAYS present a visible exit, and MUST also be exitable by `WORKSPACE > Task Scopes > Exit Task Scope` and by a command id;
- is NOT a Layout Preset and MUST NOT be named as one ([STU-SHL-096]).

**[STU-SHL-153] Shipped Task Scopes.** Seven are proposed: `select_and_refine`, `content_aware_fill`, `liquify`, `perspective_correct`, `develop_raw`, `trace_to_vector`, `export_slices`. The count is an open operator decision ([STU-SHL-136] OD-5); the captured prior art ships three, declared as a task-space element with its own scoped tool list and its own scoped property list. Each shipped Task Scope declares its scoped tool id list, its scoped Tool Zone parameter list, and its exit command.

**[STU-SHL-154] Model-backed tools and the exclusion boundary.** The `generative_ml` family ships as tools. Model-backed surfaces excluded on policy by the source captures are recorded as an EXCLUSION LIST, not deleted: eight effect surfaces (body tracking, roto-brush segmentation and its three matte-refinement surfaces, detail-preserving upscale, auto-reframe, face track points), a content-aware fill panel, and scene edit detection. Whether Studio re-implements any of them against Handshake's own model runtime is an open operator decision ([STU-SHL-136] OD-9); if it does, this list becomes a re-implementation list. Every `generative_ml` tool MUST declare `capability_flag:ml_model_present` in its `requires`, so a machine without a model gets `INAPPLICABLE_HERE` with a remedy rather than a broken tool.

---

### 2A. The tool catalogue

**[STU-SHL-155] The tool catalogue (normative, closed).** The 362 tools of [STU-SHL-140] are enumerated below, grouped by the 22 families of [STU-SHL-141]. EACH ROW IS ONE UNIT OF WORK. A tool absent from this table does not exist in Studio; a tool present here may not be dropped, silently merged or deferred without an operator decision recorded under [STU-SHL-136]. Each row carries: the Studio name; the stable `tool_id` used to build its `author_id` `studio.tool.<family>.<tool_id>` ([STU-MDL-100]) and its command id; its family; its menu path, which is mechanically derived from the family per [STU-SHL-039] and is where the tool is guaranteed reachable ([STU-SHL-144]); the capture provenance; and the vendor variants that collapsed into it, shipped inline so every merge is auditable and reversible from this table alone ([STU-SHL-135]). Vendor names in the last two columns are PROVENANCE ONLY and are never Studio names ([STU-SECTION-003]).

Three per-tool facts are NOT resolved by the captures and are UNKNOWN for every row rather than guessed: (1) Tool Rail group membership - roughly 120 of the 362 live in the rail's ~20 groups ([STU-SHL-148]) but the captures do not resolve which, and the assignment is closed by OD-1 and by the rail work of [STU-SHL-145]-[STU-SHL-147]; (2) the default chord, because every chord is PROVISIONAL until the arbitration is recomputed ([STU-SHL-045], SD-2); (3) Task Scope membership ([STU-SHL-153]). A row's microtask carries those three as open inputs, not as invented values.

`select_object` - Selection - object, 11 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Direct Select | `direct_select` | `select_object` | WORKSPACE > Tools > Selection - object | illustrator, indesign, photoshop | Adobe Direct Object Select Tool; Adobe Direct Select Tool; Direct Selection Tool |
| Group Select | `group_select` | `select_object` | WORKSPACE > Tools > Selection - object | indesign | Group Selection Tool |
| Object Selection | `object_selection` | `select_object` | WORKSPACE > Tools > Selection - object | affinity, photoshop, premiere | Object Selection; Object Selection Tool; Smart Selection |
| Path Selection | `path_selection` | `select_object` | WORKSPACE > Tools > Selection - object | photoshop | Path Selection Tool |
| Perspective Selection | `perspective_selection` | `select_object` | WORKSPACE > Tools > Selection - object | illustrator | Perspective Selection Tool |
| Planar Face Select | `planar_face_select` | `select_object` | WORKSPACE > Tools > Selection - object | illustrator | Adobe Planar Face Select Tool |
| Quick Selection | `quick_selection` | `select_object` | WORKSPACE > Tools > Selection - object | photoshop | Quick Selection Tool |
| Select | `select` | `select_object` | WORKSPACE > Tools > Selection - object | affinity, illustrator, indesign, premiere | Adobe Select Tool; Select; Select Tool; Selection Tool |
| Selection Brush | `selection_brush` | `select_object` | WORKSPACE > Tools > Selection - object | affinity, photoshop | Selection Brush Tool |
| Selection Removal Brush | `selection_removal_brush` | `select_object` | WORKSPACE > Tools > Selection - object | photoshop | Selection Removal Brush Tool |
| Slice Select | `slice_select` | `select_object` | WORKSPACE > Tools > Selection - object | affinity, illustrator, photoshop | Adobe Slice Select Tool; Slice Select Tool; Slice Selection Tool |

`select_region` - Selection - region, 22 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Bezier Selection | `bezier_selection` | `select_region` | WORKSPACE > Tools > Selection - region | premiere | Bezier Selection |
| Elliptical Marquee | `elliptical_marquee` | `select_region` | WORKSPACE > Tools > Selection - region | affinity, photoshop, premiere | Elliptical Marquee; Elliptical Marquee Tool |
| Feather Selection | `feather_selection` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Feather Selection Tool |
| Grow Shrink Selection | `grow_shrink_selection` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Grow / Shrink Selection Tool |
| HSL Selection | `hsl_selection` | `select_region` | WORKSPACE > Tools > Selection - region | premiere | HSL Selection |
| Hue Range Picker | `hue_range_picker` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Hue Range Picker Tool |
| Lasso | `lasso` | `select_region` | WORKSPACE > Tools > Selection - region | affinity, illustrator, photoshop | Adobe Direct Lasso Tool; Freehand Selection Tool; Lasso Tool |
| Luminance Selection | `luminance_selection` | `select_region` | WORKSPACE > Tools > Selection - region | premiere | Luminance Selection |
| Magic Wand | `magic_wand` | `select_region` | WORKSPACE > Tools > Selection - region | affinity, illustrator, photoshop | Adobe Magic Wand Tool; Flood Select Tool; Magic Wand Tool |
| Magnetic Lasso | `magnetic_lasso` | `select_region` | WORKSPACE > Tools > Selection - region | photoshop | Magnetic Lasso Tool |
| Outline Selection | `outline_selection` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Outline Selection Tool |
| Polygonal Lasso | `polygonal_lasso` | `select_region` | WORKSPACE > Tools > Selection - region | photoshop | Polygonal Lasso Tool |
| Range Mask | `range_mask` | `select_region` | WORKSPACE > Tools > Selection - region | lightroom_classic | Range Mask Tool |
| Range Selection | `range_selection` | `select_region` | WORKSPACE > Tools > Selection - region | premiere | Range Selection |
| Rectangular Marquee | `rectangular_marquee` | `select_region` | WORKSPACE > Tools > Selection - region | affinity, photoshop, premiere | Rectangular Marquee; Rectangular Marquee Tool |
| Refine Edge Brush | `refine_edge_brush` | `select_region` | WORKSPACE > Tools > Selection - region | photoshop | Refine Edge Brush Tool |
| Refine Selection | `refine_selection` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Refine Selection Tool |
| Select Sampled Color | `select_sampled_color` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Select Sampled Color Tool |
| Select Sampled Depth | `select_sampled_depth` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Select Sampled Depth Tool |
| Single Column Marquee | `single_column_marquee` | `select_region` | WORKSPACE > Tools > Selection - region | affinity, photoshop | Column Marquee Tool; Single Column Marquee Tool |
| Single Row Marquee | `single_row_marquee` | `select_region` | WORKSPACE > Tools > Selection - region | affinity, photoshop | Row Marquee Tool; Single Row Marquee Tool |
| Smooth Selection | `smooth_selection` | `select_region` | WORKSPACE > Tools > Selection - region | affinity | Smooth Selection Tool |

`navigate` - Navigation, 4 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Hand | `hand` | `navigate` | WORKSPACE > Tools > Navigation | affinity, illustrator, indesign, photoshop, premiere | Adobe Scroll Tool; Grabber Hand Tool; Hand; Hand Tool; Liquify Hand Tool |
| Page | `page` | `navigate` | WORKSPACE > Tools > Navigation | illustrator, indesign | Adobe Page Tool; Page Tool |
| Rotate View | `rotate_view` | `navigate` | WORKSPACE > Tools > Navigation | affinity, illustrator, photoshop | Adobe Rotate Canvas Tool; Rotate Canvas Tool; Rotate View Tool |
| Zoom | `zoom` | `navigate` | WORKSPACE > Tools > Navigation | affinity, illustrator, indesign, photoshop, premiere | Adobe Zoom Tool; Liquify Zoom Tool; Magnify Tool; View Tool; Zoom; Zoom Tool |

`camera_3d` - Camera and 3D scene, 4 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Dolly Camera | `dolly_camera` | `camera_3d` | WORKSPACE > Tools > Camera and 3D scene | aftereffects | Dolly Camera Tool |
| Orbit Camera | `orbit_camera` | `camera_3d` | WORKSPACE > Tools > Camera and 3D scene | aftereffects | Orbit Camera Tool |
| Pan Camera | `pan_camera` | `camera_3d` | WORKSPACE > Tools > Camera and 3D scene | aftereffects | Pan Camera Tool |
| Unified Camera | `unified_camera` | `camera_3d` | WORKSPACE > Tools > Camera and 3D scene | aftereffects | Unified Camera Tool |

`paint_raster` - Paint, 6 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Blob Brush | `blob_brush` | `paint_raster` | WORKSPACE > Tools > Paint | affinity, illustrator | Adobe Blob Brush Tool; Vector Blob Brush Tool |
| Mixer Brush | `mixer_brush` | `paint_raster` | WORKSPACE > Tools > Paint | photoshop | Mixer Brush Tool |
| Paint Brush | `paint_brush` | `paint_raster` | WORKSPACE > Tools > Paint | affinity, illustrator, photoshop | Adobe Brush Tool; Brush Tool; Paint Brush Tool |
| Path Brush | `path_brush` | `paint_raster` | WORKSPACE > Tools > Paint | affinity | Path Brush Tool |
| Pencil | `pencil` | `paint_raster` | WORKSPACE > Tools > Paint | affinity, illustrator, indesign, photoshop | Adobe Freehand Tool; Pencil Tool |
| Pixel Brush | `pixel_brush` | `paint_raster` | WORKSPACE > Tools > Paint | affinity | Pixel Tool |

`erase` - Erase, 4 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Background Erase Brush | `background_erase_brush` | `erase` | WORKSPACE > Tools > Erase | photoshop | Background Eraser Tool |
| Erase Brush | `erase_brush` | `erase` | WORKSPACE > Tools > Erase | affinity, illustrator, indesign, photoshop | Adobe Eraser Tool; Adobe Freehand Erase Tool; Erase Brush Tool; Erase Tool; Eraser Tool |
| Flood Erase | `flood_erase` | `erase` | WORKSPACE > Tools > Erase | affinity, photoshop | Flood Erase Tool; Magic Eraser Tool |
| Vector Erase Brush | `vector_erase_brush` | `erase` | WORKSPACE > Tools > Erase | affinity | Vector Erase Brush Tool |

`retouch` - Retouch and repair, 21 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Art History Brush | `art_history_brush` | `retouch` | WORKSPACE > Tools > Retouch and repair | photoshop | Art History Brush Tool |
| Astrophotography Stack | `astrophotography_stack` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Astrophotography Stack Tool |
| Bad Pixel Map | `bad_pixel_map` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Bad Pixel Map Tool |
| Blemish Removal | `blemish_removal` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Blemish Removal Tool |
| Clone Brush | `clone_brush` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity, photoshop | Clone Brush Tool; Clone Stamp Tool |
| Content Aware Move | `content_aware_move` | `retouch` | WORKSPACE > Tools > Retouch and repair | photoshop | Content-Aware Move Tool |
| Defringe | `defringe` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Defringe Tool |
| Denoise | `denoise` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Denoise Tool |
| Dust And Scratches | `dust_and_scratches` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Dust & Scratches Tool |
| FFT Denoise | `fft_denoise` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | FFT Denoise Tool |
| Frequency Separation | `frequency_separation` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Frequency Separation Tool |
| Healing Brush | `healing_brush` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity, lightroom_classic, photoshop | Healing Brush Tool; Healing Tool; Spot Healing Brush Tool |
| Inpainting Brush | `inpainting_brush` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Inpainting Brush Tool |
| Light Field Focus | `light_field_focus` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Light Field Focus Tool |
| Noise Reduction | `noise_reduction` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Noise Reduction Tool |
| Panorama | `panorama` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity | Panorama Tool |
| Patch | `patch` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity, photoshop | Patch Tool |
| Pattern Stamp | `pattern_stamp` | `retouch` | WORKSPACE > Tools > Retouch and repair | photoshop | Pattern Stamp Tool |
| Red Eye Removal | `red_eye_removal` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity, lightroom_classic, photoshop | Red Eye Removal Tool; Red Eye Tool |
| Sampling Brush | `sampling_brush` | `retouch` | WORKSPACE > Tools > Retouch and repair | photoshop | Sampling Brush Tool |
| Undo Brush | `undo_brush` | `retouch` | WORKSPACE > Tools > Retouch and repair | affinity, photoshop | History Brush Tool; Undo Brush Tool |

`tone_brush` - Tone and local adjustment brushes, 12 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Adjustment Brush | `adjustment_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, lightroom_classic | Adjustment Brush Tool; Masking Brush Tool |
| Blur Brush | `blur_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, photoshop | Blur Brush Tool; Blur Tool |
| Burn Brush | `burn_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, photoshop | Burn Brush Tool; Burn Tool |
| Color Replacement Brush | `color_replacement_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, photoshop | Color Replacement Brush Tool; Color Replacement Tool |
| Dodge Brush | `dodge_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, photoshop | Dodge Brush Tool; Dodge Tool |
| Filter Brush | `filter_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity | Filter Brush Tool |
| Median Brush | `median_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity | Median Brush Tool |
| Sharpen Brush | `sharpen_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, photoshop | Sharpen Brush Tool; Sharpen Tool |
| Smudge Brush | `smudge_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, photoshop | Smudge Brush Tool; Smudge Tool |
| Sponge Brush | `sponge_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity, photoshop | Sponge Brush Tool; Sponge Tool |
| Targeted Adjustment | `targeted_adjustment` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | lightroom_classic, photoshop | Target Adjustment Tool; Targeted Adjustment Tool |
| Tone Brush | `tone_brush` | `tone_brush` | WORKSPACE > Tools > Tone and local adjustment brushes | affinity | Tone Brush Tool |

`draw_path` - Draw and edit paths, 21 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Add Anchor Point | `add_anchor_point` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator, indesign, photoshop | Add Anchor Point Tool; Adobe Add Anchor Point Tool |
| Blend | `blend` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator | Adobe Blend Tool |
| Contour | `contour` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity | Contour Tool |
| Convert Anchor Point | `convert_anchor_point` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator, indesign, photoshop | Adobe Anchor Point Tool; Convert Direction Point Tool; Convert Point Tool |
| Corner | `corner` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity | Corner Tool |
| Curvature | `curvature` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator | Adobe Curvature Tool |
| Delete Anchor Point | `delete_anchor_point` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator, indesign, photoshop | Adobe Delete Anchor Point Tool; Delete Anchor Point Tool; Remove Anchor Point Tool |
| Freeform Pen | `freeform_pen` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | photoshop | Freeform Pen Tool |
| Freehand Smooth | `freehand_smooth` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator | Adobe Freehand Smooth Tool |
| Image Trace | `image_trace` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity | Image Trace Tool |
| Knife | `knife` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity, illustrator | Adobe Knife Tool; Knife Tool |
| Node | `node` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity | Node Tool; Shape Node Tool |
| Pen | `pen` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity, illustrator, indesign, photoshop, premiere | Adobe Pen Tool; Adobe Quick Pen Tool; Pen; Pen Tool |
| Planar Paint Bucket | `planar_paint_bucket` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator | Adobe Planar Paintbucket Tool |
| Poly Line | `poly_line` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity | Poly Line Tool |
| Scissors | `scissors` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator, indesign | Adobe Scissors Tool; Scissors Tool |
| Segment | `segment` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity | Segment Tool |
| Shape Builder | `shape_builder` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity, illustrator | Adobe Shape Builder Tool; Shape Builder Tool |
| Shaper | `shaper` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | illustrator | Adobe Shaper Tool |
| Smooth Path | `smooth_path` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | indesign | Smooth Tool |
| Width | `width` | `draw_path` | WORKSPACE > Tools > Draw and edit paths | affinity, illustrator | Adobe Width Tool; Stroke Width Tool |

`shape` - Shapes, 37 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Arc | `arc` | `shape` | WORKSPACE > Tools > Shapes | illustrator, premiere | Adobe Arc Tool; Arc |
| Arrow | `arrow` | `shape` | WORKSPACE > Tools > Shapes | affinity | Arrow Tool |
| Callout Ellipse | `callout_ellipse` | `shape` | WORKSPACE > Tools > Shapes | affinity | Callout Ellipse Tool |
| Callout Rounded Rectangle | `callout_rounded_rectangle` | `shape` | WORKSPACE > Tools > Shapes | affinity | Callout Rounded Rectangle Tool |
| Cat | `cat` | `shape` | WORKSPACE > Tools > Shapes | affinity | Cat Tool |
| Cloud | `cloud` | `shape` | WORKSPACE > Tools > Shapes | affinity | Cloud Tool |
| Cog | `cog` | `shape` | WORKSPACE > Tools > Shapes | affinity | Cog Tool |
| Crescent | `crescent` | `shape` | WORKSPACE > Tools > Shapes | affinity | Crescent Tool |
| Custom Shape | `custom_shape` | `shape` | WORKSPACE > Tools > Shapes | photoshop | Custom Shape Tool |
| Diamond | `diamond` | `shape` | WORKSPACE > Tools > Shapes | affinity | Diamond Tool |
| Donut | `donut` | `shape` | WORKSPACE > Tools > Shapes | affinity | Donut Tool |
| Double Star | `double_star` | `shape` | WORKSPACE > Tools > Shapes | affinity | Double Star Tool |
| Ellipse | `ellipse` | `shape` | WORKSPACE > Tools > Shapes | affinity, illustrator, indesign, photoshop, premiere | Adobe Ellipse Shape Tool; Ellipse; Ellipse Tool; Oval Tool |
| Flare | `flare` | `shape` | WORKSPACE > Tools > Shapes | illustrator | Adobe Flare Tool |
| Grid | `grid` | `shape` | WORKSPACE > Tools > Shapes | affinity | Grid Tool |
| Heart | `heart` | `shape` | WORKSPACE > Tools > Shapes | affinity | Heart Tool |
| Line | `line` | `shape` | WORKSPACE > Tools > Shapes | affinity, illustrator, indesign, photoshop, premiere | Adobe Line Tool; Line; Line Tool |
| Parametric Mesh | `parametric_mesh` | `shape` | WORKSPACE > Tools > Shapes | aftereffects | Mesh / parametric mesh |
| Pie | `pie` | `shape` | WORKSPACE > Tools > Shapes | affinity, premiere | Pie; Pie Tool |
| Polar Grid | `polar_grid` | `shape` | WORKSPACE > Tools > Shapes | illustrator | Adobe Polar Grid Tool |
| Polygon | `polygon` | `shape` | WORKSPACE > Tools > Shapes | affinity, illustrator, indesign, photoshop, premiere | Adobe Shape Construction Regular Polygon Tool; Polygon; Polygon Tool |
| Radial Circle | `radial_circle` | `shape` | WORKSPACE > Tools > Shapes | affinity | Radial Circle Tool |
| Rectangle | `rectangle` | `shape` | WORKSPACE > Tools > Shapes | affinity, illustrator, indesign, photoshop, premiere | Adobe Rectangle Shape Tool; Rectangle; Rectangle Tool |
| Rectangular Grid | `rectangular_grid` | `shape` | WORKSPACE > Tools > Shapes | illustrator | Adobe Rectangular Grid Tool |
| Rounded Rectangle | `rounded_rectangle` | `shape` | WORKSPACE > Tools > Shapes | affinity, illustrator, photoshop | Adobe Rounded Rectangle Tool; Rounded Rectangle Tool |
| Sphere | `sphere` | `shape` | WORKSPACE > Tools > Shapes | affinity | Sphere Tool |
| Spiral | `spiral` | `shape` | WORKSPACE > Tools > Shapes | affinity, illustrator | Adobe Shape Construction Spiral Tool; Spiral Tool |
| Square Star | `square_star` | `shape` | WORKSPACE > Tools > Shapes | affinity | Square Star Tool |
| Star | `star` | `shape` | WORKSPACE > Tools > Shapes | affinity, illustrator | Adobe Shape Construction Star Tool; Star Tool |
| Tear | `tear` | `shape` | WORKSPACE > Tools > Shapes | affinity | Tear Tool |
| Three Point Circle | `three_point_circle` | `shape` | WORKSPACE > Tools > Shapes | affinity | Three Point Circle Tool |
| Three Point Polygon | `three_point_polygon` | `shape` | WORKSPACE > Tools > Shapes | affinity | Three Point Polygon Tool |
| Three Point Rectangle | `three_point_rectangle` | `shape` | WORKSPACE > Tools > Shapes | affinity | Three Point Rectangle Tool |
| Trapezoid | `trapezoid` | `shape` | WORKSPACE > Tools > Shapes | affinity | Trapezoid Tool |
| Triangle | `triangle` | `shape` | WORKSPACE > Tools > Shapes | affinity | Triangle Tool |
| Two Point Circle | `two_point_circle` | `shape` | WORKSPACE > Tools > Shapes | affinity | Two Point Circle Tool |
| Two Point Rectangle | `two_point_rectangle` | `shape` | WORKSPACE > Tools > Shapes | affinity | Two Point Rectangle Tool |

`type` - Type and tables, 18 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Area Type | `area_type` | `type` | WORKSPACE > Tools > Type and tables | illustrator, indesign | Adobe Area Type Tool; ME Type Tool |
| Artistic Text | `artistic_text` | `type` | WORKSPACE > Tools > Type and tables | affinity | Artistic Text Tool |
| Baseline Grid | `baseline_grid` | `type` | WORKSPACE > Tools > Type and tables | affinity | Baseline Grid Tool |
| Edit Text | `edit_text` | `type` | WORKSPACE > Tools > Type and tables | affinity | Edit Text Tool |
| Frame Grid | `frame_grid` | `type` | WORKSPACE > Tools > Type and tables | indesign | FrameGrid Horz Tool; Horizontal Grid Tool |
| Frame Text | `frame_text` | `type` | WORKSPACE > Tools > Type and tables | affinity | Frame Text Tool |
| Path Type | `path_type` | `type` | WORKSPACE > Tools > Type and tables | illustrator, indesign | Adobe Path Type Tool; ME Path Tool; ME Type on a Path Tool; Path Type Horz Tool; Path Type ME Tool; Type on a Path Tool |
| Spelling | `spelling` | `type` | WORKSPACE > Tools > Type and tables | affinity | Spelling Tool |
| Table | `table` | `type` | WORKSPACE > Tools > Type and tables | affinity | Table Tool |
| Text Flow | `text_flow` | `type` | WORKSPACE > Tools > Type and tables | affinity | Text Flow Tool |
| Touch Type | `touch_type` | `type` | WORKSPACE > Tools > Type and tables | illustrator | Adobe Touch Type Tool |
| Type | `type` | `type` | WORKSPACE > Tools > Type and tables | illustrator, indesign, photoshop, premiere | Adobe Type Tool; Horizontal Type Tool; Type; Type Tool |
| Type Mask | `type_mask` | `type` | WORKSPACE > Tools > Type and tables | photoshop | Horizontal Type Mask Tool |
| Vertical Area Type | `vertical_area_type` | `type` | WORKSPACE > Tools > Type and tables | illustrator | Adobe Vertical Area Type Tool |
| Vertical Frame Grid | `vertical_frame_grid` | `type` | WORKSPACE > Tools > Type and tables | indesign | FrameGrid Vert Tool; Vertical Grid Tool |
| Vertical Path Type | `vertical_path_type` | `type` | WORKSPACE > Tools > Type and tables | illustrator, indesign | Adobe Vertical Path Type Tool; Path Type Vert Tool; Vertical Type on a Path Tool |
| Vertical Type | `vertical_type` | `type` | WORKSPACE > Tools > Type and tables | illustrator, indesign, photoshop, premiere | Adobe Vertical Type Tool; Vertical Type; Vertical Type Tool |
| Vertical Type Mask | `vertical_type_mask` | `type` | WORKSPACE > Tools > Type and tables | photoshop | Vertical Type Mask Tool |

`fill_color` - Fill, gradient and colour, 11 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Color Picker | `color_picker` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity, illustrator, indesign, photoshop | Adobe Eyedropper Tool; Color Picker Tool; Color Sampler Tool; Eyedropper Tool; Sampler Tool |
| Color Theme | `color_theme` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | indesign | Color Theme Tool |
| Fill | `fill` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity | Fill Tool |
| Flood Fill | `flood_fill` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity, photoshop | Flood Fill Tool; Paint Bucket Tool |
| Gradient | `gradient` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity, illustrator, indesign, photoshop | Adobe Gradient Vector Tool; Gradient Swatch Tool; Gradient Tool |
| Gradient Feather | `gradient_feather` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | indesign | Gradient Feather Tool |
| Line Fill | `line_fill` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity | Line Fill Tool |
| Mesh | `mesh` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | illustrator | Adobe Mesh Editing Tool |
| Style Picker | `style_picker` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity | Style Picker Tool |
| Transparency | `transparency` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity | Transparency Tool |
| Vector Flood Fill | `vector_flood_fill` | `fill_color` | WORKSPACE > Tools > Fill, gradient and colour | affinity | Vector Flood Fill Tool |

`crop_frame` - Crop, frame and page, 18 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Artboard | `artboard` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity, photoshop | Artboard Tool |
| Bleed | `bleed` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity | Bleed Tool |
| Content Collector | `content_collector` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | indesign | Content Collector Tool |
| Content Placer | `content_placer` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | indesign | Content Placer Tool |
| Crop | `crop` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity, illustrator, indesign, lightroom_classic, photoshop | Adobe Crop Tool; Crop Overlay Tool; Crop Tool |
| Data Merge Layout | `data_merge_layout` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity | Data Merge Layout Tool |
| Ellipse Frame | `ellipse_frame` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | indesign | Ellipse Frame Tool |
| Gap | `gap` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | indesign | Gap Tool |
| Guides | `guides` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity | Guides Tool |
| Layout | `layout` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity | Layout Tool |
| Perspective Crop | `perspective_crop` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | photoshop | Perspective Crop Tool |
| Picture Frame Ellipse | `picture_frame_ellipse` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity, indesign | Oval Frame Tool; Picture Frame Ellipse Tool |
| Picture Frame Polygon | `picture_frame_polygon` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | indesign | Polygon Frame Tool |
| Picture Frame Rectangle | `picture_frame_rectangle` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity, indesign, photoshop | Frame Tool; Picture Frame Rectangle Tool; Rectangle Frame Tool |
| Place | `place` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity | Place Tool |
| QR Code | `qr_code` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity | QR Code Tool |
| Slice | `slice` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity, illustrator, photoshop | Adobe Slice Tool; Slice Tool |
| Vector Crop | `vector_crop` | `crop_frame` | WORKSPACE > Tools > Crop, frame and page | affinity | Vector Crop Tool |

`transform` - Transform and warp, 42 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Affine | `affine` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Affine Tool |
| Bloat | `bloat` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Bloat Tool |
| Crystallize | `crystallize` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Cyrstallize Tool |
| Deform | `deform` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Deform Tool |
| Displacement Map | `displacement_map` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Displacement Map Tool |
| Equation Transform | `equation_transform` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Equation Transform Tool |
| Equirectangular Projection | `equirectangular_projection` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Equirectangular Projection Tool |
| Fade | `fade` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Fade Tool |
| Free Transform | `free_transform` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator, indesign | Adobe Free Transform Tool; Free Transform Tool |
| Liquify | `liquify` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Tool |
| Liquify Freeze | `liquify_freeze` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Freeze Tool |
| Liquify Mesh Clone | `liquify_mesh_clone` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Mesh Clone Tool |
| Liquify Push | `liquify_push` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Push Forward Tool; Liquify Push Left Tool |
| Liquify Reconstruct | `liquify_reconstruct` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Reconstruct Tool |
| Liquify Thaw | `liquify_thaw` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Thaw Tool |
| Liquify Turbulence | `liquify_turbulence` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Turbulence Tool |
| Mesh Warp | `mesh_warp` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Mesh Warp Tool |
| Mirror | `mirror` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Mirror Tool |
| Move | `move` | `transform` | WORKSPACE > Tools > Transform and warp | affinity, indesign, photoshop | Move Tool; Position Tool |
| Normals | `normals` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Normals Tool |
| Pan Behind | `pan_behind` | `transform` | WORKSPACE > Tools > Transform and warp | aftereffects | Pan Behind (Anchor Point) Tool |
| Perspective | `perspective` | `transform` | WORKSPACE > Tools > Transform and warp | affinity, lightroom_classic | Perspective Tool; Transform Geometry Tool |
| Perspective Grid | `perspective_grid` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Perspective Grid Tool |
| Perspective Projection | `perspective_projection` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Perspective Projection Tool |
| Pinch Punch | `pinch_punch` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Liquify Pinch Tool; Liquify Punch Tool; Pinch / Punch Tool |
| Point Transform | `point_transform` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Point Transform Tool |
| Pucker | `pucker` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Pucker Tool |
| Puppet Overlap | `puppet_overlap` | `transform` | WORKSPACE > Tools > Transform and warp | aftereffects | Puppet Overlap Pin Tool |
| Puppet Pin | `puppet_pin` | `transform` | WORKSPACE > Tools > Transform and warp | aftereffects | Puppet Position Pin Tool |
| Puppet Starch | `puppet_starch` | `transform` | WORKSPACE > Tools > Transform and warp | aftereffects | Puppet Starch Pin Tool |
| Reflect | `reflect` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Reflect Tool |
| Reshape | `reshape` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Reshape Tool |
| Resize | `resize` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Resize Tool |
| Ripple | `ripple` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Ripple Tool |
| Rotate | `rotate` | `transform` | WORKSPACE > Tools > Transform and warp | affinity, illustrator, indesign, premiere | Adobe Rotate Tool; Rotate; Rotate Tool |
| Scale | `scale` | `transform` | WORKSPACE > Tools > Transform and warp | affinity, illustrator, indesign | Adobe Scale Tool; Scale Tool |
| Scallop | `scallop` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Scallop Tool |
| Shear | `shear` | `transform` | WORKSPACE > Tools > Transform and warp | affinity, illustrator, indesign | Adobe Shear Tool; Shear Tool |
| Transform Focal Point | `transform_focal_point` | `transform` | WORKSPACE > Tools > Transform and warp | affinity | Transform Focal Point Tool |
| Twirl | `twirl` | `transform` | WORKSPACE > Tools > Transform and warp | affinity, illustrator | Adobe New Twirl Tool; Liquify Twirl Tool; Twirl Tool |
| Warp | `warp` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Warp Tool |
| Wrinkle | `wrinkle` | `transform` | WORKSPACE > Tools > Transform and warp | illustrator | Adobe Wrinkle Tool |

`measure_annotate` - Measure and annotate, 8 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Alignment | `alignment` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | affinity, photoshop | Alignment Tool |
| Area | `area` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | affinity | Area Tool |
| Convert Document | `convert_document` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | affinity | Convert Document Tool |
| Count | `count` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | photoshop | Count Tool |
| Macro | `macro` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | affinity | Macro Tool |
| Measure | `measure` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | affinity, illustrator, indesign, photoshop | Adobe Measure Tool; Measure Tool; Ruler Tool |
| Note | `note` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | indesign, photoshop | Note Tool |
| Properties | `properties` | `measure_annotate` | WORKSPACE > Tools > Measure and annotate | affinity | Properties Tool |

`mask_channel` - Mask and channel, 9 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Apply Image | `apply_image` | `mask_channel` | WORKSPACE > Tools > Mask and channel | affinity | Apply Image Tool |
| Linear Gradient Mask | `linear_gradient_mask` | `mask_channel` | WORKSPACE > Tools > Mask and channel | lightroom_classic | Linear Gradient Mask Tool |
| Mask Erase | `mask_erase` | `mask_channel` | WORKSPACE > Tools > Mask and channel | affinity | Mask Erase Tool |
| Mask Gradient | `mask_gradient` | `mask_channel` | WORKSPACE > Tools > Mask and channel | affinity | Mask Gradient Tool |
| Mask Paint | `mask_paint` | `mask_channel` | WORKSPACE > Tools > Mask and channel | affinity | Mask Paint Tool |
| Radial Gradient Mask | `radial_gradient_mask` | `mask_channel` | WORKSPACE > Tools > Mask and channel | lightroom_classic | Radial Gradient Mask Tool |
| Raster Matte | `raster_matte` | `mask_channel` | WORKSPACE > Tools > Mask and channel | affinity | Raster Matte Tool |
| Source Image Mask | `source_image_mask` | `mask_channel` | WORKSPACE > Tools > Mask and channel | affinity | Add To Source Image Mask Tool; Erase From Source Image Mask Tool |
| Transform Source Image | `transform_source_image` | `mask_channel` | WORKSPACE > Tools > Mask and channel | affinity | Transform Source Image Tool |

`adjustment_live` - Live adjustment tools, 36 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Black And White Adjustment | `black_and_white_adjustment` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Black And White Adjustment Tool |
| Brightness Contrast | `brightness_contrast` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Brightness / Contrast Tool |
| Channel Mixer | `channel_mixer` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Channel Mixer Tool |
| Clarity | `clarity` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Clarity Tool |
| Color Balance | `color_balance` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Color Balance Tool |
| Color Overlay Filter Effect | `color_overlay_filter_effect` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Color Overlay Filter Effect Tool |
| Colorize | `colorize` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Colorize Tool |
| Curves Adjustment | `curves_adjustment` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity, lightroom_classic | Curves Adjustment Tool; Tone Curve Tool |
| Develop | `develop` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Develop Tool |
| Develop Fits | `develop_fits` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Develop FITS Tool |
| Exposure Adjustment | `exposure_adjustment` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Exposure Adjustment Tool |
| Gradient Map | `gradient_map` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Gradient Map Tool |
| Haze Removal | `haze_removal` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Haze Removal Tool |
| HSL Shift | `hsl_shift` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | HSL Shift Tool |
| Invert | `invert` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Invert Tool |
| Lens Correction | `lens_correction` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Lens Correction Tool |
| Lens Distortion | `lens_distortion` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Lens Distortion Tool |
| Lens Filter | `lens_filter` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Lens Filter Tool |
| Levels | `levels` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Levels Tool |
| LUT | `lut` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | LUT Tool |
| OCIO | `ocio` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | OCIO Tool |
| Posterize | `posterize` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Posterize Tool |
| Recolor | `recolor` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Recolor Tool |
| Remove Vignette | `remove_vignette` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Remove Vignette Tool |
| Selective Color | `selective_color` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Selective Color Tool |
| Shadows Highlights | `shadows_highlights` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Shadows / Highlights Tool |
| Soft Proof | `soft_proof` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Soft Proof Tool |
| Split Toning | `split_toning` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Split Toning Tool |
| Threshold | `threshold` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Threshold Tool |
| Tone Compression Adjustment | `tone_compression_adjustment` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Tone Compression Adjustment Tool |
| Tone Map | `tone_map` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Tone Map Tool |
| Tone Stretch Adjustment | `tone_stretch_adjustment` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Tone Stretch Adjustment Tool |
| Vibrance | `vibrance` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Vibrance Tool |
| Vignette | `vignette` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | Vignette Tool |
| White Balance | `white_balance` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity, lightroom_classic | White Balance Selector Tool; White Balance Tool |
| White Balance Adjustment | `white_balance_adjustment` | `adjustment_live` | WORKSPACE > Tools > Live adjustment tools | affinity | White Balance Adjustment Tool |

`filter_live` - Live filter tools, 44 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| 3D Filter Effect | `3d_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | 3D Filter Effect Tool |
| Bevel Emboss Filter Effect | `bevel_emboss_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Bevel / Emboss Filter Effect Tool |
| Bilateral Blur | `bilateral_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Bilateral Blur Tool |
| Bloom | `bloom` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Bloom Tool |
| Box Blur | `box_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Box Blur Tool |
| Camera Shake Blur | `camera_shake_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Camera Shake Blur Tool |
| Custom Blur | `custom_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Custom Blur Tool |
| Depth Of Field | `depth_of_field` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Depth of Field Tool |
| Diffuse | `diffuse` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Diffuse Tool |
| Diffuse Glow | `diffuse_glow` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Diffuse Glow Tool |
| Emboss | `emboss` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Emboss Tool |
| Field Blur | `field_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Field Blur Tool |
| Gaussian | `gaussian` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Gaussian Tool |
| Gaussian Blur Filter Effect | `gaussian_blur_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Gaussian Blur Filter Effect Tool |
| Glitch | `glitch` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Glitch Tool |
| Gradient Overlay Filter Effect | `gradient_overlay_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Gradient Overlay Filter Effect Tool |
| Halftone | `halftone` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Halftone Tool |
| High Pass | `high_pass` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | High Pass Tool |
| Inner Glow Filter Effect | `inner_glow_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Inner Glow Filter Effect Tool |
| Inner Shadow Filter Effect | `inner_shadow_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Inner Shadow Filter Effect Tool |
| Inner Shadow Offset | `inner_shadow_offset` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Inner Shadow Offset Tool |
| Lens Blur | `lens_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity, lightroom_classic | Lens Blur Tool |
| Lighting | `lighting` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Lighting Tool |
| Maximum Blur | `maximum_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Maximum Blur Tool |
| Median Blur | `median_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Median Blur Tool |
| Minimum Blur | `minimum_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Minimum Blur Tool |
| Motion Blur | `motion_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Motion Blur Tool |
| Multi Band Sharpen | `multi_band_sharpen` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Multi Band Sharpen Tool |
| Noise | `noise` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Noise Tool |
| Outer Glow Filter Effect | `outer_glow_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Outer Glow Filter Effect Tool |
| Outer Shadow Filter Effect | `outer_shadow_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Outer Shadow Filter Effect Tool |
| Outer Shadow Offset | `outer_shadow_offset` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Outer Shadow Offset Tool |
| Outline Filter Effect | `outline_filter_effect` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Outline Filter Effect Tool |
| Perlin Noise | `perlin_noise` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Perlin Noise Tool |
| Pixelate | `pixelate` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Pixelate Tool |
| Portrait Blur | `portrait_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Portrait Blur Tool |
| Portrait Lighting | `portrait_lighting` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Portrait Lighting Tool |
| Procedural Texture | `procedural_texture` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Procedural Texture Tool |
| Radial Blur | `radial_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Radial Blur Tool |
| Texture | `texture` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Texture Tool |
| Unsharp Mask | `unsharp_mask` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Unsharp Mask Tool |
| Voronoi | `voronoi` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Voronoi Tool |
| Z Map Blur | `z_map_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Z-Map Blur Tool |
| Zoom Blur | `zoom_blur` | `filter_live` | WORKSPACE > Tools > Live filter tools | affinity | Zoom Blur Tool |

`generative_ml` - Generative and model-backed, 7 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Generative Edit | `generative_edit` | `generative_ml` | WORKSPACE > Tools > Generative and model-backed | affinity | Generative Edit Tool |
| Generative Expand | `generative_expand` | `generative_ml` | WORKSPACE > Tools > Generative and model-backed | affinity | Generative Expand Tool |
| Generative Extend | `generative_extend` | `generative_ml` | WORKSPACE > Tools > Generative and model-backed | premiere | Generative Extend |
| Generative Fill | `generative_fill` | `generative_ml` | WORKSPACE > Tools > Generative and model-backed | affinity | Generative Fill Tool |
| Remove Background | `remove_background` | `generative_ml` | WORKSPACE > Tools > Generative and model-backed | affinity | Remove Background Tool |
| Super Resolution | `super_resolution` | `generative_ml` | WORKSPACE > Tools > Generative and model-backed | affinity | Super Resolution Tool |
| Super Resolve Document | `super_resolve_document` | `generative_ml` | WORKSPACE > Tools > Generative and model-backed | affinity | Super Resolve Document Tool |

`timeline_edit` - Timeline edit, 9 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Rate Stretch | `rate_stretch` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Rate Stretch |
| Razor | `razor` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Razor |
| Remix | `remix` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Remix |
| Ripple Edit | `ripple_edit` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Ripple Edit |
| Rolling Edit | `rolling_edit` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Rolling Edit |
| Slide | `slide` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Slide |
| Slip | `slip` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Slip |
| Track Select Backward | `track_select_backward` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Track Select Backward |
| Track Select Forward | `track_select_forward` | `timeline_edit` | WORKSPACE > Tools > Timeline edit | premiere | Track Select Forward |

`symbol_instance` - Symbol and instance, 8 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Symbol Screener | `symbol_screener` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Screener Tool |
| Symbol Scruncher | `symbol_scruncher` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Scruncher Tool |
| Symbol Shifter | `symbol_shifter` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Shifter Tool |
| Symbol Sizer | `symbol_sizer` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Sizer Tool |
| Symbol Spinner | `symbol_spinner` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Spinner Tool |
| Symbol Sprayer | `symbol_sprayer` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Sprayer Tool |
| Symbol Stainer | `symbol_stainer` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Stainer Tool |
| Symbol Styler | `symbol_styler` | `symbol_instance` | WORKSPACE > Tools > Symbol and instance | illustrator | Adobe Symbol Styler Tool |

`data_graph` - Data and graph, 10 tools:

| Tool | tool_id | Family | Menu path | Provenance | Vendor variants recorded |
|---|---|---|---|---|---|
| Area Graph | `area_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Area Graph Tool |
| Bar Graph | `bar_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Bar Graph Tool |
| Charts | `charts` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | CC Charts Tool |
| Column Graph | `column_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Column Graph Tool |
| Line Graph | `line_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Line Graph Tool |
| Pie Graph | `pie_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Pie Graph Tool |
| Radar Graph | `radar_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Radar Graph Tool |
| Scatter Graph | `scatter_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Scatter Graph Tool |
| Stacked Bar Graph | `stacked_bar_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Stacked Bar Graph Tool |
| Stacked Column Graph | `stacked_column_graph` | `data_graph` | WORKSPACE > Tools > Data and graph | illustrator | Adobe Stacked Column Graph Tool |

**[STU-SHL-156] Catalogue integrity.** The row count of [STU-SHL-155] MUST equal the count asserted in [STU-SHL-140], and the per-family row counts MUST equal the member counts asserted in [STU-SHL-141]. A change to either count without a matching change to this table is a spec defect. If the operator decides OD-3 in favour of commands ([STU-SHL-142]), the 80 rows of `adjustment_live` and `filter_live` move to a command catalogue and BOTH counts change together; they are never silently deleted.

---

### 3. The options surface — the two-zone Context Bar

**[STU-SHL-160] One region, two zones.** The Context Bar is ONE region ([STU-SHL-064]) with TWO single-occupant zones. The two zones exist because the question "what will the tool do next" and the question "what is selected" are two different questions and both are real; the captured field splits exactly along that line, with three applications driving the bar from the active TOOL and two driving it from the SELECTION.

| Zone | Position | Driven by | Collapses to |
|---|---|---|---|
| Tool Zone | left, fixed | the active tool | the tool identity chip alone when the tool declares no options |
| Selection Zone | right, contextual | the slot resolver over `slot = "context-bar"` against the current selection ([STU-SHL-057]) | empty when nothing is selected |

**[STU-SHL-161] The split rule (normative).** If changing a value changes what the tool will DO NEXT, it belongs in the Tool Zone. If it changes what is ALREADY on the canvas, it belongs in the Properties panel or the Selection Zone. This one rule decides every placement and MUST be cited in a descriptor's provenance when a control's zone is non-obvious.

**[STU-SHL-162] Tool Zone contents, in declared order.** tool identity chip; tool preset picker (permanent, leftmost after the chip); the tool's declared controls in their declared order; overflow disclosure; reset-to-default.

**[STU-SHL-163] Shared Context Bar contract.**

| Property | Value |
|---|---|
| height | 34px |
| interactive control cap | 12 per zone |
| overflow | a GROUPED FLYOUT, never truncation. One captured application's worst case is 81 context-toolbar strings on a single tool and it solves that with a grouped flyout rather than truncating |
| a tool declaring more than 30 controls | becomes a Properties-panel citizen and mirrors only its top 3 controls in the Tool Zone |
| tab stacks | FORBIDDEN. No captured vendor with a workspace file stacks tabs on the top edge |

**[STU-SHL-164] When a bar becomes a panel.** Once a parameter set exceeds roughly a dozen controls it stops being a bar. The measured shape of the problem: across one captured effect corpus the median is 4 parameters per effect, the 90th percentile is 24, the maximum is 2,385, 26 effects exceed 50, and one colour-grading surface alone carries 98 parameters in 18 sections. A parameter set beyond the cap of [STU-SHL-163] renders as a panel with collapsible groups, and the Tool Zone mirrors only its top 3.

**[STU-SHL-165] Legibility mechanisms (normative).** A dense parameter surface MUST provide all six:

1. **Declared groups as first-class nodes**, generated from `ParamSpec.group_path` — not re-decided by the implementer.
2. **Collapsed by default** where the capture declares it, from `ParamSpec.collapsed_by_default`.
3. **Overflow, never truncation** ([STU-SHL-163]).
4. **Presets before parameters.** The captured corpora supply the preset content: roughly 17,000 preset entries from one application, 455 brushes from another, 325 grading looks from a third.
5. **Search inside the parameter set**, matching `display_name` and `group_path`. This is the one legibility mechanism not copied from a vendor.
6. **Show-only-non-default**, computable with no extra authoring from `ParamSpec.default`, which is populated for 980 parameters in the highest-quality captured source alone.

---

### 4. ParamSpec — the generated numeric contract

**[STU-SHL-170] ParamSpec is generated, never authored twice.** Every numeric, enumerated or bounded parameter in Studio has exactly ONE `ParamSpec`, emitted by the build-time descriptor generator of [STU-MAN-100] from the captured parameter metadata. A call site names a generated id and nothing else. Nobody types a range, a unit, a precision or a default into UI source. The scale that forces this: roughly 14,700 parameters (14,681 counted across the captured corpora) against 362 tools. Hand-configuration at that scale is not viable and would drift from the reference behaviour on the first refresh.

**[STU-SHL-171] ParamSpec fields (normative).** When a UiDescriptor's `kind` is `param` it additionally carries:

| Field | Type | Semantics |
|---|---|---|
| `value_kind` | enum | scalar, integer, boolean, enum, colour, point, percentage, time, angle |
| `hard_min` / `hard_max` | f64, nullable | the LEGAL domain of the value |
| `soft_min` / `soft_max` | f64 | the USEFUL drag range. **Present as fields ALWAYS, even when equal to the hard bounds** ([STU-SHL-172]) |
| `bounds_unknown` | bool | true when no range was recovered; see [STU-SHL-174] |
| `bound_provenance` | enum `{declared, observed, inferred}` | how the bound was obtained; see [STU-SHL-173] |
| `default` | value | the reset target |
| `precision` | integer | decimal places as rendered |
| `unit` / `display_unit` | string | the stored unit and the unit shown, which may differ |
| `display_flags` | set | `PERCENT`, `PIXEL`, `REVERSE` |
| `step_default` / `step_coarse` / `step_fine` | f64 | see [STU-SHL-191] |
| `options[]` / `default_index` | list | the enumerated value list where the parameter carries one |
| `group_path` | path | the declared parameter group this belongs to |
| `collapsed_by_default` | bool | from the captured collapse flag |
| `keyframable` | bool | STATIC and generated ([STU-SHL-177]) |

**[STU-SHL-172] Hard and soft bounds are SEPARATE FIELDS FROM THE FIRST COMMIT.** They MUST NOT be collapsed into one range, even where the two are equal, and even in the first implementation. This is the single most expensive-to-reverse decision in the control design. The measurement: in the highest-quality captured parameter source, 833 parameters declare a hard range, 606 declare a soft range, 606 declare BOTH, and **366 of those 606 — 60.4% — have a soft range NARROWER than the legal one**. A control that knows only one range would be wrong more often than right. Corroborating cases: one colour-grading temperature parameter is hard `[-150, 150]` and soft `[-100, 100]`; another application ships a separate edit range on 11 controls of which 9 differ. Retrofitting a second bound after the widget ships means touching every call site. A CI check MUST assert that both fields are present on every `kind == param` descriptor.

**[STU-SHL-173] `bound_provenance == observed` MUST NOT clamp.** An OBSERVED bound is the minimum and maximum seen across shipped presets, not a declared legal domain. When `bound_provenance` is `observed`, the control uses the observed range as the SOFT range and leaves the HARD range open. Clamping to an observed maximum would forbid legal values that simply never appear in a shipped preset — a silent, permanent capability loss that no test would catch, because nothing would ever try the forbidden value. This MUST be enforced in the `ParamSpec` constructor, not by convention. The known case: 32 bounded brush parameter tags in one captured application are observed across 455 shipped presets, not declared.

**[STU-SHL-174] `bounds_unknown` renders unclamped and reports itself.** A parameter with no recovered range renders as an UNCLAMPED number entry AND writes itself into a coverage-gap report. It MUST NOT invent a range. The honest limit is measured: only 22.7% of one application's numeric controls carry the complete range-plus-unit-plus-default contract, and 79.7% of another's numeric properties carry no range at all. Generation is a large head start, not a finished job.

**[STU-SHL-175] Metadata source precedence (normative, ordered).** When two captures describe the same parameter, the higher rank wins.

| Rank | Source | Why | Coverage |
|---|---|---|---|
| 1 | the compositing effect catalogue's typed parameter records | the only source carrying hard bounds, soft bounds, default, precision, units, display units, display flags, options, group boundaries, collapse flags and the keyframability flag in ONE record | 1,573 records across 208 effects; 833 hard, 606 soft, 366 differing, 980 defaults, 499 precisions, 74 group pairs, 294 collapse flags, 327 cannot-time-vary flags, 157 parameters with option lists |
| 2 | the colour-grading surface | hard and soft bounds per parameter | 98 parameters, 84 hard, 14 soft all differing, 15 collapsible group headers over 18 sections |
| 3 | the vector application's dialog parameters | slider range, separate edit range, unit token, decimal places | 2,109 parameters over 559 layouts; 352 numeric controls, 118 slider ranges, 11 edit ranges of which 9 differ, 166 units, 194 decimal places, 1,177 initial values, 185 enumerated |
| 4 | the video effect catalogue | breadth, thin on bounds | 9,654 rows, 9,411 named, only 615 with min/max, 741 defaults, 135 unit families |
| 5 | the raster application's type library | help strings, few ranges | 820 properties, 663 help strings, 35 ranges, 140 defaults, 43 units; 137 of 172 numeric properties have NO recovered range |
| 6 | brush and adjustment parameters | **CAUTION: OBSERVED, not declared** ([STU-SHL-173]) | 38 parameter tags, 32 with bounds |
| 7 | the photo-develop parameters | observed statistics | 389 parameters over 21 panels, 290 with observed numeric statistics |

**[STU-SHL-176] Field-to-capture mapping (normative).** The generator MUST read these fields from these sources. Four corrections are called out because two earlier deliberation documents got them wrong and a generator built from those documents would read the wrong field.

| ParamSpec field | Read from |
|---|---|
| `hard_min` / `hard_max` | valid-min/valid-max (833); grading min/max (84); slider range (118); type-library constraints (35); video effect min/max (615) |
| `soft_min` / `soft_max` | slider-min/slider-max (**606**, corrected from an earlier figure of 618); grading UI min/max (14); edit range versus slider range (11, of which 9 differ) |
| `default` | 980 + 1,177 + 741 + 84 + 140 across the five sources |
| `precision` | precision (499); decimal places (194) and digits (261) |
| `unit` / `display_unit` | units (72) and display units (54); unit (166); unit family (135); help-string units (43) |
| `display_flags` | display-flag bits: `PERCENT` 73, `PIXEL` 8, plus `REVERSE` and a reserved bit declared in the bit table |
| `options[]` | **157 parameters carry an option list**, totalling 1,047 option strings. 422 option lists were recovered across the string pools. These are THREE different figures and MUST NOT be conflated; `options[]` is fed by the 157. The figure 2,001 cited in two earlier deliberation documents is wrong. |
| `group_path` | group-start / group-end, which are **parameter TYPES (codes 13 and 14), not flags** — 74 pairs; plus 18 grading sections with 15 group headers, and 21 develop panels |
| `collapsed_by_default` | the collapse-twirly flag, which **IS a flag (bit 0x20)** — 294 occurrences |
| `keyframable` | the INVERSE of the cannot-time-vary flag (bit 0x2), 327 occurrences |
| `summary` | see 14.32 [STU-MAN-105] |

The generator MUST read two DIFFERENT mechanisms here: group boundaries are parameter types and the collapse and time-vary properties are flag bits. An earlier reading treated all four as one class.

**[STU-SHL-177] `keyframable` is static and generated.** `ParamSpec.keyframable` is the inverse of the captured cannot-time-vary flag. **1,246 of 1,573 typed parameters — 79.2% — are keyframable.** That number sizes the keyframe work directly and it makes the animated case the MAJORITY case in a composition, not an edge case. `keyframable` is a STATIC property of the parameter; the RUNTIME state of a particular property in a particular document is `TemporalState` ([STU-SHL-200]) and the two MUST NOT be conflated.

---

### 5. ScrubValue — the scrubbable numeric control

**[STU-SHL-185] One widget, instantiated from a ParamSpec.** Every value in Studio is edited through ONE widget, `ScrubValue`, constructed from a generated id: `ScrubValue::new(ids::STUDIO_OPTIONS_PEN_WIDTH)`. Every bound, step, unit, precision, default, option list and keyframability comes from the generated spec. The call site names an id and NOTHING else. There is no signature that accepts a literal range, a literal unit or a literal label, so a hand-typed range is a TYPE ERROR rather than a review comment. This is what makes the control unable to drift from the reference behaviour.

**[STU-SHL-186] Hover.** The cursor becomes a COLUMN resize cursor, deliberately distinct from a splitter's horizontal/vertical resize cursor ([STU-SHL-103]), plus a 2px gutter tint. Both cues are required: a plain number box does not advertise that it is draggable, and the gutter tint is the only cue that works on pen, touch and hover-suppressed contexts where there is no hover pointer at all.

**[STU-SHL-187] Click to type.** A press with under 3px of movement enters TEXT EDIT and selects all. `Enter` or `Tab` commits. `Escape` reverts to the pre-edit value. Blur commits.

**[STU-SHL-188] Press and drag, both axes.** A press with movement scrubs continuously in SCREEN space ([STU-SHL-104]). Both axes contribute: `delta = dx − dy`, so rightward and upward both INCREASE. Summing rather than axis-locking means a diagonal drag works and there is no discontinuity when the direction changes mid-gesture.

**[STU-SHL-189] Wheel.** The wheel over the hovered field steps by `step_default` with no click required, and the field CONSUMES the event. Panel scrollability is guaranteed by the reserved 10px scroll lane and by wheel-over-label ([STU-SHL-105]).

**[STU-SHL-190] Modifiers.** A ScrubValue MUST apply exactly this magnitude map to both the drag gesture and the wheel step, and MUST NOT extend it: `Shift` is COARSE and multiplies the step by 10; `Ctrl` is FINE and divides the step by 10; `Shift+Ctrl` multiplies by 100; `Alt` is excluded and MUST NOT acquire a magnitude meaning, because it is already the duplicate, subtract-from-selection and sample-alternate modifier in every captured application and is load-bearing in the timeline surface where scrubbing matters most. Modifiers MUST be sampled CONTINUOUSLY during the gesture rather than latched at press, and accumulation MUST happen in VALUE space so the value does not jump when the operator changes magnitude mid-drag. No modifier may change which parameter is written, only by how much. The full reasoning, including why the objection to `Ctrl` is defeated by claim-at-press, is in [STU-SHL-106].

**[STU-SHL-191] Sensitivity.** `step_default = max(10^(−precision), soft_range / 200)`, so one 200px drag traverses the SOFT range exactly once. `step_coarse = step_default × 10`, `step_fine = step_default / 10`. A parameter with `bounds_unknown` falls back to a magnitude-relative step derived from the current value.

**[STU-SHL-192] Clamping.** Clamping is ALWAYS to `hard_min` / `hard_max`. Approaching a SOFT bound DECELERATES the drag and parks there; crossing a soft bound requires dragging through a short resistance zone or typing the value. Reaching a HARD clamp shows a visual stop but does NOT end the gesture, so dragging back returns immediately rather than after re-traversing the overshoot. Where `bound_provenance` is `observed`, no hard clamp is applied ([STU-SHL-173]).

**[STU-SHL-193] Formatting.** Rendered at `ParamSpec.precision` with `unit` or `display_unit` appended. The `PERCENT` display flag renders a stored 0..1 as 0..100%. The rendered string and the accessible value string MUST be the SAME string ([STU-MDL-113]).

**[STU-SHL-194] Undo.** One entry per GESTURE, per [STU-SHL-107]. The press, or the first wheel notch, opens a coalescing scope keyed by `author_id`. Intermediate values update the document LIVE but write into the open scope. The scope closes into ONE entry on release, on focus loss, on a modal opening, on `Escape` (which aborts and restores the pre-press value), or 400ms after the last wheel notch. A test MUST drive EACH exit path and assert exactly one entry.

**[STU-SHL-195] Double-click and secondary-click.** Double-click resets to `ParamSpec.default` — free, because defaults are captured for 980, 1,177, 741, 84 and 140 parameters across the five sources. Secondary-click opens: reset to default, copy value, paste value, add or remove a keyframe at the playhead, add or edit an expression, and "what does this do" opening the manual anchor ([STU-SHL-233]).

**[STU-SHL-196] Geometry.** Field body minimum 72 × 24px ([STU-SHL-072]). The field's hit rect is inset 8px from any splitter hit strip ([STU-SHL-103]). The AccessKit bounds reported for the field MUST be its GRAB rect, not its glyph rect ([STU-MDL-112]).

---

### 6. TemporalState

**[STU-SHL-200] Why TemporalState exists and where it lives.** A numeric control over a static number is not sufficient. In a composition almost every numeric field is BOTH scrubbable AND keyframable — 1,246 of 1,573 typed parameters ([STU-SHL-177]) — and any property may be replaced by an EXPRESSION, stored as source text on the property's own value stream rather than in a document-level script store. `ParamSpec.keyframable` is STATIC and generated; `TemporalState` is RUNTIME, per property per document, and is NOT part of the ParamSpec. Every ScrubValue in the product carries a TemporalState whether or not its document has a timeline, because the state is what tells a static field that it IS static.

**[STU-SHL-201] The four states (normative, closed).**

| State | Meaning | Additional fields | Affordance |
|---|---|---|---|
| `Static` | a stored constant | — | the stopwatch control is PRESENT but unlit when the parameter is keyframable, and ABSENT when it is not |
| `Animated` | the value is a function of time; the field renders the EVALUATED value at the playhead | `has_key_at_playhead`, `prev_key_time`, `next_key_time`, `key_count` | a lit stopwatch plus a three-part keyframe navigator: previous key, toggle key at playhead, next key |
| `Expression` | the value is COMPUTED; the field renders the evaluated value and is NOT directly scrubbable | `source`, `error` | an expression badge. The field body does NOT claim a press gesture and the cursor does NOT change to the column resize cursor |
| `AnimatedAndExpression` | both are present; the expression WINS and the keyframes become its input, readable in expression scope | union of the two above | renders as `Expression` with a keyframed-input badge |

**[STU-SHL-202] Scrub behaviour by state (normative).**

| State | On press / on commit |
|---|---|
| `Static`, keyframable, auto-keyframe OFF | changes the constant value; ONE undo entry |
| `Static`, keyframable, auto-keyframe ON | CREATES the first keyframe at the playhead; ONE undo entry containing BOTH the keyframe creation and the value ([STU-SHL-107]) |
| `Static`, not keyframable | changes the constant value; ONE undo entry; no stopwatch is rendered |
| `Animated` | sets the value AT THE PLAYHEAD: modifies the key if one exists there, creates one if not. Still ONE undo entry |
| `Expression` or `AnimatedAndExpression` | **REFUSED AT PRESS** — see [STU-SHL-203] |

Auto-keyframe is a DOCUMENT-LEVEL toggle, reachable at `MOTION > Enable Auto-Keyframe` and as a command id.

**[STU-SHL-203] Scrubbing an expression-driven field is REFUSED AT PRESS.** The press is refused, not silently swallowed and not applied. The refusal is the availability predicate of [STU-SHL-046] applied to a VALUE FIELD: `availability_state = INAPPLICABLE_HERE`, `reason_code = EXPRESSION_DRIVEN`, with remedies "edit expression" and "disable expression". This is what unifies the gating design of 14.30 with the numeric-control requirement: a value field is an element like any other, its state is produced by the one predicate, and the field advertises the refusal BEFORE the gesture starts — the cursor does not change, the body does not claim the press, and the badge is visible. Scrubbing would have to rewrite the expression, which is not what the operator asked for and is not reversible as one undo entry.

The refusal MUST be equally explicit on the model path: a `SetValue` on an expression-driven field is REFUSED WITH ITS REASON, never silently applied ([STU-MDL-115]). If the two paths differ, the model's view of the document and the operator's view diverge, which is precisely the failure the inspector exists to catch.

**[STU-SHL-204] Affordance placement.** The stopwatch and the three keyframe-navigator controls are SEPARATE HIT TARGETS adjacent to the field, NEVER inside the field body, because [STU-SHL-102] restricts drags to designated chrome and [STU-SHL-103] requires an 8px inset. Their addresses are derived from the field's own owner-based address ([STU-MDL-102]):

`studio.panel.<panel>.<param-path>.animate`, `.key.prev`, `.key.toggle`, `.key.next`.

**[STU-SHL-205] Keyframe navigation commands.** Every temporal affordance has a command id and a menu leaf; none is pointer-only. The keyframe navigation and edit set includes previous keyframe, next keyframe and their selected-layer variants; nudge keyframes earlier and later with a coarse variant; select all visible keyframes; deselect all keyframes; paste reversed keyframes; toggle expression; and eight add-or-delete-keyframe-at-current-time commands scoped to position, scale, rotation, opacity, anchor point, audio level, mask shape and mask feather. Their default chords are PROVISIONAL pending [STU-SHL-131].

**[STU-SHL-206] Temporal interpolation vocabulary (normative, closed).** Six members: `Linear`, `Bezier`, `Continuous Bezier`, `Auto Bezier`, `Hold`, `Current Settings`. The default ease influence is 0.16666666666, which appears verbatim in shipped preset keyframe records. Spatial interpolation is NOT specified here and is declared debt ([STU-SHL-132] SD-4).

---

### 7. The single write path

**[STU-SHL-210] `set_param_clamped` (normative).** Every write to a parameter value in Studio goes through ONE function:

```
set_param_clamped(author_id, value, time, intent, source) -> Result<CommitReceipt, Refusal>
```

| Argument | Domain |
|---|---|
| `author_id` | the field's owner-based address ([STU-MDL-102]) |
| `value` | the requested value in stored units |
| `time` | the document time the write applies at; required, never implicit |
| `intent` | `SetConstant` \| `SetAtTime` \| `CreateKey` \| `RefuseExpressionDriven` |
| `source` | `pointer_drag` \| `wheel` \| `text_entry` \| `accesskit` \| `script` |

**[STU-SHL-211] All five sources use the same path.** Pointer drag, wheel, text entry, the accessibility set-value action, and a script or model command ALL call this one function. No caller may have its own clamp, its own rounding, its own undo scope or its own change event. The failure this prevents is specific: if any caller clamps independently, the accessible value and the rendered value will EVENTUALLY disagree — and that disagreement is exactly the failure the out-of-process inspector exists to catch, which it would then be structurally unable to catch, because the value it reads would have been produced by the same divergent path it is trying to verify.

**[STU-SHL-212] What one call produces.** One call produces one clamp, one rounding to `precision`, one document mutation, one entry in the open undo scope (or one discrete entry when `source` is `accesskit`, `text_entry` or `script`), and one change event. `time` is carried into the history entry so an undo of a `SetAtTime` restores the value AT THAT TIME, not the value at the current playhead.

**[STU-SHL-213] Refusals are typed and returned.** A refused write returns a typed `Refusal` carrying the `reason_code` of [STU-SHL-051] and the `remedy_command_id`. It MUST NOT return success, MUST NOT write a history entry, and MUST NOT emit a change event.

---

### 8. Accessibility of value fields

**[STU-MDL-110] Role selection (normative).** `Role::Slider` where a SOFT RANGE EXISTS; `Role::SpinButton` otherwise. An expression-driven field is additionally `read_only`. The rule is "a soft range exists", NOT "a soft range differs from the hard range": the accessible role should describe the INTERACTION available, and a soft range means a drag range exists whether or not its two numbers happen to coincide with the hard range. A role that flips when a capture is refreshed and two bounds coincide is unstable, and instability in a role is exactly what breaks an inspector's assertions across capture revisions. The size of the difference between the two candidate rules is 240 parameters in one captured corpus alone.

**[STU-MDL-111] Exposed properties (normative).** Every ScrubValue node exposes:

| Property | Value |
|---|---|
| `numeric_value` | the value EVALUATED AT THE CURRENT TIME |
| `min_numeric_value` | `hard_min` |
| `max_numeric_value` | `hard_max` |
| `numeric_value_step` | `step_default` |
| `numeric_value_jump` | `step_coarse` |
| `value` | the formatted string EXACTLY as rendered, including unit and decimal places |
| `description` | the unit and, where it differs, the soft range |
| `temporal_state` | one of the four states of [STU-SHL-201] |
| `time` | the playhead the value was evaluated at |
| `keyframed_at_time` | whether a keyframe exists at that playhead |

The last three are the fields [STU-MDL-105] adds to the shell's UI tree node. Without them a test asserting a scrub result on an animated property is UNFALSIFIABLE.

**[STU-MDL-112] The reported bounds MUST be the GRAB rect.** A field's accessibility bounds are its 72 × 24px grab rect, not its glyph rect, so an inspector computing a click point from the reported bounds lands INSIDE the draggable body. A glyph-rect bound would produce clicks that miss the grab area and tests that fail for a reason unrelated to the behaviour under test.

**[STU-MDL-113] Agreement tests (normative).** For every numeric descriptor:

1. assert `format(accessible_value) == rendered_text`;
2. assert that driving the value through the accessibility set-value action yields the CLAMPED expectation and EXACTLY ONE undo entry;
3. for an `Animated` property, assert the value reported at time `T` equals the value the property graph evaluates at `T`;
4. for an `Expression` property, assert that a set-value attempt returns a refusal carrying `EXPRESSION_DRIVEN` and that the document is unchanged.

**[STU-MDL-114] Enumerated parameters.** A parameter carrying `options[]` exposes its option list and its selected index through the accessibility node, and its option tokens MUST be the LITERAL captured tokens, not localised display text. Localisation swaps the descriptor set, never the token ([STU-SHL-246]).

**[STU-MDL-115] Refusal parity.** The operator path and the model path MUST refuse identically. An accessibility or command-API write that the pointer path would refuse MUST be refused with the same `reason_code`, and a write that the pointer path would accept MUST be accepted with the same clamp and the same single history entry.

**[STU-MDL-116] Tool state is inspectable.** The active tool, the active tool group face, the active Task Scope (or null), and the resolved Context Bar zones MUST be readable from the accessibility tree without pixel screen-reading, per [STU-MDL-002]. `studio.tool.active`, `studio.task_scope.active` and `studio.slot.context-bar.resolved` are the addresses.

---

### 9. Obligations

**[STU-SHL-220] Universal command contract.** Every tool, every Task Scope entry and exit, every Context Bar control and every parameter write introduced by this sub-section MUST satisfy [STU-CON-007] in full: model-invokable, parallel-safe through the per-file CRDT and lease path, deterministic, and visually verifiable through the render harness and the accessibility inspector without foreground focus steal. A tool with no model invocation path is a conformance defect under [STU-MDL-006].

**[STU-SHL-221] Parallel-safety granularity.** Concurrent parameter writes MUST use the expected-revision precondition of [STU-SDB-004] at the granularity of the individual PROPERTY, not the whole document. Two model lanes scrubbing two different parameters of one document MUST both succeed; two lanes writing the same property at the same time MUST produce exactly one success and one typed conflict carrying `LEASE_HELD_BY_OTHER_ACTOR`.

**[STU-SHL-222] Validation descriptors.** This sub-section contributes at minimum these `StudioValidationDescriptor` checks (14.24): `param_missing_soft_bounds_field`, `param_bounds_unknown_without_gap_report_row`, `param_observed_bound_used_as_clamp`, `param_accessible_value_disagrees_with_rendered_text`, `param_write_bypassed_single_path`, `param_undo_entries_per_gesture_not_one`, `expression_driven_field_accepted_a_write`, `expression_refusal_differs_between_operator_and_model_path`, `tool_missing_menu_leaf`, `tool_missing_command_id`, `tool_family_not_in_closed_set`, `tool_reachable_from_fewer_than_four_projections`, `task_scope_without_visible_exit`, `layout_preset_altered_tool_availability`, `context_bar_zone_exceeded_control_cap`, `context_bar_truncated_instead_of_overflowing`, `scrub_role_flipped_between_capture_revisions`.

**[STU-SHL-223] Manual obligation.** Every tool, family, Task Scope, Context Bar control and parameter in this sub-section MUST have a UserManual entry per [STU-MAN-001] and MUST be reachable by the four search axes of [STU-MAN-004]. Every closed enumeration here — the 22 families, the four temporal states, the five write sources, the four write intents, the six interpolation members, the nine `value_kind` members, the three `bound_provenance` members — MUST appear in the model-facing manual layer as its LITERAL token list. The generation contract that makes this satisfiable across 362 tools and roughly 14,700 parameters is 14.32.


---

### 10. Microtask Derivation

**[STU-SHL-224] Derivation rule (NORMATIVE).** The tools-and-controls microtask set is derived from this sub-section MECHANICALLY, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. **Each numbered clause of this sub-section**, except the bookkeeping clauses named in [STU-SHL-225]. A clause states a contract, a rule, a structure or an enumeration that can be implemented and PROVEN independently, and it yields one microtask whether or not the sentence carrying it happens to use MUST: a stored contract may be stated in the indicative mood.
2. **Each ROW of a catalogue table** — a table whose FIRST COLUMN names a separate implementable subject rather than a facet of one subject. Each such row is its own microtask, because one microtask reading "362 tools" or "90 panels" is not implementable and would let the work disappear behind a number. The remaining columns of the row are that microtask's acceptance criteria.
3. **Each enumeration table, taken WHOLE** — its members are acceptance criteria of one microtask, not separate microtasks.
4. **Each command, shortcut, binding, preset or template table, taken WHOLE.** Binding a key is not a unit of implementation work and MUST NOT be one microtask per row.
5. **Each parameter table, taken WHOLE**, where the row's seven bound fields are its acceptance criteria.

The catalogue table in this sub-section is the tool catalogue of [STU-SHL-155], whose 362 rows are the 362 tools. Each tool row is its own microtask because each tool is separate implementation work: its own gesture, its own options set, its own `requires` expression, its own menu leaf and its own manual entry. The worked-example table of [STU-SHL-151] is deliberately NOT a catalogue — its rows re-cite tools the catalogue already enumerates, and counting them again would file two microtasks against one tool.

**[STU-SHL-225] Clauses that yield NO microtask.** Exactly one class: the five clauses of this derivation sub-section itself — [STU-SHL-224] through [STU-SHL-228]. **Every other clause in this sub-section yields**, including the obligation clauses, because proving an obligation holds across 362 tools is itself work.

**[STU-SHL-226] An open item still yields a microtask.** As [STU-SHL-118]. A clause blocked on an operator decision or a capture gap yields a microtask whose FIRST acceptance criterion is resolving that dependency. In particular: the `adjustment_live` and `filter_live` disposition ([STU-SHL-142]) is blocked on OD-3, and its 80 catalogue rows remain in the derived set as BLOCKED rather than being dropped — if OD-3 moves them to commands, both the count and the catalogue change together and the rows are re-derived, never deleted ([STU-SHL-156]). The model-backed exclusion boundary ([STU-SHL-154]) is blocked on OD-9 and remains derived. Spatial interpolation ([STU-SHL-206]) is blocked on SD-4 and remains derived. Every tool row carries three declared-unknown inputs — rail group, chord and Task Scope membership — as open inputs of its microtask rather than as invented values ([STU-SHL-155]).

**[STU-SHL-227] Yields index.** Applying [STU-SHL-224] through [STU-SHL-226] to this sub-section yields the counts below. Every count is enumerated from the module text, not estimated.

| Unit group | Clauses | Yields |
|---|---|---|
| Tool registry, families and the four-projection rule | [STU-SHL-140]-[STU-SHL-144] | 6 |
| The Tool Rail, variant disclosure and Task Scopes | [STU-SHL-145]-[STU-SHL-154] | 10 |
| The 362-tool catalogue, one unit per tool | [STU-SHL-155]-[STU-SHL-156] | 364 |
| The two-zone Context Bar and its legibility mechanisms | [STU-SHL-160]-[STU-SHL-165] | 7 |
| ParamSpec: shape, bounds law, provenance, source adapters, field mapping | [STU-SHL-170]-[STU-SHL-177] | 8 |
| ScrubValue interaction contract | [STU-SHL-185]-[STU-SHL-196] | 12 |
| TemporalState, its four states and its behaviour rows | [STU-SHL-200]-[STU-SHL-206] | 9 |
| The single clamped write path | [STU-SHL-210]-[STU-SHL-213] | 4 |
| Parallel-safety granularity, validators and the manual binding | [STU-SHL-220]-[STU-SHL-223] | 4 |
| Value-field accessibility contract | [STU-MDL-110]-[STU-MDL-116] | 8 |
| **Module total** | 70 clauses | **432** |

The single row that is not one-per-clause is the tool catalogue, which contributes 362 catalogue-row units beside the two clauses that own and constrain it. Five enumeration tables — the family register, the Context Bar contract, the four temporal states, the scrub-behaviour-by-state table and the exposed accessibility properties — each contribute one enumeration unit beside their clause, with their members as acceptance criteria.

**[STU-SHL-228] Anchor binding.** As [STU-SHL-119A]. A microtask derived here cites its clause anchor directly, a tool microtask additionally cites its `tool_id`, and binding clears `spec_anchor_status`.
