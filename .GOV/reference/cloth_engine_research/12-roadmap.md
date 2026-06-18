---
file_id: cloth-engine-roadmap
topic_id: T-ROADMAP
title: "Kernel-Gated Build Roadmap"
status: draft
depends_on:
  - T-CLOTH-SOLVER
  - T-COLLISION
  - T-KERNEL-INTEGRATION
summary: "Phased, kernel-gated build plan for the Handshake Tailor creative module: slice 1 is a GPU substrate proof on a sphere, slice 2 is the real production go/no-go on exaggerated-proportion capsules with full collision and the model-first API, then garment authoring, kernel authority integration, and export pipeline."
sources: 22
updated_at: "2026-06-17"
---

## [T-ROADMAP] Kernel-Gated Build Roadmap

This document defines the phased, kernel-gated build plan for the Handshake Tailor creative module. The engine is Marvelous Designer-equivalent cloth/garment simulation built entirely inside Handshake — no side applications, no external simulation servers. It attaches to kernel primitives (EventLedger, Postgres authority, CRDT, sandbox, promotion gate, model lanes) and delivers model-first (LLM-steerable) garment authoring as its core differentiator.

The roadmap is organized as six slices. Each slice has a clear gate criterion that blocks forward progress. Slices 1–2 are the substrate and go/no-go probes. Slices 3–6 are the feature build. Kernel primitive dependencies are called out per slice.

---

### [T-ROADMAP.overview] Roadmap Overview and Gate Philosophy

The overall gate philosophy matches the Handshake kernel build pattern: a model-written or model-proposed artefact enters the sandbox, runs through a validation gate, and is promoted to authority only after the gate passes. The Tailor roadmap applies the same discipline to the build itself — each slice produces a testable, inspectable artefact, and the next slice begins only when the gate criterion is met.

**Dependency chain:**

```text
Slice 1 (wgpu substrate, sphere patch)
  -> Slice 2 (GO/NO-GO: cloth + jiggle on capsules, self-collision, model API)
    -> Slice 3 (2D pattern authoring, seam/sewing constraints, garment authoring via model lanes)
      -> Slice 4 (fabric property system, anisotropic materials, keyframeable params)
        -> Slice 5 (kernel authority integration: Postgres, EventLedger, CRDT, promotion gate)
          -> Slice 6 (export pipeline, UV-from-pattern, game-engine rigging, EveryWear-equivalent)
```

**Kernel primitives consumed per slice:**

| Slice | Kernel Primitives Needed |
|-------|--------------------------|
| 1 | None (standalone crate only) |
| 2 | ModelAdapter trait (LLM steering API) |
| 3 | ModelAdapter, SandboxAdapter, LlmClient |
| 4 | ModelAdapter, SandboxAdapter, ValidatorRunner |
| 5 | Postgres/EventLedger, CRDT, PromotionGate, no_sqlite_tripwire |
| 6 | ArtifactManifest, PromotionReceipt, Tauri IPC commands |

**Crate split (established before Slice 1):**

```text
tailor-solver/   (standalone Rust crate, workspace member)
  src/
    solver.rs             (ClothSolver trait, ClothMesh, SimParams)
    constraints/          (stretch, bend, seam, volume, collision, tack)
    gpu/                  (wgpu device, pipelines, buffer management)
    shaders/              (WGSL compute shaders)
    cpu_fallback.rs       (softy-backed CPU path for CI and headless)
  Cargo.toml              (wgpu, parry3d, glam, bytemuck; NO sqlx, NO tauri)

handshake_core/src/tailor/  (creative module, kernel-integrated)
  mod.rs
  garment.rs              (GarmentDraft, GarmentAsset, domain types)
  solver_binding.rs       (TailorSandboxAdapter, wraps solver crate)
  simulation.rs           (SimRun orchestration, substep scheduling)
  material.rs             (FabricSpec, MaterialLibrary)
  seam.rs                 (SeamGraph, SeamConstraintBuffer)
  pattern.rs              (PanelGeometry, PatternAuthority)
  event_family.rs         (event family constants)
  storage_glue.rs         (Postgres queries for tailor authority tables)
  api.rs                  (axum Router, tailor endpoints)
```

This split keeps wgpu/WGSL outside `handshake_core` (which has no GPU code today), and keeps sqlx/Tauri outside the solver crate (per the UI-agnostic prior decision). The solver crate is consumed via the `ClothSolver` trait boundary, so swapping the GPU backend or adding a CPU fallback path is a single impl change.

---

### [T-ROADMAP.slice1] Slice 1: GPU Substrate — XPBD Patch on a Sphere

**Goal.** Prove that the XPBD solver crate runs on the developer machine's GPU via wgpu/WGSL, produces physically plausible drape, and that the Cargo workspace plumbing is correct. This is pure infrastructure; no kernel integration, no garment authoring, no seams.

**Marvelous Designer mapping.** None directly — this is the equivalent of proving that MD's solver can run at all before exposing any authoring UI.

**OSS reference.** The `ccincotti3/webgpu_cloth_simulator` and `jspdown/cloth` WebGPU XPBD implementations both start from this exact point: a flat grid of particles, distance + bending constraints, gravity, sphere collision. The WGSL constraint-coloring pattern from `ccincotti3` is the direct template. The `vitalight/Velvet` CUDA engine establishes the Jacobi-with-delta-accumulation parallelization strategy that translates to WGSL.

**Deliverable.** A standalone Bevy testbed binary (inside `tailor-solver/examples/sphere_drape.rs`) that renders a flat cloth grid draping over a static sphere, simulated 100% on GPU via WGSL compute shaders. Bevy is a throwaway viewport here — it does NOT enter the solver crate.

**Solver pipeline for this slice:**

```rust
// Per-frame GPU dispatch (simplified):
// 1. Upload particle positions + velocities to storage buffers
// 2. for substep in 0..n_substeps:
//      a. dispatch predict_positions compute shader (Verlet integrate + gravity)
//      b. for color in constraint_colors:
//           dispatch solve_stretch compute shader (distance constraints, this color group)
//           dispatch solve_bend compute shader (dihedral bending, this color group)
//      c. dispatch apply_sphere_collision compute shader
//      d. dispatch update_velocities compute shader
// 3. Readback or render directly from GPU buffer via vertex-pull

// WGSL workgroup pattern (64 threads, 1D physics):
// @compute @workgroup_size(64, 1, 1)
// fn predict(@builtin(global_invocation_id) id: vec3<u32>) { ... }
```

