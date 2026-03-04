# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Calendar-Lens-v3

## STUB_METADATA
- WP_ID: WP-1-Calendar-Lens-v3
- BASE_WP_ID: WP-1-Calendar-Lens
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Calendar lens drift)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md Calendar lens requirements (Calendar view, sources, events, selection)
  - Handshake_Master_Spec_v02.139.md Timeline/ActivitySpan interoperability (Calendar as first-class time data)
  - Handshake_Master_Spec_v02.139.md Capabilities + Consent (calendar sync and external access)

## INTENT (DRAFT)
- What: Implement the Calendar lens surface as a first-class UI + API workflow for viewing and filtering calendar events.
- Why: Calendar is a core organizing substrate for personal workflows; without it, downstream surfaces (timeline, reminders, microtask scheduling, governance traceability) are missing time-ground truth.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Calendar lens UI surface (baseline):
    - View: day/week/month (at least one view shipped first, others staged).
    - Filter by source (CalendarSource) and date range; basic text search over event title.
  - Backend API for CalendarSource/CalendarEvent query:
    - Query events by time window + source IDs.
    - Stable IDs + provenance fields.
  - Minimal integration points:
    - Optional: link/overlay Calendar events into existing timeline/flight recorder UI views (read-only).
- OUT_OF_SCOPE:
  - Full bidirectional edit/write-back to external calendar providers.
  - Recurrence rule edge-cases beyond a clearly documented MVP subset.

## ACCEPTANCE_CRITERIA (DRAFT)
- User can open Calendar lens and see events for a selected time window.
- User can add at least one CalendarSource (even if local/ICS-only in MVP) and filter by source.
- Calendar events are persisted and reload without drift across app restarts.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Calendar storage (separate stub): WP-1-Calendar-Storage-v1.
- Calendar sync engine (separate stub): WP-1-Calendar-Sync-Engine-v1 (if external sources are in-scope).
- Capability/consent invariants for any external access (net.http, secrets.use).

## RISKS / UNKNOWNs (DRAFT)
- Time zone correctness and DST edge cases.
- Event privacy: avoid accidental indexing/leakage into search surfaces without explicit consent/policy.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Calendar-Lens-v3.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Calendar-Lens-v3` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

