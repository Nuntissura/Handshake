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
- For `REFINEMENT_ENFORCEMENT_PROFILE: HYDRATED_RESEARCH_V1`, this packet is auto-hydrated from the signed refinement; manual drift is forbidden and `just pre-work` enforces alignment.

---

# Task Packet: WP-1-Workflow-Projection-Correlation-v1

## METADATA
- TASK_ID: WP-1-Workflow-Projection-Correlation-v1
- WP_ID: WP-1-Workflow-Projection-Correlation-v1
- BASE_WP_ID: WP-1-Workflow-Projection-Correlation
- DATE: 2026-03-29T00:31:13.295Z
- MERGE_BASE_SHA: facce56f879d4ee990f62566b12a8b26d8bc61d7 (git merge-base main HEAD at creation time; use for deterministic `just post-work --range` evidence)
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
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL: gpt-5.4
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
- SESSION_REGISTRY_FILE: ../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json
- SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION: 2
- SESSION_PLUGIN_ATTEMPT_TIMEOUT_SECONDS: 20
- SESSION_WATCH_POLICY: EVENT_WATCH_PRIMARY_HEARTBEAT_FALLBACK
- SESSION_WAKE_CHANNEL_PRIMARY: VS_CODE_FILE_WATCH
- SESSION_WAKE_CHANNEL_FALLBACK: WP_HEARTBEAT
- CLI_ESCALATION_HOST_DEFAULT: SYSTEM_TERMINAL
- MODEL_FAMILY_POLICY: OPENAI_GPT_SERIES_ONLY
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Workflow-Projection-Correlation-v1
<!-- The WP Validator shares the coder branch/worktree [CX-503G]. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_LOCAL_BRANCH: feat/WP-1-Workflow-Projection-Correlation-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtc-projection-correlation-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Workflow-Projection-Correlation-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Workflow-Projection-Correlation-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Workflow-Projection-Correlation-v1
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Workflow-Projection-Correlation-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Workflow-Projection-Correlation-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Workflow-Projection-Correlation-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Workflow-Projection-Correlation-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Workflow-Projection-Correlation-v1
- EXTERNAL_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | CODE_VERDICT | GOVERNANCE_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT
- EXTERNAL_VALIDATOR_DISPOSITIONS: NONE | OUTDATED_ONLY
- EXTERNAL_VALIDATOR_LEGAL_VERDICTS: PASS | FAIL | PENDING
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V3
- GOVERNED_VALIDATOR_SPLIT_FIELDS: VALIDATION_CONTEXT | GOVERNANCE_VERDICT | TEST_VERDICT | CODE_REVIEW_VERDICT | HEURISTIC_REVIEW_VERDICT | SPEC_ALIGNMENT_VERDICT | ENVIRONMENT_VERDICT | DISPOSITION | LEGAL_VERDICT | SPEC_CONFIDENCE
- GOVERNED_VALIDATOR_COMPLETION_FIELDS: WORKFLOW_VALIDITY | SCOPE_VALIDITY | PROOF_COMPLETENESS | INTEGRATION_READINESS | DOMAIN_GOAL_COMPLETION
- CODER_HANDOFF_RIGOR_PROFILE: RUBRIC_SELF_AUDIT_V2
- CLAUSE_CLOSURE_MONITOR_PROFILE: CLAUSE_MONITOR_V1
<!-- Required for new packets: CLAUSE_MONITOR_V1 -->
- SEMANTIC_PROOF_PROFILE: DIFF_SCOPED_SEMANTIC_V1
<!-- Required for new packets: DIFF_SCOPED_SEMANTIC_V1 -->
- SPEC_DEBT_REGISTRY: .GOV/roles_shared/records/SPEC_DEBT_REGISTRY.md
- **Status:** Validated (PASS)
<!-- Allowed: Ready for Dev | In Progress | Blocked | Done | Validated (PASS) | Validated (FAIL) | Validated (OUTDATED_ONLY) -->
- MAIN_CONTAINMENT_STATUS: CONTAINED_IN_MAIN
<!-- Allowed: NOT_STARTED | MERGE_PENDING | CONTAINED_IN_MAIN | NOT_REQUIRED -->
- MERGED_MAIN_COMMIT: 1f1495a1c0801f17e8e99a01fec859962a717722
<!-- Use NONE until the approved closure commit is actually contained in local `main`. -->
- MAIN_CONTAINMENT_VERIFIED_AT_UTC: 2026-03-29T18:31:50.9205288Z
<!-- For PACKET_FORMAT_VERSION >= 2026-03-25: `Done` means merge-pending PASS only; `Validated (PASS)` is reserved for closures already contained in local `main`. -->
- CURRENT_MAIN_COMPATIBILITY_STATUS: COMPATIBLE
<!-- For PACKET_FORMAT_VERSION >= 2026-03-26. Allowed: NOT_RUN | COMPATIBLE | ADJACENT_SCOPE_REQUIRED | BLOCKED -->
- CURRENT_MAIN_COMPATIBILITY_BASELINE_SHA: 1f1495a1c0801f17e8e99a01fec859962a717722
<!-- Full local `main` HEAD sha inspected by the Integration Validator when current-main compatibility is checked. -->
- CURRENT_MAIN_COMPATIBILITY_VERIFIED_AT_UTC: 2026-03-29T18:31:50.9205288Z
<!-- RFC3339 UTC; required when CURRENT_MAIN_COMPATIBILITY_STATUS is not NOT_RUN. -->
- PACKET_WIDENING_DECISION: NOT_REQUIRED
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
- BUILD_ORDER_DEPENDS_ON: WP-1-Workflow-Engine, WP-1-AI-Job-Model, WP-1-Flight-Recorder, WP-1-Locus-Phase1-Integration-Occupancy, WP-1-Artifact-System-Foundations
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: NONE
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Workflow-Projection-Correlation-v1
- LOCAL_WORKTREE_DIR: ../wtc-projection-correlation-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Workflow-Projection-Correlation-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Workflow-Projection-Correlation-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja290320260124
- PACKET_FORMAT_VERSION: 2026-03-29

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PASS
Blockers: NONE
Next: INTEGRATION_CLOSEOUT_SYNC

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: HSK-WF-001 durable node persistence plus recovery-safe node lineage must become bounded export anchors | CODE_SURFACES: `src/backend/handshake_core/src/storage/mod.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/schemas.rs` | TESTS: `storage::tests::workflow_node_execution_persists_inputs_and_outputs`; `bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage` | EXAMPLES: `export_manifest.json` with `scope.kind = "workflow_run"` and explicit `workflow_run_id`, `export_manifest.json` with `scope.kind = "workflow_node_execution"` and explicit `workflow_run_id` plus `workflow_node_execution_id`, `workflow_node_executions.jsonl` containing one exported node execution line with stable ids and bounded hashes, exportable inventory row that surfaces a workflow-run anchor without degrading to a broad time-window entry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: AI Job Model runtime identity requires `workflow_run_id` to remain a first-class runtime anchor | CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/templates.rs` | TESTS: `bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes`; `bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors` | EXAMPLES: `export_manifest.json` with `scope.kind = "workflow_run"` and explicit `workflow_run_id`, `export_manifest.json` with `scope.kind = "workflow_node_execution"` and explicit `workflow_run_id` plus `workflow_node_execution_id`, `workflow_node_executions.jsonl` containing one exported node execution line with stable ids and bounded hashes, exportable inventory row that surfaces a workflow-run anchor without degrading to a broad time-window entry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: Debug Bundle manifest scope and exporter contract must admit workflow-run and node-execution scope kinds | CODE_SURFACES: `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/schemas.rs`, `src/backend/handshake_core/src/bundles/templates.rs` | TESTS: `bundles::validator::tests::val_bundle_001_reports_missing_files`; `bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes`; `bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage` | EXAMPLES: `export_manifest.json` with `scope.kind = "workflow_run"` and explicit `workflow_run_id`, `export_manifest.json` with `scope.kind = "workflow_node_execution"` and explicit `workflow_run_id` plus `workflow_node_execution_id`, `workflow_node_executions.jsonl` containing one exported node execution line with stable ids and bounded hashes, exportable inventory row that surfaces a workflow-run anchor without degrading to a broad time-window entry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
  - CLAUSE: export manifest requirements that already mention `workflow_run_id` must be reconciled with explicit scope law and workflow-node inventory proof | CODE_SURFACES: `src/backend/handshake_core/src/bundles/schemas.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/validator.rs` | TESTS: `bundles::validator::tests::val_bundle_001_reports_missing_files`; `bundles::zip::tests::bundle_determinism_hash_stable`; golden manifest assertions for workflow scope kinds | EXAMPLES: `export_manifest.json` with `scope.kind = "workflow_run"` and explicit `workflow_run_id`, `export_manifest.json` with `scope.kind = "workflow_node_execution"` and explicit `workflow_run_id` plus `workflow_node_execution_id`, `workflow_node_executions.jsonl` containing one exported node execution line with stable ids and bounded hashes, exportable inventory row that surfaces a workflow-run anchor without degrading to a broad time-window entry | DEBT_IDS: NONE | CODER_STATUS: PROVED | VALIDATOR_STATUS: CONFIRMED
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture`
  - `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture`
