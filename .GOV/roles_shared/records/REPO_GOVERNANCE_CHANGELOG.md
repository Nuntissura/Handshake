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
