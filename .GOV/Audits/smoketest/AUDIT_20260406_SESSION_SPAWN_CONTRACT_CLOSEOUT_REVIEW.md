# AUDIT_20260406_SESSION_SPAWN_CONTRACT_CLOSEOUT_REVIEW

## METADATA

- AUDIT_ID: AUDIT-20260406-SESSION-SPAWN-CONTRACT-CLOSEOUT
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260406-SESSION-SPAWN-CONTRACT
- REVIEW_KIND: CLOSEOUT
- DATE_UTC: 2026-04-06
- AUTHOR: Orchestrator (Claude Opus 4.6)
- HISTORICAL_BASELINE_PACKET: NONE
- ACTIVE_RECOVERY_PACKET: NONE
- LINEAGE_STATUS: NONE
- RELATED_PREVIOUS_REVIEWS:
  - AUDIT-20260405-PRODUCT-GOVERNANCE-ARTIFACT-REGISTRY-CLOSEOUT
- SCOPE:
  - WP-1-Session-Spawn-Contract-v1 activation through closeout
  - Branch feat/WP-1-Session-Spawn-Contract-v1 at 300287d
  - Contained in main at 4b48f8c
  - Orchestrator-managed lane with Codex Spark 5.3 xhigh coder and Claude Code Opus 4.6 WP Validator
- RESULT:
  - PRODUCT_REMEDIATION: PASS
  - MASTER_SPEC_AUDIT: PASS
  - WORKFLOW_DISCIPLINE: PARTIAL
  - ACP_RUNTIME_DISCIPLINE: PARTIAL
  - MERGE_PROGRESSION: PASS
- KEY_COMMITS_REVIEWED:
  - c3487de MT-001 committed (initial session spawn request/response structs)
  - 04fce59 MT-001 fix pass (compile errors resolved)
  - ed5336e MT-002 committed (validate_spawn_request with INV-SPAWN-001/002, TRUST-003)
  - 31217bf MT-003 committed (FR-EVT-SESS-SPAWN events, cascade cancel)
  - 9c74a6d MT-004 committed (Role Mailbox AnnounceBack)
  - 300287d Validator fix pass (6 findings resolved in one commit)
  - 4b48f8c Merged to main
- EVIDENCE_SOURCES:
  - .GOV/task_packets/WP-1-Session-Spawn-Contract-v1/packet.md
  - .GOV/refinements/WP-1-Session-Spawn-Contract-v1.md
  - ACP broker session logs (coder session, validator session)
  - Coder branch feat/WP-1-Session-Spawn-Contract-v1 commit history
  - Main branch merge at 4b48f8c
- RELATED_GOVERNANCE_ITEMS:
  - RGF-89 (per-MT coder instructions / role boundary enforcement)
  - RGF-93 (completion notifications)
  - RGF-94 (feature discovery checkpoint)
- RELATED_CHANGESETS:
  - NONE

---

## 1. Executive Summary

The product code is correct: 1429 lines across 6 files, 17/17 tests pass, zero regressions. The Session Spawn Contract delivers SessionSpawnRequest/Response structs, validate_spawn_request with three invariant checks (INV-SPAWN-001/002, TRUST-003), five FR-EVT-SESS-SPAWN events, cascade cancel, and Role Mailbox AnnounceBack.

The workflow showed material improvement over the previous WP (Artifact Registry). Three things got better:

1. The microtask loop WORKED: the coder committed per-MT (4 commits), stopped for review as instructed, and the validator found 6 real issues that the coder fixed in one pass. The FAIL-to-PASS cycle proved incremental steering works.
2. The orchestrator did NOT edit product code. RGF-89 role boundary was honored. When compile errors were found, the orchestrator sent fix instructions to the coder via session-send instead of fixing directly.
3. Feature discovery (RGF-94) produced 4 new primitives, 2 new stubs, 3 matrix edges, and 6 UI controls discovered during refinement.

Three things still hurt:

1. ACP broker intermittent availability (~70% success rate, 3 failures in 10 dispatch attempts).
2. Closeout formatting still required manual orchestrator intervention (validator report format, clause closure matrix updates, VALIDATION_REPORTS section ordering).
3. Communication was ACP-prompt-level only, not governed-receipt-level. No WP_COMMUNICATIONS receipts, no review_request/response artifacts through the governed mailbox.

## 2. Lineage and What This Run Needed To Prove

This is the second WP activation after the 2026-04-04 parallel WP recovery audit and the first WP after the Artifact Registry closeout review (AUDIT-20260405). It needed to prove:

- The microtask loop can actually execute: coder commits per-MT, validator reviews per-MT, fix cycles are bounded.
- The orchestrator can honor role boundaries (RGF-89) instead of directly editing product code.
- The ACP broker session-send relay can carry structured per-MT prompts and validator findings back to the coder.
- The session spawn contract concept (a core backend primitive for governed session lifecycle) can be implemented as a bounded module.

### What Improved vs Previous Smoketest

