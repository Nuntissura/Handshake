# Phase 1 Evidence Map (Spec v02.103)

## Inputs
- Spec (authority): `Handshake_Master_Spec_v02.103.md`
- Roadmap source: `Handshake_Master_Spec_v02.103.md:20233` (Section 7.6.3 Phase 1)
- Task Board: `.GOV/roles_shared/TASK_BOARD.md`

## Purpose
Map each Phase 1 Roadmap "MUST deliver" item to the governing (non-roadmap) spec sections that define the requirements, and map each item to the current WP IDs on the Task Board.

## Notes on governance
- The Roadmap itself (`Handshake_Master_Spec_v02.103.md:20065`) contains normative language ("MUST deliver"), but the governance rule states the Roadmap is a pointer and Main Body is the authority.
- This map points Phase 1 Roadmap items to Main Body sections (and explicitly flags when an item points only to non-authoritative sections like Section 7).
- Task Board status labels in parentheses (Done / Ready for Dev / Stub Backlog) are taken from `.GOV/roles_shared/TASK_BOARD.md`.

---

## Addendum (v02.105) â€” Ready-for-Dev FAIL remediation stubs

`.GOV/roles_shared/TASK_BOARD.md` marks many Phase 1 items as FAIL (revalidation) and provides an additive stub revision under `.GOV/task_packets/stubs/`.
When selecting the WP to execute for an item below, prefer the additive stub revision and activate it (Technical Refinement Block â†’ USER_SIGNATURE â†’ official packet) before any implementation work.

| Task Board WP | Additive Stub WP |
|---|---|
| `WP-1-ACE-Auditability` | `WP-1-ACE-Auditability-v2` |
| `WP-1-ACE-Runtime` | `WP-1-ACE-Runtime-v2` |
| `WP-1-AI-Job-Model-v3` | `WP-1-AI-Job-Model-v4` |
| `WP-1-AI-UX-Actions` | `WP-1-AI-UX-Actions-v2` |
| `WP-1-AI-UX-Rewrite` | `WP-1-AI-UX-Rewrite-v2` |
| `WP-1-AI-UX-Summarize-Display` | `WP-1-AI-UX-Summarize-Display-v2` |
| `WP-1-Atelier-Lens` | `WP-1-Atelier-Lens-v2` |
| `WP-1-Calendar-Lens` | `WP-1-Calendar-Lens-v2` |
| `WP-1-Canvas-Typography` | `WP-1-Canvas-Typography-v2` |
| `WP-1-Capability-SSoT` | `WP-1-Capability-SSoT-v2` |
| `WP-1-Distillation` | `WP-1-Distillation-v2` |
| `WP-1-Editor-Hardening` | `WP-1-Editor-Hardening-v2` |
| `WP-1-Flight-Recorder-UI-v2` | `WP-1-Flight-Recorder-UI-v3` |
| `WP-1-Governance-Hooks` | `WP-1-Governance-Hooks-v2` |
| `WP-1-MCP-End-to-End` | `WP-1-MCP-End-to-End-v2` |
| `WP-1-MCP-Skeleton-Gate` | `WP-1-MCP-Skeleton-Gate-v2` |
| `WP-1-Metrics-OTel` | `WP-1-Metrics-OTel-v2` |
| `WP-1-Metrics-Traces` | `WP-1-Metrics-Traces-v2` |
| `WP-1-MEX-Observability` | `WP-1-MEX-Observability-v2` |
| `WP-1-MEX-Safety-Gates` | `WP-1-MEX-Safety-Gates-v2` |
| `WP-1-MEX-UX-Bridges` | `WP-1-MEX-UX-Bridges-v2` |
| `WP-1-Migration-Framework` | `WP-1-Migration-Framework-v2` |
| `WP-1-Model-Profiles` | `WP-1-Model-Profiles-v2` |
| `WP-1-Mutation-Traceability` | `WP-1-Mutation-Traceability-v2` |
| `WP-1-OSS-Governance` | `WP-1-OSS-Governance-v2` |
| `WP-1-PDF-Pipeline` | `WP-1-PDF-Pipeline-v2` |
| `WP-1-Photo-Studio` | `WP-1-Photo-Studio-v2` |
| `WP-1-RAG-Iterative` | `WP-1-RAG-Iterative-v2` |
| `WP-1-Semantic-Catalog` | `WP-1-Semantic-Catalog-v2` |
| `WP-1-Supply-Chain-MEX` | `WP-1-Supply-Chain-MEX-v2` |
| `WP-1-Workspace-Bundle` | `WP-1-Workspace-Bundle-v2` |

