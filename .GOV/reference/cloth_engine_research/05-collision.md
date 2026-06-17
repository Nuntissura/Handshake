---
file_id: cloth-engine-collision
topic_id: T-COLLISION
title: "Collision: Cloth-vs-Body, Self, Multi-Layer, Exaggerated Proportions"
status: draft
depends_on:
  - T-CLOTH-SOLVER
summary: "GPU collision detection and response for cloth-vs-avatar (SDF + capsule proxy), self-collision (spatial hash + curvature culling), multi-layer garments (layer-ordered constraints), and the exaggerated-proportion robustness case (large-bust inter-collider overlap)."
sources: 30
updated_at: "2026-06-17"
---

## [T-COLLISION] Collision: Cloth-vs-Body, Self, Multi-Layer, Exaggerated Proportions

### [T-COLLISION.overview] Overview and MD Feature Mapping

Marvelous Designer's collision system is the deepest part of its production value. It supports:

- **Cloth-vs-avatar collision**: every cloth particle is kept outside the avatar mesh at each substep. MD uses a proprietary internal mesh-SDF baked per pose at simulation time. GPU acceleration (CUDA) was brought to CPU collision-accuracy parity in MD 2024.2.
- **Self-collision**: optional per-garment flag; sphere-proximity default, with a higher-fidelity iterative mode. MD 2026.0 added GPU-accelerated Trim Simulation including trim-cloth collisions.
- **Multi-layer**: cloth layers stack naturally because each garment registers as both a collision emitter and receiver. MD handles cloth-cloth collisions as an extension of self-collision with per-garment thickness offsets.
- **Exaggerated proportions**: MD defaults target realistic proportions. For content with heavily amplified anatomy (very large bust relative to torso), MD users manually add extra collision geometry inside the outfit via dummy rigid trims placed on the body, a known practitioner workaround.

The Tailor engine must match all four. The exaggerated-proportion case is the explicit robustness long-tail named in this topic: it forces the collision system to handle inter-collider overlap (two breast-proxy spheres partially occupying the same volume) and cloth tunneling through a narrow ribcage-to-bust transition.

---

### [T-COLLISION.cloth-vs-body-approaches] Cloth-vs-Body: SDF vs Capsule Proxy vs Mesh

Three approaches exist in the field for cloth-avatar collision. All three are relevant to the Handshake design; the engine should support two of them in a layered strategy.

#### Approach A: Analytical Capsule / Sphere Proxy Hierarchy (game-engine standard)

The avatar body is approximated as a hierarchical set of capsules and spheres assigned to skeleton segments: neck, left/right clavicle, chest, abdomen, pelvis, left/right upper-arm, forearm, thigh, calf, foot, and — critically for the exaggerated-proportion case — separate left-breast and right-breast sub-bones.

Standard game-engine practice (Unity Cloth, Unreal Engine Chaos Cloth, iClone, PhysX 3.x) uses this approach for real-time performance:
- **Unity**: SphereCollider and CapsuleCollider arrays on the Cloth component; conic capsule (two-sphere + connecting cone) for limbs; continuous collision detection against each proxy.
- **Unreal Engine Chaos Cloth**: per-bone capsule list authored in Skeleton editor; inter-cloth collision distance + stiffness as separate properties.
- **iClone**: 32-body budget (capsules count as 2 spheres); convex hull and self-mesh collision explicitly unsupported for soft cloth in the Reallusion runtime.

For Tailor the capsule proxy approach is the **primary avatar collision mode**:
- Runs entirely in WGSL compute; no CPU readback needed per substep.
- Capsule-vs-particle distance is closed-form (no BVH traversal).
- Exaggerated-proportion characters need additional hand-placed breast proxies beyond the standard skeleton capsule set.

XPBD constraint formula for a single capsule collider:

```rust
// For particle p at predicted position x_pred:
// c(x) = distance(x, capsule) - (r_capsule + r_particle + thickness_offset)
// gradient = (x - closest_point_on_capsule).normalize()
// delta_x = -c(x) * gradient / w_p  (mass-weighted, w_p = 1/mass)
// Applied as positional correction in the substep solver loop.
```

#### Approach B: Signed Distance Field (SDF) — pose-baked or shallow-network

A baked SDF volume of the avatar body, re-sampled at each frame or pose change, allows GPU-parallel per-particle collision queries in O(1) per particle via 3D texture lookup. Two field variants exist in the 2024–2026 literature:

**Static/pose-baked SDF** (Velvet architecture, standard in offline solvers): the avatar mesh SDF is baked into a 3D grid (e.g. 64³ or 128³ voxels) once per keyframe or per animation pose transition. The grid is uploaded to GPU as a `texture3d<f32>`. At each substep:

