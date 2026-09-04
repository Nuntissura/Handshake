---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
bundle_status: "staged_draft_not_yet_in_bundle"
module_id: "14-09"
section_id: "14.9"
title: "14.9 Studio -- Effects, Filters, Adjustments & the Effect-Parameter Contract"
supersedes_module_section: "14. Studio -- Unified Creative Suite, sub-section 14.9 (lines 1644-2146 of spec-modules/14-studio-creative-suite.md at v02.205)"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 14.9 Effects, Filters, Adjustments & the Effect-Parameter Contract

## 14.9.0 Status of this sub-section

**[STU-FX-100] REPLACEMENT SCOPE.** This sub-section REPLACES sub-section 14.9 as it stood at
v02.205. Clauses [STU-FX-001] through [STU-FX-040] are retained where this sub-section does not
contradict them: the non-destructive effect model, the effect stack, effect masks, blend-composited
stack entries, cross-domain targets, determinism, GPU isolation, effect styles, and the model /
GUI / diagnostic / manual obligation all remain in force verbatim. What v02.205 lacked, and what
this sub-section adds, is the **contract**: the typed parameter record every effect parameter must
carry, the parameter kind system, the value encodings, the enumerated option model, and the
per-effect parameter catalogues with real bounds, defaults, units and precision. v02.205's 14.9
described what each effect does; it did not state a single numeric range an implementer could build
against. That is the defect this sub-section repairs.

**[STU-FX-101] CLAUSE-LEVEL AMENDMENTS TO 14.9 AS IT STOOD.** The following prior clauses are
amended rather than retained unchanged.

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Prior clause | Disposition |
|---|---|
| [STU-FX-010] | AMENDED. "Effect parameters are typed and unit-bearing" is upgraded to the full seven-field record of [STU-FX-103]. Declaring a unit is necessary but no longer sufficient. |
| [STU-FX-013] and every catalogue table in groups 3-16 | AMENDED. Those tables remain the domain-level Studio effect naming and behaviour statement. Where a catalogue row in this sub-section states a bound, default, unit, precision or enumerated option for the same effect, the value here wins, because it is read from a shipped binary and the v02.205 row was written from help-page prose. |
| [STU-FX-011] | EXTENDED. Determinism now additionally requires that a parameter clamped at a hard bound, a parameter driven past a soft bound, and a parameter driven by an expression ([STU-MOT-070]) all produce the same result for the same effective value. |
| [STU-FX-006b] | RETAINED and reinforced. Source-side per-kind instance caps stay non-normative for authoring. |
| [STU-FX-035] | RETAINED. 14.4 still owns the tonal/colour adjustment set; this sub-section owns the parameter contract those adjustments must also satisfy. |

**[STU-FX-102] NO SIDECAR AUTHORITY.** Every bound, default, unit, precision, enumerated option
and behaviour statement in this sub-section is stated here, in the Master Spec, and is implementable
without reading anything else. The green-room capture files are named only as derivation provenance
in the accompanying `.provenance.json`; they are not co-authority, they are not required reading,
and a Studio implementer with no access to them MUST be able to build every clause below. Where a
capture and this sub-section were to disagree, the disagreement is a defect in this sub-section to
be repaired against the capture, not a licence to read the capture as authority ([STU-SECTION-002]
as amended in 14.0).

---

## 14.9.1 The Effect-Parameter Contract

This group is the single most consequential contract in the Studio specification. It is stated
first because every other numeric surface in Studio -- raster adjustments, vector live effects,
develop parameters, motion-graphics template controls, export parameters, audio processors -- is
required to satisfy it.

### 1. The record

**[STU-FX-103] `StudioEffectParameter` is the single typed parameter record.** Every parameter of
every `StudioLiveFilter`, `StudioAdjustment`, layer-style kind, and any other Studio surface that
exposes a numeric, enumerated, colour, point, path, layer-reference or boolean value to an operator
or a model MUST be represented by exactly one `StudioEffectParameter` record (schema id
`hsk.studio.effect_parameter@1`). The record carries the following fields, each of which is a
SEPARATE field. Collapsing any two of them is forbidden by [STU-FX-107].

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Type | Required | Meaning |
|---|---|---|---|
| `param_key` | stable string | yes | Identity of this parameter within its effect kind. Stable across versions; never derived from the display label. |
| `param_index` | u16 | yes | Ordinal position within the effect's parameter list, and the wire order for positional invocation. |
| `display_name` | localized string | yes | Operator-facing label. |
| `kind` | `StudioParameterKind` | yes | One of the 15 kinds of [STU-FX-112]. |
| `hard_min` | number \| null | yes (may be null) | Smallest value the engine accepts. See [STU-FX-104]. |
| `hard_max` | number \| null | yes (may be null) | Largest value the engine accepts. See [STU-FX-104]. |
| `soft_min` | number \| null | yes (may be null) | Lower end of the range the control presents by default. See [STU-FX-105]. |
| `soft_max` | number \| null | yes (may be null) | Upper end of the range the control presents by default. See [STU-FX-105]. |
| `bound_state` | `StudioBoundState` | yes | Provenance of the four bound fields. See [STU-FX-106]. |
| `default` | value \| null | yes (may be null) | Value the parameter takes on a fresh application of the effect. |
| `unit` | `StudioUnitToken` \| null | yes (may be null) | Real unit token. See [STU-FX-108]. |
| `precision` | u8 \| null | yes (may be null) | Decimal places the source declares. See [STU-FX-109]. |
| `step`, `coarse_step`, `fine_step` | number | yes | Scrub increments. See [STU-FX-110]. |
| `animatable` | bool | yes | Whether the parameter may hold keyframes or an expression ([STU-MOT-030], [STU-MOT-070]). |
| `interpolable` | bool | yes | Whether values between two keyframes are computed or held. |
| `options` | `StudioParameterOption[]` \| null | when `kind = enumeration` | See [STU-FX-116]. |
| `import_key` | string \| null | no | Source identifier preserved for round-trip only. Never an operator- or model-facing name ([STU-SECTION-003]). |

### 2. Hard bounds

**[STU-FX-104] `hard_min` and `hard_max` are the engine's acceptance range.** A value outside
`[hard_min, hard_max]` is an ERROR, not a clamp: the typed command surface MUST reject it with a
determinate `EFFECT_PARAM_OUT_OF_RANGE` result naming the parameter, the offending value, and both
hard bounds. A model-authored command carrying an out-of-range value fails validation at the
`StudioValidationDescriptor` stage and never reaches the `PromotionGate` ([STU-ARC-005]). An
operator-side control MUST NOT silently clamp a typed entry to a hard bound; it MUST surface the
rejection. Where one hard bound is known and the other is not, the known one is enforced and the
unknown one is `null` with `bound_state` recording the asymmetry.

### 3. Soft bounds

**[STU-FX-105] `soft_min` and `soft_max` are the range the control presents.** They govern the
travel of a slider, the default extent of a scrub gesture, and the range a graph editor draws
without rescaling. They are NOT a validation surface. An operator or a model MAY set a value
outside the soft range and inside the hard range; doing so is legal, MUST succeed, and MUST cause
the control to extend or rescale rather than refuse. A parameter whose current value lies outside
its soft range MUST render in an explicit out-of-soft-range state so the operator can see that the
control is showing an extended range.

**[STU-FX-105a] The split is real and it is large.** Of the parameter records recovered from
shipped effect binaries, 833 declare a hard range and 734 declare at least one soft bound. 606
declare the complete four-field set. Of those 606, **366 -- 60.4 percent -- have a soft range that
differs from the hard range.** Narrowing to the 618 parameters that declare a soft minimum
alongside any hard bound, 378 differ, which is 61.2 percent. The canonical worked example is the
four-corner gradient generator's `Blend` parameter: `hard_min` 5, `hard_max` 10000, `soft_min` 10,
`soft_max` 500, `default` 100, `precision` 1. A slider built to 5..10000 is unusable; an engine
built to 10..500 rejects four fifths of the legal domain. Both facts are true simultaneously and
only two separate field pairs can express them.

### 4. Absent bounds

[STU-FX-106] **`bound_state` records where the bounds came from, and "unknown" is a legal, required
answer.** The enumeration is normative:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| `bound_state` | Meaning | Implementer obligation |
|---|---|---|
| `declared_both` | Source declares a hard range and a soft range. | Enforce hard, present soft. |
| `declared_hard_only` | Source declares a hard range; no soft range exists. | Enforce hard; set soft equal to hard AND keep `bound_state` at `declared_hard_only` so the equality is never mistaken for a declaration. |
| `declared_soft_only` | Source declares a UI range only. | Present soft; leave `hard_min`/`hard_max` null and do NOT reject on range. |
| `unbounded_in_source` | The source declares no bound at all. | Leave all four null. Do NOT invent a bound. Do NOT clamp. The control scales to the value in hand. |
| `observed_only` | A range was derived by tabulating values across shipped presets. | Leave `hard_min`/`hard_max` null; MAY seed `soft_min`/`soft_max` from the observation, marked as such. Clamping to an observed range is FORBIDDEN because it forbids legal values the observation never sampled. |

[STU-FX-106a] **Absent bounds are the common case and the specification says so rather than
inventing values.** Across 9,654 parameter rows recovered from the video-editing effect surface,
only 631 -- 6.5 percent -- carry any bound at all. The remaining 93.5 percent are
`unbounded_in_source`. This is not a capture failure; those effects genuinely ship without declared
bounds, and a Studio implementation that invents ranges for them will reject legal operator input
and will diverge from every imported project. The catalogue tables in this sub-section mark every
such parameter explicitly.

### 5. The irreversible mistake

[STU-FX-107] **Collapsing hard and soft bounds into one range is FORBIDDEN and is not
recoverable.** No Studio type, wire schema, SurrealDB field, MCP `inputSchema`, UI model, import
path, export path, preset format, or test fixture may represent a parameter's range as a single
`(min, max)` pair. Once two distinct pairs are written into one, the distinction cannot be restored
from Studio's own data and must be re-derived from an external source that may not be available.
This applies from the first commit: emit all four fields even when a source declares only one,
marking the others absent per [STU-FX-106] rather than equal. A schema, migration, or serializer
that offers only one pair fails validation ([STU-FX-123]).

### 6. Units

**[STU-FX-108] `unit` carries a real unit token, never a guess.** The normative `StudioUnitToken`
enumeration is: `pixels`, `percent`, `degrees`, `seconds`, `frames`, `milliseconds`, `hertz`,
`decibels`, `points`, `millimetres`, `inches`, `document_units`, `samples`, `bits_per_channel`,
`ratio`, `index`, `unitless`. A parameter whose source declares no unit carries `unitless`; it does
NOT inherit a plausible unit. Two display conventions recovered from the effect binaries are
carried as separate presentation flags rather than as units, because they alter presentation and
not storage: a `percent` display flag (the stored value is shown multiplied to a percentage) and a
`pixel` display flag (the stored value is shown in document pixels and rescales with document
resolution). A third recovered flag, `reverse`, inverts the direction of the control's travel
without changing the stored value. All three are recorded on the parameter and MUST be honoured by
both the operator control and any generated documentation.

**[STU-FX-108a]** Unit law [STU-DOC-003] applies without exception: mixed-unit parameters are
forbidden, colour-valued parameters carry an explicit `StudioColorProfile` reference, and unit
conversion happens at the API decode boundary and nowhere else.

### 7. Precision

**[STU-FX-109] `precision` is the decimal-place count the source declares**, not a rendering
preference. It governs the number of decimals a control displays and accepts, the rounding applied
before a value is committed to authority, and the tolerance used when comparing two values for
equality in history, undo, and promotion-equivalence checks. Where a source declares no precision,
`precision` is null and the parameter is treated as full-float in storage while the control chooses
a display precision from the soft range's magnitude; that choice is a UI heuristic and MUST be
labelled as one, never written back into the parameter record. In the recovered records, 506 of
1,573 parameters declare a precision; the remaining 1,067 are null.

### 8. The scrubbable numeric control

[STU-FX-110] **Every numeric parameter is scrubbable, and the scrub increments are part of the
parameter record.** `step` is the increment for one unit of scrub travel or one arrow-key press;
`coarse_step` and `fine_step` are the modified increments. Their derivation rule is normative and
deterministic so that two Studio builds agree:

- If `precision` is declared, `step = 10^(-precision)`, `fine_step = 10^(-(precision+1))`,
  `coarse_step = step * 10`.
- If `precision` is null and a soft range exists, `step = (soft_max - soft_min) / 200`,
  `fine_step = step / 10`, `coarse_step = step * 10`.
- If neither exists, `step = 1`, `fine_step = 0.1`, `coarse_step = 10`.
- An `integer` or `enumeration` kind forces `step = coarse_step = fine_step = 1`.

**[STU-FX-110a]** Scrubbing MUST clamp at `hard_min`/`hard_max` where they are declared, and MUST pass
freely through `soft_min`/`soft_max`, extending the control. Typed entry follows [STU-FX-104]:
inside hard bounds it succeeds regardless of soft bounds; outside hard bounds it is rejected, not
clamped. The scrub gesture, the typed entry, the arrow-key path and the AccessKit-exposed
increment/decrement actions MUST all route through one clamp implementation so an assistive-
technology adjustment cannot reach a value the pointer path forbids, and vice versa. The exact
coarse and fine modifier keys are an open operator decision recorded in [STU-FX-149].

### 9. Parameter kinds

**[STU-FX-112] The normative `StudioParameterKind` enumeration.** The effect binaries expose 19
parameter type codes, four of which are structural or reserved. Studio's kind set is the deduped
15 below; the mapping column is provenance for import.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| `StudioParameterKind` | Value shape | Source type codes |
|---|---|---|
| `scalar` | float, honours all four bounds | `SLIDER`, `FIX_SLIDER`, `FLOAT_SLIDER` |
| `integer` | signed integer, honours all four bounds | integer-typed sliders and steppers |
| `angle` | float in degrees, wraps rather than clamps unless a hard bound is declared | `ANGLE` |
| `boolean` | bool, carries an optional inline checkbox label | `CHECKBOX` |
| `color` | 4-channel value plus a required `StudioColorProfile` reference | `COLOR` |
| `point_2d` | (x, y), stored as fractions of layer size by default | `POINT` |
| `point_3d` | (x, y, z) | `POINT_3D` |
| `enumeration` | 1-based index into `options` | `POPUP` |
| `layer_reference` | reference to another `StudioLayer` used as an effect input | `LAYER` |
| `path` | reference to a `StudioVectorPath` / mask path | `PATH` |
| `arbitrary_data` | opaque typed blob (LUT payload, curve, shader graph, mesh) with a declared schema id | `ARBITRARY_DATA`, `CUSTOM` |
| `action` | invocable button; has no stored value | `BUTTON` |
| `group` | structural container; has no stored value | `GROUP_START` / `GROUP_END` |
| `label` | display-only row; has no stored value | `NO_DATA` |
| `normalised_scalar` | float in 0..1 with a declared mapping to a real-world value | audio processor input slots, see [STU-FX-121] |

**[STU-FX-112a]** The four source codes with no Studio kind (`RESERVED2`, `RESERVED3`, and the two
group terminators folded into `group`) are recorded here so an importer encountering them has a
defined disposition: reserved codes are dropped with a warning; group terminators close the open
group.

### 10. Parameter behaviour flags

**[STU-FX-113] The normative parameter flag set.** Eight behaviour flags were recovered from the
effect binaries. Studio carries them as typed booleans on `StudioEffectParameter`, not as an opaque
bitfield.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio flag | Meaning | Source bit |
|---|---|---|
| `animatable` | inverse of the source's cannot-time-vary bit. When false the parameter may hold neither a keyframe nor an expression. | `0x2` (CANNOT_TIME_VARY) |
| `interpolable` | inverse of the source's cannot-interpolate bit. When false the parameter holds discrete values only; keyframes on it behave as `hold` regardless of the declared interpolation. | `0x4` (CANNOT_INTERP) |
| `collapsed_by_default` | the parameter's disclosure twirl starts closed. | `0x20` (COLLAPSE_TWIRLY) |
| `supervised` | changing this parameter may rewrite sibling parameters; the effect must be re-queried after the change rather than assuming the other values are unchanged. | `0x40` (SUPERVISE) |
| `legacy_default_override` | on import of an older document, use the stored value rather than the current default. | `0x80` |
| `is_track_matte_input` | a `layer_reference` parameter whose referenced layer is consumed as a matte rather than as image content. | `0x100` |
| `excluded_from_change_detection` | changes to this parameter do not by themselves invalidate a cached render. | `0x200` |
| `no_reveal_on_unhide` | when a hidden parameter group is revealed, this parameter stays hidden. | `0x400` |

**[STU-FX-113a]** `supervised` is load-bearing for the model surface: a model that sets a supervised
parameter and then reads back a sibling it did not set MUST re-read rather than assume, and the
typed command receipt for a supervised change MUST return the full post-change parameter set.

### 11. Value encodings

[STU-FX-115] **Recovered value encodings are normative for import and for the descriptor, and are
normalised on the way into Studio.** Four encodings appear in shipped effect binaries:

- **16.16 fixed point.** A 32-bit fixed-point on-disk value equal to the real value multiplied by
  65536. Studio stores the decoded real value; the encoding survives only in the import path.
- **1-based enumeration index.** Enumerated options are indexed from 1, not 0. Studio preserves
  1-based indexing on the wire and in authority so that an imported document's stored indices
  remain valid without an offset table. Index 0 is reserved and invalid.
- **Point as percentage of layer size.** Point defaults are expressed as percentages of the host
  layer's width and height, so an effect applied to a differently-sized layer lands in the same
  relative place. Studio stores `point_2d` defaults in this normalised form and resolves to
  document pixels at evaluation.
- **8-bit ARGB.** Colour defaults are 8-bit ARGB. Studio widens to its working colour depth on
  import and attaches the document's `StudioColorProfile`; it never stores a bare device colour
  ([STU-DOC-003]).

### 12. Enumerated option lists

**[STU-FX-116] `StudioParameterOption` is the enumerated-option record.** 433 enumerated option
lists were recovered from the effect binaries and are reproduced verbatim in the catalogues below.
Each option carries `index` (1-based, matching [STU-FX-115]), `label` (localized), and
`is_separator`. A separator is a layout row: it occupies an index, it is NOT selectable, and an
importer or a model that selects a separator index MUST be rejected. In the recovered lists a
separator appears as the literal label `(-`; Studio normalises that to `is_separator = true` and an
empty label. `default_index` names the option a fresh application selects, and it is a required
field on every `enumeration` parameter.

**[STU-FX-116a]** Enumerated option indices are STABLE. Adding an option appends a new index; it never
renumbers an existing one, because stored documents hold indices, not labels.

### 13. Parameter grouping

**[STU-FX-117] Parameters form a tree, not a flat list.** `group` parameters open a named
collapsible container and a matching terminator closes it; `depth` on each parameter records its
nesting level. The tree is normative for the operator inspector layout, for the model surface
(where a group is an object in the generated `inputSchema`), and for generated manual entries. A
group has no value of its own and never appears in a value stream.

### 14. Colour, point, path and layer-reference parameters

**[STU-FX-118]** A `color` parameter's value is meaningless without a profile. Every `color` parameter
carries a required `StudioColorProfile` reference and is evaluated in the layer's declared working
space ([STU-FX-006a]).

**[STU-FX-119]** A `point_2d` or `point_3d` parameter exposes an on-canvas manipulator in addition to
its numeric fields, and the manipulator and the fields are two projections of one value
([STU-DOC-004]). Both paths MUST be reachable headlessly by a typed command ([STU-FX-038]).

**[STU-FX-120]** A `layer_reference` parameter names another `StudioLayer` in the same composition as
an input to the effect -- a displacement map, a matte source, a gradient reference, a depth map.
The reference is by stable `layer_id`, never by index or by name, so reordering or renaming layers
cannot silently repoint an effect. A `layer_reference` carrying `is_track_matte_input` consumes the
referenced layer's alpha or luminance rather than its colour ([STU-CMP-030]).

**[STU-FX-120a]** A `path` parameter references a `StudioVectorPath` or a mask path on any layer, and
is the mechanism by which an effect is driven along drawn geometry.

### 15. Normalised-input parameters and declared mapping expressions

[STU-FX-121] **`normalised_scalar` is a parameter whose stored value is a 0..1 normalised input and
whose real-world value is produced by a declared mapping.** This shape is used by the audio
processor family, where the processing core takes normalised inputs and the interface presents real
units. The record carries three additional required fields:

- `input_slot`: the processor input the normalised value feeds.
- `value_mapping`: the closed-form expression mapping normalised to real value.
- `default_normalised`: the normalised default, from which `default` is computed by the mapping.

The mapping expressions recovered are all affine over a declared min/max pair, of the form
`value = normalised * (max - min) + min`. Studio stores BOTH the normalised value and the derived
real value, presents the real value with its unit, and round-trips the normalised value so an
imported processor state is bit-preserved. 172 parameter rows across 31 processor definitions carry
a fully declared mapping; they are reproduced in 14.9.7.

### 16. Validation of the contract itself

**[STU-FX-123] `STUDIO_EFFECT_PARAM_CONTRACT` is a required `StudioValidationDescriptor` check**
and runs on every effect descriptor at build time and on every effect registration at startup. It
fails when any of the following is true for any parameter:

1. Fewer than four bound fields exist in the type, or two of them share storage.
2. `bound_state` is absent, or is `declared_both` while a bound field is null.
3. `bound_state` is `observed_only` and a hard bound is non-null.
4. `unit` is absent (rather than explicitly `unitless`).
5. `kind` is `enumeration` and `options` or `default_index` is absent, or `default_index` names a
   separator, or any option index is 0.
6. `kind` is `color` and no `StudioColorProfile` reference is present.
7. `kind` is `normalised_scalar` and `value_mapping` is absent.
8. `step`, `coarse_step` or `fine_step` disagrees with the [STU-FX-110] derivation.
9. `animatable` is true and `interpolable` is absent.

A descriptor that fails this check cannot be registered, so an effect with an incomplete parameter
contract cannot ship.

---

## 14.9.2 The Effect Descriptor and the Effect Registry

[STU-FX-124] **`StudioEffectDescriptor` (schema id `hsk.studio.effect_descriptor@1`) is the
registration record for one effect kind.** It carries: `filter_kind` (the stable catalogue key used
by `StudioLiveFilter.filter_kind`), `display_name`, `category` ([STU-FX-126]), `targets` (the R/V/T/G
set of [STU-FX-005]), `parameters` (an ordered `StudioEffectParameter` tree), `gpu_requirement`
([STU-FX-125]), `supported_bit_depths` ([STU-FX-006a]), `render_scope` ([STU-FX-002]),
`is_deterministic` and, when it is not, the `seed` parameter key ([STU-FX-011]), `description`, and
`import_keys` (source identifiers preserved for round-trip).

**[STU-FX-125] `gpu_requirement` is a three-valued declaration**, not a boolean: `required` (the
effect has no CPU path; absence of a supported backend yields a determinate
`EFFECT_GPU_UNAVAILABLE` per [STU-FX-018]), `accelerated` (a GPU path and a promotion-equivalent CPU
fallback both exist), `cpu_only` (no GPU path). Of the effects recovered from the compositing
application's registry, 264 carry a GPU registration; 95 of those are effects whose plug-in binary
is present in the install read, and 154 GPU registrations name effects not installed there. A
Studio build declares `gpu_requirement` from its own implementation, never from an import.

**[STU-FX-126] The normative Studio effect category set.** Studio ships ONE category vocabulary
across every source. It is the deduped union of the compositing application's 46 declared
categories and the editing application's 33, with source-only, obsolete, debug and test buckets
dropped and vendor product names removed ([STU-SECTION-003]):

`blur_and_sharpen`, `distort`, `perspective`, `color_correction`, `channel`, `keying`, `matte`,
`generate`, `noise_and_grain`, `stylize`, `simulation`, `time`, `transition`, `transform`, `text`,
`expression_controls`, `utility`, `depth_and_3d_channel`, `immersive_video`, `audio`,
`layer_effects`, `image_control`, `tracking_and_stabilization`, `paint`.

**[STU-FX-126a]** Source categories that do not map are recorded here with their disposition so no
capability is silently lost: `Obsolete` / `OBSOLETE_CATEGORY` (26 entries) are import-only
compatibility shims and are NOT registered as authorable Studio effects; `Debug`, `Test` and
`Ignored` (10 entries) are vendor-internal and are dropped; `Dissolve`, `Slide` and `Wipe` and
their `*_CATEGORY` duplicates fold into `transition`; `Adjust`, `Color Styles`, `Lights & Blurs`,
`Lights & Glows`, `Grunge & Distort`, `Smart Tools`, `Transformers` and `Animation` are vendor
marketing groupings whose members fold into the deduped categories above by behaviour;
`Cinema 4D` and `Boris FX Mocha` are third-party host bridges and are recorded as
adapter-lane rows under [STU-FX-032], not as native Studio effects; `Photoshop Layer Effects`
folds into `layer_effects` ([STU-FX-025], and see 14.9.5).

**[STU-FX-127] Effect identity is by behaviour, not by source.** Where two source applications ship
the same effect, it is ONE Studio `filter_kind` with both source identifiers in `import_keys`. A
microtask set derived from this sub-section MUST therefore produce one implementation task per
Studio `filter_kind`, never one per source application ([STU-SECTION-003]).

**[STU-FX-127b] The dedup rule is expressed by TABLE STRUCTURE, and the split is normative.** A
rule stated only in prose cannot be applied by anything that reads the catalogue mechanically,
because nothing in a row says whether that row is a second sighting of an effect already listed. The
catalogues therefore carry the answer structurally: in 14.9.6 the rows that introduce a new
`filter_kind` and the rows that dedup onto 14.9.3 are SEPARATE TABLES, each with its own derivation
marker, and the same split is applied to the preset-defined pseudo-effects of [STU-FX-133b]. The
two tables MUST NOT be merged back together by a later editor: merging them destroys the only
machine-readable statement of which rows are work and which are provenance, and re-deriving it would
require the capture again. Both tables keep every row and every column, and the dedup table keeps
its `Import key (provenance)` values verbatim, because import matching and round-trip run off that
column and not off the split.

**[STU-FX-127c] The identity test is the Studio NAME, and the capture's engine-level count is
provenance that does not resolve to a row list.** After the rename of [STU-FX-127a], two rows carry
the same Studio name if and only if they are the same `filter_kind`; that is a test any reader or
tool can apply to the tables as they stand. Applying it: of the 371 rows in 14.9.6's effect
catalogue, 192 carry a name already present in 14.9.3 and dedup onto it, and 179 rows carry 173
distinct new names; of the 107 preset-defined pseudo-effect rows, 106 dedup onto 14.9.3's own
pseudo-effect table and 1 is new. The capture separately reports an ENGINE-level count -- 337 of the
617 editing-application entries sharing the compositing application's engine, 141
editing-application-native, 138 audio processors and 1 grading intrinsic owned by 14.8 -- and that
count is retained here as provenance. It does NOT agree with the name test and it does NOT resolve
to a list of rows: nothing recovered says which 337 they are, and neither the `Engine` column nor
any other column reproduces the partition. The normative dedup for this sub-section is therefore the
name test above. Reconciling the 337 against the 192 requires re-deriving effect identity from the
binaries and is a declared gap; it is recorded as such and is NOT closed by choosing 337 rows to
make the arithmetic agree.

**[STU-FX-127a] 137 catalogue rows carried a source or vendor token in the Studio-effect NAME
column and have been renamed; the captured string survives in the import key.** [STU-SECTION-003] is
absolute: a source product name, a source-application abbreviation and a third-party vendor prefix
are provenance, never a Studio-facing name. The renamed populations are 76 rows carrying a
`CC `/`CS ` vendor prefix, 41 carrying an `AE`/`AEFilter` source-application abbreviation (5 of them
additionally carrying a third-party vendor's product word), 12 naming a source raster application,
5 naming a vendor grading product, and one each for a third-party keyer, a third-party 3D bridge and
a third-party planar tracker. Three catalogue category headings naming vendor products were renamed
for the same reason, which also removes their contradiction with [STU-FX-126a].

Four rules govern the rename and they are normative:

1. **Behaviour, parameters, bounds, GPU status and category are unchanged.** Only the operator-facing
   name changed. No capability was added, removed or reinterpreted.
2. **Round-trip is preserved by the `Import key (provenance)` column**, which still carries the
   captured identifier verbatim for every renamed row. Every captured string that was removed from
   the name column is recoverable from that column; an importer matches on the import key and never
   on the Studio name.
3. **A vendor prefix is stripped, not kept as decoration.** Where the stripped name collided with a
   different, separately registered effect in the SAME catalogue, the row carries the qualifier
   `(Alternate Engine)`, which asserts only that the capture registered two implementations -- four
   rows: `Radial Blur`, `Smear`, `Threshold` and `Grid Wipe`.
4. **Where a renamed row is the same engine as a row already named in another catalogue, both rows
   now carry the SAME Studio name.** That is [STU-FX-127]'s dedup made visible: one `filter_kind`,
   several import keys. Ten names in the video-editing catalogue are shared this way and this is
   correct, not a collision.

Two rows could not be given a behavioural name because the capture does not establish what they do:
`AE_LStr:AE_OLD_MT` and `AE_LStr:AEGPDriver`. They are named `Legacy Matte Entry (unidentified)` and
`Effect Host Bridge (registration only)`, which state what is known and no more; inventing a
capability for them would be worse than the vendor string was ([STU-FX-129]).

### Coverage accounting

**[STU-FX-128] What the catalogues below cover, stated honestly.** The compositing application's
registry declares 635 effect entries. 153 of those are registration-only: the registry names them
but no binary is present in the install read, so no parameter record could be recovered. 482 exist
in the install: 339 ship as a plug-in binary, 106 are preset-defined pseudo-effects, and 37 are
registry entries whose binary was absent. Of the 482, **208 have fully typed parameter records
covering 1,573 parameters**, and those are reproduced in 14.9.4. The remaining effects are listed
in the catalogue with their category, GPU status and identity, but WITHOUT parameter records,
because none could be read. An implementer MUST treat a catalogue row with no parameter table as
**specified in identity and behaviour but NOT yet specified in parameters**, and MUST raise the gap
rather than invent a parameter list. This is declared gap [STU-FX-146].

**[STU-FX-129] Vendor prose coverage is low and this is declared, not hidden.** Only 62 of the 635
effect entries carry a recovered plain-English description, and only 42 of those are effects
installed in the read. Every other behaviour statement in the catalogues below is Studio's own
normative statement of what the effect does, derived from its parameter set and its category. The
generated UserManual entry for an effect with no recovered vendor description MUST be authored, not
templated, and the tooltip generation contract ([STU-FX-151]) carries the same limit.

---

## 14.9.3 Catalogue: the compositing effect set

[STU-FX-130] **The tables in this group are the normative Studio effect set for the compositing and
motion domain.** Each row is ONE Studio effect. The `GPU` column states `Req` where a GPU
registration exists for that effect and `--` where none does; per [STU-FX-125] a Studio build
declares its own requirement, and `Req` here means "the field implementation is GPU-backed and a
CPU fallback must be proven promotion-equivalent before it is claimed". The `Typed params` column
is the number of parameter records recovered; a row showing `--` is covered by [STU-FX-128]. The
`Import key` column is provenance for round-trip only and is never a Studio-facing name.


**Blur & Sharpen** (21 effects)

*Derivation: catalogue table, splits per row; yields 20 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Bilateral Blur | -- | -- | _no vendor description recovered_ | `ADBE Bilateral` |
| Box Blur | -- | -- | _no vendor description recovered_ | `ADBE Box Blur` |
| Camera Lens Blur | -- | -- | _no vendor description recovered_ | `ADBE Camera Lens Blur` |
| Camera-Shake Deblur | -- | -- | _no vendor description recovered_ | `ADBE CameraShakeDeblur` |
| Cross Blur | -- | -- | _no vendor description recovered_ | `CS CrossBlur` |
| Radial Blur (Alternate Engine) | -- | -- | _no vendor description recovered_ | `CC Radial Blur` |
| Fast Radial Blur | -- | 4 | _no vendor description recovered_ | `CC Radial Fast Blur` |
| Vector Blur | -- | -- | _no vendor description recovered_ | `CC Vector Blur` |
| Channel Blur | Req | 7 | _no vendor description recovered_ | `ADBE Channel Blur` |
| Compound Blur | Req | -- | _no vendor description recovered_ | `ADBE Compound Blur` |
| Directional Blur | Req | 3 | _no vendor description recovered_ | `ADBE Motion Blur` |
| Fast Blur (Legacy) | Req | -- | _no vendor description recovered_ | `ADBE Fast Blur` |
| Fast Box Blur | -- | 5 | _no vendor description recovered_ | `ADBE Box Blur2` |
| Gaussian Blur | Req | 4 | _no vendor description recovered_ | `ADBE Gaussian Blur 2` |
| Gaussian Blur (Legacy) | -- | -- | _no vendor description recovered_ | `ADBE Gaussian Blur` |
| Radial Blur | -- | 7 | _no vendor description recovered_ | `ADBE Radial Blur` |
| Reduce Interlace Flicker | Req | -- | Reduces combing and flicker in interlaced footage for a steadier look. | `ADBE Reduce Interlace Flicker` |
| Sharpen | Req | -- | Increases edge contrast to make details appear crisper. | `ADBE Sharpen` |
| Smart Blur | -- | -- | _no vendor description recovered_ | `ADBE Smart Blur` |
| Unsharp Mask | Req | -- | Sharpens the image by enhancing edge contrast with control over amount and radius. | `ADBE Unsharp Mask` |
| Unsharp Mask | -- | -- | _no vendor description recovered_ | `ADBE Unsharp Mask2` |

**Distort** (39 effects)

*Derivation: catalogue table, splits per row; yields 38 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Bezier Warp | -- | -- | _no vendor description recovered_ | `ADBE BEZMESH` |
| Bulge | -- | 8 | _no vendor description recovered_ | `ADBE Bulge` |
| Bend Region | -- | -- | _no vendor description recovered_ | `CC Bend It` |
| Bend Layer | -- | -- | _no vendor description recovered_ | `CC Bender` |
| Blob Displace | -- | 23 | _no vendor description recovered_ | `CC Blobbylize` |
| Flow Motion | -- | -- | _no vendor description recovered_ | `CC Flo Motion` |
| Grid Shear | -- | -- | _no vendor description recovered_ | `CC Griddler` |
| Lens Distort | -- | -- | _no vendor description recovered_ | `CC Lens` |
| Page Turn | -- | -- | _no vendor description recovered_ | `CC Page Turn` |
| Power Pin | -- | 13 | _no vendor description recovered_ | `CC Power Pin` |
| Ripple Pulse | -- | -- | _no vendor description recovered_ | `CC Ripple Pulse` |
| Slant | -- | -- | _no vendor description recovered_ | `CC Slant` |
| Smear (Alternate Engine) | -- | -- | _no vendor description recovered_ | `CC Smear` |
| Split | -- | -- | _no vendor description recovered_ | `CC Split` |
| Split 2 | -- | -- | _no vendor description recovered_ | `CC Split 2` |
| Tiler | -- | -- | _no vendor description recovered_ | `CC Tiler` |
| Corner Pin | Req | -- | Repositions each corner independently to warp the clip with perspective. | `ADBE Corner Pin` |
| Detail-preserving Upscale | -- | -- | _no vendor description recovered_ | `ADBE Upscale` |
| Displacement Map | -- | -- | _no vendor description recovered_ | `ADBE Displacement Map` |
| Liquify | -- | -- | _no vendor description recovered_ | `ADBE LIQUIFY` |
| Magnify | Req | 11 | _no vendor description recovered_ | `ADBE Magnify` |
| Mesh Warp | -- | 5 | _no vendor description recovered_ | `ADBE MESH WARP` |
| Mirror | Req | -- | Reflects the image across a center axis to create mirrored symmetry. | `ADBE Mirror` |
| Offset | Req | 3 | Shifts the image within the frame by moving its visible area. | `ADBE Offset` |
| Optics Compensation | -- | -- | _no vendor description recovered_ | `ADBE Optics Compensation` |
| Polar Coordinates | -- | 3 | _no vendor description recovered_ | `ADBE Polar Coordinates` |
| Puppet | -- | -- | _no vendor description recovered_ | `ADBE FreePin3` |
| Reshape | -- | -- | _no vendor description recovered_ | `ADBE RESHAPE` |
| Ripple | -- | 8 | _no vendor description recovered_ | `ADBE Ripple` |
| Rolling Shutter Repair | Req | -- | _no vendor description recovered_ | `ADBE Rolling Shutter` |
| Smear | -- | -- | _no vendor description recovered_ | `ADBE SCHMEAR` |
| Spherize | Req | 3 | Bends the image outward as if it were mapped onto a sphere. | `ADBE Spherize` |
| Transform | Req | -- | Controls position, scale, rotation, skew, and anchor point in a single effect. | `ADBE Geometry` |
| Transform | Req | 13 | Controls position, scale, rotation, skew, and anchor point in a single effect. | `ADBE Geometry2` |
| Turbulent Displace | Req | 15 | Uses turbulent noise to warp and distort the image in a fluid way. | `ADBE Turbulent Displace` |
| Twirl | Req | 4 | Twists pixels around a center point to create a spiral distortion. | `ADBE Twirl` |
| Warp | -- | -- | _no vendor description recovered_ | `ADBE WRPMESH` |
| Warp Stabilizer | Req | -- | Analyzes motion and stabilizes shaky footage for smoother playback. | `ADBE SubspaceStabilizer` |
| Wave Warp | Req | 9 | Uses wave patterns to ripple and distort the image. | `ADBE Wave Warp` |

**Perspective** (12 effects)

*Derivation: catalogue table, splits per row; yields 12 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| 3D Camera Tracker | -- | -- | _no vendor description recovered_ | `ADBE 3D Tracker` |
| 3D Glasses | -- | -- | _no vendor description recovered_ | `ADBE 3D Glasses2` |
| 3D Glasses (Obsolete) | -- | -- | _no vendor description recovered_ | `ADBE 3D Glasses` |
| Basic 3D | Req | 6 | Adds simple 3D rotation and perspective controls to the layer. | `ADBE Basic 3D` |
| Bevel Alpha | Req | 5 | _no vendor description recovered_ | `ADBE Bevel Alpha` |
| Bevel Edges | Req | -- | _no vendor description recovered_ | `ADBE Bevel Edges` |
| Cylinder | -- | -- | _no vendor description recovered_ | `CC Cylinder` |
| Environment | -- | -- | _no vendor description recovered_ | `CC Environment` |
| Sphere | -- | -- | _no vendor description recovered_ | `CC Sphere` |
| Spotlight | -- | -- | _no vendor description recovered_ | `CC Spotlight` |
| Drop Shadow | Req | 7 | Adds a shadow behind text or graphics to create depth. | `ADBE Drop Shadow` |
| Radial Shadow | Req | -- | _no vendor description recovered_ | `ADBE Radial Shadow` |

**Color Correction** (47 effects)

*Derivation: catalogue table, splits per row; yields 41 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Auto Color | -- | -- | _no vendor description recovered_ | `ADBE AutoColor` |
| Auto Contrast | -- | 6 | _no vendor description recovered_ | `ADBE AutoContrast` |
| Auto Levels | -- | -- | _no vendor description recovered_ | `ADBE AutoLevels` |
| Black & White | -- | -- | _no vendor description recovered_ | `ADBE Black&White` |
| Brightness & Contrast | -- | -- | _no vendor description recovered_ | `ADBE Brightness & Contrast` |
| Brightness & Contrast | Req | -- | Adjusts brightness and contrast across the image to correct exposure and tonal range. | `ADBE Brightness & Contrast 2` |
| Broadcast Colors | -- | -- | _no vendor description recovered_ | `ADBE Broadcast Colors` |
| Color Neutralizer | -- | -- | _no vendor description recovered_ | `CS Color Neutralizer` |
| Color Offset | -- | -- | _no vendor description recovered_ | `CC Color Offset` |
| Kernel Convolution | -- | -- | _no vendor description recovered_ | `CS Kernel` |
| Toner | -- | 8 | _no vendor description recovered_ | `CC Toner` |
| Change Color | -- | -- | _no vendor description recovered_ | `ADBE Change Color` |
| Change to Color | -- | -- | _no vendor description recovered_ | `ADBE Change To Color` |
| Channel Mixer | -- | 14 | _no vendor description recovered_ | `ADBE CHANNEL MIXER` |
| Color Balance | -- | -- | _no vendor description recovered_ | `ADBE Color Balance` |
| Color Balance | -- | 11 | _no vendor description recovered_ | `ADBE Color Balance 2` |
| Color Balance (HLS) | -- | 4 | _no vendor description recovered_ | `ADBE Color Balance (HLS)` |
| Color Link | -- | -- | _no vendor description recovered_ | `ADBE Color Link` |
| Color Stabilizer | -- | -- | _no vendor description recovered_ | `ADBE Deflicker` |
| Colorama | -- | 31 | _no vendor description recovered_ | `APC Colorama` |
| Curves | -- | 3 | _no vendor description recovered_ | `ADBE CurvesCustom` |
| Equalize | -- | -- | _no vendor description recovered_ | `ADBE Equalize` |
| Exposure | -- | -- | _no vendor description recovered_ | `ADBE Exposure` |
| Exposure | -- | 23 | _no vendor description recovered_ | `ADBE Exposure2` |
| Gamma/Pedestal/Gain | -- | -- | _no vendor description recovered_ | `ADBE Gamma/Pedestal/Gain2` |
| Hue/Saturation | -- | 10 | _no vendor description recovered_ | `ADBE HUE SATURATION` |
| Leave Color | -- | 6 | _no vendor description recovered_ | `ADBE Leave Color` |
| Levels | -- | -- | _no vendor description recovered_ | `ADBE Easy Levels` |
| Levels | -- | 10 | _no vendor description recovered_ | `ADBE Easy Levels2` |
| Levels (Individual Controls) | -- | -- | _no vendor description recovered_ | `ADBE Pro Levels` |
| Levels (Individual Controls) | -- | -- | _no vendor description recovered_ | `ADBE Pro Levels2` |
| Color Grade | Req | -- | Provides professional color correction and grading controls for exposure, contrast, and color balance. | `ADBE Lumetri` |
| OCIO CDL Transform | -- | -- | _no vendor description recovered_ | `ADBE OCIO CDL Transform` |
| OCIO Color Space Transform | -- | -- | _no vendor description recovered_ | `ADBE OCIO Color Space Transform` |
| OCIO Display Transform | -- | -- | _no vendor description recovered_ | `ADBE OCIO Display Transform` |
| OCIO File Transform | -- | -- | _no vendor description recovered_ | `ADBE OCIO FILE Transform` |
| OCIO Look Transform | -- | -- | _no vendor description recovered_ | `ADBE OCIO Look Transform` |
| Photo Filter | -- | -- | _no vendor description recovered_ | `ADBE Photo Filter` |
| Photo Filter | -- | -- | _no vendor description recovered_ | `ADBE PhotoFilterPS` |
| PS Arbitrary Map | -- | -- | _no vendor description recovered_ | `ADBE PS Arbitrary Map` |
| Selective Color | -- | -- | _no vendor description recovered_ | `ADBE SelectiveColor` |
| Shadow/Highlight | -- | -- | _no vendor description recovered_ | `ADBE ShadowHighlight` |
| Three-Way Color Corrector | -- | -- | _no vendor description recovered_ | `ADBE Three-Way Color Corrector` |
| Tint | Req | 5 | Maps the image to two chosen colors for stylized or monochrome looks. | `ADBE Tint` |
| Tritone | -- | 5 | _no vendor description recovered_ | `ADBE Tritone` |
| Vibrance | -- | -- | _no vendor description recovered_ | `ADBE Vibrance` |
| Video Limiter | Req | -- | Limits RGB values to help keep footage within broadcast-safe levels. | `ADBE DigitalVideoLimiter` |

**Channel** (17 effects)

*Derivation: catalogue table, splits per row; yields 15 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Alpha Levels | -- | -- | _no vendor description recovered_ | `ADBE Alpha Levels2` |
| Alpha Levels | -- | -- | _no vendor description recovered_ | `ADBE Alpha Levels3` |
| Arithmetic | Req | -- | _no vendor description recovered_ | `ADBE Arithmetic` |
| Blend | Req | -- | _no vendor description recovered_ | `ADBE Blend` |
| Calculations | Req | 14 | _no vendor description recovered_ | `ADBE Calculations` |
| Composite | -- | 5 | _no vendor description recovered_ | `CS Composite` |
| Composite (obsolete) | -- | -- | _no vendor description recovered_ | `CC Composite` |
| Channel Combiner | -- | 9 | _no vendor description recovered_ | `ADBE Channel Combiner` |
| Compound Arithmetic | Req | -- | _no vendor description recovered_ | `ADBE Compound Arithmetic` |
| Invert | Req | 3 | Inverts the image colors to create a negative look. | `ADBE Invert` |
| Minimax | -- | 6 | _no vendor description recovered_ | `ADBE Minimax` |
| Remove Color Matting | -- | 3 | _no vendor description recovered_ | `ADBE Remove Color Matting` |
| Set Channels | -- | 10 | _no vendor description recovered_ | `ADBE Set Channels` |
| Set Matte | Req | -- | _no vendor description recovered_ | `ADBE Set Matte2` |
| Set Matte | Req | 7 | _no vendor description recovered_ | `ADBE Set Matte3` |
| Shift Channels | -- | 5 | _no vendor description recovered_ | `ADBE Shift Channels` |
| Solid Composite | Req | 5 | _no vendor description recovered_ | `ADBE Solid Composite` |

**Keying** (14 effects)

*Derivation: catalogue table, splits per row; yields 14 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Advanced Spill Suppressor | -- | 11 | _no vendor description recovered_ | `ADBE Spill2` |
| Simple Wire Removal | -- | -- | _no vendor description recovered_ | `CC Simple Wire Removal` |
| Color Difference Key | -- | -- | _no vendor description recovered_ | `ADBE Color Difference Key` |
| Color Key | Req | -- | Keys out a selected color range to isolate subjects or backgrounds. | `ADBE Color Key` |
| Color Range | -- | -- | _no vendor description recovered_ | `ADBE Color Range` |
| Difference Matte | Req | -- | _no vendor description recovered_ | `ADBE Difference Matte2` |
| Extract | -- | -- | _no vendor description recovered_ | `ADBE Extract` |
| Inner/Outer Key | -- | -- | _no vendor description recovered_ | `ADBE ATG Extract` |
| Key Cleaner | -- | 5 | _no vendor description recovered_ | `ADBE KeyCleaner` |
| Chroma Key (Primary) | -- | 80 | _no vendor description recovered_ | `Keylight 906` |
| Linear Color Key | -- | -- | _no vendor description recovered_ | `ADBE Linear Color Key2` |
| Luma Key | -- | -- | _no vendor description recovered_ | `ADBE Luma Key` |
| Spill Suppressor | -- | -- | _no vendor description recovered_ | `ADBE Spill Suppressor` |
| Unmult | -- | 6 | _no vendor description recovered_ | `ADBE Unmult` |

**Matte** (2 effects)

*Derivation: catalogue table, splits per row; yields 2 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Matte Choker | -- | -- | _no vendor description recovered_ | `ADBE Matte Choker` |
| Simple Choker | -- | 3 | _no vendor description recovered_ | `ADBE Simple Choker` |

**Generate** (27 effects)

*Derivation: catalogue table, splits per row; yields 27 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| 4-Color Gradient | Req | 15 | Creates a gradient that blends smoothly between four corner colors. | `ADBE 4ColorGradient` |
| Advanced Lightning | Req | 31 | _no vendor description recovered_ | `ADBE Lightning 2` |
| Audio Spectrum | -- | -- | _no vendor description recovered_ | `ADBE AudSpect` |
| Audio Waveform | -- | -- | _no vendor description recovered_ | `ADBE AudWave` |
| Beam | -- | -- | _no vendor description recovered_ | `ADBE Laser` |
| Glue Gun | -- | -- | _no vendor description recovered_ | `CC Glue Gun` |
| Light Burst | -- | -- | _no vendor description recovered_ | `CC Light Burst 2.5` |
| Light Rays | -- | -- | _no vendor description recovered_ | `CC Light Rays` |
| Light Sweep | -- | 10 | _no vendor description recovered_ | `CC Light Sweep` |
| Threads | -- | -- | _no vendor description recovered_ | `CS Threads` |
| Cell Pattern | Req | 19 | _no vendor description recovered_ | `ADBE Cell Pattern` |
| Checkerboard | Req | 13 | _no vendor description recovered_ | `ADBE Checkerboard` |
| Circle | Req | 13 | _no vendor description recovered_ | `ADBE Circle` |
| Ellipse | Req | -- | _no vendor description recovered_ | `ADBE ELLIPSE` |
| Eyedropper Fill | Req | -- | _no vendor description recovered_ | `ADBE Eyedropper Fill` |
| Fill | -- | 8 | _no vendor description recovered_ | `ADBE Fill` |
| Fractal | -- | -- | _no vendor description recovered_ | `ADBE Fractal` |
| Gradient Ramp | Req | 9 | _no vendor description recovered_ | `ADBE Ramp` |
| Grid | Req | -- | _no vendor description recovered_ | `ADBE Grid` |
| Lens Flare | Req | -- | _no vendor description recovered_ | `ADBE Lens Flare` |
| Lightning | Req | -- | _no vendor description recovered_ | `ADBE Lightning` |
| Paint Bucket | Req | -- | _no vendor description recovered_ | `ADBE Paint Bucket` |
| Radio Waves | -- | 44 | _no vendor description recovered_ | `APC Radio Waves` |
| Scribble | -- | 30 | _no vendor description recovered_ | `ADBE Scribble Fill` |
| Stroke | -- | 12 | _no vendor description recovered_ | `ADBE Stroke` |
| Vegas | -- | 35 | _no vendor description recovered_ | `APC Vegas` |
| Write-on | Req | -- | _no vendor description recovered_ | `ADBE Write-on` |

**Noise & Grain** (17 effects)

*Derivation: catalogue table, splits per row; yields 13 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Add Grain | -- | -- | _no vendor description recovered_ | `VISINF Grain Implant` |
| Curl Noise | -- | -- | _no vendor description recovered_ | `ADBE Curl Noise` |
| Dust & Scratches | Req | 4 | _no vendor description recovered_ | `ADBE Dust & Scratches` |
| Fractal Noise | -- | 32 | _no vendor description recovered_ | `ADBE Fractal Noise` |
| Match Grain | -- | -- | _no vendor description recovered_ | `VISINF Grain Duplication` |
| Median | -- | 3 | _no vendor description recovered_ | `ADBE PS Median` |
| Median (Legacy) | Req | 3 | _no vendor description recovered_ | `ADBE Median` |
| Noise | Req | -- | _no vendor description recovered_ | `ADBE Noise` |
| Noise | Req | 4 | _no vendor description recovered_ | `ADBE Noise2` |
| Noise Alpha | Req | -- | _no vendor description recovered_ | `ADBE Noise Alpha` |
| Noise Alpha | Req | 10 | _no vendor description recovered_ | `ADBE Noise Alpha2` |
| Noise HLS | Req | -- | _no vendor description recovered_ | `ADBE Noise HLS` |
| Noise HLS | Req | -- | _no vendor description recovered_ | `ADBE Noise HLS2` |
| Noise HLS Auto | Req | -- | _no vendor description recovered_ | `ADBE Noise HLS Auto` |
| Noise HLS Auto | Req | -- | _no vendor description recovered_ | `ADBE Noise HLS Auto2` |
| Remove Grain | -- | -- | _no vendor description recovered_ | `VISINF Grain Removal` |
| Turbulent Noise | -- | -- | _no vendor description recovered_ | `ADBE AIF Perlin Noise 3D` |

**Stylize** (26 effects)

*Derivation: catalogue table, splits per row; yields 25 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Brush Strokes | Req | -- | Stylizes the image with painted brush-stroke textures. | `ADBE Brush Strokes` |
| Cartoon | -- | -- | _no vendor description recovered_ | `ADBE Cartoonify` |
| Block Load | -- | -- | _no vendor description recovered_ | `CS BlockLoad` |
| Burn Film | -- | -- | _no vendor description recovered_ | `CC Burn Film` |
| Glass | -- | 24 | _no vendor description recovered_ | `CC Glass` |
| Hex Tile | -- | -- | _no vendor description recovered_ | `CS HexTile` |
| Kaleida | -- | -- | _no vendor description recovered_ | `CC Kaleida` |
| Smoothie | -- | -- | _no vendor description recovered_ | `CC Mr. Smoothie` |
| Plastic | -- | -- | _no vendor description recovered_ | `CC Plastic` |
| Repeat Tile | -- | 7 | _no vendor description recovered_ | `CC RepeTile` |
| Threshold (Alternate Engine) | -- | -- | _no vendor description recovered_ | `CC Threshold` |
| Threshold RGB | -- | -- | _no vendor description recovered_ | `CC Threshold RGB` |
| Vignette | -- | 5 | _no vendor description recovered_ | `CS Vignette` |
| Color Emboss | Req | 5 | Creates a raised embossed look using the clip's colors. | `ADBE Color Emboss` |
| Emboss | Req | -- | _no vendor description recovered_ | `ADBE Emboss` |
| Find Edges | Req | 3 | Highlights contours by detecting the edges in the image. | `ADBE Find Edges` |
| Glow | -- | 15 | _no vendor description recovered_ | `ADBE Glo2` |
| Mosaic | Req | 4 | _no vendor description recovered_ | `ADBE Mosaic` |
| Motion Tile | -- | 9 | _no vendor description recovered_ | `ADBE Tile` |
| Posterize | Req | 2 | Reduces the number of tonal values to create a stylized look. | `ADBE Posterize` |
| Roughen Edges | Req | 16 | Distorts the edge of the image with a rough, irregular border. | `ADBE Roughen Edges` |
| Scatter | -- | -- | _no vendor description recovered_ | `ADBE Scatter` |
| Strobe Light | Req | -- | Flashes the image on and off to create a strobe effect. | `ADBE Strobe` |
| Texturize | Req | -- | _no vendor description recovered_ | `ADBE Texturize` |
| Threshold | Req | -- | _no vendor description recovered_ | `ADBE Threshold` |
| Threshold | Req | 2 | _no vendor description recovered_ | `ADBE Threshold2` |

**Simulation** (22 effects)

*Derivation: catalogue table, splits per row; yields 22 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Card Dance | -- | -- | _no vendor description recovered_ | `APC CardDanceCam` |
| Caustics | -- | -- | _no vendor description recovered_ | `APC Caustics` |
| Ball Action | -- | -- | _no vendor description recovered_ | `CC Ball Action` |
| Bubbles | -- | -- | _no vendor description recovered_ | `CC Bubbles` |
| Drizzle | -- | -- | _no vendor description recovered_ | `CC Drizzle` |
| Hair | -- | -- | _no vendor description recovered_ | `CC Hair` |
| Mercury Particles | -- | 33 | _no vendor description recovered_ | `CC Mr. Mercury` |
| Particle Systems II | -- | -- | _no vendor description recovered_ | `CC Particle Systems II` |
| Particle World | -- | -- | _no vendor description recovered_ | `CC Particle World` |
| Pixel Polly | -- | -- | _no vendor description recovered_ | `CC Pixel Polly` |
| Particle Systems Classic (obsolete) | -- | -- | _no vendor description recovered_ | `CC PS Classic` |
| Particle Systems LE Classic (obsolete) | -- | -- | _no vendor description recovered_ | `CC PS LE Classic` |
| Rain (obsolete) | -- | -- | _no vendor description recovered_ | `CC Rain` |
| Rainfall | -- | -- | _no vendor description recovered_ | `CSRainfall` |
| Scatterize | -- | -- | _no vendor description recovered_ | `CC Scatterize` |
| Snow (obsolete) | -- | -- | _no vendor description recovered_ | `CC Snow` |
| Snowfall | -- | -- | _no vendor description recovered_ | `CSSnowfall` |
| Star Burst | -- | -- | _no vendor description recovered_ | `CC Star Burst` |
| Foam | -- | 47 | _no vendor description recovered_ | `APC Foam` |
| Particle Playground | -- | -- | _no vendor description recovered_ | `ADBE Playgnd` |
| Shatter | -- | -- | _no vendor description recovered_ | `APC Shatter` |
| Wave World | -- | 48 | _no vendor description recovered_ | `APC Wave World` |

**Time** (10 effects)

*Derivation: catalogue table, splits per row; yields 10 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Force Motion Blur | -- | -- | _no vendor description recovered_ | `CC Force Motion Blur` |
| Time Blend | -- | -- | _no vendor description recovered_ | `CC Time Blend` |
| Time Blend FX | -- | -- | _no vendor description recovered_ | `CC Time Blend FX` |
| Wide Time | -- | -- | _no vendor description recovered_ | `CC Wide Time` |
| Echo | Req | 6 | _no vendor description recovered_ | `ADBE Echo` |
| Pixel Motion Blur | Req | -- | _no vendor description recovered_ | `ADBE OFMotionBlur` |
| Posterize Time | Req | 2 | Reduces the frame rate to create stepped, stylized motion. | `ADBE Posterize Time` |
| Time Difference | -- | 6 | _no vendor description recovered_ | `ADBE Difference` |
| Time Displacement | -- | -- | _no vendor description recovered_ | `ADBE Time Displacement` |
| Timewarp | Req | -- | _no vendor description recovered_ | `ADBE Timewarp` |

**Transition** (17 effects)

*Derivation: catalogue table, splits per row; yields 17 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Block Dissolve | Req | 6 | _no vendor description recovered_ | `ADBE Block Dissolve` |
| Card Wipe | -- | 61 | _no vendor description recovered_ | `APC CardWipeCam` |
| Glass Wipe | -- | -- | _no vendor description recovered_ | `CC Glass Wipe` |
| Grid Wipe (Alternate Engine) | -- | -- | _no vendor description recovered_ | `CC Grid Wipe` |
| Image Wipe | -- | -- | _no vendor description recovered_ | `CC Image Wipe` |
| Jaws Wipe | -- | -- | _no vendor description recovered_ | `CC Jaws` |
| Light Wipe | -- | -- | _no vendor description recovered_ | `CC Light Wipe` |
| Line Sweep | -- | -- | _no vendor description recovered_ | `CS LineSweep` |
| Radial Scale Wipe | -- | -- | _no vendor description recovered_ | `CC Radial ScaleWipe` |
| Scale Wipe | -- | -- | _no vendor description recovered_ | `CC Scale Wipe` |
| Twister | -- | -- | _no vendor description recovered_ | `CC Twister` |
| Warp-o-Matic | -- | -- | _no vendor description recovered_ | `CC WarpoMatic` |
| Gradient Wipe | Req | -- | Reveals one clip through a gradient map for a smooth transition. | `ADBE Gradient Wipe` |
| Iris Wipe | -- | -- | _no vendor description recovered_ | `ADBE IRIS_WIPE` |
| Linear Wipe | Req | 4 | _no vendor description recovered_ | `ADBE Linear Wipe` |
| Radial Wipe | Req | 6 | _no vendor description recovered_ | `ADBE Radial Wipe` |
| Venetian Blinds | Req | 5 | _no vendor description recovered_ | `ADBE Venetian Blinds` |

**Text** (4 effects)

*Derivation: catalogue table, splits per row; yields 4 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Basic Text | -- | -- | _no vendor description recovered_ | `ADBE Basic Text2` |
| Numbers | -- | -- | _no vendor description recovered_ | `ADBE Numbers2` |
| Path Text | -- | -- | _no vendor description recovered_ | `ADBE Path Text` |
| Timecode | -- | -- | _no vendor description recovered_ | `ADBE Timecode` |

**Expression Controls** (8 effects)

*Derivation: catalogue table, splits per row; yields 8 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| 3D Point Control | -- | -- | _no vendor description recovered_ | `ADBE Point3D Control` |
| Angle Control | -- | 2 | _no vendor description recovered_ | `ADBE Angle Control` |
| Checkbox Control | -- | 2 | _no vendor description recovered_ | `ADBE Checkbox Control` |
| Color Control | -- | 2 | _no vendor description recovered_ | `ADBE Color Control` |
| Dropdown Menu Control | -- | -- | _no vendor description recovered_ | `ADBE Dropdown Control` |
| Layer Control | -- | 2 | _no vendor description recovered_ | `ADBE Layer Control` |
| Point Control | -- | 2 | _no vendor description recovered_ | `ADBE Point Control` |
| Slider Control | -- | 2 | _no vendor description recovered_ | `ADBE Slider Control` |

**Utility** (9 effects)

*Derivation: catalogue table, splits per row; yields 7 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Apply Color LUT | -- | -- | _no vendor description recovered_ | `ADBE Apply Color LUT` |
| Apply Color LUT | -- | -- | _no vendor description recovered_ | `ADBE Apply Color LUT2` |
| Overbrights | -- | -- | _no vendor description recovered_ | `CC Overbrights` |
| Cineon Converter | Req | -- | Converts between Cineon log and linear color values. | `ADBE Cineon Converter` |
| Cineon Converter | -- | -- | _no vendor description recovered_ | `ADBE Cineon Converter2` |
| Color Profile Converter | -- | -- | _no vendor description recovered_ | `ADBE ProfileToProfile` |
| Grow Bounds | -- | 2 | _no vendor description recovered_ | `ADBE GROW BOUNDS` |
| HDR Compander | -- | 4 | _no vendor description recovered_ | `ADBE Compander` |
| HDR Highlight Compression | -- | -- | _no vendor description recovered_ | `ADBE HDR ToneMap` |

**3D Channel** (8 effects)

*Derivation: catalogue table, splits per row; yields 8 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| 3D Channel Extract | -- | -- | _no vendor description recovered_ | `ADBE AUX CHANNEL EXTRACT` |
| Cryptomatte | -- | -- | _no vendor description recovered_ | `Cryptomatte` |
| Depth Matte | -- | -- | _no vendor description recovered_ | `ADBE DEPTH MATTE` |
| Depth of Field | -- | -- | _no vendor description recovered_ | `ADBE DEPTH FIELD` |
| EXtractoR | -- | -- | _no vendor description recovered_ | `EXtractoR` |
| Fog 3D | -- | -- | _no vendor description recovered_ | `ADBE FOG_3D` |
| ID Matte | -- | -- | _no vendor description recovered_ | `ADBE ID MATTE` |
| IDentifier | -- | -- | _no vendor description recovered_ | `IDentifier` |

**Immersive Video** (13 effects)

*Derivation: catalogue table, splits per row; yields 13 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| VR Blur | Req | -- | Applies blur to 360° footage while preserving the immersive projection. | `Mettle SkyBox Blur` |
| VR Chromatic Aberrations | Req | -- | Adds color separation artifacts to immersive 360° footage. | `Mettle SkyBox Chromatic Aberrations` |
| VR Color Gradients | Req | -- | Applies gradient color treatments designed for 360° video. | `Mettle SkyBox Color Gradients` |
| VR Converter | -- | -- | _no vendor description recovered_ | `Mettle SkyBox Converter` |
| VR De-Noise | Req | -- | Reduces noise in immersive 360° footage. | `Mettle SkyBox Denoise` |
| VR Digital Glitch | Req | -- | Adds glitch artifacts designed for immersive 360° video. | `Mettle SkyBox Digital Glitch` |
| VR Fractal Noise | Req | -- | Generates fractal noise for immersive 360° visuals. | `Mettle SkyBox Fractal Noise` |
| VR Glow | Req | -- | Adds a glowing light treatment designed for 360° footage. | `Mettle SkyBox Glow` |
| VR Gradient Wipe | Req | -- | Reveals the next scene with a wipe-based transition. | `Mettle SkyBox Gradient Wipe` |
| VR Plane to Sphere | Req | -- | Projects flat video onto a spherical 360° canvas. | `Mettle SkyBox Project 2D` |
| VR Rotate Sphere | Req | -- | Rotates spherical 360° footage around its viewing axis. | `Mettle SkyBox Rotate Sphere` |
| VR Sharpen | Req | -- | Sharpens immersive 360° footage while preserving its projection. | `Mettle SkyBox Sharpen` |
| VR Sphere To Plane | -- | -- | _no vendor description recovered_ | `Mettle SkyBox Viewer` |

**Audio** (13 effects)

*Derivation: catalogue table, splits per row; yields 13 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Backwards | -- | -- | _no vendor description recovered_ | `ADBE Aud Reverse` |
| Bass & Treble | -- | -- | _no vendor description recovered_ | `ADBE Aud BT` |
| Compressor | -- | 9 | _no vendor description recovered_ | `ADBE Aud Compressor` |
| Delay | -- | -- | _no vendor description recovered_ | `ADBE Aud Delay` |
| Distortion | -- | -- | _no vendor description recovered_ | `ADBE Aud Distortion` |
| Flange & Chorus | -- | -- | _no vendor description recovered_ | `ADBE Aud_Flange` |
| Gate | -- | -- | _no vendor description recovered_ | `ADBE Aud Gate` |
| High-Low Pass | -- | -- | _no vendor description recovered_ | `ADBE Aud HiLo` |
| Modulator | -- | 5 | _no vendor description recovered_ | `ADBE Aud Modulator` |
| Parametric EQ | -- | -- | _no vendor description recovered_ | `ADBE Param EQ` |
| Reverb | -- | 7 | _no vendor description recovered_ | `ADBE Aud Reverb` |
| Stereo Mixer | -- | -- | _no vendor description recovered_ | `ADBE Aud Stereo Mixer` |
| Tone | -- | 8 | _no vendor description recovered_ | `ADBE Aud Tone` |

**Layer Effects** (6 effects)

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Bevel And Emboss | -- | -- | _no vendor description recovered_ | `ADBE PSL Bevel Emboss` |
| Drop Shadow (Layer Effect) | -- | -- | _no vendor description recovered_ | `ADBE PSL Drop Shadow` |
| Inner Glow | -- | -- | _no vendor description recovered_ | `ADBE PSL Inner Glow` |
| Inner Shadow | -- | -- | _no vendor description recovered_ | `ADBE PSL Inner Shadow` |
| Outer Glow | -- | -- | _no vendor description recovered_ | `ADBE PSL Outer Glow` |
| Solid Fill | -- | -- | _no vendor description recovered_ | `ADBE PSL Solid Fill` |

**Paint** (1 effects)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Paint | -- | -- | _no vendor description recovered_ | `ADBE Paint` |

**Debug** (1 effects)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Video Abstraction | -- | -- | _no vendor description recovered_ | `ADBE VidAbs` |

**Ignored** (1 effects)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Durer | -- | -- | _no vendor description recovered_ | `AEGP` |

**External 3D Scene Bridge -- adapter lane** (1 effects)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| External 3D Scene Bridge | -- | -- | _no vendor description recovered_ | `CINEMA 4D Effect` |

**Planar Tracking Bridge -- adapter lane** (1 effects)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Planar Tracking Bridge | -- | -- | _no vendor description recovered_ | `mochaAECC` |

**Obsolete** (1 effects)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| mocha shape | -- | -- | _no vendor description recovered_ | `ISL MochaShapeImporter` |

**SLIDE_CATEGORY** (1 effects)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| Split (Legacy) | Req | -- | _no vendor description recovered_ | `ADBE Split` |

**Pseudo-effect (preset-only)** (106 effects)

*Derivation: catalogue table, splits per row; yields 105 microtasks, one per Studio effect.*

| Studio effect | GPU | Typed params | Behaviour (from capture) | Import key (provenance) |
|---|---|---|---|---|
| 2D Text Box | -- | 6 | _no vendor description recovered_ | `Pseudo/ADBE 2D Text Box` |
| Animated Shape Control | -- | 2 | _no vendor description recovered_ | `ADBE CM Animated Shape 3` |
| Animated Shape Control | -- | 5 | _no vendor description recovered_ | `ADBE CM Animated Shape Control` |
| Autoscroll - horizontal | -- | 1 | _no vendor description recovered_ | `ADBE CM AutoscrollHorizontal` |
| Autoscroll - vertical | -- | 1 | _no vendor description recovered_ | `ADBE CM AutoscrollVertical` |
| Bounce | -- | 5 | _no vendor description recovered_ | `ADBE DE Bounce` |
| Bounce At Marker | -- | 3 | _no vendor description recovered_ | `ADBE DE Bounce At Marker` |
| Bounce On Beat | -- | 5 | _no vendor description recovered_ | `ADBE DE Bounce On Beat` |
| Bounce Random | -- | 5 | _no vendor description recovered_ | `ADBE DE Bounce Random` |
| Card Wipe Master Control | -- | 2 | _no vendor description recovered_ | `ADBE CM TransCard` |
| Chaser Control | -- | 4 | _no vendor description recovered_ | `ADBE CM Animated Shape 2` |
| Color Swirl | -- | 2 | _no vendor description recovered_ | `ADBE Color Swirl` |
| Corner Reveal | -- | 4 | _no vendor description recovered_ | `ADBE CM CornerReveal` |
| Counter Controls | -- | 12 | _no vendor description recovered_ | `Pseudo/ADBE Counter Controls` |
| Cracked Tiles | -- | 2 | _no vendor description recovered_ | `ADBE CM CrackedTiles` |
| Crop Edges | -- | 2 | _no vendor description recovered_ | `ADBE CM CropEdges` |
| Currency Controls | -- | 15 | _no vendor description recovered_ | `Pseudo/ADBE Currency Controls` |
| Dissolve - unmelt | -- | 2 | _no vendor description recovered_ | `ADBE CM DissolveUnmelt` |
| Dissolve Master Control | -- | 1 | _no vendor description recovered_ | `ADBE CM TransDissolve` |
| Drift Over Time | -- | 2 | _no vendor description recovered_ | `ADBE CM Throw` |
| Face Measurements | -- | 6 | _no vendor description recovered_ | `Pseudo/ADBE Animal Head14` |
| Fade In+Out - frames | -- | 2 | _no vendor description recovered_ | `ADBE CM FadeInOutFrames` |
| Fade In+Out - msec | -- | 2 | _no vendor description recovered_ | `ADBE CM FadeInOutmsec` |
| Fade Master Control | -- | 1 | _no vendor description recovered_ | `ADBE CM TransFade` |
| Fly to Inset | -- | 5 | _no vendor description recovered_ | `ADBE CM FlyToInset` |
| Follow | -- | 7 | _no vendor description recovered_ | `ADBE DE Follow` |
| Getting Jiggy | -- | 10 | _no vendor description recovered_ | `ADBE Getting Jiggy` |
| Grid Wipe | -- | 4 | _no vendor description recovered_ | `ADBE CM GridWipe` |
| Inset Video - framed | -- | 3 | _no vendor description recovered_ | `ADBE CM InsetVideoFramed` |
| Inset Video - torn edges | -- | 1 | _no vendor description recovered_ | `ADBE CM InsetVideoTorn` |
| Iris Wipe Master Controls | -- | 3 | _no vendor description recovered_ | `ADBE CM TransIris` |
| Jiggle | -- | 5 | _no vendor description recovered_ | `ADBE DE Jiggle` |
| Jiggle At Marker | -- | 3 | _no vendor description recovered_ | `ADBE DE Jiggle At Marker` |
| Jiggle On Beat | -- | 5 | _no vendor description recovered_ | `ADBE DE Jiggle On Beat` |
| Jiggle Random | -- | 5 | _no vendor description recovered_ | `ADBE DE Jiggle Random` |
| Light Leaks - layer markers | -- | 1 | _no vendor description recovered_ | `ADBE CM LightLeaksMarkers` |
| Light Leaks - random | -- | 2 | _no vendor description recovered_ | `ADBE CM LightLeaksRandom` |
| Mask Fade Controls | -- | 2 | _no vendor description recovered_ | `ADBE CM TransFadeMask` |
| Mood Lighting - amorphous | -- | 3 | _no vendor description recovered_ | `ADBE CM MoodLightAmorph` |
| Mood Lighting - digital | -- | 3 | _no vendor description recovered_ | `ADBE CM MoodLightDigital` |
| Mood Lighting - streaks | -- | 3 | _no vendor description recovered_ | `ADBE CM MoodLightStreaks` |
| Opacity Flash - layer markers | -- | 1 | _no vendor description recovered_ | `ADBE CM OpacityFlashMarkers` |
| Opacity Flash - random | -- | 2 | _no vendor description recovered_ | `ADBE CM OpacityFlashRandom` |
| Opacity Pulse | -- | 6 | _no vendor description recovered_ | `ADBE DE Opacity Pulse` |
| Opacity Pulse At Marker | -- | 4 | _no vendor description recovered_ | `ADBE DE Opacity Pulse At Marker` |
| Opacity Pulse On Beat | -- | 6 | _no vendor description recovered_ | `ADBE DE Opacity Pulse On Beat` |
| Opacity Pulse Random | -- | 6 | _no vendor description recovered_ | `ADBE DE Opacity Pulse Random` |
| Orbit | -- | 4 | _no vendor description recovered_ | `ADBE DE Orbit` |
| Orbit 3D | -- | 7 | _no vendor description recovered_ | `ADBE DE Orbit 3D` |
| Oscillate | -- | 6 | _no vendor description recovered_ | `ADBE DE Oscillate` |
| Oscillate At Marker | -- | 4 | _no vendor description recovered_ | `ADBE DE Oscillate At Marker` |
| Oscillate On Beat | -- | 6 | _no vendor description recovered_ | `ADBE DE Oscillate On Beat` |
| Oscillate Random | -- | 7 | _no vendor description recovered_ | `ADBE DE Oscillate Random` |
| Pattern Template | -- | 5 | _no vendor description recovered_ | `Pseudo/ADBE Pattern Template` |
| Pendulum | -- | 5 | _no vendor description recovered_ | `ADBE DE Pendulum` |
| Pendulum At Marker | -- | 3 | _no vendor description recovered_ | `ADBE DE Pendulum At Marker` |
| Pendulum On Beat | -- | 5 | _no vendor description recovered_ | `ADBE DE Pendulum On Beat` |
| Pendulum Random | -- | 5 | _no vendor description recovered_ | `ADBE DE Pendulum Random` |
| Percentage Controls | -- | 13 | _no vendor description recovered_ | `Pseudo/ADBE Percentage Controls` |
| Pulse | -- | 4 | _no vendor description recovered_ | `ADBE DE Pulse` |
| Pulse At Marker | -- | 2 | _no vendor description recovered_ | `ADBE DE Pulse At Marker` |
| Pulse On Beat | -- | 4 | _no vendor description recovered_ | `ADBE DE Pulse On Beat` |
| Pulse Random | -- | 4 | _no vendor description recovered_ | `ADBE DE Pulse Random` |
| Radial Wipe Master Controls | -- | 4 | _no vendor description recovered_ | `ADBE CM TransRadial` |
| Random Fill Color | -- | 8 | _no vendor description recovered_ | `ADBE DE Random Fill Color` |
| Random Motion | -- | 6 | _no vendor description recovered_ | `ADBE DE Random Motion` |
| Random Motion 1D | -- | 6 | _no vendor description recovered_ | `ADBE DE Random Motion 1D` |
| Random Opacity | -- | 6 | _no vendor description recovered_ | `ADBE DE Random Opacity` |
| Random Rotation | -- | 6 | _no vendor description recovered_ | `ADBE DE Random Rotation` |
| Random Rotation 3D | -- | 10 | _no vendor description recovered_ | `ADBE DE Random Rotation 3D` |
| Random Scale | -- | 7 | _no vendor description recovered_ | `ADBE DE Random Scale` |
| Rotate Over Time | -- | 1 | _no vendor description recovered_ | `ADBE CM Spin` |
| Sample Image | -- | 3 | _no vendor description recovered_ | `ADBE Sample Image` |
| Scale Bounce - layer markers | -- | 2 | _no vendor description recovered_ | `ADBE CM ScaleBounceMarkers` |
| Scale Bounce - random | -- | 3 | _no vendor description recovered_ | `ADBE CM ScaleBounceRandom` |
| Separate XYZ Position | -- | 3 | _no vendor description recovered_ | `ADBE Separate XYZ Position` |
| Separate XYZ Scale | -- | 3 | _no vendor description recovered_ | `ADBE Separate XYZ Scale` |
| Slide - variable | -- | 3 | _no vendor description recovered_ | `ADBE CM SlideVariable` |
| Slide Master Control | -- | 2 | _no vendor description recovered_ | `ADBE CM TransSlide` |
| Stereo 3D Controls | -- | 2 | _no vendor description recovered_ | `ADBE Stereo 3D Controls` |
| Stretch Master Control | -- | 1 | _no vendor description recovered_ | `ADBE CM TransStretch` |
| Stretch Master Control (edge) | -- | 2 | _no vendor description recovered_ | `ADBE CM TransDirection` |
| Stretch Master Control(corner) | -- | 2 | _no vendor description recovered_ | `ADBE CM TransCorner` |
| Swarm | -- | 3 | _no vendor description recovered_ | `ADBE DE Swarm` |
| Timer Controls | -- | 10 | _no vendor description recovered_ | `Pseudo/ADBE Timer Controls` |
| Trace Path | -- | 2 | _no vendor description recovered_ | `Pseudo/ADBE Trace Path` |
| Transition Master Control | -- | 1 | _no vendor description recovered_ | `ADBE CM TransComplete` |
| Wiggle - gelatin | -- | 2 | _no vendor description recovered_ | `ADBE CM WiggleGelatin` |
| Wiggle - position | -- | 2 | _no vendor description recovered_ | `ADBE CM WigglePosition` |
| Wiggle - rotation | -- | 2 | _no vendor description recovered_ | `ADBE CM WiggleRotation` |
| Wiggle - scale | -- | 4 | _no vendor description recovered_ | `ADBE CM WiggleScale` |
| Wiggle - shear | -- | 2 | _no vendor description recovered_ | `ADBE CM WiggleShear` |
| Wigglerama | -- | 7 | _no vendor description recovered_ | `ADBE CM Wigglerama` |
| Wipe Master Control | -- | 1 | _no vendor description recovered_ | `ADBE CM TransWipe` |
| Wipe Master Controls | -- | 2 | _no vendor description recovered_ | `ADBE CM TransWipeFeath` |
| Wobble Bounce | -- | 8 | _no vendor description recovered_ | `ADBE DE Wobble Bounce` |
| Wobble Bounce At Marker | -- | 6 | _no vendor description recovered_ | `ADBE DE Wobble Bounce At Marker` |
| Wobble Bounce On Beat | -- | 8 | _no vendor description recovered_ | `ADBE DE Wobble Bounce On Beat` |
| Wobble Bounce Random | -- | 9 | _no vendor description recovered_ | `ADBE DE Wobble Bounce Random` |
| Z Spring | -- | 8 | _no vendor description recovered_ | `ADBE DE Z Spring` |
| Z Spring At Marker | -- | 6 | _no vendor description recovered_ | `ADBE DE Z Spring At Marker` |
| Zoom - 2D spin | -- | 2 | _no vendor description recovered_ | `ADBE CM Zoom2DSpin` |
| Zoom - 3D tumble | -- | 3 | _no vendor description recovered_ | `ADBE CM Zoom3DTumble` |
| Zoom - bubble | -- | 1 | _no vendor description recovered_ | `ADBE CM ZoomBubble` |
| Zoom - spiral | -- | 4 | _no vendor description recovered_ | `ADBE CM ZoomSpiral` |
| Zoom - wobble | -- | 2 | _no vendor description recovered_ | `ADBE CM ZoomWobble` |
---

## 14.9.4 Typed parameter records

**[STU-FX-131] The parameter records in this group are normative.** Each table states, for one
effect, every recovered parameter with its kind, its hard range, its soft range, its default, its
unit, its precision and its behaviour flags, followed by the verbatim enumerated option list for
every enumeration parameter. A blank hard or soft bound is `unbounded_in_source` per [STU-FX-106]
and MUST NOT be filled in. A row whose hard and soft ranges differ is a [STU-FX-105a] case and both
pairs MUST survive into the implementation.

**[STU-FX-131a]** Reading convention: `hard_min`/`hard_max` are the engine's acceptance range;
`soft_min`/`soft_max` are the control's presented range; `--` means the source declares nothing and
Studio declares nothing. `precision` `--` means null per [STU-FX-109]. The `flags` column carries
the [STU-FX-113] source bits; `CANNOT_TIME_VARY` maps to `animatable = false` and `CANNOT_INTERP`
to `interpolable = false`.


#### Blur & Sharpen

**Fast Radial Blur** -- `CC Radial Fast Blur`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 2 | Amount | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | -- |
| 3 | Zoom | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Zoom` (1-based, default index 1): 1=Standard; 2=Brightest; 3=Darkest

**Channel Blur** -- `ADBE Channel Blur`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Red Blurriness | FIX_SLIDER | 0 | 32767 | 0 | 127 | 0 | -- | 1 | -- |
| 2 | Green Blurriness | FIX_SLIDER | 0 | 32767 | 0 | 127 | 0 | -- | 1 | -- |
| 3 | Blue Blurriness | FIX_SLIDER | 0 | 32767 | 0 | 127 | 0 | -- | 1 | -- |
| 4 | Alpha Blurriness | FIX_SLIDER | 0 | 32767 | 0 | 127 | 0 | -- | 1 | -- |
| 5 | Edge Behavior | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 6 | Blur Dimensions | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Blur Dimensions` (1-based, default index 1): 1=Horizontal and Vertical; 2=Horizontal; 3=Vertical

**Directional Blur** -- `ADBE Motion Blur`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Direction | ANGLE | 0 | -1.5e-05 | 0 | 7936 | 0 | degrees | -- | -- |
| 2 | Blur Length | FIX_SLIDER | 0 | 1000 | 0 | 20 | 0 | -- | 1 | -- |

**Fast Box Blur** -- `ADBE Box Blur2`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Blur Radius | FIX_SLIDER | 0 | 30000 | 0 | 127 | 0 | -- | 1 | -- |
| 2 | Iterations | SLIDER | 1 | 50 | 1 | 5 | 3 | -- | -- | -- |
| 3 | Blur Dimensions | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 4 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

Enumerated options for `Blur Dimensions` (1-based, default index 1): 1=Horizontal and Vertical; 2=Horizontal; 3=Vertical

**Gaussian Blur** -- `ADBE Gaussian Blur 2`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Blurriness | FLOAT_SLIDER | 0 | 30000 | 0 | 50 | 0 | -- | 1 | -- |
| 2 | Blur Dimensions | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

Enumerated options for `Blur Dimensions` (1-based, default index 1): 1=Horizontal and Vertical; 2=Horizontal; 3=Vertical

**Radial Blur** -- `ADBE Radial Blur`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Amount | FIX_SLIDER | 0 | 1000 | 0 | 100 | 10 | -- | 1 | -- |
| 2 | Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | SUPERVISE |
| 3 | Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 4 | Antialiasing (Best Quality) | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 5 | _(unnamed)_ | NO_DATA | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 6 | Random Seed | SLIDER | 0 | 10000 | 0 | 1000 | 0 | -- | -- | -- |

Enumerated options for `Type` (1-based, default index 1): 1=Spin; 2=Zoom

Enumerated options for `Antialiasing (Best Quality)` (1-based, default index 1): 1=Low; 2=High

#### Distort

**Bulge** -- `ADBE Bulge`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Horizontal Radius | FIX_SLIDER | 0 | 8000 | 0 | 250 | 50 | -- | 1 | -- |
| 2 | Vertical Radius | FIX_SLIDER | 0 | 8000 | 0 | 250 | 50 | -- | 1 | -- |
| 3 | Bulge Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 4 | Bulge Height | FIX_SLIDER | -4 | 4 | -1 | 1 | 1 | -- | 1 | -- |
| 5 | Taper Radius | FIX_SLIDER | 0 | 8000 | 0 | 250 | 0 | -- | 1 | -- |
| 6 | Antialiasing (Best Qual Only) | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 7 | Pinning | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

Enumerated options for `Antialiasing (Best Qual Only)` (1-based, default index 1): 1=Low; 2=High

**Blob Displace** -- `CC Blobbylize`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Blobbiness | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Blob Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 3 | Property | POPUP | -- | -- | -- | -- | index 6 | -- | -- | -- |
| 4 | Softness | FIX_SLIDER | 0 | 500 | 1 | 50 | 20 | -- | 1 | -- |
| 5 | Cut Away | FIX_SLIDER | 0 | 100 | 0 | 100 | 25 | -- | 1 | -- |
| 6 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 7 | Light | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 8 | Light Intensity | FIX_SLIDER | 0 | 1000 | 0 | 150 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 9 | Light Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 10 | Light Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 11 | Light Height | FIX_SLIDER | -100 | 100 | 0 | 100 | 65 | -- | 1 | COLLAPSE_TWIRLY |
| 12 | Light Position | POINT | -- | -- | -- | -- | (25%, 25%) of layer | -- | -- | -- |
| 13 | Light Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 14 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 15 | Shading | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 16 | Ambient | FIX_SLIDER | 0 | 200 | 0 | 100 | 75 | -- | 1 | -- |
| 17 | Diffuse | FIX_SLIDER | 0 | 100 | 0 | 100 | 25 | -- | 1 | -- |
| 18 | Specular | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 19 | Roughness | FIX_SLIDER | 0.000992 | 0.5 | 0.000992 | 0.25 | 0.024994 | -- | 3 | -- |
| 20 | Metal | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 21 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 22 | Using | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |

Enumerated options for `Property` (1-based, default index 6): 1=Red; 2=Green; 3=Blue; 4=Alpha; 5=Luminance; 6=Lightness

Enumerated options for `Light Type` (1-based, default index 1): 1=Distant Light; 2=Point Light

Enumerated options for `Using` (1-based, default index 1): 1=Effect Light; 2=AE Lights

**Power Pin** -- `CC Power Pin`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Top Left | POINT | -- | -- | -- | -- | (0%, 0%) of layer | -- | -- | -- |
| 3 | Top Right | POINT | -- | -- | -- | -- | (100%, 0%) of layer | -- | -- | -- |
| 4 | Bottom Left | POINT | -- | -- | -- | -- | (0%, 100%) of layer | -- | -- | -- |
| 5 | Bottom Right | POINT | -- | -- | -- | -- | (100%, 100%) of layer | -- | -- | -- |
| 6 | Perspective | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | percent | 1 | COLLAPSE_TWIRLY |
| 7 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | CANNOT_TIME_VARY |
| 8 | Expansion (%) | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 9 | Top | FIX_SLIDER | -200 | 200 | 0 | 50 | 0 | -- | 1 | -- |
| 10 | Left | FIX_SLIDER | -200 | 200 | 0 | 50 | 0 | -- | 1 | -- |
| 11 | Right | FIX_SLIDER | -200 | 200 | 0 | 50 | 0 | -- | 1 | -- |
| 12 | Bottom | FIX_SLIDER | -200 | 200 | 0 | 50 | 0 | -- | 1 | -- |
| 13 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

**Magnify** -- `ADBE Magnify`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 2 | Size | FIX_SLIDER | 1 | 4000 | 10 | 600 | 100 | -- | 1 | -- |
| 3 | Magnification | FIX_SLIDER | 100 | 20000 | 100 | 600 | 150 | -- | 1 | -- |
| 4 | Feather | FIX_SLIDER | 0 | 1000 | 0 | 50 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 5 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |
| 8 | Scaling | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 9 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 10 | Shape | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 11 | Link | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 12 | Blending Mode | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |

Enumerated options for `Scaling` (1-based, default index 1): 1=Standard; 2=Soft; 3=Scatter

Enumerated options for `Shape` (1-based, default index 1): 1=Circle; 2=Square

Enumerated options for `Link` (1-based, default index 1): 1=None; 2=Size To Magnification; 3=Size & Feather To Magnification

Enumerated options for `Blending Mode` (1-based, default index 2): 1=None; 2=Normal; 4=Add; 5=Multiply; 6=Screen; 7=Overlay; 8=Soft Light; 9=Hard Light; 11=Color Dodge; 12=Color Burn; 14=Darken; 15=Lighten; 16=Difference; 17=Exclusion; 19=Hue; 20=Saturation; 21=Color; 22=Luminosity

**Mesh Warp** -- `ADBE MESH WARP`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Rows | SLIDER | 1 | 31 | 1 | 31 | 7 | -- | -- | CANNOT_TIME_VARY |
| 2 | Columns | SLIDER | 1 | 31 | 1 | 31 | 7 | -- | -- | CANNOT_TIME_VARY |
| 4 | Distortion Mesh | ARBITRARY_DATA | -- | -- | -- | -- | -- | -- | -- | COLLAPSE_TWIRLY |
| 5 | Quality | SLIDER | 1 | 10 | 1 | 10 | 8 | -- | -- | -- |

**Offset** -- `ADBE Offset`  (GPU-accelerated)

Shifts the image within the frame by moving its visible area.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Shift Center To | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 2 | Blend With Original | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | percent | 1 | -- |

**Polar Coordinates** -- `ADBE Polar Coordinates`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Interpolation | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | percent | 1 | -- |
| 2 | Type of Conversion | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |

Enumerated options for `Type of Conversion` (1-based, default index 2): 1=Rect to Polar; 2=Polar to Rect

**Ripple** -- `ADBE Ripple`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Radius | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | -- | 1 | -- |
| 2 | Center of Ripple | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 3 | Type of Conversion | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 4 | Wave Speed | FIX_SLIDER | -15 | 15 | -6 | 6 | 1 | -- | 1 | CANNOT_INTERP |
| 5 | Wave Width | FIX_SLIDER | 2 | 100 | 2 | 100 | 20 | -- | 1 | -- |
| 6 | Wave Height | FIX_SLIDER | 0 | 400 | 0 | 100 | 20 | -- | 1 | -- |
| 7 | Ripple Phase | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |

Enumerated options for `Type of Conversion` (1-based, default index 1): 1=Asymmetric; 2=Symmetric

**Spherize** -- `ADBE Spherize`  (GPU-accelerated)

Bends the image outward as if it were mapped onto a sphere.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Radius | FIX_SLIDER | 0 | 2500 | 0 | 250 | 0 | -- | 1 | -- |
| 2 | Center of Sphere | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |

**Transform** -- `ADBE Geometry2`  (GPU-accelerated)

Controls position, scale, rotation, skew, and anchor point in a single effect.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Anchor Point | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 2 | Position | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 3 | Scale Height | FIX_SLIDER | -30000 | 30000 | -200 | 200 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 4 | Scale Width | FIX_SLIDER | -30000 | 30000 | -200 | 200 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 5 | Skew | FIX_SLIDER | -70 | 70 | -70 | 70 | 0 | -- | 1 | -- |
| 6 | Skew Axis | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 7 | Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 8 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 9 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 10 | Shutter Angle | FIX_SLIDER | 0 | 360 | 0 | 360 | 0 | -- | 2 | -- |
| 11 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | SUPERVISE |
| 12 | Sampling | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |

Enumerated options for `Sampling` (1-based, default index 1): 1=Bilinear; 2=Bicubic

**Turbulent Displace** -- `ADBE Turbulent Displace`  (GPU-accelerated)

Uses turbulent noise to warp and distort the image in a fluid way.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Displacement | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | Amount | FIX_SLIDER | -10000 | 10000 | 0 | 100 | 50 | -- | 1 | -- |
| 3 | Size | FIX_SLIDER | 2 | 1000 | 5 | 400 | 100 | -- | 1 | -- |
| 4 | Offset (Turbulence) | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 5 | Complexity | FIX_SLIDER | 1 | 10 | 1 | 5 | 1 | -- | 1 | COLLAPSE_TWIRLY |
| 6 | Evolution | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 7 | Evolution Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 8 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | SUPERVISE |
| 9 | Cycle (in Revolutions) | SLIDER | 1 | 88 | 1 | 30 | 1 | -- | -- | SUPERVISE |
| 10 | Random Seed | SLIDER | 0 | 100000 | 0 | 1000 | 0 | -- | -- | SUPERVISE |
| 11 | Random Seed | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 12 | Pinning | POPUP | -- | -- | -- | -- | index 3 | -- | -- | SUPERVISE |
| 13 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 14 | Antialiasing for Best Quality | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Displacement` (1-based, default index 1): 1=Turbulent; 2=Bulge; 3=Twist; 5=Turbulent Smoother; 6=Bulge Smoother; 7=Twist Smoother; 9=Vertical Displacement; 10=Horizontal Displacement; 11=Cross Displacement

Enumerated options for `Pinning` (1-based, default index 3): 1=None; 3=Pin All; 4=Pin Horizontal; 5=Pin Vertical; 6=Pin Left; 7=Pin Right; 8=Pin Top; 9=Pin Bottom; 11=Pin All Locked; 12=Pin Horizontal Locked; 13=Pin Vertical Locked; 14=Pin Left Locked; 15=Pin Right Locked; 16=Pin Top Locked; 17=Pin Bottom Locked

Enumerated options for `Antialiasing for Best Quality` (1-based, default index 1): 1=Low; 2=High

**Twirl** -- `ADBE Twirl`  (GPU-accelerated)

Twists pixels around a center point to create a spiral distortion.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Angle | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 2 | Twirl Radius | FIX_SLIDER | 0 | 100 | 0 | 100 | 30 | -- | 1 | -- |
| 3 | Twirl Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |

**Wave Warp** -- `ADBE Wave Warp`  (GPU-accelerated)

Uses wave patterns to ripple and distort the image.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Wave Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 2 | Wave Height | FIX_SLIDER | -32000 | 32000 | 0 | 100 | 10 | -- | 0 | -- |
| 3 | Wave Width | FIX_SLIDER | 1 | 32000 | 1 | 100 | 40 | -- | 0 | -- |
| 4 | Direction | ANGLE | 1 | 32000 | 1 | 100 | 40 | degrees | 0 | -- |
| 5 | Wave Speed | FIX_SLIDER | -100 | 100 | 0 | 5 | 1 | -- | 1 | CANNOT_INTERP |
| 6 | Pinning | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 7 | Phase | ANGLE | -100 | 100 | 0 | 5 | 1 | degrees | 1 | -- |
| 8 | Antialiasing (Best Quality) | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Wave Type` (1-based, default index 1): 1=Sine; 2=Square; 3=Triangle; 4=Sawtooth; 5=Circle; 6=Semicircle; 7=Uncircle; 8=Noise; 9=Smooth Noise

Enumerated options for `Pinning` (1-based, default index 1): 1=None; 2=All Edges; 3=Center; 5=Left Edge; 6=Top Edge; 7=Right Edge; 8=Bottom Edge; 10=Horizontal Edges; 11=Vertical Edges

Enumerated options for `Antialiasing (Best Quality)` (1-based, default index 1): 1=Low; 2=Medium; 3=High

#### Perspective

**Basic 3D** -- `ADBE Basic 3D`  (GPU-accelerated)

Adds simple 3D rotation and perspective controls to the layer.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Swivel | ANGLE | 0 | -1.5e-05 | 0 | 3840 | 0 | degrees | -- | -- |
| 2 | Tilt | ANGLE | 0 | -1.5e-05 | 0 | 3840 | 0 | degrees | -- | -- |
| 3 | Distance to Image | FIX_SLIDER | -30000 | 30000 | 0 | 100 | 0 | -- | 1 | -- |
| 4 | Specular Highlight | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 5 | Preview | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Bevel Alpha** -- `ADBE Bevel Alpha`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Edge Thickness | FIX_SLIDER | 0 | 200 | 0 | 10 | 2 | -- | 2 | -- |
| 2 | Light Angle | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 3 | Light Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 4 | Light Intensity | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.399994 | -- | 2 | -- |

**Drop Shadow** -- `ADBE Drop Shadow`  (GPU-accelerated)

Adds a shadow behind text or graphics to create depth.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Shadow Color | COLOR | -- | -- | -- | -- | ARGB #FF000000 | -- | -- | -- |
| 2 | Opacity | FIX_SLIDER | 0 | 255 | 0 | 255 | 127.5 | percent | 0 | -- |
| 3 | Direction | ANGLE | 0 | 255 | 0 | 255 | 127.5 | degrees | 0 | -- |
| 4 | Distance | FIX_SLIDER | 0 | 4000 | 0 | 120 | 5 | -- | 1 | -- |
| 5 | Softness | FIX_SLIDER | 0 | 30000 | 0 | 250 | 0 | -- | 1 | -- |
| 6 | Shadow Only | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

#### Color Correction

**Auto Contrast** -- `ADBE AutoContrast`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Temporal Smoothing (seconds) | FLOAT_SLIDER | 0 | 10 | 0 | 10 | 0 | -- | 2 | COLLAPSE_TWIRLY,SUPERVISE |
| 2 | Scene Detect | CHECKBOX | -- | -- | -- | -- | false | -- | -- | SUPERVISE |
| 3 | Black Clip | FLOAT_SLIDER | 0 | 10 | 0 | 10 | 0.1 | percent | 2 | COLLAPSE_TWIRLY,SUPERVISE |
| 4 | White Clip | FLOAT_SLIDER | 0 | 10 | 0 | 10 | 0.1 | percent | 2 | COLLAPSE_TWIRLY,SUPERVISE |
| 5 | Blend With Original | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | COLLAPSE_TWIRLY |

**Toner** -- `CC Toner`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Highlights | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 2 | Midtones | COLOR | -- | -- | -- | -- | ARGB #FF806446 | -- | -- | -- |
| 3 | Shadows | COLOR | -- | -- | -- | -- | ARGB #FF000000 | -- | -- | -- |
| 4 | Blend w. Original | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | percent | 1 | -- |
| 5 | Tones | POPUP | -- | -- | -- | -- | index 2 | -- | -- | SUPERVISE |
| 6 | Brights | COLOR | -- | -- | -- | -- | ARGB #FFC0AA78 | -- | -- | -- |
| 7 | Darktones | COLOR | -- | -- | -- | -- | ARGB #FF40320A | -- | -- | -- |

Enumerated options for `Tones` (1-based, default index 2): 1=Duotone; 2=Tritone; 3=Pentone; 5=Solid

**Channel Mixer** -- `ADBE CHANNEL MIXER`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Red-Red | FIX_SLIDER | -200 | 200 | -200 | 200 | 100 | -- | 0 | -- |
| 2 | Red-Green | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 3 | Red-Blue | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 4 | Red-Const | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 5 | Green-Red | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 6 | Green-Green | FIX_SLIDER | -200 | 200 | -200 | 200 | 100 | -- | 0 | -- |
| 7 | Green-Blue | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 8 | Green-Const | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 9 | Blue-Red | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 10 | Blue-Green | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 11 | Blue-Blue | FIX_SLIDER | -200 | 200 | -200 | 200 | 100 | -- | 0 | -- |
| 12 | Blue-Const | FIX_SLIDER | -200 | 200 | -200 | 200 | 0 | -- | 0 | -- |
| 13 | Monochrome | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Color Balance** -- `ADBE Color Balance 2`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Shadow Red Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 2 | Shadow Green Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 3 | Shadow Blue Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 4 | Midtone Red Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 5 | Midtone Green Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 6 | Midtone Blue Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 7 | Highlight Red Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 8 | Highlight Green Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 9 | Highlight Blue Balance | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 10 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Color Balance (HLS)** -- `ADBE Color Balance (HLS)`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Hue | ANGLE | 0 | -1.5e-05 | 0 | 7936 | 0 | degrees | -- | -- |
| 2 | Lightness | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |
| 3 | Saturation | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | -- |

**Colorama** -- `APC Colorama`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Get Phase From | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 3 | Input Phase | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 4 | Add Phase | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 6 | Add Phase From | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 8 | Phase Shift | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 10 | Use Preset Palette | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 12 | Output Cycle | ARBITRARY_DATA | -- | -- | -- | -- | -- | -- | -- | -- |
| 14 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 15 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 16 | Modify | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 17 | Output Cycle | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 18 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 20 | Matching Color | COLOR | -- | -- | -- | -- | ARGB #FFFF0000 | -- | -- | -- |
| 22 | Matching Tolerance | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.5 | -- | 2 | COLLAPSE_TWIRLY |
| 24 | Matching Softness | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 26 | Matching Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 27 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 28 | Mask Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 29 | Modify | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 30 | Masking Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 32 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 34 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 36 | Blend With Original | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 0 | COLLAPSE_TWIRLY |
| 37 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 38 | Cycle Repetitions | FIX_SLIDER | 0 | 64 | 0 | 20 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 39 | Pixel Selection | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 40 | Add Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 49 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 51 | Masking | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 57 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `Get Phase From` (1-based, default index 1): 1=Intensity; 2=Red; 3=Green; 4=Blue; 5=Hue; 6=Lightness; 7=Saturation; 8=Value; 9=Alpha; 10=Zero

Enumerated options for `Add Phase From` (1-based, default index 1): 1=Intensity; 2=Red; 3=Green; 4=Blue; 5=Hue; 6=Lightness; 7=Saturation; 8=Value; 9=Alpha; 10=Zero

Enumerated options for `Use Preset Palette` (1-based, default index 1): 1=[none]; 2=Alpha Ramp; 3=Hue Cycle; 4=Negative; 5=Ramp Red; 6=Ramp Green; 7=Ramp Blue; 8=Ramp Grey; 9=RGB; 10=Saturation Ramp; 11=Solarize Red; 12=Solarize Green; 13=Solarize Blue; 14=Solarize Grey; 16=Carribean; 17=Clay; 18=Copper; 19=Deep Ocean; 20=Earthenware; 21=Fire; 22=Fire And Smoke; 23=Golden 1; 24=Golden 2; 25=Granite; 26=Horizon; 27=Leather; 28=Moldy; 29=Mossy; 30=Old Glory; 31=Rusty; 32=Sepia 1; 33=Sepia 2; 34=Skink; 35=Sunset

Enumerated options for `Modify` (1-based, default index 1): 1=All; 2=Red; 3=Green; 4=Blue; 5=RG; 6=GB; 7=RB; 8=Hue; 9=Lightness; 10=Saturation; 11=HL; 12=LS; 13=HS; 14=None

Enumerated options for `Matching Mode` (1-based, default index 1): 1=Off; 2=RGB; 3=Hue; 4=Chroma

Enumerated options for `Masking Mode` (1-based, default index 1): 1=Off; 2=Intensity; 3=Alpha; 4=Inverted Intensity; 5=Inverted Alpha

Enumerated options for `Add Mode` (1-based, default index 1): 1=Wrap; 2=Clamp; 3=Average; 4=Screen

**Curves** -- `ADBE CurvesCustom`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Curves | ARBITRARY_DATA | -- | -- | -- | -- | -- | -- | -- | -- |
| 2 | Channel: | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |

Enumerated options for `Channel:` (1-based, default index 1): 1=RGB; 2=Red; 3=Green; 4=Blue; 5=Alpha

**Exposure** -- `ADBE Exposure2`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Channels: | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | Master | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 3 | Exposure | FLOAT_SLIDER | -100 | 100 | -4 | 4 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 4 | Offset | FLOAT_SLIDER | -2 | 2 | -0.5 | 0.5 | 0 | -- | 4 | COLLAPSE_TWIRLY |
| 5 | Gamma Correction | FLOAT_SLIDER | 0.1 | 10 | 0.1 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 6 | Master | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 7 | Red | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 8 | Red Exposure | FLOAT_SLIDER | -100 | 100 | -4 | 4 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 9 | Red Offset | FLOAT_SLIDER | -2 | 2 | -0.5 | 0.5 | 0 | -- | 4 | COLLAPSE_TWIRLY |
| 10 | Red Gamma Correction | FLOAT_SLIDER | 0.1 | 10 | 0.1 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 11 | Red | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 12 | Green | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 13 | Green Exposure | FLOAT_SLIDER | -100 | 100 | -4 | 4 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 14 | Green Offset | FLOAT_SLIDER | -2 | 2 | -0.5 | 0.5 | 0 | -- | 4 | COLLAPSE_TWIRLY |
| 15 | Green Gamma Correction | FLOAT_SLIDER | 0.1 | 10 | 0.1 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 16 | Green | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 17 | Blue | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 18 | Blue Exposure | FLOAT_SLIDER | -100 | 100 | -4 | 4 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 19 | Blue Offset | FLOAT_SLIDER | -2 | 2 | -0.5 | 0.5 | 0 | -- | 4 | COLLAPSE_TWIRLY |
| 20 | Blue Gamma Correction | FLOAT_SLIDER | 0.1 | 10 | 0.1 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 21 | Blue | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 22 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

Enumerated options for `Channels:` (1-based, default index 1): 1=Master; 2=Individual Channels

**Hue/Saturation** -- `ADBE HUE SATURATION`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Channel Control | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 3 | Channel Range | ARBITRARY_DATA | -- | -- | -- | -- | -- | -- | -- | -- |
| 4 | Master Hue | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | CANNOT_TIME_VARY,SUPERVISE |
| 5 | Master Saturation | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 0 | CANNOT_TIME_VARY,SUPERVISE |
| 6 | Master Lightness | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 0 | CANNOT_TIME_VARY,SUPERVISE |
| 7 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 8 | Colorize Hue | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 9 | Colorize Saturation | FIX_SLIDER | 0 | 100 | 0 | 100 | 25 | -- | 0 | COLLAPSE_TWIRLY |
| 10 | Colorize Lightness | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 0 | COLLAPSE_TWIRLY |

Enumerated options for `Channel Control` (1-based, default index 1): 1=Master; 2=Reds; 3=Yellows; 4=Greens; 5=Cyans; 6=Blues; 7=Magentas

**Leave Color** -- `ADBE Leave Color`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Amount to Decolor | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |
| 2 | Color To Leave | COLOR | -- | -- | -- | -- | ARGB #00FF0000 | -- | -- | -- |
| 3 | Tolerance | FIX_SLIDER | 0 | 100 | 0 | 100 | 15 | percent | 1 | -- |
| 4 | Edge Softness | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |
| 5 | Match colors | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Match colors` (1-based, default index 1): 1=Using RGB; 2=Using Hue

**Levels** -- `ADBE Easy Levels2`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Channel: | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 2 | Histogram | ARBITRARY_DATA | -- | -- | -- | -- | -- | -- | -- | -- |
| 3 | Input Black | FLOAT_SLIDER | -10000 | 10000 | 0 | 1 | 0 | pixel | 1 | CANNOT_TIME_VARY,COLLAPSE_TWIRLY,SUPERVISE |
| 4 | Input White | FLOAT_SLIDER | -10000 | 10000 | 0 | 1 | 1 | pixel | 1 | CANNOT_TIME_VARY,COLLAPSE_TWIRLY,SUPERVISE |
| 5 | Gamma | FLOAT_SLIDER | 0 | 5 | 0 | 5 | 1 | -- | 2 | CANNOT_TIME_VARY,COLLAPSE_TWIRLY,SUPERVISE |
| 6 | Output Black | FLOAT_SLIDER | -10000 | 10000 | 0 | 1 | 0 | pixel | 1 | CANNOT_TIME_VARY,COLLAPSE_TWIRLY,SUPERVISE |
| 7 | Output White | FLOAT_SLIDER | -10000 | 10000 | 0 | 1 | 1 | pixel | 1 | CANNOT_TIME_VARY,COLLAPSE_TWIRLY,SUPERVISE |
| 8 | Clip To Output Black | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 9 | Clip To Output White | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |

Enumerated options for `Channel:` (1-based, default index 1): 1=RGB; 2=Red; 3=Green; 4=Blue; 5=Alpha

Enumerated options for `Clip To Output Black` (1-based, default index 3): 1=On; 2=Off; 3=Off for 32 bpc Color

Enumerated options for `Clip To Output White` (1-based, default index 3): 1=On; 2=Off; 3=Off for 32 bpc Color

**Tint** -- `ADBE Tint`  (GPU-accelerated)

Maps the image to two chosen colors for stylized or monochrome looks.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Map Black To | COLOR | -- | -- | -- | -- | ARGB #00000000 | -- | -- | -- |
| 2 | Map White To | COLOR | -- | -- | -- | -- | ARGB #00FFFFFF | -- | -- | -- |
| 3 | Amount to Tint | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | -- |
| 4 | _(unnamed)_ | BUTTON | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |

**Tritone** -- `ADBE Tritone`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Highlights | COLOR | -- | -- | -- | -- | ARGB #00FFFFFF | -- | -- | -- |
| 2 | Midtones | COLOR | -- | -- | -- | -- | ARGB #007F6446 | -- | -- | -- |
| 3 | Shadows | COLOR | -- | -- | -- | -- | ARGB #00000000 | -- | -- | -- |
| 4 | Blend With Original | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |

#### Channel

**Calculations** -- `ADBE Calculations`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Input | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Input Channel | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 4 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 5 | Second Source | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 6 | Second Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 7 | Second Layer Channel | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 8 | Second Layer Opacity | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | percent | 0 | -- |
| 9 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 10 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 11 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 12 | Blending Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 13 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

Enumerated options for `Input Channel` (1-based, default index 1): 1=RGBA; 2=Gray; 3=Red; 4=Green; 5=Blue; 6=Alpha

Enumerated options for `Second Layer Channel` (1-based, default index 1): 1=RGBA; 2=Gray; 3=Red; 4=Green; 5=Blue; 6=Alpha

Enumerated options for `Blending Mode` (1-based, default index 1): 1=Normal; 2=Copy; 4=Darken; 5=Multiply; 6=Color Burn; 7=Classic Color Burn; 9=Add; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Classic Color Dodge; 15=Overlay; 16=Soft Light; 17=Hard Light; 18=Linear Light; 19=Vivid Light; 20=Pin Light; 22=Difference; 23=Classic Difference; 24=Exclusion; 26=Hue; 27=Saturation; 28=Color; 29=Luminosity; 31=Stencil Alpha; 32=Stencil Luma; 33=Silhouette Alpha; 34=Silhouette Luma; 36=Alpha Add; 37=Luminescent Add

**Composite** -- `CS Composite`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | -- |
| 2 | Transfer Mode | POPUP | -- | -- | -- | -- | index 3 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | COLLAPSE_TWIRLY |
| 4 | Top Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `Transfer Mode` (1-based, default index 3): 1=Copy; 2=Behind; 3=In front; 5=Darken; 6=Multiply; 7=Linear Burn; 8=Color Burn; 9=Classic Color Burn; 11=Add; 12=Lighten; 13=Screen; 14=Linear Dodge; 15=Color Dodge; 16=Classic Color Dodge; 18=Overlay; 19=Soft Light; 20=Hard Light; 21=Linear Light; 22=Vivid Light; 23=Pin Light; 24=Hard Mix; 26=Difference; 27=Classic Difference; 28=Exclusion; 30=Hue; 31=Saturation; 32=Color; 33=Luminosity; 35=Stencil Alpha; 36=Stencil Luma; 37=Silhouette Alpha; 38=Silhouette Luma; 40=Add Alpha; 41=Luminescent Premul

**Channel Combiner** -- `ADBE Channel Combiner`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Source Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 3 | Source Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 4 | Source Layer | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 5 | From | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 6 | To | POPUP | -- | -- | -- | -- | index 7 | -- | -- | SUPERVISE |
| 7 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 8 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

Enumerated options for `From` (1-based, default index 1): 1=RGB to HLS; 2=HLS to RGB; 3=RGB to YUV; 4=YUV to RGB; 5=Straight to Premultiplied; 7=Red; 8=Green; 9=Blue; 10=Alpha; 12=Hue; 13=Lightness; 14=Luminance; 15=Saturation; 16=Saturation Multiplied; 18=Min RGB; 19=Max RGB

Enumerated options for `To` (1-based, default index 7): 1=Red; 2=Green; 3=Blue; 4=Alpha; 6=Hue; 7=Lightness; 8=Saturation; 10=Red only; 11=Green only; 12=Blue only; 13=Alpha only; 15=Hue only; 16=Lightness only; 17=Saturation only

**Invert** -- `ADBE Invert`  (GPU-accelerated)

Inverts the image colors to create a negative look.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Channel | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 2 | Blend With Original | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 0 | -- |

Enumerated options for `Channel` (1-based, default index 1): 1=RGB; 2=Red; 3=Green; 4=Blue; 6=HLS; 7=Hue; 8=Lightness; 9=Saturation; 11=YIQ; 12=Luminance; 13=In Phase Chrominance; 14=Quadrature Chrominance; 16=Alpha

**Minimax** -- `ADBE Minimax`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Operation | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 2 | Radius | SLIDER | 0 | 32000 | 0 | 127 | 0 | -- | -- | -- |
| 3 | Channel | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 4 | Direction | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 5 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | USE_VALUE_FOR_OLD_PROJECTS |

Enumerated options for `Operation` (1-based, default index 2): 1=Minimum; 2=Maximum; 3=Minimum Then Maximum; 4=Maximum Then Minimum

Enumerated options for `Channel` (1-based, default index 1): 1=Color; 2=Alpha and Color; 3=Red; 4=Green; 5=Blue; 6=Alpha

Enumerated options for `Direction` (1-based, default index 1): 1=Horizontal & Vertical; 2=Just Horizontal; 3=Just Vertical

**Remove Color Matting** -- `ADBE Remove Color Matting`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Background Color | COLOR | -- | -- | -- | -- | ARGB #00000000 | -- | -- | -- |
| 2 | Clipping | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Set Channels** -- `ADBE Set Channels`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Source Layer 1 | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Set Red To Source 1�s | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 3 | Source Layer 2 | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 4 | Set Green To Source 2�s | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 5 | Source Layer 3 | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 6 | Set Blue To Source 3�s | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 7 | Source Layer 4 | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 8 | Set Alpha To Source 4�s | POPUP | -- | -- | -- | -- | index 4 | -- | -- | -- |
| 9 | If Layer Sizes Differ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

Enumerated options for `Set Red To Source 1�s` (1-based, default index 1): 1=Red; 2=Green; 3=Blue; 4=Alpha; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full; 10=Off

Enumerated options for `Set Green To Source 2�s` (1-based, default index 2): 1=Red; 2=Green; 3=Blue; 4=Alpha; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full; 10=Off

Enumerated options for `Set Blue To Source 3�s` (1-based, default index 3): 1=Red; 2=Green; 3=Blue; 4=Alpha; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full; 10=Off

Enumerated options for `Set Alpha To Source 4�s` (1-based, default index 4): 1=Red; 2=Green; 3=Blue; 4=Alpha; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full; 10=Off

**Set Matte** -- `ADBE Set Matte3`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Take Matte From Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Use For Matte | POPUP | -- | -- | -- | -- | index 4 | -- | -- | -- |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 4 | If Layer Sizes Differ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 5 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 6 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

Enumerated options for `Use For Matte` (1-based, default index 4): 1=Red Channel; 2=Green Channel; 3=Blue Channel; 4=Alpha Channel; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full; 10=Off

**Shift Channels** -- `ADBE Shift Channels`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Take Alpha From | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 2 | Take Red From | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 3 | Take Green From | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 4 | Take Blue From | POPUP | -- | -- | -- | -- | index 4 | -- | -- | -- |

Enumerated options for `Take Alpha From` (1-based, default index 1): 1=Alpha; 2=Red; 3=Green; 4=Blue; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full On; 10=Full Off

Enumerated options for `Take Red From` (1-based, default index 2): 1=Alpha; 2=Red; 3=Green; 4=Blue; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full On; 10=Full Off

Enumerated options for `Take Green From` (1-based, default index 3): 1=Alpha; 2=Red; 3=Green; 4=Blue; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full On; 10=Full Off

Enumerated options for `Take Blue From` (1-based, default index 4): 1=Alpha; 2=Red; 3=Green; 4=Blue; 5=Luminance; 6=Hue; 7=Lightness; 8=Saturation; 9=Full On; 10=Full Off

**Solid Composite** -- `ADBE Solid Composite`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Source Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |
| 2 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 3 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | -- |
| 4 | Blending Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Blending Mode` (1-based, default index 1): 1=Normal; 3=Add; 4=Multiply; 5=Screen; 6=Overlay; 7=Soft Light; 8=Hard Light; 10=Color Dodge; 11=Color Burn; 13=Darken; 14=Lighten; 15=Difference; 16=Exclusion; 18=Hue; 19=Saturation; 20=Color; 21=Luminosity

#### Keying

**Advanced Spill Suppressor** -- `ADBE Spill2`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Method | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | Ultra Settings | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 3 | Key Color | COLOR | -- | -- | -- | -- | ARGB #FF00FF00 | -- | -- | COLLAPSE_TWIRLY |
| 4 | Tolerance | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 5 | Desaturate | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 6 | Spill Range | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 7 | Spill Color Correction | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 8 | Luma Correction | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 9 | Luma Correction | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 1001 | Suppression | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |

Enumerated options for `Method` (1-based, default index 1): 1=Standard; 2=Ultra

**Key Cleaner** -- `ADBE KeyCleaner`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Strength | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |
| 2 | Reduce Chatter | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 6 | Additional Edge Radius | FLOAT_SLIDER | 0 | 250 | 0 | 50 | 10 | -- | 1 | COLLAPSE_TWIRLY |
| 16 | Alpha Contrast | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | COLLAPSE_TWIRLY |

**Chroma Key (Primary)** -- `Keylight 906`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | About | NO_DATA | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 2 | View | POPUP | -- | -- | -- | -- | index 11 | -- | -- | CANNOT_TIME_VARY |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 4 | Screen Colour | COLOR | -- | -- | -- | -- | ARGB #FF000000 | -- | -- | SUPERVISE |
| 5 | Screen Gain | FIX_SLIDER | 0 | 5000 | 0 | 200 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 6 | Screen Balance | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 7 | Despill Bias | COLOR | -- | -- | -- | -- | ARGB #FF7F7F7F | -- | -- | SUPERVISE |
| 8 | Alpha Bias | COLOR | -- | -- | -- | -- | ARGB #FF7F7F7F | -- | -- | SUPERVISE |
| 9 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | CANNOT_TIME_VARY |
| 10 | Screen Pre-blur | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 11 | Screen Matte | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 12 | Clip Black | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 13 | Clip White | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 14 | Clip Rollback | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 15 | Screen Shrink/Grow | FIX_SLIDER | -5000 | 5000 | -20 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 16 | Screen Softness | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 17 | Screen Despot Black | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 18 | Screen Despot White | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 19 | Replace Method | POPUP | -- | -- | -- | -- | index 4 | -- | -- | CANNOT_TIME_VARY |
| 20 | Replace Colour | COLOR | -- | -- | -- | -- | ARGB #FF7F7F7F | -- | -- | -- |
| 21 | Screen Matte | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 22 | Inside Mask | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 23 | Inside Mask | PATH | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 24 | Inside Mask Softness | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 25 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 26 | Replace Method | POPUP | -- | -- | -- | -- | index 2 | -- | -- | CANNOT_TIME_VARY |
| 27 | Replace Colour | COLOR | -- | -- | -- | -- | ARGB #FF7F7F7F | -- | -- | -- |
| 28 | Source Alpha | POPUP | -- | -- | -- | -- | index 3 | -- | -- | CANNOT_TIME_VARY |
| 29 | Inside Mask | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 30 | Outside Mask | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 31 | Outside Mask | PATH | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 32 | Outside Mask Softness | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 33 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 34 | Outside Mask | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 35 | Foreground Colour Correction | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 36 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 37 | Saturation | FIX_SLIDER | -5000 | 5000 | -100 | 100 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 38 | Contrast | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 39 | Brightness | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 40 | Colour Suppression | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 41 | Suppress | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY |
| 42 | Suppression Balance | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 43 | Suppression Amount | FIX_SLIDER | 0 | 5000 | 0 | 100 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 44 | Colour Suppression | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 45 | Colour Balancing | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 46 | Hue | FIX_SLIDER | -5000 | 5000 | 0 | 360 | 0 | -- | 1 | COLLAPSE_TWIRLY,SUPERVISE |
| 47 | Sat | FIX_SLIDER | -5000 | 5000 | -20 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY,SUPERVISE |
| 48 | Colour Balance Wheel | NO_DATA | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 49 | Colour Balancing | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 50 | Foreground Colour Correction | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 51 | Edge Colour Correction | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 52 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 53 | Edge Hardness | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 54 | Edge Softness | FIX_SLIDER | 0 | 5000 | 0 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 55 | Edge Grow | FIX_SLIDER | -5000 | 5000 | -20 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 56 | Saturation | FIX_SLIDER | -5000 | 5000 | -100 | 100 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 57 | Contrast | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 58 | Brightness | FIX_SLIDER | -100 | 100 | -100 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 59 | Edge Colour Suppression | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 60 | Suppress | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY |
| 61 | Suppression Balance | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | COLLAPSE_TWIRLY |
| 62 | Suppression Amount | FIX_SLIDER | 0 | 5000 | 0 | 100 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 63 | Edge Colour Suppression | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 64 | Colour Balancing | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 65 | Hue | FIX_SLIDER | -5000 | 5000 | 0 | 360 | 0 | -- | 1 | COLLAPSE_TWIRLY,SUPERVISE |
| 66 | Sat | FIX_SLIDER | -5000 | 5000 | -20 | 20 | 0 | -- | 1 | COLLAPSE_TWIRLY,SUPERVISE |
| 67 | Colour Balance Wheel | NO_DATA | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 68 | Colour Balancing | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 69 | Edge Colour Correction | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 70 | Source Crops | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 71 | X Method | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY |
| 72 | Y Method | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY |
| 73 | Edge Colour | COLOR | -- | -- | -- | -- | ARGB #FF000000 | -- | -- | SUPERVISE |
| 74 | Edge Colour Alpha | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | CANNOT_TIME_VARY,COLLAPSE_TWIRLY,SUPERVISE |
| 75 | Left | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | -- | 1 | -- |
| 76 | Right | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 77 | Top | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | -- | 1 | -- |
| 78 | Bottom | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 79 | Source Crops | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `View` (1-based, default index 11): 1=Source; 2=Source Alpha; 3=Corrected Source; 4=Colour Correction Edges; 5=Screen Matte; 6=Inside Mask; 7=Outside Mask; 8=Combined Matte; 9=Status; 10=Intermediate Result; 11=Final Result

Enumerated options for `Replace Method` (1-based, default index 4): 1=None; 2=Source; 3=Hard Colour; 4=Soft Colour

Enumerated options for `Replace Method` (1-based, default index 2): 1=None; 2=Source; 3=Hard Colour; 4=Soft Colour

Enumerated options for `Source Alpha` (1-based, default index 3): 1=Ignore; 2=Add To Inside Mask; 3=Normal

Enumerated options for `Suppress` (1-based, default index 1): 1=None; 2=Red; 3=Green; 4=Blue; 5=Cyan; 6=Magenta; 7=Yellow

Enumerated options for `Suppress` (1-based, default index 1): 1=None; 2=Red; 3=Green; 4=Blue; 5=Cyan; 6=Magenta; 7=Yellow

Enumerated options for `X Method` (1-based, default index 1): 1=Colour; 2=Repeat; 3=Reflect; 4=Wrap

Enumerated options for `Y Method` (1-based, default index 1): 1=Colour; 2=Repeat; 3=Reflect; 4=Wrap

**Unmult** -- `ADBE Unmult`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 30 | Background Color | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 31 | Black Level | FLOAT_SLIDER | -10000 | 10000 | 0 | 1 | 0 | pixel | 0 | COLLAPSE_TWIRLY,SUPERVISE |
| 32 | Softness | FLOAT_SLIDER | 0 | 10000 | 0 | 1 | 1 | pixel | 0 | -- |
| 33 | Remove Color Matting | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 34 | Clip HDR Results | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

Enumerated options for `Background Color` (1-based, default index 1): 1=Black; 2=White

#### Matte

**Simple Choker** -- `ADBE Simple Choker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | View | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 2 | Choke Matte | FIX_SLIDER | -100 | 100 | -10 | 10 | 0 | -- | 2 | -- |

Enumerated options for `View` (1-based, default index 1): 1=Final Output; 2=Matte

#### Generate

**4-Color Gradient** -- `ADBE 4ColorGradient`  (GPU-accelerated)

Creates a gradient that blends smoothly between four corner colors.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Point 1 | POINT | -- | -- | -- | -- | (10%, 10%) of layer | -- | -- | -- |
| 2 | Color 1 | COLOR | -- | -- | -- | -- | ARGB #FFFFFF00 | -- | -- | -- |
| 3 | Point 2 | POINT | -- | -- | -- | -- | (90%, 10%) of layer | -- | -- | -- |
| 4 | Color 2 | COLOR | -- | -- | -- | -- | ARGB #FF00FF00 | -- | -- | -- |
| 5 | Point 3 | POINT | -- | -- | -- | -- | (10%, 90%) of layer | -- | -- | -- |
| 6 | Color 3 | COLOR | -- | -- | -- | -- | ARGB #FFFF00FF | -- | -- | -- |
| 7 | Point 4 | POINT | -- | -- | -- | -- | (90%, 90%) of layer | -- | -- | -- |
| 8 | Color 4 | COLOR | -- | -- | -- | -- | ARGB #FF0000FF | -- | -- | -- |
| 9 | Blend | FIX_SLIDER | 5 | 10000 | 10 | 500 | 100 | -- | 1 | -- |
| 10 | Jitter | FIX_SLIDER | 0 | 500 | 0 | 100 | 0 | percent | 1 | COLLAPSE_TWIRLY |
| 11 | Positions & Colors | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 12 | Color 4 | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 13 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |
| 14 | Blending Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Blending Mode` (1-based, default index 1): 1=None; 2=Normal; 4=Add; 5=Multiply; 6=Screen; 7=Overlay; 8=Soft Light; 9=Hard Light; 11=Color Dodge; 12=Color Burn; 14=Darken; 15=Lighten; 16=Difference; 17=Exclusion; 19=Hue; 20=Saturation; 21=Color; 22=Luminosity

**Advanced Lightning** -- `ADBE Lightning 2`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Lightning Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | Origin | POINT | -- | -- | -- | -- | (45%, 5%) of layer | -- | -- | SUPERVISE |
| 3 | Contextual Control | POINT | -- | -- | -- | -- | (50%, 95%) of layer | -- | -- | SUPERVISE |
| 4 | Conductivity State | FIX_SLIDER | 0 | 32767 | 0 | 50 | 0 | -- | 1 | COLLAPSE_TWIRLY,SUPERVISE |
| 5 | Core Settings | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY,SUPERVISE |
| 6 | Core Radius | FIX_SLIDER | 0 | 40 | 1 | 4 | 2 | -- | 1 | -- |
| 7 | Core Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 75 | percent | 1 | COLLAPSE_TWIRLY |
| 8 | Core Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | COLLAPSE_TWIRLY |
| 9 | Core Color | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 10 | Glow Settings | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 11 | Glow Radius | FIX_SLIDER | 1 | 400 | 20 | 100 | 50 | -- | 1 | -- |
| 12 | Glow Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | percent | 1 | -- |
| 13 | Glow Color | COLOR | -- | -- | -- | -- | ARGB #FF3232FF | -- | -- | -- |
| 14 | Glow Color | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 15 | Alpha Obstacle | FIX_SLIDER | -100 | 100 | -10 | 10 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 16 | Turbulence | FIX_SLIDER | 0 | 10 | 0.5 | 1.25 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 17 | Forking | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.25 | percent | 1 | -- |
| 18 | Decay | FIX_SLIDER | 0 | 100 | 0 | 1 | 0.299988 | -- | 2 | COLLAPSE_TWIRLY |
| 19 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 20 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 21 | Expert Settings | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 22 | Complexity | SLIDER | 1 | 10 | 1 | 8 | 6 | -- | -- | -- |
| 23 | Min. Forkdistance | SLIDER | 8 | 1024 | 16 | 256 | 64 | -- | -- | -- |
| 24 | Termination Threshold | FIX_SLIDER | 0.25 | 100 | 1 | 100 | 100 | percent | 1 | -- |
| 25 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 26 | Fractal Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 27 | Core Drain | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |
| 28 | Fork Strength | FIX_SLIDER | 0 | 100 | 0 | 100 | 70 | percent | 1 | -- |
| 29 | Fork Variation | FIX_SLIDER | 0 | 100 | 0 | 100 | 20 | percent | 1 | -- |
| 30 | Fork Variation | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `Lightning Type` (1-based, default index 1): 1=Direction; 2=Strike; 3=Breaking; 4=Bouncey; 6=Omni; 7=Anywhere; 8=Vertical; 9=Two-Way Strike

Enumerated options for `Fractal Type` (1-based, default index 1): 1=Linear; 2=Semi Linear; 3=Spline

**Light Sweep** -- `CC Light Sweep`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Center | POINT | -- | -- | -- | -- | (50%, 25%) of layer | -- | -- | -- |
| 2 | Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 3 | Shape | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 4 | Width | FIX_SLIDER | 0 | 4000 | 0 | 200 | 50 | -- | 1 | -- |
| 5 | Sweep Intensity | FIX_SLIDER | 0 | 500 | 0 | 100 | 25 | -- | 1 | -- |
| 6 | Edge Intensity | FIX_SLIDER | 0 | 500 | 0 | 100 | 50 | -- | 1 | -- |
| 7 | Edge Thickness | FIX_SLIDER | 0 | 20 | 0 | 10 | 4 | -- | 2 | -- |
| 8 | Light Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFAF0 | -- | -- | -- |
| 9 | Light Reception | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Shape` (1-based, default index 3): 1=Linear; 2=Smooth; 3=Sharp

Enumerated options for `Light Reception` (1-based, default index 1): 1=Add; 2=Composite; 3=Cutout

**Cell Pattern** -- `ADBE Cell Pattern`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Cell Pattern | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 3 | Contextual Slider | FIX_SLIDER | 0 | 10000 | 0 | 600 | 100 | -- | 2 | -- |
| 4 | Overflow | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 5 | Disperse | FIX_SLIDER | 0 | 1.5 | 0 | 1 | 1 | -- | 2 | -- |
| 6 | Size | FIX_SLIDER | 2 | 2000 | 10 | 500 | 60 | -- | 1 | -- |
| 7 | Offset | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 8 | Tiling Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 9 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | SUPERVISE |
| 10 | Cells Horizontal | SLIDER | 1 | 1000 | 2 | 20 | 4 | -- | -- | SUPERVISE |
| 11 | Cells Vertical | SLIDER | 1 | 1000 | 2 | 20 | 4 | -- | -- | SUPERVISE |
| 12 | Cells Vertical | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 13 | Evolution | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | SUPERVISE |
| 14 | Evolution Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 15 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | SUPERVISE |
| 16 | Cycle (in Revolutions) | SLIDER | 1 | 88 | 1 | 30 | 1 | -- | -- | SUPERVISE |
| 17 | Random Seed | SLIDER | 0 | 100000 | 0 | 1000 | 0 | -- | -- | COLLAPSE_TWIRLY |
| 18 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `Cell Pattern` (1-based, default index 1): 1=Bubbles; 2=Crystals; 3=Plates; 4=Static Plates; 5=Crystallize; 7=Pillow; 8=Crystals HQ; 9=Plates HQ; 10=Static Plates HQ; 11=Crystallize HQ; 12=Mixed Crystals; 13=Tubular

Enumerated options for `Overflow` (1-based, default index 1): 1=Clip; 2=Soft Clamp; 3=Wrap Back

**Checkerboard** -- `ADBE Checkerboard`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Anchor | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 2 | Size From | POPUP | -- | -- | -- | -- | index 2 | -- | -- | SUPERVISE |
| 3 | Corner | POINT | -- | -- | -- | -- | (58%, 58%) of layer | -- | -- | -- |
| 4 | Width | FIX_SLIDER | 1 | 4000 | 2 | 200 | 16 | -- | 1 | -- |
| 5 | Height | FIX_SLIDER | 1 | 4000 | 2 | 200 | 8 | -- | 1 | -- |
| 6 | Feather | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 7 | Width | FIX_SLIDER | 0 | 400 | 0 | 20 | 0 | -- | 1 | -- |
| 8 | Height | FIX_SLIDER | 0 | 400 | 0 | 20 | 0 | -- | 1 | -- |
| 9 | Height | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 10 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 11 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |
| 12 | Blending Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Size From` (1-based, default index 2): 1=Corner Point; 2=Width Slider; 3=Width & Height Sliders

Enumerated options for `Blending Mode` (1-based, default index 1): 1=None; 2=Normal; 3=Stencil Alpha; 5=Add; 6=Multiply; 7=Screen; 8=Overlay; 9=Soft Light; 10=Hard Light; 12=Color Dodge; 13=Color Burn; 15=Darken; 16=Lighten; 17=Difference; 18=Exclusion; 20=Hue; 21=Saturation; 22=Color; 23=Luminosity

**Circle** -- `ADBE Circle`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 2 | Radius | FIX_SLIDER | -10000 | 10000 | 0 | 600 | 75 | -- | 1 | -- |
| 3 | Edge | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 4 | Contextual Slider | FIX_SLIDER | 0 | 10000 | 0 | 600 | 10 | -- | 1 | COLLAPSE_TWIRLY |
| 5 | Feather | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 6 | Feather Outer Edge | FIX_SLIDER | 0 | 10000 | 0 | 100 | 0 | -- | 1 | -- |
| 7 | Feather Inner Edge | FIX_SLIDER | 0 | 10000 | 0 | 100 | 0 | -- | 1 | -- |
| 8 | Feather Inner Edge | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 9 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 10 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 11 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |
| 12 | Blending Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Edge` (1-based, default index 1): 1=None; 2=Edge Radius; 3=Thickness; 4=Thickness * Radius; 5=Thickness & Feather * Radius

Enumerated options for `Blending Mode` (1-based, default index 1): 1=None; 2=Normal; 3=Stencil Alpha; 5=Add; 6=Multiply; 7=Screen; 8=Overlay; 9=Soft Light; 10=Hard Light; 12=Color Dodge; 13=Color Burn; 15=Darken; 16=Lighten; 17=Difference; 18=Exclusion; 20=Hue; 21=Saturation; 22=Color; 23=Luminosity

**Fill** -- `ADBE Fill`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Fill Mask | PATH | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 2 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFF0000 | -- | -- | -- |
| 3 | Horizontal Feather | FIX_SLIDER | 0 | 999 | 0 | 50 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 4 | Vertical Feather | FIX_SLIDER | 0 | 999 | 0 | 50 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 5 | Opacity | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | percent | 1 | -- |
| 6 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 7 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | CANNOT_TIME_VARY |

**Gradient Ramp** -- `ADBE Ramp`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Start of Ramp | POINT | -- | -- | -- | -- | (50%, 0%) of layer | -- | -- | -- |
| 2 | Start Color | COLOR | -- | -- | -- | -- | ARGB #FF000000 | -- | -- | -- |
| 3 | End of Ramp | POINT | -- | -- | -- | -- | (50%, 100%) of layer | -- | -- | -- |
| 4 | End Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 5 | Ramp Shape | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 6 | Ramp Scatter | FIX_SLIDER | 0 | 512 | 0 | 50 | 0 | -- | 1 | -- |
| 7 | Blend With Original | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | percent | 1 | -- |
| 8 | _(unnamed)_ | BUTTON | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |

Enumerated options for `Ramp Shape` (1-based, default index 1): 1=Linear Ramp; 2=Radial Ramp

**Radio Waves** -- `APC Radio Waves`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Wave Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 4 | Producer Point | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 6 | Parameters are set at | POPUP | -- | -- | -- | -- | index 1 | -- | -- | COLLAPSE_TWIRLY |
| 8 | Sides | SLIDER | 3 | 128 | 3 | 128 | 64 | -- | -- | COLLAPSE_TWIRLY |
| 10 | Curve Size | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 12 | Curvyness | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 14 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | COLLAPSE_TWIRLY |
| 16 | Star Depth | FIX_SLIDER | -1 | 10 | -1 | 1 | -0.5 | -- | 2 | COLLAPSE_TWIRLY |
| 18 | Source Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 20 | Source Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 22 | Value Channel | POPUP | -- | -- | -- | -- | index 5 | -- | -- | COLLAPSE_TWIRLY |
| 23 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 24 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | COLLAPSE_TWIRLY |
| 26 | Value Threshold | FIX_SLIDER | 0 | 255 | 0 | 255 | 127 | -- | 2 | COLLAPSE_TWIRLY |
| 28 | Pre-Blur | FIX_SLIDER | 0 | 50 | 0 | 50 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 30 | Tolerance | FIX_SLIDER | 0 | 5 | 0 | 5 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 32 | Contour | SLIDER | 1 | 1000 | 1 | 50 | 1 | -- | -- | COLLAPSE_TWIRLY |
| 34 | Frequency | FIX_SLIDER | 0 | 500 | 0 | 20 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 36 | Expansion | FIX_SLIDER | 0 | 1000 | 0 | 20 | 5 | -- | 2 | COLLAPSE_TWIRLY |
| 38 | Orientation | ANGLE | 0 | 1000 | 0 | 20 | 5 | degrees | 2 | COLLAPSE_TWIRLY |
| 40 | Velocity | FIX_SLIDER | 0 | 10000 | 0 | 500 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 42 | Direction | ANGLE | 0 | 1000 | 0 | 20 | 5 | degrees | 2 | COLLAPSE_TWIRLY |
| 43 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 44 | Spin | FIX_SLIDER | -10000 | 10000 | -360 | 360 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 46 | Color | COLOR | -- | -- | -- | -- | ARGB #FF0000FF | -- | -- | COLLAPSE_TWIRLY |
| 48 | Profile | POPUP | -- | -- | -- | -- | index 1 | -- | -- | COLLAPSE_TWIRLY |
| 49 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 50 | Start Width | FIX_SLIDER | 1 | 100 | 1 | 50 | 5 | -- | 2 | COLLAPSE_TWIRLY |
| 51 | Wave Motion | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 52 | End Width | FIX_SLIDER | 1 | 100 | 1 | 50 | 5 | -- | 2 | COLLAPSE_TWIRLY |
| 54 | Opacity | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 56 | Lifespan (sec) | FIX_SLIDER | 0 | 1000 | 0 | 30 | 10 | -- | 3 | COLLAPSE_TWIRLY |
| 58 | Fade-in Time | FIX_SLIDER | 0 | 300 | 0 | 30 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 60 | Fade-out Time | FIX_SLIDER | 0 | 300 | 0 | 30 | 5 | -- | 3 | COLLAPSE_TWIRLY |
| 62 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | COLLAPSE_TWIRLY |
| 64 | Render Quality | SLIDER | 1 | 16 | 1 | 16 | 4 | -- | -- | COLLAPSE_TWIRLY |
| 66 | Mask | PATH | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 68 | Polygon | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 69 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 70 | Image Contour | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 71 | Stroke | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 72 | Mask | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 87 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `Wave Type` (1-based, default index 1): 1=Polygon; 2=Image Contours; 3=Mask

Enumerated options for `Parameters are set at` (1-based, default index 1): 1=Birth; 2=Each Frame

Enumerated options for `Value Channel` (1-based, default index 5): 1=Intensity; 2=Red; 3=Green; 4=Blue; 5=Alpha; 6=Hue; 7=Lightness; 8=Saturation; 9=Value

Enumerated options for `Profile` (1-based, default index 1): 1=Square; 2=Triangle; 3=Sawtooth Out; 4=Sawtooth In; 5=Gaussian; 6=Bell; 7=Sine

**Scribble** -- `ADBE Scribble Fill`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Mask | PATH | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 6 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 8 | Stroke Width | FIX_SLIDER | 0.099991 | 50 | 0.099991 | 25 | 2 | -- | 1 | COLLAPSE_TWIRLY |
| 9 | Edge Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 10 | Angle | ANGLE | 0 | 1 | 0 | 1 | 1 | degrees | 1 | COLLAPSE_TWIRLY |
| 12 | Curviness | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.049988 | percent | 0 | COLLAPSE_TWIRLY |
| 21 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 24 | Opacity | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | percent | 1 | COLLAPSE_TWIRLY |
| 26 | Composite | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 28 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 30 | Start | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |
| 31 | Stroke Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 32 | End | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | -- |
| 38 | Path Overlap | FIX_SLIDER | -1000 | 1000 | -100 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 40 | Path Overlap Variation | FIX_SLIDER | 0 | 1000 | 0 | 100 | 5 | -- | 1 | COLLAPSE_TWIRLY |
| 44 | Wiggles/Second | FIX_SLIDER | 0 | 30 | 0 | 30 | 10 | -- | 2 | COLLAPSE_TWIRLY |
| 45 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 46 | Random Seed | FIX_SLIDER | 1 | 1000 | 1 | 1000 | 1 | -- | 0 | COLLAPSE_TWIRLY |
| 48 | Wiggle Type | POPUP | -- | -- | -- | -- | index 3 | -- | -- | SUPERVISE |
| 50 | Fill Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 52 | Edge Width | FIX_SLIDER | 1 | 1000 | 1 | 100 | 20 | -- | 1 | COLLAPSE_TWIRLY |
| 54 | Join | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 56 | End Cap | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 58 | Miter Limit | FIX_SLIDER | 1 | 500 | 1 | 10 | 4 | -- | 1 | COLLAPSE_TWIRLY |
| 60 | Spacing | FIX_SLIDER | 0.099991 | 200 | 0.099991 | 50 | 5 | -- | 1 | COLLAPSE_TWIRLY |
| 62 | Spacing Variation | FIX_SLIDER | 0 | 200 | 0 | 50 | 1 | -- | 1 | COLLAPSE_TWIRLY |
| 64 | Scribble | POPUP | -- | -- | -- | -- | index 3 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 66 | Curviness Variation | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.009995 | percent | 0 | COLLAPSE_TWIRLY |
| 68 | Start/End Apply To | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |

Enumerated options for `Composite` (1-based, default index 2): 1=On Original Image; 2=On Transparent; 3=Reveal Original Image

Enumerated options for `Wiggle Type` (1-based, default index 3): 1=Static; 2=Jumpy; 3=Smooth

Enumerated options for `Fill Type` (1-based, default index 1): 1=Inside; 2=Centered Edge; 3=Inside Edge; 4=Outside Edge; 5=Left Edge; 6=Right Edge

Enumerated options for `Join` (1-based, default index 1): 1=Round; 2=Bevel; 3=Miter

Enumerated options for `End Cap` (1-based, default index 1): 1=Round; 2=Butt; 3=Projecting

Enumerated options for `Scribble` (1-based, default index 3): 1=None; 3=Single Mask; 4=All Masks; 5=All Masks Using Modes

Enumerated options for `Start/End Apply To` (1-based, default index 2): 1=Mask Path; 2=Scribble Result

**Stroke** -- `ADBE Stroke`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Path | PATH | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 3 | Brush Size | FIX_SLIDER | 0 | 200 | 0 | 25 | 2 | -- | 1 | -- |
| 4 | Brush Hardness | FIX_SLIDER | 0 | 0.949997 | 0 | 0.949997 | 0.75 | percent | 0 | -- |
| 5 | Opacity | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | percent | 1 | -- |
| 6 | Spacing | FIX_SLIDER | 0 | 100 | 0 | 100 | 15 | percent | 2 | CANNOT_INTERP |
| 7 | Paint Style | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 8 | Start | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |
| 9 | End | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | -- |
| 10 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | CANNOT_TIME_VARY |
| 11 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `Paint Style` (1-based, default index 1): 1=On Original Image; 2=On Transparent; 3=Reveal Original Image

**Vegas** -- `APC Vegas`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Input Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 4 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 6 | If Layer Sizes Differ | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 8 | Blend Mode | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 10 | Channel | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 12 | Threshold | FIX_SLIDER | 0 | 255 | 0 | 255 | 127 | -- | 2 | COLLAPSE_TWIRLY |
| 14 | Pre-Blur | FIX_SLIDER | 0 | 50 | 0 | 50 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 16 | Tolerance | FIX_SLIDER | 0 | 5 | 0 | 5 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 18 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFF00 | -- | -- | COLLAPSE_TWIRLY |
| 20 | Width | FIX_SLIDER | 0.5 | 100 | 0.5 | 25 | 2 | -- | 2 | COLLAPSE_TWIRLY |
| 22 | Hardness | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 24 | Length | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 26 | Segment Distribution | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 27 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 28 | Segments | SLIDER | 1 | 250 | 1 | 100 | 32 | -- | -- | COLLAPSE_TWIRLY |
| 30 | Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 32 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 33 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 34 | Random Seed | SLIDER | 1 | 1000 | 1 | 20 | 1 | -- | -- | COLLAPSE_TWIRLY |
| 35 | Segments | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 36 | Start Opacity | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 38 | Mid-point Opacity | FIX_SLIDER | -1 | 1 | -1 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 40 | Mid-point Position | FIX_SLIDER | 0.000992 | 0.998993 | 0.000992 | 0.998993 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 42 | End Opacity | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 44 | Render | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 46 | Selected Contour | SLIDER | 1 | 1000 | 1 | 50 | 1 | -- | -- | -- |
| 48 | Shorter Contours Have | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 49 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 50 | Path | PATH | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 51 | Rendering | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 52 | Stroke | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 54 | Image Contours | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 56 | Mask/Path | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 69 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `If Layer Sizes Differ` (1-based, default index 1): 1=Center; 2=Stretch to Fit

Enumerated options for `Blend Mode` (1-based, default index 2): 1=Transparent; 2=Over; 3=Under; 4=Stencil

Enumerated options for `Channel` (1-based, default index 1): 1=Intensity; 2=Red; 3=Green; 4=Blue; 5=Alpha; 6=Hue; 7=Lightness; 8=Saturation; 9=Value

Enumerated options for `Segment Distribution` (1-based, default index 2): 1=Bunched; 2=Even

Enumerated options for `Render` (1-based, default index 1): 1=All Contours; 2=Selected Contour

Enumerated options for `Shorter Contours Have` (1-based, default index 1): 1=Same Number of Segments; 2=Fewer Segments

Enumerated options for `Stroke` (1-based, default index 1): 1=Image Contours; 2=Mask/Path

#### Noise & Grain

**Dust & Scratches** -- `ADBE Dust & Scratches`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Radius | SLIDER | 0 | 255 | 0 | 255 | 1 | -- | -- | -- |
| 2 | Threshold | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | pixel | 1 | -- |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Fractal Noise** -- `ADBE Fractal Noise`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Fractal Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 2 | Noise Type | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 4 | Contrast | FIX_SLIDER | 0 | 10000 | 0 | 400 | 100 | -- | 1 | -- |
| 5 | Brightness | FIX_SLIDER | -10000 | 10000 | -100 | 100 | 0 | -- | 1 | COLLAPSE_TWIRLY |
| 6 | Overflow | POPUP | -- | -- | -- | -- | index 4 | -- | -- | COLLAPSE_TWIRLY |
| 7 | Transform | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 8 | Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 9 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | SUPERVISE |
| 10 | Scale | FIX_SLIDER | 1 | 10000 | 20 | 600 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 11 | Scale Width | FIX_SLIDER | 1 | 10000 | 20 | 600 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 12 | Scale Height | FIX_SLIDER | 1 | 10000 | 20 | 600 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 13 | Offset Turbulence | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | COLLAPSE_TWIRLY |
| 14 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 15 | Complexity | FIX_SLIDER | 1 | 20 | 1 | 10 | 6 | -- | 1 | -- |
| 16 | Sub Settings | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 17 | Sub Influence (%) | FIX_SLIDER | 1.5e-05 | 10000 | 25 | 100 | 70 | -- | 1 | -- |
| 18 | Sub Scaling | FIX_SLIDER | 10 | 10000 | 25 | 100 | 56 | -- | 1 | -- |
| 19 | Sub Rotation | ANGLE | 0.039062 | 0.063095 | 0.097656 | 0.390625 | 0.21875 | degrees | -- | -- |
| 20 | Sub Offset | POINT | -- | -- | -- | -- | (0%, 0%) of layer | -- | -- | -- |
| 21 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 22 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 23 | Evolution | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 24 | Evolution Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 25 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | SUPERVISE |
| 26 | Cycle (in Revolutions) | SLIDER | 1 | 88 | 1 | 30 | 1 | -- | -- | SUPERVISE |
| 27 | Random Seed | SLIDER | 0 | 100000 | 0 | 1000 | 0 | -- | -- | SUPERVISE |
| 28 | Random Seed | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 29 | Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | percent | 1 | COLLAPSE_TWIRLY |
| 30 | Blending Mode | POPUP | -- | -- | -- | -- | index 2 | -- | -- | COLLAPSE_TWIRLY |
| 31 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | COLLAPSE_TWIRLY |

Enumerated options for `Fractal Type` (1-based, default index 1): 1=Basic; 3=Turbulent Smooth; 4=Turbulent Basic; 5=Turbulent Sharp; 7=Dynamic; 8=Dynamic Progressive; 9=Dynamic Twist; 11=Max; 12=Smeary; 13=Swirly; 14=Rocky; 15=Cloudy; 16=Terrain; 17=Subscale; 18=Small Bumps; 19=Strings; 20=Threads

Enumerated options for `Noise Type` (1-based, default index 3): 1=Block; 2=Linear; 3=Soft Linear; 4=Spline

Enumerated options for `Overflow` (1-based, default index 4): 1=Clip; 2=Soft Clamp; 3=Wrap Back; 4=Allow HDR Results

Enumerated options for `Blending Mode` (1-based, default index 2): 1=None; 2=Normal; 4=Add; 5=Multiply; 6=Screen; 7=Overlay; 8=Soft Light; 9=Hard Light; 11=Color Dodge; 12=Color Burn; 14=Darken; 15=Lighten; 16=Difference; 17=Exclusion; 19=Hue; 20=Saturation; 21=Luminosity

**Median** -- `ADBE PS Median`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Radius | SLIDER | 0 | 255 | 0 | 10 | 0 | -- | -- | -- |
| 2 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Median (Legacy)** -- `ADBE Median`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Radius | SLIDER | 0 | 255 | 0 | 10 | 0 | -- | -- | -- |
| 2 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Noise** -- `ADBE Noise2`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Amount of Noise | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |
| 2 | Noise Type | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 3 | Clipping | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Noise Alpha** -- `ADBE Noise Alpha2`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Noise | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | Amount | FLOAT_SLIDER | 0 | 10000 | 0 | 100 | 0 | percent | 1 | -- |
| 3 | Original Alpha | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 4 | Overflow | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 5 | Contextual Control | ANGLE | 1.000015 | 0 | 0 | 0 | 0 | degrees | -- | -- |
| 6 | Noise Options (Animation) | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 7 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | SUPERVISE |
| 8 | Cycle (in Revolutions) | SLIDER | 1 | 88 | 1 | 30 | 1 | -- | -- | SUPERVISE |
| 9 | Cycle (in Revolutions) | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |

Enumerated options for `Noise` (1-based, default index 1): 1=Uniform Random; 2=Squared Random; 4=Uniform Animation; 5=Squared Animation

Enumerated options for `Original Alpha` (1-based, default index 2): 1=Add; 2=Clamp; 3=Scale; 4=Edges

Enumerated options for `Overflow` (1-based, default index 2): 1=Clip; 2=Wrap Back; 3=Wrap

#### Stylize

**Glass** -- `CC Glass`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Surface | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Bump Map | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 3 | Property | POPUP | -- | -- | -- | -- | index 6 | -- | -- | -- |
| 4 | Softness | FIX_SLIDER | 0 | 500 | 1 | 50 | 20 | -- | 1 | -- |
| 5 | Height | FIX_SLIDER | -100 | 100 | -50 | 50 | 25 | -- | 1 | -- |
| 6 | Displacement | FIX_SLIDER | -500 | 500 | -100 | 100 | 100 | -- | 1 | -- |
| 7 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 8 | Light | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 9 | Light Intensity | FIX_SLIDER | 0 | 1000 | 0 | 150 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 10 | Light Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 11 | Light Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 12 | Light Height | FIX_SLIDER | -100 | 100 | 0 | 100 | 65 | -- | 1 | COLLAPSE_TWIRLY |
| 13 | Light Position | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 14 | Light Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 15 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 16 | Shading | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 17 | Ambient | FIX_SLIDER | 0 | 200 | 0 | 100 | 50 | -- | 1 | -- |
| 18 | Diffuse | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | -- |
| 19 | Specular | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | -- | 1 | -- |
| 20 | Roughness | FIX_SLIDER | 0.000992 | 0.5 | 0.000992 | 0.25 | 0.024994 | -- | 3 | -- |
| 21 | Metal | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 22 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 23 | Using | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |

Enumerated options for `Property` (1-based, default index 6): 1=Red; 2=Green; 3=Blue; 4=Alpha; 5=Luminance; 6=Lightness

Enumerated options for `Light Type` (1-based, default index 1): 1=Distant Light; 2=Point Light

Enumerated options for `Using` (1-based, default index 1): 1=Effect Light; 2=AE Lights

**Repeat Tile** -- `CC RepeTile`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Expand Right | SLIDER | 0 | 20000 | 0 | 400 | 0 | -- | -- | -- |
| 2 | Expand Left | SLIDER | 0 | 20000 | 0 | 400 | 0 | -- | -- | -- |
| 3 | Expand Down | SLIDER | 0 | 20000 | 0 | 400 | 0 | -- | -- | -- |
| 4 | Expand Up | SLIDER | 0 | 20000 | 0 | 400 | 0 | -- | -- | -- |
| 5 | Tiling | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 6 | Blend Borders | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | percent | 1 | COLLAPSE_TWIRLY |

Enumerated options for `Tiling` (1-based, default index 1): 1=Repeat; 2=Checker Flip H; 3=Checker Flip V; 4=Unfold; 5=Checker 180 deg; 6=Checker Flip 45 deg; 7=Checker 90 deg CCW; 8=Checker 90 deg CW; 9=Rosette; 10=Random; 11=None; 13=Turn CW; 14=Turn CCW; 15=Twist; 16=Slide; 17=Brick

**Vignette** -- `CS Vignette`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Amount | FLOAT_SLIDER | -1000 | 1000 | -100 | 100 | 100 | -- | 1 | -- |
| 2 | Angle of View | FLOAT_SLIDER | 0 | 120 | 5 | 90 | 45 | -- | 1 | -- |
| 3 | Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 5 | Pin Highlights | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 0 | -- | 1 | -- |

**Color Emboss** -- `ADBE Color Emboss`  (GPU-accelerated)

Creates a raised embossed look using the clip's colors.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 2 | Relief | FIX_SLIDER | 0 | 1000 | 0 | 10 | 1 | -- | 2 | -- |
| 3 | Contrast | SLIDER | 0 | 32767 | 0 | 200 | 100 | -- | -- | -- |
| 4 | Blend With Original | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 0 | -- |

**Find Edges** -- `ADBE Find Edges`  (GPU-accelerated)

Highlights contours by detecting the edges in the image.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 2 | Blend With Original | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | percent | 0 | -- |

**Glow** -- `ADBE Glo2`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Glow Based On | POPUP | -- | -- | -- | -- | index 2 | -- | -- | SUPERVISE |
| 2 | Glow Threshold | FIX_SLIDER | 0 | 255 | 0 | 255 | 153 | percent | 1 | -- |
| 3 | Glow Radius | FIX_SLIDER | 0 | 1000 | 0 | 100 | 10 | -- | 1 | -- |
| 4 | Glow Intensity | FIX_SLIDER | 0 | 255 | 0 | 4 | 1 | -- | 1 | -- |
| 5 | Composite Original | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 6 | Glow Operation | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 7 | Glow Colors | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 8 | Color Looping | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 9 | Color Loops | FIX_SLIDER | 1 | 127 | 1 | 10 | 1 | -- | 1 | -- |
| 10 | Color Phase | ANGLE | 0.003906 | 0.496094 | 0.003906 | 0.039062 | 0.003906 | degrees | -- | -- |
| 11 | A & B Midpoint | FIX_SLIDER | 0.009995 | 0.990005 | 0.009995 | 0.990005 | 0.5 | percent | 0 | -- |
| 12 | Color A | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 13 | Color B | COLOR | -- | -- | -- | -- | ARGB #FF000000 | -- | -- | -- |
| 14 | Glow Dimensions | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Glow Based On` (1-based, default index 2): 1=Alpha Channel; 2=Color Channels

Enumerated options for `Composite Original` (1-based, default index 2): 1=On Top; 2=Behind; 3=None

Enumerated options for `Glow Operation` (1-based, default index 3): 1=None; 2=Normal; 3=Add; 4=Multiply; 5=Dissolve; 6=Screen; 7=Overlay; 8=Soft Light; 9=Hard Light; 10=Darken; 11=Lighten; 12=Difference; 13=Hue; 14=Saturation; 15=Color; 16=Luminosity; 17=Color Dodge; 18=Color Burn; 19=Exclusion; 20=Stencil Alpha; 21=Stencil Luma; 22=Silhouette Alpha; 23=Silhouette Luma; 24=Luminescent Premultiply; 25=Alpha Add

Enumerated options for `Glow Colors` (1-based, default index 1): 1=Original Colors; 2=A & B Colors; 3=Arbitrary Map

Enumerated options for `Color Looping` (1-based, default index 3): 1=Sawtooth A>B; 2=Sawtooth B>A; 3=Triangle A>B>A; 4=Triangle B>A>B

Enumerated options for `Glow Dimensions` (1-based, default index 1): 1=Horizontal and Vertical; 2=Horizontal; 3=Vertical

**Mosaic** -- `ADBE Mosaic`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Horizontal Blocks | SLIDER | 1 | 4000 | 1 | 200 | 10 | -- | -- | -- |
| 2 | Vertical Blocks | SLIDER | 1 | 4000 | 1 | 200 | 10 | -- | -- | -- |
| 3 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Motion Tile** -- `ADBE Tile`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Tile Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 2 | Tile Width | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 3 | Tile Height | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 4 | Output Width | FIX_SLIDER | 0 | 30000 | 0 | 1000 | 100 | -- | 1 | -- |
| 5 | Output Height | FIX_SLIDER | 0 | 30000 | 0 | 1000 | 100 | -- | 1 | -- |
| 6 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 7 | Phase | ANGLE | 0 | 30000 | 0 | 1000 | 100 | degrees | 1 | -- |
| 8 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Posterize** -- `ADBE Posterize`  (GPU-accelerated)

Reduces the number of tonal values to create a stylized look.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Level | FLOAT_SLIDER | 2 | 255 | 2 | 32 | 7 | -- | 0 | -- |

**Roughen Edges** -- `ADBE Roughen Edges`  (GPU-accelerated)

Distorts the edge of the image with a rough, irregular border.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Edge Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | Border | FIX_SLIDER | 0 | 500 | 0 | 32 | 8 | -- | 2 | -- |
| 3 | Edge Sharpness | FIX_SLIDER | 0 | 10 | 0 | 2 | 1 | -- | 2 | -- |
| 4 | Fractal Influence | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 5 | Scale | FIX_SLIDER | 10 | 1000 | 20 | 300 | 100 | -- | 1 | -- |
| 6 | Stretch Width or Height | FIX_SLIDER | -100 | 100 | -5 | 5 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 7 | Offset (Turbulence) | POINT | -- | -- | -- | -- | (0%, 0%) of layer | -- | -- | COLLAPSE_TWIRLY |
| 8 | Complexity | SLIDER | 1 | 10 | 1 | 6 | 2 | -- | -- | COLLAPSE_TWIRLY |
| 9 | Evolution | ANGLE | 1.5e-05 | 0.000153 | 1.5e-05 | 9.2e-05 | 3.1e-05 | degrees | 2 | -- |
| 10 | Edge Color | COLOR | -- | -- | -- | -- | ARGB #FF993300 | -- | -- | -- |
| 11 | Evolution Options | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 12 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | SUPERVISE |
| 13 | Cycle (in Revolutions) | SLIDER | 1 | 88 | 1 | 30 | 1 | -- | -- | SUPERVISE |
| 14 | Random Seed | SLIDER | 0 | 100000 | 0 | 1000 | 0 | -- | -- | SUPERVISE |
| 15 | Random Seed | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,SUPERVISE |

Enumerated options for `Edge Type` (1-based, default index 1): 1=Roughen; 2=Roughen Color; 3=Cut; 4=Spiky; 5=Rusty; 6=Rusty Color; 7=Photocopy; 8=Photocopy Color

**Threshold** -- `ADBE Threshold2`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Level | FLOAT_SLIDER | -30000 | 30000 | 0 | 1 | 0.5 | pixel | 0 | -- |

#### Simulation

**Mercury Particles** -- `CC Mr. Mercury`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Radius X | FIX_SLIDER | 0 | 1024 | 0 | 100 | 5 | -- | 1 | -- |
| 2 | Radius Y | FIX_SLIDER | 0 | 1024 | 0 | 100 | 5 | -- | 1 | -- |
| 3 | Producer | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 4 | Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 5 | Velocity | FIX_SLIDER | -1024 | 1024 | 0 | 5 | 1 | -- | 1 | -- |
| 6 | Birth Rate | FIX_SLIDER | 0 | 1024 | 0 | 5 | 1 | -- | 1 | -- |
| 7 | Longevity (sec) | FIX_SLIDER | 0 | 3000 | 0 | 6 | 2 | -- | 1 | CANNOT_TIME_VARY |
| 8 | Gravity | FIX_SLIDER | -1024 | 1024 | -2 | 2 | 1 | -- | 1 | -- |
| 9 | Resistance | FIX_SLIDER | -1024 | 1024 | 0 | 1 | 0 | -- | 2 | -- |
| 10 | Extra | FIX_SLIDER | -1024 | 1024 | 0 | 2 | 1 | -- | 1 | -- |
| 11 | Animation | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 12 | Blob Influence | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | percent | 1 | -- |
| 13 | Influence Map | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 14 | Blob Birth Size | FIX_SLIDER | 0 | 1024 | 0 | 2 | 0.149994 | -- | 2 | -- |
| 15 | Blob Death Size | FIX_SLIDER | 0 | 1024 | 0 | 2 | 0.75 | -- | 2 | -- |
| 16 | Light | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 17 | Light Intensity | FIX_SLIDER | 0 | 1000 | 0 | 150 | 100 | -- | 1 | COLLAPSE_TWIRLY |
| 18 | Light Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 19 | Light Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 20 | Light Height | FIX_SLIDER | -100 | 100 | 0 | 100 | 65 | -- | 1 | COLLAPSE_TWIRLY |
| 21 | Light Position | POINT | -- | -- | -- | -- | (25%, 25%) of layer | -- | -- | -- |
| 22 | Light Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 23 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 24 | Shading | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 25 | Ambient | FIX_SLIDER | 0 | 200 | 0 | 100 | 100 | -- | 1 | -- |
| 26 | Diffuse | FIX_SLIDER | 0 | 100 | 0 | 100 | 25 | -- | 1 | -- |
| 27 | Specular | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 28 | Roughness | FIX_SLIDER | 0.000992 | 0.5 | 0.000992 | 0.25 | 0.024994 | -- | 3 | -- |
| 29 | Metal | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |
| 30 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 31 | Using | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY,SUPERVISE |
| 32 | Material Opacity | FIX_SLIDER | 0 | 100 | 0 | 100 | 100 | -- | 1 | -- |

Enumerated options for `Animation` (1-based, default index 1): 1=Explosive; 2=Fractal Explosive; 3=Twirl; 4=Twirly; 5=Vortex; 6=Fire; 8=Direction; 9=Direction Normalized; 10=Bi-Directional; 11=Bi-Directional Normalized; 13=Jet; 14=Jet Sideways

Enumerated options for `Influence Map` (1-based, default index 3): 1=Blob out; 2=Blob in; 3=Blob in & out; 4=Blob out sharp; 5=Constant Blobs

Enumerated options for `Light Type` (1-based, default index 1): 1=Distant Light; 2=Point Light

Enumerated options for `Using` (1-based, default index 1): 1=Effect Light; 2=AE Lights

**Foam** -- `APC Foam`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | View | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 4 | Producer Point | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 5 | Producer | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 6 | Producer X Size | FIX_SLIDER | 0 | 0.449997 | 0 | 0.449997 | 0.029999 | -- | 3 | COLLAPSE_TWIRLY |
| 8 | Producer Y Size | FIX_SLIDER | 0 | 0.449997 | 0 | 0.449997 | 0.029999 | -- | 3 | COLLAPSE_TWIRLY |
| 10 | Producer Orientation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 12 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 14 | Production Rate | FIX_SLIDER | 0 | 100 | 0 | 10 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 16 | Size | FIX_SLIDER | 0.049988 | 4 | 0.049988 | 4 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 18 | Size Variance | FIX_SLIDER | 0 | 4 | 0 | 4 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 19 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 20 | Lifespan | FIX_SLIDER | 0 | 1000 | 0 | 1000 | 300 | -- | 3 | COLLAPSE_TWIRLY |
| 21 | Bubbles | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 22 | Bubble Growth Speed | FIX_SLIDER | 0.009995 | 1 | 0.009995 | 1 | 0.099991 | -- | 3 | COLLAPSE_TWIRLY |
| 24 | Strength | FIX_SLIDER | 0 | 100 | 0 | 100 | 10 | -- | 3 | COLLAPSE_TWIRLY |
| 26 | Initial Speed | FIX_SLIDER | 0 | 10 | 0 | 10 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 28 | Initial Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 30 | Wind Speed | FIX_SLIDER | 0 | 10 | 0 | 10 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 32 | Wind Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 33 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 34 | Turbulence | FIX_SLIDER | 0 | 2 | 0 | 2 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 35 | Physics | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 36 | Wobble Amount | FIX_SLIDER | 0 | 0.399994 | 0 | 0.399994 | 0.049988 | -- | 3 | COLLAPSE_TWIRLY |
| 38 | Repulsion | FIX_SLIDER | 0 | 1 | 0 | 1 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 40 | Pop Velocity | FIX_SLIDER | 0 | 10 | 0 | 10 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 42 | Viscosity | FIX_SLIDER | 0 | 4 | 0 | 4 | 0.099991 | -- | 3 | COLLAPSE_TWIRLY |
| 44 | Stickiness | FIX_SLIDER | 0 | 4 | 0 | 4 | 0.75 | -- | 3 | COLLAPSE_TWIRLY |
| 46 | Zoom | FIX_SLIDER | 0.00499 | 10 | 0.00499 | 5 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 48 | Universe Size | FIX_SLIDER | 0.099991 | 50 | 0.099991 | 5 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 50 | Blend Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 52 | Bubble Texture | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| 54 | Bubble Texture Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 56 | Bubble Orientation | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 57 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 58 | Environment Map | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 60 | Reflection Strength | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 62 | Reflection Convergence | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.799988 | -- | 3 | COLLAPSE_TWIRLY |
| 63 | Rendering | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 64 | Flow Map | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 66 | Flow Map Steepness | FIX_SLIDER | 0 | 100 | 0 | 5 | 5 | -- | 3 | COLLAPSE_TWIRLY |
| 68 | Flow Map Fits | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 70 | Simulation Quality | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 72 | Random Seed | SLIDER | 0 | 16 | 0 | 16 | 1 | -- | -- | COLLAPSE_TWIRLY |
| 79 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 81 | Flow Map | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 89 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `View` (1-based, default index 1): 1=Draft; 2=Draft + Flow Map; 3=Rendered

Enumerated options for `Blend Mode` (1-based, default index 1): 1=Transparent; 2=Solid Old on top; 3=Solid New on top

Enumerated options for `Bubble Texture` (1-based, default index 3): 1=User Defined; 3=Default Bubble; 4=Amber Bock; 5=Water Beads; 6=Spit; 7=Cartoon Coffee; 8=Winter Stream; 9=Soda Water; 10=Orange Soda; 11=Nuclear Waste; 12=Red Tide; 13=Magma Marbles; 14=Sunset Foam; 15=Pepto; 16=Algae; 17=Blisters; 18=Bubble Wrap; 19=Grape Soda

Enumerated options for `Bubble Orientation` (1-based, default index 1): 1=Fixed; 2=Physical Orientation; 3=Bubble Velocity

Enumerated options for `Flow Map Fits` (1-based, default index 1): 1=Universe; 2=Screen

Enumerated options for `Simulation Quality` (1-based, default index 1): 1=Normal; 2=High; 3=Intense

**Wave World** -- `APC Wave World`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | View | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| 4 | Horizontal Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 5 | Wireframe Controls | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 6 | Vertical Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 8 | Vertical Scale | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 10 | Brightness | FIX_SLIDER | -5 | 5 | 0 | 1 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 12 | Contrast | FIX_SLIDER | 0 | 2 | 0 | 1 | 0.25 | -- | 3 | COLLAPSE_TWIRLY |
| 13 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 14 | Gamma Adjustment | FIX_SLIDER | 0.299988 | 3 | 0.299988 | 3 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 15 | Height Map Controls | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 16 | Render Dry Areas As | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 18 | Transparency | FIX_SLIDER | 0 | 1 | 0 | 0.099991 | 0.009995 | -- | 3 | COLLAPSE_TWIRLY |
| 20 | Reflect Edges | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 24 | Pre-roll (seconds) | FIX_SLIDER | 0 | 60 | 0 | 5 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 26 | Grid Resolution | SLIDER | 10 | 1000 | 20 | 300 | 40 | -- | -- | COLLAPSE_TWIRLY |
| 27 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 29 | Simulation | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 30 | Wave Speed | FIX_SLIDER | 0.009995 | 10 | 0.009995 | 1 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 32 | Damping | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.009995 | -- | 3 | COLLAPSE_TWIRLY |
| 36 | Ground | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 38 | Steepness | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.25 | -- | 3 | COLLAPSE_TWIRLY |
| 40 | Height | FIX_SLIDER | 0 | 1 | 0 | 1 | 0.299988 | -- | 3 | COLLAPSE_TWIRLY |
| 42 | Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 43 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 44 | Position | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 45 | Ground | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 46 | Height/Length | FIX_SLIDER | 0.000992 | 100 | 0.000992 | 1 | 0.099991 | -- | 3 | COLLAPSE_TWIRLY |
| 48 | Width | FIX_SLIDER | 0.000992 | 100 | 0.000992 | 1 | 0.099991 | -- | 3 | COLLAPSE_TWIRLY |
| 50 | Angle | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 52 | Amplitude | FIX_SLIDER | 0 | 5 | 0 | 1 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 54 | Frequency | FIX_SLIDER | 0 | 20 | 0 | 3 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 55 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 56 | Phase | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 57 | Producer 1 | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 58 | Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 60 | Position | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 62 | Height/Length | FIX_SLIDER | 0.000992 | 100 | 0.000992 | 1 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 64 | Width | FIX_SLIDER | 0.000992 | 100 | 0.000992 | 1 | 0.5 | -- | 3 | COLLAPSE_TWIRLY |
| 66 | Angle | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 68 | Amplitude | FIX_SLIDER | 0 | 5 | 0 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 70 | Frequency | FIX_SLIDER | 0 | 20 | 0 | 3 | 1 | -- | 3 | COLLAPSE_TWIRLY |
| 72 | Phase | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 75 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 77 | Producer 2 | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 90 | Wave Strength | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 3 | COLLAPSE_TWIRLY |
| 92 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 95 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `View` (1-based, default index 2): 1=Height Map; 2=Wireframe Preview

Enumerated options for `Render Dry Areas As` (1-based, default index 1): 1=Solid; 2=Transparent

Enumerated options for `Reflect Edges` (1-based, default index 1): 1=None; 2=All; 3=Left; 4=Top; 5=Right; 6=Bottom; 7=Left & Top; 8=Right & Left; 9=Right & Top; 10=Bottom & Left; 11=Bottom & Top; 12=Bottom & Right; 13=Right & Left & Top; 14=Bottom & Left & Top; 15=Bottom & Right & Left; 16=Bottom & Right & Top

Enumerated options for `Type` (1-based, default index 1): 1=Ring; 2=Line

Enumerated options for `Type` (1-based, default index 1): 1=Ring; 2=Line

#### Time

**Echo** -- `ADBE Echo`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Echo Time (seconds) | FLOAT_SLIDER | -30000 | 30000 | -5 | 5 | -0.033333 | -- | 3 | -- |
| 2 | Number Of Echoes | SLIDER | 0 | 30000 | 0 | 10 | 1 | -- | -- | -- |
| 3 | Starting Intensity | FLOAT_SLIDER | 0 | 1 | 0 | 1 | 1 | -- | 2 | -- |
| 4 | Decay | FLOAT_SLIDER | 0 | 100 | 0 | 1 | 1 | -- | 2 | -- |
| 5 | Echo Operator | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Echo Operator` (1-based, default index 1): 1=Add; 2=Maximum; 3=Minimum; 4=Screen; 5=Composite In Back; 6=Composite In Front; 7=Blend

**Posterize Time** -- `ADBE Posterize Time`  (GPU-accelerated)

Reduces the frame rate to create stepped, stylized motion.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Frame Rate | FIX_SLIDER | 0.009995 | 999 | 1 | 64 | 24 | -- | 1 | CANNOT_INTERP |

**Time Difference** -- `ADBE Difference`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Target | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Time Offset (sec) | FIX_SLIDER | -2000 | 2000 | -1 | 1 | 0 | -- | 3 | -- |
| 3 | Contrast | FIX_SLIDER | -10000 | 10000 | -100 | 100 | 50 | -- | 1 | -- |
| 4 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 5 | Alpha Channel | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |

Enumerated options for `Alpha Channel` (1-based, default index 1): 1=Original; 2=Target; 3=Blend; 4=Max; 5=Full On; 7=Lightness of Result; 8=Max of Result; 10=Alpha Difference; 11=Alpha Difference Only

#### Transition

**Block Dissolve** -- `ADBE Block Dissolve`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Transition Completion | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 0 | -- |
| 2 | Block Width | FIX_SLIDER | 1 | 32000 | 1 | 127 | 0 | -- | 1 | -- |
| 3 | Block Height | FIX_SLIDER | 1 | 32000 | 1 | 127 | 0 | -- | 1 | -- |
| 4 | Feather | FIX_SLIDER | 0 | 32000 | 0 | 100 | 0 | -- | 1 | -- |
| 5 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Card Wipe** -- `APC CardWipeCam`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 2 | Transition Completion | FIX_SLIDER | 0 | 100 | 0 | 100 | 25 | percent | 0 | -- |
| 4 | Transition Width | FIX_SLIDER | 0 | 100 | 0 | 100 | 50 | percent | 0 | -- |
| 6 | Back Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 8 | Rows & Columns | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 10 | Rows | SLIDER | 1 | 1000 | 1 | 250 | 9 | -- | -- | COLLAPSE_TWIRLY |
| 12 | Columns | SLIDER | 1 | 1000 | 1 | 250 | 12 | -- | -- | COLLAPSE_TWIRLY |
| 14 | Card Scale | FIX_SLIDER | 0 | 10 | 0 | 1 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 16 | Flip Axis | POPUP | -- | -- | -- | -- | index 0 | -- | -- | -- |
| 18 | Flip Direction | POPUP | -- | -- | -- | -- | index 0 | -- | -- | -- |
| 20 | Flip Order | POPUP | -- | -- | -- | -- | index 0 | -- | -- | -- |
| 22 | Gradient Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 24 | Timing Randomness | FIX_SLIDER | 0 | 1 | 0 | 1 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 26 | Random Seed | SLIDER | 1 | 1000 | 1 | 10 | 1 | -- | -- | COLLAPSE_TWIRLY |
| 28 | Camera System | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 30 | X Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 32 | Y Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 34 | Z Rotation | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | COLLAPSE_TWIRLY |
| 36 | X,Y Position | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 38 | Z Position | FIX_SLIDER | -1000 | 1000 | 0.099991 | 10 | 2 | -- | 2 | -- |
| 40 | Focal Length | FIX_SLIDER | 10 | 1000 | 20 | 300 | 70 | -- | 2 | -- |
| 42 | Transform Order | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 44 | Upper Left Corner | POINT | -- | -- | -- | -- | (0%, 0%) of layer | -- | -- | -- |
| 46 | Upper Right Corner | POINT | -- | -- | -- | -- | (100%, 0%) of layer | -- | -- | -- |
| 47 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 48 | Lower Left Corner | POINT | -- | -- | -- | -- | (0%, 100%) of layer | -- | -- | -- |
| 50 | Lower Right Corner | POINT | -- | -- | -- | -- | (100%, 100%) of layer | -- | -- | -- |
| 52 | _(unnamed)_ | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| 54 | Focal Length | FIX_SLIDER | 10 | 1000 | 20 | 300 | 70 | -- | 2 | -- |
| 56 | Light Intensity | FIX_SLIDER | 0 | 50 | 0 | 5 | 1 | -- | 2 | -- |
| 58 | Light Color | COLOR | -- | -- | -- | -- | ARGB #FFFFFFFF | -- | -- | -- |
| 60 | Light Position | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 62 | Light Depth | FIX_SLIDER | -100 | 100 | -5 | 5 | 1 | -- | 3 | -- |
| 63 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 64 | Light Type | POPUP | -- | -- | -- | -- | index 2 | -- | -- | SUPERVISE |
| 66 | Ambient Light | FIX_SLIDER | 0 | 2 | 0 | 2 | 0.25 | -- | 2 | -- |
| 68 | Diffuse Reflection | FIX_SLIDER | 0 | 2 | 0 | 2 | 0.75 | -- | 2 | -- |
| 70 | Specular Reflection | FIX_SLIDER | 0 | 2 | 0 | 2 | 0 | -- | 3 | -- |
| 72 | Highlight Sharpness | FIX_SLIDER | 0 | 100 | 0 | 50 | 10 | -- | 2 | -- |
| 74 | X Jitter Amount | FIX_SLIDER | 0 | 5 | 0 | 1 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 76 | X Jitter Speed | FIX_SLIDER | 0 | 1000 | 0 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 78 | Y Jitter Amount | FIX_SLIDER | 0 | 5 | 0 | 1 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 79 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 80 | Y Jitter Speed | FIX_SLIDER | 0 | 1000 | 0 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 82 | Z Jitter Amount | FIX_SLIDER | 0 | 25 | 0 | 1 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 84 | Z Jitter Speed | FIX_SLIDER | 0 | 1000 | 0 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 86 | X Rot Jitter Amount | FIX_SLIDER | 0 | 360 | 0 | 90 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 88 | X Rot Jitter Speed | FIX_SLIDER | 0 | 1000 | 0 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 89 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 90 | Y Rot Jitter Amount | FIX_SLIDER | 0 | 360 | 0 | 90 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 91 | Position Jitter | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 92 | Y Rot Jitter Speed | FIX_SLIDER | 0 | 1000 | 0 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 94 | Z Rot Jitter Amount | FIX_SLIDER | 0 | 360 | 0 | 90 | 0 | -- | 2 | COLLAPSE_TWIRLY |
| 96 | Z Rot Jitter Speed | FIX_SLIDER | 0 | 1000 | 0 | 10 | 1 | -- | 2 | COLLAPSE_TWIRLY |
| 98 | Camera Position | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 100 | Corner Pins | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 102 | Lighting | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 104 | Material | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 105 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 107 | Rotation Jitter | GROUP_START | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY,COLLAPSE_TWIRLY |
| 121 | _(unnamed)_ | GROUP_END | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

Enumerated options for `Rows & Columns` (1-based, default index 1): 1=Independent; 2=Columns Follows Rows

Enumerated options for `Flip Axis` (1-based, default index 0): 1=X; 2=Y; 3=Random

Enumerated options for `Flip Direction` (1-based, default index 0): 1=Positive; 2=Negative; 3=Random

Enumerated options for `Flip Order` (1-based, default index 0): 1=Left to Right; 2=Right to Left; 3=Top to Bottom; 4=Bottom to Top; 5=Top Left to Bottom Right; 6=Top Right to Bottom Left; 7=Bottom Left to Top Right; 8=Bottom Right to Top Left; 9=Gradient

Enumerated options for `Camera System` (1-based, default index 1): 1=Camera Position; 2=Corner Pins; 3=Comp Camera

Enumerated options for `Transform Order` (1-based, default index 1): 1=Rotate XYZ, Position; 2=Rotate XZY, Position; 3=Rotate YXZ, Position; 4=Rotate YZX, Position; 5=Rotate ZXY, Position; 6=Rotate ZYX, Position; 7=Position, Rotate XYZ; 8=Position, Rotate XZY; 9=Position, Rotate YXZ; 10=Position, Rotate YZX; 11=Position, Rotate ZXY; 12=Position, Rotate ZYX

Enumerated options for `Light Type` (1-based, default index 2): 1=Point Source; 2=Distant Source; 3=First Comp Light

**Linear Wipe** -- `ADBE Linear Wipe`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Transition Completion | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 0 | -- |
| 2 | Wipe Angle | ANGLE | 0 | 100 | 0 | 100 | 0 | degrees | 0 | -- |
| 3 | Feather | FIX_SLIDER | 0 | 32000 | 0 | 100 | 0 | -- | 1 | -- |

**Radial Wipe** -- `ADBE Radial Wipe`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Transition Completion | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 0 | -- |
| 2 | Start Angle | ANGLE | 0 | 100 | 0 | 100 | 0 | degrees | 0 | -- |
| 3 | Wipe Center | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |
| 4 | Wipe | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| 5 | Feather | FIX_SLIDER | 0 | 32000 | 0 | 100 | 0 | -- | 1 | -- |

Enumerated options for `Wipe` (1-based, default index 1): 1=Clockwise; 2=Counterclockwise; 3=Both

**Venetian Blinds** -- `ADBE Venetian Blinds`  (GPU-accelerated)

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Transition Completion | FIX_SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 0 | -- |
| 2 | Direction | ANGLE | 0 | 0 | 0 | 0 | 0 | degrees | 0 | -- |
| 3 | Width | FIX_SLIDER | 1 | 32000 | 1 | 127 | 20 | -- | 0 | -- |
| 4 | Feather | FIX_SLIDER | 0 | 32000 | 0 | 100 | 0 | -- | 1 | -- |

#### Expression Controls

**Angle Control** -- `ADBE Angle Control`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Angle | ANGLE | 0 | -1.5e-05 | 0 | 0 | 0 | degrees | 0 | -- |

**Checkbox Control** -- `ADBE Checkbox Control`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Checkbox | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Color Control** -- `ADBE Color Control`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Color | COLOR | -- | -- | -- | -- | ARGB #FFFF0000 | -- | -- | -- |

**Layer Control** -- `ADBE Layer Control`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |

**Point Control** -- `ADBE Point Control`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Point | POINT | -- | -- | -- | -- | (50%, 50%) of layer | -- | -- | -- |

**Slider Control** -- `ADBE Slider Control`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Slider | FLOAT_SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |

#### Utility

**Grow Bounds** -- `ADBE GROW BOUNDS`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Pixels | FLOAT_SLIDER | 0 | 10000 | 0 | 100 | 10 | -- | 0 | -- |

**HDR Compander** -- `ADBE Compander`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Mode: | POPUP | -- | -- | -- | -- | index 1 | -- | -- | SUPERVISE |
| 2 | Gain | FLOAT_SLIDER | 0.001 | 100 | 0.001 | 20 | 1 | -- | 2 | -- |
| 3 | Gamma | FLOAT_SLIDER | 0.1 | 10 | 0.1 | 10 | 1 | -- | 2 | -- |

Enumerated options for `Mode:` (1-based, default index 1): 1=Compress Range; 2=Expand Range

#### Audio

**Compressor** -- `ADBE Aud Compressor`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Threshold (dB) | FLOAT_SLIDER | -60 | 0 | -60 | 0 | -16 | -- | 1 | -- |
| 2 | Ratio (x:1) | FLOAT_SLIDER | 0.4 | 30 | 0.4 | 30 | 3 | -- | 1 | -- |
| 3 | Knee (db) | FLOAT_SLIDER | 0 | 30 | 0 | 30 | 15 | -- | 0 | -- |
| 4 | Attack (ms) | FLOAT_SLIDER | 0 | 400 | 0 | 400 | 6 | -- | 0 | -- |
| 5 | Release (ms) | FLOAT_SLIDER | 1 | 4000 | 1 | 4000 | 440 | -- | 0 | COLLAPSE_TWIRLY |
| 6 | Auto Release | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| 7 | Makeup Gain (dB) | FLOAT_SLIDER | -30 | 30 | -30 | 30 | 0 | -- | 1 | -- |
| 8 | Output Limit (dB) | FLOAT_SLIDER | -30 | 0 | -30 | 0 | -1 | -- | 1 | -- |

**Modulator** -- `ADBE Aud Modulator`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Modulation Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY |
| 2 | Modulation Rate | FLOAT_SLIDER | 0 | 1000 | 0 | 20 | 10 | -- | 2 | -- |
| 3 | Modulation Depth | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 2 | percent | 2 | -- |
| 4 | Amplitude Modulation | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 50 | percent | 2 | -- |

Enumerated options for `Modulation Type` (1-based, default index 1): 1=Sine; 2=Triangle

**Reverb** -- `ADBE Aud Reverb`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Reverb Time (ms) | FLOAT_SLIDER | 0 | 5000 | 0 | 300 | 100 | -- | 2 | CANNOT_TIME_VARY |
| 2 | Diffusion | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 75 | percent | 2 | -- |
| 3 | Decay | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 25 | percent | 2 | -- |
| 4 | Brightness | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 10 | percent | 2 | -- |
| 5 | Dry Out | FLOAT_SLIDER | 0 | 400 | 0 | 100 | 90 | percent | 2 | -- |
| 6 | Wet Out | FLOAT_SLIDER | 0 | 400 | 0 | 100 | 10 | percent | 2 | -- |

**Tone** -- `ADBE Aud Tone`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | _(unnamed)_ | LAYER | -- | -- | -- | -- | -- | -- | -- | CANNOT_TIME_VARY |
| 1 | Waveform options | POPUP | -- | -- | -- | -- | index 1 | -- | -- | CANNOT_TIME_VARY |
| 2 | Frequency 1 | FLOAT_SLIDER | 0 | 30000 | 0 | 3000 | 440 | -- | 2 | COLLAPSE_TWIRLY |
| 3 | Frequency 2 | FLOAT_SLIDER | 0 | 30000 | 0 | 3000 | 493.679993 | -- | 2 | COLLAPSE_TWIRLY |
| 4 | Frequency 3 | FLOAT_SLIDER | 0 | 30000 | 0 | 3000 | 554.400024 | -- | 2 | COLLAPSE_TWIRLY |
| 5 | Frequency 4 | FLOAT_SLIDER | 0 | 30000 | 0 | 3000 | 587.400024 | -- | 2 | COLLAPSE_TWIRLY |
| 6 | Frequency 5 | FLOAT_SLIDER | 0 | 30000 | 0 | 3000 | 659.119995 | -- | 2 | COLLAPSE_TWIRLY |
| 7 | Level | FLOAT_SLIDER | 0 | 100 | 0 | 100 | 20 | percent | 2 | -- |

Enumerated options for `Waveform options` (1-based, default index 1): 1=Sine; 2=Triangle; 3=Saw; 4=Square; 5=White Noise

#### Pseudo-effect (preset-only)

**2D Text Box** -- `Pseudo/ADBE 2D Text Box`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Measure Text | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| -- | Measure At Time [sec] | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Scaling | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Padding | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Offset | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Round Corners | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |

Enumerated options for `Measure Text` (1-based, default index 2): 1=At Single Time; 2=During Whole Comp; 3=After Time; 4=Before Time

**Animated Shape Control** -- `ADBE CM Animated Shape 3`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Speed | SLIDER | -10000 | 10000 | 0 | 100 | 10 | -- | -- | -- |
| -- | Random Seed | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |

**Animated Shape Control** -- `ADBE CM Animated Shape Control`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Speed | SLIDER | -10000 | 10000 | 0 | 100 | 10 | -- | -- | -- |
| -- | Dimensions | POINT | -- | -- | -- | -- | (0.5, 0.5) fraction of layer | -- | -- | -- |
| -- | Rounding | SLIDER | -10000 | 10000 | 0 | 100 | 0 | -- | -- | -- |
| -- | Spread | SLIDER | 0 | 10000 | 0 | 100 | 10 | -- | -- | -- |
| -- | Random Seed | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |

**Autoscroll - horizontal** -- `ADBE CM AutoscrollHorizontal`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Speed (pixels/second) | SLIDER | -10000 | 10000 | -1000 | 1000 | 100 | -- | -- | -- |

**Autoscroll - vertical** -- `ADBE CM AutoscrollVertical`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Speed (pixels/second) | SLIDER | -10000 | 10000 | -1000 | 1000 | -25 | -- | -- | -- |

**Bounce** -- `ADBE DE Bounce`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Bounce At Marker** -- `ADBE DE Bounce At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |

**Bounce On Beat** -- `ADBE DE Bounce On Beat`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |
| -- | Audio Keyframe Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Audio Threshold | SLIDER | 0 | 100 | -- | -- | 15 | -- | -- | -- |

**Bounce Random** -- `ADBE DE Bounce Random`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |
| -- | Min Delay | SLIDER | 0.1 | 100 | -- | 10 | 0.5 | -- | -- | -- |
| -- | Max Delay | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |

**Card Wipe Master Control** -- `ADBE CM TransCard`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | reverse direction | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Chaser Control** -- `ADBE CM Animated Shape 2`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Box Step Speed | SLIDER | -10000 | 10000 | 0 | 100 | 10 | -- | -- | -- |
| -- | Size | POINT | -- | -- | -- | -- | (0.5, 0.5) fraction of layer | -- | -- | -- |
| -- | Spread | SLIDER | 1 | 1000 | 1 | 100 | 10 | -- | -- | -- |
| -- | Elements | SLIDER | 1 | 100 | -- | -- | 0 | -- | 0 | -- |

**Color Swirl** -- `ADBE Color Swirl`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | -- |
| -- | Final Color | COLOR | -- | -- | -- | -- | RGB [20, 192, 50] | -- | -- | -- |

**Corner Reveal** -- `ADBE CM CornerReveal`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | right (not left) | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | bottom (not top) | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Feather | SLIDER | 0 | 1000 | 0 | 100 | 0 | -- | -- | -- |

**Counter Controls** -- `Pseudo/ADBE Counter Controls`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Animate Value | SLIDER | -1000000 | 1000000 | 0 | 100 | 100 | -- | 2 | -- |
| -- | 10x Value Multiplier | SLIDER | -1000000 | 1000000 | 0 | 100 | 2 | -- | 0 | -- |
| -- | Pad Zeros | SLIDER | -1000000 | 1000000 | 0 | 100 | 4 | -- | 0 | -- |
| -- | Digits After Decimal | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 0 | -- |
| -- | Formatting | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Number Format | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| -- | Show Original Text | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |
| -- | Grouping | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Show Group Separators | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Group Spacing | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Group Separator Offset | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Align Group Spacing | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |

Enumerated options for `Number Format` (1-based, default index 1): 1=1,234,567.01; 2=1.234.567,01; 3=1 234 567,01

Enumerated options for `Show Original Text` (1-based, default index 3): 1=Before Numbers; 2=After Numbers; 3=Hide Original Text

Enumerated options for `Align Group Spacing` (1-based, default index 3): 1=Left; 2=Right; 3=Center

**Cracked Tiles** -- `ADBE CM CrackedTiles`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Tile Cracking | SLIDER | 0 | 100 | -- | -- | 33 | percent | -- | -- |
| -- | Tiles Across | SLIDER | 2 | 2000 | 2 | 200 | 50 | -- | -- | -- |

**Crop Edges** -- `ADBE CM CropEdges`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Crop Amount (per edge) | SLIDER | 0 | 50 | -- | -- | 5 | percent | -- | -- |
| -- | Feather | SLIDER | 0 | 1000 | 0 | 100 | 0 | -- | -- | -- |

**Currency Controls** -- `Pseudo/ADBE Currency Controls`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Animate Value | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | 10x Value Multiplier | SLIDER | -1000000 | 1000000 | 0 | 100 | 3 | -- | 0 | -- |
| -- | Pad Zeros | SLIDER | -1000000 | 1000000 | 0 | 100 | 5 | -- | 0 | -- |
| -- | Digits After Decimal | SLIDER | -1000000 | 1000000 | 0 | 100 | 2 | -- | 0 | -- |
| -- | Formatting | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Currency Format | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| -- | Currency Symbol | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| -- | Use Currency Code | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Currency Symbol Position | POINT | -- | -- | -- | -- | (0, 0) fraction of layer | -- | -- | -- |
| -- | Currency Symbol Scale | SLIDER | -1000000 | 1000000 | 0 | 100 | 100 | percent | 1 | -- |
| -- | Grouping | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Show Group Separators | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Group Spacing | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Group Separator Offset | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Align Group Spacing | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |

Enumerated options for `Currency Format` (1-based, default index 1): 1=$1,234,567.01; 2=1.234.567,01 $; 3=1 234 567,01 $

Enumerated options for `Currency Symbol` (1-based, default index 1): 1=$ USD; 2=€ EUR; 3=¥ JPY; 4=£ GBP

Enumerated options for `Align Group Spacing` (1-based, default index 3): 1=Left; 2=Right; 3=Center

**Dissolve - unmelt** -- `ADBE CM DissolveUnmelt`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Maximum Distortion | SLIDER | -10000 | 10000 | 0 | 5000 | 100 | -- | -- | -- |

**Dissolve Master Control** -- `ADBE CM TransDissolve`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |

**Drift Over Time** -- `ADBE CM Throw`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Direction | ANGLE | -- | -- | -- | -- | 135 | degrees | -- | -- |
| -- | Speed (pixels/second) | SLIDER | -10000 | 10000 | -1000 | 1000 | 50 | -- | -- | -- |

**Face Measurements** -- `Pseudo/ADBE Animal Head14`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Face Offset | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Face Scale | SLIDER | 0 | 30000 | 0 | 500 | 100 | percent | 1 | -- |
| -- | Face Orientation | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Left Eye | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Right Eye | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Mouth | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |

**Fade In+Out - frames** -- `ADBE CM FadeInOutFrames`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Fade In Duration (frames) | SLIDER | 0 | 1000 | -- | -- | 15 | -- | -- | -- |
| -- | Fade Out Duration (frames) | SLIDER | 0 | 1000 | -- | -- | 15 | -- | -- | -- |

**Fade In+Out - msec** -- `ADBE CM FadeInOutmsec`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Fade In Duration (msec) | SLIDER | 0 | 30000 | 0 | 10000 | 500 | -- | -- | -- |
| -- | Fade Out Duration (msec) | SLIDER | 0 | 30000 | 0 | 10000 | 500 | -- | -- | -- |

**Fade Master Control** -- `ADBE CM TransFade`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |

**Fly to Inset** -- `ADBE CM FlyToInset`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Target Scale | SLIDER | 0 | 100 | -- | -- | 33.3 | percent | -- | -- |
| -- | Target Position | POINT | -- | -- | -- | -- | (0.733, 0.267) fraction of layer | -- | -- | -- |
| -- | Frame Size | SLIDER | 0 | 100 | 0 | 30 | 8 | -- | -- | -- |
| -- | Frame Color | COLOR | -- | -- | -- | -- | RGB [229, 229, 229] | -- | -- | -- |

**Follow** -- `ADBE DE Follow`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Delay | SLIDER | 0 | 100 | -- | 10 | 0.1 | -- | 2 | -- |
| -- | Leader | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Position | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Rotation | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Scale | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Opacity | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Based On Index | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Getting Jiggy** -- `ADBE Getting Jiggy`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Oh Boy 1 | SLIDER | -50 | 50 | -- | -- | 5 | -- | -- | -- |
| -- | Oh Boy 2 | SLIDER | -50 | 50 | 2 | 8 | 5 | -- | -- | -- |
| -- | Oh Boy Percent | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Oh Boy Pixel | SLIDER | 0 | 1 | -- | -- | 0.5 | pixel | -- | -- |
| -- | Follow Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Hopscotch | COLOR | -- | -- | -- | -- | RGB [128, 64, 192] | -- | -- | -- |
| -- | Center Point | POINT | -- | -- | -- | -- | (0.5, 0.5) fraction of layer | -- | -- | -- |
| -- | Left Edge Point | POINT | -- | -- | -- | -- | (0, 0.5) fraction of layer | -- | -- | -- |
| -- | Jig | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Funky Chicken | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |

**Grid Wipe** -- `ADBE CM GridWipe`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Cell Size | SLIDER | 1 | 2000 | -- | -- | 20 | -- | -- | -- |
| -- | Grid Angle | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | Feather | SLIDER | 0 | 1000 | 0 | 100 | 0 | -- | -- | -- |

**Inset Video - framed** -- `ADBE CM InsetVideoFramed`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Crop Amount | SLIDER | 0 | 50 | -- | -- | 5 | percent | -- | -- |
| -- | Frame Size | SLIDER | 0 | 100 | 0 | 30 | 8 | -- | -- | -- |
| -- | Frame Color | COLOR | -- | -- | -- | -- | RGB [229, 229, 229] | -- | -- | -- |

**Inset Video - torn edges** -- `ADBE CM InsetVideoTorn`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Crop Amount | SLIDER | 0 | 50 | -- | -- | 5 | percent | -- | -- |

**Iris Wipe Master Controls** -- `ADBE CM TransIris`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Invert Alpha | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Feather | SLIDER | 0 | 1000 | 0 | 100 | 0 | -- | -- | -- |

**Jiggle** -- `ADBE DE Jiggle`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | 0 | 100 | -- | 100 | 50 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Jiggle At Marker** -- `ADBE DE Jiggle At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | 0 | 100 | -- | 100 | 50 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |

**Jiggle On Beat** -- `ADBE DE Jiggle On Beat`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | 0 | 100 | -- | 100 | 50 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Audio Keyframe Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Audio Threshold | SLIDER | 0 | 100 | -- | -- | 15 | -- | -- | -- |

**Jiggle Random** -- `ADBE DE Jiggle Random`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | 0 | 100 | -- | 100 | 50 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Min Delay | SLIDER | 0.1 | 100 | -- | 10 | 0.5 | -- | -- | -- |
| -- | Max Delay | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |

**Light Leaks - layer markers** -- `ADBE CM LightLeaksMarkers`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Flash Width (msec) | SLIDER | 1 | 10000 | 1 | 1000 | 200 | -- | -- | -- |

**Light Leaks - random** -- `ADBE CM LightLeaksRandom`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Chance of Flashing | SLIDER | 0 | 100 | -- | -- | 100 | percent | -- | -- |
| -- | Flash Nervousness | SLIDER | 1 | 10000 | 1 | 500 | 50 | -- | -- | -- |

**Mask Fade Controls** -- `ADBE CM TransFadeMask`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Feather | SLIDER | 0 | 1000 | 0 | 100 | 0 | -- | -- | -- |

**Mood Lighting - amorphous** -- `ADBE CM MoodLightAmorph`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Evolution Speed | SLIDER | -1000 | 1000 | -500 | 500 | 150 | -- | -- | -- |
| -- | Cloud Size | SLIDER | 1 | 5000 | 10 | 500 | 75 | -- | -- | -- |
| -- | Intensity | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |

**Mood Lighting - digital** -- `ADBE CM MoodLightDigital`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Evolution Speed | SLIDER | -1000 | 1000 | -500 | 500 | 100 | -- | -- | -- |
| -- | Block Size | SLIDER | 1 | 5000 | 20 | 1000 | 250 | -- | -- | -- |
| -- | Intensity | SLIDER | 0 | 100 | -- | -- | 75 | percent | -- | -- |

**Mood Lighting - streaks** -- `ADBE CM MoodLightStreaks`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Evolution Speed | SLIDER | -1000 | 1000 | -500 | 500 | 200 | -- | -- | -- |
| -- | Streak Width | SLIDER | 1 | 1000 | 10 | 500 | 75 | -- | -- | -- |
| -- | Intensity | SLIDER | 0 | 100 | -- | -- | 75 | percent | -- | -- |

**Opacity Flash - layer markers** -- `ADBE CM OpacityFlashMarkers`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Flash Width (msec) | SLIDER | 1 | 10000 | 1 | 2000 | 500 | -- | -- | -- |

**Opacity Flash - random** -- `ADBE CM OpacityFlashRandom`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Chance of Flashing | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Flash Nervousness | SLIDER | 1 | 10000 | 1 | 500 | 50 | -- | -- | -- |

**Opacity Pulse** -- `ADBE DE Opacity Pulse`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Min Opacity | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Max Opacity | SLIDER | 0 | 100 | -- | -- | 100 | -- | -- | -- |
| -- | Attack | SLIDER | 0 | 25 | -- | 15 | 10 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 5 | -- | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0.01 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Opacity Pulse At Marker** -- `ADBE DE Opacity Pulse At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Min Opacity | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Max Opacity | SLIDER | 0 | 100 | -- | -- | 100 | -- | -- | -- |
| -- | Attack | SLIDER | 0 | 25 | -- | 15 | 10 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 5 | -- | -- | -- |

**Opacity Pulse On Beat** -- `ADBE DE Opacity Pulse On Beat`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Min Opacity | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Max Opacity | SLIDER | 0 | 100 | -- | -- | 100 | -- | -- | -- |
| -- | Attack | SLIDER | 0 | 25 | -- | 15 | 10 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 5 | -- | -- | -- |
| -- | Audio Keyframe Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Audio Threshold | SLIDER | 0 | 100 | -- | -- | 15 | -- | -- | -- |

**Opacity Pulse Random** -- `ADBE DE Opacity Pulse Random`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Min Opacity | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Max Opacity | SLIDER | 0 | 100 | -- | -- | 100 | -- | -- | -- |
| -- | Attack | SLIDER | 0 | 25 | -- | 15 | 10 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 5 | -- | -- | -- |
| -- | Min Delay | SLIDER | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | -- |
| -- | Max Delay | SLIDER | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | -- |

**Orbit** -- `ADBE DE Orbit`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Radius | SLIDER | 0 | 3000 | -- | 500 | 150 | -- | -- | -- |
| -- | Frequency | SLIDER | -30 | 30 | -15 | 15 | 0.5 | -- | 2 | -- |
| -- | Starting Phase | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | Layer To Orbit | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |

**Orbit 3D** -- `ADBE DE Orbit 3D`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Radius | SLIDER | 0 | 3000 | -- | 500 | 150 | -- | -- | -- |
| -- | Frequency | SLIDER | -30 | 30 | -15 | 15 | 0.5 | -- | 2 | -- |
| -- | Starting Phase | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | Layer To Orbit | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Z Tilt | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | X Tilt | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | Elevation | SLIDER | -3000 | 3000 | -500 | 500 | 0 | -- | -- | -- |

**Oscillate** -- `ADBE DE Oscillate`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 150 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 5 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Direction | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Oscillate At Marker** -- `ADBE DE Oscillate At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 150 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 5 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Direction | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |

**Oscillate On Beat** -- `ADBE DE Oscillate On Beat`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 150 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 5 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Direction | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | Audio Keyframe Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Audio Threshold | SLIDER | 0 | 100 | -- | -- | 15 | -- | -- | -- |

**Oscillate Random** -- `ADBE DE Oscillate Random`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 150 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 5 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Random Direction | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Direction | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |
| -- | Min Delay | SLIDER | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | -- |
| -- | Max Delay | SLIDER | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | -- |

**Pattern Template** -- `Pseudo/ADBE Pattern Template`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Color | COLOR | -- | -- | -- | -- | RGB [255, 255, 255] | -- | -- | -- |
| -- | Pattern Size | SLIDER | -1000000 | 1000000 | 0 | 100 | 100 | percent | 0 | -- |
| -- | Repeat Width (% of Comp) | SLIDER | -1000000 | 1000000 | 0 | 100 | 100 | percent | 0 | -- |
| -- | Repeat Height (% of Comp) | SLIDER | -1000000 | 1000000 | 0 | 100 | 100 | percent | 0 | -- |
| -- | Animation Speed | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |

**Pendulum** -- `ADBE DE Pendulum`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -180 | 180 | -- | -- | 45 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 3 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Pendulum At Marker** -- `ADBE DE Pendulum At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -180 | 180 | -- | -- | 45 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 3 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |

**Pendulum On Beat** -- `ADBE DE Pendulum On Beat`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -180 | 180 | -- | -- | 45 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 3 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |
| -- | Audio Keyframe Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Audio Threshold | SLIDER | 0 | 100 | -- | -- | 15 | -- | -- | -- |

**Pendulum Random** -- `ADBE DE Pendulum Random`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -180 | 180 | -- | -- | 45 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 3 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 1 | -- | -- | -- |
| -- | Min Delay | SLIDER | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | -- |
| -- | Max Delay | SLIDER | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | -- |

**Percentage Controls** -- `Pseudo/ADBE Percentage Controls`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Animate Value | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | 10x Value Multiplier | SLIDER | -1000000 | 1000000 | 0 | 100 | 1 | -- | 0 | -- |
| -- | Pad Zeros | SLIDER | -1000000 | 1000000 | 0 | 100 | 3 | -- | 0 | -- |
| -- | Digits After Decimal | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 0 | -- |
| -- | Formatting | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Number Format | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| -- | Percent Symbol Position | POINT | -- | -- | -- | -- | (0, 0) fraction of layer | -- | -- | -- |
| -- | Percent Symbol Scale | SLIDER | -1000000 | 1000000 | 0 | 100 | 100 | percent | 1 | -- |
| -- | Grouping | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Show Group Separators | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Group Spacing | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Group Separator Offset | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Align Group Spacing | POPUP | -- | -- | -- | -- | index 3 | -- | -- | -- |

Enumerated options for `Number Format` (1-based, default index 1): 1=1,234,567.01; 2=1.234.567,01; 3=1 234 567,01

Enumerated options for `Align Group Spacing` (1-based, default index 3): 1=Left; 2=Right; 3=Center

**Pulse** -- `ADBE DE Pulse`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -100 | 300 | -- | 100 | 50 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 4 | -- | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0.01 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Pulse At Marker** -- `ADBE DE Pulse At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -100 | 300 | -- | 100 | 50 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 4 | -- | -- | -- |

**Pulse On Beat** -- `ADBE DE Pulse On Beat`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -100 | 300 | -- | 100 | 50 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 4 | -- | -- | -- |
| -- | Audio Keyframe Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Audio Threshold | SLIDER | 0 | 100 | -- | -- | 15 | -- | -- | -- |

**Pulse Random** -- `ADBE DE Pulse Random`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -100 | 300 | -- | 100 | 50 | -- | -- | -- |
| -- | Decay | SLIDER | 4 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Min Delay | SLIDER | 0.1 | 100 | -- | 10 | 0.5 | -- | -- | -- |
| -- | Max Delay | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |

**Radial Wipe Master Controls** -- `ADBE CM TransRadial`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | ...the other corner | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | counterclockwise | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Feather | SLIDER | 0 | 1000 | 0 | 100 | 0 | -- | -- | -- |

**Random Fill Color** -- `ADBE DE Random Fill Color`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Time | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |
| -- | Transition Time Variation | SLIDER | 0 | 100 | -- | 5 | 0.5 | -- | -- | -- |
| -- | Base Color | COLOR | -- | -- | -- | -- | RGB [96, 192, 160] | -- | -- | -- |
| -- | Max Hue Variation | SLIDER | 0 | 180 | -- | -- | 60 | -- | -- | -- |
| -- | Max Saturation Variation | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Max Lightness Variation | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Random Start Time | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Value | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Random Motion** -- `ADBE DE Random Motion`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Travel Time | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |
| -- | Travel Time Variation | SLIDER | 0 | 100 | -- | 5 | 0.5 | -- | -- | -- |
| -- | Horizontal Range | SLIDER | 0 | 3000 | -- | 1000 | 300 | -- | -- | -- |
| -- | Vertical Range | SLIDER | 0 | 3000 | -- | 1000 | 220 | -- | -- | -- |
| -- | Random Start Time | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Value | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Random Motion 1D** -- `ADBE DE Random Motion 1D`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Travel Time | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |
| -- | Travel Time Variation | SLIDER | 0 | 100 | -- | 5 | 0.5 | -- | -- | -- |
| -- | Range | SLIDER | 0 | 3000 | -- | 1000 | 300 | -- | -- | -- |
| -- | Random Start Time | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Value | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Vertical Motion | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Random Opacity** -- `ADBE DE Random Opacity`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Travel Time | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |
| -- | Travel Time Variation | SLIDER | 0 | 100 | -- | 5 | 0.5 | -- | -- | -- |
| -- | Min Opacity | SLIDER | 0 | 100 | -- | -- | 10 | -- | -- | -- |
| -- | Max Opacity | SLIDER | 0 | 100 | -- | -- | 100 | -- | -- | -- |
| -- | Random Start Time | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Value | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Random Rotation** -- `ADBE DE Random Rotation`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Travel Time | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |
| -- | Travel Time Variation | SLIDER | 0 | 100 | -- | 5 | 0.5 | -- | -- | -- |
| -- | Min Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | -360 | -- | -- | -- |
| -- | Max Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | 360 | -- | -- | -- |
| -- | Random Start Time | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Value | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Random Rotation 3D** -- `ADBE DE Random Rotation 3D`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Travel Time | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |
| -- | Travel Time Variation | SLIDER | 0 | 100 | -- | 5 | 0.5 | -- | -- | -- |
| -- | Min X Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | -- |
| -- | Max X Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | -- |
| -- | Min Y Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | -360 | -- | -- | -- |
| -- | Max Y Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | 360 | -- | -- | -- |
| -- | Min Z Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | -- |
| -- | Max Z Rotation | SLIDER | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | -- |
| -- | Random Start Time | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Value | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Random Scale** -- `ADBE DE Random Scale`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Travel Time | SLIDER | 0.1 | 100 | -- | 10 | 1 | -- | -- | -- |
| -- | Travel Time Variation | SLIDER | 0 | 100 | -- | 5 | 0.5 | -- | -- | -- |
| -- | Min Scale | SLIDER | 0 | 1000 | -- | 500 | 25 | -- | -- | -- |
| -- | Max Scale | SLIDER | 0 | 1000 | -- | 500 | 100 | -- | -- | -- |
| -- | Lock X And Y | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Time | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Random Start Value | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Rotate Over Time** -- `ADBE CM Spin`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Rotation (degrees/second) | ANGLE | -- | -- | -- | -- | 10 | degrees | -- | -- |

**Sample Image** -- `ADBE Sample Image`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Center | POINT | -- | -- | -- | -- | (0.5, 0.5) fraction of layer | -- | -- | -- |
| -- | Radius | SLIDER | 0.01 | 10000 | 0.01 | 2048 | 0.5 | -- | -- | -- |
| -- | Sampled Color Output | COLOR | -- | -- | -- | -- | RGB [128, 128, 128] | -- | -- | -- |

**Scale Bounce - layer markers** -- `ADBE CM ScaleBounceMarkers`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Bounce Duration (msec) | SLIDER | 1 | 10000 | 1 | 2000 | 500 | -- | -- | -- |
| -- | Target Scale Change | SLIDER | 0 | 10000 | 0 | 1000 | 200 | percent | -- | -- |

**Scale Bounce - random** -- `ADBE CM ScaleBounceRandom`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Chance of Bouncing | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Bounce Nervousness | SLIDER | 1 | 10000 | 1 | 500 | 50 | -- | -- | -- |
| -- | Target Scale Change | SLIDER | 0 | 10000 | 0 | 1000 | 200 | percent | -- | -- |

**Separate XYZ Position** -- `ADBE Separate XYZ Position`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | X Position | SLIDER | -30000 | 30000 | -1000 | 1000 | 0 | -- | -- | -- |
| -- | Y Position | SLIDER | -30000 | 30000 | -1000 | 1000 | 0 | -- | -- | -- |
| -- | Z Position | SLIDER | -30000 | 30000 | -1000 | 1000 | 0 | -- | -- | -- |

**Separate XYZ Scale** -- `ADBE Separate XYZ Scale`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | X Scale | SLIDER | -1000000 | 1000000 | -100 | 100 | 100 | percent | -- | -- |
| -- | Y Scale | SLIDER | -1000000 | 1000000 | -100 | 100 | 100 | percent | -- | -- |
| -- | Z Scale | SLIDER | -1000000 | 1000000 | -100 | 100 | 100 | percent | -- | -- |

**Slide - variable** -- `ADBE CM SlideVariable`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Entrance Angle | ANGLE | -- | -- | -- | -- | -70 | degrees | -- | -- |
| -- | Initial Scale | SLIDER | 0 | 500 | 0 | 200 | 25 | percent | -- | -- |

**Slide Master Control** -- `ADBE CM TransSlide`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | reverse direction | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Stereo 3D Controls** -- `ADBE Stereo 3D Controls`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Camera Separation | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Convergence Options | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |

**Stretch Master Control** -- `ADBE CM TransStretch`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |

**Stretch Master Control (edge)** -- `ADBE CM TransDirection`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | ...the other side | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Stretch Master Control(corner)** -- `ADBE CM TransCorner`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | ...the other corner | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |

**Swarm** -- `ADBE DE Swarm`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | 0 | 1000 | -- | 500 | 100 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 3 | -- | -- | -- |
| -- | Leader | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |

**Timer Controls** -- `Pseudo/ADBE Timer Controls`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Animate Value | SLIDER | -1000000 | 1000000 | 0 | 100 | 9.9999 | -- | 2 | -- |
| -- | Formatting | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Time Format | POPUP | -- | -- | -- | -- | index 2 | -- | -- | -- |
| -- | Show Labels | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |
| -- | Label Position Offset | POINT | -- | -- | -- | -- | (0, 0) fraction of layer | -- | -- | -- |
| -- | Label Size | SLIDER | -1000000 | 1000000 | 0 | 100 | 32 | -- | 2 | -- |
| -- | Label Spacing | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |
| -- | Grouping | GROUP | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Separator Character | POPUP | -- | -- | -- | -- | index 1 | -- | -- | -- |
| -- | Spacing | SLIDER | -1000000 | 1000000 | 0 | 100 | 0 | -- | 2 | -- |

Enumerated options for `Time Format` (1-based, default index 2): 1=MM:SS; 2=HH:MM:SS; 3=DD:HH:MM:SS

Enumerated options for `Separator Character` (1-based, default index 1): 1=Colons; 2=Dots; 3=Spaces

**Trace Path** -- `Pseudo/ADBE Trace Path`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Progress | SLIDER | 0 | 100 | 0 | 100 | 0 | percent | 1 | -- |
| -- | Loop | CHECKBOX | -- | -- | -- | -- | true | -- | -- | -- |

**Transition Master Control** -- `ADBE CM TransComplete`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |

**Wiggle - gelatin** -- `ADBE CM WiggleGelatin`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Wiggle Speed (wigs/sec) | SLIDER | 0 | 100 | -- | -- | 1 | -- | -- | -- |
| -- | Wiggle Amount | SLIDER | 0 | 70 | -- | -- | 20 | -- | -- | -- |

**Wiggle - position** -- `ADBE CM WigglePosition`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Wiggle Speed (wigs/sec) | SLIDER | 0 | 100 | -- | -- | 1 | -- | -- | -- |
| -- | Wiggle Amount (pixels) | SLIDER | 0 | 10000 | 0 | 500 | 50 | -- | -- | -- |

**Wiggle - rotation** -- `ADBE CM WiggleRotation`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Wiggle Speed (wigs/sec) | SLIDER | 0 | 100 | -- | -- | 1 | -- | -- | -- |
| -- | Wiggle Amount (degrees) | ANGLE | -- | -- | -- | -- | 30 | degrees | -- | -- |

**Wiggle - scale** -- `ADBE CM WiggleScale`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Wiggle Speed (wigs/sec) | SLIDER | 0 | 100 | -- | -- | 1 | -- | -- | -- |
| -- | Wiggle Amount | SLIDER | 0 | 10000 | 0 | 1000 | 10 | percent | -- | -- |
| -- | Wiggle Width Separately? | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Wiggle Width | SLIDER | 0 | 10000 | 0 | 1000 | 10 | percent | -- | -- |

**Wiggle - shear** -- `ADBE CM WiggleShear`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Wiggle Speed (wigs/sec) | SLIDER | 0 | 100 | -- | -- | 1 | -- | -- | -- |
| -- | Wiggle Amount | SLIDER | 0 | 70 | -- | -- | 20 | -- | -- | -- |

**Wigglerama** -- `ADBE CM Wigglerama`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Wiggle Speed (wigs/sec) | SLIDER | 0 | 100 | -- | -- | 1 | -- | -- | -- |
| -- | Wiggle Nervousness | SLIDER | 1 | 20 | -- | -- | 1 | -- | -- | -- |
| -- | Wiggle Position (pixels) | SLIDER | 0 | 10000 | 0 | 500 | 25 | -- | -- | -- |
| -- | Wiggle Rotation (degrees) | ANGLE | -- | -- | -- | -- | 30 | degrees | -- | -- |
| -- | Wiggle Scale | SLIDER | 0 | 10000 | 0 | 1000 | 15 | percent | -- | -- |
| -- | Wiggle Width Separately? | CHECKBOX | -- | -- | -- | -- | false | -- | -- | -- |
| -- | Wiggle Width | SLIDER | 0 | 10000 | 0 | 1000 | 10 | percent | -- | -- |

**Wipe Master Control** -- `ADBE CM TransWipe`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |

**Wipe Master Controls** -- `ADBE CM TransWipeFeath`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Feather | SLIDER | 0 | 1000 | 0 | 100 | 0 | -- | -- | -- |

**Wobble Bounce** -- `ADBE DE Wobble Bounce`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |
| -- | Wobble Amplitude | SLIDER | 0 | 100 | -- | 100 | 35 | -- | -- | -- |
| -- | Wobble Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Wobble Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Wobble Bounce At Marker** -- `ADBE DE Wobble Bounce At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |
| -- | Wobble Amplitude | SLIDER | 0 | 100 | -- | 100 | 35 | -- | -- | -- |
| -- | Wobble Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Wobble Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |

**Wobble Bounce On Beat** -- `ADBE DE Wobble Bounce On Beat`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |
| -- | Wobble Amplitude | SLIDER | 0 | 100 | -- | 100 | 35 | -- | -- | -- |
| -- | Wobble Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Wobble Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |
| -- | Audio Keyframe Layer | LAYER | -- | -- | -- | -- | -- | -- | -- | -- |
| -- | Audio Threshold | SLIDER | 0 | 100 | -- | -- | 15 | -- | -- | -- |

**Wobble Bounce Random** -- `ADBE DE Wobble Bounce Random`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | 0 | -- | 250 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 1 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |
| -- | Wobble Amplitude | SLIDER | 0 | 100 | -- | 100 | 35 | -- | -- | -- |
| -- | Wobble Frequency | SLIDER | 0 | 30 | -- | 15 | 4 | -- | -- | -- |
| -- | Wobble Decay | SLIDER | 0 | 25 | -- | 15 | 0.7 | -- | -- | -- |
| -- | Min Delay | SLIDER | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | -- |
| -- | Max Delay | SLIDER | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | -- |
| -- | Internal Use Only | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Z Spring** -- `ADBE DE Z Spring`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | -- | -- | -500 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 3 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Wander Amount | SLIDER | 0 | 500 | -- | -- | 50 | -- | -- | -- |
| -- | Rotational Amplitude | SLIDER | -360 | 360 | -- | -- | 50 | -- | -- | -- |
| -- | Rotational Frequency | SLIDER | 0 | 30 | -- | 15 | 1.5 | -- | -- | -- |
| -- | Delay | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |
| -- | Cycle Time | SLIDER | 0 | 100 | -- | -- | 0 | -- | -- | -- |

**Z Spring At Marker** -- `ADBE DE Z Spring At Marker`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Amplitude | SLIDER | -1000 | 1000 | -- | -- | -500 | -- | -- | -- |
| -- | Frequency | SLIDER | 0 | 30 | -- | 15 | 3 | -- | -- | -- |
| -- | Decay | SLIDER | 0 | 25 | -- | 15 | 3 | -- | -- | -- |
| -- | Wander Amount | SLIDER | 0 | 500 | -- | -- | 50 | -- | -- | -- |
| -- | Rotational Amplitude | SLIDER | -360 | 360 | -- | -- | 50 | -- | -- | -- |
| -- | Rotational Frequency | SLIDER | 0 | 30 | -- | 15 | 1.5 | -- | -- | -- |

**Zoom - 2D spin** -- `ADBE CM Zoom2DSpin`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Spin Amount | ANGLE | -- | -- | -- | -- | 180 | degrees | -- | -- |

**Zoom - 3D tumble** -- `ADBE CM Zoom3DTumble`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | X tumble | ANGLE | -- | -- | -- | -- | 60 | degrees | -- | -- |
| -- | Y tumble | ANGLE | -- | -- | -- | -- | 360 | degrees | -- | -- |

**Zoom - bubble** -- `ADBE CM ZoomBubble`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |

**Zoom - spiral** -- `ADBE CM ZoomSpiral`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Spiral Size (pixels) | SLIDER | 0 | 2000 | -- | -- | 200 | -- | -- | -- |
| -- | Spiral Start Angle (degrees) | ANGLE | -- | -- | -- | -- | -360 | degrees | -- | -- |
| -- | Spiral End Angle (degrees) | ANGLE | -- | -- | -- | -- | 0 | degrees | -- | -- |

**Zoom - wobble** -- `ADBE CM ZoomWobble`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| # | Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | flags |
|---|---|---|---|---|---|---|---|---|---|---|
| -- | Transition Completion | SLIDER | 0 | 100 | -- | -- | 50 | percent | -- | -- |
| -- | Wobble Amount | ANGLE | -- | -- | -- | -- | 60 | degrees | -- | -- |
---

## 14.9.5 Layer styles as effect kinds

**[STU-FX-132] The ten layer-style kinds are `StudioLiveFilter` kinds, not a parallel system.**
Clause [STU-FX-025] already collapses the layer-style panel, the vector stylize effects and the
frame-effect panel into one cross-domain effect set. This group supplies the parameter contract for that
set: ten style kinds carrying 89 typed parameters between them, recovered with hard ranges, soft
ranges, defaults, units, precisions and full enumerated option lists. Every one is a stack entry
that interleaves with filters and adjustments in one order ([STU-FX-026], [STU-FX-012]).

**[STU-FX-132a]** Two contract details in this set are normative and easy to lose. First, the shadow
and glow styles carry a `Use Global Light` boolean alongside a local angle: when the boolean is
true the style reads a composition-level lighting angle instead of its own, so the angle parameter
is present but not authoritative. Studio models this as a per-composition `global_light_angle`
property with per-style opt-in, not as a hidden coupling between parameters. Second, several style
distance and size parameters declare a hard maximum of 30000 against a soft maximum of 100 -- a 300x
ratio -- which is the clearest available demonstration of why [STU-FX-107] exists.


**Drop Shadow** -- `dropShadow/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 5 | -- | -- | `dropShadow/mode2` |
| Color | COLOR | -- | -- | -- | -- | RGB [0, 0, 0] | -- | -- | `dropShadow/color` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 75 | percent | 0 | `dropShadow/opacity` |
| Use Global Light | CHECKBOX | -- | -- | -- | -- | false | -- | -- | `dropShadow/useGlobalAngle` |
| Angle | ANGLE | -- | -- | -- | -- | 120 | degrees | -- | `dropShadow/localLightingAngle` |
| Distance | SLIDER | 0 | 30000 | -- | 100 | 5 | -- | -- | `dropShadow/distance` |
| Spread | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `dropShadow/chokeMatte` |
| Size | SLIDER | 0 | 250 | -- | 100 | 5 | -- | -- | `dropShadow/blur` |
| Noise | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `dropShadow/noise` |
| Layer Knocks Out Drop Shadow | CHECKBOX | -- | -- | -- | -- | true | -- | -- | `dropShadow/layerConceals` |

Enumerated options for `Blend Mode` (1-based, default index 5): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

**Inner Shadow** -- `innerShadow/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 5 | -- | -- | `innerShadow/mode2` |
| Color | COLOR | -- | -- | -- | -- | RGB [0, 0, 0] | -- | -- | `innerShadow/color` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 75 | percent | 0 | `innerShadow/opacity` |
| Use Global Light | CHECKBOX | -- | -- | -- | -- | false | -- | -- | `innerShadow/useGlobalAngle` |
| Angle | ANGLE | -- | -- | -- | -- | 120 | degrees | -- | `innerShadow/localLightingAngle` |
| Distance | SLIDER | 0 | 30000 | -- | 100 | 5 | -- | -- | `innerShadow/distance` |
| Choke | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `innerShadow/chokeMatte` |
| Size | SLIDER | 0 | 250 | -- | 100 | 5 | -- | -- | `innerShadow/blur` |
| Noise | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `innerShadow/noise` |

Enumerated options for `Blend Mode` (1-based, default index 5): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

**Outer Glow** -- `outerGlow/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 11 | -- | -- | `outerGlow/mode2` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 75 | percent | 0 | `outerGlow/opacity` |
| Noise | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `outerGlow/noise` |
| Color Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `outerGlow/AEColorChoice` |
| Color | COLOR | -- | -- | -- | -- | RGB [255, 255, 190] | -- | -- | `outerGlow/color` |
| Gradient | GRADIENT | -- | -- | -- | -- | -- | -- | -- | `outerGlow/gradient` |
| Gradient Smoothness | SLIDER | 0 | 100 | -- | -- | 100 | percent | -- | `outerGlow/gradientSmoothness` |
| Technique | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `outerGlow/glowTechnique` |
| Spread | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `outerGlow/chokeMatte` |
| Size | SLIDER | 0 | 250 | -- | 100 | 5 | -- | -- | `outerGlow/blur` |
| Range | SLIDER | 1 | 100 | -- | -- | 50 | percent | -- | `outerGlow/inputRange` |
| Jitter | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `outerGlow/shadingNoise` |

Enumerated options for `Blend Mode` (1-based, default index 11): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

Enumerated options for `Color Type` (1-based, default index 1): 1=Single Color; 2=Gradient

Enumerated options for `Technique` (1-based, default index 1): 1=Softer; 2=Precise

**Inner Glow** -- `innerGlow/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 11 | -- | -- | `innerGlow/mode2` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 75 | percent | 0 | `innerGlow/opacity` |
| Noise | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `innerGlow/noise` |
| Color Type | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `innerGlow/AEColorChoice` |
| Color | COLOR | -- | -- | -- | -- | RGB [255, 255, 190] | -- | -- | `innerGlow/color` |
| Gradient | GRADIENT | -- | -- | -- | -- | -- | -- | -- | `innerGlow/gradient` |
| Gradient Smoothness | SLIDER | 0 | 100 | -- | -- | 100 | percent | -- | `innerGlow/gradientSmoothness` |
| Technique | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `innerGlow/glowTechnique` |
| Source | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `innerGlow/innerGlowSource` |
| Choke | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `innerGlow/chokeMatte` |
| Size | SLIDER | 0 | 250 | -- | 100 | 5 | -- | -- | `innerGlow/blur` |
| Range | SLIDER | 1 | 100 | -- | -- | 50 | percent | -- | `innerGlow/inputRange` |
| Jitter | SLIDER | 0 | 100 | -- | -- | 0 | percent | -- | `innerGlow/shadingNoise` |

Enumerated options for `Blend Mode` (1-based, default index 11): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

Enumerated options for `Color Type` (1-based, default index 1): 1=Single Color; 2=Gradient

Enumerated options for `Technique` (1-based, default index 1): 1=Softer; 2=Precise

Enumerated options for `Source` (1-based, default index 1): 1=Edge; 2=Center

**Bevel and Emboss** -- `bevelEmboss/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Style | POPUP | -- | -- | -- | -- | index 2 | -- | -- | `bevelEmboss/bevelStyle` |
| Technique | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `bevelEmboss/bevelTechnique` |
| Depth | SLIDER | 1 | 1000 | -- | -- | 100 | percent | -- | `bevelEmboss/strengthRatio` |
| Direction | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `bevelEmboss/bevelDirection` |
| Size | SLIDER | 0 | 250 | -- | 250 | 5 | -- | -- | `bevelEmboss/blur` |
| Soften | SLIDER | 0 | 16 | -- | 16 | 0 | -- | -- | `bevelEmboss/softness` |
| Use Global Light | CHECKBOX | -- | -- | -- | -- | false | -- | -- | `bevelEmboss/useGlobalAngle` |
| Angle | ANGLE | -- | -- | -- | -- | 120 | degrees | -- | `bevelEmboss/localLightingAngle` |
| Altitude | ANGLE | -- | -- | -- | -- | 30 | degrees | -- | `bevelEmboss/localLightingAltitude` |
| Highlight Mode | POPUP | -- | -- | -- | -- | index 11 | -- | -- | `bevelEmboss/highlightMode` |
| Highlight Color | COLOR | -- | -- | -- | -- | RGB [255, 255, 255] | -- | -- | `bevelEmboss/highlightColor` |
| Highlight Opacity | SLIDER | 0 | 100 | -- | -- | 75 | percent | 0 | `bevelEmboss/highlightOpacity` |
| Shadow Mode | POPUP | -- | -- | -- | -- | index 5 | -- | -- | `bevelEmboss/shadowMode` |
| Shadow Color | COLOR | -- | -- | -- | -- | RGB [0, 0, 0] | -- | -- | `bevelEmboss/shadowColor` |
| Shadow Opacity | SLIDER | 0 | 100 | -- | -- | 75 | percent | 0 | `bevelEmboss/shadowOpacity` |

Enumerated options for `Style` (1-based, default index 2): 1=Outer Bevel; 2=Inner Bevel; 3=Emboss; 4=Pillow Emboss; 5=Stroke Emboss

Enumerated options for `Technique` (1-based, default index 1): 1=Smooth; 2=Chisel Hard; 3=Chisel Soft

Enumerated options for `Direction` (1-based, default index 1): 1=Up; 2=Down

Enumerated options for `Highlight Mode` (1-based, default index 11): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

Enumerated options for `Shadow Mode` (1-based, default index 5): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

**Satin** -- `chromeFX/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 5 | -- | -- | `chromeFX/mode2` |
| Color | COLOR | -- | -- | -- | -- | RGB [0, 0, 0] | -- | -- | `chromeFX/color` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 50 | percent | 0 | `chromeFX/opacity` |
| Angle | ANGLE | -- | -- | -- | -- | 19 | degrees | -- | `chromeFX/localLightingAngle` |
| Distance | SLIDER | 1 | 250 | -- | 100 | 11 | -- | -- | `chromeFX/distance` |
| Size | SLIDER | 0 | 250 | -- | 100 | 14 | -- | -- | `chromeFX/blur` |
| Invert | CHECKBOX | -- | -- | -- | -- | true | -- | -- | `chromeFX/invert` |

Enumerated options for `Blend Mode` (1-based, default index 5): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

**Color Overlay** -- `solidFill/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `solidFill/mode2` |
| Color | COLOR | -- | -- | -- | -- | RGB [255, 0, 0] | -- | -- | `solidFill/color` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 100 | percent | 0 | `solidFill/opacity` |

Enumerated options for `Blend Mode` (1-based, default index 1): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

**Gradient Overlay** -- `gradientFill/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `gradientFill/mode2` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 100 | percent | 0 | `gradientFill/opacity` |
| Gradient | GRADIENT | -- | -- | -- | -- | -- | -- | -- | `gradientFill/gradient` |
| Gradient Smoothness | SLIDER | 0 | 100 | -- | -- | 100 | percent | -- | `gradientFill/gradientSmoothness` |
| Angle | ANGLE | -- | -- | -- | -- | 90 | degrees | -- | `gradientFill/angle` |
| Style | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `gradientFill/type` |
| Reverse | CHECKBOX | -- | -- | -- | -- | false | -- | -- | `gradientFill/reverse` |
| Align with Layer | CHECKBOX | -- | -- | -- | -- | true | -- | -- | `gradientFill/align` |
| Scale | SLIDER | 10 | 150 | -- | -- | 100 | percent | -- | `gradientFill/scale` |
| Offset | POINT | -- | -- | -- | -- | -- | -- | -- | `gradientFill/offset` |

Enumerated options for `Blend Mode` (1-based, default index 1): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

Enumerated options for `Style` (1-based, default index 1): 1=Linear; 2=Radial; 3=Angle; 4=Reflected; 5=Diamond

**Pattern Overlay** -- `patternFill/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `patternFill/mode2` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 100 | percent | 0 | `patternFill/opacity` |
| Link with Layer | CHECKBOX | -- | -- | -- | -- | true | -- | -- | `patternFill/align` |
| Scale | SLIDER | 1 | 1000 | -- | -- | 100 | percent | -- | `patternFill/scale` |
| Offset | POINT | -- | -- | -- | -- | -- | -- | -- | `patternFill/phase` |

Enumerated options for `Blend Mode` (1-based, default index 1): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

**Stroke** -- `frameFX/enabled`

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Kind | hard_min | hard_max | soft_min | soft_max | default | unit | precision | property key |
|---|---|---|---|---|---|---|---|---|---|
| Blend Mode | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `frameFX/mode2` |
| Color | COLOR | -- | -- | -- | -- | RGB [255, 0, 0] | -- | -- | `frameFX/color` |
| Size | SLIDER | 1 | 250 | -- | 100 | 3 | -- | -- | `frameFX/size` |
| Opacity | SLIDER | 0 | 100 | -- | -- | 100 | percent | 0 | `frameFX/opacity` |
| Position | POPUP | -- | -- | -- | -- | index 1 | -- | -- | `frameFX/style` |

Enumerated options for `Blend Mode` (1-based, default index 1): 1=Normal; 2=Dissolve; 4=Darken; 5=Multiply; 6=Color Burn; 7=Linear Burn; 8=Darker Color; 10=Lighten; 11=Screen; 12=Color Dodge; 13=Linear Dodge; 14=Lighter Color; 16=Overlay; 17=Soft Light; 18=Hard Light; 19=Vivid Light; 20=Linear Light; 21=Pin Light; 22=Hard Mix; 24=Difference; 25=Exclusion; 27=Hue; 28=Saturation; 29=Color; 30=Luminosity; 32=Subtract; 33=Divide

Enumerated options for `Position` (1-based, default index 1): 1=Outside; 2=Inside; 3=Center
---

## 14.9.6 Catalogue: the video-editing effect set

[STU-FX-133] **The tables in this group are the normative Studio effect set contributed by the
video-editing domain.** 371 video effects and 107 preset-defined pseudo-effects. Where a row's
engine column says the effect is the compositing application's engine, it is the SAME Studio
`filter_kind` as the corresponding row in 14.9.3 and dedups to it ([STU-FX-127]); such rows are
listed in their own table below, per [STU-FX-127b], so that no capability is lost, the editing-side
parameter surface is recorded, and the dedup is machine-readable rather than implied. The
`Params` column counts recovered parameter rows; per [STU-FX-106a] only 6.5 percent of those rows
carry any bound, and the bounded subset is reproduced in full in [STU-FX-133a].


**(uncategorised)** (371)

*Derivation: catalogue table, splits per row; yields 34 microtasks, one per new Studio `filter_kind`; the other 337 rows dedup onto 14.9.3 under [STU-FX-127] and yield 0.*

**New Studio effect kinds contributed by the editing application** (179 rows)

*Derivation: catalogue table, splits per row; yields 173 microtasks, one per new Studio `filter_kind`.*

| Studio effect | Engine | Params | Presets | Description (from capture) | Import key (provenance) |
|---|---|---|---|---|---|
| Active | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:Active` |
| Legacy Matte Entry (unidentified) | ae_native | 10 | 0 | _no vendor description recovered_ | `AE_LStr:AE_OLD_MT` |
| Black And White | mediacore | 1 | 0 | BlackAndWhite... | `MediaCore:AEBlackAndWhite` |
| Color And Contrast | mediacore | 28 | 0 | Color And Contrast... | `MediaCore:AEColorAndContrast` |
| Contrast | mediacore | 6 | 0 | Contrast... | `MediaCore:AEContrast` |
| Cross Dissolve | mediacore | 1 | 0 | Cross Dissolve... | `MediaCore:AECrossDissolve` |
| Clock Wipe | mediacore | 1 | 0 | Clock Wipe... | `MediaCore:AEFilterClockWipe` |
| Broadcast Level Limiter | mediacore | 9 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterDigitalVideoLimiter` |
| 3D Spinback Transition | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterFilmImpact3DSpinback` |
| Channel Blur Transition | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterFilmImpactChannelBlur` |
| Gradient Transition | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterFilmImpactGradient` |
| Noise Transition | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterFilmImpactNoise` |
| Slide Transition | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterFilmImpactSlide` |
| Morph Cut | mediacore | 1 | 0 | MorphCut... | `MediaCore:AEFilterMorphCut` |
| Oil Paint | mediacore | 32 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterOilPaint` |
| Page Peel | mediacore | 3 | 0 | PagePeel... | `MediaCore:AEFilterPagePeel` |
| Radial Wipe Transition | mediacore | 1 | 0 | Radial Wipe... | `MediaCore:AEFilterRadialWipeTransition` |
| Split Transition | mediacore | 1 | 0 | Split... | `MediaCore:AEFilterSplit` |
| Immersive Chroma Leaks | mediacore | 0 | 0 | VR Chroma Leaks... | `MediaCore:AEFilterVRChromaLeaks` |
| Immersive Chromatic Aberration | mediacore | 0 | 0 | VR Chromatic Aberrations... | `MediaCore:AEFilterVRChromaticAberrations` |
| Immersive Gradient Wipe | mediacore | 0 | 0 | VR Gradient Wipe... | `MediaCore:AEFilterVRGradientWipe` |
| Immersive Iris Wipe | mediacore | 0 | 0 | VR Iris Wipe... | `MediaCore:AEFilterVRIrisWipe` |
| Immersive Light Leaks | mediacore | 0 | 0 | VR Light Leaks... | `MediaCore:AEFilterVRLightLeaks` |
| Immersive Light Rays | mediacore | 0 | 0 | VR Light Rays... | `MediaCore:AEFilterVRLightRays` |
| Immersive Mobius Zoom | mediacore | 0 | 0 | VR Mobius Zoom... | `MediaCore:AEFilterVRMobiusZoom` |
| Immersive Random Blocks | mediacore | 0 | 0 | VR Random Blocks... | `MediaCore:AEFilterVRRandomBlocks` |
| Immersive Sphere To Plane | mediacore | 0 | 0 | VR Sphere To Plane... | `MediaCore:AEFilterVRSphereToPlane` |
| Immersive Spherical Blur | mediacore | 0 | 0 | VR Spherical Blur... | `MediaCore:AEFilterVRSphericalBlur` |
| Whip Transition | mediacore | 1 | 0 | Whip | `MediaCore:AEFilterWhip` |
| Effect Host Bridge (registration only) | ae_native | 1 | 0 | _no vendor description recovered_ | `AE_LStr:AEGPDriver` |
| Image Matte Key | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:AEImageMatteKey` |
| Metadata Passthrough | mediacore | 54 | 0 | Metadata... | `MediaCore:AEMetadata` |
| Midtones | mediacore | 5 | 0 | _no vendor description recovered_ | `MediaCore:AEMidtones` |
| Morph Cut | mediacore | 9 | 0 | _no vendor description recovered_ | `MediaCore:AEMorphCut` |
| Non-Red Key | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:AENonRedKey` |
| Alpha Adjust | mediacore | 5 | 0 | AlphaAdjust... | `MediaCore:AlphaAdjust` |
| Alpha Glow | mediacore | 6 | 0 | Alpha Glow | `MediaCore:AEFilterAlphaGlow` |
| Amplitude | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:Amplitude` |
| ARRIRAW Development Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:ARRIRAWSourceSettingsEffect` |
| ARRIRAWSourceSettings | mediacore | 7 | 0 | ARRIRAW Development Settings... | `MediaCore:ARRIRAWSourceSettings` |
| ASC CDL | mediacore | 10 | 0 | ASC CDL... | `MediaCore:AEFilterASCCDL` |
| Auto Reframe | mediacore | 10 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterAutoReframe` |
| AX | ae_native | 96 | 0 | _no vendor description recovered_ | `AE_LStr:AX` |
| Barn Doors | mediacore | 0 | 0 | BarnDoors... | `MediaCore:AEFilterBarnDoors` |
| BEE | ae_native | 1013 | 0 | _no vendor description recovered_ | `AE_LStr:BEE` |
| BEZ | ae_native | 7 | 0 | _no vendor description recovered_ | `AE_LStr:BEZ` |
| Blob | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:Blob` |
| Blue Screen Key | mediacore | 4 | 0 | BlueScreenKey... | `MediaCore:AEBlueScreenKey` |
| BM | ae_native | 67 | 0 | _no vendor description recovered_ | `AE_LStr:BM` |
| Camera Blur | mediacore | 0 | 0 | CameraBlur... | `MediaCore:AEFilterCameraBlur` |
| CAMLIGHT | ae_native | 45 | 0 | _no vendor description recovered_ | `AE_LStr:CAMLIGHT` |
| Canon Cinema RAW Light Source Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:CanonRawSourceSettingsEffect` |
| CanonRawSourceSettings | mediacore | 11 | 0 | Canon Raw Source Settings... | `MediaCore:CanonRawSourceSettings` |
| Center Split | mediacore | 0 | 0 | Center Split... | `MediaCore:AEFilterCenterSplit` |
| Change To Color | ae_native | 11 | 0 | Change color using HLS interpolation. | `ADBE Change To Color` |
| Channel_Mixer | ae_native | 13 | 0 | Combines color channels. | `ADBE CHANNEL MIXER` |
| Chroma Key | mediacore | 7 | 0 | ChromaKey... | `MediaCore:AEChromaKey` |
| CinemaDNG Source Settings | mediacore | 4 | 0 | Lumetri Source Settings... | `MediaCore:CinemaDNGSourceSettings` |
| Cineon Source Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:DPXSourceSettingsEffect` |
| Clip Name | mediacore | 15 | 0 | ClipName... | `MediaCore:AEClipName` |
| Color And Contrast | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterColorAndContrast` |
| Color Balance 2 | ae_native | 11 | 0 | Adjusts strengths of color channels and preserves luminosity. | `ADBE Color Balance 2` |
| Color Shift | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterColorShift` |
| ColorFast | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:ColorFast` |
| ColorStyles | mediacore | 23 | 0 | _no vendor description recovered_ | `MediaCore:ColorStyles` |
| ColorStylesAEFlare | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:ColorStylesAEFlare` |
| ColorStylesAEMidtones | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:ColorStylesAEMidtones` |
| ColorSwatches | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:ColorSwatches` |
| Compress Expand | ae_native | 8 | 0 | _no vendor description recovered_ | `AE_LStr:Compress_Expand` |
| Contrast | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterContrast` |
| Convolution Kernel | mediacore | 12 | 10 | _no vendor description recovered_ | `PR.ADBE Convolution Kernel New` |
| COR | ae_native | 15 | 0 | _no vendor description recovered_ | `AE_LStr:COR` |
| Crop | mediacore | 7 | 0 | Crop... | `MediaCore:AECrop` |
| D CIN | ae_native | 5 | 0 | _no vendor description recovered_ | `AE_LStr:D_CIN` |
| D EI | ae_native | 16 | 0 | _no vendor description recovered_ | `AE_LStr:D_EI` |
| D JSX | ae_native | 2 | 0 | _no vendor description recovered_ | `AE_LStr:D_JSX` |
| D MCEXP | ae_native | 9 | 0 | _no vendor description recovered_ | `AE_LStr:D_MCEXP` |
| D PS | ae_native | 34 | 0 | _no vendor description recovered_ | `AE_LStr:D_PS` |
| D PS3 | ae_native | 30 | 0 | _no vendor description recovered_ | `AE_LStr:D_PS3` |
| D PST | ae_native | 5 | 0 | _no vendor description recovered_ | `AE_LStr:D_PST` |
| D RLA | ae_native | 29 | 0 | _no vendor description recovered_ | `AE_LStr:D_RLA` |
| D YM | ae_native | 70 | 0 | _no vendor description recovered_ | `AE_LStr:D_YM` |
| D ZPIC | ae_native | 26 | 0 | _no vendor description recovered_ | `AE_LStr:D_ZPIC` |
| De-esser | ae_native | 6 | 0 | An Audio De-esser for use in After Effects. | `ADBE Aud Deesser` |
| DigitalVideoLimiter | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:DigitalVideoLimiter` |
| DipTransitions | mediacore | 4 | 0 | _no vendor description recovered_ | `MediaCore:DipTransitions` |
| DPXSourceSettings | mediacore | 12 | 0 | DPX Source Settings... | `MediaCore:DPXSourceSettings` |
| EGG | ae_native | 2385 | 0 | _no vendor description recovered_ | `AE_LStr:EGG` |
| Eight-Point Garbage Matte | mediacore | 8 | 0 | GarbageMatte8... | `MediaCore:AEGarbageMatte8` |
| Escher | ae_native | 47 | 0 | _no vendor description recovered_ | `ADBE Escher` |
| F65SourceSettings | mediacore | 3 | 0 | F65 Source Settings... | `MediaCore:F65SourceSettings` |
| Fast Blur | ae_native | 3 | 2 | _no vendor description recovered_ | `AE.ADBE Fast Blur` |
| Fast Blur | ae_native | 4 | 0 | Apply a smooth blur to an image. | `ADBE Fast Blur` |
| FILE | ae_native | 124 | 0 | _no vendor description recovered_ | `AE_LStr:FILE` |
| Film Color | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterColorFilm` |
| Flare | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterFlare` |
| FLO | ae_native | 14 | 0 | _no vendor description recovered_ | `AE_LStr:FLO` |
| FLT | ae_native | 161 | 0 | _no vendor description recovered_ | `AE_LStr:FLT` |
| Four-Point Garbage Matte | mediacore | 4 | 0 | GarbageMatte... | `MediaCore:AEGarbageMatte` |
| GOV | ae_native | 15 | 0 | _no vendor description recovered_ | `AE_LStr:GOV` |
| Graphics | mediacore | 110 | 0 | _no vendor description recovered_ | `MediaCore:Graphics` |
| Grow_Bounds | ae_native | 1 | 0 | Grows the bounds of a layer | `ADBE GROW BOUNDS` |
| Horizontal Flip | mediacore | 0 | 0 | Horizontal Flip | `MediaCore:AEFilterHorizontalFlip` |
| HSL Mask | mediacore | 12 | 0 | HSL Mask... | `MediaCore:AEHSLMask` |
| HueVsHue | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:HueVsHue` |
| HueVsLuma | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:HueVsLuma` |
| HueVsSat | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:HueVsSat` |
| Inset | mediacore | 0 | 0 | Inset... | `MediaCore:AEFilterInset` |
| LightingEffect | mediacore | 27 | 0 | _no vendor description recovered_ | `MediaCore:LightingEffect` |
| LIST | ae_native | 31 | 0 | _no vendor description recovered_ | `AE_LStr:LIST` |
| LUT Transform | mediacore | 1 | 0 | LUT Transform, v%ld.%ld#{cr}#{cr}#{copy}2026-{{*CopyYear*}} Adobe Systems Inc. | `MediaCore:AEFilterLUTTransform` |
| M | ae_native | 41 | 0 | _no vendor description recovered_ | `AE_LStr:M` |
| Mask | mediacore | 8 | 0 | Mask... | `MediaCore:AEMask` |
| Mask2 | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEMask2` |
| MC | ae_native | 26 | 0 | _no vendor description recovered_ | `AE_LStr:MC` |
| Midtones | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterMidtones` |
| Motion | mediacore | 15 | 0 | Motion... | `MediaCore:Motion` |
| MPEG Source Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:MPEGSourceSettingsEffect` |
| MPEGSourceSettings | mediacore | 2 | 0 | MPEG Source Settings... | `MediaCore:MPEGSourceSettings` |
| MSK | ae_native | 4 | 0 | _no vendor description recovered_ | `AE_LStr:MSK` |
| MT | ae_native | 117 | 0 | _no vendor description recovered_ | `AE_LStr:MT` |
| Mute | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:Mute` |
| MXF/ARRIRAW Development Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:ArriRawMXFSourceSettingsEffect` |
| Non-Additive Dissolve | mediacore | 0 | 0 | Non-Additive Dissolve, v%ld.%ld#{cr}#{cr}#{copy}1992-{{*CopyYear*}} Adobe Systems Inc.#{cr}#{cr}The Luminance of image A is mapped onto image B. | `MediaCore:AEFilterNonAdditiveDissolve` |
| OM | ae_native | 27 | 0 | _no vendor description recovered_ | `AE_LStr:OM` |
| Opacity | mediacore | 3 | 0 | Opacity... | `MediaCore:Opacity` |
| PF | ae_native | 19 | 0 | _no vendor description recovered_ | `AE_LStr:PF` |
| Layer Style Support | ae_native | 11 | 0 | _no vendor description recovered_ | `AE_LStr:PhotoshopLayerStyleSupport` |
| PIN | ae_native | 131 | 0 | _no vendor description recovered_ | `AE_LStr:PIN` |
| PLUG | ae_native | 24 | 0 | _no vendor description recovered_ | `AE_LStr:PLUG` |
| PNGIO | ae_native | 40 | 0 | _no vendor description recovered_ | `AE_LStr:PNGIO` |
| PR | ae_native | 13 | 0 | _no vendor description recovered_ | `AE_LStr:PR` |
| PREF | ae_native | 13 | 0 | _no vendor description recovered_ | `AE_LStr:PREF` |
| ProcAmp | mediacore | 7 | 0 | ProcAmp... | `MediaCore:ProcAmp` |
| PRORES RAW Source Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:ProResRawSourceSettingsEffect` |
| ProResRawSourceSettings | mediacore | 4 | 0 | ProRes Raw Source Settings... | `MediaCore:ProResRawSourceSettings` |
| PT | ae_native | 2 | 0 | _no vendor description recovered_ | `AE_LStr:PT` |
| Range | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:Range` |
| RED Source Settings | mediacore | 85 | 0 | RED Source Settings... | `MediaCore:REDSourceSettings` |
| Replicate | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterReplicate` |
| RESHAPE | ae_native | 15 | 0 | Reshape a portion of the image. | `ADBE RESHAPE` |
| RG | ae_native | 1 | 0 | _no vendor description recovered_ | `AE_LStr:RG` |
| RGB Difference Key | mediacore | 5 | 0 | RGBDifferenceKey... | `MediaCore:AERGBDifferenceKey` |
| SDR Conform | mediacore | 3 | 0 | SDR Conform... | `MediaCore:AEFilterSDRConform` |
| Simple Text | mediacore | 13 | 0 | SimpleText... | `MediaCore:AESimpleText` |
| Sixteen-Point Garbage Matte | mediacore | 16 | 0 | GarbageMatte16... | `MediaCore:AEGarbageMatte16` |
| SLU | ae_native | 4 | 0 | _no vendor description recovered_ | `AE_LStr:SLU` |
| SMEAR | ae_native | 14 | 0 | Distort the image using a curve. | `AE_LStr:SMEAR` |
| SND | ae_native | 53 | 0 | _no vendor description recovered_ | `AE_LStr:SND` |
| Solarize | mediacore | 1 | 2 | _no vendor description recovered_ | `PR.ADBE Solarize` |
| Sony RAW MXF Source Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:SonyRawMXFSourceSettingsEffect` |
| Sony Raw Source Settings | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:F65SourceSettingsEffect` |
| SonyRawMXFSourceSettings | mediacore | 3 | 0 | Sony RAW MXF Source Settings... | `MediaCore:SonyRawMXFSourceSettings` |
| SOUP | ae_native | 10 | 0 | _no vendor description recovered_ | `AE_LStr:SOUP` |
| Stabilizer | ae_native | 43 | 0 | _no vendor description recovered_ | `AE_LStr:Stabilizer` |
| Standard3D | ae_native | 45 | 0 | _no vendor description recovered_ | `AE_LStr:Standard3D` |
| SY | ae_native | 8 | 0 | _no vendor description recovered_ | `AE_LStr:SY` |
| TDB | ae_native | 99 | 0 | _no vendor description recovered_ | `AE_LStr:TDB` |
| Texture | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterColorTexture` |
| Texture | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEColorTexture` |
| ThreeWayColorCorrector | ae_native | 57 | 0 | Three Way Color Correction. | `AE_LStr:ThreeWayColorCorrector` |
| TOTD | ae_native | 14 | 0 | _no vendor description recovered_ | `AE_LStr:TOTD` |
| Track Matte Key | mediacore | 3 | 0 | TrackMatteKey... | `MediaCore:AETrackMatteKey` |
| Tracker3d | ae_native | 13 | 0 | _no vendor description recovered_ | `AE_LStr:Tracker3d` |
| TXT | ae_native | 14 | 0 | _no vendor description recovered_ | `AE_LStr:TXT` |
| Ultra Key | mediacore | 26 | 0 | UltraKey... | `MediaCore:AEUltraKey` |
| UOverride | ae_native | 118 | 0 | _no vendor description recovered_ | `AE_LStr:UOverride` |
| VAL | ae_native | 5 | 0 | _no vendor description recovered_ | `AE_LStr:VAL` |
| Variable Vibrance | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterVariableVibrance` |
| Vertical Flip | mediacore | 0 | 0 | Vertical Flip | `MediaCore:AEFilterVerticalFlip` |
| VR | mediacore | 229 | 0 | _no vendor description recovered_ | `MediaCore:VR` |
| VR Chroma Leaks | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRChromaLeaks` |
| VR Iris Wipe | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRIrisWipe` |
| VR Light Leaks | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRLightLeaks` |
| VR Light Rays | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRLightRays` |
| VR Mobius Zoom | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRMobiusZoom` |
| VR Projection | mediacore | 20 | 0 | VR Projection... | `MediaCore:AEVRProjection` |
| VR Random Blocks | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRRandomBlocks` |
| VR Spherical Blur | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRSphericalBlur` |

**Rows that dedup onto the compositing effect set of 14.9.3** (192 rows)

*Derivation: catalogue table listing rows that dedup onto an existing filter_kind under [STU-FX-127]; yields no microtask of its own.*

| Studio effect | Engine | Params | Presets | Description (from capture) | Import key (provenance) |
|---|---|---|---|---|---|
| 3D Channel Extract | ae_native | 22 | 0 | Displays auxiliary 3D data | `AE_LStr:3D_Channel_Extract` |
| 3D Glasses | ae_native | 12 | 0 | Composite two layers for stereoscopic viewing. | `ADBE 3D Glasses` |
| 4-Color Gradient | ae_native | 14 | 0 | Create color gradient of four blending color points. | `AE_LStr:4_Color_Gradient` |
| Add Grain | ae_native | 62 | 0 | Add film grain to an image. | `AE_LStr:Add_Grain` |
| Advanced Lightning | ae_native | 35 | 0 | Create lightning bolts. | `AE_LStr:Advanced_Lightning` |
| Auto Color | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:AEAutoColor` |
| Echo | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterEcho` |
| Lightning | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterLightning2` |
| Posterize Time | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterPosterize_Time` |
| Shadow/Highlight | mediacore | 1 | 0 | _no vendor description recovered_ | `MediaCore:AEShadowHighlight` |
| Texturize | mediacore | 9 | 0 | Color Texture, v%ld.%ld#{cr}#{cr}#{copy}2025-{{*CopyYear*}} Adobe Systems Inc. | `MediaCore:AETexture` |
| Alpha Levels | ae_native | 5 | 0 | Adjust alpha levels and gamma. | `AE_LStr:Alpha_Levels` |
| Arithmetic | ae_native | 6 | 0 | Perform miscellaneous arithmetic functions on image pixels. | `ADBE Arithmetic` |
| Audio Spectrum | ae_native | 24 | 0 | Displays the frequency spectrum of an audio layer. | `AE_LStr:Audio_Spectrum` |
| Audio Waveform | ae_native | 17 | 0 | Displays the waveform of an audio layer. | `ADBE Audio Waveform` |
| Auto Color | ae_native | 7 | 0 | Automatically adjust color by searching for shadows, midtones and highlights. | `AE_LStr:Auto_Color` |
| Auto Contrast | ae_native | 6 | 0 | Automatically adjust overall contrast. | `AE_LStr:Auto_Contrast` |
| Auto Levels | ae_native | 6 | 0 | Automatically adjust color channels individually. | `AE_LStr:Auto_Levels` |
| Backwards | ae_native | 1 | 0 | Time reverses the audio of a layer. | `AE_LStr:Backwards` |
| Basic 3D | ae_native | 8 | 0 | Transform the image in three dimensional space. | `ADBE Basic 3D` |
| Basic Text | ae_native | 31 | 0 | Performs basic character generation. | `AE_LStr:Basic_Text` |
| Bass & Treble | ae_native | 3 | 0 | Adjust the Bass & Treble of an audio layer. | `ADBE Aud BT` |
| Beam | ae_native | 12 | 0 | Displays a beam of light. | `AE_LStr:Beam` |
| Bevel Alpha | ae_native | 4 | 0 | Give the alpha boundaries of a layer a chiseled appearance. | `ADBE Bevel Alpha` |
| Bevel Edges | ae_native | 4 | 2 | _no vendor description recovered_ | `AE.ADBE Bevel Edges` |
| Bevel Edges | ae_native | 4 | 0 | Give a beveled appearance to layer edges. | `ADBE Bevel Edges` |
| Bezier Warp | ae_native | 16 | 0 | Apply a cubic coons warp to the image | `AE_LStr:Bezier_Warp` |
| Bilateral Blur | ae_native | 3 | 0 | Apply a bilateral blur to an image. | `ADBE Bilateral` |
| Blend | ae_native | 5 | 0 | Blend two layers together with different modes. The Crossfade mode is useful for fading between two layers that have transparent regions. | `ADBE Blend` |
| Block Dissolve | ae_native | 5 | 0 | Makes a layer disappear in random blocks. | `ADBE Block Dissolve` |
| Box Blur | ae_native | 5 | 0 | Apply repeated box blurs to an image. | `ADBE Box Blur` |
| Brightness & Contrast | ae_native | 2 | 0 | Adjust image brightness and contrast. | `AE_LStr:BrightnessAndContrast2` |
| Brightness & Contrast | ae_native | 2 | 0 | Adjust image brightness and contrast. | `AE_LStr:BrightnessAndContrast` |
| Broadcast Colors | ae_native | 4 | 0 | Adjust #{ldquo}hot#{rdquo} colors to be safe for broadcast. | `ADBE Broadcast Colors` |
| Brush Strokes | ae_native | 7 | 0 | Give a paint brushed appearance to an image. | `ADBE Brush Strokes` |
| Bulge | ae_native | 9 | 0 | Distort the image around a point. | `ADBE Bulge` |
| Calculations | ae_native | 12 | 0 | Performs arithmetic operations using source and input layer channels. | `ADBE Calculations` |
| Camera Lens Blur | ae_native | 28 | 0 | Blur images using common camera iris shapes to simulate the blur of a camera lens. | `AE_LStr:ShapeBlur` |
| Cartoon | ae_native | 18 | 0 | _no vendor description recovered_ | `AE_LStr:Cartoon` |
| Cell Pattern | ae_native | 23 | 0 | Creates cellular patterns. | `ADBE Cell Pattern` |
| Change Color | ae_native | 9 | 0 | Adjust hue, saturation, and lightness of a range of colors. | `ADBE Change Color` |
| Channel Blur | ae_native | 8 | 0 | Apply different amounts of blurring to red, green, blue and alpha channels. | `ADBE Channel Blur` |
| Channel Combiner | ae_native | 13 | 0 | View or move selected channels. | `ADBE Channel Combiner` |
| Checkerboard | ae_native | 14 | 0 | Creates a checkerboard pattern in the alpha channel.#{lf} | `ADBE CheckerBoard` |
| Cineon Converter | ae_native | 18 | 0 | Apply standard linear to logarithmic conversion curves. | `ADBE Cineon Converter` |
| Circle | ae_native | 19 | 0 | Creates a circle. | `ADBE Circle` |
| Color Balance | ae_native | 3 | 0 | Adjust strength of color channels. | `ADBE Color Balance` |
| Color Balance (HLS) | ae_native | 3 | 0 | Adjust strength of hue, lightness, and saturation channels. | `AE_LStr:Color_HLS` |
| Color Difference Key | ae_native | 19 | 0 | Color-difference keying. | `ADBE Color Difference Key` |
| Color Emboss | ae_native | 4 | 0 | Accentuate image edges at a given angle to simulate texture. | `ADBE Color Emboss` |
| Color Key | ae_native | 4 | 0 | Make a range close to a key color be transparent. | `ADBE Color Key` |
| Color Link | ae_native | 9 | 0 | Colorize a layer with the average color of a layer.#{lf} | `ADBE Color Link` |
| Color Profile Converter | ae_native | 17 | 0 | Convert an image between color spaces using ICC profiles. | `ADBE ProfileToProfile` |
| Color Range | ae_native | 10 | 0 | Key an image based on a range of colors. | `ADBE Color Range` |
| Color Stabilizer | ae_native | 8 | 0 | Stabilizes image exposure. | `AE_LStr:Color_Stabilizer` |
| Compound Arithmetic | ae_native | 6 | 0 | Perform arithmetic operations between layers. | `ADBE Compound Arithmetic` |
| Compound Blur | ae_native | 5 | 0 | Uses the luminance of another layer to blur pixels in current layer. | `ADBE Compound Blur` |
| Compressor | ae_native | 8 | 0 | An Audio Compressor for use in After Effects. | `ADBE Aud Compressor` |
| Corner Pin | ae_native | 5 | 0 | Distort an image to a convex quadrilateral. | `ADBE Corner Pin` |
| Curl Noise | ae_native | 39 | 0 | Creates fluid-like or swirling animated 2D noise that can be self-contained, based on the effects before it, or modulated by another layer. | `ADBE Curl Noise` |
| Curves | ae_native | 26 | 0 | Adjust tonal ranges of an image. | `AE_LStr:Curves` |
| Delay | ae_native | 7 | 0 | Applies delay to the audio of a layer. | `AE_LStr:Delay` |
| Depth Matte | ae_native | 12 | 0 | Matte a layer by depth. | `ADBE DEPTH MATTE` |
| Depth of Field | ae_native | 14 | 0 | Blur a layer by depth. | `AE_LStr:Depth_of_Field` |
| Detail-preserving Upscale | ae_native | 7 | 0 | Enlarge a layer (for example, from SD to HD) while preserving edge sharpness. Noise reduction can be applied at the same time. | `ADBE Upscale` |
| Difference Matte | ae_native | 6 | 0 | Key an image based on the colors from another layer. | `AE_LStr:Difference_Matte` |
| Directional Blur | ae_native | 6 | 0 | Blur an image directionally. | `AE_LStr:Directional_Blur` |
| Displacement Map | ae_native | 15 | 0 | Offset pixels based on another layer#{rsquo}s pixel values. | `ADBE Displacement Map` |
| Distortion | ae_native | 9 | 0 | A set of simple distortion models for use with Audio in After Effects. | `ADBE Aud Distortion` |
| Drop Shadow | ae_native | 9 | 0 | Draws a drop shadow based on the image#{rsquo}s alpha channel. | `ADBE Drop Shadow` |
| Dust & Scratches | ae_native | 5 | 0 | Replace a pixel with the median values within a given radius depending on the threshold. | `AE_LStr:Dust` |
| Echo | ae_native | 5 | 0 | Blend frames from different times. | `ADBE Echo` |
| Ellipse | ae_native | 9 | 0 | Draws a thick ellipse. | `ADBE ELLIPSE` |
| Emboss | ae_native | 4 | 0 | Impress image into a gray sheet with depth taken from differences at the given angle. | `ADBE Emboss` |
| Equalize | ae_native | 3 | 0 | Redistribute pixel values to represent a more even brightness balance. | `ADBE Equalize` |
| Exposure | ae_native | 18 | 0 | Adjust exposure in stops. | `ADBE Exposure` |
| Extract | ae_native | 8 | 0 | Key an image based on a range of one channel. | `ADBE Extract` |
| Eyedropper Fill | ae_native | 6 | 0 | Colorize a layer with color sampled from the layer.#{lf} | `ADBE Eyedropper Fill` |
| Fill | ae_native | 7 | 0 | Fill a path with a color. | `ADBE Fill` |
| Find Edges | ae_native | 2 | 0 | Find edges of a layer. | `ADBE Find Edges` |
| Flange & Chorus | ae_native | 10 | 0 | Applies Flange & Chorus to the audio of a layer. | `ADBE Aud_Flange` |
| Fog 3D | ae_native | 16 | 0 | Fog a layer by depth | `ADBE FOG_3D` |
| Fractal | ae_native | 32 | 0 | Generates Mandelbrot and Julia fractal images. | `ADBE Fractal` |
| Fractal Noise | ae_native | 35 | 0 | Create fractal based patterns. | `ADBE Fractal Noise` |
| Gate | ae_native | 4 | 0 | A Noise Gate for use in After Effects. | `ADBE Aud Gate` |
| Gaussian Blur | ae_native | 6 | 0 | Applies a Gaussian blur to an image. | `ADBE Gaussian Blur` |
| Gaussian Blur | mediacore | 7 | 0 | _no vendor description recovered_ | `MediaCore:AEGaussianBlur` |
| Glow | ae_native | 20 | 0 | Create glows based on alpha or color channels. | `AE_LStr:Glow` |
| Gradient Ramp | ae_native | 8 | 0 | Create a ramp of color. | `ADBE Ramp` |
| Gradient Wipe | ae_native | 6 | 0 | Use the luminance of another layer to create a wipe. | `ADBE Gradient Wipe` |
| Grid | ae_native | 17 | 0 | Render grids. | `ADBE Grid` |
| High-Low Pass | ae_native | 5 | 0 | Applies a high or low pass filter to the audio of a layer. | `AE_LStr:High_Low_Pass` |
| Hue/Saturation | ae_native | 36 | 0 | Photoshop Hue/Saturation/Lightness effect. | `ADBE HUE SATURATION` |
| ID Matte | ae_native | 15 | 0 | Matte a layer by material or object ID. | `ADBE ID MATTE` |
| Invert | ae_native | 4 | 0 | Reverse colors in a color space of your choice. | `ADBE Invert` |
| Iris Wipe | ae_native | 8 | 0 | Performs a star shaped wipe by modifying the alpha channel. | `ADBE IRIS_WIPE` |
| Leave Color | ae_native | 6 | 0 | Retain color information pixels similar to a given color. | `ADBE Leave Color` |
| Lens Flare | ae_native | 5 | 0 | Generate a synthetic lens flare. Originally written by John Knoll. | `ADBE Lens Flare` |
| Levels | ae_native | 42 | 0 | Adjust levels and gamma of an image. | `AE_LStr:Levels2` |
| Lightning | ae_native | 26 | 0 | Simulate electric arcs and lightning. | `ADBE Lightning` |
| Linear Color Key | ae_native | 8 | 0 | Key out pixels of a given color. | `AE_LStr:Linear_Color_Key` |
| Linear Wipe | ae_native | 3 | 0 | Performs a linear wipe by modifying the alpha channel. | `ADBE Linear Wipe` |
| Liquify | ae_native | 35 | 0 | Distort image by applying Liquify Brushes | `ADBE LIQUIFY` |
| Luma Key | ae_native | 6 | 0 | Make regions of image relative to a given luminance be transparent. | `ADBE Luma Key` |
| Luma Key | mediacore | 2 | 0 | LumaKey... | `MediaCore:AELumaKey` |
| Color Grade | lumetri | 98 | 325 | _no vendor description recovered_ | `AE.ADBE Lumetri` |
| Color Grade | mediacore | 137 | 0 | Color... | `MediaCore:AELumetri` |
| Magnify | ae_native | 13 | 0 | Magnify portion of a layer. | `ADBE Magnify` |
| Match Grain | ae_native | 69 | 0 | Matches film grain in an image. | `AE_LStr:Match_Grain` |
| Matte Choker | ae_native | 7 | 0 | Choke and spread alpha channels. | `ADBE Matte Choker` |
| Median | ae_native | 3 | 0 | Replace a pixel with the median values within a given radius. | `ADBE Median` |
| Mesh Warp | ae_native | 10 | 0 | Apply an n x m Coons warp to the image | `ADBE MESH WARP` |
| Minimax | ae_native | 5 | 0 | Replace each pixel with the minimum or maximum pixel value in a certain radius. | `ADBE Minimax` |
| Mirror | ae_native | 2 | 0 | Reflect an image across a line. | `ADBE Mirror` |
| Modulator | ae_native | 5 | 0 | Applies a modulation effect to the audio of a layer. | `AE_LStr:Modulator` |
| Mosaic | ae_native | 3 | 0 | Break an image up into rectangluar regions of solid color. | `ADBE Mosaic` |
| Motion Tile | ae_native | 11 | 0 | Tiles an image with motion blur. | `AE_LStr:Motion_Tile` |
| Noise | ae_native | 5 | 0 | Add noise to an image. | `ADBE Noise` |
| Noise Alpha | ae_native | 13 | 0 | Introduce noise to the alpha channel of a layer. | `ADBE Noise Alpha` |
| Noise HLS | ae_native | 8 | 0 | Introduce noise to the HLS channels of the layer.#{lf} | `ADBE Noise HLS` |
| Noise HLS Auto | ae_native | 8 | 0 | Introduce noise to the HLS channels of the layer.#{lf} | `ADBE Noise HLS Auto` |
| Numbers | ae_native | 30 | 0 | Generates ordered and random numerical sequences. | `AE_LStr:Numbers` |
| Offset | ae_native | 2 | 0 | Blend in an offset copy of the image. | `ADBE Offset` |
| Optics Compensation | ae_native | 10 | 0 | Introduce or remove lens distortion. | `ADBE Optics Compensation` |
| Paint Bucket | ae_native | 19 | 0 | A Paint bucket for RGB and Alpha. | `ADBE Paint Bucket` |
| Parametric EQ | ae_native | 11 | 0 | Applies Frequency Equalization to the audio of a layer. | `AE_LStr:Parametric_EQ` |
| Particle Playground | ae_native | 224 | 0 | A basic particle simulation effect. | `AE_LStr:Playground` |
| Path Text | ae_native | 70 | 0 | Draws text along a path. | `ADBE Path Text` |
| Photo Filter | ae_native | 5 | 0 | Simulates a colored lens filter. | `ADBE Photo Filter` |
| Bevel And Emboss | ae_native | 10 | 0 | Recreates Photoshop#{rsquo}s Bevel And Emboss layer effect | `AE_LStr:Photoshop_Bevel_And_Emboss` |
| Solid Fill | ae_native | 1 | 0 | Recreates Photoshop#{rsquo}s Solid Fill layer effect | `AE_LStr:Photoshop_Solid_Fill` |
| Polar Coordinates | ae_native | 3 | 0 | Convert and interpolate between rectangular and polar coordinate systems. | `ADBE Polar Coordinates` |
| Posterize | ae_native | 1 | 0 | Reduce color information in an image. | `ADBE Posterize` |
| Posterize Time | ae_native | 1 | 0 | Impose a specific frame rate on a layer. | `ADBE Posterize Time` |
| PS Arbitrary Map | ae_native | 7 | 0 | Apply Photoshop arbitrary maps to an image. | `ADBE PS Arbitrary Map` |
| Radial Blur | ae_native | 6 | 0 | Blur an image around a point. Portions by John Knoll. | `ADBE Radial Blur` |
| Radial Shadow | ae_native | 15 | 0 | Casts a projected shadow. | `ADBE Radial Shadow` |
| Radial Wipe | ae_native | 6 | 0 | Performs a radial wipe by modifying the alpha channel. | `ADBE Radial Wipe` |
| Reduce Interlace Flicker | ae_native | 1 | 0 | Suppress high vertical frequencies. | `ADBE Reduce Interlace Flicker` |
| Remove Color Matting | ae_native | 3 | 0 | Remove color haloing from a premultiplied image. | `ADBE Remove Color Matting` |
| Remove Grain | ae_native | 57 | 0 | Removes film grain from an image. | `AE_LStr:Remove_Grain` |
| Reverb | ae_native | 7 | 0 | Applies reverb to the audio of a layer. | `AE_LStr:Reverb` |
| Ripple | ae_native | 8 | 0 | Distort the image in a wave-like radial manner. | `ADBE Ripple` |
| Roughen Edges | ae_native | 17 | 0 | Roughens the alpha edges of a layer. | `ADBE Roughen Edges` |
| Scatter | ae_native | 4 | 0 | Scatters the pixels of an image, maintaining overall color levels. | `ADBE Scatter` |
| Set Channels | ae_native | 11 | 0 | Set channels of this layer to channels from other layers. | `ADBE Set Channels` |
| Set Matte | ae_native | 7 | 0 | Create traveling mattes. | `AE_LStr:Set_Matte` |
| Sharpen | ae_native | 1 | 0 | Sharpen an image by emphasizing differences between pixels. | `ADBE Sharpen` |
| Shift Channels | ae_native | 5 | 0 | Move around channels in the image. | `ADBE Shift Channels` |
| Simple Choker | ae_native | 2 | 0 | Choke and spread alpha channels. | `ADBE Simple Choker` |
| Slider Control | ae_native | 17 | 0 | Control for use with expressions. | `ADBE Slider Control` |
| Smart Blur | ae_native | 6 | 0 | Blur an image preserving the edges. | `ADBE Smart Blur` |
| Solid Composite | ae_native | 4 | 0 | Composites layer with a solid color. | `ADBE Solid Composite` |
| Spherize | ae_native | 3 | 0 | Distort the image around a point by stretching onto a half sphere of given radius. | `ADBE Spherize` |
| Spill Suppressor | ae_native | 3 | 0 | Remove color contamination from keyed layers. | `AE_LStr:Spill_Suppression` |
| Stereo Mixer | ae_native | 5 | 0 | Mixes the stereo channels of an audio. | `AE_LStr:Stereo_Mixer` |
| Strobe Light | ae_native | 8 | 0 | Perform arithmetic on a layer at regular and irregular intervals. | `AE_LStr:Strobe_Light` |
| Stroke | ae_native | 14 | 0 | Stroke mask outlines | `ADBE Stroke` |
| Texturize | ae_native | 5 | 0 | Use another layer to impart a texture to the current layer. | `ADBE Texturize` |
| Three-Way Color Corrector | mediacore | 2 | 0 | _no vendor description recovered_ | `MediaCore:ColorThreeWay` |
| Threshold | ae_native | 1 | 0 | Displays a black or white image based on gray levels. | `ADBE Threshold` |
| Time Difference | ae_native | 7 | 0 | Calculates the pixel difference between two layers. | `AE_LStr:Time_Difference` |
| Time Displacement | ae_native | 5 | 0 | Use another layer to displace the time of pixels in current layer. | `ADBE Time Displacement` |
| Timecode | ae_native | 18 | 0 | Read & burn timecode information. | `ADBE Timecode` |
| Timecode | mediacore | 32 | 0 | Timecode... | `MediaCore:AETimecode` |
| Timewarp | ae_native | 47 | 0 | _no vendor description recovered_ | `ADBE Timewarp` |
| Tint | ae_native | 4 | 0 | Map image brightness onto a scale from a #{ldquo}white color#{rdquo} to a #{ldquo}black color.#{rdquo} | `ADBE Tint` |
| Tone | ae_native | 8 | 0 | Renders an audio tone. | `AE_LStr:Tone` |
| Transform | ae_native | 16 | 0 | Performs geometric manipulation. | `AE_LStr:Transform` |
| Tritone | ae_native | 4 | 0 | Set highlight, midtone, and shadow colors. | `ADBE Tritone` |
| Turbulent Displace | ae_native | 19 | 0 | Displace a layer using fractal noise. | `ADBE Turbulent Displace` |
| Turbulent Noise | ae_native | 37 | 0 | Create turbulent based patterns. | `AE_LStr:Turbulent_Noise` |
| Twirl | ae_native | 5 | 0 | Smear the image by rotating around a given point. | `ADBE Twirl` |
| Unmult | ae_native | 6 | 0 | Creates transparency by converting black or white backgrounds to an alpha channel based on luminance, with adjustable softness for smooth transitions. | `ADBE Unmult` |
| Unsharp Mask | ae_native | 4 | 0 | Enhances sharpness of a layer by adjusting the contrast of edge details. | `ADBE Unsharp Mask` |
| Venetian Blinds | ae_native | 4 | 0 | Performs a directional, banded wipe by modifying the alpha channel. | `ADBE Venetian Blinds` |
| VR Blur | mediacore | 0 | 0 | VR Gaussian Blur... | `MediaCore:AEFilterVRGaussianBlur` |
| VR Chromatic Aberrations | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEFilterVRChromaticAberration` |
| VR Color Gradients | mediacore | 0 | 0 | VR Color Gradients... | `MediaCore:AEFilterVRColorGradient` |
| VR Converter | mediacore | 0 | 0 | VR Converter... | `MediaCore:AEFilterVRConverter` |
| VR De-Noise | mediacore | 0 | 0 | VR De-Noise... | `MediaCore:AEFilterVRDenoise` |
| VR Digital Glitch | mediacore | 0 | 0 | VR Digital Glitch... | `MediaCore:AEFilterVRDigitalGlitch` |
| VR Fractal Noise | mediacore | 0 | 0 | VR Fractal Noise... | `MediaCore:AEFilterVRFractalNoise` |
| VR Glow | mediacore | 0 | 0 | VR Glow... | `MediaCore:AEFilterVRGlow` |
| VR Gradient Wipe | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRGradientWipe` |
| VR Plane to Sphere | mediacore | 0 | 0 | VR Plane to Sphere... | `MediaCore:AEFilterVRProject2D` |
| VR Rotate Sphere | mediacore | 0 | 0 | VR Rotate Sphere... | `MediaCore:AEFilterVRRotateSphere` |
| VR Sharpen | mediacore | 0 | 0 | VR Sharpen... | `MediaCore:AEFilterVRSharpen` |
| VR Sphere To Plane | mediacore | 0 | 0 | _no vendor description recovered_ | `MediaCore:AEVRSphereToPlane` |
| Warp | ae_native | 9 | 0 | Apply a warp to the image | `AE_LStr:Warp` |
| Wave Warp | ae_native | 9 | 0 | Use a wave to distort a layer along an axis. | `ADBE Wave Warp` |
| Write-on | ae_native | 10 | 0 | Paint strokes onto an image | `AE_LStr:Write_on` |

**[STU-FX-133a] Every bounded parameter in the video-editing effect surface.** These 631 rows are
the complete set that declares a bound. Note the separate `soft_min`/`soft_max` columns: the same
hard/soft split of [STU-FX-105] is present here under different source field names, and it must
survive. Every parameter of every effect NOT in this table is `unbounded_in_source` and MUST be
implemented without a range check. `precision` is `--` on every row: this surface declares bounds,
defaults and units but no decimal count, and the column is present and empty rather than absent, so
that the seven fields of [STU-FX-105] are addressable on every row of every parameter table in this
sub-section. An implementer authors the decimal count deliberately under [STU-FX-109]; it is never
inferred from the number of digits a default happens to print.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Effect | Parameter | Type | hard_min | hard_max | soft_min | soft_max | default | unit | precision | keyframable |
|---|---|---|---|---|---|---|---|---|---|---|
| 2D Text Box | Measure At Time [sec] | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| 2D Text Box | Scale X | float | -1000000 | 1000000 | 0 | 100 | 100 | percent | -- | true |
| 2D Text Box | Scale Y | float | -1000000 | 1000000 | 0 | 100 | 100 | percent | -- | true |
| 2D Text Box | Padding Overall | float | -1000000 | 1000000 | 0 | 100 | 35 | -- | -- | true |
| 2D Text Box | Padding Left | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| 2D Text Box | Padding Right | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| 2D Text Box | Padding Top | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| 2D Text Box | Padding Bottom | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| 2D Text Box | Offset X | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| 2D Text Box | Offset Y | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| 2D Text Box | Roundness | float | -1000000 | 1000000 | 0 | 100 | 20 | -- | -- | true |
| 2D Text Box | Corner Sharpness | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Adaptive Noise Reduction | inf_reduce_noise_by | linear scalar | 0 | 40 | -- | -- | 20 | reduce_noise | -- | -- |
| Adaptive Noise Reduction | inf_noisiness | linear scalar | 0 | 100 | -- | -- | 30 | noisiness | -- | -- |
| Adaptive Noise Reduction | inf_fine_tune_noise_floor | linear scalar | -10 | 10 | -- | -- | 2 | fine_tune_noise_floor | -- | -- |
| Adaptive Noise Reduction | inf_signal_threshold | linear scalar | -20 | 20 | -- | -- | 2.5 | signal_threshold | -- | -- |
| Adaptive Noise Reduction | inf_spectral_decay | linear scalar | 20 | 750 | -- | -- | 140.00032 | spectral_decay | -- | -- |
| Adaptive Noise Reduction | inf_broadband_preservation | linear scalar | 0 | 500 | -- | -- | 100 | broadband_preservation | -- | -- |
| Animated Shape Control | Speed | float | -10000 | 10000 | 0 | 100 | 10 | -- | -- | true |
| Animated Shape Control | Rounding | float | -10000 | 10000 | 0 | 100 | 0 | -- | -- | true |
| Animated Shape Control | Spread | float | 0 | 10000 | 0 | 100 | 10 | -- | -- | true |
| Animated Shape Control | Speed | float | -10000 | 10000 | 0 | 100 | 10 | -- | -- | true |
| Animated Shape Control | Speed | float | -10000 | 10000 | 0 | 100 | 10 | -- | -- | true |
| AutoClickRemover | inf_threshold | linear scalar | 1 | 100 | -- | -- | 29.999971 | threshold | -- | -- |
| AutoClickRemover | inf_complexity | linear scalar | 1 | 100 | -- | -- | 15.999985 | complexity | -- | -- |
| Autoscroll - horizontal | Speed (pixels/second) | float | -10000 | 10000 | -1000 | 1000 | 100 | -- | -- | true |
| Autoscroll - vertical | Speed (pixels/second) | float | -10000 | 10000 | -1000 | 1000 | -25 | -- | -- | true |
| Bevel Edges | Edge Thickness | -- | 0 | 0.5 | -- | -- | -- | -- | -- | -- |
| Bevel Edges | Light Angle | -- | -32768 | 32767 | -- | -- | -- | -- | -- | -- |
| Bevel Edges | Light Color | -- | 0 | 1.8446744073709552e+19 | -- | -- | -- | -- | -- | -- |
| Bevel Edges | Light Intensity | -- | 0 | 1 | -- | -- | -- | -- | -- | -- |
| Bounce | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Bounce | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Bounce | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Bounce | Delay | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Bounce | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Bounce At Marker | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Bounce At Marker | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Bounce At Marker | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Bounce On Beat | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Bounce On Beat | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Bounce On Beat | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Bounce On Beat | Audio Threshold | float | 0 | 100 | -- | -- | 15 | -- | -- | true |
| Bounce Random | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Bounce Random | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Bounce Random | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Bounce Random | Min Delay | float | 0.1 | 100 | -- | 10 | 0.5 | -- | -- | true |
| Bounce Random | Max Delay | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Card Wipe Master Control | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Chaser Control | Box Step Speed | float | -10000 | 10000 | 0 | 100 | 10 | -- | -- | true |
| Chaser Control | Spread | float | 1 | 1000 | 1 | 100 | 10 | -- | -- | true |
| Chaser Control | Elements | float | 1 | 100 | -- | -- | 0 | -- | -- | true |
| Color Swirl | Transition Completion | float | 0 | 100 | -- | -- | 0 | percent | -- | true |
| Convolution Kernel | M11 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M12 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M13 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M21 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M22 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M23 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M31 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M32 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | M33 | -- | -30 | 30 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | Offset | -- | -32768 | 32767 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | Scale | -- | -32768 | 32767 | -- | -- | -- | -- | -- | -- |
| Convolution Kernel | Process Alpha | -- | false | true | -- | -- | -- | -- | -- | -- |
| Corner Reveal | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Corner Reveal | Feather | float | 0 | 1000 | 0 | 100 | 0 | -- | -- | true |
| Counter Controls | Animate Value | float | -1000000 | 1000000 | 0 | 100 | 100 | -- | -- | true |
| Counter Controls | 10x Value Multiplier | float | -1000000 | 1000000 | 0 | 100 | 2 | -- | -- | true |
| Counter Controls | Pad Zeros | float | -1000000 | 1000000 | 0 | 100 | 4 | -- | -- | true |
| Counter Controls | Digits After Decimal | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Counter Controls | Group Spacing | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Counter Controls | Group Separator Offset | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Cracked Tiles | Tile Cracking | float | 0 | 100 | -- | -- | 33 | percent | -- | true |
| Cracked Tiles | Tiles Across | float | 2 | 2000 | 2 | 200 | 50 | -- | -- | true |
| Crop Edges | Crop Amount (per edge) | float | 0 | 50 | -- | -- | 5 | percent | -- | true |
| Crop Edges | Feather | float | 0 | 1000 | 0 | 100 | 0 | -- | -- | true |
| Currency Controls | Animate Value | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Currency Controls | 10x Value Multiplier | float | -1000000 | 1000000 | 0 | 100 | 3 | -- | -- | true |
| Currency Controls | Pad Zeros | float | -1000000 | 1000000 | 0 | 100 | 5 | -- | -- | true |
| Currency Controls | Digits After Decimal | float | -1000000 | 1000000 | 0 | 100 | 2 | -- | -- | true |
| Currency Controls | Currency Symbol Scale | float | -1000000 | 1000000 | 0 | 100 | 100 | percent | -- | true |
| Currency Controls | Group Spacing | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Currency Controls | Group Separator Offset | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Dissolve - unmelt | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Dissolve - unmelt | Maximum Distortion | float | -10000 | 10000 | 0 | 5000 | 100 | -- | -- | true |
| Dissolve Master Control | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Drift Over Time | Speed (pixels/second) | float | -10000 | 10000 | -1000 | 1000 | 50 | -- | -- | true |
| Dynamics | attack_time | linear scalar | 0 | 500 | -- | -- | 0 | attack | -- | -- |
| Dynamics | release_time | linear scalar | 0 | 2000 | -- | -- | 0 | release | -- | -- |
| Dynamics | envelope_attack_time | linear scalar | 0 | 500 | -- | -- | 0 | attack | -- | -- |
| Dynamics | envelope_release_time | linear scalar | 0 | 2000 | -- | -- | 0 | release | -- | -- |
| Dynamics | look_ahead_time | linear scalar | 0 | 500 | -- | -- | 0 | attack | -- | -- |
| Dynamics | input_gain | linear scalar | -48 | 48 | -- | -- | -48 | decibel | -- | -- |
| Dynamics | output_gain | linear scalar | -48 | 48 | -- | -- | -48 | decibel | -- | -- |
| Dynamics | low_frequency | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Dynamics | high_frequency | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Dynamics | UNUSED1 | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Dynamics | link_channels | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Dynamics | use_rms | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Dynamics | use_splines | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Dynamics | use_noise_gate | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Dynamics | makeup_gain | linear scalar | 0 | 100 | -- | -- | 0 | makeup | -- | -- |
| Face Measurements | Offset X | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Offset Y | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Offset Z | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Face Scale | float | 0 | 30000 | 0 | 500 | 100 | percent | -- | true |
| Face Measurements | Left Eyebrow Distance | float | -30000 | 30000 | -500 | 500 | 100 | percent | -- | true |
| Face Measurements | Left Eyelid Openness | float | -30000 | 30000 | -500 | 500 | 100 | percent | -- | true |
| Face Measurements | Left Eye Gaze X | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Left Eye Gaze Y | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Right Eyebrow Distance | float | -30000 | 30000 | -500 | 500 | 100 | percent | -- | true |
| Face Measurements | Right Eyelid Openness | float | -30000 | 30000 | -500 | 500 | 100 | percent | -- | true |
| Face Measurements | Right Eye Gaze X | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Right Eye Gaze Y | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Mouth Offset X | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Mouth Offset Y | float | -30000 | 30000 | -500 | 500 | 0 | percent | -- | true |
| Face Measurements | Mouth Scale Width | float | -30000 | 30000 | -500 | 500 | 100 | percent | -- | true |
| Face Measurements | Mouth Scale Height | float | -30000 | 30000 | -500 | 500 | 100 | percent | -- | true |
| Face Track Points | Scale | float | -30000 | 30000 | -500 | 500 | 100 | percent | -- | true |
| Face Track Points | Orientation X | float | -359.9999 | 359.9999 | -359.9999 | 359.9999 | 0 | -- | -- | true |
| Face Track Points | Orientation Y | float | -359.9999 | 359.9999 | -359.9999 | 359.9999 | 0 | -- | -- | true |
| Face Track Points | Orientation Z | float | -359.9999 | 359.9999 | -359.9999 | 359.9999 | 0 | -- | -- | true |
| Fade In+Out - frames | Fade In Duration (frames) | float | 0 | 1000 | -- | -- | 15 | -- | -- | true |
| Fade In+Out - frames | Fade Out Duration (frames) | float | 0 | 1000 | -- | -- | 15 | -- | -- | true |
| Fade In+Out - msec | Fade In Duration (msec) | float | 0 | 30000 | 0 | 10000 | 500 | -- | -- | true |
| Fade In+Out - msec | Fade Out Duration (msec) | float | 0 | 30000 | 0 | 10000 | 500 | -- | -- | true |
| Fade Master Control | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Fast Blur | Blurriness | -- | 0 | 32767 | -- | -- | -- | -- | -- | -- |
| Fast Blur | Blur Dimensions | -- | 0 | 2 | -- | -- | -- | -- | -- | -- |
| Fast Blur |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Flanger | initial | linear scalar | 0 | 20 | -- | -- | 0 | delay | -- | -- |
| Flanger | final | linear scalar | 0 | 20 | -- | -- | 0 | delay | -- | -- |
| Flanger | phase | linear scalar | 0 | 360 | -- | -- | 0 | phase | -- | -- |
| Flanger | cycles | linear scalar | 0.001 | 60 | -- | -- | 0.001 | cycles | -- | -- |
| Flanger | mix | linear scalar | 0 | 100 | -- | -- | 0 | mix | -- | -- |
| Flanger | feedback | linear scalar | 0 | 100 | -- | -- | 0 | feedback | -- | -- |
| Flanger | beats | linear scalar | 0.03333 | 2000 | -- | -- | 0.03333 | beats | -- | -- |
| Fly to Inset | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Fly to Inset | Target Scale | float | 0 | 100 | -- | -- | 33.3 | percent | -- | true |
| Fly to Inset | Frame Size | float | 0 | 100 | 0 | 30 | 8 | -- | -- | true |
| Follow | Delay | float | 0 | 100 | -- | 10 | 0.1 | -- | -- | true |
| Getting Jiggy | Oh Boy 1 | float | -50 | 50 | -- | -- | 5 | -- | -- | true |
| Getting Jiggy | Oh Boy 2 | float | -50 | 50 | 2 | 8 | 5 | -- | -- | true |
| Getting Jiggy | Oh Boy Percent | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Getting Jiggy | Oh Boy Pixel | float | 0 | 1 | -- | -- | 0.5 | pixels | -- | true |
| GraphicEQ10Bands | accuracy | linear scalar | 10 | 8192 | -- | -- | 999.997454 | accuracy | -- | -- |
| GraphicEQ10Bands | gain | linear scalar | -64 | 64 | -- | -- | 0 | gain | -- | -- |
| GraphicEQ10Bands | range | linear scalar | 1.5 | 120 | -- | -- | 47.808615 | range | -- | -- |
| GraphicEQ10Bands | amp1 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp2 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp3 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp4 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp5 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp6 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp7 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp8 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp9 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ10Bands | amp10 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | accuracy | linear scalar | 10 | 8192 | -- | -- | 999.997454 | accuracy | -- | -- |
| GraphicEQ20Bands | gain | linear scalar | -64 | 64 | -- | -- | 0 | gain | -- | -- |
| GraphicEQ20Bands | range | linear scalar | 1.5 | 120 | -- | -- | 47.808615 | range | -- | -- |
| GraphicEQ20Bands | amp1 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp2 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp3 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp4 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp5 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp6 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp7 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp8 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp9 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp10 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp11 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp12 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp13 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp14 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp15 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp16 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp17 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp18 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp19 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ20Bands | amp20 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | accuracy | linear scalar | 10 | 8192 | -- | -- | 999.997454 | accuracy | -- | -- |
| GraphicEQ30Bands | gain | linear scalar | -64 | 64 | -- | -- | 0 | gain | -- | -- |
| GraphicEQ30Bands | range | linear scalar | 1.5 | 120 | -- | -- | 47.808615 | range | -- | -- |
| GraphicEQ30Bands | amp1 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp2 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp3 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp4 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp5 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp6 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp7 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp8 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp9 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp10 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp11 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp12 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp13 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp14 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp15 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp16 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp17 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp18 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp19 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp20 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp21 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp22 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp23 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp24 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp25 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp26 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp27 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp28 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp29 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| GraphicEQ30Bands | amp30 | linear scalar | -70 | 70 | -- | -- | 0 | amp | -- | -- |
| Grid Wipe | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Grid Wipe | Cell Size | float | 1 | 2000 | -- | -- | 20 | -- | -- | true |
| Grid Wipe | Feather | float | 0 | 1000 | 0 | 100 | 0 | -- | -- | true |
| Hard Limiter | max_amp | linear scalar | -100 | 0 | -- | -- | -50 | max_amp | -- | -- |
| Hard Limiter | input_boost | linear scalar | -100 | 50 | -- | -- | 20 | input_boost | -- | -- |
| Hard Limiter | lookahead_time | linear scalar | 5 | 20 | -- | -- | 7.1 | lookahead_time | -- | -- |
| Hard Limiter | release_time | linear scalar | 40 | 200 | -- | -- | 100 | release_time | -- | -- |
| Inset Video - framed | Crop Amount | float | 0 | 50 | -- | -- | 5 | percent | -- | true |
| Inset Video - framed | Frame Size | float | 0 | 100 | 0 | 30 | 8 | -- | -- | true |
| Inset Video - torn edges | Crop Amount | float | 0 | 50 | -- | -- | 5 | percent | -- | true |
| Iris Wipe Master Controls | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Iris Wipe Master Controls | Feather | float | 0 | 1000 | 0 | 100 | 0 | -- | -- | true |
| Jiggle | Amplitude | float | 0 | 100 | -- | 100 | 50 | -- | -- | true |
| Jiggle | Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Jiggle | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Jiggle | Delay | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Jiggle | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Jiggle At Marker | Amplitude | float | 0 | 100 | -- | 100 | 50 | -- | -- | true |
| Jiggle At Marker | Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Jiggle At Marker | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Jiggle On Beat | Amplitude | float | 0 | 100 | -- | 100 | 50 | -- | -- | true |
| Jiggle On Beat | Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Jiggle On Beat | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Jiggle On Beat | Audio Threshold | float | 0 | 100 | -- | -- | 15 | -- | -- | true |
| Jiggle Random | Amplitude | float | 0 | 100 | -- | 100 | 50 | -- | -- | true |
| Jiggle Random | Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Jiggle Random | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Jiggle Random | Min Delay | float | 0.1 | 100 | -- | 10 | 0.5 | -- | -- | true |
| Jiggle Random | Max Delay | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Light Leaks - layer markers | Flash Width (msec) | float | 1 | 10000 | 1 | 1000 | 200 | -- | -- | true |
| Light Leaks - random | Chance of Flashing | float | 0 | 100 | -- | -- | 100 | percent | -- | true |
| Light Leaks - random | Flash Nervousness | float | 1 | 10000 | 1 | 500 | 50 | -- | -- | true |
| Color Grade | Blob | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | Basic Correction | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade | Input LUT | -- | 0 | 998 | -- | -- | -- | -- | -- | -- |
| Color Grade | HDR White | -- | 100 | 1000 | -- | -- | -- | -- | -- | -- |
| Color Grade | White Balance | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | WB Selector | -- | 0 | 1.8446744073709552e+19 | -- | -- | -- | -- | -- | -- |
| Color Grade | Temperature | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | Tint | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Tone | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Exposure | -- | -7 | 7 | -- | -- | -- | -- | -- | -- |
| Color Grade | Contrast | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | Highlights | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | Shadows | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | Whites | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | Blacks | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | HDR Specular | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Saturation | -- | 0 | 300 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Creative | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade | Look | -- | 0 | 998 | -- | -- | -- | -- | -- | -- |
| Color Grade | Intensity | -- | 0 | 200 | -- | -- | -- | -- | -- | -- |
| Color Grade | Adjustments | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Faded Film | -- | 0 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | Sharpen | -- | -100 | 100 | -- | -- | -- | -- | -- | -- |
| Color Grade | Vibrance | -- | -100 | 100 | -- | -- | -- | -- | -- | -- |
| Color Grade | Saturation | -- | 0 | 300 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade | Tint Balance | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Curves | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | RGB Curves | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | HDR Range | -- | 100 | 10000 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Hue Saturation Curve | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Color Wheels | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | HDR White | -- | 100 | 1000 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | HSL Secondary | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | Key | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Set color | -- | 0 | 1.8446744073709552e+19 | -- | -- | -- | -- | -- | -- |
| Color Grade | Add color | -- | 0 | 1.8446744073709552e+19 | -- | -- | -- | -- | -- | -- |
| Color Grade | Remove color | -- | 0 | 1.8446744073709552e+19 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | 0 | 2 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Refine | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Denoise | -- | 0 | 100 | -- | -- | -- | -- | -- | -- |
| Color Grade | Blur | -- | 0 | 1000 | -- | -- | -- | -- | -- | -- |
| Color Grade | Blur | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Correction | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade | Temperature | -- | -300 | 300 | -- | -- | -- | -- | -- | -- |
| Color Grade | Tint | -- | -300 | 300 | -- | -- | -- | -- | -- | -- |
| Color Grade | Contrast | -- | -150 | 150 | -- | -- | -- | -- | -- | -- |
| Color Grade | Sharpen | -- | -100 | 100 | -- | -- | -- | -- | -- | -- |
| Color Grade | Saturation | -- | 0 | 300 | -- | -- | -- | -- | -- | -- |
| Color Grade | Saturation | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Vignette | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | Amount | -- | -5 | 5 | -- | -- | -- | -- | -- | -- |
| Color Grade | Midpoint | -- | 0 | 100 | -- | -- | -- | -- | -- | -- |
| Color Grade | Roundness | -- | -100 | 100 | -- | -- | -- | -- | -- | -- |
| Color Grade | Feather | -- | 0 | 100 | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | SpeedGrade Custom | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade | Custom Layer | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | unused | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | unused | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | false | -- | -- | -- | -- | -- | -- |
| Color Grade |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Color Grade | Embedded LUTs | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Mask Fade Controls | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Mask Fade Controls | Feather | float | 0 | 1000 | 0 | 100 | 0 | -- | -- | true |
| Mood Lighting - amorphous | Evolution Speed | float | -1000 | 1000 | -500 | 500 | 150 | -- | -- | true |
| Mood Lighting - amorphous | Cloud Size | float | 1 | 5000 | 10 | 500 | 75 | -- | -- | true |
| Mood Lighting - amorphous | Intensity | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Mood Lighting - digital | Evolution Speed | float | -1000 | 1000 | -500 | 500 | 100 | -- | -- | true |
| Mood Lighting - digital | Block Size | float | 1 | 5000 | 20 | 1000 | 250 | -- | -- | true |
| Mood Lighting - digital | Intensity | float | 0 | 100 | -- | -- | 75 | percent | -- | true |
| Mood Lighting - streaks | Evolution Speed | float | -1000 | 1000 | -500 | 500 | 200 | -- | -- | true |
| Mood Lighting - streaks | Streak Width | float | 1 | 1000 | 10 | 500 | 75 | -- | -- | true |
| Mood Lighting - streaks | Intensity | float | 0 | 100 | -- | -- | 75 | percent | -- | true |
| Motion | Position | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Motion | Scale | -- | 0 | 600 | -- | -- | -- | -- | -- | -- |
| Motion | Scale Width | -- | 0 | 600 | -- | -- | -- | -- | -- | -- |
| Motion |  | -- | false | true | -- | -- | -- | -- | -- | -- |
| Motion | Rotation | -- | -32768 | 32767 | -- | -- | -- | -- | -- | -- |
| Motion | Anchor Point | -- | -- | -- | -- | -- | -- | -- | -- | -- |
| Opacity Flash - layer markers | Flash Width (msec) | float | 1 | 10000 | 1 | 2000 | 500 | -- | -- | true |
| Opacity Flash - random | Chance of Flashing | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Opacity Flash - random | Flash Nervousness | float | 1 | 10000 | 1 | 500 | 50 | -- | -- | true |
| Opacity Pulse | Min Opacity | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Opacity Pulse | Max Opacity | float | 0 | 100 | -- | -- | 100 | -- | -- | true |
| Opacity Pulse | Attack | float | 0 | 25 | -- | 15 | 10 | -- | -- | true |
| Opacity Pulse | Decay | float | 0 | 25 | -- | 15 | 5 | -- | -- | true |
| Opacity Pulse | Delay | float | 0 | 100 | -- | -- | 0.01 | -- | -- | true |
| Opacity Pulse | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Opacity Pulse At Marker | Min Opacity | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Opacity Pulse At Marker | Max Opacity | float | 0 | 100 | -- | -- | 100 | -- | -- | true |
| Opacity Pulse At Marker | Attack | float | 0 | 25 | -- | 15 | 10 | -- | -- | true |
| Opacity Pulse At Marker | Decay | float | 0 | 25 | -- | 15 | 5 | -- | -- | true |
| Opacity Pulse On Beat | Min Opacity | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Opacity Pulse On Beat | Max Opacity | float | 0 | 100 | -- | -- | 100 | -- | -- | true |
| Opacity Pulse On Beat | Attack | float | 0 | 25 | -- | 15 | 10 | -- | -- | true |
| Opacity Pulse On Beat | Decay | float | 0 | 25 | -- | 15 | 5 | -- | -- | true |
| Opacity Pulse On Beat | Audio Threshold | float | 0 | 100 | -- | -- | 15 | -- | -- | true |
| Opacity Pulse Random | Min Opacity | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Opacity Pulse Random | Max Opacity | float | 0 | 100 | -- | -- | 100 | -- | -- | true |
| Opacity Pulse Random | Attack | float | 0 | 25 | -- | 15 | 10 | -- | -- | true |
| Opacity Pulse Random | Decay | float | 0 | 25 | -- | 15 | 5 | -- | -- | true |
| Opacity Pulse Random | Min Delay | float | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | true |
| Opacity Pulse Random | Max Delay | float | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | true |
| Orbit | Radius | float | 0 | 3000 | -- | 500 | 150 | -- | -- | true |
| Orbit | Frequency | float | -30 | 30 | -15 | 15 | 0.5 | -- | -- | true |
| Orbit 3D | Radius | float | 0 | 3000 | -- | 500 | 150 | -- | -- | true |
| Orbit 3D | Frequency | float | -30 | 30 | -15 | 15 | 0.5 | -- | -- | true |
| Orbit 3D | Elevation | float | -3000 | 3000 | -500 | 500 | 0 | -- | -- | true |
| Oscillate | Amplitude | float | -1000 | 1000 | 0 | -- | 150 | -- | -- | true |
| Oscillate | Frequency | float | 0 | 30 | -- | 15 | 5 | -- | -- | true |
| Oscillate | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Oscillate | Delay | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Oscillate | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Oscillate At Marker | Amplitude | float | -1000 | 1000 | 0 | -- | 150 | -- | -- | true |
| Oscillate At Marker | Frequency | float | 0 | 30 | -- | 15 | 5 | -- | -- | true |
| Oscillate At Marker | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Oscillate On Beat | Amplitude | float | -1000 | 1000 | 0 | -- | 150 | -- | -- | true |
| Oscillate On Beat | Frequency | float | 0 | 30 | -- | 15 | 5 | -- | -- | true |
| Oscillate On Beat | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Oscillate On Beat | Audio Threshold | float | 0 | 100 | -- | -- | 15 | -- | -- | true |
| Oscillate Random | Amplitude | float | -1000 | 1000 | 0 | -- | 150 | -- | -- | true |
| Oscillate Random | Frequency | float | 0 | 30 | -- | 15 | 5 | -- | -- | true |
| Oscillate Random | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Oscillate Random | Min Delay | float | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | true |
| Oscillate Random | Max Delay | float | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | true |
| Parametric Equalizer | low_filter_range | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | lpcutoff | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | lpamp | linear scalar | -48 | 48 | -- | -- | -48 | amp | -- | -- |
| Parametric Equalizer | hpcutoff | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | hpamp | linear scalar | -48 | 48 | -- | -- | -48 | amp | -- | -- |
| Parametric Equalizer | center1 | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | amp1 | linear scalar | -48 | 48 | -- | -- | -48 | amp | -- | -- |
| Parametric Equalizer | enable1 | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | center2 | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | amp2 | linear scalar | -48 | 48 | -- | -- | -48 | amp | -- | -- |
| Parametric Equalizer | enable2 | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | center3 | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | amp3 | linear scalar | -48 | 48 | -- | -- | -48 | amp | -- | -- |
| Parametric Equalizer | enable3 | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | center4 | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | amp4 | linear scalar | -48 | 48 | -- | -- | -48 | amp | -- | -- |
| Parametric Equalizer | enable4 | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | center5 | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | amp5 | linear scalar | -48 | 48 | -- | -- | -48 | amp | -- | -- |
| Parametric Equalizer | enable5 | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | gain | linear scalar | -96 | 48 | -- | -- | -96 | gain | -- | -- |
| Parametric Equalizer | constant_q | boolean | false | true | -- | -- | 1 | -- | -- | -- |
| Parametric Equalizer | low2ndorder | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | high2ndorder | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | ultraquiet | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | lpenable | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | hpenable | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | q1 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q2 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q3 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q4 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q5 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q_width1 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q_width2 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q_width3 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q_width4 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | q_width5 | linear scalar | 0.0001 | 10000 | -- | -- | 0.0001 | q | -- | -- |
| Parametric Equalizer | hipassenable | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | hipassfreq | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Parametric Equalizer | lopassenable | boolean | false | true | -- | -- | 0 | -- | -- | -- |
| Parametric Equalizer | lopassfreq | linear scalar | 20 | sample_rate / 2 | -- | -- | -- | filter | -- | -- |
| Pattern Template | Pattern Size | float | -1000000 | 1000000 | 0 | 100 | 100 | percent | -- | true |
| Pattern Template | Repeat Width (% of Comp) | float | -1000000 | 1000000 | 0 | 100 | 100 | percent | -- | true |
| Pattern Template | Repeat Height (% of Comp) | float | -1000000 | 1000000 | 0 | 100 | 100 | percent | -- | true |
| Pattern Template | Animation Speed | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Pendulum | Amplitude | float | -180 | 180 | -- | -- | 45 | -- | -- | true |
| Pendulum | Frequency | float | 0 | 30 | -- | 15 | 3 | -- | -- | true |
| Pendulum | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Pendulum | Delay | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Pendulum | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Pendulum At Marker | Amplitude | float | -180 | 180 | -- | -- | 45 | -- | -- | true |
| Pendulum At Marker | Frequency | float | 0 | 30 | -- | 15 | 3 | -- | -- | true |
| Pendulum At Marker | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Pendulum On Beat | Amplitude | float | -180 | 180 | -- | -- | 45 | -- | -- | true |
| Pendulum On Beat | Frequency | float | 0 | 30 | -- | 15 | 3 | -- | -- | true |
| Pendulum On Beat | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Pendulum On Beat | Audio Threshold | float | 0 | 100 | -- | -- | 15 | -- | -- | true |
| Pendulum Random | Amplitude | float | -180 | 180 | -- | -- | 45 | -- | -- | true |
| Pendulum Random | Frequency | float | 0 | 30 | -- | 15 | 3 | -- | -- | true |
| Pendulum Random | Decay | float | 0 | 25 | -- | 15 | 1 | -- | -- | true |
| Pendulum Random | Min Delay | float | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | true |
| Pendulum Random | Max Delay | float | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | true |
| Percentage Controls | Animate Value | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Percentage Controls | 10x Value Multiplier | float | -1000000 | 1000000 | 0 | 100 | 1 | -- | -- | true |
| Percentage Controls | Pad Zeros | float | -1000000 | 1000000 | 0 | 100 | 3 | -- | -- | true |
| Percentage Controls | Digits After Decimal | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Percentage Controls | Percent Symbol Scale | float | -1000000 | 1000000 | 0 | 100 | 100 | percent | -- | true |
| Percentage Controls | Group Spacing | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Percentage Controls | Group Separator Offset | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Pulse | Amplitude | float | -100 | 300 | -- | 100 | 50 | -- | -- | true |
| Pulse | Decay | float | 0 | 25 | -- | 15 | 4 | -- | -- | true |
| Pulse | Delay | float | 0 | 100 | -- | -- | 0.01 | -- | -- | true |
| Pulse | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Pulse At Marker | Amplitude | float | -100 | 300 | -- | 100 | 50 | -- | -- | true |
| Pulse At Marker | Decay | float | 0 | 25 | -- | 15 | 4 | -- | -- | true |
| Pulse On Beat | Amplitude | float | -100 | 300 | -- | 100 | 50 | -- | -- | true |
| Pulse On Beat | Decay | float | 0 | 25 | -- | 15 | 4 | -- | -- | true |
| Pulse On Beat | Audio Threshold | float | 0 | 100 | -- | -- | 15 | -- | -- | true |
| Pulse Random | Amplitude | float | -100 | 300 | -- | 100 | 50 | -- | -- | true |
| Pulse Random | Decay | float | 4 | 25 | -- | 15 | 3 | -- | -- | true |
| Pulse Random | Min Delay | float | 0.1 | 100 | -- | 10 | 0.5 | -- | -- | true |
| Pulse Random | Max Delay | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Radial Wipe Master Controls | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Radial Wipe Master Controls | Feather | float | 0 | 1000 | 0 | 100 | 0 | -- | -- | true |
| Random Fill Color | Transition Time | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Random Fill Color | Transition Time Variation | float | 0 | 100 | -- | 5 | 0.5 | -- | -- | true |
| Random Fill Color | Max Hue Variation | float | 0 | 180 | -- | -- | 60 | -- | -- | true |
| Random Fill Color | Max Saturation Variation | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Random Fill Color | Max Lightness Variation | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Random Motion | Travel Time | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Random Motion | Travel Time Variation | float | 0 | 100 | -- | 5 | 0.5 | -- | -- | true |
| Random Motion | Horizontal Range | float | 0 | 3000 | -- | 1000 | 300 | -- | -- | true |
| Random Motion | Vertical Range | float | 0 | 3000 | -- | 1000 | 220 | -- | -- | true |
| Random Motion 1D | Travel Time | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Random Motion 1D | Travel Time Variation | float | 0 | 100 | -- | 5 | 0.5 | -- | -- | true |
| Random Motion 1D | Range | float | 0 | 3000 | -- | 1000 | 300 | -- | -- | true |
| Random Opacity | Travel Time | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Random Opacity | Travel Time Variation | float | 0 | 100 | -- | 5 | 0.5 | -- | -- | true |
| Random Opacity | Min Opacity | float | 0 | 100 | -- | -- | 10 | -- | -- | true |
| Random Opacity | Max Opacity | float | 0 | 100 | -- | -- | 100 | -- | -- | true |
| Random Rotation | Travel Time | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Random Rotation | Travel Time Variation | float | 0 | 100 | -- | 5 | 0.5 | -- | -- | true |
| Random Rotation | Min Rotation | float | -3600 | 3600 | -1000 | 1000 | -360 | -- | -- | true |
| Random Rotation | Max Rotation | float | -3600 | 3600 | -1000 | 1000 | 360 | -- | -- | true |
| Random Rotation 3D | Travel Time | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Random Rotation 3D | Travel Time Variation | float | 0 | 100 | -- | 5 | 0.5 | -- | -- | true |
| Random Rotation 3D | Min X Rotation | float | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | true |
| Random Rotation 3D | Max X Rotation | float | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | true |
| Random Rotation 3D | Min Y Rotation | float | -3600 | 3600 | -1000 | 1000 | -360 | -- | -- | true |
| Random Rotation 3D | Max Y Rotation | float | -3600 | 3600 | -1000 | 1000 | 360 | -- | -- | true |
| Random Rotation 3D | Min Z Rotation | float | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | true |
| Random Rotation 3D | Max Z Rotation | float | -3600 | 3600 | -1000 | 1000 | 0 | -- | -- | true |
| Random Scale | Travel Time | float | 0.1 | 100 | -- | 10 | 1 | -- | -- | true |
| Random Scale | Travel Time Variation | float | 0 | 100 | -- | 5 | 0.5 | -- | -- | true |
| Random Scale | Min Scale | float | 0 | 1000 | -- | 500 | 25 | -- | -- | true |
| Random Scale | Max Scale | float | 0 | 1000 | -- | 500 | 100 | -- | -- | true |
| Sample Image | Radius | float | 0.01 | 10000 | 0.01 | 2048 | 0.5 | -- | -- | true |
| Scale Bounce - layer markers | Bounce Duration (msec) | float | 1 | 10000 | 1 | 2000 | 500 | -- | -- | true |
| Scale Bounce - layer markers | Target Scale Change | float | 0 | 10000 | 0 | 1000 | 200 | percent | -- | true |
| Scale Bounce - random | Chance of Bouncing | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Scale Bounce - random | Bounce Nervousness | float | 1 | 10000 | 1 | 500 | 50 | -- | -- | true |
| Scale Bounce - random | Target Scale Change | float | 0 | 10000 | 0 | 1000 | 200 | percent | -- | true |
| Separate XYZ Position | X Position | float | -30000 | 30000 | -1000 | 1000 | 0 | -- | -- | true |
| Separate XYZ Position | Y Position | float | -30000 | 30000 | -1000 | 1000 | 0 | -- | -- | true |
| Separate XYZ Position | Z Position | float | -30000 | 30000 | -1000 | 1000 | 0 | -- | -- | true |
| Separate XYZ Scale | X Scale | float | -1000000 | 1000000 | -100 | 100 | 100 | percent | -- | true |
| Separate XYZ Scale | Y Scale | float | -1000000 | 1000000 | -100 | 100 | 100 | percent | -- | true |
| Separate XYZ Scale | Z Scale | float | -1000000 | 1000000 | -100 | 100 | 100 | percent | -- | true |
| Slide - variable | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Slide - variable | Initial Scale | float | 0 | 500 | 0 | 200 | 25 | percent | -- | true |
| Slide Master Control | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Solarize | Threshold | -- | 0 | 254 | -- | -- | -- | -- | -- | -- |
| Stereo 3D Controls | Stereo Scene Depth | float | 0 | 100 | 0 | 100 | 3 | percent | -- | true |
| Stereo 3D Controls | Convergence Z Offset | float | -50000 | 50000 | -1000 | 1000 | 0 | -- | -- | true |
| Stretch Master Control | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Stretch Master Control (edge) | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Stretch Master Control(corner) | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Studio Reverb | lowcut | linear scalar | 20 | 4000 | -- | -- | 879.9984 | lowcut | -- | -- |
| Studio Reverb | highcut | linear scalar | 20 | 20000 | -- | -- | 13800.0062 | highcut | -- | -- |
| Studio Reverb | roomsize | linear scalar | 1 | 100 | -- | -- | 70.00003 | roomsize | -- | -- |
| Studio Reverb | width | linear scalar | 0 | 100 | -- | -- | 25 | width | -- | -- |
| Studio Reverb | diffusion | linear scalar | 0 | 100 | -- | -- | 50 | diffusion | -- | -- |
| Studio Reverb | damping | linear scalar | 0 | 100 | -- | -- | 50 | damping | -- | -- |
| Studio Reverb | decay | linear scalar | 200 | 10000 | -- | -- | 2500.0012 | decay | -- | -- |
| Studio Reverb | earlyreflections | linear scalar | 0 | 100 | -- | -- | 52 | earlyreflections | -- | -- |
| Studio Reverb | dry | linear scalar | 0 | 100 | -- | -- | 75 | dry | -- | -- |
| Studio Reverb | wet | linear scalar | 0 | 100 | -- | -- | 25 | wet | -- | -- |
| Swarm | Amplitude | float | 0 | 1000 | -- | 500 | 100 | -- | -- | true |
| Swarm | Frequency | float | 0 | 30 | -- | 15 | 3 | -- | -- | true |
| Timer Controls | Animate Value | float | -1000000 | 1000000 | 0 | 100 | 9.9999 | -- | -- | true |
| Timer Controls | Label Size | float | -1000000 | 1000000 | 0 | 100 | 32 | -- | -- | true |
| Timer Controls | Label Spacing | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Timer Controls | Spacing | float | -1000000 | 1000000 | 0 | 100 | 0 | -- | -- | true |
| Trace Path | Progress | float | 0 | 100 | 0 | 100 | 0 | percent | -- | true |
| Transition Master Control | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Wiggle - gelatin | Wiggle Speed (wigs/sec) | float | 0 | 100 | -- | -- | 1 | -- | -- | true |
| Wiggle - gelatin | Wiggle Amount | float | 0 | 70 | -- | -- | 20 | -- | -- | true |
| Wiggle - position | Wiggle Speed (wigs/sec) | float | 0 | 100 | -- | -- | 1 | -- | -- | true |
| Wiggle - position | Wiggle Amount (pixels) | float | 0 | 10000 | 0 | 500 | 50 | -- | -- | true |
| Wiggle - rotation | Wiggle Speed (wigs/sec) | float | 0 | 100 | -- | -- | 1 | -- | -- | true |
| Wiggle - scale | Wiggle Speed (wigs/sec) | float | 0 | 100 | -- | -- | 1 | -- | -- | true |
| Wiggle - scale | Wiggle Amount | float | 0 | 10000 | 0 | 1000 | 10 | percent | -- | true |
| Wiggle - scale | Wiggle Width | float | 0 | 10000 | 0 | 1000 | 10 | percent | -- | true |
| Wiggle - shear | Wiggle Speed (wigs/sec) | float | 0 | 100 | -- | -- | 1 | -- | -- | true |
| Wiggle - shear | Wiggle Amount | float | 0 | 70 | -- | -- | 20 | -- | -- | true |
| Wigglerama | Wiggle Speed (wigs/sec) | float | 0 | 100 | -- | -- | 1 | -- | -- | true |
| Wigglerama | Wiggle Nervousness | float | 1 | 20 | -- | -- | 1 | -- | -- | true |
| Wigglerama | Wiggle Position (pixels) | float | 0 | 10000 | 0 | 500 | 25 | -- | -- | true |
| Wigglerama | Wiggle Scale | float | 0 | 10000 | 0 | 1000 | 15 | percent | -- | true |
| Wigglerama | Wiggle Width | float | 0 | 10000 | 0 | 1000 | 10 | percent | -- | true |
| Wipe Master Control | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Wipe Master Controls | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Wipe Master Controls | Feather | float | 0 | 1000 | 0 | 100 | 0 | -- | -- | true |
| Wobble Bounce | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Wobble Bounce | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Wobble Bounce | Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce | Wobble Amplitude | float | 0 | 100 | -- | 100 | 35 | -- | -- | true |
| Wobble Bounce | Wobble Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Wobble Bounce | Wobble Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce | Delay | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Wobble Bounce | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Wobble Bounce At Marker | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Wobble Bounce At Marker | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Wobble Bounce At Marker | Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce At Marker | Wobble Amplitude | float | 0 | 100 | -- | 100 | 35 | -- | -- | true |
| Wobble Bounce At Marker | Wobble Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Wobble Bounce At Marker | Wobble Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce On Beat | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Wobble Bounce On Beat | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Wobble Bounce On Beat | Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce On Beat | Wobble Amplitude | float | 0 | 100 | -- | 100 | 35 | -- | -- | true |
| Wobble Bounce On Beat | Wobble Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Wobble Bounce On Beat | Wobble Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce On Beat | Audio Threshold | float | 0 | 100 | -- | -- | 15 | -- | -- | true |
| Wobble Bounce Random | Amplitude | float | -1000 | 1000 | 0 | -- | 250 | -- | -- | true |
| Wobble Bounce Random | Frequency | float | 0 | 30 | -- | 15 | 1 | -- | -- | true |
| Wobble Bounce Random | Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce Random | Wobble Amplitude | float | 0 | 100 | -- | 100 | 35 | -- | -- | true |
| Wobble Bounce Random | Wobble Frequency | float | 0 | 30 | -- | 15 | 4 | -- | -- | true |
| Wobble Bounce Random | Wobble Decay | float | 0 | 25 | -- | 15 | 0.7 | -- | -- | true |
| Wobble Bounce Random | Min Delay | float | 0.1 | 100 | -- | 10 | 0.75 | -- | -- | true |
| Wobble Bounce Random | Max Delay | float | 0.1 | 100 | -- | 10 | 1.5 | -- | -- | true |
| Wobble Bounce Random | Internal Use Only | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Z Spring | Amplitude | float | -1000 | 1000 | -- | -- | -500 | -- | -- | true |
| Z Spring | Frequency | float | 0 | 30 | -- | 15 | 3 | -- | -- | true |
| Z Spring | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Z Spring | Wander Amount | float | 0 | 500 | -- | -- | 50 | -- | -- | true |
| Z Spring | Rotational Amplitude | float | -360 | 360 | -- | -- | 50 | -- | -- | true |
| Z Spring | Rotational Frequency | float | 0 | 30 | -- | 15 | 1.5 | -- | -- | true |
| Z Spring | Delay | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Z Spring | Cycle Time | float | 0 | 100 | -- | -- | 0 | -- | -- | true |
| Z Spring At Marker | Amplitude | float | -1000 | 1000 | -- | -- | -500 | -- | -- | true |
| Z Spring At Marker | Frequency | float | 0 | 30 | -- | 15 | 3 | -- | -- | true |
| Z Spring At Marker | Decay | float | 0 | 25 | -- | 15 | 3 | -- | -- | true |
| Z Spring At Marker | Wander Amount | float | 0 | 500 | -- | -- | 50 | -- | -- | true |
| Z Spring At Marker | Rotational Amplitude | float | -360 | 360 | -- | -- | 50 | -- | -- | true |
| Z Spring At Marker | Rotational Frequency | float | 0 | 30 | -- | 15 | 1.5 | -- | -- | true |
| Zoom - 2D spin | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Zoom - 3D tumble | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Zoom - bubble | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Zoom - spiral | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |
| Zoom - spiral | Spiral Size (pixels) | float | 0 | 2000 | -- | -- | 200 | -- | -- | true |
| Zoom - wobble | Transition Completion | float | 0 | 100 | -- | -- | 50 | percent | -- | true |

**[STU-FX-133b] Preset-defined pseudo-effects.** 107 entries in the editing application and 106 in
the compositing application are not plug-in binaries but effect identities defined by a shipped
preset: a named parameter surface over an underlying effect graph. Studio models these as
`StudioEffectStack` presets in the `StudioStyleRegistry` ([STU-FX-039]) that expose a curated
parameter subset, NOT as new `filter_kind` values, because doing otherwise would create a second
effect-identity space. Their exposed parameters are ordinary `StudioEffectParameter` records and
carry the full contract.


**Presets (pseudo-effect backing shipped animation presets)** (107)

*Derivation: catalogue table, splits per row; yields 107 microtasks, one per preset-defined pseudo-effect.*

**Preset-defined pseudo-effects new to the editing application** (1 row)

*Derivation: catalogue table, splits per row; yields 1 microtasks, one per new preset-defined pseudo-effect.*

| Studio effect | Engine | Params | Presets | Description (from capture) | Import key (provenance) |
|---|---|---|---|---|---|
| Face Track Points | ae_native | 151 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Animal Head66` |

**Preset-defined pseudo-effects that dedup onto 14.9.3** (106 rows)

*Derivation: catalogue table listing rows that dedup onto an existing filter_kind under [STU-FX-127]; yields no microtask of its own.*

| Studio effect | Engine | Params | Presets | Description (from capture) | Import key (provenance) |
|---|---|---|---|---|---|
| 2D Text Box | ae_native | 27 | 0 | _no vendor description recovered_ | `Pseudo/ADBE 2D Text Box` |
| Animated Shape Control | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE CM Animated Shape Control` |
| Animated Shape Control | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE CM Animated Shape 3` |
| Autoscroll - horizontal | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM AutoscrollHorizontal` |
| Autoscroll - vertical | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM AutoscrollVertical` |
| Bounce | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Bounce` |
| Bounce At Marker | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE DE Bounce At Marker` |
| Bounce On Beat | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Bounce On Beat` |
| Bounce Random | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Bounce Random` |
| Card Wipe Master Control | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM TransCard` |
| Chaser Control | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE CM Animated Shape 2` |
| Color Swirl | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE Color Swirl` |
| Corner Reveal | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE CM CornerReveal` |
| Counter Controls | ae_native | 12 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Counter Controls` |
| Cracked Tiles | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM CrackedTiles` |
| Crop Edges | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM CropEdges` |
| Currency Controls | ae_native | 15 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Currency Controls` |
| Dissolve - unmelt | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM DissolveUnmelt` |
| Dissolve Master Control | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM TransDissolve` |
| Drift Over Time | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM Throw` |
| Face Measurements | ae_native | 24 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Animal Head14` |
| Fade In+Out - frames | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM FadeInOutFrames` |
| Fade In+Out - msec | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM FadeInOutmsec` |
| Fade Master Control | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM TransFade` |
| Fly to Inset | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE CM FlyToInset` |
| Follow | ae_native | 7 | 0 | _no vendor description recovered_ | `ADBE DE Follow` |
| Getting Jiggy | ae_native | 14 | 0 | _no vendor description recovered_ | `ADBE Getting Jiggy` |
| Grid Wipe | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE CM GridWipe` |
| Inset Video - framed | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM InsetVideoFramed` |
| Inset Video - torn edges | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM InsetVideoTorn` |
| Iris Wipe Master Controls | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM TransIris` |
| Jiggle | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Jiggle` |
| Jiggle At Marker | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE DE Jiggle At Marker` |
| Jiggle On Beat | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Jiggle On Beat` |
| Jiggle Random | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Jiggle Random` |
| Light Leaks - layer markers | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM LightLeaksMarkers` |
| Light Leaks - random | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM LightLeaksRandom` |
| Mask Fade Controls | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM TransFadeMask` |
| Mood Lighting - amorphous | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM MoodLightAmorph` |
| Mood Lighting - digital | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM MoodLightDigital` |
| Mood Lighting - streaks | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM MoodLightStreaks` |
| Opacity Flash - layer markers | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM OpacityFlashMarkers` |
| Opacity Flash - random | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM OpacityFlashRandom` |
| Opacity Pulse | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Opacity Pulse` |
| Opacity Pulse At Marker | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE DE Opacity Pulse At Marker` |
| Opacity Pulse On Beat | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Opacity Pulse On Beat` |
| Opacity Pulse Random | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Opacity Pulse Random` |
| Orbit | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE DE Orbit` |
| Orbit 3D | ae_native | 7 | 0 | _no vendor description recovered_ | `ADBE DE Orbit 3D` |
| Oscillate | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Oscillate` |
| Oscillate At Marker | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE DE Oscillate At Marker` |
| Oscillate On Beat | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Oscillate On Beat` |
| Oscillate Random | ae_native | 7 | 0 | _no vendor description recovered_ | `ADBE DE Oscillate Random` |
| Pattern Template | ae_native | 5 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Pattern Template` |
| Pendulum | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Pendulum` |
| Pendulum At Marker | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE DE Pendulum At Marker` |
| Pendulum On Beat | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Pendulum On Beat` |
| Pendulum Random | ae_native | 5 | 0 | _no vendor description recovered_ | `ADBE DE Pendulum Random` |
| Percentage Controls | ae_native | 13 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Percentage Controls` |
| Pulse | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE DE Pulse` |
| Pulse At Marker | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE DE Pulse At Marker` |
| Pulse On Beat | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE DE Pulse On Beat` |
| Pulse Random | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE DE Pulse Random` |
| Radial Wipe Master Controls | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE CM TransRadial` |
| Random Fill Color | ae_native | 8 | 0 | _no vendor description recovered_ | `ADBE DE Random Fill Color` |
| Random Motion | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Random Motion` |
| Random Motion 1D | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Random Motion 1D` |
| Random Opacity | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Random Opacity` |
| Random Rotation | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Random Rotation` |
| Random Rotation 3D | ae_native | 10 | 0 | _no vendor description recovered_ | `ADBE DE Random Rotation 3D` |
| Random Scale | ae_native | 7 | 0 | _no vendor description recovered_ | `ADBE DE Random Scale` |
| Rotate Over Time | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM Spin` |
| Sample Image | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE Sample Image` |
| Scale Bounce - layer markers | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM ScaleBounceMarkers` |
| Scale Bounce - random | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM ScaleBounceRandom` |
| Separate XYZ Position | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE Separate XYZ Position` |
| Separate XYZ Scale | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE Separate XYZ Scale` |
| Slide - variable | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM SlideVariable` |
| Slide Master Control | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM TransSlide` |
| Stereo 3D Controls | ae_native | 7 | 0 | _no vendor description recovered_ | `ADBE Stereo 3D Controls` |
| Stretch Master Control | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM TransStretch` |
| Stretch Master Control (edge) | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM TransDirection` |
| Stretch Master Control(corner) | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM TransCorner` |
| Swarm | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE DE Swarm` |
| Timer Controls | ae_native | 10 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Timer Controls` |
| Trace Path | ae_native | 2 | 0 | _no vendor description recovered_ | `Pseudo/ADBE Trace Path` |
| Transition Master Control | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM TransComplete` |
| Wiggle - gelatin | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM WiggleGelatin` |
| Wiggle - position | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM WigglePosition` |
| Wiggle - rotation | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM WiggleRotation` |
| Wiggle - scale | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE CM WiggleScale` |
| Wiggle - shear | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM WiggleShear` |
| Wigglerama | ae_native | 7 | 0 | _no vendor description recovered_ | `ADBE CM Wigglerama` |
| Wipe Master Control | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM TransWipe` |
| Wipe Master Controls | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM TransWipeFeath` |
| Wobble Bounce | ae_native | 8 | 0 | _no vendor description recovered_ | `ADBE DE Wobble Bounce` |
| Wobble Bounce At Marker | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Wobble Bounce At Marker` |
| Wobble Bounce On Beat | ae_native | 8 | 0 | _no vendor description recovered_ | `ADBE DE Wobble Bounce On Beat` |
| Wobble Bounce Random | ae_native | 9 | 0 | _no vendor description recovered_ | `ADBE DE Wobble Bounce Random` |
| Z Spring | ae_native | 8 | 0 | _no vendor description recovered_ | `ADBE DE Z Spring` |
| Z Spring At Marker | ae_native | 6 | 0 | _no vendor description recovered_ | `ADBE DE Z Spring At Marker` |
| Zoom - 2D spin | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM Zoom2DSpin` |
| Zoom - 3D tumble | ae_native | 3 | 0 | _no vendor description recovered_ | `ADBE CM Zoom3DTumble` |
| Zoom - bubble | ae_native | 1 | 0 | _no vendor description recovered_ | `ADBE CM ZoomBubble` |
| Zoom - spiral | ae_native | 4 | 0 | _no vendor description recovered_ | `ADBE CM ZoomSpiral` |
| Zoom - wobble | ae_native | 2 | 0 | _no vendor description recovered_ | `ADBE CM ZoomWobble` |
---

## 14.9.7 The audio effect domain

This group is new scope. Sub-section 14.9 as it stood at v02.205 had no audio surface at all,
because [STU-OVR-015] placed video outside Studio and audio came with it. [STU-OVR-015] is
superseded ([STU-VID-001]), and a professional editing and VFX product without audio processing is
not a professional editing product.

**[STU-FX-134] Studio ships a native audio effect domain.** An audio effect is a
`StudioLiveFilter` whose `filter_kind` is registered in the `audio` category and whose target is an
audio-bearing `StudioLayer`, `StudioClip` ([STU-VID-020]) or bus ([STU-FX-137]). It obeys every
clause of 14.9.1 without exception: the same typed parameter record, the same hard/soft split, the
same enumerated-option model, the same determinism requirement. 138 audio processors are in scope,
of which 31 carry a fully declared parameter contract with real unit ranges and closed-form mapping
expressions.

**[STU-FX-135] Audio effects are non-destructive stack entries.** An audio processor applied to a
clip or a track is an ordered entry in that object's `StudioEffectStack`, re-editable, reorderable,
individually bypassable, and removable with exact restoration of the prior result, exactly as
clauses [STU-FX-001] through [STU-FX-003] require of image effects. There is no "render audio to
apply" step.

**[STU-FX-136] Audio parameters are `normalised_scalar` where the source declares a mapping.** Per [STU-FX-121]
the record stores the normalised input, the declared affine mapping and the derived
real value, and presents the real value in its declared unit. The complete contracts follow.

**[STU-FX-137] Audio routing is a first-class part of the document, not an effect property.**
Recovered from the sequence model: a sequence declares a master channel configuration, a per-track
channel type, submix tracks, sends, panner assignments, per-track volume, pan, mute, solo, lock and
sync-lock, and a keyframe mode. Studio's normative audio routing model is:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Concept | Contract |
|---|---|
| Master configuration | One per sequence. Enumeration: `mono`, `stereo`, `5.1`, `multichannel_adaptive`, `sixteen_channel`. Of 392 shipped sequence configurations read, 373 declare `stereo` and 19 declare `multichannel_adaptive`; the other three values are declared by the model and unused by the shipped set. |
| Channel layout presets | Three shipped: Mono (1 channel), Stereo (2 channels: labels 100, 101), 5.1 (6 channels). Each carries an ordered channel-label list; the layout is data, and a custom layout is an ordinary instance of the same record. |
| Track | Carries `channel_type`, `volume` (default 1.0, linear gain), `pan` (default 0), `mute`, `solo`, `locked`, `sync_locked`, `targeted`, `is_submix`, `keyframe_mode`, an ordered send list and an ordered panner-assignment list. |
| Send | A named routing from a track to a submix with its own level. |
| Sample rate | Declared per sequence. 386 of 392 shipped configurations declare 48000 Hz; 6 declare 32000 Hz. Studio's default is 48000 Hz. |
| Audio time display | Audio time is displayed in samples in all 392 shipped configurations; `milliseconds` is the other declared display mode. Video and audio time displays are independent ([STU-VID-012]). |

**[STU-FX-138] The guided audio-adjustment surface.** A declarative, mode-driven adjustment layer
sits above the raw processors: an audio clip is assigned a content type, and the type selects which
adjustment model groups are offered. This is a UI-layer contract over ordinary effects, not a
separate engine. The normative mode set is `generic`, `dialogue`, `music`, `sfx`, `ambience`, and
the model groups are `loudness`, `restoration`, `clarity`, `soundeffects`, `volume`, `duration`,
`ducking`, `pan`, `stereowidth`. The mode-to-group bindings recovered are:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Mode | Model groups (shared definition) | Model groups (editing-application definition) |
|---|---|---|
| `generic` (hidden by default) | volume | volume |
| `dialogue` | loudness, restoration, clarity, soundeffects, volume | _absent from the editing-application file in this install_ |
| `music` | loudness, duration, volume | loudness, duration, ducking, volume |
| `sfx` | loudness, soundeffects, pan, volume | loudness, soundeffects, pan, volume |
| `ambience` | loudness, soundeffects, stereowidth, volume | loudness, soundeffects, stereowidth, ducking, volume |

**[STU-FX-138a]** Two facts about that table are declared rather than resolved. The two shipped
definition files disagree: the shared file declares five modes and the editing-application file
declares four, omitting dialogue, and the editing-application file adds `ducking` to music and
ambience. Studio adopts the UNION -- five modes, with `ducking` available on `music` and `ambience`
-- and records the disagreement here so the choice is visible. The loudness model's shipped default
configuration declares `DefaultLoudness = -21`, `MinLoudness = -42`, `MaxLoudness = -12` (all
decibels, LUFS-referenced) and a `LoudnessStandard` enumeration whose members were not recovered;
that enumeration is declared gap [STU-FX-147].

### Audio processor parameter contracts

**[STU-FX-136a]** Each table below is one processor's complete declared contract: the normalised input
slot, the real-world hard range, the derived real default, the normalised default, and the exact
mapping expression. These are reproduced verbatim because a mapping expression cannot be inferred
from a range.

**[STU-FX-136b] The audio processor sheets declare no control range, no unit token and no decimal
count, and those three fields are carried as absent rather than as copies.** `soft_min`, `soft_max`,
`unit` and `precision` are `--` on every row of every table in this group, with `--` reading exactly
as [STU-FX-131a] defines it. The sheets declare a normalised input domain and an affine mapping into
a real-world range; they do not declare a second, narrower range for the control, so setting
`soft_min`/`soft_max` equal to `hard_min`/`hard_max` would assert something the source never said
and would be unrecoverable ([STU-FX-105], [STU-FX-106]). The real-world unit is legible from several
parameter names -- a spectral decay is a time, a threshold on a dynamics processor is a level -- but
legible is not declared, so the unit is authored deliberately under [STU-FX-107] and recorded as
authored. `default` carries the derived real value and `default (normalised)` carries the stored
normalised form; they are two representations of ONE default, per [STU-FX-121], and neither may be
dropped, because the mapping is the only thing that relates them.


**Adaptive Noise Reduction** -- sheet `AdaptiveNoiseReductionUI`, 9 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| inf_reduce_noise_by | in0 | 0 | 40 | -- | -- | 20 | 0.5 | -- | -- | `value = normalised * (reduce_noise_max - reduce_noise_min) + reduce_noise_min` |
| inf_noisiness | in1 | 0 | 100 | -- | -- | 30 | 0.3 | -- | -- | `value = normalised * (noisiness_max - noisiness_min) + noisiness_min` |
| inf_fine_tune_noise_floor | in2 | -10 | 10 | -- | -- | 2 | 0.6 | -- | -- | `value = normalised * (fine_tune_noise_floor_max - fine_tune_noise_floor_min) + fine_tune_noise_floor_min` |
| inf_signal_threshold | in3 | -20 | 20 | -- | -- | 2.5 | 0.5625 | -- | -- | `value = normalised * (signal_threshold_max - signal_threshold_min) + signal_threshold_min` |
| inf_spectral_decay | in4 | 20 | 750 | -- | -- | 140.00032 | 0.164384 | -- | -- | `value = normalised * (spectral_decay_max - spectral_decay_min) + spectral_decay_min` |
| inf_broadband_preservation | in5 | 0 | 500 | -- | -- | 100 | 0.2 | -- | -- | `value = normalised * (broadband_preservation_max - broadband_preservation_min) + broadband_preservation_min` |

**AutoClickRemover** -- sheet `AutoClickRemoverUI`, 3 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| inf_threshold | in0 | 1 | 100 | -- | -- | 29.999971 | 0.292929 | -- | -- | `value = normalised * (threshold_max - threshold_min) + threshold_min` |
| inf_complexity | in1 | 1 | 100 | -- | -- | 15.999985 | 0.151515 | -- | -- | `value = normalised * (complexity_max - complexity_min) + complexity_min` |

**Chorus/Flanger** -- sheet `ChorusFlanger`, 5 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| width | in1 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| intensity | in2 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| transience | in3 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |

**Convolution Reverb** -- sheet `ConvolutionReverbUI`, 9 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| mix | in0 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| pre_delay | in4 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |

**Dynamics** -- sheet `DynamicsUI`, 34 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| attack_time | in0 | 0 | 500 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (attack_max - attack_min) + attack_min` |
| release_time | in1 | 0 | 2000 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (release_max - release_min) + release_min` |
| envelope_attack_time | in2 | 0 | 500 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (attack_max - attack_min) + attack_min` |
| envelope_release_time | in3 | 0 | 2000 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (release_max - release_min) + release_min` |
| look_ahead_time | in4 | 0 | 500 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (attack_max - attack_min) + attack_min` |
| input_gain | in5 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (decibel_max - decibel_min) + decibel_min` |
| output_gain | in6 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (decibel_max - decibel_min) + decibel_min` |
| low_frequency | in7 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| high_frequency | in8 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| UNUSED1 | in9 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| link_channels | in11 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| use_rms | in12 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| UNUNSED2 | in13 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised` |
| use_splines | in14 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| use_noise_gate | in15 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| makeup_gain | in31 | 0 | 100 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (makeup_max - makeup_min) + makeup_min` |

**Flanger** -- sheet `FlangerUI`, 10 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| initial | in0 | 0 | 20 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (delay_max - delay_min) + delay_min` |
| final | in1 | 0 | 20 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (delay_max - delay_min) + delay_min` |
| phase | in2 | 0 | 360 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (phase_max - phase_min) + phase_min` |
| cycles | in3 | 0.001 | 60 | -- | -- | 0.001 | 0 | -- | -- | `value = normalised * (cycles_max - cycles_min) + cycles_min` |
| mix | in4 | 0 | 100 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (mix_max - mix_min) + mix_min` |
| feedback | in5 | 0 | 100 | -- | -- | 0 | 0 | -- | -- | `value = normalised * (feedback_max - feedback_min) + feedback_min` |
| beats | in9 | 0.03333 | 2000 | -- | -- | 0.03333 | 0 | -- | -- | `value = normalised * (beats_max - beats_min) + beats_min` |

**GraphicEQ10Bands** -- sheet `GraphicEQ__Bands`, 14 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| accuracy | in0 | 10 | 8192 | -- | -- | 999.997454 | 0.120997 | -- | -- | `value = normalised * (accuracy_max - accuracy_min) + accuracy_min` |
| gain | in1 | -64 | 64 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (gain_max - gain_min) + gain_min` |
| range | in2 | 1.5 | 120 | -- | -- | 47.808615 | 0.39079 | -- | -- | `value = normalised * (range_max - range_min) + range_min` |
| amp1 | in4 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp2 | in5 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp3 | in6 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp4 | in7 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp5 | in8 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp6 | in9 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp7 | in10 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp8 | in11 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp9 | in12 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp10 | in13 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |

**GraphicEQ20Bands** -- sheet `GraphicEQ__Bands`, 24 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| accuracy | in0 | 10 | 8192 | -- | -- | 999.997454 | 0.120997 | -- | -- | `value = normalised * (accuracy_max - accuracy_min) + accuracy_min` |
| gain | in1 | -64 | 64 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (gain_max - gain_min) + gain_min` |
| range | in2 | 1.5 | 120 | -- | -- | 47.808615 | 0.39079 | -- | -- | `value = normalised * (range_max - range_min) + range_min` |
| amp1 | in4 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp2 | in5 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp3 | in6 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp4 | in7 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp5 | in8 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp6 | in9 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp7 | in10 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp8 | in11 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp9 | in12 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp10 | in13 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp11 | in14 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp12 | in15 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp13 | in16 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp14 | in17 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp15 | in18 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp16 | in19 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp17 | in20 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp18 | in21 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp19 | in22 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp20 | in23 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |

**GraphicEQ30Bands** -- sheet `GraphicEQ__Bands`, 34 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| accuracy | in0 | 10 | 8192 | -- | -- | 999.997454 | 0.120997 | -- | -- | `value = normalised * (accuracy_max - accuracy_min) + accuracy_min` |
| gain | in1 | -64 | 64 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (gain_max - gain_min) + gain_min` |
| range | in2 | 1.5 | 120 | -- | -- | 47.808615 | 0.39079 | -- | -- | `value = normalised * (range_max - range_min) + range_min` |
| amp1 | in4 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp2 | in5 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp3 | in6 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp4 | in7 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp5 | in8 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp6 | in9 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp7 | in10 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp8 | in11 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp9 | in12 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp10 | in13 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp11 | in14 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp12 | in15 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp13 | in16 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp14 | in17 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp15 | in18 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp16 | in19 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp17 | in20 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp18 | in21 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp19 | in22 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp20 | in23 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp21 | in24 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp22 | in25 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp23 | in26 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp24 | in27 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp25 | in28 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp26 | in29 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp27 | in30 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp28 | in31 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp29 | in32 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| amp30 | in33 | -70 | 70 | -- | -- | 0 | 0.5 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |

**GuitarSuite** -- sheet `GuitarSuiteUI`, 13 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| compressor_amount | in0 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| filter_resonance | in2 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| distortion_amount | in3 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| mix_amount | in4 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |

**Hard Limiter** -- sheet `HardLimiterUI`, 7 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| max_amp | in0 | -100 | 0 | -- | -- | -50 | 0.5 | -- | -- | `value = normalised * (max_amp_max - max_amp_min) + max_amp_min` |
| input_boost | in1 | -100 | 50 | -- | -- | 20 | 0.8 | -- | -- | `value = normalised * (input_boost_max - input_boost_min) + input_boost_min` |
| lookahead_time | in2 | 5 | 20 | -- | -- | 7.1 | 0.14 | -- | -- | `value = normalised * (lookahead_time_max - lookahead_time_min) + lookahead_time_min` |
| release_time | in3 | 40 | 200 | -- | -- | 100 | 0.375 | -- | -- | `value = normalised * (release_time_max - release_time_min) + release_time_min` |

**Parametric Equalizer** -- sheet `ParametricEQ`, 45 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| low_filter_range | in44 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| lpcutoff | in0 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| lpamp | in1 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| hpcutoff | in2 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| hpamp | in3 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| center1 | in4 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| amp1 | in6 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| enable1 | in7 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| center2 | in8 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| amp2 | in10 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| enable2 | in11 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| center3 | in12 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| amp3 | in14 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| enable3 | in15 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| center4 | in16 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| amp4 | in18 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| enable4 | in19 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| center5 | in20 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| amp5 | in22 | -48 | 48 | -- | -- | -48 | 0 | -- | -- | `value = normalised * (amp_max - amp_min) + amp_min` |
| enable5 | in23 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| gain | in24 | -96 | 48 | -- | -- | -96 | 0 | -- | -- | `value = normalised * (gain_max - gain_min) + gain_min` |
| constant_q | in25 | false | true | -- | -- | 1 | 1 | -- | -- | `value = normalised > 0.5` |
| low2ndorder | in26 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| high2ndorder | in27 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| ultraquiet | in28 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| lpenable | in29 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| hpenable | in30 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| q1 | in5 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q2 | in9 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q3 | in13 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q4 | in17 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q5 | in21 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q_width1 | in32 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q_width2 | in33 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q_width3 | in34 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q_width4 | in35 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| q_width5 | in36 | 0.0001 | 10000 | -- | -- | 0.0001 | 0 | -- | -- | `value = normalised * (q_max - q_min) + q_min` |
| hipassenable | in37 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| hipassfreq | in38 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |
| lopassenable | in40 | false | true | -- | -- | 0 | 0 | -- | -- | `value = normalised > 0.5` |
| lopassfreq | in41 | 20 | sample_rate / 2 | -- | -- | -- | 0 | -- | -- | `value = normalised * (filter_max - filter_min) + filter_min` |

**Phaser** -- sheet `PhaserUI`, 9 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| mix | in1 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| intensity | in5 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| depth | in6 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |

**Studio Reverb** -- sheet `StudioReverbUI`, 12 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| lowcut | in0 | 20 | 4000 | -- | -- | 879.9984 | 0.21608 | -- | -- | `value = normalised * (lowcut_max - lowcut_min) + lowcut_min` |
| highcut | in1 | 20 | 20000 | -- | -- | 13800.0062 | 0.68969 | -- | -- | `value = normalised * (highcut_max - highcut_min) + highcut_min` |
| roomsize | in2 | 1 | 100 | -- | -- | 70.00003 | 0.69697 | -- | -- | `value = normalised * (roomsize_max - roomsize_min) + roomsize_min` |
| width | in3 | 0 | 100 | -- | -- | 25 | 0.25 | -- | -- | `value = normalised * (width_max - width_min) + width_min` |
| diffusion | in4 | 0 | 100 | -- | -- | 50 | 0.5 | -- | -- | `value = normalised * (diffusion_max - diffusion_min) + diffusion_min` |
| damping | in5 | 0 | 100 | -- | -- | 50 | 0.5 | -- | -- | `value = normalised * (damping_max - damping_min) + damping_min` |
| decay | in6 | 200 | 10000 | -- | -- | 2500.0012 | 0.234694 | -- | -- | `value = normalised * (decay_max - decay_min) + decay_min` |
| earlyreflections | in7 | 0 | 100 | -- | -- | 52 | 0.52 | -- | -- | `value = normalised * (earlyreflections_max - earlyreflections_min) + earlyreflections_min` |
| dry | in9 | 0 | 100 | -- | -- | 75 | 0.75 | -- | -- | `value = normalised * (dry_max - dry_min) + dry_min` |
| wet | in10 | 0 | 100 | -- | -- | 25 | 0.25 | -- | -- | `value = normalised * (wet_max - wet_min) + wet_min` |

**Surround Reverb** -- sheet `SurroundReverbUI`, 14 normalised input slots

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | Slot | hard_min | hard_max | soft_min | soft_max | default | default (normalised) | unit | precision | Normalised-to-value mapping |
|---|---|---|---|---|---|---|---|---|---|---|
| center_input_gain | in0 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| lfe_input_gain | in1 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| pre_delay | in5 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| center_wet_level | in8 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
| mix | in11 | -- | -- | -- | -- | -- | 0 | -- | -- | `value = normalised * 100` |
### The audio processor catalogue

**[STU-FX-134a]** The complete audio processor set. Rows without a contract table above are specified
in identity and behaviour but not yet in parameters, per [STU-FX-128].


**(uncategorised)** (138)

*Derivation: catalogue table, splits per row; yields 138 microtasks, one per audio processor.*

| Studio effect | Engine | Params | Presets | Description (from capture) | Import key (provenance) |
|---|---|---|---|---|---|
| Adaptive Noise Reduction | dva_audio | 6 | 0 | _no vendor description recovered_ | `AudioFilter:AdaptiveNoiseReduction` |
| AddSchemaPropertyDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:AddSchemaPropertyDialog.adam` |
| Render Settings Dialog Sheet | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:AERenderSettingsDialog.adam` |
| Amplify | dva_audio | 4 | 0 | _no vendor description recovered_ | `AudioFilter:Amplify` |
| Analog Delay | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:AnalogDelay` |
| ArriRawLogC4MXFSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ArriRawLogC4MXFSetup` |
| ArriRawLogC4MXFSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ArriRawLogC4MXFSetup.adam` |
| ArriRawMXFSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ArriRawMXFSetup` |
| ArriRawMXFSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ArriRawMXFSetup.adam` |
| ARRIRAWSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ARRIRAWSetup` |
| ARRIRAWSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ARRIRAWSetup.adam` |
| AutoClickRemover | dva_audio | 2 | 0 | _no vendor description recovered_ | `AudioFilter:AutoClickRemover` |
| BarsAndToneSettingsWithIdentifiersDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:BarsAndToneSettingsWithIdentifiersDialog.adam` |
| BasicTextOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:BasicTextOptions.adam` |
| ButtonUpgradeTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ButtonUpgradeTest.adam` |
| CanonRawSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:CanonRawSetup` |
| CanonRawSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:CanonRawSetup.adam` |
| Channel Mixer | dva_audio | 2 | 0 | _no vendor description recovered_ | `AudioFilter:ChannelMixer` |
| ChannelMixerUI | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ChannelMixerUI` |
| Chorus/Flanger | dva_audio | 3 | 0 | _no vendor description recovered_ | `AudioFilter:ChorusFlanger` |
| Classic3DSettings.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:Classic3DSettings.adam` |
| CompressorBand | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:CompressorBand` |
| Convolution Reverb | dva_audio | 2 | 0 | _no vendor description recovered_ | `AudioFilter:ConvolutionReverb` |
| DeEsser | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:DeEsser` |
| DeHummer | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:DeHummer` |
| DeNoiseDeReverb | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:DeNoiseDeReverb` |
| DialogTypekit.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:DialogTypekit.adam` |
| DialogTypekitWarning.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:DialogTypekitWarning.adam` |
| Distortion | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:Distortion` |
| Dry/Wet Mixer | dva_audio | 1 | 0 | _no vendor description recovered_ | `AudioFilter:DryWetMixer` |
| Dynamics | dva_audio | 38 | 0 | _no vendor description recovered_ | `AudioFilter:Dynamics` |
| EAConvertToProductionTabGeneral.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAConvertToProductionTabGeneral.adam` |
| EAHistorySyncDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAHistorySyncDialog.adam` |
| EAHistorySyncTabDetails.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAHistorySyncTabDetails.adam` |
| EAHostedLoginNotEntitledDialog | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAHostedLoginNotEntitledDialog` |
| EAInviteUserManagementDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAInviteUserManagementDialog.adam` |
| EAManageProductionsDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAManageProductionsDialog.adam` |
| EAManageProductionsTab_ArchivedProductions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAManageProductionsTab_ArchivedProductions.adam` |
| EAManageProductionsTab_Invites.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAManageProductionsTab_Invites.adam` |
| EAManageProductionsTab_Productions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAManageProductionsTab_Productions.adam` |
| EAMediaMappingsDialog | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAMediaMappingsDialog` |
| EAMediaMappingsDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAMediaMappingsDialog.adam` |
| EAMediaVolumesDialog | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAMediaVolumesDialog` |
| EAMediaVolumesDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAMediaVolumesDialog.adam` |
| EANewProductionDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionDialog.adam` |
| EANewProductionDialogWithSharedLocation.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionDialogWithSharedLocation.adam` |
| EANewProductionTabColor.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionTabColor.adam` |
| EANewProductionTabGeneral.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionTabGeneral.adam` |
| EANewProductionTabGeneralWithSharedLocation | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionTabGeneralWithSharedLocation` |
| EANewProductionTabIngestSettings.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionTabIngestSettings.adam` |
| New Production Tab -- Collaboration Options | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionTabPremiereCollabOptions.adam` |
| New Production Tab -- Collaboration Scratch Disks | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionTabPremiereCollabScratchDisks.adam` |
| New Production Tab -- Options | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EANewProductionTabPremiereOptions` |
| EAProgressDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAProgressDialog.adam` |
| EAPublishDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAPublishDialog.adam` |
| EAReflectionProgressDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAReflectionProgressDialog.adam` |
| EAResolveDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EAResolveDialog.adam` |
| EASaveAsDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EASaveAsDialog.adam` |
| EATakeOverEditDialog | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EATakeOverEditDialog` |
| EditMediaBrowserColumnsDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:EditMediaBrowserColumnsDialog.adam` |
| eveReproTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:eveReproTest.adam` |
| eveUpgradeTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:eveUpgradeTest.adam` |
| F65Setup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:F65Setup` |
| F65Setup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:F65Setup.adam` |
| FFT Filter | dva_audio | 16 | 0 | _no vendor description recovered_ | `AudioFilter:FFTFilter` |
| Flanger | dva_audio | 7 | 0 | _no vendor description recovered_ | `AudioFilter:Flanger` |
| FWDeviceControlTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:FWDeviceControlTest.adam` |
| GraphicEQ10Bands | dva_audio | 13 | 0 | _no vendor description recovered_ | `AudioFilter:GraphicEQ10Bands` |
| GraphicEQ20Bands | dva_audio | 23 | 0 | _no vendor description recovered_ | `AudioFilter:GraphicEQ20Bands` |
| GraphicEQ30Bands | dva_audio | 33 | 0 | _no vendor description recovered_ | `AudioFilter:GraphicEQ30Bands` |
| GuitarSuite | dva_audio | 4 | 0 | _no vendor description recovered_ | `AudioFilter:GuitarSuite` |
| Hard Limiter | dva_audio | 11 | 0 | _no vendor description recovered_ | `AudioFilter:HardLimiter` |
| ImporterDPXPrefs | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ImporterDPXPrefs` |
| ImporterDPXPrefs.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ImporterDPXPrefs.adam` |
| ImporterFrameGenPrefs | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ImporterFrameGenPrefs` |
| ImporterFrameGenPrefs.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ImporterFrameGenPrefs.adam` |
| JpegOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:JpegOptions.adam` |
| Latency Generator | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LatencyGenerator` |
| LayoutUpgradeTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LayoutUpgradeTest.adam` |
| LeaderSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LeaderSetup` |
| LeaderSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LeaderSetup.adam` |
| LocalDialogsTestView | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LocalDialogsTestView` |
| Loudness Meter | dva_audio | 12 | 0 | _no vendor description recovered_ | `AudioFilter:LoudnessMeter` |
| LoudnessMeterProxy | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LoudnessMeterProxy` |
| Color Grade Setup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LumetriSetup` |
| Color Grade Setup Sheet | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:LumetriSetup.adam` |
| Mastering | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:Mastering` |
| MetadataPreferencesDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:MetadataPreferencesDialog.adam` |
| MP4Setup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:MP4Setup` |
| MP4Setup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:MP4Setup.adam` |
| MPEGSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:MPEGSetup` |
| MPEGSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:MPEGSetup.adam` |
| Multiband Compressor | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:MultiBandCompressor` |
| MultiChannelAudioFilterBase | dva_audio | 1 | 0 | _no vendor description recovered_ | `AudioFilter:MultiChannelAudioFilterBase` |
| Notch | dva_audio | 28 | 0 | _no vendor description recovered_ | `AudioFilter:Notch` |
| NumbersOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:NumbersOptions.adam` |
| Parametric Equalizer | dva_audio | 91 | 0 | _no vendor description recovered_ | `AudioFilter:ParametricEQ` |
| ParametricDynamics | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ParametricDynamics` |
| ParametricEQ.V7 | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ParametricEQ.V7` |
| ParticlePlaygroundCannonOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ParticlePlaygroundCannonOptions.adam` |
| ParticlePlaygroundGridOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ParticlePlaygroundGridOptions.adam` |
| ParticlePlaygroundOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ParticlePlaygroundOptions.adam` |
| PathTextOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:PathTextOptions.adam` |
| Phaser | dva_audio | 3 | 0 | _no vendor description recovered_ | `AudioFilter:Phaser` |
| png_dialog_hdr10_script.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:png_dialog_hdr10_script.adam` |
| PopupUpgradeTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:PopupUpgradeTest.adam` |
| PrefsEveTestDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:PrefsEveTestDialog.adam` |
| ProjectSettingsTestTab | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ProjectSettingsTestTab` |
| ProjectSettingsTestTab.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ProjectSettingsTestTab.adam` |
| ProResRawSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ProResRawSetup` |
| ProResRawSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ProResRawSetup.adam` |
| ProResRawSetupDlg | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ProResRawSetupDlg` |
| ProResRawSetupDlg.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:ProResRawSetupDlg.adam` |
| REDImporterSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:REDImporterSetup` |
| REDImporterSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:REDImporterSetup.adam` |
| REDImporterSetupLarge | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:REDImporterSetupLarge` |
| REDImporterSetupLarge.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:REDImporterSetupLarge.adam` |
| REDImporterSetupWhiteBalance | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:REDImporterSetupWhiteBalance` |
| REDImporterSetupWhiteBalance.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:REDImporterSetupWhiteBalance.adam` |
| SetupDialog | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:SetupDialog` |
| Single-band Compressor | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:SingleBandCompressor` |
| Single-band Dynamics | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:SingleBandDynamics` |
| SonyRawMXFSetup | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:SonyRawMXFSetup` |
| SonyRawMXFSetup.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:SonyRawMXFSetup.adam` |
| Spectrum Meter | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:SpectrumMeter` |
| Stream Mixer | dva_audio | 1 | 0 | _no vendor description recovered_ | `AudioFilter:StreamMixer` |
| Studio Reverb | dva_audio | 23 | 0 | _no vendor description recovered_ | `AudioFilter:StudioReverb` |
| Surround Reverb | dva_audio | 5 | 0 | _no vendor description recovered_ | `AudioFilter:SurroundReverb` |
| tabUpgradeTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:tabUpgradeTest.adam` |
| TestPanel1.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:TestPanel1.adam` |
| TestPanel2.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:TestPanel2.adam` |
| TestPanel3.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:TestPanel3.adam` |
| TextEditBaselineTabbedTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:TextEditBaselineTabbedTest.adam` |
| TextEditUpgradeTest.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:TextEditUpgradeTest.adam` |
| TgaOptions.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:TgaOptions.adam` |
| Tube-modeled Compressor | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:TubeModeledCompressor` |
| VisualShortcutsDialog.adam | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:VisualShortcutsDialog.adam` |
| Vocal Enhancer | dva_audio | 0 | 0 | _no vendor description recovered_ | `AudioFilter:VocalEnhancer` |
---

## 14.9.8 Effect presets, styles and portability

[STU-FX-142] **Shipped effect presets are data, and their contract is what this specification
states -- not their names.** 621 preset files were read from the compositing application and 379
from the editing application. They resolve into two shapes, and Studio's `StudioStyleRegistry`
([STU-FX-039]) MUST represent both:

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Preset shape | Count | Contract |
|---|---|---|
| Effect preset | 293 | An ordered `StudioEffectStack` fragment: one or more effect instances with their parameter values, applied to the target as new stack entries. |
| Property preset | 325 | A set of values, keyframes and expressions bound to named property paths on the target object -- not effects at all. Applying one writes into the target's property tree ([STU-MOT-010]). |
| Empty or unrecognised | 3 | Recorded so the count reconciles; not shipped. |

**[STU-FX-142a]** Across those presets: 1,048 effect instances, **79,014 value streams**, 3,217
keyframes and 42 expressions. The value-stream count is the load-bearing number: a preset is not a
handful of slider positions, it is a dense parameter graph, and a Studio preset format that cannot
carry per-property keyframes and expressions alongside static values cannot represent the shipped
material.

**[STU-FX-143] A preset carries a category path, not a flat name.** 36 category paths were
recovered, several of them nested two deep. Studio's registry stores the path, so a browsable
hierarchy is data rather than UI structure.

**[STU-FX-144] Effect styles and presets are portable data.** [STU-FX-039a] already requires this;
this sub-section adds the specific obligation that a preset's referenced assets -- displacement
maps, custom convolution kernels, LUT payloads, material graphs, impulse responses for convolution
reverb, texture images -- are content-addressed artifact references resolved through the configured
artifact tier, never absolute paths, so relocating a project or moving it to another disk carries
the effect library intact ([GLOBAL-PORTABILITY] posture as expressed by [STU-FX-039a]).

---

## 14.9.9 Declared gaps

These are the things this sub-section does NOT specify. They are stated so an implementer knows the
boundary of the contract rather than discovering it, and so each becomes a tracked decision rather
than an invented value.

**[STU-FX-145] GAP -- soft bounds for the video-editing effect surface.** 93.5 percent of that
surface's 9,654 parameter rows declare no bound of any kind. Studio implements them
`unbounded_in_source` per [STU-FX-106a]. Choosing usable soft ranges for those controls is a
per-effect design task that MUST produce `bound_state = declared_soft_only` records authored
deliberately, and MUST NOT be back-filled into `hard_min`/`hard_max`.

**[STU-FX-146] GAP -- parameter records for 274 of the 482 installed compositing effects.** 208
have typed records; the rest are specified in identity, category and GPU status only. Each remaining
effect requires either a recovered record or a deliberately authored one before it can be
implemented. A microtask that implements one of these effects MUST carry its parameter authoring as
explicit scope.

**[STU-FX-147] GAP -- the loudness-standard enumeration.** The guided audio surface declares a
`LoudnessStandard` integer whose members were not recovered. Studio MUST NOT guess; the enumeration
is authored from the applicable broadcast loudness standards as a named decision.

**[STU-FX-148] GAP -- per-effect promotion-equivalence tolerances.** [STU-FX-037] requires a
tolerance per effect kind and a golden-image set; neither exists yet for any of the catalogue rows
here. The tolerances are authored in 14.24 and this sub-section requires only that every catalogue
row is covered.

**[STU-FX-149] OPEN DECISION -- the scrubbable control's coarse and fine modifier keys.** The
derivation of the increments is fixed by [STU-FX-110]; which modifier selects which increment is an
operator decision, recorded here so that the two are not conflated.

**[STU-FX-150] GAP -- vendor prose for 573 of 635 effect identities.** Per [STU-FX-129] only 62
carry a recovered description. Studio's own behaviour statements in the catalogues are normative;
the operator-facing manual prose for the remainder must be authored, and any tooltip-generation
contract must declare this coverage limit honestly rather than emitting a templated sentence.

---

## 14.9.10 Model steerability, GUI, diagnostics and manual obligation

**[STU-FX-151]** [STU-FX-040] applies unchanged to every clause and every catalogue row in this
sub-section: one primitive with two projections, full model visibility and typed steerability of
the effect stack and every parameter, parallel-safe and attributable authorship, the headless and
quiet law, Argus visual-diagnostic coverage of effect rendering and GPU-fallback state, and a
dual-audience UserManual entry. This sub-section adds three obligations specific to the parameter
contract:

1. The generated MCP `inputSchema` for an effect MUST expose `hard_min`, `hard_max`, `soft_min`,
   `soft_max`, `default`, `unit` and `precision` as separate properties, so a model can tell what it
   is allowed to set apart from what the slider shows.
2. The Argus diagnostic for a parameter MUST report its current value, its four bounds, its
   `bound_state` and whether the value currently lies outside the soft range, so a visual review
   can distinguish "control is showing an extended range" from "control is broken".
3. A UserManual entry for an effect MUST state hard and soft ranges separately wherever they
   differ, because an operator told only the slider range will believe legal values are illegal.

---

## 14.9.11 Microtask Derivation

**[STU-FX-160] Microtask derivation index.** Applying the shared derivation convention to this
sub-section yields exactly 1,042 microtasks. The correspondence is NORMATIVE and CLOSED: a microtask
corresponds to a yielding clause or to a table unit as marked, and to nothing else.

Rule 0 -- derivation markers are authoritative. Every table in this sub-section carries an italic
`*Derivation: ...*` marker sentence directly above it stating how many microtasks that table yields.
The marker is normative. A tool that classifies a table differently from its marker has diverged
from this sub-section and MUST be corrected to the marker, not the reverse. The five marker forms
are: parameter table taken whole (1); enumeration table taken whole (1); preset or command table
taken whole (1); catalogue table splitting per row (N); contract table carried into the clause's own
microtask (0). A sixth form, reading aid inside a non-yielding clause, also yields 0.

Rule A -- one microtask per yielding clause. Every numbered clause yields exactly one microtask
EXCEPT the members of the no-yield set of [STU-FX-160a]. A sub-lettered anchor
([STU-FX-105a], [STU-FX-131a], [STU-FX-136b]) is a clause for this purpose and yields on its own account.

Rule B -- table units, counted from the markers of rule 0. A parameter table is a unit in its own
right even though it sits inside a clause that is also a unit, because its rows are bound-sets that
have to be individually proven; folding it into its clause loses that proof obligation. This is the
largest single population in this sub-section and it is deliberate: an effect's catalogue row is the
obligation to implement that effect's identity, category, GPU posture and behaviour, and its
parameter table is the separate obligation to honour every one of its bound-sets, units, precisions
and enumerated option lists exactly. An enumeration table is a unit for the same reason, its members
being the criteria. A catalogue table splits because each row names a separately implementable
subject -- one Studio effect, one pseudo-effect, one audio processor. A contract table does not split
and is not its own unit: it describes the fields of the single contract its clause already defines.

**[STU-FX-160a] The no-yield set: 10 clauses.** Nothing else may be excluded, and a clause not on
this list yields under rule A whether or not it is convenient.
In this list a MEMBER of the set is written in backticks, as `STU-AREA-nnn`, and an anchor written
in brackets, as [STU-AREA-nnn], is a REFERENCE and is not excluded from anything. The two forms
are visually distinct so that a reader and a tool can both count the members without parsing the
surrounding English.

The members:

1. **Supersession and amendment.** `STU-FX-100` (replacement scope) and `STU-FX-101`, whose
   prior-clause disposition table is a supersession record, not work.
2. **Authority.** `STU-FX-102` (no sidecar authority).
3. **This derivation section.** `STU-FX-160`, `STU-FX-160a`, `STU-FX-161`, `STU-FX-162`,
   `STU-FX-162a`, `STU-FX-163` and `STU-FX-164`.

Clause [STU-FX-151] is NOT in the no-yield set even though its lead paragraph restates [STU-FX-040]: it
carries three obligations specific to the parameter contract -- the MCP `inputSchema` exposing all
seven fields separately, the Argus diagnostic reporting all four bounds and the `bound_state`, and
the UserManual stating hard and soft ranges separately -- and those are real, provable work that
exists nowhere else. Tables inside a non-yielding clause yield nothing.

**[STU-FX-161] Microtask content obligation.** A microtask derived under [STU-FX-160] MUST carry
into its own body: the clause anchor, or the catalogue row with its Studio name, category, GPU
status and import keys; the FULL parameter record of every parameter it touches with `hard_min`,
`hard_max`, `soft_min`, `soft_max`, `default`, `unit` and `precision` as SEVEN SEPARATE fields, with
`--` preserved on every side the source did not declare and never copied from its twin
([STU-FX-105], [STU-FX-107]); the verbatim enumerated option list with its 1-based indices for every
enumeration parameter ([STU-FX-116]); the mapping expression for every `normalised_scalar`
([STU-FX-121]); and the `bound_state` of every parameter ([STU-FX-106]). A microtask for a catalogue
row with NO typed parameter record MUST carry the parameter authoring as explicit scope and MUST say
that the record is absent rather than presenting an invented one ([STU-FX-128], [STU-FX-146]). No
microtask may cite the green-room corpus as its source of truth: the corpus is provenance for HOW a
clause was derived, and this sub-section is the authority ([STU-SECTION-002]).

**[STU-FX-162] Ledger.**

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Ledger line | Basis | Yields |
|---|---|---|
| Clauses in 14.9 | anchors 100 through 164, sub-lettered anchors included | 74 |
| less the no-yield set | the 10 clauses of [STU-FX-160a] | -10 |
| **Rule A subtotal** | one microtask per yielding clause | **64** |
| Parameter tables | 234 tables: 208 typed records of 14.9.4, 10 layer-style records of 14.9.5, 15 audio processor contracts of 14.9.7, and the bounded video-editing surface of 133a | 234 |
| Enumeration tables | 4 tables: `bound_state` of 106, `StudioParameterKind` of 112, the flag set of 113, the guided-adjustment modes of 138 | 4 |
| Preset tables | 1 table: the preset-shape inventory of 142, taken whole because its rows are counts, not subjects | 1 |
| Catalogue: compositing effect set of 130 | one per Studio `filter_kind`, across 28 category tables holding 445 rows; 18 rows are a second import key for a kind already named in the same table | 427 |
| Catalogue: new effect kinds from the editing application, 133 | one per new `filter_kind`; 179 rows carrying 173 distinct names | 173 |
| Catalogue: rows that dedup onto 14.9.3, 133 | 192 rows, each a second import key for a kind 14.9.3 already yielded ([STU-FX-127b]) | 0 |
| Catalogue: new preset-defined pseudo-effects, 133b | one per new pseudo-effect | 1 |
| Catalogue: pseudo-effects that dedup onto 14.9.3, 133b | 106 rows, the same preset identities seen from the second application | 0 |
| Catalogue: audio processor set of 134a | one per audio processor; the `audio` category shares no `filter_kind` with the image catalogues, so two coincidental name matches are NOT dedup | 138 |
| Contract tables | 2 tables carried into the owning clause's microtask: the parameter record of 103 and the routing model of 137 | 0 |
| Reading aids in non-yielding clauses | 2 tables: the disposition table of 101 and this ledger | 0 |
| **Rule B subtotal** | table units | **978** |
| **Total microtasks yielded by 14.9** | rule A plus rule B | **1042** |

**[STU-FX-162a] Catalogue arithmetic, stated because it is the largest number in the ledger and the
easiest to get wrong.** Every figure here is countable from the tables as they stand.

445 rows are listed in 14.9.3 against 482 effects that exist in the install ([STU-FX-128]); the
37-row difference is the registry entries whose binary was absent and which carry no catalogue row.
Those 445 rows carry 427 distinct Studio names: 18 rows are a second import key for a kind already
named in the same category table, which is [STU-FX-127] operating inside one catalogue, so 14.9.3
yields 427 and not 445.

616 rows are listed in 14.9.6 and 14.9.7 against the 617 entries recovered from the editing
application; the one absent row is the grading intrinsic owned by 14.8. They now sit in five tables
rather than three, because [STU-FX-127b] requires the dedup to be structural: 179 rows carrying 173
new names, 192 rows that dedup onto 14.9.3, 1 new preset-defined pseudo-effect, 106 pseudo-effects
that dedup, and 138 audio processors.

The catalogues therefore yield 427 + 173 + 1 + 138 = 739 microtasks, which is the number of distinct
Studio `filter_kind` values this sub-section names. A tool that reads any of these tables
structurally and ignores its marker will produce 445, 371 or 107 and be wrong; the markers state
427, 173 and 1, and the markers win ([STU-FX-160] rule 0).

**[STU-FX-163] An open item or a blocked dependency does NOT remove a microtask.** A clause or a
catalogue row that declares a gap, an open decision, an absent parameter record, an unrecovered
enumeration or an unrecovered description still yields its microtask, and that microtask's FIRST
acceptance row MUST read "the named gap is raised to the operator as a capture request and is NOT
closed by an invented value". The clauses carrying a declared gap or open decision
are [STU-FX-145], [STU-FX-146], [STU-FX-147], [STU-FX-148], [STU-FX-149] and [STU-FX-150]. The largest
population of blocked microtasks is not in that list, however: it is the 274 installed compositing
effects with no typed parameter record ([STU-FX-146]) and the 93.5 percent of the video-editing
parameter surface that declares no bound ([STU-FX-145]). Those catalogue microtasks are NOT
downgraded, deferred or merged, and their parameter authoring is scope, not a precondition to be
waved through.

**[STU-FX-164] Anchor binding.** A microtask derived from this sub-section cites the clause anchor
directly, and a catalogue microtask additionally cites its row's Studio effect name and its import
key. A microtask staged before this sub-section landed carries
`spec_anchor_status = "PROVISIONAL"`; binding it to an anchor or a catalogue row here clears that
status. A microtask that cannot cite either is out of scope for the effect domain and MUST be
re-derived or retired, not activated.
