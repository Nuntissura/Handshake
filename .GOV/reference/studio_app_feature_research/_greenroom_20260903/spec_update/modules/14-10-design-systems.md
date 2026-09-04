---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-10"
section_id: "14.10"
title: "14.10 Design Systems, Components & Variables"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
supersedes_section: "14.10 in .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md lines 2147-2338"
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-10-design-systems.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.10 Design Systems, Components & Variables

This sub-section is the normative Studio feature set for design systems: reusable component definitions, variant sets, component properties, the instance override model, the design-token variable system with its collections and modes, the multi-property style registry, and the responsive layout contract (auto layout, constraints, layout grids). It is the deduped union of the source suites' design-system surfaces collapsed to one Studio primitive and one command family per shared capability ([STU-SECTION-003]). Every capability here operates on `StudioLayer` nodes inside the single unified `StudioDocument` ([STU-DOC-001]).

### 0. Baseline, supersession and disposition

**[STU-DS-100] Baseline preservation and supersession.** Clauses [STU-DS-001] through [STU-DS-051] of v02.205 remain in force as the behavioural surface of this domain and are NOT retired. Clauses [STU-DS-100] and above add the field-level, enumerated-value, bound and default contract those clauses assumed but did not state. Where a v02.205 clause and a clause in this block disagree, the clause in this block wins and the v02.205 clause is amended to match; each such case is named explicitly in [STU-DS-101].

**[STU-DS-101] Explicit supersessions.** The following v02.205 clauses are narrowed or corrected here:

| v02.205 clause | Disposition | Replacement |
|---|---|---|
| [STU-DS-005] asset-browser surface | NARROWED | The design-system asset browser lists components, styles and variable collections ONLY. It is not a file or media catalog; media and file assets resolve through CKC per [STU-ASSET-004]. |
| [STU-DS-013] "Exposed nested" listed as a component property TYPE | CORRECTED | Exposed-nested is not a member of the component-property type enumeration ([STU-DS-110]); it is an instance-level flag (`is_exposed_instance`) plus a derived read-only projection ([STU-DS-118]). |
| [STU-DS-022] four variable types | EXTENDED | The resolved type set is six ([STU-DS-125]); the stored type set is eight ([STU-DS-126]). |
| [STU-DS-036] grid flow "with per-child row/column span" | EXTENDED | Grid flow carries its own eleven-member contract ([STU-DS-146]); span is expressed on the child, not the container. |
| [STU-DS-046a]/[STU-DS-046b] constraint value lists | CORRECTED | Both axes take the SAME five-member enumeration ([STU-DS-150]); the v02.205 lists used different display words for the same members. |
| [STU-DS-050] design-system analytics | UNCHANGED in intent, bounded here by [STU-DS-172]. |

---

### 1. The component object contract

**[STU-DS-102] Component node kinds.** Studio MUST model three distinct authority node kinds in the component system, and MUST NOT collapse them into fewer:

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own.*
| Studio node kind | Role | Container of |
|---|---|---|
| `component` | one reusable definition | ordinary `StudioLayer` children |
| `component_set` | a variant set: the container that owns a variant-property schema | two or more `component` nodes, each a variant member |
| `component_instance` | a live reference to exactly one `component` | override records only |

A `component_set` is not itself instantiable; instantiating a set resolves to its default member ([STU-DS-107]).

**[STU-DS-103] Component node required fields.** A `component` node MUST carry, in addition to every `StudioLayer` field:

| Field | Type | Required | Semantics |
|---|---|---|---|
| `component_id` | prefixed string `SCMP-{uuid_v7}` | yes | stable identity at every API, EventLedger and receipt boundary ([STU-ARC-004]) |
| `description` | string | yes (may be empty) | plain-text description |
| `description_markdown` | string | yes (may be empty) | markdown form of the same description; the two are separate stored fields, not one field rendered two ways |
| `documentation_links` | array of `{name, url}` | yes (may be empty) | external documentation targets |
| `publish_key` | string | yes | stable cross-document key used by library consumers; distinct from `component_id` |
| `is_remote` | boolean | yes | true when the definition arrived from a subscribed library rather than this document |
| `variant_properties` | map<string,string> or null | null unless a variant member | this member's `property=value` assignment within its parent set |
| `component_property_definitions` | array of [STU-DS-110] records | yes (may be empty) | the instance-facing knobs |

**[STU-DS-104] Component set required fields.** A `component_set` node MUST carry `component_id` (prefix `SCMP-`), `description`, `description_markdown`, `documentation_links`, `publish_key`, `is_remote`, `default_variant_id` (the `component_id` of the member returned when the set is instantiated), `variant_group_properties` (map from variant-property name to its ordered list of declared values), and `component_property_definitions`.

**[STU-DS-105] Component instance required fields.** A `component_instance` node MUST carry:

| Field | Type | Semantics |
|---|---|---|
| `instance_id` | prefixed string `SCIN-{uuid_v7}` | stable identity |
| `main_component_id` | `SCMP-*` or null | null only in the deleted-main state of [STU-DS-003a] |
| `component_properties` | map<string, {type, value, bound_variable_id?}> | resolved instance-level property values |
| `overrides` | array of `{overridden_node_id, overridden_fields[]}` | the override record set of [STU-DS-117] |
| `scale_factor` | number | uniform scale applied to the instance render; see [STU-DS-106] |
| `exposed_instance_ids` | array of `SCIN-*` | read-only projection of nested instances flagged `is_exposed_instance` |
| `is_exposed_instance` | boolean | whether THIS instance's properties surface on its containing instance's inspector |

**[STU-DS-106] `scale_factor` parameter contract.**

*Derivation: parameter table taken whole; yields 1 microtask whose acceptance criteria are its seven bound fields, each stored separately with unknown preserved.*
| Field | Value |
|---|---|
| hard_min | 0.01 |
| hard_max | NOT DECLARED IN SOURCE — Studio declares 100.0 and MUST document the choice as Studio-declared, not source-derived |
| soft_min | 0.1 |
| soft_max | 10.0 |
| default | 1.0 |
| unit | ratio (dimensionless multiplier) |
| precision | 4 decimal places |
| step / coarse_step / fine_step | 0.01 / 0.1 / 0.001 |

