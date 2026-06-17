---
file_id: cloth-engine-fabric-models
topic_id: T-FABRIC-MODELS
title: "Fabric and Material Models"
status: draft
depends_on:
  - T-CLOTH-SOLVER
  - T-MD-FEATURES
summary: "Anisotropic fabric physical properties (density, stretch weft/warp/shear, bend, buckling, friction, damping, pressure) mapped to XPBD compliance parameters, with a typed Postgres-authority preset library and Handshake-native design."
sources: 22
updated_at: "2026-06-17"
---

## [T-FABRIC-MODELS] Fabric and Material Models

### [T-FABRIC-MODELS.md-feature-map] Marvelous Designer Feature Coverage

Marvelous Designer (2025.x / 2026.0) exposes the following fabric physical property taxonomy, which defines the complete feature surface the Tailor engine must match. All properties are per-pattern-piece and can be overridden per garment assembly.

**Stretch group** (resistance to in-plane deformation):

| MD Property | Description | Typical range |
|---|---|---|
| Stretch-Weft | Resistance to horizontal stretch (across grain) | 0–100% |
| Stretch-Warp | Resistance to vertical stretch (along grain) | 0–100% |
| Shear | Resistance to diagonal / angular deformation | 0–100% |

**Bending group** (out-of-plane resistance):

| MD Property | Description | Typical range |
|---|---|---|
| Bending-Weft | Bending stiffness in the weft (horizontal) direction | 0–100 |
| Bending-Warp | Bending stiffness in the warp (vertical) direction | 0–100 |
| Buckling Ratio | At what fraction of bend angle the buckling term kicks in (wrinkle frequency) | 0–100% |
| Buckling Stiffness | Absolute stiffness at the buckled corner; controls crispness vs soft fold | 0–100 |

Higher bending values produce denim/leather behavior; lower values produce silk/jersey drape. Buckling Ratio near 100% = easily wrinkled (silk); near 0% = fewer wrinkles (denim, wool).

**Mass and thickness group**:

| MD Property | Description | Typical range |
|---|---|---|
| Density | Mass per unit area (g/m²); drives gravity response | 20–600 g/m² |
| Collision Thickness | Gap maintained between cloth and collision objects | 0.1–5 mm |
| Rendering Thickness | Visual thickness for output; decoupled from simulation | 0.1–3 mm |

**Dynamics group**:

| MD Property | Description | Typical range |
|---|---|---|
| Internal Damping | Per-particle velocity damping; damps jitter | 0–100 |
| Friction | Cloth-avatar and cloth-cloth friction | 0–100% |
| Pressure | Inflatable-object mode; keyframeable in 2025.2 | 0–100 |
| Solidify | Stiffness blend between soft and rigid; keyframeable in 2025.2 | 0–100% |
| Shrinkage-Weft | Pattern shrinkage factor in horizontal direction; keyframeable in 2025.2 | -100–100% |
| Shrinkage-Warp | Pattern shrinkage factor in vertical direction; keyframeable in 2025.2 | -100–100% |

**Preset library**: MD ships named presets bundling all properties into consistent fabric archetypes. Authoritatively confirmed preset names include: Cotton, Denim, Silk, Leather, Wool, Jersey, Satin, Canvas, Rubber, and user-defined custom presets. Each preset is a named bundle of all property values above saved as a `.zpac` sidecar.

**Moat signal**: True anisotropic weft/warp/shear split is absent from most open-source XPBD solvers. GarmentCode's `stiff_ochra.json` and `soft_ochra.json` expose `warp_resistance_scale` / `weft_resistance_scale` as scalar multipliers on a shared base stretch resistance, which is a simplified anisotropy approximation. Marvelous Designer implements full orthotropic stretch and bend with independent weft/warp stiffness tensors, making the two properties truly independent axes. The Tailor solver must implement full orthotropic compliance, not scalar-multiply shortcuts.

### [T-FABRIC-MODELS.xpbd-compliance-mapping] XPBD Compliance Parameter Mapping

XPBD (Macklin et al. 2016) replaces stiffness `k` with compliance `alpha = 1/k`. The key property of XPBD is that the effective stiffness becomes timestep-independent:

```
alpha_tilde = alpha / (dt^2)

delta_lambda = -(C + alpha_tilde * lambda) / (sum_i(w_i * |grad_C_i|^2) + alpha_tilde)

delta_x_i = w_i * grad_C_i * delta_lambda
```

Where:
- `alpha` is material compliance (m²/N for stretch, m²/(N·m) for bend)
- `dt` is the substep size (frame_dt / n_substeps)
- `lambda` is the accumulated Lagrange multiplier (reset each substep)
- `C` is the constraint violation scalar
- `w_i = 1/m_i` is the inverse particle mass
- `grad_C_i` is the constraint gradient for particle `i`

