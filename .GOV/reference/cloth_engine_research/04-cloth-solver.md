---
file_id: cloth-solver-xpbd-wgpu
topic_id: T-CLOTH-SOLVER
title: "Cloth Solver: XPBD on wgpu/WGSL"
status: draft
depends_on:
  - T-CODEBASE
summary: "Design of the standalone tailor-solver Rust crate: XPBD algorithm, substepping, GPU constraint projection in WGSL via wgpu, data layout, parallelism strategy, self-collision, determinism, and Handshake integration boundary."
sources: 28
updated_at: "2026-06-17"
---

## [T-CLOTH-SOLVER] Cloth Solver: XPBD on wgpu/WGSL

### [T-CLOTH-SOLVER.overview] Overview and Scope

This topic covers the core physics solver for the Tailor creative module. The solver is a **standalone, UI-agnostic Rust crate** (`tailor-solver`) that implements Extended Position-Based Dynamics (XPBD) on the GPU via wgpu v29 and WGSL compute shaders.

The solver crate has **zero knowledge of** Handshake kernel internals, PostgreSQL, Tauri, or EventLedger. It exposes a clean `ClothSolver` trait boundary so `handshake_core`'s Tailor module (`src/tailor/`) can call it via a `SandboxAdapter` wrapper. This is the same pattern as the atelier domain adding domain logic without modifying kernel internals.

**Marvelous Designer feature parity targets for this topic:**
- Particle Distance parameter (mesh resolution control, 0.8 mm to 700 mm)
- Simulation modes: Fitting (Accurate Fabric) vs Animation (Stable)
- Substep count and iteration count controls
- GPU Simulation (MD uses proprietary CUDA; hsk uses WGSL/wgpu cross-platform)
- All physical constraint types: stretch weft/warp, shear, bend weft/warp, seam distance, volume/pressure, tack/stitch point, collision response, friction
- Self-collision with optional precise mode
- Anisotropic fabric model (MD moat: weft/warp/shear split)
- Keyframeable physical properties (MD 2025.2 moat: per-step parameter upload to GPU)

---

### [T-CLOTH-SOLVER.xpbd-algorithm] XPBD Algorithm: Core Loop

**Source: Macklin, Müller, Chentanez — "XPBD: Position-Based Simulation of Compliant Constrained Dynamics" (MIG 2016)**

XPBD extends PBD by introducing **compliance** (α, inverse stiffness) and **Lagrange multipliers** (λ) as persistent per-constraint state. This makes constraint stiffness independent of substep size — a prerequisite for stable keyframeable material parameters.

#### [T-CLOTH-SOLVER.xpbd-algorithm.loop] Main Substep Loop

```
for each frame (dt_frame):
    # External forces
    for each particle i:
        v[i] += dt_sub * w[i] * f_ext[i]   # gravity, wind
        x_pred[i] = x[i] + dt_sub * v[i]   # predict position

    # Initialize Lagrange multipliers
    for each constraint c:
        lambda[c] = 0.0

    # Substep loop (n_substeps per frame)
    for sub in 0..n_substeps:
        dt_sub = dt_frame / n_substeps
        alpha_tilde = compliance / (dt_sub * dt_sub)   # scaled compliance

        # Constraint solve (n_iters iterations per substep)
        for iter in 0..n_iters:
            for each constraint c (in color-partition order):
                C = constraint_value(c, x_pred)
                grad_C = constraint_gradient(c, x_pred)
                w_sum = sum(w[i] * |grad_C[i]|^2 for i in c.particles)
                delta_lambda = -(C + alpha_tilde * lambda[c]) / (w_sum + alpha_tilde)
                lambda[c] += delta_lambda
                for each particle i in c.particles:
                    x_pred[i] += w[i] * grad_C[i] * delta_lambda

        # Collision detection and response (after constraint solve per substep)
        detect_and_resolve_collisions(x_pred)

        # Velocity update from position change
        for each particle i:
            v[i] = (x_pred[i] - x[i]) / dt_sub
            x[i] = x_pred[i]

    # Damping
    apply_velocity_damping(v, damping_coeff)
```

**Key properties:**
- `compliance = 0.0` → rigid constraint (inextensible cloth)
- `compliance > 0.0` → elastic; larger α = softer
- Substeps, not iterations, are the primary convergence lever for cloth (per "Small Steps in Physics Simulation", Macklin et al. 2019)
- MGPBD (SIGGRAPH 2025) shows that standard XPBD Gauss-Seidel stalls on high-frequency/high-stiffness cloth at 300+ iterations; the Chebyshev smoother variant converges to 10⁻⁴ accuracy where XPBD stalls — noted as a future upgrade path

#### [T-CLOTH-SOLVER.xpbd-algorithm.compliance-table] Compliance → Marvelous Designer Property Mapping

| MD Property | XPBD Compliance (α) | Direction |
|---|---|---|
| Stretch Weft | `α_stretch_u` | U (weft) direction |
| Stretch Warp | `α_stretch_v` | V (warp) direction |
| Shear | `α_shear` | Diagonal / shear |
| Bending Weft | `α_bend_u` | Weft fold axis |
| Bending Warp | `α_bend_v` | Warp fold axis |
| Solidify | lerp(α_rigid, α_soft, solidify) | Isotropic stiffening |
| Pressure | inflation_target | Volume constraint |

---

### [T-CLOTH-SOLVER.constraint-types] Constraint Types

#### [T-CLOTH-SOLVER.constraint-types.stretch] Anisotropic Stretch and Shear (MOAT-3)

Standard XPBD uses isotropic distance constraints: `C(x1,x2) = |x2 - x1| - L0`. For Marvelous Designer parity, the solver needs **separate compliance values per fabric grain direction** — weft (U), warp (V), and shear (UV diagonal).

