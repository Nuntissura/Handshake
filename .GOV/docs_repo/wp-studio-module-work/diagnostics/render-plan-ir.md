---
file_id: WP-STUDIO-MODULE-WORK-DIAG-RENDER-PLAN-IR
file_kind: architecture_decision
updated_at: 2026-10-11T08:10:00Z
product_commit: 4c3ac91 (codex/handshake-studio-assets, pushed)
wp: WP-KERNEL-STUDIO
owner: diagnostics-architect
crates: [hsk-studio-pigment, hsk-studio-composite, hsk-studio-render-cpu, hsk-studio-render-gpu, hsk-studio-interop-psd]
status: decided_implemented_in_pigment
---

# Shared render-plan IR and backend port

<topic id="decision" status="active" summary="Owner = hsk-studio-pigment modules render_plan + blend_math; alternatives rejected">

## Decision
The render-plan IR, the `StudioBlendMode` enumeration with applicability and exactness facts, the backend port (`RenderBackend`), the source/sink ports and the render receipt live in ONE module: `hsk_studio_pigment::render_plan` (`crates/hsk-studio-pigment/src/render_plan.rs`). Composite builds plans; render-cpu and render-gpu implement `RenderBackend`. Nobody else defines a plan, blend-mode enum, exactness class, sink trait or receipt.

## Evidence (verified this session)
- Root `Cargo.toml` `[workspace.metadata.studio.dependencies.*].allowed`: composite = accord, folio, prism, pigment, nib, type, observe; render-cpu = render-gpu = accord, prism, pigment, nib, type, observe; pigment = accord, prism, observe, folio. No composite<->render edge. The renderers cannot reach folio. The modules all three can reach are accord, prism, pigment, nib, type and observe.
- `STUDIO_MODULES.json`: pigment = "CPU tiles, sample formats, masks, brush/raster operations; no GPU/device dependency"; composite = "Static render-plan evaluator consumes immutable snapshot ... typed ports"; render-cpu = "CPU render backend/reference"; render-gpu = "... GPU API types hidden from document/contracts"; accord = "Stable IDs, units, revision and capability schemas ..."; prism = "Profiles, rendering intents, linearization/ICC transforms".
- Codex CX-CAP-001: one active owner per capability; consume the lowest owner.
- Current duplication (CX-CAP-001 defect): `composite/src/blend.rs` (35-member `BlendMode`, `Fidelity`, `BlendSpace{Encoded,Linear}`, `BlendReceipt`, `evaluate_stack`), `render-cpu/src/sample_math.rs` (7-member `BlendMode`), `render-cpu/src/plan_sink.rs` (`BlendSpace{LinearLight,Encoded}`, `RenderPlan`, `Op`, `PixelSink`, `RenderReceipt`), `interop-psd/src/blend.rs` (28-member `BlendMode`). The composite brief also listed "no blend math" as a non-goal.

## Why pigment
1. It is the lowest module that all three consumers can reach AND that already owns the referenced primitives (`Rect`, `Grid`, sample formats `rgb_f32le`/`coverage_u8`/`coverage_u16le`, `Mask`, `ResolvedTile`). An IR placed lower would have to copy them.
2. Pigment's own raster/brush operations need the same blend enumeration and math (tool-only modes `behind`/`clear` are brush modes, STU-RAS-155). So pigment has to own the enumeration anyway. Develop and interop-psd also reach pigment.
3. The IR is plain data with no GPU/device types, so pigment's "no GPU/device dependency" rule still holds. The IR also has no folio types: sources are keyed by `SourceKey(u32)` plus content hashes, so renderers stay folio-free in practice as well as in Cargo.
4. No root `Cargo.toml` edit, new crate, Operator approval or edge change is needed. The design is additive.

## Alternatives rejected
- accord: lower, but it cannot name pigment's `Rect`/mask/sample formats (accord allowed = []). That would force copies (CX-CAP-001), and accord's scope is IDs/units/revision/capability envelopes, not raster (ENG-DAG-FORBID-002 keeps it a cheap boundary).
- prism: owns colour transforms, not raster ops, and cannot see pigment types.
- composite owns the IR with a render->composite edge: the renderers would then pull folio in transitively. That defeats STU-ARC-002 independent selectability and STU-CMP-093's static closure, and it needs an allow-list change.
- render-cpu owns the IR: composite and render-gpu cannot reach render-cpu, and GPU must not depend on the CPU backend.
- A new 36th crate (for example `render-plan`): it would change the 35-module set (STU-ARC-016) and STUDIO_MODULES and need Operator approval and spec enrichment. Keep it only as the fallback if pigment's responsibility text is judged too narrow.