- CANONICAL_CONTRACT_EXAMPLES:
  - `export_manifest.json` with `scope.kind = "workflow_run"` and explicit `workflow_run_id`
  - `export_manifest.json` with `scope.kind = "workflow_node_execution"` and explicit `workflow_run_id` plus `workflow_node_execution_id`
  - `workflow_node_executions.jsonl` containing one exported node execution line with stable ids and bounded hashes
  - exportable inventory row that surfaces a workflow-run anchor without degrading to a broad time-window entry
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
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
  - primary launch path is `SESSION_HOST_PREFERENCE` via `SESSION_PLUGIN_BRIDGE_ID`
  - if the plugin path fails or times out `SESSION_PLUGIN_MAX_RETRIES_BEFORE_ESCALATION` times for the same role/WP session, `CLI_ESCALATION_HOST_DEFAULT` is allowed as the escalation host
  - `SESSION_PLUGIN_REQUESTS_FILE` and `SESSION_REGISTRY_FILE` are the deterministic launch/watch state for those sessions
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Workflow-Projection-Correlation-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.6 Workflow & Automation Engine [HSK-WF-001] Durable Node Persistence (Normative)
- CONTEXT_START_LINE: 9176
- CONTEXT_END_LINE: 9178
- CONTEXT_TOKEN: The Workflow Engine MUST persist every node execution, status transition, and input/output payload to the database.
- EXCERPT_ASCII_ESCAPED:
  ```text
**[HSK-WF-001] Durable Node Persistence (Normative):**
  The Workflow Engine MUST persist every node execution, status transition, and input/output payload to the database. A "minimal" async wrapper that only logs start/end events is insufficient.
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.6.6 AI Job Model runtime identity relationship
- CONTEXT_START_LINE: 9688
- CONTEXT_END_LINE: 9693
- CONTEXT_TOKEN: - `workflow_run_id` is the **runtime** instance (one per execution attempt)
- EXCERPT_ASCII_ESCAPED:
  ```text
**Relationship:**
  - `job_id` is the **logical** identity (stable across retries, visible to users)
  - `workflow_run_id` is the **runtime** instance (one per execution attempt)

  **Key Principle:** There is no separate AI jobs executor. The workflow engine (Section 2.6) is the **only** execution path for AI jobs.
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 4.3 Recovery-safe run history [ADD v02.165]
- CONTEXT_START_LINE: 32778
- CONTEXT_END_LINE: 32780
- CONTEXT_TOKEN: stable `workflow_run_id`, `workflow_node_execution_id`
- EXCERPT_ASCII_ESCAPED:
  ```text
- **INV-RECOVER-003:** All recovery actions MUST be logged in Flight Recorder with `FR-EVT-WF-RECOVERY` correlation.

  [ADD v02.165] Recovery-safe run history MUST preserve queue-state transitions, workflow-node execution lineage, tool-call lineage, checkpoint chronology, and operator replay decisions by stable `workflow_run_id`, `workflow_node_execution_id`, `session_id`, `tool_call_id`, and `checkpoint_id` values.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.10 Debug Bundle export manifest scope contract
- CONTEXT_START_LINE: 57826
- CONTEXT_END_LINE: 57833
- CONTEXT_TOKEN: kind: "problem" | "job" | "workflow_run" | "workflow_node_execution" | "time_window" | "workspace";
- EXCERPT_ASCII_ESCAPED:
  ```text
// Scope
  scope: {
    kind: "problem" | "job" | "workflow_run" | "workflow_node_execution" | "time_window" | "workspace";
    problem_id?: string;
    job_id?: string;
    workflow_run_id?: string;
    workflow_node_execution_id?: string;
    time_range?: {
      start: string;
      end: string;
    };
    wsid?: string;
  };
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 2.3.10 DebugBundleExporter trait contract
- CONTEXT_START_LINE: 58282
- CONTEXT_END_LINE: 58289
- CONTEXT_TOKEN: /// Export a debug bundle for the given scope.
- EXCERPT_ASCII_ESCAPED:
  ```text
#[async_trait]
  pub trait DebugBundleExporter: Send + Sync {
      /// Export a debug bundle for the given scope.
      ///
      /// # Arguments
      /// * `request` - Export parameters including scope and redaction mode
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 10.5.6.12 VAL-BUNDLE-001: Debug Bundle Completeness (Expanded)
- CONTEXT_START_LINE: 58682
- CONTEXT_END_LINE: 58686
- CONTEXT_TOKEN: included.workflow_node_execution_count
- EXCERPT_ASCII_ESCAPED:
  ```text
