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

### 2026.04.01.15 / GOV-CHANGE-20260401-15

- STATUS: APPLIED
- SUMMARY: captured the next orchestrator follow-on after confirming that the new `2026-04-01` packet-law bundle is enforced in code but still under-exposed in orchestrator-facing protocol and launch surfaces
- CHANGE_TYPE: OPERATOR_PACKET_LAW_VISIBILITY_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-56`
- OUTCOME: the governance board now tracks an orchestrator-specific visibility item so packet creation, next-command surfaces, and operator-facing protocol stop hiding the active `PACKET_FORMAT_VERSION`, `DATA_CONTRACT_PROFILE`, anti-vibe, signed-scope-debt, and validator-proof obligations that are already enforced later by coder/validator checks

### 2026.04.01.16 / GOV-CHANGE-20260401-16

- STATUS: APPLIED
- SUMMARY: captured the validator-side path-truth drift after finding a remaining hard-rule reference to the legacy flat refinement path in the validator protocol
- CHANGE_TYPE: VALIDATOR_PATH_TRUTH_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-57`
- OUTCOME: the governance board now tracks the remaining validator path-truth cleanup so refinement-completeness rules stop referencing only `.GOV/refinements/WP-{ID}.md` after the packet family migrated to co-located `task_packets/WP-{ID}/refinement.md`

### 2026.04.01.17 / GOV-CHANGE-20260401-17

- STATUS: APPLIED
- SUMMARY: captured the coder-side path-truth drift after finding legacy packet-read and refinement-read instructions still embedded in the coder protocol workflow
- CHANGE_TYPE: CODER_PATH_TRUTH_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-58`
- OUTCOME: the governance board now tracks the coder-side path-truth cleanup so pre-work guidance stops telling the coder to use legacy wildcard packet paths or only `.GOV/refinements/WP-{ID}.md` when the official packet family already supports co-located `packet.md` and `refinement.md`

### 2026.04.02.01 / GOV-CHANGE-20260402-01

- STATUS: APPLIED
- SUMMARY: completed `RGF-55` by removing stale hard-coded refinement-path wording from orchestrator outputs and aligning the shared path docs with the current folder packet family
- CHANGE_TYPE: PATH_TRUTH_ALIGNMENT_PATCH
- DRIVER_EVIDENCE:
  - `RGF-55`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: operator-facing orchestrator guidance now references the resolved current refinement path instead of hard-coding `.GOV/refinements/WP-{ID}.md`, and the shared protocol/invariant docs now describe the folder packet/refinement layout as current truth while still naming the legacy flat layout explicitly as compatibility-only

### 2026.04.02.02 / GOV-CHANGE-20260402-02

- STATUS: APPLIED
- SUMMARY: completed `RGF-57` and `RGF-58` by aligning validator and coder protocol path guidance to the current folder packet family while preserving legacy flat compatibility wording
- CHANGE_TYPE: ROLE_PROTOCOL_PATH_TRUTH_ALIGNMENT_PATCH
- DRIVER_EVIDENCE:
  - `RGF-57`
  - `RGF-58`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: validator refinement review now names the official current refinement path instead of only the flat legacy path, coder pre-work guidance now reads the current packet/refinement layout first and treats flat files as explicit compatibility-only paths, and the stale wildcard packet-read examples are removed from coder workflow instructions

### 2026.04.02.03 / GOV-CHANGE-20260402-03

- STATUS: APPLIED
- SUMMARY: completed `RGF-53` and `RGF-56` by surfacing the `2026-04-01` packet-law bundle in the shared operator docs and orchestrator packet-create/resume flow
- CHANGE_TYPE: PACKET_LAW_VISIBILITY_ALIGNMENT_PATCH
- DRIVER_EVIDENCE:
  - `RGF-53`
  - `RGF-56`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/START_HERE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: shared/operator-facing docs now state that packet creation activates the `2026-04-01` law bundle rather than just scaffolding files, and orchestrator packet-create/resume output now surfaces the active packet format, data-contract posture, handoff/report rigor profile, anti-vibe and signed-scope-debt consequences, and data-contract proof obligations so new governed lanes do not start blind to the checks that will later enforce closure truth

### 2026.04.02.04 / GOV-CHANGE-20260402-04

- STATUS: APPLIED
- SUMMARY: completed `RGF-52` and `RGF-54` by making data-contract activation or waiver an explicit packet decision, enforcing it at claim and validator closeout, adding end-to-end regression coverage for the new packet family, and recording the grandfathered legacy packet-family compatibility surface explicitly
- CHANGE_TYPE: DATA_CONTRACT_DECISION_AND_PACKET_LAW_REGRESSION_PATCH
- DRIVER_EVIDENCE:
  - `RGF-52`
  - `RGF-54`
  - `AUDIT-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1-SMOKETEST-STARTUP-REVIEW`
  - `SMOKETEST-REVIEW-20260331-PROJECT-PROFILE-EXTENSION-REGISTRY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/data-contract-lib.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles_shared/tests/data-contract-lib.test.mjs`
  - `.GOV/roles_shared/tests/new-packet-law-regression.test.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles_shared/docs/DEPRECATION_SUNSET_PLAN.md`
  - `.GOV/roles_shared/records/COMPATIBILITY_SHIM_LEDGER.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `PACKET_FORMAT_VERSION >= 2026-04-01` packets now carry an explicit `DATA_CONTRACT_DECISION` that must either activate the LLM-first data contract with reviewable evidence or explicitly waive it as not data-bearing; claim and validator closeout gates reject mismatched or conflicted waivers, regression coverage now proves the active-vs-waived-vs-grandfathered behavior end-to-end, and the older packet family remains explicitly tracked as an `ACTIVE_COMPAT` governance shim instead of an undocumented implicit exception