**Stiffness-to-compliance translation**: `alpha = 1.0 / stiffness`. Low `alpha` = near-inextensible (leather, denim). High `alpha` = very soft (chiffon, silk).

**Anisotropic constraint functions for cloth**:

The orthotropic StVK (Saint Venant-Kirchhoff) shell model defines separate constraint functions for each strain direction. For a triangle with deformation gradient `F`:

- Stretch-weft constraint: `C_weft(x) = F[:,0] · F[:,0] - 1` (weft column of F)
- Stretch-warp constraint: `C_warp(x) = F[:,1] · F[:,1] - 1` (warp column of F)
- Shear constraint: `C_shear(x) = F[:,0] · F[:,1]`
- Bend-weft constraint: dihedral angle between adjacent triangles sharing weft-direction edges
- Bend-warp constraint: dihedral angle between adjacent triangles sharing warp-direction edges

Each constraint carries its own compliance scalar, giving six independent anisotropic parameters per fabric. This matches MD's Stretch-Weft, Stretch-Warp, Shear, Bending-Weft, Bending-Warp, and a combined Buckling compliance for the buckled-corner stiffness change.

**Approximate compliance ranges for common fabric archetypes** (these are simulation-space values, not physical units; vary by mesh resolution and substep count):

| Fabric | Stretch compliance (alpha_s) | Bend compliance (alpha_b) | Density g/m² |
|---|---|---|---|
| Silk / chiffon | 1e-4 to 1e-3 | 1e-2 to 1e-1 | 20–60 |
| Jersey / knit | 1e-5 to 1e-4 | 5e-3 to 5e-2 | 80–180 |
| Cotton / poplin | 1e-6 to 1e-5 | 1e-3 to 1e-2 | 100–200 |
| Denim | 1e-8 to 1e-7 | 1e-5 to 1e-4 | 280–450 |
| Leather | 1e-9 to 1e-8 | 1e-6 to 1e-5 | 400–600 |
| Rubber | 1e-4 (very stretchy) | 1e-3 | 200–400 |

These ranges are calibrated to the nikhilr612/xpbdrs default `length_compliance: 0.001` (cotton-like) and validated against the GarmentCode `stiff_ochra.json` (bend_resistance: 2.0 maps to stiff/leather tier) vs `soft_ochra.json` (bend_resistance: 0.03 maps to silk/jersey tier).

**Damping**: XPBD supports position-based velocity damping via a `damping_factor` scalar applied to velocity before each substep. The GarmentCode default_sim_props uses `global_damping_factor: 0.25`. Per-material internal damping adds a separate `kd` coefficient to the constraint delta:

```
delta_lambda_damped = -(C + alpha_tilde * lambda + beta_tilde * C_dot) / denom
```

Where `beta` is the damping compliance (analogous to Rayleigh damping).

**Friction**: Implemented as a position correction tangent to the contact normal at collision resolution time. Coulomb friction model: `|delta_x_tangent| <= mu * |delta_x_normal|`. The `mu` value maps directly from the MD friction coefficient (0–1).

**Pressure / inflation**: Adds a volume-preservation constraint `C_vol = V_current / V_rest - 1` with its own compliance `alpha_pressure`. The MD Pressure property maps to the rest volume target ratio.

### [T-FABRIC-MODELS.oss-reference] Open-Source Reference Implementations

**GarmentCode (ETH Zurich, MIT, Python)**
Repo: https://github.com/maria-korosteleva/GarmentCode

The `assets/Sim_props/` directory contains fabric material presets:

- `stiff_ochra.json` (leather/denim-like): `bend_resistance: 2.0`, `stretch_resistance: 400.0`, `shear_resistance: 0.4`, `density: 0.015`, `friction: 0.01`, `body_friction: 0.25`, `collision_thickness: 0.04`, `air_drag: 0.01`, `bend_damp: 0.1`, `stretch_damp: 0.2`. Includes `warp_resistance_scale: 1.0` / `weft_resistance_scale: 1.0` as anisotropy multipliers.

- `soft_ochra.json` (silk/jersey-like): Same structure but `bend_resistance: 0.03` (67x softer bend than stiff preset). Confirms that bend resistance is the primary differentiator between fabric archetypes.

- `default_sim_props.yaml` (Qualoth-style engine): Uses `garment_edge_ke: 1.0` (very soft edge elasticity), `garment_tri_ke: 10000.0` (triangle stretch stiffness), `fabric_density: 1.0`, `fabric_friction: 0.5`, `body_collision_thickness: 0.25`. This is the Qualoth FEM simulator property format, not XPBD, but maps to the same conceptual parameters.