- `scope.workflow_run_id` is present when `scope.kind = "workflow_run"`
  - `scope.workflow_run_id` and `scope.workflow_node_execution_id` are present when `scope.kind = "workflow_node_execution"`
  - `included.workflow_node_execution_count` matches the number of lines in `workflow_node_executions.jsonl` when that file is present
  - A `workflow_node_execution` scoped bundle contains exactly one targeted node execution record and all listed node executions share the scoped `workflow_run_id`
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: HSK-WF-001 durable node persistence plus recovery-safe node lineage must become bounded export anchors | WHY_IN_SCOPE: persisted workflow and node ids are only useful for bounded export if the bundle contract admits them directly | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/storage/mod.rs`, `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/schemas.rs` | EXPECTED_TESTS: `storage::tests::workflow_node_execution_persists_inputs_and_outputs`; `bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage` | RISK_IF_MISSED: node lineage stays persisted but not exportable, forcing manual reconstruction
  - CLAUSE: AI Job Model runtime identity requires `workflow_run_id` to remain a first-class runtime anchor | WHY_IN_SCOPE: workflow-scoped bundle export must bind to runtime execution rather than only to logical job identity | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/templates.rs` | EXPECTED_TESTS: `bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes`; `bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors` | RISK_IF_MISSED: exports silently collapse runtime lineage into job-level approximations
  - CLAUSE: Debug Bundle manifest scope and exporter contract must admit workflow-run and node-execution scope kinds | WHY_IN_SCOPE: current Main Body scope union blocks the intended packet scope | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/schemas.rs`, `src/backend/handshake_core/src/bundles/templates.rs` | EXPECTED_TESTS: `bundles::validator::tests::val_bundle_001_reports_missing_files`; `bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes`; `bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage` | RISK_IF_MISSED: code either remains incomplete or drifts outside the spec
  - CLAUSE: export manifest requirements that already mention `workflow_run_id` must be reconciled with explicit scope law and workflow-node inventory proof | WHY_IN_SCOPE: manifest law is currently internally inconsistent across required ids, scope kinds, and inventory counts | EXPECTED_CODE_SURFACES: `src/backend/handshake_core/src/bundles/schemas.rs`, `src/backend/handshake_core/src/bundles/exporter.rs`, `src/backend/handshake_core/src/bundles/validator.rs` | EXPECTED_TESTS: `bundles::validator::tests::val_bundle_001_reports_missing_files`; `bundles::zip::tests::bundle_determinism_hash_stable`; golden manifest assertions for workflow scope kinds | RISK_IF_MISSED: validators can pass bundles that claim workflow correlation without complete manifest evidence
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: `DebugBundleRequest.scope` | PRODUCER: `src/backend/handshake_core/src/bundles/exporter.rs` callers | CONSUMER: `src/backend/handshake_core/src/bundles/exporter.rs` exporter implementation | SERIALIZER_TRANSPORT: in-process Rust struct serialized into manifest | VALIDATOR_READER: bundle validator and downstream manifest readers | TRIPWIRE_TESTS: workflow-run scope and workflow-node scope exporter tests | DRIFT_RISK: code can add new scope kinds without matching manifest/validator support
  - CONTRACT: `BundleManifest.scope` | PRODUCER: `build_manifest_scope` in `src/backend/handshake_core/src/bundles/exporter.rs` | CONSUMER: bundle validator, export templates, operator tooling | SERIALIZER_TRANSPORT: `export_manifest.json` | VALIDATOR_READER: bundle validator manifest parsing | TRIPWIRE_TESTS: golden manifest assertions for `workflow_run` and `workflow_node_execution` | DRIFT_RISK: manifest can overstate workflow correlation without canonical scope fields
  - CONTRACT: workflow node execution inventory file | PRODUCER: bundle exporter inventory writer | CONSUMER: validator, replay/audit readers, future operator tooling | SERIALIZER_TRANSPORT: `workflow_node_executions.jsonl` | VALIDATOR_READER: bundle validator inventory checks | TRIPWIRE_TESTS: node-scope export test plus validator fixture checks | DRIFT_RISK: node lineage remains implicit and semantically unverified
  - CONTRACT: exportable inventory projection | PRODUCER: `list_exportable` in `src/backend/handshake_core/src/bundles/exporter.rs` | CONSUMER: operator tooling and future Command Center bundle pickers | SERIALIZER_TRANSPORT: in-process response payloads | VALIDATOR_READER: targeted unit tests over inventory rows | TRIPWIRE_TESTS: `bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors` | DRIFT_RISK: workflow anchors remain invisible even after backend support lands
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Wait for the Main Body enrichment and `SPEC_CURRENT` advance; do not start code against the old scope contract.
  - Extend bundle scope and manifest schema for `workflow_run` and `workflow_node_execution`.
  - Implement workflow-run and node-execution export filtering in `bundles/exporter.rs` using existing persisted lineage and current Flight Recorder correlation fields.
  - Add canonical workflow-node execution inventory emission and manifest counts.
  - Extend `list_exportable` to surface workflow correlation anchors.
  - Add targeted exporter and validator tripwire tests for workflow-run and node-execution scope semantics.
- HOT_FILES:
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture
- CARRY_FORWARD_WARNINGS:
  - Do not widen into generic Task Board schema redesign or Dev Command Center UI work.
  - Do not invent new workflow ids or recorder ids when persisted workflow and node lineage already exist.
  - Do not use broad time-window exports as a substitute for explicit workflow-run or node-execution scope kinds.
  - Keep mailbox, diagnostics-only, and Locus-only bundle bridges as separate follow-on packets.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - workflow and recovery law requiring stable `workflow_run_id` and `workflow_node_execution_id`
  - debug bundle scope union and exporter contract after enrichment
  - export manifest requirements for `workflow_run_id` and workflow-node inventory proof
  - exportable inventory visibility for workflow correlation anchors
- FILES_TO_READ:
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
- COMMANDS_TO_RUN:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture
  - rg -n "BundleScope|workflow_run_id|workflow_node_execution|list_exportable|build_manifest_scope|collect_events|collect_jobs" src/backend/handshake_core/src
- POST_MERGE_SPOTCHECKS:
  - Verify no workflow-scoped export silently falls back to time-window semantics.
  - Verify node-scoped export includes only the targeted node lineage and bound workflow run.
  - Verify manifest counts and inventory files stay deterministic across repeated exports.
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - The final bundle inventory filename can be `workflow_node_executions.jsonl` as proposed here, but the exact filename is not yet product-code-proven.
  - The cleanest internal join strategy for node-scoped export across diagnostics, jobs, and Flight Recorder events is not yet code-proven.
  - Whether Command Center should surface workflow-run and node-execution anchors in one picker or separate grouped views remains intentionally out of scope for this packet.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: NO
- RESEARCH_CURRENCY_VERDICT: NOT_APPLICABLE
- RESEARCH_DEPTH_VERDICT: NOT_APPLICABLE
- GITHUB_PROJECT_SCOUTING_VERDICT: NOT_APPLICABLE
- SOURCE_LOG:
  - NONE
- RESEARCH_SYNTHESIS:
  - Internal sources are sufficient because the unresolved work is explicit in the current Master Spec and in the current local exporter/workflow code.
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
  - PRIM-WorkflowRun
  - PRIM-WorkflowNodeExecution
  - PRIM-AiJob
  - PRIM-FlightRecorder
  - PRIM-DebugBundleExporter
  - PRIM-BundleScope
  - PRIM-BundleManifest
  - PRIM-DebugBundleRequest
- PRIMITIVES_EXPOSED:
  - PRIM-WorkflowRun
  - PRIM-WorkflowNodeExecution
  - PRIM-DebugBundleExporter
  - PRIM-BundleScope
  - PRIM-BundleManifest
  - PRIM-DebugBundleRequest
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.director
  - engine.archivist
  - engine.librarian
  - engine.analyst
  - engine.dba
  - engine.context
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NO_CHANGE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Flight Recorder
  - Locus
  - Execution / Job Runtime
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - WorkflowRun + FlightRecorder + DebugBundleExporter + Locus correlation -> IN_THIS_WP (stub: NONE)
  - WorkflowNodeExecution + BundleManifest + export inventory + replay-safe chronology -> IN_THIS_WP (stub: NONE)
  - WorkflowRun + AiJob runtime identity + workflow-scoped bundle manifest -> IN_THIS_WP (stub: NONE)
  - WorkflowNodeExecution + FlightRecorder event filtering + targeted diagnostics set -> IN_THIS_WP (stub: NONE)
  - WorkflowRun + list_exportable inventory + operator bundle selection -> IN_THIS_WP (stub: NONE)
  - WorkflowNodeExecution + workflow_node_executions.jsonl + downstream replay readers -> IN_THIS_WP (stub: NONE)
  - WorkflowRun + deterministic zip manifest hashes + portable exported entity counts -> IN_THIS_WP (stub: NONE)
  - WorkflowNodeExecution + backend-portable storage joins + bounded export filtering -> IN_THIS_WP (stub: NONE)
  - Locus-ready projection + workflow_run anchor + direct bundle seed path -> IN_THIS_WP (stub: NONE)
  - FlightRecorder chronology + workflow_node_execution_id + replay-safe export manifest -> IN_THIS_WP (stub: NONE)
  - WorkflowRun + templates-generated prompts + exported workflow context -> IN_THIS_WP (stub: NONE)
  - WorkflowNodeExecution + diagnostics bridge + export validator proof -> IN_THIS_WP (stub: NONE)
  - WorkflowRun + workflow_node_execution inventory + storage portability posture -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: workflow-run bounded export anchor | SUBFEATURES: workflow-run manifest scope, workflow-run exporter routing, workflow-run list_exportable inventory | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-DebugBundleExporter, PRIM-BundleScope, PRIM-DebugBundleRequest | MECHANICAL: engine.director, engine.context, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: `v02.179` now declares the scope law; this packet must implement and prove the runtime/exporter path.
  - PILLAR: Execution / Job Runtime | CAPABILITY_SLICE: workflow-node execution bounded export anchor | SUBFEATURES: node-scoped manifest scope, node-scoped bundle filtering, node execution inventory file | PRIMITIVES_FEATURES: PRIM-WorkflowNodeExecution, PRIM-BundleManifest, PRIM-BundleScope, PRIM-DebugBundleExporter | MECHANICAL: engine.archivist, engine.analyst, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: node lineage is already explicit in `v02.179`; current code still needs the bounded export and inventory path.
  - PILLAR: Flight Recorder | CAPABILITY_SLICE: workflow-correlated recorder evidence reuse | SUBFEATURES: export filtering by existing workflow ids, bounded event inclusion, replay-safe chronology | PRIMITIVES_FEATURES: PRIM-FlightRecorder, PRIM-WorkflowRun, PRIM-WorkflowNodeExecution, PRIM-BundleScope | MECHANICAL: engine.context, engine.analyst | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: reuse current recorder correlation fields instead of adding new event families.
  - PILLAR: Locus | CAPABILITY_SLICE: workflow correlation handoff into bounded export | SUBFEATURES: Locus-ready bundle seed path, progress-to-bundle anchor resolution, durable workflow correlation joins | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-DebugBundleRequest, PRIM-BundleManifest | MECHANICAL: engine.director, engine.archivist | ROI: MEDIUM | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: keep this limited to existing joins after the spec update lands; do not widen into Task Board schema redesign
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: backend-portable workflow correlation joins | SUBFEATURES: storage-neutral workflow-run joins, storage-neutral node execution joins, deterministic bundle filtering across storage backends | PRIMITIVES_FEATURES: PRIM-WorkflowRun, PRIM-WorkflowNodeExecution, PRIM-DebugBundleExporter | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: the bundle-scope contract must stay portable across current SQLite and future PostgreSQL execution paths.
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: workflow-node export inventory for machine-readable replay | SUBFEATURES: workflow node execution inventory lines, stable ids, bounded hash fields | PRIMITIVES_FEATURES: PRIM-WorkflowNodeExecution, PRIM-BundleManifest, PRIM-DebugBundleRequest | MECHANICAL: engine.librarian, engine.analyst, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: explicit workflow-node export records reduce later model dependence on transcript reconstruction.
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: Workflow-run scoped debug bundle export | JobModel: WORKFLOW | Workflow: Workflow Engine -> Debug Bundle export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: existing workflow_id-correlated event families | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: the runtime substrate exists and now needs bounded workflow-run export implementation and proof.
  - Capability: Workflow-node-execution scoped debug bundle export | JobModel: WORKFLOW | Workflow: Workflow Engine -> Debug Bundle export | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: existing workflow_id-correlated event families plus node lineage joins | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: node execution ids are already persisted; this packet must connect them to exporter and manifest behavior.
  - Capability: Exportable inventory for workflow correlation anchors | JobModel: WORKFLOW | Workflow: Debug Bundle export inventory | ToolSurface: COMMAND_CENTER | ModelExposure: OPERATOR_ONLY | CommandCenter: PLANNED | FlightRecorder: NONE | Locus: PLANNED | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: `list_exportable` currently inventories jobs and diagnostics only.
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: NEEDS_SCOPE_EXPANSION
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Locus-Debug-Bundle-Bridge-v1 -> KEEP_SEPARATE
  - WP-1-Diagnostics-Debug-Bundle-Bridge-v1 -> KEEP_SEPARATE
  - WP-1-Role-Mailbox-Debug-Bundle-Bridge-v1 -> KEEP_SEPARATE
  - WP-1-Debug-Bundle-v3 -> EXPAND_IN_THIS_WP
  - WP-1-AI-Job-Model-v4 -> KEEP_SEPARATE
  - WP-1-Workflow-Engine-v4 -> KEEP_SEPARATE
  - WP-1-Flight-Recorder-v4 -> KEEP_SEPARATE
  - WP-1-Locus-Phase1-Integration-Occupancy-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/storage/mod.rs -> IMPLEMENTED (WP-1-Workflow-Engine-v4)
  - ../handshake_main/src/backend/handshake_core/src/workflows.rs -> IMPLEMENTED (WP-1-Workflow-Engine-v4)
  - ../handshake_main/src/backend/handshake_core/src/bundles/exporter.rs -> PARTIAL (WP-1-Debug-Bundle-v3)
  - ../handshake_main/src/backend/handshake_core/src/bundles/schemas.rs -> PARTIAL (WP-1-Debug-Bundle-v3)
  - ../handshake_main/src/backend/handshake_core/src/bundles/templates.rs -> PARTIAL (WP-1-Debug-Bundle-v3)
  - ../handshake_main/src/backend/handshake_core/src/locus/task_board.rs -> PARTIAL (WP-1-Locus-Phase1-Integration-Occupancy-v1)
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
- What: Expand the Debug Bundle contract so `workflow_run` and `workflow_node_execution` become first-class bounded export scopes, with matching manifest fields, export inventory, and validator proof.
- Why: Backend workflow failures should be exportable and replayable from stable workflow lineage rather than reconstructed manually from jobs, diagnostics, or broad time windows.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/bundles/validator.rs
  - src/backend/handshake_core/src/bundles/zip.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
  - src/backend/handshake_core/tests
- OUT_OF_SCOPE:
  - Dev Command Center UI redesign
  - generic Task Board schema redesign
  - replay execution beyond bounded export/projection contract
  - mailbox-specific bundle scope work
- TOUCHED_FILE_BUDGET: 7
<!-- Max unique in-scope files allowed in the evaluated diff. Raise intentionally before coding if the packet truly needs broader edit spread. -->
- BROAD_TOOL_ALLOWLIST: NONE
<!-- Allowed: NONE | FORMATTER | CODEGEN | SEARCH_REPLACE | MIGRATION_REWRITE -->
## WAIVERS GRANTED
- (Record explicit user waivers here per [CX-573F]. Prefer pipe records so the computed policy gate can classify them deterministically.)
- (Format: `- WAIVER_ID: CX-... | STATUS: ACTIVE | COVERS: SCOPE, PROOF, TEST, ENVIRONMENT, PROTECTED_SURFACE, HEURISTIC | SCOPE: <WP/local scope> | JUSTIFICATION: <why> | APPROVER: <user/operator> | EXPIRES: <date or condition>`.)
- NONE

## QUALITY_GATE
### TEST_PLAN
```bash
cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture
```

### DONE_MEANS
- Current code implements the explicit `v02.179` `workflow_run` and `workflow_node_execution` bundle scope kinds end to end.
- exporter, manifest schema, and validator all accept and prove those bounded scope kinds.
- workflow-scoped and node-scoped bundle exports only include correlated jobs/events/node records.
- canonical export inventory includes workflow node execution records and manifest counts.

- PRIMITIVES_EXPOSED:
  - PRIM-WorkflowRun
  - PRIM-WorkflowNodeExecution
  - PRIM-DebugBundleExporter
  - PRIM-BundleScope
  - PRIM-BundleManifest
  - PRIM-DebugBundleRequest
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-03-29T00:31:13.295Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- SPEC_ANCHOR_PRIMARY: Workflow persistence and recovery law already require stable `workflow_run_id` and `workflow_node_execution_id`, but the current Debug Bundle manifest scope union does not allow those ids as canonical bounded export anchors.
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
  - .GOV/spec/Handshake_Master_Spec_v02.179.md
  - src/backend/handshake_core/src/bundles/exporter.rs
  - src/backend/handshake_core/src/bundles/schemas.rs
  - src/backend/handshake_core/src/bundles/templates.rs
  - src/backend/handshake_core/src/workflows.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/storage/tests.rs
- SEARCH_TERMS:
  - workflow_run_id
  - workflow_node_execution
  - BundleScope
  - build_manifest_scope
  - list_exportable
  - export_for_job
- RUN_COMMANDS:
  ```bash
