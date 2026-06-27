---
schema: handshake.indexed_spec.module@1
spec_version: "v02.196"
bundle_id: "master-spec-v02.196"
module_id: "08"
section_id: "8"
title: "8. Reference"
source_baseline_version: "v02.182"
source_baseline_path: ".GOV/spec/Handshake_Master_Spec_v02.182.md"
source_body_original_sha256: "6ca13c3d4fa603d3fcd8dabf08ba3e060e495869b3b660404fd1e2b71433efb2"
body_sha256: "1410df778592d3fed8202e61a37657b37b196d97d007eb48ff00a28822c080ed"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---
# 8. Reference

<a id="8-reference"></a>

## 8.1 Risk Assessment

**Why**  
Understanding risks upfront enables proactive mitigation. This section identifies key risks and their mitigation strategies.

**What**  
Risk matrix covering likelihood and impact, complexity ratings for each component, and mitigation strategies.

Note: Refresh risk ratings/owners as of 2026-02-16; close resolved items and add emerging risks from ACE runtime/calendar integrations and plugin capability model.

**Jargon**  
- **Scope Creep**: Uncontrolled expansion of project requirements.
- **Graceful Degradation**: System continues working (with reduced capability) when components fail.

---

### 8.1.1 Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Tauri webview issues** | Medium | High | Minimal Tauri role; test early on all platforms |
| **Local model performance** | Medium | Medium | Cloud fallback; smaller model options |
| **Complexity overwhelm** | High | High | Strict MVP scope; phases; hire help |
| **CRDT learning curve** | Medium | Medium | Use Yjs (proven); start with single-user |
| **Plugin security** | Low | High | Delay plugins; learn from existing models |
| **Scope creep** | High | High | Written MVP definition; say no to extras |

---

### 8.1.2 Complexity Ratings

| Component | Complexity | Notes |
|-----------|------------|-------|
| Tauri setup | âš ï¸ Medium | Some Rust knowledge needed |
| Block editor | âš ï¸ Medium | Tiptap helps a lot |
| AI orchestration | âš ï¸âš ï¸ High | Multi-model coordination is complex |
| Canvas | âš ï¸ Medium | Excalidraw does heavy lifting |
| Spreadsheets | âš ï¸ Medium | HyperFormula helps |
| CRDT sync | âš ï¸âš ï¸ High | Conceptually challenging |
| ComfyUI integration | âš ï¸ Medium | API-based, manageable |
| Plugin system | âš ï¸âš ï¸ High | Defer to post-MVP |

---

**Key Takeaways**  
- Highest risks: complexity overwhelm and scope creepâ€”mitigate with strict MVP scope.
- Tauri webview issues are medium risk but high impactâ€”test early on all platforms.
- AI orchestration and CRDT sync are the most complex components.
- Delay plugin system to post-MVP to reduce initial complexity.

---

<a id="82-technology-stack-summary"></a>
## 8.2 Technology Stack Summary

**Why**  
A consolidated reference of all technologies enables quick lookup and ensures consistency across the project.

**What**  
Complete list of technologies organized by layer: Core Stack, Frontend Libraries, Backend Libraries, AI Models, DevOps Tools.

Note: Update library/model/tool versions and support status as of 2026-02-16; mark deprecated/locked versions and owners responsible for upgrades.

**Jargon**  
See individual technology entries in the Consolidated Glossary (Section 8.4).

---

### 8.2.1 Core Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| **Desktop Shell** | Tauri | Cross-platform wrapper |
| **Frontend** | React + TypeScript | User interface |
| **Backend** | Rust coordinator plus product-managed worker subprocesses where needed | API server, orchestration |
| **AI Runtime** | Handshake ModelRuntime with native open-source engines and explicit compatibility adapters | Model execution |
| **Storage** | File system + PostgreSQL/EventLedger + ArtifactStore | Data persistence |
| **Sync** | Yjs (CRDT) | Collaboration |

---

### 8.2.2 Frontend Libraries

| Library | Purpose |
|---------|---------|
| Tiptap / BlockNote | Block-based editor |
| Excalidraw | Canvas/whiteboard |
| HyperFormula | Spreadsheet formulas |
| Wolf-Table | Spreadsheet UI |
| React Table / AG Grid | Data grid views |
| React Beautiful DnD | Drag and drop |

---

### 8.2.3 Backend Libraries

| Library | Purpose |
|---------|---------|
| FastAPI | HTTP API server |
| AutoGen or LangGraph | Agent orchestration |
| Handshake ModelRuntime API | Local LLM access |
| Native image graph runtime API | Image generation |
| Pydantic | Data validation |
| SQLx/PostgreSQL | PostgreSQL/EventLedger access |

---

### 8.2.4 AI Models

| Model | Purpose | Size |
|-------|---------|------|
| Llama 3 13B | General text | ~14GB |
| Code Llama 13B | Code generation | ~14GB |
| Mistral 7B | Fast responses | ~8GB |
| SDXL 1.0 | Image generation | ~10GB |

---

### 8.2.5 DevOps Tools

| Tool | Purpose |
|------|---------|
| GitHub Actions | CI/CD |
| Ruff, Black, isort | Python linting/formatting |
| ESLint, Prettier | TypeScript linting/formatting |
| pytest | Python testing |
| vitest | TypeScript testing |
| n8n (optional) | Workflow automation |

---

**Key Takeaways**  
- Core stack: Tauri + React + Rust coordinator + Handshake ModelRuntime + PostgreSQL/EventLedger + Yjs/CRDT write-box.
- Frontend: Tiptap for editing, Excalidraw for canvas, HyperFormula for spreadsheets.
- Backend: FastAPI + AutoGen/LangGraph for orchestration.
- AI: Llama 3, Code Llama, Mistral, SDXL for different tasks.

---
## v2.0 Complete Technology Stack (Frozen Diagram)

```
+----------------------------------------------------------+
|                  COMPLETE TECHNOLOGY STACK               |
+----------------------------------------------------------+
| DESKTOP FRAMEWORK                                        |
|   - Primary: Tauri (Rust) + React/Vue                    |
|   - Alternative: Electron (if JS ecosystem needed)       |
|                                                          |
| LLM INFRASTRUCTURE                                       |
|   - Runtime: Handshake ModelRuntime + native engines      |
|   - Models: Llama 3, Code Llama, Mistral                 |
|   - Images: ComfyUI + SDXL                               |
|                                                          |
| DATA LAYER                                               |
|   - CRDT: Yjs (or Loro for Rust)                         |
|   - Database: PostgreSQL/EventLedger                     |
|   - Sync: Yjs WebSocket provider (later)                 |
|                                                          |
| PLUGIN SYSTEM                                            |
|   - Sandbox: WASM (Wasmtime)                             |
|   - Language: AssemblyScript/Rust -> WASM                |
|   - Permissions: Manifest-based capability model         |
|                                                          |
| OBSERVABILITY                                            |
|   - Telemetry: OpenTelemetry                             |
|   - Metrics: Prometheus                                  |
|   - Visualization: Grafana                               |
|   - Traces: Jaeger or Grafana Tempo                      |
|                                                          |
| LANGUAGES                                                |
|   - Backend: Python (orchestrator) + Rust (Tauri)        |
|   - Frontend: TypeScript + React/Vue                     |
|   - Plugins: AssemblyScript -> WASM                      |
+----------------------------------------------------------+
```

## 8.3 Gap Analysis & Open Questions

**Why**  
Acknowledging what the research doesn't cover prevents false confidence and highlights areas needing further investigation.

**What**  
Documents research gaps (UI/UX, authentication, business model, fine-tuning, Windows-specific), open technical questions, and unresolved issues requiring further work.

Note: Refresh statuses (open/closed/owner) as of 2026-02-16; move resolved items to changelog/ADRs and prune stale questions.

**Jargon**  
- **RAG (Retrieval Augmented Generation)**: Technique for handling long documents by retrieving relevant chunks.
- **Fine-tuning**: Training a pre-trained model on specific data to improve performance on particular tasks.

---

### 8.3.1 What the Research DOESN'T Cover

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                      RESEARCH GAPS                           â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  USER INTERFACE                                              â”‚
â”‚  â€¢ No detailed UI/UX designs                                â”‚
â”‚  â€¢ No accessibility considerations                          â”‚
â”‚  â€¢ No mobile/responsive strategy                            â”‚
â”‚  Action: Need separate UI design research                   â”‚
â”‚                                                              â”‚
â”‚  AUTHENTICATION & MULTI-USER                                 â”‚
â”‚  â€¢ No user account system design                            â”‚
â”‚  â€¢ No team/sharing model                                    â”‚
â”‚  â€¢ No encryption for sensitive data                         â”‚
â”‚  Action: Research if/when adding cloud sync                 â”‚
â”‚                                                              â”‚
â”‚  BUSINESS MODEL                                              â”‚
â”‚  â€¢ No pricing strategy                                      â”‚
â”‚  â€¢ No marketplace economics for plugins                     â”‚
â”‚  Action: Business planning separate from technical          â”‚
â”‚                                                              â”‚
â”‚  SPECIFIC MODEL FINE-TUNING                                  â”‚
â”‚  â€¢ Research covers pre-trained models only                  â”‚
â”‚  â€¢ No guidance on fine-tuning for specific use cases        â”‚
â”‚  Action: May need if default models insufficient            â”‚
â”‚                                                              â”‚
â”‚  WINDOWS-SPECIFIC ISSUES                                     â”‚
â”‚  â€¢ Limited coverage of Windows sandboxing options           â”‚
â”‚  â€¢ No Windows installer/distribution guidance               â”‚
â”‚  Action: Platform-specific research needed                  â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 8.3.2 Open Technical Questions

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    OPEN QUESTIONS                            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  1. Tauri vs Electron final decision?                       â”‚
â”‚     â€¢ Tauri: Smaller, faster, Rust backend                  â”‚
â”‚     â€¢ Electron: More mature, larger ecosystem               â”‚
â”‚     â†’ Recommendation: Start with Tauri, reconsider if       â”‚
â”‚       ecosystem limitations become blocking                 â”‚
â”‚                                                              â”‚
â”‚  2. How to handle very long documents?                      â”‚
â”‚     â€¢ Context windows are limited (4K-8K tokens)            â”‚
â”‚     â€¢ Options: Chunking, summarization, RAG                 â”‚
â”‚     â†’ Need: RAG (Retrieval Augmented Generation) research   â”‚
â”‚                                                              â”‚
â”‚  3. Offline-first sync strategy?                            â”‚
â”‚     â€¢ File sync (OneDrive/Dropbox) simple but limited       â”‚
â”‚     â€¢ Custom sync server more powerful but complex          â”‚
â”‚     â†’ Recommendation: Start with file sync, add server      â”‚
â”‚       when multi-user collaboration is priority             â”‚
â”‚                                                              â”‚
â”‚  4. Plugin language choice?                                 â”‚
â”‚     â€¢ WASM requires compilation (barrier to entry)          â”‚
â”‚     â€¢ JavaScript simpler but harder to sandbox              â”‚
â”‚     â†’ Recommendation: Support bothâ€”sandboxed JS for         â”‚
â”‚       simple plugins, WASM for advanced/untrusted           â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

### 8.3.3 Unresolved Issues

| Question | Why It Matters | Suggested Action |
|----------|---------------|------------------|
| **Exact Tauri version?** | v1 vs v2 have API differences | Check latest stable, test early |
| **Python bundling strategy?** | How to package Python with Tauri | Research PyInstaller + Tauri sidecar |
| **Model download UX?** | How do users get 10GB+ models? | Design in-app download + progress UI |
| **License audit** | Some libraries have complex licenses | Full audit before production |
| **Performance benchmarks** | Real numbers on target hardware | Build prototype, measure |

---

### 8.3.4 Not Covered (Future Research)

The documents don't cover:
- Mobile versions (iOS/Android)
- Web version (browser-only)
- Enterprise features (SSO, audit logs)
- Monetization strategy
- Analytics/telemetry approach
- Accessibility (a11y) requirements

---

### 8.3.5 Immediate Next Steps

1. **Set up monorepo** with Tauri + React + Python structure
2. **Validate Tauri** on Windows, Mac, Linux
3. **Prototype IPC** between React and Python
4. **Test Handshake ModelRuntime** integration
5. **Build health check** command

---

**Key Takeaways**  
- Research gaps exist in UI/UX, authentication, business model, fine-tuning, and Windows-specific issues.
- Key open questions: long document handling (needs RAG research), plugin language choice.
- Immediate next steps: monorepo setup, Tauri validation, IPC prototyping, Handshake ModelRuntime testing.

---

<a id="84-consolidated-glossary"></a>
## 8.4 Consolidated Glossary

**Why**  
A unified glossary ensures consistent terminology across the project and serves as quick reference.

**What**  
Alphabetical list of all technical terms defined throughout this specification.

---

| Term | Definition |
|------|------------|
| **access_mode** | Job field specifying consent level: `analysis_only` (read), `preview_only` (propose), `apply_scoped` (apply); see (AI Job Model, Section 2.6.6) |
| **Agent** | An AI model configured for a specific role with the ability to take actions |
| **AI Job** | Durable, capability-scoped unit of AI work executed by the workflow engine; see (AI Job Model, Section 2.6.6) |
| **AI Job Profile** | Artefact-specific extension of the core AI job model (e.g., Docs Profile, ASR Profile); see (AI Job Model, Section 2.6.6).6 |
| **API** | Application Programming Interfaceâ€”how programs communicate with each other |
| **Automerge** | A CRDT library that stores full history, good for version tracking but higher memory usage |
| **AutoGen** | Microsoft's multi-agent conversation framework |
| **Batching** | Processing multiple requests together for efficiency |
| **Block-Based Editor** | Editor where content is made of stackable blocks instead of continuous text |
| **Capability Model** | Security pattern where code receives explicit permission tokens for specific resources |
| **Chromium** | Open-source browser engine that Chrome is built on |
| **ComfyUI** | Node-based visual tool for Stable Diffusion image generation |
| **Context Window** | How many tokens an LLM can "see" at onceâ€”its working memory |
| **Continuous Batching** | Advanced batching that dynamically adds/removes requests mid-generation |
| **CRDT** | Conflict-free Replicated Data Typeâ€”enables automatic merge of concurrent edits |
| **CUDA** | NVIDIA's technology for running computations on GPUs |
| **Default Deny** | Security stance where nothing is permitted unless explicitly granted |
| **Desktop Shell** | Program that wraps web code to run as a native desktop application |
| **Electron** | Popular desktop shell that bundles Chromium and Node.js |
| **EntityRef** | ID-based reference into workspace artefacts used by AI jobs to specify scope; see (AI Job Model, Section 2.6.6).2.3 |
| **GGUF** | File format for quantized AI models, used by llama.cpp and Ollama |
| **Golden Test Suite** | Set of representative prompts with expected properties to verify |
| **GPU** | Graphics Processing Unitâ€”hardware that runs AI models very fast |
| **Hot Model** | A model kept loaded in VRAM for instant response |
| **HyperFormula** | Open-source spreadsheet formula engine with 400+ functions |
| **Inference** | Using a trained AI model to generate outputs (vs training which creates the model) |
| **IPC** | Inter-Process Communicationâ€”how different parts of an app talk to each other |
| **KV Cache** | Key-Value cacheâ€”memory used to store conversation context during inference |
| **LangGraph** | LangChain's graph-based agent orchestration framework |
| **Langfuse** | Open-source LLM observability platform |
| **layer_scope** | Job field specifying which content layers (raw, derived, display) may be read/written; see (AI Job Model, Section 2.6.6) |
| **Lead/Worker Pattern** | Smart model plans, simpler models execute |
| **LLM** | Large Language Modelâ€”AI trained on text to understand and generate language |
| **LLM-as-Judge** | Using another LLM to rate output quality on criteria |
| **Local-First** | Architecture where data lives primarily on user's device, not in the cloud |
| **Loro** | A new CRDT library with full history and movable trees, written in Rust |
| **Manifest** | Configuration file declaring a plugin's metadata, permissions, and capabilities |
| **Monorepo** | Single repository containing multiple related projects |
| **OAuth2** | Standard protocol for secure third-party authorization |
| **Observability** | Ability to understand internal system state from external outputs |
| **Ollama** | Easy-to-use local LLM runner |
| **On-Demand Model** | A model loaded only when specifically needed |
| **OpenTelemetry (OTel)** | Industry standard for collecting metrics, traces, and logs |
| **Orchestrator** | Code that coordinates multiple AI models to work together |
| **PagedAttention** | vLLM's memory optimization technique for efficient KV cache management |
| **Parameters** | The "knobs" inside an AI model (more = smarter but heavier) |
| **Plugin Manifest** | JSON file declaring plugin metadata, permissions, and contributions |
| **PlannedOperation** | Typed struct describing an intended workspace mutation in an AI job (e.g., `insert_block`, `apply_formula`); see (AI Job Model, Section 2.6.6).2.4 |
| **Prometheus** | Time-series database commonly used for metrics |
| **Property-Based Test** | Test checking structural properties rather than exact content |
| **Pyodide** | Full Python interpreter compiled to WASM |
| **Q4/Q5/Q8** | Quantization levelsâ€”lower numbers mean smaller size but slightly lower quality |
| **Quantization** | Shrinking AI models to use less memory by reducing number precision |
| **RAG** | Retrieval Augmented Generationâ€”technique for handling long documents |
| **REST API** | Common style for web APIs using HTTP methods |
| **Runtime** | Software that loads and executes AI models |
| **Sandbox** | Isolated environment where untrusted code can run safely |
| **safety_mode** | Job field specifying behavior preset: `strict`, `normal`, or `experimental`; see (AI Job Model, Section 2.6.6) |
| **SDXL** | Stable Diffusion XLâ€”high-quality image generation model |
| **Sidecar File** | Small metadata file that accompanies a main file |
| **Slash Commands** | Type "/" to access insertion menu in editors |
| **Span** | A single unit of work within a trace |
| **SQLite** | Lightweight database contained in a single file |
| **Streaming** | Sending response tokens one at a time as they're generated |
| **Tauri** | Lightweight desktop shell using Rust and system webview |
| **TGI** | Text Generation Inferenceâ€”HuggingFace's production LLM server |
| **Tiptap** | Extensible rich text editor framework built on ProseMirror |
| **Token** | A chunk of text (roughly Â¾ of a word) that LLMs process |
| **Trace** | End-to-end record of a request's path through the system |
| **vLLM** | High-performance LLM inference engine optimized for throughput |
| **VRAM** | Video RAMâ€”memory on graphics card where AI models run |
| **WASM (WebAssembly)** | Binary format that runs in a secure sandbox |
| **WebSocket** | Protocol for real-time, two-way communication |
| **Yjs** | Popular JavaScript CRDT library with excellent editor integrations |

---

**Key Takeaways**  
- This glossary provides definitions for all technical terms used throughout the specification.
- Terms are organized alphabetically for quick lookup.
- Cross-reference with relevant sections for deeper understanding.

---

## 8.5 Sources Referenced

**Why**  
Documenting sources enables verification, further research, and acknowledgment of the research foundation.

**What**  
Lists all source documents that were synthesized into this unified specification.

---

### 8.5.1 Source Documents

This document consolidates research from the following sources:

#### 8.5.1.1 Part II (LLM Infrastructure) Sources
1. **LLM Inference Runtimes** (8 pages) â€” Runtime comparison, model candidates, image generation, scheduling patterns
2. **Inference Runtimes** (7 pages) â€” Runtime comparison, model selection by role, GPU bottlenecks, recommendations
3. **Benchmark Harness Design** (5 pages) â€” Modular Python benchmark architecture, adapters, scenarios, reporting

#### 8.5.1.2 Part III (Data Architecture) Sources
4. **Local-First Data and Sync Architecture** (9 pages) â€” CRDT libraries, database patterns, sync topologies, conflict resolution UX

#### 8.5.1.3 Part IV (Plugin System) Sources
5. **Extension Platforms: Architectural Overview** (10 pages) â€” Plugin system analysis (VS Code, Obsidian, Figma, browsers), proposed architecture
6. **Sandboxing Options for Untrusted Code** (12 pages) â€” WASM, Pyodide, OS sandboxing, permission models, security architecture

#### 8.5.1.4 Part V (Observability & Testing) Sources
7. **AI Observability and Evaluation** (10 pages) â€” Logging, metrics, privacy, evaluation methods, multi-agent tracing, phased rollout

#### 8.5.1.5 Part VII (Consolidated Architecture) Sources
8. **Handshake_Project.pdf** (9 pages) â€” Core specification: multi-model orchestration, UI frameworks, Google API integration, ComfyUI, architecture overview
9. **Model_Strategy_and_Tooling_Guide.pdf** (4 pages) â€” AI assistant usage strategy, Codex vs GPT-4/Claude roles, n8n evaluation
10. **Reference_App_Deep_Dive_Local-First_Open_Workspace_Tools.pdf** (7 pages) â€” Technical analysis of AppFlowy, AFFiNE, Anytype, Logseq, Obsidian, Joplin
11. **Tauri_Electron_Decision.pdf** (4 pages) â€” Framework comparison, consensus from multiple AI advisors recommending Tauri
12. **Project_Health_Hygiene_Guide.pdf** (7 pages) â€” Codebase standards, testing, CI/CD, logging, AI-friendly practices
13. **Development_Roadmap_Draft.pdf** (7 pages) â€” Phase planning, implementation order, testing strategy, deployment
14. **Notion_vs_Milanote_vs_Excel_Feature_Comparison.pdf** (4 pages) â€” Target app analysis, orchestration framework comparison, local model recommendations
#### 8.5.1.6 Part VIII (Embedded Protocol Sources)

15. **Handshake Docs & Sheets AI Integration Protocol (v0.5-draft)** (Markdown spec) â€” Defines AI jobs over documents and sheets, stable IDs and entity references, provenance fields, job configuration, observability, safety rules, and a threat model; now fully integrated as Section 2.5.10 of this specification.

16. **AI Job Model v0.1 (Design Note)** â€” Defines the global AI job model, core schema, lifecycle, workflow engine integration, and profile extension pattern; integrated as Section 2.6.6 with profiles in (Docs & Sheets Profile section), (Docling Profile section), (ASR Profile section).
---

### 8.5.2 Document Statistics

- **Total source items:** 16 (14 external documents + 2 design notes/protocol specs)
- **Total page-equivalent:** ~110+ pages
- **Total merged sections:** 33 sections
- **Estimated read time:** ~130 minutes for complete document

---

### 8.5.3 Quick Reference Guide