### 2026.04.02.05 / GOV-CHANGE-20260402-05

- STATUS: APPLIED
- SUMMARY: recorded and fixed the permanent-worktree reseed helper bug discovered during the `user_ilja` refresh sequence, where the helper failed on a checked-out target branch and then falsely treated expected `.GOV` junction drift as a dirty-worktree failure
- CHANGE_TYPE: TOPOLOGY_RESEED_HELPER_MAINTENANCE_CAPTURE
- DRIVER_EVIDENCE:
  - `RGF-59`
  - `MAINT-20260402-PERMANENT-WORKTREE-RESEED-HELPER`
- SURFACES:
  - `.GOV/roles_shared/scripts/topology/reseed-permanent-worktree-from-main.mjs`
  - `.GOV/roles_shared/scripts/topology/git-topology-lib.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: `just reseed-permanent-worktree-from-main wt-ilja ...` now updates the checked-out permanent branch safely with `checkout -B` semantics instead of trying to force-move the live branch ref, and the helper's cleanliness gate now checks for non-`.GOV` dirt so expected governance-junction replacement in permanent non-main worktrees does not cause a false failure after a successful reseed

### 2026.04.03.01 / GOV-CHANGE-20260403-01

- STATUS: APPLIED
- SUMMARY: captured new governance follow-ons from the storage-trait smoketest closeout review covering terminal runtime residue, pre-implementation packet validity, broker-safe closeout, and review-verdict deduplication
- CHANGE_TYPE: STORAGE_TRAIT_SMOKETEST_FOLLOW_ON_CAPTURE
- DRIVER_EVIDENCE:
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/Audits/smoketest/AUDIT_20260403_STORAGE_TRAIT_PURITY_V1_SMOKETEST_CLOSEOUT_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-60`
  - `RGF-61`
  - `RGF-62`
  - `RGF-63`
- OUTCOME: the governance board now tracks the four highest-value follow-ons surfaced by the storage-trait closeout review, with `RGF-60` activated first because the current terminal runtime surface still carries stale residue after successful contained-main closure

### 2026.04.03.02 / GOV-CHANGE-20260403-02

