---
file_id: cloth-engine-kernel-integration
topic_id: T-KERNEL-INTEGRATION
title: "Handshake Kernel Integration (Sandbox, Promotion, EventLedger, CRDT, Model Lanes)"
status: draft
depends_on:
  - T-CODEBASE
summary: "How the Tailor creative module binds to every Handshake kernel primitive: garment/wardrobe assets as Postgres/EventLedger authority, collaborative pattern editing via CRDT, model-authored garments running in the sandbox and reaching authority only after validation and promotion, model lanes as the LLM-steerable authoring surface, and the full typed-event and typed-contract inventory."
sources: 28
updated_at: "2026-06-17"
---

## [T-KERNEL-INTEGRATION] Handshake Kernel Integration (Sandbox, Promotion, EventLedger, CRDT, Model Lanes)

### [T-KERNEL-INTEGRATION.overview] Integration Architecture Overview

Tailor is a **creative module** that attaches to five kernel primitives introduced in
`wtc-kernel-009`. It does not own a separate runtime, a separate storage backend, or a separate
event bus. Every durable authority write goes through PostgreSQL. Every mutation emits an
EventLedger receipt. Every model-authored garment proposal runs in the sandbox and crosses the
promotion gate before becoming an authority row. Every collaborative panel edit flows through the
CRDT layer. Every LLM call goes through the `LlmClient` trait (HSK-TRAIT-004).

The module pattern to follow is `src/atelier/` — a domain-owned `PgPool` reference, a dedicated
`event_family` constant block, domain-specific submodules, and a storage glue file parallel to
`storage/kb003_storage.rs`. Tailor adds its own domain alongside atelier rather than
inside it.

```
handshake_core/
  src/
    tailor/           <- new domain module (mirrors atelier/ pattern)
      mod.rs
      event_family.rs       <- tailor.* event-family string constants
      garment.rs            <- GarmentDraftV1, GarmentAssetV1, WardrobeV1
      material.rs           <- FabricPropertySetV1, MaterialLibraryEntryV1
      seam.rs               <- SeamDefinitionV1, GatherRatioV1
      panel.rs              <- PatternPanelV1, PanelEdgeV1
      solver_binding.rs     <- ClothSolverRequest/Response bridge to tailor-solver crate
      simulation.rs         <- SimulationRunV1, SimulationResultV1
      validation.rs         <- GarmentValidationDescriptor (wraps kb003 ValidationDescriptor)
      storage_glue.rs       <- Postgres row types + migration SQL
      api.rs                <- Axum Router: POST /tailor/garments, POST /tailor/garments/:id/simulate, ...
    api/
      tailor.rs       <- thin Tauri command + Axum route wiring
```

The standalone WGSL/wgpu solver crate (`handshake-cloth-solver`) is a **separate Cargo workspace
crate** with no `handshake_core` dependency. `tailor/solver_binding.rs` depends on it via
a trait boundary (`pub trait ClothSolver: Send + Sync`), keeping the GPU crate UI-agnostic.

---

### [T-KERNEL-INTEGRATION.postgres-authority] PostgreSQL / EventLedger Authority Layer

#### Codebase anchor

The sole authority backend is `PostgresDatabase` backed by `sqlx::PgPool` (enforced by
`no_sqlite_tripwire::guard_authority_write()` in `kernel/sandbox/no_sqlite_tripwire.rs`).
The write pattern for every domain mutation is the EventLedger INSERT seen in
`storage/postgres.rs` around line 3454: `WITH inserted AS (INSERT INTO kernel_event_ledger ...
ON CONFLICT (idempotency_key) DO NOTHING RETURNING ...)`.

`WriteContext` (required arg for all `Database` trait write methods) carries the active
`KernelActor` for the audit trail. The Tailor module always supplies a `WriteContext` populated
from the appropriate actor (see [T-KERNEL-INTEGRATION.actors]).

#### Garment authority schema

