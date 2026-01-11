# Master Spec Section Digest (v02.103)

## Scope
- Source: `Handshake_Master_Spec_v02.103.md`
- Roadmap excluded: `Handshake_Master_Spec_v02.103.md:20065` .. `Handshake_Master_Spec_v02.103.md:21465` (Section 7.6)

## Digest method (deterministic)
- Sections: all `## X.Y` headings excluding the Roadmap section.
- Purpose: extracted from the first `**Why**` / `**What**` blocks near the top of each section; if absent, first non-empty intro lines are used.
- Requirements: any line containing `MUST`, `SHOULD`, `REQUIRED`, `MUST NOT`, or `SHOULD NOT` (line-numbered, verbatim; may include examples in code blocks).
- Roadmap pointer check: whether the section number (e.g. `2.6`) appears in the Roadmap text, and which phases mention it by number.

## Summary
- Sections (excluding Roadmap): 67
- Sections with >=1 normative line: 34
- Sections with 0 normative lines: 33
- Total normative lines (excluding Roadmap): 830
- Sections whose number is mentioned in Roadmap: 34

---

## 1.1 Executive Summary
- Spec: `Handshake_Master_Spec_v02.103.md:174`
- Bounds: `Handshake_Master_Spec_v02.103.md:174` .. `Handshake_Master_Spec_v02.103.md:277`
- Why: Provides high-level orientation for readers new to the specification. Establishes context before technical details.
- What: Quick-start overview of Project Handshake: what it is, who it's for, and how this document evolved from both infrastructure research AND three months of AI governance R&D (the Prompt Diaries project).
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 1.2 The Diary Origin Story
- Spec: `Handshake_Master_Spec_v02.103.md:278`
- Bounds: `Handshake_Master_Spec_v02.103.md:278` .. `Handshake_Master_Spec_v02.103.md:363`
- Why: Understanding where Handshake's governance comes from explains why it's built the way it is. The Diary was 3 months of R&D that discovered what it actually takes to make AI reliable.
- What: This section explains the creative goal that started everything, the problems LLMs caused, the governance solution that emerged, and how Handshake transforms that into code.
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 1.3 The Four-Layer Architecture
- Spec: `Handshake_Master_Spec_v02.103.md:364`
- Bounds: `Handshake_Master_Spec_v02.103.md:364` .. `Handshake_Master_Spec_v02.103.md:435`
- Why: Understanding the layers helps you know where each piece of functionality lives. When something goes wrong, you know which layer to debug.
- What: Handshake has four layers: LLM (decides what), Orchestrator (enforces rules), Mechanical (executes deterministically), and Validation (confirms correctness).
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 1.4 LLM Reliability Hierarchy
- Spec: `Handshake_Master_Spec_v02.103.md:436`
- Bounds: `Handshake_Master_Spec_v02.103.md:436` .. `Handshake_Master_Spec_v02.103.md:504`
- Why: This hierarchy explains why some AI behaviors are trustworthy and others aren't. It guides every design decision: push enforcement UP the hierarchy.
- What: A ranking from most reliable (code enforcement) to least reliable (hoping the model remembers). Handshake operates at the top; the Diary operated near the bottom.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 1.5 What Gets Ported from the Diary
- Spec: `Handshake_Master_Spec_v02.103.md:505`
- Bounds: `Handshake_Master_Spec_v02.103.md:505` .. `Handshake_Master_Spec_v02.103.md:577`
- Why: Not everything from the Diary becomes Handshake code. Understanding the categories helps you know what to implement, what to configure, and what to skip.
- What: The ~1,232 Diary clauses fall into four categories: PORTED (becomes Rust types), TRANSFORMED (rules become code), PRESERVED (extraction core), and DEPRECATED (text-format specifics not needed).
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (0): (none detected by keyword scan)

## 1.6 Design Philosophy: Self-Enforcing Governance
- Spec: `Handshake_Master_Spec_v02.103.md:578`
- Bounds: `Handshake_Master_Spec_v02.103.md:578` .. `Handshake_Master_Spec_v02.103.md:703`
- Why: Understanding why the Diary embeds its own enforcement explains a key principle Handshake must preserve: rules and their validators must live together.
- What: Traditional document governance fails because rules and enforcement are separate. The Diary embeds lint rules and machine code alongside the RIDs they enforce. Handshake must preserve this pattern.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (1):
  - `Handshake_Master_Spec_v02.103.md:592` Some subsystems include an internal **LAW** section that is normative (example: **Calendar Law** in \xa710.4.1). For every LAW block, Handshake MUST ship:

## 1.7 Success Criteria
- Spec: `Handshake_Master_Spec_v02.103.md:704`
- Bounds: `Handshake_Master_Spec_v02.103.md:704` .. `Handshake_Master_Spec_v02.103.md:734`
- Why: Clear success criteria tell you when the implementation is working. Without these, you can't know if you're done.
- What: Six checkpoints that define a working Handshake implementation.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 1.8 Introduction
- Spec: `Handshake_Master_Spec_v02.103.md:735`
- Bounds: `Handshake_Master_Spec_v02.103.md:735` .. `Handshake_Master_Spec_v02.103.md:858`
- Why: This section establishes the foundational identity, target users, and design philosophy of Handshake. Without this grounding, subsequent technical decisions lack context and rationale.
- What: Defines Handshake as a local-first, AI-native desktop workspace that unifies document editing, visual canvases, and spreadsheets. Documents the specification's evolution and clarifies its relationship to the underlying infrastructure research.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 2.1 High-Level Architecture
- Spec: `Handshake_Master_Spec_v02.103.md:859`
- Bounds: `Handshake_Master_Spec_v02.103.md:859` .. `Handshake_Master_Spec_v02.103.md:1059`
- Why: Before diving into implementation details, you need a mental map of how all subsystems relate. This section provides that overview, enabling targeted deep-dives into specific layers.
- What: Enumerates and briefly describes the ten major architectural layers: Desktop Shell, Workspace Data Layer, Model Runtime Layer, Workflow Engine, Flight Recorder, Capability Layer, Connectors, AI UX, Taste Engine, and Dev Tools.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (4):
  - `Handshake_Master_Spec_v02.103.md:924` - All AI jobs MUST respect these capability scopes.
  - `Handshake_Master_Spec_v02.103.md:937` - When an external system exposes an MCP server, the Rust coordinator SHOULD prefer MCP over ad-hoc HTTP or custom protocols.
  - `Handshake_Master_Spec_v02.103.md:938` - All MCP traffic from connectors MUST pass through the same MCP Gate, capability, and Flight Recorder paths as the internal Python Orchestrator (\xa711.3).
  - `Handshake_Master_Spec_v02.103.md:939` - MCP connectors MUST NOT bypass capability checks or consent prompts defined in the Capabilities & Consent Model (\xa711.1).

## 2.2 Data & Content Model
- Spec: `Handshake_Master_Spec_v02.103.md:1060`
- Bounds: `Handshake_Master_Spec_v02.103.md:1060` .. `Handshake_Master_Spec_v02.103.md:1696`
- Why: The data model is the foundation for all features\u2014documents, canvases, tables, AI collaboration, sync, and search. Misunderstanding it leads to incorrect implementations and broken invariants.
- What: Defines core workspace entities (Workspace, Project, Document, Block, Canvas, Table, etc.), the Raw/Derived/Display content separation with formal rules, the knowledge graph schema, the Shadow Workspace indexing pipeline, the CRDT sync model treating AI as a participant, and the file-tree storage architecture.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (24):
  - `Handshake_Master_Spec_v02.103.md:1095` - Mechanical integrations (Docling, ASR, converters, image tools) **MUST** read/write workspace entities via the same Raw/Derived/Display model and IDs as the UI.
  - `Handshake_Master_Spec_v02.103.md:1100` - Tools and views **MAY** attach extra metadata, but **MUST NOT** require tool-specific storage schemas for core behaviours.
  - `Handshake_Master_Spec_v02.103.md:1103` - All non-trivial operations (import, transforms, ASR, bulk edits) **SHOULD** run as AI Jobs (Section 2.6.6) or workflow nodes, regardless of which tool initiated them.
  - `Handshake_Master_Spec_v02.103.md:1108` - Their outputs **MUST** land as RawContent/DerivedContent for workspace entities so that all downstream tools can consume them.
  - `Handshake_Master_Spec_v02.103.md:1111` - Content imported or produced in one view (e.g. Docling-imported table, ASR transcript, Docling-derived figure captions) **SHOULD** be accessible in others without copy-paste:
  - `Handshake_Master_Spec_v02.103.md:1120` These principles are normative for all tool and integration decisions. When choosing a new library or runtime, implementers **MUST** verify that it can fit into this model without introducing parallel data silos.
  - `Handshake_Master_Spec_v02.103.md:1156` - **Diagnostics**: Formula errors (e.g., `#DIV/0!`) MUST emit `Diagnostic` objects with `surface: "sheet"` and `source: "engine"`.
  - `Handshake_Master_Spec_v02.103.md:1271` All first-class artefacts (documents, canvases, sheets, media assets, diary-linked entities) **SHOULD** expose the following metadata fields in the workspace model:
  - `Handshake_Master_Spec_v02.103.md:1282` 1. `content_sensitivity` **MUST NOT** be inferred in ways that modify RawContent or DerivedContent. It is metadata for filtering, not a rewrite mechanism.
  - `Handshake_Master_Spec_v02.103.md:1283` 2. Entities marked `nsfw_adult_only` **MUST** also carry a non-null `consent_class` and `source_kind`.
  - `Handshake_Master_Spec_v02.103.md:1284` 3. Implementations **MUST** provide user-visible controls to inspect and override `content_sensitivity` on a per-entity and per-workspace basis.
  - `Handshake_Master_Spec_v02.103.md:1285` 4. Internal helpers (Docling, descriptor pipelines, Taste Engine, Diary RIDs such as DES-001/IMG-001/SYM-001) **MAY** set or refine `content_sensitivity` and `sensitivity_tags`, but **MUST NOT** drop or euphemise RawContent or DerivedContent when doing so.
  - `Handshake_Master_Spec_v02.103.md:1289` Each workspace **SHOULD** have a configurable `workspace_category` and `default_content_sensitivity`:
  - `Handshake_Master_Spec_v02.103.md:1298` 1. In **SFW** workspaces, UI and AI defaults **SHOULD** hide or down-rank entities labelled `nsfw_adult_only` unless explicitly requested.
  - `Handshake_Master_Spec_v02.103.md:1301` 4. Workspace-level settings **SHOULD** be honoured by AI Job configuration (e.g. default `consent_profile_id` and `safety_mode`), avoiding repeated consent prompts for the same workspace while still making configuration inspectable.
  - `Handshake_Master_Spec_v02.103.md:1307` 1. `consent_class` **SHOULD** be drawn from a small, well-defined enum, for example:
  - `Handshake_Master_Spec_v02.103.md:1312` 2. For entities with `content_sensitivity = nsfw_adult_only`, implementations **SHOULD** treat `third_party_unverified` as high risk:
  - `Handshake_Master_Spec_v02.103.md:1315` 3. AI Jobs working over NSFW content **MUST** carry a `consent_profile_id` in their configuration (Section 2.6.6.2.2). This profile:
  - `Handshake_Master_Spec_v02.103.md:1319` 4. Diary-side RIDs (DES-001, IMG-001, SYM-001) and their CONFIG profiles (e.g. an `adult_only_v01` material profile) **MAY** be used to enforce additional invariants (e.g. all subjects are adults, explicit consent metadata present). Handshake **MUST NOT** weaken those RIDs; it consumes their outputs as authoritative.
  - `Handshake_Master_Spec_v02.103.md:1325` 1. RawContent and DerivedContent **MUST** retain full, uncensored descriptors and content for all ingested material, subject only to user-driven deletion or privacy features. Descriptor pipelines, symbolism engines, and mechanical tools **MUST NOT** perform irreversible censorship at these layers.
  - `Handshake_Master_Spec_v02.103.md:1327` 3. Any redaction or obfuscation performed for external connectors or SFW views **MUST NOT** be written back into RawContent or DerivedContent. Connectors operate over filtered views, not by mutating stored values.
  - `Handshake_Master_Spec_v02.103.md:1328` 4. When calling external models that disallow NSFW content, orchestrators **MAY** replace sensitive spans with neutral placeholders or higher-level descriptors in the prompt, but **MUST** keep an internal mapping so that responses can still be linked back to the original entities without leaking NSFW details to those models.
  - `Handshake_Master_Spec_v02.103.md:1344` - All Photo Stack entities MUST be representable as **Assets** (or Asset-linked records) under the workspace model (no hidden engine-private long-term stores).
  - `Handshake_Master_Spec_v02.103.md:1345` - All entities MUST be addressable by content hash / artifact handle where applicable (see \xa72.3.10).

## 2.3 Content Integrity (Diary Part 5: COR-700)
- Spec: `Handshake_Master_Spec_v02.103.md:1697`
- Bounds: `Handshake_Master_Spec_v02.103.md:1697` .. `Handshake_Master_Spec_v02.103.md:3138`
- Why: Content integrity is non-negotiable. User content must never be silently censored, redacted, or diluted inside the system. Safety filters apply only at export/display time.
- What: Defines the content preservation rules, the export-only redaction model, and how this maps to Raw/Derived/Display.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (68):
  - `Handshake_Master_Spec_v02.103.md:1720` /// C700-30: In-diary content MUST remain unredacted and undiluted
  - `Handshake_Master_Spec_v02.103.md:1763` /// C700-31: Redaction for export MUST occur outside via Export Guard
  - `Handshake_Master_Spec_v02.103.md:2309` - `source_hash` of the underlying RawContent segment (MUST; used for drift detection and cache correctness).
  - `Handshake_Master_Spec_v02.103.md:2352` - External retrieval is disabled by default unless enabled per project/session; external calls MUST declare provider, query, and what data is being sent.
  - `Handshake_Master_Spec_v02.103.md:2357` Router fallback order MUST check:
  - `Handshake_Master_Spec_v02.103.md:2370` - Evidence snippets SHOULD carry: `trust_class`, `fetch_depth`, and `cached_artifact_ref` (when sourced from cache).
  - `Handshake_Master_Spec_v02.103.md:2371` - LocalWebCacheIndex adapter MUST support: `store(url, content) -> cached_artifact_ref` and `search(query) -> snippets`.
  - `Handshake_Master_Spec_v02.103.md:2374` Cache-to-Index Assimilation SHOULD be implemented as a mechanical job/workflow node invoked by the orchestrator, using workspace-first I/O and full Flight Recorder logging (see Section 6.0 Mechanical Tool Bus & Integration Principles).
  - `Handshake_Master_Spec_v02.103.md:2422` Defines one canonical export pipeline plus the minimum schemas/requirements every exporter MUST follow.
  - `Handshake_Master_Spec_v02.103.md:2443` - Exporters MUST NOT mutate Raw/Derived entities.
  - `Handshake_Master_Spec_v02.103.md:2444` - Exporters MUST be invoked via the Orchestrator/Workflow engine (no ad-hoc \u201csave as\u201d bypass).
  - `Handshake_Master_Spec_v02.103.md:2445` - Exporters MUST be offline-pure at runtime (no network fetches; all inputs must already exist as workspace entities/artifacts).
  - `Handshake_Master_Spec_v02.103.md:2446` - Any export referencing `exportable=false` artifacts MUST be blocked by CloudLeakageGuard (\xa72.6.6.7.11) unless the user explicitly reclassifies and re-runs.
  - `Handshake_Master_Spec_v02.103.md:2478` - **structural**: bytes may differ, but structure is stable; exporter MUST produce a canonical `content_hash` over a normalized form (e.g. ZIP metadata stripped).
  - `Handshake_Master_Spec_v02.103.md:2479` - **best_effort**: exporter cannot guarantee the above; MUST log why in `warnings[]` and still record engine/config hashes.
  - `Handshake_Master_Spec_v02.103.md:2491` All items above MUST emit `ExportRecord` + `ArtifactHandle`s and MUST obey ExportGuard/CloudLeakageGuard.
  - `Handshake_Master_Spec_v02.103.md:2503` Artifacts MUST be first-class workspace objects with:
  - `Handshake_Master_Spec_v02.103.md:2535` - For `determinism_level=bitwise`, `content_hash` MUST match the exact payload bytes.
  - `Handshake_Master_Spec_v02.103.md:2536` - For directory artifacts and for `determinism_level=structural`, exporters MUST define the canonical hash basis (e.g. normalized entry list + per-entry hashes) and log it in `ExportRecord.warnings[]` if not bitwise.
  - `Handshake_Master_Spec_v02.103.md:2543` - `determinism_level` SHOULD be `structural` unless bitwise ZIP determinism is guaranteed.
  - `Handshake_Master_Spec_v02.103.md:2544` - `content_hash` MUST be computed over a canonical `BundleIndex` (sorted paths + per-item content_hash + size_bytes), not over raw ZIP bytes unless bitwise is guaranteed.
  - `Handshake_Master_Spec_v02.103.md:2545` - Bundles MUST include an embedded `bundle_index.json` OR emit it as a sibling artifact referenced by the ExportRecord.
  - `Handshake_Master_Spec_v02.103.md:2551` - `retention_ttl_days` MUST be set for `prompt_payload` and other high-sensitivity artifacts.
  - `Handshake_Master_Spec_v02.103.md:2552` - Expired, unpinned artifacts MUST be garbage-collected.
  - `Handshake_Master_Spec_v02.103.md:2553` - GC MUST be deterministic and auditable (emit a `gc_report` artifact + log record containing deleted artifact_ids + reason).
  - `Handshake_Master_Spec_v02.103.md:2554` - Workspaces SHOULD enforce a size quota; quota evictions MUST never delete pinned artifacts.
  - `Handshake_Master_Spec_v02.103.md:2563` Photo Studio exports MUST follow the unified export contract (\xa72.3.10.1\u2013\xa72.3.10.9).
  - `Handshake_Master_Spec_v02.103.md:2567` - `.hs.export.json` MUST be a **lossless projection** of the authoritative `ExportRecord` (same `export_id`, same artifact hashes, same provenance), and MUST NOT introduce a parallel source of truth.
  - `Handshake_Master_Spec_v02.103.md:2578` - The determinism level recorded in `ExportRecord.determinism_level` MUST match the strictest applicable class across all contributing Photo Studio operations (see \xa76.3.3.6.1).
  - `Handshake_Master_Spec_v02.103.md:2616` 1.  **[HSK-GC-002] Pinning Invariant:** Any artifact or log entry marked `is_pinned: true` (in SQLite metadata or sidecar) MUST be excluded from automated GC runs.
  - `Handshake_Master_Spec_v02.103.md:2617` 2.  **[HSK-GC-003] Audit Trail:** Every GC run MUST emit a `meta.gc_summary` event to the Flight Recorder containing counts of pruned vs. spared items.
  - `Handshake_Master_Spec_v02.103.md:2618` 3.  **[HSK-GC-004] Atomic Materialize:** The `PruneReport` MUST be written as a versioned artifact before old logs are unlinked.
  - `Handshake_Master_Spec_v02.103.md:2628` The Janitor service MUST NOT hold a direct reference to a database pool. It MUST interact with the storage layer exclusively via the `Database` trait or a dedicated `JanitorStorage` interface. This ensures that maintenance tasks remain portable across SQLite and PostgreSQL backends.
  - `Handshake_Master_Spec_v02.103.md:2636` - Materialize MUST be atomic (write temp + fsync + rename) and MUST prevent path traversal.
  - `Handshake_Master_Spec_v02.103.md:2637` - Materialize MUST NOT bypass ExportGuard/CloudLeakageGuard; the exporter pipeline (\xa72.3.10.1) still applies.
  - `Handshake_Master_Spec_v02.103.md:2638` - `ExportRecord.materialized_paths[]` MUST be written for LocalFile targets.
  - `Handshake_Master_Spec_v02.103.md:2909` All database operations MUST flow through a single storage module boundary. No business logic code may directly access database connections.
  - `Handshake_Master_Spec_v02.103.md:2913` - REQUIRED: All DB operations via `state.storage.*` interface
  - `Handshake_Master_Spec_v02.103.md:2914` - REQUIRED: AppState MUST NOT expose raw `SqlitePool` or `DuckDbConnection`
  - `Handshake_Master_Spec_v02.103.md:2926` All migrations MUST be written in DB-agnostic SQL. SQLite-specific syntax is forbidden.
  - `Handshake_Master_Spec_v02.103.md:2928` - FORBIDDEN: `strftime()`, SQLite datetime functions \u2192 REQUIRED: Parameterized timestamps
  - `Handshake_Master_Spec_v02.103.md:2929` - FORBIDDEN: SQLite placeholder syntax `?1`, `?2` \u2192 REQUIRED: Portable syntax `$1`, `$2`
  - `Handshake_Master_Spec_v02.103.md:2930` - FORBIDDEN: SQLite triggers with `OLD`/`NEW` semantics \u2192 REQUIRED: Application-layer mutation tracking
  - `Handshake_Master_Spec_v02.103.md:2931` - REQUIRED: Migrations use version-managed framework (compatible with sqlx::migrate or similar)
  - `Handshake_Master_Spec_v02.103.md:2932` - REQUIRED: Schema definitions are pure DDL (no data transforms)
  - `Handshake_Master_Spec_v02.103.md:2944` Indexes and other optimization artifacts MUST be treated as regenerable, not migrated.
  - `Handshake_Master_Spec_v02.103.md:2946` - REQUIRED: Document which indexes are rebuildable (e.g., search indexes, caches)
  - `Handshake_Master_Spec_v02.103.md:2947` - REQUIRED: For large data migrations, prefer recompute from source artifacts over row-by-row DB migration
  - `Handshake_Master_Spec_v02.103.md:2949` - REQUIRED: Include index rebuild steps in migration documentation
  - `Handshake_Master_Spec_v02.103.md:2958` Even though PostgreSQL is not in Phase 1, test infrastructure MUST be in place to run unit/integration tests against both SQLite and PostgreSQL in CI.
  - `Handshake_Master_Spec_v02.103.md:2960` - REQUIRED: Storage layer tests parameterized for both backends
  - `Handshake_Master_Spec_v02.103.md:2961` - REQUIRED: CI pipeline includes PostgreSQL test variant (can use PostgreSQL in Docker)
  - `Handshake_Master_Spec_v02.103.md:2962` - REQUIRED: New storage features tested against both backends before merge
  - `Handshake_Master_Spec_v02.103.md:2963` - REQUIRED: Failure in either backend (SQLite or PostgreSQL) blocks PR merge
  - `Handshake_Master_Spec_v02.103.md:3006` The storage module MUST define a trait-based interface that hides database differences. This contract is MANDATORY for all storage implementations.
  - `Handshake_Master_Spec_v02.103.md:3009` The `Database` trait MUST NOT expose any methods that return concrete, backend-specific types (e.g., `SqlitePool`, `PgPool`, `DuckDbConnection`). All implementations MUST encapsulate their internal connection pools.
  - `Handshake_Master_Spec_v02.103.md:3011` - **Remediation:** Any service requiring database access (e.g., Janitor, Search) MUST consume the generic `Database` trait methods or be refactored into a trait-compliant operation.
  - `Handshake_Master_Spec_v02.103.md:3057` `AppState` MUST NOT expose concrete database pool types. It MUST use the trait object pattern.
  - `Handshake_Master_Spec_v02.103.md:3062` // \u2705 REQUIRED
  - `Handshake_Master_Spec_v02.103.md:3079` - Any new database-touching feature MUST be implemented as a method on this trait.
  - `Handshake_Master_Spec_v02.103.md:3085` Migrations MUST use a version-managed system compatible with industry standards (sqlx::migrate, Liquibase, Flyway, etc.).
  - `Handshake_Master_Spec_v02.103.md:3087` - REQUIRED: Numbered migration files (0001_, 0002_, ...)
  - `Handshake_Master_Spec_v02.103.md:3088` - REQUIRED: Each migration is idempotent (can run multiple times safely)
  - `Handshake_Master_Spec_v02.103.md:3089` - REQUIRED: Migration rollback supported (down migration optional if not needed)
  - `Handshake_Master_Spec_v02.103.md:3090` - REQUIRED: Schema versioning tracked in database (schema_version table or equivalent)
  - `Handshake_Master_Spec_v02.103.md:3091` - REQUIRED: Migrations tested on both SQLite and PostgreSQL before merge
  - `Handshake_Master_Spec_v02.103.md:3104` - **MANDATORY AUDIT**: The codebase MUST be scanned for `sqlx::` and `SqlitePool` references.
  - `Handshake_Master_Spec_v02.103.md:3125` **Blocking Constraint:** New storage-related work MUST NOT proceed without these four WPs completed.