- **Microtask loop executed**: The previous WP (Artifact Registry) had zero microtask structure — the coder did all work in one pass, no per-MT steering loop occurred. This WP had 4 explicit MTs with commits per MT, and the validator reviewed the full diff and found 6 real issues. The fix cycle was bounded to one pass.
- **Role boundary honored**: The previous WP had a role violation where the orchestrator directly edited governance_artifact_registry.rs to fix an import path. This WP sent compile fix instructions to the coder via session-send instead. RGF-89 was enforced.
- **Feature discovery worked**: The previous WP had zero feature discovery. This WP used RGF-94 to discover 4 new primitives, 2 new stubs, 3 matrix edges, and 6 UI controls.
- **Validator findings were genuine**: The WP Validator found dead code (FR-EVT-004 no emit function), a missing struct (CascadeCancelRecord), a race condition in test, and a cross-surface integration gap (FR whitelist for announce_back). These are real negative proof findings, not shallow PASS language.
- **ACP broker still intermittent**: Broker availability was ~70% (3 failures in 10 dispatches), similar to the previous WP. No improvement here.
- **Closeout still manual**: The orchestrator still had to manually format the validator report, update clause closure matrices, and fix VALIDATION_REPORTS section ordering. Same pain as the previous WP.

## 3. Product Outcome

Product code added (1429 lines across 6 files):

- SessionSpawnRequest and SessionSpawnResponse structs defining the governed session spawn contract.
- validate_spawn_request function enforcing three invariant checks:
  - INV-SPAWN-001: request field completeness
  - INV-SPAWN-002: role/worktree compatibility
  - TRUST-003: trust boundary validation
- Five FR-EVT-SESS-SPAWN events covering the session spawn lifecycle.
- Cascade cancel mechanism for propagating cancellation through spawn chains.
- Role Mailbox AnnounceBack for structured role-to-orchestrator acknowledgement.

Signed scope is closed. 17/17 tests pass.

Adjacent spec debt:

- 2 new stubs were identified during feature discovery for downstream WPs (cascade cancel persistence, announce_back routing).
- 3 matrix edges were surfaced for cross-surface integration (FR whitelist for announce_back, event routing for cascade cancel, session spawn request validation in the integration test harness).
- No database-backed implementation exists. Only in-memory test implementations are provided. Persistence is deferred.

## 4. Timeline

| Time (UTC) | Event |
|---|---|
| ~01:00 | Signature recorded |
| ~01:05 | Packet created, coder worktree ready |
| ~01:10 | Coder session started via ACP broker (intermittent, needed retry) |
| ~01:15 | MT-001 prompt sent with per-MT instructions (RGF-89 enforced) |
| ~01:32 | MT-001 committed (c3487de), coder STOPPED for review |
| ~01:35 | Orchestrator found compile errors, sent fix instructions to coder via session-send (NOT fixing directly — RGF-89 role boundary honored) |
| ~01:42 | MT-001 fixed (04fce59) |
| ~01:45 | MT-002 prompt sent |
| ~01:50 | MT-002 committed (ed5336e) |
| ~01:52 | MT-003 prompt sent |
| ~01:55 | MT-003 committed (31217bf) |
| ~01:57 | MT-004 prompt sent |
| ~02:10 | MT-004 committed (9c74a6d) |
| ~02:15 | Validator started, FAIL verdict with 6 findings |
| ~02:20 | Fix instructions sent to coder (6 items via SEND_PROMPT) |
| ~02:50 | Coder fixed and committed (300287d amended) |
| ~03:00 | Tests pass 17/17 on coder branch |
| ~03:10 | Merged to main at 4b48f8c |
| ~03:20 | Closeout, sessions cancelled, terminals reclaimed |

Estimated total elapsed: ~2.3 hours. Estimated orchestrator token cost: MEDIUM (less than previous WP due to per-MT prompts and no refinement format iteration).

## 5. Structured Failure Ledger

### 5.1 MEDIUM — Microtask Communication Was Implicit, Not Governed

- FINDING_ID: SMOKE-FIND-20260406-01
- CATEGORY: WORKFLOW_DISCIPLINE
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: UX_AMBIGUITY
- SURFACE: WP_COMMUNICATIONS / ACP broker prompt relay / governed mailbox
- SEVERITY: MEDIUM
- STATUS: TRACKED
- RELATED_GOVERNANCE_ITEMS:
  - RGF-89
  - RGF-93
- REGRESSION_HOOKS:
  - Check for WP_COMMUNICATIONS receipt files after each MT commit
  - Check for review_request/response artifacts in governed mailbox
- Evidence:
  - The coder received per-MT prompts and stopped between MTs (4 commits: c3487de, ed5336e, 31217bf, 9c74a6d).
  - The validator findings were routed back to the coder via session-send (not orchestrator edits).
  - No structured WP_COMMUNICATIONS receipts were created.
  - No review_request/response messages were sent through the governed mailbox.
  - RGF-93 completion notifications did not trigger orchestrator checks.
  - Communication was ACP-prompt-level only.
