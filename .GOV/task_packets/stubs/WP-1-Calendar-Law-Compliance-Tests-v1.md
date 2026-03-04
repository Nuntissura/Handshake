# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Calendar-Law-Compliance-Tests-v1

## STUB_METADATA
- WP_ID: WP-1-Calendar-Law-Compliance-Tests-v1
- BASE_WP_ID: WP-1-Calendar-Law-Compliance-Tests
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: MEDIUM
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Calendar-Storage, WP-1-Calendar-Sync-Engine, WP-1-Calendar-Policy-Integration
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Calendar compliance posture)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Calendar compliance and policy invariants (consent, retention, minimization)
  - Handshake_Master_Spec_v02.139.md Flight Recorder evidence posture (leak-safe payload rules)

## INTENT (DRAFT)
- What: Add explicit test coverage for Calendar ingestion/storage behavior that is safety- and compliance-critical (consent/retention/minimization invariants).
- Why: Calendar data is sensitive; regressions must be prevented by deterministic tests and leak-safe evidence.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Tests for:
    - no external sync without explicit consent receipt
    - minimization/redaction of sensitive fields in logs/events
    - retention/delete-by-source behavior
    - capability-gated operations for sync and export.
- OUT_OF_SCOPE:
  - Jurisdiction-specific legal analysis (engineering test posture only).

## ACCEPTANCE_CRITERIA (DRAFT)
- A targeted test suite fails if calendar sync can run without consent/caps or if sensitive payloads leak into Flight Recorder events.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Calendar storage + sync engine exist (other stubs).
- Consent bundle and capability gate surfaces exist.

## RISKS / UNKNOWNs (DRAFT)
- Defining what constitutes a "sensitive field" requires a spec-extracted allowlist to avoid ambiguity.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Calendar-Law-Compliance-Tests-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Calendar-Law-Compliance-Tests-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
