---
file_id: cloth-engine-model-first-api
topic_id: T-MODEL-FIRST-API
title: "Model-First API and LLM Steering"
status: draft
depends_on:
  - T-GARMENT-AUTHORING
  - T-KERNEL-INTEGRATION
summary: "Primitives, API surfaces, and self-correction harness designed so an LLM drives garment/cloth/avatar creation directly — the crate public API equals the model API."
sources: 29
updated_at: "2026-06-17"
---

## [T-MODEL-FIRST-API] Model-First API and LLM Steering

This topic defines the differentiating capability of the Handshake Tailor engine: every public crate surface is designed to be issued by a language model, not a human cursor. The model is not a plugin layered on top of a human-facing API; the model API *is* the crate's primary public API. Human-facing affordances (sliders, panels) are projections derived from the same typed contracts the model calls.

This maps to **MOAT-8** from the feature survey: Marvelous Designer 2026 has no LLM-steerable sewing-pattern authoring. ChatGarment, GarmentDiffusion, AIpparel, NGL-Prompter, and Design2GarmentCode demonstrate that the field is converging on LLM-to-sewing-pattern pipelines, but none of them are embedded inside a deterministic local-first runtime with EventLedger authority, CRDT collaboration, and sandbox/promotion gates. Handshake owns that combination.

---

### [T-MODEL-FIRST-API.design-principle] Core Design Principle

The crate public API equals the model API. Every function, tool definition, and typed contract that an LLM calls is the same function a human trigger would invoke — there is no separate "model shim" layered on top of a human API. The consequences:

1. All inputs are typed JSON — no pixel coordinates, no drag handles, no stateful UI session required.
2. All outputs are typed JSON receipts — the model can read every result without parsing prose.
3. Every mutation emits an EventLedger receipt, so model actions are attributable, auditable, and replayable.
4. The model can self-correct by reading typed feedback from validation runs and simulation receipts.
5. The MCP gate is the model's entry point; the same gate enforces human-in-the-loop consent for irreversible mutations.

---

### [T-MODEL-FIRST-API.field-survey] Field Survey: LLM-to-Garment Pipelines

**ChatGarment (CVPR 2025, MPI — github.com/biansy000/ChatGarment)**

Fine-tunes LLaVA on GarmentCodeRC JSON. The LLM receives an image and/or a text prompt; it outputs structured JSON between `<STARTS>` and `<ENDS>` tokens. A projection MLP (5120 → 76 floats, all normalized to [0,1]) recovers continuous parameters from the embedding. The schema is flat and hierarchical:

```json
{
  "meta": { "upper": "T-shirt", "wb": "None", "bottom": "Pants" },
  "pants": { "length": 0.203, "width": 0.062, "flare": 0.516, "rise": 0.816,
              "cuff": { "type": "None" } }
}
```

Token count was reduced from ~900 to ~350 by stripping irrelevant keys per garment type. Editing is multi-turn: the old JSON is concatenated with a text edit instruction and re-submitted. GPT-4o generates well-formed editing prompts in evaluation. This demonstrates that a compact, flat typed JSON schema is the right LLM input/output format for garment authoring.

**NGL-Prompter (MPI, arxiv 2602.20700, Feb 2026)**

Introduces Natural Garment Language (NGL) as an intermediate between a VLM's natural-language garment description and GarmentCode JSON. The key insight: VLMs describe garments well in natural language but regress GarmentCode parameters poorly. NGL restructures GarmentCode into a VLM-legible form, then a deterministic mapping converts NGL back to valid GarmentCode. No fine-tuning needed — the pipeline queries off-the-shelf VLMs. Achieves state-of-the-art on geometry metrics and handles multi-layer outfits. Applicable to Handshake: the Tailor engine context bundle can carry a natural-language garment description; the model translates it to `GarmentSpec` JSON via an NGL-style intermediate.

**AIpparel (CVPR 2025, georgenakayama.github.io/AIpparel)**

Fine-tunes large multimodal models on 120,000+ unique garments. Uses a novel sewing-pattern tokenizer: each panel becomes specialized tokens with positional embeddings for vertex positions and 3D transforms. Output is decoded directly to simulation-ready sewing patterns. Enables language-instructed editing: an existing sewing pattern plus a text instruction produces a modified pattern. Demonstrates that panel-level tokenization (not flat float vectors) scales to full garment complexity.

**Design2GarmentCode (CVPR 2025, style3d.github.io/design2garmentcode)**

Two-agent pipeline: a DSL-GA (DSL Generation Agent) trained on GarmentCode grammar generates prompts for an MMUA (Multi-Modal Understanding Agent), which extracts design features from images/text/sketches. Two validation loops: rule-based validation ensures MMUA output supports a complete garment program; post-generation validation compares generated designs against inputs and suggests modifications. Achieves 100% simulation success rate for text-guided generation vs 84% for prior methods. The two-loop validation is the direct model for the Handshake Tailor engine's `ValidationRunner` integration: fast structural validation before the expensive solver run.

**Textile IR (Open Mode, arxiv 2601.02792, Jan 2026)**

Defines a seven-layer Verification Ladder from cheap syntactic checks (milliseconds) to full physics simulation (seconds) to industrial and LCA checks (minutes to hours). LLM generates pattern modifications as symbolic proposals in GarmentCode DSL; fast filters reject geometrically invalid patterns at parse time; physics simulation validates accepted candidates. The ladder gates expensive work behind cheap checks — exactly the shape of the Handshake sandbox/validation/promotion gate stack.