rg -n "WorkflowRun|WorkflowNodeExecution|workflow_run_id|workflow_node_execution" src/backend/handshake_core/src
  rg -n "enum BundleScope|build_manifest_scope|collect_events|collect_jobs|list_exportable|export_for_job" src/backend/handshake_core/src/bundles/exporter.rs
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture
  ```
- RISK_MAP:
  - "workflow-scoped export leaks unrelated records" -> "debug bundles become over-broad and unsafe for replay or sharing"
  - "node-scoped export lacks canonical node inventory" -> "validators can pass semantically incomplete bundles"
  - "implementation reconstructs lineage from time windows" -> "chronology and evidence can drift silently"
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
- **Target File**: `src/backend/handshake_core/src/bundles/exporter.rs`
- **Start**: 20
- **End**: 2916
- **Line Delta**: 980
- **Pre-SHA1**: `2cf29ae41b8f80c154817e055267ee19d2b8381d`
- **Post-SHA1**: `78393a2002c8f2938465f3e8e1ac46b1b4e181db`
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
- **Lint Results**: Not run; proof chain is the clean-room packet test plan plus deterministic gate checks.
- **Artifacts**: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/isolated-six-file.patch`
- **Timestamp**: `2026-03-29T03:38:25.2854116Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: committed single-rev diff `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..274341181b694e8ae6699b047117d136bbd3f041`; clean-room proof rerun against `../handshake_main` HEAD `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`
- **Target File**: `src/backend/handshake_core/src/bundles/schemas.rs`
- **Start**: 12
- **End**: 494
- **Line Delta**: 45
- **Pre-SHA1**: `135d989e878ee11e955bfadbf6cf6d4f53636502`
- **Post-SHA1**: `83d11997545ea857dd09c726e13a057a47d78aca`
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
- **Lint Results**: Not run; proof chain is the clean-room packet test plan plus deterministic gate checks.
- **Artifacts**: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/isolated-six-file.patch`
- **Timestamp**: `2026-03-29T03:38:25.2854116Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: committed single-rev diff `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..274341181b694e8ae6699b047117d136bbd3f041`; clean-room proof rerun against `../handshake_main` HEAD `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`
- **Target File**: `src/backend/handshake_core/src/bundles/templates.rs`
- **Start**: 4
- **End**: 260
- **Line Delta**: 27
- **Pre-SHA1**: `c0f26f15069ebbcb21271e3f673a9f81f6e44d5c`
- **Post-SHA1**: `d74c9658fef0e48f1fca1d1f729de7dc5380b4a0`
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
- **Lint Results**: Not run; proof chain is the clean-room packet test plan plus deterministic gate checks.
- **Artifacts**: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/isolated-six-file.patch`
- **Timestamp**: `2026-03-29T03:38:25.2854116Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: committed single-rev diff `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..274341181b694e8ae6699b047117d136bbd3f041`; clean-room proof rerun against `../handshake_main` HEAD `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`
- **Target File**: `src/backend/handshake_core/src/bundles/validator.rs`
- **Start**: 12
- **End**: 857
- **Line Delta**: 216
- **Pre-SHA1**: `6a8bbe6b16a8aaab343cb2c8d944468982ee8ede`
- **Post-SHA1**: `f1948b019be45a986fa42cda999935d10ae0f2b7`
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
- **Lint Results**: Not run; proof chain is the clean-room packet test plan plus deterministic gate checks.
- **Artifacts**: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/isolated-six-file.patch`
- **Timestamp**: `2026-03-29T03:38:25.2854116Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: committed single-rev diff `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..274341181b694e8ae6699b047117d136bbd3f041`; clean-room proof rerun against `../handshake_main` HEAD `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`
- **Target File**: `src/backend/handshake_core/src/bundles/zip.rs`
- **Start**: 84
- **End**: 105
- **Line Delta**: 3
- **Pre-SHA1**: `d37cdab7fa8838a47e5d172481350857741069d8`
- **Post-SHA1**: `7d61a4c30fd7b0d06fca341f6278f87aad65c029`
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
- **Lint Results**: Not run; proof chain is the clean-room packet test plan plus deterministic gate checks.
- **Artifacts**: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/isolated-six-file.patch`
- **Timestamp**: `2026-03-29T03:38:25.2854116Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: committed single-rev diff `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..274341181b694e8ae6699b047117d136bbd3f041`; clean-room proof rerun against `../handshake_main` HEAD `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`
- **Target File**: `src/backend/handshake_core/src/workflows.rs`
- **Start**: 4671
- **End**: 11827
- **Line Delta**: 40
- **Pre-SHA1**: `a77b1a14aad10787f4fb5b4c8347de5b8ec484e2`
- **Post-SHA1**: `87d0edfefde88a9e15144afa6b141aa2cf3b915b`
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
- **Lint Results**: Not run; proof chain is the clean-room packet test plan plus deterministic gate checks.
- **Artifacts**: `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Workflow-Projection-Correlation-v1/isolated-six-file.patch`
- **Timestamp**: `2026-03-29T03:38:25.2854116Z`
- **Operator**: `CODER_A`
- **Spec Target Resolved**: .GOV/spec/SPEC_CURRENT.md -> Handshake_Master_Spec_v02.179.md
- **Notes**: committed single-rev diff `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..274341181b694e8ae6699b047117d136bbd3f041`; clean-room proof rerun against `../handshake_main` HEAD `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`
## STATUS_HANDOFF
- Current WP_STATUS: `In Progress`; committed product handoff is fixed at `274341181b694e8ae6699b047117d136bbd3f041`, and packet hygiene is now concrete for deterministic gate evaluation.
- What changed in this update: removed the packet BOM, replaced the placeholder `## VALIDATION`, `## STATUS_HANDOFF`, `## EVIDENCE_MAPPING`, and `## EVIDENCE` sections, and recorded the six-file manifest coverage plus fresh clean-room proof-chain evidence.
- Requirements / clauses self-audited: workflow bundle scope kinds, workflow-only event completeness, node-lineage export, manifest and inventory alignment, validator enforcement, and clean-room `workflows.rs` contract compatibility inside the signed six-file surface.
- Checks actually run: `git -C ..\\handshake_main apply --check ..\\gov_runtime\\roles_shared\\WP_COMMUNICATIONS\\WP-1-Workflow-Projection-Correlation-v1\\isolated-six-file.patch`; `git -C ..\\handshake_main apply ..\\gov_runtime\\roles_shared\\WP_COMMUNICATIONS\\WP-1-Workflow-Projection-Correlation-v1\\isolated-six-file.patch`; six packet proof `cargo test` commands in `..\\handshake_main`; `git -C ..\\handshake_main apply -R ..\\gov_runtime\\roles_shared\\WP_COMMUNICATIONS\\WP-1-Workflow-Projection-Correlation-v1\\isolated-six-file.patch`; `git -C ..\\handshake_main status --short`.
- Known gaps / weak spots: packet hygiene is repaired, but this checkout still carries unrelated branch-local product drift outside the signed surface, so `just pre-work` and `just post-work --rev 274341181b694e8ae6699b047117d136bbd3f041` cannot pass here without approval-gated state hiding or cleanup.
- Heuristic risks / maintainability concerns: workflow correlation remains bounded to proven `workflow_id` and run `job_id` anchors; if flight-recorder exact-id recovery changes, the exporter tripwires in `bundles/exporter.rs` must continue to catch completeness regressions.
- Validator focus request: confirm that packet hygiene is now sufficient for deterministic post-work review and treat the remaining blocker as checkout drift only, not a feature-semantic defect.
- Rubric contract understanding proof: this packet promises bounded `workflow_run` and `workflow_node_execution` export anchors, validator-visible manifest evidence, and clean-room compile compatibility on current `main`; those claims are now backed by code refs, manifest rows, and fresh clean-room commands rather than chat summaries.
- Rubric scope discipline proof: no new product surface was added after commit `274341181b694e8ae6699b047117d136bbd3f041`; the signed product diff remains exactly the six committed files, and this update changes packet hygiene only.
- Rubric baseline comparison: compared the committed single-rev diff `f85d767d8ae8a56121f224f6e12ed2df6f973d6b..274341181b694e8ae6699b047117d136bbd3f041` plus the isolated six-file patch against clean `handshake_main` HEAD `a1fb1773e5cf506ec9d926a14ce7b0c0d2bf025c`; clean-room apply, test, and reverse-apply stayed within that boundary.
- Rubric end-to-end proof: the isolated six-file patch applied cleanly to `../handshake_main`, all six packet proof commands passed there, and the patch reversed back to a clean tree.
- Rubric architecture fit self-review: the signed changes extend the existing debug-bundle/export contract with first-class workflow anchors and compile-contract fixes without widening API, UI, or new shared product surfaces.
- Rubric heuristic quality self-review: the strongest remaining issue is environmental rather than semantic; deterministic gates still refuse to ignore unrelated dirty files in this checkout, which is the correct behavior.
- Rubric anti-gaming / counterfactual check: if any manifest row, file:line evidence anchor, or clean-room proof command were removed, the deterministic gate and validator audit would lose direct traceability; if unrelated branch-local dirt were ignored, the current gates would stop enforcing the signed scope boundary.
- Next step / handoff hint: if Operator approval is granted for approval-gated dirt parking, push the WP backup branch, stash only the unrelated branch-local drift, rerun `just pre-work WP-1-Workflow-Projection-Correlation-v1`, rerun `just post-work WP-1-Workflow-Projection-Correlation-v1 --rev 274341181b694e8ae6699b047117d136bbd3f041`, then restore the parked state; without that approval, treat committed-handoff as semantically proven but not gate-passable from this checkout.

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
- `Validated (FAIL)` and `Validated (OUTDATED_ONLY)` must use `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`.
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
- REQUIREMENT: "Current code implements the explicit `v02.179` `workflow_run` and `workflow_node_execution` bundle scope kinds end to end."
- EVIDENCE: `src/backend/handshake_core/src/bundles/exporter.rs:158`, `src/backend/handshake_core/src/bundles/schemas.rs:13`, `src/backend/handshake_core/src/workflows.rs:7884`
- REQUIREMENT: "exporter, manifest schema, and validator all accept and prove those bounded scope kinds."
- EVIDENCE: `src/backend/handshake_core/src/bundles/schemas.rs:33`, `src/backend/handshake_core/src/bundles/templates.rs:173`, `src/backend/handshake_core/src/bundles/validator.rs:303`, `src/backend/handshake_core/src/bundles/validator.rs:847`
- REQUIREMENT: "workflow-scoped and node-scoped bundle exports only include correlated jobs/events/node records."
- EVIDENCE: `src/backend/handshake_core/src/bundles/exporter.rs:456`, `src/backend/handshake_core/src/bundles/exporter.rs:525`, `src/backend/handshake_core/src/bundles/exporter.rs:1186`, `src/backend/handshake_core/src/bundles/exporter.rs:1213`, `src/backend/handshake_core/src/bundles/exporter.rs:2686`, `src/backend/handshake_core/src/bundles/exporter.rs:2775`
- REQUIREMENT: "canonical export inventory includes workflow node execution records and manifest counts."
- EVIDENCE: `src/backend/handshake_core/src/bundles/schemas.rs:127`, `src/backend/handshake_core/src/bundles/exporter.rs:1333`, `src/backend/handshake_core/src/bundles/exporter.rs:1479`, `src/backend/handshake_core/src/bundles/exporter.rs:1665`, `src/backend/handshake_core/src/bundles/exporter.rs:2372`
- REQUIREMENT: "Clean-room committed surface compiles against the current structured-collaboration contract without widening scope."
- EVIDENCE: `src/backend/handshake_core/src/workflows.rs:4669`, `src/backend/handshake_core/src/workflows.rs:4705`, `src/backend/handshake_core/src/workflows.rs:4734`, `src/backend/handshake_core/src/workflows.rs:4769`, `src/backend/handshake_core/src/workflows.rs:11793`
## EVIDENCE
- COMMAND: `git -C ..\\handshake_main apply --check ..\\gov_runtime\\roles_shared\\WP_COMMUNICATIONS\\WP-1-Workflow-Projection-Correlation-v1\\isolated-six-file.patch`
- EXIT_CODE: 0
- PROOF_LINES: `apply --check` returned no stdout or stderr; patch is applicable to clean `handshake_main`.
- COMMAND: `git -C ..\\handshake_main apply ..\\gov_runtime\\roles_shared\\WP_COMMUNICATIONS\\WP-1-Workflow-Projection-Correlation-v1\\isolated-six-file.patch`
- EXIT_CODE: 0
- PROOF_LINES: patch applied cleanly to `../handshake_main`.
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test storage::tests::workflow_node_execution_persists_inputs_and_outputs ... ok`; `test result: ok. 1 passed; 0 failed`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test bundles::validator::tests::val_bundle_001_reports_missing_files ... ok`; `test result: ok. 1 passed; 0 failed`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test bundles::zip::tests::bundle_determinism_hash_stable ... ok`; `test result: ok. 1 passed; 0 failed`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes ... ok`; `test result: ok. 1 passed; 0 failed`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage ... ok`; `test result: ok. 1 passed; 0 failed`
- COMMAND: `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture`
- EXIT_CODE: 0
- PROOF_LINES: `test bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors ... ok`; `test result: ok. 1 passed; 0 failed`
- COMMAND: `git -C ..\\handshake_main status --short -- src/backend/handshake_core/src/bundles/exporter.rs src/backend/handshake_core/src/bundles/schemas.rs src/backend/handshake_core/src/bundles/templates.rs src/backend/handshake_core/src/bundles/validator.rs src/backend/handshake_core/src/bundles/zip.rs src/backend/handshake_core/src/workflows.rs`
- EXIT_CODE: 0
- PROOF_LINES: `M src/backend/handshake_core/src/bundles/exporter.rs`; `M src/backend/handshake_core/src/bundles/schemas.rs`; `M src/backend/handshake_core/src/bundles/templates.rs`; `M src/backend/handshake_core/src/bundles/validator.rs`; `M src/backend/handshake_core/src/bundles/zip.rs`; `M src/backend/handshake_core/src/workflows.rs`
- COMMAND: `git -C ..\\handshake_main apply -R ..\\gov_runtime\\roles_shared\\WP_COMMUNICATIONS\\WP-1-Workflow-Projection-Correlation-v1\\isolated-six-file.patch`
- EXIT_CODE: 0
- PROOF_LINES: reverse apply returned no stdout or stderr; clean-room patch was fully removed.
- COMMAND: `git -C ..\\handshake_main status --short`
- EXIT_CODE: 0
- PROOF_LINES: `<empty>`
- COMMAND: `just pre-work WP-1-Workflow-Projection-Correlation-v1`
- EXIT_CODE: 1
- PROOF_LINES: `Pre-work validation FAILED`; `Branch-local out-of-scope edits detected before work starts: PRODUCT_OUT_OF_SCOPE: src/backend/handshake_core/src/api/jobs.rs ... src/backend/handshake_core/src/storage/loom.rs`
- COMMAND: `just post-work WP-1-Workflow-Projection-Correlation-v1 --rev 274341181b694e8ae6699b047117d136bbd3f041`
- EXIT_CODE: 1
- PROOF_LINES: `[WP_COMMUNICATION_HEALTH] PASS: Kickoff exchange is complete`; `Post-work validation FAILED (deterministic manifest gate; not tests)`; `Branch-local scope drift detected outside the evaluated diff: PRODUCT_OUT_OF_SCOPE: src/backend/handshake_core/src/api/jobs.rs ... src/backend/handshake_core/src/storage/loom.rs`
- COMMAND: `just validator-packet-complete WP-1-Workflow-Projection-Correlation-v1`
- EXIT_CODE: 0
- PROOF_LINES: `validator-packet-complete: PASS - WP-1-Workflow-Projection-Correlation-v1 has required fields.`
- COMMAND: `just wp-communication-health-check WP-1-Workflow-Projection-Correlation-v1 KICKOFF`
- EXIT_CODE: 0
- PROOF_LINES: `[WP_COMMUNICATION_HEALTH] PASS: Kickoff exchange is complete`; `open_review_items=0`
- COMMAND: `just validator-handoff-check WP-1-Workflow-Projection-Correlation-v1`
- EXIT_CODE: 1
- PROOF_LINES: `[VALIDATOR_HANDOFF_CHECK] FAIL: Committed handoff validation failed`; `pre_work_status=FAIL`; `post_work_status=FAIL`
- COMMAND: `just validator-git-hygiene`
- EXIT_CODE: 0
- PROOF_LINES: `validator-git-hygiene: PASS - .gitignore coverage and artifact checks clean.`

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
  - `Validated (FAIL)` and `Validated (OUTDATED_ONLY)` require `MAIN_CONTAINMENT_STATUS: NOT_REQUIRED`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST include these top fields:
  - `VALIDATION_CONTEXT: OK | CONTEXT_MISMATCH`
  - `GOVERNANCE_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `TEST_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `CODE_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `HEURISTIC_REVIEW_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `SPEC_ALIGNMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `ENVIRONMENT_VERDICT: PASS | FAIL | PARTIAL | BLOCKED | NOT_RUN`
  - `DISPOSITION: NONE | OUTDATED_ONLY`
  - `LEGAL_VERDICT: PASS | FAIL | PENDING`
  - `SPEC_CONFIDENCE: NONE | PARTIAL_DIFF_SCOPED | REVIEWED_DIFF_SCOPED | POST_MERGE_RECHECKED`
