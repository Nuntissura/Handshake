---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-15"
section_id: "14.15"
title: "14.15 Whiteboard & Diagramming"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
supersedes_section: "14.15 in .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md lines 3078-3143"
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-15-whiteboard.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.15 Whiteboard & Diagramming

This sub-section is the normative Studio whiteboard and diagramming surface. v02.205 stated the posture — a mode of the one `StudioDocument`, board objects as `StudioLayer` kinds, widgets as plugins, facilitation as a collaboration projection — and gave an eleven-row object table in prose. This module gives each board object kind its field-level contract.

### 0. Baseline, supersession and disposition

**[STU-WB-100] Baseline preservation and supersession.** Clauses [STU-WB-001] through [STU-WB-011] of v02.205 remain in force in full. Clauses [STU-WB-100] and above add the record shapes and enumerations. Explicit corrections:

| v02.205 clause | Disposition | Replacement |
|---|---|---|
| [STU-WB-004] eleven-row object table | EXTENDED | Twelve board node kinds with field-level contracts ([STU-WB-104] through [STU-WB-118]); the drawing/ink row is folded into the vector ink kind and a twelfth kind — link-unfurl — is separated from the media-embed row because it carries different fields. |
| [STU-WB-004] "Connectors are smart: they attach to object anchor magnets" | EXTENDED | The connector endpoint model is three closed variants ([STU-WB-108]) and the endpoint cap set is twelve members ([STU-WB-109]). |
| [STU-WB-006] board widgets | EXTENDED | The widget synced-state contract is [STU-WB-119]. |

**[STU-WB-101] Domain proportionality, stated honestly.** Whiteboarding is a lower-priority parity domain than raster, vector, layout, typography, colour, effects, design system and video. The capability registry tags 35 rows `whiteboard`, against 4,609 for layout and 3,826 for vector — this is the thinnest domain in Studio by roughly two orders of magnitude. The board node kinds below are recovered from a single source object model; there is no second corroborating capture. This module is therefore COMPLETE against its evidence and NARROW in absolute terms, and that is a deliberate scope choice recorded here rather than a gap discovered later.

---

### 1. Board mode and board nodes

**[STU-WB-102] Board node kinds are `StudioLayer` kinds.** Every board object is a `StudioLayer` whose `kind` selects a board payload. Board nodes share the selection, transform, blend, opacity, lock, visibility, export and history surfaces of every other Studio layer; this sub-section states only what is ADDITIONAL per kind.

**[STU-WB-103] Board node kind enumeration (normative, closed, twelve members).** `sticky`, `shape_with_text`, `connector`, `stamp`, `highlight`, `washi_tape`, `table`, `code_block`, `section`, `media_embed`, `link_unfurl`, `widget`. Freehand ink is not a separate kind: a marker or highlighter stroke is a `StudioVectorPath` with an ink profile, per [STU-WB-117].

