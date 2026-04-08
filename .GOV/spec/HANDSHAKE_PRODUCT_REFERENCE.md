# Handshake Product Reference

Companion to the Master Spec (`SPEC_CURRENT.md`). Single-page entry point for models and humans.
Every row links to a spec section. This file describes what Handshake IS — not roadmap status or WP backlog.

> **REFERENCE ONLY.** This file is a navigation aid. All decisions, technical advice, and implementation guidance MUST be derived from the Master Spec (via `SPEC_CURRENT.md`), not from this summary. When in doubt, read the spec section referenced in the §ref column. Do not cite this file as authority for design choices.

---

## 1. Product Identity

Handshake is a local-first, AI-native desktop application combining Notion-like documents, Milanote-style visual canvas, and Excel-like spreadsheets — all running on your machine with local AI models and optional cloud escalation. It integrates a 1,200+ clause governance system making AI behavior reliable, auditable, and deterministic across every creative and execution surface. Every feature is designed as a force multiplier — primitives, tools, engines, and pillars interweave so each combination produces more capability than the sum of its parts.

---

## 2. Tech Stack

| Layer | Technology | Spec §ref |
|---|---|---|
| Desktop shell | Tauri (not Electron) | §1.1.3, §2.1.1 |
| Backend coordinator | Rust | §2.1.1 |
| AI orchestration | Python (AutoGen/LangGraph) | §1.1.3 |
| Frontend | React + TypeScript | §1.1.3 |
| Rich text editor | Tiptap / ProseMirror + BlockNote | §7.1.1 |
| Spreadsheet engine | Wolf-Table + HyperFormula | §7.1.0, §2.2.1.13 |
| Code editor | Monaco + Monarch grammar | §10.2 |
| Canvas | Excalidraw | §6.3.3.5 |
| Storage (local) | SQLite (primary) | §2.3.13 |
| Storage (cloud) | PostgreSQL (multi-user ready) | §2.3.13 |
| Storage boundary | Database trait — portable SQL, dual-backend tested | §2.3.13 |
| Observability DB | DuckDB (Flight Recorder) | §11.5 |
| Collaboration | Yjs CRDTs | §1.1.3 |
| Embedding (local) | nomic-embed-text via Ollama | §2.3.14 |
| LLM integration | Multi-provider MCP with capability gating | §2.1.7 |
| Document ingestion | Docling (MIT, layout-aware) | §6.1 |
| File storage | File-tree based, human-readable, git-friendly | §1.1.3 |

---

## 3. Pillars

The 22 product pillars. Every WP refinement forces a status declaration against each pillar to surface force multipliers.

