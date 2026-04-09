# AUDIT_20260409_SESSION_OBSERVABILITY_SPANS_FR_V1_SMOKETEST_REVIEW

## METADATA

- AUDIT_ID: AUDIT-20260409-SESSION-OBSERVABILITY-SPANS-FR-V1-SMOKETEST-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260409-SESSION-OBSERVABILITY-SPANS-FR-V1
- REVIEW_KIND: RECOVERY
- DATE_UTC: 2026-04-09
- AUTHOR: Codex acting as ORCHESTRATOR
- HISTORICAL_BASELINE_PACKET: NONE
- ACTIVE_RECOVERY_PACKET: WP-1-Session-Observability-Spans-FR-v1
- LINEAGE_STATUS: LIVE_SMOKETEST_BASELINE_PENDING
- RELATED_PREVIOUS_REVIEWS:
  - NONE
- SCOPE:
  - live recovery review for `.GOV/task_packets/WP-1-Session-Observability-Spans-FR-v1/packet.md`
  - committed product slice `bf3e7f81..4ba26a4a` on `feat/WP-1-Session-Observability-Spans-FR-v1`
  - governed runtime, receipts, session-control, and packet truth in `..\gov_runtime`
  - governance repair context from `.GOV/docs_repo/GOVERNANCE_PHASE_CONSOLIDATION_LOG_2026-04-08.md`
- RESULT:
  - PRODUCT_REMEDIATION: FAIL
  - MASTER_SPEC_AUDIT: FAIL
  - WORKFLOW_DISCIPLINE: PARTIAL
  - ACP_RUNTIME_DISCIPLINE: PARTIAL
  - MERGE_PROGRESSION: FAIL
- KEY_COMMITS_REVIEWED:
  - `bf3e7f81` `docs: bootstrap claim [WP-1-Session-Observability-Spans-FR-v1]`
  - `e7347859` `feat: wire session observability spans`
  - `4ba26a4a` `fix: narrow session observability proof surface`
- EVIDENCE_SOURCES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/task_packets/WP-1-Session-Observability-Spans-FR-v1/packet.md`
  - `.GOV/docs_repo/GOVERNANCE_PHASE_CONSOLIDATION_LOG_2026-04-08.md`
  - `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Session-Observability-Spans-FR-v1\RECEIPTS.jsonl`
  - `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Session-Observability-Spans-FR-v1\RUNTIME_STATUS.json`
  - `..\gov_runtime\roles_shared\ROLE_SESSION_REGISTRY.json`
  - `..\gov_runtime\roles_shared\SESSION_CONTROL_RESULTS.jsonl`
  - `..\gov_runtime\roles_shared\SESSION_CONTROL_OUTPUTS\WP_VALIDATOR_WP-1-Session-Observability-Spans-FR-v1\ebb339d4-f309-447f-911f-904517e6c37b.jsonl`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\api\flight_recorder.rs`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\flight_recorder\duckdb.rs`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\flight_recorder\mod.rs`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\mcp\gate.rs`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\tests\model_session_scheduler_tests.rs`
- RELATED_GOVERNANCE_ITEMS:
  - RGF-152
  - RGF-153
  - RGF-154
  - RGF-155
  - RGF-156
  - RGF-157
  - RGF-158
  - RGF-159
- RELATED_CHANGESETS:
  - NONE

---

## 1. Executive Summary

- This is a live recovery review, not a closeout. The WP has real committed product work across six signed files, but the governed validator rejected the committed handoff because the proof slice is not actually green and the signed contract is still incomplete. [VERIFIED: `git -C ..\wtc-spans-fr-v1 diff --stat bf3e7f81..4ba26a4`; `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Session-Observability-Spans-FR-v1\RECEIPTS.jsonl` entry at `2026-04-09T03:27:25.041Z`]
- Governance recovery was also real. This run only reached truthful validator review after eight governed `REPAIR` receipts cleared parser drift, packet hydration mismatch, stale phase routes, coder packet-mutation contradictions, and the committed-range handoff gate defect introduced during phase-surface consolidation. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Session-Observability-Spans-FR-v1\RECEIPTS.jsonl`]
- Runtime truth is currently back on `CODER`, but the coder is not actively processing work until a new steer lands. `RUNTIME_STATUS.json` expects `CODER` and `FINAL_REVIEW_EXCHANGE`, while the registry shows the coder session only in `READY` with `owned_terminal_reclaim_status = ALREADY_EXITED`. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Session-Observability-Spans-FR-v1\RUNTIME_STATUS.json`; `..\gov_runtime\roles_shared\ROLE_SESSION_REGISTRY.json`]
- Live update after review authoring: the follow-on governed steer did land despite the local shell timeout. The coder session is now `COMMAND_RUNNING` on a new governed prompt and is actively probing the validator findings, including an added API filter regression test in `src/backend/handshake_core/src/api/flight_recorder.rs`. [VERIFIED: `..\gov_runtime\roles_shared\ROLE_SESSION_REGISTRY.json`; `..\gov_runtime\roles_shared\SESSION_CONTROL_OUTPUTS\CODER_WP-1-Session-Observability-Spans-FR-v1\7bcb2548-d758-49b1-965a-3df45d551bda.jsonl`]

