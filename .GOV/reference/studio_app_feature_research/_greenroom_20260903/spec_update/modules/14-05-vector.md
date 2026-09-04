---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206-draft"
bundle_id: "master-spec-v02.206"
module_id: "14-05"
section_id: "14.5"
title: "14.5 Studio -- Vector Graphics & Illustration"
supersedes: "master-spec-v02.205 spec-modules/14-studio-creative-suite.md lines 490-895 (sub-section 14.5)"
derivation_basis: "green-room installed-application captures, 2026-09-03/04"
metadata_rule: "frontmatter is machine metadata; body follows after this block. body_sha256 and source_body_original_sha256 are assigned at bundle assembly per [CX-105D]."
---

# 14.5 Vector Graphics & Illustration

[REWRITE v02.206] This sub-section replaces the v02.205 text of 14.5 in full. The superseded text
was written from vendor help pages and scraped tables of contents. This text is written from the
parsed binaries of the installed applications: the vector suite's COM type library, its EVE dialog
layout source, its serialized live-effect libraries and its shipped preset libraries, and the
collaborative design suite's published object-model declarations. Every numeric bound, default,
unit, decimal-place count and enumerated value below is transcribed from one of those captures; none
is invented. Anchors `[STU-VEC-001]` through `[STU-VEC-073]` that still state a true requirement are
preserved here; where a captured behaviour contradicts one, the clause says so and the new anchor
supersedes the contradicted part.

Vector Graphics & Illustration is the Studio domain that owns editable resolution-independent
geometry: paths, vector networks, parametric shapes, boolean/geometry composition, fills and
strokes, the multi-attribute appearance model, brushes, transforms and distortions, and the
procedural constructs (repeat, blend, live paint, gradient mesh, image trace, intertwine, global
edit). It is the deduped normative union of the illustration surfaces of the source suites,
collapsed into one Studio primitive set per [STU-SECTION-003]. No source product name is a Studio
tool, command, panel, parameter or manual name.

Every capability in this sub-section operates on `StudioLayer` nodes whose `kind` is `vector` inside
the unified `StudioDocument` (14.3), sharing the same selection, history, color, effect, mask, and
export surfaces as every other Studio domain. Vector geometry is owned by two canonical primitives
-- `StudioVectorPath` (single ordered path) and `StudioVectorNetwork` (multi-edge topology) -- and
paint/appearance is owned by the shared `StudioGradient`, `StudioPattern`, `StudioSwatch`,
`StudioStyleRegistry`, and `StudioEffectStack` primitives (14.23). This sub-section references those
canonical contracts and MUST NOT redefine their fields; where any statement here conflicts with
14.23, 14.23 wins.

The compute-heavy geometry, tessellation, boolean, stroking, and brush-instancing work is owned by
the `VectorEngine` trait in the `studio-engine` crate (14.2); `handshake_core::studio::vector`
reaches it only through that typed boundary and never embeds GPU/tessellation dependencies in
`handshake_core` ([STU-ARC-002]). Durable vector authority is SurrealDB/EventLedger only
([STU-SDB-002]); no second store, cache or fixture database is permitted anywhere in this domain,
including tests.

---

## 14.5.0 Reading Rules: The Parameter Contract, Enumerations, and Derivation

**[STU-VEC-100] The seven-field numeric parameter contract.** Every numeric parameter defined
anywhere in 14.5 MUST be declared with SEVEN INDEPENDENT FIELDS. They are not interchangeable and
MUST NOT be collapsed:

*Derivation: reading aid inside a non-yielding clause; yields no microtask. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Meaning | Rule when unknown |
|---|---|---|
| `hard_min` | Smallest value the engine accepts. A smaller value is a validation error, not a clamp-and-continue. | Declare `unknown`; do NOT substitute `soft_min`. |
| `hard_max` | Largest value the engine accepts. | Declare `unknown`; do NOT substitute `soft_max`. |
| `soft_min` | Low end of the range the default control presents. A user or model MAY type below it, down to `hard_min`. | Declare `unknown`; do NOT substitute `hard_min`. |
| `soft_max` | High end of the range the default control presents. | Declare `unknown`; do NOT substitute `hard_max`. |
| `default` | Value the parameter holds on a newly created object or a freshly opened command. | Declare `unknown`. |
| `unit` | Token from the Studio unit vocabulary ([STU-VEC-101]). Never a guess. | Declare `unknown`. |
| `precision` | Decimal places carried and displayed. A fixed integer, or a named preference token. | Declare `unknown`. |

A parameter row that shows `unknown` in a bound column means the capture did not declare that bound.
An implementer MUST accept any value on that side subject to the bounds that ARE known, MUST NOT
clamp to the soft range, and MUST NOT invent a limit. Widening or narrowing a declared bound is a
spec change requiring a new clause, not an implementation decision.

**[STU-VEC-102] Soft bounds are UI presentation only.** A control whose `soft_min`/`soft_max` are
declared MUST present that range as its default drag/scrub extent, and MUST still accept typed and
model-supplied values across the full `hard_min`..`hard_max` interval. A model command MUST NOT be
rejected merely because its value falls outside the soft range. Where a capture declares the two
ranges as equal they are still emitted as four separate fields; equality is a fact about that
parameter, not a licence to store one range.

**[STU-VEC-103] Step contract.** Every scrubbable numeric parameter MUST additionally declare
`step`, `coarse_step` and `fine_step`. Where a capture does not declare them, Studio's default
derivation applies and MUST be recorded as DERIVED, not captured: `step = 10^(-precision)`,
`coarse_step = 10 x step`, `fine_step = step / 10` rounded to the parameter's precision. These three
are the only derived numeric fields permitted in 14.5. A parameter whose `precision` is `unknown`
has `step` `unknown` and MUST NOT be given a scrub gesture until its precision is captured.

**[STU-VEC-101] Unit vocabulary.** Every length-, angle- and ratio-bearing field MUST carry one of
these unit tokens; a bare number with no unit token is forbidden ([STU-DOC-003]).

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Token | Meaning |
|---|---|
| `pt` | PostScript point (1/72 in). The canonical internal geometry unit. |
| `px` | Device/document pixel. |
| `mm`, `cm`, `in`, `pc` | Millimetre, centimetre, inch, pica. |
| `document_unit` | The unit the containing `StudioDocument` declares. Resolved at the API decode boundary, never stored ambiguously. |
| `ruler_unit` | The unit the active ruler presents. Distinct from `document_unit`: a tool may scrub in ruler units while the stored geometry is `pt`. |
| `percent` | Percentage; 100 means unity unless the row says otherwise. |
| `deg` | Degrees. |
| `rad` | Radians. |
| `ppi`, `dpi` | Pixels/dots per inch. |
| `count` | Dimensionless integer count. |
| `ratio` | Dimensionless real multiplier. |
| `per_in` | Occurrences per inch. |
| `context` | The unit is selected by a sibling mode control on the same command; the row names that control. Never left implicit. |

**[STU-VEC-104] Enumeration contract.** Every enumerated parameter MUST declare its complete member
list, and each member MUST carry a stable Studio identifier AND the integer or token value the
capture recorded, so import/export and model commands round-trip without name matching. An
enumeration listed without values is non-conformant. Studio identifiers are Handshake-native; a
source product name never appears in a member identifier ([STU-SECTION-003]).

**[STU-VEC-105] Capture-conflict rule.** Where two captures disagree about the same parameter (for
example a polygon side-count limit that differs between two source applications), 14.5 MUST record
BOTH observed values, MUST name which Studio behaviour is normative, and MUST NOT silently pick one.
A deduped Studio feature inherits the WIDEST hard range across the captures that contributed to it
unless a clause states a narrower Studio-specific limit and gives the reason.

**[STU-VEC-106] Observed-value rule.** Values recovered by mining shipped preset or library content
(as opposed to a declared range in a dialog or type library) are OBSERVED, not legal bounds. An
observed range MUST be labelled `observed` and MUST NOT be used as a validation bound. Observed
ranges are admissible only as evidence that a parameter exists, as a sanity check on a default, and
as fixture material for tests.

**[STU-VEC-107] Microtask derivation rule.** The microtask set for this domain is derived
mechanically from this sub-section and from nothing else. Exactly one microtask is derived per
NUMBERED CLAUSE that introduces implementable behaviour; a clause whose only content is a
cross-reference, a reading rule, or a restatement of a cross-cutting obligation derives none. A
clause carrying a parameter table derives ONE microtask covering that whole table, because the
table is one coherent behaviour. A clause that both defines a primitive and enumerates its
operations derives one microtask for the primitive and one per operation family named in its
table. Every derived microtask inherits: the clause anchor as its `spec_anchor`; the clause's
parameter table verbatim as its `implementation_notes` payload; the clause's enumerations as its
acceptance vocabulary; one acceptance row per parameter bound-set and one per enumeration; and
the cross-cutting obligations of [STU-VEC-041] by reference, never restated. The derivation index
is [STU-VEC-199].

---

## 14.5.1 Vector Geometry Model: Paths and Vector Networks

**[STU-VEC-001]** Studio MUST provide exactly two canonical vector geometry primitives, and every vector
tool, shape, boolean result, brush spine, and import target MUST resolve to one of them:
`StudioVectorPath` (schema id `hsk.studio.vector_path@1`), an ordered sequence of anchors forming one
open or closed contour; and `StudioVectorNetwork` (schema id `hsk.studio.vector_network@1`), a graph
of anchors joined by first-class selectable edges where any anchor MAY join three or more edges and
enclosed regions MAY exist without a single closed contour. A `StudioVectorPath` MUST be losslessly
promotable to a `StudioVectorNetwork`; the reverse conversion is lossy and MUST be an explicit
flatten/simplify operation, never implicit.

**[STU-VEC-108] `StudioVectorNetwork` element contract.** The network is three parallel arrays plus
an index topology, and MUST serialise in exactly this shape:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Element | Fields | Notes |
|---|---|---|
| `vertices[]` | `x`, `y`, `stroke_cap?`, `stroke_join?`, `corner_radius?`, `handle_mirroring?` | Per-vertex cap/join/radius/mirroring override the object-level value when present. All four are optional; absent means "inherit from the object". |
| `segments[]` | `start` (vertex index), `end` (vertex index), `tangent_start?` (vector), `tangent_end?` (vector) | A segment with both tangents absent or zero-length is a straight line. Each tangent is relative to its own endpoint. |
| `regions[]` | `winding_rule`, `loops[][]` (arrays of segment indices), `fills?`, `fill_style_id?` | A region is one or more closed loops of segment indices. Per-region fill state is REQUIRED, not optional, so one layer holds filled and unfilled regions simultaneously. |

Indices are positions in the sibling array; an out-of-range index is a validation error, never a
clamp. Studio MUST expose set-network as an ATOMIC replace of all three arrays so a model can author
topology in one command rather than by incremental mutation, and MUST expose the derived read-only
`fill_geometry` and `stroke_geometry` path sets so a model can inspect the rendered outline without
re-deriving it.

**[STU-VEC-002]** An anchor MUST carry a position (document units per [STU-DOC-003]), an incoming
tangent handle, an outgoing tangent handle, and a handle-mirroring mode.

**[STU-VEC-109] Handle-mirroring enumeration.** Superseding the informal member names
in [STU-VEC-002], the normative enumeration is:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value | Behaviour |
|---|---|---|
| `handle_mirroring.none` | `NONE` | No mirroring; the two tangents move independently. Corner behaviour. |
| `handle_mirroring.angle` | `ANGLE` | Tangents share a direction, keep independent lengths. |
| `handle_mirroring.angle_and_length` | `ANGLE_AND_LENGTH` | Tangents are fully symmetric in direction and length. Smooth behaviour. |

Handle mirroring is settable per vertex AND per object. When an object-level read finds mixed
per-vertex values it MUST return an explicit `mixed` sentinel rather than a single value, and a
model MUST be able to read that sentinel and MUST NOT receive a silently-picked representative.
A second captured classification exists as a two-member point type (`smooth` = 1, `corner` = 2);
Studio MUST expose that as a DERIVED read-only projection of `handle_mirroring` (`none`, or
zero-length handles, project to `corner`; both other members project to `smooth`) and MUST NOT store
it independently.

**[STU-VEC-116] Anchor selection state.** Anchor selection is not a boolean. The captured
enumeration distinguishes which part of the anchor is selected, and Studio MUST carry the same
five-member state so a model can address a handle without addressing its anchor:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value |
|---|---|
| `anchor_selection.none` | `1` |
| `anchor_selection.anchor` | `2` |
| `anchor_selection.in_handle` | `3` |
| `anchor_selection.out_handle` | `4` |
| `anchor_selection.both_handles` | `5` |

**[STU-VEC-003]** A segment (edge) between two anchors MUST be a cubic Bezier; a straight segment is the
degenerate case where both governing handles are zero-length. Segments MUST be independently
selectable and directly reshapeable (drag-to-bend), and dragging a straight segment MUST convert it
to a curve by synthesizing handles on its endpoints without destroying adjacent geometry.

**[STU-VEC-004]** Every closed contour and every enclosed network region MUST carry an explicit fill
rule, and a `StudioVectorNetwork` MUST support per-region fill state so a single vector layer MAY
contain both filled and unfilled regions independently, and MUST support a paint-bucket region
operation that fills or clears one enclosed region without altering the network topology.

**[STU-VEC-110] Winding-rule enumeration**, superseding the informal names in [STU-VEC-004]:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value | Behaviour |
|---|---|---|
| `winding.non_zero` | `NONZERO` | Non-zero winding. |
| `winding.even_odd` | `EVENODD` | Even-odd / alternate. |
| `winding.none` | `NONE` | Legal only on a serialised path's winding field, meaning "this path carries no fill contribution". A REGION MUST NOT carry `winding.none`. |

**[STU-VEC-111] Path polarity.** Completing [STU-VEC-043]: the captured type library exposes contour
direction as a two-member polarity enumeration that Studio MUST carry with these values.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value |
|---|---|
| `polarity.positive` | `1` |
| `polarity.negative` | `-1` |

**[STU-VEC-043]** Every contour MUST carry an explicit path direction, and Studio MUST provide a
reverse-direction command; direction MUST be preserved through import/export and MUST govern
even-odd/winding hole resolution ([STU-VEC-004], [STU-VEC-110]) and the start/end semantics of
arrowheads ([STU-VEC-020]) and text-on-path ([STU-VEC-038]). Reversing direction MUST NOT alter
anchor positions or handle geometry.

**[STU-VEC-005]** Studio MUST provide live (non-destructive) corner treatment on individual anchors via
a `StudioCornerSpec` on the anchor, carrying a corner kind, a per-anchor radius, and a
corner-smoothing percentage that blends a rounded corner toward a continuous-curvature squircle.
Parametric shapes ([STU-VEC-010]) MUST expose the same corner spec per vertex (uniform or
per-corner), and corner edits MUST remain re-editable until the layer is explicitly
expanded/flattened.

**[STU-VEC-112] Corner-kind enumeration and corner numeric contract.** The informal list
in [STU-VEC-005] is superseded by the captured six-member corner-effect enumeration:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value | Shape |
|---|---|---|
| `corner.none` | `none` | Sharp corner. |
| `corner.round` | `rounded corner` | Convex circular arc. |
| `corner.round_inverted` | `inverse rounded corner` | Concave circular arc. |
| `corner.inset` | `inset corner` | Square notch cut into the corner. |
| `corner.bevel` | `bevel corner` | Straight chamfer. |
| `corner.fancy` | `fancy corner` | Decorative compound corner. |

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `corner_radius` (per anchor, per shape corner) | 0 | unknown | unknown | unknown | 0 | `document_unit` | preference-controlled: `dimension_precision` |
| `corner_smoothing` | 0 | unknown | unknown | unknown | 0 | `percent` | unknown |

`corner_radius` carries a hard lower bound of 0 -- a negative radius is a validation error -- and
no captured upper bound: it is clamped at edit time to the shorter of the two incident edge lengths
per [STU-VEC-066], and that clamp is a geometric consequence, not a declared maximum. The captured
display precision of a corner radius is a PREFERENCE TOKEN, not a fixed integer; Studio MUST carry
precision as a preference-controlled value, MUST expose the resolved integer to models, and MUST
NOT hard-code a decimal count.

**[STU-VEC-006]** Studio MUST provide the non-destructive path-topology operations below as first-class
commands, each available to both the operator UI and the model command surface as the identical
typed contract per [STU-DOC-004]: offset path (numeric inset/outset copy with join-style handling of
corners), simplify (anchor-count reduction at an adjustable strength while approximating the source
curve), outline stroke (convert a stroked path to filled geometry matching weight, align, caps,
joins, and dashes), and join / average (join selected open endpoints and average anchor positions on
one or both axes).

**[STU-VEC-113] Offset-path parameter contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `offset` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `miter_limit` | unknown | unknown | unknown | unknown | unknown | `ratio` | unknown |

