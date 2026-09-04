---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-11-prototyping"
section_id: "14.11"
title: "14.11 Prototyping & Interaction (prototyping half)"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
scope_split_note: "This module owns the PROTOTYPING half of 14.11 only. Motion timeline, keyframing, easing-curve authoring on the timeline, motion paths and animated export are owned by a separate module and are NOT restated here."
supersedes_section: "The prototyping clauses of 14.11 in .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md lines 2339-2554"
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-11-prototyping.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.11 Prototyping & Interaction (prototyping half)

This module is the normative field-level contract for Studio's interaction surface: the reaction record, the trigger vocabulary, the action vocabulary, transition and easing value shapes, overlay behaviour, scroll and overflow behaviour, runtime variable writes and conditionals, and presentation. It replaces the behavioural sketch in v02.205 [STU-PRO-001] through [STU-PRO-027] and [STU-PRO-043] with the enumerated tokens, record shapes and parameter bounds an implementer needs.

### 0. Scope disposition, baseline and supersession

**[STU-PRO-100] Scope boundary — motion is not in this module.** Clauses [STU-PRO-028] through [STU-PRO-032] (`StudioMotionTimeline`, keyframes, auto-keyframe recording, anchor points, timeline segment easing, on-canvas motion paths, preset animation styles, animated components, model-assisted keyframe generation) and [STU-PRO-044] / [STU-PRO-044a] / [STU-PRO-044b] (animated export, Lottie posture, motion-inspection projection) are the motion surface and are owned by a separate module. This module MUST NOT be read as retiring, amending or restating them. The one shared artefact between the two halves is the easing value shape of [STU-PRO-119]; the motion module is the owner of how easing is authored on a timeline segment, this module is the owner of how easing is stored on a transition.

**[STU-PRO-101] Baseline preservation and supersession.** Clauses [STU-PRO-001] through [STU-PRO-027] and [STU-PRO-034] through [STU-PRO-043c] of v02.205 remain in force unchanged, and [STU-PRO-033] is redefined at its own anchor in this module. Clauses [STU-PRO-100] and above add the missing field-level contract. Explicit corrections:

| v02.205 clause | Disposition | Replacement |
|---|---|---|
| [STU-PRO-004] trigger table (9 rows) | EXTENDED AND CORRECTED | The trigger enumeration is seventeen members ([STU-PRO-105]); the v02.205 table omitted the four controller-family members and conflated `ON_CLICK` with `MOUSE_UP`. |
| [STU-PRO-014] action table (10 rows) | EXTENDED AND CORRECTED | The action enumeration is seventeen members ([STU-PRO-110]) over five navigation sub-kinds ([STU-PRO-111]); media control is six sub-actions plus two skip forms plus one seek form, not one row. |
| [STU-PRO-018] transition list | RESTRUCTURED | Transitions are three record shapes ([STU-PRO-116] through [STU-PRO-118]), not one flat list. |
| [STU-PRO-019] easing catalog | RESTRUCTURED | Easing is a two-variant value shape ([STU-PRO-119]); the named presets are Studio-declared parameter sets over that shape ([STU-PRO-120]). |
| [STU-PRO-020a] overlay "default position presets" | ENUMERATED | Exactly eight positions plus manual ([STU-PRO-123]). |
| [STU-PRO-033] naming | REPLACED | The clause is redefined at its own anchor in this module, so the v02.205 wording does not reach the assembled bundle at all. Its force is unchanged: the object class is enumerated in [STU-PRO-166], no object kind is added or removed, and the source product name is recorded only in the provenance sidecar, per [STU-SECTION-003]. |
| [STU-PRO-021] overflow list | CONFIRMED and given its token set ([STU-PRO-125]). |

---

### 1. The reaction record

**[STU-PRO-102] Reaction is the single interaction record.** Every interaction on a `StudioLayer` is a REACTION: `{trigger, actions[]}`. `trigger` is a trigger record ([STU-PRO-106]) or null; `actions` is an ORDERED array of action records ([STU-PRO-112]). A null trigger means the reaction is authored but not armed; it MUST persist and MUST NOT fire.

**[STU-PRO-103] Reactions live on the node.** Any `StudioLayer` that can be a hotspot carries `reactions`: an ordered array of reaction records. Order is authority — it is the evaluation order when two reactions share a trigger. Studio MUST expose `node.set_reactions` as a typed command; because a reaction may reference a destination node in a not-yet-loaded page, the command MUST be declared asynchronous.

**[STU-PRO-104] Multiple actions per trigger.** A single reaction's `actions` array executes in order. Actions that mutate variables ([STU-PRO-113]) take effect before any later action in the same array reads them. A navigation action ends the array: actions after a `NODE`, `BACK` or `CLOSE` action in the same reaction MUST NOT execute, and authoring them is a validation warning.

---

### 2. Triggers

**[STU-PRO-105] Trigger type enumeration (normative, closed).** Exactly seventeen members:

