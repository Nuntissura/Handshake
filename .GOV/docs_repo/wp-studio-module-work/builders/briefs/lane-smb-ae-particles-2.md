---
file_id: LANE-SMB-AE-PARTICLES-2
file_kind: lane_brief
updated_at: 2026-10-31T11:29:59Z
wp: WP-KERNEL-STUDIO
lane: SMB-AE-PARTICLES-2
lane_file: builders/lanes/hsk-studio-ae-particles-2.json
crates: [hsk-studio-composite (simulation types; engine parts only)]
model: opus
effort: high
binding_rules: builders/briefs/wave-parity-2-common.md (HOST INTEGRATION (1)-(7), rules 4a-4e, PUSH SAFETY, RUN LOGS, RED COMMITS, loop cap, lane worktree)
---

<topic id="target" status="active" summary="Four STU-CMP-075 simulation types on the existing particle base: Rain, Snow, Star burst, classic particle systems">

## Target

Engine parts of the queued ledger lane SMB-AE-PARTICLES-2: more STU-CMP-075 simulation types, composite only. Engine-only under D6 (no GUI, no GPU code).

**Spec** (`spec-modules/14-studio-creative-suite.md`): STU-CMP-075 table :29827-29848 (22 capabilities, 14 named); STU-CMP-075a :29850 (deterministic start frame, identical output for identical parameters and seed, scrubbing backwards never differs from playing forwards, no cross-frame parallelism); STU-CMP-075b :29859 and STU-FX-146 :18991 (no recovered parameter record: the record is authored as explicit scope, `studio_declared`; vendor-prefixed names are import keys, never Studio names); STU-FX-011 :19192 (explicit integer seed).

**MTs closed** (index status PENDING, `gui_host` none, `gpu` via_render_provider_port, write path `crates/hsk-studio-composite/**`): MT-34184 Rain, MT-34185 Snow, MT-34189 Star burst, MT-34177 Particle systems (classic). Each MT's AC-001/AC-002 and the headless half of AC-003; the operator-surface tooltip half of AC-003 waits for the GUI (D7).