```wgsl
// sdf_query: lookup trilinearly-interpolated signed distance
fn sdf_query(pos: vec3<f32>) -> f32 { /* trilinear sample of 3d texture */ }
fn sdf_normal(pos: vec3<f32>) -> vec3<f32> {
    // central-difference gradient of the sdf texture
    let eps = 0.001;
    return normalize(vec3(
        sdf_query(pos + vec3(eps, 0.0, 0.0)) - sdf_query(pos - vec3(eps, 0.0, 0.0)),
        sdf_query(pos + vec3(0.0, eps, 0.0)) - sdf_query(pos - vec3(0.0, eps, 0.0)),
        sdf_query(pos + vec3(0.0, 0.0, eps)) - sdf_query(pos - vec3(0.0, 0.0, eps)),
    ));
}

// Collision response: push out along gradient if inside body
fn resolve_sdf_collision(x: ptr<function, vec3<f32>>, thickness: f32) {
    let d = sdf_query(*x);
    if d < thickness {
        *x += (thickness - d) * sdf_normal(*x);
    }
}
```

Position correction formula: `x_corrected = x + (δ − sdf(x)) · ∇sdf(x)` where `δ` is the collision thickness margin (default 2–5 mm). This is the same formula used by Velvet for its pre-stabilization SDF pass, run before the main XPBD constraint loop.

**Shallow SDF** (Akar et al., arxiv 2411.06719, 2024): partitions the animated body into per-joint regions, each modelled by a tiny neural network (~2 hidden layers) that predicts local signed distance from joint-relative coordinates. At inference the per-joint SDFs are stitched together via a boundary-aware weighting. Real-time GPU performance; handles skin deformation better than a static baked grid. The extra output flag (interior vs. boundary SDF value) prevents ambiguity in overlapping regions — directly relevant to the inter-breast overlap case.

For the Tailor engine, the SDF approach is the **secondary / high-fidelity collision mode**, used when avatar topology is complex enough that the capsule proxy set cannot adequately cover it without manual tuning (see the exaggerated-proportion case below). Velvet's architecture uses SDF as a pre-stabilization pass before XPBD constraints — that pattern should be adopted here.

#### Approach C: Full Triangle-Mesh Collision (OpenClo.ai spec, 2024)

Some solvers (OpenClo.ai Rust/WASM implementation) use the avatar mesh directly as a triangle-mesh collision target with a `TriangleSpatialHash` (3×3×3 cell neighbourhood query, O(1) per particle). This provides the highest geometric fidelity but is expensive for dense body meshes and requires BVH updates every frame.

For Handshake: full mesh collision is a **future extension**, not the initial target. Capsule proxy + SDF fallback covers the vast majority of production cases.

---

### [T-COLLISION.capsule-proxy-spec] Capsule Proxy Specification (Handshake Design)

The avatar body proxy is described as a `ClothBodyProxy` stored in PostgreSQL as a JSONB column on the `tailor_avatars` table. At runtime the proxy is uploaded to GPU as a flat buffer.

```rust
/// Handshake Tailor engine — tailor-solver crate
/// Capsule: segment from a to b, uniform radius r.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCapsule {
    pub a:  [f32; 4],   // .xyz = endpoint a, .w = radius
    pub b:  [f32; 4],   // .xyz = endpoint b, .w = padding (keep 16-byte aligned)
}

/// Sphere proxy (for breast/bust sub-volumes and joint spheres)
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSphere {
    pub center: [f32; 4],  // .xyz = center, .w = radius
}

/// Full body proxy: fixed-size arrays sized for WGSL bind group.
/// Max 32 capsules + 16 spheres covers standard skeleton plus
/// 2 breast-proxy spheres per side (4 extra) + rib cage sphere.
pub struct ClothBodyProxy {
    pub capsules:      Vec<GpuCapsule>,  // max 32
    pub spheres:       Vec<GpuSphere>,   // max 16
    pub thickness_mm:  f32,              // global collision offset
}
```