| Token | Fires when | Auto-revert |
|---|---|---|
| `ON_CLICK` | primary pointer click, or touch tap, completes on the hotspot | no |
| `ON_DRAG` | the hotspot is dragged, with continuous movement mapping | no |
| `ON_HOVER` | the pointer is over the hotspot | yes, on exit |
| `ON_PRESS` | pointer or touch is held down on the hotspot | yes, on release |
| `MOUSE_ENTER` | the pointer enters the hotspot bounds | no |
| `MOUSE_LEAVE` | the pointer exits the hotspot bounds | no |
| `MOUSE_DOWN` | pointer press begins on the hotspot | no |
| `MOUSE_UP` | pointer press is released over the hotspot | no |
| `AFTER_TIMEOUT` | the frame-level dwell timer elapses | no |
| `KEYBOARD` | a declared key or key combination is input | no |
| `ON_KEY_DOWN` | a declared key transitions to pressed | no |
| `ON_MEDIA_HIT` | a media-bearing layer reaches a declared timestamp | no |
| `ON_MEDIA_END` | a media-bearing layer finishes playback | no |
| `XBOX_ONE` | a declared button on that controller family is input | no |
| `PS4` | a declared button on that controller family is input | no |
| `SWITCH_PRO` | a declared button on that controller family is input | no |
| `UNKNOWN_CONTROLLER` | a declared button on an unrecognised controller is input | no |

`ON_HOVER` and `ON_PRESS` are the auto-reverting pair; `MOUSE_ENTER`/`MOUSE_LEAVE` and `MOUSE_DOWN`/`MOUSE_UP` are the non-reverting pairs used to build explicit press state machines ([STU-PRO-010], [STU-PRO-011]).

**[STU-PRO-106] Trigger record shape.** A trigger record is `{type, ...type_specific}`:

- `AFTER_TIMEOUT` carries `timeout` — see [STU-PRO-107].
- `KEYBOARD`, `ON_KEY_DOWN` and the four controller members carry `device` (a token from the controller members plus `KEYBOARD`) and `key_codes`: an ordered array of integer key or button codes. An empty `key_codes` array is a validation error.
- `ON_MEDIA_HIT` carries `media_hit_time` — see [STU-PRO-108].
- `ON_DRAG` carries `delay` (see [STU-PRO-107]) and MAY carry a drag-axis constraint token `HORIZONTAL` \| `VERTICAL` \| `BOTH` (default `BOTH`).
- All other members carry no additional fields.

**[STU-PRO-107] Timeout / delay parameter contract.**

*Derivation: parameter table taken whole; yields 1 microtask whose acceptance criteria are its seven bound fields, each stored separately with unknown preserved.*
| Field | Value |
|---|---|
| hard_min | 0 |
| hard_max | NOT DECLARED IN SOURCE; Studio declares 3 600 000 (one hour) and MUST label the bound `studio_declared` |
| soft_min | 0 |
| soft_max | 10 000 |
| default | 800 |
| unit | milliseconds — Studio-declared. The source capture stores the field as a bare number with no unit token; Studio fixes the unit here so no implementer has to guess it. |
| precision | 0 decimal places (integer milliseconds) |
| step / coarse_step / fine_step | 10 / 100 / 1 |

**[STU-PRO-108] Media timestamp parameter contract.**

*Derivation: parameter table taken whole; yields 1 microtask whose acceptance criteria are its seven bound fields, each stored separately with unknown preserved.*
| Field | Value |
|---|---|
| hard_min | 0 |
| hard_max | the duration of the referenced media, resolved at bind time; a value beyond it is a validation error, not a clamp |
| soft_min | 0 |
| soft_max | the referenced media duration |
| default | 0 |
| unit | milliseconds — Studio-declared, as [STU-PRO-107] |
| precision | 0 decimal places |
| step / coarse_step / fine_step | 100 / 1000 / 1 |

**[STU-PRO-109] Touch mapping.** On a touch surface, `ON_CLICK` maps to tap, `MOUSE_DOWN` to touch-down, `MOUSE_UP` to touch-up, `ON_PRESS` to tap-and-hold, and `ON_DRAG` to swipe. `ON_HOVER`, `MOUSE_ENTER` and `MOUSE_LEAVE` have no touch equivalent and MUST be reported as inert in the running prototype's capability receipt rather than silently ignored.

---

### 3. Actions

**[STU-PRO-110] Action type enumeration (normative, closed).** Exactly seventeen members: `BACK`, `CLOSE`, `NODE`, `URL`, `SET_VARIABLE`, `SET_VARIABLE_MODE`, `CONDITIONAL`, `UPDATE_MEDIA_RUNTIME`, `PLAY`, `PAUSE`, `TOGGLE_PLAY_PAUSE`, `MUTE`, `UNMUTE`, `TOGGLE_MUTE_UNMUTE`, `SKIP_FORWARD`, `SKIP_BACKWARD`, `SKIP_TO`. The nine media members from `PLAY` onward are the legal values of the `media_action` field inside an `UPDATE_MEDIA_RUNTIME` record ([STU-PRO-114]); they are enumerated at the top level because the model command surface addresses them directly.

**[STU-PRO-111] Navigation sub-kind enumeration (normative, closed).** An action of type `NODE` carries `navigation`, exactly one of five members:

| Token | Behaviour | Pushes history |
|---|---|---|
| `NAVIGATE` | replaces the top-level frame with the destination | yes |
| `CHANGE_TO` | switches an instance, including a nested one, to another variant in place ([STU-DS-109]) | no |
| `OVERLAY` | layers the destination frame above the current frame | no, unless the overlay declares otherwise |
| `SWAP` | replaces the currently open overlay with the destination | no |
| `SCROLL_TO` | scrolls the viewport or the nearest scroll container to the destination | no |

**[STU-PRO-112] Action record shapes (normative).**

