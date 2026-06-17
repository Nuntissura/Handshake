---
file_id: cloth-engine-t-garment-authoring
topic_id: T-GARMENT-AUTHORING
title: "Garment Authoring: Patterns, Sewing, and Parametric/Model-First Definition"
status: draft
depends_on:
  - T-MD-FEATURES
  - T-CODEBASE
summary: "2D pattern/seam authoring system, parametric garment representation, LLM-steerable JSON schema, CRDT-editable Rust types, and Handshake-native kernel bindings for garment authority, sandbox, and promotion."
sources: 28
updated_at: "2026-06-17"
---

## [T-GARMENT-AUTHORING] Garment Authoring: Patterns, Sewing, and Parametric/Model-First Definition

This document covers the garment authoring layer of the Handshake Tailor engine: how 2D sewing
patterns and seams are represented as data, how open-source projects encode them, how an LLM can
emit a garment definition that drives the XPBD solver, and how the whole stack binds to the
Handshake kernel primitives (PostgreSQL/EventLedger authority, CRDT collaborative editing, sandbox,
validation, promotion, and model lanes).

The Tailor engine's authoring layer is the entry point for every downstream subsystem. The solver
consumes a triangulated mesh derived from authored panels; UV islands come from the 2D panel
outlines; material properties attach to panels; seam constraints are generated from stitch
definitions. Getting the authoring representation right is the critical design decision.

---

### [T-GARMENT-AUTHORING.md-feature-map] Marvelous Designer Feature Mapping

**Target feature groups from MD 2025–2026:**

| MD Feature | Engine Equivalent | Complexity |
|---|---|---|
| 2D Pattern window — rectangle, circle, polygon create tools | `PanelSpec` with vertex+edge encoding | Medium |
| Edit Pattern, Edit Curvature, Edit Curve Point | CRDT-tracked panel mutation | Medium |
| 3D Pen / 3D Pencil (2026.0) — sketch on avatar → 2D pattern | Model-driven 3D-to-2D projection tool | High |
| Pattern Drafter (beta 2025.1) — AI-assisted drafting | LLM → `GarmentDraftV1` JSON | High (differentiator) |
| Segment Sewing, Free Sewing, 1:N gathering sewing | `SeamSpec` with ratio field | Hard (MOAT-2) |
| Dart, Pleat (2025.2) | `DartSpec`, `PleatSpec` as derived panel mutations | Medium |
| Fold seam lines | `SeamKind::Fold` variant | Low |
| Lacing Tool (2026.0) — eyelet-based lacing pattern | `LacingSpec` as seam overlay | Medium |
| Pattern Archive (2026.0) — per-pattern version snapshots | EventLedger + CRDT snapshot per garment_id | Low (free via kernel) |
| Modular Library — save/load garment groups | `GarmentLibraryEntryV1` in Postgres authority | Medium |

The bidirectional 2D↔3D loop (MOAT-1) is the hardest feature: editing a panel in 2D must re-derive
the 3D simulation mesh, and simulation outputs must update panel edge lengths in 2D. This requires
the authoring representation to be the single source of truth that both the editor surface and the
XPBD solver read from.

---

### [T-GARMENT-AUTHORING.oss-pattern-representations] OSS Pattern Representations

#### GarmentCode (ETH Zurich — github.com/maria-korosteleva/GarmentCode)

The most complete open-source sewing-pattern intermediate representation. Published in SIGGRAPH
Asia 2023, major v2.0.0 released August 2024, online configurator restored June 2025.

**Core types (Python/pygarment library):**

- `Edge` — base type; subtypes: `StraightEdge`, `CircleEdge`, `CurveEdge` (Bézier), `EdgeSequence`
- `Panel` — flat 2D fabric piece; fields: edge loop (ordered directed edges), 6D placement (3D
  position + rotation quaternion), optional grain direction
- `Interface` — named set of panel edges that can participate in stitches; carries ruffle
  coefficient for gathering ratio
- `Component` — hierarchical composition of panels + sub-components + stitching rules
- `StitchingRule` — pairs two `Interface` objects; flattened to an edge-pair list on serialization

**Serialization (JSON, format extended from Korosteleva & Lee 2021):**

```json
{
  "pattern": {
    "panels": {
      "front_bodice": {
        "vertices": [[0, 0], [30, 0], [30, 50], [0, 50]],
        "edges": [
          {"endpoints": [0, 1], "curvature": null},
          {"endpoints": [1, 2], "curvature": {"type": "quadratic", "control": [32, 25]}},
          {"endpoints": [2, 3], "curvature": null},
          {"endpoints": [3, 0], "curvature": null}
        ],
        "translation": [0, 0, 10],
        "rotation": [0, 0, 0, 1]
      }
    },
    "stitches": [
      {
        "from": {"panel": "front_bodice", "edge": 1},
        "to": {"panel": "back_bodice", "edge": 3}
      }
    ]
  },
  "properties": {
    "name": "BasicShirt",
    "units": "cm"
  }
}
```

The mesh generation pipeline (GarmentCodeData, ECCV 2024) converts this to simulation-ready
triangles:
1. Place vertices at ~1 cm spacing along each edge (arc-length parameterization)
2. Run constrained Delaunay triangulation inside each panel outline
3. Place each panel at its 6D transform in 3D space
4. Merge vertex pairs across stitched edge pairs at midpoints
5. Preserve original edge lengths as rest-length constraints for the solver

**GarmentCodeRC (ChatGarment, CVPR 2025):** A "Richer & Cleaner" simplified variant with:
- Categorical garment-type + design-option fields rather than explicit panel/edge geometry
- All floats normalized to [0, 1]; 76 continuous parameters decoded from an MLP projection layer
- Average token count reduced from 900 → 350 for LLM fine-tuning
- GarmentCode engine decodes GarmentCodeRC JSON → full panel/edge/stitch representation