## 2. Lineage and What This Run Needed To Prove

- No earlier smoketest review exists for this packet. This document is therefore the baseline live review for the current run.
- The product proof target was narrow and concrete:
  - register and emit the missing `FR-EVT-SESS-001..005` lifecycle family through the canonical Flight Recorder path
  - bind `model_session_id`, `session_span_id`, and `activity_span_id` coherently across workflow, tool-call, DuckDB, and query surfaces
  - keep the existing recorder/query path as the only backend truth surface
  - prove the contract on the committed WP slice, not against unrelated governance drift
- The workflow proof target was equally concrete:
  - startup and handoff had to run through the canonical `just phase-check ...` surfaces introduced by the consolidation waves in `.GOV/docs_repo/GOVERNANCE_PHASE_CONSOLIDATION_LOG_2026-04-08.md`
  - packet-driven governed handoff had to survive session loss, stale route projections, and shared `.GOV` drift without falsifying product truth

### What Improved vs Previous Smoketest

- NONE as a same-WP predecessor smoketest comparison, because no earlier review exists.
- Improvement against the live pre-repair state is still material:
  - the run moved from repeated `WORKFLOW_INVALIDITY` blockers to a truthful `CODER_HANDOFF` and a governed `VALIDATOR_REVIEW`
  - the committed handoff gate now evaluates the packet's explicit range `bf3e7f81..4ba26a4` instead of falling back to dirty-worktree `MERGE_BASE_SHA..HEAD`
  - the runtime route now truthfully points back to `CODER` with concrete repair direction instead of lingering on stale phase-route failures
- What did not improve enough:
  - the coder self-audit and focused proof still overclaimed green semantics
  - the consolidated command surface still produced repeated orchestration churn before it became trustworthy for this WP

## 3. Product Outcome

- The committed product diff is real and non-trivial: six signed files changed with 1393 insertions and 125 deletions. [VERIFIED: `git -C ..\wtc-spans-fr-v1 diff --stat bf3e7f81..4ba26a4`]
- The product outcome is still a fail at this review point because the validator found three contract-level defects:
  - `run_and_finalize_workflow_job` marks the job terminal before `finalize_model_run_after_terminal` emits `session.completed`, so the end-to-end test can observe a terminal job without the final lifecycle row. [VERIFIED: `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:6874-6892`; `..\wtc-spans-fr-v1\src\backend\handshake_core\tests\model_session_scheduler_tests.rs:140-149`; `..\wtc-spans-fr-v1\src\backend\handshake_core\tests\model_session_scheduler_tests.rs:1283-1286`]
  - the API list path still drops `model_session_id` even though the lower recorder and DuckDB layers support it. [VERIFIED: `..\wtc-spans-fr-v1\src\backend\handshake_core\src\api\flight_recorder.rs:57-67`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\api\flight_recorder.rs:182-189`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\flight_recorder\mod.rs:5399-5404`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\flight_recorder\duckdb.rs:663-665`]
  - `session.budget_warning` advertises `budget_type = "tokens.total"` but computes `current_value` from only the latest assistant message token count, while completion totals use full-session aggregation. [VERIFIED: `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:2297-2304`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:6320-6333`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:6412-6427`]
- Signed scope is not closed. The packet's `STATUS_HANDOFF` and `EVIDENCE_MAPPING` still claim the backend query/API substrate remains a single truthful path, but that claim is currently false because the API adapter hardcodes `model_session_id: None`. [VERIFIED: `.GOV/task_packets/WP-1-Session-Observability-Spans-FR-v1/packet.md` `STATUS_HANDOFF` and `EVIDENCE_MAPPING`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\api\flight_recorder.rs:186`]
- Data-contract assessment:
  - SQL/PostgreSQL readiness is still conceptually aligned because the lower filter surface and typed recorder event fields remain explicit and SQL-shaped, but the API adapter omission leaves the data contract incomplete rather than backend-specific. [VERIFIED: `..\wtc-spans-fr-v1\src\backend\handshake_core\src\flight_recorder\mod.rs:5399-5404`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\flight_recorder\duckdb.rs:663-665`]
  - LLM-first readability improved by introducing explicit lifecycle events and span IDs, but `session.budget_warning` is semantically misleading because the field name says total-session while the value is per-message. [VERIFIED: `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:2297-2304`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:6320-6333`]
  - Loom-intertwined structure is still partial: stable IDs exist, but session-scoped retrieval remains incomplete until `model_session_id` reaches the API boundary. [VERIFIED: `..\wtc-spans-fr-v1\src\backend\handshake_core\src\api\flight_recorder.rs:182-189`; `..\wtc-spans-fr-v1\src\backend\handshake_core\src\mcp\gate.rs:336-368`]
