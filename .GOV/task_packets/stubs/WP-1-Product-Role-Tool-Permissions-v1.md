# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Product-Role-Tool-Permissions-v1

## STUB_METADATA
- WP_ID: WP-1-Product-Role-Tool-Permissions-v1
- BASE_WP_ID: WP-1-Product-Role-Tool-Permissions
- CREATED_AT: 2026-04-06T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Session-Scoped-Capabilities-Consent-Gate
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Role-based tool permission enforcement in the product session manager
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec session capabilities and consent gate
  - Handshake_Master_Spec role definitions and permissions
  - Handshake_Master_Spec TRUST model and capability narrowing
- DISCOVERY_ORIGIN: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION (RGF-105, Overstory tool-call guard pattern and TRUST-003 capability narrowing)

## INTENT (DRAFT)
- What: Role-based tool permission enforcement in the product session manager. Validators get read-only file access (can read any file, cannot write to product code paths). Coders get write access within their declared scope. Permissions enforced at the tool execution layer, not via prompts. Based on Overstory tool-call guard pattern and TRUST-003 capability narrowing.
- Why: Prompt-based permission enforcement is unreliable: models can be prompted to bypass stated restrictions. Enforcing permissions at the tool execution layer guarantees that validators cannot write to product code and coders cannot write outside their declared scope, regardless of model behavior. This is a fundamental security boundary for multi-session orchestration where different roles have different trust levels. Aligns with the TRUST-003 capability narrowing principle.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Permission model: role -> allowed tool set -> allowed path scopes.
  - Validator role: read-only file access (Read, Grep, Glob); no Write, Edit, Bash write operations.
  - Coder role: read/write access scoped to declared MT paths; no access outside scope.
  - Orchestrator role: full tool access (privileged).
  - Permission enforcement at the tool execution layer (intercept before tool runs).
  - Denied tool call logging via Flight Recorder (who tried what, why denied).
  - Permission configuration per role in the session spawn contract.
  - Graceful denial response to the session (not a crash, a structured denial message).
- OUT_OF_SCOPE:
  - Dynamic permission escalation during a session (permissions are fixed at spawn time for v1).
  - Fine-grained file-level ACLs (path-scope-based, not per-file).
  - Network access control (separate concern).
  - The consent gate itself (already exists in WP-1-Session-Scoped-Capabilities-Consent-Gate).

## ACCEPTANCE_CRITERIA (DRAFT)
- Validator sessions can execute read-only tools (Read, Grep, Glob) on any file path.
- Validator sessions are blocked from executing write tools (Write, Edit, Bash write operations) on product code paths.
- Coder sessions can execute read/write tools only within their declared MT scope paths.
- Coder sessions are blocked from writing outside their declared scope.
- Orchestrator sessions have unrestricted tool access.
- Permission enforcement occurs at the tool execution layer, not via prompt instructions.
- Denied tool calls produce a structured denial message to the session (not a crash).
- Denied tool calls are logged via Flight Recorder with role, tool, path, and denial reason.
- Permission configuration is declared in the session spawn contract per role.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on WP-1-Session-Scoped-Capabilities-Consent-Gate for the capability and consent infrastructure.
- Requires the tool execution layer to support pre-execution interceptors.
- Integrates with the session spawn contract for permission declaration.
- No spec blockers identified.

## RISKS / UNKNOWNs (DRAFT)
- Risk: Overly restrictive permissions may block legitimate tool usage patterns (e.g., coder needs to read a shared config file outside MT scope).
- Risk: Bash tool is difficult to scope precisely; commands can be composed to bypass simple path checks.
- Risk: Permission model must balance security with usability; too many denied calls degrade session effectiveness.
- Unknown: Whether Bash tool requires a whitelist approach (allowed commands only) or a deny-list approach (block known dangerous operations).
- Unknown: How to handle cross-scope dependencies where a coder's MT legitimately requires reading/modifying files in another MT's scope.

## DISCOVERY_ORIGIN
- Source: RESEARCH-20260406-AGENT-SWARM-PARALLEL-ORCHESTRATION
- RGF Reference: RGF-105
- Pattern: Overstory tool-call guard pattern with TRUST-003 capability narrowing for role-based tool permission enforcement.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Product-Role-Tool-Permissions-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Product-Role-Tool-Permissions-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/records/TASK_BOARD.md` entry from STUB to Ready for Dev.