**Constraint types in Slice 1:**
- Stretch (isotropic, edge-length distance constraints)
- Dihedral bending (angle between adjacent triangles)
- Sphere SDF collision (single static sphere, capsule proxy optional)
- Gravity (uniform body force)

**Constraint graph coloring.** Constraints are partitioned into independent color groups (particles sharing a constraint cannot be in the same color group). Same-color constraints are solved in parallel on GPU with no write conflicts. This is the Gauss-Seidel parallelization strategy from `ccincotti3/webgpu_cloth_simulator`.

**wgpu pipeline structure:**

```rust
// One ComputePipeline per shader pass; pipelines reused across substeps
// Storage buffers: positions (read-write), velocities (read-write),
//   rest_lengths (read-only), constraint_indices (read-only),
//   constraint_colors (read-only), sim_params (uniform)
// Bind groups match @group(0) @binding(N) in WGSL
// Iterative substep: queue.submit() between substep sets only when readback needed;
//   within a single frame, chain dispatches inside one CommandEncoder
```

**Gate criterion — Slice 1:**
> The sphere_drape example binary runs without panic on the developer GPU (Vulkan/DX12), cloth drapes over the sphere under gravity with visible wrinkles, simulation is visually stable for 300+ frames at 60 fps target, and `cargo test` for the solver crate passes in headless mode using the CPU fallback path (`softy` or `xpbdrs` backend gated by `#[cfg(not(feature = "gpu"))]`).

**CI note.** GPU is unavailable in most CI runners. The `cpu_fallback.rs` path (using `softy` or pure-Rust constraint solve) runs in CI. The GPU path is developer-machine only until a GPU CI runner is provisioned.

**What is explicitly NOT in Slice 1:**
- No seam constraints
- No self-collision
- No anisotropic material
- No kernel integration
- No model API
- No Bevy dependency inside the solver crate (only in the `examples/` binary)

---

### [T-ROADMAP.slice2] Slice 2: GO/NO-GO Gate — Cloth and Jiggle on Exaggerated-Proportion Capsules

**Goal.** This is the critical production go/no-go for the engine. It must demonstrate that the solver handles the two hardest cases for the Handshake avatar production target: (a) cloth on capsule-proxy bodies with exaggerated proportions (large bust, wide hip, non-standard shoulder-to-waist ratio), and (b) cloth self-collision and multi-layer cloth-on-cloth collision. It also introduces the model-first API surface so the LLM can steer simulation parameters.

**Why this is the real go/no-go.** MD's GPU simulation reached CPU-accuracy parity for cloth-avatar collision in 2024.2, and added GPU trim simulation in 2026.0. Getting collision quality right on exaggerated-proportion capsules (not just academic spheres) is the hardest engineering problem in the engine. If the solver cannot pass this gate within acceptable quality and performance, the entire creative module is not viable.

**Marvelous Designer mapping:**
- Avatar system (body proxy capsules, non-standard proportions) → MD Group 5
- Self-collision → MD Group 4 (particle self-collision, sphere-based mode)
- Secondary motion / jiggle → soft-body volume/pressure constraints applied to bust capsule deformable zones
- Multi-layer cloth → MD Group 4 (cloth-on-cloth collision for layered garments)

**OSS reference.**
- `vitalight/Velvet`: spatial hashing with CUB radix sort (~50% faster than Thrust) for neighbor finding; neighbor caching across substep iterations — this is the WGSL spatial hash reference.
- `ccincotti3/webgpu_cloth_simulator`: self-collision via spatial hash, substep velocity clamping, friction application — five-step XPBD self-collision protocol.
- Carmen Cincotti's blog post on cloth self-collisions: spatial hash table foundation, parameter configuration (collision distance = min(2r, rest_length)), substep integration, velocity clamping (v_max = r × n_substeps / Δt), friction application.
- PSCC (Parallel Self-Collision Culling with Spatial Hashing on GPUs, ACM CGIT 2018): normal cone culling + spatial hashing GPU-parallel BVH traversal — implementation reference for the WGSL self-collision pass.
- XRTailor (OpenXRLab): dual-mode architecture (Swift/Quality), multi-color Gauss-Seidel constraint coloring, Chebyshev acceleration for convergence speedup; architecture reference for the quality-vs-speed tradeoff.
- Bolt (NVIDIA, April 2025): three-stage transfer/drape/rig pipeline; fast XPBD draping with collision resolution and drape untangling for garments on diverse body proportions — validates that XPBD can handle diverse body shapes at scale.

**New constraint types in Slice 2:**

```rust
// Constraint additions over Slice 1:
// - CapsuleCollisionConstraint: cloth particle vs. capsule body segment
//   (parry3d::shape::Capsule for shape queries, SDF-based push-out)
// - SelfCollisionConstraint: cloth particle vs. cloth particle
//   (GPU spatial hash, velocity clamping, friction)
// - VolumeConstraint (pressure): soft-body inflation for deformable bust zone
//   (XPBD volume constraint from Matthias Müller "Unbreakable Soft Bodies" paper)
// - MultiLayerCollisionConstraint: inner garment vs. outer garment particle pairs
//   (same spatial hash, separate layer masks)
```

**Body proxy setup:**

```rust
// Avatar representation for cloth simulation purposes:
// - Rigid capsule segments for limbs, torso, neck (parry3d Capsule shapes)
// - Deformable soft-body zones for bust (volume/pressure XPBD soft body)
//   using tetrahedral mesh inside a coarse tet cage on a surface capsule
// - Capsule radii and proportions set from GarmentSpec body_measurements field
// - No Rapier rigid body simulation — capsule positions come from skeleton pose,
//   not from a physics sim; Rapier is used only for shape queries (parry3d)

pub struct AvatarProxy {
    pub segments: Vec<CapsuleSegment>,   // parry3d capsules, pose-driven
    pub soft_zones: Vec<SoftBodyZone>,  // XPBD tet mesh for secondary motion
}

pub struct CapsuleSegment {
    pub shape: parry3d::shape::Capsule,
    pub transform: glam::Mat4,
}

pub struct SoftBodyZone {
    pub tet_mesh: TetMesh,               // coarse tet cage
    pub skin_particles: Vec<usize>,      // cloth particles in contact zone
    pub pressure_compliance: f32,        // XPBD volume compliance
}
```

