---
file_id: cloth-engine-autofit-retargeting
topic_id: T-AUTOFIT
title: "Auto-Fit and Retargeting Across Body Morphs"
status: draft
depends_on:
  - T-GARMENT-AUTHORING
  - T-COLLISION
summary: "Handshake-native design for auto-fitting a garment authored on one body onto a different avatar, covering pattern grading, body-shape transfer, re-draping, UV/texture preservation, and the model-first refit API binding to kernel primitives."
sources: 34
updated_at: "2026-06-17"
---

## [T-AUTOFIT] Auto-Fit and Retargeting Across Body Morphs

### [T-AUTOFIT.md-features] Marvelous Designer Feature Mapping

Marvelous Designer addresses the garment-to-body reuse problem through four distinct mechanisms, each covering a different layer of the retargeting stack.

**Auto Fitting (2025.0 improved).** Automatically adjusts the size of 2D pattern pieces and re-simulates the garment on a target custom avatar. In MD's implementation the 2D panels are scaled (with seam-length ratios preserved) and the result is re-draped from scratch. A "Re-Target Draping / Re-Drape 3D Arrangement" option keeps pattern panel scale fixed and re-simulates the arrangement on the new body, useful when only pose or slight proportions change and the designer wants to preserve the original fit intention.

**Garment Fit Properties (2025.0).** Per-garment fitting behavior controls how tightly a garment wraps on a specific avatar, exposed as a per-garment property set. This decouples "design intent" from "body-specific fit margin," so the same garment design can be applied with different ease offsets to different avatars without re-authoring the patterns.

**Blend Shape Avatar (2026.0).** Avatars carrying blend-shape / morph-target data can have their body geometry changed in real time while a simulated garment re-drapes interactively. MD's stated intent is collision-driven body shape adjustment to avoid clothing penetration rather than facial expression — effectively real-time incremental retargeting via morph interpolation. This mirrors the USPTO patent art for "progressive drape update on avatar morph for virtual fitting" (US10249100B2), which describes draping garments across a continuous sequence of morphing shapes.

**2D Pattern Scale with 3D Shape Preservation (2026.0).** When scaling 2D pattern pieces to repurpose a garment across body proportions, the 3D garment shape is now preserved. This addresses the traditional problem where uniform 2D scaling produces proportionally wrong drape on a different figure.

**EveryWear Auto Fitting + Rig Transfer.** EveryWear (the CLO/MD game-optimization pipeline) includes an auto-fit stage that adjusts 2D patterns and 3D garments to a target avatar, followed by an automatic weight-painting rig pass that transfers the avatar's joint influence to the garment mesh. In 2026.0 an experimental Rig Template system automates joint creation specifically for wide-silhouette garments (skirts, capes) that would otherwise have poor skinning without helper joints.

The key moat MD holds is the **bidirectional 2D-to-3D loop during retargeting**: editing pattern scale feeds the solver, and the solver equilibrium state feeds back accurate boundary lengths and UV islands to the 2D editor. Open-source approaches address individual steps (pattern optimization, re-draping, UV recovery) but no single open-source pipeline closes this loop with real-time operator feedback.

---

### [T-AUTOFIT.oss-approaches] Open-Source Retargeting Approaches

#### DressAnyone — Differentiable-XPBD Pattern Optimization (ETH Zurich / Meta, 2024)

**Repo / paper:** `https://arxiv.org/abs/2405.19148`, project page `https://igl.ethz.ch/projects/dress_anyone/`  
**Language:** Python (PyTorch + XPBD differentiable layer)

DressAnyone is the most complete academic pipeline for automatic garment pattern refitting across body shapes. Its core innovation is gradient descent **through** a differentiable XPBD cloth simulator.

Pipeline:

1. Fit a statistical body model (SMPL) to the target body mesh to obtain a canonical A-pose proxy with consistent topology.
2. Compute an initial uniform scale factor from the target-to-reference area ratio.
3. Represent pattern deformation via a **Green-coordinate control cage**: `x̄ = W₁ζ + W₂n(ζ)` where `ζ` are control point positions and `W₁, W₂` are fixed weight matrices. This enforces conformal (shape-preserving) deformation of pattern panels with far fewer parameters than per-vertex optimization.
4. Iterate: drape the current pattern onto the target body via XPBD forward simulation to equilibrium; backpropagate gradients through the adjoint method; update control points.

Loss terms:

- **Target shape matching (`L_SM`):** boundary, seam, and interior vertex positions.
- **Boundary curvature (`L_curvature`):** prevents distortion of panel outline curvature under large deformations.
- **Pattern matching (`L_PM`):** equal seam edge lengths across mated panel pairs — prevents gathering artifacts at changed seams.
- **Total area (`L_TA`):** per-panel area match — important for loose garments prone to wrinkling.

Symmetry enforcement: for symmetric garments, gradient averaging is applied across bilaterally paired control points.

