# AUDIT_20260408_PARALLEL_WP_CRASH_RECOVERY_CLOSEOUT_REVIEW

## METADATA

- AUDIT_ID: AUDIT-20260408-PARALLEL-WP-CRASH-RECOVERY-CLOSEOUT-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260408-PARALLEL-WP-CRASH-RECOVERY-CLOSEOUT
- REVIEW_KIND: CLOSEOUT
- DATE_UTC: 2026-04-08
- AUTHOR: Codex acting as ORCHESTRATOR
- HISTORICAL_BASELINE_PACKET: NONE
- ACTIVE_RECOVERY_PACKET: WP-1-Product-Governance-Check-Runner-v1, WP-1-Workspace-Safety-Parallel-Sessions-v1
- LINEAGE_STATUS: LIVE_SMOKETEST_BASELINE_RECOVERED
- RELATED_PREVIOUS_REVIEWS:
  - AUDIT-20260404-PARALLEL-WP-ACP-STEERING-RECOVERY-REVIEW
  - AUDIT-20260408-PARALLEL-WP-CRASH-RECOVERY-POSTMORTEM
- SCOPE:
  - prior failed crash-recovery attempt documented in `.GOV/Audits/smoketest/AUDIT_20260408_PARALLEL_WP_CRASH_RECOVERY_POSTMORTEM.md`
  - governed recovery closeout for `WP-1-Product-Governance-Check-Runner-v1`
  - governed recovery closeout for `WP-1-Workspace-Safety-Parallel-Sessions-v1`
  - contained-main outcome in `..\handshake_main` and live runtime/communications truth in `..\gov_runtime`
- RESULT:
  - PRODUCT_REMEDIATION: PASS
  - MASTER_SPEC_AUDIT: PASS
  - WORKFLOW_DISCIPLINE: PARTIAL
  - ACP_RUNTIME_DISCIPLINE: PARTIAL
  - MERGE_PROGRESSION: PASS
- KEY_COMMITS_REVIEWED:
  - `27d095a` `feat: MT-004 implement check runner service execution contract`
  - `37b7c9b` `feat: wire workspace safety enforcement into runtime`
  - `d7a4161` `feat: allocate and cleanup session worktrees`
  - `3ee738e` `Merge remote-tracking branch 'refs/remotes/localwp/feat/WP-1-Workspace-Safety-Parallel-Sessions-v1'`
- EVIDENCE_SOURCES:
  - `.GOV/Audits/smoketest/AUDIT_20260408_PARALLEL_WP_CRASH_RECOVERY_POSTMORTEM.md`
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\RECEIPTS.jsonl`
  - `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\NOTIFICATIONS.jsonl`
  - `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\RECEIPTS.jsonl`
  - `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\NOTIFICATIONS.jsonl`
  - `..\gov_runtime\roles_shared\SESSION_CONTROL_OUTPUTS\INTEGRATION_VALIDATOR_WP-1-Product-Governance-Check-Runner-v1\d9de61f8-2053-4749-b6b9-1ac3696eabe3.jsonl`
  - `..\gov_runtime\roles_shared\SESSION_CONTROL_OUTPUTS\INTEGRATION_VALIDATOR_WP-1-Workspace-Safety-Parallel-Sessions-v1\92967998-3108-4f1a-bdb8-591cd9875001.jsonl`
  - `..\handshake_main\src\backend\handshake_core\src\flight_recorder\mod.rs`
  - `..\handshake_main\src\backend\handshake_core\src\flight_recorder\duckdb.rs`
  - `..\handshake_main\src\backend\handshake_core\src\mex\gates.rs`
  - `..\handshake_main\src\backend\handshake_core\src\workflows.rs`
  - `..\handshake_main\src\backend\handshake_core\src\storage\sqlite.rs`
- RELATED_GOVERNANCE_ITEMS:
  - RGF-88
  - RGF-89
- RELATED_CHANGESETS:
  - NONE

---

## 1. Executive Summary

- The inherited postmortem baseline was a failed crash-recovery attempt: protocol drift, churn, and no governed completion. [VERIFIED: `.GOV/Audits/smoketest/AUDIT_20260408_PARALLEL_WP_CRASH_RECOVERY_POSTMORTEM.md`]
- This closeout recovered both WPs into truthful terminal state. `WP-1-Product-Governance-Check-Runner-v1` is contained in current `main` via `27d095a`, and `WP-1-Workspace-Safety-Parallel-Sessions-v1` was merged into current `main` at `3ee738e`. [VERIFIED: `git -C ..\handshake_main log --oneline --decorate -n 8`]
- Governed validator traffic recovered for the workspace-safety lane, including the accepted WAIVABLE/BENEFICIAL ruling, branch-head PASS, integration closeout exchange, and auto-route back to orchestrator verdict progression. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\NOTIFICATIONS.jsonl`]
- The merged product surface retains both workspace-safety enforcement and earlier session-checkpoint / flight-recorder behavior rather than overwriting one with the other. [VERIFIED: `rg -n "merge_back_artifact|workspace|checkpoint|session"` over `..\handshake_main\src\backend\handshake_core\src\mex\gates.rs`, `..\handshake_main\src\backend\handshake_core\src\flight_recorder\mod.rs`, `..\handshake_main\src\backend\handshake_core\src\flight_recorder\duckdb.rs`, `..\handshake_main\src\backend\handshake_core\src\workflows.rs`, `..\handshake_main\src\backend\handshake_core\src\storage\sqlite.rs`]
- At review-authoring time, the remaining control-plane defect was post-closeout route projection: the WP was already closed and contained, but the `VERDICT` route-health surface still projected an active expectation and therefore reported a false red. That follow-up pass was repaired in this same session.