## Phase 1 Roadmap MUST Deliver -> Main Body Spec Anchors -> Task Board WPs

### P1-01 Model runtime integration (LLM core)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20250`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:10580` (4.1 LLM Infrastructure)
  - `Handshake_Master_Spec_v02.103.md:10786` (4.2 LLM Inference Runtimes)
  - `Handshake_Master_Spec_v02.103.md:11117` (4.3 Model Selection & Roles)
  - `Handshake_Master_Spec_v02.103.md:5214` (2.6.6.2.5 Runtime and Models)
- Task Board WPs:
  - `WP-1-LLM-Core-v3` (Done)
  - `WP-1-Model-Profiles` (Ready for Dev)

### P1-02 AI Job Model (minimum viable implementation)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20258`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:5107` (2.6.6 AI Job Model (Global))
  - `Handshake_Master_Spec_v02.103.md:5151` (2.6.6.2 Core Schema)
  - `Handshake_Master_Spec_v02.103.md:5254` (2.6.6.2.8 Normative Rust Types)
- Task Board WPs:
  - `WP-1-AI-Job-Model-v3` (Ready for Dev)

### P1-03 Workflow & Automation Engine (minimum viable)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20265`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:4896` (2.6 Workflow & Automation Engine)
  - `Handshake_Master_Spec_v02.103.md:5004` (2.6.4 Execution & Durability)
  - `Handshake_Master_Spec_v02.103.md:5022` (2.6.5 Safety & Validation Pipeline)
- Task Board WPs:
  - `WP-1-Workflow-Engine-v4` (Done)

### P1-04 Capability and consent enforcement (minimal slice)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20273`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:29725` (11.1 Capabilities & Consent Model)
  - `Handshake_Master_Spec_v02.103.md:5456` (2.6.6.5.2 Capabilities)
  - `Handshake_Master_Spec_v02.103.md:5744` (2.6.8.4 Gate Matrix (Normative))
- Task Board WPs:
  - `WP-1-Capability-SSoT` (Ready for Dev)
  - `WP-1-MCP-End-to-End` (Ready for Dev)
  - `WP-1-Terminal-LAW-v3` (Done)
  - `WP-1-MEX-UX-Bridges` (Ready for Dev)

### P1-05 Flight Recorder (always-on, with UI)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20282`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:910` (2.1.5 Observability & Flight Recorder)
  - `Handshake_Master_Spec_v02.103.md:26839` (10.5 Operator Consoles: Debug & Diagnostics)
  - `Handshake_Master_Spec_v02.103.md:31130` (11.4 Diagnostics Schema (Problems/Events))
  - `Handshake_Master_Spec_v02.103.md:31366` (11.5 Flight Recorder Event Shapes & Retention)
- Task Board WPs:
  - `WP-1-Flight-Recorder-v3` (Done)
  - `WP-1-Operator-Consoles-v3` (Done)
  - `WP-1-Flight-Recorder-UI-v2` (Ready for Dev)
  - `WP-1-Validator-Error-Codes-v1` (Done)

### P1-06 Baseline metrics and traces (debugging AI behaviour)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20302`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:5235` (2.6.6.2.7 Observability and Telemetry)
  - `Handshake_Master_Spec_v02.103.md:12422` (5.3 AI Observability)
  - `Handshake_Master_Spec_v02.103.md:31366` (11.5 Flight Recorder Event Shapes & Retention)
  - `Handshake_Master_Spec_v02.103.md:34513` (11.10.3 Metrics & Tokens)
- Task Board WPs:
  - `WP-1-Metrics-OTel` (Ready for Dev)
  - `WP-1-Metrics-Traces` (Ready for Dev)
  - `WP-1-Tokenization-Service-v3` (Done)