`joins` is the enumeration `{joins.miter = 0, joins.round = 1, joins.bevel = 2}` captured from the
offset-path dialog and MUST use those integer values. Across 186 shipped serialized instances
`offset` was `observed` in -35.0 .. 18.0 and `miter_limit` `observed` at exactly 1.0, 4.0 and 6.0;
under [STU-VEC-106] neither is a validation bound. Offset path MUST offer a live preview toggle,
captured as a first-class dialog parameter; the preview toggle MUST NOT change the committed result.

**[STU-VEC-114] Simplify parameter contract.** Simplify is a four-parameter operation, not a single
"strength":

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `curve_precision` | unknown | unknown | 0 | 100 | 0 | `percent` | 2 |
| `angle_threshold` | unknown | unknown | 0 | 180 | 0 | `deg` | 2 |
| `smoothing` | unknown | unknown | 0 | 100 | 0 | `percent` | 2 |
| `convert_to_straight_lines` | boolean | -- | -- | -- | `false` | -- | -- |

`show_original_path` (default `false`) and `preview` (default `true`) are display-only and MUST NOT
alter the result. The standalone smoothing operation of the smooth tool ([STU-VEC-007]) uses the
same `smoothing` parameter with `unit = percent`, `precision = 2` and a captured default of 20.

**[STU-VEC-044]** Curvature continuity at an anchor MUST be classifiable and preservable: an anchor is
C0 (position only, corner), C1 (tangent-continuous, `handle_mirroring.angle`), or G2-approximate
(curvature-smoothed via corner smoothing [STU-VEC-005]). Tools that convert or reshape geometry MUST
NOT silently downgrade a smooth anchor to a corner; any continuity change MUST be an explicit,
history-tracked edit.

**[STU-VEC-045]** Geometry precision MUST be carried at a resolution independent of the current zoom or
artboard scale, and coordinate decode/encode MUST occur only at the API boundary per [STU-DOC-003].
Studio MUST NOT round anchor or handle coordinates to device pixels except when the operator or
model explicitly invokes pixel snapping ([STU-VEC-029]).

**[STU-VEC-115] Object dimension and opacity domain.** A vector art item's own width, height and
opacity carry captured declared ranges. These are HARD bounds: an operation whose result exceeds
them MUST fail closed with a dimension error rather than truncate.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `item_width` | 0.0 | 16348.0 | unknown | unknown | unknown | `pt` | unknown |
| `item_height` | 0.0 | 16348.0 | unknown | unknown | unknown | `pt` | unknown |
| `item_opacity` | 0.0 | 100.0 | unknown | unknown | 100.0 | `percent` | unknown |

A second capture in the same type library declares an absolute-transform entry range of 0 ..
16384 `document_unit`. Per [STU-VEC-105] both are recorded: the normative Studio hard maximum for
a STORED item dimension is 16348 `pt`; the 16384 figure applies only to the transform-entry field
of [STU-VEC-157], which MUST validate against the item bound after unit conversion and MUST
reject the gap rather than store an over-size item.

**[STU-VEC-117] Path measurement readouts.** Every path MUST expose, as typed read-only values that
a model can query without rendering, its enclosed `area` in square `pt` and its `length` in `pt`.
These are captured as first-class properties of the path object and are the typed backing for the
measurement surface required by [STU-VEC-057].

---

## 14.5.2 Drawing and Editing Tools

**[STU-VEC-007]** Studio MUST provide the deduped vector drawing/editing tool set in the table below.
Each row is ONE Studio tool that subsumes all listed source-suite variants per [STU-SECTION-003]; a
source product's tool name is never the Studio tool name. Every tool MUST emit its edits as
`studio.vector` events through the sandbox -> validation -> promotion lifecycle ([STU-ARC-005]) when
model-authored.

*Derivation: catalogue table, splits per row; yields 17 microtasks, one per Studio tool. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio tool | Function (normative) | Options clause |
|---|---|---|
| Pen | Place anchors and straight/curved segments; connect to any existing network anchor, not only endpoints; drag to create mirrored tangent handles | [STU-VEC-118] |
| Curvature | Draw and edit smooth curves by placed points with rubber-band preview; click toggles corner/smooth | [STU-VEC-118] |
| Anchor Convert | Toggle anchor corner<->smooth, break/join tangents, drag a straight edge to a curve | [STU-VEC-109] |
| Freehand (pencil) | Draw freehand strokes auto-smoothed to a path | [STU-VEC-119] |
| Node select | Select and edit individual anchors, handles, segments and regions; multi-node selection and alignment | [STU-VEC-116] |
| Reshape | Adjust a path region while preserving overall curve continuity | -- |
| Width | Add/move/remove width points along a stroke to author a variable-width profile on canvas; reset profile | [STU-VEC-122] |
| Scissors | Split a path at a clicked parametric point into two coincident open endpoints | [STU-VEC-048] |
| Knife | Cut paths/shapes along a freehand or straight cut line into separate closed objects | [STU-VEC-048] |
| Join | Join selected open endpoints; average anchor positions on one or both axes | [STU-VEC-006] |
| Shape Builder | Drag across overlapping regions to merge, click to extract/delete regions, composing geometry without manual boolean stacking | [STU-VEC-123] |
| Blob brush | Paint filled unified vector shapes that merge with same-attribute geometry | [STU-VEC-120] |
| Vector eraser | Erase along a dragged path, splitting/trimming vector geometry | [STU-VEC-121] |
| Corner | Apply live corner treatment ([STU-VEC-112]) to selected anchors | [STU-VEC-112] |
| Vector crop | Non-destructively crop vector/placed objects to a region without discarding content | -- |
| Point transform | Transform an object around a movable, node-keyed origin | [STU-VEC-157] |
| Sculpt | Push/pull geometry with a radial falloff whose strength is a percentage | [STU-VEC-124] |

**[STU-VEC-008]** The pen, curvature, freehand, blob brush and brush tools MUST author either a
`StudioVectorPath` or, where the drawn geometry joins existing edges or creates enclosed regions,
contribute to a `StudioVectorNetwork`; the tool MUST NOT silently discard network topology by
collapsing to a single contour.

**[STU-VEC-009]** The node-select tool MUST expose, for the current selection, the anchor mirroring mode
([STU-VEC-109]), the per-anchor corner spec ([STU-VEC-112]), the anchor selection state
([STU-VEC-116]) and the region fill state ([STU-VEC-110]) as directly editable typed values, so a
no-context model can read and set every geometry attribute without pixel-picking.

**[STU-VEC-046]** Studio MUST provide drawing modes that govern where new vector art is placed relative
to the selection: `draw_normal` (above the active layer), `draw_behind` (below the current
selection), and `draw_inside` (clipped into the selected object as an automatic clip). The active
drawing mode MUST be a persisted, model-readable tool state and MUST apply uniformly to the pen,
freehand, shape, brush and blob tools.

**[STU-VEC-047]** Tool options MUST be persisted per tool and exposed as typed, model-settable values. A
model MUST be able to configure a tool and then invoke it deterministically; interactive-only tool
state that a model cannot read or set is forbidden where a structured path is practical
([STU-DOC-004]). The per-tool option contracts are [STU-VEC-118] through [STU-VEC-124].

**[STU-VEC-118] Pen and curvature tool option contract.** The pen family carries a small set of
captured edit-behaviour selectors rather than numeric parameters. Studio MUST expose all five as
persisted enumerated tool state:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Option | Kind | Members | default |
|---|---|---|---|
| `anchor_edit_mode` | enumeration | captured as an indexed popup; Studio member ids are `anchor_edit.default`, and further members are UNKNOWN in the capture (the popup's item list was not recoverable) | index `0` |
| `arc_edit_mode` | enumeration | as above, members UNKNOWN | index `0` |
| `line_edit_mode` | enumeration | as above, members UNKNOWN | index `0` |
| `arc_match_mode` | enumeration | as above, members UNKNOWN | index `0` |
| `line_match_mode` | enumeration | as above, members UNKNOWN | index `0` |

The five selectors ARE captured as existing, persisted, defaulted controls; their member lists were
NOT recovered. Under [STU-VEC-100] and [GLOBAL-ANTISPECULATION-004] the member lists are declared
UNKNOWN and MUST NOT be invented. An implementer MUST ship the five selectors as typed enumerated
state with at least the captured default index and MUST raise a spec-gap rather than guess members.
The rubber-band preview required by [STU-VEC-047] is a separate boolean, captured on the curvature
tool's own options panel.

**[STU-VEC-119] Freehand (pencil) tool option contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `fidelity` | unknown | unknown | 0 | 4 | unknown | `count` | 0 |
| `close_within_distance` | unknown | unknown | unknown | unknown | unknown | `px` | 0 |
| `join_within_distance` | unknown | unknown | unknown | unknown | unknown | `px` | 2 |
| `smooth_fidelity` (smooth tool) | unknown | unknown | unknown | unknown | 20 | `percent` | 0 |

Booleans, all captured on the same panel: `fill_new_strokes`, `keep_selected`,
`alt_toggles_smooth_tool`, `close_paths_when_ends_are_within` (gate for `close_within_distance`),
`edit_selected_paths` (gate for `join_within_distance`), `live_preview`, `live_curve_fitting`,
`round_caps_on_new_document`. Defaults for these booleans are UNKNOWN in the capture and MUST be
declared unknown rather than assumed. `fidelity` is captured on a 0..4 slider on the blob-brush
panel and as a percentage slider on the brush-manager panel; per [STU-VEC-105] Studio normalises
fidelity to a single `percent` parameter with `precision = 0` and MUST record the 0..4 discrete
scale as the coarse detent set, not as the storage type.

**[STU-VEC-120] Blob-brush tool option contract.** The blob brush is a filled-geometry painter, not
a stroke tool; its nib parameters are the calligraphic nib of [STU-VEC-152].

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `fidelity` | unknown | unknown | 0 | 4 | unknown | `count` | unknown |
| `size` | unknown | unknown | unknown | unknown | 1 | `ruler_unit` | 1 |
| `angle` | unknown | unknown | unknown | unknown | 0 | `deg` | 0 |
| `roundness` | unknown | unknown | unknown | unknown | 100 | `percent` | 0 |

Booleans: `keep_selected` (default `false`), `merge_only_with_selection` (default `false`). Each of
`size`, `angle` and `roundness` additionally carries a dynamics selector drawn from [STU-VEC-150].

**[STU-VEC-121] Vector-eraser tool option contract.** The eraser nib is the same three-parameter nib
as the blob brush, with different captured defaults:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `angle` | unknown | unknown | unknown | unknown | 0 | `deg` | 0 |
| `roundness` | unknown | unknown | unknown | unknown | 100 | `percent` | 0 |
| `size` | unknown | unknown | unknown | unknown | 0 | `ruler_unit` | 1 |

Each parameter carries a dynamics selector from [STU-VEC-150].

**[STU-VEC-122] Width-point editing contract.** A variable-width profile is a list of width points;
each point carries `position` (0..1 along the path, `ratio`) and `width`. Editing a width point MUST
expose three captured booleans as typed state, because they change the result and a model cannot
infer them: `adjust_adjoining_width_points` (default `false`), `single_width_only_incoming`
(default `false`), `single_width_only_outgoing` (default `false`). A width point MUST be able to
carry independent incoming and outgoing widths; a profile in which every point has equal incoming
and outgoing width is the uniform case, not a separate type.

**[STU-VEC-123] Shape-builder and region-merge option contract.** The gesture front-end
of [STU-VEC-050] carries captured gap handling that MUST be exposed as typed state:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Option | Kind | Members / bounds | default |
|---|---|---|---|
| `gap_detection` | boolean | -- | UNKNOWN (captured without a default) |
| `gap_length` | enumeration | `gap.small = 0`, `gap.medium = 1`, `gap.large = 2`, `gap.custom = 3` | UNKNOWN |
| `treat_open_filled_path_as_closed` | boolean | -- | UNKNOWN |
| `click_stroke_splits_path` | boolean | -- | `true` |
| `colour_pick_source` | enumeration | `pick.artwork = 0`, `pick.swatches = 1` | `pick.artwork` |
| `cursor_swatch_preview` | boolean | -- | `true` |
| `cut_style` | enumeration | `cut.straight_line = 0`, `cut.freeform = 1` | index `1` selected in the capture |
| `highlight_fill` | boolean | -- | UNKNOWN |
| `highlight_stroke_when_editable` | boolean | -- | UNKNOWN |

`gap.custom` requires a numeric custom gap length; the capture declares the mode but not the
numeric field's bounds, unit or default, so all seven fields of that parameter are UNKNOWN and MUST
be raised as a spec gap before the custom mode is implemented.

**[STU-VEC-124] Sculpt / push-pull contract.** The captured sculpt surface carries a single
percentage strength driving a radial deformation:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `strength` | unknown | unknown | unknown | unknown | unknown | `percent` | unknown |

It carries a preview boolean. This is the least-specified tool in the capture; an implementer MUST
treat every numeric field as a spec gap and MUST NOT ship a guessed range.

**[STU-VEC-048]** Scissors and knife operations MUST define deterministic results: scissors splits a
path at a parametric point on a segment into two open endpoints sharing a coincident position; knife
cuts across one or more objects along a freehand or straight cut line, producing separately closed
regions where the cut crosses filled area and open endpoints where it crosses strokes only. Both
MUST preserve the source appearance stack on the resulting fragments. A cut whose result would be an
empty region MUST fail closed with an explicit empty-result error, matching the captured
empty-pathfinder-result constraint of [STU-VEC-133].

**[STU-VEC-071]** Vector selection MUST use the shared `StudioSelectionSet` primitive (14.3) and MUST
support object selection, direct anchor/segment/region selection, marquee and lasso selection, and
selection-by-attribute. Selection scope MUST be model-addressable as a typed query, not only
mouse-driven.

**[STU-VEC-125] Selection-by-attribute predicate set.** The captured attribute-selection surface
enumerates exactly which attributes participate, and carries per-attribute enable state with
defaults. Studio MUST implement this predicate set and MUST expose it as a typed query object:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Predicate | Included by default |
|---|---|
| `fill_colour` | `true` |
| `stroke_colour` | `false` |
| `stroke_weight` | `false` |
| `opacity` | `false` |
| `blend_mode` | `false` |

A tolerance parameter governs colour matching; its bounds, unit and default are UNKNOWN in the
capture and MUST be declared as a spec gap. [STU-VEC-037]'s wider "select same" list (graphic style,
shape kind, symbol/component instance) is preserved and is a Studio superset of the captured five;
those three additional predicates MUST default to `false`.

**[STU-VEC-126] Document cleanup contract.** Studio MUST provide a document-scoped cleanup command
whose captured scope is exactly three independent boolean targets, each of which MUST be separately
selectable and separately reported in the result receipt: `stray_points` (isolated anchors with no
segment), `unpainted_objects` (objects with neither fill nor stroke), `empty_text_paths` (text
frames with no content). Cleanup MUST be one history entry ([STU-VEC-073]) and MUST report counts
per target so a model can verify the effect without re-scanning the document.

**[STU-VEC-072]** Vector object management (group/ungroup, arrange/z-order, lock, hide, rename) is owned
by the shared `StudioLayer`/`StudioLayerGraph` surface (14.3); the vector domain MUST consume it and
MUST NOT reimplement grouping or stacking. Group/ungroup MUST preserve child identity and appearance
([STU-VEC-070]), and z-order MUST be the ordering input consumed by order-dependent geometry
operations ([STU-VEC-049]). The captured z-order command set is the four-member enumeration
`{z_order.bring_to_front = 1, z_order.bring_forward = 2, z_order.send_backward = 3,
z_order.send_to_back = 4}` and Studio MUST use those values.

---

## 14.5.3 Parametric Shape Catalog

**[STU-VEC-010]** Studio MUST provide a parametric shape catalog. Each shape is a live
`StudioVectorPath`/`StudioVectorNetwork` whose defining parameters remain editable until the
operator or model explicitly expands it to raw geometry. Expansion MUST be an explicit,
history-tracked command. The normative catalog is [STU-VEC-127]; a shape present in any source suite
MUST be representable, and the per-shape parameters MUST be preserved -- no parameter is dropped in
dedup.

**[STU-VEC-127] Parametric shape catalog with captured bounds.** SUPERSEDES the unbounded shape
table of v02.205 [STU-VEC-010], which listed shape parameters with no ranges, no defaults and no
units, and which named an independent "outer radius" parameter on the star that no capture carries.

The catalog is stated as two tables. The first names the nine Studio shapes and is the build
surface: each row is one shape to implement. The second carries the captured numeric bounds for those
shapes' parameters and is a single bound-set contract, not a second list of shapes; several shapes
occupy more than one row there because their parameters are grouped by kind.

