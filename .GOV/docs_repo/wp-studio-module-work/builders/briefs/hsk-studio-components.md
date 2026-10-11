---
file_id: BUILDER-BRIEF-HSK-STUDIO-COMPONENTS
file_kind: builder_brief
updated_at: 2026-10-11T05:20:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-components
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-COMPONENTS` / key `components` / crate `hsk-studio-components`; foundation MT-35539 (test target `instance_resolution`, cases `variant_mode_override_resolution`, `alias_type_disjoint_stale_cancel`); write scope `handshake-studio-assets/crates/hsk-studio-components/**`. Files per MT: `src/instance_resolution.rs`, `src/constraints.rs`, `tests/instance_resolution.rs`.
- Responsibility (verbatim): "Components, variables, responsive constraints, prototype actions within same document; no separate Figma-like app."
- Foundation capability: "Apply real instance property/constraint/variable resolution in same Folio graph without separate document authority."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-layout`, `hsk-studio-observe`. Slice needs accord, folio, observe only; the `layout` edge is for text/frame reflow in a later capability (so this slice can start in wave 1; MT-35539 lists MT-35537 as dependency for the full foundation).
- Forbidden: handshake_core/native/SurrealDB; a second document, component database or private variable store (ENG-DAG-FORBID-003, shared_owners.mutation); prototype/temporal sampling (Motion), interaction widgets (Controls); egui/wgpu.
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `components-consumer` (`--descriptor`), 30 s `bounded_case`. Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`, 14.10 l.19658-20272)
- STU-DS-102..105 (l.19681+): three node kinds `component` (`SCMP-`), `component_set` (variant container; instantiating resolves to `default_variant_id`), `component_instance` (`SCIN-`; fields `instance_id`, `main_component_id`, `component_properties{type,value,bound_variable_id?}`, `overrides[{overridden_node_id, overridden_fields[]}]`, `scale_factor`, `exposed_instance_ids` read-only); component fields incl. `variant_properties`, `variant_group_properties`, `component_property_definitions`.
- STU-DS-115..120 (l.19776+): overridable field set is CLOSED (`characters, fills, strokes, stroke_weight, effects, visible, nested main_component_id, nested component_properties, opacity, corner_radius family, layout_sizing_horizontal/vertical`); precedence highest-first (1) local override (2) `component_properties` binding (3) main definition (4) inherited style/variable binding; ambiguous double source = validation error; overrides are never a flattened subtree copy; swap/variant switch re-matches by (layer name, hierarchy position) and REPORTS `overrides_dropped`.
- STU-DS-121..129 (l.19794+): `StudioVariable` (`SVAR-`, `resolved_type`, `values_by_mode` exactly one entry per collection mode, `scopes`), `StudioVariableCollection` (`SVCL-`, ordered `modes`, `default_mode_id`); mode count min 1, no invented ceiling.
- STU-DS-151/152 (l.19945+): constraints closed set `MIN|CENTER|MAX|STRETCH|SCALE` on both axes; default `{MIN, MIN}` for non-auto-layout children. STU-DS-161 (l.19989): external bytes are CKC links, never copied.
- STU-CON-015/021 (Folio single authority), STU-DOC-001 (`kind` open vocabulary incl. `component_instance`), STU-ARC-011 (disjoint-property footprints), STU-ARC-016. Accord already has `SCHEMA_STUDIO_COMPONENT/COMPONENT_INSTANCE/VARIABLE/VARIABLE_COLLECTION/STYLE_REGISTRY/AUTO_LAYOUT/CONSTRAINT` and valid DomainId prefixes `SCMP,SCIN,SVAR,SVCL,SSTY` (verified).
- Later scope: auto layout 14.10 §8 (l.19871+, STU-DS-130..150), layout grids, publishing/libraries (§7), style registry (§6), prototyping 14.11 (l.20273+, Motion), `_mt_plan_1011_designsys_proto.json` (77 MTs, MT-321 family).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, ~700-1000 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[4]` (MT-35539; "Resolve canonical component properties/variants, aliases/modes and responsive constraints into same Folio graph; property-granular overrides retain provenance and cycle refusal"; provider "NOT_INSPECTED; use existing canonical component mechanics"; cases above + `disjoint_property_concurrency`, `cancel_stale_prototype_action`; oracle "Original editable component fixtures with named variant/variable/layout/prototype interactions and independent typed property snapshots; native FIG acceptance separate"); visual-components-team `preparation.json` (components scope MT-33087..33367 + 35324..35342; spec209 units MT-36237..36240); MT-35539 steps: "variant then mode then per-instance override, retaining every field source; never mutate base component", "white/grey/black traversal for aliases; reject cycles/type mismatch/unsupported property, never substitute zero or drop fields", "one footprint proposal after full success; same-property stale refused, disjoint-property admissible"; `.GOV/reference/studio_app_feature_research/05-studio-primitive-map.md` (style registry/components), `21-figma-feature-map.md`.
Reuse (verified): `folio::{StudioLayer{layer_id,kind,payload,..}, Payload::PrimitiveReference{schema_id,object_id}, Resolver::primitive_layer_binding(kind,schema)}` - an instance layer is `kind:"component_instance"` + `PrimitiveReference{schema_id:"hsk.studio.component_instance@1", object_id:"SCIN-..."}` (the host resolver must answer `primitive_layer_binding`; provide a helper `instance_layer(..)->folio::StudioLayer`); `accord::{DomainId::parse/for_schema, SchemaId::parse, SCHEMA_STUDIO_COMPONENT_INSTANCE, SCHEMA_STUDIO_VARIABLE, Length/Unit::Points, CancellationToken}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. Records live in this crate as plain typed values (no new schema id minted); durable authority stays Folio/host (STU-CON-008).
1. Records (`instance_resolution.rs`): `ComponentDef{component_id, nodes:Vec<DefNode{node_id, name, parent:Option<String>, fields:BTreeMap<String, Value>}>, property_defs}`, `ComponentSet{component_id, default_variant_id, variant_group_properties:BTreeMap<String,Vec<String>>, members:Vec<ComponentDef /*each with variant_properties*/>}`, `Instance{instance_id, main_component_id:Option<String>, component_properties:BTreeMap<String,PropValue{ty,value,bound_variable_id:Option<String>}>, overrides:Vec<Override{node_id, field, value}>}`, `Variable{variable_id, resolved_type:VarType, values_by_mode:BTreeMap<String,Value /*may be Value::Alias(svar)*/>}`, `Collection{collection_id, modes:Vec<(mode_id,name)>, default_mode_id}`; `enum Value{Bool,Num(f64),Str,Color([f64;4]),Alias(String)}`; every id validated with `DomainId::parse` + prefix. SPEC GAP (UNVERIFIED): STU-DS-117 lists override records as `{overridden_node_id, overridden_fields[]}` without stating where override VALUES are stored; the slice keeps `(node_id, field, value)` in the record and flags this in the lane file for spec enrichment.
2. Resolution: `pub fn resolve_instance(inst:&Instance, defs:&DefRegistry, vars:&VarRegistry, mode:Option<&str>, base_revision:u64, cancel:&CancellationToken) -> Result<Resolution, ComponentsError>`: select variant (`select_variant(set, &assignment)`; set instantiation -> `default_variant_id`; no member -> `NoSuchVariant`), then for each requested `(node_id, field)` apply DS-116 precedence and record `Source::{Override,ComponentProperty,Definition,VariableMode{collection,mode},Style}`; ambiguous double-source -> `AmbiguousSource`; field not in the closed DS-115 set written as override -> `FieldNotOverridable`; unknown property -> `UnsupportedProperty` (never zero/drop); variable alias chains resolved with white/grey/black DFS (`AliasCycle`), alias target type != `resolved_type` -> `TypeMismatch`; mode defaults to the collection `default_mode_id`; value missing for a mode -> `MissingModeValue` (never zero).
3. `constraints.rs`: `pub enum ConstraintKind{Min,Center,Max,Stretch,Scale}`, `pub struct Constraints{horizontal, vertical}` (default Min/Min), `pub fn apply_constraints(child:RectPt, old_parent:(f64,f64), new_parent:(f64,f64), c:Constraints) -> Result<RectPt, ComponentsError>` implementing DS-151 per axis (MIN keeps left/top offset; MAX keeps right/bottom offset; CENTER keeps centre offset and size; STRETCH keeps both edge offsets (resizes); SCALE keeps x/width as the same fraction of the parent); all values points, finite and non-negative sizes else `InvalidGeometry`; hand-check: child x=10,w=20 in parent 100->200 wide: MIN -> x=10,w=20; MAX -> x=110,w=20; CENTER -> x=60,w=20; STRETCH -> x=10,w=120; SCALE -> x=20,w=40.
4. Footprint proposal: `ResolutionProposal{base_revision, fields:Vec<ResolvedField{node_id, field, value, source}>, read_footprint:Vec<(String,String)>, write_footprint:Vec<(String,String)>}` returned only after FULL success; `pub fn check_footprint(expected:&BTreeMap<(String,String),u64>, current:&BTreeMap<(String,String),u64>, write:&[(String,String)]) -> Result<(), ComponentsError::StaleProperty{node,field}>`: same-property revision mismatch refused, disjoint-property drift admissible (STU-ARC-011). Budget (`max_nodes`, `max_alias_depth`) -> `BudgetExceeded`; cancel per node -> `Canceled` with inputs unchanged (all inputs `&`, nothing mutated).
5. Observability: `DESCRIPTOR` + `report(...)`: `AliasCycle|TypeMismatch|UnsupportedProperty|FieldNotOverridable|AmbiguousSource|StaleProperty|NoSuchVariant`->`Failure(Validation)`, `BudgetExceeded`->`Failure(Unsupported)`, `Canceled`->`Canceled`, Ok->`Success`. Example `components-consumer`: variant width + mode colour + text override, then re-edit of the original property (MT consumer scenario).
Oracle note: not an Adobe row; original-app oracle = Figma-authored local component fixtures (variants/variables/modes/auto-layout) observed black-box with public docs only (native `.fig` import is `interop-fig`, blocked separately).

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- None for the slice. Candidate for the later auto-layout capability: `taffy` 0.14.0 (DioxusLabs, MIT, published 2026-08-24, flexbox/grid/block layout; verified crates.io) - evaluate against STU-DS-130..150 auto-layout parameter contract before adopting (spec parameters are Figma-style auto layout, taffy is CSS flex/grid: mapping gap UNVERIFIED); do NOT add now.
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-components --tests` (then `clippy`, `test --test instance_resolution`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/instance_resolution.rs">

## Tests (<=5, `bounded_case`; MT case names)
1. `variant_mode_override_resolution`: set with `size=small|large` members, instance picks `large`; variable `Brand/Primary` has modes light/dark; resolve colour in dark; a local override on `characters` wins; each result carries the right `Source`; base `ComponentDef` value equal before/after (never mutated).
2. `alias_type_disjoint_stale_cancel`: alias A->B->A -> `AliasCycle`; alias to a variable of another `resolved_type` -> `TypeMismatch`; stale same-property revision refused while a disjoint-property change is admitted; pre-cancelled token -> `Canceled`, no proposal.
3. `override_rules`: writing a non-DS-115 field -> `FieldNotOverridable`; two sources claiming one field -> `AmbiguousSource`; unknown property -> `UnsupportedProperty`; missing mode value -> `MissingModeValue` (not zero).
4. `constraints_hand_computed`: the five-kind table above on both axes plus default Min/Min and a negative/non-finite size -> `InvalidGeometry`.
5. `instance_layer_folio_shape`: `instance_layer(..)` builds a `folio::StudioLayer` that passes `folio::validate_document` when the caller resolver accepts `("component_instance","hsk.studio.component_instance@1")` and yields `Unavailable` with `Unresolved`.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No auto layout solver, layout grids, styles registry, library publish/subscribe, swap/variant-switch `overrides_dropped` receipts beyond a typed list (add next), exposed-instance projection, prototypes/reactions (14.11), Layout reflow integration, `.fig` mapping (interop-fig), Chronicle patch generation, GUI/Controls, host wiring.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