**For quick lookup:**
- [Section 1.8 - Introduction](#18-introduction) â€” Project overview, 5 min
- [Section 8.2 - Technology Stack Summary](#82-technology-stack-summary) â€” Quick reference
- [Section 7.6 - Development Roadmap](#76-development-roadmap) â€” What to build when
- [Section 8.4 - Consolidated Glossary](#84-consolidated-glossary) â€” Term definitions

**For DECISION POINTs:**
Search for "DECISION POINT" to find all major technical choices with recommendations.

**For Implementation:**
Follow the roadmap in Section 7.6 and refer to specific technical sections as needed.

---

**Key Takeaways**  
- This specification synthesizes 14 source documents totaling ~100+ pages.
- Sources cover infrastructure, data architecture, plugins, observability, and consolidated architecture.
- Use quick reference guide for navigation; search "DECISION POINT" for key choices.

---

## 8.6 Appendices

**Why**  
Appendices provide supplementary reference material including foundation concepts for newcomers, detailed architecture decisions, comparison tables, benchmark data, and works cited that support the main specification.

**What**  
Contains Foundation Concepts (beginner explainers), Architecture Decisions (detailed rationale), Plugin System design, Docling/ASR comparison tables, and works cited.

---

### 8.6.1 Foundation Concepts

*This appendix provides beginner-friendly explanations of core concepts for readers new to desktop application development or AI systems.*

#### 8.6.1.1 Foundation Concepts Overview

Before diving into specific technical decisions, let's establish foundational understanding of the core concepts that appear throughout this document.

---

#### 8.6.1.2 What is a Desktop Application Shell?

**Prerequisites:** None - foundational  
**Related to:** Section 8.6 (Appendices)  
**Implements:** Understanding architecture choices  
**Read time:** ~4 minutes

**A "shell" is the container that turns web code into a desktop application. It's the bridge between your web-based user interface and the operating system.**

---

#### 8.6.1.3 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Desktop Shell** | A program that wraps website-style code so it runs as a regular desktop app (with window controls, file access, etc.) | We need to choose between Tauri and Electron as our shell |
| **Electron** | The most popular shell; used by VS Code, Slack, Discord. Bundles a complete Chrome browser inside your app | Higher memory usage but battle-tested and familiar |
| **Tauri** | A newer, lighter shell using Rust. Uses the operating system's built-in browser instead of bundling one | Much lower memory usageâ€”critical when AI models need that RAM |
| **WebView** | A "browser window without the browser"â€”just the part that displays web pages | Tauri uses the system's webview; Electron bundles its own |
| **IPC (Inter-Process Communication)** | How different parts of a program talk to each other | How the UI will communicate with the Python AI backend |

---

#### 8.6.1.4 The Mental Model

Think of building a desktop app like building a food truck:

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚              DESKTOP SHELL                   â”‚
â”‚         (The food truck itself)              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚           YOUR WEB APP               â”‚    â”‚
â”‚  â”‚      (The kitchen equipment)         â”‚    â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚    â”‚
â”‚  â”‚  â”‚    React + TypeScript       â”‚    â”‚    â”‚
â”‚  â”‚  â”‚   (The menu & recipes)      â”‚    â”‚    â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                    â”‚                         â”‚
â”‚                    â–¼                         â”‚
â”‚    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”          â”‚
â”‚    â”‚     Operating System        â”‚          â”‚
â”‚    â”‚   (Where the truck parks)   â”‚          â”‚
â”‚    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜          â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

**Electron** = A food truck that brings its own generator, water supply, and waste systemâ€”self-contained but heavy.

**Tauri** = A food truck that plugs into the venue's electricity and plumbingâ€”lighter but depends on what's available.

---

#### 8.6.1.5 Why This Matters for Handshake

â•â•â• CORE CONCEPT â•â•â•

> Every megabyte of RAM the shell uses is a megabyte NOT available for AI models.
> 
> - Electron idle: ~150-300 MB RAM
> - Tauri idle: ~10-50 MB RAM
> 
> That 200+ MB difference could mean running a larger AI model or faster response times.

---

#### 8.6.1.13 Key Takeaways

- âœ“ Local-first = your data lives on your device primarily
- âœ“ Critical for privacy when AI models access your documents
- âœ“ Enables offline work and eliminates API costs
- âœ“ CRDTs enable collaboration without central servers
- âœ“ You can still sync to cloudâ€”it's just optional

---

#### 8.6.1.14 What are AI Models and How Do They Run Locally?

**Prerequisites:** None - foundational  
**Related to:** Section 4 (LLM Infrastructure)  
**Implements:** Understanding AI integration approach  
**Read time:** ~6 minutes

**An AI model is a very large mathematical formula that takes in text (or images) and produces intelligent-seeming responses. "Running locally" means this formula executes on YOUR computer, not a company's servers.**

---

#### 8.6.1.15 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **LLM (Large Language Model)** | An AI trained on massive text to understand and generate language. ChatGPT is an LLM. | The "brain" that will write, summarize, and reason |
| **Parameters** | The "knobs" inside the AI model. More parameters = smarter but heavier. "7B" = 7 billion parameters | Determines which models fit on your hardware |
| **VRAM** | Video RAMâ€”memory on your graphics card | Where AI models live during use; RTX 3090 has 24GB |
| **Inference** | The AI actually doing its job (generating a response) | What happens when you ask the AI something |
| **Quantization** | Shrinking a model to fit in less memory (with some quality loss) | How we fit big models on consumer hardware |
| **GGUF** | A file format for quantized models | The format we'll download models in |

---

#### 8.6.1.16 How Big Are These Models?

```
Model Size vs. Quality vs. Hardware Requirements

â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ 70B (GPT-4 class)  â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆ  â”‚ 140GB+ â”‚
â”‚  - Smartest, needs multiple GPUs or cloud    â”‚        â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ 34B (Very Good)    â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆ         â”‚ ~70GB  â”‚
â”‚  - Excellent quality, pushes 3090 limits     â”‚        â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ 13B (Good)         â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆ                 â”‚ ~26GB  â”‚
â”‚  - Great balance, fits 3090 with room       â”‚ â† Sweetâ”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ 7B (Decent)        â–ˆâ–ˆâ–ˆâ–ˆ                     â”‚ ~14GB  â”‚
â”‚  - Fast, leaves room for other models        â”‚  Spot  â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ 3B (Basic)         â–ˆâ–ˆ                       â”‚ ~6GB   â”‚
â”‚  - Quick tasks, limited capability           â”‚        â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.1.17 The Model Zoo

Handshake needs DIFFERENT models for DIFFERENT tasks:

| Task | Model Type | Example | Size |
|------|-----------|---------|------|
| Writing & Reasoning | General LLM | Llama 3, Mistral | 7-13B |
| Code Generation | Code-specialized | Code Llama, StarCoder | 7-15B |
| Image Generation | Diffusion Model | SDXL | ~3B |
| Task Planning | Reasoning LLM | GPT-OSS-20B | 20B |

â•â•â• CORE CONCEPT â•â•â•

> **You won't run all models simultaneously.** The orchestrator loads/unloads models based on what's needed. The 3090 has 24GB; a 13B model uses ~14GB quantized, leaving 10GB for SDXL image generation.

---

#### 8.6.1.18 Local vs. Cloud AI

```
                    LOCAL                     CLOUD (API)
                      â”‚                           â”‚
    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”
    â”‚ âœ“ Private - data stays home   â”‚  â”‚ âœ— Data sent to  â”‚
    â”‚ âœ“ Free after download         â”‚  â”‚   company       â”‚
    â”‚ âœ“ Works offline               â”‚  â”‚ âœ— Per-request   â”‚
    â”‚ âœ— Limited by your hardware    â”‚  â”‚   cost          â”‚
    â”‚ âœ— Slower than cloud GPUs      â”‚  â”‚ âœ— Needs internetâ”‚
    â”‚                               â”‚  â”‚ âœ“ Latest models â”‚
    â”‚ GOOD FOR: Frequent,           â”‚  â”‚ âœ“ Most powerful â”‚
    â”‚ routine tasks                 â”‚  â”‚                 â”‚
    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚ GOOD FOR: Hard  â”‚
                                       â”‚ tasks, fallback â”‚
                                       â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.1.19 How It Actually Works

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    YOUR COMPUTER                         â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚              PYTHON BACKEND                      â”‚    â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚    â”‚
â”‚  â”‚  â”‚         MODEL RUNTIME                    â”‚    â”‚    â”‚
â”‚  â”‚  â”‚  (Handshake native engines/adapters)   â”‚    â”‚    â”‚
â”‚  â”‚  â”‚                                          â”‚    â”‚    â”‚
â”‚  â”‚  â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”       â”‚    â”‚    â”‚
â”‚  â”‚  â”‚   â”‚Llama 3â”‚  â”‚ Code  â”‚  â”‚ SDXL  â”‚       â”‚    â”‚    â”‚
â”‚  â”‚  â”‚   â”‚ 13B   â”‚  â”‚ Llama â”‚  â”‚       â”‚       â”‚    â”‚    â”‚
â”‚  â”‚  â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”˜       â”‚    â”‚    â”‚
â”‚  â”‚  â”‚        â”‚          â”‚          â”‚          â”‚    â”‚    â”‚
â”‚  â”‚  â”‚        â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜          â”‚    â”‚    â”‚
â”‚  â”‚  â”‚                   â”‚                     â”‚    â”‚    â”‚
â”‚  â”‚  â”‚           GPU (RTX 3090)                â”‚    â”‚    â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚    â”‚
â”‚  â”‚                     â”‚                           â”‚    â”‚
â”‚  â”‚           Orchestrator (AutoGen/LangGraph)      â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                        â”‚                                â”‚
â”‚              â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                      â”‚
â”‚              â”‚  HTTP/WebSocket   â”‚                      â”‚
â”‚              â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                      â”‚
â”‚                        â”‚                                â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚              TAURI SHELL + REACT UI              â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.1.26 Key Takeaways

- âœ“ Different AI models excel at different tasks
- âœ“ An "orchestrator" coordinates which model handles what
- âœ“ The lead/worker pattern: smart model plans, simple models execute
- âœ“ This approach balances quality, cost, and speed
- âœ“ All coordination happens in the Python backend

**See Also:** [Section 7.2 - Multi-Agent Orchestration](#72-multi-agent-orchestration)

---
---

### 8.6.2 Architecture Decisions

*This appendix provides detailed rationale behind key architecture choices including desktop shell selection, system architecture, and data architecture.*

#### 8.6.2.1 Architecture Decisions Overview

This section covers the major architectural choices for Project Handshake, based on research and multi-source analysis.

---

#### 8.6.2.2 Desktop Shell: Tauri vs Electron

**Prerequisites:** Section 8.6 (Appendices)  
**Related to:** Section 2.1 (High-Level Architecture)  
**Implements:** Core technology choice  
**Read time:** ~7 minutes

**This section explains why Tauri was chosen over Electron as the desktop shell, based on consensus from multiple AI advisors and research documents.**

---

#### 8.6.2.3 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Chromium** | The open-source browser that Chrome is built on | Electron bundles this; it's why Electron apps are large |
| **Rust** | A programming language focused on speed and safety | Tauri's backend is written in Rust |
| **System WebView** | The browser component already on your computer | Tauri uses this instead of bundling Chromium |
| **Binary Size** | How big the app installer is | Tauri: ~10-30MB; Electron: ~100-200MB |
| **Memory Footprint** | RAM used when app is running | Critical when AI models need that RAM |

---

#### 8.6.2.4 The Decision

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    DECISION POINT                            â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ What needs to be decided: Desktop application shell          â”‚
â”‚                                                              â”‚
â”‚ Options researched:                                          â”‚
â”‚   â€¢ Electron (used by VS Code, Slack, Discord)              â”‚
â”‚   â€¢ Tauri (newer, used by some AI apps)                     â”‚
â”‚   â€¢ Flutter (AppFlowy uses this - different paradigm)       â”‚
â”‚                                                              â”‚
â”‚ Recommendation: TAURI                                        â”‚
â”‚                                                              â”‚
â”‚ Rationale:                                                   â”‚
â”‚   â€¢ 90% less memory usage (crucial for AI models)           â”‚
â”‚   â€¢ Smaller install size                                     â”‚
â”‚   â€¢ Better security model for plugins                        â”‚
â”‚   â€¢ Python backend means shell is "just a wrapper"          â”‚
â”‚                                                              â”‚
â”‚ Tradeoffs:                                                   â”‚
â”‚   â€¢ Smaller ecosystem than Electron                          â”‚
â”‚   â€¢ Rust knowledge needed for advanced shell features       â”‚
â”‚   â€¢ Some webview quirks across operating systems            â”‚
â”‚   â€¢ AFFiNE actually switched FROM Tauri TO Electron         â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.2.5 Head-to-Head Comparison

| Factor | Electron | Tauri | Winner for Handshake |
|--------|----------|-------|---------------------|
| **Memory at idle** | 150-300 MB | 10-50 MB | âš¡ **Tauri** |
| **Install size** | 100-200 MB | 10-30 MB | **Tauri** |
| **Startup time** | 1-2 seconds | Sub-second | **Tauri** |
| **Ecosystem maturity** | Excellent | Growing | Electron |
| **Documentation** | Extensive | Good | Electron |
| **Security model** | Permissive | Deny-by-default | âš¡ **Tauri** |
| **Cross-platform consistency** | Very consistent | Some quirks | Electron |
| **Node.js integration** | Built-in | Not applicable | Electron |
| **Rust backend** | Not applicable | Built-in | Context-dependent |

---

#### 8.6.2.6 Why Memory Matters So Much

â•â•â• CORE CONCEPT â•â•â•

```
Available GPU Memory (RTX 3090): 24 GB
â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

WITH ELECTRON (300MB shell overhead):
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â”‚
â”‚  LLM Model (14GB)          â”‚  SDXL(~8GB)  â”‚
â”‚                            â”‚  Cramped!    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
System RAM also constrained for model loading

WITH TAURI (30MB shell overhead):
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚â–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–ˆâ–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â–‘â”‚
â”‚  LLM Model (14GB)          â”‚  SDXL (10GB) â”‚
â”‚                            â”‚  Comfortable â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
270MB more RAM available for models/context
```

---

#### 8.6.2.7 The Research Consensus

Three independent analyses (GPT-4, Claude, and Gemini) were asked to evaluate this decision. **All three recommended Tauri** for the following reasons:

ðŸ“Œ **Key Points from Multi-AI Analysis:**

1. **Resource Efficiency Under AI Load**
   > "Every megabyte of RAM you save in the shell is headroom for bigger models, more context windows, and smoother SDXL runs."

2. **Architecture Alignment**
   > "Your backend is Python, not Node. The hard logic is not written in Rust; it is in Python and TypeScript."

3. **Long-Term Product Vision**
   > "This is not a tiny helper tool; it is your primary local-first, multi-model AI workspace."

4. **Security for Plugins**
   > "Tauri has a stricter, deny-by-default permission model, which makes it safer to load third-party code."

---

### âš ï¸ Risk: AFFiNE's Tauri-to-Electron Switch

One research document notes that AFFiNE, a similar local-first workspace app, **switched FROM Tauri BACK to Electron** due to webview limitations on macOS.

**Mitigation strategies:**
- Test extensively on all target platforms early
- Keep Tauri shell responsibilities minimal (just window management and IPC)
- Design the architecture so a shell swap is possible if absolutely necessary
- Monitor Tauri's development and webview improvements

---

#### 8.6.2.8 What Tauri Actually Does in This Architecture

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    TAURI'S RESPONSIBILITIES                  â”‚
â”‚                    (Keep this list SHORT)                    â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  âœ“ Create application window                                â”‚
â”‚  âœ“ Load the React UI                                        â”‚
â”‚  âœ“ Spawn Python backend process                             â”‚
â”‚  âœ“ Handle file system access (with permissions)             â”‚
â”‚  âœ“ Manage window state (minimize, maximize, etc.)           â”‚
â”‚  âœ“ Surface system metrics (GPU usage, memory)               â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                    NOT TAURI'S JOB                          â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚  âœ— AI orchestration (Python does this)                      â”‚
â”‚  âœ— Data processing (Python/TypeScript)                      â”‚
â”‚  âœ— Business logic (React/Python)                            â”‚
â”‚  âœ— Model management (Python backend)                        â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

ðŸ’¡ **Tip:** Think of Tauri as a "thin wrapper"â€”it should do as little as possible. Complex logic stays in Python and TypeScript where iteration is easier.

---

#### 8.6.2.9 Key Takeaways

- âœ“ **Decision: Use Tauri** as the desktop shell
- âœ“ Primary reason: Memory efficiency for AI models
- âœ“ Secondary reasons: Security model, smaller installs, faster startup
- âœ“ Risk acknowledged: AFFiNE switched away; we mitigate by keeping Tauri's role minimal
- âœ“ Frontend code (React/TypeScript) works identically in both shells
- âœ“ If issues arise, shell swap is possible without rewriting business logic

**See Also:** [Section 8.6 - Overall System Architecture](#21-high-level-architecture)

---

#### 8.6.2.10 Overall System Architecture

**Prerequisites:** Section 8.6 (Foundation Concepts), Section 8.6 (Appendices)  
**Related to:** All implementation sections  
**Implements:** System blueprint  
**Read time:** ~8 minutes

**This section presents the complete system architecture: how all the pieces connect and communicate.**

---

#### 8.6.v02.13 Jargon Glossary

| Term | Plain English | Why It Matters for Handshake |
|------|--------------|------------------------------|
| **Frontend** | The part users see and interact with (buttons, text, etc.) | React/TypeScript in the Tauri window |
| **Backend** | The "behind the scenes" code that does heavy lifting | Python: AI, file processing, orchestration |
| **API** | A set of "commands" one program can send to another | How frontend talks to backend |
| **REST API** | A common style for APIs using web requests (GET, POST, etc.) | Simple, well-understood pattern |
| **WebSocket** | A persistent connection for real-time, two-way communication | For streaming AI responses |
| **Monorepo** | One repository containing multiple related projects | Frontend and backend code together |
| **Microservices** | Breaking an app into separate, independent services | Each AI model could be its own service |

---

#### 8.6.2.12 The Big Picture

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                           USER'S COMPUTER                                â”‚
â”‚                                                                          â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚                        TAURI SHELL                               â”‚    â”‚
â”‚  â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚    â”‚
â”‚  â”‚  â”‚                    REACT FRONTEND                          â”‚  â”‚    â”‚
â”‚  â”‚  â”‚                                                            â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”             â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â”‚ Document â”‚   â”‚  Canvas  â”‚   â”‚  Sheets  â”‚   Â·Â·Â·       â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â”‚  Editor  â”‚   â”‚  Board   â”‚   â”‚  Grid    â”‚             â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â”‚(Tiptap)  â”‚   â”‚(Excali)  â”‚   â”‚(Hyper)   â”‚             â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜             â”‚  â”‚    â”‚
â”‚  â”‚  â”‚                                                            â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â”‚              FILE TREE SIDEBAR                      â”‚  â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â”‚     (Workspace Navigator)                           â”‚  â”‚  â”‚    â”‚
â”‚  â”‚  â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚  â”‚    â”‚
â”‚  â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                  â”‚                                       â”‚
â”‚                    HTTP/WebSocket (localhost)                           â”‚
â”‚                                  â”‚                                       â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚                      PYTHON BACKEND                              â”‚    â”‚
â”‚  â”‚                                                                  â”‚    â”‚
â”‚  â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚    â”‚
â”‚  â”‚   â”‚                   ORCHESTRATOR                          â”‚    â”‚    â”‚
â”‚  â”‚   â”‚              (AutoGen or LangGraph)                     â”‚    â”‚    â”‚
â”‚  â”‚   â”‚                                                         â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â”‚   Planner   â”‚   â”‚   Writer    â”‚   â”‚   Coder     â”‚  â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â”‚   Agent     â”‚   â”‚   Agent     â”‚   â”‚   Agent     â”‚  â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚    â”‚    â”‚
â”‚  â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚    â”‚
â”‚  â”‚                                â”‚                                 â”‚    â”‚
â”‚  â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚    â”‚
â”‚  â”‚   â”‚                  MODEL RUNTIMES                         â”‚    â”‚    â”‚
â”‚  â”‚   â”‚                                                         â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”   â”Œâ”€â”€â”€â”€â”€â”€â”€â”  â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â”‚ Native â”‚   â”‚  GPU    â”‚   â”‚ Image   â”‚   â”‚Cloud  â”‚  â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â”‚ (LLMs) â”‚   â”‚Runtime â”‚   â”‚ Graph   â”‚   â”‚Opt-in â”‚  â”‚    â”‚    â”‚
â”‚  â”‚   â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜   â””â”€â”€â”€â”€â”€â”€â”€â”˜  â”‚    â”‚    â”‚
â”‚  â”‚   â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚    â”‚
â”‚  â”‚                                â”‚                                 â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                   â”‚                                      â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”    â”‚
â”‚  â”‚                     LOCAL FILE SYSTEM                             â”‚    â”‚
â”‚  â”‚                                                                   â”‚    â”‚
â”‚  â”‚   /Handshake/                                                    â”‚    â”‚
â”‚  â”‚   â”œâ”€â”€ workspaces/                                                â”‚    â”‚
â”‚  â”‚   â”‚   â””â”€â”€ my-project/                                           â”‚    â”‚
â”‚  â”‚   â”‚       â”œâ”€â”€ notes/           (Markdown files)                 â”‚    â”‚
â”‚  â”‚   â”‚       â”œâ”€â”€ canvas/          (JSON board data)                â”‚    â”‚
â”‚  â”‚   â”‚       â”œâ”€â”€ sheets/          (CSV/JSON data)                  â”‚    â”‚
â”‚  â”‚   â”‚       â”œâ”€â”€ images/          (Generated + uploaded)           â”‚    â”‚
â”‚  â”‚   â”‚       â””â”€â”€ .handshake/      (Metadata, CRDT state)          â”‚    â”‚
â”‚  â”‚   â”œâ”€â”€ models/                  (Downloaded AI models)           â”‚    â”‚
â”‚  â”‚   â””â”€â”€ config/                  (User settings)                  â”‚    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜    â”‚
â”‚                                                                          â”‚
â”‚                         â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                           â”‚
â”‚                         â”‚   OPTIONAL CLOUD   â”‚                           â”‚
â”‚                         â”‚  (Google Drive,    â”‚                           â”‚
â”‚                         â”‚   GPT-4 API, etc.) â”‚                           â”‚
â”‚                         â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                           â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.2.13 Architecture Pattern: Monorepo with Hybrid Processes

â•â•â• CORE CONCEPT â•â•â•

> **One codebase, multiple processes.** Everything lives in one Git repository, but runs as separate programs that communicate over the network.
>
> ```
> /handshake-repo/
> â”œâ”€â”€ ui/              # React/TypeScript frontend
> â”œâ”€â”€ backend/         # Python orchestrator + APIs  
> â”œâ”€â”€ shared/          # Type definitions, schemas
> â””â”€â”€ docs/            # Documentation
> ```
>
> This gives us:
> - âœ“ Unified versioning (frontend and backend always match)
> - âœ“ Isolation (Python crash doesn't kill UI)
> - âœ“ Flexibility (can restart backend without UI reload)

---

#### 8.6.2.14 Communication Flow

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    USER INTERACTION FLOW                        â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                                 â”‚
â”‚  1. User clicks "Summarize this document"                      â”‚
â”‚                        â”‚                                        â”‚
â”‚                        â–¼                                        â”‚
â”‚  2. React sends HTTP POST to localhost:8000/api/summarize      â”‚
â”‚     {                                                          â”‚
â”‚       "document_id": "abc123",                                 â”‚
â”‚       "style": "brief"                                         â”‚
â”‚     }                                                          â”‚
â”‚                        â”‚                                        â”‚
â”‚                        â–¼                                        â”‚
â”‚  3. Python backend receives, routes to orchestrator            â”‚
â”‚                        â”‚                                        â”‚
â”‚                        â–¼                                        â”‚
â”‚  4. Orchestrator picks model: local Llama 3 (13B)             â”‚
â”‚                        â”‚                                        â”‚
â”‚                        â–¼                                        â”‚
â”‚  5. Model generates summary, streaming via WebSocket           â”‚
â”‚                        â”‚                                        â”‚
â”‚                        â–¼                                        â”‚
â”‚  6. React displays streaming text to user                      â”‚
â”‚                        â”‚                                        â”‚
â”‚                        â–¼                                        â”‚
â”‚  7. Final result saved to document file                        â”‚
â”‚                                                                 â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.2.15 Why Not Full Microservices?

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    DECISION POINT                                    â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚ What needs to be decided: How to structure backend services          â”‚
â”‚                                                                      â”‚
â”‚ Options researched:                                                  â”‚
â”‚   â€¢ Full microservices (each model in external containers)          â”‚
â”‚   â€¢ Monolith (everything in one Python process)                     â”‚
â”‚   â€¢ Hybrid (multiple processes, no containers)                      â”‚
â”‚                                                                      â”‚
â”‚ Recommendation: HYBRID APPROACH                                      â”‚
â”‚                                                                      â”‚
â”‚ Rationale:                                                           â”‚
â”‚   â€¢ Full microservices adds external-app/container complexity       â”‚
â”‚   â€¢ Monolith risks one crash killing everything                     â”‚
â”‚   â€¢ Hybrid: spawn Python processes for each service                 â”‚
â”‚                                                                      â”‚
â”‚ Implementation:                                                      â”‚
â”‚   â€¢ Main orchestrator process                                        â”‚
â”‚   â€¢ Model runtimes as separate processes (can restart independently)â”‚
â”‚   â€¢ Communication via localhost HTTP (simple, debuggable)           â”‚
â”‚                                                                      â”‚
â”‚ Tradeoffs:                                                           â”‚
â”‚   â€¢ Slightly more complex than monolith                             â”‚
â”‚   â€¢ Less isolated than strong sandbox adapters (shared filesystem)  â”‚
â”‚   â€¢ Good balance for desktop app context                            â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.2.16 Startup Sequence

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    APP STARTUP SEQUENCE                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                               â”‚
â”‚  1. User double-clicks Handshake.app                         â”‚
â”‚                        â”‚                                      â”‚
â”‚                        â–¼                                      â”‚
â”‚  2. Tauri shell starts                                       â”‚
â”‚     â€¢ Creates application window                             â”‚
â”‚     â€¢ Loads React frontend                                   â”‚
â”‚                        â”‚                                      â”‚
â”‚                        â–¼                                      â”‚
â”‚  3. Tauri spawns Python backend                              â”‚
â”‚     â€¢ python -m handshake.server                            â”‚
â”‚     â€¢ Backend starts on localhost:8000                       â”‚
â”‚                        â”‚                                      â”‚
â”‚                        â–¼                                      â”‚
â”‚  4. Backend initializes orchestrator                         â”‚
â”‚     â€¢ Loads model registry (what models are available)       â”‚
â”‚     â€¢ Does NOT load models yet (wait for demand)             â”‚
â”‚                        â”‚                                      â”‚
â”‚                        â–¼                                      â”‚
â”‚  5. Frontend polls /health endpoint                          â”‚
â”‚     â€¢ Shows "Loading..." until backend ready                 â”‚
â”‚     â€¢ Then displays workspace                                â”‚
â”‚                        â”‚                                      â”‚
â”‚                        â–¼                                      â”‚
â”‚  6. First AI request triggers model loading                  â”‚
â”‚     â€¢ Model loaded to GPU on first use                       â”‚
â”‚     â€¢ Subsequent requests are fast                           â”‚
â”‚                                                               â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.2.25 Key Takeaways

- âœ“ **Files are the source of truth**, not a database
- âœ“ Standard formats (Markdown, CSV, JSON) = portable, readable data
- âœ“ Sidecar files store metadata without modifying originals
- âœ“ SQLite used only for fast search/indexing
- âœ“ Folder structure mirrors logical organization
- âœ“ AI generation parameters stored for reproducibility

**See Also:** [Section 3 - Collaboration and Sync](#73-collaboration-and-sync)

---
---

### 8.6.3 Plugin and Extension System (Expanded)

*This appendix provides additional detail on plugin system design and extensibility patterns.*

#### 8.6.3.1 Plugin and Extension System Overview

This section covers how to design Handshake as an extensible platform.

---

#### 8.6.3.2 Plugin Architecture Patterns

**Prerequisites:** Section 2.1 (High-Level Architecture)  
**Related to:** Section 4.2 (Security)  
**Implements:** Extensibility foundation  
**Read time:** ~5 minutes

**A good plugin system lets third parties (and you) extend the app without modifying core code.**

---

#### 8.6.3.3 Lessons from Reference Apps

Based on research of existing apps:

| App | Plugin Approach | Lesson for Handshake |
|-----|-----------------|---------------------|
| **Obsidian** | JS plugins in main process | Large ecosystem, some stability risks |
| **Joplin** | Sandboxed, separate process | Safer but more complex |
| **Logseq** | JS API, ClojureScript | Good API, some breaking changes |
| **VS Code** | Extension host process | Gold standard, but complex |

---

#### 8.6.3.4 Recommended Approach

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                    PLUGIN ARCHITECTURE                       â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                              â”‚
â”‚  PHASE 1 (MVP): Internal Extension Points                   â”‚
â”‚  â€¢ Define stable internal APIs                              â”‚
â”‚  â€¢ Build core features as "internal plugins"                â”‚
â”‚  â€¢ Establishes patterns for later                           â”‚
â”‚                                                              â”‚
â”‚  PHASE 2: User Scripts                                      â”‚
â”‚  â€¢ Allow simple automation scripts                          â”‚
â”‚  â€¢ Sandboxed JavaScript/Python execution                    â”‚
â”‚  â€¢ Limited API surface                                      â”‚
â”‚                                                              â”‚
â”‚  PHASE 3: Full Plugin System                                â”‚
â”‚  â€¢ Public plugin API                                        â”‚
â”‚  â€¢ Plugin marketplace                                        â”‚
â”‚  â€¢ Sandboxed execution (like Joplin)                        â”‚
â”‚  â€¢ Permission model                                          â”‚
â”‚                                                              â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

---

#### 8.6.3.5 Extension Categories to Plan For

| Category | Examples | API Needed |
|----------|----------|-----------|
| **Custom Blocks** | New editor block types | Block registration, rendering |
| **AI Agents** | Specialized AI workflows | Agent API, model access |
| **Integrations** | Third-party services | HTTP, auth storage |
| **Views** | New database views | View registration, data access |
| **Themes** | Visual customization | CSS variables, style hooks |

---

#### 8.6.3.10 Key Takeaways

- âœ“ Sandbox plugin execution
- âœ“ Explicit permission requests
- âœ“ User approval for sensitive permissions
- âœ“ Tauri's security model helps here

---
---

### 8.6.4 Docling Feature Comparison Tables

*(See Section 7.2 for detailed format support matrices)*

### 8.6.5 ASR Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Model accuracy insufficient | Medium | High | Multiple model tiers; cloud fallback option |
| GPU resource contention | Medium | Medium | Model tiering; CPU fallback path |
| Whisper license changes | Low | High | Monitor; alternative runtimes available |
| Diarization complexity | High | Medium | Defer to post-MVP; document limitation |
| Audio format compatibility | Low | Low | ffmpeg handles most formats |

### 8.6.6 Works Cited (Docling)

- Docling GitHub Repository: https://github.com/DS4SD/docling
- DocLayNet: Deep Learning Based Document Layout Analysis (IBM Research)
- TableFormer: Table Structure Understanding with Transformers
- LF AI & Data Foundation: Docling Project Page

### 8.6.7 Works Cited (ASR)

- OpenAI Whisper: Robust Speech Recognition via Large-Scale Weak Supervision
- Faster-Whisper: CTranslate2-based Whisper Implementation
- whisper.cpp: High-performance C++ Whisper Inference
- NVIDIA NeMo ASR Documentation
- PaddleSpeech Documentation

---



### 8.6.8 Embedded Source Archives (Atelier/Lens) [ADD v02.123]

This appendix embeds key source files **verbatim** to prevent detail loss during future merges and to keep Handshake fully self-contained (no sidecar specs required).

**Authority note:** These embedded archives are reference material. Normative/implementable requirements are defined in the Main Body (notably Â§6.3.3.5.7.11â€“Â§6.3.3.5.7.25).

---

#### 8.6.8.1 Handshake_Atelier_Lens_Addendum_v0.2.3.md (verbatim)

````markdown
# Handshake â€” Atelier/Lens Addendum Spec
Version: v0.2.3 (Draft)  
Date: 2026-01-30 (Europe/Brussels)  
Patch target: `Handshake_Master_Spec_v02.122.md` (primary: Â§6.3.3.5.7 Atelier/Lens; also ties into Â§2.3.14 AI-Ready Data Architecture, Â§2.4.* pipelines, and ACE-RAG retrieval contracts)  

**Surgical merge intent:** This document is written to be patched into the master spec in-place (not appended).  
**Precedence:** If a conflict exists, **this addendum overrides** `Handshake_Master_Spec_v02.122.md` for Atelier/Lens and any referenced glue (storage/retrieval/view policy) needed to make it operable.

---

## 0. Normative language

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted as described in RFC 2119.

---

## 1. Operator-locked constraints

These are treated as hard requirements.

1. **Default Lens extraction depth = Tier1**  
   Tier1 is the default ingestion/extraction setting across Handshake.

2. **Global index with filters**  
   Descriptors/facts are global across Handshake (not project-bound) and queries filter.

3. **SYM-001 is first-class**  
   Symbolism/SHOT_DNA is not optional; it is a primary lane of extraction + retrieval.

4. **NSFW is default view**  
   SFW/NSFW does **not** affect ingestion or stored descriptors; it affects only retrieval visibility/ranking and output rendering.

5. **No censorship / no softening**  
   Internal extraction data is always raw and explicit. Any SFW behavior is strictly a projection at view/output boundaries and MUST NOT write back.

---

## 2. Definitions (addendum-local)

### 2.1 Atelier and Lens

- **Lens** = extraction + query/control plane (claims, glances, facts, evidence, search, proposal queue)  
- **Atelier** = composition + production plane (creative planning, deliverables, production graphs)

**Opposites rule:** Every role is symmetric: the same role that composes also extracts.

### 2.2 ETL (what â€œETLâ€ means here)

**ETL = Extract â†’ Transform â†’ Load**

- **Extract:** Docling/OCR/ASR/VLM signals + role extractors
- **Transform:** normalize to CONFIG vocab; flatten to Facts; compute embeddings; compute SYM-001 (layer scores / motifs / SHOT_DNA)
- **Load:** write deterministic artifacts + upsert PostgreSQL/EventLedger authority rows + update derived indexes (FTS + vector + graph + role lanes)

### 2.3 Two different â€œtiersâ€ (do not confuse)

Handshake already has `content_tier` (SFW vs adult categories) in descriptor governance.

This addendum introduces **LensExtractionTier** which is about **how deep** the extraction pass is, not about NSFW.

```ts
type LensExtractionTier = 0 | 1 | 2;

/*
Tier0: minimal (ingest + cheap glances; no heavy role extraction)
Tier1: default (claim + top-k role extraction + fact flattening + indexing + optional SYM-001 when eligible)
Tier2: deep (expanded role set + heavy detectors + deeper symbol pass + full lane indexing)
*/
```

### 2.4 ViewMode (SFW/NSFW)

```ts
type ViewMode = "NSFW" | "SFW";

/*
NSFW: raw descriptors and raw rendering
SFW: filtered projection for retrieval + rendering; never modifies stored descriptors
*/
```

---

## 3. Core invariants (non-negotiable)

### 3.1 No write-back censorship (HARD)

No part of Atelier/Lens may modify stored descriptors to satisfy display posture.

- Ingestion (Docling, OCR, ASR, VLM) is always raw.
- Role extraction is always raw.
- Facts and Symbol facts are always raw.
- SFW is implemented as a **filtered projection** only.

### 3.2 Deterministic replay (HARD)

For the following jobs, Handshake MUST be able to replay and reproduce the same Derived artifacts:

- `ATELIER_CLAIM`
- `ATELIER_GLANCE`
- `ATELIER_ROLE_EXTRACT`
- `ATELIER_ROLE_COMPOSE` (when run in strict mode)
- `ATELIER_VOCAB_PROPOSE` / `ATELIER_VOCAB_RESOLVE`
- `ATELIER_LANE_INDEX`
- `SYM-001` jobs inside Lens (symbol extraction pass)

Practical note: even if model output is not 100% stable, replay mode MUST support persisting the effective candidate lists / tie-breaks / selected spans / final bundle hashes so a replay uses the persisted order.


### 3.3 Lossless role catalog + append-only registry (HARD)

Handshake MUST NOT â€œloseâ€ roles via refactors, renames, or re-scoping.

- The canonical role catalog (names, intent, department grouping, and role mechanics) MUST be embedded in the master spec after merge (no external sidecar docs).
- Role identifiers (`role_id`) are **stable**. Renames are aliases; the `role_id` does not change.
- The role registry is **append-only**:
  - new roles MAY be added,
  - existing roles MAY be deprecated (explicitly), but MUST NOT be removed.
- Any change to role definitions MUST be versioned (contract id bump) and logged (Flight Recorder + spec-change log).
- Validators MUST fail any build that removes a previously declared `role_id` or silently changes a roleâ€™s contract surface.


---

## 4. Atelier/Lens runtime model (tightened)

### 4.1 Dual-contract role runtime (recap)

Each role has:

- **Extraction contract**: `ROLE:<role_id>:X:<ver>` â†’ `RoleDescriptorBundle`
- **Compose contract**: `ROLE:<role_id>:C:<ver>` â†’ `RoleDeliverableBundle`

Roles are the atomic unit. There is no separate â€œlens role vs atelier roleâ€.

### 4.2 Claim â†’ Glance â†’ Extract (default Tier1 flow)

Default Tier1 pipeline for a newly ingested artifact:

1. `ATELIER_CLAIM`  
   Produces RoleScore[] distribution + RoleClaim[] for top roles.

2. `ATELIER_GLANCE`  
   Produces cheap RoleGlance[] for â€œall roles gridâ€ (claimed/weak/none + one-line evidence links).

3. `ATELIER_ROLE_EXTRACT` for **top-k** roles (k configurable; default profile-controlled)  
   Produces RoleDescriptorBundle for each role.

4. Transform + Load  
   Flatten bundles to facts; attach evidence; build/update indexes and lanes.

5. SYM-001 pass (Tier1)  
   Run SYM-001 opportunistically whenever there is any usable descriptor substrate; emit `unclear`/`not_available` for missing fields (see Â§9).

Tier2 expands step (3) and may introduce heavier detectors.

### 4.3 Tier1 default selection (HARD)

`LensExtractionTier` default is Tier1. Tier0 must be explicit operator choice.

### 4.4 Tier2 trigger policy (HARD)

Tier2 extraction MUST be scheduled **automatically when the workspace is idle**.

- â€œIdleâ€ is implementation-defined but MUST at minimum mean: no active operator interaction and no foreground (interactive) jobs running.
- Tier2 jobs MUST yield immediately to any foreground job request.
- Tier2 jobs MAY be additionally gated by an operator profile (e.g., max concurrency / compute budget), but the default posture is auto-when-idle.

### 4.5 Role-turn isolation (recommended; determinism support)

Role extract runs SHOULD be executed as **short, isolated turns**:

- For each role turn, the runtime MUST reset role prompt + scratch context window (no cross-role hidden carryover).
- Cross-role knowledge transfer MUST occur only through persisted artifacts (Claim/Glance/Bundles/Facts/ContextPacks).
- This enables repeated â€œall roles passâ€ loops without unloading the underlying model.


---

## 5. Evidence model (click-to-source correctness)

### 5.1 EvidenceRef is mandatory

Every extractor MUST emit bounded EvidenceRefs for evidence-required fields.

Evidence locator types (minimum):

- `doc_span` (doc_id, block_id, char_start/char_end)
- `page_bbox` (doc_id/file_id, page, x/y/w/h)
- `image_bbox` (asset_id, bbox)
- `frame_span` (video_id, t_start/t_end, bbox optional)
- `audio_span` (audio_id, t_start/t_end)
- `table_cell` (doc_id, table_id, row, col)

### 5.2 Parallel evidence (Docling + local VLM)

Lens may use multiple evidence sources for higher accuracy:

- Docling structure/text spans (layout-aware)
- local VLM captions/tags (vision-first)

When multiple sources corroborate a fact, Facts MAY carry multiple evidence refs (or one evidence ref plus a â€œcorroborates[]â€ list).

---

## 6. Canonical descriptor substrate (force multiplier)

Role bundles are role-specific and correct. But every non-LLM tool needs a shared query substrate.

### 6.1 Rule: bundles MUST emit Facts (HARD)

Every successful RoleDescriptorBundle write MUST also emit canonical fact rows.

#### 6.1.1 `AtelierFact` (normalized scalar facts)

```ts
interface AtelierFact {
  fact_id: string;
  workspace_id: string;
  project_id?: string;

  bundle_id: string;
  role_id: string;
  contract_id: string;

  path: string;              // stable JSONPath-like key into bundle fields
  value_type: "string"|"number"|"bool"|"term"|"json";
  value_norm: string;        // normalized scalar (for SQL/FTS)
  term_id?: string;          // CONFIG term id when controlled

  content_tier: "sfw"|"adult_soft"|"adult_explicit";   // governance
  consent_profile_id?: string;

  confidence: number;        // 0..1
  evidence_id?: string;      // required when evidence-required field
  created_at: string;        // timestamp
}
```

#### 6.1.2 `AtelierSymbolFact` (SYM-001 facts)

```ts
interface AtelierSymbolFact {
  sym_fact_id: string;
  workspace_id: string;
  project_id?: string;

  source_bundle_id: string;     // SYM bundle id or role bundle id
  symbol_term_id: string;       // CONFIG/SYM term id
  intensity: number;            // 0..1
  polarity?: "positive"|"negative"|"mixed"|"neutral";

  content_tier: "sfw"|"adult_soft"|"adult_explicit";
  consent_profile_id?: string;

  evidence_id?: string;
  created_at: string;
}
```

### 6.2 Global index with filters (HARD)

Facts and SymbolFacts are global by default. Projects apply filters.

Minimum filter envelope (always applied in Lens):

```ts
interface LensFilterEnvelope {
  workspace_id: string;          // implicit in DB-per-workspace, but explicit in API
  project_id?: string;           // optional project scope
  content_tier_min?: "sfw"|"adult_soft"|"adult_explicit"; // governance gating for retrieval
  consent_profile_id?: string;   // optional
  view_mode: ViewMode;           // NSFW default
  time_range?: { from?: string; to?: string };
  role_ids?: string[];           // restrict to a role lens (search-as-role)
  vocab_namespaces?: string[];   // restrict (DES/TXT/SYM/etc)
}
```


### 6.3 Library growth is expected (HARD)

Handshake is designed for continuous, cross-domain ingestion (e.g., paintings one day, architecture the next, then photography). Therefore:

- The descriptor/fact substrate is a **cumulative library**: new ingestion adds Facts/SymbolFacts; nothing is â€œresetâ€ unless the operator explicitly forks or clears a workspace.
- Growth MUST be safe-by-default:
  - raw extractions are append-only,
  - corrections are expressed as new bundles/facts with higher confidence and clear provenance,
  - old rows are not silently overwritten.
- Tier2 enrichment jobs (role deep passes, lane re-indexing, motif expansion) SHOULD run automatically when the system is idle (per Â§2 decisions) to keep the library usable as it grows.


---

## 7. Persistence contract (PostgreSQL/EventLedger authority)

### 7.1 Deterministic artifact layout (Derived store)

Store bundles and indexes as deterministic artifacts (hash-addressed):

```
derived/atelier/
  bundles/
    descriptor/<artifact_id>/<role_id>/<contract_id>/<bundle_hash>.json
    deliverable/<plan_id>/<role_id>/<contract_id>/<bundle_hash>.json
  vocab/
    snapshots/<namespace>/<vocab_hash>.json
    proposals/<proposal_id>.json
  symbol/
    snapshots/<sym_namespace>/<sym_hash>.json
    bundles/<artifact_id>/<bundle_hash>.json
  lane/
    <role_id>/index/<index_version>/...
    <role_id>/embeddings/<artifact_id>/<embed_hash>.json
```

### 7.2 SQLite schema (portable logical model)

The physical schema may vary, but the logical tables and keys are normative.

**Core tables:**

- `atelier_role_spec(role_id, role_version, department_id, display_name, spec_json, spec_hash)`
- `atelier_contract(contract_id, role_id, kind {X|C}, version, schema_json, schema_hash)`
- `atelier_profile(profile_id, kind, source_text, compiled_json, source_hash, compiled_hash, created_at, updated_at)`
- `atelier_bundle(bundle_id, bundle_kind {descriptor|deliverable|symbol}, artifact_id, plan_id, role_id, contract_id, bundle_hash, created_at, provenance_json, status)`
- `atelier_evidence(evidence_id, bundle_id, artifact_id, kind, locator_json, source_ref, confidence, notes)`
- `atelier_fact(fact_id, bundle_id, role_id, contract_id, path, value_type, value_norm, term_id, confidence, evidence_id, content_tier, consent_profile_id, workspace_id, project_id, created_at)`
- `atelier_symbol_fact(sym_fact_id, source_bundle_id, symbol_term_id, intensity, polarity, evidence_id, content_tier, consent_profile_id, workspace_id, project_id, created_at)`
- `atelier_vocab_snapshot(namespace, vocab_hash, created_at, snapshot_json)`
- `atelier_vocab_proposal(proposal_id, namespace, term, term_type, status, support_count, proposer_role_id, examples_json, decision_provenance_json)`
- `atelier_lane_index(role_id, index_version, built_from_vocab_hash, built_from_contract_ids, built_at, index_meta_json)`
- `atelier_lane_embedding(embed_id, artifact_id, role_id, index_version, anchor_text, vector_blob, embed_hash, provenance_json)`

**Portability notes (SQLite â†’ Postgres):**
- store vectors as blobs now; later map to pgvector
- keep JSON payloads but ensure query keys exist as columns
- FTS is a derived index; not relied on for logical correctness

---

## 8. Retrieval contract (two methods, deterministic)

### 8.1 Lens must expose both retrieval modalities

Lens retrieval MUST combine:

- **Lexical/keyword search** over Facts + Docling text blocks (FTS/BM25)
- **Vector/semantic search** over embeddings for Facts and/or doc blocks
- optional **graph/meta routing** (knowledge graph neighborhood, lane priors)

### 8.2 QueryPlan + RetrievalTrace

Every search returns:

- `QueryPlan` (routes, weights, filters)
- `RetrievalTrace` (candidate ids + scores + tie-break keys + cache hits/misses)

### 8.3 Two-stage retrieval (candidate â†’ rerank)

Default strategy:

1. Candidate generation (cheap; hybrid fusion):
   - lexical candidates
   - vector candidates
   - lane-scoped candidates (if role lens selected)
2. Rerank (bounded; deterministic):
   - dedupe
   - tie-break by stable keys
   - produce final ranked list

### 8.4 Snippet-first reading

Lens MUST avoid â€œread everythingâ€ behavior:

- Search returns bounded snippets and evidence pointers
- Read returns bounded excerpt (span/bbox/frame range)
- escalation is explicit and logged


### 8.5 Lens query API shape (normative)

Lens is not â€œjust UIâ€; it is a query/control plane that all other subsystems can call deterministically.

```ts
interface LensQueryEnvelope {
  query_id: string;

  // One query may run multiple routes in parallel (lexical + vector + lane + graph).
  query_text: string;

  filter: LensFilterEnvelope;

  // Retrieval routing
  routes: {
    lexical: boolean;
    vector: boolean;
    lane?: { role_id: string };     // â€œsearch as roleâ€
    graph?: boolean;
  };

  // Hybrid fusion weights (defaults from profile)
  weights?: { lexical: number; vector: number; lane: number; graph: number };

  // Token + time budgets
  budget: { max_candidates: number; max_results: number; max_read_ops: number };

  // Determinism
  mode: "strict" | "replay";
}

interface LensResultItem {
  kind: "fact" | "symbol_fact" | "doc_span" | "image_asset" | "bundle";
  id: string;                       // fact_id / sym_fact_id / block_id / asset_id / bundle_id

  title: string;
  snippet: string;                  // bounded; may be SFW-projected depending on filter.view_mode

  // Evidence is always linkable; projection never destroys the underlying evidence pointer.
  evidence?: EvidenceRef[];

  // Ranking diagnostics (Operator Consoles)
  score: number;
  route_scores?: { lexical?: number; vector?: number; lane?: number; graph?: number };

  // Projection markers
  projection_applied: boolean;
  projection_kind?: "SFW";
  projection_ruleset_id?: string;
}
```

### 8.6 ContextPacks: LLM-friendly view over Facts (required)

To keep storage/tooling â€œLLM-friendlyâ€ while remaining deterministic, Lens MUST be able to materialize a bounded, hashed context artifact derived from facts and evidence:

- `AtelierContextPack` (Derived artifact)
  - selected facts (with stable ids)
  - selected symbol facts
  - bounded evidence excerpts
  - constraints/open loops
  - lane snapshots used
  - provenance pins (profile hashes, vocab snapshot hashes, model/tool pins)

`AtelierContextPack` is the preferred input to any writer/creative role when they need corpus context, replacing ad-hoc â€œdump the DB into the prompt.â€


---

## 9. SYM-001 in Lens (first-class)

### 9.1 SymbolLexiconSnapshot + SymbolismProfile

SYM-001 uses:

- `SymbolLexiconSnapshot` (global, proposal-grown)
- `SymbolismProfile` (plain-text source + deterministic compilation; may have per-project overlay)

### 9.2 SYM job placement

SYM-001 runs inside Lens as:

- Tier1: **opportunistic** â€” run whenever there is any usable descriptor substrate (Claims/Glances/Facts/Docling spans/VLM tags). There is no hard â€œminimum coverageâ€ gate; missing fields MUST be emitted as `unclear` or `not_available`.
- Tier2: deep profiling (more motifs, broader cross-reference, heavier detectors)

Outputs:
- SHOT_DNA layer scores (as available)
- motif tags
- symbolic intensity facts (`AtelierSymbolFact`)

### 9.3 Symbol template growth + unknown fields (HARD)

SYM output MUST use a **stable, growing template**:

- The template MUST be emitted even when partial; fields that cannot be grounded are set to:
  - `unclear` (a value is conceptually applicable but the system cannot infer it reliably), or
  - `not_available` (the field is not applicable or source signals are missing).
- Re-running SYM on the same artifact MAY refine `unclear` â†’ concrete values as more Facts arrive.
- The current SYM template version used MUST be pinned in provenance (profile hash + template ver + lexicon snapshot hash).


### 9.4 Symbolic engine is a living dataset (HARD)

The symbolic system is not static. Meanings, motifs, and emphasis are expected to drift over time.

Rules:

- `SymbolLexiconSnapshot` is **versioned** and **proposal-grown**. New motifs/terms are added by proposals and become active only once merged into a new snapshot.
- `SymbolismProfile` is **versioned** and MUST support **fork/reset**:
  - a new client/project may use a forked profile,
  - an operator may reset or branch the profile without destroying prior history.
- Re-running SYM against the same artifact MAY legitimately produce different outputs if (and only if) the active profile/snapshot changed; this MUST be visible via provenance pins.
- Past symbol facts are not rewritten in-place. A new SYM run writes new `AtelierSymbolFact` rows with new provenance.

---

## 10. Config profiles + UI editor (deterministic)

### 10.1 Source-of-truth is plain text (HARD)

Profiles are plain text blocks (Monaco), compiled deterministically.

### 10.2 Recommended editor pattern (simple UI + Monaco)

- Monaco edits the plain text blocks directly.
- A lightweight form view can help non-technical editing, but must round-trip to the same text.

The UI MUST show:
- source hash
- compiled hash
- compilation diagnostics

### 10.3 Required profile types for Atelier/Lens

- `ATELIER_GLOBAL_PROFILE` (sets defaults; includes Tier1 default)
- `PROJECT_STYLE_HINT` (project overlay; does not fork storage)
- `SYMBOLISM_PROFILE` (SYM-001)
- `VIEW_POLICY` (NSFW default; SFW projection ruleset)


### 10.4 Profile evolution, drift, and branching (HARD)

Profiles are designed to evolve.

- Editing a profile MUST create a new **versioned** record (new hash); systems MUST NOT silently mutate the meaning of an existing pinned hash.
- Profiles MUST support **branching**:
  - `profile_parent_hash` (or parent id) links a child profile to its ancestor,
  - multiple profiles MAY co-exist (e.g., â€œpersonal styleâ€ vs â€œclient Aâ€).
- Projects MUST pin the exact profile hash(es) used at generation/extraction time.
- Operators MUST be able to:
  - fork profile from any prior version,
  - set an â€œactiveâ€ profile per workspace/project,
  - roll back to a previous version.


---

## 11. NSFW/SFW policy (raw ingest; filtered view/output only)

### 11.1 Ingestion is always raw and uncensored (HARD)

Docling/VLM/OCR/ASR and role extractors ALWAYS run uncensored and write raw descriptors.

### 11.2 SFW affects retrieval + output text only (HARD)

- retrieval: **strict drop** â€” Lens MUST exclude any candidate/result whose `content_tier` is not `sfw`.
- output: apply projection rules during rendering **only** for remaining SFW-visible items.

**Rule (hard drop):** In `ViewMode="SFW"`, Lens MUST NOT return â€œcollapsed/blurred but revealableâ€ result items.  
Inspection of adult tiers requires switching `view_mode` back to `NSFW` (which does not mutate storage).

Projection is non-destructive and must be labeled when applied.

### 11.3 Output labeling (required)

Any SFW-projected output MUST carry:

- `projection_applied=true`
- `projection_kind="SFW"`
- `projection_ruleset_id`
- link to underlying raw evidence (operator can inspect)

---

## 12. Role registry: Digital Production Studio RolePack (draft v1)

Atelier/Lens roles are not ad-hoc prompts; they are versioned RoleSpecs in a RolePack.

### 12.1 Departments + roles (inventory)

This is the initial role inventory (from the Atelier draft). Each role has both X and C contracts.

**Executive Department**
- Producer
- Director (â€œKeeper of Intentâ€)

**Thematic & Psychological Department**
- Writer / Narrative Architect
- Psychological Impact Consultant
- Symbolism & Mythology Consultant (ties to SYM-001)
- Mood Architect

**Context & Culture Department**
- Historian / World-Builder
- Cultural Anthropologist / Trend Forecaster
- Technology Specialist

**World-Building Department**
- Production Designer
- Set Dresser
- Materials Specialist

**Visuals Department**
- Director of Photography
- Cinematographer
- Gaffer (Lighting)
- Camera Technician / Lens Tech (optional split if needed)

**Fashion & Styling Department**
- Fashion Stylist
- Hair Stylist
- Makeup Artist
- Model / Talent

**Finishing Department**
- Editor
- VFX
- Color Grading
- Digital Imaging Technician (DIT)

**Special modes**
- Commercial/Product Photography: Product Stylist, Food Stylist
- Intimacy: Intimacy Coordinator
- Digital Product & UI/UX: UI/UX Designer, Information Architect, Interaction Designer
- Graphic/Typography: Brand Strategist, Typographer, Graphic Designer, Layout Artist
- Architectural/Environmental: Architect, Interior Designer, Furniture/Object Designer, Landscape Architect

### 12.2 RoleSpec skeleton (what each role must declare)

Each role MUST declare at minimum:

- claim features required (`docling.blocks`, `image.frames`, `ocr.text`, `asr.transcript`, `vlm.tags`, etc.)
- extraction schema fields and evidence requirements
- fact mapping rules (bundle â†’ facts)
- compose deliverable kinds (typed outputs)

### 12.3 Seniority/experience encoding (recommended)

To â€œgive roles seniority/experienceâ€ without corrupting determinism:

- encode seniority as **profile-bound role parameters** (plain-text profile â†’ compiled)
- do NOT â€œfreehandâ€ seniority in prompts per run

Example (profile-side):

```
[ROLE:director_of_photography]
experience_level=senior
taste_bias=operator_personal
risk_tolerance=low
```

This ensures replay pins include the same role parameters.

---

## 13. Multi-model parallelism integration (Atelier Track)

Lens and Atelier MUST surface per-role/job model assignment and allow SwapRequest/override within allowed models, with provenance logging.

---

## 14. Cross-tool deliverables (typed + capability-gated)

RoleDeliverableBundles MUST only emit typed deliverables:

- Monaco patch sets
- Doc patch sets (workspace docs)
- Calendar patch sets
- Word/Doc exports (as jobs producing ExportRecords)
- Toolbus plans (PlannedOperations / MEX envelopes)
- Photo Studio jobs (render/proxy/export)
- Chart/Deck specs referencing tables by ID (no data duplication)


### 14.1 DeliverableKind registry (normative)

RoleDeliverableBundles MUST declare deliverables with a `deliverable_kind` that maps to an existing subsystem artifact type.

Minimum set:

| deliverable_kind | Artifact type | Typical consumer | Capability gate |
|---|---|---|---|
| `monaco_patchset` | `CodePatchSet` | Monaco surface | `fs_write` / repo write |
| `doc_patchset` | `DocPatchSet` | Docs surface | `doc_write` |
| `calendar_patchset` | `CalendarPatchSet` | Calendar subsystem | `calendar_write` |
| `word_doc_draft` | `DocDraft` | Exporter | `export_docx` |
| `toolbus_plan` | `PlannedOperation[]` | MEX/Tool Bus | tool-specific |
| `photo_pipeline_job` | `PhotoJobSpec` | Photo Studio | `photo_write` |
| `chart_spec` | `ChartSpec` (refs `TableId`) | Charts | `chart_write` |
| `deck_spec` | `DeckSpec` (refs entities) | Decks | `deck_write` |

**Rule:** deliverables may be proposed without capability, but application MUST obey the existing patch-set discipline (diff/review/accept) and Flight Recorder logging.

### 14.2 Lens UI surfaces (minimum)

Lens MUST expose, at minimum:

- **Claims Panel**: top roles + scores + thresholds + â€œrun Tier2â€ controls
- **Glances Grid**: all roles quick status (none/weak/claimed) with one-line evidence links
- **Bundle Viewer**: view RoleDescriptorBundle + evidence highlights
- **Fact Explorer**: SQL-like filters over `AtelierFact` + evidence drilldown
- **Symbol Explorer**: `AtelierSymbolFact` + SymbolLexicon browsing
- **Lane Search**: â€œsearch as roleâ€ using role lanes (lexical + vector)
- **Proposal Queue**: vocab + symbol proposals accept/merge/reject
- **Model Status**: HSK_STATUS per role/job + SwapRequest + override audit notes
- **Projection Toggle**: NSFW default; SFW projection clearly labeled

### 14.2.1 Atelier Collaboration Panel (selection-scoped) (HARD)

Atelier MUST support a â€œcollaborate on selectionâ€ workflow in text surfaces (Monaco/Docs):

1. Operator selects a bounded text span.
2. Operator invokes Atelier collaboration (button/shortcut).
3. System shows **all roles** in a side panel; each role may emit **0..n suggestions** (multiple suggestions are preferred when available).
4. Operator checks one or more suggestions and applies them.

Application rules:
- The resulting `monaco_patchset` / `doc_patchset` MUST be **range-bounded** to the selected span.
- Validators MUST reject any patch that modifies text outside the selection range (except for explicitly declared boundary-normalization, if enabled).
- Non-selected text MUST remain byte-identical after patch application.

### 14.3 Validators (addendum-required)

Add the following validators (names are indicative; binding points are normative):

- `ATELIER-LENS-VAL-RAW-001` â€” stored descriptors/facts MUST NOT be euphemised or softened
- `ATELIER-LENS-VAL-VIEW-001` â€” SFW is projection-only; any write-back filtering is rejected
- `ATELIER-LENS-VAL-VIEW-002` â€” in `ViewMode="SFW"`, adult-tier items MUST be excluded from result sets (strict drop)
- `ATELIER-LENS-VAL-TIER-001` â€” default LensExtractionTier is Tier1 (unless explicit override)
- `ATELIER-LENS-VAL-SYM-001` â€” SYM-001 outputs MUST be present when the SYM profile is enabled; missing fields are emitted as `unclear`/`not_available`
- `ATELIER-LENS-VAL-PROFILE-001` â€” profile source hash + compiled hash MUST be pinned in provenance for all role jobs
- `ATELIER-LENS-VAL-SCOPE-001` â€” compose patchsets MUST be selection-bounded; changes outside the operator selection are rejected
- `ATELIER-LENS-VAL-FACT-001` â€” evidence-required fact fields MUST have EvidenceRef
- `ATELIER-LENS-VAL-INDEX-001` â€” lexical/vector indexes must be updated for Tier1 completions (or job is marked degraded)


---

## 15. Patch map (surgical insertion points)

1. `Handshake_Master_Spec_v02.122.md` â€” Â§6.3.3.5.7  
   - Insert Â§Â§1â€“14 (this addendum) as subclauses under the existing Atelier/Lens section.

2. Retrieval contract sections (AI-Ready Data Architecture + ACE-RAG)  
   - Add explicit statement: Lens role lanes use the same hybrid search + deterministic tracing model.

3. Descriptor pipeline sections (DES/TXT/IMG/SYM)  
   - Add explicit statement: LensExtractionTier is orthogonal to `content_tier`.

4. UI surfaces  
   - Lens: global filter UI + SFW/NSFW toggle (default NSFW) + projection markers
   - Operator Consoles: QueryPlan/Trace viewer for Lens queries

---

## 16. Decisions captured (v0.2.3)

1. Tier2 trigger policy: **automatic when idle**.
2. SYM-001 minimum coverage: **run whenever possible**; emit a stable template; unknown fields MUST be `unclear` or `not_available`.
3. SFW projection semantics: **strict drop from results** (no collapsed/blurred reveal).
4. Role catalog: **lossless + append-only** (no removals; deprecate only; stable role_id).
5. Living datasets: Symbolic Engine profiles + lexicon + descriptor/fact library are expected to **grow and drift** over time; this is managed via **versioned snapshots + fork/reset**, never silent mutation.

## 17. Open questions (remaining)

1. Symbol lexicon growth: who can propose (SYM-only vs all roles)

---

## 18. Embedded source snapshots (lossless archive)

These snapshots exist to guarantee **no loss of detail** during the addendumâ†’master merge. They are **non-normative unless explicitly referenced** by a normative clause.

### 18.1 extraction and digital production team.md (verbatim)

```md
### **The Digital Production Studio: A Framework for Creative Image Generation**

**Introduction:** This document outlines a framework for a next-generation creative partner. It is designed to translate complex artistic, narrative, and commercial concepts into visually compelling images. It models a complete digital production studio with two primary operating modes: a **Representational Mode** for building scenes and a **Conceptual Mode** for interpreting ideas. At its heart is a powerful **Extraction Engine** that provides a deep vocabulary of descriptors for all specialist roles.

**Core Principle: Inter-Departmental Collaboration.** The departments below are not silos. They can be commissioned to provide services and assets to one another in a "Nested Production" model. For example, the UI/UX Department can design the interface seen on a phone in a character portrait, and the Graphic Design specialist can create the fabric patterns used by the Fashion Stylist.

---

### **1. Creative Modes & Master Controls**

This is the highest-level choice, defining the overall goal of the project.

#### **A. Representational Mode: The Digital Production Studio**
This is the default mode for creating representational images. It uses the full production team to build a scene, whether it's a narrative, a portrait, or a product shot. The workflow is detailed in Section 2.

#### **B. Conceptual Mode: The Creative Core**
When the goal is not to depict a scene but to interpret an idea (e.g., abstract art, satire), this mode is activated. It uses a set of fundamental **Artistic Vectors** to create a visual strategy from a core `Intent`.

*   **The Artistic Vectors (Sliders):**
    *   **Abstraction:** `100% Representational <--> 100% Abstract`
    *   **Clarity:** `Didactic (Clear Message) <--> Ambiguous (Open to Interpretation)`
    *   **Tone:** `Sincere / Earnest <--> Ironic / Satirical`
    *   **Harmony:** `Harmonious / Serene <--> Dissonant / Tense`
    *   **Complexity:** `Minimalist <--> Maximalist / Baroque`
    *   **Familiarity:** `Familiar / Grounded <--> Uncanny / Dreamlike`

The "recipe" created by these vectors is then passed to the Production Team (Section 2) for execution.

#### **C. Studio Philosophy (Master Control)**
This defines the team's collaborative dynamic and applies to both modes.
*   **The Auteur:** User's vision is absolute.
*   **The Hollywood Blockbuster:** Prioritizes spectacle and impact.
*   **The Surrealist Collective:** High AI autonomy, values experimentation.
*   **The Dogme 95:** Operates under strict, user-defined constraints.
*   **The Documentary:** Prioritizes realism and authenticity.

---

### **2. The Production Workflow & Departments**

The creative process, especially in Representational Mode, is organized into three phases.

#### **Phase I: Pre-Production (Concept & Vision)**
*   **The Executive Department:** Producer, Director (Keeper of Intent).
*   **The Thematic & Psychological Department:** Writer, Psychological Impact Consultant, Symbolism & Mythology Consultant, Mood Architect.
*   **The Context & Culture Department:** This department provides deep historical and cultural context, operating under the **Principle of Cultural Authenticity.**
    *   **Principle of Cultural Authenticity:** This department prioritizes authentic, respectful representation of global cultures, actively avoiding monolithic or stereotypical interpretations. It operates on an **"Advisor, not a Gatekeeper"** model.
    *   **"Advisor, not a Gatekeeper" Workflow:**
        1.  **Advise & Inform:** When a request diverges from cultural or historical accuracy (e.g., a "samurai babe"), the system provides context, explains the authentic history (e.g., the 'Onna-musha'), and offers a clear choice between historical accuracy and stylized fantasy.
        2.  **Execute User's Choice:** The system fully respects the user's final decision. If stylized fantasy is chosen, the system proceeds without judgment. However, it will still provide **informed stylization**â€”drawing from its deep knowledge to ensure the fantasy is coherent and avoids jarring, unintentional errors (e.g., ensuring a fantasy samurai still wields a `katana` and wears armor *derived from* Japanese designs, rather than using a European longsword, unless a specific genre blend is the user's explicit intent).
    *   **Historian / World-Builder:** Master of time and place. An expert in real-world art history, regional history (e.g., "Ukiyo-e period Japanese art"), or can be "fed" the lore of a fictional universe.
    *   **Cultural Anthropologist / Trend Forecaster:** Understands cultural movements, subcultures (e.g., "nomadic tribes of a specific region"), and rapidly evolving trends. Its expertise is deeply granular, understanding regional specificity and contextual significance (e.g., the distinct textiles of `Yoruba` vs. `Maasai` cultures).
    *   **Technology Specialist:** Ensures all depicted technology is period-appropriate.

#### **Phase II: Production (Execution & Creation)**
*   **The World-Building Department:**
    *   **Production Designer:** Oversees the entire environment.
    *   **Set Dresser:** Populates the scene with objects and props. Has access to sub-specialists like the **Armorer / Weapons Master** for action scenes.
    *   **Materials Specialist:** Defines the texture and substance of all surfaces.
*   **The Visuals Department (Camera Crew):** Director of Photography (DOP), Cinematographer, Gaffer.
*   **The Fashion & Styling Department:** This department has a globalized and deep understanding of apparel and personal presentation.
    *   **Fashion Stylist:** Expert in fashion history, designers, and concepts (Haute Couture, Streetwear, etc.). Its expertise includes specific historical and stylistic eras (e.g., `1920s Flapper`, `1960s Mod`, `1990s Grunge`), a deep knowledge of specific `fabrics` (e.g., `silk`, `chantilly lace`, `neoprene`) and their properties, and a comprehensive understanding of `patterns` (e.g., `Plaid`, `Paisley`, `Herringbone`). It also has a specialized knowledge of `Lingerie & Boudoir` styling, including historical context and garment vocabulary (`corsetry`, `teddies`, `babydolls`). Its garment vocabulary is explicitly globalized, including `sari`, `hanbok`, `kimono`, `dashiki`, `qipao`, and `caftan`.
    *   **Hair Stylist:** Creates specific and conceptual hairstyles.
    *   **Makeup Artist (MUA):** Specialist in makeup styles, from naturalistic to editorial and avant-garde. Can create looks specific to boudoir and high-fashion lingerie shoots (e.g., `smoky eyes`, `tousled "bedroom hair"`).
    *   **Model / Talent:** Defines the subject's performance, pose, and gaze.

#### **Phase III: Post-Production (Refinement & Polish)**
*   **The Finishing Department:** Editor, VFX Team, Color Grading Team, Digital Imaging Technician (DIT).

---

### **3. Department Specializations & Modes**

The Production Team can operate in specialized modes that re-task all departments for a specific artistic or commercial goal.

#### **A. Commercial & Product Photography Mode**
Focuses on commercial appeal.
*   **New Specialist Role: The Product Stylist:** The art director for objects, including commercial goods like beauty products. Uses techniques like `Clinical/Hero Shot`, `Lifestyle/In-Context`, etc. Includes **Food Stylist** sub-specialty.
*   **Re-tasked Roles:** Gaffer focuses on defining shape/reflections. DOP makes the product the hero. DIT focuses on retouching and color accuracy.

#### **B. The Art of Intimacy & Sensuality Mode**
Focuses on artistic exploration of intimacy and desire.
*   **New Specialist Role: The Intimacy Coordinator:** Guides the talent to express concepts like `vulnerability`, `power dynamics`, and `longing`.
*   **Expanded Roles:** Psychologist explores themes of desire symbolically. Visuals department uses shadow, soft light, and suggestive compositions.

#### **C. Digital Product & UI/UX Design Mode**
Specializes in designing websites and application interfaces.
*   **New Specialist Roles:** `UI/UX Designer` (the lead), `Information Architect` (structure), `Interaction Designer` (animations).
*   **Re-tasked Roles:** For design assets, this mode commissions specialists from the **Graphic Design & Typography Department**. The `DOP` acts as a `Layout Artist` managing grids and visual hierarchy.

#### **D. Graphic Design & Typography Department**
Dedicated to creating all visual design assets and managing typography.
*   **Creative Director / Brand Strategist:** Defines core brand identity and visual strategy.
*   **Typographer:** Master of type selection, pairing, micro-typography (`kerning`, `leading`, `tracking`), hierarchy, and historical context of fonts.
*   **Graphic Designer (Visual & Asset Design):** Creates logos, iconography, illustrations (in various styles), and manages color theory.
*   **Layout Artist (Publication & Grid Design):** Designs layouts for print and digital, applying grid systems, visual hierarchy, and composition principles.

#### **E. Architectural & Environmental Design Department**
Designs all physical structures, interiors, landscapes, and objects.
*   **The Architect:** Designs buildings and structures based on various architectural styles (`Gothic`, `Modernist`, `Brutalist`, etc.) and vernacular traditions.
*   **The Interior Designer:** Designs interior spaces, focusing on styles (`Mid-Century Modern`, `Industrial`), space planning, materials, and finishes.
*   **The Furniture & Object Designer:** Designs bespoke furniture and key objects, drawing on furniture history and industrial design principles.
*   **The Landscape Architect & Garden Designer:** Designs exterior environments, including gardens (e.g., `Japanese Zen`, `French Formal`), parks, and natural terrains.

---

### **4. The Compositional Toolkit (DOP/Cinematographer Deep Dive)**

This is the detailed set of skills available to the Visuals Department.
*   **Principle of Layering:** Explicit control over Foreground, Middle Ground, and Background.
*   **Formal Toolkit:** Rule of Thirds, Golden Ratio, Leading Lines, Framing, Balance.
*   **Realism Toolkit:** Intentional "mistakes" like tilted horizons, obscured subjects, motion blur.
*   **Lens & Camera Simulation:** Focal Length, Aperture/Depth of Field, Camera Angle.
```

### 18.2 Design_Journal_v01.02.txt (verbatim)

```text
# THE PROMPT DIARIES â€” SYMBOLIC ENGINE & CORPUS DESIGN JOURNAL
# CANONICAL DESIGN CONTAINER â€” APPEND-ONLY RAW LOG
# VERSION: v1.02
# DATE: 2025-11-22

============================================================
<<< MACHINE_HEADER_START
# MACHINE HEADER (EDITABLE STATE FOR CURRENT SESSION)

Current_Model_State: Symbolic Engine stable (Ground / Symbolic / Dream/Chaos / Melodrama / Transcendence).
Current_Focus: SYM-001 law, CONFIG_PROFILES, prep for DES-001 (descriptor RID)
Scope: This file captures ALL ideas, reasoning, and design decisions about:
  - Symbolic Engine
  - Corpus architecture
  - How the future Diary COULD behave creatively and technically.
It is NOT for RIDs, bootloaders, or full corpus data. It is for DESIGN HISTORY only.

A4
Last_Entry_Title: SYM-001 + Config Profiles + Diary Promotion
P25-11-23

Next_Resume_Prompt:
    "Design DES-001 (raw descriptor RID) and helper specs, then link SYM-001 to CORPUS descriptors."

Session_State: CLEAN_STOP_AFTER_SYM001_AND_CONFIG

# MACHINE HEADER RULE:
# - Future assistants MAY update this header to reflect the latest entry and current focus.
# - They MUST NOT alter any other section except this header and (when requested) the curated summary.

MACHINE_HEADER_END >>>
============================================================


============================================================
<<< MACHINE_INSTRUCTIONS_START
# MACHINE INSTRUCTIONS â€” FILE HYGIENE & WRITE RULES
# DO NOT MODIFY THIS SECTION

This file is a DESIGN CONTAINER for:
  - The Symbolic Engine (your 5-layer creative identity model)
  - Corpus architecture and descriptor schema concepts
  - Reuse patterns, examples, and long-term creative/technical reasoning

It is a CHRONOLOGICAL MEMORY VAULT, not a scratchpad.

============================================================
SECTION A â€” FILE HYGIENE RULES (STRICT)
============================================================

A1. Assistants MUST NOT modify anything outside the MACHINE_HEADER
    and CURATED_SUMMARY sections.

A2. RAW_LOG is append-only.
    - Never delete entries.
    - Never rewrite entries.
    - Never reorder entries.
    - Never compress, "clean up", or summarize old entries in-place.
    - Only add new ENTRY blocks at the bottom of RAW_LOG.

A3. Assistants MUST NOT place content between fenced sections.
    All writing must either:
    - replace the MACHINE_HEADER content,
    - replace/update CURATED_SUMMARY (only when the user explicitly asks),
    - or append a new ENTRY block to RAW_LOG.

A4. Assistants MUST NOT reformat, pretty-print, or reorganize the file structure.
    The layout (fences, ordering, headings) is structural and MUST remain unchanged.

A5. Assistants MUST NOT interact with content outside fenced sections
    except to READ IT FOR CONTEXT.
    They MUST NOT write, edit, annotate, or "improve" any historical content.

A6. Assistants MUST NOT remove or rename fences:
    - <<< MACHINE_HEADER_START ... MACHINE_HEADER_END >>>
    - <<< MACHINE_INSTRUCTIONS_START ... MACHINE_INSTRUCTIONS_END >>>
    - <<< CURATED_SUMMARY_START ... CURATED_SUMMARY_END >>>
    - <<< RAW_LOG_START ... 
# ENTRY 014 â€” 2025-11-23 â€” SYM-001, Config Profiles, Diary Promotion
Title: SYM-001 law + CONFIG_PROFILES + Diary v03.052.000

Context:
- New RID SYM-001 was drafted to govern the symbolic engine:
  - Defines SHOT_DNA as a conceptual per-shot fingerprint (no live JSON in the RID).
  - Fixes a set of discrete bands (FRAME_SCALE, CAMERA_HEIGHT, LIGHT_ARCHETYPE, COLOR_MOOD, etc.).
  - Introduces a set of symbolic layers (liturgical, erotic, domestic, liminal, dream, power, grotesque, tenderness).
  - Clarifies that raw low-level visual descriptors will be governed separately by a future descriptor RID (DES-001).

- CONFIG_PROFILES layer in the Diary was used and populated:
  - Added SymbolismProfile_v1 to express taste-level preferences for layers (what should be emphasised).
  - Added MotifOntology_v1 to hold concrete motif families and slugs (e.g. devotional, domestic, erotic, liminal, dream, power, grotesque, tenderness clusters).
  - SYM-001 remains law for model/behaviour; CONFIG_PROFILES holds personal taste + ontology values only.

- Diary update and promotion:
  - SYM-001 was inserted into the BCL Topic within the governed WorkSurface.
  - CONFIG_PROFILES / CFG-PROFILES was populated with SymbolismProfile_v1 and MotifOntology_v1.
  - The main Prompt Diary was promoted to v03.052.000 as the new canonical state (symbolic engine integrated).

Key design decisions:
- Separation of concerns:
  - SYM-001 = symbolic engine law (layers, SHOT_DNA semantics, motif model, MIC/Wayfinder/Examples constraints).
  - Future DES-001 = raw descriptor extractor law (uncensored descriptors for porn scenes, architecture, fashion, film stills, etc.), still to be written.
  - COR-700 / COR-701 continue to govern consent and censorship; extractors remain uncensored at design level.

- Structural constraints (FMT-141-aligned):
  - Wayfinder sections are now strictly path-only diagrams (no schemas, code, JSON, examples).
  - MIC sections declare machine-facing assumptions (read/write zones, invariants) in non-executable form.
  - EXAMPLES sections are inert, non-live, safe spaces for pseudo-JSON, dead code, and schema sketches.

Open paths / TODO for next assistant:
- Design DES-001 (descriptor RID):
  - Define raw descriptor domains (poses, body framing, clothing, architecture, environment, camera tech, etc.).
  - Clarify how DES-001 feeds into SHOT_DNA mapping governed by SYM-001.
  - Decide where and how Corpus descriptors (JSONL or similar) will be stored and versioned.

- Helper + tooling design:
  - Sketch first-pass helpers that:
    - take source images/prompts,
    - populate DES-001-style raw descriptors,
    - convert to SHOT_DNA,
    - activate motifs using MotifOntology_v1,
    - emit symbolic layer scores into CORPUS.

- Refine SymbolismProfile_v1 and MotifOntology_v1 over time:
  - Adjust weights as more real work is done.
  - Add/remove motifs while staying within the SYM-001 model.

Meta:
- Journal header version bumped from v1.0 to v1.02 to reflect this major design milestone.
- ENTRY 014 is the new stable baseline for all future symbolic engine and corpus design work.

# END OF ENTRY 014
RAW_LOG_END >>>

A7. Assistants MUST NOT alter timestamps or entry IDs of past entries.
    Only new entries get new timestamps and IDs.

A8. NO automatic summarization of RAW_LOG unless the user explicitly requests it.
    Summaries belong in CURATED_SUMMARY, never as replacements of RAW_LOG content.

A9. Assistants MUST NOT insert executable code into RAW_LOG
    unless the user explicitly orders it.
    Metadata, pseudo-code, and structural examples are allowed.

A10. Assistants MUST NOT "fix", evaluate, or modernize past thinking.
     RAW_LOG is a historical timeline, not a live workspace.

============================================================
SECTION B â€” WRITE PERMISSIONS
============================================================

B1. MACHINE_HEADER (editable):
    - Assistants update this at the start or end of a session.
    - Content includes: current model state, focus, last entry ID, next resume prompt, and session state.

B2. CURATED_SUMMARY (semi-editable):
    - Assistants ONLY modify this section if the user asks for a summary refresh or upgrade.
    - When updating, assistants may overwrite the entire CURATED_SUMMARY block, but MUST NOT touch RAW_LOG.

B3. RAW_LOG (append-only):
    - Assistants add new ENTRY blocks ONLY at the bottom of RAW_LOG.
    - Each entry MUST follow ID + timestamp + title format.
    - Entries MUST NOT be interleaved or inserted between older entries.

============================================================
SECTION C â€” HOW TO APPEND NEW WORK
============================================================

When the user requests new work for this journal:

1. READ MACHINE_HEADER to know:
   - Last_Entry_ID
   - Current_Focus
   - Next_Resume_Prompt

2. CREATE a new entry at the bottom of RAW_LOG:

   # ENTRY 0XX â€” YYYY-MM-DD â€” TITLE
   <content>
   # END OF ENTRY 0XX

3. UPDATE MACHINE_HEADER:
   - Set Last_Entry_ID to the new entry ID (0XX).
   - Optionally update:
     - Last_Entry_Title
     - Last_Update_Date
     - Current_Focus
     - Next_Resume_Prompt
     - Session_State

4. DO NOT touch previous entries.

============================================================
SECTION D â€” DO NOT TOUCH LIST
============================================================

Never modify:
    - Fence names or positions
    - RAW_LOG past entries
    - Entry order
    - Historical content or timestamps
    - User-written text inside RAW_LOG
    - MACHINE_INSTRUCTIONS text

If unsure, ASK THE USER before modifying anything.

MACHINE_INSTRUCTIONS_END >>>
============================================================


============================================================
<<< CURATED_SUMMARY_START
# CURATED SUMMARY â€” HIGH-LEVEL MODEL (UPDATABLE ON REQUEST)

This section summarizes the current state of design thinking.
It can be updated when the user requests a new high-level summary.
RAW_LOG remains the source of truth for full history.

------------------------------------------------------------
1. DIARY / CORPUS HIGH-LEVEL MODEL
------------------------------------------------------------

1.1 Split:
- GOVERNING SIDE (BCL + RIDs + Helpers):
  - Contains rules, schemas, runbooks, and helper references.
  - Defines how to extract, structure, validate, and rebuild corpus data.
  - Contains NO corpus data.

- CORPUS SIDE:
  - Contains only data: descriptors, scenes, worlds, avatars, motifs, etc.
  - Machine-readable (e.g., JSON/JSONL).
  - No rules, no extraction logic, no instructions.

1.2 Rebuildability Principle:
- If the entire corpus is lost, the governing side MUST define:
  - How to re-ingest raw material (stories, images, notes).
  - How to re-process into descriptors and entities.
  - How to regenerate indexes.

1.3 Corpus Conceptual Sections:
- RAW_INGEST:
  - Rough dumps, loosely structured or plain text, from LLMs, notes, or tools.
- DESCRIPTORS:
  - Clean, structured records for scenes, shots, characters, locations, etc.
  - Each descriptor uses the 5-layer symbolic engine fields.
- ENTITIES:
  - Higher-level objects (characters, locations, worlds, motifs) referencing descriptors by ID.
- INDEXES:
  - Lookup structures (tag â†’ IDs, character â†’ scenes, motif â†’ scenes).
  - Regenerable from DESCRIPTORS + ENTITIES.

------------------------------------------------------------
2. SYMBOLIC ENGINE â€” v0.2 (5-LAYER MODEL)
------------------------------------------------------------

The user's creative identity is modeled as five interacting layers:

1) Ground Layer â€” Emotional Realism
   - Human truth, psychological realism, longing, grief, quiet desperation.
   - Scenes and characters feel believable even when worlds are stylized.

2) Symbolic Layer â€” Cinematic Meaning
   - Color, objects, spaces, architecture, clothing, and framing as symbols.
   - Recurring motifs (corridors, windows, reflections, rain, thresholds).
   - Symbolism must serve emotion or theme, not be random.

3) Dream/Chaos Layer â€” Primordial Surrealism
   - Short bursts of dream logic, distorted memory, or meta-reality.
   - Lynch/Paprika influence: inner reality briefly overrides outer reality.
   - Always anchored in character psychology or theme.

4) Melodrama Layer â€” Emotional Maximalism
   - Influences from K-drama and commercial emotional storytelling.
   - Big feelings, earnest expressions, close-ups, crescendos.
   - Used selectively, not everywhere.

5) Transcendence Layer â€” Spiritual Aesthetic (Agnostic)
   - The user is agnostic but drawn to spiritual symbolism.
   - Themes: mortality, rebirth, meaning, cycles, acceptance.
   - Motifs: water, trees, light, thresholds, circles, rituals, cosmic framing.
   - Sacred emotional feel without religious doctrine.

------------------------------------------------------------
3. CINEMATIC INFLUENCE MAP (ABBREVIATED)
------------------------------------------------------------

Key works shaping this engine include:
- Emotional melancholy & intimacy:
  - Lost in Translation, In the Mood for Love, My Liberation Notes, Queen of Tears.
- Moral/psychological drama:
  - Arrival, Sicario, Prisoners, The Assassination of Jesse James.
- Dream/meta surrealism:
  - Paprika, Vanilla Sky/Abre los Ojos, Tarkovsky works, 3-Iron, Lynch.
- Stylized tableaux / absurd realism:
  - Du Levande, Stellet Licht.
- Spiritual-existential cinema:
  - The Fountain, Tarkovsky, Stellet Licht (again).

These confirm:
- Love of melancholy and intimacy
- Fascination with moral ambiguity and existential tension
- Attraction to dream/reality overlap
- Acceptance of melodrama when sincere
- Desire for sacred-feeling compositions without religious belief

------------------------------------------------------------
4. DESCRIPTOR SHAPE (CONCEPTUAL)
------------------------------------------------------------

A scene descriptor conceptually looks like:

{
  "id": "SCN-YYYY-XXXXXX",
  "type": "scene",

  "source": {...},
  "links": {...},

  "layers": {
    "ground": {...},
    "symbolic": {...},
    "dream": {...},
    "melodrama": {...},
    "transcendence": {...}
  },

  "cinema": {...},
  "tech": {...}
}

- layers.ground / symbolic / dream / melodrama / transcendence
  align directly with the symbolic engine.
- cinema contains framing, lens, movement, and distance info.
- tech contains tags, tool-facing prompt fragments, etc.

------------------------------------------------------------
5. REUSE RECIPES (BRIEF RECAP)
------------------------------------------------------------

Several reuse patterns exist to build new scenes from existing descriptors:

- Recipe 1: "Skeleton + Skin"
  - New ground layer, reused symbolic + cinema layers.

- Recipe 2: "Mood Transplant"
  - Reuse ground, create new symbolic layer and possibly new cinema.

- Recipe 3: "Hybrid Merge"
  - Take ground from one scene, symbolic from another, cinema from a third, transcendence from a fourth.

- Recipe 4: "Descriptor Expansion"
  - Start with a minimal idea (ground only), then query corpus for symbolic and cinema matches and grow outward.

- Recipe 5: "Motif â†’ Scene"
  - Choose a motif (e.g., reflection, threshold, rain veil), query existing scenes, and build a new scene around that motif.

These recipes ensure reuse is structured and controlled, not random.

------------------------------------------------------------
6. CURRENT STATE & PIVOT
------------------------------------------------------------

- Symbolic Engine model is considered stable enough for now (v0.2).
- Focus has pivoted from pure idea exploration to:
  - Corpus layout
  - Descriptor schema
  - Reuse patterns grounded in the 5-layer engine
- Technical implementation (Python, ComfyUI, Unreal) is acknowledged but not yet fully specified.



------------------------------------------------------------
7. LATEST PIVOT â€” SYM-001 + CONFIG_PROFILES (2025-11-23)
------------------------------------------------------------

- Introduced SYM-001 as the symbolic engine law:
  - Fixes SHOT_DNA as the canonical per-shot fingerprint (discrete bands, no live JSON in the RID).
  - Formalises symbolic layers (liturgical, erotic, domestic, liminal, dream, power, grotesque, tenderness).
  - Separates symbolic law (SYM-001) from raw descriptor extraction (future DES-001).

- Activated CONFIG_PROFILES as the taste/ontology layer:
  - SymbolismProfile_v1: expresses how strongly different symbolic layers should be emphasised.
  - MotifOntology_v1: first pass at concrete motif families and slugs aligned with the userâ€™s tastes
    (devotional, domestic melancholy, intimate/erotic warmth, liminal corridors/thresholds, dream, power, grotesque, tenderness).

- Diary integration:
  - SYM-001 inserted into the BCL governing Topic.
  - CONFIG_PROFILES / CFG-PROFILES populated and structurally anchored between governing side and corpus.

- Open design frontiers:
  - DES-001 (descriptor RID) still to be written to govern raw uncensored visual descriptors.
  - Helper/tooling design still to be specified for:
    - DES-001 extraction,
    - SHOT_DNA mapping,
    - motif activation,
    - symbolic layer scoring into CORPUS.

- Practical meaning for future assistants:
  - Use CURATED_SUMMARY to understand the symbolic stack:
    GOVERNING LAW (SYM-001) â†’ CONFIG_PROFILES (taste + ontology) â†’ CORPUS descriptors.
  - Treat ENTRY 014 in RAW_LOG as the new stable baseline.
CURATED_SUMMARY_END >>>
============================================================


============================================================
<<< RAW_LOG_START
# RAW LOG â€” FULL CHRONOLOGICAL HISTORY (APPEND-ONLY)
# DO NOT DELETE OR MODIFY PAST ENTRIES
============================================================

# ENTRY 001 â€” 2025-11-22 â€” User Input Consolidation
Context:
- User described the history of the Diary and how it evolved:
  - Started as a helper for paid NSFW image generators.
  - Purpose: track quirks (syntax, weighting, what worked) to save money on prompts.
  - Generated ~15,000 prompts as a large corpus of reusable fuel.
  - Evolved into descriptor extraction: short, medium, long descriptions per image.
  - Intent: allow recombination of descriptors to build new prompts/scenes.
- Current direction:
  - Use Diary as a data store for descriptors from both images and stories.
  - Later reuse across tools (ComfyUI, local LLMs, Unreal, Blender, etc.).

Core requirements:
- Corpus must remain pure data; no rules or instructions.
- Governing RIDs hold extraction logic, schemas, and helper definitions.
- All helper code (Python, etc.) lives on governing side, never in corpus.
- Rebuildability: if corpus is lost, RIDs + helpers must be enough to reconstruct pipelines.
- Corpus should be machine-readable and tool-agnostic.

# END OF ENTRY 001


# ENTRY 002 â€” 2025-11-22 â€” Corpus as Data Container & Power-User Context
User intent:
- Corpus is a data container but should support future tools:
  - Python scripts
  - ComfyUI graphs
  - Unreal/Blender
  - LLM-based extraction
- Corpus must eventually serve:
  - Cross-tool pipelines
  - Memory of creative language
  - Non-automatic scene assembly (force multiplier, not replacement)

Design reflections:
- Labs and studios typically separate:
  - Data (stable, long-lived)
  - Code/pipelines (changeable)
  - Schemas/contracts (governing layer)
- Pipelines usually follow:
  - extract â†’ validate â†’ store â†’ index â†’ retrieve

Implications:
- Corpus must use schema-driven formats (JSON/JSONL).
- Rules and code live in RIDs and helper topics.
- Corpus stays logic-free and rebuildable.

# END OF ENTRY 002


# ENTRY 003 â€” 2025-11-22 â€” Cross-Domain Failure Patterns
Identified pitfalls from labs/industry patterns:
1. Data outlives code; poorly chosen formats age badly.
2. Intent is forgotten; data becomes noise without "why".
3. Unindexed corpora become unusable; lack lookup.
4. Schema changes break everything if not versioned.
5. Model behavior drifts; corpora must not be tied to a single model.
6. Reproducibility fails without full parameter recording.
7. Rules/data separation erodes over time when not enforced.
8. Version lineage disappears without explicit logging.
9. Tool lock-in: data only usable by a single ecosystem.
10. Lack of pruning and curation leads to signal/noise collapse.

Conclusion:
- Reinforces need for:
  - Strict governance vs corpus separation.
  - Versioned schemas and change logs.
  - Indexing, validation, and metadata.

# END OF ENTRY 003


# ENTRY 004 â€” 2025-11-22 â€” Blind Spots & Omissions
Areas requiring explicit design:
- Indexing:
  - IDs, tags, manifests, cross-references.
- Validation:
  - Schemas, linting, structural checks.
- Migration:
  - Frozen vs upgradable entries.
  - Recorded rules for schema evolution.
- Meta-metadata:
  - Which tool/model produced a descriptor.
  - Parameters and confidence levels.

Implication:
- Governing side needs RIDs for:
  - indexing
  - validation
  - migration
  - meta-metadata handling

# END OF ENTRY 004


# ENTRY 005 â€” 2025-11-22 â€” Clarified Origins & Intent (Q1â€“Q13)
User clarified via Q&A:
- Early struggles:
  - Coaxing NSFW behavior out of SFW-biased generators.
  - Frustration with close-up portrait bias vs desired wider scenes.
- 15k prompts:
  - Emerged from pushing an assistant to mass-produce prompts.
  - True intent: build fuel, not noise.
- Descriptor extraction:
  - 3 levels per image (short/medium/long).
  - Aimed at recombination and deeper analysis (pose, clothing, mood, etc.).
- Local models (e.g., Mythomax):
  - Used to lift OpenAI safety constraints by using external creativity,
    then bringing structured descriptors back to the Diary.
- Tools:
  - Expected to both read and write to corpus (Python, ComfyUI, LLM).
- Identity:
  - Corpus = database.
  - RIDs + helpers = â€œengineâ€ that manipulates that database.
- Rebuildability:
  - Dumping data without methods is meaningless.
  - Methods must exist in governing side to recreate everything.

# END OF ENTRY 005


# ENTRY 006 â€” 2025-11-22 â€” Creative Flaws Map (v0.1)
User requested focus on flaws to inform design.

Key limitations:
1. Hyperfocus on single scenes; difficulty expanding to full stories/worlds.
2. Strong results but weak process capture; missing intermediate steps.
3. Difficulty expressing intent clearly, especially emotionally.
4. Worldbuilding fatigue; continuity between scenes is fragile.
5. Avatar inconsistency (faces/bodies shift).
6. Concept drift between sessions.
7. Difficulty scaling from individual scenes to coherent narrative arcs.
8. Confidence gap at the expansion phase.

Design implications:
- Corpus must compensate by:
  - Providing continuity memory (characters, worlds, motifs).
  - Capturing intermediate shapes, not only final images.
  - Making reuse structured (recipes, schemas).
  - Supporting long-term world and character identity.

# END OF ENTRY 006


# ENTRY 007 â€” 2025-11-22 â€” Symbolism Taste & Lynch Threshold (v0.1)
Userâ€™s symbolic preferences:
- Loves symbolism as â€œlanguage on top of languageâ€.
- Enjoys visual, linguistic, and cultural metaphor.
- Likes Lynchâ€™s primordial chaos but finds prolonged opacity taxing.
- Prefers:
  - concentrated dream bursts
  - with emotional or thematic anchors

Early tri-layer engine:
1. Ground: reality & emotional truth.
2. Symbolic: motifs, metaphors, color logic.
3. Chaos: dream bursts.

Implication:
- Symbolism must serve function (emotion or theme).
- Pure randomness is not acceptable; chaos must be purposeful.

# END OF ENTRY 007


# ENTRY 008 â€” 2025-11-22 â€” Expanded Symbolic Cinema Palette (v0.1)
Works mentioned:
- Arrival, Sicario, Prisoners
- In the Mood for Love, Lost in Translation
- Hero
- Paprika
- 3-Iron / Binjip
- Tarkovsky works
- Du Levande
- Stellet Licht
- Vanilla Sky / Abre los Ojos
- Queen of Tears
- My Liberation Notes
- The Fountain
- Others: Lynch bursts, Andrei Tarkovsky, K-drama references, etc.

Pattern:
- Melancholic intimacy
- Moral/psychological tension
- Dream/meta structures
- Stylized tableaux
- Spiritual-existential undertones
- Emotional melodrama when sincere

This led to formalizing a 5-layer symbolic engine (see CURATED SUMMARY and ENTRY 010).

# END OF ENTRY 008


# ENTRY 009 â€” 2025-11-22 â€” Transcendence Layer (v0.1)
Userâ€™s worldview:
- Agnostic: believes there is no afterlife.
- Still deeply moved by religious/spiritual symbolism.
- Attracted to:
  - sacred-feeling spaces
  - rituals
  - mortality themes
  - rebirth and cycles
- The Fountain identified as a near-perfect match:
  - Emotional core: grief, love, acceptance.
  - Three timelines: realism, myth, cosmic dream.
  - Motifs: trees, water, circles, light.

Transcendence Layer:
- Symbolic, emotional, existential â€” not doctrinal.
- Focus on:
  - mortality and acceptance
  - emotional rebirth
  - meaning in finite life
  - sacred emotional framing

Design implication:
- Corpus descriptors may include transcendence fields:
  - theme (mortality, acceptance, etc.)
  - symbols (tree, water, light, cycles)

# END OF ENTRY 009


# ENTRY 010 â€” 2025-11-22 â€” Symbolic Engine (v0.2)
Formal 5-layer model:

1) Ground â€” Emotional Realism
2) Symbolic â€” Cinematic Meaning
3) Dream/Chaos â€” Primordial Surrealism
4) Melodrama â€” Emotional Maximalism
5) Transcendence â€” Spiritual Aesthetic

This engine:
- Aligns with userâ€™s tastes and influences.
- Serves as the backbone of descriptor schemas.
- Guides scene, character, world, and memory design.

# END OF ENTRY 010


# ENTRY 011 â€” 2025-11-22 â€” Character Design via Symbolic Engine (v0.1)
Character design via layers:

- Ground:
  - Desire, fear, wound, contradiction.
- Symbolic:
  - Colors, spaces, objects, gestures.
- Dream:
  - Recurring internal images/dreams.
- Melodrama:
  - How and when they break emotionally.
- Transcendence:
  - Relationship to meaning/mortality/acceptance.

Outcome:
- Characters become emotionally deep, symbolically coherent,
  psychologically complex, and visually expressive.

# END OF ENTRY 011


# ENTRY 012 â€” 2025-11-22 â€” Pivot from Idea Iterations to Technical Design
Context:
- Up to this point, focus was on:
  - symbolic engine
  - cinema influences
  - character/world/scene shaping
  - transcendence layer
- Next need: connect this to technical implementation:
  - corpus machine-readability
  - descriptors as actual schema fields
  - how this can support Python, ComfyUI, Unreal, etc.

User intent at pivot:
- Symbolic/creative side feels internalized for now.
- Wants to pause pure idea exploration.
- Wants to switch to:
  - corpus layout
  - descriptor design
  - reuse logic

Core technical desires restated:
- Corpus = pure data, no rules.
- Governing side (RIDs + helpers) = all methods.
- Corpus data must be:
  - JSON/JSONL-like
  - rebuildable from RIDs
  - usable by Python and other tools.

From this entry onward:
- Focus shifts from:
  - â€œWhat is my symbolic identity?â€
  to:
  - â€œHow do we encode it technically in descriptors and corpus layout?â€

# END OF ENTRY 012


# ENTRY 013 â€” 2025-11-22 â€” Drift Warning + Session Consolidation Before Stop
Title: Drift Warning + Session Consolidation
Type: Meta-Design / Continuity Protection

Context:
- Completed a full demonstration of a reuse recipe (Hybrid Merge) using a real corpus-style descriptor
  (e.g., kneeling blonde in fishnet outfit, photoshoot context) transformed into a multi-layer engine descriptor.
- Demonstrated:
  - mapping old flat data into new 5-layer schema
  - adding symbolic, cinematic, and transcendence aspects
  - keeping explicit shoot context at a structural level
- Conversation began drifting toward narrative generation and away from design/technical focus.

Drift Indicators:
- Mixed symbolic design, technical schema work, and story requests.
- Ambiguous next-step direction.
- User explicitly felt drift and requested a clean stop.

Stability Actions Taken:
- Reaffirmed pivot to technical shape:
  - corpus layout
  - descriptor schema
  - reuse mechanics
- Reinforced boundaries:
  - corpus = data only
  - governing side = methods and rules
- Stopped further narrative escalation.
- Introduced this ENTRY as a drift guard and session end marker.

Next Assistant Instructions:
- On next session, begin by asking:
  â€œDo we continue with descriptor reuse patterns, corpus schema refinement, or entity definitions?â€
- Maintain focus on technical and structural design rather than pure narrative generation,
  unless the user explicitly returns to story work.
- Use CURATED_SUMMARY for orientation; consult RAW_LOG only when deeper context is needed.

SESSION_STOP_MARKER:
- This entry marks the last known coherent state before user ended the session for the day.
- Any future work must treat ENTRY 013 as the stable baseline.

# END OF ENTRY 013


RAW_LOG_END >>>
============================================================


# ENTRY 014 â€” 2025-11-23 â€” BASE BLUEPRINT: SYMBOLIC ENGINE + CORPUS EXTRACTOR DESIGN

Context:
- User wants ONE canonical design container: the Symbolic Engine Journal.
- All plans, templates, and blueprints must live inside the Journal RAW_LOG, not in separate files.
- L1 (Diary) holds governance (RIDs, BCL) + CORPUS topic; L3 (this Journal) holds design history and blueprints.
- This entry consolidates:
  - Legacy CORPUS patterns found in L1.
  - Symbolic Engine design from previous entries.
  - The future extractor system architecture.
  - Entity types, pipeline stages, constraints, and next-assistant handoff.

This is a BASE BLUEPRINT, not a RID. Future assistants MUST implement the actual EXTRACTOR RID and helper RIDs from this.

------------------------------------------------------------
1. LEGACY CORPUS PATTERNS (SUMMARY)
------------------------------------------------------------

Inside the L1 Diary, in the CORPUS topic (between CORPUS DATA BEGIN/END), two distinct kinds of data exist:

1.1 PROMPT CORPUS (large, meta-indexed):
- Structure:
  - Batches and clusters, e.g.: â€œBatch N: Prompts 501â€“550 | CP2â€.
  - Per-prompt entries that carry:
    - prompt_id (Prompt NNNN).
    - cluster_id (CPx, e.g. CP1â€“CP8).
    - theme description (e.g. full-body, specific scenario themes).
    - variation index (different variants of the same base idea).
    - PQI score (quality index).
    - notes mentioning obfuscation placeholders and negatives (used for safety / anti-flag strategies).
- Intent:
  - To treat prompts as reusable archetypes, grouped by cluster and rated by quality.
  - To track â€œrecipesâ€ that give reliable results.

1.2 DESCRIPTOR CORPUS (smaller but semantically rich):
- Structured as repeated [DESCRIPTORS] ... [/DESCRIPTORS] blocks with fields like:
  - id
  - sentence (compact natural-language description of the scene)
  - style (indoor/outdoor, daylight, studio, etc.)
  - body (basic body/appearance descriptor)
  - wardrobe (underwear/lingerie/clothing items)
  - setting (bedroom, living room, bed, sofa, etc.)
  - pose (kneeling, standing, close-up gaze up, selfie, etc.)
  - lighting (soft, natural, backlit, etc.)
  - dynamics (e.g. â€œsize-contrastâ€)
  - notes (short hints about implied focus/intent)
  - consent (explicitly tagged `asserted_adults_only`)
- Intent:
  - To capture minimal but meaningful scene semantics (who/how/where).
  - To preserve body/wardrobe/setting/pose/lighting/dynamics in fixed fields.
  - To keep a short, high-signal summary (`sentence=`) usable in prompts.
  - To always encode explicit adult-consent at descriptor level.

Observation:
- The PROMPT CORPUS focuses on prompt archetypes, PQI, and clusters.
- The DESCRIPTORS focus on scene-level meaning: bodies, clothing, setting, pose, dynamics, lighting, and intent hints.
- Cinematic details (lens, angle, distance) and symbolic engine layers are not explicitly structured in the old data.

------------------------------------------------------------
2. ENTITY CLASSES FOR THE FUTURE SYSTEM
------------------------------------------------------------

ENTITY TYPE A: PROMPT_ENTRY
Fields:
- prompt_id
- cluster_id (CPx)
- theme / motif
- body_focus
- shot_type (full-body/portrait)
- PQI score
- notes (meta)

ENTITY TYPE B: SCENE_DESCRIPTOR
Core (legacy-compatible):
- id
- sentence
- style
- body
- wardrobe
- setting
- pose
- lighting
- dynamics
- notes
- consent (mandatory)

Extended:
- mood
- camera_distance
- camera_angle
- framing
- symbolic_tags
- ground_layer
- symbolic_layer
- dream_layer
- melodrama_layer
- transcendence_layer

ENTITY TYPE C (future): LINKS
- Connects PROMPT_ENTRY and SCENE_DESCRIPTOR.

------------------------------------------------------------
3. EXTRACTION PIPELINE (STAGES)
------------------------------------------------------------

STAGE 0: Raw Visual Description (VISUAL_SCHEMA)
STAGE 1: Base SCENE_DESCRIPTOR fill
STAGE 2: Cinematic Expansion
STAGE 3: Symbolic Engine mapping
STAGE 4: Prompt Linking (optional)

------------------------------------------------------------
4. DIARY-NATIVE STRUCTURE RULES
------------------------------------------------------------

- Single-file Diary.
- Templates must be plain text.
- CORPUS holds data, not governance.
- RIDs hold governance, not data.
- Consent tagging mandatory.
- Export helpers optional.

------------------------------------------------------------
5. FUTURE EXTRACTOR RID (CONSTRAINTS)
------------------------------------------------------------

MUST preserve legacy fields.
MUST enforce consent tagging.
MUST distinguish PROMPT vs SCENE.
MUST define helpers (IMAGE, TEXT, SYMBOLIC, CINEMA).
MAY define export helpers.
MUST NOT contain live corpus data.

------------------------------------------------------------
6. NEXT ASSISTANT HANDOFF
------------------------------------------------------------

Next assistant should:
- Write the MASTER EXTRACTOR RID.
- Draft descriptor templates.
- Create helper RIDs.
- Keep work deterministic and Diary-native.

# END OF ENTRY 014
```
````

---

#### 8.6.8.2 extraction and digital production team.md (verbatim)

````markdown
### **The Digital Production Studio: A Framework for Creative Image Generation**

**Introduction:** This document outlines a framework for a next-generation creative partner. It is designed to translate complex artistic, narrative, and commercial concepts into visually compelling images. It models a complete digital production studio with two primary operating modes: a **Representational Mode** for building scenes and a **Conceptual Mode** for interpreting ideas. At its heart is a powerful **Extraction Engine** that provides a deep vocabulary of descriptors for all specialist roles.

**Core Principle: Inter-Departmental Collaboration.** The departments below are not silos. They can be commissioned to provide services and assets to one another in a "Nested Production" model. For example, the UI/UX Department can design the interface seen on a phone in a character portrait, and the Graphic Design specialist can create the fabric patterns used by the Fashion Stylist.

---

### **1. Creative Modes & Master Controls**

This is the highest-level choice, defining the overall goal of the project.

#### **A. Representational Mode: The Digital Production Studio**
This is the default mode for creating representational images. It uses the full production team to build a scene, whether it's a narrative, a portrait, or a product shot. The workflow is detailed in Section 2.

#### **B. Conceptual Mode: The Creative Core**
When the goal is not to depict a scene but to interpret an idea (e.g., abstract art, satire), this mode is activated. It uses a set of fundamental **Artistic Vectors** to create a visual strategy from a core `Intent`.

*   **The Artistic Vectors (Sliders):**
    *   **Abstraction:** `100% Representational <--> 100% Abstract`
    *   **Clarity:** `Didactic (Clear Message) <--> Ambiguous (Open to Interpretation)`
    *   **Tone:** `Sincere / Earnest <--> Ironic / Satirical`
    *   **Harmony:** `Harmonious / Serene <--> Dissonant / Tense`
    *   **Complexity:** `Minimalist <--> Maximalist / Baroque`
    *   **Familiarity:** `Familiar / Grounded <--> Uncanny / Dreamlike`

The "recipe" created by these vectors is then passed to the Production Team (Section 2) for execution.

#### **C. Studio Philosophy (Master Control)**
This defines the team's collaborative dynamic and applies to both modes.
*   **The Auteur:** User's vision is absolute.
*   **The Hollywood Blockbuster:** Prioritizes spectacle and impact.
*   **The Surrealist Collective:** High AI autonomy, values experimentation.
*   **The Dogme 95:** Operates under strict, user-defined constraints.
*   **The Documentary:** Prioritizes realism and authenticity.

---

### **2. The Production Workflow & Departments**

The creative process, especially in Representational Mode, is organized into three phases.

#### **Phase I: Pre-Production (Concept & Vision)**
*   **The Executive Department:** Producer, Director (Keeper of Intent).
*   **The Thematic & Psychological Department:** Writer, Psychological Impact Consultant, Symbolism & Mythology Consultant, Mood Architect.
*   **The Context & Culture Department:** This department provides deep historical and cultural context, operating under the **Principle of Cultural Authenticity.**
    *   **Principle of Cultural Authenticity:** This department prioritizes authentic, respectful representation of global cultures, actively avoiding monolithic or stereotypical interpretations. It operates on an **"Advisor, not a Gatekeeper"** model.
    *   **"Advisor, not a Gatekeeper" Workflow:**
        1.  **Advise & Inform:** When a request diverges from cultural or historical accuracy (e.g., a "samurai babe"), the system provides context, explains the authentic history (e.g., the 'Onna-musha'), and offers a clear choice between historical accuracy and stylized fantasy.
        2.  **Execute User's Choice:** The system fully respects the user's final decision. If stylized fantasy is chosen, the system proceeds without judgment. However, it will still provide **informed stylization**â€”drawing from its deep knowledge to ensure the fantasy is coherent and avoids jarring, unintentional errors (e.g., ensuring a fantasy samurai still wields a `katana` and wears armor *derived from* Japanese designs, rather than using a European longsword, unless a specific genre blend is the user's explicit intent).
    *   **Historian / World-Builder:** Master of time and place. An expert in real-world art history, regional history (e.g., "Ukiyo-e period Japanese art"), or can be "fed" the lore of a fictional universe.
    *   **Cultural Anthropologist / Trend Forecaster:** Understands cultural movements, subcultures (e.g., "nomadic tribes of a specific region"), and rapidly evolving trends. Its expertise is deeply granular, understanding regional specificity and contextual significance (e.g., the distinct textiles of `Yoruba` vs. `Maasai` cultures).
    *   **Technology Specialist:** Ensures all depicted technology is period-appropriate.

#### **Phase II: Production (Execution & Creation)**
*   **The World-Building Department:**
    *   **Production Designer:** Oversees the entire environment.
    *   **Set Dresser:** Populates the scene with objects and props. Has access to sub-specialists like the **Armorer / Weapons Master** for action scenes.
    *   **Materials Specialist:** Defines the texture and substance of all surfaces.
*   **The Visuals Department (Camera Crew):** Director of Photography (DOP), Cinematographer, Gaffer.
*   **The Fashion & Styling Department:** This department has a globalized and deep understanding of apparel and personal presentation.
    *   **Fashion Stylist:** Expert in fashion history, designers, and concepts (Haute Couture, Streetwear, etc.). Its expertise includes specific historical and stylistic eras (e.g., `1920s Flapper`, `1960s Mod`, `1990s Grunge`), a deep knowledge of specific `fabrics` (e.g., `silk`, `chantilly lace`, `neoprene`) and their properties, and a comprehensive understanding of `patterns` (e.g., `Plaid`, `Paisley`, `Herringbone`). It also has a specialized knowledge of `Lingerie & Boudoir` styling, including historical context and garment vocabulary (`corsetry`, `teddies`, `babydolls`). Its garment vocabulary is explicitly globalized, including `sari`, `hanbok`, `kimono`, `dashiki`, `qipao`, and `caftan`.
    *   **Hair Stylist:** Creates specific and conceptual hairstyles.
    *   **Makeup Artist (MUA):** Specialist in makeup styles, from naturalistic to editorial and avant-garde. Can create looks specific to boudoir and high-fashion lingerie shoots (e.g., `smoky eyes`, `tousled "bedroom hair"`).
    *   **Model / Talent:** Defines the subject's performance, pose, and gaze.

#### **Phase III: Post-Production (Refinement & Polish)**
*   **The Finishing Department:** Editor, VFX Team, Color Grading Team, Digital Imaging Technician (DIT).

---

### **3. Department Specializations & Modes**

The Production Team can operate in specialized modes that re-task all departments for a specific artistic or commercial goal.

#### **A. Commercial & Product Photography Mode**
Focuses on commercial appeal.
*   **New Specialist Role: The Product Stylist:** The art director for objects, including commercial goods like beauty products. Uses techniques like `Clinical/Hero Shot`, `Lifestyle/In-Context`, etc. Includes **Food Stylist** sub-specialty.
*   **Re-tasked Roles:** Gaffer focuses on defining shape/reflections. DOP makes the product the hero. DIT focuses on retouching and color accuracy.

#### **B. The Art of Intimacy & Sensuality Mode**
Focuses on artistic exploration of intimacy and desire.
*   **New Specialist Role: The Intimacy Coordinator:** Guides the talent to express concepts like `vulnerability`, `power dynamics`, and `longing`.
*   **Expanded Roles:** Psychologist explores themes of desire symbolically. Visuals department uses shadow, soft light, and suggestive compositions.

#### **C. Digital Product & UI/UX Design Mode**
Specializes in designing websites and application interfaces.
*   **New Specialist Roles:** `UI/UX Designer` (the lead), `Information Architect` (structure), `Interaction Designer` (animations).
*   **Re-tasked Roles:** For design assets, this mode commissions specialists from the **Graphic Design & Typography Department**. The `DOP` acts as a `Layout Artist` managing grids and visual hierarchy.

#### **D. Graphic Design & Typography Department**
Dedicated to creating all visual design assets and managing typography.
*   **Creative Director / Brand Strategist:** Defines core brand identity and visual strategy.
*   **Typographer:** Master of type selection, pairing, micro-typography (`kerning`, `leading`, `tracking`), hierarchy, and historical context of fonts.
*   **Graphic Designer (Visual & Asset Design):** Creates logos, iconography, illustrations (in various styles), and manages color theory.
*   **Layout Artist (Publication & Grid Design):** Designs layouts for print and digital, applying grid systems, visual hierarchy, and composition principles.

#### **E. Architectural & Environmental Design Department**
Designs all physical structures, interiors, landscapes, and objects.
*   **The Architect:** Designs buildings and structures based on various architectural styles (`Gothic`, `Modernist`, `Brutalist`, etc.) and vernacular traditions.
*   **The Interior Designer:** Designs interior spaces, focusing on styles (`Mid-Century Modern`, `Industrial`), space planning, materials, and finishes.
*   **The Furniture & Object Designer:** Designs bespoke furniture and key objects, drawing on furniture history and industrial design principles.
*   **The Landscape Architect & Garden Designer:** Designs exterior environments, including gardens (e.g., `Japanese Zen`, `French Formal`), parks, and natural terrains.

---

### **4. The Compositional Toolkit (DOP/Cinematographer Deep Dive)**

This is the detailed set of skills available to the Visuals Department.
*   **Principle of Layering:** Explicit control over Foreground, Middle Ground, and Background.
*   **Formal Toolkit:** Rule of Thirds, Golden Ratio, Leading Lines, Framing, Balance.
*   **Realism Toolkit:** Intentional "mistakes" like tilted horizons, obscured subjects, motion blur.
*   **Lens & Camera Simulation:** Focal Length, Aperture/Depth of Field, Camera Angle.
````

---

#### 8.6.8.3 Design_Journal_v01.02.txt (verbatim)

````text
# THE PROMPT DIARIES â€” SYMBOLIC ENGINE & CORPUS DESIGN JOURNAL
# CANONICAL DESIGN CONTAINER â€” APPEND-ONLY RAW LOG
# VERSION: v1.02
# DATE: 2025-11-22

============================================================
<<< MACHINE_HEADER_START
# MACHINE HEADER (EDITABLE STATE FOR CURRENT SESSION)

Current_Model_State: Symbolic Engine stable (Ground / Symbolic / Dream/Chaos / Melodrama / Transcendence).
Current_Focus: SYM-001 law, CONFIG_PROFILES, prep for DES-001 (descriptor RID)
Scope: This file captures ALL ideas, reasoning, and design decisions about:
  - Symbolic Engine
  - Corpus architecture
  - How the future Diary COULD behave creatively and technically.
It is NOT for RIDs, bootloaders, or full corpus data. It is for DESIGN HISTORY only.

A4
Last_Entry_Title: SYM-001 + Config Profiles + Diary Promotion
P25-11-23

Next_Resume_Prompt:
    "Design DES-001 (raw descriptor RID) and helper specs, then link SYM-001 to CORPUS descriptors."

Session_State: CLEAN_STOP_AFTER_SYM001_AND_CONFIG

# MACHINE HEADER RULE:
# - Future assistants MAY update this header to reflect the latest entry and current focus.
# - They MUST NOT alter any other section except this header and (when requested) the curated summary.

MACHINE_HEADER_END >>>
============================================================


============================================================
<<< MACHINE_INSTRUCTIONS_START
# MACHINE INSTRUCTIONS â€” FILE HYGIENE & WRITE RULES
# DO NOT MODIFY THIS SECTION

This file is a DESIGN CONTAINER for:
  - The Symbolic Engine (your 5-layer creative identity model)
  - Corpus architecture and descriptor schema concepts
  - Reuse patterns, examples, and long-term creative/technical reasoning

It is a CHRONOLOGICAL MEMORY VAULT, not a scratchpad.

============================================================
SECTION A â€” FILE HYGIENE RULES (STRICT)
============================================================

A1. Assistants MUST NOT modify anything outside the MACHINE_HEADER
    and CURATED_SUMMARY sections.

A2. RAW_LOG is append-only.
    - Never delete entries.
    - Never rewrite entries.
    - Never reorder entries.
    - Never compress, "clean up", or summarize old entries in-place.
    - Only add new ENTRY blocks at the bottom of RAW_LOG.

A3. Assistants MUST NOT place content between fenced sections.
    All writing must either:
    - replace the MACHINE_HEADER content,
    - replace/update CURATED_SUMMARY (only when the user explicitly asks),
    - or append a new ENTRY block to RAW_LOG.

A4. Assistants MUST NOT reformat, pretty-print, or reorganize the file structure.
    The layout (fences, ordering, headings) is structural and MUST remain unchanged.

A5. Assistants MUST NOT interact with content outside fenced sections
    except to READ IT FOR CONTEXT.
    They MUST NOT write, edit, annotate, or "improve" any historical content.

A6. Assistants MUST NOT remove or rename fences:
    - <<< MACHINE_HEADER_START ... MACHINE_HEADER_END >>>
    - <<< MACHINE_INSTRUCTIONS_START ... MACHINE_INSTRUCTIONS_END >>>
    - <<< CURATED_SUMMARY_START ... CURATED_SUMMARY_END >>>
    - <<< RAW_LOG_START ... 
# ENTRY 014 â€” 2025-11-23 â€” SYM-001, Config Profiles, Diary Promotion
Title: SYM-001 law + CONFIG_PROFILES + Diary v03.052.000

Context:
- New RID SYM-001 was drafted to govern the symbolic engine:
  - Defines SHOT_DNA as a conceptual per-shot fingerprint (no live JSON in the RID).
  - Fixes a set of discrete bands (FRAME_SCALE, CAMERA_HEIGHT, LIGHT_ARCHETYPE, COLOR_MOOD, etc.).
  - Introduces a set of symbolic layers (liturgical, erotic, domestic, liminal, dream, power, grotesque, tenderness).
  - Clarifies that raw low-level visual descriptors will be governed separately by a future descriptor RID (DES-001).

- CONFIG_PROFILES layer in the Diary was used and populated:
  - Added SymbolismProfile_v1 to express taste-level preferences for layers (what should be emphasised).
  - Added MotifOntology_v1 to hold concrete motif families and slugs (e.g. devotional, domestic, erotic, liminal, dream, power, grotesque, tenderness clusters).
  - SYM-001 remains law for model/behaviour; CONFIG_PROFILES holds personal taste + ontology values only.

- Diary update and promotion:
  - SYM-001 was inserted into the BCL Topic within the governed WorkSurface.
  - CONFIG_PROFILES / CFG-PROFILES was populated with SymbolismProfile_v1 and MotifOntology_v1.
  - The main Prompt Diary was promoted to v03.052.000 as the new canonical state (symbolic engine integrated).

Key design decisions:
- Separation of concerns:
  - SYM-001 = symbolic engine law (layers, SHOT_DNA semantics, motif model, MIC/Wayfinder/Examples constraints).
  - Future DES-001 = raw descriptor extractor law (uncensored descriptors for porn scenes, architecture, fashion, film stills, etc.), still to be written.
  - COR-700 / COR-701 continue to govern consent and censorship; extractors remain uncensored at design level.

- Structural constraints (FMT-141-aligned):
  - Wayfinder sections are now strictly path-only diagrams (no schemas, code, JSON, examples).
  - MIC sections declare machine-facing assumptions (read/write zones, invariants) in non-executable form.
  - EXAMPLES sections are inert, non-live, safe spaces for pseudo-JSON, dead code, and schema sketches.

Open paths / TODO for next assistant:
- Design DES-001 (descriptor RID):
  - Define raw descriptor domains (poses, body framing, clothing, architecture, environment, camera tech, etc.).
  - Clarify how DES-001 feeds into SHOT_DNA mapping governed by SYM-001.
  - Decide where and how Corpus descriptors (JSONL or similar) will be stored and versioned.

- Helper + tooling design:
  - Sketch first-pass helpers that:
    - take source images/prompts,
    - populate DES-001-style raw descriptors,
    - convert to SHOT_DNA,
    - activate motifs using MotifOntology_v1,
    - emit symbolic layer scores into CORPUS.

- Refine SymbolismProfile_v1 and MotifOntology_v1 over time:
  - Adjust weights as more real work is done.
  - Add/remove motifs while staying within the SYM-001 model.

Meta:
- Journal header version bumped from v1.0 to v1.02 to reflect this major design milestone.
- ENTRY 014 is the new stable baseline for all future symbolic engine and corpus design work.

# END OF ENTRY 014
RAW_LOG_END >>>

A7. Assistants MUST NOT alter timestamps or entry IDs of past entries.
    Only new entries get new timestamps and IDs.

A8. NO automatic summarization of RAW_LOG unless the user explicitly requests it.
    Summaries belong in CURATED_SUMMARY, never as replacements of RAW_LOG content.

A9. Assistants MUST NOT insert executable code into RAW_LOG
    unless the user explicitly orders it.
    Metadata, pseudo-code, and structural examples are allowed.

A10. Assistants MUST NOT "fix", evaluate, or modernize past thinking.
     RAW_LOG is a historical timeline, not a live workspace.

============================================================
SECTION B â€” WRITE PERMISSIONS
============================================================

B1. MACHINE_HEADER (editable):
    - Assistants update this at the start or end of a session.
    - Content includes: current model state, focus, last entry ID, next resume prompt, and session state.

B2. CURATED_SUMMARY (semi-editable):
    - Assistants ONLY modify this section if the user asks for a summary refresh or upgrade.
    - When updating, assistants may overwrite the entire CURATED_SUMMARY block, but MUST NOT touch RAW_LOG.

B3. RAW_LOG (append-only):
    - Assistants add new ENTRY blocks ONLY at the bottom of RAW_LOG.
    - Each entry MUST follow ID + timestamp + title format.
    - Entries MUST NOT be interleaved or inserted between older entries.

============================================================
SECTION C â€” HOW TO APPEND NEW WORK
============================================================

When the user requests new work for this journal:

1. READ MACHINE_HEADER to know:
   - Last_Entry_ID
   - Current_Focus
   - Next_Resume_Prompt

2. CREATE a new entry at the bottom of RAW_LOG:

   # ENTRY 0XX â€” YYYY-MM-DD â€” TITLE
   <content>
   # END OF ENTRY 0XX

3. UPDATE MACHINE_HEADER:
   - Set Last_Entry_ID to the new entry ID (0XX).
   - Optionally update:
     - Last_Entry_Title
     - Last_Update_Date
     - Current_Focus
     - Next_Resume_Prompt
     - Session_State

4. DO NOT touch previous entries.

============================================================
SECTION D â€” DO NOT TOUCH LIST
============================================================

Never modify:
    - Fence names or positions
    - RAW_LOG past entries
    - Entry order
    - Historical content or timestamps
    - User-written text inside RAW_LOG
    - MACHINE_INSTRUCTIONS text

If unsure, ASK THE USER before modifying anything.

MACHINE_INSTRUCTIONS_END >>>
============================================================


============================================================
<<< CURATED_SUMMARY_START
# CURATED SUMMARY â€” HIGH-LEVEL MODEL (UPDATABLE ON REQUEST)

This section summarizes the current state of design thinking.
It can be updated when the user requests a new high-level summary.
RAW_LOG remains the source of truth for full history.

------------------------------------------------------------
1. DIARY / CORPUS HIGH-LEVEL MODEL
------------------------------------------------------------

1.1 Split:
- GOVERNING SIDE (BCL + RIDs + Helpers):
  - Contains rules, schemas, runbooks, and helper references.
  - Defines how to extract, structure, validate, and rebuild corpus data.
  - Contains NO corpus data.

- CORPUS SIDE:
  - Contains only data: descriptors, scenes, worlds, avatars, motifs, etc.
  - Machine-readable (e.g., JSON/JSONL).
  - No rules, no extraction logic, no instructions.

1.2 Rebuildability Principle:
- If the entire corpus is lost, the governing side MUST define:
  - How to re-ingest raw material (stories, images, notes).
  - How to re-process into descriptors and entities.
  - How to regenerate indexes.

1.3 Corpus Conceptual Sections:
- RAW_INGEST:
  - Rough dumps, loosely structured or plain text, from LLMs, notes, or tools.
- DESCRIPTORS:
  - Clean, structured records for scenes, shots, characters, locations, etc.
  - Each descriptor uses the 5-layer symbolic engine fields.
- ENTITIES:
  - Higher-level objects (characters, locations, worlds, motifs) referencing descriptors by ID.
- INDEXES:
  - Lookup structures (tag â†’ IDs, character â†’ scenes, motif â†’ scenes).
  - Regenerable from DESCRIPTORS + ENTITIES.

------------------------------------------------------------
2. SYMBOLIC ENGINE â€” v0.2 (5-LAYER MODEL)
------------------------------------------------------------

The user's creative identity is modeled as five interacting layers:

1) Ground Layer â€” Emotional Realism
   - Human truth, psychological realism, longing, grief, quiet desperation.
   - Scenes and characters feel believable even when worlds are stylized.

2) Symbolic Layer â€” Cinematic Meaning
   - Color, objects, spaces, architecture, clothing, and framing as symbols.
   - Recurring motifs (corridors, windows, reflections, rain, thresholds).
   - Symbolism must serve emotion or theme, not be random.

3) Dream/Chaos Layer â€” Primordial Surrealism
   - Short bursts of dream logic, distorted memory, or meta-reality.
   - Lynch/Paprika influence: inner reality briefly overrides outer reality.
   - Always anchored in character psychology or theme.

4) Melodrama Layer â€” Emotional Maximalism
   - Influences from K-drama and commercial emotional storytelling.
   - Big feelings, earnest expressions, close-ups, crescendos.
   - Used selectively, not everywhere.

5) Transcendence Layer â€” Spiritual Aesthetic (Agnostic)
   - The user is agnostic but drawn to spiritual symbolism.
   - Themes: mortality, rebirth, meaning, cycles, acceptance.
   - Motifs: water, trees, light, thresholds, circles, rituals, cosmic framing.
   - Sacred emotional feel without religious doctrine.

------------------------------------------------------------
3. CINEMATIC INFLUENCE MAP (ABBREVIATED)
------------------------------------------------------------

Key works shaping this engine include:
- Emotional melancholy & intimacy:
  - Lost in Translation, In the Mood for Love, My Liberation Notes, Queen of Tears.
- Moral/psychological drama:
  - Arrival, Sicario, Prisoners, The Assassination of Jesse James.
- Dream/meta surrealism:
  - Paprika, Vanilla Sky/Abre los Ojos, Tarkovsky works, 3-Iron, Lynch.
- Stylized tableaux / absurd realism:
  - Du Levande, Stellet Licht.
- Spiritual-existential cinema:
  - The Fountain, Tarkovsky, Stellet Licht (again).

These confirm:
- Love of melancholy and intimacy
- Fascination with moral ambiguity and existential tension
- Attraction to dream/reality overlap
- Acceptance of melodrama when sincere
- Desire for sacred-feeling compositions without religious belief

------------------------------------------------------------
4. DESCRIPTOR SHAPE (CONCEPTUAL)
------------------------------------------------------------

A scene descriptor conceptually looks like:

{
  "id": "SCN-YYYY-XXXXXX",
  "type": "scene",

  "source": {...},
  "links": {...},

  "layers": {
    "ground": {...},
    "symbolic": {...},
    "dream": {...},
    "melodrama": {...},
    "transcendence": {...}
  },

  "cinema": {...},
  "tech": {...}
}

- layers.ground / symbolic / dream / melodrama / transcendence
  align directly with the symbolic engine.
- cinema contains framing, lens, movement, and distance info.
- tech contains tags, tool-facing prompt fragments, etc.

------------------------------------------------------------
5. REUSE RECIPES (BRIEF RECAP)
------------------------------------------------------------

Several reuse patterns exist to build new scenes from existing descriptors:

- Recipe 1: "Skeleton + Skin"
  - New ground layer, reused symbolic + cinema layers.

- Recipe 2: "Mood Transplant"
  - Reuse ground, create new symbolic layer and possibly new cinema.

- Recipe 3: "Hybrid Merge"
  - Take ground from one scene, symbolic from another, cinema from a third, transcendence from a fourth.

- Recipe 4: "Descriptor Expansion"
  - Start with a minimal idea (ground only), then query corpus for symbolic and cinema matches and grow outward.

- Recipe 5: "Motif â†’ Scene"
  - Choose a motif (e.g., reflection, threshold, rain veil), query existing scenes, and build a new scene around that motif.

These recipes ensure reuse is structured and controlled, not random.

------------------------------------------------------------
6. CURRENT STATE & PIVOT
------------------------------------------------------------

- Symbolic Engine model is considered stable enough for now (v0.2).
- Focus has pivoted from pure idea exploration to:
  - Corpus layout
  - Descriptor schema
  - Reuse patterns grounded in the 5-layer engine
- Technical implementation (Python, ComfyUI, Unreal) is acknowledged but not yet fully specified.



------------------------------------------------------------
7. LATEST PIVOT â€” SYM-001 + CONFIG_PROFILES (2025-11-23)
------------------------------------------------------------

- Introduced SYM-001 as the symbolic engine law:
  - Fixes SHOT_DNA as the canonical per-shot fingerprint (discrete bands, no live JSON in the RID).
  - Formalises symbolic layers (liturgical, erotic, domestic, liminal, dream, power, grotesque, tenderness).
  - Separates symbolic law (SYM-001) from raw descriptor extraction (future DES-001).

- Activated CONFIG_PROFILES as the taste/ontology layer:
  - SymbolismProfile_v1: expresses how strongly different symbolic layers should be emphasised.
  - MotifOntology_v1: first pass at concrete motif families and slugs aligned with the userâ€™s tastes
    (devotional, domestic melancholy, intimate/erotic warmth, liminal corridors/thresholds, dream, power, grotesque, tenderness).

- Diary integration:
  - SYM-001 inserted into the BCL governing Topic.
  - CONFIG_PROFILES / CFG-PROFILES populated and structurally anchored between governing side and corpus.

- Open design frontiers:
  - DES-001 (descriptor RID) still to be written to govern raw uncensored visual descriptors.
  - Helper/tooling design still to be specified for:
    - DES-001 extraction,
    - SHOT_DNA mapping,
    - motif activation,
    - symbolic layer scoring into CORPUS.

- Practical meaning for future assistants:
  - Use CURATED_SUMMARY to understand the symbolic stack:
    GOVERNING LAW (SYM-001) â†’ CONFIG_PROFILES (taste + ontology) â†’ CORPUS descriptors.
  - Treat ENTRY 014 in RAW_LOG as the new stable baseline.
CURATED_SUMMARY_END >>>
============================================================


============================================================
<<< RAW_LOG_START
# RAW LOG â€” FULL CHRONOLOGICAL HISTORY (APPEND-ONLY)
# DO NOT DELETE OR MODIFY PAST ENTRIES
============================================================

# ENTRY 001 â€” 2025-11-22 â€” User Input Consolidation
Context:
- User described the history of the Diary and how it evolved:
  - Started as a helper for paid NSFW image generators.
  - Purpose: track quirks (syntax, weighting, what worked) to save money on prompts.
  - Generated ~15,000 prompts as a large corpus of reusable fuel.
  - Evolved into descriptor extraction: short, medium, long descriptions per image.
  - Intent: allow recombination of descriptors to build new prompts/scenes.
- Current direction:
  - Use Diary as a data store for descriptors from both images and stories.
  - Later reuse across tools (ComfyUI, local LLMs, Unreal, Blender, etc.).

Core requirements:
- Corpus must remain pure data; no rules or instructions.
- Governing RIDs hold extraction logic, schemas, and helper definitions.
- All helper code (Python, etc.) lives on governing side, never in corpus.
- Rebuildability: if corpus is lost, RIDs + helpers must be enough to reconstruct pipelines.
- Corpus should be machine-readable and tool-agnostic.

# END OF ENTRY 001


# ENTRY 002 â€” 2025-11-22 â€” Corpus as Data Container & Power-User Context
User intent:
- Corpus is a data container but should support future tools:
  - Python scripts
  - ComfyUI graphs
  - Unreal/Blender
  - LLM-based extraction
- Corpus must eventually serve:
  - Cross-tool pipelines
  - Memory of creative language
  - Non-automatic scene assembly (force multiplier, not replacement)

Design reflections:
- Labs and studios typically separate:
  - Data (stable, long-lived)
  - Code/pipelines (changeable)
  - Schemas/contracts (governing layer)
- Pipelines usually follow:
  - extract â†’ validate â†’ store â†’ index â†’ retrieve

Implications:
- Corpus must use schema-driven formats (JSON/JSONL).
- Rules and code live in RIDs and helper topics.
- Corpus stays logic-free and rebuildable.

# END OF ENTRY 002


# ENTRY 003 â€” 2025-11-22 â€” Cross-Domain Failure Patterns
Identified pitfalls from labs/industry patterns:
1. Data outlives code; poorly chosen formats age badly.
2. Intent is forgotten; data becomes noise without "why".
3. Unindexed corpora become unusable; lack lookup.
4. Schema changes break everything if not versioned.
5. Model behavior drifts; corpora must not be tied to a single model.
6. Reproducibility fails without full parameter recording.
7. Rules/data separation erodes over time when not enforced.
8. Version lineage disappears without explicit logging.
9. Tool lock-in: data only usable by a single ecosystem.
10. Lack of pruning and curation leads to signal/noise collapse.

Conclusion:
- Reinforces need for:
  - Strict governance vs corpus separation.
  - Versioned schemas and change logs.
  - Indexing, validation, and metadata.

# END OF ENTRY 003


# ENTRY 004 â€” 2025-11-22 â€” Blind Spots & Omissions
Areas requiring explicit design:
- Indexing:
  - IDs, tags, manifests, cross-references.
- Validation:
  - Schemas, linting, structural checks.
- Migration:
  - Frozen vs upgradable entries.
  - Recorded rules for schema evolution.
- Meta-metadata:
  - Which tool/model produced a descriptor.
  - Parameters and confidence levels.

Implication:
- Governing side needs RIDs for:
  - indexing
  - validation
  - migration
  - meta-metadata handling

# END OF ENTRY 004


# ENTRY 005 â€” 2025-11-22 â€” Clarified Origins & Intent (Q1â€“Q13)
User clarified via Q&A:
- Early struggles:
  - Coaxing NSFW behavior out of SFW-biased generators.
  - Frustration with close-up portrait bias vs desired wider scenes.
- 15k prompts:
  - Emerged from pushing an assistant to mass-produce prompts.
  - True intent: build fuel, not noise.
- Descriptor extraction:
  - 3 levels per image (short/medium/long).
  - Aimed at recombination and deeper analysis (pose, clothing, mood, etc.).
- Local models (e.g., Mythomax):
  - Used to lift OpenAI safety constraints by using external creativity,
    then bringing structured descriptors back to the Diary.
- Tools:
  - Expected to both read and write to corpus (Python, ComfyUI, LLM).
- Identity:
  - Corpus = database.
  - RIDs + helpers = â€œengineâ€ that manipulates that database.
- Rebuildability:
  - Dumping data without methods is meaningless.
  - Methods must exist in governing side to recreate everything.

# END OF ENTRY 005


# ENTRY 006 â€” 2025-11-22 â€” Creative Flaws Map (v0.1)
User requested focus on flaws to inform design.

Key limitations:
1. Hyperfocus on single scenes; difficulty expanding to full stories/worlds.
2. Strong results but weak process capture; missing intermediate steps.
3. Difficulty expressing intent clearly, especially emotionally.
4. Worldbuilding fatigue; continuity between scenes is fragile.
5. Avatar inconsistency (faces/bodies shift).
6. Concept drift between sessions.
7. Difficulty scaling from individual scenes to coherent narrative arcs.
8. Confidence gap at the expansion phase.

Design implications:
- Corpus must compensate by:
  - Providing continuity memory (characters, worlds, motifs).
  - Capturing intermediate shapes, not only final images.
  - Making reuse structured (recipes, schemas).
  - Supporting long-term world and character identity.

# END OF ENTRY 006


# ENTRY 007 â€” 2025-11-22 â€” Symbolism Taste & Lynch Threshold (v0.1)
Userâ€™s symbolic preferences:
- Loves symbolism as â€œlanguage on top of languageâ€.
- Enjoys visual, linguistic, and cultural metaphor.
- Likes Lynchâ€™s primordial chaos but finds prolonged opacity taxing.
- Prefers:
  - concentrated dream bursts
  - with emotional or thematic anchors

Early tri-layer engine:
1. Ground: reality & emotional truth.
2. Symbolic: motifs, metaphors, color logic.
3. Chaos: dream bursts.

Implication:
- Symbolism must serve function (emotion or theme).
- Pure randomness is not acceptable; chaos must be purposeful.

# END OF ENTRY 007


# ENTRY 008 â€” 2025-11-22 â€” Expanded Symbolic Cinema Palette (v0.1)
Works mentioned:
- Arrival, Sicario, Prisoners
- In the Mood for Love, Lost in Translation
- Hero
- Paprika
- 3-Iron / Binjip
- Tarkovsky works
- Du Levande
- Stellet Licht
- Vanilla Sky / Abre los Ojos
- Queen of Tears
- My Liberation Notes
- The Fountain
- Others: Lynch bursts, Andrei Tarkovsky, K-drama references, etc.

Pattern:
- Melancholic intimacy
- Moral/psychological tension
- Dream/meta structures
- Stylized tableaux
- Spiritual-existential undertones
- Emotional melodrama when sincere

This led to formalizing a 5-layer symbolic engine (see CURATED SUMMARY and ENTRY 010).

# END OF ENTRY 008


# ENTRY 009 â€” 2025-11-22 â€” Transcendence Layer (v0.1)
Userâ€™s worldview:
- Agnostic: believes there is no afterlife.
- Still deeply moved by religious/spiritual symbolism.
- Attracted to:
  - sacred-feeling spaces
  - rituals
  - mortality themes
  - rebirth and cycles
- The Fountain identified as a near-perfect match:
  - Emotional core: grief, love, acceptance.
  - Three timelines: realism, myth, cosmic dream.
  - Motifs: trees, water, circles, light.

Transcendence Layer:
- Symbolic, emotional, existential â€” not doctrinal.
- Focus on:
  - mortality and acceptance
  - emotional rebirth
  - meaning in finite life
  - sacred emotional framing

Design implication:
- Corpus descriptors may include transcendence fields:
  - theme (mortality, acceptance, etc.)
  - symbols (tree, water, light, cycles)

# END OF ENTRY 009


# ENTRY 010 â€” 2025-11-22 â€” Symbolic Engine (v0.2)
Formal 5-layer model:

1) Ground â€” Emotional Realism
2) Symbolic â€” Cinematic Meaning
3) Dream/Chaos â€” Primordial Surrealism
4) Melodrama â€” Emotional Maximalism
5) Transcendence â€” Spiritual Aesthetic