- For `PACKET_FORMAT_VERSION >= 2026-03-22`, every appended governed validation report MUST also include these completion-layer fields:
  - `WORKFLOW_VALIDITY: VALID | INVALID | PARTIAL | BLOCKED | NOT_RUN`
  - `SCOPE_VALIDITY: IN_SCOPE | OUT_OF_SCOPE | PARTIAL | BLOCKED | NOT_RUN`
  - `PROOF_COMPLETENESS: PROVEN | NOT_PROVEN | PARTIAL | BLOCKED | NOT_RUN`
  - `INTEGRATION_READINESS: READY | NOT_READY | PARTIAL | BLOCKED | NOT_RUN`
  - `DOMAIN_GOAL_COMPLETION: COMPLETE | INCOMPLETE | PARTIAL | BLOCKED | NOT_RUN`
- For `PACKET_FORMAT_VERSION >= 2026-03-15`, every appended governed validation report MUST also include:
  - `CLAUSES_REVIEWED:`
    - one bullet per in-scope MUST/SHOULD clause reviewed, each with file:line evidence or an explicit proof note
    - when `CLAUSE_CLOSURE_MONITOR_PROFILE=CLAUSE_MONITOR_V1`, reuse the exact clause text from `CLAUSE_CLOSURE_MATRIX`
  - `NOT_PROVEN:`
    - `- NONE` only when nothing remains unproven
    - otherwise list each unresolved clause/gap explicitly