**[STU-DS-107] Required component commands.** Studio MUST expose at least the following as typed `StudioCommand`s (14.14) over the node kinds of [STU-DS-102]; each is model-invokable, dry-runnable, receipted and undoable per [STU-AUT-002]:

`component.create_from_selection`, `component.create_one_per_top_level_node`, `component.clone`, `component.create_instance`, `component.create_slot`, `component.list_instances` (returns every `component_instance` referencing this definition), `component_set.combine_as_variants`, `component_set.extract_member`, `component_set.set_default_variant`, `instance.swap_component`, `instance.set_properties`, `instance.detach` (returns the resulting plain frame node id), `instance.reset_overrides` (all), `instance.remove_overrides` (selected fields), `instance.get_main_component`, `component.restore_main_from_instance` ([STU-DS-003a]).

`component.list_instances` MUST have both an immediate form scoped to the current document and a deferred form scoped to every enabled library document; the deferred form MUST be declared asynchronous in its command contract because it may need to load documents that are not resident.

---

### 2. Variant sets

**[STU-DS-108] Variant-property schema.** A `component_set`'s `variant_group_properties` maps each variant-property name to an ORDERED list of its declared string values. Order is authority: it drives picker order and the model-facing enumeration order, and reordering is a mutation that MUST emit an EventLedger event.

**[STU-DS-109] Variant assignment completeness and uniqueness.** Every member of a `component_set` MUST carry exactly one value for EVERY property in `variant_group_properties`. Two failures are `StudioValidationDescriptor` errors (14.24), not UI hints:

- `variant_assignment_incomplete` — a member omits a declared property.
- `variant_assignment_duplicate` — two members carry identical assignments across all properties ([STU-DS-007]).

---

### 3. Component properties

**[STU-DS-110] Component-property type enumeration (normative, closed).** The type set is exactly five members. Studio MUST NOT add a sixth without amending this clause:

| Token | Binds to | Instance value type | Options record |
|---|---|---|---|
| `BOOLEAN` | visibility of one or more nested layers | boolean, or a variable alias to a BOOLEAN variable | none |
| `TEXT` | the character content of one or more text layers | string, or a variable alias to a STRING variable | none |
| `INSTANCE_SWAP` | a nested `component_instance` slot | `SCMP-*` component id, or a variable alias | `preferred_values`: array of `{type, key}` where `type` ∈ {`COMPONENT`, `COMPONENT_SET`} |
| `VARIANT` | the containing set's variant selection | string drawn from the property's declared value list | the declared value list |
| `SLOT` | a structural placeholder region | node reference, or empty | none |

**[STU-DS-111] Component-property definition record.** Each entry of `component_property_definitions` MUST carry `property_name` (string), `type` (a token from [STU-DS-110]), `default_value` (string, boolean, or a variable-alias record), and `options` (the per-type record from [STU-DS-110], absent for types that declare none).

**[STU-DS-112] Component-property mutation commands.** Studio MUST expose `component.add_property`, `component.edit_property` (rename and/or change default and/or change options in one call), and `component.delete_property`. Renaming a property MUST rewrite every referencing instance's `component_properties` key in the same transaction; a rename that would orphan an instance value is a validation failure, not a partial write.

**[STU-DS-113] Property references on child layers.** A child layer inside a component definition MUST carry a `component_property_references` map naming which of its own fields are driven by which component property (for example `{"visible": "showIcon#12:3", "characters": "label#4:1"}`). This map is authority: it is what makes a property binding survive a definition edit, and it MUST be preserved across copy, clone and library update.

**[STU-DS-114] Preferred-values fallback.** An `INSTANCE_SWAP` property's `preferred_values` list restricts the default picker set. Studio MUST still offer a search-all fallback across every component visible to the document ([STU-DS-015]). Selecting outside the preferred list is legal and MUST NOT be recorded as a validation error.

---

### 4. The override model

**[STU-DS-115] Overridable field set (normative, closed).** The fields an instance may locally override are exactly:

`characters` (text content), `fills`, `strokes`, `stroke_weight`, `effects`, `visible`, `main_component_id` of a nested instance (nested swap), `component_properties` of a nested exposed instance, `opacity`, `corner_radius` family, and `layout_sizing_horizontal` / `layout_sizing_vertical`. Any other field on a node inside an instance is inherited and MUST NOT be locally writable.

**[STU-DS-116] Override precedence.** Resolution order for any field on a node inside an instance is, highest first: (1) local override record on this instance; (2) value from the instance's own `component_properties` binding via [STU-DS-113]; (3) the main component definition's value; (4) the inherited style or variable binding on the definition. A field MUST resolve through exactly one of these; an ambiguous double-source is a validation error.

**[STU-DS-117] Override record shape.** Each entry of `overrides` MUST carry `overridden_node_id` (the id of the node INSIDE the instance) and `overridden_fields` (array of field names drawn from [STU-DS-115]). Studio MUST NOT store overrides as a flattened copy of the subtree.

**[STU-DS-118] Exposed nested instances.** A nested `component_instance` whose `is_exposed_instance` is true surfaces its own `component_properties` on the containing instance's inspector. `exposed_instance_ids` on the containing instance is a READ-ONLY derived projection of that flag and MUST NOT be written directly. This replaces the v02.205 treatment of "exposed nested" as a property type ([STU-DS-101]).

**[STU-DS-119] Override survival across swap and variant switch.** On `instance.swap_component` and on a variant switch, each override record is re-matched against the new definition by (layer name, position in the child hierarchy). A record that matches is carried; a record that does not match is DROPPED and MUST be reported in the command receipt as `overrides_dropped` with the node ids and field names. Silent reassignment to a different node is forbidden ([STU-DS-018]).

**[STU-DS-120] Reset semantics.** `instance.reset_overrides` clears every record. `instance.remove_overrides` clears only the named `(node_id, field)` pairs. Both are single undoable operations producing one `StudioHistoryEntry`.

---

### 5. Variables and collections

**[STU-DS-121] Variable required fields.** A `StudioVariable` MUST carry:

| Field | Type | Semantics |
|---|---|---|
| `variable_id` | `SVAR-{uuid_v7}` | stable identity |
| `name` | string | slash-path organisation per [STU-DS-005a] |
| `description` | string | |
| `hidden_from_publishing` | boolean | excluded from library publish when true |
| `variable_collection_id` | `SVCL-*` | owning collection |
| `publish_key` | string | cross-document key |
| `is_remote` | boolean | arrived from a subscribed library |
| `resolved_type` | token from [STU-DS-125] | |
| `values_by_mode` | map<mode_id, value> | exactly one entry per mode of the owning collection |
| `scopes` | array of tokens from [STU-DS-128] | picker eligibility |
| `code_syntax` | map<platform, string> | platform ∈ [STU-DS-129] |

**[STU-DS-122] Variable collection required fields.** A `StudioVariableCollection` MUST carry `collection_id` (`SVCL-{uuid_v7}`), `name`, `hidden_from_publishing`, `publish_key`, `is_remote`, `is_extension` (boolean), `modes` (ordered array of `{mode_id, name}`), `default_mode_id`, and `variable_ids`.

**[STU-DS-123] Mode-count contract.** Mode count per collection: hard_min 1; hard_max NOT DECLARED IN SOURCE — Studio declares no fixed ceiling and MUST NOT introduce a plan-gated cap ([STU-DS-023]); soft_min 1; soft_max NOT DECLARED IN SOURCE; default 1 (a collection is created with one mode named `Mode 1`); unit = count; precision = integer. Studio MUST emit a performance advisory (not an error) when a document's total (variables x modes) product exceeds an implementation-declared threshold, and MUST state that threshold in the UserManual.

**[STU-DS-124] Collection extension.** `collection.extend(name)` creates an EXTENDED collection carrying `is_extension = true`, `parent_collection_id`, `root_collection_id`, `variable_ids` (inherited), and `variable_overrides` (map from inherited variable id to per-mode override values). An extended collection MUST NOT add modes of its own; `mode.add` is unavailable on it and `mode.remove` operates only on its own extended mode ids. `collection.remove_overrides_for_variable(variable_id)` clears one variable's overrides. This is the multi-brand token mechanism of [STU-DS-027].

**[STU-DS-125] Resolved variable type enumeration (normative, closed).** Exactly six members: `BOOLEAN`, `COLOR`, `EASING`, `FLOAT`, `STRING`, `TIMING`. `EASING` and `TIMING` exist so animation timing values are first-class tokens ([STU-DS-028]); they are consumed by the prototyping surface (14.11) and by the motion surface.

**[STU-DS-126] Stored variable data-type enumeration (normative, closed).** A stored variable VALUE may additionally be an alias or a computed expression. Exactly eight members: `BOOLEAN`, `COLOR`, `EASING`, `EXPRESSION`, `FLOAT`, `STRING`, `TIMING`, `VARIABLE_ALIAS`. `EXPRESSION` and `VARIABLE_ALIAS` are storage forms only; both MUST resolve to one of the six members of [STU-DS-125] at read time.

**[STU-DS-127] Variable value record.** A stored value MUST be `{type, resolved_type, value}` where `type` is from [STU-DS-126], `resolved_type` from [STU-DS-125], and `value` is the literal, an alias record `{type: "VARIABLE_ALIAS", id}`, or an expression record `{expression_function, expression_arguments[]}` ([STU-DS-131]).

**[STU-DS-128] Variable scope enumeration (normative, closed).** Exactly twenty-two members: `ALL_SCOPES`, `ALL_FILLS`, `FRAME_FILL`, `SHAPE_FILL`, `TEXT_FILL`, `STROKE_COLOR`, `STROKE_FLOAT`, `EFFECT_COLOR`, `EFFECT_FLOAT`, `CORNER_RADIUS`, `GAP`, `WIDTH_HEIGHT`, `OPACITY`, `FONT_FAMILY`, `FONT_SIZE`, `FONT_STYLE`, `FONT_WEIGHT`, `LETTER_SPACING`, `LINE_HEIGHT`, `PARAGRAPH_INDENT`, `PARAGRAPH_SPACING`, `TEXT_CONTENT`. A variable whose `scopes` array contains `ALL_SCOPES` is offered everywhere its `resolved_type` is legal. Scope MUST be enforced identically in the operator picker and in the model command surface ([STU-DS-025]).

**[STU-DS-129] Code-syntax platform enumeration (normative, closed).** Studio MUST accept exactly three members and no others: `WEB`, `ANDROID`, `iOS`. A fourth platform token MUST be refused with a typed error rather than stored. `code_syntax` values are consumed by the codegen surface (14.14) in preference to the raw variable name ([STU-DS-027]).

**[STU-DS-130] Bindable-field enumerations (normative, closed).** A variable may be bound only to a field named in one of these four closed sets. Binding elsewhere is a typed error, not a silent no-op.

- **Node fields (27):** `width`, `height`, `min_width`, `max_width`, `min_height`, `max_height`, `opacity`, `visible`, `characters`, `corner_radius`, `top_left_radius`, `top_right_radius`, `bottom_left_radius`, `bottom_right_radius`, `item_spacing`, `counter_axis_spacing`, `grid_row_gap`, `grid_column_gap`, `padding_top`, `padding_right`, `padding_bottom`, `padding_left`, `stroke_weight`, `stroke_top_weight`, `stroke_right_weight`, `stroke_bottom_weight`, `stroke_left_weight`.
- **Text fields (8):** `font_family`, `font_size`, `font_style`, `font_weight`, `letter_spacing`, `line_height`, `paragraph_indent`, `paragraph_spacing`.
- **Effect fields (5):** `color`, `offset_x`, `offset_y`, `radius`, `spread`.
- **Layout-grid fields (4):** `count`, `gutter_size`, `offset`, `section_size`.

**[STU-DS-131] Expression function enumeration (normative, closed).** Exactly fifteen members, used by variable expressions and by prototype conditionals ([STU-PRO-126]): `ADDITION`, `SUBTRACTION`, `MULTIPLICATION`, `DIVISION`, `NEGATE`, `EQUALS`, `NOT_EQUAL`, `GREATER_THAN`, `GREATER_THAN_OR_EQUAL`, `LESS_THAN`, `LESS_THAN_OR_EQUAL`, `AND`, `OR`, `NOT`, `VAR_MODE_LOOKUP`. Each takes an ordered `expression_arguments` array of [STU-DS-127] value records. Evaluation MUST be deterministic and side-effect-free; alias and expression cycles are validation errors ([STU-DS-026]).

