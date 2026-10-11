---
file_id: BUILDER-BRIEF-HSK-STUDIO-CONTROLS
file_kind: builder_brief
updated_at: 2026-10-11T07:20:00Z
wp: WP-KERNEL-STUDIO
crate: hsk-studio-controls
---

<topic id="identity" status="active" summary="module identity, edges">

## Identity
- Module `STUDIO-MODULE-CONTROLS` / key `controls` / crate `hsk-studio-controls`; foundation MT-35553 (planned test file `tests/foundation_numeric.rs`; per-test deadline 60 s per prep); write scope `handshake-studio-assets/crates/hsk-studio-controls/**`.
- Responsibility (verbatim): "One shared egui control/panel mechanic configured by capability registry; agent projections/manual use same semantics; never imports full native app."
- Foundation capability: "One real configured shared control edits typed value and exposes same descriptor to model/manual/accessibility; no per-domain mechanic clone."
- Allowed deps (STUDIO_MODULES == root Cargo.toml `allowed`, verified): `hsk-studio-accord`, `hsk-studio-observe`. NOTE: `controls` is itself a dependency of `harness` (allowed there) - keep the public API small and stable.
- Forbidden: handshake_core/handshake_native/`eframe`/native shell/SurrealDB/GPU/codecs (MT-35553 step 1); a second numeric/clamp/step implementation anywhere (STU-FX-110a: "no domain creates a copied clamp, step or gesture implementation"); the reducer declaring a proposal "committed" (an injected document actor owns commit).
- Conventions: edition 2024, rust-version 1.97.1, path deps `{ path="../hsk-studio-x", version="=0.1.0", default-features=false }`, `pub const DESCRIPTOR`, example `controls-consumer` (`--descriptor`), `bounded_case` (60 s). Root `Cargo.toml` steward-owned; the egui pin needs a dep-request.

</topic>

<topic id="spec" status="active" summary="anchors for first slice">

## Spec (`.GOV/spec/master-spec-v02.215/spec-modules/14-studio-creative-suite.md`)
- STU-ARC-014 (l.~199): ONE reusable panel/control mechanic with typed capability configuration (text fields, sliders, swatches, menus, windows, gizmos, timeline values); the same descriptors feed structured tools, AccessKit, UserManual and Argus; agent-local view/selection/navigation MUST NOT change operator focus or another agent's view; actor attribution reconciles `agent_label` with `client_session_id` through one adapter.
- STU-FX-104 (l.12570): hard bounds are the acceptance range - out-of-range typed entry is an ERROR naming parameter, offending value and both bounds, NEVER a silent clamp; STU-FX-105: soft bounds only govern presentation/scrub travel, a value outside soft and inside hard is legal, succeeds and the control shows an explicit out-of-soft-range state; STU-FX-107 (l.12624): collapsing hard+soft into one `(min,max)` pair is FORBIDDEN in any type/wire/UI model - always four fields; STU-FX-109: `precision` = declared decimal places; STU-FX-110 (l.12666): `step = 10^-precision`, `fine_step = step/10`, `coarse_step = step*10`; pointer sensitivity is a separate field and modifiers apply once; STU-FX-110a: pointer scrubbing clamps ONLY hard bounds and crosses soft bounds; exact typing bypasses resistance (in-hard-range ok, out-of-hard-range rejects); pointer/typed/arrow/accessibility/model routes use one descriptor+validation owner; STU-FX-149 closes modifier defaults; STU-SHL-190..192 (shell sensitivity/resistance - read before adding pointer resistance); STU-FX-123 (schema must expose both bound pairs).
- STU-ARC-015 (visual receipts), STU-ARC-016, STU-QUIET (14.20 l.22360: no foreground windows/focus theft), 14.22 UserManual (l.22396), 14.16 model visibility (l.22290), 14.21 operator unification (l.22378).
- Later scope: panels/windows/gizmos/swatches/menus/timeline values, drag-reorder, dockable panes (host owns egui_tiles), `_mt_plan_00_foundation.json` MT-009/010/015, shell MT family (STU-SHL-*).

</topic>

<topic id="minimal-slice" status="active" summary="1-3 h vertical slice">

