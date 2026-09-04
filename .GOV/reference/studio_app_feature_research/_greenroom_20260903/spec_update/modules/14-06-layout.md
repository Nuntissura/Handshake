---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206-draft"
bundle_id: "master-spec-v02.206"
module_id: "14-06"
section_id: "14.6"
title: "14.6 Studio -- Page Layout & Publishing"
supersedes: "master-spec-v02.205 spec-modules/14-studio-creative-suite.md lines 896-1239 (sub-section 14.6)"
derivation_basis: "green-room installed-application captures, 2026-09-03/04"
declared_yields_total: 217
yields_ledger_clause: "STU-LAY-199"
anchor_prefix: "STU-LAY"
metadata_rule: "frontmatter is machine metadata; body follows after this block. body_sha256 and source_body_original_sha256 are assigned at bundle assembly per [CX-105D]."
---

# 14.6 Page Layout & Publishing

[REWRITE v02.206] This sub-section replaces the v02.205 text of 14.6 in full. The superseded text
was written from vendor help pages. This text is written from the parsed binaries of the installed
page-layout application: its scripting object model recovered from the binary scripting-element
resources, its dialog and panel control trees, its shipped export presets read key by key, its menu
and action resources, and its error tables. Every numeric bound, default, unit, enumerated value and
constraint below is transcribed from one of those captures; none is invented. Anchors
`[STU-LAY-001]` through `[STU-LAY-067]` that still state a true requirement are preserved here;
where a captured behaviour contradicts one, the clause says so and the new anchor supersedes the
contradicted part.

Page Layout & Publishing is the Studio domain catalog for multi-page, print-and-publish document
construction: the page/spread and parent-page model, threaded text stories, placed-graphic frames
and their linked resources, tables, the layout-facing style system, long-document assembly (books,
tables of contents, indexes, notes, cross-references), grids and guides, and the prepress/output
pipeline (preflight, packaging, print, PDF/X, separations, data merge). Per [STU-SECTION-003] each
shared capability collapses to exactly ONE Studio primitive and ONE command family, and no source
product, panel, format or menu name is a Studio name.

This catalog operates entirely on the shared Studio primitive set of 14.3 and MUST NOT introduce a
parallel layout document model. A layout document is a `StudioDocument` ([STU-DOC-001]) whose
containers are `StudioPageSpread` nodes; page furniture, frames, tables and placed assets are
`StudioLayer` nodes; flowed copy is `StudioTextStory`; every named format is a record in
`StudioStyleRegistry`; every ruler/baseline/column construct is a `StudioLayoutGrid`; every
render-to-output configuration is a `StudioExportRecipe`. Field-level definitions for every type,
enum, event, table and validation check named here are owned by 14.23; where this catalog and 14.23
conflict, 14.23 wins.

Domain boundaries this catalog holds:

- The **typography engine** (glyph shaping, composers, OpenType, kerning/tracking, hyphenation and
  justification internals, `StudioTypeStyle` attribute semantics) is owned by 14.7. This catalog owns
  the APPLICATION of type styles inside layout and references 14.7 for the attribute payloads.
- **Vector geometry and path editing** (frame outlines as paths, stroke geometry, text-on-path curve
  maths, boolean shape building) are owned by 14.5. This catalog owns frames AS LAYOUT CONTAINERS.
- **Raster/placed-image pixel editing** is owned by 14.4; this catalog owns placement, linking and
  fitting of raster assets.
- **Colour pipeline and profiles** are owned by 14.8; this catalog owns the prepress SURFACES that
  drive that pipeline to output.
- **Object effects and transparency** share the `StudioEffectStack` primitive with 14.9; this catalog
  states their layout-frame targeting and carries their captured parameter bounds.
- **Interactive, multi-state, media and EPUB export** touchpoints are owned by 14.11; this catalog
  defines the layout-side authoring of those objects and hands off export.
- **Per-file history and undo** are owned by 14.19.

Durable layout authority is SurrealDB/EventLedger only ([STU-SDB-002]); no second store, cache or
fixture database is permitted anywhere in this domain, including tests. Bulk binary that layout
authority references -- placed images, embedded fonts collected by packaging, PDF page caches --
lives in content-addressed artifact storage with SurrealDB holding the records and references.

---

## 14.6.0 Reading Rules: The Parameter Contract, Units, Enumerations, and Derivation

**[STU-LAY-100] The seven-field numeric parameter contract.** Every numeric parameter defined
anywhere in 14.6 MUST be declared with the same SEVEN INDEPENDENT FIELDS defined in [STU-VEC-100],
restated here so this sub-section stands alone: `hard_min` and `hard_max` (the values the engine
accepts; exceeding them is a validation error, not a clamp), `soft_min` and `soft_max` (the range
the default control presents; a user or model MAY type past them but not past the hard bounds),
`default`, `unit`, and `precision`. The two ranges MUST NOT be collapsed. Where a bound is not
declared by the capture the table says `unknown`, and an implementer MUST NOT substitute the other
range, MUST NOT clamp to the soft range, and MUST NOT invent a limit. `step`, `coarse_step` and
`fine_step` follow [STU-VEC-103].

The layout capture is unusually rich in HARD bounds because the source application declares them in
prose on the property itself ("Range: 0 to 100", "Range depends on unit. For points: 0.0 to 8.0").
Where a captured range is expressed PER UNIT, the row records the point-based bound as the canonical
value and names the unit conversion; a converter MUST derive the other units from the canonical
value and MUST NOT store per-unit constants.

**[STU-LAY-101] Measurement systems and unit vocabulary.** SUPERSEDES the ten-item list of
v02.205 [STU-LAY-053], which named "points, picas, inches, decimal inches, millimeters,
centimeters, ciceros, agates, pixels, and a custom unit". The captured measurement-system
enumeration has TWELVE members and does not contain a pixel member:

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Studio member | In the captured measurement enumeration |
|---|---|
| `measure.points` | yes |
| `measure.picas` | yes |
| `measure.inches` | yes |
| `measure.inches_decimal` | yes |
| `measure.millimeters` | yes |
| `measure.centimeters` | yes |
| `measure.ciceros` | yes |
| `measure.agates` | yes |
| `measure.q` | yes -- omitted by v02.205 |
| `measure.ha` | yes -- omitted by v02.205 |
| `measure.american_points` | yes -- omitted by v02.205 |
| `measure.custom` | yes |

Separately, the captured constraint vocabulary declares the set of unit SUFFIXES accepted in a
typed measurement string: `cm`, `mm`, `in`, `pt`, `%`, `px`, `vh`, `vw`, `vmin`, `vmax`. `px` and
the four viewport-relative units are therefore legal INPUT units but are not selectable measurement
SYSTEMS. Studio MUST carry both facts: the twelve-member ruler enumeration and the ten-member input
suffix set. Rejecting an unrecognised suffix is a typed error ([STU-LAY-166]), not a silent
fallback.

Measurement systems are selectable PER AXIS (horizontal and vertical independently). Every
length-bearing layout field MUST carry an explicit unit per [STU-DOC-003]; the document declares a
default layout unit and mixed-unit fields are forbidden. The canonical stored unit for layout
geometry is `pt`.

Ruler origin is a three-member enumeration: `{ruler_origin.spread, ruler_origin.page,
ruler_origin.spine}`.

**[STU-LAY-102] Enumeration contract.** As [STU-VEC-104]: every enumerated parameter MUST declare
its complete member list; each member carries a stable Studio identifier plus the token or integer
the capture recorded. Studio identifiers are Handshake-native. Where the capture recovered member
NAMES but not their integer values, the members are normative and the values are `unknown`; an
interchange writer MUST use the token form and MUST NOT invent integers.

**[STU-LAY-103] Capture-conflict rule.** As [STU-VEC-105]: where two captures of the same
application disagree (this occurs in the layout capture between the scripting object model and the
error/constraint tables), 14.6 MUST record BOTH and MUST name which is normative for Studio. A
conflict is never resolved by silently preferring one source.

**[STU-LAY-104] Microtask derivation rule.** As [STU-VEC-107]: exactly one microtask is derived
per numbered clause that introduces implementable behaviour, plus one per operation family
named in a clause's table. Reading rules ([STU-LAY-100] to [STU-LAY-104]), the cross-cutting
clause [STU-LAY-067], and the derivation index [STU-LAY-199] derive none. Each derived
microtask carries its clause's parameter table verbatim and its enumerations in full, and cites
this sub-section as its authority, never the green-room corpus ([STU-SECTION-002]). The
derivation index is [STU-LAY-199].

---

## 14.6.1 Page and Spread Model

**[STU-LAY-001]** A layout `StudioDocument` MUST hold an ordered set of `StudioPageSpread` containers. A
`StudioPageSpread` holds one or more pages; a facing-pages document pairs pages across a binding
spine, and a non-facing document holds single-page spreads. The spread is the unit of parent-page
application, spanning-object placement and print imposition.

**[STU-LAY-105] Document geometry contract.** The captured document-preference and document-preset
objects declare the complete new-document and document-setup surface. Studio MUST carry every field
below; each is a document-authority value, not a view preference.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `pages_per_document` | 1 | 9999 | unknown | unknown | unknown | `count` | 0 |
| `page_width` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `page_height` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `bleed_top` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `bleed_bottom` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `bleed_inside_or_left` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `bleed_outside_or_right` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `slug_top` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `slug_bottom` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `slug_inside_or_left` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `slug_outside_or_right` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `start_page_number` | 1 | 999999 | unknown | unknown | 1 | `count` | 0 |
| `column_count` | unknown | unknown | unknown | unknown | unknown | `count` | 0 |
| `column_gutter` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |

Booleans and enumerations on the same objects:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Members / default |
|---|---|---|
| `facing_pages` | boolean | default unknown |
| `bleed_uniform` | boolean | captured default `true`; when `true` the top bleed value governs all four sides |
| `slug_uniform` | boolean | captured default `false`; when `false` each slug side is independent |
| `page_orientation` | enumeration | `orientation.landscape`, `orientation.portrait` |
| `page_binding` | enumeration | `binding.default`, `binding.right_to_left`, `binding.left_to_right` |
| `document_intent` | enumeration | `intent.print`, `intent.web`, `intent.mobile` |
| `create_primary_text_frame` | boolean | see [STU-LAY-008] |
| `column_guides_locked` | boolean | default unknown |
| `overprint_black_on_save` | boolean | default unknown |
| `page_size` | string | a named preset identifier, or a free string for custom |

`bleed_uniform` and `slug_uniform` have OPPOSITE captured defaults. That asymmetry is real behaviour
and MUST be preserved: a new document has one bleed value driving four sides but four independent
slug values. An implementation that makes both uniform, or both independent, is non-conformant.

**[STU-LAY-106] Spread page-count contract.** SUPERSEDES the v02.205 [STU-LAY-002] statement that a
spread supports "up to a document-configured maximum of at least ten" pages. NO such number exists
in the capture. What the capture declares is:

- a spread carries a `page_count` and a `binding_location` (the index of the binding spine within
  the spread);
- an `allow_page_shuffle` flag whose captured semantics are exact: when TRUE it "guarantees that
  when pages are added to a spread it will contain a maximum of TWO pages"; when FALSE it "allows
  pages to be added or moved into existing spreads";
- a `preserve_layout_when_shuffling` flag whose captured semantics are: when TRUE it preserves the
  layout of spreads that already held more than two pages at the moment shuffle was enabled; when
  FALSE it converts multi-page spreads back to two-page spreads if they were created or changed
  since shuffle was enabled;
- `allow_page_shuffle` exists at BOTH document scope and spread scope, so a single island spread can
  opt out while the rest of the document repaginates normally;
- two distinct typed limit errors: a publication page limit and a SPREAD page limit, both of whose
  messages carry a substituted numeric limit rather than a literal.

Studio's normative behaviour: multi-page (island) spreads are supported; the maximum pages per
spread is a CONFIGURED LIMIT surfaced through a typed error carrying the limit value, and Studio MUST
expose that limit as a readable configuration value so a model can check before acting rather than
fail after. Studio MUST NOT hard-code ten, and MUST NOT hard-code two.

**[STU-LAY-002]** Multi-page (island) spreads are normative: a `StudioPageSpread` MUST support holding
more than two pages to model gatefold, trifold, accordion and other fold formats, and a per-document
and per-spread allow-shuffle flag MUST control whether repagination may reflow pages into or out of a
spread; disabling shuffle preserves an island spread during page insertion and deletion. SUPERSEDED
IN PART by [STU-LAY-106]: the v02.205 wording "up to a document-configured maximum of at least ten"
is withdrawn, because no such number exists in the capture and the limit is a configured value
surfaced through a typed error.

**[STU-LAY-003]** Pages MUST support mixed sizes and orientations within one document. A page carries
its own trim size, orientation, margins, bleed, slug and liquid-layout rule independent of sibling
pages. Page and spread operations -- insert, move, duplicate, delete, reorder by drag, hide/unhide
from view and output, apply colour labels, and a move/copy-pages operation for precise placement --
MUST be exposed as typed commands and MUST emit `studio.layout` EventLedger events.

**[STU-LAY-107] Page object contract.** The captured page object declares these fields; Studio MUST
carry all of them:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Notes |
|---|---|---|
| `document_offset` | integer | The page's sequential position in the document, independent of its section-relative number. |
| `applied_section` | reference | The section governing this page's number ([STU-LAY-108]). |
| `applied_alternate_layout` | reference | The alternate-layout section this page belongs to ([STU-LAY-112]). |
| `applied_parent` | reference | The parent spread applied to this page ([STU-LAY-110]). |
| `parent_transform` | matrix | The transform applied to the parent page before it is applied to this page. Studio MUST carry it; parent application is not necessarily an identity placement. |
| `override_list` | list of references | The overridden parent items on this page ([STU-LAY-007]). |
| `parent_page_items` | list of references | Items originating on the parent that have NOT been overridden or detached. Distinct from `override_list`; both are required. |
| `side` | enumeration | `page_side.right_hand`, `page_side.left_hand`, `page_side.single_sided`. |
| `page_colour_label` | colour or UI-colour token | Non-printing organisational label. |
| `layout_rule` | enumeration | [STU-LAY-111]. |
| `snapshot_blending_mode` | enumeration | `snapshot.ignore`, `snapshot.use_nearest`, `snapshot.limited`, `snapshot.full` -- governs how stored layout snapshots interpolate when page geometry changes. |
| `grid_starting_point` | enumeration | `grid_start.top_outside`, `grid_start.top_inside`, `grid_start.bottom_outside`, `grid_start.bottom_inside`, `grid_start.center_vertical`, `grid_start.center_horizontal`, `grid_start.center_completely`. |
| `use_parent_grid` | boolean | Inherit the layout grid from the applied parent. |
| `applied_trap_preset` | reference | [STU-LAY-152]. |
| `tab_order` | ordered list | Focus order for interactive form fields ([STU-LAY-165]). |

Page-scoped commands captured on the same object and required by Studio: move (with a location
option and a binding option), duplicate, add, delete, reframe, resize, transform, adjust layout
([STU-LAY-112]), remove override, detach, select, snapshot current layout, delete layout snapshot,
delete all layout snapshots. The three snapshot commands are the storage behind
`snapshot_blending_mode` and MUST be exposed to models, not only to the UI.

**[STU-LAY-004]** View-only spread rotation (90, 180, 270 degrees) MUST be available for editing rotated
content without transforming the underlying objects; the rotation is a view state on the
`StudioPageSpread` and MUST NOT change stored object geometry.

**[STU-LAY-005]** A layout document MUST carry a document-wide layer model: named layers span every
page/spread and MUST reorder every page's objects together when reordered.

**[STU-LAY-109] Document layer contract.** The captured layer object declares exactly these fields;
Studio MUST carry all of them and MUST NOT reduce them to visible/locked:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Notes |
|---|---|---|
| `name` | string | Must be unique; a duplicate name is a typed error ([STU-LAY-166]). |
| `visible` | boolean | |
| `locked` | boolean | A move onto a locked layer is a typed error, as is moving locked items to another layer. |
| `printable` | boolean | Independent of `visible`: a layer may be visible on screen and suppressed from output. |
| `layer_colour` | colour or UI-colour token | Selection-handle colour for items on this layer. |
| `show_guides` | boolean | Per-layer guide visibility. |
| `lock_guides` | boolean | Per-layer guide lock, independent of `show_guides`. |
| `ignore_text_wrap` | boolean | When true, objects on this layer do not displace text even when hidden -- the wrap-when-hidden policy. |
| `expendable` | boolean | Whether the layer may be discarded by cleanup/flatten operations. |

Layer commands: merge, add, move, duplicate, delete. Deleting the last remaining layer is a typed
error. A layer additionally exposes `all_page_items` and `all_graphics` as typed queries so a model
can enumerate a layer's contents without walking pages.

---

### 14.6.1.1 Parent (Master) Pages

**[STU-LAY-006]** Parent pages (the deduped Studio name for master pages) are reusable page templates
applied to document pages. The parent-page model MUST support: multiple parents applied to one page
simultaneously, each applied parent surfacing as its own `StudioLayer` band in the layer graph;
parent-based-on-parent inheritance, cascading changes; nested parents; application to page ranges;
and loading parents from another document.

**[STU-LAY-110] Parent application and override storage.** The captured page object stores parent
state as THREE independent fields that Studio MUST keep separate ([STU-LAY-107]): the applied parent
reference, the parent transform matrix, and TWO disjoint item lists -- the overridden items and the
still-inherited parent items. A model MUST be able to read both lists and MUST be able to distinguish
an item it has overridden from an item it has merely selected. The captured commands are
`remove_override` (restore inheritance) and `detach` (sever the link permanently); they are DIFFERENT
operations with different results and MUST NOT be merged. Both are available at page scope AND at
spread scope.

**[STU-LAY-007]** Parent item override semantics MUST be preserved exactly: override a single parent
item on a document page, override-all parent items on a page, detach an overridden item from its
parent, remove overrides, and a per-item `allow_overrides` flag that can be disabled so an item
cannot be locally altered. `allow_overrides` is a captured per-item boolean and is the mechanism by
which a template author locks furniture; Studio MUST honour it as a hard refusal, not a warning. The
captured constraint vocabulary names the specific failure "cannot unlink an item that was not
overridden", which Studio MUST reproduce as a distinct typed error.

**[STU-LAY-008]** A primary text frame MUST be supportable on a parent page: a designated text frame
that new pages auto-adopt, that re-threads automatically when the applied parent changes, and that
resizes to new page geometry without manual override. Primary text frames are the anchor for smart
text reflow ([STU-LAY-123]). The captured document preference `create_primary_text_frame` governs
whether a newly created document's first parent carries one.

### 14.6.1.2 Liquid Layout, Adjust Layout, and Alternate Layouts

**[STU-LAY-009]** Studio MUST implement responsive layout adaptation when page size, orientation,
margins or bleed change. Two mechanisms coexist: per-page liquid rules applied continuously as
geometry changes, and the on-demand adjust-layout operation.

