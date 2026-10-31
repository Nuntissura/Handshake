---
file_id: LANE-SMB-AE-EXPR-LIBRARY-1
file_kind: lane_brief
updated_at: 2026-10-31T18:38:25Z
wp: WP-KERNEL-STUDIO
lane: SMB-AE-EXPR-LIBRARY-1
lane_file: builders/lanes/hsk-studio-ae-expr-library-1.json
crates: [hsk-studio-motion (expression identifier catalogue, signatures, dual spelling, seeded random reference), hsk-studio-motion-js (After Effects globals prelude bindings)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="STU-MOT-073..076: the 333-identifier catalogue with dual spelling and the 37 recovered signatures in Motion; Vector Math, Other Math, Interpolation, Color Conversion and seeded Randomness bound in the MotionJS prelude; every other identifier a typed unavailable">

## Target

Audit top gap AE-07 (`research/parity-20261023-video-audio.json` per_app.after_effects): "only value, time and linear() are After Effects-compatible". At 63ecf5a6 the whole library is `AE_GLOBALS_PRELUDE` (motion-js `src/provider.rs:40`, `linear` only); Motion has no identifier catalogue. Engine-only under D6.

**Spec** (`spec-modules/14-studio-creative-suite.md`, v02.225): STU-MOT-073 :27629 (333 identifiers, normative target); STU-MOT-073a :27634 (accept the exact declared snake/camel pairs, canonical camelCase on write, never guess aliases by removing underscores) with STU-MOT-144 :29037; **recovered records**: the category table :27646-27657 (12 rows, counts 18+9+6+6+5+4+5+16+9+19+30+206) and the 37 signatures with their DEFAULTS :27659-27701; STU-MOT-074 :27703 (six load-bearing families); STU-MOT-075 :27717 (defaults are contract: `wiggle(freq, amp, octaves = 1, amp_mult = .5, t = time)`); STU-MOT-076 :27728 (deterministic; unseeded `random()` seeds from the property's stable path and the time); STU-MOT-078 :27740; STU-MOT-083 :27778 and STU-MOT-255..259 :29202-29210 (JS profile in MotionJS).

**MTs** (gui_host none, write `crates/hsk-studio-motion/**`): closes MT-33916 Vector Math, MT-33920 Other Math, MT-33919 Color Conversion, MT-33918 Interpolation, MT-33917 Random Numbers, MT-33912 dual spelling, MT-33933 signatures (AC rows; the tooltip half of each row's last AC waits for D7). Advances MT-33911 (333 registered, most unbound), MT-33915 Global and MT-33926 leading block (registered, unbound), MT-35297, MT-35301 Randomness (temporalWiggle open), MT-35302, MT-35303 (vector math half), MT-33934 (seeded randomness only).

**Placement:** MotionJS owns the JS profile (motion `src/expression.rs:1-4`; MT-33912's fulfillment names `hsk-studio-motion-js`), so the bindings go into motion-js although the row MTs list motion only. Record the divergence in the lane notes.

**Dispatch gate G1 (lead):** the row MTs say an identifier without an authoritative signature or semantics returns `SOURCE_FUNCTION_BINDING_UNAVAILABLE` (e.g. MT-33916 step 3). Vector Math, Other Math, HSL and hex conversion are bound by their mathematical definitions; `linear` by the D5 oracle (AE-03). `ease`, `easeIn`, `easeOut` and the random generator have no recovered formula: bind them as `studio_declared` (After Effects parity UNVERIFIED, D5) only if the lead confirms; otherwise they stay typed unavailable and slices 3-4 shrink to their mathematical members. LEAD DECISION 2026-10-31T18:46:19Z: G1 NOT confirmed; the MT text wins (MT-33916 step 3, MT-33917 AC: "Invocation missing authoritative signature/semantics returns SOURCE_FUNCTION_BINDING_UNAVAILABLE, never identity/no-op success"). Bind an identifier only when its semantics are fixed by a mathematical definition or recorded in the spec text (cite the line); `ease`/`easeIn`/`easeOut`, `random`, `gaussRandom`, `noise`, `wiggle` and the generator stay typed SOURCE_FUNCTION_BINDING_UNAVAILABLE unless the spec records their semantics. Slices 3-4 shrink accordingly; the catalogue, spellings, signatures and the typed refusals still close the row MTs whose AC asks for exactly that.

### Slices, one local commit each, red tests first

1. **Motion catalogue** (new `src/expr_catalogue.rs`): every identifier of :27646-27657 verbatim in its category (tests pin 333 and each row count); dual-spelling pairs = both spellings present in one row and equal under the snake/camel rule, nothing else (list them in the lane notes); the 37 signatures verbatim with defaults, attached to an identifier only where the spec names it (:27717-27726: `wiggle`, `temporalWiggle`, `loopOut`, `createPath`); per identifier a typed binding `bound{provenance: oracle | mathematical | studio_declared}` or `unavailable`. Exact lookup; a near spelling is a typed unknown, never an alias.
2. **MotionJS bindings, pure maths** (motion-js): extend the prelude with Vector Math (9) and Other Math (4) plus their declared snake spellings; every catalogue identifier not bound resolves to a typed `Unsupported` (`ExprErrorCode` motion `src/expression.rs:10`) with the user's span, never an identity or no-op. A test walks the catalogue and checks the bound/unavailable split against what the engine answers.
3. **Interpolation + Color Conversion:** `linear` spellings kept; `ease`/`easeIn`/`easeOut` (G1); `rgbToHsl`, `hslToRgb` (4-component, alpha passed through: Studio reading), `hexToRgb` (hostile text, V4-08: three adversarial tests).
4. **Randomness** (G1): Rust reference generator in motion (new `src/expr_random.rs`): a pure function of (`ExprRequest.seed` `src/expression.rs:110`, tick, `seedRandom(seed, timeless = false)` state); `random`, `gaussRandom`, `noise`, `wiggle` with the recovered defaults; the JS mirror is bit-identical to the Rust reference over a shared case table (STU-MOT-076 every backend). `Math.random` stays denied (motion-js `src/quickjs.rs:132`).
- **Constraints:** the prelude stays ONE line (`user_span` `src/provider.rs:57` subtracts exactly one line); user source bounds unchanged (`:455`); existing `quickjs_contract.rs` and `d5_ae_expressions.rs` stay green (the AE-03 linear oracle). Terminal outcomes only through `report_eval` (`src/provider.rs:523`) and motion `report` (`src/receipt.rs:31`), codes only. Motion `DESCRIPTOR` (`src/receipt.rs:57`) gains one `expression_catalogue` key.

### Carried forward
- Property sampling and Looping (MT-35299, MT-35300: need HostRead reads at other ticks), temporalWiggle, Space Transforms, Global time conversions (need composition frame rate), Sub-objects, 3D, the leading block; keyframe operation set STU-MOT-038 (motion, next wave).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "via_render_provider_port", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-MOT-073", "STU-MOT-073a", "STU-MOT-073b", "STU-MOT-074", "STU-MOT-075", "STU-MOT-076", "STU-MOT-078", "STU-MOT-083", "STU-MOT-144", "STU-MOT-255", "STU-MOT-259", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045", "GUI-DESKTOP-025", "GUI-DESKTOP-028", "GUI-DESKTOP-032", "GPU-QUOTA-010", "STU-ARC-002", "STU-ARC-018"], "refactor_exposure": "low", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"jobs": "MT-33912 declares bounded_admission_via_job_port and ring_visible for the provider path; this lane adds no queue or worker: evaluation stays in the existing QuickJS worker under its existing quotas", "attribution": "evaluation is read-only (STU-MOT-076); no mutating command is added", "gpu": "no GPU use; via_render_provider_port is the MT default for Motion"}, "recorded_at": "2026-10-31T18:38:25Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ae-expr-library-1`, branch `lane/smb-ae-expr-library-1`, base 63ecf5a6)

- motion: new `src/expr_catalogue.rs`, `src/expr_random.rs`, `mod`/`pub use` lines in `src/lib.rs`, the `DESCRIPTOR` key in `src/receipt.rs`, new `tests/expr_catalogue.rs`.
- motion-js: `src/provider.rs` (prelude and `with_ae_globals` only; a new `src/prelude.rs` is allowed), `src/descriptor.rs`, new `tests/ae_expression_library.rs` (`#![cfg(feature = "quickjs")]`).
- No `Cargo.toml` change (no feature change, F-R46-8), no root input, no new dependency.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over motion, motion-js and their direct dependents by manifest (composite reaches motion only through `--features motion`, its round unit; motion-js has no dependents), plus `-p hsk-studio-motion-js --features quickjs` and `--features quickjs,test-faults` (`.config/round-features.toml:17`, 4e); nextest WHOLE motion suite (default unit) and WHOLE motion-js suite in the default, `quickjs` and `quickjs,test-faults` units (the new test file is quickjs-gated: 4e); doctests if added (4d). Name the log per RUN LOGS. Then `lane-check.sh --owner smb-ae-expr-library-1 --per-sha origin/codex/handshake-studio-assets`; push only on `PER_SHA_RESULT OK`.
- Push order: motion is in motion-js's closure, both in this lane: push the batch in one push. No other lane of this wave is in the closure (accord, observe, worker); any order with the other three. Motion sits in R50's composite `motion` unit (SMB-AE-WATER-SIMS-1): push only when no R50 remediation run with composite[motion] is in flight. Do NOT start the bundled run until the lead confirms G1, names the disk and confirms R50 judged SMB-R49-FOLLOWUPS-1 (accord, observe).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