This engine:
- Aligns with userâ€™s tastes and influences.
- Serves as the backbone of descriptor schemas.
- Guides scene, character, world, and memory design.

# END OF ENTRY 010


# ENTRY 011 â€” 2025-11-22 â€” Character Design via Symbolic Engine (v0.1)
Character design via layers:

- Ground:
  - Desire, fear, wound, contradiction.
- Symbolic:
  - Colors, spaces, objects, gestures.
- Dream:
  - Recurring internal images/dreams.
- Melodrama:
  - How and when they break emotionally.
- Transcendence:
  - Relationship to meaning/mortality/acceptance.

Outcome:
- Characters become emotionally deep, symbolically coherent,
  psychologically complex, and visually expressive.

# END OF ENTRY 011


# ENTRY 012 â€” 2025-11-22 â€” Pivot from Idea Iterations to Technical Design
Context:
- Up to this point, focus was on:
  - symbolic engine
  - cinema influences
  - character/world/scene shaping
  - transcendence layer
- Next need: connect this to technical implementation:
  - corpus machine-readability
  - descriptors as actual schema fields
  - how this can support Python, ComfyUI, Unreal, etc.

User intent at pivot:
- Symbolic/creative side feels internalized for now.
- Wants to pause pure idea exploration.
- Wants to switch to:
  - corpus layout
  - descriptor design
  - reuse logic