## Spec gaps recorded (for enrichment; not resolved here)
- SPEC GAP 1: STU-CMP-020c says "14.8 owns the `StudioBlendMode` primitive" (14.8 = Color Management, i.e. prism), but the canonical enumeration STU-RAS-154/039 sits in 14.4 Raster (pigment). Pigment is chosen because module responsibilities assign raster/brush ops there. If enrichment settles on 14.8/prism, the enum moves one level down into prism and pigment re-exports it. That is non-breaking, because every pigment consumer also reaches prism.
- SPEC GAP 2: STU-CMP-020 lists 38 compositing members, including Dancing Dissolve, Classic Color Burn/Dodge/Difference, Add, Stencil/Silhouette Alpha/Luma, Alpha Add and Luminescent Premul, which have no STU-RAS-154/039 discriminant. The enum is `#[non_exhaustive]` so they can be added later without breaking consumers. The six STU-CMP-020a members are stack-wide mattes, so they become NEW node kinds, not `Layer.mode` values.
- DAG GAP: motion (shape fill/stroke blend STU-MOT-101, per-character STU-MOT-093) cannot reach pigment. Either the steward/Operator adds pigment to motion's allowed list, or motion stores the u16 discriminant and composite converts it. The same applies to folio: authored layer records store the u16 discriminant (folio sees only accord), and composite converts it with `StudioBlendMode::try_from(u16)`. An unknown value is a typed refusal, never Normal.

</topic>

<topic id="blend-math-owner" status="active" summary="Single owner of per-pixel blend math and exactness classes = pigment::blend_math (implemented)">