| # | Pillar | What it is | Technical approach | Spec §ref |
|---|---|---|---|---|
| 1 | Flight Recorder | Event-level observability for every runtime action | DuckDB event store, typed event families (FR-EVT-001+), trace correlation, 7-30d retention, replay UI | §11.5 |
| 2 | Calendar | Time-structured workspace view — not just appointments but activity correlation | React Big Calendar (dumb view) + canonical backend law, ActivitySpan join, temporal correlation with FR | §10.4 |
| 3 | Monaco | Code editor surface | Monaco + Monarch grammar for Handshake syntax, LSP bridges, block-level edit IDs for AI ops | §10.2 |
| 4 | Word clone | Block-based document editor (Notion-like) | Tiptap/ProseMirror + BlockNote, block IDs, `ai_origin` provenance, Yjs collab, slash commands | §7.1.1, §2.5.10 |
| 5 | Excel clone | Spreadsheet surface | Wolf-Table grid + HyperFormula (400+ functions), stable cell IDs, `ai_source` provenance, MEX adapter | §2.2.1.13, §7.1.0 |
| 6 | Locus | Structured work tracking — WP/MT lifecycle with full observability | Canonical structured records, Bronze/Silver/Gold sync, dependency management, Spec Router integration | §2.3.15 |
| 7 | Loom | Artifact retrieval library — block-as-unit-of-meaning | Heaper-pattern UX (All/Unlinked/Sorted/Pins), relational linking, content hashing, AI-generated metadata | §10.12 |
| 8 | Work packets (product) | In-product governed task containers | Immutable after signature, scope/closure/traceability contracts, variant lineage (v2, v3) | §Part 5 |
| 9 | Task board (product) | In-product status projection and phase closure gates | Single source of truth, phase closure requires all-VALIDATED + spec regression + supply chain audit | §Part 6 |
| 10 | MicroTask | Atomic work unit executor with claim/validate/escalate lifecycle | AI Job profile, iteration budgets (per-MT + total + wall-clock), smart drop-back, LoRA-aware escalation | §2.6.6.8.5 |
| 11 | Command Center (DCC) | Operator visibility surface for sessions, governance, artifacts, approvals | Panels: project registry, worktrees, execution sessions, approval inbox, conversation timeline, tool call ledger | §10.11 |
| 12 | Front End Memory System | Bounded provenance-first memory for the front-end model | 4 classes (working/episodic/semantic/procedural), MemoryPack ≤500 tokens ≤24 items, review-gated procedural writes, replay-grade | §2.6.6.7.6.2 |
| 13 | Execution / Job Runtime | AI Job Model, Workflow Engine, ModelSession, session scheduling | Typed workflow DAGs, durable SQLite-backed state, crash recovery, lane-based priority, cooperative cancellation | §2.6, §4.3.9 |
| 14 | Spec to prompt | Transforms spec sections into deterministic prompt envelopes | SpecPromptCompiler loads SpecPromptPacks, injects CapabilitySnapshots, records pack SHA-256 + token counts | §2.6.8.5.2 |
| 15 | SQL to PostgreSQL shift readiness | Storage portability via Database trait boundary | Portable SQL only, dual-backend migration testing, rebuildable indexes, no DB-specific syntax | §2.3.13 |
| 16 | LLM-friendly data | All data structured for model consumption | Bronze/Silver/Gold medallion, hybrid indexing (vector+keyword+graph), semantic chunking (AST-aware code, header-recursive docs) | §2.3.14 |
| 17 | Stage | Evidence-grade media capture/import pipeline | Browser surface with Stage Apps, capture jobs through Workflow Engine, artifact bundles + SHA-256 manifests | §10.13 |
| 18 | Studio | Creative shell — Canvas, Photo, Lens, design surfaces | Cross-surface runtime orchestration, Darkroom photo engines, explicit lineage between Stage/ASR/Lens artifacts | §6.3.3.5, §10.10 |
| 19 | Atelier/Lens | Collaboration panel + governed creative extraction | LensExtractionTier (cheap vs deep), role-lane retrieval, proposal workflows with evidence, replayable versioning | §6.3.3.5 |
| 20 | Skill distillation / LoRA | Training pair extraction from governed work for model specialization | Escalation-driven candidate generation, teacher→student format, adapter-only LoRA/QLoRA/DoRA, benchmark-gated | §2.6.6.8.13, §9 |
| 21 | ACE | Autonomous Collaboration Engine — context engineering runtime | WorkingContext compilation per model call, tiered memory (durable vs per-call), ContextSnapshot for audit/replay | §2.6.6.7 |
| 22 | RAG | Retrieval-Augmented Generation — Shadow Workspace + evidence pipeline | Bronze/Silver/Gold layers, hybrid search (HNSW + BM25 + graph), cross-encoder reranking, deterministic candidate selection | §2.3.8, §2.3.14 |

---

## 4. Mechanical Engines

The 22 spec-grade engines (§11.8). Each is a stand-alone feature surface and force multiplier. All engines route through the Mechanical Tool Bus with governance, Flight Recorder logging, and capability gating.

### Engineering & Manufacturing

