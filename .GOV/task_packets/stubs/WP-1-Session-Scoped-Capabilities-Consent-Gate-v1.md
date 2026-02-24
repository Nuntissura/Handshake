# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1

## STUB_METADATA
- WP_ID: WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- BASE_WP_ID: WP-1-Session-Scoped-Capabilities-Consent-Gate
- CREATED_AT: 2026-02-24T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> items 31 (session-scoped capabilities) + 4.3.9.14 consent gate + 4.3.9.20 trust boundary rules
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.20 Inbound Trust Boundary Rules (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.12 ModelSession (capability_grants + capability_token_ids)
  - Handshake_Master_Spec_v02.137.md 6.0.2 Unified Tool Surface Contract -> Tool Gate (session-scoped capability intersection)
  - Handshake_Master_Spec_v02.137.md 11.1 Capability Registry + receipts/tokens (session-scoped effective grants)

## INTENT (DRAFT)
- What: Enforce session-scoped capability intersection and consent gating for cloud calls across parallel sessions, and implement inbound trust boundary rules for cross-session/system message provenance.
- Why: Parallel orchestration is unsafe without deny-by-default per-session effective capabilities, durable consent receipts for fan-out, and strict inbound provenance/anti-bypass constraints.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Session-scoped effective capabilities:
    - ModelSession `capability_grants` and `capability_token_ids` wired to CapabilityRegistry receipts/tokens.
    - Tool Gate enforcement uses per-session capability intersection (not global grants).
    - Child sessions have equal-or-narrower tool permissions than parent (TRUST-003).
  - Cloud consent gate lifecycle:
    - ProjectionPlan + ConsentReceipt binding to session_ids, with scopes (single call / session / WP / broadcast).
    - Scheduler verification of valid receipts before dispatching cloud `model_run`.
    - Revocation cancels pending jobs and blocks sessions deterministically.
  - Inbound trust boundary rules:
    - SYSTEM message provenance restriction (runtime-only).
    - Cross-session routed messages carry source attribution + hashes + trusted/untrusted flags.
    - No global bypass flags for sandbox/approvals/capabilities (session-scoped debug only, logged).
- OUT_OF_SCOPE:
  - Provider tool calling adapters (tracked in WP-1-Provider-Feature-Coverage-Agentic-Ready-v1).
  - Scheduler mechanics (tracked in WP-1-ModelSession-Core-Scheduler-v1).

## ACCEPTANCE_CRITERIA (DRAFT)
- Tool Gate evaluates required_capabilities against the per-session effective grants; missing grant blocks with explicit reason and Flight Recorder evidence.
- A broadcast to N cloud sessions requires a BROADCAST_SCOPED receipt enumerating session_ids; adding sessions requires a new receipt.
- Revoking consent cancels pending model_run jobs and transitions affected sessions to BLOCKED deterministically.
- SYSTEM message injection is runtime-only; any external source attempting SYSTEM injection is rejected and recorded.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Capability registry + approval token plumbing (existing capability WPs).
- Depends on: Tool Gate + Tool Registry baseline (WP-1-Unified-Tool-Surface-Contract-v1).
- Coordinates with: Cloud escalation consent history (WP-1-Cloud-Escalation-Consent-v2) to avoid duplicating artifacts; this WP extends the model to parallel sessions.

## RISKS / UNKNOWNs (DRAFT)
- Risk: inconsistent enforcement between local/MCP/Stage Bridge transports; Tool Gate must be single enforcement point.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Session-Scoped-Capabilities-Consent-Gate-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Session-Scoped-Capabilities-Consent-Gate-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