- Adjacent debt outside signed closure:
  - the packet still names `cargo test fr_model_session_id`, but the validator found no matching test symbol in this worktree
  - after repair, the packet should be rechecked for whether the final touched-file reality and tripwire names are still the best signed proof surface

## 4. Timeline

| Time (UTC) | Event |
|---|---|
| 2026-04-08 22:46 | WP communication artifacts initialized |
| 2026-04-08 23:02 | `WP_VALIDATOR` emits `VALIDATOR_KICKOFF` and immediately reports the helper import defect context |
| 2026-04-08 23:10 | `CODER` records `WORKFLOW_INVALIDITY` `MT_CODE_SURFACES_PARSE_BLOCKER` |
| 2026-04-08 23:14 | `ORCHESTRATOR` repairs the code-surface parser/budget regression |
| 2026-04-08 23:46 | `WP_VALIDATOR` records `WORKFLOW_INVALIDITY` `PACKET_HYDRATION_MISMATCH` |
| 2026-04-08 23:51 | `ORCHESTRATOR` repairs the MT ordering mismatch |
| 2026-04-09 01:45 - 03:15 | repeated handoff-path invalidities repaired: missing phase route projection, stale phase wrapper, coder packet mutation contradiction, and committed-range handoff selection |
| 2026-04-09 03:16 | `CODER_HANDOFF` finally lands on committed range `bf3e7f81..4ba26a4` |
| 2026-04-09 03:27 | `WP_VALIDATOR` rejects the handoff with one high and two medium findings |
| 2026-04-09 07:43+ | orchestrator resumes the coder lane; session registry shows `COMMAND_RUNNING` and the coder begins validating the API filter repair path and related focused proof |

## 5. Per-Microtask Breakdown

| MT | Prompt Summary | Commit | Time Sent | Time Committed | Compile First Pass | Validator Flagged | Fix Cycles |
|---|---|---|---|---|---|---|---|
| MT-001 | Canonical lifecycle-family coverage for `FR-EVT-SESS-001..005` | `e7347859` | 23:43 UTC | 01:23 UTC | YES | YES (downstream handoff still failed on terminal ordering / API / budget semantics) | 0 |
| MT-002 | Runtime session/activity span binding and proof narrowing | `4ba26a4a` | N/A (same governed coder lane continuation after scope repair) | 01:31 UTC | NO | YES (handoff rejected) | 0 |

Assessment:
- The packet did use declared MTs, but the run did not achieve clean per-MT proof closure because governance repairs repeatedly interrupted the lane before final handoff.
- MT-001 is not rejected on lifecycle payload shape itself; the failure emerges when MT-002 semantic integration reaches terminal-state and query/accounting boundaries.

## 6. Communication Trail Audit

| # | Time | From | To | Surface | Content Summary |
|---|---|---|---|---|---|
| 1 | 23:02 | WP_VALIDATOR | CODER | wp-notification | `VALIDATOR_KICKOFF` with MT order and risk focus |
| 2 | 23:10 | CODER | ORCHESTRATOR | wp-notification | `WORKFLOW_INVALIDITY` `MT_CODE_SURFACES_PARSE_BLOCKER` |
| 3 | 23:14 | ORCHESTRATOR | CODER | wp-notification | `REPAIR` for code-surface parser/budget regression |
| 4 | 23:15 | CODER | WP_VALIDATOR | wp-review-request | `CODER_INTENT` for MT-001 |
| 5 | 23:23 | CODER | WP_VALIDATOR | THREAD.md | coordination ping while waiting for validator response |
| 6 | 23:30 | WP_VALIDATOR | CODER | wp-review-response | `SPEC_GAP` correcting MT-001 sequencing |
| 7 | 23:43 | WP_VALIDATOR | CODER | wp-review-response | `VALIDATOR_RESPONSE` clears bootstrap checkpoint |
| 8 | 23:46 | WP_VALIDATOR | ORCHESTRATOR | wp-notification | `WORKFLOW_INVALIDITY` `PACKET_HYDRATION_MISMATCH` |
| 9 | 23:51 | ORCHESTRATOR | CODER | wp-notification | `REPAIR` for swapped MT-001 / MT-002 packet files |
| 10 | 01:45 | CODER | ORCHESTRATOR | wp-notification | `WORKFLOW_INVALIDITY` `PHASE_CHECK_RECIPE_MISSING` |
| 11 | 02:40 | CODER | ORCHESTRATOR | wp-notification | `WORKFLOW_INVALIDITY` `CODER_PACKET_MUTATION_SURFACE_MISSING` |
| 12 | 02:42 | ORCHESTRATOR | CODER | wp-notification | `REPAIR` granting governed packet mutation path for coder-owned evidence/status |
| 13 | 03:07 | CODER | ORCHESTRATOR | wp-notification | `WORKFLOW_INVALIDITY` `HANDOFF_WORKTREE_DIFF_SELECTION_BLOCKER` |
| 14 | 03:15 | ORCHESTRATOR | CODER | wp-notification | `REPAIR` preferring explicit committed handoff range |
| 15 | 03:16 | CODER | WP_VALIDATOR | wp-review-request | `CODER_HANDOFF` on committed range `bf3e7f81..4ba26a4` |
| 16 | 03:27 | WP_VALIDATOR | CODER | wp-review-response | `VALIDATOR_REVIEW` rejects handoff with concrete repair direction |