**DiffXPBD (ACM CGIT 2023, arxiv 2301.01396)**

Differentiable XPBD enables gradient-based estimation of cloth material parameters: compliance coefficients (weft/warp), bending stiffness, Lamé parameters, body shape PCA coefficients, and time-varying external forces. The forward pass stores constraint values and gradients; a discrete adjoint backward pass propagates through the simulation. Demonstrated on 26M+ DOF meshes with efficient sparse linear system solves. Applicable to Handshake: when a model proposes fabric properties and the simulation output deviates from a target, DiffXPBD-style inverse fitting can refine the parameters without re-prompting the LLM.

**MCP (Model Context Protocol, June 2025 spec, modelcontextprotocol.io)**

The de facto LLM tool-connectivity standard as of 2026: 10,000+ public MCP servers, adopted by all major providers. Tools expose `inputSchema` (JSON Schema) and `outputSchema` (JSON Schema) at discovery time; tool results carry `structuredContent` (typed JSON object) alongside `content` (text). The Handshake kernel already contains an MCP gate (`src/mcp/gate.rs`, `src/mcp/schema.rs`). Tailor exposes its model API as MCP tools through this gate, so any model connected to the Handshake MCP server can issue garment authoring calls without bespoke integration.

**rmcp / schemars (Rust MCP SDK)**

rmcp is the official Rust MCP SDK. Tools are defined with `#[tool]` macros; parameter types derive `schemars::JsonSchema` to auto-generate input schemas. `CallToolResult::success(vec![Content::json(value)])` returns structured JSON. `schemars` v1.0 uses `///` doc comments for field descriptions — the schema description doubles as LLM-facing documentation.

---

### [T-MODEL-FIRST-API.garment-spec-schema] The Canonical Garment Specification Schema

The `GarmentSpec` is the canonical typed contract that flows between the LLM and Tailor. It is the model API's primary input type. It is stored as JSONB in the `tailor_garments` Postgres table and carried in `ContextBundle.allowed_context` when invoking a `TailorModelAdapter`.

```rust
// tailor-solver/src/spec.rs  (standalone crate, no Handshake deps)
// Derives serde + schemars so it auto-generates MCP inputSchema descriptions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Top-level garment specification. This is the LLM's primary output type
/// and the solver's primary input type.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Complete specification for a garment: panels, seams, material, and avatar binding.")]
pub struct GarmentSpec {
    /// Unique identifier for this spec version (kebab-case, operator-assigned or LLM-generated).
    pub spec_id: String,
    /// Garment category. Controls which panels and seams are valid.
    pub garment_type: GarmentType,
    /// One entry per 2D pattern panel (bodice, sleeve, front, back, leg, etc.).
    pub panels: Vec<PanelSpec>,
    /// Seam definitions joining panel edges. Use ratio sewing for gather/pleat.
    pub seams: Vec<SeamSpec>,
    /// Fabric physical properties. Anisotropic: weft != warp.
    pub fabric: FabricProperties,
    /// Avatar reference for fit and collision proxy.
    pub avatar: AvatarBinding,
    /// Optional: time-varying property overrides for animation simulation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<SimKeyframe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "2D pattern panel with polygon outline and grain direction.")]
pub struct PanelSpec {
    /// Unique name within this garment (e.g. 'front-bodice', 'left-sleeve').
    pub panel_id: String,
    /// Polygon vertices in centimetres, counter-clockwise. Min 3 points.
    pub vertices_cm: Vec<[f32; 2]>,
    /// Grain direction angle in degrees (0 = vertical warp direction).
    pub grain_angle_deg: f32,
    /// 3D placement hint: position offset from avatar surface in cm.
    /// LLM can set this to [0,0,0] and let the solver fit.
    pub placement_offset_cm: [f32; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Seam joining two panel edge segments. Supports 1:1, 1:N (gathering), and M:N sewing.")]
pub struct SeamSpec {
    pub seam_id: String,
    /// Source panel and edge indices (start_vertex_idx, end_vertex_idx).
    pub from: SeamEdgeRef,
    /// Target panel and edge indices.
    pub to: SeamEdgeRef,
    /// Gathering ratio: from_length / to_length. 1.0 = straight seam.
    /// Values > 1.0 gather the 'from' edge onto the shorter 'to' edge.
    pub gather_ratio: f32,
    /// Whether to flip the normal at this seam (inside vs outside fold).
    pub flip_normal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SeamEdgeRef {
    pub panel_id: String,
    pub start_vertex_idx: usize,
    pub end_vertex_idx: usize,
}

/// Anisotropic fabric physical properties matching Marvelous Designer's
/// Stretch-Weft/Warp/Shear + Bending-Weft/Warp model.
/// All values are normalised [0.0, 1.0] where 1.0 = stiffest/most resistant.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Physical fabric properties. Weft = horizontal (cross-grain), Warp = vertical (grain direction).")]
pub struct FabricProperties {
    /// Preset name for LLM convenience. Applied first; individual fields override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<FabricPreset>,
    /// Resistance to stretch in the weft (cross-grain) direction.
    pub stretch_weft: f32,
    /// Resistance to stretch in the warp (grain) direction.
    pub stretch_warp: f32,
    /// Resistance to diagonal shear / crease propensity.
    pub shear: f32,
    /// Bending stiffness in the weft direction. High = denim/leather, Low = silk.
    pub bending_weft: f32,
    /// Bending stiffness in the warp direction.
    pub bending_warp: f32,
    /// Wrinkle frequency control: ratio of stiffness applied at buckling corners.
    pub buckling_ratio: f32,
    /// Mass per unit area in g/m^2. Drives inertia and gravity response.
    pub density_g_m2: f32,
    /// Collision thickness in mm: gap maintained between cloth and collision objects.
    pub collision_thickness_mm: f32,
    /// Friction coefficient [0.0, 1.0]. High = garment grips body, resists sliding.
    pub friction: f32,
    /// Internal damping [0.0, 1.0]. Damps velocity/jitter between substeps.
    pub internal_damping: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FabricPreset {
    Cotton, Denim, Silk, Jersey, Leather, Satin, Linen, Wool, Spandex, Chiffon,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AvatarBinding {
    /// Avatar asset ID from tailor_material_library or a built-in parametric body.
    pub avatar_id: String,
    /// Body measurement overrides for parametric avatars.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurements_cm: Option<BodyMeasurements>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BodyMeasurements {
    pub height_cm: f32,
    pub chest_cm: f32,
    pub waist_cm: f32,
    pub hips_cm: f32,
    pub inseam_cm: f32,
}

/// Time-varying property override for a specific simulation frame.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimKeyframe {
    /// Frame index (0-based).
    pub frame: u32,
    /// Override stretch_weft at this frame (MD's keyframeable shrinkage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stretch_weft: Option<f32>,
    /// Override stretch_warp at this frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stretch_warp: Option<f32>,
    /// Inflation pressure override (for inflatable objects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pressure: Option<f32>,
    /// Solidify (stiffness blend rigid<->soft) override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solidify: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GarmentType {
    Tshirt, Shirt, Jacket, Blazer, Dress, Skirt, Pants, Shorts, Bodice, Cape, Custom,
}
```