**Seam constraints** are honored at two levels: seam-line matching in `L_SM` and equal-length enforcement in `L_PM`. No ratio-sewing / M:N gather support in the published version.

**Runtime:** 5–30 minutes on CPU (AMD Threadripper PRO); the bottleneck is forward XPBD simulation plus adjoint backpropagation.

**Limitations:** single-layer only; A-pose only; cannot add new pattern pieces (e.g., extra sleeve panels needed for a radically different silhouette).

This is the direct algorithmic reference for the Handshake offline refit path.

#### NVIDIA Bolt — Transfer / Drape / Rig Pipeline (NVIDIA, Apr 2025)

**Paper:** `https://arxiv.org/pdf/2504.17614`  
**Language:** Python (Nvidia Warp GPU framework)

Bolt is an automated garment transfer pipeline designed for **at-scale** clothing of virtual characters — fitting outfits authored on a source body to thousands of new bodies. It runs three sequential stages.

**Stage 1 — Garment Transfer.** Each garment's 3D mesh is morphed from source to target via body-surface correspondences (skinning-weight transfer from the body rig). The 2D sewing pattern rest state is then optimized to match the transferred 3D positions, preserving seam shapes and boundary features.

**Stage 2 — Progressive Draping.** Garments are simulated layer by layer (innermost first) to untangle and drape each layer onto the body. XPBD is used for cloth physics via Nvidia Warp's GPU-accelerated PBD. "Progressive" means beginning with high stiffness and gradually relaxing to production parameters — this resolves deep interpenetrations without per-step manual intervention.

**Stage 3 — Rig Transfer.** Skinning weights from the body's skeletal rig are transferred to the garment mesh, producing a directly animatable rigged cloth asset.

The critical difference from DressAnyone is that Bolt operates in a **feed-forward** regime (no iterative gradient descent) to support high throughput, at the cost of less physically accurate pattern optimization for extreme body shapes.

#### Intersection-Free Garment Retargeting — PolyFEM Barrier Method (NYU GCL, SIGGRAPH 2025)

**Repo:** `https://github.com/Huangzizhou/cloth-fit`  
**Paper:** SIGGRAPH 2025 Conference Papers, Article 44 (`https://dl.acm.org/doi/10.1145/3721238.3730590`)  
**Language:** C++ (PolyFEM + PolySolve)

This work targets the hardest retargeting case: **non-human avatars with unrealistic body proportions** (e.g., goblin, fox-girl, T-rex), where SMPL-centric correspondence approaches break down.

Algorithm:

- Training-free; optimizes garment mesh directly in the mesh representation.
- Optimization objectives: (1) surface preservation (garment shape similarity), (2) curve preservation (curvature / torsion of boundary features like hem and cuff), (3) positional constraints (relative placement of design features), (4) contact **barrier** term preventing intersections.
- Barrier stiffness is a function of mesh resolution and geometry scale — higher near-contact distances (`dhat`) require larger barrier weight.
- Outputs simulation-ready garment models usable in downstream animation.
- Runtime: average 97 seconds across 54 body-garment combinations (avatars 2,000–6,000 vertices, garments 600–6,000 vertices).

The barrier method (IPC-style) is the key differentiator: it guarantees intersection-free output without post-processing, which is the safety property needed before a Handshake promotion gate can accept a retargeted garment.

#### GarmentCode + GarmentCodeData — Parametric Measurement-Driven Grading (ETH Zurich)

**Repo:** `https://github.com/maria-korosteleva/GarmentCode`  
**Body measurements:** `https://github.com/mbotsch/GarmentMeasurements`  
**Dataset paper:** `https://arxiv.org/html/2405.17609v2`

GarmentCode takes a different approach: instead of retargeting a draped mesh, it re-**programs** the pattern from body measurements. The `GarmentMeasurements` companion extracts 25 anthropometric measurements from a body mesh (bust, waist, hip circumference, shoulder width, arm length, inseam, etc.) via measurement planes aligned to body geometry. These drive GarmentCode's parametric pattern program, producing a freshly generated sewing pattern sized for the target body.

GarmentCodeData (115,000 data points, ECCV 2024) demonstrates this at scale:
- 5,000 sampled body shapes from a CAESAR-derived PCA statistical model.
- Per-body measurement extraction feeds into GarmentCode pattern programs.
- XPBD draping (Nvidia Warp GPU) from T-pose with a progressive initialization (gravity-free stitch establishment, then full physics to static equilibrium).
- Output: JSON sewing pattern + draped 3D mesh + UV segmentation.

This represents the **parametric grading** approach: bodies further from the reference are handled by re-parameterizing the design program, not by deforming an existing mesh. The trade-off is that garment style parameters (neckline shape, sleeve design, hem curve) must be re-specified as structured parameters, not transferred geometrically.