*Derivation: catalogue table, splits per row; yields 9 microtasks, one per parametric shape. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio shape | Editable parameters | Live-shape behaviour |
|---|---|---|
| Rectangle | `width`, `height`, per-corner `corner_radius` (4 independent), `corner_kind`, `corner_smoothing` | Corner spec per [STU-VEC-112]; radius clamps per [STU-VEC-066]. |
| Ellipse / arc | `start_angle`, `end_angle`, `inner_ratio` | Ring, donut, pie and segment are `inner_ratio` and sweep states of one shape, not four shapes. |
| Polygon | `side_count`, `inset_percentage` | `inset_percentage` gives an N-gon its star-ness without changing shape kind. |
| Star | `point_count`, `inner_radius` | Outer radius is the object's own size, not an independent parameter. |
| Line / segment | `length`, `angle` | Snap-to-perpendicular and snap-to-tangent while drawing per [STU-VEC-029]. |
| Arc segment | `slope`, `type`, `base_along`, `fill_arc` | Open or closed arc on a settable base axis. |
| Rectangular grid | `skew_horizontal`, `skew_vertical`, `use_outside_rectangle_as_frame`, `fill_grid` | Row and column dividers with per-axis skew. |
| Polar grid | `skew_radial`, `skew_angular`, `create_compound_path_from_ellipses`, `fill_grid` | Concentric and radial dividers with per-axis skew. |
| Spiral | `turns`, `decay` | Decay is the per-turn radius ratio. |

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio shape | Parameters | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|---|
| Rectangle | `width`, `height` | 0.0 | 16348.0 | unknown | unknown | unknown | `pt` | unknown |
| Rectangle | `corner_radius` per corner (4 independent) plus `corner_kind`, `corner_smoothing` | see [STU-VEC-112] | | | | | | |
| Ellipse / arc | `start_angle`, `end_angle` | unknown | unknown | unknown | unknown | unknown | `deg` | unknown |
| Ellipse / arc | `inner_ratio` (ring / donut / pie / segment) | unknown | unknown | unknown | unknown | unknown | `ratio` | unknown |
| Polygon | `side_count` | 3 | 20 (vector-suite capture) / 100 (layout-suite capture) | 3 | 20 | unknown | `count` | 0 |
| Polygon | `inset_percentage` (star-ness of an N-gon) | 0 | 100 | unknown | unknown | unknown | `percent` | 0 |
| Star | `point_count` | 3 | 50 | 3 | 50 | unknown | `count` | 0 |
| Star | `inner_radius` | unknown | unknown | unknown | unknown | unknown | `ratio` | unknown |
| Line / segment | `length` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| Line / segment | `angle` | unknown | unknown | unknown | unknown | unknown | `deg` | unknown |
| Arc segment | `slope` | unknown | unknown | -100 | 100 | 0 | `percent` | 2 |
| Rectangular grid | `skew_horizontal`, `skew_vertical` | unknown | unknown | -500 | 500 | 0 | `percent` | 2 |
| Polar grid | `skew_radial`, `skew_angular` | unknown | unknown | -500 | 500 | 0 | `percent` | 2 |
| Spiral | `turns`, `decay` | unknown | unknown | unknown | unknown | unknown | `count`, `percent` | unknown |

The bound-set table above repeats a shape name in its first column wherever that shape carries
parameters in more than one unit group. It is a single bound-set contract, not a second shape
catalogue: the nine shapes to build are the nine rows of the catalogue table that precedes it.

Per [STU-VEC-105] the polygon `side_count` maximum is recorded from two captures that disagree: the
vector suite's live-shape control declares a 3..20 slider; the layout suite's polygon preference
declares a hard 3..100. The NORMATIVE Studio hard range is 3..100 (the widest captured hard range);
3..20 is the normative SOFT range because it is the only captured presentation range. Both facts are
retained so neither is lost.

Non-numeric shape members captured on these tools:

- Arc segment `type` (open / closed) and `base_along` (X axis / Y axis) are indexed popups whose
  member lists were not recovered; both default to index `1`. Members UNKNOWN.
- Arc segment `fill_arc` boolean, default `false`.
- Line segment `fill_line` boolean, default UNKNOWN.
- Rectangular grid `use_outside_rectangle_as_frame` boolean, default `true`; `fill_grid` boolean,
  default `true`.
- Polar grid `create_compound_path_from_ellipses` boolean, default `false`; `fill_grid` boolean,
  default `true`.

**[STU-VEC-011]** The shape catalog is extensible: adding a Studio-native parametric shape MUST be a
matter of registering a new parameter schema against the same shape-primitive contract, and MUST NOT
require a new layer kind or a parallel geometry model. Every parametric behaviour that carries
geometry meaning MUST be preserved under a Studio-native name.

**[STU-VEC-128] Extended-primitive registration requirement.** The extended parametric primitives
(cog with teeth/hole, crescent, heart, tear, cloud, callout with tail position/size, double star,
square star, arrow with head/tail style and shaft thickness, trapezoid/triangle/diamond with
apex/midpoint/edge offsets, encoded-payload data glyph with error-correction level) are in scope and
MUST be registrable under [STU-VEC-011]. NO parameter bounds, defaults, units or precisions for
these shapes were recovered by the green room. Each is therefore a declared SPEC GAP: an implementer
MUST register the shape and MUST raise the missing parameter contract rather than invent bounds.
This clause exists so the gap is recorded in the spec instead of being silently dropped.

**[STU-VEC-065]** Parametric shapes MUST support both handle-based on-canvas parameter editing and
numeric entry of every parameter, and the two MUST be equivalent. A shape's parameters MUST remain
individually editable after transforms; a uniform scale MUST NOT silently expand a live shape to raw
geometry unless expansion is requested ([STU-VEC-010]). A transform command MUST expose the captured
`scale_corners` boolean (default `false`) so the operator or model chooses whether corner radii
scale with the object ([STU-VEC-157]).

**[STU-VEC-066]** Rectangles, frames and vector anchors MUST support per-corner independent radius (a
uniform value or four/N independent values) plus the corner smoothing of [STU-VEC-112]; a corner
radius MUST clamp to the available edge length rather than produce invalid geometry, and the clamp
behaviour MUST be deterministic: the applied radius is `min(requested_radius, shorter_incident_edge
/ 2)` evaluated per corner, and the clamped value MUST be reported back to the caller so a model
sees what was actually applied rather than what it asked for.

**[STU-VEC-129] Live-shape conversion.** Converting an arbitrary path INTO a parametric shape MUST
be supported and MUST be an explicit command carrying the target shape kind plus, where the target
is a polygon, `side_count`, `inset_percentage` and `corner_radius`. The captured conversion API takes
exactly those three arguments alongside the shape selector. Conversion MUST fail closed with an
explicit error when the source cannot be represented; the captured constraint vocabulary names four
such failures that Studio MUST reproduce as distinct typed errors rather than one generic failure:
cannot convert shape; cannot convert point to shape; cannot convert an orthogonal line to a shape;
cannot convert a line to a closed path.

---

## 14.5.4 Boolean, Compound, and Geometry Operations

**[STU-VEC-012]** Studio MUST provide ONE unified geometry-operation set that subsumes every source
suite's boolean/pathfinder/geometry command family per [STU-SECTION-003].

**[STU-VEC-130] Canonical geometry-operation enumeration.** SUPERSEDES the ten-row table of
v02.205 [STU-VEC-012], which omitted three captured operations (hard mix, soft mix, trap) and gave
no operation tokens. The captured operation enumeration has thirteen members; Studio MUST carry
all thirteen with these stable identifiers and MUST use the captured token as the wire value:

*Derivation: catalogue table, splits per row; yields 13 microtasks, one per geometry operation. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio operation | Captured token | Result (normative) |
|---|---|---|
| `geom.union` | `uniteCommand` | Merge selected regions into one combined outline. |
| `geom.intersect` | `intersectCommand` | Keep only the overlapping region. |
| `geom.exclude` | `excludeCommand` | Keep non-overlapping regions (even-odd result). |
| `geom.subtract_front` | `backMinusFrontCommand` | Remove upper region(s) from the lowest. |
| `geom.subtract_back` | `frontMinusBackCommand` | Remove lower region(s) from the topmost. |
| `geom.divide` | `divideCommand` | Split all overlaps into separate closed regions. |
| `geom.trim` | `trimCommand` | Remove hidden overlaps, keeping region boundaries. |
| `geom.merge` | `mergeCommand` | Remove hidden overlaps and merge adjacent same-paint regions. |
| `geom.crop` | `cropCommand` | Clip artwork to the topmost region. |
| `geom.outline` | `outlineCommand` | Convert region borders to stroked outline geometry. |
| `geom.hard_mix` | `hardCommand` | Composite overlaps by taking the extreme channel value per component. |
| `geom.soft_mix` | `softCommand` | Composite overlaps by blending with a mixing rate. |
| `geom.trap` | `trapCommand` | Generate a prepress trap between adjacent regions ([STU-VEC-132]). |

A second capture, from the collaborative design suite, declares a FOUR-member live boolean node
type: `UNION`, `INTERSECT`, `SUBTRACT`, `EXCLUDE`. Per [STU-VEC-105] both are recorded. The
normative Studio set is the thirteen above; the four-member set is the subset that MUST be
expressible as a LIVE boolean node under [STU-VEC-013], and the remaining nine MAY be flatten-only
where a live formulation is not defined.

Offset/expand-stroke ([STU-VEC-113], [STU-VEC-021]) is NOT a member of this enumeration: it is a
path-topology operation under [STU-VEC-006] and MUST NOT be re-registered as a boolean.

**[STU-VEC-131] Geometry-operation option contract.** Three captured options govern every operation
in [STU-VEC-130] and MUST be exposed as persisted, model-settable state, not hidden constants:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Option | Kind | Bounds / members | default |
|---|---|---|---|
| `precision` | numeric | hard_min unknown, hard_max unknown, soft_min unknown, soft_max unknown, unit `pt`, precision unknown | unknown |
| `remove_redundant_points` | boolean | -- | unknown |
| `divide_and_outline_remove_unpainted` | boolean | -- | unknown |

`precision` is captured as a numeric field carrying the literal suffix `points`; its bounds and
default were not recovered and MUST NOT be invented. It is the coordinate-snapping tolerance the
boolean core uses; a model MUST be able to read it, because two runs at different precisions produce
different geometry and [STU-VEC-049] requires byte-identical repeatability only at a fixed
precision.

**[STU-VEC-132] Trap operation contract.** `geom.trap` produces prepress trap geometry and carries
two captured booleans: `traps_with_process_colour` and `reverse_traps`, both with UNKNOWN defaults.
Trap WIDTH, black width, join/end style, appearance thresholds and image placement are owned by the
layout/prepress trap-preset contract ([STU-LAY-152]) and MUST NOT be redefined here; the vector
domain consumes that preset. This clause exists so the vector-side operation is not lost and so the
prepress preset is named as its parameter source.

**[STU-VEC-013]** Every boolean operation MUST support a live (non-destructive) result mode and a
flattened (destructive) result mode. In live mode the result is a `StudioCompoundShape` whose child
geometry and per-child operator remain individually editable and movable while the composite outline
updates; flatten MUST be an explicit command that bakes the composite into a single
`StudioVectorNetwork`. Live boolean groups MUST participate in the appearance model as a single
styleable object, and MUST expose the captured derived `fill_geometry` and `stroke_geometry` so a
model can read the composite outline without flattening it.

**[STU-VEC-070]** Flatten MUST be defined as the general destructive-merge command over any selection
(boolean groups, live constructs and text outlines included), producing one `StudioVectorNetwork`
that bakes the composite geometry; it MUST be distinct from group/ungroup (which preserve child
identity) and from expand-appearance ([STU-VEC-054], which bakes paint/effects). A model invoking
flatten MUST be able to predict that child identity and live parameters are lost.

**[STU-VEC-014]** Compound paths (a single object with holes formed by multiple contours under a shared
fill rule) MUST be a distinct, supported construct from compound/boolean groups, and Studio MUST
provide make/release commands for them. Releasing a compound or boolean construct MUST restore the
independent child geometry.

**[STU-VEC-049]** Geometry operations MUST be deterministic and repeatable: the same operation over the
same input geometry, z-order and `precision` ([STU-VEC-131]) MUST produce byte-identical output
geometry, and each operation's dependence on z-order MUST be stated in its typed contract so a model
can predict the result from selection order alone. In [STU-VEC-130] the z-order-dependent operations
are exactly `geom.subtract_front`, `geom.subtract_back`, `geom.crop`, `geom.trim`, `geom.merge` and
`geom.trap`; the others are order-independent and MUST produce identical results under any input
permutation.

**[STU-VEC-050]** The shape-builder tool ([STU-VEC-007]) MUST be defined as an interactive front-end
over the same `VectorEngine` boolean core used by the explicit geometry operations
([STU-VEC-130]); merge gestures MUST resolve to `geom.union` and extract gestures to
`geom.subtract_front`/`geom.divide`, so gestural and command-driven geometry share one deterministic
implementation and one result contract.

**[STU-VEC-133] Geometry-operation failure vocabulary.** The captured constraint vocabulary names
distinct failures that Studio MUST reproduce as separate typed errors rather than one generic
"operation failed": empty pathfinder result (the operation describes an empty region); result too
small; result too large; illegal dimension; illegal scale value; illegal skew value with the
captured message bound `the value must be between -360.0 and 360.0 degrees`; path index out of
bounds; point index out of bounds; cannot join path points; object is locked; selection contains
frames with no content. Each MUST be individually distinguishable by a model from its typed error
code, because the remediation differs per case.

---

## 14.5.5 Fills

**[STU-VEC-015]** A vector object's paint MUST be expressed as an ordered stack of fill entries
(see [STU-VEC-022]); each fill entry MUST carry a fill kind, per-fill opacity, per-fill blend mode
and a visibility toggle. Fills MUST be independently reorderable and removable.

**[STU-VEC-134] Paint-kind enumeration.** SUPERSEDES the five informal fill kinds of
v02.205 [STU-VEC-015]. Two captures contribute; both are recorded per [STU-VEC-105]. The
vector suite's colour-kind enumeration carries integer values that Studio MUST preserve at
the interchange boundary:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio member | Captured value | Meaning |
|---|---|---|
| `paint.none` | `0` | No paint. |
| `paint.process_cmyk` | `1` | Process CMYK component colour. |
| `paint.gray` | `2` | Single-channel grey. |
| `paint.rgb` | `3` | RGB component colour. |
| `paint.spot` | `4` | Named spot colour with tint ([STU-VEC-141]). |
| `paint.pattern` | `5` | Tiled pattern ([STU-VEC-138]). |
| `paint.gradient` | `6` | Gradient ramp ([STU-VEC-135]). |

The collaborative-suite capture adds two paint kinds that the integer enumeration above does not
carry and that Studio MUST support as first-class members with token values, because they are
distinct paint behaviours and not colour models: `paint.image` (token `IMAGE`, [STU-VEC-139]) and
`paint.video` (token `VIDEO`). `paint.lab` is additionally required by [STU-VEC-019]'s Lab entry
requirement and by the spot-colour Lab definition; it carries no captured integer and its
interchange value is UNKNOWN.

`paint.video` is an animated fill. v02.205 [STU-VEC-018] stated that "video/animated fills are an
optional playback concern and MUST NOT be a required vector feature". That statement is RETAINED as
a vector-domain scope edge: `paint.video` MUST be representable and MUST round-trip, but rendering
its playback is owned by the motion/video domain, not by 14.5.

**[STU-VEC-135] Gradient geometry enumeration.** SUPERSEDES the six-member informal list of
v02.205 [STU-VEC-016]. Two captures disagree on the closed set and both are recorded:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio member | Captured value | Source of the value |
|---|---|---|
| `gradient.linear` | `1` / `GRADIENT_LINEAR` | Both captures. |
| `gradient.radial` | `2` / `GRADIENT_RADIAL` | Both captures. |
| `gradient.angular` | `GRADIENT_ANGULAR` | Collaborative-suite capture only; no integer captured. |
| `gradient.diamond` | `GRADIENT_DIAMOND` | Collaborative-suite capture only; no integer captured. |
| `gradient.freeform` | UNKNOWN | Present as a distinct authoring construct in the vector suite (point/line editing mode) but NOT a member of its two-member gradient-type enumeration. Studio carries it as a gradient kind; its interchange value is UNKNOWN. |
| `gradient.mesh` | UNKNOWN | Present as a distinct object kind (mesh item), NOT as a gradient-type enumerator. Studio carries it as a gradient kind per [STU-VEC-034]; its interchange value is UNKNOWN. |

The normative Studio set is all six. The recorded fact that only `linear` and `radial` carry captured
integers means an interchange writer MUST NOT assume a numeric round-trip for the other four and
MUST use the token form.

**[STU-VEC-136] Gradient-stop contract.** Every gradient carries an ordered stop list. Per stop:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `ramp_position` | 0.0 | 100.0 | unknown | unknown | unknown | `percent` | unknown |
| `midpoint` | 13.0 | 87.0 | unknown | unknown | unknown | `percent` | unknown |
| `opacity` | 0.0 | 100.0 | unknown | unknown | unknown | `percent` | unknown |

