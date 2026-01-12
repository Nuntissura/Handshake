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

**Backlog stubs (pre-activation):**
- Track not-yet-activated work as STUB items (no USER_SIGNATURE yet). Details live in `docs/task_packets/stubs/`.
- Stubs MUST be activated into official task packets before any coding starts (see `docs/ORCHESTRATOR_PROTOCOL.md`).
- Base WP ↔ packet revision mapping (v2/v3/v4) is tracked in `docs/WP_TRACEABILITY_REGISTRY.md`.

---

## Active (Cross-Branch Status)

This section exists to keep the Operator's **main-branch** Task Board up to date when multiple Coders are working in separate WP branches/worktrees.

Rules:
- This section is informational for visibility across branches (who is working on what).
- Do NOT use `[IN_PROGRESS]` here (that token is reserved for the script-checked `## In Progress` list).
- Validator maintains this section on `main` via small docs-only "status sync" commits.

Entry format (recommended):
- `- **[WP_ID]** - [ACTIVE] - branch: feat/WP-{ID} - coder: <name/model> - last_sync: YYYY-MM-DD`

---


## Ready for Dev

A WP is only Ready for Dev if its Active Packet (per `docs/WP_TRACEABILITY_REGISTRY.md`) is an official packet under `docs/task_packets/` (not a stub).


## Stub Backlog (Not Activated)
- **[WP-1-Governance-Kernel-Conformance-v1]** - [STUB]
- **[WP-1-Cross-Tool-Interaction-Conformance-v1]** - [STUB]
- **[WP-1-LocalFirst-Agentic-MCP-Posture-v1]** - [STUB]
- **[WP-1-Spec-Router-Session-Log]** - [STUB]
- **[WP-1-Dev-Experience-ADRs]** - [STUB]
- **[WP-1-Global-Silent-Edit-Guard]** - [STUB]
- **[WP-1-Response-Behavior-ANS-001]** - [STUB]
- **[WP-1-Capability-SSoT-v2]** - [STUB]
- **[WP-1-Flight-Recorder-UI-v3]** - [STUB]
- **[WP-1-AI-Job-Model-v4]** - [STUB]
- **[WP-1-ACE-Runtime-v2]** - [STUB]
- **[WP-1-Mutation-Traceability-v2]** - [STUB]
- **[WP-1-Metrics-OTel-v2]** - [STUB]
- **[WP-1-OSS-Governance-v2]** - [STUB]
- **[WP-1-Supply-Chain-MEX-v2]** - [STUB]
- **[WP-1-ACE-Auditability-v2]** - [STUB]
- **[WP-1-RAG-Iterative-v2]** - [STUB]
- **[WP-1-Model-Profiles-v2]** - [STUB]
- **[WP-1-MEX-Safety-Gates-v2]** - [STUB]
- **[WP-1-MEX-Observability-v2]** - [STUB]
- **[WP-1-MEX-UX-Bridges-v2]** - [STUB]
- **[WP-1-MCP-Skeleton-Gate-v2]** - [STUB]
- **[WP-1-AI-UX-Actions-v2]** - [STUB]
- **[WP-1-AI-UX-Rewrite-v2]** - [STUB]
- **[WP-1-AI-UX-Summarize-Display-v2]** - [STUB]
- **[WP-1-Editor-Hardening-v2]** - [STUB]
- **[WP-1-Canvas-Typography-v2]** - [STUB]
- **[WP-1-PDF-Pipeline-v2]** - [STUB]
- **[WP-1-Photo-Studio-v2]** - [STUB]
- **[WP-1-Atelier-Lens-v2]** - [STUB]
- **[WP-1-Calendar-Lens-v2]** - [STUB]
- **[WP-1-Distillation-v2]** - [STUB]
- **[WP-1-Governance-Hooks-v2]** - [STUB]
- **[WP-1-Workspace-Bundle-v2]** - [STUB]
- **[WP-1-Semantic-Catalog-v2]** - [STUB]
- **[WP-1-MCP-End-to-End-v2]** - [STUB]
- **[WP-1-Metrics-Traces-v2]** - [STUB]


## In Progress

Assignee/model is recorded in the task packet (CODER_MODEL, CODER_REASONING_STRENGTH). Task Board stays minimal.


