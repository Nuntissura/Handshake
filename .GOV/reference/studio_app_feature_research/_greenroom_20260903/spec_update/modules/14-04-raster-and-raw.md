---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206-draft"
bundle_id: "master-spec-v02.206"
module_id: "14-04"
section_id: "14.4"
covers_section_ids: ["14.4", "14.12"]
title: "14.4 Studio -- Raster Imaging & Photo Editing / 14.12 Raw Develop Pipeline"
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
supersedes: "master-spec-v02.205 spec-modules/14-studio-creative-suite.md lines 198-489 (sub-section 14.4) and lines 2555-2723 (sub-section 14.12)"
derivation_basis: "green-room installed-application captures, 2026-09-03/04"
metadata_rule: "frontmatter is machine metadata; body follows after this block. body_sha256 and source_body_original_sha256 are assigned at bundle assembly per [CX-105D]."
provenance_sidecar: "14-04-raster-and-raw.provenance.json"
provenance_rule: "the sidecar records how each clause was derived and carries no normative content; this module is self-sufficient authority"
---
## 14.4 Raster Imaging & Photo Editing

Studio's raster imaging surface is the deduped union of the pixel-editing capability of every source
suite in Studio's parity basis, rebuilt as Handshake-native pixel primitives. It is the pixel-editing
domain of the unified `StudioDocument`: raster content lives as `StudioLayer` nodes whose `kind`
selects a pixel payload, sharing one selection surface, one masking surface, one color pipeline, one
history, and one export surface with every other Studio domain (14.3, [STU-DOC-004]). This
sub-section is the normative Studio raster feature set and it is self-sufficient: an implementer with
no other document MUST be able to build every primitive named here, and to derive the microtask set
that builds them, from this sub-section alone. No source-suite product name is a Studio tool, panel,
adjustment, or command name. Every raster capability MUST be exposed as a non-destructive primitive
wherever a non-destructive form exists: destructive-in-place editing is a mode of a primitive, never
the only path. Canonical field-level types, schema ids, event variants, tables, and validation checks
for every primitive named here (`StudioLayer`, `StudioRasterTile`, `StudioSelectionSet`,
`StudioMask`, `StudioAdjustment`, `StudioLiveFilter`, `StudioBlendMode`, `StudioEffectStack`,
`StudioGradient`, `StudioPattern`, `StudioColorProfile`, and the `StudioParameterSpec` added
by [STU-RAS-100]) are defined in 14.23; where this sub-section and 14.23 conflict, 14.23 wins.

---

### 0. Replacement, Derivation, and Anchor Disposition

**[STU-RAS-100] REPLACEMENT DECLARATION.** This module replaces the v02.205 bodies of 14.4 and 14.12
in full. The superseded text was authored from vendor documentation; this text is authored from
behaviour parsed out of the installed applications themselves — type libraries, resource-compiled
dialog layouts, binary preset containers, develop-engine settings serializations, and shipped colour
and lens profile corpora. Where this module and any older Section 14 statement disagree about
observable behaviour, this module wins and the older statement is a defect to repair, not a source to
reconcile against. The derivation record for every clause in this module lives in the companion
provenance sidecar named in this module's frontmatter; that sidecar is audit metadata and carries no
normative content. This module stands alone as authority, per [STU-SECTION-002].

**[STU-RAS-101] ANCHOR DISPOSITION.** Every anchor in the v02.205 bodies of 14.4 and 14.12 carries an
explicit disposition. `RETAINED` means the clause is reproduced in this module with its meaning
unchanged. `EXTENDED` means the clause is retained and one or more new clauses add captured detail it
lacked; the retained text remains true. `SUPERSEDED-IN-PART` means a named portion of the clause is
replaced by a new clause because the captured behaviour contradicts it; the clause states which
portion. No anchor is silently dropped, and no retired anchor id is ever reused. New anchors in this
module begin at 100 in each prefix so that nothing collides with the v02.205 range, which ran to
STU-RAS-051 and STU-RAW-018. `[STU-RAS-140]` is deliberately unissued; it was drafted and renumbered
before publication and MUST NOT be assigned later. The letter-suffixed anchors `[STU-RAW-008a]`,
`[STU-RAW-014a]` and `[STU-RAW-014b]` are inherited from v02.205 and are retained exactly as written;
new anchors never use letter suffixes. `[STU-RAW-014c]` was NEVER ASSIGNED. It appears in no Master
Spec bundle from v02.200 through v02.205, where the develop enhance/mask family runs
`[STU-RAW-014]`, `[STU-RAW-014a]` and `[STU-RAW-014b]` and stops; it is therefore not a clause that
was renumbered, and no text was lost with it. A microtask contract citing it is citing a clause that
has never existed and cannot be validated against one. The behaviour such a contract names — the
Enhance split into a native deterministic tier and an optional model-adapter tier, with its
re-editability lifecycle and receipts — is carried in this module by `[STU-RAW-014]` and
`[STU-RAW-130]`, and a stale citation MUST be corrected to those anchors rather than resolved by
writing a new clause to fit it. `[STU-RAW-014c]` MUST NOT be assigned later. Two further
letter-suffixed ids that earlier drafts of this module used in cross-references were likewise NEVER
ASSIGNED and MUST NOT be assigned later: `[STU-CON-007c]` and `[STU-RAS-001c]`. Neither is an anchor.
Each was a pointer at a lettered bullet INSIDE a clause — property (c) of `[STU-CON-007]`
(DETERMINISTIC, which requires an unseeded stochastic operation to expose an explicit seed), and
obligation (c) of `[STU-RAS-001]` (Argus observability: structured state, receipts and a visual
snapshot path) — written in anchor form, so a reader following it reached nothing and a microtask
citing it could not be validated. The contracts are unchanged and live in those two parent clauses;
every reference in this module now cites the parent anchor and names the bullet in prose. A stale
citation to either suffixed id MUST be corrected the same way, never resolved by writing a new
clause to fit it.

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| v02.205 anchor | Disposition | Extending / superseding clause |
|---|---|---|
| STU-RAS-001 .. STU-RAS-005 | RETAINED | — (STU-RAS-004 EXTENDED by [STU-RAS-116]) |
| STU-RAS-006 | RETAINED | — |
| STU-RAS-007 | EXTENDED | [STU-RAS-113], [STU-RAS-114] |
| STU-RAS-008, STU-RAS-009 | RETAINED | — |
| STU-RAS-010 | EXTENDED | [STU-RAS-115] |
| STU-RAS-011 | EXTENDED | [STU-RAS-117] |
| STU-RAS-012, STU-RAS-013, STU-RAS-014, STU-RAS-051 | RETAINED | — |
| STU-RAS-015, STU-RAS-016 | EXTENDED | [STU-RAS-124] |
| STU-RAS-017 | EXTENDED | [STU-RAS-118] |
| STU-RAS-018 | EXTENDED | [STU-RAS-119], [STU-RAS-120] |
| STU-RAS-019 | RETAINED | — |
| STU-RAS-020 | EXTENDED | [STU-RAS-121], [STU-RAS-122], [STU-RAS-123] |
| STU-RAS-021, STU-RAS-022 | RETAINED | — |
| STU-RAS-023 | SUPERSEDED-IN-PART (its parameter list and its "stroke-smoothing 0–100" claim) | [STU-RAS-125] .. [STU-RAS-136] |
| STU-RAS-024 | EXTENDED | [STU-RAS-137], [STU-RAS-138] |
| STU-RAS-025 | EXTENDED | [STU-RAS-160], [STU-RAS-161] |
| STU-RAS-026, STU-RAS-027, STU-RAS-028 | EXTENDED | [STU-RAS-139], [STU-RAS-170] |
| STU-RAS-029 | EXTENDED | [STU-RAS-141], [STU-RAS-142] |
| STU-RAS-030 | EXTENDED | [STU-RAS-143] |
| STU-RAS-031 | EXTENDED | [STU-RAS-144] |
| STU-RAS-032 | EXTENDED | [STU-RAS-145] .. [STU-RAS-148] |
| STU-RAS-033 | EXTENDED | [STU-RAS-149] |
| STU-RAS-034 | EXTENDED | [STU-RAS-150] |
| STU-RAS-035 | EXTENDED | [STU-RAS-151], [STU-RAS-152], [STU-RAS-153] |
| STU-RAS-036, STU-RAS-037 | RETAINED | — |
| STU-RAS-038, STU-RAS-039 | EXTENDED | [STU-RAS-154], [STU-RAS-155] |
| STU-RAS-040 | SUPERSEDED-IN-PART (its one-instance-per-effect-kind model) | [STU-RAS-156], [STU-RAS-157], [STU-RAS-158] |
| STU-RAS-041 | EXTENDED | [STU-RAS-159] |
| STU-RAS-042, STU-RAS-043 | RETAINED | — |
| STU-RAS-044 | RETAINED | — |
| STU-RAS-045, STU-RAS-046 | EXTENDED | [STU-RAS-164] |
| STU-RAS-047 | RETAINED | — |
| STU-RAS-048 | SUPERSEDED-IN-PART (its authority direction) | [STU-RAS-102] |
| STU-RAS-049 | EXTENDED | [STU-RAS-163] |
| STU-RAS-050 | RETAINED | — |
| STU-RAW-001 | EXTENDED | [STU-RAW-100], [STU-RAW-106] |
| STU-RAW-002 | SUPERSEDED-IN-PART (its flat "translate the incoming parameters" import rule) | [STU-RAW-101], [STU-RAW-102], [STU-RAW-103] |
| STU-RAW-003, STU-RAW-004 | RETAINED | — |
| STU-RAW-005 | SUPERSEDED-IN-PART (its "each with an explicit unit/range" universal claim) | [STU-RAW-102], [STU-RAW-110], [STU-RAW-111] |
| STU-RAW-006 | EXTENDED | [STU-RAW-112], [STU-RAW-113] |
| STU-RAW-007 | EXTENDED | [STU-RAW-114] |
| STU-RAW-008a | EXTENDED | [STU-RAW-115] |
| STU-RAW-008 | EXTENDED | [STU-RAW-116] |
| STU-RAW-009 | EXTENDED | [STU-RAW-117], [STU-RAW-127] |
| STU-RAW-010 | EXTENDED | [STU-RAW-118] |
| STU-RAW-011 | EXTENDED | [STU-RAW-119] |
| STU-RAW-012 | EXTENDED | [STU-RAW-104], [STU-RAW-120] |
| STU-RAW-013 | EXTENDED | [STU-RAW-125], [STU-RAW-126], [STU-RAW-128] |
| STU-RAW-014a | EXTENDED | [STU-RAW-121], [STU-RAW-122], [STU-RAW-123] |
| STU-RAW-014b | RETAINED | — |
| STU-RAW-014 | EXTENDED | [STU-RAW-130] |
| STU-RAW-015 | EXTENDED | [STU-RAW-124] |
| STU-RAW-016 | EXTENDED | [STU-RAW-131] |
| STU-RAW-017, STU-RAW-018 | RETAINED | — |

**[STU-RAS-102] SPEC-COMPLETENESS RULE (supersedes the authority direction of [STU-RAS-048]).** Every
raster capability Studio ships MUST be represented by a clause or table row in 14.4. A capability
present in the derivation corpus but absent here is a defect in this sub-section, to be repaired by
adding a clause, and MUST NOT be resolved by reading the corpus at implementation time. The
capability inventory this sub-section was written against holds 2,700 raster-domain rows and 471
raw-domain rows; row counts are not clause counts, because a preset's *name* is evidence while the
normative statement is the *contract* around it. The completeness test is loss of contract, not loss
of rows: if an implementer would have to guess a range, a default, an enumerated value, a unit, or
what a feature does, that thing belongs in this sub-section.

---

### 1. The Parameter Contract (normative for 14.4 and 14.12)

**[STU-RAS-103] `StudioParameterSpec`.** Every numeric parameter exposed by any Studio raster or
develop primitive MUST be declared as a `StudioParameterSpec` (schema id
`hsk.studio.parameter_spec@1`, a canonical primitive added to the [STU-DOC-002] set and field-owned by
14.23) carrying these fields as SEPARATE, independently-serialized values. Collapsing any two of them
into one is forbidden.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Meaning | Absent value |
|---|---|---|
| `hard_min`, `hard_max` | the values the engine accepts. A value outside them is an error, not a clamp target. | `unknown` |
| `soft_min`, `soft_max` | the range the default control presents. An operator or model MAY type a value beyond a soft bound; it MUST NOT exceed a hard bound. | `unknown` |
| `default` | the factory value the parameter holds when the operator has made no edit. | `unknown` |
| `unit` | a token from the [STU-RAS-105] unit vocabulary. | `unknown` |
| `precision` | decimal places carried by the value. | `unknown` |
| `step`, `coarse_step`, `fine_step` | increments for the scrubbable control. | derived per [STU-RAS-107] |
| `observed_min`, `observed_max` | the range seen across shipped authoring data. Evidence only. | `absent` |
| `bound_classification` | `declared` \| `observed` \| `derived` \| `unknown`, per bound field. | required |

**[STU-RAS-104] HARD AND SOFT BOUNDS ARE NEVER EQUATED.** `hard_*` and `soft_*` MUST both be emitted
from the first implementation of every parameter even when only one is known; the unknown one is
emitted as `unknown`, never as a copy of the known one. This is not stylistic. In the captured
parameter corpora the two ranges differ on the majority of parameters that declare both, and once
they have been written as one value the distinction cannot be recovered without re-deriving it from
the source applications. A `StudioParameterSpec` whose `soft_min == hard_min` and
`soft_max == hard_max` is only legal when a source actually declared them equal and
`bound_classification` says `declared` for all four.

**[STU-RAS-105] UNIT VOCABULARY.** `unit` MUST be one of the following tokens. This list is closed;
adding a unit is a governed spec change. `percent`, `pixels`, `degrees`, `points`, `picas`, `inches`,
`millimetres`, `centimetres`, `pixels_per_inch`, `relative_distance`, `normalized_0_1`,
`normalized_signed_1`, `kelvin`, `stops_ev`, `levels_0_255`, `gradient_position_0_4096`,
`milliseconds`, `count`, `none`. `document_unit` is the deferred token for a length that resolves
against the document's declared unit per [STU-DOC-003]; a field carrying `document_unit` MUST also
carry the resolved unit at the API decode boundary, and a mixed-unit field remains forbidden.

**[STU-RAS-106] THE `unknown` TOKEN IS NORMATIVE.** Where a bound, default, unit, or precision was not
declared by any source and could not be derived, the field MUST carry the literal token `unknown` and
`bound_classification` MUST say `unknown`. An implementer MUST NOT invent a value for an `unknown`
field, MUST NOT clamp to it, and MUST NOT treat an `observed_*` value as its substitute. An `unknown`
hard bound means the engine accepts any value the type admits until a governed spec change narrows
it; the implementation MUST accept such values and MUST NOT reject them for being outside an observed
range. Resolving an `unknown` is a governed spec enrichment, never an implementation decision.

**[STU-RAS-107] OBSERVED RANGES ARE NOT LIMITS.** An `observed_min` / `observed_max` pair records the
range a parameter was seen to take across shipped authoring data (presets, brush libraries, styles,
catalogs). It is a lower bound on the true range and nothing more. Clamping to an observed range is
forbidden, because it would forbid legal values; presenting an observed range as a `hard_*` or
`soft_*` bound is a spec-conformance defect. Where this sub-section states an observed range it says
so in the row.

**[STU-RAS-108] SCRUBBABLE NUMERIC CONTROL.** Every `StudioParameterSpec` drives one shared numeric
control primitive: a value field that is type-editable, drag-scrubbable on its label, and
keyboard-adjustable. Where `step`, `coarse_step` and `fine_step` were not captured, they are derived
by this normative Studio rule — which is Studio policy, not observed vendor behaviour, and is
classified `derived`: `step = 10^(-precision)` when `precision` is known and `1` when it is `unknown`;
`coarse_step = step * 10`; `fine_step = step / 10` clamped to no finer than `10^(-precision)`. Drag
and keyboard adjustment MUST clamp at `hard_min`/`hard_max`, MUST pass freely through
`soft_min`/`soft_max`, and MUST expose the clamped value, the clamping event, and both bound pairs to
AccessKit and to the model command surface so a model can tell a clamp from a no-op.

**[STU-RAS-109] ENUMERATED VALUES ARE CLOSED AND INTEGER-STABLE.** Every enumerated parameter MUST
declare its complete member list with a stable integer discriminant per member. Integer values in
this sub-section are the canonical Studio discriminants; they are chosen to match the discriminants
carried by the captured source enumerations so that imported documents round-trip without a mapping
table, and they MUST NOT be renumbered. Adding a member is a governed spec change. An unknown
enumerator arriving on import MUST produce an unsupported-value receipt ([STU-RAS-162]) and MUST NOT
be silently coerced to a neighbour. Where two captured source families declare *conflicting*
discriminants for the same concept — which happens, and [STU-RAS-151] is the worked case — Studio
declares its own contiguous numbering and the per-source import/export mapping table becomes normative
and MUST be stated in the clause that owns the enumeration. Silence about a conflicting mapping is a
defect.

**[STU-RAS-110] PARAMETER SPECS ARE MODEL-VISIBLE.** The full `StudioParameterSpec` for every
parameter of every command MUST be readable through the typed model command surface (14.16) without
invoking the command, so a model can plan an edit against real bounds instead of probing. The
`schemars`-generated `inputSchema` for a command MUST carry `hard_*` as schema bounds and `soft_*`,
`default`, `unit`, `precision`, `step` family, and `bound_classification` as annotations. A parameter
whose `hard_*` is `unknown` MUST NOT emit a fabricated schema bound.

**[STU-RAS-111] PARAMETER-CONTRACT VALIDATION.** `StudioValidationDescriptor` (14.24) MUST carry a
check that fails any shipped parameter whose `StudioParameterSpec` omits a field, equates hard and
soft bounds without `declared` classification on all four, carries a numeric value where the source
evidence supports only `unknown`, or clamps to an `observed_*` value. This check is blocking severity.

**[STU-RAS-112] DERIVATION CLASSIFICATION VOCABULARY.** Every value in this module and every
`StudioParameterSpec` field carries one of four classifications, and each token means the same thing
everywhere in 14.4 and 14.12. `declared` — the value was stated by the source as a constraint on the
parameter; it is a fact about the engine. `observed` — the value was seen in shipped authoring data;
it is a fact about that data and a lower bound on the engine's true range, never a limit
([STU-RAS-107]). `derived` — the value was computed from parsed data, or it is Studio's own stated
policy; the clause carrying it states the derivation so it can be checked and disagreed with.
`unknown` — no source declared it and it could not be derived; [STU-RAS-106] governs. A row that
states a number without a classification is a defect.

---

### 2. Cross-Cutting Raster Obligations

**[STU-RAS-001]** Every raster feature in this sub-section that creates, changes, or removes an
operator-visible surface (a layer, a selection, a mask, an adjustment, a filter layer, a stroke, a
transform, a channel, or a document color/bit-depth state) MUST expose (a) a native operator GUI
control per the Studio shell and model-visibility contract (14.16), (b) a typed, deterministic,
model-steerable command with a stable identifier and a `schemars`-generated `inputSchema` (14.16,
14.14), (c) Argus observability — structured state, receipts, and a visual/pixel snapshot path for a
no-context model (14.16, 14.20), and (d) a dual-audience UserManual entry (14.22). This obligation is
stated once here and is normative for every feature row and clause in 14.4; it MUST NOT be re-stated
per feature and MUST NOT be omitted per feature.

**[STU-RAS-002]** Raster operations MUST obey the headless/quiet law (14.20): brush strokes, filter
previews, ML-backed selections, transforms, and batch pixel work MUST run without stealing focus,
popping foreground windows, or hijacking input, and MUST be observable through logs, receipts, and
snapshots rather than through a visible application window.

**[STU-RAS-003]** A model-authored raster mutation (any command batch that changes pixel authority —
pixel writes, layer-graph edits, mask edits, adjustment/filter parameter changes, channel operations,
mode/bit-depth conversions) MUST NOT write a SurrealDB authority record directly. It MUST enter the
kernel sandbox, be validated by the `StudioValidationDescriptor` catalog (14.24), and pass the
`PromotionGate` (`PromotionDecisionV1: Accepted`) before authority changes, exactly as [STU-ARC-005]
requires. This lifecycle is not optional and MUST NOT be bypassed on model confidence.

**[STU-RAS-004]** Raster pixel data MUST be stored and composited as `StudioRasterTile` tiles (14.23), not
as monolithic full-frame buffers, so that large documents, partial edits, undo/redo (14.19), and CRDT
collaborative editing operate on bounded tile deltas. All compute-heavy pixel work (compositing,
filtering, transform resampling, ML selection/inpaint inference) MUST execute in the `studio-engine`
crate through the `RasterEngine`/`RenderEngine` traits and MUST NOT introduce `wgpu`/WGSL/GPU
dependencies into `handshake_core` ([STU-ARC-002]).

**[STU-RAS-005]** Every raster operation MUST be reversible through the unified per-document history/undo
surface (14.19); an operation that cannot be represented as a `StudioHistoryEntry` delta MUST NOT be
shipped as a raster command.

**[STU-RAS-116] BULK BINARY IS NOT AUTHORITY-DATABASE CONTENT.** Raster tiles, sampled brush-tip
bitmaps, erodible-tip height maps, pattern bitmaps, colour lookup tables, and camera/lens profile
payloads MUST live in Handshake's content-addressed artifact tier, with SurrealDB holding the records
and typed references ([STU-SDB-003]). The scale that forces this is real: a single shipped raster and
vector brush-library pair measures roughly 350 MB of tip bitmaps, and the camera and lens profile
corpora of [STU-RAW-125] and [STU-RAW-127] measure roughly 895 MB and 471 MB respectively. This does
not authorize a second database ([STU-OVR-003]); the artifact tier is not an authority store, and
every reference from a `StudioLayer`, `StudioPattern`, brush preset, or profile record to an artifact
MUST be a typed content-addressed id resolvable through the kernel `ResourceBroker`.

---

### 3. Raster Document, Layer Kinds, and Layer Numerics

**[STU-RAS-006]** A raster document is a `StudioDocument` whose layer tree contains one or more
raster-domain `StudioLayer` nodes over one or more `StudioArtboard` containers. Multiple named
artboards MUST be supported inside one document, each with its own pixel dimensions, background,
guides, layer auto-nesting, and export path, without forking the document (see 14.6 for layout
geometry).

**[STU-RAS-007]** Studio MUST provide the following normative raster `StudioLayer` kinds. Each row is one
deduped primitive; the `StudioLayer.kind` discriminant and its payload contract are canonical in
14.23.

*Derivation: catalogue table, splits per row; yields 7 microtasks, one per layer kind.*

| Layer kind (`StudioLayer.kind`) | Normative behavior |
|---|---|
| `raster` (pixel layer) | Editable pixel layer holding `StudioRasterTile` data at the document bit depth; target of all painting, retouching, and destructive filters. |
| `placed_asset` (non-destructive placed container) | Encapsulates source content (raster or vector) unrasterized at native resolution; hosts non-destructive transforms, filters, and effects and supports nested child-document editing. §4 defines its instance/link semantics. |
| `group` | Nests child layers with shared opacity, blend mode, effects, mask, and clip scope; default group blend is Pass Through ([STU-RAS-154]). |
| `adjustment` | Hosts one `StudioAdjustment` applied non-destructively to layers below (or clipped to its parent) with a built-in mask (§9). |
| `live_filter` | Hosts one `StudioLiveFilter` (a re-editable, maskable filter effect) applied non-destructively to layers below or clipped to its parent ([STU-RAS-037]). |
| `fill` | Whole-scope re-editable fill: solid color (`StudioSwatch`), gradient (`StudioGradient`), or pattern (`StudioPattern`), including a live tiling-pattern mode where painting one tile updates the repeat (§8). |
| `mask` | Grayscale alpha mask node hiding/revealing its parent; paintable, fillable, selection-derived, or parametric (§5). |

**[STU-RAS-113] LAYER KIND VERSUS ADJUSTMENT KIND ARE TWO FIELDS, NOT ONE.** The captured source
object models expose a single flat 24-member layer-kind enumeration in which fourteen members are not
layer kinds at all but adjustment identities (levels, curves, colour balance, brightness/contrast,
hue/saturation, selective colour, channel mixer, gradient map, inversion, threshold, posterize, photo
filter, exposure, black-and-white, vibrance, colour lookup). Studio MUST NOT copy that conflation:
`StudioLayer.kind` is the seven-member set of [STU-RAS-007], and an adjustment layer's identity lives
in `StudioAdjustment.kind` (§9). Import MUST map each flat member onto the pair
`(kind = adjustment, StudioAdjustment.kind = <row>)`, and export MUST reverse it, so the round trip is
lossless. The flat enumeration's three remaining non-Studio members — a 3D layer kind, a video layer
kind, and a smart-object kind — map as follows: smart object maps to `placed_asset`; the video layer
kind is owned by the video/timeline domain and MUST NOT be reintroduced as a raster layer kind; the 3D
layer kind is intentionally out of Studio scope and MUST produce an unsupported-feature receipt on
import ([STU-RAS-162]) rather than a silent drop.

**[STU-RAS-114] LAYER STACK ORDERING IS AN EXPLICIT ENUMERATED PLACEMENT.** Every command that inserts
or moves a `StudioLayer` MUST take a placement enumerator, not an index alone:
`place_inside = 0`, `place_at_beginning = 1`, `place_at_end = 2`, `place_before = 3`, `place_after = 4`.
Index-only reordering is forbidden because it is ambiguous across concurrent CRDT edits.