**NGL (Natural Garment Language, MPI arxiv 2602.20700, 2026):** Training-free variant. Restructures
GarmentCode parameters into vocabulary aligned with VLM natural-language descriptions. A
deterministic rule-based parser maps NGL output back to valid GarmentCode. Supports multi-layer
outfits. Uses logit-constrained VLM decoding to enforce schema-valid outputs.

**Design2GarmentCode (Style3D, CVPR 2025):** Synthesizes GarmentCode-compliant parametric programs
rather than raw JSON, via two fine-tuned agents (DSL-GA for grammar, MMUA for multimodal
understanding). Achieves 100% simulation success rate vs DressCode's 84%.

#### GarmageNet (Style3D, ACM ToG SIGGRAPH Asia 2025 — github.com/Style3D/garmagenet-impl)

Novel representation called "Garmage": each panel encoded as a structured **geometry image** (a
regular grid encoding surface position, normal, UV). A latent diffusion transformer synthesizes
panel-wise geometry images. `GarmageJigsaw` neural module predicts point-to-point sewing
connections along panel contours. Converts multi-modal inputs (text, sketch, point cloud) to
simulation-ready garment assets. Dataset: GarmageSet (10,000+ garments with structural/style
annotations). The geometry-image encoding bridges 2D structural patterns and 3D garment shapes.

#### AIpparel (George Nakayama et al., CVPR 2025 — georgenakayama.github.io/AIpparel)

Fine-tunes LLaVA-1.5-7B on GCD-MM (120,000+ garments with text, image, sewing pattern
annotations). Key innovation: a **sewing pattern tokenizer** that encodes each panel as a sequence
of special tokens incorporating vertex positions and 3D transforms via positional embeddings. The
first vertex in each panel's local frame is fixed at origin (0,0) as a normalization anchor.
Supports text-to-garment, image-to-garment, and edit-instruction-to-modified-garment tasks. Panels
are 2D planar surfaces with vertices and edges; edge endpoints pair to stitches. Uses a novel
fine-tuning objective for both discrete tokens and continuous parameters.

#### Dress-1-to-3 (arxiv 2502.03449, 2025)

Represents sewing patterns as sets of **quadratic Bézier curves** forming disconnected patches.
Fields: curve vertices `P = {P_i}` (patch boundary endpoints), control points `K = {K_e}` (one per
edge). Differentiable discretization: arc-length parameterization → boundary vertices; Delaunay
triangulation → interior vertices expressed via harmonic coordinates for gradient flow. Uses CIPC
(Codimensional Incremental Potential Contact) rather than XPBD, but the pattern representation is
solver-agnostic and directly applicable.

#### Freesewing (freesewing.org — archived to Codeberg April 2025)

JavaScript parametric pattern platform. Core types: `Part`, `Point`, `Path`, `Snippet`, `Option`.
Measurement-driven: each pattern option is a typed parameter with name, range, and
display metadata. Plugin system for dart/seam extensions. Useful as a reference for how a pattern
DSL exposes typed design options as first-class API parameters that an LLM can emit.

---

### [T-GARMENT-AUTHORING.garment-schema] Handshake-Native Garment Schema

The Handshake garment schema is the typed Rust/JSON authority for garment definitions. It must:
1. Be emittable by an LLM (structured JSON with bounded parameter sets)
2. Round-trip to/from GarmentCode JSON for interop with ChatGarment/AIpparel/NGL toolchains
3. Be stored as Postgres authority rows with EventLedger receipts on every mutation
4. Support CRDT-tracked collaborative editing of panel geometry and seam definitions
5. Feed the XPBD solver crate as a constraint buffer (panels → triangulated mesh → distance/bend
   constraints + seam constraints)

**Core Rust types for the `tailor` module:**