```wgsl
// tailor-solver/shaders/collision_body.wgsl
struct GpuCapsule { a: vec4<f32>, b: vec4<f32> }
struct GpuSphere  { center: vec4<f32> }

@group(1) @binding(0) var<storage, read> capsules: array<GpuCapsule>;
@group(1) @binding(1) var<storage, read> spheres:  array<GpuSphere>;
@group(1) @binding(2) var<uniform>       collision_params: CollisionParams;

struct CollisionParams {
    num_capsules:   u32,
    num_spheres:    u32,
    thickness:      f32,
    _pad:           f32,
}

fn closest_point_on_segment(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / dot(ab, ab), 0.0, 1.0);
    return a + t * ab;
}

fn resolve_capsule(pos: ptr<function, vec3<f32>>, cap: GpuCapsule, thickness: f32) {
    let cp = closest_point_on_segment(*pos, cap.a.xyz, cap.b.xyz);
    let r  = cap.a.w + thickness;
    let d  = *pos - cp;
    let len = length(d);
    if len < r {
        *pos = cp + d * (r / max(len, 1e-6));
    }
}

fn resolve_sphere(pos: ptr<function, vec3<f32>>, sph: GpuSphere, thickness: f32) {
    let d   = *pos - sph.center.xyz;
    let r   = sph.center.w + thickness;
    let len = length(d);
    if len < r {
        *pos = sph.center.xyz + d * (r / max(len, 1e-6));
    }
}

@compute @workgroup_size(64)
fn collision_body_pass(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let idx = gid.x;
    if idx >= arrayLength(&positions) { return; }
    var pos = positions[idx].xyz;
    let thickness = collision_params.thickness;

    for (var i = 0u; i < collision_params.num_capsules; i++) {
        resolve_capsule(&pos, capsules[i], thickness);
    }
    for (var i = 0u; i < collision_params.num_spheres; i++) {
        resolve_sphere(&pos, spheres[i], thickness);
    }

    positions[idx] = vec4(pos, positions[idx].w);
}
```

This shader runs as a **pre-constraint pass** within each XPBD substep, before stretch and bend constraints are solved. Following Velvet's architecture, this prevents large velocity imparting from fast-moving collision objects.

---

### [T-COLLISION.self-collision-spatial-hash] Self-Collision: Spatial Hash GPU Architecture

Self-collision is the dominant computational bottleneck in GPU cloth simulation (Efficient Self-Collision Culling paper, MDPI Mathematics 2026). The Tailor solver uses a two-phase approach: **curvature culling** (cheap, GPU-parallel pre-filter) then **spatial hash query** (narrow-phase).

#### Phase 1: Discrete Curvature Culling (2026 technique)

From the April 2026 paper (MDPI Mathematics 14(9) 1504): a resolution-independent discrete curvature metric derived from the h²-normalized Laplace-Beltrami operator identifies flat cloth regions that are geometrically collision-inactive. These regions are pruned before the spatial hash is built. The curvature metric is cheap to compute in a GPU compute pass over the triangle mesh:

```wgsl
// Curvature pre-filter: mark which particles are in high-curvature regions
// (collision candidates) vs flat regions (skip self-collision).
@compute @workgroup_size(64)
fn curvature_cull_pass(@builtin(global_invocation_id) gid: vec3<u32>) {
    let vi = gid.x;
    // Laplace-Beltrami discrete curvature approximation:
    // H(v) = (1 / (2A)) * sum_j (cot(alpha_j) + cot(beta_j)) * (v - v_j)
    // |H(v)| > curvature_threshold => active; else => inactive (skip)
    let curvature = compute_mean_curvature(vi); // implemented per-vertex
    active_flags[vi] = select(0u, 1u, curvature > params.curvature_threshold);
}
```

This reduces the number of particles entering the spatial hash by 40–70% for typical garments (most cloth surface is locally flat).

#### Phase 2: Spatial Hash Construction and Query (PSCC + Carmen Cincotti approach)

The spatial hash is built over active particles only. The hash function from Matthias Müller (used in xpbd self-collision):

```wgsl
fn hash_coords(xi: i32, yi: i32, zi: i32, table_size: u32) -> u32 {
    let h = (u32(xi) * 92837111u) ^ (u32(yi) * 689287499u) ^ (u32(zi) * 283923481u);
    return h % table_size;
}

fn cell_coord(v: f32, spacing: f32) -> i32 {
    return i32(floor(v / spacing));
}
```

The CPU-side (Rust) construction follows the four-step algorithm:
1. Count particles per cell into `count_buffer`.
2. Prefix-sum `count_buffer` to produce `cell_start` array.
3. Scatter particle indices into `particle_list` using decremented counts.
4. Upload both arrays to GPU storage buffers for query.

Cell size is set to `2 * r_particle` (twice the particle collision radius), where `r_particle` is derived from the cloth particle distance setting (MD equivalent: `particle_distance / 2`).

#### Phase 3: Collision Constraint Resolution

For each active particle `i`, query the 27 neighbouring cells (3×3×3 neighbourhood). For each candidate particle `j`:
- Compute `d = x_i - x_j`
- If `|d| < 2 * r_particle`: apply XPBD distance constraint correction with compliance `alpha_self_collision`.
- Apply velocity-level friction damping as in Carmen Cincotti's formulation:
  `dx_i += d * (factor * delta_v) * h` where `factor` is a damping scalar (0–1).

GPU parallelism strategy: **Jacobi iteration** (Velvet approach) with delta accumulation. Each particle writes its Δx to a separate `delta_buffer` using `atomicAdd`; the main position buffer is updated after all constraints for the substep are collected. This avoids race conditions without constraint graph coloring, at the cost of one extra read-back pass per substep.

