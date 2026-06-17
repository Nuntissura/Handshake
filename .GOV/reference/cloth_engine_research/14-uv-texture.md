---
file_id: cloth-engine-t-uv-texture
topic_id: T-UV-TEXTURE
title: "UV-from-Pattern, Texturing, and PBR Maps"
status: draft
depends_on:
  - T-GARMENT-AUTHORING
  - T-FABRIC-MODELS
  - T-AUTOFIT
  - T-RENDER-VIEWPORT
summary: "UV islands as exact flattened 2D pattern pieces (MOAT-6): ARAP post-simulation unfurl, UV packing, graphic layer data model, PBR map generation, grain-direction-to-UV binding as authority, and the Postgres tailor_* texture/material schema."
sources: 28
updated_at: "2026-06-17"
---

## [T-UV-TEXTURE] UV-from-Pattern, Texturing, and PBR Maps

This document consolidates the UV, texturing, and PBR map generation design for the Tailor engine.
It closes MD MOAT-6 ("UV-from-pattern accuracy") and resolves the flatten-algorithm contradiction
scattered across T-GARMENT-AUTHORING and T-AUTOFIT. It also defines the graphic layer data model,
PBR material pipeline, grain-direction authority binding, and the Postgres `tailor_*` texture schema
that serves as the canonical material authority surface.

**Key design decisions resolved in this document:**

1. UV islands = the exact 2D panel boundaries carried through the mesh pipeline; no unwrapping pass
   required at mesh generation time because the pattern IS the UV.
2. The post-simulation flatten pass (bidirectional loop feedback and refit UV recompute) uses ARAP,
   not LSCM or ABF++. Rationale in [T-UV-TEXTURE.flatten-algorithm-decision].
3. UV packing uses `rectangle-pack` (deterministic, no image dependency) for layout computation;
   rendering always uses the packed atlas coordinates.
4. PBR map generation is a Tauri command that runs on the operator workstation, not in the solver
   crate; the solver crate produces no PBR maps.
5. Fabric grain direction is authority data on `PanelSpec::grain_angle_deg`; it controls both the
   XPBD anisotropic constraint axis (T-FABRIC-MODELS) and the UV coordinate frame rotation.

---

### [T-UV-TEXTURE.md-feature-map] Marvelous Designer Feature Mapping

**Source: T-MD-FEATURES Group 8 (UV, Texturing, Rendering).**

| MD Feature | MOAT? | Tailor mapping |
|---|---|---|
| UV Editor mode — pattern pieces auto-placed as UV islands | MOAT-6 | UV islands = 2D panel vertices carried through mesh pipeline; no post-hoc unwrapping |
| Automatic UV Packing | — | `rectangle-pack` Rust crate; deterministic axis-aligned packing |
| UV Editor Filtering for Selected Garments (2026.0) | — | Filter `tailor_uv_islands` rows by garment_id |
| Side and Back UV Expansion (2025.0) | — | All panels get UV islands at mesh generation time; no distinction front/back/side |
| Grain Direction — per-pattern grainline angle entry | — | `PanelSpec::grain_angle_deg` is authority; rotates UV frame in solver + texture sampler |
| UV Bake Textures — Diffuse, Normal, Opacity export | — | `tailor_capture_frame` + `export_garment` with UV-bake mode; Tauri command |
| Edit Texture 2D — graphic layer on 2D pattern | — | `TailorGraphicLayerV1` placed in panel-local 2D space; stored in Postgres |
| Fabric texture + graphic layer | — | `tailor_material_assignments` links preset + graphic layers to panel |
| PBR material — Metallic, Roughness, Emissive, Normal, Opacity | — | `TailorPbrMaterialV1` schema; stored in `tailor_pbr_materials` |
| PBR Map Generator (2024.1) | — | CPU-side normal/roughness/metalness/displacement generation from diffuse; Tauri command |
| Blend Graphic with Fabric (2025.0) | — | `TailorGraphicLayerV1::blend_mode` field |
| Recolor Base Color Map on Export (2025.0) | — | Export pipeline applies `recolor_map` transform before writing texture files |
| Fur Strand Material (experimental) | — | Out of scope for MVP |
| Toon Shader (2026.0) | — | wgpu Toon pass in viewport; not a UV/texture concern |
| All-Quad Mesh Conversion (2025.0) | — | Post-sim quad conversion deferred post-v1; UV islands survive |

**MOAT-6 statement (from T-MD-FEATURES):**

> UV islands are exact flattened 2D pattern pieces. Fabric texture grain direction is always
> physically accurate because the UV matches the sewing pattern, not an unwrapped 3D mesh.
> Replicating this requires an unfurl/flatten post-simulation pass that no other tool performs
> automatically. `[D4]`

The Tailor engine addresses MOAT-6 at two points:

- At **mesh generation time**: UV coordinates assigned directly from 2D panel-local vertex positions.
  This is trivially correct and zero-cost — the pattern IS the UV; no parameterization algorithm runs.
- At **post-simulation time** (bidirectional loop or refit): a ARAP flatten pass recomputes UV
  positions to match the physically simulated surface. This is the `[D4]` hard part.

---

### [T-UV-TEXTURE.uv-from-pattern-mechanics] UV-from-Pattern: Core Mechanics

#### [T-UV-TEXTURE.uv-from-pattern-mechanics.what-it-means] What "UV islands = pattern" means

In conventional 3D UV unwrapping, UV coordinates are computed by an algorithm (LSCM, ABF++, smart
UV project, etc.) that cuts and flattens the 3D surface with minimal distortion. This decouples UV
from the sewing pattern. If you paint a fabric weave texture onto the 3D UV, the warp/weft
directions will be approximately correct but can drift depending on how the unwrap algorithm
handles curvature and seam placement.

In MD, and in the Tailor engine, the UV is not computed from the 3D surface: **the UV is the 2D
pattern.** Panel vertices in panel-local 2D space directly become UV coordinates. The 3D mesh is
derived from the 2D pattern (via the pattern-to-mesh pipeline in T-GARMENT-AUTHORING); the UV
coordinates travel alongside every vertex through that derivation and through the solver. They are
never re-derived from the 3D shape.

This gives several properties for free:

- **Grain direction accuracy**: the grain angle on the 2D pattern is the grain angle in UV space.
  No re-alignment needed.
- **Seam consistency**: vertices on both sides of a seam get UV coordinates from their respective
  2D panel origins. After the seam constraint closes the seam, the UV boundary matches the seam
  boundary in 3D. Texture seams align with physical seams.
- **Graphic layer preservation**: a graphic layer (logo, print, embroidery) placed at 2D pattern
  coordinates maps correctly to the 3D garment surface without reprojection.
- **Reuse after refit**: when a garment is retargeted to a different body, the post-ARAP-unfurl
  pass recomputes UV for the new drape while keeping boundary UVs pinned — graphic layer anchors
  survive if designed against the boundary vertices.

#### [T-UV-TEXTURE.uv-from-pattern-mechanics.mesh-gen-uv-assignment] UV assignment at mesh generation

