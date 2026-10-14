---
file_id: BUILDER-BRIEF-LANE-SMB-COMPOSITE-TEMPORAL
file_kind: builder_brief
updated_at: 2026-10-14T19:06:53Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-composite
lane: SMB-COMPOSITE-TEMPORAL
owned_paths: [crates/hsk-studio-composite/src/temporal.rs, crates/hsk-studio-composite/src/temporal/**, crates/hsk-studio-composite/tests/temporal.rs, the `mod temporal;`/`pub use` lines + DESCRIPTOR `unsupported`/`operation` update in crates/hsk-studio-composite/src/lib.rs, the `[features]`/optional dependency lines in crates/hsk-studio-composite/Cargo.toml]
depends_on: [decision T1 (optional edge composite -> motion behind feature `motion`) for the motion adapter; without T1 the slice ships the port + frame lowering and the adapter waits]
author_role: brief writer (parity follow-up 2026-10-14); no product code written, no cargo run
evidence_head: product codex/handshake-studio-assets 9a2c840
parity_refs: [AE-01, AE-03, AE-22, PR-20..22 (effects/titles over time), top_gap after_effects#1]
---

<topic id="finding" status="active" summary="What exists today">

## Finding (verified in code)
- Composite is static by design: DESCRIPTOR `unsupported` lists `time`, `3d`, `precomp`, `matte` (`src/lib.rs:156`); "static_closure: depends only on accord folio pigment observe; no motion pulse ...".
- Static path to reuse unchanged: `lower(snap, expected_revision, blend_space, limits, cancel) -> LoweredPlan` (`src/lower_plan.rs:210`), `evaluate` (`src/evaluate_static.rs:372`), `lower_to_render_plan(plan, report, snap, root, target, info) -> RenderLowering{plan, sources: SourceTable, mask_sources: MaskTable, exactness}` (`src/lower_render_plan.rs:386`, struct `:101`). `SourceTable` maps `SourceKey -> TileRef` and `TileRef.layer_id` names the Folio layer (`hsk-studio-folio/src/lib.rs:234-246`).
- Pigment's plan places every tile by an integer `TileInput.placement: Rect` and every mask by `MaskInput.placement` (`hsk-studio-pigment/src/render_plan.rs:104-121`); `Layer.opacity: f32` (`:147-153`). The plan IR has NO affine transform/resampling (parity AE-03 "render plan has no layer transforms").
- Folio: `StudioLayer.visible`, `appearance.opacity` percent (`hsk-studio-folio/src/lib.rs:183-196`, `appearance.rs:59-63`), `StudioMask.linked` "Mask moves with the layer (linked) or stays fixed in the document" (`appearance.rs:49-55`); `validate_document(doc, budget, cancel, resolver) -> Snapshot` builds a derived snapshot (`lib.rs:1097`). No `StudioComposition` record (AE-01: "no composition record in folio"; `ClipSource::Primitive` only references one, `timeline.rs:114-117`).
- Motion evaluates keyframes at exact ticks: `Property::new(key, static_value, keys)` (`hsk-studio-motion/src/property.rs:71`), `Property::sample(EvalTick, cancel) -> Vec<f64>` (`:295`), `Keyframe`/`Interpolation` linear/Bezier/hold (`src/keyframe.rs:35-145`, AE-04 VALIDATED), `EvalTick` i64 (`src/time.rs:7`). Motion's Cargo deps: accord + observe only.
- Ticks: `accord::TICKS_PER_SECOND = 254_016_000_000` (`hsk-studio-accord/src/lib.rs:87`), `accord::Ticks` (`:363`); frame <-> tick conversion is pulse's (`hsk-studio-pulse/src/time.rs:14 frames_to_ticks`), which composite cannot reach (no edge) — callers pass ticks.

</topic>

<topic id="ownership" status="active" summary="Owners per CX-CAP-001">

## Owners
- **Composite** owns time-based compositing: STU-CMP-002 "This sub-section owns ... time-based compositing operations. It does NOT own the property tree, keyframes or expressions -- 14.26, which this sub-section consumes." Module map: "Temporal/effect providers attach through typed ports; no unconditional motion/pulse engine dependency in the static render closure."
- **Motion** owns keyframe evaluation (consumed, not copied). **Pulse** owns frame/tick conversion (caller side). **Folio** owns the future `StudioComposition` record (slice 3, folio owner). **Pigment** owns any future transform/resample operator in the plan IR (slice 2, pigment owner).
- Decision **T1** (lead; module-map drift recorded like D9): add `hsk-studio-motion` to composite's allowed list as an OPTIONAL dependency behind a non-default feature `motion`. The default build keeps the static closure exactly as today (proved by `cargo check -p hsk-studio-composite` without features), which is what "no unconditional motion dependency" asks. Rejected: adapter in harness (harness is a consumer, has no composite/motion edges, and time-based compositing would sit outside its owner); a new glue crate (more crates for ~80 lines).

</topic>

<topic id="spec-basis" status="active" summary="Clauses (spec v02.216 module 14)">

## spec_basis
- STU-MOT-001 (l.25675): "`StudioComposition` (schema id `hsk.studio.composition@1`) is the motion document primitive. A composition is a timed, sized, colour-managed container holding an ORDERED STACK of `StudioLayer` nodes ... a member of the unified `StudioDocument`".
- STU-MOT-020 (l.27321): "Every property's keyframe times, the work area, layer in/out points, markers and the playhead share this one time base. A composition placed inside a sequence maps its time to sequence ticks at the boundary ([STU-VID-012])".
- STU-MOT-013 (l.27272): transform group "anchor point, position ..., scale, rotation ..., and opacity"; rotation stored unwrapped.
- STU-CMP-010 (l.29270) order per layer: 1 source at remapped time, 2 masks, 3 effects, 4 layer styles, "5. **Transform.** Apply anchor point, scale, rotation, position and the parent chain", "6. **Opacity.**", 7 track matte, 8 blend. STU-CMP-010b: "masks are applied BEFORE effects, and the transform is applied AFTER effects."
- STU-CMP-002 (l.29227): ownership boundary quoted above.

</topic>

<topic id="scope" status="active" summary="Slice 1 (~1-2 h)">

## Slice 1: frame N of a layer stack = static lowering + per-layer time state
New module `composite::temporal` (`#![deny(clippy::arithmetic_side_effects)]`):
1. Port (default build, no motion types):
   ```rust
   pub struct LayerFrameState { pub visible: bool, pub opacity_scale: f64 /* 0..=1 */, pub translate_px: [f64; 2] }
   pub trait TemporalSource {
       /// None = the layer is not animated (static state applies).
       fn layer_state(&self, layer_id: &str, tick: i64, cancel: &CancellationToken) -> Result<Option<LayerFrameState>, TemporalError>;
   }
   pub struct LayerTiming { pub layer_id: String, pub in_tick: i64, pub out_tick: i64 } // [in, out)
   ```
2. `lower_frame(snap, expected_revision, blend_space, limits, root, target, info, timings: &[LayerTiming], source: &dyn TemporalSource, tick: i64, cancel) -> Result<FrameLowering, CompositeError>`:
   - derive a frame document from `snap.document()`: a layer outside its `[in, out)` window or with `visible == false` from the port gets `visible = false`; opacity becomes `static_opacity * opacity_scale` (step 6 of STU-CMP-010); re-validate with `folio::validate_document` (frame snapshot is disposable, never stored);
   - run the EXISTING `lower` -> `evaluate` -> `lower_to_render_plan` unchanged;
   - apply translation AFTER lowering (STU-CMP-010 step 5 after effects): for every `TileInput` whose `SourceTable` entry has that `layer_id`, shift `placement.x/y` by the translation; shift `MaskInput.placement` only when the layer's `StudioMask.linked` is true; re-run `RenderPlan::validate`;
   - translation must be an integer number of pixels: a fractional, scale, rotation or anchor change is refused `transform_requires_resampling` (typed `Unsupported`, never rounded silently); this is the honest limit of the current plan IR;
   - `FrameLowering { lowering: RenderLowering, tick, applied: Vec<(layer_id, LayerFrameState)>, refused: Vec<...> }`.
3. Feature `motion` (after T1): `MotionTemporalSource` holding, per layer id, motion `Property` values for `opacity` (1 component, percent 0..100) and `position` (2 components, px) and calling `Property::sample(EvalTick::new(tick), cancel)`; it is the only motion-aware code in composite.
4. DESCRIPTOR: `time` moves from `unsupported` to a `temporal` entry (integer translation + opacity + in/out only; transforms needing resampling refused; NOT_PROVEN vs After Effects).

## Non-goals (slice 1)
- Scale/rotation/anchor/parenting (needs a pigment transform/resample operator: slice 2, pigment owner, then composite lowers to it), motion blur, time remap, frame blending, precomps, track mattes, 3D, expressions (motion-js), audio, video sources (reel), a `StudioComposition` Folio record, render queue/encode (AE-22), frame-rate maths (pulse, caller side).

## Slice 2 (pigment owner first, new lane)
- Pigment plan operator `Transform{affine 2x3, filter: nearest|bilinear}` on `Layer` + render-cpu implementation; composite lowers position/scale/rotation/anchor/parent chain to it.
## Slice 3 (folio owner, new lane)
- `StudioComposition` record (`hsk.studio.composition@1`, settings of STU-MOT-004) with layer in/out ticks and per-property value streams stored as keyframe lists of doubles (STU-MOT-020a); composite then reads timings from Folio instead of the `LayerTiming` argument.

</topic>

<topic id="native" status="active" summary="Relation to the native format">

## Native format
- Slice 1 stores nothing: the frame document and plan are disposable; timings and the temporal source are caller inputs.
- Target persistence (slice 3): the composition and its keyframe streams live in the Folio graph as `StudioComposition` (one `StudioDocument`, STU-MOT-001), serialised by package like every other record; rendered frames are derived and never stored as authority.

</topic>

<topic id="ui" status="active" summary="UI dependency">

## UI
- None. Timeline panel, playhead scrubbing and preview playback are GUI-host work (D11); `lower_frame(tick)` is what a scrubber or render queue calls per frame.

</topic>

<topic id="deps" status="active" summary="Edges and crates.io">

## Dependencies
- Decision T1: `hsk-studio-composite` allowed list + `hsk-studio-motion` (steward: `steward-sync.sh --add-edge hsk-studio-composite=hsk-studio-motion`); manifest line `hsk-studio-motion = { path = "../hsk-studio-motion", version = "=0.1.0", default-features = false, optional = true }` and `[features] motion = ["dep:hsk-studio-motion"]`. Check with the steward that `workspace-probe.rs` accepts an optional path dependency + `dep:` feature (harness already uses this pattern: `import-tiff = [..., "dep:hsk-studio-interop-tiff"]`).
- No crates.io dependency.

</topic>

<topic id="acceptance" status="active" summary="Acceptance and proof">

## Acceptance (at most 5 focused tests, `tests/temporal.rs`; reuse the document builders in `tests/common`)
1. **In/out window.** Two raster layers, layer B with `[in, out) = [10584000000, 21168000000)` (frames 1..2 at 24 fps, ticks computed in the test as `TICKS_PER_SECOND / 24`): at tick 0 the plan has only A; at tick 10584000000 it has A and B; at 21168000000 only A.
2. **Opacity over time.** A test `TemporalSource` returning `opacity_scale = 0.25` at tick T gives `Layer.opacity == static * 0.25` in the plan; rendering one pixel through pigment's single-pixel stack oracle (`hsk_studio_pigment::blend_math::evaluate_stack`, `src/blend_math.rs:804`) gives the expected composite.
3. **Translation.** `translate_px = [3, -2]` shifts every `TileInput.placement` of that layer by exactly (3, -2), shifts a linked mask, leaves an unlinked mask in place; `[0.5, 0]` refuses `transform_requires_resampling`; the static path (`lower_to_render_plan` on the original snapshot) is byte-identical to before (no regression).
4. **Motion adapter (feature `motion`, only after T1):** a linear keyframe pair opacity 0 % at tick 0 -> 100 % at `TICKS_PER_SECOND` sampled at half a second gives `opacity_scale == 0.5`; a hold keyframe stays constant until the next key.
5. **Hostile (V4-02).** `dims_u32_max`: a translation that pushes `placement.x` past `i64::MAX` refuses `Overflow` (checked add), never wraps; `aggregate_over_budget`: `timings` longer than `Limits.max_steps` (4096) refuses before work; nonfinite `opacity_scale`/translation refuses; `in_tick >= out_tick` refuses; an unknown layer id in `timings` refuses `DanglingInput`.

## Proof commands
- `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-composite --tests` (default features: static closure unchanged)
- same prefix: `cargo clippy -p hsk-studio-composite --tests`; `cargo nextest run -p hsk-studio-composite --test temporal`
- after T1: same prefix `cargo nextest run -p hsk-studio-composite --features motion --test temporal`

</topic>

<topic id="risks" status="active" summary="Risks and mitigations">

## Risks
- **Integer-only translation looks like a toy.** It is the honest maximum of today's plan IR; the refusal is typed and the descriptor says so. Real transforms need the pigment slice (sequenced next); do not resample inside composite (pigment owns pixel maths, CX-CAP-001).
- **Re-validating a full Folio document per frame** costs time on big docs. Accepted for slice 1 (correctness first); slice 3 reads timings from Folio and can patch only the affected layers.
- **Static regression in a heavily validated crate (PASS_V9 lanes).** Mitigation: new module only; the existing `render_plan_lowering` and `static_plan` tests must stay green (run them once at READY: `cargo nextest run -p hsk-studio-composite`).
- **Probe/edge risk for the optional motion edge.** Mitigation: ship the port first; the feature lands only after T1 and a steward probe run.

</topic>

<topic id="cohesion" status="active" summary="How it serves several apps">

## Cohesion
- After Effects: compositions over time are the same layer stack, blend modes, masks and effects the Photoshop path renders, evaluated per tick; Premiere: titles/graphics clips (`ClipSource::Primitive` -> composition) render through the same `lower_frame`; Figma prototype animations and Photoshop timeline frames reuse the port; one tick base (accord) for video, motion and audio (STU-MOT-020 -> STU-VID-012).

</topic>