| # | Engine ID | Name | Purpose |
|---|---|---|---|
| 1 | `engine.spatial` | Spatial | Generate and validate parametric 3D geometry; export CAD/mesh formats; compute fit/clearance reports |
| 2 | `engine.machinist` | Machinist | Generate, parse, visualize, and validate toolpaths and machine instructions (CNC/3D printing/laser) |
| 3 | `engine.physics` | Physics | Unit consistency checking, conversions, and formula evaluation with explicit units |
| 4 | `engine.simulation` | Simulation | Run physics simulations (FEA/CFD/dynamics) as governed batch jobs; produce plots/fields |
| 5 | `engine.hardware` | Hardware | Controlled access to cameras/mics/USB/serial/sensors with explicit consent and evidence capture |

### Creative Studio

| # | Engine ID | Name | Purpose |
|---|---|---|---|
| 6 | `engine.director` | Director | Render timelines, generate animations, and perform deterministic video transforms/transcodes |
| 7 | `engine.composer` | Composer | Symbolic music processing, engraving, synthesis, mixing, and audio analysis |
| 8 | `engine.artist` | Artist | Scriptable vector/photo/painting transforms producing editable outputs (SVG, graded images, layered files) |
| 9 | `engine.publisher` | Publisher | Print-ready layout, typesetting, kinetic typography renders, and font tooling under policy |

### Culinary & Home

| # | Engine ID | Name | Purpose |
|---|---|---|---|
| 10 | `engine.sous_chef` | Sous Chef | Parse, standardize, scale recipes and generate shopping lists with unit safety |
| 11 | `engine.food_safety` | Food Safety | Deterministic checks against safety curves, nutrition databases, and fermentation telemetry rules |
| 12 | `engine.logistics` | Logistics | Meal plans, pantry inventory, label printing, and schedule optimization for household workflows |

### Organization & Knowledge

| # | Engine ID | Name | Purpose |
|---|---|---|---|
| 13 | `engine.archivist` | Archivist | Preserve web/media sources to prevent link rot; capture snapshots; store bundles with canonical hashing |
| 14 | `engine.librarian` | Librarian | Metadata extraction, bibliography formatting, photo EXIF categorization, and ebook conversions |
| 15 | `engine.analyst` | Analyst | Read-only analytics over personal sources (maildirs/tasks/time logs) producing structured datasets and reports |

### Data & Infrastructure

| # | Engine ID | Name | Purpose |
|---|---|---|---|
| 16 | `engine.wrangler` | Wrangler | Data extraction, validation, conversion, and dedup at scales beyond LLM context limits |
| 17 | `engine.dba` | DBA | In-process databases for OLAP queries and indexing, returning results as artifacts |
| 18 | `engine.sovereign` | Sovereign | Encryption/signing/key usage and controlled sync/p2p; secrets never enter LLM context |

### Travel & Spatial Intelligence

| # | Engine ID | Name | Purpose |
|---|---|---|---|
| 19 | `engine.guide` | Guide | Geocode, route, and verify travel facts with evidence capture and replay semantics |

### Developer Tools & System Context

| # | Engine ID | Name | Purpose |
|---|---|---|---|
| 20 | `engine.context` | Context | Deterministic local search (grep + index) returning snippet references rather than raw file dumps |
| 21 | `engine.version` | Version | Versioning operations without requiring CLI; supports undo/history/diff/sync under policy |
| 22 | `engine.sandbox` | Sandbox | Run code safely (prefer WASM/container) to produce artifacts and structured results; enforce strict allowlists |

---

## 5. Primitive Index

~400+ primitives in Appendix 12.4 (`hs_primitive_tool_tech_matrix@2`). Each entry has: `primitive_id` (PRIM-*), `title`, `kind` (ts_interface / rust_struct / rust_enum / react_component / spec_schema / py_dataclass / rust_trait).

Primitives are the atomic building blocks. The interaction matrix (§6) connects them to features and each other. Full index in spec Appendix 12.4.