**Model-first API (introduced in Slice 2):**

The LLM-steerable surface uses the `ModelAdapter` trait from `kernel/model_adapter.rs`. A `TailorSimParamAdapter` implements `ModelAdapter` and exposes simulation parameters as JSON tool arguments that an LLM can propose. The adapter runs inside the kernel's sandbox — parameter proposals are validated before being applied to the solver.

```rust
// TailorSimParamAdapter implements ModelAdapter
// ContextBundle carries: current GarmentDraftV1 JSON,
//   current SimParamsV1 JSON, quality target description (string),
//   prior sim run results (visual_score, error_metrics)
// artifact_payload output: SimParamsDeltaV1 (partial update to SimParamsV1)
// Validation: SimParamsDeltaV1 must pass SimParamsValidator before apply

#[derive(Serialize, Deserialize)]
pub struct SimParamsV1 {
    pub particle_distance_mm: f32,      // mesh resolution (0.8–700mm)
    pub n_substeps: u32,                // substep count per frame
    pub n_iterations: u32,              // constraint iterations per substep
    pub gravity: glam::Vec3,
    pub wind: WindSpec,
    pub self_collision: bool,
    pub self_collision_distance_mm: f32,
    pub friction_coefficient: f32,
    pub sim_mode: SimMode,              // Swift (4 substep/10 iter) or Quality (1/200)
}

#[derive(Serialize, Deserialize)]
pub enum SimMode { Swift, Quality }
```

**Gate criterion — Slice 2 (the real go/no-go):**
> A cloth grid draped over the exaggerated-proportion capsule avatar (large bust, wide hip body proportions) must: (1) show no permanent penetration into body segments at rest, (2) show visible secondary motion / jiggle of the soft bust zone under rapid avatar motion, (3) show correct multi-layer cloth separation when two cloth layers are present, (4) run in Swift mode at ≥ 30 fps for a 2000-particle cloth at 1080p-equivalent preview resolution on a mid-range GPU (RTX 3060 equivalent), and (5) an LLM-proposed `SimParamsDeltaV1` JSON (passed through `TailorSimParamAdapter`) must be accepted by the sandbox, validated, and correctly applied to a subsequent simulation run, with an EventLedger receipt emitted for the parameter change.
>
> **If this gate fails**, the Tailor engine is NOT viable and the creative module must be redesigned before proceeding to Slice 3.

**What is NOT in Slice 2:**
- No seam/sewing constraints (garments are still flat grids)
- No 2D pattern panels
- No full kernel authority tables (Postgres tables created in Slice 5)
- No CRDT collaborative editing
- No export pipeline

---

### [T-ROADMAP.slice3] Slice 3: Garment Authoring — 2D Pattern Panels, Seam Constraints, Model-First Pattern API

**Goal.** Introduce the garment representation: 2D panel geometry, seam/sewing constraint graph, and the LLM-steerable garment authoring pipeline (ChatGarment-style JSON-to-pattern generation through the model lane).

**Marvelous Designer mapping:**
- 2D pattern authoring (Group 1): rectangles, polygons, internal lines, fold seams, darts
- Sewing system (Group 2): segment sewing, 1:N ratio sewing (gather), fold seam lines
- Garment library (Group 9): save/load draft garments as typed JSON authority rows

**OSS reference.**
- `GarmentCode` (ETH Zurich, MIT): parametric sewing pattern DSL; core types `Edge`, `Panel`, `Component`, `Interface`; JSON panel+edge schema is the direct model for `GarmentPanelV1` and `SeamGraphV1` authority types.
- `ChatGarment` (CVPR 2025, MPI): fine-tunes LLaVA VLM to output GarmentCode JSON; three modes (estimate-from-image, generate-from-text, edit-via-dialogue); simplified GarmentCode token count from 900 to 350 for fine-tuning — establishes the prompt engineering patterns for Handshake's `TailorGarmentAuthorModelAdapter`.
- `GarmentDiffusion` (IJCAI 2025): edge-oriented token representations for diffusion-based pattern generation; 10x shorter sequence than SewingGPT; state-of-the-art on DressCodeData — architecture reference for the pattern token vocabulary.
- `NGL-Prompter` (MPI, Feb 2026): training-free pipeline; Natural Garment Language restructuring of GarmentCode for VLM legibility; queries VLMs to extract garment parameters and maps to valid GarmentCode — validates that a Handshake-native prompt template can drive LLM pattern authoring without fine-tuning.
- `Design2GarmentCode` (CVPR 2025, Style3D): LMM generates GarmentCode pattern-making programs from images/text/sketches — confirms multi-modal input (image + text) can feed the garment authoring model lane.

**Authority types:**

```rust
// GarmentDraftV1: the pre-simulation pattern representation
// Stored as JSONB in tailor_garments.draft_json (Postgres)
// Authority row written only after sandbox validation pass

#[derive(Serialize, Deserialize)]
pub struct GarmentDraftV1 {
    pub garment_id: Uuid,
    pub workspace_id: String,
    pub schema_version: &'static str,  // "garment_draft_v1"
    pub panels: Vec<GarmentPanelV1>,
    pub seam_graph: SeamGraphV1,
    pub body_measurements: BodyMeasurementsV1,
    pub garment_type: GarmentType,     // Top, Skirt, Pants, Dress, ...
    pub fit_properties: FitPropertiesV1,
    pub material_spec: FabricSpecV1,   // preliminary; refined in Slice 4
}

#[derive(Serialize, Deserialize)]
pub struct GarmentPanelV1 {
    pub panel_id: Uuid,
    pub label: String,                 // "front_bodice", "back_bodice", ...
    pub boundary: Vec<ControlPoint>,   // 2D control points (mm units)
    pub internal_lines: Vec<InternalLine>, // darts, fold lines, stress seams
    pub uv_origin: glam::Vec2,         // lower-left corner in UV space
}

#[derive(Serialize, Deserialize)]
pub struct SeamGraphV1 {
    pub seams: Vec<SeamEdgeV1>,
}

#[derive(Serialize, Deserialize)]
pub struct SeamEdgeV1 {
    pub seam_id: Uuid,
    pub panel_a: Uuid,
    pub edge_a: EdgeRef,
    pub panel_b: Uuid,
    pub edge_b: EdgeRef,
    pub ratio_m: u32,                  // M in M:N gather ratio (1 for 1:1)
    pub ratio_n: u32,                  // N in M:N gather ratio
    pub seam_kind: SeamKind,           // Join, Fold, Tack
}
```