- What went wrong:
  - The microtask loop executed mechanically (coder committed, orchestrator relayed, validator reviewed) but none of the governed communication surfaces were used. The orchestrator relayed instructions through raw ACP broker SEND_PROMPT calls, which have no structured receipt, no acknowledgement, and no governed audit trail. This means the workflow improvement is real but fragile — it depends on the orchestrator remembering to relay, not on a governed mechanism enforcing it.
- Impact:
  - No auditable communication trail between coder and validator. If the orchestrator had dropped a relay, there would be no evidence. Future WPs cannot rely on this implicit pattern at scale.
- Mechanical fix direction:
  - After each MT commit, the coder should call `just wp-review-request` to create a structured receipt. The validator should call `just wp-review-response` to acknowledge and steer. The orchestrator should relay review responses back to the coder. RGF-93 completion notifications should trigger the orchestrator to check results instead of polling.

### 5.2 MEDIUM — ACP Broker Intermittent Availability

- FINDING_ID: SMOKE-FIND-20260406-02
- CATEGORY: ACP_RUNTIME
- ROLE_OWNER: ORCHESTRATOR
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: RUNTIME_TRUTH
- SURFACE: ACP broker / SEND_PROMPT dispatch
- SEVERITY: MEDIUM
- STATUS: TRACKED
- RELATED_GOVERNANCE_ITEMS:
  - RGF-93
- REGRESSION_HOOKS:
  - Monitor broker dispatch success rate per WP
  - Alert if success rate drops below 80%
- Evidence:
  - 3 failures in 10 dispatch attempts (~70% success rate).
  - Every SEND_PROMPT required a retry pattern.
  - Broker availability was similar to the previous WP (no improvement).
- What went wrong:
  - The ACP broker process intermittently failed to accept SEND_PROMPT dispatches. The orchestrator had to retry each dispatch, adding latency and token cost. The root cause is unclear — the broker may be dropping connections under load, or the session registration may be stale.
- Impact:
  - Each retry added ~30s latency and wasted orchestrator context on error handling. Over 10 dispatches, this accumulated to ~1.5 minutes of dead time and repeated error-handling prompts.
- Mechanical fix direction:
  - Investigate broker connection pooling or heartbeat. Add automatic retry with exponential backoff at the dispatch layer so the orchestrator does not need to manually retry. Consider RGF-93 completion notification as a fallback signal path.

### 5.3 LOW — Orchestrator Role Boundary Honored (IMPROVED)

- FINDING_ID: SMOKE-FIND-20260406-03
- CATEGORY: ROLE_ORCHESTRATOR
- ROLE_OWNER: ORCHESTRATOR
- SYSTEM_SCOPE: LOCAL
- FAILURE_CLASS: OTHER
- SURFACE: Orchestrator role lane / product code boundary
- SEVERITY: LOW
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - RGF-89
- REGRESSION_HOOKS:
  - gov-check should flag any orchestrator commits that touch product code
  - Audit trail should show SEND_PROMPT for fix instructions, not direct edits
- Evidence:
  - At ~01:35, orchestrator found compile errors in MT-001 output.
  - Instead of editing code directly (as in the previous WP), the orchestrator sent fix instructions to the coder via session-send.
  - The coder fixed the errors and committed (04fce59).
  - RGF-89 role boundary was honored throughout the run.
- What went wrong:
  - Nothing went wrong. This is a tracked improvement over the previous WP where the orchestrator directly edited governance_artifact_registry.rs (a role boundary violation). Recorded here for lineage tracking.
- Impact:
  - Positive. The role boundary enforcement worked as designed.
- Mechanical fix direction:
  - No fix needed. Continue enforcing RGF-89. Consider adding a hard gate that rejects orchestrator commits touching product source files.

### 5.4 MEDIUM — Closeout Formatting Still Manual

- FINDING_ID: SMOKE-FIND-20260406-04
- CATEGORY: WORKFLOW_DISCIPLINE
- ROLE_OWNER: ORCHESTRATOR
- SYSTEM_SCOPE: LOCAL
- FAILURE_CLASS: TOKEN_WASTE
- SURFACE: Closeout lifecycle / validator report formatting / clause closure matrix
- SEVERITY: MEDIUM
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - Measure orchestrator token cost during closeout phase
  - Check whether VALIDATION_REPORTS section ordering is automated
- Evidence:
  - The validator report format required manual orchestrator intervention.
  - Clause closure matrix updates were manual.
  - VALIDATION_REPORTS section ordering required manual correction.
  - The first validator report (FAIL) poisoned parseSectionField results, requiring manual cleanup before the PASS report could be appended.
- What went wrong:
  - The closeout phase still requires the orchestrator to manually format, order, and repair packet sections. The parseSectionField utility does not handle multiple validator reports (FAIL then PASS) gracefully — the first FAIL report's sections bleed into subsequent parsing. This is the same pain point as the previous WP.