Alternative: **Constraint graph coloring** (ccincotti3/jspdown approach) — partition self-collision constraint pairs into independent colour classes and solve each colour in parallel. Lower memory pressure but requires a preprocessing pass when cloth topology changes.

Recommendation: Start with Jacobi + atomics (simpler WGSL, lower preprocessing overhead). Add graph coloring as an optional path for high-performance mode.

---

### [T-COLLISION.multi-layer] Multi-Layer Garment Collision

Multi-layer garment collision (shirt over bra, jacket over shirt, etc.) extends self-collision to inter-garment particle pairs. The key algorithmic requirements are:

1. **Layer ordering**: establish a garment layer index (0 = innermost/bra, 1 = shirt, 2 = jacket, …). An outer garment's particles must stay outside all inner garment particles plus their thickness offset. Directional: inner does not need to stay outside outer.
2. **Rest-length offset**: each inter-layer distance constraint has a positive rest length equal to the sum of both garments' collision thickness values. Unlike self-collision (rest length = 0 approaching), inter-layer rest length is non-zero.
3. **CIPC / layer-sequential fitting** (2025 research pattern): fit inner garments first (bottom to top), then refit outer layers using the already-positioned inner layers as collision geometry. This avoids the need for a fully-coupled multi-body solver.

From the Multi-Layer XPBD Solver paper (Mercier-Aubin & Kry, SCA 2024, CGF 43(8)):
- Layers are identified by current strain rate: highly strained regions treated as elastic; low-strain regions temporarily rigidified.
- Rigidified bodies provide fast inter-layer coupling by propagating impacts across distant vertices within a layer without solving all constraints.
- The rigid/elastic decomposition is automatically regenerated each substep based on per-triangle strain thresholds.

For Handshake, the practical implementation:

```rust
/// Postgres schema: each garment row carries a layer_index (0 = innermost).
/// The solver receives a GarmentLayerSet in simulation config.
pub struct GarmentLayerSet {
    pub garments: Vec<GarmentSimData>,  // sorted ascending by layer_index
}

/// WGSL collision_multilayer.wgsl:
/// For each outer garment particle, query inner garment spatial hash.
/// Apply inter-layer distance constraint with rest = (t_outer + t_inner).
```

Inter-layer collision is resolved in a separate compute pass **after** body collision and after intra-garment constraints, once per substep. The outer garment's spatial hash uses the same structure as self-collision.

For ContourCraft-style intersection recovery (SIGGRAPH 2024): a post-sim intersection-contour loss can optionally be used as a validation-gate check (not a runtime constraint) — if inter-layer penetration contour length exceeds a threshold after N substeps, the simulation is flagged for operator review before promotion.

---

### [T-COLLISION.exaggerated-proportions] Exaggerated Proportions: Large-Bust Collision Robustness

This is the explicit long-tail robustness case named in the topic brief. The production context: avatar characters with a very large bust (cup size G–K+ or equivalent) on a narrow rib cage. This creates three distinct collision failure modes:

**Failure mode 1: Capsule volume overlap at the inter-breast gap**
With very large breast volumes the left-breast and right-breast proxy spheres or capsules partially overlap near the sternum. A cloth particle in the cleavage gap can be simultaneously pushed outward by both spheres in opposite directions, causing jitter or numerical oscillation.

**Failure mode 2: Cloth tunneling through narrow ribcage-to-bust transition**
At the underside of the bust where it meets the ribcage, the geometry transitions steeply. The standard single-capsule-per-breast proxy fails to represent this concave channel, and cloth particles tunnel through on fast animation frames.

**Failure mode 3: Under-bust crease penetration**
dForce users in the Daz community document that large-bust draping tends to collapse under-bust cloth downward rather than resolving the crease correctly. This is a constraint-ordering failure: the cloth needs to resolve contact along the underside of the breast before stretch constraints propagate the error.

#### Handshake Robustness Design

**Proxy decomposition for large-bust characters:**

```json
{
  "left_breast": [
    { "type": "sphere", "bone": "LeftBreast",   "offset_cm": [0, 0, 0],     "radius_cm": 8.0 },
    { "type": "sphere", "bone": "LeftBreast",   "offset_cm": [0, -4, 1.5],  "radius_cm": 6.0 },
    { "type": "sphere", "bone": "LeftBreastLow","offset_cm": [0, -2, 0],    "radius_cm": 5.0 }
  ],
  "right_breast": [
    { "type": "sphere", "bone": "RightBreast",  "offset_cm": [0, 0, 0],     "radius_cm": 8.0 },
    { "type": "sphere", "bone": "RightBreast",  "offset_cm": [0, -4, 1.5],  "radius_cm": 6.0 },
    { "type": "sphere", "bone": "RightBreastLow","offset_cm": [0, -2, 0],   "radius_cm": 5.0 }
  ],
  "sternum_gap": [
    { "type": "capsule", "from_cm": [-2, 5, 0], "to_cm": [-2, -2, 0], "radius_cm": 1.5 }
  ]
}
```

