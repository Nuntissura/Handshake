# AUDIT_20260410_GOVERNANCE_WORKFLOW_MIRROR_ACTIVATION_MANAGER_SMOKETEST_REVIEW

## METADATA

- AUDIT_ID: AUDIT-20260410-GOVERNANCE-WORKFLOW-MIRROR-ACTIVATION-MANAGER-SMOKETEST-REVIEW
- SMOKETEST_REVIEW_ID: SMOKETEST-REVIEW-20260410-GOVERNANCE-WORKFLOW-MIRROR
- REVIEW_KIND: CLOSEOUT
- DATE_UTC: 2026-04-10
- AUTHOR: Codex acting as ORCHESTRATOR
- HISTORICAL_BASELINE_PACKET: WP-1-Session-Observability-Spans-FR-v1
- ACTIVE_RECOVERY_PACKET: WP-1-Governance-Workflow-Mirror-v1
- LINEAGE_STATUS: LIVE_SMOKETEST_BASELINE_RECOVERED
- RELATED_PREVIOUS_REVIEWS:
  - AUDIT-20260325-ORCHESTRATOR-MANAGED-WP-WORKFLOW-REVIEW
  - AUDIT-20260409-SESSION-OBSERVABILITY-SPANS-FR-V1-SMOKETEST-REVIEW
- SCOPE:
  - live smoketest review for `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/packet.md`
  - orchestrator-managed prelaunch through `ACTIVATION_MANAGER`
  - governed runtime, receipts, registry, token usage, monitor log, and session-control truth in `../gov_runtime`
  - committed product slice `facce56f879d4ee990f62566b12a8b26d8bc61d7..7ed83940a06856359b6789060c792b8b1652ca42`
- RESULT:
  - PRODUCT_REMEDIATION: PARTIAL
  - MASTER_SPEC_AUDIT: PARTIAL
  - WORKFLOW_DISCIPLINE: PARTIAL
  - ACP_RUNTIME_DISCIPLINE: PARTIAL
  - MERGE_PROGRESSION: PARTIAL
- KEY_COMMITS_REVIEWED:
  - `1dd18c3e` `gov: track activation-manager workflow surfaces`
  - `7264ffd2` `gov: harden per-mt validator steering lane`
  - `a742b989` `gov: add non-llm relay watchdog`
  - `c6834e76` `gov: bound relay watchdog auto-repair`
  - `a67ad7b9` `gov: add conservative relay restart rung`
  - `9831674d` `gov: localize linked-worktree .GOV suppression`
  - `86cf6510` `feat: complete governance workflow mirror handoff surface`
  - `7ed83940` `fix: narrow workflow mirror handoff scope`