**Base at 4f69b03b** (lane SMB-AE-PARTICLES-AUDIO-PRESETS, `src/particles.rs`, 949 lines): random-access model :20-28 (a particle's state depends only on its own birth and the force field, so any frame is computed directly); bounds V4-01/02 :44-48; `ParticleError` :81 (codes :102); `Track` :160; `ParticleWorld` :299 (seed :300, start tick :302, `validate` :371); `ParticleLimits` :436; `particles_at` :640; `ParticleFrame` :752 with `content()` :777 -> `StaticContent` drawn by render-cpu and render-gpu through pigment; `render_particles` :912. Outcomes: composite `report` (`src/lib.rs:176`). No simulation identity exists outside `particles.rs` (grep `particle_world|ParticleWorld` over composite and folio src: no other hit).

### Slices, one local commit each, red tests first

1. **Rain** (MT-34184, "Falling-rain generation with wind, speed and depth layering"): authored record (seed, start tick, drop rate, speed, wind direction and strength, depth layers with per-layer scale, speed and opacity, streak length and width, colour, region); anti-aliased streaks; per-drop random access.
2. **Snow** (MT-34185, "the same axes as rain"): shares the precipitation core with slice 1 but is its own identity and record; flakes as dots with seeded drift.
3. **Star burst** (MT-34189, "Radial particle burst used for starfields and impacts"): radial emission from a centre with depth and perspective scaling, speed, seeded star field.
4. **Particle systems (classic)** (MT-34177, "a separate identity, not a preset of the above"): own record and stable identity; a test proves that its identity and record differ from the particle world's and that one never resolves to the other; internal integrator code may be shared.
- **Every slice:**
  - native names and stable ids, no vendor prefix;
  - parameters validated with typed refusals BEFORE allocation (V4-01/02): a `dims_u32_max` and an `aggregate_over_budget` test;
  - STU-CMP-075a tests: same seed gives the same frame digest, a frame rendered after scrubbing back equals the same frame rendered directly, the start tick is deterministic;
  - output through `ParticleFrame` / `StaticContent`: no new render path, no GPU code, no new device (rule 2);
  - terminal outcome through composite `report` with codes only (rule 5).
- Keep the public API of `particles.rs` unchanged (render-cpu `tests/particle_scene.rs` uses it); if a public item must change, add the render-cpu suite to the run (rule 4b).
- After Effects parity of the four types is UNVERIFIED (D5).

### Carried forward (not this lane; the ledger note keeps them)
- folio particle-layer record and per-frame comp hook (folio);
- GPU parity (render-gpu, SMB-R47-FOLLOWUPS-1's crate this wave);
- comp audio from folio records, bezier level keys, preset apply into the effect stack;
- the other STU-CMP-075 rows: MT-34178..MT-34183, MT-34186..MT-34188 (Shatter, Card animation, Fluid, Wave world, Caustics, Foam, Bubbles, Hair, Scatter).

</topic>

<topic id="host-integration" status="active" ingestable="true" summary="host_integration block (schema hsk.studio_mt_host_integration@1); copy into the lane file before READY">

```json
{"schema_id": "hsk.studio_mt_host_integration@1", "direction_ref": "STUDIO-INTEGRATION-REFACTOR-INVENTORY-20261030", "spec_version": "v02.225", "gui_host": "none", "gpu": "via_render_provider_port", "diagnostics": {"internal_diagnostics": "producer_through_observe", "flight_recorder": "none", "palmistry": "none"}, "control_descriptors": "none", "attribution": "required: actor/session, command id, document revision and correlation on every mutating command; correlation grants nothing", "jobs": "none", "settings": "none", "spec_refs": ["STU-CMP-075", "STU-CMP-075a", "STU-CMP-075b", "STU-FX-011", "STU-FX-146", "GUI-DESKTOP-049", "GUI-DESKTOP-050", "STU-ARC-010", "STU-VAL-002", "STU-ARC-015", "GUI-DESKTOP-041", "GUI-DESKTOP-045", "GUI-DESKTOP-025", "GUI-DESKTOP-028", "GUI-DESKTOP-032", "GPU-QUOTA-010", "STU-ARC-002", "STU-ARC-018"], "refactor_exposure": "low", "binding": "binds at dispatch: every commit of this lane follows the HOST INTEGRATION rules of the common brief", "notes": {"gpu": "frames reach render-cpu and render-gpu as StaticContent through the existing pigment route; the simulation and raster are CPU", "attribution": "the simulations are pure frame functions; no mutating command is added"}, "recorded_at": "2026-10-31T11:29:59Z"}
```

</topic>

<topic id="paths" status="active" summary="Owned paths">

## Owned paths (lane worktree `wt-studio-lanes/smb-ae-particles-2`, branch `lane/smb-ae-particles-2`, base 4f69b03b)

- composite: new simulation modules (for example `src/particles/precipitation.rs`, `src/particles/starburst.rs`, `src/particles/classic.rs`, or siblings of `src/particles.rs`), additive shared helpers in `src/particles.rs`, module lines in `src/lib.rs`, new `tests/simulations.rs`.
- No `Cargo.toml` change, no folio, motion, pulse or render crate edit, no root input.

</topic>

<topic id="proof" status="active" summary="Proof and run order">

## Proof

- One bundled run (lane-check semantics, one lock hold): step 0; clippy over composite and its direct dependents (harness-view, harness, interop-fig, pulse, render-cpu, render-gpu by manifest); nextest WHOLE composite suite in both round units, default and `["motion"]` (`.config/round-features.toml:45`); doctests if doc examples are added (4d). Name the log per RUN LOGS.
- Push in any order with SMB-PDF-PRESETS-1 and SMB-ID-BOOKLET-1 (no closure overlap), before SMB-R47-FOLLOWUPS-1 and SMB-SETTINGS-SCHEMA-1. Do NOT start the bundled run until the lead names the disk and the base.
- Set READY_FOR_VALIDATION per slice on a pushed commit. Never claim PASS.

</topic>