`midpoint`'s hard range of 13..87 is a DECLARED bound in the capture, confirmed independently by a
second application's declared range for the same field. It is not a UI nicety: a midpoint outside
13..87 MUST be rejected. `ramp_position` is stored as a percentage in the vector-suite capture and as
a 0..1 `ratio` in the collaborative-suite capture; per [STU-VEC-105] Studio stores `percent` and MUST
convert at the interchange boundary, never at the storage layer.

**[STU-VEC-137] Gradient interpolation and dither.** SUPERSEDES the clause in v02.205 [STU-VEC-016]
that required "per-gradient interpolation control (`perceptual` or `linear`)". The captured control
is a two-member popup whose members are NOT `perceptual` and `linear`:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value | Meaning |
|---|---|---|
| `gradient_interpolation.classic` | `0` | The legacy component-space ramp. |
| `gradient_interpolation.perceptual` | `1` | Perceptually uniform ramp. |

`dither` is a separate captured boolean with default `false`, and MUST be modelled as an independent
field, not as a third interpolation member. A freeform gradient additionally carries a two-member
element-kind selector `{freeform_element.points = 0, freeform_element.lines = 1}` governing whether
colour is placed as isolated points or along colour lines. Studio MUST expose all three controls
separately.

**[STU-VEC-016]** Every gradient MUST carry a multi-stop ramp with per-stop colour and opacity, an
editable midpoint/skew between stops, on-canvas handle editing of position/rotation/extent, the
interpolation and dither controls of [STU-VEC-137], and MUST be applicable to fills and to strokes;
on strokes the application mode MUST be selectable between `within`, `along` and `across` the stroke.

**[STU-VEC-017]** `StudioPattern` MUST support tiled fills with an editable tile-layout type, brick/hex
offset, tile size (with move-with-art), spacing, overlap order, and side/corner/start/end tiling for
path application. A pattern MUST be transformable independently of the object it fills. Studio MUST
also support a live-source pattern that tiles another in-document object as a repeating fill or
stroke with spacing and alignment controls.

**[STU-VEC-138] Pattern paint contract.** Two captures contribute and both are normative; the
live-source pattern is the second capture's model and MUST NOT be reimplemented as a copy of the
first.

Definition-backed pattern (the tile-artwork model) carries a transform whose captured defaults are:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `pattern_rotation` | unknown | unknown | unknown | unknown | 0.0 | `deg` | unknown |
| `pattern_shear_angle` | unknown | unknown | unknown | unknown | 0.0 | `deg` | unknown |
| `pattern_shear_axis` | unknown | unknown | unknown | unknown | 0.0 | `deg` | unknown |
| `pattern_shift_angle` | unknown | unknown | unknown | unknown | 0.0 | `deg` | unknown |
| `pattern_shift_distance` | unknown | unknown | unknown | unknown | 0.0 | `document_unit` | unknown |
| `pattern_reflect_angle` | unknown | unknown | unknown | unknown | 0.0 | `deg` | unknown |
| `pattern_scale` | unknown | unknown | unknown | unknown | unknown | `percent` | unknown |

plus `pattern_reflect` boolean, captured default `false`.

Live-source pattern (the tile-another-node model) carries:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Parameter | Kind | Members / bounds | default |
|---|---|---|---|
| `source_node_id` | reference | the in-document object being tiled | -- |
| `tile_type` | enumeration | `tile.rectangular = RECTANGULAR`, `tile.hex_by_row = HORIZONTAL_HEXAGONAL`, `tile.hex_by_column = VERTICAL_HEXAGONAL` | unknown |
| `scaling_factor` | numeric | hard/soft unknown, unit `ratio`, precision unknown | unknown |
| `spacing` | vector (x, y) | hard/soft unknown, unit `document_unit`, precision unknown | unknown |
| `horizontal_alignment` | enumeration | `align.start = START`, `align.center = CENTER`, `align.end = END` | unknown |

v02.205 [STU-VEC-017] listed a five-member tile-layout set including `brick_by_row` and
`brick_by_column`. The capture declares only three tile types. Per [STU-VEC-105] Studio's normative
tile-type set is the union of five, with `tile.brick_by_row` and `tile.brick_by_column` carrying
UNKNOWN interchange values and being declared as a spec gap for their offset parameters, which were
not recovered.

**[STU-VEC-067]** Studio MUST provide a pattern editing mode that isolates a pattern's tile artwork for
direct editing with live preview of the tiled result and dimmed neighbour copies, exiting back to
the document without materialising the tiles. Edits to a pattern definition MUST update every object
that references it ([STU-VEC-051]).

**[STU-VEC-018]** Image fills MUST support the scaling modes of [STU-VEC-139] and MUST expose
non-destructive render-time image adjustments (exposure, contrast, saturation, temperature, tint,
highlights, shadows) consistent with the raster domain (14.4). Video and animated fills are an
optional playback concern owned by the motion/video domain and MUST NOT be a required vector
rendering feature, but MUST be representable and MUST round-trip per [STU-VEC-134].

**[STU-VEC-139] Image paint contract.** SUPERSEDES the four informal scaling modes of
v02.205 [STU-VEC-018] by giving them captured tokens:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value | Behaviour |
|---|---|---|
| `image_scale.fill` | `FILL` | Scale to cover the bounds, cropping overflow. |
| `image_scale.fit` | `FIT` | Scale to fit within the bounds. |
| `image_scale.crop` | `CROP` | Non-destructive in-bounds reposition/scale/rotate of the source, keeping the full source recoverable. |
| `image_scale.tile` | `TILE` | Repeat the source at a set scale. |

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `image_rotation` | unknown | unknown | unknown | unknown | unknown | `deg` | unknown |
| `image_scaling_factor` | unknown | unknown | unknown | unknown | unknown | `ratio` | unknown |
| `image_opacity` | unknown | unknown | unknown | unknown | unknown | `percent` | unknown |

`image_rotation` is captured as a free numeric, NOT as a four-position quarter-turn selector; the
v02.205 statement that image fills rotate "in 90-degree steps" is superseded -- quarter turns are a
convenience detent, not the parameter's domain. Image fills MUST also carry a render-time adjustment
block (exposure, contrast, saturation, temperature, tint, highlights, shadows) whose numeric
contracts are owned by the raster domain (14.4) and MUST NOT be redefined here.

**[STU-VEC-019]** The colour entry surface for any fill or stroke MUST accept HEX, RGB, HSL/HSB, CMYK
and Lab input under an explicit `StudioColorProfile` ([STU-DOC-003]), provide an eyedropper that
samples anywhere on the canvas including rendered images and gradients, support out-of-gamut
warnings, and support global swatches (edit-updates-all-uses) and spot swatches with tint and Lab
definitions.

**[STU-VEC-140] Colour entry surface contract.** The captured colour picker declares its channel
radio set and its hex field:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Option | Kind | Members / bounds | default |
|---|---|---|---|
| `active_channel` | enumeration | `channel.hue = 0`, `channel.saturation = 1`, `channel.brightness = 2`, `channel.red = 3`, `channel.green = 4`, `channel.blue = 5` | index `0` |
| `hex` | string | six hex digits | `FFFFFF` |
| `web_safe_only` | boolean | -- | `false` |

The captured member ordering pairs H/S/B with R/G/B on the same six-position selector; Studio MUST
preserve the six positions and their integer values so a model can address a channel numerically.

**[STU-VEC-141] Spot and swatch library contract.** Studio MUST ship a swatch/colour system whose
captured shipped inventory is: 118 swatch library files holding 3,155 named swatch entries plus
10,011 colour-book colours across 14 binary colour books and 6 legacy XML colour books, and 659
gradient entries and 382 pattern entries carried inside those same libraries. The largest single
book carries 2,104 colours. Observed swatch kinds across the parsed libraries are `process` (1,204),
`process_cmyk` (57), `process_gray` (1), `spot` (31), `registration` (53), `gradient` (945),
`pattern` (597) and 1,866 entries whose kind marker was absent. Studio's normative requirement is
NOT to ship those specific colour books -- their contents are third-party licensed data -- but to
implement the CONTRACT they demonstrate: a swatch library is a named, importable, exportable
container of typed swatch entries; a colour book is a read-only container of named spot colours
carrying raw component values plus a declared colour space; a swatch group is an ordered named
subset of a library; and a swatch entry's kind is one of the captured kinds above and MUST be
carried explicitly rather than inferred from the value shape. Colour-book components are captured as
RAW BYTES that were not scaled to a colour space; an importer MUST therefore require an explicit
space declaration and MUST NOT guess.

**[STU-VEC-051]** A gradient or pattern fill MUST be storable as a typed swatch in the
`StudioSwatch`/`StudioStyleRegistry` surface and reusable across objects and documents; editing a
shared gradient/pattern swatch MUST update every object referencing it, and an object MUST be able
to break the link to hold a local copy. Freeform and mesh gradients MUST retain per-point editability
when stored and re-applied.

**[STU-VEC-052]** Selection-wide colour editing MUST be supported: a mixed selection MUST enumerate
every distinct colour/gradient/pattern/style in use and allow swapping each across all uses in one
edit, and the [STU-VEC-125] attribute query MUST be able to gather objects by any single paint
attribute. These operations MUST route through the standard command lifecycle so bulk recolour edits
are auditable and undoable as one history entry.

**[STU-VEC-142] Colour-harmony adjustment contract.** The captured harmony/recolour surface carries
four global adjustment sliders that Studio MUST expose as a typed command, because bulk recolour is
otherwise not model-reproducible:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `saturation_shift` | unknown | unknown | -100 | 100 | 0 | `percent` | preference-controlled: `percentage_precision` |
| `brightness_shift` | unknown | unknown | -100 | 100 | 0 | `percent` | preference-controlled: `percentage_precision` |
| `temperature_shift` | unknown | unknown | -100 | 100 | 0 | `percent` | preference-controlled: `percentage_precision` |
| `luminosity_shift` | unknown | unknown | -100 | 100 | 0 | `percent` | preference-controlled: `percentage_precision` |

A separate captured brightness/saturation control declares `soft_min = 0`, `soft_max = 100`,
`precision = 0`; per [STU-VEC-105] both are recorded and the four-slider signed form above is the
normative Studio surface. Palette generation, harmony rules and colour reduction are owned by 14.8;
the vector domain consumes them and MUST NOT fork a parallel colour model.

---

## 14.5.6 Strokes

**[STU-VEC-020]** A vector object's stroke MUST be expressed as an ordered stack of stroke entries
paralleling fills ([STU-VEC-015]); each stroke entry MUST accept multiple stacked paint fills with
per-fill opacity and blend behaviour, and MUST carry the attributes defined in [STU-VEC-143]
through [STU-VEC-148]. No captured attribute is dropped in dedup.

**[STU-VEC-143] Stroke geometry enumerations.** All three carry captured integer values that Studio
MUST preserve at the interchange boundary. Two captures agree on the member sets; the second
contributes no integers, so the integers below are the normative wire values.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Enumeration | Members |
|---|---|
| `stroke_cap` | `cap.butt = 1`, `cap.round = 2`, `cap.projecting = 3` |
| `stroke_join` | `join.miter = 1`, `join.round = 2`, `join.bevel = 3` |
| `stroke_align` | `align.center` (token `CENTER`), `align.inside` (token `INSIDE`), `align.outside` (token `OUTSIDE`); no integers captured |

`stroke_cap` and `stroke_join` are settable at object level AND per network vertex ([STU-VEC-108]);
an object-level read over mixed per-vertex values MUST return the `mixed` sentinel of [STU-VEC-109].

**[STU-VEC-144] Stroke weight contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `stroke_weight` | unknown | unknown | unknown | unknown | unknown | `document_unit` | preference-controlled: `stroke_precision` |
| `miter_limit` | 1 | 500 | unknown | unknown | unknown | `ratio` | preference-controlled: `stroke_precision` |
| `stroke_top_weight` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `stroke_bottom_weight` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `stroke_left_weight` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `stroke_right_weight` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |

`miter_limit`'s hard range 1..500 is a declared bound from the layout-suite capture and is normative
for Studio. The four per-side weights are captured on the collaborative-suite frame model; a stroke
entry MUST carry either one uniform weight or the four per-side weights, and a read of `stroke_weight`
over mixed per-side values MUST return the `mixed` sentinel rather than an average. `strokes_included_in_layout`
is a captured boolean governing whether stroke weight contributes to the object's layout bounds;
Studio MUST carry it and MUST NOT assume either behaviour.

**[STU-VEC-145] Dash contract.** A dash pattern is an ordered list of alternating dash and gap
lengths. The captured stroke panel exposes exactly THREE dash/gap pairs (six numeric fields), each
with `precision` preference-controlled by `stroke_precision`; the collaborative-suite capture
models the same thing as an unbounded array of numbers. Per [STU-VEC-105] Studio stores an
UNBOUNDED array and MUST present at least three pairs in the default control. `dashed_line` is a
captured boolean gate. Additional captured dash controls that Studio MUST carry: `dash_cap` (drawn
from [STU-VEC-143]), a corner-adjustment enumeration with captured members `adjust.none`,
`adjust.dashes`, `adjust.gaps`, `adjust.dashes_and_gaps`, and a `gap_colour` plus `gap_tint` pair
so a dash gap can be painted rather than transparent.

**[STU-VEC-146] Arrowhead catalogue.** SUPERSEDES the six informal marker names of
v02.205 [STU-VEC-020]. Two captures contribute distinct catalogues and Studio's normative
set is their union with captured tokens preserved:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Capture | Members |
|---|---|
| Layout-suite arrowhead enumeration (12) | `arrow.none`, `arrow.simple`, `arrow.simple_wide`, `arrow.triangle`, `arrow.triangle_wide`, `arrow.barbed`, `arrow.curved`, `arrow.circle`, `arrow.circle_solid`, `arrow.square`, `arrow.square_solid`, `arrow.bar` |
| Collaborative-suite stroke-cap-as-marker enumeration (8) | `NONE`, `ROUND`, `SQUARE`, `ARROW_LINES`, `ARROW_EQUILATERAL`, `TRIANGLE_FILLED`, `DIAMOND_FILLED`, `CIRCLE_FILLED` |
| Connector-specific extension (12) | adds the six entity-relationship terminators `ERD_ONE`, `ERD_MANY`, `ERD_ONE_OR_MORE`, `ERD_ZERO_OR_MORE`, `ERD_ZERO_OR_ONE`, `ERD_EXACTLY_ONE` |

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `arrow_start_scale` | 1 | 1000 | unknown | unknown | unknown | `percent` | unknown |
| `arrow_end_scale` | 1 | 1000 | unknown | unknown | unknown | `percent` | unknown |

`arrow_alignment` is the captured two-member enumeration `{arrow_align.inside_path,
arrow_align.outside_path}` governing whether the marker tip sits at the path end or beyond it. A
third capture, from the live-effect libraries, records an add-arrowheads effect whose `head_arrow`
and `tail_arrow` are integer indices into a marker catalogue with `observed` values 0, 15 and 16 and
a `scale` `observed` at 100.0; per [STU-VEC-106] those are not bounds. Studio MUST expose start and
end markers as independent fields with independent scales and a swap command.

**[STU-VEC-147] Variable-width profile contract.** A width profile is an ordered list of width
points; each point carries `position` (0..1 along the path, unit `ratio`) and `width` (unit
`document_unit`). Both fields' hard and soft bounds and precisions are UNKNOWN in the capture and
MUST NOT be invented. A profile MUST support independent incoming and outgoing widths per point
([STU-VEC-122]), a flip-along and a flip-across command, and MUST be storable as a named reusable
profile in the `StudioStyleRegistry`.

**[STU-VEC-021]** Variable-width profiles MUST be storable as reusable named profiles in the
`StudioStyleRegistry`, and outlining a variable-width or brush stroke ([STU-VEC-006], [STU-VEC-130])
MUST produce filled geometry that matches the rendered stroke including its width variation.

**[STU-VEC-148] Stroke offset construct.** The captured live-effect libraries carry a stroke-offset
effect distinct from offset path: it shifts a stroke to one side of its path and carries a single
two-state control `stroke_offset_inside` (boolean) whose two display states are captured as `Inside`
and `Outside` across 212 shipped instances. Studio MUST model this as the `stroke_align` enumeration
of [STU-VEC-143] rather than as a separate effect, and MUST record that an importer meeting the
two-state form maps `true` to `align.inside` and `false` to `align.outside`. This clause exists so
the mapping is specified rather than discovered.

**[STU-VEC-062]** Caps/joins/arrowhead configuration MUST be defined for both simple open-path endpoints
and for closed or branching (network) endpoints; where a network anchor terminates three or more
edges, Studio MUST provide advanced endpoint controls resolving cap/join rendering per incident
edge, and MUST NOT leave branching-endpoint rendering undefined. The per-vertex cap and join fields
of [STU-VEC-108] are the storage for that resolution.

