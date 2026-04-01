# Repo Governance Changelog

## Metadata

- SCHEMA_VERSION: `hsk.repo_governance_changelog@0.1`
- STATUS: ACTIVE
- PURPOSE: durable governance-only change history for the repo governance kernel
- VERSIONING_RULE: `CHANGESET_VERSION` uses sortable `YYYY.MM.DD.N`
- LINKAGE_RULE: every entry must cite a stable `CHANGESET_ID` plus the driving `AUDIT_ID` and/or `SMOKETEST_REVIEW_ID`
- RELATED_TASK_BOARD: `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`

## Linkage Keys

- AUDIT_ID: `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SMOKETEST_REVIEW_ID: `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
- AUDIT_ID: `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SMOKETEST_REVIEW_ID: `SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1`
- AUDIT_ID: `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SMOKETEST_REVIEW_ID: `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- HISTORICAL_COMPARISON_AUDIT_ID: `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`

## Entries

### 2026.03.25.1 / GOV-CHANGE-20260325-01

- STATUS: APPLIED
- SUMMARY: enforced chat-visible refinement display as a hard Orchestrator requirement
- CHANGE_TYPE: POLICY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
- OUTCOME: invisible terminal or tool output no longer counts as refinement-display proof

### 2026.03.25.2 / GOV-CHANGE-20260325-02

- STATUS: APPLIED
- SUMMARY: restricted Orchestrator helper-agent use so product-code writes require explicit operator approval and packet evidence
- CHANGE_TYPE: POLICY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
- OUTCOME: governance/spec/refinement assistance remains allowed, but Orchestrator-managed product-code delegation is blocked unless operator approval is recorded explicitly

### 2026.03.25.3 / GOV-CHANGE-20260325-03

- STATUS: APPLIED
- SUMMARY: created stable smoketest linkage and governance-maintenance tracking surfaces for post-refactor follow-on work
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260325-SCHEMA-REGISTRY-V4`
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_SCHEMA_REGISTRY_V4_SMOKETEST_RECOVERY_REVIEW.md`
- FOLLOW_ON_ITEMS:
  - `RGF-03`
  - `RGF-04`
  - `RGF-05`
  - `RGF-06`
- OUTCOME: governance remediation now tracks by stable item IDs and changeset IDs instead of improvised Work-Packet-like handling

### 2026.03.25.4 / GOV-CHANGE-20260325-04

- STATUS: APPLIED
- SUMMARY: added a dedicated contract-hardening smoketest closeout review and upgraded the smoketest template to capture lineage, runtime truth, and merge containment explicitly
- CHANGE_TYPE: RECORDKEEPING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1`
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SURFACES:
  - `.GOV/Audits/smoketest/AUDIT_20260325_CONTRACT_HARDENING_V1_SMOKETEST_CLOSEOUT_REVIEW.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
- OUTCOME: future smoketest reviews now have a stronger default structure for predecessor linkage, failure inventory, role review, ACP-runtime truth, remaining adjacent debt, and command-log evidence

### 2026.03.25.5 / GOV-CHANGE-20260325-05

- STATUS: APPLIED
- SUMMARY: made predecessor-to-successor improvement comparison a required smoketest-review element
- CHANGE_TYPE: REVIEW_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260325-CONTRACT-HARDENING-V1`
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SURFACES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_CONTRACT_HARDENING_V1_SMOKETEST_CLOSEOUT_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: follow-on smoketest reviews must now state exactly what improved relative to the previous smoketest, which makes recovery and closeout runs easier to compare and harder to overstate

### 2026.03.25.6 / GOV-CHANGE-20260325-06

