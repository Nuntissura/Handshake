# Handshake Project Task Board (Phase 1: EXHAUSTIVE STRATEGIC AUDIT)

## Spec Authority Rule [CX-598] (HARD INVARIANT)

**The Roadmap (Section 7.6) is ONLY a pointer. The Master Spec Main Body (Sections 1-6, 9-11) is the SOLE definition of "Done."**

| Principle | Enforcement |
|-----------|-------------|
| **Roadmap = Pointer** | Section 7.6.x items point to Main Body sections where requirements are defined |
| **Main Body = Truth** | Every MUST/SHOULD in Sections 1-6, 9-11 must be implemented - no exceptions |
| **No Debt** | Skipping requirements poisons the project; later phases inherit rotten foundations |
| **No Phase Closes** | Until ALL referenced Main Body requirements are VALIDATED |

**Why:** Handshake is complex software. Treating roadmap bullets as requirements (instead of pointers) leads to surface-level compliance, technical debt, and project failure.

This board provides an exhaustive tracking of all Roadmap items from A7.6.3. Phase 1 cannot close until every item below is validated against Master Spec v02.96.

---


## Ready for Dev
- **[WP-1-Flight-Recorder]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; recheck A11.5 retention/telemetry. [READY FOR DEV]
- **[WP-1-ACE-Runtime]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; refresh ACE-RAG-001 evidence. [READY FOR DEV]
- **[WP-1-Mutation-Traceability]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; revalidate A2.9.3. [READY FOR DEV]
- **[WP-1-Dual-Backend-Tests]** / FAIL: Missing STATUS/SPEC_CURRENT; re-anchor to v02.94 A2.3.12. [READY FOR DEV]
- **[WP-1-Flight-Recorder-UI]** / FAIL (SUPERSEDED by v2): DuckDB log store + UI wiring. [READY FOR DEV]
- **[WP-1-Operator-Consoles]** - Superseded by v1 (comprehensive rewrite anchored to v02.96). [SUPERSEDED]
- **[WP-1-Metrics-OTel]** / FAIL: OpenTelemetry instrumentation. [READY FOR DEV]
- **[WP-1-Diagnostic-Pipe]** - Absorbed into WP-1-Operator-Consoles-v1 (DIAG-SCHEMA is prerequisite). [SUPERSEDED]
- **[WP-1-OSS-Governance]** / FAIL: Component Register, Copyleft isolation. [READY FOR DEV]
- **[WP-1-Supply-Chain-MEX]** / FAIL: MEX v1.2 Security Gates (gitleaks, osv-scanner). [READY FOR DEV]
- **[WP-1-ACE-Auditability]** / FAIL: ContextPlan, ContextSnapshot artifacts. [READY FOR DEV]
- **[WP-1-ACE-Validators]** / FAIL: 12 Runtime Validators (A2.6.6.7.11). [READY FOR DEV]
- **[WP-1-RAG-Iterative]** / FAIL: Snippet-first policy, search->read separation. [READY FOR DEV]
- **[WP-1-Model-Profiles]** / FAIL: ModelProfile/Routing/SafetyProfile schema. [READY FOR DEV]
- **[WP-1-MEX-Safety-Gates]** / FAIL: Guard, Container, Quota engines. [READY FOR DEV]
- **[WP-1-MEX-Observability]** / FAIL: Profiler, Monitor, Repo, Formatter engines. [READY FOR DEV]
- **[WP-1-MEX-UX-Bridges]** / FAIL: Clipboard and Notifier capability actions. [READY FOR DEV]
- **[WP-1-MCP-Skeleton-Gate]** / FAIL: MCP transport, Gate middleware. [READY FOR DEV]
- **[WP-1-AI-UX-Actions]** / FAIL: Command Palette: "Ask", "Summarize". [READY FOR DEV]
- **[WP-1-AI-UX-Rewrite]** / FAIL: Rewrite selection, structured patches, Diff view. [READY FOR DEV]
- **[WP-1-AI-UX-Summarize-Display]** / FAIL: Align with v02.84 spec; add Evidence Mapping. [READY FOR DEV]
- **[WP-1-Editor-Hardening]** / FAIL: Tiptap/Excalidraw "No Silent Edits". [READY FOR DEV]
- **[WP-1-Canvas-Typography]** / FAIL: Font Registry, offline packs, no flash. [READY FOR DEV]
- **[WP-1-PDF-Pipeline]** / FAIL: Typst + qpdf deliverable packaging. [READY FOR DEV]
- **[WP-1-Photo-Studio]** / FAIL: Skeleton surface, thumbnails, recipes. [READY FOR DEV]
- **[WP-1-Atelier-Lens]** / FAIL: Role claiming, SceneState, ConflictSet. [READY FOR DEV]
- **[WP-1-Calendar-Lens]** / FAIL: Local ActivitySpan selection UI. [READY FOR DEV]
- **[WP-1-Distillation]** / FAIL: Teacher metadata, Skill Bank schema. [READY FOR DEV]
- **[WP-1-Governance-Hooks]** / FAIL: Diary RID mapping, CI compliance. [READY FOR DEV]
- **[WP-1-Workspace-Bundle]** / FAIL: Backup/transfer export. [READY FOR DEV]
- **[WP-1-Semantic-Catalog]** / FAIL: Implement SemanticCatalog per A2.6.7. [READY FOR DEV]
- **[WP-1-MCP-End-to-End]** / FAIL: MCP capability/logging chain. [READY FOR DEV]
- **[WP-1-Metrics-Traces]** / FAIL: OTel metrics/traces + validator pack. [READY FOR DEV]


