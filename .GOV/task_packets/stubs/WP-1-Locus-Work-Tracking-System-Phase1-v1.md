# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Locus-Work-Tracking-System-Phase1-v1

## STUB_METADATA
- WP_ID: WP-1-Locus-Work-Tracking-System-Phase1-v1
- BASE_WP_ID: WP-1-Locus-Work-Tracking-System-Phase1
- CREATED_AT: 2026-01-25T00:00:00Z
- STUB_STATUS: STUB (NOT READY FOR DEV)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.116.md 7.6.3 (Phase 1) -> [ADD v02.116] Locus Work Tracking System (2.3.15) - Phase 1
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - 2.3.15.3 Mechanical Operations
  - 2.3.15.4 Integration Points
  - 2.3.15.5 Storage Architecture
  - 2.3.15.6 Event Sourcing
  - 2.3.15.7 Query Interface

## INTENT (DRAFT)
- What: Introduce Locus Work Tracking System Phase 1 baseline (SQLite-backed, core WP/MT operations, Spec Router + MT Executor integration, Task Board sync, and Flight Recorder events).
- Why: Provide governance-aware, end-to-end traceability from Work Packets -> Micro-Tasks -> iterations -> gates, with deterministic status sync to `.GOV/roles_shared/TASK_BOARD.md`.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE:
  - SQLite schema and storage for work_packets, micro_tasks, mt_iterations, dependencies (per 2.3.15.5).
  - Core Locus mechanical operations (create/update/gate/close WP; register/start/record/complete MT) (per 2.3.15.3).
  - Dependency operations (add/remove + cycle detection) (per 2.3.15.7).
  - Basic query ops (query_ready / get status / get progress) (per 2.3.15.7).
  - Integrations: Spec Router and MT Executor integration points (per 2.3.15.4).
  - Task Board bidirectional sync (`locus_sync_task_board`) (per 2.3.15.4).
  - Flight Recorder events for WP/MT/DEP/TB/QUERY families (per 2.3.15.6).
- OUT_OF_SCOPE:
  - Hybrid search + Calendar policy integration (Phase 2 roadmap).
  - PostgreSQL backend + CRDT + realtime collaboration (Phase 3 roadmap).
  - Advanced analytics / AI insights / plugin API (Phase 4 roadmap).

## ACCEPTANCE_CRITERIA (DRAFT)
- Spec Router creates Work Packets visible in Locus storage (via `locus_create_wp`).
- MT Executor iterations are recorded (start -> record_iteration -> complete).
- Task Board sync occurs deterministically (no drift after `locus_sync_task_board`).
- `locus_query_ready` respects dependency blocking.
- Locus event families appear in Flight Recorder and unknown event_type fails fast.

## DEPENDENCIES / BLOCKERS (DRAFT)
- CapabilityRegistry SSoT (WP-1-Capability-SSoT) for Locus capability strings.
- Spec Router / MT Executor integration surfaces exist and are stable enough to hook.
- Flight Recorder event validation infrastructure exists (validators + diagnostics pipeline).

## RISKS / UNKNOWNs (DRAFT)
- High coupling risk: touches governance + workflow + observability surfaces.
- File-lock risk: overlaps with other WPs touching capability registry / spec router / flight recorder validators.
- Spec note: roadmap entries tagged [ADD v02.116] are "open to revision" (may require stub revision).

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Locus-Work-Tracking-System-Phase1-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Locus-Work-Tracking-System-Phase1-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.


