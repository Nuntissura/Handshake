# Handshake Primitives Catalogue (v1)

Scope: v1 documents primitives (runtime components, content processors, engines, connectors, UI surfaces, governance controls). Cross-tool workflows land in later versions. Order follows `Handshake_Master_Spec_v02.35.md`; no topics skipped (roadmap omitted).

Schema columns: **ID**, **Name**, **Category** (`Runtime`, `ContentPipeline`, `Engine`, `Connector`, `UISurface`, `Index/Search`, `Observability`, `Governance/Safety`, `Helper/Utility`), **Description**, **Reads**, **Writes**. Bullets list **Surfaces**, **Triggers / IsTriggeredBy**, **Notes / Synergy Hooks**.

---

## 1.1 Executive Summary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TAURI_DESKTOP | Tauri Desktop Shell | Runtime | Lightweight desktop host for Handshake UI; embeds React frontend. | Workspace config, UI assets | UI events, capability prompts |
| PRIM_RUST_COORDINATOR | Rust Coordinator | Runtime | Central process managing CRDT state, workflow dispatch, and service bridges. | CRDT docs, workflow graphs, capability tokens | State snapshots, job records |
| PRIM_AI_LOCAL_STACK | Local AI Stack | Engine | Local-first model bundle (LLM, code, vision) prioritizing on-device inference with cloud fallback. | Model weights, prompts | Model outputs, job telemetry |
| PRIM_GOVERNANCE_PORT | Diary Governance Port | Governance/Safety | Code-enforced rules derived from Diary RIDs; guards AI behavior. | RID manifests | Gate outcomes, violations |

- Surfaces: Tauri app windows, command palette, background.  
- Triggers: App launch, user commands, workflow invocations.  
- Notes: Hardware budget favors on-device execution; governance wraps all primitives.

### 1.1.1 TL;DR Box

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WORKSPACE_TRIAD | Workspace Trio (Docs/Canvas/Tables) | UISurface | Unified doc, canvas, and sheet surfaces for creation. | RawContent blocks, canvas nodes, table rows | DisplayContent projections |

- Surfaces: Document editor, canvas, spreadsheet view.  
- Triggers: User navigation, command palette.

### 1.1.2 What We're Building

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOCAL_AI_CLOUD | Local AI Cloud | Runtime | Desktop-hosted environment combining AI agents, governance, and mechanical tools. | Workspace graph, prompts | Job outputs, derived metadata |

- Notes: Agents collaborate; governed mechanical layer executes edits.

### 1.1.3 Key Architecture Decisions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_STACK_CHOICE | Stack Decision Set | Helper/Utility | Tauri + React/TS, Python orchestration, CRDT sync, code-enforced governance. | Config profiles | Build artifacts |

- Triggers: Build/deploy, environment setup.

### 1.1.4 Why Local-First Matters

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOCAL_FIRST_POLICY | Local-First Policy | Governance/Safety | Mandates on-device storage and inference with explicit cloud opt-in. | Workspace data, capability flags | Sync plans, fallback choices |

- Surfaces: Settings, consent dialogs.  
- Triggers: Network loss, model selection.

### 1.1.5 Hardware Context

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VRAM_BUDGETER | VRAM Budget Planner | Helper/Utility | Guides concurrent model loading (e.g., 7B/13B/SDXL) on RTX 3090 class hardware. | Model sizes, current GPU usage | Load/unload plans |

- Notes: Encourages avoiding heavy concurrent workloads; tied to runtime scheduler.

---

## 1.2 The Diary Origin Story

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RID_SYSTEM | RID Governance Corpus | Governance/Safety | Machine-checkable rules (~1,232 clauses across 14 RIDs) forming governance spine. | RID text, clause metadata | Enforcement tables, validation results |

### 1.2.1 The Goal

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DES_001 | Descriptor Extraction RID | ContentPipeline | Extracts structured image descriptors for taste tracking. | RawContent{Image} | DerivedContent{DescriptorRow} |
| PRIM_IMG_001 | Image Analysis RID | ContentPipeline | Mechanical image analysis feeding descriptors. | RawContent{Image} | DerivedContent{ImageFeatures} |
| PRIM_SYM_001 | Symbolic Layer RID | ContentPipeline | Symbolic tagging and vocabulary enforcement. | DerivedContent{Descriptors} | DerivedContent{Symbols} |

### 1.2.2 The Problem

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DRIFT_DETECTOR | Prompt Drift Detector | Governance/Safety | Identifies LLM drift/memory loss affecting rule adherence. | Conversation state | Drift alerts |

### 1.2.3 The Solution (That Became Its Own Project)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LAYER_MODEL | Governance Layers (L1/L2/L3) | Governance/Safety | Immutable/promotion/writable layers controlling content mutation. | Raw/Derived/Display refs | Layer transition records |
| PRIM_GATE_PIPELINE | Governance Gates | Governance/Safety | Validation checkpoints before operations. | Planned operations | Gate verdicts |
| PRIM_MODE_SWITCHER | Work Modes | Governance/Safety | Explicit modes with distinct permissions. | Session state | Mode transitions |
| PRIM_LINT_RULES | Governance Linters | Governance/Safety | Automated compliance checks for prompts and responses. | AI outputs | Lint findings |

### 1.2.4 What Handshake Changes

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CODE_ENFORCEMENT | Code-Enforced Governance | Governance/Safety | Translates governance clauses into compile-time/runtime constraints. | RID corpus | Type-level guards, runtime policy |

- Notes: Shifts from rules-in-context to enforced code; mechanical layer performs writes.

---

## 1.3 The Four-Layer Architecture

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LAYER_LLM | LLM Decision Layer | Engine | Decides what to change; emits structured intents. | Prompts, state snapshot | Structured instructions |
| PRIM_LAYER_ORCH | Orchestrator Layer | Runtime | Translates intents into API calls with capability checks. | Instructions, capabilities | Tool calls, job records |
| PRIM_LAYER_MECH | Mechanical Layer | Engine | Deterministic executors (Docling, CSV transformers, etc.). | RawContent | DerivedContent, artifact mutations |
| PRIM_LAYER_VALIDATION | Validation Layer | Governance/Safety | Confirms outputs via schemas, SHAs, gates. | Operation outputs | Validation receipts |

### 1.3.1 How the Layers Work Together

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LAYER_PIPE | Layered Execution Pipeline | Runtime | Chained flow LLM -> Orchestrator -> Mechanical -> Validation -> User. | Requests | Results/errors |

- Surfaces: Background, job timeline UI.  
- Triggers: User tasks, imports.

---

## 1.4 LLM Reliability Hierarchy

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RELIABILITY_STACK | Reliability Hierarchy | Governance/Safety | Ordered enforcement levels (code -> verbatim markers -> structured output -> explicit state -> rules in context). | Operation plans | Enforcement level metadata |

### 1.4.1 Why This Matters

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_STATE_SNAPSHOT | State Snapshot Injector | Governance/Safety | Provides explicit state every prompt to avoid drift. | Session state | Prompt context |
| PRIM_SCHEMA_VALIDATOR | JSON Schema Validator | Governance/Safety | Ensures structured outputs conform to schemas. | Model outputs | Validation errors |

---

## 1.5 What Gets Ported from the Diary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUST_TYPES | Governance Rust Types | Governance/Safety | Diary concepts mapped to Rust types and traits. | RID definitions | Type-safe APIs |

### 1.5.1 PORTED: Concepts Become Rust Types

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LAYER_GUARD | LayerGuard Type | Governance/Safety | Enforces L1/L2/L3 immutability/promotion rules. | Entity metadata | Guard verdicts |

### 1.5.2 TRANSFORMED: Rules Become Code Enforcement

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GATE_TRAIT | Gate Trait | Governance/Safety | Unified interface for validation gates. | Operation context | Pass/fail + reasons |

### 1.5.3 PRESERVED: The Extraction Core (The Product)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DESCRIPTOR_CORE | Descriptor Core | ContentPipeline | DES/IMG/SYM extraction logic retained as mechanical pipeline. | RawContent{Image/Text} | Derived descriptors |

### 1.5.4 DEPRECATED: Text-Format Specifics

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TEXT_RULES_DEPR | Diary Text Rules | Governance/Safety | Legacy text-format governance kept only for reference. | Diary text | Migration notes |

---

## 1.6 Design Philosophy: Self-Enforcing Governance

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SELF_ENFORCING | Self-Enforcing Stack | Governance/Safety | Governance encoded in architecture and tooling, not prompts. | RID corpus | Enforcement configs |

### 1.6.1 The Problem: Governance Drift

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DRIFT_LEDGER | Governance Drift Ledger | Observability | Tracks drift incidents and remediation. | Flight Recorder events | Drift reports |

### 1.6.2 The Solution: Embedded Enforcement

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EMBEDDED_RULES | Embedded Governance Modules | Governance/Safety | Governance compiled into orchestrator/mechanical layers. | Rule configs | Enforcement hooks |

### 1.6.3 What This Means for Handshake

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ENFORCED_PIPE | Enforced Pipeline | Runtime | All AI and mechanical actions routed through governed pipeline. | Requests | Governed actions |

### 1.6.4 Clause Provenance Pattern

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CLAUSE_PROVENANCE | Clause Provenance Tracker | Observability | Captures which clause authorized/rejected an action. | RID IDs, action logs | Provenance records |

---

## 1.7 Success Criteria

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SUCCESS_METRICS | Success Metric Set | Helper/Utility | Goal counters (reliability, performance, UX) tracked across releases. | Telemetry | KPI dashboards |

---

## 1.8 Introduction

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SPEC_EVOLUTION | Spec Evolution Tracker | Helper/Utility | Tracks spec slices and versioning. | Version history | Change logs |

### 1.8.1 Product Vision & Guiding Principles

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GUIDING_PRINCIPLES | Guiding Principle Set | Governance/Safety | Non-negotiables for local-first, AI-native, governed UX. | Spec text | Policy flags |

### 1.8.2 Specification Evolution

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SLICE_MODEL | Spec Slice Model | Helper/Utility | Incremental slice-by-topic evolution plan. | Topic backlog | Slice checklist |

### 1.8.3 Relationship to Base Research

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RESEARCH_IMPORT | Research Import Pipeline | ContentPipeline | Pulls validated research artifacts into governed product spec. | Research docs | Normalized references |

---

