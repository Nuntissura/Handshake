# Smoketest Review Template

Use this template for workflow-proof runs, recovery passes, and closeout reviews that must link into repo-governance task-board and changelog records.

## Live Document Model

This review is a LIVE DOCUMENT. It is created at WP activation time and roles append to it during execution:

- **Orchestrator** appends: broker failures, format issues, role boundary decisions, MT dispatch timestamps, relay overhead observations.
- **Coder** appends: compile errors per MT, timeout issues, scope questions, MT completion timestamps.
- **Validator** appends: review findings per MT, test results, negative proof, spec misalignments.
- At closeout, the Orchestrator compiles the final review using the live findings plus the post-smoketest rubric.
- Do NOT delegate the full review to a subagent that did not observe the run. Subagents produce plausible prose from narrated facts but do not independently verify claims against git diffs, session outputs, or actual code. This produces reviews that LOOK complete but contain factual errors.

## Claim Verification Requirement

Every smoketest review MUST cross-reference at least 3 claims against actual evidence:
- At least 1 claim verified against `git log` or `git diff` output
- At least 1 claim verified against session output JSONL (coder or validator messages)
- At least 1 claim verified against the product code (grep, file read, or test output)

Mark each verified claim with `[VERIFIED: <evidence source>]`. If a claim cannot be verified, mark it `[UNVERIFIED]` and explain why.

## Authoring Rules

- Separate product correctness from workflow/governance/runtime judgment.
- Link each review with stable `AUDIT_ID` and `SMOKETEST_REVIEW_ID`.
- When the review follows an earlier smoke review, name that lineage explicitly.
- When the review follows an earlier smoke review, include a short required subsection named `What Improved vs Previous Smoketest`.
- Include the required `Post-Smoketest Improvement Rubric` section using `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`.
- Include the required `Silent Failures, Command Surface Misuse, and Ambiguity Scan` section using `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`.
- If the rubric document is not open, this template remains authoritative. Do not omit or collapse the rubric or ambiguity-scan sections.
- Write `NONE` explicitly when a subsection truly has no findings. Do not leave sections blank.
- Do not write only a verdict summary. Capture the failure ledger, role review, runtime truth, positive controls, and concrete remediations.
- A closeout review should be honest about both what the WP fixed and what still remains adjacent debt outside the packet.
- If the packet or remediation touches data-bearing surfaces and declares an active data contract, assess SQL/PostgreSQL readiness, LLM-first readability/parseability, and Loom-intertwined structure explicitly rather than folding that judgment into generic product prose.
- Call out anti-vibe findings, accepted signed-scope debt, or shallow easy-surface work explicitly when they influenced the review. Do not leave those concerns implicit.
- Use typed failure-ledger categories and typed positive controls. Do not hide script/check defects, governance drift, or operator-UX ambiguity inside generic workflow prose.

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

## 5. Per-Microtask Breakdown

For each declared microtask, record:

| MT | Prompt Summary | Commit | Time Sent | Time Committed | Compile First Pass | Validator Flagged | Fix Cycles |
|---|---|---|---|---|---|---|---|
| MT-001 | <what was asked> | <sha> | <HH:MM> | <HH:MM> | YES/NO | YES/NO (which findings) | <count> |
| MT-002 | ... | ... | ... | ... | ... | ... | ... |

If microtasks were not used, write `MICROTASKS_NOT_USED` with reason. This is a regression signal if the WP had declared MTs.

## 6. Communication Trail Audit

List every inter-role message with timestamps and communication surface used:

| # | Time | From | To | Surface | Content Summary |
|---|---|---|---|---|---|
| 1 | HH:MM | ORCHESTRATOR | CODER | SEND_PROMPT | MT-001 instructions |
| 2 | HH:MM | CODER | ORCHESTRATOR | SESSION_SETTLE | MT-001 committed |
| ... | ... | ... | ... | ... | ... |

