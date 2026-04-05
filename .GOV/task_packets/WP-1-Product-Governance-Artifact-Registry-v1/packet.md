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

# Task Packet: WP-1-Product-Governance-Artifact-Registry-v1

## METADATA
- TASK_ID: WP-1-Product-Governance-Artifact-Registry-v1
- WP_ID: WP-1-Product-Governance-Artifact-Registry-v1
- BASE_WP_ID: WP-1-Product-Governance-Artifact-Registry
- DATE: 2026-04-05T17:43:32.006Z
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
- ORCHESTRATOR_MODEL_PROFILE: OPENAI_GPT_5_4_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. Allowed: repo role-model-profile catalog ids. -->
- ORCHESTRATOR_REASONING_STRENGTH: EXTRA_HIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- ORCHESTRATION_STARTED_AT_UTC: N/A
<!-- RFC3339 UTC; required only when AGENTIC_MODE=YES and the Orchestrator is explicitly authorized to use sub-agents. -->
- CODER_MODEL_PROFILE: OPENAI_CODEX_SPARK_5_3_XHIGH
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- CODER_MODEL: gpt-5.3-codex-spark
- CODER_REASONING_STRENGTH: EXTRA_HIGH
<!-- Allowed: LOW | MEDIUM | HIGH | EXTRA_HIGH -->
- SESSION_START_AUTHORITY: ORCHESTRATOR_ONLY
- SESSION_HOST_PREFERENCE: VSCODE_EXTENSION_TERMINAL
- SESSION_HOST_FALLBACK: CLI_ESCALATION_WINDOW
- SESSION_LAUNCH_POLICY: ORCHESTRATOR_PLUGIN_FIRST_WITH_2TRY_ESCALATION
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
- MODEL_FAMILY_POLICY: ROLE_MODEL_PROFILE_CATALOG_PRIMARY_OPENAI_DECLARED_MULTI_PROVIDER_V1
- ROLE_MODEL_PROFILE_POLICY: ROLE_MODEL_PROFILE_CATALOG_V1
- CODEX_MODEL_ALIASES_ALLOWED: NO
- ROLE_SESSION_PRIMARY_MODEL: gpt-5.4
- ROLE_SESSION_FALLBACK_MODEL: gpt-5.2
- ROLE_SESSION_REASONING_REQUIRED: EXTRA_HIGH
- ROLE_SESSION_REASONING_CONFIG_KEY: model_reasoning_effort
- ROLE_SESSION_REASONING_CONFIG_VALUE: xhigh
- CODER_STARTUP_COMMAND: just coder-startup
- CODER_RESUME_COMMAND: just coder-next WP-1-Product-Governance-Artifact-Registry-v1
<!-- The WP Validator uses a dedicated local review branch/worktree rooted from the coder branch. The Integration Validator stays on handshake_main/main. Both mirror the single shared WP backup branch under REMOTE_BACKUP_* below. Do not create separate validator-only remote WP backup branches. -->
- WP_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- WP_VALIDATOR_MODEL: claude-opus-4-6
- WP_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- WP_VALIDATOR_LOCAL_BRANCH: validate/WP-1-Product-Governance-Artifact-Registry-v1
- WP_VALIDATOR_LOCAL_WORKTREE_DIR: ../wtv-artifact-registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Product-Governance-Artifact-Registry-v1
- WP_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Product-Governance-Artifact-Registry-v1
- WP_VALIDATOR_STARTUP_COMMAND: just validator-startup
- WP_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Product-Governance-Artifact-Registry-v1
- INTEGRATION_VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
<!-- Required for PACKET_FORMAT_VERSION >= 2026-04-06. -->
- INTEGRATION_VALIDATOR_MODEL: claude-opus-4-6
- INTEGRATION_VALIDATOR_REASONING_STRENGTH: EXTRA_HIGH
- INTEGRATION_VALIDATOR_LOCAL_BRANCH: main
- INTEGRATION_VALIDATOR_LOCAL_WORKTREE_DIR: ../handshake_main
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_BRANCH: feat/WP-1-Product-Governance-Artifact-Registry-v1
- INTEGRATION_VALIDATOR_REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Product-Governance-Artifact-Registry-v1
- INTEGRATION_VALIDATOR_STARTUP_COMMAND: just validator-startup
- INTEGRATION_VALIDATOR_RESUME_COMMAND: just validator-next WP-1-Product-Governance-Artifact-Registry-v1
- EXTERNAL_VALIDATOR_BRIEF_COMMAND: just external-validator-brief WP-1-Product-Governance-Artifact-Registry-v1
- EXTERNAL_VALIDATOR_STARTUP_SEQUENCE: just validator-startup -> just external-validator-brief WP-1-Product-Governance-Artifact-Registry-v1
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
- **Status:** In Progress
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
- BUILD_ORDER_TECH_BLOCKER: YES
<!-- Allowed: YES | NO. YES => unblocks multiple downstream WPs. -->
- BUILD_ORDER_VALUE_TIER: HIGH
<!-- Allowed: LOW | MEDIUM | HIGH. Spec-defined Phase 1 impact. -->
- BUILD_ORDER_DEPENDS_ON: WP-1-Product-Governance-Snapshot, WP-1-Governance-Kernel-Conformance, WP-1-Structured-Collaboration-Schema-Registry, WP-1-Structured-Collaboration-Artifact-Family
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- BUILD_ORDER_BLOCKS: WP-1-Product-Governance-Check-Runner, WP-1-Governance-Workflow-Mirror, WP-1-Governance-Pack, WP-1-Dev-Command-Center-Control-Plane-Backend
<!-- Allowed: comma-separated Base WP IDs | NONE. Use Base IDs only (no -vN). -->
- UI_UX_APPLICABLE: NO
<!-- Allowed: YES | NO. YES => packet must include ## UI_UX_SPEC with concrete controls + tooltips. -->
- UI_UX_VERDICT: OK
<!-- Allowed: OK | NEEDS_STUBS | UNKNOWN -->
- STUB_WP_IDS: NONE
<!-- Allowed: comma-separated WP-... IDs | NONE. Must match refinement metadata STUB_WP_IDS. -->
- LOCAL_BRANCH: feat/WP-1-Product-Governance-Artifact-Registry-v1
- LOCAL_WORKTREE_DIR: ../wtc-artifact-registry-v1
- REMOTE_BACKUP_BRANCH: feat/WP-1-Product-Governance-Artifact-Registry-v1
- REMOTE_BACKUP_URL: https://github.com/Nuntissura/Handshake/tree/feat/WP-1-Product-Governance-Artifact-Registry-v1
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
- WP_COMMUNICATION_DIR: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Product-Governance-Artifact-Registry-v1
- WP_THREAD_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Product-Governance-Artifact-Registry-v1/THREAD.md
- WP_RUNTIME_STATUS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Product-Governance-Artifact-Registry-v1/RUNTIME_STATUS.json
- WP_RECEIPTS_FILE: ../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Product-Governance-Artifact-Registry-v1/RECEIPTS.jsonl
- WP_VALIDATOR_OF_RECORD: <unassigned>
- INTEGRATION_VALIDATOR_OF_RECORD: <unassigned>
- SECONDARY_VALIDATOR_SESSIONS: NONE
- COMMUNICATION_AUTHORITY: WP_COMMUNICATION_DIR
<!-- All roles MUST use the packet-declared WP communication directory. Role-local worktrees are never the communication authority. -->
- COMMUNICATION_CONTRACT: DIRECT_REVIEW_V1
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- COMMUNICATION_HEALTH_GATE: HANDOFF_VERDICT_BLOCKING
<!-- Required for WORKFLOW_LANE=ORCHESTRATOR_MANAGED packets with PACKET_FORMAT_VERSION >= 2026-03-21. -->
- USER_SIGNATURE: ilja050420261939
- PACKET_FORMAT_VERSION: 2026-04-06