- STATUS: APPLIED
- SUMMARY: completed `RGF-60` by cleaning terminal runtime projection residue during final-lane closeout and stamping validator-of-record truth from the governed lane
- CHANGE_TYPE: TERMINAL_CLOSEOUT_RUNTIME_RESIDUE_PATCH
- DRIVER_EVIDENCE:
  - `RGF-60`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: terminal closeout now resolves validator-of-record values from governed receipts and the active final-lane actor, runtime projection writes those values into `RUNTIME_STATUS.json`, and `Validated (...)` packet states now clear stale `active_role_sessions` and touched-file residue instead of leaving misleading live-session artifacts behind after closure

### 2026.04.03.03 / GOV-CHANGE-20260403-03

- STATUS: APPLIED
- SUMMARY: completed `RGF-61` by making orchestrator-managed packet claim validity fail closed before implementation starts
- CHANGE_TYPE: PRE_IMPLEMENTATION_PACKET_CLAIM_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-61`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/tests/new-packet-law-regression.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: orchestrator-managed packets with an assigned coder now leave creation with governed session-policy claim fields, and `READY_FOR_DEV` packets fail claim-check immediately if those fields are still unclaimed instead of surfacing only after implementation hardens

### 2026.04.03.04 / GOV-CHANGE-20260403-04

- STATUS: APPLIED
- SUMMARY: completed `RGF-62` by making final-lane closeout self-settle stale WP-scoped session-control state and tolerate only the current Integration Validator self-run
- CHANGE_TYPE: FINAL_LANE_CLOSEOUT_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-62`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles/validator/scripts/lib/integration-validator-closeout-lib.mjs`
  - `.GOV/roles/validator/checks/integration-validator-closeout-check.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles/validator/tests/integration-validator-closeout-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: closeout entrypoints now self-settle stale WP-scoped session-control rows before evaluation, and the closeout bundle no longer self-collides on the Integration Validator's own in-flight broker command while still failing on foreign or extra active runs

### 2026.04.03.05 / GOV-CHANGE-20260403-05

- STATUS: APPLIED
- SUMMARY: completed `RGF-63` by deduplicating decisive validator outcomes per review round and collapsing authoritative assessment surfaces
- CHANGE_TYPE: REVIEW_RECEIPT_DEDUPLICATION
- DRIVER_EVIDENCE:
  - `RGF-63`
  - `AUDIT-20260403-STORAGE-TRAIT-PURITY-V1-SMOKETEST-CLOSEOUT-REVIEW`
  - `SMOKETEST-REVIEW-20260403-STORAGE-TRAIT-PURITY-V1`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-communication-health-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: duplicate decisive validator approvals or failures for the same review round now fail closed before they enter the receipt ledger, historical duplicate decisive assessments collapse into one authoritative surface for resume/projection consumers, and packet remediation text now points at the authoritative latest validator receipt instead of a raw noisy stream

### 2026.04.05.01 / GOV-CHANGE-20260405-01

- STATUS: APPLIED
- SUMMARY: completed Wave 1 (`RGF-64` through `RGF-67`) by finishing one-hop relay dispatch, typed route payloads, computed WP spans, and microtask-first resume surfaces
- CHANGE_TYPE: WORKFLOW_EFFICIENCY_AND_LANE_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-64`
  - `RGF-65`
  - `RGF-66`
  - `RGF-67`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles/orchestrator/scripts/orchestrator-steer-next.mjs`
  - `.GOV/roles/orchestrator/scripts/lib/manual-relay-envelope-lib.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-report.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-microtask-lib.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-microtask-lib.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles/orchestrator/tests/manual-relay-envelope-lib.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-68`
  - `RGF-69`
  - `RGF-70`
  - `RGF-71`
  - `RGF-72`
  - `RGF-73`
