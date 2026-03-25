# Smoketest Review Template

Use this template for workflow-proof runs, smoke tests, or post-run reviews that must link into repo-governance task-board and changelog records.

## METADATA

- AUDIT_ID: <AUDIT-YYYYMMDD-<short-name>>
- SMOKETEST_REVIEW_ID: <SMOKETEST-REVIEW-YYYYMMDD-<short-name>>
- DATE_UTC: <YYYY-MM-DD>
- AUTHOR: <name/role>
- SCOPE:
  - <historical baseline reviewed>
  - <current recovery or smoke-test run reviewed>
  - <product code / branch / packet / runtime surfaces reviewed>
- RESULT:
  - PRODUCT_REMEDIATION: <PASS|FAIL|PARTIAL>
  - MASTER_SPEC_AUDIT: <PASS|FAIL|PARTIAL>
  - WORKFLOW_DISCIPLINE: <PASS|FAIL|PARTIAL>
  - MERGE_PROGRESSION: <PASS|FAIL|PARTIAL>
- KEY_COMMITS_REVIEWED:
  - <sha> <summary>
- EVIDENCE_SOURCES:
  - <audit paths, packet paths, runtime ledgers, code paths>
- RELATED_GOVERNANCE_ITEMS:
  - <RGF-... or NONE>
- RELATED_CHANGESETS:
  - <GOV-CHANGE-... or NONE>

---

## 1. Executive Summary

- <high-signal summary of what really succeeded and what failed>

## 2. What Needed To Be True In Product Code

- <master-spec-tight product work actually required>

## 3. Product Findings

- <remaining product-code gaps with reasons and code evidence>

## 4. Workflow / Protocol Failures

- Orchestrator:
  - <failures>
- Coder:
  - <failures>
- WP Validator:
  - <failures>
- Integration Validator:
  - <failures>

## 5. Systemic Failures

- <workflow, governance, or runtime patterns that failed across roles>

## 6. Communication Review

- <direct coder-validator traffic quality, missed exchanges, relay problems, notification issues>

## 7. ACP Runtime / Session Control Findings

- <broker, queue, session-control, topology, or closeout issues>

## 8. Governance Conflicts / Concerns

- <policy ambiguity, split truth, missing hard gates, record drift>

## 9. Product-Spec Audit Addendum

- <deeper product-only findings if the signed WP scope still leaves Master Spec gaps>

## 10. Remediation Recommendations

- <governance remediations>
- <product-code remediations>
- <role or workflow improvements>
- <template or documentation changes>

## 11. Command Log

- `<command>` -> <PASS|FAIL> (<notes>)