## Minimal slice (~700-1000 LOC; core is egui-free, adapter is a module)
PREP DECISION (shared-team `preparation.json#/representative_authored_field_proposals` module `controls`, MT-35553 + `shared_contract_proposals.provider_selection`): "Controls selects headless egui 0.33 mechanic with inspected host egui 0.33 family, no eframe/native device"; descriptors: "One owned command/control schema supplies model, AccessKit, UserManual and Argus descriptors; no independently maintained manual or UI command semantics"; MT upstream reference: `designcraft@66c7ce7e.../crates/ui-egui/src/widgets.rs:1-205` (`NumField`: stable IDs + configuration reusable; REPLACE its `NumKind::parse` character-stripping (accepts `12+3` arithmetic and drops junk characters) and unconditional range clamp). Pinned Craft cache: `.../gov_runtime/tools/studio-craft-source-cache/designcraft-66c7ce7e.../crates/ui-egui/src/widgets.rs` (MIT OR Apache-2.0).
MT-35553 fixed fixture (verified in the MT json): scalar `precision 2`, `initial 0.50`, `hard [0,1]`, `soft [0.2,0.8]`, revision `fixture-R0`; exact text `0.75` -> proposed 0.75; `1.25`, non-finite, stale revision, revoked grant, Escape/cancel -> typed rejection/discard preserving value 0.50 and the original revision; pointer movement clamps to hard bounds and may cross soft bounds.
Reuse (verified): `accord::{CancellationToken, ActorContext (attribution), Unit/Length}`; `observe::{Observe, Observation, Outcome, FailureCode, SinkPort}` (allowlisted counters only; denied values stay redacted).
Version contradiction check: HOST lock (`handshake_main/src/frontend/handshake_native/Cargo.lock`) = egui/eframe/egui_kittest 0.33.3, accesskit 0.21.1, wgpu 27.0.1; Craft projects use egui 0.36. STU-ARC-018 says the host GUI family stays at its inspected pins -> use `egui =0.33.3` (NOT 0.36).
1. `descriptor.rs`: `pub struct NumericControl{control_id:String /*stable, becomes egui::Id source*/, label:String, property:PropertyAddress{owner_id:String, key:String}, unit:Option<Unit>, precision:u8, hard_min:Option<f64>, hard_max:Option<f64>, soft_min:Option<f64>, soft_max:Option<f64>, step:f64, fine_step:f64, coarse_step:f64, enabled:bool}` with `NumericControl::new_scalar(..)` DERIVING step/fine/coarse from precision (STU-FX-110) and validating `hard_min<=soft_min<=soft_max<=hard_max` where present (`InvalidDescriptor`); four-field bounds ALWAYS (never a single pair, STU-FX-107). `pub struct ControlState{value:f64, revision:String}`.
2. `reducer.rs` (the single owner of legality): `pub enum Action{Text(String), Pointer{target:f64}, Arrow{dir:i8, modifier:Modifier::{None,Fine,Coarse}}, AccessibilityIncrement{dir:i8}, ModelSet(f64), Cancel}`; `pub fn reduce(c:&NumericControl, s:&ControlState, a:&Action, ctx:&ActionContext{expected_revision:String, grant:GrantState::{Active,Revoked}, actor:ActorContext}, cancel:&CancellationToken) -> Outcome`; `Outcome::{Proposed(Proposal{value:f64, from_revision:String, soft_state:SoftState::{InRange,OutOfSoft}, route:Route}), Rejected(Rejection), Discarded}` with `Rejection::{InvalidText, NonFinite, OutOfHardBounds{value,hard_min,hard_max}, StaleRevision{expected,current}, GrantRevoked, Disabled, Canceled}`. Rules: `Text` parsed by a STRICT decimal parser (optional sign, digits, one dot, optional exponent, optional exact unit suffix equal to `c.unit`; no arithmetic, no junk-stripping; reject `0.75abc`; value kept at full parsed precision, display rounding only), hard-bound violation = REJECT with both bounds (never clamp), soft violation within hard = `Proposed` with `OutOfSoft`; `Pointer` clamps ONLY to hard bounds and may cross soft; `Arrow/AccessibilityIncrement` move by step/fine/coarse once (no double modifier), staying within hard bounds (at a bound: `Rejected(OutOfHardBounds)` or no-op documented - pick reject, record it); `Cancel`/Escape -> `Discarded` (state untouched); stale revision or revoked grant are checked BEFORE any proposal is produced; the reducer never mutates `ControlState` and never reports "committed".
3. Projections from the SAME descriptor (`projection.rs`): `model_json(&NumericControl, &ControlState) -> String` (hand-written deterministic JSON incl. all four bounds, step trio, unit, precision, allowed actions), `manual_topic(&NumericControl) -> ManualTopic{id, title, body}` and `access_props(&NumericControl, &ControlState) -> AccessProps{role:"spin_button", label, value_text, numeric_value, min, max, step}`; a test asserts the three agree field-for-field and that stable ids are identical (`control_id` everywhere).
4. Actor views (STU-ARC-014): `ViewRegistry` keyed by `ActorKey{principal_id, session_id}` (from `accord::ActorContext`): per-actor `focused_control:Option<String>`; moving agent A's focus never changes the operator's or agent B's (test both ways); value/revision live in `ControlState` only.
5. `egui_adapter.rs` (module `adapter_egui`): `pub fn show(ui:&mut egui::Ui, c:&NumericControl, s:&ControlState) -> Option<Action>` emitting `Action`s from a text field + drag + arrow keys with `egui::Id::new(&c.control_id)`; headless proof drives `egui::Context::run` frames with synthetic `RawInput` events (type "0.75" + Enter; Escape; drag) and feeds the resulting `Action` into `reduce` - NO window, NO eframe, NO wgpu. If `ctx.run` needs fonts, keep egui's `default_fonts`; the `accesskit` feature gives `accesskit_update` in `PlatformOutput` to inspect role/value (egui 0.33.3 feature `accesskit` verified; API names UNVERIFIED, read docs.rs/egui/0.33.3).
6. Observability: `DESCRIPTOR` JSON + `report(&Outcome, cancel, &mut Observe, &mut impl SinkPort)`: `Proposed` -> `Success`, `Rejected(*)` -> `Failure(Validation)` (values redacted: only the rejection code), `Canceled|Discarded` -> `Canceled`. Example `controls-consumer --descriptor | --text 0.75`.
Oracle note: not an Adobe-behaviour row (Adobe UI is not copied); the oracle is the MT fixture numbers above plus STU-FX-104..110a rules.