- STATUS: APPLIED
- SUMMARY: required explicit `Handshake (Product)` versus `Repo Governance` scope splits in operator-facing chat and role guidance
- CHANGE_TYPE: OPERATOR_UX_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/START_HERE.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `../handshake_main/AGENTS.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: operator-facing reasoning now distinguishes product code/spec/WP work from repo-governance/ACP/protocol work, even when the domain language is governance-themed

### 2026.03.25.7 / GOV-CHANGE-20260325-07

- STATUS: APPLIED
- SUMMARY: formalized a mandatory post-smoketest improvement rubric for workflow smoothness, Master Spec gap reduction, and token-cost pressure
- CHANGE_TYPE: REVIEW_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_ORCHESTRATOR_MANAGED_WP_WORKFLOW_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: future smoketest reviews must now state whether the workflow got smoother, whether the real Master Spec gap list got smaller, and whether the run got cheaper in operator/token cost, with a named next structural fix for each target

### 2026.03.25.8 / GOV-CHANGE-20260325-08

- STATUS: APPLIED
- SUMMARY: expanded smoketest reviews and live role guidance to treat silent failures, wrong command usage, ambiguity, and governance-document churn as explicit workflow signals
- CHANGE_TYPE: REVIEW_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/Audits/smoketest/AUDIT_20260325_ORCHESTRATOR_MANAGED_WP_WORKFLOW_REVIEW.md`
- OUTCOME: future reviews and role guidance now treat false greens, wrong tool-family choices, ambiguous task/path truth, and repeated governance-document or command-surface rereads as explicit evidence of workflow ambiguity and avoidable token burn

### 2026.03.25.9 / GOV-CHANGE-20260325-09

- STATUS: APPLIED
- SUMMARY: moved minimal live-read-set and anti-rediscovery guidance into governed startup prompts and made the smoketest template more self-sufficient
- CHANGE_TYPE: TOKEN_DISCIPLINE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: coder, validator, and integration-validator startup prompts now carry the minimal live read set directly, repeated protocol rereads or command rediscovery are explicitly treated as ambiguity signals, and the smoketest template remains usable even when the separate rubric document is not open

### 2026.03.25.10 / GOV-CHANGE-20260325-10

- STATUS: APPLIED
- SUMMARY: added the remaining post-smoketest workflow concerns as sequenced governance items after `RGF-06`
- CHANGE_TYPE: PLANNING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
  - `SMOKETEST-REVIEW-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-09`
  - `RGF-10`
  - `RGF-11`
  - `RGF-12`
- OUTCOME: the remaining concerns around invalidity rules, undeclared auxiliary worktrees, mid-run approval relapse, and signed-scope/current-main compatibility now exist as explicit sequenced governance work instead of only audit prose

### 2026.03.25.11 / GOV-CHANGE-20260325-11

- STATUS: APPLIED
- SUMMARY: split validator PASS closure from main containment so `Done` becomes merge-pending and `Validated (PASS)` requires local `main` containment proof
- CHANGE_TYPE: WORKFLOW_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/lib/merge-progression-truth-lib.mjs`
  - `.GOV/roles_shared/checks/merge-progression-truth-check.mjs`
  - `.GOV/roles_shared/checks/packet-truth-check.mjs`
  - `.GOV/roles_shared/checks/task-board-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/orchestrator/scripts/task-board-set.mjs`
  - `.GOV/roles_shared/records/TASK_BOARD.md`
- OUTCOME: packets on the new format can no longer claim `Validated (PASS)` until the approved closure commit is recorded and proven to be contained in local `main`; Task Board and runtime truth now distinguish `[MERGE_PENDING]` from `[VALIDATED]`

### 2026.03.25.12 / GOV-CHANGE-20260325-12

- STATUS: APPLIED
- SUMMARY: added an integration-validator closeout preflight so final PASS commit clearance fails if the final lane cannot resolve the committed target or if WP-scoped session-control truth is still unsettled
- CHANGE_TYPE: FINAL_LANE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `justfile`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: orchestrator-managed final review now has a machine-checked preflight for topology safety and WP-scoped atomic closeout, and PASS commit clearance fails before partial closure truth can be written when that preflight is not satisfied

### 2026.03.25.13 / GOV-CHANGE-20260325-13

- STATUS: APPLIED
- SUMMARY: added deterministic session-control self-settlement so recoverable orphaned requests gain exactly one terminal result row without manual ledger repair
- CHANGE_TYPE: ACP_RUNTIME_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
  - `AUDIT-20260325-CONTRACT-HARDENING-V1-SMOKETEST-CLOSEOUT-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-self-settle-lib.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles_shared/tests/session-control-self-settle-lib.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: broker startup and governed session-control helpers now auto-recover missing terminal result rows for recoverable orphan cases, which reduces manual runtime truth repair and makes `SESSION_CONTROL_REQUESTS` -> `SESSION_CONTROL_RESULTS` convergence machine-driven