#### ChatGarment — LLM Material Parameter Estimation (CVPR 2025)

**Repo / site:** `https://chatgarment.github.io/`, `https://github.com/biansy000/ChatGarment`

ChatGarment demonstrates LLM-based material parameter estimation: given a text description or image of a fabric, an LLM scores four high-level descriptors (rigid/soft, heavy/light, wrinkle/smooth, perceived thickness), and these scores map deterministically to simulation parameters (stretching stiffness, bending stiffness, density, collision thickness). This is directly applicable to the Handshake refit API — when an operator says "refit this silk blouse to a curvier body," the model lane can re-estimate fabric parameters for the new drape and include them in the refit request payload.

---

### [T-AUTOFIT.design] Handshake-Native Design

The Handshake auto-fit module lives inside `src/tailor/` as part of the Tailor creative module. It does not introduce new kernel primitives; it wires existing ones together into a garment-refit pipeline.

#### [T-AUTOFIT.design.refit-modes] Refit Modes

Three refit modes are required, matching the three research paradigms above.

```rust
/// Placed in src/tailor/refit.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum RefitMode {
    /// Re-drape the existing garment on the target body without changing
    /// 2D pattern panels. Used when body proportions are close (pose change,
    /// minor morph). Fast: one XPBD forward pass.
    RedrapeOnly {
        target_body_id: Uuid,
        progressive_drape: bool,
    },
    /// Scale-and-redrape: apply a uniform or per-axis scale to 2D panels
    /// derived from body measurement ratios, then re-drape. Used for
    /// proportionally similar bodies with different overall size.
    ScaleAndRedrape {
        target_body_id: Uuid,
        ease_overrides: Option<EaseOverrideMap>,
    },
    /// Full differentiable optimization: gradient descent through XPBD
    /// to minimize DressAnyone-style composite loss. Slow (minutes offline).
    /// Used for significantly different body shapes or non-human avatars.
    OptimizePatterns {
        target_body_id: Uuid,
        max_iterations: u32,
        convergence_threshold: f32,
        preserve_seam_ratios: bool,
    },
}
```

#### [T-AUTOFIT.design.body-proxy] Body Shape Proxy

Body shapes are stored as `ClothBodyProxyV1` authority rows in PostgreSQL. The proxy format is body-model agnostic but carries enough structure for the solver crate to build collision geometry.

```rust
/// Stored as JSONB in tailor_body_proxies table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClothBodyProxyV1 {
    pub body_proxy_id: Uuid,
    pub workspace_id: String,
    /// Source format hint ("smpl", "metahuman", "custom_obj", "vrm", "gltf")
    pub source_kind: String,
    /// 25 anthropometric measurements extracted from the body mesh.
    /// Keys follow GarmentMeasurements naming: bust_circ_mm, waist_circ_mm,
    /// hip_circ_mm, shoulder_width_mm, arm_length_mm, inseam_mm, etc.
    pub measurements_mm: BTreeMap<String, f32>,
    /// Capsule set for fast XPBD collision (avatar body-segment approximation).
    /// Each capsule: {joint_name, p0: [f32;3], p1: [f32;3], radius_mm: f32}
    pub capsule_set: Vec<CollisionCapsuleV1>,
    /// Optional full low-resolution triangle mesh (~500 triangles) for mesh-mode
    /// collision. Stored as artifact_ref pointing to a Bundle artifact.
    pub lores_mesh_artifact_ref: Option<String>,
    /// Skinning joint hierarchy for rig-transfer (Bolt Stage 3).
    pub joint_hierarchy: Option<JointHierarchyV1>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionCapsuleV1 {
    pub joint_name: String,
    pub p0: [f32; 3],
    pub p1: [f32; 3],
    pub radius_mm: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointHierarchyV1 {
    pub joints: Vec<JointV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointV1 {
    pub name: String,
    pub parent: Option<String>,
    pub local_bind_transform: [[f32; 4]; 4],
}
```

The capsule set is the primary collision representation used in the XPBD solver during re-draping. It is computed from the body mesh at body proxy creation time, following the approach established in the cloth-simulation literature where body segments are approximated as capsule chains (hips, torso, shoulders, upper/lower arms, upper/lower legs, head/neck). The regressor from body shape parameters to capsule parameters described in the garment fitting literature is implemented as a post-processing step in the body proxy builder.

#### [T-AUTOFIT.design.measurement-extraction] Body Measurement Extraction

Measurement extraction follows the GarmentCodeData approach: a measurement builder ingests a body mesh OBJ and emits the `measurements_mm` map. The builder is a standalone function in the solver crate (no PostgreSQL dependency) that the Tailor module calls at body proxy creation time.