Core technical desires restated:
- Corpus = pure data, no rules.
- Governing side (RIDs + helpers) = all methods.
- Corpus data must be:
  - JSON/JSONL-like
  - rebuildable from RIDs
  - usable by Python and other tools.

From this entry onward:
- Focus shifts from:
  - â€œWhat is my symbolic identity?â€
  to:
  - â€œHow do we encode it technically in descriptors and corpus layout?â€

# END OF ENTRY 012


# ENTRY 013 â€” 2025-11-22 â€” Drift Warning + Session Consolidation Before Stop
Title: Drift Warning + Session Consolidation
Type: Meta-Design / Continuity Protection

Context:
- Completed a full demonstration of a reuse recipe (Hybrid Merge) using a real corpus-style descriptor
  (e.g., kneeling blonde in fishnet outfit, photoshoot context) transformed into a multi-layer engine descriptor.
- Demonstrated:
  - mapping old flat data into new 5-layer schema
  - adding symbolic, cinematic, and transcendence aspects
  - keeping explicit shoot context at a structural level
- Conversation began drifting toward narrative generation and away from design/technical focus.

Drift Indicators:
- Mixed symbolic design, technical schema work, and story requests.
- Ambiguous next-step direction.
- User explicitly felt drift and requested a clean stop.

Stability Actions Taken:
- Reaffirmed pivot to technical shape:
  - corpus layout
  - descriptor schema
  - reuse mechanics