The GarmentCode sim props use three independent resistance values (stretch_resistance, shear_resistance, bend_resistance) plus per-direction scale multipliers (warp/weft). The Tailor solver should promote these multipliers to fully independent compliance scalars.

**nikhilr612/xpbdrs (Rust, MIT)**
Repo: https://github.com/nikhilr612/xpbdrs

API (from inspected README):
```rust
let params = XpbdParams {
    length_compliance: 0.001,   // stretch stiffness (cotton-like)
    volume_compliance: 0.001,   // for tetrahedral volumes (not cloth-specific)
    n_substeps: 10,
    time_substep: 0.016 / 10.0,
    ..Default::default()
};
```

Single isotropic `length_compliance` only — no weft/warp/shear split. The Tailor solver must extend this to a `ClothMaterialCompliance` struct with six independent scalars.

**vitalight/Velvet (C++17/CUDA, MIT)**
Repo: https://github.com/vitalight/Velvet

Uses Jacobi iteration with delta accumulation for GPU parallelism. Constraint pipeline: stretch (edge distance) → attachment (long-range) → bending (dihedral). Material properties enter as stiffness values per-constraint-type. The GPU Jacobi approach accumulates `deltas` buffer across parallel constraint evaluations and applies them after all constraints are processed. This is the reference implementation pattern for the WGSL compute kernels.

**RobDavenport/softy (Rust, MIT)**
Repo: https://github.com/RobDavenport/softy

Uses spring-damper + Verlet/PBD with `DampingMode` enum (critical/underdamped/overdamped). `VerletGrid` supports structural/shear/bend links. Useful as a CPU fallback reference for the damping mode abstraction.

**ccincotti3/webgpu_cloth_simulator (TypeScript/WGSL)**
Repo: https://github.com/ccincotti3/webgpu_cloth_simulator

Implements distance constraint, isometric bending, self-collision in WGSL compute shaders. Uses constraint graph coloring for GPU-parallel Gauss-Seidel. The `Cloth.ts` and `Constraint.ts` types define per-constraint compliance values passed as GPU buffer uniforms. This is the direct WGSL constraint coloring template for the Tailor solver.

**jspdown/cloth (TypeScript/WGSL)**
Repo: https://github.com/jspdown/cloth

WebGPU XPBD with small-step technique. 100% GPU execution. Parallel Gauss-Seidel with constraint graph coloring. Material compliance passed as per-constraint scalar in GPU storage buffers.

### [T-FABRIC-MODELS.rust-design] Handshake-Native Rust Design

The fabric model lives in the standalone `tailor-solver` Rust crate (no Handshake/Tauri/sqlx dependencies) with a separate Tailor creative module storage glue layer.

**Core solver-crate types (tailor-solver/src/material.rs)**:

```rust
/// Anisotropic XPBD compliance for a single fabric.
/// All fields are in m²/N (stretch) or dimensionless ratios
/// that the solver normalizes by triangle area / edge length.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClothMaterialCompliance {
    /// In-plane stretch compliance along the weft (horizontal/cross-grain) axis.
    /// Lower = stiffer (denim: ~1e-8, silk: ~1e-4).
    pub stretch_weft: f32,
    /// In-plane stretch compliance along the warp (vertical/along-grain) axis.
    pub stretch_warp: f32,
    /// In-plane shear compliance (diagonal deformation resistance).
    pub stretch_shear: f32,
    /// Out-of-plane bending compliance along weft-direction edges.
    pub bend_weft: f32,
    /// Out-of-plane bending compliance along warp-direction edges.
    pub bend_warp: f32,
    /// Compliance at the buckling corner (wrinkle stiffness reduction factor).
    /// 0.0 = same stiffness at buckle; 1.0 = zero stiffness at buckle (sharp fold).
    pub buckling_ratio: f32,
    /// Additional stiffness at the buckled corner (additive to bend compliance).
    pub buckling_stiffness: f32,
}

/// Per-fabric physical parameters: mass, thickness, dynamics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClothMaterialPhysics {
    /// Surface density in kg/m² (convert from g/m² at load time: divide by 1000).
    /// Cotton: 0.12–0.20; Denim: 0.28–0.45; Silk: 0.02–0.06.
    pub density_kg_per_m2: f32,
    /// Simulation collision envelope in metres (default: 0.0025 = 2.5 mm).
    pub collision_thickness_m: f32,
    /// Coulomb friction coefficient for cloth-avatar contact (0–1).
    pub friction: f32,
    /// Cloth-cloth friction coefficient for self-collision resolution (0–1).
    pub self_friction: f32,
    /// Rayleigh-like velocity damping multiplier per substep (0–1).
    pub internal_damping: f32,
    /// Air drag coefficient (opposes velocity, simulates resistance).
    pub air_drag: f32,
    /// Inflation pressure target (0 = no pressure; >0 = inflatable mode).
    pub pressure: f32,
    /// Stiffness blend [0,1]: 0 = fully soft, 1 = fully rigid (solidify).
    pub solidify: f32,
    /// Shrinkage multiplier in weft direction (1.0 = no shrinkage, 0.9 = 10% shrink).
    pub shrinkage_weft: f32,
    /// Shrinkage multiplier in warp direction.
    pub shrinkage_warp: f32,
}

/// Complete fabric material descriptor used by the solver.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FabricMaterial {
    pub compliance: ClothMaterialCompliance,
    pub physics: ClothMaterialPhysics,
    /// Grain direction in the 2D pattern (angle in radians from horizontal).
    /// Controls how weft/warp axes align with pattern coordinates.
    pub grain_angle_rad: f32,
}

impl FabricMaterial {
    /// Returns a cotton-like default suitable for general-purpose testing.
    pub fn cotton() -> Self { /* see preset table */ }
    pub fn silk() -> Self { /* ... */ }
    pub fn denim() -> Self { /* ... */ }
    pub fn leather() -> Self { /* ... */ }
    pub fn jersey() -> Self { /* ... */ }
}
```