- Impact:
  - Closeout consumed disproportionate orchestrator tokens relative to the actual product work. The orchestrator spent ~20 minutes on formatting and section ordering that should be mechanical.
- Mechanical fix direction:
  - Build a `just wp-closeout-format` helper that takes the validator report, appends it to the packet in the correct position, updates the clause closure matrix, and handles the FAIL-then-PASS report sequence. The parseSectionField utility needs a mode that scopes to the latest report rather than the first match.

### 5.5 MEDIUM — Feature Discovery Checkpoint Worked (IMPROVED)

- FINDING_ID: SMOKE-FIND-20260406-05
- CATEGORY: PRODUCT_SCOPE
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: OTHER
- SURFACE: RGF-94 feature discovery / refinement surface
- SEVERITY: MEDIUM
- STATUS: MONITOR
- RELATED_GOVERNANCE_ITEMS:
  - RGF-94
- REGRESSION_HOOKS:
  - Check that feature discovery checkpoint produces non-empty results for each WP
  - Verify discovered items are tracked in downstream WP stubs or backlog
- Evidence:
  - During refinement, RGF-94 feature discovery surfaced: 4 new primitives, 2 new stubs, 3 matrix edges, 6 UI controls.
  - The previous WP (Artifact Registry) had zero feature discovery.
- What went wrong:
  - Nothing went wrong. This is a tracked improvement. Recorded here because the discovered items create downstream tracking obligations: the 2 stubs and 3 matrix edges need to appear in future WP scopes or be explicitly deferred.
- Impact:
  - Positive. Feature discovery reduced the risk of hidden adjacent debt. The 2 stubs (cascade cancel persistence, announce_back routing) and 3 matrix edges (FR whitelist, event routing, integration test harness) are now visible.
- Mechanical fix direction:
  - Ensure discovered stubs and matrix edges are captured in the backlog or a dedicated discovery ledger. Consider making `just wp-closeout` check that all discovered items have a tracking destination.

### 5.6 LOW — Terminal Reclamation Returned Zero

- FINDING_ID: SMOKE-FIND-20260406-06
- CATEGORY: ACP_RUNTIME
- ROLE_OWNER: ORCHESTRATOR
- SYSTEM_SCOPE: LOCAL
- FAILURE_CLASS: RUNTIME_TRUTH
- SURFACE: Terminal reclamation / session cleanup
- SEVERITY: LOW
- STATUS: MONITOR
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - Check reclaimed_count after each closeout
  - Verify terminals are still alive at reclaim time
- Evidence:
  - Terminal reclamation attempted at ~03:20 but `reclaimed_count=0`.
  - Terminals had already exited before the reclaim ran.
- What went wrong:
  - The governed sessions (coder and validator) self-terminated before the orchestrator's closeout reclaim step ran. The reclaim mechanism works correctly — it just found nothing to reclaim. This is benign but means the reclaim step is currently a no-op in the common case where sessions self-terminate.
- Impact:
  - Minimal. The previous WP had terminals cluttering the operator's desktop because they were never reclaimed. This WP's terminals self-exited, which is better behavior, but it means the reclaim mechanism was not actually exercised.
- Mechanical fix direction:
  - Consider tracking terminal exit timestamps in the session registry so the orchestrator can distinguish "already exited" from "still running but reclaim failed." This makes the closeout log more informative.

## 6. Role Review

### 6.1 Orchestrator Review

Strengths:

- Honored RGF-89 role boundary throughout the run. When compile errors were found in MT-001, sent fix instructions to the coder via session-send instead of editing product code directly. This is a material improvement over the previous WP.
- Used per-MT prompts (RGF-89 enforced) to give the coder bounded, clear instructions for each microtask.
- Used fire-and-forget dispatch pattern, reducing polling overhead compared to the previous WP.
- Relayed validator findings (6 items) back to the coder via SEND_PROMPT, maintaining the role separation.

Failures:

- Still performed manual closeout formatting: validator report format, clause closure matrix updates, VALIDATION_REPORTS section ordering all required manual intervention.
- The first validator report (FAIL) poisoned parseSectionField results, requiring manual cleanup.
- Did not use governed communication surfaces (WP_COMMUNICATIONS receipts, review_request/response messages). All communication was through raw ACP broker SEND_PROMPT calls.

Assessment:

- IMPROVED. The orchestrator's behavior was materially better than the previous WP. Role boundary honored, per-MT prompts used, fire-and-forget dispatch reduced polling. The remaining pain is concentrated in closeout formatting, which is a tooling gap rather than a discipline gap.

### 6.2 Coder Review

Strengths:

- Followed MT-001 through MT-004 with commits per MT (c3487de, ed5336e, 31217bf, 9c74a6d). This is the first WP where the microtask loop actually executed as designed.
- Stopped for review as instructed between MTs.
- Fixed all 6 validator findings in one pass (300287d). The fix cycle was bounded and efficient.
- 17/17 tests pass on the coder branch.