```sql
-- Migration: tailor_garments (authority row per promoted garment asset)
CREATE TABLE IF NOT EXISTS tailor_garments (
    garment_id          TEXT PRIMARY KEY,           -- "GAR-{uuid_v7}"
    workspace_id        TEXT NOT NULL,
    wardrobe_id         TEXT,                        -- optional wardrobe grouping
    draft_json          JSONB NOT NULL,              -- GarmentDraftV1 canonical JSON
    material_json       JSONB NOT NULL,              -- FabricPropertySetV1
    status              TEXT NOT NULL DEFAULT 'draft',  -- draft | simulated | promoted | archived
    promoted_at_utc     TIMESTAMPTZ,
    promotion_receipt_id TEXT,                       -- FK to kb003_promotion_receipts
    event_ledger_event_id TEXT NOT NULL,             -- FK to kernel_event_ledger.event_id
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_garments_workspace ON tailor_garments (workspace_id);
CREATE INDEX IF NOT EXISTS ix_tailor_garments_status    ON tailor_garments (status);

-- Migration: tailor_simulation_runs (one row per XPBD solver sandbox run)
CREATE TABLE IF NOT EXISTS tailor_simulation_runs (
    sim_run_id          TEXT PRIMARY KEY,            -- "CSIM-{uuid_v7}"
    garment_id          TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    sandbox_run_id      TEXT NOT NULL REFERENCES kb003_sandbox_runs (run_id),
    solver_version      TEXT NOT NULL,
    substeps            INTEGER NOT NULL,
    iterations          INTEGER NOT NULL,
    sim_artifact_ref    TEXT,                        -- path to simulated mesh bundle
    status              TEXT NOT NULL,
    requested_at_utc    TIMESTAMPTZ NOT NULL,
    finished_at_utc     TIMESTAMPTZ,
    event_ledger_event_id TEXT NOT NULL
);

-- Migration: tailor_material_library (named fabric property presets)
CREATE TABLE IF NOT EXISTS tailor_material_library (
    material_id         TEXT PRIMARY KEY,            -- "MAT-{uuid_v7}"
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,
    fabric_class        TEXT NOT NULL,               -- "cotton" | "denim" | "silk" | ...
    property_json       JSONB NOT NULL,              -- FabricPropertySetV1
    source              TEXT NOT NULL DEFAULT 'operator',  -- operator | model | imported
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Migration: tailor_wardrobe (grouping of promoted garments)
CREATE TABLE IF NOT EXISTS tailor_wardrobe (
    wardrobe_id         TEXT PRIMARY KEY,            -- "WRD-{uuid_v7}"
    workspace_id        TEXT NOT NULL,
    name                TEXT NOT NULL,
    description         TEXT,
    event_ledger_event_id TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

#### EventLedger write call (Tailor domain)

Every Tailor domain mutation emits via `NewKernelEvent::builder(...)`, following the pattern
confirmed in `kernel/mod.rs` line 512 and `storage/postgres.rs` line 3454:

```rust
// Example: recording a new garment draft proposal from a model actor
let event = NewKernelEvent::builder(
    task_run_id,
    session_run_id,
    KernelEventType::TailorGarmentDraftProposed,   // new variant (see event taxonomy below)
    KernelActor::ModelAdapter("tailor-garment-adapter-v1".into()),
)
.aggregate("tailor_garment", &garment_id)
.idempotency_key(format!("CGDP-{garment_id}-{session_run_id}"))
.payload(serde_json::json!({
    "garment_id": garment_id,
    "workspace_id": workspace_id,
    "draft_schema": "hsk.cloth.garment_draft@1",
    "panel_count": panel_count,
    "seam_count": seam_count,
    "material_id": material_id,
}))
.source_component("tailor::garment")
.build()?;

db.insert_kernel_event(write_ctx, event).await?;
```

---

### [T-KERNEL-INTEGRATION.event-taxonomy] Typed Event Taxonomy (KernelEventType Additions)

New variants are added to the `KernelEventType` enum in `kernel/mod.rs` and registered in
`required_first_slice_events()`. The existing SCREAMING_SNAKE_CASE pattern and string IDs are
followed exactly.

```rust
// Additions to KernelEventType enum:

// Garment draft lifecycle (model-authored or operator-authored)
TailorGarmentDraftProposed,        // LLM or operator submits a new garment JSON spec
TailorGarmentDraftUpdated,         // CRDT-mediated panel/seam edit promoted to draft

// Simulation lifecycle (XPBD solver sandbox run)
TailorSimRunRequested,      // solver sandbox run requested for a garment draft
TailorSimRunStarted,        // TailorSandboxAdapter.run() started
TailorSimRunCompleted,      // solver returned mesh + UV artifact bundle
TailorSimRunRejected,       // solver sandbox denied or mesh validation failed

// Validation
TailorGarmentValidationRecorded,   // ValidationRunner produced GarmentValidationReport

// Promotion (sandbox -> authority)
TailorGarmentPromoted,             // PromotionGate accepted; authority row written
TailorGarmentPromotionRejected,    // PromotionGate rejected with typed reason

// CRDT collaborative editing
TailorPanelCrdtUpdateRecorded,     // panel geometry CRDT update logged
TailorPanelCrdtSnapshotRecorded,   // panel snapshot checkpoint
TailorPanelAiEditProposalRecorded, // model proposes a panel geometry edit
TailorPanelAiEditProposalDecided,  // operator/validator approves or rejects proposal

// Material library
TailorMaterialRecorded,            // new material preset written to authority
TailorMaterialUpdated,             // material preset updated

