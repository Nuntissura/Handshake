# Handshake Project Task Board (Phase 1: EXHAUSTIVE STRATEGIC AUDIT)

This board provides an exhaustive tracking of all Roadmap items from §7.6.3. 
**Phase 1 cannot close until every item below is VALIDATED ✅.**

---

## 🚨 PHASE 1 CLOSURE GATES (ROADMAP ALIGNMENT)

### Core Foundations
1. **[WP-1-Storage-Foundation]**: trait-based storage, portable SQL, dual-backend testing. [VALIDATED ✅]
2. **[WP-1-Gate-Check-Tool]**: automated binary phase gate validator. [VALIDATED ✅]
3. **[WP-1-Tokenization-Service]**: model-aware BPE counting and budgeting. [VALIDATED ✅]

---

## Roadmap Audit (Code Archaeology) 🔍
*All items below are mandatory Phase 1 deliverables currently under archaeology/validation.*

### Infrastructure & Engineering
- **[WP-1-LLM-Core]**: Ollama integration, preloaded model config. [PENDING 🔍]
- **[WP-1-AI-Job-Model]**: Global job schema, Doc profile subset. [PENDING 🔍]
- **[WP-1-Workflow-Engine]**: Persistence, crash recovery, node status. [PENDING 🔍]
- **[WP-1-Capability-SSoT]**: Centralized Registry, unknown-capability validator. [PASS ✅ - Ready for Validation]
- **[WP-1-Flight-Recorder]**: DuckDB log store, model call tagging. [PARTIAL 🟡]
- **[WP-1-Operator-Consoles]**: Timeline, Jobs, Problems, Evidence UI. [PARTIAL 🟡]
- **[WP-1-Metrics-OTel]**: OpenTelemetry instrumentation, simple trace IDs. [FAIL 🔴]
- **[WP-1-Diagnostic-Pipe]**: DIAG-SCHEMA fingerprinting and grouping. [FAIL 🔴]
- **[WP-1-OSS-Governance]**: Component Register, Copyleft isolation. [PARTIAL 🟡]
- **[WP-1-Supply-Chain-MEX]**: MEX v1.2 Security Gates (gitleaks, osv-scanner). [FAIL 🔴]

### ACE Runtime & RAG (ACE-RAG-001)
- **[WP-1-ACE-Auditability]**: ContextPlan, ContextSnapshot artifacts. [FAIL 🔴]
- **[WP-1-ACE-Validators]**: 12 Runtime Validators (§2.6.6.7.11). [FAIL 🔴]
- **[WP-1-ACE-RAG-Plumbing]**: QueryPlan, RetrievalTrace, hard budgets. [FAIL 🔴]
- **[WP-1-RAG-Iterative]**: Snippet-first policy, search->read separation. [FAIL 🔴]
- **[WP-1-Model-Profiles]**: ModelProfile/Routing/SafetyProfile schema. [FAIL 🔴]

### Mechanical & Terminal
- **[WP-1-Terminal-LAW]**: Hardened execution, timeout, session binding. [PASS ✅ - Ready for Validation]
- **[WP-1-MEX-Safety-Gates]**: Guard, Container, Quota engines. [FAIL 🔴]
- **[WP-1-MEX-Observability]**: Profiler, Monitor, Repo, Formatter engines. [FAIL 🔴]
- **[WP-1-MEX-UX-Bridges]**: Clipboard and Notifier capability actions. [FAIL 🔴]
- **[WP-1-MEX-v1.2-Runtime]**: Engine registry, Conformance Harness. [FAIL 🔴]
- **[WP-1-MCP-Skeleton-Gate]**: MCP transport, Gate middleware. [FAIL 🔴]

### UX & Creative Surface
- **[WP-1-AI-UX-Actions]**: Command Palette: "Ask", "Summarize". [PENDING 🔍]
- **[WP-1-AI-UX-Rewrite]**: Rewrite selection, structured patches, Diff view. [PENDING 🔍]
- **[WP-1-Editor-Hardening]**: Tiptap/Excalidraw "No Silent Edits". [PENDING 🔍]
- **[WP-1-Canvas-Typography]**: Font Registry, offline packs, no flash. [FAIL 🔴]
- **[WP-1-PDF-Pipeline]**: Typst + qpdf deliverable packaging. [FAIL 🔴]
- **[WP-1-Photo-Studio]**: Skeleton surface, thumbnails, recipes. [FAIL 🔴]
- **[WP-1-Atelier-Lens]**: Role claiming, SceneState, ConflictSet. [FAIL 🔴]

### Bundles & Distillation
- **[WP-1-Debug-Bundle]**: Redacted repro packets (§7.6.3.5). [FAIL 🔴]
- **[WP-1-Workspace-Bundle]**: Backup/transfer export. [FAIL 🔴]
- **[WP-1-Calendar-Lens]**: Local ActivitySpan selection UI. [FAIL 🔴]
- **[WP-1-Distillation]**: teacher metadata, Skill Bank schema. [FAIL 🔴]
- **[WP-1-Governance-Hooks]**: Diary RID mapping, CI compliance. [PENDING 🔍]

---

## Ready for Dev
(None - Audit in Progress)

## Done
- **[WP-1-Storage-Foundation]** [VALIDATED ✅]
- **[WP-1-Gate-Check-Tool]** [VALIDATED ✅]
- **[WP-1-Tokenization-Service]** [VALIDATED ✅]
- **[WP-Test-Sample]** [VALIDATED ✅]
- **[WP-Codex-v0.8]** [VALIDATED ✅]