Surface values: `SEND_PROMPT` (raw ACP), `wp-review-request` (governed receipt), `wp-review-response` (governed receipt), `wp-notification` (governed notification), `THREAD.md` (append-only thread), `SESSION_SETTLE` (broker self-settle), `MANUAL_RELAY` (operator-brokered).

Assessment:
- GOVERNED_RECEIPT_COUNT: <number of wp-review-request/response/notification messages>
- RAW_PROMPT_COUNT: <number of raw SEND_PROMPT messages>
- GOVERNED_RATIO: <governed / total> (target: >50% for orchestrator-managed WPs)
- COMMUNICATION_VERDICT: <GOVERNED|MOSTLY_GOVERNED|IMPLICIT|NONE>

## 7. Structured Failure Ledger

Repeat this block for every material workflow, runtime, governance, or product finding.

### 7.1 <severity + short title>

- FINDING_ID: <SMOKE-FIND-YYYYMMDD-01>
- CATEGORY: <WORKFLOW_DISCIPLINE|ACP_RUNTIME|ROLE_ORCHESTRATOR|ROLE_CODER|ROLE_WP_VALIDATOR|ROLE_INTEGRATION_VALIDATOR|GOVERNANCE_CHECK|SCRIPT_OR_CHECK|GOVERNANCE_DRIFT|OPERATOR_UX|OUT_OF_SCOPE_WORK|STALLING|TOOLING|PRODUCT_SCOPE|TOKEN_COST|TIMELINE|TERMINAL_HYGIENE>
- ROLE_OWNER: <ORCHESTRATOR|CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR|OPERATOR|SHARED>
- SYSTEM_SCOPE: <LOCAL|CROSS_ROLE|CONTROL_PLANE>
- FAILURE_CLASS: <CHECK_FAILURE|SCRIPT_DEFECT|RUNTIME_TRUTH|STATUS_DRIFT|OUT_OF_SCOPE|STALL|COMMAND_SURFACE_MISUSE|UX_AMBIGUITY|TOKEN_WASTE|TERMINAL_LEAK|OTHER>
- SURFACE: <packet path / runtime surface / helper / session / role lane>
- SEVERITY: <HIGH|MEDIUM|LOW>
- STATUS: <OPEN|TRACKED|FIXED_DURING_RUN|MONITOR>
- RELATED_GOVERNANCE_ITEMS:
  - <RGF-... or NONE>
- REGRESSION_HOOKS:
  - <test, command, audit probe, or runtime evidence path>
- Evidence:
  - <receipt, code, runtime, git, or timeline evidence>
- What went wrong:
  - <concise failure description>
- Impact:
  - <what it blocked, slowed, or made ambiguous>
- Mechanical fix direction:
  - <gate, helper, template, projection, or lifecycle fix>

### 7.2 <repeat as needed>

## 8. Role Review

### 8.1 Orchestrator Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

### 8.2 Coder Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

### 8.3 WP Validator Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

### 8.4 Integration Validator Review

Strengths:

- <what worked>

Failures:

- <what failed>

Assessment:

- <overall role judgment>

## 9. Review Of Coder and Validator Communication

- <quality of direct review traffic, review loop shape, missed acknowledgements, relay concerns>
- <did the coder and validator communicate directly or only through orchestrator relay?>
- <were governed receipts (wp-review-request/response) used or was all communication through raw SEND_PROMPT?>

## 9a. Memory Discipline

- MEMORY_WRITES_BY_ROLE:
  - ORCHESTRATOR: <count or NONE>
  - CODER: <count or NONE>
  - WP_VALIDATOR: <count or NONE>
  - INTEGRATION_VALIDATOR: <count or NONE>
- MEMORY_WRITE_EVIDENCE:
  - <list each memory write with role, type (episodic/semantic/procedural), topic, and source command/artifact>
- DUAL_WRITE_COMPLIANCE: <YES|NO|PARTIAL> (both Claude memory and repo governance memory DB)
- MEMORY_VERDICT: <CLEAN|PARTIAL|NONE>
- Assessment:
  - <did each role that should have written memory actually do so?>
  - <were insights captured during the run or only at closeout?>
  - <was anything written to vendor-locked memory without dual-writing to repo?>

