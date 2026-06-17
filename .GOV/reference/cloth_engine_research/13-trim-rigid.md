---
file_id: cloth-engine-trim-rigid
topic_id: T-TRIM-RIGID
title: "Trims, Accessories, and Cloth-Rigid Coupling"
status: draft
depends_on:
  - T-MD-FEATURES
  - T-CLOTH-SOLVER
  - T-COLLISION
summary: "Mixed cloth-rigid XPBD constraint graph for trims, zippers, buttons, tacks, eyelets, and pattern-to-rigid-body conversion: solver design, WGSL coupling pass, Postgres tailor_* schema, kernel binding, and model-first API. Closes MD MOAT-5."
sources: 22
updated_at: "2026-06-17"
---

## [T-TRIM-RIGID] Trims, Accessories, and Cloth-Rigid Coupling

### [T-TRIM-RIGID.overview] Overview and MD Feature Mapping

Marvelous Designer's trim system is one of its four production moats (MOAT-5 from `[T-MD-FEATURES.moat-summary]`): rigid and semi-rigid 3D objects — buttons, buckles, zippers, eyelets, rivets, hooks — that attach to cloth panels and participate in the XPBD solver as coupled rigid bodies rather than simulated cloth. The trim mesh does not deform; it moves as a single rigid object whose position and orientation are constrained to the cloth surface via tack attachment points. As of MD 2025.2, the constraint between cloth and trim (the tack) became keyframeable, enabling animated stitching and unstitching. As of MD 2026.0, the full trim-plus-cloth collision system became GPU-accelerated (previously CPU-only for trims in 2025.x). MD 2025.2 also added pattern-to-trim conversion: a cloth pattern panel can be baked into a rigid trim inline, enabling armor and hard-surface costume authoring inside the same session and solver state.

Marvelous Designer features this topic covers (from Group 7 of `[T-MD-FEATURES.group-7-trims-accessories]`):

| MD Feature | Version | Difficulty | Handshake design section |
|---|---|---|---|
| Trim import (OBJ/FBX) | legacy | `[D1]` | [T-TRIM-RIGID.trim-asset-model] |
| Glue tool (click-to-place) | legacy | `[D2]` | [T-TRIM-RIGID.tack-data-model] |
| Tack tool (multi-point attachment) | legacy | `[D2]` | [T-TRIM-RIGID.tack-data-model] |
| Stiffness control for trim | legacy | `[D2]` | [T-TRIM-RIGID.trim-physics-params] |
| Trim Simulation with Collision (CPU) | 2024.2 | `[D3]` | [T-TRIM-RIGID.mixed-constraint-graph] |
| GPU-Accelerated Trim Simulation | 2026.0 | `[D4]` | [T-TRIM-RIGID.wgsl-coupling-pass] |
| Pattern-to-Trim conversion | 2025.2 | `[D4]` | [T-TRIM-RIGID.pattern-to-rigid] |
| Two-Way Zippers | 2025.0 | `[D3]` | [T-TRIM-RIGID.zippers] |
| Keyframeable tack strength | 2025.2 | `[D3]` | [T-TRIM-RIGID.keyframeable-tack] |
| Keyframeable trim weight | 2025.2 | `[D3]` | [T-TRIM-RIGID.keyframeable-tack] |
| Default trim library | legacy | `[D1]` | [T-TRIM-RIGID.trim-asset-model] |
| Lacing Tool + eyelets | 2026.0 | `[D3]` | [T-TRIM-RIGID.zippers] |

The implementation gap from OSS XPBD solvers: all OSS GPU cloth solvers (`ccincotti3/webgpu_cloth_simulator`, `jspdown/cloth`, `vitalight/Velvet`, `openxrlab/xrtailor`) are cloth-only and use only **static** collision proxies. None implement a dynamic rigid trim mesh that participates in the solver as a moving rigid body coupled to cloth particles. The Tailor engine must build this from first principles using the Müller/Macklin XPBD rigid body coupling formulation (SCA 2020) as the mathematical reference.

---

### [T-TRIM-RIGID.md-moat-5] MOAT-5: Mixed Cloth-Rigid Simulation Graph

The core moat is the **mixed constraint graph**: a single XPBD solver pass in which:
1. Cloth particles solve standard stretch, bend, and seam constraints.
2. Rigid trim bodies solve positional and rotational coupling constraints against cloth tack points.
3. Rigid-cloth contact constraints prevent trim meshes from penetrating cloth panels.
4. All constraint types share the same substep loop, the same Lagrange multiplier update, and the same GPU dispatch.

Marvelous Designer's trim collision was CPU-only through 2025.x and reached GPU acceleration in 2026.0 (buttons and buckles GPU-accelerated per the 2026.0 release). The Tailor engine targets GPU from the start via WGSL compute.

The key algorithmic reference is "Detailed Rigid Body Simulation with Extended Position Based Dynamics" (Müller, Macklin, Chentanez, Jeschke, Kim — SCA 2020, CGF 39(8)). This paper provides the complete formulation for mixing rigid body quaternion+position DOFs with soft particle position DOFs in a unified XPBD constraint graph, handling contacts, joints, and soft-rigid coupling in one solver pass.

The `InteractiveComputerGraphics/PositionBasedDynamics` library (C++, MIT) includes a concrete `RigidBodyClothCouplingDemo.cpp` showing `addRigidBodyParticleBallJoint` — the ball joint constraint that pins a world-space attachment point on a rigid body to a cloth particle position.

---

### [T-TRIM-RIGID.rigid-body-xpbd] Rigid Body State in the XPBD Solver

A trim rigid body adds six degrees of freedom beyond the cloth particle's three translational DOFs: a world-space centroid position **x** and an orientation quaternion **q**. The Müller 2020 formulation tracks:

```
state per rigid body:
  x: vec3     -- world-space centroid
  q: quat     -- orientation (unit quaternion)
  v: vec3     -- linear velocity
  omega: vec3 -- angular velocity
  mass:  f32  -- body mass (from trim weight property)
  I:     mat3 -- inertia tensor (computed from mesh at load time)
```

Within the XPBD substep, rigid body updates follow the same predict-constrain-integrate pattern as cloth particles:

```
// Predict phase (same as cloth particles)
x_pred = x + dt_sub * v
q_pred = q + 0.5 * dt_sub * quat(0, omega) * q  // quaternion derivative
normalize(q_pred)

// Per constraint: delta_x and delta_q updates
// The correction to a point r_local on the rigid body (world = x + R(q) * r_local):
//   r_world = x + R(q_pred) * r_local
//   delta_x += delta_p            // translational correction
//   delta_q += 0.5 * quat(0, R(q_pred) * cross(r_local, delta_p)) * q_pred
//              (angular update via back-projected lever arm)
//   (divide delta_q by |I^{-1} * r x (r x delta_p)| for mass weighting)

// Velocity update (after all constraints)
v     = (x_pred - x) / dt_sub
omega = 2 * (q_pred_conj * q_delta).xyz / dt_sub
x     = x_pred
q     = q_pred
```

