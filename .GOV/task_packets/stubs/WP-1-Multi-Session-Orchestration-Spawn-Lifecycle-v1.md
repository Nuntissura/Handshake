# TASK_PACKET_STUB_TEMPLATE

This is a BACKLOG STUB. It is NOT an executable Task Packet.

**ARCHIVED / SUPERSEDED (2026-02-24):** This stub was split into smaller Phase 1 stubs. Do not activate this stub.

Split stubs:
- WP-1-ModelSession-Core-Scheduler-v1
- WP-1-Session-Spawn-Contract-v1
- WP-1-Session-Scoped-Capabilities-Consent-Gate-v1
- WP-1-Provider-Feature-Coverage-Agentic-Ready-v1
- WP-1-Workspace-Safety-Parallel-Sessions-v1
- WP-1-Session-Crash-Recovery-Checkpointing-v1
- WP-1-Session-Observability-Spans-FR-v1

Rules:
- No USER_SIGNATURE is requested/required for stubs.
- No refinement file is required for stubs.
- Coder/Validator MUST NOT start work from a stub.
- When activating a stub into a real WP, follow `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md` (Technical Refinement Block + USER_SIGNATURE + refinement + `just create-task-packet`).
- If a Base WP later gains multiple packets (revisions), record Base WP -> Active Packet in `.GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md`.

---

# Work Packet Stub: WP-1-Multi-Session-Orchestration-Spawn-Lifecycle-v1

