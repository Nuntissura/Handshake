---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-14"
section_id: "14.14"
title: "14.14 Automation, Scripting & Plugin/API Surface"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
supersedes_section: "14.14 in .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md lines 2889-3077"
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-14-automation.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.14 Automation, Scripting & Plugin/API Surface

This sub-section is the normative automation surface for Studio at professional depth: the recorded-action and macro model with its per-step parameter contract, the batch runner, the typed scripting object model, the model-steerable command surface required by [STU-CON-007], the find/change surface, the plugin contract, and the inspect/codegen projection.

### 0. Baseline, supersession and disposition

**[STU-AUT-100] Baseline preservation and supersession.** Clauses [STU-AUT-001] through [STU-AUT-027] of v02.205 remain in force as the architectural frame of this domain: one typed command contract, the four admissibility properties, the promotion lifecycle, command batches, capability namespacing, the object-model exposure rule, the descriptor path, stable-id addressing, the modal-gate rule, and the plugin and inspect postures. Clauses [STU-AUT-100] and above add the record shapes, enumerations, bounds and scale contracts those clauses assumed. Explicit corrections:

| v02.205 clause | Disposition | Replacement |
|---|---|---|
| [STU-AUT-010] "a recorded macro is a persisted `StudioCommandBatch`" | CORRECTED | A recorded action is a persisted ACTION record with a step list ([STU-AUT-103]); a `StudioCommandBatch` is the transactional envelope it replays INSIDE. Conflating the two loses per-step enable/disable, per-step dialog policy, and per-step re-parameterisation. |
| [STU-AUT-011] macro control table | EXTENDED | Each control row is now a field on the action or step record ([STU-AUT-103], [STU-AUT-104]) with a declared domain. |
| [STU-AUT-025] native asset browser | SUPERSEDED IN OWNERSHIP | See [STU-ASSET-017]: the capability is preserved, the owner is CKC, and Studio holds no catalog state. |
| [STU-AUT-006] "the scripting DOM equivalent" | EXTENDED | The object model's shape, its type system and its scale contract are stated in [STU-AUT-115] through [STU-AUT-124]. |

---

### 1. Actions and macros — the recorded-automation model

**[STU-AUT-101] One recorded-automation primitive.** Studio has exactly one recorded-automation primitive, `StudioAction`, organised into `StudioActionSet` containers. The v02.205 term "macro" is an alias for `StudioAction` and MUST NOT become a second primitive with different semantics.

**[STU-AUT-102] Action set record (normative shape).** `{action_set_id, name, actions[]}` where `actions` is ORDERED. Action sets are the unit of import, export, enable and disable. A set is portable: exporting one produces a self-contained artifact that another Studio instance can import without carrying document state.

**[STU-AUT-103] Action record (normative shape and field domains).**

| Field | Type / domain | Default | Semantics |
|---|---|---|---|
| `action_id` | `SACT-{uuid_v7}` | — | stable identity |
| `index` | integer >= 0 | — | position in the owning set; order is authority |
| `name` | string | — | display name; MUST be localisable without changing `action_id` |
| `function_key_index` | integer 0–12 | 0 | 0 = no function-key binding; 1–12 = F1–F12 |
| `shift_modifier` | boolean | false | shift required with the function key |
| `command_modifier` | boolean | false | platform command/control modifier required |
| `color_index` | integer 0–7 | 0 | display colour band in the action panel; 0 = none |
| `expanded` | boolean | false | UI disclosure state, persisted with the action |
| `declared_step_count` | integer >= 0 | — | the step count the record claims; a mismatch with `steps.length` is a load error, not a warning |
| `steps` | ordered array of [STU-AUT-104] | — | the recorded step list |

The 0–12 function-key domain and the 0–7 colour domain are the shipped domains observed across 149 recorded actions in 11 sets; they are declared here as closed. A binding collision between two actions in enabled sets is a registration error naming both.

**[STU-AUT-104] Action step record (normative shape and field domains).**

| Field | Type / domain | Default | Semantics |
|---|---|---|---|
| `index` | integer >= 0 | — | position within the action; order is authority |
| `enabled` | boolean | true | a disabled step is skipped on playback but preserved in the record ([STU-AUT-011] step enable/exclude) |
| `expanded` | boolean | false | UI disclosure state |
| `with_dialog` | boolean | false | when true, playback pauses and surfaces this step's parameter dialog for operator input |
| `dialog_options` | integer 0–3 ([STU-AUT-105]) | 0 | the dialog policy for this step |
| `command_id` | stable string | — | the `StudioCommand` this step invokes; the recorded-command identity |
| `display_name` | string | — | the human label captured at record time |
| `has_parameters` | boolean | — | whether a parameter descriptor follows |
| `parameter_class` | stable string or null | null | the descriptor class of the parameter block |
| `parameters` | map<string, typed value ([STU-AUT-106])> or null | null | the recorded parameter set |

**[STU-AUT-105] Dialog-option enumeration (normative, closed, four members).** `dialog_options` is exactly one of:

| Value | Token | Meaning |
|---|---|---|
| 0 | `silent` | never show a dialog for this step, even in interactive playback |
| 1 | `show_if_configured` | show the dialog only when the step's `with_dialog` flag is set |
| 2 | `show_in_interactive_playback` | show the dialog in interactive playback, suppress it in batch |
| 3 | `always_show` | always show the dialog, including inside a batch job |

The shipped distribution across 1,543 recorded steps is 901 `silent`, 630 `show_in_interactive_playback`, 10 `always_show` and 2 `show_if_configured`; 119 of those steps additionally carry `with_dialog = true`. In a headless batch run under [STU-AUT-112], `always_show` is the ONLY value that can block; a batch job MUST refuse to start when any enabled step in its action carries `always_show`, and MUST name the step, rather than hanging on a dialog no one can answer.

**[STU-AUT-106] Recorded parameter value types (normative, closed, seven members).** A recorded parameter value is exactly one of:

| Type | Wire form | Example semantic |
|---|---|---|
| `reference` | an object reference resolving to a document object by stable id ([STU-AUT-008]) | "the current layer", "the selection" |
| `class` | a type token | "convert to RGB colour mode" |
| `enumerated` | a two-part `family.member` token | "fill contents = white", "blend mode = normal" |
| `measured` | a number PLUS a unit token from [STU-AUT-107] | "canvas width = 115.046 percent" |
| `number` | a bare number with no declared unit | an iteration count |
| `boolean` | true or false | "anti-alias = true" |
| `string` | a text value | a layer name |
| `list` | an ordered array of any of the above | a multi-point path |