The key identity: a constraint applied at a world-space point **p** on the rigid body induces both a linear correction `delta_x` and an angular correction `delta_q` derived from the cross product of the body-frame lever arm with the constraint force direction. This is the essential difference from a cloth particle constraint (which only corrects `x_pred`).

In the GPU implementation, the effective inverse mass for rigid-to-particle coupling is:

```
w_rigid(r) = 1/mass + dot(r x n, I_inv * (r x n))
```

where `r = R(q) * r_local` is the world-space lever arm from the centroid to the attachment point, `n` is the constraint gradient direction, and `I_inv` is the inverse inertia tensor. This term replaces `w = 1/mass` for rigid body particles.

**Rust GPU type for a trim rigid body:**

```rust
// tailor-solver/src/types.rs
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTrimBody {
    pub pos:         [f32; 4],   // xyz = centroid, w = inv_mass
    pub quat:        [f32; 4],   // xyzw orientation quaternion
    pub vel:         [f32; 4],   // xyz = linear velocity, w = unused
    pub omega:       [f32; 4],   // xyz = angular velocity, w = unused
    pub pos_pred:    [f32; 4],   // predicted centroid after external forces
    pub quat_pred:   [f32; 4],   // predicted orientation
    pub inertia_inv: [f32; 12],  // 3x3 inverse inertia tensor (row-major, std430 padded)
    pub stiffness:   f32,        // trim stiffness blend (0=free, 1=fully rigid kinematic)
    pub trim_index:  u32,        // index into tailor_trims table
    pub _pad:        [f32; 2],
}
// Stride: 128 bytes. Buffer size: N_trims * 128

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTackConstraint {
    pub body_idx:       u32,    // index into GpuTrimBody buffer
    pub particle_idx:   u32,    // index into GpuParticle buffer
    pub r_local:        [f32; 4], // body-frame attachment offset from centroid (xyz, w=unused)
    pub compliance:     f32,    // α = tack compliance (0 = rigid; 1e-4 = elasticated)
    pub strength_scale: f32,    // keyframeable tack strength multiplier [0, 1]
    pub _pad:           [f32; 2],
}
// Stride: 32 bytes.
```

---

### [T-TRIM-RIGID.mixed-constraint-graph] Mixed Soft-Rigid Constraint Graph

The XPBD substep loop is extended with two new constraint passes after the existing stretch/bend/seam passes:

```
Per substep (n_substeps per frame):
  Pass 1: predict.wgsl          -- cloth particles + trim rigid bodies (extended)
  Pass 2..K: stretch.wgsl       -- cloth stretch per color partition
  Pass K+1..M: bend.wgsl        -- cloth bend per color partition
  Pass M+1: seam.wgsl           -- seam distance constraints
  Pass M+2: tack.wgsl           -- NEW: cloth-particle to rigid-body ball joints
  Pass M+3: trim_contact.wgsl   -- NEW: rigid trim mesh vs cloth triangle contact
  Pass M+4: body_collide.wgsl   -- cloth vs avatar capsule proxy
  Pass M+5: self_collide.wgsl   -- cloth self-collision
  Pass M+6: velocity.wgsl       -- velocity update (cloth particles + trim bodies)
```

**Why tack and trim_contact are separate passes:**

The tack pass updates both the cloth particle and the rigid body DOFs. Because different constraints in the same pass may reference the same rigid body or the same cloth particle, **constraint graph coloring is still required** for the tack pass: two tack constraints that share a rigid body or a cloth particle must be in different color classes. In practice, a single button has 2–4 tack points; neighboring buttons do not share particles. Graph coloring for tacks typically produces 2–4 color classes.

The trim_contact pass is a one-sided inequality constraint: it fires only when the rigid trim mesh surface penetrates a cloth triangle. Unlike tack constraints (equality constraints, always active), contact constraints use a `max(0, -C)` form. This is the same structure as the cloth-avatar body collision pass but with the rigid trim mesh as the collision object.

---

### [T-TRIM-RIGID.wgsl-coupling-pass] WGSL Tack Coupling Pass Sketch

