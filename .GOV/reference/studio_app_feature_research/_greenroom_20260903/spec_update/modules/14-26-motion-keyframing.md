---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
bundle_status: "staged_draft_not_yet_in_bundle"
module_id: "14-26"
section_id: "14.26"
title: "14.26 Studio -- Motion Graphics, the Property Tree, Keyframing & Expressions"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 14.26 Motion Graphics, the Property Tree, Keyframing & Expressions

## 14.26.0 Status, scope and authority

### 1. Why this sub-section exists

**[STU-MOT-000]** Section 14 at v02.205 offered "prototyping/motion" as one domain, specified in 14.11
as `StudioMotionTimeline`: a docked timeline with one track per animated layer, keyframable
position, scale, rotation and opacity, a shared easing catalogue, motion paths and playback modes.
That is a competent prototyping-animation surface and it is retained. It is NOT a motion-graphics
system. A motion-graphics system requires a composition model, a layer stack with a full property
tree in which ANY property can be keyframed, spatial as well as temporal interpolation, a graph
editor over value and speed, time remapping, text animators with selectors, procedural shape
operators, and an expression language that can replace any property's value with a computed one.
None of those exist in 14.11. This sub-section specifies them.

**[STU-MOT-002] Ownership boundaries.** This sub-section owns compositions, the layer stack, the
property tree, the keyframe model, the graph editor, time remapping, the expression language, text
animators, shape operators and the render/output contract for compositions. It does NOT own clip
editing, sequences or tracks -- 14.25. It does NOT own layer compositing, blending, mattes, keying,
masks-as-mattes, tracking, cameras, lights or 3D rendering -- 14.27, which consumes the property
tree specified here. It does NOT own effect parameters -- 14.9 as replaced. It does NOT own
prototype flows, triggers, actions, overlays, device frames or interactive documents -- 14.11,
which is unchanged.

**[STU-MOT-002a] Relationship to [STU-PRO-028] and the prototyping motion timeline.** They are ONE
engine with two authoring surfaces, exactly as [STU-PRO-040d] already requires of the interactive-
document animation surface. `StudioMotionTimeline` becomes the SIMPLIFIED PROJECTION of the model
specified here: the same `StudioKeyframe` records, the same interpolation types, the same shared
easing catalogue ([STU-PRO-019]), the same motion paths, restricted to a per-layer clip view with
the four common transform properties promoted. A prototype animation authored in 14.11 and a motion
graphic authored here are the same data. There is no second keyframe format, no second easing set,
and no conversion step.

**[STU-MOT-003] No sidecar authority.** Every structure, enumeration, default, range and evaluation
rule below is stated here. The green-room captures are derivation provenance recorded in the
accompanying `.provenance.json` ([STU-SECTION-002] as amended).

---

## 14.26.1 Compositions, the layer stack and layer model

### 1. The composition

[STU-MOT-001] **`StudioComposition` (schema id `hsk.studio.composition@1`) is the motion document
primitive.** A composition is a timed, sized, colour-managed container holding an ORDERED STACK of
`StudioLayer` nodes, each with a property tree ([STU-MOT-010]). It is a member of the unified
`StudioDocument` ([STU-DOC-001]) and shares one selection, one history, one colour pipeline and one
export surface with every other Studio surface. A composition MAY be used as a layer source inside
another composition (precomposition, [STU-CMP-005]), MAY be placed as a clip in a sequence
([STU-VID-030]), and MAY be packaged with exposed controls as a motion-graphics template
([STU-VID-070]).

**[STU-MOT-004] The normative composition settings record.** Every bound and default below is read
from the shipped composition-settings surface and carries the full [STU-FX-103] parameter contract,
with the seven fields SEPARATE as [STU-FX-105] requires.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Field | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | Contract |
|---|---|---|---|---|---|---|---|---|---|
| `name` | string | -- | -- | -- | -- | -- | -- | -- | Required, unique within its parent folder. |
| `width` | integer | 4 | 30000 | -- | -- | 720 | pixels | -- | |
| `height` | integer | 4 | 30000 | -- | -- | 480 | pixels | -- | |
| `lock_aspect_ratio` | boolean | -- | -- | -- | -- | false | -- | -- | When true, editing one dimension scales the other by the locked ratio. |
| `pixel_aspect_ratio` | rational | -- | -- | -- | -- | 1:1 | ratio | -- | Rational, never a float ([STU-VID-011]). |
| `frame_rate` | scalar | 1 | 999 | -- | -- | 29.97 | fps | -- | Presented alongside a preset picker offering 8, 12, 15, 23.976, 24, 25, 29.97, 30, 50, 59.94, 60, 120. Stored per [STU-MOT-020]. |
| `drop_frame` | enum | -- | -- | -- | -- | -- | -- | -- | Members: `drop_frame` \| `non_drop_frame`. Display only ([STU-VID-012c]). No default declared. |
| `resolution` | enum | -- | -- | -- | -- | -- | -- | -- | Members: `full` \| `half` \| `third` \| `quarter` \| `custom`. A preview-quality divisor; does NOT change `width`/`height`. No default declared. |
| `start_timecode` | time | -- | -- | -- | -- | 0 | timecode | -- | The composition's zero point in display time. |
| `duration` | time | -- | -- | -- | -- | -- | timecode | -- | Required at creation; no default declared. |
| `background_color` | colour | -- | -- | -- | -- | -- | -- | -- | Carries a `StudioColorProfile` ref ([STU-FX-118]). Not rendered into an alpha-bearing output. |
| `preserve_frame_rate_when_nested` | boolean | -- | -- | -- | -- | false | -- | -- | When true the composition keeps its own frame rate inside a parent or a render queue. |
| `preserve_resolution_when_nested` | boolean | -- | -- | -- | -- | false | -- | -- | |
| `shutter_angle` | scalar | 0 | 720 | -- | -- | 180 | degrees | -- | Motion-blur exposure. 180 degrees is a half-frame exposure. |
| `shutter_phase` | scalar | -360 | 360 | -- | -- | 0 | degrees | -- | Offset of the exposure window relative to the frame. |
| `samples_per_frame` | integer | 2 | 64 | -- | -- | 16 | -- | -- | Motion-blur samples for 3D layers, shape layers and sampling effects. |
| `adaptive_sample_limit` | integer | 2 | 256 | -- | -- | 128 | -- | -- | Upper bound on adaptive per-frame sampling for 2D motion. |
| `renderer` | enum | -- | -- | -- | -- | -- | -- | -- | Members and defaults per [STU-CMP-060]. Selects which 3D feature set the composition's layers may use. |

**[STU-MOT-004b] Reading convention for the table above, and why every `soft_min`/`soft_max` cell
is empty.** `--` means the source declares nothing and Studio declares nothing, exactly
as [STU-FX-131a] defines it for the effect parameter records. The composition-settings surface declares
a VALID range only; it declares no separate control range, so `soft_min` and `soft_max` are UNKNOWN
for every row and are carried as empty rather than as a copy of the hard bound. Setting a soft bound
equal to its hard twin here would assert something the source never said and would be
unrecoverable without re-deriving from the capture ([STU-FX-105], [STU-FX-106]). `precision` is
likewise undeclared on this surface; an integer `Kind` constrains the stored value but is not a
declared decimal count. An implementer who needs a control range chooses one, records it as
`soft_bound_source = "implementation"`, and never promotes it to `hard_*`.

**[STU-MOT-004a]** The composition settings surface is the single place these values live. A render
queue item MAY override a subset of them at render time ([STU-MOT-120]) without mutating the
composition.

### 2. Layer types

**[STU-MOT-005] The normative `StudioLayer.kind` enumeration for compositions is ten members.**
This extends, and does not replace, the `StudioDocument` layer-kind list of [STU-DOC-001]; a
composition layer is a `StudioLayer` whose kind is one of:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Kind | Contract |
|---|---|
| `footage` | An audio/video layer whose source is an imported media item. Carries the full transform, effects, masks and time properties. |
| `composition` | An audio/video layer whose source is another `StudioComposition` (a precomposition). |
| `solid` | A single-colour layer of a declared size. The workhorse carrier for effects that generate rather than filter. |
| `text` | A text layer with a source-text property, path options, more options and an animator collection ([STU-MOT-090]). |
| `shape` | A procedurally-defined vector layer built from shape operators ([STU-MOT-100]). |
| `camera` | A camera. Renders nothing; defines the view for 3D layers ([STU-CMP-040]). |
| `light` | A light. Renders nothing; illuminates 3D layers ([STU-CMP-045]). |
| `null` | An invisible zero-content layer used purely as a parent or an expression target ([STU-MOT-008]). |
| `adjustment` | A layer whose effects apply to everything composited beneath it in the stack rather than to its own content ([STU-CMP-015]). |
| `guide` | A layer visible while authoring and excluded from render by default ([STU-MOT-121]). |

### 3. Layer common properties

**[STU-MOT-006] Every composition layer carries the same common property set**, independent of
kind. These are layer attributes, not property-tree entries, and are addressed separately from [STU-MOT-010]'s
tree.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Attribute | Type | Contract |
|---|---|---|
| `layer_id` | stable id | `SLYR-{uuid_v7}` per [STU-ARC-004]. |
| `index` | u16 | Position in the stack. 1 is topmost. Renumbering on reorder is an implementation detail; `layer_id` is what references use ([STU-FX-120]). |
| `name` | string | Operator-set; defaults to the source name and tracks it until explicitly renamed. |
| `label_color` | enum | Same 16-member palette as [STU-VID-020]. |
| `enabled` | bool | Video output on/off. |
| `audio_enabled` | bool | Independent of `enabled`. |
| `solo` | bool | When any layer in the composition is soloed, only soloed layers render. |
| `shy` | bool | Hidden from the timeline outline (not from the render) when the composition's shy filter is on. |
| `locked` | bool | Refuses selection and edit. |
| `quality` | enum | See [STU-MOT-007]. |
| `in_point`, `out_point` | time | The span over which the layer exists in the composition. |
| `start_time` | time | Where the layer's source time zero sits on the composition timeline. |
| `stretch` | rational | Time-stretch factor; negative reverses. |
| `motion_blur` | bool | Per-layer opt-in; the composition's master motion-blur switch gates all of them. |
| `frame_blending` | enum | See [STU-MOT-007a]. |
| `is_3d` | bool | Promotes the transform group to three dimensions ([STU-CMP-035]). |
| `collapse_or_continuous_rasterize` | bool | On a composition layer, passes transforms through to the nested composition instead of rendering it to a raster first; on a shape or vector layer, rasterizes after transform so scaling stays sharp. One switch, two meanings by layer kind, and BOTH are normative ([STU-CMP-016]). |
| `blend_mode` | enum | Owned by 14.27 ([STU-CMP-020]). |
| `track_matte` | record | Owned by 14.27 ([STU-CMP-030]). |
| `parent` | layer ref \| null | See [STU-MOT-008]. |

[STU-MOT-007] **The layer quality enumeration is normative: `wireframe`, `draft`, `best`, plus two
sampling selections `bilinear` and `bicubic`.** Five members recovered. `wireframe`, `draft` and
`best` select the render fidelity of the layer's own content; `bilinear` and `bicubic` select the
resampling filter used when the layer is transformed. Studio models these as TWO fields --
`quality` (`wireframe` | `draft` | `best`) and `resample_filter` (`bilinear` | `bicubic`) --
because they are orthogonal, and records that the source presented them as one menu.

**[STU-MOT-007a] The frame-blending enumeration is normative: `off`, `frame_mix`, `pixel_motion`.**
`frame_mix` blends adjacent source frames; `pixel_motion` synthesises intermediate frames by motion
estimation. This is the same three-way choice as the clip timeline's frame-interpolation method
([STU-VID-031]) and MUST be one implementation with two entry points ([STU-DOC-004]), not two
retiming engines. `frame_sampling` in [STU-VID-031] is this enumeration's `off`.

### 4. Parenting

**[STU-MOT-008] A layer MAY declare one parent layer, forming a transform hierarchy.** The child's
transform is evaluated in the parent's transform space; changing the parent's position, rotation or
scale moves the child. Normative rules:

1. The hierarchy is a forest. A cycle is a validation error, refused at command time.
2. Setting or clearing a parent MUST offer both behaviours as explicit typed variants: preserve the
   child's world transform by compensating its local values, or keep the local values and let the
   child jump. Choosing silently is forbidden because both are legitimate and the operator's intent
   is not recoverable afterwards.
3. A `null` layer is the idiomatic parent: it has no content, so it exists to be a transform node.
4. Parenting affects the transform group only. It does NOT inherit opacity, effects, masks, blend
   mode or track matte; those are stack-order concerns owned by 14.27.
5. A layer's effective transform is available to expressions through the space-transform functions
   of [STU-MOT-074].

**[STU-MOT-009] Stack order is render order and it is bottom-to-top.** Layer index 1 is topmost in
the outline and is composited LAST. This is stated because it is the opposite of the naive reading
and because [STU-CMP-010] depends on it.

---

## 14.26.2 The property tree

### 1. The core idea

[STU-MOT-010] **`StudioProperty` (schema id `hsk.studio.property@1`) is a value stream, and every
layer carries a TREE of them. Any property in that tree can be keyframed, and any property in that
tree can instead be driven by an expression.** This is the single most important structural claim
in this sub-section, and it is what separates a motion-graphics system from an animation feature.
It is not "position, scale, rotation and opacity are animatable"; it is "every leaf of the property
tree is animatable unless it declares otherwise" ([STU-FX-113]'s `animatable` flag is the
exception mechanism, not the permission mechanism).

**[STU-MOT-010a]** A `StudioProperty` carries:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Contract |
|---|---|
| `property_key` | Stable identity within its group. Never the display name. |
| `display_name` | Localized label. |
| `value_kind` | The `StudioParameterKind` of [STU-FX-112]. A property and an effect parameter are the SAME record type; an effect's parameters are a subtree of the layer's property tree ([STU-MOT-017]). |
| `state` | `static` \| `keyframed` \| `expression_driven` \| `expression_over_keyframes`. See [STU-MOT-071], which carries this four-member enumeration and its state machine. |
| `static_value` | The value when `state = static`, and the pre-expression base when an expression is present. |
| `keyframes` | Ordered `StudioKeyframe` list when keyframed ([STU-MOT-030]). |
| `expression` | UTF-8 source when expression-driven ([STU-MOT-070]). |
| `expression_enabled` | Bool, independent of whether an expression exists. Disabling keeps the source. |
| `dimension_count` | Number of scalar components. Position is 2 or 3; opacity is 1; colour is 4. |
| `dimensions_separated` | Bool. When true each component becomes its own keyframable sub-property with its own interpolation. Separating and rejoining are lossy in opposite directions and MUST be explicit typed commands with a stated conversion, never automatic. |
| `min_value`, `max_value` | The stream's declared bounds, when it has them. These are `hard_min`/`hard_max` under [STU-FX-104] and follow the same `bound_state` discipline. |
| `is_spatial` | Bool. A spatial property additionally carries spatial tangents and renders a motion path ([STU-MOT-036]). |

**[STU-MOT-011] Property addressing is a stable path, and the path is the model surface.** A
property is addressed as `composition_id / layer_id / group_key / ... / property_key`. The path
uses stable keys at every level, never display names and never indices, so renaming a layer, a
group or an effect instance cannot break a reference held by an expression, a preset, a
motion-graphics template's exposed control, or a model-issued command. 1,238 distinct property
identity keys and 759 parent/child containment relationships were recovered from real serialized
documents; the identity keys are reproduced as import keys in [STU-MOT-012] and the containment
relations are the shape Studio's tree must express.

### 2. The recovered property tree

**[STU-MOT-012] The property tree topology below is normative as SHAPE.** Thirteen property-group
topics are declared on composition layers, and each layer kind exposes the subset that applies to
it. The identity keys are import keys ([STU-FX-103]) and never Studio-facing names.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Topic | Root key | Applies to | Owned by |
|---|---|---|---|
| Transform | `ADBE Transform Group` | every layer | [STU-MOT-013] |
| 3D material options | `ADBE Material Options Group` | 3D layers | [STU-MOT-014] |
| 3D geometry options | `ADBE Extrsn Options Group`, `ADBE Plane Options Group` | 3D layers under a 3D renderer | [STU-MOT-014] |
| Camera options | `ADBE Camera Options Group` | camera layers | 14.27 ([STU-CMP-040]) |
| Light options | `ADBE Light Options Group` | light layers | 14.27 ([STU-CMP-045]) |
| Audio | `ADBE Audio Group` | audio-bearing layers | [STU-MOT-016] |
| Masks | `ADBE Mask Parade` / `ADBE Mask Atom` | every raster-bounded layer | 14.27 ([STU-CMP-025]) |
| Effects | `ADBE Effect Parade` | every layer | [STU-MOT-017] |
| Layer styles | `ADBE Layer Styles` | every layer | 14.9 ([STU-FX-132]) |
| Time remap | `ADBE Time Remapping` | footage and composition layers | [STU-MOT-045] |
| Text | `ADBE Text Properties` | text layers | [STU-MOT-090] |
| Shape | `ADBE Root Vectors Group` | shape layers | [STU-MOT-100] |
| Data | `ADBE Data Group` | data-driven layers | [STU-MOT-019] |

Each edge is a parent/child containment observed in real serialized composition documents;
`obs` is the number of documents in which the edge appeared. Only the structural property
groups are expanded here; effect-instance roots all attach the same
`ADBE Effect Built In Params` subtree, which is listed once.


**`ADBE Transform Group`**
  - `ADBE Anchor Point` (obs 616)
  - `ADBE Envir Appear in Reflect` (obs 793)
  - `ADBE Opacity` (obs 481)
  - `ADBE Position` (obs 686)
  - `ADBE Position_0` (obs 793)
  - `ADBE Position_1` (obs 793)
  - `ADBE Position_2` (obs 438)
  - `ADBE Rotate X` (obs 219)
  - `ADBE Rotate Y` (obs 197)
  - `ADBE Rotate Z` (obs 674)
  - `ADBE Scale` (obs 419)

**`ADBE Material Options Group`**
  - `ADBE Accepts Lights` (obs 221)
  - `ADBE Accepts Shadows` (obs 222)
  - `ADBE Ambient Coefficient` (obs 218)
  - `ADBE Appears in Reflections` (obs 358)
  - `ADBE Casts Shadows` (obs 256)
  - `ADBE Diffuse Coefficient` (obs 218)
  - `ADBE Fresnel Coefficient` (obs 358)
  - `ADBE Glossiness Coefficient` (obs 358)
  - `ADBE Index of Refraction` (obs 358)
  - `ADBE Light Transmission` (obs 357)
  - `ADBE Metal Coefficient` (obs 218)
  - `ADBE Reflection Coefficient` (obs 358)
  - `ADBE Shadow Color` (obs 219)
  - `ADBE Shininess Coefficient` (obs 218)
  - `ADBE Specular Coefficient` (obs 218)
  - `ADBE Transp Rolloff` (obs 358)
  - `ADBE Transparency Coefficient` (obs 358)

**`ADBE Extrsn Options Group`**
  - `ADBE Bevel Depth` (obs 15)
  - `ADBE Bevel Direction` (obs 358)
  - `ADBE Bevel Styles` (obs 16)
  - `ADBE Extrsn Depth` (obs 1)

**`ADBE Plane Options Group`**
  - `ADBE Plane Curvature` (obs 8)
  - `ADBE Plane Subdivision` (obs 2)

**`ADBE Camera Options Group`**
  - `ADBE Camera Aperture` (obs 4)
  - `ADBE Camera Blur Level` (obs 347)
  - `ADBE Camera Focus Area Width` (obs 70)
  - `ADBE Camera Focus Distance` (obs 114)
  - `ADBE Camera Split Blur Level` (obs 70)
  - `ADBE Camera Zoom` (obs 116)
  - `ADBE Iris Aspect Ratio` (obs 347)
  - `ADBE Iris Diffraction Fringe` (obs 347)
  - `ADBE Iris Highlight Gain` (obs 347)
  - `ADBE Iris Highlight Threshold` (obs 347)
  - `ADBE Iris Hightlight Saturation` (obs 347)
  - `ADBE Iris Rotation` (obs 347)
  - `ADBE Iris Roundness` (obs 347)
  - `ADBE Iris Shape` (obs 347)

**`ADBE Light Options Group`**
  - `ADBE Casts Shadows` (obs 18)
  - `ADBE Light Backgd Blur` (obs 18)
  - `ADBE Light Backgd Opacity` (obs 18)
  - `ADBE Light Backgd Visible` (obs 18)
  - `ADBE Light Color` (obs 13)
  - `ADBE Light Cone Angle` (obs 16)
  - `ADBE Light Cone Feather 2` (obs 15)
  - `ADBE Light Env Atom` (obs 18)
  - `ADBE Light Falloff Distance` (obs 17)
  - `ADBE Light Falloff Start` (obs 16)
  - `ADBE Light Falloff Type` (obs 13)
  - `ADBE Light Intensity` (obs 14)
  - `ADBE Light Shadow Darkness` (obs 11)
  - `ADBE Light Shadow Diffusion` (obs 12)

**`ADBE Audio Group`**
  - `ADBE Audio Levels` (obs 5)

**`ADBE Mask Parade`**
  - `ADBE Mask Atom` (obs 146)
    - `ADBE Mask Feather` (obs 31)
    - `ADBE Mask Offset` (obs 31)
    - `ADBE Mask Opacity` (obs 31)
    - `ADBE Mask Shape` (obs 40)

**`ADBE Mask Atom`**
  - `ADBE Mask Feather` (obs 31)
  - `ADBE Mask Offset` (obs 31)
  - `ADBE Mask Opacity` (obs 31)
  - `ADBE Mask Shape` (obs 40)

**`ADBE Effect Parade`**
  - `ADBE 4ColorGradient` (obs 1)
  - `ADBE Angle Control` (obs 14)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Aud Compressor` (obs 5)
    - `ADBE Effect Built In Params` (obs 6)
  - `ADBE Aud Modulator` (obs 10)
    - `ADBE Effect Built In Params` (obs 11)
  - `ADBE Aud Reverb` (obs 4)
    - `ADBE Effect Built In Params` (obs 4)
  - `ADBE Aud Tone` (obs 10)
    - `ADBE Effect Built In Params` (obs 4)
  - `ADBE AutoContrast` (obs 1)
  - `ADBE Basic 3D` (obs 1)
  - `ADBE Bevel Alpha` (obs 7)
    - `ADBE Effect Built In Params` (obs 5)
  - `ADBE Block Dissolve` (obs 5)
  - `ADBE Box Blur2` (obs 13)
    - `ADBE Effect Built In Params` (obs 5)
  - `ADBE Bulge` (obs 1)
  - `ADBE CHANNEL MIXER` (obs 1)
  - `ADBE CM Animated Shape Control` (obs 1)
    - `ADBE Effect Built In Params` (obs 4)
  - `ADBE CM AutoscrollHorizontal` (obs 1)
  - `ADBE CM AutoscrollVertical` (obs 1)
  - `ADBE CM CrackedTiles` (obs 1)
  - `ADBE CM CropEdges` (obs 1)
  - `ADBE CM DissolveUnmelt` (obs 1)
  - `ADBE CM FadeInOutFrames` (obs 1)
  - `ADBE CM FadeInOutmsec` (obs 1)
  - `ADBE CM FlyToInset` (obs 1)
  - `ADBE CM GridWipe` (obs 1)
  - `ADBE CM InsetVideoFramed` (obs 1)
  - `ADBE CM InsetVideoTorn` (obs 1)
  - `ADBE CM LightLeaksMarkers` (obs 1)
  - `ADBE CM LightLeaksRandom` (obs 1)
  - `ADBE CM MoodLightAmorph` (obs 1)
  - `ADBE CM MoodLightDigital` (obs 1)
  - `ADBE CM MoodLightStreaks` (obs 1)
  - `ADBE CM OpacityFlashMarkers` (obs 1)
  - `ADBE CM OpacityFlashRandom` (obs 1)
  - `ADBE CM ScaleBounceMarkers` (obs 1)
  - `ADBE CM ScaleBounceRandom` (obs 1)
  - `ADBE CM Spin` (obs 1)
  - `ADBE CM Throw` (obs 1)
  - `ADBE CM TransCard` (obs 3)
  - `ADBE CM TransDissolve` (obs 5)
  - `ADBE CM TransFade` (obs 5)
  - `ADBE CM TransFadeMask` (obs 14)
  - `ADBE CM TransSlide` (obs 3)
  - `ADBE CM TransStretch` (obs 4)
  - `ADBE CM TransWipe` (obs 3)
  - `ADBE CM WiggleGelatin` (obs 1)
  - `ADBE CM WigglePosition` (obs 1)
  - `ADBE CM WiggleRotation` (obs 1)
  - `ADBE CM WiggleScale` (obs 1)
  - `ADBE CM WiggleShear` (obs 1)
  - `ADBE CM Wigglerama` (obs 1)
  - `ADBE CM Zoom2DSpin` (obs 1)
  - `ADBE CM Zoom3DTumble` (obs 1)
  - `ADBE CM ZoomBubble` (obs 1)
  - `ADBE CM ZoomSpiral` (obs 1)
  - `ADBE CM ZoomWobble` (obs 1)
  - `ADBE Calculations` (obs 17)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Cell Pattern` (obs 2)
  - `ADBE Channel Blur` (obs 3)
  - `ADBE Channel Combiner` (obs 2)
  - `ADBE Checkbox Control` (obs 20)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Checkerboard` (obs 2)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Circle` (obs 3)
  - `ADBE Color Balance (HLS)` (obs 6)
  - `ADBE Color Balance 2` (obs 1)
  - `ADBE Color Control` (obs 13)
    - `ADBE Effect Built In Params` (obs 4)
  - `ADBE Color Emboss` (obs 4)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Compander` (obs 2)
  - `ADBE CurvesCustom` (obs 9)
  - `ADBE Difference` (obs 1)
  - `ADBE Drop Shadow` (obs 7)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Dust & Scratches` (obs 1)
  - `ADBE Easy Levels2` (obs 27)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Echo` (obs 2)
  - `ADBE Exposure2` (obs 2)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Fill` (obs 4)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Find Edges` (obs 4)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Fractal Noise` (obs 52)
    - `ADBE Effect Built In Params` (obs 9)
  - `ADBE GROW BOUNDS` (obs 1)
  - `ADBE Gaussian Blur 2` (obs 6)
  - `ADBE Geometry2` (obs 67)
    - `ADBE Effect Built In Params` (obs 5)
  - `ADBE Glo2` (obs 10)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE HUE SATURATION` (obs 4)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Invert` (obs 8)
  - `ADBE KeyCleaner` (obs 1)
  - `ADBE Layer Control` (obs 5)
  - `ADBE Leave Color` (obs 1)
  - `ADBE Lightning 2` (obs 2)
  - `ADBE Linear Wipe` (obs 25)
  - `ADBE MESH WARP` (obs 2)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Magnify` (obs 1)
  - `ADBE Median` (obs 2)
  - `ADBE Minimax` (obs 7)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Mosaic` (obs 7)
  - `ADBE Motion Blur` (obs 4)
  - `ADBE Noise Alpha2` (obs 1)
  - `ADBE Noise2` (obs 7)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Offset` (obs 2)
  - `ADBE PS Median` (obs 3)
    - `ADBE Effect Built In Params` (obs 4)
  - `ADBE Point Control` (obs 10)
    - `ADBE Effect Built In Params` (obs 5)
  - `ADBE Polar Coordinates` (obs 2)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Posterize` (obs 3)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Posterize Time` (obs 4)
  - `ADBE Radial Wipe` (obs 2)
  - `ADBE Ramp` (obs 14)
    - `ADBE Effect Built In Params` (obs 7)
  - `ADBE Remove Color Matting` (obs 1)
  - `ADBE Ripple` (obs 3)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Roughen Edges` (obs 2)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Sample Image` (obs 1)
  - `ADBE Scribble Fill` (obs 1)
  - `ADBE Separate XYZ Position` (obs 1)
  - `ADBE Separate XYZ Scale` (obs 1)
  - `ADBE Set Channels` (obs 1)
  - `ADBE Set Matte3` (obs 12)
    - `ADBE Effect Built In Params` (obs 4)
  - `ADBE Shift Channels` (obs 9)
  - `ADBE Simple Choker` (obs 2)
    - `ADBE Effect Built In Params` (obs 3)
  - `ADBE Slider Control` (obs 390)
    - `ADBE Effect Built In Params` (obs 32)
  - `ADBE Solid Composite` (obs 25)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Spherize` (obs 1)
  - `ADBE Spill2` (obs 1)
  - `ADBE Stroke` (obs 3)
  - `ADBE Threshold2` (obs 1)
  - `ADBE Tile` (obs 2)
  - `ADBE Tint` (obs 4)
  - `ADBE Tritone` (obs 29)
  - `ADBE Turbulent Displace` (obs 8)
    - `ADBE Effect Built In Params` (obs 5)
  - `ADBE Twirl` (obs 1)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Unmult` (obs 1)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Venetian Blinds` (obs 10)
    - `ADBE Effect Built In Params` (obs 2)
  - `ADBE Wave Warp` (obs 5)
  - `APC CardWipeCam` (obs 5)
    - `ADBE Effect Built In Params` (obs 2)
  - `APC Colorama` (obs 12)
    - `ADBE Effect Built In Params` (obs 9)
  - `APC Foam` (obs 1)
    - `ADBE Effect Built In Params` (obs 2)
  - `APC Radio Waves` (obs 1)
  - `APC Vegas` (obs 2)
  - `APC Wave World` (obs 1)
  - `CC Blobbylize` (obs 1)
    - `ADBE Effect Built In Params` (obs 2)
  - `CC Glass` (obs 1)
    - `ADBE Effect Built In Params` (obs 2)
  - `CC Light Sweep` (obs 2)
    - `ADBE Effect Built In Params` (obs 3)
  - `CC Mr. Mercury` (obs 2)
    - `ADBE Effect Built In Params` (obs 3)
  - `CC Power Pin` (obs 1)
  - `CC Radial Fast Blur` (obs 4)
    - `ADBE Effect Built In Params` (obs 2)
  - `CC RepeTile` (obs 1)
    - `ADBE Effect Built In Params` (obs 2)
  - `CC Toner` (obs 2)
  - `CS Composite` (obs 8)
    - `ADBE Effect Built In Params` (obs 2)
  - `CS Vignette` (obs 1)
  - `Keylight 906` (obs 1)
  - `Pseudo/171912` (obs 11)
    - `ADBE Effect Built In Params` (obs 12)
  - `Pseudo/932602` (obs 2)
    - `ADBE Effect Built In Params` (obs 3)
  - `Pseudo/@@/TFyY0wHRuOmFCPFeBtaUw` (obs 1)
  - `Pseudo/@@2UQjp/2eQV2tRS9afjpLlQ` (obs 1)
  - `Pseudo/@@2pH6WmYMSfO73JoabR05ew` (obs 1)
  - `Pseudo/@@3ylWA2zSTs6lHC8orqUOKg` (obs 1)
  - `Pseudo/@@5PbgI0x5QPerx2KEoXkwZQ` (obs 1)
  - `Pseudo/@@7Vv8RRfYRhWZ65TLbHg5pQ` (obs 2)
  - `Pseudo/@@8RvLiHL+Tpy0aX/KnWjTrQ` (obs 2)
  - `Pseudo/@@8lFDEhBnQZi4NEeyL9Tlow` (obs 2)
  - `Pseudo/@@Aa6jZXylSVyDai9CyO6W0g` (obs 1)
  - `Pseudo/@@CFdj2IdWRUSdh5h2eFCoTQ` (obs 1)
  - `Pseudo/@@Fp+TrX3qSl+Fl6EzceZhKw` (obs 1)
  - `Pseudo/@@GUskMT9fSX+mbTAPmtkNtA` (obs 1)
  - `Pseudo/@@NOVaxycmQdiZsQXuqdMoqw` (obs 1)
  - `Pseudo/@@OMludPqsS0ifGA70qIaLCA` (obs 1)
  - `Pseudo/@@RTfJvgYsQpyK/J1FVPKfAg` (obs 1)
  - `Pseudo/@@Ta55j2S2SC+/ZjjQL5X3Tw` (obs 9)
  - `Pseudo/@@ThSYIDjKRgegkcnAeai6Kg` (obs 1)
  - `Pseudo/@@TsQ7i+RRSOyDu2cYaMh/+A` (obs 1)
  - `Pseudo/@@UcQnFoClT3y2OWHdVXs/MQ` (obs 1)
  - `Pseudo/@@VNDELydNR06EEQh7GK+VzQ` (obs 1)
  - `Pseudo/@@YVtWE+0XShOIFRf26W1xjg` (obs 2)
  - `Pseudo/@@Yd9WltsYToaG7EtIvGWKGw` (obs 1)
  - `Pseudo/@@c35pPzhdTkKXabCupD6UNw` (obs 1)
  - `Pseudo/@@cQXzIgy7R1+uicD4w1BlQw` (obs 2)
  - `Pseudo/@@emgz74CrTpKBvhfTe4Q0pw` (obs 1)
  - `Pseudo/@@gs1C2TueTS6w7k6Qwfj9iA` (obs 1)
  - `Pseudo/@@iDEdPRTGQKuq9OLYGcbK/g` (obs 4)
  - `Pseudo/@@jWNan6YkT3+hq+TJTw2cig` (obs 1)
  - `Pseudo/@@lPa2qa6XT0a7wtP0y7Qm0w` (obs 4)
  - `Pseudo/@@n0BB+wVOT0qWwP0N7FrGQQ` (obs 1)
  - `Pseudo/@@nkoVEkLBTQyeYNXsPyxv4g` (obs 10)
  - `Pseudo/@@q8+OzEHrTruxZzSnzZFg/A` (obs 1)
  - `Pseudo/@@teyL6weaRaOz8Grbz1I3NQ` (obs 2)
  - `Pseudo/@@xdUe/428T/q4jEAsQTUmcA` (obs 2)
  - `Pseudo/@@yADZVOKjSLil6WsDUzZ+aA` (obs 2)
  - `Pseudo/ADBE 2D Text Box` (obs 2)
  - `Pseudo/ADBE Counter Controls` (obs 1)
  - `Pseudo/ADBE Currency Controls` (obs 1)
  - `Pseudo/ADBE Percentage Controls` (obs 1)
  - `Pseudo/ADBE Timer Controls` (obs 1)

**`ADBE Layer Styles`**
  - `ADBE Blend Options Group` (obs 358)
    - `ADBE Adv Blend Group` (obs 358)
      - `ADBE Layer Fill Opacity2` (obs 8)
  - `bevelEmboss/enabled` (obs 358)
    - `bevelEmboss/blur` (obs 9)
    - `bevelEmboss/highlightColor` (obs 2)
    - `bevelEmboss/highlightMode` (obs 1)
    - `bevelEmboss/highlightOpacity` (obs 1)
    - `bevelEmboss/localLightingAltitude` (obs 2)
    - `bevelEmboss/localLightingAngle` (obs 1)
    - `bevelEmboss/shadowColor` (obs 7)
    - `bevelEmboss/shadowMode` (obs 2)
    - `bevelEmboss/shadowOpacity` (obs 2)
    - `bevelEmboss/softness` (obs 6)
    - `bevelEmboss/strengthRatio` (obs 1)
  - `chromeFX/enabled` (obs 358)
    - `chromeFX/blur` (obs 1)
    - `chromeFX/color` (obs 1)
    - `chromeFX/distance` (obs 1)
    - `chromeFX/invert` (obs 1)
    - `chromeFX/opacity` (obs 1)
  - `dropShadow/enabled` (obs 358)
  - `frameFX/enabled` (obs 358)
  - `gradientFill/enabled` (obs 358)
  - `innerGlow/enabled` (obs 358)
    - `innerGlow/blur` (obs 15)
    - `innerGlow/color` (obs 13)
    - `innerGlow/gradientSmoothness` (obs 6)
    - `innerGlow/inputRange` (obs 7)
    - `innerGlow/mode2` (obs 13)
    - `innerGlow/opacity` (obs 7)
    - `innerGlow/shadingNoise` (obs 6)
  - `innerShadow/enabled` (obs 358)
    - `innerShadow/blur` (obs 2)
    - `innerShadow/color` (obs 2)
    - `innerShadow/distance` (obs 2)
    - `innerShadow/localLightingAngle` (obs 1)
    - `innerShadow/opacity` (obs 2)
  - `outerGlow/enabled` (obs 358)
    - `outerGlow/blur` (obs 5)
    - `outerGlow/color` (obs 5)
    - `outerGlow/mode2` (obs 5)
  - `patternFill/enabled` (obs 358)
  - `solidFill/enabled` (obs 358)
    - `solidFill/color` (obs 5)
    - `solidFill/mode2` (obs 5)
    - `solidFill/opacity` (obs 5)