</topic>

<topic id="deps" status="active" summary="third-party deps (verified 2026-10-11)">

## Deps
- `egui =0.33.3` with `default-features=false, features=["default_fonts","accesskit"]` (docs.rs 0.33.3: default = `default_fonts` only; `accesskit` optional and enables `dep:accesskit`; host lock shows accesskit 0.21.1 - let cargo resolve that same version through egui, do not add accesskit directly). MIT OR Apache-2.0 (egui; UNVERIFIED on crates.io page, standard). NOT `eframe`.
- dev-dependency `egui_kittest =0.33.3` ONLY if the plain `egui::Context::run` route is not enough (docs.rs: 5 features, NO defaults; `wgpu`/`snapshot`/`eframe`/`x11` pull heavy crates - do not enable). Host uses `egui_kittest = { version = "0.33", features = ["wgpu"] }`; the asset must stay GPU-free.
- Not needed: `accesskit` direct, `serde` (hand-written JSON like the other foundations), `kurbo`.
- Build/test: `cd "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-studio-assets/handshake-studio-assets" && source "D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/docs_repo/wp-studio-module-work/builders/studio-env.sh" && cargo check -p hsk-studio-controls --tests` (then `clippy`, `test --test foundation_numeric`). egui is a mid-size build: build once in the shared warm target.

</topic>

<topic id="tests" status="active" summary="<=5 focused tests in tests/foundation_numeric.rs">

## Tests (<=5, `bounded_case`)
1. `fixture_text_pointer_arrow`: scalar fixture above: Text `0.75` -> `Proposed(0.75, OutOfSoft=false)`; Text `0.9` -> `Proposed` with `OutOfSoft`; pointer target 1.7 -> clamped 1.0 (hard) and 0.9 crossing soft is allowed; Arrow +1 -> 0.51, Coarse -> 0.60; derived `step/fine/coarse` == 0.01/0.001/0.1. SPEC QUESTION (UNVERIFIED): `fine_step` 0.001 is finer than `precision` 2 - STU-FX-109 calls precision a declared decimal count, not a rounding rule; do NOT round stored values to precision silently - keep the full-precision proposed value, round only for display text, and record the choice in the lane file for spec enrichment.
2. `rejections_preserve_state`: Text `1.25` -> `OutOfHardBounds{1.25,0,1}`; `NaN`, `inf`, `0.75abc`, `12+3`, empty -> typed rejection; stale revision, revoked grant, disabled, Escape -> rejected/discarded; in every case a clone of `ControlState` before/after is equal (0.50, `fixture-R0`).
3. `one_descriptor_three_projections`: `model_json`, `manual_topic`, `access_props` expose identical `control_id`, unit, precision, all four bounds, steps; schema check: the JSON contains `hard_min,hard_max,soft_min,soft_max` as four separate keys (STU-FX-107).
4. `actor_views_isolated`: agent A focuses control X; operator and agent B focus unchanged; reverse direction too; `ControlState` untouched.
5. `headless_egui_frames`: with `egui::Context::default()` run frames with synthetic input (text commit, Escape, drag) through `adapter_egui::show` -> actions -> `reduce`; no window/eframe; accessibility update (if `accesskit` enabled) carries the control's label and value text.

</topic>

<topic id="non-goals" status="active" summary="out of scope + pointers">

## Non-goals / later scope
- No panels/docking/windows/swatch/menu/gizmo/timeline widgets, no theming/tokens, no pointer resistance (STU-SHL-192) beyond hard clamp, no integer/enum domains (step/coarse/fine = 1 rule in STU-FX-110: add next), no Argus/UserManual wiring (descriptor projections only), no document commit (injected actor), no eframe/wgpu/native pane, no per-domain control clones.
- Report READY_FOR_VALIDATION only; never self-PASS.

</topic>
