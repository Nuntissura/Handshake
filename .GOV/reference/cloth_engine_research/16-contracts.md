---
file_id: cloth-engine-t-contracts
topic_id: T-CONTRACTS
title: "Canonical Tailor Authority Contracts"
status: draft
depends_on:
  - T-GARMENT-AUTHORING
  - T-CLOTH-SOLVER
  - T-COLLISION
  - T-FABRIC-MODELS
  - T-AUTOFIT
  - T-MODEL-FIRST-API
  - T-KERNEL-INTEGRATION
  - T-TRIM-RIGID
  - T-UV-TEXTURE
  - T-ANIMATION
  - T-CODEBASE
summary: "The single source of truth that resolves every schema/event/table/migration drift the coherence review found: one GarmentSpec, one KernelEventType additions list, one body-proxy/avatar schema incl. tailor_avatars, one dated migration convention, one ValidationDescriptor catalog, the determinism-vs-promotion tolerance resolution, and the full canonical tailor_* table set."
sources: 24
updated_at: "2026-06-17"
---

## [T-CONTRACTS] Canonical Tailor Authority Contracts

This topic is the **authority all other Tailor topics defer to**. Where any of `00`-`15` define a
schema, event name, table, migration name, or check, and that definition differs from this topic,
**this topic wins**. The other topics remain valid as design rationale, OSS evidence, and
implementation sketches; their concrete contract surfaces (Rust struct field names, SQL column
names, event-variant names) are superseded by the canonical forms defined here.

It resolves the drift catalogued in `index.yaml` → `known_issues`:
`KI-CONTRACTS-DRIFT`, `KI-MIGRATION-COLLISION`, `KI-DETERMINISM-VS-PROMOTION`, the schema half of
`KI-NAMING`, and the `tailor_avatars` gap.

This is reference material, not a Master Spec or validator authority (per
`research_paper.authority_note`). It is the canonical input a future implementation Work Packet
copies its contracts from; it does not gate code by itself. "Canonical" here means canonical
**within this research package**.

### [T-CONTRACTS.method] Reconciliation Method and Codebase Ground Truth

Every contract below was decided by (1) reading the conflicting definitions across `03/04/05/06/07/
09/10/13/14/15`, (2) verifying the load-bearing codebase facts against the live `wtc-kernel-009`
worktree, and (3) choosing the form that is most internally consistent, most LLM-emittable, and most
aligned with the existing kernel conventions. The decision rule when two topics disagreed was:
**prefer the form that matches the verified codebase convention; when the codebase is silent, prefer
the model-first form (`schemars`-derivable, flat, typed) from `T-MODEL-FIRST-API`.**

Codebase facts verified directly in `wtc-kernel-009` on 2026-06-17 (these correct stale claims in
`01-codebase-inventory.md`):

```text
FACT-1  KernelEventType has 67 variants (live), not 54 (01) and not 66 (index correction).
        Verified by enumerating the enum block in src/kernel/mod.rs. Wire format is
        SCREAMING_SNAKE_CASE via as_str(), e.g.
          KnowledgeLoomCanvasBoardRecorded => "KNOWLEDGE_LOOM_CANVAS_BOARD_RECORDED".
        All variants are registered in required_first_slice_events().

FACT-2  Migration 0334_loom_canvas_boards.sql EXISTS (WP-KERNEL-009 MT-261). Numbered
        migrations now run 0001..0335 (0335_loom_block_collection_views.sql is the highest
        numbered). Proposing 0334_tailor_garments.sql (03/01) or 0151_* (03) or 0333_* (01)
        COLLIDES. Confirmed: 194 forward .sql files + .down.sql pairs.

FACT-3  The codebase has a SECOND, dated migration convention used for new forward work:
        2026_05_18_fems_pinned.sql + 2026_05_18_fems_pinned.down.sql. This is the collision-free
        forward path. Dated migrations carry an explicit .down.sql reverse pair.

FACT-4  Recent migrations use TEXT PRIMARY KEY with prefixed string IDs (block_id, placement_id,
        tier_row_id, collection_id), NOT `UUID PRIMARY KEY DEFAULT gen_random_uuid()`. Verified in
        0334_loom_canvas_boards.sql and 0332_media_asset_tiers.sql. This means 05/06/07/14's
        `UUID PRIMARY KEY DEFAULT gen_random_uuid()` form is OFF-CONVENTION; 03/10/13's
        `TEXT PRIMARY KEY` form is correct.

FACT-5  No tailor_* tables and no avatar table exist yet. tailor_avatars is genuinely undefined
        across the package; it must be authored here (this topic).

FACT-6  The atelier domain event_family convention is dot-namespaced lowercase:
        "atelier.<domain>.<verb>" (e.g. "atelier.collection.created"). The Tailor analogue
        "tailor.<domain>.<verb>" is correct and collision-free.
```

The currency findings supplied with this task (XPBD/wgpu v29/CubeCL/GarmentCode/ChatGarment/MGPBD/
Chebyshev-GS/Warp/XRTailor/yrs-vs-Loro/`KI-DETERMINISM-VS-PROMOTION`) were treated as current and
folded into the relevant subsections; nothing in them contradicts a contract below.

---

### [T-CONTRACTS.naming] Canonical Naming Rules (applied to every contract here)

Per `research_paper.naming`, locked in for all contract surfaces:

| Surface | Canonical form | Example |
|---|---|---|
| Module | `handshake_core::tailor` (`src/tailor/`) | `src/tailor/garment.rs` |
| Solver crate | `tailor-solver` (workspace member, no `handshake_core` dep) | — |
| Solver trait | `ClothSolver` (physics term stays `cloth`) | `tailor-solver/src/lib.rs` |
| EventLedger variants | `Tailor*` PascalCase enum variant | `TailorGarmentPromoted` |
| EventLedger wire string | `TAILOR_*` SCREAMING_SNAKE_CASE via `as_str()` | `"TAILOR_GARMENT_PROMOTED"` |
| `event_family` constant | `tailor.<domain>.<verb>` lowercase dotted | `"tailor.garment.promoted"` |
| Postgres tables | `tailor_*` snake_case | `tailor_garments` |
| Domain data types | `Garment*`, `Panel`, `Seam`, `Fabric*` (DOMAIN stays Garment) | `GarmentSpec`, `PanelSpec` |
| Physics types | `Cloth*`, `ClothParticle`, `ClothConstraint` (physics stays cloth) | `ClothSolver` |
| Schema ID constant | `hsk.tailor.<record>@<v>` | `hsk.tailor.garment_spec@1` |

**Two naming corrections this topic makes against the other files:**

1. **Schema-ID namespace is `hsk.tailor.*`, not `hsk.cloth.*`.** Files `09/10/13/15` use
   `hsk.cloth.garment_draft@1`, `hsk.cloth.solver_request@1`, etc. The canonical namespace is
   `hsk.tailor.*` to match the feature/module/table/event prefix (the kernel convention is
   `hsk.<domain>.<record>@<v>`, and the domain is `tailor`). Physics-internal records that never
   leave the `tailor-solver` crate boundary (`solver_request`, `solver_result`) MAY stay
   `hsk.cloth.*` because they name a *cloth-physics* payload, not a Tailor-domain authority record;
   this is the one allowed `cloth` exception and is listed explicitly in
   [T-CONTRACTS.schema-ids].

2. **The canonical top-level garment type is `GarmentSpec` (not `GarmentSpecV1`, not
   `GarmentDraftV1`).** See [T-CONTRACTS.garment-spec].

---

### [T-CONTRACTS.garment-spec] ONE Canonical GarmentSpec

**The drift (`KI-CONTRACTS-DRIFT`):** the garment spec is defined four times with incompatible
choices:

| File | Type name | Units | Gather | Fabric values | Vertices | Edge shape |
|---|---|---|---|---|---|---|
| `03` | `GarmentSpecV1` | **cm** | `gather_ratio: f32` | raw compliance via `material_id` | explicit `vertices`+`edges` w/ `EdgeShape` enum | full `EdgeShape` (Straight/Quadratic/Cubic/Arc) |
| `09` | `GarmentSpec` | **cm** | `gather_ratio: f32` | **normalized [0,1]** inline `FabricProperties` | `vertices_cm` only (no edges) | none (polygon only) |
| `10` | `GarmentDraftV1` | **normalized [0,1]** | `ratio: f32` | inline `material_json` + raw | `vertices`+`edges` w/ `curve_type` string | `curve_type: "straight"|"bezier"` + `control_points` |
| `13/15` | extend `GarmentDraftV1` | (inherits 10) | (inherits 10) | (inherits 10) | (inherits 10) | (inherits 10) |

**Canonical decision.** The canonical type is **`GarmentSpec`** (the `T-MODEL-FIRST-API` name,
because the model API type IS the authority type — that is the differentiator). It is
`schemars::JsonSchema`-derivable so the MCP `inputSchema` is auto-generated. It lives in the
`tailor-solver` crate (`tailor-solver/src/spec.rs`) so the crate public API equals the model API.
The decided field shapes:

- **Units: centimetres (cm), everywhere.** `09`'s `vertices_cm` naming made the unit explicit;
  `03` agreed on cm; `10`'s normalized-[0,1] vertices are **rejected** for the authority type
  because (a) physical panel geometry, seam rest-lengths, dart depth, collision thickness, and body
  measurements are all physical quantities the solver needs in real units, (b) normalization throws
  away absolute scale which the XPBD compliance normalization (`alpha / rest_length^2`,
  T-FABRIC-MODELS Risk 1) needs, and (c) cm matches GarmentCode's own `"units": "cm"` for
  round-trip interop (`03` garmentcode-interop). The `_cm` suffix is mandatory on every length
  field name so the unit is self-documenting. **Normalized [0,1] survives only inside the Tier-1
  GarmentCodeRC LLM input** (the 76-float ChatGarment vector), which is a *pre-decode* convenience
  representation, never the authority spec — it is decoded to cm `GarmentSpec` before storage.

- **Gather: ONE float field `gather_ratio: f32` on `SeamSpec`, defined as `from_length /
  to_length`.** `10`/`15`'s `ratio` is renamed to `gather_ratio` (matches `03`, `09`). The
  "M:N ints" alternative implied by `03`'s prose ("1:N and M:N ratio sewing") is **rejected as a
  stored field**: the solver represents M:N gathering by resampling both edges to equal vertex
  count `N` at mesh-generation time and emitting `N` point constraints
  (`03` seam-constraint-encoding), so the authority only needs the scalar ratio. `1.0` = flat seam;
  `> 1.0` = gather the `from` edge onto the shorter `to` edge. Valid range `(0.0, 20.0]`
  (see [T-CONTRACTS.validation]).

- **Fabric values: normalized [0,1] in `GarmentSpec` (the LLM-facing surface); raw XPBD compliance
  in the solver and the preset library.** This is the one place the two regimes coexist by design,
  and the conversion boundary is made explicit. `09`'s `FabricProperties` (all fields normalized
  [0,1], "1.0 = stiffest") is the **authority form inside `GarmentSpec`**, because an LLM reasons
  about "stretchy vs stiff" on a 0..1 scale far better than about `5e-8` compliance. The raw
  anisotropic compliance (`stretch_weft: 5e-8`, T-FABRIC-MODELS `ClothMaterialCompliance`) lives in
  `tailor_material_presets` and in the solver crate only. The `FabricProperties::preset` field
  selects a named preset; explicit normalized fields override it. The mapping
  normalized-[0,1] → raw-compliance is owned by the preset/decoder layer
  (`T-FABRIC-MODELS.xpbd-compliance-mapping`), is non-linear (logarithmic, because compliance spans
  `1e-9..1e-3`), and is applied when building the `SolverMesh` material buffer — never stored twice.

- **Panel/edge/seam shapes: explicit vertices + typed edges (the `03` shape), in cm.** `09`'s
  polygon-only panels (no edge curves) are **rejected** because Bézier edge curves are required for
  realistic panel outlines (necklines, armholes) and for GarmentCode round-trip. `10`'s
  `curve_type: "bezier"` string + `control_points` is **rejected** in favour of `03`'s typed
  `EdgeShape` enum (`Straight | Quadratic | Cubic | Arc`), which is type-safe and `schemars`-clean.
  Edges reference vertex indices into the panel's `vertices_cm` array (the `03` model).

The canonical Rust (authority + model-emittable; lives in `tailor-solver/src/spec.rs`):

```rust
// tailor-solver/src/spec.rs  (standalone crate; no handshake_core deps)
// Derives serde + schemars so the MCP inputSchema is auto-generated.
// Stored as JSONB in tailor_garments.spec_json. THIS is the canonical garment type.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical garment specification. The LLM's primary output type AND the solver's
/// primary input type AND the Postgres authority JSONB. Supersedes GarmentSpecV1 (03)
/// and GarmentDraftV1 (10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Complete garment: panels, seams, darts, pleats, fabric, avatar binding.")]
pub struct GarmentSpec {
    /// Schema id constant: "hsk.tailor.garment_spec@1".
    pub schema_id: String,
    /// Stable garment identifier. Authority id form: "GAR-{uuid_v7}".
    pub garment_id: String,
    pub workspace_id: String,
    pub name: String,
    pub garment_type: GarmentType,
    /// 2D pattern panels. All coordinates in centimetres.
    pub panels: Vec<PanelSpec>,
    /// Seam definitions joining panel edges. Use gather_ratio for gather/pleat sewing.
    pub seams: Vec<SeamSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub darts: Vec<DartSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pleats: Vec<PleatSpec>,
    /// Fabric physical properties as a normalized [0,1] LLM-facing surface.
    pub fabric: FabricProperties,
    /// Avatar/body-proxy binding for fit and collision.
    pub avatar: AvatarBinding,
    /// Optional trim placements (buttons, zippers, eyelets) — see T-TRIM-RIGID.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trim_placements: Vec<TrimPlacementRef>,
    /// Optional natural-language description (NGL-Prompter intermediate); aids edit coherence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natural_description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GarmentType {
    Tshirt, Shirt, Jacket, Blazer, Dress, Skirt, Pants, Shorts, Bodice, Cape, Hood, Sleeve, Custom,
}

/// 2D point in panel-local coordinate space, in CENTIMETRES.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Vec2Cm { pub x: f32, pub y: f32 }

/// 6D placement of a panel in 3D space (initial draping pose).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Transform3D {
    /// Translation in centimetres.
    pub translation_cm: [f32; 3],
    /// Unit quaternion [x, y, z, w].
    pub rotation: [f32; 4],
}

/// Edge shape in panel-local 2D (cm). Typed enum supersedes 10's curve_type string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EdgeShape {
    Straight,
    Quadratic { control_cm: Vec2Cm },
    Cubic { control_a_cm: Vec2Cm, control_b_cm: Vec2Cm },
    Arc { curvature: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EdgeSpec {
    /// [start, end] indices into the parent panel's vertices_cm array.
    pub endpoints: [u32; 2],
    pub shape: EdgeShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fold_angle_deg: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "2D pattern panel; vertices in centimetres, counter-clockwise.")]
pub struct PanelSpec {
    /// Kebab-case id unique within the garment, e.g. "front-bodice".
    pub panel_id: String,
    /// Outline vertices in panel-local 2D space, CENTIMETRES, counter-clockwise. Min 3.
    pub vertices_cm: Vec<Vec2Cm>,
    /// Ordered directed edges closing the outline loop.
    pub edges: Vec<EdgeSpec>,
    /// 6D placement for initial draping.
    pub placement: Transform3D,
    /// Fabric grain direction, degrees from panel horizontal. None = isotropic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grain_angle_deg: Option<f32>,
    /// Optional per-panel material preset id (tailor_material_presets.preset_id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_preset_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SeamKind { Join, Fold, Tack }

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SeamEndpoint {
    pub panel_id: String,
    /// Index into PanelSpec::edges.
    pub edge_index: u32,
    /// Optional sub-range [0.0,1.0] for partial-edge (Free) sewing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f32; 2]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Seam joining two panel edges. gather_ratio = from_length/to_length.")]
pub struct SeamSpec {
    pub seam_id: String,
    pub kind: SeamKind,
    pub from: SeamEndpoint,
    pub to: SeamEndpoint,
    /// Gathering ratio = from_length / to_length. 1.0 = flat. (0.0, 20.0].
    /// CANONICAL field name (supersedes 10/15 `ratio`).
    pub gather_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DartSpec {
    pub dart_id: String,
    pub panel_id: String,
    pub tip_vertex: u32,
    pub opening_edges: [u32; 2],
    pub depth_cm: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PleatSpec {
    pub pleat_id: String,
    pub panel_id: String,
    pub kind: PleatKind,
    pub count: u32,
    pub depth_cm: f32,
    pub interval_cm: f32,
    pub fold_angle_deg: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PleatKind { Knife, Box, Accordion }

/// Fabric physical properties — NORMALIZED [0.0,1.0] LLM-facing surface (the 09 form).
/// 1.0 = stiffest/most resistant. The non-linear map to raw XPBD compliance is owned by
/// the preset/decoder layer (T-FABRIC-MODELS) and applied at solver-mesh build time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Fabric properties, normalized [0,1]. Weft=cross-grain, Warp=grain.")]
pub struct FabricProperties {
    /// Named preset applied first; explicit fields below override it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<FabricPreset>,
    pub stretch_weft: f32,
    pub stretch_warp: f32,
    pub shear: f32,
    pub bending_weft: f32,
    pub bending_warp: f32,
    pub buckling_ratio: f32,
    /// Mass per unit area in g/m^2 (the ONE non-normalized fabric field; physical, LLM-legible).
    pub density_g_m2: f32,
    /// Collision thickness in mm (physical; LLM-legible).
    pub collision_thickness_mm: f32,
    pub friction: f32,
    pub internal_damping: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FabricPreset {
    Cotton, Denim, Silk, Jersey, Leather, Satin, Linen, Wool, Spandex, Chiffon, Canvas, Rubber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AvatarBinding {
    /// Body-proxy authority id (tailor_avatars.avatar_id). Form: "AVT-{uuid_v7}"
    /// or a built-in parametric body slug (e.g. "avatar1-smplx-default").
    pub avatar_id: String,
    /// Optional measurement overrides (cm) for parametric bodies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurements_cm: Option<BodyMeasurements>,
}

/// Body measurements in CENTIMETRES (LLM-facing). NOTE: the authority body-proxy stores
/// the full 25-measurement set in MILLIMETRES (see [T-CONTRACTS.body-proxy]); this cm
/// subset is the LLM convenience surface and is converted at the boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BodyMeasurements {
    pub height_cm: f32,
    pub bust_cm: f32,
    pub waist_cm: f32,
    pub hip_cm: f32,
    pub inseam_cm: f32,
}

/// Lightweight reference to a trim placement (full trim authority is tailor_trim_placements).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TrimPlacementRef {
    pub placement_id: String,
    pub trim_category: String,
}
```