**`ADBE Time Remapping`** -- root present in the match-name catalogue; no containment edge observed in the shipped documents read.

**`ADBE Text Properties`**
  - `ADBE Text Animators` (obs 545)
    - `ADBE Text Animator` (obs 545)
      - `ADBE Text Animator Properties` (obs 9)
        - `ADBE 3DText Back Ambient` (obs 486)
        - `ADBE 3DText Back Bright` (obs 486)
        - `ADBE 3DText Back Diffuse` (obs 486)
        - `ADBE 3DText Back Fresnel` (obs 486)
        - `ADBE 3DText Back Gloss` (obs 486)
        - `ADBE 3DText Back Hue` (obs 486)
        - `ADBE 3DText Back IOR` (obs 486)
        - `ADBE 3DText Back Metal` (obs 486)
        - `ADBE 3DText Back Opacity` (obs 486)
        - `ADBE 3DText Back RGB` (obs 486)
        - `ADBE 3DText Back Reflection` (obs 486)
        - `ADBE 3DText Back Sat` (obs 486)
        - `ADBE 3DText Back Shininess` (obs 486)
        - `ADBE 3DText Back Specular` (obs 486)
        - `ADBE 3DText Back XparRoll` (obs 486)
        - `ADBE 3DText Back Xparency` (obs 486)
        - `ADBE 3DText Bevel Ambient` (obs 486)
        - `ADBE 3DText Bevel Bright` (obs 486)
        - `ADBE 3DText Bevel Depth` (obs 486)
        - `ADBE 3DText Bevel Diffuse` (obs 486)
        - `ADBE 3DText Bevel Fresnel` (obs 486)
        - `ADBE 3DText Bevel Gloss` (obs 486)
        - `ADBE 3DText Bevel Hue` (obs 486)
        - `ADBE 3DText Bevel IOR` (obs 486)
        - `ADBE 3DText Bevel Metal` (obs 486)
        - `ADBE 3DText Bevel Opacity` (obs 486)
        - `ADBE 3DText Bevel RGB` (obs 486)
        - `ADBE 3DText Bevel Reflection` (obs 486)
        - `ADBE 3DText Bevel Sat` (obs 486)
        - `ADBE 3DText Bevel Shininess` (obs 486)
        - `ADBE 3DText Bevel Specular` (obs 486)
        - `ADBE 3DText Bevel XparRoll` (obs 486)
        - `ADBE 3DText Bevel Xparency` (obs 486)
        - `ADBE 3DText Extrude Depth` (obs 486)
        - `ADBE 3DText Front Ambient` (obs 486)
        - `ADBE 3DText Front Bright` (obs 486)
        - `ADBE 3DText Front Diffuse` (obs 486)
        - `ADBE 3DText Front Fresnel` (obs 486)
        - `ADBE 3DText Front Gloss` (obs 486)
        - `ADBE 3DText Front Hue` (obs 486)
        - `ADBE 3DText Front IOR` (obs 486)
        - `ADBE 3DText Front Metal` (obs 486)
        - `ADBE 3DText Front Opacity` (obs 486)
        - `ADBE 3DText Front RGB` (obs 486)
        - `ADBE 3DText Front Reflection` (obs 486)
        - `ADBE 3DText Front Sat` (obs 486)
        - `ADBE 3DText Front Shininess` (obs 486)
        - `ADBE 3DText Front Specular` (obs 486)
        - `ADBE 3DText Front XparRoll` (obs 486)
        - `ADBE 3DText Front Xparency` (obs 486)
        - `ADBE 3DText Side Ambient` (obs 486)
        - `ADBE 3DText Side Bright` (obs 486)
        - `ADBE 3DText Side Diffuse` (obs 486)
        - `ADBE 3DText Side Fresnel` (obs 486)
        - `ADBE 3DText Side Gloss` (obs 486)
        - `ADBE 3DText Side Hue` (obs 486)
        - `ADBE 3DText Side IOR` (obs 486)
        - `ADBE 3DText Side Metal` (obs 486)
        - `ADBE 3DText Side Opacity` (obs 486)
        - `ADBE 3DText Side RGB` (obs 486)
        - `ADBE 3DText Side Reflection` (obs 486)
        - `ADBE 3DText Side Sat` (obs 486)
        - `ADBE 3DText Side Shininess` (obs 486)
        - `ADBE 3DText Side Specular` (obs 486)
        - `ADBE 3DText Side XparRoll` (obs 486)
        - `ADBE 3DText Side Xparency` (obs 486)
        - `ADBE Text Anchor Point` (obs 37)
        - `ADBE Text Anchor Point 3D` (obs 488)
        - `ADBE Text Blur` (obs 488)
        - `ADBE Text Character Change Type` (obs 525)
        - `ADBE Text Character Offset` (obs 525)
        - `ADBE Text Character Range` (obs 525)
        - `ADBE Text Character Replace` (obs 525)
        - `ADBE Text Fill Brightness` (obs 525)
        - `ADBE Text Fill Color` (obs 525)
        - `ADBE Text Fill Hue` (obs 525)
        - `ADBE Text Fill Opacity` (obs 525)
        - `ADBE Text Fill Saturation` (obs 525)
        - `ADBE Text Line Anchor` (obs 525)
        - `ADBE Text Line Spacing` (obs 525)
        - `ADBE Text Opacity` (obs 526)
        - `ADBE Text Position` (obs 37)
        - `ADBE Text Position 3D` (obs 488)
        - `ADBE Text Rotation` (obs 525)
        - `ADBE Text Rotation X` (obs 488)
        - `ADBE Text Rotation Y` (obs 488)
        - `ADBE Text Scale` (obs 39)
        - `ADBE Text Scale 3D` (obs 488)
        - `ADBE Text Skew` (obs 525)
        - `ADBE Text Skew Axis` (obs 525)
        - `ADBE Text Stroke Brightness` (obs 525)
        - `ADBE Text Stroke Color` (obs 526)
        - `ADBE Text Stroke Hue` (obs 525)
        - `ADBE Text Stroke Opacity` (obs 526)
        - `ADBE Text Stroke Saturation` (obs 525)
        - `ADBE Text Stroke Width` (obs 526)
        - `ADBE Text Track Type` (obs 525)
        - `ADBE Text Tracking Amount` (obs 525)
        - `ADBE Text VF Axis 1` (obs 3)
      - `ADBE Text Selectors` (obs 14)
        - `ADBE Text Expressible Selector` (obs 41)
        - `ADBE Text Selector` (obs 481)
        - `ADBE Text Wiggly Selector` (obs 82)
  - `ADBE Text Document` (obs 44)
  - `ADBE Text More Options` (obs 215)
    - `ADBE Text Anchor Point Align` (obs 58)
    - `ADBE Text Anchor Point Option` (obs 17)
    - `ADBE Text Character Blend Mode` (obs 1)
    - `ADBE Text Render Order` (obs 2)
    - `ADBE Text Variable Font Spacing` (obs 3)
  - `ADBE Text Path Options` (obs 112)
    - `ADBE Text First Margin` (obs 6)
    - `ADBE Text Force Align Path` (obs 6)
    - `ADBE Text Last Margin` (obs 6)
    - `ADBE Text Path` (obs 6)
    - `ADBE Text Perpendicular To Path` (obs 6)
    - `ADBE Text Reverse Path` (obs 6)

**`ADBE Root Vectors Group`**
  - `ADBE Vector Filter - Repeater` (obs 5)
    - `ADBE Vector Repeater Copies` (obs 21)
    - `ADBE Vector Repeater Offset` (obs 17)
    - `ADBE Vector Repeater Order` (obs 17)
    - `ADBE Vector Repeater Transform` (obs 21)
      - `ADBE Vector Repeater Anchor` (obs 18)
      - `ADBE Vector Repeater Opacity 1` (obs 18)
      - `ADBE Vector Repeater Opacity 2` (obs 18)
      - `ADBE Vector Repeater Position` (obs 22)
      - `ADBE Vector Repeater Rotation` (obs 18)
      - `ADBE Vector Repeater Scale` (obs 18)
  - `ADBE Vector Filter - Roughen` (obs 1)
    - `ADBE Vector Roughen Detail` (obs 1)
    - `ADBE Vector Roughen Points` (obs 1)
    - `ADBE Vector Roughen Size` (obs 1)
    - `ADBE Vector Temporal Freq` (obs 1)
  - `ADBE Vector Filter - Trim` (obs 3)
    - `ADBE Vector Trim End` (obs 20)
    - `ADBE Vector Trim Offset` (obs 20)
    - `ADBE Vector Trim Start` (obs 20)
    - `ADBE Vector Trim Type` (obs 20)
  - `ADBE Vector Graphic - Fill` (obs 4)
    - `ADBE Vector Blend Mode` (obs 90)
    - `ADBE Vector Composite Order` (obs 90)
    - `ADBE Vector Fill Color` (obs 100)
    - `ADBE Vector Fill Opacity` (obs 90)
    - `ADBE Vector Fill Rule` (obs 90)
  - `ADBE Vector Graphic - Stroke` (obs 2)
    - `ADBE Vector Blend Mode` (obs 95)
    - `ADBE Vector Composite Order` (obs 95)
    - `ADBE Vector Stroke Color` (obs 108)
    - `ADBE Vector Stroke Dashes` (obs 112)
      - `ADBE Vector Stroke Dash 1` (obs 97)
      - `ADBE Vector Stroke Dash 2` (obs 97)
      - `ADBE Vector Stroke Dash 3` (obs 97)
      - `ADBE Vector Stroke Gap 1` (obs 97)
      - `ADBE Vector Stroke Gap 2` (obs 97)
      - `ADBE Vector Stroke Gap 3` (obs 97)
      - `ADBE Vector Stroke Offset` (obs 97)
    - `ADBE Vector Stroke Line Cap` (obs 109)
    - `ADBE Vector Stroke Line Join` (obs 109)
    - `ADBE Vector Stroke Miter Limit` (obs 109)
    - `ADBE Vector Stroke Opacity` (obs 108)
    - `ADBE Vector Stroke Taper` (obs 112)
      - `ADBE Vector Taper End Ease` (obs 110)
      - `ADBE Vector Taper End Length` (obs 110)
      - `ADBE Vector Taper End Width` (obs 110)
      - `ADBE Vector Taper EndWidthPx` (obs 97)
      - `ADBE Vector Taper Length Units` (obs 97)
      - `ADBE Vector Taper Start Ease` (obs 110)
      - `ADBE Vector Taper Start Length` (obs 110)
      - `ADBE Vector Taper Start Width` (obs 110)
      - `ADBE Vector Taper StartWidthPx` (obs 97)
    - `ADBE Vector Stroke Wave` (obs 112)
      - `ADBE Vector Taper Wave Amount` (obs 110)
      - `ADBE Vector Taper Wave Cycles` (obs 97)
      - `ADBE Vector Taper Wave Phase` (obs 110)
      - `ADBE Vector Taper Wave Units` (obs 97)
      - `ADBE Vector Taper Wavelength` (obs 110)
    - `ADBE Vector Stroke Width` (obs 112)
  - `ADBE Vector Group` (obs 69)
    - `ADBE Vector Blend Mode` (obs 118)
    - `ADBE Vector Materials Group` (obs 139)
      - `ADBE Vec3D Back Ambient` (obs 168)
      - `ADBE Vec3D Back Diffuse` (obs 168)
      - `ADBE Vec3D Back Fresnel` (obs 168)
      - `ADBE Vec3D Back Gloss` (obs 168)
      - `ADBE Vec3D Back IOR` (obs 168)
      - `ADBE Vec3D Back Metal` (obs 168)
      - `ADBE Vec3D Back RGB` (obs 168)
      - `ADBE Vec3D Back Reflection` (obs 168)
      - `ADBE Vec3D Back Shininess` (obs 168)
      - `ADBE Vec3D Back Specular` (obs 168)
      - `ADBE Vec3D Back XparRoll` (obs 168)
      - `ADBE Vec3D Back Xparency` (obs 168)
      - `ADBE Vec3D Bevel Ambient` (obs 168)
      - `ADBE Vec3D Bevel Diffuse` (obs 168)
      - `ADBE Vec3D Bevel Fresnel` (obs 168)
      - `ADBE Vec3D Bevel Gloss` (obs 168)
      - `ADBE Vec3D Bevel IOR` (obs 168)
      - `ADBE Vec3D Bevel Metal` (obs 182)
      - `ADBE Vec3D Bevel RGB` (obs 182)
      - `ADBE Vec3D Bevel Reflection` (obs 168)
      - `ADBE Vec3D Bevel Shininess` (obs 182)
      - `ADBE Vec3D Bevel Specular` (obs 182)
      - `ADBE Vec3D Bevel XparRoll` (obs 168)
      - `ADBE Vec3D Bevel Xparency` (obs 168)
      - `ADBE Vec3D Front Ambient` (obs 168)
      - `ADBE Vec3D Front Diffuse` (obs 168)
      - `ADBE Vec3D Front Fresnel` (obs 168)
      - `ADBE Vec3D Front Gloss` (obs 168)
      - `ADBE Vec3D Front IOR` (obs 168)
      - `ADBE Vec3D Front Metal` (obs 168)
      - `ADBE Vec3D Front RGB` (obs 168)
      - `ADBE Vec3D Front Reflection` (obs 168)
      - `ADBE Vec3D Front Shininess` (obs 168)
      - `ADBE Vec3D Front Specular` (obs 168)
      - `ADBE Vec3D Front XparRoll` (obs 168)
      - `ADBE Vec3D Front Xparency` (obs 168)
      - `ADBE Vec3D Side Ambient` (obs 168)
      - `ADBE Vec3D Side Diffuse` (obs 168)
      - `ADBE Vec3D Side Fresnel` (obs 168)
      - `ADBE Vec3D Side Gloss` (obs 168)
      - `ADBE Vec3D Side IOR` (obs 168)
      - `ADBE Vec3D Side Metal` (obs 168)
      - `ADBE Vec3D Side RGB` (obs 168)
      - `ADBE Vec3D Side Reflection` (obs 168)
      - `ADBE Vec3D Side Shininess` (obs 168)
      - `ADBE Vec3D Side Specular` (obs 168)
      - `ADBE Vec3D Side XparRoll` (obs 168)
      - `ADBE Vec3D Side Xparency` (obs 168)
    - `ADBE Vector Transform Group` (obs 139)
      - `ADBE Vector Anchor` (obs 182)
      - `ADBE Vector Group Opacity` (obs 181)
      - `ADBE Vector Position` (obs 185)
      - `ADBE Vector Rotation` (obs 181)
      - `ADBE Vector Scale` (obs 182)
      - `ADBE Vector Skew` (obs 181)
      - `ADBE Vector Skew Axis` (obs 181)
    - `ADBE Vectors Group` (obs 139)
      - `ADBE Vector Filter - Merge` (obs 18)
        - `ADBE Vector Merge Type` (obs 16)
      - `ADBE Vector Filter - Offset` (obs 4)
        - `ADBE Vector Offset Amount` (obs 4)
        - `ADBE Vector Offset Copies` (obs 4)
        - `ADBE Vector Offset Copy Offset` (obs 4)
        - `ADBE Vector Offset Line Join` (obs 4)
        - `ADBE Vector Offset Miter Limit` (obs 4)
      - `ADBE Vector Filter - PB` (obs 8)
        - `ADBE Vector PuckerBloat Amount` (obs 8)
      - `ADBE Vector Filter - RC` (obs 1)
        - `ADBE Vector RoundCorner Radius` (obs 1)
      - `ADBE Vector Filter - Repeater` (obs 17)
        - `ADBE Vector Repeater Copies` (obs 21)
        - `ADBE Vector Repeater Offset` (obs 17)
        - `ADBE Vector Repeater Order` (obs 17)
        - `ADBE Vector Repeater Transform` (obs 21)
      - `ADBE Vector Filter - Trim` (obs 20)
        - `ADBE Vector Trim End` (obs 20)
        - `ADBE Vector Trim Offset` (obs 20)
        - `ADBE Vector Trim Start` (obs 20)
        - `ADBE Vector Trim Type` (obs 20)
      - `ADBE Vector Filter - Twist` (obs 1)
        - `ADBE Vector Twist Angle` (obs 1)
        - `ADBE Vector Twist Center` (obs 1)
      - `ADBE Vector Filter - Wiggler` (obs 2)
        - `ADBE Vector Correlation` (obs 2)
        - `ADBE Vector Random Seed` (obs 2)
        - `ADBE Vector Spatial Phase` (obs 2)
        - `ADBE Vector Temporal Phase` (obs 2)
        - `ADBE Vector Wiggler Transform` (obs 2)
        - `ADBE Vector Xform Temporal Freq` (obs 2)
      - `ADBE Vector Filter - Zigzag` (obs 2)
        - `ADBE Vector Zigzag Detail` (obs 2)
        - `ADBE Vector Zigzag Points` (obs 2)
        - `ADBE Vector Zigzag Size` (obs 2)
      - `ADBE Vector Graphic - Fill` (obs 96)
        - `ADBE Vector Blend Mode` (obs 90)
        - `ADBE Vector Composite Order` (obs 90)
        - `ADBE Vector Fill Color` (obs 100)
        - `ADBE Vector Fill Opacity` (obs 90)
        - `ADBE Vector Fill Rule` (obs 90)
      - `ADBE Vector Graphic - Stroke` (obs 112)
        - `ADBE Vector Blend Mode` (obs 95)
        - `ADBE Vector Composite Order` (obs 95)
        - `ADBE Vector Stroke Color` (obs 108)
        - `ADBE Vector Stroke Dashes` (obs 112)
        - `ADBE Vector Stroke Line Cap` (obs 109)
        - `ADBE Vector Stroke Line Join` (obs 109)
        - `ADBE Vector Stroke Miter Limit` (obs 109)
        - `ADBE Vector Stroke Opacity` (obs 108)
        - `ADBE Vector Stroke Taper` (obs 112)
        - `ADBE Vector Stroke Wave` (obs 112)
        - `ADBE Vector Stroke Width` (obs 112)
      - `ADBE Vector Group` (obs 65)
      - `ADBE Vector Shape - Ellipse` (obs 56)
        - `ADBE Vector Ellipse Position` (obs 55)
        - `ADBE Vector Ellipse Size` (obs 58)
        - `ADBE Vector Shape Direction` (obs 55)
      - `ADBE Vector Shape - Group` (obs 49)
        - `ADBE Vector Shape` (obs 50)
        - `ADBE Vector Shape Direction` (obs 29)
      - `ADBE Vector Shape - Rect` (obs 73)
        - `ADBE Vector Rect Position` (obs 71)
        - `ADBE Vector Rect Roundness` (obs 71)
        - `ADBE Vector Rect Size` (obs 73)
        - `ADBE Vector Shape Direction` (obs 71)
      - `ADBE Vector Shape - Star` (obs 29)
        - `ADBE Vector Shape Direction` (obs 29)
        - `ADBE Vector Star Inner Radius` (obs 29)
        - `ADBE Vector Star Inner Roundess` (obs 29)
        - `ADBE Vector Star Outer Radius` (obs 29)
        - `ADBE Vector Star Outer Roundess` (obs 29)
        - `ADBE Vector Star Points` (obs 29)
        - `ADBE Vector Star Position` (obs 29)
        - `ADBE Vector Star Rotation` (obs 29)
        - `ADBE Vector Star Type` (obs 29)
  - `ADBE Vector Shape - Ellipse` (obs 2)
    - `ADBE Vector Ellipse Position` (obs 55)
    - `ADBE Vector Ellipse Size` (obs 58)
    - `ADBE Vector Shape Direction` (obs 55)
  - `ADBE Vector Shape - Group` (obs 1)
    - `ADBE Vector Shape` (obs 50)
    - `ADBE Vector Shape Direction` (obs 29)

**`ADBE Data Group`** -- root present in the match-name catalogue; no containment edge observed in the shipped documents read.

**`ADBE Effect Built In Params`**
  - `ADBE Effect Mask Opacity` (obs 1043)
  - `ADBE Effect Mask Parade` (obs 1043)
    - `ADBE Effect Mask` (obs 1)
      - `ADBE Effect Path Stream Ref` (obs 1)
  - `ADBE Force CPU GPU` (obs 1041)

**`ADBE Vectors Group`**
  - `ADBE Vector Filter - Merge` (obs 18)
    - `ADBE Vector Merge Type` (obs 16)
  - `ADBE Vector Filter - Offset` (obs 4)
    - `ADBE Vector Offset Amount` (obs 4)
    - `ADBE Vector Offset Copies` (obs 4)
    - `ADBE Vector Offset Copy Offset` (obs 4)
    - `ADBE Vector Offset Line Join` (obs 4)
    - `ADBE Vector Offset Miter Limit` (obs 4)
  - `ADBE Vector Filter - PB` (obs 8)
    - `ADBE Vector PuckerBloat Amount` (obs 8)
  - `ADBE Vector Filter - RC` (obs 1)
    - `ADBE Vector RoundCorner Radius` (obs 1)
  - `ADBE Vector Filter - Repeater` (obs 17)
    - `ADBE Vector Repeater Copies` (obs 21)
    - `ADBE Vector Repeater Offset` (obs 17)
    - `ADBE Vector Repeater Order` (obs 17)
    - `ADBE Vector Repeater Transform` (obs 21)
      - `ADBE Vector Repeater Anchor` (obs 18)
      - `ADBE Vector Repeater Opacity 1` (obs 18)
      - `ADBE Vector Repeater Opacity 2` (obs 18)
      - `ADBE Vector Repeater Position` (obs 22)
      - `ADBE Vector Repeater Rotation` (obs 18)
      - `ADBE Vector Repeater Scale` (obs 18)
  - `ADBE Vector Filter - Trim` (obs 20)
    - `ADBE Vector Trim End` (obs 20)
    - `ADBE Vector Trim Offset` (obs 20)
    - `ADBE Vector Trim Start` (obs 20)
    - `ADBE Vector Trim Type` (obs 20)
  - `ADBE Vector Filter - Twist` (obs 1)
    - `ADBE Vector Twist Angle` (obs 1)
    - `ADBE Vector Twist Center` (obs 1)
  - `ADBE Vector Filter - Wiggler` (obs 2)
    - `ADBE Vector Correlation` (obs 2)
    - `ADBE Vector Random Seed` (obs 2)
    - `ADBE Vector Spatial Phase` (obs 2)
    - `ADBE Vector Temporal Phase` (obs 2)
    - `ADBE Vector Wiggler Transform` (obs 2)
      - `ADBE Vector Wiggler Anchor` (obs 2)
      - `ADBE Vector Wiggler Position` (obs 2)
      - `ADBE Vector Wiggler Rotation` (obs 2)
      - `ADBE Vector Wiggler Scale` (obs 2)
    - `ADBE Vector Xform Temporal Freq` (obs 2)
  - `ADBE Vector Filter - Zigzag` (obs 2)
    - `ADBE Vector Zigzag Detail` (obs 2)
    - `ADBE Vector Zigzag Points` (obs 2)
    - `ADBE Vector Zigzag Size` (obs 2)
  - `ADBE Vector Graphic - Fill` (obs 96)
    - `ADBE Vector Blend Mode` (obs 90)
    - `ADBE Vector Composite Order` (obs 90)
    - `ADBE Vector Fill Color` (obs 100)
    - `ADBE Vector Fill Opacity` (obs 90)
    - `ADBE Vector Fill Rule` (obs 90)
  - `ADBE Vector Graphic - Stroke` (obs 112)
    - `ADBE Vector Blend Mode` (obs 95)
    - `ADBE Vector Composite Order` (obs 95)
    - `ADBE Vector Stroke Color` (obs 108)
    - `ADBE Vector Stroke Dashes` (obs 112)
      - `ADBE Vector Stroke Dash 1` (obs 97)
      - `ADBE Vector Stroke Dash 2` (obs 97)
      - `ADBE Vector Stroke Dash 3` (obs 97)
      - `ADBE Vector Stroke Gap 1` (obs 97)
      - `ADBE Vector Stroke Gap 2` (obs 97)
      - `ADBE Vector Stroke Gap 3` (obs 97)
      - `ADBE Vector Stroke Offset` (obs 97)
    - `ADBE Vector Stroke Line Cap` (obs 109)
    - `ADBE Vector Stroke Line Join` (obs 109)
    - `ADBE Vector Stroke Miter Limit` (obs 109)
    - `ADBE Vector Stroke Opacity` (obs 108)
    - `ADBE Vector Stroke Taper` (obs 112)
      - `ADBE Vector Taper End Ease` (obs 110)
      - `ADBE Vector Taper End Length` (obs 110)
      - `ADBE Vector Taper End Width` (obs 110)
      - `ADBE Vector Taper EndWidthPx` (obs 97)
      - `ADBE Vector Taper Length Units` (obs 97)
      - `ADBE Vector Taper Start Ease` (obs 110)
      - `ADBE Vector Taper Start Length` (obs 110)
      - `ADBE Vector Taper Start Width` (obs 110)
      - `ADBE Vector Taper StartWidthPx` (obs 97)
    - `ADBE Vector Stroke Wave` (obs 112)
      - `ADBE Vector Taper Wave Amount` (obs 110)
      - `ADBE Vector Taper Wave Cycles` (obs 97)
      - `ADBE Vector Taper Wave Phase` (obs 110)
      - `ADBE Vector Taper Wave Units` (obs 97)
      - `ADBE Vector Taper Wavelength` (obs 110)
    - `ADBE Vector Stroke Width` (obs 112)
  - `ADBE Vector Group` (obs 65)
    - `ADBE Vector Blend Mode` (obs 118)
    - `ADBE Vector Materials Group` (obs 139)
      - `ADBE Vec3D Back Ambient` (obs 168)
      - `ADBE Vec3D Back Diffuse` (obs 168)
      - `ADBE Vec3D Back Fresnel` (obs 168)
      - `ADBE Vec3D Back Gloss` (obs 168)
      - `ADBE Vec3D Back IOR` (obs 168)
      - `ADBE Vec3D Back Metal` (obs 168)
      - `ADBE Vec3D Back RGB` (obs 168)
      - `ADBE Vec3D Back Reflection` (obs 168)
      - `ADBE Vec3D Back Shininess` (obs 168)
      - `ADBE Vec3D Back Specular` (obs 168)
      - `ADBE Vec3D Back XparRoll` (obs 168)
      - `ADBE Vec3D Back Xparency` (obs 168)
      - `ADBE Vec3D Bevel Ambient` (obs 168)
      - `ADBE Vec3D Bevel Diffuse` (obs 168)
      - `ADBE Vec3D Bevel Fresnel` (obs 168)
      - `ADBE Vec3D Bevel Gloss` (obs 168)
      - `ADBE Vec3D Bevel IOR` (obs 168)
      - `ADBE Vec3D Bevel Metal` (obs 182)
      - `ADBE Vec3D Bevel RGB` (obs 182)
      - `ADBE Vec3D Bevel Reflection` (obs 168)
      - `ADBE Vec3D Bevel Shininess` (obs 182)
      - `ADBE Vec3D Bevel Specular` (obs 182)
      - `ADBE Vec3D Bevel XparRoll` (obs 168)
      - `ADBE Vec3D Bevel Xparency` (obs 168)
      - `ADBE Vec3D Front Ambient` (obs 168)
      - `ADBE Vec3D Front Diffuse` (obs 168)
      - `ADBE Vec3D Front Fresnel` (obs 168)
      - `ADBE Vec3D Front Gloss` (obs 168)
      - `ADBE Vec3D Front IOR` (obs 168)
      - `ADBE Vec3D Front Metal` (obs 168)
      - `ADBE Vec3D Front RGB` (obs 168)
      - `ADBE Vec3D Front Reflection` (obs 168)
      - `ADBE Vec3D Front Shininess` (obs 168)
      - `ADBE Vec3D Front Specular` (obs 168)
      - `ADBE Vec3D Front XparRoll` (obs 168)
      - `ADBE Vec3D Front Xparency` (obs 168)
      - `ADBE Vec3D Side Ambient` (obs 168)
      - `ADBE Vec3D Side Diffuse` (obs 168)
      - `ADBE Vec3D Side Fresnel` (obs 168)
      - `ADBE Vec3D Side Gloss` (obs 168)
      - `ADBE Vec3D Side IOR` (obs 168)
      - `ADBE Vec3D Side Metal` (obs 168)
      - `ADBE Vec3D Side RGB` (obs 168)
      - `ADBE Vec3D Side Reflection` (obs 168)
      - `ADBE Vec3D Side Shininess` (obs 168)
      - `ADBE Vec3D Side Specular` (obs 168)
      - `ADBE Vec3D Side XparRoll` (obs 168)
      - `ADBE Vec3D Side Xparency` (obs 168)
    - `ADBE Vector Transform Group` (obs 139)
      - `ADBE Vector Anchor` (obs 182)
      - `ADBE Vector Group Opacity` (obs 181)
      - `ADBE Vector Position` (obs 185)
      - `ADBE Vector Rotation` (obs 181)
      - `ADBE Vector Scale` (obs 182)
      - `ADBE Vector Skew` (obs 181)
      - `ADBE Vector Skew Axis` (obs 181)
    - `ADBE Vectors Group` (obs 139)
  - `ADBE Vector Shape - Ellipse` (obs 56)
    - `ADBE Vector Ellipse Position` (obs 55)
    - `ADBE Vector Ellipse Size` (obs 58)
    - `ADBE Vector Shape Direction` (obs 55)
  - `ADBE Vector Shape - Group` (obs 49)
    - `ADBE Vector Shape` (obs 50)
    - `ADBE Vector Shape Direction` (obs 29)
  - `ADBE Vector Shape - Rect` (obs 73)
    - `ADBE Vector Rect Position` (obs 71)
    - `ADBE Vector Rect Roundness` (obs 71)
    - `ADBE Vector Rect Size` (obs 73)
    - `ADBE Vector Shape Direction` (obs 71)
  - `ADBE Vector Shape - Star` (obs 29)
    - `ADBE Vector Shape Direction` (obs 29)
    - `ADBE Vector Star Inner Radius` (obs 29)
    - `ADBE Vector Star Inner Roundess` (obs 29)
    - `ADBE Vector Star Outer Radius` (obs 29)
    - `ADBE Vector Star Outer Roundess` (obs 29)
    - `ADBE Vector Star Points` (obs 29)
    - `ADBE Vector Star Position` (obs 29)
    - `ADBE Vector Star Rotation` (obs 29)
    - `ADBE Vector Star Type` (obs 29)

**`ADBE Vector Group`**
  - `ADBE Vector Blend Mode` (obs 118)
  - `ADBE Vector Materials Group` (obs 139)
    - `ADBE Vec3D Back Ambient` (obs 168)
    - `ADBE Vec3D Back Diffuse` (obs 168)
    - `ADBE Vec3D Back Fresnel` (obs 168)
    - `ADBE Vec3D Back Gloss` (obs 168)
    - `ADBE Vec3D Back IOR` (obs 168)
    - `ADBE Vec3D Back Metal` (obs 168)
    - `ADBE Vec3D Back RGB` (obs 168)
    - `ADBE Vec3D Back Reflection` (obs 168)
    - `ADBE Vec3D Back Shininess` (obs 168)
    - `ADBE Vec3D Back Specular` (obs 168)
    - `ADBE Vec3D Back XparRoll` (obs 168)
    - `ADBE Vec3D Back Xparency` (obs 168)
    - `ADBE Vec3D Bevel Ambient` (obs 168)
    - `ADBE Vec3D Bevel Diffuse` (obs 168)
    - `ADBE Vec3D Bevel Fresnel` (obs 168)
    - `ADBE Vec3D Bevel Gloss` (obs 168)
    - `ADBE Vec3D Bevel IOR` (obs 168)
    - `ADBE Vec3D Bevel Metal` (obs 182)
    - `ADBE Vec3D Bevel RGB` (obs 182)
    - `ADBE Vec3D Bevel Reflection` (obs 168)
    - `ADBE Vec3D Bevel Shininess` (obs 182)
    - `ADBE Vec3D Bevel Specular` (obs 182)
    - `ADBE Vec3D Bevel XparRoll` (obs 168)
    - `ADBE Vec3D Bevel Xparency` (obs 168)
    - `ADBE Vec3D Front Ambient` (obs 168)
    - `ADBE Vec3D Front Diffuse` (obs 168)
    - `ADBE Vec3D Front Fresnel` (obs 168)
    - `ADBE Vec3D Front Gloss` (obs 168)
    - `ADBE Vec3D Front IOR` (obs 168)
    - `ADBE Vec3D Front Metal` (obs 168)
    - `ADBE Vec3D Front RGB` (obs 168)
    - `ADBE Vec3D Front Reflection` (obs 168)
    - `ADBE Vec3D Front Shininess` (obs 168)
    - `ADBE Vec3D Front Specular` (obs 168)
    - `ADBE Vec3D Front XparRoll` (obs 168)
    - `ADBE Vec3D Front Xparency` (obs 168)
    - `ADBE Vec3D Side Ambient` (obs 168)
    - `ADBE Vec3D Side Diffuse` (obs 168)
    - `ADBE Vec3D Side Fresnel` (obs 168)
    - `ADBE Vec3D Side Gloss` (obs 168)
    - `ADBE Vec3D Side IOR` (obs 168)
    - `ADBE Vec3D Side Metal` (obs 168)
    - `ADBE Vec3D Side RGB` (obs 168)
    - `ADBE Vec3D Side Reflection` (obs 168)
    - `ADBE Vec3D Side Shininess` (obs 168)
    - `ADBE Vec3D Side Specular` (obs 168)
    - `ADBE Vec3D Side XparRoll` (obs 168)
    - `ADBE Vec3D Side Xparency` (obs 168)
  - `ADBE Vector Transform Group` (obs 139)
    - `ADBE Vector Anchor` (obs 182)
    - `ADBE Vector Group Opacity` (obs 181)
    - `ADBE Vector Position` (obs 185)
    - `ADBE Vector Rotation` (obs 181)
    - `ADBE Vector Scale` (obs 182)
    - `ADBE Vector Skew` (obs 181)
    - `ADBE Vector Skew Axis` (obs 181)
  - `ADBE Vectors Group` (obs 139)
    - `ADBE Vector Filter - Merge` (obs 18)
      - `ADBE Vector Merge Type` (obs 16)
    - `ADBE Vector Filter - Offset` (obs 4)
      - `ADBE Vector Offset Amount` (obs 4)
      - `ADBE Vector Offset Copies` (obs 4)
      - `ADBE Vector Offset Copy Offset` (obs 4)
      - `ADBE Vector Offset Line Join` (obs 4)
      - `ADBE Vector Offset Miter Limit` (obs 4)
    - `ADBE Vector Filter - PB` (obs 8)
      - `ADBE Vector PuckerBloat Amount` (obs 8)
    - `ADBE Vector Filter - RC` (obs 1)
      - `ADBE Vector RoundCorner Radius` (obs 1)
    - `ADBE Vector Filter - Repeater` (obs 17)
      - `ADBE Vector Repeater Copies` (obs 21)
      - `ADBE Vector Repeater Offset` (obs 17)
      - `ADBE Vector Repeater Order` (obs 17)
      - `ADBE Vector Repeater Transform` (obs 21)
        - `ADBE Vector Repeater Anchor` (obs 18)
        - `ADBE Vector Repeater Opacity 1` (obs 18)
        - `ADBE Vector Repeater Opacity 2` (obs 18)
        - `ADBE Vector Repeater Position` (obs 22)
        - `ADBE Vector Repeater Rotation` (obs 18)
        - `ADBE Vector Repeater Scale` (obs 18)
    - `ADBE Vector Filter - Trim` (obs 20)
      - `ADBE Vector Trim End` (obs 20)
      - `ADBE Vector Trim Offset` (obs 20)
      - `ADBE Vector Trim Start` (obs 20)
      - `ADBE Vector Trim Type` (obs 20)
    - `ADBE Vector Filter - Twist` (obs 1)
      - `ADBE Vector Twist Angle` (obs 1)
      - `ADBE Vector Twist Center` (obs 1)
    - `ADBE Vector Filter - Wiggler` (obs 2)
      - `ADBE Vector Correlation` (obs 2)
      - `ADBE Vector Random Seed` (obs 2)
      - `ADBE Vector Spatial Phase` (obs 2)
      - `ADBE Vector Temporal Phase` (obs 2)
      - `ADBE Vector Wiggler Transform` (obs 2)
        - `ADBE Vector Wiggler Anchor` (obs 2)
        - `ADBE Vector Wiggler Position` (obs 2)
        - `ADBE Vector Wiggler Rotation` (obs 2)
        - `ADBE Vector Wiggler Scale` (obs 2)
      - `ADBE Vector Xform Temporal Freq` (obs 2)
    - `ADBE Vector Filter - Zigzag` (obs 2)
      - `ADBE Vector Zigzag Detail` (obs 2)
      - `ADBE Vector Zigzag Points` (obs 2)
      - `ADBE Vector Zigzag Size` (obs 2)
    - `ADBE Vector Graphic - Fill` (obs 96)
      - `ADBE Vector Blend Mode` (obs 90)
      - `ADBE Vector Composite Order` (obs 90)
      - `ADBE Vector Fill Color` (obs 100)
      - `ADBE Vector Fill Opacity` (obs 90)
      - `ADBE Vector Fill Rule` (obs 90)
    - `ADBE Vector Graphic - Stroke` (obs 112)
      - `ADBE Vector Blend Mode` (obs 95)
      - `ADBE Vector Composite Order` (obs 95)
      - `ADBE Vector Stroke Color` (obs 108)
      - `ADBE Vector Stroke Dashes` (obs 112)
        - `ADBE Vector Stroke Dash 1` (obs 97)
        - `ADBE Vector Stroke Dash 2` (obs 97)
        - `ADBE Vector Stroke Dash 3` (obs 97)
        - `ADBE Vector Stroke Gap 1` (obs 97)
        - `ADBE Vector Stroke Gap 2` (obs 97)
        - `ADBE Vector Stroke Gap 3` (obs 97)
        - `ADBE Vector Stroke Offset` (obs 97)
      - `ADBE Vector Stroke Line Cap` (obs 109)
      - `ADBE Vector Stroke Line Join` (obs 109)
      - `ADBE Vector Stroke Miter Limit` (obs 109)
      - `ADBE Vector Stroke Opacity` (obs 108)
      - `ADBE Vector Stroke Taper` (obs 112)
        - `ADBE Vector Taper End Ease` (obs 110)
        - `ADBE Vector Taper End Length` (obs 110)
        - `ADBE Vector Taper End Width` (obs 110)
        - `ADBE Vector Taper EndWidthPx` (obs 97)
        - `ADBE Vector Taper Length Units` (obs 97)
        - `ADBE Vector Taper Start Ease` (obs 110)
        - `ADBE Vector Taper Start Length` (obs 110)
        - `ADBE Vector Taper Start Width` (obs 110)
        - `ADBE Vector Taper StartWidthPx` (obs 97)
      - `ADBE Vector Stroke Wave` (obs 112)
        - `ADBE Vector Taper Wave Amount` (obs 110)
        - `ADBE Vector Taper Wave Cycles` (obs 97)
        - `ADBE Vector Taper Wave Phase` (obs 110)
        - `ADBE Vector Taper Wave Units` (obs 97)
        - `ADBE Vector Taper Wavelength` (obs 110)
      - `ADBE Vector Stroke Width` (obs 112)
    - `ADBE Vector Group` (obs 65)
    - `ADBE Vector Shape - Ellipse` (obs 56)
      - `ADBE Vector Ellipse Position` (obs 55)
      - `ADBE Vector Ellipse Size` (obs 58)
      - `ADBE Vector Shape Direction` (obs 55)
    - `ADBE Vector Shape - Group` (obs 49)
      - `ADBE Vector Shape` (obs 50)
      - `ADBE Vector Shape Direction` (obs 29)
    - `ADBE Vector Shape - Rect` (obs 73)
      - `ADBE Vector Rect Position` (obs 71)
      - `ADBE Vector Rect Roundness` (obs 71)
      - `ADBE Vector Rect Size` (obs 73)
      - `ADBE Vector Shape Direction` (obs 71)
    - `ADBE Vector Shape - Star` (obs 29)
      - `ADBE Vector Shape Direction` (obs 29)
      - `ADBE Vector Star Inner Radius` (obs 29)
      - `ADBE Vector Star Inner Roundess` (obs 29)
      - `ADBE Vector Star Outer Radius` (obs 29)
      - `ADBE Vector Star Outer Roundess` (obs 29)
      - `ADBE Vector Star Points` (obs 29)
      - `ADBE Vector Star Position` (obs 29)
      - `ADBE Vector Star Rotation` (obs 29)
      - `ADBE Vector Star Type` (obs 29)