### 5.1 Storage & Data

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-Database | Database trait boundary | rust_trait |
| PRIM-AiReadyDataPipeline | Bronze/Silver/Gold ingestion pipeline | rust_struct |
| PRIM-EmbeddingRegistry | Embedding model versioning | rust_struct |
| PRIM-HybridWeights | Vector/keyword fusion weights | rust_struct |
| PRIM-HybridRetrievalParams | Hybrid search configuration | rust_struct |
| PRIM-DocIngestSpec | Document ingestion specification | rust_struct |
| PRIM-DocIngestResult | Ingestion result with lineage | rust_struct |
| PRIM-GoldenQuerySpec | Gold-tier query definition | rust_struct |
| PRIM-DeterminismMode | strict / replay / best_effort | rust_enum |
| PRIM-RetrievalBudgets | Token/item budgets for retrieval | rust_struct |
| PRIM-RetrievalFilters | Scope/trust/classification filters | rust_struct |
| PRIM-QueryPlan | Retrieval execution plan | rust_struct |
| PRIM-RetrievalTrace | Retrieval provenance record | rust_struct |

### 5.2 Session & Execution

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-ModelSession | Persistent model conversation unit | rust_struct |
| PRIM-SessionMessage | Single message in a session | rust_struct |
| PRIM-SessionSchedulerConfig | Concurrency/rate/lane config | rust_struct |
| PRIM-RoutingStrategy | Model selection strategy | rust_enum |
| PRIM-SpawnLimits | Session spawn budgets | rust_struct |
| PRIM-RateLimitReservation | Rate limit reservation | rust_struct |
| PRIM-RateLimitOutcome | Rate limit result | rust_struct |
| PRIM-WorkflowRun | Workflow execution instance | rust_struct |
| PRIM-WorkflowNodeExecution | Single node execution record | rust_struct |
| PRIM-MicroTaskSummary | MT status snapshot | rust_struct |

### 5.3 AI / LLM

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-AiJob | AI job lifecycle container | rust_struct |
| PRIM-AiJobMcpFields | MCP-specific job fields | rust_struct |
| PRIM-AiJobMcpUpdate | MCP job mutation | rust_struct |
| PRIM-AccessMode | analysis_only / preview_only / apply_scoped | rust_enum |
| PRIM-SpecPromptPackV1 | Versioned prompt template | spec_schema |
| PRIM-PromptEnvelopeV1 | Compiled prompt with budgets | spec_schema |
| PRIM-WorkingContextV1 | Per-call context assembly | spec_schema |
| PRIM-ContextBlockV1 | Single context block in envelope | spec_schema |
| PRIM-LoadedSpecPromptPack | Resolved prompt pack | spec_schema |

### 5.4 Governance & Workflow

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-GateConfig | Capability gate configuration | rust_struct |
| PRIM-GateStatuses | Gate evaluation result set | rust_struct |
| PRIM-ToolPolicy | Tool access policy | rust_struct |
| PRIM-WorkPacketPhase | WP lifecycle phase | rust_enum |
| PRIM-WorkPacketGovernance | WP governance metadata | rust_struct |
| PRIM-WorkPacketType | WP type classification | rust_enum |
| PRIM-LocusCreateWpParams | WP creation parameters | rust_struct |
| PRIM-LocusSyncTaskBoardParams | Task board sync parameters | rust_struct |
| PRIM-LocusOperation | Locus state mutation | rust_struct |

### 5.5 UI / Presentation

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-AtelierCollaborationPanel | Collaboration panel component | react_component |
| PRIM-CanvasView | Canvas surface component | react_component |
| PRIM-ExcalidrawCanvas | Excalidraw integration | react_component |
| PRIM-CommandPalette | Command palette | react_component |
| PRIM-DocumentView | Document editor view | react_component |
| PRIM-FlightRecorderView | FR timeline view | react_component |
| PRIM-MediaDownloaderView | Media download UI | react_component |
| PRIM-ViewModeToggle | View mode switcher | react_component |
| PRIM-WorkspaceSidebar | Workspace navigation | react_component |
| PRIM-AiJobsDrawer | Job status drawer | react_component |