```rust
// In tailor-solver crate: src/body/measurements.rs

/// Extract 25 standard anthropometric measurements from a body mesh.
/// Measurement planes are optimized within ±2 mm of their initial position
/// following the GarmentCodeData fit-aware method.
pub fn extract_measurements(mesh: &TriMesh) -> Result<BTreeMap<String, f32>, MeasurementError> {
    // 1. Orient mesh to anatomical A-pose axes.
    // 2. Locate landmark vertices: top-of-head, left/right shoulder tips,
    //    left/right hip crests, knee centroids, ankle centroids, crotch.
    // 3. For circumference measurements: find horizontal plane at body-height
    //    fraction; slice mesh; compute perimeter of the cross-section ring.
    //    Adjust plane ±2 cm to find local circumference minimum/maximum per
    //    measurement spec.
    // 4. For linear measurements: compute geodesic distance between landmark
    //    pairs (inseam, arm length) or Euclidean (shoulder width, torso height).
    todo!("implement: landmark detection + plane-slice circumference + geodesic linear")
}
```

The extracted measurements feed into two consumers: (a) the parametric grading path (derive per-panel scaling ratios), and (b) the GarmentCode JSON generator for fully re-programmed patterns.

#### [T-AUTOFIT.design.pattern-grading] Pattern Grading — Scale-and-Redrape Path

For the `ScaleAndRedrape` mode the engine computes per-panel scale factors from measurement ratios between source and target body proxies.

```rust
// src/tailor/refit.rs

/// Compute per-panel scale factors from source and target measurements.
/// Returns (width_scale, height_scale) per panel_id.
pub fn compute_panel_scales(
    source_measurements: &BTreeMap<String, f32>,
    target_measurements: &BTreeMap<String, f32>,
    ease_overrides: &EaseOverrideMap,
    panels: &[GarmentPanelV1],
) -> BTreeMap<String, PanelScale> {
    // Horizontal scale: driven by circumference measurements.
    //   bust_scale = (target.bust_circ_mm + ease.bust) / (source.bust_circ_mm + ease.bust)
    //   hip_scale, waist_scale derived similarly.
    // Vertical scale: driven by inseam, torso height, arm length.
    // Per-panel assignment: each panel is tagged with a body-region
    //   (bodice_front, bodice_back, sleeve, skirt, trouser_leg, etc.)
    //   and the corresponding measurement pair is used.
    // Seam ratios (1:N) are preserved: only rest-length changes, not
    //   the seam constraint graph topology.
    todo!()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelScale {
    pub width_scale: f32,
    pub height_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EaseOverrideMap {
    /// Ease added per body region in mm. Keyed by region name.
    pub ease_mm: BTreeMap<String, f32>,
}
```

Scaled panels are written back as a new `GarmentDraftV1` row (a derived draft, not replacing the source), emitting a `TailorDraftScaled` EventLedger event.

#### [T-AUTOFIT.design.drape-initialization] Re-Drape Initialization

All three refit modes converge on a re-drape step. Correct initialization is critical: a garment placed in an interpenetrating configuration rarely untangles correctly under standard XPBD constraint solving.

Initialization strategy (following GarmentCodeData + Bolt Stage 2):

1. **Gravity-free stitch pass:** hold panel centroids at approximate target body surface positions (semantic segmentation: bodice panels → torso centroid, sleeve → arm centroid, skirt → hip centroid), enforce stretch and seam constraints without gravity, run for 50 substeps. Panels settle into approximate closed shape without falling.
2. **Stiffened physics pass:** apply full gravity and collision but with stretch stiffness ×10 and bending stiffness ×10. Run until <5% of vertices are moving. This resolves coarse interpenetrations without tearing geometry.
3. **Full physics pass:** drop stiffness to designed material parameters, run to equilibrium (< 1.5% of vertices moving > 0.04 cm between frames), max 2,400 frames.

This three-pass progressive relaxation pattern is implemented as a `RefitDrapeStrategy` in the solver crate and invoked from the `TailorSandboxAdapter`.

```rust
// tailor-solver crate: src/refit/drape_init.rs

pub struct RefitDrapeStrategy {
    pub gravity_free_substeps: u32,      // default: 50
    pub stiffened_damping_factor: f32,   // default: 10.0
    pub stiffened_convergence_pct: f32,  // default: 0.05
    pub full_physics_convergence_pct: f32, // default: 0.015
    pub max_frames: u32,                 // default: 2400
    pub timeout_secs: f32,               // default: 300.0
}
```

#### [T-AUTOFIT.design.blend-shape] Blend-Shape Incremental Refit

For the blend-shape / morph-target avatar use case (MD 2026.0 feature equivalent), the engine supports incremental refit: the body proxy changes via a blend parameter `t ∈ [0, 1]` interpolating between a base capsule set and a morphed capsule set. The solver re-drapes incrementally from the previous equilibrium position rather than from scratch, reducing simulation time proportionally to the blend delta.

