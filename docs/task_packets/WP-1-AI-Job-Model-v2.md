# Task Packet: WP-1-AI-Job-Model-v2

## Metadata
- TASK_ID: WP-1-AI-Job-Model-v2
- DATE: 2025-12-26
- REQUESTOR: User
- AGENT_ID: orchestrator-gemini
- ROLE: Orchestrator
- STATUS: DONE [VALIDATED]
- RISK_TIER: HIGH
- USER_SIGNATURE: ilja261220250149

## Scope
- **What**: Implement normative AI Job Model structs and persistence per §2.6.6.2.8.
- **Why**: Harden the system against "minimal implementation" failures and provide the foundation for the Workflow Engine and Flight Recorder linkage.
- **IN_SCOPE_PATHS**:
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/storage/sqlite.rs
  * src/backend/handshake_core/src/models.rs
  * src/backend/handshake_core/migrations/0006_expand_ai_job_model.sql
- **OUT_OF_SCOPE**:
  * Implementing the full Workflow Engine logic (WP-1-Workflow-Engine).
  * UI surfacing of granular job metrics (Phase 2).

## Quality Gate
- **RISK_TIER**: HIGH
  - Justification: Core architectural entity; failure blocks workflow execution and auditability.
- **TEST_PLAN**:
  ```bash
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just validator-spec-regression
  just validator-scan WP-1-AI-Job-Model-v2
  just validator-hygiene-full
  ```
- **DONE_MEANS**:
  * ✅ `AiJob`, `JobMetrics`, `JobState`, and `AccessMode` structs/enums match §2.6.6.2.8 in v02.88 exactly.
  * ✅ Database schema updated with new columns (`trace_id`, `workflow_run_id`, `status_reason`, `metrics` JSON).
  * ✅ `Database` trait updated to use the expanded `AiJob` struct for all job-related methods.
  * ✅ Unit tests verify state transitions and serialization/deserialization of the expanded model.
  * ✅ No forbidden patterns (unwrap/expect/panic/dbg/Value in domain).

## ROLLBACK_HINT
```bash
git revert <commit-sha>
```

## BOOTSTRAP
- **FILES_TO_OPEN**:
  * docs/START_HERE.md
  * docs/SPEC_CURRENT.md
  * Handshake_Master_Spec_v02.88.md
  * src/backend/handshake_core/src/storage/mod.rs
  * src/backend/handshake_core/src/models.rs
- **SEARCH_TERMS**:
  * "pub struct AiJob"
  * "JobState"
  * "Database"
  * "create_ai_job"
- **RUN_COMMANDS**:
  ```bash
  just dev
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  ```
- **RISK_MAP**:
  * "Migration failure" -> Database layer
  * "Serialization mismatch" -> Storage layer
  * "Trait breaking change" -> Entire backend

## Authority
- **SPEC_ANCHOR**: §2.6.6.2.8 (Normative Rust Types)
- **SPEC_CURRENT**: Handshake_Master_Spec_v02.88.md
- **Codex**: Handshake Codex v1.4.md
- **Task Board**: docs/TASK_BOARD.md

## Notes
- **Assumptions**: None.
- **Open Questions**: None.
- **Dependencies**: WP-1-Flight-Recorder (VALIDATED) provides the trace_id infrastructure.

---

## VALIDATION REPORT — WP-1-AI-Job-Model-v2 (2025-12-26)
Verdict: PASS

**Scope Inputs:**
- Task Packet: `docs/task_packets/WP-1-AI-Job-Model-v2.md`
- Spec: `Handshake_Master_Spec_v02.88 §2.6.6.2.8`
- Coder: [[coder gpt codex]]

**Files Checked:**
- `src/backend/handshake_core/src/storage/mod.rs`
- `src/backend/handshake_core/src/storage/sqlite.rs`
- `src/backend/handshake_core/src/storage/postgres.rs`
- `src/backend/handshake_core/src/workflows.rs`
- `src/backend/handshake_core/src/jobs.rs`
- `src/backend/handshake_core/migrations/0006_expand_ai_job_model.sql`

**Findings:**
- [§2.6.6.2.8-REQ] Struct Alignment: PASS. `AiJob`, `JobMetrics`, `JobState`, `AccessMode`, and `SafetyMode` match the normative spec exactly. Evidence: `mod.rs:300-360`.
- [§2.6.6.2.8-REQ] Persistence: PASS. SQLite and Postgres backends correctly map the new columns, including JSON-serialized metrics and operations. Evidence: `sqlite.rs:45-140`, `postgres.rs:134-230`.
- [Observability]: PASS. `trace_id` is now a mandatory field in `NewAiJob` and is correctly propagated from `jobs.rs` and `workflows.rs`.
- [Build Restoration]: PASS. Type mismatches in `workflows.rs` were resolved by parsing `workflow_run.id` as a `Uuid` before updating job status.
- [Forbidden Patterns]: PASS. Audit confirms only standard `unwrap_or_else` usage for trace ID fallbacks.
- [Red Team Audit]: PASS. Migration script includes COALESCE fallbacks for existing data, preventing data corruption during the schema update.

**REASON FOR PASS:**
The implementation fulfills the core schema hardening requirements of §2.6.6.2.8. It successfully transitions the system from a "minimal" job model to a normative, trace-aware model that supports granular metrics and linked workflow runs. The restoration of the build unblocks all concurrent worklines.

---

**Last Updated:** 2025-12-26
**User Signature Locked:** ilja261220250149