- For `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`, every appended governed validation report MUST also include:
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
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`
  - `NEGATIVE_PROOF:`
    - list at least one spec requirement the validator verified is NOT fully implemented
    - this proves the validator independently read the code rather than trusting coder summaries
    - `- NONE` is illegal; every codebase has at least one gap or partial implementation
    - required for `GOVERNED_VALIDATOR_REPORT_PROFILE=SPLIT_DIFF_SCOPED_RIGOR_V3`
- Rule: do not claim spec correctness with a generic PASS paragraph. `SPEC_ALIGNMENT_VERDICT=PASS` is only valid when the diff-scoped clauses are listed under `CLAUSES_REVIEWED` and `NOT_PROVEN` is exactly `- NONE`.
- Rule: `HEURISTIC_REVIEW_VERDICT=PASS` is only valid when `QUALITY_RISKS` is exactly `- NONE`.
- Rule: `LEGAL_VERDICT=PASS` is only valid when `DIFF_ATTACK_SURFACES`, `INDEPENDENT_CHECKS_RUN`, `COUNTERFACTUAL_CHECKS`, and `SPEC_CLAUSE_MAP` are all present and non-empty, and `SPEC_CLAUSE_MAP` entries include file:line evidence.
- Rule: `Verdict: PASS` is legal only when `VALIDATION_CONTEXT=OK`, `WORKFLOW_VALIDITY=VALID`, `SCOPE_VALIDITY=IN_SCOPE`, `PROOF_COMPLETENESS=PROVEN`, `INTEGRATION_READINESS=READY`, `DOMAIN_GOAL_COMPLETION=COMPLETE`, and `LEGAL_VERDICT=PASS`.
- Rule: if `PROOF_COMPLETENESS` is anything other than `PROVEN`, the top-line `Verdict` MUST NOT be `PASS`; use `NOT_PROVEN`, `FAIL`, `BLOCKED`, or `OUTDATED_ONLY` honestly.
- Rule: `PROOF_COMPLETENESS=PROVEN` is legal only when `NOT_PROVEN` is exactly `- NONE`.
- Rule: `WORKFLOW_VALIDITY=VALID` is legal only when `VALIDATION_CONTEXT=OK` and `GOVERNANCE_VERDICT=PASS`.
- Rule: `LEGAL_VERDICT=PASS` is legal only when `PROOF_COMPLETENESS=PROVEN`.
- Rule: for `VALIDATOR_RISK_TIER=HIGH`, include at least 2 `INDEPENDENT_CHECKS_RUN` items and at least 2 `COUNTERFACTUAL_CHECKS` items.
- Rule: for `VALIDATOR_RISK_TIER=MEDIUM|HIGH`, include at least 1 `BOUNDARY_PROBES` item and at least 1 `NEGATIVE_PATH_CHECKS` item.
- Rule: `NEGATIVE_PROOF` must list at least one spec requirement verified as NOT fully implemented. This is the strongest anti-gaming measure.

VALIDATION REPORT - WP-1-Workflow-Projection-Correlation-v1
Verdict: PASS
VALIDATION_CONTEXT: OK
GOVERNANCE_VERDICT: PASS
TEST_VERDICT: PASS
CODE_REVIEW_VERDICT: PASS
HEURISTIC_REVIEW_VERDICT: PASS
SPEC_ALIGNMENT_VERDICT: PASS
ENVIRONMENT_VERDICT: PASS
DISPOSITION: NONE
LEGAL_VERDICT: PASS
SPEC_CONFIDENCE: POST_MERGE_RECHECKED
WORKFLOW_VALIDITY: VALID
SCOPE_VALIDITY: IN_SCOPE
PROOF_COMPLETENESS: PROVEN
INTEGRATION_READINESS: READY
DOMAIN_GOAL_COMPLETION: COMPLETE
VALIDATOR_RISK_TIER: HIGH

Scope Inputs:
- Task Packet: `.GOV/task_packets/WP-1-Workflow-Projection-Correlation-v1/packet.md`
- Spec: `Handshake_Master_Spec_v02.179.md`
- Reviewed prepare head: `274341181b694e8ae6699b047117d136bbd3f041`
- Contained main commit: `1f1495a1c0801f17e8e99a01fec859962a717722`
- Governed evidence: `../gov_runtime/roles_shared/validator_gates/WP-1-Workflow-Projection-Correlation-v1.json`; `command_id=b5ad15ce-5dd0-4654-94e1-36e721a829bf`

CLAUSES_REVIEWED:
- HSK-WF-001 durable node persistence plus recovery-safe node lineage must become bounded export anchors -> `src/backend/handshake_core/src/storage/tests.rs:3013`; `src/backend/handshake_core/src/bundles/exporter.rs:1463`; `src/backend/handshake_core/src/bundles/exporter.rs:1479`; `src/backend/handshake_core/src/bundles/exporter.rs:2775`; `src/backend/handshake_core/src/bundles/exporter.rs:2831`; `src/backend/handshake_core/src/bundles/schemas.rs:220`; `src/backend/handshake_core/src/workflows.rs:24289`
- AI Job Model runtime identity requires `workflow_run_id` to remain a first-class runtime anchor -> `src/backend/handshake_core/src/workflows.rs:7883`; `src/backend/handshake_core/src/workflows.rs:7892`; `src/backend/handshake_core/src/workflows.rs:4673`; `src/backend/handshake_core/src/workflows.rs:4705`; `src/backend/handshake_core/src/bundles/templates.rs:29`; `src/backend/handshake_core/src/bundles/templates.rs:59`; `src/backend/handshake_core/src/bundles/templates.rs:166`; `src/backend/handshake_core/src/bundles/templates.rs:226`
- Debug Bundle manifest scope and exporter contract must admit workflow-run and node-execution scope kinds -> `src/backend/handshake_core/src/bundles/schemas.rs:6`; `src/backend/handshake_core/src/bundles/schemas.rs:30`; `src/backend/handshake_core/src/bundles/schemas.rs:454`; `src/backend/handshake_core/src/bundles/schemas.rs:485`; `src/backend/handshake_core/src/bundles/exporter.rs:1103`; `src/backend/handshake_core/src/bundles/exporter.rs:1185`; `src/backend/handshake_core/src/bundles/exporter.rs:1213`; `src/backend/handshake_core/src/bundles/exporter.rs:1463`; `src/backend/handshake_core/src/bundles/exporter.rs:2686`; `src/backend/handshake_core/src/bundles/exporter.rs:2775`
- export manifest requirements that already mention `workflow_run_id` must be reconciled with explicit scope law and workflow-node inventory proof -> `src/backend/handshake_core/src/bundles/validator.rs:340`; `src/backend/handshake_core/src/bundles/validator.rs:780`; `src/backend/handshake_core/src/bundles/validator.rs:845`; `src/backend/handshake_core/src/bundles/validator.rs:1120`; `src/backend/handshake_core/src/bundles/zip.rs:75`

NOT_PROVEN:
- NONE

MAIN_BODY_GAPS:
- NONE

QUALITY_RISKS:
- NONE

DIFF_ATTACK_SURFACES:
- Manifest scope enum expansion could have drifted between request parsing, manifest serialization, exporter selection, validator rules, and human-facing templates.
- Workflow-node bundle exports could have leaked unrelated jobs/events or omitted `workflow_node_executions.jsonl` while still passing a narrow happy-path export.
- Workflow correlation anchors could have appeared in manifests but failed to persist into runtime summary artifacts or exportable inventory.

INDEPENDENT_CHECKS_RUN:
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml storage::tests::workflow_node_execution_persists_inputs_and_outputs -- --exact --nocapture` from `../handshake_main` => `ok`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::validator::tests::val_bundle_001_reports_missing_files -- --exact --nocapture` from `../handshake_main` => `ok`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::zip::tests::bundle_determinism_hash_stable -- --exact --nocapture` from `../handshake_main` => `ok`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_run_scope_exports_only_bound_jobs_and_nodes -- --exact --nocapture` from `../handshake_main` => `ok`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::workflow_node_execution_scope_exports_single_node_lineage -- --exact --nocapture` from `../handshake_main` => `ok`
- `cargo test --manifest-path src/backend/handshake_core/Cargo.toml bundles::exporter::tests::list_exportable_includes_workflow_correlation_anchors -- --exact --nocapture` from `../handshake_main` => `ok`
- `just validator-handoff-check WP-1-Workflow-Projection-Correlation-v1` from `../wtc-projection-correlation-v1` with parked out-of-scope drift => `[VALIDATOR_HANDOFF_CHECK] PASS`