**[STU-VEC-063]** On-canvas stroke and gradient editing MUST be exposed through a live annotator
(draggable weight, dash, arrowhead, gradient-stop and gradient-geometry handles) whose every
manipulation maps to a typed value edit, so the same change is reproducible via the model command
surface ([STU-VEC-047]). The annotator MUST be toggleable/hideable without losing the underlying
values.

**[STU-VEC-064]** Studio MUST provide a directional-transparency (opacity-gradient) authoring
surface that applies a gradient in the alpha channel of an object independently of its colour
fill, editable on canvas like a colour gradient ([STU-VEC-016]); this MUST reduce to a gradient on
the object's opacity within the shared appearance model, not a separate object type. Its stops use
the [STU-VEC-136] contract, including the 13..87 midpoint hard bound.

**[STU-VEC-149] Dynamic (hand-drawn) stroke construct.** A non-destructive hand-drawn wobble applied
to a stroke MUST be available with independent frequency, wiggle and smoothing parameters. NO
bounds, defaults, units or precisions for these three were recovered by the green room; all three
parameters are declared a SPEC GAP and MUST NOT be shipped with invented ranges. The closest captured
relative is the scribble-fill construct of [STU-VEC-159], whose parameters ARE captured and which
MUST NOT be substituted for this one.

---

## 14.5.7 Appearance Model, Graphic Styles, and Effects

**[STU-VEC-022]** Studio MUST provide a per-object appearance stack: an object, group or layer MUST
carry an ordered list of appearance rows composed of fills ([STU-VEC-015]), strokes ([STU-VEC-020]),
opacity/blend settings and effect entries, where each row is independently toggleable, duplicable,
reorderable (reorder changes render order) and deletable. A single object MUST support multiple
fills and multiple strokes, each with its own paint, opacity, blend mode and effects. This is the
same appearance surface used across Studio domains and MUST NOT be a vector-only reimplementation.

**[STU-VEC-023]** Live effects MUST be attachable at any of these scopes and MUST be carried in the
shared `StudioEffectStack` primitive (14.9, 14.23): the whole object; a group/layer target ring
affecting all children collectively; and a single fill or stroke row within the appearance stack.
Effects MUST remain non-destructive and re-editable until an explicit expand-appearance command
([STU-VEC-054]) bakes them into concrete geometry/raster.

**[STU-VEC-151] Canonical blend-mode set.** SUPERSEDES two v02.205
statements: [STU-VEC-025]'s reference to "the single canonical
`StudioBlendMode` set" without enumerating it, and 14.6's [STU-LAY-032] claim
that blend modes are "the standard sixteen". Three captures contribute and
they DO NOT AGREE on the closed set.

Captured sixteen-member set with integer values, present identically in two independent
applications, and normative as the Studio interchange values:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Value | | Studio member | Value |
|---|---|---|---|---|
| `blend.normal` | `0` | | `blend.darken` | `8` |
| `blend.multiply` | `1` | | `blend.lighten` | `9` |
| `blend.screen` | `2` | | `blend.difference` | `10` |
| `blend.overlay` | `3` | | `blend.exclusion` | `11` |
| `blend.soft_light` | `4` | | `blend.hue` | `12` |
| `blend.hard_light` | `5` | | `blend.saturation` | `13` |
| `blend.color_dodge` | `6` | | `blend.color` | `14` |
| `blend.color_burn` | `7` | | `blend.luminosity` | `15` |

The third capture declares NINETEEN members: the sixteen above plus `blend.linear_burn`,
`blend.linear_dodge` and `blend.pass_through`. Per [STU-VEC-105] the normative Studio set is the
NINETEEN; the three additional members carry token values only (`LINEAR_BURN`, `LINEAR_DODGE`,
`PASS_THROUGH`) and have UNKNOWN integer values, so an interchange writer targeting the
sixteen-value integer form MUST map them explicitly and MUST fail closed rather than emit an invalid
integer. `blend.pass_through` is legal ONLY on a group, frame or layer, never on a leaf object; it is
the required default for groups and frames per [STU-VEC-025], which is why the "sixteen" formulation
was self-contradictory and is superseded here.

**[STU-VEC-025]** Opacity and blend mode MUST be settable at object, group, layer and individual
appearance-row level.

**[STU-VEC-153] Opacity, mask and knockout contract.** The captured transparency surface declares
exactly these controls; Studio MUST expose all of them as typed, model-settable state:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Option | Kind | Bounds / members | default |
|---|---|---|---|
| `opacity` | numeric | hard 0..100, soft unknown, unit `percent`, precision `0` | `100` |
| `blend_mode` | enumeration | [STU-VEC-151] | `blend.normal` for leaves, `blend.pass_through` for groups/frames |
| `mask_clip` | boolean | -- | `false` |
| `invert_mask` | boolean | -- | `false` |
| `isolate_blending` | boolean | -- | `false` |
| `knockout_group` | boolean | -- | `false` |
| `opacity_and_mask_define_knockout_shape` | boolean | -- | `false` |

A knockout state that must distinguish "explicitly off" from "inherited" MUST use the captured
four-member enumeration `{knockout.unknown = -1, knockout.disabled = 0, knockout.enabled = 1,
knockout.inherited = 2}` rather than a two-state boolean, because inheritance is a distinct third
state and collapsing it loses information.

**[STU-VEC-053]** Studio MUST support sibling/clip masking in addition to opacity masks: a vector object
marked use-as-mask MUST clip the objects above it within its container to its region
non-destructively and reversibly, with a selectable mask mode. The captured mask-mode enumeration is
`{mask.alpha = ALPHA, mask.vector = VECTOR, mask.luminance = LUMINANCE}` and Studio MUST use those
tokens. Masking MUST be a shared Studio capability ([STU-DOC-004]) using the canonical `StudioMask`
primitive, not a vector-only reimplementation.

**[STU-VEC-024]** Graphic styles MUST be named, reusable appearance presets stored in the
`StudioStyleRegistry`. Studio MUST support: apply a style to a selection; additive-merge of a style
onto an object's existing appearance; break-link to hold a local copy; redefine-from-selection,
updating all linked users; shared style libraries across documents; a "new art inherits current
appearance vs. basic appearance" toggle; and clear-appearance and reduce-to-basic-appearance
commands. Effect stacks MUST also be publishable as named effect styles reusable like colour and
type styles.

**[STU-VEC-161] Graphic-style library contract.** The captured shipped inventory is 12 graphic-style
library files carrying 314 primary style entries (326 including built-ins), and those same files
carry 224 gradients, 24 patterns, 49 swatches, 5 brushes and 12 embedded filter definitions as
SUPPORTING definitions that the styles reference. The normative contract this demonstrates and that
Studio MUST implement: a graphic-style library is a self-contained document that carries both the
style records AND every paint, gradient, pattern, brush and effect definition they reference;
importing one style MUST import its transitive dependency set; and a library MUST declare which of
its entries are primary (offered in the picker) and which are supporting (referenced only). A style
import that leaves a dangling reference is a validation failure, not a warning.

**[STU-VEC-154] Live-effect registry contract.** An effect entry in a `StudioEffectStack` MUST carry
a stable effect identifier, a parameter map keyed by stable parameter identifiers, and a version
stamp. The captured effect surface is organised as a nine-group menu index carrying the following
Studio-native operation families; the group names below are the Studio names and each family's
parameter contract is the clause named:

*Derivation: catalogue table, splits per row; yields 9 microtasks, one per live-effect family. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio effect family | Members | Parameter contract |
|---|---|---|
| Colour adjust | adjust colour balance; blend front-to-back; blend horizontally; blend vertically; convert to process; convert to greyscale; invert; overprint black; saturate | 14.8 (colour domain) |
| Convert to shape | rectangle; rounded rectangle; ellipse | [STU-VEC-158] |
| Create | fill-and-stroke for clipping mask; object mosaic | [STU-VEC-158] |
| Trim marks | create trim marks | 14.6 prepress |
| Distort | free distort; pucker and bloat; roughen; simplify; tweak; twist; zig zag | [STU-VEC-155], [STU-VEC-114] |
| Path | offset path | [STU-VEC-113] |
| Geometry | the thirteen operations of [STU-VEC-130] applied as live effects | [STU-VEC-130] |
| Stylize | add arrowheads; drop shadow; inner glow; outer glow; round corners | [STU-VEC-156] |
| Warp | fifteen named warp styles plus the envelope make/edit/release/expand/reset commands | [STU-VEC-168], [STU-VEC-169] |

The captured registry additionally records that effects are STORED as `(value, type, key)` triples
in the document, with types drawn from `{Int, Real, Bool, String}`. Studio MUST store effect
parameters as a typed map with exactly that type vocabulary so a model can read and write an effect
parameter without knowing the effect, and MUST reject an untyped parameter write.

**[STU-VEC-155] Distortion-effect parameter contracts.**

Roughen -- captured with BOTH a slider range and a separate edit range on its size parameter, the
canonical demonstration of [STU-VEC-100]:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `roughen_size_absolute` | 0 | 7200 | 0 | 100 | 1 | `document_unit` | 2 |
| `roughen_size_relative` | unknown | unknown | 0 | 100 | 1 | `percent` | 2 |
| `roughen_detail` | unknown | unknown | 0 | 100 | 1 | `per_in` | 2 |

plus `roughen_mode` (`{mode.relative = false, mode.absolute = true}`, both captured with
`default = false`, meaning neither radio is pre-selected in the layout and Studio MUST choose
`mode.relative` explicitly and record that choice as a Studio decision) and `roughen_points`
(`{points.smooth, points.corner}`). Absolute size accepts 0..7200 `document_unit` while its slider
presents only 0..100: collapsing those two would silently forbid 98.6 percent of the legal range.
Serialized instances additionally carry `absoluteness` and `roundness` as 0/1 real flags, `asiz`
`observed` 0.3437..7.4191, `dtal` `observed` 4.0..57.1628, `size` `observed` 1.0..11.0.

Zig zag:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `zigzag_size` | unknown | unknown | unknown | unknown | unknown | `context` (selected by `zigzag_mode`) | 2 |
| `zigzag_ridges_per_segment` | unknown | unknown | unknown | unknown | unknown | `count` | 2 |

plus `zigzag_mode` (`{mode.relative, mode.absolute}`) and `zigzag_points` (`{points.smooth,
points.corner}`). Serialized instances carry `amount` `observed` 0.6877..10.7633, `relAmount`
`observed` 1.0..17.0, `ridges` `observed` 0.0..30.0.

Pucker and bloat:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `pucker_bloat_factor` | unknown | unknown | unknown | unknown | unknown | `percent` | 2 |

Negative values pucker, positive values bloat. Serialized instances `observed` -86.9995..-25.0.

Twist:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `twist_angle` | unknown | unknown | unknown | unknown | unknown | `deg` | unknown |

Serialized instances `observed` 29.9994..120.0.

Tweak -- moves anchors and control points by independent horizontal and vertical amounts:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `tweak_horizontal` | unknown | unknown | 0 | 100 | 10 | `context` (selected by `tweak_mode`) | 2 |
| `tweak_vertical` | unknown | unknown | 0 | 100 | 10 | `context` (selected by `tweak_mode`) | 2 |

plus `tweak_mode` (`{mode.relative = false, mode.absolute = true}`) and three independently
defaulted booleans: `modify_anchor_points` (`true`), `modify_in_control_points` (`true`),
`modify_out_control_points` (`true`). The capture records that the edit range for the two amounts is
declared but its reference was unresolved, so `hard_min`/`hard_max` are UNKNOWN and MUST NOT be set
equal to the slider range.

Free distort -- a four-corner projective map. Its parameters are eight source coordinates and eight
destination coordinates (`src0..src3` and `dst0..dst3`, each with a horizontal and vertical
component), all unit `document_unit`, all bounds and precisions UNKNOWN. This is a geometric mapping,
not a set of independent sliders, and MUST be exposed as a typed quad-to-quad correspondence so a
model can specify a perspective without dragging.

**[STU-VEC-156] Stylize-effect parameter contracts.**

Drop shadow -- ten captured parameter keys across 172 shipped instances:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `shadow_opacity` | unknown | unknown | unknown | unknown | unknown | `ratio` (0..1 in storage) | unknown |
| `shadow_offset_x` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `shadow_offset_y` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `shadow_blur` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `shadow_darkness` | unknown | unknown | unknown | unknown | unknown | `percent` | unknown |

plus `shadow_blend_mode` (from [STU-VEC-151]; captured default index `1` = `blend.multiply`),
`shadow_colour_source` (`{source.colour = Color, source.darkness = Darkness}`, captured default
`source.colour`), and a boolean recording whether the blur uses the shared raster blur path.
`observed` across the shipped set: opacity 0.31..1.0, offset_x -5.0..11.0, offset_y 0.5..11.0, blur
0.0..6.48, darkness 10.0..100.0. Note that `shadow_opacity` is stored as a 0..1 `ratio` while the
object-level opacity of [STU-VEC-153] is a 0..100 `percent`; the two MUST NOT share a converter.

Inner glow:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `inner_glow_opacity` | unknown | unknown | unknown | unknown | unknown | `ratio` | unknown |
| `inner_glow_blur` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |

plus `inner_glow_blend_mode` (default index `1`) and `inner_glow_origin`
(`{origin.center = Center, origin.edge = Edge}`). `observed`: opacity 0.27..1.0, blur 3.6437..29.0.

Outer glow:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `outer_glow_opacity` | unknown | unknown | unknown | unknown | unknown | `ratio` | unknown |
| `outer_glow_blur` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |

plus `outer_glow_blend_mode` (default index `1`). `observed`: opacity 0.3..1.0, blur 0.9109..10.0.

Feather (soft-edge mask):

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `feather_radius` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |

`observed` 1.0..30.0 across 170 shipped instances.

Round corners (as an EFFECT, distinct from the live corner spec of [STU-VEC-112], which is geometry):

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `round_corners_radius` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |

`observed` 1.368..20.0. Studio MUST keep the effect and the geometry corner distinct: the effect
applies to the rendered appearance and is baked by expand-appearance; the geometry corner changes
the path and is baked by expand-shape.

Every effect in this clause carries a `preview` boolean; captured defaults are `false` where
captured at all, and a preview toggle MUST NOT alter the committed result.

**[STU-VEC-157] Transform operation contract.** The transform-each command and the live transform
effect share one parameter set. This is the second canonical demonstration of [STU-VEC-100]: every
numeric row carries a slider range AND a separate, much wider edit range.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `scale_horizontal` | -4000 | 4000 | 0 | 200 | 1.0 | `percent` | 2 |
| `scale_vertical` | -4000 | 4000 | 0 | 200 | 1.0 | `percent` | 2 |
| `scale_horizontal_absolute` | 0 | 16384 | unknown | unknown | 100.0 | `document_unit` | 4 |
| `scale_vertical_absolute` | 0 | 16384 | unknown | unknown | 100.0 | `document_unit` | 4 |
| `move_horizontal` | -4000 | 4000 | -100 | 100 | 1.0 | `document_unit` | 4 |
| `move_vertical` | -4000 | 4000 | -100 | 100 | 1.0 | `document_unit` | 4 |
| `rotate` | unknown | unknown | unknown | unknown | 0 | `deg` | unknown |
| `copies` | unknown | unknown | unknown | unknown | 0 | `count` | 0 |

plus `scale_mode` (`{mode.relative = false, mode.absolute = true}`), `reference_point` (nine-position
anchor, captured as an integer 1..9 in serialized instances with `observed` values 1, 3, 4), and
these booleans with captured defaults: `transform_objects`, `transform_patterns`,
`scale_strokes_and_effects` (`false`), `scale_corners` (`false`), `reflect_x` (`false`),
`reflect_y` (`false`), `randomize` (`false`), `preview` (`false`). Serialized instances also carry
`rotate` in both degrees and radians; Studio MUST store ONE canonical angle unit (`deg`) and MUST
derive the other at the interchange boundary, never store both.

`scale_horizontal_absolute`'s 0..16384 edit range MUST still be validated against the [STU-VEC-115]
item bound of 0..16348 `pt` after unit conversion; the two are different numbers and the narrower
one wins.

**[STU-VEC-158] Shape-conversion and mosaic effect contracts.**

Convert-to-shape carries `target_shape` (`{shape.rectangle = Rectangle, shape.rounded_rectangle =
RoundedRectangle, shape.ellipse = Ellipse}`) and `sizing_mode` (`{sizing.absolute = Absolute,
sizing.relative = Relative}`, captured default `sizing.absolute`). The width/height/extra-width/
extra-height numerics that accompany it were NOT recovered; they are a declared spec gap.