Failures:

- MT-001 had compile errors on the first pass (fixed at 04fce59). Fewer than the previous WP but still present.
- No structured WP_COMMUNICATIONS receipts were created. The coder relied on the orchestrator to relay all communication.

Assessment:

- GOOD. The coder's behavior was correct and disciplined. Per-MT commits, stopped for review, fixed validator findings in one pass. The compile errors on MT-001 are a minor issue. The lack of structured communication is a shared gap, not a coder-specific failure.

### 6.3 WP Validator Review

Strengths:

- Found 6 real issues including: dead code (FR-EVT-004 no emit function), missing struct (CascadeCancelRecord), race condition in test, and cross-surface integration gap (FR whitelist for announce_back).
- Provided genuine independent negative proof. These findings are not shallow — they required reading the code, running tests, and checking cross-surface integration.
- Produced a clear FAIL verdict with actionable fix items, enabling a bounded fix cycle.

Failures:

- Reviewed the full 4-MT diff as one unit rather than per-MT. True per-MT review would have caught the MT-001 compile errors earlier and potentially caught the dead code in MT-003 before MT-004 was written.
- Did not use governed communication surfaces (review_request/response messages).

Assessment:

- EXCELLENT. The validator's findings were genuine, actionable, and independently critical. The full-diff-at-once review pattern is suboptimal but still produced high-quality results. This is the strongest validator performance in the recent WP series.

### 6.4 Integration Validator Review

Strengths:

- The orchestrator applied the diff to main, ran tests, and confirmed 17/17 pass before committing (4b48f8c).

Failures:

- Integration validation was manual (orchestrator-driven). No governed integration validator session was launched.
- Same gap as the previous WP — there is no structured integration validation role in the current workflow.

Assessment:

- Manual, same as previous WP. No improvement. A governed integration validator session would add an independent check that the merge to main is clean and that no regressions were introduced by the merge itself (as opposed to the coder branch tests).

## 7. Review Of Coder and Validator Communication

What improved:

- The coder received per-MT prompts and stopped between MTs. This is the first WP where the coder/orchestrator/validator loop had any structure at all.
- Validator findings (6 items) were routed back to the coder via session-send. The coder did not receive the findings through orchestrator code edits.
- The fix cycle was bounded: one SEND_PROMPT with 6 items, one fix commit (300287d), one re-test pass (17/17).

What is still missing:

- No structured WP_COMMUNICATIONS receipts were created at any point during the run.
- No review_request/response messages were sent through the governed mailbox.
- RGF-93 completion notifications did not trigger the orchestrator to check results — the orchestrator was still polling.
- The communication was ACP-prompt-level only, not governed-receipt-level.

How to improve:

- After each MT commit, the coder should call `just wp-review-request` to create a structured receipt.
- The validator should call `just wp-review-response` to acknowledge and steer.
- The orchestrator should relay review responses back to the coder using governed message surfaces, not raw SEND_PROMPT.
- RGF-93 completion notifications should trigger the orchestrator to check results instead of polling.
- The next WP should try launching the validator after EACH MT commit, not after all MTs. This would catch issues like the MT-001 compile errors and MT-003 dead code before subsequent MTs are written.

## 8. ACP Runtime / Session Control Findings

Broker availability:

- 3 failures in 10 SEND_PROMPT dispatch attempts (~70% success rate).
- Every dispatch required a retry pattern, adding latency and orchestrator context waste.
- Broker availability was similar to the previous WP (no improvement).
- The root cause is still unclear: possible connection pooling, stale session registration, or broker process instability.

Session control:

- Coder session started via ACP broker at ~01:10, needed retry due to intermittent broker.
- Validator session started at ~02:15 without incident.
- Both sessions self-terminated before the orchestrator's closeout reclaim step.
- Terminal reclamation returned `reclaimed_count=0` (benign — terminals had already exited).

Completion notifications:

- RGF-93 completion notifications worked (the orchestrator received completion signals) but were not used as the primary trigger. The orchestrator was still polling for results.

Runtime truth:

- Runtime truth was clean by closeout. No orphan sessions, no stale terminals, no stuck brokers. The auto-reclaim on broker settle (27b01dd) and orphan scan at startup would have caught any residual state, but none was needed.

## 9. Governance Linkage and Board Mapping

- BOARD_LINKS:
  - SMOKE-FIND-20260406-01 -> RGF-89, RGF-93 (governed communication gap)
  - SMOKE-FIND-20260406-02 -> RGF-93 (broker reliability)
  - SMOKE-FIND-20260406-03 -> RGF-89 (role boundary — IMPROVED, now tracked as positive control)
  - SMOKE-FIND-20260406-04 -> NONE (new: closeout formatting automation needed)
  - SMOKE-FIND-20260406-05 -> RGF-94 (feature discovery — IMPROVED)
  - SMOKE-FIND-20260406-06 -> NONE (terminal reclamation — benign)