### P1-07 AI UX in the editor (basic actions)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20320`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:3615` (2.5 AI Interaction Patterns)
  - `Handshake_Master_Spec_v02.103.md:5516` (2.6.6.6.4 Docs AI Job Profile (Normative))
  - `Handshake_Master_Spec_v02.103.md:8896` (2.9 Deterministic Edit Process (COR-701))
  - `Handshake_Master_Spec_v02.103.md:24038` (10.2 Monaco Editor Experience)
  - `Handshake_Master_Spec_v02.103.md:26839` (10.5 Operator Consoles: Debug & Diagnostics)
- Task Board WPs:
  - `WP-1-AI-UX-Actions` (Ready for Dev)
  - `WP-1-AI-UX-Rewrite` (Ready for Dev)
  - `WP-1-AI-UX-Summarize-Display` (Ready for Dev)
  - `WP-1-Editor-Hardening` (Ready for Dev)

### P1-08 Governance hooks (Diary alignment)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20333`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:1697` (2.3 Content Integrity (COR-700))
  - `Handshake_Master_Spec_v02.103.md:7614` (2.8 Governance Runtime (Diary Parts 1-2))
  - `Handshake_Master_Spec_v02.103.md:8896` (2.9 Deterministic Edit Process (COR-701))
  - `Handshake_Master_Spec_v02.103.md:9486` (2.10 Session Logging (LOG-001))
- Task Board WPs:
  - `WP-1-Governance-Hooks` (Ready for Dev)