```rust
// src/tailor/garment.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// A 2D point in panel-local coordinate space (cm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

/// A 3D position/rotation for panel placement in 3D space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform3D {
    pub translation: [f32; 3],
    /// Unit quaternion [x, y, z, w]
    pub rotation: [f32; 4],
}

/// Edge shape variants. All in panel-local 2D coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EdgeShape {
    /// Straight edge (linear segment).
    Straight,
    /// Quadratic Bézier; one control point in panel-local coords.
    Quadratic { control: Vec2 },
    /// Cubic Bézier; two control points.
    Cubic { control_a: Vec2, control_b: Vec2 },
    /// Circular arc; signed curvature (positive = left).
    Arc { curvature: f32 },
}

/// One directed edge in a panel outline.
/// `endpoints` are indices into the `PanelSpec::vertices` array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeSpec {
    /// Indices [start, end] into parent panel's vertex array.
    pub endpoints: [usize; 2],
    pub shape: EdgeShape,
    /// Optional fold direction for fold seam lines (MD equivalent: Fold Seam Line).
    pub fold_angle_deg: Option<f32>,
}

/// A 2D pattern panel — one flat piece of fabric.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelSpec {
    /// Stable panel identifier within the garment (kebab-case, e.g. "front-bodice").
    pub panel_id: String,
    /// Vertices in panel-local 2D space (cm), forming the outline loop.
    pub vertices: Vec<Vec2>,
    /// Ordered directed edges connecting the vertices into a closed polygon.
    pub edges: Vec<EdgeSpec>,
    /// 6D placement of this panel in 3D space (for initial draping position).
    pub placement: Transform3D,
    /// Fabric grain direction as an angle from panel horizontal (deg). None = isotropic.
    pub grain_angle_deg: Option<f32>,
    /// Reference to a material preset applied to this panel.
    pub material_id: Option<String>,
}

/// Seam type variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeamKind {
    /// Standard join seam (default).
    Join,
    /// Fold/topstitch seam: cloth folds at this line rather than joining two pieces.
    Fold,
    /// Tack point constraint between two garment pieces or trim attachments.
    Tack,
}

/// One seam connecting two panel edges.
/// Supports 1:N and M:N ratio sewing for gathering (MD MOAT-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeamSpec {
    /// Stable seam identifier within the garment.
    pub seam_id: String,
    pub kind: SeamKind,
    /// Source side: panel_id + edge index.
    pub from: SeamEndpoint,
    /// Target side: panel_id + edge index.
    pub to: SeamEndpoint,
    /// Gathering ratio: length_from / length_to. 1.0 = flat seam; > 1.0 = gathering on `from`.
    /// Maps to MD's 1:N ratio sewing. The solver uses this to scale rest-length constraints.
    pub gather_ratio: f32,
}

/// One end of a seam definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeamEndpoint {
    pub panel_id: String,
    /// Index into `PanelSpec::edges`.
    pub edge_index: usize,
    /// Optional sub-range [0.0, 1.0] for Free Sewing (partial-edge seam).
    pub range: Option<[f32; 2]>,
}

/// A dart: removes a wedge from a panel to add 3D shaping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DartSpec {
    pub dart_id: String,
    pub panel_id: String,
    /// Dart tip vertex index in the panel.
    pub tip_vertex: usize,
    /// Two edge indices that form the dart's opening.
    pub opening_edges: [usize; 2],
    pub depth_cm: f32,
}

/// A pleat (2025.2 MD feature).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PleatSpec {
    pub pleat_id: String,
    pub panel_id: String,
    pub kind: PleatKind,
    pub count: u32,
    pub depth_cm: f32,
    pub interval_cm: f32,
    pub fold_angle_deg: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PleatKind {
    Knife,
    Box,
    Accordion,
}

/// The complete garment definition: panels + seams + darts + pleats.
/// This is the authority record stored in Postgres and emittable by LLMs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GarmentSpecV1 {
    pub schema_id: String,       // "hsk.tailor.garment_spec@1"
    pub garment_id: String,      // UUID v7 string
    pub workspace_id: String,
    pub name: String,
    pub panels: Vec<PanelSpec>,
    pub seams: Vec<SeamSpec>,
    pub darts: Vec<DartSpec>,
    pub pleats: Vec<PleatSpec>,
    /// Reference to body measurements this garment was fitted for.
    pub body_measurement_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Status in the promotion lifecycle.
    pub status: GarmentStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GarmentStatus {
    Draft,
    SandboxPending,
    Validated,
    Promoted,
    Rejected,
}
```

**Postgres migration** (added to the numbered migration sequence, e.g. `migrations/0151_tailor_garments.sql`):

```sql
-- tailor_garments: authority rows for promoted garment specs
CREATE TABLE IF NOT EXISTS tailor_garments (
    garment_id          TEXT PRIMARY KEY,
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'draft',
    spec_json           JSONB NOT NULL,
    body_measurement_id TEXT,
    event_ledger_event_id TEXT REFERENCES kernel_event_ledger(event_id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ix_tailor_garments_workspace
    ON tailor_garments (workspace_id);
CREATE INDEX IF NOT EXISTS ix_tailor_garments_status
    ON tailor_garments (workspace_id, status);

-- tailor_garment_crdt_docs: links each garment to its CRDT document for collaborative editing
CREATE TABLE IF NOT EXISTS tailor_garment_crdt_docs (
    garment_id          TEXT NOT NULL REFERENCES tailor_garments(garment_id),
    crdt_document_id    TEXT NOT NULL UNIQUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (garment_id, crdt_document_id)
);

-- tailor_material_library: named physical property presets
CREATE TABLE IF NOT EXISTS tailor_material_library (
    material_id         TEXT PRIMARY KEY,
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,
    properties_json     JSONB NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

---

### [T-GARMENT-AUTHORING.llm-emit-surface] LLM-Steerable Garment Authoring API

This is the key differentiator the foundation context identifies as MOAT-8: MD has no
LLM-steerable pattern authoring as of 2026.0. Tailor owns this gap.

**Three-tier LLM authoring approach (from OSS landscape):**

**Tier 1 — Parametric spec (cheapest, fastest, GarmentCodeRC-style):**
LLM emits a compact design-intent JSON with categorical + continuous parameters. A decoder function
expands this into a full `GarmentSpecV1`. This is what ChatGarment (CVPR 2025) demonstrates with 76
continuous parameters and a categorical garment-type field. Token count ~350. Best for interactive
dialogue, quick iteration, and constrained generation.

**Tier 2 — Direct panel/seam JSON (higher fidelity, AIpparel-style):**
LLM emits a `GarmentSpecV1`-compatible JSON directly with panel vertices, edge shapes, and seam
definitions. AIpparel (CVPR 2025) demonstrates this with a custom sewing pattern tokenizer that
encodes vertex positions and 3D transforms via positional embeddings. More expressive but requires
the model to be fine-tuned or prompted with schema examples.

**Tier 3 — Program synthesis (Design2GarmentCode-style, highest fidelity):**
LLM emits a Rust/Python-style parametric program that generates the `GarmentSpecV1` when executed.
The program references body measurements as variables. Design2GarmentCode (CVPR 2025) achieves 100%
simulation success rate with this approach. Highest fidelity; requires DSL fine-tuning but
eliminates numerical hallucination.

**Handshake `TailorModelAdapter` (Rust sketch):**

```rust
// src/tailor/model_adapter.rs

use crate::kernel::model_adapter::{ModelAdapter, ModelAdapterRequest, ModelAdapterOutput};
use crate::kernel::context_bundle::ContextBundle;
use crate::kernel::{KernelActor, KernelEventType, KernelResult};