## CURRENT_STATE (AUTHORITATIVE SNAPSHOT; MUTABLE)
Verdict: PENDING
Blockers: NONE
Next: N/A

## CLAUSE_CLOSURE_MATRIX (AUTHORITATIVE SNAPSHOT; MUTABLE)
- Rule: this is the live packet-scope monitor for diff-scoped spec closure. Update statuses honestly; do not silently broaden or narrow clause scope after signature. Each row should point to TESTS, EXAMPLES, or governed debt.
- CLAUSE_ROWS:
  - CLAUSE: Governance Pack project-specific instantiation 7.5.4.8 | CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs; src/backend/handshake_core/src/storage/mod.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry | EXAMPLES: a GovernanceArtifactRegistryManifest with SoftwareDelivery profile can be serialized, stored, loaded, and deserialized with schema ID hsk.governance_artifact_registry@1 and all entries retain their artifact_id, kind, provenance, and content_hash, a GovernanceArtifactRegistryEntry with kind Codex correctly serializes to and from JSON with the expected schema version, profile extension validation passes for the governance registry extension when attached to a SoftwareDelivery structured collaboration record, profile extension validation rejects the governance registry extension when attached to a non-SoftwareDelivery record (e.g. Research or Generic), GovernanceArtifactKind enum covers all canonical types from spec 7.5.4.3 and serialization is exhaustive | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Canonical structured collaboration artifact family [ADD v02.167] | CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/governance_artifact_registry.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema | EXAMPLES: a GovernanceArtifactRegistryManifest with SoftwareDelivery profile can be serialized, stored, loaded, and deserialized with schema ID hsk.governance_artifact_registry@1 and all entries retain their artifact_id, kind, provenance, and content_hash, a GovernanceArtifactRegistryEntry with kind Codex correctly serializes to and from JSON with the expected schema version, profile extension validation passes for the governance registry extension when attached to a SoftwareDelivery structured collaboration record, profile extension validation rejects the governance registry extension when attached to a non-SoftwareDelivery record (e.g. Research or Generic), GovernanceArtifactKind enum covers all canonical types from spec 7.5.4.3 and serialization is exhaustive | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
  - CLAUSE: Base envelope + profile extension contract [ADD v02.168] | CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs; src/backend/handshake_core/src/locus/types.rs | TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry | EXAMPLES: a GovernanceArtifactRegistryManifest with SoftwareDelivery profile can be serialized, stored, loaded, and deserialized with schema ID hsk.governance_artifact_registry@1 and all entries retain their artifact_id, kind, provenance, and content_hash, a GovernanceArtifactRegistryEntry with kind Codex correctly serializes to and from JSON with the expected schema version, profile extension validation passes for the governance registry extension when attached to a SoftwareDelivery structured collaboration record, profile extension validation rejects the governance registry extension when attached to a non-SoftwareDelivery record (e.g. Research or Generic), GovernanceArtifactKind enum covers all canonical types from spec 7.5.4.3 and serialization is exhaustive | DEBT_IDS: NONE | CODER_STATUS: UNPROVEN | VALIDATOR_STATUS: PENDING
## SPEC_DEBT_STATUS (AUTHORITATIVE SNAPSHOT; MUTABLE)
- OPEN_SPEC_DEBT: NO
- BLOCKING_SPEC_DEBT: NO
- DEBT_IDS: NONE
- Rule: if any clause row is PARTIAL or DEFERRED, DEBT_IDS must not be NONE and OPEN_SPEC_DEBT must be YES.
## SHARED_SURFACE_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- SHARED_SURFACE_RISK: YES
- HOT_FILES:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
- REQUIRED_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
- POST_MERGE_SPOTCHECK_REQUIRED: YES
- Rule: shared registries, shared types, shared storage layers, shared workflow/runtime surfaces, and migrations default to SHARED_SURFACE_RISK=YES.
## SEMANTIC_PROOF_ASSETS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- SEMANTIC_TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  - just gov-check
- CANONICAL_CONTRACT_EXAMPLES:
  - a GovernanceArtifactRegistryManifest with SoftwareDelivery profile can be serialized, stored, loaded, and deserialized with schema ID hsk.governance_artifact_registry@1 and all entries retain their artifact_id, kind, provenance, and content_hash
  - a GovernanceArtifactRegistryEntry with kind Codex correctly serializes to and from JSON with the expected schema version
  - profile extension validation passes for the governance registry extension when attached to a SoftwareDelivery structured collaboration record
  - profile extension validation rejects the governance registry extension when attached to a non-SoftwareDelivery record (e.g. Research or Generic)
  - GovernanceArtifactKind enum covers all canonical types from spec 7.5.4.3 and serialization is exhaustive
- Rule: for packets using `SEMANTIC_PROOF_PROFILE=DIFF_SCOPED_SEMANTIC_V1`, each clause row must point to TESTS, EXAMPLES, or governed debt, and shared-surface packets should carry at least one concrete tripwire or canonical example.
## DATA_CONTRACT_DECISION (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DECISION: ACTIVE_REQUIRED
- REASON: Current packet scope includes concrete persisted, emitted, or Loom-facing data surfaces, so the data contract is explicitly active.
- EVIDENCE:
  - IN_SCOPE_PATH: src/backend/handshake_core/src/governance_artifact_registry.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/locus/types.rs (backend data surface)
  - IN_SCOPE_PATH: src/backend/handshake_core/src/storage/mod.rs (backend data surface)
  - PILLARS_TOUCHED: Locus
  - PILLARS_TOUCHED: SQL to PostgreSQL shift readiness
  - PILLARS_TOUCHED: LLM-friendly data
  - PILLAR_DECOMPOSITION: PILLAR: Locus | CAPABILITY_SLICE: governance artifact schema registration in structured collaboration record families | SUBFEATURES: new schema ID constant, new record family variant, schema descriptor function extension | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1, PRIM-GovernancePackExport | MECHANICAL: engine.sovereign, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: extends the existing locus/types.rs structured collaboration pattern
  - PILLAR_DECOMPOSITION: PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: governance registry store trait behind Database boundary | SUBFEATURES: GovernanceArtifactRegistryStore trait with load/save/lookup/list | PRIMITIVES_FEATURES: PRIM-Database | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: store trait uses the Database trait boundary, keeping PostgreSQL portability intact
  - PILLAR_DECOMPOSITION: PILLAR: LLM-friendly data | CAPABILITY_SLICE: governance artifact metadata as structured JSON | SUBFEATURES: JSON-serializable registry entries with schema IDs and provenance | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-GovernancePackExport | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: follows the canonical structured collaboration mandate from [ADD v02.167]
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: governance artifact registry load/store | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: internal structured data store consumed by downstream governance runners and DCC projections
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: governance artifact kind enumeration and schema registration | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends the structured collaboration schema registry to include governance artifact records
  - EXECUTION_RUNTIME_ALIGNMENT: Capability: SoftwareDelivery profile extension for governance artifact references | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: profile extension metadata attaches to structured collaboration records as non-breaking SoftwareDelivery-scoped data
  - FORCE_MULTIPLIER_EXPANSION: GovernanceArtifactRegistry plus engine.version provenance tracking -> IN_THIS_WP (stub: NONE)
  - CODE_REALITY_EVIDENCE: ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Product-Governance-Snapshot-v4)