**[STU-WB-104] Sticky note contract.** Additional fields: `text` (a text sublayer, so a sticky's typography is the ordinary `StudioTextStory` surface and not a board-private text model), `author_visible` (boolean), `author_name` (string), `is_wide_width` (boolean). A sticky auto-sizes to its text; `is_wide_width` selects the wide variant. `author_name` is a DISPLAY string captured at creation, distinct from the `KernelActor` on the creating event — the actor is authority, the display name is content and MUST be editable without rewriting history.

**[STU-WB-105] Shape-with-text contract.** Additional fields: `shape_type` (an enumerated diagram-shape token), `text` (a text sublayer), `corner_radius` (number, optional), plus the fill, stroke and blend surfaces of an ordinary layer. It resizes around its text. `shape_type` is a closed enumeration Studio declares — the source model's token list was not recovered as an explicit union, so Studio MUST declare its own set (at minimum rectangle, rounded rectangle, ellipse, diamond, triangle, parallelogram, hexagon, cylinder, cloud, document, predefined-process and terminator), MUST label it `studio_declared`, and MUST NOT present it as source-derived.

**[STU-WB-106] Connector contract.** Additional fields: `connector_line_type` ([STU-WB-107]), `connector_start` and `connector_end` (endpoint records, [STU-WB-108]), `connector_start_stroke_cap` and `connector_end_stroke_cap` ([STU-WB-109]), `text` (an inline label as a text sublayer), `text_background` (a label backing sublayer), `corner_radius` (number, optional, applied at elbow bends), and `rotation`. A connector carries stroke and blend but no fill.

**[STU-WB-107] Connector line-type enumeration (normative, closed, three members).** `STRAIGHT`, `ELBOWED`, `CURVED`. Default `ELBOWED`.

**[STU-WB-108] Connector endpoint record (normative, three closed variants).** An endpoint is exactly one of:

| Variant | Fields | Behaviour |
|---|---|---|
| position | `{position: {x, y}}` | pinned to a canvas coordinate; does not follow any object |
| magnet | `{endpoint_node_id, magnet}` | attached to a node's anchor magnet; follows the node and reroutes |
| position-and-node | `{position: {x, y}, endpoint_node_id}` | attached to a node at a specific relative point |

`magnet` is a token naming which anchor is used: at minimum `AUTO`, `TOP`, `RIGHT`, `BOTTOM`, `LEFT` and `CENTER`. `AUTO` re-picks the nearest anchor as geometry changes; the others pin. A connector whose `endpoint_node_id` no longer resolves MUST transition to a broken state, MUST retain its last known position so the diagram does not collapse, and MUST be reported — never silently deleted.

**[STU-WB-109] Connector stroke-cap enumeration (normative, closed, twelve members).** `NONE`, `ARROW_EQUILATERAL`, `ARROW_LINES`, `TRIANGLE_FILLED`, `CIRCLE_FILLED`, `DIAMOND_FILLED`, `ERD_ONE`, `ERD_MANY`, `ERD_ONE_OR_MORE`, `ERD_ZERO_OR_ONE`, `ERD_ZERO_OR_MORE`, `ERD_EXACTLY_ONE`. The six entity-relationship caps are first-class members, not decorations: they are what makes the board usable for data modelling, and dropping them reduces the connector to an arrow.

**[STU-WB-110] Stamp contract.** Additional fields: an author reference resolvable asynchronously to the placing account. A stamp is a droppable marker (dot, star, heart, plus-one and similar) whose attribution is queryable, which is what makes stamp-based voting tallyable ([STU-WB-121]). Stamp artwork is a CKC asset link per [STU-ASSET-005], not embedded bytes.

**[STU-WB-111] Highlight contract.** A highlight is a vector-like board node carrying corner radius and the vector-network surface, used to mark a region of the board. It participates in the ordinary blend and opacity surfaces.

**[STU-WB-112] Washi tape contract.** A decorative tape strip. It is a `StudioVectorPath`-backed node whose fill is a `StudioPattern` (14.8), and it carries the stickable behaviour of [STU-WB-118]. It has no board-private pattern model.

**[STU-WB-113] Table contract.** Additional fields: `num_rows`, `num_columns` (both read-only derived), plus the fill and blend surfaces. Required commands: `cell_at(row, column)`, `insert_row(index)`, `insert_column(index)`, `remove_row(index)`, `remove_column(index)`, `move_row(from, to)`, `move_column(from, to)`, `resize_row(index, height)`, `resize_column(index, width)`. A cell is its own node carrying a fill and a text sublayer. This is the board table; it shares the `StudioLayer` table kind with the layout domain's table and MUST NOT be a second table model ([STU-DOC-004]).

**[STU-WB-114] Code-block contract.** Additional fields: `code` (string) and `code_language` (a token). The language token set MUST be the codegen language enumeration of [STU-AUT-164] so a board code block and a codegen result speak the same vocabulary. Syntax colouring uses the colouring-scheme mechanism of [STU-WEB-045]; there is no board-private highlighter.

**[STU-WB-115] Section contract.** Additional fields: `section_contents_hidden` (boolean — the collapse state), plus fill, stroke, corner-radius and per-corner radius, and `dev_status` from the two-member enumeration of [STU-DS-159]. A section is a named container: it groups board content, collapses, and is addressable as a navigation and voting target.

**[STU-WB-116] Media-embed and link-unfurl contracts.** These are two kinds, not one:

- `media_embed` carries a media payload record and supports resize with and without constraints. Its bytes are a CKC asset link per [STU-ASSET-005].
- `link_unfurl` carries an unfurled link-preview payload: the source URL plus the recovered title, description and thumbnail. The thumbnail is a CKC asset. Unfurling requires a network fetch and is therefore an OPTIONAL adapter under [STU-OVR-002]: with no network, the node renders as a plain link and MUST say so, never as a broken image.

An `embed` node carrying an external interactive frame is a third case and MUST be capability-gated: rendering third-party interactive content inside the board is a network and execution capability, declared and consent-granted, never ambient.

**[STU-WB-117] Ink contract.** Marker and highlighter strokes are `StudioVectorPath` nodes carrying an ink profile: `{tool: marker | highlighter, width, opacity, blend_mode, pressure_curve?}`. Highlighter ink defaults to a multiply blend so overlaps read as ink rather than as paint. Ink is tuned for whiteboarding, not precision illustration, but it is the SAME vector primitive: a board stroke can be selected, edited and exported through the ordinary vector surface ([STU-DOC-004]).

**[STU-WB-118] Stickable behaviour.** Stamp, highlight and washi-tape nodes carry a `stuck_to_node_id`: when set, the node moves with the node it is stuck to. This is what makes a vote stamp stay on the sticky it was placed on when the board is rearranged. A `stuck_to_node_id` that no longer resolves MUST unstick the node in place and report it.

**[STU-WB-119] Widget contract.** A widget node carries `widget_id` (the plugin-declared widget identity), `widget_synced_state` (a read-only key-value map replicated to every viewer), `synced_state_overrides` and `synced_map_overrides` (per-instance overrides), plus commands `set_widget_synced_state` and `clone_widget`. Synced state travels over the CRDT collaboration substrate (14.17) — a widget MUST NOT open its own transport. Widgets are plugins on the `board_widget` extension point ([STU-AUT-157]) running under the kernel capability and consent gates ([STU-AUT-018]); there is no special-cased built-in widget.

---

### 2. Board structure and collaboration

**[STU-WB-120] Multi-board documents.** [STU-WB-002] stands: one document holds several boards, switchable from the shared page navigator. A board is a page-kind container; it is not a distinct top-level document type.

**[STU-WB-121] Facilitation record shapes.** [STU-WB-007] and [STU-WB-008] stand. The records:

- **Voting session:** `{session_id, facilitator_actor, target_scope (a node id set or a section id), votes_per_participant, anonymous (boolean), state (open | revealed | closed), votes[]}` where each vote is `{voter_actor, target_node_id, count}`. When `anonymous` is true, `voter_actor` is retained in authority for audit but MUST NOT be projected to participants. Tally is a derived read over the vote set, computed locally.
- **Timer:** `{timer_id, duration, remaining, state (idle | running | paused | done), audio_enabled}`. `duration` contract: hard_min 1; hard_max NOT DECLARED IN SOURCE (Studio declares 86400); soft_min 30; soft_max 3600; default 300; unit seconds; precision 0. Timer state is session state, not document authority.
- **Cursor chat:** a transient message attached to a live cursor; it is presence state and MUST NOT be persisted to document authority.
- **Spotlight / follow:** `{presenter_actor, followers[]}` over the presence and viewport surfaces of 14.17. Ending spotlight releases every follower's viewport.

**[STU-WB-122] Timer and presence events.** A model or plugin may subscribe to timer lifecycle events; the closed set is `timer_start`, `timer_stop`, `timer_pause`, `timer_resume`, `timer_done`, `timer_adjust`, and it is the same set declared in [STU-AUT-158]. There is no board-private event bus.

**[STU-WB-123] Collaboration substrate.** [STU-WB-003] stands: board editing operates over the CRDT substrate with attributable, receipted, undoable changes under the same parallel-workflow guarantees as the rest of Studio. Board nodes MUST merge without a board-specific conflict rule; where two lanes edit the same connector endpoint or the same table cell, the ordinary record-granular expected-revision precondition of [STU-SDB-004] applies.

---

### 3. Templates, generation and interop

**[STU-WB-124] Board template contract.** A board template MUST be stored as `{template_id, name, description, preview_asset_id, node_payload}` where `preview_asset_id` is a CKC asset link and never embedded bytes, extending the template surface required by [STU-WB-005]. Template distribution is local; there is no hosted marketplace dependency ([STU-AUT-020]).

**[STU-WB-125] Diagram generation contract.** [STU-WB-009] stands and is bounded: generation emits a `StudioCommandBatch` built through the command-builder pattern of [STU-AUT-124], so a generated diagram is ONE reviewable, promotion-gated, undoable proposal with one history entry, authored under a model `KernelActor`. A generation that produced 200 nodes MUST undo in one step. Model-driven board operations — summarise stickies, cluster, sort, ideate — are ordinary commands over board nodes and inherit the receipting and quiet-law guarantees of the rest of Studio.

**[STU-WB-126] Generation determinism.** Generation is a model-lane capability and is therefore not bit-deterministic across runs. That is acceptable BECAUSE the output is a reviewable proposal, not a direct mutation. What MUST be deterministic is the APPLICATION of an accepted proposal: applying the same proposal to the same document revision twice MUST produce identical state.

**[STU-WB-127] Board import and export.** [STU-WB-010] stands: board interop routes through 14.13. Boards export to raster, document and tabular forms through `StudioExportRecipe`; delimited data imports as stickies or table rows. The board round-trip target is matrix row 36, a local-copy NRT family; cloud sync for that family is an optional adapter ([STU-IO-013]), never a runtime dependency.

**[STU-WB-128] Tabular export mapping.** Exporting a board to a tabular form MUST declare its column mapping: at minimum node kind, node id, text content, section membership, author display name, colour, and vote tally where a voting session exists. The mapping is part of the recipe, not a hidden convention.

---

### 4. Declared gaps and obligations

**[STU-WB-129] DECLARED GAP — shape-type vocabulary is Studio-declared.** [STU-WB-105]'s diagram shape set was not recovered as an explicit enumeration from any capture. Studio declares its own set and labels it `studio_declared`. An implementer must not treat it as source-derived, and a later capture may extend it.

**[STU-WB-130] DECLARED GAP — presentation and slide node kinds are out of scope here.** The source object model additionally declares slide, slide-row, slide-grid and interactive-slide-element node kinds. Those belong to a presentation domain, which section 14 does not have. They are NOT folded into whiteboard mode, because a slide deck is not a board: it has ordered pages, a transition model and a presenter mode that the board surface does not. Their disposition is an open scope question for the operator, recorded here so it is not discovered late.

**[STU-WB-131] DECLARED GAP — no second corroborating capture.** Every board node kind in this module comes from one source object model. Unlike raster, vector, layout and colour, where several source applications corroborate each other, whiteboard has one witness. A field named here is exactly as reliable as that one capture, and the reliability is stated rather than implied.

**[STU-WB-132] Asset library binding.** Every board asset — stamp artwork, embedded media, unfurl thumbnails, template previews, washi-tape pattern sources — is a CKC placed-asset link per [STU-ASSET-005]. The board surface holds no media store, no thumbnail cache and no template asset folder of its own ([STU-ASSET-004]).

**[STU-WB-133] Contextual panel binding.** Every board object inspector — the sticky inspector, the shape inspector, the connector inspector, the table inspector, the code-block inspector, the section inspector, the widget inspector — MUST declare its binding through the contextual property-panel contract ([STU-WEB-030] through [STU-WEB-044]), with `binds_to` naming the board node kind and `document_types` restricted to the board document mode. Widget inspectors supplied by plugins declare a plugin-namespaced `author_id_prefix` ([STU-AUT-161]).

**[STU-WB-134] Command-surface obligation.** Every board node kind MUST have node-factory, read and mutate commands in the corpus of [STU-AUT-147], and each MUST satisfy [STU-CON-007] in full. Specifically, a model MUST be able to: create any of the twelve kinds; attach a connector between two named nodes with a named magnet; read the resolved geometry of a connector after an endpoint moves; read a voting tally; and capture the board region to bytes for visual verification — all without a foreground window and without screen-reading.

**[STU-WB-135] Validation descriptor set.** This sub-section contributes at minimum: `connector_endpoint_node_unresolvable`, `connector_endpoint_magnet_unknown`, `stuck_to_node_unresolvable`, `table_index_out_of_range`, `code_block_language_unknown`, `widget_plugin_not_installed`, `widget_synced_state_schema_violation`, `voting_session_votes_exceed_allowance`, `voting_anonymous_actor_projected`, `link_unfurl_network_capability_not_granted`, `embed_execution_capability_not_granted`, `board_export_column_mapping_missing`.

**[STU-WB-136] GUI / Argus / UserManual obligation.** [STU-WB-011] remains in force unchanged and additionally covers every record shape, enumeration, bound and command introduced by [STU-WB-100] through [STU-WB-135]. Every enumeration here MUST appear in the model-facing UserManual as its literal token list, and the twelve board node kinds MUST each carry an operator entry and a model entry.

---

### 5. Microtask Derivation

**[STU-WB-137] Derivation rule (NORMATIVE).** The whiteboard microtask set is derived from this module mechanically, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. Each numbered clause that states a **board node contract** ([STU-WB-102], [STU-WB-104], [STU-WB-105], [STU-WB-106], [STU-WB-110], [STU-WB-111], [STU-WB-112], [STU-WB-113], [STU-WB-114], [STU-WB-115], [STU-WB-116], [STU-WB-117], [STU-WB-118], [STU-WB-119]), a **closed enumeration** ([STU-WB-103], [STU-WB-107], [STU-WB-109], [STU-WB-122]), an **endpoint variant set** ([STU-WB-108]), a **facilitation record** ([STU-WB-121]), a **template or generation contract** ([STU-WB-124], [STU-WB-125], [STU-WB-126], [STU-WB-128]), or a **command-surface guarantee** ([STU-WB-134]), where that clause can be implemented and proven independently of its siblings.
2. Each **validation-descriptor clause** in sub-section 6, [STU-WB-142] through [STU-WB-153]. Each of the 12 descriptors named in [STU-WB-135] is stated as its own clause precisely so it yields its own microtask: a check is a unit of implementable, independently provable work, and one microtask reading "implement 12 checks" is not implementable by the small models these contracts are sized for. A descriptor list inside a single clause, whether as prose or as a table, is one unit to any derivation tool and therefore loses 11 units of real work.
3. Each **declared gap** — in this module exactly three, [STU-WB-129], [STU-WB-130] and [STU-WB-131]. Each yields a microtask under [STU-WB-138], not nothing.

No other unit yields a microtask. Exactly 8 clauses in this module yield nothing, and they are:

- **Baseline, scope-fence and supersession clauses** — [STU-WB-100] and [STU-WB-101], which sit under the bookkeeping heading `0. Baseline, supersession and disposition`. These are discharged when the v02.206 bundle lands, not by a work packet.
- **Pure pointer clauses** — [STU-WB-120]. Each restates a clause that already carries the contract; the microtask lives there.
- **This derivation sub-section itself** — its five clauses yield nothing.

Every other clause yields at least one unit. This list is the module's declared non-yielding set and is the authority a derivation tool reconciles against.

**[STU-WB-138] Open items and blocked dependencies.** This module declares three, and every one of them YIELDS a microtask whose FIRST acceptance criterion is resolving the named dependency:

| Declared gap | Clause | First acceptance criterion of its microtask |
|---|---|---|
| The diagram shape-type vocabulary was never recovered as an explicit value union | [STU-WB-129] | Either recover the union from a capture and replace the Studio-declared set, or record operator acceptance of the twelve Studio-declared shapes and remove the `studio_declared` caveat. The clause is amended either way before any shape-authoring microtask activates. |
| Presentation and slide node kinds exist in the capture but section 14 has no presentation domain | [STU-WB-130] | Obtain the operator's scope decision on whether a presentation domain is created, folded elsewhere, or declared out of scope, and record it. These four node kinds MUST NOT be silently absorbed into whiteboard mode. |
| The whole domain rests on one source object model with no corroborating capture | [STU-WB-131] | Either obtain a second capture that corroborates the twelve board node kinds and their fields, or record operator acceptance of the single-witness basis with its reliability stated in the UserManual. |

A declared gap MUST NOT be dropped from the yields index, because a gap that yields nothing disappears silently and is rediscovered at implementation time. The same rule governs any open item a later amendment introduces.

**[STU-WB-139] Microtask content obligation.** A microtask derived under [STU-WB-137] MUST carry into its own body: the clause anchor; the COMPLETE member list of every closed enumeration it touches, as literal tokens — all twelve board node kinds, all twelve connector stroke caps, all three connector line types, all three endpoint variants; the additional field set of the node kind it implements, distinguished from the shared `StudioLayer` surface it inherits; the CKC placed-asset binding of [STU-WB-132] where it touches bytes; and the `studio_declared` label of [STU-WB-105] where it touches the shape vocabulary. A microtask that says "implement connectors" without the twelve stroke caps and the three endpoint variants does not satisfy this clause.

**[STU-WB-140] Yields index (NORMATIVE).** The counts below are the derivation surface of this module under [STU-WB-137]. They are not estimates: they are the measured output of applying that rule to this module's text, and every row states which unit kinds it contributes.

| Unit group | Clauses | Units by kind | Yields |
|---|---|---|---|
| Board mode and board nodes | [STU-WB-102]-[STU-WB-119] | 18 clause | 18 |
| Board structure and collaboration | [STU-WB-121]-[STU-WB-123] | 3 clause | 3 |
| Templates, generation and interop | [STU-WB-124]-[STU-WB-128] | 5 clause | 5 |
| Declared gaps and obligations | [STU-WB-129]-[STU-WB-136] | 8 clause | 8 |
| Validation Descriptor Catalogue | [STU-WB-142]-[STU-WB-153] | 12 validator | 12 |
| Clauses yielding nothing | 8 clauses, listed in [STU-WB-137] | — | 0 |
| **Module total** | | **54 clauses** | **46** |

Of this module's 54 clauses, 8 yield nothing and 46 yield at least one unit; tables inside yielding clauses contribute the remainder. The module total is **46**. The last numeric column is the yields count.

**[STU-WB-141] Anchor binding.** A microtask derived from this module cites its clause anchor directly. A microtask staged before this module landed carries `spec_anchor_status = "PROVISIONAL"`; binding it to an anchor in [STU-WB-100]–[STU-WB-153], or to a preserved v02.205 anchor in [STU-WB-001]–[STU-WB-011], clears that status. A microtask that cannot cite either is out of scope for the whiteboard domain and MUST be re-derived or retired, not activated.

---

### 6. Validation Descriptor Catalogue

Each descriptor below is its own clause because each is its own unit of implementable, independently provable work: feed the runtime a document that violates the rule and assert the check fires with the stated diagnostic. [STU-WB-135] names the set; the clauses in this sub-section state what each member catches, which clause it enforces, its severity, and what its diagnostic MUST name. Every one is a `StudioValidationDescriptor` in the catalogue of 14.24.

**[STU-WB-142] `connector_endpoint_node_unresolvable`.** The whiteboard validator MUST reject, with severity `warning`, a document or command in which a connector endpoint's `endpoint_node_id` resolves to no node, enforcing [STU-WB-108]. The diagnostic MUST name the connector and the endpoint; the connector transitions to a broken state retaining its last known position, and is never silently deleted.

**[STU-WB-143] `connector_endpoint_magnet_unknown`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which a magnet endpoint names an anchor outside the declared magnet set, enforcing [STU-WB-108]. The diagnostic MUST name the connector, the endpoint and the unknown magnet token.

**[STU-WB-144] `stuck_to_node_unresolvable`.** The whiteboard validator MUST reject, with severity `warning`, a document or command in which a stamp, highlight or washi-tape node's `stuck_to_node_id` resolves to no node, enforcing [STU-WB-118]. The diagnostic MUST name the node; it unsticks in place rather than disappearing.

**[STU-WB-145] `table_index_out_of_range`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which a board table row or column operation names an index outside the table's current bounds, enforcing [STU-WB-113]. The diagnostic MUST name the operation, the index and the table's dimensions.

**[STU-WB-146] `code_block_language_unknown`.** The whiteboard validator MUST reject, with severity `warning`, a document or command in which a board code block declares a language token outside the codegen language enumeration, enforcing [STU-WB-114]. The diagnostic MUST name the node and the token; the block renders unhighlighted rather than failing.

**[STU-WB-147] `widget_plugin_not_installed`.** The whiteboard validator MUST reject, with severity `warning`, a document or command in which a widget node names a `widget_id` for which no plugin is installed, enforcing [STU-WB-119]. The diagnostic MUST name the node and the widget id; the node renders as an inert placeholder retaining its synced state.

**[STU-WB-148] `widget_synced_state_schema_violation`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which a widget's synced state does not satisfy the schema its plugin declares, enforcing [STU-WB-119]. The diagnostic MUST name the widget, the key and the expected type.

**[STU-WB-149] `voting_session_votes_exceed_allowance`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which a participant's votes in one session exceed the session's `votes_per_participant`, enforcing [STU-WB-121]. The diagnostic MUST name the session and the participant count only; when the session is anonymous the participant is not named.

**[STU-WB-150] `voting_anonymous_actor_projected`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which an anonymous session projects `voter_actor` to participants rather than retaining it for audit only, enforcing [STU-WB-121]. The diagnostic MUST name the session; this is a privacy defect, not a display bug.

**[STU-WB-151] `link_unfurl_network_capability_not_granted`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which a link-unfurl node attempts a network fetch without a granted network capability, enforcing [STU-WB-116]. The diagnostic MUST name the node and the capability; the node renders as a plain link and says so.

**[STU-WB-152] `embed_execution_capability_not_granted`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which an embed node renders third-party interactive content without a granted execution capability, enforcing [STU-WB-116]. The diagnostic MUST name the node and the capability.

**[STU-WB-153] `board_export_column_mapping_missing`.** The whiteboard validator MUST reject, with severity `error`, a document or command in which a tabular board export declares no column mapping, enforcing [STU-WB-128]. The diagnostic MUST name the recipe; the mapping is part of the recipe, never a hidden convention.