**GPU buffer layout (tailor-solver/src/gpu/material_uniform.rs)**:

The WGSL solver consumes material as a uniform buffer bound at `@group(0) @binding(2)`:

```rust
/// Mirrors the WGSL `MaterialParams` struct byte-for-byte.
/// Must be repr(C) + bytemuck::Pod for safe GPU upload.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialParamsGpu {
    // Compliance pack (6 x f32)
    pub stretch_weft: f32,
    pub stretch_warp: f32,
    pub stretch_shear: f32,
    pub bend_weft: f32,
    pub bend_warp: f32,
    pub buckling_ratio: f32,
    // Physics pack (6 x f32)
    pub density_kg_per_m2: f32,
    pub friction: f32,
    pub self_friction: f32,
    pub internal_damping: f32,
    pub air_drag: f32,
    pub pressure: f32,
    // Animation overrides (4 x f32 for keyframeable props)
    pub solidify: f32,
    pub shrinkage_weft: f32,
    pub shrinkage_warp: f32,
    pub _pad: f32,  // 16-byte alignment
}
```

**WGSL material uniform (tailor-solver/src/shaders/cloth_material.wgsl)**:

```wgsl
struct MaterialParams {
    stretch_weft: f32,
    stretch_warp: f32,
    stretch_shear: f32,
    bend_weft: f32,
    bend_warp: f32,
    buckling_ratio: f32,
    density_kg_per_m2: f32,
    friction: f32,
    self_friction: f32,
    internal_damping: f32,
    air_drag: f32,
    pressure: f32,
    solidify: f32,
    shrinkage_weft: f32,
    shrinkage_warp: f32,
    _pad: f32,
}

@group(0) @binding(2) var<uniform> material: MaterialParams;

// Anisotropic stretch constraint correction for one edge.
// dir_uv: [0,1] = weft edge, [0,0] = warp edge
fn stretch_compliance(dir_uv: vec2<f32>) -> f32 {
    let weft_w = dir_uv.x;
    let warp_w = 1.0 - dir_uv.x;
    return material.stretch_weft * weft_w + material.stretch_warp * warp_w;
}

// Anisotropic bend compliance for a dihedral pair.
fn bend_compliance(is_weft_edge: u32) -> f32 {
    if is_weft_edge == 1u {
        return material.bend_weft;
    }
    return material.bend_warp;
}

// XPBD constraint correction (single-constraint, positional).
fn xpbd_delta_lambda(
    C: f32,
    lambda: f32,
    alpha: f32,
    dt: f32,
    w_sum: f32,
) -> f32 {
    let alpha_tilde = alpha / (dt * dt);
    return -(C + alpha_tilde * lambda) / (w_sum + alpha_tilde);
}
```

**Keyframeable properties**: MD 2025.2 added keyframeable shrinkage, solidify, and pressure. The solver crate exposes a `MaterialKeyframe` struct that the Tailor module uploads as an updated `MaterialParamsGpu` uniform at the start of each frame:

```rust
/// Per-frame material override for animated properties.
/// Merged with the base FabricMaterial at simulation time.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialKeyframe {
    pub frame: u32,
    pub shrinkage_weft: Option<f32>,
    pub shrinkage_warp: Option<f32>,
    pub solidify: Option<f32>,
    pub pressure: Option<f32>,
}
```