/// Request payload passed to the LLM for garment authoring.
/// Serialized into ContextBundle::payload for ModelAdapterRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailorAuthoringContext {
    /// Natural language design intent from operator.
    pub design_intent: String,
    /// Optional reference garment for editing (Tier 2/3 edit mode).
    pub existing_garment_id: Option<String>,
    /// Body measurement parameters (bust, waist, hip, height, etc. in cm).
    pub body_measurements: serde_json::Value,
    /// Authoring tier hint: "parametric" | "panel_json" | "program".
    pub authoring_tier: String,
    /// Schema example to include in the system prompt for grounding.
    pub schema_hint: Option<String>,
}

/// The LLM's response, parsed from ModelAdapterOutput::artifact_payload.
/// Variant depends on authoring_tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tier", rename_all = "snake_case")]
pub enum TailorAuthoringOutput {
    /// Tier 1: compact parametric spec decoded to GarmentSpecV1 by decoder.
    Parametric {
        garment_type: String,
        design_options: serde_json::Value, // categorical fields
        continuous_params: Vec<f32>,       // 76 floats normalized [0,1]
    },
    /// Tier 2: direct GarmentSpecV1-compatible JSON.
    PanelJson {
        garment_spec: GarmentSpecV1,
    },
    /// Tier 3: GarmentCode-compatible Python/DSL program text.
    Program {
        program_text: String,
        program_kind: String, // "garmentcode_python" | "handshake_dsl"
    },
}
```

**ModelAdapter invocation flow:**

```
Operator prompt
    │
    ▼
TailorAuthoringContext::new(design_intent, body_measurements, tier)
    │
    ▼ serialize into ContextBundle
ModelAdapterRequest { context_bundle, actor: KernelActor::ModelAdapter("tailor-garment-v1") }
    │
    ▼ invoke via ModelAdapter trait (routes through LlmClient / ModelRuntime registry)
ModelAdapterOutput { artifact_payload: TailorAuthoringOutput, ... }
    │
    ▼ decode + validate
GarmentSpecV1 (draft)
    │
    ▼ write to tailor_garments (status='sandbox_pending') + emit EventLedger event
    │   KernelEventType::TailorGarmentDraftProposed
    │
    ▼ TailorSandboxAdapter::run()  [sandbox execution]
    │   → triangulate panels
    │   → run XPBD draping
    │   → return artifact_refs (mesh bundle)
    │
    ▼ ValidationRunner checks: mesh topology, seam closure, panel overlap
    │   → KernelEventType::TailorGarmentValidated
    │
    ▼ PromotionGate::evaluate()
    │   → PromotionDecisionV1 (Accepted | Rejected)
    │   → KernelEventType::TailorGarmentPromoted
    │
    ▼ tailor_garments status='promoted'; authority write to Postgres
```

**EventLedger event types** (new `KernelEventType` variants for the Tailor domain):

```rust
// Added to kernel/mod.rs KernelEventType enum:
// tailor domain
TailorGarmentDraftProposed,
TailorSimRunStarted,
TailorSimRunCompleted,
TailorSimRunRejected,
TailorGarmentValidated,
TailorGarmentValidationFailed,
TailorGarmentPromoted,
TailorGarmentPromotionRejected,
TailorGarmentCrdtUpdateRecorded,
TailorGarmentCrdtSnapshotTaken,
TailorMaterialLibraryUpdated,
```

Each variant follows the existing SCREAMING_SNAKE_CASE wire format and must be registered in
`required_first_slice_events()` in `kernel/mod.rs`.

---

### [T-GARMENT-AUTHORING.crdt-collaborative-editing] CRDT Collaborative Editing of Garment Panels

The kernel's CRDT layer (`kernel/crdt/`) already provides `CrdtUpdateRecordV1`,
`CrdtSnapshotRecordV1`, `KnowledgeStateVectorV1`, and the `yjs_bridge` / `promotion_bridge`
infrastructure. Garment panel collaborative editing maps directly onto this without new CRDT
infrastructure.

**Mapping:**
- Each `GarmentSpecV1` is a CRDT document: `garment_id` → `crdt_document_id`
- Panel vertex moves, edge curvature edits, seam definition changes → `CrdtUpdateRecordV1` rows
  in `kernel_crdt_updates` table, keyed by `actor_site`
- Concurrent edits from multiple operator sessions or from a model adapter running in parallel →
  causality resolution via `KnowledgeStateVectorV1` (BTreeMap<actor_site, u64> version vector)
- `ai_edit_proposal` submodule: model-proposed garment mutations arrive as proposals (not
  immediate authority writes), consistent with how AI edits work for documents today

**CRDT conflict resolution for panel geometry:**
- Vertex position: last-writer-wins per vertex index (vertex index is stable within a panel version)
- Edge shape: last-writer-wins per edge index
- Seam definitions: set-union of SeamSpec additions; deletions require explicit tombstone

**Claim promotion flow** (from `kernel/crdt/claim_promotion.rs` pattern):
When a model adapter proposes panel changes, they land in the CRDT sandbox layer
(`CrdtStorageAuthorityPosture::MemoryOnly` → promoted to `PostgresEventLedger` after
operator review or validation gate approval).

**No new CRDT infrastructure needed.** The Tailor module reuses `kernel_crdt_updates` table,
`CrdtUpdateRecordV1` schema, and `yjs_bridge` serialization for Yjs-compatible delta encoding of
panel geometry changes.

---

### [T-GARMENT-AUTHORING.sandbox-promotion-flow] Sandbox and Promotion Flow

Following the `kb003_storage.rs` / `PromotionGate` pattern exactly:

**Step 1 — Draft proposal (EventLedger: TailorGarmentDraftProposed):**
```rust
// storage glue: src/tailor/storage_glue.rs
pub async fn insert_garment_draft(
    pool: &PgPool,
    spec: &GarmentSpecV1,
    write_ctx: &WriteContext,
) -> TailorResult<String> {
    guard_authority_write(AuthorityMode::Postgres)?;
    sqlx::query!(
        "INSERT INTO tailor_garments (garment_id, workspace_id, name, status, spec_json, created_at, updated_at)
         VALUES ($1, $2, $3, 'draft', $4, now(), now())",
        spec.garment_id, spec.workspace_id, spec.name,
        serde_json::to_value(spec)?
    ).execute(pool).await?;
    // Emit EventLedger receipt
    let event = NewKernelEvent::builder(
        write_ctx.task_run_id.clone(),
        write_ctx.session_run_id.clone(),
        KernelEventType::TailorGarmentDraftProposed,
        KernelActor::ModelAdapter("tailor-garment-v1".into()),
    )
    .aggregate("tailor_garment", &spec.garment_id)
    .idempotency_key(format!("garment-draft-{}", spec.garment_id))
    .payload(json!({ "garment_id": spec.garment_id, "name": spec.name }))
    .source_component("tailor::storage_glue")
    .build()?;
    // ... insert to kernel_event_ledger via existing pattern
    Ok(spec.garment_id.clone())
}
```

**Step 2 — Sandbox solver run (TailorSandboxAdapter):**
Implements `SandboxAdapter` trait from `kernel/sandbox/adapter.rs`:
```rust
pub struct TailorSandboxAdapter {
    solver: Arc<dyn ClothSolver>,  // XPBD solver crate trait boundary
    adapter_id: String,
}