**[STU-RAS-115] LAYER OPACITY PARAMETER CONTRACT (extends [STU-RAS-010]).** Fill-opacity and
layer-opacity MUST be independent, canonical `StudioLayer` fields: fill-opacity fades pixel/fill
content while leaving `StudioEffectStack` effects at full strength; layer-opacity fades the whole
layer including effects. Both MUST be exposed and MUST NOT be collapsed into a single opacity field.
Their parameter contracts are:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `StudioLayer.opacity` | 0.0 | 100.0 | 0.0 | 100.0 | 100.0 | percent | 1 | hard declared; soft declared equal to hard; default derived |
| `StudioLayer.fill_opacity` | 0.0 | 100.0 | 0.0 | 100.0 | 100.0 | percent | 1 | hard declared; soft declared equal to hard; default derived |
| `StudioLayer(group).opacity` | 0.0 | 100.0 | 0.0 | 100.0 | 100.0 | percent | 1 | hard declared; soft declared equal to hard; default derived |
| `StudioMask.density` | 0.0 | 100.0 | 0.0 | 100.0 | 100.0 | percent | unknown | hard declared; default derived |
| `StudioMask.feather` | 0.0 | 10000.0 | 0.0 | 1000.0 | 0.0 | pixels | unknown | hard declared; soft unknown-until-declared, stated here as Studio policy and classified `derived` |

The `default` values above are `derived`: no source declares a factory default for layer opacity, and
the value 100 percent is the identity of the operation. This derivation is stated so an implementer
does not have to guess it, and it is the only legal derivation — an identity default.

**[STU-RAS-008]** Adjustment, live-filter, fill, and mask layers MUST be non-destructive and re-editable at
any time: their parameters MUST persist as structured `StudioLayer` payload and MUST NOT be baked into
pixels until an explicit rasterize/merge command. A destructive equivalent (apply-in-place) MUST be
available as an explicit operator/model command, and MUST emit a distinct history entry.

**[STU-RAS-009]** Studio MUST support layer organization metadata as canonical `StudioLayer` fields:
freeform tags (including export-semantic and accessibility tags), color labels, and named layer states
(saved visibility/config sets, including query-based states) that recall document variations without
duplicating the document. A layer/object find surface MUST query these fields.

**[STU-RAS-010]** Fill-opacity and layer-opacity are independent canonical fields; see [STU-RAS-115] for
their parameter contracts.

**[STU-RAS-011]** Studio MUST support pixel-layer rasterization, merge-down, merge-visible, flatten, and
stamp-visible (merge-visible-to-new-layer) operations; each MUST be an explicit command emitting a
`StudioHistoryEntry`, and MUST honor matting/defringe/remove-halo cleanup of edge fringe left by a
prior selection or extraction.

**[STU-RAS-117] RASTERIZE TARGET AND MATTE ENUMERATIONS (extends [STU-RAS-011]).** The rasterize
command MUST take an explicit target enumerator rather than always rasterizing the whole layer:
`text_contents = 1`, `shape = 2`, `fill_content = 3`, `layer_clipping_path = 4`, `entire_layer = 5`,
`linked_layers = 6`. The matting/defringe command MUST take an explicit matte enumerator:
`none = 1`, `foreground_color = 2`, `background_color = 3`, `white = 4`, `black = 5`,
`semi_gray = 6`, `neutral_gray = 7`.

---

### 4. Placed-Asset Container and Masking Semantics

**[STU-RAS-012]** The `placed_asset` layer MUST support both embedded mode (source content stored inside
the document) and linked mode (source referenced from an external file), with explicit conversion in
both directions, and MUST surface link health (up-to-date / modified / missing) as inspectable state
with an update-all command.

**[STU-RAS-013]** Duplicating a `placed_asset` MUST create a shared-source instance (edits to the source
propagate to all instances), while an explicit "new independent copy" command MUST create a detached
source. Replacing a `placed_asset` source MUST preserve all applied transforms, live filters,
adjustments, and effects across every instance.

**[STU-RAS-014]** A `placed_asset` MUST support: non-destructive accumulated transforms with a
reset-transforms command; unpacking back into its component layers in place (convert-to-layers);
exporting its embedded source back to a standalone file in its original format (export-contents
touchpoint to 14.13); and statistical stack rendering (mean, median, maximum, range, and the other
stack modes) over a multi-layer container for noise reduction and analysis. A "collect linked assets
into one portable folder" (package) command MUST be provided as a document-portability touchpoint (see
14.13; honor [GLOBAL-PORTABILITY] — relocatable, not machine-locked).

**[STU-RAS-051]** Studio MUST provide a focus-stack / seamless-blend composite operation: given a
multi-layer stack, automatically align (reusing the auto-align primitive), then per-region select and
blend the sharpest/best-exposed content into one seamless result (extended depth of field, and
seamless panorama/exposure blending), producing a non-destructive masked composite. This composite
reuses the existing auto-align, statistical-stack ([STU-RAS-014]), masking ([STU-RAS-119]), and
HDR/exposure ([STU-RAS-149]) primitives; it is a named model-steerable command subject
to [STU-CON-007].

**[STU-RAS-015]** `StudioMask` is the single canonical masking primitive across all Studio domains (14.3).
Studio MUST support these deduped mask forms, all attachable to any maskable layer or group:

*Derivation: catalogue table, splits per row; yields 5 microtasks, one per mask form.*

| Mask form | Normative behavior |
|---|---|
| Grayscale (pixel) mask | Paintable/fillable 8/16-bit alpha mask hiding/revealing its parent; created blank, from selection, or from a channel. |
| Vector mask | Path-defined (`StudioVectorPath`) mask with resolution-independent edges; convertible to/from a pixel mask and combinable with one. |
| Clipping mask | Uses a base layer's content/alpha to clip the layers clipped to it. |
| Compound mask | Combines multiple mask nodes non-destructively via boolean operators (add / subtract / intersect / xor). |
| Parametric (live) mask | Non-destructive mask generated live from image properties and auto-updating with the image, in the normative types Hue-Range, Luminosity-Range, and Band-pass; stays re-editable. |

**[STU-RAS-016]** Masks MUST support density/opacity, feather, and enable/disable, and MUST be linkable or
unlinkable from parent-layer position. Any selection (§5) MUST be convertible to a mask, and any mask
MUST be loadable as a selection, through one shared conversion path.

**[STU-RAS-124] MASK MODIFY CONTROL CONTRACT (extends [STU-RAS-015], [STU-RAS-016]).** The shared
mask-modify surface MUST expose exactly two numeric parameters with the contracts given
in [STU-RAS-115] — density in percent with a declared hard range of 0..100, and feather in pixels with a
declared hard range of 0..10000 — plus the boolean enable, the boolean position-link, an invert
command, and a mask-to-selection / selection-to-mask conversion pair. The same two parameters and the
same declared hard ranges MUST be used by the adjustment-brush mask-modify surface; there is one mask
parameter set, not one per host.

---

### 5. Selection

**[STU-RAS-017]** `StudioSelectionSet` (14.3) is the single canonical selection primitive. All selection
tools produce, refine, or consume a `StudioSelectionSet`; there is no per-tool bespoke selection
representation. Every selection-producing tool MUST expose the shared combine modes, a document-wide
default anti-alias toggle, and per-tool feather.

**[STU-RAS-118] SELECTION COMBINE MODE ENUMERATION (extends [STU-RAS-017]).** The combine mode is a
closed four-member enumeration shared by every selection-producing tool, every mask boolean, and every
model command that yields a selection: `replace = 1`, `extend = 2` (add), `diminish = 3` (subtract),
`intersect = 4`. The vector/shape boolean operation enumeration is a separate four-member set —
`add = 1`, `xor = 2`, `intersect = 3`, `subtract = 4` — and MUST NOT be merged with the selection
combine set even though four of the operations have the same names, because the two carry different
discriminants and different semantics (pixel coverage versus path winding). Each tool MUST expose the
mode both as an explicit typed argument and as a modifier-key affordance.

**[STU-RAS-018]** Studio MUST provide the following deduped selection tools/commands. Each row is one
primitive.

*Derivation: catalogue table, splits per row; yields 12 microtasks, one per selection tool or operation.*

| Tool or operation | Normative behavior |
|---|---|
| Geometric marquee | Rectangular, elliptical, single-row (1px), and single-column (1px) pixel selections with fixed-ratio/fixed-size constraint, draw-from-center, and proportional modifiers. |
| Freehand lasso | Freehand-drawn selection boundary. |
| Polygonal lasso | Straight-segment click-to-build selection. |
| Magnetic lasso | Edge-snapping selection with width, contrast, and frequency controls. |
| Quick/selection brush | Painted selection that grows to matching regions and snaps to edges, with add/subtract by stroke and an on-canvas selection overlay. |
| Flood select (tolerance) | Selects similar color/tone from a sampled point by tolerance, with contiguous and sample-all-layers options. |
| Object select | Auto-selects a detected object under a hover, rectangle, or lasso region using an on-device model, with multi-part component selection and optional matting. |
| Subject select | One-command on-device selection of the dominant subject(s), recordable into a macro/batch. |
| Sky select | One-command on-device selection of sky regions. |
| Tonal-range select | Selects Shadows, Midtones, or Highlights tonal bands. |
| Luminosity / alpha select | Builds a selection from a layer's luminosity or content/alpha for luminosity-masking workflows. |
| Color-range select | Eyedropper-sampled fuzziness-masked selection by color similarity with a live preview. |

**[STU-RAS-119] MARQUEE CONSTRAINT PARAMETER CONTRACT (extends [STU-RAS-018]).** The geometric marquee
MUST carry a constraint mode enumeration and the numeric fields the constraint consumes. The captured
tool-preset serialization proves the field set and the way aspect is stored: aspect is a numeric ratio
pair, not a string, and fixed size carries its own unit selector per axis.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `constraint_mode` | enum | enum | — | — | `normal` | none | 0 | `normal` \| `fixed_ratio` \| `fixed_size` (observed as an integer mode field) |
| `aspect_width`, `aspect_height` | unknown | unknown | unknown | unknown | unknown | count | 3 | observed 4.000 and 6.000 in shipped presets, stored scaled by 1000 |
| `fixed_width`, `fixed_height` | unknown | unknown | unknown | unknown | unknown | document_unit | unknown | observed 64.0 with a per-axis unit selector |
| `feather` | 0.0 | unknown | 0.0 | 250.0 | 0.0 | pixels | unknown | default and soft bound `derived`; hard max not declared by any source |
| `anti_alias` | — | — | — | — | true | none | — | boolean, observed default true |
| `contiguous` | — | — | — | — | true | none | — | boolean, flood select only |
| `sample_all_layers` | — | — | — | — | false | none | — | boolean |

**[STU-RAS-120] SELECTION MODIFY OPERATIONS.** `StudioSelectionSet` MUST expose Grow, Shrink (with a
circular-application option), Feather, Smooth, Border, Expand and Contract as typed commands. Each
takes a single radius/width parameter in `pixels` with a decimal-place count declared by the control
and a hard maximum of `unknown`; no source declares the maximum, so the implementation MUST accept any
positive value the type admits and MUST NOT clamp to a guessed ceiling ([STU-RAS-106]).

**[STU-RAS-121] UNIFIED EDGE-REFINEMENT SURFACE (extends [STU-RAS-020]).** One surface — not two — owns
matte and edge refinement for hair and fine detail, and it is the same surface whether it was entered
from a selection or from a mask. Its normative control set is:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `view_mode` | enum | enum | — | — | `overlay` | none | — | closed set: `overlay`, `marching_ants`, `on_black`, `on_white`, `black_and_white`, `on_layers`, `reveal_layer`; `declared` |
| `show_radius` | — | — | — | — | false | none | — | boolean; preview toggle |
| `show_original` | — | — | — | — | false | none | — | boolean; preview toggle |
| `smart_radius` | — | — | — | — | false | none | — | boolean; adaptive edge-detection radius |
| `radius` | unknown | unknown | unknown | unknown | unknown | pixels | 1 | unit and precision `declared`; both hard bounds, both soft bounds and the default are undeclared by every source |
| `smooth` | unknown | unknown | unknown | unknown | unknown | count | 0 | unit `declared` as a unitless smoothing count; all four bounds and the default undeclared |
| `feather` | unknown | unknown | unknown | unknown | unknown | pixels | 1 | all four bounds and the default undeclared |
| `contrast` | unknown | unknown | unknown | unknown | unknown | percent | 1 | all four bounds and the default undeclared |
| `shift_edge` | unknown | unknown | unknown | unknown | unknown | percent | 1 | signed; all four bounds and the default undeclared |
| `decontaminate_colours` | — | — | — | — | false | none | — | boolean; gates the amount control |
| `decontamination_amount` | unknown | unknown | unknown | unknown | unknown | percent | 1 | all four bounds and the default undeclared |
| `output_to` | enum | enum | — | — | `selection` | none | — | closed set: `selection`, `layer_mask`, `new_layer`, `new_layer_with_mask`, `new_document`, `new_document_with_mask`; `declared` |
| `remember_settings` | — | — | — | — | false | none | — | boolean; persists the control state as the surface default |

The six numeric controls above — `radius`, `smooth`, `feather`, `contrast`, `shift_edge` and
`decontamination_amount` — carry a declared unit and precision and no declared hard bound, soft bound
or default. That is a genuine gap in the derivation corpus, not an omission in this clause: the source
surfaces carry the unit label on the control but declare no minimum, maximum, or default anywhere an
offline parse can reach. They MUST ship as `unknown` per [STU-RAS-106] and MUST NOT be clamped, and
the soft bounds MUST NOT be back-filled from the hard bounds or from any observed spread.

**[STU-RAS-122] REFINEMENT BRUSH SET.** The edge-refinement surface MUST provide a brush set with the
modes `refine_edge`, `add_to_selection`, `subtract_from_selection`, plus a matte/foreground/background
mode set and a feather mode, each driven by the one brush engine of §6 rather than by a private brush
implementation. It MUST additionally provide an auto-masking toggle, a quick-select brush, a lasso, and
a polygonal lasso inside the same surface, each producing into the same `StudioSelectionSet`.

**[STU-RAS-123] SKY-REPLACEMENT COMPOSITE PARAMETER CONTRACT.** The sky-replacement composite is a
named model-steerable command that selects the sky region ([STU-RAS-018]), substitutes a replacement
sky asset, and harmonizes the foreground. Its parameters carry declared hard bounds:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `shift_edge` | -100 | 100 | unknown | unknown | unknown | percent | 0 | hard declared |
| `border_smoothness` | 0 | 100 | unknown | unknown | unknown | percent | 0 | hard declared |
| `sky_brightness` | -100 | 100 | unknown | unknown | unknown | percent | 0 | hard declared |
| `sky_temperature` | -100 | 100 | unknown | unknown | unknown | percent | 0 | hard declared |
| `sky_scale` | 50 | 400 | unknown | unknown | unknown | percent | 0 | hard declared |
| `foreground_lighting` | 0 | 100 | unknown | unknown | unknown | percent | 0 | hard declared |
| `edge_lighting` | 0 | 100 | unknown | unknown | unknown | percent | 0 | hard declared |
| `color_adjustment` | 0 | 100 | unknown | unknown | unknown | percent | 0 | hard declared |
| `output_to` | enum | enum | — | — | `new_layers` | none | — | `new_layers` \| `duplicate_layer` |
| `flip_sky`, `sky_move`, `sky_fade` | — | — | — | — | — | — | — | orientation and placement of the replacement asset |

This composite MUST be implemented on the native on-device path of [STU-RAS-044]; no cloud sky
library is a dependency, and the replacement sky asset is an ordinary Studio asset.

**[STU-RAS-019]** Object-select, subject-select, and sky-select MUST run on-device by default with no cloud
dependency; any cloud-accelerated variant is an optional adapter lane per §14 and MUST NOT be a core
dependency. On-device ML selection is a native Studio primitive, not a provider feature.

**[STU-RAS-020]** Studio MUST provide selection-refinement operations as commands on `StudioSelectionSet`
per [STU-RAS-120], and one unified edge-refinement surface per [STU-RAS-121] and [STU-RAS-122].

**[STU-RAS-021]** Quick-mask mode MUST let an operator or model convert the active `StudioSelectionSet`
into a paintable grayscale/rubylith overlay channel, edit it with any painting tool, and convert it
back to a selection, with alternative display modes.

**[STU-RAS-022]** Studio MUST support saving a selection to a persistent alpha/spare channel and reloading
it as a selection, preserved in round-trippable interchange formats (14.13). Save/load selection, alpha
channels, and the Channels surface (§10) are one shared mechanism, not three.

---

### 6. The Brush Engine

**[STU-RAS-023]** Studio MUST implement one native brush engine driving every painting and retouching tool.
Brush presets MUST be saved and reused as named presets, and tool presets MUST capture a tool plus its
full option configuration. This clause is RETAINED IN PART: its statement of a single engine, of named
brush presets and of tool presets stands, and its enumeration of the engine's parameters and its
"stroke-smoothing 0–100" claim are SUPERSEDED by [STU-RAS-125] through [STU-RAS-136].

**[STU-RAS-125] ONE BRUSH ENGINE (supersedes the parameter list of [STU-RAS-023]).** Studio MUST
implement exactly one native brush engine, in `studio-engine` behind `RasterEngine`, driving every
painting tool, every retouching tool, every eraser, every mask-painting surface, every selection
brush, every refinement brush, and every develop-time local brush. [STU-RAS-023]'s statement that
there is one engine is retained. Its enumeration of that engine's parameters as "brush tip/preset,
size, hardness, blend mode, opacity, flow, spacing, angle/roundness, pressure and tilt dynamics, and a
stroke-smoothing option set (0–100 ...)" is SUPERSEDED and MUST NOT be implemented: it is roughly a
tenth of the real surface and its smoothing range is wrong. The parsed brush libraries carry 223
distinct parameter keys across 643 shipped presets in one source serialization, and a 38-tag
parameter schema over 455 shipped brushes in another; the engine is a tip model plus a
five-sub-engine composition (dual tip, scatter, texture, colour dynamics, wet mixing) plus eleven
independently-controlled dynamics channels. [STU-RAS-126] through [STU-RAS-136] are the replacement.

**[STU-RAS-126] TIP MODEL — FOUR KINDS, ONE PRIMITIVE.** `StudioBrushTip` MUST support four tip kinds
in one primitive, discriminated by a `tip_kind` field, because a shipped library mixes them freely:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| `tip_kind` | Definition | Evidence of use |
|---|---|---|
| `parametric` | Round/elliptical tip generated from diameter, hardness, angle and roundness. | the default and majority case |
| `sampled` | A bitmap tip referenced by a stable tip id, resolved from the artifact tier ([STU-RAS-116]). | 316 of 643 presets in one library reference a sampled tip by id |
| `erodible` | A tip with a height map that wears down over a stroke and can be re-sharpened. | 77 of 643 presets; 50 carry a height-map payload of 100 to 676 bytes |
| `airbrush_spray` | A conical spray tip with splat statistics rather than a footprint. | carried by the same erodible-tip parameter block |

A tip is identified by a stable id and a name; the id — not the name — is the reference used by layer
payloads, tool presets, and dual-brush references, because a shipped library contains duplicate names
(three presets sharing one name were observed in a single library).

**[STU-RAS-127] PRIMARY TIP PARAMETER CONTRACT.** Every brush tip MUST carry these parameters with
these contracts. `declared` bounds come from the shipped brush-control surface; `observed` ranges are
the spread across shipped presets and are evidence only ([STU-RAS-107]).

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed range | classification |
|---|---|---|---|---|---|---|---|---|---|
| `diameter` | 1 | 5000 | 1 | 5000 | unknown | pixels | 1 | 1.0 .. 504.0 | hard declared, soft declared |
| `hardness` | 0 | 100 | 0 | 100 | unknown | percent | 1 | 0.0 .. 100.0 | hard declared, soft declared |
| `angle` | -180 | 180 | -180 | 180 | 0.0 | degrees | 1 | -102.0 .. 90.0 | bounds `derived` from the angular domain; default `derived` (identity) |
| `roundness` | 0 | 100 | 0 | 100 | 100.0 | percent | 1 | 0.0 .. 100.0 | bounds `derived` from the percentage domain; default `derived` (identity) |
| `spacing` | unknown | unknown | 1 | 1000 | 25.0 | percent | 1 | 1.0 .. 190.0 | spacing above 100 percent is legal and shipped; hard max undeclared |
| `spacing_enabled` | — | — | — | — | true | none | — | true in 637 of 643 | boolean |
| `flip_x`, `flip_y` | — | — | — | — | false | none | — | false in all 643 | boolean |
| `tip_shape_index` | 0 | 9 | — | — | 0 | count | 0 | 0 .. 9 | ten built-in parametric tip footprints; `observed` |
| `sampled_tip_id` | — | — | — | — | absent | none | — | — | artifact reference, required when `tip_kind = sampled` |

The `diameter` hard maximum of 5000 pixels and the `hardness` range of 0..100 percent are declared by
the shipped tip control and are the only two tip bounds any source declares. The `spacing` hard
maximum is `unknown` and MUST NOT be clamped to 100: shipped presets carry spacing up to 190 percent
on a primary tip and up to 279 percent on a dual tip, so a 100 percent clamp would break shipped art.

**[STU-RAS-128] ERODIBLE-TIP PARAMETER CONTRACT.** A tip with `tip_kind = erodible` or
`airbrush_spray` additionally carries:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed range | classification |
|---|---|---|---|---|---|---|---|---|---|
| `erodible_shape` | 0 | 1 | — | — | 0 | count | 0 | 0 .. 1 | tip solid shape selector; `observed` |
| `length_ratio` | unknown | unknown | unknown | unknown | 100.0 | percent | 1 | 100.0 only | `observed` |
| `tip_hardness` | 1 | 100 | 1 | 100 | unknown | percent | 1 | 1.0 .. 100.0 | `observed`; treat 1 as a floor, not zero |
| `airbrush_cutoff_angle` | unknown | unknown | unknown | unknown | unknown | degrees | 0 | 1.0 .. 45.0 | `observed` |
| `airbrush_granularity` | 0 | 100 | 0 | 100 | unknown | percent | 1 | 0.0 .. 100.0 | `observed` |
| `airbrush_streakiness` | unknown | unknown | unknown | unknown | unknown | percent | 1 | 1.0 only | `observed` |
| `airbrush_splat_size` | unknown | unknown | unknown | unknown | unknown | percent | 1 | 1.0 .. 72.0 | `observed` |
| `airbrush_splat_count` | unknown | unknown | unknown | unknown | unknown | count | 0 | 1.0 .. 200.0 | `observed` |
| `height_map_grid_size` | unknown | unknown | unknown | unknown | unknown | count | 0 | 5 .. 13 | square grid edge; payload size is grid² × 4 bytes |
| `height_map` | — | — | — | — | absent | none | — | 100 .. 676 bytes | artifact reference ([STU-RAS-116]) |
| `is_customized` | — | — | — | — | false | none | — | true in 7 presets | boolean; marks an operator-edited height map |

Erosion MUST be modelled as stroke-progressive tip wear with an explicit sharpen command that restores
the tip; without the sharpen command the erosion is not reversible within a stroke and the tool cannot
be used as shipped art expects.

**[STU-RAS-129] BRISTLE-PHYSICS PARAMETER CONTRACT.** A tip MAY carry a bristle physics model, gated by
a boolean. It is a distinct sub-engine from the erodible tip and the two MUST NOT be merged.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed range | classification |
|---|---|---|---|---|---|---|---|---|---|
| `physics_enabled` | — | — | — | — | false | none | — | true in 135 of 643 | boolean |
| `bristle_density` | unknown | unknown | 0.0 | 1.0 | unknown | normalized_0_1 | 2 | 0.01 .. 1.00 | `observed` |
| `bristle_length` | unknown | unknown | 0.0 | 4.0 | unknown | normalized_0_1 | 2 | 0.25 .. 2.52 | `observed`; exceeds 1.0, so it is a multiplier, not a fraction |
| `bristle_clumping` | unknown | unknown | 0.0 | 1.0 | unknown | normalized_0_1 | 2 | 0.25 only | `observed` |
| `bristle_thickness` | unknown | unknown | 0.0 | 2.0 | unknown | normalized_0_1 | 2 | 0.01 .. 2.00 | `observed` |
| `bristle_stiffness` | unknown | unknown | 0.0 | 1.0 | unknown | normalized_0_1 | 2 | 0.01 .. 1.00 | `observed` |

Every bound above is `unknown` as a hard bound; the soft bounds are `derived` from the observed spread
rounded outward to the next natural boundary, and that derivation is stated here rather than hidden so
it can be corrected when a declared bound is found. Clamping to the observed spread is forbidden.

**[STU-RAS-130] DYNAMICS CHANNEL MODEL — TWO SERIALIZATIONS, ONE STUDIO PRIMITIVE.** A dynamics channel
makes one brush parameter vary along a stroke. The two captured source families model it differently
and Studio MUST implement the union as one `StudioBrushDynamics` record, because discarding either
half loses shipped behaviour:

- The first family models a channel as `{ control, fade_step, jitter, minimum }`: `control` is an
  integer selecting the driving signal, `fade_step` is the step count over which a fade control
  completes, `jitter` is the random spread in percent, `minimum` is the floor the parameter may not
  fall below.
- The second family models a channel as `{ base_value, variance, controller, variance_mode,
  time_limit_ms, reverse, response_curve }`, where `response_curve` is an explicit spline —
  a point count, a linear flag, an x array, a y array and a spline-coefficient array — and where a
  curve may be shared by reference between several channels of the same brush.

`StudioBrushDynamics` MUST carry all of it: `base_value`, `variance`, `jitter`, `minimum`,
`control` (the driving-signal enumerator), `variance_mode`, `fade_step`, `time_limit_ms`, `reverse`,
and `response_curve`. The response curve is normative and MUST NOT be reduced to a preset easing
list: shipped brushes carry curves with 2 to 11 points including a squared-response size curve stored
as eleven evenly spaced x values with `y = x²` plus eleven spline coefficients, and an identity curve
stored as two points. Curve sharing MUST be preserved on import and export by reference, not by
duplication, so that editing a shared curve changes every channel that references it exactly as it
does in the source document.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Field | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `control` | 0 | unknown | — | — | 0 (`off`) | enum | 0 | 0..6 and 8, 9 in one family; 0..4, 6..9 in the other | see [STU-RAS-131] |
| `jitter` | 0 | unknown | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 771.0 | hard max undeclared; a shipped scatter jitter of 771 percent proves 100 is not a limit |
| `minimum` | 0 | 100 | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 100.0 | `observed` |
| `fade_step` | 1 | unknown | 1 | 100 | 25 | count | 0 | 1 .. 100 | `observed`; 25 is the modal shipped value and the stated `derived` default |
| `variance` | unknown | unknown | unknown | unknown | 0.0 | normalized_0_1 | 6 | — | `derived` identity default |
| `variance_mode` | 0 | 2 | — | — | 0 | enum | 0 | 0, 1, 2 | `observed`; three modes |
| `time_limit_ms` | unknown | unknown | unknown | unknown | 100.0 | milliseconds | 1 | 5, 12, 20, 24, 32, 40, 100, 400 | default `observed` at 100.0 on 5,412 of 5,467 channel instances |
| `reverse` | — | — | — | — | false | none | — | true and false both shipped | boolean |