**`ADBE Text Animators`**
  - `ADBE Text Animator` (obs 545)
    - `ADBE Text Animator Properties` (obs 9)
      - `ADBE 3DText Back Ambient` (obs 486)
      - `ADBE 3DText Back Bright` (obs 486)
      - `ADBE 3DText Back Diffuse` (obs 486)
      - `ADBE 3DText Back Fresnel` (obs 486)
      - `ADBE 3DText Back Gloss` (obs 486)
      - `ADBE 3DText Back Hue` (obs 486)
      - `ADBE 3DText Back IOR` (obs 486)
      - `ADBE 3DText Back Metal` (obs 486)
      - `ADBE 3DText Back Opacity` (obs 486)
      - `ADBE 3DText Back RGB` (obs 486)
      - `ADBE 3DText Back Reflection` (obs 486)
      - `ADBE 3DText Back Sat` (obs 486)
      - `ADBE 3DText Back Shininess` (obs 486)
      - `ADBE 3DText Back Specular` (obs 486)
      - `ADBE 3DText Back XparRoll` (obs 486)
      - `ADBE 3DText Back Xparency` (obs 486)
      - `ADBE 3DText Bevel Ambient` (obs 486)
      - `ADBE 3DText Bevel Bright` (obs 486)
      - `ADBE 3DText Bevel Depth` (obs 486)
      - `ADBE 3DText Bevel Diffuse` (obs 486)
      - `ADBE 3DText Bevel Fresnel` (obs 486)
      - `ADBE 3DText Bevel Gloss` (obs 486)
      - `ADBE 3DText Bevel Hue` (obs 486)
      - `ADBE 3DText Bevel IOR` (obs 486)
      - `ADBE 3DText Bevel Metal` (obs 486)
      - `ADBE 3DText Bevel Opacity` (obs 486)
      - `ADBE 3DText Bevel RGB` (obs 486)
      - `ADBE 3DText Bevel Reflection` (obs 486)
      - `ADBE 3DText Bevel Sat` (obs 486)
      - `ADBE 3DText Bevel Shininess` (obs 486)
      - `ADBE 3DText Bevel Specular` (obs 486)
      - `ADBE 3DText Bevel XparRoll` (obs 486)
      - `ADBE 3DText Bevel Xparency` (obs 486)
      - `ADBE 3DText Extrude Depth` (obs 486)
      - `ADBE 3DText Front Ambient` (obs 486)
      - `ADBE 3DText Front Bright` (obs 486)
      - `ADBE 3DText Front Diffuse` (obs 486)
      - `ADBE 3DText Front Fresnel` (obs 486)
      - `ADBE 3DText Front Gloss` (obs 486)
      - `ADBE 3DText Front Hue` (obs 486)
      - `ADBE 3DText Front IOR` (obs 486)
      - `ADBE 3DText Front Metal` (obs 486)
      - `ADBE 3DText Front Opacity` (obs 486)
      - `ADBE 3DText Front RGB` (obs 486)
      - `ADBE 3DText Front Reflection` (obs 486)
      - `ADBE 3DText Front Sat` (obs 486)
      - `ADBE 3DText Front Shininess` (obs 486)
      - `ADBE 3DText Front Specular` (obs 486)
      - `ADBE 3DText Front XparRoll` (obs 486)
      - `ADBE 3DText Front Xparency` (obs 486)
      - `ADBE 3DText Side Ambient` (obs 486)
      - `ADBE 3DText Side Bright` (obs 486)
      - `ADBE 3DText Side Diffuse` (obs 486)
      - `ADBE 3DText Side Fresnel` (obs 486)
      - `ADBE 3DText Side Gloss` (obs 486)
      - `ADBE 3DText Side Hue` (obs 486)
      - `ADBE 3DText Side IOR` (obs 486)
      - `ADBE 3DText Side Metal` (obs 486)
      - `ADBE 3DText Side Opacity` (obs 486)
      - `ADBE 3DText Side RGB` (obs 486)
      - `ADBE 3DText Side Reflection` (obs 486)
      - `ADBE 3DText Side Sat` (obs 486)
      - `ADBE 3DText Side Shininess` (obs 486)
      - `ADBE 3DText Side Specular` (obs 486)
      - `ADBE 3DText Side XparRoll` (obs 486)
      - `ADBE 3DText Side Xparency` (obs 486)
      - `ADBE Text Anchor Point` (obs 37)
      - `ADBE Text Anchor Point 3D` (obs 488)
      - `ADBE Text Blur` (obs 488)
      - `ADBE Text Character Change Type` (obs 525)
      - `ADBE Text Character Offset` (obs 525)
      - `ADBE Text Character Range` (obs 525)
      - `ADBE Text Character Replace` (obs 525)
      - `ADBE Text Fill Brightness` (obs 525)
      - `ADBE Text Fill Color` (obs 525)
      - `ADBE Text Fill Hue` (obs 525)
      - `ADBE Text Fill Opacity` (obs 525)
      - `ADBE Text Fill Saturation` (obs 525)
      - `ADBE Text Line Anchor` (obs 525)
      - `ADBE Text Line Spacing` (obs 525)
      - `ADBE Text Opacity` (obs 526)
      - `ADBE Text Position` (obs 37)
      - `ADBE Text Position 3D` (obs 488)
      - `ADBE Text Rotation` (obs 525)
      - `ADBE Text Rotation X` (obs 488)
      - `ADBE Text Rotation Y` (obs 488)
      - `ADBE Text Scale` (obs 39)
      - `ADBE Text Scale 3D` (obs 488)
      - `ADBE Text Skew` (obs 525)
      - `ADBE Text Skew Axis` (obs 525)
      - `ADBE Text Stroke Brightness` (obs 525)
      - `ADBE Text Stroke Color` (obs 526)
      - `ADBE Text Stroke Hue` (obs 525)
      - `ADBE Text Stroke Opacity` (obs 526)
      - `ADBE Text Stroke Saturation` (obs 525)
      - `ADBE Text Stroke Width` (obs 526)
      - `ADBE Text Track Type` (obs 525)
      - `ADBE Text Tracking Amount` (obs 525)
      - `ADBE Text VF Axis 1` (obs 3)
    - `ADBE Text Selectors` (obs 14)
      - `ADBE Text Expressible Selector` (obs 41)
        - `ADBE Text Expressible Amount` (obs 41)
        - `ADBE Text Range Type2` (obs 41)
      - `ADBE Text Selector` (obs 481)
        - `ADBE Text Index End` (obs 467)
        - `ADBE Text Index Offset` (obs 467)
        - `ADBE Text Index Start` (obs 467)
        - `ADBE Text Percent End` (obs 469)
        - `ADBE Text Percent Offset` (obs 469)
        - `ADBE Text Percent Start` (obs 467)
        - `ADBE Text Range Advanced` (obs 477)
      - `ADBE Text Wiggly Selector` (obs 82)
        - `ADBE Text Character Correlation` (obs 82)
        - `ADBE Text Range Type2` (obs 82)
        - `ADBE Text Selector Mode` (obs 82)
        - `ADBE Text Spatial Phase` (obs 82)
        - `ADBE Text Temporal Freq` (obs 82)
        - `ADBE Text Temporal Phase` (obs 82)
        - `ADBE Text Wiggly Lock Dim` (obs 82)
        - `ADBE Text Wiggly Max Amount` (obs 82)
        - `ADBE Text Wiggly Min Amount` (obs 82)
        - `ADBE Text Wiggly Random Seed` (obs 76)

**`ADBE Text Animator`**
  - `ADBE Text Animator Properties` (obs 9)
    - `ADBE 3DText Back Ambient` (obs 486)
    - `ADBE 3DText Back Bright` (obs 486)
    - `ADBE 3DText Back Diffuse` (obs 486)
    - `ADBE 3DText Back Fresnel` (obs 486)
    - `ADBE 3DText Back Gloss` (obs 486)
    - `ADBE 3DText Back Hue` (obs 486)
    - `ADBE 3DText Back IOR` (obs 486)
    - `ADBE 3DText Back Metal` (obs 486)
    - `ADBE 3DText Back Opacity` (obs 486)
    - `ADBE 3DText Back RGB` (obs 486)
    - `ADBE 3DText Back Reflection` (obs 486)
    - `ADBE 3DText Back Sat` (obs 486)
    - `ADBE 3DText Back Shininess` (obs 486)
    - `ADBE 3DText Back Specular` (obs 486)
    - `ADBE 3DText Back XparRoll` (obs 486)
    - `ADBE 3DText Back Xparency` (obs 486)
    - `ADBE 3DText Bevel Ambient` (obs 486)
    - `ADBE 3DText Bevel Bright` (obs 486)
    - `ADBE 3DText Bevel Depth` (obs 486)
    - `ADBE 3DText Bevel Diffuse` (obs 486)
    - `ADBE 3DText Bevel Fresnel` (obs 486)
    - `ADBE 3DText Bevel Gloss` (obs 486)
    - `ADBE 3DText Bevel Hue` (obs 486)
    - `ADBE 3DText Bevel IOR` (obs 486)
    - `ADBE 3DText Bevel Metal` (obs 486)
    - `ADBE 3DText Bevel Opacity` (obs 486)
    - `ADBE 3DText Bevel RGB` (obs 486)
    - `ADBE 3DText Bevel Reflection` (obs 486)
    - `ADBE 3DText Bevel Sat` (obs 486)
    - `ADBE 3DText Bevel Shininess` (obs 486)
    - `ADBE 3DText Bevel Specular` (obs 486)
    - `ADBE 3DText Bevel XparRoll` (obs 486)
    - `ADBE 3DText Bevel Xparency` (obs 486)
    - `ADBE 3DText Extrude Depth` (obs 486)
    - `ADBE 3DText Front Ambient` (obs 486)
    - `ADBE 3DText Front Bright` (obs 486)
    - `ADBE 3DText Front Diffuse` (obs 486)
    - `ADBE 3DText Front Fresnel` (obs 486)
    - `ADBE 3DText Front Gloss` (obs 486)
    - `ADBE 3DText Front Hue` (obs 486)
    - `ADBE 3DText Front IOR` (obs 486)
    - `ADBE 3DText Front Metal` (obs 486)
    - `ADBE 3DText Front Opacity` (obs 486)
    - `ADBE 3DText Front RGB` (obs 486)
    - `ADBE 3DText Front Reflection` (obs 486)
    - `ADBE 3DText Front Sat` (obs 486)
    - `ADBE 3DText Front Shininess` (obs 486)
    - `ADBE 3DText Front Specular` (obs 486)
    - `ADBE 3DText Front XparRoll` (obs 486)
    - `ADBE 3DText Front Xparency` (obs 486)
    - `ADBE 3DText Side Ambient` (obs 486)
    - `ADBE 3DText Side Bright` (obs 486)
    - `ADBE 3DText Side Diffuse` (obs 486)
    - `ADBE 3DText Side Fresnel` (obs 486)
    - `ADBE 3DText Side Gloss` (obs 486)
    - `ADBE 3DText Side Hue` (obs 486)
    - `ADBE 3DText Side IOR` (obs 486)
    - `ADBE 3DText Side Metal` (obs 486)
    - `ADBE 3DText Side Opacity` (obs 486)
    - `ADBE 3DText Side RGB` (obs 486)
    - `ADBE 3DText Side Reflection` (obs 486)
    - `ADBE 3DText Side Sat` (obs 486)
    - `ADBE 3DText Side Shininess` (obs 486)
    - `ADBE 3DText Side Specular` (obs 486)
    - `ADBE 3DText Side XparRoll` (obs 486)
    - `ADBE 3DText Side Xparency` (obs 486)
    - `ADBE Text Anchor Point` (obs 37)
    - `ADBE Text Anchor Point 3D` (obs 488)
    - `ADBE Text Blur` (obs 488)
    - `ADBE Text Character Change Type` (obs 525)
    - `ADBE Text Character Offset` (obs 525)
    - `ADBE Text Character Range` (obs 525)
    - `ADBE Text Character Replace` (obs 525)
    - `ADBE Text Fill Brightness` (obs 525)
    - `ADBE Text Fill Color` (obs 525)
    - `ADBE Text Fill Hue` (obs 525)
    - `ADBE Text Fill Opacity` (obs 525)
    - `ADBE Text Fill Saturation` (obs 525)
    - `ADBE Text Line Anchor` (obs 525)
    - `ADBE Text Line Spacing` (obs 525)
    - `ADBE Text Opacity` (obs 526)
    - `ADBE Text Position` (obs 37)
    - `ADBE Text Position 3D` (obs 488)
    - `ADBE Text Rotation` (obs 525)
    - `ADBE Text Rotation X` (obs 488)
    - `ADBE Text Rotation Y` (obs 488)
    - `ADBE Text Scale` (obs 39)
    - `ADBE Text Scale 3D` (obs 488)
    - `ADBE Text Skew` (obs 525)
    - `ADBE Text Skew Axis` (obs 525)
    - `ADBE Text Stroke Brightness` (obs 525)
    - `ADBE Text Stroke Color` (obs 526)
    - `ADBE Text Stroke Hue` (obs 525)
    - `ADBE Text Stroke Opacity` (obs 526)
    - `ADBE Text Stroke Saturation` (obs 525)
    - `ADBE Text Stroke Width` (obs 526)
    - `ADBE Text Track Type` (obs 525)
    - `ADBE Text Tracking Amount` (obs 525)
    - `ADBE Text VF Axis 1` (obs 3)
  - `ADBE Text Selectors` (obs 14)
    - `ADBE Text Expressible Selector` (obs 41)
      - `ADBE Text Expressible Amount` (obs 41)
      - `ADBE Text Range Type2` (obs 41)
    - `ADBE Text Selector` (obs 481)
      - `ADBE Text Index End` (obs 467)
      - `ADBE Text Index Offset` (obs 467)
      - `ADBE Text Index Start` (obs 467)
      - `ADBE Text Percent End` (obs 469)
      - `ADBE Text Percent Offset` (obs 469)
      - `ADBE Text Percent Start` (obs 467)
      - `ADBE Text Range Advanced` (obs 477)
        - `ADBE Text Levels Max Ease` (obs 468)
        - `ADBE Text Levels Min Ease` (obs 468)
        - `ADBE Text Random Seed` (obs 467)
        - `ADBE Text Randomize Order` (obs 468)
        - `ADBE Text Range Shape` (obs 469)
        - `ADBE Text Range Type2` (obs 469)
        - `ADBE Text Range Units` (obs 467)
        - `ADBE Text Selector Max Amount` (obs 467)
        - `ADBE Text Selector Mode` (obs 467)
        - `ADBE Text Selector Smoothness` (obs 467)
    - `ADBE Text Wiggly Selector` (obs 82)
      - `ADBE Text Character Correlation` (obs 82)
      - `ADBE Text Range Type2` (obs 82)
      - `ADBE Text Selector Mode` (obs 82)
      - `ADBE Text Spatial Phase` (obs 82)
      - `ADBE Text Temporal Freq` (obs 82)
      - `ADBE Text Temporal Phase` (obs 82)
      - `ADBE Text Wiggly Lock Dim` (obs 82)
      - `ADBE Text Wiggly Max Amount` (obs 82)
      - `ADBE Text Wiggly Min Amount` (obs 82)
      - `ADBE Text Wiggly Random Seed` (obs 76)

**`ADBE Text Selectors`**
  - `ADBE Text Expressible Selector` (obs 41)
    - `ADBE Text Expressible Amount` (obs 41)
    - `ADBE Text Range Type2` (obs 41)
  - `ADBE Text Selector` (obs 481)
    - `ADBE Text Index End` (obs 467)
    - `ADBE Text Index Offset` (obs 467)
    - `ADBE Text Index Start` (obs 467)
    - `ADBE Text Percent End` (obs 469)
    - `ADBE Text Percent Offset` (obs 469)
    - `ADBE Text Percent Start` (obs 467)
    - `ADBE Text Range Advanced` (obs 477)
      - `ADBE Text Levels Max Ease` (obs 468)
      - `ADBE Text Levels Min Ease` (obs 468)
      - `ADBE Text Random Seed` (obs 467)
      - `ADBE Text Randomize Order` (obs 468)
      - `ADBE Text Range Shape` (obs 469)
      - `ADBE Text Range Type2` (obs 469)
      - `ADBE Text Range Units` (obs 467)
      - `ADBE Text Selector Max Amount` (obs 467)
      - `ADBE Text Selector Mode` (obs 467)
      - `ADBE Text Selector Smoothness` (obs 467)
  - `ADBE Text Wiggly Selector` (obs 82)
    - `ADBE Text Character Correlation` (obs 82)
    - `ADBE Text Range Type2` (obs 82)
    - `ADBE Text Selector Mode` (obs 82)
    - `ADBE Text Spatial Phase` (obs 82)
    - `ADBE Text Temporal Freq` (obs 82)
    - `ADBE Text Temporal Phase` (obs 82)
    - `ADBE Text Wiggly Lock Dim` (obs 82)
    - `ADBE Text Wiggly Max Amount` (obs 82)
    - `ADBE Text Wiggly Min Amount` (obs 82)
    - `ADBE Text Wiggly Random Seed` (obs 76)

**`ADBE Text Selector`**
  - `ADBE Text Index End` (obs 467)
  - `ADBE Text Index Offset` (obs 467)
  - `ADBE Text Index Start` (obs 467)
  - `ADBE Text Percent End` (obs 469)
  - `ADBE Text Percent Offset` (obs 469)
  - `ADBE Text Percent Start` (obs 467)
  - `ADBE Text Range Advanced` (obs 477)
    - `ADBE Text Levels Max Ease` (obs 468)
    - `ADBE Text Levels Min Ease` (obs 468)
    - `ADBE Text Random Seed` (obs 467)
    - `ADBE Text Randomize Order` (obs 468)
    - `ADBE Text Range Shape` (obs 469)
    - `ADBE Text Range Type2` (obs 469)
    - `ADBE Text Range Units` (obs 467)
    - `ADBE Text Selector Max Amount` (obs 467)
    - `ADBE Text Selector Mode` (obs 467)
    - `ADBE Text Selector Smoothness` (obs 467)

**`ADBE Text More Options`**
  - `ADBE Text Anchor Point Align` (obs 58)
  - `ADBE Text Anchor Point Option` (obs 17)
  - `ADBE Text Character Blend Mode` (obs 1)
  - `ADBE Text Render Order` (obs 2)
  - `ADBE Text Variable Font Spacing` (obs 3)

**`ADBE Text Path Options`**
  - `ADBE Text First Margin` (obs 6)
  - `ADBE Text Force Align Path` (obs 6)
  - `ADBE Text Last Margin` (obs 6)
  - `ADBE Text Path` (obs 6)
  - `ADBE Text Perpendicular To Path` (obs 6)
  - `ADBE Text Reverse Path` (obs 6)

**Effect-instance roots observed carrying `ADBE Effect Built In Params` (55):** `ADBE Angle Control`, `ADBE Aud Compressor`, `ADBE Aud Modulator`, `ADBE Aud Reverb`, `ADBE Aud Tone`, `ADBE Bevel Alpha`, `ADBE Box Blur2`, `ADBE CM Animated Shape 2`, `ADBE CM Animated Shape 3`, `ADBE CM Animated Shape Control`, `ADBE Calculations`, `ADBE Checkbox Control`, `ADBE Checkerboard`, `ADBE Color Control`, `ADBE Color Emboss`, `ADBE Drop Shadow`, `ADBE Easy Levels2`, `ADBE Exposure2`, `ADBE Fill`, `ADBE Find Edges`, `ADBE Fractal Noise`, `ADBE Geometry2`, `ADBE Glo2`, `ADBE HUE SATURATION`, `ADBE MESH WARP`, `ADBE Minimax`, `ADBE Noise2`, `ADBE PS Median`, `ADBE Point Control`, `ADBE Polar Coordinates`, `ADBE Posterize`, `ADBE Radial Blur`, `ADBE Ramp`, `ADBE Ripple`, `ADBE Roughen Edges`, `ADBE Set Matte3`, `ADBE Simple Choker`, `ADBE Slider Control`, `ADBE Solid Composite`, `ADBE Turbulent Displace`, `ADBE Twirl`, `ADBE Unmult`, `ADBE Venetian Blinds`, `APC CardWipeCam`, `APC Colorama`, `APC Foam`, `CC Blobbylize`, `CC Glass`, `CC Light Sweep`, `CC Mr. Mercury`, `CC Radial Fast Blur`, `CC RepeTile`, `CS Composite`, `Pseudo/171912`, `Pseudo/932602`

**[STU-MOT-013] The transform group is normative and its members are:** anchor point, position
(with separated X, Y and Z sub-streams observable in the recovered documents), scale, rotation
about Z, rotation about X, rotation about Y, orientation (3D), and opacity. Rotation properties are
UNWRAPPED: they accumulate past 360 degrees and store full revolutions plus a residual angle, so a
keyframe pair from 350 to 370 degrees animates 20 degrees forward rather than 330 backward. This is
not a display convention; it is a storage requirement, and collapsing rotation into a 0..360 range
destroys animation.

**[STU-MOT-013a]** Anchor point is the pivot for rotation and scale. It is a property of the layer, is
keyframable like any other, and moving it moves the layer's content relative to its position unless
the operator uses the anchor-point tool, which compensates position to keep the content still.
Both behaviours are typed commands; neither is implicit.

**[STU-MOT-014] 3D material and geometry options are property groups on the layer**, not renderer
settings. The recovered material members are: accepts lights, accepts shadows, casts shadows,
appears in reflections, light transmission, ambient coefficient, diffuse coefficient, specular
coefficient (intensity), specular shininess, metal coefficient, reflection coefficient, glossiness
coefficient, fresnel coefficient, index of refraction, transparency coefficient, transparency
rolloff, and shadow colour. The recovered geometry members are: extrusion depth, bevel depth, bevel
style, bevel direction, plane curvature and plane subdivision. Every one is keyframable. Which of
them a renderer honours is the renderer's declaration ([STU-CMP-060]), and a property that the
active renderer ignores MUST be shown as inactive with a stated reason, never hidden.

**[STU-MOT-016] The audio group carries `audio_levels`**, a keyframable stereo-pair level stream.
Audio level is animated on the same keyframe model as every other property; there is no separate
audio automation format ([STU-FX-135]).

[STU-MOT-017] **An effect instance is a property GROUP in the layer's tree, and its parameters are
that group's children.** This is what makes [STU-FX-011b] work without a bridge: an effect
parameter is already a `StudioProperty`, so keyframing it is the ordinary operation. Every effect
instance additionally carries a built-in parameter group holding, at minimum, an effect mask
collection, an effect-mask opacity and a CPU/GPU force selector; these are Studio-level properties
present on every effect regardless of its own parameter list, and they are the mechanism behind [STU-FX-004]'s
per-entry effect mask.

**[STU-MOT-019] The data group binds an external data source to properties.** A data layer exposes
`data_value`, `data_key_count`, `data_key_times` and `data_key_values` streams addressed by a
`data_path`, so a property can be driven by a structured data file rather than by hand-authored
keyframes. Studio's contract: the data source is a content-addressed artifact reference, the bind
is re-evaluated when the artifact changes, and a missing or malformed source yields a determinate
`DATA_SOURCE_UNAVAILABLE` result and the property falls back to its static value rather than to
zero.

---

## 14.26.3 Time and the keyframe model

### 1. Composition time

**[STU-MOT-020] A composition's time model.** Time in a composition is expressed in seconds as a
rational over the composition's frame rate, and is quantised to frame boundaries for keyframe
placement by default with an explicit sub-frame mode available. Every property's keyframe times,
the work area, layer in/out points, markers and the playhead share this one time base. A
composition placed inside a sequence maps its time to sequence ticks at the boundary
([STU-VID-012]); the mapping is exact when the frame rates divide and is otherwise resampled by the
declared frame-blending method ([STU-MOT-007a]), never by silent nearest-frame selection.

**[STU-MOT-020a] The value-stream storage contract.** A property's animated value is a stream, and
the normative storage shape is: an ordered keyframe list, a key count, a fixed per-key record size,
and per-key big-endian double-precision components. The recovered on-disk shape confirms the
requirement precisely: keyframes hold `time`, an in-interpolation code, an out-interpolation code,
and `(record_size - 8) / 8` double values of which the leading entries are the value, one per
stream component, and the remainder are tangents. Studio stores doubles, not floats: a 32-bit float
cannot hold a frame-accurate time at a long duration, and cannot round-trip a tangent without
visible drift on a slow ease.

**[STU-MOT-020b]** A property that is NOT animated stores a single static value array of the same
component count. A property that is animated ADDITIONALLY retains its static value, so disabling
every keyframe returns exactly the prior static value rather than the value at frame zero.

**[STU-MOT-020c]** A stream MAY declare its own minimum and maximum bounds as separate fields alongside
the values. These are the property's `hard_min`/`hard_max` and follow [STU-FX-104] and [STU-FX-106]
without exception.

### 2. The keyframe record

**[STU-MOT-030] `StudioKeyframe` (schema id `hsk.studio.keyframe@1`).** A keyframe is a value at a
time with independent incoming and outgoing interpolation. Required fields:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Contract |
|---|---|
| `time` | Composition time. Unique within its property; two keyframes may not share a time. |
| `value` | One value per stream component. |
| `in_interpolation` | `StudioInterpolation`, see [STU-MOT-031]. Governs the segment ARRIVING at this keyframe. |
| `out_interpolation` | `StudioInterpolation`. Governs the segment LEAVING this keyframe. |
| `in_temporal_tangent` | Per component: `{speed, influence}`. See [STU-MOT-033]. |
| `out_temporal_tangent` | Per component: `{speed, influence}`. |
| `in_spatial_tangent` | Present only when the property `is_spatial`. A vector offset. See [STU-MOT-036]. |
| `out_spatial_tangent` | Present only when the property `is_spatial`. |
| `roving` | Bool. See [STU-MOT-035]. |
| `label_color` | Optional; keyframes are selectable objects and may be labelled. |

**[STU-MOT-030a] In and out interpolation are INDEPENDENT and this is not optional.** The recovered
keyframe records store them as two separate single-byte codes, and the observed code pairs are
overwhelmingly asymmetric: of the distinct in/out pairs tabulated across 3,217 shipped keyframes,
the three symmetric pairs `1,1`, `2,2` and `3,3` account for 3,250 occurrences while several
hundred distinct asymmetric pairs account for the rest. A keyframe model with one interpolation
field per keyframe cannot represent "ease out of this keyframe but arrive at it linearly", which is
the single most common hand-animation gesture. One field per keyframe is a specification error.

### 3. Interpolation types

**[STU-MOT-031] The normative `StudioInterpolation` enumeration is five members.**

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Member | Behaviour |
|---|---|
| `linear` | Constant rate between the two keyframes. |
| `bezier` | Rate shaped by an independently editable tangent on each side of the keyframe. The two sides do not constrain each other, so a corner in the value curve is expressible. |
| `continuous_bezier` | Bezier whose two tangents are constrained to be collinear, so the curve passes through the keyframe smoothly. Adjusting one side rotates the other. |
| `auto_bezier` | Continuous bezier whose tangent direction is computed from the neighbouring keyframes and re-computed when they move. Editing either tangent by hand converts it to `continuous_bezier`. |
| `hold` | The value is held at this keyframe until the next one, then jumps. A `hold` out-interpolation makes the segment a step regardless of the next keyframe's in-interpolation. |

**[STU-MOT-031a]** The interpolation menu additionally offers a `current_settings` selection which is a
UI convenience meaning "leave each selected keyframe as it is"; it is NOT an interpolation type and
MUST NOT appear in the stored enumeration.

**[STU-MOT-031b]** A property whose `interpolable` flag is false ([STU-FX-113]) behaves as `hold` on
every segment regardless of the stored interpolation, and its interpolation control is shown
inactive with that reason rather than hidden.

### 4. Temporal tangents

[STU-MOT-033] **A temporal tangent is `{speed, influence}` per component, and both halves are
required.** `speed` is the rate of value change at the keyframe, in the property's own units per
second. `influence` is the proportion of the segment's duration over which that speed dominates,
expressed as a fraction in 0..1 (presented as a percentage). Two keyframes and their four tangent
halves fully determine the segment; nothing else is stored.

**[STU-MOT-034] The default ease influence is 0.16666666666.** This value appears verbatim in
shipped keyframe records and corresponds to 16.667 percent -- one sixth of the segment. It is the
influence assigned by the ease keyframe assistants ([STU-MOT-037]) and is Studio's default for a
newly-created bezier tangent. It is stated to this precision because a spec that says "about one
sixth" produces animation that does not match imported material, and because 1/6 exactly and
0.16666666666 differ in the last places in a way that is visible when comparing two renders
frame-by-frame at a tolerance tight enough to be worth having.

**[STU-MOT-034a]** Studio stores influence as the decimal fraction, not as a percentage integer, and
its `precision` is at least 5 decimal places ([STU-FX-109]).

### 5. Spatial tangents and motion paths

[STU-MOT-036] **A spatial property has spatial tangents in addition to temporal ones, and they are
a different thing.** A position keyframe carries a temporal tangent describing how FAST the value
changes and a spatial tangent describing WHICH WAY the path curves. Editing one does not change the
other. The composition viewer renders a spatial property's keyframes as a motion path: a curve
through the keyframe positions with a dot per frame, so dot spacing shows speed. Dragging a
keyframe moves the value; dragging a spatial tangent handle bends the path; dragging in the graph
editor changes timing. [STU-PRO-031] already requires the motion path in the prototyping surface;
this clause states the underlying model it draws.

**[STU-MOT-036a]** Spatial interpolation has its own type selection, independent of temporal
interpolation: `linear` produces straight path segments with corners at keyframes, `bezier` and
`auto_bezier` produce curves. A property may legally be temporally linear and spatially bezier, or
the reverse.

**[STU-MOT-036b] DECLARED GAP.** The exact on-disk layout of spatial tangents inside the recovered
keyframe record is NOT proven. The record's trailing double values are known to hold tangents, and
the expression vocabulary confirms that spatial streams expose `inTangents` and `outTangents`, but
the shipped preset corpus read exercised only temporal keyframes, so no spatial tangent was
observed in place. Studio's spatial tangent model above is specified from the expression surface
and from the motion-path behaviour, and the IMPORT mapping for spatial tangents must be established
against real spatially-animated documents before an importer claims fidelity. This is declared gap [STU-MOT-141].

### 6. Roving keyframes

**[STU-MOT-035] A roving keyframe holds a value but not a time.** When `roving` is true, the
keyframe's position on the timeline is recomputed so that speed is constant across the keyframes on
either side of it, letting an operator place a path through several points and get even motion
without hand-timing each one. The first and last keyframes of a property may never rove. Roving is
meaningful only on spatial properties.

### 7. Keyframe assistants

**[STU-MOT-037] The normative ease assistant set is three operations**, each applying [STU-MOT-034]'s
default influence:

- **Ease** -- sets both in and out interpolation of the selected keyframes to bezier and both
  influences to the default.
- **Ease In** -- the incoming side only.
- **Ease Out** -- the outgoing side only.

A fourth operation toggles `hold` on the selected keyframes' outgoing interpolation. These are the
operations behind the shared easing catalogue of [STU-PRO-019]: a named easing preset in that
catalogue resolves to a tangent pair here, so a spring or a custom cubic-bezier chosen in the
prototyping surface and an ease applied here are the same stored data.

### 8. Keyframe operations

**[STU-MOT-038] The normative keyframe operation set.** Each is a typed command and one reversible
history step.

*Derivation: catalogue table, splits per row; yields 15 microtasks, one per keyframe operation.*

| Operation | Contract |
|---|---|
| Add or delete at current time | Per property, toggling. Adding captures the property's current evaluated value, so adding a keyframe never changes the render. |
| Add and reveal | Adds a keyframe and expands the property in the outline. |
| Enable animation on a property | Converts `static` to `keyframed` by creating the first keyframe at the current time. |
| Disable animation | Converts back to `static`, discarding keyframes, with the retained static value of [STU-MOT-020b]. |
| Select all keyframes on a property, layer, or composition | Three scopes, separately addressable. |
| Select visible keyframes and exposed properties | Scoped to what the outline currently shows. |
| Deselect all keyframes and properties | |
| Go to next / previous keyframe | Two variants: across all visible properties, and restricted to selected layers or properties. |
| Shift selected keyframes by one frame / by ten frames, earlier or later | The ten-frame step is the shared multi-step preference ([STU-VID-035a]). |
| Drag to move in time | Snaps to the playhead, to other keyframes, and to layer in/out points. |
| Copy, paste, and REVERSE paste | Reverse paste inverts the time order of the pasted keyframes. It is a distinct operation, not a modifier. |
| Set interpolation | Per side, per selection, from the [STU-MOT-031] enumeration. |
| Edit interpolation numerically | Opens the typed interpolation record for the selection ([STU-MOT-055]). |
| Edit velocity numerically | Opens the typed tangent record for the selection ([STU-MOT-055]). |
| Show properties with keyframes | Filters the outline to animated properties; an extend variant adds to the current filter. |