```wgsl
// tailor-solver/shaders/tack.wgsl
// Solves cloth-particle ↔ trim-rigid-body ball-joint constraints (tack points).
// Each tack constraint connects one cloth particle to one world-space point on a rigid body.
// One dispatch per tack color partition (same coloring discipline as stretch constraints).

struct GpuTrimBody {
    pos:         vec4<f32>,   // .xyz = centroid, .w = inv_mass
    quat:        vec4<f32>,   // orientation quaternion (xyzw)
    vel:         vec4<f32>,
    omega:       vec4<f32>,
    pos_pred:    vec4<f32>,
    quat_pred:   vec4<f32>,
    inertia_inv: array<vec4<f32>, 3>,  // 3x4 = 3 rows of 3x3 (std430: rows padded to vec4)
    stiffness:   f32,
    trim_index:  u32,
    _pad:        vec2<f32>,
}

struct GpuTackConstraint {
    body_idx:       u32,
    particle_idx:   u32,
    r_local:        vec4<f32>,  // body-frame lever arm offset
    compliance:     f32,
    strength_scale: f32,
    _pad:           vec2<f32>,
}

@group(0) @binding(0) var<storage, read_write> particles:  array<GpuParticle>;
@group(0) @binding(1) var<storage, read_write> bodies:     array<GpuTrimBody>;
@group(0) @binding(2) var<storage, read>       tacks:      array<GpuTackConstraint>;
@group(0) @binding(3) var<storage, read_write> tack_lambdas: array<f32>;
@group(0) @binding(4) var<uniform>             params:     GpuSimParams;

// Rotate a local vector by a unit quaternion
fn quat_rotate(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let qv = q.xyz;
    let qw = q.w;
    return v + 2.0 * qw * cross(qv, v) + 2.0 * cross(qv, cross(qv, v));
}

// Inverse inertia tensor multiply: I_inv * v
fn inertia_inv_mul(body: GpuTrimBody, v: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        dot(body.inertia_inv[0].xyz, v),
        dot(body.inertia_inv[1].xyz, v),
        dot(body.inertia_inv[2].xyz, v),
    );
}

@compute @workgroup_size(64)
fn tack_solve(@builtin(global_invocation_id) gid: vec3<u32>) {
    let ci = gid.x;
    if ci >= arrayLength(&tacks) { return; }

    let t = tacks[ci];

    // Early-out if tack strength is zero (released tack — animatable per MOAT-4)
    if t.strength_scale < 1e-5 { return; }

    var body     = bodies[t.body_idx];
    var particle = particles[t.particle_idx];

    // World-space attachment point on the rigid body
    let r_world = quat_rotate(body.quat_pred, t.r_local.xyz);
    let p_body  = body.pos_pred.xyz + r_world;  // world-space tack point on trim

    // World-space particle position
    let p_cloth = particle.position_pred.xyz;

    // Constraint value: distance between trim attachment and cloth particle
    let diff = p_cloth - p_body;
    let dist = length(diff);
    if dist < 1e-6 { return; }
    let n = diff / dist;  // constraint gradient direction

    // Effective inverse mass of the rigid body at attachment point r_world
    let r_cross_n = cross(r_world, n);
    let w_body = body.pos.w  // inv_mass
                 + dot(r_cross_n, inertia_inv_mul(body, r_cross_n));

    let w_cloth = particle.inv_mass;
    let w_sum   = w_body + w_cloth;
    if w_sum < 1e-8 { return; }

    // XPBD update (compliance scaled by strength_scale for keyframeable tack)
    let alpha      = (t.compliance / t.strength_scale) / (params.dt_sub * params.dt_sub);
    let C          = dist;  // ball joint: C = |p_cloth - p_body|, want C = 0
    let delta_lam  = -(C + alpha * tack_lambdas[ci]) / (w_sum + alpha);
    tack_lambdas[ci] += delta_lam;

    // Correction magnitudes
    let dx_cloth =  w_cloth * n * delta_lam;
    let dx_body  = -w_body  * n * delta_lam;

    // Apply cloth particle correction
    particle.position_pred = vec4(particle.position_pred.xyz + dx_cloth, particle.position_pred.w);
    particles[t.particle_idx] = particle;

    // Apply rigid body translational correction
    body.pos_pred = vec4(body.pos_pred.xyz + dx_body, body.pos_pred.w);

    // Apply rigid body angular correction via quaternion update
    // delta_q = 0.5 * quat(0, I_inv * (r_world x dx_body)) * q_pred
    let ang_impulse  = inertia_inv_mul(body, cross(r_world, dx_body));
    let dq           = vec4<f32>(ang_impulse, 0.0);
    let q_delta      = 0.5 * quat_mul(dq, body.quat_pred);
    body.quat_pred   = normalize(body.quat_pred + q_delta);
    bodies[t.body_idx] = body;
}

// Quaternion multiplication helper
fn quat_mul(a: vec4<f32>, b: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(
        a.w*b.x + a.x*b.w + a.y*b.z - a.z*b.y,
        a.w*b.y - a.x*b.z + a.y*b.w + a.z*b.x,
        a.w*b.z + a.x*b.y - a.y*b.x + a.z*b.w,
        a.w*b.w - a.x*b.x - a.y*b.y - a.z*b.z,
    );
}
```

**GPU concurrency note:** The tack pass writes to both `particles` and `bodies` arrays. Tack constraints are coloured by a graph where nodes are particle indices and body indices, and edges connect every pair sharing a tack constraint. Within a single color partition, no particle index and no body index appears more than once — the writes are race-free without atomics.

---

### [T-TRIM-RIGID.trim-contact-pass] Trim-Cloth Contact Pass

When a rigid trim mesh surface penetrates a cloth triangle, a one-sided contact constraint fires to push the cloth triangle outward. This is structurally identical to the cloth-vs-avatar capsule collision pass (`body_collide.wgsl` in `[T-CLOTH-SOLVER]`), but with the rigid trim's current predicted pose applied to its mesh triangles at runtime.

The trim mesh is represented as a `GpuTrimMesh` containing the mesh triangles in **body-local** space. At each substep predict phase, a pre-pass transforms all trim triangle vertices to world space using the current `body.pos_pred` and `body.quat_pred`. The resulting world-space triangle buffer is used for cloth-particle proximity queries.

```wgsl
// tailor-solver/shaders/trim_contact.wgsl (sketch)
// For each cloth particle, query world-space trim triangle positions.
// Apply one-sided contact constraint if particle is inside trim collision volume.

@compute @workgroup_size(64)
fn trim_contact_pass(@builtin(global_invocation_id) gid: vec3<u32>) {
    let pi = gid.x;
    if pi >= params.n_particles { return; }

    var pos = particles[pi].position_pred.xyz;
    let inv_m_cloth = particles[pi].inv_mass;
    let thickness   = params.collision_dist;

    for (var ti = 0u; ti < arrayLength(&trim_world_triangles); ti++) {
        let tri   = trim_world_triangles[ti];
        let pt    = closest_point_on_triangle(pos, tri.v0.xyz, tri.v1.xyz, tri.v2.xyz);
        let d_vec = pos - pt;
        let dist  = length(d_vec);
        if dist < thickness && dist > 1e-6 {
            // One-sided contact: push cloth particle out
            let n  = d_vec / dist;
            let C  = dist - thickness;  // negative when penetrating
            let dX = -C * n * inv_m_cloth / (inv_m_cloth + trim_bodies[tri.body_idx].pos.w);
            pos   += dX;
        }
    }

    particles[pi].position_pred = vec4(pos, particles[pi].position_pred.w);
}
```

For production performance, a BVH over the trim world-space triangles is rebuilt each substep (or cached from the previous substep for small trim displacements) to cull distant cloth particles before the per-triangle query. For small trims (buttons, buckles: 20–200 triangles), a brute-force per-particle loop is viable; for large armor plates (1000+ triangles), a BVH or AABB pre-filter is required.

---

### [T-TRIM-RIGID.trim-physics-params] Trim Physics Parameters

MD trims have three primary physical parameters beyond standard mesh properties:

| MD Property | Type | Effect | Default |
|---|---|---|---|
| Stiffness | 0–1000 | How rigidly the trim maintains its shape; high = fully rigid kinematic | 100 |
| Trim Weight | g | Physical mass of the trim rigid body; affects inertia and gravity response | derived from mesh + density |
| Tack Strength | 0–100 | How strongly the tack constraints bind the trim to the garment; 0 = detached | 100 |

In the solver, **Stiffness** maps to the rigid body's kinematic mode blend:
- Stiffness = 1000 (max): the trim body is fully kinematic — it does not move dynamically and only couples through tack constraints. Modeled as `inv_mass = 0` for the translational DOF and the body acts as a static collision object that drags cloth particles with it via tack constraints.
- Stiffness < 1000: the trim is semi-rigid — it responds to constraint forces, gravity, and tack tension. `inv_mass` is non-zero.

**Trim Weight** directly sets the mass for the `GpuTrimBody.inv_mass` field. MD 2025.2 made this keyframeable, meaning the mass can change per-substep via the same `GpuSimParams` UBO upload pattern used for `solidify_blend` and `shrink_u/v` in `[T-CLOTH-SOLVER.gpu-architecture.data-layout]`.

