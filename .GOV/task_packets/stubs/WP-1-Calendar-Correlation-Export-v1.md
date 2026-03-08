# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Calendar-Correlation-Export-v1

## STUB_METADATA
- WP_ID: WP-1-Calendar-Correlation-Export-v1
- BASE_WP_ID: WP-1-Calendar-Correlation-Export
- CREATED_AT: 2026-03-08T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- BUILD_ORDER_DOMAIN: CROSS_BOUNDARY
- BUILD_ORDER_TECH_BLOCKER: NO
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Calendar-Storage, WP-1-Calendar-Sync-Engine, WP-1-Calendar-Policy-Integration, WP-1-Flight-Recorder, WP-1-Role-Mailbox
- BUILD_ORDER_BLOCKS: NONE
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.150.md 7.6.3 (Phase 1) -> backend combo expansion: calendar correlation export
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.150.md 10.4 Calendar (Normative)
  - Handshake_Master_Spec_v02.150.md 11.9.2 Calendar Range as a Query Surface (Normative)
  - Handshake_Master_Spec_v02.150.md 11.9.3 CalendarEvent and ActivitySpan Join Semantics (Normative)
  - Handshake_Master_Spec_v02.150.md 11.9.4 Minimum Slice for Calendar and Flight Recorder (Normative)
  - Handshake_Master_Spec_v02.150.md 2.3.10 Debug Bundle export (Normative)

## INTENT (DRAFT)
- What: Treat calendar event windows and time-range queries as canonical backend correlation/export anchors for jobs, activity spans, recorder events, and later mailbox/task follow-ons.
- Why: Calendar should operate as a backend force multiplier for bounded evidence export and time-scoped reasoning, not only as a passive date surface.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Define the canonical link between `CalendarEventWindowQuery`, recorder time windows, bundle export scope, and job/activity correlation.
  - Ensure calendar-derived export remains bounded, policy-aware, and deterministic.
  - Leave room for later calendar/mailbox and calendar/project-brain follow-ons without guessing the UI first.
- OUT_OF_SCOPE:
  - Full calendar lens UX redesign.
  - External provider write-back semantics beyond the already-scoped calendar policy work.

## ACCEPTANCE_CRITERIA (DRAFT)
- Calendar event windows are explicitly named as valid debug bundle/time-range export anchors.
- Time-window export semantics are tied to stable calendar/query ids rather than loose UI filters.
- Later implementations can correlate calendar windows with jobs and activity spans without inventing new backend nouns.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on: Calendar storage, sync engine, policy integration, Flight Recorder, and Role Mailbox context flow.
- Research seed: adapt incremental sync and time-window export discipline from mature calendar systems such as Google Calendar API and CalDAV-style sync contracts while preserving Handshake-first governance rules.

## RISKS / UNKNOWNs (DRAFT)
- Risk: calendar time windows silently broaden into unbounded export scope.
- Risk: sync-token and window semantics diverge across storage backends or future providers.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Calendar-Correlation-Export-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Calendar-Correlation-Export-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
