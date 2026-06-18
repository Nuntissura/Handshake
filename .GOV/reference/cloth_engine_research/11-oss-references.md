---
file_id: cloth-engine-oss-references
topic_id: T-OSS-REFERENCES
title: "Open-Source References and Adaptation Map"
status: draft
depends_on:
  - T-CLOTH-SOLVER
  - T-COLLISION
summary: "Consolidated inventory of every OSS project studied for the Handshake Tailor engine, with repo URL, language, license, what to adapt vs. avoid, and the Handshake-native adaptation note for each."
sources: 28
updated_at: "2026-06-17"
---

## [T-OSS-REFERENCES] Open-Source References and Adaptation Map

This document is a greppable, per-project inventory of every open-source reference used to design the Handshake Tailor engine. For each entry the record states: repository URL, primary language, license, latest known version or date, what to adapt, what to avoid, and the Handshake-native adaptation note tying it to the solver crate (`tailor-solver`) or the Handshake Tailor creative module (`src/tailor/`).

Projects are grouped into five layers matching the architecture decision in the foundation context:

1. GPU compute backends
2. XPBD/PBD solver implementations (the algorithmic core)
3. Rigid-body / collision-proxy libraries
4. Bevy-ecosystem cloth integrations (testbed only)
5. AI-driven garment / pattern generation tools

---

### [T-OSS-REFERENCES.gpu-wgpu] wgpu — Cross-Platform Rust GPU API

| Field | Value |
|---|---|
| Repo | <https://github.com/gfx-rs/wgpu> |
| Language | Rust |
| License | Apache-2.0 / MIT dual |
| Latest confirmed | v29.0.3 (2025-05-02); v29.0.1 (2025-03-26) |
| Crates.io | <https://crates.io/crates/wgpu> |

**What it is.** wgpu is the safe, portable Rust GPU API built on the WebGPU specification. It abstracts Vulkan (Windows/Linux), Metal (macOS/iOS), DX12 (Windows), OpenGL ES (fallback), and WebGPU (browser via WASM). Compute pipelines run WGSL shaders; storage buffers are typed with `BufferBindingType::Storage`; workgroup dispatch is via `encoder.dispatch_workgroups(x, y, z)`.

**What to adapt.**

- Use wgpu as the sole GPU transport layer inside `tailor-solver`. No CUDA or Metal-specific code in the solver crate; wgpu's Naga shader translator compiles WGSL to SPIR-V (Vulkan), MSL (Metal), and HLSL (DX12) at runtime.
- Compute pipeline pattern: one `wgpu::ComputePipeline` per solver phase (integrate velocities, solve stretch constraints, solve bend constraints, detect collision, apply friction). Each pipeline reads/writes typed storage buffers holding particle positions, velocities, constraint lists, and collision proxy data.
- Use `wgpu::Buffer` with `STORAGE | COPY_SRC | COPY_DST` for particle position arrays and constraint buffers. Use `MAP_READ` staging buffers to read back simulation results to CPU for EventLedger persistence.
- Recommended workgroup size: 64 invocations per workgroup for 1D physics dispatch (one particle per invocation).

```rust
// tailor-solver Cargo.toml dependency
[dependencies]
wgpu = "29"
bytemuck = { version = "1", features = ["derive"] }
```

**What to avoid.**

- Do not merge wgpu into `handshake_core`. The kernel crate has zero GPU code today and must stay sqlx/Tauri-focused. The solver crate is a separate workspace member.
- Do not target OpenGL ES as a primary backend for cloth simulation; storage buffers have reduced limits on OpenGL; Vulkan/Metal/DX12 are the production targets.
- Dimforge (wgsparkl) found WGSL difficult for advanced math due to lack of a module system. Mitigate by keeping each WGSL file to one constraint family and using wgsl-bindgen for typesafe cross-file interfaces rather than relying on WGSL's limited module features.

**Handshake-native adaptation note.** `tailor-solver` is a standalone Rust crate added as a Cargo workspace member of the Handshake monorepo. It takes `wgpu` as a direct dependency. The `ClothSolver` trait in `tailor-solver/src/lib.rs` is UI-agnostic; `handshake_core`'s `TailorSandboxAdapter` calls it via the trait boundary, not via wgpu directly.

---

### [T-OSS-REFERENCES.gpu-cubecl] CubeCL — Multi-Backend GPU Kernel DSL

| Field | Value |
|---|---|
| Repo | <https://github.com/tracel-ai/cubecl> |
| Language | Rust |
| License | MIT / Apache-2.0 dual |
| Latest confirmed | v0.10.0 (2026-05-11) |

**What it is.** CubeCL is a Rust-language JIT GPU kernel DSL. The `#[cube]` proc-macro rewrites Rust functions to a portable IR that compiles to CUDA (NVIDIA), HIP (AMD/ROCm), Metal (Apple), Vulkan (SPIR-V), WebGPU/WGSL, and CPU with SIMD. Parallelism axes: Vector (SIMD lane), Plane (warp/wavefront), CubeDim (workgroup), CubeCount (dispatch grid). Supports comptime specialization for constraint-type dispatch and automatic vectorization.

**What to adapt.**

- CubeCL is the recommended optional CUDA fast-path for `tailor-solver`, gated behind a `cubecl-cuda` feature flag. When the user has an NVIDIA GPU and the feature is enabled, constraint solve loops run CubeCL kernels instead of WGSL kernels.
- The `#[cube]` macro is particularly useful for the anisotropic stretch constraint kernel, which needs per-edge weft/warp compliance values: comptime specialization can eliminate the compliance-direction branch at compile time for a given material preset.
- CubeCL's CPU backend with SIMD serves as a development/test backend when a real GPU is absent, keeping the constraint solver path exercisable in CI without a GPU.

```rust
// Feature-gated CubeCL in tailor-solver Cargo.toml
[features]
default = ["wgpu-backend"]
wgpu-backend = ["dep:wgpu"]
cubecl-cuda = ["dep:cubecl", "cubecl/cuda"]

[dependencies]
cubecl = { version = "0.10", optional = true }
```

**What to avoid.**

- Do not make CubeCL a required dependency. The primary solver path must work on any wgpu-capable GPU (Vulkan/Metal/DX12) without CUDA installed.
- CubeCL v0.10 is still pre-1.0; do not depend on its API stability for the initial tailor-solver design. Pin the version and treat CubeCL as an enhancement path.

**Handshake-native adaptation note.** The `ClothSolver` trait exposes an async `simulate` method. The WGSL wgpu implementation is `WgpuClothSolver`; the CubeCL CUDA implementation is `CubeclClothSolver`. Both implement `ClothSolver`. The kernel primitive (SandboxAdapter) calls only the trait; runtime selects the backend from a config field in the `TailorEngineConfig` authority row in PostgreSQL.