### 5.6 Media & Content

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-LoomViewFilters | Loom library filters | rust_struct |
| PRIM-LoomSearchFilters | Loom search parameters | rust_struct |
| PRIM-LoomBlockSearchResult | Loom search result | rust_struct |

### 5.7 Calendar & Temporal

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-CalendarSource | Calendar data source | rust_struct |
| PRIM-CalendarSourceUpsert | Calendar source mutation | rust_struct |
| PRIM-CalendarEvent | Calendar event entity | rust_struct |
| PRIM-CalendarEventUpsert | Calendar event mutation | rust_struct |
| PRIM-CalendarEventWindowQuery | Time-window query | rust_struct |
| PRIM-CalendarSourceSyncState | Sync state machine | rust_struct |
| PRIM-CalendarSourceProviderType | Source provider enum | rust_enum |
| PRIM-CalendarSyncStateStage | Sync stage enum | rust_enum |
| PRIM-CalendarEventStatus | Event status enum | rust_enum |
| PRIM-CalendarEventVisibility | Event visibility enum | rust_enum |
| PRIM-CalendarEventExportMode | Export mode enum | rust_enum |
| PRIM-CalendarSourceWritePolicy | Write policy enum | rust_enum |
| PRIM-ActivitySpanBinding | Temporal activity binding | spec_schema |

### 5.8 Tools & MCP

| Primitive ID | Name | Kind |
|---|---|---|
| PRIM-McpContext | MCP execution context | rust_struct |
| PRIM-ToolTransportBindings | Tool transport configuration | rust_struct |
| PRIM-ToolRegistryEntry | Tool registration record | rust_struct |
| PRIM-GatedMcpClient | Capability-gated MCP client | rust_struct |
| PRIM-McpToolDescriptor | MCP tool definition | rust_struct |
| PRIM-McpResourceDescriptor | MCP resource definition | rust_struct |
| PRIM-JsonRpcMcpClient | JSON-RPC MCP transport | rust_struct |
| PRIM-McpCall | MCP tool invocation | rust_struct |
| PRIM-EngineAdapter | Mechanical engine adapter | rust_struct |
| PRIM-MexRegistry | MEX engine registry | rust_struct |
| PRIM-MexRuntimeError | MEX runtime error | rust_enum |
| PRIM-AdapterError | Engine adapter error | rust_enum |

> Full primitive index with ~400+ entries: Appendix 12.4 in Master Spec

---

## 6. Force Multiplier Map

Designed-in pillar × pillar interactions. These are intentional architectural combinations tracked in the interaction matrix (Appendix 12.6, `hs_interaction_matrix@2`).