### 2026.03.25.14 / GOV-CHANGE-20260325-14

- STATUS: APPLIED
- SUMMARY: added explicit modeled lineage for packets that are both failed historical closures and live smoketest baselines
- CHANGE_TYPE: WORKFLOW_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT_20260321_PARALLEL_WP1_V3_PRODUCT_SPEC_ALIGNMENT`
  - `AUDIT-20260325-SCHEMA-REGISTRY-V4-SMOKETEST-RECOVERY-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/records/WP_TRACEABILITY_REGISTRY.md`
  - `.GOV/roles_shared/records/TASK_BOARD.md`
  - `.GOV/roles_shared/scripts/lib/historical-smoketest-lineage-lib.mjs`
  - `.GOV/roles_shared/checks/historical-smoketest-lineage-check.mjs`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: historical failed packets can now remain visible as live smoketest lineage anchors without overloading stub/backlog or superseded truth, and `gov-check` enforces the modeled linkage

### 2026.03.25.15 / GOV-CHANGE-20260325-15

- STATUS: APPLIED
- SUMMARY: added machine-visible orchestrator-managed workflow invalidity receipts and blocked manual checkpoint relapse on those lanes
- CHANGE_TYPE: WORKFLOW_INVALIDITY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-invalidity-flag.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/coder/checks/pre-work.mjs`
  - `.GOV/roles/coder/checks/post-work-check.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles/coder/checks/coder-skeleton-checkpoint.mjs`
  - `.GOV/roles_shared/checks/skeleton-approved.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: orchestrator-managed WPs now emit a structured `WORKFLOW_INVALIDITY` state for invalid procedure, communication-health and validator closeout can fail on that ledgered state, and manual skeleton checkpoint/approval commands are no longer silently tolerated on those lanes

### 2026.03.25.16 / GOV-CHANGE-20260325-16

- STATUS: APPLIED
- SUMMARY: enforced packet-declared WP topology and rejected undeclared auxiliary worktrees during live checks and closeout
- CHANGE_TYPE: TOPOLOGY_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-declared-topology-lib.mjs`
  - `.GOV/roles_shared/checks/worktree-concurrency-check.mjs`
  - `.GOV/roles_shared/checks/wp-declared-topology-check.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/tests/wp-declared-topology-lib.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_WORKTREES.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- OUTCOME: auxiliary detached check/postwork/validator clones are now mechanically visible as topology violations, the global concurrency gate and final validator gates share the same topology law, and `just wp-declared-topology-check WP-{ID}` exposes the packet-declared topology for one WP directly

### 2026.03.26.01 / GOV-CHANGE-20260326-01

- STATUS: APPLIED
- SUMMARY: blocked routine post-signature Operator interruption on orchestrator-managed lanes and made resume outputs name blocker classes explicitly
- CHANGE_TYPE: OPERATOR_AUTONOMY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/scripts/lib/role-resume-utils.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed role startup prompts now forbid routine post-signature Operator approval/proceed/checkpoint relapse on orchestrator-managed lanes, resume output carries machine-visible `BLOCKER_CLASS` state, and protocol alignment checks fail if that contract drifts

### 2026.03.26.02 / GOV-CHANGE-20260326-02

- STATUS: APPLIED
- SUMMARY: added signed-scope compatibility truth and blocked ad hoc packet widening during final-lane closeout
- CHANGE_TYPE: SCOPE_GOVERNANCE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/signed-scope-compatibility-lib.mjs`
  - `.GOV/roles_shared/tests/signed-scope-compatibility-lib.test.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: modern packets now carry explicit current-`main` compatibility and packet-widening truth, final-lane closeout fails on stale compatibility baselines or ungoverned adjacent scope drift, and new packets default to the stricter 2026-03-26 packet format

### 2026.03.26.03 / GOV-CHANGE-20260326-03

- STATUS: APPLIED
- SUMMARY: added terminal closeout projection sync so task-board terminal moves reject packet drift and immediately refresh runtime projection truth
- CHANGE_TYPE: CLOSEOUT_TRUTH_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles/orchestrator/scripts/task-board-set.mjs`
  - `.GOV/roles_shared/scripts/lib/merge-progression-truth-lib.mjs`
  - `.GOV/roles_shared/tests/merge-progression-truth-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: terminal Task Board moves now fail if packet status disagrees with the requested board state, runtime projection fields are refreshed immediately from packet truth on those transitions, and merge-progression checks now detect lagging runtime `current_packet_status`

### 2026.03.26.04 / GOV-CHANGE-20260326-04

- STATUS: APPLIED
- SUMMARY: added a canonical integration-validator context bundle so final-lane review can open one authority/path/source-of-truth surface instead of rediscovering final-lane context manually
- CHANGE_TYPE: FINAL_LANE_CONTEXT_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-context-brief.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-command-surface.test.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/validator/README.md`
  - `justfile`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: the Integration Validator now has a single read-only context bundle command that surfaces authority, actor/session identity, committed handoff target, declared topology, current-`main` compatibility, and the exact final-lane command sequence, which reduces repeated protocol rereads and path/authority rediscovery