## Done
- **[WP-1-Migration-Framework-v2]** - [VALIDATED]
- **[WP-1-ACE-Validators-v4]** - [VALIDATED]
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
- **[WP-1-Storage-Abstraction-Layer-v3]** - [VALIDATED]
- **[WP-1-AppState-Refactoring-v3]** - [VALIDATED]
- **[WP-1-Dual-Backend-Tests-v2]** - [VALIDATED]
- **[WP-1-Terminal-LAW-v3]** - [VALIDATED]
- **[WP-1-MEX-v1.2-Runtime-v3]** - [VALIDATED]
- **[WP-1-Operator-Consoles-v3]** - [VALIDATED]



## Blocked

---

## Superseded (Archive)
- **[WP-1-AppState-Refactoring]** - [SUPERSEDED]
- **[WP-1-AppState-Refactoring-v2]** - [SUPERSEDED]
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
- **[WP-1-ACE-Validators]** - [SUPERSEDED]
- **[WP-1-ACE-Validators-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Validators-v3]** - [SUPERSEDED]
- **[WP-1-Dual-Backend-Tests]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-UI]** - [SUPERSEDED]
- **[WP-1-LLM-Core]** - [SUPERSEDED]
- **[WP-1-Security-Gates]** - [SUPERSEDED]
- **[WP-1-Security-Gates-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW-v2]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime-v2]** - [SUPERSEDED]
- **[WP-1-Terminal-LAW]** - [SUPERSEDED]
- **[WP-1-MEX-v1.2-Runtime]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle-v2]** - [SUPERSEDED]
- **[WP-1-Debug-Bundle]** - [SUPERSEDED]
- **[WP-1-Storage-Abstraction-Layer]** - [SUPERSEDED]
- **[WP-1-Storage-Abstraction-Layer-v2]** - [SUPERSEDED]
- **[WP-1-ACE-Auditability]** - [SUPERSEDED]
- **[WP-1-ACE-Runtime]** - [SUPERSEDED]
- **[WP-1-AI-Job-Model-v3]** - [SUPERSEDED]
- **[WP-1-AI-UX-Actions]** - [SUPERSEDED]
- **[WP-1-AI-UX-Rewrite]** - [SUPERSEDED]
- **[WP-1-AI-UX-Summarize-Display]** - [SUPERSEDED]
- **[WP-1-Atelier-Lens]** - [SUPERSEDED]
- **[WP-1-Calendar-Lens]** - [SUPERSEDED]
- **[WP-1-Canvas-Typography]** - [SUPERSEDED]
- **[WP-1-Capability-SSoT]** - [SUPERSEDED]
- **[WP-1-Distillation]** - [SUPERSEDED]
- **[WP-1-Editor-Hardening]** - [SUPERSEDED]
- **[WP-1-Flight-Recorder-UI-v2]** - [SUPERSEDED]
- **[WP-1-Governance-Hooks]** - [SUPERSEDED]
- **[WP-1-MCP-End-to-End]** - [SUPERSEDED]
- **[WP-1-MCP-Skeleton-Gate]** - [SUPERSEDED]
- **[WP-1-Metrics-OTel]** - [SUPERSEDED]
- **[WP-1-Metrics-Traces]** - [SUPERSEDED]
- **[WP-1-MEX-Observability]** - [SUPERSEDED]
- **[WP-1-MEX-Safety-Gates]** - [SUPERSEDED]
- **[WP-1-MEX-UX-Bridges]** - [SUPERSEDED]
- **[WP-1-Migration-Framework]** - [SUPERSEDED]
- **[WP-1-Model-Profiles]** - [SUPERSEDED]
- **[WP-1-Mutation-Traceability]** - [SUPERSEDED]
- **[WP-1-OSS-Governance]** - [SUPERSEDED]
- **[WP-1-PDF-Pipeline]** - [SUPERSEDED]
- **[WP-1-Photo-Studio]** - [SUPERSEDED]
- **[WP-1-RAG-Iterative]** - [SUPERSEDED]
- **[WP-1-Semantic-Catalog]** - [SUPERSEDED]
- **[WP-1-Supply-Chain-MEX]** - [SUPERSEDED]
- **[WP-1-Workspace-Bundle]** - [SUPERSEDED]