The schema derives `JsonSchema` via `schemars`. The MCP gate auto-generates the `inputSchema` for `tools/list` discovery from this derive, so the LLM sees field descriptions without any manual schema writing.

---

### [T-MODEL-FIRST-API.simulation-receipt] Simulation Receipt: The Model's Typed Feedback

When the solver completes (success or failure), Tailor emits a `SimulationReceipt` as the `structuredContent` in the MCP tool response. This is the primary feedback the model reads to self-correct.

```rust
// handshake_core/src/tailor/simulation.rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Typed receipt returned to the LLM after a simulation sandbox run.
/// Carries enough information for the model to diagnose failure and
/// propose a corrected GarmentSpec without human intervention.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimulationReceipt {
    pub sim_run_id: String,
    /// Human-readable status for the LLM to use in reasoning.
    pub status: SimStatus,
    /// Mesh statistics from the completed simulation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_stats: Option<MeshStats>,
    /// Validation findings from the ValidationRunner. Each finding
    /// names a specific seam, panel, or property that failed.
    pub validation_findings: Vec<ValidationFinding>,
    /// Drape quality score [0.0, 1.0] if simulation ran to completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drape_quality_score: Option<f32>,
    /// Whether self-intersections were detected in the final frame.
    pub self_intersections_detected: bool,
    /// Whether any seam edge remained open (not fully closed at rest).
    pub open_seam_detected: bool,
    /// Recommended next action for the model to take.
    pub recommended_action: RecommendedAction,
    /// Artifact bundle ID if simulation was promoted to authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promoted_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimStatus {
    /// Solver completed all substeps; mesh is valid.
    Completed,
    /// Solver ran but output failed validation (open seams, intersections).
    CompletedWithIssues,
    /// Garment spec failed fast validation (malformed panels, invalid seam refs).
    RejectedAtValidation,
    /// Sandbox was denied before the solver ran (policy, capability check).
    SandboxDenied,
    /// Solver exceeded the allowed substep budget.
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MeshStats {
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub particle_distance_mm: f32,
    pub sim_frames: u32,
    pub substeps_per_frame: u32,
    pub solver_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidationFinding {
    /// Machine-readable finding code for pattern matching.
    pub code: String,
    /// Severity: 'error' (blocks promotion), 'warning' (advisory), 'info'.
    pub severity: String,
    /// The specific panel_id or seam_id the finding refers to, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_id: Option<String>,
    /// LLM-readable description of what went wrong and how to fix it.
    pub description: String,
    /// Suggested field path in GarmentSpec and suggested value range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<SuggestedFix>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuggestedFix {
    /// JSON pointer path into GarmentSpec (e.g. "/fabric/stretch_weft").
    pub field_path: String,
    /// Suggested value or range as a string.
    pub suggested_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedAction {
    /// Receipt carries a promotion gate approval; model should call promote_garment.
    PromoteGarment,
    /// Simulation had issues; model should edit GarmentSpec and re-simulate.
    EditAndResimulate,
    /// Fast validation failed; model should correct the spec before simulating.
    CorrectSpecFirst,
    /// Sandbox denied; operator must adjust policy before model can proceed.
    RequiresOperatorAction,
}
```

The `ValidationFinding` list is the model's self-correction fuel: each finding names the exact panel or seam that failed, gives a `suggested_fix` with a JSON pointer path and value range, and the `recommended_action` tells the model what to do next. The model does not need to parse prose — it pattern-matches on `finding.code` and applies `suggested_fix`.

---

### [T-MODEL-FIRST-API.mcp-tools] MCP Tool Definitions Exposed to the LLM

Tailor exposes five MCP tools through the Handshake MCP gate. These are the entire model-facing API surface. All are `#[tool]`-annotated functions in `handshake_core/src/tailor/api.rs`; parameter structs derive `JsonSchema` for automatic `inputSchema` generation.