### 2026.03.26.05 / GOV-CHANGE-20260326-05

- STATUS: APPLIED
- SUMMARY: added a dedicated operator-rule-restatement invalidity helper and projected those cases to a machine-visible lane-reset route
- CHANGE_TYPE: WORKFLOW_INVALIDITY_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles_shared/scripts/wp/wp-operator-rule-restatement.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `justfile`
- OUTCOME: when the Operator has to restate a core orchestrator-managed lane rule, the repo now has a dedicated helper that records the condition as `OPERATOR_RULE_RESTATEMENT`, and communication/runtime projection routes that WP to `LANE_RESET_REQUIRED` instead of treating it as generic invalidity noise

### 2026.03.26.06 / GOV-CHANGE-20260326-06

- STATUS: APPLIED
- SUMMARY: narrowed validator gate write surfaces so wrong-lane orchestrator-managed usage fails early and points to the correct helper family
- CHANGE_TYPE: COMMAND_SURFACE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW`
- SURFACES:
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/tests/validator-next.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: `validator-gate-*` writes on orchestrator-managed packets now fail before mutating state when the current branch/worktree does not resolve to a governed validator lane, and the error points callers to `validator-next`, `integration-validator-context-brief`, or `external-validator-brief` instead of letting wrong-tool attempts masquerade as legitimate gate progression

### 2026.03.31.01 / GOV-CHANGE-20260331-01

- STATUS: APPLIED
- SUMMARY: restored communications-repair command-surface parity and made WP communications template drift fail closed before invalid runtime artifacts can be written
- CHANGE_TYPE: TOOLING_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/tests/ensure-wp-communications.test.mjs`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-command-surface.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-40`
- OUTCOME: the sanctioned `just ensure-wp-communications WP-{ID}` repair helper now exists, unreplaced `{{TOKEN}}` drift in packet communication templates is rejected before file writes, and regression checks cover both command-surface parity and template-token completeness

### 2026.03.31.02 / GOV-CHANGE-20260331-02

- STATUS: APPLIED
- SUMMARY: hardened orchestrator-managed launch so the ordinary launch path auto-issues the first governed ACP start instead of stopping at a launch-only false green
- CHANGE_TYPE: ACP_RUNTIME_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/GOVERNED_WORKFLOW_EXAMPLES.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/checks/protocol-alignment-check.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-40`
- OUTCOME: supported `launch-*` paths now auto-attempt the initial governed `START_SESSION`, startup proof becomes visible in launch summaries, and the explicit `start-*` helpers remain available as recovery tools instead of being required for the normal orchestrator-managed path