Object mosaic converts a placed raster into a grid of vector tiles and carries: `constrain_axis`
(`{axis.width, axis.height}`), `tile_colour_mode` (`{mosaic.colour, mosaic.gray}`),
`resize_using_percentages` (boolean), `delete_source_raster` (boolean). Tile counts, tile spacing and
the resize numerics were NOT recovered; declared spec gap.

**[STU-VEC-159] Scribble-fill contract.** A procedural hand-drawn fill. Fully captured, and the
richest single effect contract in the vector capture:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `path_overlap` | unknown | unknown | -1000 | 1000 | 0 | `ruler_unit` | 2 |
| `path_overlap_variation` | unknown | unknown | 0 | 1000 | 5 | `ruler_unit` | 2 |
| `stroke_width` | unknown | unknown | 0.01 | 1000 | 3 | `ruler_unit` | 2 |
| `curviness` | unknown | unknown | 0 | 100 | 5 | `percent` | 2 |
| `curviness_variation` | unknown | unknown | 0 | 100 | 1 | `percent` | 2 |
| `spacing` | unknown | unknown | 0.01 | 1000 | 5 | `ruler_unit` | 2 |
| `spacing_variation` | unknown | unknown | 0 | 1000 | 0.5 | `ruler_unit` | 2 |
| `angle` | unknown | unknown | unknown | unknown | unknown | `deg` | unknown |

`observed` across 59 shipped instances: angle -144.0..148.0, edge overlap -9.0..2.0, overlap
variation 0.0..8.0, scribbliness 0.0..0.84, scribble variation 0.0..1.0, spacing 0.2..15.0, spacing
variation 0.0..18.0, stroke width 0.2..8.0. A named settings preset selector accompanies the
sliders; its member list was not recovered and is a declared spec gap.

**[STU-VEC-160] Raster-effect settings contract.** Effects that rasterise (blurs, glows, feathers,
shadows) render at a document-scoped resolution. The captured settings block declares:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `raster_effect_resolution` | 72.0 | 2400.0 | unknown | unknown | 300.0 | `dpi` | unknown |
| `raster_effect_padding` | unknown | unknown | unknown | unknown | 0.0 | `pt` | unknown |

plus `colour_model` (enumeration, captured default `default_colour_model`), `anti_aliasing`
(boolean, `false`), `create_clipping_mask` (boolean, `false`), `preserve_transparency` (boolean,
`false`), `convert_spot_colours_to_process` (boolean, `false`). Changing
`raster_effect_resolution` changes every rasterising effect's output, so it MUST be a
document-authority field carried in the EventLedger, not a view preference, and a model MUST be able
to read it before predicting an effect's result.

**[STU-VEC-054]** Expand-appearance MUST bake the current appearance stack (multiple fills/strokes,
per-row effects, brushes, live corners and live constructs) into concrete geometry and/or raster
layers that reproduce the rendered result, and MUST be the single explicit boundary between
non-destructive editing and destructive materialisation. Expansion MUST be one history entry and
MUST NOT occur implicitly during ordinary edits, save or export. The captured expand command carries
three independent scope booleans -- `expand_object`, `expand_fill`, `expand_stroke` -- plus a
gradient-handling selector `{gradient.to_mesh, gradient.to_specified_steps}` with a step count.
Studio MUST expose all five so an expand is predictable; a step-count bound was not recovered and is
a declared spec gap. The captured library data additionally records two gradient-preservation
policies (`keep_gradients_editable`, `convert_gradients`) and two blend-expansion policies
(`convert_blends`, `rasterize_blends`) that Studio MUST carry as export/expand options.

---

## 14.5.8 Brushes

**[STU-VEC-162] Brush primitive and kind enumeration.** [STU-VEC-026] required a single canonical
`StudioVectorBrush` with a discriminated `kind`. The captured shipped inventory confirms exactly five
kinds and no others, with these per-kind entry counts across 25 shipped brush libraries totalling
561 brush definitions:

*Derivation: catalogue table, splits per row; yields 5 microtasks, one per brush kind.*

| Studio brush kind | Captured kind token | Shipped entries | Behaviour |
|---|---|---|---|
| `brush.calligraphic` | `calligraphic` | 45 | Angled-nib stroke with angle, roundness and size. |
| `brush.scatter` | `scatter` | 58 | Distribute copies of source art along the path. |
| `brush.art` | `art` | 238 | Stretch source artwork along the path length. |
| `brush.pattern` | `pattern` | 206 | Tile side/corner/start/end tiles along the path. |
| `brush.bristle` | `bristle` | 14 | Simulate natural bristle painting. |

Two further brush behaviours are required by [STU-VEC-026] and carry NO captured parameter contract:
`brush.image` (raster-textured organic stroke along an editable vector spine) and `brush.custom`
(capture any single vector layer as a reusable brush). Both are declared SPEC GAPS: they MUST be
registrable against the same primitive, and their parameter sets MUST be raised rather than invented.
The captured libraries confirm image-brush and bristle-brush library files exist (10 and 14 entries
respectively) but their per-brush parameter strings were captured VERBATIM and NOT decoded into
fields, so no bound may be read from them.

**[STU-VEC-026]** Studio MUST provide vector brushes as a single canonical `StudioVectorBrush` primitive
with a discriminated `kind` field; a brush is applied either as a path stroke ([STU-VEC-020]) or via
the brush/blob tools ([STU-VEC-007]).

**[STU-VEC-150] Brush and tool dynamics enumeration.** Every brush and nib parameter that can vary
along a stroke carries a dynamics selector. The captured enumeration is seven members on the art and
pattern brushes and on the nib tools, and seven members with `random` substituted for
`width_points_profile` on the scatter brush. Studio MUST carry the UNION as one enumeration and MUST
declare per-parameter which members are legal:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio member | Captured value | Meaning |
|---|---|---|
| `dynamics.fixed` | `0` | Constant along the stroke. |
| `dynamics.width_profile` | `1` | Driven by the variable-width profile ([STU-VEC-147]). |
| `dynamics.random` | `1` on the scatter brush | Random within a min/max pair. Conflicts with `dynamics.width_profile` on value `1`; the two are per-brush-kind alternatives and an interchange writer MUST resolve by brush kind, never by value alone. |
| `dynamics.pressure` | `2` | Stylus pressure. |
| `dynamics.stylus_wheel` | `3` | Stylus wheel. |
| `dynamics.tilt` | `4` | Stylus tilt. |
| `dynamics.bearing` | `5` | Stylus bearing. |
| `dynamics.rotation` | `6` | Stylus barrel rotation. |

A parameter under any member other than `dynamics.fixed` carries TWO values (a minimum and a
maximum), not one. Studio MUST store both and MUST NOT collapse them when the selector is
`dynamics.fixed`; the captured layouts keep the second field alive and remember its value across
selector changes.

**[STU-VEC-163] Brush colorisation enumeration.** Identical across three captured brush dialogs and
therefore normative with these values:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value | Behaviour |
|---|---|---|
| `colorize.none` | `0` | Keep the brush art's own colours. |
| `colorize.tints` | `1` | Remap to tints of the stroke colour. |
| `colorize.tints_and_shades` | `2` | Remap to tints and shades of the stroke colour. |
| `colorize.hue_shift` | `3` | Shift the brush art's hues toward the stroke colour. |

**[STU-VEC-027]** Each brush kind MUST support the colorisation method of [STU-VEC-163] and per-path
stroke-option overrides that alter brush parameters on one applied stroke without editing the shared
brush definition. Brush definitions MUST be storable in the `StudioStyleRegistry` and shareable
across documents.

**[STU-VEC-152] Per-kind brush parameter contracts.**

Calligraphic nib -- also the nib of the blob brush ([STU-VEC-120]) and the vector eraser
([STU-VEC-121]):

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `nib_angle` | unknown | unknown | unknown | unknown | 0 | `deg` | 0 |
| `nib_roundness` | unknown | unknown | unknown | unknown | 100 | `percent` | 0 |
| `nib_size` | unknown | unknown | unknown | unknown | 0 | `ruler_unit` | 1 |

Each of the three carries an independent dynamics selector from [STU-VEC-150].

Scatter brush -- the clearest hard/soft split in the brush capture:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `scatter_size` | 1 | 10000 | 10 | 1000 | 100 | `percent` | 0 |
| `scatter_spacing` | 1 | 10000 | 10 | 1000 | 100 | `percent` | 0 |
| `scatter_offset` | -10000 | 10000 | -1000 | 1000 | 0 | `percent` | 0 |
| `scatter_rotation` | -360 | 360 | -180 | 180 | 0 | `deg` | 0 |

Each of the four carries a dynamics selector and, under any non-fixed member, a second bounding
value. `scatter_rotation_relative_to` is the captured two-member enumeration
`{rotation_relative.page = 0, rotation_relative.path = 1}`. A second capture of the same panel
declares the four parameters WITHOUT edit ranges; per [STU-VEC-105] the hard ranges above are
normative because they are the only captured hard ranges, and the sliderless variant contributes
only the confirmation that the defaults are 100/100/0/0.

Art brush:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `art_width` | unknown | unknown | 1 | 1000 | 1 | `percent` | 2 |
| `art_width_variation` | unknown | unknown | 1 | 1000 | 1 | `percent` | 2 |
| `art_guide_start` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |
| `art_guide_end` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |

plus `art_scale_mode` (`{scale.proportional = 0, scale.stretch_to_stroke_length = 1,
scale.stretch_between_guides = 2}`, captured default `scale.proportional`), `art_width_dynamics`
(from [STU-VEC-150]), `flip_along` (boolean, `false`), `flip_across` (boolean, `false`),
`colorization` (from [STU-VEC-163], default `colorize.none`). A second capture of the same dialog
declares `precision = 0` for the two width fields; per [STU-VEC-105] Studio carries `precision = 2`
(the wider) and MUST record that a two-decimal art-brush width is legal.

Pattern brush:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `pattern_brush_scale_min` | unknown | unknown | 1 | 1000 | 1 | `percent` | 0 |
| `pattern_brush_scale_max` | unknown | unknown | 1 | 1000 | 100 | `percent` | 0 |
| `pattern_brush_scale` (fixed form) | unknown | unknown | unknown | unknown | 100 | `percent` | 2 |

plus `pattern_brush_scale_dynamics` (from [STU-VEC-150]), `fit_mode`
(`{fit.stretch_to_fit = 1, fit.add_space_to_fit = 2, fit.approximate_path = 4}` -- note the captured
values are 1, 2, 4 and NOT 1, 2, 3), `flip_along` (`false`), `flip_across` (`false`),
`colorization` (from [STU-VEC-163]), `show_auto_generated_corner_tiles` (boolean). A pattern brush
carries five independent tile slots -- side, outer corner, inner corner, start and end -- and Studio
MUST model them as five named references, not as an ordered array.

Bristle brush -- the only brush whose every parameter carries both a slider range and a default:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `bristle_size` | unknown | unknown | unknown | unknown | 5 | `mm` | 2 |
| `bristle_length` | unknown | unknown | 25 | 300 | 150 | `percent` | 0 |
| `bristle_density` | unknown | unknown | 1 | 100 | 50 | `percent` | 0 |
| `bristle_thickness` | unknown | unknown | 1 | 100 | 50 | `percent` | 0 |
| `bristle_paint_opacity` | unknown | unknown | 1 | 100 | 50 | `percent` | 0 |
| `bristle_stiffness` | unknown | unknown | 1 | 100 | 50 | `percent` | 0 |

plus `bristle_shape` (an enumerated nib-shape popup whose member list was NOT recovered -- declared
spec gap). `bristle_size` is captured with an explicit millimetre unit token, NOT a ruler unit; a
converter that treats it as `ruler_unit` will be wrong on every non-metric document.

**[STU-VEC-164] Brush library contract.** The captured shipped inventory is 25 brush library
files holding 561 primary brush entries, organised into six named families (arrows, artistic,
borders, bristle, decorative, image, vector packs and stylus-pen sets), with per-file entry
counts from 7 to 56. Each library additionally carries a brush-manager ORDER list with the same
cardinality as its brush list. The normative contract: a brush library is an ordered, named
container; the order is authored data and MUST be preserved on import, not re-sorted; and a
library MAY carry supporting swatch and filter definitions its brushes reference, which MUST
import transitively per [STU-VEC-161].

---

## 14.5.9 Artboards, Frames, and Layout Touchpoints

**[STU-VEC-028]** Vector artwork MUST be placeable on one or more `StudioArtboard` containers within a
single `StudioDocument` ([STU-DOC-001]). Studio MUST support many artboards per document and
per-artboard ruler-origin selection (document-global origin or per-artboard reset). Frame
containers, constraints and auto-layout are owned by the layout domain (14.6) and design-system
domain (14.10); the vector domain MUST consume those primitives rather than fork them.

**[STU-VEC-165] Artboard contract.** The captured artboard object and its options dialog declare
exactly these fields; Studio MUST carry all of them as typed, model-settable state:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Bounds / members | default |
|---|---|---|---|
| `name` | string | -- | empty |
| `bounds` | rectangle in `pt` | governed by [STU-VEC-115] | -- |
| `ruler_origin` | point, relative to the artboard's lower-left corner | -- | artboard lower-left |
| `ruler_pixel_aspect_ratio` | numeric | hard 0.1..10.0, soft unknown, unit `ratio`, precision unknown | unknown |
| `show_center_mark` | boolean | -- | unknown |
| `show_cross_hairs` | boolean | -- | unknown |
| `show_safe_areas` | boolean | -- | unknown |
| `fade_region_outside_artboard` | boolean | -- | unknown |
| `update_while_dragging` | boolean | -- | unknown |
| `background_fill` | enumeration | `artboard_fill.white = 0`, `artboard_fill.black = 1`, `artboard_fill.transparent = 2`, `artboard_fill.custom = 4` | `artboard_fill.transparent` (the capture labels value 2 as the default) |
| `preset` | reference to a named size preset | -- | -- |

`ruler_pixel_aspect_ratio` is used only when the artboard's ruler unit is `px`; its hard range
0.1..10.0 is a declared bound and MUST be enforced. The `background_fill` enumeration's value `3`
is captured as absent -- the member list skips from 2 to 4 -- and Studio MUST NOT reuse value 3.

Ruler-origin scope is a three-member enumeration captured on the layout side and shared with the
vector domain: `{ruler_origin.spread, ruler_origin.page, ruler_origin.spine}`. Whether the ruler
coordinate system may be changed at all is itself a captured queryable predicate, and Studio MUST
expose it so a model can test before setting rather than fail after.

**[STU-VEC-029]** Studio MUST provide the vector-relevant alignment and measurement surfaces below:
align/distribute to selection, key object or artboard with numeric spacing values; ruler guides,
object-to-guide conversion and release-guide-back-to-object; live distance guides showing spacing
between the selection and its neighbours or the artboard; snap-to-perpendicular and snap-to-tangent
while drawing; pixel-snapping options for raster-targeted output; isolation mode with a breadcrumb
to exit levels; and distribute/orient objects along an arbitrary path spine.

**[STU-VEC-166] Align target enumeration and distribute contract.** The captured align surface
declares a three-member target selector that Studio MUST carry with these values:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value |
|---|---|
| `align_to.selection` | `0` |
| `align_to.key_object` | `1` |
| `align_to.artboard` | `2` |

Distribute-by-spacing takes an explicit numeric gap; its bounds, unit and precision were NOT
recovered and are a declared spec gap. A pixel-alignment boolean is captured on the symbol options
surface (`align_to_pixel_grid`, default `false`) and on the item model as a queryable per-item
`pixel_aligned` predicate; Studio MUST expose both -- the document-level intent and the per-item
fact -- because they answer different questions.

**[STU-VEC-057]** Studio MUST provide vector measurement/inspection surfaces usable by operator and
model: measure distance and angle between points, report enclosed region area in document units
([STU-VEC-117]), and enumerate document vector inventory (object counts, fonts-as-outlines,
linked/embedded placed images, spot colours, and pattern/gradient usage) for audit. Measurement
readouts MUST be typed values, not screen-only overlays.

**[STU-VEC-058]** Reusable vector symbol instances are owned by the design-system domain (14.10) via
`StudioComponent`/`StudioComponentInstance`; the vector domain MUST expose its geometry as valid
symbol source and MUST honour per-instance override/sync behaviour defined there, but MUST NOT fork
a parallel symbol model.

**[STU-VEC-167] Symbol source contract.** The captured symbol-options surface declares the fields a
vector object must carry to be valid symbol source, and Studio MUST carry them on the vector side
even though the component lifecycle lives in 14.10:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Members / bounds | default |
|---|---|---|---|
| `symbol_name` | string | -- | a generated placeholder |
| `symbol_kind` | enumeration | `symbol.dynamic = 2`, `symbol.static = 1` | `symbol.dynamic` (captured selected value `2`) |
| `enable_nine_slice_guides` | boolean | -- | `false` |
| `align_to_pixel_grid` | boolean | -- | `false` |
| `export_type` | enumeration | indexed popup, member list NOT recovered -- declared spec gap | index `0` |
| `registration_point` | enumeration | `reg.top_left = 1`, `reg.top_center = 2`, `reg.top_right = 3`, `reg.middle_left = 4`, `reg.center = 5`, `reg.middle_right = 6`, `reg.bottom_left = 7`, `reg.bottom_center = 8`, `reg.bottom_right = 9` | `reg.center` |
| `instance_name` | string, per instance | -- | empty |