**[STU-DS-132] Explicit mode assignment.** Any container `StudioLayer` and any page MUST carry `explicit_variable_modes`: a map from `collection_id` to `mode_id`. Absence of an entry means "inherit from ancestor"; the document root falls back to each collection's `default_mode_id`. Studio MUST expose `node.set_explicit_variable_mode_for_collection` and `node.clear_explicit_variable_mode_for_collection` as typed commands. Setting or clearing re-resolves every bound property in the subtree in one transaction ([STU-DS-024]).

**[STU-DS-133] Variable API surface.** Studio MUST expose, as typed commands: `variable.create` (by collection id or collection reference), `variable.remove`, `variable.set_value_for_mode`, `variable.resolve_for_consumer` (resolve a variable as seen by a given node, honouring [STU-DS-132]), `variable.values_by_mode_for_collection` (for extended collections), `variable.remove_override_for_mode`, `variable.set_code_syntax`, `variable.remove_code_syntax`, `variable.create_alias`, `variable_collection.create`, `variable_collection.extend`, `variable_collection.add_mode`, `variable_collection.rename_mode`, `variable_collection.remove_mode`, `variable_collection.remove`, plus binding commands `bind_variable_for_paint`, `bind_variable_for_effect`, `bind_variable_for_layout_grid` and the generic `bind_variable_for_node_field`. Each read command MUST exist in both a resident form and a library-loading asynchronous form.

---

### 6. Style registry

**[STU-DS-134] Style kind enumeration (normative, closed).** Studio MUST accept exactly four members and no others: `PAINT`, `TEXT`, `EFFECT`, `GRID`. A fifth style kind MUST NOT be introduced without amending this clause. These map one-to-one onto the four rows of [STU-DS-029].

**[STU-DS-135] Style record required fields.** Every `StudioStyleRegistry` entry MUST carry `style_id` (`SSTY-{uuid_v7}`), `kind` (from [STU-DS-134]), `name` (slash-path), `description`, `description_markdown`, `documentation_links`, `publish_key`, `is_remote`, and the kind-specific payload.

**[STU-DS-136] Inherited style-field enumeration (normative, closed).** The fields on a `StudioLayer` that hold a style reference are exactly six: `fill_style_id`, `stroke_style_id`, `text_style_id`, `effect_style_id`, `grid_style_id`, `background_style_id`. No other field may hold a style reference.

**[STU-DS-137] Style-mutation change surface (normative, closed).** A style-change event MUST name at least one of these twenty-one changed properties, and no others: `name`, `description`, `documentation_links`, `remote`, `type`, `paint`, `effects`, `layout_grids`, `font_size`, `letter_spacing`, `line_height`, `leading_trim`, `list_spacing`, `hanging_list`, `hanging_punctuation`, `paragraph_indent`, `paragraph_spacing`, `text_case`, `text_decoration`, `text_wrap_style`, `plugin_data`. This list is the authority for style-diff receipts and for library update review ([STU-DS-019b]).

**[STU-DS-138] Style verbs.** The four uniform verbs of [STU-DS-034a] — apply, merge, break-link, redefine-from-selection — MUST each be a typed command, and `redefine_from_selection` MUST return in its receipt the count and ids of every node re-rendered by the redefinition.

**[STU-DS-139] Shipped style-library sizing contract.** Studio's shipped graphic-style and symbol libraries are sized to the field baseline recovered from the source corpus: 314 graphic styles across 12 library files, 884 symbols across 28 library files, 561 brushes across 25 library files, and 14,207 swatch-family entries across 118 library files (3,155 swatches, 10,011 named colours, 659 gradients, 382 patterns). These are ENTRY counts, not file counts. Studio's own shipped libraries need not match those numbers, but the loader, the picker, the search index and the paging behaviour MUST be specified and tested against a library of at least that size, and the UserManual MUST state the tested ceiling.

---

### 7. Publishing and libraries

**[STU-DS-140] Publish status enumeration (normative, closed).** Exactly three members: `UNPUBLISHED`, `CURRENT`, `CHANGED`. Every publishable entity — `component`, `component_set`, style, variable, variable collection — MUST expose an asynchronous `get_publish_status` command returning one of these three.

**[STU-DS-141] Publishable-entity common fields.** Every publishable entity MUST carry the same five fields: `description`, `description_markdown`, `documentation_links`, `is_remote`, `publish_key`. This is one contract across components, styles and variables; there MUST NOT be per-kind divergence ([STU-DS-004]).

**[STU-DS-142] Detached-source enumeration (normative, closed).** When an instance is detached, the receipt MUST record the detach source as exactly one of `local` (main definition was in this document) or `library` (main definition came from a subscribed library). This distinction drives the drift report of [STU-AUT-124].

**[STU-DS-143] Library update transaction.** Applying a library update is a single transaction over the consuming document: every affected definition is replaced, every instance re-resolved, and every dropped override reported per [STU-DS-119], or nothing changes. A partial apply is forbidden. Model-authored library operations pass the sandbox -> validation -> `PromotionGate` lifecycle ([STU-DS-021]).

---

### 8. Auto layout — full parameter contract

**[STU-DS-144] Auto-layout container fields (normative, closed set of seventeen).** A frame with auto layout MUST expose exactly these container fields:

| Field | Type / domain | Default | Notes |
|---|---|---|---|
| `layout_mode` | `NONE` \| `HORIZONTAL` \| `VERTICAL` \| `GRID` | `NONE` | `NONE` means the frame is not an auto-layout container |
| `layout_wrap` | `NO_WRAP` \| `WRAP` | `NO_WRAP` | legal only when `layout_mode = HORIZONTAL` |
| `primary_axis_sizing_mode` | `FIXED` \| `AUTO` | `AUTO` | `AUTO` = hug contents on the flow axis |
| `counter_axis_sizing_mode` | `FIXED` \| `AUTO` | `AUTO` | `AUTO` = hug contents on the cross axis |
| `primary_axis_align_items` | `MIN` \| `CENTER` \| `MAX` \| `SPACE_BETWEEN` | `MIN` | packing/distribution on the flow axis |
| `counter_axis_align_items` | `MIN` \| `CENTER` \| `MAX` \| `BASELINE` | `MIN` | `BASELINE` is the text-baseline alignment of [STU-DS-041] |
| `counter_axis_align_content` | `AUTO` \| `SPACE_BETWEEN` | `AUTO` | applies only when `layout_wrap = WRAP` |
| `item_spacing` | number, see [STU-DS-145] | 0 | gap along the flow axis |
| `counter_axis_spacing` | number or null, see [STU-DS-145] | null | gap between wrapped lines; null = use `item_spacing` |
| `padding_top` | number, see [STU-DS-145] | 0 | |
| `padding_right` | number, see [STU-DS-145] | 0 | |
| `padding_bottom` | number, see [STU-DS-145] | 0 | |
| `padding_left` | number, see [STU-DS-145] | 0 | |
| `horizontal_padding` | number, see [STU-DS-145] | 0 | convenience writer that sets `padding_left` and `padding_right` together; reading it when the two differ returns the left value and MUST set a `mixed` flag on the read receipt |
| `vertical_padding` | number, see [STU-DS-145] | 0 | convenience writer for top and bottom, same mixed rule |
| `strokes_included_in_layout` | boolean | false | when true, child stroke weight counts in spacing calculations ([STU-DS-045]) |
| `item_reverse_z_index` | boolean | false | when true, overlapping children render last-on-top; this is the canvas-stacking setting of [STU-DS-039] |

`SPACE_BETWEEN` on `primary_axis_align_items` is the "auto" gap of [STU-DS-038]; there is no separate `auto` token on `item_spacing`.

**[STU-DS-145] Spacing and padding parameter contract.** `item_spacing`, `counter_axis_spacing`, `grid_row_gap`, `grid_column_gap` and the four padding fields share this contract:

| Field | `item_spacing` / `counter_axis_spacing` | padding fields | `grid_row_gap` / `grid_column_gap` |
|---|---|---|---|
| hard_min | NOT DECLARED IN SOURCE; Studio declares -10000 (negative gaps are legal per [STU-DS-038]) | 0 | 0 |
| hard_max | NOT DECLARED IN SOURCE; Studio declares 100000 | NOT DECLARED IN SOURCE; Studio declares 100000 | NOT DECLARED IN SOURCE; Studio declares 100000 |
| soft_min | -100 | 0 | 0 |
| soft_max | 200 | 200 | 200 |
| default | 0 | 0 | 0 |
| unit | document unit ([STU-DOC-003]) | document unit | document unit |
| precision | 2 decimal places | 2 decimal places | 2 decimal places |
| step / coarse_step / fine_step | 1 / 10 / 0.1 | 1 / 10 / 0.1 | 1 / 10 / 0.1 |

Every Studio-declared bound in this table MUST be labelled `studio_declared` in the machine contract so a later capture can replace it without ambiguity.

**[STU-DS-146] Grid flow contract (normative).** When `layout_mode = GRID`, the container additionally exposes exactly these eight fields plus three commands:

| Field | Type / domain | Default |
|---|---|---|
| `grid_row_count` | integer, hard_min 1, hard_max NOT DECLARED IN SOURCE (Studio declares 1000), soft_max 24, precision integer | 1 |
| `grid_column_count` | integer, same contract as `grid_row_count` | 1 |
| `grid_row_gap` | number, see [STU-DS-145] | 0 |
| `grid_column_gap` | number, see [STU-DS-145] | 0 |
| `grid_row_sizes` | ordered array of track-size records ([STU-DS-147]) | one `HUG` track |
| `grid_column_sizes` | ordered array of track-size records | one `HUG` track |
| `grid_auto_tracks` | `NONE` \| `ROWS` | `NONE` |
| `grid_items_positioning` | `MANUAL` \| `ROW_AUTO_FLOW` | `ROW_AUTO_FLOW` |

Commands: `grid.append_child_at(node, row_index, column_index)`, `grid.reorder_rows(options)`, `grid.reorder_columns(options)`. Both reorder commands MUST return the resulting ordered track entries so a model can verify the move without a second read.

**[STU-DS-147] Grid track-size record.** `{type, value?}` where `type` ∈ {`FIXED`, `FLEX`, `HUG`}. `value` is required for `FIXED` (document unit, precision 2) and for `FLEX` (dimensionless flex factor, hard_min 0, default 1, precision 4); it MUST be absent for `HUG`.

**[STU-DS-148] Auto-layout child fields (normative, closed set of three, plus two sizing fields).** A child participating in auto layout carries:

| Field | Type / domain | Default | Semantics |
|---|---|---|---|
| `layout_align` | `MIN` \| `CENTER` \| `MAX` \| `STRETCH` \| `INHERIT` | `INHERIT` | cross-axis alignment override for this child |
| `layout_grow` | number, hard_min 0, hard_max 1, soft range 0..1, default 0, precision 0 (integer 0 or 1 in practice), unit dimensionless | 0 | 1 = grow to fill remaining primary-axis space |
| `layout_positioning` | `AUTO` \| `ABSOLUTE` | `AUTO` | `ABSOLUTE` is the ignore-auto-layout escape of [STU-DS-044]; an `ABSOLUTE` child falls back to `StudioConstraint` ([STU-DS-048]) |
| `layout_sizing_horizontal` | `FIXED` \| `HUG` \| `FILL` | `FIXED` | the per-axis resizing mode of [STU-DS-042] |
| `layout_sizing_vertical` | `FIXED` \| `HUG` \| `FILL` | `FIXED` | |

**[STU-DS-149] Sizing clamps.** `min_width`, `max_width`, `min_height`, `max_height` bound hug and fill resizing ([STU-DS-043]). Contract: hard_min 0; hard_max NOT DECLARED IN SOURCE (Studio declares 100000); soft_min 0; soft_max 10000; default null (unset, meaning unclamped); unit document unit; precision 2 decimal places. A `min` greater than the corresponding `max` is a validation error, not a silent clamp.