- CHANGESET_LINKS:
  - NONE (no governance changesets required by this review; improvements are tooling/helper gaps)
- POLICY_OR_TEMPLATE_FOLLOWUPS:
  - parseSectionField needs a scoped-to-latest-report mode to handle FAIL-then-PASS sequences
  - `just wp-closeout-format` helper needed to automate validator report appending and clause closure matrix update
  - `just wp-review-request` and `just wp-review-response` commands needed to create governed communication receipts
  - Consider a hard gate in gov-check that rejects orchestrator commits touching product source files (formalizing RGF-89)

## 10. Positive Controls Worth Preserving

### 10.1 Microtask Loop Execution

- CONTROL_ID: SMOKE-CONTROL-20260406-01
- CONTROL_TYPE: WORKFLOW_STABILITY
- SURFACE: Orchestrator per-MT prompt / coder commit-per-MT / validator review cycle
- What went well:
  - The coder committed per-MT (4 commits), stopped for review as instructed, and the validator found 6 real issues. The fix cycle was bounded to one pass. This is the first WP where the microtask loop actually executed as designed.
- Why it mattered:
  - The previous WP had zero microtask structure. This WP proved that per-MT prompts, commit-per-MT discipline, and a bounded fix cycle can work in the current infrastructure. This should remain the baseline for all future WPs.
- Evidence:
  - Commits c3487de (MT-001), ed5336e (MT-002), 31217bf (MT-003), 9c74a6d (MT-004).
  - Validator FAIL with 6 findings, coder fix at 300287d, re-test 17/17 PASS.
- REGRESSION_GUARDS:
  - Future WPs should be audited for commit-per-MT discipline
  - Smoketest reviews should check that the validator reviewed per-MT (or at minimum, that the coder stopped between MTs)

### 10.2 Role Boundary Enforcement (RGF-89)

- CONTROL_ID: SMOKE-CONTROL-20260406-02
- CONTROL_TYPE: REGRESSION_GUARD
- SURFACE: Orchestrator role lane / product code boundary
- What went well:
  - The orchestrator found compile errors in MT-001 and sent fix instructions to the coder via session-send instead of editing code directly. RGF-89 was honored throughout the run.
- Why it mattered:
  - The previous WP had a role violation where the orchestrator directly edited product code. This WP proved that the orchestrator can use the session-send relay to maintain role separation even when the coder produces broken code. This should remain the baseline.
- Evidence:
  - At ~01:35, orchestrator sent fix instructions via session-send. Coder fixed at 04fce59.
  - No orchestrator commits touch product source files in the entire run.
- REGRESSION_GUARDS:
  - gov-check should flag orchestrator commits touching product source files
  - Smoketest reviews should verify that fix instructions were relayed, not applied directly

### 10.3 Genuine Validator Negative Proof

- CONTROL_ID: SMOKE-CONTROL-20260406-03
- CONTROL_TYPE: PRODUCT_PROOF
- SURFACE: WP Validator review lane
- What went well:
  - The validator found 6 real issues: dead code (FR-EVT-004 no emit function), missing struct (CascadeCancelRecord), race condition in test, and cross-surface integration gap (FR whitelist for announce_back). These are genuine findings that required reading the code and checking cross-surface integration.
- Why it mattered:
  - The validator's FAIL verdict forced a bounded fix cycle that improved the final code. Without the validator, the dead code and missing struct would have shipped to main. Independent negative proof is the core value proposition of the governed validation workflow.
- Evidence:
  - Validator FAIL verdict with 6 actionable findings.
  - Coder fixed all 6 in one pass (300287d).
  - 17/17 tests pass after fix.
- REGRESSION_GUARDS:
  - Validator verdicts should always include specific code-level findings, not just PASS/FAIL
  - Smoketest reviews should check that validator findings are actionable and code-grounded

### 10.4 Feature Discovery Checkpoint (RGF-94)

- CONTROL_ID: SMOKE-CONTROL-20260406-04
- CONTROL_TYPE: PRODUCT_PROOF
- SURFACE: RGF-94 feature discovery / refinement surface
- What went well:
  - During refinement, the feature discovery checkpoint surfaced 4 new primitives, 2 new stubs, 3 matrix edges, and 6 UI controls. These items are now visible for downstream tracking.
- Why it mattered:
  - The previous WP had zero feature discovery. This WP proved that RGF-94 can surface adjacent scope during refinement, reducing the risk of hidden debt.
- Evidence:
  - 4 primitives, 2 stubs, 3 matrix edges, 6 UI controls discovered.
  - Stubs: cascade cancel persistence, announce_back routing.
  - Matrix edges: FR whitelist for announce_back, event routing for cascade cancel, session spawn request validation in integration test harness.
- REGRESSION_GUARDS:
  - Future WP refinements should run RGF-94 discovery and produce non-empty results
  - Discovered items should have a tracking destination (backlog, stub WP, or explicit deferral)

## 11. Remaining Product or Spec Debt