- OUTCOME: orchestrator-managed relay no longer requires a separate start turn and later steer turn for the same routine wakeup, governed prompts now carry typed route payloads instead of generic resume prose, `wp-timeline` computes control-command and review-exchange spans on top of the merged event stream, manual relay remains structured and first-class, continuation waivers are honored mechanically, and the compact role resume surfaces now expose declared active/next microtasks so coder and validator work can continue at MT granularity instead of broad WP guesswork

### 2026.04.05.02 / GOV-CHANGE-20260405-02

- STATUS: APPLIED
- SUMMARY: completed Wave 2 authority and validator hardening (`RGF-68` through `RGF-70`) by projecting milestone/task-board truth from shared helpers and upgrading validator law to `SPLIT_DIFF_SCOPED_RIGOR_V4`
- CHANGE_TYPE: AUTHORITY_REDUCTION_AND_VALIDATOR_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-68`
  - `RGF-69`
  - `RGF-70`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-05`
  - `SMOKE-FIND-20260404-06`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-authority-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/packet-runtime-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-review-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/ensure-wp-communications.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/validator-report-profile-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/computed-policy-gate-lib.mjs`
  - `.GOV/roles_shared/checks/session-policy-check.mjs`
  - `.GOV/roles/validator/checks/validator-packet-complete.mjs`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/templates/WP_RUNTIME_STATUS_TEMPLATE.json`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/tests/packet-runtime-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-review-projection-lib.test.mjs`
  - `.GOV/roles_shared/tests/ensure-wp-communications.test.mjs`
  - `.GOV/roles_shared/tests/wp-communication-health-lib.test.mjs`
  - `.GOV/roles_shared/tests/computed-policy-gate-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-71`
  - `RGF-72`
  - `RGF-74`
  - `RGF-75`
- OUTCOME: runtime and review projections now stamp derived milestone and task-board truth from one authority layer instead of scattered status mappers, contract-heavy direct-review lanes no longer stay pinned to stale bootstrap checkpoints once later validator proof exists, and new packets now default to `SPLIT_DIFF_SCOPED_RIGOR_V4`, forcing explicit primitive-retention, shared-surface, and current-main interaction evidence for stronger medium/high-risk closure audits

### 2026.04.05.03 / GOV-CHANGE-20260405-03

- STATUS: APPLIED
- SUMMARY: completed Wave 2 smoketest ledger hardening (`RGF-73`) by converting smoke reviews from narrative-only postmortems into stable finding and positive-control ledgers linked back to board items
- CHANGE_TYPE: RECORDKEEPING_AND_POSTMORTEM_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-73`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-07`
- SURFACES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/Audits/smoketest/AUDIT_20260404_PARALLEL_WP_ACP_STEERING_RECOVERY_REVIEW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-71`
  - `RGF-72`
  - `RGF-74`
  - `RGF-75`
- OUTCOME: smoketest reviews now have stable `SMOKE-FIND-*` and `SMOKE-CONTROL-*` surfaces, board items can cite exact smoke findings instead of only whole audit documents, and the recovery audit now records both failure linkage and positive controls in a mechanically traceable format

### 2026.04.05.04 / GOV-CHANGE-20260405-04