- EVIDENCE_SOURCES:
  - `.GOV/templates/SMOKETEST_REVIEW_TEMPLATE.md`
  - `.GOV/roles_shared/docs/POST_SMOKETEST_IMPROVEMENT_RUBRIC.md`
  - `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/packet.md`
  - `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/MT-001.md`
  - `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/MT-002.md`
  - `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/MT-003.md`
  - `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/MT-004.md`
  - `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/MT-005.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Governance-Workflow-Mirror-v1/THREAD.md`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Governance-Workflow-Mirror-v1/RECEIPTS.jsonl`
  - `../gov_runtime/roles_shared/WP_COMMUNICATIONS/WP-1-Governance-Workflow-Mirror-v1/RUNTIME_STATUS.json`
  - `../gov_runtime/roles_shared/ROLE_SESSION_REGISTRY.json`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_RESULTS.jsonl`
  - `../gov_runtime/roles_shared/WP_TOKEN_USAGE/WP-1-Governance-Workflow-Mirror-v1.json`
  - `../gov_runtime/roles_shared/SESSION_MONITORS/WP-1-Governance-Workflow-Mirror-v1-monitor.log`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/ACTIVATION_MANAGER_WP-1-Governance-Workflow-Mirror-v1/111f40d5-51cb-4ce9-aa97-1749fb893ade.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/ACTIVATION_MANAGER_WP-1-Governance-Workflow-Mirror-v1/066737a1-d1f5-4464-a572-5b5f98f7753a.jsonl`
  - `../gov_runtime/roles_shared/SESSION_CONTROL_OUTPUTS/INTEGRATION_VALIDATOR_WP-1-Governance-Workflow-Mirror-v1/27ff39eb-e1ab-4b90-a4ba-931f32302c70.jsonl`
  - `../wtc-workflow-mirror-v1/src/backend/handshake_core/src/runtime_governance.rs`
  - `../wtc-workflow-mirror-v1/src/backend/handshake_core/src/locus/types.rs`
  - `../wtc-workflow-mirror-v1/src/backend/handshake_core/src/role_mailbox.rs`
  - `../wtc-workflow-mirror-v1/src/backend/handshake_core/src/workflows.rs`
  - `../wtc-workflow-mirror-v1/src/backend/handshake_core/src/flight_recorder/mod.rs`
  - `../handshake_main/src/backend/handshake_core/src/api/flight_recorder.rs`
- RELATED_GOVERNANCE_ITEMS:
  - RGF-162
  - RGF-163
  - RGF-166
  - RGF-167
- RELATED_CHANGESETS:
  - NONE

---

## 1. Executive Summary

- The new `ORCHESTRATOR_MANAGED` lane is materially better than the earlier manual-heavy baseline because `ACTIVATION_MANAGER` took over refinement, readiness, packet/worktree prep, and file-first handoff. The prelaunch verdict and mechanical checks were real, not narrated. [VERIFIED: `.../ACTIVATION_MANAGER.../111f40d5-51cb-4ce9-aa97-1749fb893ade.jsonl`; `.../066737a1-d1f5-4464-a572-5b5f98f7753a.jsonl`]
- The product slice is substantial and real: the final signed diff is five files with 1931 insertions and 71 deletions, covering runtime governance, Locus types, workflow/session-log wiring, and Flight Recorder event surfaces. [VERIFIED: `git -C ../wtc-workflow-mirror-v1 diff --stat facce56f879d4ee990f62566b12a8b26d8bc61d7..7ed83940a06856359b6789060c792b8b1652ca42`; product code paths listed in `EVIDENCE_SOURCES`]
- The coder and validator eventually communicated through governed receipts instead of needing the Orchestrator as the normal relay, but that behavior had to be repaired during the run. The direct per-MT review loop became trustworthy only after overlap-forward and packet-proof gating fixes landed. [VERIFIED: `THREAD.md`; `RECEIPTS.jsonl`; `git log` items `7264ffd2`, `a742b989`, `c6834e76`, `a67ad7b9`]
- The new ACP watchdog/monitor system was useful and justified. It detected stale relay states, re-steered conservatively, respected active output freshness, and later ran as a detached 15-minute observer. It was not turnkey at start-of-run; it became reliable because the smoketest itself forced hardening. [VERIFIED: `RECEIPTS.jsonl` `STEERING` receipts from `ORCHESTRATOR_WATCHDOG`; `SESSION_MONITORS/WP-1-Governance-Workflow-Mirror-v1-monitor.log`]
- Final closure is honest rather than falsely green. The WP ended `Validated (OUTDATED_ONLY)`, not `PASS`, because local current `main` had adjacent-scope drift in `src/backend/handshake_core/src/api/flight_recorder.rs` outside the signed packet. That honesty is a positive control, not a workflow failure. [VERIFIED: `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/packet.md`; `RUNTIME_STATUS.json`; `../handshake_main/src/backend/handshake_core/src/api/flight_recorder.rs`]

## 2. Lineage and What This Run Needed To Prove

- This review follows the earlier session-observability smoketest and the March orchestrator-managed workflow review. The historical question was no longer just "can the product slice be built," but "can the governed workflow run with less Orchestrator babysitting while still staying truthful?"
- The product truth target was narrow:
  - project per-WP gate and activation artifacts into runtime-owned governance surfaces
  - expose workflow-facing Spec Session Log continuity through product seams, not repo-governance files
  - preserve hard `.GOV/` rejection and runtime-owned `.handshake/gov/` storage
  - add the `FR-EVT-GOV-GATES-001` and `FR-EVT-GOV-WP-001` event family without inventing a parallel subsystem
- The workflow truth target was equally narrow:
  - prove `ACTIVATION_MANAGER` can handle refinement and readiness to the same bar as the manual Orchestrator lane
  - prove per-MT coder/validator review can run with governed receipts and bounded overlap
  - prove the ACP watchdog can observe, re-steer, and escalate conservatively rather than thrashing

### What Improved vs Previous Smoketest

- Prelaunch quality improved materially. The earlier baseline left most heavy reasoning and packet creation burden on the Orchestrator; this run moved refinement, readiness, and worktree creation to `ACTIVATION_MANAGER` with file-first handback and explicit readiness evidence. [VERIFIED: activation-manager JSONL evidence]
- Communication maturity improved materially. The previous smoketest still relied heavily on the Orchestrator as the functional relay. This run achieved repeated `REVIEW_REQUEST` and `REVIEW_RESPONSE` pairs across MT-001 through MT-003 and both final handoffs. [VERIFIED: `THREAD.md`; `RECEIPTS.jsonl`]
- Workflow truth became stricter. False greens around clause proof, token budgets, worktree suppression, and closeout semantics were surfaced and patched during the run rather than being narrated away.
- What did not improve enough:
  - ACP launch/steer is still too repair-heavy for a supposedly short smoke run.
  - The watchdog exists because the underlying lane still stalls or drifts often enough to justify an always-on repair surface.
  - Final closeout still needed governance patches before terminal status synchronized cleanly.

## 3. Product Outcome

- The product slice is real and correctly scoped at closeout: `runtime_governance.rs`, `locus/types.rs`, `role_mailbox.rs`, `workflows.rs`, and `flight_recorder/mod.rs`. The earlier `flight_recorder/duckdb.rs` widening was deliberately removed in the superseding handoff. [VERIFIED: `git -C ../wtc-workflow-mirror-v1 diff --stat ...`; `git -C ../wtc-workflow-mirror-v1 show -s --format="%H %cI %s" 7ed83940`; product file reads]
- The signed WP scope is functionally closed. The packet's five clause rows all ended `CODER_STATUS: PROVED` and `VALIDATOR_STATUS: CONFIRMED`. [VERIFIED: `.GOV/task_packets/WP-1-Governance-Workflow-Mirror-v1/packet.md`]
- The product did not honestly qualify for contained-main `PASS` closure because current `main` had adjacent-scope drift: duplicate `model_session_id` fields in `src/backend/handshake_core/src/api/flight_recorder.rs` with no local diff on this WP branch. [VERIFIED: `packet.md`; `RUNTIME_STATUS.json`; `rg -n "model_session_id" ../handshake_main/src/backend/handshake_core/src/api/flight_recorder.rs`]
- Master Spec alignment is good inside the signed slice and incomplete outside it. No enrichment was needed for this WP, but a follow-on packet is required for the adjacent-scope mainline defect.

## 4. Timeline

| Time (UTC) | Event |
|---|---|
| 2026-04-09 21:03 | Packet activated and WP communication thread initialized |
| 2026-04-09 21:13-21:14 | `WP_VALIDATOR` kickoff establishes adversarial scope and tripwire expectations |
| 2026-04-09 22:46 | `CODER_INTENT` begins MT-001..MT-005 lane |
| 2026-04-09 22:47-23:18 | early ACP/worktree/scope invalidities appear; Orchestrator patches governance-control defects in place |
| 2026-04-10 00:02 | `MT-001` review request lands |
| 2026-04-10 00:37 | `MT-001 PASS` |
| 2026-04-10 01:44 | `MT-002` review request lands |
| 2026-04-10 02:12 | `MT-002 PASS` |
| 2026-04-10 02:23 | `MT-003` review request lands |
| 2026-04-10 02:59-03:00 | `MT-003 PASS` and overlap-forward motion continues |
| 2026-04-10 03:00 | detached 15-minute monitor/watchdog begins recurring oversight |
| 2026-04-10 05:55 | coder final handoff `86cf6510` |
| 2026-04-10 06:04 | validator final `PASS` on `86cf6510` |
| 2026-04-10 06:46 | superseding narrowed handoff `7ed83940` |
| 2026-04-10 06:50 | validator `PASS` on `7ed83940`, authority transfers to `INTEGRATION_VALIDATOR` |
| 2026-04-10 08:51 | Integration closeout records terminal `OUTDATED_ONLY` status |

## 5. Per-Microtask Breakdown

| MT | Prompt Summary | Commit | Time Sent | Time Committed | Compile First Pass | Validator Flagged | Fix Cycles |
|---|---|---|---|---|---|---|---|
| MT-001 | Per-WP gate and activation mirror with canonical WP identity | `86cf6510` (rolled-up product handoff) | 00:02 UTC | 05:45 UTC | YES | YES (non-blocking fmt spill and verdict-summary note) | 0 |
| MT-002 | Workflow-facing Spec Session Log append/query and stable id linkage | `86cf6510` (rolled-up product handoff) | 01:44 UTC | 05:45 UTC | YES | YES (non-blocking idempotency and constructor notes) | 0 |
| MT-003 | Hard `.GOV/` rejection and runtime-boundary proof | `86cf6510` (rolled-up product handoff) | 02:23 UTC | 05:45 UTC | YES | YES (non-blocking canonicalization note) | 0 |
| MT-004 | Check-result linkage to evidence refs without storage-boundary bypass | `86cf6510` | 05:55 UTC | 05:45 UTC | YES | NO | 0 |
| MT-005 | `FR-EVT-GOV-GATES-001` and `FR-EVT-GOV-WP-001` recorder coverage | `7ed83940` | 06:46 UTC | 06:45 UTC | YES | YES (non-blocking DuckDB decode follow-on note) | 1 |

Assessment:
- MICROTASKS_WERE_USED and the lane did eventually behave like a true per-MT governed loop.
- The main regression is not lack of microtasks; it is that the loop only became mechanically trustworthy after governance/runtime fixes landed mid-run.

## 6. Communication Trail Audit

| # | Time | From | To | Surface | Content Summary |
|---|---|---|---|---|---|
| 1 | 21:13 UTC | WP_VALIDATOR | CODER | wp-notification | kickoff defines five clause rows, tripwire tests, and adversarial focus |
| 2 | 22:46 UTC | CODER | WP_VALIDATOR | wp-review-request | `CODER_INTENT` for MT-001..MT-005 execution order |
| 3 | 22:52 UTC | WP_VALIDATOR | CODER | wp-review-response | `INTENT_CHECKPOINT CLEAR` |
| 4 | 23:01 UTC | CODER | ORCHESTRATOR | wp-notification | `WORKFLOW_INVALIDITY` `SIGNED_SCOPE_DRIFT` / prelaunch-control breakage |
| 5 | 23:18 UTC | ORCHESTRATOR | CODER | wp-notification | `REPAIR` after scope/worktree control-plane patch |
| 6 | 00:02 UTC | CODER | WP_VALIDATOR | wp-review-request | `MT-001` overlap review request |
| 7 | 00:37 UTC | WP_VALIDATOR | CODER | wp-review-response | `MT-001 PASS` |
| 8 | 01:44 UTC | CODER | WP_VALIDATOR | wp-review-request | `MT-002` overlap review request |
| 9 | 02:12 UTC | WP_VALIDATOR | CODER | wp-review-response | `MT-002 PASS` |
| 10 | 02:23 UTC | CODER | WP_VALIDATOR | wp-review-request | `MT-003` overlap review request |
| 11 | 02:59 UTC | WP_VALIDATOR | CODER | wp-review-response | `MT-003 PASS` |
| 12 | 05:55 UTC | CODER | WP_VALIDATOR | wp-review-request | final handoff on `86cf6510` |
| 13 | 06:04 UTC | WP_VALIDATOR | CODER | wp-review-response | final `PASS` on `86cf6510` |
| 14 | 06:46 UTC | CODER | WP_VALIDATOR | wp-review-request | superseding narrowed handoff on `7ed83940` |
| 15 | 06:50 UTC | WP_VALIDATOR | CODER | wp-review-response | superseding `PASS`, transfer to `INTEGRATION_VALIDATOR` |
| 16 | 08:51 UTC | INTEGRATION_VALIDATOR | ORCHESTRATOR | SESSION_SETTLE / STATUS | terminal `OUTDATED_ONLY` closeout |

Assessment:
- GOVERNED_RECEIPT_COUNT: 26 governed-like receipts/notifications
- RAW_PROMPT_COUNT: 2 material raw launch/steer paths still mattered at startup/repair time
- GOVERNED_RATIO: 0.93
- COMMUNICATION_VERDICT: MOSTLY_GOVERNED

## 7. Structured Failure Ledger

### 7.1 HIGH: Activation Manager prelaunch succeeded, but ACP launch/steer truth still required repeated repair

- FINDING_ID: SMOKE-FIND-20260410-01
- CATEGORY: ACP_RUNTIME
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: RUNTIME_TRUTH
- SURFACE: ACP launch/steer, session registry, session settle, startup/resume routing
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - RGF-162
- REGRESSION_HOOKS:
  - `SESSION_CONTROL_RESULTS.jsonl`
  - `ROLE_SESSION_REGISTRY.json`
  - activation-manager output JSONLs
- Evidence:
  - activation-manager handoff/readiness are clean, but overall session-control results still show 10 failed commands out of 44
  - early run required repeated governance patching before coder/validator flow became stable
- What went wrong:
  - `ACTIVATION_MANAGER` quality was high, but the broker/session-control layer around it was not yet idempotent or mechanically boring
- Impact:
  - a supposedly short smoke run became a long control-plane repair run
- Mechanical fix direction:
  - finish cached-aware, idempotent, broker-aware launch/steer recovery so role startup is no longer the main friction source

### 7.2 HIGH: The coder and validator overlap-forward loop was not mechanical at start-of-run

- FINDING_ID: SMOKE-FIND-20260410-02
- CATEGORY: WORKFLOW_DISCIPLINE
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CROSS_ROLE
- FAILURE_CLASS: STATUS_DRIFT
- SURFACE: overlap review routing, next-actor projection, relay behavior
- SEVERITY: HIGH
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - RGF-163
- REGRESSION_HOOKS:
  - `THREAD.md`
  - `RECEIPTS.jsonl`
  - `wp-communication-health-lib.test.mjs`
- Evidence:
  - the user correctly observed that the coder should not wait for validator remediation review before moving to the next MT
  - the overlap-forward and deferred loop-back behavior had to be patched during the run
- What went wrong:
  - the workflow initially behaved more like blocking review than bounded overlap review
- Impact:
  - risk of idle coder time and unnecessary Orchestrator relay involvement
- Mechanical fix direction:
  - keep the validator as a sidecar reviewer and only queue remediation debt behind the current active MT

### 7.3 MEDIUM: Watchdog/monitor were necessary and useful, but not mature on first contact

- FINDING_ID: SMOKE-FIND-20260410-03
- CATEGORY: SCRIPT_OR_CHECK
- ROLE_OWNER: ORCHESTRATOR
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: SCRIPT_DEFECT
- SURFACE: `wp-relay-watchdog`, `session-stall-scan`, detached session monitor
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - RGF-166
  - RGF-167
- REGRESSION_HOOKS:
  - `SESSION_MONITORS/WP-1-Governance-Workflow-Mirror-v1-monitor.log`
  - watchdog `STEERING` receipts
- Evidence:
  - conservative re-steer, cycle budgets, stale-active-run reporting, and freshness-aware waiting were all added during the smoketest
- What went wrong:
  - the new watcher existed only because the underlying lane still needed active mechanical supervision
- Impact:
  - better than babysitting, but still not "set and forget"
- Mechanical fix direction:
  - keep the watcher local and deterministic, but reduce the number of runtime cases that require it

### 7.4 MEDIUM: Packet clause proof surfaces were falsely green before validator confirmation

- FINDING_ID: SMOKE-FIND-20260410-04
- CATEGORY: GOVERNANCE_DRIFT
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: STATUS_DRIFT
- SURFACE: packet clause closure matrix and coder handoff mutation rules
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - packet closure monitor tests
  - live packet reconcile through communications helper
- Evidence:
  - coder-owned packet surfaces promoted proof claims before validator acceptance and had to be normalized back to pending/claimed truth
- What went wrong:
  - packet prose was becoming more authoritative than runtime validator truth
- Impact:
  - false green risk at exactly the point where the packet should be hardest to game
- Mechanical fix direction:
  - coder handoff may update coder-owned claims; validator confirmation must own proof state

### 7.5 MEDIUM: Linked-worktree `.GOV` suppression leaked into the common exclude surface

- FINDING_ID: SMOKE-FIND-20260410-05
- CATEGORY: TOOLING
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: SCRIPT_DEFECT
- SURFACE: linked worktree reseed/suppression path
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `reseed-permanent-worktree-from-main.test.mjs`
- Evidence:
  - `.GOV/` suppression intended only for WP worktrees polluted the gov-kernel common exclude and repeatedly hid kernel governance files
- What went wrong:
  - the helper treated linked-worktree exclude resolution as worktree-local when it was actually shared
- Impact:
  - confusing git state and repeated hygiene/debug effort
- Mechanical fix direction:
  - keep suppression truly worktree-local and scrub stale `.GOV/` markers from shared exclude files

### 7.6 MEDIUM: Gross-token budgeting produced a false blocker on cached-heavy ACP runs

- FINDING_ID: SMOKE-FIND-20260410-06
- CATEGORY: TOKEN_COST
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: TOKEN_WASTE
- SURFACE: WP token-budget policy and usage reporting
- SEVERITY: MEDIUM
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - RGF-162
- REGRESSION_HOOKS:
  - `WP_TOKEN_USAGE/WP-1-Governance-Workflow-Mirror-v1.json`
  - token budget tests/report output
- Evidence:
  - gross input was `742,038,148`, cached input was `719,041,664`, fresh input was `22,996,484`
  - policy initially treated the gross figure as the hard budget
- What went wrong:
  - cached replay was being punished as if it were fresh reasoning cost
- Impact:
  - false `TOKEN_BUDGET_EXCEEDED` blocker on an otherwise live lane
- Mechanical fix direction:
  - keep gross cost as telemetry, but enforce on fresh uncached input for governed WP gating

### 7.7 LOW: Integration closeout still hit command-surface ambiguity under non-PASS closure

- FINDING_ID: SMOKE-FIND-20260410-07
- CATEGORY: OPERATOR_UX
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: CONTROL_PLANE
- FAILURE_CLASS: UX_AMBIGUITY
- SURFACE: closeout/phase-check command surface
- SEVERITY: LOW
- STATUS: FIXED_DURING_RUN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `phase-check CLOSEOUT`
  - session-control/closeout logs
- Evidence:
  - non-PASS closeout required patching so `OUTDATED_ONLY` behaved like a terminal, closeable state
- What went wrong:
  - closeout semantics were too `PASS`-centric
- Impact:
  - valid terminal outcomes still looked broken until governance was patched
- Mechanical fix direction:
  - keep honest terminal states first-class and mechanically synchronized

### 7.8 MEDIUM: Current local `main` still contains adjacent-scope drift outside the signed packet

- FINDING_ID: SMOKE-FIND-20260410-08
- CATEGORY: PRODUCT_SCOPE
- ROLE_OWNER: SHARED
- SYSTEM_SCOPE: LOCAL
- FAILURE_CLASS: OUT_OF_SCOPE
- SURFACE: `../handshake_main/src/backend/handshake_core/src/api/flight_recorder.rs`
- SEVERITY: MEDIUM
- STATUS: OPEN
- RELATED_GOVERNANCE_ITEMS:
  - NONE
- REGRESSION_HOOKS:
  - `rg -n "model_session_id" ../handshake_main/src/backend/handshake_core/src/api/flight_recorder.rs`
- Evidence:
  - duplicate `model_session_id` fields on local `main` with no local diff in the signed five-file WP scope
- What went wrong:
  - the product slice closed, but current main had already drifted in an adjacent surface that blocked honest contained-main pass
- Impact:
  - final closeout had to be `OUTDATED_ONLY`
- Mechanical fix direction:
  - open a follow-on WP for the adjacent mainline defect instead of widening a closed packet after the fact

## 8. Role Review

### 8.1 Orchestrator Review

Strengths:

- patched governance/control-plane bugs in place instead of bypassing the governed lane
- kept final closeout honest when `PASS` was no longer truthful
- built a real watcher/monitor surface rather than reverting to full-time manual babysitting

Failures:

- still had to do too much mid-run repair work for a smoke WP
- built some of the required workflow mechanics during the run rather than entering with them already stable

Assessment:

- Strong operational recovery role performance, but the role is still compensating for too much infrastructure immaturity.

### 8.1a Activation Manager Review

Strengths:

- handled refinement and readiness at the correct bar
- used file-first handoff instead of token-heavy pasted refinement blocks
- produced a concrete readiness artifact with real mechanical checks and no spec-enrichment drift

Failures:

- NONE in product/refinement quality
- the surrounding ACP/session-control path still made activation look less autonomous than intended

Assessment:

- This role is worth keeping. The prelaunch burden should stay here rather than moving back to the Orchestrator.

### 8.2 Coder Review

Strengths:

- produced a real signed product slice with meaningful runtime, workflow, session-log, and Flight Recorder changes
- responded well once the overlap-forward workflow became mechanical
- narrowed scope honestly in the superseding handoff

Failures:

- early formatter spill widened the diff beyond the intended signed scope
- depended on workflow/governance repairs to keep forward motion clean

Assessment:

- Product work quality is good. The main drag was control-plane instability, not lack of coding substance.

### 8.3 WP Validator Review

Strengths:

- defined adversarial scope clearly at kickoff
- delivered concrete pass/fail review messages tied to clauses and tests
- eventually acted as the real MT reviewer instead of a ceremonial final checkpoint

Failures:

- overlap-sidecar behavior was not mechanical enough at the start of the run

Assessment:

- Strong review quality. Workflow integration was initially under-mechanized, not intellectually weak.

### 8.4 Integration Validator Review

Strengths:

- preserved honest closeout semantics and refused to fake contained-main success
- identified adjacent-scope current-main incompatibility instead of silently widening scope

Failures:

- needed governance patching so non-PASS closeout would settle cleanly

Assessment:

- Correct technical judgment. The lane was blocked more by closeout mechanics than by review weakness.

## 9. Review Of Coder and Validator Communication

- This run is the first recent governed lane where coder and validator communication felt meaningfully direct rather than theatrically direct. The evidence trail shows repeated `REVIEW_REQUEST` and `REVIEW_RESPONSE` pairs with the Orchestrator mostly acting as a monitor/repair owner rather than an every-message relay. [VERIFIED: `THREAD.md`; `RECEIPTS.jsonl`]
- The user's intended rule is correct and is now reflected in behavior: the coder should keep moving while the validator reviews the previous MT, and remediation debt should queue behind the current active MT rather than stopping the lane immediately.
- That said, the behavior only became trustworthy after workflow repairs. This is improvement, not final maturity.

## 9a. Memory Discipline

- Repo memory was used correctly for recurring workflow failures, command-surface workarounds, and policy decisions.
- The main memory-discipline improvement was the move to file-first refinement handoff. That reduced repeated long chat-pastes and kept the Orchestrator context cleaner.
- Memory pressure is still higher than it should be because ACP/session-control repairs, topology issues, and repeated status truth checks still create avoidable context churn.

## 9b. Build Artifact Hygiene

- Product build artifact hygiene ended acceptable after the superseding narrow handoff removed formatter spill and kept the signed product scope to five files.
- Governance/build artifact hygiene improved during the run:
  - linked-worktree `.GOV` suppression no longer leaks into gov-kernel
  - reseed/flush logic was hardened for slow systems and NAS-backed paths
- The system is better than it was at start-of-run, but still too repair-sensitive for a smoke lane marketed as "easy."

## 10. ACP Runtime / Session Control Findings

- Session-control command success is improved but not yet smooth: 34 completed commands and 10 failed commands on this WP.
- The watchdog inside ACP is directionally correct:
  - it re-steers conservatively
  - it respects active-run freshness
  - it can run detached on a 15-minute loop
- The broader ACP/runtime layer is still too eager to accumulate launch, steer, settle, and budget drift issues that the watcher then has to mask.
- The right long-term shape remains:
  - models think
  - local scripts watch
  - restart/escalation policy stays bounded and idempotent

## 11. Terminal Hygiene

- Terminal/session hygiene is improved relative to earlier runs.
- Final registry truth shows the relevant governed sessions closed, and system-terminal ownership records for terminal-owning roles ended `ALREADY_EXITED` rather than leaking visible stale windows. [VERIFIED: `ROLE_SESSION_REGISTRY.json`]
- This is not yet a perfect `9-10` rubric state because terminal ownership, broker reuse, and unsettled command accounting still needed runtime repair during the run.

## 12. Governance Linkage and Board Mapping

- `RGF-162`: ACP role-session idempotence and mechanical recovery was directly exercised and partially advanced by this smoketest.
- `RGF-163`: per-MT validator steering and deferred loop-back repair was directly exercised and materially improved.
- `RGF-166`: the non-LLM relay watchdog was created specifically because this run proved the need for a local watcher.
- `RGF-167`: the autonomous relay repair ladder and watcher-service hardening are still not finished; this run is evidence that the item must stay active.

## 13. Positive Controls Worth Preserving

- POS-20260410-01 | CATEGORY: WORKFLOW_DISCIPLINE
  - Activation Manager file-first refinement and readiness handoff should remain the default for orchestrator-managed WPs.
- POS-20260410-02 | CATEGORY: ACP_RUNTIME
  - Freshness-aware watchdog behavior is the right design posture: prefer waiting on clearly active runs over impatient re-steer spam.
- POS-20260410-03 | CATEGORY: WORKFLOW_DISCIPLINE
  - Honest `OUTDATED_ONLY` closeout is a strength. It prevented a fake green contained-main result.
- POS-20260410-04 | CATEGORY: COMMUNICATION
  - Direct governed MT review traffic between coder and validator is worth preserving and extending.

## 14. Cost Attribution

- Gross token usage was very high:
  - gross input: `742,038,148`
  - cached input: `719,041,664`
  - fresh input: `22,996,484`
  - output: `2,873,233`
  - command count: `44`
  - turn count: `19`
- Interpretation:
  - most of the frightening number is cached replay, not fresh reasoning
  - fresh reasoning cost is still high for a supposedly short smoke WP
  - the main overhead drivers were launch/steer repair, repeated session-control inspections, topology repair, and closeout repair

## 15. Comparison Table (vs Previous WP)

| Metric | Previous Smoketest `WP-1-Session-Observability-Spans-FR-v1` | Current Smoketest `WP-1-Governance-Workflow-Mirror-v1` | Direction |
|---|---|---|---|
| Signed diff size | ~1518 lines changed | ~2002 lines changed | Larger |
| Declared microtasks | 2 | 5 | Better decomposition |
| Governed receipt count | 25 | 28 | Slightly better |
| Session-control failures | 14 of 39 | 10 of 44 | Improved but still high |
| Communication maturity | partial governed review | mostly governed MT review loop | Improved |
| Final closeout honesty | `Validated (PASS)` and contained-main | `Validated (OUTDATED_ONLY)` with adjacent-scope follow-on required | More honest |
| Prelaunch burden on Orchestrator | high | lower because `ACTIVATION_MANAGER` handled refinement/readiness | Improved |

## 16. Remaining Product or Spec Debt

- Adjacent-scope local-main defect in `src/backend/handshake_core/src/api/flight_recorder.rs` requires a follow-on WP.
- DuckDB decode support for the two new governance Flight Recorder payload types remains a follow-on note after the final scope narrowing.
- No Master Spec enrichment was required for this WP. Remaining debt is product/runtime/control-plane debt, not spec-text ambiguity.

## Post-Smoketest Improvement Rubric

### Workflow Smoothness

- TREND: IMPROVED
- CURRENT_STATE: HIGH
- NUMERIC_SCORE: 6
- Evidence:
  - `ACTIVATION_MANAGER` successfully owned refinement/readiness
  - per-MT review loop worked by the middle of the run
  - multiple control-plane repairs were still required
- What improved:
  - less manual Orchestrator babysitting than the earlier baseline
  - stronger per-MT governed review
- What still hurts:
  - too many runtime/control-plane patches for a smoke WP
  - closeout still needed governance repair
- Next structural fix:
  - finish the ACP recovery ladder so launch/steer/settle are boring before the next smoke run

### Master Spec Gap Reduction

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- NUMERIC_SCORE: 7
- Evidence:
  - all five signed clause rows ended confirmed inside the packet
  - no spec enrichment was required
  - adjacent-scope main drift is now explicit instead of hidden
- What improved:
  - the actual governance workflow mirror product gap is now closed in signed scope
- What still hurts:
  - mainline adjacent debt prevented contained-main pass
- Next structural fix:
  - follow-on packet for the `api/flight_recorder.rs` mainline defect

### Token Cost Pressure

- TREND: FLAT
- CURRENT_STATE: HIGH
- NUMERIC_SCORE: 3
- Evidence:
  - gross input remained enormous even after cached-aware budgeting
  - launch/steer/closeout repair still consumed too many turns
- What improved:
  - false budget blockers no longer fire on cached-heavy runs
- What still hurts:
  - the workflow is still too expensive in fresh reasoning for a smoke lane
- Next structural fix:
  - reduce repair traffic and move more observation/retry logic into local deterministic watchers

### Communication Maturity

- TREND: IMPROVED
- CURRENT_STATE: MEDIUM
- NUMERIC_SCORE: 8
- Evidence:
  - repeated governed `REVIEW_REQUEST` / `REVIEW_RESPONSE` usage
  - 0.93 governed communication ratio
- What improved:
  - coder and validator finally used direct governed surfaces as intended
- What still hurts:
  - the loop still needed mid-run repair before becoming trustworthy
- Next structural fix:
  - preserve overlap-forward review mechanically and make validator wake/repair fully routine

### Terminal and Session Hygiene

- TREND: IMPROVED
- CURRENT_STATE: LOW
- NUMERIC_SCORE: 7
- Evidence:
  - final governed sessions closed cleanly
  - terminal-owning roles ended with reclaim status `ALREADY_EXITED`
- What improved:
  - fewer stale visible terminal outcomes than earlier runs
- What still hurts:
  - command settlement and broker/session truth still drift too easily mid-run
- Next structural fix:
  - continue tightening session settlement and terminal ownership reconciliation

## Silent Failures, Command Surface Misuse, and Ambiguity Scan

- Silent failures or false greens:
  - packet clause proof turned green before validator confirmation
  - gross-token budget looked catastrophic until cached/fresh truth was separated
- Wrong tool or command-family usage:
  - non-PASS closeout paths were under-specified and behaved like edge cases
  - worktree `.GOV` suppression targeted the wrong git metadata surface in linked-worktree topology
- Task/path/worktree ambiguity:
  - linked-worktree exclude scope was ambiguous enough to pollute gov-kernel
  - current-main adjacency versus signed packet scope needed explicit terminal policy to avoid fake widening
- Read amplification and governance-document churn:
  - this run still caused too many runtime/status/tooling rereads because the control plane was not yet stable enough to trust without inspection

## 19. Suggested Remediations

- Finish `RGF-162` so ACP startup/steer/resume are idempotent and cached-aware by default.
- Finish `RGF-163` so per-MT validator overlap review never regresses into blocking behavior.
- Keep `RGF-166` and `RGF-167` active until the watchdog/monitor system becomes boring enough that smoke runs no longer prove its necessity.
- Open the follow-on product packet for the adjacent current-main `api/flight_recorder.rs` defect instead of widening the closed packet.
- Keep `ACTIVATION_MANAGER` as the default prelaunch executor for orchestrator-managed WPs.

## 20. Command Log

- `git -C ../wtc-workflow-mirror-v1 diff --stat facce56f879d4ee990f62566b12a8b26d8bc61d7..7ed83940a06856359b6789060c792b8b1652ca42`
- `git -C ../wtc-workflow-mirror-v1 rev-list --count facce56f879d4ee990f62566b12a8b26d8bc61d7..7ed83940a06856359b6789060c792b8b1652ca42`
- `git -C ../wtc-workflow-mirror-v1 show -s --format="%H %cI %s" 86cf6510`
- `git -C ../wtc-workflow-mirror-v1 show -s --format="%H %cI %s" 7ed83940`
- `rg -n "model_session_id" ../handshake_main/src/backend/handshake_core/src/api/flight_recorder.rs`
- targeted reads of:
  - `packet.md`
  - `THREAD.md`
  - `RECEIPTS.jsonl`
  - `RUNTIME_STATUS.json`
  - `ROLE_SESSION_REGISTRY.json`
  - `SESSION_CONTROL_RESULTS.jsonl`
  - `WP_TOKEN_USAGE/WP-1-Governance-Workflow-Mirror-v1.json`
  - activation-manager, validator, and integration-validator output JSONLs

## LIVE_FINDINGS_LOG (append-only during WP execution)

- The Activation Manager handoff model is good and should remain the default for orchestrator-managed prelaunch.
- The watchdog exists for a real reason, but the fact that it was needed this much is also a finding.
- Honest `OUTDATED_ONLY` closure is a success condition for workflow truth, not a cosmetic downgrade.