```rust
// src/tailor/refit.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendShapeRefitRequest {
    pub garment_id: Uuid,
    pub base_body_proxy_id: Uuid,
    pub target_body_proxy_id: Uuid,
    /// Interpolation parameter. 0.0 = base, 1.0 = target.
    pub blend_t: f32,
    /// If true, warm-start solver from the last equilibrium state of the
    /// base body drape. Requires the prior simulation output to be in cache.
    pub warm_start: bool,
}
```

The warm-start flag passes the prior simulation particle positions to the XPBD solver's initial state buffer, exploiting temporal coherence exactly as a game engine would when animating a morph target.

#### [T-AUTOFIT.design.uv-preservation] UV / Texture Preservation After Refit

MD's moat-6 feature (UV islands = exact flattened 2D pattern pieces) gives physically accurate fabric grain direction. After retargeting, the UV islands must be recomputed from the new pattern panels to preserve this property.

The UV recompute pass runs as a post-simulation step in the solver crate:

```rust
// tailor-solver crate: src/uv/unfurl.rs

/// Flatten a simulated 3D panel to 2D UV space using as-rigid-as-possible
/// (ARAP) energy minimization, preserving local area at the expense of
/// global shape (acceptable because seam boundaries are pinned).
///
/// Input: 3D triangle mesh of one panel after simulation equilibrium.
/// Output: 2D UV coordinates for each vertex, in pattern space [0,1]^2.
pub fn arap_unfurl_panel(panel: &Panel3D) -> Result<Vec<[f32; 2]>, UnfurlError> {
    // 1. Pin boundary vertices at their initial 2D pattern positions (from
    //    the pre-simulation panel geometry).
    // 2. Minimize ARAP energy: sum over triangles of ||J_k - R_k||_F^2
    //    where J_k is the Jacobian of the deformation and R_k is the closest
    //    rotation. Solved via alternating local (SVD per triangle) + global
    //    (sparse linear system) steps.
    // 3. Return UV array indexed by the panel's vertex array.
    todo!()
}
```

The ARAP unfurl ensures the UV island matches the physically simulated garment surface rather than the flat pre-sim pattern. Texture maps authored on the original UV island are still valid because the pattern boundary vertices are pinned — only interior UV positions are adjusted. For garments with graphics layers (printed text, logos), the graphics layer position in 2D pattern space is preserved unchanged.

#### [T-AUTOFIT.design.postgres-schema] PostgreSQL Schema

```sql
-- Migration: tailor_body_proxies
CREATE TABLE tailor_body_proxies (
    body_proxy_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id         TEXT NOT NULL,
    source_kind          TEXT NOT NULL,                  -- 'smpl', 'custom_obj', 'vrm', 'gltf'
    measurements_json    JSONB NOT NULL DEFAULT '{}'::jsonb,
    capsule_set_json     JSONB NOT NULL DEFAULT '[]'::jsonb,
    lores_mesh_artifact_ref TEXT,
    joint_hierarchy_json JSONB,
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX tailor_body_proxies_workspace_idx ON tailor_body_proxies(workspace_id);

-- Migration: tailor_refit_runs
CREATE TABLE tailor_refit_runs (
    refit_run_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id           UUID NOT NULL,
    source_body_proxy_id UUID REFERENCES tailor_body_proxies(body_proxy_id),
    target_body_proxy_id UUID NOT NULL REFERENCES tailor_body_proxies(body_proxy_id),
    refit_mode           TEXT NOT NULL,                  -- 'redrape_only', 'scale_and_redrape', 'optimize_patterns'
    status               TEXT NOT NULL DEFAULT 'requested',
    sandbox_run_id       UUID,                           -- FK to kb003_sandbox_runs when promoted
    output_garment_id    UUID,                           -- FK to tailor_garments (promoted result)
    scale_factors_json   JSONB,
    optimization_params_json JSONB,
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

#### [T-AUTOFIT.design.eventledger] EventLedger Events

New `KernelEventType` variants for the refit lifecycle:

```rust
// Additions to KernelEventType in kernel/mod.rs

// Body proxy lifecycle
BodyProxyCreated,           // operator imports a new avatar
BodyProxyMeasurementsExtracted, // automated measurement extraction completed

// Refit request lifecycle
TailorRefitRequested,       // operator or model requests a refit
TailorRefitPatternScaled,   // scale_and_redrape: scaled panels computed
TailorRefitDrapeStarted,    // XPBD re-drape began in sandbox
TailorRefitDrapeCompleted,  // XPBD re-drape completed
TailorRefitOptimizationStep, // optimize_patterns: one gradient iteration
TailorRefitUvRecomputed,    // ARAP unfurl pass completed
TailorRefitValidated,       // validation gate passed
TailorRefitPromoted,        // output garment promoted to authority
TailorRefitRejected,        // refit failed validation (topology, intersection)
```

Events emitted via `NewKernelEvent::builder(task_run_id, session_run_id, event_type, actor).aggregate("tailor_refit_run", refit_run_id).idempotency_key(...).payload(json!({...})).build()`.

#### [T-AUTOFIT.design.sandbox] Sandbox Integration

Refit runs execute inside the existing `SandboxAdapter` / `SandboxRunV1` lifecycle, following the same pattern as model-authored garment simulation (T-COLLISION dependency).

The `TailorSandboxAdapter` implements `SandboxAdapter`:

```rust
// src/tailor/sandbox_adapter.rs