This over-parameterized multi-sphere decomposition per breast is the industry-standard workaround documented in CC/iClone, Unity, and Unreal pipelines. The sternum capsule explicitly guards the inter-breast gap.

**Overlap jitter fix**: when two spheres overlap, a cloth particle may receive contradictory push corrections in a single substep. Fix:

```wgsl
// In collision_body_pass: accumulate push vectors before applying.
// If two or more sphere corrections point in opposing half-spaces,
// take the one with the largest magnitude (priority to deepest penetration).
var push_acc = vec3(0.0);
var push_magnitude = 0.0;
for each sphere {
    let correction = compute_sphere_correction(pos, sphere);
    if length(correction) > push_magnitude {
        push_magnitude = length(correction);
        push_acc = correction;
    }
}
pos += push_acc;
```

This maximum-magnitude selection (rather than summing all corrections) prevents the jitter that arises when two opposing pushes cancel to zero.

**Under-bust tunneling fix**: run body collision resolution **twice per substep** when the avatar proxy contains breast spheres (detected by presence of `BreastL`/`BreastR` bone tags in the proxy). The first pass handles the main body; the second pass resolves residual penetrations along the under-bust surface. This doubles body collision cost but only for avatars with large-bust proxies, which is a selectable flag in `ClothSimConfig`.

**SDF fallback for extreme cases**: when the breast proxy sphere count exceeds 6 and the capsule count exceeds 10 (heuristic for complex topology), fall back to a baked SDF volume covering only the torso–breast region. The SDF is baked from the avatar mesh at resolution 64×64×64 covering the bounding box of the breast region. This is triggered as an automatic mode switch in `ClothBodyProxy::select_collision_mode()`.

**Model-steerable proxy suggestion**: the LLM cloth authoring model can suggest an initial proxy decomposition for a given avatar based on its topology (vertex count in the breast bone sub-mesh region, bone hierarchy, morph target parameters). The suggestion is stored as a `garment_proxy_suggestion` JSONB column in the `tailor_avatars` table, reviewed and confirmed by the operator before simulation runs.

---

### [T-COLLISION.parry-rigid-proxy] Parry for Rigid Body Proxy Construction

Parry3d (dimforge, Apache-2, v0.25.3) is used **on the CPU side** for:
1. Constructing the initial avatar capsule proxy hierarchy from an imported avatar mesh via V-HACD convex decomposition.
2. Collision queries between the avatar proxy and cloth mesh during validation (not runtime).
3. Broad-phase collision culling using the AABB tree (`Bvh`) to pre-filter which cloth triangles could possibly intersect the avatar proxy before the GPU pass runs.

Key parry3d types for the Tailor engine:

```rust
use parry3d::shape::{Capsule, Ball, Compound, SharedShape};
use parry3d::query::{closest_points, PointQuery};
use parry3d::math::{Isometry, Point};

// Build avatar body proxy from imported mesh (CPU, pre-sim):
fn build_avatar_proxy(avatar_mesh: &TriMesh, skeleton: &Skeleton) -> ClothBodyProxy {
    let mut capsules = Vec::new();
    let mut spheres = Vec::new();

    for bone in skeleton.limb_bones() {
        let cap = Capsule::new(
            Point::from(bone.head_ws()),
            Point::from(bone.tail_ws()),
            bone.collision_radius(),
        );
        capsules.push(GpuCapsule::from_parry(&cap));
    }

    // Special breast bones: decompose into 3-sphere per side using VHACD
    for breast_bone in skeleton.breast_bones() {
        let sub_mesh = avatar_mesh.sub_mesh_for_bone(breast_bone);
        let vhacd = VHACDParameters { concavity: 0.005, ..Default::default() };
        let convex_parts = parry3d::transformation::vhacd(
            sub_mesh.vertices(), sub_mesh.indices(), &vhacd
        );
        // Extract sphere approximations from each convex part
        for part in &convex_parts {
            let bounding_sphere = part.local_bounding_sphere();
            spheres.push(GpuSphere::from_parry(&bounding_sphere));
        }
    }

    ClothBodyProxy { capsules, spheres, thickness_mm: 2.5 }
}
```

Parry is **never** used in the real-time substep loop. All runtime collision resolution runs in WGSL compute shaders on GPU. Parry is a CPU-only pre-processing tool for proxy construction and validation.

The 2025 Rapier/Parry update added sparse voxel colliders and a new dynamic BVH (SIMD-accelerated). The BVH is useful for the broad-phase cloth-vs-body AABB culling on CPU, and the dynamic BVH is worth adopting for the offline proxy construction step if avatar body parts are animated during a bake pass.

---