**Seam constraint translation to solver:**

```rust
// SeamGraph -> SeamConstraintBuffer (GPU upload)
// Each SeamEdgeV1 maps to a set of distance constraints between
// paired particle indices on the two seam edges.
// For M:N ratio sewing: rest_length for the N-side particles is scaled
// by M/N relative to the actual edge length, producing the gather effect.
// SeamConstraintBuffer is a flat array uploaded to a read-only storage buffer
// before each simulation run.

pub struct SeamConstraint {
    pub particle_a: u32,
    pub particle_b: u32,
    pub rest_length: f32,           // 0.0 = full join; scaled for gather
    pub compliance: f32,
}
```

**LLM garment authoring pipeline (model lane integration):**

```rust
// TailorGarmentAuthorModelAdapter implements ModelAdapter
// Input: ContextBundle with body_measurements JSON + text/image description
// Output: artifact_payload = GarmentDraftV1 JSON
// Flow: LlmClient::completion() -> parse GarmentCode-compatible JSON
//       -> decode to GarmentDraftV1 -> sandbox validation -> promotion gate

// Model lane routing: uses kernel LlmClient trait (HSK-TRAIT-004)
// ollama local model (llama3 or similar) is the default path
// BYOK path via ModelRuntime registry for remote models

// Prompt template pattern (NGL-Prompter / ChatGarment reference):
// system: "You are a garment pattern authoring assistant. Output only valid
//          GarmentDraftV1 JSON matching the schema..."
// user: "<body_measurements_json>\n<description_or_image_tokens>\n
//         Generate a {garment_type} garment pattern."
```

**Gate criterion — Slice 3:**
> An LLM-authored `GarmentDraftV1` JSON (produced by `TailorGarmentAuthorModelAdapter` via the kernel model lane) must be decodable to a valid seam-connected panel mesh, simulate on the Slice 2 capsule avatar without catastrophic panel intersection at rest, and produce a visually recognizable garment silhouette. Seam constraint ratio (1:N gather) must produce visible gather when ratio > 1. An EventLedger receipt of kind `TailorGarmentDraftProposed` must be emitted for each LLM-authored draft entering the sandbox.

**What is NOT in Slice 3:**
- No anisotropic fabric parameters (fabric uses isotropic defaults)
- No Postgres authority tables yet (garment JSON is held in sandbox memory)
- No CRDT collaborative editing
- No UV or export pipeline

---

### [T-ROADMAP.slice4] Slice 4: Fabric Property System — Anisotropic Materials, Keyframeable Parameters

**Goal.** Introduce the full physical fabric model: anisotropic weft/warp/shear stretch, anisotropic bending, density, buckling, and keyframeable per-step parameter upload. This is the "moat-3" and "moat-4" capability from the MD feature map.

**Marvelous Designer mapping:**
- Fabric physical properties (Group 3): Stretch-Weft/Warp/Shear, Bending-Weft/Warp, Buckling Ratio/Stiffness, Density, Friction, Pressure, Solidify, Shrinkage Weft/Warp
- Keyframeable fabric properties (Group 6, 2025.2 feature): time-varying stiffness, pressure, shrinkage during simulation

**OSS reference.**
- XRTailor Quality Mode: FEM-based anisotropic stiffness with 1 substep / 200 iterations — confirms that high-iteration anisotropic solve is the quality path.
- XRTailor Swift Mode: compliance-based material model with 4 substeps / 10 iterations — the real-time preview path.
- Macklin et al. XPBD 2016: compliance (α = 1/stiffness) as XPBD parameter; separate compliance per constraint axis enables anisotropic behavior without changing constraint structure.
- MGPBD (SIGGRAPH 2025): multigrid-accelerated global XPBD solver via Unsmoothed Aggregation AMG + PCG; addresses XPBD stalling at high resolution / high stiffness — the convergence fix to be evaluated in Phase 4 quality testing.

**Anisotropic fabric in WGSL:**

```wgsl
// FabricParams uniform buffer — one entry per panel, indexed by particle's panel_id
struct FabricParams {
    stretch_weft: f32,    // compliance for weft (horizontal) stretch constraints
    stretch_warp: f32,    // compliance for warp (vertical) stretch constraints
    stretch_shear: f32,   // compliance for diagonal shear constraints
    bend_weft: f32,       // compliance for weft bending
    bend_warp: f32,       // compliance for warp bending
    buckling_ratio: f32,  // bending compliance ratio at buckle corners
    density: f32,         // g/m^2 -> particle mass (density * triangle_area / 3)
    friction: f32,        // cloth-avatar friction coefficient
    pressure: f32,        // current pressure (0 = no inflation)
    shrinkage_weft: f32,  // current shrinkage scaling (1.0 = no shrinkage)
    shrinkage_warp: f32,
    _pad: vec2<f32>,
}

// Weft/warp axis is the grain direction stored per-panel in UV space
// A constraint knows its axis angle from the UV parameterization of the mesh
// compliance = lerp(stretch_weft, stretch_warp, cos^2(theta_from_weft_axis))
// This gives per-constraint anisotropic compliance without separate buffers
```

**Keyframeable parameters:**

```rust
// KeyframeTrack: typed time-series for each animatable param
// Per-substep: upload current param values to FabricParams uniform buffer
// Interpolation: lerp between keyframe values at current sim_time

pub struct FabricKeyframeTrack {
    pub param: AnimatableFabricParam,
    pub keyframes: Vec<(f32, f32)>,    // (time_seconds, value)
}

pub enum AnimatableFabricParam {
    Pressure,
    Solidify,
    ShrinkageWeft,
    ShrinkageWarp,
    TackStrength,          // for keyframed stitch/unstitch effects
}
```