**[STU-RAS-131] DYNAMICS CHANNELS AND DRIVING SIGNALS.** Studio MUST expose an independent
`StudioBrushDynamics` channel for each of these parameters. Eleven channels are normative; a channel
that is off costs nothing but MUST still exist in the schema so a model can set it.

`size`, `angle`, `roundness`, `scatter`, `count`, `opacity`, `flow`, `wetness`, `mix`, `texture_depth`,
`colour`. Two further channels — `tip_curvature` and `stroke_acceleration` — are normative for tips
that carry them, and the second source family additionally drives `scale_x`, `scale_y`, `hue_shift`,
`saturation_shift` and `luminance_shift` as dynamics rather than as scalars; those five MUST also be
channels, giving eighteen in total.

The driving-signal enumeration is closed: `off = 0`, `fade = 1`, `pen_pressure = 2`, `pen_tilt = 3`,
`stylus_wheel = 4`, `rotation = 5`, `initial_direction = 6`, `direction = 7`, `initial_rotation = 8`,
`velocity = 9`. Values above 9 seen in the captured data are composite selectors and MUST be decoded
as a signal plus a modifier flag rather than as new signals; an unrecognized selector on import
produces an unsupported-value receipt ([STU-RAS-162]).

Three tip-level dynamics limits sit outside the per-channel record and MUST be separate fields:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `minimum_diameter` | 0 | 100 | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 75.0 | `observed`; floor for the size channel |
| `minimum_roundness` | 1 | 100 | 1 | 100 | 25.0 | percent | 1 | 1.0 .. 100.0 | `observed`; floor for the roundness channel |
| `tilt_scale` | unknown | unknown | 0 | 200 | 100.0 | percent | 1 | 60.0 .. 200.0 | `observed`; scales the tilt signal |

**[STU-RAS-132] SCATTER AND DUAL TIP.** Scatter and dual tip are two sub-engines, each gated by its own
boolean, each with its own parameters, and the dual tip carries a complete second tip and its own
scatter. Collapsing the dual tip into a "secondary texture" is forbidden; 138 of 643 shipped presets
in one library depend on it.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `scatter_enabled` | — | — | — | — | false | none | — | true in 65 of 643 | boolean |
| `scatter_both_axes` | — | — | — | — | false | none | — | true in 30 of 65 | boolean |
| `scatter_count` | 1 | unknown | 1 | 16 | 1 | count | 0 | 1 .. 8 | `observed` |
| `dual_tip_enabled` | — | — | — | — | false | none | — | true in 138 of 643 | boolean |
| `dual_tip` | — | — | — | — | absent | none | — | — | a complete `StudioBrushTip` per [STU-RAS-127] |
| `dual_tip_blend_mode` | enum | enum | — | — | `multiply` | none | — | multiply, colour burn, darken, overlay, hard mix, linear burn observed | a `StudioBlendMode` value (§12) |
| `dual_tip_count` | 1 | unknown | 1 | 16 | 1 | count | 0 | 1 .. 10 | `observed` |
| `dual_tip_flip` | — | — | — | — | false | none | — | true in 8 of 138 | boolean |
| `dual_tip_spacing` | unknown | unknown | 1 | 1000 | 100.0 | percent | 1 | 1.0 .. 279.0 | `observed` |

**[STU-RAS-133] TEXTURE SUB-ENGINE.** A brush MAY be textured by a `StudioPattern`; 107 of 643 shipped
presets are. The texture is referenced by pattern id, not by name.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `texture_enabled` | — | — | — | — | false | none | — | true in 107 of 643 | boolean |
| `texture_pattern_id` | — | — | — | — | absent | none | — | — | `StudioPattern` reference |
| `texture_scale` | unknown | unknown | 1 | 1000 | 100.0 | percent | 1 | 1.0 .. 280.0 | `observed` |
| `texture_depth` | 0 | 100 | 0 | 100 | 100.0 | percent | 1 | 0.0 .. 100.0 | `observed` |
| `texture_minimum_depth` | 0 | 100 | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 100.0 | `observed` |
| `texture_brightness` | unknown | unknown | -150 | 150 | 0.0 | count | 0 | -150.0 .. 53.0 | `observed`; signed, unitless |
| `texture_contrast` | unknown | unknown | -100 | 100 | 0.0 | count | 0 | -50.0 .. 100.0 | `observed`; signed, unitless |
| `texture_blend_mode` | enum | enum | — | — | `height` | none | — | height, multiply, subtract, linear height, colour burn, linear burn observed | includes two height-field modes that are texture-only and are not layer blend modes |
| `texture_invert` | — | — | — | — | false | none | — | true in 26 of 107 | boolean |
| `texture_each_tip` | — | — | — | — | true | none | — | true in 97 of 107 | boolean |
| `texture_protect` | — | — | — | — | false | none | — | false in all observed | boolean; locks texture registration across tools |

The two height-field texture blend modes are normative additions to `StudioBlendMode` marked
texture-only in the applicability metadata of [STU-RAS-155]; they are not available on layers.

**[STU-RAS-134] COLOUR DYNAMICS.** Colour dynamics vary the painted colour along the stroke. 17 of 643
shipped presets use it, and it is the only sub-engine whose parameters are per-tip-or-per-stroke
switchable.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `colour_dynamics_enabled` | — | — | — | — | false | none | — | true in 17 of 643 | boolean |
| `foreground_background_jitter` | 0 | 100 | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 100.0 | `observed` |
| `hue_jitter` | 0 | 100 | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 87.0 | `observed` |
| `saturation_jitter` | 0 | 100 | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 75.0 | `observed` |
| `brightness_jitter` | 0 | 100 | 0 | 100 | 0.0 | percent | 1 | 0.0 .. 36.0 | `observed` |
| `purity` | -100 | 100 | -100 | 100 | 0.0 | percent | 1 | 0.0 only | `observed`; signed saturation bias |
| `apply_per_tip` | — | — | — | — | true | none | — | true in 16 of 17 | boolean; false means once per stroke |

**[STU-RAS-135] WET MIXING AND BRUSH POSE.** The mixer sub-engine and the pose override are separate
and both are normative.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `wetness` | 0 | 100 | 0 | 100 | unknown | percent | 0 | 0.0 .. 100.0 | `observed` |
| `dryness` (load) | 1 | 100 | 1 | 100 | unknown | percent | 0 | 1.0 .. 100.0 | `observed` |
| `mix` | 0 | 100 | 0 | 100 | unknown | percent | 0 | 0.0 .. 100.0 | `observed` |
| `flow` | 0 | 100 | 0 | 100 | 100.0 | percent | 0 | 5.0 .. 100.0 | `observed`; `derived` identity default |
| `opacity` (tool) | 0 | 100 | 0 | 100 | 100.0 | percent | 0 | 30.0 .. 100.0 | `observed`; `derived` identity default |
| `auto_load_after_stroke` | — | — | — | — | unknown | none | — | true in 41 of 82 | boolean |
| `auto_clean_after_stroke` | — | — | — | — | unknown | none | — | true in 69 of 82 | boolean |
| `load_solid_colour_only` | — | — | — | — | false | none | — | true in 2 of 82 | boolean |
| `sample_all_layers` | — | — | — | — | false | none | — | true in 20 of 82 | boolean |
| `reservoir_state` | — | — | — | — | — | none | — | — | the wet reservoir is stroke state, persisted with the tool preset |
| `pose_override_enabled` | — | — | — | — | false | none | — | true in 5 of 643 | boolean |
| `pose_pressure` | 0 | 100 | 0 | 100 | unknown | percent | 1 | 9.0 .. 21.0 | `observed` |
| `pose_tilt_x`, `pose_tilt_y` | -100 | 100 | -100 | 100 | 0 | count | 0 | -100 .. 100 | `observed` |
| `pose_angle` | -180 | 180 | -180 | 180 | 0 | degrees | 0 | 0 only | `derived` from the angular domain |

Wetness, dryness, mix and flow are the four mixer inputs and MUST be four fields; they carry no
decimal places in the captured serialization, so `precision = 0`.

**[STU-RAS-136] STROKE SMOOTHING (supersedes the "0–100" claim in [STU-RAS-023]).** Stroke smoothing is
not one 0..100 slider. It is one numeric strength plus five independent booleans, and there are two
distinct smoothing serializations in the captured data whose ranges do not agree:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `smoothing_enabled` | — | — | — | — | false | none | — | true in 112 of 138 | boolean |
| `smoothing_strength` | 0 | 100 | 0 | 100 | 0 | percent | 0 | 0 .. 23 | soft/hard 0..100 `derived` from the percentage domain; the observed spread across shipped presets is 0..23 and is NOT the limit |
| `smoothing_legacy_strength` | unknown | unknown | 0 | 10 | 0 | count | 0 | 0 .. 10 | a second, coarser smoothing field on a different serialization; retained for round-trip, not exposed as a second operator control |
| `pulled_string_mode` | — | — | — | — | false | none | — | false in all observed | boolean; the stroke follows only when the leash is taut |
| `stroke_catch_up` | — | — | — | — | true | none | — | true in all observed | boolean |
| `catch_up_on_stroke_end` | — | — | — | — | false | none | — | false in all observed | boolean |
| `zoom_compensation` | — | — | — | — | true | none | — | true in all observed | boolean |
| `pressure_smoothing` | — | — | — | — | false | none | — | false in all observed | boolean |

Shipping a single 0..100 smoothing slider satisfies none of the five behavioural modes and would not
reproduce a single shipped brush preset.

**[STU-RAS-165] SHIPPED BRUSH-LIBRARY CONTRACT.** Studio MUST ship a brush library of at least the
scale and shape of the captured field libraries and MUST expose the library as data, not as code.
The normative library contract is: brushes are organized into named categories; a brush belongs to
exactly one category and one of two application kinds, `raster` or `vector`; and every brush is a
complete `StudioBrushTip` plus its sub-engine records, so an operator or model can open any shipped
brush and edit every parameter. The captured field reference is 455 brushes across 16 categories
split 225 raster / 230 vector — Acrylics, Basic, Dry Media, Engraving, Gouaches, Image Brushes, Inks,
Markers, Masking, Oils, Patterns, Pencils, Pens, Sprays and Spatters, Textures, Watercolours — plus a
second library of 643 presets whose tips resolve to 3 brush classes. Studio's own category set is a
Handshake-native naming decision, not a copy of those names; what is normative is that the library is
categorized, that both application kinds exist, and that no brush is a black box.

**[STU-RAS-166] TOOL PRESETS.** A tool preset MUST capture a tool identity plus that tool's complete
option configuration, including its brush record where the tool is brush-driven, and MUST be
addressable by stable id. The captured evidence shows tool presets carry the tool's full option block
(for a marquee: constraint mode, aspect pair, fixed size pair with per-axis units, feather, style and
anti-alias), which is why a tool preset cannot be modelled as a name plus a brush reference.

---

### 7. Painting, Retouching, and Erasing Tools

**[STU-RAS-024]** Studio MUST provide the following deduped painting tools, all driven by the one brush
engine of §6:

*Derivation: catalogue table, splits per row; yields 7 microtasks, one per painting tool.*

| Tool | Normative behavior |
|---|---|
| Brush | Soft/antialiased strokes in the foreground color with full brush dynamics. |
| Pencil / pixel | Hard-edged, aliased, pixel-aligned strokes; supports auto-erase to background over foreground-colored pixels. |
| Mixer brush | Wet-paint mixing with canvas colors using wetness, load, mix, and flow ([STU-RAS-135]). |
| Color-replacement brush | Paints a replacement color over sampled colors while preserving underlying texture/luminosity. |
| Pattern stamp | Paints with a `StudioPattern`, optionally aligned and impressionist-styled. |
| History / snapshot brush | Paints pixels from a chosen history state or snapshot back into the image. |
| Art-history brush | Paints stylized strokes derived from a history state with style, fidelity, and area controls. |

**[STU-RAS-137] BRUSH-DRIVEN TOOL IDENTITY IS A CLOSED ENUMERATION.** Every brush-driven tool carries
a stable discriminant so a brush preset, a tool preset, a macro step, and a model command all name the
same tool: `pencil = 1`, `brush = 2`, `eraser = 3`, `background_eraser = 4`, `clone_stamp = 5`,
`pattern_stamp = 6`, `healing_brush = 7`, `history_brush = 8`, `art_history_brush = 9`, `smudge = 10`,
`blur = 11`, `sharpen = 12`, `dodge = 13`, `burn = 14`, `sponge = 15`, `colour_replacement = 16`.
Studio's own additions to this family — the selection brush, the refinement brush, the mask paint and
mask erase brushes, the inpaint brush, the median brush, the tone brush and the develop-local brush —
MUST receive discriminants above 16 and MUST NOT reuse a value in this list.

**[STU-RAS-138] HISTORY-SOURCED PAINTING PARAMETER CONTRACT.** The history brush and the art-history
brush paint from a named history state or snapshot, which MUST be an explicit typed reference to a
`StudioHistoryEntry`, not an implicit "previous state".

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `source_history_entry` | — | — | — | — | absent | none | — | — | required typed reference |
| `art_history_style` | 1 | unknown | 1 | 10 | 1 | enum | 0 | 1 .. 6 | six stroke styles shipped; the enumeration is closed at whatever Studio declares and MUST be named, not numbered, at the API |
| `art_history_tolerance` | 0 | 100 | 0 | 100 | 0 | percent | 0 | 0 only | `observed` |
| `art_history_area` | unknown | unknown | 0 | 500 | unknown | pixels | 0 | 20 .. 50 | `observed` |
| `pattern_stamp_pattern` | — | — | — | — | absent | none | — | — | `StudioPattern` reference by id |
| `pattern_stamp_aligned` | — | — | — | — | true | none | — | true in 3 of 4 | boolean |
| `pattern_stamp_impressionist` | — | — | — | — | false | none | — | true in 1 of 4 | boolean |

**[STU-RAS-026]** Studio MUST provide the following deduped retouching family as one primitive group
operating on pixel layers and, where the target allows, directly on placed-asset layers:

*Derivation: catalogue table, splits per row; yields 9 microtasks, one per retouching tool.*

| Tool | Normative behavior |
|---|---|
| Clone stamp | Paints exact pixel copies from a sampled source point with aligned sampling, sample-layer scope, cross-document sources, and a clone-source overlay. |
| Healing brush | Paints from a sampled source or pattern while matching texture, lighting, and shading of the destination. |
| Spot/blemish heal | Removes small blemishes by painting or single click, auto-sampling repair texture from the surroundings with no source point. |
| Patch | Repairs a drawn or selected region by dragging it over source pixels, in normal or content-aware mode with structure and colour adaptation. |
| Content-aware move | Moves or extends a selected object and content-aware fills the vacated area, with structure/colour adaptation and transform-on-drop. |
| Inpaint / object remove | Brushes over an unwanted region and synthesizes a fill from surrounding data using an on-device model. |
| Red-eye | Removes red flash reflections with pupil-size and darken controls while preserving eye detail. |
| Dust and scratches | Removes small defects by radius and threshold; also available as a live filter (14.9). |
| Frequency separation | Splits an image into low-frequency colour and high-frequency detail bands for retouching, and recombines them. |

**[STU-RAS-027]** Studio MUST provide the local tonal and detail retouch brushes as brush-engine tools:
Dodge (lighten) and Burn (darken) with tonal-range targeting, exposure and protect-tones; Sponge
(saturate/desaturate) with vibrance protection; and the local-effect brushes Blur (soften), Sharpen
(edge contrast with protect-detail), Median (edge-preserving noise reduction), and Smudge (smear pixels
in the drag direction, with finger painting). A tone brush that applies a non-destructive tonal shape
by painting, and an undo brush that paints an earlier history state back in, are members of this same
family and MUST be driven by the one brush engine.

**[STU-RAS-028]** Studio MUST provide the following deduped eraser family: a general eraser (erase to
transparency or background colour, in brush/pencil/block modes, and erase-to-history-state); a
background eraser (erase a sampled background colour to transparency while protecting a foreground
colour, with sampling modes and tolerance); a magic/flood eraser (erase all similar-coloured pixels to
transparency in one action, by tolerance and contiguity); and an undo brush. Where the target is a
non-destructive layer, [STU-RAS-170] governs.

**[STU-RAS-139] RETOUCH AND ERASE TOOL OPTION CONTRACTS.** The retouch family of [STU-RAS-026], the
local tonal brushes of [STU-RAS-027], and the eraser family of [STU-RAS-028] are retained unchanged in
scope. Their captured option contracts are:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Tool | Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|---|
| Smudge | `strength` | 0 | 100 | 0 | 100 | unknown | percent | 0 | `observed` 24 .. 100 |
| Smudge | `finger_painting` | — | — | — | — | false | none | — | boolean |
| Smudge | `sample_all_layers` | — | — | — | — | false | none | — | boolean |
| Dodge / Burn | `tonal_range` | enum | enum | — | — | `midtones` | none | — | `shadows` \| `midtones` \| `highlights` |
| Dodge / Burn | `exposure` | 0 | 100 | 0 | 100 | 50 | percent | 0 | `derived` midpoint default |
| Dodge / Burn | `protect_tones` | — | — | — | — | true | none | — | boolean |
| Sponge | `mode` | enum | enum | — | — | `desaturate` | none | — | `saturate` \| `desaturate` |
| Sponge | `flow` | 0 | 100 | 0 | 100 | 50 | percent | 0 | `derived` midpoint default |
| Sponge | `protect_vibrance` | — | — | — | — | true | none | — | boolean |
| Sharpen | `protect_detail` | — | — | — | — | true | none | — | boolean |
| Clone stamp | `aligned` | — | — | — | — | true | none | — | boolean |
| Clone stamp | `sample_scope` | enum | enum | — | — | `current_layer` | none | — | `current_layer` \| `current_and_below` \| `all_layers` |
| Clone stamp | `source_rotation` | -180 | 180 | -180 | 180 | 0 | degrees | 1 | `derived` from the angular domain |
| Eraser | `mode` | enum | enum | — | — | `brush` | none | — | `brush` \| `pencil` \| `block` |
| Eraser | `erase_to_history` | — | — | — | — | false | none | — | boolean |
| Magic / flood eraser | `tolerance` | 0 | 255 | 0 | 255 | 32 | levels_0_255 | 0 | `derived` from the 8-bit level domain; default `derived` |
| Magic / flood eraser | `contiguous` | — | — | — | — | true | none | — | boolean |
| Background eraser | `sampling` | enum | enum | — | — | `continuous` | none | — | `continuous` \| `once` \| `background_swatch` |
| Background eraser | `limits` | enum | enum | — | — | `find_edges` | none | — | `discontiguous` \| `contiguous` \| `find_edges` |
| Background eraser | `protect_foreground` | — | — | — | — | false | none | — | boolean |

Every `default` marked `derived` above is stated so an implementer does not guess; where a source
declared nothing, the derivation rule is either the identity of the operation or the midpoint of a
declared symmetric range, and the clause says which.

**[STU-RAS-170] ERASING ON A NON-DESTRUCTIVE LAYER ROUTES THROUGH MASKING.** Where the target of an
erase is a `placed_asset` or any layer whose pixels are not directly editable, the erase MUST be
applied to that layer's `StudioMask` rather than to its pixels, and the command receipt MUST say so.
Destroying pixels on a container layer is only permitted after an explicit rasterize ([STU-RAS-117]).

---

### 8. Fill, Gradient, Pattern, and the Preset Libraries

**[STU-RAS-025]** Studio MUST provide fill and gradient primitives: a bucket/flood fill filling similar
contiguous areas with a color or `StudioPattern` by tolerance and anti-alias; a gradient tool drawing
and editing `StudioGradient` fills interactively on layers, fill layers, and masks with on-canvas stop
handles; and gradient geometry modes Linear, Radial, Angle, Reflected, and Diamond. Foreground/
background fill and stroke-selection commands MUST be provided. Gradients and patterns authored here
are the same `StudioGradient`/`StudioPattern` primitives used by fill layers and effects.

**[STU-RAS-160] `StudioGradient` MODEL AND PARAMETER CONTRACT (extends [STU-RAS-025]).** A gradient is
two independent stop lists over one position axis — a colour stop list and a transparency stop list —
plus a form and an interpolation value. The two lists are independent: shipped gradients carry, for
example, two colour stops and twenty transparency stops on the same gradient. Modelling transparency
as a fourth channel of the colour stop is forbidden because it cannot represent that.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Field | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `form` | enum | enum | — | — | `stops` | none | — | `stops` \| `noise`; both shipped (318 stop-form and 11 noise-form gradients observed in one library of 329) |
| `stop.location` | 0 | 4096 | 0 | 4096 | unknown | gradient_position_0_4096 | 0 | `declared`: the position axis is an integer 0..4096, not a percentage; 4096 is the end. A stop has no factory default position — it is authored — so `default` is `unknown` by construction |
| `stop.midpoint` | 0 | 100 | 0 | 100 | 50 | percent | 0 | `observed` 13 .. 95; the default 50 is the modal shipped value and the identity midpoint |
| `stop.colour_type` | enum | enum | — | — | `user` | none | — | `user` \| `foreground` \| `background`; a stop may defer to the live foreground or background swatch |
| `transparency_stop.opacity` | 0 | 100 | 0 | 100 | 100 | percent | 1 | `observed` 0 .. 100 |
| `interpolation` | 0 | 4096 | 0 | 4096 | 4096 | gradient_position_0_4096 | 0 | smoothness; 4096 observed on every shipped stop-form gradient |
| `noise.colour_model` | enum | enum | — | — | `rgb` | none | — | `rgb` \| `hsb` \| `lab`; a noise gradient declares the space its randomness runs in |
| `noise.minimum[]`, `noise.maximum[]` | 0 | 100 | 0 | 100 | unknown | percent | 0 | four-element per-channel bounds; `observed` 0..43 and 0..100; no factory default was recoverable |
| `noise.roughness` | 0 | 4096 | 0 | 4096 | unknown | gradient_position_0_4096 | 0 | `observed` 696 .. 4096 |
| `noise.seed` | 0 | 4294967295 | — | — | random | count | 0 | `observed` 33254481 .. 2078578623; a seed is REQUIRED so the gradient is deterministic ([STU-CON-007] property (c)) |
| `noise.restrict_colours` | — | — | — | — | false | none | — | boolean |
| `noise.add_transparency` | — | — | — | — | false | none | — | boolean |
| `geometry` | enum | enum | — | — | `linear` | none | — | `linear` \| `radial` \| `angle` \| `reflected` \| `diamond` \| `shape_burst` |
| `reverse`, `dither`, `align_with_layer` | — | — | — | — | false / false / true | none | — | booleans |
| `angle` | -180 | 180 | -180 | 180 | 0 | degrees | 1 | `observed` -175 .. 180 |
| `scale` | unknown | unknown | 10 | 150 | 100 | percent | 1 | `observed` 10 .. 150 |
| `offset_x`, `offset_y` | unknown | unknown | -100 | 100 | 0 | percent | 6 | `observed` -35.05 .. 56.28 |

The `shape_burst` geometry is a sixth mode observed only as a stroke-effect gradient type; it is
normative and MUST NOT be dropped in the collapse to five geometry modes named in [STU-RAS-025].

**[STU-RAS-161] `StudioPattern` MODEL.** A pattern is a named, id-addressed tile bitmap with a declared
colour mode and pixel dimensions, stored in the artifact tier ([STU-RAS-116]) and referenced by id.
The captured field library carries 312 patterns whose tiles run from 1×1 to 946×946 pixels in three
colour modes — RGB (178), Grayscale (108) and Indexed (26) — proving that a pattern is not
RGB-only and that a 1-pixel tile is legal. A pattern reference MUST carry both the id and the name;
the id is authoritative and the name is a label.

**[STU-RAS-162] UNSUPPORTED-VALUE RECEIPTS.** Any import that encounters an enumerator, a preset
family, a parameter, or a payload Studio cannot represent MUST emit a typed unsupported-value receipt
naming the surface, the key, the raw value, and the disposition (`dropped` / `approximated` /
`preserved_opaque`), and MUST NOT silently coerce or discard it. The receipt is part of the
`StudioImportProfile` decode result (14.13) and is inspectable by a model.

**[STU-RAS-167] PRESET FAMILIES ARE FIRST-CLASS, TYPED, AND SEPARATELY PANELLED.** Studio MUST ship
these preset families as distinct typed registries, each with its own create / rename / duplicate /
delete / reorder / import / export commands and its own panel. They MUST NOT be collapsed into one
kind-filtered library panel: that consolidation is only usable if a panel can be open more than once
simultaneously, which Studio does not require, so the merge is deferred debt rather than a
simplification.

*Derivation: catalogue table, splits per row; yields 12 microtasks, one per preset family registry.*

| Primitive family | Primitive | Captured field scale | Structural contract |
|---|---|---|---|
| Brushes | `StudioBrushTip` + sub-engines | 643 presets in one library, 455 in another across 16 categories | [STU-RAS-126] .. [STU-RAS-136], [STU-RAS-165] |
| Gradients | `StudioGradient` | 329 | see [STU-RAS-160] |
| Patterns | `StudioPattern` | 312 | see [STU-RAS-161] |
| Layer styles | `StudioEffectStack` entry in `StudioStyleRegistry` | 370 | [STU-RAS-156] .. [STU-RAS-158] |
| Custom shapes | `StudioVectorPath` library entry | 725 | id + name + bounds; owned by 14.5, registered here as a raster-reachable fill/stamp source |
| Swatches | `StudioSwatch` | 8,901 across 24 containers | see [STU-RAS-168] |
| Colour books | named spot-colour library | 5,243 entries across 12 books | see [STU-RAS-168] |
| Contours | `StudioContour` | 42 | see [STU-RAS-158] |
| Tool presets | tool identity + full option block | 63 | see [STU-RAS-166] |
| Macros / actions | `StudioMacro` | 149 macros carrying 1,543 steps over 102 distinct step kinds | see [STU-RAS-169] |
| 3D LUTs | `StudioColorLookup` | 27 (20 cube-class, 7 look-class) | consumed by the colour-lookup adjustment (§9) |
| Adjustment presets | per-`StudioAdjustment` preset | 153 across 21 adjustment types in one source; 173 single-adjustment preset files in another | see [STU-RAS-153] |