### [T-COLLISION.wgsl-self-collision-shader] WGSL Self-Collision Shader Sketch

```wgsl
// tailor-solver/shaders/self_collision.wgsl
// Two-pass: (1) build spatial hash over active (high-curvature) particles.
//           (2) query hash and apply XPBD distance constraints.

struct SpatialHashParams {
    spacing:      f32,
    table_size:   u32,
    num_particles: u32,
    r_particle:   f32,  // collision radius per particle
    alpha:        f32,  // XPBD compliance for self-collision
    dt_substep:   f32,
}

@group(0) @binding(0) var<storage, read>       positions:    array<vec4<f32>>;
@group(0) @binding(1) var<storage, read_write> pred_pos:     array<vec4<f32>>;
@group(0) @binding(2) var<storage, read>       inv_mass:     array<f32>;
@group(0) @binding(3) var<storage, read>       active_flags: array<u32>;
@group(1) @binding(0) var<storage, read>       cell_start:   array<u32>;
@group(1) @binding(1) var<storage, read>       particle_list:array<u32>;
@group(1) @binding(2) var<storage, read_write> delta_pos:    array<vec4<f32>>;
@group(1) @binding(3) var<storage, read_write> delta_count:  array<u32>;
@group(2) @binding(0) var<uniform>             sc_params:    SpatialHashParams;

fn hash_cell(xi: i32, yi: i32, zi: i32) -> u32 {
    return ((u32(xi) * 92837111u) ^ (u32(yi) * 689287499u) ^ (u32(zi) * 283923481u))
           % sc_params.table_size;
}

@compute @workgroup_size(64)
fn self_collision_query(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if i >= sc_params.num_particles || active_flags[i] == 0u { return; }

    let xi = pred_pos[i];
    let spacing = sc_params.spacing;
    let r2 = 2.0 * sc_params.r_particle;
    let min_dist_sq = r2 * r2;
    let alpha = sc_params.alpha / (sc_params.dt_substep * sc_params.dt_substep);

    let cx = i32(floor(xi.x / spacing));
    let cy = i32(floor(xi.y / spacing));
    let cz = i32(floor(xi.z / spacing));

    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dz = -1; dz <= 1; dz++) {
                let cell_hash = hash_cell(cx + dx, cy + dy, cz + dz);
                let start = cell_start[cell_hash];
                let end   = cell_start[cell_hash + 1u];
                for (var k = start; k < end; k++) {
                    let j = particle_list[k];
                    if j <= i { continue; }  // avoid double counting
                    let xj = pred_pos[j];
                    let d  = xi - xj;
                    let dist_sq = dot(d.xyz, d.xyz);
                    if dist_sq < min_dist_sq && dist_sq > 0.00001 {
                        // XPBD distance constraint
                        let dist  = sqrt(dist_sq);
                        let c     = dist - r2;  // constraint violation (negative = penetrating)
                        let w_i   = inv_mass[i];
                        let w_j   = inv_mass[j];
                        let w_sum = w_i + w_j;
                        if w_sum < 0.000001 { continue; }
                        let lambda = -c / (w_sum + alpha);
                        let n      = d.xyz / dist;
                        // Accumulate deltas (Jacobi: no immediate write to pred_pos)
                        atomicAdd(&delta_pos_x[i], bitcast<u32>(lambda * w_i * n.x));
                        atomicAdd(&delta_pos_y[i], bitcast<u32>(lambda * w_i * n.y));
                        atomicAdd(&delta_pos_z[i], bitcast<u32>(lambda * w_i * n.z));
                        atomicAdd(&delta_pos_x[j], bitcast<u32>(-lambda * w_j * n.x));
                        // ... etc
                        atomicAdd(&delta_count[i], 1u);
                        atomicAdd(&delta_count[j], 1u);
                    }
                }
            }
        }
    }
}
// A second pass averages and applies accumulated deltas to pred_pos.
```

Note: WGSL `atomicAdd` on `f32` requires the `shader-f16` or `atomic_f32` extension which is not universally available in wgpu 0.20/0.29. The practical workaround is to use `atomicAdd` on `i32` with a fixed-point encoding (multiply by 1000, cast to i32, sum, divide back). This is the standard pattern in WebGPU cloth simulators (jspdown, ccincotti3).

---

### [T-COLLISION.kernel-binding] Kernel Primitive Bindings

**PostgreSQL / EventLedger authority:**

Every collision proxy configuration is an authority artifact in Postgres:

```sql
-- Migration: tailor_body_proxies table
CREATE TABLE tailor_body_proxies (
    proxy_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id         UUID NOT NULL REFERENCES tailor_garments(garment_id),
    avatar_id          UUID NOT NULL REFERENCES tailor_avatars(avatar_id),
    proxy_json         JSONB NOT NULL,  -- ClothBodyProxy serialized
    mode               TEXT NOT NULL,   -- 'capsule', 'capsule+sdf', 'sdf'
    breast_proxy_mode  TEXT,            -- 'standard', 'multi-sphere', 'sdf-fallback'
    collision_thickness_mm FLOAT NOT NULL DEFAULT 2.5,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id)
);
```

