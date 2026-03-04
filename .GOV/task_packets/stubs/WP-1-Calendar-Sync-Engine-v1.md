# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Calendar-Sync-Engine-v1

## STUB_METADATA
- WP_ID: WP-1-Calendar-Sync-Engine-v1
- BASE_WP_ID: WP-1-Calendar-Sync-Engine
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Calendar-Storage, WP-1-MEX-v1.2-Runtime, WP-1-Workflow-Engine
- BUILD_ORDER_BLOCKS: WP-1-Calendar-Law-Compliance-Tests
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (calendar_sync mechanical engine)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Calendar sync mechanical engine requirements
  - Handshake_Master_Spec_v02.139.md Capabilities & Consent model (net.http, secrets.use)
  - Handshake_Master_Spec_v02.139.md Flight Recorder evidence for sync jobs

## INTENT (DRAFT)
- What: Implement `calendar_sync` as a first-class mechanical engine job that ingests calendar sources into durable CalendarEvent storage.
- Why: Without deterministic sync, Calendar lens will drift and external sources cannot be trusted or audited.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Mechanical engine + workflow runner wiring:
    - Register engine entry.
    - Implement job profile and runner that writes into Calendar storage.
  - Source adapters:
    - MVP: local ICS file import OR a single provider with strict capability/consent gating.
  - Observability:
    - Flight Recorder events for sync start/end, items imported, dedup hits, failures.
- OUT_OF_SCOPE:
  - Multi-provider, bidirectional write-back, and conflict resolution.

## ACCEPTANCE_CRITERIA (DRAFT)
- Running calendar_sync ingests events into Calendar storage and updates the Calendar lens view.
- Sync is idempotent (repeat sync does not duplicate events).
- All external access is capability-gated and leak-safe in logs/events.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Calendar storage: WP-1-Calendar-Storage-v1.
- Mechanical Tool Bus / engine allowlisting exists.
- Provider credentials storage (if external source is selected) and consent UX.

## RISKS / UNKNOWNs (DRAFT)
- Provider API churn and rate limits.
- Privacy and consent ergonomics; must be fail-closed.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Calendar-Sync-Engine-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Calendar-Sync-Engine-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