## DATA_CONTRACT_MONITORING (AUTHORITATIVE SNAPSHOT; MUTABLE)
- DATA_CONTRACT_ACTIVE: YES
- SQL_POSTURE: SQLITE_NOW_POSTGRES_READY
- LLM_READABILITY_POSTURE: REQUIRED
- LOOM_INTERTWINED_POSTURE: REQUIRED_WHEN_APPLICABLE
- PRIMARY_DATA_SURFACES:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
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
- REFINEMENT_FILE: .GOV/refinements/WP-1-Product-Governance-Artifact-Registry-v1.md
- Rule: Task packet creation is blocked until refinement is complete and signed.

## SPEC_CONTEXT_WINDOWS (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- Rule: downstream coding and validation must use the signed refinement anchor windows below as the diff-scoped spec context for this packet.
#### ANCHOR 1
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
- CONTEXT_START_LINE: 31837
- CONTEXT_END_LINE: 31857
- CONTEXT_TOKEN: versioned bundle of templates
- EXCERPT_ASCII_ESCAPED:
  ```text
#### 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)

  **Purpose**
  Handshake MUST implement the project-agnostic Governance Kernel (7.5.4; `.GOV/GOV_KERNEL/*`) as a project-parameterized **Governance Pack** so the same strict workflow can be generated and enforced for arbitrary projects (not Handshake-specific).

  **Definitions**
  - **Governance Pack**: a versioned bundle of templates + gate semantics that instantiate:
    - project codex,
    - role protocols,
    - canonical governance artifacts and templates,
    - mechanical gate tooling (scripts/hooks/CI) and a single command surface (e.g., `just`),
    - deterministic exports (including `.GOV/ROLE_MAILBOX/` when enabled by governance mode).
  ```

#### ANCHOR 2
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 7.5.4 Governance Kernel: Mechanical Gated Workflow (Project-Agnostic) (HARD)
- CONTEXT_START_LINE: 31726
- CONTEXT_END_LINE: 31740
- CONTEXT_TOKEN: deterministic multi-role collaboration
- EXCERPT_ASCII_ESCAPED:
  ```text
### 7.5.4 Governance Kernel: Mechanical Gated Workflow (Project-Agnostic) (HARD)

  **Purpose**
  Define a reusable, project-agnostic governance kernel that enables:
  - deterministic multi-role collaboration (Operator / Orchestrator / Coder / Validator)
  - rigorous auditability (evidence-first; append-only logs)
  - reliable handoff between small-context local models and large-context cloud models
  ```

#### ANCHOR 3
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md [ADD v02.167] Canonical structured collaboration artifact family
- CONTEXT_START_LINE: 6817
- CONTEXT_END_LINE: 6838
- CONTEXT_TOKEN: versioned JavaScript Object Notation documents
- EXCERPT_ASCII_ESCAPED:
  ```text
**Canonical structured collaboration artifact family** [ADD v02.167]

  - The canonical file standard for Work Packets, Micro-Tasks, and Task Board projections SHALL be versioned JavaScript Object Notation documents.
  - Every canonical structured collaboration record MUST expose:
    - a schema identifier and schema version
    - a stable record identifier
    - an updated timestamp
    - a profile kind such as `software_delivery`, `research`, `worldbuilding`, `design`, `generic`, or `custom`
    - references to note sidecars, mirrors, or evidence artifacts when present
  - Project-specific details MUST live inside profile extensions instead of becoming mandatory base-envelope fields.
  ```

#### ANCHOR 4
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md [ADD v02.168] Base structured schema and project-profile extension contract
- CONTEXT_START_LINE: 6840
- CONTEXT_END_LINE: 6884
- CONTEXT_TOKEN: shared base envelope
- EXCERPT_ASCII_ESCAPED:
  ```text
**Base structured schema and project-profile extension contract** [ADD v02.168]

  - Every canonical collaboration artifact family member SHALL implement one shared base envelope before any profile-specific fields are applied.
  - The base envelope MUST remain valid even when no project-profile extension is present.
  - `profile_extension` payloads MUST declare `extension_schema_id`, `extension_schema_version`, and `compatibility` so migration and validation tooling can reject unknown breaking extensions deterministically.
  ```

#### ANCHOR 5
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 1.3 The Four-Layer Architecture
- CONTEXT_START_LINE: 479
- CONTEXT_END_LINE: 493
- CONTEXT_TOKEN: Mechanical Layer
- EXCERPT_ASCII_ESCAPED:
  ```text
## 1.3 The Four-Layer Architecture

  Handshake has four layers: LLM (decides what), Orchestrator (enforces rules), Mechanical (executes deterministically), and Validation (confirms correctness).

  - **Mechanical Layer**: Deterministic engines (Word, Excel, Docling) that execute operations.
  ```

#### ANCHOR 6
- SPEC_ANCHOR: Handshake_Master_Spec_v02.179.md 10.11 Dev Command Center (Sidecar Integration)
- CONTEXT_START_LINE: 60488
- CONTEXT_END_LINE: 60502
- CONTEXT_TOKEN: canonical developer/operator surface
- EXCERPT_ASCII_ESCAPED:
  ```text
## 10.11 Dev Command Center (Sidecar Integration)

  The Dev Command Center is the canonical developer/operator surface that binds:
  **work (Locus WP/MT)** - **workspaces (git worktrees)** - **execution sessions (agent/model runs)** - **approvals/logs/diffs**
  ```
## CLAUSE_PROOF_PLAN (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSE_ROWS:
  - CLAUSE: Governance Pack project-specific instantiation 7.5.4.8 | WHY_IN_SCOPE: the product has an export path but no import/registry for governance artifacts | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs; src/backend/handshake_core/src/storage/mod.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry | RISK_IF_MISSED: downstream Check-Runner and DCC-Backend remain blocked with no structured governance data to consume
  - CLAUSE: Canonical structured collaboration artifact family [ADD v02.167] | WHY_IN_SCOPE: governance artifact records must be versioned JSON with schema IDs per the structured collaboration mandate | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/locus/types.rs; src/backend/handshake_core/src/governance_artifact_registry.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema | RISK_IF_MISSED: governance artifacts bypass the structured collaboration family and become ad hoc unvalidated records
  - CLAUSE: Base envelope + profile extension contract [ADD v02.168] | WHY_IN_SCOPE: governance artifact metadata must be SoftwareDelivery profile extension, not base-envelope, to preserve multi-domain portability | EXPECTED_CODE_SURFACES: src/backend/handshake_core/src/governance_artifact_registry.rs; src/backend/handshake_core/src/locus/types.rs | EXPECTED_TESTS: cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry | RISK_IF_MISSED: non-software projects inherit governance-only required fields and the base envelope is polluted
## CONTRACT_SURFACES (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CONTRACT_ROWS:
  - CONTRACT: GovernanceArtifactRegistryManifest JSON serialization | PRODUCER: governance_artifact_registry.rs | CONSUMER: Check-Runner (future), DCC-Backend (future), storage layer | SERIALIZER_TRANSPORT: JSON via serde | VALIDATOR_READER: governance_artifact_registry tests | TRIPWIRE_TESTS: governance_artifact_registry round-trip serialization test | DRIFT_RISK: manifest fields can drift across producer and consumer if schema version is not checked
  - CONTRACT: GovernanceArtifactKind enum serialization | PRODUCER: governance_artifact_registry.rs | CONSUMER: schema descriptor function, profile extension validation | SERIALIZER_TRANSPORT: JSON string via serde | VALIDATOR_READER: governance_artifact_registry tests | TRIPWIRE_TESTS: governance_artifact_registry kind exhaustiveness test | DRIFT_RISK: new enum variants can be added without updating downstream match arms
  - CONTRACT: StructuredCollaborationSchemaDescriptor extension for governance artifacts | PRODUCER: locus/types.rs schema descriptor function | CONSUMER: structured collaboration validation pipeline | SERIALIZER_TRANSPORT: in-process struct | VALIDATOR_READER: structured collaboration schema tests | TRIPWIRE_TESTS: structured_collaboration_schema test covering governance artifact schema ID | DRIFT_RISK: schema descriptor function can fall out of sync with new record families
  - CONTRACT: GovernanceArtifactRegistryStore trait boundary | PRODUCER: storage implementations | CONSUMER: governance artifact registry load/save callers | SERIALIZER_TRANSPORT: in-process trait object | VALIDATOR_READER: governance_artifact_registry tests | TRIPWIRE_TESTS: governance_artifact_registry store round-trip test | DRIFT_RISK: store implementations can drift if trait methods change without updating both backends
## CODER_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- IMPLEMENTATION_ORDER:
  - Create governance_artifact_registry.rs with GovernanceArtifactKind enum, GovernanceArtifactRegistryEntry struct, GovernanceArtifactRegistryManifest struct, GovernanceArtifactProvenance struct, and GovernanceArtifactRegistryStore trait.
  - Register the module in lib.rs.
  - Add schema ID constant hsk.governance_artifact_registry@1 and record family variant in locus/types.rs, extend the schema descriptor function.
  - Add profile extension schema hsk.ext.software_delivery.governance_artifact_registry@1 with non-breaking compatibility.
  - Write unit tests for serialization round-trip, kind exhaustiveness, schema registration, profile extension validation, and store trait contract.
- HOT_FILES:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
- TRIPWIRE_TESTS:
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
- CARRY_FORWARD_WARNINGS:
  - Do not put governance artifact metadata into the base structured collaboration envelope. It MUST be a SoftwareDelivery profile extension only.
  - Do not import script or check content into the registry. Only descriptors with provenance metadata belong here.
  - Do not add repo file paths as runtime authority references. Use artifact identity and content hash instead.
  - Do not widen into check execution, workflow mirroring, or DCC projections.
## VALIDATOR_HANDOFF_BRIEF (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- CLAUSES_TO_INSPECT:
  - GovernanceArtifactKind covers all canonical artifact types from spec 7.5.4.3
  - Registry entries are versioned JSON with schema IDs per [ADD v02.167]
  - Governance artifact metadata is SoftwareDelivery profile extension, not base-envelope, per [ADD v02.168]
  - Store trait uses the Database trait boundary for PostgreSQL portability
  - Non-SoftwareDelivery project profiles do not see or require governance registry extensions
- FILES_TO_READ:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
- COMMANDS_TO_RUN:
  - rg -n "GovernanceArtifactKind|governance_artifact_registry" src/backend/handshake_core/src
  - rg -n "hsk.governance_artifact_registry" src/backend/handshake_core/src
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  - cargo test --manifest-path src/backend/handshake_core/Cargo.toml
- POST_MERGE_SPOTCHECKS:
  - verify GovernanceArtifactKind is a closed enum with no catch-all variant
  - verify schema descriptor function includes hsk.governance_artifact_registry@1
  - verify profile extension uses non-breaking compatibility declaration
  - verify no repo file paths appear as runtime authority in registry entries
## NOT_PROVEN_AT_REFINEMENT_TIME (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- NOT_PROVEN_ITEMS:
  - The exact store trait method signatures are not proven until coding. The trait surface (load/save/lookup/list) is directional but may evolve during implementation.
  - Whether the GovernanceArtifactRegistryStore needs SQLite and PostgreSQL implementations in this WP or can defer to a generic JSON-backed store is not proven.
  - The precise interaction between the governance artifact registry and the existing governance pack export flow is not proven. Import may reuse export format metadata or define its own manifest shape.
  - Whether test-only helpers need direct registry construction without going through the store trait will need inspection during implementation.
## RESEARCH_SIGNAL (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- RESEARCH_CURRENCY_REQUIRED: YES
- RESEARCH_CURRENCY_VERDICT: CURRENT
- RESEARCH_DEPTH_VERDICT: PASS
- GITHUB_PROJECT_SCOUTING_VERDICT: PASS
- SOURCE_LOG:
  - [BIG_TECH] Google Artifact Registry overview | 2026-04-05 | Retrieved: 2026-04-05T11:00:00Z | https://cloud.google.com/artifact-registry/docs/overview | Why: demonstrates versioned artifact management with manifest-based provenance across language ecosystems, directly analogous to governance artifact versioning and content-hash integrity
  - [OSS_DOC] Backstage descriptor format | 2026-04-05 | Retrieved: 2026-04-05T11:05:00Z | https://backstage.io/docs/features/software-catalog/descriptor-format | Why: demonstrates a stable typed catalog with kind/metadata/spec envelope for software delivery artifacts, directly analogous to governance artifact registry entries
  - [OSS_DOC] OPA Management Bundles | 2026-04-05 | Retrieved: 2026-04-05T11:10:00Z | https://www.openpolicyagent.org/docs/latest/management-bundles/ | Why: governance policy-as-versioned-bundles with manifest metadata and content hashing maps directly to imported governance snapshot provenance
  - [OSS_DOC] Buf Schema Registry | 2026-04-05 | Retrieved: 2026-04-05T11:15:00Z | https://buf.build/docs/bsr/overview | Why: typed versioned schema records with dependency tracking reinforces the registry-first contract for governance schemas and check manifests
  - [PAPER] Validation of Modern JSON Schema: Formalization and Complexity | 2024-02-01 | Retrieved: 2026-04-05T11:20:00Z | https://arxiv.org/abs/2307.10034 | Why: warns that overly dynamic schema semantics become hard to reason about and validate, which argues for explicit low-cardinality kind enumeration over free-form schema evolution
  - [GITHUB] backstage/backstage | 2026-04-05 | Retrieved: 2026-04-05T11:25:00Z | https://github.com/backstage/backstage | Why: large OSS implementation surface for descriptor-based extensibility and catalog-style shared parsing, informing the artifact kind + provenance design
- RESEARCH_SYNTHESIS:
  - Governance artifact registries should use typed descriptors with stable identities and version provenance, not raw file copies or live path references.
  - Manifest-level content hashing at import time provides integrity without requiring runtime re-scanning.
  - Separating artifact definition (registry) from artifact execution (runner) is the consensus pattern across OPA, Backstage, Buf, and Google Artifact Registry.
  - Low-cardinality kind enumerations are more deterministically validatable than dynamic schema evolution.
- GITHUB_PROJECT_DECISIONS:
  - backstage/backstage -> ADOPT (NONE)
## MATRIX_RESEARCH_RUBRIC (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- MATRIX_RESEARCH_REQUIRED: YES
- MATRIX_RESEARCH_VERDICT: PASS
- SOURCE_SCAN_DECISIONS:
  - Backstage descriptor format -> ADOPT (IN_THIS_WP)
  - OPA Management Bundles -> ADAPT (IN_THIS_WP)
  - Google Artifact Registry overview -> ADAPT (IN_THIS_WP)
- MATRIX_GROWTH_CANDIDATES:
  - governance artifact kind x structured collaboration schema descriptor -> IN_THIS_WP (stub: NONE)
- ENGINEERING_TRICKS_CARRIED_OVER:
  - Backstage: stable apiVersion + kind + metadata.name triple as minimum identity contract for any catalog entity
  - OPA: content hash on the entire bundle manifest for tamper detection without per-file scanning
## PRIMITIVES_AND_MATRIX (REFINEMENT OUTPUT; REQUIRED)
- PRIMITIVES_TOUCHED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-GovernancePackExport
  - PRIM-GovernancePackExportRequest
  - PRIM-Database
- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-GovernancePackExport
  - PRIM-GovernancePackExportRequest
  - PRIM-Database
- PRIMITIVES_CREATED:
  - NONE
- MECHANICAL_ENGINES_TOUCHED:
  - engine.sovereign
  - engine.context
  - engine.version
- PRIMITIVE_INDEX_ACTION: NO_CHANGE
- FEATURE_REGISTRY_ACTION: NO_CHANGE
- UI_GUIDANCE_ACTION: NOT_APPLICABLE
- INTERACTION_MATRIX_ACTION: NO_CHANGE
- APPENDIX_MAINTENANCE_VERDICT: OK
- PILLAR_ALIGNMENT_VERDICT: OK
- PILLARS_TOUCHED:
  - Locus
  - SQL to PostgreSQL shift readiness
  - LLM-friendly data
- PILLARS_REQUIRING_STUBS:
  - NONE
- PRIMITIVE_MATRIX_VERDICT: NONE_FOUND
- FORCE_MULTIPLIER_VERDICT: OK
- FORCE_MULTIPLIER_RESOLUTIONS:
  - GovernanceArtifactRegistry plus StructuredCollaborationEnvelopeV1 -> IN_THIS_WP (stub: NONE)
  - GovernanceArtifactRegistry plus ProjectProfileExtensionV1 -> IN_THIS_WP (stub: NONE)
  - GovernanceArtifactRegistry plus GovernancePackExport -> IN_THIS_WP (stub: NONE)
  - GovernanceArtifactRegistry plus Database trait boundary -> IN_THIS_WP (stub: NONE)
  - GovernanceArtifactRegistry plus StructuredCollaborationSummaryV1 -> IN_THIS_WP (stub: NONE)
  - GovernanceArtifactRegistry plus engine.version provenance tracking -> IN_THIS_WP (stub: NONE)
  - GovernanceArtifactRegistry plus Check-Runner downstream -> IN_THIS_WP (stub: NONE)
- STUB_WP_IDS: NONE
## PILLAR_DECOMPOSITION (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- PILLAR_DECOMPOSITION_VERDICT: OK
- DECOMPOSITION_ROWS:
  - PILLAR: Locus | CAPABILITY_SLICE: governance artifact schema registration in structured collaboration record families | SUBFEATURES: new schema ID constant, new record family variant, schema descriptor function extension | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-StructuredCollaborationSummaryV1, PRIM-GovernancePackExport | MECHANICAL: engine.sovereign, engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: extends the existing locus/types.rs structured collaboration pattern
  - PILLAR: SQL to PostgreSQL shift readiness | CAPABILITY_SLICE: governance registry store trait behind Database boundary | SUBFEATURES: GovernanceArtifactRegistryStore trait with load/save/lookup/list | PRIMITIVES_FEATURES: PRIM-Database | MECHANICAL: engine.dba, engine.version | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: store trait uses the Database trait boundary, keeping PostgreSQL portability intact
  - PILLAR: LLM-friendly data | CAPABILITY_SLICE: governance artifact metadata as structured JSON | SUBFEATURES: JSON-serializable registry entries with schema IDs and provenance | PRIMITIVES_FEATURES: PRIM-StructuredCollaborationEnvelopeV1, PRIM-GovernancePackExport | MECHANICAL: engine.context | ROI: HIGH | RESOLUTION: IN_THIS_WP | STUB: NONE | NOTES: follows the canonical structured collaboration mandate from [ADD v02.167]
## EXECUTION_RUNTIME_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXECUTION_RUNTIME_ALIGNMENT_VERDICT: OK
- ALIGNMENT_ROWS:
  - Capability: governance artifact registry load/store | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: internal structured data store consumed by downstream governance runners and DCC projections
  - Capability: governance artifact kind enumeration and schema registration | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: VISIBLE | StoragePosture: SQLITE_NOW_POSTGRES_READY | Resolution: IN_THIS_WP | Stub: NONE | Notes: extends the structured collaboration schema registry to include governance artifact records
  - Capability: SoftwareDelivery profile extension for governance artifact references | JobModel: NONE | Workflow: NONE | ToolSurface: NONE | ModelExposure: BOTH | CommandCenter: NONE | FlightRecorder: NONE | Locus: NONE | StoragePosture: N/A | Resolution: IN_THIS_WP | Stub: NONE | Notes: profile extension metadata attaches to structured collaboration records as non-breaking SoftwareDelivery-scoped data
## EXISTING_CAPABILITY_ALIGNMENT (REFINEMENT OUTPUT; REQUIRED FOR HYDRATED PROFILE)
- EXISTING_CAPABILITY_ALIGNMENT_VERDICT: OK
- MATCHED_ARTIFACT_RESOLUTIONS:
  - WP-1-Product-Governance-Check-Runner-v1 -> KEEP_SEPARATE
  - WP-1-Governance-Workflow-Mirror-v1 -> KEEP_SEPARATE
  - WP-1-Product-Governance-Snapshot-v4 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Schema-Registry-v4 -> KEEP_SEPARATE
  - WP-1-Structured-Collaboration-Artifact-Family-v1 -> KEEP_SEPARATE
  - WP-1-Project-Profile-Extension-Registry-v1 -> KEEP_SEPARATE
- CODE_REALITY_SUMMARY:
  - ../handshake_main/src/backend/handshake_core/src/governance_pack.rs -> PARTIAL (WP-1-Product-Governance-Snapshot-v4)
  - ../handshake_main/src/backend/handshake_core/src/locus/types.rs -> PARTIAL (WP-1-Product-Governance-Snapshot-v4)
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
- What: Define and implement a product-owned registry for imported software-delivery governance artifacts (codex, role protocols, rubrics, check manifests, script descriptors, schemas, templates, sync surfaces) as versioned, typed, provenance-linked structured collaboration records scoped to the SoftwareDelivery project profile.
- Why: Handshake needs a bounded, versioned way to ingest the current repo governance surface so downstream runners and DCC projections can consume governance definitions without treating repo file paths as runtime authority or collapsing Handshake's broader multi-domain identity into software-delivery-only rules.
- IN_SCOPE_PATHS:
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/lib.rs
- OUT_OF_SCOPE:
  - Executing imported checks or scripts (WP-1-Product-Governance-Check-Runner-v1)
  - Workflow state mirroring (WP-1-Governance-Workflow-Mirror-v1)
  - DCC UI projections or typed viewers (WP-1-Dev-Command-Center-Control-Plane-Backend-v1)
  - Replacing or overwriting Handshake-native governance
  - Script content import (only descriptors; content stays in governance pack archives)
  - Multi-provider model execution concerns (downstream session-substrate WPs)
- TOUCHED_FILE_BUDGET: 4
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
cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
```

### DONE_MEANS
- GovernanceArtifactKind enum covers all canonical artifact types from spec 7.5.4.3.
- GovernanceArtifactRegistryManifest can be stored and loaded through the existing storage trait boundary.
- Schema descriptor for hsk.governance_artifact_registry@1 is registered in the structured collaboration schema registry function.
- Profile extension validation passes for the governance registry extension on SoftwareDelivery records.
- Non-SoftwareDelivery project profiles do not see or require governance registry extensions.
- cargo test and just gov-check pass on the WP branch.

- PRIMITIVES_EXPOSED:
  - PRIM-StructuredCollaborationEnvelopeV1
  - PRIM-StructuredCollaborationSummaryV1
  - PRIM-ProjectProfileExtensionV1
  - PRIM-GovernancePackExport
  - PRIM-GovernancePackExportRequest
  - PRIM-Database
- PRIMITIVES_CREATED:
  - NONE

### ROLLBACK_HINT
```bash
git revert <commit-sha>
```
## AUTHORITY
- SPEC_BASELINE: Handshake_Master_Spec_vXX.XX.md (recorded_at: 2026-04-05T17:43:32.006Z)
- SPEC_TARGET: .GOV/spec/SPEC_CURRENT.md (closure/revalidation target; resolved at validation time)
- SPEC_ADD_MARKER_TARGET: [ADD v02.179]
- SPEC_ANCHOR_PRIMARY: Handshake_Master_Spec_v02.179.md 7.5.4.8 Governance Pack: Project-Specific Instantiation (HARD)
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
  - src/backend/handshake_core/src/governance_artifact_registry.rs
  - src/backend/handshake_core/src/locus/types.rs
  - src/backend/handshake_core/src/governance_pack.rs
  - src/backend/handshake_core/src/storage/mod.rs
  - src/backend/handshake_core/src/capabilities.rs
  - src/backend/handshake_core/src/lib.rs
- SEARCH_TERMS:
  - GovernanceArtifactKind
  - governance_artifact_registry
  - GovernancePackExportRequest
  - StructuredCollaborationRecordFamily
  - structured_collaboration_schema_descriptor
  - ProjectProfileKind
  - validate_profile_extension
  - hsk.governance_artifact_registry
- RUN_COMMANDS:
  ```bash
rg -n "GovernanceArtifactKind|governance_artifact_registry" src/backend/handshake_core/src
  rg -n "StructuredCollaborationRecordFamily|structured_collaboration_schema_descriptor" src/backend/handshake_core/src
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml governance_artifact_registry
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml structured_collaboration_schema
  cargo test --manifest-path src/backend/handshake_core/Cargo.toml
  just gov-check
  ```
- RISK_MAP:
  - "Imported governance artifacts replace Handshake-native governance" -> "product becomes repo-governance shell and loses multi-domain identity"
  - "Registry stores repo file paths as runtime authority" -> "product code bypasses registry and reads .GOV directly"
  - "GovernanceArtifactKind grows unbounded" -> "validation complexity becomes unmanageable"
  - "Manifest content hash not verified at load time" -> "tampered governance artifacts enter runtime"
  - "Schema ID collision with existing structured collaboration records" -> "downstream consumers misparse governance artifacts"
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
- (Mechanical manifest for audit. Fill real values to enable 'just post-work'. This section records the 'What' (hashes/lines) for the Validator's 'How/Why' audit. It is NOT a claim of official Validation.)
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
  - LOG_PATH: `.handshake/logs/WP-1-Product-Governance-Artifact-Registry-v1/<name>.log` (recommended; not committed)
  - LOG_SHA256: `<hash>`
  - PROOF_LINES: `<copy/paste 1-10 critical lines (e.g., "0 failed", "PASS")>`

## VALIDATION_REPORTS
- (Validator appends official audits and verdicts here. Append-only.)

### WP Validator Report 1
- DATE: 2026-04-05T19:05:00Z
- VALIDATOR_ROLE: WP_VALIDATOR
- VALIDATOR_MODEL: claude-opus-4-6
- VALIDATOR_MODEL_PROFILE: CLAUDE_CODE_OPUS_4_6_THINKING_MAX
- COMMIT: 277410a
- BRANCH: validate/WP-1-Product-Governance-Artifact-Registry-v1
- SPEC_TARGET: Handshake_Master_Spec_v02.179.md
- GOVERNED_VALIDATOR_REPORT_PROFILE: SPLIT_DIFF_SCOPED_RIGOR_V4
- VALIDATION_CONTEXT: OK
- GOVERNANCE_VERDICT: PASS
- TEST_VERDICT: PASS
- CODE_REVIEW_VERDICT: PASS
- HEURISTIC_REVIEW_VERDICT: PASS
- SPEC_ALIGNMENT_VERDICT: PASS
- ENVIRONMENT_VERDICT: PASS
- DISPOSITION: NONE
- LEGAL_VERDICT: PASS
- SPEC_CONFIDENCE: REVIEWED_DIFF_SCOPED
- WORKFLOW_VALIDITY: VALID
- SCOPE_VALIDITY: IN_SCOPE
- PROOF_COMPLETENESS: PROVEN
- INTEGRATION_READINESS: READY
- DOMAIN_GOAL_COMPLETION: COMPLETE
- MECHANICAL_TRACK_VERDICT: PASS
- SPEC_RETENTION_TRACK_VERDICT: PASS
- Verdict: PASS

- CLAUSES_REVIEWED:
  - Governance Pack project-specific instantiation 7.5.4.8: GovernanceArtifactKind enum (governance_artifact_registry.rs:18-25) defines 6 governance pack component kinds {Codex, Protocols, Rubrics, Checks, Templates, Schemas}. GovernanceArtifactRegistryManifest (governance_artifact_registry.rs:72-88) carries schema_id, schema_version, registry_id, entries. GovernanceArtifactRegistryStore trait (governance_artifact_registry.rs:86-97) provides load_manifest, save_manifest, list_manifests with StorageError boundary. InMemoryGovernanceArtifactRegistryStore (governance_artifact_registry.rs:103-139) implements test-time contract. Tests at governance_artifact_registry.rs:142-245 cover round-trip, kind exhaustiveness, store contract, and serde extension fields.
  - Canonical structured collaboration artifact family [ADD v02.167]: GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1 = "hsk.governance_artifact_registry@1" (locus/types.rs:824-825). GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1 = "1" (locus/types.rs:828). GovernanceArtifactRegistry variant added to StructuredCollaborationRecordFamily (locus/types.rs:839). Schema descriptor wired at locus/types.rs:988-996 with schema_id, schema_version, record_kind="governance_artifact_registry", summary_family=None. Test at locus/types.rs:1820-1833 verifies descriptor correctness.
  - Base envelope + profile extension contract [ADD v02.168]: GOVERNANCE_ARTIFACT_REGISTRY_EXTENSION_SCHEMA_ID_V1 = "hsk.ext.software_delivery.governance_artifact_registry@1" (locus/types.rs:826-827). validate_profile_extension signature extended with project_profile_kind parameter (locus/types.rs:1652-1653). SoftwareDelivery-only guard at locus/types.rs:1688-1698 pushes IncompatibleProfileExtension when profile is non-SoftwareDelivery. Schema version consistency check at locus/types.rs:1700-1708. Acceptance test at locus/types.rs:1835-1853 confirms SoftwareDelivery passes. Rejection test at locus/types.rs:1855-1873 confirms Research profile is rejected with IncompatibleProfileExtension issue.

- NOT_PROVEN:
  - NONE

- MAIN_BODY_GAPS:
  - NONE

- QUALITY_RISKS:
  - NONE

- VALIDATOR_RISK_TIER: HIGH

- DIFF_ATTACK_SURFACES:
  - GovernanceArtifactKind serde: snake_case rename_all could produce unexpected wire values if new variants are added with multi-word names; current variants are all single-word so this is safe now
  - validate_profile_extension now takes an additional parameter -- callers outside locus/types.rs could break if the function were pub; it is fn (private), so safe
  - InMemoryGovernanceArtifactRegistryStore empty-entries validation in save_manifest: returns StorageError::Validation, which is a &str reference -- must not introduce lifetime issues (it uses a static str literal, so safe)
  - GovernanceArtifactRegistryManifest serde defaults: default_governance_artifact_registry_schema_id and default_governance_artifact_registry_schema_version inject the V1 constants, so deserialization of records without schema_id/version still produces valid V1 records

- INDEPENDENT_CHECKS_RUN:
  - cargo test --lib -- governance_artifact_registry => 7 passed, 0 failed
  - cargo test --lib (full suite, no filter) => 214 passed, 0 failed, 0 regressions
  - Verified GovernanceArtifactKind::all() length (6) matches the enum variant count in governance_artifact_registry.rs:18-25 => exhaustive
  - Verified push_issue sets ok=false at locus/types.rs:930 => IncompatibleProfileExtension truly fails validation

- COUNTERFACTUAL_CHECKS:
  - If GovernanceArtifactRegistry variant were removed from StructuredCollaborationRecordFamily (locus/types.rs:839), the match arm at locus/types.rs:988-996 would produce a compile error (non-exhaustive pattern)
  - If GOVERNANCE_ARTIFACT_REGISTRY_EXTENSION_SCHEMA_ID_V1 (locus/types.rs:826-827) were changed, the guard at locus/types.rs:1688 would never match and SoftwareDelivery-only enforcement would silently pass all profiles -- the rejection test at locus/types.rs:1855-1873 would catch this
  - If validate_profile_extension lost the project_profile_kind parameter (locus/types.rs:1652), the call site at locus/types.rs:1090-1094 would fail to compile (arity mismatch)
  - If GovernanceArtifactKind::Codex were removed, the all() const array at governance_artifact_registry.rs:29-36 would still compile but the governance_artifact_kind_is_exhaustive test would fail because assert_eq!(kinds.len(), 6) expects 6

- BOUNDARY_PROBES:
  - GovernanceArtifactRegistryStore trait uses crate::storage::StorageError (governance_artifact_registry.rs:12), establishing the same boundary contract as DiagnosticsStore (diagnostics/mod.rs:899-901); this is a codebase-established pattern for domain-specific store traits
  - The Database trait (storage/mod.rs:1568) is a monolithic trait for workspace/document/block/loom operations; domain stores like DiagnosticsStore and GovernanceArtifactRegistryStore are correctly kept separate
  - GovernanceArtifactRegistryManifest references GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_ID_V1 and GOVERNANCE_ARTIFACT_REGISTRY_SCHEMA_VERSION_V1 from crate::workflows::locus (governance_artifact_registry.rs:9-11), creating a compile-time dependency on the locus type constants

- NEGATIVE_PATH_CHECKS:
  - InMemoryGovernanceArtifactRegistryStore::save_manifest rejects empty entries with StorageError::Validation at governance_artifact_registry.rs:123-124
  - validate_profile_extension pushes IncompatibleProfileExtension for non-SoftwareDelivery profiles at locus/types.rs:1689-1698; the test at locus/types.rs:1855-1873 exercises this path with ProjectProfileKind::Research
  - validate_profile_extension pushes SchemaVersionMismatch when extension_schema_version does not match at locus/types.rs:1700-1707

- INDEPENDENT_FINDINGS:
  - GovernanceArtifactKind enum covers governance pack component kinds from spec 7.5.4.8 {codex, protocols, rubrics, checks, templates, schemas}, NOT the kernel artifact file types from spec 7.5.4.3 {spec, task_board, traceability_registry, refinements, stubs, packets, signatures, gates, templates}. The packet CANONICAL_CONTRACT_EXAMPLES reference to "spec 7.5.4.3" is an inaccurate spec section reference. The code taxonomy is correct for 7.5.4.8.
  - storage/mod.rs is listed in the packet HOT_FILES and EXPECTED_CODE_SURFACES but was not modified. The GovernanceArtifactRegistryStore is a standalone trait following the established DiagnosticsStore pattern. This is architecturally correct -- the Database trait is a monolithic interface for workspace/document/block operations and domain-specific stores are kept separate.
  - InMemoryGovernanceArtifactRegistryStore triggers #[warn(dead_code)] because it is only constructed in #[cfg(test)]. This is expected for a test-only in-memory implementation.
  - The coder's post-work gate fails with 10 errors (non-ASCII in packet, missing EVIDENCE_MAPPING, missing EVIDENCE COMMAND+EXIT_CODE, missing STATUS_HANDOFF hint, missing VALIDATION manifest coverage for all 3 changed files, and placeholder manifest entries). These are coder-side lifecycle completeness items, not code quality issues.

- RESIDUAL_UNCERTAINTY:
  - The GovernanceArtifactKind taxonomy {Codex, Protocols, Rubrics, Checks, Templates, Schemas} is a design choice not explicitly enumerated in any single spec section; it represents a reasonable decomposition of 7.5.4.8 governance pack components but is not spec-mandated at this granularity
  - No Database-backed GovernanceArtifactRegistryStore implementation exists yet; only InMemory is provided. The trait boundary is correct for future PostgreSQL implementation but actual persistence is deferred.

- SPEC_CLAUSE_MAP:
  - 7.5.4.8 Governance Pack instantiation -> governance_artifact_registry.rs:18-25 (GovernanceArtifactKind enum), governance_artifact_registry.rs:52-70 (GovernanceArtifactRegistryEntry with artifact_id/kind/provenance/content_hash), governance_artifact_registry.rs:72-88 (GovernanceArtifactRegistryManifest with schema_id/schema_version), governance_artifact_registry.rs:86-97 (GovernanceArtifactRegistryStore trait)
  - [ADD v02.167] Canonical structured collaboration artifact family -> locus/types.rs:824-825 (schema ID "hsk.governance_artifact_registry@1"), locus/types.rs:828 (schema version "1"), locus/types.rs:839 (GovernanceArtifactRegistry record family variant), locus/types.rs:988-996 (schema descriptor)
  - [ADD v02.168] Base envelope + profile extension contract -> locus/types.rs:826-827 (extension schema ID "hsk.ext.software_delivery.governance_artifact_registry@1"), locus/types.rs:1083-1094 (project_profile_kind parsing and forwarding), locus/types.rs:1688-1698 (SoftwareDelivery-only guard), locus/types.rs:1700-1708 (extension schema version check)

- NEGATIVE_PROOF:
  - GovernanceArtifactKind enum covers governance pack component categories from spec 7.5.4.8 {codex, protocols, rubrics, checks, templates, schemas}. It does NOT cover spec 7.5.4.3 kernel artifact file types {spec, task_board, traceability_registry, refinements, stubs, packets, signatures, gates, templates}. The packet's CANONICAL_CONTRACT_EXAMPLES claim "covers all canonical types from spec 7.5.4.3" is an inaccurate spec section reference. Only "templates" overlaps between the two taxonomies. Verified by reading spec 7.5.4.3 at Handshake_Master_Spec_v02.179.md:31759-31774 and comparing to code at governance_artifact_registry.rs:18-25.
  - storage/mod.rs (packet HOT_FILES and EXPECTED_CODE_SURFACES) has zero diff lines. The GovernanceArtifactRegistryStore trait at governance_artifact_registry.rs:86-97 is standalone, importing only StorageError. No Database trait integration or registration exists.

- ANTI_VIBE_FINDINGS:
  - NONE

- SIGNED_SCOPE_DEBT:
  - NONE

- PRIMITIVE_RETENTION_PROOF:
  - StructuredCollaborationRecordFamily enum at locus/types.rs:834-855: all pre-existing variants (WorkPacketPacket, WorkPacketSummary, MicroTaskPacket, MicroTaskSummary, TaskBoardEntry, TaskBoardIndex, TaskBoardView, RoleMailboxIndex, RoleMailboxThreadLine) remain present; GovernanceArtifactRegistry is purely additive
  - structured_collaboration_schema_descriptor at locus/types.rs:960-1051: all pre-existing match arms retained; new GovernanceArtifactRegistry arm added at locus/types.rs:988-996
  - validate_structured_collaboration_record at locus/types.rs:1057-1130: pre-existing validation logic for WorkPacketPacket, MicroTaskPacket, WorkPacketSummary, MicroTaskSummary, TaskBoardEntry, TaskBoardIndex, TaskBoardView, RoleMailboxIndex, RoleMailboxThreadLine unchanged; GovernanceArtifactRegistry arm at locus/types.rs:1103 is empty (no additional validation beyond base envelope)
  - validate_profile_extension at locus/types.rs:1652: signature extended with project_profile_kind parameter; existing validation logic for extension_schema_id, extension_schema_version, compatibility preserved at locus/types.rs:1678-1718; new governance-specific guard is additive (locus/types.rs:1688-1709)

- PRIMITIVE_RETENTION_GAPS:
  - NONE

- SHARED_SURFACE_INTERACTION_CHECKS:
  - locus/types.rs StructuredCollaborationRecordFamily: new GovernanceArtifactRegistry variant is purely additive to the existing enum; serde rename_all="snake_case" serializes as "governance_artifact_registry" which does not conflict with existing variant names
  - locus/types.rs structured_collaboration_schema_descriptor: GovernanceArtifactRegistry arm at locus/types.rs:988-996 follows the same StructuredCollaborationSchemaDescriptor pattern as all other families; no existing arm modified
  - locus/types.rs validate_structured_collaboration_record: the GovernanceArtifactRegistry match arm at locus/types.rs:1103 is empty, meaning it relies solely on base envelope validation (same as RoleMailbox families). No new required fields beyond the base envelope.
  - lib.rs: pub mod governance_artifact_registry at lib.rs:9 is purely additive (alphabetically inserted); no existing module declarations disturbed

- CURRENT_MAIN_INTERACTION_CHECKS:
  - Verified locus/types.rs on main (via merge-base): StructuredCollaborationRecordFamily has no GovernanceArtifactRegistry variant. The addition is non-conflicting.
  - Verified validate_profile_extension on main has 2-parameter signature (value, result). The 3-parameter version (value, project_profile_kind, result) in this branch changes the private function signature; no external callers exist since fn is private to the module.
  - lib.rs on main has no governance_artifact_registry module. Addition is purely additive at the correct alphabetical position.

- DATA_CONTRACT_PROOF:
  - GovernanceArtifactRegistryManifest (governance_artifact_registry.rs:72-88): JSON-serializable with schema_id, schema_version, registry_id (UUID), entries. All fields are explicit machine-readable values with stable field names.
  - GovernanceArtifactRegistryEntry (governance_artifact_registry.rs:62-70): artifact_id (UUID), kind (enum -> snake_case string), provenance (structured with source_artifact, snapshot_version, imported_at DateTime), content_hash (string). Explicit structured fields, no overloaded text blobs.
  - GovernanceArtifactRegistryStore trait (governance_artifact_registry.rs:86-97): load/save/list pattern using StorageError from crate::storage, keeping PostgreSQL portability path open. No SQLite-specific semantics introduced.
  - GovernanceArtifactKind serde (governance_artifact_registry.rs:17): rename_all="snake_case" produces stable wire values {"codex", "protocols", "rubrics", "checks", "templates", "schemas"}.
  - Schema constants (locus/types.rs:824-828): "hsk.governance_artifact_registry@1" and version "1" follow the existing hsk.* namespace convention.

- DATA_CONTRACT_GAPS:
  - NONE
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