**[STU-RAS-168] SWATCH AND COLOUR-BOOK MODEL.** A `StudioSwatch` MUST carry its colour components
*and* the colour space those components are in; it is not an RGB triple. The captured field library of
8,901 swatches is stored predominantly in Lab (3,505), CMYK (3,266) and RGB (2,004) with a small HSB
tail (29) and 84 entries whose space id did not resolve — those 84 are a known parse gap and MUST be
treated as an unsupported-value receipt case on import, not as RGB. A colour book entry additionally
carries a printed code distinct from its display name (for example a name and a separate code string
in the same record), and both MUST round-trip.

**[STU-RAS-169] MACRO / ACTION MODEL.** A recorded macro is an ordered list of steps; each step carries
an enabled flag, a dialog-suppression flag, a step-kind identifier, a display name, and an optional
typed parameter descriptor. A macro carries an optional function-key binding with modifier flags and a
colour label. The captured field reference is 149 macros containing 1,543 steps over 102 distinct step
kinds, with `set`, `make`, `select`, `duplicate`, `fill`, `convert_mode`, `delete`, `transform`,
`reset`, `merge_layers_new` and `merge_layers` the ten most frequent — that is, macro steps are
ordinary Studio commands, so the macro system MUST be built on the typed command surface of 14.16 and
MUST NOT define a parallel action vocabulary. A step whose command is unknown on import produces an
unsupported-value receipt and MUST be preserved opaque so the macro can still round-trip.

---

### 9. Adjustments

**[STU-RAS-035]** `StudioAdjustment` is the single canonical adjustment primitive; every adjustment kind
MUST be usable both as a non-destructive `adjustment`-kind layer (with built-in mask, clip-to-parent
option, and re-editable parameters) and as an explicit destructive apply-in-place command. Presets and
one-click adjustment creation MUST be supported.

**[STU-RAS-036]** Where a single named control exists in more than one source family, it MUST map to
exactly one `StudioAdjustment` kind per [STU-SECTION-003]; suite-specific extra parameters are merged
into that one kind's parameter set.

**[STU-RAS-037]** `StudioLiveFilter` is the single canonical live-filter primitive: a re-editable, maskable
filter hosted on a `live_filter`-kind layer or applied to a `placed_asset` container. The concrete
live-filter catalog is enumerated once in 14.9; 14.4 owns only the non-destructive live-filter-layer
mechanism, the attachment to placed-asset containers, per-filter masking, and the rule that any 14.9
filter that can run non-destructively MUST be available as a `StudioLiveFilter` with a destructive
apply-to-pixels form as well.

**[STU-RAS-151] THE `StudioAdjustment` KIND ENUMERATION.** The adjustment set is closed. Studio
declares its own contiguous discriminants because the two captured source families number the same
adjustments differently and irreconcilably (for example one numbers Levels 6 and Curves 7 inside a
flat layer-kind enum; the other numbers Levels 2 and Curves 52 in a dedicated adjustment registry).
Both per-source mapping tables are normative import/export data and live in the `StudioImportProfile`
for each format (14.13); neither source numbering is Studio's.

*Derivation: catalogue table, splits per row; yields 34 microtasks, one per `StudioAdjustment` kind.*

| Adjustment | Parameters (unit / precision) | Bounds | Notes |
|---|---|---|---|
| `brightness_contrast` | `brightness` (count, 0), `contrast` (count, 0), `use_legacy` (bool) | both `unknown` / soft -150..150 `derived` from the shipped control width of 6 digits | the legacy flag switches the transfer function and is a shipped field, not a compatibility shim |
| `levels` | `input_black`, `input_white`, `output_black`, `output_white` (normalized_0_1, 6), `gamma` (count, 2), per-channel variants of all five | inputs/outputs hard 0.0..1.0 `derived` from the normalized domain; `gamma` hard `unknown`, soft 0.1..9.99 `derived`, observed 0.5..2.0 | see [STU-RAS-152] on the canonical scale |
| `curves` | `channel` (enum), `points[]` of `{input, output, corner}` (levels_0_255, 0), up to five independent splines (composite + R + G + B + a fifth reserved) | point coordinates hard 0..255 `declared` by the curve serialization; point count hard `unknown`, observed 2..9 on shipped contour curves | on-image targeting and auto algorithms are commands on this kind |
| `exposure` | `exposure` (stops_ev, 2), `offset` (normalized_signed_1, 4), `gamma` (count, 2), `use_legacy` (bool) | all hard `unknown`; precisions `declared` by the control (2, 4, 2) | the differing precisions are declared and MUST NOT be unified |
| `vibrance` | `vibrance` (percent, 0), `saturation` (percent, 0) | hard `unknown`, soft -100..100 `derived` | |
| `hue_saturation` | per-band `hue` (degrees, 0), `saturation` (percent, 0), `lightness` (percent, 0); `colorize` (bool); band range edges | hard `unknown`; band count and range edges are data, not constants | one source stores the bands as parallel arrays plus a range array and a version field; all three MUST persist |
| `white_balance` | `temperature` (count, 0), `tint` (count, 0) | hard `unknown`, soft -100..100 `derived` | distinct from the develop white balance of [STU-RAW-110], which is in kelvin |
| `colour_balance` | nine values: `{shadows, midtones, highlights}` × `{cyan_red, magenta_green, yellow_blue}` (count, 0); `preserve_luminosity` (bool) | hard `unknown`, soft -100..100 `derived` from a 6-digit control | the nine-field shape is confirmed by both source families independently |
| `photo_filter` | `filter_colour` (colour), `density` (percent, 0), `preserve_luminosity` (bool) | `density` hard `unknown`, soft 1..100 `derived` from a 3-digit percent control | one source stores the colour as L/a/b plus a preset selector; both forms MUST decode |
| `black_and_white` | six weights `red`, `yellow`, `green`, `cyan`, `blue`, `magenta` (percent, 0); `tint_enabled` (bool), `tint_hue` (degrees, 0), `tint_saturation` (percent, 0) | weights hard `unknown`, soft -200..300 `derived`; hue hard 0..360 `declared` by the degree unit | the six-weight shape is confirmed by both source families |
| `channel_mixer` | an output-channel weight matrix, `constant` per output, `monochrome` (bool) | hard `unknown`, soft -200..200 `derived` | stored as a weight list; the list length is channel-count-dependent, not fixed |
| `selective_colour` | nine ranges `reds`, `yellows`, `greens`, `cyans`, `blues`, `magentas`, `whites`, `neutrals`, `blacks`, each a CMYK quadruple (percent, 0); `method` (enum `relative = 1` \| `absolute = 2`) | quadruples hard -100..100 `derived` from a 4-digit percent control; `method` `declared` | |
| `colour_lookup` | `lut` (artifact reference), `lut_kind` (enum `3dlut` \| `abstract` \| `device_link`), `dither` (bool) | — | consumes the 3D LUT preset family of [STU-RAS-167] |
| `ocio` | `source_space`, `destination_space`, `config` | — | parameters unobserved in shipped presets; block is `unknown` per [STU-RAS-153] |
| `gradient_map` | `gradient` (`StudioGradient`), `reverse` (bool), `dither` (bool) | — | |
| `recolour` | `hue` (degrees, 0), `saturation` (percent, 0), `lightness` (percent, 0) | hard `unknown` | |
| `split_toning` | `highlight_hue`, `shadow_hue` (degrees, 0), `highlight_saturation`, `shadow_saturation` (percent, 0), `balance` (percent, 0) | hue hard 0..360 `derived`; saturation hard 0..100 `derived`; balance soft -100..100 `derived` | subsumed by develop colour grading ([STU-RAW-116]) as its two-range degenerate case, but retained as a raster adjustment |
| `invert` | none | — | parameterless; block unobserved |
| `posterize` | `levels` (count, 0) | hard 2..255 `derived`; the control is 3 digits | |
| `threshold` | `level` (levels_0_255, 0), `false_colour`, `true_colour` (colour) | `level` hard 0..255 `declared` by the level domain | one source carries the two output colours explicitly; they are normative and default to black and white |
| `shadows_highlights` | `shadow_amount`, `shadow_tone_width` (percent, 0), `shadow_radius` (pixels, 0), `highlight_amount`, `highlight_tone_width` (percent, 0), `highlight_radius` (pixels, 0), `colour_correction` (count, 0), `midtone_contrast` (count, 0), `black_clip`, `white_clip` (percent, 3) | amounts hard 0..100 `derived`; radii hard `unknown`; clips precision 3 `declared` | the ten-field shape is `declared` by the source method signature |
| `tone_compression` (HDR tone map) | `method` (enum), `exposure`, `gamma`, `compression` (count, unknown), plus a transform block | hard `unknown` | ships eight presets; replaces the older "HDR toning" name |
| `tone_stretch` | unobserved | `unknown` | ships no presets; parameter block `unknown` per [STU-RAS-153] |
| `desaturate` | none | — | parameterless in-place command |
| `equalize` | none | — | parameterless in-place command |
| `auto_contrast`, `auto_levels`, `auto_tone` | none | — | parameterless analysis commands that write into `levels` / `curves` |
| `match_colour` | `luminance`, `colour_intensity`, `fade` (percent, 0), `neutralize` (bool), source selection | hard 0..100 `derived` | savable as a settings preset |
| `replace_colour` | `fuzziness` (percent, 0), sampled colour set, `hue`, `saturation`, `lightness` shift | `fuzziness` hard 0..200 `derived` | |
| `clarity` | `amount` (count, 0) | hard `unknown`; control is 4 digits | midtone local contrast |
| `dehaze` | `amount` (count, 0) | hard `unknown` | |
| `grain` | `amount` (count, 0), plus size and roughness | hard `unknown`; control is 4 digits | |
| `soft_proof` | `profile`, `intent` (enum), `black_point_compensation` (bool), `gamut_check` (bool) | `intent` `declared`, see [STU-RAS-150] | the four-field shape is `declared` by the shipped adjustment node |
| `normals` | unobserved | `unknown` | ships no presets; parameter block `unknown` |
| `gaussian_blur` (adjustment form) | `blur_amount` (count, 1) | hard `unknown`; precision 1 `declared` | the only blur that ships as an adjustment node rather than a live filter |

**[STU-RAS-152] ADJUSTMENT SCALE DECLARATION.** Several adjustments are serialized on different numeric
scales by different source families for the same control — levels black/white/output points appear as
0..255 integers in one family and as 0.0..1.0 reals in another, and the tonal points of a curve appear
as 0..255 integers regardless of document bit depth. Studio MUST declare exactly one canonical scale
per parameter, and it MUST be the bit-depth-independent one: `normalized_0_1` for levels input/output
points and for any tonal position that must survive an 8/16/32-bit conversion. The 0..255 form is a
display projection for 8-bit UI, produced by the presentation layer, never stored. Curve control-point
coordinates are the stated exception: they remain `levels_0_255` because the shipped curve and contour
serializations are integer 0..255 with a per-point corner flag, and changing that scale would break
round-trip of every shipped curve and contour preset. Every adjustment parameter MUST record its
canonical scale in its `StudioParameterSpec`, and every `StudioImportProfile` MUST carry the
conversion; a conversion done implicitly in a UI layer is a defect.

**[STU-RAS-153] UNOBSERVED ADJUSTMENT PARAMETER BLOCKS ARE STATED UNKNOWNS.** Four adjustment kinds —
`invert`, `ocio`, `normals` and `tone_stretch` — ship no presets in the captured field data, so their
parameter blocks were never observed and their fields are `unknown`. This is an absence of *evidence*,
never an absence of the adjustment: all four exist as registered adjustment nodes. An implementer MUST
implement them from their stated semantics and MUST record their parameters as a governed spec
enrichment once determined; inventing bounds for them is forbidden. The remaining twenty-one
adjustment kinds carry 153 shipped presets between them, from 2 (`threshold`) to 17 (`hue_saturation`)
each, and the preset values are `observed` evidence for those kinds' ranges under [STU-RAS-107].

---

### 10. Channels, Colour Modes, and Bit Depth

**[STU-RAS-031]** Studio MUST provide a Channels surface exposing per-document color channels, alpha
channels, and spot-color channels with visibility, editability, reorder, rename, and thumbnail
controls, and MUST support converting channel content to/from selections, spare channels, and masks
(the same mechanism as [STU-RAS-022]). Studio MUST provide Duplicate / Split / Merge channel
operations and the two channel-math operations: Apply-Image (blend a source layer/channel onto a
target with blend mode, opacity, invert, mask, and preserve-transparency) and Calculations (combine
two source channels with a blend operation, outputting a new channel, document, or selection).

**[STU-RAS-144] CHANNEL TYPE ENUMERATION AND CHANNEL-MATH CONTRACT.** `StudioChannel.kind` is closed:
`component = 1`, `masked_area_alpha = 2`, `selected_area_alpha = 3`, `spot_colour = 4`. The two alpha
kinds differ in polarity and MUST NOT be merged; a masked-area alpha stores the *masked* region while a
selected-area alpha stores the *selected* region, and conflating them inverts every imported saved
selection. Channel options carry `opacity` in percent with a 3-digit control, and a spot channel
carries `solidity` in percent with a 3-digit control in addition to its ink colour. Apply-Image and
Calculations each take `opacity` in percent, `scale` with precision 3 and `offset`, plus a source
channel reference, a blend mode, an invert flag and an optional mask.

**[STU-RAS-032]** Studio MUST support the following document color modes as canonical
`StudioColorProfile`-bound states, with explicit convert commands and mode-appropriate feature
availability: RGB, CMYK, Lab, Grayscale, Bitmap, Indexed Color, Duotone, and Multichannel. No implicit
device color is permitted; every value carries a `StudioColorProfile` reference ([STU-DOC-003]).

**[STU-RAS-145] DOCUMENT COLOUR MODE ENUMERATION AND THE CONVERT-MODE ASYMMETRY.**
`StudioDocument.colour_mode` is closed: `grayscale = 1`, `rgb = 2`, `cmyk = 3`, `lab = 4`,
`bitmap = 5`, `indexed = 6`, `multichannel = 7`, `duotone = 8`. The convert-mode command's target set
is deliberately *smaller* than the mode set: the captured conversion enumeration carries seven targets
— grayscale, rgb, cmyk, lab, bitmap, indexed, multichannel — and does **not** carry duotone. Studio
MUST reproduce this asymmetry rather than "fix" it: a document enters duotone mode through the duotone
ink-setup command ([STU-RAS-148]), which is a different operation from a colour-space conversion
because it defines ink plates and transfer curves rather than re-encoding colour. A convert-mode
command taking `duotone` as a target MUST be rejected with a typed error naming the ink-setup command.
The new-document mode set is smaller again — `grayscale`, `rgb`, `cmyk`, `lab`, `bitmap` — and a new
document's initial fill is `white = 1`, `background_colour = 2`, or `transparent = 3`.

**[STU-RAS-146] BITMAP (1-BIT) CONVERSION CONTRACT.** Converting to 1-bit MUST take a method
enumerator and its method-specific parameters: `threshold_50 = 1`, `pattern_dither = 2`,
`diffusion_dither = 3`, `halftone_screen = 4`, `custom_pattern = 5`. The halftone-screen method takes a
frequency (a 7-digit numeric control), an angle in degrees, and a dot shape from the closed set
`round = 1`, `diamond = 2`, `ellipse = 3`, `line = 4`, `square = 5`, `cross = 6`. Output resolution is
in `pixels_per_inch` with a `declared` default of 72.0. The custom-pattern method takes a
`StudioPattern` reference.

**[STU-RAS-147] INDEXED-COLOUR CONVERSION CONTRACT.** Converting to indexed colour MUST take:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `palette` | enum | enum | — | — | `exact` | none | — | closed twelve-member set: `exact = 1`, `mac_os = 2`, `windows = 3`, `web = 4`, `uniform = 5`, `local_perceptual = 6`, `local_selective = 7`, `local_adaptive = 8`, `master_perceptual = 9`, `master_selective = 10`, `master_adaptive = 11`, `previous = 12`; `declared` |
| `colours` | 2 | 256 | 2 | 256 | 256 | count | 0 | hard `derived` from the 8-bit index domain; default `declared` |
| `forced_colours` | enum | enum | — | — | `none` | none | — | `none = 1`, `black_and_white = 2`, `primaries = 3`, `web = 4`; `declared` |
| `transparency` | — | — | — | — | true | none | — | boolean; `declared` |
| `matte` | enum | enum | — | — | `none` | none | — | the seven-member matte set of [STU-RAS-117]; `declared` |
| `dither` | enum | enum | — | — | `diffusion` | none | — | `none = 1`, `diffusion = 2`, `pattern = 3`, `noise = 4`; `declared` |
| `dither_amount` | 1 | 100 | 1 | 100 | 75 | percent | 0 | hard `declared`; the default 75 is `declared` by one shipped save surface and MUST be used |
| `preserve_exact_colours` | — | — | — | — | false | none | — | boolean |

The resulting colour table MUST be editable and exportable as a `StudioColorTable` preset, and an
indexed document MUST expose it.

**[STU-RAS-148] DUOTONE INK CONTRACT.** Duotone MUST support one to four ink plates
(mono/duo/tri/quadtone), each carrying a named ink colour and its own transfer curve over the tonal
range, plus overprint colour definitions. The captured field library ships 114 duotone presets, which
are the evidence that the plate-plus-curve shape is the real serialization; a duotone modelled as a
colour pair without per-ink curves cannot load one of them.

**[STU-RAS-033]** Studio MUST support 8-, 16-, and 32-bit-per-channel documents. 32-bit floating point
MUST store HDR luminance beyond display range. Studio MUST provide: a merge-bracketed-exposures-to-HDR
operation (with ghost removal and tone-mapping); tone-mapping between 32/16/8-bit; a 32-bit HDR editing
workflow with a preview-exposure control; and the reduced tool/filter/blend-mode availability at 32-bit
MUST be surfaced as inspectable capability state rather than silent failure.

**[STU-RAS-149] BIT-DEPTH ENUMERATION AND CAPABILITY GATING.** `StudioDocument.bits_per_channel` is
closed and its discriminants are the depths themselves: `1`, `8`, `16`, `32`. The 1-bit depth is
reachable only through bitmap mode ([STU-RAS-146]). Every command, adjustment, live filter and blend
mode MUST declare its supported depth set as inspectable capability metadata; a command invoked at an
unsupported depth MUST fail with a typed capability error naming the depth and the supported set, and
MUST NOT silently no-op or silently promote the document. Preview exposure at 32-bit is a view-state
parameter in `stops_ev`, not a document edit, and MUST NOT enter the history.

**[STU-RAS-034]** Studio color management MUST provide working-space profile settings and mismatch policies
as saved presets; Assign-Profile (retag without changing values) and Convert-to-Profile (convert values
with a rendering-intent choice); embed-profile-on-save; soft-proof previewing an output condition
without converting; gamut warning; a color picker supporting HSB/RGB/Lab/CMYK/hex with an
out-of-gamut warning; and spot-color library selection from installed color books. Soft-proofing MUST
also be available as an in-stack `StudioAdjustment` (§9). Full color-pipeline authority is 14.8; the
raster surface consumes it and MUST NOT fork it.

**[STU-RAS-150] COLOUR-MANAGEMENT ENUMERATIONS AND COLOUR-COMPONENT CONTRACTS.** These enumerations are
closed and shared with 14.8. Rendering intent: `perceptual = 1`, `saturation = 2`,
`relative_colorimetric = 3`, `absolute_colorimetric = 4`. Proof source space: `document = 1`,
`proof = 2`. Profile assignment kind: `none = 1`, `working = 2`, `custom = 3` — and a custom profile is
selected by *name*, with the kind field left unset, which is a real serialization constraint that MUST
be honoured on import. Built-in working RGB spaces carry zero-based discriminants:
`adobe_rgb = 0`, `colormatch_rgb = 1`, `prophoto_rgb = 2`, `srgb = 3`. Document measurement units:
`pixels = 1`, `inches = 2`, `centimetres = 3`, `millimetres = 4`, `points = 5`, `picas = 6`,
`percent = 7`; type units are the three-member subset `pixels = 1`, `millimetres = 4`, `points = 5`.

Colour component contracts are `declared` and are the same everywhere in Studio:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Colour model | Components | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|---|
| RGB | red, green, blue | 0.0 | 255.0 | unknown | unknown | 255.0 each | count | 1 |
| Grayscale | gray | 0.0 | 100.0 | unknown | unknown | 0.0 | percent | 1 |
| CMYK | cyan, magenta, yellow, black | 0.0 | 100.0 | unknown | unknown | unknown | percent | 1 |
| Lab | L | 0.0 | 100.0 | unknown | unknown | unknown | count | 1 |
| Lab | a, b | -128.0 | 127.0 | unknown | unknown | unknown | count | 1 |
| HSB | hue | 0.0 | 360.0 | unknown | unknown | unknown | degrees | 1 |
| HSB | saturation, brightness | 0.0 | 100.0 | unknown | unknown | unknown | percent | 1 |

Every *hard* bound in that table is `declared`. No source declares a soft bound for any colour
component, so every soft bound is `unknown` and MUST be stored as `unknown` rather than mirrored from
the hard bound: a control that presents the full hard domain by default is a *decision* the
implementer records, not a fact this spec carries. Note that RGB components are declared as *floating point*
0.0..255.0, not as integers: rounding them to `u8` at the API boundary is a precision defect that
breaks round-trip of shipped style and gradient data, which carries values such as 239.996 and
240.00000089.

---

### 11. Transforms and Content-Aware Operations

**[STU-RAS-029]** Studio MUST provide the following deduped transform and reshaping primitives on layers,
selections, and placed-asset containers. Numeric and handle-based input MUST both be supported, and
every transform MUST carry explicit typed units per [STU-DOC-003].

*Derivation: catalogue table, splits per row; yields 10 microtasks, one per transform primitive.*

| Transform primitive | Normative behavior |
|---|---|
| Move | Moves selection or layer content, with auto-select and layer-bounds hover options. |
| Crop | Crops/expands canvas with ratio and absolute presets, overlay guides, straighten, delete-vs-hide cropped pixels, crop-to-selection, and content-aware fill of newly exposed areas on commit. |
| Perspective crop | Crops while correcting keystoned perspective to a straight-on rectangle via corner handles. |
| Free transform | Scale, rotate, skew, distort, and flip in one interactive operation with numeric entry and reference-point control. |
| Warp | Grid/handle mesh warp with warp presets and custom control-point deformation, including bezier mesh warp. |
| Puppet warp | Pin-and-deform mesh with density and expansion controls for organic reshaping. |
| Perspective warp | Multi-plane perspective reshaping/correction (single- and dual-plane). |
| Content-aware scale | Scales while protecting flagged content from distortion using a protection mask/channel. |
| Content-aware fill | Fills a selection by synthesizing plausible pixels from sampled source regions ([STU-RAS-143]). |
| Liquify | Localized mesh push-forward, push-left, twirl, pinch, punch, turbulence, mesh-clone, and reconstruct brushes, with freeze/thaw masking. |

**[STU-RAS-141] TRANSFORM REFERENCE POINT AND RESAMPLING ENUMERATIONS.** Every transform, canvas
resize, and image resize takes an explicit nine-member anchor: `top_left = 1`, `top_centre = 2`,
`top_right = 3`, `middle_left = 4`, `middle_centre = 5`, `middle_right = 6`, `bottom_left = 7`,
`bottom_centre = 8`, `bottom_right = 9`. Every resampling operation takes an explicit nine-member
method: `none = 1`, `nearest_neighbour = 2`, `bilinear = 3`, `bicubic = 4`, `bicubic_sharper = 5`,
`bicubic_smoother = 6`, `bicubic_automatic = 7`, `automatic = 8`, `preserve_details = 9`. The
`preserve_details` method additionally takes a noise-reduction amount in percent. Resampling MUST NOT
default silently: `automatic` is a *declared* choice that records which concrete method it resolved to
in the command receipt, so a replayed edit is deterministic ([STU-CON-007] property (c)).

**[STU-RAS-142] TRANSFORM, CROP AND TRIM PARAMETER CONTRACTS.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Operation | Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|---|
| Rotate / rotate canvas | `angle` | -360 | 360 | unknown | unknown | unknown | degrees | 2 | hard bounds `derived`; precision 2 `declared` by the rotate control; no soft bound declared |
| Free transform | `scale_x`, `scale_y` | unknown | unknown | unknown | unknown | unknown | percent | 2 | negative scale is legal and is how flip is expressed, so a non-negative hard minimum MUST NOT be assumed |
| Free transform | `skew_x`, `skew_y` | -89.9 | 89.9 | unknown | unknown | unknown | degrees | 2 | hard bounds `derived`; no soft bound declared |
| Free transform | `translate_x`, `translate_y` | unknown | unknown | unknown | unknown | unknown | document_unit | 2 | no bound declared by any source |
| Free transform | `warp_style` | enum | enum | — | — | unknown | none | — | closed sixteen-member set, `declared`; no source declares which member is the default: `none = 1`, `arc = 2`, `arc_lower = 3`, `arc_upper = 4`, `arch = 5`, `bulge = 6`, `shell_lower = 7`, `shell_upper = 8`, `flag = 9`, `wave = 10`, `fish = 11`, `rise = 12`, `fish_eye = 13`, `inflate = 14`, `squeeze = 15`, `twist = 16` |
| Canvas size | `width`, `height` | unknown | unknown | unknown | unknown | unknown | document_unit | 2 | no bound declared; plus the nine-member anchor of [STU-RAS-141] |
| Image size | `width`, `height` | unknown | unknown | unknown | unknown | unknown | document_unit | 2 | no bound declared |
| Image size | `resolution` | unknown | unknown | unknown | unknown | unknown | pixels_per_inch | 2 | no bound declared; plus the resample method of [STU-RAS-141] and its noise-reduction amount |
| Crop | `bounds`, `width`, `height` | unknown | unknown | unknown | unknown | unknown | document_unit | 2 | no bound declared |
| Crop | `angle` | unknown | unknown | unknown | unknown | unknown | degrees | 2 | no bound declared; plus `delete_cropped_pixels` (bool) and `content_aware_fill_on_expand` (bool) |
| Trim | `basis` | enum | enum | — | — | unknown | none | — | `declared`: `transparent_pixels = 0`, `top_left_pixel = 1`, `bottom_right_pixel = 9`; the non-contiguous discriminant 9 is real and MUST be preserved |
| Trim | `top`, `left`, `bottom`, `right` | — | — | — | — | unknown | none | — | booleans selecting which edges to trim |
| Offset | `horizontal`, `vertical` | unknown | unknown | unknown | unknown | unknown | pixels | 2 | no bound declared |
| Offset | `undefined_areas` | enum | enum | — | — | unknown | none | — | `declared`: `set_to_layer_fill = 1`, `wrap_around = 2`, `repeat_edge_pixels = 3` |
| Flip canvas / layer | `direction` | enum | enum | — | — | unknown | none | — | `declared`: `horizontal` \| `vertical` |
| Puppet warp | `density`, `expansion`, `pin_set` | unknown | unknown | unknown | unknown | unknown | count / pixels | 0 | no bound declared |
| Puppet warp | `mode` | enum | enum | — | — | unknown | none | — | the member set was not recovered by the capture and is `unknown`; it MUST be established by governed spec enrichment, not guessed |
| Liquify | brush `size`, `density`, `pressure`, `rate` | unknown | unknown | unknown | unknown | unknown | pixels / percent | 0 | no bound declared; driven by the §6 brush engine; freeze/thaw masks are `StudioMask` values |

