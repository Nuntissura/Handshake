# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Unified-Tool-Surface-Contract-v1

## STUB_METADATA
- WP_ID: WP-1-Unified-Tool-Surface-Contract-v1
- BASE_WP_ID: WP-1-Unified-Tool-Surface-Contract
- CREATED_AT: 2026-02-23T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> [ADD v02.136] Unify tool invocation: local tool calling and MCP MUST use the same Tool Registry + Tool Gate + Flight Recorder event model (no bypass).
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 6.0.2 Unified Tool Surface Contract (Local Tool Calling + MCP) (Normative)
  - Handshake_Master_Spec_v02.137.md 6.0.2.5 Canonical invocation envelope (HTC-1.0) (MUST)
  - Handshake_Master_Spec_v02.137.md 6.0.2.5.1 HTC-1.0 JSON Schema file (SSoT) (MUST)
  - Handshake_Master_Spec_v02.137.md 6.0.2.9 Conformance tests (MUST)
  - Handshake_Master_Spec_v02.137.md 11.3.0 Canonical Tool Contract Binding (Normative)
  - Handshake_Master_Spec_v02.137.md 11.5 Flight Recorder Event Shapes & Retention -> FR-EVT-007 (ToolCallEvent)

## INTENT (DRAFT)
- What: Implement the Unified Tool Surface Contract (HTC v1.0) as a single source of truth for tool identity/schemas/side-effects, and enforce that *all* tool calls (local + MEX + MCP) pass through a single Tool Gate with canonical Flight Recorder logging.
- Why: Prevent “dual tool schema” drift and tool-call bypass paths that break determinism, capabilities/consent enforcement, secret redaction, and auditability across local and cloud-model orchestration.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Tool Registry SSoT:
    - Single canonical tool definition store (HTC-1.0), including `tool_id`, `tool_version`, schemas, `side_effect`, idempotency semantics, and redaction metadata.
    - MCP tool discovery/schemas generated from the Tool Registry (no parallel schema).
  - Tool Gate (single enforcement point):
    - Capability/consent enforcement for every tool invocation (local + MCP + MEX).
    - Payload limits (incl. 32KB rule) + strict artifact-first args/results (hashes computed *after* redaction).
    - Idempotency keys enforced where required.
  - Observability:
    - Every tool invocation emits FR-EVT-007 (ToolCallEvent) with correlation fields (`trace_id`, tool identity/version, transport, timing, capability_ids, redacted refs/hashes).
  - Conformance:
    - Tool contract conformance tests (Spec §6.0.2.9) in CI to fail any bypass or schema divergence.
- OUT_OF_SCOPE:
  - Phase 2+ “Design Studio shell/IA recontextualization” work.
  - Full DCC Tool Call Ledger UX (coordinate with DCC WP; keep this WP focused on contract + enforcement + logging).
  - Handshake-as-MCP-server (local) beyond the minimum needed to prove schema unification and Tool Gate enforcement.

## ACCEPTANCE_CRITERIA (DRAFT)
- At least 10 tools are registered in the Tool Registry with stable `tool_id` + `tool_version` and correct `side_effect` + idempotency semantics.
- Local tool calling and MCP tool calling use the same Tool Registry-derived schema and both route through Tool Gate (no bypass paths).
- Every tool invocation emits FR-EVT-007 (ToolCallEvent) with redacted `args_ref`/`result_ref` + hashes computed after redaction, and is linkable to its parent job/workflow via `trace_id`.
- Tool contract conformance tests exist and fail deterministically on:
  - any tool call that bypasses Tool Gate
  - any divergence between MCP-exposed schemas and Tool Registry schemas

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Capability SSoT + approval plumbing; Flight Recorder schema enforcement; MEX runtime tool invocation plumbing.
- Coordinates with:
  - WP-1-Cross-Tool-Interaction-Conformance-v1 (existing tool.* conformance proof; reconcile with FR-EVT-007 expectations)
  - WP-1-Dev-Command-Center-MVP-v1 (Tool Call Ledger / approvals UX)
  - WP-1-MCP-End-to-End-v2 / WP-1-MCP-Skeleton-Gate-v2 (MCP transport + schema exposure posture)

## RISKS / UNKNOWNs (DRAFT)
- Risk: introducing Tool Gate breaks existing tool paths if bypasses exist; must inventory and enforce centrally.
- Risk: secret/PII leakage in tool args/results; requires strict redaction + artifact-first refs.
- Risk: schema drift between local tools and MCP tools; requires generation and conformance tests.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Unified-Tool-Surface-Contract-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Unified-Tool-Surface-Contract-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