## 9b. Build Artifact Hygiene

- BUILD_TARGET_PATH: `<WORKSPACE_ROOT>/Handshake Artifacts` (resolve from topology; typically a sibling of the worktree root)
- BUILD_TARGET_CLEANED_BY: <CODER|WP_VALIDATOR|INTEGRATION_VALIDATOR|NONE|N/A>
- BUILD_TARGET_CLEANED_AT: <timestamp or N/A>
- BUILD_TARGET_STATE_AT_CLOSEOUT: <CLEAN|STALE|NOT_CHECKED>
- Assessment:
  - <did the responsible role clean the build target folder after compilation/testing?>
  - <were stale artifacts left behind that could confuse subsequent WP runs?>
  - <if N/A, explain why build artifacts were not produced>

## 10. ACP Runtime / Session Control Findings

- <broker, queue, session-control, topology, or closeout issues>
- <whether runtime truth was clean or repaired>
- <broker dispatch success rate: N successes / M attempts = X%>

## 11. Terminal Hygiene

- TERMINALS_LAUNCHED: <count>
- TERMINALS_CLOSED_ON_COMPLETION: <count>
- TERMINALS_CLOSED_ON_FAILURE: <count>
- TERMINALS_RECLAIMED_AT_CLOSEOUT: <count>
- STALE_BLANK_TERMINALS_REMAINING: <count>
- TERMINAL_HYGIENE_VERDICT: <CLEAN|PARTIAL|FAILED>

Assessment:
- <did terminals close automatically after sessions finished?>
- <were any blank/stale terminals left on the operator's desktop?>
- <what needs to change in the launch/reclaim mechanism?>

## 12. Governance Linkage and Board Mapping

- BOARD_LINKS:
  - <FINDING_ID -> RGF-... | NONE>
- CHANGESET_LINKS:
  - <FINDING_ID -> GOV-CHANGE-... | NONE>
- POLICY_OR_TEMPLATE_FOLLOWUPS:
  - <template/check/protocol/helper drift exposed by this review>

## 13. Positive Controls Worth Preserving

### 13.1 <short positive control title>

- CONTROL_ID: <SMOKE-CONTROL-YYYYMMDD-01>
- CONTROL_TYPE: <REGRESSION_GUARD|WORKFLOW_STABILITY|RUNTIME_TRUTH|OPERATOR_UX|PRODUCT_PROOF|COST_REDUCTION>
- SURFACE: <role lane / helper / packet law / runtime surface>
- What went well:
  - <the concrete behavior that should remain the baseline>
- Why it mattered:
  - <what worked and why it should remain the baseline>
- Evidence:
  - <test, receipt, command, code path, or audit evidence>
- REGRESSION_GUARDS:
  - <test, command, invariant, or runtime surface that should keep this control alive>

### 13.2 <repeat as needed>

## 14. Cost Attribution

Break down time and token cost by lifecycle phase:

| Phase | Time (min) | Orchestrator Tokens (est) | Notes |
|---|---|---|---|
| Refinement | <N> | <N or %> | <format iteration? discovery? research?> |
| Per-MT Coding (total) | <N> | <N or %> | <how many MT prompts? retries?> |
| Validation | <N> | <N or %> | <FAIL/fix cycles?> |
| Fix Cycle | <N> | <N or %> | <items fixed? coder turns?> |
| Closeout | <N> | <N or %> | <manual formatting? section ordering?> |
| Polling/Waiting | <N> | <N or %> | <how many poll cycles? fire-and-forget?> |
| TOTAL | <N> | <N or %> | |

If exact token counts are unavailable, use percentages of total estimated cost.

## 15. Comparison Table (vs Previous WP)

