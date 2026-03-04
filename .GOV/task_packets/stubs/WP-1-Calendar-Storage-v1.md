# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Calendar-Storage-v1

## STUB_METADATA
- WP_ID: WP-1-Calendar-Storage-v1
- BASE_WP_ID: WP-1-Calendar-Storage
- CREATED_AT: 2026-03-04T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Audit remediation against Handshake_Master_Spec_v02.139.md (Calendar entities + persistence)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.139.md CalendarSource and CalendarEvent schema requirements
  - Handshake_Master_Spec_v02.139.md Storage boundary + portable migrations (SQLite + Postgres)
  - Handshake_Master_Spec_v02.139.md Provenance fields (mandatory)

## INTENT (DRAFT)
- What: Add persistent storage for CalendarSource/CalendarEvent with portable migrations and a Database-trait-conformant DAL.
- Why: Calendar lens and sync cannot be correct or auditable without durable, queryable event persistence.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Portable migrations for:
    - calendar_sources (id, kind/provider, display_name, created_at, config/provenance)
    - calendar_events (id, source_id, start_ts, end_ts, title, location, raw_ref/provenance)
    - indexes for time-range queries (source_id + start/end).
  - Storage API surface for:
    - upsert/list sources
    - upsert/query events (idempotent)
    - delete-by-source (disconnect).
- OUT_OF_SCOPE:
  - Full-text indexing and embeddings for calendar notes (layer later).
  - Complex recurrence expansion beyond a stored raw field in MVP.

## ACCEPTANCE_CRITERIA (DRAFT)
- Calendar sources and events persist and can be queried by time window efficiently.
- Migrations run cleanly on SQLite and Postgres (no runtime DDL hacks).
- A small conformance test suite verifies portable schema + basic CRUD.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Storage abstraction layer and migration framework already exist.

## RISKS / UNKNOWNs (DRAFT)
- Recurrence and timezone normalization decisions can cause long-term drift; must be versioned.
- Event privacy requires clear provenance and policy hooks (separate stub).

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Calendar-Storage-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Calendar-Storage-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.