## 2.1 High-Level Architecture

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SHADOW_WORKSPACE | Shadow Workspace | Index/Search | Background parser/chunker/embedder feeding retrieval. | RawContent | Index shards, DerivedContent |
| PRIM_CAPABILITY_SYSTEM | WASI-Style Capability System | Governance/Safety | Scoped, time-limited permissions for tools/agents. | Capability configs | Tokens, audit logs |
| PRIM_FLIGHT_RECORDER | Flight Recorder | Observability | Logs prompts, tool calls, workflows for replay. | Job streams | DuckDB logs |

### 2.1.1 Desktop Shell & Coordinator

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TAURI_APP | Tauri App Shell | UISurface | Desktop container hosting React UI. | UI assets | UI events |
| PRIM_RUST_CORE | Rust Core Coordinator | Runtime | Manages CRDT docs, workflows, service bridges. | Workspace graph | State updates |

### 2.1.2 Workspace Data Layer

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SQLITE_STORE | SQLite Workspace Store | Runtime | Persists docs/canvases/tables with CRDT data. | CRDT ops | Snapshots |
| PRIM_GRAPH_STORE | Embedded Graph Store | Index/Search | Graph-relational DB (Cozo/Kuzu/DuckDB ext). | Entity edges | Graph projections |
| PRIM_CRDT_ENGINE | CRDT Engine (Yjs class) | Runtime | Real-time collaboration including AI site IDs. | CRDT updates | Merged document state |

### 2.1.3 Model Runtime Layer

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_OLLAMA | Ollama Runtime Adapter | Engine | HTTP bridge to local Ollama-hosted models. | Prompts | Completions |
| PRIM_RUNTIME_VLLM | vLLM/TGI Runtime Adapter | Engine | High-throughput server integration for long contexts. | Prompts | Completions |
| PRIM_RUNTIME_COMFYUI | ComfyUI Bridge | Engine | Image generation/graph execution backend. | Graph configs | Image artifacts |
| PRIM_MCP_GATEWAY | MCP Gateway | Connector | Routes MCP server calls through Rust gate. | MCP requests | MCP responses |

### 2.1.4 Automation & Workflow Engine

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WORKFLOW_RUNTIME | Local Workflow Runtime | Runtime | Typed node-graph executor with durable state. | Workflow defs, triggers | Job instances, checkpoints |
| PRIM_AI_JOB_RUNNER | AI Job Runner | Runtime | Executes AI jobs (LLM, embeddings, ASR, imports). | Job specs | Artifacts, logs |

### 2.1.5 Observability & Flight Recorder

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DUCKDB_RECORDER | DuckDB Flight Recorder | Observability | Structured event sink and replay dataset. | Event envelopes | Queryable logs |

### 2.1.6 Capability & Security Layer

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CAPABILITY_TOKEN | Capability Token Service | Governance/Safety | Issues, scopes, and audits capability tokens. | Policy configs | Tokens, denials |

### 2.1.7 Connectors & External Data Layer

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_JMAP_CONNECTOR | JMAP Email Connector | Connector | Imports/syncs mail via JMAP into workspace graph. | Mailboxes | RawContent{Email}, Derived summaries |
| PRIM_CALDAV_CONNECTOR | CalDAV Connector | Connector | Syncs calendar events with idempotent outbox. | Calendar feed | RawContent{Events} |
| PRIM_WEBHOOK_CONNECTOR | Webhook/HTTP Connector | Connector | Generic inbound/outbound HTTP hooks. | HTTP payloads | Workflow triggers |
| PRIM_MCP_CONNECTOR | External MCP Connector | Connector | Prefers MCP servers for structured knowledge. | MCP schemas | RawContent/DerivedContent |

### 2.1.8 AI UX & Interaction Layer

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_COMMAND_PALETTE | Command Palette | UISurface | Entry point for explicit tasks and tool calls. | User intents | Job starts |
| PRIM_STRUCTURAL_EDITOR | Structural Editor | UISurface | Contextual refactors and transforms. | Entity refs | Mutations (via mechanical layer) |
| PRIM_BACKGROUND_AGENT | Background Agent Surface | UISurface | Passive suggestions/linking/clustering. | Workspace signals | Suggestions |

### 2.1.9 Taste Engine & Personalisation Layer

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TASTE_ENGINE | Taste Engine | Engine | Builds user taste vectors and descriptors for prompt injection. | DescriptorRows, embeddings | Taste profiles |

### 2.1.10 Dev Tools & Extension Platform

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DEV_TERMINAL | Integrated Terminal | UISurface | Embedded terminal with policy/sandbox modes. | Commands | Output streams |
| PRIM_EXTENSION_API | Extension API | Connector | Plugin/script interface with sandboxing. | Plugin manifests | Plugin effects |

### 2.1.11 Hardware Context: The RTX 3090 Setup

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VRAM_PLANNER | VRAM Planner | Helper/Utility | Calculates safe concurrent model loads. | Model sizes, GPU stats | Load plans |

### 2.1.11.1 VRAM Budget

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VRAM_BUDGET_RULES | VRAM Budget Rules | Governance/Safety | Guardrails for model concurrency on 24GB VRAM. | Runtime telemetry | Scheduler constraints |

### 2.1.11.2 Speed: GPU vs CPU

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_PLACER | Runtime Placement Advisor | Helper/Utility | Chooses GPU vs CPU placement based on speed/capacity. | Model specs, load | Placement decisions |

### 2.1.11.3 Practical Rules of Thumb

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONCURRENCY_RULES | Model Concurrency Rules | Governance/Safety | Limits mixing SDXL + large LLMs; reserves context headroom. | Workload queue | Admission outcomes |

### 2.1.12 Architecture Block Diagram

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ARCH_BLOCK | Architecture Block | Helper/Utility | Canonical mapping of UI -> Orchestrator -> Runtimes -> Data. | Topology configs | Reference diagrams |

---

## 2.2 Data & Content Model

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_UNIFIED_NODE | Unified Node Schema | ContentPipeline | Logical super-node for blocks/canvas/workflow nodes. | RawContent | Normalized nodes |

### 2.2.0 Tool Integration Principles

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TOOL_AGNOSTIC_CORE | Tool-Agnostic Core Schema | Governance/Safety | Enforces single workspace graph for all tools. | Entity metadata | Policy checks |

### 2.2.1 Core Entities

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ENTITY_WORKSPACE | Workspace Entity | Runtime | Root container for projects/resources. | Workspace manifest | Project refs |
| PRIM_ENTITY_DOC | Document Entity | Runtime | CRDT-EUR'based block tree. | Blocks | CRDT patches |
| PRIM_ENTITY_CANVAS | Canvas Entity | Runtime | Spatial layout of nodes/edges. | Canvas nodes | Placement updates |
| PRIM_ENTITY_TABLE | Table Entity | Runtime | Schema + rows for structured data. | Rows | Cell updates |
| PRIM_ENTITY_TASK_EVENT | Task/Event Entity | Runtime | Structured scheduling/assignment object. | Task data | Status changes |
| PRIM_ENTITY_ASSET | Asset Entity | Runtime | Files/media stored with metadata. | File blobs | Metadata updates |
| PRIM_ENTITY_EXTERNAL | External Resource Entity | Connector | Mail/calendar/web items mapped into workspace. | External payloads | Workspace projections |
| PRIM_ENTITY_WORKFLOW | Workflow Entity | Runtime | Node graph operating on workspace/external resources. | Workflow defs | Run history |

#### 2.2.1.1 Unified Node Schema (Logical Super-Node)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOGICAL_NODE | Logical Node Contract | ContentPipeline | Fields for id/content/parent/graph/spatial/kernel state. | Node storage | Node projections |

### 2.2.2 Raw / Derived / Display: Formal Specification

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RAW_CONTENT | RawContent Layer | ContentPipeline | Canonical user/external content; immutable except promotion rules. | Canonical files | Raw state |
| PRIM_DERIVED_CONTENT | DerivedContent Layer | ContentPipeline | AI-generated metadata, embeddings, summaries. | RawContent | Derived artifacts |
| PRIM_DISPLAY_CONTENT | DisplayContent Layer | UISurface | UI projections of raw+derived with policy filters. | Raw/Derived | Rendered payloads |

#### 2.2.2.1 RawContent

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RAW_IMPORTER | Raw Importer | ContentPipeline | Ingests canonical external data (mail, PDFs, code). | External sources | RawContent files |

#### 2.2.2.2 DerivedContent

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DERIVED_GENERATOR | Derived Generator | ContentPipeline | Creates embeddings/summaries/tags/plans. | RawContent | DerivedContent |

#### 2.2.2.3 DisplayContent

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DISPLAY_RENDERER | Display Renderer | UISurface | Combines raw+derived under policy to render UI. | Raw/Derived | Rendered views |

## 2.3 Content Integrity (Diary Part 5: COR-700)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONTENT_INTEGRITY | Content Integrity Module | Governance/Safety | Enforces no in-system censorship; governs promotions. | Raw/Derived metadata | Integrity receipts |

### 2.3.1 Core Principle: No In-System Censorship

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EXPORT_ONLY_SAFETY | Export Safety Gate | Governance/Safety | Safety filters apply only on export/display, not storage. | Display requests | Filtered outputs |

### 2.3.2 Export-Only Safety

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EXPORT_FILTER | Export Filter | Governance/Safety | Applies policy to outbound content. | Raw/Derived | Export artifacts |

### 2.3.3 Mapping to Raw/Derived/Display

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LAYER_MAPPING | Layer Mapping Rules | Governance/Safety | Rules mapping integrity policies to layers. | Layer metadata | Enforcement logs |

### 2.3.4 Validator Integration

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VALIDATOR_HOOK | Validator Hook | Governance/Safety | Integrates validators into pipelines. | Operation outputs | Validation results |

### 2.3.5 Data Architecture: File-Tree Model

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FILETREE_STORE | File-Tree Store | Runtime | Human-readable file layout with sidecars. | Files | Structured paths |

### 2.3.6 File Integrity & Promotion (Diary FIH-001)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PROMOTION_GATE | Promotion Gate | Governance/Safety | Controls RawaDerivedaDisplay promotions. | Integrity proofs | Promotion logs |

### 2.3.7 Knowledge Graph & Storage

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_KG_SCHEMA | Knowledge Graph Schema | Index/Search | Graph schema for entities/relations. | Entity refs | Graph edges |