### 2026.04.01.01 / GOV-CHANGE-20260401-01

- STATUS: APPLIED
- SUMMARY: converged review-receipt projection so validator review traffic updates packet/task-board/build-order truth and runtime lifecycle state without manual repair
- CHANGE_TYPE: REVIEW_PROJECTION_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: negative `VALIDATOR_REVIEW` receipts now route back into coder remediation instead of falsely progressing toward final review, newer unresolved handoffs take precedence over older resolved review pairs, and review-driven packet/task-board/build-order/runtime truth stays converged after live direct-review traffic

### 2026.04.01.02 / GOV-CHANGE-20260401-02

- STATUS: APPLIED
- SUMMARY: added orchestrator governance-checkpoint notifications on validator-authored assessment receipts so workflow authority stays in the loop after each review decision
- CHANGE_TYPE: REVIEW_NOTIFICATION_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: validator-authored assessment receipts in orchestrator-managed lanes now emit a dedicated `ORCHESTRATOR` notification alongside the normal direct-review target, so the orchestrator can verify packet/runtime/task-board truth and ACP steering immediately after each assessment without taking over coder-validator communication

### 2026.04.01.03 / GOV-CHANGE-20260401-03

- STATUS: APPLIED
- SUMMARY: aligned `validator-next` and `orchestrator-next` with projected receipt/runtime truth so live review routes and validator assessment results surface mechanically instead of through stale packet wording
- CHANGE_TYPE: RESUME_SURFACE_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles/orchestrator/tests/orchestrator-next.test.mjs`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: validator assessment checkpoints now carry `PASS`/`FAIL`/`ASSESSED` plus the validator's reason, `validator-next` follows projected `next_expected_actor` / `waiting_on` truth before falling back to legacy packet wording, and `orchestrator-next` surfaces pending governance checkpoints with the validator result and steering/closeout follow-ons instead of silently treating them as background notifications

### 2026.04.01.04 / GOV-CHANGE-20260401-04

- STATUS: APPLIED
- SUMMARY: hardened the final validator lane so governed Integration Validator sessions always resolve live governance from the kernel and governed coder handoffs fail closed unless the PREPARE target is reviewable
- CHANGE_TYPE: FINAL_LANE_AND_HANDOFF_GATE_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/tools/handshake-acp-bridge/agent.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/coder/scripts/lib/coder-governance-lib.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed Integration Validator launch/control now injects `HANDSHAKE_GOV_ROOT=<wt-gov-kernel>/.GOV`, final-lane closeout fails if live authority resolves from `handshake_main/.GOV`, coder resume surfaces reflect validator remediation truth directly, and governed `CODER_HANDOFF` receipt appends reject dirty/non-reviewable PREPARE state instead of recording a false validator-ready handoff

### 2026.04.01.05 / GOV-CHANGE-20260401-05

- STATUS: APPLIED
- SUMMARY: tightened final-lane startup and steering prompts so Integration Validator sessions must open the kernel-governed context brief before rediscovering governance surfaces
- CHANGE_TYPE: FINAL_LANE_PROMPT_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/tests/session-control-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed Integration Validator prompts now explicitly forbid manual authority rebuilds from `handshake_main/.GOV`, require `just integration-validator-context-brief WP-{ID}` ahead of broader resume work, and keep kernel governance truth mechanically in front of the final lane even when product execution stays rooted in `handshake_main`

### 2026.04.01.06 / GOV-CHANGE-20260401-06

- STATUS: APPLIED
- SUMMARY: hardened final-lane closeout sync so contained-main closure can refresh stale compatibility truth and accept only signed-surface-preserving harmonization in local main
- CHANGE_TYPE: FINAL_LANE_CLOSEOUT_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/scripts/lib/signed-scope-surface-lib.mjs`
  - `.GOV/roles_shared/tests/signed-scope-surface-lib.test.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed closeout sync now writes packet/runtime truth before the final completion gate evaluates it, stale recorded compatibility can be refreshed by the sync itself, contained-main PASS closure allows conflict-resolved local-main harmonization only when the resulting commit stays within the signed file surface and still satisfies the governed tripwire checks, and terminal closeout retires stale coder/WP-validator steerable sessions so session-governance truth converges with terminal packet state

### 2026.04.01.07 / GOV-CHANGE-20260401-07

- STATUS: APPLIED
- SUMMARY: blocked kernel-to-main governance sync on dirty kernel state so main-side sync provenance cannot silently reference stale kernel commits
- CHANGE_TYPE: SYNC_PROVENANCE_HARDENING
- DRIVER_EVIDENCE:
  - Operator follow-on governance directive after `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/topology/sync-gov-to-main.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: `just sync-gov-to-main` now fails closed when `wt-gov-kernel/.GOV` has uncommitted changes, so `GOV_KERNEL_SYNC.json` and main-side governance sync commits always refer to committed kernel truth instead of mirroring an uncheckpointed governance snapshot under a stale kernel SHA