**Model-first parameter API for fabric:**

```rust
// TailorFabricParamModelAdapter implements ModelAdapter
// Input: ContextBundle with garment description + target aesthetic
//   ("stiff denim", "flowing silk", "inflatable vinyl", ...)
// Output: artifact_payload = FabricSpecV1 JSON
// FabricSpecV1 maps to FabricParams struct fields
// Estimation method: rule-based parameter mapping (NGL-Prompter pattern)
//   + LLM refinement from text description + visual feedback loop

// Preset library: pre-defined FabricSpecV1 JSON blobs for cotton/denim/silk/jersey/leather
// stored in tailor_material_library Postgres table (Slice 5)
```

**Gate criterion — Slice 4:**
> Denim-preset simulation must show significantly stiffer drape and fewer wrinkles than silk-preset simulation on the same panel geometry. Pressure-inflated garment (vinyl preset) must show visible inflation under increasing pressure keyframe. LLM-proposed `FabricSpecV1` JSON (from text description "stiff denim jacket with mild weft bending") must produce fabric parameters in the denim range without operator override. CPU fallback path must produce the same qualitative anisotropic behavior as the GPU path (confirmed by side-by-side visual inspection).

**What is NOT in Slice 4:**
- No Postgres authority writes (still sandbox-only)
- No CRDT
- No export pipeline

---

### [T-ROADMAP.slice5] Slice 5: Kernel Authority Integration — Postgres, EventLedger, CRDT, Promotion Gate

**Goal.** Wire the Tailor engine to all five Handshake kernel primitives: PostgreSQL authority tables (migrations), EventLedger receipts (typed `KernelEventType` variants), CRDT collaborative garment editing, sandbox/validation/promotion gate lifecycle, and `no_sqlite_tripwire`. This slice makes garment assets durable, auditable, and collaboratively editable.

**Handshake codebase dependency.** This slice directly follows the module pattern of `src/atelier/` and `storage/kb003_storage.rs`. All patterns (PgPool queries, EventLedger insert via `NewKernelEvent::builder`, CRDT via `CrdtUpdateRecordV1`, promotion gate via `PromotionGate.evaluate()`) are copied from those existing surfaces.

**Postgres migrations (new tables):**

```sql
-- Migration: tailor_001_garments.sql
CREATE TABLE tailor_garments (
    garment_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id      TEXT NOT NULL,
    schema_version    TEXT NOT NULL DEFAULT 'garment_draft_v1',
    status            TEXT NOT NULL DEFAULT 'draft',
    draft_json        JSONB NOT NULL,
    event_ledger_id   TEXT REFERENCES kernel_event_ledger(event_id),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Migration: tailor_002_simulation_runs.sql
CREATE TABLE tailor_simulation_runs (
    sim_run_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id        UUID REFERENCES tailor_garments(garment_id),
    sandbox_run_id    UUID REFERENCES kb003_sandbox_runs(sandbox_run_id),
    status            TEXT NOT NULL,    -- REQUESTED, STARTED, COMPLETED, REJECTED
    params_json       JSONB NOT NULL,
    result_artifact   UUID,             -- FK to artifact store after completion
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Migration: tailor_003_material_library.sql
CREATE TABLE tailor_material_library (
    material_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id      TEXT NOT NULL,
    label             TEXT NOT NULL,   -- "denim", "silk", "vinyl", ...
    fabric_spec_json  JSONB NOT NULL,
    is_preset         BOOLEAN NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**EventLedger — new KernelEventType variants:**

```rust
// Added to KernelEventType enum in kernel/mod.rs,
// registered in required_first_slice_events():
TailorGarmentDraftProposed,       // LLM proposes a new GarmentDraftV1
TailorGarmentDraftValidated,      // Validation gate passes on a draft
TailorGarmentDraftRejected,       // Validation gate rejects a draft
TailorSimRunStarted,              // Sandbox XPBD sim run begins
TailorSimRunCompleted,            // Sim run produces artifact bundle
TailorSimRunRejected,             // Sim run fails (solver error, timeout)
TailorGarmentPromoted,            // Garment passes promotion gate -> authority row
TailorGarmentCrdtUpdateRecorded,  // Collaborative panel edit applied
```

**EventLedger emit pattern (following existing NewKernelEvent::builder):**

```rust
let event = NewKernelEvent::builder(
    task_run_id,
    session_run_id,
    KernelEventType::TailorGarmentDraftProposed,
    KernelActor::ModelAdapter(adapter_id.to_string()),
)
.aggregate("tailor_garment", garment_id.to_string())
.idempotency_key(format!("tailor_garment_draft_proposed:{garment_id}:{draft_hash}"))
.payload(json!({
    "garment_id": garment_id,
    "garment_type": garment_draft.garment_type,
    "panel_count": garment_draft.panels.len(),
    "schema_version": "garment_draft_v1",
}))
.source_component("tailor::garment_author")
.build();
postgres.insert_event_ledger(&event).await?;
```

**Sandbox/promotion gate lifecycle for model-authored garments:**

```text
1. LLM proposes GarmentDraftV1 via TailorGarmentAuthorModelAdapter
   -> KernelEventType::TailorGarmentDraftProposed emitted
   -> Draft written to tailor_garments (status='draft')

2. TailorSandboxAdapter (implements SandboxAdapter) executes inside sandbox:
   - SandboxPolicyV1: no network, no FS writes outside artifact staging dir
   - Calls tailor-solver::ClothSolver::run_sim(draft, params)
   - Returns SandboxArtifactBundle (vertex buffer, UV map, material JSON)
   -> KernelEventType::TailorSimRunStarted / TailorSimRunCompleted emitted

3. ValidationRunner checks:
   - mesh_topology_valid: no degenerate triangles, no isolated vertices
   - seams_closed: all seam edges have a matching pair
   - no_catastrophic_penetration: max penetration depth < 2x collision_thickness
   - uv_coverage: UV islands cover all panels without overlap
   -> KernelEventType::TailorGarmentDraftValidated or TailorGarmentDraftRejected