- STATUS: APPLIED
- SUMMARY: completed Wave 3 hygiene and runtime-ownership hardening (`RGF-71`, `RGF-72`, `RGF-74`) by enforcing external artifact law, reclaiming registry-owned governed terminals, and moving work-packet naming into a compatibility-safe resolver layer
- CHANGE_TYPE: RUNTIME_HYGIENE_AND_PATH_AUTHORITY_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-71`
  - `RGF-72`
  - `RGF-74`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/scripts/lib/artifact-hygiene-lib.mjs`
  - `.GOV/roles_shared/scripts/topology/artifact-hygiene-check.mjs`
  - `.GOV/roles_shared/scripts/topology/artifact-cleanup.mjs`
  - `.GOV/roles/validator/checks/validator-git-hygiene.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs`
  - `.GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`
  - `.GOV/roles_shared/scripts/lib/wp-communications-lib.mjs`
  - `.GOV/roles_shared/schemas/WP_RUNTIME_STATUS.schema.json`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-75`
- OUTCOME: repo-local `target/` directories and stale non-canonical artifact folders now fail closed and are cleaned mechanically, governed system-terminal sessions record ownership and can be reclaimed without touching unrelated operator terminals, and high-authority work-packet helpers/docs now resolve through `runtime-paths.mjs` with `work_packets` as the canonical logical name while legacy `.GOV/task_packets/` storage remains read-compatible during the migration window

### 2026.04.05.05 / GOV-CHANGE-20260405-05

- STATUS: APPLIED
- SUMMARY: closed `RGF-75` by evaluating branch topology after Wave 3 and confirming that `main` remains the only required stable product integration branch
- CHANGE_TYPE: TOPOLOGY_POLICY_EVALUATION
- DRIVER_EVIDENCE:
  - `RGF-75`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/docs/REPO_RESILIENCE.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- OUTCOME: after artifact enforcement and worktree-hygiene hardening, `handshake_main` is clean, no repo-local `target/` remains, and no additional stable product integration branch is justified; `main` stays canonical and branch-topology expansion remains closed unless future evidence shows `main` becoming operationally unsuitable again

### 2026.04.05.06 / GOV-CHANGE-20260405-06

- STATUS: APPLIED
- SUMMARY: registered the next governance follow-on tranche (`RGF-76` through `RGF-85`) so the post-wave roadmap is explicit before implementation begins
- CHANGE_TYPE: ROADMAP_AND_SEQUENCING_UPDATE
- DRIVER_EVIDENCE:
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_CHANGELOG.md`
- FOLLOW_ON_ITEMS:
  - `RGF-76`
  - `RGF-77`
  - `RGF-78`
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-82`
  - `RGF-83`
  - `RGF-84`
  - `RGF-85`
- OUTCOME: the board now carries the next mechanical remediation set covering microtask-state hardening, full span ledgers, relay cost compression, dual-track validation, failure-ledger expansion, authority shrink, archival layout, legacy path cleanup, artifact retention, and session-batch terminal ownership

### 2026.04.05.07 / GOV-CHANGE-20260405-07

- STATUS: APPLIED
- SUMMARY: completed `RGF-76` by converting microtask handling from loose declared scope checks into a governed execution state machine with explicit active and previous microtask semantics
- CHANGE_TYPE: MICROTASK_EXECUTION_STATE_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-76`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-04`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-microtask-lib.mjs`
  - `.GOV/roles_shared/scripts/wp/wp-receipt-append.mjs`
  - `.GOV/roles_shared/scripts/session/active-lane-brief-lib.mjs`
  - `.GOV/roles/coder/scripts/coder-next.mjs`
  - `.GOV/roles_shared/tests/wp-microtask-lib.test.mjs`
  - `.GOV/roles_shared/tests/wp-receipt-append.test.mjs`
  - `.GOV/roles_shared/tests/active-lane-brief.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-77`
  - `RGF-78`
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: kickoff-reviewed microtasks remain the active execution budget, overlap review shifts execution to the next declared microtask while retaining the previous reviewed slice explicitly, coder intent cannot jump ahead out of sequence, and validator overlap resolutions now fail closed unless they bind to the immediately previous governed microtask

### 2026.04.05.08 / GOV-CHANGE-20260405-08

