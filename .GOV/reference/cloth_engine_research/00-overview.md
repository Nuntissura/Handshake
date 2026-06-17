---
file_id: cloth-engine-overview
topic_id: T-OVERVIEW
title: "Tailor Engine Research: Overview and Scope"
status: draft
depends_on: []
summary: "Frames the Handshake-native, model-first, MD-equivalent Tailor creative module on the kernel: vision, constraints, differentiator, OSS landscape anchor, and build-order note."
sources: 18
updated_at: "2026-06-17"
---

## [T-OVERVIEW] Overview and Scope

### [T-OVERVIEW.vision] Vision

The goal is a fully-featured, model-steerable garment detailer built natively inside Handshake — a creative module (Tailor) that attaches to the Handshake kernel the same way the atelier module does, without requiring any side app, third-party solver subscription, or platform-locked pipeline.

The target feature ceiling is Marvelous Designer 2026.0: a production-grade cloth authoring tool with 2D pattern panels, sewing/seam constraints, anisotropic fabric physics (stretch-weft/warp/shear, bending-weft/warp, density, friction, pressure, solidify, shrinkage), a GPU-accelerated XPBD solver, avatar fitting, rigid trim coupling, UV-from-pattern accuracy, and export interoperability (FBX, OBJ, Alembic, glTF, VRM, USD). Marvelous Designer 2026.0 added the 3D Pencil (draw garment outlines directly on an avatar surface), the Lacing Tool, GPU-accelerated trim simulation, Toon Shader, glTF/VRM avatar import, EveryWear Rig Templates, pattern archiving, and timeline markers — these represent the 2026 high-water mark for interactive cloth tooling.

The Tailor engine is not a cost-saving substitute for a Marvelous Designer subscription. The differentiator is **model-first steerability**: an LLM can author, edit, and constrain a garment via a structured JSON API rather than via a human GUI. Marvelous Designer 2026.0 has no LLM-steerable sewing-pattern authoring. The research field has demonstrated this is possible — ChatGarment (CVPR 2025) fine-tunes a VLM to produce GarmentCode JSON from text or image; NGL-Prompter (submitted Feb 2026, revised May 2026) does it training-free via Natural Garment Language; GarmentDiffusion (IJCAI 2025) generates tokenized edge-oriented pattern representations 100x faster than earlier transformer approaches — and the Handshake model-lane infrastructure makes this the native integration path rather than an external plugin.

### [T-OVERVIEW.handshake-constraints] Handshake-Native Constraints

The Tailor module is bound by the Handshake kernel contract. Every design decision must conform to the following hard constraints sourced directly from the wtc-kernel-009 codebase:

**Storage authority.** PostgreSQL + EventLedger is the sole authority backend, enforced by `no_sqlite_tripwire` and `assert_postgres_url` (CX-503S/CX-503R). No SQLite anywhere — not in the solver, not in tests, not as a cache. Garment authority rows live in PostgreSQL via `sqlx::PgPool`. EventLedger receipts are emitted for every mutation using the `NewKernelEvent::builder(...)` pattern found in `storage/postgres.rs`.

**EventLedger event taxonomy.** The kernel owns a `KernelEventType` enum with `SCREAMING_SNAKE_CASE` variants. The Tailor module extends it with variants such as `TailorGarmentDraftProposed`, `TailorSimRunStarted`, `TailorSimRunCompleted`, `TailorGarmentValidated`, `TailorGarmentPromoted`, and `TailorGarmentCrdtUpdateRecorded`, registered in `required_first_slice_events()`. Every Tailor mutation emits one such event, keyed by an idempotency key and stored with `payload_hash` (SHA-256 of canonical JSON).

**CRDT collaborative editing.** The kernel CRDT layer (`kernel/crdt/`) provides `CrdtUpdateRecordV1`, `KnowledgeStateVectorV1`, actor-site tracking, and `yjs_bridge`. Collaborative garment editing — multiple operators or model agents co-authoring a pattern simultaneously — maps directly onto the existing CRDT infrastructure by treating each garment as a CRDT document. No new CRDT infrastructure is required; the Tailor module opens a new document type on the existing `kernel_crdt_updates` table.

