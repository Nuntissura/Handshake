---
file_id: BUILDER-BRIEF-HSK-STUDIO-DEVELOP
file_kind: builder_brief
updated_at: 2026-10-11T04:45:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-develop
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-DEVELOP` / key `develop` / crate `hsk-studio-develop`; foundation MT-35538 (test target `develop_recipe`, case `unsupported_process_version_and_cancel_stale`); write scope `handshake-studio-assets/crates/hsk-studio-develop/**`. Files per MT: `src/recipe.rs`, `src/pipeline.rs`, `tests/develop_recipe.rs`.
- Responsibility (verbatim): "RAW decode/develop recipes; originals immutable through CKC/ArtifactService provider identity, target-independent scalar oracle and SIMD dispatch. Imported catalog semantics adapt into existing CKC owner, not a private Studio catalog."
- Foundation capability: "Execute bounded source/develop recipe on known pixel input; source immutable, missing RAW decoder explicit."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-pigment`, `hsk-studio-prism`, `hsk-studio-observe`. NO `folio` edge (STU-CON-045 forbids a direct Folio edge: reuse Pigment geometry). Slice uses accord, pigment, observe; prism only when profile transform lands.
- Forbidden: handshake_core/native/SurrealDB; private catalog/recipe-history DB (ENG-DAG-FORBID-003); GPU/worker pool/codec libs; `unsafe` SIMD in the first slice (scalar oracle first).
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `develop-consumer` (`--descriptor`), 30 s `bounded_case`. Root `Cargo.toml` steward-owned; dep needs via `builders/dep-requests.jsonl`.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`, read l.22572-22600 IN FULL before coding - it is the exact numeric contract)
- STU-CON-042 (l.22573): ephemeral `develop_recipe` transport over caller-granted ALREADY-DECODED camera CFA bytes (`decoded_camera_cfa_u16le`); no RAW container decoder, no new schema/prefix; only `process_version = 1.0`, `engine_version = 1.0`, `math_token = native_cfa_bilinear_hermite_v1`, `demosaic_algorithm = bilinear_reflect_native_v1`; imported process 15.4/engine 16.4 or any other token REFUSES (no auto-mapping); original labels preserved (STU-RAW-104).
- STU-CON-043: plane limits (Bayer 2..256 sides, X-Trans 6..256), even stride >= 2*width and <= 1024, total <= 262144 B, u16le row-major, padding never a sample; patterns RGGB/BGGR/GRBG/GBRG (+ fixed X-Trans 6x6), `phase_x/phase_y` in 0..period-1, floor-mod of `sensor_origin+x+phase`; per-channel `black_level_u16[3]`/`white_level_u16[3]`, white > black, sample outside its channel's closed interval = unsupported (NO clamp); normalise `binary64((s-black)/(white-black))` then ONE binary32 rounding; missing channel = mean of same-channel sites in radius 1 then radius 2 (dy outer, dx inner, binary32 accumulate in visit order, reflect at edges with period `2*(extent-1)`); none -> `missing_colour_support`; `execution_hint = scalar|sse2|auto`.
- STU-CON-044: `white_balance_mode = as_shot` only: `r[i]=1/neutral[i]; m=min(r); gain[i]=r[i]/m`; calibration `camera_to_profile_rgb` 3x3 binary64 row-major, row = `(m0*c0+m1*c1)+m2*c2`, no FMA, then binary32; `exposure` in `stops_ev` -> multiply by `exp2(exposure)`; 12 `enable_*` booleans retained; `resolved_curves` (composite/red/green/blue, monotone Hermite, 2..32 points); normalised crop `crop_left/top/right/bottom` (floor/ceil bounds), angle/flips must be identity else refuse; nothing silently clipped or zeroed.
- STU-CON-045: output tiles via existing Pigment `Grid`, `output_origin_x/y_px` map, alpha 1.0 only inside crop, preserve preimage bytes outside; STU-RAW-003/-101..-131 (feature scope), STU-ARC-012/020. Later scope: all of 14.12 (l.1925-2926: tone, HSL, grading, optics, geometry, masking, enhance, HDR), STU-RAW-003 real RAW containers (DNG/CR3/ARW...), MT plan `_mt_plan_1213_raw_interop.json` MT-411..432.

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, scalar, ~700-1000 LOC)
Prep reuse: visual-team `preparation.json#/representative_proposals[3]` (MT-35538; "Immutable process-version/enable-flag recipe, bounded CFA decode/develop chunks, scalar reference then optional SIMD dispatch; original bytes stay CKC-owned"; cases `disabled_group_preserves_recipe_and_pixels`, `scalar_simd_cfa_edges_equivalent`, `recipe_reedit_preserves_original`, `unsupported_process_version_and_cancel_stale`; provider "Lightcraft develop/scalar bilinear references; exact RAW decoder and SIMD feature pins unresolved, missing decoder explicitly blocked"; oracle "Analytic CFA matrix for component correctness and original Camera Raw/Lightroom-produced output with exact process/version/profile for feature acceptance"). Craft reference (pinned cache `.../gov_runtime/tools/studio-craft-source-cache/lightcraft-88ea1a80.../crates/{develop,raw/src/demosaic/bilinear.rs}`): read for edge handling only; the NATIVE spec math above wins on any difference. Also `.GOV/reference/studio_app_feature_research/19-studio-local-first-rust-posture.md`, `11-provider-posture-map.md` (AI enhance/denoise = parked adapters, not here).
Reuse (verified): `pigment::{Rect{x,y,width,height}, Grid{origin_x,origin_y,tile_width,tile_height}.bounds(col,row)}` (pixel contract of tiles: `rgb_f32le` colour plane + SEPARATE straight `f32le` alpha plane, `linear_light`, stride multiple of 4); `accord::{CancellationToken, ActorContext, DomainId (SDOC/SLYR prefixes)}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`.
1. `recipe.rs` - typed, validated recipe: `pub struct DevelopRecipe{process_version:(u32,u32), engine_version:(u32,u32), math_token:String, demosaic_algorithm:String, white_balance_mode:WbMode, as_shot_neutral:[f64;3], camera_to_profile_rgb:[f64;9], exposure_stops:f64, enables:Enables /*the 12 booleans, all required*/, crop:NormalizedCrop}`; `DevelopRecipe::validate(&self)->Result<(), DevelopError>` returning `UnsupportedProcessVersion{process,engine}` for anything but 1.0/1.0 + exact tokens, `UnsupportedWhiteBalance`, `NonFinite`, `NotInvertibleMatrix`, `UnsupportedCrop` (angle/flip non-identity). Original labels stay as given (no rewrite). `Enables` gate only what is supported (`enable_tone_curve`); any other enabled-but-unsupported group -> `UnsupportedContribution(name)`, disabled groups are preserved untouched in the recipe value (test `disabled_group_preserves_recipe_and_pixels`).
2. `pipeline.rs` source + demosaic: `pub struct CfaPlane{width:u32,height:u32,stride_bytes:u32,sensor_origin:(i64,i64),cfa:Cfa{pattern:Bayer::{Rggb,Bggr,Grbg,Gbrg}, phase_x:u8, phase_y:u8}, black:[u16;3], white:[u16;3], bytes:Vec<u8>}`; `CfaPlane::new(...)->Result<..>` enforces every STU-CON-043 limit (checked u32 math, padding ignored, out-of-range sample -> `SampleOutOfRange`); `pub fn demosaic_normalized(&CfaPlane, &CancellationToken) -> Result<Vec<[f32;3]>, DevelopError>` implementing the exact visit/reflect/accumulate order (scalar); X-Trans -> `Unsupported` in the slice (typed, not aliased).
3. Colour stages + crop + output: `pub fn develop(plane:&CfaPlane, recipe:&DevelopRecipe, grid:Grid, output_origin:(i64,i64), exec:ExecutionHint, cancel:&CancellationToken, cb:&mut dyn FnMut(DevelopedTile)->Result<(),SinkError>) -> Result<DevelopReceipt, DevelopError>`; per pixel: normalised demosaic sample -> gain (binary64) -> matrix (written products, no FMA) -> exposure `exp2` -> `f32` (refuse any nonzero->0 or non-finite result, no clipping) ; `working_profile == output_profile` is the ONLY supported mode (identity; a differing output profile -> `UnsupportedProfileTransform` until the Prism chunked-64px route lands); crop = floor/ceil of normalised edges mapped to `output_origin`; `DevelopedTile{bounds:Rect, colour:Vec<u8> /*rgb_f32le*/, alpha:Vec<u8> /*f32le = 1.0 inside crop*/}` emitted per grid tile intersecting the crop (bounded chunks: cancel checked per tile). `ExecutionHint::{Scalar,Auto,Sse2}`: `Sse2` -> `Unsupported` in this slice, `Auto` records `route:"scalar"` in `DevelopReceipt{source_sha256, recipe_revision, math_token, route, tiles, pixels}` (never claim SIMD).
4. Immutability + refusal path: `develop` takes `&CfaPlane` (never mutates), returns before any callback when validation, stale `expected_source_revision`, cancel or unsupported input fires; after a mid-run cancel/sink error the plane hash (`pigment::wire::digest`/own sha not needed: compare `bytes` clone in test) is unchanged and no receipt is returned (`partial publication` forbidden).
5. Observability: `DESCRIPTOR` JSON + `report(&Result<DevelopReceipt,DevelopError>, cancel, &mut Observe, &mut impl SinkPort)`: unsupported process/engine/mode/profile -> `Failure(Unsupported)`, invalid plane/range/non-finite -> `Failure(Validation)`, missing RAW container decoder -> `Failure(Unavailable)` (`develop` offers `decode_container(&[u8])` that ALWAYS returns `Err(DecoderUnavailable)` - the explicit "missing RAW decoder" outcome the capability text requires), `Canceled` -> `Canceled`. Example `develop-consumer --descriptor | --plane FILE ... [--cancel]`.
Hand-computed analytic oracle: uniform plane (all samples = black + 0.5*(white-black), RGGB 4x4, neutral [1,1,1], identity matrix, exposure 0) -> every output pixel (0.5,0.5,0.5) exactly; a 4x4 RGGB plane with distinct R/G/B levels: interior R pixel's G = mean of its 4 orthogonal-neighbour G sites only if radius-1 square rule says so - compute by hand from the spec's visit rule and assert exact f32 bits.
Oracle note (Operator; black-box only, no decompilation): the native 1.0 semantics are NOT Adobe's. Later parity row: render the same RAW through Camera Raw/Lightroom (Adobe installed) at a named process version and compare against an equivalent Studio recipe once a RAW container decoder exists; compare demosaic edge handling, black/white normalisation, as-shot WB gains, and profile/exposure; never map labels automatically (STU-CON-042).

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- None for the slice (std + accord/pigment/observe). Float discipline: no `mul_add`, no fast-math; Rust does not fuse by default - assert in a test that `a*b+c` bits equal the unfused expectation on a crafted input.
- Later candidates (not now): `rawler`/`rawloader`-class RAW container decoders (UNVERIFIED, none evaluated; STU-RAW-003 requires a public-spec decoder decision), `rayon` for tile parallelism (Craft pin "1"), SIMD via `core::arch` only after the scalar oracle (STU-CON-043 SSE2 route), `moxcms` via prism for profile transforms.
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-develop --tests` (then `clippy`, `test --test develop_recipe`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/develop_recipe.rs">

## Tests (<=5, `bounded_case`; MT case names)
1. `analytic_cfa_matrix`: the uniform-plane and tiny RGGB cases above, exact f32 bits; all four Bayer patterns + a non-zero phase give the same colour at corresponding sites; edge/corner pixels use reflection (period `2*(extent-1)`).
2. `disabled_group_preserves_recipe_and_pixels`: toggling a disabled supported group (`enable_tone_curve=false`) leaves recipe value equal and pixels identical; an ENABLED unsupported group (`enable_detail=true`) -> `UnsupportedContribution("detail")`, no tile emitted.
3. `recipe_reedit_preserves_original`: develop twice with a changed exposure; the `CfaPlane` bytes are byte-identical before/after; receipts carry the same `source_sha256`.
4. `unsupported_process_version_and_cancel_stale`: process 15.4/engine 16.4 -> `UnsupportedProcessVersion`; wrong token -> refuse; stale source revision; pre-cancelled and mid-run cancel -> `Canceled` with no receipt; `decode_container(b"II*\0")` -> `DecoderUnavailable`.
5. `limits_and_range`: width 1 / 257, stride odd or > 1024, length mismatch, white <= black, sample outside `[black,white]` -> distinct `DevelopError::code()`; a normalised nonzero value that underflows f32 -> refuse; `scalar_simd_cfa_edges_equivalent` is satisfied as `Sse2` -> typed Unsupported (scalar oracle only) until SIMD lands.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No RAW/DNG container decode, X-Trans demosaic, SIMD route, other white-balance modes, tone-curve Hermite (add next: formulas in l.22600), Prism profile transforms, masks/local adjustments, lens/geometry/HDR/denoise/enhance (AI parked), catalogs/presets/snapshots, `.lrcat` (separate `interop-lrcat`), pigment leases/masked-patch publication (STU-CON-045 full producer), GUI, host wiring.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