**Status field is NOT on `GarmentSpec`.** `03`'s `GarmentStatus` enum and `created_at`/`updated_at`
are **promotion-lifecycle metadata, not garment content** — they live on the `tailor_garments`
Postgres row (`status` column + `created_at`/`updated_at`), not inside the JSONB spec. This keeps
the model-emitted spec free of fields the model must not set. Canonical status values (the
`tailor_garments.status` CHECK domain): `draft | sandbox_pending | simulated | validated |
promoted | rejected | archived`.

**Solver-input mesh.** The pattern-to-mesh pipeline (`03`) converts `GarmentSpec` to `SolverMesh`
(canonical name, supersedes `SolverMeshV1`), defined in `tailor-solver/src/mesh.rs`. The
`SeamConstraintRecord` keeps `gather_ratio` (matching the canonical seam field). Lengths in cm.

---

### [T-CONTRACTS.body-proxy] ONE Canonical Body-Proxy / Avatar Authority Schema (incl. tailor_avatars)

**The drift (`KI-CONTRACTS-DRIFT`):** the body proxy is defined three times and all three FK to an
`tailor_avatars` table that **no topic ever defines** (`FACT-5`):

| File | Struct | PK type | Units | Proxy shape | FK target |
|---|---|---|---|---|---|
| `05` | `ClothBodyProxy` + `tailor_body_proxies` | `UUID … gen_random_uuid()` | cm (offset_cm, radius_cm) | capsules + spheres (fixed max) | `tailor_avatars(avatar_id)` (**undefined**) |
| `07` | `ClothBodyProxyV1` + `tailor_body_proxies` | `UUID … gen_random_uuid()` | **mm** (measurements_mm, radius_mm) | capsules only | none |
| `09` | `AvatarBinding.avatar_id` only | — | cm | (abstract) | `tailor_material_library` (**wrong table**) |

**Canonical decision.** Author the missing `tailor_avatars` table as the avatar-identity authority,
and split it cleanly from `tailor_body_proxies` (the solver collision geometry). Two tables, one
FK chain. `AvatarBinding.avatar_id` (09) references `tailor_avatars`, **not** `tailor_material_library`
(that was a copy-paste error in 09). Decisions:

- **PK type: `TEXT PRIMARY KEY` with prefixed ids** (`FACT-4`): `avatar_id = "AVT-{uuid_v7}"`,
  `body_proxy_id = "BPX-{uuid_v7}"`. The `UUID … gen_random_uuid()` form from `05`/`07` is
  **rejected** as off-convention.
- **Units: MILLIMETRES inside the proxy/measurement authority** (the `07` form), because the
  GarmentMeasurements field convention (`bust_circ_mm`, etc.) is mm and the solver capsule build
  reads mm. The `05` cm offsets are converted to mm. The LLM-facing `BodyMeasurements` (cm,
  in `GarmentSpec`) is converted at the API boundary — the `_mm`/`_cm` suffix makes the unit
  explicit on every field so no ambiguity survives.
- **Proxy shape: capsules + spheres** (the `05` superset), because the exaggerated-proportion
  large-bust case (`05` exaggerated-proportions, an explicit production target) requires sphere
  sub-proxies that `07`'s capsules-only form cannot express. `07`'s `CollisionCapsuleV1` is merged
  into the capsule list. The fixed-size GPU arrays (max 32 capsules + 16 spheres) live in the
  solver crate (`GpuCapsule`/`GpuSphere`, `05`); the authority JSONB stores variable-length lists.

Canonical authority schema:

```sql
-- tailor_avatars: avatar IDENTITY authority (the body a garment is fitted to).
-- This is the table that was referenced-but-never-defined across 05/09. Authored here.
CREATE TABLE IF NOT EXISTS tailor_avatars (
    avatar_id            TEXT PRIMARY KEY,                -- "AVT-{uuid_v7}"
    workspace_id         TEXT NOT NULL,
    name                 TEXT NOT NULL,
    -- Source body-model hint. Bridges the operator's 2D Avatar1/ComfyUI/SMPL-X pipeline
    -- to a 3D body proxy (the KI-PRODUCTION-BRIDGE first-mile input).
    source_kind          TEXT NOT NULL
        CHECK (source_kind IN ('smpl','smplx','metahuman','custom_obj','vrm','gltf',
                                'parametric','avatar1_2d_derived','non_humanoid')),
    -- 25 anthropometric measurements in MILLIMETRES (GarmentMeasurements naming).
    measurements_mm_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Optional source mesh artifact (the imported body OBJ/glTF) for proxy rebuild.
    source_mesh_artifact_ref TEXT,
    -- Morph/blend-shape parameters for parametric bodies (e.g. bust morph magnitude).
    morph_params_json    JSONB,
    event_ledger_event_id TEXT NOT NULL,                  -- FK kernel_event_ledger.event_id
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_avatars_workspace ON tailor_avatars (workspace_id);

-- tailor_body_proxies: solver COLLISION GEOMETRY for an avatar (capsules + spheres + optional SDF).
-- One avatar may have several proxies (standard, multi-sphere large-bust, sdf-fallback).
CREATE TABLE IF NOT EXISTS tailor_body_proxies (
    body_proxy_id        TEXT PRIMARY KEY,                -- "BPX-{uuid_v7}"
    avatar_id            TEXT NOT NULL REFERENCES tailor_avatars (avatar_id),
    workspace_id         TEXT NOT NULL,
    -- ClothBodyProxy serialized: { capsules:[{joint_name,p0_mm,p1_mm,radius_mm}],
    --                              spheres:[{bone,center_mm,radius_mm}], thickness_mm }
    proxy_json           JSONB NOT NULL,
    mode                 TEXT NOT NULL DEFAULT 'capsule'
        CHECK (mode IN ('capsule','capsule_sphere','capsule_sdf','sdf')),
    breast_proxy_mode    TEXT
        CHECK (breast_proxy_mode IS NULL OR
               breast_proxy_mode IN ('standard','multi_sphere','sdf_fallback')),
    -- Optional baked SDF + low-res mesh artifacts (large-bust / non-humanoid fallback).
    sdf_artifact_ref     TEXT,
    lores_mesh_artifact_ref TEXT,
    -- Optional skinning joint hierarchy for rig-transfer (Bolt Stage 3 / T-AUTOFIT).
    joint_hierarchy_json JSONB,
    collision_thickness_mm FLOAT NOT NULL DEFAULT 2.5,
    event_ledger_event_id TEXT NOT NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_body_proxies_avatar ON tailor_body_proxies (avatar_id);
```