impl SandboxAdapter for TailorSandboxAdapter {
    fn kind(&self) -> AdapterKind {
        AdapterKind::process_tier("tailor-solver-v1", "XPBD Cloth Solver")
    }
    fn run(&self, run: &SandboxRunV1, workspace: &SandboxWorkspaceV1, policy: &SandboxPolicyV1)
        -> Result<AdapterRunOutcome, AdapterError>
    {
        // Deserialize GarmentSpecV1 from run artifact_refs
        // Call solver.triangulate_panels(&spec) -> TriangulatedMesh
        // Call solver.drape(&mesh, &body_proxy, &sim_params) -> DrapedMesh
        // Write mesh artifact bundle to workspace sandbox dir
        // Return Completed { artifact_refs: vec![mesh_bundle_ref] }
    }
}
```

**Step 3 — ValidationRunner checks:**
The `TailorValidationDescriptor` implements the validation descriptor pattern:
- `seam_closure_check`: every SeamSpec references valid panel_id + edge_index pairs
- `panel_overlap_check`: no two panels occupy the same 3D region before draping
- `mesh_topology_check`: triangulated mesh has no degenerate triangles, no open boundaries except
  intended seam edges, manifold topology
- `gather_ratio_check`: all `SeamSpec::gather_ratio` values in (0.0, 10.0] range (10x is extreme
  gathering; higher values indicate authoring error)

**Step 4 — PromotionGate:**
Reuses `PromotionGate::evaluate()` from `kernel/kb003_promotion/gate.rs`. Inputs:
- `SandboxRunV1` carrying the mesh artifact bundle ref
- `ValidationReport` from `TailorValidationDescriptor` checks
- `OperatorApprovalEvidence` for garments that affect authority library entries
- `Kb003ArtifactBundleV1` wrapping the mesh bundle

On `PromotionOutcome::Accepted`: update `tailor_garments.status` to `'promoted'`; emit
`KernelEventType::TailorGarmentPromoted`; write `PromotionReceiptV1` to `kb003_promotion_receipts`.

---

### [T-GARMENT-AUTHORING.pattern-to-mesh-pipeline] Pattern-to-Mesh Pipeline

The pattern-to-mesh pipeline converts a `GarmentSpecV1` into a simulation-ready triangulated mesh
consumed by the XPBD solver crate (`tailor-solver`).

**Algorithm (based on GarmentCodeData ECCV 2024 mesh generation, verified against OSS implementation):**

```
1. For each PanelSpec:
   a. Sample vertices along each EdgeSpec at arc-length spacing ≤ particle_distance_cm
      (EdgeShape::Straight → linear subdivision; Quadratic/Cubic → Bézier arc-length param;
       Arc → circular arc subdivision)
   b. Collect boundary vertices as an ordered polygon (the edge loop)
   c. Constrained Delaunay triangulation (CDT) over the polygon interior
      → use a Rust CDT library (e.g. spade crate) or implement ear-clipping for convex panels
   d. Apply PanelSpec::placement transform to convert panel-local 2D vertices → 3D world positions
   e. Assign each vertex: panel_id, local UV coords (= normalized 2D position in panel space)

2. For each SeamSpec:
   a. Find matching boundary vertex pairs on the two stitched edges
   b. If gather_ratio ≠ 1.0: resample the shorter side to match vertex count of longer side
      (maintaining equal arc-length spacing) — this is the 1:N gathering representation
   c. Create SeamConstraintRecord: pairs of (vertex_a_idx, vertex_b_idx, rest_length)
      where rest_length = 0 (seam fully closed) for standard Join seams
      or rest_length = distance for Tack seams

3. For each DartSpec:
   a. Remove the dart wedge from the panel mesh
   b. Add distance constraints between the two dart edges (rest_length = 0)