---

### [T-OSS-REFERENCES.gpu-rust-gpu] rust-gpu — Rust-to-SPIR-V Compiler

| Field | Value |
|---|---|
| Repo | <https://github.com/Rust-GPU/rust-gpu> |
| Language | Rust |
| License | Apache-2.0 / MIT dual |
| Status | Early / experimental (Vulkan-only SPIR-V target) |

**What it is.** rust-gpu compiles regular Rust code (with `#![no_std]` and `spirv-std` imports) to SPIR-V, enabling GPU shader authoring in Rust. Target: Vulkan only today; WGSL/DXIL output are future goals.

**What to adapt.**

- Monitor rust-gpu as a longer-term path for writing constraint kernels in Rust rather than WGSL. Dimforge is migrating wgsparkl from WGSL to rust-gpu specifically to escape WGSL's module-system limitations and to enable potential CUDA output via rust-cuda.
- If rust-gpu reaches WGSL output support or stable multi-backend status before Tailor enters production, it becomes a viable replacement for hand-authored WGSL shaders.

**What to avoid.**

- Do not use rust-gpu for the initial `tailor-solver` implementation. Vulkan-only target breaks macOS (Metal) and browser (WebGPU) support. Use WGSL via wgpu for the v1 solver.

**Handshake-native adaptation note.** No Handshake integration in the near term. Track as a future solver-backend option. If adopted, the `ClothSolver` trait boundary means no changes to `handshake_core` are required.

---

### [T-OSS-REFERENCES.gpu-wgsl-bindgen] wgsl-bindgen — Typesafe WGSL/Rust Interface Generator

| Field | Value |
|---|---|
| Repo | <https://github.com/Swoorup/wgsl-bindgen> |
| Language | Rust |
| License | MIT |
| Latest confirmed | v0.22.2 (2026-03-31) |

**What it is.** wgsl-bindgen is a build-script code generator that parses WGSL shaders via Naga and emits typesafe Rust structs for bind groups, pipeline layouts, vertex/fragment entry descriptors, and buffer layouts. Changing a WGSL uniform type raises a compile error in the Rust caller, catching bind-group mismatches at compile time.

**What to adapt.**

- Use wgsl-bindgen as a `build.rs` step in `tailor-solver` to generate typesafe Rust bindings for every WGSL compute shader (stretch, bend, collision, self-collision, friction). This eliminates the class of runtime panic caused by mismatched bind-group slot types.
- The generated code includes `bytemuck`-compatible struct derives for uniform buffers and `encase`-compatible structs for complex shader types.

```rust
// tailor-solver/build.rs
fn main() {
    wgsl_bindgen::WgslBindgenOptionBuilder::default()
        .workspace_root(env!("CARGO_MANIFEST_DIR"))
        .add_entry_point("shaders/stretch_constraint.wgsl")
        .add_entry_point("shaders/bend_constraint.wgsl")
        .add_entry_point("shaders/collision_response.wgsl")
        .output(std::path::PathBuf::from("src/generated_bindings.rs"))
        .build()
        .unwrap()
        .generate()
        .unwrap();
}
```

**What to avoid.** Do not hand-write `wgpu::BindGroupLayoutEntry` arrays for the solver shaders; these drift silently from the WGSL source. wgsl-bindgen eliminates this drift.