**Sandbox and promotion gate.** Model-authored garment proposals are not trusted directly into authority. They run inside the kernel sandbox (`kernel/sandbox/`, `SandboxAdapter` trait, process-tier by default) and are promoted only after passing the `PromotionGate` (`kernel/kb003_promotion/gate.rs`). The gate consumes a `SandboxRunV1`, a `ValidationReport`, optional `OperatorApprovalEvidence`, and produces a `PromotionDecisionV1` (Accepted or Rejected). Garments follow the identical lifecycle as all other KB003 artifacts: `REQUESTED -> STARTED -> COMPLETED | REJECTED`.

**Model lanes.** The `ModelAdapter` trait (`kernel/model_adapter.rs`) and `LlmClient` trait (`llm/mod.rs`) are the canonical surfaces for LLM calls. The Tailor module implements a `TailorModelAdapter` that receives a `ContextBundle` containing the garment specification and constraint description, invokes the LLM via the model-lane registry, and extracts a garment JSON from `artifact_payload`. It does not implement `ModelRuntime` — it consumes it. The kernel's existing Ollama and OpenAI-compat adapters are in scope at no additional infrastructure cost.

**Module pattern.** The Tailor module follows the atelier module pattern (`src/atelier/`): a domain-owned subdirectory under `src/backend/handshake_core/src/`, a `mod.rs` with domain error type (`TailorEngineError`) and `event_family` constants, a storage glue file parallel to `storage/kb003_storage.rs`, and an API surface under `src/api/tailor.rs` registering Axum routes in `api/mod.rs`. The module receives `AppState` (and thus `postgres_pool`) by reference — no separate initialization is needed.

**No GPU code in handshake_core today.** The current codebase (`Cargo.toml` for `handshake_core`) has zero `wgpu`, `CubeCL`, or WGSL dependencies. The XPBD cloth solver must be a **standalone Rust crate** — `tailor-solver` — added as a Cargo workspace member with its own `wgpu` dependency. The `handshake_core` Tailor module calls into it through a clean trait boundary (`pub trait ClothSolver: Send + Sync`) so the solver crate is UI-agnostic and kernel-agnostic.

### [T-OVERVIEW.differentiator] The Model-First Differentiator

The cloth simulation industry is entering a phase where LLM/VLM steerability of garment authoring is technically proven but not yet productized inside any major tool. The landscape as of mid-2026:

- Marvelous Designer 2026.0: AI features limited to AI Pose Generator (Beta) and AI Image Generator (texture plugin). No LLM-steerable pattern authoring.
- ChatGarment (CVPR 2025, MPI): fine-tunes LLaVA to output GarmentCode JSON from text or image; reduced token count from 900 to 350 for training efficiency; dataset of 20K garments and 1M described images. Demonstrates the JSON-pattern bridge is achievable.
- GarmentDiffusion (IJCAI 2025, open-source): diffusion transformer on edge-token sequences; 10x shorter sequence than SewingGPT; 100x faster generation; accepts text, image, or partial pattern inputs; state-of-the-art on DressCodeData and GarmentCodeData.
- NGL-Prompter (arXiv Feb 2026): training-free; introduces Natural Garment Language (NGL) that restructures GarmentCode for VLM legibility; queries VLMs, maps output deterministically to valid sewing patterns; handles multi-layer outfits; state-of-the-art on geometry metrics without any fine-tuning.
- Textile IR (arXiv Jan 2026): proposes a bidirectional intermediate representation connecting CAD pattern tools, physics simulation, and sustainability assessment; formalizes garments as constraint-satisfaction programs; seven-layer verification from syntax to drape simulation.
- Design2GarmentCode (CVPR 2025, Style3D): LMM generates GarmentCode pattern-making programs from images, text, or sketches; program synthesis reduces dataset dependency.