**[STU-LAY-111] Liquid-rule enumeration.** SUPERSEDES the six-row informal table of
v02.205 [STU-LAY-009]. The captured enumeration has SEVEN members, and the seventh is not
in the v02.205 table:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Behaviour |
|---|---|
| `liquid.off` | No liquid adaptation; objects keep absolute geometry. |
| `liquid.scale` | All page content scales proportionally, preserving relative positions. |
| `liquid.recenter` | Content keeps original size and re-centres on the resized page. |
| `liquid.object_based` | Per-object pins to page edges plus resize constraints give mixed fixed/relative behaviour. |
| `liquid.guide_based` | Liquid guides slice the page; objects a guide crosses stretch while text reflows and images resize without distortion. |
| `liquid.use_parent` | The page inherits whatever liquid rule its parent page defines. |
| `liquid.preserve_existing` | Leave the page's existing rule untouched. Used when a rule is applied across a range and some pages must be skipped. |

The captured guide object carries a `guide_type` enumeration `{guide_type.ruler, guide_type.liquid}`
distinguishing an ordinary ruler guide from a liquid guide, plus a `guide_zone` (the snap radius).
Studio MUST carry both guide types on one primitive with that discriminator, not as two primitives.

**[STU-LAY-112] Adjust-layout contract.** The captured adjust-layout operation takes a PROPERTY BAG
naming only the values that change, and an optional page scope. Its permissible keys are exactly:
`width`, `height`, `bleedInside`, `bleedTop`, `bleedOutside`, `bleedBottom`, `leftMargin`,
`topMargin`, `rightMargin`, `bottomMargin`. Studio MUST accept a partial bag -- a caller supplies
only what changes -- and MUST accept each value either as a number in `pt` or as a measurement
string carrying any suffix from [STU-LAY-101]. Adjust layout is available at document scope, at a
page collection, and at a single page. The capture states one behavioural exception that Studio MUST
reproduce: when the operation is scoped to individual pages rather than the whole document, BLEED
changes have no effect.

**[STU-LAY-010]** Adjust layout MUST recompute object positions and sizes when page size, margins or
bleed change, with options to adjust font size (with a minimum and maximum limit), include locked
content and move ruler guides. Adjust layout is a discrete, undoable command distinct from the
continuous liquid rules. The font-size adjustment's minimum and maximum bounds, unit and default were
NOT recovered by the capture and are a declared SPEC GAP; the three booleans ARE required and MUST be
exposed.

**[STU-LAY-011]** Alternate layouts MUST be supported: multiple named page-size/orientation variants
coexist inside one `StudioDocument`, displayed side by side in the page-navigation surface, with
stories linked ([STU-LAY-019]) back to the source layout so edits can propagate. A flex/container
layout mode (container-based responsive layout with direction, wrap, alignment and spacing
properties) MUST be available with conflict reporting against fixed positioning.

The captured section object carries `alternate_layout` (a name string) and `alternate_layout_length`
(a page count), so an alternate layout is stored AS A SECTION, not as a separate document. Studio
MUST use that model. The captured constraint vocabulary declares two hard rules Studio MUST enforce:
an alternate-layout name MUST NOT be blank, and it MUST be unique and MUST NOT end with `*` or `:`.

The flex/container layout mode's parameters are owned by the design-system domain (14.10) via
`StudioAutoLayout`; the captured layout-suite menu confirms a flex-container command family exists in
this domain but its parameters were not recovered on this surface. The normative Studio behaviour is
that layout consumes `StudioAutoLayout` and MUST NOT fork a parallel container model
([STU-SECTION-003]).

**[STU-LAY-012]** Alignment and distribution MUST align/distribute selected objects to selection, a key
object, margins, page or spread, including distribute-by-spacing with explicit gap values. Gridified
drawing (splitting a single drag of any frame tool into an equal grid of frames via modifier keys)
MUST be supported. The captured align target enumeration is shared with the vector domain
([STU-VEC-166]) and MUST NOT be duplicated; layout adds `align_to.margins`, `align_to.page` and
`align_to.spread` as additional members, and the captured justification enumeration adds the two
spine-relative members `justify.to_binding_side` and `justify.away_from_binding_side` that only exist
in a facing-pages context.

**[STU-LAY-113] Margin and column contract.** The captured margin-preference object is per page (and
per parent page), not per document:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Kind | Notes |
|---|---|---|
| `margin_top`, `margin_bottom`, `margin_left`, `margin_right` | numeric, `pt` | On a facing-pages document `left`/`right` mean inside/outside. Bounds, defaults and precision UNKNOWN in the capture. |
| `column_count` | integer, `count` | |
| `column_gutter` | numeric, `pt` | |
| `column_direction` | enumeration | `{direction.horizontal, direction.vertical}` |
| `custom_columns` | boolean | When `false` columns are evenly spaced; when `true` each column may have its own width. |
| `column_positions` | array of numeric, `pt` | Distance of each column guide from the left margin, in order. Only meaningful when `custom_columns` is `true`. |

A spread additionally exposes a create-guides command taking `number_of_rows`, `number_of_columns`,
`row_gutter`, `column_gutter`, `guide_colour`, `fit_to_margins` (boolean) and `remove_existing`
(boolean), applying to every page of the spread in one operation. Studio MUST expose it as a typed
command, because it is the only captured bulk-guide operation and a model otherwise has to place
guides one at a time.

**[STU-LAY-013]** The document MUST provide crash-safe automatic recovery (recovering unsaved changes on
next launch from a configurable recovery location) and a document-states surface that lists session
edit states and can jump the document to any recorded state beyond linear undo. Both bind to the
kernel per-file history/undo model (14.19) and CRDT authority; Studio MUST NOT implement a private
undo store. The captured layout-snapshot commands of [STU-LAY-107] are the geometry-specific case of
this surface and MUST route through the same history authority.

---

## 14.6.2 Sections, Page Numbering, and Running Headers

**[STU-LAY-014]** Sections MUST partition a document for numbering.

**[STU-LAY-108] Section contract, with the captured constraints.** Two captures contribute and they
DISAGREE about the section prefix length; both are recorded per [STU-LAY-103].

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Bounds / members | default |
|---|---|---|---|
| `name` | string | -- | unknown |
| `page_start` | reference to a page | -- | -- |
| `length` | integer, `count` | derived from the next section's start | -- |
| `continue_numbering` | boolean | -- | unknown |
| `page_number_start` | integer, `count` | hard 1 .. 999999 | unknown |
| `page_number_style` | enumeration | see below | unknown |
| `include_section_prefix` | boolean | -- | unknown |
| `section_prefix` | string | scripting model says "may include up to 8 characters"; the constraint vocabulary says "specify a section prefix of no more than FIVE characters" | unknown |
| `marker` | string | The section marker text resolved by a section-marker variable. | unknown |
| `alternate_layout` | string | [STU-LAY-011] | unknown |
| `alternate_layout_length` | integer, `count` | [STU-LAY-011] | unknown |

The normative Studio `section_prefix` limit is FIVE characters, because the constraint vocabulary is
the enforcement surface and the scripting-model note is documentation. The eight-character figure is
recorded so the conflict is not lost and so an importer knows a five-to-eight character prefix may
arrive from a source document and MUST be reported rather than silently truncated.