### 2.3.8 Shadow Workspace & Indexing Pipeline

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PARSER_CHUNKER | Parser + Chunker | ContentPipeline | Tree-sitter parsing and chunking for indexing. | RawContent | Chunks |
| PRIM_EMBEDDER | Embedder Engine | Engine | Generates embeddings for search/taste. | Chunks | Embedding vectors |
| PRIM_INDEXER | Index Writer | Index/Search | Writes embeddings/chunks into vector/graph indexes. | Embeddings | Index shards |

### 2.3.9 CRDT & Sync Model (Human-AI Collaboration)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SYNC_ENGINE | Sync Engine | Runtime | Syncs CRDT state across devices/agents. | CRDT ops | Replicated state |

### 2.3.11 Taste Engine & Personalisation

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TASTE_DESCRIPTOR | Taste Descriptor | ContentPipeline | JSON taste profile fed to prompts. | DescriptorRows | Taste vectors |

---

## 2.4 Extraction Pipeline (The Product)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EXTRACTION_PIPE | Extraction Master Pipeline | ContentPipeline | Unified pipeline orchestrating IMG/DES/SYM stages. | RawContent{Image/Text} | Derived descriptors, artifacts |

### 2.4.1 Pipeline Overview

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_STAGE_ROUTER | Pipeline Stage Router | Runtime | Routes inputs to modality-specific paths. | Input metadata | Stage tasks |

### 2.4.2 IMG-001: Image Extraction Pipeline

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_IMG_EXTRACTOR | Image Descriptor Extractor | Engine | Applies image analysis and descriptor generation. | RawContent{Image} | DescriptorRows, DerivedContent |

### 2.4.3 SYM-001: Symbolic Engine

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SYMBOLIC_ENGINE | Symbolic Engine | Engine | Applies symbolic tagging/vocabulary alignment. | DescriptorRows | Symbol annotations |

### 2.4.4 Integration: The Complete Flow

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PIPE_INTEGRATION | Pipeline Integration Layer | Runtime | Connects IMG/DES/SYM with validation and storage. | Stage outputs | Consolidated artifacts |

### 2.4.4 TXT-001 - Text Descriptor Pipeline

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TXT_DESCRIPTOR | Text Descriptor Pipeline | ContentPipeline | Extracts structured descriptors from text sources. | RawContent{Text} | DescriptorRows |

---

## 2.5 AI Interaction Patterns

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AI_INTERACTION | AI Interaction Framework | Runtime | Patterns for AI tasks across docs/canvas/tables. | Workspace context | AI intents |

### 2.5.1 AI Stack & Model Roles

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MODEL_ROLESET | Model Role Set | Engine | Specialized model roles (planning, coding, vision, image gen). | Prompts | Role outputs |

### 2.5.2 Hosting & Runtime Topology

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_TOPOLOGY | Runtime Topology Manager | Runtime | Manages local runtimes (Ollama/vLLM/ComfyUI) and placement. | Hardware stats | Runtime routes |

### 2.5.3 Routing & Session Management

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SESSION_ROUTER | Session Router | Runtime | Routes prompts to models, manages session state. | Session context | Routed calls |

### 2.5.4 Multi-Agent Patterns

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MULTI_AGENT | Multi-Agent Orchestrator | Runtime | Lead/worker and fallback agent patterns. | Tasks | Agent assignments |

### 2.5.5 AI Operations on Docs

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOC_OPS | Document AI Ops | Engine | Structured editing/search/summarize on doc blocks. | RawContent{Doc} | Derived summaries/edits |

### 2.5.6 AI Operations on Canvases

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CANVAS_OPS | Canvas AI Ops | Engine | Spatial clustering, layout, card generation. | Canvas nodes | Layout updates |

### 2.5.7 AI Operations on Tables

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TABLE_OPS | Table AI Ops | Engine | Formula synthesis, cell fills, typed transforms. | Table rows | Cell edits |

### 2.5.8 Project Brain (RAG Interface)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PROJECT_BRAIN | Project Brain RAG | Index/Search | Retrieval layer over Shadow Workspace + KG. | Index shards | Ranked contexts |

### 2.5.9 Thinking Pipeline (Docs a Canvas a Workflows)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_THINKING_PIPELINE | Thinking Pipeline | Runtime | Moves ideas across docs/canvas/workflow nodes. | Entity refs | Cross-surface artifacts |

### 2.5.10 Docs & Sheets AI Job Profile

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOC_SHEET_JOB | Docs/Sheets AI Job Profile | Runtime | Standardized job schema for text/table operations. | EntityRefs | Job outputs |

---

## 2.6 Workflow & Automation Engine

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WORKFLOW_MODEL | Workflow Model | Runtime | Node graph model with triggers/control flow. | Workflow defs | Executions |

### 2.6.1 Goals & Constraints

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WORKFLOW_CONSTRAINTS | Workflow Constraints | Governance/Safety | Durable execution, resumability, safety hooks. | Runs | Constraint checks |

### 2.6.2 Workflow Model

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WORKFLOW_NODES | Workflow Node Types | Runtime | Trigger, AI, workspace ops, control flow nodes. | Inputs | Node outputs |

### 2.6.3 AI-Assisted Workflow Design

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WORKFLOW_ASSISTANT | Workflow Design Assistant | UISurface | Assists building node graphs via AI suggestions. | User goals | Proposed graphs |

### 2.6.4 Execution & Durability

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DURABLE_EXEC | Durable Executor | Runtime | Temporal-inspired executor with checkpoints/retries. | Run state | Checkpoints, retries |

### 2.6.5 Safety & Validation Pipeline

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WORKFLOW_SAFETY | Workflow Safety Gates | Governance/Safety | Gate pipeline per node/job. | Node outputs | Safety verdicts |

### 2.6.6 AI Job Model (Global)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AI_JOB_MODEL | Global AI Job Model | Runtime | Unified schema for jobs (entity refs, inputs, outputs, validators). | Job specs | Job logs |

---

## 2.7 Response Behavior Contract (Diary ANS-001)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BEHAVIOR_CONTRACT | Response Behavior Contract | Governance/Safety | Defines allowed behaviors/modes for AI replies. | Prompts, modes | Structured responses |

### 2.7.1 The Behavior Contract

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONTRACT_SCHEMA | Contract Schema | Governance/Safety | Structured output formats enforced on responses. | Response drafts | Validated responses |

### 2.7.2 Behavior by Mode

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MODE_TABLE | Mode Table | Governance/Safety | Mode-specific permissions (analysis/build/reflect). | Mode config | Mode state |

### 2.7.3 Prohibitions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PROHIBITIONS | Prohibition Set | Governance/Safety | Disallowed behaviors (guessing, fabrications). | Responses | Violations |

### 2.7.4 Integration with AI Job Model

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONTRACT_JOB_BRIDGE | Contract a Job Bridge | Runtime | Binds response contract to job schema. | Job context | Response envelopes |

### 2.7.5 Validation Gates (from Diary COR-701)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ANS_VALIDATORS | Answer Validators | Governance/Safety | Gate set applied to responses before commit. | Response payload | Validation results |

---

## 2.8 Governance Runtime (Diary Parts 1-2)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BOOTLOADER | Governance Bootloader | Governance/Safety | Initializes runtime behavior and capabilities. | Boot config | Runtime state |

### 2.8.1 Bootloader: Runtime Behavior (Part 1)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BOOT_STEPS | Bootloader Steps | Governance/Safety | Sequence enforcing boot invariants. | Boot settings | Boot logs |

### 2.8.2 Execution Charter: Bootstrap & Capabilities (Part 2)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EXEC_CHARTER | Execution Charter | Governance/Safety | Capability declarations and bootstrap rules. | Charter clauses | Capability tables |

---

## 2.9 Deterministic Edit Process (COR-701)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DETERMINISTIC_EDIT | Deterministic Edit Pipeline | Governance/Safety | Stepwise editing with gates, micro-steps, and validated outputs. | Edit requests | Patched entities |

### 2.9.1 Purpose & Core Types

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ENTITY_REF | EntityRef Type | Governance/Safety | Stable references to blocks/rows/nodes used in edits. | Entity ids | Ref payloads |

### 2.9.2 Definitions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EDIT_TERMS | Edit Definitions | Helper/Utility | Definitions for patch/mutation semantics. | Spec text | Glossary entries |

### 2.9.3 Gate Pipeline

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EDIT_GATES | Edit Gate Pipeline | Governance/Safety | Validators run before/after edits (schema, SHA). | Candidate edits | Gate results |

### 2.9.4 Micro-Steps

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MICRO_STEP | Micro-Step Executor | Runtime | Granular steps sequencing edits safely. | Edit plan | Applied micro-steps |

### 2.9.5 Assistant Behavior

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASSISTANT_BEHAVIOR | Assistant Edit Behavior | Governance/Safety | Required behaviors (ask/act/report) when editing. | Requests | Structured replies |

### 2.9.6 Reply Format Requirements

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_REPLY_SCHEMA | Reply Schema | Governance/Safety | Required reply structure for edits/jobs. | Assistant outputs | Validated replies |

---

## 2.10 Session Logging (LOG-001)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SESSION_LOGGER | Session Logger | Observability | Records session state, hygiene, and task ledger. | Session data | Log entries |

### 2.10.1 Session State (L001-50 to L001-58)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SESSION_STATE | Session State Snapshot | Observability | Captures active session metadata. | Session context | State rows |

### 2.10.2 Hygiene Rules (L001-40 to L001-51)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_HYGIENE_RULES | Session Hygiene Rules | Governance/Safety | Sanitization and handling requirements. | Requests | Hygiene actions |

### 2.10.3 Task Ledger (L001-110 to L001-145)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TASK_LEDGER | Task Ledger | Observability | Append-only ledger of tasks/jobs. | Job metadata | Ledger entries |

---

## 3.1 Local-First Data Fundamentals

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOCAL_DATA | Local Data Primitive | Runtime | Emphasizes offline-first storage and sync. | Local files | Sync manifests |

### 3.1.1 The Problem: Concurrent Editing

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONCURRENCY_ISSUES | Concurrent Edit Detector | Observability | Highlights conflicts in shared edits. | CRDT ops | Conflict alerts |

### 3.1.2 Solution: CRDTs Explained

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CRDT_LIB | CRDT Primitive | Runtime | CRDT-based merge semantics for documents/canvases. | CRDT streams | Merged state |

---

## 3.2 CRDT Libraries Comparison

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_YJS | Yjs Library | Runtime | Candidate CRDT implementation. | Updates | Document state |
| PRIM_AUTOMERGE | Automerge | Runtime | Alternative CRDT option. | Updates | Document state |
| PRIM_LORO | Loro | Runtime | Emerging CRDT option. | Updates | Document state |