EventLedger events for collision lifecycle:

```rust
// Tailor event family (event_family.rs)
pub const TAILOR_PROXY_CREATED:      &str = "TAILOR_PROXY_CREATED";
pub const TAILOR_PROXY_UPDATED:      &str = "TAILOR_PROXY_UPDATED";
pub const TAILOR_COLLISION_PASS_RAN: &str = "TAILOR_COLLISION_PASS_RAN";
pub const TAILOR_TUNNELING_DETECTED: &str = "TAILOR_TUNNELING_DETECTED";

// Emitted via NewKernelEvent::builder(task_run_id, session_run_id,
//   KernelEventType::TailorProxyCreated, KernelActor::System("tailor"))
//   .aggregate("tailor_body_proxy", proxy_id)
//   .payload(json!({ "mode": "capsule", "num_capsules": 24, "num_spheres": 6 }))
//   .build()
```

**Sandbox**: the WGSL collision shaders run inside the `TailorSandboxAdapter` (defined in `solver_binding.rs` in `src/tailor/`). The adapter wraps the GPU dispatch call. `SandboxCapability::Device` must be granted in the policy to allow GPU access (wgpu device creation). The adapter declares `AdapterIsolationTier::Process` with `Device` capability.

**Validation gate**: a `CollisionValidationCheck` runs as a `ValidationRunner` step after each sandbox simulation. It checks:
1. No particle has signed distance < `-2 * thickness` into any capsule/sphere (deep penetration).
2. No inter-layer particle pair is closer than `t_inner + t_outer - tolerance`.
3. Self-collision particle pair count < a configurable limit (prevents degenerate mesh explosions).

If any check fails, the `ValidationReport` returns `status: Failed` and the simulation is not promoted.

**CRDT**: collision proxy JSON (`ClothBodyProxy`) is CRDT-tracked as part of the garment document. An operator editing the breast proxy sphere configuration (adding a sphere, adjusting radius) creates a `CrdtUpdateRecordV1` with the diff. Conflict resolution is straightforward (last-writer-wins on the sphere array, since proxy edits are not semantically meaningful to merge).

**Model-lane (LLM steerability)**: the `TailorModelAdapter` can receive a tool call `suggest_collision_proxy` with the avatar topology as context (bone hierarchy, breast morph target magnitude). The model produces a `ClothBodyProxy` JSON proposal. This proposal goes through the sandbox→validation→promotion pipeline before becoming the authority proxy.

---

### [T-COLLISION.risks] Risks and Open Questions

1. **WGSL `atomicAdd` on f32**: wgpu 0.29 (2026) supports `f32` atomics via the `shader-atomic-float` extension on Vulkan and DX12 but not universally on Metal. The fixed-point integer workaround (multiply f32 by 10000, bitcast to u32, atomicAdd) degrades precision for very small corrections. Mitigation: measure precision loss in practice; accept if sub-mm error; add f32 atomic detection at runtime and branch to fixed-point fallback path.

2. **Breast proxy overlap jitter at extreme cup sizes**: the maximum-magnitude correction selection described above works for two opposing spheres but can fail when three or more overlapping spheres surround a particle. Mitigation: limit maximum sphere density; enforce a minimum spacing between breast proxy spheres of `0.5 * max_radius`; alternatively, blend corrections as a weighted average with penetration depth as weight.

3. **SDF baking cost for animated avatars**: re-baking a 64³ SDF grid from avatar mesh at each keyframe on CPU takes O(N_voxels × N_triangles) time naively. For a 10k-triangle body mesh and 64³ grid, this is ~2.6B operations — too slow for interactive use. Mitigation: use an accelerated GPU SDF baker (sphere-tracing or sign-propagation via jump flooding on GPU); or bake only a sparse grid covering the breast+torso region; or adopt the shallow-SDF neural approach (Akar et al. 2024) for fully real-time mode.

4. **Multi-layer tunneling at high animation velocities**: XPBD substep count may be insufficient to catch fast-moving outer garments that tunnel through inner garments between substeps. Mitigation: add continuous collision detection (CCD) as an optional pass for inter-layer pairs with high velocity delta; CCD is expensive but only needed for fast animation (walking→running transitions, jump landing).

5. **Spatial hash table size**: a fixed table_size causes hash collisions when particle count is high (>50k). Mitigation: dynamically resize hash table based on particle count at simulation startup; use `table_size = 2 * num_particles` as standard recommendation.

