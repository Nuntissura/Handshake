# Task Packet: WP-1-ModelSession-Core-Scheduler-v1

## METADATA
- TASK_ID: WP-1-ModelSession-Core-Scheduler-v1
- WP_ID: WP-1-ModelSession-Core-Scheduler-v1
- BASE_WP_ID: WP-1-ModelSession-Core-Scheduler
- DATE: 2026-03-01T20:04:08.068Z
- MERGE_BASE_SHA: 6e763ff05dbc7e52c75eaf83ee37a3168da7d1ac
- REQUESTOR: Operator (ilja)
- AGENT_ID: CodexCLI-GPT-5 (Orchestrator)
- ROLE: Orchestrator
- AGENTIC_MODE: NO
- ORCHESTRATOR_MODEL: N/A
- ORCHESTRATION_STARTED_AT_UTC: N/A
- CODER_MODEL: <unclaimed>
- CODER_REASONING_STRENGTH: <unclaimed> (LOW | MEDIUM | HIGH | EXTRA_HIGH)
- **Status:** Ready for Dev
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja010320262103
- PACKET_FORMAT_VERSION: 2026-02-01

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-ModelSession-Core-Scheduler-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## USER_CONTEXT (APPEND-ONLY)
- You asked to start `WP-1-ModelSession-Core-Scheduler-v1` because it unlocks the most downstream backend work.
- Non-technical summary: this packet builds the "traffic controller" for AI sessions so multiple model tasks can run safely, be paused/cancelled, and always leave an audit trail.
- Why this matters now: many upcoming session/telemetry/safety packets depend on this foundation being in place first.

## SCOPE
- What: Implement Phase 1 backend foundations for `ModelSession` persistence and the `Session Scheduler` baseline (`job_kind="model_run"`), including deterministic queueing/dispatch/cancellation/concurrency behavior and minimal visibility via existing Job History + Flight Recorder surfaces.
- Why: The current master spec makes session orchestration a normative requirement; without this layer, multi-session execution remains brittle, non-deterministic, and weakly auditable.
- IN_SCOPE_PATHS:
  - .GOV/refinements/WP-1-ModelSession-Core-Scheduler-v1.md
  - .GOV/task_packets/WP-1-ModelSession-Core-Scheduler-v1.md
  - .GOV/validator_gates/WP-1-ModelSession-Core-Scheduler-v1.json
  - .GOV/roles_shared/TASK_BOARD.md
  - .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/storage/postgres.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
  - src/backend/handshake_core/migrations/0012_model_sessions.sql
  - src/backend/handshake_core/migrations/0012_model_sessions.down.sql
  - src/backend/handshake_core/tests/model_session_scheduler_tests.rs
- OUT_OF_SCOPE:
  - Session spawn lifecycle contract (tracked in `WP-1-Session-Spawn-Contract-v1`).
  - Session-scoped capability intersection and consent-gate lifecycle (tracked in `WP-1-Session-Scoped-Capabilities-Consent-Gate-v1`).
  - Provider feature parity and adapters for advanced tool-calling/streaming coverage (tracked in `WP-1-Provider-Feature-Coverage-Agentic-Ready-v1`).
  - Workspace isolation hardening for parallel sessions (tracked in `WP-1-Workspace-Safety-Parallel-Sessions-v1`).
  - Crash checkpoint/resume implementation (tracked in `WP-1-Session-Crash-Recovery-Checkpointing-v1`).
  - ModelSessionSpan and full observability family expansion (tracked in `WP-1-Session-Observability-Spans-FR-v1`).

## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Include Waiver ID, Date, Scope, and Justification.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
# Run before handoff:
just pre-work WP-1-ModelSession-Core-Scheduler-v1

# Governance + deterministic checks:
just gov-check
just validator-scan
just validator-dal-audit

# Backend tests:
cargo test --manifest-path src/backend/handshake_core/Cargo.toml
cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test model_session_scheduler_tests