```rust
// handshake_core/src/tailor/mcp_tools.rs
// Registered through the existing MCP gate (src/mcp/gate.rs) via the
// tailor tool router. No new gate infrastructure needed.

use rmcp::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Tool 1: Author a new garment from a text description or structured spec.
/// The LLM calls this to create a new GarmentDraft in the sandbox.
/// Returns a SimulationReceipt with status=RejectedAtValidation if the
/// spec is structurally invalid, so the model can correct it without
/// running the expensive solver.
#[tool(description = "Create a new garment draft from a GarmentSpec. \
    The spec is fast-validated before any solver run. Returns a \
    draft_id and ValidationFindings for the model to correct.")]
async fn author_garment(
    Parameters(AuthorGarmentInput { workspace_id, spec }): Parameters<AuthorGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(description = "Input for authoring a new garment draft.")]
pub struct AuthorGarmentInput {
    #[schemars(description = "Workspace to create the garment draft in.")]
    pub workspace_id: String,
    #[schemars(description = "Full GarmentSpec. All panels and seams required. \
        Use fabric.preset for common materials; override individual fields as needed.")]
    pub spec: GarmentSpec,
}

/// Tool 2: Run the XPBD cloth solver on an existing draft.
/// Triggers a SandboxAdapter run (process tier, no network, scoped fs).
/// Streams progress updates; returns a SimulationReceipt when done.
/// The model reads the receipt to decide whether to promote or edit.
#[tool(description = "Run the XPBD cloth solver on a garment draft. \
    Validates collision, seam closure, and drape quality. \
    Returns a SimulationReceipt with ValidationFindings for self-correction.")]
async fn simulate_garment(
    Parameters(SimulateGarmentInput { draft_id, substeps, solver_iterations, frames }):
        Parameters<SimulateGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulateGarmentInput {
    pub draft_id: String,
    /// Substeps per frame [1..64]. More = more accurate collision. Default: 8.
    #[serde(default = "default_substeps")]
    pub substeps: u32,
    /// Constraint solver iterations per substep [1..32]. Default: 4.
    #[serde(default = "default_iterations")]
    pub solver_iterations: u32,
    /// Number of simulation frames to run. Default: 30 (one second at 30fps).
    #[serde(default = "default_frames")]
    pub frames: u32,
}

/// Tool 3: Edit a garment draft. Accepts a partial GarmentSpec patch.
/// The model calls this after reading a SimulationReceipt to apply
/// the suggested fixes. Uses JSON Merge Patch semantics (RFC 7396):
/// only specified fields are updated; unspecified fields are preserved.
#[tool(description = "Apply a partial update to a garment draft using JSON Merge Patch. \
    Use ValidationFinding.suggested_fix.field_path to target specific fields. \
    Returns an updated draft_id for the next simulate_garment call.")]
async fn edit_garment(
    Parameters(EditGarmentInput { draft_id, patch }): Parameters<EditGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditGarmentInput {
    pub draft_id: String,
    /// JSON Merge Patch (RFC 7396) against the existing GarmentSpec.
    /// Example: {"fabric": {"stretch_weft": 0.3}} overrides only stretch_weft.
    pub patch: serde_json::Value,
}

/// Tool 4: Promote a successfully simulated garment to PostgreSQL authority.
/// Requires the SimulationReceipt to carry recommended_action=PromoteGarment.
/// Triggers the PromotionGate; operator consent is required for this step.
/// On success, returns a garment_asset_id (Postgres authority row).
#[tool(description = "Promote a simulated garment draft to authority storage. \
    Only valid when the SimulationReceipt carries recommended_action=PromoteGarment. \
    Requires operator consent via the MCP gate before writing authority rows.")]
async fn promote_garment(
    Parameters(PromoteGarmentInput { draft_id, sim_run_id, label }):
        Parameters<PromoteGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PromoteGarmentInput {
    pub draft_id: String,
    /// sim_run_id from the SimulationReceipt (idempotency key for promotion gate).
    pub sim_run_id: String,
    /// Human-readable label for the authority garment asset.
    pub label: String,
}

/// Tool 5: Read the current state of a garment draft or authority asset.
/// Returns the full GarmentSpec plus the latest SimulationReceipt if any.
/// The model uses this to reload context after a session boundary.
#[tool(description = "Read the current GarmentSpec and latest SimulationReceipt \
    for a draft or authority garment. Use this to reload state after a handoff.")]
async fn get_garment(
    Parameters(GetGarmentInput { garment_id }): Parameters<GetGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetGarmentInput {
    pub garment_id: String,
}
```

The MCP gate validates every tool call against `SandboxPolicyV1` before execution. `promote_garment` requires `ConsentDecision::Allow` from the `ConsentProvider` (operator must confirm). The remaining tools are non-destructive and can run without explicit consent under a standard operator policy.

---

### [T-MODEL-FIRST-API.self-correction-loop] Self-Correction Loop Design

The model self-correction loop follows the Textile IR seven-layer ladder principle: cheap checks first, expensive solver last. The loop is fully machine-readable: no natural-language parsing required.