### 3.2.1 Yjs Deep Dive

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_YJS_DETAILS | Yjs Profile | Runtime | Yjs features and adapter strategy. | Yjs docs | Adapter configs |

### 3.2.2 Automerge Deep Dive

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AUTOMERGE_DETAILS | Automerge Profile | Runtime | Automerge characteristics and tradeoffs. | Library docs | Adapter configs |

### 3.2.3 Loro and Emerging Options

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LORO_DETAILS | Loro Profile | Runtime | Loro evaluation for potential adoption. | Library docs | Adapter configs |

### 3.2.4 Recommendation: Which CRDT Librarya

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CRDT_RECOMMEND | CRDT Recommendation | Helper/Utility | Preferred CRDT choice (Yjs) for Handshake. | Library comparisons | Decision record |

---

## 3.3 Database & Sync Patterns

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CRDT_DB_COMBO | CRDT + DB Pattern | Runtime | Combines CRDT with SQLite/graph storage. | CRDT logs | DB state |

### 3.3.1 Combining CRDT and Database

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CRDT_DB_BRIDGE | CRDT-DB Bridge | Runtime | Materializes CRDT to DB tables and back. | CRDT ops | Materialized views |

### 3.3.2 Sync Topologies

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SYNC_TOPOLOGY | Sync Topology Set | Runtime | P2P/relay/cloud fallback sync patterns. | Sync configs | Sync state |

---

## 3.4 Conflict Resolution UX

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONFLICT_UI | Conflict Resolution UI | UISurface | User-facing conflict resolution for CRDT merges. | Conflict deltas | User resolutions |

### 3.4.1 User-Facing Conflict Patterns

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONFLICT_PATTERNS | Conflict Pattern Library | Helper/Utility | Patterns for presenting/merging conflicts. | Conflict data | Resolutions |

### 3.4.2 Version History UI

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VERSION_HISTORY | Version History Surface | UISurface | Timeline of changes with rollback/snapshot. | Change log | Restore operations |

---

## 4.1 LLM Infrastructure

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LLM_INFRA | LLM Infra Overview | Engine | Foundational concepts (tokens, quantization, sizes). | Model specs | Guidance |

### 4.1.1 How LLMs Work (Simplified)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LLM_KNOWLEDGE | LLM Primer | Helper/Utility | Simplified explanation for implementers. | Primer text | Docs |

### 4.1.2 Key Concepts: Tokens, VRAM, Quantization

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MODEL_SIZING | Model Sizing Rules | Helper/Utility | VRAM math and quantization guidance. | Model stats | Sizing tables |

### 4.1.3 Model Sizes and What Fits

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MODEL_FIT_GUIDE | Fit Guide | Helper/Utility | Which models fit on target hardware. | Hardware profile | Fit recommendations |

---

## 4.2 LLM Inference Runtimes

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_OVERVIEW | Runtime Overview | Engine | Compares inference runtimes. | Runtime metrics | Selection notes |

### 4.2.1 What is an Inference Runtimea

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_DEFINITION | Runtime Definition | Helper/Utility | Definition and responsibilities. | Docs | Reference |

### 4.2.2 Runtime Comparison: Ollama vs vLLM vs TGI vs Others

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_COMPARISON | Runtime Comparison Table | Helper/Utility | Tradeoffs across runtimes. | Benchmarks | Decision log |

### vLLM - The Performance Champion

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VLLM_CHAMPION | vLLM Performance Profile | Engine | Highlights vLLM strengths for throughput. | Perf data | Runtime config |

### 4.2.3 Recommended Runtime Strategy

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_STRATEGY | Runtime Strategy | Helper/Utility | Preferred runtime mix and routing. | Usage patterns | Strategy doc |

### 4.3 Model Selection & Roles

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MODEL_SELECTION | Model Selection Matrix | Helper/Utility | Model recommendations per role. | Task types | Selection table |

### 4.3.1 Specialized Models for Different Tasks

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ROLE_SPECIALISTS | Role-Specialized Models | Engine | Specialized models for code, chat, vision, planning. | Task metadata | Role assignment |

### 4.3.2 Model Recommendations by Role

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ROLE_RECS | Role Recommendation Table | Helper/Utility | Mapping tasksamodels. | Task matrix | Rec list |

### 4.3.3 GPU Memory Management

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GPU_MEMORY_MGR | GPU Memory Manager | Runtime | Manages VRAM allocation/eviction. | Model load | Eviction events |

### 4.3.4 Scheduling & Contention

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUNTIME_SCHEDULER | Runtime Scheduler | Runtime | Schedules concurrent model jobs avoiding contention. | Job queue | Schedules |

### 4.3.6 Local Model Runtimes

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOCAL_RUNTIME_SET | Local Runtime Set | Engine | Supported local runtimes (Ollama/vLLM/llama.cpp). | Runtime configs | Launch commands |

### 4.3.8 ComfyUI Workflow Integration

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_COMFYUI_INTEGRATION | ComfyUI Integration | Engine | Bridges ComfyUI workflows into AI job model. | Workflow graphs | Generated images |

### 4.4 Image Generation (Stable Diffusion)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SDXL_ENGINE | SDXL Engine | Engine | Stable Diffusion/SDXL generation backend. | Prompts, seeds | Image artifacts |

### 4.4.1 SD vs SDXL Overview

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SD_CHOICE | SD/SDXL Choice Guide | Helper/Utility | Guidance on SD vs SDXL usage. | Prompt type | Choice notes |

### 4.4.2 VRAM Requirements & Performance

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SD_VRAM | SD VRAM Planner | Helper/Utility | VRAM/perf guide for SD/SDXL with LLM coexistence. | VRAM stats | Load plan |

### 4.4.3 Integrating with LLM Workloads

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SD_LLM_COORD | SD-LLM Coordinator | Runtime | Schedules image gen alongside LLM jobs safely. | Job queue | Coordinated runs |

### 4.5 Model Orchestration Policy

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ORCH_POLICY | Model Orchestration Policy | Governance/Safety | Policy governing model selection, fallback, and capability scope. | Model roster | Policy decisions |

---

## 5.1 Plugin Architecture

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_SYSTEM | Plugin System | Connector | Manifest-driven plugin registration and execution. | Plugin manifests | Plugin state |

### 5.1.1 Why Plugins Matter

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_VALUE | Plugin Value Primitive | Helper/Utility | Rationale for extensibility. | Use cases | Plugin backlog |

### 5.1.2 Learning from Existing Systems

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_LESSONS | Plugin Lessons | Helper/Utility | Patterns adopted from other systems. | Comparative notes | Design choices |

### 5.1.3 Plugin Manifest & Registration

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_MANIFEST | Plugin Manifest Schema | Connector | Declares plugin metadata, capabilities, permissions. | Manifest files | Registry entries |

### 5.1.4 Plugin Types & Categories

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_TYPES | Plugin Type Matrix | Helper/Utility | Categorizes plugins (tools, views, automation). | Plugin definitions | Type map |

### 5.1.5 API Design Patterns

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_API | Plugin API | Connector | Patterns for safe, typed plugin APIs. | API calls | Responses |

---

## 5.2 Sandboxing & Security

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SANDBOX_LAYER | Sandbox Layer | Governance/Safety | Isolation for untrusted code and mechanical runners. | Commands | Sandboxed outputs |

### 5.2.1 Why Sandbox Untrusted Code

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SANDBOX_RATIONALE | Sandbox Rationale | Helper/Utility | Reasons and threat model. | Threat assessments | Policy |

### 5.2.2 Sandboxing Technologies Compared

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SANDBOX_TECH | Sandbox Tech Matrix | Helper/Utility | Compares container/VM/wasm sandboxes. | Tech specs | Selection record |

### 5.2.3 Permission Models

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PERMISSION_MODEL | Permission Model | Governance/Safety | Capability scopes and consent prompts. | Policy | Tokens |

### 5.2.4 Recommended Security Architecture

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SECURITY_ARCH | Security Architecture | Governance/Safety | Layered security with gates and sandbox runner. | Security configs | Enforcement |

### 5.2.4 Mechanical Runner Sandbox

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_SANDBOX | Mechanical Runner Sandbox | Governance/Safety | Sandbox for Docling/ASR/computation engines. | Jobs | Sandboxed artifacts |

---

## 5.3 AI Observability

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OBSERVABILITY | AI Observability Stack | Observability | Metrics, tracing, privacy-sensitive logging. | Telemetry streams | Dashboards |

### 5.3.1 What to Monitor in AI Apps

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MONITOR_SET | Monitor Signal Set | Observability | Key signals (latency, prompts, errors, cost). | Runtime metrics | Metrics store |

### 5.3.2 Tools Comparison

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OBS_TOOL_MATRIX | Observability Tool Matrix | Helper/Utility | Evaluates tracing/logging tools. | Tool specs | Selection |

### 5.3.3 Privacy-Sensitive Logging

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PRIVACY_LOGGING | Privacy Logging Rules | Governance/Safety | Redaction/anonymization policies. | Event payloads | Sanitized logs |

### 5.3.4 Metrics & Dashboards

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_METRIC_DASH | Metrics & Dashboards | Observability | Dashboard definitions for AI ops. | Metrics | Visuals |

### 5.3.6 Distillation Observability Requirements

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DISTILL_OBS | Distillation Observability | Observability | Logging requirements for skill distillation. | Distill runs | Logs |

### 5.3.7 Log Privacy & Retention

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RETENTION_POLICY | Log Retention Policy | Governance/Safety | Privacy-aware retention/deletion. | Logs | Retention actions |

---

## 5.4 Evaluation & Quality

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_EVAL_STACK | Evaluation Stack | Observability | Testing harness for LLM outputs and agents. | Test suites | Eval results |

### 5.4.1 Testing LLM Outputs

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OUTPUT_TESTS | Output Test Suite | Governance/Safety | Tests for formatting, content, safety. | Model outputs | Test reports |

### 5.4.2 Multi-Agent Tracing

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AGENT_TRACING | Multi-Agent Tracer | Observability | Traces interactions among agents. | Agent events | Trace logs |

### 5.4.6 Governance Compliance Tests

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GOV_TESTS | Governance Compliance Tests | Governance/Safety | Validates adherence to clauses/gates. | System state | Compliance reports |

---