Canonical Rust authority types (merge of `05` + `07`; `tailor-solver` crate for the geometry, all mm):

```rust
// tailor-solver/src/body/proxy.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClothBodyProxy {
    pub body_proxy_id: String,    // "BPX-{uuid_v7}"
    pub avatar_id: String,        // "AVT-{uuid_v7}"
    /// Capsule chain (body segments). Lengths/radii in MILLIMETRES.
    pub capsules: Vec<CollisionCapsule>,
    /// Sphere sub-proxies (breast/bust sub-volumes, joint spheres). MILLIMETRES.
    pub spheres: Vec<CollisionSphere>,
    pub thickness_mm: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CollisionCapsule {
    pub joint_name: String,
    pub p0_mm: [f32; 3],
    pub p1_mm: [f32; 3],
    pub radius_mm: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CollisionSphere {
    pub bone: String,
    pub center_mm: [f32; 3],
    pub radius_mm: f32,
}
```

The GPU upload types `GpuCapsule`/`GpuSphere` (fixed max 32+16, `bytemuck::Pod`) from `05` are the
unchanged runtime representation; the authority `ClothBodyProxy` is serialized to/from them.

---

### [T-CONTRACTS.event-types] ONE Canonical KernelEventType Additions List

**The drift (`KI-CONTRACTS-DRIFT`, naming half of `KI-NAMING`):** five files add overlapping but
inconsistently-named event variants. Examples of the conflict:

```text
03:  TailorGarmentValidated          10:  TailorGarmentValidationRecorded   09:  (none)
03:  TailorSimRunStarted             10:  TailorSimRunRequested+Started       09:  TailorSimRunStarted
03:  TailorGarmentCrdtUpdateRecorded 10:  TailorPanelCrdtUpdateRecorded       09:  TailorCrdtUpdateRecorded
04:  TailorGarmentValidated          (vs 10 TailorGarmentValidationRecorded)
09 also maps TailorGarmentPromoted -> wire "GARMENT_PATTERN_PROMOTED" (drops the TAILOR_ prefix!)
```

**Canonical decision.** ONE list. Variant names are `Tailor*` PascalCase; wire strings are
`TAILOR_*` SCREAMING_SNAKE_CASE (the kernel `as_str()` convention, `FACT-1`) — `09`'s
`GARMENT_*` wire strings that drop the `TAILOR_` prefix are **rejected**. Lifecycle verbs are
normalized to one set: `…Requested`, `…Started`, `…Completed`, `…Rejected` for runs;
`…Recorded` for validation; `…Promoted`/`…PromotionRejected` for promotion. CRDT events use the
specific `TailorPanelCrdtUpdateRecorded` form (`10`) because panels, animation, and texture each
have their own CRDT sub-tree and need distinct event types.

These variants are added to `KernelEventType` in `kernel/mod.rs` (taking the count from 67 to
67+N) and **every one** is registered in `required_first_slice_events()`.

```rust
// === CANONICAL Tailor KernelEventType additions (kernel/mod.rs) ===
// variant => wire string (as_str)

// -- Garment lifecycle --
TailorGarmentDraftProposed,       // "TAILOR_GARMENT_DRAFT_PROPOSED"
TailorGarmentDraftUpdated,        // "TAILOR_GARMENT_DRAFT_UPDATED"
TailorGarmentValidationRecorded,  // "TAILOR_GARMENT_VALIDATION_RECORDED"  (supersedes 03/04 *Validated)
TailorGarmentPromoted,            // "TAILOR_GARMENT_PROMOTED"
TailorGarmentPromotionRejected,   // "TAILOR_GARMENT_PROMOTION_REJECTED"

// -- Simulation run lifecycle (XPBD solver sandbox) --
TailorSimRunRequested,            // "TAILOR_SIM_RUN_REQUESTED"
TailorSimRunStarted,              // "TAILOR_SIM_RUN_STARTED"
TailorSimRunCompleted,            // "TAILOR_SIM_RUN_COMPLETED"
TailorSimRunRejected,             // "TAILOR_SIM_RUN_REJECTED"

// -- CRDT collaborative editing (one event per sub-tree) --
TailorPanelCrdtUpdateRecorded,    // "TAILOR_PANEL_CRDT_UPDATE_RECORDED"
TailorPanelCrdtSnapshotRecorded,  // "TAILOR_PANEL_CRDT_SNAPSHOT_RECORDED"
TailorPanelAiEditProposalRecorded,// "TAILOR_PANEL_AI_EDIT_PROPOSAL_RECORDED"
TailorPanelAiEditProposalDecided, // "TAILOR_PANEL_AI_EDIT_PROPOSAL_DECIDED"
TailorCrdtConflictDetected,       // "TAILOR_CRDT_CONFLICT_DETECTED"

// -- Material / fabric presets (T-FABRIC-MODELS) --
TailorMaterialPresetRecorded,     // "TAILOR_MATERIAL_PRESET_RECORDED"
TailorMaterialPresetUpdated,      // "TAILOR_MATERIAL_PRESET_UPDATED"
TailorMaterialPresetRejected,     // "TAILOR_MATERIAL_PRESET_REJECTED"
TailorGarmentMaterialAssigned,    // "TAILOR_GARMENT_MATERIAL_ASSIGNED"

// -- Avatar / body proxy (T-COLLISION + T-AUTOFIT, reconciled here) --
TailorAvatarCreated,              // "TAILOR_AVATAR_CREATED"
TailorAvatarMeasurementsExtracted,// "TAILOR_AVATAR_MEASUREMENTS_EXTRACTED"
TailorBodyProxyCreated,           // "TAILOR_BODY_PROXY_CREATED"
TailorBodyProxyUpdated,           // "TAILOR_BODY_PROXY_UPDATED"

// -- Refit / retargeting (T-AUTOFIT) --
TailorRefitRequested,             // "TAILOR_REFIT_REQUESTED"
TailorRefitPatternScaled,         // "TAILOR_REFIT_PATTERN_SCALED"
TailorRefitDrapeCompleted,        // "TAILOR_REFIT_DRAPE_COMPLETED"
TailorRefitUvRecomputed,          // "TAILOR_REFIT_UV_RECOMPUTED"
TailorRefitPromoted,              // "TAILOR_REFIT_PROMOTED"
TailorRefitRejected,              // "TAILOR_REFIT_REJECTED"

// -- Trims, zippers, lacing, pattern-to-rigid (T-TRIM-RIGID) --
TailorTrimImported,               // "TAILOR_TRIM_IMPORTED"
TailorTrimPlaced,                 // "TAILOR_TRIM_PLACED"
TailorTrimTackUpdated,            // "TAILOR_TRIM_TACK_UPDATED"
TailorZipperDefined,              // "TAILOR_ZIPPER_DEFINED"
TailorLacingDefined,              // "TAILOR_LACING_DEFINED"
TailorPatternToTrimConverted,     // "TAILOR_PATTERN_TO_TRIM_CONVERTED"
TailorTrimContactViolation,       // "TAILOR_TRIM_CONTACT_VIOLATION"

// -- UV / texture (T-UV-TEXTURE) --
TailorUvIslandsPacked,            // "TAILOR_UV_ISLANDS_PACKED"
TailorUvFlattenCompleted,         // "TAILOR_UV_FLATTEN_COMPLETED"
TailorUvFlattenProposed,          // "TAILOR_UV_FLATTEN_PROPOSED"
TailorPbrMaterialCreated,         // "TAILOR_PBR_MATERIAL_CREATED"
TailorPbrMaterialUpdated,         // "TAILOR_PBR_MATERIAL_UPDATED"
TailorPbrMapsGenerated,           // "TAILOR_PBR_MAPS_GENERATED"
TailorGraphicLayerAdded,          // "TAILOR_GRAPHIC_LAYER_ADDED"
TailorGraphicLayerUpdated,        // "TAILOR_GRAPHIC_LAYER_UPDATED"
TailorGraphicLayerRemoved,        // "TAILOR_GRAPHIC_LAYER_REMOVED"
TailorMaterialAssignmentUpdated,  // "TAILOR_MATERIAL_ASSIGNMENT_UPDATED"

// -- Animation timeline (T-ANIMATION) --
TailorAnimationDraftCreated,      // "TAILOR_ANIMATION_DRAFT_CREATED"
TailorAnimationDraftUpdated,      // "TAILOR_ANIMATION_DRAFT_UPDATED"
TailorAnimationSimRunRequested,   // "TAILOR_ANIMATION_SIM_RUN_REQUESTED"
TailorAnimationSimRunCompleted,   // "TAILOR_ANIMATION_SIM_RUN_COMPLETED"
TailorAnimationSimRunRejected,    // "TAILOR_ANIMATION_SIM_RUN_REJECTED"
TailorAnimationDraftPromoted,     // "TAILOR_ANIMATION_DRAFT_PROMOTED"

// -- Export (T-RENDER-VIEWPORT / T-ANIMATION) --
TailorGarmentExportCompleted,     // "TAILOR_GARMENT_EXPORT_COMPLETED"

// -- Wardrobe grouping (T-KERNEL-INTEGRATION) --
TailorWardrobeCreated,            // "TAILOR_WARDROBE_CREATED"
TailorWardrobeGarmentAdded,       // "TAILOR_WARDROBE_GARMENT_ADDED"
TailorWardrobeGarmentRemoved,     // "TAILOR_WARDROBE_GARMENT_REMOVED"
```