`measured` and `number` are DIFFERENT types and MUST NOT be merged: a value with a declared unit and a value without one behave differently on document-unit change, on scaling, and on round-trip. The shipped distribution across the parameter values of 1,355 parameterised steps is 1,242 bare numbers, 1,092 references, 613 enumerated tokens, 586 measured values, 463 strings or lists, 403 booleans and 90 class tokens.

**[STU-AUT-107] Recorded unit token set (normative, closed, six members).** A `measured` value's unit is exactly one of: `percent`, `pixels`, `relative`, `angle_degrees`, `points`, `resolution_dpi`. The shipped distribution across the 586 measured values is 244 percent, 127 pixels, 90 relative, 71 angle, 42 points and 12 resolution. `relative` means the value is a multiplier against a contextual base rather than an absolute quantity; a recorder that stores a relative value as an absolute one silently breaks playback on a differently sized document, so the distinction is authority.

**[STU-AUT-108] Recorded-parameter scale contract.** The shipped baseline is 11 action sets, 149 actions, 1,543 steps of which 1,388 carry a parameter descriptor and 1,355 carry at least one parameter, spanning 102 distinct command identities and 704 distinct parameter keys. Studio's own shipped action library need not match those numbers, but the recorder, the step editor, the parameter re-editor and the player MUST be specified and tested at that scale, and the UserManual MUST state the tested ceiling. The largest single shipped action carries 26 steps.

**[STU-AUT-109] Recording is capture of the command stream.** Because every operator edit is already a `StudioCommand` ([STU-AUT-001]), recording captures the emitted command id and its resolved parameter set. Recording MUST NOT introduce a translation layer, and a recorded step MUST replay through the identical command path as the original edit. A command that cannot be recorded MUST declare `recordable: false` in its contract and MUST be insertable explicitly ([STU-AUT-110]); silent omission from a recording is forbidden.

**[STU-AUT-110] Non-recordable insertion.** Studio MUST let an operator insert a step for a `recordable: false` command by explicit selection from the command corpus, with its parameters supplied through the command's own typed input schema. This is the deduped equivalent of inserting a menu item that the recorder cannot capture.

**[STU-AUT-111] Additional step kinds (normative, closed, five members beyond a plain command step).**

| Step kind | Fields | Behaviour |
|---|---|---|
| `message_stop` | `message` (string), `allow_continue` (boolean, default true) | pauses playback and shows the message; when `allow_continue` is false, playback ends |
| `path_literal` | the exact vector-path geometry as a `StudioVectorPath` value | recreates the path on playback so geometry is not re-derived |
| `tool_stroke` | the captured stroke sample stream for a painting tool | replays the stroke; capture is opt-in per recording session |
| `conditional_branch` | `condition` (a predicate over document state), `then_action_id`, `else_action_id` (nullable) | branches to another action; the predicate domain is [STU-AUT-113] |
| `conditional_mode_change` | `source_modes[]` (set of colour-mode tokens), `target_mode` | converts colour mode only when the document's mode is in `source_modes`; otherwise the step is a no-op. This is the batch-safety step: an unconditional mode change corrupts documents that are already in the target mode |

**[STU-AUT-112] Event-bound actions.** An action MAY be bound to a document or application event: `document_open`, `document_new`, `document_before_save`, `document_after_save`, `document_before_close`, `document_after_export`, `application_start`, `selection_changed`. Event-bound actions run under the same promotion lifecycle as any other mutation and MUST be individually disableable. An event-bound action that itself triggers its own binding event MUST be detected and stopped with a typed re-entrancy diagnostic, not allowed to recurse.

**[STU-AUT-113] Conditional predicate domain (normative, closed).** A `conditional_branch` predicate is a boolean expression over exactly these document facts: document orientation (`landscape` \| `portrait` \| `square`), colour mode, bit depth, whether the document has a selection, whether the active layer is a background layer, whether the active layer is a pixel / vector / text / group / adjustment layer, whether the active layer is visible, whether the active layer has a mask, whether the document has unsaved changes, document width and height compared to a constant, and layer count compared to a constant. Predicates compose with `and`, `or`, `not`. The domain is closed so a predicate is statically checkable before a batch runs.

**[STU-AUT-114] Action library and portability.** Action sets are stored as authority records and are importable and exportable as portable artifacts registered into CKC per [STU-ASSET-012]. An exported action set carries its declared command ids and its recorded parameter keys; importing it into a Studio whose command corpus lacks one of those ids MUST report the missing id at import, and MUST import the set with that step marked unresolvable rather than silently dropping it.

---

### 2. The scripting object model

**[STU-AUT-115] The object model is a typed view, not a second model.** [STU-AUT-006] stands: the object model is a typed read/write projection over the single `StudioDocument`. This block states its shape.

**[STU-AUT-116] Suite organisation.** The object model is organised into SUITES: named groups of classes, enumerations, properties and methods sharing a domain. A suite carries `{suite_id, tag, name, description}`. The reference model recovered from the layout source application declares 28 suites covering: basics, layout, text, tables, table styles, cell styles, object styles, stroke styles, colour, links, libraries, book, hyperlinks, indexing, table of contents, XML, assignment, data merge, preflight, PDF comment import, interactive elements, preferences, user interface, and a scripting-language suite. Studio's suite set is its own; the CONTRACT is that the model is suite-organised, that a suite is the unit of capability namespacing ([STU-AUT-005]), and that every class declares its owning suite.

**[STU-AUT-117] Class record (normative shape).** A class carries `{class_id, tag, name, description, plural_name, plural_tag, plural_description, collection_type_id, suite_id, guid, collection_guid, properties[], methods[]}`. Two properties of this shape are load-bearing and MUST NOT be dropped:

- **Every class declares its own collection type.** A class and "a collection of that class" are two related but distinct types. Scripts address `document.layers` (the collection) and `document.layers[0]` (the class) with different capabilities; collapsing them forces every collection operation to be special-cased.
- **Every class carries a stable GUID separate from its human name.** The GUID survives renaming; the name is for humans. Studio MUST carry both.

**[STU-AUT-118] Primitive type system (normative, closed, eighteen members).** Every property, method parameter and method return in the object model has exactly one of these types:

| Type | Meaning |
|---|---|
| `void` | a command with no return value |
| `any` | variant |
| `short_integer` | 16-bit signed |
| `long_integer` | 32-bit signed |
| `large_integer` | 64-bit signed |
| `boolean` | |
| `string` | |
| `measurement` | a unit-bearing real: coordinates, lengths, weights. Distinct from `real` |
| `real` | a unitless double: scales, angles, percentages |
| `date_time` | |
| `file_path` | a file reference |
| `properties_record` | a bag of named property values, used for bulk set |
| `binary_stream` | binary or graphic data |
| `script_variant` | any value as seen by the script layer |
| `object_reference_mixed` | a reference that may resolve to more than one class |
| `object_reference` | a reference to one class |
| `parent_reference` | a reference to the containing object |
| `enumeration_reference` | a reference to a declared enumeration ([STU-AUT-120]) |