**Grain direction and weft/warp axis mapping**: The 2D pattern carries a `grain_angle_rad` that rotates the local UV coordinate frame before the weft/warp compliance lookup. This ensures that the fabric grain in the 2D pattern (e.g., a bias-cut at 45°) drives the correct anisotropic constraint axis on the GPU.

### [T-FABRIC-MODELS.preset-library] Preset Library Design

Presets are Postgres authority rows, not hardcoded constants. This allows the operator (and LLM authoring tools) to create, fork, and tune named fabric archetypes without code changes.

**Postgres schema (tailor_material_presets table)**:

```sql
CREATE TABLE IF NOT EXISTS tailor_material_presets (
    preset_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,
    slug                TEXT NOT NULL,          -- kebab-case, unique per workspace
    description         TEXT,
    compliance_json     JSONB NOT NULL,         -- ClothMaterialCompliance as JSON
    physics_json        JSONB NOT NULL,         -- ClothMaterialPhysics as JSON
    grain_angle_rad     DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    is_system_preset    BOOLEAN NOT NULL DEFAULT false,  -- true = bundled defaults
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, slug)
);
CREATE INDEX ix_tailor_material_presets_workspace
    ON tailor_material_presets (workspace_id, is_system_preset);
```

**Bundled system presets** (seeded via migration, `is_system_preset = true`):

```json
[
  {
    "slug": "cotton",
    "name": "Cotton",
    "compliance": {
      "stretch_weft": 5e-6, "stretch_warp": 4e-6, "stretch_shear": 2e-5,
      "bend_weft": 3e-3, "bend_warp": 3e-3,
      "buckling_ratio": 0.6, "buckling_stiffness": 0.0
    },
    "physics": {
      "density_kg_per_m2": 0.15, "collision_thickness_m": 0.0025,
      "friction": 0.4, "self_friction": 0.2, "internal_damping": 0.05,
      "air_drag": 0.01, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "silk",
    "name": "Silk",
    "compliance": {
      "stretch_weft": 2e-4, "stretch_warp": 1.5e-4, "stretch_shear": 8e-4,
      "bend_weft": 8e-2, "bend_warp": 8e-2,
      "buckling_ratio": 0.9, "buckling_stiffness": 0.0
    },
    "physics": {
      "density_kg_per_m2": 0.04, "collision_thickness_m": 0.001,
      "friction": 0.1, "self_friction": 0.05, "internal_damping": 0.02,
      "air_drag": 0.005, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "denim",
    "name": "Denim",
    "compliance": {
      "stretch_weft": 5e-8, "stretch_warp": 3e-8, "stretch_shear": 1e-7,
      "bend_weft": 5e-5, "bend_warp": 4e-5,
      "buckling_ratio": 0.1, "buckling_stiffness": 0.5
    },
    "physics": {
      "density_kg_per_m2": 0.35, "collision_thickness_m": 0.003,
      "friction": 0.55, "self_friction": 0.4, "internal_damping": 0.1,
      "air_drag": 0.02, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "leather",
    "name": "Leather",
    "compliance": {
      "stretch_weft": 5e-9, "stretch_warp": 5e-9, "stretch_shear": 2e-8,
      "bend_weft": 8e-6, "bend_warp": 8e-6,
      "buckling_ratio": 0.05, "buckling_stiffness": 0.8
    },
    "physics": {
      "density_kg_per_m2": 0.50, "collision_thickness_m": 0.004,
      "friction": 0.65, "self_friction": 0.5, "internal_damping": 0.15,
      "air_drag": 0.03, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "jersey",
    "name": "Jersey / Knit",
    "compliance": {
      "stretch_weft": 1e-4, "stretch_warp": 5e-5, "stretch_shear": 3e-4,
      "bend_weft": 2e-2, "bend_warp": 2e-2,
      "buckling_ratio": 0.85, "buckling_stiffness": 0.0
    },
    "physics": {
      "density_kg_per_m2": 0.13, "collision_thickness_m": 0.002,
      "friction": 0.35, "self_friction": 0.15, "internal_damping": 0.04,
      "air_drag": 0.008, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  }
]
```

Note: The compliance values in the preset table are calibration targets, not physical SI units. They must be validated empirically against the tailor-solver with standard test cases (drape test, cantilever test) because XPBD compliance is scale-dependent on mesh resolution and substep count. The `density_kg_per_m2` values are physically grounded (cotton 150 g/m² = 0.15 kg/m²).

**Per-pattern-piece override**: Each garment panel can reference a `preset_id` or carry an inline `FabricMaterial` override. The authority row is in `cloth_garment_panels.material_json JSONB`.