### 2026.04.01.08 / GOV-CHANGE-20260401-08

- STATUS: APPLIED
- SUMMARY: hardened final-lane boundary enforcement so wrong-lane closeout attempts emit governed invalidity and successful contained-main closeout leaves machine-readable provenance
- CHANGE_TYPE: FINAL_LANE_BOUNDARY_AND_PROVENANCE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/checks/validator_gates.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-context-brief-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-context-brief.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-47`
- OUTCOME: final-lane closeout now classifies role-boundary misuse into machine-visible `WORKFLOW_INVALIDITY` codes, kernel/orchestrator-side closeout misuse can no longer quietly become closure truth, and successful closeout sync records attributable provenance for contained-main promotion in validator gate state plus closeout receipts

### 2026.04.01.09 / GOV-CHANGE-20260401-09

- STATUS: APPLIED
- SUMMARY: added a contract-heavy intent checkpoint between `CODER_INTENT` and `CODER_HANDOFF` so governed direct-review lanes catch signed-surface/proof drift before full handoff
- CHANGE_TYPE: DIRECT_REVIEW_CHECKPOINT_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/coder/scripts/lib/coder-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: contract-heavy or under-specified intent now routes to `WP_VALIDATOR_INTENT_CHECKPOINT`, validator/coder resume helpers surface checkpoint-specific guidance, and governed `CODER_HANDOFF` fails closed until the checkpoint or any open review items are resolved so late-loop rework and false completion claims are reduced mechanically

### 2026.04.01.10 / GOV-CHANGE-20260401-10

- STATUS: APPLIED
- SUMMARY: captured new review-quality, microtask-discipline, and LLM-first data follow-ons from the WP-1 smoketest evidence so the governance board now maps them to concrete remediation work
- CHANGE_TYPE: SMOKETEST_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/Audits/smoketest/AUDIT_20260331_PROJECT_PROFILE_EXTENSION_REGISTRY_V1_SMOKETEST_STARTUP_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-48`
  - `RGF-49`
  - `RGF-50`
  - `RGF-51`
- OUTCOME: the smoketest review now explicitly records the operator requirement for validator-owned bootstrap/skeleton review, rolling microtask review overlap, zero-debt anti-vibe enforcement, and a codified LLM-first data doctrine, and the governance task board now exposes those concerns as concrete follow-on remediation items linked back to the same audit evidence

### 2026.04.01.11 / GOV-CHANGE-20260401-11