// Wardrobe
TailorWardrobeCreated,
TailorWardrobeGarmentAdded,
TailorWardrobeGarmentRemoved,
```

EventLedger `event_family` constants (matching `atelier::event_family` convention):

```rust
// src/tailor/event_family.rs
pub const TAILOR_GARMENT_DRAFT:       &str = "tailor.garment.draft";
pub const TAILOR_SIMULATION:          &str = "tailor.simulation";
pub const TAILOR_GARMENT_VALIDATION:  &str = "tailor.garment.validation";
pub const TAILOR_GARMENT_PROMOTION:   &str = "tailor.garment.promotion";
pub const TAILOR_PANEL_CRDT:          &str = "tailor.panel.crdt";
pub const TAILOR_MATERIAL:            &str = "tailor.material";
pub const TAILOR_WARDROBE:            &str = "tailor.wardrobe";
```

Schema ID constants (following `hsk.kernel.<record>@<version>` convention from
`kernel/kb003_schemas.rs`):

```rust
pub const SCHEMA_CLOTH_GARMENT_DRAFT_V1:   &str = "hsk.cloth.garment_draft@1";
pub const SCHEMA_CLOTH_MATERIAL_V1:        &str = "hsk.cloth.material@1";
pub const SCHEMA_CLOTH_SIM_RUN_V1:         &str = "hsk.cloth.simulation_run@1";
pub const SCHEMA_CLOTH_PANEL_V1:           &str = "hsk.cloth.pattern_panel@1";
pub const SCHEMA_CLOTH_SEAM_V1:            &str = "hsk.cloth.seam_definition@1";
```

---

### [T-KERNEL-INTEGRATION.actors] KernelActor Roles in the Tailor Pipeline

The `KernelActor` enum (confirmed in `kernel/mod.rs` line 456) maps to Tailor pipeline stages as
follows:

| Stage | Actor variant | Notes |
|---|---|---|
| LLM garment authoring | `ModelAdapter("tailor-garment-adapter-v1")` | Issues `TailorGarmentDraftProposed` |
| Operator panel edits | `Operator(user_id)` | Direct CRDT push via yjs_bridge |
| Model panel edit proposals | `ModelAdapter(adapter_id)` | `AiEditProposalRecorded` path |
| XPBD solver sandbox run | `System("cloth-solver-v1")` | Inside `TailorSandboxAdapter::run()` |
| Validation runner | `ValidationRunner("tailor-garment-validator-v1")` | Runs `GarmentValidationDescriptor` |
| Promotion gate | `PromotionGate("tailor-promotion-gate-v1")` | Issues `PromotionReceiptV1` |
| Material library writes | `Operator(user_id)` or `ModelAdapter(...)` | Depends on source field |

---

### [T-KERNEL-INTEGRATION.sandbox] Sandbox Integration: TailorSandboxAdapter

#### Architecture

The XPBD solver runs inside the KB003 sandbox lifecycle (`REQUESTED -> STARTED -> COMPLETED |
REJECTED`). The `SandboxAdapter` trait (`kernel/sandbox/adapter.rs` line 85) is implemented by
`TailorSandboxAdapter`:

```rust
// src/tailor/solver_binding.rs
use crate::kernel::sandbox::adapter::{
    AdapterKind, AdapterIsolationTier, AdapterRunOutcome, AdapterError, SandboxAdapter,
};
use crate::kernel::sandbox::denial::SandboxDenialRecordV1;
use crate::kernel::sandbox::policy::{SandboxCapability, SandboxPolicyV1};
use crate::kernel::sandbox::run::SandboxRunV1;
use crate::kernel::sandbox::workspace::SandboxWorkspaceV1;

pub struct TailorSandboxAdapter {
    solver: Arc<dyn ClothSolver>,      // trait object for tailor-solver crate
    solver_version: String,
}

impl SandboxAdapter for TailorSandboxAdapter {
    fn kind(&self) -> AdapterKind {
        AdapterKind::process_tier(
            "cloth-solver-process-v1",
            "XPBD Cloth Solver (process tier)",
        )
    }

