---
file_id: LANE-SMB-AI-DISTORT-EFFECTS-1
file_kind: lane_brief
updated_at: 2026-10-31T18:38:25Z
wp: WP-KERNEL-STUDIO
lane: SMB-AI-DISTORT-EFFECTS-1
lane_file: builders/lanes/hsk-studio-ai-distort-effects-1.json
crates: [hsk-studio-nib (vector live effects: Distort & Transform family on the recovered STU-VEC-155 records; convert-to-shape typed contract)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS with lane-check --per-sha, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Illustrator Distort & Transform live effects (seven STU-FX-028 rows) as pure nib operators over typed STU-VEC-154 parameter maps built from the recovered STU-VEC-155 records">

## Target

Engine-only under D6 (gui_host none, no view code, no GPU). No Illustrator distort effect exists: nib `src/shape_ops.rs` (1037 lines) holds the After Effects shape-layer operators (`pucker_bloat` :748, `zig_zag` :906, `path_noise` :943, `wiggle_path` :971, `twist` :1007; lottie-web rules, module docs :1-46), and `git grep -i "roughen|tweak|free.?distort"` over nib/src finds nothing at 63ecf5a6.

**Spec** (`spec-modules/14-studio-creative-suite.md`, v02.225): STU-FX-027 :19491 (live effects re-evaluate from source, expand only on EFFECT_BAKE); STU-FX-028 Distort & Transform table :19510-19518; STU-VEC-154 :4152 (stable effect id, typed parameter map, version stamp; type vocabulary Int/Real/Bool/String; untyped write refused); **recovered records STU-VEC-155 :4177** (roughen :4179-4196, zig zag :4198-4209, pucker and bloat :4211-4219, twist :4221-4229, tweak :4231-4244, free distort :4246-4251); STU-VEC-158 :4350 (convert-to-shape tokens; numerics NOT recovered); STU-VEC-100 :2969 (seven independent fields; unknown bounds are not invented, not clamped); STU-VEC-101 :3004 (units); STU-FX-011 :19192 (explicit seed); STU-SECTION-003 :118.

**MTs** (gui_host none, gpu none, write `crates/hsk-studio-nib/**`): closes MT-32980 Free Distort, MT-32981 Pucker & Bloat, MT-32982 Roughen, MT-32983 Transform (step-repeat), MT-32984 Tweak, MT-32985 Twist, MT-32986 Zig Zag (each AC-001..AC-004 and the headless half of AC-005; the tooltip half waits for D7). Advances the engine half of MT-30545..30550 (STU-VEC-155), MT-30540 (Distort family row), MT-30535 (registry) and MT-30559 / MT-33001..33003 (convert-to-shape); those are vulkan_shell_client or blocked by a spec gap, so none closes.

**Dispatch gate G1 (lead):** the MT steps ask for the "actual"/"source-defined" law (MT-32982 step 1 and negative case). The spec recovers parameters, not formulas. This brief follows the SMB-AE-WATER-SIMS-1 precedent: each law is a declared Studio reading in the module docs and the descriptor, Illustrator parity UNVERIFIED (D5). LEAD DECISION 2026-10-31T18:46:19Z: G1 confirmed for this lane (precedent SMB-AE-PARTICLES-2 and SMB-AE-WATER-SIMS-1). Exception: where an MT's own acceptance or negative case demands a refusal (e.g. SOURCE_FUNCTION_BINDING_UNAVAILABLE) when the law is not recovered, the MT text wins over the precedent; list each such MT in the lane notes.

### Slices, one local commit each, red tests first

1. **Typed live-effect parameters** (new `src/live_effect.rs`; MT-30535/30545 engine half):
   - effect id `hsk.studio.vector_effect.<snake>` + version stamp + ordered map key -> `Int|Real|Bool|String`; a write of the wrong type or an unknown key is refused typed (STU-VEC-154);
   - seven records copied cell for cell from STU-VEC-155: seven fields per numeric row, `unknown` kept as `None` (never a soft-range clamp; STU-VEC-100); roughen absolute size hard 0..7200 vs slider 0..100 kept apart; tweak's three booleans default true;
   - `roughen_mode` default `relative` recorded as the Studio decision the spec demands (:4190-4192); free distort as a typed quad-to-quad record (`src0..src3`, `dst0..dst3`);
   - Transform (step-repeat) has NO recovered record: its rows are `studio_declared` (move h/v, scale h/v, rotate, reflect x/y, reference point, copies, seed) and say so;
   - convert-to-shape: `target_shape` and `sizing_mode` tokens (default `sizing.absolute`) as typed records; geometry refuses `convert_to_shape_sizing_law_not_recovered` (STU-VEC-158 gap; MT-33001 step 1 forbids guessing relative = additive or multiplicative). A test pins the gap.
2. **Roughen + Zig Zag** (MT-32982, MT-32986): per-segment sampling (roughen `detail` per inch, zig zag ridges per segment), alternating or seeded displacement along the normal, smooth or corner output points; relative size against the source bounds, absolute in document units. Roughen is seeded (STU-FX-011); same seed gives identical output.
3. **Pucker & Bloat + Twist + Tweak** (MT-32981, MT-32985, MT-32984): signed percent (negative puckers); twist rotates more at the centre than the edges; tweak moves anchors, in and out handles by their own switches, relative or absolute, seeded. Reuse `shape_ops` internals only where the rule is identical; keep the public AE operators unchanged.
4. **Free Distort + Transform (step-repeat)** (MT-32980, MT-32983): projective map from the quad correspondence (degenerate or self-crossing quads refused before allocation); ordered copies by composing the affine `copies` times (`affine_mul` :340, `anchored_transform` :362), copies counted against `MAX_OP_ELEMENTS` (:56) before allocation.
- **Every operator:** a pure function of (source paths, typed record) that never mutates its input (re-evaluation from source, STU-FX-027); typed refusals with stable `code()`; non-finite input refused; V4-01/02 (`deny(arithmetic_side_effects)`, a `dims_u32_max`-style and an `aggregate_over_budget` test). Output budgets are engine resource limits, not parameter bounds: record that distinction in the module docs (STU-VEC-100 forbids invented parameter limits).
- Terminal outcomes: codes only through observe, beside `emit_rejection` (`src/proposal.rs:717`); never geometry or text. `descriptor()` (`src/descriptor.rs:2`) gains one `vector_live_effects` entry.

### Carried forward
- Effect-stack records in Folio and EFFECT_BAKE (Folio is in the R50 set); the live re-evaluation hook; Pathfinder paint ops (Outline, Hard Mix, Soft Mix, Trap stay refused, `src/pathfinder.rs:16`); object mosaic; scribble (STU-VEC-159); the control descriptors and Effect menu (D7).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "none", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-FX-027", "STU-FX-028", "STU-VEC-100", "STU-VEC-101", "STU-VEC-154", "STU-VEC-155", "STU-VEC-158", "STU-FX-011", "STU-SECTION-003", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045"], "refactor_exposure": "none", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"gui_host": "MT-30535/30540/30545..30550 are vulkan_shell_client with shared_typed_source descriptors and module_tab_feature_subtab settings; this lane builds only their engine half and registers no command, so their GUI, descriptor and settings halves stay open (D7)", "attribution": "operators are pure functions of (source, record); the stack commit that mutates a document is a later Folio/host step and carries the ActorContext there", "diagnostics": "refusal and outcome codes only; no geometry, names or parameter text reach observe"}, "recorded_at": "2026-10-31T18:38:25Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ai-distort-effects-1`, branch `lane/smb-ai-distort-effects-1`, base 63ecf5a6)