COUNTERFACTUAL_CHECKS:
- If `BundleScope::WorkflowNodeExecution` parsing at `src/backend/handshake_core/src/workflows.rs:7892-7913` were removed or loosened, node-scoped export requests would lose the required `workflow_run_id` / `workflow_node_execution_id` pair and the bounded node-lineage contract would break.
- If `src/backend/handshake_core/src/bundles/exporter.rs:1463-1479` stopped writing `workflow_node_executions.jsonl` for workflow-correlated scopes, the validator rules at `src/backend/handshake_core/src/bundles/validator.rs:845-853` and the workflow-scope exporter tests would fail instead of proving closure.

BOUNDARY_PROBES:
- Request-to-exporter probe: `src/backend/handshake_core/src/workflows.rs:7883-7913` feeds explicit workflow scope IDs into bundle requests, and `src/backend/handshake_core/src/bundles/exporter.rs:1185-1236` preserves those IDs while selecting workflow-bounded jobs, events, and node records.
- Exporter-to-validator probe: `src/backend/handshake_core/src/bundles/exporter.rs:1463-1479` and `src/backend/handshake_core/src/bundles/exporter.rs:2764-2863` match the validator invariants in `src/backend/handshake_core/src/bundles/validator.rs:780-853` for workflow-node count, scoped `workflow_run_id`, and required node inventory presence.

