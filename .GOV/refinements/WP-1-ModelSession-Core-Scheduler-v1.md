## TECHNICAL_REFINEMENT (MASTER SPEC)

Requirements (HARD):
- This block is REQUIRED for any packet with Status: Ready for Dev / In Progress.
- For EACH SPEC_ANCHOR, include an excerpt window (start/end lines) AND a context token that must appear within that window in the current SPEC_TARGET_RESOLVED spec file.
- Matching rule: context match only (token-in-window), not exact content match.
- Even when ENRICHMENT_NEEDED=NO, REASON_NO_ENRICHMENT and SPEC_EXCERPTS are provided for every anchor.
- If ENRICHMENT_NEEDED=YES, the full Proposed Spec Enrichment text must be verbatim.
- Keep this file ASCII-only.
- This file is the Technical Refinement Block required by ORCHESTRATOR_PROTOCOL Part 2.5.2.

### METADATA
- WP_ID: WP-1-ModelSession-Core-Scheduler-v1
- CREATED_AT: 2026-03-01T19:29:40Z
- SPEC_TARGET_RESOLVED: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.139.md
- SPEC_TARGET_SHA1: 0A5A9069BF8E06654DDF9B647927C2CB8A30AA6F
- USER_REVIEW_STATUS: APPROVED
- USER_SIGNATURE: ilja010320262103
- USER_APPROVAL_EVIDENCE: APPROVE REFINEMENT WP-1-ModelSession-Core-Scheduler-v1

### REQUIRED SECTIONS (per ORCHESTRATOR_PROTOCOL Part 2.5.2)

### GAPS_IDENTIFIED
- NONE (for this WP scope). The current Master Spec already defines ModelSession schema, scheduler job kind, invariants, and scheduler Flight Recorder events in normative sections.
- Constraint note: cloud-consent lifecycle, spawn lifecycle, and full session observability are explicitly defined in adjacent sections and are intentionally out-of-scope for this WP except required integration points.

### FLIGHT_RECORDER_INTERACTION (event IDs + telemetry triggers)
- FR-EVT-SESS-SCHED-001 `session_scheduler.enqueue`: emit when a `model_run` job enters queue.
- FR-EVT-SESS-SCHED-002 `session_scheduler.dispatch`: emit when queued job is dispatched.
- FR-EVT-SESS-SCHED-003 `session_scheduler.rate_limited`: emit on scheduler-imposed backoff.
- FR-EVT-SESS-SCHED-004 `session_scheduler.cancelled`: emit on cooperative cancellation.
- Minimal WP requirement: scheduler events above + visibility of queue/dispatch/cancel state via existing Job History/Flight Recorder surfaces.

### RED_TEAM_ADVISORY (security failure modes)
- Scheduler bypass risk: direct model calls outside scheduler violate INV-SCHED-001 and remove governance/audit controls.
- State integrity risk: storing session message bodies inline in events leaks sensitive content; spec requires artifact-first storage with hash references.
- Lane starvation risk: background/subagent lanes starving primary lane degrades operator control and can conceal urgent work.
- Cancellation confusion risk: using FAILED instead of CANCELLED on user cancel corrupts recovery and incident triage semantics.

### PRIMITIVES (traits/structs/enums)
- `ModelSession` (struct/schema): persisted session identity, lifecycle state, role/WP/MT bindings, memory policy, consent/capability bindings.
- `SessionMessage` (struct/schema): artifact-linked message thread entries with `content_hash` and `content_artifact_id`.
- `ModelRunJob` (job payload/schema): `job_kind="model_run"` with lane/priority/concurrency/backoff/timeout/budget fields.
- `SessionSchedulerConfig` (struct): global/per-provider/per-model concurrency and rate limits.
- `SessionRegistry` (runtime component/interface): authoritative tracker for active sessions and parent-child relationships.

### CLEARLY_COVERS (5-point checklist)
- Appears in Main Body: [ ] PASS
- Explicitly named: [ ] PASS
- Specific: [ ] PASS
- Measurable acceptance criteria: [ ] PASS
- No ambiguity: [ ] PASS
- CLEARLY_COVERS_VERDICT: PASS
- CLEARLY_COVERS_REASON: Spec sections 4.3.9.12 and 4.3.9.13 are normative and provide concrete schema + invariant requirements for exactly this WP.
- AMBIGUITY_FOUND: NO
- AMBIGUITY_REASON: NONE

### ENRICHMENT
- ENRICHMENT_NEEDED: NO
- REASON_NO_ENRICHMENT: Current authoritative spec already defines required normative behavior for ModelSession persistence and Session Scheduler mechanics; this WP is implementation of existing rules, not a spec gap.

