---
file_id: codebase-inventory
topic_id: T-CODEBASE
title: "Handshake Codebase Inventory (wtc-kernel-009)"
status: draft
depends_on: []
summary: "Complete inventory of the Handshake wtc-kernel-009 worktree: crate layout, Postgres/EventLedger APIs, sandbox/promotion gate, CRDT layer, model-lane surfaces, typed-event conventions, build commands, and the exact extension points the Tailor creative module must attach to."
sources: 21
updated_at: "2026-06-17"
---

## [T-CODEBASE] Handshake Codebase Inventory (wtc-kernel-009)

### [T-CODEBASE.worktree-layout] Worktree and Crate Layout

The worktree at `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009` contains one backend Rust crate and a Tauri 2 shell.

```
wtc-kernel-009/
  src/
    backend/
      handshake_core/          <- single Rust crate
        Cargo.toml
        src/
          lib.rs               <- AppState, module tree, top-level pub use
          main.rs              <- binary entry
          kernel/              <- kernel primitives (events, actors, sandbox, CRDT, model adapter)
          storage/             <- Database trait, PostgresDatabase impl, kb003_storage, CRDT persistence
          atelier/             <- creative module pattern to follow; PgPool domain store + EventLedger
          api/                 <- axum Routers per domain
          llm/                 <- LlmClient trait (HSK-TRAIT-004) + adapters
          model_runtime/       <- ModelRuntime trait (local inference)
          ...
        migrations/            <- 292+ numbered .sql files; sqlx migrate on startup
  app/
    src-tauri/                 <- Tauri 2 shell; thin IPC commands delegate to handshake_core
  justfile                     <- build/test/validate recipes
```

The crate name is `handshake_core`. The single binary entry is `handshake_core` (requires `app-runtime` feature). All feature gates of interest are in `Cargo.toml` lines 246-264: `runtime-full` activates all production modules; `kernel-runtime` is a sub-feature; `duckdb-flight-recorder` gates DuckDB; no `wgpu` feature exists today.

**Critical constraint for the Tailor engine:** No wgpu, WGSL, Bevy, or GPU compute dependency exists anywhere in `handshake_core` or `app/src-tauri` today. The XPBD solver GPU crate must be a NEW standalone Rust workspace crate added as a Cargo workspace member or `path =` dep, exposing a CPU-callable async trait boundary. It must NOT be merged into `handshake_core`.

### [T-CODEBASE.appstate] AppState and Shared Handles

`AppState` is defined in `src/lib.rs` lines 294-305. Every domain handler receives it by clone (it wraps `Arc`s):

```rust
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Database>,           // write-side Database trait
    pub flight_recorder: Arc<dyn FlightRecorder>,
    pub diagnostics: Arc<dyn DiagnosticsStore>,
    pub llm_client: Arc<dyn LlmClient>,       // HSK-TRAIT-004: all LLM calls
    pub capability_registry: Arc<CapabilityRegistry>,
    pub session_registry: Arc<SessionRegistry>,
    pub postgres_pool: sqlx::PgPool,          // shared PgPool for domain-owned schemas
}
```

The Tailor creative module receives `AppState` (or its `postgres_pool`) by reference; no separate pool initialization is needed. The `llm_client` field is the gateway to all model-steerable authoring.

### [T-CODEBASE.storage-backend] Storage Backend: PostgreSQL Only

`ControlPlaneStorageMode` in `src/storage/mod.rs` lines 120-165 has exactly one variant:

```rust
pub enum ControlPlaneStorageMode {
    PostgresPrimary,
}
```

There is no SQLite, no file-backed authority, no memory-only fallback. The `no_sqlite_tripwire` guard in `src/kernel/sandbox/no_sqlite_tripwire.rs` enforces this at every KB003 authority write. The Tailor module calls `guard_authority_write(mode)` identically to `kb003_storage.rs` before any garment row INSERT.

The `Database` async trait (`src/storage/mod.rs` line 1964) is the write-side contract: ~80+ methods covering workspaces, documents, CRDT, knowledge, loom, layout state, etc. Cloth garment CRUD extends this trait with new methods (`list_garments`, `get_garment`, `save_garment`, etc.) backed by `sqlx::PgPool` in `storage/postgres.rs`. All reads in non-write paths use `sqlx::PgPool` directly (as atelier does).