NEGATIVE_PATH_CHECKS:
- `src/backend/handshake_core/src/bundles/validator.rs:340-351` rejects `scope.kind=workflow_node_execution` when `scope.workflow_node_execution_id` is missing.
- `src/backend/handshake_core/src/bundles/validator.rs:845-853` and `src/backend/handshake_core/src/bundles/validator.rs:1120-1136` reject workflow-correlated bundles that omit `workflow_node_executions.jsonl`.

INDEPENDENT_FINDINGS:
- Current local `main` commit `1f1495a1c0801f17e8e99a01fec859962a717722` contains the bounded workflow-run and workflow-node bundle scope implementation; this is no longer stranded in the PREPARE worktree.
- The guarded handoff gate is green after parking unrelated coder drift, so the remaining failure surface is governance closeout execution, not missing product behavior.
- `workflow_run_id` and `workflow_node_execution_id` are carried consistently across manifest scope, templates, validator rules, exporter inventory, and runtime workflow scope parsing.

RESIDUAL_UNCERTAINTY:
- The governed ACP final-lane helper still self-blocks if `integration-validator-closeout-check` or `integration-validator-closeout-sync` is invoked from inside an active `SEND_PROMPT`, so final truth sync still depends on running the helper after the validator turn settles.
- The PREPARE worktree still contains unrelated parked coder drift under a named stash; that drift is outside signed WP scope and was intentionally excluded from containment proof.

SPEC_CLAUSE_MAP:
- `HSK-WF-001 durable node persistence plus recovery-safe node lineage must become bounded export anchors.` -> `src/backend/handshake_core/src/storage/tests.rs:3013`; `src/backend/handshake_core/src/bundles/schemas.rs:220-229`; `src/backend/handshake_core/src/bundles/exporter.rs:1463-1479`; `src/backend/handshake_core/src/bundles/exporter.rs:2775-2863`
- `AI Job Model runtime identity requires workflow_run_id to remain a first-class runtime anchor.` -> `src/backend/handshake_core/src/workflows.rs:7883-7913`; `src/backend/handshake_core/src/workflows.rs:4673-4705`; `src/backend/handshake_core/src/bundles/templates.rs:29-35`; `src/backend/handshake_core/src/bundles/templates.rs:166-175`; `src/backend/handshake_core/src/bundles/templates.rs:226-227`
- `Debug Bundle manifest scope and exporter contract must admit workflow-run and node-execution scope kinds.` -> `src/backend/handshake_core/src/bundles/schemas.rs:6-15`; `src/backend/handshake_core/src/bundles/schemas.rs:30-35`; `src/backend/handshake_core/src/bundles/schemas.rs:454-458`; `src/backend/handshake_core/src/bundles/schemas.rs:485-487`; `src/backend/handshake_core/src/bundles/exporter.rs:1103-1108`; `src/backend/handshake_core/src/bundles/exporter.rs:1185-1236`; `src/backend/handshake_core/src/bundles/exporter.rs:1463-1479`
- `Export manifest requirements that already mention workflow_run_id must be reconciled with explicit scope law and workflow-node inventory proof.` -> `src/backend/handshake_core/src/bundles/validator.rs:340-351`; `src/backend/handshake_core/src/bundles/validator.rs:780-853`; `src/backend/handshake_core/src/bundles/validator.rs:1120-1136`; `src/backend/handshake_core/src/bundles/zip.rs:75`

NEGATIVE_PROOF:
- `src/backend/handshake_core/src/bundles/exporter.rs:613-614` records `input_sha256` and `output_sha256` for workflow node executions, but the bundle validator currently proves only scope membership and inventory integrity at `src/backend/handshake_core/src/bundles/validator.rs:780-853`; it does not yet validate those node payload hashes against exported artifacts. The workflow-correlation scope is closed, while the broader node-payload integrity surface remains partial.

REASON FOR PASS:
- The signed workflow-correlation scope is contained in local `main` at `1f1495a1c0801f17e8e99a01fec859962a717722`, the committed-handoff gate is durable PASS, the bounded workflow scope tests all pass on `main`, and the remaining failure mode is a governed closeout execution bug rather than an unresolved product or spec gap.