Extending the `pattern-to-mesh` pipeline (T-GARMENT-AUTHORING):

```rust
// src/tailor/solver_binding.rs (addition to existing pipeline)

/// Assign UV coordinates to all vertices during panel triangulation.
/// UV coordinates are the normalized 2D panel-local vertex positions.
///
/// Normalization: each panel is normalized independently to [0,1]^2
/// preserving the panel's aspect ratio. The bounding box of the panel
/// vertex set is the normalization domain.
///
/// This means UV(0,0) = bottom-left corner of the panel bounding box,
/// UV(1,1) = top-right corner. Grain angle is applied in the texture
/// sampler, not in UV space (see TailorPbrMaterialV1::grain_angle_deg).
pub fn assign_panel_uvs(
    panel_local_vertices: &[[f32; 2]],  // vertices in 2D panel-local space (cm)
    panel_id: &str,
) -> Vec<[f32; 2]> {
    let min_x = panel_local_vertices.iter().map(|v| v[0]).fold(f32::MAX, f32::min);
    let min_y = panel_local_vertices.iter().map(|v| v[1]).fold(f32::MAX, f32::min);
    let max_x = panel_local_vertices.iter().map(|v| v[0]).fold(f32::MIN, f32::max);
    let max_y = panel_local_vertices.iter().map(|v| v[1]).fold(f32::MIN, f32::max);
    let w = (max_x - min_x).max(1e-6);
    let h = (max_y - min_y).max(1e-6);
    // Normalize preserving aspect ratio; no distortion.
    panel_local_vertices
        .iter()
        .map(|v| [(v[0] - min_x) / w, (v[1] - min_y) / h])
        .collect()
}
```

These per-vertex UVs are stored in `SolverMeshV1::vertex_uvs` (already defined in T-GARMENT-AUTHORING)
and survive the entire solver pipeline: stretch/bend constraints operate on 3D positions; UV
coordinates ride along as a per-vertex attribute that is never modified by the solver.

The only thing that modifies UV coordinates after mesh generation is the ARAP unfurl pass (see below).

#### [T-UV-TEXTURE.uv-from-pattern-mechanics.grain-uv-binding] Grain direction as authority

`PanelSpec::grain_angle_deg` (T-GARMENT-AUTHORING) is the grain direction in degrees from
horizontal in panel-local 2D space. It has two consumers:

1. **Solver crate (T-FABRIC-MODELS):** `FabricMaterial::grain_angle_rad` drives the WGSL
   anisotropic constraint axis lookup — weft compliance is applied to edges aligned with the
   grain's horizontal, warp compliance to edges aligned with the grain's vertical.

2. **Texture sampler (this topic):** `TailorPbrMaterialV1::grain_angle_deg` (mirrored from
   `PanelSpec`) rotates the texture sampling frame in the viewport PBR shader. In the WGSL
   fragment shader:

```wgsl
// tailor-solver/src/shaders/pbr_cloth.wgsl (fragment shader)

struct TailorMaterialUniform {
    grain_angle: f32,  // radians; from PanelSpec::grain_angle_deg converted at upload
    // ... other PBR fields
}

@group(1) @binding(0) var<uniform> material: TailorMaterialUniform;
@group(1) @binding(1) var base_color_texture: texture_2d<f32>;
@group(1) @binding(2) var base_color_sampler: sampler;

// In the fragment shader:
// Rotate UV by grain_angle before sampling.
// This aligns the warp/weft texture pattern with the physical grain on the garment.
fn rotate_uv(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    // Rotate around UV center (0.5, 0.5)
    let uv_c = uv - vec2<f32>(0.5, 0.5);
    return vec2<f32>(c * uv_c.x - s * uv_c.y, s * uv_c.x + c * uv_c.y) + vec2<f32>(0.5, 0.5);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let rotated_uv = rotate_uv(uv, material.grain_angle);
    let base_color = textureSample(base_color_texture, base_color_sampler, rotated_uv);
    // ... PBR computation
    return base_color;
}
```

The authority chain: `PanelSpec::grain_angle_deg` (Postgres JSONB) → `FabricMaterial::grain_angle_rad`
(solver crate) AND `TailorPbrMaterialV1::grain_angle_deg` (Postgres JSONB) → GPU uniform upload at
render time. Both consumers read from the same authority row; neither computes grain direction
independently.

---

### [T-UV-TEXTURE.flatten-algorithm-decision] Flatten Algorithm Decision: ARAP

**The contradiction in 03/07:** T-GARMENT-AUTHORING (bidirectional loop section) mentions "LSCM or
ABF++" for the flatten pass. T-AUTOFIT (uv-preservation section) implements `arap_unfurl_panel()`
with ARAP semantics. This document resolves the contradiction.

**Decision: use ARAP throughout. LSCM and ABF++ are eliminated.**

**Rationale:**

The flatten pass runs in two contexts:

1. **Bidirectional loop feedback** (T-GARMENT-AUTHORING): after draping, report back to the 2D
   editor that edge lengths have changed. The output is an updated set of panel vertex positions in
   2D; it feeds a CRDT delta proposal, not the UV atlas.

2. **UV recompute after refit** (T-AUTOFIT): after retargeting to a new body, the UV islands
   must reflect the post-drape surface so that texture maps placed on the original UV still make
   physical sense on the new drape.

Both contexts share the same requirement: **the panel boundary vertices are pinned** (they must stay
where the pattern authoring placed them, or where the seam constraints have settled); **only interior
vertex positions are adjusted** to minimize distortion.

This is precisely ARAP's design intent: minimize the sum of squared differences between per-triangle
Jacobians and their nearest rotations, with fixed boundary conditions. ARAP with pinned boundary
vertices converges in 3–10 alternating local/global iterations and produces an isometric result that
preserves area better than conformal methods when boundary pinning is enforced.

LSCM/ABF++ optimize angle distortion (conformal mapping) across the entire surface. They are
designed for the case where **no boundary is pinned** — the algorithm is free to choose the boundary.
Enforcing a pinned boundary on LSCM produces angle distortion that is no better than ARAP and adds
solver complexity (the conformal constraint competes with the fixed boundary). PartUV (arXiv
2511.16659, 2025) confirms this experimentally: using LSCM "leads to both higher runtime and a larger
number of charts" than ABF++. ABF++ is better than LSCM when parameterization freedom is full, but
still worse than ARAP for the pinned-boundary case relevant here.

**ARAP is already implemented in T-AUTOFIT as `arap_unfurl_panel()`.**
That function is the canonical flatten pass. It lives at `tailor-solver/src/uv/unfurl.rs`.
T-GARMENT-AUTHORING's mention of "LSCM or ABF++" is superseded by this decision; the
bidirectional loop feedback pass uses the same `arap_unfurl_panel()` call.

**No new algorithm is introduced.** One function, two call sites.

**Algorithm summary (from T-AUTOFIT, reproduced for completeness):**