### P1-09 Dev experience and ADRs
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20338`
- Spec anchors (non-main-body):
  - `Handshake_Master_Spec_v02.103.md:19893` (7.5 Development Workflow)
- Note: Section 7 is not part of the "Main Body sections 1-6, 9-11" authority set. If this is intended as a hard Phase 1 requirement, it should be promoted into the authoritative range via Spec Enrichment.
- Task Board WPs:
  - `WP-1-Dev-Experience-ADRs` (Stub Backlog)

### P1-10 Security, resource, and UX bridges for mechanical work
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20345`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:12110` (5.2 Sandboxing & Security)
  - `Handshake_Master_Spec_v02.103.md:29929` (11.2 Sandbox Policy vs Hard Isolation)
  - `Handshake_Master_Spec_v02.103.md:17183` (6.3.0 Mechanical Tool Bus Contract (MEX v1.2))
  - `Handshake_Master_Spec_v02.103.md:32213` (11.8 Mechanical Extension Specification v1.2 (Verbatim))
  - `Handshake_Master_Spec_v02.103.md:31130` (11.4 Diagnostics Schema (Problems/Events))
- Task Board WPs:
  - `WP-1-Security-Gates-v3` (Done)
  - `WP-1-MEX-Safety-Gates` (Ready for Dev)
  - `WP-1-MEX-Observability` (Ready for Dev)
  - `WP-1-MEX-UX-Bridges` (Ready for Dev)
  - `WP-1-Supply-Chain-MEX` (Ready for Dev)

### P1-11 MCP skeleton and Gate (Target 1 + job/log plumbing)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20352`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:29936` (11.3 Auth/Session/MCP Primitives)
  - `Handshake_Master_Spec_v02.103.md:29964` (11.3.2 Implementation Target 1: The Rust Gate Interceptor)
  - `Handshake_Master_Spec_v02.103.md:31366` (11.5 Flight Recorder Event Shapes & Retention)
- Task Board WPs:
  - `WP-1-MCP-Skeleton-Gate` (Ready for Dev)
  - `WP-1-MCP-End-to-End` (Ready for Dev)

### P1-12 Calendar (local-only) as a Flight Recorder lens (no external sync)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20366`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:13195` (5.4.6.4 Calendar Law Compliance Tests)
  - `Handshake_Master_Spec_v02.103.md:25122` (10.4.1 Calendar Law (verbatim import))
  - `Handshake_Master_Spec_v02.103.md:26659` (10.4.2 Calendar <-> ACE Integration)
  - `Handshake_Master_Spec_v02.103.md:26839` (10.5 Operator Consoles: Debug & Diagnostics)
- Task Board WPs:
  - `WP-1-Calendar-Lens` (Ready for Dev)

### P1-13 OSS governance baseline (build/release enforcement)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20374`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:31601` (11.7.4 OSS Licensing, Compliance, Isolation, and Determinism)
  - `Handshake_Master_Spec_v02.103.md:31651` (11.7.5 Industry Modules & OSS Foundations Spec)
- Task Board WPs:
  - `WP-1-OSS-Register-Enforcement-v1` (Done)
  - `WP-1-OSS-Governance` (Ready for Dev)
  - `WP-1-Supply-Chain-MEX` (Ready for Dev)

### P1-14 Deliverables PDF pipeline (MVP)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20389`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:34493` (11.10.1 Deliverables PDF Pipeline (creative.deliverables.pdf_packaging))
  - `Handshake_Master_Spec_v02.103.md:31651` (11.7.5 Industry Modules & OSS Foundations Spec)
- Task Board WPs:
  - `WP-1-PDF-Pipeline` (Ready for Dev)

### P1-15 Bundle Export Framework v0 (MVP)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20394`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:27063` (10.5.6 Debug Bundle (export artifact))
  - `Handshake_Master_Spec_v02.103.md:27933` (10.5.6A Workspace Bundle Export (v0))
  - `Handshake_Master_Spec_v02.103.md:31366` (11.5 Flight Recorder Event Shapes & Retention)
- Task Board WPs:
  - `WP-1-Debug-Bundle-v3` (Done)
  - `WP-1-Workspace-Bundle` (Ready for Dev)

### P1-16 AI Rewrite UI Primitives (Human-in-the-Loop)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20398`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:24038` (10.2 Monaco Editor Experience)
  - `Handshake_Master_Spec_v02.103.md:24214` (DOC-AI-001 (The Rewrite Loop))
  - `Handshake_Master_Spec_v02.103.md:8896` (2.9 Deterministic Edit Process (COR-701))
  - `Handshake_Master_Spec_v02.103.md:5107` (2.6.6 AI Job Model (Global))
  - `Handshake_Master_Spec_v02.103.md:29725` (11.1 Capabilities & Consent Model)
- Task Board WPs:
  - `WP-1-AI-UX-Rewrite` (Ready for Dev)
  - `WP-1-Editor-Hardening` (Ready for Dev)

### P1-17 Photo Studio v0 (skeleton surface + governance wiring)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20404`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:28471` (10.10 Photo Studio)
  - `Handshake_Master_Spec_v02.103.md:17825` (6.3.3.6 Darkroom Engine (Photo Stack))
  - `Handshake_Master_Spec_v02.103.md:32016` (11.7.6 Photo Stack OSS Component Matrix (Photo Studio))
  - `Handshake_Master_Spec_v02.103.md:32213` (11.8 Mechanical Extension Specification v1.2 (Verbatim))
- Task Board WPs:
  - `WP-1-Photo-Studio` (Ready for Dev)

### P1-18 Spec Router and governance session log (MVP)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20408`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:5548` (2.6.6.6.5 Spec Router Job Profile (Normative))
  - `Handshake_Master_Spec_v02.103.md:5615` (2.6.8 Prompt-to-Spec Governance Pipeline (Normative))
  - `Handshake_Master_Spec_v02.103.md:5778` (2.6.8.5 Prompt-to-Spec Router (Normative))
  - `Handshake_Master_Spec_v02.103.md:5920` (2.6.8.8 Spec Session Log (Normative))
  - `Handshake_Master_Spec_v02.103.md:29862` (11.1.5 Spec Router Policy (Normative))
- Task Board WPs:
  - `WP-1-Spec-Router-Session-Log` (Stub Backlog)

### P1-19 ACE Runtime (MVP) + Validator Pack (CI-gated)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20428`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:6001` (2.6.6.7 ACE Runtime (Agentic Context Engineering))
  - `Handshake_Master_Spec_v02.103.md:6408` (2.6.6.7.11 Validators (runtime-enforced; required))
  - `Handshake_Master_Spec_v02.103.md:27063` (10.5.6 Debug Bundle (export artifact))
  - `Handshake_Master_Spec_v02.103.md:12422` (5.3 AI Observability)
- Task Board WPs:
  - `WP-1-ACE-Validators-v4` (Done)
  - `WP-1-ACE-Runtime` (Ready for Dev)
  - `WP-1-ACE-Auditability` (Ready for Dev)
  - `WP-1-Gate-Check-Tool-v2` (Done)
  - `WP-1-Validator-Error-Codes-v1` (Done)

### P1-20 Terminal LAW (minimal slice) promoted to MUST
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20433`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:23757` (10.1 Terminal Experience)
  - `Handshake_Master_Spec_v02.103.md:31580` (11.7.1 Terminal Engine / PTY / Sandbox)
  - `Handshake_Master_Spec_v02.103.md:29929` (11.2 Sandbox Policy vs Hard Isolation)
  - `Handshake_Master_Spec_v02.103.md:29936` (11.3 Auth/Session/MCP Primitives)
- Task Board WPs:
  - `WP-1-Terminal-LAW-v3` (Done)

### P1-21 Capability single-source-of-truth + unknown-capability validator
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20438`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:29725` (11.1 Capabilities & Consent Model)
  - `Handshake_Master_Spec_v02.103.md:29902` (11.1.6 Capability Registry Generation Workflow (Normative))
  - `Handshake_Master_Spec_v02.103.md:5456` (2.6.6.5.2 Capabilities)
- Task Board WPs:
  - `WP-1-Capability-SSoT` (Ready for Dev)

### P1-22 Canvas Typography + Font Packs (Design Pack + Font Registry)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20443`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:28015` (10.6 Canvas: Typography & Font Packs)
  - `Handshake_Master_Spec_v02.103.md:34502` (11.10.2 Canvas Typography & Font Registry)
- Task Board WPs:
  - `WP-1-Canvas-Typography` (Ready for Dev)

### P1-23 Iterative Deepening (snippet-first) - MVP policy scaffolding
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20451`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:6190` (2.6.6.7.5.2 Iterative Deepening (Snippet-First Retrieval))
- Task Board WPs:
  - `WP-1-RAG-Iterative` (Ready for Dev)

### P1-24 Retrieval Correctness & Efficiency (ACE-RAG-001) - Phase 1 plumbing
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20456`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:6514` (2.6.6.7.14 Retrieval Correctness & Efficiency (ACE-RAG-001) (normative))
  - `Handshake_Master_Spec_v02.103.md:26839` (10.5 Operator Consoles: Debug & Diagnostics)
  - `Handshake_Master_Spec_v02.103.md:31366` (11.5 Flight Recorder Event Shapes & Retention)
- Task Board WPs:
  - `WP-1-ACE-Runtime` (Ready for Dev)
  - `WP-1-ACE-Auditability` (Ready for Dev)
  - `WP-1-RAG-Iterative` (Ready for Dev)
  - `WP-1-Semantic-Catalog` (Ready for Dev)

### P1-25 Atelier Lens Runtime v0.1 (Role claiming + dual-contract extraction)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20467`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:17405` (6.3.3.5 Atelier Engine)
  - `Handshake_Master_Spec_v02.103.md:17595` (6.3.3.5.7 Atelier Role Dual-Contract Runtime)
  - `Handshake_Master_Spec_v02.103.md:6001` (2.6.6.7 ACE Runtime (Agentic Context Engineering))
- Task Board WPs:
  - `WP-1-Atelier-Lens` (Ready for Dev)

### P1-26 Mechanical Extension v1.2 runtime contract (MEX) - Phase 1 foundations
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20481`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:17183` (6.3.0 Mechanical Tool Bus Contract (MEX v1.2))
  - `Handshake_Master_Spec_v02.103.md:32213` (11.8 Mechanical Extension Specification v1.2 (Verbatim))
  - `Handshake_Master_Spec_v02.103.md:12110` (5.2 Sandboxing & Security)
- Task Board WPs:
  - `WP-1-MEX-v1.2-Runtime-v3` (Done)
  - `WP-1-MEX-Safety-Gates` (Ready for Dev)
  - `WP-1-MEX-Observability` (Ready for Dev)
  - `WP-1-MEX-UX-Bridges` (Ready for Dev)

### P1-27 Phase 1 closure: storage backend portability work packets (CX-DBP-030)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20491`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:3098` (2.3.12.5 Phase 1 Closure Requirements [CX-DBP-030])
- Task Board WPs:
  - `WP-1-Storage-Foundation-v3` (Done)
  - `WP-1-Storage-Abstraction-Layer-v3` (Done)
  - `WP-1-AppState-Refactoring-v3` (Done)
  - `WP-1-Dual-Backend-Tests-v2` (Done)
  - `WP-1-Migration-Framework` (Ready for Dev)

### P1-28 CapabilityRegistry single source of truth (WP-1-Capability-SSoT)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20498`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:29902` (11.1.6 Capability Registry Generation Workflow (Normative))
  - `Handshake_Master_Spec_v02.103.md:29725` (11.1 Capabilities & Consent Model)
- Task Board WPs:
  - `WP-1-Capability-SSoT` (Ready for Dev)

### P1-29 Global Silent Edit Guard (WP-1-Global-Silent-Edit-Guard)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20501`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:8962` (2.9.3 Mutation Traceability (normative))
  - `Handshake_Master_Spec_v02.103.md:9002` (2.9.3.2 Storage Guard Trait)
- Task Board WPs:
  - `WP-1-Global-Silent-Edit-Guard` (Stub Backlog)
  - `WP-1-Mutation-Traceability` (Ready for Dev)

### P1-30 Phase 1 final gap closure details (Section 11.10)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20504`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:34489` (11.10 Implementation Notes: Phase 1 Final Gaps)
  - `Handshake_Master_Spec_v02.103.md:34523` (11.10.4 Phase 1 Final Gap Closure (Calendar, OSS, Validators))
- Task Board WPs (non-exhaustive; explicit Phase 1 "final gap" clusters tracked on the board):
  - `WP-1-Calendar-Lens` (Ready for Dev)
  - `WP-1-OSS-Governance` (Ready for Dev)
  - `WP-1-PDF-Pipeline` (Ready for Dev)
  - `WP-1-Canvas-Typography` (Ready for Dev)
  - `WP-1-Metrics-OTel` (Ready for Dev)
  - `WP-1-ACE-Validators-v4` (Done)
  - `WP-1-ACE-Runtime` (Ready for Dev)

### P1-31 Response Behavior Contract (Diary ANS-001)
- Roadmap item: `Handshake_Master_Spec_v02.103.md:20509`
- Spec anchors:
  - `Handshake_Master_Spec_v02.103.md:6917` (2.7 Response Behavior Contract (Diary ANS-001))
  - `Handshake_Master_Spec_v02.103.md:7614` (2.8 Governance Runtime (Diary Parts 1-2))
  - `Handshake_Master_Spec_v02.103.md:8745` (2.8.v02.13 ANS-001 Invocation (EXEC-057 to EXEC-060))
- Task Board WPs:
  - `WP-1-Response-Behavior-ANS-001` (Stub Backlog)

---

## SPEC_ANCHOR hygiene flags (Task Board)
These are Task Board WPs whose *task packet* SPEC_ANCHOR still points at Roadmap sections (`7.6.*`) instead of Main Body anchors.

### Roadmap-only SPEC_ANCHOR (needs Main Body anchors)
- `WP-1-AI-UX-Actions`
- `WP-1-AI-UX-Rewrite`
- `WP-1-Calendar-Lens`
- `WP-1-Canvas-Typography`
- `WP-1-Distillation`
- `WP-1-Editor-Hardening`
- `WP-1-Governance-Hooks`
- `WP-1-MEX-Observability`
- `WP-1-MEX-Safety-Gates`
- `WP-1-MEX-UX-Bridges`
- `WP-1-Metrics-OTel`

### Includes Roadmap + Main Body in SPEC_ANCHOR (still contains Roadmap pointers)
- `WP-1-Atelier-Lens`
- `WP-1-Model-Profiles`
- `WP-1-OSS-Governance`
- `WP-1-PDF-Pipeline`
- `WP-1-RAG-Iterative`
- `WP-1-Terminal-LAW`