## 5.5 Benchmark Harness

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BENCHMARK | Benchmark Harness | Observability | Scenario runner for performance/quality benchmarking. | Scenarios | Benchmark results |

### 5.5.1 Benchmark Architecture

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BENCH_ARCH | Benchmark Architecture | Observability | Architecture for adapters and reporting. | Adapter configs | Bench logs |

### 5.5.2 Scenarios & Adapters

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BENCH_ADAPTERS | Benchmark Adapters | Helper/Utility | Scenario adapters to tools/runtimes. | Scenario defs | Adapter outputs |

### 5.5.3 Reporting & Analysis

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BENCH_REPORTS | Benchmark Reports | Observability | Reporting pipeline for benchmark runs. | Bench data | Reports |

### 5.5.4 Summary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BENCH_SUMMARY | Benchmark Summary | Helper/Utility | Aggregated findings. | Bench results | Summary |

### 5.5.5 Findings

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BENCH_FINDINGS | Benchmark Findings | Helper/Utility | Key findings captured as primitives. | Bench runs | Findings log |

### 5.5.6 Recommendations

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BENCH_RECS | Benchmark Recommendations | Helper/Utility | Actionable recommendations. | Findings | Rec list |

---

## 6.0 Mechanical Tool Bus & Integration Principles

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_BUS | Mechanical Tool Bus | Connector | Abstraction layer for mechanical engines. | Tool manifests | Tool calls |

---

## 6.1 Document Ingestion: Docling Subsystem

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_ENGINE | Docling Engine | Engine | Document ingestion/parsing with layout/structure extraction. | RawContent{PDF/Doc} | DerivedContent{DoclingDocument} |

### 6.1.1 Part I - Docling Evaluation for Project Handshake (GPT research v1)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_EVAL_V1 | Docling Eval v1 | Helper/Utility | Initial evaluation results guiding integration. | Eval data | Findings |

### 6.1.2 Docling Evaluation for Project Handshake

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_EVAL_V2 | Docling Eval v2 | Helper/Utility | Updated assessment. | Eval data | Findings |

### 6.1.3 Part II - Docling Integration Assessment for Project Handshake (Spec-style)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_SPEC | Docling Integration Spec | ContentPipeline | Spec for integrating Docling into workspace model. | Pipeline requirements | Integration plan |

### 6.1.4 Docling Integration Assessment for Project Handshake

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_ASSESS | Docling Assessment | Helper/Utility | Summarized findings for integration. | Assessment docs | Action items |

### 6.1.5 Part III - Architectural Evaluation of IBM Docling for the Handshake Workspace

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_ARCH | Docling Architectural Evaluation | Helper/Utility | Architectural fit analysis. | Architecture docs | Decisions |

### 6.1.6 Docling AI Job Profile

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_JOB | Docling AI Job Profile | Runtime | Standard job profile for Docling imports. | Files | Parsed docs, tables |

### 6.1.11 Chunking, Embeddings, and Indexing Config (Docling + Shadow Workspace)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_CHUNK_CONFIG | Docling Chunk/Embed Config | ContentPipeline | Config for chunking/embedding imported docs. | Docling output | Index-ready chunks |

---

## 6.2 Speech Recognition: ASR Subsystem

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_ENGINE | ASR Engine | Engine | Speech recognition pipeline with batch/stream modes. | Audio streams/files | Transcripts, timings |

### 6.2.1 X.1 Goals, Scope, and Constraints

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_SCOPE | ASR Scope | Helper/Utility | Goals and constraints of ASR integration. | Spec | Scope log |

### 6.2.2 X.2 Model Landscape and Selection Rationale

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_SELECTION | ASR Model Selection | Helper/Utility | Evaluates ASR models. | Benchmarks | Selection |

### 6.2.3 X.3 Handshake ASR Architecture

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_ARCH | ASR Architecture | Runtime | Architecture for ASR ingestion + integration. | Audio | Transcripts |

### 6.2.4 X.4 Runtime Modes: Batch vs Streaming

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_MODES | ASR Runtime Modes | Runtime | Batch and streaming modes. | Audio input | Mode-specific outputs |

### 6.2.5 X.5 Customization and Fine-Tuning

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_CUSTOMIZE | ASR Customization | Engine | Fine-tuning/custom vocabulary hooks. | Training data | Tuned models |

### 6.2.6 X.6 Post-Processing, Diarization, and LLM Tools

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_POST | ASR Post-Processor | ContentPipeline | Diarization/punctuation/segmentation; LLM tool tie-ins. | Raw transcripts | Cleaned transcripts |

### 6.2.7 X.7 Risk, Compliance, and Limitations

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_RISK | ASR Risk Controls | Governance/Safety | Privacy/compliance safeguards. | ASR outputs | Risk reports |

### 6.2.8 X.8 Evaluation and Benchmarks

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_BENCH | ASR Benchmarks | Observability | WER/latency eval harness. | Audio sets | Metrics |

### 6.2.9 X.9 Roadmap and Implementation Plan

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_ROADMAP | ASR Roadmap | Helper/Utility | Implementation steps (non-governance roadmap note). | Plan | Progress |

### 6.2.10 X.10 Appendices

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_APPENDIX | ASR Appendix | Helper/Utility | Supporting data/models references. | Appendix | Reference |

### 6.v02.13 Open-Source ASR Technology: A Senior Architect's Comprehensive Guide (2024-2025)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_GUIDE | OS ASR Guide | Helper/Utility | Comprehensive ASR technology guide. | Guide text | Notes |

### 6.2.12 Executive Summary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_SUMMARY | ASR Exec Summary | Helper/Utility | High-level summary of ASR approach. | Summary | Briefings |

### 6.2.13 Open-Source ASR Model Landscape

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_LANDSCAPE | ASR Model Landscape | Helper/Utility | Survey of open ASR models. | Model catalog | Selection notes |

### 6.2.14 Deployment Patterns and Infrastructure

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_DEPLOY | ASR Deployment Pattern | Runtime | Deployment topologies for ASR services. | Infra configs | Deploy manifests |

### 6.2.15 Feature Analysis and Gap Assessment

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_FEATURES | ASR Feature Analysis | Helper/Utility | Capabilities and gaps. | Feature matrix | Gap log |

### 6.2.16 Comprehensive Use Case Catalog

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_USE_CASES | ASR Use Case Catalog | Helper/Utility | Enumerated ASR use cases. | Use case list | Prioritized backlog |

### 6.2.17 Technical Implementation Guide

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_IMPL_GUIDE | ASR Implementation Guide | Helper/Utility | Technical steps for integration. | Guide | Tasks |

### 6.2.18 Known Limitations and Technical Difficulties

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_LIMITS | ASR Limitations | Governance/Safety | Known issues/edge cases. | Incident data | Mitigations |

### 6.2.19 Legal and Ethical Compliance Framework

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_COMPLIANCE | ASR Compliance Framework | Governance/Safety | Legal/ethical safeguards. | Policy | Compliance logs |

### 6.2.20 Integration Patterns and Framework Guidance

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_INTEGRATION | ASR Integration Patterns | Connector | Guidance for integrating ASR with MCP/tools. | Patterns | Integration configs |

### whisper-rs (Rust) is production-ready

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_WHISPER_RS | whisper-rs Runtime | Engine | Rust ASR runtime option. | Audio | Transcripts |

### 6.2.21 Tool Combinations and Multimodal Integration

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_MULTIMODAL | Multimodal ASR Toolchain | Engine | Combines ASR with LLM tools and vision. | Audio + context | Enriched transcripts |

### 6.2.22 Cloud Provider Deployment Matrix

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_CLOUD_MATRIX | ASR Cloud Matrix | Helper/Utility | Deployment options across cloud providers. | Cloud specs | Matrix |

### 6.2.23 Future Outlook and Adoption Recommendations

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_OUTLOOK | ASR Outlook | Helper/Utility | Future roadmap recommendations. | Trends | Guidance |

### 6.2.24 Validation Summary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_VALIDATION | ASR Validation Summary | Observability | Validation outcomes. | Eval results | Summary |

### 6.2.25 Risk Assessment

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_RISK_MATRIX | ASR Risk Matrix | Governance/Safety | Risk matrix for ASR. | Risks | Mitigation plan |

### 6.2.26 ASR AI Job Profile

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_JOB | ASR AI Job Profile | Runtime | Standard job schema for ASR. | Audio refs | Transcripts, timestamps |

---

## 6.3 Mechanical Extension Engines

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENGINES | Mechanical Extension Engines | Engine | Catalog of deterministic external engines. | Tool inputs | Artifacts |

### 6.3.1 Architectural Invariant

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_INVARIANT | Mechanical Engine Invariant | Governance/Safety | Engines run deterministically with governed IO. | Jobs | Deterministic outputs |

### 6.3.2 Domain 1: Engineering & Manufacturing

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN1 | Eng/Manufacturing Toolset | Engine | Mechanical tools for engineering domain. | CAD/eng inputs | Outputs (CAD/analytics) |

### 6.3.3 Domain 2: Creative Studio

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN2 | Creative Studio Toolset | Engine | Creative pipelines (image/video). | Assets | Creative outputs |

### 6.3.4 Domain 3: Culinary & Home

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN3 | Culinary/Home Toolset | Engine | Domain-specific engines. | Recipes/media | Derived outputs |

### 6.3.5 Domain 4: Organization & Knowledge

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN4 | Organization Toolset | Engine | Org/knowledge mechanical tools. | Org data | Processed artifacts |

### 6.3.6 Domain 5: Data & Infrastructure

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN5 | Data/Infra Toolset | Engine | Data pipelines/infrastructure utilities. | Data streams | Processed data |

### 6.3.7 Domain 6: Travel & Spatial Intelligence

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN6 | Travel/Spatial Toolset | Engine | Spatial tools. | Geo inputs | Spatial outputs |

### 6.3.8 Domain 7: Developer Tools & System Context

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN7 | Dev Tools Toolset | Engine | Developer/system-context mechanical tools. | Code repos | Build/test artifacts |

### 6.3.9 Domain 8: OS Primitives & Desktop Integration

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN8 | OS/Desktop Toolset | Engine | OS-level integrations. | OS events | OS actions |

### 6.3.10 Domain 9: Software Engineering & DevOps

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN9 | DevOps Toolset | Engine | Build/deploy/test automation engines. | CI configs | CI outputs |