`measurement` versus `real` is the same distinction as `measured` versus `number` in [STU-AUT-106] and is equally load-bearing: a measurement carries the document unit and converts; a real does not.

**[STU-AUT-119] Property record (normative shape).** `{property_id, tag, name, description, type, flags, default_value?, declared_on_class_id}`. `flags` MUST at minimum encode read-only, optional, and collection-valued. A property whose type is `enumeration_reference` MUST name the enumeration.

**[STU-AUT-120] Enumeration record (normative shape).** `{enumeration_id, tag, name, description, enumerators[]}` where each enumerator is `{tag, name, description}`. Enumerations are EXTENSIBLE by declaration: a later-loaded module may add enumerators to an existing enumeration. The reference model declares 526 enumerations carrying 3,000 enumerators, with 123 declared extensions adding 260 further enumerators. Studio MUST support the extension mechanism, MUST merge extensions deterministically (by enumeration id, then by declaration order), and MUST expose the merged set — never the pre-merge set — to scripts and to the model command surface.

**[STU-AUT-121] Method record (normative shape).** `{method_id, tag, name, description, return_type, parameters[], declared_on_class_id}` where each parameter is `{tag, name, description, type, is_optional, default_value?}`. A parameter's declared default is authority: omitting the parameter MUST behave identically to passing the default.

**[STU-AUT-122] Object-model scale contract.** The reference model recovered from the layout source application declares 516 classes, 526 enumerations with 3,000 enumerators, 3,398 properties over 2,837 distinct property names, 783 methods with 1,349 parameters, 39 typedefs, 8,065 class-to-property edges and 1,607 class-to-method edges. The reference model recovered from the compositing source application declares 44 classes with 1,708 members. The reference model recovered from the vector-and-raster source application declares 367 API classes with 3,150 methods across 19 capability modules and 415 script-layer classes with 5,412 members. Studio's own object model is its own; the CONTRACT is that the generator, the type-stub emitter, the documentation generator and the MCP schema generator MUST all be specified and tested against a model of at least 500 classes and 3,000 properties, and MUST emit from ONE source of truth so the four artifacts cannot drift ([STU-AUT-002]).

**[STU-AUT-123] Capability-module organisation.** The object model's capability surface MUST be organised into named modules that a caller imports explicitly, so a script or plugin declares which capability areas it touches. The reference model recovered from the vector-and-raster source application organises 582 imported symbols into 19 modules: application, document object model, geometry, story/text, raster, user interface, layer effects, commands, colours, fills, brushes, hatches, line styles, fonts, filesystem, network, buffer, timers, and common. Studio's module set maps onto the capability namespaces of [STU-AUT-005]: importing a module is requesting its capability, and an ungranted module fails closed at import, not at first call.

**[STU-AUT-124] Command-builder pattern for batch mutation.** For multi-node structural edits, the object model MUST expose a COMMAND BUILDER: an object that accumulates node-creation and node-mutation calls and then commits them as ONE `StudioCommandBatch` ([STU-AUT-004]) with one proposal, one promotion decision and one coalesced history entry. The reference model recovered from the vector-and-raster source application exposes exactly this shape — a child-node command builder with 69 typed `add*` methods, one per node kind (adjustment nodes, filter nodes, container nodes, and so on), each taking the parent and the node's parameter record. Studio MUST provide the same shape rather than requiring N separate mutating calls, because N separate calls produce N promotion decisions and N undo entries for what the operator experiences as one edit.

**[STU-AUT-125] Expression language surface (declared, bounded).** Studio MUST provide an expression language for property-level computed values, distinct from the imperative command surface. The reference model recovered from the compositing source application declares 333 expression identifiers across 12 categories — global, layer, sub-objects, general, properties, vector math, random numbers, interpolation, colour conversion, other math, 3D, and space transforms — with 37 declared argument signatures. Studio's expression language MUST: be deterministic and side-effect-free; declare every identifier's argument signature and return type; be evaluable headlessly; and share the variable primitives of 14.10 rather than defining a parallel value model. **DECLARED GAP:** this module does NOT enumerate the 333 identifiers. Enumerating them is its own microtask against the captured identifier table, and an implementer must not infer them.

---

### 3. Batch processing

**[STU-AUT-126] Batch job record (normative shape).** A batch job carries `{job_id, source_set ([STU-AUT-127]), applied_automation ([STU-AUT-128]), output_targets[] ([STU-AUT-129]), filename_template ([STU-AUT-130]), error_policy ([STU-AUT-131]), concurrency, post_processing}`. It executes on the kernel Job Runtime as a headless, bounded, quiet job per 14.20 and [STU-AUT-012].

**[STU-AUT-127] Source-set kinds (normative, closed, five members).** `folder` (with a recurse flag and an include/exclude glob set), `explicit_file_list`, `open_documents`, `selection_within_document` (for per-artboard or per-layer batches), and `catalog_query` (a CKC search expression per [STU-ASSET-010], which is how a smart collection becomes a batch input).

**[STU-AUT-128] Applied-automation kinds (normative, closed, three members).** `action` (a `StudioAction` by id), `command_sequence` (an inline ordered command list), or `none` (a pure format-conversion pass). All three are legal; `none` is what makes the format-conversion batch of [STU-AUT-013] a configuration of the one runner rather than a separate tool.

**[STU-AUT-129] Output-target record.** `{format_id, export_recipe_id, destination, colour_profile_policy, resize_policy, overwrite_policy}`. A job MAY carry MORE THAN ONE output target, producing several formats per source in a single pass ([STU-AUT-013] multi-format output). Each target produces its own artifact and its own catalog registration per [STU-ASSET-012].

**[STU-AUT-130] Filename-template contract.** The template is a token string. The token set MUST at minimum include: original filename, original extension, sequence number (with a declared start value and zero-padding width), date and time components, document dimensions, the output format, the scale suffix, and a free custom string. Sequence-number contract: hard_min 0; hard_max NOT DECLARED IN SOURCE (Studio declares 2^31-1); soft_min 1; soft_max 100000; default 1; unit = count; precision = integer. A template that would produce a collision resolves under `overwrite_policy` ([STU-AUT-131]), never by silent overwrite.

**[STU-AUT-131] Collision and error policy enumerations (normative, closed).**