## In Progress


## Done
- **[WP-1-Debug-Bundle-v2]** - Debug Bundle export per 10.5.6.5-12 (schemas, trait, API, redactor, UI). [VALIDATED]
- **[WP-1-Operator-Consoles-v1]** - Operator Consoles v1 per §10.5 + DIAG-SCHEMA §11.4 (Problems/Jobs/Timeline/Evidence). [VALIDATED]
- **[WP-1-MEX-v1.2-Runtime-v2]** - MEX v1.2 runtime contract (envelopes, gates, registry) per §6.3.0 + §11.8. [VALIDATED]
- **[WP-1-Terminal-LAW-v2]** - Session types + AI isolation per §10.1. [VALIDATED]
- **[WP-1-Security-Gates-v2]** - Terminal/RCE guardrails per §10.1. [VALIDATED]
- **[WP-1-Capability-SSoT]** - Centralized Capability Registry SSoT (A11.1). [VALIDATED]
- **[WP-1-Flight-Recorder-v2]** - Upgrading Observation Surface (A11.5). [VALIDATED]
- **[WP-1-Tokenization-Service-v2]** - Normative Tokenizer Trait (A4.6.1) with panic-free fallback. [VALIDATED]
- **[WP-1-Storage-Foundation-v2]** - Enforcing Trait Purity (A2.3.12.3) and Mandatory Audit. [VALIDATED]
- **[WP-1-Flight-Recorder-UI-v2]** - Upgrading Observation Surface (A11.5). [VALIDATED]
- **[WP-1-Workflow-Engine-v3]** - Mandatory Startup Recovery Loop (A2.6.1). [VALIDATED]
- **[WP-1-ACE-Validators-v3]** - Runtime Validators hardened (A2.6.6.7.11). [VALIDATED]
- **[WP-1-AI-Job-Model-v3]** - Hardened Job Model (Enums, Metrics Integrity, Poisoning Trap). [VALIDATED]
- **[WP-1-Storage-Abstraction-Layer-v2]** [VALIDATED]
- **[WP-1-LLM-Core]** - LLM Core Adapter (A4.2.3). [VALIDATED]
- **[WP-1-Gate-Check-Tool]** [VALIDATED]
- **[WP-1-AppState-Refactoring-v2]** - Enforcing Trait Purity (A2.3.12.3). [VALIDATED]
## Blocked

---

## Superseded (Archive)
- **[WP-1-Operator-Consoles]** - Superseded by v1 (comprehensive rewrite anchored to v02.96 §10.5 + §11.4).
- **[WP-1-Diagnostic-Pipe]** - Absorbed into WP-1-Operator-Consoles-v1 (DIAG-SCHEMA §11.4 is prerequisite component).
- **[WP-1-Flight-Recorder]** - Superseded by v2 (Spec alignment §11.5).
- **[WP-1-Workflow-Engine-v2]** - Superseded by v3 (Audit remediation).
- **[WP-1-AI-Job-Model-v2]** - Superseded by v3 (Spec alignment §2.6.6.2.8).
- **[WP-1-ACE-Validators-v2]** - Superseded by v3 (Hardened security remediation).
- **[WP-1-Security-Gates]** - Superseded by v2 (Spec drift v02.84 → v02.96).
- **[WP-1-Terminal-LAW]** - Superseded by v2 (Stale SPEC_ANCHOR, incomplete structure).
- **[WP-1-MEX-v1.2-Runtime]** - Superseded by v2 (Stale SPEC_ANCHOR v02.84, no implementation).
- **[WP-1-Debug-Bundle]** - Superseded by v2 (Stale SPEC_ANCHOR v02.84, comprehensive rewrite with 10.5.6.5-12 enrichment).