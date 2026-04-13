# TASK_PACKET_TEMPLATE

Copy this into each new task packet and fill all fields.

Requirements:
- Keep packets ASCII-only (required by deterministic gates).
- Use SPEC_BASELINE for provenance (spec at creation time).
- Use SPEC_TARGET as the authoritative spec for closure/revalidation (usually .GOV/spec/SPEC_CURRENT.md).
- WP_ID and filename MUST NOT include date/time stamps; use `-v{N}` for revisions (e.g., `WP-1-Tokenization-Service-v3`).
- If multiple packets exist for the same Base WP, update `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` (Base WP -> Active Packet).
- Packet metadata is the authoritative lifecycle truth. `TASK_BOARD.md`, `WP_TRACEABILITY_REGISTRY.md`, and `BUILD_ORDER.md` are projections and must reconcile to this header.
- Active packet rule: the packet mapped by `BASE_WP_ID` in `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md` is the current contract. Any other official packet with the same `BASE_WP_ID` is older history and must be tracked as `SUPERSEDED` on the Task Board.
- For `REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1`, this packet is auto-hydrated from the signed refinement; manual drift is forbidden and `just phase-check STARTUP ... CODER` enforces alignment.

---

# Task Packet: WP-1-Distillation-v2

## METADATA
- TASK_ID: WP-1-Distillation-v2
- WP_ID: WP-1-Distillation-v2
- BASE_WP_ID: WP-1-Distillation
- DATE: 2026-04-13T22:59:04.968Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just phase-check HANDOFF ... CODER --range` evidence)
- REQUESTOR: Operator
- AGENT_ID: Orchestrator
- ROLE: Orchestrator
- REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1
- PACKET_HYDRATION_PROFILE: HYDRATED_RESEARCH_V1
- WORKFLOW_LANE: ORCHESTRATOR_MANAGED
<!-- Required before packet creation: MANUAL_RELAY | ORCHESTRATOR_MANAGED -->
- EXECUTION_OWNER: CODER_A
<!-- Required before packet creation: CODER_A .. CODER_Z -->
- WORKFLOW_AUTHORITY: ORCHESTRATOR
<!-- Current repo-governance owner for workflow steering and hard-gate progression. -->
- TECHNICAL_ADVISOR: WP_VALIDATOR
<!-- Advisory WP-scoped validator; may question and challenge coder work but does not own final merge authority. -->
- TECHNICAL_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final technical verdict authority across the WP batch. -->
- MERGE_AUTHORITY: INTEGRATION_VALIDATOR
<!-- Final merge-to-main authority. -->
- AGENTIC_MODE: NO
<!-- Allowed: YES | NO. Default NO; set to YES only with explicit operator-authorized sub-agent use. -->
- ORCHESTRATOR_MODEL: N/A
<!-- Required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- ACTIVATION_MANAGER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Optional but authoritative when Activation Manager launch or repair resumes from the packet. -->
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: claude-opus-4-6
- CODER_REASONING_STRENGTH: EXTRA_HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: HANDSHAKE_ACP_BROKER
- SESSION_HOST_FALLBACK: SYSTEM_TERMINAL_REPAIR_ONLY
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_ACP_DIRECT_HEADLESS_PRIMARY
- ROLE_SESSION_RUNTIME: CLI
- CLI_SESSION_TOOL: codex
- SESSION_PLUGIN_BRIDGE_ID: handshake.handshake-session-bridge
- SESSION_PLUGIN_BRIDGE_COMMAND: handshakeSessionBridge.processLaunchQueue
- SESSION_PLUGIN_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_CONTROL_REQUESTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_REQUESTS.jsonl
- SESSION_CONTROL_RESULTS_FILE: ../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl
- SESSION_COMPATIBILITY_SURFACE: VSCODE_PLUGIN_REPAIR_ONLY
- SESSION_COMPATIBILITY_QUEUE_FILE: ../gov_runtime/roles_shared/SESSION_LAUNCH_REQUESTS.jsonl
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Distillation-v2
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: gpt-5.4
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Distillation-v2
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-distillation-v2
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Distillation-v2
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Distillation-v2
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup WP_VALIDATOR
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP_VALIDATOR WP-1-Distillation-v2
- INTEGRATION_VALIDATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: gpt-5.4
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Distillation-v2
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Distillation-v2
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup INTEGRATION_VALIDATOR
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next INTEGRATION_VALIDATOR WP-1-Distillation-v2
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Distillation-v2
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup VALIDATOR -> just external-validator-brief WP-1-Distillation-v2
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY | ABANDONED
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_DUAL_TRACK_FIELDS: MECHANICAL_TRACK_VERDICT | SPEC_RETENTION_TRACK_VERDICT
<!-- For PACKET_FORMAT_VERSION >= 2026-04-05 and RISK_TIER=MEDIUM|HIGH, both governed dual-track fields become mandatory at validator closeout. -->
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- DATA_CONTRACT_PROFILE: LLM_FIRST_DATA_V1
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01. Allowed: NONE | LLM_FIRST_DATA_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Ready for Dev
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) | Validated (ABANDONED) -->
- MAIN_CONTAINMENT_STATUS: NOT_STARTED
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: NONE
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: NONE
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: N/A
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NONE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NONE | NOT_REQUIRED | FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED -->
- PACKET_WIDENING_EVIDENCE: N/A
<!-- Use follow-on/superseding WP id, audit id, or short rationale when widening is required. -->
- ZERO_DELTA_PROOF_ALLOWED: NO
<!-- Allowed: YES | NO. YES => deterministic post-work may accept an empty diff only for an explicitly proof-only/status-sync packet. -->
- RISK_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH -->
- BUILD_ORDER_DOMAIN: BACKEND
<!-- Allowed: BACKEND | FRONTEND | GOV | CROSS_BOUNDARY -->
- BUILD_ORDER_TECH_BLOCKER: NO
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Micro-Task-Executor, WP-1-Artifact-System-Foundations
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-MTE-LoRA-Wiring
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: WP-1-MTE-LoRA-Wiring-v1, WP-1-Session-Spawn-Conversation-Distillation-v1
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Distillation-v2
- LOCAL_WORKTREE_DIR: ../wtc-distillation-v2
- REMOTE_BACKUP_BRANCH: feat/WP-1-Distillation-v2
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Distillation-v2
- REMOTE_BACKUP_LIFECYCLE: TEMPORARY
<!-- WP backup branches may be deleted after Operator-approved cleanup; later dead links are non-blocking. -->
- BACKUP_PUSH_STATUS: REQUIRED_BEFORE_DESTRUCTIVE_OPS
<!-- Treat the WP backup branch as the phase-boundary recovery branch. Preserve the latest committed restart-safe state at packet/refinement checkpoint, bootstrap claim, skeleton checkpoint, skeleton approval, and before destructive/state-hiding local git actions. -->
- HEARTBEAT_INTERVAL_MINUTES: 15
<!-- Integer minutes; update runtime status/receipts on event boundaries and at this interval only while actively working. -->
- STALE_AFTER_MINUTES: 45
<!-- Integer minutes; heartbeat older than this threshold is stale. -->
- MAX_CODER_REVISION_CYCLES: 3
- MAX_VALIDATOR_REVIEW_CYCLES: 3
- MAX_RELAY_ESCALATION_CYCLES: 2
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Distillation-v2
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Distillation-v2/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Distillation-v2/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Distillation-v2/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja140420260053
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Section 9.1.1 Data model (SkillBankLogEntry, QualityMeta, TelemetryMeta, PrivacyMeta) | CODE_SURFACES: src/backend/handshake_core/src/models/skill_bank.rs | TESTS: cargo test -p handshake_core -- skill_bank | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Section 9.1.2 SQL schema (skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run) | CODE_SURFACES: src/backend/handshake_core/migrations/0017_skill_bank_distillation.sql | TESTS: cargo test -p handshake_core -- migration | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Section 9.1.3.1 compute_data_trust_score | CODE_SURFACES: src/backend/handshake_core/src/distillation/scoring.rs | TESTS: cargo test -p handshake_core -- data_trust_score | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Section 9.1.3.2 build_distill_dataset (new + replay batches) | CODE_SURFACES: src/backend/handshake_core/src/distillation/dataset.rs | TESTS: cargo test -p handshake_core -- distill_dataset | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Section 9.1.4 Evaluation and promotion gates | CODE_SURFACES: src/backend/handshake_core/src/distillation/eval.rs | TESTS: cargo test -p handshake_core -- eval_promotion | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Section 5.3.6 Distillation observability (FR events per stage) | CODE_SURFACES: src/backend/handshake_core/src/flight_recorder/mod.rs | TESTS: cargo test -p handshake_core -- flight_recorder_distill | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Section 2.6.6.8.13 Learning Integration (DistillationCandidate persistence) | CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | TESTS: cargo test -p handshake_core -- distillation_candidate | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Section 9 PII/secret redaction (redact_entry) | CODE_SURFACES: src/backend/handshake_core/src/distillation/redaction.rs | TESTS: cargo test -p handshake_core -- redaction | EXAMPLES: Fixture: SkillBankLogEntry with all 52 columns populated (golden test row), Fixture: DistillationCandidate with teacher/student snapshot refs and trust score, Fixture: AdapterCheckpoint with parent lineage chain (3-deep), Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs (existing DistillationInfo, PendingDistillationCandidate)
  - src/backend/handshake_core/src/flight_recorder/mod.rs (existing MicroTaskDistillationCandidate event)
  - src/backend/handshake_core/src/storage/mod.rs (existing JobKind::DistillationEval)
  - src/backend/handshake_core/src/capabilities.rs (needs distillation gates)
  - src/backend/handshake_core/migrations/ (latest migration is 0016)
- REQUIRED_TRIPWIRE_TESTS:
  - Migration applies cleanly: cargo test -p handshake_core -- migration
  - All Skill Bank CRUD: cargo test -p handshake_core -- skill_bank
  - Data trust score produces valid output: cargo test -p handshake_core -- data_trust_score
  - Candidate persistence round-trip: cargo test -p handshake_core -- distillation_candidate
  - Checkpoint lineage traversal: cargo test -p handshake_core -- checkpoint_lineage
  - Eval gate blocks unqualified promotion: cargo test -p handshake_core -- eval_gate
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test -p handshake_core -- skill_bank_schema_presence (verify all 6 tables created by migration)
  - cargo test -p handshake_core -- distillation_candidate_persistence (verify MTE candidate writes to Skill Bank)
  - cargo test -p handshake_core -- data_trust_score_range (verify score in 0.0..=1.0)
  - cargo test -p handshake_core -- checkpoint_lineage_query (verify parent chain traversal)
  - cargo test -p handshake_core -- eval_gate_promotion (verify benchmark threshold enforcement)
  - cargo test -p handshake_core -- export_control_capability (verify capability gate blocks unauthorized export)
  - cargo test -p handshake_core -- redaction_scrubbing (verify PII removal from training examples)
- CANONICAL_CONTRACT_EXAMPLES:
  - Fixture: SkillBankLogEntry with all 52 columns populated (golden test row)
  - Fixture: DistillationCandidate with teacher/student snapshot refs and trust score
  - Fixture: AdapterCheckpoint with parent lineage chain (3-deep)
  - Fixture: EvalRun with pass_at_k, compile_rate, collapse_indicator metrics
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/ (storage trait extensions) (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/migrations/ (new migration for Skill Bank tables) (migration/sql surface)
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLAR_DECOMPOSITION: PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Distillation lineage persistence | SUBFEATURES: teacher/student ids, tokenizer metadata, checkpoint parents, eval decisions | PRIMITIVES_FEATURES: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Core durable lineage from spec Section 9.1.2
  - PILLAR_DECOMPOSITION: PILLAR: MicroTask | CAPABILITY_SLICE: Persistent candidate capture | SUBFEATURES: escalation-driven distillation candidate write to Skill Bank | PRIMITIVES_FEATURES: PRIM-DistillationCandidate | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Extends existing ephemeral candidate to durable artifact
  - PILLAR_DECOMPOSITION: PILLAR: ACE | CAPABILITY_SLICE: Context provenance in training | SUBFEATURES: Context Pack hashes and PromptEnvelope hashes in distillation observability | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Read-only integration; hashes recorded but ACE runtime not modified
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Distillation tables Postgres portability | SUBFEATURES: portable SQL for skill_log_entry, distill_job, adapter_checkpoint, eval_run, distill_example | PRIMITIVES_FEATURES: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: All SQL must be SQLITE_NOW_POSTGRES_READY; avoid SQLite-specific syntax
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Skill Bank log entry persistence | JobModel: AI_JOB | Workflow: distillation_candidate_capture | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-MT-015 | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Extends MTE escalation path to write durable SkillBankLogEntry
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Distillation job orchestration | JobModel: WORKFLOW | Workflow: distillation_pipeline | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-SELECT, FR-EVT-DISTILL-TEACHER, FR-EVT-DISTILL-STUDENT, FR-EVT-DISTILL-SCORE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Full select-teacher-student-score pipeline
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Adapter checkpoint management | JobModel: WORKFLOW | Workflow: adapter_training_lifecycle | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-CHECKPOINT | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: LoRA/QLoRA/DoRA checkpoint creation with lineage
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Eval gating and promotion | JobModel: WORKFLOW | Workflow: eval_promotion_pipeline | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-EVAL, FR-EVT-DISTILL-PROMOTE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Benchmark-gated promotion with rollback safety
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: Data trust scoring | JobModel: MECHANICAL_TOOL | Workflow: NONE | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: NONE | FlightRecorder: FR-EVT-DISTILL-SCORE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Deterministic 0-1 scoring algorithm per spec 9.1.3.1
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: PII/secret redaction | JobModel: MECHANICAL_TOOL | Workflow: NONE | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: Log-time and pre-training scrubbing per spec 9.1.1.3
  - FORCE_MULTIPLIER_EXPANSION: Distillation + Context Pack provenance -> IN_THIS_WP (stub: NONE)
  - FORCE_MULTIPLIER_EXPANSION: MTE escalation + Skill Bank persistence -> IN_THIS_WP (stub: NONE)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/models/ (new skill_bank module)
  - src/backend/handshake_core/src/distillation/ (new module)
  - src/backend/handshake_core/src/workflows.rs (extend candidate persistence)
  - src/backend/handshake_core/src/flight_recorder/ (distillation stage events)
  - src/backend/handshake_core/src/storage/ (storage trait extensions)
  - src/backend/handshake_core/migrations/ (new migration for Skill Bank tables)
  - src/backend/handshake_core/src/capabilities.rs (distillation capability gates)
- DATA_CONTRACT_RULES:
  - Keep persisted and emitted structure SQL-backed and PostgreSQL-ready; do not introduce fresh SQLite-only semantics unless the packet or spec explicitly requires them.
  - Prefer explicit machine-readable fields, enums, ids, relations, and provenance over presentation-only strings, overloaded text blobs, or parser-only implied meaning.
  - Preserve stable ids, explicit relations, backlink-friendly fields, provenance anchors, and retrieval-friendly summaries so Loom and graph/search consumers can traverse the data without reparsing UI text.
- VALIDATOR_DATA_PROOF_HINTS:
  - Prove the touched data surfaces remain SQLite-now and PostgreSQL-ready or justify any backend-specific semantics explicitly.
  - Prove the emitted or persisted shapes stay LLM-first readable and parseable with stable field names and explicit structured values.
  - Prove Loom-facing ids, relations, provenance anchors, and retrieval fields remain explicit where the packet touches them.
## WP_COMMUNICATIONS (NON-AUTHORITATIVE; REQUIRED FOR NEW PACKETS)
- RULE: The task packet remains authoritative for scope, status, branch/worktree truth, acceptance, and verdict.
- PURPOSE: The per-WP communication folder holds freeform discussion, liveness state, and deterministic receipts for multi-session work.
- COMMUNICATION AUTHORITY RULE:
  - all roles use the packet-declared `WP_COMMUNICATION_DIR`
  - do not improvise role-local inboxes or worktree-local communication authority
  - if there is any dispute about where to write, `WP_COMMUNICATION_DIR` wins
- AUTHORITY SPLIT:
  - `WORKFLOW_AUTHORITY` owns workflow steering and hard gates
  - `TECHNICAL_ADVISOR` is the WP-scoped advisory validator
  - `TECHNICAL_AUTHORITY` owns final technical verdict authority
  - `MERGE_AUTHORITY` owns merge-to-main authority
  - `WP_VALIDATOR_OF_RECORD` and `INTEGRATION_VALIDATOR_OF_RECORD` name the active validator sessions once assigned
- THREAD.md:
  - append-only freeform conversation for Operator, Orchestrator, Coder, and Validator
  - may contain steering, questions, clarifications, and soft coordination
- RUNTIME_STATUS.json:
  - non-authoritative liveness and watch state
  - uses repo-governance runtime states: `submitted | working | input_required | completed | failed | canceled`
  - use for next expected actor, waiting state, validator trigger, heartbeat posture, validation readiness, and bounded review-loop counters
- RECEIPTS.jsonl:
  - append-only deterministic receipt ledger
  - one JSON object per line
  - use for assignment, status, heartbeat, steering, repair, validation, and handoff receipts
- DIRECT REVIEW CONTRACT:
  - For `WORKFLOW_LANE=ORCHESTRATOR_MANAGED` packets with `PACKET_FORMAT_VERSION >= 2026-03-21`, `COMMUNICATION_CONTRACT` MUST be `DIRECT_REVIEW_V1` and `COMMUNICATION_HEALTH_GATE` MUST be `HANDOFF_VERDICT_BLOCKING`.
  - Required structured receipts for the coder <-> WP validator lane:
    - `VALIDATOR_KICKOFF` (`WP_VALIDATOR -> CODER`)
    - `CODER_INTENT` (`CODER -> WP_VALIDATOR`, correlated to kickoff)
    - `CODER_HANDOFF` (`CODER -> WP_VALIDATOR`)
    - `VALIDATOR_REVIEW` (`WP_VALIDATOR -> CODER`, correlated to handoff)
  - For `PACKET_FORMAT_VERSION >= 2026-03-22`, `VERDICT` also requires one direct coder <-> integration-validator review pair recorded in receipts with a shared `correlation_id` / `ack_for` chain.
  - Review-tracked receipt appends auto-write notifications for the explicit target role and auto-project the next actor / validator wake state back into `RUNTIME_STATUS.json`.
  - `just wp-thread-append` remains valid for soft coordination only. It does not satisfy the required direct-review contract by itself.
  - `just wp-communication-health-check WP-{ID} KICKOFF|HANDOFF|VERDICT` is the machine gate for this contract.
- SESSION START + WAKE RULE:
  - only `WORKFLOW_AUTHORITY` may start repo-governed Coder, WP Validator, and Integration Validator sessions
  - ordinary launch path is `SESSION_HOST_PREFERENCE` through the governed ACP control lane
  - `SESSION_CONTROL_REQUESTS_FILE`, `SESSION_CONTROL_RESULTS_FILE`, and `SESSION_REGISTRY_FILE` are the deterministic launch/steer/watch state for ordinary governed sessions
  - `SESSION_COMPATIBILITY_SURFACE` and `SESSION_COMPATIBILITY_QUEUE_FILE` exist only for explicit compatibility or repair launches; they are not the default `AUTO` path
- WATCH RULE:
  - primary wake-up channel is `SESSION_WAKE_CHANNEL_PRIMARY`
  - fallback wake-up channel is `SESSION_WAKE_CHANNEL_FALLBACK`
  - do not rely on continuous polling when a watch event can be used
- CONFLICT RULE:
  - if THREAD.md, RUNTIME_STATUS.json, or RECEIPTS.jsonl conflicts with this packet, this packet wins
- LOOP LIMIT RULE:
  - do not exceed `MAX_CODER_REVISION_CYCLES`, `MAX_VALIDATOR_REVIEW_CYCLES`, or `MAX_RELAY_ESCALATION_CYCLES` without explicit Operator steering recorded in the packet or thread
- HEARTBEAT RULE:
  - do not poll continuously
  - update `RUNTIME_STATUS.json` and append a receipt on session start, major phase change, blocker/unblock, handoff, completion, and every `HEARTBEAT_INTERVAL_MINUTES` only while active

## SUB_AGENT_DELEGATION (OPTIONAL; OPERATOR-GATED)
- SUB_AGENT_DELEGATION: DISALLOWED
- OPERATOR_APPROVAL_EVIDENCE: N/A
- SUB_AGENT_REASONING_ASSUMPTION: LOW (HARD)
- NOTE: `AGENTIC_MODE: YES` means sub-agent use is explicitly authorized for this WP; `AGENTIC_MODE: NO` means all roles remain single-session.
- RULES (if SUB_AGENT_DELEGATION=ALLOWED):
  - Sub-agents produce draft code only; Primary Coder verifies against SPEC_CURRENT + task packet acceptance criteria before applying.
  - Sub-agents MUST NOT edit any governance surface (`.GOV/**`, including task packets/refinements and `## VALIDATION_REPORTS`).
  - Only Primary Coder runs gates, records EVIDENCE/EVIDENCE_MAPPING/VALIDATION manifest, commits, and hands off.
  - See: `/.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md` Section 6.
- NOTE: Set `SUB_AGENT_DELEGATION: ALLOWED` only with explicit Operator approval; when ALLOWED, replace `OPERATOR_APPROVAL_EVIDENCE` with the exact approval line from chat.

## TECHNICAL_REFINEMENT (MASTER SPEC)
- REFINEMENT_FILE: .GOV/refinements/WP-1-Distillation-v2.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md Section 9 Continuous Local Skill Distillation (Skill Bank and Pipeline)
- CONTEXT_START_LINE: 53539
- CONTEXT_END_LINE: 53600
- CONTEXT_TOKEN: data_trust_score
- EXCERPT_ASCII_ESCAPED:
  ```text
# 9. Continuous Local Skill Distillation (Skill Bank & Pipeline)

  **Why**
  - Capture the complete Skill Bank and distillation pipeline (teacher/student) inside the Master Spec without losing any technical detail.
  - Ensure alignment with AI Job Model, Workflow Engine, Flight Recorder, and capability/privacy controls.

  **How it integrates**
  - Data model fields (messages, snapshots, engines, context refs, telemetry, quality, trust, checkpoints, examples) map to Section 3 storage/indexing and provenance rules; no token logs are stored, tokenization is per-engine at train time.
  - Distillation jobs (sample/select -> teacher -> student -> score -> checkpoint -> eval/promotion) must run through the Workflow Engine with capability gates; Flight Recorder logs models, tokenizers, params, files, tools, metrics, reward features, lineage, and data_signature/job_ids_json.

  **Quality-Weighted Training Data Selection:**
  Training data selection MUST weight samples by signals:
  1. User signal: thumbs up/down, edit ratio (from QualityMeta)
  2. Auto-eval: tests passed, compile success, reasoning score
  3. Retrieval signal: Was retrieved content used? Did it help?
  4. Data trust score: Combined 0-1 weight for training (data_trust_score field)
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.6.6.8.13 Learning Integration
- CONTEXT_START_LINE: 14210
- CONTEXT_END_LINE: 14258
- CONTEXT_TOKEN: enable_distillation
- EXCERPT_ASCII_ESCAPED:
  ```text
##### 2.6.6.8.13 Learning Integration

  ###### 2.6.6.8.13.1 Skill Bank Integration

  When escalation occurs and policy.enable_distillation = true, a distillation candidate MUST be generated:

  interface DistillationCandidate {
    skill_log_entry_id: UUID;
    mt_id: string;
    wp_id: string;
    student_attempt: { model_id, lora_id?, prompt_snapshot_ref, output_snapshot_ref, outcome, iterations };
    teacher_success: { model_id, lora_id?, prompt_snapshot_ref, output_snapshot_ref, outcome, iterations };
    task_type_tags: string[];
    contributing_factors: string[];
    data_trust_score: number;
    distillation_eligible: boolean;
  }

  [ADD v02.157] Runtime implementations MAY stage candidates in a bounded pending queue before promotion into canonical Skill Bank artifacts, but that queue MUST remain recorder-visible and retain teacher/student prompt refs, tokenizer metadata, task tags, and trust signals.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 5.3.6 Distillation Observability Requirements
- CONTEXT_START_LINE: 23078
- CONTEXT_END_LINE: 23084
- CONTEXT_TOKEN: Flight Recorder events
- EXCERPT_ASCII_ESCAPED:
  ```text
### 5.3.6 Distillation Observability Requirements
  - Distillation jobs MUST emit Flight Recorder events for each stage (select, teacher run, student run, score, checkpoint, eval, promote/rollback) with trace IDs.
  - Required fields: model/tokenizer ids, inference params, context refs (files/spec sections/tools), metrics (pass@k, compile/test rates, collapse indicators), reward features, lineage (parent_checkpoint_id), data_signature, job_ids_json, promotion decisions.
  - PII/secret handling: apply log-time redaction and pre-training scrubbing; enforce capability-based export controls for Skill Bank artifacts.
  - Dashboards/traces should surface promotion gates vs teacher/previous checkpoints and collapse indicators for regression detection.
  - [ADD v02.157] Distillation observability MUST also record Context Pack hashes/freshness decisions, PromptEnvelope hashes, and pending-candidate queue transitions whenever Context Packs or Spec Router artifacts shape teacher/student inputs.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.180.md 2.5.12 Context Packs AI Job Profile
- CONTEXT_START_LINE: 9305
- CONTEXT_END_LINE: 9354
- CONTEXT_TOKEN: context_pack_builder
- EXCERPT_ASCII_ESCAPED:
  ```text
### 2.5.12 Context Packs AI Job Profile

  **Implements:** AI Job Model (Section 2.6.6)
  **Profile ID:** context_pack_builder_v0.1
  **Status:** Draft (internal)

  **Why**
  Retrieval-backed answers and transformations improve correctness and token efficiency when the system prefers mechanical, reusable compactions over raw snippet dumps. A ContextPack is a derived, provenance-bound artifact (facts/constraints/open loops + anchors) that can be retrieved cheaply and assembled deterministically into PromptEnvelopes.

  [ADD v02.156] ContextPack payloads, anchors, coverage, freshness guards, and canonical artifact serialization are portable retrieval contracts.
  [ADD v02.157] ContextPack freshness policy/decision, build/reuse hashes, and recorder-visible build/select/refresh outcomes are canonical backend contracts for later distillation, replay, and model onboarding.
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: Section 9.1.1 Data model (SkillBankLogEntry, QualityMeta, TelemetryMeta, PrivacyMeta) | WHY_IN_SCOPE: Core data structures for durable distillation lineage | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/models/skill_bank.rs | EXPECTED_TESTS: cargo test -p handshake_core -- skill_bank | RISK_IF_MISSED: No persistent training data; pipeline cannot operate
  - CLAUSE: Section 9.1.2 SQL schema (skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run) | WHY_IN_SCOPE: Durable storage for Skill Bank artifacts | EXPECTED_CODE_SURFACES: src/backend/handshake_core/migrations/0017_skill_bank_distillation.sql | EXPECTED_TESTS: cargo test -p handshake_core -- migration | RISK_IF_MISSED: Data model exists only in memory; no queryable lineage
  - CLAUSE: Section 9.1.3.1 compute_data_trust_score | WHY_IN_SCOPE: Quality-weighted training data selection | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/scoring.rs | EXPECTED_TESTS: cargo test -p handshake_core -- data_trust_score | RISK_IF_MISSED: Training data selection is unweighted; collapse risk increases
  - CLAUSE: Section 9.1.3.2 build_distill_dataset (new + replay batches) | WHY_IN_SCOPE: Dataset assembly for adapter training | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/dataset.rs | EXPECTED_TESTS: cargo test -p handshake_core -- distill_dataset | RISK_IF_MISSED: No dataset assembly; training cannot start
  - CLAUSE: Section 9.1.4 Evaluation and promotion gates | WHY_IN_SCOPE: Benchmark-gated promotion prevents silent regression | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/eval.rs | EXPECTED_TESTS: cargo test -p handshake_core -- eval_promotion | RISK_IF_MISSED: Adapters promoted without quality validation; silent regressions
  - CLAUSE: Section 5.3.6 Distillation observability (FR events per stage) | WHY_IN_SCOPE: Pipeline visibility and debugging | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/flight_recorder/mod.rs | EXPECTED_TESTS: cargo test -p handshake_core -- flight_recorder_distill | RISK_IF_MISSED: Silent pipeline failures; no audit trail
  - CLAUSE: Section 2.6.6.8.13 Learning Integration (DistillationCandidate persistence) | WHY_IN_SCOPE: MTE escalation produces training data | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/workflows.rs | EXPECTED_TESTS: cargo test -p handshake_core -- distillation_candidate | RISK_IF_MISSED: Candidates remain ephemeral; training data lost on process exit
  - CLAUSE: Section 9 PII/secret redaction (redact_entry) | WHY_IN_SCOPE: Privacy safety in training data | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/distillation/redaction.rs | EXPECTED_TESTS: cargo test -p handshake_core -- redaction | RISK_IF_MISSED: PII leaks into training data; compliance and safety failure
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: SkillBankLogEntry JSON/SQL | PRODUCER: MTE escalation handler (workflows.rs) | CONSUMER: distillation dataset builder, Flight Recorder | SERIALIZER_TRANSPORT: serde_json / SQLite row | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- skill_bank_serialization | DRIFT_RISK: Field additions in spec vs struct mismatch
  - CONTRACT: DistillationCandidate artifact | PRODUCER: MTE escalation (workflows.rs) | CONSUMER: Skill Bank persistence, FR event logger | SERIALIZER_TRANSPORT: serde_json artifact | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- distillation_candidate_shape | DRIFT_RISK: Existing ephemeral struct vs spec DistillationCandidate interface divergence
  - CONTRACT: AdapterCheckpoint lineage | PRODUCER: adapter training pipeline | CONSUMER: eval gating, promotion logic, export controls | SERIALIZER_TRANSPORT: SQLite row with parent_checkpoint_id FK | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- checkpoint_lineage | DRIFT_RISK: Orphaned checkpoints if parent FK not enforced
  - CONTRACT: FR-EVT-DISTILL-* event payloads | PRODUCER: distillation pipeline stages | CONSUMER: Flight Recorder storage, dashboards | SERIALIZER_TRANSPORT: FlightRecorderEventType enum + JSON payload | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- flight_recorder_distill_events | DRIFT_RISK: Missing required fields (model/tokenizer ids, context refs, lineage)
  - CONTRACT: DataTrustScore computation | PRODUCER: compute_data_trust_score | CONSUMER: dataset builder, training data selection | SERIALIZER_TRANSPORT: f64 in 0.0..=1.0 range | VALIDATOR_READER: validator-spec-regression | TRIPWIRE_TESTS: cargo test -- data_trust_score_range | DRIFT_RISK: Score outside valid range or missing input signals
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - 1. SQL migration: create skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run tables and replay_candidates view
  - 2. Data model structs: SkillBankLogEntry, QualityMeta, TelemetryMeta, PrivacyMeta, DistillJob, DistillExample, AdapterCheckpoint, EvalRun
  - 3. Storage trait extensions: CRUD operations for all Skill Bank entities
  - 4. PII/secret redaction: redact_entry function with log-time and pre-training scrubbing modes
  - 5. Data trust scoring: compute_data_trust_score with multi-signal quality aggregation
  - 6. Distillation candidate persistence: extend MTE escalation path to write SkillBankLogEntry
  - 7. Dataset assembly: build_distill_dataset with new and replay batch support
  - 8. Adapter training lifecycle: checkpoint creation with parent lineage
  - 9. Eval gating and promotion: benchmark-gated promotion with rollback safety
  - 10. Export controls: capability gates for checkpoint and eval artifact export
  - 11. Flight Recorder events: FR-EVT-DISTILL-* for all pipeline stages
  - 12. Context Pack and PromptEnvelope hash integration in observability
- HOT_FILES:
  - src/backend/handshake_core/src/workflows.rs (existing DistillationInfo, PendingDistillationCandidate)
  - src/backend/handshake_core/src/flight_recorder/mod.rs (existing MicroTaskDistillationCandidate event)
  - src/backend/handshake_core/src/storage/mod.rs (existing JobKind::DistillationEval)
  - src/backend/handshake_core/src/capabilities.rs (needs distillation gates)
  - src/backend/handshake_core/migrations/ (latest migration is 0016)
- TRIPWIRE_TESTS:
  - Migration applies cleanly: cargo test -p handshake_core -- migration
  - All Skill Bank CRUD: cargo test -p handshake_core -- skill_bank
  - Data trust score produces valid output: cargo test -p handshake_core -- data_trust_score
  - Candidate persistence round-trip: cargo test -p handshake_core -- distillation_candidate
  - Checkpoint lineage traversal: cargo test -p handshake_core -- checkpoint_lineage
  - Eval gate blocks unqualified promotion: cargo test -p handshake_core -- eval_gate
- CARRY_FORWARD_WARNINGS:
  - PendingDistillationCandidate in workflows.rs is ephemeral and JSON-serialized to artifact; migration to SQL-backed SkillBankLogEntry must preserve compatibility during transition
  - JobKind::DistillationEval exists but is dead code; handler implementation must match workflow engine dispatch patterns used by other job kinds
  - SQL must be SQLITE_NOW_POSTGRES_READY: avoid SQLite-specific syntax (e.g., AUTOINCREMENT vs SERIAL)
  - Cross-tokenizer safety: do not assume teacher and student share the same tokenizer; always validate tokenizer_id match or use cross-tokenizer-safe replay
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - Section 9.1.2 SQL schema: all 6 tables and 1 view match spec column definitions
  - Section 9.1.3.1 compute_data_trust_score: algorithm matches spec formula
  - Section 9.1.4 eval gating: promotion requires benchmark thresholds
  - Section 5.3.6 observability: all FR-EVT-DISTILL-* events include required fields
  - Section 2.6.6.8.13 candidate persistence: MTE escalation writes durable SkillBankLogEntry
  - PII/secret redaction: redact_entry covers all spec-defined sensitive fields
  - Export controls: capability gate enforcement for local-only artifacts
- FILES_TO_READ:
  - src/backend/handshake_core/src/models/skill_bank.rs
  - src/backend/handshake_core/src/distillation/
  - src/backend/handshake_core/migrations/0017_skill_bank_distillation.sql
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/capabilities.rs
- COMMANDS_TO_RUN:
  - cargo test -p handshake_core
  - just validator-spec-regression
  - just validator-scan WP-1-Distillation-v2
- POST_MERGE_SPOTCHECKS:
  - Verify skill_log_entry table exists after migration
  - Verify FR-EVT-DISTILL-PROMOTE event includes parent_checkpoint_id and promotion decision
  - Verify export capability gate rejects unauthorized checkpoint download
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - Actual LoRA/QLoRA/DoRA training convergence quality: spec defines adapter-only posture and hyperparameter tracking but real training quality depends on runtime data volume and distribution
  - Cross-tokenizer replay fidelity: spec mandates cross-tokenizer-safe distillation but actual fidelity under diverse tokenizer pairs requires field validation
  - Data trust score calibration: the compute_data_trust_score formula can be implemented per spec but optimal weight calibration requires real-world training data evaluation
  - Collapse indicator sensitivity: spec defines collapse monitoring but threshold tuning requires observed training runs
  - Migration file number (0017 assumed): actual next migration number depends on main branch state at coding time
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - NONE
- GITHUB_PROJECT_DECISIONS:
  - NONE
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: NO
- MATRIX_RESEARCH_VERDICT: NOT_APPLICABLE
- SOURCE_SCAN_DECISIONS:
  - NONE
- MATRIX_GROWTH_CANDIDATES:
  - NONE
- ENGINEERING_TRICKS_CARRIED_OVER:
  - NONE
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-DistillationCandidate
  - PRIM-PendingDistillationCandidate
  - PRIM-FlightEvent
  - PRIM-JobKind
  - PRIM-SkillBankLogEntry
  - PRIM-AdapterCheckpoint
  - PRIM-DataQualityMetrics
  - PRIM-RedactionMode
- PRIMITIVES_EXPOSED:
  - PRIM-SkillBankLogEntry
  - PRIM-AdapterCheckpoint
  - PRIM-DistillationCandidate
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.analyst
  - engine.dba
  - engine.sovereign
  - engine.context
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Flight Recorder
  - MicroTask
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - Skill distillation / LoRA
  - ACE
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: NEEDS_STUBS
- FORCE_MULTIPLIER_RESOLUTIONS:
  - Distillation + Flight Recorder stage events -> IN_THIS_WP (stub: NONE)
  - Distillation + Context Pack provenance -> IN_THIS_WP (stub: NONE)
  - MTE escalation + Skill Bank persistence -> IN_THIS_WP (stub: NONE)
  - Adapter checkpoint + Artifact System -> IN_THIS_WP (stub: NONE)
  - LoRA inference wiring + trained adapters -> NEW_STUB (stub: WP-1-MTE-LoRA-Wiring-v1)
  - Spawn conversation histories + training data -> NEW_STUB (stub: WP-1-Session-Spawn-Conversation-Distillation-v1)
  - Distillation tables + Postgres portability -> IN_THIS_WP (stub: NONE)
  - Export controls + capability enforcement -> IN_THIS_WP (stub: NONE)
  - Data trust scoring + eval metrics -> IN_THIS_WP (stub: NONE)
  - PII redaction + privacy enforcement -> IN_THIS_WP (stub: NONE)
  - Checkpoint lineage + rollback safety -> IN_THIS_WP (stub: NONE)
  - Distillation observability + DBA storage -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: WP-1-MTE-LoRA-Wiring-v1, WP-1-Session-Spawn-Conversation-Distillation-v1
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: NEEDS_STUBS
- DECOMPOSITION_ROWS:
  - PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Distillation lineage persistence | SUBFEATURES: teacher/student ids, tokenizer metadata, checkpoint parents, eval decisions | PRIMITIVES_FEATURES: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint | MECHANICAL: engine.dba | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Core durable lineage from spec Section 9.1.2
  - PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Benchmark-gated adapter lifecycle | SUBFEATURES: LoRA/QLoRA/DoRA training config, eval suite, promotion/rollback | PRIMITIVES_FEATURES: PRIM-DataQualityMetrics, PRIM-AdapterCheckpoint | MECHANICAL: engine.analyst | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Adapter training outcomes compared against teacher and previous checkpoint
  - PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: LoRA inference wiring | SUBFEATURES: lora_id in provider request envelope, adapter selection by task tags | PRIMITIVES_FEATURES: NONE | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: NEW_STUB | STUB: WP-1-MTE-LoRA-Wiring-v1 | NOTES: Out of scope for this WP; requires provider client changes
  - PILLAR: Skill distillation / LoRA | CAPABILITY_SLICE: Export and replay controls | SUBFEATURES: capability-gated export, deterministic replay metadata | PRIMITIVES_FEATURES: PRIM-RedactionMode | MECHANICAL: engine.sovereign | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Prevents off-device leakage of local-only checkpoints
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: Distillation stage observability | SUBFEATURES: FR-EVT-DISTILL-* events, Context Pack hash tracking, PromptEnvelope hash tracking | PRIMITIVES_FEATURES: PRIM-FlightEvent | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Per spec Section 5.3.6 and v02.157 additions
  - PILLAR: MicroTask | CAPABILITY_SLICE: Persistent candidate capture | SUBFEATURES: escalation-driven distillation candidate write to Skill Bank | PRIMITIVES_FEATURES: PRIM-DistillationCandidate | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Extends existing ephemeral candidate to durable artifact
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: Distillation job execution | SUBFEATURES: DistillationEval job handler, workflow engine integration, capability gates | PRIMITIVES_FEATURES: PRIM-JobKind, PRIM-DistillationCandidate | MECHANICAL: NONE | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: JobKind::DistillationEval gets full handler
  - PILLAR: ACE | CAPABILITY_SLICE: Context provenance in training | SUBFEATURES: Context Pack hashes and PromptEnvelope hashes in distillation observability | PRIMITIVES_FEATURES: NONE | MECHANICAL: engine.context | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: Read-only integration; hashes recorded but ACE runtime not modified
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: Distillation tables Postgres portability | SUBFEATURES: portable SQL for skill_log_entry, distill_job, adapter_checkpoint, eval_run, distill_example | PRIMITIVES_FEATURES: PRIM-SkillBankLogEntry, PRIM-DistillationCandidate, PRIM-AdapterCheckpoint | MECHANICAL: engine.dba | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: All SQL must be SQLITE_NOW_POSTGRES_READY; avoid SQLite-specific syntax
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Skill Bank log entry persistence | JobModel: AI_JOB | Workflow: distillation_candidate_capture | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-MT-015 | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Extends MTE escalation path to write durable SkillBankLogEntry
  - Capability: Distillation job orchestration | JobModel: WORKFLOW | Workflow: distillation_pipeline | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-SELECT, FR-EVT-DISTILL-TEACHER, FR-EVT-DISTILL-STUDENT, FR-EVT-DISTILL-SCORE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Full select-teacher-student-score pipeline
  - Capability: Adapter checkpoint management | JobModel: WORKFLOW | Workflow: adapter_training_lifecycle | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-CHECKPOINT | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: LoRA/QLoRA/DoRA checkpoint creation with lineage
  - Capability: Eval gating and promotion | JobModel: WORKFLOW | Workflow: eval_promotion_pipeline | ToolSurface: NONE | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: FR-EVT-DISTILL-EVAL, FR-EVT-DISTILL-PROMOTE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Benchmark-gated promotion with rollback safety
  - Capability: Data trust scoring | JobModel: MECHANICAL_TOOL | Workflow: NONE | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: NONE | FlightRecorder: FR-EVT-DISTILL-SCORE | Locus: NONE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: Deterministic 0-1 scoring algorithm per spec 9.1.3.1
  - Capability: PII/secret redaction | JobModel: MECHANICAL_TOOL | Workflow: NONE | ToolSurface: NONE | ModelExposure: LOCAL | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: Log-time and pre-training scrubbing per spec 9.1.1.3
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-MTE-LoRA-Wiring-v1 -> KEEP_SEPARATE
  - WP-1-Session-Spawn-Conversation-Distillation-v1 -> KEEP_SEPARATE
  - WP-1-MTE-Summaries-v1 -> KEEP_SEPARATE
  - WP-1-MTE-Resource-Caps-v1 -> KEEP_SEPARATE
  - WP-1-Distillation -> EXPAND_IN_THIS_WP
  - WP-1-Micro-Task-Executor-v1 -> KEEP_SEPARATE
  - WP-1-Artifact-System-Foundations-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> IMPLEMENTED (WP-1-Distillation)
  - ../handshake_main/src/backend/handshake_core/src/flight_recorder/mod.rs -> IMPLEMENTED (WP-1-Micro-Task-Executor-v1)
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs -> PARTIAL (WP-1-Distillation)
  - ../handshake_main/src/backend/handshake_core/src/capabilities.rs -> NOT_PRESENT (NONE)
  - ../handshake_main/src/backend/handshake_core/migrations/ -> NOT_PRESENT (NONE)
## GUI_IMPLEMENTATION_ADVICE (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- GUI_ADVICE_REQUIRED: NO
- GUI_IMPLEMENTATION_ADVICE_VERDICT: NOT_APPLICABLE
- GUI_REFERENCE_DECISIONS:
  - NONE
- HANDSHAKE_GUI_ADVICE:
  - NONE
- HIDDEN_GUI_REQUIREMENTS:
  - NONE
- GUI_ENGINEERING_TRICKS_TO_CARRY:
  - NONE
## SCOPE
- What: Implement the full Skill Bank and distillation backend: durable data model (SkillBankLogEntry, DistillJob, AdapterCheckpoint, EvalRun), SQL schema, data trust scoring, benchmark-gated adapter lifecycle, export controls, PII/secret redaction, and full-pipeline Flight Recorder observability.
- Why: The spec hardcodes LoRA/QLoRA/DoRA posture, PromptEnvelope/Context Pack reuse, and distillation evidence requirements (v02.115 through v02.157), but the implementation has only ephemeral in-memory candidates and a stub job kind. The learning substrate cannot function without persistent lineage, eval gating, and promotion safety.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/models/ (new skill_bank module)
  - src/backend/handshake_core/src/distillation/ (new module)
  - src/backend/handshake_core/src/workflows.rs (extend candidate persistence)
  - src/backend/handshake_core/src/flight_recorder/ (distillation stage events)
  - src/backend/handshake_core/src/storage/ (storage trait extensions)
  - src/backend/handshake_core/migrations/ (new migration for Skill Bank tables)
  - src/backend/handshake_core/src/capabilities.rs (distillation capability gates)
- OUT_OF_SCOPE:
  - End-user UI polish for model-training consoles
  - Full-model fine-tuning beyond adapter-only posture
  - LoRA inference wiring (WP-1-MTE-LoRA-Wiring-v1)
  - Spawn conversation distillation pipeline (WP-1-Session-Spawn-Conversation-Distillation-v1)
- TOUCHED_FILE_BUDGET: 12
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC, GOVERNANCE | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- (For explicit post-signature continuation after `TOKEN_BUDGET_EXCEEDED` / `POLICY_CONFLICT`, use `COVERS: GOVERNANCE` and name the policy conflict directly in `SCOPE` or `JUSTIFICATION`.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test -p handshake_core
  just validator-spec-regression
  just validator-scan WP-1-Distillation-v2
```

### DONE_MEANS
- skill_log_entry, distill_job, distill_example, adapter_checkpoint, eval_run tables exist and pass migration
- SkillBankLogEntry, DistillJob, AdapterCheckpoint, EvalRun structs implement storage trait with SQLite-now-Postgres-ready SQL
- compute_data_trust_score algorithm produces valid 0-1 scores from multi-signal quality aggregation
- Distillation candidate persistence pipeline connects MTE escalation to Skill Bank durable storage
- Checkpoint lineage is queryable (parent chains via parent_checkpoint_id)
- Eval gating enforces benchmark thresholds (pass@k, compile/test rates, collapse indicators) before promotion
- Rollback-safe promotion: failed promotion reverts to previous checkpoint without data loss
- Export controls prevent off-device leakage of local-only checkpoints via capability gates
- Flight Recorder events emitted for each distillation stage (select, teacher, student, score, checkpoint, eval, promote/rollback)
- Context Pack hashes and PromptEnvelope hashes recorded in distillation observability per v02.157
- PII/secret redaction applied at log time (redact_entry) and pre-training scrubbing
- All tests pass: cargo test -p handshake_core, just validator-spec-regression

- PRIMITIVES_EXPOSED:
  - PRIM-SkillBankLogEntry
  - PRIM-AdapterCheckpoint
  - PRIM-DistillationCandidate
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-13T22:59:04.968Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.180]
- SPEC_ANCHOR_PRIMARY: Section 9 Continuous Local Skill Distillation (Skill Bank and Pipeline)
- Codex: .GOV/codex/Handshake_Codex_v1.4.md
- Task Board: .GOV/roles_shared/records/TASK_BOARD.md
- WP Traceability: .GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md
## LINEAGE_AUDIT (ALL VERSIONS) [CX-580E]
- Required when `WP_ID` includes `-v{N}`.
- List every prior packet for `BASE_WP_ID` (filenames/paths) and state what is preserved vs changed.
- Hard rule: Do not drop prior requirements; carry them forward explicitly.
- If this is not a revision packet, write: `N/A`.

## BOOTSTRAP
- FILES_TO_OPEN:
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/flight_recorder/mod.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
  - .GOV/spec/Handshake_Master_Spec_v02.180.md
- SEARCH_TERMS:
  - SkillBankLogEntry, distill_job, adapter_checkpoint, eval_run
  - DistillationCandidate, PendingDistillationCandidate, DistillationInfo
  - data_trust_score, compute_data_trust_score, enable_distillation
  - MicroTaskDistillationCandidate, DistillationEval
  - LoRA, QLoRA, DoRA, redact_entry
- RUN_COMMANDS:
  ```bash
cargo test -p handshake_core
  just validator-spec-regression
  just validator-scan WP-1-Distillation-v2
  just gov-check
  ```
- RISK_MAP:
  - "Model collapse from self-distilled data dominance" -> "data_trust_score weighting with collapse indicators monitored in eval gating"
  - "Cross-tokenizer corruption in teacher/student comparisons" -> "Tokenizer metadata required per-candidate; compatibility validated at dataset assembly"
  - "Silent regression from adapter promotion without strict eval" -> "Benchmark-gated promotion with rollback-safe checkpoint lineage"
  - "PII leakage into training data" -> "Log-time redaction plus pre-training scrubbing; capability-gated export controls"
  - "Checkpoint storage exhaustion" -> "Artifact system GC and retention policies apply to distillation artifacts"
## SKELETON
- Proposed interfaces/types/contracts:
- Open questions:
- Notes:

## UI_UX_SPEC (REQUIRED IF UI_UX_APPLICABLE=YES)
- UI_UX_APPLICABLE=NO in the signed refinement. No user-facing surface is in scope for this packet.
## END_TO_END_CLOSURE_PLAN [CX-E2E-001]
- END_TO_END_CLOSURE_PLAN_APPLICABLE: NO
- TRUST_BOUNDARY: N/A
- SERVER_SOURCES_OF_TRUTH:
  - NONE
- REQUIRED_PROVENANCE_FIELDS:
  - NONE
- VERIFICATION_PLAN:
  - Record end-to-end trust/provenance requirements only if this WP introduces a cross-boundary apply path.
- ERROR_TAXONOMY_PLAN:
  - N/A for initial coder handoff.
- UI_GUARDRAILS:
  - N/A for initial coder handoff.
- VALIDATOR_ASSERTIONS:
  - Validate the packet-scoped spec anchors, in-scope files, and deterministic evidence recorded during implementation.
## IMPLEMENTATION
- (Coder fills after the docs-only skeleton checkpoint commit exists.)

## HYGIENE
- (Coder fills after implementation; list activities and commands run. Outcomes may be summarized here, but detailed logs should go in ## EVIDENCE.)

## VALIDATION
- (Mechanical manifest for audit. Fill real values to enable `just phase-check HANDOFF <WP_ID> CODER`. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
- If the WP changes multiple non-`.GOV/` files, repeat the manifest block once per changed file (multiple `**Target File**` entries are supported).
- SHA1 hint: stage your changes and run `just cor701-sha <changed file>` to get deterministic `Pre-SHA1` / `Post-SHA1` values.
- **Target File**: `N/A (fill after implementation)`
- **Start**: N/A
- **End**: N/A
- **Line Delta**: N/A
- **Pre-SHA1**: `N/A`
- **Post-SHA1**: `N/A`
- **Gates Passed**:
  - [ ] anchors_present
  - [ ] window_matches_plan
  - [ ] rails_untouched_outside_window
  - [ ] filename_canonical_and_openable
  - [ ] pre_sha1_captured
  - [ ] post_sha1_captured
  - [ ] line_delta_equals_expected
  - [ ] all_links_resolvable
  - [ ] manifest_written_and_path_returned
  - [ ] current_file_matches_preimage
- **Lint Results**:
- **Artifacts**:
- **Timestamp**:
- **Operator**:
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_vXX.XX.md
- **Notes**:
## STATUS_HANDOFF
- (Use this to list touched files and summarize work done without claiming a validation verdict. Mirror freeform discussion and liveness into the WP communication folder when present.)
- Rule for `CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2`: do not write a generic "ready for validation" note. Include both the standard handoff core and the rubric-proof fields below with the strongest self-critique you can defend.
- Current WP_STATUS:
- What changed in this update:
- Requirements / clauses self-audited:
- Checks actually run:
- Known gaps / weak spots:
- Heuristic risks / maintainability concerns:
- Validator focus request:
- Rubric contract understanding proof:
- Rubric scope discipline proof:
- Rubric baseline comparison:
- Rubric end-to-end proof:
- Rubric architecture fit self-review:
- Rubric heuristic quality self-review:
- Rubric anti-gaming / counterfactual check:
<!-- For PACKET_FORMAT_VERSION >= 2026-04-01 and CODER_HANDOFF_RIGOR_PROFILE=RUBRIC_SELF_AUDIT_V2, also include: -->
- Rubric anti-vibe / substance self-check:
- Signed-scope debt ledger:
- Data contract self-check:
- Next step / handoff hint:

## MERGE_PROGRESSION_TRUTH
- For `PACKET_FORMAT_VERSION >= 2026-03-25`, PASS closure is two-step and must stay explicit:
  - validator PASS append before merge-to-main:
    - set `**Status:** Done`
    - set `MAIN_CONTAINMENT_STATUS: MERGE_PENDING`
    - keep `MERGED_MAIN_COMMIT: NONE`
    - keep `MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A`
    - project Task Board `## Done` token as `[MERGE_PENDING]`
  - after merge-to-main authority finishes and local `main` actually contains the approved closure commit:
    - set `**Status:** Validated (PASS)`
    - set `MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN`
    - record `MERGED_MAIN_COMMIT: <main-contained sha>`
    - record `MAIN_CONTAINMENT_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - project Task Board `## Done` token as `[VALIDATED]`
- `Validated (FAIL)`, `Validated (OUTDATED_ONLY)`, and `Validated (ABANDONED)` must use `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`.
- Historical packet variants older than `PACKET_FORMAT_VERSION 2026-03-25` keep their legacy closure semantics and must not be rewritten in place only to satisfy this newer merge-progression model.

## SIGNED_SCOPE_COMPATIBILITY_TRUTH
- For `PACKET_FORMAT_VERSION >= 2026-03-26`, final-lane PASS clearance must record current-`main` compatibility explicitly before `just validator-gate-commit`:
  - if the signed packet still covers the required integration work against current local `main`, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: NOT_REQUIRED`
    - `PACKET_WIDENING_EVIDENCE: N/A`
  - if current local `main` introduces adjacent shared-surface work outside the signed packet, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: ADJACENT_SCOPE_REQUIRED`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: FOLLOW_ON_WP_REQUIRED | SUPERSEDING_PACKET_REQUIRED`
    - `PACKET_WIDENING_EVIDENCE: <new WP id / packet id / audit id / short governed rationale>`
  - if compatibility cannot be checked honestly, set:
    - `CURRENT_MAIN_COMPATIBILITY_STATUS: BLOCKED`
    - `CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: <current local main HEAD sha>`
    - `CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: <RFC3339 UTC>`
    - `PACKET_WIDENING_DECISION: NONE`
    - `PACKET_WIDENING_EVIDENCE: N/A`
- `CURRENT_MAIN_COMPATIBILITY_STATUS: NOT_RUN` is legal only before final-lane compatibility review starts.

## EVIDENCE_MAPPING
- (Coder appends proof that DONE_MEANS + SPEC_ANCHOR requirements exist in code/tests. No verdicts.)
- Format (repeat as needed):
  - REQUIREMENT: "<quote DONE_MEANS bullet or SPEC_ANCHOR requirement>"
  - EVIDENCE: `N/A (fill during implementation)`
## EVIDENCE
- (Coder appends logs, test outputs, and proof of work here. No verdicts.)
- Recommended evidence format (prevents chat truncation; enables audit):
  - COMMAND: `<paste>`
  - EXIT_CODE: `<int>`
  - LOG_PATH: `.handshake/logs/WP-1-Distillation-v2/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)
- For `PACKET_FORMAT_VERSION >= 2026-03-25`, merge progression truth is part of closure law:
  - `**Status:** Done` means PASS is recorded but main containment is still pending and requires:
    - `MAIN_CONTAINMENT_STATUS: MERGE_PENDING`
    - `MERGED_MAIN_COMMIT: NONE`
    - `MAIN_CONTAINMENT_VERIFIED_AT_UTC: N/A`
  - `**Status:** Validated (PASS)` is reserved for closures already contained in local `main` and requires:
    - `MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN`
    - `MERGED_MAIN_COMMIT: <main-contained sha>`
    - `MAIN_CONTAINMENT_VERIFIED_AT_UTC: <RFC3339 UTC>`
  - `Validated (FAIL)`, `Validated (OUTDATED_ONLY)`, and `Validated (ABANDONED)` require `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY | ABANDONED`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- For `PACKET_FORMAT_VERSION >= 2026-03-22`, every appended governed validation report MUST also include these completion-layer fields:
  - `WORKFLOW_VALIDITY: VALID | INVALID | PARTIAL | BLOCKED | NOT_RUN`
  - `SCOPE_VALIDITY: IN_SCOPE | OUT_OF_SCOPE | PARTIAL | BLOCKED | NOT_RUN`
  - `PROOF_COMPLETENESS: PROVEN | NOT_PROVEN | PARTIAL | BLOCKED | NOT_RUN`
  - `INTEGRATION_READINESS: READY | NOT_READY | PARTIAL | BLOCKED | NOT_RUN`
  - `DOMAIN_GOAL_COMPLETION: COMPLETE | INCOMPLETE | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, every appended governed validation report MUST also include:
  - `MECHANICAL_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_RETENTION_TRACK_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST also include:
  - `CLAUSES_REVIEWED:`
    - one bullet per in-scope MUST/SHOULD clause reviewed, each with file:line evidence or an explicit proof note
    - when `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, reuse the exact clause text from `CLAUSE_CLOSURE_MATRIX`
  - `NOT_PROVEN:`
    - `- NONE` only when nothing remains unproven
    - otherwise list each unresolved clause/gap explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `MAIN_BODY_GAPS:`
    - `- NONE` only when no main-body requirement remains unproven, partial, or weakly evidenced
    - otherwise list each unresolved MUST/SHOULD gap explicitly
  - `QUALITY_RISKS:`
    - `- NONE` only when no material maintainability, brittleness, ambiguity, or heuristic-quality risk remains
    - otherwise list each residual code-quality risk explicitly
  - `VALIDATOR_RISK_TIER: LOW | MEDIUM | HIGH`
    - validator-assigned risk tier; MUST NOT be lower than the packet `RISK_TIER`
  - `DIFF_ATTACK_SURFACES:`
    - list the failure surfaces the validator derived from reading the diff directly
  - `INDEPENDENT_CHECKS_RUN:`
    - list validator-owned checks that were not copied from coder evidence, formatted as `what => observed`
  - `COUNTERFACTUAL_CHECKS:`
    - list concrete code-path / symbol counterfactuals in the form `If X were removed or altered, Y would break`
    - naming a test only is insufficient; name the file, symbol, or code path
  - `BOUNDARY_PROBES:`
    - required for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`
    - record the validator's interface / producer-consumer / storage / contract boundary checks
  - `NEGATIVE_PATH_CHECKS:`
    - required for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`
    - record invalid, missing, adversarial, or failure-path checks the validator ran
  - `INDEPENDENT_FINDINGS:`
    - list what the validator learned independently, even if the conclusion is baseline confirmation
  - `RESIDUAL_UNCERTAINTY:`
    - list remaining uncertainty explicitly; for `VALIDATOR_RISK_TIER=HIGH`, `- NONE` is illegal
  - `SPEC_CLAUSE_MAP:`
    - map each packet requirement to `file:line` evidence proving it is implemented
    - entries must include concrete code references (file paths, line numbers, or symbol names)
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`
  - `NEGATIVE_PROOF:`
    - list at least one spec requirement the validator verified is NOT fully implemented
    - this proves the validator independently read the code rather than trusting coder summaries
    - `- NONE` is illegal; every codebase has at least one gap or partial implementation
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`
- For `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `ANTI_VIBE_FINDINGS:`
    - `- NONE` only when the validator found no shallow easy-surface work, no weakly justified implementation, and no vibe-coded behavior inside signed scope
    - otherwise list each anti-vibe finding explicitly
  - `SIGNED_SCOPE_DEBT:`
    - `- NONE` only when no signed-scope debt, cleanup IOU, or "fix later" residue was accepted
    - otherwise list each signed-scope debt item explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, every appended governed validation report MUST also include:
  - `PRIMITIVE_RETENTION_PROOF:`
    - record the concrete file:line or symbol evidence proving previously exposed or expected primitives remain present after the change
    - for `RISK_TIER=MEDIUM|HIGH`, `- NONE` is illegal
  - `PRIMITIVE_RETENTION_GAPS:`
    - `- NONE` only when no primitive was weakened, dropped, or silently re-shaped inside signed scope
    - otherwise list each retained-feature or primitive-composition gap explicitly
  - `SHARED_SURFACE_INTERACTION_CHECKS:`
    - record the concrete producer/consumer, registry, type, runtime, or contract interaction checks reviewed across shared surfaces
    - for `RISK_TIER=MEDIUM|HIGH` or `SHARED_SURFACE_RISK=YES`, `- NONE` is illegal
  - `CURRENT_MAIN_INTERACTION_CHECKS:`
    - record the concrete current-`main` caller, consumer, or compatibility interactions reviewed against the packet diff
    - for `RISK_TIER=MEDIUM|HIGH` or `CURRENT_MAIN_COMPATIBILITY_STATUS=PASS`, `- NONE` is illegal
- When `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, every appended governed validation report MUST also include:
  - `DATA_CONTRACT_PROOF:`
    - list concrete code paths, emitted artifacts, or storage/query surfaces proving the active data contract was reviewed
  - `DATA_CONTRACT_GAPS:`
    - `- NONE` only when no SQL-portability, LLM-parseability, or Loom-intertwined gap remains inside signed scope
    - otherwise list each remaining gap explicitly
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, `HEURISTIC_REVIEW_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `SPEC_ALIGNMENT_VERDICT=PASS` is legal only when `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-01` and `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3|SPLIT_DIFF_SCOPED_RIGOR_V4`, `LEGAL_VERDICT=PASS` is legal only when `ANTI_VIBE_FINDINGS` and `SIGNED_SCOPE_DEBT` are both exactly `- NONE`.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `SPEC_ALIGNMENT_VERDICT=PASS` and `Verdict: PASS` are legal only when `PRIMITIVE_RETENTION_GAPS` is exactly `- NONE`.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `LEGAL_VERDICT=PASS` is legal only when `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS` all contain concrete code or symbol evidence.
- Rule: for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, `RISK_TIER=MEDIUM|HIGH` requires non-empty `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, and `CURRENT_MAIN_INTERACTION_CHECKS`.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `MECHANICAL_TRACK_VERDICT=PASS` is legal only when `GOVERNANCE_VERDICT`, `TEST_VERDICT`, `CODE_REVIEW_VERDICT`, `HEURISTIC_REVIEW_VERDICT`, `ENVIRONMENT_VERDICT`, `WORKFLOW_VALIDITY`, `SCOPE_VALIDITY`, `PROOF_COMPLETENESS`, `INTEGRATION_READINESS`, and `DOMAIN_GOAL_COMPLETION` are all in their PASS states.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `SPEC_RETENTION_TRACK_VERDICT=PASS` is legal only when `SPEC_ALIGNMENT_VERDICT=PASS`, `NOT_PROVEN`, `MAIN_BODY_GAPS`, and `PRIMITIVE_RETENTION_GAPS` are all exactly `- NONE`, and the report contains concrete `PRIMITIVE_RETENTION_PROOF`, `SHARED_SURFACE_INTERACTION_CHECKS`, `CURRENT_MAIN_INTERACTION_CHECKS`, `SPEC_CLAUSE_MAP`, and `NEGATIVE_PROOF` evidence.
- Rule: for `PACKET_FORMAT_VERSION >= 2026-04-05`, `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V4`, and `RISK_TIER=MEDIUM|HIGH`, `LEGAL_VERDICT=PASS` and `Verdict: PASS` are legal only when `MECHANICAL_TRACK_VERDICT=PASS` and `SPEC_RETENTION_TRACK_VERDICT=PASS`.
- Rule: when `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, `LEGAL_VERDICT=PASS` is legal only when `DATA_CONTRACT_PROOF` is present and `DATA_CONTRACT_GAPS` is exactly `- NONE`.
- Rule: `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- Rule: if `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line `Verdict` MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, `OUTDATED_ONLY`, or `ABANDONED` honestly.
- Rule: `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- Rule: `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- Rule: `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.