| Combo | Pillars / Features involved | What it enables | IMX edge |
|---|---|---|---|
| Calendar × Flight Recorder | Calendar, FR | Query FR events by time window; correlate work activity with calendar blocks via ActivitySpan | IMX-010 |
| Calendar × Locus | Calendar, Locus | Correlate workload windows with WP/MT execution timelines | IMX-011 |
| Locus × DCC | Locus, Command Center | Project execution status projected into DCC panels | IMX-013 |
| Loom × AI-Ready Data | Loom, RAG | Library artifacts feed into Shadow Workspace Bronze/Silver/Gold retrieval | IMX-014 |
| Atelier/Lens × Loom | Atelier/Lens, Loom | Lens extraction uses Loom context for creative retrieval | IMX-015 |
| Unified Tool Surface × DCC | Execution, Command Center | Governed tool calls surfaced in DCC tool call ledger | IMX-012 |
| Unified Tool Surface × FR | Execution, FR | Every tool call logged with FR-EVT-007 events | IMX-002 |
| Unified Tool Surface × Capabilities | Execution, ACE | All tool calls gated by capability/consent system | IMX-001 |
| MCP → Unified Tool Surface | Tools, Execution | MCP tools route through same governance gates as native tools | IMX-003 |
| Spec Router × Locus | Spec to prompt, Locus | SpecRouter creates and binds work packets into Locus | IMX-004 |
| MT Executor × Locus | MicroTask, Locus | MT iterations recorded in Locus with FR-EVT-MT-* events | IMX-005 |
| ACE × AI-Ready Data | ACE, RAG | ACE retrieval pipeline uses AI-Ready Data hybrid search (ACE-RAG-001) | IMX-006 |
| Operator Consoles × FR | Command Center, FR | Evidence drilldown from DCC into FR timeline | IMX-007 |
| Loom × FR | Loom, FR | Loom operations emit FR-EVT-LOOM-* events | IMX-008 |
| Media Downloader × Stage | Stage, Loom | Media downloads reuse Stage sessions for auth | IMX-009 |
| Role Mailbox × AI-Ready Data | Execution, RAG | Message content indexed for retrieval | IMX-016 |
| Storage Portability × FR | SQL→PG, FR | Artifact lineage logging across storage backends | IMX-020 |
| Locus × Debug Bundle | Locus, FR | WP export anchor for debug bundles | IMX-019 |
| Spec Router × Capabilities | Spec to prompt, ACE | CapabilitySnapshot injected into compiled prompts | IMX-018 |

> Full interaction matrix with 100+ edges: Appendix 12.6 in Master Spec

---

## 7. Tool Surface

### 7.1 Unified Tool Surface

All tool calls (MCP and native) route through the same governance gates: capability check → consent check → FR logging → execution → result capture.

| Category | What it covers | Spec §ref |
|---|---|---|
| MCP tools | External tool access via JSON-RPC MCP protocol, routed through capability gates | §2.1.7 |
| Mechanical engines | 22 engine adapters (§11.8) exposed as governed tool calls | §11.8 |
| Workspace ops | File/entity CRUD, search, navigation — all through Workflow Engine | §2.6 |
| AI ops | Model invocation, prompt compilation, job lifecycle | §2.6.6 |

### 7.2 DCC Panels

| Panel | What it shows | Spec §ref |
|---|---|---|
| Project Registry | Multi-project workspace overview | §10.11 |
| Workspaces / Worktrees | Git worktree management and WP binding | §10.11 |
| Objective Anchor Store | Long-lived goals and intent tracking | §10.11 |
| Execution Session Manager | Active model sessions, lane status, token usage | §10.11 |
| Approval Inbox | Pending reviews: memory proposals, capability requests, MT escalations | §10.11 |
| Git Review / Commit | Diff viewer, commit flows, branch management | §10.11 |
| Conversation Timeline | Unified session conversation history | §10.11 |
| Codebase Search | Local search across workspace | §10.11 |
| Build / Test / Run Queues | Local build and test orchestration | §10.11 |
| Tool Call Ledger | All governed tool invocations with timing and results | §10.11.5.13 |
| Front End Memory Panel | Memory browser, write review, MemoryPack preview, conflict queue | §10.11.5.14 |

### 7.3 Flight Recorder Event Families

| Family | Event codes | What it captures |
|---|---|---|
| Terminal | FR-EVT-001 | Shell command execution |
| Editor | FR-EVT-002 | Document/code edits |
| Diagnostic | FR-EVT-003 | Errors, warnings, diagnostics |
| Retention | FR-EVT-004 | Data retention and linkability |
| Debug Bundle | FR-EVT-005 | Export events |
| LLM Inference | FR-EVT-006 | Model calls with timing/tokens |
| Tool Call | FR-EVT-007 | Governed tool invocations |
| Loom | FR-EVT-LOOM-* | Library operations |
| MicroTask | FR-EVT-MT-* | MT lifecycle events |
| Memory | FR-EVT-MEM-001..005 | FEMS proposal/review/commit/pack/status |
