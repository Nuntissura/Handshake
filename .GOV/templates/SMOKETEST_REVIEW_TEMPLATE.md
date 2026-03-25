# Smoketest Review Template

Use this template for workflow-proof runs, recovery passes, and closeout reviews that must link into repo-governance task-board and changelog records.

Authoring rules:

- Separate product correctness from workflow/governance/runtime judgment.
- Link each review with stable `AUDIT_ID` and `SMOKETEST_REVIEW_ID`.
- When the review follows an earlier smoke review, name that lineage explicitly.
- When the review follows an earlier smoke review, include a short required subsection named `What Improved vs Previous Smoketest`.
- Include the required `Post-Smoketest Improvement Rubric` section using `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`.
- Include the required `Silent Failures, Command Surface Misuse, and Ambiguity Scan` section using `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`.
- If the rubric document is not open, this template remains authoritative. Do not omit or collapse the rubric or ambiguity-scan sections.
- Write `NONE` explicitly when a subsection truly has no findings. Do not leave sections blank.
- Do not write only a verdict summary. Capture the failure inventory, role review, runtime truth, positive signals, and concrete remediations.
- A closeout review should be honest about both what the WP fixed and what still remains adjacent debt outside the packet.

## METADATA

- AUDIT_ID: <AUDIT-YYYYMMDD-<short-name>>
- SMOKETEST_REVIEW_ID: <SMOKETEST-REVIEW-YYYYMMDD-<short-name>>
- REVIEW_KIND: <RECOVERY|CLOSEOUT|PROOF_RUN|COMPARISON>
- DATE_UTC: <YYYY-MM-DD>
- AUTHOR: <name/role>
- HISTORICAL_BASELINE_PACKET: <WP-... or NONE>
- ACTIVE_RECOVERY_PACKET: <WP-... or NONE>
- LINEAGE_STATUS: <LIVE_SMOKETEST_BASELINE_PENDING|LIVE_SMOKETEST_BASELINE_RECOVERED|NONE>
- RELATED_PREVIOUS_REVIEWS:
  - <AUDIT_ID or NONE>
- SCOPE:
  - <historical or predecessor baseline reviewed>
  - <current WP / run reviewed>
  - <integrated branch / commit / runtime surfaces reviewed>
- RESULT:
  - PRODUCT_REMEDIATION: <PASS|FAIL|PARTIAL>
  - MASTER_SPEC_AUDIT: <PASS|FAIL|PARTIAL>
  - WORKFLOW_DISCIPLINE: <PASS|FAIL|PARTIAL>
  - ACP_RUNTIME_DISCIPLINE: <PASS|FAIL|PARTIAL>
  - MERGE_PROGRESSION: <PASS|FAIL|PARTIAL>
- KEY_COMMITS_REVIEWED:
  - <sha> <summary>
- EVIDENCE_SOURCES:
  - <audit paths, packet paths, runtime ledgers, control ledgers, code paths>
- RELATED_GOVERNANCE_ITEMS:
  - <RGF-... or NONE>
- RELATED_CHANGESETS:
  - <GOV-CHANGE-... or NONE>

---

## 1. Executive Summary

- <high-signal summary of what really succeeded and what failed>

## 2. Lineage and What This Run Needed To Prove

- <how this review relates to prior smoke reviews or audits>
- <the exact product truths that needed to become true>

### What Improved vs Previous Smoketest

- <the specific product gaps that are now closed relative to the prior smoketest review>
- <the specific workflow/runtime failures that improved, even if the workflow is still not fully clean>
- <if nothing improved, say that explicitly rather than skipping this subsection>

## 3. Product Outcome

- <what changed in product code>
- <whether the signed scope is closed>
- <what adjacent spec debt still remains, if any>

## 4. Timeline

- <key lifecycle moments from kickoff through closeout>

## 5. Failure Inventory

### 5.1 <severity + short title>

Evidence:

- <receipt, code, runtime, or git evidence>

Reason:

- <why it happened>

Impact:

- <what it blocked or weakened>

Judgment:

- <why this matters>

### 5.2 <repeat as needed>

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

### 6.2 Coder Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

### 6.3 WP Validator Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

### 6.4 Integration Validator Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

## 7. Review Of Coder and Validator Communication

- <quality of direct review traffic, review loop shape, missed acknowledgements, relay concerns>

## 8. ACP Runtime / Session Control Findings

- <broker, queue, session-control, topology, or closeout issues>
- <whether runtime truth was clean or repaired>

## 9. Governance Implications

- <policy ambiguity, split truth, missing hard gates, record drift, or confirmed follow-on items>

## 10. Positive Signals Worth Preserving

- <specific workflow or product behaviors that should remain the baseline>

## 11. Remaining Product or Spec Debt

- <adjacent or broader debt that should stay visible even if the WP passed>

## 12. Post-Smoketest Improvement Rubric

### 12.1 Workflow Smoothness

- TREND: <IMPROVED|FLAT|REGRESSED>
- CURRENT_STATE: <LOW|MEDIUM|HIGH>
- Evidence:
  - <operator burden, orchestration friction, runtime/status repair, topology drift>
- What improved:
  - <specific workflow friction removed relative to previous smoketest>
- What still hurts:
  - <specific workflow friction still present>
- Next structural fix:
  - <single highest-value governance/runtime fix>

### 12.2 Master Spec Gap Reduction

- TREND: <IMPROVED|FLAT|REGRESSED>
- CURRENT_STATE: <LOW|MEDIUM|HIGH>
- Evidence:
  - <gaps closed, gaps still open, new adjacent debt surfaced>
- What improved:
  - <specific product/spec gap reduction relative to previous smoketest>
- What still hurts:
  - <remaining product/spec debt>
- Next structural fix:
  - <single highest-value next product/proof fix>

### 12.3 Token Cost Pressure

- TREND: <IMPROVED|FLAT|REGRESSED>
- CURRENT_STATE: <LOW|MEDIUM|HIGH>
- Evidence:
  - <repeated prompts, operator clarifications, repair-heavy closeout, duplicate checks>
- What improved:
  - <specific sources of waste removed relative to previous smoketest>
- What still hurts:
  - <remaining sources of avoidable token/operator cost>
- Next structural fix:
  - <single highest-value cost-reduction fix>

## 13. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 13.1 Silent Failures / False Greens

- <where a surface looked healthy or complete before truth was actually settled>

### 13.2 Systematic Wrong Tool or Command Calls

- <wrong helper, wrong role surface, wrong command family, stale or invalid command usage>

### 13.3 Task and Path Ambiguity

- <ambiguous scope, ambiguous file ownership, ambiguous worktree/path source of truth, ambiguous packet wording>

### 13.4 Read Amplification / Governance Document Churn

- <repeated protocol rereads, repeated command-surface discovery, repeated path/status re-checking that signals ambiguity>

### 13.5 Hardening Direction

- <what should become a gate, prompt change, template change, canonical shortcut, or status surface>

## 14. Suggested Remediations

### Governance / Runtime

- <governance remediations>

### Product / Validation Quality

- <product or proof remediations>

### Documentation / Review Practice

- <template or documentation changes>

## 15. Command Log

- `<command>` -> <PASS|FAIL|PARTIAL> (<notes>)