[STU-MOT-039] **A keyframe operation on a multi-component property applies per component unless the
property is unseparated**, in which case the components move together. This is why
`dimensions_separated` ([STU-MOT-010a]) is a stored property rather than a view mode.

---

## 14.26.4 The graph editor

**[STU-MOT-050] The graph editor is a REQUIRED surface, not an advanced option.** A timeline that
shows keyframes as markers on a bar can express when a value changes but not how, and an animator
cannot do professional work without seeing and editing the curve. Studio's timeline has two display
modes over the same data -- the LAYER BAR view and the GRAPH EDITOR -- toggled by one command, and
every keyframe operation of [STU-MOT-038] works in both.

**[STU-MOT-051] The graph editor has two graph types and they show different things.**

- **Value graph.** The vertical axis is the property's value. The curve's shape is the value over
  time; a bezier tangent handle is visible and draggable directly.
- **Speed graph.** The vertical axis is the rate of change. A constant-speed segment is a
  horizontal line; an ease is a curve to and from zero. This graph is the only practical way to
  match the speed of two different properties, and it is where `influence` is manipulated
  intuitively.

A multi-component property shows one curve per component, individually selectable, and a colour per
component that is stable across the session.

**[STU-MOT-052] Normative graph editor operations:** choose which properties are graphed (selected
layers, selected properties, or all animated properties); auto-fit the vertical axis to the visible
curves, or fit-all, or set the range manually; box-select and lasso-select keyframes; drag
keyframes in two dimensions (time and value simultaneously) with axis constraint modifiers; drag
tangent handles with an option to break or maintain continuity; snap to frame boundaries with a
toggle; show or hide the reference graph of non-selected properties; show the audio waveform behind
the graph; and apply the ease assistants of [STU-MOT-037] to the selection.

**[STU-MOT-053] The graph editor is a projection of the property tree, not a separate document.**
Everything it edits is a `StudioKeyframe` field. There is no graph-editor-only data, so a model that
sets tangents through the typed command surface produces exactly what a hand-drag produces
([STU-DOC-004]).

**[STU-MOT-055] Two numeric dialogs are required and they are the model surface's shape.** A
keyframe INTERPOLATION record exposes, for the selection: in-interpolation type, out-interpolation
type, and for spatial properties the spatial interpolation type plus the roving flag. A keyframe
VELOCITY record exposes, per component: incoming speed, incoming influence, outgoing speed,
outgoing influence, and a continuity toggle linking the two sides. Both are typed records reachable
headlessly, which is what makes tangent authorship model-steerable at all; a tangent editable only
by dragging a handle is not.

---

## 14.26.5 Time remapping

**[STU-MOT-045] Time remapping turns a layer's source time into a keyframable property.** Enabling
it on a footage or composition layer creates a `time_remap` property whose value AT a composition
time is the SOURCE time to sample. Normative contract:

1. Enabling creates exactly two keyframes: source time 0 at the layer's in point, and the source's
   final frame at its out point, so enabling changes nothing about the render.
2. The property is an ordinary `StudioProperty` and every keyframe operation, interpolation type
   and expression applies to it.
3. A decreasing segment plays the source backwards. A flat segment freezes. A steeper segment plays
   faster.
4. The layer's `out_point` is no longer bounded by the source duration once time remapping is
   enabled; the layer may extend arbitrarily and the remap decides what it shows.
5. Values outside the source's range are clamped to the first or last frame; this is a clamp, not
   an error, and is the one place in Studio where a hard bound clamps silently, because the
   alternative -- refusing to render -- is worse and because the clamp is visible in the result.
6. Frame blending ([STU-MOT-007a]) governs what happens between source frames under a non-integer
   remap.
7. Clip-level speed and time remap in the sequence surface ([STU-VID-031a]) resolve to this same
   property; they are two entry points to one mechanism.

---

## 14.26.6 The expression language

This group specifies a programming language surface. It is the third of the four things this
sub-section owns that nothing else in Section 14 provides, and it is what makes procedural motion,
rigging and data-driven graphics possible at all.

### 1. Expressions replace values

[STU-MOT-070] **An expression is a FIRST-CLASS ALTERNATIVE to a property's value, not an
annotation on it.** When a property is expression-driven, the expression's return value IS the
property's value at every evaluated time. The property's keyframes, if any, remain stored and are
reachable from inside the expression as a base value, which is how "keyframed animation plus a
procedural wobble" is expressed. The expression is stored as NUL-terminated UTF-8 source attached
to the value stream, alongside rather than instead of the keyframes.