The Tailor engine's differentiator is owning this pipeline natively inside the kernel model-lane infrastructure: LLM proposes a garment JSON spec via `TailorModelAdapter` -> sandbox runs the XPBD solver -> `ValidationRunner` checks mesh topology, seam closure, collision clearances -> `PromotionGate` accepts or rejects -> accepted garment becomes a PostgreSQL authority row with a full EventLedger audit trail. This loop is deterministic, reproducible, and observable — the operator can replay any garment authoring session from EventLedger events alone. No equivalent exists in the current cloth tooling market.

### [T-OVERVIEW.prior-rd] Link to Prior Avatar1 R&D

This research builds on prior decisions made during the Avatar1 side R&D (cross-checked with the Handshake codebase):

**Solver core.** XPBD (Extended Position-Based Dynamics, Macklin et al. 2016) in a standalone, UI-agnostic Rust crate. GPU compute via WGSL compute shaders over wgpu. Constraint types: stretch (anisotropic weft/warp/shear), dihedral bend (anisotropic weft/warp), seam/distance, volume/pressure, stitch point (tack), self-collision, cloth-avatar collision, friction.

**GPU backend.** wgpu v29 (Apache-2/MIT) as the cross-platform transport (Vulkan/Metal/DX12/WebGPU via Naga shader translation). WGSL for all compute shaders. CubeCL as an optional CUDA fast-path behind a feature flag. `wgsl_bindgen` / `wgsl_to_wgpu` for compile-time typesafe Rust/WGSL bind-group interfaces.

**Rigid collision proxies only.** Rapier/parry (Apache-2, Dimforge) provides rigid body shapes (capsules, convex hulls, trimesh compounds) for avatar segment proxies and trim rigid bodies. Rapier has confirmed no cloth/soft-body solver; XPBD cloth constraints are hand-written in WGSL.

**Bevy as throwaway testbed.** Bevy (or Avian for ECS physics) is used only in the solver crate's `examples/testbed` for interactive visual validation of constraint behavior. Bevy is not a dependency of the solver crate itself and has zero surface area in `handshake_core`.

**OxiPhysics.** A new pure-Rust unified physics engine (Apache-2, cool-japan, released April 2026, v0.1.2 as of June 2026) with an `oxiphysics-softbody` crate implementing PBD/XPBD, cloth with self-collision, hair, ropes, and surgical simulation. GPU support (wgpu) is roadmapped for v0.2.0 but not yet available; the CPU implementation and crate architecture are a useful reference for the `tailor-solver` design. The `oxiphysics-collision` crate (GJK/EPA, BVH, CCD) is an alternative reference to parry for specialized collision detection patterns.

**Pattern intermediate representation.** GarmentCode (ETH Zurich, MIT-adjacent) is the best available open-source sewing-pattern schema: hierarchical JSON with `Panel`, `Edge`, `Component`, `Interface` primitives. The Handshake garment authority schema is a typed Rust equivalent that round-trips to/from GarmentCode JSON for interop with ChatGarment-style LLM tooling and external pipeline tools (Maya, Blender, Style3D).

### [T-OVERVIEW.module-topology] Module Topology

The Tailor module occupies three layers in the Handshake runtime:

```text
┌─────────────────────────────────────────────────────────────────────┐
│  handshake_core (single Rust crate)                                 │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  src/tailor/                 (Tailor creative module)         │   │
│  │    mod.rs                    TailorEngineError, event_family  │   │
│  │    garment.rs                GarmentDraft, GarmentAsset      │   │
│  │    material.rs               FabricProperties, anisotropy    │   │
│  │    seam.rs                   SeamDefinition, ratio sewing    │   │
│  │    simulation.rs             ClothSimRequest, SimResult      │   │
│  │    solver_binding.rs         ClothSolver trait + bridge      │   │
│  │    model_adapter.rs          TailorModelAdapter (LLM lane)   │   │
│  │    sandbox_adapter.rs        TailorSandboxAdapter            │   │
│  │    validation.rs             mesh topology, seam closure     │   │
│  │    storage_glue.rs           sqlx + PgPool authority writes  │   │
│  ├──────────────────────────────────────────────────────────────┤   │
│  │  src/api/tailor.rs           Axum routes (REST API)         │   │
│  └──────────────────────────────────────────────────────────────┘   │
│  (attaches to AppState::postgres_pool, AppState::llm_client,       │
│   kernel CRDT layer, PromotionGate, SandboxRunner)                  │
└─────────────────────────────────────────────────────────────────────┘
               │  ClothSolver trait boundary (async, Send+Sync)
               ▼
┌──────────────────────────────────────────────────────────────────────┐
│  tailor-solver                  (standalone Rust workspace crate)    │
│    src/solver.rs                XPBD simulation loop                 │
│    src/constraints/             stretch, bend, seam, collision       │
│    src/gpu/                     wgpu device, queue, bind groups      │
│    shaders/                     *.wgsl compute shaders               │
│    examples/testbed/            Bevy/Avian throwaway viewport        │
└──────────────────────────────────────────────────────────────────────┘
```