4. Assemble into SolverMeshV1 (input to tailor-solver):
```

```rust
// src/tailor/solver_binding.rs
// (this struct is serialized and passed to the tailor-solver crate)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMeshV1 {
    pub schema_id: String,    // "hsk.tailor.solver_mesh@1"
    pub garment_id: String,
    /// Flat array of 3D vertex positions [x, y, z, x, y, z, ...]
    pub vertex_positions: Vec<f32>,
    /// Flat array of triangle indices [i0, i1, i2, i0, i1, i2, ...]
    pub triangle_indices: Vec<u32>,
    /// Per-vertex: which panel_id the vertex belongs to (for material lookup)
    pub vertex_panel_ids: Vec<String>,
    /// Per-vertex: UV coordinate in panel-local space (for UV-from-pattern)
    pub vertex_uvs: Vec<[f32; 2]>,
    /// Seam constraints: vertex index pairs + rest lengths
    pub seam_constraints: Vec<SeamConstraintRecord>,
    /// Per-panel material properties (indexed by panel_id)
    pub material_map: std::collections::HashMap<String, MaterialPropertiesV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamConstraintRecord {
    pub vertex_a: u32,
    pub vertex_b: u32,
    /// Rest length in cm. 0.0 for closed seams; >0 for tack constraints.
    pub rest_length_cm: f32,
    /// Gathering ratio carried forward for compliance scaling.
    pub gather_ratio: f32,
    pub kind: SeamKind,
}
```

**UV-from-pattern accuracy (MD MOAT-6):** Because UV coordinates are derived directly from the 2D
panel-local vertex positions (before the 3D placement transform is applied), UV islands are exact
flattened pattern pieces. Fabric grain direction maps to UV orientation via
`PanelSpec::grain_angle_deg`. This is the correct behaviour that MD achieves and that conventional
3D UV unwrapping does not.

---

### [T-GARMENT-AUTHORING.bidirectional-2d-3d-loop] Bidirectional 2D↔3D Loop

MD's primary authoring moat (MOAT-1) is the live bidirectional link: edit 2D panel → 3D drape
updates; drape output → 2D edge lengths update. Implementing this correctly is the hardest part of
the authoring layer.

**Architecture for the Handshake bidirectional loop:**

```
GarmentSpecV1 (authority, Postgres)
    │
    ├─► [Pattern-to-Mesh pipeline] → SolverMeshV1 → XPBD solver → DrapedMeshV1
    │                                                                       │
    │                                                           [Unfurl/flatten pass]
    │                                                                       │
    └─◄─ Updated PanelSpec::vertices (edge length correction from drape) ──┘
```

**Key constraint:** The unfurl/flatten pass (drape → 2D update) must update `GarmentSpecV1` panels
as a CRDT delta, not a full replace, so concurrent operator panel edits are not overwritten by the
solver feedback.

**Flatten pass algorithm (reference: Dress-1-to-3 inverse approach):**
After draping, for each panel:
1. Extract the panel's triangle cluster from the draped mesh
2. Apply least-squares conformal mapping (LSCM) or ABF++ to flatten the draped 3D surface back to
   2D (minimizes angle distortion, preserves shape)
3. Compare flattened edge lengths to original `PanelSpec` edge lengths
4. If edge lengths differ by more than `flatten_tolerance_cm` (default 0.5 cm):
   - Emit a `GarmentEdgeLengthCorrection` CRDT proposal (not an authority write)
   - Surface the correction in the operator control room for acceptance/rejection

**Risk:** The flatten pass adds significant complexity and CPU cost. For the forward research phase,
the bidirectional loop can be deferred to a later milestone; the first milestone targets
unidirectional (panel → 3D drape only) with the flatten pass as a validation report output.

---

### [T-GARMENT-AUTHORING.module-layout] Module and Crate Layout

Following the atelier domain pattern (`src/atelier/mod.rs`):

```
src/backend/handshake_core/src/
└── tailor/
    ├── mod.rs              # pub module declarations + TailorEngineError
    ├── event_family.rs     # Tailor domain EventLedger event family constants
    ├── garment.rs          # GarmentSpecV1, PanelSpec, SeamSpec, DartSpec, PleatSpec (above)
    ├── solver_binding.rs   # SolverMeshV1, pattern-to-mesh pipeline, SeamConstraintRecord
    ├── model_adapter.rs    # TailorModelAdapter, TailorAuthoringContext/Output
    ├── sandbox_adapter.rs  # TailorSandboxAdapter implementing SandboxAdapter trait
    ├── validation.rs       # TailorValidationDescriptor (seam_closure, mesh_topology, etc.)
    ├── material.rs         # MaterialPropertiesV1, material preset library
    ├── simulation.rs       # SimParamsV1, keyframeable property support
    ├── storage_glue.rs     # Postgres CRUD + EventLedger emissions for Tailor domain
    └── api.rs              # Axum Router (GET/POST /tailor/garments/*)
```

**Standalone solver crate** (separate from `handshake_core`, added as Cargo workspace member):

```
tailor-solver/
├── Cargo.toml             # wgpu, bytemuck, glam, parry3d; no sqlx, no tauri
├── src/
│   ├── lib.rs             # pub trait ClothSolver: Send + Sync
│   ├── mesh.rs            # SolverMeshV1 deserialize + GPU buffer construction
│   ├── xpbd/
│   │   ├── mod.rs
│   │   ├── constraint_stretch.wgsl
│   │   ├── constraint_bend.wgsl
│   │   ├── constraint_seam.wgsl
│   │   └── collision.wgsl
│   ├── gpu_solver.rs      # WgpuClothSolver implementing ClothSolver
│   └── cpu_solver.rs      # CpuClothSolver fallback (softy-inspired)
└── examples/
    └── bevy_testbed/      # Bevy viewport for visual testing; NOT a dep of lib