- STATUS: APPLIED
- SUMMARY: added validator-owned bootstrap/skeleton clearance plus bounded rolling overlap review so governed direct-review lanes catch weak intent earlier without letting microtask debt accumulate silently
- CHANGE_TYPE: EARLY_REVIEW_AND_MICROTASK_BACKPRESSURE_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-review-exchange.mjs`
  - `.GOV/roles_shared/schemas/WP_RECEIPT.schema.json`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/scripts/lib/validator-governance-lib.mjs`
  - `.GOV/roles/validator/scripts/validator-next.mjs`
  - `.GOV/roles_shared/tests/wp-communications-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/tests/protocol-alignment-check.test.mjs`
  - `.GOV/roles/validator/tests/validator-governance-lib.test.mjs`
- FOLLOW_ON_ITEMS:
  - `RGF-50`
  - `RGF-51`
- OUTCOME: after every governed `CODER_INTENT`, the lane now waits for explicit WP-validator bootstrap/skeleton clearance instead of silently trusting coder intent; completed microtasks can be reviewed in bounded overlap mode with a hard queue limit of 2; overlap review backlog now creates mechanical backpressure; full `CODER_HANDOFF` fails closed until overlap review debt is drained; startup prompts, resume helpers, runtime route truth, and the operator command surface now all describe the same early-review law

### 2026.04.01.12 / GOV-CHANGE-20260401-12

- STATUS: APPLIED
- SUMMARY: codified the LLM-first data contract and anti-vibe zero-debt review law so new packets must prove data posture explicitly and PASS can no longer coexist with shallow signed-scope work
- CHANGE_TYPE: DATA_CONTRACT_AND_ZERO_DEBT_REVIEW_HARDENING
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/data-contract-lib.mjs`
  - `.GOV/roles_shared/tests/data-contract-lib.test.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles/coder/checks/post-work-check.mjs`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles/validator/docs/VALIDATOR_ANTI_GAMING_RUBRIC.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: new packets (`PACKET_FORMAT_VERSION >= 2026-04-01`) can declare `DATA_CONTRACT_PROFILE=LLM_FIRST_DATA_V1`, packet creation and claim-time checks now require an authoritative data-contract monitoring block, validator closure now requires explicit `DATA_CONTRACT_PROOF` and `DATA_CONTRACT_GAPS` for active data-contract packets, coder handoffs on the new packet format now include anti-vibe, signed-scope-debt, and data-contract self-check fields, and governed RIGOR_V3 PASS law now rejects unresolved anti-vibe findings or signed-scope debt on the new packet format instead of leaving those concerns as prose-only review style

### 2026.04.01.13 / GOV-CHANGE-20260401-13

- STATUS: APPLIED
- SUMMARY: captured `roles_shared` follow-on gaps after the new packet-law hardening so explicit data-contract activation, shared-doc alignment, and end-to-end regression coverage are tracked as concrete governance work
- CHANGE_TYPE: POST_HARDENING_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-52`
  - `RGF-53`
  - `RGF-54`
- OUTCOME: the governance board now tracks three `roles_shared` follow-ons that were left implicit after `RGF-50` and `RGF-51`: making data-contract activation explicit instead of keyword-inferred, aligning shared docs/operator-facing command surfaces with the new packet law, and adding an end-to-end regression plus explicit migration policy for older packet families

### 2026.04.01.14 / GOV-CHANGE-20260401-14

- STATUS: APPLIED
- SUMMARY: captured the next orchestrator/shared follow-on after spotting stale legacy refinement-path guidance in operator-facing next-command surfaces and shared docs
- CHANGE_TYPE: PATH_TRUTH_ALIGNMENT_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-55`
- OUTCOME: the governance board now tracks a dedicated path-truth alignment item for the post-layout-migration drift where orchestrator/operator/shared surfaces still hard-code the legacy flat `.GOV/refinements/WP-{ID}.md` path instead of resolving the current co-located packet/refinement layout or using path-neutral wording
