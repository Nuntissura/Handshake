---
file_id: BUILDER-BRIEF-HSK-STUDIO-RENDER-CPU
file_kind: builder_brief
updated_at: 2026-10-11T04:10:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-render-cpu
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-RENDER_CPU` / key `render_cpu` / crate `hsk-studio-render-cpu`; foundation MT-35545 (`proof.checks`, test target `cpu_plan`, case `unsupported_op_stale_result`); write scope `handshake-studio-assets/crates/hsk-studio-render-cpu/**`.
- Responsibility (verbatim): "CPU render backend/reference; opt-in professional reference cases require external independent oracles."
- Foundation capability: "Render real plan to pixels with revision/color receipt and bounded sink, compare independent known reference."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-prism`, `hsk-studio-pigment`, `hsk-studio-nib`, `hsk-studio-type`, `hsk-studio-observe`. NOTE: NO `folio`, NO `composite` edge. Slice uses accord, pigment, nib, observe.
- Forbidden: handshake_core/native/SurrealDB (ENG-DAG-FORBID-001); wgpu/GPU/codec/native handles (GPU is `render-gpu`, STU-ARC-002); `composite`/`render-gpu` edges; per-pixel profile conversion (STU-COL-163). Files per MT-35545: `src/plan_sink.rs`, `src/sample_math.rs`, `tests/cpu_plan.rs`.
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `render-cpu-consumer` (`--descriptor`), 30 s `bounded_case`. Root `Cargo.toml` is steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-COL-161 (l.10285 table): compositor sits between input transform and output transform; STU-COL-163: RenderEngine receives buffers ALREADY in working/blending space and materialised transforms as data, MUST NOT call a ColorEngine per pixel / resolve profiles / pick intent; STU-COL-162: `blending_space` is explicit and stored.
- STU-RAS-154 (l.1618+): canonical `StudioBlendMode` discriminants (normal=2, darken=4, multiply=5, lighten=8, screen=9, overlay=12, difference=18, ... 30 members, `behind`/`clear` tool-only, `pass_through` group-only); STU-RAS-155: inapplicable/unknown mode = typed capability error naming the mode, never silent no-op; STU-RAS-039: `average/negation/reflect/glow` have NO recovered function (do not guess).
- STU-FX-036/037 (l.19634): CPU fallback produces promotion-equivalent output; byte-stable on fixed backend; STU-ARC-015: render receipt binds exact revision, renderer/version, colour/profile, region; unsupported fallback never proves the edited result; STU-ARC-008/012/013: bounded chunks, cancel/deadline observed BETWEEN chunks, release on every exit, previews may coalesce but committed output never silently drops.
- Later scope: 14.24 validation/promotion equivalence (l.22678), STU-FX catalogue (14.9), STU-RAS-130..160 blend/layer-effect detail, STU-CMP-092..094 (plan lowering = composite). MT plan domain: `_mt_plan_04_raster.json` (MT-017 RasterEngine), `_mt_plan_09_effects.json`, `_mt_plan_0708_typography_color.json`.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std + kurbo, ~700-1000 LOC)
Prep reuse (do not re-derive): visual-team `preparation.json#/representative_proposals[6]` (MT-35545 algorithm: "Execute selected static plan in bounded tiles/chunks to caller sink; retain sample/profile/alpha ordering and explicit unsupported operations"; cases `known_alpha_mask_sample_math`, `chunk_boundary_equals_independent_reference`, `sink_error_cancel_releases_bytes`, `unsupported_op_stale_result`; oracle "Hand-computed tiny output and independent original image/print corpus. CPU-vs-GPU equality alone cannot prove both correct."); `.GOV/reference/studio_app_feature_research/13-layer-graph-vertical-slice.md` (fixtures lg-fixture-001/002: stack + mask visual hash); `19-studio-local-first-rust-posture.md`; `11-provider-posture-map.md` (no AI/cloud). Craft reference (pinned cache `.../gov_runtime/tools/studio-craft-source-cache/photocraft-175603a6.../crates/{compose,raster}`; MIT OR Apache-2.0): reference only, never parity proof.
Reuse (verified by grep): `pigment::{Rect{x,y,width,height}.intersect/area/contains, Grid{origin_x,origin_y,tile_width,tile_height}.bounds(col,row), ResolvedTile (pub fields: bounds, sample_format, colour_bytes, colour_stride_bytes, alpha_bytes, alpha_stride_bytes, colour_sha256, alpha_sha256, transfer, alpha_association, profile; validate_shape(Grid), validate_hashes())}` - canonical pixel contract: `rgb_f32le` colour plane (12 B/px) + SEPARATE straight `f32le` alpha plane (4 B/px), `linear_light`, stride multiple of 4; `nib::{Anchor{position,incoming,outgoing: Point}, Point::pt(x,y), Winding::{NonZero,EvenOdd}}` + `kurbo` (nib pins `=0.13.1`); `accord::{CancellationToken, ActorContext, DomainId}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`.
OPEN DECISION (flag to lead): composite lowers the plan and render-cpu executes it, but Cargo `allowed` gives NO shared edge between them (and render-cpu has no folio). Until the steward decides where a neutral plan IR lives (candidate: a module in `pigment`, both allow it), this slice defines render-cpu's OWN minimal IR (`plan_sink.rs`); do not import composite types.
1. `sample_math.rs`: `pub enum BlendMode{Normal=2, Darken=4, Multiply=5, Lighten=8, Screen=9, Overlay=12, Difference=18}` (STU-RAS-154 discriminants) with `TryFrom<u16>` -> `Err(RenderError::UnsupportedBlend(u16))` for every other member; `pub fn composite_straight(mode:BlendMode, backdrop:[f32;4], source:[f32;4], opacity:f32, coverage:f32) -> [f32;4]` using the W3C Compositing & Blending Level 1 general formula for straight alpha: `as=src.a*opacity*coverage; ao=as+ab*(1-as); C=((1-as)*ab*Cb+(1-ab)*as*Cs+as*ab*B(Cb,Cs))/ao` (ao<=0 -> zeros; as<=0 -> backdrop). Separable `B`: multiply `cb*cs`, screen `cb+cs-cb*cs`, darken/lighten min/max, difference `|cb-cs|`, overlay = hardlight(cs,cb). Arithmetic is space-agnostic: the plan carries `blend_space` label only; NO conversion here.
2. `plan_sink.rs` plan + executor: `pub struct RenderPlan{revision:u64, extent:Rect, profile_sha256:[u8;32], blend_space:BlendSpace/*label*/, ops:Vec<Op>}`, `enum Op{Fill{rect:Rect, rgba:[f32;4], blend:BlendMode, opacity:f32}, Tile{source:SourceId, dst_x:i64, dst_y:i64, blend:BlendMode, opacity:f32}, PathFill{anchors:Vec<nib::Anchor>, closed:bool, winding:nib::Winding, rgba:[f32;4], blend:BlendMode}}`; `trait TileSource: Send+Sync{ fn resolve(&self, id:&SourceId) -> Option<&pigment::ResolvedTile<'_>>; }`. `pub fn render(plan:&RenderPlan, expected_revision:u64, src:&dyn TileSource, chunk:u32, cancel:&CancellationToken, sink:&mut dyn PixelSink) -> Result<RenderReceipt, RenderError>`; `trait PixelSink{ fn accept(&mut self, rect:Rect, colour:&[u8], alpha:&[u8]) -> Result<(), SinkError>; }` (colour/alpha emitted in the SAME canonical planes). Executes per chunk of `chunk x chunk` px (64 default): cancel checked before each chunk, stale `expected_revision != plan.revision` -> `StaleRevision` before any sink call, sink error -> abort, release scratch, no partial publication claims (`RenderReceipt` only on full success). Source tiles pass `validate_shape` + `validate_hashes` before use (bad stride/hash -> typed error).
3. Receipt: `RenderReceipt{revision, profile_sha256, extent, sample_format:"rgb_f32le", alpha:"straight", chunks, ops_executed, peak_scratch_bytes, renderer:"hsk-studio-render-cpu/0.1"}` (STU-ARC-015 binding); `pub const DESCRIPTOR`.
4. Path fill (stretch): flatten `nib::Anchor` list to `kurbo::BezPath` -> polyline (tolerance 0.05 px), 4x4 supersample coverage per pixel with exact winding (NonZero/EvenOdd), feed `coverage` into `composite_straight`; hand-checkable on a 4x4 square (coverage 1.0 inside, 0.0 outside, 0.5 on a half-pixel edge).
5. Observability: `report(&Result<RenderReceipt,RenderError>, cancel, &mut Observe, &mut impl SinkPort)`: Ok -> `Success`; `Canceled` -> `Canceled`; unsupported blend/op -> `Failure(Unsupported)`; stale/bad stride/hash -> `Failure(Validation)`; sink failure -> `Failure(Unavailable)`.
Hand-computed oracle (tiny): backdrop (0.2,0.4,0.6,1.0), source (1,0,0,a=1) opacity 0.5: Normal -> (0.6,0.2,0.3,1.0); Multiply -> (0.2,0.2,0.3,1.0).
Oracle note (Operator; black-box only, no decompilation): later compare against Photoshop-exported renders of a blend-mode ladder PSD (30-mode set) and layered documents: (a) arithmetic space (Photoshop blends gamma-encoded document values by default; Craft source comments claim this plus Vivid Light/Hard Mix/Colour Burn/Dodge edge cases and a 1.45 text gamma - all UNVERIFIED by us), (b) 8-bit rounding, (c) coverage/AA of path edges, (d) opacity/fill/clipping order. Without Adobe-produced references parity is NOT_PROVEN.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- Slice: only `kurbo =0.13.1` (default-features=false, features=["std"]; exact pin already used by nib; Linebender, MIT OR Apache-2.0, pure Rust, prep visual-team lists kurbo/linesweeper as nib candidates). No other crate; do NOT add `image`, `tiny-skia`, `rayon`.
- Candidates later: `rayon` (parallel chunks; Craft pins "1", UNVERIFIED current), `vello_cpu` 0.2 (Craft designcraft/vectorcraft pin; pure-Rust CPU rasteriser, f32 pipeline UNVERIFIED), `tiny-skia` 0.12.0 (BSD-3-Clause, 2026-02-02, verified; 8-bit premultiplied RGBA - format mismatch with f32 straight planes, so rejected for the core).
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-render-cpu --tests` (then `clippy`, `test --test cpu_plan`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/cpu_plan.rs">

## Tests (<=5, `bounded_case`; MT case names in brackets)
1. `known_alpha_mask_sample_math`: the hand-computed Normal/Multiply values above (+ opacity 0 returns backdrop bytes unchanged, source alpha 0 likewise).
2. `chunk_boundary_equals_independent_reference`: 5x3 extent with 2x2 chunks vs a naive per-pixel reference loop in the test; byte-equal planes; ops spanning chunk seams.
3. `sink_error_cancel_releases_bytes`: sink fails on chunk 2 -> `SinkError` surfaced, no receipt; pre-cancelled and mid-run cancelled token -> `Canceled`; scratch counter returns to 0.
4. `unsupported_op_stale_result`: `BlendMode::try_from(26 /*hard_mix*/)` -> `UnsupportedBlend(26)` (no silent Normal); `expected_revision` mismatch -> `StaleRevision` and sink never called; tile with bad stride/hash -> typed error.
5. `receipt_binds_revision_profile_and_observe`: receipt carries revision/profile hash/sample format; `report()` emits exactly one terminal frame.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No GPU, no SIMD dispatch, no rayon, no ICC/profile transforms (prism), no text/glyph drawing (type), no strokes/gradients/patterns/layer effects, no remaining blend modes (hue/saturation/colour/luminosity, dodge/burn, light modes, hard_mix etc. need exact functions per STU-RAS-154 + an Adobe oracle), no dissolve (needs seed contract), no plan lowering from Folio (composite), no 16/32-bit integer paths, no host sink wiring.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