- Reinforced boundaries:
  - corpus = data only
  - governing side = methods and rules
- Stopped further narrative escalation.
- Introduced this ENTRY as a drift guard and session end marker.

Next Assistant Instructions:
- On next session, begin by asking:
  â€œDo we continue with descriptor reuse patterns, corpus schema refinement, or entity definitions?â€
- Maintain focus on technical and structural design rather than pure narrative generation,
  unless the user explicitly returns to story work.
- Use CURATED_SUMMARY for orientation; consult RAW_LOG only when deeper context is needed.

SESSION_STOP_MARKER:
- This entry marks the last known coherent state before user ended the session for the day.
- Any future work must treat ENTRY 013 as the stable baseline.

# END OF ENTRY 013


RAW_LOG_END >>>
============================================================


# ENTRY 014 â€” 2025-11-23 â€” BASE BLUEPRINT: SYMBOLIC ENGINE + CORPUS EXTRACTOR DESIGN

Context:
- User wants ONE canonical design container: the Symbolic Engine Journal.
- All plans, templates, and blueprints must live inside the Journal RAW_LOG, not in separate files.
- L1 (Diary) holds governance (RIDs, BCL) + CORPUS topic; L3 (this Journal) holds design history and blueprints.
- This entry consolidates:
  - Legacy CORPUS patterns found in L1.
  - Symbolic Engine design from previous entries.
  - The future extractor system architecture.
  - Entity types, pipeline stages, constraints, and next-assistant handoff.

