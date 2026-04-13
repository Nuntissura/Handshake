# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Calendar-Storage-v2

## STUB_METADATA
- WP_ID: WP-1-Calendar-Storage-v2
- BASE_WP_ID: WP-1-Calendar-Storage
- CREATED_AT: 2026-04-07T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- ACTIVATION_MANAGER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
- BUILD_ORDER_DOMAIN: BACKEND
- BUILD_ORDER_TECH_BLOCKER: YES
- BUILD_ORDER_VALUE_TIER: HIGH
- BUILD_ORDER_RISK_TIER: HIGH
- BUILD_ORDER_DEPENDS_ON: WP-1-Migration-Framework, WP-1-Storage-Abstraction-Layer, WP-1-Storage-Trait-Purity, WP-1-Storage-Capability-Boundary-Refactor
- BUILD_ORDER_BLOCKS: WP-1-Calendar-Lens, WP-1-Calendar-Sync-Engine, WP-1-Calendar-Policy-Integration, WP-1-Calendar-Law-Compliance-Tests, WP-1-Calendar-Correlation-Export, WP-1-Calendar-Mailbox-Correlation
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Calendar is a pillar feature; storage foundation for calendar-as-cross-cutting-substrate across Handshake
- PHASE1_SCHEDULING: Later stages of Phase 1 (calendar pillar requires broad primitive interweaving discovered through modern refinement)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - CalendarSource and CalendarEvent schema requirements
  - Storage boundary + portable migrations (SQLite + Postgres)
  - Provenance fields (mandatory)
  - Mutation governance (Hard Invariant)
  - Pillar 2: Portable Schema & Migrations [CX-DBP-011]
  - Pillar 4: Dual-Backend Testing Early [CX-DBP-013]

## Why this stub exists
This is a supersession of `WP-1-Calendar-Storage-v1`.

v1 was activated, coded, and implementation-completed under GPT-5 Codex CLI (2026-03-06) but never validated. It was refined against spec v02.141 (current: v02.179, 38 versions behind) and used the old refinement workflow which did not perform primitive mixing or feature discovery.

Calendar is a **pillar feature** in Handshake -- it should be interwoven with almost every subsystem (loom, mailbox, workflow, governance, consent, flight recorder, structured collaboration, workspace context). The v1 refinement treated it as an isolated storage-only concern. A modern refinement must discover the full primitive surface and ensure calendar storage is designed as a cross-cutting substrate, not a siloed persistence layer.

The v1 code on branch `feat/WP-1-Calendar-Storage-v1` can inform the v2 implementation but must not be assumed valid against the current spec or storage patterns (trait purity, capability boundary refactor, and structured collaboration have all landed since v1).

## Prior packet
- Prior WP_ID: `WP-1-Calendar-Storage-v1`
- Prior packet: `.GOV/task_packets/WP-1-Calendar-Storage-v1.md`
- Prior stub: `.GOV/task_packets/stubs/WP-1-Calendar-Storage-v1.md`
- Prior refinement: `.GOV/refinements/WP-1-Calendar-Storage-v1.md`
- Prior branch: `feat/WP-1-Calendar-Storage-v1` (deleted)
- Prior spec baseline: Handshake_Master_Spec_v02.141.md
- Prior status: Implementation complete, never validated, spec-stale

## Known gaps from v1
- No primitive mixing or feature discovery in refinement (old workflow)
- Spec baseline v02.141 is 38 versions stale (current: v02.179)
- Storage trait has been restructured since v1 (trait purity + capability boundary refactor landed)
- No interweaving with structured collaboration, loom correlation, mailbox, workflow projection, or governance overlay
- Calendar-as-substrate design (cross-cutting primitive surface) was never explored

## INTENT (DRAFT)
- What: Add portable, dual-backend persistent storage for CalendarSource and CalendarEvent through the Database trait, with full primitive mixing to ensure calendar storage is designed as a cross-cutting substrate that integrates cleanly with Handshake's broader topology.
- Why: Calendar is a pillar feature. Every subsystem that reasons about time, scheduling, context, or provenance will eventually touch calendar data. Getting the storage foundation right -- with proper interweaving across loom, mailbox, workflow, governance, consent, and structured collaboration -- prevents expensive rework and ensures calendar can serve as a force multiplier across the product.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - Portable migrations for calendar_sources and calendar_events (dual-backend)
  - Database trait calendar storage methods (aligned with current trait purity patterns)
  - Idempotent upsert, time-window queries, source-scoped cleanup
  - Primitive discovery: identify all cross-cutting touchpoints where calendar data flows into or out of other subsystems
  - Stub generation for any discovered downstream integration gaps
- OUT_OF_SCOPE:
  - Calendar Lens UI, API handlers, search/full-text surfaces
  - calendar_sync workflow orchestration, provider adapters
  - Recurrence expansion beyond stored raw fields
  - Direct SQL from API/UI code or runtime DDL

## ACCEPTANCE_CRITERIA (DRAFT)
- Calendar sources and events persist and can be queried by time window on both backends.
- Migrations are portable, replay-safe, and undo-safe on SQLite and Postgres.
- Storage API conforms to current trait purity and capability boundary patterns.
- Primitive matrix identifies all cross-cutting calendar touchpoints and generates stubs where needed.
- Dual-backend conformance tests cover CRUD, time-window queries, migration safety, and idempotent upsert.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Storage abstraction layer, migration framework, trait purity, and capability boundary refactor are all validated and merged.
- Should be scheduled in later Phase 1 stages to allow broader primitive surface to stabilize first.

## RISKS / UNKNOWNs (DRAFT)
- Recurrence and timezone normalization decisions can cause long-term drift; must be versioned.
- Event privacy requires clear provenance and policy hooks (separate stub).
- Calendar-as-substrate means the primitive surface may be large; refinement must timebox discovery to avoid scope explosion while still capturing the interweaving.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Calendar-Storage-v2.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Calendar-Storage-v2` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