### [T-FABRIC-MODELS.kernel-binding] Handshake Kernel Primitive Binding

**EventLedger receipts**: Every preset mutation emits a `NewKernelEvent` via the existing builder pattern:

```rust
// In src/tailor/material.rs
pub async fn create_preset(
    pool: &PgPool,
    actor: KernelActor,
    task_run_id: &str,
    session_run_id: &str,
    req: CreateFabricPresetRequest,
) -> TailorResult<FabricPresetRow> {
    guard_authority_write(AuthorityMode::Postgres)?;
    
    // Insert preset row
    let row = sqlx::query_as!(FabricPresetRow, /* INSERT ... RETURNING */)
        .fetch_one(pool)
        .await?;
    
    // Emit EventLedger receipt
    let event = NewKernelEvent::builder(
        task_run_id,
        session_run_id,
        KernelEventType::TailorMaterialPresetRecorded,
        actor,
    )
    .aggregate("tailor_material_preset", &row.preset_id.to_string())
    .idempotency_key(&format!("tailor-preset-{}", row.preset_id))
    .payload(json!({
        "preset_id": row.preset_id,
        "workspace_id": req.workspace_id,
        "slug": req.slug,
        "name": req.name,
        "compliance_summary": {
            "stretch_weft": req.compliance.stretch_weft,
            "bend_weft": req.compliance.bend_weft,
        }
    }))
    .source_component("tailor::material")
    .build()?;
    
    insert_kernel_event(pool, event).await?;
    Ok(row)
}
```

**New KernelEventType variants** required for the Tailor material subsystem:

```rust
// Added to KernelEventType enum in kernel/mod.rs
TailorMaterialPresetRecorded,      // system preset seeded or user preset created
TailorMaterialPresetUpdated,       // property edit on an existing preset
TailorMaterialPresetDeleted,       // soft-delete of user preset
TailorGarmentMaterialAssigned,     // preset linked to a garment panel
TailorMaterialKeyframeRecorded,    // animated material property keyframe saved
```

**CRDT for collaborative preset editing**: If two operators edit the same preset concurrently, the CRDT layer merges via `CrdtUpdateRecordV1`. The garment material CRDT document type maps `preset_id -> crdt_document_id`. Conflict resolution strategy: last-write-wins per property key (each compliance scalar is an independent CRDT map entry), matching the YJS `Map` semantics already in the kernel's `yjs_bridge`.

**Sandbox validation of model-authored presets**: When the LLM authors a new preset via the model lane, the proposed `FabricMaterial` JSON enters the sandbox:

1. `TailorSandboxAdapter` runs a lightweight XPBD drape test (1-second drape of a 0.5m × 0.5m square mesh under gravity, 30 substeps).
2. `ValidationRunner` checks: (a) no NaN/Inf in particle positions, (b) final mesh bounding box within expected bounds for the claimed fabric type, (c) stretch compliance not zero (would make solver diverge), (d) density > 0.
3. `PromotionGate` accepts: emits `TailorMaterialPresetRecorded` event, writes Postgres row.
4. `PromotionGate` rejects: emits `TailorMaterialPresetRejected` event with `PromotionRejectionReason::ValidationFailed` and a diagnostic JSON payload naming the failed check.

**No-SQLite tripwire**: Every `tailor_material_presets` INSERT calls `guard_authority_write(AuthorityMode::Postgres)` before the sqlx query, mirroring `kb003_storage.rs` and the atelier domain.

### [T-FABRIC-MODELS.model-first-api] Model-First (LLM-Steerable) API Surface

The LLM interacts with fabric presets through structured tool calls via the `ModelAdapter` trait. The Tailor material tool definitions exposed through the MCP gate:

**Tool: `tailor_fabric_preset_create`**

```json
{
  "name": "tailor_fabric_preset_create",
  "description": "Create a new named fabric material preset with anisotropic XPBD compliance and physical properties. The preset will be validated by running a drape simulation before promotion to authority storage.",
  "input_schema": {
    "type": "object",
    "required": ["workspace_id", "name", "fabric_archetype"],
    "properties": {
      "workspace_id": { "type": "string" },
      "name": { "type": "string", "description": "Human-readable preset name, e.g. 'Heavy Cotton Canvas'" },
      "fabric_archetype": {
        "type": "string",
        "enum": ["cotton", "silk", "denim", "leather", "jersey", "wool", "rubber", "custom"],
        "description": "Base archetype to initialize compliance from; use 'custom' to specify all values manually."
      },
      "compliance_overrides": {
        "type": "object",
        "description": "Optional: override specific compliance values from the archetype defaults.",
        "properties": {
          "stretch_weft": { "type": "number" },
          "stretch_warp": { "type": "number" },
          "stretch_shear": { "type": "number" },
          "bend_weft": { "type": "number" },
          "bend_warp": { "type": "number" },
          "buckling_ratio": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
        }
      },
      "physics_overrides": {
        "type": "object",
        "properties": {
          "density_g_per_m2": { "type": "number", "minimum": 5.0, "maximum": 2000.0 },
          "friction": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
          "internal_damping": { "type": "number", "minimum": 0.0, "maximum": 1.0 }
        }
      }
    }
  }
}
```