### 6.3.11 Domain 10: Language & Linguistics

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MECH_ENG_DOMAIN10 | Language Toolset | Engine | NLP/linguistics engines. | Text corpora | NLP artifacts |

---

## 7.1 User Interface Components

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_UI_COMPONENTS | UI Component Suite | UISurface | Core UI components per surface. | UI state | User actions |

### 7.1.0 Cross-View Tool Integration Overview

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CROSS_VIEW | Cross-View Integration | UISurface | Shared tool integration across views. | Entity refs | Cross-view updates |

### 7.1.1 Rich Text Editor (Notion-like)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RICH_TEXT_EDITOR | Rich Text Editor | UISurface | Block-based editor with AI assistance. | Blocks | CRDT updates |

### 7.1.2 Freeform Canvas (Milanote-like)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FREEFORM_CANVAS | Freeform Canvas | UISurface | Spatial canvas for cards/frames. | Canvas nodes | Layout updates |

### 7.1.4 Additional Views: Kanban, Calendar, Timeline

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ADDITIONAL_VIEWS | Kanban/Calendar/Timeline Views | UISurface | Alternative projections over unified schema. | Entities | View states |

---

## 7.2 Multi-Agent Orchestration

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AGENT_ORCH | Agent Orchestrator | Runtime | Coordinates multiple agents with lead/worker roles. | Tasks | Agent outputs |

### 7.2.1 Framework Comparison: AutoGen vs LangGraph vs CrewAI

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AGENT_FRAMEWORKS | Agent Framework Comparison | Helper/Utility | Evaluates orchestration frameworks. | Framework features | Choice notes |

### 7.2.2 The Lead/Worker Pattern

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LEAD_WORKER | Lead/Worker Pattern | Runtime | Lead agent delegates to workers with fallbacks. | Tasks | Delegations |

### 7.2.4 Task Routing and Fallback Logic

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ROUTING_LOGIC | Task Routing Logic | Runtime | Routes tasks across agents/models with fallback. | Task metadata | Routing decisions |

---

## 7.3 Collaboration and Sync

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_COLLAB_SYNC | Collaboration & Sync | Runtime | Collaboration features leveraging CRDTs and external integration. | CRDT streams | Sync updates |

### 7.3.1 Understanding CRDTs

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CRDT_EDUCATION | CRDT Education Module | Helper/Utility | Explains CRDT behavior to users/devs. | CRDT docs | Guides |

### 7.3.3 Google Workspace Integration

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GOOGLE_CONNECTOR | Google Workspace Connector | Connector | Integration for Google docs/sheets/calendar. | Google APIs | Synced entities |

---

## 7.4 Reference Application Analysis

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_REF_APPS | Reference Application Lens | Helper/Utility | Lessons from reference apps. | App studies | Insights |

### 7.4.1 Reference Applications

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_REF_APP_LIST | Reference App List | Helper/Utility | Catalog of benchmark apps. | Studies | Comparison |

### 7.4.2 Lessons Learned

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_REF_LESSONS | Lessons Learned | Helper/Utility | Applied lessons to Handshake UI/runtime. | Analyses | Action items |

---

## 7.5 Development Workflow

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DEV_WORKFLOW | Dev Workflow | Helper/Utility | Guidance for using AI coding assistants, CI/CD. | Dev tasks | Workflow docs |

### 7.5.1 Using AI Coding Assistants Effectively

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AI_ASSIST_GUIDE | AI Assistant Guide | Helper/Utility | How to pair with AI coding assistants. | Dev requests | Guidance |

### 7.5.3 CI/CD and Testing Strategy

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CI_STRATEGY | CI/CD Strategy | Governance/Safety | Testing and CI/CD patterns. | Repo state | CI configs |

---

## 7.6 Development Roadmap

Ignored per scope instructions (no primitives recorded).

---

## 8.1 Risk Assessment

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RISK_MATRIX | Risk Matrix | Governance/Safety | Risks with severity/mitigation tracking. | Risk entries | Mitigation plans |

### 8.1.1 Risk Matrix

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RISK_ENTRY | Risk Entry Primitive | Governance/Safety | Individual risk item with rating. | Risk data | Updates |

### 8.1.2 Complexity Ratings

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_COMPLEXITY_RATING | Complexity Rating Model | Helper/Utility | Complexity scoring for features. | Feature list | Ratings |

---

## 8.2 Technology Stack Summary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_STACK_SUMMARY | Stack Summary | Helper/Utility | Snapshot of chosen technologies. | Stack data | Summary |

### 8.2.1 Core Stack

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CORE_STACK | Core Stack Primitive | Helper/Utility | Rust + React/TS + Python + SQLite/CRDT. | Stack choices | Baseline configs |

### 8.2.2 Frontend Libraries

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FE_LIBS | Frontend Library Set | Helper/Utility | UI libs (React, styling, editor libs). | FE deps | FE config |

### 8.2.3 Backend Libraries

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BE_LIBS | Backend Library Set | Helper/Utility | Backend libs (FastAPI, orchestration). | BE deps | Config |

### 8.2.4 AI Models

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AI_MODEL_SET | AI Model Set | Engine | Models curated for roles. | Model catalog | Model configs |

### 8.2.5 DevOps Tools

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DEVOPS_TOOLS | DevOps Toolchain | Helper/Utility | DevOps stack (CI, containers, monitoring). | Tool list | Pipelines |

---

## v2.0 Complete Technology Stack (Frozen Diagram)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_STACK_DIAGRAM | Stack Diagram v2.0 | Helper/Utility | Frozen diagram of full stack. | Diagram | Reference |

---

## 8.3 Gap Analysis & Open Questions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GAP_ANALYSIS | Gap Analysis | Helper/Utility | Tracks unresolved questions. | Gap list | Resolutions |

### 8.3.1 What the Research DOESN'T Cover

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RESEARCH_GAPS | Research Gap List | Helper/Utility | Missing coverage areas. | Gap notes | Follow-ups |

### 8.3.2 Open Technical Questions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OPEN_TECH_QS | Open Technical Questions | Helper/Utility | Outstanding technical questions. | Questions | Answers |

### 8.3.3 Unresolved Issues

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_UNRESOLVED | Unresolved Issue Tracker | Helper/Utility | Tracks unresolved issues. | Issues | Status updates |

### 8.3.4 Not Covered (Future Research)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FUTURE_RESEARCH | Future Research Topics | Helper/Utility | Items slated for future research. | Topic list | Research backlog |

### 8.3.5 Immediate Next Steps

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_NEXT_STEPS | Immediate Next Steps | Helper/Utility | Action items from gap analysis. | Actions | Progress updates |

---

## 8.4 Consolidated Glossary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GLOSSARY | Glossary | Helper/Utility | Consolidated definitions. | Terms | Glossary entries |

---

## 8.5 Sources Referenced

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SOURCES | Source Reference List | Helper/Utility | Cited documents and research. | Citations | Reference list |

### 8.5.1 Source Documents

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SOURCE_DOCS | Source Docs | Helper/Utility | Canonical source documents. | Source files | Index |

### 8.5.2 Document Statistics

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOC_STATS | Document Statistics | Helper/Utility | Stats about source docs. | Docs | Stats |

### 8.5.3 Quick Reference Guide

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_QUICK_REF | Quick Reference Guide | Helper/Utility | Fast lookup guide. | Specs | Guide |

---

## 8.6 Appendices

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_APPENDICES | Appendix Collection | Helper/Utility | Expanded appendices (plugins, Docling, ASR). | Appendix content | Reference |

### 8.6.1 Foundation Concepts

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FOUNDATION | Foundation Concepts | Helper/Utility | Core conceptual appendices. | Concepts | Notes |

### 8.6.2 Architecture Decisions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ARCH_DECISIONS | Architecture Decisions | Helper/Utility | Recorded architecture decisions. | Decisions | ADRs |

### aa Risk: AFFiNE's Tauri-to-Electron Switch

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AFFINE_RISK | AFFiNE TauriaElectron Risk | Governance/Safety | Risk noted about platform switch. | Risk note | Mitigation |

### 8.6.3 Plugin and Extension System (Expanded)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_EXPANDED | Expanded Plugin System | Connector | Deep dive on plugin/extension. | Plugin details | APIs |

### 8.6.4 Docling Feature Comparison Tables

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_FEATURE_TABLE | Docling Feature Table | Helper/Utility | Comparison tables for Docling features. | Docling metrics | Tables |

### 8.6.5 ASR Risk Matrix

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_RISK_APPENDIX | ASR Risk Appendix | Governance/Safety | ASR risk matrix details. | Risk data | Matrix |

### 8.6.6 Works Cited (Docling)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_CITED | Docling Works Cited | Helper/Utility | References for Docling research. | Citations | List |

### 8.6.7 Works Cited (ASR)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_CITED | ASR Works Cited | Helper/Utility | References for ASR research. | Citations | List |

---

## 8.7 Version History & Subsection Versioning

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VERSION_HISTORY | Version History | Helper/Utility | Master spec version tracking. | Versions | Change log |

### 8.7.1 Master Specification Versions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MASTER_VERSIONS | Master Version Table | Helper/Utility | List of master spec versions. | Version data | Table |

### 8.7.2 Subsection Independent Versions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SUBSECTION_VERSIONS | Subsection Version Map | Helper/Utility | Versioning per subsection. | Subsection data | Map |

### 8.7.3 Change Log (Recent)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CHANGE_LOG | Change Log | Helper/Utility | Recent changes log. | Updates | Log |

---

## 8.1 Risk Assessment

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RISK_MATRIX | Risk Matrix | Governance/Safety | Risks with severity/mitigation tracking. | Risk entries | Mitigation plans |

### 8.1.1 Risk Matrix

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RISK_ENTRY | Risk Entry Primitive | Governance/Safety | Individual risk item with rating. | Risk data | Updates |

### 8.1.2 Complexity Ratings

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_COMPLEXITY_RATING | Complexity Rating Model | Helper/Utility | Complexity scoring for features. | Feature list | Ratings |

---

## 8.2 Technology Stack Summary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_STACK_SUMMARY | Stack Summary | Helper/Utility | Snapshot of chosen technologies. | Stack data | Summary |

### 8.2.1 Core Stack

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CORE_STACK | Core Stack Primitive | Helper/Utility | Rust + React/TS + Python + SQLite/CRDT. | Stack choices | Baseline configs |

### 8.2.2 Frontend Libraries

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FE_LIBS | Frontend Library Set | Helper/Utility | UI libs (React, styling, editor libs). | FE deps | FE config |