**Explicitly superseded variant names** (do NOT use; listed so other topics can be reconciled):
`TailorGarmentValidated` (03/04) → `TailorGarmentValidationRecorded`;
`TailorPatternValidated` (01) → `TailorGarmentValidationRecorded`;
`TailorCrdtUpdateRecorded` (09) / `TailorGarmentCrdtUpdateRecorded` (03/04) →
`TailorPanelCrdtUpdateRecorded`; bare `BodyProxyCreated`/`BodyProxyMeasurementsExtracted` (07,
missing the `Tailor` prefix) → `TailorBodyProxyCreated`/`TailorAvatarMeasurementsExtracted`;
`TailorMaterialLibraryUpdated` (03/01) → `TailorMaterialPresetRecorded`/`…Updated`;
`TailorDraftScaled` (07) → `TailorRefitPatternScaled`.

Canonical `event_family` constants (dot-namespaced, `FACT-6`), in `src/tailor/event_family.rs`:

```rust
pub const TAILOR_GARMENT:       &str = "tailor.garment";
pub const TAILOR_SIMULATION:    &str = "tailor.simulation";
pub const TAILOR_PANEL_CRDT:    &str = "tailor.panel.crdt";
pub const TAILOR_MATERIAL:      &str = "tailor.material";
pub const TAILOR_AVATAR:        &str = "tailor.avatar";
pub const TAILOR_BODY_PROXY:    &str = "tailor.body_proxy";
pub const TAILOR_REFIT:         &str = "tailor.refit";
pub const TAILOR_TRIM:          &str = "tailor.trim";
pub const TAILOR_UV:            &str = "tailor.uv";
pub const TAILOR_TEXTURE:       &str = "tailor.texture";
pub const TAILOR_ANIMATION:     &str = "tailor.animation";
pub const TAILOR_WARDROBE:      &str = "tailor.wardrobe";
pub const TAILOR_EXPORT:        &str = "tailor.export";
```

---

### [T-CONTRACTS.schema-ids] ONE Canonical Schema-ID Set

Namespace is `hsk.tailor.*` (the domain), superseding the `hsk.cloth.*` strings in `09/10/13/15`.
The single allowed `hsk.cloth.*` exception is the pair of *solver-crate-internal physics payloads*
that never become Tailor-domain authority rows.

```rust
// src/tailor/schemas.rs  (kernel-side, mirrors kb003_schemas.rs convention)
pub const SCHEMA_TAILOR_GARMENT_SPEC_V1:    &str = "hsk.tailor.garment_spec@1";       // GarmentSpec
pub const SCHEMA_TAILOR_MATERIAL_PRESET_V1: &str = "hsk.tailor.material_preset@1";
pub const SCHEMA_TAILOR_AVATAR_V1:          &str = "hsk.tailor.avatar@1";
pub const SCHEMA_TAILOR_BODY_PROXY_V1:      &str = "hsk.tailor.body_proxy@1";
pub const SCHEMA_TAILOR_TRIM_V1:            &str = "hsk.tailor.trim@1";
pub const SCHEMA_TAILOR_TRIM_PLACEMENT_V1:  &str = "hsk.tailor.trim_placement@1";
pub const SCHEMA_TAILOR_PBR_MATERIAL_V1:    &str = "hsk.tailor.pbr_material@1";
pub const SCHEMA_TAILOR_GRAPHIC_LAYER_V1:   &str = "hsk.tailor.graphic_layer@1";
pub const SCHEMA_TAILOR_UV_ISLAND_V1:       &str = "hsk.tailor.uv_island@1";
pub const SCHEMA_TAILOR_ANIMATION_DRAFT_V1: &str = "hsk.tailor.garment_animation_draft@1";
pub const SCHEMA_TAILOR_REFIT_REQUEST_V1:   &str = "hsk.tailor.refit_request@1";
pub const SCHEMA_TAILOR_SIM_RECEIPT_V1:     &str = "hsk.tailor.simulation_receipt@1";

// Allowed hsk.cloth.* exception — solver-crate-internal physics payloads only
// (cross the tailor-solver trait boundary; never stored as authority rows):
pub const SCHEMA_CLOTH_SOLVER_REQUEST_V1:   &str = "hsk.cloth.solver_request@1";
pub const SCHEMA_CLOTH_SOLVER_RESULT_V1:    &str = "hsk.cloth.solver_result@1";
```

---

### [T-CONTRACTS.migration-naming] ONE Dated Migration-Naming Convention

**The drift (`KI-MIGRATION-COLLISION`):** four naming schemes appear across the package
(`0151_*` in 03; `0334_*`/`0335_*`/`0336_*` in 01; `2026_06_17_*` in 13/14; `2026_MM_DD_*` hint
in 07/14). `0334_tailor_garments.sql` **collides** with the live `0334_loom_canvas_boards.sql`
(`FACT-2`); the numbered space is contested by parallel work packets.

**Canonical decision.** Use the codebase's **dated** convention (`FACT-3`):

```text
migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.sql
migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.down.sql      (required reverse pair)
```

Rules:
1. The date is the **authoring date of the migration**, assigned when the implementation WP writes
   it — NOT `2026_06_17` (this research date). The research package must NOT hard-code a specific
   date; it specifies the *convention* only. Example placeholder used in this package:
   `2026_MM_DD_tailor_garments.sql`.
