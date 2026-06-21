---
schema: handshake.indexed_spec.module@1
spec_version: "v02.195"
bundle_id: "master-spec-v02.195"
module_id: "13"
section_id: "13"
title: "13. Tailor -- Cloth/Garment Engine"
source_baseline_version: "v02.194"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "68890d4c67fdb7315319337f7d4eae1d95d4c92e0c145e49348cde21431e4d9b"
body_sha256: "68890d4c67fdb7315319337f7d4eae1d95d4c92e0c145e49348cde21431e4d9b"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 13. Tailor -- Cloth/Garment Engine [TAI-SECTION-001]

Tailor is the Handshake-native cloth/garment authoring and simulation creative module (a Marvelous-Designer-equivalent "detailer"). It attaches to the kernel like the atelier module: a `handshake_core::tailor` domain module bound to PostgreSQL/EventLedger authority, CRDT collaboration, the sandbox->validation->promotion lifecycle, and model lanes, plus a standalone UI-agnostic `tailor-solver` Rust crate (XPBD on WGSL/wgpu; `parry` rigid proxies) reached only through the `ClothSolver` trait. This section is product LAW. The canonical authority for every type, field, unit, event, schema-id, table, migration, validation check, and promotion-equivalence rule is sub-section 13.14 (Canonical Tailor Authority Contracts); where any other sub-section conflicts with 13.14, 13.14 wins. The research package `.GOV/reference/cloth_engine_research/` is non-normative provenance only.

## 13.1 Overview, Scope, and Model-First Differentiator

---

### 1. What Tailor Is

Tailor is the Handshake-native garment-authoring and cloth-simulation creative module. It is a
kernel-attached creative module in the same architectural position as the atelier module
(`src/atelier/`): a domain subdirectory under `handshake_core` that receives `AppState` by
reference, emits `KernelEventType` variants to the EventLedger, operates through the kernel
sandbox and `PromotionGate`, and persists authority rows to PostgreSQL via `sqlx::PgPool`.

Tailor MUST consist of two compile units:

1. **`handshake_core::tailor`** (`src/tailor/`) — the kernel-bound creative module: authority
   storage, EventLedger emission, CRDT collaborative editing, sandbox dispatch, promotion gate
   integration, model-lane binding, and REST API surface (`src/api/tailor.rs`).

2. **`tailor-solver`** — a standalone Rust workspace crate with no dependency on `handshake_core`.
   It contains the XPBD cloth simulation loop, WGSL compute shaders, `wgpu` device management,
   rigid collision proxies (via `rapier`/`parry`), and the `ClothSolver` trait that forms the
   crate's public boundary. `handshake_core::tailor` calls into `tailor-solver` only through
   `ClothSolver`.

The term **cloth** MUST be used inside `tailor-solver` for physics types (`ClothSolver`,
`ClothParticle`, `ClothConstraint`, `ClothBodyProxy`). The term **Garment** / **Tailor** MUST be
used for all domain, feature, event, table, and schema-id identifiers in `handshake_core::tailor`
and in the REST/MCP API surface. This dual-terminology rule resolves the physics-vs-domain naming
tension throughout the system; it is not a inconsistency.

---

### 2. Feature Scope

#### 2.1 In Scope

Tailor MUST implement the following capability groups, each corresponding to a downstream spec
sub-section:

| Capability group | Sub-section |
|---|---|
| 2D sewing-pattern authoring: panels, darts, pleats, Bézier edges, grain | T-PATTERN-SCHEMA |
| Seam and sewing constraints, including M:N ratio gathering | T-SEAM-CONSTRAINTS |
| Anisotropic fabric model: weft/warp/shear/buckling, normalized [0,1] LLM surface | T-FABRIC-MODEL |
| XPBD GPU cloth solver: substeps, iterations, self-collision, cloth-avatar collision | T-SOLVER-CORE |
| Self-collision and inter-layer spacing | T-COLLISION |
| Avatar and body-proxy binding for fit and collision | T-AVATAR-BINDING |
| Keyframeable physical properties and animation timeline | T-ANIMATION |
| Rigid trim coupling: buttons, zippers, lacing, pattern-to-trim conversion | T-TRIM-RIGID |
| UV-from-pattern: ARAP flatten, island packing, grain-accurate UVs | T-UV-TEXTURE |
| Garment authority storage: `tailor_*` PostgreSQL tables, EventLedger | T-ASSET-AUTHORITY |
| Import/export interoperability: OBJ, FBX, Alembic, glTF, GarmentCode JSON | T-PIPELINE-INTEROP |
| LLM-steerable garment authoring via `TailorModelAdapter` and MCP tools | T-MODEL-LANE |
| Sandbox, validation gate (~35 checks), and `PromotionGate` integration | T-KERNEL-INTEGRATION |
| WGSL compute shader architecture | T-WGSL-SHADERS |
| Validation check catalog (`TailorValidationDescriptor`) | T-VALIDATION |

The feature ceiling for "fully implemented" MUST be Marvelous Designer 2026.0 as documented in the
non-normative research source `cloth_engine_research/02-md-feature-map.md` (T-MD-FEATURES). All
eight D4 moat features identified there MUST be addressed by a corresponding normative requirement
in the sub-section that owns them; MOAT-7 (EveryWear-equivalent automated game rigging) MAY be
deferred to post-v1 but MUST be noted as a deferred requirement in the T-PIPELINE-INTEROP
sub-section.

#### 2.2 Out of Scope

The following are explicitly out of scope for Tailor:

- **SQLite.** Tailor MUST NOT introduce any SQLite dependency anywhere — not in `tailor-solver`,
  not in tests, not as a development cache. The `no_sqlite_tripwire` and `assert_postgres_url`
  kernel tripwires apply to the Tailor module without exception.

- **External cloth-authoring applications.** Tailor is a Handshake-native module. It MUST NOT
  require a Marvelous Designer, CLO3D, or any other subscription-gated or platform-locked tool at
  runtime. Interoperability with those tools' export formats (OBJ, FBX, `.zprj` pattern JSON) is
  in scope as import/export; runtime dependency on them is not.

- **Bevy / Avian as a production dependency.** Bevy and Avian physics MAY be used exclusively
  inside `tailor-solver/examples/testbed/` as a throwaway interactive viewport for constraint
  validation during solver development. They MUST NOT appear in `tailor-solver/src/` or in any
  dependency of `handshake_core`.

- **OpenAI / Anthropic / Google as required infrastructure.** The model-lane binding
  (`TailorModelAdapter`) MUST use the kernel's existing `LlmClient` trait and model-lane registry.
  No specific LLM provider is required; the existing Ollama and OpenAI-compatible adapters
  already registered in the kernel are sufficient. Tailor MUST NOT add a provider-specific
  dependency.

- **`tailor-solver` as a Tauri command surface.** Tauri command bindings for Tailor
  (`tailor_simulate`, `tailor_get_garment`, `tailor_promote_garment`) are the final integration
  layer and MUST be authored only after `handshake_core::tailor` and `tailor-solver` are
  integration-tested against the kernel sandbox and promotion gate.

- **Numbered `0NNN_*` SQL migrations.** Tailor MUST use the dated migration convention
  (`YYYY_MM_DD_tailor_<topic>.sql` + `.down.sql`). Numbered migrations are forbidden because the
  `0001`–`0335` integer space is a shared sequence other work packets append to; dated names
  cannot collide on that sequence. See T-CONTRACTS §migration-naming.

---

### 3. The Model-First Differentiator

Tailor's defining differentiator over all current cloth-authoring tools, including Marvelous
Designer 2026.0, is **model-steerability as a first-class design constraint**, not a post-hoc
plugin.

#### 3.1 The Gap Tailor Fills

As of 2026.0, Marvelous Designer's AI features are limited to a cloud-gated AI Pose Generator
(Beta) and an AI Image Generator texture plugin. It has no LLM-steerable sewing-pattern authoring
and no structured JSON API a model can call to propose, edit, or validate a garment. The research
field has independently demonstrated this pipeline is achievable — ChatGarment (CVPR 2025),
NGL-Prompter (arXiv 2602.20700, Feb 2026), GarmentDiffusion (IJCAI 2025), and
Design2GarmentCode (CVPR 2025) each prove that a VLM or diffusion model can produce a valid
structured garment representation from text, image, or sketch input. Handshake is the first
production system to own this pipeline natively inside kernel model-lane infrastructure.

The non-normative research provenance for this differentiator is `cloth_engine_research/00-overview.md`
(T-OVERVIEW.differentiator) and `cloth_engine_research/02-md-feature-map.md`
(T-MD-FEATURES.group-11-ai-model-steerability). This sub-section states the normative
requirements that make the differentiator real.

#### 3.2 Normative Model-Steerability Requirements

**[TAI-OVR-001]** Tailor MUST provide a `TailorModelAdapter` that implements the kernel's
`ModelAdapter` trait and is registered in the model-lane registry. It MUST accept a
`ContextBundle` containing a `GarmentSpec` (schema id `hsk.tailor.garment_spec@1`) and a
natural-language constraint description, invoke an LLM via the `LlmClient` trait, and extract a
`GarmentSpec` from `artifact_payload`.

**[TAI-OVR-002]** `GarmentSpec` MUST be the single type shared between the LLM's primary output
surface and the solver's primary input surface and the PostgreSQL authority JSONB column
(`tailor_garments.spec_json`). It MUST derive `schemars::JsonSchema` so its MCP `inputSchema` is
auto-generated without manual schema maintenance.

**[TAI-OVR-003]** `GarmentSpec` MUST use centimetres for all physical lengths (field names MUST
carry a `_cm` suffix on every length field), normalized `[0.0, 1.0]` for fabric stiffness
parameters (where `1.0` = stiffest), and `f32` for the `gather_ratio` field on `SeamSpec`
(defined as `from_length / to_length`, valid range `(0.0, 20.0]`). These unit conventions MUST
NOT be changed without a new major schema version increment. See T-CONTRACTS §garment-spec for
the full canonical type.

**[TAI-OVR-004]** A model-authored `GarmentSpec` MUST NOT be written directly to a Postgres
authority row. It MUST enter the kernel sandbox (`SandboxAdapter` trait, process-tier by default),
be validated by the `TailorValidationDescriptor` (the ~35-check catalog in T-CONTRACTS
§validation), and pass the `PromotionGate` (`PromotionDecisionV1: Accepted`) before the garment
row receives `status = 'promoted'`. This lifecycle is not optional and MUST NOT be bypassed for
model-authored garments regardless of LLM confidence.

**[TAI-OVR-005]** The `SimulationReceipt` type (schema id `hsk.tailor.simulation_receipt@1`) MUST
be returned to the model as MCP `structuredContent` after every simulate or validate call. It MUST
include `validation_findings` carrying the stable `code` values from T-CONTRACTS §validation, a
`severity` of `"blocking" | "advisory" | "info"`, and an optional `suggested_fix` with a
JSON-pointer `field_path` into `GarmentSpec` and a `suggested_value`. This gives the model a
deterministic self-correction loop without natural-language interpretation of error text.

**[TAI-OVR-006]** Promotion equivalence for garments validated across different GPU backends or
driver versions MUST use the `MeshComparator` tolerance check (max per-vertex position deviation
≤ `epsilon_mm`, default `0.1 mm`), NOT a SHA-256 `content_hash` equality check. Content hashes
are reserved for same-machine, same-run idempotency and EventLedger receipt fingerprinting only.
See T-CONTRACTS §determinism.

**[TAI-OVR-007]** Every garment authoring session MUST be fully reproducible from EventLedger
receipts alone. A replay of the `Tailor*` event sequence for a garment MUST reconstruct the
complete authority state of that garment without reference to chat history, session context, or
agent-local memory.

#### 3.3 Contrast with Marvelous Designer

| Capability dimension | Marvelous Designer 2026.0 | Tailor (this spec) |
|---|---|---|
| LLM-steerable pattern authoring | None | Required: `TailorModelAdapter` + MCP tools |
| Structured garment JSON API | None (GUI-only) | `GarmentSpec` (`hsk.tailor.garment_spec@1`) |
| Sandbox + validation gate | None | Required: ~35-check `TailorValidationDescriptor` |
| EventLedger audit trail | None | Required: every mutation emits a `Tailor*` event |
| CRDT collaborative editing | Cloud sync only (CLO-SET) | Required: `TailorPanelCrdtUpdateRecorded` |
| Promotion equivalence (cross-backend) | N/A | Required: `MeshComparator` ε=0.1 mm |
| Operator-only control | GUI-first | Model-first; operator is a participant, not the only actor |

Tailor MUST NOT replicate MD's GUI-first interaction model as its design center. The GUI (Tauri
command surface) is a projection over the model-steerable kernel authority, not the primary
authoring path.

---

### 4. Kernel Creative Module Framing

Tailor is a **creative module** in the Handshake kernel, not a standalone application and not a
plugin. This framing has the following normative consequences.

**[TAI-OVR-008]** `handshake_core::tailor` MUST follow the atelier module pattern:
- A `src/tailor/` directory with `mod.rs` defining `TailorEngineError` and `event_family`
  constants.
- Storage glue in `src/tailor/storage_glue.rs` (parallel to `storage/kb003_storage.rs`).
- Axum routes in `src/api/tailor.rs` registered in `api/mod.rs`.
- All module files receive `AppState` (and thus `PgPool`, `LlmClient`, `SandboxRunner`,
  `PromotionGate`, and CRDT infrastructure) by reference. No separate initialization is permitted.

**[TAI-OVR-009]** `handshake_core`'s `Cargo.toml` MUST NOT gain `wgpu`, `CubeCL`, or any WGSL
dependency. All GPU code is isolated in the `tailor-solver` workspace crate. `handshake_core`
accesses the solver only through the `ClothSolver: Send + Sync` trait boundary defined in
`tailor-solver/src/lib.rs`.

**[TAI-OVR-010]** The `tailor-solver` crate MUST be a Cargo workspace member in the Handshake
monorepo, not a separately versioned external crate, so solver and kernel are always tested
together.

**[TAI-OVR-011]** The Tailor module MUST NOT be activated as a build-target work packet until the
Handshake kernel governance baseline (WP-KERNEL-009 and its successors) is stable enough that the
sandbox, promotion gate, and CRDT surfaces it depends on are not simultaneously under active
structural change. The `tailor-solver` crate MAY be prototyped in isolation before the kernel is
ready, because it has no `handshake_core` dependency.

**[TAI-OVR-012]** EventLedger event variants for Tailor MUST follow the canonical addition list in
T-CONTRACTS §event-types. The Tailor module adds variants to `KernelEventType` in
`kernel/mod.rs` (wire format `TAILOR_*` SCREAMING_SNAKE_CASE via `as_str()`) and registers every
variant in `required_first_slice_events()`. No variant from the superseded-names list in
T-CONTRACTS §event-types MUST appear in new code.

**[TAI-OVR-013]** PostgreSQL tables for Tailor MUST use `TEXT PRIMARY KEY` with prefixed string
IDs (e.g., `garment_id = "GAR-{uuid_v7}"`, `avatar_id = "AVT-{uuid_v7}"`) following the
codebase convention established in migrations `0332_media_asset_tiers.sql` and
`0334_loom_canvas_boards.sql`. UUID primary keys with `gen_random_uuid()` are off-convention for
new Tailor tables and MUST NOT be used.

**[TAI-OVR-014]** Every Tailor authority table row MUST carry an `event_ledger_event_id TEXT NOT
NULL` foreign key to `kernel_event_ledger.event_id`. Every INSERT or mutating UPDATE to a Tailor
table MUST be preceded by a `guard_authority_write(AuthorityMode::Postgres)` call enforcing the
`no_sqlite_tripwire`. The canonical table set is defined in T-CONTRACTS §tables; no additional
tables are authoritative until they appear there or in a subsequent normative amendment to that
section.

---

### 5. Build-Order Summary

The build order for the Tailor module is normative:

1. **`tailor-solver` prototype (independent).** The solver crate, XPBD constraints, WGSL shaders,
   and `MeshComparator` MAY be prototyped as a standalone workspace crate before the kernel
   governance baseline is stable. This does not require a running Handshake kernel.

2. **Kernel prerequisite gate.** `handshake_core::tailor` MUST NOT be authored or integrated until
   the sandbox (`SandboxAdapter`), `PromotionGate`, and CRDT layer are stable against concurrent
   kernel work packets.

3. **`handshake_core::tailor` module.** Domain types, storage glue, EventLedger binding, CRDT
   documents, model adapter, and sandbox adapter are authored and integration-tested against the
   kernel primitives.

4. **Tauri command surface.** `tailor_simulate`, `tailor_get_garment`, and `tailor_promote_garment`
   Tauri commands are the final layer, added after step 3 passes integration tests.

---

### 6. Non-Normative Research Provenance

The design decisions in this module are grounded in the following non-normative research sources,
available in `cloth_engine_research/`:

- `00-overview.md` (T-OVERVIEW) — vision, Handshake-native constraints, differentiator, OSS
  landscape, module topology, build order, risks.
- `02-md-feature-map.md` (T-MD-FEATURES) — complete Marvelous Designer 2026.0 feature taxonomy
  with difficulty ratings and moat flags; the requirements ceiling for feature coverage.
- `16-contracts.md` (T-CONTRACTS) — the canonical authority for all types, field names, units,
  event variants, schema IDs, table definitions, migration naming, validation check catalog,
  and promotion-equivalence resolution. All normative requirements in this and all other Tailor
  sub-sections MUST use T-CONTRACTS as the contract surface; where any other research topic
  conflicts with T-CONTRACTS, T-CONTRACTS wins.

These documents are reference material, not product law. This spec sub-section and its siblings
are product law.

---
file_id: tailor-spec-02-architecture
spec_id: TAILOR-SPEC-02
title: "Architecture: tailor-solver Crate + handshake_core::tailor Module"
status: draft
section: "### <N>.<i> Architecture: tailor-solver Crate + handshake_core::tailor Module"
provenance_research:
  - "T-CODEBASE (01-codebase-inventory.md)"
  - "T-CLOTH-SOLVER (04-cloth-solver.md)"
  - "T-KERNEL-INTEGRATION (10-kernel-integration.md)"
  - "T-CONTRACTS (16-contracts.md) — canonical authority for all types, names, events, schemas"
updated_at: "2026-06-17"
---

## 13.2 Architecture: tailor-solver Crate + handshake_core::tailor Module

This section specifies the two-component architecture of the Tailor garment engine: the
standalone `tailor-solver` Rust crate that owns all GPU physics, and the
`handshake_core::tailor` module that owns all kernel authority. These two components MUST remain
strictly separated at a defined trait boundary. No exceptions to the separation are
permitted by this spec.

Non-normative provenance: `T-CODEBASE`, `T-CLOTH-SOLVER`, and `T-KERNEL-INTEGRATION` in the
research package at
`wt-gov-kernel/.GOV/reference/cloth_engine_research/` contain design rationale, OSS evidence,
and implementation sketches. `T-CONTRACTS` (`16-contracts.md`) is the canonical authority for
every type name, event variant, wire string, schema ID, table name, migration name, and
validation check cited in this section. Where any research-package document conflicts with
`T-CONTRACTS`, `T-CONTRACTS` governs.

---

##### <N>.<i>.1 Crate Split and Dependency Direction

**ARCH-001.** The garment engine MUST be implemented as exactly two units:

1. `tailor-solver` — a standalone Rust crate (Cargo workspace member) that implements
   XPBD cloth physics via wgpu v29 / WGSL compute shaders.
2. `handshake_core::tailor` — a domain module in the `handshake_core` crate
   (source path `src/tailor/`) that owns all PostgreSQL authority, EventLedger receipts,
   CRDT collaboration, sandbox lifecycle, promotion gate binding, and model-lane surfaces.

**ARCH-002.** Dependency direction MUST be one-way: `handshake_core::tailor` depends on
`tailor-solver` via a trait object (`Box<dyn ClothSolver>` or `Arc<dyn ClothSolver>`).
`tailor-solver` MUST NOT depend on `handshake_core`, `sqlx`, `tauri`, or any Handshake
kernel crate. Circular dependencies are forbidden.

**ARCH-003.** No file in `handshake_core` other than `src/tailor/solver_binding.rs`
MUST import or reference anything from the `tailor-solver` crate. This is the single kernel
entry point for GPU physics.

**ARCH-004.** `wgpu`, `wgsl_to_wgpu`, `bytemuck`, `encase`, `parry3d`, and any other GPU or
physics-geometry dependency MUST appear only in `tailor-solver/Cargo.toml`. They MUST NOT be
added to `handshake_core/Cargo.toml`.

**Rationale.** `handshake_core` has no GPU compute dependency today (verified in `T-CODEBASE`
`FACT-NO-GPU`). Introducing `wgpu` into the kernel crate would pull Naga, SPIR-V toolchains,
and platform GPU SDK bindings into every build of the application, including CI runs that
do not exercise the cloth solver.

---

##### <N>.<i>.2 The ClothSolver Trait (Canonical Boundary Contract)

**ARCH-005.** `tailor-solver` MUST expose the following public trait as its primary boundary.
The canonical form is authoritative; implementations MUST match it exactly.

```rust
// tailor-solver/src/lib.rs

/// The sole boundary between handshake_core::tailor and the GPU physics crate.
/// Two implementors are required: ClothSolverGpu (wgpu) and ClothSolverCpu (test/no-GPU
/// fallback). Both must implement this trait identically.
#[async_trait::async_trait]
pub trait ClothSolver: Send + Sync {
    /// One-time initialization: upload GarmentSpec mesh and material to the GPU.
    /// MUST be called before simulate(). Calling simulate() without a prior successful
    /// load_garment() MUST return ClothSolverError::NotLoaded.
    async fn load_garment(
        &mut self,
        mesh: SolverMesh,
        material: FabricMaterial,
    ) -> Result<(), ClothSolverError>;

    /// Run the XPBD simulation for n_frames. Returns the final simulated mesh state.
    /// MUST be deterministic per-backend: identical inputs, same GPU backend and driver,
    /// MUST produce identical SolverResult byte-for-byte.
    async fn simulate(
        &mut self,
        n_frames: u32,
        params: SimRunParams,
    ) -> Result<SolverResult, ClothSolverError>;

    /// Upload keyframeable material parameter overrides for the current substep
    /// (MD MOAT-4: per-frame solidify, pressure, shrinkage, tack compliance).
    /// MAY be called between simulate() calls for animation runs.
    fn update_params(&mut self, frame_params: MaterialFrameParams);

    /// Unload the current garment and free all GPU buffers. After unload(),
    /// simulate() MUST return ClothSolverError::NotLoaded until load_garment() is called again.
    async fn unload(&mut self);

    /// SHA-256 of the final position buffer from the most recent simulate() call.
    /// MUST return None if no simulation has completed since the last load_garment().
    /// Used for same-backend idempotency only — MUST NOT be used for cross-backend
    /// promotion equivalence (see ARCH-031).
    fn last_content_hash(&self) -> Option<[u8; 32]>;

    /// Human-readable solver version string recorded in tailor_simulation_runs.solver_version.
    fn solver_version(&self) -> &str;
}
```

**ARCH-006.** `SimRunParams`, `MaterialFrameParams`, `SolverResult`, `SolverMesh`,
`FabricMaterial`, and `ClothSolverError` MUST be defined in `tailor-solver/src/` and
re-exported from `tailor-solver/src/lib.rs`. They are the only types that cross the crate
boundary.

**ARCH-007.** `SolverMesh` (canonical name; supersedes `SolverMeshV1` from research topic `03`)
MUST be defined in `tailor-solver/src/mesh.rs` and MUST be producible from a `GarmentSpec`
(defined in `tailor-solver/src/spec.rs`) via a `GarmentSpec::to_solver_mesh()` method or
equivalent triangulation pipeline. `GarmentSpec` is the authority garment type
(see `T-CONTRACTS` `[T-CONTRACTS.garment-spec]`); `SolverMesh` is its physics-runtime
derivative.

**ARCH-008.** `SolverResult` MUST contain at minimum:

```rust
// tailor-solver/src/lib.rs (or src/types.rs)
pub struct SolverResult {
    /// Final particle positions, flat [f32; N*3], metres or centimetres per GarmentSpec units.
    pub positions: Vec<f32>,
    pub normals:   Vec<f32>,
    pub uvs:       Vec<f32>,
    pub indices:   Vec<u32>,
    /// SHA-256 of positions bytes. Same-backend idempotency only; see ARCH-031.
    pub content_hash: [u8; 32],
    pub n_frames:     u32,
    pub gpu_mem_peak: u64,
}
```

**ARCH-009.** `ClothSolverError` MUST be a `thiserror`-derived enum. It MUST include at least
the variants `NotLoaded`, `NoAdapter` (no suitable GPU adapter found), `GpuError(String)`,
`MeshInvalid(String)`, and `Timeout`. The kernel-side `TailorEngineError` wraps
`ClothSolverError` but MUST NOT re-export it directly to callers outside `solver_binding.rs`.

---

##### <N>.<i>.3 tailor-solver Crate: Internal Module Layout

**ARCH-010.** The `tailor-solver` crate MUST follow this module layout. Additions are
permitted; removal of required modules is not.

```text
tailor-solver/
  Cargo.toml               -- [package] name = "tailor-solver"; no handshake_core dep
  build.rs                 -- wgsl_to_wgpu codegen: generates Rust bind-group structs from
                              WGSL via naga; eliminates runtime bind-group alignment bugs
  src/
    lib.rs                 -- pub trait ClothSolver; pub use of all public types
    spec.rs                -- GarmentSpec + all sub-types (canonical garment type per T-CONTRACTS)
    mesh.rs                -- SolverMesh, SeamConstraintRecord; GarmentSpec -> mesh triangulation
    solver.rs              -- ClothSolverGpu: wgpu Device/Queue, pipeline init, substep dispatch
    solver_cpu.rs          -- ClothSolverCpu: CPU fallback; required for tests and no-GPU CI
    constraints.rs         -- constraint graph build + greedy graph coloring; color partition
                              ranges stored here; coloring computed once at load_garment()
    material.rs            -- FabricMaterial, GpuSimParams, MaterialFrameParams;
                              normalized FabricProperties -> raw XPBD compliance mapping
    body/
      proxy.rs             -- ClothBodyProxy, CollisionCapsule, CollisionSphere (mm);
                              GpuCapsule/GpuSphere GPU upload types (bytemuck::Pod)
    self_collision.rs      -- SpatialHash; neighbor list management; broad phase
    types.rs               -- GpuParticle, GpuStretchConstraint, GpuBendConstraint,
                              GpuSeamConstraint (bytemuck::Pod + bytemuck::Zeroable)
    compare.rs             -- MeshComparator: tolerance-based promotion equivalence (ARCH-031)
    error.rs               -- ClothSolverError (thiserror)
  shaders/
    predict.wgsl
    stretch.wgsl
    bend.wgsl
    seam.wgsl
    self_collide.wgsl
    body_collide.wgsl
    velocity.wgsl
    hash_build.wgsl
  examples/
    bevy_testbed.rs        -- Bevy 0.18 throwaway viewport for visual solver testing ONLY;
                              MUST NOT be a library dependency; gated behind [[example]]
  tests/
    determinism.rs         -- per-backend determinism integration test (required; see ARCH-030)
    constraint_correctness.rs
```

**ARCH-011.** The `tailor-solver` crate MUST compile with the following mandatory dependencies
and no others in `[dependencies]` without explicit justification in the work packet:

```toml
[dependencies]
wgpu        = "29"
bytemuck    = { version = "1", features = ["derive"] }
encase      = { version = "0.8", features = ["glam"] }
glam        = "0.29"
parry3d     = "0.17"
thiserror   = "2"
tracing     = "0.1"
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
schemars    = { version = "0.8", features = ["derive"] }
async-trait = "0.1"

[build-dependencies]
wgsl_to_wgpu = "0.15"

[features]
cuda = ["cubecl"]                  # optional CubeCL fast path; NOT required for MVP

[dev-dependencies]
# test/example-only:
bevy        = { version = "0.18", optional = true, features = ["bevy_render"] }
```

`schemars` is required because `GarmentSpec` in `spec.rs` derives `JsonSchema` so the MCP
`inputSchema` is auto-generated (see `T-CONTRACTS` `[T-CONTRACTS.garment-spec]`). Bevy
MUST be `dev-dependencies` only and MUST NOT appear in `[dependencies]`.

**ARCH-012.** WGSL shaders MUST be compiled into the binary at build time via
`wgpu::include_wgsl!()` or `wgsl_to_wgpu` codegen. Shaders MUST NOT be read from disk at
runtime. This ensures the solver binary is self-contained and portable.

---

##### <N>.<i>.4 handshake_core::tailor Module: Internal Layout

**ARCH-013.** `handshake_core::tailor` MUST follow the `src/atelier/` creative-module pattern
exactly. It MUST be declared as `pub mod tailor;` in `src/lib.rs` gated behind the
`runtime-full` feature. The module layout MUST be:

```text
src/tailor/
  mod.rs              -- TailorEngineError, TailorEngineResult<T>; re-exports
  event_family.rs     -- all tailor.* event_family string constants (ARCH-017)
  schemas.rs          -- all hsk.tailor.* schema-id constants (ARCH-018)
  garment.rs          -- garment authority row CRUD; status transitions
  solver_binding.rs   -- TailorSandboxAdapter (SandboxAdapter impl); THE ONLY file that
                         imports tailor-solver crate types
  simulation.rs       -- SimulationRun row management; artifact bundle assembly
  material.rs         -- tailor_material_presets CRUD; preset->compliance mapping invocation
  validation.rs       -- TailorValidationDescriptor wrapping KB003 ValidationDescriptor;
                         full ~35-check catalog (T-CONTRACTS [T-CONTRACTS.validation])
  crdt_bridge.rs      -- garment-specific CRDT document helpers (panel CRDT sub-tree)
  avatar.rs           -- tailor_avatars + tailor_body_proxies CRUD
  wardrobe.rs         -- tailor_wardrobe CRUD
  model_adapter.rs    -- TailorModelAdapter (ModelAdapter impl; LLM garment authoring)
  storage_glue.rs     -- all tailor_* sqlx queries using PgPool directly (mirror of
                         storage/kb003_storage.rs pattern)
  api.rs              -- Axum Router with tailor/* HTTP endpoints
```

**ARCH-014.** `src/tailor/mod.rs` MUST declare:

```rust
// src/tailor/mod.rs

pub type TailorEngineResult<T> = Result<T, TailorEngineError>;

#[derive(Debug, thiserror::Error)]
pub enum TailorEngineError {
    #[error("storage: {0}")]
    Storage(#[from] sqlx::Error),
    #[error("solver: {0}")]
    Solver(#[from] tailor_solver::ClothSolverError),
    #[error("validation: {0}")]
    Validation(String),
    #[error("promotion rejected: {0}")]
    PromotionRejected(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("event ledger: {0}")]
    EventLedger(String),
    #[error("forbidden storage mode")]
    ForbiddenStorage,
    #[error("garment spec: {0}")]
    GarmentSpec(String),
}
```

**ARCH-015.** Every Tailor authority write function in `storage_glue.rs` MUST call
`guard_authority_write(AuthorityMode::PostgresPrimary)` as its first statement, identical to
the pattern in `storage/kb003_storage.rs`. SQLite is forbidden as a Tailor authority backend.
Failure to call the tripwire is a correctness defect, not a style issue.

**ARCH-016.** `handshake_core::tailor` receives `AppState` (or `AppState.postgres_pool:
sqlx::PgPool`) by reference or clone at construction time. It MUST NOT initialize a separate
database pool. It MUST consume `AppState.llm_client: Arc<dyn LlmClient>` for all LLM calls
(per HSK-TRAIT-004). It MUST NOT implement `LlmClient` or `ModelRuntime`.

---

##### <N>.<i>.5 Canonical Naming: Event Variants, Wire Strings, Schema IDs, Event-Family Constants

All names in this section are authoritative per `T-CONTRACTS`. Implementations MUST use these
exact identifiers.

**ARCH-017.** `event_family.rs` MUST declare the following constants and no others for MVP scope.
Additional families may be added per the `T-CONTRACTS` `[T-CONTRACTS.event-types]` list for
non-MVP features.

```rust
// src/tailor/event_family.rs
pub const TAILOR_GARMENT:    &str = "tailor.garment";
pub const TAILOR_SIMULATION: &str = "tailor.simulation";
pub const TAILOR_PANEL_CRDT: &str = "tailor.panel.crdt";
pub const TAILOR_MATERIAL:   &str = "tailor.material";
pub const TAILOR_AVATAR:     &str = "tailor.avatar";
pub const TAILOR_BODY_PROXY: &str = "tailor.body_proxy";
pub const TAILOR_REFIT:      &str = "tailor.refit";
pub const TAILOR_TRIM:       &str = "tailor.trim";
pub const TAILOR_UV:         &str = "tailor.uv";
pub const TAILOR_TEXTURE:    &str = "tailor.texture";
pub const TAILOR_ANIMATION:  &str = "tailor.animation";
pub const TAILOR_WARDROBE:   &str = "tailor.wardrobe";
pub const TAILOR_EXPORT:     &str = "tailor.export";
```

**ARCH-018.** `schemas.rs` MUST declare the following schema-ID constants. The namespace is
`hsk.tailor.*` for all Tailor-domain authority records. The single permitted `hsk.cloth.*`
exception covers the two solver-crate-internal physics payloads that never become authority rows.

```rust
// src/tailor/schemas.rs
pub const SCHEMA_TAILOR_GARMENT_SPEC_V1:    &str = "hsk.tailor.garment_spec@1";
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

// Allowed hsk.cloth.* exception — solver-crate-internal physics payloads only:
pub const SCHEMA_CLOTH_SOLVER_REQUEST_V1:   &str = "hsk.cloth.solver_request@1";
pub const SCHEMA_CLOTH_SOLVER_RESULT_V1:    &str = "hsk.cloth.solver_result@1";
```

Schema IDs from research topics `09`, `10`, `13`, and `15` that use the `hsk.cloth.*` namespace
for Tailor-domain authority records (e.g. `hsk.cloth.garment_draft@1`) are superseded and MUST
NOT appear in implementation files.

**ARCH-019.** The following `KernelEventType` variants MUST be added to the enum in
`kernel/mod.rs` and MUST be registered in `required_first_slice_events()`. Variant names are
`Tailor*` PascalCase; wire strings produced by `as_str()` are `TAILOR_*` SCREAMING_SNAKE_CASE.
Superseded names from research topics are listed for reconciliation; they MUST NOT be used.

```rust
// Additions to KernelEventType (kernel/mod.rs):

// Garment lifecycle
TailorGarmentDraftProposed,       // "TAILOR_GARMENT_DRAFT_PROPOSED"
TailorGarmentDraftUpdated,        // "TAILOR_GARMENT_DRAFT_UPDATED"
TailorGarmentValidationRecorded,  // "TAILOR_GARMENT_VALIDATION_RECORDED"
                                  //   supersedes: TailorGarmentValidated (01/03/04),
                                  //               TailorPatternValidated (01)
TailorGarmentPromoted,            // "TAILOR_GARMENT_PROMOTED"
TailorGarmentPromotionRejected,   // "TAILOR_GARMENT_PROMOTION_REJECTED"

// Simulation run lifecycle
TailorSimRunRequested,            // "TAILOR_SIM_RUN_REQUESTED"
TailorSimRunStarted,              // "TAILOR_SIM_RUN_STARTED"
TailorSimRunCompleted,            // "TAILOR_SIM_RUN_COMPLETED"
TailorSimRunRejected,             // "TAILOR_SIM_RUN_REJECTED"

// CRDT collaborative editing
TailorPanelCrdtUpdateRecorded,    // "TAILOR_PANEL_CRDT_UPDATE_RECORDED"
                                  //   supersedes: TailorGarmentCrdtUpdateRecorded (03/04),
                                  //               TailorCrdtUpdateRecorded (09)
TailorPanelCrdtSnapshotRecorded,  // "TAILOR_PANEL_CRDT_SNAPSHOT_RECORDED"
TailorPanelAiEditProposalRecorded,// "TAILOR_PANEL_AI_EDIT_PROPOSAL_RECORDED"
TailorPanelAiEditProposalDecided, // "TAILOR_PANEL_AI_EDIT_PROPOSAL_DECIDED"
TailorCrdtConflictDetected,       // "TAILOR_CRDT_CONFLICT_DETECTED"

// Material / fabric presets
TailorMaterialPresetRecorded,     // "TAILOR_MATERIAL_PRESET_RECORDED"
                                  //   supersedes: TailorMaterialLibraryUpdated (01/03)
TailorMaterialPresetUpdated,      // "TAILOR_MATERIAL_PRESET_UPDATED"
TailorMaterialPresetRejected,     // "TAILOR_MATERIAL_PRESET_REJECTED"
TailorGarmentMaterialAssigned,    // "TAILOR_GARMENT_MATERIAL_ASSIGNED"

// Avatar / body proxy
TailorAvatarCreated,              // "TAILOR_AVATAR_CREATED"
TailorAvatarMeasurementsExtracted,// "TAILOR_AVATAR_MEASUREMENTS_EXTRACTED"
                                  //   supersedes: BodyProxyMeasurementsExtracted (07, missing prefix)
TailorBodyProxyCreated,           // "TAILOR_BODY_PROXY_CREATED"
                                  //   supersedes: BodyProxyCreated (07, missing prefix)
TailorBodyProxyUpdated,           // "TAILOR_BODY_PROXY_UPDATED"

// Wardrobe grouping
TailorWardrobeCreated,            // "TAILOR_WARDROBE_CREATED"
TailorWardrobeGarmentAdded,       // "TAILOR_WARDROBE_GARMENT_ADDED"
TailorWardrobeGarmentRemoved,     // "TAILOR_WARDROBE_GARMENT_REMOVED"
```

Each variant MUST be registered in `required_first_slice_events()` immediately after being
added to the enum. A variant present in the enum but absent from `required_first_slice_events()`
is a correctness defect.

---

##### <N>.<i>.6 GarmentSpec: Canonical Type Location and Rules

**ARCH-020.** The canonical garment type is `GarmentSpec`. It MUST be defined in
`tailor-solver/src/spec.rs`. It MUST derive `serde::Serialize`, `serde::Deserialize`, and
`schemars::JsonSchema`. Its full field definition is specified in `T-CONTRACTS`
`[T-CONTRACTS.garment-spec]` and is reproduced there verbatim; the spec sub-section is
non-normative for field details — `T-CONTRACTS` governs.

**ARCH-021.** `GarmentSpec` MUST carry `schema_id: String` with value
`"hsk.tailor.garment_spec@1"` (from `SCHEMA_TAILOR_GARMENT_SPEC_V1`). Status, lifecycle
timestamps, and promotion metadata MUST NOT be fields on `GarmentSpec`; they live on the
`tailor_garments` Postgres row (`status` column with CHECK domain `draft | sandbox_pending |
simulated | validated | promoted | rejected | archived`).

**ARCH-022.** All length fields in `GarmentSpec` MUST use centimetres and MUST carry the `_cm`
suffix (e.g. `vertices_cm`, `depth_cm`, `interval_cm`, `translation_cm`). Normalized `[0.0,
1.0]` vertex coordinates are rejected for the authority spec; they MUST NOT appear in
`GarmentSpec` or `PanelSpec`. The only permitted non-unit-scaled fabric fields are the two
physically LLM-legible exceptions: `density_g_m2` (grams per square metre) and
`collision_thickness_mm` (millimetres), both of which carry explicit unit suffixes.

**ARCH-023.** The canonical seam gather field MUST be named `gather_ratio: f32`
(defined as `from_length / to_length`). The name `ratio` from research topics `10` and `15`
is superseded. Valid range is `(0.0, 20.0]`; the `GATHER_RATIO_RANGE` validation check
(ARCH-033) enforces this.

**ARCH-024.** `FabricProperties` MUST use normalized `[0.0, 1.0]` values for all dimensionless
parameters (the LLM-facing surface). The non-linear mapping from normalized values to raw
XPBD compliance MUST be performed by the preset/decoder layer in `tailor-solver/src/material.rs`
at `SolverMesh` build time and MUST NOT be stored twice in any authority row.

**ARCH-025.** Edge curves in `PanelSpec` MUST use the typed `EdgeShape` enum
(`Straight | Quadratic { control_cm } | Cubic { control_a_cm, control_b_cm } | Arc { curvature }`).
The string-keyed `curve_type: "bezier"` form from research topic `10` is superseded and MUST NOT
appear in any implementation.

---

##### <N>.<i>.7 TailorSandboxAdapter: Solver Bridge

**ARCH-026.** `src/tailor/solver_binding.rs` MUST define `TailorSandboxAdapter` implementing
the kernel's `SandboxAdapter` trait. `TailorSandboxAdapter` MUST hold an
`Arc<dyn ClothSolver>` (or `Arc<Mutex<dyn ClothSolver>>`) to call the physics crate. It MUST
NOT hold a `ClothSolverGpu` directly, so that the CPU fallback can be injected for tests.

**ARCH-027.** `TailorSandboxAdapter::run()` is synchronous in the `SandboxAdapter` trait
contract (per `T-CODEBASE` `[T-CODEBASE.sandbox]`). Where `ClothSolver::simulate()` is async,
`TailorSandboxAdapter::run()` MUST bridge to the async runtime via
`tokio::runtime::Handle::current().block_on(async { ... })`. This impedance is a known design
consequence of the existing synchronous `SandboxAdapter` trait; the implementation MUST
document this in a code comment and note it as a candidate for trait upgrade in the work packet
tracker.

**ARCH-028.** `TailorSandboxAdapter::run()` MUST call `self.pre_check(run, policy, ...)` as
its first action, passing the required `SandboxCapability` set. The cloth solver requires
`LocalFilesystemRead` and `LocalFilesystemWrite` (for mesh artifact staging). It MUST NOT
request `NetworkAccess`. Any policy denial MUST be returned as `AdapterRunOutcome::Denied`
without invoking the solver.

**ARCH-029.** On successful completion, `TailorSandboxAdapter::run()` MUST return
`AdapterRunOutcome::Completed { artifact_refs }` where `artifact_refs` contains the paths to
the simulated vertex buffer, normal buffer, UV buffer, and index buffer files written to the
sandbox workspace scratch path. Paths MUST be relative to the sandbox workspace root, not
absolute machine-local paths.

---

##### <N>.<i>.8 Determinism and Promotion Equivalence

**ARCH-030.** Per-backend determinism is required and MUST be tested. The `tests/determinism.rs`
integration test in `tailor-solver` MUST verify that calling `simulate()` twice on the same
`ClothSolverGpu` instance with identical inputs produces `SolverResult` values where
`content_hash` is identical. This test MUST pass in CI (it runs on whatever GPU backend is
available in the CI environment).

**ARCH-031.** Cross-backend promotion equivalence MUST use `MeshComparator`, not `content_hash`
comparison. `MeshComparator` MUST be defined in `tailor-solver/src/compare.rs` and MUST implement
the following contract from `T-CONTRACTS` `[T-CONTRACTS.determinism]`:

```rust
// tailor-solver/src/compare.rs

pub struct MeshComparatorResult {
    pub equivalent: bool,
    pub max_vertex_deviation_mm: f32,
    pub mean_vertex_deviation_mm: f32,
    /// All topology invariants that did not match (empty if all match).
    pub topology_mismatches: Vec<String>,
}

/// Canonical promotion equivalence check.
/// Primary: per-vertex Euclidean deviation <= epsilon_mm for all vertices,
///          in canonical vertex order (topology-derived, stored at garment load time).
/// Secondary (exact): vertex_count, triangle_count, seam_edge_pair_count, panel_count.
/// Verdict: equivalent iff all secondary invariants match AND max deviation <= epsilon_mm.
pub fn compare(
    a: &SolverResult,
    b: &SolverResult,
    epsilon_mm: f32,
) -> MeshComparatorResult { ... }
```

Default `epsilon_mm` is `0.1` (0.1 mm). The `PromotionGate` validation step that re-runs the
simulation for equivalence checking MUST call `MeshComparator::compare(a, b, epsilon_mm)`.
It MUST NOT compare `content_hash` values for cross-backend or cross-driver promotion decisions.

**ARCH-032.** For animated simulation runs (runs where `MaterialFrameParams` are updated per
frame), `MeshComparator::compare()` MUST additionally accept a shape-envelope mode: per-frame
bounding box within `bbox_epsilon_mm` (default `1.0`) plus `SEAMS_CLOSED` check passing, as
the promotion equivalence basis. This is required because wind turbulence uses functions whose
cross-vendor float precision differs and exact per-vertex reproduction is not achievable
cross-backend for animated runs.

---

##### <N>.<i>.9 Validation Descriptor

**ARCH-033.** `src/tailor/validation.rs` MUST define `TailorValidationDescriptor` wrapping the
KB003 `ValidationDescriptor`. It MUST register all checks from the `T-CONTRACTS`
`[T-CONTRACTS.validation]` catalog relevant to the garment's active feature set (trims,
multi-layer, refit, material preset). The check catalog contains approximately 35 checks
across fast, mesh, post-simulation, multi-layer, trim, material-preset, and refit stages.
Two severities only: `Blocking` (any failure prevents promotion) and `Advisory`
(recorded and visible; blocks only when `treat_advisory_as_blocking = true` in
`PromotionGateInputs`).

Each check MUST carry the canonical `code` string from `T-CONTRACTS` (e.g. `PANEL_CLOSURE`,
`GATHER_RATIO_RANGE`, `SEAMS_CLOSED`, `NO_INTERPENETRATION`) so the model can pattern-match
the code in `ValidationFinding` and self-correct the `GarmentSpec`.

`ValidationReport::aggregate_blocks_promotion()` (existing kernel method) MUST be used as the
sole promotion decision driver. Tailor MUST NOT implement a parallel promotion logic path.

---

##### <N>.<i>.10 Migration Naming

**ARCH-034.** All Tailor Postgres migrations MUST use the dated convention:

```text
migrations/YYYY_MM_DD_tailor_<topic>.sql
migrations/YYYY_MM_DD_tailor_<topic>.down.sql   (required reverse pair)
```

The date is the authoring date at implementation time. Numbered `0NNN_*` migrations are
forbidden for Tailor because the numbered integer namespace is contested by parallel work
packets; `0334_loom_canvas_boards.sql` already occupies position 0334
(per `T-CONTRACTS` `FACT-2`). Every forward migration MUST ship a `.down.sql` reverse pair
(per `T-CONTRACTS` `FACT-3`, following the `2026_05_18_fems_pinned` precedent).

Suggested per-topic migration files (dated at authoring):

```text
YYYY_MM_DD_tailor_garments.sql
YYYY_MM_DD_tailor_material_presets.sql
YYYY_MM_DD_tailor_avatars.sql
YYYY_MM_DD_tailor_body_proxies.sql
YYYY_MM_DD_tailor_simulation_runs.sql
YYYY_MM_DD_tailor_refit_runs.sql
YYYY_MM_DD_tailor_trims.sql
YYYY_MM_DD_tailor_texture_tables.sql
YYYY_MM_DD_tailor_wardrobe.sql
YYYY_MM_DD_tailor_garments_animation_col.sql    -- ALTER TABLE ADD COLUMN animation_json
```

---

##### <N>.<i>.11 Table Primary Keys and Id Prefixes

**ARCH-035.** All Tailor Postgres tables MUST use `TEXT PRIMARY KEY` with prefixed string IDs
(per `T-CONTRACTS` `FACT-4`). The `UUID PRIMARY KEY DEFAULT gen_random_uuid()` form is
off-convention and MUST NOT be used. Canonical prefix assignments:

| Table | PK column | Id prefix |
|---|---|---|
| `tailor_garments` | `garment_id` | `GAR-` |
| `tailor_garment_crdt_docs` | composite: `(garment_id, crdt_document_id)` | `CRDT-GAR-{garment_id}` |
| `tailor_material_presets` | `preset_id` | `MAT-` |
| `tailor_avatars` | `avatar_id` | `AVT-` |
| `tailor_body_proxies` | `body_proxy_id` | `BPX-` |
| `tailor_simulation_runs` | `sim_run_id` | `SIM-` |
| `tailor_refit_runs` | `refit_run_id` | `RFT-` |
| `tailor_trims` | `trim_id` | `TRIM-` |
| `tailor_trim_placements` | `placement_id` | `PLAC-` |
| `tailor_zippers` | `zipper_id` | `ZIP-` |
| `tailor_lacings` | `lacing_id` | `LACE-` |
| `tailor_uv_islands` | `island_id` | `UVI-` |
| `tailor_pbr_materials` | `material_id` | `PBR-` |
| `tailor_graphic_layers` | `layer_id` | `GLYR-` |
| `tailor_material_assignments` | `assignment_id` | `ASGN-` |
| `tailor_wardrobe` | `wardrobe_id` | `WRD-` |

The `CSIM-` prefix from research topic `10` for simulation run IDs is superseded; `SIM-` is
canonical.

**ARCH-036.** Every Tailor authority table row MUST carry `event_ledger_event_id TEXT NOT NULL`
as a foreign-key reference to `kernel_event_ledger.event_id`. A row MUST NOT be inserted
without a prior `NewKernelEvent` emission whose returned `event_id` populates this column.

---

##### <N>.<i>.12 Extension Points in Existing Files

**ARCH-037.** The following existing `handshake_core` files MUST be modified to integrate the
Tailor module. No other existing file MUST be modified without explicit justification in the
work packet.

| File | Required addition |
|---|---|
| `src/kernel/mod.rs` | Add all `KernelEventType` variants from ARCH-019; register each in `required_first_slice_events()` |
| `src/lib.rs` | Add `pub mod tailor;` gated behind `runtime-full` feature |
| `src/api/mod.rs` | Add `pub mod tailor;` import; merge `tailor::routes(state.clone())` in `routes()` |
| `src/storage/mod.rs` | Add new `Database` trait methods: `list_garments`, `get_garment`, `save_garment`, `update_garment_status` (at minimum) |
| `src/storage/postgres.rs` | Concrete `impl Database` for new methods, following the EventLedger INSERT + `ON CONFLICT (idempotency_key) DO NOTHING` pattern from `storage/postgres.rs` ~line 3454 |

**ARCH-038.** Business logic MUST NOT be placed in Tauri command wrappers
(`app/src-tauri/src/commands/`). Tauri commands for Tailor MUST be thin delegates that call
`AppState` methods or axum handler functions. All garment authoring, simulation, validation,
and promotion logic MUST reside in `handshake_core::tailor`.

---

##### <N>.<i>.13 Build Integration

**ARCH-039.** `tailor-solver` MUST be added to the Handshake Cargo workspace as a member or
referenced via `path =` in `handshake_core/Cargo.toml`. The crate MUST be gated behind a
`cloth-solver` Cargo feature in `handshake_core/Cargo.toml` so that CI builds that do not
exercise GPU code can skip `tailor-solver` compilation:

```toml
# handshake_core/Cargo.toml (relevant additions)
[features]
cloth-solver = ["dep:tailor-solver"]
runtime-full = ["cloth-solver", ...]

[dependencies.tailor-solver]
path = "../tailor-solver"   # or workspace member path
optional = true
```

`src/tailor/solver_binding.rs` MUST be compiled only when the `cloth-solver` feature is active
(`#[cfg(feature = "cloth-solver")]`). When the feature is inactive, `TailorSandboxAdapter` MUST
NOT exist, and any call site that would reference it MUST produce a compile error, not a silent
no-op.

**ARCH-040.** `CARGO_TARGET_DIR` for all Tailor crate builds MUST be
`../Handshake_Artifacts/handshake-cargo-target` (following the existing `justfile` convention).
This MUST NOT be changed or overridden in `tailor-solver/Cargo.toml`.

---

##### <N>.<i>.14 Required Tests

**ARCH-041.** The `tailor-solver` crate MUST include these test modules at a minimum:

- `tests/determinism.rs`: verifies that two `simulate()` calls with identical inputs on the
  same backend produce identical `content_hash` (ARCH-030).
- `tests/constraint_correctness.rs`: unit tests for each constraint type verifying that
  a single-constraint simulation converges to the expected rest length or angle within
  tolerance.
- A `ClothSolverCpu` integration test that exercises the full `ClothSolver` trait without a
  GPU device, confirming the fallback path is exercisable in CI.

**ARCH-042.** `handshake_core` MUST include a test that constructs a minimal `GarmentSpec`
(single rectangular panel, one seam, cotton preset), passes it through
`GarmentSpec::to_solver_mesh()`, verifies the resulting `SolverMesh` has the expected particle
count and constraint count, and confirms the `PANEL_CLOSURE` and `SEAM_EDGE_REF` fast
pre-solver validation checks pass. This test MUST NOT require a live Postgres connection or a
GPU device.

---

##### <N>.<i>.15 Constraints and Prohibitions Summary

For implementors, this is the non-exhaustive list of hard prohibitions derived from the above
requirements. Each maps to the governing rule.

| Prohibited action | Governing rule |
|---|---|
| Adding `sqlx`, `tauri`, or any `handshake_core` type to `tailor-solver/Cargo.toml` | ARCH-002 |
| Importing `tailor-solver` types in any file other than `solver_binding.rs` | ARCH-003 |
| Adding `wgpu` to `handshake_core/Cargo.toml` | ARCH-004 |
| Using `UUID PRIMARY KEY DEFAULT gen_random_uuid()` in any Tailor migration | ARCH-035 |
| Using numbered `0NNN_*` migration filenames for Tailor | ARCH-034 |
| Using `hsk.cloth.*` schema IDs for Tailor-domain authority records | ARCH-018 |
| Using superseded event variant names (`TailorGarmentValidated`, `TailorCrdtUpdateRecorded`, `CSIM-` prefix, `GARMENT_*` wire strings without `TAILOR_` prefix) | ARCH-019 |
| Using `gather_ratio` field name `ratio` | ARCH-023 |
| Using string `curve_type` for edge shapes instead of `EdgeShape` enum | ARCH-025 |
| Comparing `content_hash` for cross-backend promotion equivalence | ARCH-031 |
| Inserting a Tailor authority row without calling `guard_authority_write()` first | ARCH-015 |
| Emitting an event with a `Tailor*` variant not registered in `required_first_slice_events()` | ARCH-019 |
| Placing business logic in Tauri command wrappers | ARCH-038 |
| Bevy appearing in `[dependencies]` (not `[dev-dependencies]`) of `tailor-solver` | ARCH-011 |

---
file_id: tailor-spec-solver-core
section_id: solver
title: "XPBD Solver Core (WGSL/wgpu)"
status: draft
updated_at: "2026-06-17"
provenance_sources:
  - "cloth_engine_research/04-cloth-solver.md (T-CLOTH-SOLVER) — non-normative design rationale and OSS evidence"
  - "cloth_engine_research/16-contracts.md (T-CONTRACTS) — canonical authority for all names, types, and contracts cited below"
contracts_authority: "16-contracts.md [T-CONTRACTS]"
---

## 13.3 XPBD Solver Core (WGSL/wgpu)

---

##### <N>.<i>.1 Scope and Placement

This sub-section specifies the `tailor-solver` standalone Rust crate: the XPBD substepping
algorithm, constraint projection pipeline executed as WGSL compute shaders via wgpu, constraint
parallelism strategy (graph coloring / Jacobi), `SolverMode` variants including the
Chebyshev+Gauss-Seidel upgrade path, and per-backend determinism requirements.

**Out of scope here:** GarmentSpec authoring (see §<garment-authoring>), body-proxy collision
geometry (see §<collision>), fabric material preset mapping (see §<fabric-models>), and
sandbox/promotion EventLedger integration (see §<kernel-integration>).

**Canonical contract authority.** All type names, field names, schema IDs, event variants, table
names, and migration conventions cited in this sub-section are governed by T-CONTRACTS
(`16-contracts.md`). Where a name here differs from a name in the research source file
(`04-cloth-solver.md`), T-CONTRACTS wins.

---

##### <N>.<i>.2 Crate Identity and Isolation Contract

The physics solver MUST be implemented as a standalone Rust workspace member named
**`tailor-solver`**.

The `tailor-solver` crate MUST NOT import `handshake_core`, `tauri`, any PostgreSQL driver, or any
EventLedger primitive. It is UI-agnostic and kernel-agnostic. The sole coupling point to
`handshake_core` is `src/tailor/solver_binding.rs` (the `TailorSandboxAdapter`), which imports
`tailor-solver` — never the reverse.

Physics-internal types (`ClothSolver`, `ClothParticle`, `ClothConstraint`, `GpuParticle`,
`GpuSimParams`, `SolverMesh`, `MeshComparator`) MUST use the `Cloth*` / `Gpu*` naming convention
for physics-layer types. Domain-facing types (`GarmentSpec`, `SolverResult`, `SimRunParams`)
use `Garment*` / `Solver*` / `Sim*` naming. See T-CONTRACTS §[T-CONTRACTS.naming].

The `ClothSolver` public trait MUST be defined in `tailor-solver/src/lib.rs`. The primary
implementation is `ClothSolverGpu` (wgpu WGSL backend). A `ClothSolverCpu` fallback
implementation SHOULD be provided for environments without GPU access; it MUST implement the same
trait and produce results satisfying the MeshComparator tolerance (§<N>.<i>.7).

The crate's required dependencies are:

```toml
# tailor-solver/Cargo.toml  (canonical; all versions as of 2026-06-17)
[dependencies]
wgpu        = "29"
bytemuck    = { version = "1", features = ["derive"] }
encase      = { version = "0.8", features = ["glam"] }
glam        = "0.29"
parry3d     = "0.17"
thiserror   = "2"
tracing     = "0.1"
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
schemars    = { version = "0.8", features = ["derive"] }

[features]
multigrid = []          # gates SolverMode::Multigrid / MGPBD backend (§<N>.<i>.6)
cuda      = ["cubecl"]  # optional CubeCL fast path; never required for correctness

[build-dependencies]
wgsl_to_wgpu = "0.15"   # build.rs: compile-time WGSL→Rust bind-group structs
```

Bevy MUST NOT appear as a crate dependency. It is permitted only in `[[example]]` targets gated
behind an optional feature (e.g. `bevy-testbed`) so the solver lib compiles without it.

---

##### <N>.<i>.3 XPBD Algorithm: Normative Substep Loop

The solver MUST implement Extended Position-Based Dynamics (XPBD) as defined by Macklin, Müller,
and Chentanez (MIG 2016). The Lagrange multiplier form is mandatory: per-constraint `lambda`
state MUST be maintained across iterations within a substep and reset to zero at the start of
each substep. This makes constraint stiffness independent of substep size — a prerequisite for
stable keyframeable material parameters (§<N>.<i>.5).

The normative per-frame loop is:

```
for each frame (dt_frame):
    # 1. External forces and position prediction
    for each particle i:
        v[i]          += dt_sub * w[i] * f_ext[i]   # gravity + wind
        x_pred[i]      = x[i] + dt_sub * v[i]

    # 2. Substep loop
    for sub in 0..n_substeps:
        dt_sub = dt_frame / n_substeps
        alpha_tilde[c] = compliance[c] / (dt_sub * dt_sub)   # per constraint

        # Reset Lagrange multipliers at substep start
        for each constraint c:
            lambda[c] = 0.0

        # 3. Constraint solve (n_iters iterations per substep)
        for iter in 0..n_iters:
            for each constraint c (dispatched by color partition — see §<N>.<i>.4):
                C          = constraint_value(c, x_pred)
                grad_C     = constraint_gradient(c, x_pred)
                w_sum      = Σ w[i] * |grad_C[i]|²  for i in c.particles
                delta_lam  = -(C + alpha_tilde[c] * lambda[c]) / (w_sum + alpha_tilde[c])
                lambda[c] += delta_lam
                x_pred[i] += w[i] * grad_C[i] * delta_lam   for each i in c.particles

        # 4. Collision response (after constraint solve, each substep)
        body_collision_response(x_pred)     # cloth vs avatar capsule/sphere proxies
        self_collision_response(x_pred)     # cloth vs cloth (§<N>.<i>.8)

        # 5. Velocity update
        for each particle i:
            v[i] = (x_pred[i] - x[i]) / dt_sub
            x[i] = x_pred[i]

    # 6. Velocity damping (once per frame, after all substeps)
    apply_velocity_damping(v, damping_coeff)
```

**Compliance convention.** `compliance = 0.0` produces a rigid (inextensible) constraint.
`compliance > 0.0` is elastic; larger values yield softer behavior. The solver MUST accept
compliance values in the range used by the XPBD anisotropic fabric model: `1e-9` (near-rigid
denim/leather) to `1e-3` (very soft chiffon/spandex). The mapping from normalized
`FabricProperties` fields (the LLM-facing `[0,1]` surface in `GarmentSpec`) to raw compliance
is owned exclusively by the fabric preset/decoder layer (§<fabric-models>) and MUST NOT be
duplicated in this crate.

**Substep vs. iteration budget.** The solver MUST prefer increasing `n_substeps` over increasing
`n_iters` as the primary convergence lever, consistent with Macklin et al. (2019) "Small Steps
in Physics Simulation." Default values for `SimMode::Fitting` SHOULD be `n_substeps = 10`,
`n_iters = 5`. Default values for `SimMode::Animation` SHOULD be `n_substeps = 4`, `n_iters = 3`.
These defaults MUST be overridable via `SimRunParams` (§<N>.<i>.5).

---

##### <N>.<i>.4 Constraint Types

###### <N>.<i>.4.1 Anisotropic Stretch and Shear

The solver MUST implement separate compliance values for weft (U grain), warp (V grain), and
shear (UV diagonal) stretch directions. Isotropic distance-only constraints MUST NOT be the sole
stretch representation: they cannot reproduce the weft/warp stiffness split that is a core
differentiator from MD.

The anisotropic formulation MUST use per-triangle Green strain components in Voigt notation.
For a mesh triangle with rest-frame edge vectors `(du, dv)` and deformed vectors `(eu, ev)`:

```
C_u(x)  = |eu| - |du|             # weft stretch
C_v(x)  = |ev| - |dv|             # warp stretch
C_uv(x) = eu·ev / (|eu|·|ev|)    # shear angle
```

Each component MUST carry its own GPU-resident compliance value
(`compliance_u`, `compliance_v`, stored in `GpuStretchConstraint`). The anisotropic blend within
a single edge constraint is achieved by interpolating using `grain_cos` (cosine of the angle
between the edge and the U grain axis, stored in `GpuStretchConstraint.grain_cos`).

The mathematical reference for this formulation is: "XPBD Simulation of Constitutive Materials
with Exponential Strain Tensor" (ACM MIG 2025, doi:10.1145/3769047.3769050).

###### <N>.<i>.4.2 Dihedral Bending (Weft/Warp, Buckling)

Bending constraints MUST be dihedral (angular), operating on four particles forming two adjacent
triangles sharing an interior edge:

```
C_bend(x1,x2,x3,x4) = acos( n1·n2 / (|n1|·|n2|) ) - theta_rest
```

where `n1`, `n2` are face normals and `theta_rest` is the rest dihedral angle.

Separate compliance values for bending along edges aligned with the U grain
(`GpuBendConstraint.compliance_u`) and V grain (`GpuBendConstraint.compliance_v`) MUST be stored
and applied based on `GpuBendConstraint.edge_grain`.

The **buckling ratio** (MD Buckling Ratio/Stiffness equivalent) MUST be implemented as a
non-linear compliance ramp: at dihedral angles below `buckle_ratio` threshold use
`compliance_u` / `compliance_v`; at angles exceeding the threshold switch to
`GpuBendConstraint.buckle_alpha` (the stiff-side compliance). Both fields MUST be stored in
`GpuBendConstraint` and uploaded to the GPU per-garment-load.

###### <N>.<i>.4.3 Seam / Sewing Distance Constraints

Each seam MUST be realized as a set of edge-pair distance constraints. The canonical
`SeamSpec.gather_ratio` field (T-CONTRACTS §[T-CONTRACTS.garment-spec], range `(0.0, 20.0]`) MUST
control the rest-length scaling: `rest_length_gpu = edge_rest_length / gather_ratio`. The GPU
struct is `GpuSeamConstraint` with field `ratio: f32` derived from `gather_ratio`.

M:N gathering MUST be implemented by resampling both seam edges to equal vertex count `N` at
`SolverMesh` generation time and emitting `N` point constraints — storing the scalar
`gather_ratio` is sufficient in the authority spec; no M:N integer pair is stored.

###### <N>.<i>.4.4 Stitch / Tack Point Constraints

Point constraints between trim attachment tacks and cloth particles MUST be supported with
per-tack compliance (`tack_compliance: f32`) uploadable via `MaterialFrameParams` (§<N>.<i>.5).
This enables animated tack compliance (MD 2025.2 animated stitching/unstitching parity).

###### <N>.<i>.4.5 Volume / Pressure Constraint

A soft global volume constraint MUST be supported for inflatable garments. The target volume
`pressure_target: f32` MUST be part of `GpuSimParams` so it is uploadable per-substep,
enabling keyframeable pressure (MD 2025.2 parity). Compliance for the volume constraint MUST be
large (soft); the default value SHOULD be `1e-3`.

---

##### <N>.<i>.5 Solver Parameters and Keyframeable State

The following canonical types are normative. They are defined in `tailor-solver/src/lib.rs` and
carry `serde` + `schemars::JsonSchema` derives. Schema IDs from T-CONTRACTS §[T-CONTRACTS.schema-ids]:
physics-internal payloads crossing the `ClothSolver` trait boundary use
`hsk.cloth.solver_request@1` and `hsk.cloth.solver_result@1`; the simulation receipt stored in
`tailor_simulation_runs` uses `hsk.tailor.simulation_receipt@1`.

```rust
// tailor-solver/src/lib.rs

/// Parameters fixed for the duration of one simulation run.
/// Schema id (internal): hsk.cloth.solver_request@1
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct SimRunParams {
    /// Substeps per animation frame. Primary convergence lever. Must be >= 1.
    pub n_substeps:  u32,
    /// Constraint solver iterations per substep. Must be >= 1.
    pub n_iters:     u32,
    /// Seconds per frame (e.g. 1.0/30.0 for 30 fps).
    pub dt_frame:    f32,
    /// Fitting = high accuracy (n_substeps=10, n_iters=5 default).
    /// Animation = stable real-time (n_substeps=4, n_iters=3 default).
    pub mode:        SolverMode,
    /// Deterministic seed for wind/perturbation noise. Must be stored in
    /// tailor_simulation_runs.seed so re-runs use the same seed.
    pub seed:        u64,
    /// MeshComparator tolerance for promotion equivalence (mm). Default: 0.1.
    pub epsilon_mm:  f32,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize,
         schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SolverMode {
    /// High-accuracy XPBD with Gauss-Seidel via graph-colored constraint partitions.
    Fitting,
    /// Stable real-time XPBD, same algorithm, lower substep/iter budget.
    Animation,
    /// Chebyshev-accelerated Gauss-Seidel (MGPBD-style upgrade).
    /// Requires the `multigrid` feature flag. Falls back to Fitting if the feature
    /// is absent. Intended for high-resolution (> 10k particles) or high-stiffness cloth
    /// where standard XPBD Gauss-Seidel stalls at 300+ iterations.
    ChebyshevGs,
}

/// Keyframeable material state uploaded to GpuSimParams once per substep (MD 2025.2 parity).
/// Enables parameter animation without re-uploading the full constraint buffer.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct MaterialFrameParams {
    /// 0.0 = soft (compliance-governed); 1.0 = rigid (near-zero compliance isotropic).
    pub solidify_blend:  f32,
    /// Inflation volume target. 0.0 = no inflation.
    pub pressure_target: f32,
    /// Weft (U) shrinkage rate per frame.
    pub shrink_u:        f32,
    /// Warp (V) shrinkage rate per frame.
    pub shrink_v:        f32,
    /// Tack/stitch point compliance (all tacks share one global value this frame).
    pub tack_compliance: f32,
}

/// Output of one ClothSolver::simulate call.
/// Schema id (internal): hsk.cloth.solver_result@1
#[derive(Debug, Clone)]
pub struct SolverResult {
    /// Final particle positions as flat [x0,y0,z0, x1,y1,z1, ...].
    pub positions:    Vec<f32>,
    /// Final vertex normals (same layout as positions).
    pub normals:      Vec<f32>,
    /// UV coordinates from input mesh (unchanged by simulation).
    pub uvs:          Vec<f32>,
    /// Triangle index buffer.
    pub indices:      Vec<u32>,
    /// SHA-256 of positions bytes.
    /// PURPOSE: same-machine idempotency and EventLedger fingerprint ONLY.
    /// MUST NOT be used as a cross-backend promotion equivalence check
    /// (see §<N>.<i>.7 and T-CONTRACTS §[T-CONTRACTS.determinism]).
    pub content_hash: [u8; 32],
    /// Number of frames simulated.
    pub n_frames:     u32,
    /// Peak GPU memory used in bytes.
    pub gpu_mem_peak: u64,
}
```

`GpuSimParams` (the GPU-resident UBO written at every substep) MUST include at minimum:
`dt_sub: f32`, `n_particles: u32`, `gravity: [f32; 3]`, `damping: f32`,
`collision_dist: f32`, `friction: f32`, `pressure_target: f32`, `solidify_blend: f32`,
`shrink_u: f32`, `shrink_v: f32`, `wind: [f32; 3]`, and a deterministic noise seed.
All fields that appear in `MaterialFrameParams` MUST map 1:1 to a field in `GpuSimParams` so
that `update_params` requires only a single uniform buffer write, not a constraint buffer rebuild.

---

##### <N>.<i>.6 SolverMode: Gauss-Seidel (Standard) and Chebyshev+GS (Upgrade)

**Standard path — `SolverMode::Fitting` and `SolverMode::Animation`.**
The solver MUST implement parallel Gauss-Seidel constraint projection via constraint graph
coloring (§<N>.<i>.4, §<N>.<i>.9). This is the required baseline. All constraint types defined
in §<N>.<i>.4 MUST be available in this mode.

**Upgrade path — `SolverMode::ChebyshevGs`.**
The solver SHOULD implement a Chebyshev-accelerated Gauss-Seidel solver as the `ChebyshevGs`
mode, gated by the `multigrid` Cargo feature. The design reference is MGPBD (Multigrid
Accelerated Global XPBD, arxiv:2505.13390, SIGGRAPH 2025): the Chebyshev smoother converges
high-resolution or high-stiffness cloth to `1e-4` accuracy where standard Gauss-Seidel stalls
at 300+ iterations. The XRTailor implementation (real-time garment simulation for XR) confirms
this improvement for garment-scale meshes.

Requirements for `ChebyshevGs`:
- The same `ClothSolver` trait, `GarmentMesh`, and `SolverResult` types MUST be used; the mode
  MUST NOT require a different garment upload format.
- The Chebyshev acceleration coefficient schedule MUST be stored in `SimRunParams`-derived state
  (not hard-coded); the implementation MAY add optional fields to `SimRunParams` for tuning when
  the `multigrid` feature is active.
- If the `multigrid` feature is absent and `SolverMode::ChebyshevGs` is requested, the solver
  MUST fall back to `SolverMode::Fitting` and MUST emit a `tracing::warn!` logging the fallback.
- `ChebyshevGs` MUST satisfy the same determinism requirements as the standard solver
  (§<N>.<i>.7): per-backend reproducibility, color-partition dispatch, seeded noise.

The `multigrid` feature MUST be disabled by default in `Cargo.toml`. It is an optional
performance enhancement, not a correctness dependency.

---

##### <N>.<i>.7 Per-Backend Determinism

**Requirement.** The solver MUST be deterministic **per-backend**: given the same GPU backend
(Vulkan, Metal, or DX12), the same driver version, the same `SimRunParams` (including `seed`),
and the same input `GarmentMesh`, two successive calls to `ClothSolver::simulate` on the same
machine MUST produce identical `SolverResult.positions` bytes and identical `content_hash`.

**Cross-backend determinism is NOT required and MUST NOT be asserted.** WGSL compiled via Naga
to SPIR-V (Vulkan), MSL (Metal), and HLSL (DX12) does not guarantee identical floating-point
sub-expression ordering across backends. The `content_hash` MUST NOT be compared across backends
for promotion equivalence.

**Promotion equivalence uses `MeshComparator`, not hash equality.** The `MeshComparator` type
MUST be implemented in `tailor-solver/src/compare.rs` as a pure function with no external
dependencies. Its canonical contract (from T-CONTRACTS §[T-CONTRACTS.determinism]):

```rust
// tailor-solver/src/compare.rs

pub struct MeshCompareResult {
    /// Max per-vertex Euclidean position deviation in mm.
    pub max_deviation_mm: f32,
    /// Mean per-vertex Euclidean position deviation in mm.
    pub mean_deviation_mm: f32,
    /// Secondary topology invariants (must all be true for equivalence).
    pub vertex_count_match:    bool,
    pub triangle_count_match:  bool,
    pub seam_edge_pair_match:  bool,
    pub panel_count_match:     bool,
    /// True iff all secondary invariants match AND max_deviation_mm <= epsilon_mm.
    pub equivalent:            bool,
}

/// Compare two SolverResult meshes for promotion equivalence.
/// Vertices MUST be in the canonical order determined by mesh topology +
/// constraint coloring (computed once at garment load; see §<N>.<i>.9).
///
/// epsilon_mm: position tolerance in millimetres. Default: 0.1 mm.
/// For animated runs with wind, use bbox_epsilon_mm instead (default 1.0 mm).
pub fn compare(
    a: &SolverResult,
    b: &SolverResult,
    epsilon_mm: f32,
) -> MeshCompareResult {
    // Implementation: iterate vertex pairs in canonical order,
    // compute Euclidean distance, track max and mean.
    // Check secondary invariants: vertex_count, triangle_count, seam pair count,
    // panel count (derived from index buffer structure).
    // equivalent = secondary_all_match && max_deviation_mm <= epsilon_mm
    unimplemented!()
}

/// Shape-envelope comparison for animated runs with wind turbulence.
/// Accepts if per-frame bounding box deviates <= bbox_epsilon_mm AND seams are closed.
pub fn compare_envelope(
    a: &SolverResult,
    b: &SolverResult,
    bbox_epsilon_mm: f32,
) -> bool {
    unimplemented!()
}
```

The `ValidationRunner` in `handshake_core` (§<kernel-integration>) MUST call
`MeshComparator::compare` (or `compare_envelope` for animated runs) — NEVER `content_hash`
equality — when re-running a simulation to confirm promotion-gate reproducibility.

**Four concrete determinism requirements the implementation MUST satisfy:**

1. **Fixed budget.** `n_substeps` and `n_iters` MUST be fixed per run (stored in
   `tailor_simulation_runs`). Adaptive substep or adaptive iteration strategies MUST NOT be
   used in the promotion-eligible path.
2. **Stable constraint order.** Constraint graph coloring MUST be computed once at `load_garment`
   time from mesh topology and stored as CPU-side `stretch_colors: Vec<(u32, u32)>` and
   `bend_colors: Vec<(u32, u32)>` index ranges. The color partition order MUST be deterministic
   from mesh connectivity (not from random initialization). It MUST be stable across re-loads of
   the same garment.
3. **Seeded noise.** All stochastic inputs (wind turbulence, positional perturbation for
   degeneracy breaking) MUST be generated from the deterministic `SimRunParams.seed` value.
   The seed MUST be stored in `tailor_simulation_runs` and replayed identically on re-run.
4. **Race-free writes.** Color-partition GPU dispatch (§<N>.<i>.9) MUST ensure that within any
   single dispatch, no two workgroups write to the same particle. This eliminates write-race
   non-determinism. Atomic float operations MUST NOT be used for delta accumulation (WGSL does
   not support `atomicAdd<f32>`; see also wgpu issue #5329).

---

##### <N>.<i>.8 wgpu Backend and WGSL Compute Pipeline

###### <N>.<i>.8.1 Backend Coverage

The solver MUST compile and run correctly on all wgpu backends available on the Handshake
primary deployment target (Windows: Vulkan + DX12; macOS: Metal; Linux: Vulkan). WASM/WebGPU
MUST be considered a secondary target; no WASM-specific code path is required but the crate MUST
NOT use non-WASM-safe APIs in the main library (only in `[[example]]` targets).

The GPU adapter MUST be requested with `PowerPreference::HighPerformance`. If no adapter is
found, `ClothSolverGpu::new()` MUST return `ClothSolverError::NoAdapter` and the kernel MUST
fall back to `ClothSolverCpu`.

###### <N>.<i>.8.2 Shader Compilation

All WGSL shaders MUST be validated at compile time using `wgsl_to_wgpu` (version `0.15` in
`build.rs`). This generates typesafe Rust bind-group structs from WGSL layout declarations and
MUST catch bind-group alignment bugs before runtime. Shaders MUST be embedded via
`wgpu::include_wgsl!` so they are bundled with the binary; no runtime shader file loading
is permitted.

Workgroup size for all 1D physics compute dispatches MUST be **64 threads** (matching the GPU
warp/wave size on AMD and NVIDIA for coalesced memory access). The dispatch count for a buffer
of `N` elements MUST be `ceil(N / 64)`.

###### <N>.<i>.8.3 GPU Data Layout

All GPU-visible structs MUST derive `bytemuck::Pod` and `bytemuck::Zeroable`. Struct fields
MUST be aligned to `std430` (WGSL storage buffer) rules; use `encase` where padding is
non-trivial. The canonical GPU struct set is:

```rust
// tailor-solver/src/types.rs

/// One particle slot. Stride: 80 bytes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuParticle {
    pub position:      [f32; 4],   // xyz + w=unused (vec4 alignment)
    pub velocity:      [f32; 4],   // xyz + w=unused
    pub position_pred: [f32; 4],   // predicted position after external forces
    pub delta:         [f32; 4],   // accumulated position correction (Jacobi mode)
    pub normal:        [f32; 4],   // vertex normal xyz + w=unused
    pub inv_mass:      f32,        // 0.0 = pinned (infinite mass)
    pub uv:            [f32; 2],   // UV for grain direction lookup
    pub _pad:          f32,
}

/// Anisotropic stretch constraint (one per mesh edge). Stride: 32 bytes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuStretchConstraint {
    pub i0:           u32,
    pub i1:           u32,
    pub rest_length:  f32,         // centimetres (matches GarmentSpec units)
    pub compliance_u: f32,         // weft compliance (alpha_u)
    pub compliance_v: f32,         // warp compliance (alpha_v)
    pub grain_cos:    f32,         // cos(edge angle vs U grain) for aniso blend
    pub _pad:         [f32; 2],
}

/// Dihedral bending constraint (one per interior edge). Stride: 48 bytes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuBendConstraint {
    pub i0: u32, pub i1: u32, pub i2: u32, pub i3: u32,
    pub rest_angle:   f32,         // theta_rest (radians)
    pub compliance_u: f32,         // weft bend compliance
    pub compliance_v: f32,         // warp bend compliance
    pub edge_grain:   f32,         // grain direction flag for aniso selection
    pub buckle_ratio: f32,         // dihedral angle threshold for stiffening
    pub buckle_alpha: f32,         // stiff-side compliance
    pub _pad:         [f32; 2],
}

/// Seam distance constraint. Stride: 32 bytes.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSeamConstraint {
    pub i0:          u32,
    pub i1:          u32,
    pub rest_length: f32,          // centimetres
    pub ratio:       f32,          // derived from SeamSpec.gather_ratio
    pub compliance:  f32,
    pub _pad:        [f32; 3],
}

/// Per-substep uniform buffer (keyframeable state). One write per substep.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSimParams {
    pub dt_sub:          f32,
    pub n_particles:     u32,
    pub gravity:         [f32; 3],
    pub damping:         f32,
    pub collision_dist:  f32,      // millimetres (matches ClothBodyProxy units)
    pub friction:        f32,
    pub pressure_target: f32,      // keyframeable
    pub solidify_blend:  f32,      // keyframeable
    pub shrink_u:        f32,      // keyframeable
    pub shrink_v:        f32,      // keyframeable
    pub wind:            [f32; 3],
    pub noise_seed:      u32,      // deterministic seed fragment for this substep
    pub _pad:            f32,
}
```

Persistent Lagrange multiplier state MUST be stored in a separate GPU storage buffer
(`buf_lambdas`) with one `f32` slot per constraint. This buffer MUST be zeroed at the start of
each substep (via a zero-fill compute pass or `wgpu::Queue::write_buffer`).

###### <N>.<i>.8.4 Compute Pass Sequence Per Substep

The following passes MUST be dispatched in this order within each substep. No pass MUST be
reordered. All passes within one substep MUST be encoded into a single `wgpu::CommandEncoder`
and submitted as one `wgpu::CommandBuffer` to ensure GPU ordering:

| Pass | Shader file | Dispatch | Notes |
|---|---|---|---|
| 1. Predict | `predict.wgsl` | `ceil(N/64)` | External forces + position prediction + shrinkage |
| 2. Hash build | `hash_build.wgsl` | `ceil(N/64)` | Spatial hash for self-collision broad phase |
| 3. Stretch solve | `stretch.wgsl` | `ceil(C_s/64)` per color | Repeated once per color partition |
| 4. Bend solve | `bend.wgsl` | `ceil(C_b/64)` per color | Repeated once per color partition |
| 5. Seam solve | `seam.wgsl` | `ceil(C_seam/64)` | Single pass (seams are few; no coloring needed) |
| 6. Self-collision | `self_collide.wgsl` | `ceil(N/64)` | Spatial hash narrow phase |
| 7. Body collision | `body_collide.wgsl` | `ceil(N/64)` | Cloth vs avatar capsule/sphere proxies |
| 8. Velocity update | `velocity.wgsl` | `ceil(N/64)` | v = (x_pred - x) / dt; x = x_pred |

Passes 3 and 4 MUST each be dispatched once per color partition, parameterized by the
`(start, count)` index range for that partition (stored in `stretch_colors` / `bend_colors`
CPU-side vectors). The color range MUST be passed to the shader via a push constant or a
per-dispatch uniform buffer write. The shader MUST use the range to select only constraints
in the current color partition.

Body-proxy capsule and sphere data MUST be uploaded from `ClothBodyProxy`
(T-CONTRACTS §[T-CONTRACTS.body-proxy]) at `load_garment` time, not rebuilt per-substep.
The fixed GPU arrays are `GpuCapsule[32]` and `GpuSphere[16]` (maximum sizes from T-COLLISION);
if a body proxy exceeds these limits, `load_garment` MUST return
`ClothSolverError::BodyProxyCapacityExceeded`.

---

##### <N>.<i>.9 Constraint Graph Coloring

The solver MUST build a constraint graph at `load_garment` time and apply greedy graph coloring
before uploading constraint buffers to the GPU. This is the canonical parallelism strategy for
WGSL XPBD (confirmed by `jspdown/cloth` and `ccincotti3/webgpu_cloth_simulator` reference
implementations, and required by the absence of `atomicAdd<f32>` in WGSL baseline — see wgpu
issue #5329).

**Coloring algorithm.** Particles are graph nodes. Each constraint that references two particles
is a graph edge. A greedy graph coloring algorithm MUST assign a color to each constraint such
that no two same-color constraints share a particle. The coloring algorithm MUST be deterministic
from mesh connectivity (not seeded with random state) so the resulting color order is stable
across garment reloads (§<N>.<i>.7 point 2).

**Post-coloring buffer layout.** Constraints MUST be sorted by color in the GPU storage buffer.
The CPU MUST store `Vec<(u32, u32)>` color-partition ranges (start index, count) for stretch and
bend constraint types separately. These ranges drive the per-color dispatch loop in §<N>.<i>.8.4.

**Expected color count.** For typical garment meshes (triangulated Delaunay grids), greedy
coloring produces 4–12 colors, yielding 4–12 sequential GPU dispatches per iteration. This is
acceptable. An optional future optimization (not required for MVP) may apply supernodal graph
clustering (ACM C&G 2022, doi:10.1016/j.cag.2022.10.009) to reduce color count.

**Jacobi delta accumulation.** Within a single color-partition dispatch, all constraint threads
write to non-overlapping particle slots (guaranteed by the coloring), so direct writes to
`deltas[i]` in WGSL are race-free. A separate apply pass (reading `deltas` and `delta_cnt` to
produce averaged position corrections) MUST be used when the Jacobi averaging scheme is active.
Atomic scatter (`atomicAdd` on `f32`) MUST NOT be used, as WGSL does not support it.

---

##### <N>.<i>.10 `ClothSolver` Public Trait

The following trait definition is normative. All method signatures, including async boundaries
and error type, MUST match exactly:

```rust
// tailor-solver/src/lib.rs

#[async_trait::async_trait]
pub trait ClothSolver: Send + Sync {
    /// One-time garment upload: mesh, constraints, body proxy, color partitions → GPU buffers.
    /// Subsequent calls replace the current garment.
    async fn load_garment(
        &mut self,
        mesh:     GarmentMesh,
        material: FabricMaterial,
    ) -> Result<(), ClothSolverError>;

    /// Simulate n_frames frames. Returns final mesh state as SolverResult.
    async fn simulate(
        &mut self,
        n_frames: u32,
        params:   SimRunParams,
    ) -> Result<SolverResult, ClothSolverError>;

    /// Upload keyframeable material state for the next substep batch.
    /// Called by the animation timeline pump (§<animation>) between frames.
    fn update_params(&mut self, params: MaterialFrameParams);

    /// Free GPU buffers for the current garment.
    async fn unload(&mut self);

    /// SHA-256 of last simulate() final position buffer.
    /// Returns None if no simulation has run since load_garment().
    fn last_content_hash(&self) -> Option<[u8; 32]>;
}
```

`GarmentMesh` (defined in `tailor-solver/src/mesh.rs`) is the triangulated solver mesh built
from a `GarmentSpec`. It carries the particle array, all constraint arrays with color partitions,
the `ClothBodyProxy` capsule/sphere data, and grain UV layout. It MUST NOT carry lifecycle
metadata (`status`, `created_at`) — those live on the `tailor_garments` Postgres row.

`FabricMaterial` (defined in `tailor-solver/src/material.rs`) carries the decoded raw XPBD
compliance values built from `GarmentSpec.fabric` via the preset/decoder layer (§<fabric-models>).
It MUST NOT carry normalized `[0,1]` values — the decoding MUST happen before `load_garment`.

`ClothSolverError` MUST be a `thiserror`-derived enum covering at minimum:
`NoAdapter`, `DeviceError(wgpu::Error)`, `ShaderCompile(String)`,
`BodyProxyCapacityExceeded`, `MeshEmpty`, `MeshDegenerate(String)`, `SimTimeout`,
`ContentHashMismatch` (for same-machine idempotency failures only — not cross-backend).

---

##### <N>.<i>.11 EventLedger Integration Boundary

The `tailor-solver` crate MUST NOT emit EventLedger events. Event emission is the exclusive
responsibility of `src/tailor/solver_binding.rs` in `handshake_core`.

On `ClothSolver::simulate` completion, `TailorSandboxAdapter` (the kernel binding) MUST emit:

| Outcome | Event variant | Wire string |
|---|---|---|
| Success | `TailorSimRunCompleted` | `"TAILOR_SIM_RUN_COMPLETED"` |
| Solver error | `TailorSimRunRejected` | `"TAILOR_SIM_RUN_REJECTED"` |

Event `event_family` MUST be `"tailor.simulation"` (T-CONTRACTS §[T-CONTRACTS.event-types],
constant `TAILOR_SIMULATION`). The event payload MUST include:
`n_frames: u32`, `content_hash: String` (hex-encoded), `gpu_mem_peak_bytes: u64`,
`solver_version: String` (crate version), `sim_run_id: String` (the `SIM-` prefixed id from
`tailor_simulation_runs`).

The `TailorSimRunCompleted` / `TailorSimRunRejected` variants MUST be registered in
`required_first_slice_events()` in `kernel/mod.rs` (T-CONTRACTS §[T-CONTRACTS.event-types]).

The `tailor_simulation_runs` table row for this run MUST be written before the event is emitted
(write-then-event ordering, consistent with the kernel convention). The table uses the
`SIM-{uuid_v7}` id prefix (T-CONTRACTS §[T-CONTRACTS.tables]). The migration file MUST follow
the dated convention: `migrations/YYYY_MM_DD_tailor_simulation_runs.sql` with a `.down.sql`
reverse pair (T-CONTRACTS §[T-CONTRACTS.migration-naming]).

---

##### <N>.<i>.12 Validation Checks Owned by This Sub-Section

The following checks from the T-CONTRACTS §[T-CONTRACTS.validation] catalog are triggered by or
gate on solver outputs. They are listed here for cross-reference; the canonical definitions and
severity classifications are in T-CONTRACTS.

| Check code | Stage | Triggered by |
|---|---|---|
| `MESH_NOT_EMPTY` | post | `SolverResult.positions` non-empty |
| `NO_DEGENERATE_TRIS` | post | Triangle area check on output mesh |
| `SEAMS_CLOSED` | post | Seam constraint pair separation <= 1 mm in final frame |
| `NO_INTERPENETRATION` | post | Cloth particle vs body capsule/sphere SDF >= -0.5 mm |
| `SELF_INTERSECTION` | post | Self-collision pair count below explosion limit |
| `DRAPE_CONVERGED` | post | Final kinetic energy below convergence threshold |
| `PRESET_NO_NAN` | preset | No NaN/Inf in drape-test positions |
| `PRESET_STRETCH_NONZERO` | preset | Stretch compliance != 0 |
| `PRESET_DENSITY_POS` | preset | `density_g_m2 > 0` |

The `MeshComparator` result MUST be attached to the `ValidationReport` when the
`ValidationRunner` re-runs a sim for promotion: `max_deviation_mm`, `mean_deviation_mm`, and
`equivalent: bool` MUST be stored in the run receipt (schema `hsk.tailor.simulation_receipt@1`).

---

##### <N>.<i>.13 Non-Normative Provenance

The design rationale, OSS evidence, WGSL code sketches, and algorithm pseudocode in
`04-cloth-solver.md` (T-CLOTH-SOLVER) are non-normative background for this sub-section.
Where any name, field, or contract in T-CLOTH-SOLVER conflicts with T-CONTRACTS (`16-contracts.md`),
T-CONTRACTS is authoritative and this sub-section follows T-CONTRACTS. Specific superseded names
from T-CLOTH-SOLVER applied here:

- `TailorGarmentValidated` (04) → `TailorGarmentValidationRecorded` (T-CONTRACTS §[T-CONTRACTS.event-types])
- `TailorSimRunStarted`/`TailorSimRunCompleted`/`TailorSimRunRejected` — canonical forms retained
- `SolverMeshV1` (04 prose) → `SolverMesh` (canonical name; no `V1` suffix on the mesh type)
- `SimMode` (04) → `SolverMode` (canonical name; `Fitting`/`Animation`/`ChebyshevGs` variants)
- `tailor_material_library` (04, handshake-binding section) → `tailor_material_presets`
  (T-CONTRACTS §[T-CONTRACTS.tables])
- `GarmentCrdtUpdateRecorded` / `TailorGarmentCrdtUpdateRecorded` (04) →
  `TailorPanelCrdtUpdateRecorded` (T-CONTRACTS §[T-CONTRACTS.event-types])

## 13.4 Collision: Body, Self, Multi-Layer, Exaggerated Proportions

<!-- id: collision -->
<!-- source research (non-normative provenance): .GOV/reference/cloth_engine_research/05-collision.md -->
<!-- canonical contracts: .GOV/reference/cloth_engine_research/16-contracts.md (T-CONTRACTS) -->

---

##### Overview

This section specifies the collision subsystem of the `tailor-solver` crate. It covers four concerns:
cloth-vs-body collision (capsule proxy primary; SDF secondary), self-collision (curvature culling +
spatial hash), multi-layer garment ordering, and exaggerated-proportion robustness (large-bust
inter-collider overlap and tunneling). All type names, table names, event variants, schema IDs, and
migration names are the canonical forms from T-CONTRACTS. The research package
`.GOV/reference/cloth_engine_research/05-collision.md` is non-normative provenance; where any
detail in that file conflicts with T-CONTRACTS, T-CONTRACTS governs.

---

##### Body-Proxy Authority

**[COL-BODY-001]** The body proxy authority type MUST be `ClothBodyProxy` defined in
`tailor-solver/src/body/proxy.rs`. It MUST carry a capsule list (`Vec<CollisionCapsule>`) and a
sphere list (`Vec<CollisionSphere>`). All lengths and radii MUST be stored in **millimetres**. The
type MUST derive `serde::{Serialize, Deserialize}` and `schemars::JsonSchema`.

```rust
// tailor-solver/src/body/proxy.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClothBodyProxy {
    pub body_proxy_id: String,   // "BPX-{uuid_v7}"
    pub avatar_id:     String,   // "AVT-{uuid_v7}"
    /// Capsule chain (body segments). All distances in MILLIMETRES.
    pub capsules:      Vec<CollisionCapsule>,
    /// Sphere sub-proxies (breast/bust sub-volumes, joint spheres). MILLIMETRES.
    pub spheres:       Vec<CollisionSphere>,
    pub thickness_mm:  f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CollisionCapsule {
    pub joint_name: String,
    pub p0_mm:      [f32; 3],
    pub p1_mm:      [f32; 3],
    pub radius_mm:  f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CollisionSphere {
    pub bone:       String,
    pub center_mm:  [f32; 3],
    pub radius_mm:  f32,
}
```

**[COL-BODY-002]** The GPU upload types for the real-time substep loop MUST be `GpuCapsule` and
`GpuSphere`, both `#[repr(C)]` and `bytemuck::Pod`. `GpuCapsule` MUST store endpoint `a` and `b`
each as `[f32; 4]` (`.xyz` = position, `.w` = radius or padding). `GpuSphere` MUST store `center`
as `[f32; 4]` (`.xyz` = center, `.w` = radius). The fixed-size GPU arrays MUST be bounded at
**max 32 capsules** and **max 16 spheres** per body proxy to fit a single WGSL bind group.

```rust
// tailor-solver/src/body/gpu.rs
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCapsule {
    pub a: [f32; 4],  // .xyz = endpoint a, .w = radius
    pub b: [f32; 4],  // .xyz = endpoint b, .w = padding
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSphere {
    pub center: [f32; 4],  // .xyz = center, .w = radius
}
```

**[COL-BODY-003]** The authority JSONB for body proxy geometry MUST be stored in
`tailor_body_proxies.proxy_json` as a serialized `ClothBodyProxy`. The Postgres table MUST use
`TEXT PRIMARY KEY` with prefix `BPX-` (T-CONTRACTS FACT-4). The `avatar_id` column MUST reference
`tailor_avatars(avatar_id)`. There MUST NOT be a `garment_id` foreign key on `tailor_body_proxies`;
the garment-to-proxy link is via `tailor_garments.body_proxy_id`.

```sql
-- migration: 2026_MM_DD_tailor_body_proxies.sql  (dated convention; T-CONTRACTS.migration-naming)
-- 2026_MM_DD_tailor_body_proxies.down.sql reverse pair is required.
CREATE TABLE IF NOT EXISTS tailor_body_proxies (
    body_proxy_id           TEXT PRIMARY KEY,             -- "BPX-{uuid_v7}"
    avatar_id               TEXT NOT NULL REFERENCES tailor_avatars (avatar_id),
    workspace_id            TEXT NOT NULL,
    proxy_json              JSONB NOT NULL,               -- serialized ClothBodyProxy
    mode                    TEXT NOT NULL DEFAULT 'capsule'
        CHECK (mode IN ('capsule', 'capsule_sphere', 'capsule_sdf', 'sdf')),
    breast_proxy_mode       TEXT
        CHECK (breast_proxy_mode IS NULL OR
               breast_proxy_mode IN ('standard', 'multi_sphere', 'sdf_fallback')),
    sdf_artifact_ref        TEXT,
    lores_mesh_artifact_ref TEXT,
    joint_hierarchy_json    JSONB,
    collision_thickness_mm  FLOAT NOT NULL DEFAULT 2.5,
    event_ledger_event_id   TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_body_proxies_avatar
    ON tailor_body_proxies (avatar_id);
```

**[COL-BODY-004]** Every `tailor_body_proxies` INSERT MUST call
`guard_authority_write(AuthorityMode::Postgres)` before writing. SQLite writes to body proxy
tables are PROHIBITED (`no_sqlite_tripwire`).

**[COL-BODY-005]** `tailor_avatars` MUST exist as the avatar-identity authority table before any
`tailor_body_proxies` row can be inserted (FK constraint). The migration for `tailor_avatars` MUST
be applied before the migration for `tailor_body_proxies`. The canonical `tailor_avatars` DDL is
defined in T-CONTRACTS `[T-CONTRACTS.body-proxy]` and is not repeated here; this section only
adds collision-specific requirements on top of it.

---

##### Cloth-vs-Body Collision: Primary Mode (Capsule + Sphere Proxy)

**[COL-CAPSULE-001]** The primary body-collision mode MUST be the capsule + sphere proxy resolved
entirely in WGSL compute shaders on GPU. There MUST NOT be a CPU readback per substep for body
collision resolution.

**[COL-CAPSULE-002]** The body-collision compute pass (`collision_body.wgsl`) MUST be a
**pre-constraint pass**: it MUST execute at the beginning of each XPBD substep, before any stretch,
bend, or seam constraint is solved. This ordering prevents large velocity impulses from being
imparted to cloth particles by fast-moving collision objects.

**[COL-CAPSULE-003]** The WGSL shader MUST bind capsule and sphere buffers via storage bindings
and a uniform parameter struct. The `CollisionParams` uniform MUST carry `num_capsules: u32`,
`num_spheres: u32`, `thickness: f32`.

```wgsl
// tailor-solver/shaders/collision_body.wgsl  (canonical binding layout)
struct GpuCapsule { a: vec4<f32>, b: vec4<f32> }
struct GpuSphere  { center: vec4<f32> }

struct CollisionParams {
    num_capsules: u32,
    num_spheres:  u32,
    thickness:    f32,
    _pad:         f32,
}

@group(1) @binding(0) var<storage, read> capsules:         array<GpuCapsule>;
@group(1) @binding(1) var<storage, read> spheres:          array<GpuSphere>;
@group(1) @binding(2) var<uniform>       collision_params:  CollisionParams;
```

**[COL-CAPSULE-004]** The capsule correction kernel MUST compute the closest point on the capsule
segment to the cloth particle position and push the particle outward by `(radius + thickness)` when
the distance is below that sum. The distance computation MUST clamp the projection parameter `t` to
`[0.0, 1.0]` to handle endpoint degeneracy.

**[COL-CAPSULE-005]** The sphere correction kernel MUST push the cloth particle radially outward to
`(sphere_radius + thickness)` when the particle is inside that distance from the sphere center.

**[COL-CAPSULE-006]** The body-collision pass workgroup size MUST be 64 threads. The dispatch
covers all cloth particles (`ceil(N_particles / 64)` workgroups).

---

##### Cloth-vs-Body Collision: Secondary Mode (SDF)

**[COL-SDF-001]** The secondary body-collision mode MUST be a baked signed-distance-field (SDF)
volume stored as a `texture3d<f32>` on GPU. This mode MUST be used when `tailor_body_proxies.mode`
is `'capsule_sdf'` or `'sdf'`.

**[COL-SDF-002]** The SDF resolution MUST default to **64×64×64 voxels** covering the bounding box
of the avatar body region. The SDF MUST be re-baked when the avatar pose changes by more than a
configurable pose-change threshold. SDF baking MUST NOT occur in the real-time substep loop; it
MUST run as a pre-simulation bake pass.

**[COL-SDF-003]** The SDF collision-response kernel MUST apply the correction:
`x_corrected = x + (delta - sdf(x)) * grad_sdf(x)` where `delta` is the collision thickness
margin and `grad_sdf` is the trilinearly interpolated central-difference gradient of the SDF
texture. This MUST run as a pre-constraint pass in the same position in the substep pipeline as the
capsule pass (`[COL-CAPSULE-002]`).

**[COL-SDF-004]** The SDF mode MUST be selected automatically by `ClothBodyProxy::select_collision_mode()`
when the breast proxy sphere count exceeds 6 and the total capsule count exceeds 10. Manual override
is permitted via `tailor_body_proxies.mode`.

**[COL-SDF-005]** GPU SDF baking MUST use a jump-flooding or sphere-tracing approach. CPU-side
O(N_voxels × N_triangles) naive baking MUST NOT be used in interactive sessions. SDF bake artifacts
MUST be stored via `tailor_body_proxies.sdf_artifact_ref`.

---

##### Proxy Construction with Parry (CPU Pre-Processing Only)

**[COL-PARRY-001]** The `parry3d` crate (dimforge, Apache-2.0) MUST be used **only on the CPU** for
pre-simulation proxy construction and for broad-phase AABB culling during validation. It MUST NOT be
invoked inside the real-time substep loop.

**[COL-PARRY-002]** Avatar capsule proxy hierarchies MUST be built by `build_avatar_proxy()` in
`src/tailor/body/parry_build.rs` (within `handshake_core::tailor`). The function takes an imported
avatar mesh and a skeleton, assigns one `CollisionCapsule` per limb bone, and decomposes breast
bones into sphere approximations via V-HACD convex decomposition (`parry3d::transformation::vhacd`).

**[COL-PARRY-003]** The `parry3d` BVH (`Bvh`) MUST be used for broad-phase cloth-vs-body AABB
culling in the validation `CollisionValidationCheck` (see `[COL-VALIDATE-001]`). The dynamic BVH
variant SHOULD be preferred for animated avatar pose baking.

**[COL-PARRY-004]** The `parry3d` dependency MUST be declared in `tailor-solver/Cargo.toml`. It
MUST NOT be a dependency of `handshake_core` directly; proxy construction and validation call the
`tailor-solver` crate boundary.

---

##### Self-Collision: Curvature Culling Pre-Filter

**[COL-SELF-001]** Self-collision resolution MUST be preceded by a curvature-culling pre-filter
pass that identifies geometrically flat cloth regions and excludes their particles from the spatial
hash. Flat regions are collision-inactive and MUST NOT enter the narrow-phase hash query.

**[COL-SELF-002]** The curvature metric MUST be the h²-normalised discrete Laplace-Beltrami
operator mean curvature `H(v)` computed per vertex over the triangulated cloth mesh. A vertex is
classified as **active** (collision candidate) when `|H(v)| > curvature_threshold`; otherwise it is
**inactive** and skipped. The `curvature_threshold` parameter MUST be exposed in `ClothSimConfig`
with a default that achieves 40–70% reduction of active particles for typical flat-panel garments.

**[COL-SELF-003]** The curvature pass MUST run in a dedicated WGSL compute shader
(`shaders/curvature_cull.wgsl`) that writes a per-particle `active_flags: array<u32>` buffer.
Workgroup size MUST be 64.

---

##### Self-Collision: Spatial Hash GPU Architecture

**[COL-SELF-004]** The narrow-phase self-collision pass MUST use a GPU spatial hash built over
**active particles only** (those with `active_flags[i] == 1`). The hash cell size MUST be
`2 * r_particle` (twice the per-particle collision radius). The table size MUST be set to at least
`2 * num_active_particles` at simulation startup to bound collision rate; it MUST be configurable
in `ClothSimConfig`.

**[COL-SELF-005]** The spatial hash MUST be constructed on CPU using the canonical four-step
algorithm: (1) count particles per cell into `count_buffer`; (2) prefix-sum to produce
`cell_start`; (3) scatter particle indices into `particle_list`; (4) upload `cell_start` and
`particle_list` to GPU storage buffers. The hash function MUST be:

```rust
fn hash_coords(xi: i32, yi: i32, zi: i32, table_size: u32) -> u32 {
    ((xi as u32).wrapping_mul(92837111)
     ^ (yi as u32).wrapping_mul(689287499)
     ^ (zi as u32).wrapping_mul(283923481))
    % table_size
}
```

**[COL-SELF-006]** The WGSL self-collision query shader MUST query the **27 neighbouring cells**
(3×3×3 neighbourhood) for each active particle. For each candidate pair `(i, j)` with `j > i`,
the shader MUST apply an XPBD distance constraint with rest distance `2 * r_particle` and compliance
`alpha_self_collision / dt_substep^2`.

**[COL-SELF-007]** Self-collision constraint application MUST use **Jacobi iteration with delta
accumulation**: each particle writes its positional correction `Δx` to a separate `delta_pos` buffer
using atomic integer operations (fixed-point encoding: multiply by 10 000, cast to `i32`, then
`atomicAdd`). The main position buffer MUST be updated in a separate averaging pass after all
constraints for the substep are collected. Direct scatter-writes to `pred_pos` within the query
shader are PROHIBITED to avoid data races.

**[COL-SELF-008]** Velocity-level friction damping MUST be applied per self-collision pair as:
`dx_i += d * (friction_factor * delta_v) * dt_substep` where `friction_factor` is configurable in
`ClothSimConfig` in `[0.0, 1.0]`.

**[COL-SELF-009]** WGSL `f32` atomics MUST NOT be assumed universally available. The implementation
MUST use fixed-point integer atomics (`i32`) as the default path. An f32-atomic fast path MAY be
enabled at runtime when the `shader-atomic-float` wgpu feature is detected.

---

##### Multi-Layer Garment Collision

**[COL-LAYER-001]** Each garment in a multi-layer stack MUST carry a non-negative integer
`layer_index` stored in `tailor_garments.layer_index` (0 = innermost). Garments MUST be simulated
in ascending `layer_index` order: inner garments are draped first, then each outer garment is
draped against the already-settled inner layers.

**[COL-LAYER-002]** Inter-layer collision MUST be resolved as a constraint between outer-garment
particles and a spatial hash built over inner-garment particles. The rest distance for each
inter-layer distance constraint MUST be `thickness_inner + thickness_outer` (the sum of both
garments' `FabricProperties.collision_thickness_mm` values), not zero.

**[COL-LAYER-003]** The inter-layer collision pass MUST execute **after** body collision and after
intra-garment constraint solving, once per substep. The outer garment's inter-layer spatial hash
MUST use the same construction algorithm as the self-collision hash (`[COL-SELF-004]` through
`[COL-SELF-005]`).

**[COL-LAYER-004]** Inter-layer collision direction MUST be enforced asymmetrically: outer-garment
particles MUST stay outside inner-garment particles, but inner-garment particles are not pushed by
outer-garment particles. This prevents the inner layer from being driven upward by the outer layer.

**[COL-LAYER-005]** A post-simulation inter-layer penetration check MUST be run as the
`INTERLAYER_SPACING` validation check (T-CONTRACTS `[T-CONTRACTS.validation]`): no inter-layer
particle pair may be closer than `(t_inner + t_outer - tolerance)` in the final simulated frame.
This check is **Blocking** severity; failure prevents promotion.

**[COL-LAYER-006]** When inter-layer penetration contour length exceeds a configurable threshold
after the full substep budget, the simulation result MUST be flagged in the `SimulationReceipt`
with a `INTERLAYER_SPACING` finding at `Advisory` severity and `recommended_action:
edit_and_resimulate`. Contour-length measurement is a post-sim check, not a runtime constraint.

---

##### Exaggerated-Proportion Robustness

This section addresses avatars with a large bust volume (production target: cup size G–K+ or
equivalent) on a narrow rib cage. Three failure modes are specified and each MUST be mitigated.

###### Failure Mode 1: Inter-Collider Overlap Jitter

**[COL-BUST-001]** When two or more body-proxy spheres partially overlap and a cloth particle
receives simultaneously contradictory push corrections from them, the implementation MUST NOT sum
the correction vectors. It MUST apply the **maximum-magnitude correction** only: accumulate all
sphere correction vectors, then apply the one with the largest Euclidean magnitude.

```wgsl
// In collision_body_pass — accumulate sphere corrections, apply only the largest.
var best_correction = vec3(0.0);
var best_mag: f32 = 0.0;
for (var i = 0u; i < collision_params.num_spheres; i++) {
    let corr = sphere_correction(pred_pos_xyz, spheres[i], collision_params.thickness);
    let mag  = length(corr);
    if mag > best_mag {
        best_mag        = mag;
        best_correction = corr;
    }
}
pos += best_correction;
```

**[COL-BUST-002]** A minimum inter-sphere spacing constraint MUST be enforced during proxy
construction: adjacent breast-proxy spheres MUST be at least `0.5 * max(radius_a, radius_b)` apart
(center-to-center). Proxy JSON that violates this constraint MUST be rejected by the
`AVATAR_BINDING` fast pre-solver check.

###### Failure Mode 2: Under-Bust Cloth Tunneling

**[COL-BUST-003]** When `tailor_body_proxies.breast_proxy_mode` is `'multi_sphere'` or
`'sdf_fallback'` (i.e., the avatar proxy contains breast sphere sub-volumes), the body-collision
pass MUST run **twice per substep**: once for the main body capsule chain, and a second time
resolving residual penetrations from the breast sphere set. This doubled pass MUST be gated on the
`breast_proxy_mode` flag and MUST NOT apply to standard (non-bust) proxies.

**[COL-BUST-004]** Breast-sphere bones MUST be identifiable by joint name convention. The proxy
builder MUST tag any bone whose name contains `"breast"`, `"bust"`, `"BreastL"`, `"BreastR"`,
`"LeftBreast"`, or `"RightBreast"` (case-insensitive) as a breast bone. The WGSL pass MUST branch
on a `has_breast_spheres: u32` uniform flag.

###### Failure Mode 3: Under-Bust Crease Penetration

**[COL-BUST-005]** Body-collision resolution MUST precede stretch constraint solving within each
substep (already enforced by `[COL-CAPSULE-002]`). For large-bust avatars, the pre-constraint
ordering MUST be verified during validation: the `NO_INTERPENETRATION` check
(T-CONTRACTS `[T-CONTRACTS.validation]`) MUST report failure if any cloth particle is deeper than
`-0.5 mm` inside any body capsule or sphere in the **final simulated frame**.

###### Proxy Decomposition for Large-Bust Avatars

**[COL-BUST-006]** The canonical multi-sphere breast proxy decomposition MUST allocate **three
spheres per breast side** (primary volume, lower-quadrant volume, under-bust capping volume) plus
**one sternum capsule** guarding the inter-breast cleavage gap. This decomposition MUST be the
default output of `build_avatar_proxy()` when V-HACD convex decomposition of the breast sub-mesh
yields three or more convex parts per side. The resulting proxy MUST not exceed the global 16-sphere
budget (`[COL-BODY-002]`).

```json
// Canonical large-bust proxy decomposition (illustrative geometry; actual values from V-HACD):
{
  "left_breast": [
    { "bone": "LeftBreast",    "center_mm": [0, 0, 0],     "radius_mm": 80.0 },
    { "bone": "LeftBreast",    "center_mm": [0, -40, 15],  "radius_mm": 60.0 },
    { "bone": "LeftBreastLow", "center_mm": [0, -20, 0],   "radius_mm": 50.0 }
  ],
  "right_breast": [
    { "bone": "RightBreast",    "center_mm": [0, 0, 0],    "radius_mm": 80.0 },
    { "bone": "RightBreast",    "center_mm": [0, -40, 15], "radius_mm": 60.0 },
    { "bone": "RightBreastLow", "center_mm": [0, -20, 0],  "radius_mm": 50.0 }
  ],
  "sternum_gap": [
    { "joint_name": "Sternum", "p0_mm": [-20, 50, 0], "p1_mm": [-20, -20, 0], "radius_mm": 15.0 }
  ]
}
```

**[COL-BUST-007]** The SDF fallback (`[COL-SDF-004]`) MUST be triggered automatically when the
breast proxy sphere count exceeds 6. The SDF volume for this mode SHOULD cover only the
torso-to-breast bounding region (not the full body) to keep bake cost within the GPU budget.

---

##### EventLedger Events

**[COL-EVENT-001]** The following `KernelEventType` variants MUST be used for collision lifecycle
events (canonical wire strings per T-CONTRACTS `[T-CONTRACTS.event-types]`):

| Variant | Wire string (`as_str()`) | Trigger |
|---|---|---|
| `TailorBodyProxyCreated` | `"TAILOR_BODY_PROXY_CREATED"` | New `tailor_body_proxies` row inserted |
| `TailorBodyProxyUpdated` | `"TAILOR_BODY_PROXY_UPDATED"` | Proxy geometry changed (CRDT edit or rebuild) |
| `TailorAvatarCreated` | `"TAILOR_AVATAR_CREATED"` | New `tailor_avatars` row inserted |
| `TailorAvatarMeasurementsExtracted` | `"TAILOR_AVATAR_MEASUREMENTS_EXTRACTED"` | Anthropometric measurements extracted from imported mesh |

Superseded variants MUST NOT be used: `TailorProxyCreated`, `TailorProxyUpdated`,
`BodyProxyCreated`, `BodyProxyMeasurementsExtracted`, `TAILOR_PROXY_CREATED`,
`TAILOR_PROXY_UPDATED`, `TAILOR_COLLISION_PASS_RAN`, `TAILOR_TUNNELING_DETECTED`.

**[COL-EVENT-002]** `event_family` constants for collision events MUST use the canonical dot-namespaced
form from `src/tailor/event_family.rs`:

```rust
pub const TAILOR_BODY_PROXY: &str = "tailor.body_proxy";
pub const TAILOR_AVATAR:     &str = "tailor.avatar";
```

**[COL-EVENT-003]** Every collision-lifecycle event MUST be emitted via
`NewKernelEvent::builder(task_run_id, session_run_id, KernelEventType::Tailor*, KernelActor::System("tailor"))`
and MUST carry the `aggregate("tailor_body_proxy", proxy_id)` or `aggregate("tailor_avatar", avatar_id)`
call as appropriate, before `.build()`.

---

##### Schema IDs

**[COL-SCHEMA-001]** The body-proxy schema ID MUST be `"hsk.tailor.body_proxy@1"` (constant
`SCHEMA_TAILOR_BODY_PROXY_V1` in `src/tailor/schemas.rs`). The avatar schema ID MUST be
`"hsk.tailor.avatar@1"` (`SCHEMA_TAILOR_AVATAR_V1`). Use of `"hsk.cloth.*"` schema IDs for
authority body-proxy or avatar records is PROHIBITED (T-CONTRACTS `[T-CONTRACTS.schema-ids]`).

---

##### Validation Checks

**[COL-VALIDATE-001]** The `TailorValidationDescriptor` for collision MUST include the following
checks from the canonical catalog (T-CONTRACTS `[T-CONTRACTS.validation]`). Severity and stage are
as defined there and reproduced here for collision-specific traceability:

| Code | Severity | Stage | What it asserts |
|---|---|---|---|
| `AVATAR_BINDING` | Blocking | fast | `AvatarBinding.avatar_id` exists in `tailor_avatars` |
| `NO_INTERPENETRATION` | Blocking | post | No cloth particle deeper than −0.5 mm inside any body capsule/sphere in the final frame |
| `INTERLAYER_SPACING` | Blocking | post | No inter-layer pair closer than `t_inner + t_outer − tolerance` |
| `SELF_INTERSECTION` | Advisory | post | Self-collision pair count below mesh-explosion limit |

**[COL-VALIDATE-002]** The `NO_INTERPENETRATION` check MUST NOT run on intermediate substep frames.
It MUST only evaluate the final position buffer of the completed draping phase. Applying this check
to mid-simulation frames is PROHIBITED, as particles legitimately pass through brief inter-collider
states during initial draping.

**[COL-VALIDATE-003]** `ValidationReport::aggregate_blocks_promotion()` is the sole promotion gate.
Any `Blocking` failure in the collision checks above MUST prevent `TailorGarmentPromoted` from being
emitted. The `SimulationReceipt` (schema `hsk.tailor.simulation_receipt@1`) MUST carry each
failing check as a `ValidationFinding` with the stable `code` value and an optional
`suggested_fix { field_path, suggested_value }` pointing into `GarmentSpec` or `ClothBodyProxy`
JSON for model self-correction.

**[COL-VALIDATE-004]** The `MeshComparator` tolerance used for promotion equivalence is
`epsilon_mm = 0.1` (default; T-CONTRACTS `[T-CONTRACTS.determinism]`). Collision re-run
determinism MUST NOT be verified by SHA-256 `content_hash` equality; it MUST use
`MeshComparator::compare(a, b, epsilon_mm)` from `tailor-solver/src/compare.rs`.

---

##### Sandbox and Kernel Binding

**[COL-KERNEL-001]** The WGSL collision shaders MUST run inside the `TailorSandboxAdapter`
(`src/tailor/solver_binding.rs` within `handshake_core::tailor`). The adapter MUST declare
`AdapterIsolationTier::Process` with `SandboxCapability::Device` to permit `wgpu` device creation.
No collision GPU dispatch MAY occur outside the sandbox boundary.

**[COL-KERNEL-002]** Body proxy geometry (capsule and sphere buffers) MUST be uploaded to GPU
storage buffers at simulation start and MUST NOT be modified while a substep loop is in progress.
Proxy updates during animation (pose-driven capsule repositioning) MUST be double-buffered.

**[COL-KERNEL-003]** The `tailor-solver` crate MUST NOT depend on `handshake_core`. Collision
proxy types (`ClothBodyProxy`, `GpuCapsule`, `GpuSphere`) live in `tailor-solver`. The kernel
binding module `handshake_core::tailor` adapts them for EventLedger and Postgres authority writes.

---

##### Model-Lane Steerability

**[COL-MODEL-001]** The `TailorModelAdapter` MUST expose a `suggest_collision_proxy` tool call.
The tool accepts avatar topology context (bone hierarchy, breast morph target magnitude,
`measurements_mm_json`) and returns a `ClothBodyProxy` JSON proposal. The proposal MUST go through
the sandbox → validation → promotion pipeline before it becomes the authority proxy row. A model
MUST NOT write directly to `tailor_body_proxies` bypassing the EventLedger.

**[COL-MODEL-002]** An operator-facing suggestion field `garment_proxy_suggestion JSONB` MUST be
present on `tailor_avatars` to store the model's proxy proposal before operator review. The field
is nullable; a non-null value signals that an unreviewed suggestion is pending. On operator
confirmation the suggestion is promoted to a `tailor_body_proxies` row via
`TailorBodyProxyCreated`.

---

##### Migration Naming

**[COL-MIG-001]** Body-proxy and avatar migrations MUST follow the dated naming convention
(T-CONTRACTS `[T-CONTRACTS.migration-naming]`). The required files are:

```text
migrations/2026_MM_DD_tailor_avatars.sql
migrations/2026_MM_DD_tailor_avatars.down.sql
migrations/2026_MM_DD_tailor_body_proxies.sql
migrations/2026_MM_DD_tailor_body_proxies.down.sql
```

`MM` and `DD` MUST be the authoring date of the migration at implementation time, not this
research date. The `tailor_avatars` migration MUST precede the `tailor_body_proxies` migration.
Numbered `0NNN_*` migration names are PROHIBITED for all Tailor tables.

---

##### Non-Normative Implementation Notes

The following are design rationale and implementation guidance, not requirements.

The capsule-primary / SDF-secondary layering follows the Velvet CUDA XPBD architecture, where an
SDF pre-stabilization pass runs before the constraint loop. The curvature-culling pre-filter derives
from Efficient Self-Collision Culling for Real-Time Cloth Simulation Using Discrete Curvature
Analysis (MDPI Mathematics 14(9) 1504, April 2026), which reports 40–70% particle reduction. The
multi-sphere breast proxy pattern is documented in production pipelines for iClone/CC4 (32-body
budget, breast bone sub-proxy setup), Unreal Chaos Cloth (per-bone capsule list), and Daz dForce
community workarounds. The maximum-magnitude correction for overlapping spheres is a
Handshake-specific solution to the jitter produced when two opposing push vectors cancel to near
zero; the alternative (weighted average by penetration depth) may be adopted in a future revision
if the maximum-magnitude heuristic proves unstable for three or more simultaneous penetrations.
The Jacobi delta-accumulation approach for self-collision is taken from ccincotti3/webgpu_cloth_simulator
and from Carmen Cincotti's XPBD cloth tutorial series. Graph coloring (ccincotti3/jspdown) is an
admissible future optimization for high-performance mode and does not change any authority contract.

## 13.5 Fabric & Material Models

<!-- id: fabric  |  KERNEL_BUILDER: renumber heading on assembly -->
<!-- Non-normative provenance: research package T-FABRIC-MODELS (06-fabric-models.md),
     reconciled by T-CONTRACTS (16-contracts.md). All canonical names, schema-ids, event
     variants, table names, and PK forms come from T-CONTRACTS; the research file supplies
     the OSS evidence and calibration data that grounds the requirements below. -->

---

##### <N>.<i>.1 Scope

This section specifies the **Fabric & Material Model** subsystem of the Tailor engine: the
anisotropic physical properties that control how a garment drapes, stretches, bends, and
interacts with the body proxy in the XPBD solver. It covers:

- The six-scalar anisotropic compliance representation (`ClothMaterialCompliance`) used inside
  the `tailor-solver` crate and on the GPU.
- The `FabricProperties` normalized [0,1] surface that models and operators author inside
  `GarmentSpec`.
- The `tailor_material_presets` Postgres table as the authority preset library, including
  the bundled system presets.
- The non-linear mapping from normalized surface to raw compliance applied at solver-mesh
  build time.
- EventLedger events, validation checks, and the sandbox->validation->promotion lifecycle
  for model-authored presets.

This section does not specify the XPBD solver loop, constraint coloring, or GPU mesh pipeline
(see the Cloth Solver section). It does not specify trim, zipper, or lacing stiffness, which
are properties of `ClothTrimAttachment`, not `FabricMaterial`.

---

##### <N>.<i>.2 Anisotropic Compliance Model

**<N>.<i>.2.1 Full orthotropic split is mandatory.**

The Tailor solver MUST implement full orthotropic XPBD compliance with independent parameters
for weft, warp, and shear axes. Scalar-multiply shortcuts (a single `stretch_resistance` with
`warp_resistance_scale` / `weft_resistance_scale` multipliers, as used by GarmentCode
`stiff_ochra.json`) MUST NOT be used as the internal compliance representation; they are
rejected because they cannot express fabrics whose weft and warp have genuinely independent
stiffness curves (e.g., bias-cut silk or non-woven composite panels).

**<N>.<i>.2.2 Canonical raw compliance struct.**

The solver crate MUST define `ClothMaterialCompliance` in `tailor-solver/src/material.rs`
exactly as follows. Field names, field count, and the unit conventions (m²/N for stretch;
dimensionless ratio for buckling) are normative; the containing crate MUST NOT introduce
alternative field names or combine these into a smaller set.

```rust
// tailor-solver/src/material.rs
// No handshake_core deps — standalone crate.

/// Anisotropic XPBD compliance for a single fabric.
/// Stretch fields: m²/N (divided by rest_length^2 at constraint init — see §<N>.<i>.4).
/// Bend fields: same unit family, divided by rest_area.
/// Buckling fields: dimensionless ratio / absolute compliance addend.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClothMaterialCompliance {
    /// In-plane stretch compliance, weft axis (cross-grain). Denim ≈ 5e-8; silk ≈ 2e-4.
    pub stretch_weft: f32,
    /// In-plane stretch compliance, warp axis (along-grain).
    pub stretch_warp: f32,
    /// In-plane shear compliance (diagonal deformation).
    pub stretch_shear: f32,
    /// Out-of-plane bending compliance, weft-direction edges.
    pub bend_weft: f32,
    /// Out-of-plane bending compliance, warp-direction edges.
    pub bend_warp: f32,
    /// Fraction of max bend angle at which reduced-stiffness term activates. [0,1].
    /// 0 = no buckling transition; 1 = buckles at any bend angle (sharp wrinkles).
    pub buckling_ratio: f32,
    /// Additional compliance added at the buckled corner. Dimensionless addend.
    pub buckling_stiffness: f32,
}
```

**<N>.<i>.2.3 Per-fabric physical parameters.**

The solver crate MUST define `ClothMaterialPhysics` in `tailor-solver/src/material.rs`.
The internal unit for density is kg/m² (SI); the g/m² LLM-facing input MUST be converted
at the API boundary (divide by 1000).

```rust
/// Per-fabric physical parameters: mass, collision, dynamics.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClothMaterialPhysics {
    /// Surface density, kg/m². Cotton: 0.10–0.20; Denim: 0.28–0.45; Silk: 0.02–0.06.
    pub density_kg_per_m2: f32,
    /// Collision envelope, metres. Default 0.0025 (2.5 mm).
    pub collision_thickness_m: f32,
    /// Coulomb friction, cloth–avatar contact. [0,1].
    pub friction: f32,
    /// Coulomb friction, cloth–cloth self-collision. [0,1].
    pub self_friction: f32,
    /// Rayleigh-like velocity damping per substep. [0,1].
    pub internal_damping: f32,
    /// Air resistance coefficient. [0,∞).
    pub air_drag: f32,
    /// Inflation pressure target ratio. 0 = no pressure.
    pub pressure: f32,
    /// Stiffness blend: 0 = fully soft; 1 = fully rigid (solidify mode).
    pub solidify: f32,
    /// Weft shrinkage multiplier. 1.0 = no shrink; 0.9 = 10% shrink.
    pub shrinkage_weft: f32,
    /// Warp shrinkage multiplier.
    pub shrinkage_warp: f32,
}

/// Complete fabric material descriptor consumed by the solver.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FabricMaterial {
    pub compliance: ClothMaterialCompliance,
    pub physics: ClothMaterialPhysics,
    /// Grain direction, radians from panel horizontal. Controls weft/warp axis alignment.
    pub grain_angle_rad: f32,
}
```

**<N>.<i>.2.4 GPU buffer layout.**

The WGSL solver MUST consume material via a `MaterialParams` uniform at
`@group(0) @binding(2)`. The Rust mirror type MUST be `repr(C)` + `bytemuck::Pod` so the
CPU can upload it without unsafe pointer arithmetic. Field order MUST match the WGSL struct
byte-for-byte. The padding field `_pad: f32` is normative (16-byte alignment requirement).

```rust
// tailor-solver/src/gpu/material_uniform.rs
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialParamsGpu {
    // Compliance pack — 6 × f32
    pub stretch_weft:      f32,
    pub stretch_warp:      f32,
    pub stretch_shear:     f32,
    pub bend_weft:         f32,
    pub bend_warp:         f32,
    pub buckling_ratio:    f32,
    // Physics pack — 6 × f32
    pub density_kg_per_m2: f32,
    pub friction:          f32,
    pub self_friction:     f32,
    pub internal_damping:  f32,
    pub air_drag:          f32,
    pub pressure:          f32,
    // Keyframeable props — 4 × f32
    pub solidify:          f32,
    pub shrinkage_weft:    f32,
    pub shrinkage_warp:    f32,
    pub _pad:              f32,   // 16-byte alignment; MUST remain at index 15
}
```

The WGSL uniform block MUST declare the same field set and order as `MaterialParamsGpu`.
The anisotropic dispatch functions `stretch_compliance(dir_uv: vec2<f32>) -> f32` and
`bend_compliance(is_weft_edge: u32) -> f32` MUST be the sole entry points from constraint
shaders into the material uniform; constraint shaders MUST NOT read compliance fields
directly.

**<N>.<i>.2.5 Grain direction and weft/warp axis tagging.**

The mesh generation pipeline MUST tag each edge and triangle with a `uv_axis` attribute
computed from the 2D pattern UV coordinates at mesh generation time. When `grain_angle_rad`
is non-zero, the weft/warp coordinate frame MUST be rotated before the compliance lookup.
Without this tagging, the solver MUST fall back to isotropic compliance using
`(stretch_weft + stretch_warp) / 2.0`; it MUST NOT silently apply the wrong axis.

**<N>.<i>.2.6 Keyframeable material properties.**

The solver MUST support per-frame overrides for `solidify`, `shrinkage_weft`,
`shrinkage_warp`, and `pressure` via a `MaterialKeyframe` struct. When one or more
keyframe tracks are active, the solver loop MUST upload an updated `MaterialParamsGpu`
uniform at the start of each frame. Keyframes MUST be stored in the EventLedger
(event `TailorMaterialPresetRecorded` for initial; future animated-preset events per
the T-ANIMATION section) so the animation state is reproducible from authority storage alone.

```rust
/// Per-frame material override for animated properties. None = use base FabricMaterial.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialKeyframe {
    pub frame: u32,
    pub shrinkage_weft: Option<f32>,
    pub shrinkage_warp: Option<f32>,
    pub solidify:       Option<f32>,
    pub pressure:       Option<f32>,
}
```

---

##### <N>.<i>.3 FabricProperties — Normalized LLM Surface

**<N>.<i>.3.1 Two-layer design is normative.**

The Tailor system MUST maintain a strict two-layer design:

| Layer | Type | Values | Who authors it | Where it lives |
|---|---|---|---|---|
| **LLM surface** | `FabricProperties` | Normalized [0,1] | Models, operators | `GarmentSpec.fabric` (JSONB) |
| **Solver layer** | `ClothMaterialCompliance` + `ClothMaterialPhysics` | Raw compliance / SI units | Preset decoder | `tailor_material_presets` + solver mesh |

The mapping between layers MUST be applied exactly once, at `SolverMesh` build time
(`handshake_core::tailor::mesh::build_solver_mesh`). Raw compliance values MUST NOT be
stored inside `GarmentSpec`; normalized values MUST NOT be passed to the solver or GPU.

**<N>.<i>.3.2 Canonical FabricProperties type.**

`FabricProperties` is defined in `tailor-solver/src/spec.rs` as part of `GarmentSpec`
(T-CONTRACTS §[T-CONTRACTS.garment-spec]). The normative field set is:

```rust
/// Fabric properties — normalized [0.0, 1.0] LLM-facing surface.
/// 1.0 = stiffest / most resistant. Decoded to raw XPBD compliance at SolverMesh build time.
/// Two fields are NOT normalized (they are physical and LLM-legible in real units):
///   density_g_m2          g/m², [5, 2000]
///   collision_thickness_mm mm,  [0.1, 5]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(description = "Fabric properties, normalized [0,1]. Weft=cross-grain, Warp=along-grain.")]
pub struct FabricProperties {
    /// Named preset applied first; explicit fields below override per-field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<FabricPreset>,
    pub stretch_weft:           f32,   // [0,1]; 1 = near-inextensible (leather)
    pub stretch_warp:           f32,
    pub shear:                  f32,
    pub bending_weft:           f32,   // [0,1]; 1 = stiff bending (denim)
    pub bending_warp:           f32,
    pub buckling_ratio:         f32,   // [0,1]; 1 = fine wrinkles (silk)
    pub density_g_m2:           f32,   // g/m²; NOT normalized
    pub collision_thickness_mm: f32,   // mm;   NOT normalized
    pub friction:               f32,   // [0,1]
    pub internal_damping:       f32,   // [0,1]
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FabricPreset {
    Cotton, Denim, Silk, Jersey, Leather, Satin, Linen, Wool, Spandex, Chiffon, Canvas, Rubber,
}
```

`FabricPreset` variants MUST correspond 1:1 to `slug` values of `is_system_preset = true`
rows in `tailor_material_presets`. The decoder MUST resolve the preset row first, then apply
per-field overrides from `FabricProperties` before producing `ClothMaterialCompliance`.

**<N>.<i>.3.3 Non-linear (logarithmic) mapping is mandatory.**

The mapping from normalized field `v ∈ [0,1]` to raw compliance `α` MUST be non-linear.
A linear map is rejected because stretch compliance spans roughly six orders of magnitude
(`1e-9` for leather to `1e-3` for chiffon); a linear interpolation over that range
produces meaningless values in most of [0,1].

The REQUIRED mapping for stretch and bending fields is logarithmic interpolation between
the per-axis bounds `[α_min, α_max]` stored in the preset or in a hard-coded
axis-type bound table:

```
α = α_max * (α_min / α_max)^(1 - v)      where v = normalized value ∈ [0,1], v=1 → α_min (stiffest)
```

This is equivalent to: `α = exp(log(α_max) + v * (log(α_min) - log(α_max)))`.

The implementation MUST live in `handshake_core::tailor::material_decoder` (or an equivalent
module in `tailor-solver` if the decoder has no kernel deps) and MUST be applied during
`build_solver_mesh`. The decoder MUST NOT be inlined at call sites.

**<N>.<i>.3.4 Compliance normalization by rest geometry.**

The XPBD compliance `α` stored in presets and produced by the decoder MUST be interpreted
as a **physical-unit stiffness target**, not a direct solver-space compliance.
At constraint initialization, the solver MUST normalize:

- Stretch constraint: `α_effective = α_raw / rest_length^2`
- Bend constraint:   `α_effective = α_raw / rest_area`

where `rest_length` is the edge rest length and `rest_area` is the average area of the two
triangles sharing the dihedral edge. This normalization makes the compliance
mesh-resolution-independent. Solvers that pass `α_raw` directly to the XPBD update equation
without this normalization will produce mesh-resolution-dependent drape and MUST NOT be
promoted to production.

---

##### <N>.<i>.4 tailor_material_presets — Postgres Authority

**<N>.<i>.4.1 Single canonical table.**

The material preset authority MUST reside in a single Postgres table named
`tailor_material_presets`. The alternative names `tailor_material_library` and
`tailor_material` that appear in prior research drafts are superseded and MUST NOT be used
(T-CONTRACTS §[T-CONTRACTS.tables]).

**<N>.<i>.4.2 Schema.**

The migration creating this table MUST follow the dated naming convention
`<YYYY>_<MM>_<DD>_tailor_material_presets.sql` + `…down.sql` (T-CONTRACTS
§[T-CONTRACTS.migration-naming]). A numbered `0NNN_tailor_material_presets.sql` MUST NOT
be used. The table schema is:

```sql
-- Migration: <YYYY>_<MM>_<DD>_tailor_material_presets.sql
-- Reverse:   <YYYY>_<MM>_<DD>_tailor_material_presets.down.sql

CREATE TABLE IF NOT EXISTS tailor_material_presets (
    preset_id             TEXT PRIMARY KEY,             -- "MAT-{uuid_v7}"
    workspace_id          TEXT NOT NULL,
    name                  TEXT NOT NULL,
    slug                  TEXT NOT NULL,                -- kebab-case; unique per workspace
    description           TEXT,
    -- Raw anisotropic compliance (ClothMaterialCompliance as JSONB).
    -- Field names MUST match ClothMaterialCompliance exactly.
    compliance_json       JSONB NOT NULL,
    -- ClothMaterialPhysics as JSONB (density_kg_per_m2, collision_thickness_m, etc.).
    physics_json          JSONB NOT NULL,
    grain_angle_rad       DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    is_system_preset      BOOLEAN NOT NULL DEFAULT false,
    event_ledger_event_id TEXT NOT NULL
        REFERENCES kernel_event_ledger(event_id),
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, slug)
);
CREATE INDEX IF NOT EXISTS ix_tailor_material_presets_workspace
    ON tailor_material_presets (workspace_id, is_system_preset);
```

Primary key MUST use the `TEXT PRIMARY KEY` with the `MAT-` prefix, consistent with the
`TEXT PRIMARY KEY` convention established in recent kernel migrations (T-CONTRACTS
§[T-CONTRACTS.tables], FACT-4). The `UUID PRIMARY KEY DEFAULT gen_random_uuid()` form is
off-convention and MUST NOT be used.

Every `tailor_material_presets` INSERT MUST call `guard_authority_write(AuthorityMode::Postgres)`
before the sqlx query. SQLite writes to this table MUST NOT be permitted.

**<N>.<i>.4.3 Bundled system presets.**

The migration MUST seed the following system presets (`is_system_preset = true`).
The `compliance_json` and `physics_json` values below are the canonical starting calibration;
they MUST be validated against the drape test suite before the migration is merged, and
updated in-migration if calibration changes. All compliance values are the pre-normalization
raw physical-unit targets (applied after rest-geometry normalization per §<N>.<i>.3.4).

```json
[
  {
    "slug": "cotton",
    "name": "Cotton",
    "compliance_json": {
      "stretch_weft": 5e-6, "stretch_warp": 4e-6, "stretch_shear": 2e-5,
      "bend_weft": 3e-3, "bend_warp": 3e-3,
      "buckling_ratio": 0.6, "buckling_stiffness": 0.0
    },
    "physics_json": {
      "density_kg_per_m2": 0.15, "collision_thickness_m": 0.0025,
      "friction": 0.4, "self_friction": 0.2, "internal_damping": 0.05,
      "air_drag": 0.01, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "silk",
    "name": "Silk",
    "compliance_json": {
      "stretch_weft": 2e-4, "stretch_warp": 1.5e-4, "stretch_shear": 8e-4,
      "bend_weft": 8e-2, "bend_warp": 8e-2,
      "buckling_ratio": 0.9, "buckling_stiffness": 0.0
    },
    "physics_json": {
      "density_kg_per_m2": 0.04, "collision_thickness_m": 0.001,
      "friction": 0.1, "self_friction": 0.05, "internal_damping": 0.02,
      "air_drag": 0.005, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "denim",
    "name": "Denim",
    "compliance_json": {
      "stretch_weft": 5e-8, "stretch_warp": 3e-8, "stretch_shear": 1e-7,
      "bend_weft": 5e-5, "bend_warp": 4e-5,
      "buckling_ratio": 0.1, "buckling_stiffness": 0.5
    },
    "physics_json": {
      "density_kg_per_m2": 0.35, "collision_thickness_m": 0.003,
      "friction": 0.55, "self_friction": 0.4, "internal_damping": 0.1,
      "air_drag": 0.02, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "leather",
    "name": "Leather",
    "compliance_json": {
      "stretch_weft": 5e-9, "stretch_warp": 5e-9, "stretch_shear": 2e-8,
      "bend_weft": 8e-6, "bend_warp": 8e-6,
      "buckling_ratio": 0.05, "buckling_stiffness": 0.8
    },
    "physics_json": {
      "density_kg_per_m2": 0.50, "collision_thickness_m": 0.004,
      "friction": 0.65, "self_friction": 0.5, "internal_damping": 0.15,
      "air_drag": 0.03, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "jersey",
    "name": "Jersey / Knit",
    "compliance_json": {
      "stretch_weft": 1e-4, "stretch_warp": 5e-5, "stretch_shear": 3e-4,
      "bend_weft": 2e-2, "bend_warp": 2e-2,
      "buckling_ratio": 0.85, "buckling_stiffness": 0.0
    },
    "physics_json": {
      "density_kg_per_m2": 0.13, "collision_thickness_m": 0.002,
      "friction": 0.35, "self_friction": 0.15, "internal_damping": 0.04,
      "air_drag": 0.008, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "wool",
    "name": "Wool",
    "compliance_json": {
      "stretch_weft": 2e-6, "stretch_warp": 1.5e-6, "stretch_shear": 5e-6,
      "bend_weft": 4e-3, "bend_warp": 4e-3,
      "buckling_ratio": 0.5, "buckling_stiffness": 0.1
    },
    "physics_json": {
      "density_kg_per_m2": 0.25, "collision_thickness_m": 0.0035,
      "friction": 0.5, "self_friction": 0.35, "internal_damping": 0.08,
      "air_drag": 0.015, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "chiffon",
    "name": "Chiffon",
    "compliance_json": {
      "stretch_weft": 5e-4, "stretch_warp": 4e-4, "stretch_shear": 1.5e-3,
      "bend_weft": 1.5e-1, "bend_warp": 1.5e-1,
      "buckling_ratio": 0.95, "buckling_stiffness": 0.0
    },
    "physics_json": {
      "density_kg_per_m2": 0.025, "collision_thickness_m": 0.0008,
      "friction": 0.05, "self_friction": 0.02, "internal_damping": 0.01,
      "air_drag": 0.003, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "canvas",
    "name": "Canvas",
    "compliance_json": {
      "stretch_weft": 8e-9, "stretch_warp": 6e-9, "stretch_shear": 2e-8,
      "bend_weft": 3e-5, "bend_warp": 3e-5,
      "buckling_ratio": 0.05, "buckling_stiffness": 0.9
    },
    "physics_json": {
      "density_kg_per_m2": 0.45, "collision_thickness_m": 0.004,
      "friction": 0.6, "self_friction": 0.45, "internal_damping": 0.12,
      "air_drag": 0.025, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  },
  {
    "slug": "rubber",
    "name": "Rubber",
    "compliance_json": {
      "stretch_weft": 1e-4, "stretch_warp": 1e-4, "stretch_shear": 5e-4,
      "bend_weft": 1e-3, "bend_warp": 1e-3,
      "buckling_ratio": 0.0, "buckling_stiffness": 0.0
    },
    "physics_json": {
      "density_kg_per_m2": 0.30, "collision_thickness_m": 0.003,
      "friction": 0.75, "self_friction": 0.6, "internal_damping": 0.2,
      "air_drag": 0.01, "pressure": 0.0, "solidify": 0.0,
      "shrinkage_weft": 1.0, "shrinkage_warp": 1.0
    }
  }
]
```

System presets (`is_system_preset = true`) MUST NOT be deleted by user-facing operations.
A soft-delete column SHOULD NOT be added; instead, custom forks via `tailor_fabric_preset_fork`
should be the operator's path to modified archetypes.

**<N>.<i>.4.4 Per-panel preset override.**

`PanelSpec` MAY carry an optional `material_preset_id: Option<String>` referencing
`tailor_material_presets.preset_id`. When present, it overrides the garment-level
`GarmentSpec.fabric` for that panel only. The authority assignment row lives in
`tailor_material_assignments` (T-CONTRACTS §[T-CONTRACTS.tables]).

---

##### <N>.<i>.5 EventLedger Integration

**<N>.<i>.5.1 Canonical event variants.**

All material-preset mutations MUST emit the following `KernelEventType` variants
(T-CONTRACTS §[T-CONTRACTS.event-types]). Wire strings are the `as_str()` SCREAMING_SNAKE_CASE form.

| Variant | Wire string | Trigger |
|---|---|---|
| `TailorMaterialPresetRecorded` | `TAILOR_MATERIAL_PRESET_RECORDED` | New preset created (system seed or user) |
| `TailorMaterialPresetUpdated` | `TAILOR_MATERIAL_PRESET_UPDATED` | Mutable field edit on existing preset |
| `TailorMaterialPresetRejected` | `TAILOR_MATERIAL_PRESET_REJECTED` | Sandbox drape validation failed |
| `TailorGarmentMaterialAssigned` | `TAILOR_GARMENT_MATERIAL_ASSIGNED` | Preset linked to a garment/panel |

The superseded variant names `TailorMaterialPresetDeleted` and `TailorMaterialLibraryUpdated`
from prior drafts MUST NOT be added to `KernelEventType`.

**<N>.<i>.5.2 event_family constants.**

Material events MUST use the `event_family` constant `tailor.material`
(`src/tailor/event_family.rs`: `pub const TAILOR_MATERIAL: &str = "tailor.material";`).

**<N>.<i>.5.3 Preset creation pattern.**

Every preset write MUST follow the kernel authority write pattern:

```rust
// src/tailor/material.rs  (handshake_core::tailor)
pub async fn create_preset(
    pool:           &PgPool,
    actor:          KernelActor,
    task_run_id:    &str,
    session_run_id: &str,
    req:            CreateFabricPresetRequest,
) -> TailorResult<FabricPresetRow> {
    guard_authority_write(AuthorityMode::Postgres)?;   // no-SQLite tripwire

    let row = sqlx::query_as!(FabricPresetRow, /* INSERT … RETURNING */ )
        .fetch_one(pool)
        .await?;

    let event = NewKernelEvent::builder(
        task_run_id, session_run_id,
        KernelEventType::TailorMaterialPresetRecorded,
        actor,
    )
    .aggregate("tailor_material_preset", &row.preset_id)
    .idempotency_key(&format!("tailor-preset-{}", row.preset_id))
    .payload(serde_json::json!({
        "preset_id":   row.preset_id,
        "workspace_id": req.workspace_id,
        "slug":        req.slug,
        "name":        req.name,
        "compliance_summary": {
            "stretch_weft": req.compliance.stretch_weft,
            "bend_weft":    req.compliance.bend_weft,
        }
    }))
    .source_component("tailor::material")
    .build()?;

    insert_kernel_event(pool, event).await?;
    Ok(row)
}
```

Schema ID for the preset payload MUST be `hsk.tailor.material_preset@1`
(T-CONTRACTS §[T-CONTRACTS.schema-ids]: `SCHEMA_TAILOR_MATERIAL_PRESET_V1`).

---

##### <N>.<i>.6 Sandbox, Validation, and Promotion for Model-Authored Presets

**<N>.<i>.6.1 Model-authored presets MUST pass drape validation before promotion.**

When the model lane emits a new or forked preset, the proposed `FabricMaterial` JSON MUST
enter the sandbox pipeline. Direct writes to `tailor_material_presets` that bypass the
sandbox MUST NOT be permitted from the model lane.

**<N>.<i>.6.2 Drape test specification.**

The sandbox validation test for a fabric preset MUST be:

- Mesh: a 0.5 m × 0.5 m square cloth panel, 5 mm particle distance (~10,000 triangles).
- Simulation: 1 second of drape under standard gravity (9.81 m/s²), 30 substeps per frame,
  at 30 fps, against a horizontal plane collision object.
- Pass condition: all `PRESET_*` blocking checks pass (see §<N>.<i>.6.3).

A coarser preview path (`cloth_preview_material`) MAY run a reduced test
(0.1 s, ~200 particles, 10 substeps) for real-time UI feedback without sandbox/promotion.
The preview path MUST NOT be used as the promotion gate.

**<N>.<i>.6.3 Applicable validation checks.**

The following checks from the canonical `ValidationDescriptor` catalog
(T-CONTRACTS §[T-CONTRACTS.validation]) apply to preset drape validation:

| Code | Severity | What it asserts |
|---|---|---|
| `PRESET_NO_NAN` | Blocking | No NaN or Inf in drape-test particle positions at any substep |
| `PRESET_STRETCH_NONZERO` | Blocking | All six compliance scalars != 0.0 (zero diverges the solver) |
| `PRESET_DENSITY_POS` | Blocking | `density_kg_per_m2 > 0` |
| `PRESET_BBOX_PLAUSIBLE` | Advisory | Drape-test bounding box within expected range for the claimed archetype |
| `FABRIC_RANGE` | Blocking | Normalized `FabricProperties` fields in [0.0, 1.0]; `density_g_m2` in [5, 2000]; `collision_thickness_mm` in [0.1, 5] |

Any `Blocking` failure MUST emit `TailorMaterialPresetRejected` with a diagnostic JSON
payload naming the failed check code. The `ValidationFinding.code` values MUST use the
exact codes from the catalog above so the model can pattern-match and self-correct.

**<N>.<i>.6.4 Self-correction contract.**

The `SimulationReceipt` returned after a failed preset validation MUST include
`ValidationFinding` entries with `code`, `severity`, and where applicable a
`suggested_fix { field_path, suggested_value }` using a JSON-pointer path into the
`FabricProperties` struct. The `recommended_action` MUST be `correct_spec_first` when
`FABRIC_RANGE` fails, or `edit_and_resimulate` when `PRESET_BBOX_PLAUSIBLE` fails.
Schema ID for the receipt is `hsk.tailor.simulation_receipt@1`.

---

##### <N>.<i>.7 Model-First API Surface

**<N>.<i>.7.1 The LLM MUST NOT reason about raw compliance values.**

MCP tool definitions for fabric presets MUST expose only the `FabricProperties` normalized
[0,1] surface plus `density_g_m2` and `collision_thickness_mm` (the two physical-unit
fields retained for LLM legibility). Raw compliance scalars (`stretch_weft: 5e-8`) MUST NOT
appear in MCP tool input schemas.

**<N>.<i>.7.2 Required MCP tools.**

The following MCP tools MUST be registered in the Tailor model lane:

`tailor_fabric_preset_create` — Create a new workspace preset from a named archetype with
optional normalized-field overrides. Required inputs: `workspace_id`, `name`,
`fabric_archetype` (one of the system preset slugs or `"custom"`). Optional:
`fabric_properties` (partial `FabricProperties`; overrides archetype defaults per field).
The archetype MUST resolve to the corresponding `tailor_material_presets` system preset row,
which supplies the raw compliance base; per-field overrides apply after logarithmic decode.

`tailor_fabric_preset_fork` — Clone an existing preset by `preset_id` with a new name and
optional `FabricProperties` field overrides. MUST follow the same sandbox->validation->
promotion flow as `tailor_fabric_preset_create`.

`tailor_fabric_preset_list` — Return workspace presets as a structured array with
`preset_id`, `slug`, `name`, `is_system_preset`, and a `compliance_summary` containing the
key differentiating normalized values (`bending_weft`, `stretch_weft`, `density_g_m2`).
MUST NOT return raw compliance scalars.

`tailor_garment_panel_assign_material` — Link a `preset_id` to a specific `panel_id` in a
garment, writing to `tailor_material_assignments` and emitting `TailorGarmentMaterialAssigned`.

**<N>.<i>.7.3 ContextBundle fabric hint.**

When the kernel constructs a `ContextBundle` for garment authoring, it MUST include:

- The current workspace preset list (slugs, names, key normalized summaries).
- The `FabricProperties` currently assigned to each panel in the garment being edited.
- A `FabricMaterialHint` derived from the `GarmentSpec.natural_description` field when
  present (e.g., `"leather corset"` → archetype: `leather`, high friction suggestion).

The model MUST use this bundle to select or tune presets without needing to know raw
compliance numbers.

---

##### <N>.<i>.8 CRDT Collaborative Editing

Concurrent edits to the same `tailor_material_presets` row MUST be merged via the kernel's
CRDT layer. The CRDT conflict resolution strategy for presets MUST be last-write-wins
per-property-key, where each compliance or physics scalar is an independent CRDT map entry
(matching the YJS `Map` semantics in the kernel's `yjs_bridge`). Edits MUST be recorded
via `TailorMaterialPresetUpdated`. `TailorCrdtConflictDetected` MUST be emitted when a
conflict is detected during merge; the merge MUST NOT silently discard either side without
recording the conflict event.

---

##### <N>.<i>.9 Constraints and Invariants

The following invariants are system-enforced and MUST be maintained at all times:

1. **Schema-ID namespace.** The schema ID constant for material preset payloads is
   `hsk.tailor.material_preset@1` (`SCHEMA_TAILOR_MATERIAL_PRESET_V1`). The namespace
   `hsk.cloth.*` MUST NOT be used for any Tailor-domain authority record
   (T-CONTRACTS §[T-CONTRACTS.schema-ids]).

2. **Single table name.** The table name is `tailor_material_presets` only.
   `tailor_material_library` and `tailor_material` are superseded aliases and MUST be
   treated as errors if found in new code.

3. **Density unit boundary.** `density_g_m2` (g/m²) on the LLM surface; `density_kg_per_m2`
   (kg/m²) in the solver and Postgres. The divide-by-1000 conversion MUST be applied
   exactly once, at the API boundary in `handshake_core::tailor::material_decoder`.

4. **Buckling as post-MVP enhancement.** The nonlinear bending model required for full
   `buckling_ratio` behavior (SIGGRAPH MIG 2025, ACM DOI 10.1145/3769047.3769050)
   SHOULD NOT block the MVP. The MVP MUST support `buckling_ratio` as a stored field and
   include it in the `FabricProperties` and `ClothMaterialCompliance` types, but the MVP
   solver MAY implement it as a linear bending fallback. The field MUST remain in the schema
   so that post-MVP solver upgrades apply without a migration or API change.

5. **No trim stiffness on FabricMaterial.** Trim and tack stiffness is a property of
   `ClothTrimAttachment`, not `FabricMaterial`. Proposals to add trim stiffness fields to
   `ClothMaterialCompliance` or `FabricProperties` MUST be rejected.

6. **MeshComparator for preset promotion equivalence.** When the sandbox re-runs a drape
   test to confirm reproducibility, the comparison MUST use `MeshComparator` with
   `epsilon_mm = 0.1` (T-CONTRACTS §[T-CONTRACTS.determinism]). Content-hash equality
   MUST NOT be used as the promotion equivalence criterion for cross-backend runs.

---

##### <N>.<i>.10 Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Compliance calibration values are mesh-resolution-dependent | Wrong drape at non-default particle densities | Rest-geometry normalization per §<N>.<i>.3.4 is mandatory; drape test suite MUST include tests at 3 mm and 10 mm particle spacing |
| Weft/warp axis tagging absent from mesher | Anisotropy silently degrades to isotropic | Mesher MUST assert `uv_axis` presence; solver MUST log a warning and apply isotropic fallback if tag missing |
| Logarithmic decode produces values outside solver-safe range | Solver divergence | Decoder MUST clamp output to `[1e-10, 1.0]` and emit an `Advisory` `ValidationFinding` if clamping occurred |
| Drape test too slow for interactive preset editing (~100–500 ms) | Poor UX for iterative tuning | Expose `cloth_preview_material` (coarse, no sandbox) for UI; gate promotion on full drape test only |
| `buckling_ratio` linear fallback produces wrong wrinkle frequency | Hero-asset visual quality gap | Document the limitation in the MVP release note; the nonlinear bending upgrade is a tracked post-MVP item |

---

*Non-normative provenance: The OSS evidence, compliance calibration tables, WGSL examples,
and GarmentCode / nikhilr612/xpbdrs / Velvet / webgpu_cloth_simulator reference
implementations that informed the requirements above are documented in
`T-FABRIC-MODELS` (`06-fabric-models.md`) and `T-CONTRACTS` (`16-contracts.md`) in the
research package at `.GOV/reference/cloth_engine_research/`. The research files are
non-normative; this section is the authority.*

## 13.6 Garment Authoring: Patterns, Seams, Parametric/Model-First

> **Sub-section id:** `garment`
> **File:** `06-garment-authoring.md`
> **Contract authority:** T-CONTRACTS (`.GOV/reference/cloth_engine_research/16-contracts.md`).
> All type names, field names, units, event variants, schema IDs, table names, migration naming,
> validation check codes, and promotion-equivalence rules cited below are verbatim from T-CONTRACTS
> and supersede any conflicting names in the research files (`03-garment-authoring.md` and siblings).
> The research files are non-normative provenance; this sub-section is product LAW.

---

##### <N>.<i>.1 Scope

This sub-section governs the garment authoring layer of the Tailor engine: the canonical data
representation for 2D sewing patterns and seams (`GarmentSpec`), the pattern-to-3D pipeline, the
three-tier parametric/LLM-emittable authoring surface, GarmentCode interop, CRDT collaborative
editing of panel geometry, and the sandbox-to-promotion lifecycle for model-authored garments.

Downstream subsystems (XPBD solver, collision, fabric models, auto-fit, UV, trims, animation) are
governed by their own sub-sections and consume the types defined here.

---

##### <N>.<i>.2 Canonical Garment Type: `GarmentSpec`

**[GAR-001]** The canonical authority type for a garment definition MUST be `GarmentSpec` (not
`GarmentSpecV1`, not `GarmentDraftV1`). It MUST live in `tailor-solver/src/spec.rs` so the
`tailor-solver` crate public API equals the model API. T-CONTRACTS [T-CONTRACTS.garment-spec]
is the definitive resolution of all prior drift across the research package.

**[GAR-002]** `GarmentSpec` MUST derive `serde::Serialize`, `serde::Deserialize`, and
`schemars::JsonSchema` so the MCP `inputSchema` is auto-generated and the type is
LLM-emittable without a hand-authored schema.

**[GAR-003]** `GarmentSpec` MUST carry `schema_id: String` whose value is the constant
`hsk.tailor.garment_spec@1` (from `src/tailor/schemas.rs`; see [GAR-SCHEMA-IDS]).
The field MUST be present on every serialized instance so receivers can version-check
without type introspection.

**[GAR-004]** Garment identity MUST use `garment_id: String` with the prefixed form `"GAR-{uuid_v7}"`.
The `UUID PRIMARY KEY DEFAULT gen_random_uuid()` form is PROHIBITED (T-CONTRACTS FACT-4).

**[GAR-005]** Status (`draft | sandbox_pending | simulated | validated | promoted | rejected |
archived`) MUST NOT appear on `GarmentSpec`. Status is lifecycle metadata and MUST live only on
the `tailor_garments.status` Postgres column. Model-emitted specs MUST NOT set status fields.

**[GAR-006]** The complete canonical `GarmentSpec` Rust definition is:

```rust
// tailor-solver/src/spec.rs
// Standalone crate; no handshake_core deps. THIS is the canonical garment type.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical garment specification.
/// LLM primary output type, solver primary input type, and Postgres authority JSONB.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Complete garment: panels, seams, darts, pleats, fabric, avatar binding.")]
pub struct GarmentSpec {
    /// Constant "hsk.tailor.garment_spec@1".
    pub schema_id: String,
    /// Prefixed id: "GAR-{uuid_v7}".
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
    /// Fabric physical properties — normalized [0,1] LLM-facing surface.
    pub fabric: FabricProperties,
    /// Avatar/body-proxy binding for fit and collision.
    pub avatar: AvatarBinding,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trim_placements: Vec<TrimPlacementRef>,
    /// Optional natural-language description; aids LLM edit coherence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natural_description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GarmentType {
    Tshirt, Shirt, Jacket, Blazer, Dress, Skirt, Pants, Shorts,
    Bodice, Cape, Hood, Sleeve, Custom,
}
```

---

##### <N>.<i>.3 Units Contract

**[GAR-UNITS-001]** All length quantities in `GarmentSpec`, `PanelSpec`, `SeamSpec`, `DartSpec`,
`PleatSpec`, `EdgeShape`, `Vec2Cm`, and `Transform3D` MUST be in centimetres (cm). The `_cm`
suffix MUST appear on every length field name to make the unit self-documenting.

**[GAR-UNITS-002]** The authority body-proxy (`tailor_body_proxies`, `ClothBodyProxy`) MUST store
lengths in millimetres. The `BodyMeasurements` struct in `GarmentSpec.avatar.measurements_cm`
MUST use centimetres as the LLM-facing convenience surface and MUST be converted at the boundary
before writing to `tailor_body_proxies`. The `_mm` / `_cm` suffixes MUST be present on every
measurement field to make the unit boundary explicit.

**[GAR-UNITS-003]** Normalized [0,1] coordinates MUST NOT appear in authority `GarmentSpec`
instances. The ChatGarment-style 76-float normalized parametric vector (T-CONTRACTS
[T-CONTRACTS.garment-spec] "Tier-1 pre-decode") is a pre-decode convenience representation only
and MUST be decoded to cm `GarmentSpec` before storage or solver invocation.

**[GAR-UNITS-004]** The canonical 2D panel point type is `Vec2Cm { x: f32, y: f32 }`. The 6D
panel placement type is `Transform3D { translation_cm: [f32; 3], rotation: [f32; 4] }`. Rust
implementations MUST use these exact names.

---

##### <N>.<i>.4 Panel Representation

**[GAR-PANEL-001]** Each 2D sewing panel MUST be represented by a `PanelSpec` with an explicit
vertex array (`vertices_cm: Vec<Vec2Cm>`, panel-local coordinates in cm, counter-clockwise
winding) and an ordered directed edge list (`edges: Vec<EdgeSpec>`) that closes the outline loop.

**[GAR-PANEL-002]** Panel vertices MUST be in counter-clockwise winding order. Clockwise panels
MUST be auto-corrected at authoring time and MAY be reported with the `WINDING` advisory check
(see [GAR-VALIDATION]). The WINDING check is advisory, not blocking.

**[GAR-PANEL-003]** Every `PanelSpec` MUST carry a `panel_id: String` that is stable and
kebab-case within the garment (e.g. `"front-bodice"`, `"back-panel"`). The panel ID is the
reference key used by `SeamSpec`, `DartSpec`, `PleatSpec`, and the UV island tables.

**[GAR-PANEL-004]** Every `PanelSpec` MUST carry a `placement: Transform3D` for its initial 3D
draping pose. This is not optional — the solver requires a starting position for every panel.

**[GAR-PANEL-005]** `PanelSpec::grain_angle_deg: Option<f32>` specifies the fabric grain direction
as degrees from panel horizontal. `None` means isotropic. The grain angle governs UV island
orientation and anisotropic fabric material direction in the solver.

**[GAR-PANEL-006]** The minimum valid panel area MUST be greater than 1.0 cm^2. The `MIN_PANEL_AREA`
blocking check (see [GAR-VALIDATION]) enforces this gate before solver invocation.

**[GAR-PANEL-007]** The canonical `PanelSpec` Rust definition is:

```rust
// tailor-solver/src/spec.rs

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

/// Edge shape in panel-local 2D (cm). Typed enum — supersedes string curve_type forms.
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
    /// Fold seam angle in degrees (fold seam lines; None = standard cut edge).
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
```

---

##### <N>.<i>.5 Edge Shape Contract

**[GAR-EDGE-001]** Edge shape MUST be represented by the typed `EdgeShape` enum with variants
`Straight`, `Quadratic`, `Cubic`, and `Arc`. The string `curve_type` form from research file `10`
is PROHIBITED for authority types.

**[GAR-EDGE-002]** `EdgeShape::Quadratic { control_cm: Vec2Cm }` represents a quadratic Bezier
edge with one control point in panel-local cm coordinates.

**[GAR-EDGE-003]** `EdgeShape::Cubic { control_a_cm: Vec2Cm, control_b_cm: Vec2Cm }` represents
a cubic Bezier edge with two control points in panel-local cm coordinates.

**[GAR-EDGE-004]** `EdgeShape::Arc { curvature: f32 }` represents a circular arc; positive
curvature bends left relative to the edge direction.

**[GAR-EDGE-005]** All Bezier control points and arc parameters MUST be expressed in panel-local
2D cm coordinates, not in 3D world-space coordinates. The `PanelSpec::placement` transform
handles the conversion to 3D for solver invocation.

**[GAR-EDGE-006]** `EdgeSpec::endpoints: [u32; 2]` MUST reference valid indices into the parent
`PanelSpec::vertices_cm` array. The `SEAM_EDGE_REF` and `PANEL_CLOSURE` blocking checks
(see [GAR-VALIDATION]) enforce this gate.

---

##### <N>.<i>.6 Seam Representation

**[GAR-SEAM-001]** Each stitch joining two panel edges MUST be represented by a `SeamSpec`.
Every `SeamSpec` MUST reference two `SeamEndpoint` values (`from` and `to`), each identifying
a `panel_id` and an `edge_index` into that panel's `edges` array.

**[GAR-SEAM-002]** The gathering ratio field MUST be named `gather_ratio: f32` on `SeamSpec`.
The alternative field name `ratio` from research file `10`/`15` is PROHIBITED for authority types.

**[GAR-SEAM-003]** `gather_ratio` is defined as `from_edge_length / to_edge_length`. A value of
`1.0` means a flat seam. A value greater than `1.0` means the `from` edge is gathered onto the
shorter `to` edge. The valid range MUST be `(0.0, 20.0]`. The `GATHER_RATIO_RANGE` blocking check
enforces this gate.

**[GAR-SEAM-004]** The solver MUST represent M:N gathering by resampling both seam edges to equal
vertex count N at mesh-generation time and emitting N point-distance constraints with rest-length
zero. The `gather_ratio` scalar MUST be carried forward on `SeamConstraintRecord` for compliance
scaling. The "M:N int pair" alternative is PROHIBITED as a stored authority field.

**[GAR-SEAM-005]** `SeamEndpoint::range: Option<[f32; 2]>` permits partial-edge (Free) sewing
for partial seams. The range values MUST be in [0.0, 1.0] representing normalized arc-length
positions along the referenced edge.

**[GAR-SEAM-006]** The canonical `SeamSpec` Rust definition is:

```rust
// tailor-solver/src/spec.rs

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
    /// Gathering ratio = from_length / to_length. 1.0 = flat seam. (0.0, 20.0].
    /// CANONICAL field name — supersedes `ratio` from research file 10/15.
    pub gather_ratio: f32,
}
```

---

##### <N>.<i>.7 Darts and Pleats

**[GAR-DART-001]** A dart (wedge removal for 3D panel shaping) MUST be represented by `DartSpec`
with `panel_id`, `tip_vertex: u32` (index into the panel's vertex array), `opening_edges: [u32; 2]`
(indices into the panel's edge array), and `depth_cm: f32`. The solver MUST add zero-rest-length
distance constraints between the two dart edge vertex sequences.

**[GAR-PLEAT-001]** A pleat MUST be represented by `PleatSpec` with `panel_id`, `kind: PleatKind`
(`knife | box | accordion`), `count: u32`, `depth_cm: f32`, `interval_cm: f32`, and
`fold_angle_deg: f32`. Pleats are derived panel mutations expanded at mesh-generation time.

```rust
// tailor-solver/src/spec.rs

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
```

---

##### <N>.<i>.8 Fabric Properties

**[GAR-FAB-001]** Fabric physical properties on `GarmentSpec` MUST be represented by
`FabricProperties` with all dimensionless parameters normalized to [0.0, 1.0] where 1.0 means
stiffest/most resistant. This is the LLM-facing surface (from T-CONTRACTS [T-CONTRACTS.garment-spec],
the `09` form). Raw anisotropic XPBD compliance values MUST NOT appear in `GarmentSpec`; they live
in `tailor_material_presets` and the solver crate only.

**[GAR-FAB-002]** The mapping from normalized [0,1] `FabricProperties` to raw XPBD compliance
MUST be non-linear (logarithmic, because compliance spans approximately `1e-9` to `1e-3`). This
mapping MUST be owned by the preset/decoder layer (governed by the fabric-models sub-section) and
applied when building the `SolverMesh` material buffer. It MUST NOT be stored twice.

**[GAR-FAB-003]** `FabricProperties::preset: Option<FabricPreset>` selects a named preset applied
first; explicit normalized fields override it on a per-field basis.

**[GAR-FAB-004]** `density_g_m2: f32` (mass per unit area in grams per square metre) and
`collision_thickness_mm: f32` are the two non-normalized fields in `FabricProperties`. They are
physically meaningful quantities an LLM can reason about directly. The `FABRIC_RANGE` blocking
check MUST enforce `density_g_m2 in [5, 2000]` and `collision_thickness_mm in [0.1, 5]`.

**[GAR-FAB-005]** The canonical `FabricProperties` Rust definition is:

```rust
// tailor-solver/src/spec.rs

/// Fabric properties, normalized [0,1]. Weft = cross-grain, Warp = grain direction.
/// 1.0 = stiffest/most resistant. Non-linear map to raw XPBD compliance owned by decoder layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Fabric properties, normalized [0,1]. Weft=cross-grain, Warp=grain.")]
pub struct FabricProperties {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<FabricPreset>,
    pub stretch_weft: f32,
    pub stretch_warp: f32,
    pub shear: f32,
    pub bending_weft: f32,
    pub bending_warp: f32,
    pub buckling_ratio: f32,
    /// Mass per unit area, grams per square metre. Range [5, 2000].
    pub density_g_m2: f32,
    /// Collision thickness in millimetres. Range [0.1, 5].
    pub collision_thickness_mm: f32,
    pub friction: f32,
    pub internal_damping: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FabricPreset {
    Cotton, Denim, Silk, Jersey, Leather, Satin, Linen, Wool, Spandex,
    Chiffon, Canvas, Rubber,
}
```

---

##### <N>.<i>.9 Avatar Binding

**[GAR-AVT-001]** Every `GarmentSpec` MUST carry `avatar: AvatarBinding` specifying which body
proxy the garment is fitted to and simulated against. `AvatarBinding.avatar_id` MUST reference a
row in `tailor_avatars` (the avatar identity authority table). The `AVATAR_BINDING` blocking check
enforces this FK existence gate before solver invocation.

**[GAR-AVT-002]** `AvatarBinding.measurements_cm: Option<BodyMeasurements>` MAY carry cm
measurement overrides for parametric bodies. These MUST be converted to mm at the boundary before
the solver capsule build reads them.

**[GAR-AVT-003]** The canonical `AvatarBinding` and `BodyMeasurements` definitions are:

```rust
// tailor-solver/src/spec.rs

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AvatarBinding {
    /// Body-proxy authority id (tailor_avatars.avatar_id). Form: "AVT-{uuid_v7}"
    /// or a built-in parametric body slug (e.g. "avatar1-smplx-default").
    pub avatar_id: String,
    /// Optional cm measurement overrides for parametric bodies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurements_cm: Option<BodyMeasurements>,
}

/// Body measurements in CENTIMETRES (LLM-facing).
/// The authority body-proxy stores the full 25-measurement set in MILLIMETRES
/// (tailor_body_proxies.proxy_json); this cm subset is converted at the API boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BodyMeasurements {
    pub height_cm: f32,
    pub bust_cm: f32,
    pub waist_cm: f32,
    pub hip_cm: f32,
    pub inseam_cm: f32,
}
```

---

##### <N>.<i>.10 Pattern-to-Mesh Pipeline

**[GAR-MESH-001]** The pattern-to-mesh pipeline MUST convert a `GarmentSpec` to a `SolverMesh`
(canonical name; supersedes `SolverMeshV1` from research file `03`) before solver invocation.
`SolverMesh` MUST be defined in `tailor-solver/src/mesh.rs`.

**[GAR-MESH-002]** For each `PanelSpec`, the pipeline MUST:
1. Sample vertices along each `EdgeSpec` at arc-length spacing not exceeding the configured
   particle distance (default 1.0 cm), using arc-length parameterization appropriate to the
   `EdgeShape` variant (linear for `Straight`, Bezier arc-length for `Quadratic`/`Cubic`,
   circular arc subdivision for `Arc`).
2. Run constrained Delaunay triangulation (CDT) over the panel polygon interior. The CDT
   implementation MUST use the `spade` crate (MIT-licensed, actively maintained).
3. Apply `PanelSpec::placement` to convert panel-local 2D vertices to 3D world positions.
4. Assign each vertex its panel ID and panel-local UV coordinates (normalized 2D position in
   panel space) for use by the UV-from-pattern pipeline.

**[GAR-MESH-003]** For each `SeamSpec`, the pipeline MUST:
1. Find matching boundary vertex sequences on the two stitched edges.
2. When `gather_ratio != 1.0`, resample both edge vertex sequences to equal count N using
   equal arc-length spacing, then emit N point-distance constraints with rest-length zero.
3. Store `gather_ratio` on each `SeamConstraintRecord` for compliance scaling by the solver.

**[GAR-MESH-004]** For each `DartSpec`, the pipeline MUST remove the dart wedge from the panel
mesh and add zero-rest-length distance constraints between the two dart edge vertex sequences.

**[GAR-MESH-005]** `SeamConstraintRecord` MUST carry `gather_ratio: f32` (matching the canonical
`SeamSpec` field name). The field name `ratio` is PROHIBITED.

**[GAR-MESH-006]** UV coordinates MUST be derived directly from the 2D panel-local vertex
positions before the 3D placement transform is applied. This ensures UV islands are exact
flattened pattern pieces. Fabric grain direction (`grain_angle_deg`) maps to UV island orientation.

**[GAR-MESH-007]** The `SolverMesh` and `SeamConstraintRecord` definitions are:

```rust
// tailor-solver/src/mesh.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverMesh {
    pub schema_id: String,           // "hsk.cloth.solver_request@1" (allowed cloth exception)
    pub garment_id: String,
    /// Flat [x, y, z, x, y, z, ...] in centimetres.
    pub vertex_positions: Vec<f32>,
    /// Flat [i0, i1, i2, ...] triangle indices.
    pub triangle_indices: Vec<u32>,
    pub vertex_panel_ids: Vec<String>,
    /// Per-vertex UV in panel-local space.
    pub vertex_uvs: Vec<[f32; 2]>,
    pub seam_constraints: Vec<SeamConstraintRecord>,
    pub material_map: std::collections::HashMap<String, FabricProperties>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeamConstraintRecord {
    pub vertex_a: u32,
    pub vertex_b: u32,
    /// Rest length in centimetres. 0.0 for closed seams.
    pub rest_length_cm: f32,
    /// Canonical field name matching SeamSpec::gather_ratio.
    pub gather_ratio: f32,
    pub kind: SeamKind,
}
```

**Note:** `SolverMesh` uses schema ID `hsk.cloth.solver_request@1` — the one allowed `hsk.cloth.*`
exception from T-CONTRACTS [T-CONTRACTS.schema-ids] for solver-crate-internal physics payloads
that never become Tailor-domain authority rows.

---

##### <N>.<i>.11 Three-Tier Parametric / LLM-Emittable Authoring

**[GAR-LLM-001]** The Tailor authoring surface MUST support three tiers of LLM authoring,
selectable by the model or operator. All three tiers MUST ultimately produce a valid `GarmentSpec`
in cm before sandbox invocation. No tier MAY bypass the sandbox-validation-promotion lifecycle.

**[GAR-LLM-002]** Tier 1 (Parametric / GarmentCodeRC-style) MUST accept a compact design-intent
payload with categorical `garment_type` and design-option fields plus a continuous parameter
vector (up to 76 f32 values, normalized [0,1] following the ChatGarment convention). A
`GarmentCodeRcDecoder` MUST decode this into a full cm `GarmentSpec` via per-category panel
template factories before storage. Tier 1 tokens average approximately 350; it is the preferred
tier for interactive dialogue.

**[GAR-LLM-003]** Tier 2 (Direct panel/seam JSON) MUST accept a `GarmentSpec`-compatible JSON
emitted directly by the model with panel vertices, edge shapes, and seam definitions. The model
MUST be prompted with the `schemars`-derived `inputSchema` so structural hallucination is
constrained. Pre-sandbox lightweight geometry checks MUST be applied before sandbox invocation
to surface obvious errors immediately in the model response.

**[GAR-LLM-004]** Tier 3 (Program synthesis) MUST accept a GarmentCode-compatible Python program
or Handshake DSL program that generates a `GarmentSpec` when executed. Tier 3 MUST achieve the
highest fidelity by eliminating numerical hallucination via program execution. The program is
executed in a sandboxed context; its output MUST be a cm `GarmentSpec`.

**[GAR-LLM-005]** The `TailorAuthoringOutput` enum MUST represent all three tiers:

```rust
// handshake_core/src/tailor/model_adapter.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tier", rename_all = "snake_case")]
pub enum TailorAuthoringOutput {
    Parametric {
        garment_type: String,
        design_options: serde_json::Value,
        /// Normalized [0,1] floats; max 76. Decoded to cm GarmentSpec before storage.
        continuous_params: Vec<f32>,
    },
    PanelJson {
        garment_spec: GarmentSpec,
    },
    Program {
        program_text: String,
        program_kind: String,  // "garmentcode_python" | "handshake_dsl"
    },
}
```

**[GAR-LLM-006]** The model-facing MCP tool `author_garment` MUST accept `TailorAuthoringOutput`,
decode it to a cm `GarmentSpec`, write a `draft` row to `tailor_garments`, emit
`TailorGarmentDraftProposed` to the EventLedger, and return a `SimulationReceipt`
(schema ID `hsk.tailor.simulation_receipt@1`) as MCP `structuredContent`.

---

##### <N>.<i>.12 GarmentCode Interop

**[GAR-GC-001]** `GarmentSpec` MUST round-trip to and from GarmentCode JSON (`"units": "cm"`)
to enable interop with ChatGarment, AIpparel, NGL-Prompter, and Design2GarmentCode toolchains.
The `GARMENTCODE_ROUNDTRIP` advisory check (see [GAR-VALIDATION]) validates fidelity post-drape.

**[GAR-GC-002]** `handshake_core::tailor` MUST expose conversion functions:

```rust
// handshake_core/src/tailor/garment_interop.rs

impl GarmentSpec {
    /// Convert FROM GarmentCode JSON (ETH Zurich open format, "units": "cm").
    /// Panels from v["pattern"]["panels"]; stitches from v["pattern"]["stitches"].
    pub fn from_garmentcode_json(v: &serde_json::Value) -> Result<Self, ConversionError>;

    /// Convert TO GarmentCode JSON.
    /// Panel list -> GarmentCode "panels" dict keyed by panel_id.
    /// Seam list -> GarmentCode "stitches" list. properties.units = "cm".
    pub fn to_garmentcode_json(&self) -> serde_json::Value;

    /// Convert FROM ChatGarment GarmentCodeRC compact JSON (Tier-1 decode path).
    /// Requires a GarmentCodeRcDecoder that maps garment_type -> panel template factory.
    pub fn from_garmentcode_rc(
        v: &serde_json::Value,
        decoder: &GarmentCodeRcDecoder,
    ) -> Result<Self, ConversionError>;
}
```

**[GAR-GC-003]** The `GarmentCodeRcDecoder` MUST implement panel template factories for at least
the following garment categories in v1: `tshirt`, `shirt`, `dress`, `pants`, `skirt`, `jacket`,
`hood`, `sleeve`. Each factory maps (design_options, continuous_params) to a valid `GarmentSpec`
in cm. Exotic categories not covered by the v1 factory set MAY produce lossy round-trips.

**[GAR-GC-004]** The GarmentCode JSON `"curvature": null` edge form MUST map to
`EdgeShape::Straight`. The `"curvature": {"type": "quadratic", "control": [x, y]}` form MUST map
to `EdgeShape::Quadratic { control_cm: Vec2Cm { x, y } }`. All coordinates MUST remain in cm
throughout the conversion without normalization.

---

##### <N>.<i>.13 CRDT Collaborative Editing

**[GAR-CRDT-001]** Each `GarmentSpec` MUST have a corresponding CRDT document keyed by
`garment_id`, linked via the `tailor_garment_crdt_docs` table. Collaborative panel vertex
edits, edge curvature changes, and seam definition mutations MUST arrive as
`TailorPanelCrdtUpdateRecorded` events on the EventLedger using the existing
`CrdtUpdateRecordV1` schema. The Tailor module MUST NOT introduce new CRDT infrastructure;
it MUST reuse `kernel_crdt_updates`, `CrdtUpdateRecordV1`, and the `yjs_bridge`
serialization for Yjs-compatible delta encoding.

**[GAR-CRDT-002]** Conflict resolution for concurrent panel geometry edits MUST use
last-writer-wins per vertex index (vertex index is stable within a panel version) and
per edge index. Seam definition additions MUST use set-union; seam deletions MUST use
an explicit tombstone.

**[GAR-CRDT-003]** Model-proposed panel mutations MUST arrive as proposals via
`TailorPanelAiEditProposalRecorded` (not as immediate authority writes) and be surfaced to
the operator via `TailorPanelAiEditProposalDecided` before promotion. This is consistent
with the existing `ai_edit_proposal` submodule pattern in the kernel CRDT layer.

**[GAR-CRDT-004]** The bidirectional 2D-to-3D loop (drape output feeding edge-length corrections
back to panel vertices as CRDT deltas) is a deferred milestone. The v1 implementation MUST
deliver unidirectional (panel to 3D drape) only. The flatten/unfurl pass MUST be planned but
MUST NOT block v1 promotion.

---

##### <N>.<i>.14 Sandbox-to-Promotion Lifecycle

**[GAR-PROM-001]** Every model-authored or operator-authored garment MUST follow the lifecycle:
`draft` → `sandbox_pending` → `simulated` → `validated` → (`promoted` | `rejected`). Status
transitions MUST be recorded as EventLedger events per [GAR-EVENTS]. Direct writes to
`tailor_garments.status = 'promoted'` without a passing `TailorGarmentValidationRecorded` event
are PROHIBITED.

**[GAR-PROM-002]** The `TailorSandboxAdapter` MUST implement the kernel `SandboxAdapter` trait.
Its `run()` method MUST: deserialize `GarmentSpec` from the sandbox run artifact refs;
invoke `ClothSolver::triangulate()` to produce `SolverMesh`; invoke `ClothSolver::drape()`
to produce the draped mesh; write the mesh artifact bundle to the sandbox workspace; and
return `AdapterRunOutcome::Completed` with artifact refs.

**[GAR-PROM-003]** Every `guard_authority_write(AuthorityMode::Postgres)` call (the `no_sqlite_tripwire`)
MUST be made before any INSERT or UPDATE to `tailor_garments` or any other `tailor_*` table.
SQLite writes to Tailor tables are PROHIBITED.

**[GAR-PROM-004]** The storage glue for garment draft insertion MUST emit a `TailorGarmentDraftProposed`
EventLedger event with `aggregate("tailor_garment", &spec.garment_id)` and an idempotency key of
`"garment-draft-{garment_id}"` before returning. No draft row MUST be persisted without a
corresponding EventLedger receipt.

**[GAR-PROM-005]** Promotion equivalence for the re-run determinism check MUST use
`MeshComparator::compare(a, b, epsilon_mm)` with default `epsilon_mm = 0.1`. SHA-256
`content_hash` comparison for cross-backend promotion equivalence is PROHIBITED.
`content_hash` is kept only for same-machine idempotency and EventLedger fingerprinting.
See T-CONTRACTS [T-CONTRACTS.determinism] for the full canonical resolution.

**[GAR-PROM-006]** `PromotionGate::evaluate()` MUST use `ValidationReport::aggregate_blocks_promotion()`:
any `Blocking` validation finding prevents promotion. The gate MAY be configured with
`treat_advisory_as_blocking = true` for stricter runs.

---

##### <N>.<i>.15 Postgres Authority Tables

**[GAR-TABLE-001]** The canonical Postgres tables for garment authoring are `tailor_garments`
and `tailor_garment_crdt_docs`. Both MUST use `TEXT PRIMARY KEY` with prefixed string IDs
(`GAR-` and `CRDT-GAR-` respectively). The `UUID PRIMARY KEY DEFAULT gen_random_uuid()` form
is PROHIBITED (T-CONTRACTS FACT-4).

**[GAR-TABLE-002]** `tailor_garments` MUST include the columns specified in T-CONTRACTS
[T-CONTRACTS.tables]: `garment_id TEXT PRIMARY KEY`, `workspace_id TEXT NOT NULL`,
`name TEXT NOT NULL`, `status TEXT NOT NULL` (CHECK domain:
`draft | sandbox_pending | simulated | validated | promoted | rejected | archived`),
`spec_json JSONB NOT NULL` (the `GarmentSpec` JSONB), `animation_json JSONB` (nullable;
T-ANIMATION sub-section), `body_proxy_id TEXT`, `wardrobe_id TEXT`,
`promotion_receipt_id TEXT`, `event_ledger_event_id TEXT NOT NULL`,
`created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`, `updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`.

**[GAR-TABLE-003]** Migrations for Tailor tables MUST use the dated naming convention
`migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.sql` with a required
`migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.down.sql` reverse pair. Numbered `0NNN_*` migrations
are PROHIBITED for all Tailor tables (T-CONTRACTS [T-CONTRACTS.migration-naming] FACT-2/FACT-3).
The garment authoring migration file MUST be named `<date>_tailor_garments.sql`.

**[GAR-TABLE-004]** `tailor_material_presets` (not `tailor_material_library`, not `tailor_material`)
is the canonical name for the fabric preset authority table (T-CONTRACTS [T-CONTRACTS.tables]).

---

##### <N>.<i>.16 Schema IDs

**[GAR-SCHEMA-IDS-001]** The canonical schema ID constants for garment authoring are in
`handshake_core/src/tailor/schemas.rs`. The namespace MUST be `hsk.tailor.*`, not `hsk.cloth.*`.
The one allowed `hsk.cloth.*` exception is solver-crate-internal physics payloads
(`hsk.cloth.solver_request@1`, `hsk.cloth.solver_result@1`) that never become authority rows.

```rust
// handshake_core/src/tailor/schemas.rs

pub const SCHEMA_TAILOR_GARMENT_SPEC_V1:    &str = "hsk.tailor.garment_spec@1";
pub const SCHEMA_TAILOR_MATERIAL_PRESET_V1: &str = "hsk.tailor.material_preset@1";
pub const SCHEMA_TAILOR_AVATAR_V1:          &str = "hsk.tailor.avatar@1";
pub const SCHEMA_TAILOR_BODY_PROXY_V1:      &str = "hsk.tailor.body_proxy@1";
pub const SCHEMA_TAILOR_SIM_RECEIPT_V1:     &str = "hsk.tailor.simulation_receipt@1";

// Allowed hsk.cloth.* exception — solver-crate-internal physics payloads only:
pub const SCHEMA_CLOTH_SOLVER_REQUEST_V1:   &str = "hsk.cloth.solver_request@1";
pub const SCHEMA_CLOTH_SOLVER_RESULT_V1:    &str = "hsk.cloth.solver_result@1";
```

---

##### <N>.<i>.17 EventLedger Events

**[GAR-EVENTS-001]** The following `KernelEventType` variants MUST be added to `kernel/mod.rs`
and registered in `required_first_slice_events()` for the garment authoring domain. Variant names
are `Tailor*` PascalCase; wire strings via `as_str()` are `TAILOR_*` SCREAMING_SNAKE_CASE
(T-CONTRACTS FACT-1). All superseded names from the research package are PROHIBITED.

```rust
// kernel/mod.rs — canonical garment-authoring EventLedger variants

// Garment lifecycle
TailorGarmentDraftProposed,       // "TAILOR_GARMENT_DRAFT_PROPOSED"
TailorGarmentDraftUpdated,        // "TAILOR_GARMENT_DRAFT_UPDATED"
TailorGarmentValidationRecorded,  // "TAILOR_GARMENT_VALIDATION_RECORDED"
TailorGarmentPromoted,            // "TAILOR_GARMENT_PROMOTED"
TailorGarmentPromotionRejected,   // "TAILOR_GARMENT_PROMOTION_REJECTED"

// Simulation run lifecycle
TailorSimRunRequested,            // "TAILOR_SIM_RUN_REQUESTED"
TailorSimRunStarted,              // "TAILOR_SIM_RUN_STARTED"
TailorSimRunCompleted,            // "TAILOR_SIM_RUN_COMPLETED"
TailorSimRunRejected,             // "TAILOR_SIM_RUN_REJECTED"

// CRDT collaborative editing
TailorPanelCrdtUpdateRecorded,    // "TAILOR_PANEL_CRDT_UPDATE_RECORDED"
TailorPanelCrdtSnapshotRecorded,  // "TAILOR_PANEL_CRDT_SNAPSHOT_RECORDED"
TailorPanelAiEditProposalRecorded,// "TAILOR_PANEL_AI_EDIT_PROPOSAL_RECORDED"
TailorPanelAiEditProposalDecided, // "TAILOR_PANEL_AI_EDIT_PROPOSAL_DECIDED"
TailorCrdtConflictDetected,       // "TAILOR_CRDT_CONFLICT_DETECTED"
```

**[GAR-EVENTS-002]** Superseded event variant names MUST NOT be used:
`TailorGarmentValidated` (03/04) is superseded by `TailorGarmentValidationRecorded`;
`TailorGarmentCrdtUpdateRecorded` (03/04) and `TailorCrdtUpdateRecorded` (09) are superseded by
`TailorPanelCrdtUpdateRecorded`. The `GARMENT_*` wire strings without the `TAILOR_` prefix (09)
are PROHIBITED.

**[GAR-EVENTS-003]** The `event_family` constant for garment events MUST be `"tailor.garment"`,
defined in `handshake_core/src/tailor/event_family.rs`:

```rust
pub const TAILOR_GARMENT:    &str = "tailor.garment";
pub const TAILOR_SIMULATION: &str = "tailor.simulation";
pub const TAILOR_PANEL_CRDT: &str = "tailor.panel.crdt";
```

---

##### <N>.<i>.18 Validation Checks

**[GAR-VALIDATION-001]** The `TailorValidationDescriptor` MUST implement the following checks for
garment authoring. Each check has a stable `code` for model self-correction. Severity is
`Blocking` (prevents promotion) or `Advisory` (recorded, does not block unless
`treat_advisory_as_blocking = true`). The `WINDING` check auto-corrects silently before
emitting an advisory.

**Fast pre-solver checks (no solver invocation required, target < 100ms):**

| Code | Severity | Assertion |
|---|---|---|
| `PANEL_CLOSURE` | Blocking | Each panel polygon is a closed non-self-intersecting loop |
| `SEAM_EDGE_REF` | Blocking | Every `SeamSpec.from/to` references a valid `panel_id` + `edge_index` |
| `GATHER_RATIO_RANGE` | Blocking | Every `SeamSpec.gather_ratio` in (0.0, 20.0] |
| `FABRIC_RANGE` | Blocking | Normalized `FabricProperties` fields in [0.0, 1.0]; `density_g_m2` in [5, 2000]; `collision_thickness_mm` in [0.1, 5] |
| `AVATAR_BINDING` | Blocking | `AvatarBinding.avatar_id` exists in `tailor_avatars` |
| `MIN_PANEL_AREA` | Blocking | Every panel area > 1.0 cm^2 |
| `WINDING` | Advisory | Panel vertices counter-clockwise (auto-corrected; INFO if fixed) |

**Mesh-quality checks (on triangulated `SolverMesh`, pre-simulation):**

| Code | Severity | Assertion |
|---|---|---|
| `MESH_TOPOLOGY` | Blocking | Manifold; no degenerate triangles; no open boundary except intended seam edges |
| `MESH_TRIANGLE_QUALITY` | Blocking | Min triangle angle >= 10 degrees; max aspect ratio <= 20 |
| `PANEL_OVERLAP` | Advisory | No two panels occupy the same 3D region before draping |

**Post-simulation checks:**

| Code | Severity | Assertion |
|---|---|---|
| `MESH_NOT_EMPTY` | Blocking | Simulated vertex buffer non-empty |
| `NO_DEGENERATE_TRIS` | Blocking | No zero-area triangles in output mesh |
| `SEAMS_CLOSED` | Blocking | Every seam constraint pair <= 1 mm separation at rest |
| `NO_INTERPENETRATION` | Blocking | No cloth particle deeper than -0.5 mm inside any body capsule/sphere (final frame only) |
| `SELF_INTERSECTION` | Advisory | Self-collision pair count below mesh-explosion limit |
| `UV_COVERAGE` | Blocking | UV islands cover >= 95% of mesh surface |
| `UV_VALIDITY` | Blocking | All UVs in [0,1]^2; no degenerate UV triangles (area > 1e-6) |
| `DRAPE_CONVERGED` | Advisory | Final kinetic energy below threshold |
| `PANEL_COUNT_MATCH` | Advisory | Simulated panel count == spec panel count |
| `GARMENTCODE_ROUNDTRIP` | Advisory | Spec round-trips to GarmentCode JSON without loss |

**[GAR-VALIDATION-002]** `MeshComparator::compare(a, b, epsilon_mm)` with default
`epsilon_mm = 0.1` MUST be used for re-run determinism comparison in the validation runner.
This function MUST live in `tailor-solver/src/compare.rs` and MUST be reused by the kernel
validation runner via the `ClothSolver` trait boundary. See T-CONTRACTS
[T-CONTRACTS.determinism] for the full comparator contract including secondary topology invariants.

---

##### <N>.<i>.19 Module and Crate Layout

**[GAR-LAYOUT-001]** The `handshake_core::tailor` module MUST be structured as follows:

```text
src/backend/handshake_core/src/tailor/
    mod.rs              # pub module declarations + TailorEngineError
    event_family.rs     # Tailor EventLedger event_family constants
    schemas.rs          # hsk.tailor.* schema ID constants
    garment_interop.rs  # GarmentSpec::from/to_garmentcode_json, from_garmentcode_rc
    model_adapter.rs    # TailorAuthoringOutput, TailorAuthoringContext
    sandbox_adapter.rs  # TailorSandboxAdapter implementing SandboxAdapter trait
    validation.rs       # TailorValidationDescriptor; all check codes from [GAR-VALIDATION]
    storage_glue.rs     # Postgres CRUD + EventLedger emissions for tailor_garments
    api.rs              # Axum Router: GET/POST /tailor/garments/*
```

**[GAR-LAYOUT-002]** The `tailor-solver` Cargo workspace crate MUST have no dependency on
`handshake_core`. It MUST expose `ClothSolver` as its public trait. Physics terminology (`Cloth*`,
`ClothSolver`, `ClothBodyProxy`) is retained in this crate. Feature identifiers and event names
remain `Tailor*` in the kernel module.

```text
tailor-solver/
    Cargo.toml          # wgpu, bytemuck, glam, parry3d; no sqlx, no tauri
    src/
        lib.rs          # pub trait ClothSolver: Send + Sync
        spec.rs         # GarmentSpec (canonical) + all nested types
        mesh.rs         # SolverMesh, triangulation pipeline
        compare.rs      # MeshComparator::compare(a, b, epsilon_mm)
        body/
            proxy.rs    # ClothBodyProxy, CollisionCapsule, CollisionSphere
        xpbd/
            mod.rs
            constraint_stretch.wgsl
            constraint_bend.wgsl
            constraint_seam.wgsl
            collision.wgsl
        gpu_solver.rs   # WgpuClothSolver implementing ClothSolver
        cpu_solver.rs   # CpuClothSolver fallback
```

**[GAR-LAYOUT-003]** The `ClothSolver` trait MUST expose at minimum:

```rust
// tailor-solver/src/lib.rs
pub trait ClothSolver: Send + Sync {
    fn triangulate(&self, spec: &GarmentSpec) -> Result<SolverMesh, SolverError>;
    fn drape(
        &self,
        mesh: &SolverMesh,
        body_proxy: &ClothBodyProxy,
        params: &SimRunParams,
    ) -> Result<DrapedMesh, SolverError>;
    fn flatten(&self, draped: &DrapedMesh) -> Result<Vec<PanelFlattenResult>, SolverError>;
    fn compare(
        &self,
        a: &DrapedMesh,
        b: &DrapedMesh,
        epsilon_mm: f32,
    ) -> MeshComparatorResult;
}
```

---

##### <N>.<i>.20 Non-Normative Provenance

The following research files are non-normative for all type names, field names, units, event
variants, schema IDs, table names, and migration names. They remain valid as design rationale,
OSS evidence, and implementation sketches. Where any research file conflicts with this sub-section,
this sub-section wins.

- `03-garment-authoring.md` (T-GARMENT-AUTHORING) — primary source for 2D pattern/seam
  algorithms, GarmentCode interop, CRDT editing approach, seam constraint encoding, and the
  three-tier LLM authoring concept.
- `16-contracts.md` (T-CONTRACTS) — the canonical contract authority this sub-section
  directly mirrors for all contract surfaces.
- ChatGarment (CVPR 2025, arxiv:2412.17811) — GarmentCodeRC 76-float parametric vector
  and normalized fabric properties basis.
- GarmentCode (ETH Zurich, SIGGRAPH Asia 2023) — panel/edge/stitch JSON format, `"units": "cm"`
  round-trip target, GarmentCodeData triangulation pipeline.
- AIpparel (CVPR 2025, arxiv:2412.03937) — sewing pattern tokenizer and direct panel JSON
  LLM fine-tuning evidence.
- Design2GarmentCode (CVPR 2025, arxiv:2412.08603) — program synthesis tier evidence
  (100% simulation success rate).
- Dress-1-to-3 (arxiv:2502.03449) — quadratic Bezier panel representation and inverse
  flatten/unfurl approach.

## 13.7 Auto-Fit & Retargeting Across Body Morphs

> **Heading placeholder.** KERNEL_BUILDER renumbers on assembly.
> Sub-section id: `autofit` | source research: `07-autofit-retargeting.md` (non-normative) |
> canonical contract authority: `16-contracts.md` (T-CONTRACTS).

---

### Normative Requirements

#### Overview and Scope

The Tailor module MUST provide a first-class garment retargeting subsystem that fits a
`GarmentSpec` authored against one `tailor_avatars` body onto a different target avatar without
requiring the operator or model to re-author the garment from scratch.

The retargeting subsystem MUST be implemented entirely within `handshake_core::tailor`
(`src/tailor/`) and the `tailor-solver` standalone crate. It MUST NOT depend on any external
garment application, OS-level process bridge, or SQLite path. All authority writes MUST call
`guard_authority_write(AuthorityMode::Postgres)` before touching any `tailor_*` table.

All schema IDs used in this sub-section are in the `hsk.tailor.*` namespace
(`SCHEMA_TAILOR_REFIT_REQUEST_V1 = "hsk.tailor.refit_request@1"`). EventLedger variants use
the `TAILOR_REFIT_*` wire strings defined in T-CONTRACTS [T-CONTRACTS.event-types]. Table PKs
use the `TEXT PRIMARY KEY` `RFT-{uuid_v7}` prefix convention
[T-CONTRACTS.tables][T-CONTRACTS.migration-naming].

---

#### §7.1 Avatar and Body-Proxy Authority

##### §7.1.1 Avatar Identity Table

The `tailor_avatars` table (defined canonically in T-CONTRACTS [T-CONTRACTS.body-proxy]) is the
identity authority for every body a garment can be fitted to. `AvatarBinding.avatar_id` in
`GarmentSpec` MUST reference a row in `tailor_avatars`, not any other table.

The Tailor module MUST support `source_kind = 'avatar1_2d_derived'` to bridge the operator's
existing Avatar1 / ComfyUI 2D pipeline into a 3D body proxy without requiring a full SMPL-X
mesh import.

##### §7.1.2 Body Proxy Collision Geometry

The `tailor_body_proxies` table stores solver collision geometry for an avatar
[T-CONTRACTS.body-proxy]. One avatar MAY have multiple proxy rows (e.g., a standard capsule-chain
proxy and a multi-sphere large-bust proxy).

Body proxy geometry MUST be stored and operated in **millimetres** throughout the authority layer
and the solver crate. The LLM-facing `BodyMeasurements` struct (in `GarmentSpec`) uses
centimetres; the conversion boundary MUST be at the API decode step, before any proxy row is
written.

The canonical Rust authority type is `ClothBodyProxy` in `tailor-solver/src/body/proxy.rs`:

```rust
// tailor-solver/src/body/proxy.rs
// All lengths and radii in MILLIMETRES.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClothBodyProxy {
    pub body_proxy_id: String,   // "BPX-{uuid_v7}"
    pub avatar_id: String,       // "AVT-{uuid_v7}"
    /// Capsule chain approximating body segments. Lengths in mm.
    pub capsules: Vec<CollisionCapsule>,
    /// Sphere sub-proxies for breast/bust sub-volumes and joint spheres. mm.
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

The GPU upload types `GpuCapsule` / `GpuSphere` (fixed max 32 capsules + 16 spheres,
`bytemuck::Pod`) in the solver crate are the runtime representation. The `ClothBodyProxy`
authority record MUST be serializable to and from these fixed-size GPU arrays.

`mode` on `tailor_body_proxies` MUST be one of `capsule | capsule_sphere | capsule_sdf | sdf`.
For non-humanoid avatars (`source_kind = 'non_humanoid'`), the Tailor module MUST accept an
operator-supplied `proxy_json` and MUST NOT require an auto-generated capsule chain.

##### §7.1.3 Body Measurement Extraction

The Tailor module MUST include a body measurement extractor in the `tailor-solver` crate
(`src/body/measurements.rs`) that accepts a body mesh `TriMesh` and emits a 25-field
anthropometric measurement map (keys follow GarmentMeasurements naming:
`bust_circ_mm`, `waist_circ_mm`, `hip_circ_mm`, `shoulder_width_mm`, `arm_length_mm`,
`inseam_mm`, etc.).

```rust
// tailor-solver/src/body/measurements.rs
/// Extract 25 standard anthropometric measurements from a body mesh.
/// Measurement planes are allowed ±20 mm from their initial position
/// to find the local extremum for each measurement (GarmentCodeData fit-aware method).
/// Returns BTreeMap keyed by GarmentMeasurements field names, values in mm.
pub fn extract_measurements(mesh: &TriMesh) -> Result<BTreeMap<String, f32>, MeasurementError>;
```

The extractor MUST validate that the input mesh is manifold and has a single connected component
before writing a `tailor_avatars` row; it MUST return a descriptive `MeasurementError` on invalid
mesh topology rather than silently producing bad measurements.

Measurement extraction MUST emit `TailorAvatarMeasurementsExtracted`
(`"TAILOR_AVATAR_MEASUREMENTS_EXTRACTED"`) with `event_family = "tailor.avatar"` on the
EventLedger upon success.

---

#### §7.2 Refit Modes

The retargeting engine MUST implement exactly three refit modes, represented as a typed enum in
`src/tailor/refit.rs`:

```rust
// src/tailor/refit.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum RefitMode {
    /// Re-drape the existing GarmentSpec panels unchanged on the target body.
    /// Used when body proportions are close (pose change, minor morph delta).
    /// Executes one three-pass XPBD forward simulation; no pattern modification.
    RedrapeOnly {
        target_avatar_id: String,   // "AVT-{uuid_v7}"
        progressive_drape: bool,
    },
    /// Scale each panel from measurement ratios (source → target), then re-drape.
    /// Used for proportionally similar bodies with different overall size.
    /// Emits TailorRefitPatternScaled before drape starts.
    ScaleAndRedrape {
        target_avatar_id: String,
        ease_overrides: Option<EaseOverrideMap>,
    },
    /// Full differentiable optimization: gradient descent through XPBD to minimize
    /// the DressAnyone composite loss (shape matching + boundary curvature +
    /// seam-length parity + panel area). Offline; timeout >= 1800 s required.
    /// Used for significantly different body shapes or non-humanoid avatars.
    OptimizePatterns {
        target_avatar_id: String,
        max_iterations: u32,
        convergence_threshold: f32,
        /// When true, gather_ratio on each SeamSpec is preserved unchanged;
        /// only panel rest-lengths scale. When false, the optimizer may adjust
        /// gather ratios within (0.0, 20.0] bounds.
        preserve_seam_ratios: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EaseOverrideMap {
    /// Ease added per body region in mm. Keys: "bust", "waist", "hip",
    /// "shoulder", "arm", "inseam".
    pub ease_mm: BTreeMap<String, f32>,
}
```

The Tailor module MUST NOT expose a fourth mode that bypasses the three-pass progressive drape
or the validation gate.

---

#### §7.3 Pattern Grading — ScaleAndRedrape Path

##### §7.3.1 Panel Scale Computation

For `RefitMode::ScaleAndRedrape`, the engine MUST compute per-panel scale factors from
measurement ratios between source and target body proxies before the drape step begins:

```rust
// src/tailor/refit.rs
/// Compute per-panel scale factors from source and target body measurements.
/// Returns (width_scale, height_scale) keyed by panel_id.
/// Horizontal scale is driven by circumference measurements (bust, waist, hip).
/// Vertical scale is driven by inseam, torso height, arm length.
/// Each panel is classified by body region and receives the appropriate ratio pair.
pub fn compute_panel_scales(
    source_measurements_mm: &BTreeMap<String, f32>,
    target_measurements_mm: &BTreeMap<String, f32>,
    ease_overrides: &EaseOverrideMap,
    panels: &[PanelSpec],
) -> Result<BTreeMap<String, PanelScale>, RefitError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PanelScale {
    pub width_scale: f32,
    pub height_scale: f32,
}
```

##### §7.3.2 Seam Ratio Invariance Under Scaling

When scaling panels, the engine MUST preserve `gather_ratio` on every `SeamSpec` unchanged.
Only the panel rest-lengths change; the seam constraint graph topology MUST remain identical to
the source `GarmentSpec`. This prevents spurious gather collapse on large-body targets.

##### §7.3.3 Derived Draft Event

After scale computation, the engine MUST write a new `tailor_garments` row (a derived draft,
`status = 'sandbox_pending'`) and MUST emit `TailorRefitPatternScaled`
(`"TAILOR_REFIT_PATTERN_SCALED"`, `event_family = "tailor.refit"`) before the drape sandbox
run is created.

---

#### §7.4 Progressive Re-Drape Initialization

All three refit modes converge on a re-drape step. The engine MUST use a three-pass progressive
relaxation strategy implemented as `RefitDrapeStrategy` in `tailor-solver/src/refit/drape_init.rs`:

```rust
// tailor-solver/src/refit/drape_init.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefitDrapeStrategy {
    /// Pass 1: gravity-free stitch pass. Panels held at body-surface centroids;
    /// stretch + seam constraints enforced without gravity. Default: 50 substeps.
    pub gravity_free_substeps: u32,
    /// Pass 2: stiffened physics pass. Gravity + collision active; stretch and
    /// bending stiffness multiplied by this factor. Default: 10.0.
    pub stiffened_damping_factor: f32,
    /// Pass 2 convergence: fraction of vertices still moving to consider settled.
    /// Default: 0.05 (5%).
    pub stiffened_convergence_pct: f32,
    /// Pass 3: full production parameters. Convergence fraction. Default: 0.015.
    pub full_physics_convergence_pct: f32,
    /// Pass 3 hard cap. Default: 2400 frames.
    pub max_frames: u32,
    /// Wall-clock timeout across all three passes. Default: 300.0 s for
    /// RedrapeOnly and ScaleAndRedrape; MUST be >= 1800.0 s for OptimizePatterns.
    pub timeout_secs: f32,
}
```

Pass 1 (gravity-free stitch) MUST be executed before any collision detection is active, so that
panels settle into approximate closed form without being repelled by the body proxy.

Pass 2 MUST apply the body proxy collision geometry (capsules + spheres) with full XPBD position
correction before relaxing stiffness in Pass 3.

Pass 3 MUST run to the `full_physics_convergence_pct` threshold or `max_frames`, whichever
comes first. If `max_frames` is reached without convergence, the run MUST record
`converged = false` in the result bundle; the `REFIT_CONVERGED` advisory validation check will
surface this condition (see §7.6).

---

#### §7.5 UV and Texture Preservation

##### §7.5.1 ARAP Re-Unfurl

After a successful re-drape simulation, the engine MUST recompute UV islands for every panel
using an as-rigid-as-possible (ARAP) energy minimization unfurl pass in
`tailor-solver/src/uv/unfurl.rs`:

```rust
// tailor-solver/src/uv/unfurl.rs
/// Flatten a simulated 3D panel to 2D UV space using ARAP energy minimization.
/// Boundary vertices are pinned at their pre-simulation 2D pattern positions.
/// Interior UVs are computed by minimizing: sum_triangles ||J_k - R_k||_F^2
/// (alternating local SVD + global sparse linear solve).
/// Output: UV coordinates per vertex in pattern space [0,1]^2.
pub fn arap_unfurl_panel(panel: &Panel3D) -> Result<Vec<[f32; 2]>, UnfurlError>;
```

Boundary vertices MUST be pinned at their original 2D panel positions. This keeps the UV island
boundary unchanged after retargeting, so texture maps authored against the source garment remain
valid at panel edges.

The engine MUST emit `TailorRefitUvRecomputed` (`"TAILOR_REFIT_UV_RECOMPUTED"`,
`event_family = "tailor.refit"`) after all panels have been unfurled.

##### §7.5.2 Graphic Layer Anchor Preservation

For panels that carry graphic layers (`tailor_graphic_layers` rows with `boundary_pinned = true`),
the ARAP unfurl MUST treat graphic-anchor vertices as additional pinned points. Only
non-anchored interior UV positions are adjusted.

If panel deformation exceeds 30% area change relative to source, the engine MUST emit a
`UV_VALIDITY` advisory finding (see §7.6) so the operator can review graphic placement.

---

#### §7.6 Refit Validation Gate

Refit output MUST pass the `TailorValidationDescriptor` before the `PromotionGate` accepts the
derived garment. The applicable checks from the canonical ValidationDescriptor catalog
[T-CONTRACTS.validation] are:

```
CHECK                  Severity  Assertion
---------------------  --------  -------------------------------------------------------
REFIT_INTERSECTION_FREE Blocking  min(particle-capsule/sphere distance) >= -0.5 mm
                                  (final simulation frame; intermediate substeps skipped)
REFIT_SEAM_CLOSURE     Blocking   mated seam edge-pair length difference < 1%
UV_VALIDITY            Blocking   all recomputed UVs in [0,1]^2; no degenerate UV tris
                                  (area > 1e-6 in UV space); >=95% mesh UV coverage
MESH_TOPOLOGY          Blocking   output mesh: manifold, no degenerate triangles,
                                  no isolated vertices
REFIT_CONVERGED        Advisory   simulation reached full_physics_convergence_pct
                                  within max_frames (not timed out)
```

A refit run with any `Blocking` failure MUST be rejected: `status` on `tailor_refit_runs`
is set to `'rejected'` and `TailorRefitRejected` (`"TAILOR_REFIT_REJECTED"`,
`event_family = "tailor.refit"`) is emitted. `Advisory` findings are recorded but MUST NOT
block promotion unless `PromotionGateInputs.treat_advisory_as_blocking = true`.

Promotion equivalence between the source and output garment meshes MUST use `MeshComparator`
with the default `epsilon_mm = 0.1` [T-CONTRACTS.determinism], not SHA-256 hash comparison.

---

#### §7.7 Blend-Shape Incremental Refit

The engine MUST support incremental refit for blend-shape / morph-target avatars, where the body
proxy geometry changes along a continuous parameter `blend_t ∈ [0.0, 1.0]`:

```rust
// src/tailor/refit.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BlendShapeRefitRequest {
    pub garment_id: String,           // "GAR-{uuid_v7}"
    pub base_body_proxy_id: String,   // "BPX-{uuid_v7}"
    pub target_body_proxy_id: String, // "BPX-{uuid_v7}"
    /// Interpolation: 0.0 = base proxy geometry, 1.0 = target proxy geometry.
    pub blend_t: f32,
    /// If true, warm-start the XPBD solver from the last equilibrium position
    /// of the base-body drape. Requires prior simulation output in cache.
    pub warm_start: bool,
}
```

When `warm_start = true`, the solver MUST initialize particle positions from the cached prior
equilibrium state rather than re-running the full three-pass progressive drape. The solver
MUST NOT warm-start if no prior result is cached; it MUST fall back to a full three-pass init
and MUST log a warning.

Blend-shape refit MUST still run the full refit validation gate (§7.6) on the output.

---

#### §7.8 Refit Sandbox Integration

Refit runs MUST execute inside the existing `SandboxAdapter` / `SandboxRunV1` lifecycle.
The `TailorSandboxAdapter` MUST implement `SandboxAdapter`:

```rust
// src/tailor/sandbox_adapter.rs
pub struct TailorRefitAdapter {
    solver: Arc<dyn ClothSolver>,
    refit_mode: RefitMode,
}

impl SandboxAdapter for TailorRefitAdapter {
    fn kind(&self) -> AdapterKind {
        AdapterKind::process_tier("tailor_refit_v1", "Tailor Refit Adapter")
    }

    fn run(
        &self,
        run: &SandboxRunV1,
        workspace: &SandboxWorkspaceV1,
        policy: &SandboxPolicyV1,
    ) -> Result<AdapterRunOutcome, AdapterError> {
        // 1. Deserialize RefitRequest (schema_id = "hsk.tailor.refit_request@1")
        //    from run.payload_json.
        // 2. Load GarmentSpec and source ClothBodyProxy from sandbox workspace.
        // 3. Dispatch to solver:
        //    a. compute_panel_scales()       — ScaleAndRedrape only
        //    b. RefitDrapeStrategy three-pass progressive relaxation
        //    c. XPBD forward sim to equilibrium
        //    d. arap_unfurl_panel() per panel — UV recompute
        //    e. Validate: REFIT_INTERSECTION_FREE, REFIT_SEAM_CLOSURE, UV_VALIDITY,
        //       MESH_TOPOLOGY, REFIT_CONVERGED
        // 4. Bundle: garment mesh + UV map + validation findings + refit_run metadata.
        // 5. Return AdapterRunOutcome::Completed { artifact_refs: [bundle_ref] }
        //    or AdapterRunOutcome::Rejected { findings } on Blocking failure.
        todo!()
    }
}
```

For `RefitMode::OptimizePatterns`, `policy.timeout_secs` MUST be set to at least `1800.0`
seconds. The sandbox MUST reject (not panic) if the timeout is shorter and the mode is
`OptimizePatterns`.

##### §7.8.1 Refit Run Table

Refit lifecycle state MUST be persisted in `tailor_refit_runs`
[T-CONTRACTS.tables]:

```sql
-- Migration: 2026_MM_DD_tailor_refit_runs.sql  (dated at WP authoring time)
CREATE TABLE IF NOT EXISTS tailor_refit_runs (
    refit_run_id         TEXT PRIMARY KEY,           -- "RFT-{uuid_v7}"
    garment_id           TEXT NOT NULL,              -- source garment
    source_body_proxy_id TEXT REFERENCES tailor_body_proxies (body_proxy_id),
    target_body_proxy_id TEXT NOT NULL REFERENCES tailor_body_proxies (body_proxy_id),
    refit_mode           TEXT NOT NULL
        CHECK (refit_mode IN ('redrape_only','scale_and_redrape','optimize_patterns')),
    status               TEXT NOT NULL DEFAULT 'requested'
        CHECK (status IN ('requested','running','completed','validated',
                           'promoted','rejected')),
    sandbox_run_id       TEXT,                       -- FK kb003_sandbox_runs.run_id
    output_garment_id    TEXT,                       -- "GAR-{uuid_v7}" (promoted result)
    scale_factors_json   JSONB,                      -- PanelScale map (ScaleAndRedrape)
    optimization_params_json JSONB,                  -- OptimizePatterns params
    event_ledger_event_id TEXT NOT NULL,             -- FK kernel_event_ledger.event_id
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_refit_runs_garment
    ON tailor_refit_runs (garment_id);
CREATE INDEX IF NOT EXISTS ix_tailor_refit_runs_target_proxy
    ON tailor_refit_runs (target_body_proxy_id);
```

The migration filename MUST follow the dated convention (`2026_MM_DD_tailor_refit_runs.sql` +
`.down.sql`) and MUST NOT use a numbered `0NNN_*` prefix [T-CONTRACTS.migration-naming].

---

#### §7.9 EventLedger Contract

The following events from [T-CONTRACTS.event-types] govern the refit lifecycle. All use
`event_family = "tailor.refit"` (constant `TAILOR_REFIT` in `src/tailor/event_family.rs`).

| KernelEventType variant        | Wire string                      | Emitted when                                   |
|-------------------------------|----------------------------------|------------------------------------------------|
| `TailorRefitRequested`        | `"TAILOR_REFIT_REQUESTED"`       | Operator or model requests a refit             |
| `TailorRefitPatternScaled`    | `"TAILOR_REFIT_PATTERN_SCALED"`  | ScaleAndRedrape scale computation complete     |
| `TailorRefitDrapeCompleted`   | `"TAILOR_REFIT_DRAPE_COMPLETED"` | XPBD re-drape finished (all three passes)      |
| `TailorRefitUvRecomputed`     | `"TAILOR_REFIT_UV_RECOMPUTED"`   | ARAP unfurl pass complete for all panels       |
| `TailorRefitPromoted`         | `"TAILOR_REFIT_PROMOTED"`        | Output garment passes gate and is promoted     |
| `TailorRefitRejected`         | `"TAILOR_REFIT_REJECTED"`        | Blocking validation failure; run rejected      |

Avatar and body-proxy lifecycle events (`TAILOR_AVATAR_CREATED`,
`TAILOR_AVATAR_MEASUREMENTS_EXTRACTED`, `TAILOR_BODY_PROXY_CREATED`,
`TAILOR_BODY_PROXY_UPDATED`) use `event_family = "tailor.avatar"` and
`"tailor.body_proxy"` respectively [T-CONTRACTS.event-types].

The superseded event names `BodyProxyCreated`, `BodyProxyMeasurementsExtracted` (from
`07-autofit-retargeting.md`, missing the `Tailor` prefix) and `TailorDraftScaled` (same file)
MUST NOT appear in implementation code; use the canonical variants above.

---

#### §7.10 CRDT for Competing Refit Proposals

Multiple model or operator agents MAY propose competing refits of the same source garment to
the same target body (e.g., one using `ScaleAndRedrape`, another using `OptimizePatterns`).

Each refit MUST produce a new `tailor_garments` derived draft row and a corresponding
`tailor_garment_crdt_docs` row with `crdt_document_id = "CRDT-GAR-{output_garment_id}"`.
Competing proposals MUST be tracked as distinct actor-site CRDT documents using the existing
`CrdtUpdateRecordV1` infrastructure; no new CRDT table is required.

The operator MUST choose which refit proposal to promote. The `PromotionGate` MUST NOT
auto-select between competing proposals.

---

#### §7.11 Model-First Refit API

##### §7.11.1 MCP Tool Surface

The model-steerable refit API MUST be exposed as an MCP tool (`refit_garment`) inside the
`TailorModelAdapter`. The model receives a context bundle and MUST emit a `RefitRequest` JSON
(schema `hsk.tailor.refit_request@1`) as its artifact output. The model MUST NOT write
directly to any `tailor_*` authority table; its output becomes a `TailorRefitRequested`
EventLedger event, and the sandbox run takes it from there.

Required context bundle fields supplied to the model:

| Field                         | Type   | Description                                            |
|------------------------------|--------|--------------------------------------------------------|
| `garment_id`                  | String | Source garment `"GAR-{uuid_v7}"`                      |
| `source_body_proxy_id`        | String | Optional; model selects if absent                     |
| `target_avatar_id`            | String | `"AVT-{uuid_v7}"` or parametric slug                  |
| `refit_intent`                | String | `fit_to_new_body \| scale_up \| scale_down \| non_human_retarget` |
| `source_measurements_mm`      | JSON   | 25-field measurement map (source body)                |
| `target_measurements_mm`      | JSON   | 25-field measurement map (target body)                |
| `fabric_material_description` | String | Optional natural-language fabric hint                 |

Expected model output (artifact_payload, validated against schema `hsk.tailor.refit_request@1`
before sandbox run creation):

```json
{
  "schema_id": "hsk.tailor.refit_request@1",
  "garment_id": "GAR-...",
  "target_avatar_id": "AVT-...",
  "refit_mode": "scale_and_redrape",
  "ease_overrides": {
    "ease_mm": { "bust": 20.0, "hip": 30.0, "waist": 15.0 }
  },
  "material_params_override": {
    "bending_weft": 0.6,
    "density_g_m2": 95.0
  },
  "rationale": "Target bust 8 cm larger; scaling bodice panels by bust ratio. Silk params preserved."
}
```

The `material_params_override` fields map to `FabricProperties` normalized fields in
`GarmentSpec`. The model MUST use the normalized [0.0, 1.0] range for fabric stiffness fields
and the physical units (`density_g_m2`, `collision_thickness_mm`) for the two physical fabric
fields [T-CONTRACTS.garment-spec].

##### §7.11.2 LLM Material Re-Estimation

When the operator changes the fabric material as part of retargeting (e.g., "same dress design,
but now in denim for the bigger avatar"), the model lane MUST estimate updated `FabricProperties`
from the natural-language material description using the four ChatGarment descriptors
(rigid/soft, heavy/light, wrinkle/smooth, perceived thickness), mapped to the normalized
`FabricProperties` fields via the preset/decoder layer (`tailor_material_presets`).

The model MUST include updated `material_params_override` in the `RefitRequest` JSON when it
re-estimates fabric parameters. The model MUST NOT hard-code raw XPBD compliance values;
the normalized surface is the sole LLM-facing contract.

---

#### §7.12 Risks, Mitigations, and Constraints

The following risks are normative constraints on the implementation.

**REFIT-RISK-1 — Non-human avatar convergence failures.**
For `source_kind = 'non_humanoid'`, the three-pass progressive drape MAY fail to converge
when body geometry has no humanoid segment structure. The implementation MUST:
(a) expose all `RefitDrapeStrategy` parameters in the refit request so operators can override
defaults for unusual avatars;
(b) surface timeout without convergence as `REFIT_CONVERGED` advisory (never as a crash or
silent success);
(c) never return a timed-out simulation result as a converged, promotable garment.

**REFIT-RISK-2 — Multi-layer garment sequencing.**
The `OptimizePatterns` differentiable path operates on a single-layer garment. For multi-layer
garments, the Tailor module MUST execute retargeting as sequential single-layer runs in
innermost-to-outermost panel order, passing each successfully retargeted layer's output mesh as
an additional collision body for the next layer. Each layer run MUST pass its own validation gate
before the next layer begins.

**REFIT-RISK-3 — ARAP UV divergence under large deformation.**
If any panel's area changes by more than 30% relative to source, the implementation MUST emit
`UV_VALIDITY` advisory and MUST provide a `"lock_graphics_layer"` option in the refit request
that pins graphic-anchor vertices during the ARAP unfurl (§7.5.2). The `lock_graphics_layer`
option MUST default to `true` when any `tailor_graphic_layers` row exists for the source garment.

**REFIT-RISK-4 — Cross-backend promotion equivalence.**
The implementation MUST use `MeshComparator` (tolerance-based, `epsilon_mm = 0.1`) for
promotion equivalence checks on refit output, not SHA-256 hash comparison [T-CONTRACTS.determinism].
SHA-256 `content_hash` on `tailor_refit_runs` is retained for same-machine idempotency only.

**REFIT-RISK-5 — `OptimizePatterns` timeout configuration.**
`OptimizePatterns` requires offline compute time comparable to 5–30 minutes. The sandbox adapter
MUST enforce `policy.timeout_secs >= 1800.0` for this mode and MUST reject the run with a
descriptive error (not silently truncate) if the timeout is shorter.

**REFIT-RISK-6 — Non-humanoid capsule set authority.**
For `source_kind = 'non_humanoid'`, the operator MUST supply the capsule set JSON at body proxy
creation time. The implementation MUST NOT attempt auto-generation of a humanoid capsule chain
for non-humanoid avatars. An automated convex-hull decomposition (`parry3d::shape::ConvexHull`)
MAY be offered as a hint, but MUST be labeled as a hint and MUST require operator approval before
being written as an authority proxy row.

---

#### §7.13 Reuse Moat

The auto-fit subsystem establishes a durable reuse moat through:

1. **Body proxy library.** Every avatar imported into `tailor_avatars` + `tailor_body_proxies`
   is reusable for all future garments without re-import. The measurement map and capsule set
   are computed once and stored as authority rows.

2. **Derived draft chain.** Each refit produces a new `tailor_garments` derived draft linked to
   the source garment via `tailor_refit_runs.garment_id`. The full refit provenance (source body,
   target body, mode, scale factors) is queryable from the authority tables without chat history.

3. **Measurement-driven grading.** The `compute_panel_scales` function (§7.3) can grade any
   garment to any target body without design re-authoring, as long as the target body has a
   `tailor_avatars` row with a `measurements_mm_json` map.

4. **Model-steerable iteration.** The model-first API (§7.11) allows a model lane to propose
   multiple refit strategies (mode selection, ease overrides, material re-estimation) as competing
   CRDT proposals, and the operator selects the best result — without the operator needing to
   understand solver parameters.

5. **Blend-shape continuity.** The `BlendShapeRefitRequest` warm-start path (§7.7) allows rapid
   interactive refit across morph-target body shape variation, reusing the prior equilibrium
   state as solver initialization, making incremental body shape exploration cheap.

---

#### §7.14 Non-Normative Provenance

The following research documents informed this sub-section and are cited as non-normative
provenance. They MUST NOT be treated as spec authority; where they conflict with
[T-CONTRACTS], T-CONTRACTS wins.

- `07-autofit-retargeting.md` — primary design rationale; OSS pipeline evidence
  (DressAnyone, Bolt, Intersection-Free Garment Retargeting, GarmentCode, ChatGarment);
  Marvelous Designer feature mapping.
- `16-contracts.md` [T-CONTRACTS] — canonical authority for all type names, field names, units,
  event variants, schema IDs, migration naming, table PKs, validation check catalog, and
  promotion equivalence. This sub-section uses its contracts verbatim.

Key OSS references (informational):
- DressAnyone (ETH Zurich / Meta, 2024): `https://arxiv.org/abs/2405.19148` — differentiable
  XPBD pattern optimization; the algorithmic reference for `OptimizePatterns` mode.
- NVIDIA Bolt (Apr 2025): `https://arxiv.org/pdf/2504.17614` — feed-forward garment transfer
  at scale; the layer-sequencing and progressive drape pattern.
- Intersection-Free Garment Retargeting (SIGGRAPH 2025):
  `https://github.com/Huangzizhou/cloth-fit` — barrier-method intersection-free guarantee;
  motivates the `REFIT_INTERSECTION_FREE` blocking check.
- GarmentCode / GarmentMeasurements (ETH Zurich):
  `https://github.com/maria-korosteleva/GarmentCode` — 25-measurement extraction convention;
  `cm` unit choice for `GarmentSpec`; measurement-driven panel grading.
- ChatGarment (CVPR 2025): `https://chatgarment.github.io/` — four-descriptor LLM material
  re-estimation; the basis for §7.11.2.

## 13.8 Trims & Cloth-Rigid Coupling

<!-- id: trimrigid -->
<!-- provenance (non-normative): .GOV/reference/cloth_engine_research/13-trim-rigid.md,
     16-contracts.md. T-CONTRACTS wins on all schema/event/table/naming conflicts. -->

---

### TR-1. Scope and Definitions

**TR-1.1** The Tailor trim system MUST implement a mixed cloth-rigid XPBD simulation graph in
which rigid trim bodies (buttons, buckles, zippers, eyelets, rivets, hooks, armor plates) and
cloth panels share a single substep loop, the same Lagrange multiplier update, and the same WGPU
compute dispatch. No separate rigid-body simulation pass outside the XPBD substep is permitted.

**TR-1.2** The following terminology is normative throughout this section:

- **Trim body**: a rigid 3D mesh that moves as a single rigid object (six DOF: centroid position
  `x: vec3` + orientation quaternion `q: quat`). It does not deform.
- **Tack**: a point constraint that couples one world-space attachment point on a trim body to one
  cloth particle. It is the fundamental cloth-rigid coupling primitive; every other trim-attachment
  mechanism (Glue, multi-point Tack, zipper rail attachment, eyelet pin) is expressed as one or
  more tack constraints.
- **Tack strength**: a scalar `strength_scale ∈ [0.0, 1.0]` that modulates the constraint
  compliance; `0.0` makes the constraint dormant (detached); `1.0` is full strength.
- **Kinematic trim**: a trim body with `inv_mass = 0`; it receives no correction from constraint
  forces and acts as a static driver. Tack constraints still pull cloth particles toward it.
- **Stiffness**: a per-trim scalar `∈ [0.0, 1000.0]`; at `1000.0` the body MUST be promoted to
  kinematic mode (`inv_mass = 0`). At lower values the body is dynamic (responds to constraint
  forces and gravity).

**TR-1.3** This section is authoritative for the trim domain. Where research note
`13-trim-rigid.md` conflicts with `16-contracts.md`, the contracts file wins. Where either
conflicts with this spec, this spec wins.

---

### TR-2. GPU Types and Buffer Layout

**TR-2.1** The `tailor-solver` crate MUST define `GpuTrimBody` and `GpuTackConstraint` as
`bytemuck::Pod` / `bytemuck::Zeroable` structs with the exact strides specified. Changing field
layout without updating all referencing WGSL struct definitions is forbidden.

```rust
// tailor-solver/src/types.rs

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTrimBody {
    pub pos:         [f32; 4],   // xyz = world centroid; w = inv_mass
    pub quat:        [f32; 4],   // xyzw orientation (unit quaternion)
    pub vel:         [f32; 4],   // xyz = linear velocity; w = unused
    pub omega:       [f32; 4],   // xyz = angular velocity; w = unused
    pub pos_pred:    [f32; 4],   // predicted centroid after external forces
    pub quat_pred:   [f32; 4],   // predicted orientation
    pub inertia_inv: [f32; 12],  // 3x3 inverse inertia tensor, row-major, std430-padded
    pub stiffness:   f32,        // [0, 1000]; 1000 => kinematic (inv_mass must be 0)
    pub trim_index:  u32,        // index into tailor_trim_placements row (for event linkage)
    pub _pad:        [f32; 2],
}
// Stride: 128 bytes. Buffer: N_trims × 128 bytes.

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTackConstraint {
    pub body_idx:       u32,      // index into GpuTrimBody buffer
    pub particle_idx:   u32,      // index into GpuParticle buffer
    pub r_local:        [f32; 4], // body-frame attachment offset from centroid (xyz; w unused)
    pub compliance:     f32,      // α; 0 = rigid; 1e-4 = elasticated
    pub strength_scale: f32,      // keyframeable [0.0, 1.0]; 0 = dormant
    pub _pad:           [f32; 2],
}
// Stride: 32 bytes. Buffer: N_tacks × 32 bytes.
```

**TR-2.2** For cord simulations (lacing), the solver MUST define `GpuCordParticle` and
`GpuCordSegment`:

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCordParticle {
    pub pos:      [f32; 4],   // xyz position; w unused
    pub vel:      [f32; 4],   // xyz velocity; w unused
    pub pos_pred: [f32; 4],   // predicted position
    pub inv_mass: f32,
    pub _pad:     [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCordSegment {
    pub i0:          u32,
    pub i1:          u32,
    pub rest_length: f32,  // mm
    pub compliance:  f32,
}
```

**TR-2.3** Inertia tensor initialization is REQUIRED at trim-asset load time. The implementation
MUST compute `GpuTrimBody.inertia_inv` from the trim mesh using the area-weighted triangle
contribution formula, then invert. A `det(I) <= 0` check MUST fail with a hard error before
inverting; such a mesh is degenerate and MUST NOT be uploaded to the GPU.

---

### TR-3. Mixed Constraint Graph: Substep Order

**TR-3.1** The XPBD substep loop MUST execute constraint passes in the following order. The two
new passes (`tack.wgsl` and `trim_contact.wgsl`) MUST appear at the positions shown and MUST NOT
be merged with cloth-only passes.

```
Per substep (n_substeps per frame):
  Pass 1:   predict.wgsl          -- cloth particles + trim rigid bodies (extended predict)
  Pass 2..K: stretch.wgsl         -- cloth stretch, per color partition
  Pass K+1..M: bend.wgsl          -- cloth bend, per color partition
  Pass M+1: seam.wgsl             -- seam distance constraints (incl. zipper active segments)
  Pass M+2: tack.wgsl             -- cloth-particle to rigid-body ball-joint constraints [NEW]
  Pass M+3: trim_contact.wgsl     -- rigid trim mesh vs cloth particle contact [NEW]
  Pass M+4: body_collide.wgsl     -- cloth vs avatar capsule/sphere proxy
  Pass M+5: self_collide.wgsl     -- cloth self-collision
  Pass M+6: velocity.wgsl         -- velocity update for cloth particles AND trim bodies
```

**TR-3.2** The predict pass MUST integrate trim rigid body state using the quaternion-derivative
form:

```
x_pred  = x + dt_sub * v
q_pred  = normalize(q + 0.5 * dt_sub * quat(0, omega) * q)
```

External forces (gravity scaled by `1/inv_mass`) MUST be applied to `v` before this integration.
Kinematic bodies (`inv_mass = 0`) MUST skip force integration; their `pos_pred`/`quat_pred` are
set by the animation driver, not by the predict pass.

**TR-3.3** The velocity update pass MUST update trim rigid body linear and angular velocity after
all constraint passes:

```
v     = (pos_pred - pos) / dt_sub
omega = 2 * (conj(q) * q_delta).xyz / dt_sub
pos   = pos_pred
q     = normalize(q_pred)
```

---

### TR-4. Tack Coupling Pass (tack.wgsl)

**TR-4.1** The tack pass MUST implement a ball-joint constraint (equality constraint, zero rest
length) between each tack's cloth particle and its trim body attachment point, using the Müller-
Macklin XPBD rigid body coupling formulation (SCA 2020, CGF 39(8)). The effective inverse mass of
the rigid body at attachment point MUST be computed as:

```
r_world = quat_rotate(body.quat_pred, r_local)
w_body  = inv_mass + dot(r_world × n, I_inv * (r_world × n))
```

where `n` is the unit constraint gradient and `I_inv` is the 3×3 inverse inertia tensor.

**TR-4.2** The XPBD Lagrange multiplier update MUST be:

```
alpha       = (compliance / strength_scale) / (dt_sub^2)
delta_lam   = -(C + alpha * lambda) / (w_body + w_cloth + alpha)
lambda      += delta_lam
```

where `C = |p_cloth - p_body_attachment|`. When `strength_scale < 1e-5` the constraint MUST be
skipped entirely (no position update, no lambda update). This is the detached-tack early-out that
enables animated stitching and unstitching.

**TR-4.3** Position corrections MUST be applied to both the cloth particle and the trim body:

```
// Cloth particle translational correction
dx_cloth  =  w_cloth / (w_body + w_cloth) * n * delta_lam

// Trim body translational correction
dx_body   = -w_body  / (w_body + w_cloth) * n * delta_lam

// Trim body angular correction via quaternion update
ang_impulse = I_inv * (r_world × dx_body)
dq          = 0.5 * quat(0, ang_impulse) * q_pred
q_pred      = normalize(q_pred + dq)
```

**TR-4.4** The tack pass MUST be color-partitioned. The constraint graph for coloring MUST include
both cloth particle indices AND trim body indices as nodes, with edges between every pair that
shares a constraint. Within a single color partition: no particle index appears more than once AND
no body index appears more than once. This invariant is a hard correctness requirement; violation
produces a GPU data race on `GpuTrimBody` without atomics. The coloring algorithm MUST verify both
node types before any GPU dispatch.

**TR-4.5** Tack lambda accumulators (`tack_lambdas`) MUST be reset to zero at the start of each
substep, matching the XPBD convention for cloth stretch/bend accumulators.

---

### TR-5. Trim-Cloth Contact Pass (trim_contact.wgsl)

**TR-5.1** The trim-cloth contact pass MUST apply a one-sided inequality constraint (`max(0, -C)`)
to push cloth particles out of trim mesh surfaces. It fires only when a cloth particle penetrates
within `collision_thickness_mm` of a trim triangle surface.

**TR-5.2** The trim mesh MUST be maintained in a **world-space triangle buffer** that is rebuilt
each substep from body-local triangle data transformed by `pos_pred`/`quat_pred`. This pre-pass
MUST run before `trim_contact.wgsl` in the same substep.

**TR-5.3** For trim meshes with fewer than 200 triangles (buttons, buckles, eyelets), a brute-
force per-cloth-particle loop over all trim triangles is acceptable. For trim meshes with 200 or
more triangles (armor plates, large accessories), an AABB pre-filter or BVH over world-space trim
triangles MUST be used to cull distant particles before the per-triangle query.

**TR-5.4** BVH rebuild MUST be forced when trim body displacement between substeps exceeds
`0.5 × collision_thickness_mm`. When displacement is below this threshold, the BVH from the
previous substep MAY be reused.

**TR-5.5** The contact pass MUST NOT write corrections back to the `GpuTrimBody` buffer (trim
bodies are not corrected by cloth contact in this pass — only cloth particles are moved). Two-way
rigid-cloth contact response (where cloth also pushes the trim) is deferred to a future spec
revision and is not required for MVP.

---

### TR-6. Authority Data Model

#### TR-6.1 Trim Asset (tailor_trims)

**TR-6.1.1** Every trim asset MUST be stored as an authority row in `tailor_trims` with `TEXT
PRIMARY KEY` using the canonical prefix `TRIM-{uuid_v7}`. The `UUID PRIMARY KEY DEFAULT
gen_random_uuid()` form is forbidden (per T-CONTRACTS FACT-4).

**TR-6.1.2** The `trim_category` column MUST enforce the following CHECK domain:

```sql
CHECK (trim_category IN (
    'button', 'buckle', 'zipper_body', 'zipper_slider', 'zipper_teeth',
    'eyelet', 'rivet', 'hook', 'armor_plate', 'cord', 'accessory', 'custom'
))
```

**TR-6.1.3** The canonical migration for this table MUST follow the dated convention from
T-CONTRACTS (e.g. `2026_MM_DD_tailor_trims.sql` where the date is set at implementation time)
with a required `.down.sql` reverse pair. Numbered `0NNN_*` migrations are forbidden.

**TR-6.1.4** The `mesh_json` MUST store the trim mesh (vertices + triangles + normals) as a
`TrimMeshV1` JSONB object. The `inertia_tensor_json` MUST store the precomputed 3×3 inertia
tensor. Both MUST be populated at import time; a row with a null or empty `mesh_json` is invalid.

**TR-6.1.5** `tack_anchor_json` MUST store the canonical attachment point positions in body-local
coordinates for the trim type (e.g., four hole positions for a four-hole button). The solver MUST
use these as the default `r_local` values for `GpuTackConstraint` when the operator does not
override them.

**TR-6.1.6** When a trim is created via pattern-to-rigid conversion, `converted_from_panel_id`
MUST be set to the source `panel_id` to preserve the audit trail. The source panel geometry MUST
be retained as a `superseded_panel` entry in the CRDT document tree; it MUST NOT be deleted.

```sql
-- Migration: 2026_MM_DD_tailor_trims.sql  (date assigned at implementation)
CREATE TABLE IF NOT EXISTS tailor_trims (
    trim_id                 TEXT PRIMARY KEY,           -- "TRIM-{uuid_v7}"
    workspace_id            TEXT NOT NULL,
    name                    TEXT NOT NULL,
    trim_category           TEXT NOT NULL
        CHECK (trim_category IN ('button','buckle','zipper_body','zipper_slider',
                                  'zipper_teeth','eyelet','rivet','hook',
                                  'armor_plate','cord','accessory','custom')),
    source_asset_ref        TEXT,
    mesh_json               JSONB NOT NULL,
    inertia_tensor_json     JSONB NOT NULL,
    default_mass_g          FLOAT NOT NULL DEFAULT 1.0
        CHECK (default_mass_g > 0),
    default_stiffness       FLOAT NOT NULL DEFAULT 100.0
        CHECK (default_stiffness >= 0.0 AND default_stiffness <= 1000.0),
    tack_anchor_json        JSONB,
    is_library_item         BOOLEAN NOT NULL DEFAULT FALSE,
    converted_from_panel_id TEXT,
    event_ledger_event_id   TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_trims_workspace ON tailor_trims (workspace_id);
CREATE INDEX IF NOT EXISTS ix_tailor_trims_category  ON tailor_trims (trim_category);
```

#### TR-6.2 Trim Placements (tailor_trim_placements)

**TR-6.2.1** Per-garment placement of a trim MUST be stored in `tailor_trim_placements` with
prefix `PLAC-{uuid_v7}`. The `tacks_json` column MUST store the full `TackDefinitionV1` array
for the placement. An empty `tacks_json` array is valid only for purely decorative trims where no
cloth coupling is desired; the `TACK_SEAM_CLOSURE` validation check is skipped for tack-less
placements.

```sql
CREATE TABLE IF NOT EXISTS tailor_trim_placements (
    placement_id            TEXT PRIMARY KEY,           -- "PLAC-{uuid_v7}"
    garment_id              TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    trim_id                 TEXT NOT NULL REFERENCES tailor_trims (trim_id),
    initial_pose_json       JSONB NOT NULL,             -- { pos:[x,y,z], quat:[x,y,z,w] }, mm
    tacks_json              JSONB NOT NULL,             -- array<TackDefinitionV1>
    stiffness_override      FLOAT
        CHECK (stiffness_override IS NULL OR
               (stiffness_override >= 0.0 AND stiffness_override <= 1000.0)),
    mass_override_g         FLOAT
        CHECK (mass_override_g IS NULL OR mass_override_g > 0),
    layer_order             INTEGER NOT NULL DEFAULT 0,
    event_ledger_event_id   TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_trim_placements_garment
    ON tailor_trim_placements (garment_id);
```

#### TR-6.3 Tack Definition

**TR-6.3.1** `TackDefinitionV1` is the canonical tack authority type, stored inside
`tacks_json`. It MUST NOT be stored as a separate table; it is owned by the placement row.

```rust
// tailor-solver/src/spec.rs  (part of the canonical GarmentSpec authority type)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TackDefinitionV1 {
    /// "TACK-{uuid_v7}"; unique within the placement.
    pub tack_id:        String,
    /// Attachment point in trim body-local coordinates (mm).
    pub r_local:        [f32; 3],
    /// UV coordinates on the target panel to find the nearest cloth particle.
    pub particle_uv:    [f32; 2],
    /// Constraint compliance. 0.0 = rigid attachment; 1e-4 = soft/elasticated.
    pub compliance:     f32,
    /// Keyframe curve: [(frame, strength_scale), ...]. Interpolated per substep.
    /// Empty = constant strength 1.0.
    pub strength_curve: Vec<[f32; 2]>,
    /// Optional label for tooling, e.g. "button-left-1", "eyelet-top-3".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label:          Option<String>,
}
```

**TR-6.3.2** A Glue placement (single click-to-place attachment) MUST be modeled as a
`TackDefinitionV1` array of length 1. A multi-point Tack MUST be modeled as an array of 2 or
more. The schema does not distinguish the two placement tools; only the count differs.

#### TR-6.4 Zipper Definitions (tailor_zippers)

**TR-6.4.1** Zipper definitions MUST be stored in `tailor_zippers` with prefix `ZIP-{uuid_v7}`.
`slider_count` MUST be 1 (standard) or 2 (two-way). For two-way zippers, `slider_b_pos` MUST NOT
be null when `slider_count = 2`.

```sql
CREATE TABLE IF NOT EXISTS tailor_zippers (
    zipper_id               TEXT PRIMARY KEY,           -- "ZIP-{uuid_v7}"
    garment_id              TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    panel_edge_a            TEXT NOT NULL,
    panel_edge_b            TEXT NOT NULL,
    tooth_interval_mm       FLOAT NOT NULL DEFAULT 5.0
        CHECK (tooth_interval_mm > 0),
    slider_count            INTEGER NOT NULL DEFAULT 1
        CHECK (slider_count IN (1, 2)),
    slider_a_pos            FLOAT NOT NULL DEFAULT 0.0
        CHECK (slider_a_pos >= 0.0 AND slider_a_pos <= 1.0),
    slider_b_pos            FLOAT
        CHECK (slider_b_pos IS NULL OR (slider_b_pos >= 0.0 AND slider_b_pos <= 1.0)),
    tooth_mesh_ref          TEXT,
    slider_mesh_ref         TEXT,
    stiffness               FLOAT NOT NULL DEFAULT 100.0,
    event_ledger_event_id   TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**TR-6.4.2** Zipper tooth seam constraints MUST be modeled as a special case of the standard seam
constraint buffer (`GpuSeamConstraint` with `gather_ratio = 1.0`). Each tooth pair generates one
seam constraint. An `active_mask` buffer (one bit per tooth pair, separate from the seam constraint
buffer) MUST gate constraint evaluation per substep. The constraint buffer itself MUST NOT be re-
uploaded per frame for slider animation; only the `active_mask` buffer is updated.

**TR-6.4.3** For two-way zippers, the active mask MUST mark a tooth pair as active when and only
when its position along the rail lies between `slider_a_pos` and `slider_b_pos` (i.e., within the
closed region). The CPU-side mask update MUST run before the seam pass of each substep.

**TR-6.4.4** Tooth-rail tack constraints MUST be included in the tack constraint buffer. One tack
per tooth rail segment connects the rail body to its panel edge. These tacks participate in tack
graph coloring (TR-4.4) and in the `ZIPPER_TOOTH_ALIGN` validation check.

#### TR-6.5 Lacing Definitions (tailor_lacings)

**TR-6.5.1** Lacing cord definitions MUST be stored in `tailor_lacings` with prefix
`LACE-{uuid_v7}`. Each lacing row owns the ordered eyelet sequence and the cord parameters.

```sql
CREATE TABLE IF NOT EXISTS tailor_lacings (
    lacing_id               TEXT PRIMARY KEY,           -- "LACE-{uuid_v7}"
    garment_id              TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    eyelet_sequence_json    JSONB NOT NULL,             -- ordered array of placement_ids (eyelet trims)
    cord_rest_length_mm     FLOAT NOT NULL DEFAULT 3.0
        CHECK (cord_rest_length_mm > 0),
    cord_compliance         FLOAT NOT NULL DEFAULT 1e-4,
    lace_pattern            TEXT NOT NULL DEFAULT 'straight'
        CHECK (lace_pattern IN ('straight', 'criss-cross')),
    event_ledger_event_id   TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**TR-6.5.2** Each eyelet in the sequence MUST be a trim placement with `trim_category = 'eyelet'`.
A lacing MUST fail validation if any `placement_id` in `eyelet_sequence_json` does not resolve to
an eyelet-category placement on the same garment.

**TR-6.5.3** Cord particles MUST be solved in the same XPBD substep loop as cloth particles. Cord
segment stretch constraints MUST be solved in a dedicated pass after the tack pass. Threading
constraints that pin a cord particle to an eyelet attachment point MUST use `GpuTackConstraint`
with `particle_idx` pointing into the cord particle buffer and `body_idx` pointing into the
eyelet's `GpuTrimBody`. They MUST participate in tack graph coloring (TR-4.4).

---

### TR-7. Trim Physics Parameters

**TR-7.1** Three per-trim physics parameters MUST be supported and MUST be settable independently
of each other:

| Parameter | Authority field | Solver mapping | Valid range |
|---|---|---|---|
| Stiffness | `tailor_trims.default_stiffness` or `stiffness_override` | At 1000: `inv_mass = 0` (kinematic). Below 1000: `inv_mass = 1/mass`. | [0.0, 1000.0] |
| Mass (g) | `tailor_trims.default_mass_g` or `mass_override_g` | `GpuTrimBody.pos.w = 1/mass_kg` | > 0 |
| Tack Strength | `TackDefinitionV1.strength_curve` | `GpuTackConstraint.strength_scale` per substep | [0.0, 1.0] |

**TR-7.2** When `stiffness = 1000.0`, the solver MUST set `GpuTrimBody.pos.w = 0` (kinematic).
The constraint still activates and drags the cloth particle; the body itself does not move. When
`stiffness < 1000.0`, `inv_mass` MUST be set from the effective mass (applying `mass_override_g`
if present, else `default_mass_g`).

**TR-7.3** Trim mass and tack strength MUST be keyframeable. Keyframe values MUST be delivered via
a mapped GPU buffer write (partial update of the `GpuTackConstraint.strength_scale` fields for the
affected tacks) rather than a full buffer re-upload. The `MaterialFrameParams` extension MUST carry
`tack_strength: f32` and `trim_weight_scale: f32` for per-frame overrides:

```rust
// Extension to MaterialFrameParams (tailor-solver/src/params.rs)
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct MaterialFrameParams {
    pub solidify_blend:    f32,
    pub pressure_target:   f32,
    pub shrink_u:          f32,
    pub shrink_v:          f32,
    pub tack_compliance:   f32,   // per-frame compliance override (0 = rigid)
    pub tack_strength:     f32,   // 0.0 = detached, 1.0 = full strength
    pub trim_weight_scale: f32,   // multiplier on GpuTrimBody inv_mass (1.0 = nominal)
}
```

**TR-7.4** `TailorSimRunCompleted` EventLedger payloads MUST include the `MaterialFrameParams`
array used during the run so that the animation is fully reproducible from the EventLedger alone,
as required by the determinism contract (`MeshComparator`, T-CONTRACTS §T-CONTRACTS.determinism).

---

### TR-8. Pattern-to-Rigid-Body Conversion

**TR-8.1** The system MUST support converting a cloth panel to a rigid trim body
(`tailor_convert_panel_to_trim` MCP tool). The conversion algorithm MUST follow these steps in
order; deviation from the step order is a correctness error:

1. Compute centroid `x_centroid = (1/N) * Σ(x_i)` over the panel's current simulated vertex
   positions.
2. Compute the inertia tensor from panel mesh vertices relative to centroid, using the area-
   weighted triangle formula and the panel's `FabricProperties.density_g_m2`.
3. Verify `det(I) > 0`; reject if not.
4. Invert `I` to produce `I_inv`.
5. Instantiate `GpuTrimBody` with `pos = x_centroid`, `quat = identity`, `inv_mass = 1/mass`,
   `inertia_inv = I_inv`.
6. Set `inv_mass = 0` for all cloth particles belonging to the converted panel (removing them
   from cloth dynamics).
7. Add the trim body to the trim buffer.
8. Convert any seam constraints touching the panel into tack constraints: each seam endpoint on
   the now-rigid side becomes a `GpuTackConstraint` with `r_local` equal to the endpoint's
   body-frame offset from the centroid.
9. Emit `TailorPatternToTrimConverted` to the EventLedger, linking `panel_id` to the resulting
   `trim_id`.

**TR-8.2** Pattern-to-rigid conversion MUST be restricted to the pre-simulation authoring phase.
It MUST NOT be performed after a simulation run has started (i.e., after `TailorSimRunStarted` has
been emitted). The pre-simulation validation gate MUST reject a `GarmentSpec` that requests
conversion on a panel that is already the subject of a completed simulation run.

**TR-8.3** The Solidify property (`solidify_blend ∈ [0, 1]`, keyframeable) MUST be implemented
as a soft alternative to full conversion. When `solidify_blend > 0`, all constraint compliances
touching the panel MUST be scaled by `(1.0 - solidify_blend)`. At `solidify_blend = 1.0` the
panel behaves as kinematic cloth (all compliances zero) but remains in the particle solver, not
the rigid body solver. Full pattern-to-rigid conversion is required for panels that need rigid
mesh collision behavior against other cloth panels.

---

### TR-9. GarmentSpec Integration

**TR-9.1** The canonical `GarmentSpec` (T-CONTRACTS §T-CONTRACTS.garment-spec) MUST include
`trim_placements: Vec<TrimPlacementRef>` as an optional field (empty by default). Schema ID for
this field follows `hsk.tailor.trim_placement@1` (T-CONTRACTS §T-CONTRACTS.schema-ids).

**TR-9.2** The `TrimPlacementRef` lightweight type in `GarmentSpec` carries only `placement_id`
and `trim_category`. The full tack definitions, pose, and physics parameters live in
`tailor_trim_placements.tacks_json`. The solver MUST load the full placement row from Postgres
when building the `SolverMesh`; `TrimPlacementRef` is a reference only.

**TR-9.3** Model agents authoring a `GarmentSpec` with trims MAY use library trim categories
without specifying a `trim_id`; the sandbox adapter MUST resolve the default library item for the
category (`is_library_item = TRUE`) when no explicit trim_id is provided.

---

### TR-10. EventLedger Events

**TR-10.1** The following `KernelEventType` variants MUST be added to `kernel/mod.rs` for the
trim domain, using canonical wire strings (T-CONTRACTS §T-CONTRACTS.event-types). All MUST be
registered in `required_first_slice_events()`:

```rust
// Trim domain — variant => wire string (as_str)
TailorTrimImported,            // "TAILOR_TRIM_IMPORTED"
TailorTrimPlaced,              // "TAILOR_TRIM_PLACED"
TailorTrimTackUpdated,         // "TAILOR_TRIM_TACK_UPDATED"
TailorZipperDefined,           // "TAILOR_ZIPPER_DEFINED"
TailorLacingDefined,           // "TAILOR_LACING_DEFINED"
TailorPatternToTrimConverted,  // "TAILOR_PATTERN_TO_TRIM_CONVERTED"
TailorTrimContactViolation,    // "TAILOR_TRIM_CONTACT_VIOLATION"
```

**TR-10.2** `event_family` constants for the trim domain MUST use the canonical dotted namespace
(T-CONTRACTS §T-CONTRACTS.event-types):

```rust
pub const TAILOR_TRIM: &str = "tailor.trim";
```

**TR-10.3** `TailorTrimContactViolation` MUST be emitted (advisory, non-blocking) when the
`TRIM_NO_PENETRATION` validation check detects deep penetration (beyond the `MeshComparator`
ε = 0.1 mm tolerance) in the final simulated frame. It MUST carry `trim_id`, `placement_id`,
and the maximum penetration depth in mm.

---

### TR-11. Validation Checks

**TR-11.1** When a `GarmentSpec` includes trim placements, the following checks from the
canonical `ValidationDescriptor` catalog (T-CONTRACTS §T-CONTRACTS.validation) MUST be applied
at the post-simulation stage:

| Check code | Severity | Assertion |
|---|---|---|
| `TRIM_NO_PENETRATION` | Blocking | No trim mesh triangle interpenetrates a cloth triangle at the final simulated frame. Penetration threshold: 0.0 mm (any contact beyond collision_thickness is a violation). |
| `TACK_SEAM_CLOSURE` | Blocking | All tack constraint distances ≤ 5 mm at the end of the draping phase. |
| `ZIPPER_TOOTH_ALIGN` | Blocking | Every tooth-rail tack attachment is within 1 mm of its nominal panel edge position. Applied only when `tailor_zippers` rows exist for the garment. |
| `LACING_CORD_LENGTH` | Blocking | No cord segment stretched beyond 200% of its `cord_rest_length_mm`. Applied only when `tailor_lacings` rows exist. |
| `TRIM_GRAVITY_STABLE` | Advisory | No trim body centroid translates more than 50 mm/frame in the final 10 frames of simulation. |
| `TACK_STRENGTH_NONZERO` | Advisory | Warn if any active tack has `strength_scale < 0.01` (potentially unintentionally detached). |

**TR-11.2** These checks MUST be skipped (not reported as failures) when no trim placements are
present in the `GarmentSpec`. A garment without trims MUST NOT produce trim-domain validation
findings.

**TR-11.3** The `ValidationDescriptor` catalog in T-CONTRACTS §T-CONTRACTS.validation is the
single authority. These checks are normative in this section only insofar as they are already
listed there; this section does not introduce new check codes. Any future trim-domain check MUST
be added to T-CONTRACTS first.

---

### TR-12. MCP Tool Surface

**TR-12.1** The `handshake_core::tailor` kernel module MUST expose the following MCP tools for
the trim domain. Tool parameter and return types MUST be `schemars::JsonSchema`-derivable so
`inputSchema` is auto-generated:

- `tailor_place_trim`: place a trim on a garment panel with tack attachment points. Accepts
  `garment_id`, `trim_category`, optional `trim_id`, `position` (panel UV or world-space),
  optional `tacks` array, optional `stiffness`.
- `tailor_define_zipper`: define a zipper between two panel edges. Accepts `garment_id`,
  `panel_edge_a`, `panel_edge_b`, `tooth_interval_mm`, `slider_count`, `slider_a_pos`, optional
  `slider_b_pos`.
- `tailor_convert_panel_to_trim`: convert a cloth panel to a rigid trim body. Accepts
  `garment_id`, `panel_id`, optional `trim_name`, optional `trim_category`
  (`'armor_plate' | 'accessory'`, default `'armor_plate'`).
- `tailor_keyframe_tack_strength`: set a keyframe on a tack's strength curve. Accepts
  `placement_id`, `tack_id`, `frame: u32`, `strength: f32 ∈ [0.0, 1.0]`.

**TR-12.2** All trim-domain tool responses MUST return a `SimulationReceipt`
(`hsk.tailor.simulation_receipt@1`) as `structuredContent`, carrying any relevant
`ValidationFinding` entries keyed to the check codes in TR-11.1.

---

### TR-13. CRDT Collaboration

**TR-13.1** Trim placements MUST be stored as entries in the garment's CRDT document tree under
`handshake_core::tailor`. An operator or model agent may propose a placement move (drag a button
to a new panel position) as an `AiEditProposalRequestV1` referencing the `placement_id` and the
new `initial_pose_json`. Tack `r_local` values are body-frame offsets and MUST NOT change when
the trim body is repositioned; the cloth-side `particle_uv` values MUST be recomputed when the
trim is moved to a different panel region.

**TR-13.2** Tack moves, strength-curve edits, and placement additions or removals MUST each emit
`TailorTrimTackUpdated` or `TailorTrimPlaced` as appropriate. The CRDT snapshot mechanism
(T-CONTRACTS §T-CONTRACTS.event-types: `TailorPanelCrdtSnapshotRecorded`) MUST include trim
placement state in its snapshot payload.

---

### TR-14. Implementation Constraints and Prohibitions

**TR-14.1** The `tailor-solver` crate MUST NOT take a dependency on `handshake_core`. All types
shared across the boundary (`GpuTrimBody`, `GpuTackConstraint`, `TackDefinitionV1`, `GarmentSpec`
with `trim_placements`) MUST be defined in `tailor-solver/src/` and re-exported from the
`handshake_core::tailor` binding module.

**TR-14.2** SQLite MUST NOT be used for any trim-domain authority write. The `no_sqlite_tripwire`
guard (`guard_authority_write(AuthorityMode::Postgres)`) MUST be called before every INSERT into
`tailor_trims`, `tailor_trim_placements`, `tailor_zippers`, and `tailor_lacings`.

**TR-14.3** The WGSL tack shader MUST NOT use atomic operations to resolve multi-invocation
writes to `GpuTrimBody`. Race-freedom is guaranteed exclusively by graph coloring (TR-4.4).
Any future change that would allow two invocations in the same dispatch to write the same
`body_idx` MUST be blocked by the coloring invariant, not by atomics.

**TR-14.4** `quat_pred` MUST be re-normalized after every angular correction in the tack pass.
Skipping normalization is a correctness error that accumulates quaternion drift across substeps.

**TR-14.5** Pattern-to-rigid conversion MUST be a non-destructive operation in the CRDT document:
the source panel geometry MUST be retained as a `superseded_panel` entry. Conversion MAY be
reversed by an operator edit that re-promotes the superseded panel and removes the trim body.

**TR-14.6** Numbered `0NNN_*` migration filenames are forbidden for all trim-domain tables.
All migrations MUST use the dated `2026_MM_DD_tailor_<topic>.sql` + `.down.sql` convention.

---

### TR-15. Non-Normative Notes

The mathematical basis for the tack coupling pass is Müller, Macklin, Chentanez, Jeschke, Kim,
"Detailed Rigid Body Simulation with Extended Position Based Dynamics" (SCA 2020, CGF 39(8)). The
`InteractiveComputerGraphics/PositionBasedDynamics` C++ library (`RigidBodyClothCouplingDemo.cpp`,
MIT) provides a concrete CPU-side reference implementation of the ball-joint constraint. WGSL
implementations of the effective inverse mass formula and quaternion correction are novel work;
the algorithm is established. Research notes in `13-trim-rigid.md` contain fuller OSS reference
maps, risk tables, and WGSL shader sketches as non-normative implementation guidance.

## 13.9 UV-from-Pattern & Texturing

> **Sub-section id:** `uvtexture`
> **Assembly file:** `09-uv-texture.md`
> **Non-normative provenance:** research topic T-UV-TEXTURE (`14-uv-texture.md`). That document
> provides algorithm rationale, OSS evidence, and rejected alternatives. This sub-section is
> product law; where the two conflict, this sub-section wins.

---

### Requirements Overview

This sub-section governs:

1. How UV coordinates are derived from 2D sewing pattern panels (UV-from-Pattern, MOAT-6).
2. The ARAP flatten algorithm used for post-simulation UV recomputation.
3. UV island packing into the atlas.
4. Grain direction as the single authority shared by the physics solver and the texture sampler.
5. The graphic layer data model.
6. PBR material definitions and map generation.
7. The `tailor_*` Postgres authority tables for all UV and texture state.
8. EventLedger events, schema IDs, CRDT rules, sandbox/promotion scope, and model-lane access.

---

### 1. UV-from-Pattern: Core Contract

#### 1.1 UV Coordinates Are the 2D Pattern

The Tailor engine MUST NOT compute UV coordinates by unwrapping the 3D garment surface. UV
coordinates MUST be derived directly from the 2D panel-local vertex positions.

During the pattern-to-mesh pipeline (`handshake_core::tailor`, `tailor-solver/src/mesh.rs`),
each panel vertex's 2D position in panel-local space (cm) MUST be normalized into `[0,1]^2`
preserving the panel's aspect ratio, using the panel bounding box as the normalization domain,
and stored as the vertex's UV coordinate. This normalization is lossless — the UV uniquely encodes
the panel-local position up to scale.

```rust
// tailor-solver/src/uv/assign.rs
/// Assign UV coordinates from 2D panel-local vertex positions.
/// Called once per panel during the pattern-to-mesh pipeline.
/// `panel_local_vertices_cm`: vertices in panel-local 2D space, centimetres.
/// Returns per-vertex UV in [0,1]^2 (panel bounding-box normalization, aspect-ratio-preserving).
pub fn assign_panel_uvs(panel_local_vertices_cm: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let min_x = panel_local_vertices_cm.iter().map(|v| v[0]).fold(f32::MAX, f32::min);
    let min_y = panel_local_vertices_cm.iter().map(|v| v[1]).fold(f32::MAX, f32::min);
    let max_x = panel_local_vertices_cm.iter().map(|v| v[0]).fold(f32::MIN, f32::max);
    let max_y = panel_local_vertices_cm.iter().map(|v| v[1]).fold(f32::MIN, f32::max);
    let w = (max_x - min_x).max(1e-6);
    let h = (max_y - min_y).max(1e-6);
    panel_local_vertices_cm
        .iter()
        .map(|v| [(v[0] - min_x) / w, (v[1] - min_y) / h])
        .collect()
}
```

These per-vertex UVs MUST be stored in `SolverMesh::vertex_uvs` (defined in
`tailor-solver/src/mesh.rs`). The XPBD solver MUST treat `vertex_uvs` as a read-only
per-vertex attribute; stretch/bend/collision constraints MUST NOT modify UV coordinates.

**Consequences that MUST hold as invariants:**

- Grain direction accuracy: `PanelSpec::grain_angle_deg` in pattern space equals the grain angle
  in UV space without re-alignment.
- Seam-texture alignment: texture seams align with physical seam edges by construction.
- Graphic layer preservation: a graphic layer placed at 2D pattern coordinates projects
  correctly onto the 3D draped surface without reprojection.

#### 1.2 Grain Direction Is Shared Authority

`PanelSpec::grain_angle_deg` (defined in `tailor-solver/src/spec.rs`, canonical form in
T-CONTRACTS) is the single authority for fabric grain direction. It MUST have exactly two
consumers and no other code path MUST compute grain direction independently:

1. **Physics solver** (`tailor-solver/src/fabric/material.rs`): `FabricMaterial::grain_angle_rad`
   drives the WGSL anisotropic constraint axis (warp vs weft compliance) as specified in the
   Fabric Models sub-section.

2. **Texture sampler** (this sub-section): `TailorPbrMaterial::grain_angle_deg` (mirrored from
   `PanelSpec::grain_angle_deg` at material-assignment time) rotates the UV sampling frame in
   the wgpu PBR fragment shader.

The UV coordinate frame rotation in the fragment shader MUST be implemented as:

```wgsl
// tailor-solver/src/shaders/pbr_cloth.wgsl  (fragment shader excerpt)
struct TailorMaterialUniform {
    grain_angle: f32,  // radians; converted from PanelSpec::grain_angle_deg at GPU upload
    // ... other PBR fields
}

fn rotate_uv(uv: vec2<f32>, angle: f32) -> vec2<f32> {
    let c = cos(angle);
    let s = sin(angle);
    let centered = uv - vec2<f32>(0.5, 0.5);
    return vec2<f32>(c * centered.x - s * centered.y,
                     s * centered.x + c * centered.y)
           + vec2<f32>(0.5, 0.5);
}
```

The authority chain MUST be: `PanelSpec::grain_angle_deg` (Postgres JSONB in
`tailor_garments.spec_json`) → `FabricMaterial::grain_angle_rad` (solver crate) AND
`TailorPbrMaterial::grain_angle_deg` (Postgres `tailor_pbr_materials`) → GPU uniform at render
time. Both consumers MUST read from the same authority row; neither MUST compute grain
direction independently.

---

### 2. Post-Simulation UV Flatten: ARAP

#### 2.1 Algorithm Selection

The Tailor engine MUST use ARAP (As-Rigid-As-Possible) for all post-simulation UV flatten
passes. LSCM and ABF++ MUST NOT be used for any UV flatten operation in this engine.

**Rationale (non-normative):** LSCM and ABF++ optimize angle distortion over a free boundary.
When the panel boundary is pinned — which is required here so that seam-edge UVs are stable
across retargeting — ARAP minimizes distortion under the pinned-boundary constraint more
efficiently than conformal methods. T-UV-TEXTURE (`14-uv-texture.md`) documents the
experimental evidence (PartUV arXiv 2511.16659) and the contradiction in earlier research
drafts that this decision resolves.

#### 2.2 Canonical Implementation

The flatten pass MUST be implemented as a single function `arap_unfurl_panel()` in
`tailor-solver/src/uv/unfurl.rs`. This function is the canonical flatten pass for both call
sites:

- **Bidirectional loop feedback** (T-GARMENT-AUTHORING): after a drape, the pass recomputes
  panel vertex positions in 2D for CRDT delta proposal back to the 2D pattern editor.
- **UV recompute after refit** (T-AUTOFIT): after retargeting to a new body, the pass
  recomputes UV island vertex positions to reflect the post-drape surface.

No second flatten implementation MUST exist. The function signature MUST be:

```rust
// tailor-solver/src/uv/unfurl.rs
/// ARAP flatten: project a simulated 3D panel mesh back to 2D UV space.
///
/// `mesh_3d_positions`: 3D vertex positions of one panel after simulation (metres or cm;
///     caller uses consistent units, result is in the same unit space as `boundary_uv_pins`).
/// `triangles`: triangle index list for this panel.
/// `boundary_vertex_indices`: indices of panel boundary vertices (seam edges + outline).
/// `boundary_uv_pins`: panel-local 2D UV coordinates for each boundary vertex (pinned; cm).
/// `max_iterations`: alternating local/global iterations (default 10, max 50).
/// `flatten_tolerance_cm`: convergence threshold — max interior vertex displacement between
///     iterations (cm). Default 0.001 (0.01 mm).
///
/// Returns updated UV coordinates for ALL vertices (boundary pins unchanged).
/// Emits `TailorUvFlattenCompleted` or `TailorUvFlattenProposed` via the caller's event sink.
pub fn arap_unfurl_panel(
    mesh_3d_positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    boundary_vertex_indices: &[u32],
    boundary_uv_pins: &[[f32; 2]],
    max_iterations: u32,
    flatten_tolerance_cm: f32,
) -> ArapUnfurlResult { todo!() }

pub struct ArapUnfurlResult {
    pub vertex_uvs: Vec<[f32; 2]>,
    pub iterations_taken: u32,
    pub max_displacement_cm: f32,
    pub converged: bool,
}
```

**Algorithm contract:**

```text
Input: 3D triangle mesh of ONE panel after simulation equilibrium.
       Boundary vertex indices + their original 2D panel positions (from PanelSpec).
Output: updated 2D UV coordinates for all vertices.

1. Initialize interior UV positions from the panel-local 2D coords (pre-drape).
2. Alternating local-global ARAP iteration (up to max_iterations):
   Local:  for each triangle, compute nearest rotation R_k via SVD of the deformation
           Jacobian between current 2D positions and the 3D triangle shape.
   Global: solve the sparse linear system
           sum_k(w_k * L_k^T * L_k) * u = sum_k(w_k * L_k^T * R_k * r_k)
           for interior UV positions u, with boundary rows fixed at boundary_uv_pins.
3. Stop when max interior vertex displacement < flatten_tolerance_cm OR iterations = max.
4. Return ArapUnfurlResult.
```

The sparse linear system MUST be solved with a direct Cholesky factorization. For panels
up to 5,000 triangles (typical garment authoring), this MUST converge in under 10 ms on CPU.
GPU ARAP is NOT required for v1.

---

### 3. UV Island Packing

#### 3.1 Requirements

The packing step MUST be run whenever any panel is added, removed, or resized in a garment.
The packing result for a given garment and simulation-run pair is immutable and MUST be
stored in `tailor_uv_islands` (see §7) as the authority atlas layout.

The packing MUST be:

- Deterministic: same `GarmentSpec` → same atlas layout across sessions and platforms.
- Overlap-free: no two UV islands MUST overlap in the packed atlas.
- No image dependency: the packing algorithm MUST operate on abstract rectangle sizes, not pixel
  buffers.

#### 3.2 Implementation: `rectangle-pack` Crate

UV island packing MUST use the `rectangle-pack` Rust crate
(`https://crates.io/crates/rectangle-pack`, MIT/Apache-2.0). This crate provides deterministic
MaxRects-style packing with no image dependency.

```rust
// tailor-solver/src/uv/packing.rs

/// Per-panel UV island placement in the packed atlas.
/// Stored as authority data in tailor_uv_islands.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct UvIslandPlacement {
    /// Matches PanelSpec::panel_id.
    pub panel_id: String,
    /// Top-left UV coordinate of the island bounding box in the atlas [0,1]^2.
    pub atlas_uv_min: [f32; 2],
    /// Bottom-right UV coordinate of the island bounding box in the atlas [0,1]^2.
    pub atlas_uv_max: [f32; 2],
    /// 90°-increment rotation applied (0, 90, 180, 270 deg). V1: always 0.
    pub rotation_deg: u32,
    /// Island width and height in normalized UV units before packing.
    pub island_size_uv: [f32; 2],
}

/// Pack all panel UV islands for a garment into a square [0,1]^2 atlas.
/// Returns placements in the same order as `panels`.
/// `panels`: (panel_id, panel_bounding_box_size_cm).
/// `panel_cm_to_atlas`: scale — cm per 1.0 atlas unit.
/// `padding`: gap between islands in atlas units.
pub fn pack_uv_islands(
    panels: &[(String, [f32; 2])],
    panel_cm_to_atlas: f32,
    padding: f32,
) -> Result<Vec<UvIslandPlacement>, rectangle_pack::RectanglePackError> {
    todo!()
}
```

The atlas coordinates stored in `tailor_uv_islands` are the authority. Vertex UVs in
`SolverMesh::vertex_uvs` remain panel-local (pre-packing). The vertex shader MUST apply the
atlas transform as an affine remap at render time:

```wgsl
// tailor-solver/src/shaders/pbr_cloth.wgsl  (vertex shader excerpt)
struct UvIslandTransform {
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
}
// island_transforms: storage buffer indexed by per-vertex panel_index
let island = island_transforms[vertex_panel_index];
let atlas_uv = island.uv_min + vertex_uv * (island.uv_max - island.uv_min);
```

#### 3.3 Atlas Fill Ratio

After every packing run, the implementation MUST compute and record `atlas_fill_ratio`
(sum of island areas / atlas area) in the `TailorUvIslandsPacked` event payload. If
`atlas_fill_ratio < 0.4`, the implementation MUST emit the ratio in the event and surface
a warning in the operator control room. Low fill ratios are advisory and MUST NOT block
simulation or promotion.

---

### 4. Graphic Layer Data Model

#### 4.1 Data Type

Graphic layers (prints, logos, embroidery, topstitching artwork) MUST be stored as positioned
rectangles in panel-local 2D space (cm). The canonical type is:

```rust
// tailor-solver/src/texture.rs

/// A graphic overlay placed on a panel in 2D pattern space.
/// schema_id: "hsk.tailor.graphic_layer@1"
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TailorGraphicLayer {
    /// Authority id: "GLYR-{uuid_v7}". Matches tailor_graphic_layers.layer_id.
    pub layer_id: String,
    pub garment_id: String,
    pub panel_id: String,
    /// Composite order; higher = rendered on top within the panel.
    pub z_order: i32,
    /// SHA-256 content-addressed artifact ref. Image MUST be PNG, JPEG, or WebP.
    pub image_artifact_ref: String,
    /// Bounding box in panel-local 2D space (cm): [x_min, y_min, x_max, y_max].
    pub panel_bbox_cm: [f32; 4],
    /// Rotation of the graphic relative to panel horizontal (degrees).
    pub rotation_deg: f32,
    pub blend_mode: GraphicBlendMode,
    /// Opacity scalar [0.0, 1.0].
    pub opacity: f32,
    /// When true, the graphic corner vertices are added to the ARAP pinned-boundary set,
    /// preventing refit UV displacement from moving the graphic. MUST default to true for
    /// design elements (logos, prints, embroidery). MUST be false only for seamless
    /// full-panel background textures.
    pub boundary_pinned: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GraphicBlendMode {
    Normal,    // standard alpha compositing over fabric
    Multiply,  // darker overlay (screen-print look)
    Screen,    // lighter overlay (foil/metallic look)
    Overlay,   // high-contrast texture detail
    Emboss,    // additive normal-map contribution (embroidery relief)
}
```

#### 4.2 Compositing Order

The wgpu fragment shader MUST apply layers in this bottom-to-top order:

```text
1. Base fabric texture (tileable, grain-rotated) from TailorPbrMaterial::base_color_texture_ref
2. Graphic layer stack (ascending z_order within the panel)
3. Normal map (contributes to PBR lighting calculation, not final color directly)
4. PBR channel maps: roughness, metalness, ambient-occlusion, displacement
```

#### 4.3 Boundary Pinning in ARAP

When an `arap_unfurl_panel()` call is triggered, the implementation MUST add the four corner
vertices of every `boundary_pinned = true` graphic layer on that panel to the set of pinned
boundary vertices, alongside the panel seam-edge vertices. The ARAP global step MUST treat
these as fixed pins so the graphic does not translate or distort relative to the panel shape.

#### 4.4 Coordinate Pipeline

The graphic layer coordinate pipeline MUST be:

```text
panel_bbox_cm  →  normalized panel UV coords  →  atlas UV coords (via UvIslandTransform)
```

A graphic layer specified in panel-local 2D space MUST project correctly onto the 3D draped
surface without 3D reprojection at any stage.

---

### 5. PBR Material System

#### 5.1 TailorPbrMaterial Type

The canonical PBR material type is:

```rust
// tailor-solver/src/texture.rs  (continued)

/// PBR render-side material for a garment panel.
/// schema_id: "hsk.tailor.pbr_material@1"
/// Physics-side complement is FabricMaterial (T-FABRIC-MODELS).
/// Both share grain_angle_deg from PanelSpec as authority.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TailorPbrMaterial {
    /// Authority id: "PBR-{uuid_v7}". Matches tailor_pbr_materials.material_id.
    pub material_id: String,
    pub workspace_id: String,
    pub name: String,
    /// Tileable base color / albedo texture. None = use base_color_srgb solid.
    pub base_color_texture_ref: Option<String>,
    /// Solid base color [r, g, b, a] in sRGB [0,1]. Used when no texture ref.
    pub base_color_srgb: [f32; 4],
    /// Tangent-space normal map (OpenGL Y-up convention). R/G/B channels.
    pub normal_map_ref: Option<String>,
    /// Roughness map (R channel). None = use roughness_scalar.
    pub roughness_map_ref: Option<String>,
    /// Roughness scalar [0,1] used when roughness_map_ref is None.
    pub roughness_scalar: f32,
    /// Metalness map (R channel). None = use metalness_scalar.
    pub metalness_map_ref: Option<String>,
    /// Metalness scalar [0,1]. MUST default to 0.0 for non-metallic fabrics.
    pub metalness_scalar: f32,
    /// Height/displacement map (R channel). None = no displacement.
    pub displacement_map_ref: Option<String>,
    /// Surface relief in cm. 0.0 = flat (no displacement).
    pub displacement_scale_cm: f32,
    /// Opacity map (R channel). None = fully opaque.
    pub opacity_map_ref: Option<String>,
    /// Opacity scalar [0,1]. Multiplied with opacity map if present.
    pub opacity_scalar: f32,
    /// Additive emissive glow [r, g, b, a] sRGB. MUST default to [0,0,0,0].
    pub emissive_color_srgb: [f32; 4],
    /// Grain direction in degrees from panel horizontal.
    /// Mirrored from PanelSpec::grain_angle_deg at material-assignment time.
    /// Applied as UV frame rotation in the PBR fragment shader (see §1.2).
    pub grain_angle_deg: f32,
    /// How many cm of garment surface maps to one tile of the base texture.
    /// Example: 10.0 → texture tiles every 10 cm.
    pub texture_tile_size_cm: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

#### 5.2 Material Assignment per Panel

The complete render material for a panel is the combination of a physics preset
(`tailor_material_presets`, T-FABRIC-MODELS) and a PBR material (`tailor_pbr_materials`),
linked via `tailor_material_assignments`:

```rust
// tailor-solver/src/texture.rs  (continued)

/// Links physics preset + PBR material + graphic layers per panel.
/// schema_id: "hsk.tailor.material_assignment@1"
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct TailorMaterialAssignment {
    /// Authority id: "ASGN-{uuid_v7}".
    pub assignment_id: String,
    pub garment_id: String,
    /// Matches PanelSpec::panel_id.
    pub panel_id: String,
    /// FK into tailor_material_presets. None = inherit garment-level default.
    pub physics_preset_id: Option<String>,
    /// FK into tailor_pbr_materials. None = system default (white-cotton appearance).
    pub pbr_material_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

A `None` physics preset MUST cause the garment-level default preset to be used. A `None` PBR
material MUST cause the system-default material (white cotton appearance) to be used. This
allows simulation to begin before texturing is complete.

#### 5.3 PBR Map Generator

The PBR map generator — equivalent to MD's PBR Map Generator (2024.1) — MUST run as a Tauri
command on the operator workstation. The `tailor-solver` crate MUST NOT produce PBR maps.

```rust
// app/src-tauri/src/commands/tailor_texture.rs

#[tauri::command]
pub async fn tailor_generate_pbr_maps(
    base_color_artifact_ref: String,
    fabric_archetype: String,   // "cotton" | "silk" | "denim" | "leather" | ...
    options: PbrMapGenOptions,
    state: tauri::State<'_, AppState>,
) -> Result<PbrMapGenResult, String> { todo!() }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct PbrMapGenOptions {
    pub gen_normal: bool,
    /// Normal map strength multiplier (1.0 = standard).
    pub normal_strength: f32,
    pub gen_roughness: bool,
    /// Base roughness blended with texture-derived roughness.
    pub roughness_base: f32,
    pub gen_metalness: bool,
    pub metalness_base: f32,
    pub gen_displacement: bool,
    pub displacement_scale_cm: f32,
    /// Generation algorithm: "sobel" (default) or "weave_matrix" (structured weave fabrics).
    pub gen_mode: PbrGenMode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PbrGenMode {
    /// Sobel-gradient luminance → normal/roughness/metalness/displacement maps.
    Sobel,
    /// Binary weave-pattern matrix + 1D yarn cross-section profiles (Khattar et al., CGF 2025).
    /// Produces analytical normals; eliminates normal-map artifacts for structured weave fabrics.
    WeaveMatrix,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PbrMapGenResult {
    pub normal_map_ref: Option<String>,
    pub roughness_map_ref: Option<String>,
    pub metalness_map_ref: Option<String>,
    pub displacement_map_ref: Option<String>,
    /// EventLedger receipt for the generation run.
    pub event_ledger_event_id: String,
}
```

The `Sobel` mode MUST implement per-channel map generation using the `image` Rust crate:

| Map | Algorithm |
|-----|-----------|
| Normal | Sobel gradient on luminance → `(dx, dy, 1.0)` normalized → RGB normal (OpenGL Y-up) |
| Roughness | Invert luminance; clamp to `[roughness_base, 1.0]` |
| Metalness | Constant `metalness_base` (0.0 for standard fabrics); luminance-threshold mask for metallic-thread patterns |
| Displacement | Luminance scaled to `displacement_scale_cm` range |
| Opacity | Derived from base-color alpha channel only; omitted if alpha is fully opaque |

The `tailor_generate_pbr_maps` Tauri command MUST be async. It MUST return an
`operation_id` immediately and deliver the `PbrMapGenResult` via a Tauri emit event when
processing completes. It MUST NOT block the Tailor panel UI.

---

### 6. Postgres Authority Tables

All UV and texture authority MUST reside in PostgreSQL. SQLite MUST NOT be used for any
table defined in this sub-section. Every INSERT into these tables MUST call
`guard_authority_write(AuthorityMode::Postgres)` before the `sqlx::query!()` macro
(mirroring the `kb003_storage.rs` no-SQLite tripwire pattern).

Migration files MUST follow the dated convention from T-CONTRACTS
(`2026_MM_DD_tailor_texture_tables.sql` + `.down.sql` reverse pair). Numbered `0NNN_*`
migration names MUST NOT be used.

All primary keys MUST use `TEXT PRIMARY KEY` with prefixed string IDs (T-CONTRACTS FACT-4):

```sql
-- Migration: 2026_MM_DD_tailor_texture_tables.sql
-- (Date assigned at implementation WP authoring time; not hardcoded here.)
-- Required reverse pair: 2026_MM_DD_tailor_texture_tables.down.sql

-- tailor_uv_islands: authority UV island placement for a garment + simulation run pair.
-- Repacked whenever panels change; immutable for a given (garment_id, simulation_run_id) pair.
CREATE TABLE IF NOT EXISTS tailor_uv_islands (
    island_id               TEXT PRIMARY KEY,          -- "UVI-{uuid_v7}"
    garment_id              TEXT NOT NULL
        REFERENCES tailor_garments (garment_id),
    simulation_run_id       TEXT,                      -- NULL = authoring-time packing (pre-sim)
    panel_id                TEXT NOT NULL,             -- matches PanelSpec::panel_id
    atlas_uv_min_x          FLOAT4 NOT NULL,
    atlas_uv_min_y          FLOAT4 NOT NULL,
    atlas_uv_max_x          FLOAT4 NOT NULL,
    atlas_uv_max_y          FLOAT4 NOT NULL,
    rotation_deg            INT NOT NULL DEFAULT 0
        CHECK (rotation_deg IN (0, 90, 180, 270)),
    island_width_uv         FLOAT4 NOT NULL,
    island_height_uv        FLOAT4 NOT NULL,
    flatten_method          TEXT NOT NULL DEFAULT 'arap'
        CHECK (flatten_method = 'arap'),               -- 'arap' is the only valid value
    atlas_fill_ratio        FLOAT4,                    -- recorded at pack time; advisory
    event_ledger_event_id   TEXT REFERENCES kernel_event_ledger (event_id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX IF NOT EXISTS ix_tailor_uv_islands_panel_run
    ON tailor_uv_islands (garment_id, simulation_run_id, panel_id);
CREATE INDEX IF NOT EXISTS ix_tailor_uv_islands_garment
    ON tailor_uv_islands (garment_id, simulation_run_id);

-- tailor_pbr_materials: render-side PBR material definitions.
CREATE TABLE IF NOT EXISTS tailor_pbr_materials (
    material_id             TEXT PRIMARY KEY,          -- "PBR-{uuid_v7}"
    workspace_id            TEXT NOT NULL,
    name                    TEXT NOT NULL,
    base_color_texture_ref  TEXT,                      -- artifact content hash or NULL
    base_color_srgb         FLOAT4[4] NOT NULL DEFAULT '{1,1,1,1}',
    normal_map_ref          TEXT,
    roughness_map_ref       TEXT,
    roughness_scalar        FLOAT4 NOT NULL DEFAULT 0.8
        CHECK (roughness_scalar BETWEEN 0.0 AND 1.0),
    metalness_map_ref       TEXT,
    metalness_scalar        FLOAT4 NOT NULL DEFAULT 0.0
        CHECK (metalness_scalar BETWEEN 0.0 AND 1.0),
    displacement_map_ref    TEXT,
    displacement_scale_cm   FLOAT4 NOT NULL DEFAULT 0.0,
    opacity_map_ref         TEXT,
    opacity_scalar          FLOAT4 NOT NULL DEFAULT 1.0
        CHECK (opacity_scalar BETWEEN 0.0 AND 1.0),
    emissive_color_srgb     FLOAT4[4] NOT NULL DEFAULT '{0,0,0,0}',
    grain_angle_deg         FLOAT4 NOT NULL DEFAULT 0.0,
    texture_tile_size_cm    FLOAT4 NOT NULL DEFAULT 10.0
        CHECK (texture_tile_size_cm > 0.0),
    is_system_preset        BOOLEAN NOT NULL DEFAULT false,
    event_ledger_event_id   TEXT REFERENCES kernel_event_ledger (event_id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_pbr_materials_workspace
    ON tailor_pbr_materials (workspace_id);

-- tailor_graphic_layers: graphic overlays positioned in panel-local 2D space (cm).
CREATE TABLE IF NOT EXISTS tailor_graphic_layers (
    layer_id                TEXT PRIMARY KEY,          -- "GLYR-{uuid_v7}"
    garment_id              TEXT NOT NULL
        REFERENCES tailor_garments (garment_id),
    panel_id                TEXT NOT NULL,             -- matches PanelSpec::panel_id
    z_order                 INT NOT NULL DEFAULT 0,
    image_artifact_ref      TEXT NOT NULL,
    panel_bbox_cm           FLOAT4[4] NOT NULL,        -- [x_min, y_min, x_max, y_max] in cm
    rotation_deg            FLOAT4 NOT NULL DEFAULT 0.0,
    blend_mode              TEXT NOT NULL DEFAULT 'normal'
        CHECK (blend_mode IN ('normal','multiply','screen','overlay','emboss')),
    opacity                 FLOAT4 NOT NULL DEFAULT 1.0
        CHECK (opacity BETWEEN 0.0 AND 1.0),
    boundary_pinned         BOOLEAN NOT NULL DEFAULT true,
    deleted_at              TIMESTAMPTZ,               -- NULL = active; non-NULL = tombstoned
    event_ledger_event_id   TEXT REFERENCES kernel_event_ledger (event_id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_graphic_layers_garment_panel
    ON tailor_graphic_layers (garment_id, panel_id, z_order)
    WHERE deleted_at IS NULL;

-- tailor_material_assignments: links physics preset + PBR material per panel.
-- Graphic layers are queried separately via tailor_graphic_layers.
CREATE TABLE IF NOT EXISTS tailor_material_assignments (
    assignment_id           TEXT PRIMARY KEY,          -- "ASGN-{uuid_v7}"
    garment_id              TEXT NOT NULL
        REFERENCES tailor_garments (garment_id),
    panel_id                TEXT NOT NULL,
    physics_preset_id       TEXT
        REFERENCES tailor_material_presets (preset_id),
    pbr_material_id         TEXT
        REFERENCES tailor_pbr_materials (material_id),
    event_ledger_event_id   TEXT REFERENCES kernel_event_ledger (event_id),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (garment_id, panel_id)
);
CREATE INDEX IF NOT EXISTS ix_tailor_material_assignments_garment
    ON tailor_material_assignments (garment_id);
```

---

### 7. Schema IDs

The canonical schema-ID constants for this sub-section (from T-CONTRACTS `hsk.tailor.*`
namespace, in `src/tailor/schemas.rs`) are:

```rust
pub const SCHEMA_TAILOR_PBR_MATERIAL_V1:   &str = "hsk.tailor.pbr_material@1";
pub const SCHEMA_TAILOR_GRAPHIC_LAYER_V1:  &str = "hsk.tailor.graphic_layer@1";
pub const SCHEMA_TAILOR_UV_ISLAND_V1:      &str = "hsk.tailor.uv_island@1";
```

`hsk.cloth.*` schema IDs MUST NOT be used for any type defined in this sub-section.

---

### 8. EventLedger Events

The following variants MUST be added to `KernelEventType` in `kernel/mod.rs` and MUST be
registered in `required_first_slice_events()`. Variant names are `Tailor*` PascalCase; wire
strings are `TAILOR_*` SCREAMING_SNAKE_CASE via `as_str()` (T-CONTRACTS FACT-1):

```rust
// UV domain
TailorUvIslandsPacked,           // "TAILOR_UV_ISLANDS_PACKED"
TailorUvFlattenCompleted,        // "TAILOR_UV_FLATTEN_COMPLETED"
TailorUvFlattenProposed,         // "TAILOR_UV_FLATTEN_PROPOSED"

// Texture / material domain
TailorPbrMaterialCreated,        // "TAILOR_PBR_MATERIAL_CREATED"
TailorPbrMaterialUpdated,        // "TAILOR_PBR_MATERIAL_UPDATED"
TailorPbrMapsGenerated,          // "TAILOR_PBR_MAPS_GENERATED"
TailorGraphicLayerAdded,         // "TAILOR_GRAPHIC_LAYER_ADDED"
TailorGraphicLayerUpdated,       // "TAILOR_GRAPHIC_LAYER_UPDATED"
TailorGraphicLayerRemoved,       // "TAILOR_GRAPHIC_LAYER_REMOVED"
TailorMaterialAssignmentUpdated, // "TAILOR_MATERIAL_ASSIGNMENT_UPDATED"
```

The `event_family` constants for these events (in `src/tailor/event_family.rs`) are:

```rust
pub const TAILOR_UV:      &str = "tailor.uv";
pub const TAILOR_TEXTURE: &str = "tailor.texture";
```

**Required payload fields per event type:**

- `TailorUvIslandsPacked`: `garment_id`, `simulation_run_id` (nullable), `island_count`,
  `atlas_fill_ratio`, `packing_algorithm: "rectangle_pack_maxrects"`.
- `TailorUvFlattenCompleted`: `garment_id`, `panel_id`, `max_displacement_cm`,
  `iterations_taken`, `converged: bool`.
- `TailorUvFlattenProposed`: same as `TailorUvFlattenCompleted` plus `crdt_proposal_id`
  (the CRDT delta proposal ref in the bidirectional loop context).
- `TailorPbrMapsGenerated`: `base_color_artifact_ref`, `generated_map_refs` (JSON object
  keyed by map type), `fabric_archetype`, `gen_mode`.

---

### 9. CRDT Collaboration

Graphic layer positions and PBR material property edits MUST use the existing
`CrdtUpdateRecordV1` table and `yjs_bridge` serialization. Within a garment's CRDT document,
the material assignment and graphic layer subtree MUST be CRDT maps keyed by `panel_id`.

Conflict resolution rules:

- PBR material field edits: last-write-wins per field (each property is an independent CRDT
  map entry).
- Graphic layer `z_order`: concurrent reordering MUST merge as a CRDT sequence. The merged
  order SHOULD be surfaced to the operator for confirmation if it differs from both
  participants' intent; the implementation MUST emit `TailorUvFlattenProposed` for operator
  review in this case.
- Graphic layer deletions: MUST use tombstone-based soft delete (`deleted_at TIMESTAMPTZ`);
  hard deletes from `tailor_graphic_layers` are PROHIBITED.

---

### 10. Sandbox and Promotion Scope

UV packing (`pack_uv_islands`) and PBR map generation (`tailor_generate_pbr_maps`) are
deterministic CPU transforms. They MUST NOT go through the `SandboxAdapter` / `PromotionGate`
pipeline. They write to authority immediately on completion.

The `SandboxAdapter` / `PromotionGate` pipeline MUST be applied to:

- Model-authored PBR material presets (via the `tailor_pbr_material_create` MCP tool call):
  enters the sandbox as a `TailorMaterialPresetRecorded` event; promotion gated by the
  `TailorValidationDescriptor`.
- Model-placed graphic layers (via the `tailor_graphic_layer_add` MCP tool call): enters the
  sandbox as a CRDT `ai_edit_proposal` in `kernel/crdt/ai_edit_proposal.rs`; requires operator
  acceptance before the `tailor_graphic_layers` row is written.

Direct operator actions — texture upload, PBR parameter editing, graphic layer placement via
the UI — MUST write to authority immediately without a sandbox pass.

---

### 11. Validation Checks

The UV and texture validation checks from the canonical `TailorValidationDescriptor` catalog
(T-CONTRACTS) that apply to this sub-section are:

| Check code | Severity | Stage | Assertion |
|------------|----------|-------|-----------|
| `UV_COVERAGE` | Blocking | post | UV islands cover >= 95% of mesh surface (pattern accuracy) |
| `UV_VALIDITY` | Blocking | post | All UVs in `[0,1]^2`; no degenerate UV triangles (area > 1e-6) |

These checks MUST run as part of the post-simulation validation gate. Any `Blocking` failure
MUST prevent promotion. `ValidationReport::aggregate_blocks_promotion()` is the canonical
decision method.

The ARAP flatten convergence warning (`converged: false` in `ArapUnfurlResult`) MUST be
emitted in the `TailorUvFlattenCompleted` event payload but MUST NOT be a blocking validation
check. It is advisory.

---

### 12. Model-Lane (MCP Tool) Access

The following MCP tools MUST be exposed for model-lane UV and texture access. All are in
`src/tailor/mcp_tools.rs` or equivalent. Tool names use the `tailor_` prefix.

```rust
// Tool: tailor_uv_inspect
// Input:  { garment_id: String, simulation_run_id: Option<String> }
// Output: { islands: Vec<UvIslandPlacement>, atlas_fill_ratio: f32, panel_count: u32 }
// Access: read-only; no sandbox.

// Tool: tailor_pbr_material_create
// Input:  TailorPbrMaterial fields (minus id/timestamps; caller sets workspace_id and name)
// Output: { material_id: String, event_ledger_event_id: String }
// Access: write; enters sandbox for model-authored presets.
// Constraint: base_color_texture_ref MUST reference an already-uploaded artifact.

// Tool: tailor_material_assign
// Input:  { garment_id: String, panel_id: String, pbr_material_id: Option<String>,
//           physics_preset_id: Option<String> }
// Output: { assignment_id: String, event_ledger_event_id: String }
// Access: write; direct authority write (not sandboxed — assignment, not garment geometry).

// Tool: tailor_graphic_layer_add
// Input:  { garment_id: String, panel_id: String, image_artifact_ref: String,
//           panel_bbox_cm: [f32;4], rotation_deg: f32, blend_mode: String, opacity: f32,
//           boundary_pinned: bool }
// Output: { proposal_id: String, layer_id: String }
// Access: write; enters CRDT ai_edit_proposal sandbox; requires operator acceptance.

// Tool: tailor_generate_pbr_maps
// Input:  { base_color_artifact_ref: String, fabric_archetype: String,
//           options: PbrMapGenOptions }
// Output: PbrMapGenResult (artifact refs + event_ledger_event_id)
// Access: Tauri command (operator workstation CPU); async; MUST NOT block UI.
```

A model completing a texturing pass after simulation SHOULD follow this sequence:

1. Call `tailor_uv_inspect` to read island placements and verify `atlas_fill_ratio >= 0.4`.
2. If a reference texture artifact is available, call `tailor_generate_pbr_maps` to obtain
   PBR map artifact refs.
3. Call `tailor_pbr_material_create` with the generated map refs and `grain_angle_deg` from
   the relevant `PanelSpec`.
4. Call `tailor_material_assign` for each panel.
5. Call `tailor_capture_frame` (T-RENDER-VIEWPORT) to obtain a visual snapshot.
6. Inspect the snapshot via a vision call to verify grain direction, texture alignment, and
   seam continuity before promoting.

---

### 13. Post-MVP Deferred Items

The following items are explicitly deferred and MUST NOT be implemented in v1:

- **UDIM tile support:** Each panel occupying a separate `[0,1]^2` UV tile requires an
  additive `udim_tile_index INT NULL` column on `tailor_uv_islands`. This is a backward-compatible
  schema addition; defer to post-MVP. The `tailor_uv_islands` migration MUST NOT include this
  column in v1.
- **UV-space texture bake:** Baking the rendered 3D garment surface (including wrinkle-baked AO)
  back to UV texture space requires a UV-space wgpu render pass. Defer to post-MVP.
- **90°-increment island rotation in packing:** The `rotation_deg` field on `UvIslandPlacement`
  and `tailor_uv_islands` is scaffolded; v1 MUST always write `rotation_deg = 0`. Rotation
  support is deferred to post-MVP.
- **FabricDiffusion integration:** An optional path routing `tailor_generate_pbr_maps` through
  a local FabricDiffusion inference server (arXiv 2410.01801, SIGGRAPH Asia 2024) for
  distortion-free texture extraction from reference photographs is a post-MVP upgrade path.
- **All-quad mesh conversion:** Post-sim quad conversion (UV islands survive the conversion)
  is deferred per T-GARMENT-AUTHORING.
- **Fur strand material:** Out of scope for MVP.
- **Toon shader:** A wgpu toon pass in the viewport is a render concern outside UV/texture
  scope; deferred per T-RENDER-VIEWPORT.

---

### 14. Risks and Mitigations

**R1 — ARAP convergence for heavily gathered panels.**
Panels with `gather_ratio > 3.0` (heavily gathered skirt panels) may not converge within the
default 10 iterations. Mitigation: the `max_iterations` parameter on `arap_unfurl_panel()`
MUST default to 10 and MUST accept up to 50. If the result exceeds `flatten_tolerance_cm`
after `max_iterations`, the implementation MUST emit a `TailorUvFlattenCompleted` event with
`converged: false` and the residual `max_displacement_cm`. This MUST NOT block export or
promotion. The UV produced is the best achievable for this drape state.

**R2 — UV packing fill ratio for irregular panel silhouettes.**
Axis-aligned bounding-box packing wastes atlas space for angled or L-shaped panel outlines.
Mitigation: record `atlas_fill_ratio` in `TailorUvIslandsPacked` and surface a warning if
below 0.4. Accept this for MVP. Post-MVP: implement polygon-outline packing.

**R3 — Grain direction drift across ARAP refit.**
The ARAP result may introduce a mean rotation to UV island interiors that causes effective
grain direction to drift from `PanelSpec::grain_angle_deg`. Mitigation: after the ARAP unfurl
pass, the implementation MUST compute the mean rotation of the updated UV island relative to
the initial UV orientation. If `|mean_rotation_deg| > 2.0°`, it MUST record
`grain_correction_deg` in the `TailorUvIslandsPacked` event payload. The texture sampler
MUST apply `grain_angle_deg + grain_correction_deg` as the total UV rotation uniform. If
`|grain_correction_deg| > 2.0°`, the implementation MUST surface a visible indicator in the
operator control room.

**R4 — Graphic layer position invalidated by ARAP refit.**
If `boundary_pinned = false` and ARAP refit significantly shifts interior UV positions, the
graphic may appear in the wrong location on the 3D garment. Mitigation: `boundary_pinned`
MUST default to `true` in `TailorGraphicLayer`. The `tailor_graphic_layers` schema enforces
this default. Operators MUST be warned via the control room UI if they set
`boundary_pinned = false` on a design-element layer.

**R5 — PBR normal map artifacts for structured weave textures.**
Sobel-gradient normal map generation produces poor results for repeating geometric weave
patterns (twill, herringbone). Mitigation: the `WeaveMatrix` `gen_mode` in `PbrMapGenOptions`
generates normals analytically from a binary weave matrix and 1D yarn cross-section profiles
(Khattar et al., CGF 2025), eliminating normal artifacts for procedural weave fabrics.
Operators working with structured weave fabrics SHOULD use `gen_mode = WeaveMatrix`.

## 13.10 Animation & Keyframe Timeline

---

##### Scope

This sub-section specifies the Tailor animation and keyframe timeline system. It covers the
keyframe data model, animation authority storage, CRDT collaborative timeline, animation-driven
simulation loop, animation import (FBX/glTF), animation-range export, and the model-first
animation API. It closes **MOAT-4** (per-substep keyframeable physical properties during
simulation) and **Group 6** (Animation and Dynamics) of the Tailor feature set.

Canonical contracts — type names, field names, units, event variants, schema IDs, table columns,
migration naming, validation checks, and promotion equivalence — are governed by **T-CONTRACTS**
(`16-contracts.md`). This sub-section uses those contracts verbatim and MUST NOT reintroduce
drift. Normative references are cited as `[T-CONTRACTS.<section>]` throughout.

Non-normative design rationale and OSS evaluation evidence lives in the research source
`15-animation.md`; this sub-section does not repeat it.

---

##### 10.1 Keyframe Data Model

###### 10.1.1 Track Types

A Tailor garment animation is a set of **typed keyframe tracks**. Each track targets one
animatable property. The canonical track set is:

| Track | Target solver property | Value type | Per-substep upload |
|---|---|---|---|
| `MaterialPressureTrack` | `GpuSimParams.pressure_target` | `f32` | Yes |
| `MaterialSolidifyTrack` | `GpuSimParams.solidify_blend` | `f32` | Yes |
| `MaterialShrinkWeftTrack` | `GpuSimParams.shrink_u` | `f32` | Yes |
| `MaterialShrinkWarpTrack` | `GpuSimParams.shrink_v` | `f32` | Yes |
| `TackComplianceTrack` | per-tack `compliance` in `GpuSeamConstraint` | `f32` (per tack) | Yes (post-MVP; see §10.6.1) |
| `WindStrengthTrack` | `GpuSimParams.wind` magnitude | `f32` | Yes |
| `WindDirectionTrack` | `GpuSimParams.wind` direction | `Vec3` (slerp) | Yes |
| `WindTurbulenceTrack` | turbulence scale | `f32` | Yes |
| `AvatarPoseTrack` | capsule proxy positions + orientations per bone | `Vec3` / `Quat` | Per-frame (capsule buffer upload) |
| `AvatarBlendShapeTrack` | avatar morph-target weights | `f32` | Per-frame |
| `MarkerTrack` | named timeline markers | `String` (STEP only) | No |

**R-ANIM-001** — The `tailor-solver` crate MUST NOT evaluate keyframe tracks. Track sampling
MUST be performed entirely in `handshake_core::tailor::animation` before each solver substep or
frame, and only the resolved scalar or vector value MUST be passed to the solver.

###### 10.1.2 Interpolation Modes

**R-ANIM-002** — Implementations MUST support the following three interpolation modes on all
value-bearing tracks:

```rust
// tailor-solver/src/animation/track.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KeyframeInterpolation {
    /// Linear lerp between consecutive keyframes. Default for material and wind tracks.
    Linear,
    /// Hold the value of the preceding keyframe until the next keyframe begins.
    Step,
    /// Cubic Hermite spline using in/out tangents. Required for avatar pose tracks.
    /// Tangent encoding matches glTF 2.0 CUBICSPLINE (in_tangent, value, out_tangent per keyframe).
    CubicSpline,
}
```

**R-ANIM-003** — The `CubicSpline` mode MUST implement glTF 2.0 cubic Hermite interpolation:

```
p(u) = (2u³ - 3u² + 1)·p₀ + (u³ - 2u² + u)·dt·m₀ + (-2u³ + 3u²)·p₁ + (u³ - u²)·dt·m₁
```

where `u = (t - t₀) / (t₁ - t₀)`, `dt = t₁ - t₀`, `m₀` is the out-tangent of the preceding
keyframe, and `m₁` is the in-tangent of the following keyframe.

**R-ANIM-004** — Quaternion rotation tracks (`AvatarPoseTrack` rotation channel) MUST use
`glam::Quat::slerp` for `Linear` interpolation and MUST de-interleave glTF CUBICSPLINE
`[in_tangent, value, out_tangent]` per-keyframe layout before storing tangents.

**R-ANIM-005** — The `MarkerTrack` MUST use `Step` interpolation only. Markers carry no
interpolated value; they carry a stable `marker_id`, a `frame` index, a human-readable `label`,
and an optional `color: [f32; 3]` (RGB 0–1 for timeline display).

###### 10.1.3 Track Evaluation Contract

**R-ANIM-006** — A `KeyframeTrack<T>` MUST return `default_value` when the track contains no
keyframes, MUST clamp to the first keyframe value before `t = 0`, and MUST clamp to the last
keyframe value after the final keyframe. It MUST NOT extrapolate.

**R-ANIM-007** — The `Lerpable` trait MUST be implemented for at minimum `f32`, `glam::Vec3`,
and `glam::Quat`. The `tailor-solver` crate MUST NOT take a mandatory dependency on any external
animation crate (`keyframe`, `spanda`, or equivalent) for this implementation; the track
evaluation is a bounded ~80-line Rust function.

---

##### 10.2 Animation Authority: `GarmentAnimationDraftV1`

###### 10.2.1 Storage

**R-ANIM-008** — The animation authority for a garment MUST be stored as a `JSONB` column
`animation_json` on the `tailor_garments` row. It MUST NOT be a separate first-class table.
`animation_json` is `NULL` when no animation has been authored; it is set when the operator or
a model creates the first animation draft.

```sql
-- Added via a dated migration: migrations/YYYY_MM_DD_tailor_garments_animation_col.sql
-- (Naming per [T-CONTRACTS.migration-naming]: dated, with .down.sql pair, no 0NNN_ prefix.)
ALTER TABLE tailor_garments ADD COLUMN IF NOT EXISTS
    animation_json JSONB;
```

**R-ANIM-009** — The schema ID for the animation draft MUST be
`"hsk.tailor.garment_animation_draft@1"` per `[T-CONTRACTS.schema-ids]`. The
`hsk.cloth.garment_animation_draft@1` form used in research drafts is superseded and MUST NOT
appear in production code.

###### 10.2.2 `GarmentAnimationDraftV1` Structure

```rust
// handshake_core/src/tailor/animation/draft.rs

/// Top-level animation draft for one garment. Serialized into tailor_garments.animation_json.
/// Schema ID: "hsk.tailor.garment_animation_draft@1"  [T-CONTRACTS.schema-ids]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GarmentAnimationDraftV1 {
    /// MUST equal "hsk.tailor.garment_animation_draft@1".
    pub schema_id: String,
    /// Authority garment id. Form: "GAR-{uuid_v7}".  [T-CONTRACTS.tables]
    pub garment_id: String,
    /// Frames per second. Range: [1.0, 120.0]. Default: 30.0.
    pub fps: f32,
    /// Total animation length in frames. MVP cap: 1800 (60 s at 30 fps).
    pub total_frames: u32,
    /// Keyframeable material property tracks (MOAT-4 core).
    pub material_tracks: MaterialAnimationTracks,
    /// Wind keyframe tracks.
    pub wind_tracks: WindAnimationTracks,
    /// Avatar pose tracks — one entry per animated bone.
    pub pose_tracks: Vec<AvatarBonePoseTrack>,
    /// Avatar morph-target weight tracks.
    pub blend_shape_tracks: Vec<BlendShapeTrack>,
    /// Named timeline markers.  [§10.1.2, R-ANIM-005]
    pub markers: Vec<AnimationMarker>,
    /// Frame range for export, inclusive. None = export all frames.  [§10.5]
    pub export_range: Option<(u32, u32)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaterialAnimationTracks {
    pub pressure:     KeyframeTrack<f32>,   // GpuSimParams.pressure_target
    pub solidify:     KeyframeTrack<f32>,   // GpuSimParams.solidify_blend
    pub shrink_weft:  KeyframeTrack<f32>,   // GpuSimParams.shrink_u
    pub shrink_warp:  KeyframeTrack<f32>,   // GpuSimParams.shrink_v
    /// Per-tack compliance tracks keyed by tack_id (post-MVP; see §10.6.1).
    pub tack_compliance: std::collections::HashMap<String, KeyframeTrack<f32>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WindAnimationTracks {
    pub strength:   KeyframeTrack<f32>,
    pub direction:  KeyframeTrack<glam::Vec3>,  // unit direction; interpolated via slerp
    pub turbulence: KeyframeTrack<f32>,         // 0.0–1.0 scale
    /// Optional positioned wind source (matches MD wind actor).
    pub position:   Option<KeyframeTrack<glam::Vec3>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AvatarBonePoseTrack {
    pub bone_id:      String,
    pub translation:  KeyframeTrack<glam::Vec3>,
    pub rotation:     KeyframeTrack<glam::Quat>,
    pub scale:        Option<KeyframeTrack<glam::Vec3>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BlendShapeTrack {
    pub blend_shape_name: String,
    pub weight:           KeyframeTrack<f32>,  // 0.0 = no blend, 1.0 = full blend
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimationMarker {
    pub marker_id: String,
    pub frame:     u32,
    pub label:     String,
    pub color:     Option<[f32; 3]>,
}
```

**R-ANIM-010** — `total_frames` MUST NOT exceed 1800 for MVP (60 seconds at 30 fps). The
validation gate MUST reject drafts that exceed this cap with a clear error message.

**R-ANIM-011** — `fps` MUST be in the range `[1.0, 120.0]`. The validation gate MUST reject
out-of-range values.

###### 10.2.3 Authority Write Guard

**R-ANIM-012** — Every write to `tailor_garments.animation_json` MUST call
`guard_authority_write(AuthorityMode::PostgresPrimary)` before the `UPDATE` statement. SQLite
writes to this column are forbidden. This is the standard no-SQLite tripwire required by the
Tailor kernel integration contract.

---

##### 10.3 EventLedger Events

**R-ANIM-013** — The following `KernelEventType` variants MUST be added to
`kernel/mod.rs` and MUST be registered in `required_first_slice_events()`. Variant names and
wire strings are canonical per `[T-CONTRACTS.event-types]`:

```rust
// Additions to KernelEventType (kernel/mod.rs):
TailorAnimationDraftCreated,   // wire: "TAILOR_ANIMATION_DRAFT_CREATED"
TailorAnimationDraftUpdated,   // wire: "TAILOR_ANIMATION_DRAFT_UPDATED"
TailorAnimationSimRunRequested,// wire: "TAILOR_ANIMATION_SIM_RUN_REQUESTED"
TailorAnimationSimRunCompleted,// wire: "TAILOR_ANIMATION_SIM_RUN_COMPLETED"
TailorAnimationSimRunRejected, // wire: "TAILOR_ANIMATION_SIM_RUN_REJECTED"
TailorAnimationDraftPromoted,  // wire: "TAILOR_ANIMATION_DRAFT_PROMOTED"
```

The `event_family` constant for all animation events is `"tailor.animation"` per
`[T-CONTRACTS.event-types]` (`TAILOR_ANIMATION` constant).

**R-ANIM-014** — Individual keyframe edits MUST NOT each emit an EventLedger event. The
`TailorAnimationDraftUpdated` event MUST be emitted once when a CRDT snapshot is promoted to
the Postgres `animation_json` column (see §10.4.2 for snapshot cadence).

**R-ANIM-015** — The `TailorAnimationDraftUpdated` payload MUST include at minimum:

```json
{
  "garment_id": "GAR-...",
  "animation_schema": "hsk.tailor.garment_animation_draft@1",
  "content_hash": "<sha256 of canonical JSON of GarmentAnimationDraftV1>",
  "fps": 30.0,
  "total_frames": 150,
  "tracks_summary": {
    "material_track_count": 4,
    "wind_track_count": 3,
    "pose_bone_count": 24,
    "blend_shape_count": 0,
    "marker_count": 0
  },
  "crdt_seq": 42
}
```

---

##### 10.4 CRDT Collaborative Timeline

###### 10.4.1 CRDT Document Mapping

**R-ANIM-016** — The animation draft MUST be stored as a nested sub-map within the existing
CRDT document for the garment (`crdt_document_id = "CRDT-GAR-{garment_id}"`). It MUST NOT have
a separate CRDT document. The animation occupies the `/animation/` path within the document:

```
CRDT document: "CRDT-GAR-{garment_id}"
  /panels/...                    <- panel geometry (existing)
  /seams/...                     <- seam definitions (existing)
  /animation/
    /fps
    /total_frames
    /material_tracks/
      /pressure/keyframes/[...]
      /solidify/keyframes/[...]
      /shrink_weft/keyframes/[...]
      /shrink_warp/keyframes/[...]
    /wind_tracks/
      /strength/keyframes/[...]
      /direction/keyframes/[...]
      /turbulence/keyframes/[...]
    /pose_tracks/{bone_id}/
      /translation/keyframes/[...]
      /rotation/keyframes/[...]
    /blend_shape_tracks/{name}/weight/keyframes/[...]
    /markers/[...]
    /export_range
```

Fine-grained collaborative edits to individual keyframe values MUST go through the `yjs_bridge`
(`push_yjs_update()`) as `YjsUpdateEnvelopeV1` diffs over the `/animation/` sub-tree.

**R-ANIM-017** — Animation CRDT edits for model-proposed changes MUST go through the
`AiEditProposalRequestV1` state machine. A model-proposed keyframe edit MUST NOT be written
directly to the authority `animation_json` column without operator approval.

###### 10.4.2 Conflict Resolution

| Concurrent scenario | CRDT resolution |
|---|---|
| Two users edit different tracks | No conflict; tracks are independent CRDT maps |
| Two users add keyframes at different times on the same track | No conflict; keyframe list is a CRDT ordered list; both inserted |
| Two users edit the same keyframe value simultaneously | Conflict; `TailorCrdtConflictDetected` event emitted; operator resolves |
| Model proposes a keyframe; operator edits same keyframe | `AiEditProposalRequestV1` state machine; operator approves or rejects the model edit first |
| `fps` or `total_frames` changed concurrently | Last-write-wins on the CRDT scalar field; `TailorAnimationDraftUpdated` fires on next snapshot |

**R-ANIM-018** — A CRDT snapshot MUST be promoted to `tailor_garments.animation_json` on each
of the following triggers: (a) operator explicit save-checkpoint action, (b) any
`TailorAnimationSimRunRequested` event (simulation MUST read from the snapshotted authority, not
the live CRDT stream), (c) model `AiEditProposalRequestV1` approval. Between snapshots, the
`kernel_crdt_updates` stream is the running state and `animation_json` is the last promoted
snapshot.

---

##### 10.5 Animation-Driven Simulation Loop

###### 10.5.1 Per-Frame Loop Architecture

**R-ANIM-019** — The `ClothSolver` trait in the `tailor-solver` crate MUST expose the following
two methods in addition to the existing batch `simulate()` method:

```rust
/// Upload updated avatar capsule body proxy positions for the current animation frame.
/// MUST be called once per frame, before simulate_frame(), outside the substep loop.
async fn update_body_proxies(
    &mut self,
    pose: &AvatarPoseSample,
) -> Result<(), ClothSolverError>;

/// Simulate exactly one frame (n_substeps internally). Retains particle state across calls
/// so cloth maintains physical continuity across the animation.
/// Caller MUST call update_body_proxies before calling simulate_frame for each frame.
async fn simulate_frame(
    &mut self,
    params: SimRunParams,
) -> Result<SolverResult, ClothSolverError>;
```

**R-ANIM-020** — The `AnimatedSimRunner` in `handshake_core::tailor::animation` MUST drive the
per-frame loop in the following order for each frame:

1. Sample all keyframe tracks at `t_s = frame_idx as f32 / fps`.
2. Call `solver.update_params(MaterialFrameParams { ... })` with sampled material values.
3. Call `solver.update_wind(wind_vec, turbulence)` with sampled wind values.
4. Call `solver.update_body_proxies(pose)` with the FK-evaluated capsule positions for this frame.
5. Call `solver.simulate_frame(params)` to advance one frame.

**R-ANIM-021** — The capsule proxy update (`update_body_proxies`) MUST be applied before the
substep loop for the current frame, not after. The solver MUST perform one constraint projection
pass against the new capsule positions before beginning substeps. This is required for XPBD
stability with kinematic bodies in motion.

**R-ANIM-022** — The wind turbulence seed MUST be `seed = frame_idx as u64` to make turbulence
deterministic when the simulation is re-run with the same animation data on the same GPU
backend. This is required so the promotion validation re-run can apply `MeshComparator`
equivalence comparison per `[T-CONTRACTS.determinism]`.

###### 10.5.2 Wind Turbulence on GPU (WGSL)

**R-ANIM-023** — The predict shader (`predict.wgsl`) MUST implement inline hash-based
spatial noise for wind turbulence. No texture sampling is permitted. The noise function MUST
use the particle's world-space position as the coordinate input, producing spatially coherent
turbulence across the cloth surface. The following GPU uniform fields are required on
`GpuSimParams` for animated wind:

```wgsl
// Required additions to GpuSimParams UBO for animation:
// params.wind           vec3<f32>  — base wind vector (strength × direction)
// params.wind_turb      f32        — turbulence scale [0, 1]
// params.wind_time_seed f32        — = frame_idx as f32 (deterministic per frame)
```

**R-ANIM-024** — The per-frame `GpuSimParams` UBO upload for animated runs MUST be performed
once per frame before `simulate_frame()`. At 30 fps with a 64-byte `GpuSimParams` struct this
is 1920 bytes/second; this upload cost MUST NOT be cited as a reason to batch or skip
per-frame parameter updates.

###### 10.5.3 Avatar Capsule Proxy Per-Frame Upload

**R-ANIM-025** — `AvatarPoseSample` is the per-frame snapshot of capsule world-space positions
and orientations. The `AnimatedSimRunner` MUST compute it from the `AvatarBonePoseTrack`
keyframe tracks using forward kinematics (FK) and capsule joint offsets before calling
`update_body_proxies`. The upload format MUST match the body collision shader's existing buffer
layout: `(center_a: [f32;3], center_b: [f32;3], radius: f32)` per capsule, in millimetres per
`[T-CONTRACTS.body-proxy]`.

###### 10.5.4 Promotion Equivalence for Animated Runs

**R-ANIM-026** — For animated simulation runs, the promotion gate MUST use
`MeshComparator` tolerance-based comparison per `[T-CONTRACTS.determinism]`, not `content_hash`
equality. Because wind turbulence uses `sin()`/`fract()` whose cross-vendor precision differs,
animated runs MUST use the shape-envelope equivalence basis: per-frame bounding box within
`bbox_epsilon_mm = 1.0` plus `SEAMS_CLOSED`, rather than per-vertex deviation, when the
simulation includes turbulence tracks with non-zero values. The standard per-vertex deviation
check (`epsilon_mm = 0.1`) applies for animated runs with zero turbulence.

---

##### 10.6 Animation Import

###### 10.6.1 Import Architecture

**R-ANIM-027** — Animation import MUST live in `handshake_core::tailor::animation::import`,
NOT in the `tailor-solver` crate. The solver crate MUST NOT take a dependency on `fbxcel-dom`
or `gltf`.

**R-ANIM-028** — The canonical output type of all import paths is `AvatarPoseSequenceV1`:

```rust
// handshake_core/src/tailor/animation/import/types.rs
pub struct AvatarPoseSequenceV1 {
    pub fps: f32,
    pub total_frames: u32,
    pub bone_tracks: Vec<AvatarBonePoseTrack>,
}
```

This type populates the `GarmentAnimationDraftV1.pose_tracks` field directly.

###### 10.6.2 glTF Skeletal Animation (Primary Import Path)

**R-ANIM-029** — glTF skeletal animation MUST be the preferred and primary import format for
avatar animation. The `gltf` crate (v1.4.1 read-only) MUST be used as the parser.

**R-ANIM-030** — The glTF import pipeline (`src/tailor/animation/import/gltf.rs`) MUST:

1. Read `animation.channels` and map each channel to the appropriate `AvatarBonePoseTrack`.
2. Map `gltf::animation::Interpolation::{Linear, Step, CubicSpline}` to the canonical
   `KeyframeInterpolation` enum.
3. For `CubicSpline` channels, de-interleave the glTF `[in_tangent, value, out_tangent]`
   per-keyframe layout into the `Keyframe<T>` struct's `in_tangent` / `value` / `out_tangent`
   fields.
4. Map `Property::MorphTargetWeights` channels to `BlendShapeTrack` entries.
5. Store all input times in seconds; convert to frame indices at sample time using `fps`.

**R-ANIM-031** — The glTF import MUST emit a `TailorAnimationDraftCreated` event on
successful import and MUST persist the resulting `GarmentAnimationDraftV1` to
`tailor_garments.animation_json` via the standard authority write path.

###### 10.6.3 FBX Joint Animation (Secondary Import Path)

**R-ANIM-032** — FBX joint animation MAY be imported using the `fbxcel-dom` crate (v0.0.6,
`fbxcel` v0.7.0). The implementation MUST pin the exact crate version in `Cargo.toml` because
`fbxcel-dom` is marked "highly experimental" and breaks across minor versions.

**R-ANIM-033** — The FBX import path MUST be gated behind a Cargo feature flag
(`feature = "tailor-fbx-import"`, off by default). This prevents the `fbxcel-dom` dependency
from entering the build when FBX import is not needed.

**R-ANIM-034** — If `fbxcel-dom` proves unstable in a given implementation cycle, the operator
recipe MUST document that the supported path is: re-export the FBX animation from the source
DCC (Blender, Maya) as glTF and use the glTF import path (§10.6.2). FBX MUST NOT be required
for any production workflow.

###### 10.6.4 MTN Format

**R-ANIM-035** — Direct import of Marvelous Designer's proprietary `*.mtn` binary format MUST
NOT be implemented. The supported path for MTN sources is re-export from MD or the source DCC
as FBX or glTF. If an operator requests MTN import, the implementation MUST document the
limitation and propose the re-export path. MTN import via an MD SDK license is deferred to a
future work packet requiring explicit operator demand.

---

##### 10.7 Animation-Range Export

**R-ANIM-036** — When `GarmentAnimationDraftV1.export_range` is set, the export pipeline MUST
filter the frame sequence to the inclusive range `[start_frame, end_frame]`. When
`export_range` is `None`, all frames MUST be exported.

**R-ANIM-037** — The export range filter function MUST be a pure function in
`src/tailor/export/animation_range.rs`:

```rust
pub fn apply_export_range(
    frames: &[GarmentFrame],
    range: Option<(u32, u32)>,
) -> &[GarmentFrame] {
    match range {
        None => frames,
        Some((start, end)) => {
            let start = (start as usize).min(frames.len());
            let end   = ((end + 1) as usize).min(frames.len());
            &frames[start..end]
        }
    }
}
```

**R-ANIM-038** — The `TailorGarmentExportCompleted` EventLedger event payload MUST include the
effective exported frame range (start and end frame indices) so downstream consumers have the
range without reading the animation draft.

**R-ANIM-039** — When exporting joint animation in FBX format, the export pipeline MUST bake
explicit keyframes at every frame in the export range (FBX auto-key). This prevents
software-specific interpolation drift when the FBX is consumed by third-party DCC tools. The
baking pass lives in `src/tailor/export/fbx_key_baker.rs` and is a post-MVP deliverable; the
MVP FBX export MAY omit baked keys with a documented limitation.

---

##### 10.8 Model-First Animation API

###### 10.8.1 MCP Tools

**R-ANIM-040** — The Tailor animation system MUST expose the following four MCP tools. Tool
schemas are `schemars`-derivable; the `inputSchema` MUST be auto-generated. Tool names are
canonical and MUST NOT be renamed.

**`tailor_animation_draft_create`** — Create or replace the animation draft for a garment.

```json
{
  "name": "tailor_animation_draft_create",
  "input_schema": {
    "type": "object",
    "required": ["garment_id", "fps", "total_frames"],
    "properties": {
      "garment_id":   { "type": "string" },
      "fps":          { "type": "number", "minimum": 1, "maximum": 120, "default": 30 },
      "total_frames": { "type": "integer", "minimum": 1, "maximum": 1800 },
      "description":  { "type": "string" }
    }
  }
}
```

**`tailor_animation_add_keyframe`** — Add or update a keyframe on a specific track.

```json
{
  "name": "tailor_animation_add_keyframe",
  "input_schema": {
    "type": "object",
    "required": ["garment_id", "track_name", "frame", "value"],
    "properties": {
      "garment_id":    { "type": "string" },
      "track_name":    { "type": "string",
                         "description": "One of: pressure, solidify, shrink_weft, shrink_warp, wind_strength, wind_direction, wind_turbulence, or a bone pose track id." },
      "frame":         { "type": "integer", "minimum": 0 },
      "value":         { "description": "f32 for scalar tracks; [x,y,z] for wind_direction; [x,y,z,w] for rotation." },
      "interpolation": { "type": "string", "enum": ["linear", "step", "cubic_spline"], "default": "linear" }
    }
  }
}
```

**`tailor_animation_simulate`** — Run the cloth simulation with the current animation draft.

```json
{
  "name": "tailor_animation_simulate",
  "input_schema": {
    "type": "object",
    "required": ["garment_id"],
    "properties": {
      "garment_id":  { "type": "string" },
      "n_substeps":  { "type": "integer", "default": 8 },
      "n_iters":     { "type": "integer", "default": 5 },
      "frame_range": { "type": "array", "items": { "type": "integer" }, "minItems": 2, "maxItems": 2 }
    }
  }
}
```

Returns a `simulation_run_id` immediately. The model MUST poll for `TailorAnimationSimRunCompleted`
or `TailorAnimationSimRunRejected`.

**`tailor_animation_export`** — Export the simulated animation in the requested format.

```json
{
  "name": "tailor_animation_export",
  "input_schema": {
    "type": "object",
    "required": ["garment_id", "simulation_run_id", "format"],
    "properties": {
      "garment_id":        { "type": "string" },
      "simulation_run_id": { "type": "string" },
      "format":            { "type": "string", "enum": ["obj_sequence", "gltf_morph", "usd", "alembic_via_blender"] },
      "fps":               { "type": "number", "default": 30 },
      "frame_range":       { "type": "array", "items": { "type": "integer" }, "minItems": 2, "maxItems": 2 }
    }
  }
}
```

**R-ANIM-041** — When a model receives a natural-language animation intent (e.g., "make the
dress ripple in strong wind during the chorus"), the `TailorModelAdapter::invoke()` path MUST
use `LlmClient.completion()` with a structured JSON output schema to generate the keyframe
list. Generated keyframes MUST be validated (values in physical range) before being written to
the CRDT document. The model MUST NOT write directly to `animation_json`; it writes via the
CRDT path, which is then snapshotted per §10.4.2.

###### 10.8.2 Canonical Model Animation Authoring Sequence

The following sequence is the required model workflow for animation authoring and MUST be
documented as the Tailor animation recipe:

1. Call `tailor_animation_draft_create` with `fps` and `total_frames`.
2. Call `tailor_animation_add_keyframe` for each required track and keyframe.
3. Call `tailor_animation_simulate` and await `TailorAnimationSimRunCompleted`.
4. Call `tailor_capture_frame` (render viewport) at representative frame indices to inspect output.
5. Read `SimulationReceipt.validation_findings` per `[T-CONTRACTS.simulation-receipt]`:
   check `SEAMS_CLOSED`, `NO_INTERPENETRATION`, and `DRAPE_CONVERGED` for the animated run.
6. If output passes inspection, call `tailor_animation_export` with the desired format.
7. Confirm `TailorGarmentExportCompleted` receipt in the EventLedger.

---

##### 10.9 Kernel Binding Summary

| Concern | Binding |
|---|---|
| Animation data storage | `tailor_garments.animation_json JSONB` (Postgres authority) |
| Animation draft mutations (coarse) | `TailorAnimationDraftUpdated` EventLedger event |
| Animation draft creation | `TailorAnimationDraftCreated` EventLedger event |
| Collaborative timeline editing | CRDT `yjs_bridge` on `/animation/` sub-tree of `CRDT-GAR-{id}` |
| Model-proposed keyframe edits | `AiEditProposalRequestV1` → operator approval |
| Animation simulation run request | `TailorAnimationSimRunRequested` event |
| Animation simulation completion | `TailorAnimationSimRunCompleted` / `TailorAnimationSimRunRejected` |
| Model-authored animation | `TailorModelAdapter.invoke()` → `LlmClient.completion()` → keyframe list → CRDT write |
| Animation export | Export pipeline with `apply_export_range` filter; `TailorGarmentExportCompleted` event |
| Authority write guard | `guard_authority_write(AuthorityMode::PostgresPrimary)` before every `animation_json` write |
| Per-substep material param upload | `ClothSolver::update_params(MaterialFrameParams)` |
| Per-frame capsule proxy upload | `ClothSolver::update_body_proxies(AvatarPoseSample)` |
| Promotion equivalence (animated) | `MeshComparator` shape-envelope (bbox + `SEAMS_CLOSED`), not hash; per `[T-CONTRACTS.determinism]` |
| Schema ID | `hsk.tailor.garment_animation_draft@1` (`SCHEMA_TAILOR_ANIMATION_DRAFT_V1`) |
| Migration | `YYYY_MM_DD_tailor_garments_animation_col.sql` + `.down.sql` (dated; no `0NNN_` prefix) |
| `event_family` constant | `"tailor.animation"` (`TAILOR_ANIMATION`) |

---

##### 10.10 Deferred Items and Known Constraints

**DEFER-ANIM-001 — Per-tack animated compliance (post-MVP).** `TackComplianceTrack` keyframing
requires writing per-tack `compliance` values to individual entries in the `GpuSeamConstraint`
storage buffer per substep. This is architecturally more complex than the UBO upload used for
pressure/solidify/shrinkage. It is deferred until the constraint GPU buffer layout is finalized.
Until then, `tack_compliance` in `MaterialAnimationTracks` MUST be stored in the draft but
MUST be silently ignored by the solver with an `[INFO]` log entry.

**DEFER-ANIM-002 — FBX auto-key baking (post-MVP).** The FBX key-baking pass
(`fbx_key_baker.rs`) that prevents interpolation drift in third-party DCC tools is post-MVP.
MVP FBX export MUST document that baked keys are not included and that consumers should apply
their own baking step.

**CONSTRAINT-ANIM-001 — Cross-vendor turbulence determinism.** Wind turbulence noise uses
`sin()` and `fract()` in WGSL, whose precision differs across GPU vendors. The determinism
guarantee for animated runs with non-zero turbulence is scoped to same GPU backend and driver.
Cross-vendor promotion uses shape-envelope equivalence per R-ANIM-026 and
`[T-CONTRACTS.determinism]`. This is not a defect; it is an architectural constraint.

**CONSTRAINT-ANIM-002 — MTN format not supported.** The Marvelous Designer `*.mtn` binary
format is proprietary with no public spec and no OSS parser. It is not supported. The required
path for MTN sources is re-export as FBX or glTF per R-ANIM-035.

**CONSTRAINT-ANIM-003 — Interactive real-time animated preview.** Per-frame capsule buffer
uploads add ~0.5–2 ms wall-clock latency per frame on typical hardware. For offline simulation
(the primary use case) this is acceptable. For interactive real-time preview, a reduced proxy
set (4 capsules, fewer substeps) MUST be used. Full-fidelity animated simulation is not
real-time.

---

*Non-normative provenance: design rationale, OSS evidence (keyframe crate evaluation, spanda,
fbxcel-dom limitations, glTF CUBICSPLINE reference implementation), and MD feature mapping are
documented in the research source `15-animation.md`. This sub-section supersedes any contract
surface in that research file; the research file remains valid as non-normative evidence.*

## 13.11 Kernel Integration (Authority, CRDT, Sandbox, Promotion, Model Lanes)

> Sub-section id: `kernel`
> KERNEL_BUILDER will assign the final `<N>.<i>` numbering on assembly.
> Non-normative provenance: research topics `T-KERNEL-INTEGRATION` (10-kernel-integration.md) and
> `T-CONTRACTS` (16-contracts.md) in the cloth_engine_research package. Where this sub-section
> conflicts with those topics, this sub-section is canonical for implementation. Where those topics
> contain algorithm rationale, OSS evidence, or risk analysis not repeated here, they remain valid
> supplementary reading.

---

##### 11-K-1  Module Identity and Placement

**11-K-1.1** The Tailor kernel-binding module MUST be implemented as `handshake_core::tailor`
(`src/tailor/`) following the `src/atelier/` creative-module pattern exactly: a domain-owned
`sqlx::PgPool` reference, a dedicated `event_family.rs` constants block, domain-specific
submodules, and a storage glue file parallel to `storage/kb003_storage.rs`.

**11-K-1.2** The XPBD cloth-physics solver MUST be implemented as a standalone Cargo workspace
crate named `tailor-solver` with no `handshake_core` dependency. `handshake_core::tailor` MUST
depend on the solver only through the `ClothSolver` trait boundary defined in
`src/tailor/solver_binding.rs`.

**11-K-1.3** Physics-internal types (particles, constraints, WGSL kernels) MUST use the `Cloth*`
prefix (`ClothSolver`, `ClothParticle`, `ClothConstraint`). Feature-level identifiers visible
outside the solver crate MUST use the `Tailor*` / `tailor_*` / `TAILOR_*` prefix per the naming
table in [T-CONTRACTS.naming]. The two prefixes MUST NOT be mixed on a single identifier.

**Canonical module layout:**

```
handshake_core/src/tailor/
  mod.rs
  event_family.rs       -- tailor.* dot-namespaced string constants
  garment.rs            -- GarmentSpec, GarmentType, PanelSpec, SeamSpec, ...
  material.rs           -- FabricProperties, FabricPreset, tailor_material_presets rows
  solver_binding.rs     -- ClothSolver trait + ClothSolverRequest/Result bridge
  simulation.rs         -- TailorSandboxAdapter, SimRunParams
  validation.rs         -- TailorValidationDescriptor (wraps KB003 ValidationDescriptor)
  storage_glue.rs       -- Postgres row types + guard_authority_write call sites
  api.rs                -- Axum Router: /tailor/... routes

tailor-solver/ (workspace crate, no handshake_core dep)
  src/
    lib.rs              -- pub trait ClothSolver
    spec.rs             -- GarmentSpec (canonical; serde + schemars)
    mesh.rs             -- SolverMesh, SeamConstraintRecord
    body/proxy.rs       -- ClothBodyProxy, CollisionCapsule, CollisionSphere
    compare.rs          -- MeshComparator
    simulate.rs         -- XPBD engine, WGSL shaders (wgpu)
```

---

##### 11-K-2  PostgreSQL as the Sole Authority Backend

**11-K-2.1** All Tailor domain authority writes MUST target PostgreSQL (`sqlx::PgPool`) and MUST
call `guard_authority_write(AuthorityMode::PostgresPrimary)` as the first statement in every
storage-glue write function. SQLite MUST NOT be used for any Tailor authority write, including
tests that run without a live Postgres instance.

**11-K-2.2** Every Tailor authority mutation MUST emit an EventLedger receipt via
`NewKernelEvent::builder(...)` → `db.insert_kernel_event(write_ctx, event)` before (or in the
same CTE as) the row INSERT/UPDATE, following the idempotent pattern in `storage/postgres.rs`
line ~3454:

```sql
WITH inserted AS (
  INSERT INTO kernel_event_ledger (...)
  ON CONFLICT (idempotency_key) DO NOTHING
  RETURNING event_id
)
INSERT INTO tailor_garments (..., event_ledger_event_id)
SELECT ..., event_id FROM inserted
ON CONFLICT (garment_id) DO NOTHING;
```

**11-K-2.3** Every `NewKernelEvent` MUST supply a `WriteContext` carrying the appropriate
`KernelActor` variant for the audit trail. Permitted actors per pipeline stage:

| Stage | KernelActor |
|---|---|
| Model garment authoring | `ModelAdapter("tailor-garment-adapter-v1")` |
| Operator panel edits (direct) | `Operator(user_id)` |
| Model panel-edit proposals | `ModelAdapter(adapter_id)` |
| XPBD solver sandbox run | `System("cloth-solver-v1")` |
| Validation runner | `ValidationRunner("tailor-garment-validator-v1")` |
| Promotion gate | `PromotionGate("tailor-promotion-gate-v1")` |
| Material preset writes | `Operator(user_id)` or `ModelAdapter(adapter_id)` |
| Avatar / body-proxy writes | `Operator(user_id)` or `System(...)` |

**11-K-2.4** The `WriteContext` actor for model-lane operations MUST be a `ModelAdapter` variant;
it MUST NOT be `Operator` or `System`. This makes model-authored rows distinguishable in the
audit log without additional metadata fields.

---

##### 11-K-3  Canonical Postgres Table Set

All Tailor tables MUST use `TEXT PRIMARY KEY` with the prefixed id form established in migrations
`0332_media_asset_tiers.sql` and `0334_loom_canvas_boards.sql` (FACT-4 from T-CONTRACTS). The
`UUID PRIMARY KEY DEFAULT gen_random_uuid()` form MUST NOT be used for any Tailor table. Every
row MUST carry an `event_ledger_event_id TEXT NOT NULL` FK to `kernel_event_ledger.event_id`.

**Canonical table set and primary-key prefixes:**

```text
TABLE                         PK prefix   Purpose
-----------------------------  ---------  -----------------------------------------------
tailor_garments               GAR-        Authority row per garment; holds spec_json JSONB
tailor_garment_crdt_docs      (composite) Per-garment CRDT document binding
tailor_material_presets       MAT-        Named fabric property presets (physics + normalized)
tailor_avatars                AVT-        Avatar identity (was undefined FK target; authored here)
tailor_body_proxies           BPX-        Solver collision geometry for an avatar
tailor_simulation_runs        SIM-        XPBD sandbox run record
tailor_refit_runs             RFT-        Refit/retargeting run record
tailor_trims                  TRIM-       Rigid trim mesh catalog
tailor_trim_placements        PLAC-       Per-garment trim placement
tailor_zippers                ZIP-        Zipper definitions on garment edges
tailor_lacings                LACE-       Lacing/eyelet sequence definitions
tailor_uv_islands             UVI-        UV-island atlas records per panel per sim run
tailor_pbr_materials          PBR-        PBR material definitions
tailor_graphic_layers         GLYR-       Graphic/decal layer per panel
tailor_material_assignments   ASGN-       Per-panel physics + PBR assignment on a garment
tailor_wardrobe               WRD-        Garment grouping / wardrobe container
```

**11-K-3.1** `tailor_garments` MUST carry the columns: `garment_id TEXT PRIMARY KEY` (`GAR-`),
`workspace_id TEXT NOT NULL`, `name TEXT NOT NULL`, `status TEXT NOT NULL` (CHECK domain:
`draft | sandbox_pending | simulated | validated | promoted | rejected | archived`),
`spec_json JSONB NOT NULL` (the canonical `GarmentSpec`), `animation_json JSONB` (nullable;
owned by the animation module), `body_proxy_id TEXT`, `wardrobe_id TEXT`,
`promotion_receipt_id TEXT`, `event_ledger_event_id TEXT NOT NULL`, `created_at TIMESTAMPTZ`,
`updated_at TIMESTAMPTZ`. The `status` field MUST NOT appear inside `spec_json`; it is
promotion-lifecycle metadata on the row, not garment content.

**11-K-3.2** `tailor_avatars` MUST be authored as follows (this table was undefined across the
research package and is normatively defined here):

```sql
CREATE TABLE IF NOT EXISTS tailor_avatars (
    avatar_id               TEXT PRIMARY KEY,          -- "AVT-{uuid_v7}"
    workspace_id            TEXT NOT NULL,
    name                    TEXT NOT NULL,
    source_kind             TEXT NOT NULL
        CHECK (source_kind IN ('smpl','smplx','metahuman','custom_obj','vrm','gltf',
                               'parametric','avatar1_2d_derived','non_humanoid')),
    measurements_mm_json    JSONB NOT NULL DEFAULT '{}'::jsonb,
    source_mesh_artifact_ref TEXT,
    morph_params_json       JSONB,
    event_ledger_event_id   TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_avatars_workspace ON tailor_avatars (workspace_id);
```

`tailor_body_proxies.avatar_id` MUST FK to `tailor_avatars(avatar_id)`. `tailor_body_proxies`
MUST NOT carry a `garment_id` FK; a garment references a proxy via
`tailor_garments.body_proxy_id`.

**11-K-3.3** `tailor_material_presets` is the single material table. The names
`tailor_material_library` and `tailor_material` MUST NOT be used. System-provided materials are
rows where `is_system_preset = true`; the "library" concept is that row subset, not a separate
table.

**11-K-3.4** `tailor_simulation_runs` sim-run id prefix MUST be `SIM-`. The prefix `CSIM-` MUST
NOT be used.

---

##### 11-K-4  Migration-Naming Convention

**11-K-4.1** All Tailor migrations MUST use the dated convention verified in
`migrations/2026_05_18_fems_pinned.sql` (FACT-3 from T-CONTRACTS):

```
migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.sql
migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.down.sql    (required reverse pair)
```

**11-K-4.2** The numbered `0NNN_tailor_*.sql` form MUST NOT be used; the integer migration space
is a shared sequence that parallel work packets append to, and any fixed number collides. (The
collision of a previously proposed `0334_tailor_garments.sql` with the live
`0334_loom_canvas_boards.sql` is the concrete proof; see T-CONTRACTS FACT-2.)

**11-K-4.3** The date in the migration filename MUST be the authoring date of that migration
file, assigned by the implementing work packet. The research package date (2026-06-17) MUST NOT
be hardcoded into migration filenames; it is the research date, not an implementation date.

**11-K-4.4** Suggested migration set (one file per concern, dated at authoring time):

```
YYYY_MM_DD_tailor_garments.sql
YYYY_MM_DD_tailor_material_presets.sql
YYYY_MM_DD_tailor_avatars.sql
YYYY_MM_DD_tailor_body_proxies.sql
YYYY_MM_DD_tailor_simulation_runs.sql
YYYY_MM_DD_tailor_refit_runs.sql
YYYY_MM_DD_tailor_trims.sql
YYYY_MM_DD_tailor_texture_tables.sql
YYYY_MM_DD_tailor_wardrobe.sql
YYYY_MM_DD_tailor_garments_animation_col.sql   (ALTER TABLE ADD COLUMN animation_json)
```

---

##### 11-K-5  Canonical GarmentSpec

`GarmentSpec` is the canonical garment type. It is:

- The LLM's primary output type (model-lane authoring surface).
- The solver's primary input type (passed into `TailorSandboxAdapter`).
- The Postgres authority JSONB stored in `tailor_garments.spec_json`.

The names `GarmentSpecV1`, `GarmentDraftV1`, and `GarmentDraftV1` MUST NOT be used as the
authority type. `GarmentSpec` is defined in `tailor-solver/src/spec.rs` (standalone crate) with
`#[derive(Serialize, Deserialize, JsonSchema)]` so the MCP `inputSchema` is auto-generated.

**11-K-5.1 Units.** All panel vertex coordinates, seam lengths, dart depths, pleat depths, and
placement translations MUST be in centimetres (cm). Every length field name MUST carry a `_cm`
suffix so the unit is self-documenting. Normalized [0,1] coordinates MUST NOT appear in the
authority `GarmentSpec`; they are permissible only in pre-decode convenience representations
(e.g., the 76-float ChatGarment input vector) that are decoded to cm before storage.

**11-K-5.2 Gather.** The seam gathering field MUST be named `gather_ratio: f32` on `SeamSpec`,
defined as `from_length / to_length`. `1.0` = flat seam; values `> 1.0` = gather the `from`
edge. Valid range is `(0.0, 20.0]` (enforced by `GATHER_RATIO_RANGE` validation check). The
alternative names `ratio` and `gather_ratio_m_n` MUST NOT be used for the stored field.

**11-K-5.3 Fabric values.** `FabricProperties` fields (stretch, shear, bending, friction,
damping, buckling) MUST be normalized `[0.0, 1.0]` in `GarmentSpec` (the LLM-facing surface).
Raw XPBD compliance values (`stretch_weft: 5e-8`, etc.) MUST live only in `tailor_material_presets`
and in the solver crate. The normalized-to-compliance mapping is owned by the preset/decoder
layer and applied at `SolverMesh` build time; it MUST NOT be stored twice.

**11-K-5.4 Panel edges.** Panels MUST use explicit vertex arrays plus typed `EdgeShape` edges
(`Straight | Quadratic { control_cm } | Cubic { control_a_cm, control_b_cm } | Arc {
curvature }`). The string-typed `curve_type: "bezier"` form MUST NOT be used in the authority
type. Polygon-only panels (no `edges` field) MUST NOT be used as the authority form.

**11-K-5.5 Status exclusion.** `GarmentSpec` MUST NOT carry `status`, `created_at`, or
`updated_at` fields. These are row-level promotion-lifecycle metadata on `tailor_garments`, not
garment content, and MUST NOT be model-emittable fields.

The canonical Rust definition lives in `tailor-solver/src/spec.rs`. Any implementation that
deviates from the field names, units, or enum variants in T-CONTRACTS [T-CONTRACTS.garment-spec]
MUST be treated as a contract violation.

---

##### 11-K-6  Schema-ID Constants

**11-K-6.1** The schema-id namespace for all Tailor authority records MUST be `hsk.tailor.*`.
The `hsk.cloth.*` namespace is reserved for solver-crate-internal physics payloads
(`hsk.cloth.solver_request@1`, `hsk.cloth.solver_result@1`) that cross the `ClothSolver` trait
boundary and are never stored as authority rows. Any other use of `hsk.cloth.*` MUST NOT occur.

**Canonical schema-id constants** (in `src/tailor/schemas.rs`):

```rust
pub const SCHEMA_TAILOR_GARMENT_SPEC_V1:    &str = "hsk.tailor.garment_spec@1";
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

// Allowed hsk.cloth.* exception — solver-crate-internal physics payloads only:
pub const SCHEMA_CLOTH_SOLVER_REQUEST_V1:   &str = "hsk.cloth.solver_request@1";
pub const SCHEMA_CLOTH_SOLVER_RESULT_V1:    &str = "hsk.cloth.solver_result@1";
```

---

##### 11-K-7  KernelEventType Additions and event_family Constants

**11-K-7.1** The following variants MUST be added to `KernelEventType` in `kernel/mod.rs`. Every
added variant MUST also be registered in `required_first_slice_events()`. Wire strings follow the
existing `as_str()` SCREAMING_SNAKE_CASE convention (FACT-1 from T-CONTRACTS).

```rust
// -- Garment lifecycle --
TailorGarmentDraftProposed,       // "TAILOR_GARMENT_DRAFT_PROPOSED"
TailorGarmentDraftUpdated,        // "TAILOR_GARMENT_DRAFT_UPDATED"
TailorGarmentValidationRecorded,  // "TAILOR_GARMENT_VALIDATION_RECORDED"
TailorGarmentPromoted,            // "TAILOR_GARMENT_PROMOTED"
TailorGarmentPromotionRejected,   // "TAILOR_GARMENT_PROMOTION_REJECTED"

// -- Simulation run lifecycle --
TailorSimRunRequested,            // "TAILOR_SIM_RUN_REQUESTED"
TailorSimRunStarted,              // "TAILOR_SIM_RUN_STARTED"
TailorSimRunCompleted,            // "TAILOR_SIM_RUN_COMPLETED"
TailorSimRunRejected,             // "TAILOR_SIM_RUN_REJECTED"

// -- CRDT collaborative editing --
TailorPanelCrdtUpdateRecorded,    // "TAILOR_PANEL_CRDT_UPDATE_RECORDED"
TailorPanelCrdtSnapshotRecorded,  // "TAILOR_PANEL_CRDT_SNAPSHOT_RECORDED"
TailorPanelAiEditProposalRecorded,// "TAILOR_PANEL_AI_EDIT_PROPOSAL_RECORDED"
TailorPanelAiEditProposalDecided, // "TAILOR_PANEL_AI_EDIT_PROPOSAL_DECIDED"
TailorCrdtConflictDetected,       // "TAILOR_CRDT_CONFLICT_DETECTED"

// -- Material / fabric presets --
TailorMaterialPresetRecorded,     // "TAILOR_MATERIAL_PRESET_RECORDED"
TailorMaterialPresetUpdated,      // "TAILOR_MATERIAL_PRESET_UPDATED"
TailorMaterialPresetRejected,     // "TAILOR_MATERIAL_PRESET_REJECTED"
TailorGarmentMaterialAssigned,    // "TAILOR_GARMENT_MATERIAL_ASSIGNED"

// -- Avatar / body proxy --
TailorAvatarCreated,              // "TAILOR_AVATAR_CREATED"
TailorAvatarMeasurementsExtracted,// "TAILOR_AVATAR_MEASUREMENTS_EXTRACTED"
TailorBodyProxyCreated,           // "TAILOR_BODY_PROXY_CREATED"
TailorBodyProxyUpdated,           // "TAILOR_BODY_PROXY_UPDATED"

// -- Refit / retargeting --
TailorRefitRequested,             // "TAILOR_REFIT_REQUESTED"
TailorRefitPatternScaled,         // "TAILOR_REFIT_PATTERN_SCALED"
TailorRefitDrapeCompleted,        // "TAILOR_REFIT_DRAPE_COMPLETED"
TailorRefitUvRecomputed,          // "TAILOR_REFIT_UV_RECOMPUTED"
TailorRefitPromoted,              // "TAILOR_REFIT_PROMOTED"
TailorRefitRejected,              // "TAILOR_REFIT_REJECTED"

// -- Trims, zippers, lacing --
TailorTrimImported,               // "TAILOR_TRIM_IMPORTED"
TailorTrimPlaced,                 // "TAILOR_TRIM_PLACED"
TailorTrimTackUpdated,            // "TAILOR_TRIM_TACK_UPDATED"
TailorZipperDefined,              // "TAILOR_ZIPPER_DEFINED"
TailorLacingDefined,              // "TAILOR_LACING_DEFINED"
TailorPatternToTrimConverted,     // "TAILOR_PATTERN_TO_TRIM_CONVERTED"
TailorTrimContactViolation,       // "TAILOR_TRIM_CONTACT_VIOLATION"

// -- UV / texture --
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

// -- Animation timeline --
TailorAnimationDraftCreated,      // "TAILOR_ANIMATION_DRAFT_CREATED"
TailorAnimationDraftUpdated,      // "TAILOR_ANIMATION_DRAFT_UPDATED"
TailorAnimationSimRunRequested,   // "TAILOR_ANIMATION_SIM_RUN_REQUESTED"
TailorAnimationSimRunCompleted,   // "TAILOR_ANIMATION_SIM_RUN_COMPLETED"
TailorAnimationSimRunRejected,    // "TAILOR_ANIMATION_SIM_RUN_REJECTED"
TailorAnimationDraftPromoted,     // "TAILOR_ANIMATION_DRAFT_PROMOTED"

// -- Export --
TailorGarmentExportCompleted,     // "TAILOR_GARMENT_EXPORT_COMPLETED"

// -- Wardrobe --
TailorWardrobeCreated,            // "TAILOR_WARDROBE_CREATED"
TailorWardrobeGarmentAdded,       // "TAILOR_WARDROBE_GARMENT_ADDED"
TailorWardrobeGarmentRemoved,     // "TAILOR_WARDROBE_GARMENT_REMOVED"
```

**11-K-7.2** The following superseded variant names MUST NOT be introduced in new code. Any
existing reference MUST be migrated to the canonical name shown:

| Superseded (do not use) | Canonical replacement |
|---|---|
| `TailorGarmentValidated` (from 03/04) | `TailorGarmentValidationRecorded` |
| `TailorPatternValidated` (from 01) | `TailorGarmentValidationRecorded` |
| `TailorCrdtUpdateRecorded` (from 09) | `TailorPanelCrdtUpdateRecorded` |
| `TailorGarmentCrdtUpdateRecorded` (from 03/04) | `TailorPanelCrdtUpdateRecorded` |
| `BodyProxyCreated` (missing prefix, from 07) | `TailorBodyProxyCreated` |
| `BodyProxyMeasurementsExtracted` (from 07) | `TailorAvatarMeasurementsExtracted` |
| `TailorMaterialLibraryUpdated` (from 03/01) | `TailorMaterialPresetRecorded` or `TailorMaterialPresetUpdated` |
| `TailorDraftScaled` (from 07) | `TailorRefitPatternScaled` |
| Wire string `GARMENT_PROMOTED` (from 09; missing prefix) | `TAILOR_GARMENT_PROMOTED` |

**11-K-7.3** `event_family` constants MUST follow the `tailor.<domain>.<verb>` dot-namespaced
lowercase form (matching the `atelier.<domain>.<verb>` convention; FACT-6). They MUST be defined
in `src/tailor/event_family.rs`:

```rust
pub const TAILOR_GARMENT:    &str = "tailor.garment";
pub const TAILOR_SIMULATION: &str = "tailor.simulation";
pub const TAILOR_PANEL_CRDT: &str = "tailor.panel.crdt";
pub const TAILOR_MATERIAL:   &str = "tailor.material";
pub const TAILOR_AVATAR:     &str = "tailor.avatar";
pub const TAILOR_BODY_PROXY: &str = "tailor.body_proxy";
pub const TAILOR_REFIT:      &str = "tailor.refit";
pub const TAILOR_TRIM:       &str = "tailor.trim";
pub const TAILOR_UV:         &str = "tailor.uv";
pub const TAILOR_TEXTURE:    &str = "tailor.texture";
pub const TAILOR_ANIMATION:  &str = "tailor.animation";
pub const TAILOR_WARDROBE:   &str = "tailor.wardrobe";
pub const TAILOR_EXPORT:     &str = "tailor.export";
```

---

##### 11-K-8  Sandbox Integration: TailorSandboxAdapter

**11-K-8.1** The XPBD solver MUST run inside the KB003 sandbox lifecycle
(`REQUESTED → STARTED → COMPLETED | REJECTED`). `TailorSandboxAdapter` MUST implement the
`SandboxAdapter` trait (`kernel/sandbox/adapter.rs`) and MUST use `AdapterIsolationTier::Process`
as the day-one isolation tier.

**11-K-8.2** The sandbox policy for the cloth solver MUST grant `SandboxCapability::LocalFilesystemRead`
and `SandboxCapability::LocalFilesystemWrite` scoped to the sandbox workspace scratch path. It
MUST NOT grant network access.

**11-K-8.3** `TailorSandboxAdapter::run()` MUST perform, in order: (1) `pre_check` against the
policy, (2) load `GarmentSpec` from the workspace artifact reference, (3) build `SolverMesh`
from the spec (pattern triangulation), (4) call `ClothSolver::simulate(ClothSolverRequest)`,
(5) write the vertex/UV/index artifact bundle to the sandbox workspace scratch path, (6) return
`AdapterRunOutcome::Completed { artifact_refs }`.

**11-K-8.4** Simulated mesh artifacts MUST use a `ClothSimulatedMeshBundle` artifact class entry
(extending `Kb003ArtifactClass`) with `content_type: "application/octet-stream"` and
`hash_policy: BinarySha256`. The `retention_root` MUST be a path resolved from `AppState.artifact_root`,
not a hardcoded absolute path.

**11-K-8.5** The `ClothSolver` trait MUST expose a `cpu_fallback` capability flag so that
headless CI and container environments where wgpu cannot initialize a GPU backend can fall back to
a CPU solver path without failing the `TailorSandboxAdapter::run()` call. The artifact manifest
MUST record which backend (GPU or CPU) was used.

---

##### 11-K-9  Validation Gate: TailorValidationDescriptor

**11-K-9.1** `TailorValidationDescriptor` MUST wrap the KB003 `ValidationDescriptor` type
(`kernel/validation/descriptor.rs`). It MUST select the applicable check subset by stage
(fast-pre-solver, mesh-quality, post-simulation, trim, multi-layer, refit, material-preset) and
by which optional feature set the garment uses.

**11-K-9.2** The canonical check catalog comprises exactly the checks listed below. Each check
has a stable `code` (for model self-correction), a `severity` of `Blocking` or `Advisory`, and a
`stage`. The catalog as a whole supersedes the scattered per-file check lists in research topics
03, 05, 06, 07, 10, and 13.

```text
CODE                   Sev       Stage     Assertion
---------------------  --------  --------  ------------------------------------------------
PANEL_CLOSURE          Blocking  fast      Each panel polygon is a closed, non-self-intersecting loop
SEAM_EDGE_REF          Blocking  fast      Every SeamSpec.from/to references a valid panel_id+edge_index
GATHER_RATIO_RANGE     Blocking  fast      Every SeamSpec.gather_ratio in (0.0, 20.0]
FABRIC_RANGE           Blocking  fast      Normalized FabricProperties in [0,1]; density_g_m2 in [5,2000]; collision_thickness_mm in [0.1,5]
AVATAR_BINDING         Blocking  fast      AvatarBinding.avatar_id exists in tailor_avatars
MIN_PANEL_AREA         Blocking  fast      Every panel area > 1.0 cm^2
WINDING                Advisory  fast      Panel vertices counter-clockwise (auto-corrected; info if fixed)
MESH_TOPOLOGY          Blocking  mesh      Manifold; no degenerate triangles; no unintended open boundary
MESH_TRIANGLE_QUALITY  Blocking  mesh      Min triangle angle >= 10 deg; max aspect ratio <= 20
PANEL_OVERLAP          Advisory  mesh      No two panels occupy the same 3D region before draping
MESH_NOT_EMPTY         Blocking  post      Simulated vertex buffer non-empty
NO_DEGENERATE_TRIS     Blocking  post      No zero-area triangles in output mesh
SEAMS_CLOSED           Blocking  post      Every seam constraint pair <= 1 mm separation at rest
NO_INTERPENETRATION    Blocking  post      No cloth particle deeper than -0.5 mm inside any body capsule/sphere (final frame)
SELF_INTERSECTION      Advisory  post      Self-collision pair count below mesh-explosion limit
UV_COVERAGE            Blocking  post      UV islands cover >= 95% of mesh surface
UV_VALIDITY            Blocking  post      All UVs in [0,1]^2; no degenerate UV triangles (area > 1e-6)
DRAPE_CONVERGED        Advisory  post      Final kinetic energy below threshold (solver converged)
PANEL_COUNT_MATCH      Advisory  post      Simulated panel count == spec panel count
GARMENTCODE_ROUNDTRIP  Advisory  post      Spec round-trips to GarmentCode JSON without loss
INTERLAYER_SPACING     Blocking  post      No inter-layer pair closer than (t_inner+t_outer-tolerance) [layered garments only]
TRIM_NO_PENETRATION    Blocking  post      No trim mesh triangle interpenetrates a cloth triangle [trims only]
TACK_SEAM_CLOSURE      Blocking  post      All tack distances <= 5 mm at end of draping [trims only]
ZIPPER_TOOTH_ALIGN     Blocking  post      Tooth-rail tacks within 1 mm of panel edge positions [zippers only]
LACING_CORD_LENGTH     Blocking  post      No cord segment stretched beyond 200% rest length [lacings only]
TRIM_GRAVITY_STABLE    Advisory  post      No trim body translating > 50 mm/frame in final 10 frames [trims only]
TACK_STRENGTH_NONZERO  Advisory  post      Warn if any tack strength < 0.01 [trims only]
PRESET_NO_NAN          Blocking  preset    No NaN/Inf in drape-test particle positions [material presets only]
PRESET_STRETCH_NONZERO Blocking  preset    Stretch compliance != 0 [material presets only]
PRESET_DENSITY_POS     Blocking  preset    Density > 0 [material presets only]
PRESET_BBOX_PLAUSIBLE  Advisory  preset    Drape-test bounding box within expected range [material presets only]
REFIT_INTERSECTION_FREE Blocking post     Min(particle-capsule distance) >= -0.5 mm after refit [refit only]
REFIT_SEAM_CLOSURE     Blocking  post      Mated seam edge-pair length diff < 1% [refit only]
REFIT_CONVERGED        Advisory  post      Refit sim reached equilibrium (did not time out) [refit only]
```

**11-K-9.3** `ValidationReport::aggregate_blocks_promotion()` (existing kernel method) MUST drive
the promotion gate decision. Any `Blocking` check failure MUST prevent promotion. Advisory check
failures MUST be recorded and surfaced but MUST NOT block promotion unless the
`PromotionGateInputs.treat_advisory_as_blocking` flag is `true`.

**11-K-9.4** Each `ValidationFinding` returned to a model MUST carry: `code` (stable string from
the catalog above), `severity` (`"blocking" | "advisory" | "info"`), optional `affected_id`
(panel_id / seam_id / trim_id), and optional `suggested_fix { field_path: JsonPointer,
suggested_value: serde_json::Value }`. This is the model self-correction contract. The
`recommended_action` field MUST be one of: `promote_garment | edit_and_resimulate |
correct_spec_first | requires_operator_action`.

---

##### 11-K-10  Promotion Gate Binding

**11-K-10.1** `PromotionGate::evaluate()` (`kernel/kb003_promotion/gate.rs`) MUST be called
after the sandbox run completes and the `TailorValidationDescriptor` issues a passing report.
The `PromotionGateInputs` bundle MUST supply: `sandbox_run`, `validation_report`,
`validation_run_id`, `artifact_bundle`, `operator_approval` (`OperatorApprovalEvidence`),
`idempotency_key` in the form `CPROM-{garment_id}-{val_run_id}`, and
`required_artifact_refs: vec![mesh_ref, uv_ref, material_ref]`.

**11-K-10.2** On `PromotionOutcome::Accepted`: (1) `tailor_garments.status` MUST be set to
`promoted`, (2) `promoted_at_utc` and `promotion_receipt_id` MUST be written, (3) a
`TailorGarmentPromoted` EventLedger event MUST be emitted with the `receipt_id` in the payload.
The garment row is then an authority row readable by all sessions.

**11-K-10.3** On `PromotionOutcome::Rejected`: (1) `tailor_garments.status` MUST remain at its
pre-promotion value, (2) a `TailorGarmentPromotionRejected` event MUST be emitted with the
typed `PromotionRejectionReason` in the payload, (3) the rejection detail MUST be surfaced to
the operator.

**11-K-10.4** The idempotency key `CPROM-{garment_id}-{val_run_id}` MUST be used without
modification. Retrying a promotion for the same garment and validation run MUST return the
original `PromotionReceiptV1` rather than creating a duplicate row.

**11-K-10.5** Automated self-approval is architecturally blocked. `OperatorApprovalEvidence`
MUST come from a real operator review receipt. The `looks_fixture()` guard in the promotion gate
MUST NOT be bypassed by the model lane.

---

##### 11-K-11  Determinism and Promotion Equivalence

**11-K-11.1** Solver determinism is per-backend: the same GPU backend plus driver version
produces identical float results; cross-backend or cross-driver-version runs produce results that
differ in float rounding due to WGSL/Naga sub-expression ordering (confirmed by wgpu issue
#5329). Implementation MUST NOT assume cross-backend bit-identical results.

**11-K-11.2** `SolverResult.content_hash` (SHA-256 of the final position buffer) MUST be used
only for same-machine, same-run idempotency and as the EventLedger receipt fingerprint. It MUST
NOT gate promotion equivalence. The `PromotionGate` validation runner MUST NOT compare
`content_hash` values across runs as a promotion criterion.

**11-K-11.3** Promotion equivalence (when re-running a sim to confirm reproducibility) MUST use
`MeshComparator::compare(a, b, epsilon_mm)` defined in `tailor-solver/src/compare.rs`. The
verdict is "equivalent for promotion" if and only if all of the following hold:

- `vertex_count`, `triangle_count`, `seam_edge_pair_count`, and `panel_count` match exactly.
- Max per-vertex Euclidean position deviation `<= epsilon_mm`.

The default `epsilon_mm` MUST be `0.1`. It MUST be a configurable field on `SimRunParams` so the
operator can tighten or loosen it per garment.

**11-K-11.4** For animated runs that include stochastic wind turbulence, per-vertex position
deviation across vendors may exceed `epsilon_mm`. For animated runs, `MeshComparator` MUST
additionally accept a shape-envelope match: per-frame bounding box within `bbox_epsilon_mm`
(default `1.0`) plus `SEAMS_CLOSED` as the equivalence basis, because turbulence is aesthetic
and exact per-vertex reproduction is not achievable cross-vendor.

---

##### 11-K-12  CRDT Collaborative Pattern Editing

**11-K-12.1** Collaborative garment-panel editing MUST reuse the existing `kernel/crdt/`
infrastructure (`kernel_crdt_updates` table, migration 0020) without modification. The Tailor
module MUST NOT introduce a separate CRDT table or a separate CRDT document model.

**11-K-12.2** The CRDT document mapping MUST be:
- `document_id` = `garment_id` (the `GAR-` prefixed id).
- `crdt_document_id` = `"CRDT-GAR-{garment_id}"` stored in `tailor_garment_crdt_docs`.
- Panel geometry, seam definitions, and per-panel material overrides are CRDT map subtrees within
  the single garment document.

**11-K-12.3** Frontend collaborative editing MUST use the `yjs_bridge` path
(`kernel/crdt/yjs_bridge.rs`): `push_yjs_update()` validates the `YjsUpdateEnvelopeV1`,
enforces linear draft ordering, appends the EventLedger receipt
(`TailorPanelCrdtUpdateRecorded`), and writes the update row. The update bytes are opaque to the
backend; Yjs merging happens in the Tauri frontend.

**11-K-12.4** Server-side panel geometry mutations proposed by a model or by post-simulation
UV feedback MUST use the `ai_edit_proposal` path (`kernel/crdt/ai_edit_proposal.rs`) and MUST
emit `TailorPanelAiEditProposalRecorded`. Models MUST NOT self-approve their own proposals;
operator or validation-runner approval is required before a model diff is applied as a CRDT
update.

**11-K-12.5** Concurrent edits to distinct panels MUST be resolved by last-write-wins on the
distinct panel subtrees (panels are independent CRDT map subtrees). Concurrent edits to the same
panel vertex MUST surface a `TailorCrdtConflictDetected` event and require operator decision via
the existing conflict-resolution path in the kernel CRDT module.

---

##### 11-K-13  Model Lanes: LLM-Steerable Garment Authoring

**11-K-13.1** All LLM calls in the Tailor module MUST route through `LlmClient::completion()`
(`llm/mod.rs`). No Tailor-specific LLM client implementation MUST be introduced; the module
MUST use `Arc<dyn LlmClient>` injected from `AppState.llm_client`.

**11-K-13.2** `TailorModelAdapter` MUST implement the `ModelAdapter` trait
(`kernel/model_adapter.rs`). Its `invoke()` method MUST: (1) call `LlmClient::completion()` with
a `CompletionRequest` that includes a `json_schema` field set to the `GarmentSpec` JSON schema
(derived via `schemars`), (2) parse and validate the response against `GarmentSpec`, (3) emit
`TailorGarmentDraftProposed` with the validated spec as payload, (4) return a
`ModelAdapterOutput` with `artifact_kind: "tailor_garment_draft"`.

**11-K-13.3** The `json_schema` field in `CompletionRequest` MUST be set for all garment
authoring calls to request constrained structured output. This MUST NOT be optional or omitted
in production model lanes.

**11-K-13.4** A second `TailorModelAdapter` variant MUST support material parameter estimation
from a fabric-swatch image (following the Image2Garment approach): (1) the operator uploads a
swatch image, (2) `LlmClient::completion()` is called with the image and the `FabricProperties`
JSON schema as constrained output format, (3) the response is validated against
`FABRIC_RANGE`, `PRESET_STRETCH_NONZERO`, and `PRESET_DENSITY_POS` checks, (4) on pass, the
material is promoted to `tailor_material_presets` via the standard authority write path.

**11-K-13.5** The recommended `CompletionRequest` temperature for garment authoring MUST be
`<= 0.2` to minimize hallucinated geometry. Higher temperatures are permitted only for
exploratory design ideation where subsequent validation will catch invalid outputs.

---

##### 11-K-14  Wardrobe Grouping

**11-K-14.1** Wardrobe grouping MUST be implemented via the `tailor_wardrobe` table (PK
`WRD-{uuid_v7}`) with `workspace_id` and `name`. Garments reference their wardrobe via
`tailor_garments.wardrobe_id`.

**11-K-14.2** Wardrobe mutations MUST emit `TailorWardrobeCreated`, `TailorWardrobeGarmentAdded`,
or `TailorWardrobeGarmentRemoved` EventLedger events as appropriate.

**11-K-14.3** Wardrobes are query-time groupings, not promotion gates. A garment MUST NOT be
blocked from promotion because it lacks a `wardrobe_id`. Wardrobe assignment is independent of
the `sandbox → validation → promotion` lifecycle.

---

##### 11-K-15  Axum API Routes

Tailor MUST expose the following Axum routes in `src/api/tailor.rs` (or `src/tailor/api.rs`),
wired from the main router following the existing `src/api/<domain>.rs` pattern:

```rust
POST   /tailor/garments                  -- create_garment_draft
GET    /tailor/garments/:id              -- get_garment
POST   /tailor/garments/:id/simulate     -- trigger_simulation
POST   /tailor/garments/:id/promote      -- promote_garment
GET    /tailor/garments/:id/crdt         -- get_crdt_state
POST   /tailor/garments/:id/crdt/push    -- push_crdt_update
GET    /tailor/materials                 -- list_materials
POST   /tailor/materials                 -- create_material
GET    /tailor/materials/:id             -- get_material
GET    /tailor/wardrobes                 -- list_wardrobes
POST   /tailor/wardrobes                 -- create_wardrobe
POST   /tailor/wardrobes/:id/garments    -- add_garment_to_wardrobe
```

Tauri commands in `app/src-tauri/src/commands/` MUST wrap these endpoints for frontend use.
Route and command naming MUST use the `tailor_` prefix.

---

##### 11-K-16  Full Garment Lifecycle (Normative Sequence)

The normative lifecycle for a model-authored garment is:

```
1. Operator describes garment intent
       |
2. TailorModelAdapter.invoke()
     -> LlmClient.completion() [constrained by GarmentSpec JSON schema]
     -> GarmentSpec parsed and fast-pre-solver checks run (PANEL_CLOSURE, SEAM_EDGE_REF,
        GATHER_RATIO_RANGE, FABRIC_RANGE, AVATAR_BINDING, MIN_PANEL_AREA)
     -> INSERT tailor_garments (status = 'draft')
     -> TailorGarmentDraftProposed EventLedger event emitted
       |
3. [Optional] CRDT collaborative editing
     -> push_yjs_update() or ai_edit_proposal flow
     -> TailorPanelCrdtUpdateRecorded / TailorPanelAiEditProposalRecorded events
       |
4. POST /tailor/garments/:id/simulate
     -> SandboxRunV1 created (status = REQUESTED)
     -> TailorSimRunRequested event emitted
     -> TailorSandboxAdapter.run() (status = STARTED)
          -> SolverMesh built from GarmentSpec
          -> ClothSolverRequest dispatched to tailor-solver crate
          -> XPBD solver: WGSL compute on wgpu (Vulkan/DX12/Metal or CPU fallback)
          -> Mesh + UV artifact bundle written to sandbox workspace scratch path
     -> SandboxRunStatus = Completed
     -> TailorSimRunCompleted event; tailor_simulation_runs row updated
       |
5. TailorValidationDescriptor executes applicable check subset
     -> Fast, mesh, post-simulation checks evaluated
     -> ValidationReport produced
     -> TailorGarmentValidationRecorded event emitted
       |
   [Any Blocking check failed?]
   YES -> TailorSimRunRejected event; operator sees blocking codes + suggested_fix; loop to step 3/4
   NO  ->
       |
6. PromotionGate.evaluate(PromotionGateInputs)
     -> Operator provides OperatorApprovalEvidence (real review receipt; no self-approval)
     -> [Rejected?] TailorGarmentPromotionRejected event; rejection reason surfaced; loop
     -> [Accepted?]
          tailor_garments.status = 'promoted'
          promoted_at_utc + promotion_receipt_id written
          TailorGarmentPromoted EventLedger event emitted
          Garment is an authority row readable by all sessions
```

**11-K-16.1** A passing API call, a non-crashing solver run, or a single visually acceptable
sample MUST NOT be claimed as a complete lifecycle. A garment reaches authority status only at
step 6 (`TailorGarmentPromoted`).

**11-K-16.2** The lifecycle above MUST be traceable end-to-end via EventLedger queries on the
`tailor.garment`, `tailor.simulation`, and `tailor.garment.promotion` event families without
reading application logs.

---

##### 11-K-17  Portability

**11-K-17.1** All Tailor module code MUST NOT contain hardcoded absolute paths. Paths MUST be
resolved from `AppState.artifact_root` or the equivalent topology-level configuration entry.

**11-K-17.2** Garment authority rows live in PostgreSQL. The connection string MUST be supplied
via environment variable, not hardcoded.

**11-K-17.3** Simulated mesh artifact bundles MUST be written to
`{artifact_root}/handshake-product/cloth/sim-meshes/` using the `Kb003ArtifactMetadata.retention_root`
path convention, so the root is relocatable by changing `artifact_root` alone.

**11-K-17.4** The `tailor-solver` crate MUST NOT hardcode any paths. All file I/O MUST go
through `ClothSolverRequest` / `ClothSolverResult` string refs supplied by the sandbox workspace
materializer.

---

##### 11-K-18  No-SQLite Tripwire

**11-K-18.1** `guard_authority_write(AuthorityMode::PostgresPrimary)` MUST be called as the
first statement in every function in `storage_glue.rs` that writes to any `tailor_*` table. This
is a hard runtime guard; violation causes an immediate error, not a warning.

**11-K-18.2** Test environments that cannot connect to PostgreSQL MUST mock the `Database` trait,
not route authority writes through SQLite. Using SQLite as a test backend for Tailor authority
tables is a policy violation, not a convenience shortcut.

---

*Provenance (non-normative):* This sub-section derives its contract surfaces from T-CONTRACTS
(16-contracts.md) and T-KERNEL-INTEGRATION (10-kernel-integration.md) in the
`wt-gov-kernel/.GOV/reference/cloth_engine_research/` package, verified against live codebase
`wtc-kernel-009` on 2026-06-17. Where any research topic conflicts with this sub-section, this
sub-section is authoritative for implementation.

## 13.12 Viewport, Visual Debug & Render/Export Handoff

<!-- id: render -->
<!-- Non-normative provenance: cloth_engine_research/08-render-viewport-export.md (T-RENDER-VIEWPORT) -->
<!-- Canonical contract authority: cloth_engine_research/16-contracts.md (T-CONTRACTS) -->

---

### 12.1 Scope

This section governs:

1. The throwaway Bevy testbed viewport used during `tailor-solver` crate development.
2. The Handshake-native wgpu viewport panel embedded in the Tauri 2 shell.
3. Model-readable visual capture and structured simulation-state metadata.
4. Geometry-cache export (glTF morph-target, OBJ sequence, USD time-sample) to downstream DCC tools (Blender, Unreal Engine 5).

Tailor is NOT a photoreal renderer. Its viewport is a simulation-quality and debugging surface only. Final photoreal rendering MUST happen in an external DCC tool consuming a Tailor geometry-cache export.

All type names, event-variant names, schema IDs, table names, PK forms, and migration-naming conventions used in this section are canonical per T-CONTRACTS (16-contracts.md) and MUST NOT be altered to match superseded names in earlier research files.

---

### 12.2 Throwaway Bevy Testbed Viewport

#### 12.2.1 Purpose and Isolation

The throwaway testbed is a developer utility crate named `handshake-cloth-testbed`. It exists solely to provide a windowed viewport for visually inspecting `tailor-solver` output during crate development.

- The testbed MUST NOT be a dependency of `handshake_core`, `app/src-tauri`, or any crate that ships to end users.
- The testbed MUST NOT import `sqlx`, `axum`, `tauri`, or any `handshake_core` type.
- The testbed MUST be compilable with a single `cargo run -p handshake-cloth-testbed` without a running PostgreSQL instance or Tauri shell.
- The testbed SHOULD be treated as a deprecation candidate once the Handshake-native viewport (12.3) is operational.

#### 12.2.2 Solver Boundary

The testbed MUST interact with the solver exclusively through the `ClothSolver` trait (T-CLOTH-SOLVER) and the `GarmentFrame` output type defined in `tailor-solver`. These are the only public types it is permitted to import from that crate.

#### 12.2.3 Bevy 0.18 Integration Pattern

The testbed MUST use Bevy 0.18's `RenderTarget::Image` + `ImageCopyDriver` headless path for CI-style visual regression captures. It MUST NOT open a windowed OS surface for headless CI runs.

The ECS wiring pattern follows bevy_silk (ManevilleF/bevy_silk) for component layout: a cloth entity with `Handle<Mesh>` + `ClothComponent` + `Transform` + `GlobalTransform`. However, the solver invoked MUST be `tailor-solver`'s XPBD GPU path, not bevy_silk's Verlet integrator. The testbed updates Bevy's `Mesh` vertex buffer each frame by feeding `GarmentFrame.positions` back through `extract_meshes`.

Headless PNG capture MUST use the staging-buffer readback pattern (`copy_texture_to_buffer` + `BufferUsages::MAP_READ` + `device.poll(PollType::Wait)`) as documented in `bevy/examples/app/headless_renderer.rs`. The captured PNG file MUST be written by the `image` crate (`image::RgbaImage::from_raw`).

#### 12.2.4 Testbed Crate Layout

```
handshake-cloth-testbed/
  Cargo.toml          # dev-only; pins Bevy to a specific version; NO handshake_core deps
  src/
    main.rs           # Bevy App builder and plugin registration
    scene.rs          # avatar proxy capsules, ground plane, lighting
    cloth_plugin.rs   # ECS plugin: spawn cloth entity, drive tailor-solver each Update tick
    debug_overlay.rs  # egui-wgpu panels: particle count, constraint residuals, step timing
    capture.rs        # headless PNG capture (RenderTarget::Image + ImageCopyDriver)
    export.rs         # per-frame OBJ dump for visual diff tooling
  examples/
    drape_sphere.rs
    xpbd_seam_test.rs
```

The `Cargo.toml` MUST include an explicit Bevy version pin and an `allow-dirty` marker comment so intentional version skew is visible. Bevy minor upgrades MUST be treated as an explicit upgrade action on this crate, not an automatic dependency pull.

#### 12.2.5 Debug Overlay

The testbed SHOULD surface a live egui-wgpu debug panel using `egui-wgpu::CallbackTrait`. The panel MUST display at minimum: particle count, constraint count, max constraint residual, kinetic energy, and step time in ms. A residual-history line plot SHOULD be included to make convergence visible without pixel analysis.

---

### 12.3 Handshake-Native Tailor Viewport

#### 12.3.1 Architecture: Rendered-to-Texture

The Handshake-native Tailor viewport MUST use a rendered-to-texture architecture. It MUST NOT attempt to embed a raw wgpu OS surface inside the Tauri 2 WebView window (the surface-competition failure mode documented in Tauri issue #9220 and the Graphite project blockers on Wayland and Windows compositor).

The required rendering path is:

1. The viewport renders into an offscreen `wgpu::Texture` (no OS window handle required).
2. On demand or at a configurable reduced frame rate (MUST default to no more than 15 fps for live preview), the rendered texture is transferred CPU-side via `copy_texture_to_buffer`.
3. The PNG bytes are delivered to the Tauri frontend via a Tauri `invoke` response or `emit` event.
4. The frontend renders the bytes into an `<img>` or `<canvas>` element inside the Tailor control-room panel.

This architecture requires no OS compositor patches and is safe to upgrade later to a native wgpu surface if `tauri-plugin-steam-overlay` reaches full cross-platform support (currently macOS-only as of June 2026 — treat as a future upgrade path only).

#### 12.3.2 ClothViewport Struct Contract

The viewport implementation MUST expose the following public interface from the `tailor-solver` crate (or a thin `handshake-tailor-viewport` sub-crate with no `handshake_core` dependencies):

```rust
// tailor-solver/src/viewport.rs
pub struct ClothViewport {
    device: wgpu::Device,
    queue: wgpu::Queue,
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    staging_buffer: wgpu::Buffer,
    solid_pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    debug_pipeline: wgpu::RenderPipeline,
    config: ViewportConfig,
}

pub struct ViewportConfig {
    pub width: u32,
    pub height: u32,
    pub render_mode: RenderMode,
    pub show_wireframe_overlay: bool,
    pub show_constraint_residuals: bool,
    pub show_normals: bool,
    pub show_particles: bool,
    pub background_color: [f32; 4],
    pub camera: CameraUniform,
}

pub enum RenderMode { Solid, Wireframe, Toon }
```

`ClothViewport` MUST NOT have a dependency on `handshake_core`. It is instantiated from `app/src-tauri` and driven via Tauri commands.

#### 12.3.3 Render Pipelines

The viewport MUST implement three wgpu render pipelines:

- **Solid pipeline**: PBR-lite fragment shader over the draped cloth mesh. Cloth geometry (`GarmentFrame.positions`, normals, UVs) feeds the vertex buffer.
- **Wireframe pipeline**: `wgpu::PrimitiveTopology::LineList` with an index buffer enumerating triangle edges. MUST be compositable as an overlay on top of the solid pipeline by a second render pass.
- **Debug pipeline**: renders particles as point sprites and constraint edges as colored lines. Constraint residuals MUST be encoded as a color gradient: green (residual near 0) through yellow to red (residual near 1.0). This is achieved in WGSL without geometry shaders:

```wgsl
// debug_constraints.wgsl
struct ConstraintDebugIn {
    @location(0) residual: f32, // normalized [0.0, 1.0]
};

@fragment
fn fs_main(in: ConstraintDebugIn) -> @location(0) vec4<f32> {
    let r = clamp(2.0 * in.residual, 0.0, 1.0);
    let g = clamp(2.0 * (1.0 - in.residual), 0.0, 1.0);
    return vec4<f32>(r, g, 0.0, 1.0);
}
```

Normal visualization MUST be implemented via a CPU-side arrow mesh (one arrow per vertex normal, uploaded as a vertex buffer each frame). WGSL geometry shaders MUST NOT be used; wgpu/WGSL does not expose them.

A Toon pipeline (wgpu toon fragment shader) MAY be implemented post-MVP. Fur-strand rendering is out of scope.

#### 12.3.4 GPU-to-CPU Readback

`ClothViewport::capture_frame()` MUST implement the staging-buffer readback pattern:

```rust
// Required readback pattern (abridged)
let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
    size: (4 * self.config.width * self.config.height) as u64,
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
    label: Some("cloth_viewport_readback"),
});
let mut encoder = self.device.create_command_encoder(&Default::default());
// ... render pass to color_texture ...
encoder.copy_texture_to_buffer(
    self.color_texture.as_image_copy(),
    wgpu::TexelCopyBufferInfo {
        buffer: &output_buffer,
        layout: wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * self.config.width),
            rows_per_image: Some(self.config.height),
        },
    },
    wgpu::Extent3d {
        width: self.config.width,
        height: self.config.height,
        depth_or_array_layers: 1,
    },
);
self.queue.submit(std::iter::once(encoder.finish()));
let slice = output_buffer.slice(..);
slice.map_async(wgpu::MapMode::Read, |_| {});
self.device.poll(wgpu::PollType::Wait);
```

The resulting raw RGBA bytes MUST be encoded as PNG via the `image` crate (`image::RgbaImage::from_raw`). `capture_frame()` MUST NOT block the Tauri main thread; it MUST be driven from an async Tauri command via `tokio::task::spawn_blocking` or equivalent.

---

### 12.4 Model-Readable Visual Capture

#### 12.4.1 TailorVisualCapture Type

Every visual capture of a simulation frame MUST be represented as a `TailorVisualCapture`. This type extends the existing Handshake visual-debugger contract (`VisualCaptureResult` in `commands/visual_debugger.rs`) with simulation-state metadata:

```rust
// src/tailor/capture.rs
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TailorVisualCapture {
    // Visual surface (compatible with existing VisualCaptureResult)
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
    pub captured_at_utc: String,
    pub render_mode: String,          // "solid" | "wireframe" | "debug_constraints"
    // Simulation identity (T-CONTRACTS canonical ids)
    pub garment_id: String,           // "GAR-{uuid_v7}"
    pub simulation_run_id: String,    // "SIM-{uuid_v7}"
    pub frame_index: u64,
    pub sim_time_seconds: f64,
    // Structured diagnostics — model-parseable without pixel analysis
    pub metadata: SimFrameMetadata,
    // Optional quality verdict written by a model agent
    pub annotation: Option<String>,
    // EventLedger receipt for this capture (TailorGarmentExportCompleted event)
    pub event_ledger_event_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimFrameMetadata {
    pub particle_count: u32,
    pub constraint_count: u32,
    pub max_constraint_residual: f32,
    pub avg_constraint_residual: f32,
    pub collision_count: u32,
    pub kinetic_energy: f32,
    pub step_time_ms: f32,
}
```

`SimFrameMetadata` MUST be returned alongside every PNG capture. A model MUST be able to use `SimFrameMetadata` as a quantitative pre-filter before invoking vision analysis on the PNG.

#### 12.4.2 Settlement Gate

A model MUST NOT accept a simulation as settled and issue a promotion-path request until BOTH of the following conditions are observed from `SimFrameMetadata`:

- `kinetic_energy` is below a configurable threshold (default: `0.01`; stored in the sandbox policy extension fields, NOT hardcoded).
- `max_constraint_residual` is below a configurable threshold (default: `0.05`; stored in the same policy).

These thresholds MUST be derivable empirically from known-good simulations and stored as parameters, not compiled constants. If either condition is not met, the model MUST request additional simulation steps or flag the run as `DRAPE_CONVERGED` advisory failing before proceeding (see T-CONTRACTS validation catalog check `DRAPE_CONVERGED`).

#### 12.4.3 Tauri Commands

The following Tauri commands MUST be implemented in `app/src-tauri/src/commands/tailor.rs`:

```rust
#[tauri::command]
pub async fn tailor_capture_frame(
    garment_id: String,
    simulation_run_id: String,
    frame_index: u64,
    render_mode: String,      // "solid" | "wireframe" | "debug_constraints"
    state: tauri::State<'_, AppState>,
) -> Result<TailorVisualCapture, String>;

#[tauri::command]
pub async fn tailor_viewport_config(
    garment_id: String,
    config: ViewportConfigPatch,
    state: tauri::State<'_, AppState>,
) -> Result<(), String>;
```

`tailor_capture_frame` MUST:

1. Resolve the simulation run from `tailor_simulation_runs` (garment_id + simulation_run_id).
2. Invoke `ClothViewport::capture_frame()` for the requested frame index and render mode.
3. Emit a `TailorGarmentExportCompleted` EventLedger event (canonical variant per T-CONTRACTS [T-CONTRACTS.event-types]) with the capture metadata as payload.
4. Return the populated `TailorVisualCapture`.

#### 12.4.4 Axum API Routes

The following routes MUST be added to `src/api/tailor.rs`:

```rust
.route("/tailor/garments/:id/capture",          post(capture_frame))
.route("/tailor/garments/:id/capture/latest",   get(get_latest_capture))
.route("/tailor/garments/:id/captures/:frame/annotate", post(annotate_capture))
```

#### 12.4.5 Capture Persistence (tailor_captures)

Captures MUST be persisted to PostgreSQL. The migration for this table MUST follow the dated naming convention (T-CONTRACTS [T-CONTRACTS.migration-naming]): `<YYYY>_<MM>_<DD>_tailor_captures.sql` with a paired `.down.sql`.

```sql
-- 2026_MM_DD_tailor_captures.sql
CREATE TABLE IF NOT EXISTS tailor_captures (
    capture_id            TEXT PRIMARY KEY,    -- "CAP-{uuid_v7}"
    garment_id            TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    simulation_run_id     TEXT,
    frame_index           BIGINT NOT NULL,
    render_mode           TEXT NOT NULL,
    png_artifact_id       TEXT,               -- FK into artifact_manifests
    metadata_json         JSONB NOT NULL,     -- SimFrameMetadata
    annotation            TEXT,
    verdict               TEXT
        CHECK (verdict IS NULL OR verdict IN ('accept', 'reject', 'needs_resim')),
    event_ledger_event_id TEXT NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_captures_garment ON tailor_captures (garment_id);
```

Rules:
- `TEXT PRIMARY KEY` with `CAP-` prefix (T-CONTRACTS [T-CONTRACTS.tables] FACT-4 convention).
- `verdict` MUST be set only by a model agent via the annotation path (12.4.6); it MUST NOT be set during the initial capture insert.
- `png_artifact_id` MUST reference the artifact registry when the PNG is stored as an artifact. If the PNG is not retained as an artifact (e.g. ephemeral inspection), the column remains NULL.

#### 12.4.6 Model Annotation (TailorCaptureAnnotated)

A model agent writing a quality verdict on a capture MUST:

1. Call `POST /tailor/garments/:id/captures/:frame/annotate` with a payload of `{ verdict, note }`.
2. The handler MUST update `tailor_captures.verdict` and `tailor_captures.annotation`.
3. The handler MUST emit a `TailorGarmentExportCompleted` EventLedger event with `event_family = "tailor.export"` and payload including `capture_id`, `verdict`, and optional `note`.

The annotation write is a light collaborative edit. It MUST go through the `ai_edit_proposal` path in `kernel/crdt/ai_edit_proposal.rs` if the garment is in a CRDT-collaborative state; otherwise direct write is permitted.

---

### 12.5 Model-First (LLM-Steerable) Capture and Export API

#### 12.5.1 MCP Tool Definitions

The following MCP tool definitions MUST be registered in the Tailor model-lane gate:

```rust
TailorTool::CaptureFrame {
    garment_id: String,
    simulation_run_id: String,
    frame_index: u64,
    render_mode: String,             // "solid" | "wireframe" | "debug_constraints"
    camera_preset: Option<String>,   // "front" | "side" | "top" | "isometric"
}

TailorTool::AnnotateCapture {
    capture_id: String,
    verdict: String,                 // "accept" | "reject" | "needs_resim"
    note: Option<String>,
}

TailorTool::ExportGarment {
    garment_id: String,
    simulation_run_id: String,
    format: String,                  // "obj_sequence" | "gltf_morph" | "usd"
    start_frame: u64,
    end_frame: u64,
    fps: f64,
}
```

Input schemas for these tools MUST be auto-generated via `schemars::JsonSchema` derive on the corresponding request types.

#### 12.5.2 Required Model Inspection Loop

When a model agent evaluates a simulation run, it MUST follow this sequence before issuing any promotion-path action:

1. Call `CaptureFrame` with `render_mode: "debug_constraints"` and read `SimFrameMetadata` to determine whether the settlement gate (12.4.2) is met.
2. If the settlement gate is not met, request additional simulation steps or reject the run. MUST NOT proceed to promotion.
3. If the settlement gate is met, call `CaptureFrame` with `render_mode: "solid"` to obtain the quality PNG.
4. Submit the PNG to the vision capability of the active `LlmClient` and evaluate visual garment quality.
5. Call `AnnotateCapture` with the resulting verdict (`accept | reject | needs_resim`) and record an EventLedger receipt.
6. If the verdict is `accept`, call `ExportGarment` with the appropriate format for the downstream DCC handoff.

---

### 12.6 Render/Export Handoff

#### 12.6.1 Export Format Requirements

Tailor MUST support the following geometry-cache export formats. All formats export `GarmentFrame` sequences (vertex positions, normals, UVs, triangle indices) in the coordinate system and units produced by the solver (centimetres; Y-up).

| Format | Target DCC | MVP status |
|---|---|---|
| OBJ numbered sequence | Blender, Houdini, Maya, any DCC | MUST — MVP; no external crate required |
| glTF 2.0 morph-target (GLB) | Blender, Three.js, Godot, web | MUST — MVP; custom binary GLB encoder required |
| USD time-sample mesh | Blender 5+, Unreal Engine 5, Houdini | SHOULD — MVP if `openusd` v0.5.0 `set_at_time` API is validated |

Alembic (`.abc`) write support MUST NOT be implemented as a native Rust path in the MVP. No production-quality pure-Rust Alembic writer exists as of June 2026 (`ogawa-rs` v0.4.0 is read-only; `ennis/alembic-rs` is WIP). The Alembic export path MUST be documented as a two-step workaround (OBJ sequence + Blender Python headless conversion) in the garment export recipe.

FBX write support MUST NOT be implemented. USD or glTF MUST be preferred for all Unreal Engine handoffs.

#### 12.6.2 OBJ Sequence Export

The OBJ sequence exporter MUST be implemented as a standalone function in `src/tailor/export/obj_sequence.rs` with no external crate dependencies beyond `std::io`. Each frame MUST be written as one `.obj` file named `frame_{:06}.obj` (zero-padded to 6 digits) in the specified output directory.

The writer MUST emit:
- `v x y z` vertex lines (6 decimal places, centimetres).
- `vn x y z` normal lines (6 decimal places).
- `vt u v` UV lines (6 decimal places).
- `f v/vt/vn v/vt/vn v/vt/vn` face lines (1-indexed per OBJ spec).

```rust
// src/tailor/export/obj_sequence.rs
pub fn export_obj_frame(
    frame: &GarmentFrame,
    frame_index: u64,
    out_dir: &std::path::Path,
) -> std::io::Result<()> {
    use std::io::Write;
    let path = out_dir.join(format!("frame_{:06}.obj", frame_index));
    let mut f = std::fs::File::create(&path)?;
    writeln!(f, "# Handshake Tailor frame {}", frame_index)?;
    for p in &frame.positions {
        writeln!(f, "v {:.6} {:.6} {:.6}", p[0], p[1], p[2])?;
    }
    for n in &frame.normals {
        writeln!(f, "vn {:.6} {:.6} {:.6}", n[0], n[1], n[2])?;
    }
    for uv in &frame.uvs {
        writeln!(f, "vt {:.6} {:.6}", uv[0], uv[1])?;
    }
    for tri in &frame.triangles {
        writeln!(f, "f {0}/{0}/{0} {1}/{1}/{1} {2}/{2}/{2}",
            tri[0] + 1, tri[1] + 1, tri[2] + 1)?;
    }
    Ok(())
}
```

#### 12.6.3 glTF Morph-Target Sequence Export

The `gltf` crate v1.4.1 is read-only and MUST NOT be used for export. The glTF export path MUST be a custom binary GLB encoder implemented in `src/tailor/export/gltf_morph.rs`.

The encoder MUST:
- Use `frames[0]` as the base mesh (positions, normals, UVs as glTF accessors).
- Represent each subsequent frame as a morph target: a POSITION delta accessor containing `frame[n].positions - frames[0].positions`.
- Use `ComponentType::FLOAT` (not normalized integer) for all morph-target position delta accessors. Normalized integer encodings MUST NOT be used because cloth simulation deltas can be large, and sub-millimetre accuracy would be lost.
- Write a glTF `animation` object whose sampler drives `mesh.weights` over time, with one weight = 1.0 at the frame's time code and 0.0 elsewhere (`STEP` interpolation).
- Pack all binary data into a single `.glb` chunk (binary glTF container).

Signature:

```rust
// src/tailor/export/gltf_morph.rs
pub fn export_gltf_morph_sequence(
    frames: &[GarmentFrame],
    fps: f64,
    out_path: &std::path::Path,
) -> std::io::Result<()>;
```

#### 12.6.4 USD Time-Sample Mesh Export

The USD exporter MUST use `openusd` v0.5.0 (`mxpv/openusd`) and MUST be gated behind a Cargo feature flag `feature = "usd-export"` so the dependency is not forced on all build targets.

The exporter MUST:
- Create a `UsdGeomMesh` at `/World/GarmentMesh`.
- Write the mesh topology (face vertex counts, face vertex indices, UVs) once at the default time.
- Write per-frame time-sample values for `points` and `normals` using `set_at_time(t, ...)` where `t = frame_index as f64`.
- Set `start_time_code`, `end_time_code`, and `frames_per_second` on the stage.

If the `openusd` v0.5.0 `set_at_time` API does not support mesh time-samples in practice (the API carries no stability guarantee until v1.0), the USD path MUST fall back to the OBJ sequence and log a `WARN`-level message. The fallback MUST NOT silently succeed and return a USD file that does not contain time samples.

Signature:

```rust
// src/tailor/export/usd_export.rs
#[cfg(feature = "usd-export")]
pub fn export_usd_sequence(
    frames: &[GarmentFrame],
    fps: f64,
    out_path: &std::path::Path,
) -> anyhow::Result<()>;
```

#### 12.6.5 Export EventLedger Receipt

Every completed export MUST emit a `TailorGarmentExportCompleted` EventLedger event (canonical variant per T-CONTRACTS [T-CONTRACTS.event-types]; wire string `"TAILOR_GARMENT_EXPORT_COMPLETED"`; `event_family = "tailor.export"`).

The event payload MUST include:

```json
{
  "garment_id": "GAR-…",
  "simulation_run_id": "SIM-…",
  "export_format": "obj_sequence | gltf_morph | usd",
  "frame_count": 120,
  "fps": 24.0,
  "output_path": "/path/to/export/dir",
  "artifact_hash": "<sha256_hex>",
  "frame_range": [0, 119]
}
```

The export artifact (directory for OBJ sequence; single file for GLB or USDC) MUST be registered in the artifact registry via `write_dir_artifact()` (`ArtifactPayloadKind::Bundle` for OBJ sequence) or `write_file_artifact()` (`ArtifactPayloadKind::File` for GLB or USDC). This makes the exported geometry discoverable by downstream model agents via the standard artifact query path.

#### 12.6.6 Export Persistence (tailor_exports)

Exports MUST be persisted to PostgreSQL. The migration MUST follow the dated naming convention: `<YYYY>_<MM>_<DD>_tailor_exports.sql` with a paired `.down.sql`.

```sql
-- 2026_MM_DD_tailor_exports.sql
CREATE TABLE IF NOT EXISTS tailor_exports (
    export_id             TEXT PRIMARY KEY,   -- "EXP-{uuid_v7}"
    garment_id            TEXT NOT NULL REFERENCES tailor_garments (garment_id),
    simulation_run_id     TEXT,
    export_format         TEXT NOT NULL
        CHECK (export_format IN ('obj_sequence', 'gltf_morph', 'usd')),
    frame_count           INT NOT NULL,
    fps                   FLOAT8 NOT NULL,
    output_path           TEXT NOT NULL,
    artifact_hash         TEXT,
    status                TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'completed', 'failed')),
    error_reason          TEXT,
    event_ledger_event_id TEXT NOT NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at          TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS ix_tailor_exports_garment ON tailor_exports (garment_id);
```

Rules:
- `TEXT PRIMARY KEY` with `EXP-` prefix (T-CONTRACTS FACT-4 convention).
- A row MUST be inserted with `status = 'pending'` before the export begins and updated to `'completed'` or `'failed'` when the export finishes. Partial exports MUST NOT be registered as `'completed'`.
- `error_reason` MUST be populated on failure.

The Axum routes for export:

```rust
.route("/tailor/garments/:id/export",  post(export_garment))
.route("/tailor/garments/:id/exports", get(list_exports))
```

---

### 12.7 Constraints and Invariants

- **No SQLite.** All Tailor persistence (captures, exports) MUST use PostgreSQL. Every INSERT on a `tailor_*` table MUST call `guard_authority_write(AuthorityMode::Postgres)` first, per the kernel `no_sqlite_tripwire` convention.
- **No hardcoded paths.** Export output paths MUST be resolved at runtime from the operator-configured artifact root, not compiled constants.
- **CRDT layer is read-only for capture and export.** The viewport and export subsystem reads `GarmentSpec` from the promoted authority row in `tailor_garments`. It MUST NOT write back into the garment CRDT document. The only exception is model annotation (12.4.6), which follows the `ai_edit_proposal` path.
- **Settlement gate is non-negotiable for promotion-path actions.** A model MUST observe the settlement gate (12.4.2) before calling `promote_garment` or any promotion-path tool. Bypassing the gate is a protocol violation.
- **MeshComparator governs promotion equivalence, not content_hash.** If a re-run determinism check is performed during the capture/export flow, it MUST use `MeshComparator::compare(a, b, epsilon_mm)` with `epsilon_mm = 0.1` (T-CONTRACTS [T-CONTRACTS.determinism]). Exact hash comparison across GPU backends MUST NOT be used as a promotion gate.
- **glTF delta accessors MUST use FLOAT.** Normalized integer component types for morph-target position deltas are prohibited (12.6.3).
- **USD export requires feature flag.** The `usd-export` Cargo feature MUST gate the `openusd` dependency. Builds without the feature MUST compile and fall back to OBJ sequence.

---

### 12.8 Risks and Mitigations

| Risk | Mitigation |
|---|---|
| `copy_texture_to_buffer` at 15 fps blocks Tauri main thread | MUST use `tokio::task::spawn_blocking`; readback is explicitly async |
| `openusd` v0.5.0 `set_at_time` API instability | USD path is feature-gated; failure path falls back to OBJ sequence with `WARN` log; OBJ sequence is the unconditional MVP |
| No pure-Rust Alembic writer | Document as a known gap; provide OBJ+Blender-Python conversion recipe; revisit if `ennis/alembic-rs` matures |
| glTF morph-target delta precision loss | `FLOAT` component type mandatory per 12.6.3; enforced in the GLB encoder |
| Settlement-gate thresholds are heuristic initially | Thresholds stored as policy parameters (not compiled constants); configurable via `SandboxPolicyV1` extension fields; to be calibrated from known-good simulation runs |
| Bevy testbed version drift | Explicit Bevy version pin in `handshake-cloth-testbed/Cargo.toml`; upgraded explicitly, not transitively |

---

*Non-normative provenance: T-RENDER-VIEWPORT (cloth_engine_research/08-render-viewport-export.md). All contracts, table names, PK forms, event variants, schema IDs, and migration conventions above are canonical per T-CONTRACTS (cloth_engine_research/16-contracts.md) and supersede any conflicting names in files 01–15 of the research package.*

## 13.13 Model-First API & LLM Steering

<!-- id: modelapi -->
<!-- Non-normative provenance: research package 09-model-first-api.md (T-MODEL-FIRST-API).
     Contract surfaces (type names, field names, units, event variants, schema IDs, table names,
     migration convention, validation codes) are governed by 16-contracts.md (T-CONTRACTS),
     which is canonical and supersedes any drift found in the research source. -->

---

##### <N>.<i>.1 Governing Principle

The `tailor-solver` crate public API MUST be the model API. No separate model shim layered on top of a human-facing API is permitted. Human-facing affordances (sliders, panels) MUST be projections derived from the same typed contracts the model calls.

Consequences that MUST hold across the entire Tailor implementation:

1. All model inputs MUST be typed JSON; no pixel coordinates, no drag handles, no stateful UI session are required.
2. All model outputs MUST be typed JSON receipts readable without prose parsing.
3. Every garment mutation MUST emit an EventLedger event so model actions are attributable, auditable, and replayable.
4. The model MUST be able to self-correct by reading typed feedback from `SimulationReceipt.validation_findings`.
5. The MCP gate MUST be the model's sole entry point; the gate MUST enforce human-in-the-loop consent for `promote_garment` before any authority write.

---

##### <N>.<i>.2 Canonical GarmentSpec (the Model's Primary Input/Output Type)

The canonical garment specification type is **`GarmentSpec`**, defined in `tailor-solver/src/spec.rs` (the standalone crate, no `handshake_core` deps). It is simultaneously the LLM's primary output type, the solver's primary input type, and the JSONB payload stored in `tailor_garments.spec_json`. These MUST remain the same type; duplicating or forking the spec type for model vs. solver use is prohibited.

The schema constant MUST be `hsk.tailor.garment_spec@1` (see [T-CONTRACTS.schema-ids]).

**Required field decisions (all canonical per T-CONTRACTS.garment-spec):**

- **Units: centimetres on all length fields.** Every length field name MUST carry a `_cm` suffix. The `_mm` suffix MUST be used inside the authority body-proxy and solver-internal types only. No field in `GarmentSpec` uses normalized [0,1] coordinates for vertices; normalized [0,1] survives only inside Tier-1 ChatGarment-style pre-decode convenience vectors, which MUST be decoded to `GarmentSpec` (cm) before storage.
- **Vertices and edges: explicit typed edge curves.** `PanelSpec` MUST carry both `vertices_cm: Vec<Vec2Cm>` and `edges: Vec<EdgeSpec>`. The `EdgeShape` enum MUST be used (`Straight | Quadratic { control_cm } | Cubic { control_a_cm, control_b_cm } | Arc { curvature }`); a `curve_type` string is prohibited.
- **Gathering: one float field `gather_ratio: f32` on `SeamSpec`**, defined as `from_length / to_length`. The field name `ratio` (from any earlier draft) MUST NOT be used. Valid range: `(0.0, 20.0]`; `1.0` = flat seam; `> 1.0` gathers the `from` edge onto the shorter `to` edge.
- **Fabric properties: normalized [0.0, 1.0] in `GarmentSpec`** (the LLM-facing surface). `1.0` = stiffest/most resistant. The non-linear map from normalized values to raw XPBD compliance MUST be owned by the preset/decoder layer and applied at solver-mesh build time; it MUST NOT be stored twice. The two non-normalized exceptions are `density_g_m2: f32` (physical, g/m²) and `collision_thickness_mm: f32` (physical, mm), which are LLM-legible physical quantities.
- **Status and timestamps MUST NOT appear in `GarmentSpec`.** They are promotion-lifecycle metadata and belong on the `tailor_garments` Postgres row (`status`, `created_at`, `updated_at`).
- **`natural_description: Option<String>`** SHOULD be carried in `GarmentSpec` as a first-class field. It preserves the NGL-Prompter-style natural-language intermediate alongside the numeric spec and improves edit coherence across multi-turn sessions.

```rust
// tailor-solver/src/spec.rs  (canonical; derives serde + schemars for MCP inputSchema)

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Complete garment: panels, seams, darts, pleats, fabric, avatar binding.")]
pub struct GarmentSpec {
    /// Schema id constant: "hsk.tailor.garment_spec@1".
    pub schema_id: String,
    /// "GAR-{uuid_v7}"
    pub garment_id: String,
    pub workspace_id: String,
    pub name: String,
    pub garment_type: GarmentType,
    /// 2D pattern panels. All coordinates in centimetres.
    pub panels: Vec<PanelSpec>,
    /// Seam definitions joining panel edges. gather_ratio = from_length/to_length.
    pub seams: Vec<SeamSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub darts: Vec<DartSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pleats: Vec<PleatSpec>,
    /// Fabric physical properties: normalized [0,1] LLM-facing surface.
    pub fabric: FabricProperties,
    /// Avatar/body-proxy binding for fit and collision.
    pub avatar: AvatarBinding,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trim_placements: Vec<TrimPlacementRef>,
    /// NGL-Prompter-style natural-language intermediate; aids edit coherence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natural_description: Option<String>,
}

/// 2D point in panel-local coordinate space, CENTIMETRES.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Vec2Cm { pub x: f32, pub y: f32 }

/// Typed edge shape (supersedes curve_type: String).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EdgeShape {
    Straight,
    Quadratic { control_cm: Vec2Cm },
    Cubic     { control_a_cm: Vec2Cm, control_b_cm: Vec2Cm },
    Arc       { curvature: f32 },
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
    /// Outline vertices in panel-local 2D, CENTIMETRES, counter-clockwise. Min 3.
    pub vertices_cm: Vec<Vec2Cm>,
    /// Ordered directed edges closing the outline loop.
    pub edges: Vec<EdgeSpec>,
    pub placement: Transform3D,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grain_angle_deg: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_preset_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Seam joining two panel edges. gather_ratio = from_length/to_length.")]
pub struct SeamSpec {
    pub seam_id: String,
    pub kind: SeamKind,
    pub from: SeamEndpoint,
    pub to: SeamEndpoint,
    /// CANONICAL field name. Valid range (0.0, 20.0]. 1.0 = flat seam.
    pub gather_ratio: f32,
}

/// Fabric physical properties — normalized [0.0,1.0] LLM-facing surface.
/// 1.0 = stiffest/most resistant. Non-linear map to raw XPBD compliance is
/// owned by the preset/decoder layer and applied at solver-mesh build time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(description = "Fabric properties, normalized [0,1]. Weft=cross-grain, Warp=grain.")]
pub struct FabricProperties {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<FabricPreset>,
    pub stretch_weft: f32,
    pub stretch_warp: f32,
    pub shear: f32,
    pub bending_weft: f32,
    pub bending_warp: f32,
    pub buckling_ratio: f32,
    /// g/m² (physical; LLM-legible). Valid range: [5.0, 2000.0].
    pub density_g_m2: f32,
    /// mm (physical; LLM-legible). Valid range: [0.1, 5.0].
    pub collision_thickness_mm: f32,
    pub friction: f32,
    pub internal_damping: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AvatarBinding {
    /// "AVT-{uuid_v7}" or built-in parametric slug. References tailor_avatars.avatar_id.
    pub avatar_id: String,
    /// cm subset for parametric bodies; converted to mm at the API boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurements_cm: Option<BodyMeasurements>,
}

/// cm subset exposed to the LLM. Authority body-proxy stores 25 measurements in mm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BodyMeasurements {
    pub height_cm: f32,
    pub bust_cm: f32,
    pub waist_cm: f32,
    pub hip_cm: f32,
    pub inseam_cm: f32,
}
```

`GarmentSpec` MUST derive `schemars::JsonSchema`. The MCP gate MUST auto-generate the `inputSchema` for `tools/list` discovery from this derive; no manual schema writing is permitted for this type.

---

##### <N>.<i>.3 SimulationReceipt: Typed Feedback for Self-Correction

When the solver completes (success or failure at any gate stage), Tailor MUST emit a `SimulationReceipt` as the `structuredContent` in the MCP tool response. The schema constant MUST be `hsk.tailor.simulation_receipt@1`.

The `SimulationReceipt` MUST carry sufficient information for the model to diagnose failure and propose a corrected `GarmentSpec` without human intervention or prose parsing. Natural-language interpretation MUST NOT be required; the model MUST be able to pattern-match on `ValidationFinding.code` values from the canonical check catalog (see [<N>.<i>.6]).

```rust
// handshake_core/src/tailor/simulation_receipt.rs

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// schema_id: "hsk.tailor.simulation_receipt@1"
/// Returned as MCP structuredContent from simulate_garment and author_garment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimulationReceipt {
    pub schema_id: String,       // "hsk.tailor.simulation_receipt@1"
    /// "SIM-{uuid_v7}" (canonical prefix; supersedes CSIM-).
    pub sim_run_id: String,
    pub status: SimStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_stats: Option<MeshStats>,
    /// Findings from the ValidationDescriptor catalog. Each finding names the
    /// exact panel_id, seam_id, or trim_id that failed, and carries a
    /// suggested_fix the model applies via edit_garment.
    pub validation_findings: Vec<ValidationFinding>,
    /// Drape quality score [0.0, 1.0] when simulation ran to completion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drape_quality_score: Option<f32>,
    pub self_intersections_detected: bool,
    pub open_seam_detected: bool,
    /// Tells the model exactly what to do next. No prose parsing required.
    pub recommended_action: RecommendedAction,
    /// Set when promotion completed; carries the authority garment_id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promoted_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimStatus {
    Completed,
    CompletedWithIssues,
    /// Spec failed fast pre-solver validation.
    RejectedAtValidation,
    SandboxDenied,
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
    /// Stable code from the canonical check catalog. Model pattern-matches on this.
    pub code: String,
    /// "blocking" | "advisory" | "info"
    pub severity: String,
    /// panel_id, seam_id, or trim_id that the finding refers to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_id: Option<String>,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_fix: Option<SuggestedFix>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SuggestedFix {
    /// JSON Pointer (RFC 6901) path into GarmentSpec, e.g. "/fabric/stretch_weft".
    pub field_path: String,
    pub suggested_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedAction {
    PromoteGarment,
    EditAndResimulate,
    CorrectSpecFirst,
    RequiresOperatorAction,
}
```

The `ValidationFinding.suggested_fix.field_path` MUST use RFC 6901 JSON Pointer syntax into the `GarmentSpec` schema. The model MUST use this path verbatim as the merge-patch target in a subsequent `edit_garment` call.

---

##### <N>.<i>.4 MCP Tool Definitions

Tailor MUST expose exactly the following MCP tools through the Handshake MCP gate (`src/mcp/gate.rs`). These constitute the entire model-facing API surface for garment authoring. All parameter structs MUST derive `schemars::JsonSchema` so `inputSchema` is auto-generated at tool discovery. No bespoke integration outside the existing gate infrastructure is permitted.

**Tool registry (canonical names, wired through the tailor tool router):**

| Tool name | Mutation? | Consent required? | Primary event emitted |
|---|---|---|---|
| `author_garment` | draft create | No | `TailorGarmentDraftProposed` |
| `simulate_garment` | sandbox run | No | `TailorSimRunRequested` → `TailorSimRunStarted` → `TailorSimRunCompleted` / `TailorSimRunRejected` |
| `edit_garment` | draft patch | No | `TailorGarmentDraftUpdated` + `TailorPanelCrdtUpdateRecorded` |
| `promote_garment` | authority write | **Yes** | `TailorGarmentPromoted` / `TailorGarmentPromotionRejected` |
| `get_garment` | read-only | No | (none) |
| `estimate_fabric_params` | sandbox run | No | `TailorSimRunRequested` → `TailorSimRunCompleted` |

```rust
// handshake_core/src/tailor/mcp_tools.rs
// Registered through src/mcp/gate.rs via the tailor tool router.

use rmcp::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// --- author_garment ---
// Runs fast pre-solver validation (< 100ms). Returns a SimulationReceipt with
// status=RejectedAtValidation on structural failure so the model corrects before
// the expensive solver run. On success, creates a draft and emits
// TailorGarmentDraftProposed.
#[tool(description = "Create a new garment draft from a GarmentSpec. Fast-validated \
    before any solver run. Returns a draft_id and a SimulationReceipt. If \
    recommended_action=CorrectSpecFirst, apply the suggested_fix patches via \
    edit_garment before calling simulate_garment.")]
async fn author_garment(
    Parameters(input): Parameters<AuthorGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthorGarmentInput {
    pub workspace_id: String,
    /// Complete GarmentSpec. schema_id must be "hsk.tailor.garment_spec@1".
    pub spec: GarmentSpec,
}

// --- simulate_garment ---
// Triggers a TailorSandboxAdapter run (process tier, scoped fs, no network).
// Streams progress; returns a SimulationReceipt when the solver finishes.
// The model reads recommended_action to decide: PromoteGarment or EditAndResimulate.
#[tool(description = "Run the XPBD cloth solver on a garment draft. Returns a \
    SimulationReceipt with validation_findings for self-correction. Use \
    substeps <= 16 and frames <= 60 for exploratory iterations; increase only \
    for final promotion-quality runs.")]
async fn simulate_garment(
    Parameters(input): Parameters<SimulateGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimulateGarmentInput {
    pub draft_id: String,
    /// Substeps per frame [1..64]. Default: 8.
    #[serde(default = "default_substeps")]
    pub substeps: u32,
    /// Constraint solver iterations per substep [1..32]. Default: 4.
    #[serde(default = "default_iterations")]
    pub solver_iterations: u32,
    /// Number of simulation frames to run. Default: 30.
    #[serde(default = "default_frames")]
    pub frames: u32,
}

// --- edit_garment ---
// Applies a JSON Merge Patch (RFC 7396) to the existing GarmentSpec.
// The model derives the patch from ValidationFinding.suggested_fix.field_path.
// Emits TailorGarmentDraftUpdated and, for panel vertex changes,
// TailorPanelCrdtUpdateRecorded (the CRDT machinery is transparent to the model).
#[tool(description = "Apply a partial update to a garment draft using JSON Merge Patch \
    (RFC 7396). Derive the patch from ValidationFinding.suggested_fix. Returns \
    an updated draft_id for the next simulate_garment call.")]
async fn edit_garment(
    Parameters(input): Parameters<EditGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditGarmentInput {
    pub draft_id: String,
    /// RFC 7396 JSON Merge Patch against the existing GarmentSpec.
    /// Unspecified fields are preserved. Example: {"fabric":{"stretch_weft":0.3}}.
    pub patch: serde_json::Value,
}

// --- promote_garment ---
// Requires SimulationReceipt.recommended_action == PromoteGarment.
// Requires ConsentDecision::Allow from ConsentProvider (operator confirmation).
// On success, writes a tailor_garments authority row (status=promoted) and
// emits TailorGarmentPromoted.
#[tool(description = "Promote a simulated garment draft to authority storage. \
    Only valid when SimulationReceipt.recommended_action == promote_garment. \
    Requires operator consent. Do NOT call if recommended_action is \
    requires_operator_action.")]
async fn promote_garment(
    Parameters(input): Parameters<PromoteGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PromoteGarmentInput {
    pub draft_id: String,
    /// sim_run_id from the SimulationReceipt (idempotency key for the PromotionGate).
    pub sim_run_id: String,
    pub label: String,
}

// --- get_garment ---
// Returns the current GarmentSpec and latest SimulationReceipt for a draft or
// authority garment. The model MUST call this after a session boundary to reload
// state instead of relying on chat history.
#[tool(description = "Read the current GarmentSpec and latest SimulationReceipt \
    for a draft or authority garment. Use this to reload state after a handoff \
    or session boundary.")]
async fn get_garment(
    Parameters(input): Parameters<GetGarmentInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetGarmentInput {
    pub garment_id: String,
}

// --- estimate_fabric_params (optional, feature-flagged) ---
// Runs a DiffXPBD-style differentiable forward-backward pass in the sandbox to
// optimise FabricProperties toward a target drape image. Feature flag: diff-xpbd.
// The model MUST attempt forward simulation with preset-derived parameters first;
// invoke this only when the drape is visually wrong and a reference image exists.
#[tool(description = "Estimate FabricProperties to match a target drape image. \
    Requires an existing draft_id with panels and seams set. Returns \
    FabricProperties the model uses in an edit_garment patch. Only invoke when \
    preset-based forward simulation produces visually wrong drape.")]
async fn estimate_fabric_params(
    Parameters(input): Parameters<EstimateFabricInput>,
) -> Result<CallToolResult, McpError> { ... }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EstimateFabricInput {
    pub draft_id: String,
    pub target_image_artifact_id: String,
    /// Gradient descent iterations. Default: 50.
    #[serde(default = "default_fabric_iterations")]
    pub max_iterations: u32,
}
```

**Constraints on tool registration:**

- Every tool MUST be registered in the tailor tool router and exposed through the existing `src/mcp/gate.rs`; no parallel gate is permitted.
- `promote_garment` MUST check `ConsentDecision::Allow` from the `ConsentProvider` before any authority write. Any other tool MUST NOT require explicit consent under a standard operator policy.
- Every tool call that mutates state MUST emit the corresponding `KernelEventType` variant via `NewKernelEvent::builder(...)` before returning, following the `kb003_storage.rs` event-emission pattern.
- The `SandboxPolicyV1` MUST enforce a maximum substep budget and iteration count for `simulate_garment` and `estimate_fabric_params`; the solver MUST return `SimStatus::TimedOut` rather than run indefinitely.

---

##### <N>.<i>.5 Self-Correction Loop

The model MUST follow a bounded self-correction loop. The loop structure MUST be:

```
author_garment(spec)
  │
  ▼
Fast Validation (< 100ms, no solver)           ← checks: PANEL_CLOSURE, SEAM_EDGE_REF,
  │                                              GATHER_RATIO_RANGE, FABRIC_RANGE,
  │                                              AVATAR_BINDING, MIN_PANEL_AREA, WINDING
  ├─ FAIL → SimulationReceipt(status=RejectedAtValidation,
  │          recommended_action=CorrectSpecFirst)
  │          → model reads suggested_fix, calls edit_garment(patch), re-calls author_garment
  │
  └─ PASS → draft created
               │
               ▼
          simulate_garment(draft_id, substeps, frames)
               │
               ├─ FAIL → SimulationReceipt(status=CompletedWithIssues or TimedOut,
               │          recommended_action=EditAndResimulate)
               │          → model reads validation_findings, applies suggested_fix patches
               │            via edit_garment, re-calls simulate_garment
               │
               └─ PASS → SimulationReceipt(status=Completed,
                          drape_quality_score >= 0.7,
                          recommended_action=PromoteGarment)
                             │
                             ▼
                        promote_garment(draft_id, sim_run_id, label)
                        [operator consent gate]
                             │
                             ▼
                        tailor_garments authority row (status=promoted)
                        EventLedger: TailorGarmentPromoted
```

**Loop bounds and hard stop conditions (MUST be enforced):**

The model MUST stop and report to the operator when any of the following conditions are true:

1. More than five `simulate_garment` iterations with the same panel configuration (vertex coordinates unchanged between iterations).
2. `SimStatus::SandboxDenied` is returned.
3. `RecommendedAction::RequiresOperatorAction` is returned.

The model MUST NOT call `promote_garment` unless `SimulationReceipt.recommended_action == promote_garment`.

**Drape quality threshold.** A `drape_quality_score >= 0.7` (on the 0.0–1.0 scale) SHOULD be required before `PromoteGarment` is issued. The exact threshold is configurable in `SimRunParams`; the default MUST be `0.7`.

**Vertex winding auto-correction.** Fast validation MUST auto-correct counter-clockwise winding on panels that arrive clockwise and emit a `WINDING` finding with `severity="info"`. The model MUST be informed of the correction via the `ValidationFinding` in the receipt; the model MUST NOT treat this as a loop-restarting failure.

---

##### <N>.<i>.6 Validation Check Catalog (Model-Facing Subset)

The canonical check catalog is defined in T-CONTRACTS.validation. This section specifies the subset directly relevant to the model's self-correction loop, with the information needed to match `code` values to corrective `edit_garment` patches.

**Fast pre-solver checks** (returned from `author_garment`; `SimStatus::RejectedAtValidation`):

| Code | Severity | What failed | Model corrective action |
|---|---|---|---|
| `PANEL_CLOSURE` | Blocking | Panel polygon not closed or self-intersecting | Fix `panels[n].vertices_cm` to form a valid closed polygon |
| `SEAM_EDGE_REF` | Blocking | `SeamSpec.from` or `to` references invalid `panel_id` or `edge_index` | Fix `seams[n].from.panel_id`, `.from.edge_index`, `.to.*` |
| `GATHER_RATIO_RANGE` | Blocking | `SeamSpec.gather_ratio` outside `(0.0, 20.0]` | Set `seams[n].gather_ratio` to a value in `(0.0, 20.0]` |
| `FABRIC_RANGE` | Blocking | Normalized fabric field outside `[0.0, 1.0]`, or `density_g_m2` outside `[5, 2000]`, or `collision_thickness_mm` outside `[0.1, 5]` | Clamp the reported field to its valid range |
| `AVATAR_BINDING` | Blocking | `AvatarBinding.avatar_id` not found in `tailor_avatars` | Use a valid `avatar_id` from `tailor_avatars` or a known built-in parametric slug |
| `MIN_PANEL_AREA` | Blocking | Panel area below 1.0 cm² | Expand `panels[n].vertices_cm` or remove the degenerate panel |
| `WINDING` | Info | Panel vertices clockwise; auto-corrected | No action required; correction already applied |

**Post-simulation checks** (returned from `simulate_garment`; `SimStatus::CompletedWithIssues`):

| Code | Severity | What failed | Model corrective action |
|---|---|---|---|
| `SEAMS_CLOSED` | Blocking | Seam constraint pair separation > 1 mm at rest | Increase `gather_ratio` toward `1.0`, or adjust `panels[n].vertices_cm` so edges are compatible lengths |
| `NO_INTERPENETRATION` | Blocking | Cloth particle deeper than −0.5 mm inside body proxy | Increase `fabric.collision_thickness_mm`, or adjust `panels[n].placement.translation_cm` away from body surface |
| `MESH_NOT_EMPTY` | Blocking | Simulated vertex buffer empty | Verify `avatar_id` is valid and `panels` are non-empty |
| `NO_DEGENERATE_TRIS` | Blocking | Zero-area triangles in output | Increase `MIN_PANEL_AREA` (expand vertices), reduce particle spacing |
| `UV_COVERAGE` | Blocking | UV islands cover < 95% of mesh surface | This is post-sim; re-simulate after seam and panel corrections |
| `SELF_INTERSECTION` | Advisory | Self-collision pair count above limit | Increase `fabric.collision_thickness_mm` or increase `substeps` |
| `DRAPE_CONVERGED` | Advisory | Final kinetic energy above convergence threshold | Increase `frames` or `substeps`; reduce `internal_damping` |

The full catalog (including trim, multi-layer, refit, and material-preset checks) is defined in T-CONTRACTS.validation and governs the `TailorValidationDescriptor`. The codes in this section MUST match T-CONTRACTS verbatim; no local aliases are permitted.

---

##### <N>.<i>.7 TailorModelAdapter: Kernel Binding

The `TailorModelAdapter` MUST implement the `ModelAdapter` trait (`src/kernel/model_adapter.rs`) and serve as the kernel-level entry point for all model-driven garment authoring. The MCP tools MUST call into this adapter. No direct database writes outside the adapter and the `PromotionGate` are permitted for garment authority rows.

The adapter MUST:

1. Deserialize `ContextBundle.allowed_context` as `GarmentSpec` before any other processing.
2. Run fast validation synchronously and return a `SimulationReceipt(status=RejectedAtValidation)` if any blocking check fails; do not proceed to draft creation.
3. Emit `TailorGarmentDraftProposed` via `NewKernelEvent::builder(...)` (wire string: `"TAILOR_GARMENT_DRAFT_PROPOSED"`) after a successful fast-validation draft creation.
4. Compute `output_hash` as SHA-256 of the canonical JSON bytes of the adapter payload and store it in `SolverResult.content_hash` for same-machine idempotency. This hash MUST NOT be used for cross-backend promotion equivalence (see [<N>.<i>.8]).

**New `KernelEventType` variants required** (added to `kernel/mod.rs` enum and `required_first_slice_events()`; wire strings use `as_str()` SCREAMING_SNAKE_CASE per T-CONTRACTS.event-types):

```rust
// Garment lifecycle
TailorGarmentDraftProposed,      // "TAILOR_GARMENT_DRAFT_PROPOSED"
TailorGarmentDraftUpdated,       // "TAILOR_GARMENT_DRAFT_UPDATED"
TailorGarmentValidationRecorded, // "TAILOR_GARMENT_VALIDATION_RECORDED"
TailorGarmentPromoted,           // "TAILOR_GARMENT_PROMOTED"
TailorGarmentPromotionRejected,  // "TAILOR_GARMENT_PROMOTION_REJECTED"

// Simulation run lifecycle
TailorSimRunRequested,           // "TAILOR_SIM_RUN_REQUESTED"
TailorSimRunStarted,             // "TAILOR_SIM_RUN_STARTED"
TailorSimRunCompleted,           // "TAILOR_SIM_RUN_COMPLETED"
TailorSimRunRejected,            // "TAILOR_SIM_RUN_REJECTED"

// CRDT collaborative editing
TailorPanelCrdtUpdateRecorded,   // "TAILOR_PANEL_CRDT_UPDATE_RECORDED"
TailorPanelCrdtSnapshotRecorded, // "TAILOR_PANEL_CRDT_SNAPSHOT_RECORDED"
TailorPanelAiEditProposalRecorded, // "TAILOR_PANEL_AI_EDIT_PROPOSAL_RECORDED"
TailorPanelAiEditProposalDecided,  // "TAILOR_PANEL_AI_EDIT_PROPOSAL_DECIDED"
TailorCrdtConflictDetected,      // "TAILOR_CRDT_CONFLICT_DETECTED"
```

Superseded variant names that MUST NOT be used: `TailorGarmentValidated`, `TailorPatternValidated`, `TailorCrdtUpdateRecorded`, `TailorGarmentCrdtUpdateRecorded`. The wire string `"GARMENT_PATTERN_PROMOTED"` (missing the `TAILOR_` prefix) is prohibited; the canonical wire string is `"TAILOR_GARMENT_PROMOTED"`.

**Schema IDs.** The canonical namespace is `hsk.tailor.*`. The strings `hsk.cloth.garment_draft@1`, `hsk.cloth.solver_request@1` that appear in earlier drafts are superseded by `hsk.tailor.*` names. The single allowed exception is the pair of solver-crate-internal physics payloads that never become authority rows: `hsk.cloth.solver_request@1` and `hsk.cloth.solver_result@1`.

---

##### <N>.<i>.8 Promotion Equivalence: MeshComparator (not content_hash)

The `PromotionGate` `ValidationRunner` MUST use the `MeshComparator` for promotion equivalence, not `content_hash` equality. Cross-backend float rounding in WGSL/wgpu means identical-quality drapes produce different hashes on different GPU backends or driver versions; gating promotion on hash equality produces spurious failures.

`content_hash` (SHA-256 of the final position buffer) MUST be used only for:
- Same-machine, same-run idempotency (deduplicate identical re-submissions on one machine).
- EventLedger receipt fingerprinting (stored in `tailor_simulation_runs.content_hash`).

The `MeshComparator` MUST implement the following equivalence check:

```text
PRIMARY (continuous, epsilon-tolerant):
  per-vertex position deviation <= epsilon_mm (default: 0.1 mm)
  compared vertex-for-vertex in canonical vertex order
  (vertex ordering is deterministic from mesh topology + constraint coloring,
   computed once at garment load and stored — stable cross-backend)
  metrics reported: max per-vertex Euclidean deviation AND mean deviation

SECONDARY (exact topology invariants — must match exactly):
  vertex_count        == expected
  triangle_count      == expected
  seam_edge_pair_count == expected
  panel_count         == expected

VERDICT: equivalent for promotion iff all SECONDARY invariants match exactly
         AND max per-vertex deviation <= epsilon_mm
```

`MeshComparator` MUST be a pure function in `tailor-solver/src/compare.rs`, reused by the kernel validation runner via the `ClothSolver` trait boundary. `epsilon_mm` MUST be a field on `SimRunParams` (default `0.1`); the operator MAY tighten it for hero assets or loosen it for exploratory runs.

For animated runs where wind turbulence is present, the comparator additionally MUST accept a shape-envelope match (per-frame bounding box within `bbox_epsilon_mm`, default `1.0 mm`, plus `SEAMS_CLOSED` passing) as the promotion equivalence basis, because cross-vendor turbulence precision cannot achieve per-vertex reproduction.

---

##### <N>.<i>.9 Context Bundle Design

The `ContextBundle.allowed_context` MUST contain everything the model needs for a garment authoring session without additional tool calls to reconstruct state. The bundle MUST include:

```json
{
  "task": "author_garment",
  "workspace_id": "<workspace_id>",
  "operator_brief": "<natural language garment description>",
  "avatar_summary": {
    "avatar_id": "<AVT-uuid_v7 or built-in slug>",
    "height_cm": 165.0,
    "bust_cm": 86.0,
    "waist_cm": 68.0,
    "hip_cm": 92.0
  },
  "garment_history": [],
  "available_presets": ["cotton", "jersey", "denim", "silk", "leather", "satin",
                        "linen", "wool", "spandex", "chiffon", "canvas", "rubber"],
  "solver_budget": {
    "max_substeps": 16,
    "max_iterations": 8,
    "max_frames": 60,
    "max_particles": 50000
  },
  "ngl_description": "<structured natural-language garment description for panel planning>",
  "reference_spec_id": null
}
```

When `reference_spec_id` is non-null (an editing session), the model MUST call `get_garment` to load the existing `GarmentSpec` and MUST apply a JSON Merge Patch via `edit_garment` rather than re-authoring from scratch. This constraint prevents spec oscillation across iterations.

The `ngl_description` field SHOULD be populated by the orchestrating system or operator before the model is invoked. It MUST describe the garment in a VLM-legible structured natural language (garment category, neckline, sleeve length, length, fit, ease) so the model can use it as a planning step before emitting panel shapes and seam definitions.

---

##### <N>.<i>.10 Fabric Parameter Estimation (Inverse Path)

The `estimate_fabric_params` tool implements a DiffXPBD-style differentiable forward-backward pass to optimise `FabricProperties` toward a target drape image. It MUST be compiled behind the `diff-xpbd` feature flag because the differentiable XPBD path is approximately 10x slower than the forward-only path.

The model MUST NOT invoke `estimate_fabric_params` unless:
- A reference image artifact is available (the `target_image_artifact_id` parameter is non-null and resolves to a valid artifact).
- Forward simulation with at least one preset-derived `FabricProperties` has already been run and its drape quality is visually unacceptable.

The tool MUST return a `SuggestedFabricParams` receipt containing the estimated `FabricProperties` as a struct the model inserts directly into an `edit_garment` patch. The format MUST be compatible with the patch accepted by `edit_garment` (a JSON Merge Patch fragment targeting `/fabric`).

The solver MUST run gradient descent on the normalized `FabricProperties` fields (the LLM-facing surface); the non-linear map to raw XPBD compliance MUST be applied inside the solver at mesh-build time, not exposed to the optimiser as a separate parameter space.

---

##### <N>.<i>.11 CRDT Collaboration Surface

Multiple model instances and/or a human operator MAY edit the same garment draft concurrently. The following MUST hold:

- Each garment draft MUST map to a `crdt_document_id` in `tailor_garment_crdt_docs` (canonical table; see T-CONTRACTS.tables).
- Panel vertex edits MUST be stored as `CrdtUpdateRecordV1` rows keyed by `(garment_id, panel_id, actor_site)` in `kernel_crdt_updates`.
- Seam edits MUST be separate `CrdtUpdateRecordV1` rows so panel and seam edits can merge independently.
- `KnowledgeStateVectorV1` MUST track per-actor version vectors; `causality_verdict` MUST detect concurrent edits.
- Concurrent edits to the same panel from two actors MUST be resolved by `promote_bridge` (last-writer-wins within a substep) unless an actor holds a lease via `KnowledgeCrdtLeaseClaimed`.
- The CRDT machinery MUST be transparent to the model: the `edit_garment` tool MUST handle merge and emit `TailorPanelCrdtUpdateRecorded`; the model MUST read the merged state via `get_garment`.

When a `TailorCrdtConflictDetected` event is emitted (merged panel set does not form a closed garment), both editing sessions MUST be notified via `SimulationReceipt(status=CompletedWithIssues, validation_findings=[{code:"SEAMS_CLOSED",...}])` on the next `simulate_garment` call. The conflict MUST NOT silently produce a promotion-eligible state.

---

##### <N>.<i>.12 Built-in Model Manual

Per the Handshake product policy, Tailor MUST include a built-in model manual embedded in the MCP server's capability advertisement and in `ContextBundle.allowed_context` for any `task = "author_garment"` session. The manual MUST enable any LLM with no prior context to operate the tool.

The manual MUST cover:

1. **Tool call order:** `author_garment` MUST be called first (fast validation, no solver). Then `simulate_garment`. Then optionally `estimate_fabric_params`. Then `promote_garment` only when `recommended_action == promote_garment`.
2. **Self-correction protocol:** After every `simulate_garment` call, the model MUST read `SimulationReceipt.validation_findings`. For each blocking finding, the model MUST extract `suggested_fix.field_path` and `suggested_fix.suggested_value`, construct a JSON Merge Patch, and call `edit_garment(patch)` before re-calling `simulate_garment`.
3. **Hard stop conditions:** Stop and report to the operator when: more than five `simulate_garment` iterations with unchanged panel vertices; `SimStatus::SandboxDenied`; `RecommendedAction::RequiresOperatorAction`.
4. **Fabric presets:** Use `FabricPreset` enum values for common materials. Override individual `FabricProperties` fields only when a specific tactile property is required. Call `estimate_fabric_params` only when a reference image exists and preset-based drape is visually wrong.
5. **Avatar measurements:** When avatar body measurements are in the operator brief, pass them in `AvatarBinding.measurements_cm`. The solver converts these to mm at the boundary for collision proxy sizing.
6. **Solver budget:** Use `substeps <= 16` and `frames <= 60` for exploratory iterations. Increase only for the final promotion run.
7. **Session handoff:** After any session boundary, call `get_garment(garment_id)` to reload `GarmentSpec` and the latest `SimulationReceipt` before proceeding. Do not rely on chat history.

The system prompt injected into `CompletionRequest.prompt` via the `LlmClient` trait MUST include the tool call order, self-correction protocol, and hard stop conditions in machine-readable form. Natural language explanation MAY supplement but MUST NOT replace the structured protocol.

---

##### <N>.<i>.13 Storage Binding (Postgres Authority Tables)

All garment authority writes MUST use Postgres. SQLite is prohibited for any Tailor authority row. Every `INSERT` and `UPDATE` MUST call `guard_authority_write(AuthorityMode::Postgres)` first.

The canonical Tailor Postgres tables relevant to this sub-section, with their primary-key prefixes (per T-CONTRACTS.tables):

| Table | PK prefix | Relevant to model API |
|---|---|---|
| `tailor_garments` | `GAR-` | Authority garment row; `spec_json JSONB` stores `GarmentSpec`; `status` CHECK (`draft\|sandbox_pending\|simulated\|validated\|promoted\|rejected\|archived`) |
| `tailor_garment_crdt_docs` | composite `(garment_id, crdt_document_id)` | One CRDT document per garment draft |
| `tailor_avatars` | `AVT-` | Avatar identity; `AvatarBinding.avatar_id` references this table |
| `tailor_simulation_runs` | `SIM-` | One row per `simulate_garment` call; `content_hash` for idempotency; FK to `kb003_sandbox_runs.run_id` |
| `tailor_material_presets` | `MAT-` | Fabric preset library; `is_system_preset` flag |

Migration naming convention MUST follow the dated form `YYYY_MM_DD_tailor_<topic>.sql` with a `.down.sql` reverse pair. Numbered `0NNN_*` migrations are prohibited for Tailor (collision risk with the shared numbered sequence). The date MUST be the authoring date of the migration, not this research date.

---

##### <N>.<i>.14 Kernel Primitive Binding Summary

| Model API surface | Kernel primitive | Source location |
|---|---|---|
| `author_garment` MCP tool | `TailorModelAdapter.invoke()` + fast `ValidationRunner` | `tailor/model_adapter.rs` |
| `simulate_garment` MCP tool | `TailorSandboxAdapter.run()` + `SandboxRunV1` lifecycle | `tailor/solver_binding.rs` |
| `edit_garment` MCP tool | `CrdtUpdateRecordV1` via `kernel_crdt_updates` | `tailor/crdt.rs` |
| `promote_garment` MCP tool | `PromotionGate.evaluate()` + `PromotionReceiptV1` | `kernel/kb003_promotion/gate.rs` |
| `get_garment` MCP tool | `Database.get_garment()` (new trait method) | `storage/postgres.rs` |
| `estimate_fabric_params` tool | `TailorSandboxAdapter` (diff-xpbd feature) | `tailor/solver_binding.rs` |
| `TailorGarmentDraftProposed` event | `NewKernelEvent::builder(...)` after `author_garment` | `tailor/storage_glue.rs` |
| `TailorSimRunCompleted` event | `NewKernelEvent::builder(...)` after solver completes | `tailor/storage_glue.rs` |
| `TailorGarmentPromoted` event | `NewKernelEvent::builder(...)` after `PromotionGate` accepts | `tailor/storage_glue.rs` |
| MCP input schema generation | `schemars::JsonSchema` derive on `GarmentSpec` et al. | `tailor-solver/src/spec.rs` |
| JSON instance validation | `jsonschema` crate (`mcp/schema.rs` pattern) | `tailor/fast_validate.rs` |
| LLM calls via `LlmClient` | `CompletionRequest` / `CompletionResponse` (HSK-TRAIT-004) | `llm/mod.rs` |
| Promotion equivalence | `MeshComparator::compare(a, b, epsilon_mm)` | `tailor-solver/src/compare.rs` |
| CRDT merge | `promote_bridge` in `kernel/crdt/promotion_bridge.rs` | `kernel/crdt/` |

---

*Non-normative provenance: research package `09-model-first-api.md` (T-MODEL-FIRST-API) and `16-contracts.md` (T-CONTRACTS). Contract surfaces defer to T-CONTRACTS; research source is rationale and OSS evidence only.*

## 13.14 Canonical Tailor Authority Contracts

<!-- id: contracts -->
<!-- KERNEL_BUILDER: renumber heading on assembly. -->
<!-- Non-normative provenance: research package T-CONTRACTS (wt-gov-kernel/.GOV/reference/cloth_engine_research/16-contracts.md, 2026-06-17). -->
<!-- This sub-section IS normative product law. All prior research sketches defer to it for every contract surface listed here. -->

---

##### Scope and Authority

This sub-section is the binding contract for all Tailor module implementation. Where any other Tailor spec sub-section defines a schema, event variant, table, column, migration name, or validation check, and that definition conflicts with this sub-section, **this sub-section MUST win**. Other sub-sections remain valid as design rationale and OSS evidence; their concrete contract surfaces are superseded by the canonical forms below.

This sub-section resolves the following cross-sub-section drift categories:

- `KI-CONTRACTS-DRIFT`: incompatible GarmentSpec/body-proxy/event-variant definitions across sub-sections
- `KI-MIGRATION-COLLISION`: numbered migration `0334_tailor_garments.sql` collides with the live `0334_loom_canvas_boards.sql` (WP-KERNEL-009 MT-261)
- `KI-DETERMINISM-VS-PROMOTION`: exact SHA-256 `content_hash` comparison fails cross-backend for promotion equivalence
- Schema-ID namespace drift (`hsk.cloth.*` vs `hsk.tailor.*`)
- Missing `tailor_avatars` table (referenced-but-never-defined across collision and autofit sub-sections)

---

##### Naming Rules

All Tailor contract surfaces MUST follow this naming table exactly. No exception is permitted without a superseding spec amendment.

| Surface | Canonical form | Example |
|---|---|---|
| Kernel binding module | `handshake_core::tailor` (`src/tailor/`) | `src/tailor/garment.rs` |
| Solver crate | `tailor-solver` (workspace member; no `handshake_core` dep) | `tailor-solver/src/lib.rs` |
| Solver trait | `ClothSolver` (physics term stays `cloth`) | `tailor-solver/src/lib.rs` |
| `KernelEventType` variants | `Tailor*` PascalCase | `TailorGarmentPromoted` |
| EventLedger wire string | `TAILOR_*` SCREAMING_SNAKE_CASE via `as_str()` | `"TAILOR_GARMENT_PROMOTED"` |
| `event_family` constants | `tailor.<domain>.<verb>` lowercase dotted | `"tailor.garment.promoted"` |
| Postgres tables | `tailor_*` snake_case | `tailor_garments` |
| Domain types | `Garment*`, `Panel`, `Seam`, `Fabric*` | `GarmentSpec`, `PanelSpec` |
| Physics types | `Cloth*`, `ClothParticle`, `ClothConstraint` | `ClothSolver`, `ClothBodyProxy` |
| Schema ID constants | `hsk.tailor.<record>@<v>` | `hsk.tailor.garment_spec@1` |

Two naming corrections this sub-section makes that MUST NOT be re-introduced:

1. **Schema-ID namespace is `hsk.tailor.*`, not `hsk.cloth.*`.** The domain is `tailor`; the kernel convention is `hsk.<domain>.<record>@<v>`. The sole allowed `hsk.cloth.*` exception is the pair of solver-crate-internal physics payloads that never cross the `tailor-solver` crate boundary as authority records (see [Schema-ID Constants] below).

2. **The canonical top-level garment type is `GarmentSpec`, not `GarmentSpecV1` and not `GarmentDraftV1`.** All references to those alternate names MUST be replaced.

---

##### ONE Canonical GarmentSpec

`GarmentSpec` MUST be the single garment authority type across the Tailor module. It is simultaneously the model's primary output type, the solver's primary input type, and the Postgres authority JSONB stored in `tailor_garments.spec_json`. It MUST be `schemars::JsonSchema`-derivable so the MCP `inputSchema` is auto-generated. It MUST live in the `tailor-solver` crate (`tailor-solver/src/spec.rs`) so the crate public API equals the model API.

The following decisions MUST be observed in every implementation:

- **Units: centimetres (cm) everywhere in `GarmentSpec`.** Every length field name MUST carry the `_cm` suffix so the unit is self-documenting. Normalized [0,1] vertex coordinates (from ChatGarment-style LLM input) are decoded to cm `GarmentSpec` before storage and MUST NOT appear in the authority type.

- **Gather: one float field `gather_ratio: f32` on `SeamSpec`, defined as `from_length / to_length`.** The field MUST be named `gather_ratio` (not `ratio`). Valid range MUST be `(0.0, 20.0]`. The solver represents M:N gathering by resampling both edges to equal vertex count N at mesh-generation time; no alternative stored field is permitted.

- **Fabric values: normalized [0,1] in `GarmentSpec`; raw XPBD compliance in the preset library and solver only.** `FabricProperties` fields MUST be normalized [0,1] (1.0 = stiffest). The non-linear map to raw compliance is owned by the preset/decoder layer and MUST NOT be stored twice.

- **Panel/edge/seam shapes: explicit vertices + typed `EdgeShape` enum, in cm.** Polygon-only panels without edge curves are NOT permitted. String `curve_type` fields are NOT permitted. The `EdgeShape` enum (`Straight | Quadratic | Cubic | Arc`) is the only permitted representation.

- **`GarmentStatus` and timestamps MUST NOT appear on `GarmentSpec`.** They are promotion-lifecycle metadata on the `tailor_garments` Postgres row (`status` column, `created_at`, `updated_at`), not garment content.

```rust
// tailor-solver/src/spec.rs  (standalone crate; no handshake_core deps)
// THIS is the canonical garment type. Supersedes GarmentSpecV1 and GarmentDraftV1.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// Optional trim placements (buttons, zippers, eyelets).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trim_placements: Vec<TrimPlacementRef>,
    /// Optional natural-language description (NGL-Prompter intermediate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub natural_description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GarmentType {
    Tshirt, Shirt, Jacket, Blazer, Dress, Skirt, Pants, Shorts,
    Bodice, Cape, Hood, Sleeve, Custom,
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

/// Edge shape in panel-local 2D (cm).
/// MUST use this typed enum; string curve_type is NOT permitted.
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
    /// Outline vertices in panel-local 2D, CENTIMETRES, counter-clockwise. Min 3.
    pub vertices_cm: Vec<Vec2Cm>,
    /// Ordered directed edges closing the outline loop.
    pub edges: Vec<EdgeSpec>,
    /// 6D placement for initial draping.
    pub placement: Transform3D,
    /// Grain direction, degrees from panel horizontal. None = isotropic.
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
    /// Gathering ratio = from_length / to_length. 1.0 = flat. Range: (0.0, 20.0].
    /// CANONICAL field name. The name `ratio` (used in earlier drafts) is NOT permitted.
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

/// Fabric physical properties — NORMALIZED [0.0, 1.0] LLM-facing surface.
/// 1.0 = stiffest/most resistant. The non-linear map to raw XPBD compliance
/// is owned by the preset/decoder layer and applied at solver-mesh build time.
/// MUST NOT store raw compliance values here.
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
    /// Mass per unit area in g/m^2 (physical, LLM-legible; not normalized).
    pub density_g_m2: f32,
    /// Collision thickness in mm (physical, LLM-legible; not normalized).
    pub collision_thickness_mm: f32,
    pub friction: f32,
    pub internal_damping: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FabricPreset {
    Cotton, Denim, Silk, Jersey, Leather, Satin, Linen, Wool,
    Spandex, Chiffon, Canvas, Rubber,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AvatarBinding {
    /// Body-proxy authority id (tailor_avatars.avatar_id). Form: "AVT-{uuid_v7}"
    /// or a built-in parametric body slug (e.g. "avatar1-smplx-default").
    /// MUST reference tailor_avatars, NOT tailor_material_library.
    pub avatar_id: String,
    /// Optional measurement overrides (cm) for parametric bodies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub measurements_cm: Option<BodyMeasurements>,
}

/// LLM-facing body measurements in CENTIMETRES.
/// The authority body-proxy stores the full 25-measurement set in MILLIMETRES
/// (tailor_body_proxies.proxy_json). This cm subset is converted at the API boundary.
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

The solver-input mesh type is `SolverMesh` (canonical name; `SolverMeshV1` is NOT permitted), defined in `tailor-solver/src/mesh.rs`. The `SeamConstraintRecord` on `SolverMesh` MUST use `gather_ratio` matching the canonical seam field. All lengths in cm.

---

##### ONE Canonical Body-Proxy / Avatar Authority Schema

The body-proxy MUST be represented as two distinct tables: `tailor_avatars` (avatar identity authority) and `tailor_body_proxies` (solver collision geometry). The missing `tailor_avatars` table is authored as a canonical contract here; no other sub-section may redefine it.

Decisions:

- **PK type: `TEXT PRIMARY KEY` with prefixed ids.** `avatar_id = "AVT-{uuid_v7}"`, `body_proxy_id = "BPX-{uuid_v7}"`. `UUID PRIMARY KEY DEFAULT gen_random_uuid()` is NOT permitted (off-convention per codebase ground truth).
- **Units in the proxy/measurement authority: MILLIMETRES.** Every field name MUST carry the `_mm` suffix. The LLM-facing `BodyMeasurements` (cm) is converted at the API boundary. Both suffixes (`_mm`, `_cm`) MUST appear on every field so no ambiguity survives.
- **Proxy shape: capsules + sphere sub-proxies.** Capsule-only proxy shapes are NOT permitted because exaggerated-proportion large-bust bodies require sphere sub-proxies. The fixed-size GPU arrays (max 32 capsules, 16 spheres; `bytemuck::Pod`) live in the solver crate; the authority JSONB stores variable-length lists.
- **A body proxy belongs to an avatar, not a garment.** `tailor_body_proxies` MUST NOT carry a `garment_id` FK. The garment references a proxy via `tailor_garments.body_proxy_id`.

```sql
-- tailor_avatars: avatar IDENTITY authority.
-- This is the table that was referenced but never defined across collision and autofit sub-sections.
CREATE TABLE IF NOT EXISTS tailor_avatars (
    avatar_id                TEXT PRIMARY KEY,          -- "AVT-{uuid_v7}"
    workspace_id             TEXT NOT NULL,
    name                     TEXT NOT NULL,
    source_kind              TEXT NOT NULL
        CHECK (source_kind IN ('smpl','smplx','metahuman','custom_obj','vrm','gltf',
                               'parametric','avatar1_2d_derived','non_humanoid')),
    -- 25 anthropometric measurements in MILLIMETRES (GarmentMeasurements naming convention).
    measurements_mm_json     JSONB NOT NULL DEFAULT '{}'::jsonb,
    -- Optional source mesh artifact ref for proxy rebuild.
    source_mesh_artifact_ref TEXT,
    -- Morph/blend-shape parameters for parametric bodies.
    morph_params_json        JSONB,
    event_ledger_event_id    TEXT NOT NULL,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_avatars_workspace ON tailor_avatars (workspace_id);

-- tailor_body_proxies: solver COLLISION GEOMETRY for an avatar (capsules + spheres + optional SDF).
-- One avatar may have several proxies (standard, multi-sphere large-bust, sdf-fallback).
-- MUST NOT carry a garment_id FK; the garment references the proxy, not the reverse.
CREATE TABLE IF NOT EXISTS tailor_body_proxies (
    body_proxy_id            TEXT PRIMARY KEY,          -- "BPX-{uuid_v7}"
    avatar_id                TEXT NOT NULL REFERENCES tailor_avatars (avatar_id),
    workspace_id             TEXT NOT NULL,
    -- ClothBodyProxy JSONB: { capsules:[{joint_name,p0_mm,p1_mm,radius_mm}],
    --                         spheres:[{bone,center_mm,radius_mm}], thickness_mm }
    proxy_json               JSONB NOT NULL,
    mode                     TEXT NOT NULL DEFAULT 'capsule'
        CHECK (mode IN ('capsule','capsule_sphere','capsule_sdf','sdf')),
    breast_proxy_mode        TEXT
        CHECK (breast_proxy_mode IS NULL OR
               breast_proxy_mode IN ('standard','multi_sphere','sdf_fallback')),
    sdf_artifact_ref         TEXT,
    lores_mesh_artifact_ref  TEXT,
    joint_hierarchy_json     JSONB,
    collision_thickness_mm   FLOAT NOT NULL DEFAULT 2.5,
    event_ledger_event_id    TEXT NOT NULL,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS ix_tailor_body_proxies_avatar ON tailor_body_proxies (avatar_id);
```

Canonical Rust geometry types (in `tailor-solver/src/body/proxy.rs`; all lengths in mm):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClothBodyProxy {
    pub body_proxy_id: String,     // "BPX-{uuid_v7}"
    pub avatar_id: String,         // "AVT-{uuid_v7}"
    /// Capsule chain (body segments). All lengths/radii in MILLIMETRES.
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

The GPU upload types `GpuCapsule`/`GpuSphere` (fixed max 32 capsules + 16 spheres, `bytemuck::Pod`) are the runtime representation in the solver crate. The authority `ClothBodyProxy` MUST be serialized to/from them.

---

##### ONE Canonical KernelEventType Additions List

All Tailor `KernelEventType` variants MUST follow this list. Variant names MUST be `Tailor*` PascalCase. Wire strings MUST be `TAILOR_*` SCREAMING_SNAKE_CASE via `as_str()`. Every variant MUST be registered in `required_first_slice_events()`. Wire strings that drop the `TAILOR_` prefix (e.g. `"GARMENT_PATTERN_PROMOTED"`) are NOT permitted.

Lifecycle verbs are normalized: `…Requested`, `…Started`, `…Completed`, `…Rejected` for runs; `…Recorded` for validation; `…Promoted` / `…PromotionRejected` for promotion. CRDT events use the specific per-sub-tree form (`TailorPanelCrdtUpdateRecorded`) because panels, animation, and texture have distinct CRDT sub-trees.

```rust
// === Canonical Tailor KernelEventType additions (src/kernel/mod.rs) ===
// Format: Variant,  // "WIRE_STRING"

// -- Garment lifecycle --
TailorGarmentDraftProposed,        // "TAILOR_GARMENT_DRAFT_PROPOSED"
TailorGarmentDraftUpdated,         // "TAILOR_GARMENT_DRAFT_UPDATED"
TailorGarmentValidationRecorded,   // "TAILOR_GARMENT_VALIDATION_RECORDED"
TailorGarmentPromoted,             // "TAILOR_GARMENT_PROMOTED"
TailorGarmentPromotionRejected,    // "TAILOR_GARMENT_PROMOTION_REJECTED"

// -- Simulation run lifecycle (XPBD solver sandbox) --
TailorSimRunRequested,             // "TAILOR_SIM_RUN_REQUESTED"
TailorSimRunStarted,               // "TAILOR_SIM_RUN_STARTED"
TailorSimRunCompleted,             // "TAILOR_SIM_RUN_COMPLETED"
TailorSimRunRejected,              // "TAILOR_SIM_RUN_REJECTED"

// -- CRDT collaborative editing (one event per sub-tree) --
TailorPanelCrdtUpdateRecorded,     // "TAILOR_PANEL_CRDT_UPDATE_RECORDED"
TailorPanelCrdtSnapshotRecorded,   // "TAILOR_PANEL_CRDT_SNAPSHOT_RECORDED"
TailorPanelAiEditProposalRecorded, // "TAILOR_PANEL_AI_EDIT_PROPOSAL_RECORDED"
TailorPanelAiEditProposalDecided,  // "TAILOR_PANEL_AI_EDIT_PROPOSAL_DECIDED"
TailorCrdtConflictDetected,        // "TAILOR_CRDT_CONFLICT_DETECTED"

// -- Material / fabric presets --
TailorMaterialPresetRecorded,      // "TAILOR_MATERIAL_PRESET_RECORDED"
TailorMaterialPresetUpdated,       // "TAILOR_MATERIAL_PRESET_UPDATED"
TailorMaterialPresetRejected,      // "TAILOR_MATERIAL_PRESET_REJECTED"
TailorGarmentMaterialAssigned,     // "TAILOR_GARMENT_MATERIAL_ASSIGNED"

// -- Avatar / body proxy --
TailorAvatarCreated,               // "TAILOR_AVATAR_CREATED"
TailorAvatarMeasurementsExtracted, // "TAILOR_AVATAR_MEASUREMENTS_EXTRACTED"
TailorBodyProxyCreated,            // "TAILOR_BODY_PROXY_CREATED"
TailorBodyProxyUpdated,            // "TAILOR_BODY_PROXY_UPDATED"

// -- Refit / retargeting --
TailorRefitRequested,              // "TAILOR_REFIT_REQUESTED"
TailorRefitPatternScaled,          // "TAILOR_REFIT_PATTERN_SCALED"
TailorRefitDrapeCompleted,         // "TAILOR_REFIT_DRAPE_COMPLETED"
TailorRefitUvRecomputed,           // "TAILOR_REFIT_UV_RECOMPUTED"
TailorRefitPromoted,               // "TAILOR_REFIT_PROMOTED"
TailorRefitRejected,               // "TAILOR_REFIT_REJECTED"

// -- Trims, zippers, lacing --
TailorTrimImported,                // "TAILOR_TRIM_IMPORTED"
TailorTrimPlaced,                  // "TAILOR_TRIM_PLACED"
TailorTrimTackUpdated,             // "TAILOR_TRIM_TACK_UPDATED"
TailorZipperDefined,               // "TAILOR_ZIPPER_DEFINED"
TailorLacingDefined,               // "TAILOR_LACING_DEFINED"
TailorPatternToTrimConverted,      // "TAILOR_PATTERN_TO_TRIM_CONVERTED"
TailorTrimContactViolation,        // "TAILOR_TRIM_CONTACT_VIOLATION"

// -- UV / texture --
TailorUvIslandsPacked,             // "TAILOR_UV_ISLANDS_PACKED"
TailorUvFlattenCompleted,          // "TAILOR_UV_FLATTEN_COMPLETED"
TailorUvFlattenProposed,           // "TAILOR_UV_FLATTEN_PROPOSED"
TailorPbrMaterialCreated,          // "TAILOR_PBR_MATERIAL_CREATED"
TailorPbrMaterialUpdated,          // "TAILOR_PBR_MATERIAL_UPDATED"
TailorPbrMapsGenerated,            // "TAILOR_PBR_MAPS_GENERATED"
TailorGraphicLayerAdded,           // "TAILOR_GRAPHIC_LAYER_ADDED"
TailorGraphicLayerUpdated,         // "TAILOR_GRAPHIC_LAYER_UPDATED"
TailorGraphicLayerRemoved,         // "TAILOR_GRAPHIC_LAYER_REMOVED"
TailorMaterialAssignmentUpdated,   // "TAILOR_MATERIAL_ASSIGNMENT_UPDATED"

// -- Animation timeline --
TailorAnimationDraftCreated,       // "TAILOR_ANIMATION_DRAFT_CREATED"
TailorAnimationDraftUpdated,       // "TAILOR_ANIMATION_DRAFT_UPDATED"
TailorAnimationSimRunRequested,    // "TAILOR_ANIMATION_SIM_RUN_REQUESTED"
TailorAnimationSimRunCompleted,    // "TAILOR_ANIMATION_SIM_RUN_COMPLETED"
TailorAnimationSimRunRejected,     // "TAILOR_ANIMATION_SIM_RUN_REJECTED"
TailorAnimationDraftPromoted,      // "TAILOR_ANIMATION_DRAFT_PROMOTED"

// -- Export --
TailorGarmentExportCompleted,      // "TAILOR_GARMENT_EXPORT_COMPLETED"

// -- Wardrobe grouping --
TailorWardrobeCreated,             // "TAILOR_WARDROBE_CREATED"
TailorWardrobeGarmentAdded,        // "TAILOR_WARDROBE_GARMENT_ADDED"
TailorWardrobeGarmentRemoved,      // "TAILOR_WARDROBE_GARMENT_REMOVED"
```

**Superseded variant names — MUST NOT be used or re-introduced:**

| Superseded name | Canonical replacement |
|---|---|
| `TailorGarmentValidated` (from garment-authoring and cloth-solver sub-sections) | `TailorGarmentValidationRecorded` |
| `TailorPatternValidated` | `TailorGarmentValidationRecorded` |
| `TailorCrdtUpdateRecorded` | `TailorPanelCrdtUpdateRecorded` |
| `TailorGarmentCrdtUpdateRecorded` | `TailorPanelCrdtUpdateRecorded` |
| `BodyProxyCreated` (missing `Tailor` prefix) | `TailorBodyProxyCreated` |
| `BodyProxyMeasurementsExtracted` (missing `Tailor` prefix) | `TailorAvatarMeasurementsExtracted` |
| `TailorMaterialLibraryUpdated` | `TailorMaterialPresetRecorded` / `TailorMaterialPresetUpdated` |
| `TailorDraftScaled` | `TailorRefitPatternScaled` |
| `TailorSimRunRequested` + `TailorSimRunStarted` as two separate "requested" forms | Exactly as listed above (one `Requested`, one `Started`) |

Canonical `event_family` constants MUST be defined in `src/tailor/event_family.rs`:

```rust
// src/tailor/event_family.rs
pub const TAILOR_GARMENT:    &str = "tailor.garment";
pub const TAILOR_SIMULATION: &str = "tailor.simulation";
pub const TAILOR_PANEL_CRDT: &str = "tailor.panel.crdt";
pub const TAILOR_MATERIAL:   &str = "tailor.material";
pub const TAILOR_AVATAR:     &str = "tailor.avatar";
pub const TAILOR_BODY_PROXY: &str = "tailor.body_proxy";
pub const TAILOR_REFIT:      &str = "tailor.refit";
pub const TAILOR_TRIM:       &str = "tailor.trim";
pub const TAILOR_UV:         &str = "tailor.uv";
pub const TAILOR_TEXTURE:    &str = "tailor.texture";
pub const TAILOR_ANIMATION:  &str = "tailor.animation";
pub const TAILOR_WARDROBE:   &str = "tailor.wardrobe";
pub const TAILOR_EXPORT:     &str = "tailor.export";
```

---

##### Schema-ID Constants

Schema IDs MUST use the `hsk.tailor.*` namespace. `hsk.cloth.*` is NOT permitted except for the two solver-crate-internal physics payloads listed below. These constants MUST be defined in `src/tailor/schemas.rs`.

```rust
// src/tailor/schemas.rs
pub const SCHEMA_TAILOR_GARMENT_SPEC_V1:    &str = "hsk.tailor.garment_spec@1";
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

// Allowed hsk.cloth.* exception — solver-crate-internal physics payloads ONLY.
// These MUST NOT be stored as Tailor-domain authority rows.
pub const SCHEMA_CLOTH_SOLVER_REQUEST_V1:   &str = "hsk.cloth.solver_request@1";
pub const SCHEMA_CLOTH_SOLVER_RESULT_V1:    &str = "hsk.cloth.solver_result@1";
```

---

##### Migration-Naming Convention

All Tailor migrations MUST use the dated convention. Numbered `0NNN_*` migrations are NOT permitted for Tailor because the integer sequence is shared across all work packets and cannot be safely reserved.

Required convention:

```text
migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.sql
migrations/<YYYY>_<MM>_<DD>_tailor_<topic>.down.sql   (required reverse pair for every forward migration)
```

Rules:

1. The date MUST be the authoring date of the migration as assigned when the implementing work packet writes it. This spec MUST NOT hard-code a specific date; it specifies the convention. Example placeholder: `2026_MM_DD_tailor_garments.sql`.
2. Every forward migration MUST ship a `.down.sql` reverse pair (following the `2026_05_18_fems_pinned.sql` / `2026_05_18_fems_pinned.down.sql` precedent).
3. Numbered migrations `0151_tailor_*`, `0334_tailor_*`, `0335_tailor_*`, `0336_tailor_*` are superseded and MUST NOT be created. `0334_tailor_garments.sql` in particular MUST NOT be created because `0334_loom_canvas_boards.sql` already occupies that slot.

Required Tailor migration set (one dated migration per concern):

```text
*_tailor_garments.sql
*_tailor_material_presets.sql
*_tailor_avatars.sql
*_tailor_body_proxies.sql
*_tailor_simulation_runs.sql
*_tailor_refit_runs.sql
*_tailor_trims.sql
*_tailor_texture_tables.sql
*_tailor_wardrobe.sql
*_tailor_garments_animation_col.sql   (ALTER TABLE tailor_garments ADD COLUMN animation_json JSONB)
```

---

##### Canonical tailor_* Postgres Table Set

The following 16 tables are the complete and canonical Tailor Postgres authority. All MUST use `TEXT PRIMARY KEY` with the stated id prefixes. Every row MUST carry an `event_ledger_event_id TEXT NOT NULL` FK to `kernel_event_ledger.event_id`. Every INSERT MUST call `guard_authority_write(AuthorityMode::Postgres)` first (the `no_sqlite_tripwire`). SQLite is NOT permitted.

```text
TABLE                        PK prefix     KEY COLUMNS / CONSTRAINTS / NOTES
---------------------------  -----------   -----------------------------------------------------------
tailor_garments              GAR-          workspace_id; name;
                                           status TEXT CHECK (status IN (
                                             'draft','sandbox_pending','simulated',
                                             'validated','promoted','rejected','archived'));
                                           spec_json JSONB (GarmentSpec);
                                           animation_json JSONB NULLABLE (T-ANIMATION: column, not table);
                                           body_proxy_id TEXT;
                                           wardrobe_id TEXT;
                                           promotion_receipt_id TEXT;
                                           event_ledger_event_id TEXT NOT NULL;
                                           created_at TIMESTAMPTZ; updated_at TIMESTAMPTZ

tailor_garment_crdt_docs     composite PK  (garment_id, crdt_document_id);
                                           FK garment_id -> tailor_garments;
                                           crdt_document_id UNIQUE ("CRDT-GAR-{garment_id}")

tailor_material_presets      MAT-          workspace_id; slug UNIQUE per workspace;
                                           compliance_json JSONB (raw anisotropic XPBD);
                                           physics_json JSONB;
                                           is_system_preset BOOL.
                                           CANONICAL NAME — tailor_material_library and
                                           tailor_material are NOT permitted.

tailor_avatars               AVT-          workspace_id; name;
                                           source_kind TEXT CHECK (see body-proxy schema above);
                                           measurements_mm_json JSONB;
                                           source_mesh_artifact_ref TEXT;
                                           morph_params_json JSONB.
                                           Authored in this sub-section; was the undefined FK target.

tailor_body_proxies          BPX-          FK avatar_id -> tailor_avatars;
                                           workspace_id;
                                           proxy_json JSONB (capsules+spheres, mm);
                                           mode CHECK; breast_proxy_mode CHECK.
                                           MUST NOT have a garment_id FK column.

tailor_simulation_runs       SIM-          FK garment_id -> tailor_garments;
                                           FK sandbox_run_id -> kb003_sandbox_runs(run_id);
                                           solver_version; substeps; iterations;
                                           content_hash (idempotency only; NOT used for promotion);
                                           result_artifact_ref.
                                           Id prefix is SIM-. CSIM- is NOT permitted.

tailor_refit_runs            RFT-          FK garment_id;
                                           FK source_body_proxy_id -> tailor_body_proxies;
                                           FK target_body_proxy_id -> tailor_body_proxies;
                                           refit_mode TEXT CHECK;
                                           output_garment_id TEXT

tailor_trims                 TRIM-         workspace_id;
                                           trim_category TEXT CHECK;
                                           mesh_json JSONB; inertia_tensor_json JSONB;
                                           is_library_item BOOL;
                                           converted_from_panel_id TEXT

tailor_trim_placements       PLAC-         FK garment_id; FK trim_id -> tailor_trims;
                                           tacks_json JSONB; initial_pose_json JSONB;
                                           layer_order INT

tailor_zippers               ZIP-          FK garment_id;
                                           panel_edge_a TEXT; panel_edge_b TEXT;
                                           slider_count INT CHECK (slider_count >= 1)

tailor_lacings               LACE-         FK garment_id;
                                           eyelet_sequence_json JSONB

tailor_uv_islands            UVI-          FK garment_id;
                                           simulation_run_id TEXT NULLABLE;
                                           panel_id TEXT;
                                           atlas_uv_min FLOAT[2]; atlas_uv_max FLOAT[2];
                                           flatten_method TEXT CHECK ('arap');
                                           UNIQUE (garment_id, simulation_run_id, panel_id)

tailor_pbr_materials         PBR-          workspace_id;
                                           *_map_ref columns (albedo, roughness, metallic,
                                             normal, ao, emissive, displacement);
                                           grain_angle_deg FLOAT

tailor_graphic_layers        GLYR-         FK garment_id; panel_id TEXT;
                                           z_order INT; blend_mode TEXT;
                                           boundary_pinned BOOL;
                                           deleted_at TIMESTAMPTZ NULLABLE (tombstone)

tailor_material_assignments  ASGN-         FK garment_id;
                                           FK physics_preset_id -> tailor_material_presets;
                                           FK pbr_material_id -> tailor_pbr_materials;
                                           UNIQUE (garment_id, panel_id)

tailor_wardrobe              WRD-          workspace_id; name
```

**Cross-file conflict resolutions that MUST be preserved:**

- The material table is `tailor_material_presets` exclusively. `tailor_material_library` and `tailor_material` are NOT permitted table names. The "material library" concept is the set of `is_system_preset = true` rows in this one table.
- `animation_json` is a NULLABLE JSONB COLUMN on `tailor_garments`, not a separate table.
- The simulation-run id prefix is `SIM-`. `CSIM-` is NOT permitted.
- `tailor_body_proxies` MUST NOT carry a `garment_id` FK column. The avatar is reachable from the garment via the proxy; the proxy does not belong to a specific garment.

---

##### ValidationDescriptor Check Catalog

The following catalog is the single normative set of Tailor validation checks. It MUST be realized as `TailorValidationDescriptor` instances wrapping the KB003 `ValidationDescriptor`. No check from a previous sub-section's scattered lists may be added, renamed, or removed without a spec amendment. Two severities: `Blocking` (any failure prevents promotion) and `Advisory` (recorded; blocks only when `PromotionGateInputs.treat_advisory_as_blocking = true`). Each check carries a stable `code` for model self-correction.

```text
CHECK CODE               SEVERITY  STAGE   ASSERTION

-- Fast pre-solver checks (run during author_garment; < 100 ms; no solver required) --
PANEL_CLOSURE            Blocking  fast    Each panel polygon is a closed, non-self-intersecting loop.
SEAM_EDGE_REF            Blocking  fast    Every SeamSpec.from/to references a valid panel_id + edge_index.
GATHER_RATIO_RANGE       Blocking  fast    Every SeamSpec.gather_ratio in (0.0, 20.0].
FABRIC_RANGE             Blocking  fast    Normalized FabricProperties fields in [0.0, 1.0];
                                           density_g_m2 in [5, 2000]; collision_thickness_mm in [0.1, 5].
AVATAR_BINDING           Blocking  fast    AvatarBinding.avatar_id exists in tailor_avatars.
MIN_PANEL_AREA           Blocking  fast    Every panel area > 1.0 cm^2.
WINDING                  Advisory  fast    Panel vertices counter-clockwise (auto-corrected; INFO if fixed).

-- Mesh-quality checks (run on triangulated SolverMesh; pre-simulation) --
MESH_TOPOLOGY            Blocking  mesh    Manifold; no degenerate triangles; no open boundary except
                                           intended seam edges.
MESH_TRIANGLE_QUALITY    Blocking  mesh    Min triangle angle >= 10 deg; max aspect ratio <= 20.
PANEL_OVERLAP            Advisory  mesh    No two panels occupy the same 3D region before draping.

-- Post-simulation cloth checks --
MESH_NOT_EMPTY           Blocking  post    Simulated vertex buffer non-empty.
NO_DEGENERATE_TRIS       Blocking  post    No zero-area triangles in output mesh.
SEAMS_CLOSED             Blocking  post    Every seam constraint pair <= 1 mm separation at rest.
NO_INTERPENETRATION      Blocking  post    No cloth particle deeper than -0.5 mm inside any body
                                           capsule/sphere (final frame only).
SELF_INTERSECTION        Advisory  post    Self-collision pair count below mesh-explosion limit.
UV_COVERAGE              Blocking  post    UV islands cover >= 95% of mesh surface.
UV_VALIDITY              Blocking  post    All UVs in [0,1]^2; no degenerate UV triangle (area > 1e-6).
DRAPE_CONVERGED          Advisory  post    Final kinetic energy below convergence threshold.
PANEL_COUNT_MATCH        Advisory  post    Simulated panel count == spec panel count.
GARMENTCODE_ROUNDTRIP    Advisory  post    Spec round-trips to GarmentCode JSON without loss.

-- Multi-layer checks (when GarmentSpec has layered garments) --
INTERLAYER_SPACING       Blocking  post    No inter-layer pair closer than (t_inner + t_outer - tolerance).

-- Trim checks (when trim_placements present) --
TRIM_NO_PENETRATION      Blocking  post    No trim mesh triangle interpenetrates a cloth triangle (final frame).
TACK_SEAM_CLOSURE        Blocking  post    All tack distances <= 5 mm at end of draping.
ZIPPER_TOOTH_ALIGN       Blocking  post    Tooth-rail tacks within 1 mm of their panel edge positions.
LACING_CORD_LENGTH       Blocking  post    No cord segment stretched beyond 200% rest length.
TRIM_GRAVITY_STABLE      Advisory  post    No trim body translating > 50 mm/frame in final 10 frames.
TACK_STRENGTH_NONZERO    Advisory  post    Warn if any tack strength < 0.01.

-- Material-preset checks (model-authored preset drape test) --
PRESET_NO_NAN            Blocking  preset  No NaN/Inf in drape-test particle positions.
PRESET_STRETCH_NONZERO   Blocking  preset  Stretch compliance != 0 (zero diverges the solver).
PRESET_DENSITY_POS       Blocking  preset  Density > 0.
PRESET_BBOX_PLAUSIBLE    Advisory  preset  Drape-test bbox within expected range for the claimed archetype.

-- Refit checks --
REFIT_INTERSECTION_FREE  Blocking  post    min(particle-capsule distance) >= -0.5 mm after refit.
REFIT_SEAM_CLOSURE       Blocking  post    Mated seam edge-pair length diff < 1%.
REFIT_CONVERGED          Advisory  post    Refit sim reached equilibrium.
```

`ValidationReport::aggregate_blocks_promotion()` decides promotion: any `Blocking` failure MUST cause rejection. The `TailorValidationDescriptor` MUST select the applicable check subset by stage and by which optional features (trims, multi-layer, refit) the garment uses. This catalog supersedes the scattered lists in the garment-authoring, collision, fabric-models, autofit, kernel-integration, and trim-rigid sub-sections.

---

##### Determinism vs Promotion-Equivalence (MeshComparator)

`SolverResult.content_hash` (SHA-256 of the final position buffer) MUST NOT be used as the promotion-equivalence gate. Cross-backend float rounding differs because WGSL/Naga do not guarantee sub-expression ordering and WGSL has no f64 (confirmed: wgpu issue #5329). Identical-quality drapes on different GPU backends produce different hashes; gating on hash equality causes spurious promotion failures.

The following two-mechanism split MUST be implemented:

```text
content_hash  (SHA-256 of final position bytes; stored in SolverResult and tailor_simulation_runs)
  PURPOSE:    Same-machine, same-backend idempotency ONLY; EventLedger receipt fingerprint.
  USE:        Deduplicate identical re-submissions on one machine/backend; record in the run receipt.
  MUST NOT:   Be used for cross-backend promotion equivalence.

MeshComparator  (tolerance-based; the canonical promotion-equivalence check)
  LOCATION:   tailor-solver/src/compare.rs (pure function; no external dep; reused by kernel
              validation runner via the ClothSolver trait boundary)

  PRIMARY (continuous, epsilon-tolerant):
    per_vertex_position_deviation <= epsilon_mm   (default: epsilon_mm = 0.1)
    Compared vertex-for-vertex in canonical vertex order.
    Vertex ordering MUST be deterministic from mesh topology + constraint coloring,
    computed once at garment load and stored (topology is stable cross-backend).
    Metric: max per-vertex Euclidean deviation AND mean deviation, both reported.

  SECONDARY (exact topology invariants — MUST match exactly):
    vertex_count          == expected
    triangle_count        == expected
    seam_edge_pair_count  == expected
    panel_count           == expected

  VERDICT:    Meshes are "equivalent for promotion" if and only if all SECONDARY invariants
              match exactly AND max per-vertex deviation <= epsilon_mm.
```

`epsilon_mm` MUST be a field on `SimRunParams` / the validation policy (default `0.1`) so the operator can tighten it for hero assets or loosen it for exploratory runs.

For **animated runs**: wind turbulence uses `sin()`/`fract()` whose cross-vendor precision differs. For animated runs the comparator MUST additionally accept a shape-envelope match — per-frame bounding box within `bbox_epsilon_mm` (default `1.0`) plus `SEAMS_CLOSED` — as the equivalence basis, because exact per-vertex reproduction across vendors is neither achievable nor required for aesthetic turbulence.

The `TailorValidationDescriptor` re-run-determinism step MUST call `MeshComparator::compare(a, b, epsilon_mm)`, NOT `a.content_hash == b.content_hash`. Every other sub-section that references "content hash comparison" for promotion MUST defer to this resolution.

---

##### Canonical Model Feedback Type

`SimulationReceipt` (schema id `hsk.tailor.simulation_receipt@1`) is the single canonical model feedback type, returned as MCP `structuredContent`. Its `validation_findings: Vec<ValidationFinding>` MUST carry:

- `code`: one of the stable codes from [ValidationDescriptor Check Catalog]
- `severity`: `"blocking" | "advisory" | "info"`
- `affected_id` (optional): panel_id, seam_id, or trim_id
- `suggested_fix` (optional): `{ field_path: String, suggested_value: serde_json::Value }` using a JSON-pointer path into `GarmentSpec`
- `recommended_action`: one of `promote_garment | edit_and_resimulate | correct_spec_first | requires_operator_action`

The MCP tool surface (`author_garment`, `simulate_garment`, `edit_garment`, `promote_garment`, `get_garment`, `estimate_fabric_params`, plus the trim/UV/animation tools) is defined in the model-first-API, trim-rigid, UV-texture, and animation sub-sections; only the receipt and finding types are canonicalized here.

---

##### Contract-Deferral Rules for Implementation Work Packets

When an implementation Work Packet consumes the Tailor spec bundle, it MUST resolve contract surfaces as follows:

| Contract surface | Authority location in this spec |
|---|---|
| Type names, field names, units | [ONE Canonical GarmentSpec], [ONE Canonical Body-Proxy / Avatar Authority Schema] |
| `KernelEventType` variants, wire strings, `event_family` constants | [ONE Canonical KernelEventType Additions List] |
| Schema-ID constants | [Schema-ID Constants] |
| Migration naming and file set | [Migration-Naming Convention] |
| Tables, columns, PK form, id prefixes | [Canonical tailor_* Postgres Table Set] |
| Validation checks and severities | [ValidationDescriptor Check Catalog] |
| Promotion equivalence | [Determinism vs Promotion-Equivalence] |

Algorithms, GPU/WGSL design, OSS adaptation notes, MCP tool behaviour details, and risk analysis remain owned by their respective Tailor sub-sections. This sub-section owns only the contract surfaces.

## 13.15 Validation, Promotion Equivalence & HBR

> **Heading placeholder.** KERNEL_BUILDER will assign the final section number on assembly.
> Sub-section id: `validation`. Source research: T-CONTRACTS [T-CONTRACTS.validation],
> [T-CONTRACTS.determinism]; T-KERNEL-INTEGRATION [T-KERNEL-INTEGRATION.validation],
> [T-KERNEL-INTEGRATION.promotion]. The research package is non-normative provenance; the
> contracts below are the normative authority.

---

##### 15-validation-hbr.1  Scope

This sub-section specifies:

1. The `TailorValidationDescriptor` check catalog — the complete staged gate every garment, material
   preset, and refit run MUST pass before promotion.
2. The `MeshComparator` promotion-equivalence contract — the tolerance-based comparator that
   MUST replace exact hash comparison for cross-backend reproducibility checks.
3. The HBR (Harness Behavior Requirements) matrix — the mandatory `INT / SWARM / VIS / QUIET /
   MAN / STOP` obligations that apply to all Tailor validation, simulation, and promotion work.

No requirements in this sub-section duplicate authority that lives in the KB003 kernel
(`ValidationDescriptor`, `ValidationReport`, `PromotionGate`, `PromotionGateInputs`,
`SandboxAdapter`). Tailor MUST reuse those kernel types unchanged. This sub-section extends them
only where Tailor-domain specifics are required.

---

##### 15-validation-hbr.2  Canonical Check Catalog

###### 15-validation-hbr.2.1  TailorValidationDescriptor

The Tailor validation gate MUST be implemented as `TailorValidationDescriptor`, a domain wrapper
around the kernel `ValidationDescriptor` type located at
`src/backend/handshake_core/src/kernel/validation/descriptor.rs`. Every check defined in this
section MUST be registered in `TailorValidationDescriptor`.

`TailorValidationDescriptor` MUST select the applicable check subset at runtime based on:

- **stage** — which checks apply to the current pipeline position (fast / mesh / post / preset /
  refit).
- **feature flags** — whether the `GarmentSpec` contains trim placements, multi-layer stacks, or
  animated sequences.

`TailorValidationDescriptor` MUST NOT apply post-simulation checks before a simulation run has
completed, and MUST NOT apply trim checks when `GarmentSpec.trim_placements` is empty.

###### 15-validation-hbr.2.2  Two Severities Only

Each check MUST carry exactly one of two severities: **`Blocking`** or **`Advisory`**.

- A `Blocking` failure MUST prevent promotion. `ValidationReport::aggregate_blocks_promotion()`
  MUST return `true` if any `Blocking` check failed.
- An `Advisory` failure MUST be recorded in the `ValidationReport` and surfaced to the model via
  `SimulationReceipt.validation_findings` (schema id `hsk.tailor.simulation_receipt@1`), but MUST
  NOT prevent promotion unless `PromotionGateInputs.treat_advisory_as_blocking` is `true`.

###### 15-validation-hbr.2.3  Check Code Contract

Every check MUST carry a stable string `code` (the `ValidationFinding.code` field in
`SimulationReceipt`). The model uses these codes for self-correction targeting. Codes MUST NOT be
renamed once assigned. The canonical codes are defined in the catalog below. Implementations MUST
NOT introduce synonymous codes for the same check.

###### 15-validation-hbr.2.4  The Full Canonical Check Catalog

The catalog below is normative. The column order is: `CODE | SEVERITY | STAGE | ASSERTION`.

Stage values: `fast` (pre-solver, author time, < 100 ms), `mesh` (triangulated `SolverMesh`,
pre-simulation), `post` (post-simulation output), `preset` (material-preset drape test),
`refit` (refit run output).

```text
-- Fast pre-solver checks (stage: fast) ----------------------------------------
PANEL_CLOSURE          Blocking  fast    Each panel polygon is a closed, non-self-intersecting
                                         loop. All vertex-index references within edges are
                                         in-bounds for the panel's vertices_cm array.
SEAM_EDGE_REF          Blocking  fast    Every SeamSpec.from and SeamSpec.to references a valid
                                         panel_id that exists in GarmentSpec.panels, and an
                                         edge_index that is in-bounds for that panel's edges array.
GATHER_RATIO_RANGE     Blocking  fast    Every SeamSpec.gather_ratio is in the open-closed range
                                         (0.0, 20.0]. Zero and negative values MUST fail.
FABRIC_RANGE           Blocking  fast    All normalized FabricProperties fields
                                         (stretch_weft, stretch_warp, shear, bending_weft,
                                         bending_warp, buckling_ratio, friction, internal_damping)
                                         are in [0.0, 1.0]. density_g_m2 is in [5.0, 2000.0].
                                         collision_thickness_mm is in [0.1, 5.0].
AVATAR_BINDING         Blocking  fast    AvatarBinding.avatar_id exists as a row in
                                         tailor_avatars with a matching workspace_id.
MIN_PANEL_AREA         Blocking  fast    Every panel's computed 2D area (from vertices_cm,
                                         Shoelace formula) is > 1.0 cm^2. Rejects degenerate
                                         or collapsed panels.
WINDING                Advisory  fast    Panel vertices_cm are counter-clockwise. If clockwise,
                                         the implementation SHOULD auto-correct the ordering and
                                         emit severity INFO in the finding; it MUST NOT block.

-- Mesh-quality checks (stage: mesh) --------------------------------------------
MESH_TOPOLOGY          Blocking  mesh    Triangulated SolverMesh is manifold. No degenerate
                                         triangles (zero-area). No open boundary edges except
                                         those corresponding to intended seam edges in the spec.
MESH_TRIANGLE_QUALITY  Blocking  mesh    Every triangle in the SolverMesh satisfies:
                                         minimum interior angle >= 10 degrees AND
                                         aspect ratio (longest/shortest edge) <= 20.
PANEL_OVERLAP          Advisory  mesh    No two panel meshes occupy the same 3D region in their
                                         initial draping placement (before simulation). Measured
                                         by AABB + narrow-phase triangle intersection test.

-- Post-simulation cloth checks (stage: post) -----------------------------------
MESH_NOT_EMPTY         Blocking  post    The simulated vertex buffer returned by the solver is
                                         non-empty (vertex count > 0).
NO_DEGENERATE_TRIS     Blocking  post    No zero-area triangles exist in the post-simulation
                                         output mesh.
SEAMS_CLOSED           Blocking  post    Every seam constraint pair has a Euclidean separation
                                         <= 1.0 mm in the final simulation frame.
NO_INTERPENETRATION    Blocking  post    No cloth particle is deeper than -0.5 mm inside any
                                         body capsule or sphere of the bound ClothBodyProxy,
                                         measured in the final simulation frame only.
                                         Intermediate substep penetrations are not checked here.
SELF_INTERSECTION      Advisory  post    The self-collision pair count is below the
                                         mesh-explosion limit defined in SimRunParams.
UV_COVERAGE            Blocking  post    UV islands cover >= 95% of the mesh surface area.
                                         Measures pattern accuracy: the simulated cloth panels
                                         must map back to the 2D spec without gaps.
UV_VALIDITY            Blocking  post    All UV coordinates are in [0.0, 1.0]^2. No degenerate
                                         UV triangles (UV area > 1e-6 per triangle).
DRAPE_CONVERGED        Advisory  post    The final kinetic energy of the simulation is below
                                         the convergence threshold defined in SimRunParams.
                                         Indicates solver convergence, not merely termination.
PANEL_COUNT_MATCH      Advisory  post    The number of distinct panel meshes in the simulated
                                         output equals GarmentSpec.panels.len().
GARMENTCODE_ROUNDTRIP  Advisory  post    GarmentSpec serializes to GarmentCode-compatible JSON
                                         and round-trips back without loss of panel topology,
                                         seam references, or gather_ratio values.

-- Multi-layer checks (stage: post; applies only when GarmentSpec has stacked layers) --
INTERLAYER_SPACING     Blocking  post    No inter-layer cloth particle pair is closer than
                                         (t_inner + t_outer - tolerance_mm), where t_inner and
                                         t_outer are the collision_thickness_mm values of the
                                         two layers and tolerance_mm is from SimRunParams.

-- Trim checks (stage: post; applies only when GarmentSpec.trim_placements is non-empty) --
TRIM_NO_PENETRATION    Blocking  post    No trim mesh triangle interpenetrates a cloth triangle
                                         in the final simulation frame.
TACK_SEAM_CLOSURE      Blocking  post    All tack distances are <= 5.0 mm at the end of draping.
ZIPPER_TOOTH_ALIGN     Blocking  post    Every zipper tooth-rail tack is within 1.0 mm of its
                                         assigned panel edge position.
LACING_CORD_LENGTH     Blocking  post    No lacing cord segment is stretched beyond 200% of its
                                         rest length.
TRIM_GRAVITY_STABLE    Advisory  post    No trim rigid body translates more than 50.0 mm/frame
                                         in the final 10 simulation frames.
TACK_STRENGTH_NONZERO  Advisory  post    Warn if any tack stiffness value is < 0.01.
                                         Indicates a possibly unintentionally loose tack.

-- Material-preset checks (stage: preset; applies to model-authored material drape tests) --
PRESET_NO_NAN          Blocking  preset  No NaN or Inf values appear in any drape-test particle
                                         position after the preset validation drape run.
PRESET_STRETCH_NONZERO Blocking  preset  Stretch compliance (weft and warp) is != 0. A zero
                                         stretch compliance diverges the XPBD solver.
PRESET_DENSITY_POS     Blocking  preset  Particle density (derived from density_g_m2) is > 0.
PRESET_BBOX_PLAUSIBLE  Advisory  preset  The drape-test bounding box is within the expected range
                                         for the material archetype claimed in FabricPreset.

-- Refit checks (stage: refit; applies after T-AUTOFIT refit runs) ---------------
REFIT_INTERSECTION_FREE Blocking refit   Minimum particle-to-capsule distance >= -0.5 mm after
                                          refit simulation completes.
REFIT_SEAM_CLOSURE      Blocking refit   Mated seam edge-pair length difference < 1% of the
                                          shorter edge length.
REFIT_CONVERGED         Advisory refit   The refit simulation reached equilibrium (did not
                                          time out before the convergence threshold).
```

Total: 35 checks (19 Blocking, 16 Advisory, across 5 stages).

###### 15-validation-hbr.2.5  ValidationFinding and SimulationReceipt

The model's feedback type is `SimulationReceipt` (schema id `hsk.tailor.simulation_receipt@1`),
returned as MCP `structuredContent`. It MUST carry a
`validation_findings: Vec<ValidationFinding>` field. Each `ValidationFinding` MUST include:

- `code: String` — one of the canonical codes from [15-validation-hbr.2.4].
- `severity: String` — `"blocking"` | `"advisory"` | `"info"`.
- `affected_id: Option<String>` — the panel_id, seam_id, or trim placement_id implicated, when
  attributable.
- `suggested_fix: Option<FixHint>` — where `FixHint` carries a `field_path: String` (JSON Pointer
  into `GarmentSpec`) and a `suggested_value: serde_json::Value`.
- `recommended_action: String` — one of `"promote_garment"` | `"edit_and_resimulate"` |
  `"correct_spec_first"` | `"requires_operator_action"`.

The model MUST use `ValidationFinding.code` to identify which spec field to correct before
re-emitting a `GarmentSpec`. The implementation MUST NOT surface anonymous or code-free findings
to the model.

###### 15-validation-hbr.2.6  TailorGarmentValidationRecorded Event

When `TailorValidationDescriptor` completes a validation run, the kernel MUST emit a
`TailorGarmentValidationRecorded` event (wire string: `"TAILOR_GARMENT_VALIDATION_RECORDED"`)
with `event_family` `"tailor.garment"`. The event payload MUST include the `garment_id`, the
`sim_run_id`, a summary of `blocking_failures: u32` and `advisory_failures: u32`, and the
`validation_run_id`. The full `ValidationReport` is stored server-side; only the summary travels
in the EventLedger payload.

The superseded variant names `TailorGarmentValidated` (files 03/04) and
`TailorPatternValidated` (file 01) MUST NOT be used. `TailorGarmentValidationRecorded` is
the sole canonical variant.

---

##### 15-validation-hbr.3  Promotion Equivalence: MeshComparator

###### 15-validation-hbr.3.1  The Hash-Equality Gap

The XPBD solver's output is deterministic per-backend (same GPU backend + driver version ⇒
identical float result), but is NOT deterministic cross-backend. WGSL/Naga sub-expression
ordering is unspecified and WGSL has no f64 (confirmed by wgpu issue #5329). An exact SHA-256
comparison of the final position buffer across backends or driver versions will produce spurious
promotion failures for identically-quality drapes. The PromotionGate MUST NOT gate on
`content_hash` equality for cross-backend promotion equivalence.

###### 15-validation-hbr.3.2  content_hash: Retained for Idempotency Only

`SolverResult.content_hash: [u8; 32]` (SHA-256 of the final position buffer) is retained for
two purposes only:

1. Same-machine, same-run deduplication of re-submitted solver requests.
2. EventLedger receipt fingerprinting (the `tailor_simulation_runs` row and the
   `TailorSimRunCompleted` event payload MAY carry it for audit purposes).

The `PromotionGate` and `TailorValidationDescriptor` MUST NOT compare `content_hash` values
between a re-run and the original run to determine equivalence.

###### 15-validation-hbr.3.3  MeshComparator: The Canonical Promotion-Equivalence Check

`MeshComparator` is the sole canonical mechanism for determining whether two simulation results
are equivalent for promotion purposes. It MUST be implemented as a pure function in
`tailor-solver/src/compare.rs` with no `handshake_core` dependencies. The kernel validation
runner accesses it via the `ClothSolver` trait boundary.

`MeshComparator` MUST apply two components:

**Primary (continuous, epsilon-tolerant):**

The per-vertex position deviation between result A and result B MUST be computed vertex-for-vertex
in the canonical vertex order. The canonical vertex order is determined once at garment load from
mesh topology and constraint coloring; it MUST be stable across backends and MUST be stored with
the simulation run record so a re-run can use the same order.

The comparator MUST compute:
- `max_deviation_mm`: the maximum per-vertex Euclidean deviation across all vertices.
- `mean_deviation_mm`: the mean per-vertex Euclidean deviation across all vertices.

Both MUST be reported in the `ValidationReport` and in `SimulationReceipt`.

The primary component passes if and only if `max_deviation_mm <= epsilon_mm`.

`epsilon_mm` defaults to `0.1` (0.1 mm). It MUST be a configurable field on `SimRunParams`
so the operator can tighten it for hero assets or loosen it for exploratory runs.

**Secondary (exact, topology invariants):**

The following counts MUST match exactly between the two results:
- `vertex_count`
- `triangle_count`
- `seam_edge_pair_count`
- `panel_count`

**Promotion-Equivalence Verdict:**

Two simulation results are "equivalent for promotion" if and only if all four secondary
invariants match exactly AND `max_deviation_mm <= epsilon_mm`. The `MeshComparator` MUST return
a typed verdict that names which component failed when the comparison does not pass.

###### 15-validation-hbr.3.4  Animated Run Exception

For animated garment runs (when `GarmentSpec` carries an animation sequence via the
`animation_json` column), wind turbulence and time-varying forces use `sin()`/`fract()` whose
cross-vendor precision differs enough that per-vertex deviation can exceed even a loose epsilon.

For animated runs, `MeshComparator` MUST additionally accept a **shape-envelope** match as the
equivalence basis when the per-vertex primary check fails:

- Per-frame bounding box deviation within `bbox_epsilon_mm` (default `1.0` mm).
- `SEAMS_CLOSED` check passes in the final frame.

The shape-envelope match is the equivalence basis for animated runs because the turbulence is
aesthetic and exact per-vertex reproduction is neither required nor achievable cross-vendor. The
`MeshComparator` MUST record in the verdict whether the primary or shape-envelope path was used.

###### 15-validation-hbr.3.5  Migration Note

All prior references to "content hash comparison" for promotion equivalence in the research
package (files 03, 04, 07, 10) are superseded by [15-validation-hbr.3.3]. No implementation work
packet MAY implement hash-equality promotion gates.

---

##### 15-validation-hbr.4  HBR Matrix: Harness Behavior Requirements for Tailor Work

The HBR matrix defines the six mandatory harness behaviors that apply during all Tailor
validation, simulation, and promotion work. These requirements apply to every model lane,
`ModelAdapter` call, sandbox adapter run, and validation runner invocation in the Tailor domain.

The six behaviors are: `INT` (Interrupt), `SWARM` (Swarm coordination), `VIS` (Visibility),
`QUIET` (Non-intrusion), `MAN` (Manual override), `STOP` (Hard stop). Each cell below states
the obligation for Tailor-domain work.

```text
BEHAVIOR  OBLIGATION FOR TAILOR WORK
--------  -----------------------------------------------------------------------
INT       A running TailorSandboxAdapter sim run MUST be interruptible by an
          operator STOP command. The ClothSolver trait MUST expose a cancel()
          signal (or equivalent async abort). The TailorSandboxAdapter.run()
          implementation MUST poll the cancel signal at every substep boundary
          and MUST return AdapterRunOutcome::Cancelled (or equivalent) immediately
          on receipt. The EventLedger MUST emit TailorSimRunRejected with reason
          "operator_cancelled" in response. No partial mesh artifact MUST be
          promoted from a cancelled run.

SWARM     When multiple model lanes author garments concurrently for the same
          workspace, each lane MUST operate on a separate garment_id. Concurrent
          CRDT edits to the SAME garment MUST go through the yjs_bridge or
          ai_edit_proposal state machine as defined in the CRDT sub-section.
          No two model lanes MAY submit simultaneous promotion requests for the
          same garment_id + sim_run_id pair; the idempotency_key
          "CPROM-{garment_id}-{val_run_id}" enforces this at the EventLedger
          INSERT level.

VIS       Every stage of the garment lifecycle MUST emit an EventLedger event
          with a typed KernelEventType variant:
            TailorGarmentDraftProposed      — draft created
            TailorSimRunRequested           — sim enqueued
            TailorSimRunStarted             — sim executing
            TailorSimRunCompleted           — sim output available
            TailorGarmentValidationRecorded — validation finished
            TailorGarmentPromoted           — authority row written
            TailorGarmentPromotionRejected  — gate rejected
          The tailor_garments.status column MUST reflect the current lifecycle
          stage at all times (draft | sandbox_pending | simulated | validated |
          promoted | rejected | archived). No silent state transitions are
          permitted. ValidationReport findings MUST be accessible to the
          operator via the API before the promotion decision is made.

QUIET     The TailorSandboxAdapter and tailor-solver crate MUST NOT:
            - Pop foreground windows or steal focus.
            - Open interactive UI dialogs.
            - Write to stdout/stderr outside of structured log channels.
            - Acquire GPU resources that would visibly compete with operator
              ComfyUI or other GPU workloads during an unattended run.
          The solver MUST write artifacts only to the sandbox workspace scratch
          path supplied by the SandboxWorkspaceV1 materializer.
          Log output MUST be structured (tracing::info!/warn!/error! crate) and
          MUST be bounded — no per-substep log lines in production mode.

MAN       The PromotionGate MUST require non-fixture OperatorApprovalEvidence
          before completing a TailorGarmentPromoted transition. The Tauri
          frontend MUST provide a garment review panel that issues real
          review_receipt_ids. Automated self-approval (fixture evidence, mocked
          approval) is architecturally blocked by OperatorApprovalEvidence
          .looks_fixture() check in the kernel gate. Model lanes MUST NOT
          construct fixture-looking approval evidence.
          EXCEPTION: unit tests and integration tests may use
          OperatorApprovalEvidence::test_fixture() exclusively in
          #[cfg(test)] context; this MUST NOT be reachable in production builds.

STOP      An operator STOP command (issued via API or Tauri command) MUST:
            1. Cancel any in-progress TailorSandboxAdapter sim run (see INT).
            2. Cancel any pending TailorValidationDescriptor run for the same
               garment_id.
            3. Leave tailor_garments.status as the last stable committed value
               (draft or simulated); MUST NOT advance status on a stopped run.
            4. Emit TailorSimRunRejected or TailorGarmentPromotionRejected with
               reason "operator_stop" in the EventLedger.
            5. Release any model lane lease held for the stopped garment_id.
          The STOP obligation is unconditional: background goal continuations,
          persistent adapter retry loops, or automatic re-submission MUST NOT
          resume Tailor work for the stopped garment until the operator
          explicitly re-triggers the simulation or promotion.
```

---

##### 15-validation-hbr.5  PostgreSQL and EventLedger Enforcement

Every Tailor validation and promotion write MUST call
`guard_authority_write(AuthorityMode::PostgresPrimary)` (from
`kernel/sandbox/no_sqlite_tripwire.rs`) at the top of the storage function, consistent with
`kb003_storage.rs` usage. SQLite MUST NOT be used for any Tailor authority row at any stage.

The `TailorGarmentValidationRecorded` and `TailorGarmentPromoted` / `TailorGarmentPromotionRejected`
events MUST use the idempotency key patterns below to prevent duplicate authority writes:

```rust
// Validation run idempotency key
format!("TVAL-{garment_id}-{sim_run_id}")

// Promotion idempotency key  (matches T-KERNEL-INTEGRATION binding)
format!("CPROM-{garment_id}-{val_run_id}")
```

---

##### 15-validation-hbr.6  Migration Naming for Validation Infrastructure

There are no dedicated validation tables — validation runs use the existing kernel
`ValidationDescriptor` / `ValidationReport` infrastructure. The `tailor_simulation_runs` table
(migration `2026_MM_DD_tailor_simulation_runs.sql`, dated at authoring time per the
[T-CONTRACTS.migration-naming] convention) carries the sim-run record and the FK to
`kb003_sandbox_runs.run_id`. This is the only Tailor-domain migration required to support the
validation gate.

Numbered `0NNN_*` migration names MUST NOT be used for Tailor migrations (the integer space is
contested by parallel kernel work packets; `0334_loom_canvas_boards.sql` proves the collision
risk). Every Tailor migration MUST use the dated convention with a `.down.sql` reverse pair.

---

##### 15-validation-hbr.7  Non-Normative Provenance

The check catalog in [15-validation-hbr.2.4] consolidates scattered check lists from research
files 03, 05, 06, 07, 10, and 13. The severity assignments and stage assignments in those files
are superseded where they conflict with the catalog above. The `MeshComparator` resolution in
[15-validation-hbr.3] resolves `KI-DETERMINISM-VS-PROMOTION` as documented in T-CONTRACTS
[T-CONTRACTS.determinism]. Both the research files and the known-issues index are non-normative
provenance; this sub-section is the normative authority for all validation, promotion-equivalence,
and HBR requirements in the Tailor module.