The PostgreSQL migrations for the cloth domain follow the numbered `.sql` convention in `migrations/`, adding:

```sql
-- tailor_garments: authority rows for promoted garment assets
-- tailor_simulation_runs: FK to kb003_sandbox_runs
-- tailor_material_library: named fabric property presets
-- tailor_seam_definitions: per-garment seam constraint store
```

### [T-OVERVIEW.build-order] Build-Order Note

The Tailor module is a **forward research topic for a future creative module**. It is not a near-term kernel-build packet. The build order is:

1. The Handshake kernel must reach a stable governance state (WP-KERNEL-009 and its successors complete) before the Tailor module is activated as a work packet.
2. The `tailor-solver` crate can be prototyped independently as a standalone workspace crate — solver R&D does not require a running Handshake kernel.
3. The Tailor creative module (`src/tailor/`) is authored and tested against the kernel primitives only after the kernel's sandbox/promotion/CRDT surfaces are stable enough to accept new creative module attachments.
4. The Tauri command surface (`tailor_simulate`, `tailor_get_garment`, `tailor_promote_garment`) is the last layer, added after the core module and solver crate are integration-tested.

The research topics in this directory (`cloth_engine_research/`) are produced now so that when the build order reaches the Tailor module, a fresh model with no context can read the research package and begin implementation without re-doing the landscape survey.

### [T-OVERVIEW.feature-scope] Feature Scope Map

The following table maps Marvelous Designer feature groups (the target ceiling) to the Handshake-native design areas covered by the remaining research topics:

| MD Feature Group | Handshake Research Topic |
|---|---|
| 2D pattern authoring (panels, darts, pleats, 3D Pencil) | T-PATTERN-SCHEMA |
| Sewing / seam system (segment, free, 1:N, M:N ratio) | T-SEAM-CONSTRAINTS |
| Fabric / material physical properties (anisotropic PBW) | T-FABRIC-MODEL |
| Simulation engine (XPBD GPU, substeps, iterations) | T-SOLVER-CORE |
| Self-collision and cloth-cloth collision | T-COLLISION |
| Avatar system / body fitting | T-AVATAR-BINDING |
| Animation and keyframeable physical properties | T-ANIMATION |
| Trims, accessories, and hardware (rigid coupling) | T-TRIM-RIGID |
| UV and texturing (UV-from-pattern, grain accuracy) | T-UV-TEXTURE |
| Garment library and asset management | T-ASSET-AUTHORITY |
| Import / export pipeline interop | T-PIPELINE-INTEROP |
| LLM-steerable garment authoring (model-first) | T-MODEL-LANE |
| EventLedger, CRDT, sandbox, promotion gate integration | T-KERNEL-INTEGRATION |
| WGSL compute shader architecture | T-WGSL-SHADERS |
| Validation gates (mesh topology, seam closure) | T-VALIDATION |

### [T-OVERVIEW.risks] Risks and Open Questions

**Anisotropic fabric model complexity.** True cloth orthotropy (separate weft/warp/shear stiffness axes in the constraint solver) is not present in any current open-source XPBD Rust crate. The `xpbdrs` crate and `softy` crate both use isotropic constraints. Implementing anisotropic stretch and bend constraints in WGSL is the highest technical risk in the solver crate and requires derivation from the academic literature (Macklin 2016, and the orthotropic XPBD extensions by Kelager et al.).