4. PromotionGate.evaluate():
   - SandboxRunV1 + ValidationReport + OperatorApprovalEvidence
   - Returns PromotionDecisionV1 (Accepted / Rejected)
   -> KernelEventType::TailorGarmentPromoted on accept
   -> tailor_garments.status updated to 'promoted'
   -> PromotionReceiptV1 written to kb003_promotion_receipts
```

**CRDT collaborative panel editing:**

```rust
// Garment panel geometry maps onto the CRDT document layer
// garment_id -> crdt_document_id (one-to-one)
// Each panel edit (move control point, add internal line, change seam ratio)
// is a CrdtUpdateRecordV1 row in kernel_crdt_updates
// yjs_bridge encodes the delta; KnowledgeStateVectorV1 tracks causality
// Concurrent edits from two operators are merged via last-write-wins
// on non-conflicting control points; conflicting seam edits surface
// as CRDT conflict markers for operator resolution

// tailor CRDT document type: "garment_panel_geometry"
// actor_site: one per connected Tauri client (operator or model)
// state_vector prefix: "hsk-tailor-sv1:"
```

**no_sqlite_tripwire integration:**

```rust
// storage_glue.rs, before any authority write:
guard_authority_write(&pool)?;  // panics if non-Postgres storage detected
```

**Gate criterion — Slice 5:**
> A full end-to-end flow must produce: (a) a `TailorGarmentDraftProposed` EventLedger receipt, (b) a completed `tailor_simulation_runs` row referencing a valid sandbox run, (c) a `ValidationRunner` report with all four checks passing, (d) a `TailorGarmentPromoted` receipt and a promoted `tailor_garments` row with `status='promoted'`, and (e) a `CrdtUpdateRecordV1` row for a single panel control point edit from a second operator session. All writes must be Postgres-backed; `no_sqlite_tripwire` must not fire.

---

### [T-ROADMAP.slice6] Slice 6: Export Pipeline — UV-from-Pattern, Game-Engine Rigging, EveryWear-Equivalent

**Goal.** Export promoted garments to OBJ/FBX/glTF with physically accurate UV islands (UV = flattened 2D pattern piece), optional skinning weight baking for game-engine use, and Tauri IPC commands for the operator UI.

**Marvelous Designer mapping:**
- UV, texturing, and rendering (Group 8): UV-from-pattern flattening, automatic UV packing, texture/material map export
- Import/export and pipeline interop (Group 10): OBJ/FBX/glTF export, animation cache (Alembic/MDD/PC2), EveryWear rigging pipeline
- EveryWear game-engine rigging (moat-7): skinning weight baking, LOD generation — partial equivalent only (full EveryWear rigging requires solver-state knowledge unavailable to open-source tools)

**UV-from-pattern accuracy (moat-6):**

The key insight is that UV islands must equal the exact flattened 2D pattern pieces, not a UV-unwrapped 3D mesh. This requires an unfurl/flatten pass after simulation:

```rust
// UV generation:
// 1. For each panel, the 2D boundary control points in GarmentPanelV1
//    ARE the UV island in pattern space (mm units, normalized to [0,1])
// 2. After simulation, the 3D cloth mesh has moved from rest position;
//    each particle's UV coordinate is fixed at its initial panel-space position
//    (no UV distortion from simulation — pattern space = UV space)
// 3. UV packing: rectangle packing of panel UV islands into a single atlas
//    using a shelf-bin-packing algorithm (no existing UV solve needed)
// 4. Fabric texture grain direction is always correct because
//    weft axis = horizontal in pattern space = horizontal in UV

// This is the distinguishing property: UV is derived from the 2D pattern,
// not from 3D mesh unwrapping. Grain direction is always preserved.
```

**Export format support (Slice 6 targets):**

| Format | Priority | Notes |
|--------|----------|-------|
| OBJ + MTL | P0 | Static frame; simplest path; required for testing |
| glTF/GLB | P1 | Preferred for real-time engines; UV + material maps |
| FBX | P2 | Blender/Maya/Houdini pipeline; skinning weights via FBX bones |
| Alembic | P3 | Animation cache; per-frame vertex positions; morph support |
| DXF | P4 | 2D pattern export for physical production |

**Skinning weights (EveryWear-equivalent, partial):**

```rust
// Skinning weight baking (simplified dual-quaternion skinning):
// For each cloth particle, find the N nearest capsule segments
// (N=4 typical), compute distance-weighted influence weights,
// normalize to sum=1. Output as bone_weights + bone_indices per vertex.
// This is a coarse approximation of MD's EveryWear solver-state-aware baking;
// quality will be lower for complex drape geometries but sufficient for
// game-engine cloth LODs that use in-engine re-simulation anyway.
```

**Tauri IPC commands (kernel API surface):**

```rust
// app/src-tauri/src/commands/tailor.rs
#[tauri::command]
async fn tailor_simulate(garment_id: String, params: SimParamsV1, state: AppState) -> ... { }

#[tauri::command]
async fn tailor_get_garment(garment_id: String, state: AppState) -> ... { }

#[tauri::command]
async fn tailor_promote_garment(garment_id: String, approval: OperatorApprovalEvidence, state: AppState) -> ... { }

#[tauri::command]
async fn tailor_export_garment(garment_id: String, format: ExportFormat, state: AppState) -> ... { }