`WriteContext` is required for all `Database` trait write methods; it carries `actor_kind`, `actor_id`, `job_id`, `workflow_id`, `edit_event_id`, `resource_id`, and `timestamp` for the audit trail. Cloth writes always supply a `WriteContext` populated from the active `KernelActor`.

### [T-CODEBASE.eventledger] EventLedger Write Pattern

Every mutation in the system emits an EventLedger receipt. The canonical write function is `append_kernel_event_with_executor` in `src/storage/postgres.rs` (~line 3439). The SQL pattern is:

```sql
INSERT INTO kernel_event_ledger (
    event_id, event_version, kernel_task_run_id, session_run_id,
    aggregate_type, aggregate_id, idempotency_key, event_type,
    actor_kind, actor_id, causation_id, correlation_id,
    payload_hash, source_component, payload, created_at
) VALUES (...)
ON CONFLICT (idempotency_key) DO NOTHING
RETURNING *
```

Followed by a `SELECT ... FROM kernel_event_ledger WHERE idempotency_key = $1` to return the existing row on conflict. This gives idempotent re-delivery for free.

The builder that constructs `NewKernelEvent` before the INSERT (`src/kernel/mod.rs` line 511):

```rust
NewKernelEvent::builder(
    kernel_task_run_id,   // String
    session_run_id,       // String
    event_type,           // KernelEventType variant
    actor,                // KernelActor
)
.aggregate("garment", garment_id.to_string())
.idempotency_key(format!("garment-draft-proposed:{garment_id}:{content_hash}"))
.payload(json!({ "garment_id": garment_id, ... }))
.source_component("tailor")
.build()
```

`event_version` defaults to `"kernel_event_v1"`. `payload_hash` is the SHA-256 hex of the canonical JSON payload bytes.

### [T-CODEBASE.kernel-event-types] KernelEventType Enum and Extension Convention

`KernelEventType` is defined in `src/kernel/mod.rs` lines 204-284 as a `SCREAMING_SNAKE_CASE` serde enum. As of the inspected revision it has 54 variants. All variants must be registered in `required_first_slice_events()` (lines 383-439) to be recognized by `TryFrom<&str>`.

The Tailor engine adds new variants to this enum. Following the existing `Knowledge*` and `AtelierDomainEventRecorded` naming conventions:

```rust
// Tailor EventLedger event variants to add
TailorGarmentDraftProposed,
TailorSimRunStarted,
TailorSimRunCompleted,
TailorSimRunRejected,
TailorPatternValidated,
TailorGarmentPromoted,
TailorGarmentCrdtUpdateRecorded,
TailorMaterialLibraryUpdated,
```

Each variant's `as_str()` arm follows the `SCREAMING_SNAKE_CASE` pattern, e.g. `"GARMENT_DRAFT_PROPOSED"`. Each must be added to `required_first_slice_events()`.

### [T-CODEBASE.kernel-actors] KernelActor Enum

`KernelActor` in `src/kernel/mod.rs` lines 456-492 has seven variants. Tailor uses them as follows:

```rust
pub enum KernelActor {
    Operator(String),          // operator approvals in promotion gate
    System(String),            // solver runs triggered by scheduler
    SessionBroker(String),     // session orchestration
    ModelAdapter(String),      // LLM-authored garment proposals
    ToolGate(String),          // MCP tool gate for model garment tools
    ValidationRunner(String),  // mesh topology / seam / collision validation
    PromotionGate(String),     // promotion gate writing accepted garment rows
}
```

The `actor_kind()` / `actor_id()` methods serialize to the `actor_kind` / `actor_id` columns in `kernel_event_ledger`.

### [T-CODEBASE.model-adapter] ModelAdapter Trait (LLM Steerability)

`ModelAdapter` trait in `src/kernel/model_adapter.rs` line 122:

```rust
#[async_trait]
pub trait ModelAdapter: Send + Sync {
    fn adapter_id(&self) -> &str;
    async fn invoke(&self, request: ModelAdapterRequest) -> KernelResult<ModelAdapterOutput>;
}
```

`ModelAdapterRequest` carries a `ContextBundle` and a `KernelActor`. `ModelAdapterOutput` carries:

```rust
pub struct ModelAdapterOutput {
    pub adapter_id: String,
    pub context_bundle_id: String,
    pub response_text: String,
    pub response_event_type: KernelEventType,
    pub tool_request: KernelToolRequest,
    pub artifact_proposal: ArtifactProposalDraft,
    pub artifact_payload: serde_json::Value, // <-- garment JSON lands here
    pub output_hash: String,
}
```

For Tailor, the `TailorModelAdapter` implements this trait. The `ContextBundle` carries the garment description text or image reference, measurement spec, and active fabric preset. `artifact_payload` carries the model-generated garment JSON (panel vertices, edge lists, seam definitions, material parameters). `output_hash` is SHA-256 of the canonical payload JSON. A `DummyEchoModelAdapter` exists as a test implementation pattern to copy.

### [T-CODEBASE.llm-client] LlmClient Trait (HSK-TRAIT-004)

`LlmClient` in `src/llm/mod.rs`:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn completion(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    fn cancel(&self, model_id: &str, token: CancellationToken) { token.cancel(); }
    async fn swap_model(&self, req: ModelSwapRequestV0_4) -> Result<(), LlmError> { ... }
}
```

Per Master Spec §4.2.3: all application code MUST interact with LLMs through this trait. Tailor calls `llm_client.completion(req)` (available via `AppState.llm_client`) to invoke model-steerable garment authoring. Existing adapters: `OllamaAdapter`, `openai_compat`. Tailor does NOT implement `LlmClient`; it consumes it.

### [T-CODEBASE.model-runtime] ModelRuntime Trait (Local Inference)

`ModelRuntime` trait in `src/model_runtime/trait.rs`:

```rust
#[async_trait]
pub trait ModelRuntime: Send + Sync {
    async fn load(&mut self, spec: LoadSpec) -> Result<ModelId, ModelRuntimeError>;
    async fn unload(&mut self, id: ModelId) -> Result<(), ModelRuntimeError>;
    fn generate(&self, req: GenerateRequest) -> TokenStream;
    async fn score(&self, id: ModelId, sequence: Vec<u32>) -> Result<Score, ModelRuntimeError>;
    async fn embed(&self, id: ModelId, text: &str) -> Result<Embedding, ModelRuntimeError>;
    fn capabilities(&self, id: ModelId) -> Result<&ModelCapabilities, ModelRuntimeError>;
    fn kv_cache(&self, id: ModelId) -> Result<KvCacheHandle, ModelRuntimeError>;
    fn lora_stack(&self, id: ModelId) -> Result<LoraStackHandle, ModelRuntimeError>;
    fn steering_hooks(&self, id: ModelId) -> Result<SteeringHookHandle, ModelRuntimeError>;
    fn cancel(&self, token: CancellationToken);
}
```

The Tailor module does NOT implement `ModelRuntime`. It consumes it via `model_runtime/registry.rs` when fine-grained token-stream access or LoRA steering is needed. The XPBD solver is not an LLM; it is a separate physics compute crate, not a `ModelRuntime` implementor.

### [T-CODEBASE.sandbox] Sandbox Adapter Trait and Lifecycle

`SandboxAdapter` trait in `src/kernel/sandbox/adapter.rs` line 85:

```rust
pub trait SandboxAdapter: Send + Sync {
    fn kind(&self) -> AdapterKind;

    fn pre_check(
        &self,
        run: &SandboxRunV1,
        policy: &SandboxPolicyV1,
        requested: &[SandboxCapability],
    ) -> Result<(), SandboxDenialRecordV1>; // default impl iterates and denies first violation