A `symbol.dynamic` instance accepts per-instance appearance overrides while remaining linked; a
`symbol.static` instance does not. That distinction is a stored kind, not a mode, and MUST survive
round-trip.

---

## 14.5.10 Transforms and Distortions

**[STU-VEC-030]** Studio MUST provide the deduped transform and distortion set below. Each is a typed
operation available identically to operator and model. Live/envelope distortions MUST be
non-destructive constructs whose contents remain editable until explicitly expanded.

*Derivation: catalogue table, splits per row; yields 8 microtasks, one per transform or distortion construct. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio transform | Function (normative) | Parameter contract |
|---|---|---|
| Move / rotate / scale / reflect / shear | Affine transforms about a settable reference point, with per-object randomised transform-each | [STU-VEC-157] |
| Free transform | Combined on-canvas move/scale/rotate/shear with constrain and perspective/distort modifiers | [STU-VEC-157] |
| Free distort | Reshape by dragging four corner points of a distortion frame | [STU-VEC-155] |
| Perspective distort | Map artwork onto a perspective plane or mockup surface | [STU-VEC-172] |
| Envelope distort | Wrap artwork in an editable envelope from a warp preset, a custom mesh grid or a top object | [STU-VEC-168], [STU-VEC-169] |
| Warp group | Live perspective/quad/mesh warp wrapping children that stay editable | [STU-VEC-169] |
| Puppet warp | Pin-and-drag mesh deformation of a single object | [STU-VEC-172] |
| Liquify-family warps | Warp, twirl, pucker, bloat, scallop, crystallize, wrinkle brush distortions | [STU-VEC-170] |

**[STU-VEC-168] Warp-style enumeration and parameters.** The captured warp menu declares fifteen
named styles plus the six envelope lifecycle commands. Studio MUST carry all fifteen as members of
one enumeration:

`warp.arc`, `warp.arc_lower`, `warp.arc_upper`, `warp.arch`, `warp.bulge`, `warp.shell_lower`,
`warp.shell_upper`, `warp.flag`, `warp.wave`, `warp.fish`, `warp.rise`, `warp.fisheye`,
`warp.inflate`, `warp.squeeze`, `warp.twist`.

The serialized live-effect capture records the style as an integer with `observed` values 1, 2, 3,
5, 8, 9, 11 and 12, confirming an integer encoding exists but not its full mapping; the mapping from
Studio member to integer is UNKNOWN and MUST be raised as a spec gap rather than guessed.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `warp_bend` | unknown | unknown | -100 | 100 | 0 | `percent` | 2 |
| `warp_distortion_horizontal` | unknown | unknown | -100 | 100 | 0 | `percent` | 2 |
| `warp_distortion_vertical` | unknown | unknown | -100 | 100 | 0 | `percent` | 2 |

plus `warp_axis` (`{axis.horizontal, axis.vertical}`, captured default `axis.horizontal`) and
`preview` (boolean, `false`). Serialized instances store bend and the two distortions as `ratio`
values with `observed` ranges -0.42..0.84, -0.29..0.44 and 0.0..0.27 respectively, confirming that
the STORED form is a ratio while the AUTHORED form is a percentage; Studio MUST convert at the
interchange boundary and MUST NOT store both.

**[STU-VEC-169] Envelope option contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `envelope_fidelity` | unknown | unknown | 0 | 100 | 0 | `percent` | 2 |

plus `envelope_raster_mode` (`{envelope_raster.clipping_mask = 0,
envelope_raster.transparency = 1}`, captured default `envelope_raster.clipping_mask`),
`anti_alias` (boolean, `false`), `distort_appearance` (boolean, `false`),
`distort_linear_gradient_fills` (boolean, `false`), `distort_pattern_fills` (boolean, `false`),
`preview` (boolean, `false`), and on the mesh-reset command `maintain_envelope_shape` (boolean,
default UNKNOWN). The mesh envelope's row and column counts are captured as a grid-creation dialog
whose numeric fields were NOT recovered; declared spec gap.

**[STU-VEC-055]** All distortion constructs that wrap editable content MUST provide an edit-contents
mode that lets the operator or model edit the underlying source geometry while the distortion
continues to apply live, and an explicit expand that bakes the distorted result. Distorting a
gradient or pattern fill MUST honour the two independent captured booleans of [STU-VEC-169]
(`distort_linear_gradient_fills`, `distort_pattern_fills`); they are SEPARATE options and MUST NOT
be merged into one "distort fills" flag.

**[STU-VEC-170] Liquify-family contract.** The captured liquify surface declares one shared option
block across the whole tool family:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `twirl_rate` | unknown | unknown | -180 | 180 | -180.0 | `deg` | preference-controlled: `angle_precision` |
| `detail` | unknown | unknown | 1 | 10 | 1 | `count` | preference-controlled: `value_precision` |
| `simplify` | unknown | unknown | 0.2 | 100 | 0.2 | `count` | preference-controlled: `value_precision` |

plus `use_pressure_pen` (boolean, `false`), `detail_enabled` (boolean, `false`),
`simplify_enabled` (boolean, `false`), `brush_affects_anchor_points` (boolean, default UNKNOWN),
`brush_affects_in_tangent_handles` (boolean, default UNKNOWN),
`brush_affects_out_tangent_handles` (boolean, default UNKNOWN), `show_brush_size` (boolean,
`false`). `detail` and `simplify` are GATED by their own enable booleans: an implementer MUST NOT
apply them when the gate is off, and a model MUST set the gate explicitly. Brush width, height and
angle for the liquify nib were NOT recovered and are a declared spec gap.

**[STU-VEC-172] Perspective and puppet-warp contract.** Both constructs are IN SCOPE and both are
declared SPEC GAPS. The captured perspective surface exposes a grid-options dialog, a plane-offset
field and a scale field, and the captured particle/symbolism surface exposes a method selector
`{method.average = 0, method.user_defined = 1, method.random = 2}` with an intensity dynamics
selector drawn from [STU-VEC-150]; no numeric bounds, defaults or units were recovered for either.
Studio MUST implement the constructs and MUST raise their parameter contracts as spec gaps rather
than ship invented ranges. This clause exists so the constructs are not silently dropped from the
domain.

**[STU-VEC-031]** Studio MUST provide the procedural repeat construct with `radial`, `grid` and `mirror`
modes, kept live so instance counts, spacing and symmetry stay editable, with make/release/options
and expand. Repeat instances MUST render from a single source definition and MUST NOT be
materialised as independent copies until expanded.

**[STU-VEC-171] Repeat-construct parameter contract.** Three captured configuration objects, one per
mode, each with real defaults:

Radial repeat:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `radial_instances` | unknown | unknown | unknown | unknown | 8 | `count` | 0 |
| `radial_radius` | unknown | unknown | unknown | unknown | -1.0 | `document_unit` | unknown |

`radial_radius` has the captured default `-1.0`, which is a SENTINEL meaning "derive the radius from
the source artwork", not a negative length. Studio MUST carry the sentinel explicitly as a nullable
radius rather than store -1, and MUST document that a literal negative radius is invalid.
`reverse_overlap` is a boolean with captured default `false`.

Grid repeat:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `grid_horizontal_spacing` | unknown | unknown | unknown | unknown | 10.0 | `document_unit` | unknown |
| `grid_vertical_spacing` | unknown | unknown | unknown | unknown | 10.0 | `document_unit` | unknown |

plus three independent flip/shift selectors -- `grid_pattern_type`, `grid_row_flip_type`,
`grid_column_flip_type` -- each drawn from the captured four-member enumeration
`{flip.none = 0, flip.horizontal = 1, flip.vertical = 2, flip.horizontal_and_vertical = 3}`, all
three with captured default `flip.none`. The capture also records the field-update bitmask values
`horizontal_spacing = 4`, `vertical_spacing = 8`, `all = 63`; Studio MUST NOT expose a bitmask on
the model surface and MUST expose named fields instead, but an interchange writer MUST preserve
those values.

Mirror (symmetry) repeat:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `symmetry_axis_rotation` | unknown | unknown | unknown | unknown | 1.57 | `rad` | unknown |

The captured default is in RADIANS (1.57, a quarter turn measured anticlockwise from the x axis) even
though every authoring angle elsewhere in this sub-section is in degrees. Studio MUST store `deg`
per [STU-VEC-157] and MUST convert at the interchange boundary; a converter that treats 1.57 as
degrees produces a visibly wrong default and is a conformance failure. The corresponding update
bitmask values are `axis_rotation = 2`, `all = 7`.

**[STU-VEC-056]** Symmetry/mirror drawing MUST be supported for vector authoring (single- or
multi-axis), reflecting live strokes across the configured axes; the mirrored result MUST be
materialisable to ordinary editable geometry on demand and MUST NOT depend on interactive-only
session state that a model cannot query.

---

## 14.5.11 Procedural Constructs

**[STU-VEC-032]** Blend: Studio MUST support blending between two or more objects to generate live
intermediate steps, with spacing modes, orientation control (align to page or to the spine), and an
editable, replaceable and reversible spine plus reverse-front-to-back. Blends MUST be live
(re-editable and re-flowable along the spine) until explicitly expanded.

**[STU-VEC-173] Blend parameter contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `blend_easing_ramp` | unknown | unknown | unknown | unknown | unknown | `percent` | 2 |
| `blend_colour_shift` | unknown | unknown | unknown | unknown | unknown | `percent` | 2 |

plus `blend_spacing_mode` (an indexed popup captured with default index `0`; its member list --
smooth colour, specified steps, specified distance -- is REQUIRED by [STU-VEC-032] but its captured
integer values were NOT recovered, so the values are UNKNOWN), `blend_easing_type` (an indexed popup
whose members were NOT recovered -- declared spec gap), and `preview` (boolean, `false`). The step
count and distance that accompany the two non-automatic spacing modes were NOT recovered; declared
spec gap. The two captured easing sliders ARE new relative to v02.205 [STU-VEC-032], which described
only the three spacing modes: a blend carries an easing ramp and a colour-acceleration shift, and
omitting them makes captured blends unreproducible.

**[STU-VEC-033]** Live Paint: Studio MUST support a live-paint construct that treats overlapping paths
as a surface of fillable faces and paintable edges, filling/stroking regions by click, with
gap-detection options that close paint leaks by gap size, plus make/merge/release/expand. Live-paint
faces MUST update automatically as the underlying paths are edited.

**[STU-VEC-174] Live-paint gap and selection contract.**

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Option | Kind | Members / bounds | default |
|---|---|---|---|
| `gap_detection` | boolean | -- | `true` (captured value `1`) |
| `paint_stops_at` | enumeration | indexed gap-size popup; member list NOT recovered | index `3` |
| `gap_preview_colour` | enumeration | indexed colour popup; member list NOT recovered | index `9` |
| `preview` | boolean | -- | `false` |
| `select_fills` | boolean | -- | `true` |
| `select_strokes` | boolean | -- | `true` |
| `cursor_swatch_preview` | boolean | -- | `true` |
| `highlight_face_under_cursor` | boolean | -- | `true` |
| `highlight_colour` | enumeration | indexed colour popup; member list NOT recovered | index `0` |

The two colour popups and the gap-size popup carry captured DEFAULT INDICES but not their member
lists. Studio MUST preserve the default indices, MUST implement the controls, and MUST raise the
member lists as spec gaps. The merge-tool variant of the same gap machinery carries a fully recovered
gap-length enumeration and is specified in [STU-VEC-123]; the two MUST resolve to ONE gap-detection
implementation per [STU-SECTION-003], with `gap.small/medium/large/custom` as the normative member
set and the live-paint popup mapping onto it.

**[STU-VEC-034]** Gradient Mesh: Studio MUST support gradient-mesh objects (`StudioGradient` kind
`gradient.mesh`) -- a grid of mesh points and lines interpolating colour and per-point opacity across
a shape -- with add/remove mesh point and line editing and conversion from an existing gradient or
shape. Mesh geometry MUST be editable with the node tooling ([STU-VEC-007]).

**[STU-VEC-175] Mesh-creation contract.** Creating a mesh from a filled object carries a captured
three-member appearance selector that Studio MUST reproduce with these values:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Captured value | Result |
|---|---|---|
| `mesh_appearance.flat` | `0` | Uniform colour across the mesh; the source fill is applied to every mesh point. |
| `mesh_appearance.to_center` | `1` | Colour concentrates toward the mesh centre. |
| `mesh_appearance.to_edge` | `2` | Colour concentrates toward the mesh edges. |

plus a `preview` boolean. The mesh row and column counts and the highlight percentage that accompany
the appearance selector were NOT recovered; declared spec gap. A mesh object carries the same
per-item opacity and dimension bounds as any other art item ([STU-VEC-115]).

**[STU-VEC-035]** Image Trace: Studio MUST provide a deterministic native raster-to-vector trace
primitive converting a placed raster into editable `StudioVectorPath`/`StudioVectorNetwork`
geometry, plus make / make-and-expand / release / expand. This native trace MUST NOT require any
provider or network; a generative vectorise is a separate optional lane ([STU-VEC-039]).

**[STU-VEC-176] Image-trace parameter contract.** SUPERSEDES the nine-option list of
v02.205 [STU-VEC-035], which was materially incomplete (it omitted the gradient-smoothing,
live-shape fitting, live-text, transparency, background-removal, colour-ignore,
auto-grouping and anchor simplification controls) and which mislabelled two distinct
captured controls as one "gray count".

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `trace_colours_automatic` | unknown | unknown | 0.0 | 100.0 | 100.0 | `percent` | unknown |
| `trace_colours_limited` | unknown | unknown | 2 | 30 | 30 | `count` | unknown |
| `trace_colours_full` | unknown | unknown | 0.0 | 100.0 | 100.0 | `percent` | 1 |
| `trace_grays` | unknown | unknown | 0.0 | 100.0 | unknown | `percent` | unknown |
| `trace_gray_levels` | 1 | 256 | unknown | unknown | unknown | `count` | 0 |
| `trace_threshold` | 1 | 256 | 1 | 255 | unknown | `count` | 0 |
| `trace_paths` | unknown | unknown | 1 | 100 | 50 | `percent` | 1 |
| `trace_corners` | unknown | unknown | 0 | 100 | 50 | `percent` | 1 |
| `trace_noise` | unknown | unknown | 1 | 100 | 50 | `px` | unknown |
| `trace_anchors` (simplify strength) | unknown | unknown | 0 | 100 | 90.0 | `percent` | 0 |
| `trace_gradient_smooth` | unknown | unknown | 1 | 100 | 100 | `percent` | 1 |
| `trace_max_stroke_weight` | unknown | unknown | unknown | unknown | unknown | `document_unit` | unknown |

The greyscale controls are TWO SEPARATE parameters and the v02.205 conflation of them is the
substantive error this clause corrects: `trace_grays` is a 0..100 percentage fidelity slider, while
`trace_gray_levels` is a declared 1..256 integer count of output grey levels. Likewise
`trace_threshold` carries a declared hard range of 1..256 in the type library and a soft slider of
1..255 in the dialog -- the two differ by one and MUST NOT be collapsed.

Enumerations:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Enumeration | Members |
|---|---|
| `trace_mode` | `trace_mode.colour = 0`, `trace_mode.grayscale = 1`, `trace_mode.black_and_white = 2` |
| `trace_colour_type` | `trace_colour.limited = 0`, `trace_colour.full = 1` |
| `trace_method` | `trace_method.abutting = 0`, `trace_method.overlapping = 1` |
| `trace_view` | `trace_view.result = 0`, `trace_view.result_with_outlines = 1`, `trace_view.outlines = 2`, `trace_view.outlines_with_source = 3`, `trace_view.source = 4` |

Booleans with captured defaults: `create_fills` (`true`), `create_strokes` (`false`),
`create_gradients` (default UNKNOWN), `fit_live_shapes` (`false`), `create_live_text` (`false`),
`snap_curves_to_lines` (`false`), `preserve_transparency` (`false`), `remove_background` (`false`),
`ignore_white` (`false`), `ignore_colour` (`false`), `auto_grouping` (`false`), `simplify_enabled`
(default UNKNOWN), `preview` (`false`), `auto_grouping_setting` (a second captured instance of the
auto-group control with the same default). String-valued fields: `palette_name` (a named swatch
library or the document library) and `colour_group_name` (a named colour group or "all"); both
default to the document library.