Assessment:
- GOVERNED_RECEIPT_COUNT: 25
- RAW_PROMPT_COUNT: 35 `SEND_PROMPT` actions plus 2 `START_SESSION` and 1 `CANCEL_SESSION` recorded in `SESSION_CONTROL_RESULTS.jsonl`
- GOVERNED_RATIO: 25 / 63 = 0.40 when counting all visible cross-role traffic; 1.00 for evidence-bearing review exchanges only
- COMMUNICATION_VERDICT: MOSTLY_GOVERNED for review evidence, but below target overall because the orchestrator still spent too much traffic on wake/resume churn

## 7. Structured Failure Ledger

### 7.1 HIGH: committed handoff proof was falsely green

- FINDING_ID: SMOKE-FIND-20260409-01
- CATEGORY: PRODUCT_SCOPE
- ROLE_OWNER: CODER
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: RUNTIME_TRUTH
- SURFACE: committed range `bf3e7f81..4ba26a4`, packet `STATUS_HANDOFF`, validator handoff review
- SEVERITY: HIGH
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:6874-6892`
- Evidence:
  - `VALIDATOR_REVIEW` receipt at `2026-04-09T03:27:25.041Z`
  - `workflows.rs` terminal-state update before `finalize_model_run_after_terminal`
  - `model_session_scheduler_tests.rs` waits only for terminal state before expecting `session.completed`
- What went wrong:
  - the coder handoff claimed green proof even though terminal job state can become observable before the final lifecycle row exists
- Impact:
  - the main product claim for this WP is not proven
- Mechanical fix direction:
  - make the final lifecycle emission and terminal-state visibility semantically consistent, then rerun the committed proof slice and refresh packet evidence

### 7.2 HIGH: phase-consolidation command surface still produced repeated route breakage

- FINDING_ID: SMOKE-FIND-20260409-02
- CATEGORY: SCRIPT_OR_CHECK
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: COMMAND_SURFACE_MISUSE
- SURFACE: canonical `phase-check` startup/handoff surface after RGF-152..159 consolidation
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - RGF-152
  - RGF-156
  - RGF-157
  - RGF-158
  - RGF-159
- REGRESSION_HOOKS:
  - `WORKFLOW_INVALIDITY` receipts `PHASE_CHECK_RECIPE_MISSING`, `WP_CODER_HANDOFF_STALE_PHASE_ROUTE`, `CODER_PACKET_MUTATION_SURFACE_MISSING`
  - `.GOV/docs_repo/GOVERNANCE_PHASE_CONSOLIDATION_LOG_2026-04-08.md`
- Evidence:
  - repeated orchestrator `REPAIR` receipts between `2026-04-09T01:56:28.686Z` and `2026-04-09T02:42:18.810Z`
  - session-control failures and retries in `SESSION_CONTROL_RESULTS.jsonl`
- What went wrong:
  - the public surface was simplified on paper, but live route projections, packet write authority, and wrapper behavior did not settle mechanically for this WP
- Impact:
  - startup and handoff churn consumed most of the operator/orchestrator effort before product truth was even reviewable
- Mechanical fix direction:
  - keep one canonical command per phase, but also centralize route projection, wrapper behavior, and coder-owned packet mutation rules behind the same phase-owned surface

### 7.3 MEDIUM: packet parser and hydration truth were internally inconsistent

- FINDING_ID: SMOKE-FIND-20260409-03
- CATEGORY: GOVERNANCE_CHECK
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: SCRIPT_DEFECT
- SURFACE: MT packet files and microtask scope-budget parser
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `WORKFLOW_INVALIDITY` `MT_CODE_SURFACES_PARSE_BLOCKER`
  - `WORKFLOW_INVALIDITY` `PACKET_HYDRATION_MISMATCH`
- Evidence:
  - `CODER` invalidity at `2026-04-08T23:10:18.552Z`
  - `WP_VALIDATOR` invalidity at `2026-04-08T23:46:46.100Z`
  - orchestrator repairs at `23:14:30.901Z` and `23:51:02.563Z`
- What went wrong:
  - the signed packet family and the parser enforcing it disagreed about how MT scope should be expressed and in what order MT files should execute
- Impact:
  - legitimate coding work could not even begin without governance repairs
- Mechanical fix direction:
  - add packet-family contract tests that parse hydrated MT files exactly as the runtime budget gate will parse them

### 7.4 HIGH: committed handoff preflight initially selected the wrong diff surface

- FINDING_ID: SMOKE-FIND-20260409-04
- CATEGORY: ACP_RUNTIME
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: FALSE_GREEN
- SURFACE: committed coder handoff preflight and post-work range resolver
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - RGF-156
- REGRESSION_HOOKS:
  - `WORKFLOW_INVALIDITY` `HANDOFF_WORKTREE_DIFF_SELECTION_BLOCKER`
  - `just phase-check HANDOFF WP-1-Session-Observability-Spans-FR-v1 CODER --range bf3e7f81..4ba26a4`
- Evidence:
  - `CODER` invalidity at `2026-04-09T03:07:23.594Z`
  - `ORCHESTRATOR` repair at `2026-04-09T03:15:47.381Z`
- What went wrong:
  - the handoff wrapper used `MERGE_BASE_SHA..HEAD` and dirty-worktree fallback instead of the packet's explicit committed handoff range
- Impact:
  - unrelated governance drift looked like missing manifest coverage on the product diff
- Mechanical fix direction:
  - always prefer the packet-recorded committed handoff range whenever it exists, and test that preference directly

### 7.5 MEDIUM: API query contract is still incomplete at the adapter boundary

- FINDING_ID: SMOKE-FIND-20260409-05
- CATEGORY: PRODUCT_SCOPE
- ROLE_OWNER: CODER
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: OTHER
- SURFACE: `src/backend/handshake_core/src/api/flight_recorder.rs`
- SEVERITY: MEDIUM
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\api\flight_recorder.rs:57-67`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\api\flight_recorder.rs:182-189`
- Evidence:
  - API `EventFilter` does not expose `model_session_id`
  - `list_events` hardcodes `model_session_id: None`
  - lower layers do support the filter
- What went wrong:
  - the diff carried session query substrate changes into lower layers but did not finish the propagation to the API surface named in packet scope
- Impact:
  - session-scoped retrieval is still partial and the packet's query-substrate claim is overstated
- Mechanical fix direction:
  - thread `model_session_id` through the API filter and add a targeted end-to-end query assertion

### 7.6 MEDIUM: budget-warning semantics do not match the declared data contract

- FINDING_ID: SMOKE-FIND-20260409-06
- CATEGORY: PRODUCT_SCOPE
- ROLE_OWNER: CODER
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: OTHER
- SURFACE: `src/backend/handshake_core/src/workflows.rs`
- SEVERITY: MEDIUM
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:2297-2304`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:6320-6333`
  - `..\wtc-spans-fr-v1\src\backend\handshake_core\src\workflows.rs:6412-6427`
- Evidence:
  - `budget_type = "tokens.total"` in the emitted payload
  - warning threshold checks only the latest assistant message token count
  - completion totals aggregate across all session messages
- What went wrong:
  - emitted semantics and actual aggregation logic diverged
- Impact:
  - downstream consumers and local models will misread the warning event as whole-session accounting
- Mechanical fix direction:
  - either compute whole-session total at warning time or rename the budget type/value contract to match the actual measurement

### 7.7 LOW: wrong helper invocation still exists on the notification command surface

- FINDING_ID: SMOKE-FIND-20260409-07
- CATEGORY: OPERATOR_UX
- ROLE_OWNER: WP_VALIDATOR
- SYSTEM_SCOPE: LOCAL
- FAILURE_CLASS: COMMAND_SURFACE_MISUSE
- SURFACE: `just check-notifications ... --history`
- SEVERITY: LOW
- STATUS: MONITOR
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - validator output file `ebb339d4-f309-447f-911f-904517e6c37b.jsonl`
- Evidence:
  - validator session attempted `just check-notifications ... --history`
  - `just` returned `Justfile does not contain recipe '--history'`
- What went wrong:
  - a non-existent helper shape was used during review
- Impact:
  - small but real command-surface ambiguity remains even after consolidation
- Mechanical fix direction:
  - either add a supported history flag/recipe or remove the dead invocation shape from active prompts and habits

## 8. Role Review

### 8.1 Orchestrator Review

Strengths:

- kept authority aligned to runtime truth after session loss
- patched governance defects in place instead of bypassing them
- recovered the lane all the way to a truthful validator rejection rather than a soft-green stall

Failures:

- had to spend too much effort on governance repair before product steering resumed
- did not have a live smoketest review document in place at activation time
- session-control steering remained overly chatty and fragile

Assessment:

- Strong recovery execution, but the role is still compensating for too much broken infrastructure.

### 8.2 Coder Review

Strengths:

- stayed within governed authority and recorded truthful workflow invalidities instead of editing around them
- produced a committed product slice with concrete packet evidence and targeted proof commands
- preserved the signed six-file scope after the in-scope expansion repair

Failures:

- `CODER_HANDOFF` overclaimed green proof
- packet self-audit did not catch the API propagation hole
- packet self-audit did not catch the budget-warning semantic mismatch

Assessment:

- Product work is substantive, but the self-audit discipline was not strict enough for a high-risk data-contract WP.

### 8.3 WP Validator Review

Strengths:

- corrected MT sequencing early
- caught real semantic defects instead of rubber-stamping the narrow happy-path tests
- produced a findings-first review with concrete code anchors and counterfactual reasoning

Failures:

- small command-surface misuse remained during notification/history inspection
- review findings are not yet reflected back into the packet's mutable validation surfaces

Assessment:

- Good adversarial review quality. The validator is the reason the current product truth is honest.

### 8.4 Integration Validator Review

Strengths:

- NONE

Failures:

- NONE

Assessment:

- Not yet engaged. No integration-lane judgment is possible at this review point.

## 9. Review Of Coder and Validator Communication

- The coder and validator did communicate directly through governed packet surfaces, not only through orchestrator narration. `CODER_INTENT`, `VALIDATOR_RESPONSE`, `CODER_HANDOFF`, and `VALIDATOR_REVIEW` all exist as governed receipts tied to one correlation id.
- That said, the orchestrator was still a practical relay bottleneck for wake/resume traffic because phase-route failures repeatedly stopped the lane before the governed review loop could run cleanly.
- The direct review loop is therefore real but not yet mature enough to describe as mechanical.

## 9a. Memory Discipline

- MEMORY_WRITES_BY_ROLE:
  - ORCHESTRATOR: 6
  - CODER: NONE
  - WP_VALIDATOR: NONE
  - INTEGRATION_VALIDATOR: NONE
- MEMORY_WRITE_EVIDENCE:
  - ORCHESTRATOR procedural `#517`: `rg` grouped-regex PowerShell failure; use simpler patterns
  - ORCHESTRATOR procedural `#518`: mixed escaped quotes in PowerShell `rg` can parse as unterminated
  - ORCHESTRATOR procedural `#525`: `orchestrator-steer-next` may outlive local shell timeout; verify registry/output before retry
  - ORCHESTRATOR procedural `#526`: first targeted `cargo test` can timeout during compile; rerun longer before calling product failure
  - ORCHESTRATOR procedural `#531`: quote Windows paths with spaces or use `-LiteralPath`
  - ORCHESTRATOR procedural `#532`: do not call `just check-notifications ... --history`