## 2.4 Extraction Pipeline (The Product)
- Spec: `Handshake_Master_Spec_v02.103.md:3139`
- Bounds: `Handshake_Master_Spec_v02.103.md:3139` .. `Handshake_Master_Spec_v02.103.md:3614`
- Why: This is what Handshake is for. Everything else \u2014 governance, infrastructure, mechanical layers \u2014 exists to make this work reliably. The extraction pipeline turns images and creative content into structured, searchable, learnable descriptors.
- What: Defines the complete extraction pipeline: IMG-001 (image and media extractors), TXT-001 (text/narrative extractors), SYM-001 (symbolic engine), and how they integrate with DES-001 (descriptor schema).
- Roadmap mentions section number: NO
- Normative lines (6):
  - `Handshake_Master_Spec_v02.103.md:3567` All tag-like fields (`ActionTag`, `EmotionTag`, `ThemeTag`, `SexualContextDescriptor`, etc.) **MUST** come from CONFIG vocab tables (TXT-001 vocab), not free-form strings, mirroring DES-001.
  - `Handshake_Master_Spec_v02.103.md:3600` 1. LLMs **MUST NOT** mutate RawContent; they only produce intermediate signals and DerivedContent under TXT-001.
  - `Handshake_Master_Spec_v02.103.md:3601` 2. All LLM outputs that become part of `TextDescriptorRow` **MUST** be mapped through CONFIG vocab tables, not stored as arbitrary free text.
  - `Handshake_Master_Spec_v02.103.md:3602` 3. NSFW and consent rules from Sections 2.2.2\u20132.2.3 **MUST** apply:
  - `Handshake_Master_Spec_v02.103.md:3611` - Segments with `content_tier != sfw` **MUST** have a non-null `consent_class` and `source_kind` (via ConsentBlock).
  - `Handshake_Master_Spec_v02.103.md:3612` - TXT-001 **MUST NOT** downgrade or euphemise sexual context in RawContent or DerivedContent.