**[STU-DS-150] Grid-child placement fields.** A child of a `GRID` container additionally carries `grid_row_anchor_index`, `grid_column_anchor_index`, `grid_row_span`, `grid_column_span` (all integers, hard_min 0 for anchors and 1 for spans, hard_max bounded by the container's declared counts, precision integer), plus a read-only `grid_child_index`. Span beyond the declared track count is a validation error.

---

### 9. Constraints

**[STU-DS-151] Constraint type enumeration (normative, closed).** Exactly five members on BOTH axes: `MIN`, `CENTER`, `MAX`, `STRETCH`, `SCALE`. Their meaning per axis:

| Token | Horizontal meaning | Vertical meaning |
|---|---|---|
| `MIN` | pin to left | pin to top |
| `MAX` | pin to right | pin to bottom |
| `CENTER` | keep centred, preserve size | keep centred, preserve size |
| `STRETCH` | pin both edges, resize width | pin both edges, resize height |
| `SCALE` | store x and width as a percentage of parent | store y and height as a percentage of parent |

This supersedes the differently-worded lists in [STU-DS-046a] and [STU-DS-046b].

**[STU-DS-152] Constraint record and default.** A non-auto-layout child MUST carry `constraints = {horizontal, vertical}` with both members drawn from [STU-DS-151]. The default is `{horizontal: MIN, vertical: MIN}` (top-left, per [STU-DS-046b]).

---

### 10. Layout grids

**[STU-DS-153] Layout-grid pattern enumeration (normative, closed).** Exactly three members: `GRID` (uniform square), `ROWS`, `COLUMNS`.

**[STU-DS-154] Uniform grid record.** `{pattern: "GRID", section_size, visible, color}`. `section_size` is the square cell size: hard_min 1; hard_max NOT DECLARED IN SOURCE (Studio declares 10000); soft_min 1; soft_max 100; default 8; unit document unit; precision 2. `visible` default true. `color` is an RGBA value carrying an explicit `StudioColorProfile` reference ([STU-DOC-003]); alpha carries the opacity of [STU-DS-047a] and there is no separate opacity field.

**[STU-DS-155] Rows/columns grid record.** `{pattern, alignment, gutter_size, count, section_size?, offset?, visible, color}` where `alignment` ∈ {`MIN`, `CENTER`, `MAX`, `STRETCH`}. `MIN`/`CENTER`/`MAX` are the fixed-size modes (left/center/right for columns, top/center/bottom for rows) and require `section_size` and `offset`; `STRETCH` is the margin-and-gutter mode and requires `offset` (the margin) and ignores `section_size` ([STU-DS-047b]). Contracts: `count` integer hard_min 1, hard_max NOT DECLARED IN SOURCE (Studio declares 1000), soft_max 24, default 12; `gutter_size` and `offset` and `section_size` follow the spacing contract of [STU-DS-145] with default `gutter_size` 20 and default `offset` 0.

**[STU-DS-156] Stacked overlays and variable binding.** A frame MUST accept an ordered array of layout-grid records that render simultaneously and toggle individually ([STU-DS-047c]). Each record's `count`, `gutter_size`, `offset` and `section_size` are variable-bindable per [STU-DS-130].

---

### 11. Annotation of design-system state

**[STU-DS-157] Annotation property enumeration (normative, closed).** A design annotation attached to a node may pin any subset of exactly these thirty-three property names, and no others: `alignItems`, `cornerRadius`, `effects`, `fills`, `fontFamily`, `fontSize`, `fontStyle`, `fontWeight`, `gridColumnAnchorIndex`, `gridColumnCount`, `gridColumnGap`, `gridColumnSpan`, `gridRowAnchorIndex`, `gridRowCount`, `gridRowGap`, `gridRowSpan`, `height`, `itemSpacing`, `layoutMode`, `letterSpacing`, `lineHeight`, `mainComponent`, `maxHeight`, `maxWidth`, `minHeight`, `minWidth`, `opacity`, `padding`, `strokeWeight`, `strokes`, `textAlignHorizontal`, `textStyleId`, `width`. A pinned property renders its LIVE value; the annotation is not a copied string.

**[STU-DS-158] Annotation record and category colours.** An annotation record is `{label, label_markdown, properties[], category_id}`. Annotation category colour is drawn from a closed eight-member set: `blue`, `green`, `orange`, `pink`, `red`, `teal`, `violet`, `yellow`.

**[STU-DS-159] Dev-status enumeration (normative, closed).** Exactly two members: `READY_FOR_DEV`, `COMPLETED`. Dev status is carried on section-kind and component-kind nodes and is the "ready for implementation" state referenced by [STU-AUT-024].

---

### 12. Panel, asset, and obligation bindings

**[STU-DS-160] Contextual panel binding.** Every design-system inspector panel named in this sub-section — the component editor, the variant-set editor, the component-property inspector, the instance inspector, the variable and collection and mode editor, the style registry, and the auto-layout / constraint / layout-grid controls — MUST declare its binding through the contextual property-panel contract ([STU-WEB-030] through [STU-WEB-044]) rather than through a hard-coded panel switch. Studio MUST NOT introduce a whole-UI persona or mode toggle to select which design-system panel is shown.

**[STU-DS-161] Asset library binding.** Any design-system entity that references external bytes — a placed image inside a component definition, a raster fill, an embedded font, a documentation attachment — MUST reference it as a CKC placed-asset link per [STU-ASSET-005] and MUST NOT copy the bytes into the design-system record. The design-system library ([STU-DS-019]) publishes DEFINITIONS; CKC owns ASSETS. The boundary is stated normatively in [STU-ASSET-011].

**[STU-DS-162] Command-surface obligation.** Every command named in [STU-DS-107], [STU-DS-112] and [STU-DS-133] MUST satisfy [STU-CON-007] in full: model-invokable through the one typed command contract (14.14), parallel-safe (two model lanes editing different components in one document MUST NOT corrupt either), deterministic (identical inputs on identical document state produce byte-identical output state), and visually verifiable (a rendered before/after of the affected subtree is obtainable through the Argus surface without foreground focus steal).

**[STU-DS-163] Parallel-safety scope.** Concurrent design-system mutations MUST use the expected-revision precondition of [STU-SDB-004] at the granularity of the individual `component`, `component_set`, `variable`, `variable_collection` or style record, NOT at document granularity. Two lanes editing two different components in one document MUST both succeed; two lanes editing the same component MUST produce exactly one success and one typed conflict.

**[STU-DS-164] Determinism of resolution.** Variable resolution ([STU-DS-132]), override resolution ([STU-DS-116]), variant selection ([STU-DS-109]) and alias chain walking ([STU-DS-131]) MUST each be a pure function of the document state and the requested mode. No resolution step may depend on iteration order of an unordered map, on wall-clock time, on locale, or on which client observed it first.

**[STU-DS-165] Validation descriptor set.** This sub-section contributes at minimum these `StudioValidationDescriptor` checks (14.24): `variant_assignment_incomplete`, `variant_assignment_duplicate`, `variable_alias_cycle`, `variable_expression_cycle`, `variable_scope_violation`, `variable_bindable_field_violation`, `variable_mode_value_missing`, `component_property_orphan_reference`, `component_property_rename_orphans_instance`, `override_field_not_overridable`, `override_double_source`, `sizing_clamp_inverted`, `grid_span_exceeds_tracks`, `style_reference_field_violation`, `library_update_would_break_instance`.

**[STU-DS-166] Analytics report contract.** The local design-system analytics report of [STU-DS-050] MUST emit, per entity, at minimum: `entity_id`, `entity_kind`, `insertion_count`, `instance_count`, `detach_count` split by the two members of [STU-DS-142], `orphaned_instance_count` (instances whose `main_component_id` resolves to nothing), and `override_rate` (fraction of instances carrying at least one override record). It is a locally computed projection over authority rows; no cloud aggregation is a prerequisite.

**[STU-DS-167] GUI / Argus / UserManual obligation.** [STU-DS-051] remains in force unchanged and additionally covers every field, enumeration and command introduced by [STU-DS-100] through [STU-DS-166]. Each enumeration in this sub-section MUST appear in the model-facing UserManual as its literal token list, not as prose.

---

### 13. Microtask Derivation

**[STU-DS-168] Derivation rule (NORMATIVE).** The design-system microtask set is derived from this module mechanically, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. Each numbered clause that states a **stored record contract** ([STU-DS-103], [STU-DS-104], [STU-DS-105], [STU-DS-111], [STU-DS-117], [STU-DS-121], [STU-DS-122], [STU-DS-127], [STU-DS-135], [STU-DS-147], [STU-DS-152], [STU-DS-154], [STU-DS-155], [STU-DS-158]), a **closed enumeration** ([STU-DS-110], [STU-DS-115], [STU-DS-125], [STU-DS-126], [STU-DS-128], [STU-DS-129], [STU-DS-130], [STU-DS-131], [STU-DS-134], [STU-DS-136], [STU-DS-137], [STU-DS-140], [STU-DS-142], [STU-DS-144], [STU-DS-148], [STU-DS-151], [STU-DS-153], [STU-DS-157], [STU-DS-159]), a **parameter contract table** ([STU-DS-106], [STU-DS-123], [STU-DS-145], [STU-DS-146], [STU-DS-149], [STU-DS-150]), a **resolution algorithm** ([STU-DS-109], [STU-DS-116], [STU-DS-119], [STU-DS-132], [STU-DS-143], [STU-DS-163], [STU-DS-164]), or a **required command set** ([STU-DS-107], [STU-DS-112], [STU-DS-133], [STU-DS-138]), where that clause can be implemented and proven independently of its siblings.
2. Each **validation-descriptor clause** in sub-section 14, [STU-DS-173] through [STU-DS-187]. Each of the 15 descriptors named in [STU-DS-165] is stated as its own clause precisely so it yields its own microtask: a check is a unit of implementable, independently provable work, and one microtask reading "implement 15 checks" is not implementable by the small models these contracts are sized for. A descriptor list inside a single clause, whether as prose or as a table, is one unit to any derivation tool and therefore loses 14 units of real work.

No other unit yields a microtask. Exactly 7 clauses in this module yield nothing, and they are:

- **Baseline, scope-fence and supersession clauses** — [STU-DS-100] and [STU-DS-101], which sit under the bookkeeping heading `0. Baseline, supersession and disposition`. These are discharged when the v02.206 bundle lands, not by a work packet.
- **This derivation sub-section itself** — its five clauses yield nothing.

Every other clause yields at least one unit. This list is the module's declared non-yielding set and is the authority a derivation tool reconciles against.

**[STU-DS-169] Open items and blocked dependencies.** This module declares no open item and no BLOCKED dependency: every clause is derivable today. Should a later amendment introduce one, that clause STILL yields a microtask, and that microtask's FIRST acceptance criterion MUST be resolving the named dependency — reading the named surface, obtaining the named decision, or raising a BLOCKED record with the exact blocker. A declared gap MUST NOT be dropped from the yields index, because a gap that yields nothing disappears silently and is rediscovered at implementation time.

**[STU-DS-170] Microtask content obligation.** A microtask derived under [STU-DS-168] MUST carry into its own body: the clause anchor; the COMPLETE member list of every closed enumeration it touches, as literal tokens; the full seven-field parameter record of every numeric parameter it touches, with `NOT DECLARED IN SOURCE` and every `studio_declared` label preserved verbatim; the record shape with every required field of every record it touches; and the parallel-safety and determinism obligations of [STU-DS-163] and [STU-DS-164] where it touches resolution. A microtask that says "implement variable scoping" without the twenty-two members of [STU-DS-128] does not satisfy this clause.

**[STU-DS-171] Yields index (NORMATIVE).** The counts below are the derivation surface of this module under [STU-DS-168]. They are not estimates: they are the measured output of applying that rule to this module's text, and every row states which unit kinds it contributes.

| Unit group | Clauses | Units by kind | Yields |
|---|---|---|---|
| The component object contract | [STU-DS-102]-[STU-DS-107] | 6 clause, 1 parameter table | 7 |
| Variant sets | [STU-DS-108]-[STU-DS-109] | 2 clause | 2 |
| Component properties | [STU-DS-110]-[STU-DS-114] | 5 clause, 1 enumeration | 6 |
| The override model | [STU-DS-115]-[STU-DS-120] | 6 clause | 6 |
| Variables and collections | [STU-DS-121]-[STU-DS-133] | 13 clause | 13 |
| Style registry | [STU-DS-134]-[STU-DS-139] | 6 clause | 6 |
| Publishing and libraries | [STU-DS-140]-[STU-DS-143] | 4 clause | 4 |
| Auto layout — full parameter contract | [STU-DS-144]-[STU-DS-150] | 7 clause | 7 |
| Constraints | [STU-DS-151]-[STU-DS-152] | 2 clause | 2 |
| Layout grids | [STU-DS-153]-[STU-DS-156] | 4 clause | 4 |
| Annotation of design-system state | [STU-DS-157]-[STU-DS-159] | 3 clause | 3 |
| Panel, asset, and obligation bindings | [STU-DS-160]-[STU-DS-167] | 8 clause | 8 |
| Validation Descriptor Catalogue | [STU-DS-173]-[STU-DS-187] | 15 validator | 15 |
| Clauses yielding nothing | 7 clauses, listed in [STU-DS-168] | — | 0 |
| **Module total** | | **88 clauses** | **83** |

Of this module's 88 clauses, 7 yield nothing and 81 yield at least one unit; tables inside yielding clauses contribute the remainder. The module total is **83**. The last numeric column is the yields count.

**[STU-DS-172] Anchor binding.** A microtask derived from this module cites its clause anchor directly. A microtask staged before this module landed carries `spec_anchor_status = "PROVISIONAL"`; binding it to an anchor in [STU-DS-100]–[STU-DS-187] clears that status. A microtask that cannot cite an anchor in this module is out of scope for the design-system domain and MUST be re-derived or retired, not activated.

---

### 14. Validation Descriptor Catalogue

Each descriptor below is its own clause because each is its own unit of implementable, independently provable work: feed the runtime a document that violates the rule and assert the check fires with the stated diagnostic. [STU-DS-165] names the set; the clauses in this sub-section state what each member catches, which clause it enforces, its severity, and what its diagnostic MUST name. Every one is a `StudioValidationDescriptor` in the catalogue of 14.24.

**[STU-DS-173] `variant_assignment_incomplete`.** The design-system validator MUST reject, with severity `error`, a document or command in which a member of a `component_set` carries no value for a property that the set's `variant_group_properties` declares, enforcing [STU-DS-109]. The diagnostic MUST name the member's `component_id` and every omitted property name.

**[STU-DS-174] `variant_assignment_duplicate`.** The design-system validator MUST reject, with severity `error`, a document or command in which two members of one `component_set` carry identical assignments across every declared variant property, so no assignment can select between them, enforcing [STU-DS-109]. The diagnostic MUST name both members and the colliding property-and-value assignment.

**[STU-DS-175] `variable_alias_cycle`.** The design-system validator MUST reject, with severity `error`, a document or command in which an alias chain returns to a variable already on the chain, so resolution would not terminate, enforcing [STU-DS-131]. The diagnostic MUST name every variable on the cycle, in resolution order.

**[STU-DS-176] `variable_expression_cycle`.** The design-system validator MUST reject, with severity `error`, a document or command in which an expression argument resolves, directly or transitively, to the variable the expression computes, enforcing [STU-DS-131]. The diagnostic MUST name the expression, its owning variable, and the cycle path.

**[STU-DS-177] `variable_scope_violation`.** The design-system validator MUST reject, with severity `error`, a document or command in which a variable is offered on, or bound to, a property that its `scopes` array does not admit, enforcing [STU-DS-128]. The diagnostic MUST name the variable, its declared scopes, and the offending property; the check MUST run identically in the operator picker and the model command surface.

**[STU-DS-178] `variable_bindable_field_violation`.** The design-system validator MUST reject, with severity `error`, a document or command in which a binding targets a field outside the four closed bindable-field sets, enforcing [STU-DS-130]. The diagnostic MUST name the field name and which of the four sets was expected.

**[STU-DS-179] `variable_mode_value_missing`.** The design-system validator MUST reject, with severity `error`, a document or command in which a variable's `values_by_mode` omits an entry for a mode its owning collection declares, enforcing [STU-DS-121]. The diagnostic MUST name the variable and the missing `mode_id`.

**[STU-DS-180] `component_property_orphan_reference`.** The design-system validator MUST reject, with severity `error`, a document or command in which a child layer's `component_property_references` names a property the owning definition does not declare, enforcing [STU-DS-113]. The diagnostic MUST name the layer, the referenced property name, and the owning component.

**[STU-DS-181] `component_property_rename_orphans_instance`.** The design-system validator MUST reject, with severity `error`, a document or command in which a property rename would leave an instance's `component_properties` key unrewritten, enforcing [STU-DS-112]. The diagnostic MUST name every affected instance; the rename MUST fail closed rather than write partially.

**[STU-DS-182] `override_field_not_overridable`.** The design-system validator MUST reject, with severity `error`, a document or command in which an override record names a field outside the closed overridable set, enforcing [STU-DS-115]. The diagnostic MUST name the field and the node it was recorded against.

**[STU-DS-183] `override_double_source`.** The design-system validator MUST reject, with severity `error`, a document or command in which a field on a node inside an instance resolves through more than one of the four precedence sources, enforcing [STU-DS-116]. The diagnostic MUST name the field and every competing source.

**[STU-DS-184] `sizing_clamp_inverted`.** The design-system validator MUST reject, with severity `error`, a document or command in which a `min_width`, `min_height`, `max_width` or `max_height` clamp is inverted so the minimum exceeds the maximum, enforcing [STU-DS-149]. The diagnostic MUST name both clamps and their values; the pair MUST NOT be silently reordered or clamped.

**[STU-DS-185] `grid_span_exceeds_tracks`.** The design-system validator MUST reject, with severity `error`, a document or command in which a grid child's row or column span reaches beyond the container's declared track count, enforcing [STU-DS-150]. The diagnostic MUST name the child, its span, and the container's declared count.

**[STU-DS-186] `style_reference_field_violation`.** The design-system validator MUST reject, with severity `error`, a document or command in which a style reference is stored on a field outside the closed set of six style-bearing fields, enforcing [STU-DS-136]. The diagnostic MUST name the field and the style id it holds.

**[STU-DS-187] `library_update_would_break_instance`.** The design-system validator MUST reject, with severity `error`, a document or command in which applying a library update would leave one or more instances unresolvable, enforcing [STU-DS-143]. The diagnostic MUST name every instance that would break; the whole update transaction is refused, never partially applied.