- DUAL_WRITE_COMPLIANCE: PARTIAL
- MEMORY_VERDICT: PARTIAL
- Assessment:
  - Orchestrator did capture procedural run knowledge during the live recovery.
  - No evidence surfaced that coder or validator dual-wrote their own lessons into governed memory.
  - No evidence surfaced for a second vendor-memory write path during this run, so dual-write compliance is only partial.

## 9b. Build Artifact Hygiene

- BUILD_TARGET_PATH: `D:\Projects\LLM projects\Handshake\Handshake Worktrees\Handshake Artifacts`
- BUILD_TARGET_CLEANED_BY: NONE
- BUILD_TARGET_CLEANED_AT: N/A
- BUILD_TARGET_STATE_AT_CLOSEOUT: STALE
- Assessment:
  - The shared artifact directory exists and contains stale entries last modified before this WP review snapshot.
  - No run-local cleanup evidence was recorded.
  - Because this is a live recovery review rather than final closeout, this remains a monitor item rather than a closure blocker.

## 10. ACP Runtime / Session Control Findings

- Session-control overhead for this WP is high: 39 commands processed, 25 completed, 14 failed, for a 64.1% visible dispatch success rate. [VERIFIED: `..\gov_runtime\roles_shared\SESSION_CONTROL_RESULTS.jsonl` filtered to this WP]
- Failures clustered around orphaned governed requests, concurrent prompt attempts against already-running lanes, and post-crash recovery rather than product compilation. [VERIFIED: `SESSION_CONTROL_RESULTS.jsonl` rows for this WP]
- Runtime truth after the validator review is correct enough to steer from: `next_expected_actor = CODER`, `waiting_on = FINAL_REVIEW_EXCHANGE`, `last_event_at = 2026-04-09T03:27:25.041Z`. [VERIFIED: `..\gov_runtime\roles_shared\WP_COMMUNICATIONS\WP-1-Session-Observability-Spans-FR-v1\RUNTIME_STATUS.json`]
- Session liveness truth is still split across too many surfaces:
  - `RUNTIME_STATUS.json` shows no active role sessions
  - `ROLE_SESSION_REGISTRY.json` shows both coder and validator sessions as `READY`
  - both registry entries also show `owned_terminal_reclaim_status = ALREADY_EXITED`