# Post-work deterministic validation:
just cargo-clean
just post-work WP-1-ModelSession-Core-Scheduler-v1 --range 6e763ff05dbc7e52c75eaf83ee37a3168da7d1ac..HEAD
```

### DONE_MEANS
- `ModelSession` and `SessionMessage` persistence exists in workspace DB with artifact-first thread storage (`content_hash`, `content_artifact_id`) and no inline content leakage in governance/FR payloads.
- `job_kind="model_run"` is represented in job model paths and is dispatched by a dedicated session scheduler path (not ad-hoc direct completion calls in production paths).
- Concurrency + queue semantics enforce spec invariants: when limits are reached, jobs are queued (not dropped), visible as queued, and later dispatched deterministically.
- Cancellation is cooperative and results in cancelled semantics (not failed semantics) with deterministic reason attribution.
- Session scheduler emits and validates `FR-EVT-SESS-SCHED-001..004` (`enqueue`, `dispatch`, `rate_limited`, `cancelled`) with required payload fields.
- Session registry authority is implemented for scheduler-facing lifecycle reads (`session_id`, state, parent-child relation support), and scheduler/UI paths query that authority.

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_v02.139.md (recorded_at: 2026-03-01T20:04:08.068Z)
- SPEC_TARGET: .GOV/roles_shared/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ANCHOR:
  - Handshake_Master_Spec_v02.139.md 4.3.9.12 ModelSession: First-Class Session Data Model (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.139.md 4.3.9.13 Session Scheduler: Model Calls as Queued Work (Normative) [ADD v02.137]
  - Handshake_Master_Spec_v02.139.md 7.2.0.5 Multi-Model Infrastructure (Normative) [UPDATED v02.137]
- Codex: Handshake Codex v1.4.md
- Task Board: .GOV/roles_shared/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/WP_TRACEABILITY_REGISTRY.md

## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Prior packet artifacts:
  - .GOV/task_packets/stubs/WP-1-ModelSession-Core-Scheduler-v1.md (stub; non-executable)
- Preserved requirements from stub:
  - First-class `ModelSession` persistence with artifact-first message storage.
  - Session Scheduler baseline for `model_run` with queueing, cancellation, and concurrency limits.
  - Minimal system visibility through Job History + Flight Recorder hooks.
- Changes in this activated packet:
  - Converted stub into executable signed packet with SPEC_CURRENT v02.139 anchors.
  - Added measurable DONE_MEANS, deterministic TEST_PLAN, exact IN_SCOPE_PATHS, and coder bootstrap details.

## BOOTSTRAP
- FILES_TO_OPEN:
  - .GOV/roles_shared/START_HERE.md
  - .GOV/roles_shared/SPEC_CURRENT.md
  - .GOV/roles_shared/ARCHITECTURE.md
  - Handshake_Master_Spec_v02.139.md
  - .GOV/refinements/WP-1-ModelSession-Core-Scheduler-v1.md
  - .GOV/task_packets/stubs/WP-1-ModelSession-Core-Scheduler-v1.md
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/jobs.rs
  - src/backend/handshake_core/src/api/jobs.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/sqlite.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/flight_recorder/duckdb.rs
- SEARCH_TERMS:
  - "ModelSession"
  - "session_messages"
  - "model_sessions"
  - "job_kind"
  - "model_run"
  - "SessionSchedulerConfig"
  - "INV-SCHED-001"
  - "INV-SCHED-002"
  - "FR-EVT-SESS-SCHED-001"
  - "FR-EVT-SESS-SCHED-004"
  - "session_registry"
  - "queue_wait_ms"
  - "concurrency_group"
  - "rate_limited"
  - "cancelled_by"
- RUN_COMMANDS:
  ```bash
  rg -n "ModelSession|model_run|SessionScheduler|FR-EVT-SESS-SCHED|session_registry" src/backend/handshake_core/src
  just pre-work WP-1-ModelSession-Core-Scheduler-v1
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just post-work WP-1-ModelSession-Core-Scheduler-v1 --range 6e763ff05dbc7e52c75eaf83ee37a3168da7d1ac..HEAD
  ```
- RISK_MAP:
  - "scheduler bypass" -> "model invocations execute outside governance and cannot be reliably audited"
  - "queue drop under load" -> "user-visible work silently disappears and completion state becomes untrustworthy"
  - "state mapping drift" -> "ModelSession state diverges from underlying JobState and breaks operator reasoning"
  - "artifact leakage" -> "session message content leaks into logs/events violating artifact-first discipline"
  - "lane starvation" -> "primary lane actions stall behind subagent/background traffic"

## SKELETON
- Proposed interfaces/types/contracts:
  - `ModelSession` persistence contract in workspace DB (`model_sessions`, `session_messages`) with artifact handle references for message content.
  - `SessionSchedulerConfig` runtime settings bound to scheduler dispatch logic (global/provider/model limits, rate limits).
  - `ModelRunJob` job payload contract (`job_kind="model_run"`, `lane`, `priority`, `concurrency_group`, `cancellation_token`, `timeout_ms`, budgets).
  - Session scheduler API: enqueue, dispatch, cancel, and queue-state query with deterministic ordering policy.
  - Flight Recorder payload contracts for `FR-EVT-SESS-SCHED-001..004`.
- Open questions:
  - Should queue ordering be strict `(priority ASC, created_at ASC, job_id ASC)` or include lane-weighting tie-breakers from day one?
  - Do we extend existing job tables for scheduler queue metadata, or create dedicated session scheduler tables for clearer migration rollback?
- Notes:
  - This packet is a dependency unlocker for session spawn, session safety, session observability, and multi-model lifecycle telemetry packets.
  - Coder must keep this packet focused on baseline foundations; downstream session feature packets should not be absorbed here.

## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: YES
- TRUST_BOUNDARY: client->server->storage->flight_recorder
- SERVER_SOURCES_OF_TRUTH:
  - Server-side persisted `model_sessions`/`session_messages` rows are source-of-truth for lifecycle and thread metadata.
  - Scheduler queue state is derived from server-side job and scheduler records, not client-provided state snapshots.
  - FR scheduler events are emitted from server transitions only (enqueue/dispatch/rate-limit/cancel).
- REQUIRED_PROVENANCE_FIELDS:
  - `session_id`, `job_id`, `job_kind`, `lane`, `priority`, `concurrency_group`
  - `queue_wait_ms`, `attempt`, `backoff_ms`, `cancelled_by`, `reason`
  - `content_hash`, `content_artifact_id` (for message thread entries), never inline message content
- VERIFICATION_PLAN:
  - Unit + integration tests prove queue/not-drop behavior at concurrency limits.
  - Deterministic state transition tests assert `ModelSession.state` mapping against active `model_run` `JobState`.
  - FR schema validation tests assert required keys and reject malformed scheduler payloads.
- ERROR_TAXONOMY_PLAN:
  - `session_not_found`
  - `scheduler_invariant_violation`
  - `concurrency_limit_exceeded_queued`
  - `scheduler_dispatch_denied_missing_receipt` (integration placeholder for consent-gate packet)
  - `scheduler_cancel_invalid_state`
- UI_GUARDRAILS:
  - Display queued vs running state from server-sourced fields only.
  - Disable cancel action for terminal states.
  - Show last scheduler event and reason for blocked/rate-limited status.
- VALIDATOR_ASSERTIONS:
  - `model_run` invocations are scheduler-routed in production code paths.
  - Queue overflow behavior enqueues rather than drops.
  - FR-EVT-SESS-SCHED-001..004 events are emitted/validated with required fields.
  - Message thread storage remains artifact-first (`content_hash` + `content_artifact_id`).

## IMPLEMENTATION
- Added `job_kind="model_run"` to storage job-kind modeling and parsing.
- Added first-class session persistence models and DAL contract methods:
  - `ModelSessionState`, `SessionMessageRole`, `ModelSession`, `NewModelSession`, `SessionMessage`, `NewSessionMessage`.
  - `Database` trait methods for upsert/read/update model session and append/list session messages.
- Implemented SQLite and Postgres persistence paths for model session registry + artifact-first session thread refs:
  - `model_sessions` and `session_messages` schema ensure (`CREATE TABLE IF NOT EXISTS`) in both backends.
  - strict `content_hash` validation (`sha256` hex length/charset) for session messages.
- Implemented session scheduler execution path in workflow runtime for `model_run`:
  - enqueue path (`Queued`) with deterministic ordering, single-dispatch loop lock, and queue-not-drop behavior under limits.
  - dispatch path (`Running`) with workflow/node execution records and scheduler kick on terminal completion.
  - cooperative cancellation path mapping to `cancelled` semantics (not `failed`) and session state sync.
  - scheduler-facing model session authority reads (`get_model_session*`) used for dispatch eligibility and limits.
- Added and wired FR scheduler event family:
  - `FR-EVT-SESS-SCHED-001..004` via new `FlightRecorderEventType` variants.
  - strict payload validators for enqueue/dispatch/rate_limited/cancelled.
  - DuckDB readback mapping for new event strings.
- Added targeted integration tests in `model_session_scheduler_tests.rs` for:
  - artifact-first session persistence,
  - queue/not-drop + deterministic dispatch ordering,
  - cooperative cancellation -> cancelled semantics,
  - FR scheduler payload validation.

## HYGIENE
- Ran required bootstrap gates:
  - `just hard-gate-wt-001` (pass)
  - `just pre-work WP-1-ModelSession-Core-Scheduler-v1` (pass)
- Ran required validation and test commands:
  - `just gov-check` (pass)
  - `just validator-scan` (fail; baseline pre-existing out-of-scope findings in `spec_router/*` and existing placeholder-token hits)
  - `just validator-dal-audit` (pass)
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test model_session_scheduler_tests` (pass)
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml` (fail; environment memory/paging issue with `os error 1455` / crate metadata mmap failures)
  - `just cargo-clean` (pass)
  - `just post-work WP-1-ModelSession-Core-Scheduler-v1 --range 6e763ff05dbc7e52c75eaf83ee37a3168da7d1ac..HEAD` (rerun after this packet update)
- Migration naming correction handling:
  - Operator authorized correction acknowledged: avoid conflicting `0012_*`.
  - No `0012_*` migration files were created in this changeset.
  - No `0014_model_sessions.sql` file was added in this diff because persistence schema is currently ensured in DAL runtime paths for both SQLite and Postgres.

## VALIDATION
- **Target File**: `.GOV/task_packets/WP-1-ModelSession-Core-Scheduler-v1.md`
- **Start**: 1
- **End**: 275
- **Line Delta**: 275
- **Pre-SHA1**: `0000000000000000000000000000000000000000`
- **Post-SHA1**: `750d52169cf3c132ab19fe955543c6ff718403ae`
- **Gates Passed**:
  - [x] anchors_present
  - [x] window_matches_plan
  - [x] rails_untouched_outside_window
  - [x] filename_canonical_and_openable
  - [x] pre_sha1_captured
  - [x] post_sha1_captured
  - [x] line_delta_equals_expected
  - [x] all_links_resolvable
  - [x] manifest_written_and_path_returned
  - [x] current_file_matches_preimage
- **Lint Results**: `just validator-scan` baseline fail on existing out-of-scope findings (see EVIDENCE).
- **Artifacts**: N/A
- **Timestamp**: 2026-03-01
- **Operator**: coder-a (WP-1)
- **Spec Target Resolved**: .GOV/roles_shared/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.139.md
- **Notes**: range preimage for this file is absent at `6e763ff05dbc7e52c75eaf83ee37a3168da7d1ac` because this packet file is added later in branch history.

## STATUS_HANDOFF
- Current WP_STATUS: READY_FOR_VALIDATOR (with known environment/baseline blockers documented).
- What changed in this update:
  - Implemented model session persistence + scheduler path + FR event validation + targeted test suite.
  - Filled packet implementation/hygiene/validation/evidence sections with concrete details.
  - Recorded deterministic command outcomes and blockers.
  - Recorded migration naming correction context:
    - Initial packet sketch referenced `0012_model_sessions*`.
    - Repo already has occupied `0012_*` and `0013_*`.
    - Operator-authorized correction was acknowledged; no conflicting `0012_*` files were created.
    - If migration artifact is later required, use `0014_model_sessions.sql` / `.down.sql`.
- Next step / handoff hint:
  - Validator can audit code/evidence mapping now.
  - Full-suite `cargo test` rerun may require increased Windows paging memory in this environment.

## EVIDENCE_MAPPING
- REQUIREMENT: "`ModelSession` and `SessionMessage` persistence exists in workspace DB with artifact-first thread storage (`content_hash`, `content_artifact_id`)."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1306`
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1743`
  - EVIDENCE: `src/backend/handshake_core/src/storage/sqlite.rs:3972`
  - EVIDENCE: `src/backend/handshake_core/src/storage/postgres.rs:3381`
- REQUIREMENT: "`job_kind=\"model_run\"` is represented in job model paths and dispatched by a dedicated session scheduler path."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:936`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2897`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2576`
- REQUIREMENT: "Concurrency + queue semantics enforce spec invariants: enqueue (not drop) at limits and deterministic dispatch behavior."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2162`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2439`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:260`
- REQUIREMENT: "Cancellation is cooperative and results in cancelled semantics (not failed semantics)."
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2647`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2809`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:388`
- REQUIREMENT: "Session scheduler emits and validates `FR-EVT-SESS-SCHED-001..004` payloads."
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:162`
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/mod.rs:4027`
  - EVIDENCE: `src/backend/handshake_core/src/flight_recorder/duckdb.rs:896`
  - EVIDENCE: `src/backend/handshake_core/tests/model_session_scheduler_tests.rs:463`
- REQUIREMENT: "Session registry authority is implemented for scheduler-facing lifecycle reads (`session_id`, state, parent-child support), queried by scheduler paths."
  - EVIDENCE: `src/backend/handshake_core/src/storage/mod.rs:1745`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2463`
  - EVIDENCE: `src/backend/handshake_core/src/workflows.rs:2770`

## EVIDENCE
- COMMAND: `just hard-gate-wt-001`
  - EXIT_CODE: 0
  - PROOF_LINES: `GATE PASS: Workflow sequence verified.`
- COMMAND: `just pre-work WP-1-ModelSession-Core-Scheduler-v1`
  - EXIT_CODE: 0
  - PROOF_LINES: `pre-work checks completed`
- COMMAND: `just gov-check`
  - EXIT_CODE: 0
  - PROOF_LINES: `SPEC_CURRENT ok: Handshake_Master_Spec_v02.139.md`; `worktree-concurrency-check ok`
- COMMAND: `just validator-scan`
  - EXIT_CODE: 1
  - PROOF_LINES: `validator-scan: FAIL - findings detected`; `FORBIDDEN_PATTERN (rust) "expect\\(" ... src/backend/handshake_core/src/spec_router/spec_prompt_pack.rs`; `PLACEHOLDER/MOCK "placeholder" ... spec_router/* and existing workflows token`
  - NOTES: baseline pre-existing out-of-scope findings; no new in-scope WP code finding was introduced by this command output.
- COMMAND: `just validator-dal-audit`
  - EXIT_CODE: 0
  - PROOF_LINES: `validator-dal-audit: PASS (DAL checks clean).`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml --test model_session_scheduler_tests`
  - EXIT_CODE: 0
  - PROOF_LINES: `running 4 tests`; `test result: ok. 4 passed; 0 failed`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml`
  - EXIT_CODE: 1
  - PROOF_LINES: `memory allocation ... failed`; `os error 1455`; `can't find crate for handshake_core` (cascade after memory/paging failure)
  - NOTES: environment blocker (Windows paging/memory pressure), not a deterministic functional failure in targeted WP tests.
- COMMAND: `just cargo-clean`
  - EXIT_CODE: 0
  - PROOF_LINES: `Removed 1304 files, 6.9GiB total`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