**Tool: `tailor_fabric_preset_list`** — returns workspace presets as a structured JSON array with `preset_id`, `slug`, `name`, `is_system_preset`, and a `compliance_summary` with the key differentiating values (bend_weft, stretch_weft, density).

**Tool: `tailor_fabric_preset_fork`** — clone an existing preset by `preset_id` with a new name and optional property overrides.

**LLM authoring context**: When the model receives a `ContextBundle` for garment authoring, it includes the current workspace preset list, the garment's assigned panel materials, and a `FabricMaterialHint` derived from the garment description (e.g., "leather corset" → archetype: leather, high friction, low compliance). The model uses this context to select or tune presets without needing to reason about raw compliance numbers.

**Natural language to preset mapping**: The LLM tool call flow for "make this panel heavier denim":

1. LLM calls `tailor_fabric_preset_fork` with `source_preset_id = denim_system_preset`, `density_g_per_m2 = 450` (heavier), `bend_weft *= 1.5`.
2. `ModelAdapter` routes the tool call to `TailorModelAdapter::invoke()`.
3. `artifact_proposal` contains the proposed `FabricMaterial` JSON.
4. Sandbox runs the drape validation test.
5. On pass: `PromotionGate` writes the new preset row; EventLedger receipt emitted.
6. Response returns `preset_id` of the new "Heavy Denim" preset.
7. LLM calls `tailor_garment_panel_assign_material` to attach it to the target panel.

### [T-FABRIC-MODELS.risks] Risks and Open Questions

**Risk 1 — Compliance calibration is mesh-resolution-dependent.** XPBD compliance values are not in physical SI units; they depend on triangle edge length (particle distance). A cotton preset tuned at 5 mm particle distance will behave differently at 2 mm. The Tailor solver must normalize compliance by edge rest-length (or triangle area) at constraint initialization. The normalized formula is: `alpha_physical = alpha_raw / rest_length^2` for stretch, `alpha_physical = alpha_raw / rest_area` for bend. The preset library must store physical-unit stiffness and convert at constraint setup time, not store solver-space compliance directly. This is a significant design choice to lock in early.

**Risk 2 — True weft/warp orthotropic constraints require UV-aware mesh construction.** The weft/warp axis alignment depends on the grain direction in the 2D pattern. The solver must tag each edge and triangle with a `uv_axis` attribute computed from the 2D pattern UV coordinates at mesh generation time. Without this tagging, the anisotropic compliance lookup falls back to isotropic. UV-axis computation at mesh time is an upstream dependency on the meshing module (T-CLOTH-SOLVER).

**Risk 3 — Buckling ratio implementation is non-trivial.** MD's Buckling Ratio controls the angle threshold at which the reduced-stiffness bend term activates. This is a nonlinear bending model: stiffness is high for small angles (flat fabric) and drops at the buckling angle. Implementing this correctly in WGSL requires a conditional compliance lookup or a nonlinear bending constraint (not standard XPBD). The SIGGRAPH 2025 paper "XPBD Simulation of Constitutive Materials with Exponential Strain Tensor" (ACM DOI 10.1145/3769047.3769050) provides a constitutive model that may generalize this. Initial implementation can use linear bending and add buckling as a post-MVP enhancement.

**Risk 4 — Keyframeable properties require per-frame GPU buffer updates.** Animated shrinkage/solidify/pressure means the `MaterialParamsGpu` uniform must be re-uploaded every frame when animation is active. This is a small GPU transfer (64 bytes) but requires the simulation loop to check for active keyframe tracks and apply interpolated values. The keyframe storage must be in the EventLedger (as `TailorMaterialKeyframeRecorded` events) so the animation state is reproducible from authority storage alone.

**Risk 5 — Preset validation via drape test may be too slow for interactive editing.** A 1-second drape test with 30 substeps on a 0.5m² mesh at 5 mm particle distance (~2000 particles) takes roughly 100–500 ms on GPU. This is acceptable for promotion-gate validation but too slow for real-time material preview in the UI. The Tauri layer should expose a `cloth_preview_material` command that runs a much coarser preview (0.1-second, 200 particles, 10 substeps) directly without the sandbox/promotion pipeline.