The anisotropic formulation uses Green strain in Voigt notation. For a triangle with rest-frame edge vectors `(du, dv)` and deformed vectors `(eu, ev)`:

```
C_u(x) = |eu| - |du|             # weft stretch
C_v(x) = |ev| - |dv|             # warp stretch
C_uv(x) = eu·ev / (|eu|·|ev|)   # shear angle
```

Each carries its own compliance `(α_u, α_v, α_uv)` mapped directly from MD Stretch-Weft, Stretch-Warp, Shear parameters.

**OSS reference:** `ccincotti3/webgpu_cloth_simulator` implements distance + angular bending + self-collision in WGSL. `jspdown/cloth` implements the "small steps" substep variant with parallel Gauss-Seidel. Neither implements anisotropic weft/warp split — this is the moat feature that must be hand-built.

**2025 ACM SIGGRAPH MIG paper** ("XPBD Simulation of Constitutive Materials with Exponential Strain Tensor", dl.acm.org/doi/10.1145/3769047.3769050) formalizes anisotropic XPBD using orthotropic Young's moduli with weft and warp subscripts; this paper is the direct mathematical reference for the weft/warp constraint formulation.

#### [T-CLOTH-SOLVER.constraint-types.bend] Dihedral Bending (Weft/Warp)

Dihedral (angular) bending constraints operate on four particles forming two adjacent triangles sharing an edge:

```
C_bend(x1,x2,x3,x4) = acos( n1·n2 / (|n1|·|n2|) ) - θ_rest
```

where `n1`, `n2` are the face normals of the two triangles, and `θ_rest` is the rest dihedral angle.

For anisotropic bending (Bending-Weft vs Bending-Warp), the compliance `α_bend` is assigned based on whether the shared edge runs along the U or V grain direction of the UV layout.

**Buckling Ratio / Buckling Stiffness (MD):** Implemented as a non-linear compliance ramp: at small angles use `α_bend_soft`, at angles exceeding a threshold switch to `α_bend_stiff`. The ratio parameter controls the blend threshold, and stiffness controls the stiff-side α value.

**OSS reference:** `ccincotti3/webgpu_cloth_simulator` implements isometric bending and angular bending in WGSL; the `InteractiveComputerGraphics/PositionBasedDynamics` (deepwiki reference) documents PbdDihedralConstraint with the acos formulation.

#### [T-CLOTH-SOLVER.constraint-types.seam] Seam / Sewing Distance Constraints (MOAT-2)