**[STU-RAS-143] CONTENT-AWARE FILL CONTROL CONTRACT (extends [STU-RAS-030]).** Content-aware fill,
content-aware scale, content-aware crop-expand and inpaint/remove MUST have a native on-device
implementation in `studio-engine`. Any generative/provider-model variant is an optional adapter lane
(§14) and MUST NOT be the only implementation. The content-aware fill surface's control set is:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `sampling_area_overlay` | — | — | — | — | unknown | none | — | boolean plus an overlay colour and an `indicates` enum selecting whether the overlay marks the sampled or the excluded region; the overlay is inspectable state, required by [STU-RAS-001] obligation (c) |
| `sampling_overlay_opacity` | unknown | unknown | unknown | unknown | unknown | percent | unknown | unit `declared`; no bound, precision or default declared |
| `sampling_area_mode` | enum | enum | — | — | unknown | none | — | closed set: `auto` \| `rectangular` \| `custom`; plus a `sample_all_layers` boolean |
| `colour_adaptation` | enum | enum | — | — | unknown | none | — | closed set: `none` \| `default` \| `high` \| `very_high` |
| `rotation_adaptation` | enum | enum | — | — | unknown | none | — | closed set: `none` \| `low` \| `medium` \| `high` \| `full` |
| `scale`, `mirror` | — | — | — | — | unknown | none | — | booleans; allow the synthesizer to scale or mirror sampled patches |
| `additional_document_source` | — | — | — | — | absent | none | — | a typed reference plus a `flexibility` enum; a second document may be the sampling source, which is why the sampling source is a typed reference, not a rectangle |
| `output_brightness`, `output_contrast` | unknown | unknown | unknown | unknown | unknown | percent | unknown | post-fill harmonization; unit `declared` only |
| `output_cyan_red`, `output_magenta_green`, `output_yellow_blue` | unknown | unknown | unknown | unknown | unknown | percent | unknown | post-fill colour harmonization; unit `declared` only |
| `output_to` | enum | enum | — | — | unknown | none | — | closed set: `current_layer` \| `new_layer` \| `duplicate_layer` |

Every numeric bound on this surface is `unknown` in all four fields, and so is every default: the
source declares the control class and unit but no minimum, maximum, precision or default. The soft
bounds MUST NOT be filled in from the hard bounds, and neither MUST be filled in from the other.

**[STU-RAS-030]** Content-aware operations are native and on-device per [STU-RAS-143].

---

### 12. Blend Modes

**[STU-RAS-038]** `StudioBlendMode` is the single canonical blend-mode enum, shared by layers, groups, brush
tools, fills, effects and every other Studio domain.

**[STU-RAS-154] CANONICAL BLEND-MODE ENUMERATION.** The set is closed and the discriminants are
canonical. They are the discriminants carried by the captured source enumerations, which agree with
each other on every shared member, so an imported document round-trips without a mapping table.

*Derivation: catalogue table, splits per row; yields 30 microtasks, one per blend mode.*

| Blend mode (`StudioBlendMode`) | Discriminant | Group | Applicability |
|---|---|---|---|
| `pass_through` | 1 | structural | group layers only; the default group mode |
| `normal` | 2 | normal | all |
| `dissolve` | 3 | normal | all |
| `darken` | 4 | darken | all |
| `multiply` | 5 | darken | all |
| `colour_burn` | 6 | darken | all |
| `linear_burn` | 7 | darken | all |
| `lighten` | 8 | lighten | all |
| `screen` | 9 | lighten | all |
| `colour_dodge` | 10 | lighten | all |
| `linear_dodge_add` | 11 | lighten | all |
| `overlay` | 12 | contrast | all |
| `soft_light` | 13 | contrast | all |
| `hard_light` | 14 | contrast | all |
| `vivid_light` | 15 | contrast | all |
| `linear_light` | 16 | contrast | all |
| `pin_light` | 17 | contrast | all |
| `difference` | 18 | comparative | all |
| `exclusion` | 19 | comparative | all |
| `hue` | 20 | component | all |
| `saturation` | 21 | component | all |
| `colour` | 22 | component | all |
| `luminosity` | 23 | component | all |
| `behind` | 24 | normal | **tool only** — paints only transparent areas; not settable on a layer |
| `clear` | 25 | normal | **tool only** — paints to transparency; not settable on a layer |
| `hard_mix` | 26 | contrast | all |
| `lighter_colour` | 27 | lighten | all |
| `darker_colour` | 28 | darken | all |
| `subtract` | 29 | comparative | all |
| `divide` | 30 | comparative | all |

**[STU-RAS-155] BLEND-MODE APPLICABILITY IS INSPECTABLE METADATA, NOT SILENT FAILURE.**
`StudioBlendMode` MUST carry, per member, three applicability facts as queryable capability metadata:
(a) layer-settable versus tool-only — `behind` and `clear` are tool-only, `pass_through` is group-only,
and the captured layer-mode enumeration confirms this by omitting `behind` and `clear` from the layer
set while the tool-mode enumeration includes them; (b) supported document bit depths, since the mode
set narrows at 32-bit ([STU-RAS-149]); and (c) host applicability, because two texture-only height-field
modes exist ([STU-RAS-133]) that are legal on a brush texture and illegal on a layer. Setting an
inapplicable mode MUST fail with a typed capability error naming the mode and the applicable set; a
silent no-op is forbidden.

**[STU-RAS-039]** Additional comparative/intensity modes present in one source family but not another
MUST be included as `StudioBlendMode` values so no source blend behaviour is lost. They receive
discriminants above 30 in the order below, and their applicability metadata MUST be populated
per [STU-RAS-155]. They are enumerated here rather than named only in prose so that each carries
its own implementable contract exactly as the thirty members of [STU-RAS-154] do:

*Derivation: catalogue table, splits per row; yields 5 microtasks, one per additional blend mode.*

| Blend mode (`StudioBlendMode`) | Discriminant | Group | Applicability and compositing function |
|---|---|---|---|
| `average` | 31 | comparative | applicability `unknown`; the compositing function was not recovered by the capture and MUST be established by governed spec enrichment before implementation, not guessed |
| `negation` | 32 | comparative | applicability `unknown`; compositing function `unknown`, as above |
| `reflect` | 33 | intensity | applicability `unknown`; compositing function `unknown`, as above |
| `glow` | 34 | intensity | applicability `unknown`; compositing function `unknown`, as above |
| `erase` | 35 | normal | applicability `unknown`; compositing function `unknown`, as above. The name is the source family's own and its semantics MUST be confirmed before a discriminant is frozen |

The group column above is the clause's own "comparative/intensity" classification and is the only
property the capture supports; every other property of these five is a stated unknown
under [STU-RAS-106], so each row's first acceptance criterion is recovering its compositing function
and applicability set rather than implementing a guessed formula. One captured source family additionally
enumerates blend modes with a small dense integer set of its own (observed 0..21 on brush records);
that numbering is a per-format import concern and MUST be translated by the `StudioImportProfile`,
never adopted as Studio's.

---

### 13. Layer Effects and Advanced Blending

**[STU-RAS-040]** `StudioEffectStack` is the single canonical layer-effects/styles primitive. Studio MUST
implement the following non-destructive, re-editable, per-layer effect kinds, each re-orderable and
independently maskable: Bevel & Emboss, Stroke, Inner Shadow, Inner Glow, Satin, Colour Overlay,
Gradient Overlay, Pattern Overlay, Outer Glow, and Drop Shadow. A document-wide Global Light angle MUST
be shareable across shadow and bevel effects; a contour/gloss-contour editor MUST be provided. Effect
combinations MUST be saveable as reusable styles (a `StudioStyleRegistry` entry), and effects MUST be
copyable/pasteable between layers, scalable by percentage, hideable, removable, and convertible into
standalone pixel layers.

**[STU-RAS-156] EFFECTS ARE MULTI-INSTANCE (supersedes the one-instance-per-kind model implied
by [STU-RAS-040]).** A `StudioEffectStack` is an ordered list of effect instances, and several instances
of the *same* effect kind may coexist on one layer. This is not an enhancement; it is required to load
shipped art. The captured field library of 370 layer styles carries explicit multi-instance lists —
drop shadow observed with up to 4 simultaneous instances, stroke with up to 8, gradient overlay with up
to 8, inner shadow with up to 2, and colour overlay with up to 2 — alongside a separate single-instance
slot per kind that the same style may also populate. A model that allows one drop shadow per layer
cannot represent a style that ships with four, and there is no lossless way to collapse them.

Studio's normative model is therefore:

- `StudioEffectStack.effects` is an ordered list of `StudioEffect` instances. Order is authoritative
  and MUST be preserved; render order follows list order.
- Each instance carries `kind`, a stable instance id, `enabled`, `visible_in_editor`, and its
  kind-specific parameter block.
- The minimum instance capacity per kind that Studio MUST support is 10; the hard maximum is `unknown`
  and MUST NOT be enforced below 10.
- Import MUST merge a source's single-instance slot and its multi-instance list for the same kind into
  one ordered list, preserving both, and MUST NOT drop either. Export to a format carrying that
  split MUST reproduce it.
- The stack carries a stack-wide `scale` in percent (observed 100.0, `derived` default 100.0) and a
  stack-wide master enable, both of which apply to every instance.

**[STU-RAS-157] PER-EFFECT PARAMETER CONTRACTS.** Every effect instance carries the common block plus
its kind block. Bounds marked `unknown` were not declared by any source; the `observed` column is the
spread across the 370 captured shipped styles and is evidence only ([STU-RAS-107]).

Common block, present on every effect kind:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `enabled` | — | — | — | — | true | none | — | true on every shipped instance | boolean |
| `blend_mode` | enum | enum | — | — | per kind | none | — | see kind rows | closed `StudioBlendMode` set, §12 |
| `opacity` | 0 | 100 | unknown | unknown | per kind | percent | 1 | 0.0 .. 100.0 | hard bounds `derived`; no soft bound declared |
| `colour` | see [STU-RAS-150] | see [STU-RAS-150] | unknown | unknown | per kind | — | 1 | RGB floats 0.0..255.0 | component bounds are the colour-model contract of [STU-RAS-150]; a colour MAY additionally carry a spot-book id and key |
| `contour` | see [STU-RAS-158] | see [STU-RAS-158] | unknown | unknown | `linear` | — | — | 2..9 points | named curve plus point list |
| `anti_alias` | — | — | — | — | false | none | — | true on a minority | boolean |
| `noise` | 0 | 100 | unknown | unknown | 0.0 | percent | 1 | 0.0 .. 79.0 | hard bounds `derived`; no soft bound declared |
| `angle` | -180 | 180 | unknown | unknown | 120.0 | degrees | 1 | -180.0 .. 180.0 | hard bounds `derived`; no soft bound declared |
| `use_global_light` | — | — | — | — | true | none | — | true and false both shipped | boolean |

Kind blocks:

*Derivation: catalogue table, splits per row; yields 11 microtasks, one per layer-effect kind.*

| Effect kind | Kind-specific parameters | unit | hard bounds | observed |
|---|---|---|---|---|
| `bevel_emboss` | `style` (`inner_bevel` \| `outer_bevel` \| `emboss` \| `pillow_emboss` \| `stroke_emboss`), `technique` (`smooth` \| `chisel_hard` \| `chisel_soft`), `direction` (`up` \| `down`), `depth`, `size`, `soften`, `altitude`, `gloss_contour`, `highlight_mode`/`_colour`/`_opacity`, `shadow_mode`/`_colour`/`_opacity`, `anti_alias_gloss`, `use_contour`, `use_texture` | percent / pixels / degrees | `depth` `unknown`; `size` `unknown`; `soften` `unknown`; `altitude` 0..90 `derived` | depth 1.0..1000.0 %, size 0.0..185.0 px, soften 0.0..16.0 px, altitude 0.0..75.0°, highlight/shadow opacity 0.0..100.0 % |
| `bevel_emboss` texture sub-block | `pattern`, `scale`, `depth`, `invert`, `align_with_layer`, `phase_x`, `phase_y` | percent / count | `unknown` | scale 23.0..1000.0 %, depth -272.0..301.0 % (signed, so an inverted texture is a negative depth, not a flag) |
| `stroke` | `size`, `position` (`inside` \| `outside` \| `centre`), `fill_type` (`solid` \| `gradient` \| `pattern`), `colour` \| `gradient` \| `pattern`, gradient `type`/`angle`/`scale`/`reverse`/`dither`/`align`/`offset`, pattern `scale`/`phase`/`link`, `overprint` | pixels / percent | `size` `unknown` | size 1.0..161.0 px, pattern scale 51.0..184.0 %, gradient interpolation 1311..4096 |
| `inner_shadow` | `distance`, `choke`, `size` | pixels | all `unknown` | distance 0.0..30000.0 px, choke 0.0..100.0 px, size 0.0..109.0 px — the 30000 px distance is real shipped data and proves no small clamp exists |
| `inner_glow` | `technique` (`softer` \| `precise`), `source` (`edge` \| `centre`), `choke`, `size`, `range`, `jitter`, `colour` or `gradient` | pixels / percent | all `unknown` | choke 0.0..50.0 px, size 0.0..213.0 px, range 27.0..100.0 %, jitter 0.0..69.0 % |
| `outer_glow` | `technique`, `spread`, `size`, `range`, `jitter`, `colour` or `gradient` | pixels / percent | all `unknown` | spread 0.0..45.0 px, size 0.0..65.0 px, range 41.0..87.0 %, jitter 0.0..19.0 % |
| `satin` | `distance`, `size`, `invert` | pixels | all `unknown` | distance 3.0..250.0 px, size 3.0..188.0 px |
| `colour_overlay` | `colour` | — | — | may carry a spot-book id and key alongside RGB |
| `gradient_overlay` | `gradient`, `type`, `reverse`, `dither`, `align_with_layer`, `angle`, `scale`, `offset_x`, `offset_y` | percent / degrees | `scale` `unknown` | scale 10.0..150.0 %, offset -35.05..56.28 % |
| `pattern_overlay` | `pattern`, `scale`, `phase_x`, `phase_y`, `link_with_layer` | percent / count | `scale` `unknown` | scale 13.0..1000.0 %, phase -1789..137 |
| `drop_shadow` | `distance`, `spread`, `size`, `layer_knocks_out` | pixels | all `unknown` | distance 0.0..60.0 px, spread 0.0..100.0 px, size 0.0..65.0 px |

**[STU-RAS-158] `StudioContour` MODEL AND GLOBAL LIGHT.** A contour is a named curve carried by value
inside the effect that uses it *and* registerable as a reusable preset. Its serialization is a point
list where each point is `{input, output, corner}` with `input` and `output` in `levels_0_255`
(hard 0..255, `declared`) and `corner` a boolean that suppresses smoothing at that point. The captured
field library ships 42 contours whose point counts run 2 to 9. A contour reference MUST carry the name
*and* the resolved point list, because shipped styles carry contours named `Custom` whose points exist
only in the style. Global Light is a document-level `{angle, altitude}` pair in degrees, shared by
every effect whose `use_global_light` is true; changing it MUST update every subscribing effect in one
history entry.

**[STU-RAS-041]** Studio MUST provide advanced/conditional blending on every layer as canonical
`StudioLayer` fields: Blend-If ranges, knockout, per-channel blend enablement, and the
interior-effects / clipped-effects / transparency-shapes-layer toggles. Fill-versus-opacity
([STU-RAS-115]) is part of this advanced-blending surface.

**[STU-RAS-159] BLEND-IF RANGE MODEL — EIGHT VALUES PER CHANNEL, NOT FOUR.** A blend range is stored per
channel as **eight** values, not four: source black start, source black end, source white start,
source white end, and the same four for the destination. The paired values are what make a Blend-If
slider *split* and feather; collapsing each pair to one number turns every split slider into a hard
cut. Every value is in `levels_0_255` with hard bounds 0..255 (`declared`). The defaults are
`source_black = (0, 0)`, `source_white = (255, 255)`, `destination_black = (0, 0)`,
`destination_white = (255, 255)` — that is, the identity range — and these are `derived`. A blend-range
entry additionally carries a channel reference, so the set is per-channel and the composite gray entry
is one member of it, not a separate mechanism.

---

### 14. Provider, Cloud, and Generative Posture

**[STU-RAS-044]** Studio's raster domain is local-first: every raster primitive named in 14.4 — including
the ML-backed ones (object select, subject select, sky select, sky replacement, inpaint/remove,
content-aware fill/scale, HDR merge, noise/stack reduction, denoise, upscale) — MUST have a native,
on-device implementation in `studio-engine` that runs offline with no account, sign-in, or cloud call,
per [STU-OVR-002]. On-device inference is a core Studio capability, not a provider dependency.

**[STU-RAS-045]** Cloud-, account-, or vendor-generative features are NOT core Studio features. They are
recorded here as normative rows and MUST be either an optional `StudioModelAdapter` lane (14.14) or
intentionally omitted; none may become a runtime dependency of any core raster primitive.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Source-suite feature (provenance) | Studio posture |
|---|---|
| Text-prompted generative fill / generative expand | Optional adapter lane over a pluggable local or remote generative backend; the native content-aware fill/inpaint primitive ([STU-RAS-143]) is the non-optional baseline. |
| Cloud "neural"/generative filter items requiring a vendor cloud | Adapter lane per filter where a local model exists; otherwise intentionally omitted. On-device filters branded "neural" upstream ship as native `StudioLiveFilter`/adjustments. |
| Vendor generative image service | Adapter lane only; never a dependency; never a Studio brand or panel name. |
| Cloud-backed distraction/object removal | Adapter acceleration only; the on-device inpaint/remove primitive is the baseline. |
| Vendor cloud asset libraries | Replaced by native local `placed_asset` links and the Handshake asset library; vendor-cloud sync is an optional adapter. |
| Vendor cloud documents and cloud version history | Replaced by native local `StudioDocument` history/undo (14.19); vendor cloud storage is omitted. Collaboration is native CRDT (14.16/14.17). |
| Vendor share-for-review / cloud project spaces | Replaced by native Studio collaboration/review surfaces; vendor cloud is omitted. |

**[STU-RAS-046]** Any adapter lane MUST be opt-in, MUST route through the sandbox → validation →
PromotionGate lifecycle ([STU-RAS-003]), MUST be attributable in the audit log via `KernelActor`, and
MUST degrade to the native on-device primitive (or a clear "adapter unavailable" state) when the
provider is absent — it MUST NOT block or break any core raster workflow.

**[STU-RAS-164] METERED THIRD-PARTY MODELS ARE AN ADAPTER SHAPE, NOT A TOOL SHAPE.** The captured
field evidence shows a shipped denoise surface whose control set is a *model selector*, a partner-model
attribution line, a model-error state, and a consumable-credit readout — that is, the vendor ships a
metered third-party model behind what looks like a filter. Studio MUST NOT reproduce that shape as a
core tool. Where a `StudioModelAdapter` is metered, externally hosted, or third-party, the adapter
surface MUST expose, as inspectable state and in every command receipt: the adapter identity, the model
identity and version, whether execution was local or remote, and any consumption the run incurred. A
core Studio tool MUST NOT display a credit balance, and a run that would consume a metered resource
MUST NOT start without an explicit typed acknowledgement in the command input.

---

### 15. Diagnostics, Export Touchpoints, and Domain Authority

**[STU-RAS-049]** Studio MUST provide the raster tonal-diagnostics feedback surface: a live histogram
(per-composite and per-channel, with clip warnings), a persistent multi-point colour sampler, an
eyedropper sampling into the active swatch with sample-size and sample-layer scopes, and an
info/readout of pixel values, position and dimensions under the pointer or over a selection.

**[STU-RAS-163] DIAGNOSTIC SURFACE CONTRACT.** The colour sampler MUST support at least four
simultaneous persistent sample points, each storing a document-space position and reading live values
in a selectable colour model. The eyedropper's sample size is a closed enumeration of averaged square
neighbourhoods — `point`, `3x3`, `5x5`, `11x11`, `31x31`, `51x51`, `101x101` — and its sample scope is
`current_layer`, `current_and_below`, or `all_layers`. The histogram MUST expose its underlying bin
counts and its clip counts to the model command surface as structured data, not only as a picture,
because a no-context model verifying an exposure edit needs the numbers ([STU-VAL-004]). Measurement
and count tooling reads pixel length in `pixels` with a document measurement scale.

**[STU-RAS-042]** Raster layers, artboards, selections and slices MUST be exportable through the single
unified `StudioExportRecipe` surface; the full export/format matrix is normative in 14.13. 14.4 owns
only the raster-side touchpoints: a web-slice tool producing independently exportable canvas regions
with per-slice name/URL/type, artboard-scoped export paths, per-layer/per-group export markers via
layer tags ([STU-RAS-009]), and export of a placed-asset container's embedded source back to a
standalone file.

**[STU-RAS-043]** Studio MUST preserve alpha channels, spot channels, layer groups, masks,
adjustment/live-filter/fill layers, placed-asset links and layer effects across import and export
wherever the target format supports them (14.13), and MUST report, as inspectable state, any capability
lost on flatten/export rather than dropping it silently ([STU-RAS-162]).

**[STU-RAS-171] RASTER-SIDE EXPORT PARAMETER CONTRACTS.** 14.13 owns the format matrix; 14.4 owns these
declared raster encoder bounds, which MUST be carried into the `StudioExportRecipe` parameter specs
rather than re-derived:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Encoder parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| JPEG quality | 0 | 12 | 0 | 12 | 3 | count | 0 | hard `declared`; soft `derived` equal to hard; default `declared` |
| JPEG progressive scan count | 3 | 5 | 3 | 5 | unknown | count | 0 | hard `declared` |
| PNG compression | 0 | 9 | 0 | 9 | 0 | count | 0 | hard `declared`; default `declared` |
| TIFF embedded JPEG quality | 0 | 12 | 0 | 12 | unknown | count | 0 | hard `declared` |
| PDF embedded JPEG quality | 0 | 12 | 0 | 12 | unknown | count | 0 | hard `declared` |
| GIF dither amount | 1 | 100 | 1 | 100 | 75 | percent | 0 | hard `declared`; default `declared` |
| Web-optimized colour count | 2 | 256 | 2 | 256 | 256 | count | 0 | hard `derived` from the 8-bit index domain; default `declared` |
| Web-optimized quality | 0 | 100 | 0 | 100 | 60 | count | 0 | hard `derived` from the percentage domain; default `declared` |
| Web-optimized lossy | 0 | 100 | 0 | 100 | 0 | count | 0 | hard `derived`; default `declared` |
| Web-optimized web-snap | 0 | 100 | 0 | 100 | 0 | percent | 0 | hard `derived`; default `declared` |
| Web-optimized blur | unknown | unknown | 0.0 | 2.0 | 0.0 | count | 1 | default `declared`; soft `derived` |
| PDF downsample threshold and target | unknown | unknown | unknown | unknown | unknown | pixels_per_inch | 0 | unit `declared`; everything else `unknown` |

**[STU-RAS-047]** Where a raster capability is also reachable from another Studio domain (a mask from
vector, a gradient/pattern from layout, an export slice, a colour profile), it is the SAME primitive
exposed through the same typed command ([STU-DOC-004]); 14.4 MUST NOT reimplement or rename it. The
full filter catalog is 14.9, the colour pipeline is 14.8, camera raw / develop is 14.12, export and
interop is 14.13, and canonical contracts are 14.23; 14.4 references these and MUST NOT fork them.

**[STU-RAS-172] FILTER PARAMETER ENUMERATIONS ARE OWNED BY 14.9 AND ARE NAMED HERE ONLY TO PREVENT
LOSS.** The captured filter surface carries 41 typed destructive filter operations with full parameter
signatures and a set of filter-specific closed enumerations — noise distribution (`uniform = 1`,
`gaussian = 2`); radial blur method (`spin = 1`, `zoom = 2`) and quality (`draft = 1`, `good = 2`,
`best = 3`); smart blur mode (`normal = 1`, `edge_only = 2`, `overlay_edge = 3`) and quality
(`low = 1`, `medium = 2`, `high = 3`); spherize mode (`normal = 1`, `horizontal = 2`, `vertical = 3`);
zigzag style (`around_centre = 1`, `out_from_centre = 2`, `pond_ripples = 3`); wave type (`sine = 1`,
`triangular = 2`, `square = 3`); ripple size (`small = 1`, `medium = 2`, `large = 3`); polar conversion
(`rectangular_to_polar = 1`, `polar_to_rectangular = 2`); displacement map type (`stretch_to_fit = 1`,
`tile = 2`); undefined areas (`wrap_around = 1`, `repeat_edge_pixels = 2`); lens flare lens type
(`zoom = 1`, `prime_35 = 2`, `prime_105 = 3`, `movie_prime = 5`, with 4 deliberately unused); lens blur
depth-map source (`none = 1`, `transparency_channel = 2`, `layer_mask = 3`, `image_highlight = 4`);
and de-interlace field selectors (`odd = 1`, `even = 2`) and replacement methods (`duplication = 1`,
`interpolation = 2`). These belong to 14.9's catalog and MUST be authored there with full parameter
contracts. They are enumerated here, without their per-filter parameter tables, so that the transfer
of ownership is explicit and no enumeration is lost in the handover.