    fn run(
        &self,
        run: &SandboxRunV1,
        workspace: &SandboxWorkspaceV1,
        policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError>;
}
```

`AdapterIsolationTier` has three variants: `Process` (day-one default, native Rust child), `HardIsolation` (opt-in container/microVM), `Wasm` (reserved). The Tailor solver sandbox uses `Process` tier.

`AdapterRunOutcome` is:

```rust
pub enum AdapterRunOutcome {
    Started,
    Completed { artifact_refs: Vec<String> },
    Denied(SandboxDenialRecordV1),
}
```

`artifact_refs` in `Completed` are the paths/references to simulated mesh artifacts. The sandbox run lifecycle is stored in `kb003_sandbox_runs` table as `REQUESTED -> STARTED -> COMPLETED | REJECTED`.

For Tailor, `TailorSandboxAdapter` implements `SandboxAdapter`: `run()` invokes the XPBD solver crate (the standalone `tailor-solver` crate) via its public async trait, serializes the simulated mesh data to an artifact bundle, and returns `Completed { artifact_refs }`.

### [T-CODEBASE.promotion-gate] Promotion Gate

`PromotionGate` in `src/kernel/kb003_promotion/gate.rs`. The `evaluate()` method signature:

```rust
pub fn evaluate<S: Kb003Storage>(
    inputs: PromotionGateInputs<'_>,
    storage: &mut S,
) -> Result<PromotionGateOutput, PromotionGateError>
```

`PromotionGateInputs` bundles:
- `sandbox_run: &SandboxRunV1` — the completed solver run
- `validation_report: &ValidationReport` — mesh topology, seam closure, collision checks
- `artifact_bundle: &Kb003ArtifactBundleV1` — simulated mesh artifacts
- `approval: OperatorApprovalEvidence` — operator review receipt
- `idempotency_key: String` — keyed on `(garment_id, solver_run_id, content_hash)`
- `treat_advisory_as_blocking: bool` — strict mode flag

`PromotionGateOutput` carries `decision: PromotionDecisionV1` (outcome = `Accepted | Rejected { reason }`) and `receipt: PromotionReceiptV1`. The receipt is always issued regardless of outcome.

`PromotionRejectionReason` is a typed enum covering `ValidationFailure`, `MissingApproval`, `SandboxDenied`, `ArtifactMissing`, `DuplicateIdempotencyKey`, `PostgresFailure`, etc. Garment-specific rejection reasons (e.g. `OpenSeamDetected`, `MeshSelfIntersection`) can be mapped to `ValidationFailure` with descriptor names surfaced in the report, or a new enum variant can be added.

### [T-CODEBASE.crdt] CRDT Layer

The CRDT module lives at `src/kernel/crdt/` with submodules:

| File | Purpose |
|---|---|
| `actor_site.rs` | `KnowledgeActorKind` enum, `CrdtActorSiteIdV1` derivation (SHA-256 of kind+id+workspace) |
| `state_vector.rs` | `KnowledgeStateVectorV1` (BTreeMap site→clock, `hsk-sv1:` prefix), causality verdicts |
| `persistence.rs` | `CrdtUpdateRecordV1`, `CrdtSnapshotRecordV1`, schema IDs |
| `ai_edit_proposal.rs` | AI edit proposals with lease guards and denial receipts |
| `yjs_bridge.rs` | Yjs CRDT update bytes encoding/decoding |
| `promotion_bridge.rs` | CRDT promotion from draft state to authority after gate |
| `claim_promotion.rs` | Lease claims and recovery |
| `rich_document_snapshot.rs` | Snapshot records for full document state |

`CrdtUpdateRecordV1` structure (from `persistence.rs`):

```rust
pub struct CrdtUpdateRecordV1 {
    pub schema_id: String,              // "hsk.kernel.crdt_update_record@1"
    pub workspace_id: String,
    pub document_id: String,
    pub crdt_document_id: String,       // garment_id maps here for Tailor domain
    pub update_id: String,
    pub update_seq: u64,
    pub update_sha256: String,
    pub update_bytes_ref: String,       // pointer to the encoded CRDT delta bytes
    pub actor_id: String,
    pub actor_kind: String,
    pub session_id: String,
    pub trace_id: String,
    pub state_vector_before: String,    // "hsk-sv1:site1=3,site2=1"
    pub state_vector_after: String,
    pub replay_metadata: CrdtReplayMetadataV1,
    pub event_ledger_stream_id: String,
    pub event_ledger_event_id: String,
    pub storage_authority: CrdtStorageAuthorityPosture, // always PostgresEventLedger
}
```

For collaborative garment pattern editing: `crdt_document_id = garment_id`. Each panel edit, seam line change, or material property update is a `CrdtUpdateRecordV1` row in `kernel_crdt_updates`. Tailor does NOT need new CRDT infrastructure; it maps garment geometry (control points, seam definitions, fabric property maps) as a new CRDT document type using the existing `kernel_crdt_updates` table via `KnowledgeActorKind::LocalModel` or `KnowledgeActorKind::Operator` actors.

`KnowledgeStateVectorV1` from `state_vector.rs`: a per-site version vector that produces causality verdicts (`Equal | Dominates | DominatedBy | Concurrent`) from `compare(&self, other: &Self)`.

### [T-CODEBASE.kb003-storage-tables] KB003 Storage Tables

From `src/storage/kb003_storage.rs`, the existing sandbox/promotion schema:

```sql
-- sandbox runs
CREATE TABLE IF NOT EXISTS kb003_sandbox_runs (
    run_id              TEXT PRIMARY KEY,
    kernel_task_run_id  TEXT NOT NULL,
    session_run_id      TEXT NOT NULL,
    adapter_kind        TEXT NOT NULL,
    policy_version_id   TEXT NOT NULL,
    workspace_id        TEXT NOT NULL,
    status              TEXT NOT NULL,          -- REQUESTED|STARTED|COMPLETED|REJECTED
    requested_at_utc    TIMESTAMPTZ NOT NULL,
    started_at_utc      TIMESTAMPTZ,
    finished_at_utc     TIMESTAMPTZ,
    denial_id           TEXT,
    artifact_refs       JSONB NOT NULL DEFAULT '[]'::jsonb
);

-- sandbox policies, validation runs, promotion decisions, promotion receipts
-- (parallel tables; FK-linked)
```

Tailor reuses all four KB003 tables. Garment-specific metadata goes into `artifact_refs JSONB` and `summary_json JSONB` columns. New Tailor-domain tables (garment authority rows, simulation run records, material library) are added via numbered migration files following the `0333_*.sql` naming pattern.

### [T-CODEBASE.atelier-pattern] Atelier Domain as the Extension Pattern

`src/atelier/mod.rs` is the canonical example of a creative module attaching to the kernel. Key architectural choices Tailor replicates:

1. **Domain-owned PgPool** — atelier takes `PgPool` directly, not the `Database` trait, for domain-specific tables with custom schemas.
2. **Domain event family constants** — `pub mod event_family { pub const CHARACTER_CREATED: &str = "atelier.character.created"; ... }` — dot-namespaced, hierarchical, discoverable by replay.
3. **Domain error type** — `AtelierError` wraps `sqlx::Error`, `NotFound`, `Conflict`, `ForbiddenStorage`, `Validation`, `EventLedger`, with `AtelierResult<T>` alias.
4. **SQLite guard** — `assert_postgres_url()` at domain init, mirroring `no_sqlite_tripwire`.
5. **Submodule organization** — one `.rs` file per domain sub-area (`core.rs`, `media.rs`, `moodboards.rs`, `pose.rs`, etc.).
6. **EventLedger emission on every mutation** — uses `NewKernelEvent::builder(...)` to emit `KernelEventType::AtelierDomainEventRecorded` with an atelier event family string in the payload.

Tailor follows this exactly:

```
src/tailor/           (new module, mirrors atelier layout)
  mod.rs                    (TailorEngineError, TailorEngineResult, event_family constants)
  garment.rs                (GarmentDraft, GarmentAsset, garment CRUD)
  solver_binding.rs         (TailorSandboxAdapter; bridges to tailor-solver crate)
  simulation.rs             (SimulationRun, sim lifecycle, artifact bundle assembly)
  material.rs               (MaterialLibrary, MaterialPreset, fabric property types)
  seam.rs                   (SeamDefinition, StitchSpec, ratio-sewing types)
  pattern.rs                (PanelGeometry, internal lines, darts/pleats)
  validation.rs             (mesh topology checks, seam closure, collision validation)
  crdt_bridge.rs            (garment-specific CRDT document helpers)
  event_family.rs           (all Tailor domain event family string constants)
  storage_glue.rs           (Tailor-specific sqlx queries using PgPool directly)
```

### [T-CODEBASE.api-layer] API Layer Pattern

`src/api/mod.rs` shows the axum router assembly pattern: each domain exposes a `routes(state: AppState) -> Router` function registered in `api/mod.rs`. Tailor adds `src/api/tailor.rs`:

```rust
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/tailor/garments",              post(create_garment_draft))
        .route("/tailor/garments/:id",          get(get_garment))
        .route("/tailor/garments/:id/simulate", post(trigger_simulation))
        .route("/tailor/garments/:id/promote",  post(promote_garment))
        .route("/tailor/garments/:id/crdt",     get(get_garment_crdt_state))
        .with_state(state)
}
```

Routes are registered in `api/mod.rs` `routes()` alongside existing domain routes.

The Tauri shell (`app/src-tauri/src/commands/`) adds thin Tauri command wrappers (`tailor_simulate`, `tailor_get_garment`, `tailor_promote_garment`) that delegate to the axum layer or call `AppState` methods directly. Business logic stays in `handshake_core`.

### [T-CODEBASE.migrations] Migrations Pattern

The migrations directory contains 292+ numbered `.sql` files (`0001_init.sql` through `0333_loom_ai_suggestions.sql` plus two dated 2026 migrations). All are forward-only. Tailor adds numbered migration files:

```sql
-- 0334_tailor_garments.sql
CREATE TABLE IF NOT EXISTS tailor_garments (
    garment_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id        TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    crdt_document_id    TEXT NOT NULL UNIQUE,
    status              TEXT NOT NULL DEFAULT 'draft', -- draft|simulated|promoted|rejected
    draft_json          JSONB NOT NULL DEFAULT '{}'::jsonb,
    promoted_json       JSONB,
    event_ledger_event_id TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS ix_tailor_garments_workspace ON tailor_garments (workspace_id);
CREATE INDEX IF NOT EXISTS ix_tailor_garments_status ON tailor_garments (status);

-- 0335_tailor_simulation_runs.sql
CREATE TABLE IF NOT EXISTS tailor_simulation_runs (
    sim_run_id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    garment_id          UUID NOT NULL REFERENCES tailor_garments(garment_id),
    sandbox_run_id      TEXT REFERENCES kb003_sandbox_runs(run_id),
    solver_version      TEXT NOT NULL,
    substeps            INTEGER NOT NULL,
    iterations          INTEGER NOT NULL,
    particle_distance_mm REAL NOT NULL,
    status              TEXT NOT NULL DEFAULT 'requested',
    mesh_artifact_ref   TEXT,
    event_ledger_event_id TEXT,
    requested_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at        TIMESTAMPTZ
);

-- 0336_tailor_material_library.sql
CREATE TABLE IF NOT EXISTS tailor_material_presets (
    preset_id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id        TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    properties_json     JSONB NOT NULL, -- stretch_weft, stretch_warp, bend_weft, bend_warp, shear, density, etc.
    event_ledger_event_id TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### [T-CODEBASE.build-commands] Build and Test Commands

From `justfile`:

```bash
# Run all backend tests (the Tailor solver crate adds [[test]] entries here):
cargo test --manifest-path src/backend/handshake_core/Cargo.toml \
    --target-dir ../Handshake_Artifacts/handshake-cargo-target

# Lint:
cargo clippy --manifest-path src/backend/handshake_core/Cargo.toml \
    --all-targets --all-features \
    --target-dir ../Handshake_Artifacts/handshake-cargo-target

# Full validate (docs-check + codex-check + lint + fmt + tests + deny):
just validate

# Tauri dev mode:
just dev
```

`CARGO_TARGET_DIR` is always external (`../Handshake_Artifacts/handshake-cargo-target`). The Tailor solver standalone crate adds its own `[[test]]` entries in `Cargo.toml` parallel to the existing `kernel_crdt_persistence_tests` pattern. The standalone crate Cargo.toml path is e.g. `src/tailor-solver/Cargo.toml`; it is added to the workspace or referenced as a `path =` dep from `handshake_core`.

### [T-CODEBASE.extension-points] Exact Extension Points: Paths and Type Names

This section enumerates every file and type the Tailor creative module touches or implements.

**Extend (add variants / methods to existing files):**

| File | What to add |
|---|---|
| `src/kernel/mod.rs` | 8 new `KernelEventType` variants; register in `required_first_slice_events()` |
| `src/storage/mod.rs` | New `Database` trait methods: `list_garments`, `get_garment`, `save_garment`, `update_garment_status`, `get_garment_crdt_state` |
| `src/storage/postgres.rs` | Concrete impl of new `Database` trait methods using `sqlx::PgPool` + EventLedger receipt pattern |
| `src/api/mod.rs` | Add `pub mod tailor;` import and merge `tailor::routes(state.clone())` in `routes()` |
| `src/lib.rs` | Add `pub mod tailor;` module declaration (behind `runtime-full` feature) |

**Create (new files):**

| File | Type/trait to implement |
|---|---|
| `src/tailor/mod.rs` | `TailorEngineError`, `TailorEngineResult<T>`, `event_family` constants |
| `src/tailor/garment.rs` | `GarmentDraft`, `GarmentAsset`, `GarmentStatus` enum |
| `src/tailor/solver_binding.rs` | `TailorSandboxAdapter` implementing `SandboxAdapter` trait |
| `src/tailor/material.rs` | `MaterialPreset`, `FabricProperties` (weft/warp/shear/bend/density) |
| `src/tailor/seam.rs` | `SeamDefinition`, `StitchSpec`, `SeamRatio` (for 1:N gather) |
| `src/tailor/pattern.rs` | `PanelGeometry`, `InternalLine`, `DartSpec`, `PleatSpec` |
| `src/tailor/crdt_bridge.rs` | Garment CRDT document helpers using existing `kernel_crdt_updates` table |
| `src/tailor/storage_glue.rs` | Cloth-specific sqlx queries for garment tables |
| `src/tailor/validation.rs` | Mesh topology, seam closure, collision checks returning `ValidationReport` |
| `src/api/tailor.rs` | Axum router with 5 endpoints |
| `src/backend/handshake_core/migrations/0334_tailor_garments.sql` | Forward migration |
| `src/backend/handshake_core/migrations/0335_tailor_simulation_runs.sql` | Forward migration |
| `src/backend/handshake_core/migrations/0336_tailor_material_library.sql` | Forward migration |
| (new Cargo workspace member) `src/tailor-solver/` | Standalone `tailor-solver` crate with wgpu + WGSL shaders; no `sqlx`, no Tauri, no Bevy; exposes `pub trait ClothSolver: Send + Sync` |

**Consume (call, not implement):**

| Type | Where | Used for |
|---|---|---|
| `LlmClient` (HSK-TRAIT-004) | `AppState.llm_client` | Model-steerable garment authoring prompts |
| `ModelAdapter` | `TailorModelAdapter` wrapping `LlmClient` | Structured garment JSON proposal via `ContextBundle` |
| `PromotionGate::evaluate()` | `src/kernel/kb003_promotion/gate.rs` | Accept/reject promoted garment assets |
| `CrdtUpdateRecordV1` | `kernel_crdt_updates` table | Record panel/seam collaborative edits |
| `KnowledgeStateVectorV1` | `src/kernel/crdt/state_vector.rs` | Causality comparison for concurrent edits |
| `NewKernelEvent::builder()` | `src/kernel/mod.rs` | Emit EventLedger receipts on every mutation |
| `guard_authority_write()` | `src/kernel/sandbox/no_sqlite_tripwire.rs` | No-SQLite tripwire on every Tailor authority write |
| `SandboxPolicyV1` | `src/kernel/sandbox/policy.rs` | Policy scope for solver sandbox run (no network, scoped fs) |

### [T-CODEBASE.no-gpu-today] GPU / wgpu Status in the Codebase

A complete search of `Cargo.toml` and all source files confirms: no `wgpu`, `wgsl`, `bevy`, `vulkan`, `metal`, `dx12`, or GPU compute dependency exists anywhere in `handshake_core` or `app/src-tauri` as of the inspected revision. The features list in `Cargo.toml` lines 246-264 has no GPU feature gate.

The `tailor-solver` standalone crate introduces `wgpu` (v29, Apache-2/MIT) as a NEW direct dependency scoped to that crate only. `handshake_core` depends on `tailor-solver` via a trait boundary only:

```rust
// In tailor-solver/src/lib.rs
pub trait ClothSolver: Send + Sync {
    async fn run_simulation(&self, input: ClothSimInput) -> Result<ClothSimOutput, ClothSolverError>;
    fn solver_version(&self) -> &str;
}

// In handshake_core/src/tailor/solver_binding.rs
// TailorSandboxAdapter holds a Box<dyn ClothSolver> or Arc<dyn ClothSolver>
// The concrete WGSL GPU solver in tailor-solver implements this trait
// The CPU fallback (for tests / no-GPU environments) also implements this trait
```

This boundary ensures `handshake_core` compiles without wgpu and that the Tailor solver crate is independently testable.

### [T-CODEBASE.risks] Risks and Open Questions

**Risk 1: Single-crate growth.** `handshake_core` is already large (~8 k+ lines in `storage/postgres.rs` alone). Adding Tailor as another domain module follows the atelier pattern exactly, so the risk is managed, but the postgres.rs impl file will keep growing. Mitigation: create a `storage/tailor_storage.rs` sidecar file (like `kb003_storage.rs`) and put Tailor-specific SQL there.

**Risk 2: Migration numbering collision.** 292 migrations already exist; new ones must be numbered above the current highest (`0333`, `2026_05_18`). Use a dated naming convention (`2026_06_17_tailor_garments.sql`) to avoid integer sequence conflicts with other parallel work packets.

**Risk 3: KernelEventType enum size.** Adding 8 variants to a 54-variant enum is fine structurally; the risk is `required_first_slice_events()` becoming a maintenance burden if other work packets add variants simultaneously. Open question: should Tailor events be gated behind an `#[cfg(feature = "tailor-engine")]` feature to keep the baseline binary smaller?

**Risk 4: Async SandboxAdapter.** The current `SandboxAdapter::run()` is synchronous (`fn run`, not `async fn run`). The XPBD solver crate's `ClothSolver::run_simulation` will be async (GPU compute schedules on wgpu's async device). The `TailorSandboxAdapter::run()` will need to block on the async runtime using `tokio::runtime::Handle::current().block_on(...)` until the `SandboxAdapter` trait is made async. This is a known design impedance; document as an open question for the work packet contract.

**Risk 5: CRDT document type for garment geometry.** The existing CRDT layer was designed for rich text documents (Yjs). Garment panel geometry (vertex coordinates, bezier control points, seam edge lists) is a different data shape. Tailor may need a custom conflict resolution strategy (last-write-wins per panel vertex, or operational transform for seam edits) that the generic Yjs CRDT bridge does not provide. Open question: define a garment-specific CRDT document type or use JSONB columns with optimistic locking and event sourcing instead of Yjs updates.

### [T-CODEBASE.sources] Sources

- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/Cargo.toml` — crate manifest, features, test targets, dependency list
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/lib.rs` — AppState struct, module tree
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/mod.rs` — KernelEventType (54 variants), KernelActor, NewKernelEvent builder, required_first_slice_events()
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/model_adapter.rs` — ModelAdapter trait, ModelAdapterRequest, ModelAdapterOutput, ArtifactProposalDraft
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/sandbox/adapter.rs` — SandboxAdapter trait, AdapterIsolationTier, AdapterRunOutcome
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/sandbox/no_sqlite_tripwire.rs` — guard_authority_write(), AuthorityMode, KB003_NO_SQLITE_AUTHORITY_POLICY_ID
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/kb003_promotion/gate.rs` — PromotionGate::evaluate(), PromotionGateInputs, PromotionGateOutput, OperatorApprovalEvidence
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/kb003_artifact_classes.rs` — Kb003ArtifactClass enum, HashPolicy, Kb003ArtifactMetadata
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/crdt/persistence.rs` — CrdtUpdateRecordV1, CrdtStorageAuthorityPosture, schema IDs
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/crdt/state_vector.rs` — KnowledgeStateVectorV1, KnowledgeStateVectorOrdering
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/kernel/crdt/actor_site.rs` — KnowledgeActorKind, CrdtActorSiteIdV1, derivation schema ID
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/storage/mod.rs` — ControlPlaneStorageMode (PostgresPrimary only), Database trait declaration, WriteContext
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/storage/kb003_storage.rs` — MIGRATION_KB003_SANDBOX_RUNS_V1, MIGRATION_KB003_SANDBOX_POLICIES_V1, Kb003Storage trait
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/storage/postgres.rs` — append_kernel_event_with_executor, EventLedger INSERT+ON CONFLICT pattern (~line 3454)
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/atelier/mod.rs` — atelier domain module pattern: PgPool, event_family constants, AtelierError, AtelierResult, module list
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/llm/mod.rs` — LlmClient trait (HSK-TRAIT-004), CompletionRequest, CompletionResponse
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/model_runtime/trait.rs` — ModelRuntime trait, full method surface
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/src/api/mod.rs` — axum router assembly pattern, all registered domain route modules
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/src/backend/handshake_core/migrations/0001_init.sql` — baseline schema (workspaces, documents, blocks)
- `D:/Projects/LLM projects/Handshake/Handshake Worktrees/wtc-kernel-009/justfile` — build commands: test, lint, validate, dev, CARGO_TARGET_DIR
- `https://github.com/gfx-rs/wgpu` — wgpu v29 cross-platform GPU compute backend (absent from handshake_core today; introduced by tailor-solver)