### 8.2.3 Backend Libraries

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_BE_LIBS | Backend Library Set | Helper/Utility | Backend libs (FastAPI, orchestration). | BE deps | Config |

### 8.2.4 AI Models

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AI_MODEL_SET | AI Model Set | Engine | Models curated for roles. | Model catalog | Model configs |

### 8.2.5 DevOps Tools

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DEVOPS_TOOLS | DevOps Toolchain | Helper/Utility | DevOps stack (CI, containers, monitoring). | Tool list | Pipelines |

---

## v2.0 Complete Technology Stack (Frozen Diagram)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_STACK_DIAGRAM | Stack Diagram v2.0 | Helper/Utility | Frozen diagram of full stack. | Diagram | Reference |

---

## 8.3 Gap Analysis & Open Questions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GAP_ANALYSIS | Gap Analysis | Helper/Utility | Tracks unresolved questions. | Gap list | Resolutions |

### 8.3.1 What the Research DOESN'T Cover

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RESEARCH_GAPS | Research Gap List | Helper/Utility | Missing coverage areas. | Gap notes | Follow-ups |

### 8.3.2 Open Technical Questions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OPEN_TECH_QS | Open Technical Questions | Helper/Utility | Outstanding technical questions. | Questions | Answers |

### 8.3.3 Unresolved Issues

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_UNRESOLVED | Unresolved Issue Tracker | Helper/Utility | Tracks unresolved issues. | Issues | Status updates |

### 8.3.4 Not Covered (Future Research)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FUTURE_RESEARCH | Future Research Topics | Helper/Utility | Items slated for future research. | Topic list | Research backlog |

### 8.3.5 Immediate Next Steps

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_NEXT_STEPS | Immediate Next Steps | Helper/Utility | Action items from gap analysis. | Actions | Progress updates |

---

## 8.4 Consolidated Glossary

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GLOSSARY | Glossary | Helper/Utility | Consolidated definitions. | Terms | Glossary entries |

---

## 8.5 Sources Referenced

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SOURCES | Source Reference List | Helper/Utility | Cited documents and research. | Citations | Reference list |

### 8.5.1 Source Documents

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SOURCE_DOCS | Source Docs | Helper/Utility | Canonical source documents. | Source files | Index |

### 8.5.2 Document Statistics

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOC_STATS | Document Statistics | Helper/Utility | Stats about source docs. | Docs | Stats |

### 8.5.3 Quick Reference Guide

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_QUICK_REF | Quick Reference Guide | Helper/Utility | Fast lookup guide. | Specs | Guide |

---

## 8.6 Appendices

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_APPENDICES | Appendix Collection | Helper/Utility | Expanded appendices (plugins, Docling, ASR). | Appendix content | Reference |

### 8.6.1 Foundation Concepts

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FOUNDATION | Foundation Concepts | Helper/Utility | Core conceptual appendices. | Concepts | Notes |

### 8.6.2 Architecture Decisions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ARCH_DECISIONS | Architecture Decisions | Helper/Utility | Recorded architecture decisions. | Decisions | ADRs |

### aa Risk: AFFiNE's Tauri-to-Electron Switch

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_AFFINE_RISK | AFFiNE TauriaElectron Risk | Governance/Safety | Risk noted about platform switch. | Risk note | Mitigation |

### 8.6.3 Plugin and Extension System (Expanded)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_EXPANDED | Expanded Plugin System | Connector | Deep dive on plugin/extension. | Plugin details | APIs |

### 8.6.4 Docling Feature Comparison Tables

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_FEATURE_TABLE | Docling Feature Table | Helper/Utility | Comparison tables for Docling features. | Docling metrics | Tables |

### 8.6.5 ASR Risk Matrix

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_RISK_APPENDIX | ASR Risk Appendix | Governance/Safety | ASR risk matrix details. | Risk data | Matrix |

### 8.6.6 Works Cited (Docling)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DOCLING_CITED | Docling Works Cited | Helper/Utility | References for Docling research. | Citations | List |

### 8.6.7 Works Cited (ASR)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ASR_CITED | ASR Works Cited | Helper/Utility | References for ASR research. | Citations | List |

---

## 8.7 Version History & Subsection Versioning

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_VERSION_HISTORY | Version History | Helper/Utility | Master spec version tracking. | Versions | Change log |

### 8.7.1 Master Specification Versions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MASTER_VERSIONS | Master Version Table | Helper/Utility | List of master spec versions. | Version data | Table |

### 8.7.2 Subsection Independent Versions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SUBSECTION_VERSIONS | Subsection Version Map | Helper/Utility | Versioning per subsection. | Subsection data | Map |

### 8.7.3 Change Log (Recent)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CHANGE_LOG | Change Log | Helper/Utility | Recent changes log. | Updates | Log |

---

## 9.1 Canonical Specification (verbatim import)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SKILL_DISTILL_SPEC | Skill Distillation Canonical Spec | Governance/Safety | Imported specification for skill distillation pipeline. | Spec text | Distill configs |

### Handshake Continuous Local Skill Distillation - Technical Specification

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SKILL_DISTILL_PIPE | Skill Distillation Pipeline | ContentPipeline | Local skill distillation process with gating and scoring. | SkillBank logs | Distill datasets |

---

## 10 Product Surfaces

General product surface primitives captured in subsections below.

### 10.1 Terminal Experience

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERMINAL_SURFACE | Terminal Surface | UISurface | Integrated terminal with policy/sandbox modes and `run_command` API. | Commands | Output streams |

#### 10.1.1 Security, Capabilities, and API (LAW)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERM_SECURITY | Terminal Security Model | Governance/Safety | Policy vs hard isolation, consent caching, capability scopes. | Capability state | Enforcement logs |

#### 10.1.1.2 Consent / capability UX and caching

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERM_CONSENT | Terminal Consent Cache | Governance/Safety | Approval caching and escalation UX. | Consent records | Approvals |

#### 10.1.1.3 `run_command` API contract

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUN_COMMAND | Run Command API | Connector | API for executing commands with capability checks. | Command requests | Command results |

#### 10.1.1.4 Sandboxing/Security patterns

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERM_SANDBOX | Terminal Sandbox Mode | Governance/Safety | Container/VM sandbox option for terminal sessions. | Session config | Sandbox state |

### 10.1.2 Logging, Matchers, UX, Platform (LAW)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERM_LOGGING | Terminal Logging | Observability | Logs command executions to Flight Recorder. | Command events | Logs |

### 10.1.3 v1 Scope & Invariants (LAW)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERM_SCOPE | Terminal Scope v1 | Governance/Safety | Scope/invariants for initial release. | Scope doc | Constraints |

### 10.1.4 Design Notes & Feature Map (Non-normative)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERM_FEATURE_MAP | Terminal Feature Map | Helper/Utility | Non-normative design map. | Notes | Ideas |

### 10.1.5 Review & Hardening Map (Non-normative)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TERM_HARDENING | Terminal Hardening Plan | Governance/Safety | Hardening checklist. | Risks | Mitigations |

### 10.2 Monaco Editor Experience

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MONACO_SURFACE | Monaco Editor Surface | UISurface | Code editor surface with bundling/LSP bridges. | Code files | Edits |

### 10.2.1 IDs, AST, Bundling (LAW)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MONACO_IDS | Monaco ID/AST Layer | Runtime | Block/AST IDs and bundling rules. | Source files | AST metadata |

### 10.2.2 Diagnostics & Flight Recorder (LAW)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MONACO_DIAG | Monaco Diagnostics | Observability | Diagnostics streamed to Flight Recorder. | LSP events | Logs |

### 10.2.3 v1 Scope & Invariants (LAW)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MONACO_SCOPE | Monaco Scope v1 | Governance/Safety | Invariants for Monaco integration. | Scope doc | Constraints |

### 10.2.4 Design Notes & Feature Map (Non-normative)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MONACO_FEATURE_MAP | Monaco Feature Map | Helper/Utility | Design/feature map. | Notes | Ideas |

### 10.2.5 Review & Hardening Map (Non-normative)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MONACO_HARDENING | Monaco Hardening Plan | Governance/Safety | Hardening steps. | Risks | Mitigations |

---

### 10.3 Mail Client

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_SURFACE | Mail Surface | UISurface | Native mail experience integrated with workspace graph. | Mail data | DisplayContent, responses |

### 10.3.1 Motivation and Goals

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_GOALS | Mail Goals | Helper/Utility | Goals for integrated mail. | Goals | Success criteria |

### 10.3.2 Architectural Context

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_CONTEXT | Mail Architecture Context | Connector | Mail within Handshake architecture. | Mail topology | Integration plan |

### 10.3.3 Mail as First-Class Content

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_FIRST_CLASS | First-Class Mail Content | ContentPipeline | Treat mail as RawContent with DerivedContent. | Email messages | Descriptors/tags |

### 10.3.4 Mechanical Ingestion Path for Mail

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_INGEST | Mail Ingestion Path | ContentPipeline | Mechanical ingestion (JMAP) into workspace. | Mailboxes | RawContent{Mail} |

### 10.3.5 Descriptor Domains and TXT-001 for Mail

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_TXT001 | Mail TXT-001 Descriptor | ContentPipeline | Applies TXT-001 descriptors to mail bodies. | Mail text | DescriptorRows |

### 10.3.6 Mechanical Engines Exploited for Mail

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_MECH_ENGINES | Mail Mechanical Engines | Engine | Engines used on mail (classifiers, summarizers). | Mail content | DerivedContent |

### 10.3.7 Classification-Based Governance (Corporate Secrets, PII, etc.)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_CLASSIFIER | Mail Governance Classifier | Governance/Safety | Classification for secrets/PII gating. | Mail data | Classification labels |

### 10.3.8 Mail Enriches the Rest of Handshake

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_ENRICHMENT | Mail Enrichment Hooks | Connector | Links mail to tasks/docs/canvases. | Mail metadata | Cross-refs |

### 10.3.9 AI Jobs, Workflow DSL, and Mail

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_JOB_PROFILE | Mail AI Job Profile | Runtime | Job profile for mail actions (triage, reply). | Mail entity refs | Replies, labels |

### 10.3.10 UX: One Cohesive GUI, No Extra Mail App

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_UI | Mail UI Integration | UISurface | Mail integrated into core UI. | Mail threads | UI interactions |

### 10.3.11 Security, Edge Cases, and Safety

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_SECURITY | Mail Security Layer | Governance/Safety | Safety for mail ingestion/processing. | Mail data | Safety logs |