6. **Validation gate false positives**: the deep-penetration check may fire legitimately during the first few substeps of draping (before cloth settles). Mitigation: skip the collision validation gate for substep-level intermediate states; only run it on the final frame of the draping phase.

---

### [T-COLLISION.sources] Sources

- https://www.mdpi.com/2227-7390/14/9/1504 — Efficient Self-Collision Culling for Real-Time Cloth Simulation Using Discrete Curvature Analysis (April 2026)
- https://dl.acm.org/doi/10.1145/3203188 — PSCC: Parallel Self-Collision Culling with Spatial Hashing on GPUs (ACM CGIT 2018)
- https://carmencincotti.com/2022-11-21/cloth-self-collisions/ — Cloth Self Collisions XPBD tutorial (spatial hash + friction)
- https://carmencincotti.com/2022-10-31/spatial-hash-maps-part-one/ — Spatial Hash Map construction for XPBD
- https://carmencincotti.com/2022-11-07/spatial-hash-maps-part-two/ — Spatial Hash Map query for XPBD
- https://github.com/vitalight/Velvet — Velvet CUDA XPBD engine (SDF pre-stabilization, Jacobi iteration, CUB radix sort spatial hash, neighbor cache)
- https://github.com/ccincotti3/webgpu_cloth_simulator — WebGPU XPBD cloth with self-collision (constraint graph coloring, WGSL)
- https://github.com/jspdown/cloth — WebGPU XPBD cloth (small-step technique, 100% GPU)
- https://arxiv.org/abs/2411.06719 — Shallow SDFs for Kinematic Collision Bodies (Akar et al., November 2024)
- https://arxiv.org/html/2405.09522v1 — ContourCraft: Multi-Layer Garment Intersection Resolution (SIGGRAPH 2024)
- https://dl.acm.org/doi/10.1145/3641519.3657408 — ContourCraft ACM SIGGRAPH 2024 publication
- https://arxiv.org/abs/2403.19272 — Mil2: Efficient GPU Cloth Simulation with Non-distance Barriers and Subspace Reuse (2024)
- https://onlinelibrary.wiley.com/doi/10.1111/cgf.15186 — A Multi-Layer Solver for XPBD (Mercier-Aubin & Kry, SCA 2024)
- https://min-tang.github.io/home/ICloth/ — I-Cloth: Incremental Collision Handling for GPU-Based Interactive Cloth Simulation
- https://mmacklin.com/sdfcontact.pdf — Local Optimization for Robust Signed Distance Field Collision (Macklin)
- https://www.researchgate.net/publication/393771717_Real-Time_Cloth_Simulation_Using_WebGPU_Evaluating_Limits_of_High-Resolution — Real-Time Cloth Simulation Using WebGPU (2025)
- https://arxiv.org/abs/2507.11794 — Real-Time Cloth Simulation Using WebGPU: Evaluating Limits of High-Resolution (2025): 60fps at 640K nodes; 100K cloth vs 100K body triangle collision in real-time
- https://sueraywang.github.io/project/xpbdmmd/ — Real-Time XPBD Skirt Simulation (sphere/cylinder body proxies, GPU)
- https://github.com/sssamuelll/openclo — OpenClo.ai cloth simulation spec (Rust/WASM XPBD, TriangleSpatialHash body collision)
- https://github.com/dimforge/parry — Parry3d collision detection library (capsule, ball, compound, VHACD)
- https://parry.rs/ — Parry documentation
- https://crates.io/crates/parry3d — Parry3d on crates.io
- https://dimforge.com/blog/2026/01/09/the-year-2025-in-dimforge/ — Rapier/Parry 2025 review and 2026 roadmap (dynamic BVH, voxel colliders, wgrapier GPU path)
- https://soupday.github.io/cc_unity_tools/physics.html — CC/iClone Unity Cloth Collider Manager (capsule hierarchy, breast bone setup)
- https://forum.reallusion.com/549912/Dynamic-Collision-Shapes-for-Soft-Cloth-in-iClone — iClone 32-body collision limit, capsule-only for soft cloth
- https://www.daz3d.com/forums/discussion/547231/alternative-ways-of-skinning-the-clothes-breast-cat/p3 — Daz large-bust cloth simulation issues and workarounds
- https://dl.acm.org/doi/10.1145/3658177 — Proxy Asset Generation for Cloth Simulation in Games (ACM TOG 2024)
- https://github.com/ext-sakamoro/ALICE-SDF — ALICE-SDF (wgpu SDF collision, interval arithmetic AABB, cloth support hook)
- https://support.marvelousdesigner.com/hc/en-us/articles/55837641308313-Marvelous-Designer-2026-0-New-Feature-List — MD 2026.0 GPU Trim Simulation feature
- https://support.marvelousdesigner.com/hc/en-us/articles/47358125463321-Simulation-Properties — MD simulation properties: self-collision, substeps, iteration counts