**Open question — Density unit convention.** GarmentCode uses `density: 0.015` (no unit stated; likely kg/m² given the scale). MD uses g/m². The Handshake schema should standardize on kg/m² internally (the SI unit) and accept g/m² input from the LLM API (dividing by 1000 at the boundary). The LLM tool schema should document `density_g_per_m2` as the input unit to match operator mental models.

**Open question — Per-material-preset tack and trim stiffness.** MD's Tack tool applies point constraints between trims and garment fabric. Trim stiffness interacts with the garment panel material. The Handshake design should define whether tack/trim stiffness is a property on the trim object, on the attachment point, or on the garment panel material. Current recommendation: trim stiffness is a property of the `ClothTrimAttachment` struct, not the `FabricMaterial`, to keep material presets focused on cloth-only behavior.

### [T-FABRIC-MODELS.sources] Sources

- https://support.marvelousdesigner.com/hc/en-us/articles/47358420149017-FABRIC-PHYSICAL-PROPERTIES-Adjust-Stretch-Weft-Warp-Shear (MD stretch properties; verified 2026)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358432358809-FABRIC-PHYSICAL-PROPERTIES-Adjust-Bending-Weft-Warp (MD bending properties; verified 2026)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358420908313-FABRIC-PHYSICAL-PROPERTIES-Adjust-Buckling-Ratio (MD buckling ratio; verified 2026)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358443144985-FABRIC-PHYSICAL-PROPERTIES-Adjust-Buckling-Stiffness (MD buckling stiffness; verified 2026)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358440962713-FABRIC-PHYSICAL-PROPERTIES-Physical-Property-Preset (MD preset system; verified 2026)
- https://support.marvelousdesigner.com/hc/en-us/articles/47358303451289-8-Physical-Properties (MD physical properties overview; verified 2026)
- https://matthias-research.github.io/pages/publications/XPBD.pdf (Macklin et al. 2016 XPBD paper — compliance/stiffness formulation; alpha_tilde derivation)
- https://arxiv.org/abs/2212.08790 (Estimating Cloth Elasticity Parameters Using XPBD — compliance-to-physical mapping methodology)
- https://arxiv.org/html/2401.15169v1 (Yarn-level to shell-level elasticity estimation — orthotropic StVK stiffness matrix s00/s11/s22 framework)
- https://dl.acm.org/doi/10.1145/3769047.3769050 (XPBD constitutive materials with exponential strain tensor, SIGGRAPH MIG 2025 — nonlinear bending model reference)
- https://github.com/maria-korosteleva/GarmentCode (GarmentCode — Sim_props directory; stiff/soft/mid-bending preset values inspected)
- https://raw.githubusercontent.com/maria-korosteleva/GarmentCode/main/assets/Sim_props/stiff_ochra.json (Inspected: bend_resistance 2.0, stretch_resistance 400.0, warp/weft scale multipliers)
- https://raw.githubusercontent.com/maria-korosteleva/GarmentCode/main/assets/Sim_props/soft_ochra.json (Inspected: bend_resistance 0.03 — 67x softer than stiff preset)
- https://raw.githubusercontent.com/maria-korosteleva/GarmentCode/main/assets/Sim_props/default_sim_props.yaml (Inspected: Qualoth-format ke/kd/density parameters)
- https://github.com/nikhilr612/xpbdrs (Rust XPBD crate — XpbdParams API with length_compliance 0.001 default)
- https://github.com/vitalight/Velvet (CUDA XPBD cloth solver — Jacobi delta-accumulation pattern for GPU parallelism)
- https://github.com/ccincotti3/webgpu_cloth_simulator (WebGPU XPBD — WGSL constraint graph coloring; material compliance as GPU buffer uniform)
- https://github.com/jspdown/cloth (WebGPU XPBD small-step — 100% GPU execution reference for WGSL material parameter layout)
- https://github.com/RobDavenport/softy (Rust Verlet/PBD — DampingMode enum; structural/shear/bend VerletGrid)
- https://arxiv.org/html/2405.17609v2 (GarmentCodeData — fabric material distribution; three bending-tier approach; confirmed minimalistic anisotropy)
- https://onlinelibrary.wiley.com/doi/10.1111/cgf.15029 (Practical Methods to Estimate Fabric Mechanics from Metadata — KES-FB / FAST measurement to simulation parameter pipeline)
- https://andrewdcampbell.github.io/clothsim/ (CS184 cloth simulation — spring constant ks ranges 100–100000; damping 0–0.8%; bending scaled at 0.2× structural stiffness)