pub struct TailorSandboxAdapter {
    solver: Arc<dyn ClothSolver>,
    refit_mode: RefitMode,
}

impl SandboxAdapter for TailorSandboxAdapter {
    fn kind(&self) -> AdapterKind {
        AdapterKind::process_tier("tailor_refit_v1", "Tailor Refit Adapter")
    }

    fn run(
        &self,
        run: &SandboxRunV1,
        workspace: &SandboxWorkspaceV1,
        policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError> {
        // 1. Deserialize RefitRequest from run.payload_json.
        // 2. Load garment panels and source body proxy from sandbox workspace.
        // 3. Dispatch to solver crate:
        //    a. compute_panel_scales() if ScaleAndRedrape
        //    b. run drape initialization (3-pass progressive relaxation)
        //    c. run XPBD forward sim to equilibrium
        //    d. arap_unfurl_panel() for each panel (UV recompute)
        //    e. check intersection-free (all particle-body distances > 0)
        // 4. Bundle output: garment mesh JSON + UV map + refit_run metadata.
        // 5. Return AdapterRunOutcome::Completed { artifact_refs: [bundle_ref] }
        todo!()
    }
}
```

The no_sqlite_tripwire must be called before any authority write inside the adapter, identical to `kb003_storage.rs`.

#### [T-AUTOFIT.design.validation] Validation Gate

The `ValidationRunner` checks refit output before the `PromotionGate` can accept it. Tailor-specific validation checks for refit runs:

```rust
// src/tailor/validation.rs

pub fn validate_refit_output(bundle: &RefitOutputBundle) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();

    // 1. Intersection check: no garment particle inside any body capsule.
    //    Fail if min(particle-capsule distance) < -0.5 mm.
    findings.extend(check_intersection_free(&bundle.particle_positions, &bundle.capsules));

    // 2. Seam closure: all mated seam edge pairs have length difference < 1%.
    findings.extend(check_seam_closure(&bundle.panels, &bundle.seams));

    // 3. UV validity: all UV coordinates in [0,1]^2, no degenerate triangles
    //    (area > 1e-6 in UV space).
    findings.extend(check_uv_validity(&bundle.uv_maps));

    // 4. Mesh topology: no non-manifold edges, no isolated vertices.
    findings.extend(check_mesh_topology(&bundle.panels));

    // 5. Convergence: simulation reached equilibrium (not timed out).
    if !bundle.converged {
        findings.push(ValidationFinding::warn("refit_convergence",
            "simulation did not converge within timeout; result may be inaccurate"));
    }

    findings
}
```

A refit output with any `Error`-severity finding is rejected by the `PromotionGate` and a `TailorRefitRejected` event is emitted. Warning-severity findings are recorded but do not block promotion.

#### [T-AUTOFIT.design.crdt] CRDT for Retargeted Garment Drafts

Each refit that produces a new garment draft is a new CRDT document. Multiple agents can propose competing refits of the same source garment to the same target body (e.g., one using `ScaleAndRedrape`, another using `OptimizePatterns`). The CRDT layer tracks each proposal as a separate actor-site document. The operator chooses which version to promote.

```rust
// CRDT document type for a retargeted garment draft.
// Uses existing CrdtUpdateRecordV1 table; no new CRDT infrastructure needed.

let crdt_doc_id = format!("refit-draft:{}", refit_run_id);
// Update record committed after each optimization iteration (OptimizePatterns mode)
// or after scale computation (ScaleAndRedrape mode).
```

---

### [T-AUTOFIT.model-first-api] Model-First Refit API

The LLM-steerable refit API surfaces as a `TailorModelAdapter` implementing `ModelAdapter`.

```rust
// src/tailor/model_adapter.rs