## 2. Lineage and What This Run Needed To Prove

- The immediate inherited baseline is the prior orchestrator postmortem at `.GOV/Audits/smoketest/AUDIT_20260408_PARALLEL_WP_CRASH_RECOVERY_POSTMORTEM.md`.
- That postmortem established five concrete deficits: failed governed resumption, no merge progression, protocol misuse, ACP churn, and operator intervention.
- This closeout therefore needed to prove four narrower truths:
  - both WPs could be resumed through governed ACP lanes rather than manual relay
  - the `sqlite.rs` drift on Check-Runner could be resolved without widening packet truth
  - the workspace-safety out-of-scope touches in `workflows.rs` and `flight_recorder/duckdb.rs` could be adjudicated against the Master Spec and either waived or remediated
  - both WPs could reach truthful contained-main state with packet/runtime closeout surfaces aligned

### What Improved vs Previous Smoketest

- The previous postmortem ended with `WORKFLOW_DISCIPLINE: FAIL`, `ACP_RUNTIME_DISCIPLINE: FAIL`, and `MERGE_PROGRESSION: FAIL`. This closeout ended with both WPs integrated into `main`, which is the single most important delta.
- Check-Runner moved from "validator says integration-ready but recovery stalled" to contained-main truth. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\NOTIFICATIONS.jsonl` and `git -C ..\handshake_main log --oneline --decorate -n 8`]
- Workspace-Safety moved from crash-recovery churn and repeated MT review noise to explicit waiver acceptance at `2026-04-08T11:29:13.181Z`, final branch-head PASS at `2026-04-08T11:31:56.219Z`, and integration review progression at `2026-04-08T11:58:34.680Z`. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\NOTIFICATIONS.jsonl`]
- What did not improve enough: closeout sync still needed manual truth repair at the packet header, and post-closeout `VERDICT` route-health still lagged the already-closed runtime.

## 3. Product Outcome

- `WP-1-Product-Governance-Check-Runner-v1`
  - The governed validator had already established `FINAL PASS` with `15/15 governance_check tests green` and `All 8 DONE_MEANS verified` at head `bc5dd71`. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\NOTIFICATIONS.jsonl`]
  - The recovery closeout preserved that truth, reverted the stray `sqlite.rs` drift instead of silently widening scope, and retained contained-main presence through `27d095a`. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\NOTIFICATIONS.jsonl`; `git -C ..\handshake_main log --oneline --decorate -n 8`]