```
┌─────────────────────────────────────────────────────────────────┐
│  LLM issues author_garment(spec)                                │
│                          │                                       │
│                          ▼                                       │
│  Fast Validation (< 100ms, no solver)                           │
│  ├── Panel closure check (polygon validity)                      │
│  ├── Seam edge reference check (panel_id + vertex_idx valid)    │
│  ├── gather_ratio range check [0.1 .. 20.0]                    │
│  ├── Fabric property range check [0.0 .. 1.0]                  │
│  └── Avatar binding check (avatar_id exists)                    │
│                          │                                       │
│              ┌───────────┴──────────┐                           │
│         PASS │                      │ FAIL                       │
│              ▼                      ▼                           │
│  draft created              SimulationReceipt(                  │
│              │                status=RejectedAtValidation,      │
│              │                validation_findings=[...],        │
│              │                recommended_action=CorrectSpecFirst)│
│              │                      │                           │
│              │              LLM reads findings,                 │
│              │              applies patch via edit_garment,     │
│              │              re-calls author_garment   ◄─────────┘
│              │                                                   │
│              ▼                                                   │
│  LLM calls simulate_garment(draft_id, substeps=8, frames=30)   │
│                          │                                       │
│                          ▼                                       │
│  XPBD Solver (GPU, sandbox process)                             │
│  ├── Anisotropic stretch constraints (weft/warp/shear)          │
│  ├── Bending constraints (weft/warp dihedral)                   │
│  ├── Seam distance constraints (with gather_ratio scaling)      │
│  ├── Collision response (avatar capsule proxies via parry)      │
│  └── Self-collision (spatial hash, optional)                    │
│                          │                                       │
│              ┌───────────┴──────────┐                           │
│     PASS     │                      │ FAIL                       │
│     (drape   ▼                      ▼                           │
│     quality  │              SimulationReceipt(                  │
│     >= 0.7)  │                status=CompletedWithIssues,       │
│              │                self_intersections_detected=true, │
│              │                validation_findings=[             │
│              │                  {code:'SEAM_GAP',              │
│              │                   affected_id:'front-bodice',   │
│              │                   suggested_fix:{               │
│              │                     field_path:'/panels/0/vertices_cm',│
│              │                     suggested_value:'increase polygon area'}}],│
│              │                recommended_action=EditAndResimulate)│
│              │                      │                           │
│              │              LLM reads findings,                 │
│              │              calls edit_garment(patch),          │
│              │              calls simulate_garment again ◄──────┘
│              │                                                   │
│              ▼                                                   │
│  SimulationReceipt(                                             │
│    status=Completed,                                            │
│    drape_quality_score=0.83,                                    │
│    recommended_action=PromoteGarment)                           │
│              │                                                   │
│              ▼                                                   │
│  LLM calls promote_garment(draft_id, sim_run_id, label)        │
│  [operator consent gate]                                        │
│              │                                                   │
│              ▼                                                   │
│  Garment authority row in PostgreSQL                            │
│  EventLedger: TailorGarmentPromoted                            │
└─────────────────────────────────────────────────────────────────┘
```

The loop is bounded. A `TailorSandboxAdapter` policy (`SandboxPolicyV1`) enforces a maximum substep budget and iteration count; the solver returns `SimStatus::TimedOut` rather than running indefinitely. The model is told the budget upfront via the tool description and can choose lighter settings for exploratory iterations.

---

### [T-MODEL-FIRST-API.cloth-model-adapter] TailorModelAdapter: Kernel Integration

The `TailorModelAdapter` is the Handshake kernel-level entry point for LLM-driven garment authoring. It implements the `ModelAdapter` trait (`src/kernel/model_adapter.rs:122`). The MCP tools call into this adapter; the adapter converts the `GarmentSpec` into a `ContextBundle` and issues an `artifact_proposal` pointing to the serialized spec.

```rust
// handshake_core/src/tailor/model_adapter.rs

use async_trait::async_trait;
use serde_json::json;

use crate::kernel::context_bundle::ContextBundle;
use crate::kernel::model_adapter::{
    ArtifactProposalDraft, ModelAdapter, ModelAdapterOutput, ModelAdapterRequest,
    KernelToolRequest,
};
use crate::kernel::{KernelActor, KernelEventType, KernelResult};

pub struct TailorModelAdapter {
    pub adapter_id: String,
}

#[async_trait]
impl ModelAdapter for TailorModelAdapter {
    fn adapter_id(&self) -> &str { &self.adapter_id }

    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput> {
        // allowed_context carries the GarmentSpec as JSONB.
        let context = &request.context_bundle.allowed_context;
        // Deserialize to GarmentSpec for validation.
        let spec: GarmentSpec = serde_json::from_value(context.clone())
            .map_err(|e| KernelError::InvalidEvent("invalid GarmentSpec in context bundle"))?;
        // Run fast validation before any artifact is created.
        let findings = fast_validate(&spec);
        let payload = json!({
            "adapter_id": self.adapter_id,
            "context_bundle_id": request.context_bundle.context_bundle_id,
            "garment_type": spec.garment_type,
            "panel_count": spec.panels.len(),
            "seam_count": spec.seams.len(),
            "validation_findings": findings,
        });
        let output_hash = sha256_hex(&canonical_json_bytes(&payload));
        Ok(ModelAdapterOutput {
            adapter_id: self.adapter_id.clone(),
            context_bundle_id: request.context_bundle.context_bundle_id,
            response_text: format!("tailor-adapter:{}", &output_hash[..16]),
            response_event_type: KernelEventType::ModelResponseRecorded,
            tool_request: KernelToolRequest {
                tool_request_id: format!("TOOLREQ-{}", &output_hash[..16]),
                event_type: KernelEventType::ToolRequestRecorded,
                tool_id: "tailor.author_garment".to_string(),
                reason: "garment draft proposal from model".to_string(),
            },
            artifact_proposal: ArtifactProposalDraft {
                artifact_proposal_id: format!("AP-{}", &output_hash[16..32]),
                event_type: KernelEventType::ArtifactProposed,
                artifact_kind: "garment_draft".to_string(),
                content_hash: output_hash.clone(),
            },
            artifact_payload: payload,
            output_hash,
        })
    }
}
```