    fn run(
        &self,
        run: &SandboxRunV1,
        workspace: &SandboxWorkspaceV1,
        policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError> {
        // 1. pre_check: policy must allow LocalFilesystemRead (mesh write) + ComputeGpu
        self.pre_check(run, policy, &[
            SandboxCapability::LocalFilesystemRead,
            SandboxCapability::LocalFilesystemWrite,
        ])?;
        // 2. Load GarmentDraftV1 from workspace artifact_refs
        // 3. Issue ClothSolverRequest to tailor-solver via trait boundary
        // 4. Write simulated mesh + UV artifact bundle to workspace scratch path
        // 5. Return Completed { artifact_refs: vec![mesh_ref, uv_ref, material_ref] }
        todo!("TailorSandboxAdapter::run implementation")
    }
}

/// Trait boundary separating handshake_core from the wgpu/WGSL solver crate.
/// tailor-solver implements this; handshake_core depends on the trait only.
pub trait ClothSolver: Send + Sync {
    fn solver_version(&self) -> &str;
    fn simulate(&self, request: ClothSolverRequest) -> Result<ClothSolverResult, ClothSolverError>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClothSolverRequest {
    pub schema_version: String,         // "hsk.cloth.solver_request@1"
    pub garment_draft: serde_json::Value, // GarmentDraftV1 JSON
    pub avatar_collision_proxy: serde_json::Value, // capsule/trimesh proxy JSON
    pub substeps: u32,
    pub iterations: u32,
    pub gravity: [f32; 3],
    pub frame_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClothSolverResult {
    pub schema_version: String,         // "hsk.cloth.solver_result@1"
    pub vertex_buffer_ref: String,      // path to GPU readback vertex buffer (f32 XYZ)
    pub uv_buffer_ref: String,          // path to UV coordinate buffer
    pub index_buffer_ref: String,       // path to triangle index buffer
    pub material_params_ref: String,    // path to final material JSON
    pub frame_count: u32,
    pub final_energy: f64,              // convergence diagnostic
}
```

#### Sandbox policy for the cloth solver

The cloth solver needs `LocalFilesystemRead` (load draft) and `LocalFilesystemWrite` (write mesh
output). It does **not** need network access. The policy follows `policy_scoped_local.rs` — the
`policy_default_deny` base with an explicit `LocalFilesystemRead` + `LocalFilesystemWrite` grant
scoped to the sandbox workspace scratch path.

The `AdapterIsolationTier::Process` default is used for day-one. HardIsolation (container) is an
opt-in upgrade for production deployment where the wgpu compute driver must be available inside
the container — this is noted as a risk (see [T-KERNEL-INTEGRATION.risks]).

#### Artifact bundle

Simulated mesh artifacts use `Kb003ArtifactClass` bundle semantics. Tailor adds its own artifact class entry:

```rust
// Additions to Kb003ArtifactClass (or a parallel ClothArtifactClass enum):
ClothSimulatedMeshBundle,   // content_type: "application/octet-stream" (multi-file bundle)
                             // hash_policy: BinarySha256
                             // retention_root: "handshake-product/cloth/sim-meshes"
```

---

### [T-KERNEL-INTEGRATION.validation] Validation Gate: GarmentValidationDescriptor

The `ValidationDescriptor` pattern (`kernel/validation/descriptor.rs`) is extended for garment
mesh validation. A `GarmentValidationDescriptor` wraps the KB003 descriptor contract:

```rust
// Validation checks registered in GarmentValidationDescriptor:
// BLOCKING checks (any failure prevents promotion):
//   - mesh_not_empty:          simulated vertex buffer is non-empty
//   - no_degenerate_triangles: no zero-area triangles in output mesh
//   - seams_closed:            all seam constraint pairs have <= 1mm separation after sim
//   - no_interpenetration:     cloth mesh does not intersect avatar collision proxy
//   - uv_coverage:             UV islands cover >= 95% of mesh surface (pattern accuracy)
//   - material_params_valid:   FabricPropertySetV1 fields are in physical range
//
// ADVISORY checks (failure visible but does not block unless strict mode):
//   - low_final_energy:        final_energy < threshold (solver converged)
//   - panel_count_matches_draft: simulated panels == draft panel count
//   - garment_code_roundtrip:  draft JSON round-trips to GarmentCode JSON without loss
```

The `ValidationReport::aggregate_blocks_promotion()` method from `kernel/validation/report.rs`
drives the promotion gate decision — Tailor reuses this path unchanged.

---

### [T-KERNEL-INTEGRATION.promotion] Promotion Gate Binding

The KB003 `PromotionGate::evaluate()` (`kernel/kb003_promotion/gate.rs` line 1) is called after
the sandbox run completes and the validation runner issues a `GarmentValidationReport`. The full
input bundle (`PromotionGateInputs`) is constructed:

```rust
let inputs = PromotionGateInputs {
    sandbox_run:               &sim_sandbox_run,      // SandboxRunV1 from tailor_simulation_runs
    validation_report:         &garment_val_report,   // ValidationReport from validation run
    validation_run_id:         val_run_id.clone(),
    artifact_bundle:           &artifact_bundle,      // vertex + UV + material refs
    approval:                  operator_approval,     // OperatorApprovalEvidence
    idempotency_key:           format!("CPROM-{garment_id}-{val_run_id}"),
    required_artifact_refs:    vec![mesh_ref, uv_ref, material_ref],
    latest_known_run_id:       Some(sim_run_id.clone()),
    treat_advisory_as_blocking: false,
};

let receipt: PromotionReceiptV1 = promotion_gate.evaluate(
    &inputs,
    &storage,       // Arc<dyn Kb003Storage>
).await?;
```

On `PromotionOutcome::Accepted`:
1. `tailor_garments.status` is set to `promoted`.
2. `tailor_garments.promoted_at_utc` and `promotion_receipt_id` are written.
3. `TailorGarmentPromoted` EventLedger event is emitted with the `receipt_id` in payload.
4. The garment asset is now readable as an authority row.

On `PromotionOutcome::Rejected`:
1. `tailor_garments.status` remains `simulated`.
2. `TailorGarmentPromotionRejected` is emitted with `PromotionRejectionReason` in payload.
3. The `PromotionReceiptV1.storage_error_detail` is available for operator inspection.
4. The operator may trigger a new simulation run (retry path) or archive the draft.

The idempotency key format `CPROM-{garment_id}-{val_run_id}` ensures that retrying a promotion
for the same garment + validation run returns the original receipt rather than creating a
duplicate — exactly matching the behavior proved in the KB003 tests.

---

### [T-KERNEL-INTEGRATION.crdt] CRDT Layer: Collaborative Pattern Editing

#### How garment panels map to the CRDT document model

The existing CRDT layer (`kernel/crdt/`) persists `CrdtUpdateRecordV1` rows in
`kernel_crdt_updates` (migration 0020). The mapping for cloth pattern editing:

| Cloth concept | CRDT mapping |
|---|---|
| Garment draft | `document_id = garment_id` |
| Per-garment CRDT document | `crdt_document_id = "CRDT-GAR-{garment_id}"` |
| Panel geometry (vertices, bezier handles) | CRDT map inside the document |
| Seam definitions (edge pairs + ratios) | CRDT list inside the document |
| Material property overrides per panel | CRDT map entries |

The **yjs_bridge** path (`kernel/crdt/yjs_bridge.rs`) handles frontend collaborative editing:
`push_yjs_update()` validates the `YjsUpdateEnvelopeV1`, enforces linear draft ordering, appends
the EventLedger receipt (`KnowledgeCrdtUpdateRecorded`), and writes the update row. Tailor reuses this infrastructure unchanged — the update bytes are opaque to the backend; Yjs
merging happens in the Tauri frontend.

For server-side panel geometry mutations (e.g., the XPBD solver proposes UV-adjusted control
points after simulation), the **ai_edit_proposal** path (`kernel/crdt/ai_edit_proposal.rs`) is
used:

```rust
// Model proposing a panel vertex adjustment after simulation feedback
let proposal = AiEditProposalRequestV1 {
    workspace_id:          workspace_id.clone(),
    document_id:           garment_id.clone(),
    crdt_document_id:      format!("CRDT-GAR-{garment_id}"),
    base_update_seq:       head_seq,
    base_state_vector:     head_state_vector.to_string(),
    proposed_diff:         serde_json::json!({
        "op": "panel_vertex_adjust",
        "panel_id": panel_id,
        "vertex_idx": vertex_idx,
        "delta_uv": [du, dv],
        "rationale": "UV unfurl requires 2.3mm adjustment at panel seam boundary",
    }),
    source_span_citations: vec![format!("sim-run:{sim_run_id}")],
    actor:                 KnowledgeActorIdV1::model_adapter("tailor-garment-adapter-v1"),
    session_id:            session_run_id.clone(),
    correlation_id:        garment_id.clone(),
    lease_id:              Some(model_lane_lease_id.clone()),
};
```

The proposal state machine (`proposed -> approved | rejected`) follows the existing
`kernel/crdt/ai_edit_proposal.rs` flow. Models cannot self-approve — operator or validation
runner approval is required before the diff is applied as a CRDT update.

#### State vector and conflict resolution

`KnowledgeStateVectorV1` (`kernel/crdt/state_vector.rs`) carries per-site version vectors with
`hsk-sv1:` prefix. Causality verdicts (`Equal | Dominates | DominatedBy | Concurrent`) drive
concurrent-save decisions. For panel geometry, `Concurrent` edits (two operators editing different
panels simultaneously) are resolved by last-write-wins on distinct panels, since panels are
independent CRDT map subtrees. Concurrent edits to the **same panel vertex** surface as a
`KnowledgeCrdtConflictDetected` event requiring operator decision — the same conflict UI path
already implemented for knowledge documents.

---

### [T-KERNEL-INTEGRATION.model-lanes] Model Lanes: LLM-Steerable Garment Authoring

#### LlmClient integration (HSK-TRAIT-004)

All LLM calls in Tailor route through `LlmClient::completion()` (`llm/mod.rs` line 36).
The Tailor module holds a reference to `Arc<dyn LlmClient>` injected from `AppState.llm_client`.
No Tailor-specific LLM client implementation is needed.

Garment authoring prompts follow the ChatGarment / AIpparel pattern (CVPR 2025): the LLM
receives a structured context describing the garment type, body measurements, design intent, and
any reference image captions, and outputs a JSON object conforming to `GarmentDraftV1`.

```rust
// Tailor module garment authoring via LlmClient
let request = CompletionRequest {
    trace_id:    Uuid::now_v7(),
    prompt:      tailor_authoring_prompt(&garment_context),
    model_id:    preferred_model_id,
    max_tokens:  2048,
    temperature: Some(0.2),    // low temperature for structured JSON output
    system:      Some(GARMENT_AUTHORING_SYSTEM_PROMPT.into()),
    json_schema: Some(GARMENT_DRAFT_JSON_SCHEMA.into()),  // constrained decoding
    ..Default::default()
};
let response = llm_client.completion(request).await?;
let garment_draft: GarmentDraftV1 = serde_json::from_str(&response.text)?;
```

The `json_schema` field requests constrained structured output from the model, matching the
NGL-Prompter approach (arxiv 2602.20700, Feb 2026) of mapping VLM outputs to deterministic
GarmentCode-compatible JSON.

#### ModelAdapter integration

The `ModelAdapter` trait (`kernel/model_adapter.rs` line 122) is implemented by
`TailorModelAdapter` for the full garment authoring flow that includes tool use:

```rust
pub struct TailorModelAdapter {
    adapter_id: String,
    llm_client: Arc<dyn LlmClient>,
    storage:    Arc<dyn Database>,
    pg_pool:    sqlx::PgPool,
}

#[async_trait]
impl ModelAdapter for TailorModelAdapter {
    fn adapter_id(&self) -> &str { &self.adapter_id }

    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput> {
        // context_bundle carries:
        //   - garment_context: body measurements, style intent, reference image captions
        //   - workspace_id, task_run_id, session_run_id
        //   - material_library: available FabricPropertySetV1 presets
        //   - constraint_description: free-text design constraints
        let ctx = &request.context_bundle;

        // 1. Call LlmClient to generate GarmentDraftV1 JSON
        // 2. Parse and validate GarmentDraftV1 (schema, panel count, seam closure)
        // 3. Return artifact_payload = garment_draft JSON; artifact_proposal with hash
        let garment_draft = self.generate_garment_draft(ctx).await?;
        let payload = serde_json::to_value(&garment_draft)?;
        let content_hash = sha256_hex(&canonical_json_bytes(&payload));

        Ok(ModelAdapterOutput {
            adapter_id:          self.adapter_id.clone(),
            context_bundle_id:   ctx.bundle_id.clone(),
            response_text:       format!("Garment draft generated: {}", garment_draft.garment_id),
            response_event_type: KernelEventType::TailorGarmentDraftProposed,
            tool_request:        KernelToolRequest { /* no tool request for pure draft gen */ .. },
            artifact_proposal:   ArtifactProposalDraft {
                artifact_proposal_id: format!("CGAP-{}", Uuid::now_v7()),
                event_type:           KernelEventType::TailorGarmentDraftProposed,
                artifact_kind:        "tailor_garment_draft".into(),
                content_hash:         content_hash.clone(),
            },
            artifact_payload: payload,
            output_hash:      content_hash,
        })
    }
}
```

#### GarmentDraftV1 — the canonical LLM output schema

The authority schema for model-authored garments bridges ChatGarment (CVPR 2025),
GarmentCode (ETH Zurich, `github.com/maria-korosteleva/GarmentCode`), and the Handshake
typed-JSON contract convention:

```json
{
  "schema_version": "hsk.cloth.garment_draft@1",
  "garment_id": "GAR-...",
  "workspace_id": "WS-...",
  "garment_type": "t-shirt",
  "panels": [
    {
      "panel_id": "P-front",
      "vertices": [[0.0, 0.0], [0.3, 0.0], [0.3, 0.5], [0.0, 0.5]],
      "edges": [
        {"edge_id": "E-front-bottom", "from_vertex": 0, "to_vertex": 1, "curve_type": "straight"},
        {"edge_id": "E-front-side-r", "from_vertex": 1, "to_vertex": 2, "curve_type": "bezier",
         "control_points": [[0.32, 0.15], [0.31, 0.35]]}
      ],
      "material_id": "MAT-cotton-medium",
      "grain_direction_deg": 0.0
    }
  ],
  "seams": [
    {
      "seam_id": "SEM-front-back-shoulder",
      "from_edge_id": "E-front-shoulder",
      "to_edge_id": "E-back-shoulder",
      "ratio": 1.0,
      "stitch_type": "normal"
    },
    {
      "seam_id": "SEM-gather-waist",
      "from_edge_id": "E-yoke-bottom",
      "to_edge_id": "E-body-top",
      "ratio": 1.5,
      "stitch_type": "gather"
    }
  ],
  "avatar_measurements": {
    "height_cm": 165.0,
    "bust_cm": 85.0,
    "waist_cm": 68.0,
    "hip_cm": 92.0
  }
}
```

The `ratio` field in seams (`SeamDefinitionV1`) carries the MD gather ratio (1:N) — the sewing
constraint maps directly to an XPBD rest-length multiplier in the solver.

#### Model lane for material parameter estimation

Image2Garment (arxiv 2601.09658, Jan 2026, Stanford/Google) demonstrates that a VLM fine-tuned
on fabric images can predict physical fabric parameters (stretch weft/warp, bending stiffness,
density) from a single image. Tailor's material authoring path implements this:

1. Operator uploads a fabric swatch image.
2. `TailorModelAdapter` (a second `ModelAdapter` impl) sends the image + the
   `FabricPropertySetV1` JSON schema as constrained output format.
3. The LLM returns a `FabricPropertySetV1` estimate.
4. The estimate runs through a material validation check (physical range enforcement).
5. The validated material is promoted to `tailor_material_library` as a new authority row.

---

### [T-KERNEL-INTEGRATION.api] Axum API Routes

Following the `src/api/<domain>.rs` pattern (confirmed from `src/api/` directory listing):

```rust
// src/api/tailor.rs
use axum::{Router, routing::{get, post}, extract::{State, Path}, Json};
use crate::lib::AppState;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/tailor/garments",               post(create_garment_draft))
        .route("/tailor/garments/:id",           get(get_garment))
        .route("/tailor/garments/:id/simulate",  post(trigger_simulation))
        .route("/tailor/garments/:id/promote",   post(promote_garment))
        .route("/tailor/garments/:id/crdt",      get(get_crdt_state))
        .route("/tailor/garments/:id/crdt/push", post(push_crdt_update))
        .route("/tailor/materials",              get(list_materials).post(create_material))
        .route("/tailor/materials/:id",          get(get_material))
        .route("/tailor/wardrobes",              get(list_wardrobes).post(create_wardrobe))
        .route("/tailor/wardrobes/:id/garments", post(add_garment_to_wardrobe))
        .with_state(state)
}
```

Tauri commands in `app/src-tauri/src/commands/` expose these endpoints to the frontend:

```rust
#[tauri::command]
async fn tailor_simulate(state: tauri::State<'_, AppState>, garment_id: String) -> Result<...> { ... }
#[tauri::command]
async fn tailor_get_garment(state: tauri::State<'_, AppState>, garment_id: String) -> Result<...> { ... }
#[tauri::command]
async fn tailor_promote_garment(state: tauri::State<'_, AppState>, garment_id: String, ...) -> Result<...> { ... }
```

---

### [T-KERNEL-INTEGRATION.full-lifecycle] Full Model-Authored Garment Lifecycle

```
Operator describes garment intent
        |
        v
TailorModelAdapter.invoke()
  -> LlmClient.completion() [structured JSON output constrained by GarmentDraftV1 schema]
  -> GarmentDraftV1 parsed + validated (schema check)
  -> INSERT tailor_garments (status=draft)
  -> NewKernelEvent TailorGarmentDraftProposed emitted
        |
        v
[Optional CRDT collaborative editing]
  -> push_yjs_update() / ai_edit_proposal flow
  -> KnowledgeCrdtUpdateRecorded / TailorPanelCrdtUpdateRecorded events
        |
        v
POST /tailor/garments/:id/simulate
  -> SandboxRunV1 created (REQUESTED)
  -> TailorSimRunRequested EventLedger event
  -> TailorSandboxAdapter.run() called (STARTED)
     -> ClothSolverRequest -> tailor-solver crate
     -> XPBD solver: WGSL compute shaders on wgpu (Vulkan/DX12/Metal)
     -> Mesh + UV artifact bundle written to sandbox workspace
  -> SandboxRunStatus::Completed
  -> TailorSimRunCompleted event; artifact_refs in sandbox run row
        |
        v
ValidationRunner executes GarmentValidationDescriptor
  -> Blocking checks: mesh_not_empty, seams_closed, no_interpenetration, uv_coverage
  -> Advisory checks: low_final_energy, panel_count_matches_draft
  -> ValidationReport produced; TailorGarmentValidationRecorded event
        |
     [blocking check failed?]
     yes -> TailorSimRunRejected; operator sees blocking reason; loop back
     no  ->
        |
        v
PromotionGate.evaluate(PromotionGateInputs { sandbox_run, validation_report, ... })
  -> PromotionDecisionV1 produced
  -> [Rejected?] -> TailorGarmentPromotionRejected event; rejection reason surfaced
  -> [Accepted?] ->
       tailor_garments.status = 'promoted'
       tailor_garments.promotion_receipt_id = receipt.receipt_id
       TailorGarmentPromoted EventLedger event
       Garment is now an authority row readable by all sessions
```

---

### [T-KERNEL-INTEGRATION.no-sqlite-tripwire] PostgreSQL Enforcement

The `no_sqlite_tripwire::guard_authority_write()` function (`kernel/sandbox/no_sqlite_tripwire.rs`)
must be called at the top of every Tailor domain authority write, identical to its usage in
`storage/kb003_storage.rs`:

```rust
// At the top of each Tailor storage glue write function:
guard_authority_write(AuthorityMode::PostgresPrimary)?;
```

This ensures that if the storage layer is misconfigured (e.g., in a test environment with a
non-Postgres backend), Tailor authority writes fail fast rather than silently falling back to an
unsupported backend.

---

### [T-KERNEL-INTEGRATION.portability] Disk-Agnostic Portability

All Tailor paths follow the `topology.yaml` portability contract:

- Garment authority rows live in PostgreSQL — portable by definition (connection string in
  environment variable, not hardcoded).
- Simulated mesh artifact bundles are written to the external artifact root
  `../Handshake_Artifacts/handshake-product/cloth/sim-meshes/` (relative to the repo root),
  following the `Kb003ArtifactMetadata.retention_root` convention.
- No absolute paths are hardcoded in the Tailor module code. Paths are resolved via
  `AppState.artifact_root` or equivalent topology-level config.
- The `tailor-solver` standalone crate has no hardcoded paths; all I/O goes through the
  `ClothSolverRequest` / `ClothSolverResult` API which uses `String` refs for file paths
  supplied by the sandbox workspace materializer.

---

### [T-KERNEL-INTEGRATION.marvelous-designer-mapping] Marvelous Designer Feature Map to Kernel Primitives

| MD Feature | Kernel primitive binding |
|---|---|
| 2D pattern panel creation | `GarmentDraftV1.panels[]` in `tailor_garments` Postgres row; CRDT-tracked |
| Sewing seams (1:N gather) | `GarmentDraftV1.seams[].ratio` field; XPBD rest-length constraint in solver |
| Fabric physical properties | `FabricPropertySetV1` in `tailor_material_library`; authority row per material |
| Simulation run | `TailorSandboxAdapter` + `kb003_sandbox_runs` table |
| Simulation result / mesh | `Kb003ArtifactBundleV1` in sandbox `artifact_refs`; promoted via `PromotionGate` |
| UV islands from pattern | Solver outputs UV buffer; `uv_coverage` validation check enforces accuracy |
| Collaborative editing | CRDT `yjs_bridge` + `ai_edit_proposal`; `kernel_crdt_updates` table |
| LLM-steerable authoring | `TailorModelAdapter` + `LlmClient.completion()` structured JSON |
| Material parameter from image | `TailorModelAdapter` + `LlmClient.completion()` (Image2Garment approach) |
| Garment library / presets | `tailor_wardrobe` + `tailor_material_library` authority tables |
| Validation gate | `GarmentValidationDescriptor`; KB003 `ValidationRunner` + `ValidationReport` |
| Operator approval / promotion | `OperatorApprovalEvidence` + `PromotionGate.evaluate()` |

---

### [T-KERNEL-INTEGRATION.risks] Risks and Open Questions

**RISK-1: wgpu GPU driver availability inside sandbox process tier.**
The `Process` tier sandbox (default) inherits the host GPU driver. On Windows this works for
Vulkan/DX12. On headless CI or containers, wgpu may fall back to `dx12` or fail to initialize.
Mitigation: the `ClothSolver` trait must expose a `cpu_fallback` feature flag; the sandbox policy
should record solver backend used (GPU/CPU) in the artifact manifest.

**RISK-2: CRDT conflict resolution for panel geometry.**
Concurrent edits to the **same panel vertex** by two operators simultaneously produce
`KnowledgeCrdtConflictDetected`. The existing conflict UI (`kernel/crdt/conflict_ui.rs`) surfaces
this, but panel geometry conflicts require a spatial merge hint (which vertex, which coordinate).
The `proposed_diff` in `AiEditProposalRequestV1` carries per-vertex metadata, but the conflict
resolution UI needs a domain-specific geometry diff renderer not yet designed.

**RISK-3: Structured JSON output reliability from base LLMs.**
ChatGarment and AIpparel require fine-tuned models for reliable GarmentCode JSON output.
Base models (Ollama local, Claude via BYOK) may hallucinate invalid panel geometry or violate
physical constraints. Mitigation: the `GarmentValidationDescriptor` blocking checks catch
invalid drafts before simulation is invoked; the `schema_check` validation step rejects
structurally invalid JSON before the sandbox run is created.

**RISK-4: Promotion gate requires operator approval evidence.**
`OperatorApprovalEvidence.looks_fixture()` (`kernel/kb003_promotion/gate.rs` line 99) rejects
fixture-looking evidence. The Tailor UI must surface a real approval workflow to the operator
rather than auto-approving. For automated batch garment generation, the approval evidence must
come from a real operator review receipt — automated self-approval is architecturally blocked.

**RISK-5: tailor-solver crate compilation time on first build.**
A new Cargo workspace crate with wgpu (which pulls in Naga, SPIR-V tools, and platform SDK
bindings) will significantly increase cold build time. Mitigation: the crate should be feature-
gated (`#[cfg(feature = "cloth-solver")]`) so CI builds that do not test GPU code can skip it.

**OPEN-1: GarmentDraftV1 round-trip to GarmentCode JSON.**
The `garment_code_roundtrip` advisory validation check requires a GarmentCode JSON serializer.
GarmentCode is Python-based (`github.com/maria-korosteleva/GarmentCode`); Tailor needs a Rust serializer that produces GarmentCode-compatible output. This is not a
blocking requirement for MVP but enables ChatGarment-style interop.

**OPEN-2: Keyframeable fabric properties.**
MD 2025.2 introduced keyframeable fabric properties (shrinkage warp/weft, pressure, solidify)
for animated cloth effects. The Handshake `GarmentDraftV1` schema does not yet carry a timeline.
A future `GarmentAnimationDraftV1` extension would add a `keyframes[]` array to the draft JSON,
with per-frame `FabricPropertySetV1` overrides uploaded as per-substep parameter buffers in the
XPBD solver.

**OPEN-3: Operator approval UI surface in Tauri.**
The promotion gate requires `OperatorApprovalEvidence` with a non-fixture `review_receipt_id`.
The Tauri frontend needs a garment review panel that issues real review receipt IDs — a UI
design requirement not covered in this research document.

---

### [T-KERNEL-INTEGRATION.sources] Sources

**Handshake codebase (wtc-kernel-009, read directly):**
- `src/backend/handshake_core/src/kernel/mod.rs` — `KernelEventType`, `KernelActor`, `NewKernelEvent::builder()`
- `src/backend/handshake_core/src/kernel/model_adapter.rs` — `ModelAdapter` trait, `ModelAdapterOutput`
- `src/backend/handshake_core/src/kernel/sandbox/adapter.rs` — `SandboxAdapter` trait, `AdapterKind`, `AdapterRunOutcome`
- `src/backend/handshake_core/src/kernel/sandbox/run.rs` — `SandboxRunV1`, `SandboxRunStatus` lifecycle
- `src/backend/handshake_core/src/kernel/kb003_promotion/gate.rs` — `PromotionGate::evaluate()`, `PromotionGateInputs`, `OperatorApprovalEvidence`
- `src/backend/handshake_core/src/kernel/kb003_promotion/receipt.rs` — `PromotionReceiptV1`
- `src/backend/handshake_core/src/kernel/kb003_promotion/decision.rs` — `PromotionOutcome`, `PromotionRejectionReason`
- `src/backend/handshake_core/src/kernel/kb003_schemas.rs` — schema-id constants, KB003 EventLedger event-type constants
- `src/backend/handshake_core/src/kernel/kb003_artifact_classes.rs` — `Kb003ArtifactClass`, `HashPolicy`, `Kb003ArtifactMetadata`
- `src/backend/handshake_core/src/kernel/crdt/mod.rs` — CRDT submodule inventory
- `src/backend/handshake_core/src/kernel/crdt/persistence.rs` — `CrdtUpdateRecordV1`, `CrdtStorageAuthorityPosture`
- `src/backend/handshake_core/src/kernel/crdt/state_vector.rs` — `KnowledgeStateVectorV1`, `KnowledgeStateVectorOrdering`
- `src/backend/handshake_core/src/kernel/crdt/ai_edit_proposal.rs` — `AiEditProposalRequestV1`, review state machine
- `src/backend/handshake_core/src/kernel/crdt/yjs_bridge.rs` — `push_yjs_update()`, `YjsUpdateEnvelopeV1`
- `src/backend/handshake_core/src/kernel/sandbox/no_sqlite_tripwire.rs` — `guard_authority_write()`
- `src/backend/handshake_core/src/storage/kb003_storage.rs` — migration SQL patterns, `Kb003Storage` trait
- `src/backend/handshake_core/src/storage/postgres.rs` — EventLedger INSERT pattern (line ~3454)
- `src/backend/handshake_core/src/atelier/mod.rs` — canonical creative module pattern
- `src/backend/handshake_core/src/llm/mod.rs` — `LlmClient` trait (HSK-TRAIT-004), `CompletionRequest`
- `src/backend/handshake_core/src/lib.rs` — `AppState` struct
- `src/backend/handshake_core/src/api/` — API route pattern

**AI garment authoring research (external, 2025-2026):**
- [ChatGarment: Garment Estimation, Generation and Editing via LLMs (CVPR 2025)](https://chatgarment.github.io/)
- [ChatGarment GitHub](https://github.com/biansy000/ChatGarment)
- [ChatGarment arxiv](https://arxiv.org/html/2412.17811v1)
- [AIpparel: A Multimodal Foundation Model for Digital Garments (CVPR 2025)](https://georgenakayama.github.io/AIpparel/)
- [AIpparel arxiv](https://arxiv.org/abs/2412.03937)
- [Image2Garment: Simulation-ready Garment Generation from a Single Image (Jan 2026)](https://arxiv.org/abs/2601.09658)
- [NGL-Prompter: Training-Free Sewing Pattern Estimation (Feb 2026)](https://arxiv.org/abs/2602.20700)
- [GarmentCode (ETH Zurich, Python)](https://github.com/maria-korosteleva/GarmentCode)
- [pygarment on PyPI](https://pypi.org/project/pygarment/2.0.0)

**CRDT and event sourcing (external):**
- [Loro CRDT library (Rust)](https://github.com/loro-dev/loro)
- [Loro Extended PostgreSQL storage adapter](https://github.com/schoolAI/loro-extended)
- [yrs Yjs Rust port](https://lib.rs/crates/yrs)
- [event_sourcing.rs (sqlx + Postgres)](https://github.com/primait/event_sourcing.rs)
- [PostgreSQL event sourcing reference](https://github.com/eugene-khyst/postgresql-event-sourcing)

**GPU compute (external):**
- [wgpu (Rust, v29)](https://wgpu.rs/doc/wgpu/)
- [WebGPU Shading Language W3C spec](https://www.w3.org/TR/WGSL/)
- [wgpu-llm-core (wgpu WGSL compute)](https://lib.rs/crates/wgpu-llm-core)