**Companion tool.** `wgsl_to_wgpu` by ScanMountGoat (<https://github.com/ScanMountGoat/wgsl_to_wgpu>, v0.16.0 as of ~June 2025, MIT) is an alternative with similar goals and naga-based parsing. Either tool works; wgsl-bindgen has more recent activity and broader bind-group coverage.

**Handshake-native adaptation note.** Applied at solver crate build time only. Zero impact on `handshake_core`.

---

### [T-OSS-REFERENCES.xpbd-ccincotti3] webgpu_cloth_simulator — WebGPU XPBD Constraint Reference

| Field | Value |
|---|---|
| Repo | <https://github.com/ccincotti3/webgpu_cloth_simulator> |
| Language | TypeScript + WGSL |
| License | Not specified (educational / reference) |
| Constraints | Distance, isometric bending, angular bending, self-collision |

**What it is.** A WebGPU + WGSL XPBD cloth simulator with distance constraints, fast bending, isometric bending, and angular-bending constraints. Implements GPU-parallel constraint solving. Based on Macklin et al. 2016 XPBD paper and NVIDIA's small-step technique. The WGSL shader files are the primary reference.

**What to adapt.**

- Constraint type coverage: the distance constraint (stretch) and isometric bending constraint WGSL implementations are direct algorithmic templates for the Tailor solver's WGSL shaders.
- Self-collision detection approach: particle neighborhood lookup and collision response in WGSL.
- The overall per-frame GPU dispatch loop: `integrate -> solve_constraints (N iterations) -> update_velocities -> resolve_collisions` maps directly to the tailor-solver pipeline.

```wgsl
// Pattern from ccincotti3: stretch constraint kernel structure
@compute @workgroup_size(64)
fn solve_stretch(@builtin(global_invocation_id) id: vec3<u32>) {
    let constraint_idx = id.x;
    if constraint_idx >= arrayLength(&constraints) { return; }
    let c = constraints[constraint_idx];
    let p0 = positions[c.i0];
    let p1 = positions[c.i1];
    let d = p1.xyz - p0.xyz;
    let len = length(d);
    let rest = c.rest_length;
    let compliance = c.compliance; // XPBD: stiffness encoded as compliance
    // ... delta position accumulation
}
```

**What to avoid.**

- TypeScript architecture: ignore all non-WGSL code; the Rust GPU abstraction layer is wgpu not WebGPU-JS.
- No license specified; treat as reference/inspiration for algorithm shape only, not copy-paste. Re-implement the constraint math from the underlying papers (Macklin 2016, Kelager 2010 for isometric bending).

**Handshake-native adaptation note.** The constraint families (stretch, bend, self-collision) map to three WGSL shader files in `tailor-solver/shaders/`. The Lagrange multiplier buffer (`lambda_buffer`) required by XPBD is a per-constraint GPU buffer uploaded fresh each substep.

---

### [T-OSS-REFERENCES.xpbd-jspdown] jspdown/cloth — WebGPU XPBD with Graph Coloring

| Field | Value |
|---|---|
| Repo | <https://github.com/jspdown/cloth> |
| Language | TypeScript (92.6%) + WGSL |
| License | MIT |
| Live demo | <https://jspdown.github.io/cloth/> |

**What it is.** A WebGPU XPBD cloth simulator that runs 100% on the GPU. Uses the small-step XPBD technique from Macklin et al. The key architectural contribution is **parallel Gauss-Seidel via constraint graph coloring**: constraints are partitioned into color groups where no two constraints in the same group share a particle; each color group is solved in a separate GPU dispatch, eliminating write hazards without accumulation buffers.

**What to adapt.**

- **Constraint graph coloring algorithm**: before the simulation starts, assign each constraint a color integer such that no two adjacent constraints share a color; store constraints sorted by color in the GPU buffer; during solve, dispatch one workgroup per color group. This is the safest parallelization strategy for XPBD on GPU and avoids the Jacobi delta-accumulation approach's stability tradeoff.
- Bend and stretch compliance parameters as adjustable uniforms matching Marvelous Designer's weft/warp/shear parameter model.
- The MIT license makes algorithm study and architectural adaptation more permissive than ccincotti3.

```wgsl
// Pattern from jspdown: per-color-group dispatch
// host side: for each color c, dispatch only constraints[color_c]
// GPU side: no atomic needed because constraints in same color share no particles
@compute @workgroup_size(64)
fn solve_color_group(@builtin(global_invocation_id) id: vec3<u32>) {
    let local_constraint_idx = id.x;
    let constraint = color_group_buffer[local_constraint_idx];
    // direct read-modify-write is race-free within a color group
    positions[constraint.i0] += delta0;
    positions[constraint.i1] += delta1;
}
```

**What to avoid.**

- TypeScript architecture; the pattern is the WGSL dispatch strategy, not the JS bindings.
- The graph coloring pre-computation is a CPU step that must happen once when a garment is loaded and the constraint list is finalized; do not re-color every frame.

**Handshake-native adaptation note.** When `GarmentSimulationRun` is initiated via the sandbox, the `TailorSandboxAdapter` calls `solver.precompute_constraint_colors(&garment_mesh)` once after loading the authority garment rows from PostgreSQL, then runs the per-frame dispatch loop. The color assignment is stored in the `GarmentMeshCache` in-memory (not persisted per frame; regenerated on garment load).

---

### [T-OSS-REFERENCES.xpbd-velvet] Velvet — CUDA XPBD Architecture Reference

| Field | Value |
|---|---|
| Repo | <https://github.com/vitalight/Velvet> |
| Language | C++17 + CUDA 11.1 |
| License | MIT |

**What it is.** A CUDA-accelerated XPBD cloth engine. Constraint types: stretch (distance), long-range attachment (LRA), bending (dihedral), SDF collision, spatial-hash particle-particle collision. Uses **Jacobi iteration** (not Gauss-Seidel): all constraint corrections are accumulated in an atomic delta buffer, then averaged and applied once per iteration step, enabling fully data-parallel GPU execution with no coloring pre-computation required. Spatial hashing uses CUB radix sort and caches neighbor lists across substep iterations for performance.

**What to adapt.**

- **Jacobi accumulation buffer pattern**: the `delta_positions` buffer is zeroed each iteration, each constraint kernel atomically adds its correction using `atomicAdd` on the delta, a final kernel divides by the per-particle correction count and applies. This is the alternative to graph coloring and has different stability/cost tradeoffs (Jacobi is slightly less stable but avoids the coloring precompute).
- **Long-range attachment (LRA) constraints**: Velvet implements LRA to prevent cloth from tunneling through fast-moving collision objects. This maps to Marvelous Designer's trim-collision robustness. LRA adds a constraint that fires only when particle-particle or particle-body distance exceeds a threshold, dampening explosive interpenetration.
- **Spatial hash neighbor caching**: caching the neighbor list across substep iterations (refreshing only every N frames) reduces spatial-hash query cost significantly for cloth-cloth self-collision.
- **SDF collision**: signed distance field against a precomputed body SDF, fast per-particle GPU lookup. Reference for avatar body collision.

**What to avoid.**

- CUDA dependency: the entire GPU kernel layer is CUDA-only; reimplement in WGSL for tailor-solver's primary path.
- C++ architecture; use as algorithmic reference only.

**Handshake-native adaptation note.** The LRA constraint type is the primary adapter from Velvet for the `tailor-solver`. It maps to Marvelous Designer's collision thickness / self-collision layer behavior. LRA constraints are stored in the PostgreSQL `tailor_garment_constraints` JSONB column alongside stretch and bend constraints, with `constraint_kind: "lra"`.

---

### [T-OSS-REFERENCES.xpbd-xpbdrs] xpbdrs — Pure-Rust XPBD for Cloth/Deformables

| Field | Value |
|---|---|
| Repo | <https://github.com/nikhilr612/xpbdrs> |
| Language | Rust |
| License | MIT |
| Backend | CPU only |
| Constraints | Edge-length, tetrahedral volume, weak bending |

**What it is.** The only current pure-Rust XPBD crate targeting cloth and deformable meshes. CPU-only. Constraints: edge-length (stretch), tetrahedral volume (for soft-body), and weak bending (dihedral angle for adjacent triangle pairs). Uses `glam` math. Supports TetGen ASCII input and serde/bincode serialization.

**What to adapt.**

- The Rust constraint trait structure and integration loop are useful as a starting-point type sketch for `tailor-solver`'s CPU fallback path. The `Constraint` trait and `XpbdSolver` struct provide a clean Rust API model.
- The weak bending constraint implementation (adjacent triangle dihedral angle) is the closest Rust-native reference for the `BendConstraintKernel` WGSL shader's math.
- `glam` math crate choice: consistent with Rapier post-0.32 and Avian; use `glam` for all CPU-side vector math in `tailor-solver`.

**What to avoid.**

- CPU-only: xpbdrs has no GPU backend; do not use it as the primary solver path. Use as CPU fallback and algorithmic reference only.
- No anisotropic (weft/warp/shear) constraint support; xpbdrs uses isotropic edge-length constraints. The Handshake solver must implement anisotropic stretch separately (two constraint axes: weft direction and warp direction per edge, with independent compliance values).

**Handshake-native adaptation note.** `xpbdrs` informs the `CpuClothSolver` fallback in `tailor-solver` used when no GPU is available (headless server CI, integration tests without a GPU device). The fallback path ensures sandbox runs succeed even on CI runners with no GPU.

---

### [T-OSS-REFERENCES.xpbd-softy] softy — Verlet Cloth with Tearing (CPU/WASM)

| Field | Value |
|---|---|
| Repo | <https://github.com/RobDavenport/softy> |
| Language | Rust |
| License | MIT / Apache-2.0 dual |
| Backend | CPU, `no_std`, WASM compatible |
| Constraints | Distance, pin, angle, bounds; VerletGrid structural/shear/bend + tearing |

**What it is.** A CPU Verlet + PBD Rust library for rope, cloth, and soft-body. `VerletGrid` provides structural links (horizontal/vertical distance), shear links (diagonal distance), and bending links (skip-one distance), plus cloth **tearing** via link removal when stretch exceeds threshold. `no_std` and WASM compatible. Last updated March 2026.

**What to adapt.**

- **Cloth tearing**: Marvelous Designer does not expose tearing natively, but the Tailor engine may add it as a creative-mode feature. Softy's link-removal-on-threshold model is the simplest tearing implementation: when a constraint's rest-length stretch ratio exceeds a per-garment `tear_threshold`, remove the constraint from the active list. In the GPU version, implement as a conditional write to a `torn_constraints` bitfield buffer.
- **Pin constraint**: softy's `PinConstraint` (zero-DOF vertex anchor) maps to Marvelous Designer's "pin" tool and to XPBD attachment constraints used for seam endpoints during fitting.
- WASM compatibility architecture: softy demonstrates how to keep a Rust physics library WASM-compatible. tailor-solver should follow the same `no_std` + optional `wasm-bindgen` feature pattern for future browser/Tauri-WebView use.

**What to avoid.**

- Verlet integration for cloth is less stable at large timesteps than XPBD. Do not use softy's integration scheme for the production solver. Use XPBD (Macklin 2016) for timestep-independent stiffness.

**Handshake-native adaptation note.** Softy's CPU tearing model informs the `GarmentTearRecord` event type emitted to the EventLedger when a tear constraint fires during a sandbox simulation run.

---

### [T-OSS-REFERENCES.collision-rapier] Rapier — Rigid-Body Collision Proxy Layer

| Field | Value |
|---|---|
| Repo | <https://github.com/dimforge/rapier> |
| Language | Rust |
| License | Apache-2.0 |
| Latest confirmed | v0.32 (2025) |
| Website | <https://rapier.rs/> |

**What it is.** Rapier is a mature 2D/3D Rust rigid-body physics engine by Dimforge. Rigid body types: Dynamic, Fixed, KinematicPosition, KinematicVelocity. Shapes via `parry3d`: balls, capsules, convex hulls, trimesh, compound. Uses impulse-based TGS (Temporal Gauss-Seidel) Soft solver for contacts. **No cloth or soft-body solver confirmed as of 2025 review; wgsparkl (MPM, separate project) handles soft-body effects.** GPU rigid-body work (wgrapier/rust-gpu) is in progress but not production.

**What to adapt.**

- Use Rapier only for **rigid collision proxy construction**: the avatar body is decomposed into a set of Rapier `ColliderBuilder::capsule` or `ColliderBuilder::ball` shapes (one per body segment: head, neck, torso, upper arm, lower arm, upper leg, lower leg, foot). These proxy shapes are extracted as `parry3d` shapes and passed to the XPBD cloth solver as static collision objects.
- Rapier's `rapier3d::geometry::Collider` shape descriptors are the input format for the body proxy mesh. The XPBD solver reads position/orientation of each proxy shape each frame and resolves cloth particle penetrations against them.

```rust
// tailor-solver: avatar body proxy via parry3d shapes
use parry3d::shape::{Capsule, SharedShape};
pub struct AvatarBodyProxy {
    pub segments: Vec<(SharedShape, Isometry3<f32>)>,
}
```

**What to avoid.**

- Do not use Rapier as the cloth solver or for soft-body simulation. Rapier has no cloth constraints.
- Do not use Rapier's full rigid-body world (broad phase, narrow phase, integrator) for the avatar proxy; it adds unnecessary overhead. Extract `parry3d` shapes directly for static collision geometry.
- Dimforge is moving away from WGSL for GPU physics (wgrapier -> rust-gpu); do not depend on wgrapier WGSL bindings.

**Handshake-native adaptation note.** The avatar body proxy is computed once from the `AvatarBodySpec` JSON stored in the `tailor_garments.avatar_spec_json` JSONB column, converted to `parry3d` shapes, and uploaded to the GPU as a static collision-proxy buffer for the duration of the simulation sandbox run.

---

### [T-OSS-REFERENCES.collision-parry] parry — Collision Detection Primitives

| Field | Value |
|---|---|
| Repo | <https://github.com/dimforge/parry> |
| Language | Rust |
| License | Apache-2.0 |
| Crates | `parry2d`, `parry3d`, `parry2d-f64`, `parry3d-f64` |

**What it is.** Parry is Dimforge's standalone Rust collision detection library. GJK, SAT, SDF, BVH traversal. Shapes: Ball, Capsule, Cuboid, ConvexHull, Trimesh, Compound, Halfspace. Used by both Rapier and Avian as the collision shape backend.

**What to adapt.**

- `parry3d` is the source of `SharedShape` descriptors for the avatar body proxy (see Rapier entry above).
- `parry3d::query::point_in_shape` and `parry3d::query::distance` for CPU-side nearest-point queries during constraint pre-computation.
- Trimesh shape: the avatar mesh imported as OBJ/FBX is converted to a `parry3d::shape::TriMesh` for broad-phase body collision. For GPU collision, this trimesh is voxelized into a signed distance field (SDF) texture uploaded to the GPU.

**What to avoid.** Do not use parry's BVH/DBVT for cloth-cloth self-collision on GPU; implement spatial hashing in WGSL instead (see Velvet pattern). Parry's CPU BVH is fine for rigid proxy queries during constraint setup.

**Handshake-native adaptation note.** `parry3d` is a direct dependency of `tailor-solver` (no feature gate required). It is used in the solver's CPU layer for avatar proxy shape construction and in the `TailorValidationRunner` to verify that a promoted garment's simulated mesh does not interpenetrate the avatar body at any frame.

---

### [T-OSS-REFERENCES.bevy-avian] Avian — ECS Physics Engine (Testbed Viewport Only)

| Field | Value |
|---|---|
| Repo | <https://github.com/avianphysics/avian> |
| Language | Rust |
| License | MIT / Apache-2.0 dual |
| Latest confirmed | v0.6.1 (2026-03-23) |
| Bevy compatibility | Avian 0.6 / Bevy 0.18 |

**What it is.** Avian is the ECS-driven Bevy physics engine that superseded `bevy_xpbd`. Rigid bodies + collision (parry-backed) + joints. Impulse-based TGS Soft solver for contacts. Supports custom XPBD constraints via the `XpbdConstraint` trait. No cloth or soft-body built-in.

**What to adapt.**

- The `XpbdConstraint` trait in Avian shows the Rust API pattern for user-defined XPBD constraints integrated into a Bevy ECS world. This is the direct architectural inspiration for the `ClothConstraint` trait in `tailor-solver`'s CPU layer.
- Avian's `Collider` and `RigidBody` ECS components in the Bevy testbed viewport allow the developer to place avatar proxy rigid bodies alongside cloth mesh entities without writing custom physics plumbing.
- For the throw-away Bevy testbed (kept strictly out of the solver crate): add `avian3d` as a dev-dependency in a `tailor-solver-testbed` example crate; cloth mesh rendered as a Bevy `Mesh`; Avian rigid bodies for floor/avatar proxy; the `tailor-solver` XPBD engine runs as a separate system writing to a shared particle buffer that Bevy renders.

**What to avoid.**

- Do not add Avian as a non-dev dependency of `tailor-solver`. The solver crate must be Bevy-free.
- Do not use Avian's contact solver for cloth-avatar collision in production; use the XPBD collision kernels in the WGSL solver instead.

**Handshake-native adaptation note.** Avian lives only in `tailor-solver-testbed/` (a separate example crate, never imported by `handshake_core`). The Tauri-based Handshake UI renders garments using its own wgpu render pipeline, not Bevy.

---

### [T-OSS-REFERENCES.bevy-silk] bevy_silk — CPU Verlet Cloth for Bevy

| Field | Value |
|---|---|
| Repo | <https://github.com/ManevilleF/bevy_silk> |
| Language | Rust |
| License | MIT |
| Latest confirmed | v0.10.0 / Bevy 0.17 |
| Backend | CPU Verlet; no GPU |

**What it is.** bevy_silk is a CPU Verlet-based cloth engine for Bevy using stick (distance) constraints. Anchored vertices for pinning. Wind forces (constant and sinusoidal). Collision with Rapier and Avian (experimental). Bevy 0.7-0.17 support (0.10.0 targets Bevy 0.17).

**What to adapt.**

- ECS component design: `ClothComponent`, `ClothAnchor`, `ClothConfig` as Bevy components on the mesh entity. Reference for how cloth properties are exposed to a Bevy ECS world without the solver knowing about ECS.
- Wind force model: constant wind vector + sinusoidal gust component is the reference for the `WindFieldUniform` GPU buffer in `tailor-solver`.
- Pin/anchor vertex system: maps to XPBD attachment constraints (zero compliance, infinite stiffness) for garment fitting endpoints.

**What to avoid.**

- Do not use bevy_silk's Verlet integration or stick constraints for production simulation; XPBD is required for timestep-independent stiffness and anisotropic fabric.
- CPU-only: bevy_silk has no GPU path; the tailor-solver WGSL path replaces it entirely.
- Collision is marked experimental in bevy_silk; do not depend on its collision for correctness.

**Handshake-native adaptation note.** bevy_silk is reference only; no Handshake dependency. Wind force model inspires the `WindConfig` field in `TailorSimulationParams` (stored in PostgreSQL `tailor_garments.simulation_params_json`).

---

### [T-OSS-REFERENCES.garment-garmentcode] GarmentCode — Parametric Sewing Pattern DSL

| Field | Value |
|---|---|
| Repo | <https://github.com/maria-korosteleva/GarmentCode> |
| Language | Python (99.8%) |
| License | MIT |
| Demo | <https://garmentcode.ethz.ch> |

**What it is.** GarmentCode (ETH Zurich) is a modular parametric sewing-pattern programming framework. Core types: `Edge`, `Panel`, `Component`, `Interface`. Garment programs in `assets/garment_programs/` accept YAML body-measurement and design-parameter presets; output is a JSON pattern configuration and a 3D draped mesh (via Qualoth). Used as the JSON intermediate by ChatGarment, DressCode/SewingGPT, Design2GarmentCode, and NGL-Prompter.

**What to adapt.**

- The **JSON panel+edge schema** is the direct model for the Handshake `GarmentSpec` authority schema. A `GarmentSpec` stored in `tailor_garments.draft_json` (PostgreSQL JSONB) should round-trip to/from GarmentCode JSON for interop with ChatGarment-style LLM tooling.
- Panel/edge type vocabulary: `panel_id`, `edges: [{kind, control_points, rest_length}]`, `stitch_pairs: [{panel_a, edge_a, panel_b, edge_b, ratio}]`, `fabric_params: {stretch_weft, stretch_warp, shear, bend_weft, bend_warp, density_g_m2}`. This maps one-to-one to the constraint types the XPBD solver must implement.
- The `Interface` type (seam definition linking two `Panel` edges) is the representation model for the sewing constraint list passed to the solver.

```rust
// tailor-solver: GarmentSpec round-trip type sketch
#[derive(serde::Serialize, serde::Deserialize)]
pub struct GarmentSpec {
    pub panels: Vec<Panel>,
    pub stitches: Vec<StitchPair>,
    pub fabric_params: FabricParams,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StitchPair {
    pub panel_a: String,
    pub edge_a: usize,
    pub panel_b: String,
    pub edge_b: usize,
    pub ratio: f32, // 1.0 = 1:1, <1.0 = gathering
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct FabricParams {
    pub stretch_weft_compliance: f32,
    pub stretch_warp_compliance: f32,
    pub shear_compliance: f32,
    pub bend_weft_compliance: f32,
    pub bend_warp_compliance: f32,
    pub density_g_m2: f32,
    pub collision_thickness_mm: f32,
}
```

**What to avoid.**

- Do not port GarmentCode Python to Rust. Use GarmentCode JSON as an interchange format only; the Rust solver reads typed `GarmentSpec` structs deserialized from that JSON.
- GarmentCode uses Qualoth (commercial) for simulation; the Handshake solver replaces Qualoth entirely.

**Handshake-native adaptation note.** The `TailorModelAdapter` (implementing `ModelAdapter` from `kernel/model_adapter.rs`) receives a ChatGarment-style LLM response containing GarmentCode JSON, deserializes it into a `GarmentSpec`, validates the topology (closure, no degenerate edges, valid stitch pairs), and if valid, stores it as `draft_json` in a new `tailor_garments` PostgreSQL row with `status = 'draft'`, emitting a `TailorGarmentDraftProposed` EventLedger event.

---

### [T-OSS-REFERENCES.garment-chatgarment] ChatGarment — LLM-to-GarmentCode VLM

| Field | Value |
|---|---|
| Repo | <https://github.com/biansy000/ChatGarment> |
| Language | Python (LLaVA fine-tune) |
| License | Apache-2.0 |
| Publication | CVPR 2025 |
| Site | <https://chatgarment.github.io/> |

**What it is.** ChatGarment fine-tunes LLaVA (a large VLM) using LoRA on GarmentCode JSON to produce structured garment specifications from three input modes: estimate from image, generate from text, edit via dialogue. Output: a JSON file with garment type + continuous numerical panel/edge parameters, decoded by GarmentCode into simulation-ready 2D sewing patterns. The VLM token count was reduced from ~900 to ~350 by simplifying GarmentCode's grammar for fine-tuning.

**What to adapt.**

- **Model-first authoring pattern**: ChatGarment is the field proof that an LLM can output structured GarmentCode JSON in a single inference pass, replacing manual 2D panel editing. This is the architectural model for the Tailor engine's LLM-steerable garment authoring: the `TailorModelAdapter` issues a completion request via `LlmClient`, and the response is parsed as `GarmentSpec` JSON.
- The **three-mode API** (estimate-from-image, generate-from-text, edit-via-dialogue) maps to three tool definitions in the Handshake MCP gate: `garment.estimate`, `garment.generate`, `garment.edit`. Each tool call's output is validated and promoted via the existing sandbox/validation/promotion pipeline.
- **Dialogue editing mode**: the edit-via-dialogue mode (change collar width, lengthen sleeves, etc.) maps to the CRDT garment document: each LLM-authored edit is a `CrdtUpdateRecordV1` against the `GarmentSpec` document, with conflict resolution handled by the existing `CrdtSnapshotRecordV1` + `yjs_bridge` layer.
- Apache-2.0 license permits the Handshake team to adapt the fine-tune methodology to train a Handshake-specific garment generation model if/when the model-lane strategy calls for a fine-tuned local model.

**What to avoid.**

- Do not ship a ChatGarment model weight dependency in the Handshake kernel. The `LlmClient` trait abstracts over any model (local Ollama, official-CLI bridge, BYOK). ChatGarment's methodology is the training recipe, not a required runtime dependency.
- LLaVA-specific inference code is Python; the Handshake adapter calls the model via the provider-agnostic `LlmClient` trait.

**Handshake-native adaptation note.** `TailorModelAdapter` wraps `LlmClient::completion()` with a system prompt that instructs the model to respond with GarmentCode-compatible JSON. The prompt format is based on the ChatGarment inference prompts but adapted to the `GarmentSpec` Rust schema. The adapter lives at `src/tailor/model_adapter.rs` in `handshake_core`.

---

### [T-OSS-REFERENCES.garment-garmentdiffusion] GarmentDiffusion — Diffusion Transformer Sewing Pattern Generator

| Field | Value |
|---|---|
| Repo | <https://github.com/Shenfu-Research/GarmentDiffusion> |
| Language | Python (model) / JavaScript (project site) |
| License | Not specified in repo (research code) |
| Publication | IJCAI 2025 |
| arXiv | <https://arxiv.org/abs/2504.21476> |

**What it is.** GarmentDiffusion is a diffusion transformer for 3D sewing pattern generation. Input modalities: text, image, or incomplete pattern. Output: vectorized centimeter-precise edge token representations of sewing patterns, 10x shorter sequence than SewingGPT. Accepts DressCodeData and GarmentCodeData benchmarks.

**What to adapt.**

- **Edge-token representation**: GarmentDiffusion encodes 3D sewing patterns as compact edge tokens with edge-order index and panel-order index. This is an alternative compact wire representation for the `GarmentSpec` JSON that reduces LLM context length. Evaluate for fine-tuned local model scenarios where token budget is tight.
- **Multi-modal input**: text + image + incomplete-pattern inputs demonstrate that a garment generation model must handle partial specifications gracefully. The Handshake `TailorModelAdapter` should accept `PartialGarmentSpec` (some panels filled, some empty) as input context alongside the text prompt.

**What to avoid.**

- No explicit open-source license confirmed in the repo; treat implementation code as research reference only, not copy-pasteable.
- Python diffusion training pipeline; not portable to Rust runtime. Tailor consumes generation results via the `LlmClient` trait, not the diffusion pipeline directly.

**Handshake-native adaptation note.** GarmentDiffusion is a model-training reference for the Handshake team if a fine-tuned local garment-diffusion model is added to the model-lane registry. The `ModelRuntime` trait in `handshake_core/src/model_runtime/trait.rs` would host the inference path; Tailor consumes the output the same way as any other `LlmClient` response.

---

### [T-OSS-REFERENCES.garment-dresscode] DressCode / SewingGPT — GPT Autoregressive Pattern Generator

| Field | Value |
|---|---|
| Repo | <https://github.com/IHe-KaiI/DressCode> |
| Language | Python |
| License | Not specified |
| Publication | SIGGRAPH 2024 |

**What it is.** DressCode is a text-driven 3D garment generation framework. SewingGPT is its core: a GPT-style autoregressive model generating sewing patterns conditioned on text via cross-attention. Full pipeline: LLM for shape + texture prompts -> SewingGPT for sewing patterns -> fine-tuned Stable Diffusion for texture. Established the DressCodeData benchmark. Output: 3D sewing patterns (mesh) + PBR textures compatible with Marvelous Designer.

**What to adapt.**

- **DressCodeData benchmark**: the dataset is the standard evaluation benchmark for sewing pattern generation quality; use it to measure tailor-solver simulation fidelity against reference patterns if/when the Handshake team evaluates generated garments at scale.
- **LLM-for-prompts -> sewing-model pipeline** architecture: the two-stage approach (LLM generates semantics -> specialized model generates geometry) is worth considering if a single end-to-end model proves too variable for production quality. The Handshake `TailorModelAdapter` could implement this two-stage call inside a single `ModelAdapter.invoke()` by chaining two `LlmClient::completion()` calls.

**What to avoid.**

- SewingGPT autoregressive generation is superseded in speed by GarmentDiffusion (100x faster claimed). Do not design the primary generation path around autoregressive sewing-pattern generation.

**Handshake-native adaptation note.** DressCodeData is a validation dataset reference for the `TailorValidationRunner` acceptance tests; not a runtime dependency.

---

### [T-OSS-REFERENCES.garment-ngl] NGL-Prompter — Training-Free VLM-to-GarmentCode

| Field | Value |
|---|---|
| arXiv | <https://arxiv.org/abs/2602.20700> |
| Institution | MPI for Intelligent Systems (Black lab) |
| Language | Python |
| License | Not specified (research preprint, Feb 2026) |

**What it is.** NGL-Prompter introduces Natural Garment Language (NGL), a restructuring of GarmentCode into terms that large VLMs can read and generate more accurately. The pipeline is training-free: it prompts a VLM (no fine-tuning) to extract NGL-structured garment parameters from an image, then deterministically maps NGL to valid GarmentCode. Handles multi-layer outfits. State-of-the-art on Dress4D, CloSe, and in-the-wild fashion images as of Feb 2026.

**What to adapt.**

- **Training-free approach**: for the Tailor engine's LLM-steerable authoring tier, NGL-Prompter demonstrates that a strong general-purpose VLM (GPT-4V, Claude, Gemini Ultra) can produce accurate GarmentCode JSON via structured prompting alone, without requiring a fine-tuned model. This aligns with Handshake's BYOK model-lane strategy: the `TailorModelAdapter` can use the operator's own API key and a capable general model rather than requiring a locally fine-tuned garment specialist.
- **NGL intermediate language**: restructure the system prompt in `TailorModelAdapter` so the model produces NGL-style parameter descriptions first (natural language garment description with measurements), then a second prompt call translates NGL to `GarmentSpec` JSON. This two-hop approach reduces hallucination compared to asking the model to generate raw JSON in one shot.
- **Multi-layer outfit handling**: NGL-Prompter's ability to handle layered clothing (jacket over shirt over pants) informs the `GarmentSpec.layers: Vec<LayerSpec>` field, where each layer has its own panels, stitches, and fabric params, and the solver processes layers in order (innermost first) with collision between layers.

**What to avoid.**

- No code repository confirmed as of this writing; NGL-Prompter is a preprint. Adapt the prompt-engineering insight, not specific code.

**Handshake-native adaptation note.** The NGL two-hop prompting pattern is encoded in the `TailorModelAdapter`'s `build_context_bundle()` method as a structured system prompt injected into the `ContextBundle` passed to `ModelAdapter.invoke()`. The first hop uses `ModelAdapterRequest` with `tool_hint: GarmentDescribe`; the second hop uses `tool_hint: GarmentSpecialize`. Both hops are logged to the EventLedger as separate `ModelAdapterInvoked` events linked by `causation_id`.

---

### [T-OSS-REFERENCES.garment-dress123] Dress-1-to-3 — Differentiable XPBD for Pattern Refinement

| Field | Value |
|---|---|
| Site | <https://dress-1-to-3.github.io/> |
| arXiv | <https://arxiv.org/abs/2502.03449> |
| Publication | ACM Transactions on Graphics / SIGGRAPH 2025 |
| Institution | UCLA, University of Utah |

**What it is.** Dress-1-to-3 reconstructs physics-plausible, simulation-ready separated garments with sewing patterns from a single in-the-wild image. Pipeline: (1) image-to-coarse-sewing-pattern model, (2) multi-view diffusion model for 3D reference, (3) differentiable garment simulator (XPBD-based) used to refine the sewing pattern via gradient descent against the multi-view reference.

**What to adapt.**

- **Differentiable XPBD for pattern refinement**: the paper demonstrates that XPBD solver outputs can be made differentiable (via the differentiable simulation technique from warp/Taichi), allowing gradient-based optimization of pattern parameters (seam lengths, panel dimensions) to match a visual reference. This is the forward-research path for a Handshake "auto-fit" feature: given a reference avatar body scan, optimize `GarmentSpec` parameters to minimize fit error by running the differentiable solver in a loop.
- **Simulation-ready separated garments**: the goal of producing per-garment-layer separated simulation meshes (jacket, shirt, pants as separate layers) directly informs the `GarmentSpec.layers` design above.

**What to avoid.**

- No public code repository confirmed. The differentiable XPBD technique is a forward-research reference, not an immediate implementation target for `tailor-solver` v1.

**Handshake-native adaptation note.** The auto-fit gradient-descent loop is flagged as a future `TailorFittingOptimizer` service in the Tailor module, beyond the initial `simulate-on-static-avatar` sandbox path.

---

### [T-OSS-REFERENCES.wgsparkl] wgsparkl / Slosh — MPM Soft-Body (Dimforge, GPU)

| Field | Value |
|---|---|
| Repo | <https://github.com/dimforge/wgsparkl> |
| Language | Rust + WGSL (wgsparkl) / Slang (Slosh successor) |
| License | Apache-2.0 (dual, per dimforge blog) |

**What it is.** wgsparkl is Dimforge's MPM (Material Point Method) soft-body simulation with WebGPU/WGSL. MPM handles large deformations, fracture, and granular media better than XPBD for certain material classes. Dimforge is migrating it to Slang (renamed Slosh) due to WGSL module-system limitations. Very early stage (7 commits, 20 stars as of inspection).

**What to adapt.**

- **Practical lesson from Dimforge**: WGSL's lack of a module system is a real pain point for complex GPU physics math. Mitigate in `tailor-solver` by keeping WGSL files small (one constraint family per file), using `wgsl-bindgen` for typesafe Rust/WGSL interfaces, and encapsulating all shader constants in Rust-side uniform buffers rather than WGSL constants.
- MPM is architecturally relevant if Tailor later adds rubber/leather large-deformation materials or fluid-like fabric behaviors beyond XPBD's range.

**What to avoid.**

- MPM is not suitable for thin cloth simulation; XPBD is the correct choice for garment draping. wgsparkl is a reference for GPU physics infrastructure lessons, not for cloth constraints.
- Do not follow Dimforge's Slang migration path for the initial tailor-solver; Slang is not yet cross-platform in the same way as WGSL+wgpu.

**Handshake-native adaptation note.** The WGSL-module-system lesson is the only immediately actionable output: apply it as a shader architecture constraint in the `tailor-solver` design.

---

### [T-OSS-REFERENCES.adapt-summary] Adaptation Priority Summary

The table below consolidates adoption decisions. "Primary" = used in the initial `tailor-solver` build. "CubeCL" = optional CUDA fast-path. "Testbed" = Bevy dev-only example. "Ref only" = algorithm/schema inspiration, no code dependency.

| Project | Layer | Adopt level | Key contribution |
|---|---|---|---|
| wgpu v29 | GPU backend | Primary | Cross-platform GPU transport, WGSL compute |
| CubeCL v0.10 | GPU backend | Optional CUDA | `#[cube]` Rust GPU kernels, CUDA fast-path |
| rust-gpu | GPU backend | Future watch | Rust-to-SPIR-V, track for WGSL replacement |
| wgsl-bindgen v0.22 | Tooling | Primary | Typesafe WGSL/Rust build-time binding gen |
| wgsl_to_wgpu v0.16 | Tooling | Alternate | Same goal as wgsl-bindgen; choose one |
| ccincotti3/webgpu_cloth | XPBD reference | Ref only | Constraint type set, angular bending math |
| jspdown/cloth | XPBD reference | Ref only | Graph-coloring parallel Gauss-Seidel pattern |
| Velvet | XPBD reference | Ref only | Jacobi accumulation, LRA constraints, spatial hash caching |
| xpbdrs | XPBD Rust crate | Primary (CPU fallback) | CPU fallback path, `glam` math |
| softy | Verlet cloth | Ref only | Tearing model, pin constraint, WASM pattern |
| Rapier v0.32 | Collision proxy | Primary (shape extraction) | Rigid body proxy shapes (not cloth solver) |
| parry3d | Collision | Primary | Avatar proxy `SharedShape`, trimesh, SDF |
| Avian v0.6 | Bevy ECS | Testbed only | XpbdConstraint trait pattern, testbed viewport |
| bevy_silk v0.10 | Bevy cloth | Ref only | Wind force model, pin/anchor ECS pattern |
| GarmentCode | Pattern DSL | Primary (schema) | `GarmentSpec` JSON round-trip schema |
| ChatGarment | LLM authoring | Primary (pattern) | `TailorModelAdapter` design, 3-mode API |
| GarmentDiffusion | Diffusion gen | Future model ref | Edge-token compact representation |
| DressCode/SewingGPT | Autoregressive | Ref only | DressCodeData benchmark, 2-stage pipeline pattern |
| NGL-Prompter | Prompt eng | Primary (prompting) | NGL two-hop prompting, multi-layer outfits |
| Dress-1-to-3 | Diff XPBD | Future (auto-fit) | Differentiable XPBD for pattern refinement |
| wgsparkl/Slosh | MPM GPU | Ref only | WGSL module-system lessons |

---

### [T-OSS-REFERENCES.risks] Risks and Open Questions

**Risk 1 — No Rust GPU XPBD cloth crate with anisotropic constraints exists.** `xpbdrs` is CPU-only and isotropic. The `tailor-solver` WGSL constraint kernels must be written from scratch using Velvet/ccincotti3/jspdown as algorithmic references. Mitigation: start with isotropic stretch + isometric bending (matching 80% of MD's fabric behavior), add anisotropic weft/warp/shear as a v2 feature.

**Risk 2 — WGSL module system limitations.** Dimforge abandoned WGSL for wgsparkl due to lack of module support. Mitigation: keep WGSL files small (one shader per constraint family), use wgsl-bindgen to generate the Rust interface, and accept that WGSL requires duplication of math utilities across shader files.

**Risk 3 — ChatGarment / GarmentDiffusion license uncertainty.** ChatGarment is Apache-2.0 (confirmed). GarmentDiffusion has no explicit license in the repo. NGL-Prompter is a preprint. Tailor uses these as prompting pattern references and schema inspiration, not as runtime dependencies; the risk is limited.

**Risk 4 — CubeCL API pre-1.0 stability.** CubeCL v0.10 (2026-05-11) may break API before Tailor ships. Mitigation: gate CubeCL behind a feature flag; the WGSL primary path is the production default.

**Risk 5 — Rapier has no cloth solver and will not add one.** Confirmed by Dimforge 2025 review. The XPBD cloth constraints are entirely hand-written in WGSL. Rapier is used only for rigid collision proxy shapes. Risk: if Rapier's `parry3d` shape API changes between v0.32 and a future version, the avatar proxy extraction code in the solver crate must be updated.

**Open question 1.** Should the solver use constraint graph coloring (jspdown pattern, no atomic needed) or Jacobi accumulation (Velvet pattern, requires `atomicAdd`)? Graph coloring has better numerical properties and determinism; Jacobi requires no precompute. Decision: implement graph coloring for the initial WGSL path; add Jacobi as an alternative for dynamically changing constraint topologies (cut cloth / tearing).

**Open question 2.** GarmentCode Python is the reference schema; does Handshake need a Rust GarmentCode deserializer? Recommendation: implement a thin `garmentcode_compat` module in `src/tailor/` that maps GarmentCode JSON field names to `GarmentSpec` Rust fields. Do not port the full GarmentCode DSL; the operator authors garments via the LLM layer, not by writing Python GarmentCode programs.

---

### [T-OSS-REFERENCES.sources] Sources

1. <https://github.com/gfx-rs/wgpu> — wgpu repo, backends confirmed
2. <https://crates.io/crates/wgpu> — wgpu v29.0.3 version confirmed
3. <https://github.com/gfx-rs/wgpu/releases> — wgpu v29.0.3 (2025-05-02), v29.0.1 (2025-03-26) release dates
4. <https://github.com/tracel-ai/cubecl> — CubeCL v0.10.0 (2026-05-11), backends, license
5. <https://github.com/Rust-GPU/rust-gpu> — rust-gpu, Vulkan-only SPIR-V, status
6. <https://github.com/Swoorup/wgsl-bindgen> — wgsl-bindgen v0.22.2 (2026-03-31), MIT
7. <https://github.com/ScanMountGoat/wgsl_to_wgpu> — wgsl_to_wgpu v0.16.0, MIT
8. <https://github.com/ccincotti3/webgpu_cloth_simulator> — TypeScript/WGSL XPBD, constraint types
9. <https://github.com/jspdown/cloth> — TypeScript/WGSL XPBD, graph coloring, MIT
10. <https://jspdown.github.io/cloth/> — live demo
11. <https://github.com/vitalight/Velvet> — C++/CUDA XPBD, Jacobi, LRA, spatial hash, MIT
12. <https://github.com/nikhilr612/xpbdrs> — Rust CPU XPBD, MIT, constraints confirmed
13. <https://github.com/RobDavenport/softy> — Rust Verlet cloth, MIT/Apache-2, tearing, WASM
14. <https://github.com/dimforge/rapier> — Rapier Apache-2, v0.32, no cloth
15. <https://rapier.rs/> — Rapier website
16. <https://dimforge.com/blog/2026/01/09/the-year-2025-in-dimforge/> — Rapier 0.32 confirmed, wgsparkl/Slosh, GPU rust-gpu plans, no cloth solver
17. <https://github.com/dimforge/parry> — parry Apache-2, shapes
18. <https://github.com/avianphysics/avian> — Avian v0.6.1 (2026-03-23), Bevy 0.18, MIT/Apache-2
19. <https://github.com/ManevilleF/bevy_silk> — bevy_silk v0.10.0/Bevy 0.17, MIT, CPU Verlet
20. <https://github.com/maria-korosteleva/GarmentCode> — GarmentCode MIT, Panel/Edge/Interface types
21. <https://garmentcode.ethz.ch> — GarmentCode demo
22. <https://github.com/biansy000/ChatGarment> — ChatGarment Apache-2.0, LLaVA, 3 modes, local inference
23. <https://chatgarment.github.io/> — ChatGarment site
24. <https://github.com/Shenfu-Research/GarmentDiffusion> — GarmentDiffusion, edge tokens, IJCAI 2025
25. <https://arxiv.org/abs/2504.21476> — GarmentDiffusion arXiv
26. <https://github.com/IHe-KaiI/DressCode> — DressCode/SewingGPT, SIGGRAPH 2024
27. <https://arxiv.org/abs/2602.20700> — NGL-Prompter arXiv (Feb 2026), NGL language, training-free VLM
28. <https://arxiv.org/abs/2502.03449> — Dress-1-to-3 arXiv, differentiable XPBD, SIGGRAPH 2025
29. <https://github.com/dimforge/wgsparkl> — wgsparkl MPM WGSL, Apache-2, early stage
30. <https://arxiv.org/html/2412.17811v1> — ChatGarment arXiv HTML, LLaVA fine-tune detail
31. <https://matthias-research.github.io/pages/publications/XPBD.pdf> — Macklin et al. 2016 XPBD paper