- STATUS: APPLIED
- SUMMARY: completed `RGF-77` by upgrading `wp-timeline` from a compact report view into a richer span ledger with stage-tagged control, token, review, and microtask execution windows
- CHANGE_TYPE: TIMELINE_AND_COST_LEDGER_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-77`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-03`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-report.mjs`
  - `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-78`
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: `just wp-timeline` now emits stage-tagged span rows with stable span ids, explicit token-command windows, review-exchange durations, and microtask execution spans, plus summary counts for span families and measured span coverage so relay and delay hot spots are attributable without rereading scattered ledgers

### 2026.04.05.09 / GOV-CHANGE-20260405-09

- STATUS: APPLIED
- SUMMARY: completed `RGF-78` by attaching measured relay burden and default lane policy to the timeline surface, then surfacing the cheaper-lane guidance in the operator workflow docs and lane-selection gate output
- CHANGE_TYPE: RELAY_COST_POLICY_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-78`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-01`
  - `SMOKE-FIND-20260404-02`
- SURFACES:
  - `.GOV/roles_shared/scripts/session/wp-timeline-lib.mjs`
  - `.GOV/roles_shared/scripts/session/wp-timeline-report.mjs`
  - `.GOV/roles_shared/tests/wp-timeline-lib.test.mjs`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-79`
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: `just wp-timeline` now reports relay command counts, relay token share, burden level, and a recommended lane, while signature-time operator guidance now states the default policy explicitly: use `MANUAL_RELAY` for small and medium WPs unless autonomous steering or multi-WP parallelism is clearly worth the extra prompt tax

### 2026.04.05.10 / GOV-CHANGE-20260405-10

- STATUS: APPLIED
- SUMMARY: completed `RGF-79` by making the validator split explicit for new medium/high V4 packets, with separate mechanical and spec-retention track verdicts enforced in both structure checks and computed closure policy
- CHANGE_TYPE: DUAL_TRACK_VALIDATOR_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-79`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-06`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/validator-report-profile-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles/validator/checks/validator-report-structure-check.mjs`
  - `.GOV/roles_shared/scripts/lib/computed-policy-gate-lib.mjs`
  - `.GOV/roles_shared/tests/computed-policy-gate-lib.test.mjs`
  - `.GOV/roles/validator/tests/validator-report-structure-check.test.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/orchestrator-next.mjs`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-80`
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: new packets default to `PACKET_FORMAT_VERSION=2026-04-05`, creation/resume output now surfaces the dual-track law, and medium/high V4 validator closure must explicitly prove both the mechanical closure track and the deep spec-retention/shared-surface/current-main track before PASS remains legal

### 2026.04.05.11 / GOV-CHANGE-20260405-11

- STATUS: APPLIED
- SUMMARY: completed `RGF-80` by turning smoketest reviews into a typed failure/control ledger shape, then hardening the audit skeleton and maintenance workflow around that richer schema
- CHANGE_TYPE: FAILURE_LEDGER_SCHEMA_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-80`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-07`
- SURFACES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/scripts/audit/generate-post-run-audit-skeleton.mjs`
  - `.GOV/roles_shared/tests/generate-post-run-audit-skeleton.test.mjs`
  - `.GOV/roles_shared/docs/GOVERNANCE_MAINTENANCE_WORKFLOW.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-81`
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: new smoketest reviews now declare typed `CATEGORY`, `ROLE_OWNER`, `SYSTEM_SCOPE`, and `FAILURE_CLASS` fields plus typed positive controls with `CONTROL_TYPE`, `What went well`, and `REGRESSION_GUARDS`, and the post-run audit skeleton emits those placeholders by default instead of leaving the structure half-narrative

### 2026.04.05.12 / GOV-CHANGE-20260405-12

- STATUS: APPLIED
- SUMMARY: completed `RGF-81` by collapsing duplicated task-board status truth and repo-root work-packet path fallback logic into shared authority helpers, then rewiring resume/governance consumers to read those helpers instead of re-implementing local variants
- CHANGE_TYPE: AUTHORITY_SURFACE_SHRINK
- DRIVER_EVIDENCE:
  - `RGF-81`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
  - `SMOKE-FIND-20260404-05`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/wp-authority-projection-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`
  - `.GOV/roles_shared/scripts/session/session-governance-state-lib.mjs`
  - `.GOV/roles_shared/scripts/lib/role-resume-utils.mjs`
  - `.GOV/roles_shared/tests/runtime-paths.test.mjs`
  - `.GOV/roles_shared/tests/session-governance-state-lib.test.mjs`
  - `.GOV/roles_shared/tests/role-resume-utils.test.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-83`
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: task-board terminal and active-status truth plus repo-root work-packet/task-board resolution now come from shared helpers, so resume/governance surfaces stop carrying their own packet-path fallback and board-status regex copies