Each seam is a set of edge-pair distance constraints with optional **ratio weighting** for gathering (MD's 1:N and M:N sewing).

```
C_seam(x_a, x_b, ratio) = |x_b - x_a| - L0 * ratio
```

For 1:N gathering: a single warp edge `x_a..x_a'` is sewn to N weft edges `x_b1..x_b2..x_bN`. The rest length `L0` is scaled by `1/N` on the weft side. Each seam definition is stored as a `SeamConstraintBlock` struct (see data layout below) uploaded to the GPU as a storage buffer.

#### [T-CLOTH-SOLVER.constraint-types.volume] Volume / Pressure Constraint

For inflatable objects (MD Pressure property):

```
C_vol(x1..xN) = V(mesh) - V_target
```

V_target grows when Pressure > 0. This is a soft global constraint; compliance is large (very soft). MD 2025.2 made Pressure keyframeable — implemented in the solver by uploading a new `V_target` to the GPU each substep.

#### [T-CLOTH-SOLVER.constraint-types.stitch] Stitch / Tack Point Constraints

Point constraints between trim attachment points and garment particles:

```
C_tack(x_garment, x_trim_attachment) = |x_garment - x_trim_attachment|
```

MD 2025.2 made tack strength keyframeable (animated stitching/unstitching). In the solver, tack compliance is a per-step uploaded parameter.

#### [T-CLOTH-SOLVER.constraint-types.collision] Collision Response

Two collision types:
1. **Body-particle collision**: each cloth particle vs avatar capsule proxies (built from Rapier/parry rigid bodies). Implemented as a one-sided distance constraint `C(x_p, surface) = SDF(x_p) - r` where `r` is collision thickness.
2. **Self-collision**: cloth particle vs cloth triangle. Covered in `[T-CLOTH-SOLVER.self-collision]`.

---

### [T-CLOTH-SOLVER.gpu-architecture] GPU Architecture: WGSL Compute Shaders via wgpu

#### [T-CLOTH-SOLVER.gpu-architecture.crate-deps] Standalone Crate Dependencies

```toml
# tailor-solver/Cargo.toml
[package]
name = "tailor-solver"
version = "0.1.0"
edition = "2021"

[dependencies]
wgpu = "29"          # cross-platform GPU: Vulkan/Metal/DX12/WebGPU
bytemuck = { version = "1", features = ["derive"] }
encase = { version = "0.8", features = ["glam"] }
glam = "0.29"        # math (same as xpbdrs, Rapier post-0.32)
parry3d = "0.17"     # rigid body collision proxy shapes (capsule, trimesh)
thiserror = "2"
tracing = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Optional CUDA fast path
[features]
cuda = ["cubecl"]
[dependencies.cubecl]
version = "0.5"
optional = true

[build-dependencies]
wgsl_to_wgpu = "0.15"   # build.rs: typesafe Rust/WGSL bind-group structs

# Bevy ONLY in examples/testbed — never in solver lib
[[example]]
name = "bevy_testbed"
required-features = ["bevy-testbed"]
```

wgpu v29.0.3 (2026-05-02) supports: Vulkan (Windows/Linux), Metal (macOS/iOS), DX12 (Windows), OpenGL/GLES best-effort, WebGPU/WASM. WGSL is always supported natively; Naga translates to SPIR-V → MSL/HLSL/GLSL for non-WebGPU backends. No CUDA dependency by default — the `cuda` feature gates a CubeCL fast path.

`wgsl_to_wgpu` (build-time) parses WGSL via naga and generates Rust structs with bytemuck/encase derives and compile-time layout assertions, eliminating runtime bind-group alignment bugs.

#### [T-CLOTH-SOLVER.gpu-architecture.backend-matrix] wgpu Backend Matrix

| Platform | Primary Backend | Fallback |
|---|---|---|
| Windows | Vulkan, DX12 | OpenGL |
| Linux | Vulkan | OpenGL |
| macOS | Metal | — |
| iOS | Metal | — |
| WebAssembly | WebGPU | WebGL2 |

For the Handshake local-first desktop target (Windows primary), Vulkan and DX12 are both first-class. The solver must work on all three without branching — WGSL cross-compilation via Naga handles this.

#### [T-CLOTH-SOLVER.gpu-architecture.data-layout] GPU Data Layout

All particle and constraint data lives in wgpu `BufferBindingType::Storage` buffers. Structs use `bytemuck::Pod` + `bytemuck::Zeroable` derives. `encase` handles std430 padding automatically for WGSL-visible structs.

```rust
// --- Particle buffer (one entry per particle) ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuParticle {
    pub position:      [f32; 4],  // xyz + w=unused (vec4 alignment)
    pub velocity:      [f32; 4],  // xyz + w=unused
    pub position_pred: [f32; 4],  // predicted position after external forces
    pub delta:         [f32; 4],  // accumulated correction (Jacobi mode)
    pub normal:        [f32; 4],  // vertex normal (xyz + w=unused)
    pub inv_mass:      f32,
    pub uv:            [f32; 2],  // UV for grain direction lookup
    pub _pad:          f32,
}
// Stride: 80 bytes. Buffer size: N_particles * 80

// --- Stretch constraint (one per edge in the mesh) ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuStretchConstraint {
    pub i0:            u32,       // particle index
    pub i1:            u32,       // particle index
    pub rest_length:   f32,
    pub compliance_u:  f32,       // weft compliance (α_u)
    pub compliance_v:  f32,       // warp compliance (α_v)
    pub grain_cos:     f32,       // cos(angle between edge and U-axis) for aniso blend
    pub _pad:          [f32; 2],
}
// Stride: 32 bytes.

// --- Dihedral bending constraint (one per interior edge = two adjacent triangles) ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuBendConstraint {
    pub i0: u32, pub i1: u32, pub i2: u32, pub i3: u32, // 4 particles
    pub rest_angle:    f32,    // θ_rest
    pub compliance_u:  f32,   // weft bend compliance
    pub compliance_v:  f32,   // warp bend compliance
    pub edge_grain:    f32,   // grain direction for aniso selection
    pub buckle_ratio:  f32,   // angle threshold for stiffening
    pub buckle_alpha:  f32,   // stiff-side compliance
    pub _pad:          [f32; 2],
}
// Stride: 48 bytes.

// --- Seam constraint (one per seam edge pair, supports ratio sewing) ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSeamConstraint {
    pub i0:          u32,
    pub i1:          u32,
    pub rest_length: f32,
    pub ratio:       f32,     // 1.0 = 1:1, 0.5 = 1:2 gathering, etc.
    pub compliance:  f32,
    pub _pad:        [f32; 3],
}
// Stride: 32 bytes.

// --- Per-frame parameter UBO (keyframeable material state) ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSimParams {
    pub dt_sub:          f32,
    pub n_particles:     u32,
    pub gravity:         [f32; 3],
    pub damping:         f32,
    pub collision_dist:  f32,
    pub friction:        f32,
    pub pressure_target: f32,   // keyframeable
    pub solidify_blend:  f32,   // keyframeable
    pub shrink_u:        f32,   // keyframeable (Shrinkage Weft)
    pub shrink_v:        f32,   // keyframeable (Shrinkage Warp)
    pub wind:            [f32; 3],
    pub _pad:            f32,
}
```

`GpuSimParams` is uploaded to a `BufferUsages::UNIFORM` buffer at the start of each substep, enabling keyframeable material properties (MD 2025.2 MOAT-4) at zero additional GPU kernel cost — the shader reads params from the UBO.

#### [T-CLOTH-SOLVER.gpu-architecture.shaders] WGSL Compute Shaders

The solver requires six compute passes per substep, dispatched sequentially:

| Pass | Shader | Dispatch |
|---|---|---|
| 1. Predict | `predict.wgsl` | ceil(N/64) workgroups |
| 2. Hash build | `hash_build.wgsl` | ceil(N/64) |
| 3. Stretch solve | `stretch.wgsl` (per color partition) | ceil(constraints/64) |
| 4. Bend solve | `bend.wgsl` (per color partition) | ceil(constraints/64) |
| 5. Seam solve | `seam.wgsl` | ceil(seams/64) |
| 6. Self-collision | `self_collide.wgsl` | ceil(N/64) |
| 7. Body collision | `body_collide.wgsl` | ceil(N/64) |
| 8. Velocity update | `velocity.wgsl` | ceil(N/64) |

Passes 3 and 4 run multiple times (once per color partition), with the constraint index range re-parameterized via a push constant or small UBO update between dispatches.

**Shader workgroup size:** 64 — the wgpu best practice for 1D physics dispatches (matches warp/wave size on AMD/NVIDIA for coalesced memory access).

**Predict shader (predict.wgsl):**
```wgsl
@group(0) @binding(0) var<storage, read_write> particles: array<GpuParticle>;
@group(0) @binding(1) var<uniform>             params:    GpuSimParams;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if i >= params.n_particles { return; }
    var p = particles[i];
    // External forces: gravity + wind
    let f_ext = vec3(params.gravity) + vec3(params.wind) * (1.0 - p.inv_mass);
    p.velocity += f_ext * params.dt_sub;
    p.position_pred = p.position + p.velocity * params.dt_sub;
    // Apply shrinkage: scale prediction toward centroid
    p.position_pred *= vec4(
        vec3(1.0) - vec3(params.shrink_u, params.shrink_v, 0.0) * params.dt_sub,
        1.0
    );
    particles[i] = p;
}
```

**Stretch constraint shader (stretch.wgsl) — Jacobi delta accumulation:**
```wgsl
// Jacobi: read predicted positions, write deltas. No atomics on positions.
// Two passes: (a) compute delta per constraint, (b) apply averaged deltas.
@group(0) @binding(0) var<storage, read>       positions: array<GpuParticle>;
@group(0) @binding(1) var<storage, read>       constraints: array<GpuStretchConstraint>;
@group(0) @binding(2) var<storage, read_write> deltas:    array<vec4<f32>>; // per-particle sum
@group(0) @binding(3) var<storage, read_write> delta_cnt: array<u32>;       // per-particle count
@group(0) @binding(4) var<uniform>             params:    GpuSimParams;
@group(0) @binding(5) var<storage, read_write> lambdas:   array<f32>; // persistent λ

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ci = gid.x;
    if ci >= arrayLength(&constraints) { return; }

    let c = constraints[ci];
    let p0 = positions[c.i0].position_pred.xyz;
    let p1 = positions[c.i1].position_pred.xyz;
    let diff = p1 - p0;
    let dist = length(diff);
    if dist < 1e-6 { return; }

    // Anisotropic compliance blend using grain direction
    let cos_g = c.grain_cos;
    let alpha = mix(c.compliance_u, c.compliance_v, abs(cos_g));
    let alpha_tilde = alpha / (params.dt_sub * params.dt_sub);

    let C = dist - c.rest_length;
    let w0 = positions[c.i0].inv_mass;
    let w1 = positions[c.i1].inv_mass;
    let w_sum = w0 + w1;
    if w_sum < 1e-8 { return; }

    let delta_lambda = -(C + alpha_tilde * lambdas[ci]) / (w_sum + alpha_tilde);
    lambdas[ci] += delta_lambda;

    let n = diff / dist;
    let dx0 = -w0 * n * delta_lambda;
    let dx1 =  w1 * n * delta_lambda;

    // Accumulate (Jacobi: atomic add to f32 in vec4 not supported in WGSL baseline;
    // use constraint graph coloring instead — same-color constraints share no particles,
    // so direct write to deltas[i] is race-free within a color partition dispatch)
    deltas[c.i0] += vec4(dx0, 0.0);
    deltas[c.i1] += vec4(dx1, 0.0);
    delta_cnt[c.i0] += 1u;
    delta_cnt[c.i1] += 1u;
}
```

> Note on atomics: WGSL does support `atomicAdd` on `atomic<i32>` and `atomic<u32>`, but NOT on `f32`. For float delta accumulation, **constraint graph coloring** (same-color constraints share no particles) is the correct GPU-portable approach — used by both `ccincotti3/webgpu_cloth_simulator` and `jspdown/cloth`. The Velvet CUDA approach (Jacobi with averaged deltas in a separate apply pass) is an alternative that avoids atomics entirely at the cost of an extra buffer read pass.

#### [T-CLOTH-SOLVER.gpu-architecture.parallelism] Parallelism Strategy: Constraint Graph Coloring

Both reference implementations (`ccincotti3/webgpu_cloth_simulator`, `jspdown/cloth`) use **parallel Gauss-Seidel via constraint graph coloring**:

1. **Build constraint graph** (CPU, once at garment load): particles are nodes, constraints are edges.
2. **Color the graph** (greedy graph coloring, CPU): assign colors `{0, 1, 2, ...}` so no two same-color constraints share a particle.
3. **Store constraints sorted by color partition** in the GPU storage buffer.
4. **Per substep iteration**: dispatch one compute pass per color; within each dispatch, all constraints in that color solve in parallel with no write races.

Typical cloth mesh graph coloring produces 4–12 colors. This gives 4–12 sequential GPU dispatches per iteration, each fully parallel. This is more GPU-efficient than atomic scatter (no float atomics in WGSL baseline) and more correct than naive Jacobi averaging.

**2025 advance:** "Parallel block Neo-Hookean XPBD using graph clustering" (ACM C&G 2022, ref: dl.acm.org/doi/10.1016/j.cag.2022.10.009) groups highly dependent constraints into supernodes before coloring, reducing color count and increasing parallelism. Applicable as a future optimization pass.

**MGPBD alternative (SIGGRAPH 2025):** For very high-resolution or very stiff cloth, standard XPBD Gauss-Seidel stalls on low-frequency modes. MGPBD (arxiv.org/abs/2505.13390, github.com/chunleili/mgpbd) uses a multigrid preconditioned conjugate gradient with Chebyshev smoother — converges where 300 XPBD iterations stall. The WGSL crate should expose a `SolverMode::Xpbd` / `SolverMode::Multigrid` toggle, with MGPBD as a later optional solver backend behind a feature flag.

---

### [T-CLOTH-SOLVER.self-collision] Self-Collision Detection

Self-collision is the most expensive cloth pass. Two algorithms are layered:

#### [T-CLOTH-SOLVER.self-collision.spatial-hash] Broad Phase: Spatial Hashing

Based on PSCC ("Parallel Self-Collision Culling with Spatial Hashing on GPUs", Tang et al., ACM CGIT 2018, dl.acm.org/doi/10.1145/3203188) and Velvet's implementation:

1. **Build spatial hash** (GPU compute, `hash_build.wgsl`):
   - Each particle is hashed to a cell `(floor(x/cell_size), floor(y/cell_size), floor(z/cell_size))`.
   - Cell size = 2× collision thickness.
   - Hash: `(i*73856093 ^ j*19349663 ^ k*83492791) % table_size`.
   - Sort particles by hash key (GPU radix sort — in Rust/wgpu, use a wgpu-side radix sort crate or port the CUB approach; Velvet replaced `thrust::sort_by_key` with `cub::DeviceRadixSort` for ~50% speedup).
2. **Build cell start/end index array** (prefix sum, GPU compute).
3. **Query neighbors** per particle: check own cell + 26 adjacent cells.

Velvet's optimization: **cache neighbors across substep iterations** via an `interleaved_hash` reuse parameter. Re-hash every N substeps (typically N=4–8); between re-hashes, reuse the neighbor list from the previous hash. This amortizes the hash rebuild cost.

**WGSL limitation:** WGSL has no built-in radix sort. Options:
- Implement a GPU prefix-sum + counting sort in WGSL (straightforward, correct).
- Use a Rust-side wgpu compute crate for sorting (e.g. `wgpu-sort` pattern).
- Accept O(N log N) fallback for initial version; optimize later.

#### [T-CLOTH-SOLVER.self-collision.narrow-phase] Narrow Phase: Curvature Culling

"Efficient Self-Collision Culling for Real-Time Cloth Simulation Using Discrete Curvature Analysis" (April 2026, doi.org/10.3390/math14091504) introduces a **resolution-independent discrete curvature metric** via the h2-normalized Laplace-Beltrami operator to skip collision checks on flat cloth regions. Integration is with XPBD dihedral bending constraints directly — the curvature estimate is computed from the same dihedral angles already in the bend constraint buffer.

Flat cloth regions (low curvature) skip the narrow-phase collision test entirely. This is a high-ROI optimization: most cloth surface is flat at any given frame, and the GPU bandwidth savings from skipping flat-region narrow phase tests are significant.

**Initial implementation:** simpler sphere-based per-particle self-collision (like MD's default mode) — check pairs within collision distance using spatial hash neighbors. The curvature culling optimization is flagged as a Phase 2 addition.

---

### [T-CLOTH-SOLVER.determinism] Determinism and Reproducibility

Handshake requires deterministic simulation for EventLedger receipts and sandbox/promotion gate (re-running the simulation must produce the same result for validation).

Requirements:
1. **Fixed substep count and iteration count per simulation run** — no adaptive stepping.
2. **Constraint graph coloring is computed once** at garment load from the mesh topology and stored in the garment authority row; the color partition order is deterministic from mesh connectivity.
3. **GPU dispatch order is deterministic** — passes are always dispatched in the same sequence; no asynchronous ordering.
4. **Random number sources are seeded** — wind turbulence uses a deterministic seed stored in `GpuSimParams`.
5. **Floating-point associativity:** wgpu/WGSL does not guarantee sub-expression reordering across compilers. The constraint graph coloring eliminates data-dependent write races (the primary source of non-determinism), but cross-platform float rounding may differ between Vulkan/Metal/DX12. For the Handshake sandbox/validation flow, determinism is **per-backend** (same result on re-run on the same machine), not cross-platform.
6. **Jacobi delta accumulation ordering:** color-partition dispatch ensures each pass writes to non-overlapping particles — no accumulation order variation within a pass.

The `SolverResult` output includes a `content_hash: [u8; 32]` (SHA-256 of the final position buffer bytes) written to the sandbox `SandboxArtifactBundle`. The `ValidationRunner` in `handshake_core` re-runs the simulation and compares content hashes to confirm determinism.

---

### [T-CLOTH-SOLVER.crate-design] Standalone Rust Crate Design (`tailor-solver`)

#### [T-CLOTH-SOLVER.crate-design.module-layout] Module Layout

```
tailor-solver/
  Cargo.toml
  build.rs                    # wgsl_to_wgpu codegen
  src/
    lib.rs                    # pub trait ClothSolver; pub struct ClothSolverGpu
    solver.rs                 # ClothSolverGpu impl (wgpu device, pipelines)
    mesh.rs                   # GarmentMesh, particle generation, UV layout
    constraints.rs            # constraint graph building, coloring, buffer upload
    material.rs               # FabricMaterial, GpuSimParams mapping
    self_collision.rs         # SpatialHash, neighbor list management
    body_collision.rs         # capsule proxies from parry3d shapes
    types.rs                  # GpuParticle, GpuStretchConstraint, etc. (bytemuck)
    error.rs                  # ClothSolverError (thiserror)
  shaders/
    predict.wgsl
    stretch.wgsl
    bend.wgsl
    seam.wgsl
    self_collide.wgsl
    body_collide.wgsl
    velocity.wgsl
    hash_build.wgsl
  examples/
    bevy_testbed.rs           # Bevy 0.18 + Avian throwaway testbed viewport
  tests/
    determinism.rs            # determinism integration test
    constraint_correctness.rs # unit tests per constraint type
```

#### [T-CLOTH-SOLVER.crate-design.trait-boundary] Public Trait Boundary

```rust
// src/lib.rs

/// The boundary that handshake_core's tailor module talks to.
/// Implementors: ClothSolverGpu (wgpu), ClothSolverCpu (fallback, no GPU).
#[async_trait::async_trait]
pub trait ClothSolver: Send + Sync {
    /// One-time initialization: upload mesh, constraints, and initial positions to GPU.
    async fn load_garment(&mut self, mesh: GarmentMesh, material: FabricMaterial) -> Result<(), ClothSolverError>;

    /// Simulate n_frames with the given params. Returns the final mesh state.
    async fn simulate(
        &mut self,
        n_frames: u32,
        params: SimRunParams,
    ) -> Result<SolverResult, ClothSolverError>;

    /// Update keyframeable material parameters mid-run (MD MOAT-4).
    fn update_params(&mut self, params: MaterialFrameParams);

    /// Unload current garment, free GPU buffers.
    async fn unload(&mut self);

    /// Content hash of the last simulated position buffer (for determinism validation).
    fn last_content_hash(&self) -> Option<[u8; 32]>;
}

/// Parameters constant for an entire simulation run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimRunParams {
    pub n_substeps:  u32,          // substeps per frame
    pub n_iters:     u32,          // constraint solver iterations per substep
    pub dt_frame:    f32,          // seconds per frame (1/fps)
    pub mode:        SimMode,      // Fitting (accurate) vs Animation (stable)
    pub seed:        u64,          // deterministic seed for wind/perturbation
}

/// Keyframeable material state uploaded per substep (MD MOAT-4).
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct MaterialFrameParams {
    pub solidify_blend:  f32,   // 0=soft, 1=rigid
    pub pressure_target: f32,   // inflation volume target
    pub shrink_u:        f32,   // weft shrinkage rate
    pub shrink_v:        f32,   // warp shrinkage rate
    pub tack_compliance: f32,   // stitch point stiffness
}

#[derive(Debug, Clone)]
pub struct SolverResult {
    /// Final particle positions as flat [f32; N*3].
    pub positions:    Vec<f32>,
    /// Final vertex normals.
    pub normals:      Vec<f32>,
    /// UV coordinates (unchanged from input; UV = 2D pattern shape).
    pub uvs:          Vec<f32>,
    /// Face index buffer.
    pub indices:      Vec<u32>,
    /// SHA-256 of positions bytes for determinism validation.
    pub content_hash: [u8; 32],
    /// Frame count simulated.
    pub n_frames:     u32,
    /// Peak GPU memory used (bytes).
    pub gpu_mem_peak: u64,
}
```

#### [T-CLOTH-SOLVER.crate-design.gpu-init] GPU Initialization Pattern

```rust
// src/solver.rs
pub struct ClothSolverGpu {
    device:            wgpu::Device,
    queue:             wgpu::Queue,
    // Compute pipelines (one per shader pass)
    pipeline_predict:  wgpu::ComputePipeline,
    pipeline_stretch:  wgpu::ComputePipeline,
    pipeline_bend:     wgpu::ComputePipeline,
    pipeline_seam:     wgpu::ComputePipeline,
    pipeline_self_col: wgpu::ComputePipeline,
    pipeline_body_col: wgpu::ComputePipeline,
    pipeline_velocity: wgpu::ComputePipeline,
    // GPU buffers (None until load_garment)
    buf_particles:     Option<wgpu::Buffer>,
    buf_stretch:       Option<wgpu::Buffer>,
    buf_bend:          Option<wgpu::Buffer>,
    buf_seam:          Option<wgpu::Buffer>,
    buf_lambdas:       Option<wgpu::Buffer>,
    buf_params:        Option<wgpu::Buffer>,     // uniform
    buf_spatial_hash:  Option<wgpu::Buffer>,
    // Constraint color partitions (CPU-side index ranges)
    stretch_colors:    Vec<(u32, u32)>,          // (start, count) per color
    bend_colors:       Vec<(u32, u32)>,
    n_particles:       u32,
    last_hash:         Option<[u8; 32]>,
}

impl ClothSolverGpu {
    pub async fn new() -> Result<Self, ClothSolverError> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or(ClothSolverError::NoAdapter)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await?;

        // Load WGSL shaders (embedded via include_wgsl! macro)
        let shader_predict = device.create_shader_module(wgpu::include_wgsl!("../shaders/predict.wgsl"));
        // ... (repeat for each shader)

        let pipeline_predict = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("tailor_predict"),
            layout: None,  // auto-layout from shader reflection
            module: &shader_predict,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        // ... (repeat for each pipeline)

        Ok(Self { device, queue, pipeline_predict, /* ... */ })
    }
}
```

#### [T-CLOTH-SOLVER.crate-design.dispatch-loop] Per-Substep Dispatch Loop

```rust
fn dispatch_substep(
    &self,
    encoder: &mut wgpu::CommandEncoder,
    color_range: (u32, u32),
    n_particles: u32,
) {
    // Pass 1: predict
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline_predict);
        pass.set_bind_group(0, &self.bg_predict, &[]);
        pass.dispatch_workgroups((n_particles + 63) / 64, 1, 1);
    }
    // Pass 2: hash build
    { /* ... hash_build pipeline ... */ }

    // Passes 3+4: stretch and bend constraints, one dispatch per color
    for &(start, count) in &self.stretch_colors {
        // update push constant or small UBO with (start, count)
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline_stretch);
        pass.set_bind_group(0, &self.bg_stretch, &[]);
        pass.dispatch_workgroups((count + 63) / 64, 1, 1);
    }
    // (repeat for bend colors)

    // Pass 5: seam constraints
    { /* ... seam pipeline ... */ }
    // Pass 6: self-collision
    { /* ... self_collide pipeline ... */ }
    // Pass 7: body collision (avatar capsules)
    { /* ... body_collide pipeline ... */ }
    // Pass 8: velocity update
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline_velocity);
        pass.set_bind_group(0, &self.bg_velocity, &[]);
        pass.dispatch_workgroups((n_particles + 63) / 64, 1, 1);
    }
}
```

---

### [T-CLOTH-SOLVER.handshake-binding] Binding to Handshake Kernel Primitives

The `tailor-solver` crate is called exclusively from `src/tailor/solver_binding.rs` inside `handshake_core`. This is the only file that imports the crate. No other kernel file touches wgpu or GPU compute.

#### [T-CLOTH-SOLVER.handshake-binding.sandbox-adapter] SandboxAdapter Integration

```rust
// src/tailor/solver_binding.rs
use tailor_solver::{ClothSolver, ClothSolverGpu, SimRunParams, SolverResult};

pub struct TailorSandboxAdapter {
    solver: Arc<Mutex<ClothSolverGpu>>,
}

impl SandboxAdapter for TailorSandboxAdapter {
    fn kind() -> AdapterKind { AdapterKind::Process }

    fn run(
        &self,
        run: &SandboxRunV1,
        workspace: &SandboxWorkspaceRef,
        policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError> {
        // 1. Deserialize garment JSON from run.input_payload
        let garment_mesh: GarmentMesh = serde_json::from_value(run.input_payload.clone())?;
        let sim_params: SimRunParams = serde_json::from_value(run.sim_params.clone())?;

        // 2. Run XPBD solver
        let result: SolverResult = tokio::runtime::Handle::current()
            .block_on(async {
                let mut solver = self.solver.lock().await;
                solver.load_garment(garment_mesh.clone(), garment_mesh.material.clone()).await?;
                solver.simulate(sim_params.n_frames, sim_params).await
            })?;

        // 3. Emit EventLedger receipt for simulation completion
        let event = NewKernelEvent::builder(
            run.task_run_id,
            run.session_run_id,
            KernelEventType::TailorSimRunCompleted,
            KernelActor::System("tailor-solver".into()),
        )
        .aggregate("tailor_simulation_run", &run.run_id.to_string())
        .idempotency_key(&format!("tailor-sim-{}", run.run_id))
        .payload(serde_json::json!({
            "n_frames": result.n_frames,
            "content_hash": hex::encode(result.content_hash),
            "gpu_mem_peak_bytes": result.gpu_mem_peak,
        }))
        .source_component("tailor::solver_binding")
        .build()?;
        // (event written to EventLedger via postgres_pool in the outer call stack)

        // 4. Package artifact bundle
        let artifact = ArtifactPayloadKind::Bundle; // vertex + UV + normal + index buffers
        Ok(AdapterRunOutcome::completed(run.run_id, result.to_artifact_bundle()))
    }
}
```

#### [T-CLOTH-SOLVER.handshake-binding.event-types] New KernelEventType Variants

These must be added to the `KernelEventType` enum in `kernel/mod.rs` and registered in `required_first_slice_events()`:

```rust
// In KernelEventType enum:
TailorGarmentDraftProposed,        // LLM authored a garment JSON via TailorModelAdapter
TailorSimRunRequested,             // sandbox run requested for XPBD simulation
TailorSimRunStarted,               // TailorSandboxAdapter.run() began
TailorSimRunCompleted,             // XPBD solver completed, SolverResult artifact bundled
TailorSimRunRejected,              // solver error or policy denial
TailorGarmentValidated,            // ValidationRunner passed mesh checks
TailorGarmentPromoted,             // PromotionGate accepted garment to authority
TailorGarmentCrdtUpdateRecorded,   // collaborative panel edit recorded via CrdtUpdateRecordV1
```

#### [T-CLOTH-SOLVER.handshake-binding.model-steerability] Model-Steerable Solver Parameters

The `ModelAdapter` surface for the Tailor engine receives garment authoring requests from an LLM (via `TailorModelAdapter`). The LLM outputs a garment JSON that includes simulation parameters:

```json
{
  "garment_type": "blouse",
  "panels": [ ... ],
  "seams": [ ... ],
  "material": {
    "stretch_weft": 0.05,
    "stretch_warp": 0.03,
    "shear": 0.08,
    "bend_weft": 0.002,
    "bend_warp": 0.002,
    "density_g_m2": 120,
    "friction": 0.4,
    "collision_thickness_mm": 2.5
  },
  "sim_params": {
    "n_substeps": 10,
    "n_iters": 5,
    "mode": "Fitting"
  }
}
```

The LLM maps fabric descriptions to compliance values via a `FabricPresetLibrary` table stored in PostgreSQL (`tailor_material_library`). Named presets (cotton, denim, silk, jersey) are authority rows; the LLM selects a preset name and optionally overrides individual parameters. This maps to MD's preset fabric library feature.

---

### [T-CLOTH-SOLVER.risks] Risks and Open Questions

| Risk | Severity | Mitigation |
|---|---|---|
| WGSL float atomics absent — no `atomicAdd<f32>` in WGSL baseline | High | Constraint graph coloring (write races eliminated) is the primary approach; Jacobi apply-pass is fallback |
| Non-determinism across GPU backends (Vulkan vs Metal vs DX12 float rounding) | Medium | Scope determinism guarantee to per-backend; document cross-backend variance; use content hash for same-backend validation |
| GPU radix sort not available in WGSL stdlib — spatial hash sort requires custom implementation | Medium | Start with CPU-side sort + buffer upload for spatial hash; add WGSL counting sort in Phase 2 |
| MGPBD multigrid requires Chebyshev smoother not yet implemented in WGSL | Low (Phase 2) | Start with standard XPBD Gauss-Seidel (color-partition); add MGPBD backend behind feature flag later |
| Anisotropic weft/warp grain direction requires reliable UV → grain angle computation | Medium | UV must be authored with consistent grain axis; validate in garment schema at load time |
| Self-collision is O(N²) worst case even with spatial hash | Medium | Curvature culling (2026 paper) for Phase 2; sphere-mode self-collision for Phase 1 |
| wgpu v29 MSRV 1.87 — may lag Handshake's MSRV policy | Low | Check handshake_core Cargo.toml MSRV; align or gate wgpu crate behind `gpu` feature flag |
| Sandbox process isolation: wgpu device initialization inside a sandboxed process requires GPU device access from within the process tier | Medium | Process-tier sandbox (default) allows GPU access; Hard Isolation (container/microVM) would require GPU passthrough — document this constraint in `SandboxPolicyV1` for the cloth adapter |

---

### [T-CLOTH-SOLVER.sources] Sources

1. **jspdown/cloth** — WebGPU XPBD cloth simulator (TypeScript/WGSL): https://github.com/jspdown/cloth
2. **ccincotti3/webgpu_cloth_simulator** — WebGPU XPBD with distance/bending/self-collision in WGSL: https://github.com/ccincotti3/webgpu_cloth_simulator
3. **vitalight/Velvet** — CUDA XPBD cloth solver, Jacobi+spatial hash architecture: https://github.com/vitalight/Velvet
4. **nikhilr612/xpbdrs** — Pure Rust XPBD crate (edge/volume/bend, glam, no GPU): https://github.com/nikhilr612/xpbdrs
5. **gfx-rs/wgpu** — Cross-platform Rust GPU API, v29.0.3: https://github.com/gfx-rs/wgpu
6. **wgpu docs.rs** — v29 compute types (Device, Queue, Buffer, ComputePipeline): https://docs.rs/wgpu/latest/wgpu/
7. **ScanMountGoat/wgsl_to_wgpu** — Build-time typesafe Rust/WGSL binding generation: https://github.com/ScanMountGoat/wgsl_to_wgpu
8. **tracel-ai/CubeCL** — Multi-platform JIT GPU kernel DSL for Rust (optional CUDA path): https://github.com/tracel-ai/cubecl
9. **Macklin, Müller, Chentanez — XPBD (MIG 2016)** — Core algorithm with Lagrange multiplier compliance: https://matthias-research.github.io/pages/publications/XPBD.pdf
10. **XPBD Simulation of Constitutive Materials with Exponential Strain Tensor (ACM MIG 2025)** — Anisotropic weft/warp/shear XPBD formulation: https://dl.acm.org/doi/10.1145/3769047.3769050
11. **MGPBD: Multigrid Accelerated Global XPBD Solver (SIGGRAPH 2025)** — Convergence limits of standard XPBD, Chebyshev smoother for cloth: https://arxiv.org/abs/2505.13390
12. **MGPBD GitHub** — CUDA/Taichi implementation: https://github.com/chunleili/mgpbd
13. **PSCC: Parallel Self-Collision Culling with Spatial Hashing on GPUs (ACM CGIT 2018)** — GPU spatial hash broad phase for cloth self-collision: https://dl.acm.org/doi/10.1145/3203188
14. **Efficient Self-Collision Culling for Real-Time Cloth Simulation Using Discrete Curvature Analysis (April 2026)** — Curvature-based culling + XPBD dihedral integration: https://doi.org/10.3390/math14091504
15. **Real-Time Cloth Simulation Using WebGPU: Evaluating Limits of High-Resolution (arxiv 2025)** — WebGPU cloth performance benchmarks (640K nodes @ 60fps without collision): https://arxiv.org/html/2507.11794v1
16. **Parallel block Neo-Hookean XPBD using graph clustering (ACM C&G 2022)** — Supernodal constraint graph coloring for improved GPU parallelism: https://dl.acm.org/doi/10.1016/j.cag.2022.10.009
17. **XPBD emergentmind topic page** — XPBD overview and references: https://www.emergentmind.com/topics/extended-position-based-dynamics-xpbd
18. **InteractiveComputerGraphics/PositionBasedDynamics XPBD DeepWiki** — PbdDihedralConstraint and XPBD loop documentation: https://deepwiki.com/InteractiveComputerGraphics/PositionBasedDynamics/2.2-extended-position-based-dynamics-(xpbd)
19. **LearnWebGPU — Compute Pipeline** — WebGPU compute pipeline setup reference (C++, WebGPU-portable): https://eliemichel.github.io/LearnWebGPU/basic-compute/compute-pipeline.html
20. **High-Performance GPGPU with Rust and wgpu (DEV.to)** — Rust wgpu compute pipeline, buffer, bind group, dispatch example: https://dev.to/jaysmito101/high-performance-gpgpu-with-rust-and-wgpu-4l9i
21. **WebGPU Compute Shaders Explained (Medium 2026)** — Workgroup/thread/dispatch mental model: https://medium.com/@osebeckley/webgpu-compute-shaders-explained-a-mental-model-for-workgroups-threads-and-dispatch-eaefcd80266a
22. **Efficient GPU Cloth Simulation with Non-distance Barriers and Subspace Reuse (arxiv 2024)** — Alternative GPU cloth solver (barrier-based, not XPBD): https://arxiv.org/pdf/2403.19272
23. **Real-Time Skirt Simulation with XPBD on GPU (Sueray Wang 2025)** — NVIDIA Warp XPBD + Jacobi GPU cloth, body collision with capsule proxies: https://sueraywang.github.io/project/xpbdmmd/
24. **wgpu crates.io** — Package registry, v29.0.3: https://crates.io/crates/wgpu
25. **WebGPU Shading Language W3C spec** — WGSL language reference: https://www.w3.org/TR/WGSL/
26. **dimforge/rapier** — Rust rigid body engine (collision proxy shapes only, no cloth): https://github.com/dimforge/rapier
27. **dimforge/parry** — Rust collision detection (capsule, convex hull, trimesh shapes for body proxy): https://github.com/dimforge/parry
28. **wgpu issue #5329 — atomicAdd data race in WGSL** — Confirms f32 atomic limitation in WGSL, constraint coloring necessity: https://github.com/gfx-rs/wgpu/issues/5329