2. Every forward migration ships a `.down.sql` reverse pair (the `2026_05_18_fems_pinned` precedent).
3. Numbered `0NNN_*` migrations are **forbidden** for Tailor — the integer space is a shared
   sequence other work packets append to, and any fixed number races them. Dated names cannot
   collide on the integer sequence.
4. Suggested Tailor migration set (one per concern, dated at authoring time):
   `*_tailor_garments.sql`, `*_tailor_material_presets.sql`, `*_tailor_avatars.sql`,
   `*_tailor_body_proxies.sql`, `*_tailor_simulation_runs.sql`, `*_tailor_refit_runs.sql`,
   `*_tailor_trims.sql`, `*_tailor_texture_tables.sql`, `*_tailor_wardrobe.sql`,
   `*_tailor_garments_animation_col.sql` (the `ALTER TABLE … ADD COLUMN animation_json`).

This supersedes the numbered-migration examples in `01` (`0334`-`0336`), `03` (`0151`), and any
fixed date in `13`/`14`.

---

### [T-CONTRACTS.tables] The Full Canonical tailor_* Postgres Table Set

Every Tailor authority table, reconciled to ONE set. All use `TEXT PRIMARY KEY` with prefixed ids
(`FACT-4`); every row carries `event_ledger_event_id TEXT` (FK `kernel_event_ledger.event_id`);
every INSERT calls `guard_authority_write(AuthorityMode::Postgres)` first (`no_sqlite_tripwire`,
`FACT-…`/T-KERNEL-INTEGRATION). The canonical column/id forms below override the per-file variants.

```text
TABLE                          PK (prefix)          KEY FKs / notes
-----------------------------  -------------------  --------------------------------------------
tailor_garments                garment_id  GAR-     workspace_id; status CHECK domain (see below);
                                                    spec_json JSONB (GarmentSpec); animation_json
                                                    JSONB (nullable, T-ANIMATION); wardrobe_id;
                                                    promotion_receipt_id; body_proxy_id
tailor_garment_crdt_docs       (garment_id,         FK garment_id; crdt_document_id UNIQUE
                                crdt_document_id)    "CRDT-GAR-{garment_id}"
tailor_material_presets        preset_id   MAT-     workspace_id; slug UNIQUE per ws;
                                                    compliance_json (raw aniso) + physics_json;
                                                    is_system_preset. (Supersedes the names
                                                    tailor_material_library (03/10) and
                                                    tailor_material (naming table). ONE name.)
tailor_avatars                 avatar_id   AVT-     workspace_id; source_kind CHECK;
                                                    measurements_mm_json. (Authored here; was the
                                                    undefined FK target.)
tailor_body_proxies            body_proxy_id BPX-   FK avatar_id -> tailor_avatars; proxy_json
                                                    (capsules+spheres, mm); mode/breast_proxy_mode
tailor_simulation_runs         sim_run_id  SIM-     FK garment_id; FK sandbox_run_id ->
                                                    kb003_sandbox_runs(run_id); solver_version;
                                                    substeps; iterations; content_hash;
                                                    result_artifact_ref. (Prefix is SIM-, ONE
                                                    form; supersedes 10's CSIM-.)
tailor_refit_runs              refit_run_id RFT-    FK garment_id; FK source/target body_proxy_id;
                                                    refit_mode CHECK; output_garment_id
tailor_trims                   trim_id     TRIM-    workspace_id; trim_category CHECK; mesh_json;
                                                    inertia_tensor_json; is_library_item;
                                                    converted_from_panel_id
tailor_trim_placements         placement_id PLAC-   FK garment_id; FK trim_id; tacks_json;
                                                    initial_pose_json; layer_order
tailor_zippers                 zipper_id   ZIP-     FK garment_id; panel_edge_a/b; slider_count CHECK
tailor_lacings                 lacing_id   LACE-    FK garment_id; eyelet_sequence_json
tailor_uv_islands              island_id   UVI-     FK garment_id; simulation_run_id (nullable);
                                                    panel_id; atlas_uv_min/max; flatten_method
                                                    ('arap' only); UNIQUE(garment_id,run,panel)
tailor_pbr_materials           material_id PBR-     workspace_id; *_map_ref columns; grain_angle_deg
tailor_graphic_layers          layer_id    GLYR-    FK garment_id; panel_id; z_order; blend_mode;
                                                    boundary_pinned; deleted_at (tombstone)
tailor_material_assignments    assignment_id ASGN-  FK garment_id; FK physics_preset_id; FK
                                                    pbr_material_id; UNIQUE(garment_id,panel_id)
tailor_wardrobe                wardrobe_id WRD-     workspace_id; name
```

Notes that resolve specific cross-file conflicts:

- **Material table is `tailor_material_presets`** (the `06` name), NOT `tailor_material_library`
  (`03`/`10`) and NOT `tailor_material` (the naming-table abbreviation). The library *concept* is
  the set of `is_system_preset = true` rows in this one table.