**[STU-MOT-071] The four property states are normative and form a state machine**
([STU-MOT-010a]'s `state` field):

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| State | Value comes from | Keyframes | Expression |
|---|---|---|---|
| `static` | `static_value` | none | none |
| `keyframed` | interpolated keyframes | present | none or present-but-disabled |
| `expression_driven` | expression result | none | present and enabled |
| `expression_over_keyframes` | expression result, with the interpolated keyframe value available to it as the base | present | present and enabled |

Transitions between states are typed commands with stated data effects: adding an expression to a
`keyframed` property moves it to `expression_over_keyframes` and destroys nothing; disabling the
expression returns it to `keyframed`; removing the expression discards the source and requires
confirmation; a "convert expression to keyframes" command bakes the evaluated result at the
composition frame rate into real keyframes and moves the property to `keyframed`, and is a normal
reversible history step.

### 2. The expression-driven control -- the required "animated" state

[STU-MOT-072] **A scrubbable numeric control MUST have an explicit expression-driven state, and
this is a hard UI contract, not a suggestion.** [STU-FX-110] specifies the scrubbable control for a
static parameter. The moment expressions exist, that control has a problem: the value it shows is
computed, so dragging it cannot write where the operator expects. Studio's normative resolution:

1. **The control renders in a distinguishable EXPRESSION-DRIVEN state** whenever
   `state ∈ {expression_driven, expression_over_keyframes}` and `expression_enabled` is true. The
   state is conveyed by more than colour (accessibility), is exposed through AccessKit as a
   read-only-with-reason state, and is reported by the Argus diagnostic ([STU-MOT-150]).
2. **The displayed value is the EVALUATED value at the current time**, refreshed as the playhead
   moves. It is never the stale static value.
3. **Scrubbing or typing into an expression-driven control is REFUSED, not silently ignored and not
   silently applied to the base value.** The refusal is a determinate
   `PROPERTY_IS_EXPRESSION_DRIVEN` result naming the property and offering the three legal
   continuations below. Silently writing to the base value is the worst option available, because
   the operator sees no change and believes the control is broken.
4. **Three legal continuations are offered inline, and each is a typed command:** disable the
   expression (returning the control to the underlying state), edit the expression, or bake the
   expression to keyframes and then edit.
5. **A property that is `keyframed` but not expression-driven behaves differently again:** its
   control is live and editable, and a scrub or typed entry at a time where a keyframe exists
   modifies that keyframe, while a scrub at a time where none exists CREATES one at the playhead if
   the property is in an auto-keyframe recording mode ([STU-PRO-029b]) and otherwise offsets the
   whole animation. Which of those two happens MUST be an explicit, visible, persisted mode, never
   inferred, because both are standard behaviours in the field and an operator who guesses wrong
   destroys an animation without noticing.
6. **Every keyframable property's control carries a keyframe toggle** -- the "stopwatch" affordance
   of [STU-PRO-029] -- which is what moves the property between `static` and `keyframed`, and a
   separate expression toggle which moves it into and out of the expression states. Two toggles,
   two independent axes; conflating them into one control is a specification error.

### 3. The identifier surface

[STU-MOT-073] **The expression language exposes 333 identifiers in 12 declared categories, and the
complete list is normative as the capability target.** These are not a convenience library; they are
the language's standard surface, and an expression system missing the space-transform or the
interpolation families cannot express rigging or easing.

**[STU-MOT-073a] Dual spelling is a real compatibility contract.** The identifier table ships both
a legacy `snake_case` and a modern `camelCase` spelling for most members -- `point_of_interest` and
`pointOfInterest`, `value_at_time` and `valueAtTime`, `to_comp` and `toComp`. Studio's language MUST
accept both spellings and MUST normalise to one canonical spelling on write, so that imported
expressions keep working and newly-authored ones are consistent. Which spelling is canonical is a
named decision, recorded as [STU-MOT-144].

**[STU-MOT-073b] 206 of the 333 identifiers carry no category header on disk.** They form the
table's leading block and are real members -- the camera, light, mask, path, velocity, text-style
and string surfaces are all in it -- but no category can be attributed to them without inventing
one, so none is invented. They are listed as an explicitly uncategorised block below, and
categorising them for the operator-facing reference is a documentation task, not a discovery task.

*Derivation: catalogue table, splits per row; yields 12 microtasks, one per expression identifier category.*

| Category | Count | Identifiers |
|---|---|---|
| Global | 18 | `comp`, `footage`, `this_comp`, `this_layer`, `this_property`, `thisComp`, `thisProject`, `time`, `color_depth`, `colorDepth`, `posterize_time`, `posterizeTime`, `timeToFrames`, `framesToTime`, `timeToTimecode`, `timeToNTSCTimecode`, `timeToFeetAndFrames`, `timeToCurrentFormat` |
| Vector Math | 9 | `add`, `mul`, `clamp`, `dot`, `cross`, `normalize`, `length`, `look_at`, `lookAt` |
| Random Numbers | 6 | `seed_random`, `seedRandom`, `random`, `gauss_random`, `gaussRandom`, `noise` |
| Interpolation | 6 | `linear`, `ease`, `ease_in`, `ease_out`, `easeIn`, `easeOut` |
| Color Conversion | 5 | `rgb_to_hsl`, `hsl_to_rgb`, `rgbToHsl`, `hslToRgb`, `hexToRgb` |
| Other Math | 4 | `degrees_to_radians`, `radians_to_degrees`, `degreesToRadians`, `radiansToDegrees` |
| Sub-objects | 5 | `source`, `sourceTime`, `sourceRectAtTime`, `effect`, `mask` |
| General | 16 | `parent`, `has_parent`, `in_point`, `out_point`, `start_time`, `has_video`, `has_audio`, `hasParent`, `inPoint`, `outPoint`, `startTime`, `hasVideo`, `hasAudio`, `audio_active`, `audioActive`, `sampleImage` |
| Properties | 9 | `anchor_point`, `anchorPoint`, `scale`, `rotation`, `opacity`, `audio_levels`, `time_remap`, `audioLevels`, `timeRemap` |
| 3D | 19 | `orientation`, `rotationX`, `rotationY`, `rotationZ`, `castsShadows`, `casts_shadows`, `lightTransmission`, `light_transmission`, `acceptsShadows`, `accepts_shadows`, `acceptsLights`, `accepts_lights`, `ambient`, `diffuse`, `specularIntensity`, `specular`, `specularShininess`, `shininess`, `metal` |
| Space Transforms | 30 | `to_comp`, `from_comp`, `to_world`, `from_world`, `to_comp_vec`, `from_comp_vec`, `to_world_vec`, `from_world_vec`, `from_comp_to_surface`, `world_position_to_psd`, `world_rotation_to_psd`, `world_scale_to_psd`, `toComp`, `fromComp`, `toWorld`, `fromWorld`, `toCompVec`, `fromCompVec`, `toWorldVec`, `fromWorldVec`, `fromCompToSurface`, `vec`, `framesPerSecond`, `maxValOrArray`, `valOrArray`, `rgbaArray`, `hslaArray`, `hexString`, `degrees`, `radians` |
| (leading block, no category header on disk) | 206 | `point_of_interest`, `pointOfInterest`, `zoom`, `depthOfField`, `focusDistance`, `depth_of_field`, `focus_distance`, `aperture`, `blur_level`, `blurLevel`, `irisShape`, `irisRotation`, `irisRoundness`, `irisAspectRatio`, `irisDiffractionFringe`, `highlightGain`, `highlightThreshold`, `highlightSaturation`, `intensity`, `color`, `cone_angle`, `coneAngle`, `cone_feather`, `coneFeather`, `shadow_darkness`, `shadowDarkness`, `shadow_diffusion`, `shadowDiffusion`, `param`, `maskPath`, `path`, `maskOpacity`, `maskFeather`, `feather`, `maskExpansion`, `expansion`, `invert`, `points`, `inTangents`, `outTangents`, `isClosed`, `pointOnPath`, `normalOnPath`, `tangentOnPath`, `createPath`, `velocity`, `velocity_at_time`, `velocityAtTime`, `speedAtTime`, `wiggle`, `temporal_wiggle`, `temporalWiggle`, `smooth`, `loop_in`, `loop_out`, `loop_in_duration`, `loop_out_duration`, `loopIn`, `loopOut`, `loopInDuration`, `loopOutDuration`, `value_at_time`, `valueAtTime`, `speed`, `speed_at_time`, `key`, `nearest_key`, `num_keys`, `nearestKey`, `nextKey`, `previousKey`, `numKeys`, `.index`, `markerName`, `valueOf`, `toString`, `toUpperCase`, `charAt`, `charCodeAt`, `fromCharCode`, `indexOf`, `lastIndexOf`, `match`, `replace`, `search`, `slice`, `split`, `substr`, `substring`, `toLowerCase`, `toLocaleLowerCase`, `toLocaleUpperCase`, `localeCompare`, `layer`, `marker`, `num_layers`, `numLayers`, `layerByComment`, `active_camera`, `activeCamera`, `width`, `height`, `duration`, `frame_duration`, `ntscDropFrame`, `displayStartTime`, `shutter_angle`, `shutter_phase`, `bg_color`, `pixel_aspect`, `frameDuration`, `shutterAngle`, `shutterPhase`, `bgColor`, `pixelAspect`, `index`, `comment`, `sourceText`, `sourceData`, `dataValue`, `dataKeyCount`, `dataKeyTimes`, `dataKeyValues`, `dataPath`, `chapter`, `url`, `frameTarget`, `protectedRegion`, `eventCuePoint`, `cuePointName`, `parameters`, `sub`, `div`, `position`, `Date`, `fullPath`, `linearBlending`, `bitsPerChannel`, `.name`, `numEntries`, `numProperties`, `propertyGroup`, `propertyIndex`, `active`, `enabled`, `cos`, `acos`, `tan`, `atan`, `atan2`, `sin`, `sqrt`, `exp`, `pow`, `log`, `abs`, `round`, `ceil`, `floor`, `min`, `max`, `text`, `items`, `textAtTime`, `createStyle`, `getStyleAt`, `setAllCaps`, `setApplyFill`, `setApplyStroke`, `setAutoLeading`, `setBaselineDirection`, `setBaselineOption`, `setBaselineShift`, `setDigitSet`, `setFauxBold`, `setFauxItalic`, `setFillColor`, `setFont`, `setFontSize`, `setHorizontalScaling`, `setKerning`, `setKerningType`, `setLeading`, `setLigature`, `setLineJoin`, `replaceText`, `setSmallCaps`, `setStrokeColor`, `setStrokeWidth`, `setText`, `EvalExpr`, `setTracking`, `setTsume`, `setVerticalScaling`, `setDirection`, `setEveryLineComposer`, `setFirstLineIndent`, `setHangingRoman`, `setJustification`, `setLeadingType`, `setLeftMargin`, `setRightMargin`, `setSpaceAfter`, `setSpaceBefore`, `asin`, `BEEp_EvalExprEngines` |

**Argument signatures recovered (37):**

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| # | Signature | Category |
|---|---|---|
| 1 | `percentage = 0.5, t = time` | -- |
| 2 | `points = [[0,0], [100,0], [100,100], [0,100]], inTangents = [], outTangents = [], isClosed = true` | -- |
| 3 | `freq, amp, octaves = 1, amp_mult = .5, t = time` | -- |
| 4 | `width = .2, samples = 5, t = time` | -- |
| 5 | `type = "cycle", numKeyframes = 0` | -- |
| 6 | `type = "cycle", duration = 0` | -- |
| 7 | `countUp = 1` | -- |
| 8 | `otherLayer, relIndex` | -- |
| 9 | `dataPath, t0 = startTime, t1 = endTime` | -- |
| 10 | `y, x` | -- |
| 11 | `value, exponent` | -- |
| 12 | `value1, value2` | -- |
| 13 | `charIndex, t = time` | -- |
| 14 | `value, s = start index, n = number of characters` | -- |
| 15 | `value, s` | -- |
| 16 | `*` | -- |
| 17 | `+` | -- |
| 18 | `vec1, vec2` | Space Transforms |
| 19 | `vec, amount` | Space Transforms |
| 20 | `value, limit1, limit2` | Space Transforms |
| 21 | `point1, point2` | Space Transforms |
| 22 | `fromPoint, atPoint` | Space Transforms |
| 23 | `seed, timeless = false` | Space Transforms |
| 24 | `minValOrArray, maxValOrArray` | Space Transforms |
| 25 | `t, value1, value2` | Space Transforms |
| 26 | `t, tMin, tMax, value1, value2` | Space Transforms |
| 27 | `point, t = time` | Space Transforms |
| 28 | `vec, t = time` | Space Transforms |
| 29 | `point, radius = [.5, .5], postEffect = true, t = time` | Space Transforms |
| 30 | `t = time + thisComp.displayStartTime, fps = 1.0 / thisComp.frameDuration, isDuration = false` | Space Transforms |
| 31 | `frames, fps = 1.0 / thisComp.frameDuration` | Space Transforms |
| 32 | `t = time + thisComp.displayStartTime, timecodeBase = 30, isDuration = false` | Space Transforms |
| 33 | `t = time + thisComp.displayStartTime, ntscDropFrame = false, isDuration = false` | Space Transforms |
| 34 | `t = time + thisComp.displayStartTime, fps = 1.0 / thisComp.frameDuration, framesPerFoot = 16, isDuration = false` | Space Transforms |
| 35 | `t = time + thisComp.displayStartTime, fps = 1.0 / thisComp.frameDuration, isDuration = false, ntscDropFrame = thisComp.ntscDropFrame` | Space Transforms |
| 36 | `t = time` | Space Transforms |
| 37 | `t = time, includeExtents = false` | Space Transforms |

[STU-MOT-074] **Six identifier families are load-bearing and are called out because an
implementation that ships the language without them ships a calculator.**

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per load-bearing identifier family.*

| Family | Members | Why it is load-bearing |
|---|---|---|
| Space transforms (30) | `toComp`, `fromComp`, `toWorld`, `fromWorld`, `toCompVec`, `fromCompVec`, `toWorldVec`, `fromWorldVec`, `fromCompToSurface` and their legacy spellings | Converting a point between a layer's space, the composition's space and world space is what makes rigging across a parent hierarchy possible. Without them, an expression cannot answer "where is that layer, from here". |
| Property sampling (in the uncategorised block) | `valueAtTime`, `velocityAtTime`, `speedAtTime`, `nearestKey`, `nextKey`, `previousKey`, `numKeys`, `key` | Reading another property at another time is how delay, echo, follow and inertia rigs are built. |
| Looping | `loopIn`, `loopOut`, `loopInDuration`, `loopOutDuration` with a `type = "cycle"` argument | Cycling, ping-ponging, continuing and offsetting an existing keyframe range without duplicating keyframes. |
| Randomness | `seedRandom`, `random`, `gaussRandom`, `noise`, `wiggle`, `temporalWiggle` | Procedural variation. `seedRandom` is what makes it REPRODUCIBLE, and Studio requires the seed to be explicit exactly as [STU-FX-011] requires it of effects. |
| Interpolation (6) | `linear`, `ease`, `easeIn`, `easeOut` and legacy spellings | Remapping one range onto another with an easing curve, in code. |
| 3D (19) and vector math (9) | `orientation`, `rotationX/Y/Z`, `castsShadows`, material coefficients; `add`, `mul`, `clamp`, `dot`, `cross`, `normalize`, `length`, `lookAt` | Reading and computing 3D state, which is what binds this sub-section to 14.27. |

[STU-MOT-075] **37 argument signatures are recovered and are normative for the functions they
describe**, including their DEFAULT ARGUMENT VALUES, which are part of the contract and cannot be
guessed. They are reproduced in full in the table above. Four examples of why they matter:
`wiggle(freq, amp, octaves = 1, amp_mult = .5, t = time)` -- the octave count and amplitude
multiplier defaults define the noise character; `temporalWiggle(width = .2, samples = 5, t = time)`;
`loopOut(type = "cycle", numKeyframes = 0)` where 0 means the whole range; and
`createPath(points = [[0,0], [100,0], [100,100], [0,100]], inTangents = [], outTangents = [],
isClosed = true)` which defines the default path a procedural mask starts from.

### 4. Evaluation

**[STU-MOT-076] Expression evaluation is deterministic and side-effect-free.** Given the same
composition state and the same time, an expression MUST return the same value on every evaluation
and on every backend. It may read the document; it may NOT write to it, may NOT perform I/O, may NOT
observe wall-clock time, and may NOT observe evaluation order. Randomness is available only through
the seeded functions of [STU-MOT-074], and an unseeded `random()` call MUST derive its seed
deterministically from the property's stable path and the current time, so that a re-render matches.

**[STU-MOT-077] A dependency cycle is a validation error, detected and reported, never a hang.**
Expressions form a dependency graph over properties. Studio computes that graph, evaluates in
topological order, and refuses a cycle at the moment the expression is committed, naming the
participating property paths.

[STU-MOT-078] **An expression error disables the expression and states why; it never renders a
wrong frame silently.** On error, the property falls back to its underlying state
([STU-MOT-071]), the property is marked in an explicit error state, and the error carries the
expression source position. A composition containing a disabled-by-error expression MUST report
that fact in its render receipt, so a batch render cannot quietly ship broken output.

**[STU-MOT-079] Expressions are sandboxed.** Expression evaluation runs inside the kernel sandbox
tier with no filesystem, network, process or environment access, a bounded evaluation budget per
frame, and a determinate `EXPRESSION_BUDGET_EXCEEDED` result on overrun. A model-authored expression
traverses sandbox -> validation -> `PromotionGate` like every other model-authored mutation
([STU-ARC-005]); an expression is code, and admitting model-authored code to a document without
that lifecycle would be the single largest hole in Studio's safety posture.

### 5. Expression controls

[STU-MOT-080] **Expression control effects are a normative bridge between the operator surface and
the language.** Eight control kinds exist -- slider, angle, point, 3D point, colour, checkbox,
dropdown menu and layer -- each an effect that renders nothing and exists only to expose a named,
keyframable, scrubbable parameter that expressions on other properties can read. They are how a rig
gets an operator-facing control panel without a plugin. They are ordinary `StudioLiveFilter` kinds
in the `expression_controls` category ([STU-FX-126]) and carry the full parameter contract of
14.9.1.

### 6. The authoring surface

**[STU-MOT-081] The expression editor is a first-class panel with a stated feature set:** an
editable source view with syntax highlighting and error marking; a language reference browsable by
the categories of [STU-MOT-073]; a property pick-whip that inserts the stable path of a
click-targeted property ([STU-MOT-011]); a snippet library organised by category, with the shipped
categories being 3D, Behaviors, Looping, Physics, Utility and Wiggles; save-a-snippet with a
new-category flow; apply-to-all across a selection; find-all-expressions and
find-matching-expressions across a composition; select-matching-expressions; replace-a-property-
with-an-expression-control; and a multi-selection warning when the selected properties do not share
one expression. Every one of these is reachable as a typed command.

**[STU-MOT-082] The pick-whip inserts a stable path, never a display name.** This is the single
most common source of broken expressions in the field, and [STU-MOT-011] exists to prevent it.

**[STU-MOT-083] DECLARED DECISION -- the expression language itself.** This sub-section specifies
the identifier surface, the argument signatures, the evaluation semantics, the sandbox, the error
model and the property-state machine. It does NOT specify which language syntax Studio implements.
The recovered surface is JavaScript-shaped (it includes `Date`, `Math`-family members, and String
methods such as `substring`, `toUpperCase`, `replace`, `split` and `localeCompare`), which sets the
compatibility target for IMPORT. Whether Studio's native expression language is a JavaScript subset,
a Rust-embedded scripting language, or a purpose-built deterministic expression language is an
architecture decision with real consequences for [STU-MOT-076] determinism and [STU-MOT-079]
sandboxing, and it is recorded as open decision [STU-MOT-142] rather than assumed.

---

## 14.26.7 Text animators

[STU-MOT-090] **A text layer's animator system is a property subtree, and it is the mechanism by
which text animates per character, word or line without one keyframe per glyph.** Its structure is
normative:

1. A text layer carries a SOURCE TEXT property (itself keyframable, so the words can change over
   time), a PATH OPTIONS group, a MORE OPTIONS group, and an ANIMATORS collection.
2. An ANIMATOR is a named group holding (a) one or more animated PROPERTIES that state what changes,
   and (b) one or more SELECTORS that state which characters it changes and by how much.
3. A selector produces a per-character amount in 0..1. The animator's property values are applied to
   each character scaled by that amount. Animating the SELECTOR, not the property, is what produces
   a wave of change travelling through the text -- which is why this is not expressible as
   ordinary per-property keyframes.

**[STU-MOT-091] Three selector kinds are normative:** a RANGE selector (start, end, offset, units
in percent or index, based-on characters / characters-excluding-spaces / words / lines, a shape
function, smoothness, ease-high, ease-low, and a randomize-order option with its own seed); a WIGGLE
selector (wiggles the amount over time with frequency, amplitude, correlation, temporal and spatial
phase, and a lock-dimensions option); and an EXPRESSION selector (the amount is computed by an
expression per character, with `textIndex`, `textTotal` and `selectorValue` in scope). All three
produce the same 0..1 amount and compose multiplicatively when several are present.

**[STU-MOT-092] 99 animator properties are recovered and are the capability target.** They cover
transform (anchor point, position, scale, skew, rotation, opacity), character appearance (fill
colour in RGB/HSL, stroke colour, stroke width, fill and stroke opacity), typographic controls
(tracking, line spacing, line anchor, character offset, character value, character alignment,
blur), and a large 3D-text material surface (front, back and bevel ambient, bright, diffuse,
fresnel, gloss, hue, index of refraction and metal coefficients). Every one is an ordinary
`StudioProperty` and every one is keyframable.

**[STU-MOT-093] Path options and more options are normative groups.** Path options bind the text to
a `StudioVectorPath` with first margin, last margin, reverse path, perpendicular-to-path and
force-alignment controls. More options carry the anchor-point grouping mode and its alignment, the
per-character blend mode, the render order (fill-over-stroke or stroke-over-fill, per character or
per glyph), and variable-font spacing.

**[STU-MOT-094]** Text shaping remains bound by [STU-TYP-008]: the shaping stack is native Rust of the
`cosmic-text`/`rustybuzz`/`swash` class and a platform text engine is forbidden, because per-glyph
animation demands byte-identical shaping across hosts or the same composition animates differently
on two machines.

**[STU-MOT-095] Recovered group topology appendix, and which clause each part of it serves.** The
tables and lists that follow -- shape operator families with their child properties, text property
groups, the 99 text animator properties, the three text selector kinds, and mask property groups --
are the recovered containment topology reproduced as the completeness check on the clauses that own
each area. They carry no contract of their own and they never override a clause: the shape rows
serve [STU-MOT-101], the text rows serve [STU-MOT-090] through [STU-MOT-093], and the mask rows
serve [STU-CMP-025], which owns masks as compositing geometry. Every identifier here is an import
key ([STU-FX-103]) and never a Studio-facing name ([STU-SECTION-003]). Where a row reads
`_children not recovered_` the containment is UNKNOWN and MUST NOT be inferred from a sibling.
This clause exists so that no part of the appendix is normative-looking but unreachable from the
derivation index of [STU-MOT-252]; the microtask it yields is the reconciliation that proves every
row here resolves to an owning clause.

*Derivation: catalogue table, splits per row; yields 21 microtasks, one per shape operator family.*

| Shape operator family | Semantic | Child properties |
|---|---|---|
| `ADBE Vector Shape - Rect` | Rectangle path | `ADBE Vector Rect Position`, `ADBE Vector Rect Roundness`, `ADBE Vector Rect Size`, `ADBE Vector Shape Direction` |
| `ADBE Vector Shape - Ellipse` | Ellipse path | `ADBE Vector Ellipse Position`, `ADBE Vector Ellipse Size`, `ADBE Vector Shape Direction` |
| `ADBE Vector Shape - Star` | Polystar path | `ADBE Vector Shape Direction`, `ADBE Vector Star Inner Radius`, `ADBE Vector Star Inner Roundess`, `ADBE Vector Star Outer Radius`, `ADBE Vector Star Outer Roundess`, `ADBE Vector Star Points`, `ADBE Vector Star Position`, `ADBE Vector Star Rotation`, `ADBE Vector Star Type` |
| `ADBE Vector Shape - Group` | Bezier path (Path) | `ADBE Vector Shape`, `ADBE Vector Shape Direction` |
| `ADBE Vector Graphic - Fill` | Fill | `ADBE Vector Blend Mode`, `ADBE Vector Composite Order`, `ADBE Vector Fill Color`, `ADBE Vector Fill Opacity`, `ADBE Vector Fill Rule` |
| `ADBE Vector Graphic - Stroke` | Stroke | `ADBE Vector Blend Mode`, `ADBE Vector Composite Order`, `ADBE Vector Stroke Color`, `ADBE Vector Stroke Dashes`, `ADBE Vector Stroke Line Cap`, `ADBE Vector Stroke Line Join`, `ADBE Vector Stroke Miter Limit`, `ADBE Vector Stroke Opacity`, `ADBE Vector Stroke Taper`, `ADBE Vector Stroke Wave`, `ADBE Vector Stroke Width` |
| `ADBE Vector Graphic - G-Fill` | Gradient Fill | _children not recovered_ |
| `ADBE Vector Graphic - G-Stroke` | Gradient Stroke | _children not recovered_ |
| `ADBE Vector Filter - Merge` | Merge Paths | `ADBE Vector Merge Type` |
| `ADBE Vector Filter - Offset` | Offset Paths | `ADBE Vector Offset Amount`, `ADBE Vector Offset Copies`, `ADBE Vector Offset Copy Offset`, `ADBE Vector Offset Line Join`, `ADBE Vector Offset Miter Limit` |
| `ADBE Vector Filter - PB` | Pucker & Bloat | `ADBE Vector PuckerBloat Amount` |
| `ADBE Vector Filter - Repeater` | Repeater | `ADBE Vector Repeater Copies`, `ADBE Vector Repeater Offset`, `ADBE Vector Repeater Order`, `ADBE Vector Repeater Transform` |
| `ADBE Vector Filter - RC` | Round Corners | `ADBE Vector RoundCorner Radius` |
| `ADBE Vector Filter - Trim` | Trim Paths | `ADBE Vector Trim End`, `ADBE Vector Trim Offset`, `ADBE Vector Trim Start`, `ADBE Vector Trim Type` |
| `ADBE Vector Filter - Twist` | Twist | `ADBE Vector Twist Angle`, `ADBE Vector Twist Center` |
| `ADBE Vector Filter - Roughen` | Wiggle Paths (Roughen Edges) | `ADBE Vector Roughen Detail`, `ADBE Vector Roughen Points`, `ADBE Vector Roughen Size`, `ADBE Vector Temporal Freq` |
| `ADBE Vector Filter - Wiggler` | Wiggle Transform | `ADBE Vector Correlation`, `ADBE Vector Random Seed`, `ADBE Vector Spatial Phase`, `ADBE Vector Temporal Phase`, `ADBE Vector Wiggler Transform`, `ADBE Vector Xform Temporal Freq` |
| `ADBE Vector Filter - Zigzag` | Zig Zag | `ADBE Vector Zigzag Detail`, `ADBE Vector Zigzag Points`, `ADBE Vector Zigzag Size` |
| `ADBE Vector Filter - Reveal` | Reveal (Trim) helper | _children not recovered_ |
| `ADBE Vector Transform Group` | Shape group Transform | `ADBE Vector Anchor`, `ADBE Vector Group Opacity`, `ADBE Vector Position`, `ADBE Vector Rotation`, `ADBE Vector Scale`, `ADBE Vector Skew`, `ADBE Vector Skew Axis` |
| `ADBE Vector Materials Group` | Shape group material options | `ADBE Vec3D Back Ambient`, `ADBE Vec3D Back Diffuse`, `ADBE Vec3D Back Fresnel`, `ADBE Vec3D Back Gloss`, `ADBE Vec3D Back IOR`, `ADBE Vec3D Back Metal`, `ADBE Vec3D Back RGB`, `ADBE Vec3D Back Reflection`, `ADBE Vec3D Back Shininess`, `ADBE Vec3D Back Specular`, `ADBE Vec3D Back XparRoll`, `ADBE Vec3D Back Xparency`, `ADBE Vec3D Bevel Ambient`, `ADBE Vec3D Bevel Diffuse`, `ADBE Vec3D Bevel Fresnel`, `ADBE Vec3D Bevel Gloss`, `ADBE Vec3D Bevel IOR`, `ADBE Vec3D Bevel Metal`, `ADBE Vec3D Bevel RGB`, `ADBE Vec3D Bevel Reflection`, `ADBE Vec3D Bevel Shininess`, `ADBE Vec3D Bevel Specular`, `ADBE Vec3D Bevel XparRoll`, `ADBE Vec3D Bevel Xparency`, `ADBE Vec3D Front Ambient`, `ADBE Vec3D Front Diffuse`, `ADBE Vec3D Front Fresnel`, `ADBE Vec3D Front Gloss`, `ADBE Vec3D Front IOR`, `ADBE Vec3D Front Metal`, `ADBE Vec3D Front RGB`, `ADBE Vec3D Front Reflection`, `ADBE Vec3D Front Shininess`, `ADBE Vec3D Front Specular`, `ADBE Vec3D Front XparRoll`, `ADBE Vec3D Front Xparency`, `ADBE Vec3D Side Ambient`, `ADBE Vec3D Side Diffuse`, `ADBE Vec3D Side Fresnel`, `ADBE Vec3D Side Gloss`, `ADBE Vec3D Side IOR`, `ADBE Vec3D Side Metal`, `ADBE Vec3D Side RGB`, `ADBE Vec3D Side Reflection`, `ADBE Vec3D Side Shininess`, `ADBE Vec3D Side Specular`, `ADBE Vec3D Side XparRoll`, `ADBE Vec3D Side Xparency` |

**Text property groups**

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Group | Semantic | Children |
|---|---|---|
| `ADBE Text Properties` | Text property root | `ADBE Text Animators`, `ADBE Text Document`, `ADBE Text More Options`, `ADBE Text Path Options` |
| `ADBE Text Document` | Source Text document | -- |
| `ADBE Text Path Options` | Path Options | `ADBE Text First Margin`, `ADBE Text Force Align Path`, `ADBE Text Last Margin`, `ADBE Text Path`, `ADBE Text Perpendicular To Path`, `ADBE Text Reverse Path` |
| `ADBE Text More Options` | More Options | `ADBE Text Anchor Point Align`, `ADBE Text Anchor Point Option`, `ADBE Text Character Blend Mode`, `ADBE Text Render Order`, `ADBE Text Variable Font Spacing` |
| `ADBE Text Animators` | Animators collection | `ADBE Text Animator` |
| `ADBE Text Animator` | One text animator | `ADBE Text Animator Properties`, `ADBE Text Selectors` |
| `ADBE Text Animator Properties` | Animatable text properties | `ADBE 3DText Back Ambient`, `ADBE 3DText Back Bright`, `ADBE 3DText Back Diffuse`, `ADBE 3DText Back Fresnel`, `ADBE 3DText Back Gloss`, `ADBE 3DText Back Hue`, `ADBE 3DText Back IOR`, `ADBE 3DText Back Metal`, `ADBE 3DText Back Opacity`, `ADBE 3DText Back RGB`, `ADBE 3DText Back Reflection`, `ADBE 3DText Back Sat`, `ADBE 3DText Back Shininess`, `ADBE 3DText Back Specular`, `ADBE 3DText Back XparRoll`, `ADBE 3DText Back Xparency`, `ADBE 3DText Bevel Ambient`, `ADBE 3DText Bevel Bright`, `ADBE 3DText Bevel Depth`, `ADBE 3DText Bevel Diffuse`, `ADBE 3DText Bevel Fresnel`, `ADBE 3DText Bevel Gloss`, `ADBE 3DText Bevel Hue`, `ADBE 3DText Bevel IOR`, `ADBE 3DText Bevel Metal`, `ADBE 3DText Bevel Opacity`, `ADBE 3DText Bevel RGB`, `ADBE 3DText Bevel Reflection`, `ADBE 3DText Bevel Sat`, `ADBE 3DText Bevel Shininess`, `ADBE 3DText Bevel Specular`, `ADBE 3DText Bevel XparRoll`, `ADBE 3DText Bevel Xparency`, `ADBE 3DText Extrude Depth`, `ADBE 3DText Front Ambient`, `ADBE 3DText Front Bright`, `ADBE 3DText Front Diffuse`, `ADBE 3DText Front Fresnel`, `ADBE 3DText Front Gloss`, `ADBE 3DText Front Hue`, `ADBE 3DText Front IOR`, `ADBE 3DText Front Metal`, `ADBE 3DText Front Opacity`, `ADBE 3DText Front RGB`, `ADBE 3DText Front Reflection`, `ADBE 3DText Front Sat`, `ADBE 3DText Front Shininess`, `ADBE 3DText Front Specular`, `ADBE 3DText Front XparRoll`, `ADBE 3DText Front Xparency`, `ADBE 3DText Side Ambient`, `ADBE 3DText Side Bright`, `ADBE 3DText Side Diffuse`, `ADBE 3DText Side Fresnel`, `ADBE 3DText Side Gloss`, `ADBE 3DText Side Hue`, `ADBE 3DText Side IOR`, `ADBE 3DText Side Metal`, `ADBE 3DText Side Opacity`, `ADBE 3DText Side RGB`, `ADBE 3DText Side Reflection`, `ADBE 3DText Side Sat`, `ADBE 3DText Side Shininess`, `ADBE 3DText Side Specular`, `ADBE 3DText Side XparRoll`, `ADBE 3DText Side Xparency`, `ADBE Text Anchor Point`, `ADBE Text Anchor Point 3D`, `ADBE Text Blur`, `ADBE Text Character Change Type`, `ADBE Text Character Offset`, `ADBE Text Character Range`, `ADBE Text Character Replace`, `ADBE Text Fill Brightness`, `ADBE Text Fill Color`, `ADBE Text Fill Hue`, `ADBE Text Fill Opacity`, `ADBE Text Fill Saturation`, `ADBE Text Line Anchor`, `ADBE Text Line Spacing`, `ADBE Text Opacity`, `ADBE Text Position`, `ADBE Text Position 3D`, `ADBE Text Rotation`, `ADBE Text Rotation X`, `ADBE Text Rotation Y`, `ADBE Text Scale`, `ADBE Text Scale 3D`, `ADBE Text Skew`, `ADBE Text Skew Axis`, `ADBE Text Stroke Brightness`, `ADBE Text Stroke Color`, `ADBE Text Stroke Hue`, `ADBE Text Stroke Opacity`, `ADBE Text Stroke Saturation`, `ADBE Text Stroke Width`, `ADBE Text Track Type`, `ADBE Text Tracking Amount`, `ADBE Text VF Axis 1` |
| `ADBE Text Selectors` | Selectors collection | `ADBE Text Expressible Selector`, `ADBE Text Selector`, `ADBE Text Wiggly Selector` |
| `ADBE Text Selector` | Range selector | `ADBE Text Index End`, `ADBE Text Index Offset`, `ADBE Text Index Start`, `ADBE Text Percent End`, `ADBE Text Percent Offset`, `ADBE Text Percent Start`, `ADBE Text Range Advanced` |
| `ADBE Text Wiggly Selector` | Wiggly selector | `ADBE Text Character Correlation`, `ADBE Text Range Type2`, `ADBE Text Selector Mode`, `ADBE Text Spatial Phase`, `ADBE Text Temporal Freq`, `ADBE Text Temporal Phase`, `ADBE Text Wiggly Lock Dim`, `ADBE Text Wiggly Max Amount`, `ADBE Text Wiggly Min Amount`, `ADBE Text Wiggly Random Seed` |
| `ADBE Text Expressible Selector` | Expression selector | `ADBE Text Expressible Amount`, `ADBE Text Range Type2` |
| `ADBE Text Range Advanced` | Range selector advanced options | `ADBE Text Levels Max Ease`, `ADBE Text Levels Min Ease`, `ADBE Text Random Seed`, `ADBE Text Randomize Order`, `ADBE Text Range Shape`, `ADBE Text Range Type2`, `ADBE Text Range Units`, `ADBE Text Selector Max Amount`, `ADBE Text Selector Mode`, `ADBE Text Selector Smoothness` |
| `ADBE Text Per Char 3D` | Enable Per-character 3D | -- |

**Text animator properties (99):** `ADBE 3DText Back Ambient`, `ADBE 3DText Back Bright`, `ADBE 3DText Back Diffuse`, `ADBE 3DText Back Fresnel`, `ADBE 3DText Back Gloss`, `ADBE 3DText Back Hue`, `ADBE 3DText Back IOR`, `ADBE 3DText Back Metal`, `ADBE 3DText Back Opacity`, `ADBE 3DText Back RGB`, `ADBE 3DText Back Reflection`, `ADBE 3DText Back Sat`, `ADBE 3DText Back Shininess`, `ADBE 3DText Back Specular`, `ADBE 3DText Back XparRoll`, `ADBE 3DText Back Xparency`, `ADBE 3DText Bevel Ambient`, `ADBE 3DText Bevel Bright`, `ADBE 3DText Bevel Depth`, `ADBE 3DText Bevel Diffuse`, `ADBE 3DText Bevel Fresnel`, `ADBE 3DText Bevel Gloss`, `ADBE 3DText Bevel Hue`, `ADBE 3DText Bevel IOR`, `ADBE 3DText Bevel Metal`, `ADBE 3DText Bevel Opacity`, `ADBE 3DText Bevel RGB`, `ADBE 3DText Bevel Reflection`, `ADBE 3DText Bevel Sat`, `ADBE 3DText Bevel Shininess`, `ADBE 3DText Bevel Specular`, `ADBE 3DText Bevel XparRoll`, `ADBE 3DText Bevel Xparency`, `ADBE 3DText Extrude Depth`, `ADBE 3DText Front Ambient`, `ADBE 3DText Front Bright`, `ADBE 3DText Front Diffuse`, `ADBE 3DText Front Fresnel`, `ADBE 3DText Front Gloss`, `ADBE 3DText Front Hue`, `ADBE 3DText Front IOR`, `ADBE 3DText Front Metal`, `ADBE 3DText Front Opacity`, `ADBE 3DText Front RGB`, `ADBE 3DText Front Reflection`, `ADBE 3DText Front Sat`, `ADBE 3DText Front Shininess`, `ADBE 3DText Front Specular`, `ADBE 3DText Front XparRoll`, `ADBE 3DText Front Xparency`, `ADBE 3DText Side Ambient`, `ADBE 3DText Side Bright`, `ADBE 3DText Side Diffuse`, `ADBE 3DText Side Fresnel`, `ADBE 3DText Side Gloss`, `ADBE 3DText Side Hue`, `ADBE 3DText Side IOR`, `ADBE 3DText Side Metal`, `ADBE 3DText Side Opacity`, `ADBE 3DText Side RGB`, `ADBE 3DText Side Reflection`, `ADBE 3DText Side Sat`, `ADBE 3DText Side Shininess`, `ADBE 3DText Side Specular`, `ADBE 3DText Side XparRoll`, `ADBE 3DText Side Xparency`, `ADBE Text Anchor Point`, `ADBE Text Anchor Point 3D`, `ADBE Text Blur`, `ADBE Text Character Change Type`, `ADBE Text Character Offset`, `ADBE Text Character Range`, `ADBE Text Character Replace`, `ADBE Text Fill Brightness`, `ADBE Text Fill Color`, `ADBE Text Fill Hue`, `ADBE Text Fill Opacity`, `ADBE Text Fill Saturation`, `ADBE Text Line Anchor`, `ADBE Text Line Spacing`, `ADBE Text Opacity`, `ADBE Text Position`, `ADBE Text Position 3D`, `ADBE Text Rotation`, `ADBE Text Rotation X`, `ADBE Text Rotation Y`, `ADBE Text Scale`, `ADBE Text Scale 3D`, `ADBE Text Skew`, `ADBE Text Skew Axis`, `ADBE Text Stroke Brightness`, `ADBE Text Stroke Color`, `ADBE Text Stroke Hue`, `ADBE Text Stroke Opacity`, `ADBE Text Stroke Saturation`, `ADBE Text Stroke Width`, `ADBE Text Track Type`, `ADBE Text Tracking Amount`, `ADBE Text VF Axis 1`

**Text selector kinds (3):** `ADBE Text Expressible Selector`, `ADBE Text Selector`, `ADBE Text Wiggly Selector`

**Mask property groups**

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Group | Semantic | Children |
|---|---|---|
| `ADBE Mask Parade` | Masks collection | `ADBE Mask Atom` |
| `ADBE Mask Atom` | One mask | `ADBE Mask Feather`, `ADBE Mask Offset`, `ADBE Mask Opacity`, `ADBE Mask Shape` |
| `ADBE Mask Shape` | Mask Path | -- |
| `ADBE Mask Opacity` | Mask Opacity | -- |
| `ADBE Mask Offset` | Mask Expansion | -- |
| `ADBE Mask Feather` | Mask Feather (uniform) | -- |
| `ADBE Mask Interp` | Variable-width mask feather points | -- |
---

## 14.26.8 Shape operators

[STU-MOT-100] **A shape layer is a PROCEDURAL vector graph, not a static path, and its operators
are ordered property groups.** This is what distinguishes motion-graphics vector work from
illustration vector work (14.5): the geometry is computed from parameters every frame, so every
parameter is keyframable and expression-drivable.

**[STU-MOT-101] 21 shape operator families are normative.** They divide into three roles:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Role | Operators |
|---|---|
| Path generators | Rectangle path (position, size, roundness, direction), Ellipse path (position, size, direction), Polystar path (type, points, position, rotation, inner and outer radius, inner and outer roundness, direction), Bezier path (an editable path plus direction). |
| Paint operators | Fill (colour, opacity, fill rule, blend mode, composite order), Stroke (colour, opacity, width, line cap, line join, miter limit, dashes, taper, wave, blend mode, composite order), Gradient Fill, Gradient Stroke. |
| Path modifiers | Merge Paths, Offset Paths, Pucker & Bloat, Repeater, Round Corners, Trim Paths, Twist, Wiggle Paths, Wiggle Transform, Zig Zag, and a Trim reveal helper. |

Two further groups complete the model: a per-group TRANSFORM and a per-group MATERIALS group for 3D
shape extrusion.

**[STU-MOT-102] Operator ORDER within a group is semantic and is part of the document.** A trim
before a stroke trims the geometry that gets stroked; a trim after it trims the stroke. Reordering
is a typed command and a reversible history step, and an implementation that renders operators in a
fixed order rather than the authored order is wrong.

[STU-MOT-103] **Three modifiers are called out because they are the ones motion graphics is built
on.** TRIM PATHS animates start, end and offset along the path length, which is how a line draws
itself on -- the single most common motion-graphics gesture, and impossible without a procedural
path. REPEATER duplicates a group N times with a per-copy transform and per-copy opacity ramp, which
is how radial and linear arrays are built from one shape. WIGGLE TRANSFORM applies seeded temporal
noise to a group's transform, which is procedural liveliness without keyframes; its seed obeys [STU-FX-011].

**[STU-MOT-104]** The shape geometry engine is `studio-engine`'s vector engine (14.5), reached through
the `VectorEngine` trait. Shape operators do not get a private geometry implementation
([STU-FX-016a]'s principle applied to shapes).

---

## 14.26.9 Motion presets and the scripting surface

**[STU-MOT-110] A motion preset is a `StudioStyleRegistry` entry**, and per [STU-FX-142] the shipped
corpus resolves into effect presets and PROPERTY presets. A property preset writes values, keyframes
and expressions into named property paths on the target -- it is a motion preset, and it is why [STU-FX-142a]'s
value-stream, keyframe and expression counts matter: 79,014 value streams, 3,217
keyframes and 42 expressions across the shipped set. A preset format that carries only static values
cannot represent this material.

**[STU-MOT-111] Preset application is path-based and MUST fail loudly on a mismatch.** Applying a
preset that targets a text animator to a solid layer produces a determinate
`PRESET_TARGET_INCOMPATIBLE` result naming the unmatched paths, and applies nothing, rather than
partially applying and leaving a half-rigged layer.

**[STU-MOT-112] The automation/scripting surface for motion is enumerated, and 14.14 owns it.** 44
scripting classes with 1,708 members are recovered as the object-model shape a Studio automation
API must cover: project, item collections, compositions, layers, layer collections, property
groups, properties, keyframes, render queue items, output modules, viewers, footage sources and the
text document. This sub-section requires only that the automation surface be a projection of the
same primitives specified here -- one property model, not a parallel scripting model
([STU-DOC-004]).

*Derivation: catalogue table, splits per row; yields 44 microtasks, one per scripting class.*

| Class | Members |
|---|---|
| `AVLayer` | 118 |
| `CameraLayer` | 9 |
| `CharacterRange` | 0 |
| `CompItem` | 67 |
| `ComposedLineRange` | 7 |
| `DeferredCall` | 490 |
| `File` | 61 |
| `FileSource` | 114 |
| `Folder` | 13 |
| `FolderItem` | 1 |
| `Font` | 59 |
| `FootageItem` | 5 |
| `GuideOptions` | 3 |
| `ImportOptions` | 9 |
| `ItemCollection` | 5 |
| `KeyframeEase` | 3 |
| `Layer` | 2 |
| `LayerCollection` | 21 |
| `LightLayer` | 2 |
| `MarkerValue` | 10 |
| `MaskPropertyGroup` | 6 |
| `MaterialPropertyGroup` | 188 |
| `OMCollection` | 1 |
| `OutputModule` | 14 |
| `ParagraphRange` | 0 |
| `ParametricMeshLayer` | 20 |
| `PlaceholderSource` | 0 |
| `PreviewOptions` | 8 |
| `Project` | 56 |
| `Property` | 90 |
| `PropertyGroup` | 93 |
| `RQItemCollection` | 1 |
| `RenderQueue` | 1 |
| `RenderQueueItem` | 27 |
| `Shape` | 2 |
| `ShapeLayer` | 6 |
| `SolidSource` | 1 |
| `TextDocument` | 71 |
| `TextDocumentRange` | 11 |
| `TextLayer` | 1 |
| `ThreeDModelLayer` | 17 |
| `View` | 3 |
| `ViewOptions` | 9 |
| `Viewer` | 83 |
---

## 14.26.10 Render and output for compositions

[STU-MOT-120] **A composition renders through a RENDER QUEUE, and a queue item is a triple:
(composition, render settings, one or more output modules).** Separating render settings from output
modules is normative and is not a UI convenience: it is what lets one render pass write several
outputs -- a mezzanine master, a review proxy and a still sequence -- from one evaluation, which is
the difference between an hour and three.

**[STU-MOT-121] The normative render settings record.** 104 controls recovered; the 18 that carry a
value are reproduced with their complete option lists, because a render setting whose members are
unknown cannot be implemented.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Setting | Members |
|---|---|
| `quality` | `current_settings`, `best`, `draft`, `wireframe` |
| `resolution` | `current_settings`, `full`, `half`, `third`, `quarter`, `custom` (custom takes an explicit divisor pair) |
| `disk_cache` | `current_settings`, `read_only` |
| `proxy_use` | `current_settings`, `use_all_proxies`, `use_comp_proxies_only`, `use_no_proxies` |
| `effects` | `current_settings`, `all_on`, `all_off` |
| `solo_switches` | `current_settings`, `all_off` |
| `guide_layers` | `current_settings`, `all_off` |
| `color_depth` | `current_settings`, `8_bits_per_channel`, `16_bits_per_channel`, `32_bits_per_channel` |
| `frame_blending` | `current_settings`, `on_for_checked_layers`, `off_for_all_layers` |
| `field_render` | `off`, `upper_field_first`, `lower_field_first` |
| `pulldown_3_2` | `off`, `WSSWW`, `SSWWW`, `SWWWS`, `WWWSS`, `WWSSW` (the five phase choices; each letter is a whole or split frame) |
| `motion_blur` | `current_settings`, `on_for_checked_layers`, `off_for_all_layers` |
| `time_span` | `length_of_comp`, `work_area_only`, `custom` |
| `frame_rate` | `use_comps_frame_rate` or an explicit rate, `hard_min` 0.1, `hard_max` 999 |

**[STU-MOT-121a]** `current_settings` is a real member on nine of these, meaning "take the
composition's or layer's own value". It is NOT a null; a render setting of `current_settings` and a
render setting explicitly matching the composition are different records, because the first tracks
later composition changes and the second does not.

**[STU-MOT-122] The normative output module record.** 82 controls recovered. The required fields
are: `format`; `include_project_link` (bool); `post_render_action`; `include_source_metadata`
(bool); `channels` (RGB, alpha, RGB+alpha); `depth`; `color` (premultiplied or straight);
`starting_number` (`hard_min` 0, `hard_max` 9999999) with a `use_comp_frame_number` bool for
sequences; format-specific options; a RESIZE group (`width` and `height`, each `hard_min` 1,
`hard_max` 30000, with a lock-aspect bool and a resize-quality selection); a CROP group
(`use_region_of_interest` bool plus top, left, bottom and right, each `hard_min` -30000,
`hard_max` 30000, negative values expanding the frame); an AUDIO group (sample rate, bit depth,
channel count, format options, and an explicit three-state audio output selection -- auto, forced
on with a silent track when the composition has no audio, off); and a COLOUR group (`preserve_rgb`
which disables colour conversion entirely for this output, `output_color_space`, a show-all-spaces
toggle, a convert-to-linear-light selection, a Cineon settings sub-record, and `embed_color_space`).

**[STU-MOT-122a]** A colour-management mismatch between the composition's working space and the output
module's space MUST surface as an explicit warning on the queue item before the render starts, not
after. Discovering a colour mismatch after a long render is a category of loss the specification
should prevent.

**[STU-MOT-123] Audio level display is a stated unit choice.** The audio options surface declares
two display units, decibels and percentage, and a minimum-slider selection with eight declared
steps: -12, -24, -36, -48, -60, -72, -84 and -96 dB. Studio stores linear gain and displays in the
selected unit ([STU-DOC-003]).

**[STU-MOT-124] Exporters, importers and codec parameters for compositions.** 29 exporter modules
and 35 importer modules are recovered, writing 19 distinct output extensions: aac, aif, avi, bmp,
dpx, gif, jpg, m4v, mp3, mp4, mpeg, mpg, mxf, pcm, png, tga, tif, wav, wmv. 453 codec parameter
strings are recovered. This surface is subject to [STU-VID-051] in full: adapter-boundary, typed
capability descriptor, runtime enumeration, determinate failure. It is NOT a second export system;
a composition render queue item and a sequence export recipe ([STU-VID-060]) are two entry points to
the export pipeline that 14.13 owns.

**[STU-MOT-125] Render queue items are model-steerable and headless.** Submit, monitor, cancel,
reorder, duplicate, apply a render-settings template, apply an output-module template, and set a
post-render action -- all typed commands. A render never opens a foreground window
([STU-FX-038]). Stop-render options are explicit, and a stopped render's partial output is named in
the receipt rather than left ambiguous.

[STU-MOT-126] **A `collect files` operation gathers a composition and every artifact it references
into one relocatable bundle.** This is the portability contract of [STU-FX-144] at the composition
level: after collection the bundle MUST open on another machine with no absolute path, no
machine-local root and no missing reference, or the collection reports exactly what could not be
gathered.

---

## 14.26.11 The command surface

**[STU-MOT-130] The tables below are the normative Studio motion command vocabulary**, on the same
terms as [STU-VID-041]: every row is a typed, model-invokable, parallel-safe, deterministic command
([STU-CON-007]); the recovered identifiers and bindings are import keys and a completeness check;
Studio's own ids are Handshake-native and namespaced `STUDIO_COMPOSITION_*`, `STUDIO_PROPERTY_*`,
`STUDIO_KEYFRAME_*` and `STUDIO_EXPRESSION_*` per [STU-ARC-003]. 665 commands with labels and 673
key bindings across 32 declared binding contexts are recovered.

**[STU-MOT-130a] Three command labels named a vendor product and have been renamed to name the
capability instead.** Same rule and same discipline as [STU-VID-041a]: [STU-SECTION-003] forbids a
source product name as a Studio command or manual name. Behaviour, command identity and key binding
are unchanged; the captured label is preserved as provenance:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Studio label | Captured source label (provenance) | Binding context |
|---|---|---|
| Add Composition to External Encoder Queue | `Add Comp to Adobe Media Encoder Queue` | CSwitchboard |
| Open Learning Resources | `Learn After Effects` | -- |
| Browse in Asset Browser | `Browse in Bridge` | CEggApp |

None of the three creates or removes a capability. The encoder-queue handoff submits a composition
to an EXTERNAL encoder through the adapter boundary of [STU-VID-051] and is not a second render
path; Studio's own path is the render queue of [STU-MOT-120]. `Open Learning Resources` opens the
dual-audience UserManual entry set (14.22) and MUST NOT open a network destination
([STU-OVR-002]). `Browse in Asset Browser` opens the bound asset library and is the same surface
14.29 specifies, not a separate application.

[STU-MOT-131] **The tool set for the composition viewer is normative and is 22 tools across 12
shortcut groups**, several sharing a key and cycling: selection; rotation; anchor-point (pan
behind); three camera tools (orbit, track XY, track Z) sharing one key; four pen tools (pen, add
vertex, delete vertex, convert vertex) sharing one key; four paint tools (brush, pencil, clone
stamp, eraser); four shape tools (rectangle, ellipse, freeform pen, line) with rectangle and ellipse
sharing a key; two type tools (horizontal, vertical) sharing a key; hand; and zoom. Key sharing with
cycling is a normative interaction, not an accident: pressing a shared key repeatedly advances
through that group.


**CPanoProjLayerPano** (16 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Move Layer 1 Screen Pixel Down | `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow)(Alt+DownArrow', 'modifiers': [], 'key': 'DownArrow)(Alt+DownArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 1 Screen Pixel Up | `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow)(Alt+UpArrow', 'modifiers': [], 'key': 'UpArrow)(Alt+UpArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 1 Screen Pixel to the Left | `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 1 Screen Pixel to the Right | `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels Down | `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow)(Alt+Shift+DownArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'DownArrow)(Alt+DownArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels Up | `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow)(Alt+Shift+UpArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'UpArrow)(Alt+UpArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels to the Left | `{'raw': 'Shift+LeftArrow', 'modifiers': ['Shift'], 'key': 'LeftArrow'}`; `{'raw': 'Shift+LeftArrow', 'modifiers': ['Shift'], 'key': 'LeftArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels to the Right | `{'raw': 'Shift+RightArrow', 'modifiers': ['Shift'], 'key': 'RightArrow'}`; `{'raw': 'Shift+RightArrow', 'modifiers': ['Shift'], 'key': 'RightArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Rotate Layer 1 Degree Clockwise | `{'raw': 'PadPlus', 'modifiers': [], 'key': 'PadPlus', 'numeric_keypad': True}`; `{'raw': 'PadPlus', 'modifiers': [], 'key': 'PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Rotate Layer 1 Degree Counterclockwise | `{'raw': 'PadMinus', 'modifiers': [], 'key': 'PadMinus', 'numeric_keypad': True}`; `{'raw': 'PadMinus', 'modifiers': [], 'key': 'PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Rotate Layer 10 Degrees Clockwise | `{'raw': 'Shift+PadPlus', 'modifiers': ['Shift'], 'key': 'PadPlus', 'numeric_keypad': True}`; `{'raw': 'Shift+PadPlus', 'modifiers': ['Shift'], 'key': 'PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Rotate Layer 10 Degrees Counterclockwise | `{'raw': 'Shift+PadMinus', 'modifiers': ['Shift'], 'key': 'PadMinus', 'numeric_keypad': True}`; `{'raw': 'Shift+PadMinus', 'modifiers': ['Shift'], 'key': 'PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 1% Larger | `{'raw': 'Alt+PadPlus)(Ctrl+PadPlus', 'modifiers': ['Alt'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}`; `{'raw': 'Alt+PadPlus)(Ctrl+PadPlus', 'modifiers': ['Alt'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 1% Smaller | `{'raw': 'Alt+PadMinus)(Ctrl+PadMinus', 'modifiers': ['Alt'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}`; `{'raw': 'Alt+PadMinus)(Ctrl+PadMinus', 'modifiers': ['Alt'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 10% Larger | `{'raw': 'Alt+Shift+PadPlus)(Ctrl+Shift+PadPlus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}`; `{'raw': 'Alt+Shift+PadPlus)(Ctrl+Shift+PadPlus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 10% Smaller | `{'raw': 'Alt+Shift+PadMinus)(Ctrl+Shift+PadMinus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}`; `{'raw': 'Alt+Shift+PadMinus)(Ctrl+Shift+PadMinus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |

**Clipboard** (7 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Clear (Delete) | `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'PadClear', 'modifiers': [], 'key': 'PadClear', 'numeric_keypad': True}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}` | CCompCmd, COutline, CPanoProjLayer, CSwitchboardModal, FloPano |
| Copy | `{'raw': 'Ctrl+C', 'modifiers': ['Ctrl'], 'key': 'C'}`; `{'raw': 'Ctrl+C', 'modifiers': ['Ctrl'], 'key': 'C'}` | CSwitchboard, CSwitchboardModal |
| Cut | `{'raw': 'Ctrl+X', 'modifiers': ['Ctrl'], 'key': 'X'}`; `{'raw': 'Ctrl+X', 'modifiers': ['Ctrl'], 'key': 'X'}` | CSwitchboard, CSwitchboardModal |
| Paste | `{'raw': 'Ctrl+V', 'modifiers': ['Ctrl'], 'key': 'V'}`; `{'raw': 'Ctrl+V', 'modifiers': ['Ctrl'], 'key': 'V'}` | CSwitchboard, CSwitchboardModal |
| Redo Last Action | `{'raw': 'Ctrl+Shift+Z', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Z'}`; `{'raw': 'Ctrl+Shift+Z', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Z'}` | CSwitchboard, CSwitchboardModal |
| Select All | `{'raw': 'Ctrl+A', 'modifiers': ['Ctrl'], 'key': 'A'}`; `{'raw': 'Ctrl+A', 'modifiers': ['Ctrl'], 'key': 'A'}` | CSwitchboard, CSwitchboardModal |
| Undo Last Action | `{'raw': 'Ctrl+Z', 'modifiers': ['Ctrl'], 'key': 'Z'}`; `{'raw': 'Ctrl+Z', 'modifiers': ['Ctrl'], 'key': 'Z'}` | CSwitchboard, CSwitchboardModal |

**CloneToolPresets** (5 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Clone Preset 1 | `{'raw': '3', 'modifiers': [], 'key': '3'}` | CCompCloneCmd |
| Clone Preset 2 | `{'raw': '4', 'modifiers': [], 'key': '4'}` | CCompCloneCmd |
| Clone Preset 3 | `{'raw': '5', 'modifiers': [], 'key': '5'}` | CCompCloneCmd |
| Clone Preset 4 | `{'raw': '6', 'modifiers': [], 'key': '6'}` | CCompCloneCmd |
| Clone Preset 5 | `{'raw': '7', 'modifiers': [], 'key': '7'}` | CCompCloneCmd |

**CompositionPanel-Views** (31 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Copy Frame to Clipboard | `{'raw': 'Ctrl+Alt+Shift+F5', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'F5'}` | CItem |
| Enable Draft 3D | -- | -- |
| Purge Snapshot 1 | `{'raw': 'Ctrl+Shift+F5', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F5'}` | CItem |
| Purge Snapshot 2 | `{'raw': 'Ctrl+Shift+F6', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F6'}` | CItem |
| Purge Snapshot 3 | `{'raw': 'Ctrl+Shift+F7', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F7'}` | CItem |
| Purge Snapshot 4 | `{'raw': 'Ctrl+Shift+F8', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F8'}` | CItem |
| Show Snapshot 1 | `{'raw': 'F5', 'modifiers': [], 'key': 'F5'}` | CItem |
| Show Snapshot 2 | `{'raw': 'F6', 'modifiers': [], 'key': 'F6'}` | CItem |
| Show Snapshot 3 | `{'raw': 'F7', 'modifiers': [], 'key': 'F7'}` | CItem |
| Show Snapshot 4 | `{'raw': 'F8', 'modifiers': [], 'key': 'F8'}` | CItem |
| Show/Hide Alpha Channel (Grayscale) | `{'raw': 'Alt+4', 'modifiers': ['Alt'], 'key': '4'}` | CItem |
| Show/Hide Blue Channel (Colorized) | `{'raw': 'Alt+Shift+3', 'modifiers': ['Alt', 'Shift'], 'key': '3'}` | CItem |
| Show/Hide Blue Channel (Grayscale) | `{'raw': 'Alt+3', 'modifiers': ['Alt'], 'key': '3'}` | CItem |
| Show/Hide Green Channel (Colorized) | `{'raw': 'Alt+Shift+2', 'modifiers': ['Alt', 'Shift'], 'key': '2'}` | CItem |
| Show/Hide Green Channel (Grayscale) | `{'raw': 'Alt+2', 'modifiers': ['Alt'], 'key': '2'}` | CItem |
| Show/Hide Grid | `{'raw': 'Shift+SingleQuote', 'modifiers': ['Shift'], 'key': 'SingleQuote'}` | CItem |
| Show/Hide Proportional Grid | `{'raw': 'Alt+SingleQuote', 'modifiers': ['Alt'], 'key': 'SingleQuote'}` | CItem |
| Show/Hide RGB Straight Color | `{'raw': 'Alt+Shift+4', 'modifiers': ['Alt', 'Shift'], 'key': '4'}` | CItem |
| Show/Hide Red Channel (Colorized) | `{'raw': 'Alt+Shift+1', 'modifiers': ['Alt', 'Shift'], 'key': '1'}` | CItem |
| Show/Hide Red Channel (Grayscale) | `{'raw': 'Alt+1', 'modifiers': ['Alt'], 'key': '1'}` | CItem |
| Show/Hide Title/Action Safe Guides | `{'raw': 'SingleQuote', 'modifiers': [], 'key': 'SingleQuote'}` | CItem |
| Switch Fast Preview to Adaptive Resolution | -- | -- |
| Switch Fast Preview to Off (Final Quality) | -- | -- |
| Switch Fast Preview to Wireframe | -- | -- |
| Take Snapshot 1 | `{'raw': 'Shift+F5', 'modifiers': ['Shift'], 'key': 'F5'}` | CItem |
| Take Snapshot 2 | `{'raw': 'Shift+F6', 'modifiers': ['Shift'], 'key': 'F6'}` | CItem |
| Take Snapshot 3 | `{'raw': 'Shift+F7', 'modifiers': ['Shift'], 'key': 'F7'}` | CItem |
| Take Snapshot 4 | `{'raw': 'Shift+F8', 'modifiers': ['Shift'], 'key': 'F8'}` | CItem |
| Toggle Alpha Boundary | `{'raw': 'Alt+5', 'modifiers': ['Alt'], 'key': '5'}` | CItem |
| Toggle Alpha Overlay | `{'raw': 'Alt+6', 'modifiers': ['Alt'], 'key': '6'}` | CItem |
| Toggle Refine Edge X-ray | `{'raw': 'Alt+X', 'modifiers': ['Alt'], 'key': 'X'}` | CItem |

**CompositionPanel-Zoom-Masks** (8 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Enable/Disable Display Color Management | `{'raw': 'Shift+PadSlash)(macControl+Alt+/', 'modifiers': ['Shift', 'Alt'], 'key': 'PadSlash)(macControl+/', 'numeric_keypad': True}` | CSwitchboard |
| Fit | `{'raw': 'Shift+/', 'modifiers': ['Shift'], 'key': '/'}`; `{'raw': 'Shift+/', 'modifiers': ['Shift'], 'key': '/'}` | CPanoECOutline, CPanoProjItem |
| Fit up to 100% | `{'raw': 'Alt+/', 'modifiers': ['Alt'], 'key': '/'}`; `{'raw': 'Alt+/', 'modifiers': ['Alt'], 'key': '/'}` | CPanoECOutline, CPanoProjItem |
| Select Next Mask | `{'raw': 'Alt+`', 'modifiers': ['Alt'], 'key': '`'}` | CPanoProjItem |
| Select Previous Mask | `{'raw': 'Alt+Shift+`', 'modifiers': ['Alt', 'Shift'], 'key': '`'}` | CPanoProjItem |
| Zoom In | `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom Out | `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom to 100% | `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |

**EffectControlsPanel** (15 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Collapse Selected Property Group | `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CPanoECOutline, POutlinePano, RQOutlinePano |
| Expand Selected Property Group | `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CPanoECOutline, POutlinePano, RQOutlinePano |
| Extend Selection to Next Effect | `{'raw': 'Ctrl+Shift+DownArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Extend Selection to Previous Effect | `{'raw': 'Ctrl+Shift+UpArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Fit | `{'raw': 'Shift+/', 'modifiers': ['Shift'], 'key': '/'}`; `{'raw': 'Shift+/', 'modifiers': ['Shift'], 'key': '/'}` | CPanoECOutline, CPanoProjItem |
| Fit up to 100% | `{'raw': 'Alt+/', 'modifiers': ['Alt'], 'key': '/'}`; `{'raw': 'Alt+/', 'modifiers': ['Alt'], 'key': '/'}` | CPanoECOutline, CPanoProjItem |
| Select Next Effect | `{'raw': 'Ctrl+DownArrow', 'modifiers': ['Ctrl'], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Select Previous Effect | `{'raw': 'Ctrl+UpArrow', 'modifiers': ['Ctrl'], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Switch to Composition Panel | `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}` | CEggApp, CPanoECOutline, CPanoProjLayer, FloPano |
| Zoom In | `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom In Resize | `{'raw': 'Alt+.)(Ctrl+=', 'modifiers': ['Alt'], 'key': '.)(Ctrl+='}`; `{'raw': 'Alt+.)(Ctrl+=', 'modifiers': ['Alt'], 'key': '.)(Ctrl+='}` | CPanoECOutline, CPanoProjItem |
| Zoom No Scroll | `{'raw': 'Ctrl+Alt+/', 'modifiers': ['Ctrl', 'Alt'], 'key': '/'}`; `{'raw': 'Ctrl+Alt+/', 'modifiers': ['Ctrl', 'Alt'], 'key': '/'}` | CPanoECOutline, CPanoProjItem |
| Zoom Out | `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom Out Resize | `{'raw': 'Alt+Comma)(Ctrl+-', 'modifiers': ['Alt'], 'key': 'Comma)(Ctrl+-'}`; `{'raw': 'Alt+Comma)(Ctrl+-', 'modifiers': ['Alt'], 'key': 'Comma)(Ctrl+-'}` | CPanoECOutline, CPanoProjItem |
| Zoom to 100% | `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |

**FlowchartPanel** (6 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Clear Flowchart Panel Without Confirmation | `{'raw': 'Ctrl+Delete', 'modifiers': ['Ctrl'], 'key': 'Delete'}`; `{'raw': 'Ctrl+Delete', 'modifiers': ['Ctrl'], 'key': 'Delete'}` | FloPano, POutlinePano |
| Delete | `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'PadClear', 'modifiers': [], 'key': 'PadClear', 'numeric_keypad': True}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}` | CCompCmd, COutline, CPanoProjLayer, CSwitchboardModal, FloPano |
| Switch to Composition Panel | `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}` | CEggApp, CPanoECOutline, CPanoProjLayer, FloPano |
| Zoom In | `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom Out | `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom to 100% | `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |

**FootagePanel** (6 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Go to In Point | `{'raw': 'I', 'modifiers': [], 'key': 'I'}`; `{'raw': 'I', 'modifiers': [], 'key': 'I'}` | CCompTime, CPanoProjFootage |
| Go to Next Frame | `{'raw': 'Ctrl+RightArrow)(PageDOWN', 'modifiers': ['Ctrl'], 'key': 'RightArrow)(PageDOWN'}`; `{'raw': 'Ctrl+RightArrow)(PadPageDown)(PageDOWN', 'modifiers': ['Ctrl'], 'key': 'RightArrow)(PadPageDown)(PageDOWN'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CCompTime, CDirItemTabPanelTime, CPanoProjFootage |
| Go to Out Point | `{'raw': 'O', 'modifiers': [], 'key': 'O'}`; `{'raw': 'O', 'modifiers': [], 'key': 'O'}` | CCompTime, CPanoProjFootage |
| Go to Previous Frame | `{'raw': 'Ctrl+LeftArrow)(PageUP', 'modifiers': ['Ctrl'], 'key': 'LeftArrow)(PageUP'}`; `{'raw': 'Ctrl+LeftArrow)(PadPageUp)(PageUP', 'modifiers': ['Ctrl'], 'key': 'LeftArrow)(PadPageUp)(PageUP'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CCompTime, CDirItemTabPanelTime, CPanoProjFootage |
| Set In Point at Current Time | `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |
| Set Out Point at Current Time | `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |

**General** (175 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Add Composition to External Encoder Queue | `{'raw': 'Ctrl+Alt+M', 'modifiers': ['Ctrl', 'Alt'], 'key': 'M'}` | CSwitchboard |
| Add Comp to Render Queue | `{'raw': 'Ctrl+Shift+/)(Ctrl+M', 'modifiers': ['Ctrl', 'Shift'], 'key': '/)(Ctrl+M'}` | CSwitchboard |
| Add Footage Item to Comp | `{'raw': 'Ctrl+/', 'modifiers': ['Ctrl'], 'key': '/'}`; `{'raw': 'Ctrl+/', 'modifiers': ['Ctrl'], 'key': '/'}` | CSwitchboard, POutlinePano |
| Apply Last Effect | `{'raw': 'Ctrl+Alt+Shift+E', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'E'}` | CSwitchboard |
| Apply Recently Used Animation Preset | `{'raw': 'Ctrl+Alt+Shift+F', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'F'}` | CSwitchboard |
| Auto-Orient | `{'raw': 'Ctrl+Alt+O', 'modifiers': ['Ctrl', 'Alt'], 'key': 'O'}` | CSwitchboard |
| Bring Effect Control Panel to Front | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Bring Layer Forward | `{'raw': 'Ctrl+Alt+UpArrow)(Ctrl+]', 'modifiers': ['Ctrl', 'Alt'], 'key': 'UpArrow)(Ctrl+]'}` | CSwitchboard |
| Bring Layer to Front | `{'raw': 'Ctrl+Alt+Shift+UpArrow)(Ctrl+Shift+=)(Ctrl+Shift+]', 'modifiers': ['Ctrl', 'Alt', 'Shift', 'Shift', 'Shift'], 'key': 'UpArrow)(Ctrl+=)(Ctrl+]'}` | CSwitchboard |
| Cache Frames When Idle | `{'raw': 'Alt+Shift+I', 'modifiers': ['Alt', 'Shift'], 'key': 'I'}` | CSwitchboard |
| Cache Work Area in Background | -- | -- |
| Clear (Delete) | `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'PadClear', 'modifiers': [], 'key': 'PadClear', 'numeric_keypad': True}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}` | CCompCmd, COutline, CPanoProjLayer, CSwitchboardModal, FloPano |
| Close Current Panel or Viewer Contents | `{'raw': 'Ctrl+W', 'modifiers': ['Ctrl'], 'key': 'W'}` | CSwitchboard |
| Close Current Viewer or All Viewers of Same Type | `{'raw': 'Ctrl+Shift+W', 'modifiers': ['Ctrl', 'Shift'], 'key': 'W'}` | CSwitchboard |
| Composition Settings | `{'raw': 'Ctrl+K', 'modifiers': ['Ctrl'], 'key': 'K'}` | CSwitchboard |
| Copy | `{'raw': 'Ctrl+C', 'modifiers': ['Ctrl'], 'key': 'C'}`; `{'raw': 'Ctrl+C', 'modifiers': ['Ctrl'], 'key': 'C'}` | CSwitchboard, CSwitchboardModal |
| Copy with Property Links | `{'raw': 'Ctrl+Alt+C', 'modifiers': ['Ctrl', 'Alt'], 'key': 'C'}` | CSwitchboard |
| Copy with Relative Property Links | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Cut | `{'raw': 'Ctrl+X', 'modifiers': ['Ctrl'], 'key': 'X'}`; `{'raw': 'Ctrl+X', 'modifiers': ['Ctrl'], 'key': 'X'}` | CSwitchboard, CSwitchboardModal |
| Delete All Effects from Selected Layers | `{'raw': 'Ctrl+Shift+E', 'modifiers': ['Ctrl', 'Shift'], 'key': 'E'}` | CSwitchboard |
| Deselect All Keyframes and Properties | `{'raw': 'Shift+F2)(Ctrl+Alt+Shift+A', 'modifiers': ['Shift', 'Alt', 'Shift'], 'key': 'F2)(Ctrl+A'}` | CSwitchboard |
| Deselect All Layers | `{'raw': 'F2)(Ctrl+Shift+A', 'modifiers': ['Shift'], 'key': 'F2)(Ctrl+A'}` | CSwitchboard |
| Duplicate | `{'raw': 'Ctrl+D', 'modifiers': ['Ctrl'], 'key': 'D'}` | CSwitchboard |
| Edit Original | `{'raw': 'Ctrl+E', 'modifiers': ['Ctrl'], 'key': 'E'}` | CSwitchboard |
| Effect Plugin Manager | -- | -- |
| Enable Time Remapping | `{'raw': 'Ctrl+Alt+T', 'modifiers': ['Ctrl', 'Alt'], 'key': 'T'}` | CSwitchboard |
| Find (Search Filter) | `{'raw': 'Ctrl+F', 'modifiers': ['Ctrl'], 'key': 'F'}` | CSwitchboard |
| Go to Time | `{'raw': 'Alt+Shift+J', 'modifiers': ['Alt', 'Shift'], 'key': 'J'}` | CSwitchboard |
| Group Shapes | `{'raw': 'Ctrl+G', 'modifiers': ['Ctrl'], 'key': 'G'}` | CSwitchboard |
| Help | `{'raw': 'F1', 'modifiers': [], 'key': 'F1'}` | CSwitchboard |
| Import File | `{'raw': 'Ctrl+I', 'modifiers': ['Ctrl'], 'key': 'I'}` | CSwitchboard |
| Import Multiple Files | `{'raw': 'Ctrl+Alt+I', 'modifiers': ['Ctrl', 'Alt'], 'key': 'I'}` | CSwitchboard |
| Increment and Save Project | `{'raw': 'Ctrl+Alt+Shift+S', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'S'}` | CSwitchboard |
| Interpret Footage | `{'raw': 'Ctrl+Alt+G', 'modifiers': ['Ctrl', 'Alt'], 'key': 'G'}` | CSwitchboard |
| Invert Mask | `{'raw': 'Ctrl+Shift+I', 'modifiers': ['Ctrl', 'Shift'], 'key': 'I'}` | CSwitchboard |
| Keyframe Interpolation (Edit in Dialog Box) | `{'raw': 'Ctrl+Alt+K', 'modifiers': ['Ctrl', 'Alt'], 'key': 'K'}` | CSwitchboard |
| Keyframe Velocity (Edit in Dialog Box) | `{'raw': 'Ctrl+Shift+K', 'modifiers': ['Ctrl', 'Shift'], 'key': 'K'}` | CSwitchboard |
| Layer Settings | `{'raw': 'Ctrl+Shift+Y', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Y'}` | CSwitchboard |
| Open Learning Resources | -- | -- |
| Make Movie | -- | -- |
| Mask Feather (Edit in Dialog Box) | `{'raw': 'Ctrl+Shift+F', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F'}` | CSwitchboard |
| Mask Free-Transform | `{'raw': 'Ctrl+T', 'modifiers': ['Ctrl'], 'key': 'T'}` | CSwitchboard |
| Mask Shape (Edit in Dialog Box) | `{'raw': 'Ctrl+Shift+M', 'modifiers': ['Ctrl', 'Shift'], 'key': 'M'}` | CSwitchboard |
| Move Camera and its POI to Look at Selected Layers | `{'raw': 'Ctrl+Alt+Shift+Backslash', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'Backslash'}`; `{'raw': 'F', 'modifiers': [], 'key': 'F'}` | CSwitchboard, CameraToolUI |
| New Adjustment Layer | `{'raw': 'Ctrl+Alt+Y', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Y'}` | CSwitchboard |
| New Camera Layer | `{'raw': 'Ctrl+Alt+Shift+C', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'C'}` | CSwitchboard |
| New Comp from Selection | `{'raw': 'Alt+Backslash', 'modifiers': ['Alt'], 'key': 'Backslash'}` | CSwitchboard |
| New Composition | `{'raw': 'Ctrl+N', 'modifiers': ['Ctrl'], 'key': 'N'}` | CSwitchboard |
| New Content-Aware Fill Layer | -- | -- |
| New Light Layer | `{'raw': 'Ctrl+Alt+Shift+L', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'L'}` | CSwitchboard |
| New Mask | `{'raw': 'Ctrl+Shift+N', 'modifiers': ['Ctrl', 'Shift'], 'key': 'N'}` | CSwitchboard |
| New Null Object | `{'raw': 'Ctrl+Alt+Shift+Y', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'Y'}` | CSwitchboard |
| New Project | `{'raw': 'Ctrl+Alt+N', 'modifiers': ['Ctrl', 'Alt'], 'key': 'N'}`; `{'raw': 'Ctrl+Alt+N', 'modifiers': ['Ctrl', 'Alt'], 'key': 'N'}` | CEggApp, CSwitchboard |
| New Solid Layer | `{'raw': 'Ctrl+Y', 'modifiers': ['Ctrl'], 'key': 'Y'}` | CSwitchboard |
| New Text Layer | `{'raw': 'Ctrl+Alt+Shift+T', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'T'}` | CSwitchboard |
| Next Layer Blending Mode | `{'raw': 'Shift+=', 'modifiers': ['Shift'], 'key': '='}` | CSwitchboard |
| Opacity (Edit in Dialog Box) | `{'raw': 'Ctrl+Shift+O', 'modifiers': ['Ctrl', 'Shift'], 'key': 'O'}` | CSwitchboard |
| Open Most Recent Project | `{'raw': 'Ctrl+Alt+Shift+P', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'P'}` | CSwitchboard |
| Open Project | `{'raw': 'Ctrl+O', 'modifiers': ['Ctrl'], 'key': 'O'}` | CSwitchboard |
| Orientation (Edit in Dialog Box) | `{'raw': 'Ctrl+Alt+Shift+R', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'R'}` | CSwitchboard |
| Paste | `{'raw': 'Ctrl+V', 'modifiers': ['Ctrl'], 'key': 'V'}`; `{'raw': 'Ctrl+V', 'modifiers': ['Ctrl'], 'key': 'V'}` | CSwitchboard, CSwitchboardModal |
| Paste Text Formatting Only | `{'raw': 'Ctrl+Shift+Alt+B', 'modifiers': ['Ctrl', 'Shift', 'Alt'], 'key': 'B'}` | CSwitchboard |
| Paste Text and Match Formatting | `{'raw': 'Ctrl+Shift+B', 'modifiers': ['Ctrl', 'Shift'], 'key': 'B'}` | CSwitchboard |
| Position (Edit in Dialog Box) | `{'raw': 'Ctrl+Shift+P', 'modifiers': ['Ctrl', 'Shift'], 'key': 'P'}` | CSwitchboard |
| Pre-compose | `{'raw': 'Ctrl+Shift+C', 'modifiers': ['Ctrl', 'Shift'], 'key': 'C'}` | CSwitchboard |
| Preferences | `{'raw': 'Ctrl+Alt+;', 'modifiers': ['Ctrl', 'Alt'], 'key': ';'}` | CSwitchboard |
| Previous Layer Blending Mode | `{'raw': 'Shift+-', 'modifiers': ['Shift'], 'key': '-'}` | CSwitchboard |
| Project Settings | `{'raw': 'Ctrl+Alt+Shift+K', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'K'}` | CSwitchboard |
| Purge All Memory | `{'raw': 'macControl+PadClear)(Ctrl+Alt+PadSlash', 'modifiers': ['Alt'], 'key': 'macControl+PadClear)(Ctrl+PadSlash'}` | CSwitchboard |
| Quick Apply#{comment}DVAAE-4235786 | -- | -- |
| Quit | `{'raw': 'Ctrl+Q', 'modifiers': ['Ctrl'], 'key': 'Q'}` | CSwitchboard |
| Redo Last Action | `{'raw': 'Ctrl+Shift+Z', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Z'}`; `{'raw': 'Ctrl+Shift+Z', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Z'}` | CSwitchboard, CSwitchboardModal |
| Reload Selected Footage Items | `{'raw': 'Ctrl+Alt+L', 'modifiers': ['Ctrl', 'Alt'], 'key': 'L'}` | CSwitchboard |
| Replace Footage | `{'raw': 'Ctrl+H', 'modifiers': ['Ctrl'], 'key': 'H'}` | CSwitchboard |
| Replace Selected Layer with Selected Footage Item | `{'raw': 'Ctrl+Alt+/', 'modifiers': ['Ctrl', 'Alt'], 'key': '/'}`; `{'raw': 'Ctrl+Alt+/', 'modifiers': ['Ctrl', 'Alt'], 'key': '/'}` | CSwitchboard, POutlinePano |
| Reverse Paste Keyframes | -- | -- |
| Rotation (Edit in Dialog Box) | `{'raw': 'Ctrl+Shift+R', 'modifiers': ['Ctrl', 'Shift'], 'key': 'R'}` | CSwitchboard |
| Run Script # 1 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 2 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 3 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 4 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 5 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 6 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 7 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 8 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script # 9 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #10 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #11 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #12 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #13 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #14 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #15 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #16 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #17 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #18 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #19 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script #20 | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Run Script File | `{'raw': '', 'unbound': True}` | CSwitchboard |
| Save Current Preview | `{'raw': 'macControl+Shift+C)(Ctrl+Pad0)(Ctrl+PadInsert', 'modifiers': ['Shift'], 'key': 'macControl+C)(Ctrl+Pad0)(Ctrl+PadInsert'}` | CSwitchboard |
| Save Current Preview | `{'raw': 'Ctrl+Shift+Pad0)(Ctrl+Shift+PadInsert', 'modifiers': ['Ctrl', 'Shift', 'Shift'], 'key': 'Pad0)(Ctrl+PadInsert', 'numeric_keypad': True}` | CSwitchboard |
| Save Frame As | `{'raw': 'Ctrl+Alt+S', 'modifiers': ['Ctrl', 'Alt'], 'key': 'S'}` | CSwitchboard |
| Save Project | `{'raw': 'Ctrl+S', 'modifiers': ['Ctrl'], 'key': 'S'}` | CSwitchboard |
| Save Project As | `{'raw': 'Ctrl+Shift+S', 'modifiers': ['Ctrl', 'Shift'], 'key': 'S'}` | CSwitchboard |
| Scan for Changed Footage | `{'raw': 'Ctrl+Alt+Shift+Q', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'Q'}` | CSwitchboard |
| Select All | `{'raw': 'Ctrl+A', 'modifiers': ['Ctrl'], 'key': 'A'}`; `{'raw': 'Ctrl+A', 'modifiers': ['Ctrl'], 'key': 'A'}` | CSwitchboard, CSwitchboardModal |
| Send Layer Backward | `{'raw': 'Ctrl+Alt+DownArrow)(Ctrl+[', 'modifiers': ['Ctrl', 'Alt'], 'key': 'DownArrow)(Ctrl+['}` | CSwitchboard |
| Send Layer to Back | `{'raw': 'Ctrl+Alt+Shift+DownArrow)(Ctrl+Shift+-)(Ctrl+Shift+[', 'modifiers': ['Ctrl', 'Alt', 'Shift', 'Shift', 'Shift'], 'key': 'DownArrow)(Ctrl+-)(Ctrl+['}` | CSwitchboard |
| Set Comp Resolution to Custom | `{'raw': 'Ctrl+Alt+J', 'modifiers': ['Ctrl', 'Alt'], 'key': 'J'}` | CSwitchboard |
| Set Comp Resolution to Custom for Focused Comp and Precomps Nested Within it | `{'raw': 'Ctrl+Alt+macControl+J', 'modifiers': ['Ctrl', 'Alt'], 'key': 'macControl+J'}` | CSwitchboard |
| Set Comp Resolution to Half | `{'raw': 'Ctrl+Shift+J', 'modifiers': ['Ctrl', 'Shift'], 'key': 'J'}` | CSwitchboard |
| Set Comp Resolution to Half for Focused Comp and Precomps Nested Within it | `{'raw': 'Ctrl+Shift+macControl+J', 'modifiers': ['Ctrl', 'Shift'], 'key': 'macControl+J'}` | CSwitchboard |
| Set Comp Resolution to High | `{'raw': 'Ctrl+J', 'modifiers': ['Ctrl'], 'key': 'J'}` | CSwitchboard |
| Set Comp Resolution to High for Focused Comp and Precomps Nested Within it | `{'raw': 'Ctrl+macControl+J', 'modifiers': ['Ctrl'], 'key': 'macControl+J'}` | CSwitchboard |
| Set Comp Resolution to Low | `{'raw': 'Ctrl+Alt+Shift+J', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'J'}` | CSwitchboard |
| Set Comp Resolution to Low for Focused Comp and Precomps Nested Within it | `{'raw': 'Ctrl+Alt+Shift+macControl+J', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'macControl+J'}` | CSwitchboard |
| Set Keyframe to Easy Ease | `{'raw': 'F9', 'modifiers': [], 'key': 'F9'}` | CSwitchboard |
| Set Keyframe to Easy Ease In | `{'raw': 'Shift+F9', 'modifiers': ['Shift'], 'key': 'F9'}` | CSwitchboard |
| Set Keyframe to Easy Ease Out | `{'raw': 'Ctrl+Shift+F9', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F9'}` | CSwitchboard |
| Set Layer Quality to Best | `{'raw': 'Ctrl+U', 'modifiers': ['Ctrl'], 'key': 'U'}` | CSwitchboard |
| Set Layer Quality to Best for Focused Comp and Precomps Nested Within it | `{'raw': 'Ctrl+macControl+U', 'modifiers': ['Ctrl'], 'key': 'macControl+U'}` | CSwitchboard |
| Set Layer Quality to Draft | `{'raw': 'Ctrl+Shift+U', 'modifiers': ['Ctrl', 'Shift'], 'key': 'U'}` | CSwitchboard |
| Set Layer Quality to Draft for Focused Comp and Precomps Nested Within it | `{'raw': 'Ctrl+Shift+macControl+U', 'modifiers': ['Ctrl', 'Shift'], 'key': 'macControl+U'}` | CSwitchboard |
| Set Layer Quality to Wireframe | `{'raw': 'Ctrl+Alt+Shift+U', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'U'}` | CSwitchboard |
| Set Layer Quality to Wireframe for All Layers in a Comp | `{'raw': 'Ctrl+Alt+Shift+macControl+U', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'macControl+U'}` | CSwitchboard |
| Set Layer Sampling Quality to Bicubic | `{'raw': 'Alt+Shift+B', 'modifiers': ['Alt', 'Shift'], 'key': 'B'}` | CSwitchboard |
| Set Layer Sampling Quality to Bilinear | `{'raw': 'Alt+B', 'modifiers': ['Alt'], 'key': 'B'}` | CSwitchboard |
| Set Proxy for Selected Footage Item | `{'raw': 'Ctrl+Alt+P', 'modifiers': ['Ctrl', 'Alt'], 'key': 'P'}` | CSwitchboard |
| Show File Name of Frame's Footage in Info Panel | `{'raw': 'Ctrl+Alt+E', 'modifiers': ['Ctrl', 'Alt'], 'key': 'E'}` | CSwitchboard |
| Show Flowchart for Composition | `{'raw': 'Ctrl+Shift+F11', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F11'}` | CSwitchboard |
| Show Flowchart for Project | `{'raw': 'Ctrl+F11', 'modifiers': ['Ctrl'], 'key': 'F11'}` | CSwitchboard |
| Show/Hide Audio Panel | `{'raw': 'Ctrl+4', 'modifiers': ['Ctrl'], 'key': '4'}` | CSwitchboard |
| Show/Hide Brushes Panel | `{'raw': 'Ctrl+9', 'modifiers': ['Ctrl'], 'key': '9'}` | CSwitchboard |
| Show/Hide Character Panel | `{'raw': 'Ctrl+6', 'modifiers': ['Ctrl'], 'key': '6'}` | CSwitchboard |
| Show/Hide Effect Controls Panel for Selected Layers | `{'raw': 'Ctrl+Shift+T)(F3', 'modifiers': ['Ctrl', 'Shift'], 'key': 'T)(F3'}` | CSwitchboard |
| Show/Hide Effects & Presets Panel | `{'raw': 'Ctrl+5', 'modifiers': ['Ctrl'], 'key': '5'}` | CSwitchboard |
| Show/Hide Info Panel | `{'raw': 'Ctrl+2', 'modifiers': ['Ctrl'], 'key': '2'}` | CSwitchboard |
| Show/Hide Paint Panel | `{'raw': 'Ctrl+8', 'modifiers': ['Ctrl'], 'key': '8'}` | CSwitchboard |
| Show/Hide Paragraph Panel | `{'raw': 'Ctrl+7', 'modifiers': ['Ctrl'], 'key': '7'}` | CSwitchboard |
| Show/Hide Preview Panel | `{'raw': 'Ctrl+3', 'modifiers': ['Ctrl'], 'key': '3'}` | CSwitchboard |
| Show/Hide Project Panel | `{'raw': 'Ctrl+0', 'modifiers': ['Ctrl'], 'key': '0'}` | CSwitchboard |
| Show/Hide Properties with Animation (Double-Tap) | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | CCompCompCmd, CSwitchboard |
| Show/Hide Properties with Animation (Extend; Double-Tap) | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | CCompCompCmd, CSwitchboard |
| Show/Hide Properties with Keyframes (Double-Tap) | `{'raw': 'U', 'modifiers': [], 'key': 'U'}`; `{'raw': 'U', 'modifiers': [], 'key': 'U'}` | CCompCompCmd, CSwitchboard |
| Show/Hide Properties with Keyframes (Extend; Double-Tap) | `{'raw': 'Shift+U', 'modifiers': ['Shift'], 'key': 'U'}`; `{'raw': 'Shift+U', 'modifiers': ['Shift'], 'key': 'U'}` | CCompCompCmd, CSwitchboard |
| Show/Hide Render Queue Panel | `{'raw': 'Ctrl+Alt+0', 'modifiers': ['Ctrl', 'Alt'], 'key': '0'}` | CSwitchboard |
| Show/Hide Tools Panel | `{'raw': 'Ctrl+1', 'modifiers': ['Ctrl'], 'key': '1'}` | CSwitchboard |
| Split Layer at Current Time | `{'raw': 'Ctrl+Shift+D', 'modifiers': ['Ctrl', 'Shift'], 'key': 'D'}` | CSwitchboard |
| Switch to 3D View A | `{'raw': 'F10', 'modifiers': [], 'key': 'F10'}` | CSwitchboard |
| Switch to 3D View B | `{'raw': 'F11', 'modifiers': [], 'key': 'F11'}` | CSwitchboard |
| Switch to 3D View C | `{'raw': 'F12', 'modifiers': [], 'key': 'F12'}` | CSwitchboard |
| Switch to Last 3D View | `{'raw': 'Esc', 'modifiers': [], 'key': 'Esc'}` | CSwitchboard |
| Switch to Next Item in Viewer | `{'raw': 'Shift+.', 'modifiers': ['Shift'], 'key': '.'}` | CSwitchboard |
| Switch to Previous Item in Viewer | `{'raw': 'Shift+Comma', 'modifiers': ['Shift'], 'key': 'Comma'}` | CSwitchboard |
| Switch to Workspace #1 | `{'raw': 'Shift+F10', 'modifiers': ['Shift'], 'key': 'F10'}` | CSwitchboard |
| Switch to Workspace #2 | `{'raw': 'Shift+F11', 'modifiers': ['Shift'], 'key': 'F11'}` | CSwitchboard |
| Switch to Workspace #3 | `{'raw': 'Shift+F12', 'modifiers': ['Shift'], 'key': 'F12'}` | CSwitchboard |
| Toggle Casts Shadows for Selected 3D Layers | `{'raw': 'Alt+Shift+C', 'modifiers': ['Alt', 'Shift'], 'key': 'C'}` | CSwitchboard |
| Toggle Expression for Selected Properties | `{'raw': 'Alt+Shift+=', 'modifiers': ['Alt', 'Shift'], 'key': '='}` | CSwitchboard |
| Toggle Hold Interpolation for Selected Keyframes | `{'raw': 'Ctrl+Alt+H', 'modifiers': ['Ctrl', 'Alt'], 'key': 'H'}` | CSwitchboard |
| Toggle Lock Guides | `{'raw': 'Ctrl+Alt+Shift+;', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': ';'}` | CSwitchboard |
| Toggle Lock Switch for Selected Layers | `{'raw': 'Ctrl+L', 'modifiers': ['Ctrl'], 'key': 'L'}` | CSwitchboard |
| Toggle Show Grid | `{'raw': 'Ctrl+SingleQuote', 'modifiers': ['Ctrl'], 'key': 'SingleQuote'}` | CSwitchboard |
| Toggle Show Guides | `{'raw': 'Ctrl+;', 'modifiers': ['Ctrl'], 'key': ';'}` | CSwitchboard |
| Toggle Show Layer Controls | `{'raw': 'Ctrl+Shift+H', 'modifiers': ['Ctrl', 'Shift'], 'key': 'H'}` | CSwitchboard |
| Toggle Show Rulers | `{'raw': 'Ctrl+R', 'modifiers': ['Ctrl'], 'key': 'R'}` | CSwitchboard |
| Toggle Snap to Grid | `{'raw': 'Ctrl+Shift+SingleQuote', 'modifiers': ['Ctrl', 'Shift'], 'key': 'SingleQuote'}` | CSwitchboard |
| Toggle Snap to Guides | `{'raw': 'Ctrl+Shift+;', 'modifiers': ['Ctrl', 'Shift'], 'key': ';'}` | CSwitchboard |
| Toggle Switches / Modes Column | `{'raw': 'F4', 'modifiers': [], 'key': 'F4'}` | CSwitchboard |
| Toggle Use Display Color Management | `{'raw': 'Shift+PadSlash)(macControl+Alt+/', 'modifiers': ['Shift', 'Alt'], 'key': 'PadSlash)(macControl+/', 'numeric_keypad': True}` | CSwitchboard |
| Toggle Video Switch for Selected Layers | `{'raw': 'Ctrl+Alt+Shift+V', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'V'}` | CSwitchboard |
| Trim Comp to Work Area | `{'raw': 'Ctrl+Shift+X', 'modifiers': ['Ctrl', 'Shift'], 'key': 'X'}` | CSwitchboard |
| Undo Last Action | `{'raw': 'Ctrl+Z', 'modifiers': ['Ctrl'], 'key': 'Z'}`; `{'raw': 'Ctrl+Z', 'modifiers': ['Ctrl'], 'key': 'Z'}` | CSwitchboard, CSwitchboardModal |
| Ungroup Shapes | `{'raw': 'Ctrl+Shift+G', 'modifiers': ['Ctrl', 'Shift'], 'key': 'G'}` | CSwitchboard |
| Unlock All Layers | `{'raw': 'Ctrl+Shift+L', 'modifiers': ['Ctrl', 'Shift'], 'key': 'L'}` | CSwitchboard |
| View Options | `{'raw': 'Ctrl+Alt+U', 'modifiers': ['Ctrl', 'Alt'], 'key': 'U'}` | CSwitchboard |

**GeneralPanel** (7 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Close Timeline Panels to the Left#{comment}DVAAE-4234478 | -- | -- |
| Close Timeline Panels to the Right#{comment}DVAAE-4234478 | -- | -- |
| Maximize App Window | -- | -- |
| Maximize App Window on Primary Monitor | -- | -- |
| Maximize Panel | -- | -- |
| Select Next Panel in Frame | -- | -- |
| Select Previous Panel in Frame | -- | -- |

**LayerPanel** (9 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Clear (Delete) | `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'PadClear', 'modifiers': [], 'key': 'PadClear', 'numeric_keypad': True}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}` | CCompCmd, COutline, CPanoProjLayer, CSwitchboardModal, FloPano |
| Delete Layer's Mask | `{'raw': 'Alt+Delete)(Alt+FwdDel', 'modifiers': ['Alt'], 'key': 'Delete)(Alt+FwdDel'}`; `{'raw': 'Alt+Delete)(Alt+FwdDel', 'modifiers': ['Alt'], 'key': 'Delete)(Alt+FwdDel'}` | CCompCmd, CPanoProjLayer |
| Set Layer In Point at Current Time | `{'raw': '[', 'modifiers': [], 'key': '['}`; `{'raw': '[', 'modifiers': [], 'key': '['}`; `{'raw': '[', 'modifiers': [], 'key': '['}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Set Layer Out Point at Current Time | `{'raw': ']', 'modifiers': [], 'key': ']'}`; `{'raw': ']', 'modifiers': [], 'key': ']'}`; `{'raw': ']', 'modifiers': [], 'key': ']'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Stretch Layer In Point to Current Time | `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Stretch Layer Out Point to Current Time | `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Switch to Composition Panel | `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}` | CEggApp, CPanoECOutline, CPanoProjLayer, FloPano |
| Trim Layer In Point to Current Time | `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |
| Trim Layer Out Point to Current Time | `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |

**LayerPanelMagnification** (3 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Zoom In | `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': 'Ctrl+Alt+=)(.', 'modifiers': ['Ctrl', 'Alt'], 'key': '=)(.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}`; `{'raw': '.', 'modifiers': [], 'key': '.'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom Out | `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Ctrl+Alt+-)(Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': '-)(Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}`; `{'raw': 'Comma', 'modifiers': [], 'key': 'Comma'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |
| Zoom to 100% | `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}`; `{'raw': '/', 'modifiers': [], 'key': '/'}` | CPanoECOutline, CPanoProjItem, CPanoProjLayerPanoMask, FloPano |

**LayerProperties** (4 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Show/Hide All of the Selected Layers' Properties | `{'raw': 'Ctrl+`', 'modifiers': ['Ctrl'], 'key': '`'}`; `{'raw': 'Ctrl+`', 'modifiers': ['Ctrl'], 'key': '`'}` | CCompCompCmd, CTopic |
| Twirl | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | CCompCompCmd, CTopic |
| Twirl and Expand Groups | `{'raw': 'Ctrl+Shift+`', 'modifiers': ['Ctrl', 'Shift'], 'key': '`'}`; `{'raw': 'Ctrl+Shift+`', 'modifiers': ['Ctrl', 'Shift'], 'key': '`'}` | CCompCompCmd, CTopic |
| Twirl and Preserve Visibility | `{'raw': 'Shift+`', 'modifiers': ['Shift'], 'key': '`'}`; `{'raw': 'Shift+`', 'modifiers': ['Shift'], 'key': '`'}` | CCompCompCmd, CTopic |

**MacSysShortcutsAlt** (3 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| MacSysMenuHideMe | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | MacSysShortcutsAlt, MacSysShortcutsStd |
| MacSysMenuHideOthers | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | MacSysShortcutsAlt, MacSysShortcutsStd |
| MacSysMenuMinimize | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | MacSysShortcutsAlt, MacSysShortcutsStd |

**MacSysShortcutsStd** (3 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| MacSysMenuHideMe | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | MacSysShortcutsAlt, MacSysShortcutsStd |
| MacSysMenuHideOthers | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | MacSysShortcutsAlt, MacSysShortcutsStd |
| MacSysMenuMinimize | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | MacSysShortcutsAlt, MacSysShortcutsStd |

**Markers** (20 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Create Composition Marker '0' | `{'raw': 'Shift+0', 'modifiers': ['Shift'], 'key': '0'}` | CCompMarkerCmd |
| Create Composition Marker '1' | `{'raw': 'Shift+1', 'modifiers': ['Shift'], 'key': '1'}` | CCompMarkerCmd |
| Create Composition Marker '2' | `{'raw': 'Shift+2', 'modifiers': ['Shift'], 'key': '2'}` | CCompMarkerCmd |
| Create Composition Marker '3' | `{'raw': 'Shift+3', 'modifiers': ['Shift'], 'key': '3'}` | CCompMarkerCmd |
| Create Composition Marker '4' | `{'raw': 'Shift+4', 'modifiers': ['Shift'], 'key': '4'}` | CCompMarkerCmd |
| Create Composition Marker '5' | `{'raw': 'Shift+5', 'modifiers': ['Shift'], 'key': '5'}` | CCompMarkerCmd |
| Create Composition Marker '6' | `{'raw': 'Shift+6', 'modifiers': ['Shift'], 'key': '6'}` | CCompMarkerCmd |
| Create Composition Marker '7' | `{'raw': 'Shift+7', 'modifiers': ['Shift'], 'key': '7'}` | CCompMarkerCmd |
| Create Composition Marker '8' | `{'raw': 'Shift+8', 'modifiers': ['Shift'], 'key': '8'}` | CCompMarkerCmd |
| Create Composition Marker '9' | `{'raw': 'Shift+9', 'modifiers': ['Shift'], 'key': '9'}` | CCompMarkerCmd |
| Go to Composition Marker '0' | `{'raw': '0', 'modifiers': [], 'key': '0'}` | CCompMarkerCmd |
| Go to Composition Marker '1' | `{'raw': '1', 'modifiers': [], 'key': '1'}` | CCompMarkerCmd |
| Go to Composition Marker '2' | `{'raw': '2', 'modifiers': [], 'key': '2'}` | CCompMarkerCmd |
| Go to Composition Marker '3' | `{'raw': '3', 'modifiers': [], 'key': '3'}` | CCompMarkerCmd |
| Go to Composition Marker '4' | `{'raw': '4', 'modifiers': [], 'key': '4'}` | CCompMarkerCmd |
| Go to Composition Marker '5' | `{'raw': '5', 'modifiers': [], 'key': '5'}` | CCompMarkerCmd |
| Go to Composition Marker '6' | `{'raw': '6', 'modifiers': [], 'key': '6'}` | CCompMarkerCmd |
| Go to Composition Marker '7' | `{'raw': '7', 'modifiers': [], 'key': '7'}` | CCompMarkerCmd |
| Go to Composition Marker '8' | `{'raw': '8', 'modifiers': [], 'key': '8'}` | CCompMarkerCmd |
| Go to Composition Marker '9' | `{'raw': '9', 'modifiers': [], 'key': '9'}` | CCompMarkerCmd |

**MotionTracker** (18 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Move Track Point (Not Attach Point) Down 1 Screen Pixel | `{'raw': 'Alt+DownArrow', 'modifiers': ['Alt'], 'key': 'DownArrow'}` | Tracker |
| Move Track Point (Not Attach Point) Down 10 Screen Pixels | `{'raw': 'Alt+Shift+DownArrow', 'modifiers': ['Alt', 'Shift'], 'key': 'DownArrow'}` | Tracker |
| Move Track Point (Not Attach Point) Left 1 Screen Pixel | `{'raw': 'Alt+LeftArrow', 'modifiers': ['Alt'], 'key': 'LeftArrow'}` | Tracker |
| Move Track Point (Not Attach Point) Left 10 Screen Pixels | `{'raw': 'Alt+Shift+LeftArrow', 'modifiers': ['Alt', 'Shift'], 'key': 'LeftArrow'}` | Tracker |
| Move Track Point (Not Attach Point) Right 1 Screen Pixel | `{'raw': 'Alt+RightArrow', 'modifiers': ['Alt'], 'key': 'RightArrow'}` | Tracker |
| Move Track Point (Not Attach Point) Right 10 Screen Pixels | `{'raw': 'Alt+Shift+RightArrow', 'modifiers': ['Alt', 'Shift'], 'key': 'RightArrow'}` | Tracker |
| Move Track Point (Not Attach Point) Up 1 Screen Pixel | `{'raw': 'Alt+UpArrow', 'modifiers': ['Alt'], 'key': 'UpArrow'}` | Tracker |
| Move Track Point (Not Attach Point) Up 10 Screen Pixels | `{'raw': 'Alt+Shift+UpArrow', 'modifiers': ['Alt', 'Shift'], 'key': 'UpArrow'}` | Tracker |
| Move Track Point Down 1 Screen Pixel | `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}` | Tracker |
| Move Track Point Down 10 Screen Pixels | `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}` | Tracker |
| Move Track Point Left 1 Screen Pixel | `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | Tracker |
| Move Track Point Left 10 Screen Pixels | `{'raw': 'Shift+LeftArrow', 'modifiers': ['Shift'], 'key': 'LeftArrow'}` | Tracker |
| Move Track Point Right 1 Screen Pixel | `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | Tracker |
| Move Track Point Right 10 Screen Pixels | `{'raw': 'Shift+RightArrow', 'modifiers': ['Shift'], 'key': 'RightArrow'}` | Tracker |
| Move Track Point Up 1 Screen Pixel | `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}` | Tracker |
| Move Track Point Up 10 Screen Pixels | `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}` | Tracker |
| Set In Point of 'Motion Tracker Points' Bar in Layer Panel | `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}` | Tracker |
| Set Out Point of 'Motion Tracker Points' Bar in Layer Panel | `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}` | Tracker |

**PaintPanel** (24 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Paint Tools - Go Back by Stroke Duration | `{'raw': '1)(Ctrl+PageUP', 'modifiers': [], 'key': '1)(Ctrl+PageUP'}` | CCompPaintCmd |
| Paint Tools - Go Forward by Stroke Duration | `{'raw': '2)(Ctrl+PageDOWN', 'modifiers': [], 'key': '2)(Ctrl+PageDOWN'}` | CCompPaintCmd |
| Reset Foreground/Background to Black/White | `{'raw': 'D', 'modifiers': [], 'key': 'D'}` | CCompPaintCmd |
| Set Paint Flow to 10% | `{'raw': 'Shift+Pad1', 'modifiers': ['Shift'], 'key': 'Pad1', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 20% | `{'raw': 'Shift+Pad2', 'modifiers': ['Shift'], 'key': 'Pad2', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 30% | `{'raw': 'Shift+Pad3', 'modifiers': ['Shift'], 'key': 'Pad3', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 40% | `{'raw': 'Shift+Pad4', 'modifiers': ['Shift'], 'key': 'Pad4', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 50% | `{'raw': 'Shift+Pad5', 'modifiers': ['Shift'], 'key': 'Pad5', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 60% | `{'raw': 'Shift+Pad6', 'modifiers': ['Shift'], 'key': 'Pad6', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 70% | `{'raw': 'Shift+Pad7', 'modifiers': ['Shift'], 'key': 'Pad7', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 80% | `{'raw': 'Shift+Pad8', 'modifiers': ['Shift'], 'key': 'Pad8', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 90% | `{'raw': 'Shift+Pad9', 'modifiers': ['Shift'], 'key': 'Pad9', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Flow to 100% | `{'raw': 'Shift+PadDecimal)(Shift+PadComma)(Shift+PadDelete', 'modifiers': ['Shift'], 'key': 'PadDecimal)(Shift+PadComma)(Shift+PadDelete', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 10% | `{'raw': 'Pad1', 'modifiers': [], 'key': 'Pad1', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 20% | `{'raw': 'Pad2', 'modifiers': [], 'key': 'Pad2', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 30% | `{'raw': 'Pad3', 'modifiers': [], 'key': 'Pad3', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 40% | `{'raw': 'Pad4', 'modifiers': [], 'key': 'Pad4', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 50% | `{'raw': 'Pad5', 'modifiers': [], 'key': 'Pad5', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 60% | `{'raw': 'Pad6', 'modifiers': [], 'key': 'Pad6', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 70% | `{'raw': 'Pad7', 'modifiers': [], 'key': 'Pad7', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 80% | `{'raw': 'Pad8', 'modifiers': [], 'key': 'Pad8', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 90% | `{'raw': 'Pad9', 'modifiers': [], 'key': 'Pad9', 'numeric_keypad': True}` | CCompPaintCmd |
| Set Paint Opacity to 100% | `{'raw': 'PadDecimal)(PadComma)(PadDelete', 'modifiers': [], 'key': 'PadDecimal)(PadComma)(PadDelete', 'numeric_keypad': True}` | CCompPaintCmd |
| Swap Foreground/Background Colors | `{'raw': 'X', 'modifiers': [], 'key': 'X'}` | CCompPaintCmd |

**PreviewPanel** (8 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Go Back 1 Frame | `{'raw': 'Ctrl+LeftArrow)(PageUP', 'modifiers': ['Ctrl'], 'key': 'LeftArrow)(PageUP'}`; `{'raw': 'Ctrl+LeftArrow)(PadPageUp)(PageUP', 'modifiers': ['Ctrl'], 'key': 'LeftArrow)(PadPageUp)(PageUP'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CCompTime, CDirItemTabPanelTime, CPanoProjFootage |
| Go Back 10 Frames | `{'raw': 'Shift+PageUP)(Ctrl+Shift+LeftArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageUP)(Ctrl+LeftArrow'}`; `{'raw': 'Shift+PageUP)(Ctrl+Shift+LeftArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageUP)(Ctrl+LeftArrow'}` | CCompTime, CDirItemTabPanelTime |
| Go Forward 1 Frame | `{'raw': 'Ctrl+RightArrow)(PageDOWN', 'modifiers': ['Ctrl'], 'key': 'RightArrow)(PageDOWN'}`; `{'raw': 'Ctrl+RightArrow)(PadPageDown)(PageDOWN', 'modifiers': ['Ctrl'], 'key': 'RightArrow)(PadPageDown)(PageDOWN'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CCompTime, CDirItemTabPanelTime, CPanoProjFootage |
| Go Forward 10 Frames | `{'raw': 'Shift+PageDOWN)(Ctrl+Shift+RightArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageDOWN)(Ctrl+RightArrow'}`; `{'raw': 'Shift+PageDOWN)(Ctrl+Shift+RightArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageDOWN)(Ctrl+RightArrow'}` | CCompTime, CDirItemTabPanelTime |
| Go to End of Composition | `{'raw': 'Ctrl+Alt+RightArrow)(END', 'modifiers': ['Ctrl', 'Alt'], 'key': 'RightArrow)(END'}`; `{'raw': 'Ctrl+Alt+RightArrow)(PadEnd)(END', 'modifiers': ['Ctrl', 'Alt'], 'key': 'RightArrow)(PadEnd)(END'}` | CCompTime, CDirItemTabPanelTime |
| Go to Start of Composition | `{'raw': 'Ctrl+Alt+LeftArrow)(HOME', 'modifiers': ['Ctrl', 'Alt'], 'key': 'LeftArrow)(HOME'}`; `{'raw': 'Ctrl+Alt+LeftArrow)(PadHome)(HOME', 'modifiers': ['Ctrl', 'Alt'], 'key': 'LeftArrow)(PadHome)(HOME'}` | CCompTime, CDirItemTabPanelTime |
| Preview | `{'raw': 'Shift+Space', 'modifiers': ['Shift'], 'key': 'Space'}`; `{'raw': 'Shift+Space', 'modifiers': ['Shift'], 'key': 'Space'}` | CCompTime, CDirItemTabPanelTime |
| Preview | `{'raw': 'Space', 'modifiers': [], 'key': 'Space'}`; `{'raw': 'Space', 'modifiers': [], 'key': 'Space'}` | CCompTime, CDirItemTabPanelTime |

**Previewing** (15 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Audio Preview (Here Forward) | `{'raw': 'PadDecimal)(PadComma)(PadDelete)(macControl+.', 'modifiers': [], 'key': 'PadDecimal)(PadComma)(PadDelete)(macControl+.', 'numeric_keypad': True}` | CEggApp |
| Audio Preview (Work Area) | `{'raw': 'Alt+PadDecimal)(Alt+PadComma)(Alt+PadDelete)(macControl+Alt+.', 'modifiers': ['Alt', 'Alt'], 'key': 'PadDecimal)(Alt+PadComma)(Alt+PadDelete)(macControl+.', 'numeric_keypad': True}` | CEggApp |
| Browse in Asset Browser | `{'raw': 'Ctrl+Alt+Shift+O', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'O'}` | CEggApp |
| Cancel Preview | `{'raw': 'Esc', 'modifiers': [], 'key': 'Esc'}` | CEggApp |
| Close Current AE Project | `{'raw': 'Ctrl+Alt+Shift+macControl+S', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'macControl+S'}` | CEggApp |
| New Project | `{'raw': 'Ctrl+Alt+N', 'modifiers': ['Ctrl', 'Alt'], 'key': 'N'}`; `{'raw': 'Ctrl+Alt+N', 'modifiers': ['Ctrl', 'Alt'], 'key': 'N'}` | CEggApp, CSwitchboard |
| Open Test Host Plugin | `{'raw': 'Ctrl+Shift+F12', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F12'}` | CEggApp |
| Preview (Numpad) | `{'raw': 'Pad0)(PadInsert)(macControl+0', 'modifiers': [], 'key': 'Pad0)(PadInsert)(macControl+0', 'numeric_keypad': True}` | CEggApp |
| Preview (Option Numpad 0) | `{'raw': 'Alt+Pad0)(Alt+PadInsert)(macControl+Alt+0', 'modifiers': ['Alt', 'Alt'], 'key': 'Pad0)(Alt+PadInsert)(macControl+0', 'numeric_keypad': True}` | CEggApp |
| Preview (Shift Numpad 0) | `{'raw': 'Shift+Pad0)(Shift+PadInsert)(macControl+Shift+0', 'modifiers': ['Shift', 'Shift'], 'key': 'Pad0)(Shift+PadInsert)(macControl+0', 'numeric_keypad': True}` | CEggApp |
| Show Console | `{'raw': 'Ctrl+F12', 'modifiers': ['Ctrl'], 'key': 'F12'}` | CEggApp |
| Show Debug Monitor | -- | -- |
| Switch Between Composition/Timeline Panels | `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}`; `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}` | CEggApp, CPanoECOutline, CPanoProjLayer, FloPano |
| Switch Between Current and Last Accessed Composition | `{'raw': 'Shift+Esc', 'modifiers': ['Shift'], 'key': 'Esc'}` | CEggApp |
| Toggle Mercury Transmit on External Monitor | `{'raw': 'PadSlash)(macControl+Shift+/', 'modifiers': ['Shift'], 'key': 'PadSlash)(macControl+/', 'numeric_keypad': True}` | CEggApp |

**ProjectPanel** (14 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Add Project Item to Current Composition | `{'raw': 'Ctrl+/', 'modifiers': ['Ctrl'], 'key': '/'}`; `{'raw': 'Ctrl+/', 'modifiers': ['Ctrl'], 'key': '/'}` | CSwitchboard, POutlinePano |
| Apply Remembered Interpretation of Footage Item | `{'raw': 'Ctrl+Alt+V', 'modifiers': ['Ctrl', 'Alt'], 'key': 'V'}` | POutlinePano |
| Collapse Selected Folder | `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CPanoECOutline, POutlinePano, RQOutlinePano |
| Delete Without Confirmation | `{'raw': 'Ctrl+Delete', 'modifiers': ['Ctrl'], 'key': 'Delete'}`; `{'raw': 'Ctrl+Delete', 'modifiers': ['Ctrl'], 'key': 'Delete'}` | FloPano, POutlinePano |
| Expand Selected Folder | `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CPanoECOutline, POutlinePano, RQOutlinePano |
| Extend Selection to Next Project Item | `{'raw': 'Ctrl+Shift+DownArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Extend Selection to Previous Project Item | `{'raw': 'Ctrl+Shift+UpArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| New Folder | `{'raw': 'Ctrl+Alt+Shift+N', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'N'}` | POutlinePano |
| Open Footage Item in Footage Panel | `{'raw': 'Enter', 'modifiers': [], 'key': 'Enter'}` | POutlinePano |
| Open Footage Item in New Footage Panel | -- | -- |
| Remember Interpretation of Footage Item | `{'raw': 'Ctrl+Alt+C', 'modifiers': ['Ctrl', 'Alt'], 'key': 'C'}` | POutlinePano |
| Replace Selected Layers with Selected Project Item | `{'raw': 'Ctrl+Alt+/', 'modifiers': ['Ctrl', 'Alt'], 'key': '/'}`; `{'raw': 'Ctrl+Alt+/', 'modifiers': ['Ctrl', 'Alt'], 'key': '/'}` | CSwitchboard, POutlinePano |
| Select Next Project Item | `{'raw': 'Ctrl+DownArrow', 'modifiers': ['Ctrl'], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Select Previous Project Item | `{'raw': 'Ctrl+UpArrow', 'modifiers': ['Ctrl'], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |

**PropertyGroupsandProperties** (2 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Delete | `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'PadClear', 'modifiers': [], 'key': 'PadClear', 'numeric_keypad': True}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}` | CCompCmd, COutline, CPanoProjLayer, CSwitchboardModal, FloPano |
| Rename | `{'raw': 'Return', 'modifiers': [], 'key': 'Return'}` | COutline |

**RenderQueuePanel** (8 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Collapse Selected Render Queue Item | `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CPanoECOutline, POutlinePano, RQOutlinePano |
| Expand Selected Render Queue Item | `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CPanoECOutline, POutlinePano, RQOutlinePano |
| Extend Selection to Next RQ Item or Output Module | `{'raw': 'Ctrl+Shift+DownArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Extend Selection to Previous RQ Item or Output Module | `{'raw': 'Ctrl+Shift+UpArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Select Previous RQ Item or Output Module | `{'raw': 'Ctrl+DownArrow', 'modifiers': ['Ctrl'], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Select Previous RQ Item or Output Module | `{'raw': 'Ctrl+UpArrow', 'modifiers': ['Ctrl'], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Start Render | `{'raw': 'Return)(Enter', 'modifiers': [], 'key': 'Return)(Enter'}` | RQOutlinePano |
| Stop Render | `{'raw': 'Ctrl+.)(Esc', 'modifiers': ['Ctrl'], 'key': '.)(Esc'}`; `{'raw': 'Ctrl+.)(Esc', 'modifiers': ['Ctrl'], 'key': '.)(Esc'}` | CPanoRender, RQOutlinePano |

**TextLayer** (60 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Align Selected Text Center (Horiz or Vert) | `{'raw': 'Shift+Ctrl+C', 'modifiers': ['Shift', 'Ctrl'], 'key': 'C'}` | TextLayerUI |
| Align Selected Text Left (Horiz) or Top (Vert) | `{'raw': 'Shift+Ctrl+L', 'modifiers': ['Shift', 'Ctrl'], 'key': 'L'}` | TextLayerUI |
| Align Selected Text Right (Horiz) or Bottom (Vert) | `{'raw': 'Shift+Ctrl+R', 'modifiers': ['Shift', 'Ctrl'], 'key': 'R'}` | TextLayerUI |
| Auto Leading for Selected Text | `{'raw': 'Shift+Ctrl+Alt+A', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'A'}` | TextLayerUI |
| Backspace (Previous Character) | `{'raw': 'Delete', 'modifiers': [], 'key': 'Delete'}` | TextLayerUI |
| Cancel Changes to Text | `{'raw': 'Esc', 'modifiers': [], 'key': 'Esc'}` | TextLayerUI |
| Commit Changes to Text | `{'raw': 'Enter)(Ctrl+Return', 'modifiers': [], 'key': 'Enter)(Ctrl+Return'}` | TextLayerUI |
| Decrease Baseline Shift by 10 px | `{'raw': 'Shift+Ctrl+Alt+DownArrow', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'DownArrow'}` | TextLayerUI |
| Decrease Baseline Shift by 2 px | `{'raw': 'Shift+Alt+DownArrow', 'modifiers': ['Shift', 'Alt'], 'key': 'DownArrow'}` | TextLayerUI |
| Decrease Font Size by 10 px | `{'raw': 'Shift+Ctrl+Alt+Comma', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'Comma'}` | TextLayerUI |
| Decrease Font Size by 2 px | `{'raw': 'Shift+Ctrl+Comma', 'modifiers': ['Shift', 'Ctrl'], 'key': 'Comma'}` | TextLayerUI |
| Decrease Kerning or Tracking by 100 units | `{'raw': 'Ctrl+Alt+LeftArrow', 'modifiers': ['Ctrl', 'Alt'], 'key': 'LeftArrow'}` | TextLayerUI |
| Decrease Kerning or Tracking by 20 units | `{'raw': 'Alt+LeftArrow', 'modifiers': ['Alt'], 'key': 'LeftArrow'}` | TextLayerUI |
| Decrease Leading by 10 px | `{'raw': 'Ctrl+Alt+UpArrow', 'modifiers': ['Ctrl', 'Alt'], 'key': 'UpArrow'}` | TextLayerUI |
| Decrease Leading by 2 px | `{'raw': 'Alt+UpArrow', 'modifiers': ['Alt'], 'key': 'UpArrow'}` | TextLayerUI |
| Delete (Next Character) | `{'raw': 'FwdDel', 'modifiers': [], 'key': 'FwdDel'}` | TextLayerUI |
| Extend Selection 1 Char to Left (Horiz) or 1 Line to Left (Vert) | `{'raw': 'Shift+LeftArrow', 'modifiers': ['Shift'], 'key': 'LeftArrow'}` | TextLayerUI |
| Extend Selection 1 Char to Right (Horiz) or 1 Line to Right (Vert) | `{'raw': 'Shift+RightArrow', 'modifiers': ['Shift'], 'key': 'RightArrow'}` | TextLayerUI |
| Extend Selection 1 Line Down (Horiz) or 1 Char Down (Vert) | `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}` | TextLayerUI |
| Extend Selection 1 Line Up (Horiz) or 1 Char Up (Vert) | `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}` | TextLayerUI |
| Extend Selection 1 Word Down (Vert) | `{'raw': 'Shift+Ctrl+DownArrow', 'modifiers': ['Shift', 'Ctrl'], 'key': 'DownArrow'}` | TextLayerUI |
| Extend Selection 1 Word Up (Vert) | `{'raw': 'Shift+Ctrl+UpArrow', 'modifiers': ['Shift', 'Ctrl'], 'key': 'UpArrow'}` | TextLayerUI |
| Extend Selection 1 Word to Left (Horiz) | `{'raw': 'Shift+Ctrl+LeftArrow', 'modifiers': ['Shift', 'Ctrl'], 'key': 'LeftArrow'}` | TextLayerUI |
| Extend Selection 1 Word to Right (Horiz) | `{'raw': 'Shift+Ctrl+RightArrow', 'modifiers': ['Shift', 'Ctrl'], 'key': 'RightArrow'}` | TextLayerUI |
| Extend Selection to Beginning of Line | `{'raw': 'Shift+HOME', 'modifiers': ['Shift'], 'key': 'HOME'}` | TextLayerUI |
| Extend Selection to Beginning of Text Frame | `{'raw': 'Shift+Ctrl+HOME', 'modifiers': ['Shift', 'Ctrl'], 'key': 'HOME'}` | TextLayerUI |
| Extend Selection to End of Line | `{'raw': 'Shift+END', 'modifiers': ['Shift'], 'key': 'END'}` | TextLayerUI |
| Extend Selection to End of Text Frame | `{'raw': 'Shift+Ctrl+END', 'modifiers': ['Shift', 'Ctrl'], 'key': 'END'}` | TextLayerUI |
| Increase Baseline Shift by 10 px | `{'raw': 'Shift+Ctrl+Alt+UpArrow', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'UpArrow'}` | TextLayerUI |
| Increase Baseline Shift by 2 px | `{'raw': 'Shift+Alt+UpArrow', 'modifiers': ['Shift', 'Alt'], 'key': 'UpArrow'}` | TextLayerUI |
| Increase Font Size by 10 px | `{'raw': 'Shift+Ctrl+Alt+.', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': '.'}` | TextLayerUI |
| Increase Font Size by 2 px | `{'raw': 'Shift+Ctrl+.', 'modifiers': ['Shift', 'Ctrl'], 'key': '.'}` | TextLayerUI |
| Increase Kerning or Tracking by 100 px | `{'raw': 'Ctrl+Alt+RightArrow', 'modifiers': ['Ctrl', 'Alt'], 'key': 'RightArrow'}` | TextLayerUI |
| Increase Kerning or Tracking by 20 px | `{'raw': 'Alt+RightArrow', 'modifiers': ['Alt'], 'key': 'RightArrow'}` | TextLayerUI |
| Increase Leading by 10 px | `{'raw': 'Ctrl+Alt+DownArrow', 'modifiers': ['Ctrl', 'Alt'], 'key': 'DownArrow'}` | TextLayerUI |
| Increase Leading by 2 px | `{'raw': 'Alt+DownArrow', 'modifiers': ['Alt'], 'key': 'DownArrow'}` | TextLayerUI |
| Justify Paragraph; Force Last Line | `{'raw': 'Shift+Ctrl+F', 'modifiers': ['Shift', 'Ctrl'], 'key': 'F'}` | TextLayerUI |
| Justify Paragraph; Left Align Last Line | `{'raw': 'Shift+Ctrl+J', 'modifiers': ['Shift', 'Ctrl'], 'key': 'J'}` | TextLayerUI |
| Justify Paragraph; Right Align Last Line | `{'raw': 'Shift+Ctrl+Alt+J', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'J'}` | TextLayerUI |
| Move Insertion Point to Beginning of Line | `{'raw': 'HOME', 'modifiers': [], 'key': 'HOME'}` | TextLayerUI |
| Move Insertion Point to Beginning of Text Frame | `{'raw': 'Ctrl+HOME', 'modifiers': ['Ctrl'], 'key': 'HOME'}` | TextLayerUI |
| Move Insertion Point to End of Line | `{'raw': 'END', 'modifiers': [], 'key': 'END'}` | TextLayerUI |
| Move Insertion Point to End of Text Frame | `{'raw': 'Ctrl+END', 'modifiers': ['Ctrl'], 'key': 'END'}` | TextLayerUI |
| Move Insertion Point to Next Char (Horiz) or Line (Vert) | `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | TextLayerUI |
| Move Insertion Point to Next Line (Horiz) or Char (Vert) | `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}` | TextLayerUI |
| Move Insertion Point to Next Paragraph (Horiz) or Word (Vert) | `{'raw': 'Ctrl+DownArrow', 'modifiers': ['Ctrl'], 'key': 'DownArrow'}` | TextLayerUI |
| Move Insertion Point to Next Word (Horiz) or Paragraph (Vert) | `{'raw': 'Ctrl+RightArrow', 'modifiers': ['Ctrl'], 'key': 'RightArrow'}` | TextLayerUI |
| Move Insertion Point to Previous Char (Horiz) or Line (Vert) | `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | TextLayerUI |
| Move Insertion Point to Previous Line (Horiz) or Char (Vert) | `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}` | TextLayerUI |
| Move Insertion Point to Previous Paragraph (Horiz) or Word (Vert) | `{'raw': 'Ctrl+UpArrow', 'modifiers': ['Ctrl'], 'key': 'UpArrow'}` | TextLayerUI |
| Move Insertion Point to Previous Word (Horiz) or Paragraph (Vert) | `{'raw': 'Ctrl+LeftArrow', 'modifiers': ['Ctrl'], 'key': 'LeftArrow'}` | TextLayerUI |
| Reset Horizontal Scale of Selected Text | `{'raw': 'Shift+Ctrl+X', 'modifiers': ['Shift', 'Ctrl'], 'key': 'X'}` | TextLayerUI |
| Reset Tracking of Selected Text | `{'raw': 'Shift+Ctrl+Q', 'modifiers': ['Shift', 'Ctrl'], 'key': 'Q'}` | TextLayerUI |
| Reset Vertical Scale of Selected Text | `{'raw': 'Shift+Ctrl+Alt+X', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'X'}` | TextLayerUI |
| Toggle All Caps for Selected Text | `{'raw': 'Shift+Ctrl+K', 'modifiers': ['Shift', 'Ctrl'], 'key': 'K'}` | TextLayerUI |
| Toggle Composer for Selected Paragraphs | `{'raw': 'Shift+Ctrl+Alt+T', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'T'}` | TextLayerUI |
| Toggle Small Caps for Selected Text | `{'raw': 'Shift+Ctrl+Alt+K', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': 'K'}` | TextLayerUI |
| Toggle Subscript for Selected Text | `{'raw': 'Shift+Ctrl+Alt+=', 'modifiers': ['Shift', 'Ctrl', 'Alt'], 'key': '='}` | TextLayerUI |
| Toggle Superscript for Selected Text | `{'raw': 'Shift+Ctrl+=', 'modifiers': ['Shift', 'Ctrl'], 'key': '='}` | TextLayerUI |
| ~~~ | `{'raw': 'PadClear)(PageUP)(PageDOWN)(HELP)(Insert)(F1)(F2)(F3)(F4)(F5)(F6)(F7)(F8)(F9)(F10)(F11)(F12)(F13)(F14)(F15)(F16)(F17)(F18)(F19)(F20)(F21)(F22)(F23)(F24)(Shift+Alt+RightArrow)(Shift+Alt+LeftArrow)(Cmd+Shift+Alt+RightArrow)(Cmd+Shift+Alt+LeftArrow', 'modifiers': ['Alt', 'Alt', 'Shift', 'Alt', 'Shift', 'Alt'], 'key': 'PadClear)(PageUP)(PageDOWN)(HELP)(Insert)(F1)(F2)(F3)(F4)(F5)(F6)(F7)(F8)(F9)(F10)(F11)(F12)(F13)(F14)(F15)(F16)(F17)(F18)(F19)(F20)(F21)(F22)(F23)(F24)(Shift+RightArrow)(Shift+LeftArrow)(Cmd+RightArrow)(Cmd+LeftArrow', 'numeric_keypad': True}` | TextLayerUI |

**Time** (24 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Go Back 1 Frame | `{'raw': 'Ctrl+LeftArrow)(PageUP', 'modifiers': ['Ctrl'], 'key': 'LeftArrow)(PageUP'}`; `{'raw': 'Ctrl+LeftArrow)(PadPageUp)(PageUP', 'modifiers': ['Ctrl'], 'key': 'LeftArrow)(PadPageUp)(PageUP'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CCompTime, CDirItemTabPanelTime, CPanoProjFootage |
| Go Back 10 Frames | `{'raw': 'Shift+PageUP)(Ctrl+Shift+LeftArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageUP)(Ctrl+LeftArrow'}`; `{'raw': 'Shift+PageUP)(Ctrl+Shift+LeftArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageUP)(Ctrl+LeftArrow'}` | CCompTime, CDirItemTabPanelTime |
| Go Forward 1 Frame | `{'raw': 'Ctrl+RightArrow)(PageDOWN', 'modifiers': ['Ctrl'], 'key': 'RightArrow)(PageDOWN'}`; `{'raw': 'Ctrl+RightArrow)(PadPageDown)(PageDOWN', 'modifiers': ['Ctrl'], 'key': 'RightArrow)(PadPageDown)(PageDOWN'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CCompTime, CDirItemTabPanelTime, CPanoProjFootage |
| Go Forward 10 Frames | `{'raw': 'Shift+PageDOWN)(Ctrl+Shift+RightArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageDOWN)(Ctrl+RightArrow'}`; `{'raw': 'Shift+PageDOWN)(Ctrl+Shift+RightArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'PageDOWN)(Ctrl+RightArrow'}` | CCompTime, CDirItemTabPanelTime |
| Go to Earliest In Point of Selected Layers | `{'raw': 'I', 'modifiers': [], 'key': 'I'}`; `{'raw': 'I', 'modifiers': [], 'key': 'I'}` | CCompTime, CPanoProjFootage |
| Go to End of Composition, Layer, or Footage Item | `{'raw': 'Ctrl+Alt+RightArrow)(END', 'modifiers': ['Ctrl', 'Alt'], 'key': 'RightArrow)(END'}`; `{'raw': 'Ctrl+Alt+RightArrow)(PadEnd)(END', 'modifiers': ['Ctrl', 'Alt'], 'key': 'RightArrow)(PadEnd)(END'}` | CCompTime, CDirItemTabPanelTime |
| Go to End of Work Area | `{'raw': 'Shift+END', 'modifiers': ['Shift'], 'key': 'END'}` | CCompTime |
| Go to Latest Out Point of Selected Layers | `{'raw': 'O', 'modifiers': [], 'key': 'O'}`; `{'raw': 'O', 'modifiers': [], 'key': 'O'}` | CCompTime, CPanoProjFootage |
| Go to Next Layer In or Out Point | `{'raw': 'Ctrl+Alt+Shift+RightArrow', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'RightArrow'}` | CCompTime |
| Go to Previous Layer In or Out Point | `{'raw': 'Ctrl+Alt+Shift+LeftArrow', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'LeftArrow'}` | CCompTime |
| Go to Start of Composition, Layer, or Footage Item | `{'raw': 'Ctrl+Alt+LeftArrow)(HOME', 'modifiers': ['Ctrl', 'Alt'], 'key': 'LeftArrow)(HOME'}`; `{'raw': 'Ctrl+Alt+LeftArrow)(PadHome)(HOME', 'modifiers': ['Ctrl', 'Alt'], 'key': 'LeftArrow)(PadHome)(HOME'}` | CCompTime, CDirItemTabPanelTime |
| Go to Start of Work Area | `{'raw': 'Shift+HOME', 'modifiers': ['Shift'], 'key': 'HOME'}` | CCompTime |
| Preview | `{'raw': 'Space', 'modifiers': [], 'key': 'Space'}`; `{'raw': 'Space', 'modifiers': [], 'key': 'Space'}` | CCompTime, CDirItemTabPanelTime |
| Preview (Time Shift) | `{'raw': 'Shift+Space', 'modifiers': ['Shift'], 'key': 'Space'}`; `{'raw': 'Shift+Space', 'modifiers': ['Shift'], 'key': 'Space'}` | CCompTime, CDirItemTabPanelTime |
| Set Layer In (Relative to Selection)#{comment}DVAAE-4229356 | `{'raw': 'Shift+[', 'modifiers': ['Shift'], 'key': '['}`; `{'raw': 'Shift+[', 'modifiers': ['Shift'], 'key': '['}` | CCompTime, TLOutlinePano |
| Set Layer In Point at Current Time | `{'raw': '[', 'modifiers': [], 'key': '['}`; `{'raw': '[', 'modifiers': [], 'key': '['}`; `{'raw': '[', 'modifiers': [], 'key': '['}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Set Layer Out (Relative to Selection)#{comment}DVAAE-4229356 | `{'raw': 'Shift+]', 'modifiers': ['Shift'], 'key': ']'}`; `{'raw': 'Shift+]', 'modifiers': ['Shift'], 'key': ']'}` | CCompTime, TLOutlinePano |
| Set Layer Out Point at Current Time | `{'raw': ']', 'modifiers': [], 'key': ']'}`; `{'raw': ']', 'modifiers': [], 'key': ']'}`; `{'raw': ']', 'modifiers': [], 'key': ']'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Stretch Layer In Point to Current Time | `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Stretch Layer Out Point to Current Time | `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Trim Layer In (Relative to Selection)#{comment}DVAAE-4229356 | `{'raw': 'Alt+Shift+[', 'modifiers': ['Alt', 'Shift'], 'key': '['}`; `{'raw': 'Alt+Shift+[', 'modifiers': ['Alt', 'Shift'], 'key': '['}` | CCompTime, TLOutlinePano |
| Trim Layer In Point to Current Time | `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |
| Trim Layer Out (Relative to Selection)#{comment}DVAAE-4229356 | `{'raw': 'Alt+Shift+]', 'modifiers': ['Alt', 'Shift'], 'key': ']'}`; `{'raw': 'Alt+Shift+]', 'modifiers': ['Alt', 'Shift'], 'key': ']'}` | CCompTime, TLOutlinePano |
| Trim Layer Out Point to Current Time | `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |

**TimelineLayerProperties** (105 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Add and Reveal a New Anchor Point Keyframe | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add and Reveal a New Audio Levels Keyframe | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add and Reveal a New Mask Feather Keyframe | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add and Reveal a New Mask Shape Keyframe | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add and Reveal a New Opacity Keyframe | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add and Reveal a New Position Keyframe | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add and Reveal a New Rotation Keyframe | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add and Reveal a New Scale Keyframe. | `{'raw': '', 'unbound': True}` | CCompCompCmd |
| Add/Delete Anchor Point or Position of Interest Keyframe at Current Time | `{'raw': 'Alt+Shift+A', 'modifiers': ['Alt', 'Shift'], 'key': 'A'}` | CCompCompCmd |
| Add/Delete Audio Levels Keyframe at Current Time | `{'raw': 'Alt+Shift+L', 'modifiers': ['Alt', 'Shift'], 'key': 'L'}` | CCompCompCmd |
| Add/Delete Mask Feather Keyframe at Current Time | `{'raw': 'Alt+Shift+F', 'modifiers': ['Alt', 'Shift'], 'key': 'F'}` | CCompCompCmd |
| Add/Delete Mask Shape Keyframe at Current Time | `{'raw': 'Alt+Shift+M', 'modifiers': ['Alt', 'Shift'], 'key': 'M'}` | CCompCompCmd |
| Add/Delete Opacity Keyframe at Current Time | `{'raw': 'Alt+Shift+T', 'modifiers': ['Alt', 'Shift'], 'key': 'T'}` | CCompCompCmd |
| Add/Delete Position Keyframe at Current Time | `{'raw': 'Alt+Shift+P', 'modifiers': ['Alt', 'Shift'], 'key': 'P'}` | CCompCompCmd |
| Add/Delete Rotation or Orientation Keyframe at Current Time | `{'raw': 'Alt+Shift+R', 'modifiers': ['Alt', 'Shift'], 'key': 'R'}` | CCompCompCmd |
| Add/Delete Scale Keyframe at Current Time | `{'raw': 'Alt+Shift+S', 'modifiers': ['Alt', 'Shift'], 'key': 'S'}` | CCompCompCmd |
| Align Selected Layers' In Points to Start of Composition | `{'raw': 'Alt+HOME', 'modifiers': ['Alt'], 'key': 'HOME'}` | CCompCompCmd |
| Align Selected Layers' Out Points to End of Composition | `{'raw': 'Alt+END', 'modifiers': ['Alt'], 'key': 'END'}` | CCompCompCmd |
| Decrease Layer Opacity by 1% | `{'raw': 'Ctrl+Alt+PadMinus', 'modifiers': ['Ctrl', 'Alt'], 'key': 'PadMinus', 'numeric_keypad': True}`; `{'raw': 'Ctrl+Alt+PadMinus', 'modifiers': ['Ctrl', 'Alt'], 'key': 'PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Decrease Layer Opacity by 10% | `{'raw': 'Ctrl+Alt+Shift+PadMinus', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'PadMinus', 'numeric_keypad': True}`; `{'raw': 'Ctrl+Alt+Shift+PadMinus', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Go to Next Keyframe | `{'raw': 'K', 'modifiers': [], 'key': 'K'}` | CCompCompCmd |
| Go to Next Keyframe on Selected Layers/Properties | `{'raw': 'Shift+K', 'modifiers': ['Shift'], 'key': 'K'}` | CCompCompCmd |
| Go to Previous Keyframe | `{'raw': 'J', 'modifiers': [], 'key': 'J'}` | CCompCompCmd |
| Go to Previous Keyframe on Selected Layers/Properties | `{'raw': 'Shift+J', 'modifiers': ['Shift'], 'key': 'J'}` | CCompCompCmd |
| Increase Layer Opacity by 1% | `{'raw': 'Ctrl+Alt+PadPlus', 'modifiers': ['Ctrl', 'Alt'], 'key': 'PadPlus', 'numeric_keypad': True}`; `{'raw': 'Ctrl+Alt+PadPlus', 'modifiers': ['Ctrl', 'Alt'], 'key': 'PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Increase Layer Opacity by 10% | `{'raw': 'Ctrl+Alt+Shift+PadPlus', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'PadPlus', 'numeric_keypad': True}`; `{'raw': 'Ctrl+Alt+Shift+PadPlus', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 1 Screen Pixel Down | `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow)(Alt+DownArrow', 'modifiers': [], 'key': 'DownArrow)(Alt+DownArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 1 Screen Pixel Up | `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow)(Alt+UpArrow', 'modifiers': [], 'key': 'UpArrow)(Alt+UpArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 1 Screen Pixel to the Left | `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}`; `{'raw': 'LeftArrow', 'modifiers': [], 'key': 'LeftArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 1 Screen Pixel to the Right | `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}`; `{'raw': 'RightArrow', 'modifiers': [], 'key': 'RightArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels Down | `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow)(Alt+Shift+DownArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'DownArrow)(Alt+DownArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels Up | `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow)(Alt+Shift+UpArrow', 'modifiers': ['Shift', 'Shift'], 'key': 'UpArrow)(Alt+UpArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels to the Left | `{'raw': 'Shift+LeftArrow', 'modifiers': ['Shift'], 'key': 'LeftArrow'}`; `{'raw': 'Shift+LeftArrow', 'modifiers': ['Shift'], 'key': 'LeftArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Move Layer 10 Screen Pixels to the Right | `{'raw': 'Shift+RightArrow', 'modifiers': ['Shift'], 'key': 'RightArrow'}`; `{'raw': 'Shift+RightArrow', 'modifiers': ['Shift'], 'key': 'RightArrow'}` | CCompCompCmd, CPanoProjLayerPano |
| Open Composition | `{'raw': 'Backslash', 'modifiers': [], 'key': 'Backslash'}` | CCompCompCmd |
| Rotate Layer 1 Degree Clockwise | `{'raw': 'PadPlus', 'modifiers': [], 'key': 'PadPlus', 'numeric_keypad': True}`; `{'raw': 'PadPlus', 'modifiers': [], 'key': 'PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Rotate Layer 1 Degree Counterclockwise | `{'raw': 'PadMinus', 'modifiers': [], 'key': 'PadMinus', 'numeric_keypad': True}`; `{'raw': 'PadMinus', 'modifiers': [], 'key': 'PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Rotate Layer 10 Degrees Clockwise | `{'raw': 'Shift+PadPlus', 'modifiers': ['Shift'], 'key': 'PadPlus', 'numeric_keypad': True}`; `{'raw': 'Shift+PadPlus', 'modifiers': ['Shift'], 'key': 'PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Rotate Layer 10 Degrees Counterclockwise | `{'raw': 'Shift+PadMinus', 'modifiers': ['Shift'], 'key': 'PadMinus', 'numeric_keypad': True}`; `{'raw': 'Shift+PadMinus', 'modifiers': ['Shift'], 'key': 'PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 1% Larger | `{'raw': 'Alt+PadPlus)(Ctrl+PadPlus', 'modifiers': ['Alt'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}`; `{'raw': 'Alt+PadPlus)(Ctrl+PadPlus', 'modifiers': ['Alt'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 1% Smaller | `{'raw': 'Alt+PadMinus)(Ctrl+PadMinus', 'modifiers': ['Alt'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}`; `{'raw': 'Alt+PadMinus)(Ctrl+PadMinus', 'modifiers': ['Alt'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 10% Larger | `{'raw': 'Alt+Shift+PadPlus)(Ctrl+Shift+PadPlus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}`; `{'raw': 'Alt+Shift+PadPlus)(Ctrl+Shift+PadPlus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadPlus)(Ctrl+PadPlus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scale Layer 10% Smaller | `{'raw': 'Alt+Shift+PadMinus)(Ctrl+Shift+PadMinus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}`; `{'raw': 'Alt+Shift+PadMinus)(Ctrl+Shift+PadMinus', 'modifiers': ['Alt', 'Shift', 'Shift'], 'key': 'PadMinus)(Ctrl+PadMinus', 'numeric_keypad': True}` | CCompCompCmd, CPanoProjLayerPano |
| Scroll Selected Layer to Top | `{'raw': 'X', 'modifiers': [], 'key': 'X'}` | CCompCompCmd |
| Scroll to Current Time | `{'raw': 'D', 'modifiers': [], 'key': 'D'}` | CCompCompCmd |
| Select All Visible Keyframes and Exposed Properties | `{'raw': 'Ctrl+Alt+A', 'modifiers': ['Ctrl', 'Alt'], 'key': 'A'}` | CCompCompCmd |
| Select Layer by Number: 1 | `{'raw': 'Pad1', 'modifiers': [], 'key': 'Pad1', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 2 | `{'raw': 'Pad2', 'modifiers': [], 'key': 'Pad2', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 3 | `{'raw': 'Pad3', 'modifiers': [], 'key': 'Pad3', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 4 | `{'raw': 'Pad4', 'modifiers': [], 'key': 'Pad4', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 5 | `{'raw': 'Pad5', 'modifiers': [], 'key': 'Pad5', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 6 | `{'raw': 'Pad6', 'modifiers': [], 'key': 'Pad6', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 7 | `{'raw': 'Pad7', 'modifiers': [], 'key': 'Pad7', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 8 | `{'raw': 'Pad8', 'modifiers': [], 'key': 'Pad8', 'numeric_keypad': True}` | CCompCompCmd |
| Select Layer by Number: 9 | `{'raw': 'Pad9', 'modifiers': [], 'key': 'Pad9', 'numeric_keypad': True}` | CCompCompCmd |
| Shift Layer 1 Frame Earlier | `{'raw': 'Alt+PageUP', 'modifiers': ['Alt'], 'key': 'PageUP'}` | CCompCompCmd |
| Shift Layer 1 Frame Later | `{'raw': 'Alt+PageDOWN', 'modifiers': ['Alt'], 'key': 'PageDOWN'}` | CCompCompCmd |
| Shift Layer 10 Frames Earlier | `{'raw': 'Alt+Shift+PageUP', 'modifiers': ['Alt', 'Shift'], 'key': 'PageUP'}` | CCompCompCmd |
| Shift Layer 10 Frames Later | `{'raw': 'Alt+Shift+PageDOWN', 'modifiers': ['Alt', 'Shift'], 'key': 'PageDOWN'}` | CCompCompCmd |
| Shift Selected Keyframes 1 Frame Earlier | `{'raw': 'Alt+LeftArrow', 'modifiers': ['Alt'], 'key': 'LeftArrow'}` | CCompCompCmd |
| Shift Selected Keyframes 1 Frame Later | `{'raw': 'Alt+RightArrow', 'modifiers': ['Alt'], 'key': 'RightArrow'}` | CCompCompCmd |
| Shift Selected Keyframes 10 Frames Earlier | `{'raw': 'Alt+Shift+LeftArrow', 'modifiers': ['Alt', 'Shift'], 'key': 'LeftArrow'}` | CCompCompCmd |
| Shift Selected Keyframes 10 Frames Later | `{'raw': 'Alt+Shift+RightArrow', 'modifiers': ['Alt', 'Shift'], 'key': 'RightArrow'}` | CCompCompCmd |
| Show/Hide All of the Selected Layers' Properties | `{'raw': 'Ctrl+`', 'modifiers': ['Ctrl'], 'key': '`'}`; `{'raw': 'Ctrl+`', 'modifiers': ['Ctrl'], 'key': '`'}` | CCompCompCmd, CTopic |
| Show/Hide Anchor Point or Position of Interest | `{'raw': 'A', 'modifiers': [], 'key': 'A'}` | CCompCompCmd |
| Show/Hide Anchor Point or Position of Interest (Extend) | `{'raw': 'Shift+A', 'modifiers': ['Shift'], 'key': 'A'}` | CCompCompCmd |
| Show/Hide Audio Levels | `{'raw': 'L', 'modifiers': [], 'key': 'L'}` | CCompCompCmd |
| Show/Hide Audio Levels (Extend) | `{'raw': 'Shift+L', 'modifiers': ['Shift'], 'key': 'L'}` | CCompCompCmd |
| Show/Hide Effects (Double-Tap: Expressions) | `{'raw': 'E', 'modifiers': [], 'key': 'E'}` | CCompCompCmd |
| Show/Hide Effects (Extend; Double-Tap: Expressions) | `{'raw': 'Shift+E', 'modifiers': ['Shift'], 'key': 'E'}` | CCompCompCmd |
| Show/Hide Graph Editor | `{'raw': 'Shift+F3', 'modifiers': ['Shift'], 'key': 'F3'}`; `{'raw': 'Shift+F3', 'modifiers': ['Shift'], 'key': 'F3'}` | CCompCompCmd, TLOutlinePano |
| Show/Hide Mask Feather (Double-Tap) | `{'raw': 'F', 'modifiers': [], 'key': 'F'}` | CCompCompCmd |
| Show/Hide Mask Feather (Extend; Double-Tap) | `{'raw': 'Shift+F', 'modifiers': ['Shift'], 'key': 'F'}` | CCompCompCmd |
| Show/Hide Mask Shape (Double-Tap) | `{'raw': 'M', 'modifiers': [], 'key': 'M'}` | CCompCompCmd |
| Show/Hide Mask Shape (Extend; Double-Tap) | `{'raw': 'Shift+M', 'modifiers': ['Shift'], 'key': 'M'}` | CCompCompCmd |
| Show/Hide Opacity | `{'raw': 'T', 'modifiers': [], 'key': 'T'}` | CCompCompCmd |
| Show/Hide Opacity (Extend) | `{'raw': 'Shift+T', 'modifiers': ['Shift'], 'key': 'T'}` | CCompCompCmd |
| Show/Hide Parent Column | `{'raw': 'Shift+F4', 'modifiers': ['Shift'], 'key': 'F4'}` | CCompCompCmd |
| Show/Hide Position | `{'raw': 'P', 'modifiers': [], 'key': 'P'}` | CCompCompCmd |
| Show/Hide Position (Extend) | `{'raw': 'Shift+P', 'modifiers': ['Shift'], 'key': 'P'}` | CCompCompCmd |
| Show/Hide Properties with Animation (Double-Tap) | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | CCompCompCmd, CSwitchboard |
| Show/Hide Properties with Animation (Extend; Double-Tap) | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | CCompCompCmd, CSwitchboard |
| Show/Hide Properties with Keyframes (Double-Tap) | `{'raw': 'U', 'modifiers': [], 'key': 'U'}`; `{'raw': 'U', 'modifiers': [], 'key': 'U'}` | CCompCompCmd, CSwitchboard |
| Show/Hide Properties with Keyframes (Extend; Double-Tap) | `{'raw': 'Shift+U', 'modifiers': ['Shift'], 'key': 'U'}`; `{'raw': 'Shift+U', 'modifiers': ['Shift'], 'key': 'U'}` | CCompCompCmd, CSwitchboard |
| Show/Hide Rotation and Orientation Properties | `{'raw': 'R', 'modifiers': [], 'key': 'R'}` | CCompCompCmd |
| Show/Hide Rotation and Orientation Properties (Extend) | `{'raw': 'Shift+R', 'modifiers': ['Shift'], 'key': 'R'}` | CCompCompCmd |
| Show/Hide Scale | `{'raw': 'S', 'modifiers': [], 'key': 'S'}` | CCompCompCmd |
| Show/Hide Scale (Extend) | `{'raw': 'Shift+S', 'modifiers': ['Shift'], 'key': 'S'}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 1 | `{'raw': 'Shift+Pad1', 'modifiers': ['Shift'], 'key': 'Pad1', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 2 | `{'raw': 'Shift+Pad2', 'modifiers': ['Shift'], 'key': 'Pad2', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 3 | `{'raw': 'Shift+Pad3', 'modifiers': ['Shift'], 'key': 'Pad3', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 4 | `{'raw': 'Shift+Pad4', 'modifiers': ['Shift'], 'key': 'Pad4', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 5 | `{'raw': 'Shift+Pad5', 'modifiers': ['Shift'], 'key': 'Pad5', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 6 | `{'raw': 'Shift+Pad6', 'modifiers': ['Shift'], 'key': 'Pad6', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 7 | `{'raw': 'Shift+Pad7', 'modifiers': ['Shift'], 'key': 'Pad7', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 8 | `{'raw': 'Shift+Pad8', 'modifiers': ['Shift'], 'key': 'Pad8', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Selection of Layer by Number: 9 | `{'raw': 'Shift+Pad9', 'modifiers': ['Shift'], 'key': 'Pad9', 'numeric_keypad': True}` | CCompCompCmd |
| Toggle Zoom Level | `{'raw': 'Shift+;', 'modifiers': ['Shift'], 'key': ';'}`; `{'raw': 'Shift+;', 'modifiers': ['Shift'], 'key': ';'}` | CCompCompCmd, TLOutlinePano |
| Twirl | `{'raw': '', 'unbound': True}`; `{'raw': '', 'unbound': True}` | CCompCompCmd, CTopic |
| TwirlPreserveSolo | `{'raw': 'Shift+`', 'modifiers': ['Shift'], 'key': '`'}`; `{'raw': 'Shift+`', 'modifiers': ['Shift'], 'key': '`'}` | CCompCompCmd, CTopic |
| TwirlPreserveSoloExplode | `{'raw': 'Ctrl+Shift+`', 'modifiers': ['Ctrl', 'Shift'], 'key': '`'}`; `{'raw': 'Ctrl+Shift+`', 'modifiers': ['Ctrl', 'Shift'], 'key': '`'}` | CCompCompCmd, CTopic |
| Zoom In | `{'raw': '=', 'modifiers': [], 'key': '='}`; `{'raw': '=', 'modifiers': [], 'key': '='}` | CCompCompCmd, TLOutlinePano |
| Zoom In to Frame Intervals | `{'raw': ';', 'modifiers': [], 'key': ';'}`; `{'raw': ';', 'modifiers': [], 'key': ';'}` | CCompCompCmd, TLOutlinePano |
| Zoom Out | `{'raw': '-', 'modifiers': [], 'key': '-'}`; `{'raw': '-', 'modifiers': [], 'key': '-'}` | CCompCompCmd, TLOutlinePano |
| Zoom to Work Area | `{'raw': 'Alt+;', 'modifiers': ['Alt'], 'key': ';'}`; `{'raw': 'Alt+;', 'modifiers': ['Alt'], 'key': ';'}` | CCompCompCmd, TLOutlinePano |

**TimelineLayers** (21 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Add Layer Marker | `{'raw': 'PadMultiply)(macControl+8', 'modifiers': [], 'key': 'PadMultiply)(macControl+8', 'numeric_keypad': True}` | CCompCmd |
| Add Layer Marker Using Dialog Box | `{'raw': 'Alt+PadMultiply)(Alt+macControl+8', 'modifiers': ['Alt'], 'key': 'PadMultiply)(Alt+macControl+8', 'numeric_keypad': True}` | CCompCmd |
| Center Anchor Point in Layer Content | `{'raw': 'Ctrl+Alt+HOME', 'modifiers': ['Ctrl', 'Alt'], 'key': 'HOME'}` | CCompCmd |
| Center Layer in View | `{'raw': 'Ctrl+HOME', 'modifiers': ['Ctrl'], 'key': 'HOME'}` | CCompCmd |
| Clear | `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}`; `{'raw': 'PadClear', 'modifiers': [], 'key': 'PadClear', 'numeric_keypad': True}`; `{'raw': 'Delete)(FwdDel', 'modifiers': [], 'key': 'Delete)(FwdDel'}` | CCompCmd, COutline, CPanoProjLayer, CSwitchboardModal, FloPano |
| ClearMask | `{'raw': 'Alt+Delete)(Alt+FwdDel', 'modifiers': ['Alt'], 'key': 'Delete)(Alt+FwdDel'}`; `{'raw': 'Alt+Delete)(Alt+FwdDel', 'modifiers': ['Alt'], 'key': 'Delete)(Alt+FwdDel'}` | CCompCmd, CPanoProjLayer |
| Extend Selection to Next Layer | `{'raw': 'Ctrl+Shift+DownArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}`; `{'raw': 'Shift+DownArrow', 'modifiers': ['Shift'], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Extend Selection to Previous Layer | `{'raw': 'Ctrl+Shift+UpArrow', 'modifiers': ['Ctrl', 'Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}`; `{'raw': 'Shift+UpArrow', 'modifiers': ['Shift'], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Fit Layer to Comp | `{'raw': 'Ctrl+Alt+F', 'modifiers': ['Ctrl', 'Alt'], 'key': 'F'}` | CCompCmd |
| Fit Layer to Comp Height | `{'raw': 'Ctrl+Alt+Shift+G', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'G'}` | CCompCmd |
| Fit Layer to Comp Width | `{'raw': 'Ctrl+Alt+Shift+H', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'H'}` | CCompCmd |
| Flip Horizontal | `{'raw': '', 'unbound': True}` | CCompCmd |
| Flip Vertical | `{'raw': '', 'unbound': True}` | CCompCmd |
| Open Layer Source in Layer Panel | `{'raw': 'Enter)(Return', 'modifiers': [], 'key': 'Enter)(Return'}` | CCompCmd |
| Paste Layers at Current Time | `{'raw': 'Ctrl+Alt+V', 'modifiers': ['Ctrl', 'Alt'], 'key': 'V'}` | CCompCmd |
| Reverse Layer | `{'raw': 'Ctrl+Alt+R', 'modifiers': ['Ctrl', 'Alt'], 'key': 'R'}` | CCompCmd |
| Select Next Layer | `{'raw': 'Ctrl+DownArrow', 'modifiers': ['Ctrl'], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}`; `{'raw': 'DownArrow', 'modifiers': [], 'key': 'DownArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Select Previous Layer | `{'raw': 'Ctrl+UpArrow', 'modifiers': ['Ctrl'], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}`; `{'raw': 'UpArrow', 'modifiers': [], 'key': 'UpArrow'}` | CCompCmd, CPanoECOutline, POutlinePano, RQOutlinePano |
| Set Work Area End | `{'raw': 'N', 'modifiers': [], 'key': 'N'}` | CCompCmd |
| Set Work Area Start | `{'raw': 'B', 'modifiers': [], 'key': 'B'}` | CCompCmd |
| Set Work Area to Selected Layers | `{'raw': 'Ctrl+Alt+B', 'modifiers': ['Ctrl', 'Alt'], 'key': 'B'}` | CCompCmd |

**TimelineNavigation** (16 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Set Layer In (Relative to Selection) | `{'raw': 'Shift+[', 'modifiers': ['Shift'], 'key': '['}`; `{'raw': 'Shift+[', 'modifiers': ['Shift'], 'key': '['}` | CCompTime, TLOutlinePano |
| Set Layer In Point at Current Time | `{'raw': '[', 'modifiers': [], 'key': '['}`; `{'raw': '[', 'modifiers': [], 'key': '['}`; `{'raw': '[', 'modifiers': [], 'key': '['}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Set Layer Out (Relative to Selection) | `{'raw': 'Shift+]', 'modifiers': ['Shift'], 'key': ']'}`; `{'raw': 'Shift+]', 'modifiers': ['Shift'], 'key': ']'}` | CCompTime, TLOutlinePano |
| Set Layer Out Point at Current Time | `{'raw': ']', 'modifiers': [], 'key': ']'}`; `{'raw': ']', 'modifiers': [], 'key': ']'}`; `{'raw': ']', 'modifiers': [], 'key': ']'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Show/Hide Graph Editor | `{'raw': 'Shift+F3', 'modifiers': ['Shift'], 'key': 'F3'}`; `{'raw': 'Shift+F3', 'modifiers': ['Shift'], 'key': 'F3'}` | CCompCompCmd, TLOutlinePano |
| Stretch Layer In Point to Current Time | `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Shift+Comma', 'modifiers': ['Ctrl', 'Shift'], 'key': 'Comma'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Stretch Layer Out Point to Current Time | `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}`; `{'raw': 'Ctrl+Alt+Comma', 'modifiers': ['Ctrl', 'Alt'], 'key': 'Comma'}` | CCompTime, CPanoProjLayer, TLOutlinePano |
| Toggle Zoom Level | `{'raw': 'Shift+;', 'modifiers': ['Shift'], 'key': ';'}`; `{'raw': 'Shift+;', 'modifiers': ['Shift'], 'key': ';'}` | CCompCompCmd, TLOutlinePano |
| Trim Layer In (Relative to Selection) | `{'raw': 'Alt+Shift+[', 'modifiers': ['Alt', 'Shift'], 'key': '['}`; `{'raw': 'Alt+Shift+[', 'modifiers': ['Alt', 'Shift'], 'key': '['}` | CCompTime, TLOutlinePano |
| Trim Layer In Point to Current Time | `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}`; `{'raw': 'Alt+[', 'modifiers': ['Alt'], 'key': '['}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |
| Trim Layer Out (Relative to Selection) | `{'raw': 'Alt+Shift+]', 'modifiers': ['Alt', 'Shift'], 'key': ']'}`; `{'raw': 'Alt+Shift+]', 'modifiers': ['Alt', 'Shift'], 'key': ']'}` | CCompTime, TLOutlinePano |
| Trim Layer Out Point to Current Time | `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}`; `{'raw': 'Alt+]', 'modifiers': ['Alt'], 'key': ']'}` | CCompTime, CPanoProjFootage, CPanoProjLayer, TLOutlinePano |
| Zoom In to Current Time | `{'raw': '=', 'modifiers': [], 'key': '='}`; `{'raw': '=', 'modifiers': [], 'key': '='}` | CCompCompCmd, TLOutlinePano |
| Zoom In to Frame Intervals | `{'raw': ';', 'modifiers': [], 'key': ';'}`; `{'raw': ';', 'modifiers': [], 'key': ';'}` | CCompCompCmd, TLOutlinePano |
| Zoom Out from Current Time | `{'raw': '-', 'modifiers': [], 'key': '-'}`; `{'raw': '-', 'modifiers': [], 'key': '-'}` | CCompCompCmd, TLOutlinePano |
| Zoom to Work Area | `{'raw': 'Alt+;', 'modifiers': ['Alt'], 'key': ';'}`; `{'raw': 'Alt+;', 'modifiers': ['Alt'], 'key': ';'}` | CCompCompCmd, TLOutlinePano |

**Tools** (27 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| 3D Position Gizmo | `{'raw': '4', 'modifiers': [], 'key': '4'}` | CEggAppTool |
| 3D Rotation Gizmo | `{'raw': '6', 'modifiers': [], 'key': '6'}` | CEggAppTool |
| 3D Scale Gizmo | `{'raw': '5', 'modifiers': [], 'key': '5'}` | CEggAppTool |
| 3D Wireframe Gizmo | `{'raw': 'Ctrl+Shift+1', 'modifiers': ['Ctrl', 'Shift'], 'key': '1'}` | CEggAppTool |
| Camera (Cycle Forward) | `{'raw': 'C', 'modifiers': [], 'key': 'C'}` | CEggAppTool |
| Camera (Cycle Options Forward) | `{'raw': 'Shift+C', 'modifiers': ['Shift'], 'key': 'C'}` | CEggAppTool |
| Cycle 3D Gizmo Mode Forward | -- | -- |
| Dolly Camera | `{'raw': '3', 'modifiers': [], 'key': '3'}` | CEggAppTool |
| Dolly Camera (Cycle Options Forward) | `{'raw': 'Shift+3', 'modifiers': ['Shift'], 'key': '3'}` | CEggAppTool |
| Hand | `{'raw': 'H', 'modifiers': [], 'key': 'H'}` | CEggAppTool |
| Object Matte | `{'raw': 'Alt+W', 'modifiers': ['Alt'], 'key': 'W'}` | CEggAppTool |
| Orbit Camera | `{'raw': '1', 'modifiers': [], 'key': '1'}` | CEggAppTool |
| Orbit Camera (Cycle Options Forward) | `{'raw': 'Shift+1', 'modifiers': ['Shift'], 'key': '1'}` | CEggAppTool |
| Paint (Brush/Clone/Eraser) | `{'raw': 'Ctrl+B', 'modifiers': ['Ctrl'], 'key': 'B'}` | CEggAppTool |
| Pan Behind (Anchor Point) | `{'raw': 'Y', 'modifiers': [], 'key': 'Y'}` | CEggAppTool |
| Pan Camera | `{'raw': '2', 'modifiers': [], 'key': '2'}` | CEggAppTool |
| Pan Camera (Cycle Options Forward) | `{'raw': 'Shift+2', 'modifiers': ['Shift'], 'key': '2'}` | CEggAppTool |
| Parametric Mesh | `{'raw': 'Shift+D', 'modifiers': ['Shift'], 'key': 'D'}` | CEggAppTool |
| Pen / Mask Feather (Cycle Backward) | `{'raw': 'Shift+G', 'modifiers': ['Shift'], 'key': 'G'}` | CEggAppTool |
| Pen / Mask Feather (Cycle Forward) | `{'raw': 'G', 'modifiers': [], 'key': 'G'}` | CEggAppTool |
| Puppet (Pin/Overlap/Starch) | `{'raw': 'Ctrl+P', 'modifiers': ['Ctrl'], 'key': 'P'}` | CEggAppTool |
| Rotate | `{'raw': 'W', 'modifiers': [], 'key': 'W'}` | CEggAppTool |
| Selection | `{'raw': 'V', 'modifiers': [], 'key': 'V'}` | CEggAppTool |
| Shape / Mask (Cycle Backward) | `{'raw': 'Shift+Q', 'modifiers': ['Shift'], 'key': 'Q'}` | CEggAppTool |
| Shape / Mask (Cycle Forward) | `{'raw': 'Q', 'modifiers': [], 'key': 'Q'}` | CEggAppTool |
| Text | `{'raw': 'Ctrl+T', 'modifiers': ['Ctrl'], 'key': 'T'}` | CEggAppTool |
| Zoom | `{'raw': 'Z', 'modifiers': [], 'key': 'Z'}` | CEggAppTool |

**ToolsUnifiedCamera** (2 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| Move Camera to Look At All Layers | `{'raw': '', 'unbound': True}`; `{'raw': 'Ctrl+Shift+F', 'modifiers': ['Ctrl', 'Shift'], 'key': 'F'}` | CSwitchboard, CameraToolUI |
| Move Camera to Look At Selected Layers | `{'raw': 'Ctrl+Alt+Shift+Backslash', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'Backslash'}`; `{'raw': 'F', 'modifiers': [], 'key': 'F'}` | CSwitchboard, CameraToolUI |

**ViewerPanel** (3 commands)

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Command | Default binding (Windows) | Context |
|---|---|---|
| New Viewer | `{'raw': 'Alt+Shift+N', 'modifiers': ['Alt', 'Shift'], 'key': 'N'}` | CDirTabPanel |
| Show/Hide Composition Mini-Flowchart | `{'raw': 'Tab', 'modifiers': [], 'key': 'Tab'}` | CDirTabPanel |
| Split New Viewer and Toggle Lock#{comment}DVAAE-4231888 | `{'raw': 'Ctrl+Alt+Shift+N', 'modifiers': ['Ctrl', 'Alt', 'Shift'], 'key': 'N'}` | CDirTabPanel |
---

## 14.26.12 Declared gaps

**[STU-MOT-140] GAP -- the interpolation code mapping is heuristic at the byte level.** The
recovered keyframe records store in and out interpolation as single-byte codes, and the dominant
observed values are 1, 2 and 3. The published SDK enumeration reads 0 NONE, 1 LINEAR, 2 BEZIER,
3 HOLD, and that mapping is the working assumption -- but it is a HEURISTIC, stated as one. Only the
raw codes were read off disk. Studio's own storage uses the named enumeration of [STU-MOT-031] and
is unaffected; what is unproven is the IMPORT mapping, and an importer must verify it against
round-trip before claiming fidelity. Several hundred distinct high-valued code pairs were also
observed and are not explained by that four-member enumeration; they most likely encode tangent or
continuity state packed into the same bytes, and that packing is not decoded.

**[STU-MOT-141] GAP -- spatial tangent on-disk layout.** Per [STU-MOT-036b].

**[STU-MOT-142] OPEN DECISION -- the expression language implementation.** Per [STU-MOT-083].

**[STU-MOT-143] GAP -- the shape-operator parameter contracts.** 21 operator families and 137 shape
property keys are recovered as topology, and the child property keys per operator are listed, but
their bounds, defaults, units and precisions were NOT recovered. Every shape operator therefore
needs its parameter records authored under 14.9.1 before it can be implemented, and MUST NOT be
implemented with guessed ranges. The same applies to the 99 text animator properties.

**[STU-MOT-144] OPEN DECISION -- canonical identifier spelling.** Per [STU-MOT-073a].

**[STU-MOT-145] GAP -- categorisation of 206 expression identifiers.** Per [STU-MOT-073b]. They are
real and enumerated; their grouping for the operator-facing reference must be authored.

**[STU-MOT-146] GAP -- the graph editor is specified behaviourally, not from a captured surface.**
Its existence, its two graph types, and its operations are stated from the keyframe model's
requirements and from the recovered show/hide-graph-editor command and the numeric interpolation
and velocity dialogs. No control-level capture of the graph editor panel was available. Its precise
control inventory must be designed rather than derived.

---

## 14.26.13 Model steerability, GUI, diagnostics and manual obligation

**[STU-MOT-150]** Every panel, control, property row, keyframe, curve, tangent handle and visible state
in this sub-section MUST be model-visible and typed-steerable through the Studio command surface
(14.16); MUST be headlessly inspectable, steerable and screenshot-capturable through Argus with no
foreground focus steal (14.20); and MUST ship dual-audience UserManual entries kept same-change
current (14.22). Four obligations are specific to this sub-section:

1. **The property tree MUST be enumerable and addressable in full by a model**, at every depth,
   through stable paths ([STU-MOT-011]). A model that cannot list a layer's properties cannot
   animate anything, and a partial enumeration is worse than none because it hides capability.
2. **Every tangent MUST be settable numerically** ([STU-MOT-055]). A curve that only a mouse can
   shape is not model-steerable, and the graph editor is otherwise operator-only in a product whose
   defining constraint is that it is not.
3. **The Argus diagnostic for a property MUST report its `state`, its evaluated value at the
   current time, its keyframe count, whether an expression is present and enabled, and whether that
   expression is in an error state.** [STU-MOT-072]'s expression-driven control state is exactly
   the kind of thing a screenshot cannot disambiguate, so it must be in the structured diagnostic.
4. **The UserManual entry for any keyframable property MUST document its two toggles separately**
   -- the keyframe toggle and the expression toggle -- and MUST state what happens when a scrub is
   attempted in each of the four property states, because [STU-MOT-072]'s refusal path is the
   single most confusing moment in the surface and an undocumented refusal reads as a bug.

---

## 14.26.14 Microtask Derivation

**[STU-MOT-250] Microtask derivation index.** Applying the shared derivation convention to this
sub-section yields exactly 226 microtasks. The correspondence is NORMATIVE and CLOSED: a microtask
corresponds to a yielding clause or to a table unit as marked, and to nothing else.

Rule 0 -- derivation markers are authoritative. Every table in this sub-section carries an italic
`*Derivation: ...*` marker sentence directly above it stating how many microtasks that table yields.
The marker is normative. A tool that classifies a table differently from its marker has diverged
from this sub-section and MUST be corrected to the marker, not the reverse. The five marker forms
are: parameter table taken whole (1); enumeration table taken whole (1); preset or command table
taken whole (1); catalogue table splitting per row (N); contract table carried into the clause's own
microtask (0). A sixth form, reading aid inside a non-yielding clause, also yields 0.

Rule A -- one microtask per yielding clause. Every numbered clause yields exactly one microtask
EXCEPT the members of the no-yield set of [STU-MOT-250a]. A sub-lettered anchor
([STU-MOT-020a], [STU-MOT-036b], [STU-MOT-073b]) is a clause for this purpose and yields on its own account.

Rule B -- table units, counted from the markers of rule 0. A parameter table is a unit in its own
right even though it sits inside a clause that is also a unit, because its rows are bound-sets that
have to be individually proven. An enumeration table is a unit for the same reason, its members being
the criteria. A catalogue table splits because each row names a separately implementable subject --
one keyframe operation, one expression identifier category, one shape operator family, one scripting
class. A contract table does not split and is not its own unit: it describes the fields of the single
contract its clause already defines.

Four counts in this sub-section are traps for a tool that reads structurally rather than reading the
markers:

1. **The 31 command tables of [STU-MOT-130] hold 665 rows and yield 31, not 665.** One table is one
   command family; a row is a command, a binding and a context, and it is an acceptance criterion of
   its family's microtask.
2. **The property-tree topology of [STU-MOT-012] is a CONTRACT table and yields 0.** Its thirteen
   rows are pointers naming which clause owns each property group -- four of them pointing out of
   this sub-section into 14.27 and 14.9 -- so splitting it would double-count work owned elsewhere.
3. **The recovered group-topology appendix of [STU-MOT-095] mixes classes.** Its shape-operator table
   is a catalogue and splits per family (21); its text-group and mask-group tables are contract
   tables carried into [STU-MOT-095]'s own microtask, because the text groups are owned
   by [STU-MOT-090] through [STU-MOT-093] and the mask groups by [STU-CMP-025]. The appendix exists so
   that no part of it is normative-looking and unreachable from this ledger.
4. **The 12 expression identifier categories of [STU-MOT-073b] split, the 37 argument signatures do
   not.** A category is a namespace to build; a signature is an acceptance criterion of the family it
   belongs to, and splitting both would count the same identifiers twice.

**[STU-MOT-250a] The no-yield set: 14 clauses.** Nothing else may be excluded, and a clause not on
this list yields under rule A whether or not it is convenient.
In this list a MEMBER of the set is written in backticks, as `STU-AREA-nnn`, and an anchor written
in brackets, as [STU-AREA-nnn], is a REFERENCE and is not excluded from anything. The two forms
are visually distinct so that a reader and a tool can both count the members without parsing the
surrounding English.

The members:

1. **Supersession.** `STU-MOT-000`, which records what 14.11 offered at v02.205 and why it is
   replaced.
2. **Ownership and authority.** `STU-MOT-002` (ownership boundaries), `STU-MOT-002a` (the
   relationship to [STU-PRO-028] and the prototyping motion timeline) and `STU-MOT-003` (no sidecar
   authority).
3. **Declared-gap register rows whose gap is already stated by a yielding clause.** `STU-MOT-141`
   points at [STU-MOT-036b], `STU-MOT-142` at [STU-MOT-083], `STU-MOT-144` at [STU-MOT-073a] and
   `STU-MOT-145` at [STU-MOT-073b].
4. **This derivation section.** `STU-MOT-250`, `STU-MOT-250a`, `STU-MOT-251`, `STU-MOT-252`,
   `STU-MOT-253` and `STU-MOT-254`.

Clause [STU-MOT-150] is NOT in the no-yield set: its lead paragraph restates the steerability law, but it
carries four obligations specific to the property surface -- full enumerability and addressability of
the property tree, numeric settability of every tangent, the Argus property diagnostic, and the
UserManual obligation on the two toggles and the four property states -- and those are real, provable
work. Tables inside a non-yielding clause yield nothing.

**[STU-MOT-251] Microtask content obligation.** A microtask derived under [STU-MOT-250] MUST carry
into its own body: the clause anchor, or the catalogue row and its table; the complete member list of
every enumeration it touches; the full field list of every record it touches; for any numeric
parameter, all seven fields of [STU-FX-105] as SEPARATE values with every undeclared side left `--`
and never copied from its twin ([STU-MOT-004b]); the exact stored form of anything it animates,
including the unwrapped rotation storage of [STU-MOT-013], the influence decimal of [STU-MOT-034a]
and the value-stream contract of [STU-MOT-020a]; and the determinism obligation of [STU-MOT-076]
where it touches expressions. A microtask that says "implement keyframe interpolation" without the
five members of [STU-MOT-031], the independence of in and out interpolation ([STU-MOT-030a]) and the
0.16666666666 default influence ([STU-MOT-034]) does not satisfy this clause. No microtask may cite
the green-room corpus as its source of truth ([STU-SECTION-002]).

**[STU-MOT-252] Ledger.**

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Ledger line | Basis | Yields |
|---|---|---|
| Clauses in 14.26 | anchors 000 through 254, sub-lettered anchors included | 105 |
| less the no-yield set | the 14 clauses of [STU-MOT-250a] | -14 |
| **Rule A subtotal** | one microtask per yielding clause | **91** |
| Parameter tables | 1 table: the composition settings record of 004, taken whole | 1 |
| Enumeration tables | 5 tables: layer kinds of 005, interpolation members of 031, the four property states of 071, the argument signatures of 073b, the render-settings members of 121 | 5 |
| Command tables | 31 tables: the command families of 130, each taken whole and explicitly NOT split per row | 31 |
| Catalogue: keyframe operations of 038 | one per operation | 15 |
| Catalogue: expression identifier categories of 073b | one per declared category | 12 |
| Catalogue: load-bearing identifier families of 074 | one per family | 6 |
| Catalogue: shape operator families of 095 | one per family, owned by 101 | 21 |
| Catalogue: scripting classes of 112 | one per class | 44 |
| Contract tables | 8 tables carried into the owning clause's microtask: the property fields of 010a, the keyframe record of 030, the common layer attributes of 006, the property-tree topology of 012, the text-group and mask-group topologies of 095, the operator roles of 101, the rename provenance of 130a | 0 |
| Reading aids in non-yielding clauses | 1 table: this ledger | 0 |
| **Rule B subtotal** | table units | **135** |
| **Total microtasks yielded by 14.26** | rule A plus rule B | **226** |

**[STU-MOT-253] An open item or a blocked dependency does NOT remove a microtask.** A clause that
declares a gap, an open decision, a heuristic mapping or an unrecovered layout still yields its
rule-A microtask, and that microtask's FIRST acceptance row MUST read "the named gap is raised to the
operator as a capture request and is NOT closed by an invented value". The clauses carrying a
declared gap or open decision are [STU-MOT-036b] (the spatial tangent on-disk layout), [STU-MOT-073a]
(the canonical identifier spelling), [STU-MOT-073b] (categorisation of 206
identifiers), [STU-MOT-083] (the expression language itself), [STU-MOT-140] (the interpolation code mapping, which
is heuristic at the byte level), [STU-MOT-143] (the 21 shape-operator parameter contracts)
and [STU-MOT-146] (the graph editor, specified behaviourally rather than from a
capture). [STU-MOT-095]'s microtask is of the same kind and is closed by proving that every row of the
recovered group-topology appendix resolves to an owning clause.

**[STU-MOT-254] Anchor binding.** A microtask derived from this sub-section cites the clause anchor
directly, and a catalogue microtask additionally cites its row and the table it came from. A
microtask staged before this sub-section landed carries `spec_anchor_status = "PROVISIONAL"`;
binding it to an anchor here clears that status. A microtask that cannot cite an anchor in this
sub-section is out of scope for the motion domain and MUST be re-derived or retired, not activated.