### 2026.04.05.13 / GOV-CHANGE-20260405-13

- STATUS: APPLIED
- SUMMARY: completed `RGF-83` by sweeping high-authority guidance, templates, and user-facing governance messages so they describe the logical `work_packets` model first and only mention `.GOV/task_packets/` as current physical compatibility storage
- CHANGE_TYPE: PATH_LANGUAGE_ALIGNMENT
- DRIVER_EVIDENCE:
  - `RGF-83`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/templates/AI_WORKFLOW_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `.GOV/roles/validator/README.md`
  - `.GOV/roles_shared/docs/VALIDATOR_FILE_TOUCH_MAP.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/EVIDENCE_LEDGER.md`
  - `.GOV/roles_shared/docs/QUALITY_GATE.md`
  - `.GOV/roles/coder/agentic/AGENTIC_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/checks/gate-check.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
  - `.GOV/roles/orchestrator/scripts/wp-traceability-set.mjs`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-84`
  - `RGF-82`
  - `RGF-85`
- OUTCOME: the authoritative human-facing surfaces now teach “resolve Work Packets through the logical `work_packets` model, with `.GOV/task_packets/` as compatibility storage” instead of presenting `task_packets` as the conceptual source of truth

### 2026.04.05.14 / GOV-CHANGE-20260405-14

- STATUS: APPLIED
- SUMMARY: completed `RGF-84` by turning governed artifact cleanup into a policy-backed retention workflow with durable manifests, so closeout now records exactly what residue was removed versus retained under `Handshake Artifacts`
- CHANGE_TYPE: ARTIFACT_RETENTION_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-84`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/artifact-hygiene-lib.mjs`
  - `.GOV/roles_shared/scripts/topology/artifact-cleanup.mjs`
  - `.GOV/roles/validator/scripts/integration-validator-closeout-sync.mjs`
  - `.GOV/roles_shared/tests/artifact-hygiene-lib.test.mjs`
  - `.GOV/roles_shared/docs/ARTIFACT_RETENTION_POLICY.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/REPO_RESILIENCE.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-82`
  - `RGF-85`
- OUTCOME: manual `artifact-cleanup` and integration-validator closeout now emit JSON retention manifests under `../Handshake Artifacts/handshake-tool/artifact-retention/`, preserving durable cleanup evidence while keeping canonical artifact roots and non-reclaimable residue out of the auto-delete set

### 2026.04.05.15 / GOV-CHANGE-20260405-15

- STATUS: APPLIED
- SUMMARY: completed `RGF-82` by introducing a governed Work Packet lifecycle layout with reserved archive roots and archive-aware resolver support, without migrating existing packets or breaking active-path compatibility
- CHANGE_TYPE: WORK_PACKET_LIFECYCLE_LAYOUT
- DRIVER_EVIDENCE:
  - `RGF-82`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `.GOV/roles_shared/scripts/lib/runtime-paths.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
  - `.GOV/roles_shared/tests/runtime-paths.test.mjs`
  - `.GOV/roles_shared/docs/WORK_PACKET_LIFECYCLE_LAYOUT.md`
  - `.GOV/roles_shared/docs/PROJECT_INVARIANTS.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/task_packets/_archive/README.md`
  - `.GOV/task_packets/_archive/superseded/README.md`
  - `.GOV/task_packets/_archive/validated_closed/README.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-85`
- OUTCOME: the runtime-path resolver now understands reserved archive roots for `superseded` and `validated_closed` packets, packet creation ensures the lifecycle layout exists, and the repo carries explicit archive directories plus policy docs without forcing a risky bulk packet move

### 2026.04.05.16 / GOV-CHANGE-20260405-16

- STATUS: APPLIED
- SUMMARY: completed `RGF-85` by grouping registry-owned governed terminals under explicit terminal batch ids so closeout and manual reclaim can target only the intended governed run
- CHANGE_TYPE: TERMINAL_BATCH_OWNERSHIP_HARDENING
- DRIVER_EVIDENCE:
  - `RGF-85`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
  - `SMOKETEST-REVIEW-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY`
- SURFACES:
  - `Justfile`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles_shared/scripts/session/terminal-ownership-lib.mjs`
  - `.GOV/roles_shared/scripts/session/reclaim-owned-terminals.mjs`
  - `.GOV/roles/orchestrator/scripts/session-registry-status.mjs`
  - `.GOV/roles/orchestrator/scripts/session-reset-batch-launch-mode.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles_shared/tests/session-registry-lib.test.mjs`
  - `.GOV/roles_shared/tests/terminal-ownership-lib.test.mjs`
  - `.GOV/roles_shared/schemas/ROLE_SESSION_REGISTRY.schema.json`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_SESSION_ORCHESTRATION.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- OUTCOME: governed system-terminal launches now stamp `owned_terminal_batch_id` plus the active registry batch, manual reclaim defaults to `CURRENT_BATCH` and can optionally target `ALL_BATCHES` or one explicit `BATCH_ID`, and operator-visible status surfaces now expose the active batch id so only the intended governed terminal set is reclaimed