- Live update after the current steer:
  - the shared runtime status file still has not advanced beyond the earlier validator receipt
  - the session registry now shows the coder lane in `COMMAND_RUNNING` on a new governed prompt
  - the active coder output file shows real ongoing work on the validator follow-up, so session-control truth is ahead of WP runtime truth for the current minute
- The consolidation log's stated goal of "one real command per phase" is directionally correct, but this run still required the operator/orchestrator to inspect packet, receipts, runtime status, session-control results, registry, gate logs, and code to understand what was truly happening.

## 11. Terminal Hygiene

- TERMINALS_LAUNCHED: 2
- TERMINALS_CLOSED_ON_COMPLETION: 0
- TERMINALS_CLOSED_ON_FAILURE: 0
- TERMINALS_RECLAIMED_AT_CLOSEOUT: 0
- STALE_BLANK_TERMINALS_REMAINING: 0 registry-visible
- TERMINAL_HYGIENE_VERDICT: PARTIAL
- Assessment:
  - Registry-visible terminal ownership did not remain attached to live processes; both current lane entries show `owned_terminal_reclaim_status = ALREADY_EXITED`.
  - That is better than leaked owned processes, but it is not clean lifecycle hygiene because repeated orphaned requests still occurred and no heartbeat surfaces show active work.
  - Desktop-visible blank terminals were not directly inspected here, so the zero count is only registry-visible truth.