**Tack Strength** maps to the `strength_scale` field in `GpuTackConstraint`. Setting this to 0 via the per-substep UBO makes the tack constraint dormant, effectively detaching the trim from the garment mid-animation (MD 2025.2 animated stitching/unstitching feature).

---

### [T-TRIM-RIGID.keyframeable-tack] Keyframeable Tack Strength and Trim Weight

MD 2025.2 added keyframeable tack strength and trim weight (per Group 3 and Group 6 of `[T-MD-FEATURES]`). This is MOAT-4 applied to the trim domain. Implementation follows the same pattern as `solidify_blend` and `pressure_target` in `GpuSimParams`:

```rust
// Extension to MaterialFrameParams (from [T-CLOTH-SOLVER.crate-design.trait-boundary])
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct MaterialFrameParams {
    pub solidify_blend:    f32,
    pub pressure_target:   f32,
    pub shrink_u:          f32,
    pub shrink_v:          f32,
    pub tack_compliance:   f32,   // per-frame tack compliance override (0 = rigid)
    pub tack_strength:     f32,   // 0.0 = detached, 1.0 = full strength
    pub trim_weight_scale: f32,   // multiplier on trim body inv_mass (1.0 = nominal)
}
```

The `tack_compliance` and `tack_strength` fields are used by the tack shader via a per-tack lookup in a uniform block. Because different tacks on the same garment may have different strength curves (some buttons stay attached while a zipper opens), the per-tack `strength_scale` is stored in the `GpuTackConstraint` buffer and updated each substep by a CPU-side CPU→GPU upload of the modified buffer range. This is a mapped buffer write, not a full re-upload.

EventLedger receipts for keyframe animation are emitted as `TailorSimRunCompleted` payload entries carrying the per-frame `MaterialFrameParams` array, making the animation reproducible from the EventLedger alone (required by the determinism contract in `[T-CLOTH-SOLVER.determinism]`).

---

### [T-TRIM-RIGID.tack-data-model] Tack and Attachment Data Model

A **tack** is a point constraint between a world-space point on a trim rigid body and a cloth particle. In Marvelous Designer, tacks are placed by the user clicking a point on a trim mesh and clicking a corresponding point on a cloth panel. Multiple tacks per trim are supported (e.g., a button has 2–4 tacks corresponding to its hole attachment points; a buckle has 4–8 tacks along its strap channel).

The Handshake authority schema for a tack:

```rust
// src/tailor/garment.rs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TackDefinitionV1 {
    pub tack_id:        String,       // "TACK-{uuid_v7}"
    pub trim_id:        String,       // FK → tailor_trims.trim_id
    pub panel_id:       String,       // FK → panel in GarmentDraftV1
    pub r_local:        [f32; 3],     // attachment point in trim body-local coordinates
    pub particle_uv:    [f32; 2],     // UV coordinates on panel to find nearest particle
    pub compliance:     f32,          // constraint compliance (0 = rigid attachment)
    pub strength_curve: Vec<[f32;2]>, // [(frame, strength_scale), ...] keyframe curve
    pub label:          Option<String>, // "button-left-1", "eyelet-top-3", etc.
}
```

A **Glue** placement (MD Glue tool) is modeled as a single-point tack. A **Tack** (MD Tack tool) is a multi-point tack applied across multiple attachment points. The data model is the same; only the placement count differs.

For **eyelets** (lacing tool, MD 2026.0): each eyelet is a rigid trim body with 1 tack to the garment panel at the eyelet center point, plus a constraint connecting adjacent eyelets via the lace cord mesh.

---

### [T-TRIM-RIGID.trim-asset-model] Trim Asset Model

A trim is a rigid 3D mesh stored as an OBJ or FBX import. In the Handshake authority schema, a trim is a first-class asset row in PostgreSQL:

```sql
-- Migration: tailor_trims
CREATE TABLE IF NOT EXISTS tailor_trims (
    trim_id             TEXT PRIMARY KEY,            -- "TRIM-{uuid_v7}"
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,               -- "button-round-4mm", "zipper-standard", ...
    trim_category       TEXT NOT NULL,               -- 'button' | 'buckle' | 'zipper_body' |
                                                     -- 'zipper_slider' | 'eyelet' | 'rivet' |
                                                     -- 'hook' | 'armor_plate' | 'accessory'
    source_asset_ref    TEXT NOT NULL,               -- path to OBJ/FBX file (artifact ref)
    mesh_json           JSONB NOT NULL,              -- TrimMeshV1: vertices + triangles + normals
    inertia_tensor_json JSONB NOT NULL,              -- 3x3 inertia tensor (computed from mesh)
    default_mass_g      FLOAT NOT NULL,              -- physical mass in grams
    default_stiffness   FLOAT NOT NULL DEFAULT 100,  -- stiffness parameter [0, 1000]
    tack_anchor_json    JSONB,                       -- TrimAnchorPointsV1: local r_local positions
    is_library_item     BOOLEAN NOT NULL DEFAULT FALSE,
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Migration: tailor_trim_placements (garment-specific placement of a trim)
CREATE TABLE IF NOT EXISTS tailor_trim_placements (
    placement_id        TEXT PRIMARY KEY,            -- "PLAC-{uuid_v7}"
    garment_id          TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    trim_id             TEXT NOT NULL REFERENCES tailor_trims (trim_id),
    initial_pose_json   JSONB NOT NULL,              -- TrimPoseV1: position + quat at t=0
    tacks_json          JSONB NOT NULL,              -- array of TackDefinitionV1
    stiffness_override  FLOAT,                       -- per-placement override
    mass_override_g     FLOAT,                       -- per-placement mass override
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

The `tack_anchor_json` column on `tailor_trims` stores the **canonical attachment point positions** in body-local coordinates for each trim type. For a four-hole button, this is four `r_local` vectors at the hole positions. For a zipper body, this is the edge attachment rail positions.

---

### [T-TRIM-RIGID.zippers] Two-Way Zippers and Lacing

#### Two-Way Zippers (MD 2025.0)

A zipper in MD is a compound rigid trim:
- **Zipper body**: two parallel toothed rails (rigid OBJ mesh), one on each garment panel edge.
- **Zipper slider**: the puller/slider mesh that moves along the rails.
- **Teeth**: rigid mesh elements that interlock when the slider passes.
- **Stoppers**: fixed rigid endpoints.

From an XPBD constraint perspective, a zipper has:
1. **Rail attachment tacks**: each tooth rail is a rigid trim body with edge-aligned tack constraints to its garment panel edge (one tack per tooth interval).
2. **Seam constraint from teeth**: when the zipper is closed at a position, the two tooth rails are coupled by a sequence of distance constraints between interleaved tooth points — effectively a seam constraint with a fixed rest length equal to the tooth mesh inter-tooth gap.
3. **Slider kinematic**: the slider is a kinematic rigid body that traverses the rail direction. Its position is controlled by an animation parameter (zipper openness ∈ [0, 1]).
4. **One-way constraint**: teeth behind the slider (closed region) carry active seam constraints; teeth ahead of the slider (open region) have their seam constraints disabled. The slider position parameter controls which constraints are active per substep.

For **two-way zippers** (MD 2025.0): two sliders on the same rail, one moving from the top down and one from the bottom up, creating a region of closure between them. The constraint activation logic checks whether a tooth position lies between the two slider positions.

**Handshake zipper data model:**

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZipperDefinitionV1 {
    pub zipper_id:       String,
    pub garment_id:      String,
    pub panel_edge_a:    String,        // edge ID on panel A
    pub panel_edge_b:    String,        // edge ID on panel B
    pub tooth_interval:  f32,           // spacing between teeth in mm
    pub slider_count:    u8,            // 1 or 2 (two-way)
    pub slider_a_pos:    f32,           // 0.0 (bottom) to 1.0 (top)
    pub slider_b_pos:    Option<f32>,   // second slider for two-way
    pub tooth_mesh_ref:  String,        // artifact ref to OBJ tooth mesh
    pub slider_mesh_ref: String,        // artifact ref to OBJ slider mesh
    pub stopper_mesh_ref: String,       // artifact ref to stopper mesh
    pub stiffness:       f32,
}
```