| Type | Required fields | Optional fields |
|---|---|---|
| `BACK` | none | none |
| `CLOSE` | none | none |
| `URL` | `url` (string) | `open_in_new_tab` (boolean, default true) |
| `NODE` | `destination_id` (node id or null), `navigation` ([STU-PRO-111]), `transition` ([STU-PRO-116]) or null, `preserve_scroll_position` (boolean, default false) | `overlay_relative_position` `{x, y}` (required when `navigation = OVERLAY` and the destination frame's `overlay_position_type = MANUAL`), `reset_video_position`, `reset_scroll_position`, `reset_interactive_components` (all boolean, default false) |
| `SET_VARIABLE` | `variable_id` (`SVAR-*`), `variable_value` (a value record per [STU-DS-127]) | none |
| `SET_VARIABLE_MODE` | `variable_collection_id` (`SVCL-*`), `variable_mode_id` | none |
| `CONDITIONAL` | `conditional_blocks` (ordered array of `{condition?, actions[]}`) | none |
| `UPDATE_MEDIA_RUNTIME` | `media_action` (one of the nine media members), `destination_id` (node id or null; null means "the media layer this reaction is authored on") | `amount_to_skip` (required for `SKIP_FORWARD` / `SKIP_BACKWARD`), `new_timestamp` (required for `SKIP_TO`) |

**[STU-PRO-113] Variable-write semantics.** `SET_VARIABLE` writes into the RUNNING prototype's variable state, never into document authority. Prototype runtime variable state is separate from the `values_by_mode` authority of [STU-DS-121] and is discarded when the prototype session ends, unless the flow's state-preservation setting says otherwise ([STU-PRO-003]). A `SET_VARIABLE` whose `variable_value` is an `EXPRESSION` record evaluates that expression against the current runtime state using the closed function set of [STU-DS-131].

**[STU-PRO-114] Media action semantics and parameter contract.**

- `PLAY`, `PAUSE`, `TOGGLE_PLAY_PAUSE`, `MUTE`, `UNMUTE`, `TOGGLE_MUTE_UNMUTE` take no numeric argument.
- `SKIP_FORWARD` and `SKIP_BACKWARD` take `amount_to_skip`: hard_min 0; hard_max the referenced media duration; soft_min 0; soft_max 60 000; default 5 000; unit milliseconds (Studio-declared per [STU-PRO-107]); precision 0; step/coarse/fine 1000/5000/100.
- `SKIP_TO` takes `new_timestamp`, contract identical to [STU-PRO-108].

**[STU-PRO-115] Conditional block record.** A `CONDITIONAL` action's `conditional_blocks` is an ORDERED array of `{condition?, actions[]}`. `condition` is a value record ([STU-DS-127]) that MUST resolve to `BOOLEAN`. The first block whose condition resolves true executes its `actions` array; a trailing block with `condition` absent is the else branch. Evaluation is deterministic and side-effect-scoped to variable writes and navigation ([STU-PRO-026]); expression and alias cycles are validation errors ([STU-DS-131]).

---

### 4. Transitions and easing

**[STU-PRO-116] Transition value shape (normative).** A transition on a `NODE` action is one of exactly three record shapes, discriminated by `type`:

- **Instant:** the transition field is null. No animation.
- **Simple transition** ([STU-PRO-117]).
- **Directional transition** ([STU-PRO-118]).

**[STU-PRO-117] Simple transition record.** `{type, easing, duration}` where `type` ∈ {`DISSOLVE`, `SMART_ANIMATE`, `SCROLL_ANIMATE`}. `DISSOLVE` is the cross-fade of [STU-PRO-018]. `SMART_ANIMATE` is the layer-matching tween; matching is by (layer name, position in the child hierarchy) — the SAME pairing rule as instance override matching ([STU-DS-119]) — and it tweens position, size, rotation, opacity and fill, fading unmatched layers. `SCROLL_ANIMATE` eases a scroll-position change rather than a frame change.

**[STU-PRO-118] Directional transition record.** `{type, direction, match_layers, easing, duration}` where `type` ∈ {`MOVE_IN`, `MOVE_OUT`, `PUSH`, `SLIDE_IN`, `SLIDE_OUT`} and `direction` ∈ {`LEFT`, `RIGHT`, `TOP`, `BOTTOM`}. `match_layers` (boolean, default false) applies the smart-animate pairing of [STU-PRO-117] on top of the directional move.

**[STU-PRO-119] Easing value shape (normative, closed).** Easing is `{type, easing_function_cubic_bezier?, easing_function_spring?}`:

- `easing_function_cubic_bezier` is `{x1, y1, x2, y2}` — four numbers, the two control points of a cubic bezier. Contract per component: hard_min for `x1`/`x2` is 0 and hard_max 1 (x must stay in the unit interval or the curve is not a function); hard_min/hard_max for `y1`/`y2` are NOT DECLARED IN SOURCE and Studio declares -10 and 10 so overshoot is expressible ([STU-PRO-019e]); soft range 0..1 on all four; default `{0, 0, 1, 1}` (linear); unit dimensionless; precision 4 decimal places; step/coarse/fine 0.01/0.1/0.001.
- `easing_function_spring` is `{mass, stiffness, damping, initial_velocity}` — four numbers. Contract: `mass` hard_min > 0, Studio-declared hard_max 100, soft 0.1..10, default 1, precision 3; `stiffness` hard_min > 0, Studio-declared hard_max 10000, soft 1..1000, default 100, precision 2; `damping` hard_min 0, Studio-declared hard_max 1000, soft 0..100, default 15, precision 2; `initial_velocity` hard bounds NOT DECLARED IN SOURCE, Studio declares -1000..1000, soft -10..10, default 0, precision 3. All four are dimensionless.

Exactly one of the two function records is present. Both absent, or both present, is a validation error.

**[STU-PRO-120] Named easing presets are parameter sets, not a separate mechanism.** The named presets of [STU-PRO-019] — Linear, Ease In, Ease Out, Ease In And Out, Ease In Back, Ease Out Back, Ease In And Out Back, and the four springs Gentle / Quick / Bouncy / Slow — are Studio-declared named parameter sets over [STU-PRO-119]. Studio MUST publish the exact four-number tuple behind each named preset in the UserManual and MUST store the tuple, not the name, on the transition record, so a document round-trips without depending on Studio's preset table. `Hold` (a step function that jumps to the final value) is a THIRD easing kind used only on timeline segments and is owned by the motion module; it MUST NOT appear on a transition record.

**[STU-PRO-121] Duration parameter contract.**

*Derivation: parameter table taken whole; yields 1 microtask whose acceptance criteria are its seven bound fields, each stored separately with unknown preserved.*
| Field | Value |
|---|---|
| hard_min | 0 |
| hard_max | NOT DECLARED IN SOURCE; Studio declares 30 000 and labels it `studio_declared` |
| soft_min | 0 |
| soft_max | 3 000 |
| default | 300 |
| unit | milliseconds — Studio-declared. The source capture stores `duration` as a bare number with no unit token. |
| precision | 0 decimal places |
| step / coarse_step / fine_step | 10 / 100 / 1 |

A spring easing MAY declare `duration` as derived rather than authored; when it does, the stored `duration` is the settling time computed from the spring parameters and MUST be recomputed on any spring-parameter change.

---

### 5. Overlays

**[STU-PRO-122] Overlay fields live on the destination frame.** A frame that can be opened as an overlay carries `overlay_position_type`, `overlay_background`, `overlay_background_interaction`. These are frame-level, not action-level; the action supplies only `overlay_relative_position` when the type is `MANUAL`.

**[STU-PRO-123] Overlay position enumeration (normative, closed).** Exactly nine values of `overlay_position_type`: `CENTER`, `TOP_LEFT`, `TOP_CENTER`, `TOP_RIGHT`, `BOTTOM_LEFT`, `BOTTOM_CENTER`, `BOTTOM_RIGHT`, `MANUAL`, and the implicit unset state which resolves to `CENTER`. `MANUAL` requires `overlay_relative_position = {x, y}` on the action, in document units ([STU-DOC-003]), precision 2.

**[STU-PRO-124] Overlay background enumeration (normative, closed).** `overlay_background` is exactly `{type: "NONE"}` or `{type: "SOLID_COLOR", color}` where `color` is an RGBA value with an explicit `StudioColorProfile` reference ([STU-DOC-003]); its alpha is the background opacity of [STU-PRO-020b]. `overlay_background_interaction` is exactly `NONE` or `CLOSE_ON_CLICK_OUTSIDE`.

---

### 6. Scroll, overflow and fixed positioning

**[STU-PRO-125] Overflow direction enumeration (normative, closed).** Exactly four members of `overflow_direction`: `NONE`, `HORIZONTAL`, `VERTICAL`, `BOTH`. Default `NONE`.

**[STU-PRO-126] Fixed and sticky children.** A frame carries `number_of_fixed_children`: a non-negative integer naming how many of its FIRST children, in child order, are viewport-fixed rather than scrolling ([STU-PRO-022]). Contract: hard_min 0; hard_max the frame's child count; soft_min 0; soft_max 8; default 0; unit count; precision integer. Sticky positioning is expressed as a fixed child whose `layout_positioning` is `ABSOLUTE` with a top constraint; Studio MUST expose it as a distinct authored state and MUST record which mechanism produced it in the export receipt, because HTML export ([STU-WEB-120]) maps the two to different CSS.

**[STU-PRO-127] Scroll preservation.** `preserve_scroll_position` on a `NODE` action carries the origin frame's scroll offset onto the destination ([STU-PRO-023]). It is legal only when `navigation = NAVIGATE`; on any other sub-kind it is a validation warning and is ignored at runtime.

---

### 7. Flow, state preservation and presentation

**[STU-PRO-128] Flow starting point record.** A page MUST carry `flow_starting_points`: an ordered array of `{node_id, name}`. Order is authority and drives the flow sidebar order ([STU-PRO-002]). A node id appearing twice is a validation error.

**[STU-PRO-129] Prototype background.** A page carries `prototype_backgrounds`: an ordered array of paint records rendered behind the running prototype, and `prototype_start_node_id` naming the default entry frame.

**[STU-PRO-130] State-preservation matrix.** The flow-level state-preservation setting of [STU-PRO-003] resolves independently for three state classes, and Studio MUST expose all three:

| State class | Preserved on `NAVIGATE` | Reset by |
|---|---|---|
| interactive-component variant state | per setting, default reset | `reset_interactive_components` on the action |
| runtime variable values | per setting, default preserved | an explicit `SET_VARIABLE` action |
| scroll offset and media position | per setting, default reset | `reset_scroll_position` / `reset_video_position` on the action |

**[STU-PRO-131] Navigation history.** The running prototype maintains a stack. `NAVIGATE` pushes; `BACK` pops; `CHANGE_TO`, `SCROLL_TO`, `OVERLAY` and `SWAP` do not push ([STU-PRO-003a]). `BACK` on an empty stack is a no-op and MUST emit a runtime diagnostic rather than an error.

**[STU-PRO-132] Presentation surfaces.** [STU-PRO-024] through [STU-PRO-024c] remain in force. Additionally: the presentation view's zoom modes are exactly `FIT`, `FILL`, `ACTUAL_SIZE` and `SCALE_DOWN_TO_FIT`; the device-frame catalog entry is `{device_id, display_name, screen_width, screen_height, device_pixel_ratio, chrome_asset_id}` where `chrome_asset_id` is a CKC placed-asset link per [STU-ASSET-005] and NOT an embedded image.

---

### 8. Interactive components

**[STU-PRO-133] Interactive-component contract.** A reaction authored on a variant member of a `component_set` ([STU-DS-102]) whose action is `NODE`/`CHANGE_TO` targeting a sibling member runs automatically inside every instance of that set, with no per-screen wiring ([STU-PRO-027]). The instance's current variant is runtime state, not an override record; it MUST NOT be written into the instance's `overrides` array.

**[STU-PRO-134] Nested instance targeting.** A `CHANGE_TO` action MAY target a NESTED instance by its path from the outer instance. The path is expressed as an ordered array of node ids, resolved through the same name-and-hierarchy pairing as [STU-DS-119]. A path that fails to resolve at runtime is a runtime diagnostic naming the first unresolvable segment, not a silent no-op.

---

### 9. Interactive documents (authoring side)

**[STU-PRO-033] Interactive-document objects.** Studio MUST support, on `StudioLayer` nodes, the interactive-document object class that a professional page-layout and publishing tool exposes, for interactive PDF, fixed-layout EPUB, reflowable EPUB, and HTML output. These objects are authored with the same selection, override, and history surfaces as all other Studio primitives; buttons, form fields, and multi-state objects are interactive ROLES on layers, not a separate document silo and not a parallel document model. The class is exactly the eight object kinds enumerated in [STU-PRO-166], each already obliged in detail by its own sibling clause in this group; that enumeration adds no object kind this clause did not already require, and narrows nothing it requires.

This clause is REDEFINED at its own anchor in the v02.206 bundle. The v02.205 text stated the identical obligation but named the object class after a source application, which [STU-SECTION-003] forbids: a source product name is never a Studio tool, command, panel or manual name. Redefining the clause here, rather than correcting it from a neighbouring clause, is what removes that name from the assembled document, because the assembler carries forward verbatim any anchor no staged module redefines. The product whose interactive-document surface is the provenance for this class is recorded in this module's provenance sidecar under this anchor and appears nowhere in any obligation.

**[STU-PRO-135] Interactive-document clauses retained.** [STU-PRO-034] through [STU-PRO-042a] (buttons, PDF form fields, multi-state objects, interactive-document animation triggers, timing, media placement, hyperlinks, bookmarks, QR codes, page transitions, interactivity preview) remain in force unchanged. [STU-PRO-033] is REDEFINED at its own anchor above, stating the identical obligation without naming a source product, and [STU-PRO-166] enumerates the object class it requires. This module otherwise adds only their event-model binding:

**[STU-PRO-136] Interactive-document event model.** When an interactive document targets HTML or EPUB output, its button and object events resolve against the web event model of [STU-WEB-110]: eighteen DOM event names over seventy-nine element rows, each row carrying an ordered event list and an optional default event. The interactive-document event names of [STU-PRO-034a] through [STU-PRO-034c] MUST each declare their mapping onto that model, and an event with no mapping for the target element MUST be reported in the export receipt as unsupported rather than emitted.

**[STU-PRO-137] Interactive-document behaviour parameters.** A behaviour attached to an interactive-document object is a `{behavior_id, group, display_title, safe_in_templates, parameters{}}` record whose parameter set is declared, typed and enumerated per behaviour. `safe_in_templates` is a required boolean: a behaviour that is not template-safe MUST be refused when the target region is inside a template-locked region ([STU-WEB-095]) with a typed error naming the region. The field-level basis and the shipped behaviour catalogue are specified in [STU-WEB-105] through [STU-WEB-109].

---

### 10. Command, panel, asset and obligation bindings

**[STU-PRO-138] Required prototyping commands.** Studio MUST expose at minimum: `reaction.set` (asynchronous, per [STU-PRO-103]), `reaction.list_for_node`, `reaction.list_for_page` (the bulk-edit surface of [STU-PRO-002a]), `flow.add_starting_point`, `flow.remove_starting_point`, `flow.reorder_starting_points`, `overlay.configure` (the three frame-level fields of [STU-PRO-122]), `frame.set_overflow_direction`, `frame.set_number_of_fixed_children`, `prototype.start_session`, `prototype.dispatch_trigger` (fire a named trigger on a named node in a running session — this is what makes a prototype model-testable without synthetic input), `prototype.read_runtime_variables`, `prototype.capture_frame` (render the current prototype viewport to bytes), and `prototype.end_session`.

**[STU-PRO-139] Model-steerability of the running prototype.** `prototype.dispatch_trigger` and `prototype.capture_frame` together satisfy [STU-CON-007]'s visual-verifiability requirement for this domain: a model MUST be able to drive a prototype to any reachable state and observe the result without a foreground window, without synthetic OS input, and without screen-scraping. A prototype session MUST be addressable by session id so multiple model lanes can run independent sessions over the same document concurrently.

**[STU-PRO-140] Determinism of the prototype runtime.** Given the same document state, the same starting point, and the same ordered trigger sequence, a prototype session MUST reach byte-identical runtime state. Timer-driven triggers (`AFTER_TIMEOUT`) MUST be drivable from a virtual clock supplied by the session so a deterministic replay does not depend on wall-clock timing. A prototype that cannot be replayed deterministically fails [STU-CON-007] and is not admissible.

**[STU-PRO-141] Contextual panel binding.** The flow editor, the trigger inspector, the action inspector, the overlay controls, the scroll and fixed-children controls, the device and presentation controls, and every interactive-document object editor MUST declare their binding through the contextual property-panel contract ([STU-WEB-030] through [STU-WEB-044]). Selecting a hotspot layer MUST resolve the interaction inspector by declared binding and priority, not by a hard-coded switch on layer kind.

**[STU-PRO-142] Asset library binding.** Every external byte stream referenced by this domain — device-frame chrome, placed video and audio for media triggers and media actions, QR code target payload attachments, and the poster frame of [STU-PRO-041] — MUST be a CKC placed-asset link per [STU-ASSET-005]. Prototyping MUST NOT maintain its own media store.

**[STU-PRO-143] Validation descriptor set.** This module contributes at minimum: `reaction_trigger_key_codes_empty`, `reaction_actions_after_terminal_navigation`, `action_destination_unresolvable`, `overlay_manual_position_missing`, `overlay_position_on_non_overlay_navigation`, `preserve_scroll_on_non_navigate`, `easing_both_functions_present`, `easing_no_function_present`, `media_timestamp_exceeds_duration`, `conditional_condition_not_boolean`, `conditional_expression_cycle`, `variant_target_not_sibling`, `nested_instance_path_unresolvable`, `flow_starting_point_duplicate`, `fixed_children_exceeds_child_count`.

**[STU-PRO-144] Export touchpoint.** [STU-PRO-043], [STU-PRO-043a], [STU-PRO-043b] and [STU-PRO-043c] remain in force: all interactive output is produced through `StudioExportRecipe` in 14.13. The interactive-PDF, EPUB and HTML option surfaces named there bind to the export parameter model of [STU-IO-100] and, for HTML, to the web output contract of [STU-WEB-120].

**[STU-PRO-145] GUI / Argus / UserManual obligation.** [STU-PRO-045] remains in force unchanged and additionally covers every field, enumeration, bound and command introduced by [STU-PRO-100] through [STU-PRO-144], EXCLUDING the motion clauses named in [STU-PRO-100], whose obligation is carried by the motion module. Every enumeration here MUST appear in the model-facing UserManual as its literal token list.

---

### 11. Microtask Derivation

**[STU-PRO-146] Derivation rule (NORMATIVE).** The prototyping microtask set is derived from this module mechanically, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. Each numbered clause that states an **interaction record shape** ([STU-PRO-102], [STU-PRO-103], [STU-PRO-106], [STU-PRO-112], [STU-PRO-115], [STU-PRO-116], [STU-PRO-117], [STU-PRO-118], [STU-PRO-119], [STU-PRO-122], [STU-PRO-128], [STU-PRO-129]), a **closed enumeration** ([STU-PRO-105], [STU-PRO-110], [STU-PRO-111], [STU-PRO-123], [STU-PRO-124], [STU-PRO-125]), a **parameter contract table** ([STU-PRO-107], [STU-PRO-108], [STU-PRO-114], [STU-PRO-121], [STU-PRO-126]), a **runtime semantic** ([STU-PRO-104], [STU-PRO-109], [STU-PRO-113], [STU-PRO-120], [STU-PRO-127], [STU-PRO-130], [STU-PRO-131], [STU-PRO-133], [STU-PRO-134], [STU-PRO-139], [STU-PRO-140]), or a **required command set or presentation surface** ([STU-PRO-132], [STU-PRO-136], [STU-PRO-137], [STU-PRO-138]), where that clause can be implemented and proven independently of its siblings.
2. Each **validation-descriptor clause** in sub-section 12, [STU-PRO-151] through [STU-PRO-165]. Each of the 15 descriptors named in [STU-PRO-143] is stated as its own clause precisely so it yields its own microtask: a check is a unit of implementable, independently provable work, and one microtask reading "implement 15 checks" is not implementable by the small models these contracts are sized for. A descriptor list inside a single clause, whether as prose or as a table, is one unit to any derivation tool and therefore loses 14 units of real work.

No other unit yields a microtask. Exactly 7 clauses in this module yield nothing, and they are:

- **Baseline, scope-fence and supersession clauses** — [STU-PRO-100] and [STU-PRO-101], which sit under the bookkeeping heading `0. Scope disposition, baseline and supersession`. These are discharged when the v02.206 bundle lands, not by a work packet.
- **This derivation sub-section itself** — its five clauses yield nothing.

Every other clause yields at least one unit. This list is the module's declared non-yielding set and is the authority a derivation tool reconciles against.

**[STU-PRO-147] Open items and blocked dependencies.** This module declares no open item and no BLOCKED dependency. Four parameter contracts ([STU-PRO-107], [STU-PRO-108], [STU-PRO-114], [STU-PRO-121]) rest on a Studio-declared millisecond unit because the source capture declares none; that is a RESOLVED decision recorded in the clause, not an open item, and it yields an ordinary microtask. Should a later amendment introduce a genuine open item or a BLOCKED dependency, that clause STILL yields a microtask, and that microtask's FIRST acceptance criterion MUST be resolving the named dependency. A declared gap MUST NOT be dropped from the yields index, because a gap that yields nothing disappears silently.

**[STU-PRO-148] Microtask content obligation.** A microtask derived under [STU-PRO-146] MUST carry into its own body: the clause anchor; the COMPLETE member list of every closed enumeration it touches, as literal tokens — all seventeen triggers, all seventeen actions, all five navigation sub-kinds, all nine overlay positions; the full seven-field parameter record of every numeric parameter it touches, with every `NOT DECLARED IN SOURCE` and `studio_declared` label preserved verbatim, including the millisecond-unit declaration; and the determinism obligation of [STU-PRO-140] where it touches the running prototype. A microtask that says "implement triggers" without the seventeen tokens does not satisfy this clause.

**[STU-PRO-149] Yields index (NORMATIVE).** The counts below are the derivation surface of this module under [STU-PRO-146]. They are not estimates: they are the measured output of applying that rule to this module's text, and every row states which unit kinds it contributes.

| Unit group | Clauses | Units by kind | Yields |
|---|---|---|---|
| The reaction record | [STU-PRO-102]-[STU-PRO-104] | 3 clause | 3 |
| Triggers | [STU-PRO-105]-[STU-PRO-109] | 5 clause, 2 parameter table | 7 |
| Actions | [STU-PRO-110]-[STU-PRO-115] | 6 clause, 1 enumeration | 7 |
| Transitions and easing | [STU-PRO-116]-[STU-PRO-121] | 6 clause, 1 parameter table | 7 |
| Overlays | [STU-PRO-122]-[STU-PRO-124] | 3 clause | 3 |
| Scroll, overflow and fixed positioning | [STU-PRO-125]-[STU-PRO-127] | 3 clause | 3 |
| Flow, state preservation and presentation | [STU-PRO-128]-[STU-PRO-132] | 5 clause, 1 enumeration | 6 |
| Interactive components | [STU-PRO-133]-[STU-PRO-134] | 2 clause | 2 |
| Interactive documents (authoring side) | [STU-PRO-033]-[STU-PRO-137] | 4 clause | 4 |
| Command, panel, asset and obligation bindings | [STU-PRO-138]-[STU-PRO-145] | 8 clause | 8 |
| Validation Descriptor Catalogue | [STU-PRO-151]-[STU-PRO-165] | 15 validator | 15 |
| Interactive-Document Object Class | [STU-PRO-166]-[STU-PRO-166] | 1 clause | 1 |
| Clauses yielding nothing | 7 clauses, listed in [STU-PRO-146] | — | 0 |
| **Module total** | | **68 clauses** | **66** |

Of this module's 68 clauses, 7 yield nothing and 61 yield at least one unit; tables inside yielding clauses contribute the remainder. The module total is **66**. The last numeric column is the yields count.

**[STU-PRO-150] Anchor binding.** A microtask derived from this module cites its clause anchor directly. A microtask staged before this module landed carries `spec_anchor_status = "PROVISIONAL"`; binding it to an anchor defined in this module — [STU-PRO-033], or [STU-PRO-100] through [STU-PRO-166] — clears that status. A microtask that cannot cite an anchor here, and is not derived from the motion module fenced out by [STU-PRO-100], is out of scope for the prototyping domain and MUST be re-derived or retired, not activated.

---

### 12. Validation Descriptor Catalogue

Each descriptor below is its own clause because each is its own unit of implementable, independently provable work: feed the runtime a document that violates the rule and assert the check fires with the stated diagnostic. [STU-PRO-143] names the set; the clauses in this sub-section state what each member catches, which clause it enforces, its severity, and what its diagnostic MUST name. Every one is a `StudioValidationDescriptor` in the catalogue of 14.24.

**[STU-PRO-151] `reaction_trigger_key_codes_empty`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a `KEYBOARD`, `ON_KEY_DOWN` or controller trigger declares an empty `key_codes` array, so it can never fire, enforcing [STU-PRO-106]. The diagnostic MUST name the reaction and its trigger type.

**[STU-PRO-152] `reaction_actions_after_terminal_navigation`.** The prototyping validator MUST reject, with severity `warning`, a document or command in which a reaction lists further actions after a `NODE`, `BACK` or `CLOSE` action, which can never execute, enforcing [STU-PRO-104]. The diagnostic MUST name the unreachable actions and their positions in the array.

**[STU-PRO-153] `action_destination_unresolvable`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a `NODE` action names a `destination_id` that resolves to no node in any loaded page, enforcing [STU-PRO-112]. The diagnostic MUST name the action, the destination id, and the page searched.

**[STU-PRO-154] `overlay_manual_position_missing`.** The prototyping validator MUST reject, with severity `error`, a document or command in which an action opens an overlay whose frame declares `overlay_position_type = MANUAL` but the action supplies no `overlay_relative_position`, enforcing [STU-PRO-123]. The diagnostic MUST name the action and the destination frame.

**[STU-PRO-155] `overlay_position_on_non_overlay_navigation`.** The prototyping validator MUST reject, with severity `warning`, a document or command in which an `overlay_relative_position` is supplied on a navigation sub-kind other than `OVERLAY`, where it has no effect, enforcing [STU-PRO-112]. The diagnostic MUST name the action and its navigation sub-kind.

**[STU-PRO-156] `preserve_scroll_on_non_navigate`.** The prototyping validator MUST reject, with severity `warning`, a document or command in which `preserve_scroll_position` is set on a navigation sub-kind other than `NAVIGATE`, where it is ignored at runtime, enforcing [STU-PRO-127]. The diagnostic MUST name the action and its sub-kind.

**[STU-PRO-157] `easing_both_functions_present`.** The prototyping validator MUST reject, with severity `error`, a document or command in which an easing value carries both a cubic-bezier and a spring function, so the curve is ambiguous, enforcing [STU-PRO-119]. The diagnostic MUST name the transition and both function records.

**[STU-PRO-158] `easing_no_function_present`.** The prototyping validator MUST reject, with severity `error`, a document or command in which an easing value carries neither function record, enforcing [STU-PRO-119]. The diagnostic MUST name the transition.

**[STU-PRO-159] `media_timestamp_exceeds_duration`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a media trigger timestamp, a `SKIP_TO` target, or a skip amount reaches beyond the referenced media's duration, enforcing [STU-PRO-108]. The diagnostic MUST name the value, the media layer and its resolved duration; the value MUST NOT be silently clamped.

**[STU-PRO-160] `conditional_condition_not_boolean`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a conditional block's `condition` resolves to a type other than `BOOLEAN`, enforcing [STU-PRO-115]. The diagnostic MUST name the condition, its resolved type, and the block index.

**[STU-PRO-161] `conditional_expression_cycle`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a conditional expression's arguments form a resolution cycle, enforcing [STU-PRO-115]. The diagnostic MUST name the cycle path.

**[STU-PRO-162] `variant_target_not_sibling`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a `CHANGE_TO` action targets a component that is not a member of the instance's own variant set, enforcing [STU-PRO-111]. The diagnostic MUST name the action, the instance, and the target component.

**[STU-PRO-163] `nested_instance_path_unresolvable`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a nested-instance path fails to resolve at runtime, enforcing [STU-PRO-134]. The diagnostic MUST name the first unresolvable path segment; the action MUST NOT silently no-op.

**[STU-PRO-164] `flow_starting_point_duplicate`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a page's `flow_starting_points` names the same `node_id` twice, enforcing [STU-PRO-128]. The diagnostic MUST name the duplicated node id and both array positions.

**[STU-PRO-165] `fixed_children_exceeds_child_count`.** The prototyping validator MUST reject, with severity `error`, a document or command in which a frame's `number_of_fixed_children` exceeds the number of children it actually has, enforcing [STU-PRO-126]. The diagnostic MUST name the frame, the declared count, and the real child count.

---

### 13. Interactive-Document Object Class

**[STU-PRO-166] Interactive-document object class enumerated.** [STU-PRO-033], as redefined in this module, obliges Studio to support the interactive-document object class that a professional page-layout and publishing tool exposes, and names that class by capability rather than after the application that inspired it, per [STU-SECTION-003]. This clause enumerates that class so no implementer has to infer its membership. It is exactly the eight object kinds below; each is already obliged in detail by the sibling clause its row names, so this enumeration adds no object kind that [STU-PRO-033] did not already require and removes none that it did.

*Derivation: contract table carried into this clause's own microtask as acceptance criteria; yields no microtask of its own, because every row's object kind is already obliged by the sibling clause named in that row and splitting the table would count each of them twice.*

| Interactive object kind | Already obliged by | What the object carries |
|---|---|---|
| Hyperlink | [STU-PRO-041a] | URL, file, email, page-with-zoom, text-anchor and shared-destination targets, with configurable appearance and highlight, plus automatic URL detection |
| Bookmark | [STU-PRO-041a] | a nested, sortable navigation entry emitted into PDF output |
| Generated code graphic | [STU-PRO-041a] | a QR code over web, text, SMS, email or business-card payloads, editable after generation |
| Button | [STU-PRO-034], [STU-PRO-034d] | Normal, Rollover and Click appearance states, hidden-until-triggered behaviour, the event set of [STU-PRO-034a] through [STU-PRO-034c], and the action groups of [STU-PRO-035] through [STU-PRO-037] |
| Form field | [STU-PRO-038], [STU-PRO-038a] | check box, combo box, list box, radio button, signature field and text field, each with the option set of [STU-PRO-038a] applied as its type allows |
| Multi-state object | [STU-PRO-039] | ordered states that add, reorder and delete, paste-into-state, reset-all, and hidden-until-triggered, driven by the button state actions |
| Object animation | [STU-PRO-040] through [STU-PRO-040c] | motion presets bound to event triggers, per-animation duration, play count, speed easing and animate-from/to properties, editable motion paths, and a timing surface |
| Embedded media | [STU-PRO-041] | video with poster frame, controller skin, play-on-load, loop and navigation points; and audio; both targetable by the media actions and media triggers |

Two surfaces in this group are NOT members of the class and are not object kinds: the per-page tab order of [STU-PRO-034e], which sequences keyboard focus ACROSS these objects, and the interactivity preview of [STU-PRO-042a], which exercises them. Page transitions ([STU-PRO-042]) are a property of a spread rather than an object on a layer, and are obliged there. Cross-references, index topics, book files and generated tables of contents are long-document constructs owned by the page-layout domain (14.6); they are NOT obliged by [STU-PRO-033] and this clause does not add them.

Every object kind above is authored through the same selection, override and history surfaces as every other Studio primitive: an interactive object is a ROLE carried on a `StudioLayer`, never a separate document silo and never a parallel document model. No Studio object kind, panel, command, event, action or manual entry derived from this clause carries a source product's name; the product whose interactive-document surface is the provenance for this class is recorded in this module's provenance sidecar under this anchor.