```

**Trait boundary** between `handshake_core` and `tailor-solver`:

```rust
// tailor-solver/src/lib.rs
pub trait ClothSolver: Send + Sync {
    fn triangulate(&self, spec: &GarmentSpecV1) -> Result<SolverMeshV1, SolverError>;
    fn drape(
        &self,
        mesh: &SolverMeshV1,
        body_proxy: &BodyCollisionProxyV1,
        params: &SimParamsV1,
    ) -> Result<DrapedMeshV1, SolverError>;
    fn flatten(&self, draped: &DrapedMeshV1) -> Result<Vec<PanelFlattenResult>, SolverError>;
}
```

---

### [T-GARMENT-AUTHORING.seam-constraint-encoding] Seam Constraint Encoding for the XPBD Solver

The seam constraint is the primary link between the authoring representation and the solver.
Encoding details matter for correctness.

**Standard 1:1 seam (no gathering):**
- For each matched vertex pair (v_a, v_b) across the seam: add a distance constraint with
  `rest_length = 0`, `compliance = seam_compliance` (very stiff, e.g. 1e-7)
- During XPBD: `delta_x = (|x_a - x_b| - 0) * direction` with Lagrange multiplier update

**Gathering seam (gather_ratio > 1.0, MD 1:N sewing):**
- The "from" edge is longer than the "to" edge by factor `gather_ratio`
- Resample both edges to equal vertex count N during mesh generation
- For each of the N vertex pairs: add distance constraint with `rest_length = 0`
- The "from" side has more material compressed into length L/gather_ratio; this creates the
  physical gather. The solver sees N point constraints pulling equally-spaced material together.
- `compliance` for gather seams is slightly higher (softer) than flat seams to allow the fold
  formation: `compliance = seam_compliance * gather_softness_factor` (tune: 1.2–2.0)

**WGSL seam constraint shader (sketch):**

```wgsl
// tailor-solver/src/xpbd/constraint_seam.wgsl

struct SeamConstraint {
    vertex_a: u32,
    vertex_b: u32,
    rest_length: f32,
    compliance: f32,
}

@group(0) @binding(0) var<storage, read_write> positions: array<vec3<f32>>;
@group(0) @binding(1) var<storage, read>       inv_masses: array<f32>;
@group(0) @binding(2) var<storage, read>       seam_constraints: array<SeamConstraint>;
@group(0) @binding(3) var<storage, read_write> lambdas: array<f32>;
@group(0) @binding(4) var<uniform>             dt_sq_inv: f32;

@compute @workgroup_size(64)
fn solve_seam(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&seam_constraints)) { return; }

    let c = seam_constraints[idx];
    let pa = positions[c.vertex_a];
    let pb = positions[c.vertex_b];
    let diff = pa - pb;
    let dist = length(diff);
    if (dist < 1e-6) { return; }

    let constraint_val = dist - c.rest_length;
    let w_a = inv_masses[c.vertex_a];
    let w_b = inv_masses[c.vertex_b];
    let w_sum = w_a + w_b;
    if (w_sum < 1e-10) { return; }

    // XPBD: alpha = compliance / dt^2
    let alpha = c.compliance * dt_sq_inv;
    let delta_lambda = -(constraint_val + alpha * lambdas[idx]) / (w_sum + alpha);
    lambdas[idx] += delta_lambda;

    let dir = diff / dist;
    positions[c.vertex_a] += w_a * delta_lambda * dir;
    positions[c.vertex_b] -= w_b * delta_lambda * dir;
}
```

---

### [T-GARMENT-AUTHORING.garmentcode-interop] GarmentCode Interop

The Handshake `GarmentSpecV1` must round-trip to/from GarmentCode JSON to enable interop with
ChatGarment, AIpparel, NGL-Prompter, and Design2GarmentCode toolchains.

**Conversion functions (in `src/tailor/garment.rs`):**

```rust
impl GarmentSpecV1 {
    /// Convert FROM GarmentCode JSON (the etH Zurich open format).
    pub fn from_garmentcode_json(v: &serde_json::Value) -> Result<Self, ConversionError> {
        // Parse panels: v["pattern"]["panels"] -> HashMap<String, PanelSpec>
        // Parse stitches: v["pattern"]["stitches"] -> Vec<SeamSpec>
        // Assign garment_id = Uuid::new_v7()
        // Default material_id from v["properties"]["fabric_preset"] if present
        todo!()
    }

    /// Convert TO GarmentCode JSON.
    pub fn to_garmentcode_json(&self) -> serde_json::Value {
        // Flatten PanelSpec list to GarmentCode "panels" dict keyed by panel_id
        // Flatten SeamSpec list to GarmentCode "stitches" list
        // Embed properties block with name, units="cm"
        todo!()
    }

