---
file_id: BUILDER-BRIEF-HSK-STUDIO-MOTION
file_kind: builder_brief
updated_at: 2026-10-11T02:10:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-motion
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-MOTION` / key `motion` / crate `hsk-studio-motion`; foundation MT `MT-35543` (its `implementation_steps` 1-6 are the acceptance script; depends on MT-35536 Type for the full MT, but slice 1 does not need Type). Write scope `crates/hsk-studio-motion/**` only.
- Responsibility (verbatim): "Keyframes/easing and temporal/property-expression/time/seed/dependency policy; typed provider ports preserve pure Rust domain closure. Static composition plan belongs to Composite; optional JS engine belongs to MotionJS. No vendor runtime or unconditional NLE/audio/native provider dependency."
- Foundation capability: "Evaluate keyframe/expression-policy operation at exact ticks with deterministic result/rejection."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified identical): `hsk-studio-accord`, `hsk-studio-folio`, `hsk-studio-nib`, `hsk-studio-type`, `hsk-studio-observe`. Slice 1 uses accord + observe only (folio `Operation/Port/Edge` is a LAYER operation graph, not a property dependency graph - not reused; nib/type reserved for spatial paths and text animators).
- Forbidden: handshake_core/native/SurrealDB; any JS engine, rquickjs/boa, codec/device/plugin crate in motion (they live in motion-js/others); `eval`-style own JS interpreter (adoption review rejects building a second JS translator); edges outside `allowed`.
- Conventions (verified in existing crates): edition 2024, rust-version 1.97.1; path dep `hsk-studio-accord = { path = "../hsk-studio-accord", version = "=0.1.0", default-features = false }` (single-line inline tables only, see `builders/WORKSPACE_RULES.md`); `[lib] path="src/lib.rs"`; `[[example]] name="motion-consumer"`; `pub const DESCRIPTOR: &str` JSON; tests in `tests/foundation_contract.rs` with `bounded_case` 30 s wrapper. New crates.io deps only via `builders/dep-requests.jsonl`.
- Prep refs (reuse, do not re-derive): `docs_repo/studio-collaboration-2026-10-09/modular-authoring/media-team/preparation.json` -> `module_proposals.motion` (sources MOT-020/074/076/256/259, steps, oracle, open questions); `backend-redteam/expression-provider-adoption.json` (port shape: `ports.inputs/outputs/forbidden`); `.GOV/reference/studio_app_feature_research/05-studio-primitive-map.md` (motion_timeline/keyframes owned by `studio_interaction`).

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`, 14.26 lines ~25629-29210)
- STU-MOT-010/010a (l.25818-25826): `StudioProperty` is a value stream; fields `state` (static|keyframed|expression_driven|expression_over_keyframes), `static_value` (retained under animation - MOT-020b: disabling all keys returns the static value), `keyframes`, `expression`, `expression_enabled`, `dimension_count`, `min_value/max_value`, `is_spatial`. MOT-011: property addressing by stable path (`composition_id / layer_id / group_key / ... / property_key`), never display names/indices.
- STU-MOT-020/020a (l.27321): time is a rational over the composition frame rate, quantised to frames by default; values are `f64` (doubles, not floats) - 32-bit loses frame accuracy. STU-MOT-030/030a (l.27348): keyframe `{time, value[], in_interpolation, out_interpolation, in/out_temporal_tangent per component {speed, influence}, roving}`; in and out interpolation are INDEPENDENT. STU-MOT-031 (l.27376): `linear|bezier|continuous_bezier|auto_bezier|hold`; `hold` out makes the segment a step. MOT-031b: non-`interpolable` property behaves as `hold`.
- STU-MOT-033/034/034a (l.27398-27415): temporal tangent `{speed (units/s), influence (fraction 0..1 of segment duration)}`; default ease influence `0.16666666666` exactly; store influence as fraction, >=5 decimals. Two keys + four tangent halves fully determine a segment. Spatial tangents (036) and roving (035) are later (gap MOT-141).
- STU-MOT-070/071/076/077/078 (l.27568-27744): expression is an alternative to the value; four-state machine; evaluation deterministic and side-effect-free (same state + time => same value; no wall clock); dependency cycle = validation error naming the participating property paths, refused at commit, never a hang; expression error disables it, falls back to underlying state with source position, render receipt must report disabled-by-error.
- STU-MOT-255..259 (l.29200-29210, v02.207 ADD): expression profile is synchronous ES2018-shaped JS owned by MotionJS; hosts read immutable revision + canonical evaluation tick + explicit seeds; **MOT-256 Date clock (exact)**: persisted `epoch_ms` (UTC integer ms since 1970-01-01T00:00:00.000Z, inclusive +-8_640_000_000_000_000, default 0) + `origin_tick` (default 0); `N = epoch_ms*254016000000 + (evaluation_tick - origin_tick)*1000` in checked wide math; reject `|N| > 8_640_000_000_000_000*254016000000`; `utc_ms = trunc_toward_zero(N / 254016000000)`, zero canonical; MOT-257 calendar rules (proleptic Gregorian UTC, floor-div/nonneg modulo for negatives, toISOString 4-digit years 0000..9999 else signed 6 digits, always `.sssZ`); MOT-259 stable typed outcomes (parse/unsupported/alias/reference-missing/ambiguous/dependency-cycle/budget/output-type) with revision/tick/seed attribution; pure Motion MUST resolve without MotionJS C closure.
- Gap (do NOT invent): seeded-random algorithm and the 37 host-read argument signatures are referenced by MOT-074/076 but not published as an algorithm (media-team prep question) -> only carry an explicit `Seed(u64)` identity in receipts, no RNG.
- Later: MOT-001..009 composition/layers, 036 spatial, 037/038 assistants and keyframe ops, 045 time remap, 050-055 graph editor, 090-095 text animators, 100-104 shape operators, 110-112 presets, 120-126 render queue (Composite owns plan). MT corpus: motion feature MTs approx MT-33848..MT-34069 (UNVERIFIED exact range).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (pure Rust, std only, ~800-1100 LOC)
Reuse (verified): `hsk_studio_accord::{Ticks (u64 stored), TICKS_PER_SECOND, CancellationToken (check()), DomainId (SMTL/SPTF prefixes valid), ActorContext}`; `hsk_studio_observe::{Observe, Observation, Outcome, FailureCode, SinkPort}`. Accord ticks are `u64`, but effective sample ticks may be negative after remap/posterize (MOT-256 oracle needs it): add `pub struct EvalTick(i64)` (`from_stored(Ticks)` checked, `new(i64)`).
1. Keyframe evaluation: `pub enum Interpolation{Linear,Bezier,ContinuousBezier,AutoBezier,Hold}`, `pub struct TemporalTangent{speed:f64, influence:f64}` (`DEFAULT_INFLUENCE = 0.16666666666`; influence outside 0..=1 / non-finite rejected), `pub struct Keyframe{time:Ticks, value:Vec<f64>, in_interp, out_interp, in_tangent:Vec<TemporalTangent>, out_tangent:Vec<TemporalTangent>}`, `pub struct Property{key:String, static_value:Vec<f64>, keys:Vec<Keyframe>, hard_min/hard_max:Option<f64>}`; `Property::new(..)` validates unique strictly-increasing times, equal component counts. `Property::sample(&self, tick:EvalTick, cancel) -> Result<Vec<f64>, MotionError>`: before first/after last key = held end values; segment [k0,k1] uses `k0.out_interp` for departure and `k1.in_interp` for arrival (independent); Linear = lerp; Hold = k0 value; Bezier family = cubic Bezier in (time, value) space with control points `P1=(t0 + infl_out*dur, v0 + speed_out*infl_out*dur_seconds)`, `P2=(t1 - infl_in*dur, v1 - speed_in*infl_in*dur_seconds)`, solve x(s)=t by bounded bisection/Newton (fixed iteration cap, deterministic), then y(s); per-component; result checked finite (`NonFinite`) and clamped only if hard bounds declared (report, do not silently clamp: return `OutOfBounds`).
2. Property state machine + expression policy: `pub enum PropertyState{Static,Keyframed,ExpressionDriven,ExpressionOverKeyframes}`; `pub trait ExpressionProvider: Send + Sync { fn evaluate(&self, req:&ExprRequest<'_>, host:&dyn HostRead, fuel:&mut Fuel, cancel:&CancellationToken)->Result<Vec<f64>, ExprError>; }` and `pub trait HostRead{ fn read(&self, path:&PropertyPath, tick:EvalTick, fuel:&mut Fuel)->Result<Vec<f64>, ExprError>; }`; `ExprRequest{source:&str, revision:u64, tick:EvalTick, seed:u64, base:Option<&[f64]>}` (no provider/engine types in public API). `Property::evaluate_with(.., provider)` implements MOT-071/078: expression error => property marked `disabled_by_error{code, span}`, value falls back to keyframed/static, receipt says so, nothing panics, no partial value. `Fuel` is a shared allowance across nested reads/retries (`charge(n)` -> `BudgetExceeded`).
3. Dependency graph: `pub struct PropertyPath(String)` (validated stable-key segments joined by `/`, rejects empty, `..`, control chars, >256 bytes), `pub struct DepGraph`; `DepGraph::commit_expression(path, reads:&[PropertyPath]) -> Result<(), MotionError::Cycle{participants:Vec<PropertyPath>}>` (iterative DFS/Kahn, no recursion limit crash) and `eval_order() -> Vec<PropertyPath>` topological. Missing/ambiguous reference => typed `ReferenceMissing/Ambiguous`.
4. Deterministic Date clock (MOT-256/257 subset, fully testable with integer oracle): `pub struct DateProfile{epoch_ms:i64, origin_tick:u64}` (`new` validates +-8.64e15); `pub fn utc_ms(&self, eval:EvalTick)->Result<i64, MotionError>` using `i128`, trunc toward zero, canonical zero, overflow reject; `pub fn to_iso_string(utc_ms:i64)->String` (days-from-civil algorithm, floor division, 4-digit year or signed 6-digit, always `.sssZ`) and `parse_iso_exact(&str)->Option<i64>` for `YYYY-MM-DDTHH:mm:ss[.sss]Z` only (no whitespace/offset/date-only forms).
5. Receipt + observability: `pub struct EvalReceipt{revision:u64, tick:EvalTick, seed:u64, profile:&'static str, state:PropertyState, disabled_by_error:Option<ExprErrorCode>, fuel_used:u64}`; `DESCRIPTOR` JSON; `report(result, cancel, &mut Observe, &mut impl SinkPort)` (Success/Canceled/Failure(Validation|Unsupported)); example `motion-consumer` (`--descriptor`, `--keys "t:v:in:out,..." --tick N`) prints sampled value + receipt.
Oracle/vectors (MT-35543 steps 1,6): scalar and 3-component interpolation at endpoints and midpoint, plus negative effective ticks, against independently computed analytic values (e.g. linear 0->10 over 1 s: value(0.5 s)=5 exactly = tick `127_008_000_000`); Date: `epoch_ms=0,origin 0,tick 0 -> 0`, `-1 ms (tick -254_016_000)` -> trunc toward zero, `utc_ms(max)=8_640_000_000_000_000` -> `+275760-09-13T00:00:00.000Z`, one past => reject; cycle A->B->A names both paths; stale `revision` rejected.
Oracle (black-box vs Adobe, later, no decompilation): export per-frame keyframe values from installed After Effects via its public scripting/export surface for a 2-key comp (default easy-ease, speed/influence non-default, hold, asymmetric in/out) and compare Studio `sample()` to those values at every frame (tolerance 1e-6 relative); likewise compare `Date` outputs only against the MOT-256/257 spec vectors (engine behaviour is MotionJS's). Note MOT-034 default 0.16666666666 vs the commonly seen 33.33% easy-ease influence is a spec claim to confirm black-box before locking the default.

</topic>

<topic id="deps" status="active" summary="third-party deps">

## Deps
- Slice 1: NO new crates.io dependency (std only). Do not add serde/serde_json/kurbo in slice 1; later spatial work reuses `kurbo 0.13.1` already pinned through nib (pure-rust, steward registry) via nib, not a new edge.
- Provider side (NOT in this crate): pure-Rust candidate `boa_engine 0.22.0` (Unlicense OR MIT; updated 2026-08; large dependency set; pure Rust but lacks proven aggregate quota/deadline per prep) and native candidate `rquickjs 0.14.0` (MIT; QuickJS-NG 0.16.2 C) - see `hsk-studio-motion-js` brief; verified in `backend-redteam/expression-provider-adoption.json` `pins` + crates.io 2026-10-11.
- Build/test (never on C:): `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-motion --tests` (then clippy, `cargo test -p hsk-studio-motion`).

</topic>

<topic id="tests" status="active" summary="<=5 focused tests">

## Tests (<=5, `bounded_case`)
1. `keyframe_sampling_analytic`: linear scalar + 3-component at endpoints/midpoint/negative ticks vs analytic values; hold step; asymmetric in/out (linear in, hold out); bezier with speed 0 / influence 1/3 symmetric segment is odd-symmetric about midpoint (value(mid) == (v0+v1)/2 within 1e-12).
2. `tangent_defaults_and_rejects`: default influence constant exact; influence 1.5, NaN speed, duplicate key times, unsorted keys, mismatched component counts all rejected with distinct codes.
3. `dependency_cycle_and_order`: A reads B reads A -> `Cycle{participants}` listing both paths; DAG yields deterministic topological order; missing reference typed.
4. `date_clock_vectors`: MOT-256 endpoints/negative truncation/overflow boundary and ISO formatting for years 0000, 1970, 9999, 10000, -1 and the +-8.64e15 limits; `parse_iso_exact` rejects offset/date-only/24:00.
5. `expression_policy_fallback`: test-double `ExpressionProvider` (in test file only) returning value / error / budget-exceeded / cancel: state `expression_over_keyframes` gets `base`, error disables expression and falls back with `disabled_by_error`, cancel -> `Canceled`, stale revision rejected.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No JS parsing/execution (MotionJS MT-35563), no seeded RNG/wiggle/noise (algorithm unpublished), no spatial tangents/roving/time-remap/assistants (MOT-035..045), no composition/layer model or render plan (Composite), no text animators/shape operators, no Folio mapping of `hsk.studio.property@1`, no GUI/graph editor, no Controls wiring.
- Wave: 1 (needs only accord + observe). Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