### 2026.04.05.17 / GOV-CHANGE-20260405-17

- STATUS: APPLIED
- SUMMARY: completed `RGF-86` by introducing an explicit per-role model-profile catalog into packet/stub law, launch/session-control enforcement, and claim/session policy checks
- CHANGE_TYPE: ROLE_MODEL_PROFILE_CATALOG
- DRIVER_EVIDENCE:
  - `RGF-86`
  - `AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW`
- SURFACES:
  - `justfile`
  - `.GOV/roles_shared/scripts/session/session-policy.mjs`
  - `.GOV/roles_shared/scripts/session/session-control-lib.mjs`
  - `.GOV/roles_shared/scripts/session/session-registry-lib.mjs`
  - `.GOV/roles/orchestrator/checks/orchestrator_gates.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet.mjs`
  - `.GOV/roles/orchestrator/scripts/create-task-packet-stub.mjs`
  - `.GOV/roles/orchestrator/scripts/launch-cli-session.mjs`
  - `.GOV/roles/orchestrator/scripts/session-control-command.mjs`
  - `.GOV/roles_shared/checks/task-packet-claim-check.mjs`
  - `.GOV/roles_shared/checks/session-policy-check.mjs`
  - `.GOV/roles/coder/checks/pre-work-check.mjs`
  - `.GOV/roles_shared/schemas/ROLE_SESSION_REGISTRY.schema.json`
  - `.GOV/roles_shared/schemas/SESSION_CONTROL_REQUEST.schema.json`
  - `.GOV/roles_shared/schemas/SESSION_LAUNCH_REQUEST.schema.json`
  - `.GOV/templates/TASK_PACKET_TEMPLATE.md`
  - `.GOV/templates/TASK_PACKET_STUB_TEMPLATE.md`
  - `.GOV/roles/orchestrator/ORCHESTRATOR_PROTOCOL.md`
  - `.GOV/roles/coder/CODER_PROTOCOL.md`
  - `.GOV/roles/validator/VALIDATOR_PROTOCOL.md`
  - `.GOV/roles_shared/docs/COMMAND_SURFACE_REFERENCE.md`
  - `.GOV/roles_shared/docs/ROLE_WORKFLOW_QUICKREF.md`
  - `.GOV/codex/Handshake_Codex_v1.4.md`
  - `.GOV/roles_shared/records/REPO_GOVERNANCE_REFACTOR_TASK_BOARD.md`
- FOLLOW_ON_ITEMS:
  - `RGF-87`
- OUTCOME: new packet families now record explicit role-model-profile ids, GPT remains the governed default, Claude Code Opus 4.6 Thinking Max is declared and auditable at packet level, and governed launch/session control fail closed instead of silently pretending unsupported provider runtime exists