Constraints the capture states explicitly and which Studio MUST enforce: at least one of
`create_fills` and `create_strokes` MUST be true; `ignore_white` takes effect only when
`trace_method` is `trace_method.abutting` AND `trace_mode` is `trace_mode.black_and_white`;
`trace_colours_limited` applies only when `trace_colour_type` is `trace_colour.limited`;
`trace_gray_levels` applies only in `trace_mode.grayscale`; `trace_threshold` applies only in
`trace_mode.black_and_white`.

**[STU-VEC-036]** Intertwine: Studio MUST support an intertwine construct that makes selected
overlapping objects appear woven (one object's region passing over or under another)
non-destructively, with make/release/edit. The over/under assignment at each crossing MUST be a
stored, editable, model-readable value. No parameter contract for intertwine was recovered by the
green room; its options are a declared SPEC GAP.

**[STU-VEC-037]** Global Edit: Studio MUST support a global-edit mode that edits all similar objects
together (scoped by matching shape, size and appearance across artboards) and a select-same query
set built on the [STU-VEC-125] predicate set extended with graphic style, shape kind and
symbol/component instance. Global edits MUST propagate through the standard command/validation
lifecycle so model-authored global edits are auditable ([STU-ARC-005]).

**[STU-VEC-059]** Every procedural construct in this group (blend, live paint, gradient mesh, image
trace, intertwine, repeat) MUST persist its live parameters AND its source references as vector
authority ([STU-VEC-042]) so the construct survives save/load and round-trips without being silently
flattened, and MUST expose those parameters to the model command surface. A construct that can only
be authored interactively and cannot be inspected or re-parameterised by a model is non-conformant.

**[STU-VEC-177] Decorative generator constructs.** A lens-flare generator (rays and rings, each an
independent captured boolean, both captured default `true`) and a particle/symbolism spray family
(method selector `{method.average = 0, method.user_defined = 1, method.random = 2}`, per-channel
average/user-defined selectors for density, size, spin, screen, stain and style, plus
`proportional_resizing` `true`, `resizing_affects_density` `true`, `show_brush_size_and_intensity`
`true`) are in scope. Their numeric parameters were NOT recovered and are declared SPEC GAPS. Studio
MUST NOT ship either with invented ranges, and MUST NOT quietly drop them from the domain.

---

## 14.5.12 Text-on-Path Touchpoint

**[STU-VEC-038]** A `StudioVectorPath`/`StudioVectorNetwork` MUST be usable as a typographic baseline
(text on a path) and as a text-frame boundary (area type). The vector domain owns only the geometry;
the text run, shaping, path-text options and area-type behaviour are owned by the Typography engine
(14.7) via the `StudioTextStory`/`StudioTypeStyle` primitives. The vector domain MUST expose the path
as a stable typographic reference and MUST NOT reimplement text layout. Converting text to outlines
produces standard vector geometry under this sub-section.

**[STU-VEC-068]** Editing the geometry of a path bound to text (moving anchors, reshaping segments,
reversing direction [STU-VEC-043]) MUST reflow the bound text along the updated baseline without
detaching the text run, and deleting the path MUST follow the typography domain's detach/relink
contract (14.7) rather than silently discarding the text. The vector domain MUST NOT bake text to
outlines as a side effect of geometry editing.

**[STU-VEC-178] Create-outlines constraint set.** The captured constraint vocabulary names three
distinct failures of text-to-outlines that Studio MUST reproduce as separate typed errors: outlines
cannot be created from OVERSET text; outlines cannot be created from bad or unresolvable glyph data;
and a page item cannot be created on a locked layer. Each line of converted text becomes one closed
polygon object under the captured behaviour, so create-outlines on a multi-line story MUST produce a
group of per-line objects, not one object, and that grouping is normative because downstream
geometry operations depend on it.

**[STU-VEC-069]** Vector geometry authored under this sub-section MUST round-trip through the
interchange formats owned by 14.13 (SVG as the primary open vector interchange, plus PDF and the
source-suite vector formats) preserving anchors, handles, fill rules, corner specs and appearance
stacks where the target format allows, and degrading predictably where it does not. Import/export
fidelity, format matrices and lossy-mapping rules are specified in 14.13; the vector domain MUST
expose its primitives in a form interop can serialise without a private shadow model.

---

## 14.5.13 Provider / AI Lane (Adapter-Backed, Optional)

**[STU-VEC-039]** Studio's default vector pipeline is fully local and deterministic; generative
capabilities are an OPTIONAL adapter lane consistent with the local-first posture ([STU-OVR-002]).
The deterministic recolour and raster-to-vector trace primitives are NATIVE ([STU-VEC-176], and the
deterministic recolour primitive in 14.8); the generative variants below MUST be routed through
`StudioModelAdapter` (14.23) via existing Handshake model routing, MUST be clearly marked as
adapter-backed/optional in the UI and command surface, and MUST degrade cleanly to the native
primitive or to an explicit unavailable state when no adapter is configured. No generative feature is
a required build gate for the vector domain.

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Provider/AI capability | Native fallback (normative) | Lane |
|---|---|---|
| Text-to-vector (scenes, subjects, icons) | none (adapter-only); the result is ordinary editable vector output | adapter-backed / optional |
| Text-to-pattern | native pattern authoring ([STU-VEC-138]) | adapter-backed / optional |
| Generative shape fill | native fills, gradients, patterns, live paint | adapter-backed / optional |
| Generative recolour | native deterministic recolour and harmony palettes ([STU-VEC-142], 14.8) | adapter-backed / optional |
| Generative vectorise (raster to vector) | native image trace ([STU-VEC-176]) | adapter-backed / optional |
| Generative expand (vector or image outpaint) | none (adapter-only) | adapter-backed / optional |
| Sketch-to-vector | native image trace ([STU-VEC-176]) | adapter-backed / optional |

The captured surface confirms the adapter lane is a distinct set of dialogs carrying model selection,
output quantity, content type, reference-image selection and a credit/quota surface. Studio MUST NOT
reproduce a credit or quota surface; provider availability, credentials and offline-parity
classification follow the provider registry referenced in 14.14.

**[STU-VEC-040]** Every adapter-backed generative result MUST land as ordinary local `StudioDocument`
content subject to the same authority, history and export surfaces as hand-authored geometry, and
MUST carry a `KernelActor` attribution marking it model/adapter-authored ([STU-ARC-003]). A
generative result MUST pass the sandbox -> validation -> `PromotionGate` lifecycle before it changes
document authority ([STU-ARC-005]).

**[STU-VEC-060]** The native deterministic primitives and the generative adapter lane MUST be separable:
the vector domain MUST build, validate and ship with the adapter lane entirely absent, and the
operator/model command surface MUST clearly distinguish a deterministic native command from its
optional generative counterpart so intent is never ambiguous. A generative command MUST NOT shadow,
replace or silently reroute a native deterministic command.

**[STU-VEC-061]** Prompt text, style-reference selections and any source content sent to a generative
adapter MUST be treated as model/adapter input governed by Handshake model routing and the provider
registry (14.14); this sub-section does not authorise any implicit network egress. When
adapter-backed vectorise or recolour is unavailable, Studio MUST fall back to the native primitive
named in the [STU-VEC-039] table or surface an explicit unavailable state, never a silent no-op.

---

## 14.5.14 Cross-Cutting Obligations

**[STU-VEC-041]** Every vector tool, shape, geometry operation, fill/stroke attribute, appearance row,
brush, transform and procedural construct in this sub-section MUST satisfy the Studio cross-cutting
obligations, stated once here rather than per feature:

- Model visibility and steerability -- a stable command identifier and a typed contract for every
  capability (14.16).
- Quiet/headless operation -- no focus-stealing or foreground popups during model or background work
  (14.20).
- Dual-audience UserManual -- an in-product manual entry enabling a no-context model to operate the
  capability (14.22).
- GUI test hooks and visual capture -- a stable `author_id` test hook on every operator control and
  visual-capture coverage (14.16/14.22).

A vector capability is not complete until its typed command contract, its GUI surface with stable
test hooks and its UserManual entry all exist and its geometry round-trips through the
`VectorEngine` boundary.

**[STU-VEC-179] Engine boundary and determinism.** All geometry evaluation named in 14.5 --
tessellation, boolean composition, stroking, offsetting, brush instancing, envelope and warp
evaluation, trace, mesh interpolation -- MUST live behind the `VectorEngine` trait in the
`studio-engine` crate and MUST be deterministic: identical inputs (geometry, z-order, parameter
values, `precision` per [STU-VEC-131], and `raster_effect_resolution` per [STU-VEC-160]) MUST produce
byte-identical outputs on every host. `handshake_core` MUST NOT gain `wgpu`, WGSL or any GPU
dependency for this domain ([STU-ARC-002]); GPU acceleration of these operations is permitted only
inside `studio-engine` and only where it preserves byte-identical results, otherwise the CPU path is
authoritative. Determinism is a promotion-equivalence requirement, not a performance preference: a
model-authored edit and an operator-authored edit must agree exactly for the `PromotionGate` to be
meaningful.

**[STU-VEC-180] Validation descriptor set.** The `StudioValidationDescriptor` catalog (14.24)
MUST carry, for this domain, at minimum one descriptor per numeric parameter bound-set declared
in 14.5 (asserting hard-bound rejection and soft-bound acceptance as SEPARATE assertions), one
per enumeration (asserting every member round-trips by value, not by name), one per typed error
in [STU-VEC-133] and [STU-VEC-178], and one per declared SPEC GAP asserting that the gap is still
open so a later capture can close it deliberately. A descriptor that asserts only that a command
succeeded is insufficient.

**[STU-VEC-073]** Vector history/undo MUST use the shared `StudioHistoryEntry` surface (14.19): each
discrete vector command -- a geometry edit, a geometry operation, an expand, a flatten, an
appearance change, a bulk recolour or a generative result -- MUST record exactly one history entry
that is individually undoable and redoable, and destructive commands (expand, flatten, knife,
cleanup) MUST be undoable to restore the pre-command live construct. The vector domain MUST NOT
batch unrelated edits into one entry in a way that prevents targeted undo.

**[STU-VEC-042]** All durable vector authority (geometry, appearance, styles, brushes, patterns,
gradients) MUST persist through the canonical Studio SurrealDB tables and `studio.vector`
EventLedger events defined in 14.23 under the SurrealDB-only authority guard with the
`no_sqlite_tripwire` in force ([STU-ARC-003], [STU-ARC-004]); live collaborative vector editing is
CRDT-backed. Bulk binary that vector authority references -- brush tip bitmaps, traced source
rasters, pattern tile renders, placed images -- MUST live in content-addressed artifact storage with
SurrealDB holding the records and references ([STU-SDB-002]); it MUST NOT be inlined into the
document database. Where this sub-section and 14.23 disagree on any type, field, event or schema id,
14.23 is canonical and this sub-section MUST be corrected to match.

**[STU-VEC-181] Shipped-content inventory contract.** Studio MUST ship a first-run content set for
this domain, and the captured reference inventory establishes its expected ORDER OF MAGNITUDE and its
structural contract, not its literal contents (which are third-party licensed):

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Content family | Captured library files | Captured primary entries |
|---|---|---|
| Brushes | 25 | 561 |
| Symbols | 28 | 884 |
| Graphic styles | 12 | 314 |
| Swatches (named) | 118 | 3,155 |
| Colour-book colours | 20 | 10,011 |
| Gradients | (carried inside the above) | 659 |
| Patterns | (carried inside the above) | 382 |
| Total primary entries | 183 files | 15,987 |

Normative requirements: every content family MUST be a named, ordered, importable and exportable
library; a library MUST carry its transitive supporting definitions ([STU-VEC-161]); entry counts and
FILE counts are different numbers and MUST NOT be reported interchangeably; and Studio's shipped set
MUST be independently replaceable by the operator without touching product code.

**[STU-VEC-199] Microtask derivation index.** Applying [STU-VEC-107] to this sub-section yields
exactly 276 microtasks. The correspondence is NORMATIVE and CLOSED: a microtask corresponds to a
yielding clause or to a table unit as marked, and to nothing else.

Rule 0 -- derivation markers are authoritative. Every table in this sub-section carries an italic
`*Derivation: ...*` marker sentence directly above it stating how many microtasks that table yields.
The marker is normative. A tool that classifies a table differently from its marker has diverged
from this sub-section and MUST be corrected to the marker, not the reverse. The five marker forms
are: parameter table taken whole (1); enumeration table taken whole (1); preset or command table
taken whole (1); catalogue table splitting per row (N); contract table carried into the clause's own
microtask (0). A sixth form, reading aid inside a non-yielding clause, also yields 0.

Rule 0a -- anchors inside table cells are never definitions here. Every one of the 156 clauses in
14.5 is defined as a PARAGRAPH opening with its bold anchor; not one is defined inside a table cell.
All 34 anchors that appear in cells of this sub-section are cross-references to clauses defined that
way elsewhere in it, and every table carrying one says so in its own marker. A tool that treats an
in-cell anchor as a clause definition here produces a second unit for a clause rule A has already
counted, which is a double count and not work. This rule constrains only 14.5; other modules do
define clause families in table cells, and this rule says nothing about them.

Rule A -- one microtask per yielding clause. Every numbered clause yields exactly one microtask
EXCEPT the members of the no-yield set below.

Rule B -- table units, counted from the markers of rule 0. A parameter table is a unit in its own
right even though it sits inside a clause that is also a unit, because its rows are bound-sets that
have to be individually proven; folding it into its clause loses that proof obligation. An
enumeration table is a unit for the same reason, its members being the criteria. A catalogue table
splits because each row names a separately implementable subject. A contract table does not split
and is not its own unit: it describes the fields of the single contract its clause already defines.

Declared non-yielding set

Ten clauses yield no microtask. Tables inside a non-yielding clause yield nothing either.

- `STU-VEC-100` -- reading rule: the seven-field numeric parameter contract.
- `STU-VEC-101` -- reading rule: the unit vocabulary.
- `STU-VEC-102` -- reading rule: soft bounds are UI presentation only.
- `STU-VEC-103` -- reading rule: the step contract.
- `STU-VEC-104` -- reading rule: the enumeration contract.
- `STU-VEC-105` -- reading rule: the capture-conflict rule.
- `STU-VEC-106` -- reading rule: the observed-value rule.
- `STU-VEC-107` -- reading rule: the microtask derivation rule.
- `STU-VEC-041` -- cross-cutting obligations, inherited by reference by every microtask.
- `STU-VEC-199` -- this clause, the derivation index itself.

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Ledger line | Basis | Yields |
|---|---|---|
| Clauses in 14.5 | anchors 001-073, 100-181 and 199 | 156 |
| less the no-yield set | reading rules 100-107, plus 041, plus 199 | -10 |
| **Rule A subtotal** | one microtask per yielding clause | **146** |
| Parameter tables | 41 tables, each taken whole; rows are bound-set acceptance criteria | 41 |
| Enumeration tables | 26 tables, each taken whole; members are acceptance criteria | 26 |
| Preset and inventory tables | 2 tables, each taken whole and explicitly NOT split per row | 2 |
| Catalogue: tool table of 007 | one per Studio tool | 17 |
| Catalogue: shape catalogue of 127 | one per parametric shape | 9 |
| Catalogue: geometry operations of 130 | one per operation | 13 |
| Catalogue: effect families of 154 | one per live-effect family | 9 |
| Catalogue: brush kinds of 162 | one per captured brush kind | 5 |
| Catalogue: transforms of 030 | one per transform or distortion construct | 8 |
| Contract tables | 2 tables carried into the owning clause's microtask | 0 |
| Reading aids in non-yielding clauses | 3 tables | 0 |
| **Rule B subtotal** | table units | **130** |
| **Total microtasks yielded by 14.5** | rule A plus rule B | **276** |

Two counts are traps for a tool that reads tables structurally rather than reading the markers.
The provider/AI table of 039 is taken WHOLE and yields 1, not 7: that lane is optional, is not a
build gate, and generative features are out of scope, so its rows are not seven units of work. The
shipped-content inventory of 181 is taken whole and yields 1: its rows are counts, not subjects.
A third trap was removed rather than documented: clause 127 previously carried one table whose rows
were shapes and whose columns were bounds, so it read as a catalogue to a human and as a parameter
table to a parser. It is now two tables -- a nine-row shape catalogue that splits, and a bound-set
parameter table taken whole -- and neither can be misread.

Clauses carrying a declared SPEC GAP -- 118, 124, 128, 149, 158, 172, 177 and 036 -- still yield
their rule-A microtask, and that microtask's FIRST acceptance row MUST read "the named gap is raised
to the operator as a capture request and is NOT closed by an invented value".

A microtask derived from a clause with a parameter table MUST carry that table verbatim, including
every `unknown`; a microtask derived from an enumeration MUST carry every member and its captured
value. No microtask may cite the green-room corpus as its source of truth: the corpus is provenance
for HOW a clause was derived, and this sub-section is the authority ([STU-SECTION-002]).

---