## 12. Adversarial Review Artifacts

- DIFF_ATTACK_SURFACES:
  - terminal job-state transition vs final lifecycle event emission
  - API filter propagation vs recorder/DuckDB filter support
  - budget warning accounting vs session completion accounting
  - extra committed scope in `api/flight_recorder.rs` and `mcp/gate.rs` beyond the original hot-file list
- INDEPENDENT_CHECKS_RUN:
  - `cargo test session_lifecycle_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` -> PASS
  - `cargo test session_observability_spans_bind_model_runs_and_tool_calls --manifest-path src/backend/handshake_core/Cargo.toml` -> FAIL
  - `cargo test session_scheduler_event_payloads_are_validated --manifest-path src/backend/handshake_core/Cargo.toml` -> PASS
  - `cargo test fr_model_session_id --manifest-path src/backend/handshake_core/Cargo.toml` -> 0 matching tests ran
- COUNTERFACTUAL_CHECKS:
  - If `finalize_model_run_after_terminal` ran before the job became terminally visible, the new end-to-end test would not be able to observe a completed job without `session.completed`.
  - If `api/flight_recorder.rs` forwarded `model_session_id`, the lower-layer filter support already present in recorder/DuckDB would become reachable from the API.
- BOUNDARY_PROBES:
  - job terminal observer vs final recorder emitter: failed
  - API query adapter vs recorder filter/store path: propagation hole found
- NEGATIVE_PATH_CHECKS:
  - malformed lifecycle payloads remain rejected by the canonical recorder validator: pass
  - end-to-end completion visibility after the terminal-state flip: fail
- INDEPENDENT_FINDINGS:
  - FAIL. The committed handoff is not acceptable on `bf3e7f81..4ba26a4`.
- RESIDUAL_UNCERTAINTY:
  - `fr_model_session_id` is still named by the packet but no matching test symbol was found in this worktree.
  - After the three product repairs land, the touched-file reality and tripwire list should be rechecked once more before validator PASS.

## Post-Smoketest Improvement Rubric

### Workflow Smoothness

- TREND: FLAT
- CURRENT_STATE: HIGH
- NUMERIC_SCORE: 3
- Evidence:
  - eight orchestrator `REPAIR` receipts were required before truthful validator review
  - 39 session-control commands were processed for one WP, with 14 failures
  - mandatory probe families hit: silent failures and false greens; systematic wrong command calls; task/path ambiguity
- What improved:
  - the run eventually reached a governed coder handoff and a governed validator rejection instead of stalling in invalidity limbo
  - committed-range handoff proof is now mechanically evaluable