- **Overwrite policy, four members:** `ask` (interactive only; illegal in a headless job, which MUST refuse to start rather than hang), `overwrite`, `rename` (append a disambiguating suffix), `skip`. Default `rename` for headless jobs and `ask` for interactive ones.
- **Error policy, four members:** `stop_on_first_error`, `skip_and_continue`, `retry_then_skip` (with a declared retry count and backoff), `quarantine` (move the failing input to a declared quarantine location and continue). Default `skip_and_continue` with a per-file receipt.

**[STU-AUT-132] Per-file receipting.** Every processed input emits a receipt carrying the input identity, the applied automation and its resolved parameters, each output target's artifact manifest id, the elapsed time, and the outcome. A job's aggregate result is the receipt set, not a log file. Receipts are EventLedger-bound.

**[STU-AUT-133] Portable batch artifact.** A batch job MUST be exportable as a portable, re-runnable artifact that carries its full configuration and can be applied to a supplied file set without reconstructing the job. This is the deduped droplet equivalent of [STU-AUT-013]. The artifact is registered into CKC per [STU-ASSET-012].

**[STU-AUT-134] Stack and multi-input batch operations (normative, closed set of eight).** The runner MUST support these multi-input operations as job configurations, not as separate tools: load a file set as layers of one document with optional auto-alignment; render a layer stack through a statistical mode; contact sheet; multi-page presentation document; fit-to-size resize pass; split scanned images; panorama merge; and watched-folder continuous export driven by layer-name export directives.

**[STU-AUT-135] Stack statistical mode enumeration (normative, closed, eight members).** `mean`, `median`, `maximum`, `minimum`, `range`, `standard_deviation`, `variance`, `summation`. Each operates per channel across the stack. A stack whose members differ in dimensions MUST be reported as an error naming the mismatched member, never silently cropped.

**[STU-AUT-136] Quiet-law conformance.** A batch job MUST NOT open a foreground window, steal focus, hijack keyboard input, or block on a modal dialog. Its progress, per-file outcomes, current input and cancellation state MUST be readable through structured job state. A job carrying an `always_show` dialog step ([STU-AUT-105]) or an `ask` overwrite policy ([STU-AUT-131]) MUST refuse to start, naming the offending configuration. This is a hard gate: a batch runner that can hang on a dialog is not admissible.

---

### 4. Data-driven graphics

**[STU-AUT-137] Data-driven binding record.** [STU-AUT-014] and [STU-AUT-015] stand. The binding record is `{binding_id, target_node_id, bound_field, variable_id}` where `bound_field` is drawn from the bindable-field sets of [STU-DS-130] plus `visible` and `placed_asset_link`. The variable is a `StudioVariable` (14.10); there is no automation-only variable type.

**[STU-AUT-138] Dataset record.** `{dataset_id, columns[], rows[]}` where each column declares `{name, value_type}` and `value_type` is one of `string`, `number`, `boolean`, `asset_reference`. An `asset_reference` column's cells are CKC asset identities or catalog-resolvable paths per [STU-ASSET-005]; they are NOT bare filesystem paths, so a dataset survives a move.

**[STU-AUT-139] Dataset import sources (normative, closed, four members).** `manual_entry`, `delimited_text` (with a declared delimiter, quote character, encoding and header-row flag), `structured_markup`, and `catalog_query` ([STU-ASSET-010]). Import MUST report per-row type-coercion failures rather than coercing silently.

**[STU-AUT-140] Row preview and batch expansion.** Previewing a row applies it non-destructively and MUST be revertible in one undo. Batch expansion generates one output per row through the batch runner of section 3 and inherits its receipting, quiet-law and promotion semantics. A row whose `asset_reference` cell resolves to a `missing` or `unauthorized` link ([STU-ASSET-007]) MUST fail that row under the job's error policy and MUST NOT emit a placeholder into shipped output ([STU-ASSET-009]).

---

### 5. Find and change

**[STU-AUT-141] Find/change query record (normative shape).** A saved query is:

| Field | Type | Semantics |
|---|---|---|
| `query_id` | stable string | identity |
| `name` | string | display name |
| `mode` | token from [STU-AUT-142] | which matcher runs |
| `find_expression` | string | the literal, pattern, glyph id, object criteria or colour reference |
| `change_expression` | string | the replacement; empty means "find only" |
| `scope` | token from [STU-AUT-143] | search extent |
| `options` | the option record of [STU-AUT-144] | |
| `find_format_criteria` | attribute record or null | match only content carrying these attributes |
| `change_format_criteria` | attribute record or null | apply these attributes to matches |

**[STU-AUT-142] Find/change mode enumeration (normative, closed, five members).** `text`, `pattern`, `glyph`, `object`, `colour`. These are the five modes of [STU-AUT-016]; `pattern` is the regular-expression mode that also drives pattern-based text styles (14.7).

**[STU-AUT-143] Scope enumeration (normative, closed, five members).** `selection`, `story` (the current threaded text flow), `document`, `all_open_documents`, `site_or_project` (every document in the current site or project, which is how a find/change becomes a batch operation).

**[STU-AUT-144] Find/change option record (normative, closed, seven flags).** Exactly: `include_locked_layers`, `include_locked_stories`, `include_master_pages`, `include_hidden_layers`, `include_footnotes`, `kana_sensitive`, `width_sensitive`. Shipped defaults across the reference query set: `include_footnotes` true, `width_sensitive` true, and the other five false. Case sensitivity and whole-word matching are expressed in the pattern for `pattern` mode and as separate flags for `text` mode.

**[STU-AUT-145] Find/change is one command.** A find/change execution is a single `StudioCommand`: dry-runnable (returns every match with its location and the resulting diff without mutating), receipted, and undoable as ONE operation regardless of match count. A find/change that produced 4,000 replacements MUST undo in one step.

**[STU-AUT-146] Saved-query library.** Saved queries are authority records, exportable and importable as portable artifacts registered into CKC. The reference query set ships 11 pattern queries; the contract is the record shape and the library, not the count.

---

### 6. The model-steerable command surface

**[STU-AUT-147] Command corpus contract.** Studio's automatable surface is a single enumerated corpus. Every entry carries: `command_id` (stable, never reused), `display_name`, `input_schema` and `output_schema` (both `schemars::JsonSchema`-deriving), `capability` ([STU-AUT-005]), `recordable` (boolean, [STU-AUT-109]), `dry_runnable` (boolean; false only for pure reads), `mutates_authority` (boolean), `enabler` (an optional pure predicate, [STU-AUT-148]), `undo_semantics`, `receipt_shape`, `argus_targets` (the stable `author_id`s a model can observe the effect on), and `manual_anchor`. The corpus is machine-readable authority; the operator-facing command palette and the menu tree are PROJECTIONS over it.