    /// Convert FROM ChatGarment GarmentCodeRC compact JSON (Tier 1 LLM output).
    /// Requires a DecoderRegistry that maps garment_type -> panel template factory.
    pub fn from_garmentcode_rc(
        v: &serde_json::Value,
        decoder: &GarmentCodeRcDecoder,
    ) -> Result<Self, ConversionError> {
        todo!()
    }
}
```

The `GarmentCodeRcDecoder` maps (garment_type, design_options, continuous_params) to a
`GarmentSpecV1` via a library of procedural panel templates — one template per garment category
(shirt, dress, pants, skirt, jacket). This mirrors the GarmentCode Python garment_programs library
but in Rust, compiled into the `tailor` module.

---

### [T-GARMENT-AUTHORING.risks-open-questions] Risks and Open Questions

**Risk 1 — Bidirectional loop complexity:**
The flatten/unfurl pass after draping (to feed edge length corrections back to the 2D authoring
representation) is nontrivial. LSCM requires a working conformal map implementation. This is the
most research-heavy part of the authoring layer.

Mitigation: Ship unidirectional (panel → drape) first. The flatten pass is a separate milestone.
Flag it as DEFERRED in the forward research plan.

**Risk 2 — Panel tessellation quality:**
Constrained Delaunay triangulation quality affects solver stability. Degenerate triangles
(very thin, very small) cause numerical instability in XPBD stretch constraints.

Mitigation: Add a `mesh_quality_check` in `TailorValidationDescriptor` that rejects meshes with
minimum triangle angle < 10° or maximum aspect ratio > 20. Use the Ruppert algorithm extension of
CDT if available in the chosen Rust CDT crate.

**Risk 3 — LLM hallucination of invalid panel geometry:**
LLMs emitting `PanelSpecV1` directly (Tier 2) may generate self-intersecting edge loops,
negative-area panels, or out-of-range vertex coordinates.

Mitigation: The `seam_closure_check` and `mesh_topology_check` in the `TailorValidationDescriptor`
gate catches these before promotion. Add a lightweight pre-validation in the `TailorModelAdapter`
before even submitting to the sandbox, to surface obvious geometry errors immediately in the chat
response.

**Risk 4 — GarmentCode interop fidelity:**
GarmentCode's Python panel templates use procedural logic (conditionals, loops) to generate
vertex positions from body measurements. Replicating this in Rust panel template factories may
miss edge cases for unusual body proportions.

Mitigation: Start with 5–8 common garment categories (shirt, basic dress, pants, skirt, jacket,
hood, sleeve, waistband) as panel templates. Use GarmentCodeData dataset (115k+ examples) as a
test corpus for round-trip accuracy. Accept lossy round-trips for exotic categories in v1.

**Risk 5 — Gathering seam physics quality:**
Ratio sewing (1:N) is one of MD's hardest features. The gather quality depends on how evenly
constraint particles distribute along the "from" edge and how the solver handles the resulting
stiffness imbalance between gathered and flat regions.

Mitigation: Research Velvet's Jacobi + delta-accumulation approach for handling per-constraint
compliance variation. Add a `gather_softness_factor` parameter to `SeamSpec` for per-seam tuning.
Visual regression tests in the Bevy testbed viewport against known gather references.

**Open Question 1:** Should the `GarmentSpecV1` store Bezier control points in panel-local 2D
(more compact, matches Dress-1-to-3 representation) or as 3D world-space positions (easier
for 3D Pencil / 3D Pen authoring)?

Recommendation: Panel-local 2D is the correct canonical storage (matches GarmentCode, AIpparel,
Dress-1-to-3); the 3D placement transform on `PanelSpec` handles world-space conversion.

**Open Question 2:** What Rust CDT library should be used for panel triangulation?

Candidates: `spade` (2D Delaunay + CDT, MIT, actively maintained 2025), `geo` crate with
earcut triangulation (simpler, no CDT quality guarantees). Recommendation: `spade` for its CDT
support and mesh quality guarantees.

**Open Question 3:** For the GarmentCodeRC Tier 1 decoder, how many garment template categories
are required for a useful v1?

Minimum viable: shirt, dress (A-line, shift), pants, skirt (straight, A-line), jacket/blazer,
sleeve (set-in, raglan), hood. This maps to ~8 template factories covering the GarmentCodeData
most common categories.

---

### [T-GARMENT-AUTHORING.sources] Sources

1. GarmentCode GitHub repository: https://github.com/maria-korosteleva/GarmentCode
2. GarmentCode paper (SIGGRAPH Asia 2023): https://arxiv.org/html/2306.03642
3. GarmentCodeData paper (ECCV 2024): https://arxiv.org/html/2405.17609v2
4. GarmentCodeData supplemental material: https://www.ecva.net/papers/eccv_2024/papers_ECCV/papers/07721-supp.pdf
5. pygarment PyPI: https://pypi.org/project/pygarment/
6. PyGarment library DeepWiki: https://deepwiki.com/maria-korosteleva/GarmentCode/6.1-pygarment-library
7. ChatGarment site: https://chatgarment.github.io/
8. ChatGarment paper v2 (CVPR 2025): https://arxiv.org/html/2412.17811v2
9. ChatGarment GitHub: https://github.com/biansy000/ChatGarment
10. NGL-Prompter paper (arxiv 2602.20700, 2026): https://arxiv.org/abs/2602.20700
11. Design2GarmentCode site: https://style3d.github.io/design2garmentcode/
12. Design2GarmentCode paper: https://arxiv.org/pdf/2412.08603
13. Design2GarmentCode CVPR 2025: https://cvpr.thecvf.com/virtual/2025/poster/33416
14. GarmageNet site: https://style3d.github.io/garmagenet/
15. GarmageNet GitHub: https://github.com/Style3D/garmagenet-impl
16. GarmageNet paper: https://arxiv.org/html/2504.01483v4
17. AIpparel site: https://georgenakayama.github.io/AIpparel/
18. AIpparel paper (CVPR 2025): https://arxiv.org/abs/2412.03937
19. Dress-1-to-3 paper: https://arxiv.org/html/2502.03449
20. Marvelous Designer 2D Pattern Editing docs: https://support.marvelousdesigner.com/hc/en-us/articles/900000675106-1-2D-Pattern-Editing-Create-Pattern-Tools
21. Marvelous Designer Dart docs: https://support.marvelousdesigner.com/hc/en-us/articles/47358304858265-Dart
22. Marvelous Designer Pleats docs: https://support.marvelousdesigner.com/hc/en-us/articles/51752931944857-Pleats
23. Marvelous Designer 2026.0 3D Pencil: https://digitalproduction.com/2026/04/15/marvelous-designer-2026-0-adds-3d-pencil-and-lacing/
24. HinaCloth XPBD GitHub: https://github.com/HinaPE/HinaCloth
25. XPBD paper (Macklin et al. 2016): https://matthias-research.github.io/pages/publications/XPBD.pdf
26. Handshake codebase — kernel/model_adapter.rs (inspected)
27. Handshake codebase — kernel/crdt/persistence.rs (inspected)
28. Handshake codebase — storage/kb003_storage.rs (inspected)