- `WP-1-Workspace-Safety-Parallel-Sessions-v1`
  - The governed closeout accepted the narrow enablement waiver for `workflows.rs` plus `flight_recorder/duckdb.rs`, then advanced through final coder handoff, validator PASS, and integration review. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\NOTIFICATIONS.jsonl`]
  - The merged main surface preserves workspace isolation gates, cross-session denial logic, workspace FR event mapping, and session checkpoint fields. [VERIFIED: `rg -n "merge_back_artifact|workspace|checkpoint|session"` over merged code paths]
  - Final main containment is recorded at `3ee738e`. [VERIFIED: `git -C ..\handshake_main log --oneline --decorate -n 8`]
- Adjacent debt outside signed closure:
  - Check-Runner still carries known follow-on debt the validator explicitly called non-blocking: version provenance, `input_schema`, execution-path completeness, and artifact-registry bridge work.
  - Workspace-Safety still depends on closeout surfaces correctly projecting terminal route truth after containment.

## 4. Timeline

| Time (UTC) | Event |
|---|---|
| 2026-04-08 early | Prior orchestrator recovery fails; postmortem recorded instead of closure |
| 2026-04-08 07:37 | Check-Runner WP validator records `FINAL PASS` in governed notifications |
| 2026-04-08 08:24 | Check-Runner coder handoff records `sqlite.rs` drift reverted and requests reconfirmation |
| 2026-04-08 08:37 | Check-Runner WP validator re-confirms final PASS and integration readiness |
| 2026-04-08 11:29 | Workspace-Safety validator accepts WAIVABLE/BENEFICIAL ruling for `workflows.rs` and `flight_recorder/duckdb.rs` |
| 2026-04-08 11:31 | Workspace-Safety branch-head validator review records PASS |
| 2026-04-08 11:58 | Workspace-Safety integration lane emits final review acknowledgement and `AUTO_ROUTE: direct review lane complete; orchestrator verdict progression ready` |
| 2026-04-08 12:27 | Workspace-Safety merge is present at `3ee738e`; packet truth then required manual header reconciliation |
| 2026-04-08 12:28 | Workspace-Safety governed coder and WP validator lanes close cleanly |
## 5. Per-Microtask Breakdown

### WP-1-Product-Governance-Check-Runner-v1

| MT | Prompt Summary | Commit | Time Sent | Time Committed | Compile First Pass | Validator Flagged | Fix Cycles |
|---|---|---|---|---|---|---|---|
| MT-001 | Check result model types | `1e1e113` | 21:21 | pre-recovery | YES | YES -> shape gaps deferred, then PASS | 1 |
| MT-002 | Tool contract + descriptor widening | `2bc65bc` | 21:58 | pre-recovery | NO | YES -> side effect + test move error | 2 |
| MT-003 | Typed lifecycle and result contract | `d18e745` | 22:48 | pre-recovery | YES | PASS | 1 |
| MT-004 | FR event emission + service execution contract | `bc5dd71` | pre-recovery | pre-recovery | YES | PASS, then duplicate review clearance after crash | 1 |

### WP-1-Workspace-Safety-Parallel-Sessions-v1

| MT | Prompt Summary | Commit | Time Sent | Time Committed | Compile First Pass | Validator Flagged | Fix Cycles |
|---|---|---|---|---|---|---|---|
| MT-001 | Session worktree allocation registry | `9f50d2d` | 21:22 | pre-recovery | NO | YES -> `is_empty` + module ordering, then PASS | 2 |
| MT-002 | Session-scoped denied command patterns | `a957bd` | 22:41 | pre-recovery | NO | YES -> missing tripwire test, then PASS | 2 |
| MT-003 | Merge-back artifact + conflict blocking | `1ccd7d9` | pre-recovery | pre-recovery | YES | PASS | 1 |
| MT-004 | In-scope path roots enforcement | `313b8c4` | pre-recovery | pre-recovery | YES | PASS, later crash-noise duplicates superseded | 1 |
| MT-005 | Fail-closed runtime enforcement | `37b7c9b` | recovery closeout | recovery closeout | PARTIAL | YES -> required waiver ruling on adjacent enablement touches | 1 |
| MT-006 | Cross-session denial and final cleanup | `d7a4161` | recovery closeout | recovery closeout | PARTIAL | YES -> closed through branch-head validator PASS | 1 |

## 6. Communication Trail Audit

Material governed closeout transitions reconstructed from `RECEIPTS.jsonl` and `NOTIFICATIONS.jsonl`:

| # | Time | From | To | Surface | Content Summary |
|---|---|---|---|---|---|
| 1 | 2026-04-08 07:37 | WP_VALIDATOR | CODER | `wp-notification` | Check-Runner `VALIDATOR_REVIEW: FINAL PASS` |
| 2 | 2026-04-08 08:24 | CODER | WP_VALIDATOR | `wp-notification` | Check-Runner `CODER_HANDOFF` with `sqlite.rs` drift reverted |
| 3 | 2026-04-08 08:37 | WP_VALIDATOR | CODER | `wp-notification` | Check-Runner final PASS re-confirmed and integration-ready |
| 4 | 2026-04-08 11:29 | WP_VALIDATOR | CODER | `wp-notification` | Workspace-Safety waiver accepted as WAIVABLE/BENEFICIAL |
| 5 | 2026-04-08 11:29 | WP_VALIDATOR | ORCHESTRATOR | `wp-notification` | Governance checkpoint projects next actor back to CODER |
| 6 | 2026-04-08 11:30 | CODER | WP_VALIDATOR | `wp-notification` | Workspace-Safety `CODER_HANDOFF` on branch head `d7a4161` |
| 7 | 2026-04-08 11:31 | WP_VALIDATOR | CODER | `wp-notification` | Workspace-Safety `VALIDATOR_REVIEW: PASS branch-head review` |
| 8 | 2026-04-08 11:56 | INTEGRATION_VALIDATOR | CODER | `wp-notification` | Integration final review exchange acknowledged |
| 9 | 2026-04-08 11:57 | CODER | INTEGRATION_VALIDATOR | `wp-notification` | Final integration review request for `f85d767..d7a4161` |
| 10 | 2026-04-08 11:58 | INTEGRATION_VALIDATOR | ORCHESTRATOR | `wp-notification` | `AUTO_ROUTE: direct review lane complete; orchestrator verdict progression ready` |
| 11 | 2026-04-08 12:27 | CODER | ORCHESTRATOR | `SESSION_SETTLE` | Workspace-Safety governed coder lane closed cleanly |
| 12 | 2026-04-08 12:28 | WP_VALIDATOR | ORCHESTRATOR | `SESSION_SETTLE` | Workspace-Safety governed validator lane closed cleanly |

Assessment:
- GOVERNED_RECEIPT_COUNT: >=12 material closeout messages reconstructed
- RAW_PROMPT_COUNT: 0 manual-relay messages used for the closeout recorded here
- GOVERNED_RATIO: >0.90 for material closeout transitions
- COMMUNICATION_VERDICT: MOSTLY_GOVERNED

## 7. Structured Failure Ledger

### 7.1 HIGH: closeout sync left packet headers stale after truthful containment

- FINDING_ID: SMOKE-FIND-20260408-01
- CATEGORY: SCRIPT_OR_CHECK
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: STATUS_DRIFT
- SURFACE: live packet header fields for `WP-1-Workspace-Safety-Parallel-Sessions-v1`
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `just integration-validator-closeout-check WP-1-Workspace-Safety-Parallel-Sessions-v1`
  - packet header truth vs runtime/main containment projection
- Evidence:
  - closeout sync passed, but the packet header still required manual reconciliation to `Validated (PASS)`, `CONTAINED_IN_MAIN`, baseline SHA, and verified timestamps before the final closeout gate became truthful
- What went wrong:
  - the helper path updated enough state to let progression continue but did not fully update the visible packet authority surface
- Impact:
  - operator-facing truth lagged the actual contained-main state and increased the chance of a false reopen or incorrect review conclusion
- Mechanical fix direction:
  - make closeout sync the single writer for header truth and fail closed when header projection did not update

### 7.2 HIGH: post-closeout VERDICT route-health remains red against an already-closed runtime

- FINDING_ID: SMOKE-FIND-20260408-02
- CATEGORY: ACP_RUNTIME
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: STATUS_DRIFT
- SURFACE: `wp-communication-health-check ... VERDICT` against `RUNTIME_STATUS.json`
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `just wp-communication-health-check WP-1-Workspace-Safety-Parallel-Sessions-v1 VERDICT`
  - terminal runtime projection in `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\RUNTIME_STATUS.json`
- Evidence:
  - runtime truth is terminal (`runtime_status: completed`, `next_expected_actor: NONE`, `waiting_on: CLOSED`) while the route-health surface still expects `ORCHESTRATOR` / `VERDICT_PROGRESSION`
  - post-fix follow-up: `just wp-communication-health-check WP-1-Workspace-Safety-Parallel-Sessions-v1 VERDICT` now passes
- What went wrong:
  - the route-health expectation did not treat contained-main terminal closure as a green end-state
- Impact:
  - the false red previously remained after the WP was already safely closed and integrated; that operator-facing contradiction is now removed
- Mechanical fix direction:
  - update the route-health projection to treat closed-contained WPs as terminal-green or refresh the projection immediately after closeout sync

### 7.3 MEDIUM: waiver-worthy adjacent touches required explicit spec ruling before closeout

- FINDING_ID: SMOKE-FIND-20260408-03
- CATEGORY: OUT_OF_SCOPE_WORK
- ROLE_OWNER: WP_VALIDATOR
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: OUT_OF_SCOPE
- SURFACE: `src/backend/handshake_core/src/workflows.rs`, `src/backend/handshake_core/src/flight_recorder/duckdb.rs`
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - validator waiver review in workspace-safety notifications
- Evidence:
  - the workspace-safety validator recorded that the touches were narrow enablement, beneficial against the Master Spec, and could be accepted without reopening packet scope
- What went wrong:
  - signed scope and real runtime enablement did not line up cleanly enough for mechanical acceptance without a human-readable validator ruling
- Impact:
  - closeout stalled until the ruling was explicit and governable
- Mechanical fix direction:
  - add a first-class accepted-enablements field or waiver attachment surface to packet truth so narrow beneficial touches do not look like silent scope breach

### 7.4 MEDIUM: crash-era duplicate review residue remained in communications surfaces

- FINDING_ID: SMOKE-FIND-20260408-04
- CATEGORY: WORKFLOW_DISCIPLINE
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: STALL
- SURFACE: review queues and superseded review notifications
- SEVERITY: MEDIUM
- STATUS: MONITOR
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - duplicate/superseded review items in both WP communications ledgers
- Evidence:
  - the workspace-safety communications ledger contains multiple supersession notices, escalation notices, and replayed crash-recovery review items before the final branch-head closeout path
- What went wrong:
  - crash recovery preserved too much obsolete review residue in the active route surface
- Impact:
  - extra reading and extra reconciliation were required to distinguish stale fix-loop history from the live closeout path
- Mechanical fix direction:
  - add a mechanical superseded-review compaction view or route-health should ignore items explicitly marked superseded

### 7.5 HIGH: no mandatory pre-start communication mesh check exists across governed roles

- FINDING_ID: SMOKE-FIND-20260408-05
- CATEGORY: ACP_RUNTIME
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: CHECK_FAILURE
- SURFACE: session startup and pre-work gating across ORCHESTRATOR, CODER, WP_VALIDATOR, INTEGRATION_VALIDATOR
- SEVERITY: HIGH
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - pre-work role handshake command with per-WP / per-role correlation ids
- Evidence:
  - operator feedback after closeout identified the missing gate explicitly: there is no required communication health verification across orchestrator, coder, and WP validator before productive work begins, nor a later integration-validator comm gate before final authority work
- What went wrong:
  - the workflow assumes role communication is healthy once sessions start, but it does not mechanically prove orchestrator->coder, orchestrator->wp-validator, coder<->wp-validator, and later orchestrator->integration-validator traffic before real MT work starts
- Impact:
  - communication defects are discovered late, after tokens have already been spent on startup, steering, or review attempts
- Mechanical fix direction:
  - add a required `COMM_MESH_STARTUP` gate that issues governed per-WP, per-role handshake receipts across orchestrator, coder, wp-validator, and later integration-validator before any productive work starts

### 7.6 HIGH: governance drift and fragmented checks/scripts dominated closeout cost

- FINDING_ID: SMOKE-FIND-20260408-06
- CATEGORY: TOKEN_COST
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: TOKEN_WASTE
- SURFACE: fragmented checks, tests, closeout sync helpers, and multi-surface truth repair
- SEVERITY: HIGH
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - phase-scoped composite check/test entrypoints and closeout timing audit
- Evidence:
  - operator feedback correctly identifies governance drift as the main time and token cost factor; this session also showed packet header repair, route-health repair, repeated communications review parsing, and separate helper/check surfaces instead of one phase-level truth gate
- What went wrong:
  - governance is still spread across many narrow scripts and checks, while multiple truth surfaces can drift and then require separate repair
- Impact:
  - closeout time becomes dominated by paperwork alignment and control-plane reconciliation instead of product validation
- Mechanical fix direction:
  - consolidate checks and tests into phase-level composite commands and reduce helper count behind stable phase entrypoints

### 7.7 HIGH: document misalignment persists because multiple surfaces still act like authorities

- FINDING_ID: SMOKE-FIND-20260408-07
- CATEGORY: GOVERNANCE_DRIFT
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: STATUS_DRIFT
- SURFACE: packet headers, runtime status, task board, notifications, and closeout projections
- SEVERITY: HIGH
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - single-authority projection audit across packet/runtime/task-board/health views
- Evidence:
  - even with mechanical tools, the session still needed manual packet header repair and later route-health repair because the system has multiple writable surfaces that can lag one another
- What went wrong:
  - some surfaces are projected mechanically, but others are still directly writable or updated by different helpers at different times, so the system is not yet single-authority end to end
- Impact:
  - document and runtime misalignment remains possible, and the operator pays the cost during closeout
- Mechanical fix direction:
  - collapse governance truth onto one authority surface per WP lifecycle stage and make all other surfaces read-only projections
## 8. Role Review

### 8.1 Orchestrator Review

Strengths:

- Took over from a documented failed recovery instead of narrating around it
- Preserved governed ACP progression and did not use manual relay for the closeout recorded here
- Resolved the Check-Runner `sqlite.rs` drift by preserving signed-scope truth rather than normalizing silent widening
- Drove both WPs to contained-main truth

Failures:

- Had to repair packet header truth manually after closeout sync
- Did not yet complete the post-closeout VERDICT route projection repair during the closeout itself

Assessment:

- PARTIAL PASS. Product closure and contained-main truth were achieved. Control-plane closeout still leaked one false red.

### 8.2 Coder Review

Strengths:

- Check-Runner coder provided a clean final handoff after reverting the stray drift
- Workspace-Safety coder advanced from crash residue to a branch-head handoff that the validator could PASS without reopening the packet

Failures:

- NONE in the final closeout slice that remains open

Assessment:

- PASS. The governed coder lanes finished in a way that preserved packet honesty.

### 8.3 WP Validator Review

Strengths:

- Re-confirmed Check-Runner PASS with explicit drift remediation
- Made the correct call on the workspace-safety waiver question rather than forcing arbitrary rework
- Recorded branch-head PASS with enough specificity to support integration progression

Failures:

- Crash-era duplicate review noise remained visible in active communications longer than necessary

Assessment:

- PASS. Validator judgment added real value and narrowed the closeout path correctly.

### 8.4 Integration Validator Review

Strengths:

- Acknowledged and progressed the workspace-safety integration closeout exchange through governed surfaces
- Left enough lane evidence to support truthful merge progression and main containment

Failures:

- Final route-health projection still did not collapse to a terminal-green state after containment
- Check-Runner integration-lane evidence is thinner than ideal because the environment hit usage limits after startup

Assessment:

- PARTIAL PASS. The lane enabled closure, but the closeout automation is still incomplete.

## 9. Review Of Coder and Validator Communication

- The decisive improvement over the inherited postmortem is that coder-validator communication became governable again.
- Check-Runner shows a clean late-phase sequence: validator final pass, coder handoff after drift revert, validator re-confirmation.
- Workspace-Safety shows the stronger pattern: waiver adjudication, coder handoff, validator branch-head PASS, then integration exchange and auto-route back to orchestrator.
- This was not manual relay. The evidence is in governed receipts and notifications. [VERIFIED: both WP `NOTIFICATIONS.jsonl` files]
- Remaining weakness: crash-era superseded review items still make the communication trail more expensive to read than it should be.

## 9a. Memory Discipline

- MEMORY_WRITES_BY_ROLE:
  - ORCHESTRATOR: NONE_VERIFIED
  - CODER: NONE_VERIFIED
  - WP_VALIDATOR: NONE_VERIFIED
  - INTEGRATION_VALIDATOR: NONE_VERIFIED
- MEMORY_WRITE_EVIDENCE:
  - NONE verified from the closeout artifacts reviewed here
- DUAL_WRITE_COMPLIANCE: NONE
- MEMORY_VERDICT: NONE
- Assessment:
  - This closeout review cannot verify any governed dual-write memory capture from the artifacts opened for this pass.
  - That is not a product blocker, but it means post-run learning is still too dependent on the smoketest review itself.

## 9b. Build Artifact Hygiene

- BUILD_TARGET_PATH: `..\Handshake Artifacts`
- BUILD_TARGET_CLEANED_BY: NOT_CHECKED
- BUILD_TARGET_CLEANED_AT: N/A
- BUILD_TARGET_STATE_AT_CLOSEOUT: NOT_CHECKED
- Assessment:
  - The closeout evidence opened for this review does not prove artifact cleanup one way or the other.
  - This remains an operational visibility gap.

## 10. ACP Runtime / Session Control Findings

- The ACP broker was good enough for governed completion once the lanes were back on a truthful route.
- Workspace-Safety shows clean governed closeout transitions all the way through `AUTO_ROUTE` back to orchestrator verdict progression. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\NOTIFICATIONS.jsonl`]
- Check-Runner shows that the WP validator lane was already sufficient to establish merge readiness before the integration lane hit usage limits. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\NOTIFICATIONS.jsonl`; `..\gov_runtime\roles_shared\SESSION_CONTROL_OUTPUTS\INTEGRATION_VALIDATOR_WP-1-Product-Governance-Check-Runner-v1\d9de61f8-2053-4749-b6b9-1ac3696eabe3.jsonl`]
- Runtime truth was therefore mostly repaired, not fully automated.
- Broker dispatch success rate: at least 6 successful material closeout transitions out of 6 reconstructed closeout transitions = 100% for the slice audited here.

## 11. Terminal Hygiene

- TERMINALS_LAUNCHED: [UNVERIFIED]
- TERMINALS_CLOSED_ON_COMPLETION: 2
- TERMINALS_CLOSED_ON_FAILURE: 0 in the closeout slice reconstructed here
- TERMINALS_RECLAIMED_AT_CLOSEOUT: [UNVERIFIED]
- STALE_BLANK_TERMINALS_REMAINING: [UNVERIFIED]
- TERMINAL_HYGIENE_VERDICT: PARTIAL

Assessment:
- The workspace-safety closeout does show explicit governed session closure notifications for coder and WP validator.
- The reviewed artifacts do not prove desktop-level terminal cleanliness across all lanes.
- This remains weaker than it should be for a workflow that claims mechanical closeout.

## 12. Governance Linkage and Board Mapping

- BOARD_LINKS:
  - `SMOKE-FIND-20260408-01 -> NONE`
  - `SMOKE-FIND-20260408-02 -> NONE`
  - `SMOKE-FIND-20260408-03 -> NONE`
  - `SMOKE-FIND-20260408-04 -> NONE`
- CHANGESET_LINKS:
  - NONE
- POLICY_OR_TEMPLATE_FOLLOWUPS:
  - closeout sync must own packet header truth atomically
  - route-health must understand terminal contained-main closure
  - superseded review items need a cheaper operator/audit surface

## 13. Positive Controls Worth Preserving

### 13.1 Governed waiver adjudication instead of silent scope expansion

- CONTROL_ID: SMOKE-CONTROL-20260408-01
- CONTROL_TYPE: WORKFLOW_STABILITY
- SURFACE: workspace-safety validator lane
- What went well:
  - The validator did not hand-wave the out-of-scope touches. It explicitly ruled them WAIVABLE/BENEFICIAL against the spec, then let closeout continue.
- Why it mattered:
  - This preserved packet honesty without forcing arbitrary churn or hiding real enablement dependencies.
- Evidence:
  - `2026-04-08T11:29:13.181Z` validator response and governance checkpoint in workspace-safety notifications
- REGRESSION_GUARDS:
  - keep explicit waiver/beneficial rulings as structured governed messages

### 13.2 Merged main retained both feature surfaces

- CONTROL_ID: SMOKE-CONTROL-20260408-02
- CONTROL_TYPE: PRODUCT_PROOF
- SURFACE: merged `handshake_main` product code
- What went well:
  - The final merge preserved workspace safety gates while also retaining session-checkpoint and flight-recorder event surfaces.
- Why it mattered:
  - This is the exact anti-regression proof that matters after parallel crash recovery: one WP did not erase the other.
- Evidence:
  - merged code search hits in `mex/gates.rs`, `flight_recorder/mod.rs`, `flight_recorder/duckdb.rs`, `workflows.rs`, and `storage/sqlite.rs`
- REGRESSION_GUARDS:
  - retain current-main interaction review and merged-surface search as part of closeout review

### 13.3 Governed closeout can recover from a failed recovery baseline

- CONTROL_ID: SMOKE-CONTROL-20260408-03
- CONTROL_TYPE: RUNTIME_TRUTH
- SURFACE: receipts + notifications + main containment
- What went well:
  - Even after the prior orchestrator failed, the governed ledgers were rich enough to recover, route, validate, and contain both WPs.
- Why it mattered:
  - It proves the system has real resilience value when the operator resists the temptation to switch to manual relay.
- Evidence:
  - inherited postmortem plus current closeout notifications and current `main` log
- REGRESSION_GUARDS:
  - keep crash-recovery closeout reviews tied to the actual governed receipts and notifications rather than narrative recollection
## 14. Cost Attribution

| Phase | Time (min) | Orchestrator Tokens (est) | Notes |
|---|---|---|---|
| Refinement | 0 | 0% | inherited baseline already existed |
| Per-MT Coding (total) | ~20 | ~20% | final governed handoffs and drift cleanup |
| Validation | ~35 | ~30% | validator pass re-confirmation, waiver adjudication, integration exchange |
| Fix Cycle | ~20 | ~20% | packet truth repair and closeout reconciliation |
| Closeout | ~25 | ~20% | contained-main sync and smoketest review packaging |
| Polling/Waiting | ~10 | ~10% | receipt/notification checking and lane closures |
| TOTAL | ~110 | ~100% | majority of cost was productive recovery, not panic churn |

## 15. Comparison Table (vs Previous WP)

| Metric | Previous WP | This WP | Trend |
|---|---|---|---|
| Total lines changed | N/A from failed recovery baseline | contained-main merge present | IMPROVED |
| Microtask count | 10 declared but not terminal | 10 declared and terminal | IMPROVED |
| Compile errors (first pass) | many pre-crash / not terminally resolved | bounded and resolved | IMPROVED |
| Validator findings | non-terminal postmortem residue | terminal PASS plus explicit waiver adjudication | IMPROVED |
| Fix cycles | churn-heavy and incomplete | bounded late fix loops | IMPROVED |
| Stubs discovered | non-blocking debt left open | non-blocking debt left open | FLAT |
| Governed receipts created | postmortem recovery had no governed completion | governed closeout transitions restored | IMPROVED |
| Broker dispatch failures | postmortem recorded severe churn | no material closeout dispatch failure in audited slice | IMPROVED |
| Stale terminals remaining | postmortem raised hygiene concern | still not fully proven clean | PARTIAL |
| Time to close (hours) | DNF | DONE same day | IMPROVED |

## 16. Remaining Product or Spec Debt

- Check-Runner follow-on debt called out by the validator remains outside DONE_MEANS: version provenance, `input_schema`, broader execution-path coverage, and artifact-registry bridge work.
- Workspace-Safety route-health closure is now fixed in this session; keep regression coverage so the terminal contained-main state stays green.
- Superseded crash-recovery review residue should be compacted mechanically so future audits do not have to re-parse obsolete fix-loop items.
- Startup communication between governed roles is still not proven mechanically before WP work begins.
- Check/test/script fragmentation still creates too many narrow command surfaces for each phase.

## 17. Post-Smoketest Improvement Rubric

### 17.1 Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- NUMERIC_SCORE: 6
- Evidence:
  - both WPs reached truthful contained-main state; governed messaging carried the closeout; but packet truth and route-health still needed manual repair or follow-up
- What improved:
  - the system moved from failed recovery to actual closure without switching to manual relay
- What still hurts:
  - closeout is still repair-heavy at the very end
- Next structural fix:
  - make closeout sync update packet headers and terminal route truth atomically

### 17.2 Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: LOW
- NUMERIC_SCORE: 8
- Evidence:
  - both targeted WPs are now contained in `main`; validator review preserved real negative proof and explicit non-DONE_MEANS debt instead of overstating completion
- What improved:
  - the main product gaps that motivated the two WPs are closed enough to merge and keep in `main`
- What still hurts:
  - adjacent follow-on debt still exists, especially on Check-Runner
- Next structural fix:
  - open the explicit follow-on packet(s) for the deferred non-DONE_MEANS surfaces rather than leaving them only in validator prose

### 17.3 Token Cost Pressure

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- NUMERIC_SCORE: 6
- Evidence:
  - cost shifted away from panic churn and toward actual validator/coder closeout work, but duplicate review residue and packet truth repair still consumed avoidable cycles
- What improved:
  - closeout tokens mostly went to useful governed progression
- What still hurts:
  - stale review items and route-truth repair still burn operator time
- Next structural fix:
  - compact superseded review items and make route-health terminal-aware

### 17.4 Communication Maturity

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- NUMERIC_SCORE: 7
- Evidence:
  - material closeout traffic is governed and auditable in `RECEIPTS.jsonl` / `NOTIFICATIONS.jsonl`; no manual relay was needed for the closeout recorded here
- What improved:
  - coder, validator, and integration-validator all exchanged governed state successfully after the crash-recovery failure baseline
- What still hurts:
  - the communication surface still carries too much superseded residue after a crash
- Next structural fix:
  - add a live "active review items only" projection for operators and route-health checks

### 17.5 Terminal and Session Hygiene

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- NUMERIC_SCORE: 5
- Evidence:
  - explicit governed session close notifications exist for the workspace-safety coder and validator lanes, but desktop terminal cleanliness is not fully proven from the reviewed artifacts
- What improved:
  - closeout did end with orderly lane closure instead of endless session churn
- What still hurts:
  - terminal ownership and final cleanup are still not easy to audit from the closeout record alone
- Next structural fix:
  - emit a mechanical closeout hygiene summary with terminal ownership and closure counts

## 18. Silent Failures, Command Surface Misuse, and Ambiguity Scan

### 18.1 Silent Failures / False Greens

- `integration-validator-closeout-sync` was not a sufficient proof of visible packet truth by itself. The helper could pass while the packet header still advertised stale status values.
- The `VERDICT` route-health surface is currently a false red after truthful closure, because it still expects an active progression state instead of honoring terminal containment.
- Startup communication health is also a latent silent-failure area because there is no required cross-role handshake proving that all governed lanes can talk before productive work starts.

### 18.2 Systematic Wrong Tool or Command Calls

- NONE observed in the closeout slice audited here.
- The main problem was not wrong command family usage. It was incomplete projection after otherwise-correct closeout commands.

### 18.3 Task and Path Ambiguity

- The workspace-safety packet needed an explicit ruling on whether `workflows.rs` and `flight_recorder/duckdb.rs` were narrow beneficial enablement or unauthorized widening.
- Check-Runner needed an explicit separation between packet scope and attractive adjacent drift in `sqlite.rs`.

### 18.4 Read Amplification / Governance Document Churn

- The crash-recovery residue forced extra receipt and notification scanning to distinguish superseded review items from the live closeout path.
- Packet truth had to be verified more than once because the helper path and the visible packet header diverged.
- Script and check sprawl forces too much command-surface recall during closeout. The operator concern about “1 big check or test per phase” is directionally correct.

### 18.5 Hardening Direction

- Make closeout sync the only authority for visible packet header truth.
- Make route-health terminal-aware for contained-main WPs.
- Add a compact active-review projection that ignores superseded crash-recovery residue.

## 19. Suggested Remediations

### Governance / Runtime

- Repair the projection logic behind `wp-communication-health-check ... VERDICT` so `runtime_status=completed`, `next_expected_actor=NONE`, and `waiting_on=CLOSED` are treated as terminal-green when containment truth is already recorded.
- Make closeout sync fail closed if packet header fields do not match the runtime/main projection immediately afterward.
- Add a compact active-review-items surface that hides superseded crash-recovery residue from operators and route-health checks.
- Add a required pre-work communication mesh command that proves orchestrator<->coder, orchestrator<->wp-validator, coder<->wp-validator, and later orchestrator<->integration-validator communication per WP using stable role/session/correlation ids.
- Consolidate narrow checks/tests behind phase entrypoints such as `phase-check kickoff`, `phase-check handoff`, `phase-check verdict`, with internal fan-out hidden behind one operator command.
- Move packet header, runtime status, task board, and route-health views toward a single-writer projection model so helpers cannot partially update truth.

### Product / Validation Quality

- Open follow-on work for the explicit Check-Runner non-DONE_MEANS debt rather than leaving it only in validator notes.
- Keep current-main interaction review as a non-negotiable closeout step for parallel crash-recovery WPs.

### Documentation / Review Practice

- Continue attaching the inherited failed-recovery postmortem directly to the succeeding closeout review so improvement claims are measurable.
- Add a dedicated subsection for "accepted beneficial enablement touches" to the smoketest review template and packet closeout law.
- Add a dedicated smoketest subsection for operator-cost findings when closeout or governance alignment consumes disproportionate time.

## 20. Command Log

- `Get-Content -LiteralPath 'C:\Users\Ilja Smets\.codex\skills\adversarial-code-review\SKILL.md'` -> PASS
- `Get-Content -LiteralPath '.GOV\templates\SMOKETEST_REVIEW_TEMPLATE.md'` -> PASS
- `Get-Content -LiteralPath '.GOV\roles_shared\docs\POST_SMOKETEST_IMPROVEMENT_RUBRIC.md'` -> PASS
- `Get-Content -LiteralPath '.GOV\Audits\smoketest\AUDIT_20260408_PARALLEL_WP_CRASH_RECOVERY_POSTMORTEM.md'` -> PASS
- `git -C ..\handshake_main log --oneline --decorate -n 8` -> PASS
- `rg -n "merge_back_artifact|workspace|checkpoint|session" ...` -> PASS
- `rg -n "REVIEW_REQUEST|REVIEW_RESPONSE|PASS|FAIL|correlation|wp-review" ..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\RECEIPTS.jsonl ..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Workspace-Safety-Parallel-Sessions-v1\NOTIFICATIONS.jsonl` -> PASS
- `rg -n "REVIEW_REQUEST|REVIEW_RESPONSE|PASS|FAIL|correlation|wp-review" ..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\RECEIPTS.jsonl ..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Product-Governance-Check-Runner-v1\NOTIFICATIONS.jsonl` -> PASS
- `just wp-communication-health-check WP-1-Workspace-Safety-Parallel-Sessions-v1 VERDICT` -> PASS

## LIVE_FINDINGS_LOG (append-only during WP execution)

- [2026-04-08T11:29Z] [WP_VALIDATOR] [OUT_OF_SCOPE_WORK] Workspace-Safety accepted WAIVABLE/BENEFICIAL ruling for `workflows.rs` and `flight_recorder/duckdb.rs`
- [2026-04-08T11:31Z] [WP_VALIDATOR] [VALIDATION] Workspace-Safety branch-head validator review recorded PASS
- [2026-04-08T11:58Z] [INTEGRATION_VALIDATOR] [ACP_RUNTIME] Integration lane emitted `AUTO_ROUTE` back to orchestrator verdict progression
- [2026-04-08T12:27Z] [ORCHESTRATOR] [STATUS_DRIFT] Workspace-Safety packet header still stale after closeout sync and required truth repair
- [2026-04-08T12:28Z] [ORCHESTRATOR] [TERMINAL_HYGIENE] Workspace-Safety governed coder and validator lanes closed cleanly
- [2026-04-08T12:28Z] [ORCHESTRATOR] [ACP_RUNTIME] Post-closeout `VERDICT` route-health still projects a false red against a closed-contained runtime