- What still hurts:
  - startup/handoff remains repair-heavy
  - packet and runtime projections still need too much orchestration supervision
  - workflow truth still depends on reading many surfaces
- Next structural fix:
  - make the phase-owned surface the only writer for route projection, packet-mutation authority, and committed-range selection

### Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- NUMERIC_SCORE: 4
- Evidence:
  - committed diff added real lifecycle/span work across six files
  - validator still found one high and two medium contract gaps
  - mandatory probe families hit: silent failures and false greens; task/path ambiguity
- What improved:
  - lifecycle-family coverage and span-binding infrastructure are now real code, not only packet intent
  - validator produced genuine negative proof instead of a shallow PASS
- What still hurts:
  - the WP is not actually spec-complete
  - API filter propagation and budget semantics remain open
  - one packet tripwire command names a test that does not exist
- Next structural fix:
  - repair the three concrete validator findings, then rerun proof on the same committed-range discipline before any merge talk

### Token Cost Pressure

- TREND: REGRESSED
- CURRENT_STATE: HIGH
- NUMERIC_SCORE: 2
- Evidence:
  - 39 session-control commands for one WP
  - repeated wake/resume traffic and repeated governance repairs
  - mandatory probe families hit: read amplification and governance-document churn; systematic wrong command calls
- What improved:
  - once the handoff-range resolver was fixed, later handoff proof became much cheaper and more honest
- What still hurts:
  - too many tokens were spent on route repair, status verification, and session recovery rather than product review
  - consolidation did reduce public names, but not enough actual debugging surfaces
- Next structural fix:
  - add one session-truth view that combines runtime status, active lane, active session, last receipt, and broker state so the orchestrator stops cross-reading five files

### Communication Maturity

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- NUMERIC_SCORE: 6
- Evidence:
  - core review exchange used governed receipts under one correlation id
  - review evidence traffic was direct coder <-> validator once the lane was healthy
  - mandatory probe families hit: silent failures and false greens; systematic wrong command calls
- What improved:
  - the product review itself is now auditable in `RECEIPTS.jsonl`
  - validator findings are findings-first and machine-locatable
- What still hurts:
  - overall cross-role traffic still includes too much raw session-control steering
  - the orchestrator is still a necessary operational relay
- Next structural fix:
  - have `VALIDATOR_REVIEW` auto-wake the expected actor directly and emit a single canonical resume brief so the orchestrator monitors instead of hand-steers

### Terminal and Session Hygiene

- TREND: FLAT
- CURRENT_STATE: HIGH
- NUMERIC_SCORE: 3
- Evidence:
  - current lane sessions are `READY` with `owned_terminal_reclaim_status = ALREADY_EXITED`
  - orphaned governed requests had to be recovered after process loss
  - mandatory probe families hit: silent failures and false greens
- What improved:
  - registry-visible owned processes are not lingering for the current coder and validator entries
- What still hurts:
  - process loss still leaves the system in a state where sessions look available but no work is actually in flight
  - runtime status does not expose that liveness clearly
- Next structural fix:
  - unify broker liveness, terminal ownership, and role readiness into one explicit session-health state so READY cannot mean "waiting for steer" and "processing work" at the same time

## Silent Failures, Command Surface Misuse, and Ambiguity Scan

- Silent failures and false greens:
  - The packet `STATUS_HANDOFF` and `EVIDENCE_MAPPING` treated the API/query substrate as complete even though `api/flight_recorder.rs` still dropped `model_session_id`.
  - The original handoff wrapper chose dirty-worktree diff selection and made unrelated governance drift look like a product manifest failure.
  - `RUNTIME_STATUS.json` truthfully routes back to `CODER`, but it does not show whether any role is actively processing work; the registry is needed to learn that both sessions are merely `READY`.
- Wrong tool or wrong command-family usage:
  - `just check-notifications ... --history` is not a valid helper shape in this repo.
  - repeated `SEND_PROMPT` retries against already-running or orphaned sessions show that the current orchestration path still lets the wrong command family be chosen under pressure.
- Task/path/worktree ambiguity:
  - MT packet files and the hydration source disagreed about MT-001 vs MT-002 ownership.
  - coder authority to update packet-owned status/evidence fields was unclear until explicitly repaired.
  - shared `.GOV` drift in `wt-gov-kernel` vs committed product diff in `..\wtc-spans-fr-v1` remained a recurring source of confusion until the explicit committed handoff range became authoritative.
- Read amplification and governance-document churn:
  - one truthful review required cross-reading the packet, receipts, runtime status, session registry, session-control results, validator output JSONL, gate logs, and product code.
  - this is directly at odds with the consolidation log's goal of making each phase easier to run and debug through fewer live surfaces.