- **Cascade cancel persistence**: The cascade cancel mechanism is in-memory only. A downstream WP needs to add persistence for cascade cancel chains so they survive session restarts.
- **AnnounceBack routing**: The Role Mailbox AnnounceBack is implemented but has no governed routing surface — messages are created but not consumed by any listener. A downstream WP needs to wire announce_back into the orchestrator's session control loop.
- **FR whitelist for announce_back**: The FR-EVT event system does not have announce_back in its whitelist. Cross-surface integration requires adding announce_back events to the FR whitelist.
- **Integration test harness**: The session spawn request validation is tested in unit tests (17/17) but not in an integration test harness that exercises the full spawn lifecycle (request -> validate -> session create -> events -> cascade cancel -> announce_back).
- **Database-backed stores**: No persistence layer exists for any of the new structs. Only in-memory test implementations are provided.

## 12. Post-Smoketest Improvement Rubric

### 12.1 Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- Evidence:
  - The microtask loop executed for the first time (4 MTs, commit-per-MT, validator review, bounded fix cycle).
  - The orchestrator honored role boundaries (RGF-89) instead of directly editing product code.
  - Fire-and-forget dispatch reduced polling overhead compared to the previous WP.
  - Closeout formatting was still manual and consumed disproportionate orchestrator time.
  - ACP broker intermittent availability required retries on every dispatch.
- What improved:
  - Microtask loop: from zero structure (previous WP) to 4-MT commit-per-MT discipline.
  - Role boundary: from violation (previous WP) to enforcement via session-send relay.
  - Dispatch pattern: from heavy polling to fire-and-forget with completion notification.
- What still hurts:
  - Closeout formatting is manual and token-expensive. The parseSectionField utility cannot handle FAIL-then-PASS report sequences.
  - ACP broker intermittent availability adds latency and retry overhead to every dispatch.
  - Communication is ACP-prompt-level only, not governed-receipt-level.
- Next structural fix:
  - Build `just wp-closeout-format` to automate validator report appending, clause closure matrix updates, and VALIDATION_REPORTS section ordering. This would eliminate the single largest remaining source of orchestrator token waste during closeout.

### 12.2 Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- Evidence:
  - The session spawn contract closes a major spec gap: governed session lifecycle now has a typed request/response contract with invariant checks, lifecycle events, cascade cancel, and role mailbox acknowledge-back.
  - 2 stubs remain for downstream WPs (cascade cancel persistence, announce_back routing).
  - 3 matrix edges surfaced for cross-surface integration.
  - Feature discovery (RGF-94) made adjacent debt visible instead of hidden.
- What improved:
  - Session spawn lifecycle: from untyped/unvalidated to typed contract with 3 invariant checks and 5 lifecycle events.
  - Feature discovery: from zero (previous WP) to 4 primitives, 2 stubs, 3 matrix edges, 6 UI controls.
  - Validator findings: genuine negative proof that improved the final code quality.
- What still hurts:
  - No persistence layer for any new structs. In-memory only.
  - No integration test harness for the full spawn lifecycle.
  - announce_back has no consumer — it creates messages that nothing reads.
- Next structural fix:
  - Wire announce_back into the orchestrator's session control loop so the Role Mailbox AnnounceBack messages are actually consumed. This is the highest-value next step because it closes the feedback loop between coder sessions and the orchestrator.

### 12.3 Token Cost Pressure

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- Evidence:
  - Per-MT prompts were efficient: 4 focused prompts instead of one monolithic prompt.
  - Fire-and-forget dispatch reduced polling overhead.
  - Role boundary enforcement avoided the orchestrator spending tokens on code comprehension and editing.
  - Closeout formatting still consumed disproportionate orchestrator tokens.
  - ACP broker retries added ~1.5 minutes of dead time and repeated error-handling prompts.
  - No refinement format iteration was needed (unlike the previous WP which had 6 format fix passes).
- What improved:
  - Refinement: zero format iteration (previous WP: 6 passes). This alone saved significant orchestrator tokens.
  - Dispatch: fire-and-forget instead of polling. Less orchestrator context spent on waiting.
  - Role boundary: orchestrator did not spend tokens reading and editing product code.
- What still hurts:
  - Closeout formatting: manual report formatting, clause closure matrix updates, VALIDATION_REPORTS section ordering. This is the dominant remaining cost center.
  - ACP broker retries: 3 failures in 10 dispatches, each requiring error handling and retry context.
  - Communication overhead: the orchestrator relayed all messages manually through SEND_PROMPT. Governed receipts would reduce the orchestrator's relay burden.
- Next structural fix:
  - Build `just wp-closeout-format` to automate the closeout phase. This is the single highest-value cost reduction because closeout formatting is the dominant remaining token cost center and it repeats identically across every WP.

## 13. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 13.1 Silent Failures / False Greens

