---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
bundle_status: "staged_draft_not_yet_in_bundle"
module_id: "14-27"
section_id: "14.27"
title: "14.27 Studio -- Compositing & Visual Effects"
supersedes_clause: "[STU-OVR-015]"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 14.27 Compositing & Visual Effects

## 14.27.0 Status, scope and authority

**[STU-CMP-001] Compositing is in scope at professional depth.** [STU-OVR-015] is superseded by [STU-VID-001];
the operator's instruction of 2026-09-04 -- *"full blown video capabilitie for
profesional editors and vfx artists"* -- puts visual-effects compositing inside Studio as a
first-class domain. Section 14 at v02.205 had no compositing domain at all: neither "prototyping /
motion" (14.11) nor the effect stack (14.9) is a compositing system. An effect stack applies
operations to one layer; compositing is about how MANY images combine into one, in what order, with
what alpha, through what mattes, in what colour space, under what camera. That is what this
sub-section specifies.

**[STU-CMP-002] Ownership boundaries.** This sub-section owns the layer compositing model: render
order, blend modes, alpha handling, masks as compositing geometry, track mattes, keying and matte
refinement, 3D layers, cameras, lights, renderers, depth and deep-channel compositing, immersive
projection, motion tracking and stabilization, and time-based compositing operations. It does NOT
own the property tree, keyframes or expressions -- 14.26, which this sub-section consumes. It does
NOT own effect parameter contracts or effect catalogue rows -- 14.9, which this sub-section
references and never duplicates ([STU-SECTION-003]). It does NOT own clip editing -- 14.25. It does
NOT own colour science, profiles, LUTs or grading -- 14.8, which this sub-section binds to.

**[STU-CMP-003] No sidecar authority.** Every enumeration, order, formula and structural contract
below is stated here. Green-room captures are derivation provenance in the accompanying
`.provenance.json` ([STU-SECTION-002] as amended).

**[STU-CMP-004] Compositions and sequences interoperate in both directions.** A `StudioComposition`
([STU-MOT-001]) may be placed as a clip in a `StudioSequence` ([STU-VID-020]); a `StudioSequence`
may be used as a layer source in a composition. Each is opaque to the other's editing operations and
is entered by an explicit navigation command. Neither is a wrapper for the other, and Studio does
NOT implement one in terms of the other: a sequence's non-overlapping-clips-per-track model and a
composition's overlapping-layer-stack model are genuinely different, and forcing either into the
other's shape produces a product that is bad at both.

---

## 14.27.1 The layer compositing model

### 1. Precomposition

[STU-CMP-005] **Precomposition replaces a set of layers with a single layer whose source is a new
composition containing them.** Normative rules:

1. Two variants MUST be offered explicitly and their difference MUST be stated at the point of use:
   **leave attributes** (the new composition is the size of the selection's bounding region and the
   original layers' transforms, effects, masks and blend modes stay on the OUTER layer) and
   **move attributes** (the new composition is the size of the parent and everything moves INSIDE).
   They produce different renders and the choice is not recoverable afterwards.
2. Precomposition is recursive, has no fixed depth limit, and a cycle is a validation error.
3. A precomposition boundary is a RENDER boundary by default: the inner composition renders to a
   raster, and the outer layer's transform, effects and masks apply to that raster. This is the
   behaviour [STU-CMP-016] can suspend.
4. Precompose and its inverse are single reversible history steps.

### 2. Render order

[STU-CMP-010] **The layer render order is normative and is the single most important behavioural
contract in this sub-section.** Two operations that both "apply to the layer" produce different
results depending on their position in this order, and an implementation that gets the order wrong
is wrong in a way that is very hard to diagnose from a rendered frame. For each layer, in order:

1. **Source resolution.** Evaluate the layer's source at its remapped time ([STU-MOT-045]),
   applying stretch, frame blending and the source's own colour transform into the composition's
   working space (14.8).
2. **Masks.** Combine the layer's masks in mask order using their modes ([STU-CMP-025]) into a
   single mask alpha, and multiply it into the layer's alpha.
3. **Effects.** Evaluate the `StudioEffectStack` in stack order ([STU-FX-002]). Effects therefore
   see MASKED content; a blur after a mask blurs the mask's edge, which is why masking before
   effects is the default and why an effect that must see the unmasked source needs the mask moved
   to a precomposition.
4. **Layer styles.** Apply the layer-style set ([STU-FX-132]). Layer styles evaluate AFTER effects
   by default, with an explicit per-style option to evaluate before them; the option exists because
   both orders are needed and neither is universally right.
5. **Transform.** Apply anchor point, scale, rotation, position and the parent chain
   ([STU-MOT-008]), with motion blur sampled per [STU-MOT-004]'s shutter settings.
6. **Opacity.**
7. **Track matte.** Apply the layer's track matte ([STU-CMP-030]), which consumes another layer's
   already-rendered alpha or luminance.
8. **Blend.** Composite the result onto the accumulated backdrop using the layer's blend mode
   ([STU-CMP-020]).

**[STU-CMP-010a]** Layers composite from the BOTTOM of the stack upward ([STU-MOT-009]); the accumulated
backdrop at step 8 is everything below the layer. `render_scope = below` effects ([STU-FX-002]) read
that backdrop and are the sole exception to a layer seeing only itself.

**[STU-CMP-010b]** Two order facts are called out because they are commonly got wrong: masks are
applied BEFORE effects, and the transform is applied AFTER effects. A scale applied to a layer
therefore scales the effect's OUTPUT, not its input, so a blur radius does not scale with the layer
-- which is correct, is surprising, and must be documented in the manual entry for every spatial
effect.

### 3. Adjustment layers

[STU-CMP-015] **An adjustment layer's effects apply to the composited result of everything beneath
it in the stack.** It has no content of its own. Its own masks scope WHERE its effects apply, its
opacity scales HOW MUCH they apply, and its in/out points scope WHEN. An adjustment layer is a
`StudioLayer` with `kind = adjustment` ([STU-MOT-005]), not a special object, so every property,
keyframe and expression mechanism applies to it unchanged. Adjustment layers nest: an adjustment
layer beneath another affects the lower one's already-adjusted result.

### 4. Collapse transformations and continuous rasterization

**[STU-CMP-016] One switch, two meanings by layer kind, and both are normative.**

- On a **composition layer**, it COLLAPSES TRANSFORMATIONS: the precomposition boundary stops being
  a render boundary, and the inner layers' transforms concatenate with the outer layer's rather
  than being applied to a rendered raster. The result is that scaling up a precomposed vector or
  text layer stays sharp, that inner 3D layers can interact with outer 3D space, and that the outer
  layer's blend mode and some effects become unavailable because there is no intermediate raster to
  apply them to. Which operations become unavailable MUST be stated in the UI at the moment the
  switch is thrown, not discovered by rendering.
- On a **shape, text or vector layer**, it CONTINUOUSLY RASTERIZES: the geometry is rasterized after
  the transform instead of before, so scale does not soften edges. The same trade-off applies:
  effects that need a fixed-resolution raster behave differently.

**[STU-CMP-016a]** Collapse is not a quality setting. It changes the render graph, and a specification
that describes it as "better quality" is wrong.

### 5. Blend modes

**[STU-CMP-020] The normative Studio blend-mode set is 38 members.** Recovered as 39 rows of which
one is a menu title; the 38 real members are:

Normal, Dissolve, Dancing Dissolve, Darken, Multiply, Color Burn, Classic Color Burn, Linear Burn,
Darker Color, Add, Lighten, Screen, Color Dodge, Classic Color Dodge, Linear Dodge, Lighter Color,
Overlay, Soft Light, Hard Light, Linear Light, Vivid Light, Pin Light, Hard Mix, Difference,
Classic Difference, Exclusion, Subtract, Divide, Hue, Saturation, Color, Luminosity, Stencil Alpha,
Stencil Luma, Silhouette Alpha, Silhouette Luma, Alpha Add, Luminescent Premul.

**[STU-CMP-020a]** Six of those members are NOT ordinary blend functions and must not be implemented as
if they were:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Member | Behaviour |
|---|---|
| `Stencil Alpha` | The layer's ALPHA cuts every layer beneath it in the composition down to that alpha. It is a stack-wide matte, not a per-pixel blend. |
| `Stencil Luma` | The same, driven by luminance. |
| `Silhouette Alpha` | The inverse: the layer's alpha punches a hole through everything beneath. |
| `Silhouette Luma` | The same, driven by luminance. |
| `Alpha Add` | Adds alpha rather than compositing it, used to remove the seam where two exactly-adjacent semi-transparent edges would otherwise show a line. |
| `Luminescent Premul` | Composites a premultiplied layer so that values above the alpha add light rather than being clipped, preserving hot highlights in glows and lens flares. |

**[STU-CMP-020b]** Four members are `Classic` variants of a neighbour (`Classic Color Burn`,
`Classic Color Dodge`, `Classic Difference`, and the `Dancing Dissolve` / `Dissolve` pair is a
distinct case). Studio ships all of them because they exist in imported material and produce
visibly different results; they are not deduped away. A source-suite name is not carried into the
Studio UI beyond these descriptive words ([STU-SECTION-003]).

**[STU-CMP-020c]** Blend modes are also available at three other scopes and are ONE enumeration, not
four: on an effect stack entry ([STU-FX-008]), on a layer style parameter ([STU-FX-132]), on a shape
layer's fill and stroke operators ([STU-MOT-101]), and per character on an animated text layer
([STU-MOT-093]). 14.8 owns the `StudioBlendMode` primitive; this sub-section states its member set
for the compositing surface.

[STU-CMP-021] **Blending happens in the composition's declared working space and its declared
linearity.** A composition declares a working colour space and whether compositing is LINEAR-LIGHT
([STU-VID-011]'s `allow_linear_compositing`, which the shipped configurations read declare true).
Blend results differ substantially between linear and display-referred compositing -- a screen or an
add in linear light is physically correct and looks different from the same operation on gamma-
encoded values -- so the setting is part of the document, is visible, and is recorded in every
render receipt. It is never a preference.

**[STU-CMP-022] Alpha is explicitly typed as STRAIGHT or PREMULTIPLIED at every boundary.** An
imported media item declares which it is (and where the declaration is absent, Studio asks rather
than guessing, because guessing wrong produces dark or light fringes that are then "fixed" by
degrading the matte). An output module declares which it writes ([STU-MOT-122]). Internal
compositing operates on a single declared convention and converts at the boundary. Remove-colour-
matting is the operation that recovers a straight matte from a premultiplied source given the
background colour it was multiplied against, and it is a channel operation ([STU-CMP-080]), not a
guess.

### 6. Masks

[STU-CMP-025] **A mask is a bezier path on a layer that contributes to the layer's alpha, and a
layer may carry many.** The mask collection is an ordered property group ([STU-MOT-012]) whose
entries each carry four properties, every one of them keyframable: **mask path** (the bezier
geometry itself, animatable vertex by vertex -- this is what makes rotoscoping possible), **mask
feather** (edge softness, with an optional variable-width feather along the path), **mask opacity**,
and **mask expansion** (a positive or negative offset of the path outward or inward, used to choke
or spread a matte without redrawing it).

**[STU-CMP-025a] The normative mask-mode enumeration is seven members**, recovered as eight rows of
which one is a menu title: `none`, `add`, `subtract`, `intersect`, `lighten`, `darken`,
`difference`. Each mask also carries an `inverted` boolean, which is independent of the mode. Masks
combine in ORDER: mask N's mode describes how it combines with the accumulated result of masks 1..N-1,
so reordering masks changes the result and mask order is part of the document. `none` means the path
contributes nothing to alpha and exists only as geometry for an effect to reference ([STU-FX-120a]).

**[STU-CMP-026] A unit-normalisation contract, stated because it is a live trap.** Mask opacity's
value stream declares a range of 0..100 while the stored per-keyframe values observed in shipped
documents are 0..1. The declared stream bound is the PERCENT-DISPLAY form and the stored value is
the normalised form. Studio stores the normalised 0..1 value, carries `unit = percent` with the
percent display flag ([STU-FX-108]), and presents 0..100. An implementation that treats the declared
bound as the storage range will render every imported mask at 100x opacity and clip; an
implementation that treats the stored range as the display range will present every mask as 1
percent. This is the general case of [STU-FX-115]'s encoding rule and it applies to every property
whose declared stream bounds and observed values disagree by a factor of 100.

**[STU-CMP-027] Mask geometry is the same `StudioVectorPath` primitive as 14.5's**, reached through
the same `VectorEngine` trait, with the same pen, add-vertex, delete-vertex and convert-vertex
tools ([STU-MOT-131]). There is no compositing-only path type ([STU-DOC-004]).

### 7. Track mattes

[STU-CMP-030] **A track matte uses ANOTHER layer's rendered alpha or luminance as this layer's
alpha.** `StudioTrackMatte` carries: `source_layer` (a stable `layer_id` reference, [STU-FX-120]),
`channel` (`alpha` | `luma`), `inverted` (bool), and `preserve_source_visibility`
(bool -- whether the matte layer also renders on its own, which by default it does not).

**[STU-CMP-030a] The normative matte selection set is four modes plus off**, derived from the nine
recovered rows after removing the menu title and the two relative-position variants: `no_track_matte`,
`alpha_matte`, `alpha_inverted_matte`, `luma_matte`, `luma_inverted_matte`. The recovered surface
additionally distinguishes the matte layer being ABOVE or BELOW the matted layer -- four further rows
-- because the source model bound the matte to an adjacent stack position. **Studio does not inherit
that constraint.** A track matte references a layer by stable id, so the matte layer may be anywhere
in the stack, may serve several layers at once, and does not move when layers are reordered. The
above/below distinction is recorded here as an IMPORT concern only: an importer resolves the
relative reference to a stable id at import time and the constraint disappears.

[STU-CMP-031] **Track mattes and the stencil/silhouette blend modes are two different mechanisms
and both are required.** A track matte affects ONE layer. A stencil or silhouette blend mode
([STU-CMP-020a]) affects EVERY layer beneath it in the composition. Collapsing them into one
feature loses the second behaviour, which is the standard way to cut a whole stack at once.

---

## 14.27.2 Keying and matte refinement

[STU-CMP-032] **Keying is the extraction of an alpha channel from image content, and Studio ships a
keying family, not a key.** The Studio keying capability set, deduped from 19 recovered keying
entries and 2 matte entries and named natively per [STU-SECTION-003] (the recovered names include
third-party vendor product and prefix strings, which are import keys only):

*Derivation: catalogue table, splits per row; yields 15 microtasks, one per keying or matte capability.*

| Studio capability | What it does |
|---|---|
| Chroma key (primary) | Extracts a matte from a screen colour with screen-colour selection, screen gain, screen balance, despill bias, and separate control over the matte's black point, white point, shrink/grow, softness and contrast, plus a screen pre-blur. This is the professional-grade key and it is the one with 80 recovered parameters; it is not a colour-difference toy. |
| Colour key (simple) | Removes a colour within a tolerance with edge thinning and feathering. Retained because it is cheap and sufficient for graphics. |
| Colour-difference key | Two-matte extraction using the difference between colour channels, combined into one matte with independent black/white point control per partial matte. |
| Colour range key | Builds a matte from a sampled range in a selectable colour space (Lab, YUV, RGB) with add/subtract sampling and fuzziness. |
| Luma key | Extracts on luminance with a threshold, tolerance, edge thinning and feathering. |
| Linear colour key | Matte from RGB, hue or chroma similarity with a similarity/tolerance pair, keeping or dropping the matched range. |
| Difference matte | Matte from the difference between the layer and a clean-plate reference layer, with tolerance and blur. |
| Extract | Matte from a histogram range on a chosen channel, with black/white point and softness. |
| Inner/outer key | Matte from two user-drawn paths bounding the edge, refined by edge thinning and feathering -- the tool for objects with no screen. |
| Key cleaner | Recovers matte detail lost to compression, with an alpha-contrast control and an additional-edge-radius control. |
| Spill suppressor (basic and advanced) | Removes screen colour reflected onto the subject, with hue-range, spill-range, luminance-correction and saturation-correction controls. |
| Matte choker | Chokes and spreads a matte in two passes, each with a geometric softness, choke amount and grey-level softness, to close holes and clean edges. |
| Simple choker | Single-parameter matte choke/spread. |
| Wire removal | Removes thin wires and rigging along a defined line with a thickness and slope. |
| Unmult | Derives alpha from luminance for elements shot on black, such as fire, smoke and light flares. |

**[STU-CMP-032a]** Every one of these is a `StudioLiveFilter` in the `keying` or `matte` category of [STU-FX-126],
carries the full parameter contract of 14.9.1, and appears as a catalogue row in
14.9.3. This sub-section states the capability set and the compositing role; 14.9 states the
parameters. Only 3 of the 21 recovered keying and matte entries have typed parameter records, so
the remainder fall under declared gap [STU-FX-146] and their parameter authoring is explicit scope.

**[STU-CMP-033] A key is a MATTE PIPELINE, not a single effect, and Studio's model says so.** The
normative pipeline is: pre-process (denoise, degrain, blur the screen only), extract (one or more
keys, combined), refine (choke, spread, feather, edge-blur, hole-fill), despill, and re-integrate
(edge colour correction against the new background). Each stage is one or more ordinary effect-stack
entries, so the pipeline is visible, reorderable and maskable like anything else ([STU-FX-003]).
Studio MUST NOT ship a single opaque "key" operation that hides these stages, because every real key
requires per-region treatment.

**[STU-CMP-034] DECLARED GAP -- rotoscoping.** Mask-path animation ([STU-CMP-025]) is the manual
rotoscoping mechanism and it is fully specified. An ASSISTED rotoscoping capability -- propagating a
hand-drawn boundary across frames by tracking edges -- is NOT specified here. No such tool's
parameter surface was recovered. It is a named future capability, recorded as [STU-CMP-100], and
until it is specified an implementer builds the manual path and does not improvise a propagation
algorithm.

---

## 14.27.3 Three-dimensional compositing

### 1. 3D layers

[STU-CMP-035] **A layer's `is_3d` flag promotes its transform to three dimensions and enrols it in
the composition's 3D space.** The promoted transform gains a Z position, X and Y rotation, an
orientation triple, and the material-options property group ([STU-MOT-014]). A 3D layer is still a
flat plane unless a geometry option extrudes it; "3D layer" means "positioned in 3D space", and the
distinction from "3D geometry" must be clear in the manual because it is the most common
misunderstanding of the model.

**[STU-CMP-035a]** 2D and 3D layers coexist in one stack and this creates an ordering rule that MUST be
stated: contiguous runs of 3D layers are sorted by depth and rendered together; a 2D layer, an
adjustment layer, or a layer with a blend mode that requires an intermediate raster BREAKS the run,
and 3D layers on either side of the break cannot intersect or cast shadows across it. This is not a
bug to be fixed; it is a consequence of compositing being a stack. Studio MUST surface a break
visibly in the timeline rather than letting an operator discover it by a shadow that will not
appear.

### 2. Cameras

**[STU-CMP-040] A camera layer defines the view for the 3D layers in its composition.** Its property
group is normative and every member is keyframable:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Property | Role |
|---|---|
| `zoom` | Focal distance in pixels; with the composition size this determines the field of view. Zoom and angle-of-view are two views of one value and both MUST be presented. |
| `depth_of_field` | Master enable for the optical model below. |
| `focus_distance` | Distance at which the image is sharp. A "link focus to layer" convenience is a typed command producing an expression ([STU-MOT-070]), not a hidden binding. |
| `aperture` | Aperture size; larger means shallower depth of field. |
| `blur_level` | A multiplier on the computed blur, so the physical result can be exaggerated or tamed. |
| `focus_area_width` | Width of the in-focus band. |
| `split_blur_level` | Independent blur scaling in front of and behind focus. |
| `iris_shape` | Enumerated aperture shape; drives bokeh geometry. |
| `iris_rotation` | Rotation of the iris shape. |
| `iris_roundness` | Blade curvature. |
| `iris_aspect_ratio` | Anamorphic bokeh. |
| `iris_diffraction_fringe` | Edge diffraction on the bokeh. |
| `highlight_gain` | How much bright highlights bloom into bokeh discs. |
| `highlight_threshold` | The luminance above which that happens. |
| `highlight_saturation` | Saturation of the bloomed highlights. |

**[STU-CMP-040a]** A camera additionally carries a POINT OF INTEREST -- a world-space target the camera
aims at -- which may be enabled or disabled. A one-node camera (orientation only) and a two-node
camera (position plus point of interest) are the same layer with the target enabled or not; they are
not two camera types. The point of interest is a keyframable property and is exposed to expressions.

**[STU-CMP-040b]** The camera navigation tools -- orbit, track XY, track Z ([STU-MOT-131]) -- operate on
the active camera's properties and MUST write real keyframes or values, never a hidden view state,
so that a model can reproduce any camera move a hand can make.

### 3. Lights

**[STU-CMP-045] A light layer illuminates 3D layers that accept lights.** Its property group is
normative:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Property | Role |
|---|---|
| `light_type` | Enumeration: parallel, spot, point, ambient. Parallel and spot are directional and carry a point of interest; ambient is unpositioned. |
| `color` | Light colour, carrying a `StudioColorProfile` ref. |
| `intensity` | May exceed 100 percent and may be negative; a negative light subtracts illumination, which is a real compositing technique and MUST NOT be clamped away. |
| `cone_angle` | Spot cone width. |
| `cone_feather` | Spot edge softness. |
| `falloff_type` | Enumeration selecting the distance-attenuation model. |
| `falloff_start` | Distance at which attenuation begins. |
| `falloff_distance` | Distance over which it completes. |
| `casts_shadows` | Whether this light generates shadows at all. |
| `shadow_darkness` | Shadow opacity. |
| `shadow_diffusion` | Shadow edge softness, which is what makes a shadow read as near or far. |
| `background_visible`, `background_opacity`, `background_blur` | An environment-background contribution, present as three separate properties. |
| `environment_atom` | An environment-light group binding an image-based lighting source. |

**[STU-CMP-045a]** Shadow casting requires agreement between three places: the light's `casts_shadows`,
the casting layer's material `casts_shadows`, and the receiving layer's material `accepts_shadows`.
All three are properties ([STU-MOT-014]), all three default in a way that produces no shadow, and
the UI MUST make the three-way requirement discoverable rather than leaving an operator to toggle
switches at random. Studio surfaces a determinate diagnostic naming which of the three is off.

**[STU-CMP-046] The camera and light property groups carry NO recovered numeric parameter contract,
and that absence is stated rather than filled.** [STU-CMP-040] and [STU-CMP-045] name every property
and its role, which is what was recovered. Neither table carries `hard_min`, `hard_max`, `soft_min`,
`soft_max`, `default`, `unit` or `precision`, because the camera-options and light-options property
groups were recovered as match-name topologies ([STU-MOT-012]) and not as typed parameter records.
For every numeric property in those two tables all seven fields are UNKNOWN. An implementer MUST
author them under 14.9.1 as an explicit act, MUST NOT infer a bound from the property's name, MUST
NOT set a soft bound equal to a hard bound to fill the cell, and MUST NOT clamp to a value observed
in a shipped document as though it had been declared ([STU-FX-106]). Two constraints survive the
absence and are normative: `intensity` is not bounded above at 100 percent and is not bounded below
at zero ([STU-CMP-045]), and `zoom` and angle-of-view are two presentations of one stored value
([STU-CMP-040]). This clause is the camera/light instance of declared gap [STU-CMP-104].

### 4. Depth, deep channels and 3D-render integration

**[STU-CMP-050] Studio consumes 3D-render auxiliary channels as first-class compositing inputs.**
Eight recovered capabilities define the requirement:

*Derivation: catalogue table, splits per row; yields 8 microtasks, one per auxiliary-channel capability.*

| Studio capability | Contract |
|---|---|
| Auxiliary channel extract | Extracts a named auxiliary channel from a multi-channel render -- Z depth, object ID, material ID, texture UV, surface normals, coverage, background RGB, unclamped RGB -- into a viewable, usable layer. |
| Arbitrary-channel extract | Extracts any named channel from a multi-part OpenEXR file by channel name, with explicit black-point and white-point mapping into the working range. This is what makes real EXR compositing possible; a compositor that reads only RGBA cannot do VFX. |
| Channel identifier | Displays the channel names and layer names present in a multi-channel file so an operator or model can discover what is available rather than guessing. |
| Depth matte | Builds a matte from a depth slice at a chosen depth with feathering, so 2D elements can be inserted between rendered 3D objects. |
| ID matte | Builds a matte from object or material ID values with feathering. |
| Cryptomatte | Builds anti-aliased, overlapping-aware mattes from hashed-ID channels by picking objects by name. This is the current field standard for isolating rendered objects and its absence would be conspicuous. |
| Depth-of-field from depth | Applies a lens blur driven by a depth channel rather than by a camera. |
| Depth fog | Attenuates toward a fog colour by depth, with a fog start and end. |

**[STU-CMP-050a]** These capabilities require the import path to preserve arbitrary named channels at
full bit depth. An importer that flattens a multi-part EXR to RGBA silently defeats the whole group,
so preserving named channels is a REQUIREMENT on [STU-VID-050], not an optimisation.

**[STU-CMP-050b]** Stereoscopic viewing support is present in the recovered surface as an anaglyph and
stereo-pair combination capability with per-eye convergence and colour balance. Studio carries it as
a compositing capability, distinct from the immersive projection group below.

### 5. Immersive and projected compositing

[STU-CMP-055] **Immersive compositing is projection-aware, and that is what distinguishes it from
applying the same effect to an equirectangular image.** 21 recovered immersive capabilities divide
into three roles:

Each is computed in the SPHERICAL domain so that the result has no seam at the wrap and no
distortion at the poles. Applying the flat version of any of these to an equirectangular frame
produces a visible seam, which is precisely why they exist as separate capabilities and must not be
deduped into their flat counterparts ([STU-SECTION-003] dedups identical capabilities, not
similarly-named different ones). 20 of the 21 are named below; the twenty-first is recovered as a
count and not as a name, which is an open item under [STU-CMP-123].

*Derivation: catalogue table, splits per row; yields 20 microtasks, one per immersive capability.*

| Studio capability | Role | Contract |
|---|---|---|
| Projection convert | conversion | Converts between declared projection formats. Members of the projection-type enumeration are a declared gap ([STU-CMP-106]). |
| Plane to sphere | conversion | Maps a flat plane into a position on the sphere. |
| Sphere to plane | conversion | Maps a region of the sphere back to a flat plane. |
| Sphere rotate | conversion | Rotates the sphere about the three axes, which is how horizon and heading are set. |
| Mobius zoom | conversion | A projection transform that zooms without breaking the wrap. |
| Immersive blur | effect | Blur computed in the spherical domain. |
| Immersive spherical blur | effect | Blur whose kernel follows the sphere's curvature, distinct from the above. |
| Immersive sharpen | effect | Sharpen in the spherical domain. |
| Immersive denoise | effect | Denoise in the spherical domain. |
| Immersive glow | effect | Glow whose falloff is seam-correct at the wrap. |
| Immersive chromatic aberration | effect | Lens-style channel separation, pole-correct. |
| Immersive chroma leaks | effect | Chroma bleed, pole-correct. |
| Immersive digital glitch | effect | Block and line corruption applied in the spherical domain. |
| Immersive fractal noise | effect | Procedural noise generated on the sphere rather than on the plane. |
| Immersive colour gradients | effect | Gradient generation in the spherical domain. |
| Immersive light leaks | effect | Light-leak overlay, seam-correct. |
| Immersive light rays | effect | Volumetric ray generation from a point on the sphere. |
| Immersive random blocks | effect | Randomised block displacement, seam-correct. |
| Immersive gradient wipe | transition | Gradient-driven wipe in the spherical domain. |
| Immersive iris wipe | transition | Iris wipe in the spherical domain. |

**[STU-CMP-055a]** The composition carries the immersive frame parameters ([STU-VID-018]'s record shape
applies to compositions as well as to sequences), and the viewer offers a headset-free interactive
preview with its own controls. 20 of the 21 capabilities are GPU-registered.

### 6. Renderers

[STU-CMP-060] **A composition declares a RENDERER, and the renderer determines which 3D features
its layers may use.** This is a document-level choice with real consequences, not a preference: the
same layer stack renders differently and exposes different properties under different renderers. The
normative contract:

1. A composition's renderer is part of its settings ([STU-MOT-004]) and is recorded in every render
   receipt.
2. A renderer declares a CAPABILITY SET: whether it supports extruded geometry, bevels, environment
   lighting, reflections, refraction, ray-traced shadows, and which material properties it honours.
3. A property that the active renderer does not honour is displayed INACTIVE with a stated reason,
   never hidden, so an operator can see that the value exists and why it is doing nothing
   ([STU-MOT-014]).
4. Switching renderer is a document mutation, is one history step, and MUST report which properties
   became inactive before it is applied.
5. Every renderer runs behind the `RenderEngine` trait in `studio-engine`; no renderer implementation
   may put a GPU dependency into `handshake_core` ([STU-ARC-002]).
6. The recovered surface ships a classic planar renderer ("layers can be positioned as planes in 3D
   space") plus an advanced renderer with its own options including a quality-sample count bounded
   `hard_min` 1, `hard_max` 300. Which renderers Studio ships, and their exact capability sets, is
   declared gap [STU-CMP-102].

---

## 14.27.4 Tracking and stabilization

[STU-CMP-065] **Tracking produces animation data, and that is the whole point: a track's output is
ordinary keyframes on ordinary properties ([STU-MOT-030]), which any expression can then read.**
Studio's normative tracking capability set:

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per capability.*

| Capability | Contract |
|---|---|
| Point tracking | One or more track points, each with a feature region, a search region and an attach point, analysed forward or backward over a frame range, producing per-frame position keyframes. One point solves position; two solve position, rotation and scale; four solve a corner-pin. |
| Manual correction | A track is editable: an operator or model may reposition a point on any frame and re-analyse from there. A tracker whose output cannot be corrected is unusable on real footage. |
| Apply to target | Applying a track writes keyframes onto a chosen target's chosen properties, with an explicit choice of which properties receive them. It is a normal reversible history step and the written keyframes are ordinary keyframes with no hidden link. |
| Stabilization | The same analysis applied inversely to hold a feature still, with an explicit choice of which axes to stabilize and a smoothing amount for smooth-motion rather than lock-off stabilization. |
| Planar / mask tracking | Tracking a region defined by a mask path, producing an animated mask path ([STU-CMP-025]) rather than a point. This is how a moving surface is matched. |
| 3D camera solve | Analysing footage to recover a camera path and a sparse 3D point cloud, from which a camera layer and target layers can be created. The recovered surface ships this capability; its parameter set was not recovered. |

**[STU-CMP-065a] Tracking is a long-running, cancellable, observable analysis** subject to the
headless and quiet law ([STU-FX-038]): it never opens a foreground window, reports determinate
progress, and is fully drivable by typed command so a model can track a shot without the on-canvas
UI. 18 tracker commands are recovered as the operation-surface target.

**[STU-CMP-065b] DECLARED GAP.** No tracker parameter records were recovered. Feature-region and
search-region sizing, the confidence metric and its threshold, the smoothing model and the solve's
error metric all require deliberate authoring under 14.9.1. Recorded as [STU-CMP-103].

---

## 14.27.5 Time-based compositing

[STU-CMP-070] **Effects that read frames other than the current one are a distinct class and their
contract is different.** An ordinary effect is a function of one frame; these are functions of a
frame RANGE, which changes their caching, their determinism proof and their render cost. The
normative set:

*Derivation: catalogue table, splits per row; yields 9 microtasks, one per capability.*

| Capability | Contract |
|---|---|
| Echo | Composites N earlier or later frames at a stated time offset with a decay and a compositing operator (add, maximum, minimum, screen, composite in front, composite behind). Reads a bounded window. |
| Frame difference | Composites the difference between the current frame and a frame at a stated offset, with a target channel and a pre-blur. |
| Posterize time | Resamples the layer to a lower frame rate, which is how "on twos" animation is produced. Two parameters: the target rate and the phase. |
| Time displacement | Displaces each pixel's SAMPLE TIME by the luminance of a map layer, so different parts of the frame show different moments. Reads an unbounded window and needs an explicit max-displacement bound to be renderable. |
| Time blend | Accumulates a running composite across frames with a stated blend operator, which is stateful across time and therefore MUST declare a deterministic start frame or it is not reproducible ([STU-FX-011]). |
| Wide time | Blends a stated number of frames before and after the current one. |
| Timewarp | Retimes with a speed or a source-frame parameter, using frame sampling, frame mixing or pixel-motion estimation, with motion-blur and matte-channel options. Shares its engine with [STU-MOT-007a] and [STU-VID-031]. |
| Pixel motion blur | Synthesises motion blur from estimated inter-frame motion with a shutter angle and sample count, for footage that has none. |
| Forced motion blur | Applies the composition's shutter model to a layer that would not otherwise blur, with an independent shutter angle and sample count. |

**[STU-CMP-070a] Every time-based effect MUST declare its temporal read window in its descriptor**
([STU-FX-124]), so the render scheduler knows which frames to evaluate and the region-of-interest
re-render of [STU-FX-012b] stays correct. An effect that reads outside its declared window is a
validation failure, not a performance issue.

---

## 14.27.6 Simulation

**[STU-CMP-075] Simulation effects generate elements procedurally over time and are stateful.**
22 capabilities are recovered as a count; 14 are named below. The eight that are recovered but not
named are an open item under [STU-CMP-123] and are not silently dropped.

*Derivation: catalogue table, splits per row; yields 14 microtasks, one per simulation capability.*

| Studio capability | Contract |
|---|---|
| Particle world | A full particle system with birth rate, longevity, producer position and radius, velocity, gravity, resistance, per-particle type, and size, opacity and colour over life, under a physics model. |
| Particle systems (classic) | The earlier particle generator retained because shipped documents reference it; it is a separate identity, not a preset of the above. |
| Shatter | Breaks a layer into pieces along a shatter map, driven by force fields. |
| Card animation | Divides the layer into a card grid whose per-card transform is driven by a gradient layer. |
| Fluid simulation | Fluid motion with reflection and displacement of the underlying image. |
| Wave world | Wave propagation producing a displacement map for other layers to consume. |
| Caustics | Refracted light through a simulated water surface. |
| Foam | Bubble-cluster generation with growth, stickiness and per-bubble rendering. |
| Rain | Falling-rain generation with wind, speed and depth layering. |
| Snow | Falling-snow generation with the same axes as rain. |
| Bubbles | Rising-bubble generation, distinct from foam in that it is not surface-bound. |
| Hair | Strand generation and simulation over a layer's alpha. |
| Scatter | Scatters the source into a field of instances under a distribution. |
| Star burst | Radial particle burst used for starfields and impacts. |

**[STU-CMP-075a] Statefulness is the contract that matters.** A simulation's frame N generally
depends on frame N-1, which means: it MUST declare a deterministic start frame; it MUST produce
identical output for identical parameters and seed ([STU-FX-011]); scrubbing backwards MUST either
re-simulate from the start frame or serve a cached state, and MUST NOT show a different result than
playing forwards; and the render scheduler MUST NOT parallelise across frames for a stateful
simulation. Each of these is a real correctness requirement that a stateless effect does not have,
and a specification that treats simulations as ordinary effects will produce non-reproducible
renders.

**[STU-CMP-075b]** Only 3 of the 22 have typed parameter records; the rest fall under [STU-FX-146].
Several carry a third-party vendor prefix in their recovered names, which is an import key and
never a Studio name ([STU-SECTION-003]).

---

## 14.27.7 Channel and compositing arithmetic

[STU-CMP-080] **Channel operations are the compositor's primitive arithmetic and Studio ships the
full set.** 17 capabilities are recovered as a count; 14 are named below. The three that are
recovered but not named are an open item under [STU-CMP-123].

*Derivation: catalogue table, splits per row; yields 14 microtasks, one per channel operation.*

| Studio capability | Contract |
|---|---|
| Channel source selection | Builds a result by choosing which source channel feeds each output channel. |
| Channel shift | Shifts channels between slots, including alpha. |
| Channel combine | Combines two layers' channels under a stated operator. |
| Layer blend | Blends two layers with a stated mode and a ratio. |
| Channel arithmetic | Applies a stated operator and operand to a chosen channel. |
| Compound arithmetic | Arithmetic between this layer and a second layer. |
| Calculations | Combines this layer's chosen channel with a second layer's chosen channel through a blend mode, with per-input inversion and stretching. |
| Alpha levels | Adjusts alpha with independent input and output levels. |
| Set matte | Sets this layer's matte from another layer's chosen channel, with stretch, invert and premultiply options. |
| Minimax | Morphological minimum or maximum on a chosen channel. |
| Invert | Inverts a chosen channel or a chosen colour space. |
| Solid composite | Composites a solid colour with the source under an opacity and a blend mode in one pass. |
| Remove colour matting | Recovers a straight matte from a premultiplied source given the background colour it was multiplied against ([STU-CMP-022]). |
| Composite | The general composite operator over two inputs. |

**[STU-CMP-080a]** These are where the hard/soft bound distinction and the exact enumerated option lists
of 14.9 matter most, because a channel operator selected by the wrong index silently produces a
plausible-looking wrong result. Their parameter records are in 14.9.4; 9 of the 17 have typed
records.

---

## 14.27.8 The architecture fork: layer-based versus node-based compositing

This group RECORDS a decision. It does not make one.

[STU-CMP-090] **The layer model specified in 14.27.1 through 14.27.7 is NORMATIVE and is fully
derived from real captured behaviour.** A composition is an ordered stack of layers; each layer has
a property tree; render order is [STU-CMP-010]; combination is by blend mode, track matte and
adjustment layer. Every enumeration, order and property group above was read from a shipped
application's own data. An implementer building from this sub-section builds the layer model, and
that model is complete enough to build.

**[STU-CMP-091] The node question is DECLARED SPEC DEBT, and here is the question.** High-end
visual-effects compositing is predominantly NODE-BASED: a directed acyclic graph of operations, in
which an image flows along edges and each node transforms it, rather than a stack in which each
layer carries its own private chain. The two are not a rendering detail and they are not a UI
preference. They are different DOCUMENT MODELS:

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| | Layer model | Node model |
|---|---|---|
| Structure | Ordered stack; each layer's effects are a private chain. | Directed acyclic graph; any output may feed any number of inputs. |
| Reuse of an intermediate result | Requires precomposition ([STU-CMP-005]) or a duplicate layer. | Native: connect the same output to several inputs. |
| Multi-input operations | Awkward: a layer reference parameter ([STU-FX-120]) or a track matte. | Native: a node has as many inputs as it needs. |
| Where a result can be inspected | The composition viewer, plus per-effect preview. | Any edge, by attaching a viewer to it. |
| Undo granularity | Property and stack-order changes. | Graph topology changes as well. |
| Parallelism | Constrained by stack order and by stateful backdrop reads. | The graph's own dependency edges define what can run concurrently, which is a materially better fit for a parallel-agent product. |
| Model steerability | A model edits properties and stack positions. | A model edits a graph -- which is a structure models manipulate well. |
| Discoverability for a non-specialist | Higher. The stack matches how people describe a composite. | Lower. A graph is more powerful and less immediately legible. |

[STU-CMP-092] **OPEN DECISION: whether Studio's compositing document model is layer-based only, or
layer-based with a node-graph surface, or node-based with a layer projection.** The decision is
named, it is open, and it is NOT taken in this sub-section. What is recorded:

1. **Why it is not decided from evidence.** No captured application provides a node-graph reference.
   The green room parsed installed applications, and the compositing application it parsed is
   layer-based. A node model would need its own reference basis, gathered deliberately, and
   inventing one here would be exactly the speculation this specification forbids.
2. **Handshake already has node-graph precedent, and this is the argument for taking the question
   seriously rather than deferring it forever.** The Loom graph surface and the canvas board exist
   in the current shell, with an addressing scheme (`loom://` over workspace and block identity)
   that already treats every document, rich-text block, canvas node and graph node as an addressable
   node. Studio would not be introducing graph infrastructure from nothing; it would be reusing a
   primitive the product already has, which changes the cost calculation substantially.
3. **The closest open-source field reference is a Rust, node-based, GPU-rendered 2D graphics editor**
   (Graphite, node-based, Vello-rendered), which is architecturally adjacent to Studio's own stack
   and is the natural place to start a reference gathering exercise under [GLOBAL-RESEARCH].
4. **The parallel-agent argument is the strongest one on the node side and it is Studio-specific.**
   Studio's defining constraint is that multiple models edit in parallel ([STU-CON-007], 14.17). A
   graph's dependency edges make "these two edits do not interact" a mechanical question. In a
   layer stack the same question requires reasoning about backdrop reads, adjustment-layer extent
   and stack order. That is not decisive, but it is the consideration most specific to this product
   and it should not be lost.
5. **The discoverability argument is the strongest one on the layer side**, together with the fact
   that every piece of captured evidence, every import path, and every one of the 482-plus effect
   identities in 14.9 is expressed in layer terms today.

**[STU-CMP-093] What MUST NOT be assumed while the decision is open.** Three forward-compatibility
constraints bind any implementation built from this sub-section, and they cost little now and save a
rewrite later:

1. **The render graph MUST be an explicit data structure, not the call stack.** [STU-CMP-010]'s
   order is a specification of semantics, not a mandate to implement it as nested function calls. An
   implementation that materialises the per-frame render as an explicit dependency graph can later
   grow a node surface over the same evaluator; one that hard-codes the order into control flow
   cannot.
2. **Every intermediate result MUST be addressable.** A layer's post-mask, post-effect, post-
   transform and post-matte results are the points a node graph would expose as edges. Naming them
   internally is what [STU-FX-012b]'s region-of-interest re-render and the Argus diagnostics need
   anyway.
3. **No clause in this sub-section may be written in a way that requires a layer's inputs to be
   exactly one.** `layer_reference` parameters ([STU-FX-120]) and track mattes already make a layer
   multi-input in practice; the type system must not pretend otherwise.

**[STU-CMP-094] Decision criteria, recorded so the decision can be made rather than re-litigated.**
The question should be resolved against: (a) a gathered node-graph reference basis per
[GLOBAL-RESEARCH], since none exists today; (b) whether the Loom graph surface can carry an image
graph without distorting its own purpose; (c) the parallel-model editing argument in
[STU-CMP-092.4] measured against a concrete conflict scenario rather than in the abstract; (d) the
cost of maintaining two projections if both surfaces ship; and (e) whether import fidelity from
layer-based material is achievable through a node projection without loss. The decision belongs to
the operator and requires a Spec Proposal; it is out of scope for this sub-section, which records
it as [STU-CMP-101].

---

## 14.27.9 Recovered enumeration appendix

[STU-CMP-095] **The seven enumerations recovered by content signature, reproduced verbatim as the
completeness check on this sub-section and on 14.26.** Each was located in a shipped binary by
matching a known member set, so the recovery is evidence-based rather than name-based. Each list
includes one menu-title row that is NOT a member; the counts below state both figures so the
distinction is not lost. Where a Studio clause above states a normative member set, that clause
wins; this appendix records what was read.


**layer_blending_modes** -- Layer blending mode (Mode column). 39 recovered rows, 38 after removing the menu-title row.

Values: Add; Alpha Add; Classic Color Burn; Classic Color Dodge; Classic Difference; Color Burn; Color Dodge; Color; Dancing Dissolve; Darken; Darker Color; Difference; Dissolve; Divide; Exclusion; Hard Light; Hard Mix; Hue; Lighten; Lighter Color; Linear Burn; Linear Dodge; Linear Light; Luminescent Premul; Luminosity; Multiply; Normal; Overlay; Pin Light; Saturation; Screen; Silhouette Alpha; Silhouette Luma; Soft Light; Stencil Alpha; Stencil Luma; Subtract; Vivid Light

**track_matte_modes** -- Track matte type (TrkMat column). 10 recovered rows, 9 after removing the menu-title row.

Values: Alpha Inverted Matte; Alpha Matte; Luma Inverted Matte; Luma Matte; No Track Matte; Matte Layer Above; Matte with Layer Above; Matte Layer Below; Matte with Layer Below

**mask_modes** -- Mask mode. 8 recovered rows, 7 after removing the menu-title row.

Values: Add; Darken; Difference; Intersect; Lighten; None; Subtract

**keyframe_interpolation** -- Keyframe interpolation / assistant. 7 recovered rows, 6 after removing the menu-title row.

Values: Auto Bezier; Bezier; Continuous Bezier; Current Settings; Hold; Linear

**layer_quality** -- Layer quality. 6 recovered rows, 5 after removing the menu-title row.

Values: Best; Bicubic; Bilinear; Draft; Wireframe

**frame_blending** -- Frame blending mode. 4 recovered rows, 3 after removing the menu-title row.

Values: Frame Mix; Off; Pixel Motion

**view_layout** -- Composition viewer layout. 4 recovered rows, 4 after removing the menu-title row.

Values: 1 View; 2 Views; 4 Views; Share View Options

**[STU-CMP-095a]** The composition-viewer layout
enumeration (`1 View`, `2 Views`, `4 Views`, plus a share-view-options toggle) is a VIEWER
capability, not a document property: a compositing viewer MUST support one, two and four
simultaneous views of the same composition with independently settable camera, resolution, channel
display and guide overlays per view, and a shared-options toggle that ties them together. Four
synchronised views of one 3D composition -- front, top, side and active camera -- is how 3D layout
is actually done, and a single-view viewer makes [STU-CMP-035] unusable in practice.

---

## 14.27.10 Declared gaps

**[STU-CMP-100] GAP -- assisted rotoscoping.** Per [STU-CMP-034]. Manual mask-path animation is
specified; boundary propagation is not.

**[STU-CMP-101] OPEN DECISION -- layer versus node compositing model.** Per [STU-CMP-092]. Requires
a gathered reference basis and an operator decision.

**[STU-CMP-102] GAP -- renderer inventory and capability sets.** [STU-CMP-060] specifies the
contract a renderer declaration must satisfy; which renderers Studio ships and what each supports is
not specified.

**[STU-CMP-103] GAP -- tracker parameter records.** Per [STU-CMP-065b].

[STU-CMP-104] **GAP -- parameter records for most keying, matte, 3D-channel, immersive and
simulation capabilities.** 3 of 21 keying/matte, 0 of 8 3D-channel, 0 of 21 immersive and 3 of 22
simulation capabilities have typed parameter records. Every unrecorded one is [STU-FX-146] scope and
must have its parameters authored, not guessed.

**[STU-CMP-105] GAP -- the light falloff-type and iris-shape enumerations.** Both properties are
specified and typed; their member lists were not recovered.

**[STU-CMP-106] GAP -- the immersive projection-type enumeration**, shared with [STU-VID-082].

---

## 14.27.11 Model steerability, GUI, diagnostics and manual obligation

**[STU-CMP-110]** Every panel, control, layer switch, mask, matte assignment, camera, light and visible
state in this sub-section MUST be model-visible and typed-steerable through the Studio command
surface (14.16); MUST be headlessly inspectable, steerable and screenshot-capturable through Argus
with no foreground focus steal (14.20); and MUST ship dual-audience UserManual entries kept
same-change current (14.22). Four obligations are specific to compositing:

1. **The render graph MUST be inspectable as structured data.** For a given frame, the Argus
   diagnostic MUST be able to report, per layer, which stage of [STU-CMP-010] produced what: whether
   masks contributed, whether the effect stack ran, whether a track matte was applied, which blend
   mode composited it, and whether a 3D run was broken by an intervening 2D layer
   ([STU-CMP-035a]). A rendered frame alone cannot answer any of those questions, and without the
   structured answer a compositing bug is diagnosed by guessing.
2. **Every "nothing happened" state MUST have a determinate reason.** A shadow that does not appear
   ([STU-CMP-045a]), a property the renderer ignores ([STU-CMP-060.3]), a collapsed layer whose
   blend mode is unavailable ([STU-CMP-016]), a 3D run broken by a 2D layer, a matte referencing a
   deleted layer, and an effect returning `EFFECT_GPU_UNAVAILABLE` ([STU-FX-018]) each have a named
   result and a visible indication. Silence is not an acceptable outcome.
3. **Keying and tracking MUST be fully drivable headlessly**, including screen-colour selection,
   track-point placement and per-frame correction ([STU-CMP-065]), because these are the operations
   most naturally expressed as on-canvas gestures and therefore the ones most likely to be left
   operator-only by accident.
4. **The manual entry for render order ([STU-CMP-010]) is mandatory and is not optional prose.**
   The two facts in [STU-CMP-010b] -- masks before effects, transform after effects -- explain a
   large fraction of the surprising results a compositor will encounter, and an operator who does
   not know them will conclude the product is broken.

---

## 14.27.12 Microtask Derivation

**[STU-CMP-120] Microtask derivation index.** Applying the shared derivation convention to this
sub-section yields exactly 145 microtasks. The correspondence is NORMATIVE and CLOSED: a microtask
corresponds to a yielding clause or to a table unit as marked, and to nothing else.

Rule 0 -- derivation markers are authoritative. Every table in this sub-section carries an italic
`*Derivation: ...*` marker sentence directly above it stating how many microtasks that table yields.
The marker is normative. A tool that classifies a table differently from its marker has diverged
from this sub-section and MUST be corrected to the marker, not the reverse. The five marker forms
are: parameter table taken whole (1); enumeration table taken whole (1); preset or command table
taken whole (1); catalogue table splitting per row (N); contract table carried into the clause's own
microtask (0). A sixth form, reading aid inside a non-yielding clause, also yields 0.

Rule A -- one microtask per yielding clause. Every numbered clause yields exactly one microtask
EXCEPT the members of the no-yield set of [STU-CMP-120a]. A sub-lettered anchor
([STU-CMP-010a], [STU-CMP-020c], [STU-CMP-055a]) is a clause for this purpose and yields on its own account.

Rule B -- table units, counted from the markers of rule 0. A parameter table is a unit in its own
right even though it sits inside a clause that is also a unit, because its rows are bound-sets that
have to be individually proven; folding it into its clause loses that proof obligation. An
enumeration table is a unit for the same reason, its members being the criteria. A catalogue table
splits because each row names a separately implementable subject -- one keying capability, one
auxiliary channel, one immersive capability, one tracker, one time-based effect, one simulation, one
channel operation. A contract table does not split and is not its own unit: it describes the fields
of the single contract its clause already defines.

Three counts in this sub-section are traps for a tool that reads structurally rather than reading
the markers, and all three are stated here so the ledger can be checked rather than trusted:

1. **The 38-member blend-mode set of [STU-CMP-020] is prose, not a table, and it yields nothing
   beyond its clause.** 14.8 owns the `StudioBlendMode` primitive and [STU-CMP-020c] states the
   enumeration is ONE enumeration shared across four scopes; splitting it per member here would
   double-count against 14.8. The 38 members are acceptance criteria of [STU-CMP-020]'s microtask.
   The six non-ordinary members of [STU-CMP-020a] ARE a table, and it is an enumeration table taken
   whole: those six behaviours are proven together because each one is defined by contrast with the
   ordinary blend path.
2. **The seven mask modes of [STU-CMP-025a] and the five track-matte modes of [STU-CMP-030a] are
   prose member lists inside their clauses and yield nothing beyond those clauses.** The matte modes
   in particular are the cross product of the `channel` and `inverted` fields of one record, not five
   subjects.
3. **The camera and light tables of [STU-CMP-040] and [STU-CMP-045] are PARAMETER tables even though
   every bound in them is UNKNOWN** ([STU-CMP-046]). They are marked as parameter tables because the
   proof obligation is per property, and authoring the seven fields for each property is the work.

**[STU-CMP-120a] The no-yield set: 17 clauses.** Nothing else may be excluded, and a clause not on
this list yields under rule A whether or not it is convenient.
In this list a MEMBER of the set is written in backticks, as `STU-AREA-nnn`, and an anchor written
in brackets, as [STU-AREA-nnn], is a REFERENCE and is not excluded from anything. The two forms
are visually distinct so that a reader and a tool can both count the members without parsing the
surrounding English.

The members:

1. **Supersession.** `STU-CMP-001` records that [STU-OVR-015] is superseded; a spec-state fact, not
   an implementable behaviour.
2. **Ownership and authority.** `STU-CMP-002` (ownership boundaries) and `STU-CMP-003` (no sidecar
   authority) state where authority lives.
3. **Restatement of an obligation every microtask inherits.** `STU-CMP-090`: the layer model of
   14.27.1 through 14.27.7 is normative. That attaches to all 145 microtasks by reference.
4. **Evidence and criteria feeding an open decision recorded elsewhere in this sub-section.**
   `STU-CMP-091` (the layer/node comparison) and `STU-CMP-094` (the decision criteria) are carried as
   acceptance criteria of [STU-CMP-092]'s microtask. They do not yield separately and they do not
   disappear.
5. **Declared-gap register rows whose gap is already stated by a yielding clause.** `STU-CMP-100`,
   `STU-CMP-101`, `STU-CMP-102`, `STU-CMP-103` and `STU-CMP-104` point
   at [STU-CMP-034], [STU-CMP-092], [STU-CMP-060], [STU-CMP-065b] and the per-capability microtasks
   respectively. [STU-CMP-105] and [STU-CMP-106] are NOT register rows in that sense: they are the only statement
   of their gap and they yield.
6. **This derivation section.** `STU-CMP-120`, `STU-CMP-120a`, `STU-CMP-121`, `STU-CMP-122`,
   `STU-CMP-123` and `STU-CMP-124`.

Tables inside a non-yielding clause yield nothing.

**[STU-CMP-121] Microtask content obligation.** A microtask derived under [STU-CMP-120] MUST carry
into its own body: the clause anchor, or the catalogue row and its table; the complete member list of
every enumeration it touches; the full parameter record of every parameter it touches with
`hard_min`, `hard_max`, `soft_min`, `soft_max`, `default`, `unit` and `precision` as SEPARATE fields
and every unknown side left explicitly unknown per [STU-CMP-046]; the position of its behaviour in
the render order of [STU-CMP-010] where it composites; and the determinism obligation
of [STU-CMP-075a] where it is stateful across time. A microtask derived from a clause with a parameter
table MUST carry that table verbatim including every unknown. No microtask may cite the green-room
corpus as its source of truth: the corpus is provenance for HOW a clause was derived, and this
sub-section is the authority ([STU-SECTION-002]).

**[STU-CMP-122] Ledger.**

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Ledger line | Basis | Yields |
|---|---|---|
| Clauses in 14.27 | anchors 001 through 124, sub-lettered anchors included | 73 |
| less the no-yield set | the 17 clauses of [STU-CMP-120a] | -17 |
| **Rule A subtotal** | one microtask per yielding clause | **56** |
| Parameter tables | 2 tables: the camera properties of 040 and the light properties of 045, each taken whole | 2 |
| Enumeration tables | 1 table: the six non-ordinary blend members of 020a | 1 |
| Catalogue: keying and matte capabilities of 032 | one per capability | 15 |
| Catalogue: auxiliary-channel capabilities of 050 | one per capability | 8 |
| Catalogue: immersive capabilities of 055 | one per capability | 20 |
| Catalogue: tracking capabilities of 065 | one per capability | 6 |
| Catalogue: time-based capabilities of 070 | one per capability | 9 |
| Catalogue: simulation capabilities of 075 | one per capability | 14 |
| Catalogue: channel operations of 080 | one per operation | 14 |
| Reading aids in non-yielding clauses | 2 tables: the layer/node comparison of 091 and this ledger | 0 |
| **Rule B subtotal** | table units | **89** |
| **Total microtasks yielded by 14.27** | rule A plus rule B | **145** |

**[STU-CMP-123] An open item or a blocked dependency does NOT remove a microtask.** A clause that
declares a gap, an open decision, an unrecovered enumeration, an unnamed capability or a missing
parameter record still yields its rule-A microtask, and that microtask's FIRST acceptance row MUST
read "the named gap is raised to the operator as a capture request and is NOT closed by an invented
value". The clauses carrying a declared gap or open decision are [STU-CMP-034] (assisted
rotoscoping), [STU-CMP-046] (camera and light parameter contracts), [STU-CMP-060] (renderer
inventory), [STU-CMP-065b] (tracker parameter records), [STU-CMP-092] (layer versus
node), [STU-CMP-105] (the `falloff_type` and `iris_shape` member lists) and [STU-CMP-106] (the immersive
projection-type members).

Three capability families are recovered as a COUNT that exceeds the number of capabilities this
sub-section names, and the difference is an open item rather than a silent loss: immersive 21
recovered against 20 named ([STU-CMP-055]), simulation 22 against 14 ([STU-CMP-075]), and channel
arithmetic 17 against 14 ([STU-CMP-080]). Each shortfall is an acceptance row on its clause's own
rule-A microtask, reading "the unnamed capabilities are recovered from the capture and named, or the
recovered count is corrected". Naming them raises the catalogue table's marker count, which raises
this ledger; the ledger is expected to move when that happens and MUST be updated in the same change.

**[STU-CMP-124] Anchor binding.** A microtask derived from this sub-section cites the clause anchor
directly, and a catalogue microtask additionally cites its row and the table it came from. A
microtask staged before this sub-section landed carries `spec_anchor_status = "PROVISIONAL"`; binding
it to an anchor here clears that status. A microtask that cannot cite an anchor in this sub-section
is out of scope for the compositing domain and MUST be re-derived or retired, not activated.