New `KernelEventType` variants required for the cloth domain (added to `kernel/mod.rs` enum and `required_first_slice_events()`):

```rust
// New cloth domain events — SCREAMING_SNAKE_CASE per convention.
TailorGarmentDraftProposed,       // "GARMENT_DRAFT_PROPOSED"
TailorSimRunStarted,       // "GARMENT_SIM_RUN_STARTED"
TailorSimRunCompleted,     // "GARMENT_SIM_RUN_COMPLETED"
TailorSimRunRejected,      // "GARMENT_SIM_RUN_REJECTED"
TailorGarmentPromoted,     // "GARMENT_PATTERN_PROMOTED"
TailorCrdtUpdateRecorded,  // "GARMENT_CRDT_UPDATE_RECORDED"
```

Every MCP tool call that mutates garment state emits one of these events via `NewKernelEvent::builder(...)` exactly as the existing `kb003_storage.rs` pattern shows.

---

### [T-MODEL-FIRST-API.context-bundle-design] Context Bundle Design for Garment Authoring

The `ContextBundle.allowed_context` is the single structured payload the model sees when authoring a garment. It must contain everything the model needs without requiring additional tool calls to reconstruct session state.

```json
{
  "task": "author_garment",
  "workspace_id": "WS-abc123",
  "operator_brief": "Create a fitted jersey T-shirt for Avatar1. Chest 86cm, waist 68cm. Short sleeves.",
  "avatar_summary": {
    "avatar_id": "avatar1-smplx-default",
    "height_cm": 165.0,
    "chest_cm": 86.0,
    "waist_cm": 68.0,
    "hips_cm": 92.0
  },
  "garment_history": [],
  "available_presets": ["cotton", "jersey", "denim", "silk", "leather"],
  "solver_budget": {
    "max_substeps": 16,
    "max_iterations": 8,
    "max_frames": 60,
    "max_particles": 50000
  },
  "ngl_description": "Short-sleeved fitted jersey top. Round neckline. Hip-length. Slight ease at chest.",
  "reference_spec_id": null
}
```

The `ngl_description` field is the NGL-Prompter-style intermediate: a natural-language garment description the LLM uses as a planning step before emitting the full `GarmentSpec`. The model is instructed to reason from this description into panel shapes and seam definitions. If a `reference_spec_id` is present (editing session), the model loads the existing spec via `get_garment` and applies a JSON Merge Patch.

---

### [T-MODEL-FIRST-API.fabric-estimation] Fabric Parameter Estimation (Inverse Path)

When the model cannot determine fabric parameters from a text description alone, it can request inverse parameter estimation from a reference image or a target drape shape. This uses the DiffXPBD gradient-based approach as an optional second-pass tool.

```rust
/// Tool (optional, advanced): Estimate fabric parameters from a target drape image.
/// Runs a DiffXPBD-style forward-backward pass in the sandbox to optimise
/// fabric properties to match the target. Returns an estimated FabricProperties
/// struct the model can insert into its GarmentSpec patch.
#[tool(description = "Estimate FabricProperties to match a target drape image. \
    Requires an existing draft_id (panels and seams must be set). \
    Returns FabricProperties the model can use in an edit_garment patch.")]
async fn estimate_fabric_params(
    Parameters(EstimateFabricInput { draft_id, target_image_artifact_id, max_iterations }):
        Parameters<EstimateFabricInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EstimateFabricInput {
    pub draft_id: String,
    /// Artifact ID of a reference image showing the desired drape.
    pub target_image_artifact_id: String,
    /// Gradient descent iterations. More = closer match but slower. Default: 50.
    #[serde(default = "default_fabric_iterations")]
    pub max_iterations: u32,
}
```

The implementation runs the `handshake-tailor-solver` crate's differentiable path (if compiled with the `diff-xpbd` feature flag) inside the sandbox. The optimiser runs gradient descent on `FabricProperties` parameters against the target silhouette/drape error. The returned `FabricProperties` is included in a `SuggestedFabricParams` receipt; the model appends it to an `edit_garment` patch.

This path is optional (feature-flagged) because the differentiable XPBD path is ~10x slower than the forward-only path. The model should attempt forward simulation with preset-derived parameters first and only invoke `estimate_fabric_params` when the drape is visually wrong and a reference image is available.

---

### [T-MODEL-FIRST-API.crdt-collab] CRDT Collaboration Surface for Garment Editing

Multiple model instances (or a model and a human) can edit the same garment draft concurrently. Tailor maps garment editing onto the existing CRDT layer (`kernel/crdt/`):

- Each garment draft maps to a `crdt_document_id` in `kernel_crdt_updates`.
- Panel vertex edits are `CrdtUpdateRecordV1` rows keyed by `(garment_id, panel_id, actor_site)`.
- Seam edits are separate `CrdtUpdateRecordV1` rows so panel and seam edits can merge independently.
- `KnowledgeStateVectorV1` tracks per-actor version vectors; `causality_verdict` detects concurrent edits.
- Concurrent edits to the same panel (two models both repositioning vertices) are resolved by `promote_bridge`: last-writer-wins within a substep unless a model explicitly holds a lease via `KnowledgeCrdtLeaseClaimed`.