```
Input: 3D triangle mesh of one panel after simulation equilibrium.
       Boundary vertex indices + their original 2D panel positions (from PanelSpec).
Output: Updated 2D UV coordinates for all vertices.

1. Initialize interior UV positions from the panel-local 2D coords (pre-drape).
2. Alternating local-global ARAP iteration (3-10 steps):
   Local step: for each triangle, compute nearest rotation R_k via SVD of the
               deformation Jacobian between current 2D and 3D triangle.
   Global step: solve the sparse linear system
               sum_k(w_k * L_k^T * L_k) * u = sum_k(w_k * L_k^T * R_k * r_k)
               for interior UV positions u, with boundary rows fixed.
3. Accept result. UV displacement from initial is the correction.
   If max displacement > flatten_tolerance_cm: emit CRDT proposal (bidirectional loop context).
   In refit UV context: write updated vertex_uvs array directly.
```

Implementation note: the sparse linear system can be assembled with `nalgebra-sparse` or a
custom CSR builder. For panel sizes typical in garment authoring (< 5,000 triangles per panel),
a direct Cholesky solver (e.g., `cholmod` via FFI or `faer` pure-Rust) converges in < 10 ms per
panel on CPU. GPU ARAP is not required for MVP.

---

### [T-UV-TEXTURE.uv-packing] UV Packing

UV packing places the per-panel UV islands into a single [0,1]^2 atlas (or a set of UDIM tiles for
high-resolution work) without overlap.

#### [T-UV-TEXTURE.uv-packing.requirements] Requirements

- Each panel's UV island occupies a rectangle in the atlas.
- Islands must not overlap.
- Rotations of 90° increments are permitted (fabric grain rotates with the island; the grain angle
  in the UV frame is adjusted accordingly).
- Islands should be packed efficiently (high fill ratio) to minimize wasted texel space.
- Packing must be deterministic: same garment → same atlas layout across sessions and platforms.
- Packing result is authority data: stored in `tailor_uv_islands` Postgres table.
- The packing is re-run any time a panel is added, removed, or resized; result is immutable for
  a given garment+simulation-run pair.

#### [T-UV-TEXTURE.uv-packing.algorithm] Algorithm: rectangle-pack Rust crate

UV island packing maps directly to the rectangle bin-packing problem. For garment work the number
of islands is small (typically 4–30 panels per garment), so algorithm quality matters less than
determinism and API simplicity.

**Selected crate: `rectangle-pack` (Rust, MIT/Apache-2.0)**

Repo: https://github.com/chinedufn/rectangle-pack
Docs: https://docs.rs/rectangle-pack

Properties:
- 100% deterministic: same input → same layout.
- No image dependency: operates on abstract rectangle sizes, not pixel buffers.
- Supports 2D and 3D rectangle packing.
- Configurable heuristics: `GroupedRectsToPlace`, `TargetBin`, `BinSection`.
- Zero transitive dependencies beyond `std`.

The packing algorithm is `MaxRects`-style; it places the largest remaining rectangle first into
the bin section that wastes the least space. This is appropriate for garment UV packing where
islands have moderate size variance.

Alternative crates evaluated:

| Crate | Algorithm | Verdict |
|---|---|---|
| `etagere` | Shelf packing | Designed for dynamic allocation/deallocation; suboptimal fill for static garment UV |
| `binpack2d` | Jukka Jylänki MaxRects | Good quality but less idiomatic Rust API |
| `tex-packer-core` | Skyline/MaxRects/Guillotine | Designed for pixel images; overkill; image dependency |
| `rectangle-pack` | MaxRects-style | Selected: deterministic, no image dep, clean API |

```rust
// src/tailor/uv_packing.rs

use rectangle_pack::{
    GroupedRectsToPlace, RectToInsert, TargetBin, pack_rects, RectanglePackError,
};

/// Output: per-panel UV island placement in the atlas.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UvIslandPlacement {
    pub panel_id: String,
    /// Top-left UV coordinate of the island bounding box in the packed atlas.
    pub atlas_uv_min: [f32; 2],
    /// Bottom-right UV coordinate of the island bounding box in the packed atlas.
    pub atlas_uv_max: [f32; 2],
    /// 90°-increment rotation applied to the island (0, 90, 180, 270 degrees).
    /// Grain angle must be adjusted by this rotation when sampling textures.
    pub rotation_deg: u32,
    /// Width and height of the island in normalized UV units (before packing).
    pub island_size_uv: [f32; 2],
}

/// Pack UV islands for all panels of a garment into a square atlas.
/// Islands are the per-panel UV bounding boxes from assign_panel_uvs().
/// Returns placements in the same order as panels input.
///
/// Atlas resolution hint: 4096×4096 pixels corresponds to atlas_size = 1.0 at ~4K detail.
pub fn pack_uv_islands(
    panels: &[(String, [f32; 2])],  // (panel_id, island_size_in_pattern_cm)
    panel_cm_to_atlas: f32,          // scale: how many cm maps to 1.0 atlas unit
    padding: f32,                    // padding between islands (atlas units)
) -> Result<Vec<UvIslandPlacement>, RectanglePackError> {
    let mut rects_to_place = GroupedRectsToPlace::new();
    for (i, (panel_id, size_cm)) in panels.iter().enumerate() {
        let w = (size_cm[0] * panel_cm_to_atlas + padding).ceil() as u32;
        let h = (size_cm[1] * panel_cm_to_atlas + padding).ceil() as u32;
        rects_to_place.push_rect(
            i as u32,   // rect id
            None,       // no group constraint
            RectToInsert::new(w, h, 1),
        );
    }
    let atlas_px = (1.0 / panel_cm_to_atlas * 4096.0) as u32;
    let mut target_bins = std::collections::BTreeMap::new();
    target_bins.insert(0u32, TargetBin::new(atlas_px, atlas_px, 1));

    let packing = pack_rects(
        &rects_to_place,
        &mut target_bins,
        &rectangle_pack::volume_heuristic,
        &rectangle_pack::contains_smallest_box,
    )?;

    let mut placements = Vec::with_capacity(panels.len());
    for (i, (panel_id, size_cm)) in panels.iter().enumerate() {
        if let Some(loc) = packing.packed_locations().get(&(i as u32)) {
            let x0 = loc.x() as f32 / atlas_px as f32;
            let y0 = loc.y() as f32 / atlas_px as f32;
            let w_uv = size_cm[0] * panel_cm_to_atlas;
            let h_uv = size_cm[1] * panel_cm_to_atlas;
            placements.push(UvIslandPlacement {
                panel_id: panel_id.clone(),
                atlas_uv_min: [x0, y0],
                atlas_uv_max: [x0 + w_uv, y0 + h_uv],
                rotation_deg: 0,  // rotation support: deferred to post-MVP
                island_size_uv: [w_uv, h_uv],
            });
        }
    }
    Ok(placements)
}
```

The packed atlas layout is written to `tailor_uv_islands` (see schema below) as the authority row
for a simulation run. Vertex UV coordinates in the `SolverMeshV1` are panel-local (pre-packing);
the atlas coordinates are applied as an affine transform at render time by the vertex shader:

```wgsl
// In tailor-solver/src/shaders/pbr_cloth.wgsl (vertex shader)

struct UvIslandTransform {
    uv_min: vec2<f32>,  // atlas_uv_min for this panel
    uv_max: vec2<f32>,  // atlas_uv_max for this panel
}
@group(2) @binding(0) var<storage, read> island_transforms: array<UvIslandTransform>;
// per-vertex panel_index into island_transforms is stored in vertex buffer

// Vertex shader UV transform:
let island = island_transforms[vertex_panel_index];
let atlas_uv = island.uv_min + (vertex_uv * (island.uv_max - island.uv_min));
```

---

### [T-UV-TEXTURE.graphic-layer-model] Graphic Layer Data Model

Graphic layers (prints, logos, embroidery, topstitching artwork) sit on top of the base fabric
texture. MD's "Edit Texture 2D" exposes this as a 2D surface editor. The Tailor data model stores
graphic layers as positioned rectangles in panel-local 2D space, with a blend mode and an image
artifact reference.

```rust
// src/tailor/texture.rs

/// A graphic overlay placed on a panel in 2D pattern space.
/// Multiple graphic layers can stack on a single panel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TailorGraphicLayerV1 {
    pub layer_id: String,          // UUID v7
    pub panel_id: String,          // references PanelSpec::panel_id
    pub garment_id: String,
    /// Image artifact reference (SHA256 content-addressed artifact store).
    /// The image file format must be PNG, JPEG, or WebP.
    pub image_artifact_ref: String,
    /// Bounding box in panel-local 2D space (cm), at grain-angle 0.
    /// [x_min, y_min, x_max, y_max]
    pub panel_bbox_cm: [f32; 4],
    /// Rotation of the graphic relative to panel horizontal (deg).
    pub rotation_deg: f32,
    /// Blend mode for compositing graphic over base fabric texture.
    pub blend_mode: GraphicBlendMode,
    /// Opacity scalar 0.0–1.0.
    pub opacity: f32,
    /// Whether this layer is pinned to boundary vertices (survives ARAP refit).
    pub boundary_pinned: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphicBlendMode {
    Normal,      // standard alpha compositing over fabric
    Multiply,    // darker overlay (screen printing look)
    Screen,      // lighter overlay (foil/metallic look)
    Overlay,     // high-contrast texture detail
    Emboss,      // use as additive normal-map contribution (embroidery)
}
```

**Compositing order (lowest to highest):**

```
1. Base fabric texture (tileable, grain-rotated) [from TailorPbrMaterialV1::base_color_texture_ref]
2. Graphic layer stack (ordered by layer z-order within garment panel)
3. Normal map (contributes to PBR calculation, not final color directly)
4. PBR channel maps (roughness, metalness, AO, displacement)
```

The graphic layer bbox is stored in panel-local 2D space (cm). The UV transform pipeline maps it
to atlas space:

```
panel_bbox_cm → normalized panel UV coords → atlas UV coords (via UvIslandTransform)
```

This means graphic layers specified in the 2D pattern editor will always correctly project onto the
3D draped surface without reprojection.

**Boundary pinning flag (`boundary_pinned = true`):** when this flag is set, the vertices of the
graphic layer bounding box are treated as pinned anchors in the ARAP unfurl pass. The ARAP solver
adds the graphic corner vertices to the pinned set alongside the panel boundary vertices, preventing
the unfurl from displacing the graphic from its intended position on the garment. This is the
"lock graphics layer" feature described as a mitigation in T-AUTOFIT.

---

### [T-UV-TEXTURE.pbr-material] PBR Material System

#### [T-UV-TEXTURE.pbr-material.model] TailorPbrMaterialV1 Schema

```rust
// src/tailor/texture.rs (continued)

/// PBR material definition for a garment panel or global garment override.
/// This is the render-side companion to FabricMaterial (which is physics-side).
/// Both reference the same grain_angle_deg from PanelSpec.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TailorPbrMaterialV1 {
    pub material_id: String,        // UUID v7
    pub workspace_id: String,
    pub name: String,
    /// Base color / albedo texture (tileable, normalized [0,1] UV space).
    /// None = solid base_color_srgb.
    pub base_color_texture_ref: Option<String>,
    /// Solid base color (used when base_color_texture_ref is None).
    /// sRGB, [r, g, b, a] in [0,1].
    pub base_color_srgb: [f32; 4],
    /// Normal map artifact ref (tangent-space; OpenGL Y-up convention).
    pub normal_map_ref: Option<String>,
    /// Roughness map artifact ref (R channel) or scalar fallback.
    pub roughness_map_ref: Option<String>,
    pub roughness_scalar: f32,      // used if roughness_map_ref is None; range [0,1]
    /// Metalness map artifact ref (R channel) or scalar fallback.
    pub metalness_map_ref: Option<String>,
    pub metalness_scalar: f32,
    /// Displacement/height map artifact ref (R channel).
    pub displacement_map_ref: Option<String>,
    pub displacement_scale_cm: f32, // cm of surface relief; 0 = no displacement
    /// Opacity map artifact ref (R channel). None = fully opaque.
    pub opacity_map_ref: Option<String>,
    pub opacity_scalar: f32,        // multiplied with opacity map if present
    /// Emissive color (additive glow; typically [0,0,0,0] for non-glowing fabrics).
    pub emissive_color_srgb: [f32; 4],
    /// Fabric grain direction in degrees from horizontal.
    /// Copied from PanelSpec::grain_angle_deg at material assignment time.
    /// Applied as UV rotation in the texture sampler (see [T-UV-TEXTURE.uv-from-pattern-mechanics.grain-uv-binding]).
    pub grain_angle_deg: f32,
    /// Texture scale: how many cm of the garment maps to one tile of the base texture.
    /// E.g., 10.0 means the texture tiles every 10 cm.
    pub texture_tile_size_cm: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

#### [T-UV-TEXTURE.pbr-material.assignment] Material Assignment per Panel

A garment panel's complete render material is the combination of:
- A physics material preset (`tailor_material_presets`, T-FABRIC-MODELS) — governs simulation
- A PBR material (`tailor_pbr_materials`) — governs rendering
- Zero or more graphic layers (`tailor_graphic_layers`) — graphic overlays

The link table `tailor_material_assignments` records these per-panel in a single authority row:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TailorMaterialAssignmentV1 {
    pub assignment_id: String,
    pub garment_id: String,
    pub panel_id: String,
    /// FK into tailor_material_presets (physics side).
    pub physics_preset_id: Option<String>,
    /// FK into tailor_pbr_materials (render side).
    pub pbr_material_id: Option<String>,
    /// Ordered list of graphic layer IDs to composite on this panel.
    pub graphic_layer_ids: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

A `None` physics preset means "inherit from garment-level default". A `None` PBR material means
"use system default (white cotton look)". This handles the common case where the operator wants to
start simulating before texturing.

#### [T-UV-TEXTURE.pbr-material.pbr-map-generator] PBR Map Generator (MD 2024.1 equivalent)

MD's PBR Map Generator automatically creates Displacement, Opacity, Roughness, Normal, and
Metalness maps from a base color texture. This is a CPU-side image processing pipeline, not a GPU
compute pass.

The Tailor equivalent runs as a Tauri command on the operator workstation:

```rust
// app/src-tauri/src/commands/tailor_texture.rs