## Blend math owner (scope addition from the lead)
- Decision: `hsk_studio_pigment::blend_math` is the ONE owner of `StudioBlendMode` (35 members, STU-RAS-154/039 discriminants), `ModeFacts` (key, family, applicability, exactness, reference), `Exactness` (`exact|fitted|approximate|unsupported`), `BlendSpace` (`linear_light|encoded`, `From<prism::Transfer>`), `BlendError`, the W3C s10.1/s10.2 functions (`blend_rgb`, `separable_fn`, `lum`/`set_lum`/`sat`/`set_sat`), straight-alpha `composite_over` (with the deterministic dissolve `site_unit`), `Rgba`, and the single-pixel group oracle `evaluate_stack` (returns `StackReceipt { space, exactness: ExactnessLedger }`). `render_plan` re-exports the enum, facts and ledger types; it does not define them again.
- Why pigment: the same reasons as the IR. It is the lowest module reachable by composite, render-cpu, render-gpu, develop, interop-psd and pigment's own brush ops (render-cpu cannot reach composite). STU-CMP-020c requires ONE enumeration across layer, effect, style, shape and text scopes.
- Port fidelity (commit 4c3ac91): the formulas, discriminants, keys, families, classes and reference strings are copied from composite `blend.rs` at a6f2c7e, including the darker/lighter-colour wording. Composite's `tests/blend_oracles.rs` (5 tests) and `tests/blend_laws.rs` (sweep over a 125x125 lattice) are ported into `pigment/tests/blend_math.rs` with renames only: BlendMode->StudioBlendMode, Fidelity->Exactness, MODE_TABLE->MODES, info()->facts(), Linear->LinearLight, receipt.lowest->receipt.exactness.lowest. All 6 pass, and the hand-computed oracle values are unchanged.
- OPEN, lead decision needed (numeric policy conflict between today's two copies):
  1. render-cpu does binary64 intermediate arithmetic with one binary32 rounding. pigment, like composite, is f32 throughout.
  2. render-cpu passes values outside [0,1] (HDR, linear light) unclamped through the same arithmetic. pigment, like composite, refuses non-finite or out-of-[0,1] input with `OutOfDomain` and clamps results.
  - Recommendation: render-cpu adopts pigment's domain now (typed refusal, never a silent extension), because its HDR behaviour is UNVERIFIED by its own descriptor. Any binary64 or HDR variant is ADDED inside pigment `blend_math` (new entry points, HDR at class Approximate) by a pigment-scoped change, never as a render-cpu copy.

</topic>

<topic id="ir" status="active" summary="Minimal IR node kinds and invariants">

## IR (`render_plan.rs`)
- `RenderPlan<V = NoVector> { revision: u64, target: OutputTarget, nodes: Vec<PlanNode<V>>, root: PlanNodeId }`. Nodes form an index-ordered DAG. Every reference points to an EARLIER index, so the plan is acyclic by construction. Fanout (one node feeding several layers) is allowed, and the plan is never persisted. It derives `Serialize` only (no `Deserialize`), for `digest()`.
- `OutputTarget { extent: Rect, format: SampleFormat, alpha: AlphaConvention, space: ColourTag }`. The root renders onto a transparent backdrop over `extent`.
- `ColourTag { profile_sha256: [u8;32], blend_space: BlendSpace::{LinearLight, Encoded} }` covers STU-COL-162 (explicit blending space), STU-CMP-021 (declared linearity, recorded in every receipt) and STU-COL-163 (inputs arrive already in this space, and the backend checks the tag without converting). The keys are `linear_light` and `encoded`.
- `AlphaConvention::{Straight, Premultiplied}` (STU-CMP-022). The canonical path today is Straight.
- `SampleFormat::RgbF32Le` and `MaskFormat::{CoverageU8, CoverageU16Le}` use the same keys as pigment's existing strings. Both are `#[non_exhaustive]`.
- `PlanNode<V>`:
  - `Tile(TileInput { key: SourceKey, placement: Rect, format, alpha, colour_sha256, alpha_sha256 })` is a layer source tile reference, content-addressed and folio-free.
  - `Solid { rect, rgba: [f32;4] }` is a straight RGBA fill.
  - `Vector(V)` is a typed vector coverage payload. V is chosen by builder and backend together (both reach nib). The default `NoVector` is uninhabited.
  - `Group(Group { isolated: bool, layers: Vec<Layer> })` lists its layers bottom-first, index 0 composited first. This matches composite's `input_ordinal 0 = bottom-most` convention, which is still a SPEC GAP against Photoshop.
- `Layer { input: PlanNodeId, mode: StudioBlendMode, opacity: f32, fill_opacity: f32, mask: Option<MaskInput> }`. `MaskInput { key, placement, format, sha256, inverted, density }`. Hidden layers are not emitted; composite drops them.
- `StudioBlendMode` (re-exported from `blend_math`) has 35 members with the STU-RAS-154/039 discriminants (pass_through=1 ... divide=30, average..erase=31..35). `facts()` returns `{ key, family, applicability: Layer|GroupOnly|ToolOnly|Unspecified, exactness, reference }`.
- `Exactness::{Exact, Fitted, Approximate, Unsupported}` uses `lowest_of`, following the craft case study (every operator is tagged, and the lowest class travels with the receipt).
- `Operator` covers each feature a plan can use: `Tile`, `Solid`, `Vector`, `Mask`, `Blend(mode)`, `IsolatedGroup`, `PassThroughGroup`, `PassThroughOpacity` (intrinsic Approximate), `NonIsolatedBlendedGroup`, and `FillOpacity` (fill<1 with a non-Normal mode; intrinsic Approximate until the Photoshop fill oracle, research [C]/[O]).

## Validation (`RenderPlan::validate(&PlanLimits)`; one shared owner, run by every backend)
The plan is rejected when it has no nodes, when the root is out of range, or when an input or a reference points forward. It is also rejected for:
- an empty target, tile, solid or mask rect;
- non-finite values, or opacity, fill, density or solid alpha outside [0,1];
- a tool-only mode on a layer (STU-RAS-155);
- `pass_through` on a non-group input, or on an isolated group;
- exceeding node, layer or depth limits.

An Unspecified mode (average..erase) is structurally valid, but admission refuses it as Unsupported.

</topic>

<topic id="port" status="active" summary="Backend trait, sources, sink, admission">

## Port
```rust
pub trait RenderBackend<V = NoVector>: Send + Sync {
    fn identity(&self) -> BackendIdentity;                 // name, version, Cpu|Gpu, adapter, generation
    fn exactness(&self, operator: Operator) -> Exactness;  // capability query (STU-RAS-155)
    fn execute(&self, plan: &RenderPlan<V>, request: &ExecuteRequest, sources: &dyn PlanSources,
               cancel: &CancellationToken, sink: &mut dyn TileSink) -> Result<RenderReceipt, BackendError>;
}
pub struct ExecuteRequest { expected_revision: u64, min_exactness: Exactness, deadline: Option<Instant>, max_working_bytes: u64 }
pub trait PlanSources: Send + Sync { fn tile(&self, SourceKey) -> Option<TilePlanes<'_>>; fn mask(&self, SourceKey) -> Option<crate::Mask<'_>>; }
pub trait TileSink { fn accept(&mut self, rect: Rect, colour: &[u8], alpha: &[u8]) -> Result<(), SinkError>; }
pub fn admit<V>(plan, backend: &(impl RenderBackend<V> + ?Sized), request: &ExecuteRequest, cancel: &CancellationToken) -> Result<ExactnessLedger, BackendError>;
impl RenderPlan<V> { fn validate(&self, &PlanLimits) -> Result<Vec<Operator>, PlanError>; fn digest(&self) -> Result<[u8;32], pigment::Error> /* V: Serialize */ }
impl ResolvedTile<'a> { fn planes(&self) -> Result<TilePlanes<'a>, pigment::Error> }
```
- `admit` checks cancel and stale revision, validates the plan, and then for each operator takes the lower of its intrinsic class and the backend's class. It refuses with `Unsupported{operator, exactness}` when the result is Unsupported or below `request.min_exactness`. It returns the ledger that goes into the receipt. Every backend calls it first, so a bad plan never reaches the sink and the refusal rules have one owner.
- `TilePlanes<'a>` is the folio-free resolved view (`width`, `height`, colour/alpha planes and strides, hashes, `format`, `alpha`, `space`). `ResolvedTile::planes()` adapts pigment's canonical tile. The backend refuses when the planes' hashes, format, space or size differ from the plan's `TileInput` (`SourceMismatch(key)`).
- `BackendError` is the shared typed vocabulary: `Canceled`, `DeadlineExceeded`, `StaleRevision{expected,found}`, `InvalidPlan(PlanError)`, `Unsupported{operator,exactness}`, `UnresolvedSource(key)`, `SourceMismatch(key)`, `BudgetExceeded{requested,limit}`, `Unavailable(reason)`, `DeviceLost{generation}`, `Sink(SinkError)`, `Overflow`. It provides `code()` and `diagnostic() -> observe::ResourceCode`. Backends keep their richer internal errors and map them at the port.
- GPU: wgpu types never appear in the port; `identity().adapter` is plain text. `Unavailable`/`DeviceLost` are determinate results (STU-FX-036, STU-ARC-016), never a silent CPU fallback. Routing to CPU is the caller's explicit choice after `Unsupported`/`Unavailable`.

## Receipt (STU-ARC-015, STU-CMP-021)
`RenderReceipt { plan_revision, plan_digest: [u8;32] (sha256 of the canonical JSON of the plan), backend: BackendIdentity, region: Rect, target: OutputTarget (format, alpha, colour tag = profile + blend space), exactness: ExactnessLedger { lowest, operators[{key, exactness}] }, chunks, nodes_executed, peak_working_bytes }`. The receipt exists only after the last chunk is accepted. Actor, command and grant binding is the host/Argus wrapper's job; the backend does not add it.

</topic>

<topic id="migration" status="active" summary="Steps for composite, render-cpu, render-gpu, interop-psd and a pigment follow-up">

## Migration (each builder edits only its own crate)
Composite (`hsk-studio-composite`):
1. Delete `src/blend.rs`, `tests/blend_oracles.rs` and `tests/blend_laws.rs`; pigment `tests/blend_math.rs` now holds them. Import `hsk_studio_pigment::blend_math::{StudioBlendMode, ModeFacts, MODES, Exactness, Applicability, BlendSpace, BlendError, ExactnessLedger, ...}` and re-export what the public API needs. The renames are: `BlendMode`->`StudioBlendMode`, `Fidelity`->`Exactness`, `ModeInfo`/`info()`->`ModeFacts`/`facts()`, `MODE_TABLE`->`MODES`, `BlendReceipt`->`ExactnessLedger` (plus the plan's `BlendSpace`), and `BlendSpace::Linear`->`LinearLight`. The key changes from `linear` to `linear_light`, so update the DESCRIPTOR and the consumer `--space` parser. `CompositeError::Blend(BlendError)` keeps the same variants; `BlendError::UnknownDiscriminant` is unchanged.
2. Add `lower_to_render_plan(&LoweredPlan, &EvalReport, &Snapshot, target: OutputTarget) -> Result<(RenderPlan, SourceTable), CompositeError>`. Map each visible raster `TileRef` to a `TileInput` with `SourceKey(n)`, keep `SourceTable: Vec<(SourceKey, TileRef)>` composite-side, emit `Group` nodes in `input_ordinal` order (bottom-first), drop hidden layers, and map an unbound blending profile or any `Unavailable` step to `CompositeError::Unavailable` (no plan).
3. Fold `RenderReceipt.blend` (BlendReceipt) into `render_plan::ExactnessLedger`, and run `RenderPlan::validate` before handing the plan out.
4. If composite still needs a pixel oracle (for example in the `--modes` consumer output), it calls `pigment::blend_math::evaluate_stack`; it never keeps a local copy.

Render-cpu (`hsk-studio-render-cpu`):
1. Implement `RenderBackend` for `Renderer`, with `identity` = {`hsk-studio-render-cpu`, `0.1`, Cpu}. `exactness(Blend(m))` returns `m.facts().exactness` for the 7 implemented modes and Unsupported otherwise. `Tile`, `Solid`, `IsolatedGroup` and `PassThroughGroup` return Exact once groups are implemented; until then `Group` with more than the flat root returns Unsupported.
2. Delete `sample_math.rs`'s `BlendMode`, `blend_mode_name` and blend functions. Use `StudioBlendMode::try_from(u16)` and `.key()` for refusal naming, and call `blend_math::separable_fn`/`blend_rgb`/`composite_over` per pixel, following the numeric-policy decision in blend-math-owner. Replace the local `BlendSpace`, `PixelSink`, `SinkError`, `SourceId`, `SourceTile` and `RenderReceipt` with `BlendSpace`, `TileSink`, `SinkError`, `SourceKey`, `TilePlanes` and `RenderReceipt` from pigment. `UnsupportedBlend(u16)` stays an internal error mapped to `BackendError::Unsupported` at the port. Keep `Op::PathFill` as the backend's vector payload `V`, a struct of `nib::Anchor` + closed + winding. Long term, nib owns an owned path type (request to the nib lane).
3. Execution order: `admit(...)`, resolve and verify every `TilePlanes` against its `TileInput`, then chunks. Map `RenderError` to `BackendError` at the port and keep `RenderError` internal.

Render-gpu (`hsk-studio-render-gpu`):
1. `GpuBackend` (a private `Mutex<GpuContext + Residency>`) implements `RenderBackend`, with `identity` = {`hsk-studio-render-gpu`, `0.1`, Gpu, adapter text, generation}. Replace the brief's `GpuPlan`/`GpuLayer`/`GpuReceipt` with the shared plan and receipt. `exactness(Blend(Normal|Multiply|Screen))` is `Fitted` (tolerance-equal to the CPU reference, not bit-exact) or `Exact` per the brief's measured error. Everything else is Unsupported.
2. `NoAdapter` maps to `BackendError::Unavailable("no_adapter")`, and device loss maps to `DeviceLost{generation}`. Admission (`admit`) runs before any upload.

Interop-psd: map PSD 4-byte keys to `StudioBlendMode` (pigment) instead of the crate-local 28-member enum, keeping the raw key verbatim. An unknown key is a typed loss.

Pigment follow-up (pigment-scoped, only if the lead accepts the numeric-policy options): f64 entry points and/or an HDR extension inside `blend_math`, plus STU-CMP-020 members once discriminants are enriched. render-gpu tests its WGSL against `blend_math`, never against render-cpu.

</topic>

<topic id="risks" status="active" summary="Risks and controls">

## Risks / controls
- Builders keep their crate-local IRs after this lands, which leaves two plan shapes. Control: the validator checks that composite/render-cpu public APIs contain no `BlendMode`/`BlendSpace`/`PixelSink`/`RenderReceipt` definitions outside pigment (grep), per CX-CAP-001.
- The 35-member pigment table drifts from composite's `MODE_TABLE` during the transition. Control: the ported pigment tests pin discriminants, keys, classes and oracle values. Composite deletes `blend.rs` in migration step 1, and the validator greps for any second `enum BlendMode`/`MODE_TABLE`/`fn blend_rgb` outside pigment.
- A builder edits pigment while others are compiling against it. Control (lead rule): write new module files first, add the `mod` line last in one edit, check, and commit. During this work pigment was briefly uncompilable between the `lib.rs` edit and the file write; it was fixed within minutes, and the committed 4c3ac91 compiles.
- `fill_opacity` semantics are unverified for the special modes. Control: the intrinsic class is Approximate, so exports with floor Exact refuse.
- `plan_digest` uses serde_json canonical bytes. It is stable within one build, but it is not a persisted format and not a cross-version identity.
- The vector payload generic `V` adds friction for `dyn` use. Control: backends and builders agree on V through nib. The default `NoVector` keeps simple plans non-generic in practice.

</topic>
