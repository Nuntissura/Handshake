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

This board provides an exhaustive tracking of all Roadmap items from A7.6.3. Phase 1 cannot close until every item below is validated against the current Master Spec (see `docs/SPEC_CURRENT.md`).

**Task Board entry format (enforced for In Progress/Done/Superseded via `just task-board-check`):**
- In Progress: `- **[WP_ID]** - [IN_PROGRESS]`
- Done: `- **[WP_ID]** - [VALIDATED|FAIL|OUTDATED_ONLY]`
- Superseded: `- **[WP_ID]** - [SUPERSEDED]`
Keep details (failure reasons, commands, evidence, \"SUPERSEDED by ...\") in the task packet to avoid drift/noise.

---


## Ready for Dev
- **[WP-1-Capability-SSoT]** / FAIL (revalidation): `just post-work WP-1-Capability-SSoT` fails (C701-G05 post_sha1 mismatch) for `src/backend/handshake_core/src/capabilities.rs`. [READY FOR DEV]
- **[WP-1-LLM-Core]** / FAIL (revalidation): `just post-work WP-1-LLM-Core` fails phase gate (SKELETON appears before BOOTSTRAP); packet non-ASCII + missing COR-701 manifest. [READY FOR DEV]
- **[WP-1-Flight-Recorder-UI-v2]** / FAIL (revalidation): `just gate-check WP-1-Flight-Recorder-UI-v2` fails (missing "SKELETON APPROVED" marker); `node scripts/validation/post-work-check.mjs WP-1-Flight-Recorder-UI-v2` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99; user signature field missing/pending. [READY FOR DEV]
- **[WP-1-ACE-Validators-v3]** / FAIL (revalidation): `just post-work WP-1-ACE-Validators-v3` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99; user signature field missing; TASK_BOARD was inconsistent with packet status history. [READY FOR DEV]
- **[WP-1-AI-Job-Model-v3]** / FAIL (revalidation): `just post-work WP-1-AI-Job-Model-v3` fails phase gate (missing "SKELETON APPROVED" marker); `node scripts/validation/post-work-check.mjs WP-1-AI-Job-Model-v3` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99; packet already contains a prior FAIL section; spec updated in v02.99 to include Stalled and expanded JobKind, revalidate against new list. [READY FOR DEV]
- **[WP-1-AppState-Refactoring-v2]** / FAIL (revalidation): `just post-work WP-1-AppState-Refactoring-v2` fails phase gate (missing "SKELETON APPROVED" marker); `node scripts/validation/post-work-check.mjs WP-1-AppState-Refactoring-v2` fails (non-ASCII packet + missing COR-701 manifest). Packet references v02.93 not v02.99. [READY FOR DEV]
- **[WP-1-Flight-Recorder]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; recheck A11.5 retention/telemetry. [READY FOR DEV]
- **[WP-1-ACE-Runtime]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; refresh ACE-RAG-001 evidence. [READY FOR DEV]
- **[WP-1-Mutation-Traceability]** / FAIL: Spec drift vs SPEC_CURRENT v02.94; revalidate A2.9.3. [READY FOR DEV]
- **[WP-1-Dual-Backend-Tests-v2]** / FAIL (validation): Deterministic manifest gate incomplete; see task packet VALIDATION_REPORTS. [READY FOR DEV]
- **[WP-1-Dual-Backend-Tests]** - Superseded by WP-1-Dual-Backend-Tests-v2. [SUPERSEDED]
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

Assignee/model is recorded in the task packet (CODER_MODEL, CODER_REASONING_STRENGTH). Task Board stays minimal.


## Done
- **[WP-1-LLM-Core-v3]** - [VALIDATED]
- **[WP-1-Flight-Recorder-v3]** - [VALIDATED]
- **[WP-1-OSS-Register-Enforcement-v1]** - [VALIDATED]
- **[WP-1-Tokenization-Service-v3]** - [VALIDATED]
- **[WP-1-Security-Gates-v3]** - [VALIDATED]
- **[WP-1-Gate-Check-Tool-v2]** - [VALIDATED]
- **[WP-1-Workflow-Engine-v4]** - [VALIDATED]
- **[WP-1-Debug-Bundle-v3]** - [VALIDATED]
- **[WP-1-Validator-Error-Codes-v1]** - [VALIDATED]
- **[WP-1-Storage-Foundation-v3]** - [VALIDATED]
- **[WP-1-Terminal-LAW-v3]** - [VALIDATED]
- **[WP-1-MEX-v1.2-Runtime-v3]** - [VALIDATED]
- **[WP-1-Operator-Consoles-v3]** - [VALIDATED]



## Blocked

---

## Superseded (Archive)
- **[WP-1-Tokenization-Service-20251228]** - [SUPERSEDED]
- **[WP-1-Storage-Foundation-20251228]** - [SUPERSEDED]
- **[WP-1-Gate-Check-Tool]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles-v2]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles-v1]** - [SUPERSEDED]
- **[WP-1-Operator-Consoles]** - [SUPERSEDED]
- **[WP-1-Diagnostic-Pipe]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-v2]** - [SUPERSEDED]
- **[WP-1-Workflow-Engine-v3]** - [SUPERSEDED]
- **[WP-1-Workflow-Engine-v2]** - [SUPERSEDED]
- **[WP-1-AI-Job-Model-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Validators-v2]** - [SUPERSEDED]
- **[WP-1-Security-Gates]** - [SUPERSEDED]
- **[WP-1-Security-Gates-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW-v2]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle-v2]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle]** - [SUPERSEDED]