#[tauri::command]
pub async fn tailor_generate_pbr_maps(
    base_color_artifact_ref: String,
    fabric_archetype: String,  // "cotton" | "silk" | "denim" | "leather" etc.
    options: PbrMapGenOptions,
    state: tauri::State<'_, AppState>,
) -> Result<PbrMapGenResult, String> {
    // 1. Load base color image from artifact store.
    // 2. Apply fabric-archetype-specific map generation pipeline.
    // 3. Return artifact refs for each generated map.
    todo!()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PbrMapGenOptions {
    /// Whether to generate normal map from base color via luminance gradient.
    pub gen_normal: bool,
    /// Normal map strength multiplier (1.0 = standard).
    pub normal_strength: f32,
    /// Whether to generate roughness map from base color luminance (inverted = rough where dark).
    pub gen_roughness: bool,
    /// Base roughness value for the fabric archetype (blended with texture-derived roughness).
    pub roughness_base: f32,
    /// Whether to generate metalness map (almost always false for fabrics; set for metallic threads).
    pub gen_metalness: bool,
    pub metalness_base: f32,
    /// Whether to generate displacement map from luminance.
    pub gen_displacement: bool,
    pub displacement_scale_cm: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PbrMapGenResult {
    pub normal_map_ref: Option<String>,
    pub roughness_map_ref: Option<String>,
    pub metalness_map_ref: Option<String>,
    pub displacement_map_ref: Option<String>,
    /// The generated maps are stored as artifacts; these refs are stored in TailorPbrMaterialV1.
    pub event_ledger_event_id: String,
}
```

**Map generation algorithms (CPU-side, using `image` crate):**

| Map type | Algorithm |
|---|---|
| Normal map | Sobel gradient on luminance channel → (dx, dy, 1.0) normalized → encode as RGB normal |
| Roughness map | Invert luminance (bright texture = smooth; dark weave grooves = rough) → clamp to [roughness_base, 1.0] |
| Metalness map | For most fabrics: constant `metalness_base` (0.0). For metallic thread patterns: luminance-thresholded mask |
| Displacement map | Luminance → scaled to `displacement_scale_cm` range; fabric grooves read as height |
| Opacity map | Only generated if base_color has alpha channel; otherwise omitted |

For fabric-specific map generation, the 2025 paper "A Texture-Free Practical Model for Realistic
Surface-Based Rendering of Woven Fabrics" (Khattar et al., CGF 2025) provides a reference for
how binary weave pattern matrices (1=warp over weft, 0=warp under weft) can replace traditional
texture maps entirely, reducing memory by 99.8% and render time by 27.6%. A future Tailor upgrade
can offer this mode as an alternative to bitmap PBR maps for procedural fabric archetypes.

**Reference implementation for the normal-map generation step (Rust, `image` crate):**

```rust
// src/tailor/pbr_map_gen.rs

use image::{GrayImage, RgbaImage, Luma, Rgba};

pub fn generate_normal_map(
    base_color: &RgbaImage,
    strength: f32,
) -> RgbaImage {
    let width = base_color.width();
    let height = base_color.height();
    let luma: GrayImage = image::imageops::grayscale(base_color);

    let mut normal_map = RgbaImage::new(width, height);
    for y in 1..(height - 1) {
        for x in 1..(width - 1) {
            let l = luma.get_pixel(x - 1, y).0[0] as f32 / 255.0;
            let r = luma.get_pixel(x + 1, y).0[0] as f32 / 255.0;
            let u = luma.get_pixel(x, y - 1).0[0] as f32 / 255.0;
            let d = luma.get_pixel(x, y + 1).0[0] as f32 / 255.0;
            // Sobel gradient
            let dx = (r - l) * strength;
            let dy = (d - u) * strength;
            let dz = 1.0_f32;
            let len = (dx * dx + dy * dy + dz * dz).sqrt();
            // Encode to [0,255] normal map (OpenGL convention: Y-up)
            let nx = ((dx / len * 0.5 + 0.5) * 255.0) as u8;
            let ny = ((dy / len * 0.5 + 0.5) * 255.0) as u8;
            let nz = ((dz / len * 0.5 + 0.5) * 255.0) as u8;
            normal_map.put_pixel(x, y, Rgba([nx, ny, nz, 255]));
        }
    }
    normal_map
}
```

The roughness/metalness/displacement generators follow the same `image` crate pattern with
per-pixel luminance arithmetic.

**FabricDiffusion integration (optional LLM upgrade path):**

FabricDiffusion (SIGGRAPH Asia 2024, arxiv 2410.01801) trains a denoising diffusion model to
extract distortion-free, tileable texture maps from in-the-wild clothing images. Its output is a
flat diffuse map that integrates with PBR pipelines. For production workflows where the operator
wants to extract a fabric texture from a reference photograph (rather than from an existing tileable
texture), `tailor_generate_pbr_maps` can optionally route through a FabricDiffusion inference step
before the Sobel-based map generation. This is an optional LLM/model upgrade path, not the MVP.
FabricDiffusion is Python/PyTorch; the Tauri command would invoke it via a subprocess or REST call
to a local inference server.

---

### [T-UV-TEXTURE.postgres-schema] Postgres tailor_* Texture Authority Schema

These tables are the authority for all UV and texture state. Every mutation emits an EventLedger
receipt. No SQLite.

```sql
-- Migration naming: follow the 2026_MM_DD_* convention (see KI-MIGRATION-COLLISION).
-- Example: 2026_08_15_tailor_texture_tables.sql

-- tailor_uv_islands: authority UV island placement for a garment+simulation pair.
-- Re-packed whenever panels change; immutable for a given simulation run.
CREATE TABLE tailor_uv_islands (
    island_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id          UUID NOT NULL REFERENCES tailor_garments(garment_id),
    simulation_run_id   UUID,           -- NULL if pre-simulation packing (authoring time)
    panel_id            TEXT NOT NULL,  -- references PanelSpec::panel_id
    atlas_uv_min_x      FLOAT4 NOT NULL,
    atlas_uv_min_y      FLOAT4 NOT NULL,
    atlas_uv_max_x      FLOAT4 NOT NULL,
    atlas_uv_max_y      FLOAT4 NOT NULL,
    rotation_deg        INT NOT NULL DEFAULT 0,      -- 0, 90, 180, or 270
    island_width_uv     FLOAT4 NOT NULL,
    island_height_uv    FLOAT4 NOT NULL,
    flatten_method      TEXT NOT NULL DEFAULT 'arap', -- 'arap' is the only valid value
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX tailor_uv_islands_garment_idx ON tailor_uv_islands (garment_id, simulation_run_id);
CREATE UNIQUE INDEX tailor_uv_islands_panel_run_idx
    ON tailor_uv_islands (garment_id, simulation_run_id, panel_id);

-- tailor_pbr_materials: render-side PBR material definitions.
CREATE TABLE tailor_pbr_materials (
    material_id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,
    base_color_texture_ref TEXT,        -- artifact content hash or NULL
    base_color_srgb     FLOAT4[4] NOT NULL DEFAULT '{1,1,1,1}',
    normal_map_ref      TEXT,
    roughness_map_ref   TEXT,
    roughness_scalar    FLOAT4 NOT NULL DEFAULT 0.8,
    metalness_map_ref   TEXT,
    metalness_scalar    FLOAT4 NOT NULL DEFAULT 0.0,
    displacement_map_ref TEXT,
    displacement_scale_cm FLOAT4 NOT NULL DEFAULT 0.0,
    opacity_map_ref     TEXT,
    opacity_scalar      FLOAT4 NOT NULL DEFAULT 1.0,
    emissive_color_srgb FLOAT4[4] NOT NULL DEFAULT '{0,0,0,0}',
    grain_angle_deg     FLOAT4 NOT NULL DEFAULT 0.0,
    texture_tile_size_cm FLOAT4 NOT NULL DEFAULT 10.0,
    is_system_preset    BOOLEAN NOT NULL DEFAULT false,
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX tailor_pbr_materials_workspace_idx ON tailor_pbr_materials (workspace_id);

-- tailor_graphic_layers: positioned graphic overlays in panel-local 2D space.
CREATE TABLE tailor_graphic_layers (
    layer_id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id          UUID NOT NULL REFERENCES tailor_garments(garment_id),
    panel_id            TEXT NOT NULL,
    z_order             INT NOT NULL DEFAULT 0,     -- composite order; higher = on top
    image_artifact_ref  TEXT NOT NULL,
    panel_bbox_cm       FLOAT4[4] NOT NULL,         -- [x_min, y_min, x_max, y_max] in cm
    rotation_deg        FLOAT4 NOT NULL DEFAULT 0.0,
    blend_mode          TEXT NOT NULL DEFAULT 'normal',
    opacity             FLOAT4 NOT NULL DEFAULT 1.0,
    boundary_pinned     BOOLEAN NOT NULL DEFAULT false,
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX tailor_graphic_layers_garment_panel_idx
    ON tailor_graphic_layers (garment_id, panel_id, z_order);

-- tailor_material_assignments: links physics preset + PBR material + graphic layers per panel.
CREATE TABLE tailor_material_assignments (
    assignment_id       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id          UUID NOT NULL REFERENCES tailor_garments(garment_id),
    panel_id            TEXT NOT NULL,
    physics_preset_id   UUID REFERENCES tailor_material_presets(preset_id),
    pbr_material_id     UUID REFERENCES tailor_pbr_materials(material_id),
    -- graphic_layer_ids ordered array: use tailor_graphic_layers z_order for compositing order
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (garment_id, panel_id)
);
CREATE INDEX tailor_material_assignments_garment_idx ON tailor_material_assignments (garment_id);
```

**No-SQLite tripwire**: every INSERT into these tables calls `guard_authority_write(AuthorityMode::Postgres)`
before the `sqlx::query!()`, mirroring `kb003_storage.rs`.

---

### [T-UV-TEXTURE.kernel-binding] Kernel Primitive Binding

#### [T-UV-TEXTURE.kernel-binding.event-types] New KernelEventType variants

Added to `KernelEventType` enum in `kernel/mod.rs` and registered in `required_first_slice_events()`:

```rust
// UV / texture domain events (added to existing Tailor* series)
TailorUvIslandsPacked,          // UV packing completed for a garment+run pair
TailorUvFlattenCompleted,       // ARAP unfurl pass completed for a panel
TailorUvFlattenProposed,        // ARAP correction proposed as CRDT delta (bidirectional loop)
TailorPbrMaterialCreated,       // new PBR material row written
TailorPbrMaterialUpdated,       // PBR material properties changed
TailorPbrMapsGenerated,         // PBR map generator completed; artifact refs recorded
TailorGraphicLayerAdded,        // graphic layer placed on a panel
TailorGraphicLayerUpdated,      // graphic layer moved, resized, or reordered
TailorGraphicLayerRemoved,      // graphic layer deleted (soft-delete via tombstone)
TailorMaterialAssignmentUpdated, // panel material assignment changed
```

EventLedger receipt pattern follows the builder in T-GARMENT-AUTHORING. Key payload fields per
event type:

- `TailorUvIslandsPacked`: `garment_id`, `simulation_run_id`, `island_count`, `atlas_fill_ratio`,
  `packing_algorithm: "rectangle_pack_maxrects"`.
- `TailorUvFlattenCompleted`: `garment_id`, `panel_id`, `max_displacement_cm`, `iterations_taken`,
  `converged: bool`.
- `TailorPbrMapsGenerated`: `base_color_artifact_ref`, `generated_map_refs` (JSON object),
  `fabric_archetype`, `options_summary`.

#### [T-UV-TEXTURE.kernel-binding.crdt] CRDT for texture collaborative editing

Graphic layer positions and PBR material property edits use the existing `CrdtUpdateRecordV1`
table and `yjs_bridge` serialization. Each garment has one CRDT document (established in
T-GARMENT-AUTHORING); within that document, the material assignment and graphic layer subtree
are CRDT maps keyed by `panel_id`.

Conflict resolution:
- PBR material field edits: last-write-wins per field (each material property is an independent
  CRDT map entry).
- Graphic layer z_order: concurrent reordering merges as a CRDT sequence; the merged order may
  differ from either participant's intent, which surfaces as a `TailorUvFlattenProposed` CRDT
  proposal for operator review.
- Graphic layer deletions: tombstone-based; deleted layers are marked with `deleted_at` TIMESTAMPTZ
  rather than hard-deleted from `tailor_graphic_layers`.

#### [T-UV-TEXTURE.kernel-binding.sandbox-promotion] Sandbox and promotion for texture state

UV packing and PBR map generation are **not** sandboxed operations — they are deterministic CPU
transforms that run synchronously in response to Tauri commands. They do not go through the
`SandboxAdapter` / `PromotionGate` pipeline.

The promotion gate is relevant only for:
- Model-authored PBR material presets (via `tailor_fabric_preset_create` tool call — already
  handled in T-FABRIC-MODELS sandbox validation path).
- Model-placed graphic layers (via `tailor_graphic_layer_add` tool call — enters sandbox as a CRDT
  proposal, goes through `ai_edit_proposal` in `kernel/crdt/ai_edit_proposal.rs`).

Direct operator actions (texture upload, PBR parameter editing, graphic layer placement) write to
authority immediately without a sandbox pass, identical to how the atelier domain handles direct
operator edits.

#### [T-UV-TEXTURE.kernel-binding.model-lanes] Model lane access

The model-lane gate exposes texture and UV operations via the MCP tool system. The model has
read access to all texture state and write access through proposals that enter the CRDT layer.

---

### [T-UV-TEXTURE.model-first-api] Model-First API

#### [T-UV-TEXTURE.model-first-api.tools] MCP Tool Definitions

```rust
// Tool: tailor_uv_inspect
// Purpose: return UV atlas layout for a garment so the model can assess packing quality.
// Input: { garment_id: String, simulation_run_id: Option<String> }
// Output: { islands: Vec<UvIslandPlacement>, atlas_fill_ratio: f32, panel_count: u32 }

// Tool: tailor_material_assign
// Purpose: assign PBR material and/or physics preset to a panel.
// Input: { garment_id, panel_id, pbr_material_id: Option, physics_preset_id: Option }
// Output: { assignment_id, event_ledger_event_id }

// Tool: tailor_pbr_material_create
// Purpose: create a new PBR material definition.
// Input: TailorPbrMaterialV1 fields (minus assignment-specific fields)
// Output: { material_id, event_ledger_event_id }
// Note: base_color_texture_ref must reference an already-uploaded artifact.

// Tool: tailor_graphic_layer_add
// Purpose: propose a graphic layer placement on a panel.
// Enters the CRDT sandbox layer (ai_edit_proposal); requires operator acceptance.
// Input: { garment_id, panel_id, image_artifact_ref, panel_bbox_cm, rotation_deg, blend_mode }
// Output: { proposal_id, layer_id (provisional) }

// Tool: tailor_generate_pbr_maps
// Purpose: run the PBR map generator on a base color texture.
// Input: { base_color_artifact_ref, fabric_archetype, options: PbrMapGenOptions }
// Output: PbrMapGenResult with artifact refs for each generated map.
```

#### [T-UV-TEXTURE.model-first-api.inspection-workflow] Model inspection workflow

A typical LLM workflow for texturing a garment after simulation:

```
1. Model receives context bundle with garment_id, simulation_run_id, GarmentSpecV1 JSON.
2. Calls tailor_uv_inspect(garment_id, simulation_run_id):
   → reads UvIslandPlacements; verifies atlas_fill_ratio > 0.6 (acceptable packing efficiency).
3. Reads TailorMaterialAssignmentV1 for each panel to see current physics + PBR material state.
4. If operator provided a reference texture image artifact_ref:
   a. Calls tailor_generate_pbr_maps(base_color_artifact_ref, fabric_archetype="cotton"):
      → receives normal_map_ref, roughness_map_ref etc.
   b. Calls tailor_pbr_material_create({ name: "Cotton Poplin", base_color_texture_ref, ...maps,
      grain_angle_deg: 0.0, texture_tile_size_cm: 8.0 }):
      → receives material_id.
5. For each panel that should use this material:
   Calls tailor_material_assign({ garment_id, panel_id, pbr_material_id: material_id,
     physics_preset_id: "cotton-system-preset-id" }).
6. Calls tailor_capture_frame(garment_id, simulation_run_id, frame_index, render_mode="solid"):
   → receives TailorVisualCapture with PNG + metadata.
7. Inspects PNG via vision call to verify grain direction, texture alignment, seam continuity.
8. If grain direction is wrong:
   - Reads grain_angle_deg from the relevant PanelSpec.
   - Proposes a PBR material update with corrected grain_angle_deg.
```

This workflow is fully model-driven, requires no operator UI interaction, and produces an
EventLedger audit trail of every material decision.

---

### [T-UV-TEXTURE.risks] Risks and Open Questions

**Risk 1 — ARAP convergence for heavily distorted panels.**
The ARAP alternating solver converges well for panels with less than ~30% area distortion between
2D pattern and post-simulation surface. Panels that wrinkle heavily (e.g., gathered skirt panels
with gather_ratio > 3.0) may not converge to a stable UV in 10 iterations, leaving residual
distortion in the UV island.

Mitigation: add a `flatten_max_iterations` parameter (default 10, max 50) to the ARAP solver call.
If distortion exceeds `flatten_tolerance_cm` after max iterations, emit a
`TailorUvFlattenWarning` event with the residual and surface the warning in the operator control
room. Do not block export. The UV is the best achievable for this drape state.

**Risk 2 — UV packing efficiency for asymmetric garment panels.**
Axis-aligned rectangle packing of UV islands treats each island as its bounding box. For angled or
L-shaped panel silhouettes, the bounding box wastes significant atlas space. A gather ratio 3.0
ruffle panel has a bounding box much larger than its actual area.

Mitigation: accept this for MVP. Post-MVP, implement 90°-increment island rotation (already
scaffolded in `UvIslandPlacement::rotation_deg`) and outline-accurate polygon packing (bin-pack
the island outline, not just its bbox). Track fill ratio in `TailorUvIslandsPacked` event and
surface it to the operator. Alert if fill ratio < 0.4.

**Risk 3 — Grain direction drift across refit.**
When ARAP refit adjusts UV interior positions, the effective grain direction in the UV may drift
from the panel's nominal `grain_angle_deg`. For example, if the ARAP result rotates the UV island
of a bodice front by 3° to minimize distortion, the texture grain on the garment will be 3° off.

Mitigation: after the ARAP unfurl pass, compute the mean rotation of the result relative to the
initial UV orientation and add it as a `grain_correction_deg` to `TailorUvIslandsPacked`. The
texture sampler reads `grain_angle_deg + grain_correction_deg` for the rotated UV frame. Surface
the correction as a visual indicator in the operator panel if `|grain_correction_deg| > 2.0°`.

**Risk 4 — Graphic layer position invalidated by ARAP refit.**
If a graphic layer is not boundary-pinned and the ARAP refit shifts interior UV positions
significantly, the graphic may appear in the wrong location on the 3D garment.

Mitigation: the `boundary_pinned` flag (already designed in `TailorGraphicLayerV1`) prevents
ARAP from displacing the graphic anchors. Set `boundary_pinned = true` by default for any graphic
layer that represents a design element (logo, print, embroidery). Set `boundary_pinned = false`
only for large seamless background textures that cover the whole panel.

**Risk 5 — PBR map generator quality for structured weave textures.**
Sobel-gradient normal map generation produces plausible normals for photographic fabric textures
but produces poor results for repeating geometric weave patterns (twill, herringbone) where the
pattern edges create high-frequency normal artifacts.

Mitigation: for structured weave fabrics, offer an alternative mode using the binary weave
pattern matrix approach from Khattar et al. (CGF 2025): instead of a Sobel-derived normal map,
generate the normal analytically from the weave structure (1D yarn cross-section profiles + binary
warp/weft matrix). This eliminates normal map artifacts for procedural weave fabrics. Implement
as a `gen_mode: "sobel" | "weave_matrix"` option in `PbrMapGenOptions`.

**Risk 6 — FabricDiffusion subprocess latency.**
If `tailor_generate_pbr_maps` routes through a local FabricDiffusion inference server, inference
may take 2–30 seconds on a consumer GPU. The Tauri command must be async and return an
`operation_id` immediately; the result arrives via a Tauri emit event when inference completes.
Do not block the Tailor panel UI on PBR map generation.

**Open Question A — UDIM tile support.**
High-resolution garments (fashion photography, hero character assets) benefit from UDIM tiling:
each panel occupies a separate [0,1]^2 UV tile, each tile baked to a separate 4K texture. The
current schema stores a single atlas per garment. UDIM support requires: (1) assigning each panel
to a UDIM tile index instead of packing into a shared atlas; (2) extending `tailor_uv_islands` with
a `udim_tile_index INT` column; (3) exporting one texture set per tile. Defer to post-MVP; flag
in `tailor_uv_islands` migration as a planned column.

**Open Question B — Texture bake from 3D drape to UV.**
MD's "Bake Textures" exports UV islands as Diffuse/Normal/Opacity maps. Tailor's equivalent would
bake the rendered 3D garment surface back to UV texture space (useful for capturing fabric
wrinkle-baked ambient occlusion). This requires a UV-space render pass in the wgpu pipeline (render
to UV coordinates, not screen). Complexity: medium; dependency on wgpu offscreen texture path
already established in T-RENDER-VIEWPORT. Defer to post-MVP.

---

### [T-UV-TEXTURE.sources] Sources

1. https://support.marvelousdesigner.com/hc/en-us/articles/47358198817689--Mode-UV-EDITOR — MD UV Editor mode: UV islands = exact flattened 2D pattern pieces; grain direction; automatic packing (access returned 403; content described from 02-md-feature-map.md, which verified this in 2026)
2. https://support.marvelousdesigner.com/hc/en-us/articles/47358164759193-PBR-Map-Generator-Ver-2024-1 — MD PBR Map Generator (2024.1): Displacement, Opacity, Roughness, Metalness auto-generation from base color (access returned 403; described from 02-md-feature-map.md and 06-fabric-models.md)
3. https://arxiv.org/abs/2511.16659 — PartUV: Part-Based UV Unwrapping of 3D Meshes (2025): ABF++ vs LSCM comparison; ABF++ superior to LSCM for parameterization; LSCM "higher runtime and larger number of charts"
4. https://www.cs.ubc.ca/~sheffa/papers/abf_plus_plus.pdf — ABF++: Fast and Robust Angle Based Flattening (Levy et al.): angle-based flattening; superior to LSCM for unconstrained parameterization; not designed for boundary-pinned case
5. https://dl.acm.org/doi/10.1145/1061347.1061354 — ABF++: fast and robust angle based flattening (ACM ToG): confirmed conformal mapping method; free-boundary design intent
6. https://arxiv.org/html/2403.06841v1 — Inverse Garment and Pattern Modeling with a Differentiable Simulator: ARAP with boundary pinning used for 3D-to-2D panel flattening after simulation
7. https://arxiv.org/pdf/2312.08386 — PerfectTailor: Scale-Preserving 2D Pattern Adjustment Driven by 3D Garment Editing: ARAP-based pattern flattening maintaining boundary constraints
8. D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/cloth_engine_research/07-autofit-retargeting.md — T-AUTOFIT.design.uv-preservation: arap_unfurl_panel() function design with boundary pinning; ARAP alternating local/global solver
9. D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/cloth_engine_research/03-garment-authoring.md — T-GARMENT-AUTHORING.bidirectional-2d-3d-loop: "LSCM or ABF++" mention (superseded by ARAP decision in this document)
10. D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/cloth_engine_research/03-garment-authoring.md — T-GARMENT-AUTHORING.pattern-to-mesh-pipeline: SolverMeshV1::vertex_uvs carrying panel-local UV through mesh generation
11. https://crates.io/crates/rectangle-pack — rectangle-pack Rust crate: deterministic 2D/3D rectangle bin packing; no image dependency; MIT/Apache-2.0; selected for UV island packing
12. https://docs.rs/etagere — etagere Rust crate: shelf packing for dynamic atlas allocation; evaluated and rejected (designed for deallocation, not static garment UV)
13. https://crates.io/crates/binpack2d — binpack2d: Rust implementation of Jukka Jylänki MaxRects; evaluated
14. https://crates.io/crates/tex-packer-core — tex-packer-core: Skyline/MaxRects/Guillotine packing; image dependency; evaluated and rejected
15. https://crates.io/crates/spade — spade v2.14.0 (May 2026): Rust 2D Delaunay + constrained Delaunay triangulation; MIT/Apache-2.0; used in pattern-to-mesh pipeline (T-GARMENT-AUTHORING)
16. https://arxiv.org/abs/2410.01801 — FabricDiffusion: High-Fidelity Texture Transfer for 3D Garments (SIGGRAPH Asia 2024): extracts distortion-free tileable texture maps from in-the-wild clothing images; integrates with PBR pipelines; optional upgrade path for tailor_generate_pbr_maps
17. https://dl.acm.org/doi/fullHtml/10.1145/3680528.3687637 — FabricDiffusion SIGGRAPH Asia 2024 full paper: UV space mapping; PBR pipeline coupling; diffusion model for texture rectification
18. https://onlinelibrary.wiley.com/doi/10.1111/cgf.15283 — A Texture-Free Practical Model for Realistic Surface-Based Rendering of Woven Fabrics (Khattar et al., CGF 2025): binary weave matrix + 1D yarn cross-section profiles; 99.8% memory reduction; analytical normal generation; reference for gen_mode="weave_matrix" PBR map option
19. https://arxiv.org/html/2602.16502v1 — DressWild: Feed-Forward Pose-Agnostic Garment Sewing Pattern Generation (2026): textures projected onto UV parameterization induced by sewing patterns; seam consistency and pattern-fabrication applicability confirmed
20. https://arxiv.org/abs/2504.21476 — GarmentDiffusion: 3D Garment Sewing Pattern Generation with Multimodal Diffusion Transformers (IJCAI 2025): sewing-pattern-derived UV parameterization for generated garments
21. https://humansensinglab.github.io/fabric-diffusion/ — FabricDiffusion project page: overview of tileable texture extraction pipeline
22. https://en.wikipedia.org/wiki/Least_squares_conformal_map — LSCM Wikipedia: free-boundary conformal mapping design intent; confirmed free-boundary assumption
23. https://github.com/icemiliang/lscm — LSCM C++ reference implementation: inspected for algorithm details; no Rust equivalent; confirms C++ only
24. https://docs.rs/rectangle-pack — rectangle-pack Rust docs: API for GroupedRectsToPlace, TargetBin, pack_rects, volume_heuristic
25. https://arxiv.org/html/2405.17609v2 — GarmentCodeData (ECCV 2024): UV segmentation output per panel; confirms pattern-derived UV islands in dataset generation pipeline
26. https://support.marvelousdesigner.com/hc/en-us/articles/47358146871065-Graphic-Style-Properties-Ver-2025-0 — MD Graphic Style Properties (2025.0): Material PBR default; blend modes; Metalness/Normal/Roughness map channels
27. https://support.marvelousdesigner.com/hc/en-us/articles/47358222133657--MODE-UV-EDITOR-Bake-Textures — MD Bake Textures: Diffuse/Normal/Opacity bake from UV islands; UV packing with selected shells
28. D:/Projects/LLM projects/Handshake/Handshake Worktrees/wt-gov-kernel/.GOV/reference/cloth_engine_research/06-fabric-models.md — T-FABRIC-MODELS: FabricMaterial::grain_angle_rad; WGSL anisotropic constraint axis; authority chain for grain direction