In the GPU solver, the zipper seam constraints are a special case of the standard seam constraint buffer (from `[T-CLOTH-SOLVER.constraint-types.seam]`). Each tooth pair generates one `GpuSeamConstraint` with `ratio = 1.0` and a `active` flag. The active flag is updated each substep by a CPU-side kernel parameter that computes which teeth are within the slider's closed region. This avoids re-uploading the full constraint buffer on each frame for animation; only a small active-mask buffer is updated.

#### Lacing and Eyelets (MD 2026.0)

The lacing tool creates a cord mesh threaded through a sequence of eyelet rigid trim bodies. From a constraint perspective:
- Each eyelet is a rigid trim body with a single tack constraint to the garment panel.
- The cord is a **chain of distance constraints** between consecutive cord particle positions (not a full cloth simulation — the cord uses a simple 1D chain of XPBD stretch constraints).
- At each eyelet, a **threading constraint** pins the cord particle nearest the eyelet hole to the eyelet hole center (a tack-like distance constraint between a cord particle and the eyelet rigid body attachment point).
- The cord can use a fixed rest length or a slack rest length for loose lacing effects.

**Cord chain data type:**

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCordParticle {
    pub pos:      [f32; 4],
    pub vel:      [f32; 4],
    pub pos_pred: [f32; 4],
    pub inv_mass: f32,
    pub _pad:     [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCordSegment {
    pub i0:          u32,
    pub i1:          u32,
    pub rest_length: f32,
    pub compliance:  f32,
}
```

Cord particles are solved in the same substep loop as cloth particles. Their constraints are solved after the tack pass. Threading constraints (cord-to-eyelet) use the same `GpuTackConstraint` form with `particle_idx` pointing into the cord particle buffer and `body_idx` pointing into the eyelet's `GpuTrimBody`.

---

### [T-TRIM-RIGID.pattern-to-rigid] Pattern-to-Rigid-Body Conversion (MD 2025.2)

MD 2025.2 added the ability to convert a cloth pattern panel into a rigid trim inline, enabling armor plate authoring inside MD without external DCC roundtrips. This is MOAT-5's hardest sub-feature: it requires the solver to reclassify particles from cloth DOF (three translational, no rotation) to rigid body DOF (six DOF: position + quaternion) while preserving their current simulated position.

**Conversion algorithm:**

1. **Identify the panel**: the operator selects a cloth panel to convert. Its current simulated mesh positions `{x_i}` are the rigid body's initial shape.
2. **Compute centroid**: `x_centroid = (1/N) * sum(x_i)` in the current simulated frame.
3. **Compute inertia tensor**: from the panel mesh vertices relative to the centroid, assuming uniform surface density from the panel's `FabricPropertySetV1.density_g_m2`.
4. **Create `GpuTrimBody`**: with `pos = x_centroid`, `quat = identity`, `inv_mass = 1/mass`, `inertia_inv = inverse(I)`.
5. **Remove panel particles from cloth solver**: set `inv_mass = 0` for all particles in that panel (makes them non-simulated from cloth perspective).
6. **Add rigid body to trim buffer**: the converted panel's mesh triangles become the trim's mesh triangles in body-local coordinates.
7. **Convert seam constraints that touched the panel**: seam constraints between the converted panel and adjacent cloth panels become tack constraints (the seam endpoints on the rigid side become tack attachment points at their body-local positions).
8. **Emit `TailorGarmentPromoted` update**: an EventLedger event records the conversion, linking the old `panel_id` to the new `trim_id`, so the conversion is reversible from the audit trail.

The conversion is non-destructive in the CRDT document: the original panel geometry is retained as a `superseded_panel` entry in the CRDT document tree, and the trim placement references the original geometry.

**Solidify as soft path to conversion**: MD's `Solidify` property (0–1, keyframeable since 2025.2) is a soft version of pattern-to-trim conversion. Rather than reclassifying DOFs, it blends the cloth panel's compliance toward zero:

```
// In XPBD substep: for each constraint touching a solidified panel
alpha_effective = alpha * (1.0 - solidify_blend)
```

When `solidify_blend = 1.0`, all constraints involving the panel have zero compliance — the panel behaves as a rigid body while remaining in the cloth solver as particles (not as a true rigid body). This is cheaper than full conversion and sufficient for moderate stiffening effects. Full pattern-to-trim conversion is needed for true rigid armor plates that should also collide as rigid bodies with other cloth.

---

### [T-TRIM-RIGID.postgres-schema] Postgres tailor_* Schema for Trims and Tacks

Complete schema for all trim-domain tables (supplementing the existing `tailor_garments`, `tailor_simulation_runs`, `tailor_material_library` from `[T-KERNEL-INTEGRATION.postgres-authority]`):

```sql
-- tailor_trims: authority rows for promoted trim assets
CREATE TABLE IF NOT EXISTS tailor_trims (
    trim_id             TEXT PRIMARY KEY,
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,
    trim_category       TEXT NOT NULL
        CHECK (trim_category IN ('button','buckle','zipper_body','zipper_slider',
                                  'zipper_teeth','eyelet','rivet','hook',
                                  'armor_plate','cord','accessory','custom')),
    source_asset_ref    TEXT,
    mesh_json           JSONB NOT NULL,
    inertia_tensor_json JSONB NOT NULL,
    default_mass_g      FLOAT NOT NULL DEFAULT 1.0,
    default_stiffness   FLOAT NOT NULL DEFAULT 100.0
        CHECK (default_stiffness >= 0.0 AND default_stiffness <= 1000.0),
    tack_anchor_json    JSONB,
    is_library_item     BOOLEAN NOT NULL DEFAULT FALSE,
    converted_from_panel_id TEXT,               -- set if created via pattern-to-trim conversion
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_trims_workspace ON tailor_trims (workspace_id);
CREATE INDEX IF NOT EXISTS ix_tailor_trims_category  ON tailor_trims (trim_category);

-- tailor_trim_placements: per-garment placement of a trim
CREATE TABLE IF NOT EXISTS tailor_trim_placements (
    placement_id        TEXT PRIMARY KEY,
    garment_id          TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    trim_id             TEXT NOT NULL REFERENCES tailor_trims (trim_id),
    initial_pose_json   JSONB NOT NULL,          -- { pos: [x,y,z], quat: [x,y,z,w] }
    tacks_json          JSONB NOT NULL,          -- array<TackDefinitionV1>
    stiffness_override  FLOAT,
    mass_override_g     FLOAT,
    layer_order         INTEGER NOT NULL DEFAULT 0,
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_trim_placements_garment
    ON tailor_trim_placements (garment_id);

-- tailor_zippers: per-garment zipper definitions
CREATE TABLE IF NOT EXISTS tailor_zippers (
    zipper_id           TEXT PRIMARY KEY,
    garment_id          TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    panel_edge_a        TEXT NOT NULL,
    panel_edge_b        TEXT NOT NULL,
    tooth_interval_mm   FLOAT NOT NULL DEFAULT 5.0,
    slider_count        INTEGER NOT NULL DEFAULT 1
        CHECK (slider_count IN (1, 2)),
    slider_a_pos        FLOAT NOT NULL DEFAULT 0.0
        CHECK (slider_a_pos >= 0.0 AND slider_a_pos <= 1.0),
    slider_b_pos        FLOAT
        CHECK (slider_b_pos IS NULL OR (slider_b_pos >= 0.0 AND slider_b_pos <= 1.0)),
    tooth_mesh_ref      TEXT,
    slider_mesh_ref     TEXT,
    stiffness           FLOAT NOT NULL DEFAULT 100.0,
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- tailor_lacings: per-garment lacing cord definitions (MD 2026.0 Lacing Tool)
CREATE TABLE IF NOT EXISTS tailor_lacings (
    lacing_id           TEXT PRIMARY KEY,
    garment_id          TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    eyelet_sequence_json JSONB NOT NULL,         -- ordered array of trim_placement_ids (eyelets)
    cord_rest_length_mm FLOAT NOT NULL DEFAULT 3.0,
    cord_compliance     FLOAT NOT NULL DEFAULT 1e-4,
    lace_pattern        TEXT NOT NULL DEFAULT 'straight',  -- 'straight' | 'criss-cross'
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Migration naming:** following the codebase's dated migration convention (and noting `KI-MIGRATION-COLLISION` in `index.yaml`: migration `0334_*` is already taken by `loom`), the Tailor trim migrations would use the next available dated convention, e.g. `2026_06_17_tailor_trims.sql` and `2026_06_17_tailor_lacings.sql`. The migration number is assigned by the project migration registry, not here.

---

### [T-TRIM-RIGID.kernel-binding] Kernel Primitive Bindings

**EventLedger event taxonomy additions for trims:**

```rust
// New KernelEventType variants (extend [T-KERNEL-INTEGRATION.event-taxonomy]):
TailorTrimImported,               // OBJ/FBX trim imported and validated
TailorTrimPlaced,                 // trim placed on garment with tack definitions
TailorTrimTackUpdated,            // tack definition changed (position, compliance, strength)
TailorZipperDefined,              // zipper definition created for a garment edge pair
TailorZipperSliderMoved,          // zipper slider position updated (animation keyframe)
TailorLacingDefined,              // lacing cord defined for eyelet sequence
TailorPatternToTrimConverted,     // cloth panel baked to rigid trim (pattern-to-trim)
TailorTrimSimRunIncluded,         // trim bodies included in a simulation run
TailorTrimContactViolation,       // trim-cloth contact validation detected deep penetration
```

**Sandbox integration:** the `TailorSandboxAdapter` (from `[T-KERNEL-INTEGRATION.sandbox]`) receives the full garment draft including `trim_placements`, `zippers`, and `lacings`. It:
1. Loads each `GpuTrimBody` from the `tailor_trim_placements` JSONB.
2. Uploads trim body buffer to GPU.
3. Builds tack constraints from `tacks_json` and uploads the `GpuTackConstraint` buffer.
4. Builds zipper seam constraints and tooth-rail tack constraints.
5. Runs the extended substep loop (with tack pass and trim_contact pass).
6. Reads back final trim body positions and orientations for the output artifact.

**Validation gate additions** (extending `[T-KERNEL-INTEGRATION.validation]`):

```
// BLOCKING checks (trim-specific):
//   - trim_no_penetration:    no GpuTrimBody mesh triangle interpenetrates a cloth triangle
//                             after final simulated frame
//   - tack_seam_closure:      all tack distances <= 5mm at end of draping phase
//   - zipper_tooth_alignment: tooth rail tacks within 1mm of their panel edge positions
//   - lacing_cord_length:     no cord segment stretched beyond 200% rest length

// ADVISORY checks:
//   - trim_gravity_stable:    no trim body translating > 50mm/frame in final 10 frames
//   - tack_strength_nonzero:  warn if any tack_strength < 0.01 (may be unintentionally detached)
```

**CRDT:** trim placements are stored as CRDT document entries under the garment document. Operators and model agents can propose placement moves (drag the button to a different position on the panel) as `AiEditProposalRequestV1` entries referencing the `placement_id` and the new `initial_pose_json`. Tack positions update automatically when the trim moves, since they are defined in body-local space.

---

### [T-TRIM-RIGID.model-first-api] Model-First API

The `TailorModelAdapter` (from `[T-KERNEL-INTEGRATION.model-lanes]`) is extended with MCP tools for trim authoring. Following the rmcp pattern from `[T-MODEL-FIRST-API.field-survey]`:

```rust
// src/tailor/api.rs — MCP tool definitions for trim domain

/// Place a trim on a garment panel.
#[tool(description = "Place a trim (button, buckle, zipper, eyelet) on a garment panel with tack attachment points.")]
async fn tailor_place_trim(
    /// Garment ID to attach the trim to.
    garment_id: String,
    /// Trim type from the library: 'button', 'buckle', 'eyelet', 'zipper_body', 'armor_plate', etc.
    trim_category: String,
    /// Optional library trim ID; if omitted, the default library item for the category is used.
    trim_id: Option<String>,
    /// World-space or panel-UV position for the trim centroid. Format: { "panel_id": "P-front",
    /// "uv": [0.5, 0.3] } for panel-UV placement.
    position: serde_json::Value,
    /// Array of tack definitions. Each tack: { "r_local": [dx, dy, dz], "particle_uv": [u, v],
    /// "compliance": 0.0 }. If omitted, default anchor points for the trim type are used.
    tacks: Option<Vec<serde_json::Value>>,
    /// Trim stiffness [0–1000]. Default 100.
    stiffness: Option<f32>,
) -> CallToolResult { ... }

/// Define a zipper on a garment edge pair.
#[tool(description = "Define a zipper between two garment panel edges. Supports 1- and 2-slider (two-way) zippers.")]
async fn tailor_define_zipper(
    garment_id: String,
    panel_edge_a: String,
    panel_edge_b: String,
    /// Tooth spacing in mm. Default 5.0.
    tooth_interval_mm: Option<f32>,
    /// 1 or 2 sliders.
    slider_count: Option<u8>,
    /// Slider A position [0=bottom, 1=top]. Default 1.0 (fully closed).
    slider_a_pos: Option<f32>,
    /// Slider B position for two-way zipper.
    slider_b_pos: Option<f32>,
) -> CallToolResult { ... }

/// Convert a cloth pattern panel to a rigid trim (armor plate / accessory).
#[tool(description = "Convert a simulated cloth panel to a rigid trim body. Enables armor and hard-surface costume authoring.")]
async fn tailor_convert_panel_to_trim(
    garment_id: String,
    panel_id: String,
    /// Optional name for the resulting trim.
    trim_name: Option<String>,
    /// Trim category after conversion: 'armor_plate' or 'accessory'. Default 'armor_plate'.
    trim_category: Option<String>,
) -> CallToolResult { ... }

/// Keyframe tack strength for animated stitching/unstitching.
#[tool(description = "Set tack strength keyframe for animated stitching. strength=0 detaches the trim; strength=1 fully attaches.")]
async fn tailor_keyframe_tack_strength(
    placement_id: String,
    tack_id: String,
    /// Frame number.
    frame: u32,
    /// Strength [0.0–1.0].
    strength: f32,
) -> CallToolResult { ... }
```

**Model authoring guidance for trims:** The `GarmentDraftV1` schema (from `[T-KERNEL-INTEGRATION.model-lanes]`) is extended with an optional `trim_placements` array:

```json
{
  "schema_version": "hsk.cloth.garment_draft@1",
  "garment_type": "jacket",
  "panels": [...],
  "seams": [...],
  "trim_placements": [
    {
      "placement_id": "PLAC-001",
      "trim_category": "button",
      "position": { "panel_id": "P-front-left", "uv": [0.5, 0.2] },
      "tacks": [
        { "r_local": [0, 2.0, 0], "particle_uv": [0.5, 0.21], "compliance": 0.0 },
        { "r_local": [0, -2.0, 0], "particle_uv": [0.5, 0.19], "compliance": 0.0 }
      ],
      "stiffness": 500
    }
  ],
  "zippers": [
    {
      "panel_edge_a": "E-left-center",
      "panel_edge_b": "E-right-center",
      "tooth_interval_mm": 5.0,
      "slider_count": 1,
      "slider_a_pos": 1.0
    }
  ]
}
```

An LLM generating a jacket `GarmentDraftV1` can include trim placements from the default library without needing trim mesh specifics — the solver resolves the mesh from the `trim_category` lookup against `tailor_trims.is_library_item = TRUE`.

---

### [T-TRIM-RIGID.oss-references] OSS Reference Map

| Project | Relevance | What to reuse |
|---|---|---|
| `InteractiveComputerGraphics/PositionBasedDynamics` (C++, MIT) | RigidBodyClothCouplingDemo.cpp shows `addRigidBodyParticleBallJoint` — the ball joint constraint between a rigid body and a soft particle. Direct algorithm reference. | Ball joint formula; rigid body quaternion update in substep; coupling constraint graph structure |
| Müller, Macklin et al., "Detailed Rigid Body Simulation with XPBD" (SCA 2020, CGF 39) | Canonical XPBD rigid body formulation: position + quaternion DOF, compliance-based joints, soft-rigid coupling in one solver pass. | Effective inverse mass formula `w = 1/m + (r×n)·I⁻¹(r×n)`; quaternion correction `delta_q`; unified substep loop |
| `vitalight/Velvet` (CUDA, MIT) | Implements `attach` constraints (VtClothSolverGPU): "keep particles fixed to 3D positions" and "long-range attachment constraints". Close analog to a fully kinematic tack. | Pin constraint GPU kernel pattern; Jacobi delta accumulation for attachment constraints |
| `maria-korosteleva/NvidiaWarp-GarmentCode` (NVIDIA Warp, MIT) | Implements equality- and inequality-based attachment constraints for cloth garment placement: "fixing skirt placement on waist area" is the attachment constraint that maps to tack. | Attachment constraint handling in XPBD context; two-sided (equality) vs one-sided (inequality) variants |
| `openxrlab/xrtailor` (CUDA, Apache-2) | "Binding constraints for tether effects" and "LRA constraints" in the XPBD quality mode. | Tether/binding constraint as alternative tack formulation; GPU data layout for binding constraints |
| `taichi-dev/meshtaichi/xpbd_cloth` (Taichi, Apache-2) | XPBD cloth with mesh-based constraints in Taichi; closest to the mixed-mesh simulation. | Inertia tensor computation from mesh; GPU-parallel rigid body substep |

No OSS project studied implements a full dynamic trim-as-rigid-body coupled to cloth particles in a GPU WGSL/compute shader. The closest field evidence is the Müller 2020 paper and the `PositionBasedDynamics` C++ demo. The WGSL implementation is new work; the algorithm is well-established.

---

### [T-TRIM-RIGID.risks] Risks and Open Questions

| Risk | Severity | Mitigation |
|---|---|---|
| **WGSL quaternion precision:** f32 quaternion arithmetic accumulates normalization drift over many substeps, especially for fast-spinning trims. WGSL has no f64. | Medium | Normalize `quat_pred` every substep after correction (cheap, already shown in shader sketch); apply a periodic renormalization pass every 10 substeps; trim rotation is small for garment trims. |
| **Race condition in tack pass write to same rigid body:** if two tack constraints share a body but end up in the same color class (coloring error), writes to `bodies[body_idx]` race without atomics. | High | Rigid body index must be part of the coloring graph nodes alongside particle indices; verify coloring coverage includes both node types before dispatch. |
| **WGSL lacks atomic read-modify-write on struct fields:** writing `bodies[i].pos_pred` and `bodies[i].quat_pred` in the same pass from multiple workgroup invocations requires the body be in only one constraint per dispatch. | High | Graph coloring (required above) guarantees this; document as a hard invariant that must be maintained when adding new constraint types that touch `GpuTrimBody`. |
| **BVH rebuild for trim-cloth contact per substep:** for large armor plates (>500 triangles), a full BVH rebuild per substep is expensive. | Medium | Cache the world-space triangle buffer from the previous substep; for small trim displacements (< 0.5 × collision_thickness), skip BVH rebuild. Detect large motion and force rebuild. |
| **Pattern-to-trim conversion mid-animation:** converting a cloth panel to a rigid body after simulation has started requires flushing the constraint buffer and re-uploading. | Medium | Restrict pattern-to-trim conversion to the pre-simulation authoring phase, not during animation playback; add a validation gate check that no conversions occur in the active simulation path. |
| **Zipper tooth constraint count:** a 30 cm zipper with 5 mm tooth interval has 60 teeth, each generating 1 seam constraint and 2 tack constraints per rail = 180 additional constraints per zipper. For a garment with 4 zippers this is 720 constraints, manageable but must be counted in the total constraint budget. | Low | Include zipper constraint count in the pre-simulation budget check emitted in `TailorSimRunRequested` payload. |
| **Two-way zipper active mask update:** switching which seam constraints are active as the slider moves requires a CPU-side buffer update per frame. For real-time interactive authoring, this must be fast. | Low | Use a small `active_mask` bool buffer (one bit per tooth pair); update only the changed range on slider move. The mask is a separate small GPU buffer, not the full constraint buffer. |
| **Lacing cord instability:** a cord chain attached to eyelets can develop slack/stiffness oscillation if the eyelet rigid bodies move faster than the cord constraint can propagate. | Medium | Set cord compliance low (near 0) for stiff lacing; apply position-based rod constraint (not just distance) to prevent cord twist; treat extreme lace instability as a validation advisory, not blocking. |
| **Inertia tensor computation from panel mesh:** the pattern-to-trim conversion computes the inertia tensor from cloth mesh vertices. If the panel has a highly irregular triangulation (very non-uniform particle distances), the inertia tensor will be numerically imprecise. | Low | Use the mesh-area-weighted inertia formula (triangle contribution weighted by area); this is robust for non-uniform meshes. Validate that `det(I) > 0` before inverting. |

---

### [T-TRIM-RIGID.sources] Sources

1. Müller, Macklin, Chentanez, Jeschke, Kim — "Detailed Rigid Body Simulation with Extended Position Based Dynamics" (SCA 2020, CGF 39(8)): https://dl.acm.org/doi/10.1111/cgf.14105
2. InteractiveComputerGraphics/PositionBasedDynamics — RigidBodyClothCouplingDemo.cpp (C++, MIT): https://github.com/InteractiveComputerGraphics/PositionBasedDynamics/blob/master/Demos/CouplingDemos/RigidBodyClothCouplingDemo.cpp
3. InteractiveComputerGraphics/PositionBasedDynamics DeepWiki — rigid body joint types (ball, hinge, universal, slider): https://deepwiki.com/InteractiveComputerGraphics/PositionBasedDynamics
4. vitalight/Velvet — CUDA XPBD cloth solver with attach and long-range-attachment constraints: https://github.com/vitalight/Velvet
5. maria-korosteleva/NvidiaWarp-GarmentCode — equality/inequality attachment constraints for XPBD cloth garment placement: https://github.com/maria-korosteleva/NvidiaWarp-GarmentCode
6. openxrlab/xrtailor — XPBD binding and LRA constraints for cloth: https://github.com/openxrlab/xrtailor
7. openxrlab/xrtailor DeepWiki — XRTailor XPBD constraint types: https://deepwiki.com/openxrlab/xrtailor/4.2-extended-position-based-dynamics-(xpbd)
8. Mercier-Aubin & Kry — "A Multi-Layer Solver for XPBD" (SCA 2024, CGF 43(8)): https://onlinelibrary.wiley.com/doi/10.1111/cgf.15186
9. Macklin, Müller, Chentanez — "XPBD: Position-Based Simulation of Compliant Constrained Dynamics" (MIG 2016): https://dl.acm.org/doi/pdf/10.1145/2994258.2994272
10. Survey of Rigid Body Simulation with Extended Position Based Dynamics (arXiv 2311.09327, 2023): https://arxiv.org/abs/2311.09327
11. Physically Accurate Rigid-Body Dynamics in Particle-Based Simulation — PBD-R (arXiv 2603.14634, 2026): https://arxiv.org/abs/2603.14634
12. Marvelous Designer Trims documentation: https://support.marvelousdesigner.com/hc/en-us/articles/47358146261913-Trims
13. Marvelous Designer ZIPPER: Create Zippers: https://support.marvelousdesigner.com/hc/en-us/articles/360037395551-ZIPPER-Create-Zippers
14. Marvelous Designer ZIPPER: Slider and Stopper Properties: https://support.marvelousdesigner.com/hc/en-us/articles/360037395631-ZIPPER-Zipper-Slider-and-Stopper-Properties
15. Marvelous Designer Solidify property: https://support.marvelousdesigner.com/hc/en-us/articles/47358310357529-Solidify
16. Marvelous Designer 2025.0 New Feature List — Two-Way Zippers, GPU Simulation improvements: https://support.marvelousdesigner.com/hc/en-us/articles/47358120307353-Marvelous-Designer-2025-0-New-Feature-List
17. Marvelous Designer 2025.2: MetaHumans, Pleats and Keyframes (Digital Production, Nov 2025) — pattern-to-trim, keyframeable tack strength, trim weight: https://digitalproduction.com/2025/11/18/marvelous-designer-2025-2-metahumans-pleats-keyframes/
18. CGPress: Marvelous Designer 2025.2 — keyframeable shrinkage, pressure, solidify, tack strength, trim weight: https://cgpress.org/archives/marvelous-designer-2025-2-introduces-improved-animation-controls-trim-conversion-fbx-export-metahuman-support-and-a-new-pleat-tool.html
19. Marvelous Designer 2026.0 New Feature List — GPU-Accelerated Trim Simulation, Lacing Tool: https://support.marvelousdesigner.com/hc/en-us/articles/55837641308313-Marvelous-Designer-2026-0-New-Feature-List
20. Marvelous Designer 2026.0 release (Digital Production, Apr 2026) — 3D Pencil and Lacing Tool: https://digitalproduction.com/2026/04/15/marvelous-designer-2026-0-adds-3d-pencil-and-lacing/
21. CG Channel: Marvelous Designer 2026.0 — GPU-accelerated trim buttons/buckles: https://www.cgchannel.com/2026/04/clo-virtual-fashion-releases-marvelous-designer-2026-0/
22. Real-Time Cloth Simulation Using WebGPU (arXiv 2507.11794, 2025) — WebGPU compute shader performance baselines: https://arxiv.org/abs/2507.11794