The model API for collaborative editing uses the same `edit_garment` tool: the CRDT machinery is invisible to the model — the tool server handles merge and emits `TailorCrdtUpdateRecorded` events. The model reads the merged state via `get_garment`.

---

### [T-MODEL-FIRST-API.llm-steerability-system-prompt] System Prompt / Model Manual Surface

Per `GLOBAL-BUILD-002` through `GLOBAL-BUILD-011`, Tailor must include a built-in model manual that lets any LLM with no prior context operate the tool. The manual is embedded in the MCP server's capability advertisement and in the `ContextBundle.allowed_context` when the `task = "author_garment"`.

Key sections the manual must cover:

1. **Tool order**: always call `author_garment` first (fast validation), then `simulate_garment`, then optionally `estimate_fabric_params`, then `promote_garment`.
2. **Self-correction**: read `SimulationReceipt.validation_findings` after every simulate; use `suggested_fix.field_path` + `suggested_fix.suggested_value` in an `edit_garment` patch; loop until `recommended_action=PromoteGarment` or a hard stop condition.
3. **Hard stop conditions**: `>5` simulate iterations with the same panel configuration; `SimStatus::SandboxDenied`; `RecommendedAction::RequiresOperatorAction`.
4. **Fabric presets**: use `FabricPreset` enum values for common materials; only override individual fields if a specific tactile property is required.
5. **Avatar measurements**: if avatar body measurements are in the operator brief, pass them in `AvatarBinding.measurements_cm`; the solver uses them to size collision proxies.
6. **Solver budget**: do not set `substeps > 16` or `frames > 60` in exploratory iterations; increase only for final promotion-quality runs.

The system prompt shape (injected into `CompletionRequest.prompt` via the `LlmClient` trait):

```
You are a garment authoring agent operating the Handshake Tailor engine.
You emit structured GarmentSpec JSON and read typed SimulationReceipts.
You must not emit prose where a JSON field is expected.
Available tools: author_garment, simulate_garment, edit_garment,
                 promote_garment, get_garment, estimate_fabric_params (optional).
Tool call order: author_garment -> simulate_garment -> [edit_garment -> simulate_garment]* -> promote_garment.
Self-correction: after each simulate_garment call, read validation_findings.
Apply suggested_fix.field_path patches via edit_garment before re-simulating.
Stop if recommended_action=RequiresOperatorAction or iteration count > 5.
```

---

### [T-MODEL-FIRST-API.kernel-binding-summary] Kernel Primitive Binding Summary

| Model API surface | Kernel primitive | Location |
|---|---|---|
| `author_garment` MCP tool | `TailorModelAdapter.invoke()` + fast `ValidationRunner` | `tailor/model_adapter.rs` |
| `simulate_garment` MCP tool | `TailorSandboxAdapter.run()` + `SandboxRunV1` lifecycle | `tailor/solver_binding.rs` |
| `edit_garment` MCP tool | `CrdtUpdateRecordV1` via `kernel_crdt_updates` table | `tailor/crdt.rs` |
| `promote_garment` MCP tool | `PromotionGate.evaluate()` + `PromotionReceiptV1` | `kernel/kb003_promotion/gate.rs` |
| `get_garment` MCP tool | `Database.get_garment()` (new trait method) | `storage/postgres.rs` |
| `estimate_fabric_params` | `TailorSandboxAdapter` (diff-xpbd feature) | `tailor/solver_binding.rs` |
| `TailorGarmentDraftProposed` event | `NewKernelEvent::builder(...)` after `author_garment` | `tailor/storage_glue.rs` |
| `TailorSimRunCompleted` event | `NewKernelEvent::builder(...)` after solver completes | `tailor/storage_glue.rs` |
| `TailorGarmentPromoted` event | `NewKernelEvent::builder(...)` after `PromotionGate` accepts | `tailor/storage_glue.rs` |
| MCP input schema generation | `schemars::JsonSchema` derive on `GarmentSpec` et al. | `tailor-solver/src/spec.rs` |
| JSON instance validation | `jsonschema` crate (`mcp/schema.rs` pattern) | `tailor/fast_validate.rs` |
| LLM calls via `LlmClient` | `CompletionRequest` / `CompletionResponse` (HSK-TRAIT-004) | `llm/mod.rs` |

---

### [T-MODEL-FIRST-API.risks] Risks and Open Questions

**Risk 1: Spec ambiguity causes unstable loop.**
If a text description maps to many valid `GarmentSpec` shapes, the model may oscillate between alternatives across iterations. Mitigation: `ContextBundle` should carry `reference_spec_id` when editing an existing garment; the model is constrained to patch rather than re-author from scratch. The `ngl_description` field anchors the author intent across turns.

**Risk 2: Fast validation misses solver-level failures.**
A geometrically valid spec can still produce a solver failure (e.g. a garment with correct polygon vertices that collapse to zero area under simulation). Mitigation: add `min_panel_area_cm2` check to fast validation; reject panels below 1 cm² before the solver runs.

**Risk 3: DiffXPBD inverse path is too slow for interactive loop.**
Differentiable XPBD on 50,000+ particles can take minutes per optimisation iteration. Mitigation: `estimate_fabric_params` is behind a feature flag and should only be invoked when a reference image exists; default iterations = 50 but the model can request fewer for a rough estimate.