- nib: new `src/live_effect.rs` (a `src/live_effect/` folder is allowed), `pub mod` line in `src/lib.rs` (:13-31), `src/descriptor.rs`, private helpers in `src/shape_ops.rs` only if shared (public items unchanged), new `tests/vector_live_effects.rs` (one target; test fn names carry the MT recipe names `vector_free_distort_frame`, `vector_pucker_bloat_live`, `vector_roughen_live`, `vector_step_repeat_live`, `vector_tweak_live`, `vector_twist_live`, `vector_zig_zag_live`, `shape_mosaic_gap_contract`; record the mapping in the lane notes).
- No `Cargo.toml` change, no root input, no new dependency, no file outside nib.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over nib and its direct dependents by manifest (composite, interop-fig-native, interop-fig, interop-layout, interop-vector-core, interop-vector-svg, interop-vector, render-cpu; harness-view and harness reach nib only through `viewer`, so include `-p hsk-studio-harness-view --features viewer` and `-p hsk-studio-harness --features viewer`, 4e); nextest WHOLE nib suite (default unit; nib has no round feature unit); `cargo test --doc -p hsk-studio-nib` if doc examples are added (4d). Name the log per RUN LOGS. Then `lane-check.sh --owner smb-ai-distort-effects-1 --per-sha origin/codex/handshake-studio-assets` after the rebase; push only on `PER_SHA_RESULT OK`.
- Push order: nib's closure (accord, observe, kurbo, i_overlay) holds no other lane of this wave; any order with SMB-AE-EXPR-LIBRARY-1, SMB-TYPE-OT-FEATURES-1 and SMB-PS-MODE-CONVERT-1. nib sits in the closure of R50's composite (SMB-AE-WATER-SIMS-1): push only when no R50 remediation run with nib in its closure is in flight (lead confirms). Do NOT start the bundled run until the lead names the disk and confirms R50 judged SMB-R49-FOLLOWUPS-1 (accord and observe sit in every closure).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