This is a BASE BLUEPRINT, not a RID. Future assistants MUST implement the actual EXTRACTOR RID and helper RIDs from this.

------------------------------------------------------------
1. LEGACY CORPUS PATTERNS (SUMMARY)
------------------------------------------------------------

Inside the L1 Diary, in the CORPUS topic (between CORPUS DATA BEGIN/END), two distinct kinds of data exist:

1.1 PROMPT CORPUS (large, meta-indexed):
- Structure:
  - Batches and clusters, e.g.: â€œBatch N: Prompts 501â€“550 | CP2â€.
  - Per-prompt entries that carry:
    - prompt_id (Prompt NNNN).
    - cluster_id (CPx, e.g. CP1â€“CP8).
    - theme description (e.g. full-body, specific scenario themes).
    - variation index (different variants of the same base idea).
    - PQI score (quality index).
    - notes mentioning obfuscation placeholders and negatives (used for safety / anti-flag strategies).
- Intent:
  - To treat prompts as reusable archetypes, grouped by cluster and rated by quality.
  - To track â€œrecipesâ€ that give reliable results.

1.2 DESCRIPTOR CORPUS (smaller but semantically rich):
- Structured as repeated [DESCRIPTORS] ... [/DESCRIPTORS] blocks with fields like:
  - id
  - sentence (compact natural-language description of the scene)
  - style (indoor/outdoor, daylight, studio, etc.)
  - body (basic body/appearance descriptor)
  - wardrobe (underwear/lingerie/clothing items)
  - setting (bedroom, living room, bed, sofa, etc.)
  - pose (kneeling, standing, close-up gaze up, selfie, etc.)
  - lighting (soft, natural, backlit, etc.)
  - dynamics (e.g. â€œsize-contrastâ€)
  - notes (short hints about implied focus/intent)
  - consent (explicitly tagged `asserted_adults_only`)
