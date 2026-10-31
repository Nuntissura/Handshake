---
file_id: LANE-SMB-AE-WATER-SIMS-1
file_kind: lane_brief
updated_at: 2026-10-31T14:38:14Z
wp: WP-KERNEL-STUDIO
lane: SMB-AE-WATER-SIMS-1
lane_file: builders/lanes/hsk-studio-ae-water-sims-1.json
crates: [hsk-studio-composite (STU-CMP-075 simulations: Wave world, Caustics, Bubbles, Foam)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Four more STU-CMP-075 simulation types on the SMB-AE-PARTICLES-2 base: Wave world (displacement map), Caustics (consumes it), Bubbles, Foam">

## Target

Next After Effects simulation family after SMB-AE-PARTICLES-2, composite only. Engine-only under D6 (no GUI, no GPU code, no new render path).

**Spec** (`spec-modules/14-studio-creative-suite.md`): STU-CMP-075 table :29827-29848 (Wave world :29840, Caustics :29841, Foam :29842, Bubbles :29845); STU-CMP-075a :29850 (deterministic start frame; same parameters and seed give the same output; scrubbing back re-simulates from the start or serves a cached state and never differs from forward play; no cross-frame parallelism); STU-CMP-075b :29859 and STU-FX-146 :18991 (records authored as `studio_declared`); STU-FX-011 (explicit integer seed); import keys :13258-13275 (`APC Wave World`, `APC Caustics`, `APC Foam`, `CC Bubbles`: provenance only, STU-SECTION-003 :118).

**MTs closed** (index status PENDING, `gui_host` none, `gpu` via_render_provider_port, write path `crates/hsk-studio-composite/**`): MT-34181 Wave world, MT-34182 Caustics, MT-34186 Bubbles, MT-34183 Foam. Each MT's AC-001/AC-002 and the headless half of AC-003; the operator-surface tooltip half waits for the GUI (D7).

**Base at e9c46302** (SMB-AE-PARTICLES-2): `src/simulations.rs` (448 lines): module contract :1-36 (stable ids `hsk.studio.simulation.*`, import keys never names, `ParamRow` records, closed-form slot grid, output through `ParticleFrame`); `SimulationKind` :63 (`ALL` :73, `import_keys`, `from_id`, `params`); `ParamRow` :144; `validate_rows` :166; `check_count` :207; `check_raster` :216; `draw` :315; `Simulation` :361; `outcome` :446. Submodules `precipitation.rs`, `star_burst.rs`, `classic.rs`. `src/particles.rs`: `LayerEmitter` :249 (source-raster input precedent); `ParticleLimits` :436; `ParticleFrame` :752, `content()` :777 -> `StaticContent` drawn by render-cpu and render-gpu through pigment. `report` `src/lib.rs:177`. No `SimulationKind` or `simulations::` use outside composite (grep at e9c46302).

### Slices, one local commit each, red tests first

1. **Wave world** (MT-34181, "wave propagation producing a displacement map for other layers to consume"):
   - record: seed, start tick, grid size, wave speed, damping, edge mode (reflect or absorb), point and line producers (position, amplitude, frequency, phase);
   - fixed-step finite-difference height field, simulated from the start tick for every requested frame (STU-CMP-075a re-simulation); fixed dt per tick, declared substeps; the CFL limit checked as a typed refusal;
   - output: a typed `HeightField` (f32, declared range) plus `to_frame()` as a greyscale `ParticleFrame` (0.5 = rest) through the existing `StaticContent` route.
   - The numerical scheme is a declared Studio reading; After Effects parity is UNVERIFIED (D5).
2. **Caustics** (MT-34182, "refracted light through a simulated water surface"):
   - consumes a `HeightField` (slice 1 or any caller field) and a bottom image (premultiplied f32 raster, bounded like `LayerEmitter`);
   - inputs: light direction, refractive index (water 1.33, `studio_declared`), depth, surface opacity;
   - refraction: per-pixel offset from the surface normal (central differences), bilinear sample of the bottom;
   - caustic light: forward splat of refracted rays onto the bottom plane, accumulated in a fixed order so results are deterministic.
3. **Bubbles** (MT-34186, "rising-bubble generation, distinct from foam in that it is not surface-bound"):
   - closed-form per element on the existing slot grid (random access like `precipitation.rs`);
   - rise speed with seeded share, sinusoidal wobble, growth over life, pop at the region top or at end of life;
   - shared per-bubble shading (rim, highlight, fill opacity) declared once, `studio_declared`.
4. **Foam** (MT-34183, "bubble-cluster generation with growth, stickiness and per-bubble rendering"):
   - its own identity and record; a test proves it never resolves to Bubbles;
   - bubbles bound to a producer surface region; stateful stickiness (pairwise attraction within a radius through a uniform-grid neighbour search) is re-simulated from the start tick;
   - bubbles x steps is checked against a budget before allocation; per-bubble rendering reuses slice 3's shading.
- **Every slice:**
  - a `SimulationKind` variant with stable id, native name (the STU-CMP-075 capability name) and import key from the table at :13258-13275;
  - an authored `ParamRow` table that is also the enforced validation (as in `simulations.rs:12-18`);
  - typed refusals BEFORE allocation (V4-01/02), with a `dims_u32_max` and an `aggregate_over_budget` test;
  - STU-CMP-075a tests: same seed gives the same frame digest; frame N rendered after N+k equals frame N rendered directly; the start tick is deterministic;
  - no rayon or threads inside a simulation step (deterministic reduction order); terminal outcome through `outcome` / `report` with codes only.
- Keep existing public items of `particles.rs` and `simulations.rs` unchanged (render-cpu `tests/particle_scene.rs` uses particles); new public items only.

### Carried forward (not this lane)
- A general displacement-map effect that consumes the height field on any layer (an STU-FX row, effect stack); folio simulation-layer records and the per-frame comp hook; GPU parity.
- Remaining STU-CMP-075 rows: Shatter MT-34178, Card animation MT-34179, Fluid simulation MT-34180, Hair MT-34187, Scatter MT-34188.

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "via_render_provider_port", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-CMP-075", "STU-CMP-075a", "STU-CMP-075b", "STU-FX-011", "STU-FX-146", "STU-SECTION-003", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045", "GUI-DESKTOP-025", "GUI-DESKTOP-028", "GUI-DESKTOP-032", "GPU-QUOTA-010", "STU-ARC-002", "STU-ARC-018"], "refactor_exposure": "low", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"gpu": "frames reach render-cpu and render-gpu as StaticContent through the existing pigment route; simulation and raster are CPU", "attribution": "the simulations are pure frame functions; no mutating command is added", "jobs": "a frame is one bounded call; re-simulation from the start tick is bounded by the declared step budget, no queue or worker is added"}, "recorded_at": "2026-10-31T14:38:14Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ae-water-sims-1`, branch `lane/smb-ae-water-sims-1`, base e9c46302)