// axum REST routes in src/api/tailor.rs (server-side):
// POST /tailor/garments
// GET  /tailor/garments/:id
// POST /tailor/garments/:id/simulate
// POST /tailor/garments/:id/promote
// POST /tailor/garments/:id/export
// GET  /tailor/garments/:id/crdt
```

**ArtifactManifest for promoted garments:**

```rust
// Simulated garment mesh = ArtifactPayloadKind::Bundle
// Bundle contents:
//   - vertex_buffer.bin: particle positions (Vec<[f32;3]>)
//   - uv_map.bin: UV coordinates (Vec<[f32;2]>)
//   - index_buffer.bin: triangle indices
//   - material_spec.json: FabricSpecV1
//   - skinning_weights.bin (optional, for game export)
// classification: ArtifactClassification::Low
// retention_ttl_days: None (retained until operator deletes)
// exportable: true
```

**Gate criterion — Slice 6:**
> A promoted garment (passed Slice 5 gate) must export to OBJ with correct UV islands matching the 2D panel boundaries (verified by loading in Blender and confirming grain direction), export to glTF with material maps, and export DXF 2D pattern pieces. The Tauri `tailor_export_garment` command must complete without error for all three formats. Skinning weight bake must be present on FBX export and pass a basic test (no zero-weight vertices on the cloth surface).

---

### [T-ROADMAP.dependencies] Kernel Primitive Dependency Map

This table maps each slice to the exact kernel files and traits it depends on, so a fresh model or build runner can identify dependencies without reading the full codebase.

| Slice | File / Trait | Dependency Kind |
|-------|-------------|-----------------|
| 1 | `tailor-solver` crate (new) | Creates |
| 1 | `wgpu`, `glam`, `parry3d`, `bytemuck` | External crates |
| 2 | `kernel/model_adapter.rs`: `ModelAdapter` trait | Implements (`TailorSimParamAdapter`) |
| 2 | `kernel/sandbox/adapter.rs`: `SandboxAdapter` | Implements (`TailorSandboxAdapter`) |
| 3 | `llm/mod.rs`: `LlmClient` trait (HSK-TRAIT-004) | Consumes (garment author prompts) |
| 3 | `kernel/model_adapter.rs`: `ModelAdapterRequest`, `ContextBundle` | Consumes |
| 4 | `kernel/sandbox/mod.rs`: `SandboxPolicyV1` | Consumes |
| 5 | `storage/mod.rs`: `Database` trait | Extends (new garment CRUD methods) |
| 5 | `storage/postgres.rs`: `PostgresDatabase` | Implements new trait methods |
| 5 | `kernel/mod.rs`: `KernelEventType`, `NewKernelEvent`, `KernelActor` | Extends + Consumes |
| 5 | `kernel/crdt/persistence.rs`: `CrdtUpdateRecordV1` | Consumes |
| 5 | `kernel/crdt/state_vector.rs`: `KnowledgeStateVectorV1` | Consumes |
| 5 | `kernel/kb003_promotion/gate.rs`: `PromotionGate` | Consumes |
| 5 | `kernel/sandbox/no_sqlite_tripwire.rs`: `guard_authority_write` | Calls |
| 5 | `storage/kb003_storage.rs` | Pattern reference |
| 5 | `migrations/`: new `.sql` files | Creates |
| 6 | `storage/mod.rs`: `ArtifactManifest`, `write_dir_artifact` | Consumes |
| 6 | `app/src-tauri/src/commands/`: new `tailor.rs` | Creates |
| 6 | `api/mod.rs`: route registration | Extends |

---

### [T-ROADMAP.risks] Risks and Open Questions

**Risk R1: GPU collision accuracy on exaggerated proportions (Slice 2 gate).**
Exaggerated-proportion avatars (large bust radius >> shoulder width) produce high-curvature collision surfaces that stress XPBD collision response. MD reached CPU-accuracy parity for GPU collision only in 2024.2. The WGSL path may require extra substeps or reduced particle distance (higher mesh resolution) to avoid persistent penetration. Mitigation: start with Swift mode (4 substep / 10 iter) for preview; expose Quality mode (1 substep / 200 iter) as a secondary path; allow operator override of substep count.

**Risk R2: Self-collision tunneling at high velocity (Slice 2).**
Cloth particles can tunnel through self-colliding surfaces when velocity exceeds the spatial hash cell size. The five-step protocol (spatial hash, parameter config, substep integration, velocity clamping, friction) from Carmen Cincotti's blog mitigates this. Velocity clamping formula: `v_max = r × n_substeps / Δt`. Still, fast-motion tests (rapid arm swing, jumping) may expose tunneling. Mitigation: velocity clamping is enforced in the WGSL `update_velocities` shader; detect tunneling in CI by checking max penetration depth per frame.

**Risk R3: LLM hallucinating invalid GarmentDraftV1 JSON (Slice 3).**
LLMs reliably produce well-formed JSON for constrained schemas but may produce topologically invalid panel graphs (seam references non-existent panels, boundary not closed). Mitigation: `SeamGraphValidator` runs before sandbox; `serde_json::from_value::<GarmentDraftV1>` rejects schema violations; `TailorGarmentDraftRejected` EventLedger event is emitted, and the model is re-prompted with the error.

**Risk R4: Anisotropic compliance instability at extreme values (Slice 4).**
High anisotropy (weft/warp compliance ratio > 10x) can destabilize the XPBD solve by creating near-zero compliance in one axis while the other axis is compliant. Mitigation: clamp compliance ratio to [0.1, 10.0] in `FabricSpecV1` validation; expose this clamp in the model API documentation so the LLM is aware of the valid range.

**Risk R5: CRDT conflict on seam definitions (Slice 5).**
Two operators simultaneously editing a seam ratio produces a CRDT conflict because seam ratio is a scalar, not a CRDT-native type. The `KnowledgeStateVectorV1` detects the concurrent-edit case. Mitigation: seam ratio conflicts surface as a UI conflict marker requiring explicit operator resolution (last-write-wins is not safe for seam ratios because M:N ratio validity depends on both edges' particle counts). Design the garment CRDT document to serialize seam definitions as last-write-wins on the atomic seam entry, but flag the operator when two seam definitions for the same edge pair diverge.

**Risk R6: wgpu backend coverage on developer Windows (Slice 1).**
The developer machine runs Windows. wgpu on Windows prefers DX12 (Vulkan available as a secondary). The WGSL shaders must be validated on both DX12 and Vulkan backends. Mitigation: run `cargo test --features dx12` and `--features vulkan` in the solver crate's CI configuration; use `naga` validation on all WGSL shaders as a build step.

**Open question OQ1.** Should the Tailor engine adopt MGPBD (multigrid XPBD, SIGGRAPH 2025) over standard Gauss-Seidel XPBD for high-stiffness anisotropic cases? MGPBD's Python/CUDA implementation is not directly portable to WGSL, but the algebraic multigrid preconditioner could be approximated in WGSL. Decision point: end of Slice 4 quality testing.

**Open question OQ2.** Should Slice 3 fine-tune a local model (ChatGarment-style) on `GarmentDraftV1` JSON, or rely on prompt engineering alone (NGL-Prompter pattern)? Fine-tuning requires a dataset of (description, GarmentDraftV1) pairs. The NGL-Prompter training-free approach is viable for production use cases but may hallucinate unusual garment types. Decision point: end of Slice 3 model lane testing.

**Open question OQ3.** Bevy testbed (Slice 1) — should this be retained as a permanent development viewport or replaced by a Tauri-native wgpu renderer in Slice 2? Retaining Bevy as a developer tool (gated behind a `bevy-testbed` feature flag) is the safest path. A Tauri-native wgpu renderer would require integrating wgpu surface creation with Tauri's window handle, which is non-trivial but documented. Decision point: end of Slice 2.

---

### [T-ROADMAP.timeline] Indicative Timeline and Gate Summary

This is a forward research estimate, not a commitment. Slices 1–2 are the critical path; Slices 3–6 are parallelizable in teams.

| Slice | Name | Gate Criterion Summary | Blocked Until |
|-------|------|----------------------|---------------|
| 1 | GPU Substrate | sphere_drape runs on GPU, CI passes CPU fallback | — |
| 2 | GO/NO-GO | cloth + jiggle + self-collision on exaggerated capsules, model API receipt emitted | Slice 1 gate |
| 3 | Garment Authoring | LLM-authored garment simulates on avatar, seam gather works | Slice 2 gate |
| 4 | Fabric Property System | anisotropic drape visually correct, LLM fabric estimation in range | Slice 3 gate |
| 5 | Kernel Authority | full Postgres/EventLedger/CRDT/promotion gate e2e, no_sqlite_tripwire passes | Slice 4 gate |
| 6 | Export Pipeline | OBJ+glTF+DXF export with correct UV islands, Tauri IPC commands complete | Slice 5 gate |

**Kernel dependency unlock order:**
1. Standalone solver crate (no kernel deps) — Slice 1
2. ModelAdapter + SandboxAdapter consumed — Slice 2
3. LlmClient consumed, model lane for garments — Slice 3
4. ValidationRunner consumed, SimParamsValidator — Slice 4
5. Postgres/EventLedger/CRDT/PromotionGate wired — Slice 5
6. ArtifactManifest + Tauri IPC commands — Slice 6

---

### [T-ROADMAP.sources] Sources

- https://github.com/ccincotti3/webgpu_cloth_simulator — WebGPU XPBD cloth; WGSL constraint-coloring pattern; five-step self-collision protocol reference
- https://github.com/jspdown/cloth — WebGPU XPBD; small-step technique; GPU-only simulation pipeline
- https://github.com/vitalight/Velvet — CUDA XPBD; spatial hashing with CUB radix sort; Jacobi delta accumulation; neighbor caching; architecture reference for WGSL GPU parallelization
- https://carmencincotti.com/2022-11-21/cloth-self-collisions/ — Five-step XPBD self-collision implementation guide: spatial hash, parameter config, substep integration, velocity clamping, friction
- https://github.com/openxrlab/xrtailor — GPU cloth simulation engine; dual Swift/Quality mode architecture; multi-color Gauss-Seidel; Chebyshev acceleration; anisotropy
- https://deepwiki.com/openxrlab/xrtailor/4.2-extended-position-based-dynamics-(xpbd) — XRTailor XPBD constraint types: stretch, bending, LRA, binding; GPU pipeline architecture details
- https://arxiv.org/abs/2504.17614 — Bolt (NVIDIA, April 2025): three-stage transfer/drape/rig pipeline for garments on diverse body proportions; XPBD draping with collision resolution
- https://chunleili.github.io/project-page-mgpbd/ — MGPBD: multigrid-accelerated global XPBD solver (SIGGRAPH 2025); lazy AMG setup; convergence fix for high-stiffness XPBD stalling
- https://arxiv.org/abs/2505.13390 — MGPBD paper (arXiv)
- https://chatgarment.github.io/ — ChatGarment (CVPR 2025, MPI): LLM-to-GarmentCode JSON; three authoring modes; token count reduction; model-first garment authoring reference
- https://github.com/maria-korosteleva/GarmentCode — GarmentCode (ETH Zurich, MIT): parametric sewing pattern DSL; Edge/Panel/Component/Interface types; JSON schema reference for GarmentDraftV1
- https://arxiv.org/abs/2602.20700 — NGL-Prompter (MPI, Feb 2026): training-free VLM garment parameter extraction via Natural Garment Language; validates prompt-only approach for model lane
- https://arxiv.org/html/2504.21476v1 — GarmentDiffusion (IJCAI 2025): edge-oriented token diffusion transformer; 10x shorter sequence; state-of-the-art on DressCodeData
- https://style3d.github.io/design2garmentcode/ — Design2GarmentCode (CVPR 2025, Style3D): LMM generates GarmentCode programs from images/text/sketches
- https://github.com/nikhilr612/xpbdrs — xpbdrs: only current XPBD Rust crate for cloth/deformables; edge-length + volume + bending constraints; MIT
- https://github.com/RobDavenport/softy — softy: CPU Verlet/PBD Rust crate; no_std + WASM; deterministic; CPU fallback path reference
- https://github.com/dimforge/parry — parry: Rust collision shapes (capsule, trimesh, convex hull); used for body segment collision proxies
- https://sotrh.github.io/learn-wgpu/compute/introduction/ — Learn Wgpu: compute pipeline dispatch model; storage buffers; bind groups; iterative substep pattern
- https://github.com/gfx-rs/wgpu — wgpu v29; cross-platform Rust GPU API; Vulkan/Metal/DX12/WebGPU; WGSL via Naga
- https://matthias-research.github.io/pages/publications/XPBD.pdf — Macklin et al. XPBD 2016: Lagrange multiplier accumulation; compliance as inverse stiffness; anisotropic constraint design
- https://80.lv/articles/cloth-simulation-for-games-difficulties-and-current-solutions — Game cloth simulation difficulties; proxy-based collision; pre-baked vs. real-time tradeoffs
- https://sueraywang.github.io/project/xpbdmmd/ — XPBD skirt simulation with Jacobi solver on GPU; distance constraints; sphere/cylinder collision proxies; pipeline stages