- Intent:
  - To capture minimal but meaningful scene semantics (who/how/where).
  - To preserve body/wardrobe/setting/pose/lighting/dynamics in fixed fields.
  - To keep a short, high-signal summary (`sentence=`) usable in prompts.
  - To always encode explicit adult-consent at descriptor level.

Observation:
- The PROMPT CORPUS focuses on prompt archetypes, PQI, and clusters.
- The DESCRIPTORS focus on scene-level meaning: bodies, clothing, setting, pose, dynamics, lighting, and intent hints.
- Cinematic details (lens, angle, distance) and symbolic engine layers are not explicitly structured in the old data.

------------------------------------------------------------
2. ENTITY CLASSES FOR THE FUTURE SYSTEM
------------------------------------------------------------

ENTITY TYPE A: PROMPT_ENTRY
Fields:
- prompt_id
- cluster_id (CPx)
- theme / motif
- body_focus
- shot_type (full-body/portrait)
- PQI score
- notes (meta)

ENTITY TYPE B: SCENE_DESCRIPTOR
Core (legacy-compatible):
- id
- sentence
- style
- body
- wardrobe
- setting
- pose
- lighting
- dynamics
- notes
- consent (mandatory)

Extended:
- mood
- camera_distance
- camera_angle
- framing
- symbolic_tags
- ground_layer
- symbolic_layer
- dream_layer
- melodrama_layer
- transcendence_layer

ENTITY TYPE C (future): LINKS
- Connects PROMPT_ENTRY and SCENE_DESCRIPTOR.

------------------------------------------------------------
3. EXTRACTION PIPELINE (STAGES)
------------------------------------------------------------

STAGE 0: Raw Visual Description (VISUAL_SCHEMA)
STAGE 1: Base SCENE_DESCRIPTOR fill
STAGE 2: Cinematic Expansion
STAGE 3: Symbolic Engine mapping
STAGE 4: Prompt Linking (optional)

------------------------------------------------------------
4. DIARY-NATIVE STRUCTURE RULES
------------------------------------------------------------

- Single-file Diary.
- Templates must be plain text.
- CORPUS holds data, not governance.
- RIDs hold governance, not data.
- Consent tagging mandatory.
- Export helpers optional.

------------------------------------------------------------
5. FUTURE EXTRACTOR RID (CONSTRAINTS)
------------------------------------------------------------

MUST preserve legacy fields.
MUST enforce consent tagging.
MUST distinguish PROMPT vs SCENE.
MUST define helpers (IMAGE, TEXT, SYMBOLIC, CINEMA).
MAY define export helpers.
MUST NOT contain live corpus data.

------------------------------------------------------------
6. NEXT ASSISTANT HANDOFF
------------------------------------------------------------

Next assistant should:
- Write the MASTER EXTRACTOR RID.
- Draft descriptor templates.
- Create helper RIDs.
- Keep work deterministic and Diary-native.

# END OF ENTRY 014
````

---


## 8.7 Version History & Subsection Versioning

**Why**
Clear version tracking enables understanding which features are included in each release and how subsections with independent versions relate to the master specification version.

**What**
Documents master specification version history, maps independent subsection versions to master versions, and defines versioning policy.

**Discipline**
- Dates MUST reflect actual release/publication dates (no future dating).
- Each entry SHOULD list an owner and maturity (Normative / Draft / Research) for traceability; fill missing owners/maturities.
- Subsection rows MUST stay in sync with source documents and ADRs; divergences are called out explicitly.

---

### 8.7.1 Master Specification Versions