- **`tailor_garments` is the single garment table.** `03` and `10` each defined their own
  `tailor_garments` with different columns; the canonical column set is: `garment_id` (PK, TEXT,
  `GAR-`), `workspace_id`, `name`, `status` (CHECK: `draft|sandbox_pending|simulated|validated|
  promoted|rejected|archived`), `spec_json JSONB` (the `GarmentSpec`), `animation_json JSONB`
  (nullable), `body_proxy_id TEXT`, `wardrobe_id TEXT`, `promotion_receipt_id TEXT`,
  `event_ledger_event_id TEXT NOT NULL`, `created_at`, `updated_at`. The garment FK to its body
  proxy is `body_proxy_id` (not a separate `avatar_id` column — the avatar is reachable via the
  proxy, and the spec's `AvatarBinding.avatar_id` records the authored intent).
- **`animation_json` is a COLUMN on `tailor_garments`**, not a table (`15`'s decision; preserved).
- **`tailor_simulation_runs` sim-run id prefix is `SIM-`** (one form; `10`'s `CSIM-` is dropped).
- The `05` `tailor_body_proxies` had a `garment_id` FK; that is **removed** — a proxy belongs to an
  avatar, not a garment (a garment references a proxy via `tailor_garments.body_proxy_id`). This
  fixes the `05` model where a proxy was per-garment-and-per-avatar.

---

### [T-CONTRACTS.validation] ONE Consolidated ValidationDescriptor Check Catalog

**The drift:** validation checks are scattered across `03` (seam_closure, panel_overlap,
mesh_topology, gather_ratio, mesh_quality), `05` (deep-penetration, inter-layer, self-collision
count), `06` (NaN/Inf, bbox, stretch≠0, density>0), `07` (intersection-free, seam closure, UV
validity, mesh topology, convergence), `10` (mesh_not_empty, no_degenerate_triangles, seams_closed,
no_interpenetration, uv_coverage, material_params_valid + advisory low_energy, panel_count,
roundtrip), and `13` (trim_no_penetration, tack_seam_closure, zipper_tooth_alignment,
lacing_cord_length + advisory trim_gravity_stable, tack_strength_nonzero). No single catalog exists;
severities differ between files.

**Canonical decision.** ONE catalog. It is realized as the `TailorValidationDescriptor` wrapping the
KB003 `ValidationDescriptor` (the `10` binding). Two severities only:
**`Blocking`** (any failure prevents promotion) and **`Advisory`** (recorded, visible, never blocks
unless the gate runs with `treat_advisory_as_blocking = true`, which exists in
`PromotionGateInputs`). Each check has a stable `code` (the `09` `ValidationFinding.code` form) so
the model can pattern-match and self-correct.

```text
CHECK CATALOG  (code | severity | stage | what it asserts)

-- Fast pre-solver checks (run in author_garment, < 100ms, no solver) --
PANEL_CLOSURE          Blocking  fast   each panel polygon is a closed, non-self-intersecting loop
SEAM_EDGE_REF          Blocking  fast   every SeamSpec.from/to references a valid panel_id+edge_index
GATHER_RATIO_RANGE     Blocking  fast   every SeamSpec.gather_ratio in (0.0, 20.0]
FABRIC_RANGE           Blocking  fast   normalized FabricProperties fields in [0.0,1.0];
                                        density_g_m2 in [5,2000]; collision_thickness_mm in [0.1,5]
AVATAR_BINDING         Blocking  fast   AvatarBinding.avatar_id exists in tailor_avatars
MIN_PANEL_AREA         Blocking  fast   every panel area > 1.0 cm^2 (rejects degenerate panels)
WINDING                Advisory  fast   panel vertices counter-clockwise (auto-corrected; INFO if fixed)

-- Mesh-quality checks (run on the triangulated SolverMesh, pre-sim) --
MESH_TOPOLOGY          Blocking  mesh   manifold; no degenerate triangles; no open boundary except
                                        intended seam edges
MESH_TRIANGLE_QUALITY  Blocking  mesh   min triangle angle >= 10 deg; max aspect ratio <= 20
PANEL_OVERLAP          Advisory  mesh   no two panels occupy the same 3D region before draping

-- Post-simulation cloth checks --
MESH_NOT_EMPTY         Blocking  post   simulated vertex buffer non-empty
NO_DEGENERATE_TRIS     Blocking  post   no zero-area triangles in output mesh
SEAMS_CLOSED           Blocking  post   every seam constraint pair <= 1 mm separation at rest
NO_INTERPENETRATION    Blocking  post   no cloth particle deeper than -0.5 mm inside any body
                                        capsule/sphere (final frame only; skip intermediate substeps)
SELF_INTERSECTION      Advisory  post   self-collision pair count below mesh-explosion limit
UV_COVERAGE            Blocking  post   UV islands cover >= 95% of mesh surface (pattern accuracy)
UV_VALIDITY            Blocking  post   all UVs in [0,1]^2; no degenerate UV triangles (area > 1e-6)
DRAPE_CONVERGED        Advisory  post   final kinetic energy below threshold (solver converged)
PANEL_COUNT_MATCH      Advisory  post   simulated panel count == spec panel count
GARMENTCODE_ROUNDTRIP  Advisory  post   spec round-trips to GarmentCode JSON without loss

-- Multi-layer checks (when GarmentSpec has layered garments) --
INTERLAYER_SPACING     Blocking  post   no inter-layer pair closer than (t_inner+t_outer-tolerance)

-- Trim checks (when trim_placements present; T-TRIM-RIGID) --
TRIM_NO_PENETRATION    Blocking  post   no trim mesh triangle interpenetrates a cloth triangle (final)
TACK_SEAM_CLOSURE      Blocking  post   all tack distances <= 5 mm at end of draping
ZIPPER_TOOTH_ALIGN     Blocking  post   tooth-rail tacks within 1 mm of their panel edge positions
LACING_CORD_LENGTH     Blocking  post   no cord segment stretched beyond 200% rest length
TRIM_GRAVITY_STABLE    Advisory  post   no trim body translating > 50 mm/frame in final 10 frames
TACK_STRENGTH_NONZERO  Advisory  post   warn if any tack strength < 0.01 (maybe unintentionally loose)

-- Material-preset checks (model-authored preset drape test; T-FABRIC-MODELS) --
PRESET_NO_NAN          Blocking  preset no NaN/Inf in drape-test particle positions
PRESET_STRETCH_NONZERO Blocking  preset stretch compliance != 0 (zero diverges the solver)
PRESET_DENSITY_POS     Blocking  preset density > 0
PRESET_BBOX_PLAUSIBLE  Advisory  preset drape-test bbox within expected range for the claimed archetype

-- Refit checks (T-AUTOFIT) --
REFIT_INTERSECTION_FREE Blocking post   min(particle-capsule distance) >= -0.5 mm after refit
REFIT_SEAM_CLOSURE      Blocking post   mated seam edge-pair length diff < 1%
REFIT_CONVERGED         Advisory post   refit sim reached equilibrium (did not time out)
```

`ValidationReport::aggregate_blocks_promotion()` (the existing kernel method) decides promotion: any
`Blocking` failure ⇒ rejected. The `TailorValidationDescriptor` selects the applicable subset by
stage and by which optional features (trims, multi-layer, refit) the garment uses. This single
catalog supersedes the scattered lists in `03/05/06/07/10/13`.

---

### [T-CONTRACTS.determinism] Determinism-vs-Promotion Resolution (tolerance, not hash)

**The known issue (`KI-DETERMINISM-VS-PROMOTION`).** Solver determinism is correctly scoped
**per-backend** (T-CLOTH-SOLVER.determinism: same GPU backend + driver ⇒ identical result;
cross-backend float rounding differs because WGSL/Naga do not guarantee sub-expression ordering and
WGSL has no f64 — confirmed by wgpu issue #5329 and the float-atomic limitation). But the promotion
path (T-KERNEL-INTEGRATION + T-CLOTH-SOLVER) compares an **exact SHA-256 `content_hash`** of the
final position buffer. Across GPU backends, driver versions, or sandbox containers, identical-quality
drapes produce **different hashes**, so the `ValidationRunner`'s re-run-and-compare step yields
**spurious promotion failures**. This is a load-bearing gap and the fix must be specified here before
any implementation WP is gated. The currency findings independently confirm: the gap is still open,
and tolerance-based mesh comparison is the correct fix.

**Canonical resolution.** Two distinct mechanisms, used for two distinct purposes. Exact hash is kept
ONLY for same-machine idempotency; promotion equivalence uses a tolerance-based `MeshComparator`.

```text
content_hash  (SHA-256 of final position bytes; SolverResult.content_hash)
  PURPOSE: same-machine, same-run idempotency + EventLedger receipt fingerprint ONLY.
  USE: dedupe identical re-submissions on ONE machine/backend; record in the run receipt.
  NEVER USE: cross-backend promotion equivalence. The PromotionGate ValidatorRunner MUST NOT
             gate on content_hash equality.

MeshComparator (tolerance-based; the canonical promotion equivalence check)
  PRIMARY (continuous, epsilon-tolerant):
    per_vertex_position_deviation <= epsilon_mm   (default epsilon_mm = 0.1)
      compared vertex-for-vertex in the SAME canonical vertex order (vertex ordering is
      deterministic from mesh topology + constraint coloring, which is computed once at garment
      load and stored — T-CLOTH-SOLVER.determinism point 2, so order is stable cross-backend).
      Metric: max per-vertex Euclidean deviation, AND mean deviation, both reported.
  SECONDARY (exact, topology invariants — must match exactly):
    vertex_count          == expected
    triangle_count        == expected
    seam_edge_pair_count  == expected
    panel_count           == expected
  VERDICT: meshes are "equivalent for promotion" iff all SECONDARY invariants match exactly AND
           max per-vertex deviation <= epsilon_mm.
```

Where this binds:

- `SolverResult` keeps `content_hash: [u8; 32]` (T-CLOTH-SOLVER) — re-purposed to idempotency only.
- The `TailorValidationDescriptor` re-run-determinism step (when it re-runs a sim to confirm
  reproducibility) calls `MeshComparator::compare(a, b, epsilon_mm)`, NOT `a.content_hash ==
  b.content_hash`.
- `epsilon_mm` is a field on `SimRunParams` / the validation policy (default `0.1`), so the
  operator can tighten it for hero assets or loosen it for exploratory runs.
- **Animated runs** (T-ANIMATION RISK-5): wind turbulence uses `sin()`/`fract()` whose cross-vendor
  precision differs, so even per-vertex deviation can exceed a tight epsilon. For animated runs the
  comparator additionally accepts a **shape-envelope** match (per-frame bounding box within
  `bbox_epsilon_mm`, default `1.0`, plus `SEAMS_CLOSED`) as the equivalence basis, because the
  turbulence is aesthetic and exact per-vertex reproduction is neither required nor achievable
  cross-vendor.

`MeshComparator` is a small pure function in the `tailor-solver` crate
(`tailor-solver/src/compare.rs`), reused by the kernel validation runner via the `ClothSolver`
boundary. No external dependency. This is the canonical resolution; every topic that mentions
"content hash comparison" for promotion defers to it.

---

### [T-CONTRACTS.simulation-receipt] Canonical Model Feedback Type

To prevent receipt drift, the model's typed feedback is ONE type: `SimulationReceipt`
(T-MODEL-FIRST-API), schema id `hsk.tailor.simulation_receipt@1`, returned as MCP
`structuredContent`. Its `validation_findings: Vec<ValidationFinding>` carry the `code` values from
[T-CONTRACTS.validation], a `severity` of `"blocking" | "advisory" | "info"`, an optional
`affected_id` (panel_id/seam_id/trim_id), and an optional `suggested_fix { field_path,
suggested_value }` with a JSON-pointer path into `GarmentSpec`. `recommended_action` is one of
`promote_garment | edit_and_resimulate | correct_spec_first | requires_operator_action`. This is the
single self-correction contract; the MCP tool surface (`author_garment`, `simulate_garment`,
`edit_garment`, `promote_garment`, `get_garment`, `estimate_fabric_params`, plus the trim/uv/
animation tools) is defined in `09/13/14/15` and is unchanged — only the receipt/spec/finding *types*
are canonicalized here.

---

### [T-CONTRACTS.deferral] How Other Topics Defer to This One

When the implementation WP (or any later edit) consumes the package, it resolves contracts as:

1. **Type names / field names / units** → this topic ([T-CONTRACTS.garment-spec],
   [T-CONTRACTS.body-proxy]). The other topics' struct sketches are rationale + OSS evidence.
2. **Event variants + wire strings + event_family** → [T-CONTRACTS.event-types]. The superseded-names
   list is authoritative for reconciling old references.
3. **Schema IDs** → [T-CONTRACTS.schema-ids] (`hsk.tailor.*`).
4. **Migration names** → [T-CONTRACTS.migration-naming] (dated, `.down.sql` pair, no `0NNN_*`).
5. **Tables / columns / PK form** → [T-CONTRACTS.tables] (`TEXT PRIMARY KEY`, prefixed ids).
6. **Validation checks + severities** → [T-CONTRACTS.validation].
7. **Promotion equivalence** → [T-CONTRACTS.determinism] (`MeshComparator`, not hash).

Algorithms, GPU/WGSL design, OSS adaptation notes, MCP tool *behaviour*, and risk analysis remain
owned by their home topics; this topic owns only the **contract surfaces**.

Index hygiene for the editor who lands this file: move `T-CONTRACTS` from `index.yaml` →
`planned_topics` into `topics:`; mark `KI-CONTRACTS-DRIFT`, `KI-MIGRATION-COLLISION`, and
`KI-DETERMINISM-VS-PROMOTION` resolved (pointing at this file); refresh `KI-STALE-COUNT` to the
verified live count (67) and `KI-NAMING`'s schema half to "canonicalized in T-CONTRACTS".

---

### [T-CONTRACTS.open-questions] Residual Open Questions (not blocking the contract)

1. **Bidirectional 2D↔3D loop edge-length feedback** writes panel `vertices_cm` corrections as a
   CRDT delta (T-GARMENT-AUTHORING). The contract is settled (cm, `PanelSpec::vertices_cm`); the
   open part is purely *when* to surface the correction to the operator, which is UI, not contract.
2. **`tailor-solver` crate name vs the `handshake-cloth-solver` string** that appears once in
   `10`/`09`: the canonical crate name is **`tailor-solver`** (naming table). Any `handshake-cloth-
   solver` / `handshake-tailor-solver` reference is superseded.
3. **Per-tack animated compliance buffer layout** (T-ANIMATION OPEN-1) is a GPU-layout decision, not
   an authority-contract decision; it does not change any table or schema id here.
4. **UDIM tile column on `tailor_uv_islands`** (T-UV-TEXTURE Open Question A) is a planned additive
   column (`udim_tile_index INT NULL`), compatible with this schema; defer to post-MVP.

---

### [T-CONTRACTS.sources] Sources

Codebase ground truth (live `wtc-kernel-009`, inspected 2026-06-17):
1. `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/mod.rs` — KernelEventType (67 variants verified), `as_str()` SCREAMING_SNAKE_CASE wire format, `required_first_slice_events()`
2. `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/migrations/` — 0001..0335 numbered + `2026_05_18_fems_pinned.sql`(+`.down.sql`); `0334_loom_canvas_boards.sql` collision confirmed; 194 forward files
3. `…/migrations/0334_loom_canvas_boards.sql` — proves the 0334 collision (WP-KERNEL-009 MT-261)
4. `…/migrations/2026_05_18_fems_pinned.sql` + `.down.sql` — dated migration convention with reverse pair
5. `…/migrations/0332_media_asset_tiers.sql`, `0334_loom_canvas_boards.sql` — `TEXT PRIMARY KEY` prefixed-id convention (block_id, placement_id, tier_row_id, collection_id)
6. `…/src/atelier/` (`action_receipt.rs`, `annotation.rs`, `collections.rs`) — `tailor.<domain>.<verb>` event_family convention (mirrors `atelier.<domain>.<verb>`)
7. `…/src/atelier/mod.rs` — creative-module extension pattern Tailor follows

Package topics reconciled (this folder; the per-file authority surfaces this topic supersedes for contract shapes):
8. `03-garment-authoring.md` — GarmentSpecV1 (cm, gather_ratio, EdgeShape, material_id), tailor_garments v1, validation checks
9. `04-cloth-solver.md` — SolverResult.content_hash, per-backend determinism, SimRunParams, event-variant list
10. `05-collision.md` — ClothBodyProxy (capsule+sphere, cm), tailor_body_proxies (UUID PK), undefined tailor_avatars FK
11. `06-fabric-models.md` — ClothMaterialCompliance (raw aniso), tailor_material_presets, density kg/m² vs g/m², preset event variants
12. `07-autofit-retargeting.md` — ClothBodyProxyV1 (capsule-only, mm), tailor_body_proxies/tailor_refit_runs (UUID PK), refit events
13. `09-model-first-api.md` — GarmentSpec (cm, normalized [0,1] fabric), SimulationReceipt, ValidationFinding, MCP tools, GARMENT_* wire strings
14. `10-kernel-integration.md` — GarmentDraftV1 (normalized vertices, ratio, hsk.cloth.*), tailor_garments v2, tailor_simulation_runs (CSIM-), event taxonomy
15. `13-trim-rigid.md` — trim/zipper/lacing tables (TEXT PK, dated migration), trim event variants, GarmentDraftV1 extension
16. `14-uv-texture.md` — UV/PBR/graphic tables (UUID PK), ARAP flatten decision, uv/texture event variants
17. `15-animation.md` — animation_json JSONB column decision, animation event variants, mesh-shape comparison for animated determinism
18. `01-codebase-inventory.md` — stale 54-variant count + 0334_* migration proposal (both corrected here)
19. `index.yaml` — known_issues (KI-CONTRACTS-DRIFT, KI-MIGRATION-COLLISION, KI-DETERMINISM-VS-PROMOTION, KI-STALE-COUNT, KI-NAMING)

Current external references (verification of the determinism approach and naming targets, 2026):
20. wgpu issue #5329 — WGSL f32 atomic / data-race limitation underpinning per-backend (not cross-backend) determinism: https://github.com/gfx-rs/wgpu/issues/5329
21. MGPBD (SIGGRAPH 2025) — confirms Chebyshev-accelerated GS as the future solver upgrade; not a contract change: https://arxiv.org/abs/2505.13390
22. GarmentCode (ETH Zurich) — `"units": "cm"` round-trip target that grounds the cm units decision: https://github.com/maria-korosteleva/GarmentCode
23. ChatGarment (CVPR 2025) — the normalized-[0,1] 76-float LLM input that justifies the normalized FabricProperties surface (decoded to cm): https://github.com/biansy000/ChatGarment
24. MCP June 2025 spec — structuredContent/inputSchema basis for SimulationReceipt + schemars-derived GarmentSpec: https://modelcontextprotocol.io/specification/2025-06-18/server/tools