**Risk 4: Model-authored panels have incorrect vertex winding.**
Panels must be counter-clockwise for correct normal direction. LLMs generating vertex coordinates from text often produce clockwise or mixed winding. Mitigation: fast validation auto-corrects winding direction and reports a `WINDING_CORRECTED` info finding so the model knows the fix was applied.

**Risk 5: `promote_garment` requires operator consent but model may not handle `RequiresOperatorAction` gracefully.**
If operator consent is not configured, the promotion step blocks indefinitely. Mitigation: `SimulationReceipt` includes `recommended_action=RequiresOperatorAction` when the gate detects missing consent configuration; the model is instructed to stop and report this to the operator session rather than retrying.

**Risk 6: CRDT merge conflicts on concurrent panel edits produce geometrically invalid merged state.**
Two models editing overlapping panels concurrently can produce a merged panel set that does not form a closed garment. Mitigation: the `promote_bridge` in `kernel/crdt/promotion_bridge.rs` must run a post-merge seam closure check before committing the merged state; on failure it emits `TailorCrdtConflictDetected` and both editing sessions are notified.

**Open question: should `GarmentSpec` carry NGL as a first-class field or only in `ContextBundle`?**
NGL-Prompter shows that keeping the natural-language description alongside the numeric spec improves edit coherence. Embedding it in `GarmentSpec` as `natural_description: Option<String>` would make it part of the authority row, useful for future fine-tuning or audit. Decision deferred to T-GARMENT-AUTHORING.

**Open question: how to score drape quality without a reference image?**
The `drape_quality_score` in `SimulationReceipt` needs a model-free metric for the no-reference case. Candidate metrics: mean cloth-avatar penetration depth (lower = better fit), max vertex velocity at rest (lower = converged), seam gap after simulation (lower = closed). Combination of these as a weighted scalar is the likely path; weights require empirical calibration.

---

### [T-MODEL-FIRST-API.sources] Sources

- https://arxiv.org/html/2412.17811v1 — ChatGarment: VLM-to-GarmentCodeRC JSON pipeline, schema, multi-turn editing, token counts
- https://github.com/biansy000/ChatGarment — ChatGarment repo: inference scripts, editing API
- https://arxiv.org/abs/2602.20700 — NGL-Prompter: Natural Garment Language intermediate representation, training-free VLM query pipeline
- https://www.opentrain.ai/papers/ngl-prompter-training-free-sewing-pattern-estimation-from-a-single-image--arxiv-2602.20700/ — NGL-Prompter summary: NGL restructures GarmentCode for VLM legibility, deterministic mapping
- https://georgenakayama.github.io/AIpparel/ — AIpparel: multimodal LLM garment tokeniser, panel-level token scheme, simulation-ready output
- https://arxiv.org/abs/2412.03937 — AIpparel paper: novel sewing-pattern tokenisation, 120k garment dataset
- https://style3d.github.io/design2garmentcode/ — Design2GarmentCode: two-agent (DSL-GA + MMUA) program synthesis, dual validation loops, 100% simulation success
- https://openaccess.thecvf.com/content/CVPR2025/html/Zhou_Design2GarmentCode_Turning_Design_Concepts_to_Tangible_Garments_Through_Program_Synthesis_CVPR_2025_paper.html — Design2GarmentCode CVPR 2025
- https://arxiv.org/html/2601.02792 — Textile IR: seven-layer Verification Ladder, FunSearch-style proposal-verify-iterate, bidirectional IR schema
- https://arxiv.org/html/2301.01396 — DiffXPBD: differentiable XPBD for fabric parameter estimation, adjoint backward pass, 26M DOF demonstration
- https://dl.acm.org/doi/10.1145/3606923 — DiffXPBD ACM CGIT 2023 publication
- https://modelcontextprotocol.io/specification/2025-06-18/server/tools — MCP June 2025 spec: tool definition, inputSchema, outputSchema, structuredContent, error handling
- https://rup12.net/posts/write-your-mcps-in-rust/ — rmcp Rust MCP guide: #[tool] macro, schemars derive, structured results
- https://github.com/rust-mcp-stack/rust-mcp-schema — rust-mcp-schema: type-safe MCP schema in Rust
- https://docs.rs/schemars — schemars crate: JsonSchema derive, field descriptions via doc comments
- https://github.com/GREsau/schemars — schemars repo
- https://github.com/imaurer/awesome-llm-json — LLM JSON/structured output resource list
- https://pockit.tools/blog/llm-structured-output-complete-guide/ — LLM structured output 2026: XGrammar, provider support, production patterns
- https://www.nature.com/articles/s44387-025-00057-z — Self-correcting multi-agent LLM framework for physics simulation (MCP-SIM): plan-act-reflect-revise cycle
- https://chatgarment.github.io/ — ChatGarment project page
- https://github.com/maria-korosteleva/GarmentCode — GarmentCode: Panel/Edge/Component/Interface DSL, JSON schema reference
- https://arxiv.org/abs/2412.08603 — Design2GarmentCode arxiv
- https://forgecode.dev/blog/mcp-spec-updates/ — MCP June 2025 spec update: structured output, user elicitation
- https://www.sitepoint.com/model-context-protocol-mcp/ — MCP 2026 developer guide: 10k+ servers, 97M downloads
- D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/model_adapter.rs
- D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/llm/mod.rs
- D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/context_bundle.rs
- D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/sandbox/adapter.rs
- D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/mod.rs
- D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/mcp/gate.rs
- D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/mcp/schema.rs
