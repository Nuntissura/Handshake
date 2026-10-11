---
file_id: BUILDER-BRIEF-HSK-STUDIO-COMPOSITE
file_kind: builder_brief
updated_at: 2026-10-11T04:25:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-composite
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-COMPOSITE` / key `composite` / crate `hsk-studio-composite`; foundation MT-35544 (test target `static_plan`, case `static_closure_no_temporal_provider`); write scope `handshake-studio-assets/crates/hsk-studio-composite/**`. Files per MT: `src/lower_plan.rs`, `src/evaluate_static.rs`, `tests/static_plan.rs`.
- Responsibility (verbatim): "Static render-plan evaluator consumes immutable snapshot; same semantics for previews/export. Temporal/effect providers attach through typed ports; no unconditional motion/pulse engine dependency in the static render closure."
- Foundation capability: "Build/evaluate real mixed-domain render plan against immutable revision using selected provider contracts."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `accord`, `folio`, `prism`, `pigment`, `nib`, `type`, `observe`. Slice needs accord, folio, pigment (geometry `Rect`/`Grid`), observe. NOT allowed: `render-cpu`, `render-gpu`, `motion`, `pulse`, `reel`, `score` (static closure must exclude temporal providers: STU-CMP-093).
- Forbidden: handshake_core/native/SurrealDB; GPU/codec/native handles in the port; a second graph database or saved plan authority (plans are disposable, STU-CMP-092/093).
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `composite-consumer` (`--descriptor`), 30 s `bounded_case`. Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-CMP-092 (l.29837): ONE authored Folio graph, two editing projections (layer view, node view) over the same records; semantic stack input order lives in edges; no second graph/parallel layer-order authority. STU-CMP-093 (l.29839): Composite lowers revision-bound immutable DISPOSABLE render plans; materialise post-mask/effect/transform/matte and per-effect output addresses, fanout, typed backdrop/parent/matte/effect/precomp dependencies; plans are not authoring authority; static rendering never pulls NLE/audio/native providers. STU-CMP-094: layer->graph->layer switching/reopen preserves authored records, IDs and rendering semantics.
- STU-CON-015/021 (l.22424+, 22455-22464): foundation descriptors are only `source` (value type image|vector|text, port `out`) and `composite` (input `inputs` image/many/`ordered_composite`, output `out` image/one); order = `input_ordinal` contiguous from 0; fanout allowed; render cycles invalid; unimplemented richer descriptors are UNAVAILABLE, never faked. STU-DOC-001: containment and render dependency are distinct relations.
- STU-COL-161..165 (l.10285+): compositor receives buffers in working/blending space; composite never converts profiles. STU-ARC-015 (visual receipt), STU-ARC-011 (revision footprints), STU-ARC-016.
- Later scope: masks/mattes/backdrops/effects/precomps/3D/time (STU-CMP-0xx, 14.27.1-7 l.29811+; 1159 primary MTs per visual-team inventory), STU-FX (14.9), Motion sampling (STU-MOT-*). MT plan domains: `_mt_plan_09_effects.json`, `_mt_plan_1011_designsys_proto.json`, `_mt_plan_04_raster.json`.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, ~600-900 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[5]` (MT-35544; spec_refs STU-CMP-092/093/094; algorithm "Lower Folio authored layer/operation graph into disposable revision-bound dependency plan; preserve semantic input order and explicit mask/matte/backdrop/effect addresses; evaluate immutable static snapshot"; cases `mask_matte_backdrop_order`, `layer_node_projection_same_authored_ids`, `fanout_cycle_refusal_cancel_stale`, `static_closure_no_temporal_provider`; oracle "Independent original layer/matte/effect outputs plus authored-record equality across view switch/edit/reopen; self-render is component evidence only"); `.GOV/reference/studio_app_feature_research/13-layer-graph-vertical-slice.md` (state model, fixtures lg-001 stack order, lg-004 locked denial; "visual-order proof"); `05-studio-primitive-map.md`; Craft `photocraft/crates/compose` = reference only.
Reuse (verified by grep): `folio::{Snapshot (document(), ordered_inputs(&PortAddress)->Vec<&PortAddress>, require_available()), StudioDocument{revision, layers, graph, working_space_rgb, blending_space}, StudioLayer{layer_id, kind, parent, payload, visible}, Payload::{Group,Raster{tiles},Vector,Text,PrimitiveReference,Unsupported}, StudioLayerGraph{operations, edges, outputs}, Operation{layer_id, operation_key, operation_kind: Source|Composite, inputs, outputs}, PortAddress{layer_id, operation_key, port_key}, Edge{source,target,dependency_kind,input_ordinal}, ValueType::{Image,Vector,Text}, TileRef{column,row,width,height,content_digest,...}, ProfileBinding}`; `pigment::{Rect, Grid}` for extents; `accord::{CancellationToken, DomainId}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`.
OPEN DECISION (flag to lead): `RenderPlan` consumer is render-cpu/render-gpu but NO Cargo edge exists between composite and the renderers, and the renderers have no folio. Until the steward settles a neutral plan-IR home (candidate: a module in `pigment`, which all four allow), keep `LoweredPlan` composite-local and plain-data (no folio types in its public fields beyond ids as `String`), so an adapter can convert it later.
1. `lower_plan.rs`: `pub fn lower(snap:&Snapshot, expected_revision:u64, limits:&Limits, cancel:&CancellationToken) -> Result<LoweredPlan, CompositeError>`; `LoweredPlan{revision:u64, steps:Vec<Step>, outputs:Vec<NodeAddr>, blend_profile_bound:bool}`; `Step::Source{addr:NodeAddr, layer_kind:String, visible:bool, extent:Option<Rect>}` | `Step::Composite{addr:NodeAddr, inputs:Vec<NodeAddr> /*ordered by input_ordinal*/}`; `NodeAddr{layer_id:String, operation_key:String, port_key:String}` (the SAME authored ids, never reminted). Steps are topologically ordered (Kahn, deterministic tie-break by (layer_id, operation_key)); fanout (one output feeding several composites) appears once as a step and is referenced by address; a cycle (defence-in-depth even though folio validates `RenderCycle`) -> `CompositeError::Cycle`; stale `expected_revision != snap.document().revision` -> `StaleRevision`; `Limits{max_steps,max_depth}` -> `BudgetExceeded`; cancel checked per step.
2. Order convention (SPEC GAP, UNVERIFIED): spec says `input_ordinal` is "the sole authored semantic order" but does not state direction. Slice documents and tests `ordinal 0 = first composited (bottom-most)` in `DESCRIPTOR`; record the assumption in the lane file for the validator/spec-enrichment queue; confirm later against Photoshop's layer-panel/compositing order (black-box).
3. `evaluate_static.rs`: `pub fn evaluate(plan:&LoweredPlan, snap:&Snapshot) -> Result<EvalReport, CompositeError>` computing per-step `extent` (source extent = union of the layer's `Payload::Raster` tile rects via `Grid`/`Rect`; vector/text sources -> `extent: None` + `Unavailable("vector_to_image_conversion")` because folio forbids implicit vector/text->image conversion; composite extent = union of VISIBLE child extents), hidden layers excluded from composite input lists (kept in the plan as `visible:false` for the node view), `Payload::Unsupported` -> `Unavailable` (never Normal/skip). `snap.require_available()` failure (unassigned profile / unresolved issue) -> `CompositeError::Unavailable` carried in the report, not PASS.
4. Projection proof: `pub fn layer_projection(plan:&LoweredPlan, group_layer_id:&str) -> Vec<String>` (children layer ids in composite order) and `node_projection(...)` returning the same ids through addresses; test that layer->node->layer yields identical ids and order and that `snap.encoded_bytes()` is untouched (authored-record equality; STU-CMP-094 minimal).
5. Observability: `DESCRIPTOR` + `report(...)` mapping `Ok`->`Success`, `Cycle/StaleRevision/UnsupportedDescriptor`->`Failure(Validation)`, `Unavailable`->`Failure(Unavailable)`, `Canceled`->`Canceled`; example `composite-consumer --doc FILE.json` prints the plan steps and extents.
Oracle note (Operator; black-box only, no decompilation): later compare against Photoshop/After Effects-produced references: (a) layer-panel order vs composite order, group "pass through" vs isolated groups, hidden-layer exclusion, (b) per-layer opacity/fill/clipping/mask/matte order of operations, (c) authored-record equality across layer<->node view switch and reopen. Craft's composite helper round-trips through its own code (VIS-PREP-005) - not an oracle.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- None new for the slice (std + accord/folio/pigment/observe). Folio already brings `serde`/`serde_json` (=1.0.229/=1.0.151) if the example prints JSON by dependency; otherwise hand-format like the pulse brief.
- Later candidates (not now): `rayon` for parallel evaluation (Craft pin "1", UNVERIFIED current), `taffy` is NOT relevant here (layout/components).
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-composite --tests` (then `clippy`, `test --test static_plan`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/static_plan.rs">

## Tests (<=5, `bounded_case`; copy the `mixed()` document helper from `crates/hsk-studio-folio/tests/foundation_contract.rs`)
1. `layer_node_projection_same_authored_ids`: mixed doc (3 sources -> group composite) lowered; step order, ordered inputs and ids equal the authored ones; layer->node->layer identity; `snap.encoded_bytes()` unchanged.
2. `mask_matte_backdrop_order`: ordered multi-input composite (ordinals 0,1,2 supplied out of storage order) lowers to the ordinal order; richer descriptor kind (mask/matte) in a doc that folio marks read-only/unsupported -> `Unavailable`/`UnsupportedDescriptor`, never reordered or skipped.
3. `fanout_cycle_refusal_cancel_stale`: one source feeding two composites appears once; a hand-built cyclic `LoweredPlan` input to `evaluate` (or lower on a crafted snapshot) -> `Cycle`; pre-cancelled -> `Canceled`; wrong `expected_revision` -> `StaleRevision`.
4. `static_closure_no_temporal_provider`: assert via `cargo metadata`-free check - read this crate's own `Cargo.toml` text in the test and assert it lists no `motion|pulse|reel|score|render-` dependency (guards STU-CMP-093).
5. `extents_hidden_and_unavailable`: raster tiles produce the expected union `Rect`; hidden layer excluded from the parent's extent; vector/text source and `Payload::Unsupported` reported `Unavailable`; unassigned profile blocks `blend_profile_bound`.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No pixel work (render-cpu), no blend math, no masks/mattes/backdrop/effect/precomp/3D/time/expression sampling, no colour conversion (prism), no caching, no plan serialization or persistence (plans are disposable), no Motion/Pulse/Reel/Score dependency, no host/GUI wiring, no mutation of Folio.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