**GPU solver self-collision performance.** Reaching CPU-parity collision accuracy on GPU for multi-layer garments (the Marvelous Designer 2024.2 GPU milestone) required NVIDIA engineering effort over multiple years. The WGSL spatial-hash approach (from Velvet's CUB-radix-sort reference and ccincotti3's constraint graph coloring) is the starting point, but production-quality self-collision at reasonable particle counts is an open risk. The Velvet CUDA architecture and OxiPhysics collision crate are reference implementations.

**wgpu GPU coverage on Windows (DX12 / Vulkan).** wgpu v29 targets Vulkan and DX12 as first-class Windows backends. The Handshake development environment is Windows 11 (per the comfyui-workbench env notes). Naga shader translation from WGSL to HLSL/SPIR-V is generally stable but edge cases in compute buffer layouts and binding semantics can differ between backends. Integration testing on DX12 as the primary Windows path is required.

**Bidirectional 2D-3D loop latency.** The MD "moat" feature is that 2D pattern edits drive 3D drape in near-real-time and the solver feeds accurate edge lengths back to the 2D view. In Handshake, this loop runs through the CRDT-tracked garment document -> solver crate async call -> result projection back to the operator surface. The latency of a full XPBD solve triggered by a 2D edit needs to be below perceptible threshold for interactive authoring. Incremental solve (warm-start from prior state) is a required design constraint for the solver crate.

**CRDT merge conflicts on pattern geometry.** Panel geometry (control points, edge handles) under concurrent CRDT edits by two model agents or an operator and a model can produce geometrically invalid states (overlapping panels, broken seam references). The CRDT merge must either use a geometry-aware conflict resolution strategy or route conflicting edits to the PromotionGate for operator decision. The `claim_promotion` / `promotion_bridge` pattern in the existing kernel CRDT layer is the starting point.

**GarmentCode JSON interop versioning.** GarmentCode (ETH Zurich) is evolving — the ChatGarment paper introduced GarmentCodeRC (Richer & Cleaner) with a restructured schema. Any Handshake garment schema that round-trips to GarmentCode must track schema version evolution or risk breaking interop with ChatGarment and NGL-Prompter tooling. A versioned `garment_schema_version` field in the PostgreSQL authority row and a migration path in the solver crate's JSON deserialization are required.

**OxiPhysics wgpu roadmap.** OxiPhysics v0.1.2 (June 2026) has GPU support planned for v0.2.0 but not yet implemented. If wgpu lands in oxiphysics before `tailor-solver` is implemented, it may be worth evaluating `oxiphysics-softbody` as the solver crate foundation rather than building from scratch, given its 60K-test coverage and clean Rust-only architecture. This should be reassessed at build-order step 2.

### [T-OVERVIEW.sources] Sources

- https://support.marvelousdesigner.com/hc/en-us/articles/55837641308313-Marvelous-Designer-2026-0-New-Feature-List
- https://www.cgchannel.com/2026/04/clo-virtual-fashion-releases-marvelous-designer-2026-0/
- https://digitalproduction.com/2026/04/15/marvelous-designer-2026-0-adds-3d-pencil-and-lacing/
- https://chatgarment.github.io/
- https://arxiv.org/html/2412.17811v1
- https://arxiv.org/abs/2602.20700
- https://github.com/Shenfu-Research/GarmentDiffusion
- https://style3d.github.io/design2garmentcode/
- https://arxiv.org/abs/2601.02792
- https://github.com/maria-korosteleva/GarmentCode
- https://github.com/gfx-rs/wgpu
- https://github.com/vitalight/Velvet
- https://github.com/ccincotti3/webgpu_cloth_simulator
- https://github.com/jspdown/cloth
- https://github.com/cool-japan/oxiphysics
- https://kitasanio.medium.com/oxiphysics-0-1-0-one-engine-to-replace-bullet-openfoam-lammps-and-calculix-in-pure-rust-c7cfd82a7b73
- https://github.com/dimforge/rapier
- https://matthias-research.github.io/pages/publications/XPBD.pdf