**[STU-RAS-048]** Every raster capability MUST be represented by a clause or table row here;
see [STU-RAS-102], which supersedes this clause's original authority direction.

**[STU-RAS-050]** Navigation, view, measurement and annotation tools that appear on source raster toolbars
but are not pixel-editing capabilities — pan/hand, zoom, rotate-view, navigator, screen/full-screen
modes, ruler/measure, count, and canvas notes/annotations — are shared shell, diagnostics and
collaboration primitives owned by the Studio shell and cross-cutting surfaces (14.16, 14.17), not by
14.4. They MUST be deduped to exactly one Studio primitive each in their owning surface and MUST NOT be
reimplemented, renamed or re-catalogued as raster tools here; 14.4 records them only so their source
rows are not lost during dedup. Their captured parameter facts — a rotate-view angle in `degrees` and
a cache-purge target enumeration (`undo = 1`, `history = 2`, `clipboard = 3`, `all = 4`) — travel with
them to their owning surface.

---

## 14.12 Camera Raw / Develop Pipeline

Studio ships one native, non-destructive raw develop pipeline. It is a `StudioRawDevelop` graph over a
decoded raw sensor input: an operator or a model lane applies an ordered stack of parametric develop
adjustments and local masks that never alter the original raw bytes, and the developed result becomes
a layer in a `StudioDocument` (14.3, 14.4). Every raw control group, every mask source, every enhance
operation and every profile/preset surface recorded across the source families collapses into this one
pipeline per [STU-SECTION-003]; source product names are never Studio tool, panel or command names.
The phrase "Camera Raw" in this sub-section's heading is inherited section nomenclature from the
v02.205 bundle and is retained only so the section id and title stay stable across the bundle copy. It
is not a Studio surface name: the Studio domain name is **Raw Develop**, the primitive is
`StudioRawDevelop`, and no Studio tool, panel, command, preset family or manual entry may use the
inherited phrase. Renaming the heading is a governed spec change to be taken with the bundle manifest,
not here.

Canonical primitive: `StudioRawDevelop` (schema id `hsk.studio.raw_develop@1`) is a member of
the [STU-DOC-002] primitive set and is field-owned by 14.23. All raw develop state is durable Studio
authority under the SurrealDB/EventLedger contract of [STU-ARC-004]: there is no sidecar metadata file,
no private raw-develop database and no SQLite cache. The parameter contract of [STU-RAS-103]
through [STU-RAS-112] applies in full to every parameter in this sub-section.

---

### 1. Pipeline, Parameter Surface, and the Two Laws That Govern It

**[STU-RAW-001]** `StudioRawDevelop` MUST be a non-destructive, re-editable parametric graph over an
immutable decoded raw input. The original raw sensor data MUST NOT be mutated by any develop operation;
every adjustment, every local mask and every enhance result is stored as parameters and derived
buffers, and the pipeline is fully reversible to the as-decoded state at any time.

**[STU-RAW-100] THE DEVELOP PARAMETER SURFACE.** The develop surface is a flat namespace of typed
parameters organized into panel groups for presentation. It is large and its size is normative: the
captured field engine carries **389 distinct develop parameters** across **21 panel groups**. Every one
of them MUST be a `StudioParameterSpec` ([STU-RAS-103]), MUST be individually addressable by a stable
key, and MUST be individually settable by a typed model command; a "develop settings blob" API that
takes an opaque struct is forbidden, because it defeats [STU-MDL-006] and makes a model unable to change
one slider. The normative group set and its captured parameter counts are:

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Group | Parameters | Clause |
|---|---|---|
| White balance | 7 | see [STU-RAW-110] |
| Basic tone | 16 | see [STU-RAW-111] |
| Basic presence | 6 | see [STU-RAW-111] |
| Tone curve | 18 | [STU-RAW-112], [STU-RAW-113] |
| Detail — sharpening | 4 | see [STU-RAW-114] |
| Detail — noise | 6 | see [STU-RAW-114] |
| Colour mixer / HSL | 33 | see [STU-RAW-115] |
| Colour grading | 15 | see [STU-RAW-116] |
| Lens corrections | 17 | see [STU-RAW-117] |
| Transform / geometry | 24 | see [STU-RAW-118] |
| Effects | 10 | see [STU-RAW-119] |
| Calibration | 7 | see [STU-RAW-120] |
| Masking / local | 84 | [STU-RAW-121] .. [STU-RAW-123] |
| Healing / red-eye | 62 | see [STU-RAW-124] |
| Profile | 34 | [STU-RAW-125], [STU-RAW-126] |
| Preset metadata | 24 | see [STU-RAW-128] |
| Profile look filters | 1 | see [STU-RAW-126] |
| HDR | 9 | see [STU-RAW-129] |
| Lens blur | 1 | see [STU-RAW-130] |
| Crop / orientation | 7 | see [STU-RAW-131] |
| Engine version | 4 | see [STU-RAW-104] |

Group assignment is presentation, not semantics: a parameter's group MAY change without changing its
key, and no behaviour may depend on group membership.

**[STU-RAW-101] CANONICAL SCALE LAW — ONE SCALE PER PARAMETER, PLUS A REQUIRED CONVERSION LAYER
(supersedes the flat "translate the incoming parameters" rule of [STU-RAW-002]).** The same develop
parameter is serialized on **different numeric scales** by the interchange form and by the engine's own
resolved-settings form. This is measured, not suspected: comparing the two captured serializations,
`LuminanceAdjustmentYellow` spans ±90 in the interchange form and ±1 in the engine form — a ratio of
about 0.011 — and `MaskSubCategoryID` spans up to 50008 in the interchange form and 3..12 in the engine
form. Therefore:

- Every develop `StudioParameterSpec` MUST declare exactly one **canonical Studio scale**, and that
  scale is what `StudioRawDevelop` stores.
- A conversion layer is REQUIRED, not optional, in every `StudioImportProfile` and
  `StudioExportRecipe` that reads or writes a develop parameter. The conversion MUST be declared per
  parameter, MUST be invertible, and MUST be tested round-trip.
- Studio's canonical scale for a develop slider is the one the *engine* uses, not the one the
  interchange file uses, because the engine scale is what the render maths consumes.
- A conversion performed implicitly in a UI layer, or a single scale assumed across both forms, is a
  defect. An unconvertible value produces an unsupported-value receipt ([STU-RAS-162]).

The global and local stacks are the worked example an implementer will hit first: the global tone
sliders are integers on a ±100 UI scale, while the *same named* local sliders inside a mask are reals
on a normalized ±1 scale ([STU-RAW-122]). They are two scales of one control and MUST be converted, not
conflated.

**[STU-RAW-102] ABSENT MEANS DEFAULT — AND FOR MOST PARAMETERS THE DEFAULT IS NOT RECOVERABLE FROM
DATA (supersedes the "each with an explicit unit/range" universal claim of [STU-RAW-005]).** The
captured engine writes a parameter only when it is *not* at its default. Of the 389 develop
parameters, **64 are written on effectively every image** (≥99.9 percent of 14,325 catalog rows) and
their modal value is therefore a trustworthy default; **242 are written on fewer than half of rows**,
which means their default is the *absent* value and cannot be read out of any corpus. Consequently:

- Studio MUST treat an absent develop key as "at default", never as zero and never as an error.
- Where this sub-section states a default, it is `derived` from the always-written set and is
  trustworthy. Where it states `unknown`, the default genuinely could not be recovered and
  [STU-RAS-106] governs: it MUST NOT be invented.
- Studio MUST store the develop state **fully resolved** in its own authority — every parameter
  present with an explicit value — even though the interchange forms are sparse. Sparse storage is an
  interchange concern, not an authority concern, because a replayed EventLedger must reconstruct an
  identical render without consulting a defaults table that may have changed ([STU-RAW-106]).
- Every parameter MUST additionally carry an `is_at_default` derived flag so a model can ask what an
  operator actually changed without diffing against a defaults table.

**[STU-RAW-103] INTERCHANGE DECODE AND ENCODE.** Develop settings that arrive embedded in a source
container or an adjacent metadata document are decoded by a `StudioImportProfile` (14.13) which
applies the conversion layer of [STU-RAW-101], resolves sparse keys per [STU-RAW-102], and emits an
unsupported-value receipt for anything Studio cannot represent. Export to such a container is a
`StudioExportRecipe` step that reverses both. Two decode facts are normative: (a) a shipped preset
document may carry **look-table LUT payloads** as encoded binary properties keyed by content hash — 336
such payloads were present in the captured corpus — and these are **data, not parameters**: they MUST
be stored in the artifact tier ([STU-RAS-116]) and referenced by hash, never expanded into the
parameter surface; and (b) preset documents carry parameters the engine never writes and the engine
writes parameters no preset carries (52 interchange-only and 160 engine-only out of 389), so an
importer that assumes one vocabulary will drop the other. The develop authority record is the union.

**[STU-RAW-104] PROCESS VERSION AND ENGINE VERSION ARE TWO FIELDS.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `process_version` | unknown | unknown | unknown | unknown | 15.4 | count | 1 | 10.0, 11.0, 15.4 | default `derived` from 13,286 of 14,325 rows; the render maths is versioned by this field. No soft bound is declared; the observed set is evidence, not a control range |
| `engine_version` | unknown | unknown | unknown | unknown | 16.4 | count | 1 | 10.3 .. 19.0 | the develop engine build, distinct from the process version; no soft bound declared |
| `compatible_version` | unknown | unknown | unknown | unknown | unknown | count | 0 | 234881024 .. 285474816 | a packed minimum-reader version; opaque integer, MUST round-trip; no soft bound declared |
| `has_settings` | — | — | — | — | false | none | — | true | boolean; marks a document as developed at all |

Changing `process_version` is an explicit, receipted operation because it alters the render
([STU-RAW-106]). An imported document MUST record its originating process version and MUST NOT be
silently re-rendered under a newer one; a migration MUST emit an operator- and model-visible receipt
naming both versions. Studio MUST keep every process version it has ever shipped renderable, because a
develop record is only reproducible under the version that authored it.

**[STU-RAW-105] PANEL ENABLE FLAGS ARE PART OF THE STATE.** Each develop group carries a boolean enable
that suppresses the whole group's contribution without discarding its parameters:
`enable_calibration`, `enable_colour_adjustments`, `enable_detail`, `enable_effects`,
`enable_grayscale_mix`, `enable_lens_corrections`, `enable_mask_group_based_corrections`,
`enable_red_eye`, `enable_retouch`, `enable_tone_curve`, `enable_transform`, and
`enable_distraction_removal`. Each defaults to `true` (`observed`, on every row that writes them) and
each MUST be independently settable by a model. These are the before/after and per-panel-visibility
mechanism of [STU-RAW-004]; a temporary preview toggle MUST use them rather than zeroing parameters.

**[STU-RAW-106] DETERMINISM AND REPRODUCIBILITY.** Given the same decoded raw input, the same fully
resolved `StudioRawDevelop` parameter set, and the same process version, the rendered output MUST be
bit-reproducible on the same engine build and MUST fall inside the declared cross-backend tolerance
of [STU-CON-005] across GPU backends. Any develop step with a stochastic component — grain synthesis
being the shipped case, which carries an explicit seed — MUST expose that seed as a parameter so the
render is replayable ([STU-CON-007] property (c)).

**[STU-RAW-002]** Develop authority MUST persist as Studio authority rows bound to the EventLedger
per [STU-ARC-003]/[STU-ARC-004] (event family `studio.raw`), never as an external metadata sidecar, a
proprietary raw-settings database or an embedded settings block in the raw container. Import and
export of such forms is governed by [STU-RAW-103].

### 2. Raw Input Scope and Sensor Decode

**[STU-RAW-003]** The pipeline MUST accept mosaic raw sensor inputs — both the Bayer and the X-Trans
colour-filter-array families, which are sensor-filter architectures and not products — and demosaic
them through a native deterministic engine in `studio-engine` (the `RasterEngine`/`RenderEngine`
boundary of [STU-ARC-002]). It MUST also accept a documented, publicly specified raw interchange
container, and non-raw high-bit-depth still sources, routed through the same develop surface so that
develop adjustments are available on non-raw layers as a re-editable filter. The decode boundary is
the API decode step per [STU-DOC-003], and colour values carry an explicit `StudioColorProfile` from
decode onward with no implicit device colour. The demosaic algorithm MUST be a selectable, receipted
parameter, not a hidden constant.

Provenance for the two container classes that obligation names, recorded as evidence and never as a
Studio tool, command, panel or manual name per [STU-SECTION-003]: the raw interchange container the
capture identifies is DNG, and the non-raw high-bit-depth sources are TIFF and JPEG. DNG and TIFF are
vendor-owned format specifications and appear here only as interop provenance; JPEG is an open
standard. An importer for each is a `StudioImportProfile` (14.13) named for the capability it
provides, not for the format's owner.

**[STU-RAW-004]** The pipeline MUST expose, as projections of the same `StudioRawDevelop` state to both the
operator UI and the model command surface ([STU-DOC-004]): a live histogram with shadow/highlight
clipping indication; zoom, pan and hand navigation with zoom presets and a full-screen review view;
before/after preview cycling and per-group visibility toggling via [STU-RAW-105]; a multi-image
filmstrip with sort and filter and per-image rating, colour-label and mark-for-deletion state; and a
configurable preview/settings cache with a maximum size, purge and relocation controls, held under the
no-SQLite authority rule ([STU-OVR-003]).

### 2a. Retained Group-Scope Clauses

These clauses state what each develop group MUST provide. They are retained from v02.205 with their
scope unchanged; the parameter contract for each group follows in the section named.

**[STU-RAW-005]** The Basic group MUST provide, as typed parameters on `StudioRawDevelop` each with an
`is_at_default` flag and a reset command: white balance (temperature, tint, an eyedropper picker and an
auto analysis); tone (exposure, contrast, highlights, shadows, whites, blacks); presence (texture,
clarity, dehaze, vibrance, saturation); and an auto-tone pass that proposes a full basic parameter set.
Its universal claim that every parameter carries an explicit unit *and range* is SUPERSEDED
by [STU-RAW-102]: units are known for all of them, ranges and defaults are not, and the unknown ones are
declared unknown rather than invented. Contracts in [STU-RAW-110] and [STU-RAW-111].

**[STU-RAW-006]** The Tone Curve group MUST provide a parametric curve with region sliders and adjustable
split points, a point curve on the composite channel, and independent red, green and blue point curves.
Contracts in [STU-RAW-112] and [STU-RAW-113].

**[STU-RAW-007]** The Detail group MUST provide, on the deterministic native engine, capture sharpening
(amount, radius, detail, masking), luminance noise reduction (luminance, detail, contrast), colour noise
reduction (colour, detail, smoothness) and a noise-addition control. Detail operations are
deterministic; the model-backed enhance denoise path is a distinct optional adapter ([STU-RAW-130]) and
MUST NOT be conflated with these sliders. Contract in [STU-RAW-114].

**[STU-RAW-008a]** The Colour Mixer group MUST provide per-hue-band hue, saturation and luminance
adjustment across the standard bands, plus a targeted on-image adjustment mode that maps a drag to the
underlying band. This is the same `StudioColorProfile`-aware colour surface used elsewhere in Studio
(14.8); it is not a raw-only reimplementation. Contract in [STU-RAW-115].

**[STU-RAW-008]** The Colour Grading group MUST provide independent shadow, midtone and highlight wheels
(hue and saturation per range with a per-range luminance), a global wheel, and blending and balance
controls. It subsumes split toning as its two-range degenerate case. Contract in [STU-RAW-116].

**[STU-RAW-009]** The Optics group MUST provide lens-profile correction removing geometric distortion, lens
vignetting and chromatic aberration with automatic profile match plus manual override and profile
selection, and a manual defringe with purple and green amounts, per-fringe hue-range selection and a
fringe-colour sampler. Lens profiles are Studio-native assets or imported profile data; no vendor
lens-profile service is a runtime dependency. Contracts in [STU-RAW-117] and [STU-RAW-127].

**[STU-RAW-010]** The Geometry group MUST provide automatic perspective/level correction with off, level,
vertical, full and guided modes; guided-mode reference lines the operator or model draws; manual
vertical, horizontal, rotate, aspect, scale and X/Y offset sliders; and a constrain-crop option that
trims exposed borders. Contract in [STU-RAW-118].

**[STU-RAW-011]** The Effects group MUST provide film-grain synthesis (amount, size, roughness) and a
post-crop vignette (amount, midpoint, roundness, feather, highlight priority, style). These are
develop-time creative effects distinct from the Optics lens-vignette *removal* of [STU-RAW-009]; both
MUST coexist on the same primitive without ambiguity. Contract in [STU-RAW-119].

**[STU-RAW-012]** The Calibration group MUST expose the develop process version as a first-class field,
shadow tint calibration, and red, green and blue primary hue and saturation calibration. Changing the
process version is an explicit, receipted operation because it alters reproducibility. Contracts
in [STU-RAW-104] and [STU-RAW-120].

---

### 3. White Balance

**[STU-RAW-110] WHITE BALANCE PARAMETER CONTRACT (extends [STU-RAW-005]).** White balance is
**seven** parameters, and the raw and non-raw forms use different units for the same control. Both
forms are normative.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `white_balance_mode` | enum | enum | — | — | `as_shot` | none | — | — | closed set `as_shot`, `auto`, `daylight`, `cloudy`, `shade`, `tungsten`, `fluorescent`, `flash`, `custom`; default `derived` from every row |
| `temperature` | unknown | unknown | 2000 | 50000 | unknown | kelvin | 0 | 2850 .. 16478 | raw form; soft bounds `derived`, hard `unknown`; default is the as-shot value, so no constant default exists |
| `tint` | unknown | unknown | -150 | 150 | unknown | count | 0 | -150 .. 42 | raw form |
| `incremental_temperature` | unknown | unknown | -100 | 100 | 0 | count | 0 | -13 .. 51 | non-raw form; a *relative* offset, not a kelvin value |
| `incremental_tint` | unknown | unknown | -100 | 100 | 0 | count | 0 | -13 .. 34 | non-raw form |
| `as_shot_temperature` | unknown | unknown | 2000 | 50000 | unknown | kelvin | 0 | 6000 | read-only capture metadata; the `as_shot` mode resolves to it |
| `as_shot_tint` | unknown | unknown | -150 | 150 | unknown | count | 0 | 18 | read-only capture metadata |
| `custom_temperature`, `custom_tint` | unknown | unknown | as above | as above | unknown | kelvin / count | 2 / 3 | 3900 .. 7350.9 and -3 .. 27 | the stored custom pair; note precision 2 and 3, which are `observed` and MUST NOT be rounded to integers |

Temperature is in kelvin for a raw source and is a **relative offset** for a non-raw source; these are
two parameters, not one, and Studio MUST NOT unify them. An eyedropper white-balance picker and an
auto white-balance analysis MUST both be model-invokable commands that write these fields.

### 4. Basic Tone and Presence

**[STU-RAW-111] BASIC TONE AND PRESENCE PARAMETER CONTRACT.** Two generations of tone control coexist
in the captured surface and both are written on every row, because a document authored under an older
process version keeps its old fields. Studio MUST carry both sets and MUST bind each to the process
version that consumes it; deleting the legacy set breaks every old document.

Current-generation tone (all defaults `derived` from the always-written set):

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `exposure` | unknown | unknown | -5.00 | 5.00 | 0 | stops_ev | 2 | -1.14 .. 3.34 | soft `derived`; precision 2 `observed` |
| `contrast` | unknown | unknown | -100 | 100 | 0 | count | 0 | -50 .. 75 | |
| `highlights` | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 20 | |
| `shadows` | unknown | unknown | -100 | 100 | 0 | count | 0 | -33 .. 100 | |
| `whites` | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 100 | |
| `blacks` | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 80 | |
| `texture` | unknown | unknown | -100 | 100 | unknown | count | 0 | -31 .. 60 | sparse: default not recoverable |
| `clarity` | unknown | unknown | -100 | 100 | unknown | count | 0 | -60 .. 40 | sparse |
| `dehaze` | unknown | unknown | -100 | 100 | unknown | count | 0 | -28 .. 39 | sparse |
| `vibrance` | unknown | unknown | -100 | 100 | 0 | count | 0 | -40 .. 57 | |
| `saturation` | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 46 | |

Legacy-generation tone, retained for documents authored under an older process version:
`legacy_exposure` (default 0), `legacy_brightness` (default **50**), `legacy_contrast` (default **25**),
`legacy_shadows` (default **5**), `legacy_fill_light`, `legacy_highlight_recovery`, `legacy_clarity`.
The three non-zero legacy defaults are `derived` from the always-written set and are the clearest
proof that a develop default is not always the identity value; an implementer who assumes zero will
render every legacy document wrong.

Auto-tone is a command, not a parameter: it analyses the image and writes a full basic parameter set,
and it carries `auto_tone` / `auto_grayscale_mix` request flags plus two content digests
(`auto_tone_digest`, `auto_tone_digest_no_saturation`) that record which image content the analysis was
computed from. The digests MUST round-trip so a stale auto result is detectable.

### 5. Tone Curve

**[STU-RAW-112] PARAMETRIC TONE CURVE.** The parametric curve is four region amounts plus three
adjustable split points. The three split-point defaults are `observed` and are load-bearing: a
parametric curve reconstructed with different split points is a different curve.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `parametric_highlights` | unknown | unknown | -100 | 100 | unknown | count | 0 | -63 .. 60 | sparse |
| `parametric_lights` | unknown | unknown | -100 | 100 | unknown | count | 0 | -41 .. 70 | sparse |
| `parametric_darks` | unknown | unknown | -100 | 100 | unknown | count | 0 | -50 .. 71 | sparse |
| `parametric_shadows` | unknown | unknown | -100 | 100 | unknown | count | 0 | -73 .. 93 | sparse |
| `parametric_shadow_split` | 0 | 100 | 0 | 100 | 25 | count | 0 | 10 .. 37 | default `derived`; the modal shipped value is 18 and the identity is 25 — Studio declares 25 and records the divergence |
| `parametric_midtone_split` | 0 | 100 | 0 | 100 | 50 | count | 0 | 44 .. 60 | default `observed` at 50 |
| `parametric_highlight_split` | 0 | 100 | 0 | 100 | 75 | count | 0 | 66 .. 90 | default `observed` at 75 |

**[STU-RAW-113] POINT TONE CURVES.** Four independent point curves exist — composite, red, green and
blue — plus a named curve selector and a saturation-refinement control. A curve is a point list in
`levels_0_255` (hard 0..255, `declared`), and its default is the two-point identity `[0, 0, 255, 255]`,
which is `derived` from the always-written set and is the same for all four channels. The curve name
defaults to the identity name (`linear`). `curve_refine_saturation` has a `derived` default of 100
percent and is applied to every point curve. A legacy point-curve set from the older process version
also exists (`tone_curve`, `tone_curve_red`, `tone_curve_green`, `tone_curve_blue`,
`tone_curve_name`) and MUST be retained under the same rule as the legacy tone sliders.

### 6. Detail — Sharpening and Noise

**[STU-RAW-114] DETAIL PARAMETER CONTRACT.** All four sharpening parameters are always written, so all
four defaults are trustworthy; the noise parameters are mixed.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `sharpen_amount` | unknown | unknown | 0 | 150 | 40 | count | 0 | 0 .. 75 | default `derived` — and it is **not zero** |
| `sharpen_radius` | unknown | unknown | 0.5 | 3.0 | 1.0 | pixels | 1 | 0.5 .. 1.4 | default `derived`; precision 1 `observed` |
| `sharpen_detail` | 0 | 100 | 0 | 100 | 25 | count | 0 | 0 .. 50 | default `derived` |
| `sharpen_edge_masking` | 0 | 100 | 0 | 100 | 0 | count | 0 | 0 .. 80 | default `derived` |
| `colour_noise_reduction` | 0 | 100 | 0 | 100 | 25 | count | 0 | 0 .. 50 | default `derived` — **not zero**; colour noise reduction is on by default |
| `colour_noise_detail` | 0 | 100 | 0 | 100 | 50 | count | 0 | 46 .. 57 | default `observed` |
| `colour_noise_smoothness` | 0 | 100 | 0 | 100 | 50 | count | 0 | 46 .. 50 | default `observed` |
| `luminance_noise_reduction` | 0 | 100 | 0 | 100 | 0 | count | 0 | 0 .. 25 | sparse; identity default `derived` |
| `luminance_noise_detail` | 0 | 100 | 0 | 100 | 50 | count | 0 | 50 | default `observed` |
| `luminance_noise_contrast` | 0 | 100 | 0 | 100 | 0 | count | 0 | 0 .. 20 | default `derived` |

A per-ISO default set exists as a first-class structure: a list of `{iso, colour_noise_reduction,
luminance_noise_reduction}` records that supply the defaults for an image at a given sensitivity.
Studio MUST support it, because a fixed default across ISO is wrong for every camera.

### 7. Colour Mixer, HSL, and Grayscale Mix

**[STU-RAW-115] COLOUR MIXER PARAMETER CONTRACT.** The colour mixer is **eight fixed hue bands** —
red, orange, yellow, green, aqua, blue, purple, magenta — each with three adjustments (hue, saturation,
luminance), giving 24 parameters, plus an eight-band grayscale mix used when the image is converted to
monochrome, plus a point-colour list. All are sparse, so none has a recoverable default; the identity
value 0 is `derived` and is the correct default for every band adjustment.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter family | Members | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed |
|---|---|---|---|---|---|---|---|---|---|
| `hue_adjustment_<band>` | 8 | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 100 |
| `saturation_adjustment_<band>` | 8 | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 87 |
| `luminance_adjustment_<band>` | 8 | unknown | unknown | -100 | 100 | 0 | count | 0 | -71 .. 90 |
| `grayscale_mixer_<band>` | 8 | unknown | unknown | -100 | 100 | 0 | count | 0 | -74 .. 100 |
| `convert_to_grayscale` | 1 | — | — | — | — | false | none | — | boolean, always written |
| `point_colours` | 1 | — | — | — | — | empty | none | — | a structured list; default is the empty list, `derived` from the always-written set |