- composite: `src/simulations.rs` (variants, re-exports, shared helpers only), new `src/simulations/wave_world.rs`, `caustics.rs`, `bubbles.rs`, `foam.rs`, `DESCRIPTOR` entries in `src/lib.rs`, new `tests/simulations_water.rs`.
- No folio, pigment, render-cpu or render-gpu edit, no `Cargo.toml` change (no feature change: F-R46-8), no root input, no new dependency.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold) on the disk the lead names: step 0; clippy over composite and its direct dependents (harness-view, harness, interop-fig, pulse, render-cpu, render-gpu by manifest); nextest WHOLE composite suite in BOTH round units, default and `--features motion` (`.config/round-features.toml`, 4e); `cargo test --doc -p hsk-studio-composite` if doc examples are added (4d). Name the log per RUN LOGS.
- Push order: composite's closure (folio, prism, pigment, nib, type, motion) holds SMB-FOLIO-CHANNEL-RECORD's folio and pigment, so this lane pushes BEFORE that lane; any order with SMB-PDF-STOCK-PRESETS-1 and SMB-ID-PDF-EXPORT-PREFS-1. Do NOT start the bundled run until the lead names the disk and confirms R49 judged SMB-AE-PARTICLES-2 (a FAIL there is remediated first on that lane).
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