Further captured section constraints Studio MUST enforce as distinct typed errors: a section prefix
MUST NOT contain `,` or `+`; two sections MUST NOT share a prefix; two sections MUST NOT start on the
same page; the default section MUST NOT be deleted; `page_number_start` MUST NOT be set while
`continue_numbering` is `true` (the capture states "must set continue numbering as false before
changing page number start"), which makes the two fields ORDER-DEPENDENT and a model MUST be told so;
and an empty section name is rejected.

The captured page-number-style enumeration has fourteen members in one declaration and seventeen in
another (the seventeen-member form repeats three members, which is a capture artefact and not three
extra styles). The normative Studio set is the fourteen distinct members:

`number_style.upper_roman`, `number_style.lower_roman`, `number_style.upper_letters`,
`number_style.lower_letters`, `number_style.arabic`, `number_style.kanji`,
`number_style.arabic_alif_ba_tah`, `number_style.arabic_abjad`, `number_style.hebrew_biblical`,
`number_style.hebrew_non_standard`, `number_style.single_leading_zeros`,
`number_style.double_leading_zeros`, `number_style.triple_leading_zeros`,
`number_style.full_width_arabic`.

A separate captured list numbering enumeration (used for bulleted and numbered lists, [STU-LAY-125])
has sixteen members: the fourteen above minus the three leading-zero variants and
`full_width_arabic`, plus `number_style.katakana_modern`, `number_style.katakana_traditional`,
`number_style.format_none`, `number_style.single_leading_zeros`,
`number_style.double_leading_zeros`, `number_style.triple_leading_zeros`. Studio MUST carry ONE
numbering-style enumeration that is the UNION, and MUST declare per consumer which members are legal
there, rather than shipping two enumerations that mostly overlap.

A third captured variable numbering enumeration (used by text variables, [STU-LAY-151]) has twelve
members and adds `number_style.current` meaning "whatever style the containing section declares".
`number_style.current` is legal ONLY on a text variable.

A per-section include-on-export flag is required by v02.205 [STU-LAY-014] and was NOT found on the
captured section object; it is a declared SPEC GAP. The captured spread object does carry
`spread_hidden`, and the captured document preference carries `spread_hidden_visibility`, which
together provide hide-from-view-and-output at spread granularity; Studio MUST implement those and
MUST raise the per-section flag as a gap rather than conflate the two.

**[STU-LAY-015]** Automatic page-number markers MUST resolve to the current page's section number
wherever placed (parent page, running header, TOC). A last-page-number marker with selectable
section or document scope MUST resolve for "page X of Y" constructs. Numbering markers are
text-variable records ([STU-LAY-151]) resolved at composition time. The captured variable scope
enumeration is `{variable_scope.document, variable_scope.section}`.

**[STU-LAY-016]** Running headers and footers MUST be content-derived: a header/footer field pulls the
first or last on-page text carrying a chosen paragraph or character style, with
delete-trailing-punctuation and change-case options. Running headers resolve per page against the
composed `StudioTextStory` and MUST update as content reflows. The captured variable types confirm
TWO distinct running-header variables -- one matching a character style and one matching a paragraph
style -- and Studio MUST carry them as two variable types, not one with a mode flag, because the
captured type enumeration distinguishes them. The captured change-case enumeration is
`{case.none, case.uppercase, case.lowercase, case.titlecase, case.sentencecase}`.

---

## 14.6.3 Text Frames and Story Threading

**[STU-LAY-017]** Flowed copy is a `StudioTextStory` rendered through one or more threaded text frames
(`StudioLayer` nodes of text kind).

**[STU-LAY-118] Text-frame preference contract.** The captured text-frame preference object is the
complete frame-behaviour surface. Studio MUST carry every field; the v02.205 [STU-LAY-017] list
omitted the flexible-column-width family, the footnote overrides and the column-rule inset chain.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Bounds / members | default |
|---|---|---|---|
| `text_column_count` | integer, `count` | unknown | unknown |
| `text_column_gutter` | numeric, `pt` | unknown | unknown |
| `text_column_fixed_width` | numeric, `pt` | unknown | unknown |
| `use_fixed_column_width` | boolean | when `true`, resizing the frame changes the NUMBER of columns rather than their width | unknown |
| `use_flexible_column_width` | boolean | when `true`, column width floats between a minimum and `text_column_max_width`; resizing may change the column count | unknown |
| `text_column_max_width` | numeric, `pt` | `0` means "no upper limit" -- a sentinel, not a zero width | unknown |
| `inset_spacing` | one value or four values `[top, left, bottom, right]`, `pt` | a scalar applies uniformly | unknown |
| `first_baseline_offset` | enumeration | [STU-LAY-120] | unknown |
| `minimum_first_baseline_offset` | numeric, `pt` | unknown | unknown |
| `vertical_justification` | enumeration | [STU-LAY-121] | unknown |
| `vertical_threshold` | numeric, `pt` | maximum vertical space inserted between two paragraphs; valid only when vertical justification is `vjust.justify`; ADDS to the paragraph's own space-before/space-after rather than replacing it | unknown |
| `vertical_balance_columns` | boolean | vertically justify balanced across all columns | unknown |
| `ignore_text_wrap` | boolean | this frame ignores wrap from objects above it | unknown |
| `auto_sizing_type` | enumeration | [STU-LAY-119] | unknown |
| `auto_sizing_reference_point` | enumeration | nine-position anchor; the capture states the reference point is AUTOMATICALLY adjusted to a compatible value when the type changes (for example top-left becomes top-centre for a height-only type) | unknown |
| `use_minimum_height_for_auto_sizing` | boolean | gate | unknown |
| `minimum_height_for_auto_sizing` | numeric, `pt` | unknown | unknown |
| `use_minimum_width_for_auto_sizing` | boolean | gate | unknown |
| `minimum_width_for_auto_sizing` | numeric, `pt` | unknown | unknown |
| `use_no_line_breaks_for_auto_sizing` | boolean | suppress line breaks introduced by auto-sizing | unknown |
| `column_rule_override` | boolean | gate for the whole column-rule group | unknown |
| `column_rule_stroke_width` | numeric | unknown | unknown |
| `column_rule_stroke_colour` | swatch reference | -- | unknown |
| `column_rule_stroke_type` | stroke-style reference | -- | unknown |
| `column_rule_stroke_tint` | numeric, `percent` | hard 0..100 by the shared tint bound of [STU-LAY-140] | unknown |
| `column_rule_offset` | numeric | unknown | unknown |
| `column_rule_top_inset` | numeric | unknown | unknown |
| `column_rule_bottom_inset` | numeric | unknown | unknown |
| `column_rule_inset_chain_override` | boolean | links the two insets | unknown |
| `column_rule_overprint_override` | boolean | -- | unknown |
| `footnotes_enable_overrides` | boolean | gate for per-frame footnote options | unknown |
| `footnotes_span_across_columns` | boolean | straddling footnotes | unknown |
| `footnotes_minimum_spacing` | numeric, `pt` | minimum space before the first footnote | unknown |
| `footnotes_space_between` | numeric, `pt` | space between footnotes | unknown |
| `frame_type` | enumeration | `frame.text`, `frame.grid`, `frame.unknown` -- a frame may be an ordinary text frame or a character-grid frame ([STU-LAY-117]) | unknown |
| `story_orientation` | enumeration | `{orientation.horizontal, orientation.vertical, orientation.unknown}` | unknown |

**[STU-LAY-119] Auto-sizing enumeration.**

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Behaviour |
|---|---|
| `autosize.off` | Frame geometry is fixed. |
| `autosize.height_only` | Height grows and shrinks with content. |
| `autosize.width_only` | Width grows and shrinks with content. |
| `autosize.height_and_width` | Both dimensions float independently. |
| `autosize.height_and_width_proportionally` | Both float, preserving the frame's aspect ratio. |

**[STU-LAY-120] First-baseline enumeration.**

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Behaviour |
|---|---|
| `first_baseline.ascent` | Offset by the font ascent. |
| `first_baseline.cap_height` | Offset by the capital height. |
| `first_baseline.leading` | Offset by the line's leading. |
| `first_baseline.em_box_height` | Offset by the em box height. |
| `first_baseline.x_height` | Offset by the x height. |
| `first_baseline.fixed` | Offset by an explicit `minimum_first_baseline_offset` value. |

The identical six-member enumeration governs a footnote container's first baseline and a table
cell's first baseline; Studio MUST use ONE enumeration in all three places ([STU-SECTION-003]).

**[STU-LAY-121] Vertical justification enumeration.** `{vjust.top, vjust.center, vjust.bottom,
vjust.justify}`. Only `vjust.justify` activates `vertical_threshold` and
`vertical_balance_columns`.

**[STU-LAY-018]** Story threading MUST let a single `StudioTextStory` flow through an ordered chain of
frames across pages and spreads; cutting or reordering the thread re-flows the story.

**[STU-LAY-122] Threading contract.** The captured text-frame object exposes the thread as five
typed references that Studio MUST carry: `parent_story`, `start_text_frame`, `end_text_frame`,
`previous_text_frame`, `next_text_frame`, plus `text_frame_index` (this frame's ordinal within the
story) and `overflows` (a boolean that is `true` when the story has overset text). A model MUST be
able to walk the chain in both directions and MUST be able to test for overset without rendering.
In/out ports on each frame MUST expose the chain in the operator UI, and a threads-view overlay MUST
visualise flow order.

**[STU-LAY-019]** Linked stories (place-and-link a child copy of a story) MUST show update state and
support auto-update or warn-on-parent-change, so the same copy can appear in multiple layouts and
alternate layouts. The captured linked-story option object and linked-page-item option object are
separate; Studio MUST carry both, because a linked STORY and a linked PAGE ITEM have different update
semantics. The captured link import policy enumeration is
`{import_policy.no_auto_import, import_policy.import_on_modify}` and the export policy enumeration is
`{export_policy.no_auto_export, export_policy.export_on_modify, export_policy.export_on_close,
export_policy.export_on_save}`.

**[STU-LAY-020]** Autoflow placement modes MUST be supported when placing a loaded text cursor, and are
enumerable; smart text reflow MUST automatically add or remove pages as a threaded story grows or
shrinks. The enumerations, the six reflow controls and their gating are [STU-LAY-123].

**[STU-LAY-123] Autoflow and smart text reflow contract.** SUPERSEDES nothing in
v02.205 [STU-LAY-020] and completes it with the captured reflow controls, which the
v02.205 text described only as "smart text reflow" with no fields.

The four flow modes are preserved as normative:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Behaviour |
|---|---|
| `flow.manual` | Places one frame's worth; the cursor reloads with the remainder. |
| `flow.semi_auto` | Places a frame and reloads the cursor to continue, without adding pages. |
| `flow.auto` | Adds frames and pages until the story ends. |
| `flow.fixed_page_auto` | Flows into existing pages only, without adding pages. |

The captured smart-reflow preference block carries six independent booleans that Studio MUST expose,
because their combinations produce materially different documents:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Behaviour |
|---|---|
| `smart_text_reflow` | Master enable for automatic page add/delete in response to reflow. |
| `smart_text_reflow_sync` | When enabled alongside the master, pages are added or deleted SYNCHRONOUSLY as text reflows rather than deferred. |
| `limit_to_primary_text_frames` | Restrict page addition to overridden primary text frames ([STU-LAY-008]). |
| `delete_empty_pages` | Auto-delete pages that end up holding only empty threaded frames. |
| `preserve_facing_page_spreads` | Preserve left/right pairing when facing pages are enabled. |
| `link_text_files_when_importing` | Placed text and spreadsheet files are linked rather than embedded. |

Captured defaults for all six are UNKNOWN and MUST NOT be assumed. An overset-text condition MUST be
detectable ([STU-LAY-122]) and is a preflight rule ([STU-LAY-154]).

**[STU-LAY-021]** Span and split columns MUST be a paragraph-level attribute: a paragraph may span all or
N columns of its frame, or split into sub-columns, with before/after spacing and inside/outside
gutter controls. The captured enumeration is `{span_column.single_column, span_column.span_columns,
span_column.split_columns}` with a separate count enumeration whose only captured member is
`span_count.all`; the numeric N is therefore a separate integer field alongside the `all` sentinel,
and Studio MUST carry both rather than encoding `all` as a magic number.

---

**[STU-LAY-125] Paragraph-level layout-flow attribute contract.** [STU-LAY-022] required these
attributes to be applied through the layout style system while their glyph-level rendering stays in
14.7. This clause gives them their captured bounds. All are stored on paragraph style records and
paragraph overrides.

Keep options and flow:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Members / bounds |
|---|---|---|
| `keep_with_previous` | boolean | -- |
| `keep_with_next` | integer, `count` | number of following lines to keep with this paragraph |
| `keep_all_lines_together` | boolean | -- |
| `keep_first_lines` | integer, `count` | widow control |
| `keep_last_lines` | integer, `count` | orphan control |
| `start_paragraph` | enumeration | `start.anywhere`, `start.next_column`, `start.next_frame`, `start.next_page`, `start.next_odd_page`, `start.next_even_page` |
| `balance_ragged_lines` | boolean | -- |
| `align_to_baseline_grid` | boolean | see [STU-LAY-115] |
| `hyphenate_across_columns` | boolean | -- |
| `hyphenate_last_word` | boolean | -- |

Justification numerics -- every one carries a DECLARED hard range and Studio MUST enforce all of
them. These are the layout-side storage; the composer that consumes them is owned by 14.7.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `minimum_word_spacing` | 0 | 1000 | unknown | unknown | unknown | `percent` | unknown |
| `desired_word_spacing` | 0 | 1000 | unknown | unknown | unknown | `percent` | unknown |
| `maximum_word_spacing` | 0 | 1000 | unknown | unknown | unknown | `percent` | unknown |
| `minimum_letter_spacing` | -100 | 500 | unknown | unknown | unknown | `percent` | unknown |
| `desired_letter_spacing` | -100 | 500 | unknown | unknown | unknown | `percent` | unknown |
| `maximum_letter_spacing` | -100 | 500 | unknown | unknown | unknown | `percent` | unknown |
| `minimum_glyph_scaling` | 50 | 200 | unknown | unknown | unknown | `percent` | unknown |
| `desired_glyph_scaling` | 50 | 200 | unknown | unknown | unknown | `percent` | unknown |
| `maximum_glyph_scaling` | 50 | 200 | unknown | unknown | unknown | `percent` | unknown |
| `auto_leading` | 0 | 500 | unknown | unknown | unknown | `percent` | unknown |
| `hyphen_weight` | 0 | 100 | unknown | unknown | unknown | `count` | 0 |
| `optical_margin_size` | 0.1 | 1296 | unknown | unknown | unknown | `pt` | unknown |

Studio MUST validate the ORDERING invariant the three-value groups imply --
`minimum <= desired <= maximum` for word spacing, letter spacing and glyph scaling -- and MUST report
an ordering violation as a distinct typed error, because each individual value can be inside its
hard range while the triple is invalid.

Justification alignment is the captured nine-member enumeration
`{justify.left, justify.center, justify.right, justify.left_justified, justify.right_justified,
justify.center_justified, justify.fully_justified, justify.to_binding_side,
justify.away_from_binding_side}`.

Paragraph rules, shading and borders -- each tint is bounded 0..100 `percent` and each line weight
0..1000 in its measurement unit:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field group | Fields |
|---|---|
| Rule above | `rule_above` (boolean), `rule_above_line_weight`, `rule_above_colour`, `rule_above_tint` (0..100 `percent`), `rule_above_gap_colour`, `rule_above_gap_tint` (0..100 `percent`), `rule_above_type`, `rule_above_offset`, `rule_above_left_indent`, `rule_above_right_indent`, `rule_above_width` (enumeration `{rule_width.text_width, rule_width.column_width}`), `rule_above_overprint`, `rule_above_gap_overprint` |
| Rule below | the same thirteen fields with `rule_below` prefixes |
| Paragraph shading | `paragraph_shading_on` (boolean), `paragraph_shading_tint` (0..100 `percent`), `paragraph_shading_top_origin`, `paragraph_shading_bottom_origin` |
| Paragraph border | `paragraph_border_tint` (0..100 `percent`), `paragraph_border_gap_tint` (0..100 `percent`), `paragraph_border_gap_colour`, `paragraph_border_gap_overprint`, plus per-side widths, corner shapes, offsets, merge-consecutive, clip-to-frame and do-not-print |

Drop caps: `drop_cap_lines` (integer, `count`), `drop_cap_characters` (integer, `count`),
`drop_cap_style` (a character style reference), and a scale-for-descenders boolean.

Hyphenation storage carried on the same record: `hyphenation` (boolean),
`hyphenate_capitalized_words`, `hyphenate_words_longer_than`, `hyphenate_after_first`,
`hyphenate_before_last`, `hyphenate_ladder_limit`, `hyphenation_zone`.

Indents and spacing: `first_line_indent`, `last_line_indent`, `left_indent`, `right_indent`,
`space_before`, `space_after`. Bounds, defaults and precisions for the indent and spacing family were
NOT recovered and are declared UNKNOWN.

**[STU-LAY-022]** Paragraph-level layout-flow attributes MUST be applied through the layout style
system even though their glyph-level rendering is owned by 14.7; the attribute set and its bounds
are [STU-LAY-125]. These attributes are stored on paragraph style records and paragraph overrides;
their composition is executed by the typography engine.

**[STU-LAY-023]** Text on a path MUST bind a `StudioTextStory` to a `StudioVectorPath` (geometry owned
by 14.5, [STU-VEC-038]) with path-text options (alignment to path, spacing at curves, flip, and
effect modes); this is the single Studio text-on-path capability shared with the vector and
typography domains and MUST NOT be reimplemented here.

**[STU-LAY-126] Text-wrap contract.** SUPERSEDES the five-row informal table of
v02.205 [STU-LAY-024] by giving the enumerations their captured member sets. TWO
captured enumerations exist and they differ by one member:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Enumeration | Members |
|---|---|
| `text_wrap_mode` (5 members) | `wrap.none`, `wrap.jump_object`, `wrap.next_column`, `wrap.bounding_box`, `wrap.contour` |
| `text_wrap_type` (6 members) | the same five plus `wrap.user_modified` |

Per [STU-LAY-103] both are recorded. The normative Studio enumeration is the SIX-member form:
`wrap.user_modified` is the state a wrap enters when its outline has been edited away from the
generated contour, and collapsing it into `wrap.contour` loses the fact that the outline is
hand-authored and MUST NOT be regenerated. The captured object additionally carries a
`user_modified_wrap` boolean recording the same fact; Studio MUST carry ONE of the two
representations and MUST document which, rather than both.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Members / bounds |
|---|---|---|
| `text_wrap_offset` | four values `[top, left, bottom, right]`, `pt` | bounds UNKNOWN |
| `inverse` | boolean | invert the wrap region |
| `text_wrap_side` | enumeration | `wrap_side.both`, `wrap_side.left`, `wrap_side.right`, `wrap_side.toward_spine`, `wrap_side.away_from_spine`, `wrap_side.largest_area` |
| `apply_to_parent_page_only` | boolean | when true, a wrap authored on a parent spread applies to that spread only and not to pages the parent is applied to |
| `path_geometry` | path in inner coordinates | the wrap outline, independently editable from the object's own geometry |
| `contour_source` | enumeration | the captured clipping-path type enumeration is reused: `clip.none`, `clip.detect_edges`, `clip.alpha_channel`, `clip.embedded_path`, `clip.user_modified` ([STU-LAY-133]) |

The layer-level `ignore_text_wrap` of [STU-LAY-109] and the frame-level `ignore_text_wrap`
of [STU-LAY-118] are DIFFERENT switches at different scopes and both are required.

**[STU-LAY-127] Anchored-object contract.** SUPERSEDES the informal positioning description of
v02.205 [STU-LAY-025] with the captured enumerations and fields.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Members / bounds |
|---|---|---|
| `anchored_position` | enumeration | `anchor_position.inline`, `anchor_position.above_line`, `anchor_position.custom` |
| `anchor_x_offset` | numeric, `pt` | bounds UNKNOWN |
| `anchor_y_offset` | numeric, `pt` | bounds UNKNOWN |
| `anchor_space_above` | numeric, `pt` | valid only for `anchor_position.above_line` |
| `horizontal_alignment` | enumeration | for `above_line` it is relative to the text area; for `custom` it selects the horizontal reference point. NOT valid for `inline`. |
| `horizontal_reference_point` | enumeration | `anchored_relative.column_edge`, `anchored_relative.text_frame`, `anchored_relative.page_margins`, `anchored_relative.page_edge`, `anchored_relative.anchor_location`. Valid only for `custom`. |
| `vertical_alignment` | enumeration | valid only for `custom` |
| `vertical_reference_point` | enumeration | valid only for `custom` |
| `anchor_point` | enumeration | nine-position: `anchor.top_left`, `anchor.top_center`, `anchor.top_right`, `anchor.left_center`, `anchor.center`, `anchor.right_center`, `anchor.bottom_left`, `anchor.bottom_center`, `anchor.bottom_right` |
| `spine_relative` | boolean | mirror the position across facing pages |
| `lock_position` | boolean | forbid manual repositioning |
| `pin_position` | boolean | keep the object within the text frame's top and bottom |

Commands: `insert_anchored_object` (taking an insertion point and an `anchor_position`) and
`release_anchored_object`. A default anchored-object settings record exists at document scope and
MUST be carried so new anchors inherit it. The captured constraint vocabulary declares that a member
of a group cannot be re-anchored and that an inline item cannot be moved beyond its container
boundary; both are distinct typed errors.

**[STU-LAY-024]** Text wrap MUST be a per-object property with the full option set of [STU-LAY-126].

**[STU-LAY-025]** Anchored/pinned objects MUST attach a frame to a position in text so it travels with
reflow, using the contract of [STU-LAY-127]. A pinning surface MUST manage inline and floating
anchored objects.

**[STU-LAY-026]** A text-only editing surface (story editor) MUST present a `StudioTextStory` as linear
text with a style column, depth indicator, overset marker, and inline display of notes, tracked
changes, tables and structure tags, editing the same story authority as the layout view. Placed-text
import MUST honour saved option sets: word-processor and rich-text import maps incoming styles
([STU-LAY-145]) or preserves them and carries footnotes, endnotes and tables; plain-text import
controls encoding, target dictionary and carriage-return cleanup; spreadsheet import selects sheet,
range and formatting mode.

**[STU-LAY-128] Spreadsheet-import contract.** The captured spreadsheet-import enumeration declares
exactly four table-formatting modes and Studio MUST carry all four:
`table_import.formatted_table`, `table_import.unformatted_table`,
`table_import.unformatted_tabbed_text`, `table_import.format_only_once`. A companion alignment-style
enumeration governs how incoming cell alignment is resolved:
`{align_style.from_source, align_style.left, align_style.right, align_style.center}`. Sheet name,
named view and cell range are string/reference fields on the same option set.

**[STU-LAY-027]** A find/change surface MUST operate across modes over layout content.

**[STU-LAY-129] Find/change contract.** The captured object model declares FIVE independent
find/change modes, each with its own preference object, its own find and change record, and its own
document-, story- and frame-level methods. Studio MUST implement all five as one command family with
a mode discriminator ([STU-SECTION-003]) and MUST NOT ship fewer:

*Derivation: catalogue table, splits per row; yields 5 microtasks, one per find/change mode.*

| Studio mode | Searches | Replaces |
|---|---|---|
| `find.text` | literal text with metacharacter tokens, case and whole-word toggles | text and formatting |
| `find.grep` | regular expressions with capture groups, lookarounds and location tokens | text, capture-group references and formatting |
| `find.glyph` | a glyph by glyph id or code point within a named font | a glyph |
| `find.object` | frames and objects by object formatting | object attributes or an object style |
| `find.transliterate` | text by character TYPE (script/width class) | a character type |

A colour find/change command exists at document scope as a sixth, narrower operation
(`find.colour`): it finds a colour usage and replaces it document-wide. Studio MUST expose it
separately because its scope is the swatch graph, not the text stream.

Saved queries are first-class: the capture declares eleven shipped query files plus the ability to
save, load and delete named queries, and four distinct typed errors -- invalid query name, query file
not found, query file locked or read-only, invalid search mode -- that Studio MUST reproduce. Search
scope MUST offer all documents, document, story, to-end-of-story and selection, with include toggles
for locked layers, hidden layers, parent pages and notes. The captured constraint "object contains no
text for find/change" is a distinct typed error and MUST NOT be reported as "no matches".

---

## 14.6.4 Frames and Placed Graphics

**[STU-LAY-028]** Placed graphics live in graphic frames (`StudioLayer` of placed-asset kind). Picture
frames of arbitrary shape (rectangle, ellipse or any `StudioVectorPath`) MUST clip and fit placed
content. Place options MUST be format-aware and are enumerable:

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per placed format class. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Placed format class | Place options |
|---|---|
| Layered raster | Preserve layers, layer comps, transparency and channels; per-place layer-visibility selection; colour-mode support (RGB / CMYK / Lab / greyscale). The captured layer-comp option object is a distinct record and MUST be carried per placement. |
| Vector/PDF class | Page selection; crop-to (seven members, [STU-LAY-130]); transparent background; multi-page load onto the cursor. |
| Flat raster | Apply an embedded clipping path; alpha-channel choice; per-image colour profile and rendering intent. |
| Scalable vector | Placed as scalable vector geometry. |
| Nested layout document | Page selection and layer-visibility overrides, tracked as a link. |
| Movie / sound | Poster frame, controller skin, play-on-load, loop, navigation points. Authoring here; export via 14.11. |

Multi-file placement and gridified placement MUST load multiple assets onto the cursor and place them
in sequence or as a grid.

**[STU-LAY-130] Placed-PDF crop enumeration.** The captured crop enumeration has SEVEN members, two
more than the five commonly named:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Crops to |
|---|---|
| `pdf_crop.art` | The art box. |
| `pdf_crop.crop` | The crop box. |
| `pdf_crop.trim` | The trim box. |
| `pdf_crop.bleed` | The bleed box. |
| `pdf_crop.media` | The media box. |
| `pdf_crop.content_visible_layers` | The bounding box of content on visible layers only. |
| `pdf_crop.content_all_layers` | The bounding box of all content regardless of layer visibility. |

The captured layered-raster import constraint vocabulary additionally declares the supported bit
depths and colour spaces for layered raster placement -- 8-bit RGB, CMYK, Lab, greyscale, indexed and
1-bit bitmap -- and rejects anything else with a distinct typed error rather than a generic failure.
Studio MUST reproduce that as a typed unsupported-bit-depth error and a typed unsupported-colour-space
error.

**[STU-LAY-029]** Fitting MUST support fit-content-to-frame, fit-frame-to-content, fit-content-
proportionally, fill-frame-proportionally, centre-content and clear-fitting, plus a stored frame
content-fit rule and an anchor point for placed content. Fitting MUST be expressible as an
object-style property ([STU-LAY-142]).

**[STU-LAY-131] Fitting enumeration.** SUPERSEDES the six informal fitting operations of
v02.205 [STU-LAY-029]. The captured fit enumeration has SEVEN members and includes a
content-aware member the v02.205 list omitted:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Behaviour |
|---|---|
| `fit.content_to_frame` | Scale content to the frame, ignoring proportions. |
| `fit.center_content` | Centre without scaling. |
| `fit.proportionally` | Scale to fit inside the frame, preserving proportions. |
| `fit.content_aware` | Scale and position by analysing the image content. |
| `fit.frame_to_content` | Resize the frame to the content. |
| `fit.fill_proportionally` | Scale to fill the frame, preserving proportions and cropping. |
| `fit.apply_frame_fitting_options` | Re-apply the frame's stored fitting rule. |

A separate captured data-merge fitting enumeration has seven members with a partly different
vocabulary: `fitting.proportional`, `fitting.fit_content_to_frame`, `fitting.fit_frame_to_content`,
`fitting.preserve_sizes`, `fitting.content_aware`, `fitting.honour_existing_style`,
`fitting.fill_proportional`. Per [STU-LAY-103] both are recorded; Studio MUST expose ONE fitting
enumeration that is their union and MUST declare per consumer which members are legal, rather than
two near-duplicate enumerations. The captured constraint vocabulary states that content-aware fitting
is unavailable on one platform; Studio MUST NOT reproduce that platform limitation, and MUST report
unavailability through the same typed-error channel if a build genuinely lacks it.

**[STU-LAY-030]** Linked resource management MUST maintain, for every placed external asset, a link
record; the surface MUST support relink, relink to folder, relink across file extensions, update
link(s), edit-original and edit-with (round-trip to a source application and auto-update on save),
go-to-link, reveal-in-file-manager, embed/unembed and copy-links-to. The full record and command
contract is [STU-LAY-132].

**[STU-LAY-132] Linked-resource record contract.** SUPERSEDES the v02.205 [STU-LAY-030] link-record
field list, which named six fields. The captured link object declares thirty-eight; Studio MUST carry
at least these, which are the ones that change behaviour:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Members / notes |
|---|---|---|
| `name`, `file_path`, `date`, `size` | -- | |
| `status` | enumeration | `link.normal`, `link.out_of_date`, `link.missing`, `link.embedded`, and in a second capture also `link.inaccessible`. Per [STU-LAY-103] the normative set is FIVE; an importer meeting the four-member form maps unknown to `link.inaccessible`. |
| `link_resource_format` | string | The resource's format identifier. |
| `link_resource_size` | integer | |
| `link_resource_uri` | string | |
| `link_resource_modified` | boolean | The SOURCE changed. |
| `link_object_modified` | boolean | The PLACED INSTANCE changed. These are two different facts and MUST NOT be merged. |
| `stored_state` | enumeration | `stored.normal`, `stored.cached`, `stored.contained`, `stored.embedded` |
| `rendition_type` | enumeration | `rendition.fpo` (for-position-only proxy), `rendition.actual` |
| `import_policy` | enumeration | [STU-LAY-019] |
| `export_policy` | enumeration | [STU-LAY-019] |
| `can_embed`, `can_unembed`, `can_package` | booleans | Capability predicates a model MUST test before acting. |
| `edited` | boolean | |
| `version_state`, `editing_state` | enumerations | Check-in/check-out state for shared workflows. |
| `stock_state` | enumeration | `stock.not_stock`, `stock.stock_comp`, `stock.stock_high_resolution` |
| `parent`, `index` | -- | |

Commands: edit original, edit with a named application, relink, relink to folder, relink across file
extensions, update, unlink, embed, unembed, reveal in the system file manager, copy link, replace with
original, go to source, check in, reinitialise. Effective and actual resolution in `ppi`, scale, and
the layer used are read as derived values on the placed frame, not on the link record.

A collected-item placement surface (a content conveyor) MUST hold items and item sets for repeated
placement with place-once, place-all and keep modes and a create-link toggle. Missing and modified
links are preflight rules ([STU-LAY-154]).

**[STU-LAY-031]** Clipping and masking of placed content MUST support applying an embedded clipping
path, detecting edges, using an alpha channel, or a frame-as-mask, feeding both display clipping and
the text-wrap contour source ([STU-LAY-126]). The parameter contract is [STU-LAY-133].

**[STU-LAY-133] Clipping and masking contract.** SUPERSEDES the informal list of
v02.205 [STU-LAY-031] with the captured enumeration and bounds:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Bounds / members |
|---|---|---|
| `clipping_path_type` | enumeration | `clip.none`, `clip.detect_edges`, `clip.alpha_channel`, `clip.embedded_path`, `clip.user_modified` |
| `threshold` | integer | hard 0..255, unit `count`, precision 0 |
| `tolerance` | numeric | hard 0..10, unit `ratio`, precision unknown |
| `inset_frame` | numeric | hard -10000..10000 in `pt` (the capture states the equivalent bounds in six units; `pt` is canonical per [STU-LAY-100]) |
| `include_inside_edges` | boolean | -- |
| `restrict_to_frame` | boolean | -- |
| `use_high_resolution_image` | boolean | -- |

The clipping path feeds BOTH display clipping and the text-wrap contour source ([STU-LAY-126]); it is
one stored path with two consumers and MUST NOT be duplicated.

---

**[STU-LAY-134] Layout object-effect targeting and parameter contract.** [STU-LAY-032] required
opacity, blend mode and each effect to be independently targetable to Object, Fill, Stroke or Text of
a single frame, presented as an effects tree. The captured object-style effect category objects
confirm exactly FOUR targets -- object, fill, stroke and content -- each carrying its own fourteen-
field effect settings block, so the four-target model is storage, not presentation. Studio MUST carry
four independent effect stacks per frame.

The captured effect set and its declared bounds:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Effect | Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|---|
| Drop shadow | `opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Drop shadow | `noise` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Drop shadow | `spread` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Drop shadow | `size` | 0 | 144 | unknown | unknown | unknown | `pt` | unknown |
| Drop shadow | `y_offset` | -1000 | 1000 | unknown | unknown | unknown | `pt` | unknown |
| Drop shadow | `x_offset` | -1000 | 1000 | unknown | unknown | unknown | `pt` | unknown |
| Inner shadow | `choke_amount` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Inner shadow | `noise` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Inner shadow | `opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Inner shadow | `angle` | -360 | 360 | unknown | unknown | unknown | `deg` | unknown |
| Outer glow | `noise` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Outer glow | `opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Outer glow | `spread` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Inner glow | `noise` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Inner glow | `opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Inner glow | `spread` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Bevel and emboss | `altitude` | 0 | 90 | unknown | unknown | unknown | `deg` | unknown |
| Bevel and emboss | `angle` | -180 | 180 | unknown | unknown | unknown | `deg` | unknown |
| Bevel and emboss | `depth` | 0 | 1000 | unknown | unknown | unknown | `percent` | unknown |
| Bevel and emboss | `highlight_opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Bevel and emboss | `shadow_opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Satin | `angle` | -360 | 360 | unknown | unknown | unknown | `deg` | unknown |
| Satin | `opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Basic feather | `choke_amount` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Basic feather | `noise` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Basic feather | `width` | 0 | 1000 | unknown | unknown | unknown | `pt` | unknown |
| Directional feather | `angle` | -180 | 180 | unknown | unknown | unknown | `deg` | unknown |
| Directional feather | `choke_amount` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Directional feather | `noise` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Directional feather | `top_width` | 0.2 | 250 | unknown | unknown | unknown | `pt` | unknown |
| Directional feather | `bottom_width` | 0.2 | 250 | unknown | unknown | unknown | `pt` | unknown |
| Directional feather | `right_width` | 0.2 | 250 | unknown | unknown | unknown | `pt` | unknown |
| Directional feather | `left_width` | 0.2 | 250 | unknown | unknown | unknown | `pt` | unknown |
| Gradient feather | `opacity` (per stop) | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| Gradient feather | `midpoint` (per stop) | 13 | 87 | unknown | unknown | unknown | `percent` | unknown |

Note the directional feather's angle range is captured with its endpoints written in DESCENDING
order (180 to -180). Studio MUST normalise to `hard_min = -180`, `hard_max = 180` and MUST record
that the source ordering is a capture artefact, not a signed-range inversion.

Effect enumerations:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Enumeration | Members |
|---|---|
| `feather_corner_type` | `feather_corner.sharp`, `feather_corner.rounded`, `feather_corner.diffusion` |
| `feather_mode` | `feather_mode.none`, `feather_mode.standard` |
| `bevel_style` | `bevel.outer`, `bevel.inner`, `bevel.emboss`, `bevel.pillow_emboss` |
| `bevel_technique` | `bevel_tech.smooth_contour`, `bevel_tech.chisel_hard`, `bevel_tech.chisel_soft` |
| `bevel_direction` | `bevel_dir.up`, `bevel_dir.down` |
| `glow_technique` | `glow.softer`, `glow.precise` |
| `shadow_mode` | `shadow_mode.none`, `shadow_mode.drop` |

**[STU-LAY-135] Transparency and blending contract.** Document-scoped transparency preferences carry
a global light that every shadow and bevel effect inherits unless it overrides:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `global_light_angle` | -360 | 360 | unknown | unknown | unknown | `deg` | unknown |
| `global_light_altitude` | 0 | 90 | unknown | unknown | unknown | `deg` | unknown |
| `blending_setting_opacity` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |

`blending_space` is the captured three-member enumeration `{blend_space.default, blend_space.rgb,
blend_space.cmyk}` and is a SPREAD-scoped setting: changing it changes how every transparency group
on that spread composites. `spread_flattener_override` is a per-spread flattener level drawn from
`{flattener.low, flattener.medium_low, flattener.medium, flattener.medium_high, flattener.high}`.

**[STU-LAY-032]** Object effects and transparency apply through the shared `StudioEffectStack` (14.9)
with the four-target model and bounds of [STU-LAY-134]. Blend modes MUST draw from the canonical
Studio blend-mode set defined in [STU-VEC-151]. SUPERSEDED: the v02.205 [STU-LAY-032] claim that the
blend-mode set is "the standard sixteen" is wrong. The layout capture confirms a sixteen-member
enumeration and the design-suite capture declares nineteen; the canonical Studio set is nineteen,
and `blend.pass_through` -- which the "sixteen" formulation excluded while [STU-VEC-025]
simultaneously required it as the group default -- is a member. Isolate-blending and knockout-group
flags follow [STU-VEC-153]. Effect and blend maths are owned by 14.9 and 14.8; this clause fixes the
layout targeting model and the captured bounds.

**[STU-LAY-033]** Multi-state objects MUST be authorable in layout: convert a selection to a multi-state
object, add/reorder/delete states, add objects to the visible state, paste into a state, reset all
multi-state objects, and support hidden-until-triggered.

**[STU-LAY-136] Multi-state object contract.** The captured object carries `initially_hidden`
(boolean, governing visibility in the exported file) and the commands `release_as_objects` (releases
every state and destroys the parent) and `add_items_as_state`. Each state carries `name`, `active`
(is this the state shown), `enabled` (is this state usable in the exported document),
`state_type` (for a button, the user action that selects this appearance; for a multi-state object,
a numeric identity), and the commands `release_as_object` (releases ONE state's appearance as an
ordinary page item), `move` to a new ordinal, and `add_items_to_state`. Note that
`release_as_objects` (plural, on the parent) and `release_as_object` (singular, on a state) are two
different commands with different destructive scope and MUST NOT be merged.

The captured button state-type enumeration has nine members --
`state.up`, `state.rollover`, `state.down`, `state.up_on`, `state.rollover_on`, `state.down_on`,
`state.up_off`, `state.rollover_off`, `state.down_off` -- covering plain and two-state (checkbox and
radio) buttons. Runtime state switching and interactive triggers are owned by 14.11; this clause
owns the layout authoring of the structure.

**[STU-LAY-034]** QR codes MUST be generatable as editable vector objects, reusable object libraries and
snippets MUST store, search, sort and re-place objects with per-item metadata, and frame stroke
options apply through the shared vector stroke model (14.5, [STU-VEC-143] to [STU-VEC-146]).

**[STU-LAY-137] QR-code contract.** The captured commands declare a generic QR generator plus a
dedicated contact-card generator whose argument list is exactly: first name, last name, job title,
cell phone, phone, email, organisation, street address, city, state, country, postal code, website,
plus a swatch reference for the code colour. Studio MUST expose the payload kinds as a typed union --
web hyperlink, plain text, text message, email, contact card -- with the contact card carrying the
thirteen named fields above, and MUST allow post-generation editing of the produced vector geometry.
A generated code MUST be able to load onto the placement cursor rather than being placed
immediately. Per-record QR generation is a data-merge capability ([STU-LAY-163]) and a data-merge QR
placeholder is a distinct captured object kind alongside the text and image placeholders.

**[STU-LAY-138] Object library and snippet contract.** The captured export commands are
`export_page_items_selection_to_snippet` and `export_page_items_to_snippet` (taking explicit item
ids), plus a document preference `snippet_import_uses_original_location` (boolean) governing whether
a re-placed snippet returns to its authored coordinates or lands at the cursor. Studio MUST carry
that preference, because the two behaviours are both wanted and a model cannot infer which is
active. A library item carries a type and a description as searchable metadata.

---

## 14.6.5 Tables

**[STU-LAY-035]** A table is a structured object flowed inside a `StudioTextStory`; its cell content is
text, graphics or nested tables.

**[STU-LAY-139] Table structure contract.** The captured table object carries 181 properties and 21
methods; the captured cell object carries 116 properties and 21 methods; row and column carry 103 and
104. Studio MUST implement the structural surface below, which is the behaviour-bearing subset.

Creation and conversion: insert a table with body, header and footer row counts and a column
count, optionally applying a table style; a draw-table gesture; convert text to table and table
to text with selectable row and column separators; import a spreadsheet or word-processor table
per [STU-LAY-128].

Structure commands captured on the table, row, column and cell objects: insert and delete rows and
columns, select, merge, unmerge, split (with a captured "cells too small to split" typed error),
distribute evenly, paste before and after, drag-duplicate, sort, go to a row including header and
footer sections, convert body rows to and from header and footer rows, convert a cell between text
and graphic type, clear table-style overrides, clear cell-style overrides, create outlines, auto-tag,
and recompose.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Members / bounds |
|---|---|---|
| `body_row_count`, `column_count` | integers, `count` | |
| `row_type` | enumeration | `row.body`, `row.header`, `row.footer`, `row.mixed_state` |
| `column_type` | enumeration | `column.body`, `column.header` |
| `header_columns_position` | enumeration | `header_columns.left`, `header_columns.right` |
| `break_headers`, `break_footers` | enumeration | `header_break.in_all_text_columns`, `header_break.once_per_text_frame`, `header_break.once_per_page` |
| `skip_first_header`, `skip_last_footer` | booleans | |
| `stroke_order` | enumeration | `stroke_order.row_on_top`, `stroke_order.column_on_top`, `stroke_order.best_joins`, `stroke_order.legacy_compatibility` |
| `table_direction` | enumeration | `table_dir.left_to_right`, `table_dir.right_to_left` |
| `caption_position` | enumeration | `caption.before_table`, `caption.after_table` |
| `display_order` | enumeration | `display.by_rows`, `display.by_columns` |
| `alternating_fills_type` | enumeration | `alternating.none`, `alternating.rows`, `alternating.columns` |

**[STU-LAY-140] Table stroke, fill and alternating-pattern contract.** Every tint in the table model
carries a DECLARED hard range of 0..100 `percent`; there are more than thirty such fields and Studio
MUST enforce the bound on all of them uniformly rather than per field. The tinted fields are: the
four border strokes and their gap tints; the start-row, end-row, start-column and end-column
alternating stroke and fill tints and their gap tints; the four cell edge strokes and their gap
tints; the inner row and inner column strokes and their gap tints; the diagonal-line stroke tint; and
the default row and column stroke tints.

Alternating patterns carry, per axis: a start count, a start colour, a start weight, a start type, a
start tint, a start overprint, and the matching six end fields, plus `skip_first_alternating` and
`skip_last_alternating` counts. Studio MUST carry the start and end groups independently -- an
alternating pattern of "2 blue then 3 grey" is authorable and collapsing to a single colour with an
every-N count cannot express it.

**[STU-LAY-141] Cell contract.** Studio MUST carry these behaviour-bearing cell fields:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Members / bounds |
|---|---|---|
| `cell_type` | enumeration | `cell.text`, `cell.graphic` |
| `top_inset`, `left_inset`, `bottom_inset`, `right_inset` | numeric, `pt` | text-cell insets |
| `text_top_inset`, `text_bottom_inset`, `text_left_inset`, `text_right_inset` | numeric, `pt` | a SECOND inset group, distinct from the first; both are captured and both are required |
| `graphic_top_inset`, `graphic_bottom_inset`, `graphic_left_inset`, `graphic_right_inset` | numeric, `pt` | graphic-cell insets |
| `clip_content_to_cell`, `clip_content_to_text_cell`, `clip_content_to_graphic_cell` | booleans | three independent clips, one general and two per cell type |
| `vertical_justification` | enumeration | [STU-LAY-121] |
| `first_baseline_offset`, `minimum_first_baseline_offset` | enumeration + numeric | [STU-LAY-120] |
| `rotation_angle` | numeric, `deg` | the captured value domain is the four quarter turns for cell text |
| `text_cell_rotation_follows_story_direction` | boolean | |
| `row_span`, `column_span` | integers, `count` | |
| `minimum_height`, `maximum_height` | numeric, `pt` | the row-height at-least/exactly behaviour is expressed as this pair |
| `keep_with_next_row` | boolean | |
| `start_row` | enumeration | start this row on the next frame, page, odd page or even page |
| `auto_grow` | boolean | |
| `paragraph_spacing_limit` | numeric, `pt` | |
| `diagonal_line_in_front` | boolean | draw the diagonal above or below the cell content |
| `top_left_diagonal_line`, `top_right_diagonal_line` | booleans | the two diagonals are independent; both true gives the crossed form |
| `fill_colour`, `fill_tint`, `overprint_fill` | -- | plus `gradient_fill_length`, `gradient_fill_angle` (hard -180..180 `deg`), `gradient_fill_start` |
| `applied_cell_style_priority` | integer | the precedence used to resolve conflicting styles at a shared edge |
| `top_edge_stroke_priority` and the three siblings | integers | edge-conflict resolution between adjacent cells |

The four edge-stroke PRIORITY fields are the mechanism by which two adjacent cells with different
edge strokes resolve which one draws. They are captured as first-class integers and Studio MUST carry
them; a renderer that picks by z-order or by iteration order will disagree with the source on every
mixed-stroke table.

The captured constraint vocabulary declares three table-specific typed errors Studio MUST reproduce:
invalid cell range; cells too small to split; and a text frame cannot be placed inside a table cell
(a cell holds text or a graphic, not a nested frame). Table style header-row and footer-row counts
carry their own typed validation errors.

Tables are formatted through table and cell styles ([STU-LAY-142]); the field-level model is owned by
14.23.

---

## 14.6.6 Layout Style System

**[STU-LAY-036]** Every named format is a record in the shared `StudioStyleRegistry`.

**[STU-LAY-142] Layout style-type contract.** The captured style objects and their property counts
establish the real weight of each style type; Studio MUST carry the full attribute payload of each,
not a subset.

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per layout style type. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Studio style type | Captured property count | Scope |
|---|---|---|
| `style.paragraph` | 334 | Full paragraph-level formatting; layout-flow attributes per [STU-LAY-125]; the character-attribute payload is owned by 14.7. |
| `style.character` | 161 | Partial-attribute run-level formatting; applies only explicitly set attributes over the paragraph style. |
| `style.object` | 62 | Frame/object formatting with per-category include toggles. |
| `style.table` | -- | Table-level formatting referencing up to five cell styles (header, footer, body, left column, right column) plus border and alternating patterns. |
| `style.cell` | 75 | Cell-level formatting: insets, strokes, fills, diagonal lines, an optional paragraph style. |
| `style.toc` | 11 | Stored table-of-contents definitions ([STU-LAY-148]). |

The captured OBJECT style carries SEVENTEEN independent category-enable booleans, and Studio MUST
carry every one, because an object style that cannot be scoped is not a style: `enable_fill`,
`enable_stroke`, `enable_stroke_and_corner_options`, `enable_paragraph_style`,
`enable_text_frame_general_options`, `enable_text_frame_baseline_options`,
`enable_text_frame_auto_sizing_options`, `enable_text_frame_column_rule_options`,
`enable_text_frame_footnote_options`, `enable_story_options`, `enable_text_wrap_and_others`,
`enable_anchored_object_options`, `enable_frame_fitting_options`, `enable_export_tagging`,
`enable_object_export_alt_text_options`, `enable_object_export_tagged_pdf_options`,
`enable_object_export_epub_options`, `enable_transform_attributes`, `enable_flex_layout_attributes`.

An object style additionally carries the frame's stroke surface directly -- `miter_limit` (hard
1..500), `end_cap`, `end_join`, `stroke_type`, `left_line_end`, `right_line_end`, `gap_colour`,
`gap_tint`, `overprint_gap`, `stroke_alignment`, `left_arrow_head_scale` and
`right_arrow_head_scale` (each hard 1..1000 `percent`), `arrow_head_alignment` -- plus the four
independent corner radii and corner options, an applied paragraph style, an apply-next-paragraph-
style reference, an applied named grid, a keyboard shortcut, an extended keyboard shortcut, a
non-printing flag, an emit-CSS flag, and the three accessibility export fields
(`epub_aria_role`, `epub_aria_label`, `epub_aria_label_source_type` with members
`aria_label.automatic`, `aria_label.custom`, `aria_label.none`).

Style GROUPS are a separate captured object per style type (paragraph, character, object, cell,
table) and participate in load and import. Studio MUST implement one group primitive shared across
style types rather than five ([STU-SECTION-003]).

**[STU-LAY-037]** Style application mechanics MUST be preserved across all style types: based-on
inheritance where a child stores only deltas; next-style chaining for paragraph styles; override
handling with an override indicator, an override highlighter, clear-overrides with character-versus-
paragraph scoping and a clear-on-apply toggle; redefine-style from the current selection; break-link
to freeze current formatting as local values; and style groups.

**[STU-LAY-143] Style constraint vocabulary.** The captured error tables declare the style-system
rules Studio MUST enforce as distinct typed errors, and this is the closest thing the source has to a
written style specification: a style name MUST NOT be empty; a style group name MUST NOT be empty; a
style name MUST be unique within its scope; a style group name MUST be unique; certain names are
RESERVED and MUST be refused; a style name MUST NOT exceed a maximum length; a style name MUST NOT
contain a bracket character; a based-on chain MUST NOT be circular; a group MUST NOT be copied into
itself; a parent group MUST NOT be copied into its own child; a root style MUST NOT be modified in
ways reserved to derived styles; a style pack name MUST begin with a declared prefix; a style inside
a style pack MUST NOT be renamed; a style-mapping tag MUST be valid; a text-variable name MUST NOT
exceed a maximum length; and a style operation on a locked story MUST fail closed.

Each of these is a SEPARATE typed error in the capture and MUST be a separate typed error in Studio:
a model that receives one generic "invalid style" cannot repair its input, which is the entire
purpose of the vocabulary.

Optional bundled style sets (coordinated style packs) and role-detecting automatic style application
MAY be provided over this mechanism; the captured style-pack prefix rule above governs their naming.

**[STU-LAY-038]** Pattern (regular-expression) styles MUST apply a character style automatically to every
regular-expression match inside paragraphs carrying a paragraph style. Nested styles MUST apply
character styles through or up to N occurrences of a delimiter (character, word, tab or an
end-nested-style marker) inside a paragraph, plus per-line nested line styles. Both are
paragraph-style-embedded rules resolved at composition time; the captured paragraph style carries an
`all_nested_styles` collection as a first-class property, so the rules are stored ON the style and
MUST NOT be modelled as a separate side table.

**[STU-LAY-039]** Object styles are the layout counterpart of text styles and MUST carry the full
frame-formatting surface of [STU-LAY-142]; applying an object style sets fitting ([STU-LAY-131]),
text wrap ([STU-LAY-126]), anchoring ([STU-LAY-127]), effects targeting ([STU-LAY-134]) and export
tagging ([STU-LAY-146]) in one operation. Table and cell styles MUST format tables and cells
declaratively; a table style references its component cell styles and applying it cascades through
the table.

**[STU-LAY-040]** Cross-document style transfer MUST support loading selected styles from another
document with per-style conflict resolution and MUST support incoming word-processor style mapping on
import with saved presets.

**[STU-LAY-145] Style import contract.** The captured import command takes an import FORMAT and a
GLOBAL CLASH RESOLUTION STRATEGY as explicit arguments; Studio MUST expose the strategy rather than
prompting, so a model can import deterministically. Per-style conflict resolution (use-incoming
versus auto-rename) MUST also be available. Cell-style, paragraph-style and character-style MAPPING
objects are captured as distinct record types for shared-content workflows, and Studio MUST carry a
mapping record type so an import decision is stored and repeatable rather than made afresh each time.

**[STU-LAY-146] Export tag mapping.** Each text and object style MUST declare an export tag and class
for reflowable output, plus a tagged-PDF role, editable singly or in bulk. The captured object-style
export-tag map is a first-class record type keyed by object export TYPE, so a single style carries
DIFFERENT tags for different export targets; Studio MUST carry the map, not a single tag string. This
drives accessible and tagged output ([STU-LAY-158]) and reflowable export via 14.11. The captured
accessibility fields on the style are `epub_aria_role`, `epub_aria_label` and
`epub_aria_label_source_type`; the captured typed errors are invalid EPUB type, invalid ARIA role and
invalid ARIA label.

**[STU-LAY-041]** Export tag mapping is stored on the style record in `StudioStyleRegistry`
per [STU-LAY-146].

---

## 14.6.7 Long-Document System

**[STU-LAY-042]** Books MUST bind multiple chapter `StudioDocument`s into one publication unit that
shares numbering and output. The book surface MUST support add/remove/reorder documents, designate a
style-source document, show per-document status, and open documents from the book list.

**[STU-LAY-147] Book contract and synchronisation category set.** The captured book object declares
the synchronisation categories as SEVENTEEN INDEPENDENT BOOLEANS, not as a category list; Studio MUST
carry all seventeen so a partial synchronisation is expressible:

`sync_paragraph_styles`, `sync_character_styles`, `sync_object_styles`, `sync_table_styles`,
`sync_cell_styles`, `sync_toc_styles`, `sync_swatches`, `sync_text_variables`,
`sync_bullet_numbering_lists`, `sync_cross_reference_formats`, `sync_conditional_text`,
`sync_parent_pages`, `sync_trap_styles`, `sync_mojikumi_styles`, `sync_kinsoku_styles`,
`sync_composite_fonts`, `sync_named_grids`.

Plus `smart_match_style_groups`, a two-member enumeration
`{smart_match.by_style_path, smart_match.by_style_name}` governing how a style inside a group is
matched across documents. This is the mechanism behind "smart style-group matching" and MUST be
exposed, not hidden.

Book-level fields and commands:

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field / command | Notes |
|---|---|
| `style_source_document` | The document whose styles win. |
| `repagination_option` | Enumeration governing continue / continue-on-next-odd / continue-on-next-even. |
| `insert_blank_page` | Boolean; inserts a blank page to satisfy the odd/even rule. |
| `automatic_pagination` | Boolean; repaginate on add, delete or reorder. |
| `automatic_document_conversion` | Boolean; convert older chapter documents during repagination and synchronisation. |
| `merge_identical_layers` | Boolean; merge identically named layers on PDF export. |
| `synchronize` | Command; matches every document to the style source. |
| `repaginate` | Command. |
| `update_chapter_and_paragraph_numbers` | Command. |
| `update_all_numbers` | Command; page, chapter and paragraph numbers together. |
| `update_all_cross_references` | Command; book-wide. |
| `export` | Command taking a format, a PDF preset and an explicit subset of book documents. |
| `preflight` | Command taking a report path and an auto-open flag. |
| `package` | Command; the same nine-argument form as the document-level package ([STU-LAY-155]). |
| `print` | Command. |

Each chapter carries `status` (a book-content status enumeration), `document_page_range` (a string),
`date`, `size` and `file_path`, and the commands replace, move, synchronise and preflight. The
captured constraint vocabulary declares two book typed errors Studio MUST reproduce: the book does
not contain one or more documents named for export, and the requested export format is unsupported
for a book.

**[STU-LAY-043]** Book synchronisation MUST propagate the selected categories of [STU-LAY-147] from the
style-source document. Book numbering MUST continue page and chapter numbering across documents, be
updatable on demand, and support disabling automatic numbering. Book-wide output MUST print, export,
preflight and package the whole book or a selected subset.

**[STU-LAY-144] Bullets and numbering contract.** Numbered lists MUST be continuable across stories
and across book documents for figure and table numbering. The captured list-style object is a
first-class named record in the style registry, and the captured bullet-character type enumeration
is `{bullet_char.unicode_only, bullet_char.unicode_with_font, bullet_char.glyph_with_font}` --
three distinct storage forms, because a bullet may be a code point, a code point bound to a font,
or a glyph id bound to a font, and only the third survives a font whose glyph is unencoded. The
captured constraint vocabulary declares that a bullet character and a bullet type MUST both be
specified or both be omitted; supplying one alone is a typed error. Numbering styles for lists draw
from the sixteen-member list enumeration of [STU-LAY-108]. A bullet or number list export option is
captured as `{list_export.unordered_list, list_export.as_text}` and belongs to the export tag map
of [STU-LAY-146].

**[STU-LAY-044]** Tables of contents MUST be style-driven.

**[STU-LAY-148] Table-of-contents contract.** The captured TOC style object and its entry object
declare the complete surface:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| TOC style field | Kind | Notes |
|---|---|---|
| `name` | string | |
| `title` | string | The generated heading text. |
| `title_style` | paragraph style reference | |
| `include_book_documents` | boolean | Valid when the document is part of a book. |
| `include_hidden` | boolean | Include entries from text on hidden layers. |
| `run_in` | boolean | Place the lowest-level entries on the same line as the previous entry. |
| `create_bookmarks` | boolean | Emit bookmarks for entries. |
| `make_anchor` | boolean | Create a text anchor in the SOURCE paragraph, which is what makes a TOC entry a live hyperlink target. |
| `remove_forced_line_break` | boolean | Strip forced breaks from captured entry text. |
| `numbered_paragraphs` | enumeration | How a numbered source paragraph's number is carried into the entry. |
| `set_story_direction` | enumeration | `{direction.horizontal, direction.vertical}` |

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| TOC entry field | Kind | Notes |
|---|---|---|
| `name` | string | The source paragraph style this entry maps. |
| `format_style` | paragraph style reference | The style applied to the generated entry. |
| `level` | integer, `count` | Indent level. |
| `page_number_style` | character style reference | |
| `page_number_position` | enumeration | before / after / none |
| `separator` | string | Inserted between entry text and page number. |
| `separator_style` | character style reference | |
| `sort_alphabet` | boolean | Sort this level alphabetically. |

The generate command takes a TOC style, a replace-existing flag, an optional book, a place point and
an INCLUDE-OVERSET flag; Studio MUST expose the overset flag, because a TOC generated from a document
with overset text otherwise silently omits entries. Multiple TOCs per document MUST be supported, each
with independent settings, and an update-all command MUST refresh every one.

The captured constraint vocabulary declares three TOC typed errors Studio MUST reproduce: a TOC style
MUST carry at least one entry (a default-styled TOC is the only exception); the same paragraph style
MUST NOT be mapped twice within one TOC style; and text anchors are not created for locked stories,
which silently degrades `make_anchor` and MUST be reported rather than ignored.

**[STU-LAY-045]** Indexing MUST build a topic/subtopic hierarchy through inserted index marks, with
sort-by overrides, per-reference page-range scoping and cross-reference forms.

**[STU-LAY-149] Index contract.** The captured index-options object declares exactly FOUR topic
levels (level 1 through level 4 paragraph styles), which fixes the v02.205 "at least four levels" as
exactly four in the captured model; Studio MAY exceed four but MUST support four and MUST declare its
own maximum.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Kind | Notes |
|---|---|---|
| `title`, `title_style` | string, paragraph style | |
| `replace_existing_index` | boolean | Replaces CONTENT only; does not move the index or change its other properties. |
| `include_book_documents` | boolean | |
| `include_hidden_entries` | boolean | |
| `index_format` | enumeration | `{index_format.run_in, index_format.nested}` -- governs level 2 and lower. |
| `include_section_headings` | boolean | Alphabet letters as section headings. |
| `include_empty_index_sections` | boolean | Valid only when section headings are on. |
| `level_1_style` .. `level_4_style` | paragraph styles | |
| `section_heading_style` | paragraph style | |
| `page_number_style`, `cross_reference_style`, `cross_reference_topic_style` | character styles | |
| `following_topic_separator` | string | After each topic. |
| `between_entries_separator` | string | Between entries in run-in format. |
| `page_range_separator` | string | Between the two numbers of a range. |
| `between_page_numbers_separator` | string | Between separate numbers and ranges. |
| `before_cross_reference_separator` | string | Before a cross reference. |
| `entry_end_separator` | string | At the end of each entry. |

All six separators are INDEPENDENT strings and Studio MUST carry six; collapsing them produces
indexes that cannot match a house style. An invalid separator set is a captured typed error.

The captured cross-reference-type enumeration for index entries has EIGHT members:
`xref.see_or_also_bracket`, `xref.see`, `xref.see_also`, `xref.see_herein`, `xref.see_also_herein`,
`xref.custom`, `xref.custom_before`, `xref.custom_after`. The captured index capitalisation
enumeration has four: `index_caps.selected_entry`, `index_caps.include_subentries`,
`index_caps.all_level_1`, `index_caps.all_entries`, with a captured constraint that the first two
MUST NOT be used when capitalising a whole index.

Section header sets are a distinct captured object carrying a header-set name, a language id and a
group list, and the captured header-type enumeration has TWENTY-SEVEN members covering Latin,
Nordic, Central and Eastern European, Cyrillic, Turkish, Korean (consonant and consonant-plus-vowel),
Japanese (hiragana and katakana, all and consonants-only) and Chinese (pinyin and stroke count)
alphabets. Studio MUST carry a pluggable header-set model of at least that generality; hard-coding a
Latin alphabet is non-conformant. Captured index typed errors: invalid page entry, invalid topic
entry, invalid referenced topic entry, index could not be updated because a story is locked, and a
document missing from the book.

**[STU-LAY-046]** Notes MUST include footnotes, endnotes and sidenotes.

**[STU-LAY-150] Notes contract.** The captured footnote-option object carries 42 properties and the
endnote-option object 15. Every numeric bound below is declared in the capture.

Footnote numbering and marker:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `footnote_start_at` | 1 | 400 | unknown | unknown | unknown | `count` | 0 |
| `footnote_separator_text` | 0 | 100 | unknown | unknown | unknown | `characters` | -- |
| `footnote_prefix` | 0 | 100 | unknown | unknown | unknown | `characters` | -- |
| `footnote_suffix` | 0 | 100 | unknown | unknown | unknown | `characters` | -- |

`footnote_numbering_style` draws from the captured FIFTEEN-member footnote numbering
enumeration: `upper_roman`, `lower_roman`, `upper_letters`, `lower_letters`, `arabic`,
`symbols`, `kanji`, `full_width_arabic`, `single_leading_zeros`, `double_leading_zeros`,
`asterisks`, `arabic_alif_ba_tah`, `arabic_abjad`, `hebrew_biblical`, `hebrew_non_standard`.
Note that `symbols` and `asterisks` are members here and are NOT members of the page-number
enumeration of [STU-LAY-108]; the two enumerations are related but not identical and MUST NOT be
merged.

`footnote_restart_numbering` is `{footnote_restart.dont_restart, footnote_restart.page,
footnote_restart.spread, footnote_restart.section}`. `footnote_marker_positioning` is
`{marker.normal, marker.superscript, marker.subscript, marker.ruby}`. `footnote_prefix_suffix`
position is `{prefix_suffix.none, prefix_suffix.reference, prefix_suffix.marker,
prefix_suffix.both}`. `footnote_first_baseline_offset` uses [STU-LAY-120].

Footnote layout and rules -- there are TWO independent rule groups, one for the first footnote in a
column and one for footnote text continued from a previous column, each with thirteen fields:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `rule_line_weight` | 0 | 1000 | unknown | unknown | unknown | `pt` | unknown |
| `rule_tint` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `rule_gap_tint` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `continuing_rule_line_weight` | 0 | 1000 | unknown | unknown | unknown | `pt` | unknown |
| `continuing_rule_tint` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `continuing_rule_gap_tint` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |

plus, per group, `rule_on` (boolean gate), `rule_type`, `rule_colour`, `rule_gap_colour`,
`rule_overprint`, `rule_gap_overprint`, `rule_left_indent`, `rule_width`, `rule_offset`. Plus
`space_between` (vertical space between footnotes; the capture states the paragraph style's own
space-before and space-after are IGNORED for footnotes, which Studio MUST reproduce), `spacer`
(minimum space between the bottom of the text column and the first footnote, with the same
space-before suppression), `eos_placement` (boolean: footnotes at the end of a story sit just below
the text rather than at the column bottom), `no_splitting` (boolean: footnotes may not split across
columns), and `enable_straddling` (boolean: straddling footnotes across columns).

Endnotes:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Bounds / members |
|---|---|---|
| `endnote_title` | string | 0..100 characters |
| `endnote_separator_text` | string | 0..100 characters |
| `endnote_prefix`, `endnote_suffix` | strings | 0..100 characters each |
| `endnote_title_style`, `endnote_text_style` | paragraph styles | |
| `endnote_marker_style` | character style | |
| `start_endnote_number_at` | integer, `count` | |
| `restart_endnote_numbering` | enumeration | `{endnote_restart.continuous, endnote_restart.story_restart}` |
| `scope_value` | enumeration | `{endnote_scope.story, endnote_scope.document}` |
| `frame_create_option` | enumeration | `{endnote_frame.load_place_gun, endnote_frame.new_page}` |
| `endnote_marker_positioning`, `show_endnote_prefix_suffix` | enumerations | as footnotes |

The captured constraint vocabulary declares an endnote-on-parent-page alert, which Studio MUST
reproduce as a typed warning rather than allow silently.

Sidenotes MUST anchor to the last text line or a frame edge, inside or outside the margin, with
per-scope restart. NO captured parameter contract for sidenotes was recovered; sidenote options are a
declared SPEC GAP.

An EPUB footnote-placement enumeration exists for reflowable output --
`{epub_footnote.after_story, epub_footnote.after_paragraph, epub_footnote.inside_popup}` -- and
belongs to the export handoff of [STU-LAY-165].

**[STU-LAY-047]** Cross-references MUST insert references to paragraphs or named text anchors using
editable formats assembled from building blocks, with a character style, and MUST flag stale or
broken references in preflight. Named text anchors MUST provide hyperlink and cross-reference
destinations at exact text positions across documents.

**[STU-LAY-151] Cross-reference, hyperlink and text-variable contract.**

The captured hyperlink model is SEVEN distinct object types, and Studio MUST carry all seven because
each has different resolution behaviour: a hyperlink (the link itself), a URL destination, a page
destination, an external-page destination (a page in another document), a text destination, a text
source, and a page-item source. Cross-reference sources and formats are two further types. A
hyperlink carries an appearance group: `{highlight.none, highlight.invert, highlight.outline,
highlight.inset}`, `{border_style.solid, border_style.dashed}`, `{border_width.thin,
border_width.medium, border_width.thick}`. A page destination carries a
`hyperlink_destination_page_setting` enumeration with seven members --
`{page_setting.fixed, page_setting.fit_view, page_setting.fit_window, page_setting.fit_width,
page_setting.fit_height, page_setting.fit_visible, page_setting.inherit_zoom}` -- and a
`view_percentage` with hard range 5..4000 `percent`.

Captured hyperlink and cross-reference typed errors Studio MUST reproduce: a name already in use; a
text source cannot be an insertion point (it needs a range); a destination name cannot be changed
when manual names are disallowed; a bookmark must have a name; a bookmark list is empty; a bookmark
could not be found; the destination document was not found; a cross-reference destination must be a
text anchor or a paragraph; a named character style does not exist in this document; a delimiter must
be a single character; a delimiter-inclusion attribute must be exactly `true` or `false`; a
cross-reference format string has invalid syntax; and a hyperlink cannot be created when the
selection includes only one half of an XML tag pair.

**[STU-LAY-048]** Text variables MUST be resolvable placeholders whose single edit updates every
instance. The captured variable-type enumeration has TWELVE members and Studio MUST carry all twelve:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Studio member | Resolves to |
|---|---|
| `variable.custom_text` | A reusable text placeholder. |
| `variable.file_name` | The document file name, with include-path and include-extension options. |
| `variable.last_page_number` | Section or document last page number, for page-x-of-y. |
| `variable.chapter_number` | The document chapter number, with before/after text and a numbering style. |
| `variable.output_date` | The date and time of the current print, export or package operation. |
| `variable.creation_date` | The date and time the document was first saved. |
| `variable.modification_date` | The date and time last saved to disk. |
| `variable.match_character_style` | Running header derived from the first or last on-page text carrying a character style. |
| `variable.match_paragraph_style` | Running header derived from the first or last on-page text carrying a paragraph style. |
| `variable.xref_page_number` | The page number of a cross-reference target. |
| `variable.xref_chapter_number` | The chapter number of a cross-reference target. |
| `variable.live_caption` | Metadata pulled from a nearby placed image, driving live captions. |

The v02.205 [STU-LAY-048] table listed ten variables and merged the two running-header variants into
one; that merge is superseded. A page-number variable additionally carries its own preference
object, and a variable's numbering style draws from the twelve-member variable numbering enumeration
of [STU-LAY-108] including `number_style.current`. A `convert_to_text` command MUST be available to
flatten every instance of one variable.

**[STU-LAY-049]** Conditional text MUST let named conditions hide or show tagged text ranges, and
condition sets MUST capture reusable visibility combinations.

**[STU-LAY-153] Conditional-text contract.** A condition carries `name`, `visible` (boolean),
`indicator_colour`, `indicator_method` (`{indicator.underline, indicator.highlight}`) and
`underline_indicator_appearance` (`{underline.wavy, underline.solid, underline.dashed}`). A
condition set carries a name and a list of condition/visibility pairs, plus a `redefine` command that
captures the current visibility of every condition. A document-scoped conditional-text preference
carries the indicator MODE (`{indicator_mode.show, indicator_mode.show_and_print,
indicator_mode.hide}`). Deleting a condition takes a REPLACING-WITH argument so tagged text is
re-tagged rather than orphaned; Studio MUST expose that argument. Conditional visibility resolves
BEFORE composition and therefore interacts with smart text reflow ([STU-LAY-123]).

---

## 14.6.8 Grids, Guides, and Measurement

**[STU-LAY-050]** Grid and guide constructs are `StudioLayoutGrid` records.

**[STU-LAY-115] Document and baseline grid contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `baseline_start` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `baseline_division` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `baseline_view_threshold` | 5 | 4000 | unknown | unknown | unknown | `percent` | unknown |
| `horizontal_gridline_division` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `vertical_gridline_division` | unknown | unknown | unknown | unknown | unknown | `pt` | unknown |
| `horizontal_grid_subdivision` | unknown | unknown | unknown | unknown | unknown | `count` | 0 |
| `vertical_grid_subdivision` | unknown | unknown | unknown | unknown | unknown | `count` | 0 |

plus `baseline_grid_shown`, `document_grid_shown`, `document_grid_snap_to`, `grids_in_back`
(booleans), `grid_colour` and `baseline_colour`, and `baseline_grid_relative_option` (the zero point
for the baseline offset). A FRAME may override the document baseline grid: the captured baseline
frame-grid option object carries `use_custom_baseline_frame_grid` (boolean),
`starting_offset_for_baseline_frame_grid`, `baseline_frame_grid_relative_option` (top of page, top
margin, top of frame, or frame inset), `baseline_frame_grid_increment` and
`baseline_frame_grid_colour`. Studio MUST carry both scopes.

**[STU-LAY-114] Ruler-guide contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `guide_view_threshold` (per guide) | 5.0 | 4000.0 | unknown | unknown | unknown | `percent` | unknown |
| `ruler_guides_view_threshold` (document default) | 5 | 4000 | unknown | unknown | unknown | `percent` | unknown |
| `guide_snap_to_zone` | 1 | 36 | unknown | unknown | unknown | `px` | 0 |

plus per guide: `orientation`, `location`, `guide_colour`, `guide_type` ([STU-LAY-111]),
`guide_zone`, `fit_to_page` (boolean), `page_index`, `locked`, and the parent-item override fields
(`overridden`, `allow_overrides`, `overridden_page_item_props`, `item_layer`) so a guide authored on
a parent page behaves like any other parent item. Document-scoped: `guides_shown`, `guides_locked`,
`guides_snap_to`, `guides_in_back`, `ruler_guides_colour`. Guides support copy and paste across
pages, select-all and delete-all-on-spread.

**[STU-LAY-051]** Smart guides MUST give dynamic alignment feedback against object edges and centres,
plus smart-dimension and smart-spacing hints while dragging. Snapping MUST be toggleable per
construct (grid, guides, objects).

**[STU-LAY-116] Snapping and view-precision contract.** The captured view preferences declare:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `cursor_key_increment` | 0.001 | 100 | unknown | unknown | unknown | `pt` | 3 |
| `points_per_inch` | 60 | 80 | unknown | unknown | unknown | `count` | unknown |
| `horizontal_custom_points` | 4 | 256 | unknown | unknown | unknown | `count` | 0 |
| `vertical_custom_points` | 4 | 256 | unknown | unknown | unknown | `count` | 0 |
| `zoom_percentage` | 5 | 4000 | unknown | unknown | unknown | `percent` | unknown |
| `real_precision` | 1 | 7 | unknown | unknown | unknown | `count` | 0 |

`real_precision` is the number of decimal places the layout engine carries on a real value; it is a
DOCUMENT-scoped precision governing every `precision` field in 14.6 that is not otherwise declared,
and Studio MUST expose it because two documents at different real precisions produce different
geometry from identical commands. `points_per_inch` at 60..80 is the custom-unit definition and is
what makes `measure.custom` of [STU-LAY-101] meaningful.

**[STU-LAY-052]** Layout grids and named grids (character-count-based page grids and reusable frame-grid
formats importable across documents) MUST be supported for grid-based composition. Named grids apply
to text frames ([STU-LAY-118]).

**[STU-LAY-117] Named-grid contract.** The captured grid-data-information object carries the grid
defaults shared by named, layout and frame grids, and the captured document preference carries the
four text-area fields that define a character grid: `text_area_width`, `text_area_depth`,
`text_area_depth_unit` (an enumeration), and a characters-per-line count. Also captured:
`character_aki` and `line_aki` (the space inserted before or after characters and lines), and a
`minimum_scale` with hard range 5..4000 `percent`. A page carries `grid_starting_point`
([STU-LAY-107]) and `use_parent_grid`. Grid printing and export is a separate captured preference
object. Studio MUST carry the named grid as a first-class registry record, applicable to a text frame
through the object style's `applied_named_grid` field ([STU-LAY-142]).

**[STU-LAY-053]** Measurement systems MUST be selectable per axis; the normative enumeration and the
accepted input-suffix set are [STU-LAY-101], which SUPERSEDES the ten-item list previously stated
here. Every length-bearing layout field MUST carry an explicit unit per [STU-DOC-003]; the document
declares a default layout unit and mixed-unit fields are forbidden.

---

## 14.6.9 Prepress and Output

**[STU-LAY-054]** All render-to-output operations are `StudioExportRecipe` executions dispatched through
the quiet/headless output path. Output MUST run without stealing focus or popping foreground windows
and MUST be observable as a background task with progress and cancel, per the headless/quiet law
(14.20). The captured object model confirms a first-class ASYNCHRONOUS export command distinct from
the synchronous one, and a captured typed error stating that a particular format cannot be exported
asynchronously; Studio MUST expose both paths and MUST report the per-format capability rather than
silently blocking.

**[STU-LAY-169] Export-format enumeration.** Two captures declare the export-format enumeration with
eighteen and fifteen members; the eighteen-member form is the superset and is normative:

`export.tagged_text`, `export.pdf`, `export.interactive_pdf`, `export.eps`, `export.rtf`,
`export.text`, `export.xml`, `export.jpg`, `export.png`, `export.html`, `export.html5`,
`export.html_fixed_layout`, `export.epub`, `export.epub_fixed_layout`, `export.layout_markup`,
`export.layout_markup_idmx`, `export.snippet`, `export.copy_markup`.

The three-member difference between the two captures is `export.eps`, `export.xml` and `export.jpg`,
which one declaration omits; Studio MUST support all eighteen and MUST report an unsupported
combination through the captured typed error "the specified object does not support the desired
export format" rather than failing generically. The typed-error contract is [STU-LAY-166].

### 14.6.9.1 Preflight

**[STU-LAY-055]** Studio MUST run a live preflight engine that continuously validates the active document
against a selected profile, reporting an error count and per-error fix information, limitable to a
page range, with a status indicator. Preflight MUST also be runnable on export.

**[STU-LAY-154] Preflight contract.** The captured objects are: a preflight profile (name,
description, plus add/duplicate/delete/unembed/update/save commands), a preflight profile RULE, a
preflight RULE (name, description, a stable rule id string, and a `full_feature` boolean recording
whether the rule is fully supported), a preflight rule INSTANCE, a preflight PROCESS (an active run),
a preflight OPTION object and a preflight BOOK OPTION object. Studio MUST carry all seven roles: a
profile is a named set of rule instances; a rule is a registered check; an instance is a rule plus its
parameters and severity within a profile; and a process is one execution.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Members |
|---|---|---|
| `preflight_rule_flag` (severity) | enumeration | `severity.disabled`, `severity.error`, `severity.warning`, `severity.informational` |
| `preflight_scope` | enumeration or string | `scope.all_pages`, `scope.selected_documents`, `scope.all_documents`, or a page-range string using `-` for ranges and `,` for separate pages |
| `preflight_which_layers` | enumeration | `layers.all`, `layers.visible`, `layers.visible_printable` |
| `preflight_profile_policy` | enumeration | `profile_policy.use_embedded`, `profile_policy.use_working` |
| `preflight_include_objects_on_pasteboard` | boolean | |
| `preflight_include_nonprinting_objects` | boolean | |
| `preflight_off` | boolean | Disable live preflight for this document or all documents. |
| `preflight_embed_working_profile` | boolean | Embed the working profile in new documents. |
| `preflight_working_profile` | reference | |

A rule instance's parameters are typed: the captured rule-data-type enumeration is
`{rule_data.integer, rule_data.short_integer, rule_data.real, rule_data.string, rule_data.boolean,
rule_data.object, rule_data.list}`. Studio MUST use that type vocabulary for rule parameters so a
model can author a rule instance without knowing the rule.

Captured typed errors: preflight profile name already exists; preflight profile name contains illegal
characters.

**[STU-LAY-056]** Preflight profiles are editable rule sets carrying severity thresholds, creatable,
exportable and importable as portable profile files, and embeddable in a document so recipients
preflight against the same rules. The captured profile object's `save` and `unembed` commands are the
mechanism and MUST be exposed.

**[STU-LAY-057]** The preflight rule categories are enumerable and normative:

*Derivation: catalogue table, splits per row; yields 7 microtasks, one per preflight rule category.*

| Category | Representative rules |
|---|---|
| Links / resources | Missing links, modified links, low placed-image effective resolution, outdated linked resources, passthrough compatibility. |
| Colour | Blend space, allowed plates, allowed colour spaces, overprint, rich-black violations, CMY-in-grey, mismatched RGB spaces. |
| Ink | Ink density over a threshold for fills, strokes and text. |
| Images and objects | Resolution, transparency and rasterisation-forcing effects, minimum stroke weight, non-proportional scaling, hidden objects, bleed-zone hazards. |
| Text | Missing fonts, missing characters, overset text, spelling, text patterns such as double spaces, straight quotes and double hyphens. |
| Document | Page size, page count, blank pages, bleed and slug. |
| Accessibility and data | Missing alternative text, stale data-merge sources, out-of-date TOC, unnamed anchors, invalid hyperlinks, stale cross-references. |

Preflight findings MUST be surfaced to the model-steerable command surface as STRUCTURED findings
carrying the rule id, the severity, the offending object reference and the page, not only to the
operator UI.

### 14.6.9.2 Package and Print

**[STU-LAY-058]** Package MUST collect the document, its linked resources and its fonts into a portable
folder with a report, for handoff to a printer or archive. Package MUST operate over a book as well
as a single document.

**[STU-LAY-155] Package contract.** The captured package command takes nine explicit arguments and
Studio MUST expose every one, because each changes what the recipient receives: destination path,
`copying_fonts`, `copying_linked_graphics`, `copying_profiles`, `updating_graphics`,
`including_hidden_layers`, `ignore_preflight_errors`, `creating_report`, and a version-comment
string. `ignore_preflight_errors` MUST default to refusing when preflight has errors, and MUST report
which errors were ignored when the caller overrides. Font copying is subject to font licensing and
Studio MUST record, in the report, which fonts were copied and which were withheld.

**[STU-LAY-059]** The print pipeline MUST expose its full option surface.

**[STU-LAY-156] Print contract.** The captured print-preference object carries 109 properties and the
printer-preset object 105. The declared numeric bounds are:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `scale_width` | 0 | 1000 | unknown | unknown | unknown | `percent` | unknown |
| `scale_height` | 0 | 1000 | unknown | unknown | unknown | `percent` | unknown |
| `bitmap_resolution` | 72 | 1200 | unknown | unknown | unknown | `dpi` | 0 |
| `black_angle`, `cyan_angle`, `magenta_angle`, `yellow_angle`, `spot_angle`, `composite_angle` | 0 | 360 | unknown | unknown | unknown | `deg` | unknown |
| `black_frequency`, `cyan_frequency`, `magenta_frequency`, `yellow_frequency`, `spot_frequency`, `composite_frequency` | 1 | 500 | unknown | unknown | unknown | `count` (lines per inch) | unknown |

Grouped option surface, all captured on the same object:

*Derivation: catalogue table, splits per row; yields 8 microtasks, one per print option group.*

| Group | Fields |
|---|---|
| General | `page_range`, `print_spreads`, `copies`, `collating`, `reverse_order`, `print_layers` (`{print_layers.all, print_layers.visible, print_layers.visible_printable}`), `print_parent_pages`, `print_nonprinting`, `print_blank_pages`, `print_guides_grids` |
| Setup | `paper_size`, `paper_width`, `paper_height`, `paper_offset`, `paper_gap`, `paper_transverse`, `print_page_orientation` (`{portrait, landscape, reverse_portrait, reverse_landscape}`), `scale_width`, `scale_height`, `scale_proportional`, `scale_mode`, `page_position`, `thumbnails`, `thumbnails_per_page`, `tile`, `tiling_type`, `tiling_overlap` |
| Marks and bleed | `all_printer_marks`, `crop_marks`, `bleed_marks`, `registration_marks`, `color_bars`, `page_information_marks`, `mark_type`, `mark_offset`, `mark_line_weight`, `bleed_top`, `bleed_bottom`, `bleed_inside`, `bleed_outside`, `bleed_chain`, `use_document_bleed_to_print`, `include_slug_to_print` |
| Output | `color_output`, `trapping` (`{trapping.off, trapping.application_builtin, trapping.in_rip}`), `text_as_black`, `flip`, `negative`, `screening`, `simulate_overprint`, `print_black`/`print_cyan`/`print_magenta`/`print_yellow`, `separation_screening`, `composite_screening`, `preserve_color_numbers` |
| Graphics | `send_image_data` (`{image_data.all, image_data.optimized_subsampling, image_data.proxy, image_data.none}`), `font_downloading`, `download_ppd_fonts`, `postscript_level`, `data_format`, `omit_bitmaps`, `omit_eps`, `omit_pdf`, `opi_image_replacement` |
| Colour management | `source_space`, `intent`, `profile`, `crd` |
| Advanced | `flattener_preset_name`, `ignore_spread_overrides`, `bitmap_printing`, `bitmap_resolution`, `pdf_passthrough` |
| Device | `device_type`, `printer`, `print_to`, `print_to_disk`, `print_file`, `ppd`, `ppd_file`, `print_record` |

Print presets MUST save complete print states as named, exportable and importable presets; the
captured printer-preset object carries a `printer_preset_types` enumeration
`{preset_type.default, preset_type.custom}`. Print-as-bitmap at a chosen resolution MUST be available
for non-PostScript devices via `bitmap_printing` and `bitmap_resolution`. Device-independent and
device-specific PostScript and per-page EPS creation MUST be supported. Captured typed errors: a
half-gap value greater than the crossover; an ink cannot be aliased to a process ink.

**[STU-LAY-060]** Booklet and imposition MUST arrange pages for folding.

**[STU-LAY-157] Booklet contract.** The captured booklet-option object declares:

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Members / notes |
|---|---|---|
| `booklet_type` | enumeration | `booklet.two_up_saddle_stitch`, `booklet.two_up_perfect_bound`, `booklet.two_up_consecutive`, `booklet.three_up_consecutive`, `booklet.four_up_consecutive` -- FIVE members; the v02.205 text named three |
| `page_range` | string | |
| `space_between_pages` | numeric, `pt` | |
| `bleed_between_pages` | numeric, `pt` | |
| `creep` | numeric, `pt` | |
| `signature_size` | integer, `count` | |
| `top_margin`, `bottom_margin`, `left_margin`, `right_margin` | numeric, `pt` | |
| `margins_uniform_size` | boolean | when true the top value governs all four |
| `auto_adjust_margins` | boolean | |
| `print_blank_printer_spreads` | boolean | |

Booklet printing carries its OWN 99-property print-preference object separate from the document print
preference; Studio MUST carry two print configurations, because a booklet's paper and marks settings
differ from the document's.

### 14.6.9.3 PDF and Ink/Separation Output

**[STU-LAY-061]** PDF (print) export MUST expose its full panel surface.

**[STU-LAY-158] PDF export contract.** The captured PDF export preference carries 88 properties and
the PDF export preset 73. Declared numeric bounds:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `color_bitmap_sampling_dpi` | 9 | 2400 | unknown | unknown | unknown | `dpi` | 0 |
| `grayscale_bitmap_sampling_dpi` | 9 | 2400 | unknown | unknown | unknown | `dpi` | 0 |
| `monochrome_bitmap_sampling_dpi` | 9 | 2400 | unknown | unknown | unknown | `dpi` | 0 |
| `threshold_to_compress_color` | 1x | 10x `color_bitmap_sampling_dpi` | unknown | unknown | unknown | `ratio` | unknown |
| `threshold_to_compress_gray` | 1x | 10x `grayscale_bitmap_sampling_dpi` | unknown | unknown | unknown | `ratio` | unknown |
| `threshold_to_compress_monochrome` | 1x | 10x `monochrome_bitmap_sampling_dpi` | unknown | unknown | unknown | `ratio` | unknown |
| `subset_fonts_below` | 0 | 100 | unknown | unknown | unknown | `percent` | 0 |
| `color_tile_size` | 128 | 2048 | unknown | unknown | unknown | `px` | 0 |
| `gray_tile_size` | 128 | 2048 | unknown | unknown | unknown | `px` | 0 |

The three compression thresholds are the clearest example in the layout capture of a bound that is
RELATIVE to another parameter. Studio MUST validate them against the live value of their partner
resolution, MUST NOT store an absolute maximum, and MUST re-validate when the partner changes.

Grouped surface:

*Derivation: catalogue table, splits per row; yields 6 microtasks, one per PDF export panel.*

| Panel | Fields |
|---|---|
| General | `acrobat_compatibility` (`{acrobat_4, acrobat_5, acrobat_6, acrobat_7, acrobat_8}`), `standards_compliance`, `page_range`, `export_reader_spreads`, `export_as_single_pages` with `single_pages_pdf_suffix`, `export_hidden_spread`, `export_layers` (`{layers.all, layers.visible, layers.visible_printable}`), `include_bookmarks`, `include_hyperlinks`, `include_structure`, `export_nonprinting_objects`, `export_guides_and_grids`, `interactive_elements_option`, `pdf_magnification` (ten members), `pdf_page_layout`, `open_in_full_screen`, `pdf_display_title` (`{display.file_name, display.document_title}`), `default_document_language`, `generate_thumbnails`, `optimize_pdf`, `view_pdf`, `preserve_editing_capabilities` |
| Compression | per image class: sampling method, sampling DPI, compression method, quality tier (`{compression_quality.minimum, .low, .medium, .high, .maximum, .four_bit, .eight_bit}`), plus `compress_text_and_line_art`, `crop_images_to_frames`, `compression_type` (`{compress.none, compress.structure, compress.objects}`), `color_tile_size`, `gray_tile_size` |
| Marks and bleeds | `crop_marks`, `bleed_marks`, `registration_marks`, `color_bars`, `page_information_marks`, `page_marks_offset`, `printer_mark_weight` (a nine-member enumeration of fixed weights: 0.125pt, 0.25pt, 0.50pt, 0.05mm, 0.07mm, 0.10mm, 0.15mm, 0.20mm, 0.30mm), `pdf_mark_type`, `bleed_top`/`bleed_bottom`/`bleed_inside`/`bleed_outside`, `use_document_bleed_with_pdf`, `include_slug_with_pdf` |
| Output | `pdf_color_space` (`{pdf_color.rgb, pdf_color.cmyk, pdf_color.unchanged, pdf_color.repurpose_rgb, pdf_color.repurpose_cmyk, pdf_color.gray}`), `pdf_destination_profile`, `include_icc_profiles`, `pdf_x_profile`, `output_condition`, `output_condition_name`, `oc_registry`, `simulate_overprint`, plus the four `effective_*` read-back fields that report what the export actually resolved |
| Advanced | `subset_fonts_below`, `omit_bitmaps`, `omit_eps`, `omit_pdf`, `applied_flattener_preset`, `ignore_spread_overrides` |
| Security | `use_security`, `open_document_password`, `change_security_password`, `disallow_printing`, `disallow_hi_res_printing`, `disallow_changing`, `disallow_copying`, `disallow_extraction_for_accessibility`, `disallow_document_assembly`, `disallow_notes`, `disallow_form_fill_in`, `disallow_plaintext_metadata` |

The four `effective_*` fields (effective destination profile, effective output condition, effective
OC registry, effective PDF/X profile) are READ-BACK values distinct from the requested values, and
Studio MUST expose them so a model can verify what an export actually did rather than what it asked
for. The captured constraint vocabulary declares a typed error for accessing the password property
when security is off; Studio MUST reproduce it rather than return an empty string.

Tagged and accessible PDF MUST be produced from style-to-tag mapping ([STU-LAY-146]), per-object
alternative text and roles, article-driven reading order, tab order ([STU-LAY-107]) and
document-title metadata.

**[STU-LAY-159] Stock PDF export presets.** Studio MUST ship a first-run set of PDF export presets.
The captured stock set is SIX presets, whose real values establish the normative starting points; the
captured file additionally holds seven locale-specific variants, so a preset count and a preset FILE
count are different numbers.

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Setting | High Quality Print | Press Quality | Smallest File Size | PDF/X-1a:2001 | PDF/X-3:2002 | PDF/X-4 |
|---|---|---|---|---|---|---|
| PDF version | 1.4 | 1.4 | 1.5 | 1.3 | 1.3 | 1.4 |
| Colour conversion | leave unchanged | to CMYK | to sRGB | to CMYK | device-independent | device-independent |
| Colour image resolution (`dpi`) | 300 | 300 | 100 | 300 | 300 | 300 |
| Colour downsample threshold | 1.5 | 1.5 | 1.5 | 1.5 | 1.5 | 1.5 |
| Greyscale resolution (`dpi`) | 300 | 300 | 150 | 300 | 300 | 300 |
| Monochrome resolution (`dpi`) | 1200 | 1200 | 300 | 1200 | 1200 | 1200 |
| Colour image filter | DCT | DCT | DCT | DCT | DCT | DCT |
| Auto-filter colour images | true | true | true | true | true | true |
| Embed all fonts | true | true | false | true | true | true |
| Subset fonts | true | true | true | true | true | true |
| Max subset percent | 100 | 100 | 100 | 100 | 100 | 100 |
| Optimize | true | true | true | false | false | false |
| Compress objects | tags | tags | all | off | off | off |
| Auto-rotate pages | all | none | all | none | none | none |
| Allow transparency | false | false | false | false | false | true |
| Cannot-embed-font policy | warning | error | warning | error | error | error |
| Preserve overprint | true | true | true | true | true | true |
| Compliance check | none | none | none | PDF/X-1a:2001 | PDF/X-3:2002 | PDF/X-4:2007 |
| PDF/X compliant only | false | false | false | true | true | true |

Two facts here are load-bearing and easy to get wrong. First, PDF/X-4 is the ONLY stock preset that
allows live transparency; every other stock preset flattens. Second, the preset named for the 2008
PDF/X-4 revision declares compliance against `PDF/X-4:2007`; Studio MUST carry the compliance token
separately from the preset NAME and MUST NOT derive one from the other.

Additional captured defaults common to all six: JPEG-2000 tile 256 x 256 at quality 30; the ACS image
dictionary at QFactor 0.15 with 1:1 chroma sampling; monochrome filter CCITT fax; image memory
1,048,576 bytes; start page 1 and end page -1 (a sentinel meaning "to the last page", which Studio
MUST carry as a nullable end rather than storing -1).

**[STU-LAY-062]** Prepress control surfaces that drive the colour pipeline (14.8) MUST be provided.

**[STU-LAY-160] Ink, overprint and separation contract.** SUPERSEDES the v02.205 [STU-LAY-062]
statement that PDF/X output intents cover "at least PDF/X-1a, PDF/X-3, PDF/X-4". The captured PDF/X
standards enumeration has SIX members, two of which the v02.205 text omits:

`pdfx.none`, `pdfx.x1a_2001`, `pdfx.x3_2002`, `pdfx.x1a_2003`, `pdfx.x3_2003`, `pdfx.x4_2010`.

Ink manager -- the captured ink object declares:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `ink_angle` | 0 | 360 | unknown | unknown | unknown | `deg` | unknown |
| `ink_frequency` | 1 | 500 | unknown | unknown | unknown | `count` | unknown |
| `neutral_density` | 0.001 | 10.0 | unknown | unknown | unknown | `ratio` | 3 |
| `solidity` | 0.0 | 1.0 | unknown | unknown | unknown | `ratio` | unknown |

plus `ink_type`, the four-member enumeration `{ink.normal, ink.opaque, ink.transparent,
ink.opaque_ignore}`, and per-ink spot-to-process conversion, all-spots-to-process, ink aliasing and
use-standard-Lab-values-for-spots. Captured typed error: a process ink cannot be aliased. Mixed-ink
constraints captured as typed errors Studio MUST enforce: a mixed ink may contain no more than 27
spot inks and no more than 31 inks in total, and a mixed-ink GROUP may contain no more than 1,000
swatches.

Overprint -- per-object overprint of fill, stroke and gap, plus a document-scoped black-overprint
policy (`overprint_black_on_save` in [STU-LAY-105]).

Separations preview -- per-plate on/off preview, an ink-limit view with a configurable total-ink
threshold, and per-ink coverage readouts. These MUST be readable as typed values, not only rendered.

**[STU-LAY-250] Total ink coverage contract.** Total ink coverage -- also called total area
coverage, and configured as a total ink limit -- is the arithmetic sum, at one device location, of
the tone values of every ink that prints at that location, expressed in percent. For a separation
carrying `n` inks the structural range is 0 to `n` x 100 percent, so a four-ink process page reaches
400 percent at solid registration black and a page carrying spot inks exceeds that; the `hard_max`
below is stated for the four-ink process case and scales with the ink count of the actual
separation. A location above the limit of the print condition does not dry, sets off onto the
following sheet, picks, and turns neutral shadows muddy, and on non-heatset presses it is the
dominant press defect. This sub-section already specifies separation, overprint and an ink-limit
PREVIEW in the ink contract of [STU-LAY-160], but declared no limit a document could be CHECKED
against, so a document could pass preflight, be packaged and be exported in a state the press cannot
print. Studio MUST carry the total ink limit as a first-class stored contract, MUST evaluate a
document against it, and MUST report a violation as structured data rather than only as a rendered
preview.

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `total_ink_limit` | 0 | 400 | unknown | unknown | unknown | `percent` | unknown |
| `single_ink_limit` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `black_ink_limit` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `coverage_warning_margin` | 0 | unknown | unknown | unknown | unknown | `percent` | unknown |
| `coverage_sample_resolution` | unknown | unknown | unknown | unknown | unknown | `dpi` | 0 |

The three limits are INDEPENDENT and MUST be stored as three fields. `black_ink_limit` is not
derivable from `single_ink_limit`, and `total_ink_limit` is not derivable from either: a condition
that accepts 100 percent black in a single plate may still refuse 340 percent across four. The
`hard_max` of 400 is arithmetic for a four-ink process separation, not a captured bound; for a
separation carrying more inks the structural maximum is 100 x ink count and the engine MUST scale
the bound rather than clamping to 400. Every `unknown` in the table is an undeclared bound under
[STU-LAY-100] and MUST NOT be filled from its opposite range. `coverage_warning_margin` is the
distance below the effective limit at which a finding is raised at a lower severity, so a document
sitting at the edge of a condition is visible before it crosses; it is a separate field and is never
inferred from the limit.

**[STU-LAY-251] Ink-limit declaration sites and precedence.** A total ink limit reaches a document
from more than one place, and in practice those places disagree, so the resolution order MUST be
declared rather than decided by whichever surface is read last. For a given output operation Studio
resolves the effective limit by taking the first DECLARED value in the member order of
`ink_limit_source` below, and MUST record which site supplied it alongside the value, so a
downstream reader can distinguish an operator-typed limit from one inherited from an output intent.
The terminal member `ink_limit_source.undeclared` is normative and is not a synonym for zero or for
400: when no site declares a limit the evaluation of [STU-LAY-252] MUST report `not_checked`, and
Studio MUST NOT report a pass. A silent pass on an undeclared limit is the exact failure this
contract exists to prevent.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Members |
|---|---|---|
| `ink_limit_source` | enumeration, ordered by precedence | `ink_limit_source.preflight_rule_instance`, `ink_limit_source.export_recipe`, `ink_limit_source.document_policy`, `ink_limit_source.output_intent_profile`, `ink_limit_source.press_condition_preset`, `ink_limit_source.undeclared` |
| `ink_limit_severity` | enumeration | `severity.disabled`, `severity.error`, `severity.warning`, `severity.informational` |
| `ink_limit_scope` | enumeration | `ink_limit_scope.document`, `ink_limit_scope.spread`, `ink_limit_scope.page`, `ink_limit_scope.object`, `ink_limit_scope.plate` |
| `ink_limit_check_state` | enumeration | `check_state.not_checked`, `check_state.within_limit`, `check_state.within_warning_margin`, `check_state.over_limit`, `check_state.indeterminate` |

`ink_limit_severity` reuses the preflight severity vocabulary of [STU-LAY-154] deliberately: an ink
limit is a preflight rule instance like any other and MUST NOT acquire a second severity scale.
`check_state.indeterminate` covers the case where a placed asset cannot be separated for
measurement -- a missing link, an unresolvable profile, or a passthrough object -- and MUST NOT be
collapsed into either `not_checked` or `within_limit`.

**[STU-LAY-252] Coverage evaluation contract.** A limit is meaningless unless what is measured is
the value that will actually reach a plate. Studio MUST evaluate coverage on the COMPOSED and
SEPARATED device values of the output operation, at a declared sample resolution, after every
transformation that can change them, and MUST NOT evaluate it on authored swatch values or on an
unflattened composite. An implementer may reason that a correctly built destination profile cannot
emit a separation above its own limit and that the check is therefore redundant. It is not: the
stages below all add ink AFTER the profile has done its work, and each is a normal layout
construction rather than an error.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members.*

| Field | Kind | Members |
|---|---|---|
| `coverage_input_stage` | enumeration | `stage.spot_to_process_conversion`, `stage.ink_alias_resolution`, `stage.overprint_composition`, `stage.transparency_flattening`, `stage.placed_image_separation`, `stage.authored_rich_black`, `stage.opaque_ink_handling` |
| `coverage_metric` | enumeration | `metric.maximum`, `metric.mean`, `metric.area_above_limit` |

Each stage names a real mechanism. Spot-to-process conversion and ink aliasing of [STU-LAY-160]
replace one plate with several. Overprint composition sums two objects that would otherwise knock
out. Transparency flattening of [STU-LAY-161] produces new atomic regions whose colour is neither
operand. A placed image separated elsewhere to a different limit carries its own coverage in. An
authored rich black or registration swatch is typed by hand and no profile ever sees it. Opaque and
opaque-ignore inks under the `ink_type` enumeration of [STU-LAY-160] obscure what lies beneath them,
so their contribution to a total is not a simple sum and MUST be computed from the declared ink
type rather than assumed. The three metrics are all required: a maximum alone cannot distinguish a
single antialiased edge pixel from a flooded panel, which is why the mean and the fraction of the
evaluated region above the limit are carried with it. `coverage_sample_resolution` is part of the
result, not a hidden setting, because a violation found at one resolution can vanish at another.

**[STU-LAY-253] Ink-limit violation finding.** A violation MUST be reported as a typed structured
finding in the vocabulary of [STU-LAY-166] and MUST reach the model-steerable command surface on the
same terms as every other preflight finding under [STU-LAY-057], not only as a highlight drawn over
the page. A finding that says only that ink coverage is too high is non-conformant: neither an
operator nor a model can repair the document from it, because the repair depends on which ink
contributes the excess and on which stage of [STU-LAY-252] introduced it.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Field | Meaning |
|---|---|
| `rule_id` | Stable identifier of the ink-limit rule instance that fired. |
| `severity` | The `ink_limit_severity` member in force for that rule instance. |
| `check_state` | The `ink_limit_check_state` member the evaluation reached. |
| `effective_limit` | The percentage actually applied. |
| `limit_source` | The `ink_limit_source` member that supplied `effective_limit`. |
| `measured_maximum` | Highest total coverage found in the evaluated region, in percent. |
| `measured_mean` | Mean total coverage over the evaluated region, in percent. |
| `area_above_limit` | Fraction of the evaluated region above the limit, in percent of that region. |
| `per_ink_contribution` | Per-plate tone value at the worst location, so the offending ink is named rather than inferred. |
| `page_reference` | Page or spread identity, plus the offending region in document coordinates. |
| `object_references` | The page items composing at the worst location, placed assets carried by link reference. |
| `introduced_by` | The `coverage_input_stage` member that raised the value above the limit. |
| `sample_resolution` | The resolution the evaluation ran at. |

**[STU-LAY-254] Published ink limits are reference, not a Studio default (SPEC GAP).** No captured
object declares a total-ink-limit DEFAULT, and none is invented here. The correct value is a
property of the print condition -- press process, ink set, substrate, drying and finishing -- and is
supplied by the printer, so a shipped default would be a guess that reads as authority. This is a
declared SPEC GAP: Studio MUST ship with `total_ink_limit` undeclared and MUST report
`check_state.not_checked` until a site of [STU-LAY-251] declares one. The values below are published
figures for common conditions, recorded so an implementer can size the control and build a preset
list, and they are REFERENCE ONLY: not a default, not a validated recommendation, and not a Studio
name for any of them. Published figures for the SAME characterisation data differ between
publishers, because the limit is a property of the separation built into a profile rather than of
the characterisation data set; where field guidance gives a range rather than a constant, the range
is reproduced as a range and MUST NOT be reduced to its midpoint.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*

| Print condition | Published total ink limit | Provenance of the figure |
|---|---|---|
| Sheetfed offset, gloss and matte coated, paper types 1 and 2 | 330 percent, with a 300 percent variant published beside it | ECI profiles built on the FOGRA39 characterisation for ISO 12647-2 |
| Sheetfed offset, premium coated, later ISO 12647-2 revision | 300 percent | PSO Coated v3, built on the FOGRA51 characterisation |
| Sheetfed offset, US number 1 coated | 310 percent | GRACoL 2006 |
| Web offset, US number 3 coated publication stock | 300 percent | SWOP 2006 |
| Sheetfed offset, wood-free uncoated | 260 to 300 percent | ECI uncoated profiles on the FOGRA47 and FOGRA52 characterisations; field prepress guidance gives the wider figure |
| Heatset web offset, coated | 300 to 320 percent | field prepress guidance |
| Non-heatset web offset, newsprint | 240 to 260 percent | field prepress guidance |
| Sheetfed offset, Japanese coated | 300 percent | Japan Color 2011 |
| Electrophotographic and inkjet devices | 300 to 350 percent | field prepress guidance; strongly device and substrate dependent |

The figure depends on the press process, the ink set and its tack and drying behaviour, the
substrate and its absorbency, whether drying is heatset or oxidative, the screening, the press speed
and the finishing, which is why it varies within a single named condition and why the printer is the
authority for it.

Relationship to separation and black generation. The limit is honoured in two different places and
they are not interchangeable. A destination profile honours it at SEPARATION time through its own
black generation -- how much of the neutral cyan, magenta and yellow component is replaced by black,
and how much black is added back -- so a profile built to a 300 percent limit cannot emit a
separation above 300 percent, and changing the limit changes the black generation and therefore the
appearance of every neutral in the document. Every stage enumerated in [STU-LAY-252] happens AFTER
that point, which is why the document check of this contract is not redundant with a correctly built
profile. Studio MUST NOT silently re-separate a document to bring it under a limit: because
re-separation restates every neutral, it is an operator decision recorded as a document mutation
through the propose-work lifecycle of [STU-LAY-067], never a repair applied inside an export.

**[STU-LAY-152] Trap preset contract.** Referenced by the vector domain's trap operation
([STU-VEC-132]). The captured trap-preset object declares:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `default_trap_width` | 0.0 | 8.0 | unknown | unknown | unknown | `pt` | unknown |
| `black_width` | 0.0 | 8.0 | unknown | unknown | unknown | `pt` | unknown |
| `black_color_threshold` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `black_density` | 0.001 | 10 | unknown | unknown | unknown | `ratio` | 3 |
| `sliding_trap_threshold` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `step_threshold` | 1 | 100 | unknown | unknown | unknown | `percent` | unknown |
| `color_reduction` | 0 | 100 | unknown | unknown | unknown | `percent` | unknown |

A second capture of `black_density` declares 0.001..12.0; per [STU-LAY-103] both are recorded and the
normative Studio hard maximum is 12.0 (the wider). The two width parameters are declared per unit --
0.0 to 8.0 points, 0 to 0.1111 inches, 0 to 2.822 mm, 0 to 0.2822 cm, 0p0 to 0p8 picas, 0c0 to
0c7.507 ciceros -- and `pt` is canonical per [STU-LAY-100]. `trap_end_type` is
`{trap_end.miter, trap_end.overlap}`; `trap_image_placement` is `{trap_image.center_edges,
trap_image.choke, trap_image.image_neutral_density, trap_image.images_over_spread}`. A trap preset is
assignable to a page range via the page's `applied_trap_preset` ([STU-LAY-107]).

**[STU-LAY-161] Transparency flattener contract.**

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `line_art_and_text_resolution` | 1 | 9600 | unknown | unknown | unknown | `dpi` | unknown |
| `gradient_and_mesh_resolution` | 0 | 1200 | unknown | unknown | unknown | `dpi` | unknown |

plus a raster-vector balance, convert-all-text-to-outlines, convert-all-strokes-to-outlines,
clip-complex-regions booleans (bounds not recovered for the balance -- declared SPEC GAP), and the
five-member spread flattener level of [STU-LAY-135]. A flattener PREVIEW highlighting rasterised and
outlined regions MUST be available and MUST report its findings as typed values.

**[STU-LAY-063]** Raster export MUST export a selection, ranges, or all pages and spreads with quality,
resolution, colour space, anti-aliasing, bleed and overlap ([STU-LAY-162]). Data merge MUST bind a
data source (delimited text, a JSON array of objects, or a spreadsheet, including image fields by
path) to placeholder fields in the layout and generate merged output, supporting preview records,
multiple records per page with arrangement, margins and spacing, a repeating grid-layout mode,
content-placement options, blank-line removal, record-range filtering, per-record QR-code fields and
direct-to-PDF merge without an intermediate document ([STU-LAY-163]).

**[STU-LAY-162] Raster export contract.** The captured PNG export preference declares:

*Derivation: parameter table, taken whole; yields 1 microtask whose acceptance criteria are its rows, one bound-set per row.*

| Parameter | hard_min | hard_max | soft_min | soft_max | default | unit | precision |
|---|---|---|---|---|---|---|---|
| `export_resolution` | 1.0 | 2400.0 | unknown | unknown | unknown | `dpi` | unknown |

plus `export_range_or_all_pages` (`{range.export_range, range.export_all}`), and for the JPEG class a
quality tier and a format enumeration. Raster export MUST export a selection, a range, or all pages
or spreads, with quality, resolution, colour space, anti-aliasing, bleed and overlap. A separate
captured image-resolution enumeration used for reflowable export is
`{image_res.ppi_72, image_res.ppi_96, image_res.ppi_150, image_res.ppi_300}`.

**[STU-LAY-163] Data-merge contract.** The captured data-merge model is five object types -- a data
merge, a data-merge FIELD, and three placeholder kinds (text, image, QR code) -- plus an option
object and a preference object. Studio MUST carry all three placeholder kinds; a QR placeholder is
not an image placeholder.

*Derivation: enumeration table, taken whole; yields 1 microtask whose acceptance criteria are its members. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field | Kind | Members / notes |
|---|---|---|
| `record_selection` | enumeration | all records, one record, a range |
| `record_number` | integer | valid only when the selection is one record |
| `record_range` | string | valid only when the selection is a range |
| `records_per_page` | enumeration | one record per page, or multiple |
| `arrange_by` | enumeration | rows first, or columns first |
| `row_spacing`, `column_spacing` | numeric, `pt` | |
| `left_margin`, `top_margin`, `right_margin`, `bottom_margin` | numeric, `pt` | of the TARGET document |
| `fitting_option` | enumeration | [STU-LAY-131] |
| `center_image` | boolean | preserves frame and content size; the capture states it does NOT combine with content-aware fitting |
| `link_images` | boolean | link versus embed in the target |
| `remove_blank_lines` | boolean | remove lines left blank by empty fields |
| `create_new_document` | boolean | |
| `document_size` | integer, `count` | maximum pages per generated document |

Commands: merge, preview a record, and `alert_missing_images` taking an output report path. Captured
typed errors Studio MUST reproduce: the data source file cannot be found; a placeholder has no
matching field in the source; missing placeholders; one or more record numbers in the range are
invalid; the data file is not a text file; the data source has no records or is an unsupported
format. Direct-to-PDF merge without an intermediate document MUST be supported. Data merge is a
`StudioExportRecipe` variant and the source is embeddable in the document.

### 14.6.9.4 Structured Interchange

**[STU-LAY-064]** Round-trip interchange formats -- layout markup export and open for cross-version
exchange, tagged-text plain-format round-trip, and structured-XML tagging with a structure tree, tag
mapping to and from styles, schema validation and image-copying export -- MUST be supported.

**[STU-LAY-164] Structured-XML contract.** The captured XML surface is the largest single error
domain in the layout capture (447 of 1,453 typed errors come from the XML parser alone), which
establishes that XML validity reporting is a first-class feature and not an afterthought. Studio MUST
carry: a document type definition object with a root tag, a system id and a public id; import-DTD and
import-XML commands; a structure tree with elements carrying tags and attributes; tag-to-style and
style-to-tag mapping; schema validation reporting per-node findings as typed values; and an
image-conversion enumeration for exported XML assets, captured in two forms --
`{xml_image.automatic, xml_image.jpeg, xml_image.gif}` and
`{xml_image.automatic, xml_image.jpeg, xml_image.gif, xml_image.png}`. Per [STU-LAY-103] the
four-member form is normative.

The concrete interchange file formats and matrices are owned by 14.13; this clause fixes that layout
content is representable in structured exchange form and that validation findings are structured.

---

## 14.6.10 Interactive/EPUB Touchpoint and Collaboration Posture

**[STU-LAY-065]** Layout objects with interactive behaviour -- hyperlinks, bookmarks, buttons,
multi-state objects ([STU-LAY-136]), placed media, interactive TOCs ([STU-LAY-148]) and
reflowable/fixed-layout EPUB export -- are AUTHORED in this catalog but their runtime interaction
model and reflowable export pipeline are owned by 14.11. A layout object's interactive payload MUST
be carried on its `StudioLayer` and handed to 14.11 for export; this catalog MUST NOT define a second
interactive runtime.

**[STU-LAY-165] Reflowable-export handoff contract.** The captured EPUB export preference carries 57
properties and the fixed-layout variant 34. Studio MUST carry the handoff payload named here, because
without it the layout side cannot supply what 14.11 needs:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Field group | Fields |
|---|---|
| Version and structure | `version` (`{epub.v2, epub.v3, epub.v3_with_layout}`), `export_order` (`{order.layout, order.article_panel, order.xml_structure}`), `break_document` and `paragraph_style_name` (split points), `toc_style_name`, `epub_create_page_navigation`, `navigation_style` (`{nav.none, nav.filename, nav.toc_style, nav.bookmarks}`) |
| Cover | `epub_cover` (`{cover.none, cover.first_page, cover.external_image}`), `cover_image_file` |
| Images | `custom_image_size_option`, `image_export_resolution`, `image_alignment` (`{image_align.left, image_align.center, image_align.right}`), `image_space_before`, `image_space_after`, `use_image_page_break`, `image_page_break` (`{page_break.before, page_break.after, page_break.before_and_after}`), `image_conversion`, `gif_options_palette`, `gif_options_interlaced`, `jpeg_options_quality`, `jpeg_options_format`, `use_svg_as`, `use_existing_image_on_export`, `ignore_object_conversion_settings` |
| Layout and CSS | `preserve_layout_appearance`, `generate_cascade_style_sheet`, `include_classes_in_html`, `external_style_sheets`, `javascripts`, `preserve_local_override`, `strip_soft_return`, `embed_font`, `left_margin`, `right_margin`, `top_margin`, `bottom_margin` |
| Lists and notes | `bullet_export_option`, `numbered_list_export_option`, `footnote_placement` ([STU-LAY-150]) |
| Metadata | `epub_publisher`, `id`, `epub_title`, `epub_creator`, `epub_date`, `epub_description`, `epub_rights`, `epub_subject` |
| Accessibility | `epub_accessibility_feature`, `epub_accessibility_hazard`, `epub_accessibility_mode`, `epub_accessibility_mode_sufficient`, `epub_accessibility_summary`, `epub_accessibility_conforms_to`, `epub_accessibility_certified_by`, `epub_accessibility_credentials`, `epub_accessibility_report_link` |

The nine accessibility metadata fields are first-class captured properties and MUST NOT be reduced to
a single "accessible" flag. The fixed-layout variant additionally carries a spread-control
enumeration `{fxl_spread.based_on_document, fxl_spread.physical, fxl_spread.synthetic,
fxl_spread.none}`.

Page transitions, captured as a thirteen- or fourteen-member type enumeration plus a
seventeen-member direction enumeration and a three-member duration enumeration
(`{transition_duration.fast, .medium, .slow}`), are authored on the spread ([STU-LAY-107]) and
exported by 14.11. The captured typed error "the specified direction is not supported by the page
transition" means the two enumerations are NOT independent; Studio MUST carry the legal
direction set per transition type and MUST validate the pair.

**[STU-LAY-066]** Review and collaboration MUST be local-first. The native review surface -- comments,
annotations and markup anchored to layout positions, plus a document-states/history surface beyond
linear undo (14.19) -- is a first-class native Studio capability backed by kernel CRDT collaboration,
requiring no external account or cloud service. Hosted/cloud collaboration capabilities MUST be
treated as optional adapter-backed rows, never as the primary path:

*Derivation: preset/command table, taken whole; yields 1 microtask. Its rows are acceptance criteria and MUST NOT become one microtask each.*

| Capability | Studio posture |
|---|---|
| Hosted share-for-review link with pin, highlight, strike, insert and reply comments | Adapter-backed / optional; the native local review surface is the primary path. |
| Cloud documents, autosave, cross-device version history | Adapter-backed / optional; local document plus kernel history is the primary path. |
| Invite-to-edit, shared projects, asset-library sync | Adapter-backed / optional; kernel CRDT co-edit is the primary path. |
| Hosted font activation and hosted custom-font rendering in reviews | Adapter-backed / optional; local font install is the primary path. |
| File-based assignment and check-in/check-out copy-editing workflow | Native; operates on shared local or network storage. |
| Generative assists (text variation, text-to-image, expand-image) | Adapter-backed / optional; provider-dependent via the model-lane surface. |

The captured comment-import model is four object types -- a comment plus three destination kinds
(page, page item and text) -- and an import command taking a source path and a property bag. Studio
MUST carry all four so an imported comment anchors to the right thing. The captured check-in/check-out
constraint vocabulary (locked elsewhere, locked by another user, locked exclusively, locked shared,
would clobber a lock, a valid username is required) MUST be reproduced as typed errors for the native
file-based workflow.

---

## 14.6.11 Constraint Vocabulary and Cross-Cutting Obligations

**[STU-LAY-166] Layout constraint and error vocabulary.** The captured error tables are the source
application's own validation specification: 1,453 distinct error codes across 71 plug-ins, 1,436 with
recovered English text and 421 classified as constraint candidates. Studio MUST implement a TYPED
error vocabulary of at least the granularity the captured tables demonstrate. The normative rule is
structural, not a literal transcription: every rule this sub-section states as MUST is a distinct
typed error carrying a stable identifier, a human-readable message, and enough structured context
(the offending object reference, the field, and the offending value) for a model to repair its input
without guessing.

The specific constraints Studio MUST enforce as distinct typed errors are those named inline
in [STU-LAY-105] through [STU-LAY-165]. The largest clusters, so an implementer can size the
work: XML and structure validation (447 captured codes), interactive and multi-state authoring
(96), document framework and swatches (76), general page items (48), table and cell styles
(63), assignment and check-in workflows (61), hyperlinks and cross-references (27), links
(26), fonts (26), data merge (19), package and preflight (18), PDF export (18), books (16),
sections (11), indexing (10), conditional text (10), spreads (10), layers (8), table model
(8), create-outlines (8).

Two constraint families deserve explicit restatement because they are easy to implement wrongly, and
because each is a separate validation surface with its own error set rather than a variation of the
other. They are enumerated here so the work is derivable rather than asserted in prose.

*Derivation: catalogue table, splits per row; yields 2 microtasks, one per validation family. Anchors appearing in this table's cells are cross-references to clauses defined as paragraphs elsewhere in this sub-section; they are NOT clause definitions and yield no microtask here.*

| Validation family | Distinct captured errors it MUST keep separate | Studio obligation |
|---|---|---|
| Unit and dimension validation | "not a valid dimension"; "not a valid unit", against the accepted suffix list of [STU-LAY-101]; "not a legal scale value"; "not a legal skew value", bounded -360.0 to 360.0; "object would be too small"; "object would be too large" -- SIX separate errors | Studio MUST NOT collapse them into one error. |
| Lock and permission validation | a locked object; a locked layer; a locked story; a locked guide; a locked page item on a locked layer; moving locked items to another layer -- SIX separate errors | Studio MUST report which lock blocked the operation. |

**[STU-LAY-167] Engine boundary and determinism.** Layout composition, reflow, threading, table
composition, imposition and flattening MUST live behind the `LayoutEngine` trait in the
`studio-engine` crate, and text shaping behind `TextEngine` per [STU-TYP-008], which requires a
native Rust shaping stack of the cosmic-text / rustybuzz / swash class and FORBIDS a platform text
engine even where one would be easier. `handshake_core` MUST NOT gain `wgpu`, WGSL or any GPU
dependency for this domain ([STU-ARC-002]). Composition MUST be deterministic: identical inputs
(story content, styles, frame geometry, grid state, conditional-text visibility, `real_precision`
per [STU-LAY-116]) MUST produce byte-identical line breaks, page breaks and geometry on every host.
Determinism here is a promotion-equivalence requirement: a model-authored reflow and an
operator-authored reflow must agree exactly, or the `PromotionGate` cannot compare them.

**[STU-LAY-168] Validation descriptor set.** The `StudioValidationDescriptor` catalog (14.24) MUST
carry, for this domain, at minimum: one descriptor per numeric parameter bound-set declared in 14.6,
asserting hard-bound rejection and soft-bound acceptance as SEPARATE assertions; one per enumeration,
asserting every member round-trips by token or value; one per relative bound ([STU-LAY-158]
compression thresholds) asserting revalidation when the partner value changes; one per ordering
invariant ([STU-LAY-125] justification triples); one per typed error named in [STU-LAY-166]; one per
stock PDF preset in [STU-LAY-159] asserting its captured values survive a save/load round-trip; and
one per declared SPEC GAP asserting the gap is still open. A descriptor that asserts only that a
command succeeded is insufficient.

**[STU-LAY-067]** Every capability in this catalog MUST be exposed through BOTH the operator UI and the
typed model-steerable command/MCP surface as two projections of one primitive per [STU-DOC-004], MUST
satisfy the model-visibility/steerability and parallel-workflow requirements (14.16, 14.17), MUST obey
the headless/quiet output law so preflight, package, print, PDF and merge never steal focus or block
on a foreground window (14.20), and MUST be represented in the dual-audience UserManual and the
GUI/diagnostic and accessibility surfaces (14.22, 14.16). Model-authored layout mutations MUST pass
the sandbox -> validation -> `PromotionGate` lifecycle ([STU-ARC-005]) via the propose-work system
(14.18) before layout authority rows change. All durable layout authority persists through the
canonical Studio SurrealDB tables and `studio.layout` EventLedger events (14.23) under the
SurrealDB-only authority guard ([STU-SDB-002]); live collaborative editing is CRDT-backed. This
obligation is stated once and is not restated per clause.

**[STU-LAY-199] Microtask derivation index.** Applying [STU-LAY-104] to this sub-section yields
exactly 217 microtasks. The correspondence is NORMATIVE and CLOSED: a microtask corresponds to a
yielding clause or to a table unit as marked, and to nothing else.

Rule 0 -- derivation markers are authoritative. Every table in this sub-section carries an italic
`*Derivation: ...*` marker sentence directly above it stating how many microtasks that table yields.
The marker is normative. A tool that classifies a table differently from its marker has diverged
from this sub-section and MUST be corrected to the marker, not the reverse. The five marker forms
are: parameter table taken whole (1); enumeration table taken whole (1); preset or command table
taken whole (1); catalogue table splitting per row (N); contract table carried into the clause's own
microtask (0). A sixth form, reading aid inside a non-yielding clause, also yields 0.

Rule 0a -- anchors inside table cells are never definitions here. Every one of the 142 clauses in
14.6 is defined as a PARAGRAPH opening with its bold anchor; not one is defined inside a table cell.
All 27 anchors that appear in cells of this sub-section are cross-references to clauses defined that
way elsewhere in it, and every table carrying one says so in its own marker. A tool that treats an
in-cell anchor as a clause definition here produces a second unit for a clause rule A has already
counted, which is a double count and not work. This rule constrains only 14.6; other modules do
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

Nine clauses yield no microtask. Tables inside a non-yielding clause yield nothing either.

- `STU-LAY-100` -- reading rule: the seven-field numeric parameter contract.
- `STU-LAY-101` -- reading rule: measurement systems and the unit vocabulary.
- `STU-LAY-102` -- reading rule: the enumeration contract.
- `STU-LAY-103` -- reading rule: the capture-conflict rule.
- `STU-LAY-104` -- reading rule: the microtask derivation rule.
- `STU-LAY-067` -- cross-cutting obligations, inherited by reference by every microtask.
- `STU-LAY-199` -- this clause, the derivation index itself.
- `STU-LAY-023` -- pointer clause: text-on-path is owned by 14.5 and 14.7 and this clause only
  forbids reimplementing it here.
- `STU-LAY-041` -- pointer clause: export tag mapping is defined by clause 146 and this clause
  only states where the mapping is stored.

Exactly one anchor per bullet, in backticks, is a member of this set. No other anchor in this
block is backticked, so a reader and a parser select the same nine clauses.

*Derivation: reading aid inside a non-yielding clause; yields no microtask.*

| Ledger line | Basis | Yields |
|---|---|---|
| Clauses in 14.6 | anchors 001-067, 100-169, 199 and 250-254 | 142 |
| less the no-yield set | reading rules 100-104, plus 067, plus 199, plus the two pointer clauses 023 and 041 | -9 |
| **Rule A subtotal** | one microtask per yielding clause | **133** |
| Parameter tables | 16 tables, each taken whole; rows are bound-set acceptance criteria | 16 |
| Enumeration tables | 25 tables, each taken whole; members are acceptance criteria | 25 |
| Preset and command tables | 3 tables, each taken whole and explicitly NOT split per row | 3 |
| Catalogue: placed formats of 028 | one per placed format class | 6 |
| Catalogue: preflight categories of 057 | one per rule category | 7 |
| Catalogue: find/change modes of 129 | one per search mode | 5 |
| Catalogue: style types of 142 | one per layout style type | 6 |
| Catalogue: print groups of 156 | one per print option group | 8 |
| Catalogue: PDF panels of 158 | one per PDF export panel | 6 |
| Catalogue: validation families of 166 | one per validation family | 2 |
| Contract tables | 11 tables carried into the owning clause's microtask | 0 |
| Reading aids in non-yielding clauses | 2 tables | 0 |
| **Rule B subtotal** | table units | **84** |
| **Total microtasks yielded by 14.6** | rule A plus rule B | **217** |

Four counts are traps for a tool that reads tables structurally rather than reading the markers.
Clause 156 spawns from its eight-row print GROUP table, not from the five-row numeric parameter
table that precedes it. Clause 158 spawns from its six-row PDF PANEL table, not from the nine-row
numeric parameter table that precedes it. The seventeen book synchronisation categories of 147 share
one implementation and collapse to ONE microtask, so that table is marked whole. The collaboration
posture table of 066 is taken WHOLE and yields 1, not 6: five of its six rows are optional
adapter-backed lanes rather than units of build work.

Clauses carrying a declared SPEC GAP -- 010 (adjust-layout font-size limits), 108 (per-section
include-on-export flag), 150 (sidenote options), 161 (raster-vector balance) and 254 (no captured
total-ink-limit default, which is a property of the print condition and is supplied by the printer)
-- still yield their rule-A microtask, and that microtask's FIRST acceptance row MUST read "the named
gap is raised to the operator as a capture request and is NOT closed by an invented value".

A microtask derived from a clause with a parameter table MUST carry that table verbatim, including
every `unknown`; a microtask derived from an enumeration MUST carry every member and its captured
value. No microtask may cite the green-room corpus as its source of truth: the corpus is provenance
for HOW a clause was derived, and this sub-section is the authority ([STU-SECTION-002]).

---