`luminance_adjustment_yellow` is the worked example of [STU-RAW-101]: it spans ±90 in the interchange
form and ±1 in the engine form. Its canonical Studio scale is the engine scale, and the conversion is
mandatory.

A targeted on-image adjustment mode MUST map a drag on a pixel to the band that pixel falls in and
write that band's parameter; it is a UI affordance over these same parameters, not a separate control.

### 8. Colour Grading

**[STU-RAW-116] COLOUR GRADING PARAMETER CONTRACT.** Colour grading is four wheels — shadows,
midtones, highlights and global — each a hue/saturation/luminance triple, plus blending and balance.
Twelve of these parameters are always written, so their defaults are trustworthy.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `colour_grade_<range>_hue` (4) | 0 | 360 | 0 | 360 | 0 | degrees | 0 | 0 .. 359 | hard `derived` from the hue domain; default `derived` |
| `colour_grade_<range>_saturation` (4) | 0 | 100 | 0 | 100 | 0 | count | 0 | 0 .. 73 | default `derived` |
| `colour_grade_<range>_luminance` (4) | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 100 | default `derived` |
| `colour_grade_blending` | 0 | 100 | 0 | 100 | **50** | count | 0 | 0 .. 100 | default `derived` — **not zero**; 50 is the neutral blend |
| `colour_grade_balance` | unknown | unknown | -100 | 100 | 0 | count | 0 | -100 .. 100 | |

The legacy split-toning controls (`split_toning_highlight_hue`, `split_toning_highlight_saturation`,
`split_toning_shadow_hue`, `split_toning_shadow_saturation`, `split_toning_balance`, and an
`enable_split_toning` flag) are the two-range degenerate case of the same mechanism and MUST be
retained for documents that carry them. There is a **key-aliasing hazard here that MUST be handled
explicitly**: the two families overlap in the resolved settings and a naive merge silently loses one.
The import profile MUST resolve them by process version, MUST NOT write both families for one image,
and MUST emit a receipt when it migrates a split-toning pair into the grading wheels.

### 9. Optics — Lens Correction, Defringe, and Manual Distortion

**[STU-RAW-117] OPTICS PARAMETER CONTRACT.** Fifteen of the seventeen optics parameters are always
written, which makes this the best-defaulted group in the pipeline and the one where guessing would be
most damaging.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `auto_lateral_chromatic_aberration` | 0 | 1 | — | — | **1 (on)** | none | — | default `derived`; lateral CA removal is ON by default |
| `lens_profile_enable` | 0 | 1 | — | — | **1 (on)** | none | — | default `derived`; profile correction is ON by default |
| `lens_profile_setup` | enum | enum | — | — | `lens_defaults` | none | — | default `derived`; the other observed modes are an explicit auto match and a manual selection |
| `lens_profile_name` | — | — | — | — | camera-supplied | none | — | default `derived`; resolves to the profile the capture declares |
| `lens_profile_filename` | — | — | — | — | unknown | none | — | sparse; the resolved profile file |
| `lens_profile_digest` | — | — | — | — | content hash | none | — | identifies the exact profile build used, so a re-render is reproducible |
| `lens_profile_is_embedded` | — | — | — | — | true | none | — | default `derived` |
| `lens_profile_distortion_scale` | 0 | 200 | 0 | 200 | **100** | count | 0 | default `derived`; 100 means apply the profile fully |
| `lens_profile_vignetting_scale` | 0 | 200 | 0 | 200 | **100** | count | 0 | default `derived` |
| `manual_distortion_amount` | unknown | unknown | -100 | 100 | 0 | count | 0 | default `derived` |
| `manual_vignette_amount` | unknown | unknown | -100 | 100 | 0 | count | 0 | sparse; identity default |
| `defringe_purple_amount` | 0 | 20 | 0 | 20 | **0** | count | 0 | default `derived`; soft max `derived` |
| `defringe_purple_hue_low` | 0 | 100 | 0 | 100 | **30** | count | 0 | default `derived` — a non-zero default that defines the purple hue window |
| `defringe_purple_hue_high` | 0 | 100 | 0 | 100 | **70** | count | 0 | default `derived` |
| `defringe_green_amount` | 0 | 20 | 0 | 20 | **0** | count | 0 | default `derived` |
| `defringe_green_hue_low` | 0 | 100 | 0 | 100 | **40** | count | 0 | default `derived` |
| `defringe_green_hue_high` | 0 | 100 | 0 | 100 | **60** | count | 0 | default `derived` |

The four defringe hue-window defaults (30/70 and 40/60) are the sharpest example of why guessing is
forbidden: they are not zero, not symmetric about a midpoint, and not derivable from anything except
the captured always-written set. A fringe sampler that picks a hue window from the image MUST write
these same four fields.

### 10. Geometry and Perspective

**[STU-RAW-118] GEOMETRY PARAMETER CONTRACT.** Manual perspective sliders and an automatic
perspective-correction stage coexist; both are always written.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | classification |
|---|---|---|---|---|---|---|---|---|
| `perspective_vertical`, `perspective_horizontal` | unknown | unknown | -100 | 100 | 0 | count | 0 | defaults `derived` |
| `perspective_rotate` | unknown | unknown | -10 | 10 | 0 | degrees | 1 | default `derived` |
| `perspective_scale` | unknown | unknown | 50 | 150 | **100** | count | 0 | default `derived` |
| `perspective_aspect` | unknown | unknown | -100 | 100 | 0 | count | 0 | |
| `perspective_offset_x`, `perspective_offset_y` | unknown | unknown | -100 | 100 | 0 | count | 0 | defaults `derived` |
| `auto_perspective_mode` | enum | enum | — | — | `off` | none | — | closed set `off`, `auto`, `level`, `vertical`, `full`, `guided` |
| `auto_perspective_centre_mode` | unknown | unknown | — | — | 0 | count | 0 | default `derived` |
| `auto_perspective_centre_x`, `_y` | 0.0 | 1.0 | 0.0 | 1.0 | **0.5** | normalized_0_1 | 1 | defaults `derived`; normalized image coordinates |
| `auto_perspective_focal_length_35mm` | unknown | unknown | unknown | unknown | **35** | count | 4 | default `derived`; the analysis needs a focal length and falls back to 35 |
| `auto_perspective_focal_mode` | unknown | unknown | — | — | 0 | count | 0 | default `derived` |
| `auto_perspective_guide_count` | 0 | 4 | 0 | 4 | 0 | count | 0 | guided mode reference lines; hard max `derived` from the four-segment field |
| `auto_perspective_transform_count` | unknown | unknown | — | — | **6** | count | 0 | default `derived`; the stage stores six candidate transforms |
| `auto_perspective_transform_<0..5>` | — | — | — | — | identity | none | 9 | each a nine-element row-major 3×3 matrix serialized as a comma-separated decimal string; the identity matrix is the default |
| `auto_perspective_version` | unknown | unknown | — | — | 151388160 | count | 0 | packed version of the correction model; opaque, MUST round-trip |
| `auto_perspective_preview` | — | — | — | — | false | none | — | boolean |
| `constrain_crop_to_warp` | — | — | — | — | false | none | — | boolean; trims the borders the correction exposes |

The six stored transforms are normative: the correction stage evaluates several candidate
rectifications and stores all of them with the chosen index, so switching mode does not require
re-analysis. Storing only the applied matrix loses that.

### 11. Effects — Grain and Post-Crop Vignette

**[STU-RAW-119] EFFECTS PARAMETER CONTRACT.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision | observed | classification |
|---|---|---|---|---|---|---|---|---|---|
| `grain_amount` | 0 | 100 | 0 | 100 | 0 | count | 0 | 0 .. 92 | sparse; identity default |
| `grain_size` | 0 | 100 | 0 | 100 | **25** | count | 0 | 10 .. 58 | default `derived` — non-zero |
| `grain_frequency` (roughness) | 0 | 100 | 0 | 100 | unknown | count | 0 | 17 .. 65 | sparse |
| `grain_seed` | 0 | 4294967295 | — | — | random-at-first-use | count | 0 | 397038266 .. 2831896825 | REQUIRED for determinism ([STU-RAW-106]) |
| `post_crop_vignette_amount` | -100 | 100 | -100 | 100 | 0 | count | 0 | -100 .. 100 | sparse; identity default |
| `post_crop_vignette_midpoint` | 0 | 100 | 0 | 100 | unknown | count | 0 | 0 .. 100 | sparse |
| `post_crop_vignette_roundness` | -100 | 100 | -100 | 100 | unknown | count | 0 | -100 .. 100 | sparse |
| `post_crop_vignette_feather` | 0 | 100 | 0 | 100 | unknown | count | 0 | 0 .. 100 | sparse |
| `post_crop_vignette_highlight_contrast` | 0 | 100 | 0 | 100 | unknown | count | 0 | 0 .. 1 | sparse |
| `post_crop_vignette_style` | enum | enum | — | — | unknown | none | — | 1 .. 2 | at least two styles; the enumeration is `unknown` beyond that and MUST be resolved before ship |
| `override_look_vignette` | — | — | — | — | false | none | — | — | default `derived`; suppresses a creative profile's own vignette |

The creative-vignette of this group and the lens-vignetting *removal* of [STU-RAW-117] are different
parameters on the same primitive and MUST coexist without ambiguity.

### 12. Calibration

**[STU-RAW-120] CALIBRATION PARAMETER CONTRACT.** Seven parameters, all sparse, all with the identity
default 0 (`derived`): `shadow_tint`, `red_hue`, `red_saturation`, `green_hue`, `green_saturation`,
`blue_hue`, `blue_saturation`. Observed spreads are `red_hue` -19..97, `red_saturation` -100..14,
`green_hue` -72..100, `green_saturation` -45..100, `blue_hue` -34..11, `blue_saturation` -70..31,
`shadow_tint` -25..10; all bounds are `unknown` and the soft range -100..100 is `derived`. These are
primary-hue twists applied on top of the camera profile of [STU-RAW-125], not a replacement for it.

### 13. Camera Profiles, Creative Profiles, and Presets

**[STU-RAW-013]** The pipeline MUST provide a profile stage applied *before* slider edits, offering
camera-matching and neutral/standard render profiles and look/creative profiles selectable from a
profile registry; a preset browser with hover-preview and a preset-amount slider plus user-saved
presets; snapshots (named develop-state versions reapplicable later); default-settings management per
camera model, per serial number and per ISO, and a develop output-profile selection; and per-adjustment
and per-panel add / delete / reset-to-default preset management with active toggles.

**[STU-RAW-125] CAMERA PROFILE MODEL.** A camera profile is a **colour-rendering** model, not a
geometric one, and Studio MUST implement it as this exact structure — a profile reduced to a single
matrix cannot render the shipped corpus:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Component | Presence in the captured corpus | Contract |
|---|---|---|
| `unique_camera_model` | 100 % of 4,373 profiles | the key the profile is matched on; 1,429 distinct camera models are covered |
| `colour_matrix_1`, `colour_matrix_2` | 100 % | two illuminant-referenced 3×3 matrices, XYZ → camera |
| `calibration_illuminant_1`, `_2` | 100 % | the illuminants those two matrices are referenced to; the dominant shipped pair is standard-illuminant-A with D65 (4,365 of 4,373), with a handful of A+flash, D65+A, D55+D75 and flash+tungsten pairs, so the pair MUST be data, not a constant |
| `forward_matrix_1`, `_2` | 99.68 % | camera → XYZ at D50; optional |
| `camera_calibration_1`, `_2` | optional | per-unit calibration matrices applied before the colour matrix |
| `hue_sat_map` | optional | a 3-D deformation lattice over (hue, saturation, value). Shipped lattice dimensions are **not** uniform: [90, 30, 1] on 1,116 profiles, [8, 2, 1] on 22, [90, 25, 1] on 7, [36, 10, 1] on 7, [180, 8, 1] on 3, [18, 6, 1] on 2, and [36, 20, 20] on 1. Studio MUST read the dimensions from the profile and MUST NOT assume a fixed lattice |
| `profile_look_table` | 99.89 % | a second lattice of the same shape applied **after** the hue/sat map. Shipped dimensions: [90, 16, 16] on 2,514, [36, 8, 16] on 1,490, [36, 16, 16] on 354, [45, 8, 8] on 10 |
| `profile_look_table_encoding` | 57.19 % | declares the encoding the look table operates in; absent means the default encoding |
| `profile_tone_curve` | 67.62 % | a 1-D tone curve baked into the profile |
| `baseline_exposure_offset` | 51.70 % | an exposure shift the profile applies |
| `default_black_render` | 57.19 % | whether the profile applies its own black rendering |
| `profile_embed_policy` | 100 % | governs whether the profile may travel with an exported file |
| `profile_calibration_signature`, `profile_copyright`, `profile_name` | 99.5 % / 96.5 % / 100 % | identity and provenance; 129 distinct profile names across the corpus |

Ordering is normative: camera-calibration matrix, then colour matrix interpolated between the two
illuminants, then forward matrix, then hue/sat map, then look table, then tone curve, then baseline
exposure offset. The calibration sliders of [STU-RAW-120] apply on top of this stage, not inside it.
Profile payloads live in the artifact tier ([STU-RAS-116]); the captured corpus is roughly 895 MB.

**[STU-RAW-126] CREATIVE PROFILES AND LOOK TABLES.** A creative profile is the same structure with a
look table carrying the creative render, plus an **amount** control and a set of capability flags that
declare where the profile may be offered. The captured field corpus ships 1,045 profile and preset
documents split 599 look-class and 446 ordinary, organized into 53 named groups. The capability flags
are normative and MUST be honoured by the browser: `supports_amount`, `supports_colour`,
`supports_monochrome`, `supports_output_referred`, `supports_scene_referred`,
`supports_high_dynamic_range`, `supports_normal_dynamic_range`, `show_in_presets`,
`show_in_quick_actions`, and an optional `camera_model_restriction` limiting a profile to particular
cameras. A `look_amount` scales the profile's contribution with an `observed` range of 0.3 .. 1.7 and a
`derived` default of 1.0, so a creative profile can be applied at **more** than full strength. Look
tables are content-addressed binary payloads referenced by hash ([STU-RAW-103]).

**[STU-RAW-127] LENS PROFILE MODEL.** A lens profile is a **sampled** correction model, not a formula:
it stores samples over a (focal length, focus distance, aperture) grid and the consumer interpolates
between them for the shot's actual values. Studio MUST implement it as that grid.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Sub-model | Presence across 3,597 captured profiles | Contract |
|---|---|---|
| `perspective_model` | 3,438 | `focal_length_x`, `focal_length_y`, `image_x_centre`, `image_y_centre`, `radial_distort_param_1..3`, optional `tangential_distort_param_1..2`, `scale_factor`, `residual_mean_error`, `residual_standard_deviation` |
| `fisheye_model` | 159 | the same role under a fisheye projection; a profile carrying it MUST NOT be evaluated with the rectilinear model |
| `vignette_model` | 2,282 | `vignette_model_param_1..3` |
| `chromatic_red_green_model` | 899 | lateral CA for the red-green pair |
| `chromatic_green_model` | 899 | lateral CA reference channel |
| `chromatic_blue_green_model` | 899 | lateral CA for the blue-green pair |

Lateral chromatic aberration is corrected by **three separate per-channel-pair models**, not by one
combined model; collapsing them to a single CA strength cannot reproduce a shipped correction. Sample
counts per profile run from 1 to 61,975 with a mean of 172, so the interpolator MUST handle both a
single-sample prime and a densely sampled zoom. Observed sample-axis ranges are focal length 1.27 to
1600.0 mm, aperture value 0.526069 to 12.643856, focus distance 0.086 to 1×10³² (the large value is the
infinity sentinel and MUST be treated as such, not as a number), sensor format factor 0.645 to
19.333334, and scale factor 0.955008 to 1.049565. Distortion and vignette coefficients span extreme
magnitudes — radial parameter 3 from -1.66×10⁸ to 3.21×10⁷, vignette parameter 3 from -8.70×10¹⁰ to
5.91×10⁹ — so the coefficient type MUST be `f64`; `f32` loses shipped profiles. Every sample carries a
residual mean error (0 .. 0.390258) and standard deviation (0 .. 0.022094), which MUST be surfaced as
inspectable state so a model can tell a well-fitted correction from a poor one. The corpus covers 2,167
distinct lens names calibrated on 573 camera bodies across 620,327 samples in roughly 471 MB, all of
which lives in the artifact tier.

**[STU-RAW-128] PRESET, SNAPSHOT, AND DEFAULTS REGISTRY.** A develop preset is a named, grouped,
uniquely identified partial parameter set with an optional amount control, and it MUST record which
parameters it sets — a preset that stores the full resolved state cannot be applied on top of an
existing edit. Its metadata fields are normative: `name`, `short_name`, `sort_name`, `group`,
`cluster`, `uuid`, `description`, `copyright`, `contact_info`, `preset_type`, plus the capability flags
of [STU-RAW-126] and an `is_stub` flag marking a placeholder entry. A snapshot is a *complete* named
develop state, reapplicable and deletable, and is distinct from a preset for exactly that reason.
Default-settings management MUST support per-camera-model, per-serial-number and per-ISO defaults; the
per-ISO form is the structured record of [STU-RAW-114]. A `toggle_style_amount` and
`toggle_style_digest` pair records a partially applied style so the toggle is reversible.

### 14. Masking and Local Adjustment

**[STU-RAW-014a]** Local adjustment MUST be expressed through `StudioMask` (the same masking primitive
used by 14.4 and 14.9), not a raw-only mask type. The develop masking system MUST support the full
mask-source set: manual and geometric sources (linear gradient, radial gradient, brush); range sources
(colour range, luminance range, depth range); and model-assisted sources (subject, sky, background,
landscape components, objects from a rough stroke or rectangle, and people with per-person component
sub-masks), the last of which are an optional adapter per [STU-RAW-130] and degrade to the manual
sources when no adapter is present.

**[STU-RAW-014b]** Masks MUST compose and manage: combine by add, subtract and intersect between any mask
sources; invert, including duplicate-and-invert; duplicate, rename, hide and delete; a customizable
mask overlay display; and a mask-local develop slider stack saveable as a local-adjustment preset
re-applied with an amount control. Intersect and subtract semantics are the canonical `StudioMask`
semantics of 14.23; the develop pipeline does not fork them.

**[STU-RAW-121] CORRECTION AND MASK RECORD STRUCTURE.** A local adjustment is a **correction** holding
a list of **masks** plus its own develop slider stack. Both levels carry identity and sync fields that
MUST round-trip.

Correction fields: `correction_id` (stable UUID), `correction_name`, `correction_active` (default
true), `correction_amount` (`normalized_0_1`, `observed` 0.69 .. 1.38 — note it exceeds 1.0, so it is a
gain, not a fraction; `derived` default 1.0), `correction_sync_id`, `correction_reference_x` /
`correction_reference_y` (normalized image coordinates), `correction_masks` (the list), and the local
slider stack of [STU-RAW-122].

Mask fields: `mask_id`, `mask_name`, `mask_active` (default true), `mask_inverted` (default false),
`mask_value` (`normalized_0_1`, default 1.0), `mask_blend_mode` (`observed` 0 and 1), `mask_sub_type`
(`observed` 0..3), `mask_sub_category_id` (see [STU-RAW-101]: this key's scale differs between the two
serializations and MUST be converted), `mask_version`, `mask_sync_id`, `mask_digest`,
`input_digest` + `input_digest_version`, `local_input_digest` + `local_input_digest_version`,
`model_version`, `full_mask_size` (a `"width,height"` pair), `whole_image_area`, `origin`,
`reference_point`, `error_reason`, and a `what` discriminator naming the source kind.

The four digest fields are normative and are not incidental: a model-generated mask is only valid for
the image content it was computed from, and the digests are how Studio detects that the content changed
and the mask must be recomputed. Dropping them yields silently stale masks.

Geometry by source kind:

*Derivation: catalogue table, splits per row; yields 4 microtasks, one per mask geometry source.*

| Mask source | Fields | unit and bounds |
|---|---|---|
| Linear gradient | `zero_x`, `zero_y`, `full_x`, `full_y`, `flipped` | normalized_0_1 (values outside 0..1 are legal and shipped — observed -0.29 .. 1.04 — because a gradient may originate off-canvas) |
| Radial gradient | `top`, `left`, `bottom`, `right`, `angle`, `midpoint`, `roundness`, `feather`, `flipped` | normalized_0_1 for the rect, degrees for angle, count for midpoint (default 50) and roundness |
| Brush | `dabs` (an ordered stroke list, each `"d x y"` in normalized coordinates), `radius`, `size_x`, `size_y`, `flow`, `centre_weight` (default 0.5), `centre_value`, `perimeter_value`, `alpha`, `feather` | normalized_0_1 |
| Range (colour, luminance, depth) | a sample set plus a refine/breadth control, and a map-view toggle | normalized_0_1 |

Brush dab lists are stroke data, not a rasterized mask: the mask is re-rendered from the dab list at
the current resolution, which is what makes a develop brush mask resolution-independent.

**[STU-RAW-122] THE MASK-LOCAL SLIDER STACK IS A SECOND, DIFFERENTLY-SCALED COPY OF THE GLOBAL STACK.**
Each correction carries its own develop stack: `local_exposure`, `local_contrast`, `local_highlights`,
`local_shadows`, `local_whites`, `local_blacks`, `local_clarity`, `local_dehaze`, `local_texture`,
`local_saturation`, `local_temperature`, `local_tint`, `local_hue`, `local_sharpness`,
`local_luminance_noise`, `local_moire`, `local_defringe`, `local_grain`, `local_curve_refine_saturation`,
`local_toning_hue`, `local_toning_saturation`, `local_point_colours`, `local_colour_variance`, plus
per-mask curves (a main curve and per-channel curves) and the legacy-process copies
(`local_brightness`, `local_clarity` legacy, `local_contrast` legacy, `local_exposure` legacy).

**The local stack is on a different numeric scale from its global namesake.** The global sliders are
integers on a ±100 UI scale; the local sliders are reals on a normalized ±1 scale — observed ranges
include local shadows -1.0 .. 1.0, local highlights -1.0 .. 0.8895, local exposure -0.6465 .. 0.5085,
local whites -0.16022 .. 0.91525, local blacks -0.46961 .. 0.0, local clarity -0.2957 .. 0.5882, local
texture -0.3043 .. 0.6022, local temperature -0.4237 .. 0.2542, local saturation -0.6 .. 0.45 — with up
to 6 decimal places. `local_toning_hue` is the exception: it is in `degrees` with an observed range of
0 .. 222. `local_curve_refine_saturation` is in percent with an observed range of 30 .. 100 and a
`derived` default of 100. Every local slider's canonical unit is `normalized_signed_1` with
`precision = 6` unless the row above says otherwise, every hard bound is `unknown`, and the identity
default 0 is `derived`. Treating a local slider as a ±100 integer will move it by a hundredfold.

**[STU-RAW-123] MASK SOURCE CATALOGUE AND SUBCATEGORY IDS.** Mask sources are identified by a source
kind plus a subcategory id, and the subcategory space is sparse and vendor-versioned (`observed` 2 to
50008 in one serialization and 3 to 12 in the other). Studio MUST declare its own closed source
enumeration and MUST carry the imported subcategory id opaquely alongside it so a re-export is
lossless. The normative Studio source set is: `linear_gradient`, `radial_gradient`, `brush`,
`colour_range`, `luminance_range`, `depth_range`, `subject`, `sky`, `background`, `landscape_component`,
`object`, `person`, `person_component`. A model-assisted source MUST record its model identity and
version in `model_version` and MUST be recomputable on demand.

### 15. Local Repair and Distraction Removal

**[STU-RAW-015]** The pipeline MUST provide non-destructive crop and straighten (aspect, angle, rotate,
flip), a heal/clone spot-removal tool with source-point control, and a red-eye and pet-eye correction
with pupil size and darken controls. These are develop-scoped operations on `StudioRawDevelop` and
reuse the raster retouch primitives of 14.4 where the capability is shared ([STU-DOC-004]); they are
not a parallel retouch implementation.

**[STU-RAW-124] RETOUCH RECORD STRUCTURE AND THE GENERATIVE-FILL BOUNDARY.** A retouch area is a
record carrying `spot_type` (`heal` or `clone`), `heal_version`, `feather`, `opacity`, `seed`, a
`method` (`gaussian` observed), a `source_state` (`source_set_explicitly` observed, versus an
engine-chosen source), a target dab-mask list with the same brush geometry as [STU-RAW-121], and the
source offset (`source_x`, `source_y`, `offset_y`, `centre_x`, `centre_y`, `radius`). Retouch areas and
red-eye areas are two lists, both defaulting to empty (`derived` from the always-written set), gated by
`enable_retouch` and `enable_red_eye`.

A third path exists in the captured data and Studio MUST classify it correctly: a distraction-removal
record whose fill is produced by a **generative model**, carrying a model version string, a fill-method
name, image-extent and search/target rectangles in pixels, black level and gamma type, input and patch
content digests, remap-info strings, and a list of patch *variations* the operator may cycle. That path
is a `StudioModelAdapter` lane under [STU-RAS-045] and [STU-RAS-164], not a core develop tool. Studio's
core distraction removal is the on-device inpaint of [STU-RAS-143]. Where an adapter produced a fill,
the record MUST carry the adapter identity, the model identity and version, whether it ran locally, the
patch variation set, and the digests that make the result reproducible and staleness-detectable.

### 16. HDR, Lens Blur, and Enhance