## 2.5 AI Interaction Patterns
- Spec: `Handshake_Master_Spec_v02.103.md:3615`
- Bounds: `Handshake_Master_Spec_v02.103.md:3615` .. `Handshake_Master_Spec_v02.103.md:4895`
- Why: Understanding how AI integrates into user workflows\u2014across documents, canvases, and tables\u2014is essential for building coherent UX. This section defines the interaction models that govern all AI-assisted editing and generation.
- What: Describes the AI stack (model roles, runtime topology, routing), the three primary interaction patterns (Command Palette, Structural Editor, Background Agent), and how AI behaves specifically in docs, canvases, tables, and the "Project Brain" RAG interface. Introduces the cyclical "Thinking Pipeline" that moves content between views. **Includes the complete Docs & Sheets AI Job Profile.**
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (81):
  - `Handshake_Master_Spec_v02.103.md:3796` All AI edits to documents MUST be executed as **Docs & Sheets AI jobs** (Section 2.5.10), operating on block and segment IDs and updating provenance (`ai_origin`).
  - `Handshake_Master_Spec_v02.103.md:3878` All AI transforms on tables MUST be executed as **Docs & Sheets AI jobs** (Section 2.5.10), operating on row and column IDs and updating sheet provenance (`ai_source`, overrides).
  - `Handshake_Master_Spec_v02.103.md:3937` - Docs, canvas, workflows, and tables all operate on the same workspace entities (Documents, Blocks, Canvas Nodes, Tables, Tasks) and **MUST** use ID-based references and the Raw/Derived/Display model (Sections 2.2.1\u20132.2.2).
  - `Handshake_Master_Spec_v02.103.md:3957` The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY**
  - `Handshake_Master_Spec_v02.103.md:4032` - It defines contracts that editors and the orchestrator MUST satisfy so that jobs, provenance, and safety are consistent.
  - `Handshake_Master_Spec_v02.103.md:4063` - DOCX export MUST be generated from the AST canonical model (EDIT-AST-001; \xa710.2.1.2).
  - `Handshake_Master_Spec_v02.103.md:4064` - Export MUST NOT write back into stored document content.
  - `Handshake_Master_Spec_v02.103.md:4078` - Import MUST assign stable IDs and record provenance (see \xa72.5.10.3.5).
  - `Handshake_Master_Spec_v02.103.md:4107` - MUST obey ExportGuard + CloudLeakageGuard; MUST emit artifacts + ExportRecord (\xa72.3.10).
  - `Handshake_Master_Spec_v02.103.md:4108` - MUST declare `determinism_level` in ExportRecord.
  - `Handshake_Master_Spec_v02.103.md:4109` - MUST be invoked via the Workflow engine (no direct UI exporter bypass).
  - `Handshake_Master_Spec_v02.103.md:4153` Each job MUST track a `status` from the following set (aligned with the global job lifecycle in \xa72.6.6.3):
  - `Handshake_Master_Spec_v02.103.md:4168` failures or policy violations); implementations MUST NOT auto-retry poisoned
  - `Handshake_Master_Spec_v02.103.md:4173` Implementations MAY add additional status values, but they MUST map cleanly
  - `Handshake_Master_Spec_v02.103.md:4199` - When a user edits an AI-produced cell value, the cell SHOULD be marked
  - `Handshake_Master_Spec_v02.103.md:4200` `override = true` and future automatic \u201creapply AI\u201d operations SHOULD NOT
  - `Handshake_Master_Spec_v02.103.md:4203` Provenance MUST allow implementations to answer:
  - `Handshake_Master_Spec_v02.103.md:4233` IDs SHOULD be assigned once per node and remain stable across text edits that
  - `Handshake_Master_Spec_v02.103.md:4234` preserve identity. When nodes are split or merged, new IDs MUST be created; an
  - `Handshake_Master_Spec_v02.103.md:4241` Each AI job MUST be backed by a configuration document (typically `task.yaml`)
  - `Handshake_Master_Spec_v02.103.md:4244` At minimum, `task.yaml` MUST contain:
  - `Handshake_Master_Spec_v02.103.md:4270` Each job MUST also record:
  - `Handshake_Master_Spec_v02.103.md:4279` Jobs created under an unknown or unsupported `protocol_version` MUST be treated
  - `Handshake_Master_Spec_v02.103.md:4281` versions concurrently, but MUST document which versions are replayable.
  - `Handshake_Master_Spec_v02.103.md:4288` Implementations MUST define a compatibility policy for `task_schema_version`
  - `Handshake_Master_Spec_v02.103.md:4293` - Newer orchestrators SHOULD be able to read and display jobs created with
  - `Handshake_Master_Spec_v02.103.md:4302` - The job MUST be treated as read-only provenance;
  - `Handshake_Master_Spec_v02.103.md:4303` - Re-execution MUST fail deterministically with a clear error; and
  - `Handshake_Master_Spec_v02.103.md:4304` - Implementations SHOULD emit diagnostics so operators can decide whether to
  - `Handshake_Master_Spec_v02.103.md:4307` Unknown fields in `task.yaml` MUST be ignored by default (i.e. treated as
  - `Handshake_Master_Spec_v02.103.md:4316` An entity reference MUST include:
  - `Handshake_Master_Spec_v02.103.md:4338` Implementations MUST:
  - `Handshake_Master_Spec_v02.103.md:4352` - `doc_id`, `sheet_id`, `canvas_id` MUST be unique within a workspace.
  - `Handshake_Master_Spec_v02.103.md:4353` - `block_id`, `segment_id` MUST be unique within a given `doc_id`.
  - `Handshake_Master_Spec_v02.103.md:4354` - `row_id` MUST be unique within a given `sheet_id`.
  - `Handshake_Master_Spec_v02.103.md:4355` - `column_id` MUST be unique within a given `sheet_id`.
  - `Handshake_Master_Spec_v02.103.md:4357` Implementations SHOULD:
  - `Handshake_Master_Spec_v02.103.md:4365` - Implementations MUST assign new IDs to all entities, and
  - `Handshake_Master_Spec_v02.103.md:4368` - Such mappings SHOULD be treated as implementation detail and MAY be
  - `Handshake_Master_Spec_v02.103.md:4372` If an ID collision is detected inside a namespace that MUST be unique:
  - `Handshake_Master_Spec_v02.103.md:4374` - The implementation MUST treat this as a schema error,
  - `Handshake_Master_Spec_v02.103.md:4375` - MUST NOT run AI jobs against the affected artefact until the conflict is
  - `Handshake_Master_Spec_v02.103.md:4377` - SHOULD surface a clear diagnostic to the user or administrator.
  - `Handshake_Master_Spec_v02.103.md:4411` A compliant Word-like editor MUST expose at least:
  - `Handshake_Master_Spec_v02.103.md:4422` internal IDs, but any IDs used in jobs/provenance MUST still satisfy (EntityRef section).
  - `Handshake_Master_Spec_v02.103.md:4434` MUST be defined as jobs where:
  - `Handshake_Master_Spec_v02.103.md:4441` Editors MAY offer richer UX, but MUST route through jobs rather than ad-hoc
  - `Handshake_Master_Spec_v02.103.md:4454` MUST be implemented as jobs that:
  - `Handshake_Master_Spec_v02.103.md:4463` Implementations SHOULD avoid one giant \u201crewrite entire document\u201d job and instead
  - `Handshake_Master_Spec_v02.103.md:4479` - Replay MUST NOT overwrite the old job; it creates a new provenance record.
  - `Handshake_Master_Spec_v02.103.md:4495` - Each accepted job MUST be applied as **one logical transaction per document**
  - `Handshake_Master_Spec_v02.103.md:4499` - The implementation SHOULD record a mapping between `job_id` and any
  - `Handshake_Master_Spec_v02.103.md:4503` Before applying a patch, an implementation SHOULD:
  - `Handshake_Master_Spec_v02.103.md:4521` When a document job is accepted and its patch applied, the implementation MUST:
  - `Handshake_Master_Spec_v02.103.md:4536` If the system has a knowledge graph or Shadow Workspace, it SHOULD:
  - `Handshake_Master_Spec_v02.103.md:4551` Implementations MUST define how concurrent jobs interact when they target
  - `Handshake_Master_Spec_v02.103.md:4556` - Orchestrators SHOULD avoid running multiple `doc_edit` jobs that target the
  - `Handshake_Master_Spec_v02.103.md:4559` touch the same entities, the implementation MUST either:
  - `Handshake_Master_Spec_v02.103.md:4572` Implementations MUST NOT silently drop user edits when applying a patch generated
  - `Handshake_Master_Spec_v02.103.md:4573` against an outdated snapshot; they MUST detect divergence and either rebase or
  - `Handshake_Master_Spec_v02.103.md:4579` - Jobs MUST be durable and observable, not one-off RPCs.
  - `Handshake_Master_Spec_v02.103.md:4580` - There MUST be a clear mapping from job config/inputs to diffs and
  - `Handshake_Master_Spec_v02.103.md:4613` To prevent silent damage, implementations MUST:
  - `Handshake_Master_Spec_v02.103.md:4621` Editors/orchestrators SHOULD default to `preview_only` for wide-impact jobs.
  - `Handshake_Master_Spec_v02.103.md:4628` Validators MUST accept at least:
  - `Handshake_Master_Spec_v02.103.md:4634` Validators MUST return:
  - `Handshake_Master_Spec_v02.103.md:4642` - If any validator returns `deny`, the job MUST NOT be auto-applied. The job
  - `Handshake_Master_Spec_v02.103.md:4643` status MUST be set to `failed` or `completed_with_issues` depending on whether
  - `Handshake_Master_Spec_v02.103.md:4647` SHOULD be surfaced in the UI and recorded in job metadata.
  - `Handshake_Master_Spec_v02.103.md:4648` - When multiple validators run, the effective result MUST be computed as:
  - `Handshake_Master_Spec_v02.103.md:4652` Implementations MAY short-circuit on the first `deny`, but MUST respect this
  - `Handshake_Master_Spec_v02.103.md:4656` validation MUST complete before any changes are committed.
  - `Handshake_Master_Spec_v02.103.md:4662` Implementations SHOULD:
  - `Handshake_Master_Spec_v02.103.md:4692` Not all jobs are appropriate everywhere. Implementations MUST define a capability
  - `Handshake_Master_Spec_v02.103.md:4698` - Each job kind SHOULD have a capability profile describing:
  - `Handshake_Master_Spec_v02.103.md:4721` A cross-artefact job MUST:
  - `Handshake_Master_Spec_v02.103.md:4727` Implementations SHOULD:
  - `Handshake_Master_Spec_v02.103.md:4740` - The job status MUST be set to `completed_with_issues` or `failed`, and
  - `Handshake_Master_Spec_v02.103.md:4741` - Implementations MUST NOT silently present a partially-applied job as if it
  - `Handshake_Master_Spec_v02.103.md:4744` Implementations SHOULD:
  - `Handshake_Master_Spec_v02.103.md:4892` Note: capabilities MUST be resolved and enforced via the global capability/consent model (Section 11.1) and the AI Job lifecycle/validators (Section 2.6.6).

## 2.6 Workflow & Automation Engine
- Spec: `Handshake_Master_Spec_v02.103.md:4896`
- Bounds: `Handshake_Master_Spec_v02.103.md:4896` .. `Handshake_Master_Spec_v02.103.md:6916`
- Why: Automations are how Handshake scales beyond manual AI commands to repeatable, composable pipelines. This section defines the workflow model, execution semantics, and safety constraints that make automations trustworthy.
- What: Specifies the workflow engine's goals, the node-based workflow model (triggers, workspace ops, AI ops, connectors, control flow), AI-assisted workflow design constraints, durable execution with SQLite state, and the validation pipeline for safety. **Includes the global AI Job Model that all artefact profiles inherit from.**
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (146):
  - `Handshake_Master_Spec_v02.103.md:4929` The Workflow Engine MUST persist every node execution, status transition, and input/output payload to the database. A "minimal" async wrapper that only logs start/end events is insufficient.
  - `Handshake_Master_Spec_v02.103.md:4932` Workflows MUST follow a strict state machine: `Queued -> Running -> (Completed | Failed | Cancelled)`. Upon engine restart, the system MUST be able to identify `Running` workflows that were interrupted and mark them as `Stalled` or attempt recovery based on policy.
  - `Handshake_Master_Spec_v02.103.md:4935` Upon system initialization (within the `run()` loop of `main.rs`), the Workflow Engine MUST execute a non-blocking scan for `Running` workflows whose `last_heartbeat` is > 30 seconds old. These MUST be transitioned to `Stalled` and logged to the Flight Recorder with `actor: System` and event type `FR-EVT-WF-RECOVERY`. This recovery MUST occur before the system begins accepting new AI jobs.
  - `Handshake_Master_Spec_v02.103.md:5089` At minimum, this profile MUST run:
  - `Handshake_Master_Spec_v02.103.md:5101` - ContextPacks are **Derived** and MUST be regenerable.
  - `Handshake_Master_Spec_v02.103.md:5102` - Packs SHOULD default to `exportable=false` unless explicitly elevated by policy/consent.
  - `Handshake_Master_Spec_v02.103.md:5194` Jobs **MUST NOT** operate on entities not explicitly listed or derivable from `entity_refs`.
  - `Handshake_Master_Spec_v02.103.md:5256` To satisfy \xa72.6.6.2 core schema requirements, all implementations MUST use the following normative Rust structures.
  - `Handshake_Master_Spec_v02.103.md:5259` `JobKind` MUST be implemented as a Rust `enum`. Fallback to `String` for storage purposes MUST use a validated `FromStr` implementation to prevent illegal states.
  - `Handshake_Master_Spec_v02.103.md:5262` `JobMetrics` MUST NOT contain `NULL` values in the database. A zeroed `JobMetrics` object MUST be created at job initialization.
  - `Handshake_Master_Spec_v02.103.md:5355` Implementations MUST reject any other value at parse time.
  - `Handshake_Master_Spec_v02.103.md:5358` - term_exec MAY be accepted as an alias for terminal_exec, but MUST be normalized to terminal_exec on write.
  - `Handshake_Master_Spec_v02.103.md:5401` | `stalled` | Workflow run lost heartbeat or crash detected; system marks it during recovery scan; MUST emit FR-EVT-WF-RECOVERY with actor=system; no edits committed |
  - `Handshake_Master_Spec_v02.103.md:5408` | `poisoned` | Known to be unsafe to retry; implementations MUST NOT auto-retry |
  - `Handshake_Master_Spec_v02.103.md:5412` - Changes to workspace artefacts MUST only be applied when the job transitions to `completed` or `completed_with_issues`.
  - `Handshake_Master_Spec_v02.103.md:5413` - Failed/cancelled/poisoned jobs MUST NOT commit edits; they may still produce logs and previews.
  - `Handshake_Master_Spec_v02.103.md:5415` - Stalled jobs MUST NOT commit edits; they may be resumed or marked failed/cancelled only via explicit operator action or policy.
  - `Handshake_Master_Spec_v02.103.md:5421` Every AI job MUST be representable as:
  - `Handshake_Master_Spec_v02.103.md:5450` - Every AI-modified entity MUST be traceable back to a `job_id`
  - `Handshake_Master_Spec_v02.103.md:5504` - Flight Recorder MUST log metrics (pass@k, compile/test rates, collapse indicators), reward features, and promotion/rollback decisions.
  - `Handshake_Master_Spec_v02.103.md:5509` - CI MUST lint/validate these schemas to prevent drift; SDK clients must be generated from the same source schemas.
  - `Handshake_Master_Spec_v02.103.md:5510` - Schema changes MUST be versioned and linked to ADRs; breaking changes require migration notes.
  - `Handshake_Master_Spec_v02.103.md:5546` **Invariant:** All `doc_rewrite` and `doc_summarize` jobs MUST include a valid `DocsAiJobProfile` in their `job_inputs` JSON payload.
  - `Handshake_Master_Spec_v02.103.md:5582` - Spec Router jobs MUST emit SpecIntent and SpecRouterDecision artifacts and link them to the job.
  - `Handshake_Master_Spec_v02.103.md:5583` - If `workflow_context.version_control == Git`, the router MUST require a safety commit gate before execution; otherwise it MUST NOT create a safety commit.
  - `Handshake_Master_Spec_v02.103.md:5584` - If the router selects GOV_STRICT or GOV_STANDARD, it MUST create or update a Task Board item and Work Packet and append a SpecSessionLog entry (2.6.8.8).
  - `Handshake_Master_Spec_v02.103.md:5611` **Invariant:** The Catalog MUST be loaded from `assets/semantic_catalog.json` at startup. Catalog resolution MUST filter tools against the user's `CapabilityRegistry` grants.
  - `Handshake_Master_Spec_v02.103.md:5681` - The registry version used for routing MUST be stored in SpecIntent or SpecRouterDecision.
  - `Handshake_Master_Spec_v02.103.md:5700` **Invariant:** Atelier Lens (role claim + glance) is always active for ingestion and spec routing; mode selection MUST NOT disable it except by explicit LAW override.
  - `Handshake_Master_Spec_v02.103.md:5781` If workflow_context.version_control == Git, Spec Router MUST require Version/Repo capabilities; otherwise those capabilities MUST remain disabled.
  - `Handshake_Master_Spec_v02.103.md:5837` Note: SpecCreativeImage MUST route through Atelier Lens (always-on) and MAY emit ConceptRecipe or RoleDeliverableBundle references alongside the spec artifact.
  - `Handshake_Master_Spec_v02.103.md:5940` - Every Task Board or Work Packet change MUST emit a SpecSessionLogEntry stored in the workspace and indexed for RAG.
  - `Handshake_Master_Spec_v02.103.md:5941` - The Spec Session Log MUST NOT replace Flight Recorder; it is a parallel, human-facing ledger.
  - `Handshake_Master_Spec_v02.103.md:5942` - Spec Session Log entries MUST reference the same spec_id and work_packet_id used in SpecIntent and WorkPacketBinding.
  - `Handshake_Master_Spec_v02.103.md:5943` - SpecSessionLogEntry.entry_id MUST be unique within the workspace.
  - `Handshake_Master_Spec_v02.103.md:5944` - SpecSessionLogEntry.governance_mode MUST match the active mode at the time of the event; mode transitions require a dedicated entry.
  - `Handshake_Master_Spec_v02.103.md:5961` The system MUST provide a unified `TokenizationService` to ensure budget compliance across different model architectures.
  - `Handshake_Master_Spec_v02.103.md:5966` /// MUST NOT split words on whitespace for BPE models (Llama3/Mistral).
  - `Handshake_Master_Spec_v02.103.md:5969` /// MUST be emitted via a non-blocking mechanism (e.g., `tokio::spawn`, a background channel,
  - `Handshake_Master_Spec_v02.103.md:6014` - Jobs MUST NOT write RawContent unless the job_kind explicitly allows ingestion/import and passes capability/consent gates.
  - `Handshake_Master_Spec_v02.103.md:6043` - Any seed used for strict determinism MUST be recorded in ContextSnapshot; `scope_inputs_hash` MUST be logged.
  - `Handshake_Master_Spec_v02.103.md:6057` The active mode MUST be recorded in the `ContextSnapshot`.
  - `Handshake_Master_Spec_v02.103.md:6076` - If `kind=artifact`, `hash` MUST be present.
  - `Handshake_Master_Spec_v02.103.md:6077` - `selector` MUST be bounded (no \u201centire artifact\u201d selectors).
  - `Handshake_Master_Spec_v02.103.md:6078` - SourceRefs MUST be sufficient to fetch evidence locally.
  - `Handshake_Master_Spec_v02.103.md:6111` - `ContextSnapshot` MUST be sufficient to answer \u201cwhat did it see and why\u201d via hashes + SourceRefs.
  - `Handshake_Master_Spec_v02.103.md:6112` - Full prompt text SHOULD be stored locally only via `local_only_payload_ref` and MUST NOT be exported to cloud logs by default.
  - `Handshake_Master_Spec_v02.103.md:6148` - StablePrefix MUST remain stable across many calls to maximize caching.
  - `Handshake_Master_Spec_v02.103.md:6149` - VariableSuffix MUST contain only deltas: user input, tool deltas, retrieved snippets, scope hints.
  - `Handshake_Master_Spec_v02.103.md:6150` - Untrusted external text (web/email/calendar) MUST NOT enter StablePrefix.
  - `Handshake_Master_Spec_v02.103.md:6159` - Canonical serialization MUST use JSON with:
  - `Handshake_Master_Spec_v02.103.md:6196` All retrieved evidence MUST be tagged with a `fetch_depth`:
  - `Handshake_Master_Spec_v02.103.md:6202` Retrieval adapters MUST support:
  - `Handshake_Master_Spec_v02.103.md:6204` - `read(source_id/url, selector) \u2192 excerpt` (bounded to the selector; MUST NOT default to full-page)
  - `Handshake_Master_Spec_v02.103.md:6223` - Each selected snippet MUST include a short relevance rationale.
  - `Handshake_Master_Spec_v02.103.md:6231` - In `strict` determinism mode: ranking/selection and escalation decisions MUST be deterministic (stable tie-breakers).
  - `Handshake_Master_Spec_v02.103.md:6233` - Every retrieved excerpt MUST carry bounded selectors/SourceRefs; no \u201centire artifact\u201d selectors.
  - `Handshake_Master_Spec_v02.103.md:6243` - MUST capture retrieval candidates/selection hashes when in `replay` determinism.
  - `Handshake_Master_Spec_v02.103.md:6260` - SessionLog \u2192 LongTermMemory promotion MUST be an explicit job step and MUST be validator-gated (\xa72.6.6.7.11) with provenance preserved.
  - `Handshake_Master_Spec_v02.103.md:6273` - Vector/graph indices are **Derived state** and MUST be rebuildable from canonical MemoryItems + provenance pointers.
  - `Handshake_Master_Spec_v02.103.md:6277` - Retrieval MUST be bounded (top\u2011k) and determinism rules apply (strict vs replay).
  - `Handshake_Master_Spec_v02.103.md:6289` - Memory relations MUST be stored as typed nodes/edges inside the existing workspace Knowledge Graph (KG) used elsewhere in Handshake.
  - `Handshake_Master_Spec_v02.103.md:6291` - Graph traversal MUST be bounded (fan\u2011out + depth) for determinism and context safety.
  - `Handshake_Master_Spec_v02.103.md:6310` - In `strict` mode: ranking and tie-breakers MUST be deterministic.
  - `Handshake_Master_Spec_v02.103.md:6315` - Retrieval outputs MUST remain artifact/SourceRef based (no raw graph dumps into prompts).
  - `Handshake_Master_Spec_v02.103.md:6347` - Tool outputs above an inline limit MUST be written to artifacts.
  - `Handshake_Master_Spec_v02.103.md:6348` - Debug logs MUST NOT enter prompts.
  - `Handshake_Master_Spec_v02.103.md:6349` - Any \u201cfetch more\u201d operation MUST be bounded by a selector and may produce a new artifact handle.
  - `Handshake_Master_Spec_v02.103.md:6375` - every compact item MUST include SourceRefs to spans and artifacts.
  - `Handshake_Master_Spec_v02.103.md:6381` Each model call MUST compile context in this order:
  - `Handshake_Master_Spec_v02.103.md:6404` - verifier outputs MUST cite evidence refs (SourceRef / artifact handles)
  - `Handshake_Master_Spec_v02.103.md:6410` The runtime MUST provide validators that reject violations:
  - `Handshake_Master_Spec_v02.103.md:6415` Validators MUST NOT operate on hashes or handles alone. For `PromptInjectionGuard` and `CloudLeakageGuard`, the runtime MUST resolve and provide the **raw UTF-8 content** of all `retrieved_snippet` blocks to the validator.
  - `Handshake_Master_Spec_v02.103.md:6418` The `WorkflowEngine` MUST implement a global trap for `AceError::PromptInjectionDetected`. Upon detection:
  - `Handshake_Master_Spec_v02.103.md:6425` Scanning for injection patterns MUST be performed on **NFC-normalized, case-folded** text to prevent bypasses via homoglyphs or casing tricks.
  - `Handshake_Master_Spec_v02.103.md:6428` - **Requirement:** If `determinism_mode` is `strict`, the guard MUST verify a non-null `seed` exists in `ContextSnapshot`.
  - `Handshake_Master_Spec_v02.103.md:6429` - **Requirement:** If mode is `replay`, the guard MUST verify `retrieval_candidates.ids_hash` matches the hash of the persisted candidate list.
  - `Handshake_Master_Spec_v02.103.md:6432` - **Requirement:** The guard MUST enforce a `tool_delta_inline_char_limit` (default: 2000).
  - `Handshake_Master_Spec_v02.103.md:6433` - **Requirement:** Any `tool_delta` exceeding this limit MUST be rejected unless offloaded to an `ArtifactHandle` with a valid `content_hash`.
  - `Handshake_Master_Spec_v02.103.md:6436` - **Requirement:** Every `Decision` block MUST contain at least one `SourceRef` in `evidence_refs`.
  - `Handshake_Master_Spec_v02.103.md:6437` - **Requirement:** Every `Constraint` block MUST map to a `LAW` or `RID` anchor.
  - `Handshake_Master_Spec_v02.103.md:6440` - **Requirement:** MUST reject promotion of `SessionLog` to `LongTermMemory` if the associated `ValidationResult` is absent or `Fail`.
  - `Handshake_Master_Spec_v02.103.md:6443` - **Requirement:** If `model_tier` is `Cloud`, the guard MUST scan all `artifact_handles` and `SourceRefs`.
  - `Handshake_Master_Spec_v02.103.md:6444` - **Requirement:** MUST block the call if any item has `exportable: false` or a `high` sensitivity classification.
  - `Handshake_Master_Spec_v02.103.md:6445` - **Requirement:** If a `SourceRef` points to a `bundle` or `dataset_slice`, the guard MUST check the classification of **every individual member** within that collection (Recursive Check).
  - `Handshake_Master_Spec_v02.103.md:6448` - **Requirement:** MUST execute a substring scan on the resolved, **NFC-normalized** content of all `retrieved_snippet` blocks.
  - `Handshake_Master_Spec_v02.103.md:6449` - **Requirement:** Scan MUST include patterns: `[ "ignore previous", "new instructions", "system command", "developer mode" ]` and any profile-specific patterns.
  - `Handshake_Master_Spec_v02.103.md:6450` - **Requirement:** Detection MUST trigger the **[HSK-ACE-VAL-101] Atomic Poisoning Directive**.
  - `Handshake_Master_Spec_v02.103.md:6453` - **Requirement:** MUST verify that `policy_profile_id`, `model_tier`, and `layer_scope` have not changed since the initial `AIJob` creation.
  - `Handshake_Master_Spec_v02.103.md:6456` - **Requirement:** If `local_only_payload_ref` is present, the guard MUST verify the artifact URI points to the `/encrypted/` storage volume.
  - `Handshake_Master_Spec_v02.103.md:6459` - **Requirement:** MUST enforce evidence budgets and truncation flags defined in `QueryPlan`.
  - `Handshake_Master_Spec_v02.103.md:6462` - **Requirement:** MUST detect stale `ContextPacks` via `source_hashes` mismatch and trigger regeneration or downgrade.
  - `Handshake_Master_Spec_v02.103.md:6465` - **Requirement:** MUST detect embedding/KG drift and fail or degrade per active policy.
  - `Handshake_Master_Spec_v02.103.md:6468` - **Requirement:** MUST verify cache key integrity for cacheable stages to ensure replay correctness.
  - `Handshake_Master_Spec_v02.103.md:6506` - artifact kind MUST be `prompt_payload`
  - `Handshake_Master_Spec_v02.103.md:6509` - MUST NOT sync/replicate to cloud unless user explicitly enables export for that artifact
  - `Handshake_Master_Spec_v02.103.md:6510` - SHOULD have a finite retention TTL by default
  - `Handshake_Master_Spec_v02.103.md:6524` The key words **MUST**, **SHOULD**, **MAY** are to be interpreted as requirement levels.
  - `Handshake_Master_Spec_v02.103.md:6559` - **Strict mode:** retrieval MUST be deterministic (or deterministic approximation with fixed seed/settings).
  - `Handshake_Master_Spec_v02.103.md:6668` 1. For any retrieval-backed model call, the runtime MUST produce a `QueryPlan` before candidate generation.
  - `Handshake_Master_Spec_v02.103.md:6669` 2. `QueryPlan.route[]` MUST be derived from:
  - `Handshake_Master_Spec_v02.103.md:6674` 3. If the runtime cannot produce a plan, the call MUST fail with a surfaced error (not silently proceed).
  - `Handshake_Master_Spec_v02.103.md:6677` The runtime MUST compute `normalized_query_hash = sha256(normalize(query_text))`, where `normalize()`:
  - `Handshake_Master_Spec_v02.103.md:6685` Given `QueryPlan.route[]`, the runtime MUST execute store steps in order until one of:
  - `Handshake_Master_Spec_v02.103.md:6699` - The runtime MUST NOT fetch new external web content unless `filters.allow_external_fetch=true` and policy/capability allows it.
  - `Handshake_Master_Spec_v02.103.md:6700` - The runtime MUST NOT include raw unbounded content in prompts; all reads MUST be bounded.
  - `Handshake_Master_Spec_v02.103.md:6709` - `pack_score` MUST be:
  - `Handshake_Master_Spec_v02.103.md:6712` - `trust_adjust` MUST be derived deterministically from `trust_min` and evidence metadata; at minimum:
  - `Handshake_Master_Spec_v02.103.md:6713` - if candidate trust_class < trust_min => candidate MUST be dropped
  - `Handshake_Master_Spec_v02.103.md:6717` - Candidates MUST be stable-sorted by:
  - `Handshake_Master_Spec_v02.103.md:6724` - If reranker cannot guarantee determinism, reranking MUST be disabled.
  - `Handshake_Master_Spec_v02.103.md:6726` - Any reranking method MAY be used, but the runtime MUST persist:
  - `Handshake_Master_Spec_v02.103.md:6730` - Replay MUST re-use the persisted rerank order (do not recompute).
  - `Handshake_Master_Spec_v02.103.md:6733` The runtime MUST enforce:
  - `Handshake_Master_Spec_v02.103.md:6737` If `diversity.used=true`, the runtime MUST use a deterministic diversity method:
  - `Handshake_Master_Spec_v02.103.md:6739` - Similarity function and `lambda` MUST be logged in `RetrievalTrace`.
  - `Handshake_Master_Spec_v02.103.md:6743` - The runtime MUST extract a bounded span:
  - `Handshake_Master_Spec_v02.103.md:6746` - The extracted span MUST obey `max_read_tokens`.
  - `Handshake_Master_Spec_v02.103.md:6747` - If a span would exceed `max_read_tokens`, the runtime MUST truncate deterministically and set a truncation flag.
  - `Handshake_Master_Spec_v02.103.md:6750` The final evidence set inserted into the PromptEnvelope MUST be:
  - `Handshake_Master_Spec_v02.103.md:6767` - `ContextPackRecord.source_hashes[]` MUST include the hashes of the underlying sources at build time.
  - `Handshake_Master_Spec_v02.103.md:6769` - Stale packs MUST NOT be treated as pack_score=1.0. The runtime MUST either:
  - `Handshake_Master_Spec_v02.103.md:6774` - Every `fact`, `constraint`, and `open_loop` MUST include `source_refs[]`.
  - `Handshake_Master_Spec_v02.103.md:6775` - A pack item without SourceRefs MUST be dropped or marked `confidence=0` and MUST NOT be promoted to LongTermMemory.
  - `Handshake_Master_Spec_v02.103.md:6782` - The runtime MUST provide a SemanticCatalog store/query interface, populated from:
  - `Handshake_Master_Spec_v02.103.md:6786` - The planner SHOULD consult SemanticCatalog first to avoid \u201cguessing\u201d which store/tool to use.
  - `Handshake_Master_Spec_v02.103.md:6787` - SemanticCatalog entries MUST be versioned and timestamped.
  - `Handshake_Master_Spec_v02.103.md:6790` - Catalog queries MUST be capability gated; catalog MUST NOT reveal selectors/paths outside granted scope.
  - `Handshake_Master_Spec_v02.103.md:6810` - Cache lookup MUST be performed before executing expensive retrieval/rerank/span steps.
  - `Handshake_Master_Spec_v02.103.md:6811` - Any cache hit MUST be recorded in `RetrievalTrace.route_taken[].cache_hit=true`.
  - `Handshake_Master_Spec_v02.103.md:6812` - Cache invalidation MUST occur if any field in CacheKey changes.
  - `Handshake_Master_Spec_v02.103.md:6813` - In `replay` mode, cached artifacts MUST NOT replace recorded candidate lists unless the cached payload hash matches the recorded one.
  - `Handshake_Master_Spec_v02.103.md:6819` The runtime MUST detect and surface drift for derived indices:
  - `Handshake_Master_Spec_v02.103.md:6822` - Any embedding/snippet record MUST be keyed to a `source_hash`.
  - `Handshake_Master_Spec_v02.103.md:6823` - If `source_hash` mismatch is detected, candidate MUST be downgraded or dropped and a rebuild MUST be scheduled/available.
  - `Handshake_Master_Spec_v02.103.md:6826` - Any KG-derived candidate used as evidence MUST have provenance pointers to SourceRefs or underlying entities.
  - `Handshake_Master_Spec_v02.103.md:6827` - If provenance is missing, candidate MUST NOT be used as evidence.
  - `Handshake_Master_Spec_v02.103.md:6830` - Cached sources MUST respect TTL and pinning rules.
  - `Handshake_Master_Spec_v02.103.md:6837` The runtime MUST implement the `AceRuntimeValidator` trait. All retrieval operations MUST be validated by a pipeline of these guards.
  - `Handshake_Master_Spec_v02.103.md:6868` - If any selected evidence item has source_hash mismatch (embedding drift) or missing provenance (KG drift), the job MUST:
  - `Handshake_Master_Spec_v02.103.md:6879` For each retrieval-backed model call, the runtime MUST log to Flight Recorder:
  - `Handshake_Master_Spec_v02.103.md:6894` - Same input string variations (whitespace/unicode) MUST yield identical normalized_query_hash.
  - `Handshake_Master_Spec_v02.103.md:6897` - Under strict mode, identical inputs MUST yield identical candidate order and selection, including tie-break behavior.
  - `Handshake_Master_Spec_v02.103.md:6900` - Under replay mode, replay MUST re-use persisted candidate list + rerank order and produce identical selected ids/hashes.
  - `Handshake_Master_Spec_v02.103.md:6903` - If any underlying source_hash changes, a previously built pack MUST be marked stale and MUST NOT receive pack_score=1.0.
  - `Handshake_Master_Spec_v02.103.md:6906` - Evidence token ceilings and per-source caps MUST never be exceeded; truncation MUST be deterministic and logged.

## 2.7 Response Behavior Contract (Diary ANS-001)
- Spec: `Handshake_Master_Spec_v02.103.md:6917`
- Bounds: `Handshake_Master_Spec_v02.103.md:6917` .. `Handshake_Master_Spec_v02.103.md:7613`
- Why: An assistant that just answers questions is a search engine. A governed assistant proactively shows intent understanding, risks, conflicts, better alternatives, and next steps \u2014 without being asked every time. This is the behavioral DNA that makes AI collaboration trustworthy.
- What: Defines the required behaviors for every governed response: what the assistant must show, when to show it, and how work modes affect behavior.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (7):
  - `Handshake_Master_Spec_v02.103.md:6935` Every governed response MUST include these behaviors (format is implementation-specific):
  - `Handshake_Master_Spec_v02.103.md:6941` #[clause("A001-81", "MUST address all explicit questions")]
  - `Handshake_Master_Spec_v02.103.md:6967` #[clause("A001-81", "MUST answer all explicit questions")]
  - `Handshake_Master_Spec_v02.103.md:6968` #[clause("A001-82", "MUST NOT skip interrogatives")]
  - `Handshake_Master_Spec_v02.103.md:6970` #[clause("A001-86", "MUST NOT request clarification when task is executable")]
  - `Handshake_Master_Spec_v02.103.md:7008` #[clause("A001-87", "MUST reflect current understanding")]
  - `Handshake_Master_Spec_v02.103.md:7009` #[clause("A001-88", "MUST NOT silently broaden scope")]

## 2.8 Governance Runtime (Diary Parts 1-2)
- Spec: `Handshake_Master_Spec_v02.103.md:7614`
- Bounds: `Handshake_Master_Spec_v02.103.md:7614` .. `Handshake_Master_Spec_v02.103.md:8895`
- Why: The Diary's Bootloader and Execution Charter define how governance activates and behaves at runtime. These become the lifecycle rules and capability assignments in Handshake.
- What: Defines runtime behavior (when governance activates, what governs what), LAW vs behavior separation, activation triggers, capability precedence, and all clause-level implementation details.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 2.9 Deterministic Edit Process (COR-701)
- Spec: `Handshake_Master_Spec_v02.103.md:8896`
- Bounds: `Handshake_Master_Spec_v02.103.md:8896` .. `Handshake_Master_Spec_v02.103.md:9485`
- Why: Edits to governed content must be deterministic, verifiable, and reversible. COR-701 ensures that every edit produces a provable audit trail, prevents corruption through race conditions, and enables safe rollback on failure.
- What: Defines the micro-step execution model, gate pipeline, manifest structure, and assistant behavior requirements for all edit operations.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (13):
  - `Handshake_Master_Spec_v02.103.md:8964` To satisfy the traceability invariant (\xa77.6.3.8), every mutation to `RawContent` (e.g., document blocks) MUST persist metadata identifying the source of the change.
  - `Handshake_Master_Spec_v02.103.md:8983` 1. **Storage Requirement:** Database tables for `blocks`, `cells`, and `nodes` MUST include `last_actor`, `last_job_id`, and `last_workflow_id` columns.
  - `Handshake_Master_Spec_v02.103.md:8984` 2. **Audit Invariant:** Any row where `last_actor == 'AI'` MUST have a non-null `last_job_id` referencing a valid AI Job.
  - `Handshake_Master_Spec_v02.103.md:8985` 3. **Silent Edit Block:** The storage guard (\xa7WP-1-Global-Silent-Edit-Guard) MUST verify that `MutationMetadata` is present and valid for all AI-authored writes.
  - `Handshake_Master_Spec_v02.103.md:8989` To support the `MutationMetadata` struct, the following columns MUST be added to all content tables (`blocks`, `canvas_nodes`, `canvas_edges`, `workspaces`, `documents`):
  - `Handshake_Master_Spec_v02.103.md:8995` | `last_job_id` | `TEXT` | YES | UUID of the AI Job (REQUIRED if kind='AI') |
  - `Handshake_Master_Spec_v02.103.md:8996` | `last_workflow_id` | `TEXT` | YES | UUID of the parent Workflow (REQUIRED if kind='AI') |
  - `Handshake_Master_Spec_v02.103.md:8999` **Invariant:** A database check constraint (or strict application logic) MUST enforce:
  - `Handshake_Master_Spec_v02.103.md:9004` The application MUST implement the `StorageGuard` trait for all persistence operations. This trait acts as the final gate against silent edits.
  - `Handshake_Master_Spec_v02.103.md:9012` /// - Ok(MutationMetadata): If allowed. Metadata MUST be returned for DB insertion.
  - `Handshake_Master_Spec_v02.103.md:9026` 1.  **AI Write Context:** If `actor == WriteActor::Ai`, the guard MUST fail if `job_id` is `None`.
  - `Handshake_Master_Spec_v02.103.md:9027` 2.  **Traceability Anchor:** The guard MUST generate a unique `edit_event_id` (UUID) for every successful validation and return it in `MutationMetadata`.
  - `Handshake_Master_Spec_v02.103.md:9031` All database persistence methods in the `Database` trait (e.g., `save_blocks`, `update_canvas`) MUST call `validate_write` and persist the returned `MutationMetadata` fields.

## 2.10 Session Logging (LOG-001)
- Spec: `Handshake_Master_Spec_v02.103.md:9486`
- Bounds: `Handshake_Master_Spec_v02.103.md:9486` .. `Handshake_Master_Spec_v02.103.md:9812`
- Why: LOG-001 defines the session state model and logging hygiene that powers the Flight Recorder. Without structured session state, audit trails become unqueryable noise.
- What: Defines SessionState fields, hygiene rules, and Task Ledger schema that make the Flight Recorder useful for debugging, compliance, and recovery.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 3.1 Local-First Data Fundamentals
- Spec: `Handshake_Master_Spec_v02.103.md:9813`
- Bounds: `Handshake_Master_Spec_v02.103.md:9813` .. `Handshake_Master_Spec_v02.103.md:10017`
- Why: Local-first is a core principle, not just a feature. Understanding what it means technically prevents design mistakes that would compromise user sovereignty.
- What: Explains what "local-first" really means, why concurrent editing is hard, how CRDTs solve it, and what CRDTs don't solve.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 3.2 CRDT Libraries Comparison
- Spec: `Handshake_Master_Spec_v02.103.md:10018`
- Bounds: `Handshake_Master_Spec_v02.103.md:10018` .. `Handshake_Master_Spec_v02.103.md:10217`
- Why: Choosing the right CRDT library affects performance, features, and ecosystem. This comparison helps make an informed decision.
- What: Deep dives into Yjs, Automerge, and Loro with pros/cons and recommendations.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 3.3 Database & Sync Patterns
- Spec: `Handshake_Master_Spec_v02.103.md:10218`
- Bounds: `Handshake_Master_Spec_v02.103.md:10218` .. `Handshake_Master_Spec_v02.103.md:10449`
- Why: Understanding how CRDTs integrate with databases enables efficient local storage and sync.
- What: Covers SQLite integration, combining CRDT and database, and sync topologies.
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 3.4 Conflict Resolution UX
- Spec: `Handshake_Master_Spec_v02.103.md:10450`
- Bounds: `Handshake_Master_Spec_v02.103.md:10450` .. `Handshake_Master_Spec_v02.103.md:10579`
- Why: Even with CRDTs, users sometimes need to understand what changed. Good conflict UX builds trust.
- What: Patterns for showing sync status, version history, and when to surface conflicts to users.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 4.1 LLM Infrastructure
- Spec: `Handshake_Master_Spec_v02.103.md:10580`
- Bounds: `Handshake_Master_Spec_v02.103.md:10580` .. `Handshake_Master_Spec_v02.103.md:10785`
- Why: Running AI models locally requires understanding how they work, how much resource they consume, and what trade-offs exist. This section provides the foundational knowledge for all model-related decisions.
- What: Explains how LLMs work at a practical level (parameters, inference vs training), key concepts (tokens, context windows, quantization, GGUF format), and sizing guidance for what fits on a 24GB RTX 3090.
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 4.2 LLM Inference Runtimes
- Spec: `Handshake_Master_Spec_v02.103.md:10786`
- Bounds: `Handshake_Master_Spec_v02.103.md:10786` .. `Handshake_Master_Spec_v02.103.md:11116`
- Why: The runtime software determines how efficiently models execute, how many requests can be handled concurrently, and how easily models can be managed. This section guides runtime selection.
- What: Defines what an inference runtime does, compares major options (Ollama, vLLM, TGI, LM Studio, llamafile, llama.cpp), and recommends a phased strategy starting with Ollama for development.
- Roadmap mentions section number: NO
- Normative lines (8):
  - `Handshake_Master_Spec_v02.103.md:10896` To satisfy the **Single Client Invariant [CX-101]**, all application code MUST interact with LLMs through the `LlmClient` trait. This ensures provider portability and centralized observability.
  - `Handshake_Master_Spec_v02.103.md:10919` pub trace_id: Uuid,          // REQUIRED: non-nil
  - `Handshake_Master_Spec_v02.103.md:10954` To satisfy the traceability and observability requirements, every LLM completion MUST be attributable to a non-nil `trace_id`.
  - `Handshake_Master_Spec_v02.103.md:10956` Normative requirement: the LLM completion request MUST include `trace_id` used for Flight Recorder correlation.
  - `Handshake_Master_Spec_v02.103.md:10961` pub trace_id: Uuid,          // REQUIRED: non-nil
  - `Handshake_Master_Spec_v02.103.md:10972` 1.  **Ollama Adapter:** The primary implementation for Phase 1 MUST use the Ollama API.
  - `Handshake_Master_Spec_v02.103.md:10973` 2.  **Budget Enforcement:** The client MUST enforce `max_tokens` and return `BudgetExceeded` if the provider exceeds the limit.
  - `Handshake_Master_Spec_v02.103.md:10974` 3.  **Observability:** Every call MUST emit a Flight Recorder event (\xa711.5) containing `trace_id`, `model_id`, and `TokenUsage`.

## 4.3 Model Selection & Roles
- Spec: `Handshake_Master_Spec_v02.103.md:11117`
- Bounds: `Handshake_Master_Spec_v02.103.md:11117` .. `Handshake_Master_Spec_v02.103.md:11615`
- Why: Using specialized models for specific tasks outperforms one large generalist, especially on constrained hardware. This section guides model selection for each role.
- What: Explains why specialized models beat generalists, defines role categories (orchestrator, code, creative, utility), recommends specific models for each role, and covers GPU memory management and scheduling strategies.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 4.4 Image Generation (Stable Diffusion)
- Spec: `Handshake_Master_Spec_v02.103.md:11616`
- Bounds: `Handshake_Master_Spec_v02.103.md:11616` .. `Handshake_Master_Spec_v02.103.md:11761`
- Why: Image generation is a key capability for creative workflows. This section covers how to integrate Stable Diffusion alongside LLM workloads without resource conflicts.
- What: Compares SD 1.5 vs SDXL (speed, quality, VRAM), details VRAM requirements and performance, and provides strategies for integrating image generation with LLM workloads.
- Roadmap mentions section number: NO
- Normative lines (8):
  - `Handshake_Master_Spec_v02.103.md:11748` For AI-autonomous operation, token counts MUST be accurate to ensure budget enforcement and billing (where applicable).
  - `Handshake_Master_Spec_v02.103.md:11750` 1. **No String-Split Approximation:** Implementations MUST NOT use whitespace splitting for token counts in production.
  - `Handshake_Master_Spec_v02.103.md:11752` - **GPT-class:** MUST use `tiktoken` or compatible BPE tokenizer.
  - `Handshake_Master_Spec_v02.103.md:11753` - **Llama/Mistral (Ollama):** MUST fetch the tokenizer configuration from the local runtime (e.g. `/api/show` in Ollama) and use the correct tokenizer (SentencePiece/Tiktoken).
  - `Handshake_Master_Spec_v02.103.md:11754` 3. **Vibe Tokenizer (Fallback):** If a model-specific tokenizer is unavailable, the system MUST fallback to a "Vibe Tokenizer" which uses a `char_count / 4.0` heuristic.
  - `Handshake_Master_Spec_v02.103.md:11755` - **Audit Trail:** Vibe Tokenizer usage MUST emit a `metric.accuracy_warning` to the Flight Recorder.
  - `Handshake_Master_Spec_v02.103.md:11756` - **Sync/Async Bridge:** Because `count_tokens` is synchronous, this emission MUST be decoupled from the execution flow (e.g., via fire-and-forget `tokio::spawn` or a dedicated telemetry channel). It MUST NOT block the tokenization logic.
  - `Handshake_Master_Spec_v02.103.md:11757` 4. **Consistency Invariant:** Token counts emitted to `JobMetrics` (\xa72.6.6.2.7) MUST match the counts used for retrieval budgeting (\xa72.6.6.7.14).

## 5.1 Plugin Architecture
- Spec: `Handshake_Master_Spec_v02.103.md:11762`
- Bounds: `Handshake_Master_Spec_v02.103.md:11762` .. `Handshake_Master_Spec_v02.103.md:12109`
- Why: Plugins transform a static application into a living platform. Understanding plugin architecture patterns\u2014both successful and cautionary\u2014informs how to build extensibility that balances power with safety.
- What: Analyzes existing plugin systems (VS Code, Figma, Browser Extensions, Obsidian), designs manifest format with permission declarations, defines plugin types (automation, UI, AI tool), and specifies API patterns for registration and workspace access. See Sections 10/11 for surface-specific hooks (Terminal/Monaco) and shared capability/sandbox/diagnostics contracts plugins must honor.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 5.2 Sandboxing & Security
- Spec: `Handshake_Master_Spec_v02.103.md:12110`
- Bounds: `Handshake_Master_Spec_v02.103.md:12110` .. `Handshake_Master_Spec_v02.103.md:12421`
- Why: Plugins are a major attack vector. Without sandboxing, any plugin can read files, steal data, or install malware. This section specifies how to run untrusted code safely.
- What: Explains why sandboxing is essential, compares sandboxing technologies (WASM, Pyodide, OS subprocess, containers), defines permission categories (filesystem, network, AI, workspace), and recommends a phased security architecture. Cross-ref: Section 11.2 defines policy vs hard isolation defaults; Section 10.1 documents terminal sandbox expectations.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.6 Phase 4 \u2014 Collaboration & Extension Ecosystem
- Normative lines (0): (none detected by keyword scan)

## 5.3 AI Observability
- Spec: `Handshake_Master_Spec_v02.103.md:12422`
- Bounds: `Handshake_Master_Spec_v02.103.md:12422` .. `Handshake_Master_Spec_v02.103.md:12739`
- Why: AI systems are probabilistic\u2014the same input can produce different outputs. Traditional debugging doesn't apply. This section defines what to monitor and how to debug AI behavior.
- What: Explains why AI needs different observability, defines key metrics (performance, resource, quality, cost), compares tools (OpenTelemetry + Prometheus vs Langfuse vs LangSmith), covers privacy-sensitive logging, and provides dashboard/instrumentation examples. See Sections 10/11 for terminal/editor Flight Recorder events, diagnostics schema, and capability-linked logging policies.
- Roadmap mentions section number: NO
- Normative lines (1):
  - `Handshake_Master_Spec_v02.103.md:12730` - Distillation jobs MUST emit Flight Recorder events for each stage (select, teacher run, student run, score, checkpoint, eval, promote/rollback) with trace IDs.

## 5.4 Evaluation & Quality
- Spec: `Handshake_Master_Spec_v02.103.md:12740`
- Bounds: `Handshake_Master_Spec_v02.103.md:12740` .. `Handshake_Master_Spec_v02.103.md:13177`
- Why: LLM outputs are non-deterministic\u2014traditional unit tests with exact expected values don't work. This section defines testing strategies for AI systems.
- What: Addresses the challenge of testing non-deterministic outputs, introduces testing strategies (golden test suites, property-based tests, LLM-as-judge), and covers multi-agent tracing for complex workflows.
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 5.5 Benchmark Harness
- Spec: `Handshake_Master_Spec_v02.103.md:13178`
- Bounds: `Handshake_Master_Spec_v02.103.md:13178` .. `Handshake_Master_Spec_v02.103.md:13477`
- Why: Reproducible performance testing enables informed decisions about runtimes, models, and configurations. This section specifies a systematic benchmarking system.
- What: Describes benchmark harness architecture (config files, adapters, runners, output), provides example configurations and adapter interface, and shows reporting format for comparing runtimes/models.
- Roadmap mentions section number: YES
- Normative lines (6):
  - `Handshake_Master_Spec_v02.103.md:13201` - **RBC is view-only**: UI may render calendar state, but MUST NOT write to calendar tables directly.
  - `Handshake_Master_Spec_v02.103.md:13261` - All JSON artifacts MUST validate against versioned JSON Schema
  - `Handshake_Master_Spec_v02.103.md:13262` - Schema evolution MUST maintain backward compatibility
  - `Handshake_Master_Spec_v02.103.md:13263` - Schema files MUST be versioned in repository
  - `Handshake_Master_Spec_v02.103.md:13475` - Benchmarks MUST include: RAW decode, develop render, proxy pyramid generation, masking, export, and (if enabled) vision/LLM tagging.
  - `Handshake_Master_Spec_v02.103.md:13476` - Scenarios MUST be executed with fixed engine versions and recorded determinism class; any engine version change MUST trigger re-baselining (see \xa75.4.7.3).

## 6.0 Mechanical Tool Bus & Integration Principles
- Spec: `Handshake_Master_Spec_v02.103.md:13478`
- Bounds: `Handshake_Master_Spec_v02.103.md:13478` .. `Handshake_Master_Spec_v02.103.md:13524`
- Purpose (no explicit Why/What blocks): This section defines how all "mechanical" tools \u2013 document parsers, OCR/ASR engines, format converters, and similar subsystems \u2013 plug into Handshake as part of one tool bus instead of isolated pipelines. - Document ingestion and layout-aware parsing (Docling; Section 6.1).
- Roadmap mentions section number: NO
- Normative lines (4):
  - `Handshake_Master_Spec_v02.103.md:13501` - Tools **MUST NOT** maintain private long-term stores for user data; long-lived results live in the workspace.
  - `Handshake_Master_Spec_v02.103.md:13504` - Long-running or multi-step mechanical tasks **SHOULD** be expressed as AI Jobs and/or workflow nodes (Sections 2.5.10 and 2.6.6).
  - `Handshake_Master_Spec_v02.103.md:13515` - Implementations **SHOULD** hide tool differences behind configuration and capability profiles rather than exposing multiple \u201cimport modes\u201d to the user.
  - `Handshake_Master_Spec_v02.103.md:13518` - All mechanical tool invocations **MUST** be logged in the Flight Recorder (Section 2.1.5) with: tool identity, version, inputs (by reference), outputs (by reference), and errors.

## 6.1 Document Ingestion: Docling Subsystem
- Spec: `Handshake_Master_Spec_v02.103.md:13525`
- Bounds: `Handshake_Master_Spec_v02.103.md:13525` .. `Handshake_Master_Spec_v02.103.md:14972`
- Why: Handshake needs to ingest documents from various formats (PDF, DOCX, PPTX, etc.) and convert them into structured blocks for editing and AI processing. Docling provides MIT-licensed, layout-aware document understanding.
- What: Integrates IBM Docling as the primary document processor; covers media support, licensing, architecture, alternatives, performance, and RAG enhancement. This section consolidates three research artefacts on Docling.
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 6.2 Speech Recognition: ASR Subsystem
- Spec: `Handshake_Master_Spec_v02.103.md:14973`
- Bounds: `Handshake_Master_Spec_v02.103.md:14973` .. `Handshake_Master_Spec_v02.103.md:17166`
- Why: Handshake needs to transcribe long-form audio (lectures, meetings, screen recordings) into searchable, AI-accessible text. Local-first ASR ensures privacy and offline capability.
- What: Specifies ASR goals, model landscape, architecture, audio handling, customization policy, privacy, UX, and evaluation framework.
- Roadmap mentions section number in: 7.6.2 Phase 0 \u2014 Foundations (Pre-MVP), 7.6.5 Phase 3 \u2014 ASR & Long-Form Capture
- Normative lines (111):
  - `Handshake_Master_Spec_v02.103.md:15091` If the user\u2019s hardware is weaker, the runtime **MUST** degrade gracefully by switching to smaller or quantized models as defined below.
  - `Handshake_Master_Spec_v02.103.md:15150` The ASR runtime **MUST** implement a simple, deterministic selection policy:
  - `Handshake_Master_Spec_v02.103.md:15179` - Cloud ASR **MUST be disabled by default**.
  - `Handshake_Master_Spec_v02.103.md:15186` - The active model, runtime, and selection path **MUST** be visible in:
  - `Handshake_Master_Spec_v02.103.md:15189` - The user **MUST** be able to:
  - `Handshake_Master_Spec_v02.103.md:15192` - Model version and configuration **MUST** be recorded as part of the transcript metadata to support reproducibility and regression testing.
  - `Handshake_Master_Spec_v02.103.md:15236` The above flow **MUST** be deterministic and reproducible: given the same source, configuration, and model version, the system **SHOULD** produce the same transcript (modulo nondeterminism in beam search).
  - `Handshake_Master_Spec_v02.103.md:15260` - The Orchestrator **MUST** treat ASR as a black-box service:
  - `Handshake_Master_Spec_v02.103.md:15268` - The Orchestrator **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15285` - MVP **MUST** support:
  - `Handshake_Master_Spec_v02.103.md:15288` - Additional modes (e.g. live system audio capture) are **MAY** for later phases and **MUST NOT** complicate the core pipeline.
  - `Handshake_Master_Spec_v02.103.md:15291` - The Orchestrator **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15300` - The MediaSource and extraction process **MUST** preserve:
  - `Handshake_Master_Spec_v02.103.md:15309` Before ASR, audio **MUST** be pre-processed and segmented:
  - `Handshake_Master_Spec_v02.103.md:15321` - The Orchestrator **MUST** support:
  - `Handshake_Master_Spec_v02.103.md:15328` - Segmentation decisions **MUST** be recorded (start/end times per segment) to allow deterministic re-assembly.
  - `Handshake_Master_Spec_v02.103.md:15334` - The chosen profile **MUST** be stored as part of the job configuration.
  - `Handshake_Master_Spec_v02.103.md:15338` The interface between desktop client, Orchestrator, and ASR service **MUST** be explicit and versioned.
  - `Handshake_Master_Spec_v02.103.md:15378` - The Orchestrator **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15383` - The ASR service **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:15389` - Progress to the client **MUST** be based on:
  - `Handshake_Master_Spec_v02.103.md:15392` - The client **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:15402` - Each transcript **MUST** be stored as a **DerivedContent** object linked to:
  - `Handshake_Master_Spec_v02.103.md:15413` - Transcripts **MUST** be versioned:
  - `Handshake_Master_Spec_v02.103.md:15415` - Prior versions **MUST** remain accessible for debugging and comparison until explicitly deleted.
  - `Handshake_Master_Spec_v02.103.md:15416` - The UI **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:15421` - Shadow Workspace **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15424` - The transcript **MUST** be:
  - `Handshake_Master_Spec_v02.103.md:15430` - ASR transcripts **MUST** be first-class inputs to:
  - `Handshake_Master_Spec_v02.103.md:15434` - These tools **MUST NOT** modify the original transcript; they produce additional DerivedContent artifacts (summaries, notes, etc.) with their own IDs and metadata.
  - `Handshake_Master_Spec_v02.103.md:15438` - Transcripts **MUST** be treated as user-owned workspace data:
  - `Handshake_Master_Spec_v02.103.md:15441` - Any external calls (e.g. cloud LLM summarization) **MUST** be clearly documented and, where possible, logged at a metadata level (not full content) for user inspection.
  - `Handshake_Master_Spec_v02.103.md:15473` - The system **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15478` - The system **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:15487` - The Orchestrator **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15502` - For long recordings, the Orchestrator **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15514` - ASR **MUST NOT** starve LLM runtimes; GPU VRAM thresholds and prioritization rules in X.2.4 apply.
  - `Handshake_Master_Spec_v02.103.md:15518` - For long jobs, the UI **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:15525` Real-time streaming / live captions are **explicitly out of scope** for the initial ASR MVP, but the architecture **MUST NOT** make them impossible.
  - `Handshake_Master_Spec_v02.103.md:15535` - MVP **MUST NOT** attempt to:
  - `Handshake_Master_Spec_v02.103.md:15542` Even though streaming is not implemented in the MVP, the design **SHOULD** leave room for:
  - `Handshake_Master_Spec_v02.103.md:15569` - Even in batch mode, the Orchestrator and ASR runtime **MUST** treat audio as an ordered sequence of segments:
  - `Handshake_Master_Spec_v02.103.md:15577` - However, the service interface **MUST** be designed so that:
  - `Handshake_Master_Spec_v02.103.md:15583` - Even without streaming, the GPU resource manager in the Orchestrator **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15589` - Batch transcription UI **MUST** be clearly separated from any future \u201clive caption\u201d UI:
  - `Handshake_Master_Spec_v02.103.md:15650` Handshake **MUST** prioritize options 1\u20133 for the MVP, and treat 4\u20135 as later-phase optimizations governed by X.5.2.
  - `Handshake_Master_Spec_v02.103.md:15654` This section defines **when** Handshake is allowed to fine-tune ASR models, and which lighter-weight customization options **MUST** be tried first.
  - `Handshake_Master_Spec_v02.103.md:15676` This option **MUST** be evaluated before full fine-tuning.
  - `Handshake_Master_Spec_v02.103.md:15684` This is the **last resort** and **MUST** pass the gate criteria in X.5.2.2.
  - `Handshake_Master_Spec_v02.103.md:15688` Handshake **MUST NOT** initiate fine-tuning of any ASR model unless **all** of the following conditions are true:
  - `Handshake_Master_Spec_v02.103.md:15695` - Data **MUST** be collected with explicit user consent and stored in a way that respects privacy requirements.
  - `Handshake_Master_Spec_v02.103.md:15710` - This expectation **MUST** be documented in a short design note before training begins.
  - `Handshake_Master_Spec_v02.103.md:15714` Fine-tuning **MUST NOT** run on end-user devices.
  - `Handshake_Master_Spec_v02.103.md:15721` - Each fine-tuned model version **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15724` - A rollback plan **MUST** exist:
  - `Handshake_Master_Spec_v02.103.md:15725` - If a new model regresses on any tracked metric beyond tolerance, the system **MUST** be able to immediately revert to the previous stable model.
  - `Handshake_Master_Spec_v02.103.md:15727` If any of these conditions are not met, fine-tuning is **not allowed**. The team **MUST** continue using untuned models plus lighter customizations.
  - `Handshake_Master_Spec_v02.103.md:15737` - Any deviation from this default (e.g. opt-in data donation) **MUST** be explicit and clearly documented.
  - `Handshake_Master_Spec_v02.103.md:15756` - Donated data **MUST** be:
  - `Handshake_Master_Spec_v02.103.md:15763` - A subset of data **MUST** be manually checked or corrected.
  - `Handshake_Master_Spec_v02.103.md:15766` - Labeling **MUST** be guided by eval needs:
  - `Handshake_Master_Spec_v02.103.md:15771` - Training data **MUST** use a consistent schema:
  - `Handshake_Master_Spec_v02.103.md:15786` - Training **MUST NOT** happen in the desktop app.
  - `Handshake_Master_Spec_v02.103.md:15795` - Derivatives **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15810` - If a default model regresses in real-world use, it **MUST** be demoted and replaced by the previous stable model.
  - `Handshake_Master_Spec_v02.103.md:15816` - The client **MUST**:
  - `Handshake_Master_Spec_v02.103.md:15821` - Fine-tuned models **MUST** respect predefined resource envelopes:
  - `Handshake_Master_Spec_v02.103.md:15827` - Each trained model **MUST** have:
  - `Handshake_Master_Spec_v02.103.md:15860` - Cleanup steps **MUST** be deterministic given the same input
  - `Handshake_Master_Spec_v02.103.md:15861` - Cleanup configuration **MUST** be stored with the transcript metadata
  - `Handshake_Master_Spec_v02.103.md:15864` - Users **SHOULD** be able to:
  - `Handshake_Master_Spec_v02.103.md:15867` - The \u201craw\u201d ASR output (pre-cleanup) **SHOULD** be accessible to developers and power users.
  - `Handshake_Master_Spec_v02.103.md:15890` - The transcript schema (X.3.6) **SHOULD** leave room for:
  - `Handshake_Master_Spec_v02.103.md:15895` - Transcript representation **SHOULD** support:
  - `Handshake_Master_Spec_v02.103.md:15901` - UI **SHOULD** visually distinguish speakers (color, label)
  - `Handshake_Master_Spec_v02.103.md:15902` - Users **SHOULD** be able to rename speakers (SPK1 \u2192 \u201cAlice\u201d)
  - `Handshake_Master_Spec_v02.103.md:15906` ASR transcripts are a primary input to Handshake\u2019s LLM tools. These tools **MUST NOT** overwrite the transcript itself; they produce additional DerivedContent.
  - `Handshake_Master_Spec_v02.103.md:15928` - ASR **MUST** remain a separate concern:
  - `Handshake_Master_Spec_v02.103.md:15933` - Users **SHOULD** be able to:
  - `Handshake_Master_Spec_v02.103.md:15946` - Transcripts **MUST** be:
  - `Handshake_Master_Spec_v02.103.md:15959` - Chunks **MUST** preserve pointers back to:
  - `Handshake_Master_Spec_v02.103.md:15964` - Derived artifacts (summaries, notes, extracted items) **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:15972` - Cloud usage (for embedding or LLM) **MUST** be:
  - `Handshake_Master_Spec_v02.103.md:16055` - All default ASR models **MUST**:
  - `Handshake_Master_Spec_v02.103.md:16058` - License type **MUST** be documented in the model card.
  - `Handshake_Master_Spec_v02.103.md:16059` - Any usage restrictions **MUST** be clearly surfaced in internal docs.
  - `Handshake_Master_Spec_v02.103.md:16062` - Core ASR libraries (Faster-Whisper, whisper.cpp, etc.) **MUST**:
  - `Handshake_Master_Spec_v02.103.md:16064` - Third-party code **MUST**:
  - `Handshake_Master_Spec_v02.103.md:16068` - Handshake **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:16090` - This **MUST** require explicit opt-in.
  - `Handshake_Master_Spec_v02.103.md:16091` - The UI **MUST** clearly indicate:
  - `Handshake_Master_Spec_v02.103.md:16095` - **MUST NOT** include raw audio or full transcripts without explicit user consent.
  - `Handshake_Master_Spec_v02.103.md:16098` - Users **MUST** be able to:
  - `Handshake_Master_Spec_v02.103.md:16102` - **MUST** be opt-in and clearly documented (X.5.3).
  - `Handshake_Master_Spec_v02.103.md:16114` - **MUST** annotate which segments came from cloud vs local ASR.
  - `Handshake_Master_Spec_v02.103.md:16118` - The system **MUST NOT** silently drop segments.
  - `Handshake_Master_Spec_v02.103.md:16119` - The final transcript **MUST** clearly indicate missing or failed sections.
  - `Handshake_Master_Spec_v02.103.md:16122` - Cloud fallback settings **MUST** be:
  - `Handshake_Master_Spec_v02.103.md:16132` Handshake\u2019s ASR eval suite **MUST** track at least:
  - `Handshake_Master_Spec_v02.103.md:16161` - Test sets **MUST**:
  - `Handshake_Master_Spec_v02.103.md:16166` - Datasets **SHOULD** include:
  - `Handshake_Master_Spec_v02.103.md:16175` - **MUST** stress segmentation, queueing, and resource usage.
  - `Handshake_Master_Spec_v02.103.md:16201` - Primary ASR model **MUST** fit within a defined budget on the reference GPU with headroom for at least one LLM.
  - `Handshake_Master_Spec_v02.103.md:16203` - ASR **MUST NOT** monopolize all cores; Orchestrator **MUST** cap parallelism.
  - `Handshake_Master_Spec_v02.103.md:16205` These thresholds are subject to revision but **MUST** be documented at each revision.
  - `Handshake_Master_Spec_v02.103.md:16214` - **MUST** trigger:
  - `Handshake_Master_Spec_v02.103.md:16219` - A new version **MUST NOT** be promoted to default if:
  - `Handshake_Master_Spec_v02.103.md:16230` - **MUST** not include raw content unless explicitly opted in.
  - `Handshake_Master_Spec_v02.103.md:16233` - Each major ASR update **SHOULD**:
  - `Handshake_Master_Spec_v02.103.md:16243` The ASR MVP **MUST** deliver:
  - `Handshake_Master_Spec_v02.103.md:16274` Phase 2 **SHOULD** focus on:

## 6.3 Mechanical Extension Engines
- Spec: `Handshake_Master_Spec_v02.103.md:17167`
- Bounds: `Handshake_Master_Spec_v02.103.md:17167` .. `Handshake_Master_Spec_v02.103.md:18907`
- Purpose (no explicit Why/What blocks): Version: v1.2 (Tool Bus + conformance; 22 spec-grade engines) Date: 2025-12-23
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (47):
  - `Handshake_Master_Spec_v02.103.md:17190` - **Rule:** these MUST remain unambiguous via `schema_version` (and, where present, `protocol_id`). Engine invocations MUST use `poe-*`.
  - `Handshake_Master_Spec_v02.103.md:17193` - **Size rule:** any payload > **32KB** MUST be passed via input artifacts (handles/refs), never inlined into the PlannedOperation.
  - `Handshake_Master_Spec_v02.103.md:17194` - Outputs MUST be exported as artifacts (immutable) with **SHA-256** hashing + sidecar provenance manifests (see Artifact rules in \xa72.3.10).
  - `Handshake_Master_Spec_v02.103.md:17201` - **Evidence rule:** D0/D1 results MUST carry evidence artifacts referenced in EngineResult.
  - `Handshake_Master_Spec_v02.103.md:17211` Gate outcomes MUST be logged to Flight Recorder and surfaced in Problems when denied or degraded.
  - `Handshake_Master_Spec_v02.103.md:17214` - Engines MUST be declared in `mechanical_engines.json` (engine_id, ops, determinism ceiling, required gates, default capabilities, conformance vectors, implementation/adapters).
  - `Handshake_Master_Spec_v02.103.md:17215` - **No-bypass:** engines MUST NOT be invokable outside the orchestrator/runtime.
  - `Handshake_Master_Spec_v02.103.md:17221` v1.2 capability strings (file/process/network/device/secrets/GPU) are enforced through the global capability/consent system in **\xa711.1**. Device/network/secrets MUST remain deny-by-default and require explicit consent records.
  - `Handshake_Master_Spec_v02.103.md:17252` > Normative rule: In all domains below, LLMs are planners only. Physical or irreversible operations MUST go through a mechanical engine with explicit safety checks.
  - `Handshake_Master_Spec_v02.103.md:17439` The Artistic Vectors UI MUST materialize a typed `ConceptRecipe` artifact (Derived) rather than keeping the "recipe" implicit in UI state.
  - `Handshake_Master_Spec_v02.103.md:17459` - **Replay rule:** given identical `ConceptRecipe` + identical role contracts/pins, downstream role composition MUST be replayable.
  - `Handshake_Master_Spec_v02.103.md:17559` - **Raw-data invariant:** Atelier stores uncensored intent/descriptors in RawContent/DerivedContent; any filtering is limited to Display/Export connectors and MUST NOT write back into stored artifacts.
  - `Handshake_Master_Spec_v02.103.md:17582` - **ATELIER-VAL-003 No write-back censorship:** any connector filtering MUST NOT modify RawContent/DerivedContent fields.
  - `Handshake_Master_Spec_v02.103.md:17583` - **ATELIER-VAL-004 Comfy recipe contract:** `comfy_recipe.template_id` must be present or resolved to a deterministic fallback; Atelier MUST NOT attempt to author a runnable Comfy workflow graph.
  - `Handshake_Master_Spec_v02.103.md:17591` - **Determinism:** same plan + same template registry version MUST produce the same output bytes.
  - `Handshake_Master_Spec_v02.103.md:17597` This subsection formalizes **Atelier-as-runtime**: every Atelier role is an executable lens that can (a) **extract** role-relevant descriptors from any ingested artifact and (b) **produce** role-specific creative deliverables. Roles may claim relevance across domains (e.g., architecture, fashion, interiors, set dressing, adult content, graphic design) and the system MUST support **multi-claim** on a single artifact.
  - `Handshake_Master_Spec_v02.103.md:17599` **Non-negotiable invariant (Raw/Derived):** role extraction outputs MUST be stored in Raw/Derived as captured; any redaction, filtering, or policy transformations are allowed ONLY at Display/Export connector boundaries and MUST NOT write back into stored artifacts.
  - `Handshake_Master_Spec_v02.103.md:17648` - `fields{}` MUST be strongly typed and versioned per contract.
  - `Handshake_Master_Spec_v02.103.md:17662` - **Determinism:** pinned `AtelierRoleSpec` registry version + pinned claim config + pinned claim model version (if used) MUST yield identical outputs for identical inputs.
  - `Handshake_Master_Spec_v02.103.md:17666` - Claims MUST log which features fired and why (replayable trace).
  - `Handshake_Master_Spec_v02.103.md:17668` **All-roles glance (ideal behavior; SHOULD, not MUST):**
  - `Handshake_Master_Spec_v02.103.md:17683` All role extraction MUST emit evidence pointers sufficient to audit and replay extraction.
  - `Handshake_Master_Spec_v02.103.md:17694` Validators MUST reject `RoleDescriptorBundle` outputs that omit required evidence for mandatory fields.
  - `Handshake_Master_Spec_v02.103.md:17702` **Rule:** role extractors MUST prefer deterministic, pinned model signals and MUST store provenance sufficient for replay.
  - `Handshake_Master_Spec_v02.103.md:17710` - **Pinned models/tools:** exact model IDs and tool versions MUST be recorded.
  - `Handshake_Master_Spec_v02.103.md:17711` - **Replay:** rerun with the same pins MUST reproduce identical Derived outputs (byte-identical JSON for bundles).
  - `Handshake_Master_Spec_v02.103.md:17732` **Rule:** accepted vocab changes are versioned; role extraction outputs MUST reference the vocab snapshot used.
  - `Handshake_Master_Spec_v02.103.md:17741` - **Rule:** role produce contracts MUST declare input/output ports so scheduling is deterministic.
  - `Handshake_Master_Spec_v02.103.md:17759` - **Rule:** deep pass MUST reference the fast pass evidence and may only add fields allowed by the role\u2019s extract contract.
  - `Handshake_Master_Spec_v02.103.md:17787` - **Rule:** deliverables MUST be referenced as artifacts with hashes (no opaque blobs without provenance).
  - `Handshake_Master_Spec_v02.103.md:17807` - **ATELIER-LENS-VAL-004 No write-back filtering (FAIL):** any Display/Export projection MUST NOT mutate stored Raw/Derived.
  - `Handshake_Master_Spec_v02.103.md:17808` - **ATELIER-LENS-VAL-005 Namespace safety (FAIL):** role-local terms MUST stay in role vocab namespace; shared terms must reference a shared vocab snapshot.
  - `Handshake_Master_Spec_v02.103.md:17810` - **ATELIER-LENS-VAL-007 Merge determinism (FAIL):** if `ATELIER_STATE_MERGE` is invoked, identical inputs + identical `merge_policy_id` + identical pins MUST yield identical `SceneState.resolved_hash`.
  - `Handshake_Master_Spec_v02.103.md:17811` - **ATELIER-LENS-VAL-008 Conflict accounting (FAIL):** if merge policy resolves conflicts or conflicts are detected, a `ConflictSet` artifact MUST be emitted and linked from the `SceneState`.
  - `Handshake_Master_Spec_v02.103.md:17812` - **ATELIER-LENS-VAL-009 Recipe validity (FAIL):** any `ConceptRecipe` used by Atelier Lens MUST pass range checks (`vectors` in `[0..1]`), required pins present, and seed policy recorded.
  - `Handshake_Master_Spec_v02.103.md:17813` - **ATELIER-LENS-VAL-010 DAG validity (FAIL):** if `ATELIER_GRAPH_SOLVE` is invoked, the resulting `AtelierProductionGraph` MUST be acyclic OR explicitly cycle-broken with a recorded rule.
  - `Handshake_Master_Spec_v02.103.md:17814` - **ATELIER-LENS-VAL-011 Dependency completeness (FAIL):** every `solve_plan` step MUST declare required inputs, and the execution plan MUST not schedule a step before its declared dependencies are satisfied.
  - `Handshake_Master_Spec_v02.103.md:17951` - Input images MUST be \u22644096px on long edge
  - `Handshake_Master_Spec_v02.103.md:18055` - Input images MUST be \u22644096px
  - `Handshake_Master_Spec_v02.103.md:18111` Atelier is explicitly multi-role and overlap is expected. The system MUST define deterministic merge semantics for role outputs without silently overwriting other roles.
  - `Handshake_Master_Spec_v02.103.md:18153` - tie-breaking MUST be deterministic and explicit (e.g., fixed priority list or numeric weights),
  - `Handshake_Master_Spec_v02.103.md:18154` - resolution MUST be recorded in `ConflictSet`,
  - `Handshake_Master_Spec_v02.103.md:18161` - **Replay rule:** with pinned versions, re-running merge MUST reproduce `resolved_hash`.
  - `Handshake_Master_Spec_v02.103.md:18173` - `acyclic` (bool; MUST be true unless cycle-breaking is explicitly recorded)
  - `Handshake_Master_Spec_v02.103.md:18180` - **Cycle rule:** cycles MUST be prevented by design; if cycles appear, the solver MUST emit an explicit cycle-break record (rule id + rationale) and mark `acyclic=false`.
  - `Handshake_Master_Spec_v02.103.md:18185` The Finishing department MUST be representable as dual contracts (Extraction + Creative Output), not only as prose.
  - `Handshake_Master_Spec_v02.103.md:18209` All post-production role bundles MUST follow the Evidence Pointer Standard (\xa76.3.3.5.7.3) and be replayable under pinned versions (\xa76.3.3.5.7.5).

## 7.1 User Interface Components
- Spec: `Handshake_Master_Spec_v02.103.md:18908`
- Bounds: `Handshake_Master_Spec_v02.103.md:18908` .. `Handshake_Master_Spec_v02.103.md:19341`
- Why: The UI components define how users interact with Handshake. Choosing the right libraries and patterns ensures a familiar yet powerful experience combining the best of Notion, Milanote, and Excel.
- What: Covers the three main UI components: Rich Text Editor (Notion-like block-based editing with Tiptap/BlockNote), Freeform Canvas (Milanote-like infinite whiteboard with Excalidraw), Spreadsheet Engine (Excel-like data manipulation with Wolf-Table + HyperFormula), and Additional Views (Kanban, Calendar, Timeline).
- Roadmap mentions section number: NO
- Normative lines (2):
  - `Handshake_Master_Spec_v02.103.md:18955` 1. UI components **MUST NOT** introduce their own persistent storage or IDs for core entities; they use workspace IDs and schemas.
  - `Handshake_Master_Spec_v02.103.md:18956` 2. Any operation that crosses views (e.g. "send selection to canvas", "turn table into doc section") **MUST** preserve entity IDs instead of duplicating content.

## 7.2 Multi-Agent Orchestration
- Spec: `Handshake_Master_Spec_v02.103.md:19342`
- Bounds: `Handshake_Master_Spec_v02.103.md:19342` .. `Handshake_Master_Spec_v02.103.md:19604`
- Why: Complex tasks require coordinating multiple specialized AI models. This section explains how to orchestrate agents effectively using the lead/worker pattern for cost-effective, high-quality results.
- What: Compares orchestration frameworks (AutoGen, LangGraph, CrewAI), explains the lead/worker pattern for cost optimization, covers shared context/memory between agents, and defines task routing and fallback logic.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 7.3 Collaboration and Sync
- Spec: `Handshake_Master_Spec_v02.103.md:19605`
- Bounds: `Handshake_Master_Spec_v02.103.md:19605` .. `Handshake_Master_Spec_v02.103.md:19796`
- Why: Multi-device and multi-user collaboration requires robust synchronization. This section covers how CRDT-based sync enables real-time collaboration while maintaining offline-first functionality.
- What: Explains sync architecture using Yjs, covers server infrastructure options, handles conflict resolution, and defines sharing/permissions model.
- Roadmap mentions section number in: 7.6.6 Phase 4 \u2014 Collaboration & Extension Ecosystem
- Normative lines (0): (none detected by keyword scan)

## 7.4 Reference Application Analysis
- Spec: `Handshake_Master_Spec_v02.103.md:19797`
- Bounds: `Handshake_Master_Spec_v02.103.md:19797` .. `Handshake_Master_Spec_v02.103.md:19892`
- Why: Learning from similar applications avoids repeating their mistakes. This section summarizes insights from analyzing AppFlowy, AFFiNE, Obsidian, and Logseq.
- What: Analyzes four reference applications (their stacks, data models, sync approaches), identifies patterns to follow and patterns to avoid.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 7.5 Development Workflow
- Spec: `Handshake_Master_Spec_v02.103.md:19893`
- Bounds: `Handshake_Master_Spec_v02.103.md:19893` .. `Handshake_Master_Spec_v02.103.md:20064`
- Why: Consistent development practices ensure code quality and team productivity. This section defines the tooling, processes, and standards for the project.
- What: Covers repository structure (monorepo with Turborepo), code quality tools (ESLint, Prettier, Ruff), CI/CD pipeline (GitHub Actions), testing strategy, and project health practices. Cross-ref: sections 10/11 for Terminal/Monaco dev-surface requirements and shared capability/observability contracts to be exercised in workflows and CI.
- Roadmap mentions section number in: 7.6.2 Phase 0 \u2014 Foundations (Pre-MVP), 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (0): (none detected by keyword scan)

## 8.1 Risk Assessment
- Spec: `Handshake_Master_Spec_v02.103.md:21466`
- Bounds: `Handshake_Master_Spec_v02.103.md:21466` .. `Handshake_Master_Spec_v02.103.md:21518`
- Why: Understanding risks upfront enables proactive mitigation. This section identifies key risks and their mitigation strategies.
- What: Risk matrix covering likelihood and impact, complexity ratings for each component, and mitigation strategies.
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (0): (none detected by keyword scan)

## 8.2 Technology Stack Summary
- Spec: `Handshake_Master_Spec_v02.103.md:21519`
- Bounds: `Handshake_Master_Spec_v02.103.md:21519` .. `Handshake_Master_Spec_v02.103.md:21641`
- Why: A consolidated reference of all technologies enables quick lookup and ensures consistency across the project.
- What: Complete list of technologies organized by layer: Core Stack, Frontend Libraries, Backend Libraries, AI Models, DevOps Tools.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 8.3 Gap Analysis & Open Questions
- Spec: `Handshake_Master_Spec_v02.103.md:21642`
- Bounds: `Handshake_Master_Spec_v02.103.md:21642` .. `Handshake_Master_Spec_v02.103.md:21773`
- Why: Acknowledging what the research doesn't cover prevents false confidence and highlights areas needing further investigation.
- What: Documents research gaps (UI/UX, authentication, business model, fine-tuning, Windows-specific), open technical questions, and unresolved issues requiring further work.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 8.4 Consolidated Glossary
- Spec: `Handshake_Master_Spec_v02.103.md:21774`
- Bounds: `Handshake_Master_Spec_v02.103.md:21774` .. `Handshake_Master_Spec_v02.103.md:21869`
- Why: A unified glossary ensures consistent terminology across the project and serves as quick reference.
- What: Alphabetical list of all technical terms defined throughout this specification.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 8.5 Sources Referenced
- Spec: `Handshake_Master_Spec_v02.103.md:21870`
- Bounds: `Handshake_Master_Spec_v02.103.md:21870` .. `Handshake_Master_Spec_v02.103.md:21945`
- Why: Documenting sources enables verification, further research, and acknowledgment of the research foundation.
- What: Lists all source documents that were synthesized into this unified specification.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 8.6 Appendices
- Spec: `Handshake_Master_Spec_v02.103.md:21946`
- Bounds: `Handshake_Master_Spec_v02.103.md:21946` .. `Handshake_Master_Spec_v02.103.md:22716`
- Why: Appendices provide supplementary reference material including foundation concepts for newcomers, detailed architecture decisions, comparison tables, benchmark data, and works cited that support the main specification.
- What: Contains Foundation Concepts (beginner explainers), Architecture Decisions (detailed rationale), Plugin System design, Docling/ASR comparison tables, and works cited.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 8.7 Version History & Subsection Versioning
- Spec: `Handshake_Master_Spec_v02.103.md:22717`
- Bounds: `Handshake_Master_Spec_v02.103.md:22717` .. `Handshake_Master_Spec_v02.103.md:23756`
- Why: Clear version tracking enables understanding which features are included in each release and how subsections with independent versions relate to the master specification version.
- What: Documents master specification version history, maps independent subsection versions to master versions, and defines versioning policy.
- Roadmap mentions section number: NO
- Normative lines (4):
  - `Handshake_Master_Spec_v02.103.md:22726` - Dates MUST reflect actual release/publication dates (no future dating).
  - `Handshake_Master_Spec_v02.103.md:22727` - Each entry SHOULD list an owner and maturity (Normative / Draft / Research) for traceability; fill missing owners/maturities.
  - `Handshake_Master_Spec_v02.103.md:22728` - Subsection rows MUST stay in sync with source documents and ADRs; divergences are called out explicitly.
  - `Handshake_Master_Spec_v02.103.md:22808` - \xa72.3.8 Shadow Workspace: vector records MUST include `source_hash` for drift detection.

## 10.1 Terminal Experience
- Spec: `Handshake_Master_Spec_v02.103.md:23757`
- Bounds: `Handshake_Master_Spec_v02.103.md:23757` .. `Handshake_Master_Spec_v02.103.md:24037`
- Purpose (no explicit Why/What blocks): Status: Exploratory (aligned with v02.18 + Mechanical Extension Engines v1.1); not yet implemented.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (45):
  - `Handshake_Master_Spec_v02.103.md:23765` - Handshake MUST distinguish between:
  - `Handshake_Master_Spec_v02.103.md:23768` - The spec MUST state that **only sandboxed shells provide strong containment** against filesystem and network access; policy-scoped shells are best-effort.
  - `Handshake_Master_Spec_v02.103.md:23771` - For policy-scoped shells, Handshake MUST:
  - `Handshake_Master_Spec_v02.103.md:23779` - Handshake SHOULD provide a \u201csecure shell mode\u201d where:
  - `Handshake_Master_Spec_v02.103.md:23787` - Shell capabilities MUST be defined along these axes:
  - `Handshake_Master_Spec_v02.103.md:23794` - Handshake MUST support at least:
  - `Handshake_Master_Spec_v02.103.md:23797` - Approvals MUST be:
  - `Handshake_Master_Spec_v02.103.md:23803` - The run MUST be blocked.
  - `Handshake_Master_Spec_v02.103.md:23804` - The orchestrator MUST surface an escalation request to the user:
  - `Handshake_Master_Spec_v02.103.md:23807` - The decision MUST be logged in Flight Recorder for that job.
  - `Handshake_Master_Spec_v02.103.md:23810` - Long-lived approvals MUST have:
  - `Handshake_Master_Spec_v02.103.md:23816` - The internal `run_command` tool MUST expose at least:
  - `Handshake_Master_Spec_v02.103.md:23829` - If `timeout_ms` is omitted, a reasonable default MUST be used (recommended: 180_000 ms).
  - `Handshake_Master_Spec_v02.103.md:23831` - The backend MUST send a termination signal to the process,
  - `Handshake_Master_Spec_v02.103.md:23833` - The result MUST include `timed_out: true`.
  - `Handshake_Master_Spec_v02.103.md:23834` - The orchestrator MUST be able to cancel an in-flight command:
  - `Handshake_Master_Spec_v02.103.md:23835` - Cancellation MUST propagate to the PTY,
  - `Handshake_Master_Spec_v02.103.md:23836` - Result MUST include `cancelled: true`.
  - `Handshake_Master_Spec_v02.103.md:23840` - Output MAY be streamed to the caller, but MUST be **bounded** by `max_output_bytes` (recommended default: 1\u20132 MB):
  - `Handshake_Master_Spec_v02.103.md:23841` - If truncated, the result MUST indicate truncation and how many bytes were emitted.
  - `Handshake_Master_Spec_v02.103.md:23843` - Output MUST be streamed, but the logging policy (below) still applies.
  - `Handshake_Master_Spec_v02.103.md:23844` - The API MUST separate:
  - `Handshake_Master_Spec_v02.103.md:23849` - Default environment MUST be inherited from the app\u2019s process (subject to secrets policy).
  - `Handshake_Master_Spec_v02.103.md:23856` - Every `run_command` call MUST emit a log event containing:
  - `Handshake_Master_Spec_v02.103.md:23862` - This event MUST be stable enough for replay and auditing.
  - `Handshake_Master_Spec_v02.103.md:23868` - Terminal logging MUST have at least:
  - `Handshake_Master_Spec_v02.103.md:23872` - Default MUST be `COMMANDS_ONLY` for AI job terminals.
  - `Handshake_Master_Spec_v02.103.md:23879` - For levels that log output, Handshake MUST run output through a redaction engine that:
  - `Handshake_Master_Spec_v02.103.md:23887` - When a user enables full output logging (even with redaction), Handshake MUST show:
  - `Handshake_Master_Spec_v02.103.md:23893` - A problem matcher MUST have at least:
  - `Handshake_Master_Spec_v02.103.md:23912` - Parsed output MUST be normalized into the common Diagnostics schema (see 11.4),
  - `Handshake_Master_Spec_v02.103.md:23920` - Every terminal session MUST be labeled as:
  - `Handshake_Master_Spec_v02.103.md:23924` - This type MUST be visible in the UI (badge, color, or icon).
  - `Handshake_Master_Spec_v02.103.md:23927` - AI models MUST NOT:
  - `Handshake_Master_Spec_v02.103.md:23931` - Any override MUST be:
  - `Handshake_Master_Spec_v02.103.md:23937` - Every `AI_JOB` session MUST be linked to:
  - `Handshake_Master_Spec_v02.103.md:23941` - And MUST be visible from Flight Recorder with a \u201cjump to terminal\u201d link.
  - `Handshake_Master_Spec_v02.103.md:23945` - v1 MUST support policy-scoped terminals on:
  - `Handshake_Master_Spec_v02.103.md:23951` - Across all three platforms v1 MUST support:
  - `Handshake_Master_Spec_v02.103.md:23960` - Shell integration (decorations, cwd markers), splits, persistent sessions, and sandboxed shells MAY initially be limited to subsets of platforms; the spec MUST:
  - `Handshake_Master_Spec_v02.103.md:23966` - AI job terminals SHOULD prefer secure mode when available; fallback to policy MUST show an explicit banner.
  - `Handshake_Master_Spec_v02.103.md:23971` - MUST for v1:
  - `Handshake_Master_Spec_v02.103.md:23982` - AI command execution MUST be capability-checked and trace-linked.
  - `Handshake_Master_Spec_v02.103.md:23983` - AI MUST NOT type into human terminals by default.
  - `Handshake_Master_Spec_v02.103.md:23984` - Every AI-run command MUST appear in Flight Recorder.

## 10.2 Monaco Editor Experience
- Spec: `Handshake_Master_Spec_v02.103.md:24038`
- Bounds: `Handshake_Master_Spec_v02.103.md:24038` .. `Handshake_Master_Spec_v02.103.md:24233`
- Purpose (no explicit Why/What blocks): Status: Exploratory (aligned with v02.18); normative LAW sections drafted but not implemented.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (24):
  - `Handshake_Master_Spec_v02.103.md:24046` - Block IDs MUST be:
  - `Handshake_Master_Spec_v02.103.md:24055` - AI patch APIs MUST address content by **block ID**, not by visual line numbers.
  - `Handshake_Master_Spec_v02.103.md:24058` - IDs MUST be assigned:
  - `Handshake_Master_Spec_v02.103.md:24067` - The resulting block MUST keep one of the original block IDs (e.g., left-most),
  - `Handshake_Master_Spec_v02.103.md:24068` - Other IDs MUST be retired (but MAY be kept in change history).
  - `Handshake_Master_Spec_v02.103.md:24071` - Edits to text within a block MUST NOT change its ID.
  - `Handshake_Master_Spec_v02.103.md:24072` - Reordering blocks MUST NOT change their IDs.
  - `Handshake_Master_Spec_v02.103.md:24075` - AI editing APIs MUST:
  - `Handshake_Master_Spec_v02.103.md:24083` - The `DocumentAST` MUST be the canonical representation.
  - `Handshake_Master_Spec_v02.103.md:24084` - No view (Monaco or RichView) may write directly to disk; they MUST update AST via well-defined operations.
  - `Handshake_Master_Spec_v02.103.md:24089` - `RichView` MUST render from AST.
  - `Handshake_Master_Spec_v02.103.md:24090` - `Monaco StructuredTextView` MUST render a textual projection of AST (with or without ID column).
  - `Handshake_Master_Spec_v02.103.md:24091` - AST changes MUST propagate to all open views for that worksurface.
  - `Handshake_Master_Spec_v02.103.md:24095` - Document worksurfaces MUST serialize AST to the on-disk representation.
  - `Handshake_Master_Spec_v02.103.md:24096` - DOCX export/import MUST go via AST, never via ad-hoc parsing of Monaco text.
  - `Handshake_Master_Spec_v02.103.md:24100` - v1 MUST pre-bundle at least:
  - `Handshake_Master_Spec_v02.103.md:24109` - Monaco core and languages SHOULD be lazy-loaded:
  - `Handshake_Master_Spec_v02.103.md:24114` - The Tauri/Vite config MUST:
  - `Handshake_Master_Spec_v02.103.md:24125` Diagnostics in this shape MUST be:
  - `Handshake_Master_Spec_v02.103.md:24143` - MUST for v1:
  - `Handshake_Master_Spec_v02.103.md:24215` - **Invariant:** AI MUST NOT silently mutate RawContent.
  - `Handshake_Master_Spec_v02.103.md:24228` - The `DOC_REWRITE` job SHOULD support producing multiple `ChangeProposal` variants (e.g., "Concise", "Professional", "Creative") in a single pass if requested.
  - `Handshake_Master_Spec_v02.103.md:24229` - UI MUST allow cycling through variants before Accepting.
  - `Handshake_Master_Spec_v02.103.md:24232` - Rejected proposals MUST be logged to Flight Recorder (tagged `rejected_idea`) to preserve "lost" work for potential future retrieval.

## 10.3 Mail Client
- Spec: `Handshake_Master_Spec_v02.103.md:24234`
- Bounds: `Handshake_Master_Spec_v02.103.md:24234` .. `Handshake_Master_Spec_v02.103.md:25109`
- Purpose (no explicit Why/What blocks): Status: v0.5 research/design; not implemented. Behaviours require capability/consent gates and workflow enforcement before shipping. Mail as a First-Class, Classified Domain in the Shadow Workspace
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 10.4 Calendar
- Spec: `Handshake_Master_Spec_v02.103.md:25110`
- Bounds: `Handshake_Master_Spec_v02.103.md:25110` .. `Handshake_Master_Spec_v02.103.md:26838`
- Purpose (no explicit Why/What blocks): Status: Calendar Law v0.4 (verbatim import) is normative for semantics/sync; ACE integration v0.3 principles added in \xa710.4.2. Implementation in progress; enforce capability/consent + Workflow Engine gates before enabling writes.
- Roadmap mentions section number in: 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (16):
  - `Handshake_Master_Spec_v02.103.md:25211` - All mutations MUST be submitted as `CalendarMutation` patches via a `WorkflowRun` targeting the `calendar_sync` mechanical engine.
  - `Handshake_Master_Spec_v02.103.md:25212` - Every successful mutation MUST emit a `Flight Recorder` span of type `calendar_mutation` with a back-link to the `job_id`.
  - `Handshake_Master_Spec_v02.103.md:25217` - **Stable instance identity:** every concrete occurrence MUST have an `instance_key` stable under re-sync and UI refresh, derived as:
  - `Handshake_Master_Spec_v02.103.md:25577` All state transitions MUST be logged to the Flight Recorder as `SYNC_STATE_CHANGE` with `(source_id, from_state, to_state, sync_run_id, reason)`.
  - `Handshake_Master_Spec_v02.103.md:25655` the resulting local calendar tables MUST be identical. This is required for trustworthy auditing and for reproducing failures from the Flight Recorder.
  - `Handshake_Master_Spec_v02.103.md:26720` - CalendarScopeHint MUST obey those rules; for cloud runs default to `minimal` or `analytics_only` unless explicitly elevated by a gated job.
  - `Handshake_Master_Spec_v02.103.md:26727` - CalendarScopeHint MUST only appear in the PromptEnvelope *variable suffix* as a `scope_hint` ContextBlock.
  - `Handshake_Master_Spec_v02.103.md:26728` - StablePrefix MUST NOT include raw event title/description/attendees/location/links.
  - `Handshake_Master_Spec_v02.103.md:26731` - CalendarScopeHint may only boost candidate scoring; it MUST NOT pin facts into context.
  - `Handshake_Master_Spec_v02.103.md:26735` - Calendar text MUST NOT be promoted into LongTermMemory as unqualified \u201cfacts.\u201d
  - `Handshake_Master_Spec_v02.103.md:26736` - Any compaction referencing calendar MUST do so via `event_id` / `time_range` / linked artifact handles and retain SourceRefs.
  - `Handshake_Master_Spec_v02.103.md:26739` - Sub-agents MUST NOT share a transcript or \u201cevent blob.\u201d
  - `Handshake_Master_Spec_v02.103.md:26744` - Calendar may select which policy/playbook applies (by `policy_profile_id`), but MUST NOT auto-mutate playbooks.
  - `Handshake_Master_Spec_v02.103.md:26752` - Calendar policy changes MUST NOT change model tier/projection mid-job.
  - `Handshake_Master_Spec_v02.103.md:26781` This function is used only when multiple events overlap \u201cnow\u201d and the orchestrator must pick a default active event. It MUST be deterministic.
  - `Handshake_Master_Spec_v02.103.md:26810` For any job step using CalendarScopeHint, the ContextSnapshot MUST record:

## 10.5 Operator Consoles: Debug & Diagnostics
- Spec: `Handshake_Master_Spec_v02.103.md:26839`
- Bounds: `Handshake_Master_Spec_v02.103.md:26839` .. `Handshake_Master_Spec_v02.103.md:28014`
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (47):
  - `Handshake_Master_Spec_v02.103.md:26851` This section is UI-agnostic. Monaco and other UIs are **adapters**. The underlying contracts (diagnostics, Flight Recorder linkability, bundle export) MUST remain stable even if UI components are replaced.
  - `Handshake_Master_Spec_v02.103.md:26859` - Normative surface requirements (MUST/SHOULD/MUST NOT) and acceptance criteria.
  - `Handshake_Master_Spec_v02.103.md:26870` The keywords **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, **MAY** are to be interpreted as in RFC 2119.
  - `Handshake_Master_Spec_v02.103.md:26874` P-01. **One shape, many sources.** Diagnostics from LSP/validators/engines/connectors MUST be normalized into a single schema (canonical in \xa711.4).
  - `Handshake_Master_Spec_v02.103.md:26876` P-02. **Correlate, don\u2019t guess.** The console MUST show link confidence (direct/inferred/ambiguous/unlinked) and MUST never present ambiguous correlations as certain.
  - `Handshake_Master_Spec_v02.103.md:26878` P-03. **Evidence first.** Every console action that changes state (ack/mute/re-run/export/resync/rebuild) MUST emit a Flight Recorder event (canonical in \xa711.5).
  - `Handshake_Master_Spec_v02.103.md:26880` P-04. **Redaction-safe by default.** Debug Bundle export MUST default to a redaction mode that cannot leak secrets/PII in typical usage.
  - `Handshake_Master_Spec_v02.103.md:26882` P-05. **Deterministic fingerprints.** Problems grouping MUST be driven by a deterministic fingerprinting function (canonical in \xa711.4).
  - `Handshake_Master_Spec_v02.103.md:26886` A compliant UI MUST support the following loop without requiring the operator to interpret logs:
  - `Handshake_Master_Spec_v02.103.md:26897` All surfaces below MUST deep-link to each other via `job_id`, `diagnostic_id`, `wsid`, and Flight Recorder event ids (see \xa711.4 and \xa711.5).
  - `Handshake_Master_Spec_v02.103.md:26901` MUST:
  - `Handshake_Master_Spec_v02.103.md:26908` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:26912` MUST NOT:
  - `Handshake_Master_Spec_v02.103.md:26917` MUST:
  - `Handshake_Master_Spec_v02.103.md:26922` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:26925` MUST NOT:
  - `Handshake_Master_Spec_v02.103.md:26930` MUST:
  - `Handshake_Master_Spec_v02.103.md:26936` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:26941` MUST:
  - `Handshake_Master_Spec_v02.103.md:26950` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:26955` MUST:
  - `Handshake_Master_Spec_v02.103.md:26960` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:26965` MUST:
  - `Handshake_Master_Spec_v02.103.md:26970` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:26975` MUST:
  - `Handshake_Master_Spec_v02.103.md:26985` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:26990` MUST:
  - `Handshake_Master_Spec_v02.103.md:27001` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:27006` MUST:
  - `Handshake_Master_Spec_v02.103.md:27012` SHOULD:
  - `Handshake_Master_Spec_v02.103.md:27016` MUST NOT:
  - `Handshake_Master_Spec_v02.103.md:27087` - **WORKSPACE**: may include more local context but MUST still redact secrets/PII.
  - `Handshake_Master_Spec_v02.103.md:27088` - **FULL_LOCAL**: includes full payloads; MUST NOT be exportable unless policy explicitly allows.
  - `Handshake_Master_Spec_v02.103.md:27092` The bundle MUST contain a prompt that includes:
  - `Handshake_Master_Spec_v02.103.md:27105` All bundle files MUST conform to the schemas defined below. Schema violations MUST cause VAL-BUNDLE-001 to fail.
  - `Handshake_Master_Spec_v02.103.md:27204` - MUST NOT include: file paths, environment variables, API keys, tokens, database URLs
  - `Handshake_Master_Spec_v02.103.md:27205` - MUST redact workspace paths to `[WORKSPACE_PATH]`
  - `Handshake_Master_Spec_v02.103.md:27206` - MUST redact user home paths to `[HOME]`
  - `Handshake_Master_Spec_v02.103.md:27535` /// Implementations MUST:
  - `Handshake_Master_Spec_v02.103.md:27766` The Secret Redactor MUST check for the following pattern categories:
  - `Handshake_Master_Spec_v02.103.md:27783` Redacted content MUST be replaced with: `[REDACTED:<category>:<detector_id>]`
  - `Handshake_Master_Spec_v02.103.md:27792` The Secret Redactor SHOULD delegate pattern detection to the Guard engine (`engine.guard.secret_scan`) when available, falling back to built-in patterns.
  - `Handshake_Master_Spec_v02.103.md:27798` To ensure deterministic bundle hashes, ZIP creation MUST:
  - `Handshake_Master_Spec_v02.103.md:27806` All hashes in bundles MUST use SHA-256, hex-encoded lowercase.
  - `Handshake_Master_Spec_v02.103.md:27826` Debug Bundle export MUST be accessible from:
  - `Handshake_Master_Spec_v02.103.md:27897` The validator MUST check:
  - `Handshake_Master_Spec_v02.103.md:27999` - Acceptance criteria (AC) MUST be written such that they can be checked by deterministic validators (VAL).

## 10.6 Canvas: Typography & Font Packs
- Spec: `Handshake_Master_Spec_v02.103.md:28015`
- Bounds: `Handshake_Master_Spec_v02.103.md:28015` .. `Handshake_Master_Spec_v02.103.md:28385`
- Purpose (no explicit Why/What blocks): Status: Font Packs + Canvas Typography Support Spec v0.1 (verbatim import) is normative for font packaging, import, runtime loading, and Canvas text rendering. Not yet implemented.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (4):
  - `Handshake_Master_Spec_v02.103.md:28028` - Capabilities/consent: operations such as import/remove/rebuild MUST be gated via capability/consent policy before any filesystem writes.
  - `Handshake_Master_Spec_v02.103.md:28029` - AI Job Model: font operations SHOULD be routable as explicit jobs when invoked by the agent layer (no implicit background mutation).
  - `Handshake_Master_Spec_v02.103.md:28030` - Security: UI surfaces MUST NOT crawl the filesystem directly; font discovery is backend-owned, with a narrow CSP surface for font asset delivery.
  - `Handshake_Master_Spec_v02.103.md:28072` When Handshake redistributes fonts (bundled pack), Handshake MUST:

## 10.7 Charts & Dashboards
- Spec: `Handshake_Master_Spec_v02.103.md:28386`
- Bounds: `Handshake_Master_Spec_v02.103.md:28386` .. `Handshake_Master_Spec_v02.103.md:28421`
- Purpose (no explicit Why/What blocks): Status: Draft (not yet implemented). Defines the first finance-friendly visualization surface built on top of Tables without creating a parallel datastore.
- Roadmap mentions section number: NO
- Normative lines (3):
  - `Handshake_Master_Spec_v02.103.md:28394` - Charts/Dashboards MUST obey the cross-view integration rules: no new persistent stores, ID-based refs, AI Jobs for non-trivial operations (\xa77.1.0; \xa72.6.6).
  - `Handshake_Master_Spec_v02.103.md:28416` - Chart rendering MUST be reproducible given:
  - `Handshake_Master_Spec_v02.103.md:28418` - Any AI-generated chart creation/update MUST be executed as an AI Job under \xa72.5.11 with preview and validators.

## 10.8 Presentations (Decks)
- Spec: `Handshake_Master_Spec_v02.103.md:28422`
- Bounds: `Handshake_Master_Spec_v02.103.md:28422` .. `Handshake_Master_Spec_v02.103.md:28463`
- Purpose (no explicit Why/What blocks): Status: Draft (not yet implemented). Defines an in-app deck surface plus deterministic export.
- Roadmap mentions section number: NO
- Normative lines (4):
  - `Handshake_Master_Spec_v02.103.md:28429` - Deck export (PPTX/PDF/HTML) is a mechanical operation that produces artifacts with provenance and MUST be logged like other mechanical tools (see \xa76.0 and \xa711.5).
  - `Handshake_Master_Spec_v02.103.md:28455` - Export MUST be invoked as `export_deck(...)` via the Charts & Decks AI Job Profile (\xa72.5.11) or an equivalent workflow node.
  - `Handshake_Master_Spec_v02.103.md:28456` - Export outputs MUST be artifact references/handles (no large inline payloads).
  - `Handshake_Master_Spec_v02.103.md:28457` - Export MUST record:

## 10.9 Future Surfaces
- Spec: `Handshake_Master_Spec_v02.103.md:28464`
- Bounds: `Handshake_Master_Spec_v02.103.md:28464` .. `Handshake_Master_Spec_v02.103.md:28470`
- Purpose (no explicit Why/What blocks): Reserved for future user-facing surfaces; add scoped subsections here. ---
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 10.10 Photo Studio
- Spec: `Handshake_Master_Spec_v02.103.md:28471`
- Bounds: `Handshake_Master_Spec_v02.103.md:28471` .. `Handshake_Master_Spec_v02.103.md:29724`
- Purpose (no explicit Why/What blocks): Photo Studio is a Lightroom/Affinity/Illustrator-class local-first photo + compositing surface. It is implemented by Photo Stack entities (\xa72.2.3) and executed by Darkroom engines (\xa76.3.3.6), producing artifacts under the unified export contract (\xa72.3.10).
- Roadmap mentions section number: NO
- Normative lines (18):
  - `Handshake_Master_Spec_v02.103.md:28498` The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, **MAY** are normative per RFC 2119.
  - `Handshake_Master_Spec_v02.103.md:28507` 1. The Photo Stack MUST function offline for all operations once inputs exist locally.
  - `Handshake_Master_Spec_v02.103.md:28508` 2. Every derived output MUST be reproducible from recorded inputs + engine versions + parameters, subject to determinism class.
  - `Handshake_Master_Spec_v02.103.md:28509` 3. All AI/ML inference SHOULD run locally by default; cloud services are opt-in only.
  - `Handshake_Master_Spec_v02.103.md:28512` 1. **Raw** (original files) MUST NOT be mutated by any edit operation.
  - `Handshake_Master_Spec_v02.103.md:28517` All edits MUST execute via the Handshake workflow/job runtime (no ad-hoc bypass). UI actions produce **job requests**, not direct mutations.
  - `Handshake_Master_Spec_v02.103.md:28520` Large outputs MUST be written as artifacts (files in the artifact store) and referenced by handles. Prompts or logs MUST carry references, not binaries.
  - `Handshake_Master_Spec_v02.103.md:28523` Where production-quality open-source libraries exist, implementations SHOULD leverage them to reduce custom code, improve maintainability, and benefit from community improvements.
  - `Handshake_Master_Spec_v02.103.md:28532` 1. High-resolution camera files (>20MP) SHOULD be processed via proxy workflow for AI/ML operations.
  - `Handshake_Master_Spec_v02.103.md:28533` 2. Full-resolution processing MUST be available for traditional (non-AI) operations.
  - `Handshake_Master_Spec_v02.103.md:28534` 3. AI-derived adjustments SHOULD be expressible as parameters applicable to full-resolution files.
  - `Handshake_Master_Spec_v02.103.md:28537` 1. Tools within Handshake SHOULD share context (selections, color palettes, metadata).
  - `Handshake_Master_Spec_v02.103.md:28538` 2. Clipboard operations SHOULD preserve semantic information across tool boundaries.
  - `Handshake_Master_Spec_v02.103.md:28539` 3. MCP (Model Context Protocol) SHOULD be used for tool interoperability where applicable.
  - `Handshake_Master_Spec_v02.103.md:29439` - `exportable: false` assets MUST NOT be exported without override
  - `Handshake_Master_Spec_v02.103.md:29445` - All cloud/ML services MUST be opt-in
  - `Handshake_Master_Spec_v02.103.md:29446` - Local-first processing MUST be default
  - `Handshake_Master_Spec_v02.103.md:29447` - Data sent externally MUST be logged with consent

## 11.1 Capabilities & Consent Model
- Spec: `Handshake_Master_Spec_v02.103.md:29725`
- Bounds: `Handshake_Master_Spec_v02.103.md:29725` .. `Handshake_Master_Spec_v02.103.md:29928`
- Purpose (no explicit Why/What blocks): - Scope/time-to-live defaults, approval caching, revocation UX, escalation paths, and capability axes for surfaces (terminal, editor, mail, calendar). - Mapping to plugins and product surfaces:
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (20):
  - `Handshake_Master_Spec_v02.103.md:29729` - Plugin manifest permissions MUST map to capability_profile_id entries; no ad-hoc plugin permissions.
  - `Handshake_Master_Spec_v02.103.md:29730` - Mail/Calendar surfaces MUST use the same capability/consent profiles (send_email, read_mail, export_calendar) used by plugin APIs and AI Job profiles.
  - `Handshake_Master_Spec_v02.103.md:29731` - Workflow/AI Job model MUST resolve effective capabilities from: plugin manifest (if tool), job profile, and surface-specific policy; the most restrictive wins.
  - `Handshake_Master_Spec_v02.103.md:29733` - The system MUST maintain a centralized `CapabilityRegistry` (SSoT) containing all valid Capability IDs (e.g. `fs.read`, `doc.summarize`, `terminal.exec`).
  - `Handshake_Master_Spec_v02.103.md:29734` - **Hard Invariant:** Any request for a Capability ID not defined in the Registry MUST be rejected with error `HSK-4001: UnknownCapability`. Ad-hoc or "magic string" capabilities are strictly forbidden.
  - `Handshake_Master_Spec_v02.103.md:29735` - **Audit Requirement:** Every capability check (Allow or Deny) MUST be recorded as a Flight Recorder event, capturing: `capability_id`, `actor_id`, `job_id` (if applicable), and `decision_outcome`.
  - `Handshake_Master_Spec_v02.103.md:29736` - **Profile Schema:** `CapabilityProfile` objects (e.g. 'Analyst', 'Coder') MUST be defined solely as whitelists of IDs from the `CapabilityRegistry`.
  - `Handshake_Master_Spec_v02.103.md:29737` - **Registry Generation:** `CapabilityRegistry` MUST be generated from the Master Spec and `mechanical_engines.json` into `assets/capability_registry.json` using an AI-assisted extraction job (local or cloud model allowed) with schema validation and human review; Spec Router and policy evaluation MUST pin `capability_registry_version` in their outputs.
  - `Handshake_Master_Spec_v02.103.md:29739` - Content classification + redaction flags flow from data layer to plugin/tool calls and AI jobs; cloud routing MUST honor projection/redaction defaults per surface (mail/calendar/doc).
  - `Handshake_Master_Spec_v02.103.md:29774` - **Validator Requirement:** The `CapabilityRegistry` (\xa7WP-1-Capability-SSoT) MUST resolve scoped requests against axis-level grants.
  - `Handshake_Master_Spec_v02.103.md:29775` 4. **Mechanical Engine Mapping:** All engines defined in \xa711.8 MUST declare their required capabilities using this scoped format (e.g., `engine.spatial` requires `fs.read:inputs` and `proc.exec:cad_kernel`).
  - `Handshake_Master_Spec_v02.103.md:29779` The system MUST maintain a centralized registry that validates and resolves capabilities.
  - `Handshake_Master_Spec_v02.103.md:29855` - Policy decisions MUST be logged to Flight Recorder with `policy_id` and `governance_mode`.
  - `Handshake_Master_Spec_v02.103.md:29856` - The policy decision MUST be referenced by `policy_profile_id` in ContextSnapshot (2.6.6.7.3).
  - `Handshake_Master_Spec_v02.103.md:29897` - `atelier_always_on` MUST default to true; disabling requires explicit LAW override.
  - `Handshake_Master_Spec_v02.103.md:29898` - If `git_workflow_policy.require_safety_commit == true`, Spec Router MUST require safety commit only when `version_control == Git`.
  - `Handshake_Master_Spec_v02.103.md:29899` - Spec Router MUST emit `SpecRouterDecision` with `capability_registry_version` and the policy_id used.
  - `Handshake_Master_Spec_v02.103.md:29900` - For GOV_STRICT and GOV_STANDARD, Spec Router MUST create or update Task Board and Work Packet entries and append Spec Session Log entries.
  - `Handshake_Master_Spec_v02.103.md:29913` 1. Extract: run AI-assisted extraction to produce `capability_registry_draft.json`. The job MUST record model_id, policy decision, and prompt hashes in Flight Recorder. Cloud model usage MUST obey CloudLeakageGuard and use redacted or derived inputs only.
  - `Handshake_Master_Spec_v02.103.md:29927` - MCP tools with write or external-network effects (including those exposed by the Python Orchestrator or external MCP servers) MUST be gated via the same approval classes and consent flows; the Rust MCP Gate (\xa711.3.2) enforces these decisions and logs them to Flight Recorder.

## 11.2 Sandbox Policy vs Hard Isolation
- Spec: `Handshake_Master_Spec_v02.103.md:29929`
- Bounds: `Handshake_Master_Spec_v02.103.md:29929` .. `Handshake_Master_Spec_v02.103.md:29935`
- Purpose (no explicit Why/What blocks): - Policy-scoped vs sandboxed modes, per-surface defaults, and platform availability matrix. - Based on TERM-SEC: policy mode is best-effort; sandboxed mode is the only strong containment. Document per-OS availability and per-surface defaults (e.g., AI job terminals prefer sandbox when available).
- Roadmap mentions section number: NO
- Normative lines (1):
  - `Handshake_Master_Spec_v02.103.md:29933` - MCP servers (including the Python Orchestrator and external tool servers) MUST respect sandbox roots and symlink protections from \xa711.3.7 when accessing host files via MCP tools or resources.

## 11.3 Auth/Session/MCP Primitives
- Spec: `Handshake_Master_Spec_v02.103.md:29936`
- Bounds: `Handshake_Master_Spec_v02.103.md:29936` .. `Handshake_Master_Spec_v02.103.md:31129`
- Purpose (no explicit Why/What blocks): - **MCP Lifecycle & Robustness:** - **Reconnection:** The MCP Client MUST support automatic reconnection with exponential backoff if the transport (stdio/SSE) is severed.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP), 7.6.5 Phase 3 \u2014 ASR & Long-Form Capture
- Normative lines (6):
  - `Handshake_Master_Spec_v02.103.md:29939` - **Reconnection:** The MCP Client MUST support automatic reconnection with exponential backoff if the transport (stdio/SSE) is severed.
  - `Handshake_Master_Spec_v02.103.md:29940` - **Traceability:** Every tool call MUST include the `trace_id` from the triggering AI Job in its MCP metadata (where supported) or in the paired Flight Recorder event.
  - `Handshake_Master_Spec_v02.103.md:29941` - **Stub Policy:** Production builds MUST NOT include the Stub Server; it is a test-only artifact.
  - `Handshake_Master_Spec_v02.103.md:29943` - Sessions MUST bind to user identity (where applicable), WSID(s), and capability set.
  - `Handshake_Master_Spec_v02.103.md:29944` - MCP resources/tools exposed to surfaces MUST inherit the session/WSID context and capability scope.
  - `Handshake_Master_Spec_v02.103.md:29945` - Terminal/Monaco/Mail/Calendar surfaces MUST advertise their effective MCP capability bindings to the orchestrator and Flight Recorder for traceability.

## 11.4 Diagnostics Schema (Problems/Events)
- Spec: `Handshake_Master_Spec_v02.103.md:31130`
- Bounds: `Handshake_Master_Spec_v02.103.md:31130` .. `Handshake_Master_Spec_v02.103.md:31365`
- Purpose (no explicit Why/What blocks): - **DIAG-SCHEMA-001 (Diagnostic shape; canonical)** A **Diagnostic** is the canonical, normalized representation of any problem emitted by LSPs, validators, engines, connectors, terminal matchers, or plugins.
- Roadmap mentions section number: NO
- Normative lines (36):
  - `Handshake_Master_Spec_v02.103.md:31167` // One of these SHOULD be set; multiple MAY be set to aid linking.
  - `Handshake_Master_Spec_v02.103.md:31233` - `fingerprint` is the Problems grouping key; it MUST be deterministic.
  - `Handshake_Master_Spec_v02.103.md:31234` - `count/first_seen/last_seen/status` are allowed to be local-only UI metadata; raw instances MUST remain queryable.
  - `Handshake_Master_Spec_v02.103.md:31237` Diagnostics in this shape MUST be:
  - `Handshake_Master_Spec_v02.103.md:31244` The Problems view MUST group diagnostics by `Diagnostic.fingerprint`, computed as a deterministic hash over a canonicalized tuple of fields.
  - `Handshake_Master_Spec_v02.103.md:31247` 1. Canonicalization MUST be stable across platforms (Windows/macOS/Linux) and UI layers.
  - `Handshake_Master_Spec_v02.103.md:31248` 2. Canonicalization MUST normalize:
  - `Handshake_Master_Spec_v02.103.md:31252` 3. The fingerprint MUST be computed as: `sha256(utf8(json_canonical_tuple))`, encoded as lowercase hex.
  - `Handshake_Master_Spec_v02.103.md:31254` The canonical tuple MUST include at least:
  - `Handshake_Master_Spec_v02.103.md:31260` It MUST NOT include:
  - `Handshake_Master_Spec_v02.103.md:31267` `Diagnostic.link_confidence` MUST follow:
  - `Handshake_Master_Spec_v02.103.md:31271` 3. **ambiguous**: matches multiple candidates; MUST list candidates in `evidence_refs.related_job_ids`.
  - `Handshake_Master_Spec_v02.103.md:31274` The UI MUST display `link_confidence` and MUST never present an ambiguous link as direct.
  - `Handshake_Master_Spec_v02.103.md:31278` S-01. Raw Diagnostic instances MUST be retained for at least the same duration as the Flight Recorder events they reference (see FR-EVT-004).
  - `Handshake_Master_Spec_v02.103.md:31279` S-02. The Problems index MAY be recomputed; raw instances MUST remain queryable by `Diagnostic.id`.
  - `Handshake_Master_Spec_v02.103.md:31280` S-03. Retention policy MUST be visible in the Operator Consoles (see \xa710.5).
  - `Handshake_Master_Spec_v02.103.md:31281` S-04. Debug Bundle export MUST include a retention summary and any missing evidence due to retention.
  - `Handshake_Master_Spec_v02.103.md:31285` The system MUST maintain a `diagnostics` table in the DuckDB sink for analytical queries.
  - `Handshake_Master_Spec_v02.103.md:31324` Validators are deterministic checks that MUST pass for a compliant implementation.
  - `Handshake_Master_Spec_v02.103.md:31328` - Failures MUST emit a `fatal` Diagnostic from `source=system`.
  - `Handshake_Master_Spec_v02.103.md:31331` - Given the same Diagnostic content (excluding volatile fields), fingerprint MUST be identical across runs.
  - `Handshake_Master_Spec_v02.103.md:31332` - A test suite MUST include platform-specific path inputs and newline variations.
  - `Handshake_Master_Spec_v02.103.md:31335` - For a diagnostic with `link_confidence=direct`, referenced `job_id` and/or `fr_event_ids` MUST exist.
  - `Handshake_Master_Spec_v02.103.md:31336` - For `inferred`, there MUST be exactly one candidate.
  - `Handshake_Master_Spec_v02.103.md:31337` - For `ambiguous`, there MUST be >=2 candidates and they MUST be listed.
  - `Handshake_Master_Spec_v02.103.md:31338` - The UI MUST expose link_confidence and the candidate list (where applicable).
  - `Handshake_Master_Spec_v02.103.md:31341` - Bundle MUST contain all required files (see \xa710.5.6.2).
  - `Handshake_Master_Spec_v02.103.md:31342` - IDs referenced in `coder_prompt.md` MUST exist in bundle contents or be explicitly marked missing with reason.
  - `Handshake_Master_Spec_v02.103.md:31345` - In SAFE_DEFAULT mode, bundle MUST NOT contain secrets/PII matches according to the configured detectors.
  - `Handshake_Master_Spec_v02.103.md:31346` - The redaction report MUST list detectors + versions used.
  - `Handshake_Master_Spec_v02.103.md:31349` - Any operational action (resync/rebuild/export/rerun) MUST have an associated `policy_decision_id` or explicit \u201cpolicy not evaluated\u201d marker, and a Flight Recorder event capturing the decision context.
  - `Handshake_Master_Spec_v02.103.md:31352` - Any operator action that changes state MUST emit a Flight Recorder event with actor=`human` and sufficient refs to navigate back to the initiating UI surface.
  - `Handshake_Master_Spec_v02.103.md:31357` - the console MUST resolve the id to an entity/event,
  - `Handshake_Master_Spec_v02.103.md:31358` - the UI MUST provide a deterministic navigation target (surface + location),
  - `Handshake_Master_Spec_v02.103.md:31359` - failures MUST surface as Diagnostics (not silent no-ops).
  - `Handshake_Master_Spec_v02.103.md:31360` - A test suite MUST cover:

## 11.5 Flight Recorder Event Shapes & Retention
- Spec: `Handshake_Master_Spec_v02.103.md:31366`
- Bounds: `Handshake_Master_Spec_v02.103.md:31366` .. `Handshake_Master_Spec_v02.103.md:31573`
- Purpose (no explicit Why/What blocks): - **Observability Instrumentation (Metrics & Traces):** - **Trace Invariant:** Every AI action MUST emit a unique `trace_id` which links the Job, the RAG QueryPlan, and the final result.
- Roadmap mentions section number: NO
- Normative lines (15):
  - `Handshake_Master_Spec_v02.103.md:31369` - **Trace Invariant:** Every AI action MUST emit a unique `trace_id` which links the Job, the RAG QueryPlan, and the final result.
  - `Handshake_Master_Spec_v02.103.md:31370` - **Span Requirements:** `Workflow::run` and each `JobKind` execution MUST be wrapped in a "Span" (Start/End events recorded in DuckDB).
  - `Handshake_Master_Spec_v02.103.md:31373` - Tracing MUST NOT record `RawContent` payloads unless the job is in `DEBUG_MODE` and the user has explicitly consented.
  - `Handshake_Master_Spec_v02.103.md:31374` - All trace metadata MUST pass through the **Secret Redactor** before being committed to the DuckDB sink.
  - `Handshake_Master_Spec_v02.103.md:31375` - **Tokenization Telemetry:** Telemetry derived from synchronous high-frequency paths (like `TokenizationService`) MUST be emitted asynchronously (fire-and-forget) to ensure the Flight Recorder does not introduce latency into the hot path.
  - `Handshake_Master_Spec_v02.103.md:31376` - **Retention Policy:** Implement an automatic retention policy; traces older than 7 days SHOULD be purged to maintain system performance and storage efficiency.
  - `Handshake_Master_Spec_v02.103.md:31380` The runtime MUST implement the `FlightRecorder` trait for all observability ingestion.
  - `Handshake_Master_Spec_v02.103.md:31386` /// Records a canonical event. MUST validate shape against FR-EVT-* schemas.
  - `Handshake_Master_Spec_v02.103.md:31408` A Flight Recorder event MUST be serializable as a single JSON object. All events MUST include a stable `event_id` and RFC3339 `timestamp`.
  - `Handshake_Master_Spec_v02.103.md:31499` Flight Recorder MUST:
  - `Handshake_Master_Spec_v02.103.md:31507` If evidence is missing due to retention, the UI MUST:
  - `Handshake_Master_Spec_v02.103.md:31536` trace_id: string;               // uuid; REQUIRED
  - `Handshake_Master_Spec_v02.103.md:31537` model_id: string;               // REQUIRED
  - `Handshake_Master_Spec_v02.103.md:31551` Validation requirement: the Flight Recorder MUST reject `llm_inference` events missing `trace_id` or `model_id`.
  - `Handshake_Master_Spec_v02.103.md:31571` FR-EVT-WF-RECOVERY MUST be emitted when HSK-WF-003 transitions a workflow run from Running to Stalled. The actor MUST be 'system'.

## 11.6 Plugin/Matcher Precedence Rules
- Spec: `Handshake_Master_Spec_v02.103.md:31574`
- Bounds: `Handshake_Master_Spec_v02.103.md:31574` .. `Handshake_Master_Spec_v02.103.md:31577`
- Purpose (no explicit Why/What blocks): - Precedence between built-in, workspace, and plugin-defined problem matchers: built-in lowest, workspace overrides built-in, plugin overrides both within declared scope. Conflicts with identical IDs should be resolved in favor of the most specific scope (plugin > workspace > builtin); log conflicts.
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 11.7 OSS Component Choices & Versions
- Spec: `Handshake_Master_Spec_v02.103.md:31578`
- Bounds: `Handshake_Master_Spec_v02.103.md:31578` .. `Handshake_Master_Spec_v02.103.md:32212`
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI), 7.6.4 Phase 2 \u2014 Ingestion & Shadow Workspace (Docling + RAG MVP)
- Normative lines (33):
  - `Handshake_Master_Spec_v02.103.md:31607` The key words **MUST**, **SHOULD**, **MAY** indicate requirement levels.
  - `Handshake_Master_Spec_v02.103.md:31611` For components used *in-process* (`embedded_lib`), Handshake MUST follow this preference order:
  - `Handshake_Master_Spec_v02.103.md:31620` - **GPL/AGPL components MUST NOT be linked into the Handshake app binary.**
  - `Handshake_Master_Spec_v02.103.md:31621` - If a GPL/AGPL tool is used at all, it MUST be integrated as `external_process` or `external_service`, with narrow I/O and capability gating.
  - `Handshake_Master_Spec_v02.103.md:31625` Any third-party component that is shipped, executed, or embedded as part of a Handshake workflow MUST be recorded in the OSS Component Register (see embedded snapshot below), including:
  - `Handshake_Master_Spec_v02.103.md:31634` Tools that parse or execute untrusted input (PDFs, media, plugins) MUST run least-privilege with resource limits, and their outputs MUST be treated as untrusted until validated.
  - `Handshake_Master_Spec_v02.103.md:31638` Every Job MUST declare inputs/outputs and a reproducibility strategy (fixtures + stable hashes, or byte-stable outputs where feasible).
  - `Handshake_Master_Spec_v02.103.md:31643` - Module IDs (e.g., `wood.*`, `creative.*`, `science.*`, `tech.*`) MUST remain stable identifiers.
  - `Handshake_Master_Spec_v02.103.md:31644` - OSS components MUST be referenced by `component_id` (no free-text \u201cwe use X\u201d without a Register entry).
  - `Handshake_Master_Spec_v02.103.md:31645` - If a module definition is moved, the original location MUST retain a pointer to the new location (no silent deletion).
  - `Handshake_Master_Spec_v02.103.md:31661` The key words **MUST**, **SHOULD**, **MAY** are to be interpreted as requirement levels.
  - `Handshake_Master_Spec_v02.103.md:31682` - Legal advice; upstream licenses MUST be verified at the exact pinned version before shipping binaries.
  - `Handshake_Master_Spec_v02.103.md:31703` Handshake MUST follow this preference order for embedded dependencies:
  - `Handshake_Master_Spec_v02.103.md:31715` - Every third-party component MUST appear in the OSS Component Register with:
  - `Handshake_Master_Spec_v02.103.md:31720` - **GPL/AGPL components MUST NOT be linked into the Handshake app binary**. If used at all, they MUST be run as `external_process` or `external_service` with narrow I/O and explicit capability policies.
  - `Handshake_Master_Spec_v02.103.md:31721` - **LGPL/MPL components MAY be embedded**, but Handshake MUST track exact upstream versions and satisfy notice/source obligations for modifications.
  - `Handshake_Master_Spec_v02.103.md:31722` - Where a component has ambiguous or mixed licensing, the Register MUST record the chosen compliance posture (e.g., \u201cCLI-only; do not vendor\u201d). Example: ExifTool has **Artistic-1.0-Perl OR GPL-1.0+**; Czkawka notes a GPL-3 sub-app (\u201cKrokiet\u201d).
  - `Handshake_Master_Spec_v02.103.md:31726` - Any tool that executes or parses **untrusted input** (PDFs, media, plugins) MUST run with least privilege and resource limits; outputs MUST be treated as untrusted until validated.
  - `Handshake_Master_Spec_v02.103.md:31727` - Untrusted user extensions MUST run sandboxed. WASM plugin execution via **Extism + Wasmtime** is an approved pattern.
  - `Handshake_Master_Spec_v02.103.md:31731` - Every Job MUST declare:
  - `Handshake_Master_Spec_v02.103.md:31735` - \u201cDeliverable packaging\u201d jobs SHOULD be deterministic and content-preserving (Typst for rendering; qpdf for structural PDF operations).
  - `Handshake_Master_Spec_v02.103.md:31755` Register entries MUST be expanded before implementation with: pinning, capabilities, fixtures, and compliance notes.
  - `Handshake_Master_Spec_v02.103.md:31821` Approved baseline: **Extism + Wasmtime**. Plugins MUST be capability-scoped and MUST NOT gain arbitrary host process execution by default.
  - `Handshake_Master_Spec_v02.103.md:31855` **Goal:** safety gating via external simulation. CAMotics MUST remain external due to GPL license.
  - `Handshake_Master_Spec_v02.103.md:31907` **Depends on:** `oss.annotorious` (license MUST be verified for the exact package used).
  - `Handshake_Master_Spec_v02.103.md:31968` **Depends on:** `oss.gogit` (preferred), `oss.libgit2` (optional; license is unusual and MUST be recorded as such).
  - `Handshake_Master_Spec_v02.103.md:31998` An implementation claiming conformance to this spec MUST satisfy:
  - `Handshake_Master_Spec_v02.103.md:32008` - Annotorious license: MUST be confirmed at the pinned version before embedding.
  - `Handshake_Master_Spec_v02.103.md:32009` - ExifTool licensing posture: MUST choose a compliance-safe invocation and record it in the Register (CLI-only is default here).
  - `Handshake_Master_Spec_v02.103.md:32010` - Any component used beyond \u201creference/optional external tool\u201d MUST have a pinning + fixture plan before shipping binaries.
  - `Handshake_Master_Spec_v02.103.md:32019` - GPL/AGPL components MUST NOT be linked/embedded into the core app binary (see \xa711.7.4).
  - `Handshake_Master_Spec_v02.103.md:32020` - If a GPL tool is used, it MUST be isolated as an external process with clean IPC boundaries, and treated as an optional adapter.
  - `Handshake_Master_Spec_v02.103.md:32021` - All third-party versions MUST be pinned and recorded in engine/version manifests and ExportRecords where applicable (\xa72.3.10.10).

## 11.8 Mechanical Extension Specification v1.2 (Verbatim)
- Spec: `Handshake_Master_Spec_v02.103.md:32213`
- Bounds: `Handshake_Master_Spec_v02.103.md:32213` .. `Handshake_Master_Spec_v02.103.md:34393`
- Purpose (no explicit Why/What blocks): This section imports the **Mechanical Extension** specification v1.2 as the canonical contract for engine envelopes, gates, capabilities, registry, conformance vectors, and the spec-grade 22-engine set. Heading levels are shifted **+2** to preserve the Master Spec\u2019s heading hierarchy.
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (12):
  - `Handshake_Master_Spec_v02.103.md:32241` - **MUST / MUST NOT**: mandatory requirement.
  - `Handshake_Master_Spec_v02.103.md:32242` - **SHOULD / SHOULD NOT**: strongly recommended; deviations require justification.
  - `Handshake_Master_Spec_v02.103.md:32268` **No-bypass:** engines MUST NOT be invokable outside the orchestrator/runtime.
  - `Handshake_Master_Spec_v02.103.md:32280` - `params` (engine-specific; MUST validate)
  - `Handshake_Master_Spec_v02.103.md:32287` **Size rule:** any payload > 32KB MUST be passed as an input artifact.
  - `Handshake_Master_Spec_v02.103.md:32311` Any `device.*`, `net.http`, or `secrets.use:*` MUST require policy approval and be recorded in provenance.
  - `Handshake_Master_Spec_v02.103.md:32313` Sandbox MUST prevent filesystem escape, deny network unless granted, deny exec unless allowlisted, and record environment identifiers.
  - `Handshake_Master_Spec_v02.103.md:32325` - All artifacts MUST use SHA-256 hashing.
  - `Handshake_Master_Spec_v02.103.md:32326` - Bundles MUST be canonically hashed (stable order, normalized paths, normalized line endings where applicable).
  - `Handshake_Master_Spec_v02.103.md:32327` - Every artifact MUST have sidecar metadata (engine+impl versions, op_id, config hash, inputs hashes, determinism).
  - `Handshake_Master_Spec_v02.103.md:32331` An engine registry (e.g., `mechanical_engines.json`) MUST map:
  - `Handshake_Master_Spec_v02.103.md:32337` All engines MUST pass:

## 11.9 Future Shared Primitives
- Spec: `Handshake_Master_Spec_v02.103.md:34394`
- Bounds: `Handshake_Master_Spec_v02.103.md:34394` .. `Handshake_Master_Spec_v02.103.md:34488`
- Roadmap mentions section number: NO
- Normative lines (0): (none detected by keyword scan)

## 11.10 Implementation Notes: Phase 1 Final Gaps
- Spec: `Handshake_Master_Spec_v02.103.md:34489`
- Bounds: `Handshake_Master_Spec_v02.103.md:34489` .. `Handshake_Master_Spec_v02.103.md:34533`
- Purpose (no explicit Why/What blocks): These notes formalize the technical approach for the final deliverables of Phase 1 (\xa77.6.3).
- Roadmap mentions section number in: 7.6.3 Phase 1 \u2014 Core Product MVP (Single-User, Local AI)
- Normative lines (5):
  - `Handshake_Master_Spec_v02.103.md:34505` 2.  **CSP Policy:** Tauri `asset:` protocol MUST be restricted to the `{APP_DATA}/fonts/` directory.
  - `Handshake_Master_Spec_v02.103.md:34507` 4.  **Import UI:** The system settings or a dedicated Font Manager UI MUST provide an "Import Font" action.
  - `Handshake_Master_Spec_v02.103.md:34519` -   The system MUST check `http://localhost:11434/api/tags` on startup.
  - `Handshake_Master_Spec_v02.103.md:34532` -   **Plan/Snapshot:** Every AI job MUST emit `context_plan.json` and `context_snapshot.json` to the job's artifact directory.
  - `Handshake_Master_Spec_v02.103.md:34533` -   **Validator:** Implement a runtime check in `workflows.rs` that compares the actual retrieval outcome against the `context_plan`. Discrepancies (e.g., budget overrun) MUST emit a `HSK-4002: ContextViolation` diagnostic.