### 10.3.12 Comparison with Conventional AI Mail Stacks

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_COMPARE | Mail Stack Comparison | Helper/Utility | Comparison with other stacks. | Comparison data | Notes |

### 10.3.13 Incremental Implementation Plan (High-Level)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_PLAN | Mail Implementation Plan | Helper/Utility | Phased plan for mail features. | Plan | Progress |

### 10.3.14 Conclusion

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MAIL_CONCLUSION | Mail Conclusion | Helper/Utility | Summary conclusion for mail section. | Summary | Notes |

### 10.4 Calendar

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CALENDAR_SURFACE | Calendar Surface | UISurface | Calendar integrated with capability-consent and MCP adapter. | Calendar events | DisplayContent |

### 10.4.0 Scope and positioning

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CAL_SCOPE | Calendar Scope | Helper/Utility | Scope/positioning for calendar. | Scope text | Plan |

### 10.4.1 Handshake Calendar Research v0.4 (verbatim import; headings adjusted)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CAL_RESEARCH | Calendar Research | Helper/Utility | Imported research. | Research doc | Notes |

### 10.4.2 Calendar a ACE Integration (v0.1)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CAL_ACE | Calendar-ACE Integration | Connector | MCP adapter for calendar with consent. | ACE events | Synced events |

### 10.5 Operator Consoles: Debug & Diagnostics

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OPERATOR_CONSOLE | Operator Console | UISurface | Debug/diagnostics console with triage loop. | Flight Recorder data | Debug bundles |

### 10.5.0 Purpose

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONSOLE_PURPOSE | Console Purpose | Helper/Utility | Purpose of operator console. | Spec | Notes |

### 10.5.1 Scope

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONSOLE_SCOPE | Console Scope | Helper/Utility | Scope definitions. | Scope | Constraints |

### 10.5.2 Normative language

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONSOLE_NORMATIVE | Console Normative Rules | Governance/Safety | Normative statements for console behavior. | Rules | Enforcement |

### 10.5.3 Non-negotiable principles

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONSOLE_PRINCIPLES | Console Principles | Governance/Safety | Non-negotiable principles. | Principles | Policy |

### 10.5.4 The operator triage loop (fixed)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_TRIAGE_LOOP | Operator Triage Loop | Runtime | Fixed triage steps. | Incidents | Triage actions |

### 10.5.5 Console surfaces

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONSOLE_SURFACES | Console Surfaces | UISurface | Surfaces for logs, traces, metrics. | Observability data | UI views |

### 10.5.6 Debug Bundle (export artifact)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DEBUG_BUNDLE | Debug Bundle Export | Observability | Export artifact for support. | Logs/config | Bundle file |

### 10.5.7 Acceptance criteria and validators

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CONSOLE_VALIDATORS | Console Validators | Governance/Safety | Validators for console outputs. | Console data | Validation results |

### 10.9 Future Surfaces

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FUTURE_SURFACES | Future Surface Placeholders | UISurface | Reserved future UI surfaces. | Ideas | Backlog |

---

## 11.1 Capabilities & Consent Model

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CAPABILITY_MODEL | Capability & Consent Model | Governance/Safety | Consent prompts, scope, caching across tools. | Consent state | Tokens/approvals |

---

## 11.2 Sandbox Policy vs Hard Isolation

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SANDBOX_POLICY | Sandbox Policy | Governance/Safety | Policy-scoped vs hard isolation guidance. | Sandbox config | Enforcement |

---

## 11.3 Auth/Session/MCP Primitives

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MCP_GATE | MCP Gate Interceptor | Governance/Safety | Rust gate intercepting MCP tool calls with capability checks. | MCP requests | Authorized tool calls |

### 11.3.1 Architectural Strategy and The Local-First Imperative

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOCAL_FIRST_IMP | Local-First Strategy | Governance/Safety | Strategy for MCP/auth under local-first. | Strategy docs | Policy hooks |

### 11.3.2 Implementation Target 1: The Rust 'Gate' Interceptor (Middleware Design)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_GATE_MIDDLEWARE | MCP Gate Middleware | Governance/Safety | Middleware design for gate. | MCP envelopes | Filtered calls |

### Target 1: Rust "Gate" Interceptor for `tools/call` (G-CONSENT)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_G_CONSENT | G-CONSENT Tool Gate | Governance/Safety | Gate for MCP `tools/call`. | Tool calls | Consent-checked calls |

### 11.3.3 Implementation Target 2: Reference-Based Binary Protocol (Sequence Diagram)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_REF_BINARY_PROTOCOL | Reference Binary Protocol | Connector | Binary protocol for MCP references. | Reference requests | Binary responses |

### Target 2: Reference-Based Binary Protocol (Mermaid Sequence)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_REF_PROTOCOL_SEQ | Reference Protocol Sequence | Connector | Sequence for reference-based transfers. | Transfer steps | Data streams |

### 11.3.4 Implementation Target 3: Durable Progress Mapping (SQLite Integration)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PROGRESS_SQLITE | Durable Progress Mapping | Runtime | Maps MCP notifications to SQLite jobs table. | Notifications | Job rows |

### Target 3: Durable Progress Mapping (notifications a SQLite jobs)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PROGRESS_MAPPING | Progress Mapping Flow | Runtime | Flow from notifications to job persistence. | Notifications | Job status |

### 11.3.5 Implementation Target 4: Sampling for Skill Distillation

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SAMPLING_DISTILL | Skill Distillation Sampling | ContentPipeline | Sampling messages for distillation. | Skill logs | Sample datasets |

### Target 4: Sampling for Skill Distillation (`sampling/createMessage`)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SAMPLING_API | Sampling API | Connector | API for sampling messages for distill. | Sampling requests | Sample records |

### 11.3.6 Implementation Target 5: The Logging Sink (DuckDB Integration)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOGGING_SINK | Logging Sink | Observability | MCP logging sink to DuckDB Flight Recorder. | MCP logs | DuckDB tables |

### Target 5: Logging Sink Design (MCP `logging/message` a DuckDB Flight Recorder)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_LOGGING_FLOW | Logging Flow | Observability | Flow from MCP logging to recorder. | Logging messages | Logs |

### 11.3.7 Red Team Security Audit

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RED_TEAM | Red Team Audit | Governance/Safety | Security audit tasks. | Audit findings | Mitigations |

### **7.4 MCP Hardening Checks (Symlinks + Sampling)**

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MCP_HARDENING | MCP Hardening Checks | Governance/Safety | Symlink and sampling hardening steps. | File refs | Enforcement |

### 11.3.7.5 Red Team Audit (Senior Security Researcher)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RED_TEAM_SR | Senior Red Team Audit | Governance/Safety | Senior audit tasks. | Findings | Reports |

### 1. Symlink attack on reference-based `file://` URIs

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_SYMLINK_DEFENSE | Symlink Defense | Governance/Safety | Defense against symlink attacks in references. | File refs | Blocked access |

### 2. Prompt injection via `sampling/createMessage`

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PROMPT_INJECTION_DEFENSE | Prompt Injection Defense | Governance/Safety | Defense for sampling/createMessage. | Sampling payloads | Sanitized inputs |

### 3. Proposed Rust-level hardening checks

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_RUST_HARDENING | Rust Hardening Checks | Governance/Safety | Rust-level checks to protect MCP pipeline. | Runtime state | Hardened paths |

---

## 11.4 Diagnostics Schema (Problems/Events)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_DIAG_SCHEMA | Diagnostics Schema | Observability | Schema for problems/events diagnostics. | Events | Diagnostics |

### 11.4.1 Validators (minimum set)

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MIN_VALIDATORS | Minimum Validator Set | Governance/Safety | Required validators for diagnostics. | Events | Validation outputs |

---

## 11.5 Flight Recorder Event Shapes & Retention

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FR_EVENT_SHAPES | Flight Recorder Event Shapes | Observability | Event schemas and retention rules. | Event data | Stored events |

---

## 11.6 Plugin/Matcher Precedence Rules

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_PLUGIN_PRECEDENCE | Plugin/Matcher Precedence | Governance/Safety | Rules for plugin matcher ordering. | Plugin metadata | Precedence tables |

---

## 11.7 OSS Component Choices & Versions

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OSS_CHOICES | OSS Component Choices | Helper/Utility | Selected OSS components and versions. | Component list | Version map |

### 11.7.1 Terminal Engine / PTY / Sandbox

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OSS_TERMINAL | OSS Terminal Stack | Engine | PTY/sandbox OSS choices. | OSS specs | Config |

### 11.7.2 Monaco Bundling / LSP Bridges

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OSS_MONACO | OSS Monaco Stack | Engine | Monaco bundling and LSP bridges. | OSS docs | Config |

### 11.7.3 Mail / Calendar Engines

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_OSS_MAIL_CAL | OSS Mail/Calendar Engines | Engine | OSS components for mail/calendar. | OSS list | Config |

---

## 11.9 Future Shared Primitives

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_FUTURE_SHARED | Future Shared Primitives | Helper/Utility | Placeholders for future shared primitives. | Ideas | Backlog |

### 11.9.1 ActivitySpan and SessionSpan

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_ACTIVITY_SPAN | ActivitySpan | Observability | Span model for activity/session tracing. | Trace data | Span records |

### 11.9.2 Calendar Range as a Query Surface

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CAL_QUERY_SURFACE | Calendar Query Surface | Index/Search | Calendar range as query surface. | Calendar events | Query results |

### 11.9.3 CalendarEvent and ActivitySpan Join Semantics

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_CAL_ACTIVITY_JOIN | Calendar-Activity Join | Index/Search | Join semantics between calendar events and activity spans. | Events, spans | Joined views |

### 11.9.4 Minimum Slice for Calendar and Flight Recorder

| ID | Name | Category | Description | Reads | Writes |
|----|------|----------|-------------|-------|--------|
| PRIM_MIN_CAL_FR | Minimum Calendar+FR Slice | Runtime | Minimum viable integration slice. | Calendar + FR data | Baseline feature set |

---


## Closing Notes

- Surfaces: As specified per primitive; most background-only.  
- Triggers: User commands, imports, workflow schedules, MCP calls, and validator pipelines.  
- Synergy: Pipelines compose via AI Job Model, MCP Gate, Capability System, and Shadow Workspace indexes.  
- Governance overlay: All primitives run under capability, gate, and logging controls captured above.