**[STU-RAW-129] HDR DEVELOP STATE.** The pipeline MUST carry an HDR edit mode with an SDR-compatibility
sub-stack, because an HDR develop record must still render on a standard-range display without a second
edit. Nine parameters, all with `derived` defaults from the always-written set: `hdr_edit_mode`
(default 0 = off), `hdr_max_value` (default **4**, the headroom in stops above diffuse white),
`sdr_blend`, `sdr_brightness`, `sdr_clarity`, `sdr_contrast`, `sdr_highlights`, `sdr_shadows`,
`sdr_whites` (all default 0). Hard bounds are `unknown`; the SDR sub-stack's soft range is ±100
(`derived`) matching its global namesakes. A creative profile declares whether it supports high and
normal dynamic range ([STU-RAW-126]) and the browser MUST filter on those flags.

**[STU-RAW-130] ENHANCE TIERS AND THE LENS-BLUR STAGE.** Enhance operations split into two tiers and the
split is normative.

(a) **Native deterministic tier.** The demosaic / raw-detail refinement (edge rendition, colour
rendering and artifact suppression at native resolution for Bayer and X-Trans sources) and
integer/linear super-resolution upscale MUST run on the deterministic `studio-engine` path and MUST
produce reproducible output under a fixed process version.

(b) **Adapter tier.** Model-backed denoise, model-backed raw-detail, model-backed super-resolution and
the model-assisted mask sources of [STU-RAW-123] MUST be implemented only as an optional
`StudioModelAdapter` with a local model preferred and no required cloud or account dependency. When no
adapter is installed the pipeline MUST fall back to the deterministic tier and surface a capability
receipt rather than failing. The metered-adapter obligations of [STU-RAS-164] apply in full.

(c) **Re-editability and lifecycle.** An enhance result MUST remain re-editable. An update/flatten
lifecycle MUST let model results be refreshed when models change and flattened to a baked buffer with a
documented reset path back to the editable state, and every enhance run MUST emit a receipt naming the
adapter, the model identity and version, and whether it ran locally.

A **lens-blur** stage exists as a single structured develop parameter (a depth-aware blur with its own
sub-structure, defaulting to the empty structure). It MUST be modelled as a structured develop stage,
not as a raster filter applied afterwards, because it consumes the develop pipeline's depth information
and must sit inside the develop order.

**[STU-RAW-014]** Enhance operations MUST be split into the native deterministic tier and the optional
model-adapter tier exactly as [STU-RAW-130] specifies.

### 17. Crop, Workflow Output, and Linkage to the Raster Document

**[STU-RAW-131] CROP AND ORIENTATION CONTRACT.** Crop is stored as four normalized edge positions plus
an angle and two constraint flags, never as a pixel rectangle, so it survives a resolution change:
`crop_left`, `crop_top`, `crop_right`, `crop_bottom` in `normalized_0_1` with `precision = 6`
(observed 0.001271 .. 0.999999), `crop_angle` in `degrees` with `precision = 6` (observed -2.73 ..
9.41398), `crop_constrain_aspect_ratio` (boolean, `observed` default true) and `crop_constrain_to_warp`
(boolean). All hard bounds are `unknown`; the normalized edges are bounded 0..1 by the normalized
domain (`derived`). Orientation and flip are separate discrete fields and MUST NOT be folded into the
crop angle.

**[STU-RAW-016]** Workflow options MUST configure how a developed raw is handed to the `StudioDocument`:
output colour space (`StudioColorProfile`), bit depth, output pixel dimensions and resolution, and the
open behaviour including whether the developed raw opens flat or as a re-editable placed object. The
pipeline MUST support raw-as-re-editable-object: a developed raw MAY be placed into the raster document
(14.4) as a `placed_asset` layer whose `StudioRawDevelop` settings remain editable in place — reopening
the object returns to the full develop surface with all parameters, masks and process version intact.
This is the one linked-object primitive of 14.4; there is no separate raw-embed format.

**[STU-RAW-017]** Save and output of a developed raw MUST route through `StudioExportRecipe` (14.13) for
derived deliverables with format-specific options, and MUST NOT invent a raw-develop-only export path
that bypasses the 14.13 export contract. Multi-image apply — copying develop settings from one raw to a
selection of others, paste-settings, previous-settings and preset apply across a filmstrip selection —
MUST operate on the canonical selection set, not on the visible or loaded subset. The raw-side export
facts 14.13 MUST carry are: a DNG output path with a compatibility version, a compression flag, a
conversion method that may preserve the original raw payload, and an embedded-preview cache option;
output bit depths of 8, 10, 16 and 32 bits per component; compression selectable as none, LZW or ZIP;
a resize mode enumeration of `width_and_height`, `dimensions`, `long_edge`, `short_edge`, `megapixels`
and `percentage`; an output-sharpening amount of `low`, `standard` or `high`; and a filename-collision
policy of `ask`, `overwrite`, `rename` or `skip`.

### 18. Model Steerability, Headless Operation, and Validation

**[STU-RAW-018]** Every `StudioRawDevelop` control group, mask source, enhance operation, profile and
preset action, and workflow option MUST be exposed as a typed, model-steerable command with a stable
identifier and MUST be observable through the Studio visual-debug/inspection surface (Argus) per 14.16;
all raw develop operations MUST run headless and quiet under 14.20 (no foreground window, no focus
steal, bounded and observable); and each MUST carry a dual-audience UserManual entry per 14.22 covering
purpose, inputs and outputs, and failure and recovery. A model-authored develop edit MUST pass the
sandbox → `StudioValidationDescriptor` → `PromotionGate` lifecycle of [STU-ARC-005] before it changes
authority rows; model confidence never bypasses the gate.

**[STU-RAW-132] DEVELOP-SPECIFIC VALIDATION CHECKS.** `StudioValidationDescriptor` (14.24) MUST carry
these blocking checks for the develop domain, each of which exists because the captured behaviour makes
the corresponding mistake easy and silent:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Check | Fails when |
|---|---|
| `RAW-SCALE-001` | a develop parameter is written or read without the declared conversion of [STU-RAW-101], or a local slider is handled on the global scale |
| `RAW-DEFAULT-001` | an absent interchange key is treated as zero rather than as the parameter's default ([STU-RAW-102]) |
| `RAW-DEFAULT-002` | a parameter whose default is `unknown` is shipped with an invented default |
| `RAW-PV-001` | a document is re-rendered under a different process version without a migration receipt |
| `RAW-PROFILE-001` | a camera profile is evaluated with an assumed lattice dimension, a single illuminant, or an `f32` lens coefficient |
| `RAW-MASK-001` | a mask is applied without checking its content digests, or a brush mask is stored rasterized rather than as its dab list |
| `RAW-ADAPTER-001` | a model-backed enhance or mask result is produced without an adapter receipt naming model identity, version and locality |
| `RAW-DETERM-001` | a stochastic develop stage renders without an explicit stored seed |

---

### 19. Microtask Derivation Rule (normative for 14.4 and 14.12)

**[STU-RAS-173] THIS SUB-SECTION IS THE MICROTASK SOURCE.** The implementing work for 14.4 and 14.12
MUST be cut from these clauses and from nothing else. The cut is deterministic so that two independent
authors produce the same microtask set:

1. **One microtask per clause that names a shippable behaviour.** A clause carrying a MUST that a
   reviewer can attack independently is one microtask. A clause that only classifies, defines a
   vocabulary, or hands ownership to another sub-section is not.
2. **A clause carrying a parameter-contract table splits when, and only when, the table crosses a
   primitive boundary.** [STU-RAS-127] through [STU-RAS-136] are nine microtasks because each is a
   separately testable sub-engine of the brush; [STU-RAW-111] is one microtask because its two tables
   are two generations of one control set on one primitive.
3. **Every microtask inherits the full obligation set of [STU-RAS-001] and [STU-CON-007]** — GUI
   control, typed model command, Argus observability, UserManual entry, model-invokable, parallel-safe,
   deterministic, visually verifiable — and MUST carry all of them in its acceptance criteria. A
   microtask that omits one is incomplete, not scoped.
4. **Every numeric parameter a microtask implements MUST be implemented with its complete
   `StudioParameterSpec`**, including the fields this sub-section marks `unknown`. A microtask that
   ships a parameter with an invented bound fails [STU-RAS-111].
5. **A clause that supersedes another** ([STU-RAS-125], [STU-RAS-136], [STU-RAS-156], [STU-RAW-101],
   [STU-RAW-102]) yields a microtask whose acceptance criteria include proving the superseded behaviour
   is *not* present — a single-instance effect stack, a 0..100 smoothing slider, a single-scale develop
   parameter, or an absent-means-zero import are each a specific regression to test for.
6. **A clause marking a `unknown` bound, an unobserved parameter block ([STU-RAS-153]), or an
   unresolved enumeration ([STU-RAW-119] vignette style) yields a governed spec-enrichment item, not an
   implementation guess.** These are the spec-debt register for this sub-section and MUST be tracked as
   such.

**[STU-RAS-174] SPEC-DEBT REGISTER FOR 14.4 AND 14.12.** The following are the known, named gaps in this
sub-section. Each is an absence of *evidence*, not an absence of a requirement, and each MUST be
resolved by governed spec enrichment before the clause that carries it can be closed as complete:

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Gap | Clause | What is missing |
|---|---|---|
| Edge-refinement numeric bounds | [STU-RAS-121] | six controls carry a declared unit and precision and no declared hard bound, soft bound or default (the clause said five before the parameter-contract audit; the table has always held six) |
| Content-aware fill numeric bounds | [STU-RAS-143] | every numeric on the surface carries a control class and no bounds |
| Unobserved adjustment parameter blocks | [STU-RAS-153] | `invert`, `ocio`, `normals`, `tone_stretch` ship no presets, so their fields were never observed |
| Brush physics and erodible-tip hard bounds | [STU-RAS-128], [STU-RAS-129] | observed spreads only; no declared engine limits |
| Post-crop vignette style enumeration | [STU-RAW-119] | at least two members observed; the full closed set is unresolved |
| Develop defaults for 242 sparse parameters | [STU-RAW-102] | the default is the absent value and is not recoverable from any corpus |
| Mask subcategory id space | [STU-RAW-123] | sparse, vendor-versioned, carried opaquely until a mapping is established |
| Tool and panel identifier binding in one source family | — | 143 tool identifiers and 53 panel identifiers were recovered as opaque four-character codes with no resolved display-name binding; they are evidence that the surfaces exist, not a catalogue of what they are |
| Scrubbable-control step values | [STU-RAS-108] | no source declares step, coarse step or fine step; the derivation rule in that clause is Studio policy |
| Soft bounds equal to hard bounds without a declaration | 81 rows across 20 parameter tables: [STU-RAS-115], [STU-RAS-127]-[STU-RAS-136], [STU-RAS-138], [STU-RAS-139], [STU-RAS-147], [STU-RAS-160], [STU-RAS-171], [STU-RAW-112], [STU-RAW-114], [STU-RAW-116]-[STU-RAW-119] | 86 of the 324 parameter rows in this module state a soft bound identical to their hard bound. Five of them say the source declared the equality — the three opacity/density rows of [STU-RAS-115] and `diameter` and `hardness` in [STU-RAS-127] — and those five are sound. The other 81 do not: their bounds are marked `derived` from the unit's domain or `observed` across shipped presets, so the equality is an artefact of that derivation rather than a fact the capture recorded. Per [STU-RAS-104] a soft bound the source did not declare is `unknown`, and mirroring it from the hard bound is the one collapse that cannot be undone without re-deriving from the captures. Each of the 81 MUST be re-checked against its capture and either confirmed as declared or reset to `unknown`. Until then an implementer MUST NOT read the equality as a source-declared control range |

Every row in this register is stated here rather than papered over, because a spec that hides a gap
produces an implementer who fills it with a guess.

**[STU-RAS-175] DERIVATION RULE (NORMATIVE; supersedes cut rules 1 and 2 of [STU-RAS-173]).** The
14.4 and 14.12 microtask set is derived from this module mechanically, not editorially. ONE microtask
corresponds to ONE of the following units, and to nothing else:

1. **Each clause definition** in this module that states a stored contract, an enumeration, or an
   engine behaviour a reviewer can attack independently of its siblings. One microtask per clause
   anchor — never one per sentence, and never gated on the clause containing a MUST, because a clause
   may state a stored contract in the indicative mood.
2. **Each PARAMETER TABLE, taken whole.** The table is the acceptance surface and every row's seven
   bound fields are its acceptance criteria. A parameter table is one unit however many primitives
   its rows span; it does not split.
3. **Each CATALOGUE ROW** — one row of a table whose first column names a separate implementable
   subject. The closed list of catalogued subject kinds in this module is: each raster layer kind
   (§3), each mask form (§4), each selection tool or operation (§5), each painting tool and each
   retouching tool (§7), each preset-family registry (§8), each `StudioAdjustment` kind (§9), each
   transform primitive (§11), each blend mode (§12), each layer-effect kind (§13), and each develop
   mask geometry source (14.12 §14). No other kind of row splits.
4. **Each ENUMERATION TABLE, taken whole.** Every member is an acceptance criterion.
5. **Each VALIDATION DESCRIPTOR** stated in 14.12 §18.

This module contains no command, shortcut, binding, key, menu, action, preset or template table, and
no golden-corpus case. Those unit kinds are therefore not members of its closed list and MUST NOT be
imported into it by a tool that recognises them in other modules.

Rule 0 — derivation markers are authoritative. Every table in this module carries an italic
`*Derivation: ...*` marker sentence directly above it, stating which of the classes above the table
belongs to and how many microtasks it yields. That marker is NORMATIVE. A derivation tool that
classifies a table differently from its marker, or that produces a different count for it, has
diverged from this sub-section and MUST be reconciled against it, not the reverse. The marker forms
in use here are: parameter table taken whole (1); enumeration table taken whole (1); catalogue table
splitting per row (N); contract table carried into the clause's own microtask (0); and reading aid
inside a non-yielding clause (0). The sixth form of the shared vocabulary, preset or command table
taken whole, is unused here because this module has no such table.

Cut rules 3, 4, 5 and 6 of [STU-RAS-173] are NOT superseded and still bind every unit derived under
this clause: each microtask inherits the full obligation set of [STU-RAS-001] and [STU-CON-007]; each
numeric parameter ships its complete `StudioParameterSpec` including every field marked `unknown`; a
microtask cut from a superseding clause proves the superseded behaviour is absent; and a stated
unknown is governed spec debt, never an implementation guess.

**[STU-RAS-176] THE NON-YIELDING SET IS CLOSED.** A clause or table in the following four categories
yields no microtask, and nothing outside these four categories may be treated as non-yielding:

1. **Pure cross-references.** A clause whose entire content points at where the contract actually
   lives rather than stating one. There are exactly two in this module: the fill-versus-layer-opacity
   pointer at [STU-RAS-010], whose contract is [STU-RAS-115], and the content-aware pointer at
   [STU-RAS-030], whose contract is [STU-RAS-143]. Both are retained because they hold their v02.205
   anchors; neither is cut into work twice.
2. **Restatements of an obligation that attaches to every microtask.** The GUI-control, typed-command,
   Argus-observability, UserManual, headless-and-quiet, determinism and visual-verifiability
   obligations attach to every unit derived from this module. A clause or sentence restating one of
   them adds no unit of its own; it is an acceptance criterion inside every unit.
3. **Supersession and disposition rows.** The replacement declaration, the anchor-disposition table
   and the spec-completeness rule of §0 record what happened to the v02.205 text. They are
   bookkeeping about the spec, not work on the product.
4. **This derivation sub-section itself** — all seven of its clause definitions and every table inside
   it, this list included.

Of the 181 clause definitions in this module, 12 yield no microtask: the three of §0, the two pure
cross-references, and the seven of §19. The remaining 169 yield exactly one microtask each, of which
167 are clause units and 2 are validation descriptors.

**[STU-RAS-177] AN OPEN ITEM OR A BLOCKED DEPENDENCY STILL YIELDS A MICROTASK.** A clause that
declares an `unknown` bound, an unobserved parameter block, an unresolved enumeration, an unbound
identifier space, or any other row of the spec-debt register of [STU-RAS-174] yields its microtask
exactly as every other clause does. It is never struck from the yields index, and its work is never
quietly deferred out of the set — a gap that removes work from the ledger is indistinguishable, six
months later, from work that was never specified. What the declared gap changes is the microtask's
FIRST acceptance criterion: that criterion MUST be resolving the declared dependency — recovering the
missing bound, member, default, step value or identifier binding through governed spec enrichment and
recording it back into this module — and no later criterion may be satisfied by inventing the missing
value in its place. The same rule governs a clause blocked on a contract owned by another sub-section:
the microtask exists, and its first acceptance criterion is obtaining that contract. Ten such open
items are registered in [STU-RAS-174]. Eight of them name a specific clause, and every one of those
clauses appears in the yields index below with a non-zero count, so none of that work is unscheduled.
The two that name no single clause — the unresolved tool and panel identifier binding, and the 81
undeclared soft bounds spread across twenty parameter tables — are cross-cutting audits rather than
one clause's gap; each named parameter table already yields a microtask, and the audit is that
microtask's first acceptance criterion wherever a row of it is affected.

**[STU-RAS-178] YIELDS INDEX.** One row per sub-section. `Clause definitions` counts anchors defined
in that sub-section; the four unit columns count the microtasks each class contributes; `Yields` is
their sum and is the LAST numeric column in every row, including the total row.

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Sub-section | Clause anchors | Clause definitions | Clause units | Parameter tables | Catalogue rows | Enumerations | Validators | Yields |
|---|---|---|---|---|---|---|---|---|
| 14.4 §0 Replacement, derivation, anchor disposition | [STU-RAS-100]-[STU-RAS-102] | 3 | 0 | 0 | 0 | 0 | 0 | 0 |
| 14.4 §1 The parameter contract | [STU-RAS-103]-[STU-RAS-112] | 10 | 10 | 0 | 0 | 0 | 0 | 10 |
| 14.4 §2 Cross-cutting raster obligations | [STU-RAS-001]-[STU-RAS-005], [STU-RAS-116] | 6 | 6 | 0 | 0 | 0 | 0 | 6 |
| 14.4 §3 Raster document, layer kinds, layer numerics | [STU-RAS-006]-[STU-RAS-011], [STU-RAS-113]-[STU-RAS-115], [STU-RAS-117] | 10 | 9 | 1 | 7 | 0 | 0 | 17 |
| 14.4 §4 Placed-asset container and masking semantics | [STU-RAS-012]-[STU-RAS-016], [STU-RAS-051], [STU-RAS-124] | 7 | 7 | 0 | 5 | 0 | 0 | 12 |
| 14.4 §5 Selection | [STU-RAS-017]-[STU-RAS-022], [STU-RAS-118]-[STU-RAS-123] | 12 | 12 | 3 | 12 | 0 | 0 | 27 |
| 14.4 §6 The brush engine | [STU-RAS-023], [STU-RAS-125]-[STU-RAS-136], [STU-RAS-165], [STU-RAS-166] | 15 | 15 | 10 | 0 | 1 | 0 | 26 |
| 14.4 §7 Painting, retouching, and erasing tools | [STU-RAS-024], [STU-RAS-026]-[STU-RAS-028], [STU-RAS-137]-[STU-RAS-139], [STU-RAS-170] | 8 | 8 | 2 | 16 | 0 | 0 | 26 |
| 14.4 §8 Fill, gradient, pattern, and the preset libraries | [STU-RAS-025], [STU-RAS-160]-[STU-RAS-162], [STU-RAS-167]-[STU-RAS-169] | 7 | 7 | 1 | 12 | 0 | 0 | 20 |
| 14.4 §9 Adjustments | [STU-RAS-035]-[STU-RAS-037], [STU-RAS-151]-[STU-RAS-153] | 6 | 6 | 0 | 34 | 0 | 0 | 40 |
| 14.4 §10 Channels, colour modes, and bit depth | [STU-RAS-031]-[STU-RAS-034], [STU-RAS-144]-[STU-RAS-150] | 11 | 11 | 2 | 0 | 0 | 0 | 13 |
| 14.4 §11 Transforms and content-aware operations | [STU-RAS-029], [STU-RAS-030], [STU-RAS-141]-[STU-RAS-143] | 5 | 4 | 2 | 10 | 0 | 0 | 16 |
| 14.4 §12 Blend modes | [STU-RAS-038], [STU-RAS-039], [STU-RAS-154], [STU-RAS-155] | 4 | 4 | 0 | 35 | 0 | 0 | 39 |
| 14.4 §13 Layer effects and advanced blending | [STU-RAS-040], [STU-RAS-041], [STU-RAS-156]-[STU-RAS-159] | 6 | 6 | 1 | 11 | 0 | 0 | 18 |
| 14.4 §14 Provider, cloud, and generative posture | [STU-RAS-044]-[STU-RAS-046], [STU-RAS-164] | 4 | 4 | 0 | 0 | 0 | 0 | 4 |
| 14.4 §15 Diagnostics, export touchpoints, domain authority | [STU-RAS-042], [STU-RAS-043], [STU-RAS-047]-[STU-RAS-050], [STU-RAS-163], [STU-RAS-171], [STU-RAS-172] | 9 | 9 | 1 | 0 | 0 | 0 | 10 |
| 14.12 §1 Pipeline, parameter surface, and the two laws | [STU-RAW-001], [STU-RAW-002], [STU-RAW-100]-[STU-RAW-106] | 9 | 9 | 1 | 0 | 0 | 0 | 10 |
| 14.12 §2 Raw input scope and sensor decode | [STU-RAW-003], [STU-RAW-004] | 2 | 2 | 0 | 0 | 0 | 0 | 2 |
| 14.12 §2a Retained group-scope clauses | [STU-RAW-005]-[STU-RAW-008], [STU-RAW-008a], [STU-RAW-009]-[STU-RAW-012] | 9 | 9 | 0 | 0 | 0 | 0 | 9 |
| 14.12 §3 White balance | [STU-RAW-110] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §4 Basic tone and presence | [STU-RAW-111] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §5 Tone curve | [STU-RAW-112], [STU-RAW-113] | 2 | 2 | 1 | 0 | 0 | 0 | 3 |
| 14.12 §6 Detail — sharpening and noise | [STU-RAW-114] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §7 Colour mixer, HSL, and grayscale mix | [STU-RAW-115] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §8 Colour grading | [STU-RAW-116] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §9 Optics — lens correction, defringe, distortion | [STU-RAW-117] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §10 Geometry and perspective | [STU-RAW-118] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §11 Effects — grain and post-crop vignette | [STU-RAW-119] | 1 | 1 | 1 | 0 | 0 | 0 | 2 |
| 14.12 §12 Calibration | [STU-RAW-120] | 1 | 1 | 0 | 0 | 0 | 0 | 1 |
| 14.12 §13 Camera profiles, creative profiles, and presets | [STU-RAW-013], [STU-RAW-125]-[STU-RAW-128] | 5 | 5 | 0 | 0 | 1 | 0 | 6 |
| 14.12 §14 Masking and local adjustment | [STU-RAW-014a], [STU-RAW-014b], [STU-RAW-121]-[STU-RAW-123] | 5 | 5 | 0 | 4 | 0 | 0 | 9 |
| 14.12 §15 Local repair and distraction removal | [STU-RAW-015], [STU-RAW-124] | 2 | 2 | 0 | 0 | 0 | 0 | 2 |
| 14.12 §16 HDR, lens blur, and enhance | [STU-RAW-014], [STU-RAW-129], [STU-RAW-130] | 3 | 3 | 0 | 0 | 0 | 0 | 3 |
| 14.12 §17 Crop, workflow output, linkage to the raster document | [STU-RAW-016], [STU-RAW-017], [STU-RAW-131] | 3 | 3 | 0 | 0 | 0 | 0 | 3 |
| 14.12 §18 Model steerability, headless operation, validation | [STU-RAW-018], [STU-RAW-132] | 2 | 0 | 0 | 0 | 0 | 2 | 2 |
| 14.12 §19 Microtask derivation | [STU-RAS-173]-[STU-RAS-179] | 7 | 0 | 0 | 0 | 0 | 0 | 0 |
| **Module total** | — | 181 | 167 | 33 | 146 | 2 | 2 | 350 |

**[STU-RAS-179] RECONCILIATION AND ANCHOR BINDING.** This module yields exactly 350 microtasks. That
total is the module's own count, made by applying [STU-RAS-175] to every clause and every marked table
in the text above; a derivation tool producing a different total has diverged from this sub-section
and MUST be reconciled against it, not the reverse. Three reconciliation facts are recorded here so a
later reader can tell what was fixed and why the number moved:

- **What the ledger initially over-claimed, and how it was closed.** Nine tables catalogued separate
  implementable subjects — the selection, painting and retouching tool sets, the adjustment kinds, the
  blend modes, the layer-effect kinds, the preset-family registries and the develop mask geometry
  sources — but named their first column with a word that read as a facet of one subject rather than
  as a subject. Those 146 subjects were work this module claimed and its own text did not express
  derivably. They were closed by ENUMERATING them: each first column now names the subject, each table
  carries a catalogue marker with its row count, and the five additional blend modes of [STU-RAS-039]
  were lifted out of prose into a table of their own so that they are cut like their thirty siblings.
  No ledger number was reduced to make the totals meet.
- **What was split wrongly, and how it was stated.** Two tables are pure clause indexes — the preset
  family registry's contract column and the develop parameter surface's clause column — whose cells
  cite clauses that already yield their own microtasks. Cutting a microtask from those cells would
  have counted the same work twice. They now carry an explicit marker and their citations read as
  references rather than as definitions.
- **Parameter tables are one unit each and never split.** The 33 parameter tables here yield 33
  microtasks, not 324, which is the number of parameter rows they carry between them. The rows are
  acceptance criteria of their table's microtask, exactly as [STU-RAS-175] rule 2 states. All 33
  carry `hard_min`, `hard_max`, `soft_min`, `soft_max`, `default`, `unit` and `precision` as seven
  separate columns; a table that drops one of them is invisible to derivation and its parameters
  vanish from the work entirely, which is why the count of columns is itself a normative property.

A microtask derived from this module cites its clause anchor directly. A microtask staged before this
module landed carries `spec_anchor_status = "PROVISIONAL"`; binding it to an anchor defined here
clears that status. A microtask that cannot cite an anchor defined in this module is out of scope for
the raster and develop domains and MUST be re-derived or retired, not activated — and a microtask
citing an anchor this module states was never assigned, such as the case recorded in [STU-RAS-101],
MUST be re-bound to the anchor that carries its behaviour before it can be validated at all.