#### PROPOSED_SPEC_ENRICHMENT (VERBATIM) (required if ENRICHMENT_NEEDED=YES)
```md
<not applicable; ENRICHMENT_NEEDED=NO>
```

### SPEC_ANCHORS (REQUIRED)

#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]
- CONTEXT_START_LINE: 31064
- CONTEXT_END_LINE: 31138
- CONTEXT_TOKEN: A `ModelSession` is the persistent, addressable unit of a model conversation in Handshake.
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]

  A `ModelSession` is the persistent, addressable unit of a model conversation in Handshake.

  ModelSession:
    session_id: string
    parent_session_id: string | null
    spawn_depth: int
    state: enum [CREATED, ACTIVE, PAUSED, BLOCKED, COMPLETED, FAILED, CANCELLED]
    model_id: ModelId
    backend: ModelBackend
    parameter_class: ParameterClass
    role: string
    wp_id: string | null
    mt_id: string | null
    work_profile_id: string | null
    execution_mode: ExecutionMode
    memory_policy: enum [EPHEMERAL, SESSION_SCOPED, WORKSPACE_SCOPED]
    consent_receipt_id: string | null
    capability_grants: string[]
    capability_token_ids: string[] | null

  SessionMessage:
    message_id: string
    session_id: string
    role: enum [SYSTEM, USER, ASSISTANT, TOOL_CALL, TOOL_RESULT]
    content_hash: string
    content_artifact_id: string

  Invariants include:
  - INV-SESS-001 stable session_id across UI/events/governance/artifacts
  - INV-SESS-002 content stored as artifacts, not inline in event logs
  - INV-SESS-003 one executing model call per active session

  Storage:
  - Phase 1: SQLite tables `model_sessions` and `session_messages` in workspace DB.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 4.3.9.13 Session Scheduler: Model Calls as Queued Work (Normative) [ADD v02.137]
- CONTEXT_START_LINE: 31203
- CONTEXT_END_LINE: 31275
- CONTEXT_TOKEN: job_kind: "model_run"
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 4.3.9.13 Session Scheduler: Model Calls as Queued Work (Normative) [ADD v02.137]

  ModelRunJob:
    job_kind: "model_run"
    session_id: string
    lane: enum [PRIMARY, SUBAGENT, BACKGROUND, VALIDATION]
    priority: int
    concurrency_group: string | null
    max_retries: int
    retry_backoff: enum [FIXED, EXPONENTIAL]
    timeout_ms: int
    cancellation_token: string | null
    max_tokens_budget: int | null
    estimated_cost_usd: number | null

  SessionSchedulerConfig:
    max_concurrent_sessions_global: int
    max_concurrent_sessions_per_provider: int
    max_concurrent_sessions_per_model: int
    rate_limit_requests_per_minute: int | null
    rate_limit_tokens_per_minute: int | null

  Scheduling invariants include:
  - INV-SCHED-001 route all AI_ENABLED model calls through scheduler
  - INV-SCHED-002 enqueue (not drop) when limits are hit; show QUEUED
  - INV-SCHED-003 deterministic rate-limit backoff logging
  - INV-SCHED-004 cooperative cancellation -> CANCELLED
  - INV-SCHED-005 lane isolation so primary lane is not starved

  Flight Recorder events:
  - FR-EVT-SESS-SCHED-001 session_scheduler.enqueue
  - FR-EVT-SESS-SCHED-002 session_scheduler.dispatch
  - FR-EVT-SESS-SCHED-003 session_scheduler.rate_limited
  - FR-EVT-SESS-SCHED-004 session_scheduler.cancelled
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.139.md 7.2.0.5 Multi-Model Infrastructure (Normative) [UPDATED v02.137]
- CONTEXT_START_LINE: 29870
- CONTEXT_END_LINE: 29896
- CONTEXT_TOKEN: The system MUST maintain a `SessionRegistry` that tracks all active `ModelSession` instances
- EXCERPT_ASCII_ESCAPED:
  ```text
  #### 7.2.0.5 Multi-Model Infrastructure (Normative) [UPDATED v02.137]

  The `MultiModelSession` is a governed runtime primitive.
  MultiModelSession includes:
  - active_sessions: Record<string, ModelSession>
  - scheduler_config: SessionSchedulerConfig

  Session Registry (normative):
  - The system MUST maintain a SessionRegistry that tracks active ModelSession instances,
    their states, and parent-child relationships.
  - The registry is the authority for session lifecycle.
  - UI and scheduler query this registry.
  ```