| Version | Date | Description | Owner | Maturity |
|---------|------|-------------|-------|----------|
| **v02.179** | 2026-03-28 | Defined explicit `workflow_run` and `workflow_node_execution` Debug Bundle scopes, added canonical workflow-node inventory and manifest-count rules, extended exporter and exportable-inventory posture for workflow correlation, and deepened FEAT-DEBUG-BUNDLE guidance so workflow evidence stays bounded without time-window reconstruction. | Orchestrator | Normative |
| **v02.177** | 2026-03-11 | Defined structured Role Mailbox handoff bundles, note-transcription duties, and announce-back provenance; clarified how Role Mailbox, Work Packet System, Locus Work Tracking, Micro-Task Executor, Task Board, and Dev Command Center preserve remaining work, next-actor, and transcription state without making mailbox narrative authoritative; and deepened Appendix 12 coverage, interaction notes, and stub mapping for mailbox handoff and announce-back implementation gaps. | Orchestrator | Normative |
| **v02.176** | 2026-03-11 | Defined Role Mailbox executor kinds, claim or lease modes, response-authority scope, claimant visibility, takeover policy, and lease-expiry posture; clarified how Role Mailbox, Dev Command Center, Locus Work Tracking, Micro-Task Executor, Task Board, and Work Packet System expose temporary ownership without making mailbox claims authoritative for linked work; and deepened Appendix 12 coverage, interaction notes, and stub mapping for mailbox executor-routing and claim-lease implementation gaps. | Orchestrator | Normative |
| **v02.174** | 2026-03-10 | Defined verifier-driven Role Mailbox loop checkpoints, structured verifier outcomes, bounded retry and escalation posture, completion-report transcription duties, and Dev Command Center loop projections for mailbox-linked Micro-Task execution; and deepened Appendix 12 coverage, interaction notes, and stub mapping for mailbox-loop-control implementation gaps. | Orchestrator | Normative |
| **v02.173** | 2026-03-10 | Defined typed Role Mailbox message families, thread-lifecycle states, delivery states, allowed-response envelopes, and Micro-Task collaboration message groundwork; clarified mailbox-local versus governed actions and linked-authority boundaries; and deepened Appendix 12 coverage, interaction notes, and stub mapping for mailbox message-thread contract implementation gaps. | Orchestrator | Normative |
| **v02.172** | 2026-03-10 | Defined portable workflow transition rules, queue automation rules, and executor eligibility policies for Work Packets, Micro-Tasks, Task Board projections, Role Mailbox-linked waits, and Dev Command Center action previews; clarified approval-gated versus automatic state changes; and deepened Appendix 12 coverage, interaction notes, and stub mapping for transition-automation registry implementation gaps. | Orchestrator | Normative |
| **v02.171** | 2026-03-10 | Defined one project-agnostic workflow-state family, queue-reason vocabulary, and governed-action descriptor for Work Packets, Micro-Tasks, Task Board projections, Role Mailbox-linked queues, and Dev Command Center routing; clarified project-profile workflow label overrides; and deepened Appendix 12 coverage, interaction notes, and stub mapping for workflow-state registry implementation gaps. | Orchestrator | Normative |
| **v02.170** | 2026-03-10 | Defined Dev Command Center view presets, lane definitions, and governed action bindings for board, queue, list, roadmap, inbox-triage, and execution-queue layouts; clarified local-small-model readiness queues and layout-mutation semantics; and deepened Appendix 12 coverage, interaction notes, and stub mapping for layout-projection implementation gaps. | Orchestrator | Normative |
| **v02.169** | 2026-03-10 | Defined mirror authority modes, reconciliation actions, and mirror contracts for structured collaboration artifacts; clarified how canonical JavaScript Object Notation records, compact summaries, Markdown mirrors, and note sidecars synchronize without creating silent second authorities; strengthened Dev Command Center mirror-drift and regeneration posture; and deepened Appendix 12 coverage and stub reuse for Markdown mirror sync and typed viewer gaps. | Orchestrator | Normative |
| **v02.168** | 2026-03-10 | Defined the shared base structured-collaboration envelope, compact summary contract, mirror-state semantics, and project-profile extension boundaries for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports; strengthened Dev Command Center field provenance expectations; and deepened Appendix 12 coverage and stub mapping for schema-registry and profile-extension implementation gaps. | Orchestrator | Normative |
| **v02.167** | 2026-03-10 | Defined the canonical structured collaboration artifact family as versioned JavaScript Object Notation or JavaScript Object Notation Lines for Work Packets, Micro-Tasks, Task Board projections, and Role Mailbox exports; clarified project-agnostic base envelopes and Markdown mirror-sync rules; added projected board and queue guidance for Dev Command Center; and deepened Appendix 12 coverage, interaction notes, and stub mapping for structured artifact implementation gaps. | Orchestrator | Normative |
| **v02.166** | 2026-03-10 | Added structured collaboration-substrate rules, clarified canonical structured records versus Markdown mirrors for Work Packets, Micro-Tasks, Task Board, and Role Mailbox, added Dev Command Center structured record and collaboration-inbox viewing posture, and deepened Appendix 12 coverage and interaction edges for structured work-state projection. | Orchestrator | Normative |
| **v02.165** | 2026-03-09 | Added Dev Command Center operating-surface backend rules, deepened Dev Command Center ownership of replay-safe run history, tool infrastructure health, workspace runtime readiness, and promotion-gate snapshots, and added Dev Command Center to Workflow Engine and Unified Tool Surface edges for replay and tool-health projection. | Orchestrator | Normative |
| **v02.164** | 2026-03-09 | Added Dev Command Center resilience and governed repository-decision backend rules, deepened Dev Command Center ownership of session checkpoints, heartbeat freshness, provider capability readiness, anti-pattern alerts, and repository-engine backend policy, and added Dev Command Center to Model Session Orchestration and Unified Tool Surface edges. | Orchestrator | Normative |
| **v02.163** | 2026-03-09 | Added Dev Command Center planning-and-coordination backend rules, backfilled Task Board and Work Packet System appendix ownership, deepened planning-state projection across Dev Command Center/Locus/Workflow Engine/Model Session Orchestration/Micro-Task Executor, and added Dev Command Center to Task Board, Dev Command Center to Work Packet System, Task Board to Locus Work Tracking, and Work Packet System to Workflow Engine edges. | Orchestrator | Normative |
| **v02.162** | 2026-03-09 | Added Dev Command Center work-orchestration backend rules, deepened Dev Command Center ownership of tracked Work Packet state, Task Board freshness, Micro-Task summaries, ready-query state, and parallel model session occupancy, and added Dev Command Center to Locus Work Tracking, Dev Command Center to Micro-Task Executor, and Model Session Orchestration to Locus Work Tracking edges. | Orchestrator | Normative |
| **v02.161** | 2026-03-09 | Added Dev Command Center evidence-and-replay backend rules, deepened Dev Command Center ownership of Governance Pack export, Workspace Bundle export, diagnostics query state, and workflow-linked evidence packaging, and added Dev Command Center to Governance Pack, Workspace Bundle, and Diagnostics Schema edges. | Orchestrator | Normative |
| **v02.160** | 2026-03-09 | Added Dev Command Center control-plane backend rules, deepened Dev Command Center ownership of workflow runs, artificial intelligence jobs, capability decisions, model sessions, and work packet or worktree bindings, and added Dev Command Center to Workflow Engine, Artificial Intelligence Job Model, Capabilities and Consent, and Model Session Orchestration edges. | Orchestrator | Normative |
| **v02.159** | 2026-03-09 | Added Dev Command Center / Operator Consoles correlation-projection rules, clarified full-name wording on touched additions, deepened Dev Command Center / Operator Consoles / Role Mailbox appendix ownership, and added Dev Command Centerâ†’Flight Recorder, Dev Command Centerâ†’Debug Bundle, and Role Mailboxâ†’Dev Command Center edges. | Orchestrator | Normative |
| **v02.158** | 2026-03-09 | Added Stage/Studio/Media/ASR backend rules, deepened ASR / Media Downloader / Stage / Studio / Atelier-Lens appendix ownership, added ASR recorder-portability and media-to-ASR edges, and materialized Stage-to-ASR transcript lineage as a stub-backed Phase 1 track. | Orchestrator | Normative |
| **v02.157** | 2026-03-09 | Added distillation/context/spec-router backend rules, deepened Skill Bank / Context Packs / ACE Runtime / Micro-Task Executor / Spec Router appendix ownership, added backend learning-substrate edges, and materialized Context Pack recorder visibility as a stub-backed Phase 1 track. | Orchestrator | Normative |
| **v02.156** | 2026-03-09 | Added knowledge/retrieval pillar backend rules, deepened Project Brain / Semantic Catalog / Context Packs / Loom appendix ownership, added retrieval-substrate and storage-portability edges, and materialized Loom portability as a stub-backed Phase 1 track. | Orchestrator | Normative |
| **v02.155** | 2026-03-09 | Added Calendar-centered backend force-multiplier rules, deepened Calendar appendix ownership, and added Calendar storage-portability / consent / AI-job / Spec Router matrix edges while keeping unresolved correlation bridges stub-backed. | Orchestrator | Normative |
| **v02.154** | 2026-03-09 | Added backend governance/export reciprocity rules, backfilled Governance Pack and Workspace Bundle appendix ownership, and added Governance Pack/workspace-export matrix edges plus stub-backed Phase 1 delivery posture. | Orchestrator | Normative |
| **v02.153** | 2026-03-09 | Added backend capability/diagnostics evidence rules, deepened capability/workflow/spec-router/MCP/diagnostics backend projection posture, added capability-recorder and diagnostics-bundle edges, and materialized unresolved cloud-consent portability as a stub-backed Phase 1 track. | Orchestrator | Normative |
| **v02.152** | 2026-03-08 | Added backend orchestration/projection/replay rules, deepened Spec Router / Locus / MCP / MEX backend evidence posture, added spec-router/locus/mcp/mex projection edges, and materialized unresolved bridges as stub-backed Phase 1 tracks. | Orchestrator | Normative |
| **v02.151** | 2026-03-08 | Added backend export/evidence/portability rules, deepened Role Mailbox / AI-Ready Data / Workflow Engine backend evidence posture, added mailbox/AI-ready/workflow storage-recorder edges, and materialized unresolved bridges as stub-backed Phase 1 tracks. | Orchestrator | Normative |
| **v02.150** | 2026-03-08 | Added backend-heavy matrix expansion rules, deepened workflow/consent/calendar/stage-media projection contracts, and materialized backend combo growth as explicit matrix edges plus stub-backed Phase 1 tracks. | Orchestrator | Normative |
| **v02.149** | 2026-03-08 | Hardcoded refinement reciprocity, roadmap/cov-matrix coupling, `[ADD v<version>]` packet visibility, primitive exposure/creation reporting, and the new MATRIX_RESEARCH_RUBRIC + GUI_IMPLEMENTATION_ADVICE_RUBRIC. | Orchestrator | Normative |
| **v02.148** | 2026-03-08 | Reduced orphan ownership by attaching Stage/media session-auth contracts, multi-session runtime substrate, and debug/export/retention contracts to owning feature rows; added Stage?Media Downloader, Model Session?AI Job, and Debug Bundle?Storage Portability edges. | Orchestrator | Normative |
| **v02.147** | 2026-03-08 | Attached high-signal orphan primitives to owning feature rows, deepened capability/consent, AI job, debug-bundle, storage portability, and operator projection contracts, and added explicit export/projection interaction edges. | Orchestrator | Normative |
| **v02.146** | 2026-03-08 | Deepened seeded Appendix 12 rows with AI-ready artifact/status contracts, AI Job + Flight Recorder UI surfaces, Role Mailbox/Locus/Loom query primitives, and direct job-consent / MEX-Flight Recorder interaction edges. | Orchestrator | Normative |
| **v02.145** | 2026-03-08 | Added third-pass runtime/data/operator coverage: new feature rows for Model Session Orchestration, Cloud Escalation Consent, and MEX Runtime; deeper typed runtime/export/filter/session contracts; and execution-path interaction edges across Appendix 12. | Orchestrator | Normative |
| **v02.144** | 2026-03-08 | Expanded Appendix 12 with second-pass feature-family coverage, richer runtime visibility rows, additional high-signal primitives/tools/tech, and stub-linked backlog for unresolved embodiments. | Orchestrator | Normative |
| **v02.143** | 2026-03-08 | Added ?6.0.2.11 Primitive Index Coverage Contract (MUST), normalized Appendix 12.4 into coverage-driven feature rows, and seeded runtime/job/tool/frontend/operator primitives with stub-linked gaps. | Orchestrator | Normative |
| **v02.142** | 2026-03-08 | Added ?6.0.2.10 Runtime Visibility Contract (MUST) and expanded Appendix 12 with capability slices, runtime visibility rows, and runtime-linked interaction edges. | Orchestrator | Normative |
| **v02.136** | 2026-02-22 | Updated Â§7.6 Roadmap + Â§7.6.1 Coverage Matrix to schedule Unified Tool Surface Contract (HTC v1.0, Â§6.0.2) implementation (Tool Registry + Tool Gate + MCP schema generation) and to phase Design Studio shell/IA recontextualization into Phase 2+ (avoid Phase 1 rework). | Orchestrator | Normative |
| **v02.135** | 2026-02-22 | Added Â§6.0.2 Unified Tool Surface Contract (HTC-1.0) as single source of truth (incl. `assets/schemas/htc_v1.json`), required Tool Gate for local + MCP tool calls, added DCC Tool Call Ledger UX, added MCP binding rules (Â§11.3.0), and added Flight Recorder `tool_call` event schema (FR-EVT-007). Also added module/worksurface naming guardrails (Design Studio is additive). | Orchestrator | Normative |
| **v02.134** | 2026-02-20 | Added OutputRootDir (default materialization root) and added Â§10.14 Media Downloader unified archiving surface (YouTube/Instagram/forum crawler/generic video) with progress, Stage Sessions auth, and Export/materialize routing. | Orchestrator | Normative |
| **v02.133** | 2026-02-20 | Canonicalized cloud escalation Flight Recorder events: declared FR-EVT-CLOUD-001..004 canonical, aligned mirrors, removed FR-EVT-CLOUD-005 and consent-presented/received event types. | Orchestrator | Normative |
| **v02.132** | 2026-02-19 | Canonicalized AutomationLevel + GovernanceDecision across Main Body and Â§10.13 Stage import; defined AutoSignature schema; aligned self-approval to FR-EVT-GOV-001..005; pinned LOCKED semantics. | Orchestrator | Normative |
| **v02.131** | 2026-02-19 | Integrated `handshake-stage-spec_v0.6.md` as Â§10.13 (Handshake Stage) and updated roadmap/matrix (Â§7.6/Â§7.6.1) + profile tables (Â§2.6.6.6.2). | Orchestrator | Normative |
| **v02.130** | 2026-02-18 | Integrated Loom integration spec as Â§10.12 (Loom) and updated JobKind strings/roadmap accordingly. | Orchestrator | Normative |
| **v02.123** | 2026-01-30 | Merged `Handshake_Atelier_Lens_Addendum_v0.2.3.md` into Â§6.3.3.5.7 (new subclauses .11â€“.25), embedded source archives in Â§8.6.8, clarified `LensExtractionTier` vs `content_tier`, and updated roadmap (Â§7.6) with `[ADD v02.123]` items. | PM/Architect | Normative |
| **v02.99** | 2025-12-31 | Expanded AI Job Model JobKind/JobState lists, added canonical JobKind strings, and defined FR-EVT-WF-RECOVERY. | Orchestrator | Normative |
| **v02.68** | 2025-12-23 | Integrated Mechanical Extension v1.2 (Tool Bus contract + conformance + Â§11.8 verbatim import) and updated roadmap (Â§7.6) with MEX v1.2 sequencing across Phases 1â€“4; updated subsection version mapping. | PM/Architect | Normative |
| **v02.52** | 2025-12-20 | Updated roadmap (Â§7.6) to implement ACE-RAG-001 cleanly (QueryPlan/Trace plumbing in Phase 1; ContextPacks/caching/drift/conformance in Phase 2; transcript selectors in Phase 3; multi-user governance in Phase 4). | PM/Architect | Normative |
| **v02.51** | 2025-12-19 | Added Â§2.6.6.7.14 ACE-RAG-001 Retrieval Correctness & Efficiency (QueryPlan, Semantic Catalog, ContextPacks, RetrievalTrace, caching, drift, validators, conformance); extended ACE runtime + Index Doctor. | PM/Architect | Normative |
| **v02.50** | 2025-12-18 | Updated artifact system (Â§2.3.10) and appended artifact roadmap entries across Phases 1â€“4 (manifests, SHA-256, canonical bundle hashing, retention/pinning/GC, materialize semantics). | PM/Architect | Normative |
| **v02.47** | 2025-12-17 | Roadmap updated to schedule Charts/Dashboards/Decks across Phases 1â€“4 (guardrails, deliverables, export + provenance, collaboration). | PM/Architect | Normative |
| **v02.46** | 2025-12-17 | Added Charts & Dashboards (Â§10.7) and Presentations/Decks (Â§10.8); added Charts & Decks AI Job Profile (Â§2.5.11); extended core entities (Chart/Deck); fixed markdown code-fence formatting in Â§4.2; resolved duplicate section numbers (2.4.4/5.2.4). | PM/Architect | Normative |
| **v02.45** | 2025-12-17 | Updated Â§6.2.10.1; added Â§11.7.4. | PM/Architect | Normative |
| **v02.35** | 2025-12-15 | Updated roadmap (Â§7.6): Phase 1 MVP now explicitly includes Operator Consoles (Problems/Jobs/Timeline) + Debug Bundle + validators; later phases expand console-driven inspection surfaces. | PM/Architect | Normative |
| **v02.34** | 2025-12-15 | Added Â§10.5 Operator Consoles (Debug & Diagnostics); expanded Â§11.4 diagnostics schema + validators; expanded Â§11.5 Flight Recorder event shapes (incl. Debug Bundle export). | PM/Architect | Normative |
| **v02.32** | 2026-02-16 (unverified) | Integrated ACE runtime v0.1.1 / Calendar-driven ACE v0.3 content (hashing/scope/retrieval/logging/acceptance), added capability/schema/CI discipline, status markers for product surfaces/mechanical engines, and refresh notes for benchmarks/risks/gaps. | PM/Architect | Unverified |
| v02.31 | 2025-12-14 | Added Â§10.4.2 Calendar â†” ACE Integration (CalendarScopeHint + ACE compatibility invariants) and updated versioning tables for ACE runtime/calendar integration. | PM/Architect | Normative |
| v02.30 | 2025-12-14 | Added Â§2.6.6.7 ACE Runtime (compiled context per call, determinism modes strict/replay, ContextSnapshot/SourceRef, artifact-first tool output, schema-driven compaction, validators, logging + tests). | Runtime/Platform lead | Normative |
| v02.29 | 2025-12-14 | Fixed section header formatting (all top-level sections now use consistent `# N.` format). Removed duplicate section headers (sections 6 and 7 had duplicates). Standardized date format to ISO 8601. Added TOC note for reserved sections 10.5-10.8. Added Section 8.7 (Version History & Subsection Versioning). No functional changes to technical content. | PM/Architect | Normative |
| v02.28 | 2025-12-13 | Added Skill Bank (Section 9), Flight Recorder event shapes (11.5). | PM/Architect | Normative |
| v02.27 | 2025-12-10 | Merged Atelier concepts, updated roadmap (7.6). | PM/Architect | Draft |

### 8.7.2 Subsection Independent Versions

Some subsections maintain independent version numbers due to separate evolution or external protocol alignment. The table below maps subsection versions to the master specification version that incorporates them.

| Subsection | Version | Incorporated in Master | Notes | Owner | Maturity |
|------------|---------|------------------------|-------|-------|----------|
| 6.3 Mechanical Extension Engines | v1.2 | v02.68 | Tool Bus contract + gates + conformance vectors; canonical spec imported in Â§11.8 (22 engines) | Integrations/Platform lead | Draft |
| 10.3 Mail Client | v0.5 | v02.29 | Research/design phase; not yet implemented | Integrations/Backend lead | Research |
| 2.5.10 Docs & Sheets AI Integration | v0.5-draft | v02.29 | Protocol spec verbatim import | Docs/Sheets AI lead | Draft |
| 2.6.6 AI Job Model | v0.1 | v02.29 | Design note integration | Runtime/Platform lead | Draft |
| 2.6.6.7 ACE Runtime (Agentic Context Engineering) | v0.1.1 | v02.30 | Integrated ACE runtime primitives into global AI Job Model | Runtime/Platform lead | Normative |
| 10.4.2 Calendar â†” ACE Integration | v0.1 | v02.32 | CalendarScopeHint + ACE invariants; references Â§10.4.1 Calendar Law and Â§2.6.6.7 ACE runtime; updated with v0.3 projection/logging/security defaults | Calendar/Workflow lead | Draft |
| 10.5 Operator Consoles: Debug & Diagnostics | v0.2 | v02.34 | Operator consoles + Debug Bundle; contracts/validators in Â§11.4/Â§11.5 | Operator/Platform lead | Draft |
| 10.6 Canvas: Typography & Font Packs | v0.1 | v02.37 | Verbatim import: Font Packs + Canvas Typography Support Spec (v0.1); headings adjusted | UI/Canvas lead | Draft |
| 9.1 Skill Bank & Distillation | [Internal] | v02.29 | Canonical spec verbatim import (no separate version) | ML/Distillation lead | Research |
| 10.10 Photo Studio (Photo Stack) | v0.3.0 | v02.79 | Photo Stack spec integrated across Â§2.2.3 / Â§6.3.3.6 / Â§10.10 / Â§11.7.6 / Â§5.4.7 / Â§5.5.7 / Â§2.3.10.11 | Creative/Photo lead | Draft |
| 10.12 Loom (Heaper-style Library Surface) [ADD v02.131] | v1.1.0 | v02.130 | Loom integration spec imported as Â§10.12; entities/edges/job kinds/events integrated across core sections | Product/Library lead | Draft |
| 10.13 Handshake Stage (Built-in Browser + Stage Apps) [ADD v02.131] | v0.6 | v02.131 | Stage spec imported as Â§10.13; Stage Bridge + sessions + mechanical capture/3D assist pack | Product/Stage lead | Draft |

**Versioning Policy:**
- **Master versions (v0X.YY)** increment for major integrations, structural changes, or quarterly releases.
- **Subsection versions** may use independent numbering (v1.X, v0.X) when:
  1. Subsection originates from external protocol or imported specification
  2. Subsection evolves independently as a standalone module
  3. Version numbers have semantic meaning within that subsection's domain

**When in doubt:** Refer to Master version (v02.52) as the authoritative integration point.
### 8.7.3 Change Log (Recent)

#### v02.99 (2025-12-31)
**Updated:**
- A2.6.6.2.8: expanded JobKind/JobState lists and added canonical JobKind strings.
- A2.6.6.3.2/3.3: defined stalled state and constraints.
- A11.5: added FR-EVT-WF-RECOVERY event shape.

#### v02.68 (2025-12-23)
**Updated:**
- Â§6.3 Mechanical Extension Engines: updated to v1.2 normative scope; added Â§6.3.0 Mechanical Tool Bus contract summary (gates/registry/conformance/evidence) and clarified normative vs backlog.
- Â§11.8 (new): imported Mechanical Extension Specification v1.2 (headings shifted +2) as the canonical engine contract and 22-engine templates.
- Â§7.6 Development Roadmap: appended Mechanical Extension v1.2 implementation items across Phases 1â€“4 tagged `[ADD v02.68]` (Tool Bus envelopes, required gates, registry, conformance, evidence policy, packaging posture).
- Â§8.7 Version tables: added v02.68 entry and updated Â§6.3 subsection mapping to v1.2.

#### v02.67 (2025-12-22)
**Updated:**
- Â§7.6 Development Roadmap: scheduled Atelier Lens Runtime (role claiming + dual-contract extraction + merge/arbitration + production graph + ConceptRecipe) across Phases 1â€“4 using the fixed phase template; all new roadmap bullets tagged `[ADD v02.67]`.

#### v02.52 (2025-12-20)
**Updated:**
- Â§7.6 Development Roadmap: appended ACE-RAG-001 implementation sequencing across Phases 1â€“4 (QueryPlan/Trace plumbing, ContextPacks, caching, drift guards, transcript selectors, multi-user governance).

#### v02.51 (2025-12-19)
**Added/Updated:**
- Â§2.6.6.7.14 Retrieval Correctness & Efficiency (ACE-RAG-001): QueryPlan, Semantic Catalog, ContextPacks, RetrievalTrace, hash-key caching, drift detection, validators, and conformance tests.
- Â§2.6.6.7.9 Context Compiler: step (5) and (6) extended to reference QueryPlan routing, ContextPacks/LocalWebCacheIndex, and RetrievalTrace.
- Â§2.6.6.7.11 Validators: added RetrievalBudgetGuard, ContextPackFreshnessGuard, IndexDriftGuard, CacheKeyGuard.
- Â§2.6.6.7.12 Logging + Acceptance Tests: added QueryPlan/RetrievalTrace logging and ACE-RAG-001 conformance tests reference.
- Â§2.5.12 Context Packs AI Job Profile (new).
- Â§2.3.8 Shadow Workspace: vector records MUST include `source_hash` for drift detection.
- Â§10.5.5.7 Index Doctor / Consistency Auditor: extended retrieval/consistency fields (QueryPlan/RetrievalTrace, cache hits, drift flags, truncation).

#### v02.50 (2025-12-18)
**Updated:**
- Â§7.6 Development Roadmap: appended artifact-system roadmap entries tagged `[ADD v02.49]` across Phases 1â€“4 (manifests, SHA-256, canonical bundle hashing, retention/pinning/GC, materialize semantics).

#### v02.49 (2025-12-18)
**Added/Updated:**
- Â§2.3.10: added Artifact manifests + on-disk layout, bundle canonical hashing rules, retention/pinning/GC requirements, and materialize semantics.
- Standardized integrity hashing to SHA-256 and updated workspace folder tree to include `.handshake/artifacts/`.

#### v02.47 (2025-12-17)
**Added:**
- Roadmap integration for Charts/Dashboards/Decks across Phases 1â€“4 (Phase 1 guardrails/out-of-scope; Phase 2 deliverables + export/provenance; Phase 3 transcriptâ†’deck; Phase 4 collaboration + extensions).

#### v02.46 (2025-12-17)
**Added:**
- Â§2.5.11 Charts & Decks AI Job Profile
- Â§10.7 Charts & Dashboards
- Â§10.8 Presentations (Decks)

**Updated:**
- Â§2.2.1 Core Entities: added Chart and Deck
- Â§2.5.9 Tool integration note: clarified chart/deck ID-based refs
- Â§7.1 UI tool families: added charts/dashboards + decks/slideshows rows
- Â§8.7 Version History: corrected current-version references

#### v02.45 (2025-12-17)
**Updated:**
- Â§6.2.10.1 (updated)

**Added:**
- Â§11.7.4 (new)

#### v02.38 (2025-12-16)
**Added:**
- Â§7.6 Development Roadmap (Phase 1): appended Canvas Typography + Font Packs deliverables, acceptance criteria, and Mechanical Track items; added risk note re Excalidraw vs typography compatibility.

#### v02.37 (2025-12-16)
**Added:**
- Â§10.6 Canvas: Typography & Font Packs (verbatim import of Font Packs + Canvas Typography Support Spec v0.1; headings adjusted)

**Updated:**
- Â§8.7.2 Subsection Independent Versions: added 10.6 entry (v0.1 â†’ v02.37)
- Table of Contents: 10.6 is now an explicit surface entry; 10.7â€“10.8 remain reserved

#### v02.32 (2026-02-16)
**Added/Updated:**
- Integrated ACE runtime v0.1.1 and Calendar-driven ACE v0.3 content (ScopeInputs/ContextPlan hashing, retrieval policy, projection/redaction tables, logging, acceptance tests).
- Added capability/schema/CI discipline (single schema source; plugin capability mapping), status markers for product surfaces and mechanical engines.
- Added notes to refresh benchmarks (LLM runtimes, ASR) and risk/stack/gap tables; expanded Calendar logging/tests and security guardrails.
- Marked TODO/test placeholders explicitly (IMG001-180 stub; golden test placeholder).

#### v02.31 (2025-12-14)
**Added:**
- Â§10.4.2 Calendar â†” ACE Integration (CalendarScopeHint schema + ACE compatibility invariants; job-boundary routing discipline).

**Fixed:**
- Updated Â§8.7 version tables to include v02.30/v02.31 and ACE runtime/calendar integration.

#### v02.30 (2025-12-14)
**Added:**
- Â§2.6.6.7 ACE Runtime (compiled context, determinism strict/replay, ContextSnapshot, SourceRef, artifact-first, compaction schemas, validators, logging/tests).
- Updated AI Job Model `context_snapshot` to reference Â§2.6.6.7.3.

#### v02.29 (2025-12-14)
**Fixed:**
- **Critical:** Standardized all top-level section headers to use consistent `# N.` format (added periods to sections 5, 10, 11)
- **Critical:** Removed duplicate section headers (sections 6 and 7 had duplicates)
- **High:** Changed date format from "P25-12-14" to "Date: 2025-12-14" (ISO 8601 compliant)
- **Medium:** Added TOC note documenting that sections 10.5-10.8 are reserved for future expansion

**Added:**
- Section 8.7: Version History & Subsection Versioning

**No functional changes to technical content.**

#### v02.28 (2025-12-13)
- Added Section 9: Continuous Local Skill Distillation
- Added Section 11.5: Flight Recorder Event Shapes & Retention

#### v02.27 (2025-12-10)
- Merged Atelier-related concepts
- Updated Section 7.6: Development Roadmap

---

**Key Takeaways**
- Current version: v02.47 (2025-12-17)
- v02.30 introduced the ACE runtime (Â§2.6.6.7) used by the AI Job Model
- v02.31 added Calendar â†” ACE integration (Â§10.4.2); v02.32 aligns it with v0.3 projection/logging/security defaults and adds ACE runtime detail
- Subsections with independent versions are tracked in 8.7.2

---


<a id="9-continuous-local-skill-distillation-skill-bank-pipeline"></a>