- The first validator report (FAIL) poisoned parseSectionField results, making the packet appear to have correct VALIDATION_REPORTS formatting when it actually had stale FAIL-report sections mixed with the new PASS-report sections. The orchestrator discovered this only during manual closeout formatting. A `just wp-closeout-format` helper would have detected the stale sections programmatically.
- Terminal reclamation returned `reclaimed_count=0`, which looked like a successful reclaim but actually meant the terminals had already exited. The reclaim log should distinguish "nothing to reclaim (all clean)" from "reclaim attempted and failed." Currently both produce `reclaimed_count=0`.

### 13.2 Systematic Wrong Tool or Command Calls

- NONE. The orchestrator used session-send for all coder/validator communication, which is the correct command family. No wrong tool or command family usage was observed.

### 13.3 Task and Path Ambiguity

- NONE. The WP scope was bounded (session spawn contract), the file ownership was clear (6 new files in the coder's worktree), and the packet path was unambiguous. No worktree/path/source-of-truth confusion was observed.

### 13.4 Read Amplification / Governance Document Churn

- NONE observed during the coding and validation phases. The per-MT prompt pattern gave the coder clear, bounded instructions without requiring governance document rereads.
- During closeout, the orchestrator did re-read the packet template and VALIDATION_REPORTS format rules to manually format the closeout. This is a symptom of the missing `just wp-closeout-format` helper, not a governance document ambiguity.

### 13.5 Hardening Direction

- **parseSectionField scoping**: Add a `--latest` flag or scoped mode to parseSectionField so it returns only the most recent report section, not the first match. This eliminates the FAIL-then-PASS poisoning.
- **`just wp-closeout-format`**: Build this helper to automate the entire closeout formatting phase (validator report appending, clause closure matrix update, VALIDATION_REPORTS ordering). This eliminates the dominant remaining source of manual orchestrator work.
- **`just wp-review-request` / `just wp-review-response`**: Build these to create governed communication receipts. This makes the microtask communication loop auditable and reduces the orchestrator's manual relay burden.
- **Terminal reclaim logging**: Distinguish "nothing to reclaim (all exited)" from "reclaim attempted and failed" in the reclaim output. This makes closeout logs more informative.
- **Broker retry at dispatch layer**: Add automatic retry with exponential backoff to the ACP broker dispatch surface so the orchestrator does not need to manually retry SEND_PROMPT calls.

## 14. Suggested Remediations

### Governance / Runtime

- Build `just wp-closeout-format` to automate closeout formatting (validator report append, clause closure matrix, VALIDATION_REPORTS ordering). This is the single highest-priority remediation.
- Add parseSectionField `--latest` mode to handle FAIL-then-PASS report sequences without poisoning.
- Add automatic retry with exponential backoff to the ACP broker SEND_PROMPT dispatch layer.
- Build `just wp-review-request` and `just wp-review-response` commands for governed communication receipts.
- Consider a gov-check hard gate that rejects orchestrator commits touching product source files (formalizing RGF-89 as a mechanical check).
- Improve terminal reclaim logging to distinguish "all exited" from "reclaim failed."

### Product / Validation Quality

- Launch the validator after EACH MT commit, not after all MTs. This would catch compile errors and dead code earlier in the cycle.
- Wire announce_back into the orchestrator's session control loop so AnnounceBack messages are consumed.
- Add an integration test harness for the full session spawn lifecycle.
- Track the 2 discovered stubs (cascade cancel persistence, announce_back routing) in the backlog with explicit WP targets.

### Documentation / Review Practice

- Document the per-MT prompt pattern as a standard practice in the orchestrator role guide. This WP proved it works; it should become the default.
- Document the FAIL-then-PASS validator report handling in the packet template so future orchestrators know to use `--latest` mode (once built).
- Add a "communication surface used" field to the smoketest review template so future reviews explicitly record whether governed receipts, raw SEND_PROMPT, or manual relay was used.

## 15. Command Log

- `just record-signature` -> PASS (signature recorded at ~01:00)
- `just orchestrator-prepare-and-packet` -> PASS (packet created at ~01:05)
- `just session-launch coder` -> PARTIAL (broker intermittent, needed retry at ~01:10)
- `just session-send MT-001` -> PASS (per-MT prompt sent at ~01:15)
- `just session-send MT-001-fix` -> PASS (fix instructions sent at ~01:35)
- `just session-send MT-002` -> PASS (per-MT prompt sent at ~01:45)
- `just session-send MT-003` -> PASS (per-MT prompt sent at ~01:52)
- `just session-send MT-004` -> PASS (per-MT prompt sent at ~01:57)
- `just session-launch validator` -> PASS (validator session started at ~02:15)
- `just session-send validator-fix-instructions` -> PASS (6 findings relayed at ~02:20)
- `cargo test` -> PASS (17/17 tests pass at ~03:00)
- `just merge-to-main` -> PASS (merged at 4b48f8c at ~03:10)
- `just session-cancel --all` -> PASS (sessions cancelled at ~03:20)
- `just terminal-reclaim` -> PARTIAL (reclaimed_count=0, terminals already exited)