/// ContextBundle fields for a garment refit request.
/// The model receives these as structured context and must emit a
/// RefitRequest JSON as artifact_payload.
///
/// context_bundle.metadata fields (required by model):
///   garment_id: UUID of the source garment
///   source_body_proxy_id: UUID (optional; if absent the model chooses)
///   target_body_proxy_id: UUID or target_body_description (string)
///   refit_intent: "fit_to_new_body" | "scale_up" | "scale_down" | "non_human_retarget"
///   fabric_material_description: optional string ("silk blouse", "denim jacket")
///
/// Model output (artifact_payload):
///   refit_mode: RefitMode enum value
///   ease_overrides: optional EaseOverrideMap
///   max_iterations: optional u32 (OptimizePatterns only)
///   material_params: optional updated fabric params (ChatGarment-style estimation)
pub struct TailorModelAdapter {
    llm_client: Arc<dyn LlmClient>,
}
```

The model lane call follows the standard `LlmClient::completion()` path from `src/llm/mod.rs`. The context bundle carries the garment's pattern JSON, the source and target body proxy measurements, and an intent hint. The model responds with a `RefitRequest` JSON:

```json
{
  "refit_mode": "scale_and_redrape",
  "ease_overrides": {
    "ease_mm": {
      "bust": 20.0,
      "hip": 30.0,
      "waist": 15.0
    }
  },
  "material_params_override": {
    "bending_weft": 0.6,
    "density_gm2": 95.0
  },
  "rationale": "Target bust is 8 cm larger. Scaling bodice panels by bust ratio. Silk parameters preserved."
}
```

This output is validated against a JSON schema before the refit sandbox run is created. The model does not directly write to any authority surface; its output becomes a `TailorRefitRequested` EventLedger event and the sandbox run takes it from there.

**Model-steerable material re-estimation after refit** follows the ChatGarment pattern: four LLM-scored descriptors (rigid/soft, heavy/light, wrinkle/smooth, thickness) map to XPBD compliance parameters. This is relevant when the operator changes the fabric material as part of the retargeting ("same dress design, but now in denim for the bigger avatar").

---

### [T-AUTOFIT.risks] Risks and Open Questions

**Risk 1 — Convergence failures for extreme body shapes.** Non-human avatars with very wide shoulders, extreme height ratios, or non-humanoid limb count will fail the progressive drape initialization. Mitigation: expose the `RefitDrapeStrategy` parameters in the refit request; surface convergence state as a warning-severity validation finding; never silently return a timed-out simulation as a converged result.

**Risk 2 — Single-layer limitation (DressAnyone).** The differentiable optimization path does not handle multi-layer garments. Mitigation for Handshake: treat multi-layer retargeting as sequential single-layer runs (innermost to outermost), following Bolt's Stage 2 layering order. Store layer order in the garment authority row.

**Risk 3 — ARAP UV recompute diverges from original texture intent.** If panel deformation is large (> 30% area change), ARAP interior UV positions may diverge enough to misplace graphic overlays. Mitigation: run UV validation check; warn if any UV displacement exceeds a threshold; provide a "lock graphics layer" option that pins graphic-anchor vertices and only recomputes non-anchored interior UVs.

**Risk 4 — Measurement extraction reliability.** The 25-measurement extraction pipeline depends on landmark detection from the body mesh. Meshes with unusual topology (non-manifold, multiple connected components) may produce bad measurements. Mitigation: validate mesh at body proxy creation time; fail fast with a descriptive error before writing the proxy to PostgreSQL.

**Risk 5 — Capsule set mismatch for non-humanoid avatars.** The capsule regressor is tuned for humanoid body segments. A quad-body avatar, a snake avatar, or a robot chassis will not be approximated correctly by a biped capsule chain. Mitigation: allow operators to supply a custom capsule set JSON at body proxy creation time; fall back to a coarse convex hull decomposition (via `parry3d::shape::ConvexHull`) when no capsule set is provided. Flag non-humanoid proxies with `source_kind = "non_humanoid"` for the solver.

**Risk 6 — GPU divergence between offline optimization (CPU adjoint) and runtime XPBD.** The DressAnyone optimization runs on CPU (adjoint method); the runtime solver uses GPU WGSL compute. If the two implementations produce numerically different equilibrium positions, the optimized pattern may not match runtime drape. Mitigation: use the GPU solver for both optimization substeps and runtime draping; expose a `use_gpu_for_optimization` flag; validate against CPU reference at test time.

**Open Question A — Seam ratio preservation under large deformation.** When optimizing patterns for a very different body shape, M:N ratio seams (gather constraints) should change their rest-length scaling proportionally, not their gather ratio. The correct behavior needs to be specified precisely and tested against a gather garment (e.g., a gathered skirt) across body sizes.

**Open Question B — Non-human avatar capsule set authority.** Who owns the capsule set for a non-humanoid avatar — the operator (manual definition) or an automated decomposer? The decision affects what validation is possible before promotion. Defer to operator-supplied capsule sets as the authority path; auto-decomposition as a fallback hint only.

**Open Question C — Offline optimize-patterns throughput.** At 5–30 minutes per refit (DressAnyone runtime), the `OptimizePatterns` mode is not suitable for real-time interactive use. The sandbox `policy.timeout_secs` must be set to at least 1800 seconds (30 minutes) for this mode. Consider whether to expose a "fast approximate" variant using the Bolt-style feed-forward approach before offering the full gradient-descent path.

---

### [T-AUTOFIT.sources] Sources

- `https://arxiv.org/abs/2405.19148` — DressAnyone: Automatic Physically-Based Garment Pattern Refitting (ETH Zurich / Meta, 2024/2025)
- `https://arxiv.org/html/2405.19148v1` — DressAnyone HTML full paper (technical pipeline details used above)
- `https://igl.ethz.ch/projects/dress_anyone/` — DressAnyone project page
- `https://arxiv.org/pdf/2504.17614` — Bolt: Clothing Virtual Characters at Scale (NVIDIA, Apr 2025)
- `https://github.com/Huangzizhou/cloth-fit` — Intersection-Free Garment Retargeting (SIGGRAPH 2025, C++/PolyFEM)
- `https://dl.acm.org/doi/10.1145/3721238.3730590` — Intersection-Free Garment Retargeting SIGGRAPH 2025 ACM DL entry
- `https://huangzizhou.github.io/research/cloth.html` — Intersection-Free Garment Retargeting project page
- `https://github.com/maria-korosteleva/GarmentCode` — GarmentCode parametric sewing pattern framework
- `https://github.com/mbotsch/GarmentMeasurements` — GarmentMeasurements body measurement extraction companion
- `https://arxiv.org/html/2405.17609v2` — GarmentCodeData: 3D Made-to-Measure Garments (ECCV 2024) — pipeline details
- `https://arxiv.org/abs/2504.20409` — GarmentX: Autoregressive Parametric Garment Generation (2025)
- `https://arxiv.org/html/2412.17811v1` — ChatGarment: LLM material parameter estimation (CVPR 2025)
- `https://chatgarment.github.io/` — ChatGarment project page
- `https://support.marvelousdesigner.com/hc/en-us/articles/47358335130649-Auto-Fitting` — MD Auto Fitting feature doc
- `https://support.marvelousdesigner.com/hc/en-us/articles/47358145163033-Garment-Fit-Properties-ver-2025-0` — MD Garment Fit Properties
- `https://support.marvelousdesigner.com/hc/en-us/articles/55654243482649-Blend-Shape-Avatar` — MD Blend Shape Avatar (2026.0)
- `https://support.marvelousdesigner.com/hc/en-us/articles/55837641308313-Marvelous-Designer-2026-0-New-Feature-List` — MD 2026.0 New Feature List
- `https://www.cgchannel.com/2026/04/clo-virtual-fashion-releases-marvelous-designer-2026-0/` — CG Channel MD 2026.0 overview
- `https://support.marvelousdesigner.com/hc/en-us/articles/47358431264537-Morph-Target-with-AVT-Files` — MD Morph Target / AVT
- `https://connect.clo-set.com/everywear` — EveryWear product page (auto-fitting + rig transfer)
- `https://arxiv.org/abs/2311.12194` — DiffAvatar: Differentiable Simulation body+garment co-optimization (CVPR 2024)
- `https://arxiv.org/abs/2301.01396` — DiffXPBD: Differentiable XPBD (ACM TOG 2023)
- `https://arxiv.org/pdf/2502.03449` — Dress-1-to-3: differentiable XPBD for pattern refinement (2025)
- `https://github.com/vitalight/Velvet` — Velvet CUDA XPBD cloth engine (spatial hashing, Jacobi iteration)
- `https://github.com/dimforge/parry` — parry3d: Rust collision detection (ConvexHull, capsule shapes for body proxies)
- `https://github.com/gfx-rs/wgpu` — wgpu: Rust cross-platform GPU compute backend
- `https://github.com/Shenfu-Research/GarmentDiffusion` — GarmentDiffusion: edge-token diffusion pattern generation (IJCAI 2025)
- `https://arxiv.org/abs/2602.20700` — NGL-Prompter: Natural Garment Language training-free VLM (Feb 2026)
- `https://github.com/DavidBoja/Landmarks2Anthropometry` — 3D body measurement estimation from sparse landmarks (VISAPP 2024)
- `https://arxiv.org/html/2602.16502v1` — DressWild: feed-forward pose-agnostic garment pattern generation (2026)
- `https://arxiv.org/pdf/2312.08386` — PerfectTailor: scale-preserving 2D pattern adjustment driven by 3D garment editing
- `https://pcs-sim.github.io/pcs-main.pdf` — Progressive Simulation for Cloth Quasistatics (SIGGRAPH-Asia)
- `https://arxiv.org/abs/2504.21476` — Bolt at scale reference (NVIDIA Warp GPU pipeline)
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/model_adapter.rs` — ModelAdapter trait, ArtifactProposalDraft, ModelAdapterOutput
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/sandbox/adapter.rs` — SandboxAdapter trait, AdapterRunOutcome, AdapterKind
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/atelier/mod.rs` — Atelier domain pattern (PgPool, EventLedger events, domain submodules)