## STUB_METADATA
- WP_ID: WP-1-Multi-Session-Orchestration-Spawn-Lifecycle-v1
- BASE_WP_ID: WP-1-Multi-Session-Orchestration-Spawn-Lifecycle
- CREATED_AT: 2026-02-23T00:00:00Z
- STUB_STATUS: SUPERSEDED (ARCHIVED; split into smaller stubs; do not activate)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md
- ROADMAP_POINTER: Handshake_Master_Spec_v02.137.md 7.6.3 (Phase 1) -> items 28-37 (Multi-Session Orchestration: ModelSession + Scheduler + spawn lifecycle + observability)
- SPEC_ANCHOR_CANDIDATES (Main Body, not Roadmap):
  - Handshake_Master_Spec_v02.137.md 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.13 Session Scheduler: Model Calls as Queued Work (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.14 Cloud Consent-Gate Lifecycle for Parallel Sessions (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.15 Session Spawn Contract and Lifecycle (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.17 Workspace Safety Boundaries for Parallel Sessions (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.18 Session Observability: ActivitySpan and ModelSessionSpan Binding (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.19 Session Crash Recovery and Checkpointing (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 4.3.9.20 Inbound Trust Boundary Rules (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.137.md 6.0.2 Unified Tool Surface Contract (Local Tool Calling + MCP) (Normative)
  - Handshake_Master_Spec_v02.137.md 11.5 Flight Recorder Event Shapes & Retention -> FR-EVT-SESS-* + FR-EVT-SESS-SCHED-* + FR-EVT-SESS-SPAWN-* (ADD v02.137)
  - Handshake_Master_Spec_v02.137.md 11.5.1 Flight Recorder Ingestion Contract (Normative Trait) (model_session_id correlation rule)

## INTENT (DRAFT)
- What: Implement Phase 1 baseline for multi-session orchestration: persisted ModelSessions, queued `model_run` scheduling, governed spawn lifecycle, session-scoped capability enforcement, workspace safety boundaries for parallel sessions, crash recovery/checkpointing, and end-to-end observability (Flight Recorder + spans).
- Why: Prevent runaway parallelism (budget blowups), unsafe concurrent writes, and non-auditable “ghost sessions” as multi-model execution scales beyond single-threaded chat.

## SCOPE_SKETCH (DRAFT)
- IN_SCOPE (aligns to Roadmap 7.6.3 items 28-37):
  - **ModelSession + persistence (items 28-29):**
    - ModelSession is the persisted unit of multi-turn orchestration; stored in local workspace DB.
    - Messages are stored artifact-first with explicit content-hash discipline.
    - Add/define `model_run` job_kind and a session scheduler with queueing, cancellation, and concurrency limits.
  - **Spawn lifecycle (item 30):**
    - Implement SessionSpawnRequest/Response, depth limits, per-session spawn caps.
    - Announce-back via Role Mailbox (SessionAnnounceBack) with summary artifacts.
  - **Session-scoped capabilities (item 31):**
    - Add ModelSession `capability_token_ids`.
    - Enforce deny-by-default session-scoped capability intersection in Tool Gate.
  - **Provider adapters (item 32):**
    - Provider adapters translate the Unified Tool Surface registry into provider tool schemas and back (no parallel ToolDefinition schema).
  - **Workspace safety boundaries (item 33):**
    - Deterministic policy for concurrent sessions on same workspace: worktree isolation and/or file locking; explicit conflict handling.
  - **Crash recovery / resume (item 34):**
    - Session checkpointing and idempotent recovery flow for interrupted `model_run`.
  - **DCC steering panel (item 35):**
    - Multi-session panel shows session list + state machine + spawn tree + cost/budget per session and controls (pause/resume/cancel).
  - **Observability (items 36-37):**
    - Register + emit FR-EVT-SESS-*, FR-EVT-SESS-SCHED-*, FR-EVT-SESS-SPAWN-*.
    - Add `model_session_id` correlation to base Flight Recorder event schema.
    - Implement ModelSessionSpan lifecycle and ActivitySpan linkage so session-wide queries work via model_session_id even without spans.
- OUT_OF_SCOPE:
  - Full AutomationLevel.AUTONOMOUS loops (explicitly Phase 2+).
  - Cross-workspace or multi-operator session routing (Phase 2+/4).
  - Multi-tenant “apps spawning sessions” hard isolation (Phase 4).

## ACCEPTANCE_CRITERIA (DRAFT)
- The vertical slice for multi-session runs end-to-end:
  - Create a ModelSession, enqueue multiple `model_run` jobs, observe queueing/dispatch, and cancel at least one deterministically.
  - Spawn a child session via SessionSpawnRequest/Response; enforce depth/cap limits; announce-back with summary artifact.
  - Tool calls executed within a ModelSession carry the correct session correlation and are capability-gated with deny-by-default session intersection.
  - Concurrent sessions touching the same workspace are blocked or isolated deterministically with explicit conflict reasons.
  - Crash/restart resumes (or deterministically fails) without partial side effects, using checkpointing.
  - Flight Recorder shows FR-EVT-SESS-* families and every event has correct `model_session_id` correlation where applicable.

## DEPENDENCIES / BLOCKERS (DRAFT)
- Depends on:
  - Unified Tool Surface Contract + Tool Gate baseline (see stub: WP-1-Unified-Tool-Surface-Contract-v1).
  - Locus MT occupancy integration (active_session_ids + bind/unbind ops) (spec v02.137 note; may require a Locus stub revision).
  - Capability SSoT + approval plumbing.
  - Flight Recorder schema registry / validator enforcement for new event families.
- Coordinates with:
  - WP-1-Dev-Command-Center-MVP-v1 (DCC Sessions panel / controls)
  - WP-1-Multi-Model-Orchestration-Lifecycle-Telemetry-v1 (overlap: concurrency + lifecycle telemetry; keep responsibilities distinct)

## RISKS / UNKNOWNs (DRAFT)
- Risk: budget/runaway spawn without strict caps and explicit operator controls.
- Risk: cross-session side effects without session-scoped capabilities (Tool Gate must be the single enforcement point).
- Risk: crash recovery correctness (idempotency + checkpoints) is easy to get wrong; requires deterministic replayability.
- Risk: observability drift if new event families are not included in the Flight Recorder schema registry/validator.

## ACTIVATION_CHECKLIST (REQUIRED BEFORE ANY CODING)
- [ ] Confirm the requirement exists in Master Spec Main Body (not just Roadmap).
- [ ] Produce the in-chat Technical Refinement Block (per `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`).
- [ ] Obtain USER_SIGNATURE for the WP.
- [ ] Create `.GOV/refinements/WP-1-Multi-Session-Orchestration-Spawn-Lifecycle-v1.md` (approved/signed).
- [ ] Create the official task packet via `just create-task-packet WP-1-Multi-Session-Orchestration-Spawn-Lifecycle-v1` (in `.GOV/task_packets/`).
- [ ] Copy relevant scope/acceptance notes from this stub into the official packet.
- [ ] Move `.GOV/roles_shared/TASK_BOARD.md` entry from STUB to Ready for Dev.