| Metric | Previous WP | This WP | Trend |
|---|---|---|---|
| Total lines changed | <N> | <N> | |
| Microtask count | <N> | <N> | |
| Compile errors (first pass) | <N> | <N> | |
| Validator findings | <N> | <N> | |
| Fix cycles | <N> | <N> | |
| Stubs discovered | <N> | <N> | |
| Governed receipts created | <N> | <N> | |
| Broker dispatch failures | <N> | <N> | |
| Stale terminals remaining | <N> | <N> | |
| Time to close (hours) | <N> | <N> | |

## 16. Remaining Product or Spec Debt

- <adjacent or broader debt that should stay visible even if the WP passed>

## 17. Post-Smoketest Improvement Rubric

### 17.1 Workflow Smoothness

- TREND: <IMPROVED|FLAT|REGRESSED>
- CURRENT_STATE: <LOW|MEDIUM|HIGH>
- NUMERIC_SCORE: <0-10> (0=fully manual/broken, 10=fully automated/clean)
- Evidence:
  - <operator burden, orchestration friction, runtime/status repair, topology drift>
- What improved:
  - <specific workflow friction removed relative to previous smoketest>
- What still hurts:
  - <specific workflow friction still present>
- Next structural fix:
  - <single highest-value governance/runtime fix>

### 17.2 Master Spec Gap Reduction

- TREND: <IMPROVED|FLAT|REGRESSED>
- CURRENT_STATE: <LOW|MEDIUM|HIGH>
- NUMERIC_SCORE: <0-10> (0=broad open gap surface, 10=all phase-critical gaps closed)
- Evidence:
  - <gaps closed, gaps still open, new adjacent debt surfaced>
- What improved:
  - <specific product/spec gap reduction relative to previous smoketest>
- What still hurts:
  - <remaining product/spec debt>
- Next structural fix:
  - <single highest-value next product/proof fix>

### 17.3 Token Cost Pressure

- TREND: <IMPROVED|FLAT|REGRESSED>
- CURRENT_STATE: <LOW|MEDIUM|HIGH>
- NUMERIC_SCORE: <0-10> (0=most tokens wasted on overhead, 10=nearly all tokens on productive work)
- Evidence:
  - <repeated prompts, operator clarifications, repair-heavy closeout, duplicate checks>
- What improved:
  - <specific sources of waste removed relative to previous smoketest>
- What still hurts:
  - <remaining sources of avoidable token/operator cost>
- Next structural fix:
  - <single highest-value cost-reduction fix>

## 18. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 18.1 Silent Failures / False Greens

- <where a surface looked healthy or complete before truth was actually settled>

### 18.2 Systematic Wrong Tool or Command Calls

- <wrong helper, wrong role surface, wrong command family, stale or invalid command usage>

### 18.3 Task and Path Ambiguity

- <ambiguous scope, ambiguous file ownership, ambiguous worktree/path source of truth, ambiguous packet wording>

### 18.4 Read Amplification / Governance Document Churn

- <repeated protocol rereads, repeated command-surface discovery, repeated path/status re-checking that signals ambiguity>

### 18.5 Hardening Direction

- <what should become a gate, prompt change, template change, canonical shortcut, or status surface>

## 19. Suggested Remediations

### Governance / Runtime

- <governance remediations>

### Product / Validation Quality

- <product or proof remediations>

### Documentation / Review Practice

- <template or documentation changes>

## 20. Command Log

- `<command>` -> <PASS|FAIL|PARTIAL> (<notes>)

## LIVE_FINDINGS_LOG (append-only during WP execution)

This section is append-only. Roles add findings as they occur during WP work.

Format: `- [TIMESTAMP] [ROLE] [CATEGORY] <finding>`

Example:
- [2026-04-06T01:10Z] [ORCHESTRATOR] [ACP_RUNTIME] Broker dispatch failed for SEND_PROMPT, retrying
- [2026-04-06T01:32Z] [CODER] [COMPILE_ERROR] MT-001 has 2 borrow errors in workflows.rs
- [2026-04-06T02:15Z] [WP_VALIDATOR] [CODE_REVIEW] FR-EVT-SESS-SPAWN-004 has no emit function — dead code