**[STU-AUT-148] Enabler predicate contract.** An enabler is a pure, side-effect-free predicate over the current document state, selection and context that answers "is this command applicable right now". It MUST be evaluable headlessly and cheaply, because the model surface uses it to plan rather than to probe by attempting edits. The reference command surface declares an enabler on 1,274 of its 2,176 invocable entries — roughly three in five — which is the coverage level Studio should expect, not a ceiling.

**[STU-AUT-149] Command-surface scale contract.** The reference command surface recovered from the web source application declares 2,176 invocable entries across 301 menus and 89 menubars, 508 of them carrying a keyboard shortcut, backed by 445 distinct implementing files and 264 scripted command surfaces; a further reference surface declares 693 menu paths and 1,097 actions, and another 1,491 commands. Studio's corpus is its own; the contract is that the corpus registry, the palette, the shortcut editor, the menu projection, the MCP tool generator and the UserManual generator MUST all be specified and tested at a corpus size of at least 2,000 entries and MUST be generated from the single registry so none can drift.

**[STU-AUT-150] Scripted-command surface hooks (normative, closed set of six).** A command that presents its own parameter surface declares at most: `can_accept_command()` (the enabler), `requires_document_model()` (whether a parsed document is needed), `initialize_ui()`, `receive_arguments(args)` (accept typed arguments for a headless invocation — this is what makes a dialog-bearing command scriptable), `command_buttons()` (the surface's action buttons), and `window_dimensions()`. `receive_arguments` is MANDATORY for every command that presents a parameter surface: a command that can only be driven by its dialog fails [STU-CON-007] and is not admissible.

**[STU-AUT-151] Shortcut-set contract.** Studio MUST support multiple named keyboard-shortcut sets with exactly one active per operator, per-command rebinding, locale-adaptive remapping, and conflict detection at registration. The reference baseline ships three complete sets of 776, 701 and 509 bindings with one active on a clean install, plus 23 locale-adaptive layout sets over 138 keyboard layouts. A binding conflict within a set is a registration error naming both commands; a binding that resolves to no corpus entry is a registration error naming the binding.

**[STU-AUT-152] [STU-CON-007] conformance, stated concretely.** Every corpus entry MUST satisfy all four properties, and each is testable:

- **Model-invokable.** The entry is reachable through the MCP tool surface generated from its `input_schema`, with no dialog required and no screen-reading. Test: invoke every corpus entry headlessly with schema-valid input and assert none blocks.
- **Parallel-safe.** Two model lanes invoking the entry against different targets MUST both succeed; against the same target, exactly one succeeds and the other receives a typed conflict under the expected-revision precondition of [STU-SDB-004]. Test: concurrent invocation matrix per entry class.
- **Deterministic.** Identical input against identical document state produces byte-identical output state and a receipt identical except for identity and timestamp fields. Test: double-run comparison on a fixture document.
- **Visually verifiable.** The effect is observable through a rendered capture of the named `argus_targets` with no foreground window and no focus steal. Test: capture-before, invoke, capture-after, assert the diff is non-empty and bounded to the expected region.

An entry failing any of the four is NOT admissible to the corpus. There is no "operator-only" exemption ([STU-MDL-006]).

**[STU-AUT-153] Parallel-lane isolation.** Multiple model lanes MUST be able to operate on DIFFERENT documents concurrently with no shared mutable state beyond the authority store, and on the SAME document concurrently at record granularity per domain ([STU-DS-163], [STU-WEB-131]). A lane MUST be identifiable in every receipt through its `KernelActor`, and a lane's failure MUST NOT abort another lane's transaction.

**[STU-AUT-154] Deterministic replay of a session.** A recorded sequence of corpus invocations against a known starting document revision MUST replay to the identical end state. Any command whose result depends on wall-clock time, random seeding, filesystem enumeration order, locale collation, or floating-point non-determinism MUST declare that dependency in its contract and MUST accept the dependent input as an explicit parameter (a supplied seed, a supplied clock, a supplied collation) so replay is possible.

---

### 7. Plugins

**[STU-AUT-155] Plugin contract retained.** [STU-AUT-017] through [STU-AUT-020] stand: one plugin contract, manifest-declared capabilities, consent gating, sandboxed execution, no privileged private document model, local registry distribution, no hosted marketplace dependency.

**[STU-AUT-156] Manifest record (normative shape).** `{plugin_id, name, version, entry_points[], targeted_document_modes[], requested_capabilities[], requested_network_domains[] (each with a stated reason), extension_points[], relaunch_entries[], contributed_commands[]}`. Every `contributed_command` joins the corpus of [STU-AUT-147] under the plugin's namespace and is subject to the same four admissibility properties.

**[STU-AUT-157] Extension-point enumeration (normative, closed, twelve members).** `panel` (a contextual panel declared per [STU-WEB-033]), `command`, `node_factory`, `event_subscriber`, `codegen_language`, `text_review`, `board_widget`, `relaunch_entry`, `inspector_field`, `export_writer`, `import_reader`, `validation_descriptor`. A plugin capability not declared and not consent-granted is unavailable at runtime and fails closed ([STU-AUT-019]).

**[STU-AUT-158] Plugin event subscription set (normative, closed).** A plugin may subscribe to: `run`, `selection_change`, `document_change`, `page_change`, `view_change`, `text_review`, `style_change`, `drop`, `close`, `timer_start`, `timer_stop`, `timer_pause`, `timer_resume`, `timer_done`, `timer_adjust`. Subscription is capability-gated; an unsubscribed event is not delivered.

**[STU-AUT-159] Per-node plugin data.** A plugin may store PRIVATE key-value data on a node, namespaced to itself, and SHARED key-value data on a node readable by other plugins under an explicit shared namespace. Both travel with the document. Plugin data is document authority and participates in undo, CRDT merge and export receipts. A plugin MUST NOT store secrets in node data; node data travels with the document to anyone who can read the document.

**[STU-AUT-160] Plugin-local storage.** A plugin may persist plugin-local key-value data separate from any document. It is scoped to the plugin and the account, passes the `ResourceBroker`, and MUST NOT be used as a shadow document store.

**[STU-AUT-161] Plugin UI isolation.** Plugin UI runs in an isolated surface with no direct access to shell internals and communicates with the sandboxed plugin logic through a typed message channel. A plugin panel declares its binding through the contextual panel contract ([STU-WEB-033]) and its `author_id_prefix` MUST be namespaced to the plugin so its controls are addressable and collision-free ([STU-WEB-042]).

**[STU-AUT-162] Plugin quick-run.** A plugin command MAY declare typed input parameters gathered from the command surface for a headless quick run, with no UI shown. This is the plugin-side counterpart of [STU-AUT-150]'s `receive_arguments` and is required for a plugin command to satisfy [STU-CON-007].

---

### 8. Inspect, codegen and handoff

**[STU-AUT-163] Inspect surface retained and bounded.** [STU-AUT-021], [STU-AUT-022], [STU-AUT-023] and [STU-AUT-024] stand. This block adds the enumerations they left open.

**[STU-AUT-164] Codegen language enumeration (normative, closed baseline of sixteen, extensible by plugin).** A codegen result declares its language as one of: `BASH`, `CPP`, `CSS`, `GO`, `GRAPHQL`, `HTML`, `JAVASCRIPT`, `JSON`, `KOTLIN`, `PLAINTEXT`, `PYTHON`, `RUBY`, `RUST`, `SQL`, `SWIFT`, `TYPESCRIPT`. A `codegen_language` plugin extension point ([STU-AUT-157]) may register additional languages; a registered language MUST declare its own token and MUST NOT shadow a baseline token.

**[STU-AUT-165] Codegen unit preference enumeration (normative, closed, two members).** `PIXEL` (emit absolute pixel values) and `SCALED` (emit values scaled by a declared factor). The preference is per codegen invocation and MUST be carried in the result so the consumer knows which it got.

**[STU-AUT-166] Selection-render contract.** `inspect.render_selection(node_id, format, scale)` renders any node to bytes for visual verification. `format` is drawn from the export-format set of 14.13; `scale` follows the export constraint contract of [STU-IO-114]. This is the command that satisfies the visual-verifiability half of [STU-CON-007] for every domain that does not have its own capture command.

**[STU-AUT-167] Handoff aid record shapes.** The aids of [STU-AUT-024] carry these shapes:

- **Annotation:** `{label, label_markdown, properties[], category_id}` where `properties` is drawn from the closed thirty-three-member set of [STU-DS-157] and renders LIVE values.
- **Measurement:** `{measurement_id, start: {node_id, side}, end: {node_id, side}, offset, free_text}` where `side` ∈ {`TOP`, `RIGHT`, `BOTTOM`, `LEFT`} and `offset` ∈ {`INNER`, `OUTER`}. Measurements stay in sync as geometry changes; a measurement whose endpoint node is deleted transitions to a broken state and is reported, never silently removed.
- **Dev resource link:** `{name, url, inherited_from_node_id?}` — an external ticket, repository or document reference attached to a node, optionally inherited from an ancestor.
- **Implementation status:** the two-member enumeration of [STU-DS-159].

**[STU-AUT-168] Instance-versus-main drift report.** The drift report of [STU-AUT-024] MUST list, per instance: the override records present ([STU-DS-117]), the fields whose resolved value differs from the definition, and the detach source of any detached former instance ([STU-DS-142]). It is a locally computed projection over authority rows.

**[STU-AUT-169] External adapter posture retained.** [STU-AUT-023] stands without amendment: no vendor account, hosted marketplace, hosted endpoint or subscription is a runtime dependency of any operator-, plugin- or model-facing automation capability. A hosted REST or webhook adapter is optional, capability-gated, and maps its scopes to capability names and its events to `studio.*` EventLedger triggers.

---

### 9. Obligations

**[STU-AUT-170] Asset library binding.** Every automation surface that reads or writes files — the batch runner's source sets and output targets, the dataset `asset_reference` columns, portable action and batch and query artifacts, plugin-supplied assets, and codegen output — binds to CKC per [STU-ASSET-005], [STU-ASSET-010] and [STU-ASSET-012]. Automation MUST NOT reach the filesystem directly except through a capability-gated filesystem capability declared in the command's contract, and every such reach MUST be receipted.

**[STU-AUT-171] Validation descriptor set.** This sub-section contributes at minimum: `action_declared_step_count_mismatch`, `action_shortcut_collision`, `action_step_command_unresolvable`, `action_step_parameter_key_unknown`, `action_measured_value_missing_unit`, `batch_always_show_dialog_in_headless`, `batch_ask_overwrite_in_headless`, `batch_stack_dimension_mismatch`, `dataset_column_type_coercion_failure`, `dataset_asset_reference_unresolvable`, `findchange_pattern_unbounded`, `command_missing_input_schema`, `command_missing_receive_arguments`, `command_enabler_has_side_effect`, `command_not_deterministic_undeclared`, `plugin_capability_not_granted`, `plugin_author_id_prefix_collision`, `plugin_node_data_contains_secret_pattern`, `codegen_language_token_shadows_baseline`, `event_bound_action_reentrancy`.

**[STU-AUT-172] Diagnostic-tier obligation.** Every failure mode in [STU-AUT-171] MUST be surfaced at all three diagnostic tiers wired against the current kernel base: the in-process structured diagnostic, the operator-facing diagnostics surface, and the external watcher. All three exist in the base; none may be deferred.

**[STU-AUT-173] Resource-privacy obligation.** Action sets, batch job definitions and receipts, datasets, saved queries, plugin manifests and grants, plugin-local storage, and codegen results are resource-scoped authority records. Every read and write passes the `ResourceBroker` and the record-level SurrealDB permissions of [STU-SDB-005], and cross-account and cross-project adversarial cases are part of the acceptance proof.

**[STU-AUT-174] Storage constraint.** Nothing in this sub-section introduces a second database. Action sets, batch definitions, receipts, datasets, saved queries, the command corpus registry, plugin grants and plugin-local storage are SurrealDB `SCHEMAFULL` tables. Portable artifacts and batch outputs are content-addressed artifacts. No SQLite, libSQL, Turso or PostgreSQL anywhere, including test fixtures and development caches ([STU-OVR-003]).

**[STU-AUT-175] GUI / Argus / UserManual obligation.** [STU-AUT-027] remains in force unchanged and additionally covers every record shape, enumeration, bound and command introduced by [STU-AUT-100] through [STU-AUT-174]. Every enumeration here MUST appear in the model-facing UserManual as its literal token list, and every command in the corpus MUST carry a resolved `manual_anchor` or an explicit BLOCKED record.

---

### 10. Microtask Derivation

**[STU-AUT-176] Derivation rule (NORMATIVE).** The automation microtask set is derived from this module mechanically, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. Each numbered clause that states a **recorded-automation record shape** ([STU-AUT-102], [STU-AUT-103], [STU-AUT-104], [STU-AUT-111], [STU-AUT-114]), a **closed enumeration or value-type vocabulary** ([STU-AUT-105], [STU-AUT-106], [STU-AUT-107], [STU-AUT-113], [STU-AUT-118], [STU-AUT-127], [STU-AUT-128], [STU-AUT-131], [STU-AUT-134], [STU-AUT-135], [STU-AUT-142], [STU-AUT-143], [STU-AUT-144], [STU-AUT-150], [STU-AUT-157], [STU-AUT-158], [STU-AUT-164], [STU-AUT-165]), an **object-model record shape** ([STU-AUT-116], [STU-AUT-117], [STU-AUT-119], [STU-AUT-120], [STU-AUT-121], [STU-AUT-123], [STU-AUT-124]), a **command-contract rule** ([STU-AUT-101], [STU-AUT-109], [STU-AUT-110], [STU-AUT-112], [STU-AUT-147], [STU-AUT-148], [STU-AUT-152], [STU-AUT-153], [STU-AUT-154], [STU-AUT-151]), a **batch or data-driven policy** ([STU-AUT-126], [STU-AUT-129], [STU-AUT-130], [STU-AUT-132], [STU-AUT-133], [STU-AUT-136], [STU-AUT-137], [STU-AUT-138], [STU-AUT-139], [STU-AUT-140], [STU-AUT-145], [STU-AUT-146], [STU-AUT-141]), a **plugin contract** ([STU-AUT-156], [STU-AUT-159], [STU-AUT-160], [STU-AUT-161], [STU-AUT-162]), an **inspect or handoff record** ([STU-AUT-166], [STU-AUT-167], [STU-AUT-168]), or a **scale contract** ([STU-AUT-108], [STU-AUT-122], [STU-AUT-149]), where that clause can be implemented and proven independently of its siblings.
2. Each **validation-descriptor clause** in sub-section 11, [STU-AUT-181] through [STU-AUT-200]. Each of the 20 descriptors named in [STU-AUT-171] is stated as its own clause precisely so it yields its own microtask: a check is a unit of implementable, independently provable work, and one microtask reading "implement 20 checks" is not implementable by the small models these contracts are sized for. A descriptor list inside a single clause, whether as prose or as a table, is one unit to any derivation tool and therefore loses 19 units of real work.
3. Each **declared gap** — in this module exactly one, [STU-AUT-125], the expression-identifier catalogue. It yields a microtask under [STU-AUT-177], not nothing.

No other unit yields a microtask. Exactly 9 clauses in this module yield nothing, and they are:

- **Baseline, scope-fence and supersession clauses** — [STU-AUT-100], which sits under the bookkeeping heading `0. Baseline, supersession and disposition`. These are discharged when the v02.206 bundle lands, not by a work packet.
- **Pure pointer clauses** — [STU-AUT-115], [STU-AUT-155], [STU-AUT-163]. Each restates a clause that already carries the contract; the microtask lives there.
- **This derivation sub-section itself** — its five clauses yield nothing.

Every other clause yields at least one unit. This list is the module's declared non-yielding set and is the authority a derivation tool reconciles against.

**[STU-AUT-177] Open items and blocked dependencies.** [STU-AUT-125] declares that this module does NOT enumerate the 333 expression identifiers, their 12 categories or their 37 argument signatures, and forbids an implementer inferring them. That clause YIELDS a microtask. Its FIRST acceptance criterion MUST be resolving the dependency: extracting the ordered identifier table, its per-category assignment and its argument-signature pool from the named capture record, and amending [STU-AUT-125] with the enumerated result — after which the expression-language implementation microtasks become derivable. Until that acceptance criterion passes, no downstream expression-language microtask may be activated. The same rule governs any open item or BLOCKED dependency a later amendment introduces: it STILL yields a microtask, whose first acceptance criterion is resolving the named dependency or raising a BLOCKED record naming the exact blocker. A declared gap MUST NOT be dropped from the yields index, because a gap that yields nothing disappears silently and is rediscovered at implementation time.

**[STU-AUT-178] Microtask content obligation.** A microtask derived under [STU-AUT-176] MUST carry into its own body: the clause anchor; the COMPLETE member list of every closed enumeration it touches, as literal tokens — all four dialog options, all seven recorded value types, all six unit tokens, all eighteen primitive types, all sixteen codegen languages; the full seven-field parameter record of every numeric parameter it touches; the record shape with every required field of every record it touches; the four admissibility properties of [STU-AUT-152] where it adds a command to the corpus; and the shipped scale figure of [STU-AUT-108], [STU-AUT-122] or [STU-AUT-149] where it touches a surface those clauses size. A microtask that says "record action steps" without the ten step fields of [STU-AUT-104] and the four dialog options of [STU-AUT-105] does not satisfy this clause.

**[STU-AUT-179] Yields index (NORMATIVE).** The counts below are the derivation surface of this module under [STU-AUT-176]. They are not estimates: they are the measured output of applying that rule to this module's text, and every row states which unit kinds it contributes.

| Unit group | Clauses | Units by kind | Yields |
|---|---|---|---|
| Actions and macros — the recorded-automation model | [STU-AUT-101]-[STU-AUT-114] | 14 clause, 2 enumeration | 16 |
| The scripting object model | [STU-AUT-116]-[STU-AUT-125] | 10 clause | 10 |
| Batch processing | [STU-AUT-126]-[STU-AUT-136] | 11 clause | 11 |
| Data-driven graphics | [STU-AUT-137]-[STU-AUT-140] | 4 clause | 4 |
| Find and change | [STU-AUT-141]-[STU-AUT-146] | 6 clause | 6 |
| The model-steerable command surface | [STU-AUT-147]-[STU-AUT-154] | 8 clause | 8 |
| Plugins | [STU-AUT-156]-[STU-AUT-162] | 7 clause | 7 |
| Inspect, codegen and handoff | [STU-AUT-164]-[STU-AUT-169] | 6 clause | 6 |
| Obligations | [STU-AUT-170]-[STU-AUT-175] | 6 clause | 6 |
| Validation Descriptor Catalogue | [STU-AUT-181]-[STU-AUT-200] | 20 validator | 20 |
| Clauses yielding nothing | 9 clauses, listed in [STU-AUT-176] | — | 0 |
| **Module total** | | **101 clauses** | **94** |

Of this module's 101 clauses, 9 yield nothing and 92 yield at least one unit; tables inside yielding clauses contribute the remainder. The module total is **94**. The last numeric column is the yields count.

**[STU-AUT-180] Anchor binding.** A microtask derived from this module cites its clause anchor directly. A microtask staged before this module landed carries `spec_anchor_status = "PROVISIONAL"`; binding it to an anchor in [STU-AUT-100]–[STU-AUT-200], or to a preserved v02.205 anchor in [STU-AUT-001]–[STU-AUT-027], clears that status. A microtask that cannot cite either is out of scope for the automation domain and MUST be re-derived or retired, not activated.

---

### 11. Validation Descriptor Catalogue

Each descriptor below is its own clause because each is its own unit of implementable, independently provable work: feed the runtime a document that violates the rule and assert the check fires with the stated diagnostic. [STU-AUT-171] names the set; the clauses in this sub-section state what each member catches, which clause it enforces, its severity, and what its diagnostic MUST name. Every one is a `StudioValidationDescriptor` in the catalogue of 14.24.

**[STU-AUT-181] `action_declared_step_count_mismatch`.** The automation validator MUST reject, with severity `error`, a document or command in which an action's `declared_step_count` differs from the length of its `steps` array, enforcing [STU-AUT-103]. The diagnostic MUST name the action, both counts; this fails the load, it is not a warning.

**[STU-AUT-182] `action_shortcut_collision`.** The automation validator MUST reject, with severity `error`, a document or command in which two actions in enabled sets claim the same function key and modifier combination, enforcing [STU-AUT-103]. The diagnostic MUST name both actions and the binding.

**[STU-AUT-183] `action_step_command_unresolvable`.** The automation validator MUST reject, with severity `error`, a document or command in which a recorded step names a `command_id` that resolves to no entry in the command corpus, enforcing [STU-AUT-104]. The diagnostic MUST name the step, the command id, and the owning action; on import the step is marked unresolvable rather than dropped.

**[STU-AUT-184] `action_step_parameter_key_unknown`.** The automation validator MUST reject, with severity `warning`, a document or command in which a recorded step carries a parameter key the resolved command's input schema does not declare, enforcing [STU-AUT-104]. The diagnostic MUST name the step, the key, and the command.

**[STU-AUT-185] `action_measured_value_missing_unit`.** The automation validator MUST reject, with severity `error`, a document or command in which a recorded parameter value of type `measured` carries no unit token from the closed six, enforcing [STU-AUT-107]. The diagnostic MUST name the step and the parameter key; a measured value without a unit replays wrongly on a differently sized document.

**[STU-AUT-186] `batch_always_show_dialog_in_headless`.** The automation validator MUST reject, with severity `error`, a document or command in which a headless batch job's action contains an enabled step whose `dialog_options` is `always_show`, enforcing [STU-AUT-136]. The diagnostic MUST name the step and its owning action; the job refuses to start rather than hanging.

**[STU-AUT-187] `batch_ask_overwrite_in_headless`.** The automation validator MUST reject, with severity `error`, a document or command in which a headless batch job declares an `ask` overwrite policy, enforcing [STU-AUT-131]. The diagnostic MUST name the job; it refuses to start.

**[STU-AUT-188] `batch_stack_dimension_mismatch`.** The automation validator MUST reject, with severity `error`, a document or command in which a stack statistical mode is applied to members whose dimensions differ, enforcing [STU-AUT-135]. The diagnostic MUST name every mismatched member and its dimensions; the members MUST NOT be silently cropped.

**[STU-AUT-189] `dataset_column_type_coercion_failure`.** The automation validator MUST reject, with severity `error`, a document or command in which a dataset cell cannot be coerced to its column's declared `value_type`, enforcing [STU-AUT-139]. The diagnostic MUST name the row, the column and the raw cell value.

**[STU-AUT-190] `dataset_asset_reference_unresolvable`.** The automation validator MUST reject, with severity `error`, a document or command in which an `asset_reference` cell resolves to a `missing` or `unauthorized` placed-asset link, enforcing [STU-AUT-140]. The diagnostic MUST name the row, the column and the link state; the row fails under the job's error policy and emits no placeholder.

**[STU-AUT-191] `findchange_pattern_unbounded`.** The automation validator MUST reject, with severity `error`, a document or command in which a pattern-mode query supplies an expression whose evaluation is not bounded in linear time, enforcing [STU-AUT-142]. The diagnostic MUST name the query and the offending construct.

**[STU-AUT-192] `command_missing_input_schema`.** The automation validator MUST reject, with severity `error`, a document or command in which a corpus entry declares no `input_schema`, so it cannot be model-invoked or MCP-exposed, enforcing [STU-AUT-147]. The diagnostic MUST name the command id.

**[STU-AUT-193] `command_missing_receive_arguments`.** The automation validator MUST reject, with severity `error`, a document or command in which a command presents a parameter surface but declares no typed argument entry point, enforcing [STU-AUT-150]. The diagnostic MUST name the command id; a dialog-only command fails the model-invocable property.

**[STU-AUT-194] `command_enabler_has_side_effect`.** The automation validator MUST reject, with severity `error`, a document or command in which an enabler predicate mutates state, opens a resource, or is not evaluable headlessly, enforcing [STU-AUT-148]. The diagnostic MUST name the command id and the observed side effect.

**[STU-AUT-195] `command_not_deterministic_undeclared`.** The automation validator MUST reject, with severity `error`, a document or command in which a command's output varies across two runs on identical state without declaring its dependency on a clock, seed, collation or enumeration order, enforcing [STU-AUT-154]. The diagnostic MUST name the command and the differing fields.

**[STU-AUT-196] `plugin_capability_not_granted`.** The automation validator MUST reject, with severity `error`, a document or command in which a plugin resolves a command or extension point whose capability its manifest does not declare or consent has not granted, enforcing [STU-AUT-157]. The diagnostic MUST name the plugin, the capability and the attempted command; the call fails closed.

**[STU-AUT-197] `plugin_author_id_prefix_collision`.** The automation validator MUST reject, with severity `error`, a document or command in which two plugins, or a plugin and a built-in panel, claim the same `author_id_prefix`, enforcing [STU-AUT-161]. The diagnostic MUST name both claimants; this is reported at registration, never at selection.

**[STU-AUT-198] `plugin_node_data_contains_secret_pattern`.** The automation validator MUST reject, with severity `warning`, a document or command in which per-node plugin data matches a credential or token pattern, and node data travels with the document to every reader, enforcing [STU-AUT-159]. The diagnostic MUST name the plugin, the node and the key name only, never the value.

**[STU-AUT-199] `codegen_language_token_shadows_baseline`.** The automation validator MUST reject, with severity `error`, a document or command in which a plugin-registered codegen language reuses a token from the sixteen-member baseline, enforcing [STU-AUT-164]. The diagnostic MUST name the plugin and the shadowed token.

**[STU-AUT-200] `event_bound_action_reentrancy`.** The automation validator MUST reject, with severity `error`, a document or command in which an event-bound action triggers its own binding event, so playback would recurse, enforcing [STU-AUT-112]. The diagnostic MUST name the action and the event; execution stops rather than recursing.
